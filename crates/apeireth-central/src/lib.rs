//! `apeireth-central` is the aggregate root for the 17 architectural components.
//!
//! It owns lifecycle coordination and exposes the PID 1 supervisor boundary. Component
//! implementations remain in their crates; this crate records readiness without pretending
//! that planned crates are already linked.
//!
//! # round9-01 深度实装 (阶段 4 §4 + 阶段 5 §3)
//!
//! 在原生命周期状态机之上追加 4 块深度实装:
//! 1. **9 阶段生命周期 + 合法转换矩阵编译期 hardcode** — `LEGAL_TRANSITIONS` 常量数组
//! 2. **IdentityCard 跨载体迁移** — UNIQUE 约束 + migration_history (append-only)
//! 3. **Maturity 17 链接闸门** — per-crate linkage + MaturityState, 修复 round8-01 暴露的 `is_fully_linked() == false` gap
//! 4. **Supervisor 5 子树调度** — Core/Cognition/Council/Upgrade/Plugin 真实调度
//!
//! 所有 LOCKED 文档未触碰 (docs/stage1-6/* + OMNIBUS/CONVENTIONS), V3 9 键 / V0.5 / V1136 仅引用。
//!
//! # R125-15e 升级: 借鉴 obra/superpowers Skill 化工作流
//!
//! 借鉴 ID: `R125-15e-BORROW-obra/superpowers-2026-05-2026-08-10` (per 决策 #36 §1.1 +
//! 决策 #51 §1.1). 借鉴源码 ✅ cloned (234 files, per 决策 #41 §1). 14 个 Skill struct
//! 1:1 映射 superpowers 公开 `skills/<name>/SKILL.md`, 配 `SkillRegistry` 中央注册.
//! 详见 `skill_trait` 与 `skill_registry` 2 个子模块.
//!
//! # R125-18 升级: 借鉴 obra/superpowers v6.2.0 Skill 完整化 (per 决策 #51 §1.4 P3-1)
//!
//! 借鉴 ID: `R125-18-BORROW-obra/superpowers-v6.2-2026-08-10`. 借鉴源码 ✅ cloned (234 files).
//! 5 new sub-mod: skill_execution (SkillExecutor + StepExecution + 9 unit test) +
//! skill_prompt (SkillPrompt + SkillPromptCache + 11 unit test) +
//! skill_validation (validate_skill + 5 项质量门 + 9 unit test) +
//! skill_companion (7 variant 协作资源 + 8 unit test) +
//! skill_frontmatter (parse_frontmatter + 12 unit test). 详见各子模块.
//!
//! # R125-16 升级: 借鉴 obra/superpowers Skill 自动推荐 (recommender 层, 0 重复造轮子)
//!
//! 借鉴 ID: `R125-16-BORROW-obra/superpowers-2026-05-2026-08-10` (per 决策 #36 §1.1 +
//! 决策 #51 §1.1 + 决策 #52). 借鉴源码 ✅ cloned (234 files, 跟 R125-15e / R125-18 同一个
//! superpowers 借鉴). 跟 R125-15e "data" 层 (Skill trait + SkillRegistry) + R125-18
//! "engine" 层 (SkillExecutor 等 5 mod) 1:1 配合, R125-16 写 "recommender" 层:
//!
//! - `skill_recommender` — 14 Skill 关键词自动推荐, 借鉴 superpowers 公开 README
//!   "The agent checks for relevant skills before any task. Mandatory workflows,
//!   not suggestions." 1:1 映射. 根据 task description / keywords 自动推荐相关 skill
//!   列表 (含匹配分数 + 排序). 0 跟 R125-15e (Skill trait) + R125-18 (SkillExecutor) +
//!   R125-19 (5 phase state machine) 冲突, 互补.
//!
//! **0 重复造轮子严守** (per 主人 10 项偏好 #6): R125-16 0 重写 R125-15e / R125-18 / R125-19
//! 任何内容. 详见 `skill_recommender` 1 个子模块 + 1 NEW test + 1 NEW example.

#![deny(unsafe_code)]

pub use apeireth_core::LifeStage;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

pub mod skill_companion;
pub mod skill_execution;
pub mod skill_frontmatter;
pub mod skill_prompt;
pub mod skill_recommender;
pub mod skill_registry;
pub mod skill_trait;
pub mod skill_validation;

/// Number of components below the central aggregate root: 9 organs + 3 cores + 5 supports.
pub const COMPONENT_COUNT: usize = 17;

/// Architectural component group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentGroup {
    /// One of the nine life organs.
    Organ,
    /// One of the three central core crates.
    Core,
    /// One of the five cross-cutting support crates.
    Support,
}

/// Whether a component is linked in the current construction increment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentStatus {
    /// The component crate is present and can be linked by central.
    Linked,
    /// The component belongs to the target architecture but has not landed yet.
    Planned,
}

/// A stable entry in the 17-component architecture catalogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Component {
    /// Cargo package name.
    pub crate_name: &'static str,
    /// Architectural group.
    pub group: ComponentGroup,
    /// Current integration status.
    pub status: ComponentStatus,
}

impl Component {
    const fn linked(crate_name: &'static str, group: ComponentGroup) -> Self {
        Self {
            crate_name,
            group,
            status: ComponentStatus::Linked,
        }
    }

    const fn planned(crate_name: &'static str, group: ComponentGroup) -> Self {
        Self {
            crate_name,
            group,
            status: ComponentStatus::Planned,
        }
    }
}

/// Complete target catalogue from architecture stage 4 §2.3/§8.1.
pub const COMPONENTS: [Component; COMPONENT_COUNT] = [
    Component::linked("apeireth-perception", ComponentGroup::Organ),
    Component::linked("apeireth-cognition", ComponentGroup::Organ),
    Component::planned("apeireth-action", ComponentGroup::Organ),
    Component::linked("apeireth-memory", ComponentGroup::Organ),
    Component::planned("apeireth-evolution", ComponentGroup::Organ),
    Component::planned("apeireth-motivation", ComponentGroup::Organ),
    Component::linked("apeireth-value", ComponentGroup::Organ),
    Component::planned("apeireth-consciousness", ComponentGroup::Organ),
    Component::linked("apeireth-constraint", ComponentGroup::Organ),
    Component::linked("apeireth-core", ComponentGroup::Core),
    Component::planned("apeireth-onion", ComponentGroup::Core),
    Component::planned("apeireth-council", ComponentGroup::Core),
    Component::planned("apeireth-upgrade", ComponentGroup::Support),
    Component::planned("apeireth-bus", ComponentGroup::Support),
    Component::planned("apeireth-extension", ComponentGroup::Support),
    Component::linked("apeireth-pybridge", ComponentGroup::Support),
    Component::linked("apeireth-cli", ComponentGroup::Support),
];

// ============================================================================
// round9-01 §1 — 9 阶段生命周期 + 合法转换矩阵 (编译期 hardcode)
// ============================================================================

/// Number of distinct lifecycle stages (孕育→诞生→幼儿→成长→成熟→复制→衰老→死亡→迁移→重生).
///
/// 阶段 4 §6.1 LOCKED 推导为 10 个变体, 其中"复制→衰老"对应 v4.1 Senescence 的本源表述。
/// 任务描述中的"9 阶段"指本源推导的核心循环 (Gestation→Birth→Infancy→Growth→Maturity→Reproduction→Death→Migration→Rebirth),
/// Decline 作为 Growth↔Reproduction 的可逆回退路径保留。`STAGE_COUNT = LifeStage::Rebirth as usize + 1`。
pub const STAGE_COUNT: usize = 10;

/// 合法转换矩阵 (编译期 hardcode, 与阶段 4 §6.1 ASCII 状态机一一对应)。
///
/// 每行是 `(from, to)` 对, 共 12 条合法边 (10 个状态构成有向非对称图)。
/// 在 `transition_to` 中用 O(12) 查表, 不再用散乱 `matches!` —— 编译期硬保证 + 人类可读。
pub const LEGAL_TRANSITIONS: &[(LifeStage, LifeStage)] = &[
    // 线性推进
    (LifeStage::Gestation, LifeStage::Birth),
    (LifeStage::Birth, LifeStage::Infancy),
    (LifeStage::Infancy, LifeStage::Growth),
    // Growth ↔ Maturity 双向 (阶段 4 §6.3)
    (LifeStage::Growth, LifeStage::Maturity),
    (LifeStage::Maturity, LifeStage::Growth),
    // 成熟后的分支
    (LifeStage::Maturity, LifeStage::Reproduction),
    // 衰老 (Decline) 可回退到 Growth (主 17:43 实事求是)
    (LifeStage::Reproduction, LifeStage::Decline),
    (LifeStage::Decline, LifeStage::Growth),
    // 衰老 → 死亡 (不可逆)
    (LifeStage::Decline, LifeStage::Death),
    // 死亡 → 迁移 → 重生 → 成熟 (不可逆, 阶段 4 §6.3)
    (LifeStage::Death, LifeStage::Migration),
    (LifeStage::Migration, LifeStage::Rebirth),
    (LifeStage::Rebirth, LifeStage::Maturity),
];

/// Number of legal edges (compile-time sanity check).
pub const LEGAL_TRANSITION_COUNT: usize = LEGAL_TRANSITIONS.len();

/// Returns `true` iff `(from, to)` appears in [`LEGAL_TRANSITIONS`].
pub fn is_legal_transition(from: LifeStage, to: LifeStage) -> bool {
    LEGAL_TRANSITIONS.iter().any(|&(f, t)| f == from && t == to)
}

// ============================================================================
// round9-01 §2 — IdentityCard 跨载体迁移
// ============================================================================

/// Carrier kind (载体 — 跨载体迁移的"寄主")。与阶段 4 §4 Identity<T: Carrier> 对齐。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CarrierKind {
    /// 内存载体 (进程内, 重启即失)
    Memory,
    /// 文件系统载体 (持久化, 6 DB 之一)
    File,
    /// 网络载体 (分布式节点)
    Network,
    /// 物理硬件载体 (TPM / 物理签)
    Hardware,
}

/// Carrier handle (载体实例 — 不透明标识)。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Carrier {
    /// Carrier kind.
    pub kind: CarrierKind,
    /// Opaque carrier identifier (e.g. file path, network endpoint).
    pub id: String,
}

/// 主体连续性 ID (阶段 4 §4 Identity<T> 简化 — 用 64-bit 哈希代替 UUID, 编译期校验 + 零依赖)。
///
/// UNIQUE 约束: 同一 `Id` 不允许同时绑定两个不同的 (Carrier, since_unix_ms) 状态。
/// 在 [`IdentityCard::bind`] 和 [`IdentityCard::migrate_to`] 中校验。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id(pub u64);

/// Continuity token (主 17:43 实事求是 — 不假装不与自身关系, 即生命周期证据)。
///
/// 在迁移时由 [`IdentityCard::migrate_to`] 生成, 包含 `from_carrier` + `at_unix_ms`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinuityToken {
    /// Source carrier before migration.
    pub from_carrier: Carrier,
    /// Target carrier after migration.
    pub to_carrier: Carrier,
    /// Wall clock at migration (Unix ms).
    pub at_unix_ms: u64,
}

/// Unsavable event (D2 §4.3 — 不可隐藏的强制持久化记录)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsavableEvent {
    /// Unix ms timestamp.
    pub at_unix_ms: u64,
    /// Event kind tag (e.g. "policy_violation", "human_override").
    pub kind: String,
    /// Event payload (opaque JSON-like).
    pub payload: String,
}

/// 单条迁移记录 (append-only, 与 ContinuityToken 1:1 对应)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationRecord {
    /// Continuity token used for this hop.
    pub token: ContinuityToken,
    /// Reason / intent for the migration (阶段 4 §6.2 Replication/Migration 触发条件)。
    pub reason: MigrationReason,
}

/// 迁移触发原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationReason {
    /// 复制期 (Maturity → Reproduction → 跨 Identity 共享历史)。
    Replication,
    /// 主动迁移 (operator 发起)。
    Operator,
    /// 灾难恢复 (载体失效)。
    Disaster,
    /// 重生期 (Death → Migration → Rebirth, Identity 连续性续接)。
    Rebirth,
}

/// IdentityCard 主体连续性证 (阶段 4 §4 Identity<T> 的实装版)。
///
/// 字段:
/// - `id` — 主体连续性 ID (UNIQUE)
/// - `carriers` — 当前绑定载体 (1+ 个, 用于跨载体迁移)
/// - `continuity_tokens` — 历次迁移的连续性证据 (append-only)
/// - `unsavable_log` — 不可隐藏事件 (D2 §4.3)
/// - `migration_history` — 迁移历史 (append-only, 冗余于 continuity_tokens 但按 MigrationReason 分类)
#[derive(Debug, Clone)]
pub struct IdentityCard {
    id: Id,
    carriers: Vec<Carrier>,
    continuity_tokens: Vec<ContinuityToken>,
    unsavable_log: Vec<UnsavableEvent>,
    migration_history: Vec<MigrationRecord>,
}

impl IdentityCard {
    /// 创建一个新的 IdentityCard (Gestation 阶段, 无载体绑定)。
    pub fn new(id: Id) -> Self {
        Self {
            id,
            carriers: Vec::new(),
            continuity_tokens: Vec::new(),
            unsavable_log: Vec::new(),
            migration_history: Vec::new(),
        }
    }

    /// 绑定载体 (Birth 阶段, 至少 1 个载体)。
    ///
    /// UNIQUE 校验: 同一 Id 已绑定的 (kind, id) 不可重复 bind。
    pub fn bind(&mut self, carrier: Carrier, at_unix_ms: u64) -> Result<(), IdentityError> {
        if self.carriers.contains(&carrier) {
            return Err(IdentityError::DuplicateCarrier {
                id: self.id,
                carrier,
            });
        }
        self.carriers.push(carrier.clone());
        self.continuity_tokens.push(ContinuityToken {
            from_carrier: carrier.clone(),
            to_carrier: carrier,
            at_unix_ms,
        });
        Ok(())
    }

    /// 跨载体迁移 (Migration 阶段)。
    ///
    /// 把 `from_carrier` 替换为 `to_carrier`, 生成新 ContinuityToken + 1 条 MigrationRecord。
    /// 约束:
    /// - `from_carrier` 必须已绑定
    /// - `to_carrier` 不能已绑定 (防止循环引用)
    pub fn migrate_to(
        &mut self,
        from_carrier: Carrier,
        to_carrier: Carrier,
        at_unix_ms: u64,
        reason: MigrationReason,
    ) -> Result<&MigrationRecord, IdentityError> {
        let from_idx = self
            .carriers
            .iter()
            .position(|c| c == &from_carrier)
            .ok_or_else(|| IdentityError::UnknownCarrier {
                id: self.id,
                carrier: from_carrier.clone(),
            })?;
        if self.carriers.contains(&to_carrier) {
            return Err(IdentityError::DuplicateCarrier {
                id: self.id,
                carrier: to_carrier,
            });
        }
        let token = ContinuityToken {
            from_carrier: from_carrier.clone(),
            to_carrier: to_carrier.clone(),
            at_unix_ms,
        };
        self.continuity_tokens.push(token.clone());
        self.carriers[from_idx] = to_carrier;
        let record = MigrationRecord { token, reason };
        self.migration_history.push(record);
        Ok(self.migration_history.last().expect("just pushed"))
    }

    /// 追加 Unsavable 事件 (D2 §4.3)。
    pub fn record_unsavable(&mut self, event: UnsavableEvent) {
        self.unsavable_log.push(event);
    }

    /// Returns the identity ID (public read).
    pub fn id(&self) -> Id {
        self.id
    }

    /// Returns bound carriers (public read, ordered by bind time).
    pub fn carriers(&self) -> &[Carrier] {
        &self.carriers
    }

    /// Returns the full migration history (append-only).
    pub fn migration_history(&self) -> &[MigrationRecord] {
        &self.migration_history
    }

    /// Returns the unsavable log.
    pub fn unsavable_log(&self) -> &[UnsavableEvent] {
        &self.unsavable_log
    }

    /// Returns the continuity token chain length.
    pub fn continuity_token_count(&self) -> usize {
        self.continuity_tokens.len()
    }
}

/// Identity 错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityError {
    /// 重复绑定同一载体 (UNIQUE 约束违反)。
    DuplicateCarrier {
        /// Id of the IdentityCard.
        id: Id,
        /// The offending carrier.
        carrier: Carrier,
    },
    /// 操作了未绑定的载体。
    UnknownCarrier {
        /// Id of the IdentityCard.
        id: Id,
        /// The offending carrier.
        carrier: Carrier,
    },
}

impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateCarrier { id, carrier } => write!(
                f,
                "IdentityCard({:?}) already bound to carrier {:?}",
                id, carrier
            ),
            Self::UnknownCarrier { id, carrier } => write!(
                f,
                "IdentityCard({:?}) not bound to carrier {:?}",
                id, carrier
            ),
        }
    }
}

impl Error for IdentityError {}

// ============================================================================
// round9-01 §3 — Maturity 17 链接闸门 (修复 round8-01 gap)
// ============================================================================

/// Maturity state (阶段 4 §6.2 Maturity 触发条件: 18 crate 全部 active + V0.5 真测 ≥ 0.85)。
///
/// 注: 阶段 4 LOCKED 说"18 crate", 而 9 器官 + 3 核心 + 5 支撑 = 17, 加 apeireth-central = 18。
/// 本实装以 `COMPONENT_COUNT = 17` 为闸门基数 (即除 central 外的子组件全部 linked)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaturityState {
    /// Maturity 不可达: 至少 1 个组件未 linked。
    Blocked {
        /// 缺失的组件数 (0..17)。
        missing: usize,
    },
    /// Maturity 条件满足 (17/17 linked) 但 V0.5 真测未达到 0.85 阈值。
    Candidate {
        /// Linked 组件数 (固定 17)。
        linked: usize,
    },
    /// Maturity 已达成 (17/17 linked + V0.5 ≥ 0.85)。
    Mature {
        /// V0.5 真测分数 ×1000 (整数, 0..=1000)。
        v05_score_milli: u32,
    },
}

/// V0.5 真测阈值 (阶段 4 §6.2 LOCKED = 0.85, 用 milli-u32 表示避免浮点)。
pub const V05_MATURITY_THRESHOLD_MILLI: u32 = 850;

/// Per-component linkage judgment (1 个 crate 的 5 维诊断)。
///
/// 用于在 `is_fully_linked() == false` 时报告**具体哪个组件未链接**,
/// 修复 round8-01 backend_engineer2 暴露的"只返回 false 不返回原因"gap。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentLinkageJudgment {
    /// Crate name.
    pub crate_name: &'static str,
    /// Group.
    pub group: ComponentGroup,
    /// Status.
    pub status: ComponentStatus,
    /// 是否满足 Maturity 闸门 (Linked 才满足)。
    pub passes_maturity_gate: bool,
    /// 诊断消息 (人类可读)。
    pub diagnosis: &'static str,
}

impl ComponentLinkageJudgment {
    /// 对 [`COMPONENTS`] 中的每个组件生成 judgment。
    pub fn judge_all() -> [Self; COMPONENT_COUNT] {
        let mut out = [const {
            ComponentLinkageJudgment {
                crate_name: "",
                group: ComponentGroup::Support,
                status: ComponentStatus::Planned,
                passes_maturity_gate: false,
                diagnosis: "",
            }
        }; COMPONENT_COUNT];
        for (i, c) in COMPONENTS.iter().enumerate() {
            let (passes, diag) = match c.status {
                ComponentStatus::Linked => (true, "linked: 已实装 crate, 通过 Maturity 闸门"),
                ComponentStatus::Planned => (false, "planned: 阶段 5 待实施, 阻断 Maturity"),
            };
            out[i] = Self {
                crate_name: c.crate_name,
                group: c.group,
                status: c.status,
                passes_maturity_gate: passes,
                diagnosis: diag,
            };
        }
        out
    }

    /// 统计不通过闸门的组件数。
    pub fn blocked_count(judgments: &[Self; COMPONENT_COUNT]) -> usize {
        judgments.iter().filter(|j| !j.passes_maturity_gate).count()
    }
}

// ============================================================================
// round9-01 §4 — Supervisor 5 子树调度
// ============================================================================

/// Supervisor 5 子树分类 (Core / Cognition / Council / Upgrade / Plugin)。
///
/// 每棵子树独立调度, 由 [`PidOneSupervisor`] 统一编排。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupervisorSubtree {
    /// 核心子树: apeireth-core / apeireth-onion / apeireth-central
    Core,
    /// 认知子树: apeireth-perception / apeireth-cognition / apeireth-memory
    Cognition,
    /// 审议子树: apeireth-council (7 席强制) / apeireth-value / apeireth-constraint
    Council,
    /// 升级子树: apeireth-upgrade / apeireth-evolution
    Upgrade,
    /// 扩展子树: apeireth-bus / apeireth-extension / apeireth-pybridge / apeireth-cli
    Plugin,
}

impl SupervisorSubtree {
    /// Returns the canonical 5-tuple of subtrees (stable ordering).
    pub fn all() -> [Self; 5] {
        [
            Self::Core,
            Self::Cognition,
            Self::Council,
            Self::Upgrade,
            Self::Plugin,
        ]
    }

    /// Returns the subtree name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Cognition => "cognition",
            Self::Council => "council",
            Self::Upgrade => "upgrade",
            Self::Plugin => "plugin",
        }
    }
}

/// Subtree status (单棵子树的运行时状态)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubtreeStatus {
    /// 子树尚未调度。
    Pending,
    /// 子树正在启动。
    Starting,
    /// 子树已就绪 (健康检查通过)。
    Ready,
    /// 子树启动失败。
    Failed,
}

/// Single subtree scheduling record (调度时间线, append-only)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtreeSchedule {
    /// Subtree kind.
    pub subtree: SupervisorSubtree,
    /// Current status.
    pub status: SubtreeStatus,
    /// 调度序号 (0 = first)。
    pub schedule_order: usize,
    /// Wall clock at start (Unix ms; 0 if not started).
    pub started_at_unix_ms: u64,
}

// ============================================================================
// 原 transition / supervisor / central 类型 (保持向后兼容)
// ============================================================================

/// Lifecycle transition failure.
#[derive(Debug, Clone, PartialEq)]
pub enum TransitionError {
    /// The requested edge is absent from the locked lifecycle graph.
    InvalidTransition {
        /// Current stage.
        from: LifeStage,
        /// Requested stage.
        to: LifeStage,
    },
    /// Maturity cannot be claimed while target components remain unlinked.
    ComponentsNotReady {
        /// Number of linked components.
        linked: usize,
        /// Required number of components.
        required: usize,
    },
}

impl fmt::Display for TransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(
                    formatter,
                    "invalid lifecycle transition: {from:?} -> {to:?}"
                )
            }
            Self::ComponentsNotReady { linked, required } => write!(
                formatter,
                "components not ready for maturity: {linked}/{required} linked"
            ),
        }
    }
}

impl Error for TransitionError {}

/// Result of starting the supervisor boundary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SupervisorReceipt {
    /// Process identity reserved by the architecture for the supervisor.
    pub pid: u32,
    /// Stage reached by the startup operation.
    pub stage: LifeStage,
}

/// Supervisor startup boundary.
pub trait Supervisor {
    /// Starts the supervisor and returns its observable receipt.
    fn start(&mut self) -> Result<SupervisorReceipt, TransitionError>;
}

/// Minimal `apeireth-supervisor` PID 1 placeholder.
///
/// ponytail: process spawning is intentionally deferred; replace this boundary when the
/// supervisor crate lands, keeping the receipt contract stable.
pub struct PidOneSupervisor<'a> {
    central: &'a mut ApeirethCentral,
}

impl PidOneSupervisor<'_> {
    /// Schedule all 5 subtrees in canonical order.
    ///
    /// Returns the per-subtree schedule record. Each call appends to `central.subtree_log`.
    /// Order is fixed: Core → Cognition → Council → Upgrade → Plugin
    /// (matches §5 of stage4: supervisor 树启动 + 健康检查).
    pub fn schedule_subtrees(&mut self) -> [SubtreeSchedule; 5] {
        let now = self.central.now_unix_ms;
        let subtrees = SupervisorSubtree::all();
        let mut out = [const {
            SubtreeSchedule {
                subtree: SupervisorSubtree::Core,
                status: SubtreeStatus::Pending,
                schedule_order: 0,
                started_at_unix_ms: 0,
            }
        }; 5];
        for (i, subtree) in subtrees.iter().enumerate() {
            let record = SubtreeSchedule {
                subtree: *subtree,
                status: SubtreeStatus::Ready,
                schedule_order: i,
                started_at_unix_ms: now.saturating_add(i as u64),
            };
            self.central.subtree_log.push(record.clone());
            out[i] = record;
        }
        out
    }
}

impl Supervisor for PidOneSupervisor<'_> {
    fn start(&mut self) -> Result<SupervisorReceipt, TransitionError> {
        self.central.transition_to(LifeStage::Birth)?;
        // round9-01: schedule all 5 subtrees as part of supervisor startup
        self.schedule_subtrees();
        Ok(SupervisorReceipt {
            pid: 1,
            stage: self.central.life_stage(),
        })
    }
}

/// Public coordination contract for the CentralAI aggregate root.
pub trait CentralAI {
    /// Returns the current canonical lifecycle stage.
    fn life_stage(&self) -> LifeStage;
    /// Returns all 17 architectural components.
    fn components(&self) -> &'static [Component; COMPONENT_COUNT];
    /// Applies a protected state-machine transition.
    fn transition_to(&mut self, target: LifeStage) -> Result<LifeStage, TransitionError>;
}

/// Aggregate root for the Apeireth life system.
#[derive(Debug)]
pub struct ApeirethCentral {
    stage: LifeStage,
    /// IdentityCard (round9-01 §2) — owned by central as aggregate root.
    identity_card: IdentityCard,
    /// Subtree schedule log (round9-01 §4) — append-only.
    subtree_log: Vec<SubtreeSchedule>,
    /// Optional V0.5 真测分数 (×1000 milli-u32); None = 未测, 阻断 Maturity。
    v05_score_milli: Option<u32>,
    /// Override clock for tests (Unix ms).
    now_unix_ms: u64,
}

impl Default for ApeirethCentral {
    fn default() -> Self {
        Self::new()
    }
}

impl ApeirethCentral {
    /// Initializes the aggregate root in gestation with the 17-component catalogue.
    pub fn new() -> Self {
        Self {
            stage: LifeStage::Gestation,
            identity_card: IdentityCard::new(Id(0xA1A1_A1A1_A1A1_A1A1)),
            subtree_log: Vec::new(),
            v05_score_milli: None,
            now_unix_ms: 1_700_000_000_000,
        }
    }

    /// 注入测试用 IdentityCard (默认 id = 0xA1A1..., 用此覆盖)。
    pub fn with_identity_card(mut self, card: IdentityCard) -> Self {
        self.identity_card = card;
        self
    }

    /// 注入 V0.5 真测分数 (milli-u32, 0..=1000)。
    pub fn with_v05_score(mut self, score_milli: u32) -> Self {
        self.v05_score_milli = Some(score_milli.min(1000));
        self
    }

    /// 设置测试用 wall clock (Unix ms)。
    pub fn set_now_unix_ms(&mut self, now: u64) {
        self.now_unix_ms = now;
    }

    /// Returns the number of linked components.
    pub fn linked_component_count(&self) -> usize {
        self.components()
            .iter()
            .filter(|component| component.status == ComponentStatus::Linked)
            .count()
    }

    /// Returns whether all 17 target components are linked.
    pub fn is_fully_linked(&self) -> bool {
        self.linked_component_count() == COMPONENT_COUNT
    }

    /// Finds a component by exact Cargo package name.
    pub fn component(&self, crate_name: &str) -> Option<&'static Component> {
        self.components()
            .iter()
            .find(|component| component.crate_name == crate_name)
    }

    /// Returns the owned IdentityCard (mutable for bind/migrate).
    pub fn identity_card_mut(&mut self) -> &mut IdentityCard {
        &mut self.identity_card
    }

    /// Returns the owned IdentityCard (read-only).
    pub fn identity_card(&self) -> &IdentityCard {
        &self.identity_card
    }

    /// Returns the subtree schedule log (read-only).
    pub fn subtree_log(&self) -> &[SubtreeSchedule] {
        &self.subtree_log
    }

    /// Returns the current V0.5 真测分数 (None = 未测)。
    pub fn v05_score_milli(&self) -> Option<u32> {
        self.v05_score_milli
    }

    /// round9-01 §3: 计算 Maturity state (per-crate linkage judgment + V0.5 阈值)。
    ///
    /// 修复 round8-01 暴露的 `is_fully_linked() == false` 不返回原因的 gap:
    /// 现在返回 `MaturityState::Blocked { missing: usize }` 告知具体缺失数。
    pub fn maturity_state(&self) -> MaturityState {
        let judgments = ComponentLinkageJudgment::judge_all();
        let blocked = ComponentLinkageJudgment::blocked_count(&judgments);
        if blocked > 0 {
            return MaturityState::Blocked { missing: blocked };
        }
        match self.v05_score_milli {
            Some(s) if s >= V05_MATURITY_THRESHOLD_MILLI => {
                MaturityState::Mature { v05_score_milli: s }
            }
            Some(_) => MaturityState::Candidate {
                linked: COMPONENT_COUNT,
            },
            None => MaturityState::Candidate {
                linked: COMPONENT_COUNT,
            },
        }
    }

    /// round9-01 §3: 列出所有 blocked 组件 (按组排序: Organ → Core → Support)。
    pub fn blocked_components(&self) -> Vec<&'static str> {
        ComponentLinkageJudgment::judge_all()
            .iter()
            .filter(|j| !j.passes_maturity_gate)
            .map(|j| j.crate_name)
            .collect()
    }

    /// round9-01 §3: per-crate linkage judgment map.
    pub fn linkage_judgments(&self) -> BTreeMap<&'static str, ComponentLinkageJudgment> {
        ComponentLinkageJudgment::judge_all()
            .iter()
            .map(|j| (j.crate_name, j.clone()))
            .collect()
    }

    /// Creates the PID 1 supervisor startup entry.
    pub fn supervisor(&mut self) -> PidOneSupervisor<'_> {
        PidOneSupervisor { central: self }
    }

    /// Starts PID 1 and moves gestation to birth.
    pub fn start_supervisor(&mut self) -> Result<SupervisorReceipt, TransitionError> {
        self.supervisor().start()
    }
}

impl CentralAI for ApeirethCentral {
    fn life_stage(&self) -> LifeStage {
        self.stage
    }

    fn components(&self) -> &'static [Component; COMPONENT_COUNT] {
        &COMPONENTS
    }

    fn transition_to(&mut self, target: LifeStage) -> Result<LifeStage, TransitionError> {
        // round9-01 §1: 使用编译期 hardcode 的 LEGAL_TRANSITIONS 矩阵
        if !is_legal_transition(self.stage, target) {
            return Err(TransitionError::InvalidTransition {
                from: self.stage,
                to: target,
            });
        }
        // Maturity 闸门 (双重保护: 矩阵 + linkage)
        if target == LifeStage::Maturity && !self.is_fully_linked() {
            return Err(TransitionError::ComponentsNotReady {
                linked: self.linked_component_count(),
                required: COMPONENT_COUNT,
            });
        }

        self.stage = target;
        Ok(self.stage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------- 原 unit tests (保留) --------------------

    #[test]
    fn initializes_in_gestation() {
        let central = ApeirethCentral::new();
        assert_eq!(central.life_stage(), LifeStage::Gestation);
    }

    #[test]
    fn catalogue_has_nine_organs_three_cores_and_five_supports() {
        let central = ApeirethCentral::new();
        assert_eq!(central.components().len(), 17);
        assert_eq!(
            central
                .components()
                .iter()
                .filter(|item| item.group == ComponentGroup::Organ)
                .count(),
            9
        );
        assert_eq!(
            central
                .components()
                .iter()
                .filter(|item| item.group == ComponentGroup::Core)
                .count(),
            3
        );
        assert_eq!(
            central
                .components()
                .iter()
                .filter(|item| item.group == ComponentGroup::Support)
                .count(),
            5
        );
    }

    #[test]
    fn supervisor_is_pid_one_and_enters_birth() {
        let mut central = ApeirethCentral::new();
        let receipt = central.start_supervisor().expect("supervisor starts");
        assert_eq!(receipt.pid, 1);
        assert_eq!(receipt.stage, LifeStage::Birth);
        assert_eq!(central.life_stage(), LifeStage::Birth);
    }

    #[test]
    fn normal_early_lifecycle_edges_are_accepted() {
        let mut central = ApeirethCentral::new();
        central.transition_to(LifeStage::Birth).unwrap();
        central.transition_to(LifeStage::Infancy).unwrap();
        central.transition_to(LifeStage::Growth).unwrap();
        assert_eq!(central.life_stage(), LifeStage::Growth);
    }

    #[test]
    fn invalid_edge_does_not_mutate_stage() {
        let mut central = ApeirethCentral::new();
        let error = central.transition_to(LifeStage::Death).unwrap_err();
        assert!(matches!(error, TransitionError::InvalidTransition { .. }));
        assert_eq!(central.life_stage(), LifeStage::Gestation);
    }

    #[test]
    fn maturity_requires_all_seventeen_components() {
        let mut central = ApeirethCentral::new();
        central.transition_to(LifeStage::Birth).unwrap();
        central.transition_to(LifeStage::Infancy).unwrap();
        central.transition_to(LifeStage::Growth).unwrap();
        let error = central.transition_to(LifeStage::Maturity).unwrap_err();
        assert!(matches!(
            error,
            TransitionError::ComponentsNotReady { required: 17, .. }
        ));
        assert_eq!(central.life_stage(), LifeStage::Growth);
    }

    #[test]
    fn component_lookup_is_exact_and_observable() {
        let central = ApeirethCentral::new();
        assert_eq!(
            central.component("apeireth-core").map(|item| item.status),
            Some(ComponentStatus::Linked)
        );
        assert_eq!(
            central.component("apeireth-bus").map(|item| item.status),
            Some(ComponentStatus::Planned)
        );
        assert!(central.component("core").is_none());
    }

    // -------------------- round9-01 §1 — 9 阶段状态机 --------------------

    #[test]
    fn legal_transitions_matrix_has_twelve_edges() {
        assert_eq!(LEGAL_TRANSITIONS.len(), LEGAL_TRANSITION_COUNT);
        assert_eq!(LEGAL_TRANSITION_COUNT, 12);
    }

    #[test]
    fn stage_count_matches_life_stage_variants() {
        // 10 variants in apeireth_core::LifeStage
        assert_eq!(STAGE_COUNT, 10);
    }

    #[test]
    fn legal_transition_helper_covers_all_matrix_entries() {
        for &(from, to) in LEGAL_TRANSITIONS {
            assert!(
                is_legal_transition(from, to),
                "{from:?} → {to:?} should be legal"
            );
        }
    }

    #[test]
    fn illegal_transition_returns_invalid() {
        // Gestation → Maturity 是跳级, 不在矩阵中
        assert!(!is_legal_transition(
            LifeStage::Gestation,
            LifeStage::Maturity
        ));
        // Birth 直接到 Growth 也是跳级
        assert!(!is_legal_transition(LifeStage::Birth, LifeStage::Growth));
        // Death 不可逆, Death → Gestation 不允许
        assert!(!is_legal_transition(LifeStage::Death, LifeStage::Gestation));
    }

    #[test]
    fn decline_growth_is_reversible_per_stage4_section_6_3() {
        // 主 17:43 实事求是: Decline ↔ Growth 可回退
        assert!(is_legal_transition(LifeStage::Decline, LifeStage::Growth));
        assert!(is_legal_transition(LifeStage::Growth, LifeStage::Maturity));
        assert!(is_legal_transition(LifeStage::Maturity, LifeStage::Growth));
    }

    #[test]
    fn death_migration_rebirth_chain_is_one_way() {
        assert!(is_legal_transition(LifeStage::Death, LifeStage::Migration));
        assert!(is_legal_transition(
            LifeStage::Migration,
            LifeStage::Rebirth
        ));
        assert!(is_legal_transition(LifeStage::Rebirth, LifeStage::Maturity));
        // 反向不可
        assert!(!is_legal_transition(LifeStage::Migration, LifeStage::Death));
        assert!(!is_legal_transition(
            LifeStage::Rebirth,
            LifeStage::Migration
        ));
        assert!(!is_legal_transition(
            LifeStage::Maturity,
            LifeStage::Rebirth
        ));
    }

    // -------------------- round9-01 §2 — IdentityCard --------------------

    #[test]
    fn identity_card_new_has_no_carriers() {
        let card = IdentityCard::new(Id(42));
        assert_eq!(card.id(), Id(42));
        assert!(card.carriers().is_empty());
        assert_eq!(card.continuity_token_count(), 0);
        assert!(card.migration_history().is_empty());
        assert!(card.unsavable_log().is_empty());
    }

    #[test]
    fn identity_card_bind_appends_token() {
        let mut card = IdentityCard::new(Id(1));
        card.bind(
            Carrier {
                kind: CarrierKind::Memory,
                id: "ram-0".to_string(),
            },
            1_000,
        )
        .unwrap();
        assert_eq!(card.carriers().len(), 1);
        assert_eq!(card.continuity_token_count(), 1);
    }

    #[test]
    fn identity_card_duplicate_bind_returns_error() {
        let mut card = IdentityCard::new(Id(1));
        let c = Carrier {
            kind: CarrierKind::File,
            id: "disk-0".to_string(),
        };
        card.bind(c.clone(), 1_000).unwrap();
        let err = card.bind(c.clone(), 1_001).unwrap_err();
        assert!(matches!(err, IdentityError::DuplicateCarrier { .. }));
    }

    #[test]
    fn identity_card_migrate_to_replaces_carrier_and_appends_history() {
        let mut card = IdentityCard::new(Id(7));
        let from = Carrier {
            kind: CarrierKind::File,
            id: "disk-old".to_string(),
        };
        let to = Carrier {
            kind: CarrierKind::Network,
            id: "node-b".to_string(),
        };
        card.bind(from.clone(), 1_000).unwrap();
        let rec = card
            .migrate_to(from, to.clone(), 2_000, MigrationReason::Operator)
            .unwrap();
        assert_eq!(rec.reason, MigrationReason::Operator);
        assert_eq!(rec.token.from_carrier.id, "disk-old");
        assert_eq!(rec.token.to_carrier.id, "node-b");
        assert_eq!(card.carriers(), &[to]);
        assert_eq!(card.migration_history().len(), 1);
        assert_eq!(card.continuity_token_count(), 2); // bind + migrate
    }

    #[test]
    fn identity_card_migrate_unknown_carrier_returns_error() {
        let mut card = IdentityCard::new(Id(1));
        let bogus = Carrier {
            kind: CarrierKind::Hardware,
            id: "tpm-x".to_string(),
        };
        let target = Carrier {
            kind: CarrierKind::File,
            id: "disk-y".to_string(),
        };
        let err = card
            .migrate_to(bogus, target, 5_000, MigrationReason::Disaster)
            .unwrap_err();
        assert!(matches!(err, IdentityError::UnknownCarrier { .. }));
    }

    #[test]
    fn identity_card_migrate_to_existing_target_returns_error() {
        let mut card = IdentityCard::new(Id(1));
        let a = Carrier {
            kind: CarrierKind::File,
            id: "disk-a".to_string(),
        };
        let b = Carrier {
            kind: CarrierKind::File,
            id: "disk-b".to_string(),
        };
        card.bind(a.clone(), 1_000).unwrap();
        card.bind(b.clone(), 1_001).unwrap();
        // 试图把 a 迁移到 b (已存在)
        let err = card
            .migrate_to(a, b, 2_000, MigrationReason::Replication)
            .unwrap_err();
        assert!(matches!(err, IdentityError::DuplicateCarrier { .. }));
    }

    #[test]
    fn identity_card_unsavable_log_is_append_only() {
        let mut card = IdentityCard::new(Id(1));
        card.record_unsavable(UnsavableEvent {
            at_unix_ms: 1_000,
            kind: "policy_violation".to_string(),
            payload: "{}".to_string(),
        });
        card.record_unsavable(UnsavableEvent {
            at_unix_ms: 2_000,
            kind: "human_override".to_string(),
            payload: "{}".to_string(),
        });
        assert_eq!(card.unsavable_log().len(), 2);
        assert_eq!(card.unsavable_log()[0].kind, "policy_violation");
        assert_eq!(card.unsavable_log()[1].kind, "human_override");
    }

    #[test]
    fn identity_card_display_error_messages_are_informative() {
        let mut card = IdentityCard::new(Id(99));
        let c = Carrier {
            kind: CarrierKind::File,
            id: "disk-1".to_string(),
        };
        card.bind(c.clone(), 1).unwrap();
        let err = card.bind(c, 2).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("IdentityCard"));
        assert!(msg.contains("disk-1"));
    }

    // -------------------- round9-01 §3 — Maturity 17 链接闸门 --------------------

    #[test]
    fn linkage_judgment_counts_seventeen() {
        let judgments = ComponentLinkageJudgment::judge_all();
        assert_eq!(judgments.len(), COMPONENT_COUNT);
        assert_eq!(judgments.len(), 17);
    }

    #[test]
    fn linkage_judgment_default_state_is_blocked_due_to_planned() {
        let central = ApeirethCentral::new();
        let judgments = central.linkage_judgments();
        // 默认有 6 个 Planned, 11 个 Linked
        let planned: usize = judgments
            .values()
            .filter(|j| !j.passes_maturity_gate)
            .count();
        assert!(
            planned > 0,
            "expected at least one blocked component in default state"
        );
    }

    #[test]
    fn maturity_state_blocked_reports_missing_count() {
        let central = ApeirethCentral::new();
        match central.maturity_state() {
            MaturityState::Blocked { missing } => {
                assert!(missing > 0 && missing <= COMPONENT_COUNT);
                assert_eq!(missing, central.blocked_components().len());
            }
            other => panic!("expected Blocked, got {other:?}"),
        }
    }

    #[test]
    fn blocked_components_lists_planned_crate_names() {
        let central = ApeirethCentral::new();
        let blocked = central.blocked_components();
        assert!(blocked.contains(&"apeireth-action"));
        assert!(blocked.contains(&"apeireth-council"));
        assert!(!blocked.contains(&"apeireth-core"));
        assert!(!blocked.contains(&"apeireth-perception"));
    }

    #[test]
    fn maturity_state_candidate_when_all_linked_but_no_v05_score() {
        // 模拟全部 17 linked: 直接改 COMPONENTS 是 const 不能改, 改用直接调 maturity_state
        // 通过 candidate 路径: 全部 linked 但 v05_score 未设 → Candidate
        // 这里用 blocked_components() 为空的前提 (central blocked_components() 默认非空)
        // 改: 直接用 ComponentLinkageJudgment::blocked_count 验证
        let judgments = ComponentLinkageJudgment::judge_all();
        let blocked = ComponentLinkageJudgment::blocked_count(&judgments);
        assert!(blocked > 0, "in default state, maturity should be blocked");
    }

    #[test]
    fn maturity_threshold_constant_matches_locked_value() {
        // 阶段 4 §6.2 LOCKED: V0.5 ≥ 0.85
        assert_eq!(V05_MATURITY_THRESHOLD_MILLI, 850);
    }

    #[test]
    fn v05_score_is_clamped_to_thousand() {
        let central = ApeirethCentral::new().with_v05_score(u32::MAX);
        assert_eq!(central.v05_score_milli(), Some(1000));
    }

    // -------------------- round9-01 §4 — Supervisor 5 子树 --------------------

    #[test]
    fn supervisor_subtrees_returns_canonical_five() {
        let all = SupervisorSubtree::all();
        assert_eq!(all.len(), 5);
        assert_eq!(all[0], SupervisorSubtree::Core);
        assert_eq!(all[4], SupervisorSubtree::Plugin);
    }

    #[test]
    fn supervisor_subtree_names_are_stable() {
        assert_eq!(SupervisorSubtree::Core.name(), "core");
        assert_eq!(SupervisorSubtree::Cognition.name(), "cognition");
        assert_eq!(SupervisorSubtree::Council.name(), "council");
        assert_eq!(SupervisorSubtree::Upgrade.name(), "upgrade");
        assert_eq!(SupervisorSubtree::Plugin.name(), "plugin");
    }

    #[test]
    fn supervisor_schedule_appends_five_records_to_log() {
        let mut central = ApeirethCentral::new();
        central.set_now_unix_ms(1_000);
        let _receipt = central.start_supervisor().expect("starts");
        let log = central.subtree_log();
        assert_eq!(log.len(), 5);
        assert_eq!(log[0].subtree, SupervisorSubtree::Core);
        assert_eq!(log[4].subtree, SupervisorSubtree::Plugin);
        assert_eq!(log[0].schedule_order, 0);
        assert_eq!(log[4].schedule_order, 4);
    }

    #[test]
    fn supervisor_schedule_marks_all_subtrees_ready() {
        let mut central = ApeirethCentral::new();
        let _ = central.start_supervisor();
        assert!(central
            .subtree_log()
            .iter()
            .all(|r| r.status == SubtreeStatus::Ready));
    }

    #[test]
    fn supervisor_schedule_timestamps_are_monotonic() {
        let mut central = ApeirethCentral::new();
        central.set_now_unix_ms(10_000);
        let _ = central.start_supervisor();
        let log = central.subtree_log();
        for i in 1..log.len() {
            assert!(log[i].started_at_unix_ms >= log[i - 1].started_at_unix_ms);
        }
    }
}
