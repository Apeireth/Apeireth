//! ADR-0003 / R1a：神经系统只读聚合（schema v2，`capabilities.nervous_system`）。
//!
//! v2 = v1 字段全部不变、只 additive 增加 `bus` 子对象（订阅/发布/投递计数，
//! ADR-0009 §D7 / R1b-4a）；`last_observation` 仍一律 `null`（总线不保留历史，
//! 不编造读数）。`bus = null` 表示本帧未绑总线（例如测试或未接线构造）。
//!
//! **合同口径（与 ADR 措辞的差异，已按实际落地）**：ADR 原文写「21 节 → 22 节」；
//! 本实现沿用 W3.2 `bio_mechanisms` 的既有口径——**顶层 21 节冻结合同不变**，
//! `nervous_system` 是 `capabilities` 节内的**第二个只读子对象**（第一个是
//! `bio_mechanisms`），不是新的顶层节。[`SELF_STATUS_SECTIONS`] 仍为 21 节。
//!
//! 数据来源（全部**只读**）：
//! - `enabled` 用与生产**同一批解析函数**（`bio_tone_enabled_from_env_value` /
//!   `bio_circadian_enabled_from_env_value` / `bio_care_candidates_enabled_from_env_value` /
//!   `bio_play_enabled_from_env_value` / `quorum_gate_enabled_from_env_value` /
//!   `bio_switch_enabled_from_env_value` / `bio_self_status_enabled_from_env_value`）
//!   解析既有 env 常量；不新增开关、不改开关语义、不构造任何机制实例；
//! - `observed` = 当前 `bio_mechanisms` 帧里 `wired: true` 的格数，即注册表
//!   **真实注册的 probe 数**（未注册 = 0；开关关时整节不采）。
//! - `last_observation` 一律 `null`：本模块不承载读数，读数由
//!   `capabilities.bio_mechanisms` 逐格承载（0 装诚实，不伪造观测时间）。
//!
//! 入口：`SelfStatusModule` 用 [`nervous_self_status_tool`] 包住既有的
//! `tool.self_status` 工具；装饰器只在 `capabilities.bio_mechanisms.enabled`
//! 为 `true`（即 `APEIRETH_ENABLE_BIO_SELF_STATUS=1`，与既有挂面开关共用）时
//! 插入子对象。开关关 = **不读 env、不构造快照、不插键**，旧帧形状逐字节不变。
//!
//! 回滚：删本模块 + `tool_modules.rs` 两行接线（见 ADR-0003 §4）。

use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, SessionId};
use apeireth_plugin::{FrozenInvocation, ToolCapability};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolOutcome, ToolResult};
use apeireth_tools_canonical::self_status::{
    bio_self_status_enabled_from_env_value, BioMechanismKind, SelfStatusSource, SelfStatusTool,
    BIO_SELF_STATUS_ENABLE_ENV,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::canonical::bio_circadian::{
    bio_circadian_enabled_from_env_value, BIO_CIRCADIAN_ENABLE_ENV,
};
use crate::canonical::bio_tone::{bio_tone_enabled_from_env_value, BIO_TONE_ENABLE_ENV};
use crate::canonical::care_reminder::{
    bio_care_candidates_enabled_from_env_value, BIO_CARE_CANDIDATES_ENABLE_ENV,
};
use crate::canonical::cognitive::{
    bio_switch_enabled_from_env_value, BIO_CONTEXT_POLICY_ENABLE_ENV,
    BIO_REPAIR_GUIDANCE_ENABLE_ENV,
};
use crate::canonical::nervous_bus::{NervousBus, NervousBusSnapshot};
use crate::canonical::orchestrator::{quorum_gate_enabled_from_env_value, QUORUM_GATE_ENABLE_ENV};
use crate::canonical::play_tools::{bio_play_enabled_from_env_value, BIO_PLAY_ENABLE_ENV};

/// 自述帧内承载本子对象的节名（`capabilities`，既有 21 节之一）。
pub const CAPABILITIES_SECTION: &str = "capabilities";
/// `capabilities` 节内的只读子对象键（与 `bio_mechanisms` 并列，非顶层节）。
pub const NERVOUS_SYSTEM_SUBOBJECT: &str = "nervous_system";
/// schema 版本（v2 = v1 字段不变 + `bus` additive，ADR-0009 §D7 / R1b-4a）。
pub const NERVOUS_SNAPSHOT_VERSION: u32 = 2;
/// 机制总数（SG-13…SG-26 共 14 条，闭合词表）。
pub const NERVOUS_MECHANISM_TOTAL: usize = 14;

/// 免疫记忆开关 env 名（生产解析在 cli；此处只引用**既有名字**，不新增开关）。
pub const IMMUNE_MEMORY_ENABLE_ENV: &str = "APEIRETH_ENABLE_IMMUNE_MEMORY";
/// 痛觉 / 再生共用开关 env 名（生产解析在 cli；此处只引用既有名字）。
pub const BIO_PAIN_ENABLE_ENV: &str = "APEIRETH_ENABLE_BIO_PAIN";
/// 生命周期装配入口 env 名（`bio_lifecycle` 模块的既有名字；装配根已由
/// `bio_production_wiring::install_bio_modules` 注册，见 `production.rs`）。
pub const BIO_LIFECYCLE_ENABLE_ENV: &str = "APEIRETH_ENABLE_BIO_LIFECYCLE";

/// 机制接线级别（台账四级口径的三种现状。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MechanismLevel {
    /// 装配根已接线（默认关）。
    #[serde(rename = "PRODUCTION WIRED")]
    ProductionWired,
    /// 半程：管道已接，仍有显式未接段（gaps 必填）。
    #[serde(rename = "HALF")]
    Half,
    /// 内核已实现但未接线（gaps 必填）。
    #[serde(rename = "IMPLEMENTED")]
    Implemented,
}

/// 信号源状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalState {
    /// 真实信号源已接。
    Wired,
    /// 已接但只覆盖部分输入（gaps/reason 说明）。
    Partial,
    /// 未接（reason 必填，不编造读数）。
    NotWired,
}

/// 一条信号源的只读描述。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalSource {
    /// 稳定标识（snake_case，不随实现漂移）。
    pub id: String,
    /// 接线状态。
    pub state: SignalState,
    /// 未接 / 局部的原因（`wired` 时可为 `null`）。
    pub reason: Option<String>,
}

/// 一条机制的只读账本行（schema v1）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NervousMechanism {
    /// 契约命名：`SG-<编号>.<snake_name>`（与台账/矩阵 §1.1 生物 14 行一致）。
    pub id: String,
    /// 接线级别（见 [`MechanismLevel`]）。
    pub level: MechanismLevel,
    /// 装配开关 env 名；无开关 = `null`（不虚构开关）。
    pub enabled_env: Option<String>,
    /// 开关**生效值**（只反映 env 实况，不推断可用性）。
    pub enabled: bool,
    /// 接线点描述（稳定短串，不携带运行时数据）。
    pub wiring: String,
    /// 信号源逐条状态。
    pub signal_sources: Vec<SignalSource>,
    /// 最近观测；本 schema 恒 `null`（读数由 `bio_mechanisms` 承载）。
    pub last_observation: Option<Value>,
    /// 已知缺口（半程 / 未接线条目必填；不隐藏未接段）。
    pub gaps: Vec<String>,
}

/// 汇总计数（与 `mechanisms` 同一次采集，不手填）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NervousSummary {
    /// 机制总数（恒 14）。
    pub total: usize,
    /// `level = PRODUCTION WIRED` 的行数。
    pub production_wired: usize,
    /// `level = HALF` 的行数。
    pub half: usize,
    /// `level = IMPLEMENTED` 的行数（未接线）。
    pub unwired: usize,
    /// 真实注册 probe 的机制数（从 `bio_mechanisms` 的 `wired:true` 格数取）。
    pub observed: usize,
}

/// 总线观测面（schema v2 additive；ADR-0009 §D7 / R1b-4a）。
///
/// 只报**已发生的事实**：订阅者/发布/投递计数从 [`NervousBus::snapshot`] 现取。
/// 总线不保留历史信号载荷，故 `last_observation` 恒 `null`（不编造读数）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NervousBusStatus {
    /// 全部信号类别的订阅者数合计。
    pub subscribers: usize,
    /// 全部信号类别的发布次数合计。
    pub published: u64,
    /// 全部类别累计投递次数。
    pub delivered: u64,
    /// 恒 `null`：总线不保留历史观测（ADR-0009）。
    pub last_observation: Option<String>,
}

/// `capabilities.nervous_system` 只读子对象（schema v2）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NervousSnapshot {
    /// schema 版本（恒 [`NERVOUS_SNAPSHOT_VERSION`]）。
    pub version: u32,
    /// 汇总计数。
    pub summary: NervousSummary,
    /// 14 条机制（行序 = [`BioMechanismKind::ALL`]，确定性）。
    pub mechanisms: Vec<NervousMechanism>,
    /// 总线观测面；本帧未绑总线时为 `null`（v2 additive 字段）。
    pub bus: Option<NervousBusStatus>,
}

/// env 读取口（测试注入；默认读进程 env）。
pub type EnvReader = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;

/// 从 env 现值采集一份快照（只读；调用方负责开关关时不要调用）。
#[must_use]
pub fn snapshot_from_env(observed: usize) -> NervousSnapshot {
    snapshot_from_values(|name| std::env::var(name).ok(), observed)
}

/// 从注入的 env 读取口采集一份快照（纯函数式，测试不碰进程 env）。
#[must_use]
pub fn snapshot_from_values(
    read: impl Fn(&str) -> Option<String>,
    observed: usize,
) -> NervousSnapshot {
    snapshot_from_values_with_bus(read, observed, None)
}

/// 采集一份带总线观测面的快照（v2 additive；`bus = None` 时与 v1 语义一致）。
#[must_use]
pub fn snapshot_from_values_with_bus(
    read: impl Fn(&str) -> Option<String>,
    observed: usize,
    bus: Option<NervousBusStatus>,
) -> NervousSnapshot {
    let mechanisms: Vec<NervousMechanism> = MECHANISMS
        .iter()
        .map(|spec| {
            let enabled = spec
                .enabled_env
                .map_or(false, |name| (spec.parser)(read(name).as_deref()));
            NervousMechanism {
                id: spec.id.to_string(),
                level: spec.level,
                enabled_env: spec.enabled_env.map(str::to_string),
                enabled,
                wiring: spec.wiring.to_string(),
                signal_sources: spec
                    .signals
                    .iter()
                    .map(|signal| SignalSource {
                        id: signal.id.to_string(),
                        state: signal.state,
                        reason: signal.reason.map(str::to_string),
                    })
                    .collect(),
                last_observation: None,
                gaps: spec.gaps.iter().map(|gap| (*gap).to_string()).collect(),
            }
        })
        .collect();
    let summary = summarize(&mechanisms, observed);
    NervousSnapshot {
        version: NERVOUS_SNAPSHOT_VERSION,
        summary,
        mechanisms,
        bus,
    }
}

impl NervousSnapshot {
    /// 投影成 JSON（字段合同唯一实现）。
    #[must_use]
    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }
}

fn summarize(mechanisms: &[NervousMechanism], observed: usize) -> NervousSummary {
    let count = |level: MechanismLevel| {
        mechanisms
            .iter()
            .filter(|mechanism| mechanism.level == level)
            .count()
    };
    NervousSummary {
        total: mechanisms.len(),
        production_wired: count(MechanismLevel::ProductionWired),
        half: count(MechanismLevel::Half),
        unwired: count(MechanismLevel::Implemented),
        observed: observed.min(mechanisms.len()),
    }
}

/// 从一份已渲染的自述帧派生 `capabilities.nervous_system` 子对象。
///
/// 口径（0 装诚实）：
/// - `capabilities.bio_mechanisms` 不在场（观测面未挂）→ `None`（旧帧形状）；
/// - `bio_mechanisms.enabled = false`（开关关）→ `None`，**不读 env**；
/// - 开关开 → `observed` = `mechanisms` 里 `wired: true` 的格数（注册事实），
///   其余字段由 [`snapshot_from_env`] 按既有解析函数现采。
#[must_use]
pub fn nervous_system_from_frame(frame: &Value) -> Option<Value> {
    nervous_system_from_frame_with(frame, |name| std::env::var(name).ok(), None)
}

/// 把总线快照折成 `nervous_system.bus` 子对象（纯函数，不采进程 env）。
#[must_use]
fn bus_status(snapshot: &NervousBusSnapshot) -> NervousBusStatus {
    NervousBusStatus {
        subscribers: snapshot.subscribers.values().copied().sum(),
        published: snapshot.published.values().copied().sum(),
        delivered: snapshot.delivered,
        last_observation: None,
    }
}

fn nervous_system_from_frame_with(
    frame: &Value,
    read: impl Fn(&str) -> Option<String>,
    bus: Option<NervousBusStatus>,
) -> Option<Value> {
    let capabilities = frame.get(CAPABILITIES_SECTION)?.as_object()?;
    let bio = capabilities.get("bio_mechanisms")?;
    if !bio.get("enabled")?.as_bool()? {
        return None;
    }
    let observed = bio
        .get("mechanisms")
        .and_then(Value::as_object)
        .map(|rows| {
            rows.values()
                .filter(|row| row.get("wired").and_then(Value::as_bool) == Some(true))
                .count()
        })
        .unwrap_or(0);
    Some(snapshot_from_values_with_bus(read, observed, bus).to_value())
}

/// `tool.self_status` 只读装饰器：在既有帧的 `capabilities` 节内插入
/// `nervous_system` 子对象；开关关时原样返回（逐字节不变）。
pub struct NervousSelfStatusTool {
    inner: Arc<dyn ToolCapability>,
    env: EnvReader,
    /// 可选的神经总线观测面（schema v2 additive；`None` = 不报 `bus` 子对象）。
    bus: Option<NervousBus>,
}

impl NervousSelfStatusTool {
    /// 包住既有自述工具（生产入口见 [`nervous_self_status_tool`]）。
    #[must_use]
    pub fn new(inner: Arc<dyn ToolCapability>) -> Self {
        Self {
            inner,
            env: Arc::new(|name| std::env::var(name).ok()),
            bus: None,
        }
    }

    /// 注入 env 读取口（测试用；生产走默认进程 env）。
    #[must_use]
    pub fn with_env(mut self, env: EnvReader) -> Self {
        self.env = env;
        self
    }

    /// 绑定总线观测面（R1b-4a：帧内 `capabilities.nervous_system.bus` 计数）。
    #[must_use]
    pub fn with_bus(mut self, bus: NervousBus) -> Self {
        self.bus = Some(bus);
        self
    }

    fn decorate(&self, mut result: ToolResult) -> ToolResult {
        if let ToolOutcome::Ok { value } = &mut result.outcome {
            let bus = self.bus.as_ref().map(|bus| bus_status(&bus.snapshot()));
            if let Some(nervous) =
                nervous_system_from_frame_with(value, |name| (self.env)(name), bus)
            {
                if let Some(capabilities) = value
                    .get_mut(CAPABILITIES_SECTION)
                    .and_then(Value::as_object_mut)
                {
                    capabilities.insert(NERVOUS_SYSTEM_SUBOBJECT.to_string(), nervous);
                }
            }
        }
        result
    }
}

#[async_trait]
impl ToolCapability for NervousSelfStatusTool {
    fn id(&self) -> &CapabilityId {
        self.inner.id()
    }

    fn declaration(&self) -> NormalizedTool {
        self.inner.declaration()
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let result = self.inner.invoke(call).await;
        self.decorate(result)
    }

    async fn invoke_in_session(
        &self,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
        session: Option<SessionId>,
    ) -> ToolResult {
        let result = self.inner.invoke_in_session(call, frozen, session).await;
        self.decorate(result)
    }
}

/// 生产接线口：把既有 `SelfStatusTool` 包成带 `nervous_system` 子对象的工具。
///
/// `tool_modules::SelfStatusModule` 只改这一行构造；开关关时装饰器零动作，
/// 所以旧帧形状逐字节不变（回滚 = 删本模块 + 这一行）。
#[must_use]
pub fn nervous_self_status_tool(source: Arc<dyn SelfStatusSource>) -> Arc<dyn ToolCapability> {
    Arc::new(NervousSelfStatusTool::new(Arc::new(SelfStatusTool::new(
        source,
    ))))
}

/// 生产接线口（R1b-4a）：在 [`nervous_self_status_tool`] 基础上绑定总线观测面，
/// 帧内 `capabilities.nervous_system.bus` 报订阅/发布/投递计数（默认连上装配根
/// 唯一 [`NervousBus`]；开关关时整节缺席，旧帧形状逐字节不变）。
#[must_use]
pub fn nervous_self_status_tool_with_bus(
    source: Arc<dyn SelfStatusSource>,
    bus: NervousBus,
) -> Arc<dyn ToolCapability> {
    Arc::new(NervousSelfStatusTool::new(Arc::new(SelfStatusTool::new(source))).with_bus(bus))
}

// ---------------------------------------------------------------------------
// 14 条机制的只读台账（顺序 = BioMechanismKind::ALL；id 与矩阵 §1.1 生物行一致）
// ---------------------------------------------------------------------------

fn never_enabled(_value: Option<&str>) -> bool {
    false
}

struct SignalSpec {
    id: &'static str,
    state: SignalState,
    reason: Option<&'static str>,
}

struct MechanismSpec {
    id: &'static str,
    level: MechanismLevel,
    enabled_env: Option<&'static str>,
    parser: fn(Option<&str>) -> bool,
    wiring: &'static str,
    signals: &'static [SignalSpec],
    gaps: &'static [&'static str],
}

const MECHANISMS: [MechanismSpec; NERVOUS_MECHANISM_TOTAL] = [
    MechanismSpec {
        id: "SG-13.neuromodulation",
        level: MechanismLevel::ProductionWired,
        enabled_env: Some(BIO_TONE_ENABLE_ENV),
        parser: bio_tone_enabled_from_env_value,
        wiring: "production-assembly::bio_tone",
        signals: &[
            SignalSpec {
                id: "tone_module",
                state: SignalState::Wired,
                reason: None,
            },
            SignalSpec {
                id: "tone_feedback",
                state: SignalState::NotWired,
                reason: Some("真实反馈事件源未接（半程）"),
            },
        ],
        gaps: &["真实反馈事件源未接（调质只有装配门，无运行时反馈闭环）"],
    },
    MechanismSpec {
        id: "SG-14.homeostasis",
        level: MechanismLevel::ProductionWired,
        enabled_env: Some(BIO_CONTEXT_POLICY_ENABLE_ENV),
        parser: bio_switch_enabled_from_env_value,
        wiring: "production-assembly::cognitive.bio_assembly",
        signals: &[
            SignalSpec {
                id: "context_policy",
                state: SignalState::Wired,
                reason: None,
            },
            SignalSpec {
                id: "vital_sign_inputs",
                state: SignalState::Partial,
                reason: Some("仅 ContextPressure 实测，其余维度中性基线"),
            },
        ],
        gaps: &["仅 ContextPressure 实测，其余维度中性基线"],
    },
    MechanismSpec {
        id: "SG-23.circadian",
        level: MechanismLevel::ProductionWired,
        enabled_env: Some(BIO_CIRCADIAN_ENABLE_ENV),
        parser: bio_circadian_enabled_from_env_value,
        wiring: "production-assembly::bio_circadian + care_heartbeat",
        signals: &[
            SignalSpec {
                id: "proactive_gate",
                state: SignalState::Wired,
                reason: None,
            },
            SignalSpec {
                id: "sleep_pressure",
                state: SignalState::NotWired,
                reason: Some("睡眠压力源未接（门内压力恒 0）"),
            },
        ],
        gaps: &["睡眠压力源未接（门内压力恒 0）"],
    },
    MechanismSpec {
        id: "SG-15.immune_memory",
        level: MechanismLevel::ProductionWired,
        enabled_env: Some(IMMUNE_MEMORY_ENABLE_ENV),
        parser: bio_switch_enabled_from_env_value,
        wiring: "cli-assembly::guard hook",
        signals: &[
            SignalSpec {
                id: "guard_observe",
                state: SignalState::Wired,
                reason: None,
            },
            SignalSpec {
                id: "immune_escalation_to_quorum",
                state: SignalState::Wired,
                reason: None,
            },
        ],
        gaps: &[],
    },
    MechanismSpec {
        id: "SG-16.nociception",
        level: MechanismLevel::ProductionWired,
        enabled_env: Some(BIO_PAIN_ENABLE_ENV),
        parser: bio_switch_enabled_from_env_value,
        wiring: "cli-assembly::Runtime::with_bio_pain",
        signals: &[
            SignalSpec {
                id: "pain_ledger",
                state: SignalState::Wired,
                reason: None,
            },
            SignalSpec {
                id: "failure_classification",
                state: SignalState::Partial,
                reason: Some("失败分类为近似"),
            },
        ],
        gaps: &["失败分类为近似"],
    },
    MechanismSpec {
        id: "SG-17.recovery",
        level: MechanismLevel::ProductionWired,
        enabled_env: Some(BIO_PAIN_ENABLE_ENV),
        parser: bio_switch_enabled_from_env_value,
        wiring: "cli-assembly::Runtime::with_bio_pain（与痛觉同开关）",
        signals: &[
            SignalSpec {
                id: "recovery_state_machine",
                state: SignalState::Wired,
                reason: None,
            },
            SignalSpec {
                id: "scar_ledger_learn",
                state: SignalState::NotWired,
                reason: Some("ScarLedger::learn 未接"),
            },
        ],
        gaps: &["ScarLedger::learn 未接"],
    },
    MechanismSpec {
        id: "SG-19.attachment",
        level: MechanismLevel::ProductionWired,
        enabled_env: Some(BIO_CARE_CANDIDATES_ENABLE_ENV),
        parser: bio_care_candidates_enabled_from_env_value,
        wiring: "cli-assembly::care_heartbeat + care candidates",
        signals: &[
            SignalSpec {
                id: "presence_turn",
                state: SignalState::Wired,
                reason: None,
            },
            SignalSpec {
                id: "reunion_candidate",
                state: SignalState::Wired,
                reason: None,
            },
        ],
        gaps: &[],
    },
    MechanismSpec {
        id: "SG-20.social_repair",
        level: MechanismLevel::ProductionWired,
        enabled_env: Some(BIO_REPAIR_GUIDANCE_ENABLE_ENV),
        parser: bio_switch_enabled_from_env_value,
        wiring: "production-assembly::BioAssemblyModule",
        signals: &[
            SignalSpec {
                id: "repair_plan_prompt_block",
                state: SignalState::Wired,
                reason: None,
            },
            SignalSpec {
                id: "session_event_source",
                state: SignalState::NotWired,
                reason: Some("会话事件源未接（仅显式注入 RepairPlan）"),
            },
        ],
        gaps: &["会话事件源未接（仅显式注入 RepairPlan）"],
    },
    MechanismSpec {
        id: "SG-24.quorum",
        level: MechanismLevel::ProductionWired,
        enabled_env: Some(QUORUM_GATE_ENABLE_ENV),
        parser: quorum_gate_enabled_from_env_value,
        wiring: "production-assembly::OrganModule",
        signals: &[
            SignalSpec {
                id: "quorum_gate",
                state: SignalState::Wired,
                reason: None,
            },
            SignalSpec {
                id: "guard_immune_escalation_source",
                state: SignalState::Wired,
                reason: None,
            },
        ],
        gaps: &[],
    },
    MechanismSpec {
        id: "SG-18.lifecycle",
        level: MechanismLevel::ProductionWired,
        enabled_env: Some(BIO_LIFECYCLE_ENABLE_ENV),
        parser: bio_switch_enabled_from_env_value,
        wiring: "production-assembly::bio_production_wiring.install_bio_modules",
        signals: &[SignalSpec {
            id: "tool_lifecycle_ledger",
            state: SignalState::Wired,
            reason: None,
        }],
        gaps: &["self_status.bio_mechanisms 未注册 lifecycle probe（R1b-2 接线）"],
    },
    MechanismSpec {
        id: "SG-21.play",
        level: MechanismLevel::ProductionWired,
        enabled_env: Some(BIO_PLAY_ENABLE_ENV),
        parser: bio_play_enabled_from_env_value,
        wiring: "production-assembly::play_tools（四工具面）",
        signals: &[SignalSpec {
            id: "play_tools",
            state: SignalState::Wired,
            reason: None,
        }],
        gaps: &["无 IM 卡 / 无持久化 / 沙箱非 OS 隔离"],
    },
    MechanismSpec {
        id: "SG-22.ritual",
        level: MechanismLevel::Half,
        enabled_env: Some(BIO_CARE_CANDIDATES_ENABLE_ENV),
        parser: bio_care_candidates_enabled_from_env_value,
        wiring: "production-assembly::install_bio_ritual_tools + care-candidates-pipeline",
        signals: &[
            SignalSpec {
                id: "ritual_state_store",
                state: SignalState::Wired,
                reason: None,
            },
            SignalSpec {
                id: "ritual_producer",
                state: SignalState::Wired,
                reason: None,
            },
        ],
        gaps: &["纯 CLI chat 无心跳宿主 ⇒ 仪式写入暂无候选消费（W3.7 工具面已接，候选侧留 R1b-2）"],
    },
    MechanismSpec {
        id: "SG-25.ecosystem_health",
        level: MechanismLevel::ProductionWired,
        enabled_env: None,
        parser: never_enabled,
        wiring: "production-assembly::install_bio_self_status + tool_ecosystem_probe",
        signals: &[SignalSpec {
            id: "ecosystem_probe",
            state: SignalState::Wired,
            reason: None,
        }],
        gaps: &[
            "策展表为代码内常量快照（未覆盖 tools 如实 untagged）",
            "完整 taxonomy 合同（工具声明自带标签）B 类 ADR 前置",
        ],
    },
    MechanismSpec {
        id: "SG-26.capacity",
        level: MechanismLevel::ProductionWired,
        enabled_env: Some(BIO_SELF_STATUS_ENABLE_ENV),
        parser: bio_self_status_enabled_from_env_value,
        wiring: "production-assembly::self_status_source.capacity_bio_probe",
        signals: &[SignalSpec {
            id: "task_queue_counts",
            state: SignalState::Wired,
            reason: None,
        }],
        gaps: &["bio_mechanisms 观测面最多 8/14 格注册（W3.6 迟绑定 5 + W3.7 生态 1 + 承载力 + 练习）；其余格 wired:false 且各带真因"],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mechanism_rows_cover_the_contract_vocabulary_once() {
        assert_eq!(MECHANISMS.len(), NERVOUS_MECHANISM_TOTAL);
        let mut ids: Vec<&str> = MECHANISMS.iter().map(|spec| spec.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), NERVOUS_MECHANISM_TOTAL, "id 必须唯一");
        let snapshot = snapshot_from_values(|_| None, 0);
        assert_eq!(snapshot.mechanisms.len(), NERVOUS_MECHANISM_TOTAL);
    }

    #[test]
    fn disabled_frame_never_reads_env() {
        let frame = serde_json::json!({
            "capabilities": {
                "bio_mechanisms": { "enabled": false, "mechanisms": null, "reason": "未启用" }
            }
        });
        let reads = std::cell::Cell::new(0usize);
        let value = nervous_system_from_frame_with(
            &frame,
            |_| {
                reads.set(reads.get() + 1);
                Some("1".to_string())
            },
            None,
        );
        assert!(value.is_none());
        assert_eq!(reads.get(), 0, "开关关时一个 env 都不读");
    }

    // R1b-1b 台账可校验（ADR-0009 D5）：三条结构校验。

    /// 校验 ①：每个 `enabled_env` 都必须登记在 env-registry（新增开关先登记）。
    #[test]
    fn every_enabled_env_is_registered_in_the_env_registry() {
        let registry = include_str!("../../../../../docs/architecture/env-registry.toml");
        for spec in &MECHANISMS {
            if let Some(name) = spec.enabled_env {
                assert!(
                    registry.contains(&format!("name = \"{name}\"")),
                    "{} 的开关 {name} 未登记于 docs/architecture/env-registry.toml",
                    spec.id
                );
            }
        }
    }

    /// 校验 ②：PRODUCTION WIRED 必须指向真实装配根且不留 pending 占位；其余级别必带 gaps。
    #[test]
    fn production_wired_rows_name_an_assembly_root_and_never_pending() {
        for spec in &MECHANISMS {
            if spec.level == MechanismLevel::ProductionWired {
                assert!(
                    spec.wiring.starts_with("production-assembly::")
                        || spec.wiring.starts_with("cli-assembly::"),
                    "{} 声称 PRODUCTION WIRED 但 wiring 不是装配根路径: {}",
                    spec.id,
                    spec.wiring
                );
                assert!(
                    !spec.wiring.contains("pending"),
                    "{} 的 wiring 仍含 pending 占位: {}",
                    spec.id,
                    spec.wiring
                );
            } else {
                assert!(
                    !spec.gaps.is_empty(),
                    "{} 非 PRODUCTION WIRED 必须带 gaps，不隐藏未接段",
                    spec.id
                );
            }
        }
    }

    /// 校验 ③：未接/半程信号必带真因；开关解析必须 fail-closed（`None` = 关）。
    #[test]
    fn unwired_signals_carry_reasons_and_parsers_fail_closed() {
        for spec in &MECHANISMS {
            for signal in spec.signals {
                if signal.state != SignalState::Wired {
                    let reason = signal.reason.unwrap_or_default();
                    assert!(
                        !reason.trim().is_empty(),
                        "{} 的 {} 未接/半程但无真因",
                        spec.id,
                        signal.id
                    );
                }
            }
            if spec.enabled_env.is_some() {
                assert!(
                    !(spec.parser)(None),
                    "{} 的开关解析必须 fail-closed（None = 关）",
                    spec.id
                );
                assert!(
                    (spec.parser)(Some("1")),
                    "{} 的开关解析必须认显式 1（否则台账说开而实际永不生效）",
                    spec.id
                );
            }
        }
    }
}
