//! `apeireth-orchestration::lifecycle` —— 生物机制族 W2 (SG-18): 资产生命周期内核 v0.
//!
//! # 生物原型
//!
//! - **程序性死亡（凋亡）**：细胞按程序主动退场，清掉不再需要的结构；退场不是"消失"，
//!   而是把材料回收、留下可辨认的痕迹。本模块的对应物是 [`Tombstone`]：资产退役后
//!   记录仍留在账上，带退役时间、原因与可恢复标志——**永不静默删除**。
//! - **海马体终身神经发生**：新生细胞先进入高可塑的试用期，被回路征用、被证据验证的
//!   存活并成熟；长期不被使用的则退回可回收状态（用进废退）。本模块用
//!   `Newborn → Trial → Mature` 表达新生、试用与成熟，用闲置阈值 + 冷静期表达用进废退。
//! - **冷静期（可逆窗口）**：凋亡执行前留一段可逆窗口；窗口内被再次使用即"救回"
//!   （[`LifecycleStage::Retiring`] → [`LifecycleStage::Trial`]），对应新生细胞被回路
//!   征用则存活。窗口结束才落墓碑。
//!
//! # 工程边界（0 装诚实口径）
//!
//! - **纯内存 + 注入时钟**：时间一律 `u64` 毫秒由调用方注入；**0 系统时钟、0 `chrono`、
//!   0 I/O、0 后台线程**，同一输入序列可重放。
//! - **确定性**：账本按 `asset_id` 的 `BTreeMap` 顺序存放（与注册顺序无关）；`evaluate`
//!   的迁移显式稳定排序（`asset_id` → 迁移序号）；容量淘汰用全序比较而非哈希遍历。
//! - **固定容量**：上限含墓碑；满时按 [`LifecycleLedger::register`] 文档化规则淘汰一条，
//!   并把带墓碑的记录交还调用方归档（内核不静默丢记录）。
//! - **不自动删数据**：`evaluate` 只做状态迁移与留墓碑，**不删除记录、不删除底层资产**；
//!   "退役" = 降权 + 墓碑，恢复可翻案（[`LifecycleLedger::revive`]）。
//! - **只给判定，不做动作**：不接工具注册表 / 技能目录 / 记忆清理，不停止服务、不卸载插件、
//!   不改权限、不写审计日志；这些属于 W3 接线。
//! - **0 unsafe / 0 新外部依赖**：只用本 crate 已有的 `serde`。
//!
//! # 状态机与迁移
//!
//! | 当前阶段 | 触发 | 迁移 | 备注 |
//! |---|---|---|---|
//! | `Newborn` | 首次成功使用后 `evaluate` | `Trial` | 使用本身只记账，晋升由 `evaluate` 决定 |
//! | `Trial` | `use_count >= promote_after_uses` | `Mature` | 一次 `evaluate` 内可连续晋升 |
//! | `Newborn` / `Trial` / `Mature` | 闲置 `>= retire_after_unused_ms` | `Retiring` | 进入冷静期；若首次评估已越过阈值 + 冷静期则直接 `Retired(Unused)` |
//! | `Retiring` | 再闲置满 `grace_period_ms` | `Retired(Unused)` + 墓碑 | 总闲置 `>= retire_after_unused_ms + grace_period_ms` |
//! | `Newborn` / `Trial` / `Retiring` | `trial_failures > max_trial_failures` | `Retired(Failed)` + 墓碑 | 失败是确定性证据，**不走冷静期** |
//! | `Retiring` | 再次成功使用 | `Trial` | 救回；闲置时钟以新 `last_used_ms` 重算 |
//! | `Retired` | `revive` | `Trial`，清墓碑 | 唯一复活路径；`use_count` 不重置 |
//! | 任意未退役 | 显式 `retire(reason)` | `Retired(reason)` + 墓碑 | 显式操作可提前于冷静期执行 |
//!
//! # IMPLEMENTED（未接线）
//!
//! 本模块是机制内核 v0：类型、迁移、固定容量、确定性顺序与单元测试齐备，落
//! **IMPLEMENTED** 级；**未接工具注册表 / 技能目录 / 记忆清理 / `self_status` 自述节，
//! 生产路径 0 调用点**（缺口见
//! `docs/04-internal/backend-enhancement/system-gap-ledger-2026-10-18.md` SG-18）。
//! 本批只改本文件：不改运行时装配、不改其它 crate、不碰 LOCKED。

use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// 一天的毫秒数（仅用于推导默认阈值，不对外导出）。
const DAY_MS: u64 = 24 * 60 * 60 * 1_000;

/// 默认账本容量（含墓碑的记录条数上限）。
pub const DEFAULT_CAPACITY: usize = 64;

/// 默认晋升阈值：试用期累计 3 次成功使用后成熟。
pub const DEFAULT_PROMOTE_AFTER_USES: u32 = 3;

/// 默认试用失败上限：3 次；第 4 次失败（`>` 上限）触发 `Failed` 退役。
pub const DEFAULT_MAX_TRIAL_FAILURES: u32 = 3;

/// 默认闲置退役阈值：90 天（毫秒）。一个季度未被触达即进入冷静期。
pub const DEFAULT_RETIRE_AFTER_UNUSED_MS: u64 = 90 * DAY_MS;

/// 默认退役冷静期：7 天（毫秒）。覆盖"周末/假期没用到"的假阳性。
pub const DEFAULT_GRACE_PERIOD_MS: u64 = 7 * DAY_MS;

/// 资产类别（同一账本可混装；类别只影响调用方的解释，不影响迁移规则）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    /// 工具：可调用的外部能力。
    Tool,
    /// 技能：可复用的行为脚本 / 提示词组合。
    Skill,
    /// 记忆轨道：长期记忆的检索 / 沉淀通道。
    MemoryTrack,
    /// 插件：可装卸的扩展。
    Plugin,
}

impl AssetKind {
    /// 稳定短名（日志 / 证据串用，不随 `Debug` 派生漂移）。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tool => "tool",
            Self::Skill => "skill",
            Self::MemoryTrack => "memory_track",
            Self::Plugin => "plugin",
        }
    }
}

/// 资产生命周期阶段。
///
/// 正常路径：`Newborn → Trial → Mature → Retiring → Retired`；
/// `Retired` 是墓碑态，只可由 [`LifecycleLedger::revive`] 回到 `Trial`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleStage {
    /// 新生：刚注册、尚未被使用（高可塑，也最容易被闲置 prune）。
    Newborn,
    /// 试用：已产生首次成功使用，正在积累成熟证据。
    Trial,
    /// 成熟：成功使用次数越过晋升阈值；容量淘汰时受保护（最后才回收）。
    Mature,
    /// 冷静期：已越过闲置阈值，等待 `grace_period_ms` 后落墓碑；仍可被使用救回。
    Retiring,
    /// 墓碑：已退役并留痕；不再参与使用 / 失败记账，只能复活或作为历史保留。
    Retired,
}

impl LifecycleStage {
    /// 稳定短名（日志 / 证据串用）。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Newborn => "newborn",
            Self::Trial => "trial",
            Self::Mature => "mature",
            Self::Retiring => "retiring",
            Self::Retired => "retired",
        }
    }

    /// 是否仍在服务（`Retiring` 算活着：冷静期内还能被使用救回；只有 `Retired` 不算）。
    pub fn is_live(self) -> bool {
        !matches!(self, Self::Retired)
    }
}

/// 退役原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetireReason {
    /// 长期未被触达（闲置 prune）；最温和的退役。
    Unused,
    /// 被新资产 / 新版本顶替（容量淘汰腾位也记此原因）。
    Superseded,
    /// 试用失败超过上限（只作用于未成熟阶段）。
    Failed,
    /// 运维 / 主人显式退役。
    Manual,
}

impl RetireReason {
    /// 稳定短名（日志 / 证据串用）。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unused => "unused",
            Self::Superseded => "superseded",
            Self::Failed => "failed",
            Self::Manual => "manual",
        }
    }

    /// 该原因默认产生可恢复墓碑。
    ///
    /// v0 全部为 `true`：退役只降权、不判死刑，恢复可翻案。字段为 W3 治理预留——
    /// 安全域失败（如凭据泄漏）可由治理收紧为 `false`，[`LifecycleLedger::revive`] 会拒绝。
    pub fn is_recoverable(self) -> bool {
        matches!(
            self,
            Self::Unused | Self::Superseded | Self::Failed | Self::Manual
        )
    }
}

/// 退役墓碑：一次退役的不可变留痕。
///
/// 不变量：账本内任何 `Retired` 记录都必须带墓碑（`retire` / `evaluate` 原子写入）；
/// 因此"退役"永远可被审计、可被复活（[`LifecycleLedger::revive`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tombstone {
    /// 退役发生时间（毫秒，调用方注入）。
    pub retired_ms: u64,
    /// 退役原因。
    pub reason: RetireReason,
    /// 是否允许复活；v0 所有原因均为 `true`（恢复可翻案）。
    pub recoverable: bool,
}

/// 一项资产的完整生命周期记录。
///
/// 不变量：`use_count` 单调不减；正常路径下 `stage == Retired` ⇔ `tombstone.is_some()`；
/// `last_used_ms` 是"最后一次被触达"（成功使用或失败尝试）的时间，用于闲置判定。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleRecord {
    /// 资产标识（账本内唯一）。
    pub asset_id: String,
    /// 资产类别。
    pub kind: AssetKind,
    /// 当前阶段。
    pub stage: LifecycleStage,
    /// 注册时间（毫秒）。
    pub created_ms: u64,
    /// 最后一次被触达时间（毫秒，成功使用或失败尝试都会刷新）。
    pub last_used_ms: u64,
    /// 累计成功使用次数（单调不减，复活也不清零）。
    pub use_count: u32,
    /// 未成熟阶段的累计失败次数（复活时清零，重新试用）。
    pub trial_failures: u32,
    /// 退役墓碑；`Retired` 阶段必须为 `Some`。
    pub tombstone: Option<Tombstone>,
}

impl LifecycleRecord {
    /// 是否已退役（带墓碑的历史记录）。
    pub fn is_retired(&self) -> bool {
        self.stage == LifecycleStage::Retired
    }
}

/// 生命周期策略（全部阈值可注入；[`LifecyclePolicy::default`] 是文档化默认值）。
///
/// | 字段 | 默认 | 含义 |
/// |---|---|---|
/// | `promote_after_uses` | 3 | 试用期累计成功使用达到该值后晋升 `Mature` |
/// | `max_trial_failures` | 3 | 未成熟阶段允许的失败次数上限；**超过**（`>`）即退役 `Failed` |
/// | `retire_after_unused_ms` | 90 天 | 距最后一次触达超过该时长进入 `Retiring` |
/// | `grace_period_ms` | 7 天 | `Retiring` 冷静期；期满才允许自动落墓碑 |
///
/// 阈值允许为 0（如 `grace_period_ms = 0` 表示无冷静期）；调用方自担配置责任。
/// 时间比较全部用饱和加法，配置取 `u64::MAX` 也不会溢出。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LifecyclePolicy {
    /// 晋升 `Mature` 所需的累计成功使用次数。
    pub promote_after_uses: u32,
    /// 未成熟阶段允许的失败次数上限；`trial_failures > max_trial_failures` 触发退役。
    pub max_trial_failures: u32,
    /// 闲置多久进入 `Retiring`（毫秒）。
    pub retire_after_unused_ms: u64,
    /// `Retiring` 冷静期（毫秒）；期满才允许自动落墓碑。
    pub grace_period_ms: u64,
}

impl Default for LifecyclePolicy {
    fn default() -> Self {
        Self {
            promote_after_uses: DEFAULT_PROMOTE_AFTER_USES,
            max_trial_failures: DEFAULT_MAX_TRIAL_FAILURES,
            retire_after_unused_ms: DEFAULT_RETIRE_AFTER_UNUSED_MS,
            grace_period_ms: DEFAULT_GRACE_PERIOD_MS,
        }
    }
}

/// 一次生命周期迁移（`evaluate` 的返回值；`revive` 单独返回）。
///
/// 变体只描述"刚刚发生了什么"，不携带策略快照；同一 `now_ms` 下重复 `evaluate`
/// 不会再返回同一迁移（幂等）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum LifecycleTransition {
    /// 晋升：`Newborn → Trial` 或 `Trial → Mature`。
    Promoted {
        /// 资产 id。
        asset_id: String,
        /// 迁移前阶段。
        from: LifecycleStage,
        /// 迁移后阶段。
        to: LifecycleStage,
        /// 评估时间（毫秒）。
        at_ms: u64,
    },
    /// 进入冷静期（尚未退役）。
    EnteredRetiring {
        /// 资产 id。
        asset_id: String,
        /// 进入冷静期的时间（毫秒）。
        since_ms: u64,
    },
    /// 落墓碑退役（自动或显式）。
    Retired {
        /// 资产 id。
        asset_id: String,
        /// 退役原因。
        reason: RetireReason,
        /// 退役时间（毫秒）。
        retired_ms: u64,
        /// 墓碑是否可恢复（v0 恒为 `true`）。
        recoverable: bool,
    },
    /// 从墓碑复活回试用。
    Revived {
        /// 资产 id。
        asset_id: String,
        /// 复活时间（毫秒）。
        at_ms: u64,
    },
}

impl LifecycleTransition {
    /// 迁移涉及的资产 id。
    pub fn asset_id(&self) -> &str {
        match self {
            Self::Promoted { asset_id, .. }
            | Self::EnteredRetiring { asset_id, .. }
            | Self::Retired { asset_id, .. }
            | Self::Revived { asset_id, .. } => asset_id,
        }
    }

    /// 稳定排序键：先 `asset_id` 字典序，再迁移序号（保证同输入同输出）。
    fn sort_key(&self) -> (&str, u8) {
        match self {
            Self::Promoted { asset_id, to, .. } => (
                asset_id,
                if matches!(to, LifecycleStage::Trial) {
                    0
                } else {
                    1
                },
            ),
            Self::EnteredRetiring { asset_id, .. } => (asset_id, 2),
            Self::Retired { asset_id, .. } => (asset_id, 3),
            Self::Revived { asset_id, .. } => (asset_id, 4),
        }
    }
}

/// 生命周期账本的操作错误。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleError {
    /// 资产 id 去空白后为空。
    EmptyAssetId,
    /// 资产 id 已存在（含已退役墓碑；历史不被静默覆盖）。
    AlreadyExists(String),
    /// 资产 id 不在账本中。
    NotFound(String),
    /// 资产已退役（不能再使用 / 失败记账 / 二次退役）。
    AlreadyRetired(String),
    /// 资产未退役，不能复活。
    NotRetired(String),
    /// 墓碑 `recoverable == false`，拒绝复活。
    NotRecoverable(String),
    /// `Retired` 记录缺墓碑（不变量被破坏的快照）；正常路径不可达。
    MissingTombstone(String),
}

impl std::fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyAssetId => write!(f, "asset id must not be blank"),
            Self::AlreadyExists(asset_id) => write!(f, "asset {asset_id} already exists"),
            Self::NotFound(asset_id) => write!(f, "asset {asset_id} not found"),
            Self::AlreadyRetired(asset_id) => {
                write!(f, "asset {asset_id} is retired; revive it first")
            }
            Self::NotRetired(asset_id) => write!(f, "asset {asset_id} is not retired"),
            Self::NotRecoverable(asset_id) => {
                write!(f, "asset {asset_id} has a non-recoverable tombstone")
            }
            Self::MissingTombstone(asset_id) => {
                write!(f, "asset {asset_id} is retired without a tombstone")
            }
        }
    }
}

impl std::error::Error for LifecycleError {}

/// `register` 的结果：新生记录 +（容量满时）被淘汰交还的记录。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Registration {
    /// 新注册的 `Newborn` 记录。
    pub record: LifecycleRecord,
    /// 为腾出容量而移除的记录；保证带墓碑（原有墓碑，或内核补写的 `Superseded`）。
    pub evicted: Option<LifecycleRecord>,
}

/// 生命周期账本：固定容量 + 确定性顺序的内存账。
///
/// - 迭代 / 查询顺序恒为 `asset_id` 字典序（`BTreeMap`），与注册顺序无关；
/// - `capacity` 含墓碑记录；满时由 [`LifecycleLedger::register`] 淘汰；
/// - 所有时间由调用方注入，账本不读系统时钟；
/// - 不自动删数据：退役保留记录与墓碑；复活清墓碑但保留 `use_count` 与 `created_ms`。
///
/// 序列化：本类型派生 `Serialize` / `Deserialize` 以便快照 / 回放；反序列化**不校验**
/// 不变量（信任边界为本地账本）。从外部导入时应先校验（如 `Retired` 必带墓碑）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleLedger {
    /// 按 `asset_id` 排序的记录表（顺序即确定性迭代顺序）。
    records: BTreeMap<String, LifecycleRecord>,
    /// 当前生效的策略快照。
    policy: LifecyclePolicy,
    /// 容量上限（含墓碑；构造时钳到至少 1）。
    capacity: usize,
}

impl LifecycleLedger {
    /// 新建账本；`capacity` 会被钳到至少 1（0 容量没有可表达的语义）。
    pub fn new(policy: LifecyclePolicy, capacity: usize) -> Self {
        Self {
            records: BTreeMap::new(),
            policy,
            capacity: capacity.max(1),
        }
    }

    /// 当前策略快照。
    pub fn policy(&self) -> LifecyclePolicy {
        self.policy
    }

    /// 容量上限（含墓碑）。
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 账本内记录条数（含墓碑）。
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// 账本是否为空。
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// 按 `asset_id` 查询记录（含墓碑）。
    pub fn get(&self, asset_id: &str) -> Option<&LifecycleRecord> {
        self.records.get(asset_id)
    }

    /// 所有未退役记录（`Newborn` / `Trial` / `Mature` / `Retiring`），按 `asset_id` 排序。
    ///
    /// `Retiring` 仍算 active：冷静期内它还能被使用救回。
    pub fn active(&self) -> Vec<&LifecycleRecord> {
        self.records
            .values()
            .filter(|record| record.stage.is_live())
            .collect()
    }

    /// 所有已退役记录（墓碑），按 `asset_id` 排序。
    pub fn retired(&self) -> Vec<&LifecycleRecord> {
        self.records
            .values()
            .filter(|record| record.is_retired())
            .collect()
    }

    /// 注册一项新资产。
    ///
    /// - `asset_id` 去空白后为空 → [`LifecycleError::EmptyAssetId`]；
    /// - 已存在（含已退役）→ [`LifecycleError::AlreadyExists`]（历史墓碑不被静默覆盖）；
    /// - 容量满时先淘汰一条记录，淘汰规则（确定性、文档化）：
    ///   1. 先淘汰已退役墓碑（归档窗口最边缘，按 `last_used_ms` 最久未用优先）；
    ///   2. 再淘汰未成熟资产（`Newborn` / `Trial` / `Retiring`，同样最久未用优先）；
    ///   3. 只有全部记录都是 `Mature` 时才淘汰 `Mature`（已验证结构受保护，最后回收）；
    ///   4. 同一层级内按 `last_used_ms` → `created_ms` → `asset_id` 字典序定序。
    /// - 淘汰**不是静默删除**：被淘汰记录随 [`Registration::evicted`] 交还调用方；
    ///   若它不是墓碑态，内核先写入 `retired_ms = now_ms`、`reason = Superseded` 的墓碑。
    ///   调用方若需长期留存，应自行归档返回记录（账本容量即其内存窗口）。
    pub fn register(
        &mut self,
        asset_id: &str,
        kind: AssetKind,
        now_ms: u64,
    ) -> Result<Registration, LifecycleError> {
        if asset_id.trim().is_empty() {
            return Err(LifecycleError::EmptyAssetId);
        }
        if self.records.contains_key(asset_id) {
            return Err(LifecycleError::AlreadyExists(asset_id.to_string()));
        }
        let evicted = if self.records.len() >= self.capacity {
            self.evict_least_recently_used(now_ms)
        } else {
            None
        };
        let record = LifecycleRecord {
            asset_id: asset_id.to_string(),
            kind,
            stage: LifecycleStage::Newborn,
            created_ms: now_ms,
            last_used_ms: now_ms,
            use_count: 0,
            trial_failures: 0,
            tombstone: None,
        };
        self.records.insert(asset_id.to_string(), record.clone());
        Ok(Registration { record, evicted })
    }

    /// 记录一次成功使用（调用方在真实使用成功后调用）。
    ///
    /// - `use_count` 饱和加一（单调不减）；`last_used_ms = max(last_used_ms, now_ms)`；
    /// - `Retiring` 阶段的资产被使用即救回 `Trial`（冷静期作废，重新积累成熟证据）；
    /// - `Retired` 资产拒绝使用（先 [`LifecycleLedger::revive`]），避免墓碑被静默绕过；
    /// - 本方法只记账、不晋升；晋升在 [`LifecycleLedger::evaluate`] 中发生。
    pub fn record_use(&mut self, asset_id: &str, now_ms: u64) -> Result<(), LifecycleError> {
        let record = self
            .records
            .get_mut(asset_id)
            .ok_or_else(|| LifecycleError::NotFound(asset_id.to_string()))?;
        if record.stage == LifecycleStage::Retired {
            return Err(LifecycleError::AlreadyRetired(asset_id.to_string()));
        }
        record.use_count = record.use_count.saturating_add(1);
        record.last_used_ms = record.last_used_ms.max(now_ms);
        if record.stage == LifecycleStage::Retiring {
            record.stage = LifecycleStage::Trial;
        }
        Ok(())
    }

    /// 记录一次失败尝试（未成熟阶段的试用证据）。
    ///
    /// - `trial_failures` 饱和加一；失败同样刷新 `last_used_ms`（"被触达"语义，会推迟
    ///   闲置 prune），但**不增加** `use_count`；
    /// - 退役判定仍由 [`LifecycleLedger::evaluate`] 统一做（本方法不直接改阶段）；
    /// - `Retired` 资产拒绝记账。
    pub fn record_failure(&mut self, asset_id: &str, now_ms: u64) -> Result<(), LifecycleError> {
        let record = self
            .records
            .get_mut(asset_id)
            .ok_or_else(|| LifecycleError::NotFound(asset_id.to_string()))?;
        if record.stage == LifecycleStage::Retired {
            return Err(LifecycleError::AlreadyRetired(asset_id.to_string()));
        }
        record.trial_failures = record.trial_failures.saturating_add(1);
        record.last_used_ms = record.last_used_ms.max(now_ms);
        Ok(())
    }

    /// 执行一次自动评估，返回并应用所有迁移。
    ///
    /// 迁移顺序：先按 `asset_id` 字典序，再按迁移类型序号（`Newborn→Trial` →
    /// `Trial→Mature` → `EnteredRetiring` → `Retired`）；同一 `now_ms` 重复调用返回空
    /// （幂等），同输入序列的输出完全确定。
    ///
    /// 自动规则（按记录依次判定）：
    /// 1. `Retired` 跳过；
    /// 2. 未成熟阶段 `trial_failures > max_trial_failures` → 立即 `Retired(Failed)` + 墓碑；
    /// 3. `Newborn` 有一次以上成功使用 → `Trial`；`Trial` 使用数达阈值 → `Mature`
    ///    （一次评估内可连续晋升）；
    /// 4. 闲置满 `retire_after_unused_ms`：未到 `grace_period_ms` 则进入 `Retiring`；
    ///    已越过"阈值 + 冷静期"（如首次评估发生在很久之后）则直接 `Retired(Unused)` + 墓碑。
    ///
    /// 本方法只改账本阶段与墓碑，不删除记录、不触碰任何外部资产。
    pub fn evaluate(&mut self, now_ms: u64) -> Vec<LifecycleTransition> {
        let policy = self.policy;
        let mut transitions = Vec::new();
        for record in self.records.values_mut() {
            if record.stage == LifecycleStage::Retired {
                continue;
            }
            if record.stage != LifecycleStage::Mature
                && record.trial_failures > policy.max_trial_failures
            {
                transitions.push(retire_record(record, now_ms, RetireReason::Failed));
                continue;
            }
            if record.stage == LifecycleStage::Newborn && record.use_count >= 1 {
                record.stage = LifecycleStage::Trial;
                transitions.push(LifecycleTransition::Promoted {
                    asset_id: record.asset_id.clone(),
                    from: LifecycleStage::Newborn,
                    to: LifecycleStage::Trial,
                    at_ms: now_ms,
                });
            }
            if record.stage == LifecycleStage::Trial
                && record.use_count >= policy.promote_after_uses
            {
                record.stage = LifecycleStage::Mature;
                transitions.push(LifecycleTransition::Promoted {
                    asset_id: record.asset_id.clone(),
                    from: LifecycleStage::Trial,
                    to: LifecycleStage::Mature,
                    at_ms: now_ms,
                });
            }
            let idle_deadline = record
                .last_used_ms
                .saturating_add(policy.retire_after_unused_ms);
            let retire_deadline = idle_deadline.saturating_add(policy.grace_period_ms);
            if now_ms >= retire_deadline {
                transitions.push(retire_record(record, now_ms, RetireReason::Unused));
                continue;
            }
            if now_ms >= idle_deadline && record.stage != LifecycleStage::Retiring {
                record.stage = LifecycleStage::Retiring;
                transitions.push(LifecycleTransition::EnteredRetiring {
                    asset_id: record.asset_id.clone(),
                    since_ms: now_ms,
                });
            }
        }
        transitions.sort_by(|left, right| left.sort_key().cmp(&right.sort_key()));
        transitions
    }

    /// 显式退役：立即写墓碑，可提前于冷静期。
    ///
    /// - 资产不存在 → `NotFound`；已退役 → `AlreadyRetired`（**不覆盖**已有墓碑）；
    /// - 任何未退役阶段都可显式退役（调用方对原因负责）；
    /// - v0 墓碑 `recoverable` 恒为 `true`（恢复可翻案）。
    pub fn retire(
        &mut self,
        asset_id: &str,
        reason: RetireReason,
        now_ms: u64,
    ) -> Result<LifecycleTransition, LifecycleError> {
        let record = self
            .records
            .get_mut(asset_id)
            .ok_or_else(|| LifecycleError::NotFound(asset_id.to_string()))?;
        if record.stage == LifecycleStage::Retired {
            return Err(LifecycleError::AlreadyRetired(asset_id.to_string()));
        }
        Ok(retire_record(record, now_ms, reason))
    }

    /// 复活：`Retired → Trial`，清墓碑，重置失败计数，`use_count` 与 `created_ms` 保留。
    ///
    /// - 资产不存在 → `NotFound`；未退役 → `NotRetired`；
    /// - 墓碑 `recoverable == false` → `NotRecoverable`（W3 治理收紧用；v0 正常路径不可达）；
    /// - `Retired` 但缺墓碑（不变量被破坏的快照）→ `MissingTombstone`；
    /// - `last_used_ms` 取 `max(旧值, now_ms)`，闲置时钟从复活时刻重新起算；
    /// - 复活后 `use_count` 不重置：若历史使用次数已达晋升阈值，下一次 `evaluate` 会重新
    ///   晋升 `Mature`——墓碑只降权、不清零证据。
    pub fn revive(
        &mut self,
        asset_id: &str,
        now_ms: u64,
    ) -> Result<LifecycleTransition, LifecycleError> {
        let record = self
            .records
            .get_mut(asset_id)
            .ok_or_else(|| LifecycleError::NotFound(asset_id.to_string()))?;
        if record.stage != LifecycleStage::Retired {
            return Err(LifecycleError::NotRetired(asset_id.to_string()));
        }
        let Some(tombstone) = record.tombstone.as_ref() else {
            return Err(LifecycleError::MissingTombstone(asset_id.to_string()));
        };
        if !tombstone.recoverable {
            return Err(LifecycleError::NotRecoverable(asset_id.to_string()));
        }
        record.stage = LifecycleStage::Trial;
        record.tombstone = None;
        record.trial_failures = 0;
        record.last_used_ms = record.last_used_ms.max(now_ms);
        Ok(LifecycleTransition::Revived {
            asset_id: record.asset_id.clone(),
            at_ms: now_ms,
        })
    }

    /// 容量满时淘汰一条记录并按需补墓碑。
    ///
    /// 选择规则见 [`LifecycleLedger::register`]；`Retired` 墓碑原样交还（不改写原有
    /// `retired_ms` / `reason`），活记录先转 `Retired(Superseded)` 再交还。
    fn evict_least_recently_used(&mut self, now_ms: u64) -> Option<LifecycleRecord> {
        let victim_id = self
            .records
            .values()
            .min_by(|left, right| eviction_cmp(left, right))
            .map(|record| record.asset_id.clone())?;
        let mut victim = self.records.remove(&victim_id)?;
        if victim.stage != LifecycleStage::Retired {
            victim.stage = LifecycleStage::Retired;
            victim.tombstone = Some(Tombstone {
                retired_ms: now_ms,
                reason: RetireReason::Superseded,
                recoverable: RetireReason::Superseded.is_recoverable(),
            });
        }
        Some(victim)
    }
}

impl Default for LifecycleLedger {
    fn default() -> Self {
        Self::new(LifecyclePolicy::default(), DEFAULT_CAPACITY)
    }
}

/// 写墓碑并返回迁移（`evaluate` / `retire` 共用的原子操作）。
fn retire_record(
    record: &mut LifecycleRecord,
    now_ms: u64,
    reason: RetireReason,
) -> LifecycleTransition {
    let recoverable = reason.is_recoverable();
    record.stage = LifecycleStage::Retired;
    record.tombstone = Some(Tombstone {
        retired_ms: now_ms,
        reason,
        recoverable,
    });
    LifecycleTransition::Retired {
        asset_id: record.asset_id.clone(),
        reason,
        retired_ms: now_ms,
        recoverable,
    }
}

/// 容量淘汰的层级：墓碑最先回收，未成熟资产其次，`Mature` 最后（受保护）。
fn eviction_rank(stage: LifecycleStage) -> u8 {
    match stage {
        LifecycleStage::Retired => 0,
        LifecycleStage::Newborn | LifecycleStage::Trial | LifecycleStage::Retiring => 1,
        LifecycleStage::Mature => 2,
    }
}

/// 容量淘汰的全序比较：层级 → 最久未用 → 最早创建 → `asset_id` 字典序。
fn eviction_cmp(left: &LifecycleRecord, right: &LifecycleRecord) -> Ordering {
    eviction_rank(left.stage)
        .cmp(&eviction_rank(right.stage))
        .then_with(|| left.last_used_ms.cmp(&right.last_used_ms))
        .then_with(|| left.created_ms.cmp(&right.created_ms))
        .then_with(|| left.asset_id.cmp(&right.asset_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY_MS: u64 = 24 * 60 * 60 * 1_000;

    fn policy() -> LifecyclePolicy {
        LifecyclePolicy {
            promote_after_uses: 3,
            max_trial_failures: 2,
            retire_after_unused_ms: 1_000,
            grace_period_ms: 200,
        }
    }

    fn ledger() -> LifecycleLedger {
        LifecycleLedger::new(policy(), 8)
    }

    fn ids(transitions: &[LifecycleTransition]) -> Vec<&str> {
        transitions
            .iter()
            .map(LifecycleTransition::asset_id)
            .collect()
    }

    fn promoted(
        asset_id: &str,
        from: LifecycleStage,
        to: LifecycleStage,
        at_ms: u64,
    ) -> LifecycleTransition {
        LifecycleTransition::Promoted {
            asset_id: asset_id.into(),
            from,
            to,
            at_ms,
        }
    }

    fn entered_retiring(asset_id: &str, since_ms: u64) -> LifecycleTransition {
        LifecycleTransition::EnteredRetiring {
            asset_id: asset_id.into(),
            since_ms,
        }
    }

    fn retired(asset_id: &str, reason: RetireReason, retired_ms: u64) -> LifecycleTransition {
        LifecycleTransition::Retired {
            asset_id: asset_id.into(),
            reason,
            retired_ms,
            recoverable: true,
        }
    }

    fn revived(asset_id: &str, at_ms: u64) -> LifecycleTransition {
        LifecycleTransition::Revived {
            asset_id: asset_id.into(),
            at_ms,
        }
    }

    #[test]
    fn default_policy_matches_documented_values() {
        let policy = LifecyclePolicy::default();
        assert_eq!(policy.promote_after_uses, DEFAULT_PROMOTE_AFTER_USES);
        assert_eq!(policy.promote_after_uses, 3);
        assert_eq!(policy.max_trial_failures, DEFAULT_MAX_TRIAL_FAILURES);
        assert_eq!(policy.max_trial_failures, 3);
        assert_eq!(
            policy.retire_after_unused_ms,
            DEFAULT_RETIRE_AFTER_UNUSED_MS
        );
        assert_eq!(policy.retire_after_unused_ms, 90 * DAY_MS);
        assert_eq!(policy.grace_period_ms, DEFAULT_GRACE_PERIOD_MS);
        assert_eq!(policy.grace_period_ms, 7 * DAY_MS);
        assert_eq!(LifecycleLedger::default().capacity(), DEFAULT_CAPACITY);
        assert_eq!(LifecycleLedger::new(policy, 0).capacity(), 1);
    }

    #[test]
    fn state_labels_and_serde_tags_are_stable() {
        assert_eq!(LifecycleStage::Newborn.as_str(), "newborn");
        assert_eq!(LifecycleStage::Retiring.as_str(), "retiring");
        assert_eq!(LifecycleStage::Retired.as_str(), "retired");
        assert_eq!(RetireReason::Failed.as_str(), "failed");
        assert_eq!(AssetKind::MemoryTrack.as_str(), "memory_track");
        assert_eq!(
            serde_json::to_string(&LifecycleStage::Retired).unwrap(),
            "\"retired\""
        );
        let transition = promoted("tool.a", LifecycleStage::Newborn, LifecycleStage::Trial, 7);
        let json = serde_json::to_string(&transition).unwrap();
        assert!(json.contains("\"event\":\"promoted\""));
        let back: LifecycleTransition = serde_json::from_str(&json).unwrap();
        assert_eq!(back, transition);
    }

    #[test]
    fn register_creates_newborn_and_lists_active() {
        let mut ledger = ledger();
        let registration = ledger.register("tool.a", AssetKind::Tool, 1_000).unwrap();
        assert_eq!(registration.evicted, None);
        assert_eq!(Some(&registration.record), ledger.get("tool.a"));
        let record = ledger.get("tool.a").unwrap();
        assert_eq!(record.stage, LifecycleStage::Newborn);
        assert_eq!(record.kind, AssetKind::Tool);
        assert_eq!(record.created_ms, 1_000);
        assert_eq!(record.last_used_ms, 1_000);
        assert_eq!(record.use_count, 0);
        assert_eq!(record.trial_failures, 0);
        assert_eq!(record.tombstone, None);
        assert_eq!(ledger.active().len(), 1);
        assert!(ledger.retired().is_empty());
        assert_eq!(ledger.len(), 1);
        assert!(!ledger.is_empty());
    }

    #[test]
    fn duplicate_and_blank_registration_are_rejected() {
        let mut ledger = ledger();
        ledger.register("skill.a", AssetKind::Skill, 0).unwrap();
        assert_eq!(
            ledger.register("skill.a", AssetKind::Skill, 1),
            Err(LifecycleError::AlreadyExists("skill.a".into()))
        );
        assert_eq!(
            ledger.register("   ", AssetKind::Skill, 2),
            Err(LifecycleError::EmptyAssetId)
        );
        assert_eq!(ledger.len(), 1);
    }

    #[test]
    fn use_promotes_newborn_to_trial_then_mature() {
        let mut ledger = ledger();
        ledger.register("tool.a", AssetKind::Tool, 0).unwrap();
        ledger.record_use("tool.a", 10).unwrap();
        assert_eq!(
            ledger.get("tool.a").unwrap().stage,
            LifecycleStage::Newborn,
            "使用本身只记账，晋升由 evaluate 决定"
        );
        assert_eq!(
            ledger.evaluate(10),
            vec![promoted(
                "tool.a",
                LifecycleStage::Newborn,
                LifecycleStage::Trial,
                10
            )]
        );
        ledger.record_use("tool.a", 20).unwrap();
        ledger.record_use("tool.a", 30).unwrap();
        assert_eq!(
            ledger.evaluate(30),
            vec![promoted(
                "tool.a",
                LifecycleStage::Trial,
                LifecycleStage::Mature,
                30
            )]
        );
        let record = ledger.get("tool.a").unwrap();
        assert_eq!(record.stage, LifecycleStage::Mature);
        assert_eq!(record.use_count, 3);
    }

    #[test]
    fn failure_over_limit_retires_with_failed_tombstone() {
        let mut ledger = ledger();
        ledger.register("tool.b", AssetKind::Tool, 0).unwrap();
        ledger.record_failure("tool.b", 10).unwrap();
        ledger.record_failure("tool.b", 20).unwrap();
        assert!(ledger.evaluate(20).is_empty(), "达到上限但未超过，不退役");
        ledger.record_failure("tool.b", 30).unwrap();
        assert_eq!(
            ledger.evaluate(30),
            vec![retired("tool.b", RetireReason::Failed, 30)],
            "失败是确定性证据，立即留墓碑，不走冷静期"
        );
        let record = ledger.get("tool.b").unwrap();
        assert_eq!(record.stage, LifecycleStage::Retired);
        let tombstone = record.tombstone.as_ref().expect("退役必须留墓碑");
        assert_eq!(tombstone.reason, RetireReason::Failed);
        assert_eq!(tombstone.retired_ms, 30);
        assert!(tombstone.recoverable);
        assert!(ledger.active().is_empty());
        assert_eq!(ledger.retired().len(), 1);
    }

    #[test]
    fn idle_retirement_respects_grace_period() {
        let mut ledger = ledger();
        ledger.register("tool.c", AssetKind::Tool, 0).unwrap();
        assert!(ledger.evaluate(999).is_empty());
        assert_eq!(
            ledger.evaluate(1_000),
            vec![entered_retiring("tool.c", 1_000)]
        );
        assert_eq!(
            ledger.get("tool.c").unwrap().stage,
            LifecycleStage::Retiring
        );
        assert!(ledger.evaluate(1_199).is_empty(), "冷静期未满不得退役");
        assert_eq!(
            ledger.evaluate(1_200),
            vec![retired("tool.c", RetireReason::Unused, 1_200)]
        );
        let record = ledger.get("tool.c").unwrap();
        assert_eq!(record.stage, LifecycleStage::Retired);
        assert_eq!(
            record.tombstone.as_ref().unwrap().reason,
            RetireReason::Unused
        );
    }

    #[test]
    fn evaluate_is_idempotent_at_the_same_timestamp() {
        let mut idle_ledger = ledger();
        idle_ledger.register("tool.a", AssetKind::Tool, 0).unwrap();
        assert_eq!(idle_ledger.evaluate(1_000).len(), 1);
        assert!(idle_ledger.evaluate(1_000).is_empty());
        assert_eq!(idle_ledger.evaluate(1_200).len(), 1);
        assert!(idle_ledger.evaluate(1_200).is_empty());
        assert_eq!(idle_ledger.retired().len(), 1);

        let mut promotion_ledger = ledger();
        promotion_ledger
            .register("tool.p", AssetKind::Tool, 0)
            .unwrap();
        promotion_ledger.record_use("tool.p", 10).unwrap();
        assert_eq!(promotion_ledger.evaluate(10).len(), 1);
        assert!(
            promotion_ledger.evaluate(10).is_empty(),
            "同 now 重复评估不得重复晋升"
        );
        assert_eq!(
            promotion_ledger.get("tool.p").unwrap().stage,
            LifecycleStage::Trial
        );
    }

    #[test]
    fn explicit_retire_always_leaves_a_tombstone() {
        let mut ledger = ledger();
        ledger.register("plugin.a", AssetKind::Plugin, 0).unwrap();
        assert_eq!(
            ledger.retire("plugin.a", RetireReason::Manual, 50).unwrap(),
            retired("plugin.a", RetireReason::Manual, 50)
        );
        let record = ledger.get("plugin.a").unwrap();
        assert_eq!(record.stage, LifecycleStage::Retired);
        assert_eq!(record.tombstone.as_ref().unwrap().retired_ms, 50);
        assert_eq!(
            ledger.retire("plugin.a", RetireReason::Failed, 60),
            Err(LifecycleError::AlreadyRetired("plugin.a".into()))
        );
        assert_eq!(
            ledger
                .get("plugin.a")
                .unwrap()
                .tombstone
                .unwrap()
                .retired_ms,
            50,
            "第二次退役不得覆盖第一块墓碑"
        );
    }

    #[test]
    fn retiring_asset_is_rescued_by_use() {
        let mut ledger = ledger();
        ledger
            .register("memory.a", AssetKind::MemoryTrack, 0)
            .unwrap();
        assert_eq!(ledger.evaluate(1_000).len(), 1);
        assert_eq!(
            ledger.get("memory.a").unwrap().stage,
            LifecycleStage::Retiring
        );
        ledger.record_use("memory.a", 1_100).unwrap();
        {
            let record = ledger.get("memory.a").unwrap();
            assert_eq!(record.stage, LifecycleStage::Trial);
            assert_eq!(record.last_used_ms, 1_100);
            assert_eq!(record.use_count, 1);
        }
        assert!(
            ledger.evaluate(1_200).is_empty(),
            "救回后以新的 last_used 重算闲置，旧截止线作废"
        );
        assert_eq!(ledger.get("memory.a").unwrap().stage, LifecycleStage::Trial);
    }

    #[test]
    fn revive_only_from_retired_and_clears_tombstone() {
        let mut ledger = ledger();
        ledger.register("skill.a", AssetKind::Skill, 0).unwrap();
        for at in [10, 20, 30] {
            ledger.record_use("skill.a", at).unwrap();
        }
        assert_eq!(ledger.evaluate(30).len(), 2);
        assert_eq!(ledger.get("skill.a").unwrap().stage, LifecycleStage::Mature);
        ledger
            .retire("skill.a", RetireReason::Superseded, 40)
            .unwrap();
        assert_eq!(ledger.revive("skill.a", 50), Ok(revived("skill.a", 50)));
        let record = ledger.get("skill.a").unwrap();
        assert_eq!(record.stage, LifecycleStage::Trial);
        assert_eq!(record.tombstone, None);
        assert_eq!(record.use_count, 3, "use_count 单调，不因复活清零");
        assert_eq!(record.trial_failures, 0);
        assert_eq!(record.created_ms, 0);
        assert_eq!(record.last_used_ms, 50);
        assert!(ledger.retired().is_empty());
        assert_eq!(ledger.active().len(), 1);
        assert_eq!(
            ledger.revive("skill.a", 60),
            Err(LifecycleError::NotRetired("skill.a".into()))
        );
    }

    #[test]
    fn revive_refuses_non_recoverable_tombstone() {
        let mut ledger = ledger();
        ledger.register("tool.frozen", AssetKind::Tool, 0).unwrap();
        ledger
            .retire("tool.frozen", RetireReason::Failed, 10)
            .unwrap();
        ledger
            .records
            .get_mut("tool.frozen")
            .unwrap()
            .tombstone
            .as_mut()
            .unwrap()
            .recoverable = false;
        assert_eq!(
            ledger.revive("tool.frozen", 20),
            Err(LifecycleError::NotRecoverable("tool.frozen".into()))
        );
        assert_eq!(
            ledger.get("tool.frozen").unwrap().stage,
            LifecycleStage::Retired
        );
    }

    #[test]
    fn evaluate_transitions_are_sorted_by_asset_id() {
        let mut ledger = LifecycleLedger::new(
            LifecyclePolicy {
                promote_after_uses: 1,
                ..policy()
            },
            8,
        );
        for id in ["tool.c", "tool.a", "tool.b"] {
            ledger.register(id, AssetKind::Tool, 0).unwrap();
            ledger.record_use(id, 10).unwrap();
        }
        let transitions = ledger.evaluate(10);
        assert_eq!(
            ids(&transitions),
            vec!["tool.a", "tool.a", "tool.b", "tool.b", "tool.c", "tool.c"],
            "注册顺序不影响迁移顺序"
        );
        assert_eq!(
            transitions[0],
            promoted("tool.a", LifecycleStage::Newborn, LifecycleStage::Trial, 10)
        );
        assert_eq!(
            transitions[1],
            promoted("tool.a", LifecycleStage::Trial, LifecycleStage::Mature, 10)
        );
        assert!(ledger.evaluate(10).is_empty());
    }

    #[test]
    fn capacity_evicts_least_recently_used_immature_with_tombstone() {
        let mut ledger = LifecycleLedger::new(policy(), 2);
        ledger.register("tool.a", AssetKind::Tool, 0).unwrap();
        for at in [10, 20, 30] {
            ledger.record_use("tool.a", at).unwrap();
        }
        ledger.evaluate(30);
        assert_eq!(ledger.get("tool.a").unwrap().stage, LifecycleStage::Mature);
        ledger.register("tool.b", AssetKind::Skill, 40).unwrap();
        let registration = ledger
            .register("tool.c", AssetKind::MemoryTrack, 50)
            .unwrap();
        let evicted = registration.evicted.expect("容量满必须交还被淘汰记录");
        assert_eq!(
            evicted.asset_id, "tool.b",
            "Mature 受保护：先淘汰未成熟资产"
        );
        assert_eq!(evicted.stage, LifecycleStage::Retired);
        let tombstone = evicted.tombstone.expect("淘汰也必须带墓碑");
        assert_eq!(tombstone.reason, RetireReason::Superseded);
        assert_eq!(tombstone.retired_ms, 50);
        assert!(ledger.get("tool.b").is_none());
        assert_eq!(ledger.len(), 2);
    }

    #[test]
    fn capacity_falls_back_to_least_recently_used_mature() {
        let mut ledger = LifecycleLedger::new(policy(), 1);
        ledger.register("tool.a", AssetKind::Tool, 0).unwrap();
        for at in [10, 20, 30] {
            ledger.record_use("tool.a", at).unwrap();
        }
        ledger.evaluate(30);
        let registration = ledger.register("tool.b", AssetKind::Tool, 40).unwrap();
        let evicted = registration.evicted.expect("容量满必须淘汰");
        assert_eq!(evicted.asset_id, "tool.a");
        assert_eq!(
            evicted.tombstone.expect("淘汰必须带墓碑").reason,
            RetireReason::Superseded
        );
    }

    #[test]
    fn capacity_eviction_of_retired_record_keeps_original_tombstone() {
        let mut ledger = LifecycleLedger::new(policy(), 2);
        ledger.register("tool.a", AssetKind::Tool, 0).unwrap();
        ledger.register("tool.b", AssetKind::Tool, 10).unwrap();
        ledger.retire("tool.b", RetireReason::Manual, 100).unwrap();
        let evicted = ledger
            .register("tool.c", AssetKind::Tool, 200)
            .unwrap()
            .evicted
            .expect("容量满必须淘汰");
        assert_eq!(evicted.asset_id, "tool.b");
        let tombstone = evicted.tombstone.expect("退役记录必带墓碑");
        assert_eq!(tombstone.reason, RetireReason::Manual);
        assert_eq!(tombstone.retired_ms, 100, "已退役墓碑不因腾位改写");
    }

    #[test]
    fn unknown_assets_are_rejected_by_every_mutator() {
        let mut ledger = ledger();
        assert_eq!(
            ledger.record_use("ghost", 1),
            Err(LifecycleError::NotFound("ghost".into()))
        );
        assert_eq!(
            ledger.record_failure("ghost", 1),
            Err(LifecycleError::NotFound("ghost".into()))
        );
        assert_eq!(
            ledger.retire("ghost", RetireReason::Manual, 1),
            Err(LifecycleError::NotFound("ghost".into()))
        );
        assert_eq!(
            ledger.revive("ghost", 1),
            Err(LifecycleError::NotFound("ghost".into()))
        );
    }

    #[test]
    fn retired_assets_cannot_be_used_without_revive() {
        let mut ledger = ledger();
        ledger.register("tool.a", AssetKind::Tool, 0).unwrap();
        ledger.record_use("tool.a", 10).unwrap();
        ledger.retire("tool.a", RetireReason::Unused, 20).unwrap();
        assert_eq!(
            ledger.record_use("tool.a", 30),
            Err(LifecycleError::AlreadyRetired("tool.a".into()))
        );
        assert_eq!(
            ledger.record_failure("tool.a", 30),
            Err(LifecycleError::AlreadyRetired("tool.a".into()))
        );
        assert_eq!(ledger.get("tool.a").unwrap().use_count, 1);
    }

    #[test]
    fn mature_assets_do_not_retire_on_trial_failures() {
        let mut ledger = ledger();
        ledger.register("tool.a", AssetKind::Tool, 0).unwrap();
        for at in [10, 20, 30] {
            ledger.record_use("tool.a", at).unwrap();
        }
        ledger.evaluate(30);
        for at in [40, 50, 60, 70] {
            ledger.record_failure("tool.a", at).unwrap();
        }
        assert!(
            ledger.evaluate(70).is_empty(),
            "trial 失败上限只作用于未成熟阶段"
        );
        assert_eq!(ledger.get("tool.a").unwrap().stage, LifecycleStage::Mature);
    }

    #[test]
    fn late_evaluate_retires_directly_once_grace_has_passed() {
        let mut ledger = ledger();
        ledger.register("tool.a", AssetKind::Tool, 0).unwrap();
        let transitions = ledger.evaluate(1_200);
        assert_eq!(
            transitions,
            vec![retired("tool.a", RetireReason::Unused, 1_200)],
            "补扫越过阈值 + 冷静期时直接留墓碑"
        );
        assert_eq!(ledger.get("tool.a").unwrap().stage, LifecycleStage::Retired);
    }

    #[test]
    fn serde_roundtrip_is_lossless() {
        let mut ledger = ledger();
        ledger.register("tool.a", AssetKind::Tool, 0).unwrap();
        ledger.record_use("tool.a", 10).unwrap();
        ledger.evaluate(10);
        ledger.retire("tool.a", RetireReason::Manual, 20).unwrap();

        let record = ledger.get("tool.a").unwrap().clone();
        let record_json = serde_json::to_string(&record).unwrap();
        let record_back: LifecycleRecord = serde_json::from_str(&record_json).unwrap();
        assert_eq!(record_back, record);

        let policy_json = serde_json::to_string(&policy()).unwrap();
        let policy_back: LifecyclePolicy = serde_json::from_str(&policy_json).unwrap();
        assert_eq!(policy_back, policy());

        let ledger_json = serde_json::to_string(&ledger).unwrap();
        let ledger_back: LifecycleLedger = serde_json::from_str(&ledger_json).unwrap();
        assert_eq!(ledger_back, ledger);
    }
}
