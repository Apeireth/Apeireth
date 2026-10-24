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
pub const USAGE: &str = "apeireth egress ledger [--path <LEDGER_JSONL>] [--limit <N>] [--json]\n\n  只读列出出域账本（每次模型出网判定的记录：判定 / 对象类型 / 指纹 / 目的地 / 用途）。\n  账本**零真值**：只有对象类型与不可逆指纹，没有内容原文。\n  缺省 path = 数据根（APEIRETH_DATA_DIR 或默认目录）下的 egress-ledger.jsonl。";

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
    if verb != "ledger" {
        return Err(format!("unknown egress command: {verb}\n{USAGE}"));
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
