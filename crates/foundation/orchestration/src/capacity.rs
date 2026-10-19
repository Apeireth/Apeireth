//! `apeireth-orchestration::capacity` — 生物机制族 W2 批: 承载力 / 演替 / 关键种内核 v0 (SG-26).
//!
//! # 生物原型
//! - **承载力 (carrying capacity)**: 一个环境能长期供养的种群/资源量有上限。接近软上限时
//!   个体质量与恢复力先退化（过度放牧、拥堵）；越过硬上限则新个体无法进入。
//! - **软/硬两级**: 软上限是「开始退化」的告警线，硬上限是「拒绝新负载」的边界；对应工程上
//!   的「接近配额 → 提前降载」与「配额耗尽 → 拒绝新请求」。
//! - **演替 (succession)**: 扰动后群落不是一步回到顶极，而是沿先锋 → 过渡 → 顶极的阶段序列
//!   逐步恢复；每一步都要重新验证，验证不通过则回退重新来。
//! - **关键种 (keystone species)**: 少数物种对群落结构的影响远超其生物量，它们倒下会触发
//!   连锁反应；工程上对应少数「一倒则全局风险」的模块（调度器/记忆库/工具注册表等）。
//!
//! # 工程边界
//! - **只做读数与判定**: 本模块只把注入读数变成 [`CapacityVerdict`]、把迁移请求推进
//!   [`SuccessionTracker`] 状态机、把模块健康度整理成风险清单；**不自动拒绝负载、不自动
//!   迁移、不自动回滚**，动作由接线层执行。
//! - **0 系统时钟 / 0 I/O**: 时间一律 `u64` 毫秒由调用方注入；无 `chrono`、无 `SystemTime`、
//!   无文件/网络/日志。
//! - **确定性**: [`CapacityLedger::evaluate`] 严格按输入顺序产出同长度结果（不使用 `HashMap`）；
//!   [`KeystoneRegistry::at_risk`] 按「criticality 降序 → 名字升序」全序排序；同输入 → 同输出。
//! - **退化输入有定义**: 空读数、零上限、`NaN`/无穷关键度、未知模块名都有文档化行为，不 panic；
//!   未知模块名不会被 [`KeystoneRegistry::report_health`] 隐式创建。
//! - **只读不改写**: [`CapacityLedger::evaluate`] 不修改读数；注册表写入只经
//!   [`KeystoneRegistry::register`] / [`KeystoneRegistry::report_health`] 两个显式 API。
//! - **0 新外部依赖**: 生产仅用 `serde` 派生；`serde_json` 只出现在测试。
//!
//! # IMPLEMENTED（未接线）
//! 本模块是机制内核 v0，落 **IMPLEMENTED** 级：
//! - 未接上下文预算（`context_budget`）、记忆配额（记忆条目上限）、工具注册表与任务队列；
//! - 无告警出口（不 emit 事件、不推 channel、不写日志）；
//! - 无 `self_status` 自述节；
//! - 生产路径 0 调用点（缺口见
//!   `docs/04-internal/backend-enhancement/system-gap-ledger-2026-10-18.md` SG-26）。
//!
//! W3 接线点: 调用方构造 [`CapacityReading`] → [`CapacityLedger::evaluate`] → 接线层执行拒绝/
//! 降级/告警；迁移由 [`SuccessionTracker`] 记录阶段与截止时间，超时回滚读
//! [`RollbackOutcome::late`] 标记。

use serde::{Deserialize, Serialize};

/// 接近软上限的比例（百分比）: 80%。[`CapacityLedger::evaluate`] 用整数比较
/// `used * 100 >= soft * 80`。
pub const NEAR_SOFT_PERCENT: u64 = 80;

/// 承载力维度: 上下文 token / 记忆条目 / 已注册工具 / 活跃任务.
///
/// 枚举声明顺序即规范展示顺序（[`CapacityKind::ALL`]），不随 `HashMap` 迭代顺序漂移。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapacityKind {
    /// 上下文 token 预算（单次会话/单次调用的上下文占用）。
    ContextTokens,
    /// 记忆条目配额（长期记忆库可容纳的条目数）。
    MemoryEntries,
    /// 已注册工具数（工具注册表容量）。
    RegisteredTools,
    /// 活跃任务数（调度器并发容量）。
    ActiveTasks,
}

impl CapacityKind {
    /// 规范展示顺序（[`CapacityLedger::evaluate`] 仍严格按调用方输入顺序输出，不按此重排）。
    pub const ALL: [Self; 4] = [
        Self::ContextTokens,
        Self::MemoryEntries,
        Self::RegisteredTools,
        Self::ActiveTasks,
    ];

    /// 稳定短名（日志/证据串用，不随 `Debug` 派生漂移）。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ContextTokens => "context_tokens",
            Self::MemoryEntries => "memory_entries",
            Self::RegisteredTools => "registered_tools",
            Self::ActiveTasks => "active_tasks",
        }
    }
}

/// 一个维度的两级上限: 软上限先退化、硬上限拒绝新负载.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapacityLimit {
    /// 该上限所属维度（与承载读数一致时由 [`CapacityReading::new`] 自动对齐）。
    pub kind: CapacityKind,
    /// 软上限: 达到即进入退化/告警区（判定见 [`CapacityVerdict::AtSoftLimit`]）。
    pub soft_limit: u64,
    /// 硬上限: 达到即拒绝新负载（判定优先级最高，见 [`CapacityVerdict::OverHardLimit`]）。
    pub hard_limit: u64,
}

impl CapacityLimit {
    /// 构造一个维度的两级上限（不做 `soft <= hard` 校验；错配时硬上限优先，见
    /// [`CapacityLedger::evaluate`]）。
    pub fn new(kind: CapacityKind, soft_limit: u64, hard_limit: u64) -> Self {
        Self {
            kind,
            soft_limit,
            hard_limit,
        }
    }
}

/// 一次承载读数: 某维度的已用量 + 该维度的两级上限.
///
/// `kind` 是读数主体维度；判定以 `limit` 的数值为准。两者不一致时**不报错、不 panic**，
/// 由调用方保证一致性（[`CapacityReading::new`] 会自动对齐）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapacityReading {
    /// 读数主体维度。
    pub kind: CapacityKind,
    /// 已用量（口径由维度决定: token 数 / 条目数 / 工具数 / 任务数）。
    pub used: u64,
    /// 该维度当前生效的两级上限。
    pub limit: CapacityLimit,
}

impl CapacityReading {
    /// 以维度、已用量、软/硬上限构造读数（`limit.kind` 自动设为 `kind`）。
    pub fn new(kind: CapacityKind, used: u64, soft_limit: u64, hard_limit: u64) -> Self {
        Self {
            kind,
            used,
            limit: CapacityLimit::new(kind, soft_limit, hard_limit),
        }
    }
}

/// 承载力判定: 由轻到重四档，声明顺序即严重度升序.
///
/// 判定优先级固定为 hard → soft → 80% 软上限 → healthy；[`Self::OverHardLimit`] 永远最高
/// （即使调用方给出 `soft > hard` 的错配上限）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapacityVerdict {
    /// 健康: 已用量严格低于软上限的 80%。
    Healthy,
    /// 接近软上限: 已用量达到软上限的 80%（含等号）但未到软上限。
    NearCapacity,
    /// 已到软上限: 已用量达到软上限但未到硬上限（进入退化区，应由接线层降载）。
    AtSoftLimit,
    /// 超硬上限: 已用量达到硬上限，应拒绝新负载（本模块只判定）。
    OverHardLimit,
}

impl CapacityVerdict {
    /// 稳定短名（日志/证据串用，不随 `Debug` 派生漂移）。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::NearCapacity => "near_capacity",
            Self::AtSoftLimit => "at_soft_limit",
            Self::OverHardLimit => "over_hard_limit",
        }
    }

    /// 是否应拒绝新负载（仅 [`Self::OverHardLimit`] 为真；拒绝动作由接线层执行）。
    pub fn rejects_new_load(self) -> bool {
        matches!(self, Self::OverHardLimit)
    }

    /// 是否健康（仅 [`Self::Healthy`] 为真）。
    pub fn is_healthy(self) -> bool {
        matches!(self, Self::Healthy)
    }
}

/// 承载力账本 v0: 无状态的判定器.
///
/// 设计为关联函数 [`Self::evaluate`]，与「注入读数 → 逐条判定」的纯函数语义一致；W3 若需要
/// 历史/策略可在此结构上追加字段，但 v0 不持有任何状态、不做 I/O。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapacityLedger;

impl CapacityLedger {
    /// 构造空账本（v0 无状态）。
    pub fn new() -> Self {
        Self
    }

    /// 评估一组承载读数，返回与输入**逐位对应**的判定（保持输入顺序，不使用 `HashMap`）。
    ///
    /// 每条读数独立判定，固定优先级（测试钉死）:
    /// 1. `used >= hard_limit` → [`CapacityVerdict::OverHardLimit`]（最高优先级；即使
    ///    `soft_limit > hard_limit` 也按硬上限判）；
    /// 2. `used >= soft_limit` → [`CapacityVerdict::AtSoftLimit`]；
    /// 3. `used * 100 >= soft_limit * 80`（80% 含等号）→ [`CapacityVerdict::NearCapacity`]；
    /// 4. 其余 → [`CapacityVerdict::Healthy`]。
    ///
    /// 退化语义（不 panic、无除零）: `hard_limit == 0` 时任何读数（含 `used == 0`）都判
    /// `OverHardLimit`（零容量 = 拒绝新负载，安全侧）；`soft_limit == 0` 时 `used == 0` 即判
    /// `AtSoftLimit`。空输入返回空 `Vec`。比较用 `u128` 展开，`u64::MAX` 附近也不溢出。
    pub fn evaluate(readings: &[CapacityReading]) -> Vec<CapacityVerdict> {
        readings.iter().map(verdict_for).collect()
    }
}

/// 单条读数的判定（纯函数；`u128` 防溢出）.
fn verdict_for(reading: &CapacityReading) -> CapacityVerdict {
    let used = u128::from(reading.used);
    let soft = u128::from(reading.limit.soft_limit);
    let hard = u128::from(reading.limit.hard_limit);

    if used >= hard {
        CapacityVerdict::OverHardLimit
    } else if used >= soft {
        CapacityVerdict::AtSoftLimit
    } else if used * 100 >= soft * u128::from(NEAR_SOFT_PERCENT) {
        CapacityVerdict::NearCapacity
    } else {
        CapacityVerdict::Healthy
    }
}

/// 演替阶段: 稳定顶极 / 迁移中 / 验证中.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SuccessionPhase {
    /// 稳定: 当前版本全量承载，无迁移窗口。
    #[default]
    Stable,
    /// 迁移中: 新旧版本共存，`old_ratio_allowed` 是允许保留的旧版本流量比例。
    Migrating,
    /// 验证中: 新版本已接管，等待验证通过或回滚。
    Verifying,
}

impl SuccessionPhase {
    /// 稳定短名。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Migrating => "migrating",
            Self::Verifying => "verifying",
        }
    }

    /// 是否处于稳定阶段。
    pub fn is_stable(self) -> bool {
        matches!(self, Self::Stable)
    }
}

/// 被状态机拒绝的迁移动作.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuccessionAction {
    /// 开始迁移（仅 Stable 合法）。
    BeginMigration,
    /// 推进到验证（仅 Migrating 合法）。
    Advance,
    /// 验证通过回到稳定（仅 Verifying 合法）。
    Verify,
    /// 回滚到稳定（仅 Migrating / Verifying 合法）。
    Rollback,
}

impl SuccessionAction {
    /// 稳定短名。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BeginMigration => "begin_migration",
            Self::Advance => "advance",
            Self::Verify => "verify",
            Self::Rollback => "rollback",
        }
    }
}

/// 迁移计划快照: 阶段 + 旧版本允许比例 + 起止时间.
///
/// `started_ms` / `rollback_deadline_ms` 由 [`SuccessionTracker::begin_migration`] 以注入时钟
/// 填写；`old_ratio_allowed` 在 `begin_migration` 时校验为 `0.0..=1.0` 的有限数，阶段推进
/// **不静默改写**它。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SuccessionPlan {
    /// 当前阶段。
    pub phase: SuccessionPhase,
    /// 迁移期间允许保留的旧版本流量比例（0.0 = 全切新版本，1.0 = 尚未切流量）。
    pub old_ratio_allowed: f32,
    /// 迁移开始时间（毫秒，调用方注入；0 表示尚未开始过迁移）。
    pub started_ms: u64,
    /// 回滚截止时间（毫秒，`started_ms + window_ms` 饱和加法；0 表示尚未开始过迁移）。
    pub rollback_deadline_ms: u64,
}

impl SuccessionPlan {
    /// 是否已越过回滚截止时间。
    ///
    /// 语义: 仅当阶段不是 [`SuccessionPhase::Stable`] 且 `at_ms > rollback_deadline_ms` 时为真；
    /// **恰好等于截止时刻不算超时**（含等号在「按时」一侧）。Stable 阶段没有回滚窗口，恒为
    /// `false`（历史时间戳只作审计）。
    pub fn deadline_exceeded(&self, at_ms: u64) -> bool {
        self.phase != SuccessionPhase::Stable && at_ms > self.rollback_deadline_ms
    }
}

/// 演替状态机拒绝动作的原因.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuccessionError {
    /// 当前阶段不允许该动作（非法迁移被拒绝，状态与时间戳保持不变）。
    IllegalTransition {
        /// 动作发生时的阶段。
        from: SuccessionPhase,
        /// 被拒绝的动作。
        action: SuccessionAction,
    },
    /// `old_ratio_allowed` 非法: 必须是 `0.0..=1.0` 的有限数（`NaN`/无穷/越界均拒绝）。
    InvalidOldRatio,
}

impl std::fmt::Display for SuccessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IllegalTransition { from, action } => {
                write!(f, "阶段 {} 不允许动作 {}", from.as_str(), action.as_str())
            }
            Self::InvalidOldRatio => {
                write!(f, "old_ratio_allowed 必须是 0.0..=1.0 的有限数")
            }
        }
    }
}

impl std::error::Error for SuccessionError {}

/// 一次回滚的结果（只报告事实，不执行动作）.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackOutcome {
    /// 是否超过回滚截止时间（超时必须被标记，即使回滚本身已发生）。
    pub late: bool,
    /// 超时毫秒数（按时回滚为 0）。
    pub overdue_ms: u64,
}

/// 演替状态机: `Stable → Migrating → Verifying → Stable`，`Migrating`/`Verifying` 可回滚.
///
/// 非法迁移一律返回 [`SuccessionError`] 且**不改变**已有状态（时间戳/比例/阶段原样保留）。
/// 本结构只记录状态与截止时间，不切流量、不发通知；`rollback` 也不会自动发生，必须由接线层
/// 在读到超时标记后显式调用。
///
/// serde 快照可直接持久化；反序列化**不校验**状态机合法性，恢复后应只经方法迁移。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuccessionTracker {
    /// 当前/最近一次迁移计划快照（回滚或验证通过后保留时间戳作审计）。
    plan: SuccessionPlan,
    /// 累计超时回滚次数（只增不减）。
    late_rollbacks: u64,
    /// 最近一次回滚是否超时（下次回滚时覆盖）。
    last_rollback_late: bool,
}

impl Default for SuccessionTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl SuccessionTracker {
    /// 新建处于 [`SuccessionPhase::Stable`] 的跟踪器。
    pub fn new() -> Self {
        Self {
            plan: SuccessionPlan::default(),
            late_rollbacks: 0,
            last_rollback_late: false,
        }
    }

    /// 当前计划快照（含阶段、旧版本比例、起止时间）。
    pub fn plan(&self) -> &SuccessionPlan {
        &self.plan
    }

    /// 当前阶段。
    pub fn phase(&self) -> SuccessionPhase {
        self.plan.phase
    }

    /// 是否已越过本次迁移的回滚截止时间（Stable 阶段恒为 `false`，见
    /// [`SuccessionPlan::deadline_exceeded`]）。
    pub fn deadline_exceeded(&self, at_ms: u64) -> bool {
        self.plan.deadline_exceeded(at_ms)
    }

    /// 累计超时回滚次数。
    pub fn late_rollbacks(&self) -> u64 {
        self.late_rollbacks
    }

    /// 最近一次回滚是否超时。
    pub fn last_rollback_late(&self) -> bool {
        self.last_rollback_late
    }

    /// 开始迁移: 仅 [`SuccessionPhase::Stable`] 合法.
    ///
    /// 成功时写入 `started_ms = at_ms`、`rollback_deadline_ms = at_ms + window_ms`（饱和到
    /// `u64::MAX`，不回绕），阶段进入 `Migrating`。`old_ratio_allowed` 必须为 `0.0..=1.0`
    /// 的有限数，否则返回 [`SuccessionError::InvalidOldRatio`] 且状态不变。
    pub fn begin_migration(
        &mut self,
        at_ms: u64,
        old_ratio_allowed: f32,
        window_ms: u64,
    ) -> Result<(), SuccessionError> {
        if self.plan.phase != SuccessionPhase::Stable {
            return Err(SuccessionError::IllegalTransition {
                from: self.plan.phase,
                action: SuccessionAction::BeginMigration,
            });
        }
        if !old_ratio_allowed.is_finite() || !(0.0..=1.0).contains(&old_ratio_allowed) {
            return Err(SuccessionError::InvalidOldRatio);
        }
        self.plan = SuccessionPlan {
            phase: SuccessionPhase::Migrating,
            old_ratio_allowed,
            started_ms: at_ms,
            rollback_deadline_ms: at_ms.saturating_add(window_ms),
        };
        Ok(())
    }

    /// 推进到验证: 仅 [`SuccessionPhase::Migrating`] 合法（`old_ratio_allowed` 原样保留）。
    pub fn advance(&mut self) -> Result<(), SuccessionError> {
        if self.plan.phase != SuccessionPhase::Migrating {
            return Err(SuccessionError::IllegalTransition {
                from: self.plan.phase,
                action: SuccessionAction::Advance,
            });
        }
        self.plan.phase = SuccessionPhase::Verifying;
        Ok(())
    }

    /// 验证通过回到稳定: 仅 [`SuccessionPhase::Verifying`] 合法（时间戳保留作审计）。
    pub fn verify(&mut self) -> Result<(), SuccessionError> {
        if self.plan.phase != SuccessionPhase::Verifying {
            return Err(SuccessionError::IllegalTransition {
                from: self.plan.phase,
                action: SuccessionAction::Verify,
            });
        }
        self.plan.phase = SuccessionPhase::Stable;
        Ok(())
    }

    /// 回滚到稳定: 仅 [`SuccessionPhase::Migrating`] / [`SuccessionPhase::Verifying`] 合法.
    ///
    /// 回滚**总是**回到 `Stable`；是否超时按注入的 `at_ms` 与 `rollback_deadline_ms` 比较
    /// （`at_ms > deadline` 为超时，恰好等于截止时刻按按时处理）。超时不会被拒绝，但会通过
    /// [`RollbackOutcome::late`] / [`Self::late_rollbacks`] / [`Self::last_rollback_late`]
    /// 三处标记；`overdue_ms` 给出超时毫秒数。回滚本身由调用方执行，本方法只落状态。
    pub fn rollback(&mut self, at_ms: u64) -> Result<RollbackOutcome, SuccessionError> {
        if !matches!(
            self.plan.phase,
            SuccessionPhase::Migrating | SuccessionPhase::Verifying
        ) {
            return Err(SuccessionError::IllegalTransition {
                from: self.plan.phase,
                action: SuccessionAction::Rollback,
            });
        }

        let deadline = self.plan.rollback_deadline_ms;
        let late = at_ms > deadline;
        let overdue_ms = if late { at_ms - deadline } else { 0 };

        self.plan.phase = SuccessionPhase::Stable;
        self.last_rollback_late = late;
        if late {
            self.late_rollbacks = self.late_rollbacks.saturating_add(1);
        }

        Ok(RollbackOutcome { late, overdue_ms })
    }
}

/// 模块健康度: 健康 / 退化 / 倒下.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleHealth {
    /// 健康: 正常服务，不计入 at-risk。
    Healthy,
    /// 退化: 部分能力受损，按 criticality 计入风险清单。
    Degraded,
    /// 倒下: 不可用，按 criticality 计入风险清单。
    Down,
}

impl ModuleHealth {
    /// 稳定短名。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Down => "down",
        }
    }

    /// 是否健康。
    pub fn is_healthy(self) -> bool {
        matches!(self, Self::Healthy)
    }

    /// 风险权重: 健康 0.0 / 退化 0.5 / 倒下 1.0（供 [`KeystoneModule::risk_score`] 使用）。
    pub fn weight(self) -> f32 {
        match self {
            Self::Healthy => 0.0,
            Self::Degraded => 0.5,
            Self::Down => 1.0,
        }
    }
}

/// 关键种模块: 名字 + 关键度 + 健康度.
///
/// `criticality` 语义为 `0.0..=1.0`；[`KeystoneModule::new`] 与
/// [`KeystoneRegistry::register`] 会归一化它（`NaN` → `0.0`，`±∞`/越界夹到端点，`-0.0` 归一
/// 为 `0.0`），保证排序与序列化稳定、不 panic。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeystoneModule {
    /// 模块名（注册表内唯一；空串合法但不推荐）。
    pub name: String,
    /// 关键度（0.0..=1.0）: 该模块倒下对整体的影响权重。
    pub criticality: f32,
    /// 当前健康度。
    pub health: ModuleHealth,
}

impl KeystoneModule {
    /// 构造关键种模块（关键度自动归一化，见类型文档）。
    pub fn new(name: impl Into<String>, criticality: f32, health: ModuleHealth) -> Self {
        Self {
            name: name.into(),
            criticality: normalize_criticality(criticality),
            health,
        }
    }

    /// 风险分 = 归一化 criticality × 健康权重（只作接线层排序/告警参考；`at_risk` 的排序
    /// 规则固定为 criticality 降序，不使用该分数）。
    pub fn risk_score(&self) -> f32 {
        normalize_criticality(self.criticality) * self.health.weight()
    }
}

/// 关键度归一化: `NaN` → 0.0，`-0.0` → 0.0，越界/无穷夹到 `0.0..=1.0` 端点.
fn normalize_criticality(value: f32) -> f32 {
    if value.is_nan() {
        0.0
    } else if value <= 0.0 {
        0.0
    } else if value >= 1.0 {
        1.0
    } else {
        value
    }
}

/// 关键种注册表: 名字唯一、插入序存储、风险清单确定性排序.
///
/// 只登记与管理健康度，不做重启/隔离/告警；这些动作属于接线层。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct KeystoneRegistry {
    /// 按登记顺序保存的模块（名字唯一）。
    modules: Vec<KeystoneModule>,
}

impl KeystoneRegistry {
    /// 新建空注册表。
    pub fn new() -> Self {
        Self::default()
    }

    /// 登记模块: 返回 `true` 表示新增，`false` 表示同名覆盖（条数不变，位置不变）.
    ///
    /// 登记时再次归一化关键度，防止直接构造的结构体绕过 [`KeystoneModule::new`] 的归一化。
    pub fn register(&mut self, mut module: KeystoneModule) -> bool {
        module.criticality = normalize_criticality(module.criticality);
        if let Some(existing) = self.modules.iter_mut().find(|m| m.name == module.name) {
            *existing = module;
            false
        } else {
            self.modules.push(module);
            true
        }
    }

    /// 上报健康度: 已知模块返回 `true` 并更新；未知模块返回 `false` 且**不**隐式创建.
    pub fn report_health(&mut self, name: &str, health: ModuleHealth) -> bool {
        let Some(module) = self.modules.iter_mut().find(|m| m.name == name) else {
            return false;
        };
        module.health = health;
        true
    }

    /// 按名字查模块（登记序线性扫描；注册表规模小，确定性优先）。
    pub fn get(&self, name: &str) -> Option<&KeystoneModule> {
        self.modules.iter().find(|m| m.name == name)
    }

    /// 全部模块（登记序）。
    pub fn modules(&self) -> &[KeystoneModule] {
        &self.modules
    }

    /// 模块数。
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    /// 风险清单: 所有**非健康**模块，按「criticality 降序 → 名字升序」全序排序.
    ///
    /// 空注册表返回空 `Vec`。`criticality == 0.0` 的非健康模块也会入列（排最后），是否处置
    /// 由接线层按 `risk_score` 决定；健康模块永不入列。排序是全序且稳定（同名不可能重复），
    /// 同输入 → 同输出，不依赖登记顺序或 `HashMap`。
    pub fn at_risk(&self) -> Vec<&KeystoneModule> {
        let mut risky: Vec<&KeystoneModule> = self
            .modules
            .iter()
            .filter(|m| !m.health.is_healthy())
            .collect();
        risky.sort_by(|a, b| {
            b.criticality
                .total_cmp(&a.criticality)
                .then_with(|| a.name.cmp(&b.name))
        });
        risky
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reading(kind: CapacityKind, used: u64, soft: u64, hard: u64) -> CapacityReading {
        CapacityReading::new(kind, used, soft, hard)
    }

    fn verdict_of(used: u64, soft: u64, hard: u64) -> CapacityVerdict {
        let readings = [reading(CapacityKind::ActiveTasks, used, soft, hard)];
        *CapacityLedger::evaluate(&readings)
            .first()
            .expect("单条读数必有单条判定")
    }

    fn module(name: &str, criticality: f32, health: ModuleHealth) -> KeystoneModule {
        KeystoneModule::new(name, criticality, health)
    }

    fn names<'a>(modules: &[&'a KeystoneModule]) -> Vec<&'a str> {
        modules.iter().map(|m| m.name.as_str()).collect()
    }

    fn illegal(from: SuccessionPhase, action: SuccessionAction) -> SuccessionError {
        SuccessionError::IllegalTransition { from, action }
    }

    #[test]
    fn capacity_kinds_have_fixed_order_and_stable_names() {
        assert_eq!(CapacityKind::ALL.len(), 4);
        assert_eq!(
            names_of_kinds(),
            [
                "context_tokens",
                "memory_entries",
                "registered_tools",
                "active_tasks"
            ]
        );
        for pair in CapacityKind::ALL.windows(2) {
            assert!(pair[0] < pair[1], "声明顺序即固定展示顺序");
        }
    }

    fn names_of_kinds() -> Vec<&'static str> {
        CapacityKind::ALL.iter().map(|kind| kind.as_str()).collect()
    }

    #[test]
    fn verdict_severity_and_names_are_stable() {
        let all = [
            CapacityVerdict::Healthy,
            CapacityVerdict::NearCapacity,
            CapacityVerdict::AtSoftLimit,
            CapacityVerdict::OverHardLimit,
        ];
        let as_str: Vec<&str> = all.iter().map(|verdict| verdict.as_str()).collect();
        assert_eq!(
            as_str,
            [
                "healthy",
                "near_capacity",
                "at_soft_limit",
                "over_hard_limit"
            ]
        );
        for pair in all.windows(2) {
            assert!(pair[0] < pair[1], "声明顺序即严重度升序");
        }
        assert!(CapacityVerdict::OverHardLimit.rejects_new_load());
        assert!(!CapacityVerdict::AtSoftLimit.rejects_new_load());
        assert!(CapacityVerdict::Healthy.is_healthy());
        assert!(!CapacityVerdict::NearCapacity.is_healthy());
    }

    #[test]
    fn near_capacity_boundary_is_eighty_percent_inclusive() {
        let soft = 100;
        let hard = 200;
        assert_eq!(
            CapacityLedger::evaluate(&[reading(CapacityKind::ContextTokens, 79, soft, hard)]),
            vec![CapacityVerdict::Healthy],
            "79% 严格低于 80%"
        );
        assert_eq!(
            CapacityLedger::evaluate(&[reading(CapacityKind::ContextTokens, 80, soft, hard)]),
            vec![CapacityVerdict::NearCapacity],
            "80% 含等号"
        );
        assert_eq!(
            CapacityLedger::evaluate(&[reading(CapacityKind::ContextTokens, 99, soft, hard)]),
            vec![CapacityVerdict::NearCapacity],
            "未到软上限仍是接近"
        );
    }

    #[test]
    fn soft_and_hard_boundaries_are_inclusive_and_hard_wins() {
        assert_eq!(verdict_of(99, 100, 120), CapacityVerdict::NearCapacity);
        assert_eq!(verdict_of(100, 100, 120), CapacityVerdict::AtSoftLimit);
        assert_eq!(verdict_of(119, 100, 120), CapacityVerdict::AtSoftLimit);
        assert_eq!(verdict_of(120, 100, 120), CapacityVerdict::OverHardLimit);
        assert_eq!(verdict_of(121, 100, 120), CapacityVerdict::OverHardLimit);

        // 错配 soft > hard: 硬上限优先级最高，越过 hard 一律拒绝。
        assert_eq!(verdict_of(50, 100, 50), CapacityVerdict::OverHardLimit);
        assert_eq!(verdict_of(60, 100, 50), CapacityVerdict::OverHardLimit);
    }

    #[test]
    fn zero_limits_are_defined_without_division() {
        assert_eq!(
            verdict_of(0, 0, 100),
            CapacityVerdict::AtSoftLimit,
            "soft=0: used=0 即到软上限"
        );
        assert_eq!(
            verdict_of(0, 0, 0),
            CapacityVerdict::OverHardLimit,
            "hard=0: 零容量拒绝一切读数"
        );
        assert_eq!(
            verdict_of(0, 100, 0),
            CapacityVerdict::OverHardLimit,
            "hard=0 优先于 soft"
        );
        assert_eq!(
            verdict_of(u64::MAX, u64::MAX, u64::MAX),
            CapacityVerdict::OverHardLimit
        );
        assert_eq!(
            verdict_of(u64::MAX - 1, u64::MAX, u64::MAX),
            CapacityVerdict::NearCapacity,
            "u128 展开比较，u64::MAX 附近不溢出"
        );
    }

    #[test]
    fn evaluate_preserves_input_order_and_is_deterministic() {
        let readings = [
            reading(CapacityKind::ActiveTasks, 10, 100, 200),
            reading(CapacityKind::ContextTokens, 80, 100, 200),
            reading(CapacityKind::MemoryEntries, 150, 100, 200),
            reading(CapacityKind::RegisteredTools, 200, 100, 200),
        ];
        let first = CapacityLedger::evaluate(&readings);
        assert_eq!(
            first,
            vec![
                CapacityVerdict::Healthy,
                CapacityVerdict::NearCapacity,
                CapacityVerdict::AtSoftLimit,
                CapacityVerdict::OverHardLimit,
            ]
        );
        assert_eq!(
            CapacityLedger::evaluate(&readings),
            first,
            "同输入 → 同输出"
        );

        let reversed = [readings[3], readings[2], readings[1], readings[0]];
        assert_eq!(
            CapacityLedger::evaluate(&reversed),
            vec![
                CapacityVerdict::OverHardLimit,
                CapacityVerdict::AtSoftLimit,
                CapacityVerdict::NearCapacity,
                CapacityVerdict::Healthy,
            ],
            "输出与输入逐位对应，不按固定维度重排"
        );
    }

    #[test]
    fn empty_readings_and_degenerate_inputs_do_not_panic() {
        assert!(CapacityLedger::evaluate(&[]).is_empty());
        assert_eq!(CapacityLedger::new(), CapacityLedger);

        // kind 与 limit.kind 不一致时以 limit 数值判定，不报错、不 panic。
        let mismatched = CapacityReading {
            kind: CapacityKind::ActiveTasks,
            used: 50,
            limit: CapacityLimit::new(CapacityKind::MemoryEntries, 100, 200),
        };
        assert_eq!(
            CapacityLedger::evaluate(&[mismatched]),
            vec![CapacityVerdict::Healthy]
        );
    }

    #[test]
    fn succession_happy_path_reaches_stable_again() {
        let mut tracker = SuccessionTracker::new();
        assert_eq!(tracker.phase(), SuccessionPhase::Stable);
        assert_eq!(tracker.plan(), &SuccessionPlan::default());
        assert_eq!(tracker.late_rollbacks(), 0);
        assert!(!tracker.last_rollback_late());

        tracker
            .begin_migration(1_000, 0.25, 5_000)
            .expect("Stable 可开始迁移");
        assert_eq!(tracker.phase(), SuccessionPhase::Migrating);
        assert_eq!(tracker.plan().started_ms, 1_000);
        assert_eq!(tracker.plan().rollback_deadline_ms, 6_000);
        assert_eq!(tracker.plan().old_ratio_allowed, 0.25);

        tracker.advance().expect("Migrating 可推进");
        assert_eq!(tracker.phase(), SuccessionPhase::Verifying);
        assert_eq!(
            tracker.plan().old_ratio_allowed,
            0.25,
            "阶段推进不静默改写流量比例"
        );

        tracker.verify().expect("Verifying 可验证通过");
        assert_eq!(tracker.phase(), SuccessionPhase::Stable);
    }

    #[test]
    fn succession_rejects_illegal_transitions_without_mutating_state() {
        let mut tracker = SuccessionTracker::new();
        assert_eq!(
            tracker.advance(),
            Err(illegal(SuccessionPhase::Stable, SuccessionAction::Advance))
        );
        assert_eq!(
            tracker.verify(),
            Err(illegal(SuccessionPhase::Stable, SuccessionAction::Verify))
        );
        assert_eq!(
            tracker.rollback(0),
            Err(illegal(SuccessionPhase::Stable, SuccessionAction::Rollback))
        );
        assert_eq!(tracker.plan(), &SuccessionPlan::default(), "拒绝后状态不变");

        tracker.begin_migration(0, 1.0, 100).expect("首次迁移合法");
        assert_eq!(
            tracker.begin_migration(10, 1.0, 100),
            Err(illegal(
                SuccessionPhase::Migrating,
                SuccessionAction::BeginMigration
            )),
            "迁移中不得重入"
        );
        assert_eq!(
            tracker.verify(),
            Err(illegal(
                SuccessionPhase::Migrating,
                SuccessionAction::Verify
            )),
            "未推进不得验证"
        );
        assert_eq!(
            tracker.plan().rollback_deadline_ms,
            100,
            "非法 begin 不覆盖时间戳"
        );

        tracker.advance().expect("推进合法");
        assert_eq!(
            tracker.advance(),
            Err(illegal(
                SuccessionPhase::Verifying,
                SuccessionAction::Advance
            )),
            "验证中不得重复推进"
        );
        tracker.verify().expect("验证通过合法");
        assert_eq!(
            tracker.rollback(1),
            Err(illegal(SuccessionPhase::Stable, SuccessionAction::Rollback))
        );
    }

    #[test]
    fn succession_rejects_invalid_old_ratio_and_stays_stable() {
        for ratio in [f32::NAN, f32::INFINITY, -0.01, 1.01] {
            let mut tracker = SuccessionTracker::new();
            assert_eq!(
                tracker.begin_migration(0, ratio, 100),
                Err(SuccessionError::InvalidOldRatio),
                "ratio={ratio}"
            );
            assert_eq!(tracker.phase(), SuccessionPhase::Stable);
            assert_eq!(tracker.plan(), &SuccessionPlan::default());
        }
    }

    #[test]
    fn rollback_at_deadline_is_on_time() {
        let mut tracker = SuccessionTracker::new();
        tracker
            .begin_migration(1_000, 0.5, 5_000)
            .expect("迁移合法");
        assert!(!tracker.deadline_exceeded(6_000), "恰好在截止时刻不算超时");

        let outcome = tracker.rollback(6_000).expect("截止时刻仍可回滚");
        assert!(!outcome.late);
        assert_eq!(outcome.overdue_ms, 0);
        assert_eq!(tracker.phase(), SuccessionPhase::Stable);
        assert_eq!(tracker.late_rollbacks(), 0);
        assert!(!tracker.last_rollback_late());
        assert!(!tracker.deadline_exceeded(9_999), "Stable 阶段没有回滚窗口");
        assert_eq!(
            tracker.plan().rollback_deadline_ms,
            6_000,
            "回滚后保留时间戳作审计"
        );
    }

    #[test]
    fn rollback_after_deadline_is_marked_late() {
        let mut tracker = SuccessionTracker::new();
        tracker
            .begin_migration(1_000, 0.5, 5_000)
            .expect("迁移合法");
        assert!(tracker.deadline_exceeded(6_001));

        let outcome = tracker.rollback(6_250).expect("超时仍允许回滚，但必须标记");
        assert!(outcome.late);
        assert_eq!(outcome.overdue_ms, 250);
        assert_eq!(tracker.late_rollbacks(), 1);
        assert!(tracker.last_rollback_late());
        assert_eq!(tracker.phase(), SuccessionPhase::Stable);

        // 从 Verifying 回滚同样按截止时间判定（边界含等号在「按时」一侧）。
        tracker
            .begin_migration(10_000, 0.0, 1_000)
            .expect("第二次迁移合法");
        tracker.advance().expect("推进合法");
        let outcome = tracker.rollback(11_000).expect("截止时刻回滚合法");
        assert!(!outcome.late, "边界含等号");
        assert_eq!(tracker.late_rollbacks(), 1, "按时回滚不增加超时计数");
    }

    #[test]
    fn rollback_deadline_saturates_and_zero_window_is_defined() {
        let mut tracker = SuccessionTracker::new();
        tracker
            .begin_migration(500, 1.0, 0)
            .expect("零窗口迁移合法");
        assert!(
            !tracker.deadline_exceeded(500),
            "零窗口的截止时刻即开始时刻"
        );
        assert!(tracker.deadline_exceeded(501));
        tracker.rollback(500).expect("按点回滚");
        assert!(!tracker.last_rollback_late());

        let mut saturated = SuccessionTracker::new();
        saturated
            .begin_migration(u64::MAX - 10, 1.0, 100)
            .expect("饱和迁移合法");
        assert_eq!(
            saturated.plan().rollback_deadline_ms,
            u64::MAX,
            "饱和加法不回绕"
        );
        assert!(!saturated.deadline_exceeded(u64::MAX));
        let outcome = saturated.rollback(u64::MAX).expect("最大时刻回滚");
        assert!(!outcome.late);
    }

    #[test]
    fn at_risk_sorts_by_criticality_desc_then_name() {
        let mut registry = KeystoneRegistry::new();
        registry.register(module("zeta", 0.9, ModuleHealth::Down));
        registry.register(module("beta", 0.95, ModuleHealth::Degraded));
        registry.register(module("alpha", 0.9, ModuleHealth::Down));
        registry.register(module("healthy", 1.0, ModuleHealth::Healthy));
        registry.register(module("low", 0.1, ModuleHealth::Down));

        let ordered = names(&registry.at_risk());
        assert_eq!(ordered, ["beta", "alpha", "zeta", "low"]);
        assert!(
            !registry.at_risk().iter().any(|m| m.name == "healthy"),
            "健康模块永不入列"
        );
    }

    #[test]
    fn at_risk_is_deterministic_across_registration_order() {
        let mut forward = KeystoneRegistry::new();
        for (name, criticality) in [("core", 0.8_f32), ("memory", 0.6), ("tools", 0.9)] {
            forward.register(module(name, criticality, ModuleHealth::Down));
        }

        let mut reverse = KeystoneRegistry::new();
        for (name, criticality) in [("tools", 0.9_f32), ("memory", 0.6), ("core", 0.8)] {
            reverse.register(module(name, criticality, ModuleHealth::Down));
        }

        assert_eq!(names(&forward.at_risk()), names(&reverse.at_risk()));
        assert_eq!(
            names(&forward.at_risk()),
            names(&forward.at_risk()),
            "重复调用稳定"
        );
    }

    #[test]
    fn register_dedupes_by_name_and_report_health_updates() {
        let mut registry = KeystoneRegistry::new();
        assert!(registry.is_empty());
        assert!(registry.register(module("core", 0.5, ModuleHealth::Healthy)));

        assert!(
            !registry.register(module("core", 0.75, ModuleHealth::Degraded)),
            "同名重登记 = 覆盖"
        );
        assert_eq!(registry.len(), 1);
        let stored = registry.get("core").expect("已登记");
        assert_eq!(stored.criticality, 0.75);
        assert_eq!(stored.health, ModuleHealth::Degraded);

        assert!(registry.report_health("core", ModuleHealth::Down));
        assert_eq!(
            registry.get("core").map(|m| m.health),
            Some(ModuleHealth::Down)
        );
        assert!(
            !registry.report_health("unknown", ModuleHealth::Down),
            "未知模块不隐式创建"
        );
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn criticality_is_normalized_and_risk_score_weights_health() {
        let mut registry = KeystoneRegistry::new();
        registry.register(KeystoneModule {
            name: "nan".to_string(),
            criticality: f32::NAN,
            health: ModuleHealth::Down,
        });
        registry.register(KeystoneModule {
            name: "neg".to_string(),
            criticality: -3.0,
            health: ModuleHealth::Degraded,
        });
        registry.register(KeystoneModule {
            name: "high".to_string(),
            criticality: 2.0,
            health: ModuleHealth::Healthy,
        });
        registry.register(KeystoneModule {
            name: "inf".to_string(),
            criticality: f32::INFINITY,
            health: ModuleHealth::Degraded,
        });

        assert_eq!(registry.get("nan").map(|m| m.criticality), Some(0.0));
        assert_eq!(registry.get("neg").map(|m| m.criticality), Some(0.0));
        assert_eq!(registry.get("high").map(|m| m.criticality), Some(1.0));
        assert_eq!(registry.get("inf").map(|m| m.criticality), Some(1.0));

        assert_eq!(module("m", 0.5, ModuleHealth::Down).risk_score(), 0.5);
        assert_eq!(module("m", 0.5, ModuleHealth::Degraded).risk_score(), 0.25);
        assert_eq!(module("m", 1.0, ModuleHealth::Healthy).risk_score(), 0.0);
    }

    #[test]
    fn empty_registry_and_unknown_lookups_are_defined() {
        let registry = KeystoneRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(registry.at_risk().is_empty());
        assert!(registry.get("missing").is_none());
        assert!(registry.modules().is_empty());
        assert!(SuccessionTracker::new().phase().is_stable());
    }

    #[test]
    fn public_snapshots_roundtrip_through_serde_json() {
        let value = reading(CapacityKind::MemoryEntries, 80, 100, 200);
        let json = serde_json::to_string(&value).expect("序列化");
        assert_eq!(
            serde_json::from_str::<CapacityReading>(&json).expect("反序列化"),
            value
        );

        let plan = SuccessionPlan {
            phase: SuccessionPhase::Migrating,
            old_ratio_allowed: 0.25,
            started_ms: 10,
            rollback_deadline_ms: 20,
        };
        let json = serde_json::to_string(&plan).expect("序列化");
        assert_eq!(
            serde_json::from_str::<SuccessionPlan>(&json).expect("反序列化"),
            plan
        );

        let mut tracker = SuccessionTracker::new();
        tracker
            .begin_migration(1_000, 0.5, 5_000)
            .expect("迁移合法");
        let json = serde_json::to_string(&tracker).expect("序列化");
        assert_eq!(
            serde_json::from_str::<SuccessionTracker>(&json).expect("反序列化"),
            tracker
        );

        let mut registry = KeystoneRegistry::new();
        registry.register(module("memory", 0.75, ModuleHealth::Degraded));
        let json = serde_json::to_string(&registry).expect("序列化");
        assert_eq!(
            serde_json::from_str::<KeystoneRegistry>(&json).expect("反序列化"),
            registry
        );

        assert_eq!(
            serde_json::from_str::<CapacityVerdict>("\"over_hard_limit\"").expect("反序列化"),
            CapacityVerdict::OverHardLimit
        );
    }
}
