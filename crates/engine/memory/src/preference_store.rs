//! P-arch (2026-08-27): 场景 D 例 1 PreferenceStore 0 装 impl (RC-3).
//!
//! **位置**: impl 在 `apeireth-memory` (engine), trait 在 `apeireth-plugin` (foundation).
//! 单向依赖: memory → plugin. 0 装: NoopPreferenceStore 不真正持久化, 仅 trait 接口落地.
//!
//! **0 装 PASS** (v2.0 alpha): 当前 NoopPreferenceStore
//! - `record` → 返 Ok 不持久化 (caller 可能以为是 fail-soft; 真 SQLite impl 在 RC-3)
//! - `recall_for_context` → 返空 Vec (0 装: 没有偏好数据)
//! - `forget` → 返 Ok (假装"忘了", 实际上什么都没存)
//! - `list_for_session` → 返空 Vec
//!
//! **真 SQLite impl** (v2.0.0-rc RC-3): SQLitePreferenceStore 加:
//! - 新表 `user_preferences` (session_id, topic, stance TEXT, confidence,
//!   evidence_refs TEXT (JSON array), created_at)
//! - index: `(session_id, confidence DESC)` for `recall_for_context` 加速
//! - migration v6 (per `crates/engine/memory/src/migrations.rs` 数组顺序)
//!
//! **3 阶审查** (O-6 锚 #9):
//! 1. 总体: 与 scene-d §2.1 + v2.0.0-rc-roadmap.md RC-3 对齐; 0 装 alpha 阶段仅 trait impl 落地
//! 2. 系统: impl 在 engine, trait 在 foundation (单向, 与 plugin 体系一致); SQLite impl 复用现有 storage
//! 3. 架构: runtime 拿 `Arc<dyn PreferenceStore>` 注入; 0 装: alpha 用 Noop, rc 用 SQLite
//!
//! **v1 compat**: 100+ consumer 0 破 (trait 是新增)

use apeireth_core::kernel::SessionId;
use apeireth_plugin::memory_backend::CapabilityResult;
use apeireth_plugin::preference::{PreferenceStore, UserPreference};

/// 0 装 PASS (v2.0 alpha): NoopPreferenceStore
/// 不持久化偏好, 返 Ok / 空 Vec. 真 SQLite impl 在 v2.0.0-rc RC-3.
///
/// 0 装诚实标注:
/// - `record` 真**没**存 (调用方需要知, 否则忘了"我没存")
/// - `recall_for_context` 永返空 (调用方拿不到偏好 = 视为"主人没偏好")
/// - `forget` 真**没**忘 (返 Ok 是契约要求, 但实际不存所以也无须忘)
/// - `list_for_session` 永返空
#[derive(Debug, Clone, Default)]
pub struct NoopPreferenceStore;

impl PreferenceStore for NoopPreferenceStore {
    fn record(&self, _pref: &UserPreference) -> CapabilityResult<()> {
        // 0 装 PASS: 不假装有持久化, 直接返 Ok. 调用方需要文档知道 "alpha 阶段没存".
        Ok(())
    }

    fn recall_for_context(
        &self,
        _session_id: &SessionId,
        _current_topic: &str,
        _limit: u32,
    ) -> CapabilityResult<Vec<UserPreference>> {
        // 0 装: 永返空 (alpha 阶段没偏好数据)
        Ok(Vec::new())
    }

    fn forget(&self, _pref_id: &str) -> CapabilityResult<()> {
        // 0 装: 返 Ok (假装"忘了", 但实际没存, 所以也无须忘)
        Ok(())
    }

    fn list_for_session(&self, _session_id: &SessionId) -> CapabilityResult<Vec<UserPreference>> {
        // 0 装: 永返空
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::SessionId;

    fn pref_with_confidence(sid: SessionId, confidence: f64) -> UserPreference {
        UserPreference {
            id: "pref-1".into(),
            session_id: sid,
            topic: "language preference".into(),
            stance: "主人偏好 Rust 因为性能 + 类型安全".into(),
            evidence_refs: vec!["ep-1".into()],
            created_at: 1_700_000_000,
            confidence,
            tags: vec!["language".into()],
        }
    }

    /// 0 装 PASS: NoopPreferenceStore::record 返 Ok (0 装: 不假装持久化)
    #[test]
    fn noop_record_returns_ok_without_persistence() {
        let store = NoopPreferenceStore::default();
        let sid = SessionId::new();
        let pref = pref_with_confidence(sid, 0.85);
        let result = store.record(&pref);
        assert!(result.is_ok());

        // 0 装: 再 recall 应该返空 (没存)
        let recalled = store
            .recall_for_context(&sid, "language preference", 10)
            .unwrap();
        assert!(
            recalled.is_empty(),
            "NoopPreferenceStore 不持久化, 应当 recall 空"
        );
    }

    /// 0 装 PASS: NoopPreferenceStore::recall_for_context 永返空
    #[test]
    fn noop_recall_always_empty() {
        let store = NoopPreferenceStore::default();
        let sid = SessionId::new();
        let _ = store.record(&pref_with_confidence(sid, 0.85));
        let r1 = store.recall_for_context(&sid, "language", 10).unwrap();
        let r2 = store.recall_for_context(&sid, "code style", 5).unwrap();
        let r3 = store.recall_for_context(&sid, "", 1).unwrap();
        assert!(r1.is_empty());
        assert!(r2.is_empty());
        assert!(r3.is_empty());
    }

    /// 0 装 PASS: NoopPreferenceStore::forget 返 Ok (假装"忘了", 实际没存)
    #[test]
    fn noop_forget_returns_ok_without_state() {
        let store = NoopPreferenceStore::default();
        let result = store.forget("any-id");
        assert!(result.is_ok());
    }

    /// 0 装 PASS: NoopPreferenceStore::list_for_session 永返空
    #[test]
    fn noop_list_for_session_always_empty() {
        let store = NoopPreferenceStore::default();
        let sid = SessionId::new();
        let result = store.list_for_session(&sid);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    /// trait 是 Send + Sync (NoopPreferenceStore 默认 0 装实现满足)
    /// (编译器检查: trait bounds)
    fn _assert_send_sync<T: Send + Sync>() {}
    #[test]
    fn noop_preference_store_is_send_sync() {
        _assert_send_sync::<NoopPreferenceStore>();
    }
}
