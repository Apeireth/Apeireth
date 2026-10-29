//! R8-B2 (ADR-0010 D3) 治理单一入口装配面。
//!
//! 生产治理栈 (pipeline 外包 permission preset 包装层) 作为
//! [`GovernanceChain`] 的 Gate 段, 再包成 [`ChainGovernanceHook`] 交给
//! runtime 的 `with_governance` —— 每次治理评估都过 `GovernanceChain::run`
//! 单点, preset 仍在链内 gate 位、语义不变; 未配置时 runtime 默认
//! `DenyUnconfigured` fail-closed 不变。链入口与旧入口四态行为等价真账:
//! `tests/governance_chain_equivalence.rs`。

use std::sync::Arc;

use apeireth_governance::{
    AuditSink, ChainGovernanceHook, GovernanceChain, GovernanceHook, InMemoryAuditSink,
};

/// 把治理 gate 包成 chain 单入口 hook (R8-B2 装配根切换; 内存审计默认)。
///
/// 返回值即 runtime 认识的 [`GovernanceHook`]; 裁决种类与 gate 直连等价,
/// verdict 元数据按 R8-B1 适配器映射如实标 `governance_chain`。
pub fn adapt_gate(gate: Arc<dyn GovernanceHook>) -> Arc<dyn GovernanceHook> {
    adapt_gate_with(gate, Arc::new(InMemoryAuditSink::new()))
}

/// chain 单入口 hook + 显式审计 sink (R8-B3 注入面)。
pub fn adapt_gate_with(
    gate: Arc<dyn GovernanceHook>,
    audit: Arc<dyn AuditSink>,
) -> Arc<dyn GovernanceHook> {
    let chain = Arc::new(GovernanceChain::new(gate).with_audit_sink(audit));
    Arc::new(ChainGovernanceHook::new(chain))
}

/// 生产装配 (R8-B2 + R8-B3): gate → chain 单入口 hook + 持久审计 sink。
///
/// 审计账本 = `APEIRETH_DATA_DIR`/`~/.apeireth` 下 `governance-audit.jsonl`
/// (与面板/会话库同根)。账本开不出时**如实降级内存账并打印** (ADR-0010 D4
/// 「持久化通过 AuditSink 注入实现, 默认仍是内存」; 降级 = 审计不持久、重启后
/// 无链可 verify —— 喊破, 不静默、不冒充); `run()` 期追加失败仍 fail-closed
/// (`ChainOutcome::Denied`), 降级只影响持久性、不影响守门。
pub fn production(gate: impl GovernanceHook + 'static) -> Arc<dyn GovernanceHook> {
    let audit = shared_audit_sink(crate::default_panel_data_dir().join("governance-audit.jsonl"));
    adapt_gate_with(Arc::new(gate), audit)
}

/// **进程内单例审计 sink**（按账本路径）。
///
/// 为什么必须单例：链 sink 的 `sequence = 记录数` 是**单写者**结构（见
/// `gateway::chain_audit` 模块头）。同一进程里若每个 runtime 装配各开一个
/// sink，两个 sink 各持一条内存链、共用同一文件 ⇒ 后写者基于**过期链尾**追加
/// ⇒ **分叉** ⇒ 下次启动必然校验失败 ⇒ 审计永久降级内存账。
/// 真实发生过（2026-10-25：3 条记录同一 `previous_hash`）。
fn shared_audit_sink(path: std::path::PathBuf) -> Arc<dyn AuditSink> {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static REGISTRY: OnceLock<Mutex<HashMap<std::path::PathBuf, Arc<dyn AuditSink>>>> =
        OnceLock::new();
    let registry = REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = registry
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(existing) = guard.get(&path) {
        return Arc::clone(existing);
    }
    let sink = open_persistent_sink(path.clone());
    guard.insert(path, Arc::clone(&sink));
    sink
}

/// 本进程已取的审计锁路径（模块级单一真源）。
///
/// ⚠️ 不能写成两个函数各自的 `static`：函数内 `static` 是**不同的变量**，
/// 记录与释放会看到不同容器（我第一版就这么错了，实测锁仍残留）。
static HELD_AUDIT_LOCKS: std::sync::OnceLock<std::sync::Mutex<Vec<std::path::PathBuf>>> =
    std::sync::OnceLock::new();

fn held_audit_locks() -> &'static std::sync::Mutex<Vec<std::path::PathBuf>> {
    HELD_AUDIT_LOCKS.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

/// 释放本进程持有的持久审计锁（在 `main` 正常返回路径上调用）。
///
/// 为什么需要显式释放：sink 存在**进程级注册表**（按路径单例）里，不随栈析构；
/// 进程正常返回时 `Drop` 不会跑 ⇒ `governance-audit.lock` 会永久残留
/// （实测 2026-10-25：正常退出后仍留 1 个）。陈旧锁接管虽能兜住，但那是垃圾文件。
///
/// 只删**自己持有的**那些锁；进程被强杀（SIGKILL）时本函数不跑，仍靠陈旧锁接管。
pub fn release_audit_lock() {
    let mut guard = held_audit_locks()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    for path in guard.drain(..) {
        let _ = std::fs::remove_file(&path);
    }
}

/// 记录本进程已取的锁路径（供 [`release_audit_lock`] 清理）。
fn remember_audit_lock(path: &std::path::Path) {
    let mut guard = held_audit_locks()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let lock_path = path.with_extension("lock");
    if !guard.contains(&lock_path) {
        guard.push(lock_path);
    }
}

/// 打开持久 sink；**坏链（历史分叉）隔离后重建**，其余错误如实降级内存账。
///
/// 隔离而非删除：坏链是证据（谁在何时把链写分叉），改名保留在原地旁边。
/// 隔离后重建空链 = 审计恢复持久；不隔离就只能永久降级（当前的行为）。
fn open_persistent_sink(path: std::path::PathBuf) -> Arc<dyn AuditSink> {
    match apeireth_gateway::FileChainAuditSink::open(path.clone()) {
        Ok(sink) => {
            remember_audit_lock(&path);
            Arc::new(sink)
        }
        Err(error) if error.contains("hash mismatch") => {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let quarantine = path.with_extension(format!("forked-{stamp}.jsonl"));
            match std::fs::rename(&path, &quarantine) {
                Ok(()) => {
                    eprintln!(
                        "[governance-audit] 审计链分叉（多写者历史，非本进程造成）: {error}\n\
                         [governance-audit] 已把坏链隔离到 {}（保留证据），新建空链继续持久化",
                        quarantine.display()
                    );
                    match apeireth_gateway::FileChainAuditSink::open(path) {
                        Ok(sink) => Arc::new(sink),
                        Err(retry) => {
                            eprintln!("[governance-audit] 重建审计链失败: {retry}; 降级内存账 (审计不持久, 如实)");
                            Arc::new(InMemoryAuditSink::new())
                        }
                    }
                }
                Err(rename_error) => {
                    eprintln!(
                        "[governance-audit] 审计链分叉且隔离失败 ({rename_error}); 降级内存账 (审计不持久, 如实)"
                    );
                    Arc::new(InMemoryAuditSink::new())
                }
            }
        }
        Err(error) => {
            eprintln!(
                "[governance-audit] 持久审计账本不可用: {error}; 降级内存账 (审计不持久, 如实)"
            );
            Arc::new(InMemoryAuditSink::new())
        }
    }
}

#[cfg(test)]
mod audit_sink_sharing_tests {
    use super::*;

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-cli-audit-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("mkdir");
        dir.join("governance-audit.jsonl")
    }

    /// **同进程同一账本必须共用同一个 sink**（否则两个 sink 各写各的 ⇒ 分叉）。
    #[test]
    fn same_ledger_shares_one_sink_in_process() {
        let path = scratch("share");
        let first = shared_audit_sink(path.clone());
        let second = shared_audit_sink(path.clone());
        assert!(
            Arc::ptr_eq(&first, &second),
            "同路径必须返回同一个 sink 实例（单写者）"
        );
    }

    /// 不同账本互不干扰（不是全局单例，是按路径共享）。
    #[test]
    fn different_ledgers_get_distinct_sinks() {
        let first = shared_audit_sink(scratch("a"));
        let second = shared_audit_sink(scratch("b"));
        assert!(!Arc::ptr_eq(&first, &second));
    }

    /// **坏链（历史分叉）隔离 + 重建**：坏链改名保留（证据不灭），新链可持久化。
    ///
    /// 这是真实事故的恢复路径：多写者历史把多条记录写成同一 `previous_hash`
    /// ⇒ 之后每次启动都拒开 ⇒ 审计永久降级内存账。隔离后审计恢复持久。
    #[test]
    fn forked_ledger_is_quarantined_and_audit_recovers() {
        use apeireth_core::kernel::Timestamp;
        use apeireth_governance::AuditHashChain;

        let path = scratch("forked");
        // 用真链生成两条**合法**记录，再把第二条的 previous_hash 改成与第一条同源
        //（= 分叉的样子：两个记录都挂在同一个链尾上）。
        let mut chain = AuditHashChain::new();
        let first = chain.append("a", "s", Timestamp::now()).clone();
        let second = chain.append("b", "s", Timestamp::now()).clone();
        let mut second_value = serde_json::to_value(&second).expect("to_value");
        second_value["previous_hash"] = serde_json::Value::String(first.previous_hash.clone());
        let bad = format!(
            "{}\n{}\n",
            serde_json::to_string(&first).expect("r1"),
            serde_json::to_string(&second_value).expect("r2")
        );
        std::fs::write(&path, &bad).expect("写坏链");

        let sink = open_persistent_sink(path.clone());
        let dir = path.parent().expect("parent").to_path_buf();
        let quarantined: Vec<_> = std::fs::read_dir(&dir)
            .expect("read_dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().contains("forked-"))
            .collect();
        assert_eq!(quarantined.len(), 1, "坏链应被隔离一条");
        let kept = std::fs::read_to_string(quarantined[0].path()).expect("读隔离文件");
        assert_eq!(kept, bad, "隔离文件必须是原坏链（证据不灭）");
        assert!(
            sink.append("c", "s", Timestamp::now()).is_ok(),
            "重建后的新链必须可追加（审计恢复持久化）"
        );
    }
}

#[cfg(test)]
mod audit_lock_release_tests {
    use super::*;

    /// 正常退出路径必须把锁文件清掉（否则用户数据目录里永久残留垃圾文件）。
    ///
    /// 真机事故：进程级注册表让 sink 活到进程结束 ⇒ `Drop` 不跑 ⇒
    /// `governance-audit.lock` 每次运行都留一个（实测 2026-10-25）。
    #[test]
    fn release_removes_the_lock_file_recorded_by_the_sink() {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-audit-lock-release-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let ledger = dir.join("governance-audit.jsonl");
        let sink = apeireth_gateway::FileChainAuditSink::open(ledger.clone()).expect("open");
        remember_audit_lock(&ledger);
        let lock_path = ledger.with_extension("lock");
        assert!(lock_path.exists(), "打开 sink 后应有锁文件");
        release_audit_lock();
        assert!(!lock_path.exists(), "释放后锁文件应被删除");
        drop(sink);
    }
}
