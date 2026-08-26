//! apeireth-consciousness: 意识子系统 (A12 落点 — Cognitive-Dream 6 状态机)
//!
//! **职责**: 建模主体的"意识状态机" — 6 状态 (Awake / Reflecting / Dreaming /
//! Meditating / SelfDisabling / Recovering) + 合法转换矩阵 + 转换历史 + 主体
//! 连续性 ID 锚定。
//!
//! **架构位置**: 阶段 3 §3.6 蓝图 (v1 重写) + 阶段 4 §5 机制 3 反思 (Cognitive-Dream
//! DREAMING/CONSOLIDATING 适配)。本 crate 是简化版状态机 — 不重写 R11 Cognitive-Dream
//! 的完整 IDLE/DREAMING/CONSOLIDATING/FORGETTING/VERIFYING/INTERRUPTED 链，而是按
//! 主人 8 月 1 日指令落地 6 状态紧凑版本。
//!
//! **当前状态**: A12 最小可用落地 (P4 任务 4926b6a3 by devops_engineer2).
//! 本 crate 提供 7+ pub fn + 6 单元测试 + 1 集成测试 + 1 example.
//!
//! **诚实登记**:
//! - ⚠️ 阶段 3 设计层文档中**未发现 §3.6** 章节。本 crate 按 Leader P4 任务文本的 6 状态
//!   枚举 + 合逻辑理的状态转换矩阵落地。漂移见 `reports/achievement-A12-devops-engineer2-consciousness-relation.md`.
//! - ⚠️ 与 R11 Cognitive-Dream (IDLE/DREAMING/CONSOLIDATING/FORGETTING/VERIFYING/INTERRUPTED)
//!   命名不一致 — 这是**新设计**，不修改 R11 baseline。
//!
//! **禁止**:
//! - ❌ 不修改 apeireth-core 任何已实装类型签名
//! - ❌ 不碰 R11 baseline 三值
//! - ❌ 不碰 apeireth-legacy/

#![deny(unsafe_code)]

use chrono::{DateTime, Utc};

// R22 ST-A2.2 — Cognitive-Dream 6 状态机深化 (transition_rate_limit + cycle_detector)
pub mod emotion;
pub mod plutchik; // R218
pub mod plutchik_engine; // R211: ExtendedEmotionEngine — Plutchik emotion engine 集成
pub mod plutchik_integration; // R209: Plutchik <-> BaseEmotion 6 桥接: Plutchik 8 基础 + 8 高级情绪 (R187 调研推荐)
pub mod transfer_monitor;
// R173 ST-B7.1 — bridge 7: memory -> consciousness
pub mod memory_bridge;
// R176: bridge 7 Kani proofs
mod memory_kani_proofs;
mod organ_kani_proofs;
use thiserror::Error;
use uuid::Uuid;

/// 6 状态 Cognitive-Dream 状态机。
///
/// **状态语义** (v1, A12 简化版):
/// - `Awake`         — 主备待机, 处理日常输入, 可转入反思或自禁用
/// - `Reflecting`    — 主动反思期, 评估最近输出, 可转入梦境或冥想
/// - `Dreaming`      — 夜间整合, 沉淀记忆, 可转入冥想或恢复
/// - `Meditating`    — 深度整合, 静默整合梦境产出, 可转回梦境或恢复
/// - `SelfDisabling` — L0 HA 触发紧急停机, **只可**转入恢复
/// - `Recovering`    — 恢复期, 评估损害并逐步回到 Awake
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CognitiveDreamState {
    /// 主备待机.
    Awake,
    /// 主动反思期.
    Reflecting,
    /// 梦境整合.
    Dreaming,
    /// 深度冥想整合.
    Meditating,
    /// L0 HA 紧急停机.
    SelfDisabling,
    /// 恢复期.
    Recovering,
}

impl CognitiveDreamState {
    /// 全部 6 状态 (供断言 + 完整性测试).
    pub const ALL: [CognitiveDreamState; 6] = [
        CognitiveDreamState::Awake,
        CognitiveDreamState::Reflecting,
        CognitiveDreamState::Dreaming,
        CognitiveDreamState::Meditating,
        CognitiveDreamState::SelfDisabling,
        CognitiveDreamState::Recovering,
    ];

    /// 状态中文名 (用户可感知).
    pub const fn semantic_name(self) -> &'static str {
        match self {
            CognitiveDreamState::Awake => "awake",
            CognitiveDreamState::Reflecting => "reflecting",
            CognitiveDreamState::Dreaming => "dreaming",
            CognitiveDreamState::Meditating => "meditating",
            CognitiveDreamState::SelfDisabling => "self_disabling",
            CognitiveDreamState::Recovering => "recovering",
        }
    }

    /// 状态短描述 (1 行).
    pub fn describe(self) -> &'static str {
        match self {
            CognitiveDreamState::Awake => "主备待机 — 处理日常输入",
            CognitiveDreamState::Reflecting => "主动反思 — 评估最近输出",
            CognitiveDreamState::Dreaming => "梦境整合 — 沉淀记忆",
            CognitiveDreamState::Meditating => "深度冥想 — 静默整合梦境产出",
            CognitiveDreamState::SelfDisabling => "L0 HA 紧急停机 — 等待恢复",
            CognitiveDreamState::Recovering => "恢复期 — 评估损害逐步回到 Awake",
        }
    }
}

/// 合法转换矩阵 — 6 状态间允许的转换。
///
/// 设计依据 (合逻辑理):
/// - Awake ↔ Reflecting (主备/反思切换)
/// - Reflecting → Dreaming (夜间反思触发梦境)
/// - Dreaming ↔ Meditating (深度整合切换)
/// - Dreaming/Meditating → Recovering (整合完成 → 恢复)
/// - **任意非 SelfDisabling → SelfDisabling** (L0 HA 紧急停)
/// - **SelfDisabling → Recovering** (唯一出口 — 锁)
/// - Recovering → Awake (回到常态) / → SelfDisabling (再次停机)
pub fn legal_targets(from: CognitiveDreamState) -> &'static [CognitiveDreamState] {
    use CognitiveDreamState::{Awake, Dreaming, Meditating, Recovering, Reflecting, SelfDisabling};
    match from {
        Awake => &[Reflecting, SelfDisabling],
        Reflecting => &[Awake, Dreaming, Meditating, SelfDisabling],
        Dreaming => &[Meditating, Recovering, SelfDisabling],
        Meditating => &[Dreaming, Recovering, SelfDisabling],
        SelfDisabling => &[Recovering],
        Recovering => &[Awake, SelfDisabling],
    }
}

/// 转换是否合法.
pub fn can_transition(from: CognitiveDreamState, to: CognitiveDreamState) -> bool {
    legal_targets(from).contains(&to)
}

/// 顶层错误: 状态机错误.
#[derive(Debug, Error)]
pub enum ConsciousnessError {
    /// 非法状态转换.
    #[error("illegal transition: {from:?} -> {to:?} (legal targets: {legal:?})")]
    IllegalTransition {
        /// 起始状态.
        from: CognitiveDreamState,
        /// 目标状态.
        to: CognitiveDreamState,
        /// 合法目标列表.
        legal: Vec<CognitiveDreamState>,
    },
    /// 主体连续性 ID 缺失.
    #[error("continuity_id missing for machine instance")]
    MissingContinuityId,
}

/// 统一结果类型.
pub type ConsciousnessResult<T> = Result<T, ConsciousnessError>;

/// 一条转换记录 (审计日志).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransitionRecord {
    /// 起始状态.
    pub from: CognitiveDreamState,
    /// 目标状态.
    pub to: CognitiveDreamState,
    /// 触发时间 (UTC).
    pub at: DateTime<Utc>,
    /// 触发原因 (用户/系统/L0 HA).
    pub reason: TransitionReason,
}

/// 触发原因分类.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TransitionReason {
    /// 用户/外部触发.
    UserTriggered,
    /// 系统周期触发 (e.g. 夜间反思).
    Scheduled,
    /// L0 HA 触发 (紧急停机).
    L0HaEmergency,
    /// 内部状态自决.
    Internal,
}

/// 6 状态状态机实例 — 绑定主体连续性 ID (IdentityCard.continuity_id).
#[derive(Debug, Clone)]
pub struct CognitiveDreamStateMachine {
    /// 主体连续性 ID (来自 IdentityCard).
    pub continuity_id: String,
    /// 当前状态.
    pub current: CognitiveDreamState,
    /// 转换历史 (按时间顺序).
    pub history: Vec<TransitionRecord>,
    /// 机器唯一 ID (用于审计).
    pub machine_id: Uuid,
}

impl CognitiveDreamStateMachine {
    /// 新建 — 初始 Awake.
    pub fn new(continuity_id: impl Into<String>) -> Self {
        Self {
            continuity_id: continuity_id.into(),
            current: CognitiveDreamState::Awake,
            history: Vec::new(),
            machine_id: Uuid::new_v4(),
        }
    }

    /// 指定初始状态的新建 (用于从 R11 baseline 恢复).
    pub fn with_initial(continuity_id: impl Into<String>, initial: CognitiveDreamState) -> Self {
        Self {
            continuity_id: continuity_id.into(),
            current: initial,
            history: Vec::new(),
            machine_id: Uuid::new_v4(),
        }
    }

    /// 通用转换入口 — 校验合法性, 记录历史, 返回新状态.
    pub fn transition(
        &mut self,
        to: CognitiveDreamState,
        reason: TransitionReason,
    ) -> ConsciousnessResult<CognitiveDreamState> {
        if self.continuity_id.is_empty() {
            return Err(ConsciousnessError::MissingContinuityId);
        }
        if !can_transition(self.current, to) {
            return Err(ConsciousnessError::IllegalTransition {
                from: self.current,
                to,
                legal: legal_targets(self.current).to_vec(),
            });
        }
        let record = TransitionRecord {
            from: self.current,
            to,
            at: Utc::now(),
            reason,
        };
        self.history.push(record);
        self.current = to;
        Ok(self.current)
    }

    /// 便捷: 进入 Reflecting.
    pub fn enter_reflecting(&mut self) -> ConsciousnessResult<CognitiveDreamState> {
        self.transition(
            CognitiveDreamState::Reflecting,
            TransitionReason::UserTriggered,
        )
    }

    /// 便捷: 进入 Dreaming.
    pub fn enter_dreaming(&mut self) -> ConsciousnessResult<CognitiveDreamState> {
        self.transition(CognitiveDreamState::Dreaming, TransitionReason::Scheduled)
    }

    /// 便捷: 进入 Meditating.
    pub fn enter_meditating(&mut self) -> ConsciousnessResult<CognitiveDreamState> {
        self.transition(CognitiveDreamState::Meditating, TransitionReason::Internal)
    }

    /// 便捷: 进入 SelfDisabling (L0 HA 紧急停).
    pub fn enter_self_disabling(&mut self) -> ConsciousnessResult<CognitiveDreamState> {
        self.transition(
            CognitiveDreamState::SelfDisabling,
            TransitionReason::L0HaEmergency,
        )
    }

    /// 便捷: 进入 Recovering (从 SelfDisabling 唯一出口).
    pub fn enter_recovering(&mut self) -> ConsciousnessResult<CognitiveDreamState> {
        self.transition(CognitiveDreamState::Recovering, TransitionReason::Internal)
    }

    /// 便捷: 重置回 Awake (Recovering → Awake).
    pub fn reset_to_awake(&mut self) -> ConsciousnessResult<CognitiveDreamState> {
        self.transition(CognitiveDreamState::Awake, TransitionReason::Internal)
    }

    /// 当前状态的合法目标.
    pub fn legal_targets_now(&self) -> &'static [CognitiveDreamState] {
        legal_targets(self.current)
    }

    /// 是否处于 L0 HA 紧急停机.
    pub fn is_self_disabled(&self) -> bool {
        self.current == CognitiveDreamState::SelfDisabling
    }

    /// 历史转换次数.
    pub fn transition_count(&self) -> usize {
        self.history.len()
    }
}

// ---------------------------------------------------------------------------
// 单元测试 (≥ 6)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_states_have_semantic_name() {
        for s in CognitiveDreamState::ALL {
            assert!(!s.semantic_name().is_empty());
            assert!(!s.describe().is_empty());
        }
        assert_eq!(CognitiveDreamState::ALL.len(), 6);
    }

    #[test]
    fn awake_can_only_go_to_reflecting_or_self_disabling() {
        let targets = legal_targets(CognitiveDreamState::Awake);
        assert_eq!(targets.len(), 2);
        assert!(can_transition(
            CognitiveDreamState::Awake,
            CognitiveDreamState::Reflecting
        ));
        assert!(can_transition(
            CognitiveDreamState::Awake,
            CognitiveDreamState::SelfDisabling
        ));
        assert!(!can_transition(
            CognitiveDreamState::Awake,
            CognitiveDreamState::Dreaming
        ));
        assert!(!can_transition(
            CognitiveDreamState::Awake,
            CognitiveDreamState::Recovering
        ));
    }

    #[test]
    fn self_disabling_can_only_go_to_recovering() {
        let targets = legal_targets(CognitiveDreamState::SelfDisabling);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0], CognitiveDreamState::Recovering);
        assert!(!can_transition(
            CognitiveDreamState::SelfDisabling,
            CognitiveDreamState::Awake
        ));
        assert!(!can_transition(
            CognitiveDreamState::SelfDisabling,
            CognitiveDreamState::Dreaming
        ));
    }

    #[test]
    fn machine_records_history_on_legal_transition() {
        let mut m = CognitiveDreamStateMachine::new("cid-test-1");
        assert_eq!(m.current, CognitiveDreamState::Awake);
        assert_eq!(m.transition_count(), 0);
        m.enter_reflecting().unwrap();
        m.enter_dreaming().unwrap();
        m.enter_meditating().unwrap();
        m.enter_recovering().unwrap();
        m.reset_to_awake().unwrap();
        assert_eq!(m.transition_count(), 5);
        assert_eq!(m.current, CognitiveDreamState::Awake);
    }

    #[test]
    fn illegal_transition_returns_error() {
        let mut m = CognitiveDreamStateMachine::new("cid-test-2");
        // Awake → Dreaming 不合法
        let err = m.enter_dreaming().unwrap_err();
        match err {
            ConsciousnessError::IllegalTransition { from, to, .. } => {
                assert_eq!(from, CognitiveDreamState::Awake);
                assert_eq!(to, CognitiveDreamState::Dreaming);
            }
            _ => panic!("expected IllegalTransition"),
        }
        // 转换失败不应记录历史
        assert_eq!(m.transition_count(), 0);
        assert_eq!(m.current, CognitiveDreamState::Awake);
    }

    #[test]
    fn l0_emergency_works_from_any_non_self_disabling_state() {
        // 从所有"非 SelfDisabling"状态都能进入 SelfDisabling (L0 HA 紧急停).
        for &initial in &CognitiveDreamState::ALL {
            if initial == CognitiveDreamState::SelfDisabling {
                // 已经是 SelfDisabling — 不能再"进入" SelfDisabling
                continue;
            }
            let mut m = CognitiveDreamStateMachine::with_initial("cid-test-3", initial);
            m.enter_self_disabling().unwrap();
            assert!(m.is_self_disabled());
            // 唯一出口: Recovering
            m.enter_recovering().unwrap();
            assert_eq!(m.current, CognitiveDreamState::Recovering);
        }
    }

    #[test]
    fn self_disabled_cannot_skip_recovery() {
        let mut m = CognitiveDreamStateMachine::new("cid-test-4");
        m.enter_self_disabling().unwrap();
        // SelfDisabling → Awake 非法 (必须先 Recovering)
        let err = m.reset_to_awake().unwrap_err();
        assert!(matches!(err, ConsciousnessError::IllegalTransition { .. }));
        m.enter_recovering().unwrap();
        m.reset_to_awake().unwrap();
        assert_eq!(m.current, CognitiveDreamState::Awake);
    }

    #[test]
    fn empty_continuity_id_is_rejected() {
        let mut m = CognitiveDreamStateMachine::new("");
        let err = m.enter_reflecting().unwrap_err();
        assert!(matches!(err, ConsciousnessError::MissingContinuityId));
    }
}

pub use crate::emotion::DecaySnapshot; // R237
pub use crate::emotion::{
    BaseEmotion, EmError, EmResult, EmotionEngine, EmotionEvent, EmotionSnapshot, Pad,
    ResponseStyle,
};
pub use crate::transfer_monitor::{
    CognitiveDreamMonitor, CycleInfo, MonitorError, RateLimitError, TransferSnapshot,
};
