//! 自述面「记忆账本」探测口（从 `lib.rs` 平移：该文件在 arch_guard 尺寸基线内）。
//!
//! 只读计数：会话 / 记忆 / 保护 / 教训 / 保鲜三真值 / 检索调权两真值 /
//! **教训对象（Lesson）已验证与候选计数**。任一子探测失败 = 该字段显式 null + 原因，
//! 不编数（0 与「读不到」是两件事）。

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_tools_canonical::{MemoryLedgerStats, StatusProbe};

pub(crate) fn self_status_ledger_probe(
    cognitive_db: PathBuf,
    lessons_root: PathBuf,
    retrieval_tuning_log: PathBuf,
) -> apeireth_tools_canonical::StatusProbe<apeireth_tools_canonical::MemoryLedgerStats> {
    use apeireth_memory::reflexion::ReflexionStore as _;
    Arc::new(move || {
        let store = apeireth_memory::SqliteMemoryStore::open(&cognitive_db)
            .map_err(|error| format!("memory store open failed: {error}"))?;
        let counts = store
            .ledger_counts()
            .map_err(|error| format!("memory ledger query failed: {error}"))?;
        let reflexion = apeireth_memory::reflexion::FileReflexionStore::new(lessons_root.clone());
        let (lessons, reason) = match reflexion.list_reflections() {
            Ok(reflections) => (Some(reflections.len() as u64), None),
            Err(error) => (None, Some(format!("lesson store read failed: {error}"))),
        };
        // V4-b 保鲜回路三真值 (缺源显式 null + 原因, 不编数)。
        let (pending_ingest, last_fold_at_ms, dedup_skipped, freshness_reason) =
            match store.freshness_status() {
                Ok(status) => (
                    Some(status.pending_ingest),
                    status.last_fold_at_ms,
                    Some(status.dedup_skipped),
                    None,
                ),
                Err(error) => (
                    None,
                    None,
                    None,
                    Some(format!("freshness status query failed: {error}")),
                ),
            };
        // V4-c 检索调权两真值 (缺档 = 有账零调权如实 0; 读失败 = 显式 null +
        // 原因, 不编移动)。
        let (retrieval_tuning_count, last_retrieval_tuning, tuning_reason) =
            match apeireth_runtime_assembly::read_retrieval_tuning_log(&retrieval_tuning_log) {
                Ok(summary) => {
                    let (count, last) =
                        apeireth_runtime_assembly::memory_ledger_tuning_fields(&summary);
                    (count, last, None)
                }
                Err(error) => (None, None, Some(error)),
            };
        // 教训对象（Lesson）：已验证 / 候选分列。读失败 = 显式 null + 原因，
        // 不编数（0 与「读不到」是两件事）。
        let (verified_lessons, candidate_lessons, lesson_reason) =
            match store.count_lessons_by_status() {
                Ok(counts) => (Some(counts.verified), Some(counts.candidate), None),
                Err(error) => (
                    None,
                    None,
                    Some(format!("lesson object query failed: {error}")),
                ),
            };
        Ok(apeireth_tools_canonical::MemoryLedgerStats {
            sessions: Some(counts.sessions),
            memories: Some(counts.memories),
            protected: Some(counts.protected),
            lessons,
            pending_ingest,
            last_fold_at_ms,
            dedup_skipped,
            retrieval_tuning_count,
            last_retrieval_tuning,
            verified_lessons,
            candidate_lessons,
            reason: reason
                .or(freshness_reason)
                .or(tuning_reason)
                .or(lesson_reason),
        })
    })
}
