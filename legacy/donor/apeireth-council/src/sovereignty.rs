//! Sovereignty 集成接口 — `apeireth-sovereignty` 落地后实现本 trait
//!
//! **设计**: `apeireth-council` 不依赖 `apeireth-sovereignty` (该 crate 尚未落地);
//! 但通过 [`SovereigntyHook`] trait 留口, sovereignty crate 落地后实现本 trait 即可
//! 接入 council 的裁决事件流 (`CouncilEvent`).
//!
//! **事件类型**:
//! - `DeliberationStarted` — 智囊团审议开始
//! - `OpinionIssued` — 单个 opinion 产出
//! - `HoldTriggered` — 按住触发
//! - `SovereigntyAdjudicated` — 主权仲裁结果 (按住解除)
//! - `DeliberationCompleted` — 审议完成

use crate::advisor::AdvisorOpinion;
use crate::hold::HoldTrigger;
use crate::synthesis::SynthesisReport;
use serde::{Deserialize, Serialize};

/// 智囊团事件 (sovereignty 监听).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CouncilEvent {
    /// 审议开始
    DeliberationStarted {
        /// session ID
        session_id: String,
        /// query ID
        query_id: String,
        /// 开始时间 (epoch ms)
        started_at_ms: i64,
    },
    /// 单个 opinion 产出 (供 sovereignty 实时监控)
    OpinionIssued {
        /// session ID
        session_id: String,
        /// 意见
        opinion: AdvisorOpinion,
    },
    /// 按住触发
    HoldTriggered {
        /// session ID
        session_id: String,
        /// 按住原因
        trigger: HoldTrigger,
    },
    /// 主权仲裁完成 (按住解除)
    SovereigntyAdjudicated {
        /// session ID
        session_id: String,
        /// 是否放行
        released: bool,
        /// 主权决定理由
        rationale: String,
    },
    /// 审议完成
    DeliberationCompleted {
        /// session ID
        session_id: String,
        /// 综合报告
        report: SynthesisReport,
        /// 耗时 (ms)
        elapsed_ms: u64,
    },
}

/// Sovereignty Hook trait — `apeireth-sovereignty` crate 落地后实现本 trait 接入 council.
///
/// **用法**:
/// ```ignore
/// use apeireth_council::{Council, SovereigntyHook, CouncilEvent};
///
/// struct MySovereigntyHook;
/// impl SovereigntyHook for MySovereigntyHook {
///     fn on_council_event(&self, event: &CouncilEvent) {
///         match event {
///             CouncilEvent::HoldTriggered { .. } => { /* ... */ }
///             _ => {}
///         }
///     }
/// }
///
/// let mut council = Council::new();
/// council.register_hook(Box::new(MySovereigntyHook));
/// ```
pub trait SovereigntyHook: Send + Sync {
    /// 接收 council 事件
    fn on_council_event(&self, event: &CouncilEvent);
    /// 智囊团裁决标识 (供 council 区分多 hook)
    fn hook_id(&self) -> &str {
        "default"
    }
}

/// Sovereignty hook 默认空实现 (用于测试 / 无 sovereignty 场景).
pub struct NoopSovereigntyHook;

impl SovereigntyHook for NoopSovereigntyHook {
    fn on_council_event(&self, _event: &CouncilEvent) {
        // 故意空 — sovereignty 落地后替换为真实实现
    }
    fn hook_id(&self) -> &str {
        "noop"
    }
}
