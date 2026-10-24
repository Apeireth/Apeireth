//! `apeireth egress ledger` —— 出域账本的人类出口。
//!
//! 上位：ADR-0013 §2.2（动态红线）、ADR-0014 §2 / §3（承诺分级：R2/R4 不保证，
//! 但**必须记录已知流向、提供请求路径、在发现影响时通知**）。
//!
//! 为什么要有这个出口：出网闸（`sec.egress_gate`）与出域账本
//! （`apeireth_runtime_assembly::egress_ledger`）在库级已接线，但**账本若无人可查，
//! 「记录已知流向」就只是一句承诺**。本命令是那个「可查」的最小实现 ——
//! 与 `apeireth exposure report`（暴露面体检）、`apeireth lessons`（教训）同族。
//!
//! 只读：零写。账本本体在 `APEIRETH_DATA_DIR`/`~/.apeireth` 下 `egress-ledger.jsonl`。

use std::path::PathBuf;

use apeireth_runtime_assembly::egress_ledger::EgressLedger;

/// 用法帧。
pub const USAGE: &str = "apeireth egress ledger [--path <LEDGER_JSONL>] [--limit <N>] [--json]\napeireth egress impact [--path <IMPACT_JSONL>] [--limit <N>] [--json]\napeireth egress outcome <ACTION_ID> <confirmed|failed>\napeireth egress notify <FACT_ID> <wording|partial|refuted>\n\n  ledger = 出域账本（观测面：判定 / 对象类型 / 指纹 / 目的地 / 用途）。\n  impact = 问责账本（责任面：授权 / 可逆性 / 影响谁 / 后果）。\n  两个账本**都零真值**：只有指涉 id，没有内容原文。\n  缺省 path = 数据根（APEIRETH_DATA_DIR 或默认目录）下的 egress-ledger.jsonl / impact-ledger.jsonl。";

/// 命令入口（数据根 = 默认数据目录）。
pub fn run_egress(args: &[String]) -> Result<String, String> {
    run_egress_at(&crate::default_panel_data_dir(), args)
}

/// 显式数据根入口（测试与非 env 装配用；语义与 [`run_egress`] 逐字一致）。
pub fn run_egress_at(data_root: &std::path::Path, args: &[String]) -> Result<String, String> {
    let (verb, rest) = match args.split_first() {
        Some((verb, rest)) => (verb.as_str(), rest),
        None => return Err(USAGE.to_string()),
    };
    match verb {
        "ledger" => {}
        "impact" => return run_impact(data_root, rest),
        "outcome" => return run_outcome(data_root, rest),
        "notify" => return run_correction_impact(data_root, rest),
        _ => return Err(format!("unknown egress command: {verb}\n{USAGE}")),
    }
    // 路径：--path 显式优先；否则走数据根（`APEIRETH_DATA_DIR` 或默认目录）。
    // **不新增独立 env** —— env 名数有棘轮（160 上限，只减不增），
    // 且账本位置属于「数据根」语义，不该另开一个开关。
    let path = match parse_arg(rest, "--path") {
        Some(raw) => PathBuf::from(raw),
        None => data_root.join("egress-ledger.jsonl"),
    };
    let limit = match parse_arg(rest, "--limit") {
        Some(raw) => raw
            .parse::<usize>()
            .map_err(|_| format!("--limit 需要一个非负整数: {raw}"))?,
        None => 50,
    };
    let json = rest.iter().any(|arg| arg == "--json");

    let ledger = EgressLedger::new(Some(path.clone()));
    let mut entries = ledger.entries();
    // 最近的在后；取尾部 limit 条再倒序输出（新的在前）。
    let total = entries.len();
    if entries.len() > limit {
        entries.drain(..entries.len() - limit);
    }
    entries.reverse();

    if json {
        return serde_json::to_string_pretty(&entries)
            .map_err(|error| format!("出域账本序列化失败: {error}"));
    }

    let mut frame = format!(
        "egress ledger: {total} 条在册（显示最近 {} 条）\n",
        entries.len()
    );
    if entries.is_empty() {
        frame.push_str(
            "（账本为空 —— 可能还没开过出网闸（APEIRETH_ENABLE_EGRESS_GATE=1），\
             或这一条判定之前还没有出网发生过。空账本不等于「没外流过」，只等于「没有记录」。）\n",
        );
        return Ok(frame);
    }
    for entry in entries {
        let objects = if entry.objects.is_empty() {
            "无".to_string()
        } else {
            entry.objects.join("/")
        };
        let fingerprints = if entry.fingerprints.is_empty() {
            String::new()
        } else {
            format!("  指纹={}", entry.fingerprints.join(","))
        };
        frame.push_str(&format!(
            "- [{ts}] {verdict} → {destination} (用途 {usage}) 对象={objects}{fingerprints}\n",
            ts = entry.ts_unix_ms,
            verdict = entry.verdict,
            destination = entry.destination,
            usage = entry.usage,
            objects = objects,
            fingerprints = fingerprints
        ));
    }
    Ok(frame)
}

/// `apeireth egress impact` —— **问责账本**只读出口。
///
/// 与 `ledger`（出域账本）的分工：
/// - `ledger` = 观测面：「这次出网有没有检出敏感对象、去了哪」；
/// - `impact` = 责任面：「这次行动依赖了什么授权、能不能撤回、影响谁、后果如何」。
///
/// **只读零写**，**零真值**（只指涉 id，不带内容）。
/// 「超期未回填」阈值：后果未知超过这个时长 = 问责链上的**显式缺口**。
///
/// 为什么是 24 小时：日常外部行动（发信/提交申请）当日内必然知道结果；
/// 一整天都没回填，就不是「还没来得及」，而是**没人管**。这个门槛写在这里
/// 而不是配置项 —— 它只是**报告的强调阈值**，不是决策/守门依据。
pub const STALE_UNKNOWN_MS: i64 = 24 * 60 * 60 * 1_000;

/// 一条影响声明的「后果是否超期未知」判定（纯函数，便于脱时钟测试）。
///
/// 只有**真正未知**才算缺口：已 `confirmed` / `failed` 的语句不属于未回填。
pub fn is_stale_unknown(
    outcome_is_unknown: bool,
    at_ms: i64,
    now_ms: i64,
    threshold_ms: i64,
) -> bool {
    outcome_is_unknown && now_ms.saturating_sub(at_ms) > threshold_ms
}

pub fn run_impact(data_root: &std::path::Path, rest: &[String]) -> Result<String, String> {
    let path = match parse_arg(rest, "--path") {
        Some(raw) => PathBuf::from(raw),
        None => data_root.join("impact-ledger.jsonl"),
    };
    let limit = match parse_arg(rest, "--limit") {
        Some(raw) => raw
            .parse::<usize>()
            .map_err(|_| format!("--limit 需要一个非负整数: {raw}"))?,
        None => 50,
    };
    let json = rest.iter().any(|arg| arg == "--json");

    let ledger =
        apeireth_runtime_assembly::accountability::AccountabilityLedger::new(Some(path.clone()));
    let mut statements = ledger.statements();

    // **叠加后果账**：影响声明只记「当时打算做什么」，后果由 consequence-ledger
    // 单独演进（`egress outcome` 往那写）。此前本命令**只读影响账**，导致
    // 回填后报告仍显示「后果未知」—— 回填在 CLI 上**看不出任何效果**（闭环断在读取侧）。
    // 真值读法用与自述面同一个 `ConsequenceTracker`（文件即共享真源）。
    let consequence_ledger_path = path.with_file_name("consequence-ledger.jsonl");
    let tracker = apeireth_runtime_assembly::consequence::ConsequenceTracker::with_ledger(
        consequence_ledger_path,
    );
    for statement in &mut statements {
        // **只升级、不降级**：后果账没有该行动的记录时 `current_outcome` 返
        // `Unknown`，此时**保留影响声明自带的 outcome**（早期流程可能已写过
        // `confirmed`/`failed`）。无条件覆盖会把已知后果**降级成未知** ——
        // 那是把真信息变假信息，比不叠加更糟。
        let tracked = tracker.current_outcome(&statement.action_id);
        if tracked != apeireth_runtime_assembly::accountability::Outcome::Unknown {
            statement.outcome = tracked;
        }
    }
    let total = statements.len();
    if statements.len() > limit {
        statements.drain(..statements.len() - limit);
    }
    statements.reverse();

    if json {
        return serde_json::to_string_pretty(&statements)
            .map_err(|error| format!("问责账本序列化失败: {error}"));
    }

    // 计数**必须按叠加后的语句重算**：`ledger.counts()` 读的是原始影响账，
    // 会与上面叠加过的 outcome 打架（实测：回填后「超期未回填=0」却仍写
    // 「后果未知=1」，自相矛盾）。所有计数一律从同一份叠加后数据派生。
    let counts = apeireth_runtime_assembly::accountability::AccountabilityCounts {
        total,
        irreversible: statements.iter().filter(|s| !s.reversible).count(),
        affecting_others: statements
            .iter()
            .filter(|s| s.affected != apeireth_runtime_assembly::accountability::Affected::SelfOnly)
            .count(),
        unknown_outcome: statements
            .iter()
            .filter(|s| s.outcome == apeireth_runtime_assembly::accountability::Outcome::Unknown)
            .count(),
    };
    // 「超期未回填」= 问责链上的显式缺口：后果未知且已超过阈值。
    // 这不是告警、不阻塞任何东西 —— 只是把「账面长期停在 unknown」这件事说出来。
    let now_ms = chrono::Utc::now().timestamp_millis();
    let stale = statements
        .iter()
        .filter(|statement| {
            is_stale_unknown(
                statement.outcome == apeireth_runtime_assembly::accountability::Outcome::Unknown,
                statement.at_ms,
                now_ms,
                STALE_UNKNOWN_MS,
            )
        })
        .count();
    let mut frame = format!(
        "impact ledger: {total} 条在册（显示最近 {} 条）\n\
         不可逆={} 影响他人={} 后果未知={} 超期未回填={}\n",
        statements.len(),
        counts.irreversible,
        counts.affecting_others,
        counts.unknown_outcome,
        stale
    );
    if stale > 0 {
        frame.push_str(
            "⚠ 超期未回填 > 0：这些行动的后果至今**无人回填**。\n\
             回填通道: apeireth egress outcome <ACTION_ID> <confirmed|failed>\n\
             （系统不猜外部结果；逾期不回填 = 问责链断在最后一步）\n",
        );
    }
    if statements.is_empty() {
        frame.push_str(
            "（账本为空 —— 可能还没开过出网闸，或这些行动还没被记录。\n\
             空账本不等于「没外流过」，只等于「没有记录」。）\n",
        );
        return Ok(frame);
    }
    for statement in statements {
        // **决策回执**（问责闭环第 2 步）：面向被影响的人**一行话**，不是日志。
        // 这就是 `DecisionReceipt::from_statement` 的生产调用方 —— 之前它只有测试调用，
        // `audit_unwired.py` 抓出来的真问题（能力齐全但没接电）。
        let receipt = apeireth_runtime_assembly::notification_duty::DecisionReceipt::from_statement(
            &statement,
        );
        frame.push_str(&format!("- {}\n", receipt.headline));
        for basis in &receipt.basis {
            frame.push_str(&format!("    {basis}\n"));
        }
        // 单条「超期未回填」标记（与头部的计数同源判据）
        if is_stale_unknown(
            statement.outcome == apeireth_runtime_assembly::accountability::Outcome::Unknown,
            statement.at_ms,
            now_ms,
            STALE_UNKNOWN_MS,
        ) {
            let hours = now_ms.saturating_sub(statement.at_ms) / 3_600_000;
            frame.push_str(&format!(
                "    ⏰ 超期未回填（已 {hours} 小时，后果仍未知）-- 用 egress outcome {} 回填\n",
                statement.action_id
            ));
        }
        frame.push_str(&format!(
            "    [详细] {ts} {boundary} → {destination} 行动={action_id} 可逆={reversible} 后果={outcome}\n\
             用了哪些事实: {facts}\n  依据的授权: {grants}\n",
            ts = statement.at_ms,
            boundary = statement.boundary.label(),
            destination = statement.destination,
            action_id = statement.action_id,
            reversible = statement.reversible,
            outcome = statement.outcome.label(),
            facts = if statement.facts_used.is_empty() {
                "（未记录）".to_string()
            } else {
                statement.facts_used.join(", ")
            },
            grants = if statement.grants.is_empty() {
                "（无授权记录）".to_string()
            } else {
                statement.grants.join(", ")
            },
        ));
    }
    Ok(frame)
}

/// `apeireth egress outcome <ACTION_ID> <confirmed|failed>` —— **后果回填**。
///
/// 这是问责闭环第 3 步缺的那条通道：系统自己**不知道**外部行动的结果
/// （邮件发没发出、申请提没提交），只能由**回执渠道或用户**告诉它。
///
/// 三条纪律：
/// 1. **显式回填** —— 系统不猜、不自动探测外部世界；
/// 2. **状态机把关** —— 已确认的事实不能被退回 `unknown`（`record` 会拒绝）；
/// 3. **append-only** —— 每次回填追加一条事件，历史不被改写。
pub fn run_outcome(data_root: &std::path::Path, rest: &[String]) -> Result<String, String> {
    let path = match parse_arg(rest, "--path") {
        Some(raw) => PathBuf::from(raw),
        None => data_root.join("consequence-ledger.jsonl"),
    };
    // 位置参数：<ACTION_ID> <confirmed|failed>
    //
    // ⚠️ 不能只按「不以 -- 开头」过滤：`--path <value>` 的**值**也不以 -- 开头，
    // 会被误当位置参数（我第一版就是这么错的，测试抓到）。
    // 正确做法：遇到 `--xxx` 就跳过它和它的值。
    let mut positional: Vec<&String> = Vec::new();
    let mut index = 0;
    while index < rest.len() {
        if rest[index].starts_with("--") {
            index += 2; // 跳过旗标名 + 它的值
            continue;
        }
        positional.push(&rest[index]);
        index += 1;
    }
    let (action_id, outcome_label) = match positional.as_slice() {
        [action_id, outcome] => (action_id.as_str(), outcome.as_str()),
        _ => {
            return Err(format!(
                "outcome 需要两个位置参数：<ACTION_ID> <confirmed|failed>\n{USAGE}"
            ))
        }
    };
    let outcome = match outcome_label {
        "confirmed" => apeireth_runtime_assembly::accountability::Outcome::Confirmed,
        "failed" => apeireth_runtime_assembly::accountability::Outcome::Failed,
        other => {
            return Err(format!(
                "未知后果状态 `{other}`（只接受 confirmed / failed ——                  系统不替用户猜「大概成功了」）"
            ))
        }
    };
    let mut tracker =
        apeireth_runtime_assembly::consequence::ConsequenceTracker::with_ledger(path.clone());
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    if !tracker.record(
        action_id,
        outcome,
        apeireth_runtime_assembly::consequence::OutcomeSource::UserConfirmed,
        now_ms,
    ) {
        return Err(format!(
            "回填被拒：行动 `{action_id}` 的当前后果是 `{}`，不能改成 `{}` \
             （已确认的事实不能被抹掉）",
            tracker.current_outcome(action_id).label(),
            outcome.label()
        ));
    }
    Ok(format!(
        "已回填：行动 {action_id} 的后果 = {}（来源：用户确认）\n账本：{}",
        outcome.label(),
        path.display()
    ))
}

/// `apeireth egress notify <FACT_ID> <wording|partial|refuted>` —— **纠正影响检查**。
///
/// 问责闭环第 4 步「通知义务」的生产调用方（之前 `assess_correction` 只有测试调用，
/// 被 `audit_unwired.py` 抓出来 —— 又一个「能力齐全但没接电」）。
///
/// 用户纠正一条事实后跑它，回答四个问题：
/// 1. **受影响事项**（哪几次行动、何时、去了哪）；
/// 2. **影响程度**（措辞 / 部分 / 推翻）；
/// 3. **建议补救动作**；
/// 4. **是否需要授权**（系统不擅自"到处发信"）。
///
/// **它不发任何东西** —— 只出待办。通知义务 ≠ 自动外发。
pub fn run_correction_impact(
    data_root: &std::path::Path,
    rest: &[String],
) -> Result<String, String> {
    use apeireth_runtime_assembly::accountability::AccountabilityLedger;
    use apeireth_runtime_assembly::notification_duty::{assess_correction, CorrectionSeverity};

    let mut positional: Vec<&String> = Vec::new();
    let mut index = 0;
    while index < rest.len() {
        if rest[index].starts_with("--") {
            index += 2;
            continue;
        }
        positional.push(&rest[index]);
        index += 1;
    }
    let (fact_id, severity_label) = match positional.as_slice() {
        [fact, severity] => (fact.as_str(), severity.as_str()),
        _ => {
            return Err(format!(
                "notify 需要两个位置参数：<FACT_ID> <wording|partial|refuted>\n{USAGE}"
            ))
        }
    };
    let severity = match severity_label {
        "wording" => CorrectionSeverity::Wording,
        "partial" => CorrectionSeverity::Partial,
        "refuted" => CorrectionSeverity::Refuted,
        other => {
            return Err(format!(
                "未知纠正程度 `{other}`（只接受 wording / partial / refuted ——                  程度决定补救力度，系统不替用户判重）"
            ))
        }
    };
    let path = match parse_arg(rest, "--path") {
        Some(raw) => PathBuf::from(raw),
        None => data_root.join("impact-ledger.jsonl"),
    };
    let ledger = AccountabilityLedger::new(Some(path));
    let Some(notice) = assess_correction(&ledger, fact_id, severity) else {
        return Ok(format!(
            "这条事实（{fact_id}）没有被任何外部行动引用过 ⇒ **无需通知**。\n\
             （无差别通知一切纠正是另一种噪声 —— 所以「不打扰」是设计，不是遗漏。）"
        ));
    };
    let mut out = format!(
        "纠正影响检查：事实 {fact_id}（程度 {}）\n\
         受影响事项 {} 次：\n",
        notice.severity.label(),
        notice.affected_actions.len()
    );
    for action in &notice.affected_actions {
        out.push_str(&format!(
            "  - {action_id}  时间={at}  去向={destination}  可逆={reversible}  后果={outcome}\n",
            action_id = action.action_id,
            at = action.at_ms,
            destination = action.destination,
            reversible = action.reversible,
            outcome = action.outcome.label(),
        ));
    }
    out.push_str("建议补救：\n");
    for remedy in &notice.suggested_remedies {
        out.push_str(&format!("  - {remedy}\n"));
    }
    out.push_str(&format!(
        "\n需要你授权才能执行补救：{}\n\
         **本命令不发送任何东西** —— 通知义务 ≠ 自动外发；要不要发、发给谁由你定。\n",
        if notice.needs_authorization {
            "是"
        } else {
            "否（改措辞是我们该做的）"
        }
    ));
    Ok(out)
}

/// 取 `--<name>` 后的值（最后一个出现者生效）。
fn parse_arg(args: &[String], name: &str) -> Option<String> {
    let mut found = None;
    let mut index = 0;
    while index < args.len() {
        if args[index] == name {
            found = args.get(index + 1).cloned();
            index += 2;
            continue;
        }
        index += 1;
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_runtime_assembly::egress_ledger::{EgressLedger, EgressLedgerEntry};

    fn entry(verdict: &str, objects: Vec<&str>) -> EgressLedgerEntry {
        EgressLedgerEntry {
            ts_unix_ms: 1_700_000_000_000,
            session: None,
            verdict: verdict.to_string(),
            objects: objects.into_iter().map(String::from).collect(),
            fingerprints: vec!["ef79ffc52d25".into()],
            destination: "provider:anthropic".into(),
            usage: "chat".into(),
            reason: None,
        }
    }

    fn write_ledger(tag: &str, entries: &[EgressLedgerEntry]) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "apeireth-egress-cli-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let ledger = EgressLedger::new(Some(path.clone()));
        for entry in entries {
            ledger.append(entry.clone());
        }
        path
    }

    #[test]
    fn empty_ledger_says_empty_and_does_not_claim_no_leak() {
        let path = std::env::temp_dir().join("apeireth-egress-cli-empty-does-not-exist");
        let _ = std::fs::remove_file(&path);
        let out = run_egress_at(
            std::path::Path::new("/nonexistent-root"),
            &["ledger".into(), "--path".into(), path.display().to_string()],
        )
        .expect("应成功");
        assert!(out.contains("账本为空"));
        // 关键：空账本**不等于**没外流过。
        assert!(out.contains("只等于「没有记录」"));
    }

    #[test]
    fn lists_entries_with_verdict_and_objects() {
        let path = write_ledger(
            "list",
            &[entry("pass", vec![]), entry("redacted", vec!["email"])],
        );
        let out = run_egress_at(
            std::path::Path::new("/nonexistent-root"),
            &["ledger".into(), "--path".into(), path.display().to_string()],
        )
        .expect("应成功");
        assert!(out.contains("2 条在册"));
        assert!(out.contains("redacted"));
        assert!(out.contains("对象=email"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn json_output_parses() {
        let path = write_ledger("json", &[entry("redacted", vec!["email"])]);
        let out = run_egress_at(
            std::path::Path::new("/nonexistent-root"),
            &[
                "ledger".into(),
                "--path".into(),
                path.display().to_string(),
                "--json".into(),
            ],
        )
        .expect("应成功");
        let parsed: serde_json::Value = serde_json::from_str(&out).expect("应为合法 JSON");
        assert_eq!(parsed.as_array().map(|a| a.len()), Some(1));
        let _ = std::fs::remove_file(&path);
    }

    /// `impact` 子命令：责任面（授权 / 可逆性 / 后果）。
    #[test]
    fn impact_subcommand_lists_statements() {
        use apeireth_runtime_assembly::accountability::{
            AccountabilityLedger, Affected, Boundary, ImpactStatement,
        };

        let path = std::env::temp_dir().join(format!(
            "apeireth-egress-impact-cli-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let ledger = AccountabilityLedger::new(Some(path.clone()));
        ledger.append(&ImpactStatement {
            action_id: "act-1".into(),
            at_ms: 1_700_000_000_000,
            boundary: Boundary::Provider,
            facts_used: vec!["ep-1".into()],
            inferences: Vec::new(),
            grants: vec!["grant-1".into()],
            affected: Affected::SelfOnly,
            reversible: false,
            destination: "provider:anthropic".into(),
            outcome: apeireth_runtime_assembly::accountability::Outcome::Unknown,
        });
        let out = run_egress_at(
            std::path::Path::new("/nonexistent-root"),
            &["impact".into(), "--path".into(), path.display().to_string()],
        )
        .expect("应成功");
        assert!(out.contains("act-1"));
        assert!(out.contains("不可逆"));
        assert!(out.contains("unknown"), "后果未知要如实显示");
        let _ = std::fs::remove_file(&path);
    }

    /// 后果回填：unknown → confirmed，且落盘可查。
    #[test]
    fn outcome_backfill_records_and_persists() {
        let path = std::env::temp_dir().join(format!(
            "apeireth-egress-outcome-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let args: Vec<String> = vec![
            "outcome".into(),
            "act-1".into(),
            "confirmed".into(),
            "--path".into(),
            path.display().to_string(),
        ];
        let out = run_egress_at(std::path::Path::new("/nonexistent-root"), &args).expect("应成功");
        assert!(out.contains("已回填"));
        assert!(out.contains("confirmed"));

        // 落盘可查：新实例读回同一条。
        let tracker =
            apeireth_runtime_assembly::consequence::ConsequenceTracker::with_ledger(path.clone());
        assert_eq!(
            tracker.current_outcome("act-1"),
            apeireth_runtime_assembly::accountability::Outcome::Confirmed
        );
        let _ = std::fs::remove_file(&path);
    }

    /// 已确认的事实不能被退回 —— 状态机把关。
    #[test]
    fn outcome_backfill_refuses_regression() {
        let path = std::env::temp_dir().join(format!(
            "apeireth-egress-regress-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let mut first: Vec<String> = vec![
            "outcome".into(),
            "act-1".into(),
            "confirmed".into(),
            "--path".into(),
            path.display().to_string(),
        ];
        run_egress_at(std::path::Path::new("/nonexistent-root"), &first).expect("首次应成功");
        first[2] = "failed".into();
        let err = run_egress_at(std::path::Path::new("/nonexistent-root"), &first)
            .expect_err("已确认的不能改成 failed");
        assert!(err.contains("回填被拒"), "理由应可读: {err}");
        let _ = std::fs::remove_file(&path);
    }

    /// 未知后果状态被拒（系统不替用户猜「大概成功了」）。
    #[test]
    fn outcome_backfill_rejects_guessing() {
        let path = std::env::temp_dir().join("apeireth-egress-guess-unused");
        let args: Vec<String> = vec![
            "outcome".into(),
            "act-1".into(),
            "probably-ok".into(),
            "--path".into(),
            path.display().to_string(),
        ];
        let err =
            run_egress_at(std::path::Path::new("/nonexistent-root"), &args).expect_err("应拒绝");
        assert!(
            err.contains("只接受 confirmed / failed"),
            "理由应可读: {err}"
        );
    }

    /// 纠正影响检查：**不被引用的事实 → 明确说「无需通知」**（不打扰是设计）。
    #[test]
    fn correction_impact_says_no_notification_for_unreferenced_fact() {
        let path = std::env::temp_dir().join(format!(
            "apeireth-notify-cli-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let out = run_egress_at(
            std::path::Path::new("/nonexistent-root"),
            &[
                "notify".into(),
                "ep-404".into(),
                "refuted".into(),
                "--path".into(),
                path.display().to_string(),
            ],
        )
        .expect("应成功");
        assert!(out.contains("无需通知"), "应明确说无需通知: {out}");
        assert!(out.contains("不打扰"), "应解释这是设计: {out}");
        let _ = std::fs::remove_file(&path);
    }

    /// 被引用的事实 → 列出受影响行动 + 建议补救 + **不自动外发**。
    #[test]
    fn correction_impact_lists_actions_and_never_auto_sends() {
        use apeireth_runtime_assembly::accountability::{
            AccountabilityLedger, Affected, Boundary, ImpactStatement, Outcome,
        };

        let path = std::env::temp_dir().join(format!(
            "apeireth-notify2-cli-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let ledger = AccountabilityLedger::new(Some(path.clone()));
        ledger.append(&ImpactStatement {
            action_id: "act-1".into(),
            at_ms: 1_700_000_000_000,
            boundary: Boundary::Provider,
            facts_used: vec!["ep-1".into()],
            inferences: Vec::new(),
            grants: vec!["grant-1".into()],
            affected: Affected::SelfOnly,
            reversible: false,
            destination: "provider:anthropic".into(),
            outcome: Outcome::Unknown,
        });
        let out = run_egress_at(
            std::path::Path::new("/nonexistent-root"),
            &[
                "notify".into(),
                "ep-1".into(),
                "refuted".into(),
                "--path".into(),
                path.display().to_string(),
            ],
        )
        .expect("应成功");
        assert!(out.contains("act-1"), "应列出受影响行动: {out}");
        assert!(out.contains("建议补救"), "应给补救建议: {out}");
        assert!(out.contains("不发送任何东西"), "必须声明不自动外发: {out}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn unknown_verb_is_refused() {
        let err = run_egress_at(
            std::path::Path::new("/nonexistent-root"),
            &["nonsense".into()],
        )
        .expect_err("应拒绝");
        assert!(err.contains("unknown egress command"));
    }
}

#[cfg(test)]
mod stale_unknown_tests {
    use super::*;
    use apeireth_runtime_assembly::accountability::{
        AccountabilityLedger, Affected, Boundary, ImpactStatement, Outcome,
    };

    fn statement(action_id: &str, at_ms: i64, outcome: Outcome) -> ImpactStatement {
        ImpactStatement {
            action_id: action_id.to_string(),
            at_ms,
            boundary: Boundary::ThirdParty,
            facts_used: Vec::new(),
            inferences: Vec::new(),
            grants: Vec::new(),
            affected: Affected::SelfOnly,
            reversible: true,
            destination: "provider:anthropic".to_string(),
            outcome,
        }
    }

    fn ledger_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "apeireth-impact-stale-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ))
    }

    fn write(path: &std::path::Path, statements: &[ImpactStatement]) {
        let ledger = AccountabilityLedger::new(Some(path.to_path_buf()));
        for statement in statements {
            assert!(ledger.append(&statement.clone()), "append");
        }
    }

    /// 纯判据：只有「未知 + 超期」才算缺口。
    #[test]
    fn stale_predicate_only_fires_for_old_unknowns() {
        let day = STALE_UNKNOWN_MS;
        assert!(is_stale_unknown(true, 0, day + 1, day), "超期未知 = 缺口");
        assert!(!is_stale_unknown(true, 0, day, day), "恰好门槛不算超期");
        assert!(!is_stale_unknown(false, 0, day + 1, day), "已回填不算缺口");
        // 时钟回拨：未来时间戳不误报
        assert!(!is_stale_unknown(true, day * 2, 0, day));
    }

    /// 端到端：`egress impact` 对超期未回填既**计数**又**逐条标记**。
    #[test]
    fn impact_report_flags_stale_unknowns() {
        let path = ledger_path("e2e");
        let now = chrono::Utc::now().timestamp_millis();
        write(
            &path,
            &[
                // 3 天前的未知后果 ⇒ 超期
                statement(
                    "act-old-unknown",
                    now - 3 * 24 * 3_600_000,
                    Outcome::Unknown,
                ),
                // 1 小时前的未知后果 ⇒ 未超期
                statement("act-fresh-unknown", now - 3_600_000, Outcome::Unknown),
                // 3 天前但已回填 ⇒ 不算缺口
                statement(
                    "act-old-confirmed",
                    now - 3 * 24 * 3_600_000,
                    Outcome::Confirmed,
                ),
            ],
        );

        let out = run_impact(
            std::path::Path::new("/nonexistent-root"),
            &["--path".into(), path.display().to_string()],
        )
        .expect("应成功");

        assert!(out.contains("超期未回填=1"), "头部应计数 1: {out}");
        assert!(
            out.contains("⏰ 超期未回填（已 72 小时"),
            "应标出超期那条: {out}"
        );
        assert!(
            out.contains("act-old-unknown"),
            "标记要带可回填的行动 id: {out}"
        );
        // 未超期 / 已回填的两条不得被标记
        assert_eq!(out.matches("超期未回填（已").count(), 1, "{out}");
        let _ = std::fs::remove_file(&path);
    }

    /// 账本为空时不得出现「超期」噪声（空账 ≠ 有缺口）。
    #[test]
    fn empty_ledger_has_no_stale_noise() {
        let path = ledger_path("empty");
        let _ = std::fs::remove_file(&path);
        let out = run_impact(
            std::path::Path::new("/nonexistent-root"),
            &["--path".into(), path.display().to_string()],
        )
        .expect("应成功");
        assert!(out.contains("超期未回填=0"), "{out}");
        assert!(!out.contains("⏰"), "空账不该有超期标记: {out}");
    }
}

#[cfg(test)]
mod consequence_join_tests {
    use super::*;
    use apeireth_runtime_assembly::accountability::{
        AccountabilityLedger, Affected, Boundary, ImpactStatement, Outcome,
    };
    use apeireth_runtime_assembly::consequence::ConsequenceTracker;

    fn statement(action_id: &str, at_ms: i64) -> ImpactStatement {
        ImpactStatement {
            action_id: action_id.to_string(),
            at_ms,
            boundary: Boundary::UserToOrg,
            facts_used: Vec::new(),
            inferences: Vec::new(),
            grants: Vec::new(),
            affected: Affected::NamedParty,
            reversible: false,
            destination: "smtp:hr@example.com".to_string(),
            outcome: Outcome::Unknown,
        }
    }

    fn dir(tag: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "apeireth-impact-join-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&path).expect("mkdir");
        path
    }

    /// **闭环**：后果账里的回填必须反映到 `egress impact` 上。
    ///
    /// 这是「回填只写不读」那个缺陷的回归测试 ——
    /// 修前：写入 consequence-ledger 后，impact 报告仍显示「后果未知」，
    /// 回填在 CLI 上**看不出任何效果**（问责闭环断在读取侧）。
    #[test]
    fn impact_report_reflects_backfilled_outcomes() {
        let root = dir("reflect");
        let impact_path = root.join("impact-ledger.jsonl");
        let ledger = AccountabilityLedger::new(Some(impact_path.clone()));
        let old = chrono::Utc::now().timestamp_millis() - 3 * 24 * 3_600_000;
        assert!(ledger.append(&statement("act-1", old)), "append");

        // ① 未回填：未知 + 超期
        let before = run_impact(
            std::path::Path::new("/nonexistent-root"),
            &["--path".into(), impact_path.display().to_string()],
        )
        .expect("impact");
        assert!(before.contains("后果未知=1"), "{before}");
        assert!(before.contains("超期未回填=1"), "{before}");

        // ② 回填 confirmed（`record` 需要 &mut，且来源是 OutcomeSource::UserConfirmed）
        let mut tracker = ConsequenceTracker::with_ledger(root.join("consequence-ledger.jsonl"));
        assert!(
            tracker.record(
                "act-1",
                Outcome::Confirmed,
                apeireth_runtime_assembly::consequence::OutcomeSource::UserConfirmed,
                chrono::Utc::now().timestamp_millis(),
            ),
            "record consequence"
        );

        // ③ 回填后：未知归零、超期归零、明细显示 confirmed
        let after = run_impact(
            std::path::Path::new("/nonexistent-root"),
            &["--path".into(), impact_path.display().to_string()],
        )
        .expect("impact");
        assert!(after.contains("后果未知=0"), "回填后不该再有未知: {after}");
        assert!(after.contains("超期未回填=0"), "{after}");
        assert!(
            after.contains("后果=confirmed"),
            "明细应显示回填后的后果: {after}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
