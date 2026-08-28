//! P-arch (2026-08-27): 场景 D 例 1 主人偏好 PreferenceStore trait.
//!
//! **位置**: trait 在 `apeireth-plugin` (foundation), impl 留 v2.0.0-rc (RC-3 任务).
//! 与 MemoryBackend/Experience/Perception 同位: 都是 capability 抽象.
//!
//! **场景** (per `docs/04-internal/scene-d-v2-plan.md` §2.1):
//! - 持久化主人偏好 (按时间线 + 主题标签)
//! - 检索接口: runtime 在每个 turn 调 `recall_for_context(session_id, current_topic)` 注入 transcript
//! - 写入触发: AI 在每个 turn 末调 `record(stance, evidence)` (写入点 explicit, runtime 强制;
//!   不是 AI 自己决定写不写 — 防止 AI 自我偏置, 这是 O-5 不假装锚在 trait 层面的兑现)
//!
//! **不**用 verdict cache (13 键) 做主人偏好: 二者职责分 (preference 是事实记录;
//! verdict cache 是 13 键哲学决策 cache, runtime 不强制 — per 2026-08-27 5 维分析降级)
//!
//! **0 装 PASS**: trait 是 0 装, RC-3 任务在 v2.0.0-rc.1 启动时实现真 SQLite backend
//! (复用 `apeireth-storage` 的 SQLiteConnectionPool, per scene-d §5 决策 1)
//!
//! **3 阶审查** (O-6 锚 #9, commit message 必写明):
//! 1. 总体: 与 4 件 capability 抽象 (MemoryBackend/Experience/Perception/CredentialResolver)
//!    在 foundation 集中; 场景 D 路线按 scene-d §2.1
//! 2. 系统: trait 在 foundation, impl 在 engine (单向, 与 plugin 体系一致);
//!    复用 SQLite storage (per scene-d §5 决策 1) 不开新 DB 连接管理
//! 3. 架构: runtime 拿 `Arc<dyn PreferenceStore>` 注入, 不直接 import impl crate
//!
//! **v1 compat**: trait 是新增, 0 破现有 100+ consumer

use serde::{Deserialize, Serialize};

use crate::memory_backend::CapabilityResult;
use apeireth_core::kernel::SessionId;

/// 主人偏好 (per scene-d §2.1):
/// 事实陈述 (stance) + 证据引用 (evidence_refs: 哪些 episode / session 来源) +
/// 创建时间 + 置信度 (AI 写入时不假装 100% 准确, 标 confidence 范围 0.0-1.0).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserPreference {
    /// 唯一 id (v1 实践: SHA-256(session_id + topic)[:16] hex)
    pub id: String,
    /// 所属 session (preference 跨 session 共享 — 同主人在不同 session 表态一致)
    pub session_id: SessionId,
    /// 偏好主题 (e.g. "用 Rust 不用 Python", "代码风格 = KISS", "回复用中文")
    pub topic: String,
    /// 立场 (e.g. "主人偏好 Rust 因为性能 + 类型安全")
    pub stance: String,
    /// 证据引用 (哪些 episode / note 提供了这个偏好的依据)
    pub evidence_refs: Vec<String>,
    /// 创建时间戳 (epoch millis)
    pub created_at: i64,
    /// 置信度 (0.0-1.0) — 0 装: AI 写入时**必**给真实置信度, 不假装 100%
    pub confidence: f64,
    /// 标签 (用于检索/分类)
    pub tags: Vec<String>,
}

/// 偏好 store trait (场景 D 例 1 入口)
pub trait PreferenceStore: Send + Sync {
    /// 写入一条主人偏好
    fn record(&self, pref: &UserPreference) -> CapabilityResult<()>;

    /// 按 session + 当前 topic 检索 top-N 偏好 (按 confidence desc + 时间衰减权重)
    /// **runtime 必**在每个 turn 调一次, 把结果注入 transcript (scene-d §2.1 设计)
    fn recall_for_context(
        &self,
        session_id: &SessionId,
        current_topic: &str,
        limit: u32,
    ) -> CapabilityResult<Vec<UserPreference>>;

    /// 删除某条偏好 (主人撤回时调 — 不能 假装"主人没说过", 0 装 PASS: 真删 + 审计)
    fn forget(&self, pref_id: &str) -> CapabilityResult<()>;

    /// 按 session 列出所有偏好 (主人查看 / 导出用)
    fn list_for_session(&self, session_id: &SessionId) -> CapabilityResult<Vec<UserPreference>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 0 装 PASS: UserPreference 可构造, confidence 不假装 100%
    #[test]
    fn user_preference_construction_works() {
        let sid = SessionId::new();
        let pref = UserPreference {
            id: "pref-1".into(),
            session_id: sid,
            topic: "language preference".into(),
            stance: "主人偏好 Rust 因为性能 + 类型安全".into(),
            evidence_refs: vec!["ep-1".into(), "ep-42".into()],
            created_at: 1_700_000_000,
            // 0 装: 0.85 真实置信度, 不是 1.0
            confidence: 0.85,
            tags: vec!["language".into(), "rust".into()],
        };
        assert_eq!(pref.confidence, 0.85);
        assert_eq!(pref.evidence_refs.len(), 2);
    }

    /// 0 装 PASS: PreferenceStore trait 是 0 装占位 — 没 impl, 仅 trait 边界
    /// 真 SQLite impl 在 v2.0.0-rc RC-3 任务
    #[test]
    fn preference_store_trait_is_zero_implementation() {
        // 验证 trait 定义存在, 方法签名正确
        fn _check_trait_exists<T: PreferenceStore>() {}
        // 编译通过 = trait 边界完整, 0 装: 没真正可用的 impl
    }
}
