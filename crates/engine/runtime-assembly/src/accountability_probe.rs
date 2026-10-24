//! 自述面 · 问责计数探测口（独立文件）。
//!
//! 上位：`docs/04-internal/backend-enhancement/accountability-loop-design-2026-10-23.md` §4
//! 与 §5 第 5 步（自述面接入）。
//!
//! 为什么独立成文件：`self_status_source.rs` 在 arch_guard 尺寸基线内
//! （只减不增）。探测口工厂独立后，计数字段的增删不再被那个文件的行数卡住。
//!
//! **只回计数，永不回内容原文**：只有数字。

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_tools_canonical::AccountabilityStats;

use apeireth_tools_canonical::StatusProbe;

/// 自省通道: **问责计数**探测口（影响声明 / 后果 / 待补救）。
///
/// 读两个真源：问责账本（`impact-ledger.jsonl`，影响声明）与后果状态机
/// （进程内 `ConsequenceTracker`）。**只回计数，永不回内容原文**。
///
/// 账本开不出 = 显式 `null` + 原因，不编数（与既有 probe 纪律一致）。
pub fn accountability_status_probe(
    data_dir: PathBuf,
    consequences: Arc<tokio::sync::Mutex<crate::consequence::ConsequenceTracker>>,
) -> StatusProbe<AccountabilityStats> {
    Arc::new(move || {
        let ledger = crate::accountability::AccountabilityLedger::new(Some(
            data_dir.join("impact-ledger.jsonl"),
        ));
        let counts = ledger.counts();
        // ⚠️ 不用 `blocking_lock()`：probe 可能被 async 上下文调用，
        // 而 tokio 的 `blocking_lock` 在 async 线程里会 panic。
        // 拿不到锁就**如实报错**（probe 失败 → 自述面显式 null + 原因），不假装。
        let consequence_counts = {
            let guard = consequences
                .try_lock()
                .map_err(|error| format!("consequence tracker busy: {error}"))?;
            guard.counts()
        };
        let needs_remedy = {
            let guard = consequences
                .try_lock()
                .map_err(|error| format!("consequence tracker busy: {error}"))?;
            guard.needs_remedy(&[])
        };
        Ok(AccountabilityStats {
            impact_statements: Some(counts.total as u64),
            irreversible_actions: Some(counts.irreversible as u64),
            actions_affecting_others: Some(counts.affecting_others as u64),
            outcomes_confirmed: Some(consequence_counts.confirmed as u64),
            outcomes_failed: Some(consequence_counts.failed as u64),
            outcomes_unknown: Some(consequence_counts.unknown as u64),
            needs_remedy: Some(needs_remedy.len() as u64),
            reason: None,
        })
    })
}

/// 跑一次问责探测口（拿不到就**如实报错**，不编数）。
///
/// 独立成函数是为了让 `self_status_source.rs`（尺寸基线内，只减不增）
/// 不因新增计数字段而超线。
pub fn probe_accountability(
    probe: &Option<StatusProbe<AccountabilityStats>>,
) -> (Option<AccountabilityStats>, Option<String>) {
    match probe {
        Some(probe) => match probe() {
            Ok(stats) => {
                let reason = stats.reason.clone();
                (Some(stats), reason)
            }
            Err(error) => (None, Some(format!("accountability probe failed: {error}"))),
        },
        None => (None, None),
    }
}

/// 挂上问责计数探测口（只读子对象数据源）。
///
/// 做成自由函数而非 `impl` 方法：这样 `self_status_source.rs`
/// （尺寸基线内，只减不增）不用为它加行。
#[must_use]
pub fn with_accountability_probe(
    mut source: crate::canonical::self_status_source::ProductionSelfStatusSource,
    probe: StatusProbe<AccountabilityStats>,
) -> crate::canonical::self_status_source::ProductionSelfStatusSource {
    source.accountability = Some(probe);
    source
}
