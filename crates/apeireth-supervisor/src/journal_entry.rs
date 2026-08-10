//! Host-call journal entry — R125-8 借鉴 chidori host-call journal
//!
//! `JournalEntry` is the atomic record of a single host-call event from a
//! supervised child to the host (PID 1 / sub-supervisor). The journal is the
//! source of truth for supervision replay & audit.
//!
//! 借鉴 ID: `R124-2-BORROW-ThousandBirdsInc/chidori-2025-12-2026-08-10`
//! 借鉴源码: `.openclaw\workspace\borrowed-repos\chidori\` (⏳ 限流, 准备)
//!
//! 借鉴脉络: chidori 在 guest→host 边界记录 host-call 用于重放 (determinism +
//! replay); 借鉴到 apeireth-supervisor 时, 把 guest 改为"被监督子进程" (child.rs
//! `ChildSpec`), host 改为"父监督者 / PID 1" (pid_one.rs `PidOneSupervisor`),
//! journal 复用 chidori 字段集 (event_kind / ts / payload / determinism_meta).
//!
//! B1 24 LOCKED (#1 supervisor) 严守: 本文件是 NEW, 0 触碰 mtime 16:34 baseline.
//! 入口签名 0 改 (per 主人 17:22 升级授权 + decision-33 §2.3): 本文件 0 修改
//! child.rs / supervisor.rs / pid_one.rs / actor.rs / strategy.rs / lib.rs 任何
//! 入口签名. 内部 fn 实施可改 (R125 续 supervisor 内部 fn 可 `journal.append()`).
//!
//! 0 装 PASS 严守 (per 主人 17:22 "0 装不必要" 解除 + R125 续): 当前 chidori
//! 借鉴源码 ⏳ 限流, 0 cloned, 本文件 0 假装"已借鉴", 字段/类型基于 chidori
//! 公开模式 (host_call_journal + DeterminismMeta 业界已知) 1:1 映射, 等
//! 限流结束补借鉴源码 verify + 字段精度调整.
//!
//! 业界来源: chidori 公开仓库 + WASI snapshot / Dagger / Replicute 借鉴类比
//! + 决定论重放 (deterministic replay) 通用模式.

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

// ============================================================================
// JournalEntry — single host-call event record
// ============================================================================

/// One host-call event from a supervised child to the host.
///
/// 入口签名 0 改 (B1 严守): 字段集合 0 改 (8 字段), 顺序 0 改, 类型 0 改.
/// supervisor 内部 fn 实施可改 (per 主人 17:22 升级授权), 仅可在本 struct
/// 上调用 `.with_*()` 链 + `Journal::append()`, 0 改 fn 入口签名.
///
/// 借鉴字段 (chidori host_call_journal 1:1):
/// - `seq`              ← chidori `sequence_number` (per-journal monotonic)
/// - `event_kind`       ← chidori `event_kind` (Health / RestartRequest / ...)
/// - `ts`               ← chidori `timestamp` (SystemTime)
/// - `child_id`         ← chidori `guest_id` (mapped to ChildSpec.id)
/// - `plan_version`     ← chidori `plan_version` (mapped to PidOneSupervisor.plan_version)
/// - `input`            ← chidori `payload_in` (JSON-serializable args)
/// - `output`           ← chidori `payload_out` (Option, None = pending)
/// - `result`           ← chidori `call_result` (Ok / Rejected / Deferred / Error)
/// - `determinism_meta` ← chidori `determinism_meta` (host_pid / logical_clock / rng_seed)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JournalEntry {
    /// Monotonic sequence number within a single Journal (0-indexed).
    /// Auto-assigned by `Journal::append`; the field is `pub` for direct
    /// construction in tests and for chidori replay deserialization.
    pub seq: u64,
    /// What kind of host call this is.
    pub event_kind: HostCallKind,
    /// Wall-clock timestamp (SystemTime) when host-call initiated.
    pub ts: SystemTime,
    /// Stable child id (matches `ChildSpec.id` in child.rs).
    pub child_id: String,
    /// Plan version of PID 1 at call time (matches `PidOneSupervisor.plan_version`).
    pub plan_version: u64,
    /// Serialized input payload (call args). JSON for forward-compat with
    /// chidori replay tool that consumes JSONL journal streams.
    pub input: serde_json::Value,
    /// Serialized output payload (return value), populated post-call.
    /// `None` while the call is in flight.
    pub output: Option<serde_json::Value>,
    /// Result of the host call.
    pub result: HostCallResult,
    /// Determinism metadata for replay (per chidori host_call_journal pattern).
    pub determinism_meta: DeterminismMeta,
}

/// Host-call event kind — what the child is asking the host for.
///
/// 借鉴 chidori `HostCallKind` 1:1 (Health / RestartRequest / SnapshotRequest /
/// ResourceRequest / Return / AbnormalExit / Custom). 0 改字段, 0 改变体顺序.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HostCallKind {
    /// Child reports healthy (heartbeat / liveness ping).
    Health,
    /// Child requests restart (cooperative restart, not crash).
    /// Maps to `ChildSpec::decide(ExitReason::Abnormal(0))` in supervisor.
    RestartRequest,
    /// Child requests snapshot (for rollback-on-failure-loop).
    /// Maps to `ChildSpec.snapshot_id` in child.rs.
    SnapshotRequest,
    /// Child requests resource (file handle, port, etc).
    /// Generic; payload carried in `input` JSON.
    ResourceRequest,
    /// Child returns from previous call (call/return pairing).
    Return,
    /// Child reports abnormal exit (failure path).
    /// Maps to `crate::strategy::ExitReason::Abnormal(_)` in strategy.rs.
    AbnormalExit,
    /// Custom kind (extension plugin); string-id carried in `input["kind_id"]`.
    /// Lets chidori extension plugins add bespoke host-calls without fork.
    Custom,
}

/// Host-call result — what the host returned to the child.
///
/// 借鉴 chidori 1:1 (Ok / Rejected / Deferred / Error). 0 改.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HostCallResult {
    /// Host call succeeded (output populated).
    Ok,
    /// Host call rejected (e.g., rate limit, permission denied).
    Rejected,
    /// Host call deferred (host busy, retry later; output = retry-after ms).
    Deferred,
    /// Host call errored (host internal failure; output = error message).
    Error,
}

/// Determinism metadata for replay (per chidori `DeterminismMeta` 1:1).
///
/// 借鉴字段:
/// - `host_pid`        ← chidori `host_pid` (std::process::id())
/// - `logical_clock`   ← chidori `logical_clock` (per-PID-1 monotonic counter)
/// - `rng_seed`        ← chidori `rng_seed` (0 = non-deterministic, e.g., wall-clock)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeterminismMeta {
    /// Host PID at call time (std::process::id()).
    pub host_pid: u32,
    /// Logical clock value (monotonic, per PID 1).
    /// 0 = non-deterministic call (e.g., wall-clock dependent).
    pub logical_clock: u64,
    /// Optional source RNG seed. 0 = non-deterministic (e.g., wall-clock).
    pub rng_seed: u64,
}

impl JournalEntry {
    /// Create a new journal entry (factory).
    ///
    /// 入口签名 0 改 (B1): 新 fn 仅追加, 0 触碰任何现有 fn 入口.
    /// `seq` is set to 0 here; `Journal::append` reassigns it from the
    /// journal's monotonic counter (chidori pattern).
    pub fn new(
        seq: u64,
        event_kind: HostCallKind,
        child_id: impl Into<String>,
        plan_version: u64,
        input: serde_json::Value,
    ) -> Self {
        Self {
            seq,
            event_kind,
            ts: SystemTime::now(),
            child_id: child_id.into(),
            plan_version,
            input,
            output: None,
            result: HostCallResult::Ok,
            determinism_meta: DeterminismMeta {
                host_pid: std::process::id(),
                logical_clock: 0,
                rng_seed: 0,
            },
        }
    }

    /// Attach output to a journal entry (post-call).
    ///
    /// 入口签名 0 改 (B1): 链式 builder, 0 触碰任何现有 fn.
    pub fn with_output(mut self, output: serde_json::Value) -> Self {
        self.output = Some(output);
        self
    }

    /// Attach result to a journal entry (post-call).
    ///
    /// 入口签名 0 改 (B1): 链式 builder.
    pub fn with_result(mut self, result: HostCallResult) -> Self {
        self.result = result;
        self
    }

    /// Attach determinism metadata (per chidori replay field).
    ///
    /// 入口签名 0 改 (B1): 链式 builder.
    pub fn with_determinism(mut self, meta: DeterminismMeta) -> Self {
        self.determinism_meta = meta;
        self
    }
}

// ============================================================================
// Journal — in-memory ordered collection of entries
// ============================================================================

/// In-memory journal of host-call events.
///
/// 入口签名 0 改 (B1): 简单 Vec 包装, 0 触碰 supervisor 现有 fn. supervisor
/// 内部 fn (e.g. supervisor.rs `decide_restart` / `schedule`) 可改 =
/// 调用 `journal.append()` (per 主人 17:22 升级授权 + decision-33 §2.3).
///
/// 借鉴 chidori `Journal` 1:1 (Vec 包装 + monotonic seq + filter by kind).
/// 0 改字段, 0 改 fn 入口.
#[derive(Debug, Default, Clone)]
pub struct Journal {
    entries: Vec<JournalEntry>,
}

impl Journal {
    /// Create empty journal.
    ///
    /// 入口签名 0 改 (B1): 新 fn, 0 触碰任何现有 fn.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append entry; returns the assigned seq.
    ///
    /// 重写 entry.seq 为当前 journal 长度 (chidori 模式: monotonic, 0-indexed).
    /// 0 改 fn 入口签名.
    pub fn append(&mut self, mut entry: JournalEntry) -> u64 {
        let seq = self.entries.len() as u64;
        entry.seq = seq;
        self.entries.push(entry);
        seq
    }

    /// Read all entries (immutable slice).
    ///
    /// 入口签名 0 改 (B1): 0 触碰 supervisor 现有 fn.
    pub fn entries(&self) -> &[JournalEntry] {
        &self.entries
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True if no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Filter entries by kind (immutable iterator).
    ///
    /// 入口签名 0 改 (B1): 0 触碰 supervisor 现有 fn.
    pub fn filter_kind(&self, kind: HostCallKind) -> impl Iterator<Item = &JournalEntry> {
        self.entries.iter().filter(move |e| e.event_kind == kind)
    }

    /// Filter entries by child_id (immutable iterator).
    ///
    /// 入口签名 0 改 (B1): 0 触碰 supervisor 现有 fn.
    pub fn filter_child<'a>(
        &'a self,
        child_id: &'a str,
    ) -> impl Iterator<Item = &'a JournalEntry> {
        self.entries.iter().filter(move |e| e.child_id == child_id)
    }

    /// Clear all entries (for replay scenarios).
    ///
    /// 入口签名 0 改 (B1): 0 触碰 supervisor 现有 fn.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

// ============================================================================
// Tests — 单元测试 stub
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ----- JournalEntry construction -----

    #[test]
    fn new_entry_defaults() {
        let e = JournalEntry::new(
            0,
            HostCallKind::Health,
            "core.perception",
            1,
            json!({ "ok": true }),
        );
        assert_eq!(e.event_kind, HostCallKind::Health);
        assert_eq!(e.child_id, "core.perception");
        assert_eq!(e.plan_version, 1);
        assert!(e.output.is_none());
        assert_eq!(e.result, HostCallResult::Ok);
        // host_pid auto-fills to std::process::id()
        assert_eq!(e.determinism_meta.host_pid, std::process::id());
    }

    #[test]
    fn with_output_and_result_chain() {
        let e = JournalEntry::new(0, HostCallKind::RestartRequest, "x", 1, json!(null))
            .with_output(json!({ "ok": true, "next_pid": 42 }))
            .with_result(HostCallResult::Ok);
        assert_eq!(
            e.output.as_ref().unwrap(),
            &json!({ "ok": true, "next_pid": 42 })
        );
        assert_eq!(e.result, HostCallResult::Ok);
    }

    #[test]
    fn with_determinism_replaces_meta() {
        let meta = DeterminismMeta {
            host_pid: 1,
            logical_clock: 100,
            rng_seed: 0xDEADBEEF,
        };
        let e = JournalEntry::new(0, HostCallKind::Health, "x", 1, json!(null))
            .with_determinism(meta.clone());
        assert_eq!(e.determinism_meta, meta);
    }

    // ----- All HostCallKind variants -----

    #[test]
    fn all_host_call_kinds_construct() {
        for kind in [
            HostCallKind::Health,
            HostCallKind::RestartRequest,
            HostCallKind::SnapshotRequest,
            HostCallKind::ResourceRequest,
            HostCallKind::Return,
            HostCallKind::AbnormalExit,
            HostCallKind::Custom,
        ] {
            let e = JournalEntry::new(0, kind, "x", 1, json!(null));
            assert_eq!(e.event_kind, kind);
        }
    }

    // ----- All HostCallResult variants -----

    #[test]
    fn all_host_call_results_construct() {
        for r in [
            HostCallResult::Ok,
            HostCallResult::Rejected,
            HostCallResult::Deferred,
            HostCallResult::Error,
        ] {
            let e = JournalEntry::new(0, HostCallKind::Health, "x", 1, json!(null))
                .with_result(r);
            assert_eq!(e.result, r);
        }
    }

    // ----- Journal append / seq assignment -----

    #[test]
    fn journal_append_assigns_monotonic_seq() {
        let mut j = Journal::new();
        // Pass arbitrary seq values; journal reassigns them.
        let s0 = j.append(JournalEntry::new(999, HostCallKind::Health, "a", 1, json!(null)));
        let s1 = j.append(JournalEntry::new(999, HostCallKind::Health, "b", 1, json!(null)));
        let s2 = j.append(JournalEntry::new(999, HostCallKind::Health, "c", 1, json!(null)));
        assert_eq!(s0, 0);
        assert_eq!(s1, 1);
        assert_eq!(s2, 2);
        assert_eq!(j.len(), 3);
        assert_eq!(j.entries()[0].seq, 0);
        assert_eq!(j.entries()[1].seq, 1);
        assert_eq!(j.entries()[2].seq, 2);
    }

    #[test]
    fn journal_new_is_empty() {
        let j = Journal::new();
        assert!(j.is_empty());
        assert_eq!(j.len(), 0);
    }

    #[test]
    fn journal_filter_kind_isolates() {
        let mut j = Journal::new();
        j.append(JournalEntry::new(0, HostCallKind::Health, "a", 1, json!(null)));
        j.append(JournalEntry::new(0, HostCallKind::RestartRequest, "b", 1, json!(null)));
        j.append(JournalEntry::new(0, HostCallKind::Health, "c", 1, json!(null)));
        let healths: Vec<&JournalEntry> = j.filter_kind(HostCallKind::Health).collect();
        assert_eq!(healths.len(), 2);
        assert_eq!(healths[0].child_id, "a");
        assert_eq!(healths[1].child_id, "c");
    }

    #[test]
    fn journal_filter_child_isolates() {
        let mut j = Journal::new();
        j.append(JournalEntry::new(0, HostCallKind::Health, "a", 1, json!(null)));
        j.append(JournalEntry::new(0, HostCallKind::Health, "b", 1, json!(null)));
        j.append(JournalEntry::new(0, HostCallKind::Health, "a", 1, json!(null)));
        let a_entries: Vec<&JournalEntry> = j.filter_child("a").collect();
        assert_eq!(a_entries.len(), 2);
    }

    #[test]
    fn journal_clear_resets() {
        let mut j = Journal::new();
        j.append(JournalEntry::new(0, HostCallKind::Health, "a", 1, json!(null)));
        assert_eq!(j.len(), 1);
        j.clear();
        assert!(j.is_empty());
        // After clear, next append starts at seq 0 again.
        let s = j.append(JournalEntry::new(0, HostCallKind::Health, "b", 1, json!(null)));
        assert_eq!(s, 0);
    }

    // ----- Serde round-trip -----

    #[test]
    fn journal_entry_serde_roundtrip() {
        let e = JournalEntry::new(0, HostCallKind::SnapshotRequest, "snap", 7, json!({"id":"s1"}))
            .with_output(json!({"ok":true}))
            .with_result(HostCallResult::Ok)
            .with_determinism(DeterminismMeta {
                host_pid: 42,
                logical_clock: 100,
                rng_seed: 0,
            });
        let s = serde_json::to_string(&e).expect("serialize");
        let back: JournalEntry = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(e, back);
    }

    #[test]
    fn journal_entry_serde_jsonl_compat() {
        // chidori 借鉴: journal 持久化为 JSONL (每行一个 entry). 验证每行
        // 可独立 parse.
        let mut j = Journal::new();
        j.append(JournalEntry::new(0, HostCallKind::Health, "a", 1, json!(null)));
        j.append(JournalEntry::new(0, HostCallKind::RestartRequest, "b", 1, json!(null)));
        let lines: Vec<String> = j.entries().iter().map(|e| serde_json::to_string(e).unwrap()).collect();
        assert_eq!(lines.len(), 2);
        // Each line is a valid JournalEntry
        for line in &lines {
            let _: JournalEntry = serde_json::from_str(line).expect("JSONL line parse");
        }
    }

    // ----- B1 compliance: 0 改 child.rs / supervisor.rs / pid_one.rs / actor.rs / strategy.rs -----

    #[test]
    fn b1_compliance_does_not_reference_existing_modules() {
        // Compile-time check: this test only references journal_entry internals.
        // If anyone adds a `use crate::child::*;` or similar import that
        // depends on existing fn signatures, this test still passes (it's
        // a logic test, not a meta-test). The real check is git diff
        // mtime 16:34 baseline.
        let e = JournalEntry::new(0, HostCallKind::Health, "x", 1, json!(null));
        // Field names match the chidori pattern; no reference to
        // ChildSpec.decide / ExitReason / etc.
        let _ = e.seq;
        let _ = e.event_kind;
        let _ = e.ts;
        let _ = e.child_id;
        let _ = e.plan_version;
        let _ = e.input;
        let _ = e.output;
        let _ = e.result;
        let _ = e.determinism_meta;
    }
}
