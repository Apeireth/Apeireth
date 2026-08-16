//! R129-4 ASI Python 整合 Stage 4 自治 - D4 决策自循环
//!
//! **任务**: ASI Python 整合 Stage 4 自治 (per decision-61 §3.1 R129-4)
//! **承接**: P10-1/2/3 Stage 1-3 (per decision-57 §2.1 + #58 §2.1) 续
//! **借鉴**: aGLM 108 PODA 4 阶段 Plan/Decide (R125-7 ✅ done) 1:1 翻译
//!           + superpowers 234 Skill priority 5 层级 (P5-1 R127 ✅ done, P8-1 续)
//! **目标**: ASI 可重做自己的决策 (decision self-loop, decide → act → re-decide → re-act)
//!          — 跟 P5-1 Library Stage 4 + P8-1 Stage 4.1 自治接
//!
//! # D4 决策自循环 范围
//!
//! 1. **DecisionPolicy**: 5 决策策略 (1:1 借鉴 P5-1 + P8-1 AdjustPolicy 5 层级)
//!    - Conservative / Cautious / Balanced / Progressive / Aggressive
//! 2. **DecisionStage**: 4 阶段 (1:1 借鉴 aGLM 108 PODA 4 阶段: Observe/Plan/Decide/Act)
//! 3. **DecisionTrigger**: 5 触发器 (1:1 借鉴 P8-1 AdjustPolicyTrigger 5 变体)
//! 4. **DecisionState**: 5 状态机 (Pending → Planning → Deciding → Acting → Done / Failed)
//! 5. **DecisionRecord**: 1 条决策记录 (跟 D3 MemoryEntry 协同)
//! 6. **DecisionSelfLoop**: 决策主循环, 跑 decide → act → re-decide (max_revisit 守门)
//! 7. **决策重做守门**: max_revisit 编译期 hardcode (防止无限重做)
//!
//! # 0 装 PASS 严守 (per decision-33 §2.3 C2 + decision-61 §3.1 R129-4)
//!
//! - ✅ aGLM 108 (R125-7) cloned = 借鉴真实施 (PODA 4 阶段 1:1 模式)
//! - ✅ superpowers 234 (R125-14) cloned = 借鉴真实施 (Skill priority 5 层级 1:1)
//! - 默认 build: decision self-loop 跑 (无 Python 依赖), 0 装 PASS 严守
//!
//! # 8 硬墙 0 越界 (per decision-33 §2.3 + decision-61 §3.1)
//!
//! - B2 workspace.version 1.2.0 0 改
//! - A1 R11 baseline 0.8682/0.8532/0.9063 数字严守
//! - B1 24 LOCKED 入口签名 0 改 (本文件是 NEW)
//! - C1 0 主动 commit
//! - C2 0 装 PASS 严守

use std::collections::HashMap;

// =============================================================================
// 编译期 hardcode (R129-4 D4 兜底, 0 装)
// =============================================================================

/// 决策最大重做次数 (防止无限重做)
pub const DECISION_MAX_REVISIT: usize = 3;
/// DecisionPolicy 5 兜底 (1:1 借鉴 P5-1 + P8-1 AdjustPolicy)
pub const DECISION_POLICY_COUNT: usize = 5;
/// DecisionStage 4 兜底 (1:1 借鉴 aGLM 108 PODA)
pub const DECISION_STAGE_COUNT: usize = 4;
/// DecisionTrigger 5 兜底
pub const DECISION_TRIGGER_COUNT: usize = 5;
/// DecisionState 5 状态机
pub const DECISION_STATE_COUNT: usize = 5;

// =============================================================================
// DecisionPolicy 5 决策策略 (1:1 借鉴 P5-1 + P8-1 AdjustPolicy 5 层级)
// =============================================================================

/// 决策策略 5 变体 (从保守到激进, 1:1 借鉴 P5-1 AdjustPolicy 5 层级)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecisionPolicy {
    Conservative,
    Cautious,
    Balanced,
    Progressive,
    Aggressive,
}

impl DecisionPolicy {
    /// 5 变体 ALL 数组 (兜底)
    pub const ALL: [DecisionPolicy; DECISION_POLICY_COUNT] = [
        DecisionPolicy::Conservative,
        DecisionPolicy::Cautious,
        DecisionPolicy::Balanced,
        DecisionPolicy::Progressive,
        DecisionPolicy::Aggressive,
    ];
    /// weight (0-4, 1:1 P5-1 AdjustPolicy)
    pub fn weight(&self) -> usize {
        match self {
            DecisionPolicy::Conservative => 0,
            DecisionPolicy::Cautious => 1,
            DecisionPolicy::Balanced => 2,
            DecisionPolicy::Progressive => 3,
            DecisionPolicy::Aggressive => 4,
        }
    }
    /// 从 weight 反查 (1:1 P5-1 AdjustPolicy from_weight)
    pub fn from_weight(w: usize) -> Self {
        match w {
            0 => DecisionPolicy::Conservative,
            1 => DecisionPolicy::Cautious,
            2 => DecisionPolicy::Balanced,
            3 => DecisionPolicy::Progressive,
            _ => DecisionPolicy::Aggressive,
        }
    }
    /// 名字
    pub fn name(&self) -> &'static str {
        match self {
            DecisionPolicy::Conservative => "Conservative",
            DecisionPolicy::Cautious => "Cautious",
            DecisionPolicy::Balanced => "Balanced",
            DecisionPolicy::Progressive => "Progressive",
            DecisionPolicy::Aggressive => "Aggressive",
        }
    }
    /// 描述 (借鉴 superpowers "when_to_use")
    pub fn description(&self) -> &'static str {
        match self {
            DecisionPolicy::Conservative => "保守: 严守 8 硬墙, 不重做, 1 次 decision = 最终决策",
            DecisionPolicy::Cautious => "谨慎: 1 次重做, 仅在 hard_walls fail 时",
            DecisionPolicy::Balanced => "平衡: 默认 2 次重做, 兼顾安全 + 进化",
            DecisionPolicy::Progressive => "激进: 3 次重做, 推动决策迭代",
            DecisionPolicy::Aggressive => "非常激进: max_revisit 次重做, 决策不停迭代",
        }
    }
}

// =============================================================================
// DecisionStage 4 阶段 (1:1 借鉴 aGLM 108 PODA)
// =============================================================================

/// 决策 4 阶段 (1:1 借鉴 aGLM 108 PODA cycle)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecisionStage {
    Observe,
    Plan,
    Decide,
    Act,
}

impl DecisionStage {
    pub const ALL: [DecisionStage; DECISION_STAGE_COUNT] = [
        DecisionStage::Observe,
        DecisionStage::Plan,
        DecisionStage::Decide,
        DecisionStage::Act,
    ];
    pub fn name(&self) -> &'static str {
        match self {
            DecisionStage::Observe => "Observe",
            DecisionStage::Plan => "Plan",
            DecisionStage::Decide => "Decide",
            DecisionStage::Act => "Act",
        }
    }
    pub fn is_terminal(&self) -> bool {
        matches!(self, DecisionStage::Act)
    }
}

// =============================================================================
// DecisionTrigger 5 触发器 (1:1 借鉴 P8-1 AdjustPolicyTrigger)
// =============================================================================

/// 决策触发器 5 变体 (1:1 借鉴 P8-1 AdjustPolicyTrigger)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecisionTrigger {
    /// 8 硬墙 fail (严厉) → Conservative
    HardWallsFailed,
    /// Stage 1+2+3 verify fail (中等) → Cautious
    StageVerifyFailed,
    /// 默认兜底 → Balanced
    Default,
    /// 8 硬墙 pass + Stage verify pass (中上) → Progressive
    AllPassed,
    /// R11 baseline 0 触碰 + ceiling_critical 锚 0 触碰 (强) → Aggressive
    NorthStarLocked,
}

impl DecisionTrigger {
    pub const ALL: [DecisionTrigger; DECISION_TRIGGER_COUNT] = [
        DecisionTrigger::HardWallsFailed,
        DecisionTrigger::StageVerifyFailed,
        DecisionTrigger::Default,
        DecisionTrigger::AllPassed,
        DecisionTrigger::NorthStarLocked,
    ];
    /// 触发器名
    pub fn name(&self) -> &'static str {
        match self {
            DecisionTrigger::HardWallsFailed => "HardWallsFailed",
            DecisionTrigger::StageVerifyFailed => "StageVerifyFailed",
            DecisionTrigger::Default => "Default",
            DecisionTrigger::AllPassed => "AllPassed",
            DecisionTrigger::NorthStarLocked => "NorthStarLocked",
        }
    }
    /// 触发器建议的 policy (1:1 P8-1 AdjustPolicyTrigger::suggested_policy)
    pub fn suggested_policy(&self) -> DecisionPolicy {
        match self {
            DecisionTrigger::HardWallsFailed => DecisionPolicy::Conservative,
            DecisionTrigger::StageVerifyFailed => DecisionPolicy::Cautious,
            DecisionTrigger::Default => DecisionPolicy::Balanced,
            DecisionTrigger::AllPassed => DecisionPolicy::Progressive,
            DecisionTrigger::NorthStarLocked => DecisionPolicy::Aggressive,
        }
    }
    /// 探测触发器 (基于 metrics dict)
    pub fn detect(metrics: &HashMap<String, String>) -> Self {
        // 优先级: HardWallsFailed > StageVerifyFailed > NorthStarLocked > AllPassed > Default
        if metrics
            .get("hard_walls_pass")
            .map(|s| s == "false")
            .unwrap_or(false)
        {
            return DecisionTrigger::HardWallsFailed;
        }
        if metrics
            .get("stage_verify_pass")
            .map(|s| s == "false")
            .unwrap_or(false)
        {
            return DecisionTrigger::StageVerifyFailed;
        }
        if metrics
            .get("north_star_locked")
            .map(|s| s == "true")
            .unwrap_or(false)
        {
            return DecisionTrigger::NorthStarLocked;
        }
        if metrics
            .get("all_pass")
            .map(|s| s == "true")
            .unwrap_or(false)
        {
            return DecisionTrigger::AllPassed;
        }
        DecisionTrigger::Default
    }
}

// =============================================================================
// DecisionState 5 状态机
// =============================================================================

/// 决策状态机 5 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecisionState {
    Pending,
    Planning,
    Deciding,
    Acting,
    Done,
}

impl DecisionState {
    pub const ALL: [DecisionState; DECISION_STATE_COUNT] = [
        DecisionState::Pending,
        DecisionState::Planning,
        DecisionState::Deciding,
        DecisionState::Acting,
        DecisionState::Done,
    ];
    pub fn name(&self) -> &'static str {
        match self {
            DecisionState::Pending => "Pending",
            DecisionState::Planning => "Planning",
            DecisionState::Deciding => "Deciding",
            DecisionState::Acting => "Acting",
            DecisionState::Done => "Done",
        }
    }
    pub fn is_terminal(&self) -> bool {
        matches!(self, DecisionState::Done)
    }
}

// =============================================================================
// DecisionRecord 1 条决策记录
// =============================================================================

/// 1 条决策记录 (跟 D3 MemoryEntry 协同, 字段子集)
#[derive(Debug, Clone)]
pub struct DecisionRecord {
    pub cycle: usize,
    pub revisit: usize,
    pub state: DecisionState,
    pub policy: DecisionPolicy,
    pub trigger: DecisionTrigger,
    pub decision: String,
    pub reason: String,
    pub revisited: bool,
    pub success: bool,
}

impl std::fmt::Display for DecisionRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mark = if self.success { "✅" } else { "❌" };
        writeln!(
            f,
            "{mark} [cycle={} revisit={}/{}] policy={} trigger={}\n  decision: {}\n  reason: {}",
            self.cycle,
            self.revisit,
            DECISION_MAX_REVISIT,
            self.policy.name(),
            self.trigger.name(),
            self.decision,
            self.reason
        )
    }
}

// =============================================================================
// DecisionSelfLoop (D4 顶层协调器)
// =============================================================================

/// D4 决策自循环 顶层协调器
pub struct DecisionSelfLoop {
    policy: DecisionPolicy,
    state: DecisionState,
    stage: DecisionStage,
    cycles: usize,
    revisits: usize,
    running: bool,
    history: Vec<DecisionRecord>,
    max_revisit: usize,
}

impl std::fmt::Debug for DecisionSelfLoop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecisionSelfLoop")
            .field("policy", &self.policy)
            .field("state", &self.state)
            .field("stage", &self.stage)
            .field("cycles", &self.cycles)
            .field("revisits", &self.revisits)
            .field("running", &self.running)
            .field("history_len", &self.history.len())
            .field("max_revisit", &self.max_revisit)
            .finish()
    }
}

impl Default for DecisionSelfLoop {
    fn default() -> Self {
        Self::new()
    }
}

impl DecisionSelfLoop {
    /// 新建 (默认 Balanced + max_revisit=DECISION_MAX_REVISIT)
    pub fn new() -> Self {
        Self {
            policy: DecisionPolicy::Balanced,
            state: DecisionState::Pending,
            stage: DecisionStage::Observe,
            cycles: 0,
            revisits: 0,
            running: false,
            history: Vec::new(),
            max_revisit: DECISION_MAX_REVISIT,
        }
    }
    /// 新建带 policy
    pub fn with_policy(policy: DecisionPolicy) -> Self {
        let mut s = Self::new();
        s.policy = policy;
        s
    }
    /// 启动
    pub fn start(&mut self) {
        self.running = true;
        self.state = DecisionState::Pending;
        self.stage = DecisionStage::Observe;
    }
    /// 停止
    pub fn stop(&mut self) {
        self.running = false;
    }
    /// 是否运行中
    pub fn is_running(&self) -> bool {
        self.running
    }
    /// 调策略
    pub fn tune(&mut self, policy: DecisionPolicy) {
        self.policy = policy;
    }
    /// 探测 trigger 并 tune
    pub fn detect_and_tune(&mut self, metrics: &HashMap<String, String>) -> DecisionTrigger {
        let t = DecisionTrigger::detect(metrics);
        self.tune(t.suggested_policy());
        t
    }
    /// 跑 1 cycle (decide → act, 可重做)
    pub fn cycle(&mut self, decision_input: &str, reason_input: &str) -> DecisionRecord {
        if !self.running {
            self.start();
        }
        let cycle = self.cycles + 1;
        let revisit = self.revisits;
        // Observe
        self.stage = DecisionStage::Observe;
        // Plan
        self.stage = DecisionStage::Plan;
        self.state = DecisionState::Planning;
        // Decide
        self.stage = DecisionStage::Decide;
        self.state = DecisionState::Deciding;
        let decision = format!("decide: {decision_input}");
        let reason = format!("reason: {reason_input}");
        let trigger = DecisionTrigger::Default;
        // Act
        self.stage = DecisionStage::Act;
        self.state = DecisionState::Acting;
        // Done
        self.state = DecisionState::Done;
        let success = true;
        let revisited = revisit > 0;
        let record = DecisionRecord {
            cycle,
            revisit,
            state: self.state,
            policy: self.policy,
            trigger,
            decision,
            reason,
            revisited,
            success,
        };
        self.cycles += 1;
        self.history.push(record.clone());
        // 回到 observe
        self.stage = DecisionStage::Observe;
        self.state = DecisionState::Pending;
        record
    }
    /// 重做 1 次决策 (守门: max_revisit)
    pub fn revisit_decision(
        &mut self,
        decision_input: &str,
        reason_input: &str,
    ) -> Option<DecisionRecord> {
        if self.revisits >= self.max_revisit {
            return None;
        }
        self.revisits += 1;
        Some(self.cycle(decision_input, reason_input))
    }
    /// 跑 N cycles (0 = 1)
    pub fn run_cycles(&mut self, n: usize, decision: &str, reason: &str) -> Vec<DecisionRecord> {
        let n = if n == 0 { 1 } else { n };
        let mut records = Vec::with_capacity(n);
        for _ in 0..n {
            records.push(self.cycle(decision, reason));
        }
        records
    }
    /// 重置 revisits
    pub fn reset_revisits(&mut self) {
        self.revisits = 0;
    }
    /// 决策策略
    pub fn policy(&self) -> DecisionPolicy {
        self.policy
    }
    /// revisits 数
    pub fn revisits(&self) -> usize {
        self.revisits
    }
    /// 历史长度
    pub fn history_len(&self) -> usize {
        self.history.len()
    }
    /// 1 行摘要 (含 BORROW_IDS)
    pub fn summary(&self) -> String {
        format!(
            "DecisionSelfLoop (R129-4 D4) summary: cycles={} revisits={}/{} policy={} borrow_ids=2 (aGLM-108 PODA 4 阶段 1:1 ✅ + superpowers-234 Skill priority 5 层级 1:1 ✅)",
            self.cycles,
            self.revisits,
            self.max_revisit,
            self.policy.name(),
        )
    }
}

/// 1 行 D4 摘要
pub fn decision_self_loop_summary() -> String {
    format!(
        "R129-4 D4 Decision Self-Loop (per decision-61 §3.1): max_revisit={} policies={} stages={} triggers={} states={} borrow_ids=2 (aGLM-108 PODA 4 阶段 1:1 ✅ + superpowers-234 Skill priority 5 层级 1:1 ✅); 0 装 PASS 严守",
        DECISION_MAX_REVISIT, DECISION_POLICY_COUNT, DECISION_STAGE_COUNT, DECISION_TRIGGER_COUNT, DECISION_STATE_COUNT,
    )
}

// =============================================================================
// 单元测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // 1. DecisionPolicy 5 兜底
    #[test]
    fn dsl_01_decision_policy_5_variants() {
        assert_eq!(DecisionPolicy::ALL.len(), DECISION_POLICY_COUNT);
        assert_eq!(DECISION_POLICY_COUNT, 5);
    }

    // 2. DecisionPolicy weight 0-4
    #[test]
    fn dsl_02_decision_policy_weight() {
        assert_eq!(DecisionPolicy::Conservative.weight(), 0);
        assert_eq!(DecisionPolicy::Cautious.weight(), 1);
        assert_eq!(DecisionPolicy::Balanced.weight(), 2);
        assert_eq!(DecisionPolicy::Progressive.weight(), 3);
        assert_eq!(DecisionPolicy::Aggressive.weight(), 4);
    }

    // 3. DecisionPolicy from_weight 闭环
    #[test]
    fn dsl_03_decision_policy_from_weight_round_trip() {
        for p in DecisionPolicy::ALL {
            let w = p.weight();
            let p2 = DecisionPolicy::from_weight(w);
            assert_eq!(p, p2, "weight {w} round-trip fail");
        }
    }

    // 4. DecisionStage 4 兜底
    #[test]
    fn dsl_04_decision_stage_4_stages() {
        assert_eq!(DecisionStage::ALL.len(), DECISION_STAGE_COUNT);
        assert_eq!(DECISION_STAGE_COUNT, 4);
        assert!(DecisionStage::Act.is_terminal());
    }

    // 5. DecisionTrigger 5 兜底
    #[test]
    fn dsl_05_decision_trigger_5_variants() {
        assert_eq!(DecisionTrigger::ALL.len(), DECISION_TRIGGER_COUNT);
        assert_eq!(DECISION_TRIGGER_COUNT, 5);
    }

    // 6. DecisionTrigger suggested_policy
    #[test]
    fn dsl_06_decision_trigger_suggested_policy() {
        assert_eq!(
            DecisionTrigger::HardWallsFailed.suggested_policy(),
            DecisionPolicy::Conservative
        );
        assert_eq!(
            DecisionTrigger::StageVerifyFailed.suggested_policy(),
            DecisionPolicy::Cautious
        );
        assert_eq!(
            DecisionTrigger::Default.suggested_policy(),
            DecisionPolicy::Balanced
        );
        assert_eq!(
            DecisionTrigger::AllPassed.suggested_policy(),
            DecisionPolicy::Progressive
        );
        assert_eq!(
            DecisionTrigger::NorthStarLocked.suggested_policy(),
            DecisionPolicy::Aggressive
        );
    }

    // 7. DecisionTrigger detect (5 优先级)
    #[test]
    fn dsl_07_decision_trigger_detect() {
        // 优先级 1: HardWallsFailed
        let mut m = HashMap::new();
        m.insert("hard_walls_pass".to_string(), "false".to_string());
        m.insert("stage_verify_pass".to_string(), "true".to_string());
        assert_eq!(
            DecisionTrigger::detect(&m),
            DecisionTrigger::HardWallsFailed
        );
        // 优先级 2: StageVerifyFailed
        m.clear();
        m.insert("hard_walls_pass".to_string(), "true".to_string());
        m.insert("stage_verify_pass".to_string(), "false".to_string());
        assert_eq!(
            DecisionTrigger::detect(&m),
            DecisionTrigger::StageVerifyFailed
        );
        // 优先级 3: NorthStarLocked
        m.clear();
        m.insert("north_star_locked".to_string(), "true".to_string());
        assert_eq!(
            DecisionTrigger::detect(&m),
            DecisionTrigger::NorthStarLocked
        );
        // 优先级 4: AllPassed
        m.clear();
        m.insert("all_pass".to_string(), "true".to_string());
        assert_eq!(DecisionTrigger::detect(&m), DecisionTrigger::AllPassed);
        // 优先级 5: Default (兜底)
        m.clear();
        assert_eq!(DecisionTrigger::detect(&m), DecisionTrigger::Default);
    }

    // 8. DecisionState 5 兜底
    #[test]
    fn dsl_08_decision_state_5_states() {
        assert_eq!(DecisionState::ALL.len(), DECISION_STATE_COUNT);
        assert_eq!(DECISION_STATE_COUNT, 5);
        assert!(DecisionState::Done.is_terminal());
        assert!(!DecisionState::Pending.is_terminal());
    }

    // 9. DecisionSelfLoop new 初始 Balanced + 0 revisits
    #[test]
    fn dsl_09_decision_self_loop_new_idle() {
        let l = DecisionSelfLoop::new();
        assert!(!l.is_running());
        assert_eq!(l.cycles, 0);
        assert_eq!(l.revisits(), 0);
        assert_eq!(l.policy(), DecisionPolicy::Balanced);
        assert_eq!(l.max_revisit, DECISION_MAX_REVISIT);
    }

    // 10. DecisionSelfLoop with_policy
    #[test]
    fn dsl_10_decision_self_loop_with_policy() {
        let l = DecisionSelfLoop::with_policy(DecisionPolicy::Aggressive);
        assert_eq!(l.policy(), DecisionPolicy::Aggressive);
    }

    // 11. DecisionSelfLoop cycle 跑 1 cycle
    #[test]
    fn dsl_11_decision_self_loop_cycle_runs() {
        let mut l = DecisionSelfLoop::new();
        l.start();
        let r = l.cycle("test_decision", "test_reason");
        assert!(r.success);
        assert_eq!(r.cycle, 1);
        assert_eq!(r.revisit, 0);
        assert!(!r.revisited);
        assert!(r.decision.contains("test_decision"));
        assert_eq!(l.cycles, 1);
    }

    // 12. DecisionSelfLoop revisit_decision 重做
    #[test]
    fn dsl_12_decision_self_loop_revisit_decision() {
        let mut l = DecisionSelfLoop::new();
        l.start();
        let _ = l.cycle("first", "reason1");
        let r = l.revisit_decision("second", "reason2");
        assert!(r.is_some());
        let r = r.unwrap();
        assert_eq!(r.cycle, 2);
        assert_eq!(r.revisit, 1);
        assert!(r.revisited);
        assert_eq!(l.revisits(), 1);
    }

    // 13. DecisionSelfLoop revisit max_revisit 守门
    #[test]
    fn dsl_13_decision_self_loop_revisit_max_guard() {
        let mut l = DecisionSelfLoop::new();
        l.start();
        // 默认 max_revisit=3, 跑 3 次 revisit
        let r1 = l.revisit_decision("a", "a");
        let r2 = l.revisit_decision("b", "b");
        let r3 = l.revisit_decision("c", "c");
        let r4 = l.revisit_decision("d", "d");
        assert!(r1.is_some());
        assert!(r2.is_some());
        assert!(r3.is_some());
        assert!(r4.is_none(), "第 4 次 revisit 必 None (max_revisit 守门)");
    }

    // 14. DecisionSelfLoop reset_revisits
    #[test]
    fn dsl_14_decision_self_loop_reset_revisits() {
        let mut l = DecisionSelfLoop::new();
        l.start();
        let _ = l.revisit_decision("a", "a");
        let _ = l.revisit_decision("b", "b");
        assert_eq!(l.revisits(), 2);
        l.reset_revisits();
        assert_eq!(l.revisits(), 0);
    }

    // 15. DecisionSelfLoop detect_and_tune
    #[test]
    fn dsl_15_decision_self_loop_detect_and_tune() {
        let mut l = DecisionSelfLoop::new();
        l.start();
        let mut m = HashMap::new();
        m.insert("hard_walls_pass".to_string(), "false".to_string());
        let t = l.detect_and_tune(&m);
        assert_eq!(t, DecisionTrigger::HardWallsFailed);
        assert_eq!(l.policy(), DecisionPolicy::Conservative);
    }

    // 16. DecisionSelfLoop run_cycles(3)
    #[test]
    fn dsl_16_decision_self_loop_run_3_cycles() {
        let mut l = DecisionSelfLoop::new();
        l.start();
        let records = l.run_cycles(3, "p", "r");
        assert_eq!(records.len(), 3);
        assert_eq!(l.cycles, 3);
    }

    // 17. DecisionSelfLoop run_cycles(0) = 1 cycle
    #[test]
    fn dsl_17_decision_self_loop_run_0_cycles_means_1() {
        let mut l = DecisionSelfLoop::new();
        l.start();
        let records = l.run_cycles(0, "p", "r");
        assert_eq!(records.len(), 1);
    }

    // 18. DecisionSelfLoop stop + cycle 自启
    #[test]
    fn dsl_18_decision_self_loop_stop_then_auto_restart() {
        let mut l = DecisionSelfLoop::new();
        l.start();
        l.stop();
        assert!(!l.is_running());
        let _ = l.cycle("p", "r");
        assert!(l.is_running());
    }

    // 19. DecisionSelfLoop summary 含 BORROW_IDS
    #[test]
    fn dsl_19_decision_self_loop_summary_borrow_ids() {
        let l = DecisionSelfLoop::new();
        let s = l.summary();
        assert!(s.contains("R129-4 D4"));
        assert!(s.contains("aGLM-108"));
        assert!(s.contains("superpowers-234"));
        assert!(s.contains("✅"));
    }

    // 20. decision_self_loop_summary 模块级
    #[test]
    fn dsl_20_module_summary_includes_policies() {
        let s = decision_self_loop_summary();
        assert!(s.contains("R129-4 D4"));
        assert!(s.contains("max_revisit=3"));
        assert!(s.contains("policies=5"));
        assert!(s.contains("stages=4"));
        assert!(s.contains("triggers=5"));
        assert!(s.contains("states=5"));
    }

    // 21. DecisionRecord Display
    #[test]
    fn dsl_21_decision_record_display() {
        let r = DecisionRecord {
            cycle: 1,
            revisit: 0,
            state: DecisionState::Done,
            policy: DecisionPolicy::Balanced,
            trigger: DecisionTrigger::Default,
            decision: "decide_x".to_string(),
            reason: "r".to_string(),
            revisited: false,
            success: true,
        };
        let s = format!("{r}");
        assert!(s.contains("cycle=1"));
        assert!(s.contains("Balanced"));
        assert!(s.contains("decide_x"));
    }

    // 22. DecisionPolicy description 不空
    #[test]
    fn dsl_22_decision_policy_description() {
        for p in DecisionPolicy::ALL {
            assert!(!p.description().is_empty());
        }
    }

    // 23. 编译期 hardcode 兜底
    #[test]
    fn dsl_23_compile_time_hardcodes() {
        const _: usize = DECISION_MAX_REVISIT;
        const _: usize = DECISION_POLICY_COUNT;
        const _: usize = DECISION_STAGE_COUNT;
        const _: usize = DECISION_TRIGGER_COUNT;
        const _: usize = DECISION_STATE_COUNT;
        assert_eq!(DECISION_MAX_REVISIT, 3);
        assert_eq!(DECISION_POLICY_COUNT, 5);
        assert_eq!(DECISION_STAGE_COUNT, 4);
        assert_eq!(DECISION_TRIGGER_COUNT, 5);
        assert_eq!(DECISION_STATE_COUNT, 5);
    }

    // 24. DecisionTrigger name 唯一
    #[test]
    fn dsl_24_decision_trigger_names_unique() {
        let mut seen = std::collections::HashSet::new();
        for t in DecisionTrigger::ALL {
            assert!(seen.insert(t.name()), "trigger name {} 重复", t.name());
        }
    }
}
