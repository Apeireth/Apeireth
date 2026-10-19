//! SG-21 游戏 / 幼态延续练习场内核 v0（机制内核，未接线）。
//!
//! # 生物原型
//!
//! 幼年哺乳动物的「游戏」不是无目的的乱动，而是把成年要用的技能拆成低风险预演：
//! 追逐、伏击、社交规则在安全场内反复试错，失败不致命、代价可承受。幼态延续
//! （neoteny）进一步让成年个体保留这份好奇心与可塑性 —— 练习不是长大前的过渡期，
//! 而是一条终身保留的低风险探索通道。
//!
//! 本模块把这份机制缩成三段：先按风险与预算提出一个**受控练习提案**
//! （[`PlayProposal`]），经用户 opt-in 后开一场**有边界、可中止**的练习
//! （[`PlaySession`]），结束时只提炼**技能与产物**（[`PlayOutcome`]）。
//!
//! # 工程边界
//!
//! - **纯内核**：不读系统时钟、不做 I/O、不调 LLM、不落存储、不 spawn；时间一律
//!   由调用方以 `u64` 毫秒注入（`now_ms`），因此可重放、可确定性测试。
//! - **不新增依赖**：只用 `serde` 序列化、`thiserror` 报错；无 chrono / uuid。
//! - **play 是受控探索，不是自由训练**：提案必须过政策过滤（风险 / 预算 / 时长），
//!   默认需要用户 opt-in；本模块不自动开始任何练习。
//! - **不写主体记忆**：练习产出只进 [`PlayOutcome`]，本模块**不存在**任何写主体
//!   记忆 / episode / wiki 的 API；`isolate_memory = false` 时连提案与启动都拒绝。
//! - **不消耗用户预算**：本模块只做估算与预留记录，不调用 provider、不扣费；是否
//!   真的花预算由 W3 接线时的 opt-in 流程决定。
//! - **不调度**：本模块不接 orchestration、不被 scheduler 调用、不产生 self_status
//!   节；谁在何时发起练习仍由上层决定。
//!
//! # 0 装诚实（IMPLEMENTED，未接线）
//!
//! - 已实现：类型、[`PlayLedger`] 的 propose / start / finish / abort / pending
//!   与不变量测试，本文件内自洽。
//! - 未接线：**未进 orchestration**、**尚无 scheduler 调用**、**无 self_status 节**；
//!   没有 CLI / gateway / 工具面入口，也没有 provider 调用。
//! - 因此本模块落在 **IMPLEMENTED** 级：机制与测试就位，**不是**线上行为；不声称
//!   PRODUCTION WIRED / DEFAULT ENABLED。任何「游戏练习已上线」的说法都是 0 装。
//!
//! # 不变量（测试钉死）
//!
//! 1. [`PlayLedger::propose`] 的估计值永不超过 policy 上限，也不超过调用方当前可用
//!    额度；放不下时返回 `None`，绝不缩水伪造一个「能跑」的提案。
//! 2. 风险高于 `allowed_risk` 的主题默认不提案；`NeedsApproval` 永远标记
//!    `needs_user_opt_in = true`，没有显式 opt-in 不得启动。
//! 3. 状态机只允许 `Planned -> Running -> Finished | Aborted`；终态不可复活。
//! 4. [`PlayLedger::abort`] 对已中止 session 可重复调用，且不改变任何字段（幂等）。
//! 5. 练习产出只存在于 [`PlayOutcome`]；模块内没有任何主体记忆写入接口。

use serde::{Deserialize, Serialize};

/// 默认单次练习预算上限：20 000 tokens。
pub const DEFAULT_MAX_BUDGET_TOKENS: u64 = 20_000;

/// 默认单次练习时长上限：15 分钟（900 000 毫秒）。
pub const DEFAULT_MAX_DURATION_MS: u64 = 15 * 60 * 1_000;

/// 预算估算基线：一场练习至少按 2 000 tokens 计。
pub const ESTIMATE_BASE_TOKENS: u64 = 2_000;

/// 每项技能追加的预算估计：1 500 tokens。
pub const ESTIMATE_TOKENS_PER_SKILL: u64 = 1_500;

/// 时长估算基线：一场练习至少按 1 分钟计。
pub const ESTIMATE_BASE_DURATION_MS: u64 = 60_000;

/// 每项技能追加的时长估计：2 分钟。
pub const ESTIMATE_DURATION_PER_SKILL_MS: u64 = 120_000;

/// 练习主题的风险等级，从低到高：`Safe < Sandboxed < NeedsApproval`。
///
/// 排序用于与 [`PlayPolicy::allowed_risk`] 比较：只有 `risk <= allowed_risk` 的
/// 主题才会被提案。`NeedsApproval` 无论 policy 如何都要求用户显式 opt-in。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    /// 安全：只读、无副作用、可随时丢弃的练习（例如纯推理预演）。
    Safe,
    /// 沙箱：在隔离环境里执行，失败可回滚，不触及主体状态。
    Sandboxed,
    /// 需批准：可能影响真实环境，必须由用户明确点头后才能开始。
    NeedsApproval,
}

impl RiskLevel {
    /// 理由文案里的稳定英文名（不随中文翻译漂移）。
    fn label(self) -> &'static str {
        match self {
            Self::Safe => "Safe",
            Self::Sandboxed => "Sandboxed",
            Self::NeedsApproval => "NeedsApproval",
        }
    }
}

/// 一个候选练习主题：练什么、练哪些技能、风险多高。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayTopic {
    /// 主题名（稳定标识；同一主题可跨 session 复用）。
    pub topic: String,
    /// 想练习的技能目标；数量直接影响预算 / 时长估计。
    pub skills: Vec<String>,
    /// 该主题的风险等级。
    pub risk: RiskLevel,
}

/// [`PlayLedger::propose`] 的输出：一份尚未启动、可被用户审阅的练习提案。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayProposal {
    /// 主题名，原样来自 [`PlayTopic::topic`]。
    pub topic: String,
    /// 技能目标，原样来自 [`PlayTopic::skills`]。
    pub skill_targets: Vec<String>,
    /// 估算预算（tokens）；保证 `<= min(policy 上限, 调用方可用额度)`。
    pub estimated_budget_tokens: u64,
    /// 估算时长（毫秒）；保证 `<= min(policy 上限, 调用方可用额度)`。
    pub estimated_duration_ms: u64,
    /// 是否需要用户显式 opt-in；默认 policy 下恒为 `true`。
    pub needs_user_opt_in: bool,
    /// 可解释理由：风险过滤、估算值、有效上限与 opt-in 结论。
    pub rationale: String,
}

/// 练习政策：预算 / 时长天花板、opt-in 与记忆隔离的硬约束。
///
/// 默认值见 [`PlayPolicy::default`]：20 000 tokens、15 分钟、默认要求 opt-in、
/// 强制记忆隔离、最高允许 `Sandboxed`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayPolicy {
    /// 单次练习预算上限（tokens）。
    pub max_budget_tokens: u64,
    /// 单次练习时长上限（毫秒）。
    pub max_duration_ms: u64,
    /// 是否默认要求用户 opt-in（`true` 时所有提案都带 `needs_user_opt_in`）。
    pub require_opt_in_by_default: bool,
    /// 是否强制记忆隔离；`false` 时本内核拒绝提案与启动。
    pub isolate_memory: bool,
    /// 允许的最高风险等级；高于它的主题不会被提案。
    pub allowed_risk: RiskLevel,
}

impl Default for PlayPolicy {
    /// 保守默认：20 000 tokens / 15 分钟、默认要求 opt-in、强制记忆隔离、
    /// 最高只允许 `Sandboxed`（`NeedsApproval` 默认不进入自动提案）。
    fn default() -> Self {
        Self {
            max_budget_tokens: DEFAULT_MAX_BUDGET_TOKENS,
            max_duration_ms: DEFAULT_MAX_DURATION_MS,
            require_opt_in_by_default: true,
            isolate_memory: true,
            allowed_risk: RiskLevel::Sandboxed,
        }
    }
}

impl PlayPolicy {
    /// 该风险是否被本 policy 允许（按 `Safe < Sandboxed < NeedsApproval` 取上界）。
    #[must_use]
    pub fn allows(&self, risk: RiskLevel) -> bool {
        risk <= self.allowed_risk
    }
}

/// [`PlayLedger::propose`] 的输入：候选主题与调用方当前可用额度。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayContext {
    /// 候选练习主题；按给定顺序尝试，第一个合规者胜出（确定性 first-fit）。
    pub candidates: Vec<PlayTopic>,
    /// 调用方当前可用 token 额度；`u64::MAX` 表示只受 policy 上限约束。
    pub budget_available_tokens: u64,
    /// 调用方当前可用时长（毫秒）；`u64::MAX` 表示只受 policy 上限约束。
    pub duration_available_ms: u64,
}

impl PlayContext {
    /// 用候选主题与可用额度构造上下文。
    #[must_use]
    pub fn new(
        candidates: Vec<PlayTopic>,
        budget_available_tokens: u64,
        duration_available_ms: u64,
    ) -> Self {
        Self {
            candidates,
            budget_available_tokens,
            duration_available_ms,
        }
    }
}

/// 练习 session 的生命周期状态：`Planned -> Running -> Finished | Aborted`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayStatus {
    /// 已计划：预算已预留，尚未开始执行。
    Planned,
    /// 运行中：练习正在进行，只能前进到 `Finished` 或 `Aborted`。
    Running,
    /// 已结束：终态，练习产出已写入 [`PlaySession::outcome`]。
    Finished,
    /// 已中止：终态，只能由 `Running` 进入；重复 abort 幂等，不产生 [`PlayOutcome`]。
    Aborted,
}

impl PlayStatus {
    /// 是否仍可推进（`Planned` / `Running`）。
    #[must_use]
    pub fn is_pending(self) -> bool {
        matches!(self, Self::Planned | Self::Running)
    }
}

/// 一场练习的记录。
///
/// 字段公开便于序列化与上层投影；但**状态迁移必须走 [`PlayLedger`] 的 API**，
/// 直接改写 `status` / `outcome` 不在本内核的契约内。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaySession {
    /// 调用方注入的 session id（同一账本内唯一）。
    pub id: String,
    /// 主题名，原样来自提案。
    pub topic: String,
    /// 本场预留预算（tokens），来自提案估计值。
    pub budget_tokens: u64,
    /// 启动时刻（Unix 毫秒，由调用方注入；本模块不读时钟）。
    pub started_ms: u64,
    /// 生命周期状态。
    pub status: PlayStatus,
    /// 练习产出；仅在 `Finished` 后为 `Some`。
    pub outcome: Option<PlayOutcome>,
}

impl PlaySession {
    /// `Planned -> Running`。
    fn begin(&mut self) -> Result<(), PlayError> {
        match self.status {
            PlayStatus::Planned => {
                self.status = PlayStatus::Running;
                Ok(())
            }
            from => Err(PlayError::InvalidTransition {
                from,
                to: PlayStatus::Running,
            }),
        }
    }

    /// `Running -> Finished`；实际消耗不得超过预留预算。
    fn finish(&mut self, outcome: PlayOutcome) -> Result<(), PlayError> {
        if self.status != PlayStatus::Running {
            return Err(PlayError::InvalidTransition {
                from: self.status,
                to: PlayStatus::Finished,
            });
        }
        if outcome.budget_used > self.budget_tokens {
            return Err(PlayError::BudgetUsedExceedsReserved {
                used: outcome.budget_used,
                reserved: self.budget_tokens,
            });
        }
        self.outcome = Some(outcome);
        self.status = PlayStatus::Finished;
        Ok(())
    }

    /// `Running -> Aborted`；对已 `Aborted` 幂等，`Planned` / `Finished` 不可直接
    /// 进入终态。
    fn abort(&mut self) -> Result<(), PlayError> {
        match self.status {
            PlayStatus::Aborted => Ok(()),
            PlayStatus::Running => {
                self.status = PlayStatus::Aborted;
                Ok(())
            }
            from @ (PlayStatus::Planned | PlayStatus::Finished) => {
                Err(PlayError::InvalidTransition {
                    from,
                    to: PlayStatus::Aborted,
                })
            }
        }
    }
}

/// 练习产出：只提炼技能与产物，**不写主体记忆**。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayOutcome {
    /// 本场练习学到的技能（可复用、可交给 W3 的技能账）。
    pub learned: Vec<String>,
    /// 本场产生的沙箱产物（文件、记录、样例等，原样带出）。
    pub artifacts: Vec<String>,
    /// 实际消耗预算（tokens），不得超过 session 预留值。
    pub budget_used: u64,
}

/// 练习内核的错误。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlayError {
    /// 提案要求用户 opt-in，但启动时没有显式 opt-in。
    #[error("该练习提案需要用户显式 opt-in，不能自动开始")]
    OptInRequired,
    /// 记忆隔离被关闭；play 不允许在可能污染主体记忆的 policy 下运行。
    #[error("记忆隔离关闭时禁止启动练习（play 不写主体记忆）")]
    MemoryIsolationRequired,
    /// 同一账本内 session id 重复。
    #[error("session 已存在: {0}")]
    DuplicateSession(String),
    /// 找不到指定 session。
    #[error("session 不存在: {0}")]
    SessionNotFound(String),
    /// 非法状态迁移（只允许 Planned -> Running -> Finished/Aborted）。
    #[error("非法状态迁移: {from:?} -> {to:?}")]
    InvalidTransition {
        /// 迁移前状态。
        from: PlayStatus,
        /// 迁移后状态。
        to: PlayStatus,
    },
    /// 提案预算超过 policy 上限。
    #[error("预算超限: 请求 {requested} tokens, 上限 {cap}")]
    BudgetExceeded {
        /// 提案请求的预算。
        requested: u64,
        /// policy 允许的上限。
        cap: u64,
    },
    /// 提案时长超过 policy 上限。
    #[error("时长超限: 请求 {requested} ms, 上限 {cap}")]
    DurationExceeded {
        /// 提案请求的时长。
        requested: u64,
        /// policy 允许的上限。
        cap: u64,
    },
    /// 实际消耗超过 session 预留预算。
    #[error("实际消耗 {used} tokens 超过预留 {reserved}")]
    BudgetUsedExceedsReserved {
        /// 实际消耗。
        used: u64,
        /// 预留预算。
        reserved: u64,
    },
}

/// 练习账本：持有 policy 与全部 session，是模块唯一的可变状态容器。
///
/// 账本不落盘、不读时钟；持久化与恢复由上层负责。默认构造使用
/// [`PlayPolicy::default`]。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayLedger {
    policy: PlayPolicy,
    sessions: Vec<PlaySession>,
}

impl Default for PlayLedger {
    /// 用保守默认 policy 构造空账本。
    fn default() -> Self {
        Self::new(PlayPolicy::default())
    }
}

impl PlayLedger {
    /// 用指定 policy 构造空账本。
    #[must_use]
    pub fn new(policy: PlayPolicy) -> Self {
        Self {
            policy,
            sessions: Vec::new(),
        }
    }

    /// 当前 policy（只读）。
    #[must_use]
    pub fn policy(&self) -> &PlayPolicy {
        &self.policy
    }

    /// 全部 session（按插入顺序；含终态，供审计与序列化）。
    #[must_use]
    pub fn sessions(&self) -> &[PlaySession] {
        &self.sessions
    }

    /// 按 policy 过滤候选主题，返回第一条放得下的练习提案。
    ///
    /// 过滤顺序：`isolate_memory` 必须为真 → 风险 `<= allowed_risk` → 估算预算与
    /// 时长 `<= min(policy 上限, 调用方可用额度)`。第一个同时满足全部条件的候选
    /// 胜出；全部不合格时返回 `None`（不会缩水伪造提案）。
    ///
    /// 估算公式（确定性、饱和算术）：
    /// `2 000 + 1 500 * 技能数` tokens、`60 000 + 120 000 * 技能数` 毫秒。
    #[must_use]
    pub fn propose(&self, context: &PlayContext) -> Option<PlayProposal> {
        if !self.policy.isolate_memory {
            return None;
        }
        let budget_cap = self
            .policy
            .max_budget_tokens
            .min(context.budget_available_tokens);
        let duration_cap = self
            .policy
            .max_duration_ms
            .min(context.duration_available_ms);
        for topic in &context.candidates {
            if !self.policy.allows(topic.risk) {
                continue;
            }
            let budget = estimate_budget(topic.skills.len());
            let duration = estimate_duration(topic.skills.len());
            if budget > budget_cap || duration > duration_cap {
                continue;
            }
            let needs_user_opt_in =
                self.policy.require_opt_in_by_default || topic.risk == RiskLevel::NeedsApproval;
            return Some(PlayProposal {
                topic: topic.topic.clone(),
                skill_targets: topic.skills.clone(),
                estimated_budget_tokens: budget,
                estimated_duration_ms: duration,
                needs_user_opt_in,
                rationale: build_rationale(
                    topic,
                    budget,
                    duration,
                    budget_cap,
                    duration_cap,
                    self.policy.allowed_risk,
                    needs_user_opt_in,
                ),
            });
        }
        None
    }

    /// 启动一场练习：登记 session 并进入 `Running`。
    ///
    /// 启动前重新校验（不信任外部提案）：记忆隔离必须开启；`needs_user_opt_in`
    /// 的提案、或 policy 默认要求 opt-in 时，必须收到 `user_opt_in = true`；估算值
    /// 不得超过 policy 上限；id 不得重复。任何一项失败都不会留下半启动 session。
    ///
    /// 提案应来自 [`PlayLedger::propose`]：手写提案无法携带主题风险等级，风险门控
    /// 以 `propose` 为准。
    ///
    /// `now_ms` 由调用方注入；本模块不读系统时钟。
    pub fn start(
        &mut self,
        proposal: &PlayProposal,
        id: impl Into<String>,
        now_ms: u64,
        user_opt_in: bool,
    ) -> Result<&PlaySession, PlayError> {
        if !self.policy.isolate_memory {
            return Err(PlayError::MemoryIsolationRequired);
        }
        let opt_in_required = self.policy.require_opt_in_by_default || proposal.needs_user_opt_in;
        if opt_in_required && !user_opt_in {
            return Err(PlayError::OptInRequired);
        }
        if proposal.estimated_budget_tokens > self.policy.max_budget_tokens {
            return Err(PlayError::BudgetExceeded {
                requested: proposal.estimated_budget_tokens,
                cap: self.policy.max_budget_tokens,
            });
        }
        if proposal.estimated_duration_ms > self.policy.max_duration_ms {
            return Err(PlayError::DurationExceeded {
                requested: proposal.estimated_duration_ms,
                cap: self.policy.max_duration_ms,
            });
        }
        let id = id.into();
        if self.sessions.iter().any(|session| session.id == id) {
            return Err(PlayError::DuplicateSession(id));
        }
        let mut session = PlaySession {
            id,
            topic: proposal.topic.clone(),
            budget_tokens: proposal.estimated_budget_tokens,
            started_ms: now_ms,
            status: PlayStatus::Planned,
            outcome: None,
        };
        session.begin()?;
        self.sessions.push(session);
        let index = self.sessions.len() - 1;
        Ok(&self.sessions[index])
    }

    /// 结束一场练习：`Running -> Finished`，产出写入 [`PlayOutcome`]。
    ///
    /// `outcome.budget_used` 不得超过 session 预留预算；失败时 session 保持
    /// `Running`，不产生半成品终态。
    pub fn finish(&mut self, id: &str, outcome: PlayOutcome) -> Result<&PlaySession, PlayError> {
        let session = self
            .sessions
            .iter_mut()
            .find(|session| session.id == id)
            .ok_or_else(|| PlayError::SessionNotFound(id.to_owned()))?;
        session.finish(outcome)?;
        Ok(session)
    }

    /// 中止一场练习：`Running -> Aborted`。
    ///
    /// 对已 `Aborted` 的 session 重复调用是幂等的（不改变任何字段）；`Planned`
    /// 不能跳过 `Running` 直接中止，`Finished` 是终态也不能被回退。
    pub fn abort(&mut self, id: &str) -> Result<&PlaySession, PlayError> {
        let session = self
            .sessions
            .iter_mut()
            .find(|session| session.id == id)
            .ok_or_else(|| PlayError::SessionNotFound(id.to_owned()))?;
        session.abort()?;
        Ok(session)
    }

    /// 尚未终结的 session（`Planned` / `Running`），按插入顺序。
    #[must_use]
    pub fn pending(&self) -> Vec<&PlaySession> {
        self.sessions
            .iter()
            .filter(|session| session.status.is_pending())
            .collect()
    }
}

/// 预算估计：基线 + 每技能增量，饱和算术不 panic。
fn estimate_budget(skill_count: usize) -> u64 {
    let count = u64::try_from(skill_count).unwrap_or(u64::MAX);
    ESTIMATE_BASE_TOKENS.saturating_add(ESTIMATE_TOKENS_PER_SKILL.saturating_mul(count))
}

/// 时长估计：基线 + 每技能增量，饱和算术不 panic。
fn estimate_duration(skill_count: usize) -> u64 {
    let count = u64::try_from(skill_count).unwrap_or(u64::MAX);
    ESTIMATE_BASE_DURATION_MS.saturating_add(ESTIMATE_DURATION_PER_SKILL_MS.saturating_mul(count))
}

/// 生成稳定、可审计的提案理由（无随机、无时钟、无浮点）。
fn build_rationale(
    topic: &PlayTopic,
    budget: u64,
    duration: u64,
    budget_cap: u64,
    duration_cap: u64,
    allowed_risk: RiskLevel,
    needs_user_opt_in: bool,
) -> String {
    let opt_in = if needs_user_opt_in {
        "需用户 opt-in 后才可开始"
    } else {
        "policy 允许无需额外 opt-in"
    };
    format!(
        "练习「{}」：风险 {}（allowed_risk={}）；预计 {} tokens / {} ms\
         （有效上限 {} tokens / {} ms）；技能 {} 项；{}。",
        topic.topic,
        topic.risk.label(),
        allowed_risk.label(),
        budget,
        duration,
        budget_cap,
        duration_cap,
        topic.skills.len(),
        opt_in
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 固定注入时刻（2023-11-14T22:13:20Z 附近）；测试永不读系统时钟。
    const NOW: u64 = 1_700_000_000_000;

    fn topic(name: &str, skills: &[&str], risk: RiskLevel) -> PlayTopic {
        PlayTopic {
            topic: name.into(),
            skills: skills.iter().map(|skill| (*skill).into()).collect(),
            risk,
        }
    }

    fn skills(count: usize) -> Vec<&'static str> {
        // 固定技能名，保证测试可复现。
        let pool = [
            "观察", "拆解", "复述", "试错", "复盘", "迁移", "提问", "记录",
        ];
        (0..count).map(|index| pool[index % pool.len()]).collect()
    }

    fn context(candidates: Vec<PlayTopic>, budget: u64, duration: u64) -> PlayContext {
        PlayContext::new(candidates, budget, duration)
    }

    fn open_context(candidates: Vec<PlayTopic>) -> PlayContext {
        context(candidates, u64::MAX, u64::MAX)
    }

    fn outcome(used: u64) -> PlayOutcome {
        PlayOutcome {
            learned: vec!["拆解".into()],
            artifacts: vec!["sandbox/demo.txt".into()],
            budget_used: used,
        }
    }

    fn small_topic() -> PlayTopic {
        topic("预演一次捕猎路线", &skills(2), RiskLevel::Sandboxed)
    }

    fn proposal_for(ledger: &PlayLedger, candidate: PlayTopic) -> PlayProposal {
        ledger
            .propose(&open_context(vec![candidate]))
            .expect("候选应能通过默认 policy")
    }

    #[test]
    fn policy_defaults_are_conservative() {
        let policy = PlayPolicy::default();
        assert_eq!(policy.max_budget_tokens, 20_000);
        assert_eq!(policy.max_duration_ms, 900_000);
        assert!(policy.require_opt_in_by_default);
        assert!(policy.isolate_memory);
        assert_eq!(policy.allowed_risk, RiskLevel::Sandboxed);
        assert!(policy.allows(RiskLevel::Safe));
        assert!(policy.allows(RiskLevel::Sandboxed));
        assert!(!policy.allows(RiskLevel::NeedsApproval));
    }

    #[test]
    fn proposal_estimate_never_exceeds_policy_caps() {
        let ledger = PlayLedger::default();
        // 20 项技能：预算 32 000 > 20 000，时长 2 460 000 > 900 000。
        assert!(
            ledger
                .propose(&open_context(vec![topic(
                    "过大练习",
                    &skills(20),
                    RiskLevel::Safe
                )]))
                .is_none(),
            "放不下的候选必须返回 None"
        );

        // 7 项技能刚好压线：2 000 + 1 500*7 = 12 500；60 000 + 120 000*7 = 900 000。
        let proposal = proposal_for(&ledger, topic("压线练习", &skills(7), RiskLevel::Safe));
        assert_eq!(proposal.estimated_budget_tokens, 12_500);
        assert_eq!(proposal.estimated_duration_ms, 900_000);
        assert!(proposal.estimated_budget_tokens <= ledger.policy().max_budget_tokens);
        assert!(proposal.estimated_duration_ms <= ledger.policy().max_duration_ms);
        assert_eq!(proposal.skill_targets.len(), 7);
    }

    #[test]
    fn proposal_respects_caller_headroom_below_policy() {
        let ledger = PlayLedger::default();
        let (budget, duration) = (10_000, 300_000);
        // 7 项技能超预算/超时长 → 跳过；2 项技能 5 000 / 300 000 刚好放进额度。
        let headroom_context = context(
            vec![
                topic("过大", &skills(7), RiskLevel::Safe),
                topic("刚好", &skills(2), RiskLevel::Safe),
            ],
            budget,
            duration,
        );
        let proposal = ledger
            .propose(&headroom_context)
            .expect("第二条候选应能放下");
        assert_eq!(proposal.topic, "刚好");
        assert_eq!(proposal.estimated_budget_tokens, 5_000);
        assert_eq!(proposal.estimated_duration_ms, 300_000);
        assert!(proposal.estimated_budget_tokens <= budget);
        assert!(proposal.estimated_duration_ms <= duration);

        // 额度连基线都放不下 → None。
        assert!(ledger
            .propose(&context(vec![small_topic()], 1_999, 60_000))
            .is_none());
    }

    #[test]
    fn proposal_is_deterministic_and_first_fit() {
        let ledger = PlayLedger::default();
        let candidates = vec![
            topic("需批准", &skills(1), RiskLevel::NeedsApproval),
            topic("超时长", &skills(8), RiskLevel::Safe),
            topic("入选", &skills(2), RiskLevel::Sandboxed),
        ];
        let first = ledger.propose(&open_context(candidates.clone()));
        let second = ledger.propose(&open_context(candidates));
        assert_eq!(first, second, "同一输入必须得到同一提案");
        assert_eq!(first.expect("应有提案").topic, "入选");
    }

    #[test]
    fn needs_approval_risk_is_never_auto_started() {
        let default_ledger = PlayLedger::default();
        assert!(
            default_ledger
                .propose(&open_context(vec![topic(
                    "真实环境练习",
                    &skills(1),
                    RiskLevel::NeedsApproval
                )]))
                .is_none(),
            "默认 allowed_risk=Sandboxed，NeedsApproval 不得进入提案"
        );

        // 即使 policy 放宽到 NeedsApproval 且默认不要求 opt-in，该风险仍必须 opt-in。
        let policy = PlayPolicy {
            allowed_risk: RiskLevel::NeedsApproval,
            require_opt_in_by_default: false,
            ..PlayPolicy::default()
        };
        let mut ledger = PlayLedger::new(policy);
        let proposal = proposal_for(
            &ledger,
            topic("真实环境练习", &skills(1), RiskLevel::NeedsApproval),
        );
        assert!(proposal.needs_user_opt_in);
        assert!(matches!(
            ledger.start(&proposal, "s-needs-approval", NOW, false),
            Err(PlayError::OptInRequired)
        ));
        assert!(ledger.pending().is_empty(), "被拒绝的启动不得留下 session");
        let session = ledger
            .start(&proposal, "s-needs-approval", NOW, true)
            .expect("显式 opt-in 后应能启动");
        assert_eq!(session.status, PlayStatus::Running);
    }

    #[test]
    fn safe_topic_can_skip_opt_in_only_when_policy_says_so() {
        let policy = PlayPolicy {
            require_opt_in_by_default: false,
            ..PlayPolicy::default()
        };
        let mut ledger = PlayLedger::new(policy);
        let proposal = proposal_for(&ledger, topic("纯推理预演", &skills(2), RiskLevel::Safe));
        assert!(!proposal.needs_user_opt_in);
        let session = ledger
            .start(&proposal, "s-safe", NOW, false)
            .expect("policy 允许时 Safe 练习可无需 opt-in");
        assert_eq!(session.status, PlayStatus::Running);
    }

    #[test]
    fn default_policy_requires_opt_in_for_every_proposal() {
        let mut ledger = PlayLedger::default();
        let proposal = proposal_for(&ledger, small_topic());
        assert!(proposal.needs_user_opt_in);
        assert!(proposal.rationale.contains("opt-in"));
        assert!(matches!(
            ledger.start(&proposal, "s-gated", NOW, false),
            Err(PlayError::OptInRequired)
        ));
        assert!(ledger.sessions().is_empty());

        // 即使外部手写提案把 needs_user_opt_in 改成 false，默认 policy 仍要求 opt-in。
        let mut forged = proposal.clone();
        forged.needs_user_opt_in = false;
        assert!(matches!(
            ledger.start(&forged, "s-forged", NOW, false),
            Err(PlayError::OptInRequired)
        ));
        assert!(ledger.sessions().is_empty());
        assert!(ledger.start(&proposal, "s-gated", NOW, true).is_ok());
    }

    #[test]
    fn start_rejects_foreign_proposal_beyond_policy() {
        let mut ledger = PlayLedger::default();
        let mut proposal = proposal_for(&ledger, small_topic());
        proposal.estimated_budget_tokens = DEFAULT_MAX_BUDGET_TOKENS + 1;
        assert!(matches!(
            ledger.start(&proposal, "s-over-budget", NOW, true),
            Err(PlayError::BudgetExceeded {
                requested: 20_001,
                cap: 20_000
            })
        ));
        assert!(ledger.sessions().is_empty());

        let mut proposal = proposal_for(&ledger, small_topic());
        proposal.estimated_duration_ms = DEFAULT_MAX_DURATION_MS + 1;
        assert!(matches!(
            ledger.start(&proposal, "s-over-duration", NOW, true),
            Err(PlayError::DurationExceeded {
                requested: 900_001,
                cap: 900_000
            })
        ));
        assert!(ledger.sessions().is_empty());
    }

    #[test]
    fn state_machine_is_planned_to_running_to_terminal() {
        let mut session = PlaySession {
            id: "s-machine".into(),
            topic: "状态机".into(),
            budget_tokens: 1_000,
            started_ms: NOW,
            status: PlayStatus::Planned,
            outcome: None,
        };
        // Planned 不能直接 finish / abort 到终态？abort 允许 Planned -> Aborted；
        // finish 必须先经过 Running。
        assert!(matches!(
            session.finish(outcome(10)),
            Err(PlayError::InvalidTransition {
                from: PlayStatus::Planned,
                to: PlayStatus::Finished
            })
        ));
        session.begin().expect("Planned -> Running 合法");
        assert_eq!(session.status, PlayStatus::Running);
        assert!(matches!(
            session.begin(),
            Err(PlayError::InvalidTransition {
                from: PlayStatus::Running,
                to: PlayStatus::Running
            })
        ));
        session
            .finish(outcome(10))
            .expect("Running -> Finished 合法");
        assert_eq!(session.status, PlayStatus::Finished);
        assert!(session.outcome.is_some());
        assert!(matches!(
            session.abort(),
            Err(PlayError::InvalidTransition {
                from: PlayStatus::Finished,
                to: PlayStatus::Aborted
            })
        ));
        assert!(matches!(
            session.finish(outcome(10)),
            Err(PlayError::InvalidTransition {
                from: PlayStatus::Finished,
                to: PlayStatus::Finished
            })
        ));

        // Planned 不能跳过 Running 直接进入终态。
        let mut planned = PlaySession {
            status: PlayStatus::Planned,
            ..session
        };
        assert!(matches!(
            planned.abort(),
            Err(PlayError::InvalidTransition {
                from: PlayStatus::Planned,
                to: PlayStatus::Aborted
            })
        ));
        planned.begin().expect("Planned -> Running 合法");
        planned.abort().expect("Running -> Aborted 合法");
        assert_eq!(planned.status, PlayStatus::Aborted);
        assert!(planned.begin().is_err());
        assert!(planned.finish(outcome(1)).is_err());
    }

    #[test]
    fn abort_is_idempotent_and_keeps_session_fields() {
        let mut ledger = PlayLedger::default();
        let proposal = proposal_for(&ledger, small_topic());
        ledger
            .start(&proposal, "s-abort", NOW, true)
            .expect("默认 opt-in 后应能启动");
        let first = ledger
            .abort("s-abort")
            .expect("Running -> Aborted 合法")
            .clone();
        assert_eq!(first.status, PlayStatus::Aborted);
        assert!(first.outcome.is_none());
        assert!(ledger.pending().is_empty());

        let second = ledger
            .abort("s-abort")
            .expect("重复 abort 必须幂等")
            .clone();
        assert_eq!(first, second, "重复 abort 不得改变任何字段");
        assert_eq!(ledger.sessions().len(), 1);
    }

    #[test]
    fn finish_records_outcome_and_guards_reserved_budget() {
        let mut ledger = PlayLedger::default();
        let proposal = proposal_for(&ledger, small_topic());
        ledger
            .start(&proposal, "s-finish", NOW, true)
            .expect("默认 opt-in 后应能启动");

        // 超预留：拒绝且保持 Running，不产生半成品终态。
        assert!(matches!(
            ledger.finish("s-finish", outcome(proposal.estimated_budget_tokens + 1)),
            Err(PlayError::BudgetUsedExceedsReserved { .. })
        ));
        assert_eq!(ledger.sessions()[0].status, PlayStatus::Running);
        assert!(ledger.sessions()[0].outcome.is_none());

        let recorded = ledger
            .finish("s-finish", outcome(1_234))
            .expect("Running -> Finished 合法");
        assert_eq!(recorded.status, PlayStatus::Finished);
        assert_eq!(
            recorded.outcome.as_ref().expect("应有产出").budget_used,
            1_234
        );
        assert_eq!(
            recorded.outcome.as_ref().expect("应有产出").learned,
            vec!["拆解".to_string()]
        );
        assert!(ledger.pending().is_empty());
        assert!(matches!(
            ledger.finish("s-finish", outcome(0)),
            Err(PlayError::InvalidTransition {
                from: PlayStatus::Finished,
                to: PlayStatus::Finished
            })
        ));
    }

    #[test]
    fn duplicate_session_id_is_rejected_without_second_session() {
        let mut ledger = PlayLedger::default();
        let proposal = proposal_for(&ledger, small_topic());
        ledger
            .start(&proposal, "s-dup", NOW, true)
            .expect("首次启动应成功");
        assert!(matches!(
            ledger.start(&proposal, "s-dup", NOW + 1, true),
            Err(PlayError::DuplicateSession(id)) if id == "s-dup"
        ));
        assert_eq!(ledger.sessions().len(), 1);
    }

    #[test]
    fn unknown_session_ids_are_reported() {
        let mut ledger = PlayLedger::default();
        assert!(matches!(
            ledger.abort("missing"),
            Err(PlayError::SessionNotFound(id)) if id == "missing"
        ));
        assert!(matches!(
            ledger.finish("missing", outcome(0)),
            Err(PlayError::SessionNotFound(id)) if id == "missing"
        ));
    }

    #[test]
    fn disabled_memory_isolation_refuses_propose_and_start() {
        let policy = PlayPolicy {
            isolate_memory: false,
            ..PlayPolicy::default()
        };
        let mut ledger = PlayLedger::new(policy);
        assert!(
            ledger.propose(&open_context(vec![small_topic()])).is_none(),
            "关闭记忆隔离时不得提案"
        );
        // 即使外部自己造一个提案，start 也必须拒绝。
        let proposal = PlayProposal {
            topic: "外部提案".into(),
            skill_targets: vec!["拆解".into()],
            estimated_budget_tokens: 1_000,
            estimated_duration_ms: 60_000,
            needs_user_opt_in: false,
            rationale: "外部构造".into(),
        };
        assert!(matches!(
            ledger.start(&proposal, "s-leak", NOW, true),
            Err(PlayError::MemoryIsolationRequired)
        ));
        assert!(ledger.sessions().is_empty());
    }

    #[test]
    fn pending_lists_only_non_terminal_sessions_in_order() {
        let mut ledger = PlayLedger::default();
        let proposal = proposal_for(&ledger, small_topic());
        ledger.start(&proposal, "s-1", NOW, true).expect("启动 s-1");
        ledger
            .start(&proposal, "s-2", NOW + 1, true)
            .expect("启动 s-2");
        let ids: Vec<&str> = ledger
            .pending()
            .iter()
            .map(|session| session.id.as_str())
            .collect();
        assert_eq!(ids, vec!["s-1", "s-2"]);

        ledger.finish("s-1", outcome(1)).expect("结束 s-1");
        let ids: Vec<&str> = ledger
            .pending()
            .iter()
            .map(|session| session.id.as_str())
            .collect();
        assert_eq!(ids, vec!["s-2"]);
        ledger.abort("s-2").expect("中止 s-2");
        assert!(ledger.pending().is_empty());
    }

    /// 机制不变量：练习产出只进 [`PlayOutcome`]，模块内不存在写主体记忆的 API。
    ///
    /// Rust 没有运行时反射，这里用源码扫描钉死禁止出现的主体记忆写入符号；再用
    /// JSON 形状钉死 [`PlayOutcome`] 只有技能 / 产物 / 消耗三个字段。
    #[test]
    fn practice_outputs_only_live_in_play_outcome() {
        let source = include_str!("play.rs");
        let forbidden = [
            concat!("fn ", "remember"),
            concat!("fn ", "write_memory"),
            concat!("Memory", "Store"),
            concat!("Episode", "Store"),
            concat!("save_", "episode"),
            concat!("memory_", "write"),
        ];
        for needle in forbidden {
            assert!(
                !source.contains(needle),
                "play 内核不得暴露主体记忆写入接口: {needle}"
            );
        }

        let value = serde_json::to_value(outcome(7)).expect("PlayOutcome 应可序列化");
        let object = value.as_object().expect("PlayOutcome 序列化为对象");
        assert_eq!(object.len(), 3);
        assert!(object.contains_key("learned"));
        assert!(object.contains_key("artifacts"));
        assert!(object.contains_key("budget_used"));
    }

    #[test]
    fn estimates_saturate_instead_of_overflowing() {
        assert_eq!(estimate_budget(0), ESTIMATE_BASE_TOKENS);
        assert_eq!(estimate_duration(0), ESTIMATE_BASE_DURATION_MS);
        assert_eq!(estimate_budget(usize::MAX), u64::MAX);
        assert_eq!(estimate_duration(usize::MAX), u64::MAX);
    }

    #[test]
    fn core_types_round_trip_through_json() {
        let mut ledger = PlayLedger::default();
        let proposal = proposal_for(&ledger, small_topic());
        let context = open_context(vec![small_topic()]);
        let session = ledger
            .start(&proposal, "s-json", NOW, true)
            .expect("启动")
            .clone();

        for encoded in [
            serde_json::to_value(&proposal).expect("proposal 序列化"),
            serde_json::to_value(&context).expect("context 序列化"),
            serde_json::to_value(&session).expect("session 序列化"),
            serde_json::to_value(outcome(3)).expect("outcome 序列化"),
            serde_json::to_value(PlayPolicy::default()).expect("policy 序列化"),
            serde_json::to_value(&ledger).expect("ledger 序列化"),
        ] {
            assert!(encoded.is_object(), "序列化结果应为 JSON 对象");
        }

        let decoded: PlayProposal =
            serde_json::from_value(serde_json::to_value(&proposal).expect("序列化"))
                .expect("proposal 反序列化");
        assert_eq!(decoded, proposal);
        let decoded: PlaySession =
            serde_json::from_value(serde_json::to_value(&session).expect("序列化"))
                .expect("session 反序列化");
        assert_eq!(decoded, session);
    }
}
