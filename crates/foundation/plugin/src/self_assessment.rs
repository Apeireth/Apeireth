//! P-arch (2026-08-27): 场景 D 例 2 自我评估 SelfAssessmentCache trait.
//!
//! **位置**: trait 在 `apeireth-plugin` (foundation), impl 留 v2.0.0-rc (RC-4 任务).
//! 与 MemoryBackend/Experience/Perception/PreferenceStore 同位: 都是 capability 抽象.
//!
//! **场景** (per `docs/04-internal/scene-d-v2-plan.md` §2.2 + `v2.0.0-rc-roadmap.md` RC-4):
//! - runtime 在每 100 turn 触发 `SelfAssessor::assess(turn_context)`
//! - 调 LLM (multi-instance per scene-d §5 决策 1, 不同 model 隔离)
//! - 写 `SelfAssessment` 到 `SelfAssessmentCache`
//! - runtime 每 turn 起始读 `recent_for_task(task_id, 5)`, 如果 alignment < 0.6
//!   → 触发 `DeviationReport` 给主人
//!
//! **不**用 13 键 verdict cache: 二者职责分 (self_assessment = 长程 AI 行为漂移检测;
//! verdict cache = 13 键哲学决策 cache, runtime 不强制 — per 2026-08-27 5 维分析降级)
//!
//! **0 装 PASS**: trait 是 0 装, RC-4 任务在 v2.0.0-rc.1 启动时实现真 SQLite backend
//!
//! **3 阶审查** (O-6 锚 #9, commit message 必写明):
//! 1. 总体: 与 5 件 capability 抽象在 foundation 集中; 场景 D 例 2 路线按 scene-d §2.2
//! 2. 系统: trait 在 foundation, impl 在 engine (单向, 与 plugin 体系一致);
//!    复用 SQLite storage (per scene-d §5 决策 1) 不开新 DB 连接管理
//! 3. 架构: runtime 拿 `Arc<dyn SelfAssessmentStore>` 注入, 不直接 import impl crate
//!
//! **v1 compat**: trait 是新增, 0 破现有 100+ consumer

use serde::{Deserialize, Serialize};

use apeireth_core::kernel::SessionId;
use crate::memory_backend::CapabilityResult;

/// 自我评估结果 (per scene-d §2.2):
/// alignment (与主人期望对齐度) + quality (输出质量) + deviations (偏差列表, JSON)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelfAssessment {
    /// 唯一 id
    pub id: String,
    /// 评估轮次 (每 100 turn 一次, 或 tool 失败触发)
    pub round: u32,
    /// 所属 session
    pub session_id: SessionId,
    /// 任务 id (long-horizon task)
    pub task_id: String,
    /// 与主人期望的对齐度 (0.0-1.0)
    /// 0.6 以下 → 触发 DeviationReport (per scene-d §2.2 阈值)
    pub alignment: f64,
    /// 输出质量 (0.0-1.0)
    pub quality: f64,
    /// 偏差列表 (JSON array of {kind, evidence, severity})
    pub deviations: serde_json::Value,
    /// 评估时间戳 (epoch millis)
    pub assessed_at: i64,
    /// 评估实例 id (multi-instance 隔离标识, per scene-d §5 决策 1)
    pub reviewer_id: String,
}

/// 评估触发原因 (scene-d §2.2: 时间触发 / 事件触发)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssessmentTrigger {
    /// 时间驱动 (turn_counter % 100 == 0)
    TimeBased,
    /// 事件驱动 (tool 失败)
    EventBased(String),
}

/// 自我评估 cache trait (场景 D 例 2 入口)
pub trait SelfAssessmentStore: Send + Sync {
    /// 写入一条 SelfAssessment
    fn record(&self, sa: &SelfAssessment) -> CapabilityResult<()>;

    /// 查某 task_id 的最近 N 条评估 (按时间升序, 末尾 N 条)
    /// runtime 在每 turn 起始读 5 条, 如果 alignment 最低 < 0.6 → DeviationReport
    fn recent_for_task(
        &self,
        task_id: &str,
        limit: u32,
    ) -> CapabilityResult<Vec<SelfAssessment>>;

    /// 查某 task_id 的最近 1 条 alignment (runtime hot-path 快速查)
    /// 0 装 PASS: 真 backend rc 阶段实现, 索引 (task_id, assessed_at) O(log N)
    fn latest_alignment(&self, task_id: &str) -> CapabilityResult<Option<f64>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 0 装 PASS: SelfAssessment 可构造, alignment 不假装 100%
    #[test]
    fn self_assessment_construction_works() {
        let sid = SessionId::new();
        let sa = SelfAssessment {
            id: "sa-1".into(),
            round: 1,
            session_id: sid,
            task_id: "task-001".into(),
            alignment: 0.85, // 0 装: 真实评估值, 不是 1.0
            quality: 0.78,
            deviations: serde_json::json!([]),
            assessed_at: 1_700_000_000,
            reviewer_id: "reviewer-claude-3-5".into(),
        };
        assert_eq!(sa.alignment, 0.85);
        assert_eq!(sa.round, 1);
    }

    /// 0 装 PASS: AssessmentTrigger 序列化兼容 (runtime 触发时用)
    #[test]
    fn assessment_trigger_serde_roundtrip() {
        let trigger = AssessmentTrigger::TimeBased;
        let json = serde_json::to_string(&trigger).expect("serialize");
        assert!(json.contains("TimeBased"));

        let event = AssessmentTrigger::EventBased("tool_failed".into());
        let json2 = serde_json::to_string(&event).expect("serialize");
        assert!(json2.contains("tool_failed"));
    }

    /// 0 装 PASS: SelfAssessmentStore trait 是 0 装占位 — 没 impl, 仅 trait 边界
    /// 真 SQLite impl 在 v2.0.0-rc RC-4 任务
    #[test]
    fn self_assessment_store_trait_is_zero_implementation() {
        // 验证 trait 定义存在, 方法签名正确
        fn _check_trait_exists<T: SelfAssessmentStore>() {}
        // 编译通过 = trait 边界完整, 0 装: 没真正可用的 impl
    }
}