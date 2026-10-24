//! 守卫**判定落盘**的装配（`governance-audit` 之外的第二个账）。
//!
//! # 为什么要有这个账
//!
//! 守卫的判定（`decision` / `risk_score` / `reasons`）此前只存在
//! `BehaviorChainGuardHook::recent_events` 的 `Mutex<VecDeque>` 里 ——
//! **进程重启即散**。后果：
//!
//! - 事后无法追责（ADR-0014「记录 ≠ 负责」）；
//! - `apeireth nightwatch` 的 no-degrade 复盘段**空转**（它自己报过这条：
//!   「audit chain 未持久化供闲时读取」）。
//!
//! # 纪律
//!
//! - **只记录、不决策**：挂载后每次判定追加一行 JSONL；写盘失败只计数
//!   （`decision_log_write_failures`），**绝不改变**判定结论。
//! - **不静默冒充**：账本打不开时如实降级为「不落盘」并在 stderr 说清，
//!   而不是留一个空文件让人以为审计面在跑。
//! - **与其它账同根**：落在 `APEIRETH_DATA_DIR`/`~/.apeireth` 下
//!   `guard-decisions.jsonl`（与面板、会话库、governance-audit 同一数据根）。
//!
//! # 为什么单独一个模块
//!
//! 尺寸棘轮（`cli/src/lib.rs` 在基线边缘，只减不增）：装配根只留一行调用。

use apeireth_guard::{BehaviorChainGuardHook, GuardDecisionLog, GUARD_DECISION_LOG_FILE};

/// 守夜人一次最多读多少条判定（防把整份历史读进内存）。
pub const NIGHTWATCH_AUDIT_READ_LIMIT: usize = apeireth_guard::decision_log::DEFAULT_READ_LIMIT;

/// 判定落盘路径（与其它账同根）。
pub fn decision_log_path() -> std::path::PathBuf {
    crate::default_panel_data_dir().join(GUARD_DECISION_LOG_FILE)
}

/// 挂上判定落盘面；账本打不开时**如实降级**（返回未挂载的 hook + 打印原因）。
pub fn attach_decision_log(guard: BehaviorChainGuardHook) -> BehaviorChainGuardHook {
    match GuardDecisionLog::open(decision_log_path()) {
        Ok(log) => guard.with_decision_log(std::sync::Arc::new(log)),
        Err(error) => {
            eprintln!("[guard-decisions] 判定落盘不可用: {error}; 本次不落盘（如实）");
            guard
        }
    }
}

/// 把落盘的守卫判定映射成守夜人（nightwatch）吃的审计快照。
///
/// # 这解决了什么
///
/// `apeireth nightwatch` 此前把 `NightwatchInputs.audit` 永远留空 ⇒
/// 它自己报出「audit chain 未持久化供闲时读取 —— no-degrade 复盘段空转」。
/// 现在从这个账读，no-degrade 段**真的会跑**。
///
/// # 映射口径
///
/// - `decision` ← 守卫判定标签（allow / deny / require_approval …）；
/// - `risk` ← [`apeireth_guard::risk_label_for_score`]（本仓唯一 score→档，
///   输出必落在 `governance::risk` 闭合词表内，有测试钉死）；
/// - `reason` ← 多条理由以 `; ` 连接（空理由给空串，不编造）。
///
/// 读不到 / 账不存在 ⇒ 空集合（nightwatch 会照实报缺口，不算错）。
pub fn nightwatch_audit_snapshots(
    limit: usize,
) -> Vec<apeireth_runtime_assembly::canonical::nightwatch::AuditSnapshot> {
    match GuardDecisionLog::open(decision_log_path()) {
        Ok(log) => audit_snapshots_from_log(&log, limit),
        // 账打不开 ⇒ 空集合（nightwatch 会照实报缺口，不静默冒充有审计）
        Err(_) => Vec::new(),
    }
}

/// 从**指定**判定账映射快照（纯函数，测试直调口；不读进程环境）。
///
/// 读不到 / 账不存在 ⇒ 空集合。
pub fn audit_snapshots_from_log(
    log: &GuardDecisionLog,
    limit: usize,
) -> Vec<apeireth_runtime_assembly::canonical::nightwatch::AuditSnapshot> {
    let Ok(events) = log.read_recent(limit) else {
        return Vec::new();
    };
    events
        .into_iter()
        .map(
            |event| apeireth_runtime_assembly::canonical::nightwatch::AuditSnapshot {
                decision: event.decision,
                risk: apeireth_guard::risk_label_for_score(event.risk_score).to_string(),
                reason: event.reasons.join("; "),
            },
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_is_under_the_data_root() {
        let path = decision_log_path();
        assert!(
            path.ends_with(GUARD_DECISION_LOG_FILE),
            "落盘账必须与其它账同根: {}",
            path.display()
        );
    }

    fn event(score: f64, decision: &str, reason: &str) -> apeireth_guard::GuardEventDto {
        apeireth_guard::GuardEventDto {
            timestamp_ms: 1_700_000_000_000,
            session_id: "s".to_string(),
            trace_id: "t".to_string(),
            round: 1,
            capability_id: "tool.shell".to_string(),
            stage: apeireth_guard::GuardStage::FastGuard,
            decision: decision.to_string(),
            risk_score: score,
            reasons: vec![reason.to_string()],
            evidence: Vec::new(),
            containment: None,
        }
    }

    /// 落盘账 → 守夜人快照：三字段如实搬过去，风险档走闭合词表。
    #[test]
    fn snapshots_carry_decision_risk_tier_and_reason() {
        let dir = tempfile::tempdir().expect("tempdir");
        let log = GuardDecisionLog::open(dir.path().join("d.jsonl")).expect("open");
        log.append(&event(0.9, "deny", "危险命令")).expect("append");

        let snapshots = audit_snapshots_from_log(&log, 10);
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].decision, "deny");
        assert_eq!(snapshots[0].risk, "critical", "0.9 应落 critical 档");
        assert_eq!(snapshots[0].reason, "危险命令");
    }

    /// **端到端**：风险从 critical 掉到 info ⇒ 守夜人必须报出「静默降级」。
    ///
    /// 钉住整个闭环：守卫判定落盘 → 守夜人读取 → no-degrade 段**真的跑**。
    /// 接线前 `NightwatchInputs.audit` 恒空，这段永远空转（它自己报过缺口）。
    #[test]
    fn nightwatch_reports_a_silent_risk_degrade_end_to_end() {
        let dir = tempfile::tempdir().expect("tempdir");
        let log = GuardDecisionLog::open(dir.path().join("d.jsonl")).expect("open");
        log.append(&event(0.9, "deny", "危险命令"))
            .expect("append high");
        log.append(&event(0.1, "allow", "无害读取"))
            .expect("append low");

        let snapshots = audit_snapshots_from_log(&log, 10);
        assert_eq!(snapshots.len(), 2);
        let report = apeireth_runtime_assembly::canonical::nightwatch_audit(
            &apeireth_runtime_assembly::canonical::nightwatch::NightwatchInputs {
                episodes: Vec::new(),
                audit: snapshots,
                audit_source_wired: true,
                council_ballots: Vec::new(),
                colang_sources: Vec::new(),
            },
        );
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.detail.contains("审计链风险静默降级")),
            "必须报出静默降级: {:?}",
            report.findings
        );
        assert!(
            !report
                .advisory_gaps
                .iter()
                .any(|gap| gap.contains("audit chain 未持久化")),
            "有审计数据时不该再报该缺口: {:?}",
            report.advisory_gaps
        );
    }

    /// 空账 ⇒ 守夜人**照实报缺口**（不静默冒充有审计）。
    #[test]
    fn empty_log_still_reports_the_gap_honestly() {
        let dir = tempfile::tempdir().expect("tempdir");
        let log = GuardDecisionLog::open(dir.path().join("never.jsonl")).expect("open");
        assert!(audit_snapshots_from_log(&log, 10).is_empty());
        let report = apeireth_runtime_assembly::canonical::nightwatch_audit(
            &apeireth_runtime_assembly::canonical::nightwatch::NightwatchInputs::default(),
        );
        assert!(
            report
                .advisory_gaps
                .iter()
                .any(|gap| gap.contains("audit chain 未持久化")),
            "空账必须报缺口: {:?}",
            report.advisory_gaps
        );
    }

    #[test]
    fn attaching_makes_the_hook_report_the_surface() {
        let hook = BehaviorChainGuardHook::new();
        assert!(!hook.decision_log_enabled(), "默认不落盘（零行为变化）");
        let hooked = attach_decision_log(hook);
        assert!(hooked.decision_log_enabled(), "挂上后应报告已挂载");
        assert_eq!(hooked.decision_log_write_failures(), 0);
    }
}
