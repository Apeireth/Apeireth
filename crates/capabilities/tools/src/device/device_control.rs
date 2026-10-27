//! 设备受控执行工具面 (`tool.device_control`, 声明名 `device_control`) ——
//! K 轴 L3 受控执行 / L4 代理代办骨架 (「它没有手, 但它有插头」)。
//!
//! # 三拍执行 (底线④ 先模拟后执行)
//!
//! ```text
//! 调用方            工具面 (本件)             白名单/词表          mock 适配器        状态账
//!   |  plan(设备,动作,参数)  |                      |                   |                |
//!   |---------------------->| 校验 (登记? 启用? 词表? 参数?)          |                |
//!   |      计划帧 plan_id    |                      |                   |                |
//!   |<----------------------|                      |                   |                |
//!   |  preview(plan_id)     |                      |                   |                |
//!   |---------------------->| snapshot(只读) ------->| 只读             |                |
//!   |   将发生什么 (before→after 投影)               |                   |                |
//!   |<----------------------| (零副作用: 不写设备、不落账)             |                |
//!   |  execute(plan_id, 谁批[, im_card_id])          |                   |                |
//!   |---------------------->| 计划绑定核对 + 预览票 + 分级审批证据     |                |
//!   |                       | apply(真执行) ------->| 变更状态          |                |
//!   |                       | append (何时/何设备/何动作/谁批) ------->| append-only    |
//!   |    执行帧 (before→after + 记录 id)             |                   |                |
//!   |<----------------------|                      |                   |                |
//! ```
//!
//! - **plan / preview 零副作用**: 不改设备状态、不写状态账 (预览票是进程内
//!   记忆, 不落盘)。preview 产出"将发生什么" = before → after 的确定性投影;
//! - **execute = 真执行**: 适配器调用 + 状态账追加 (`device-actions.jsonl`
//!   append-only, 逐行一个 [`apeireth_core::stored_doc::StoredDoc`] 信封);
//! - **三拍同计划绑定**: preview / execute 必须带 plan 拍的 `plan_id`, 现算
//!   不符拒绝即帧 (载荷被改即拒); **先模拟后执行**: 没预览过的计划拒绝执行。
//!
//! # 动作三档分级 (底线②③, 词表闭合)
//!
//! | 档 | 词表 | 审批语义 |
//! |---|---|---|
//! | `reversible` | [`REVERSIBLE_ACTIONS`] | 审批档: 默认每次人批, 可配自动档 (只此档可配) |
//! | `irreversible` | [`IRREVERSIBLE_ACTIONS`] (开火/开门/物理删除类) | **永远逐次人批** (无自动档) |
//! | `money` | [`MONEY_ACTIONS`] (购物/下单) | **永远逐次人批 + IM 审批卡** (无自动档) |
//!
//! 词表外动作 = 拒绝即帧 ([`grade_of`] 返回 [`None`] 即拒)。
//!
//! # 白名单 (底线①)
//!
//! 只对 [`DeviceEntry::enabled`] 的设备动作: 未登记设备拒绝即帧、已登记未
//! 启用拒绝即帧; [`DeviceRegistry::new`] 默认零设备, 内置假设备 (灯/空调/
//! 微波炉) 经 [`DeviceRegistry::with_mock_fixtures`] 显式登记启用。设备自身
//! 另有闭合动作白名单 (每设备 `actions`), 未登记动作拒绝即帧。
//!
//! # mock 适配器 (真实协议族后插)
//!
//! [`MockDeviceAdapter`] 内置三件假设备 (灯 / 空调 / 微波炉), 状态变更 =
//! [`project_action`] 的确定性投影 (preview 与 execute 同一投影, "将发生什么"
//! 即"会发生什么")。家居协议族 (本地桥/云桥/直连) 与购买渠道族接入前一律
//! mock 适配器 + 白名单语义先行 (骨架可测、协议后插) —— 真实协议接入
//! **未接线** (0 装诚实: 本批只有 mock 执行面)。
//!
//! # 开关 (双闸 fail-closed)
//!
//! [`DEVICE_CONTROL_ENABLE_ENV`] (`APEIRETH_ENABLE_DEVICE_CONTROL`) 依赖上游
//! [`DEVICE_SENSE_ENABLE_ENV`] (`APEIRETH_ENABLE_DEVICE_SENSE`) **双闸齐开**
//! 才生效; 任一未开 = 面不注册、零设备可动 (见 [`device_control_enabled_from_env`])。
//!
//! # 状态账 (append-only)
//!
//! `device-actions.jsonl`: 何时 (`at_ms`) / 何设备 / 何动作 / 谁批 (`approved_by`,
//! 自动档如实记 [`AUTO_PASS_APPROVER`]) + 分级 + 计划 id + before/after +
//! money 档的 IM 卡 id。追加写 + 落盘, 不重写、不删行; 坏行读作不存在。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::CapabilityId;
use apeireth_core::stored_doc::{scan_records, DocCompat, RecordScan, StoredDoc};
use apeireth_governance::{
    Action, Decision, GovernanceHook, GovernanceRequest, Permission, PermissionPolicy, ToolGuard,
    ToolGuardRequest,
};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::exec_pipeline::{
    AroundPolicy, OutputSchema, PreExecuteHook, PreExecuteRequest, PreVerdict, SchemaKind,
    DEFAULT_MAX_TIMEOUT_MS,
};

// ---------------------------------------------------------------------------
// 身份与开关 (注册即闸, 双闸 fail-closed)
// ---------------------------------------------------------------------------

/// 稳定能力标识 (注册表 / 治理授权 / 风险映射共用)。
pub const DEVICE_CONTROL_CAPABILITY_ID: &str = "tool.device_control";

/// 模型侧声明名 (短名, 每次请求都进上下文)。
pub const DEVICE_CONTROL_TOOL_NAME: &str = "device_control";

/// 生产执行时限 (毫秒): mock 适配器 + 账本追加都是本地即时工作。
pub const DEVICE_CONTROL_TIMEOUT_MS: u64 = 10_000;

/// 上游感知闸 (`APEIRETH_ENABLE_DEVICE_SENSE`, K 轴 L1 感知面的开关):
/// 受控执行的依赖闸 —— 感知面没开, 执行面不起 (双闸语义)。
pub const DEVICE_SENSE_ENABLE_ENV: &str = "APEIRETH_ENABLE_DEVICE_SENSE";

/// 受控执行总开关 (`APEIRETH_ENABLE_DEVICE_CONTROL`): 注册即闸, 默认关
/// fail-closed, 且依赖 [`DEVICE_SENSE_ENABLE_ENV`] 双闸齐开。
pub const DEVICE_CONTROL_ENABLE_ENV: &str = "APEIRETH_ENABLE_DEVICE_CONTROL";

/// reversible 档「自动放行」子档 (依赖 [`DEVICE_CONTROL_ENABLE_ENV`]):
/// **只对 `reversible` 档生效** —— irreversible / money 两档永远逐次人批,
/// 本开关怎么开都绕不过 (分级矩阵测试钉死)。
pub const DEVICE_AUTO_PASS_REVERSIBLE_ENV: &str = "APEIRETH_ENABLE_DEVICE_AUTO_PASS_REVERSIBLE";

/// 自动档放行时「谁批」的如实标签 (账本 / 执行帧同文, 不冒充人名)。
pub const AUTO_PASS_APPROVER: &str = "auto-pass-reversible";

/// 双闸开关实况: `APEIRETH_ENABLE_DEVICE_SENSE=1` **且**
/// `APEIRETH_ENABLE_DEVICE_CONTROL=1` 才 `true` (只认显式 `"1"`, 前后空白容忍)。
///
/// 其余任何取值 (未设 / `"0"` / `"true"` / 杂值) = 关 (fail-closed, 命名即承诺:
/// 没有这两把钥匙就是没有设备受控执行面)。
pub fn device_control_enabled_from_env() -> bool {
    env_is_one(DEVICE_SENSE_ENABLE_ENV) && env_is_one(DEVICE_CONTROL_ENABLE_ENV)
}

/// env 取值是否显式 `"1"` (前后空白容忍)。
fn env_is_one(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// reversible 自动档子开关实况 (只认显式 `"1"`; **只作用于 reversible 档**,
/// irreversible / money 两档永远逐次人批, 本开关怎么开都绕不过)。
pub fn device_control_auto_pass_reversible_from_env() -> bool {
    env_is_one(DEVICE_AUTO_PASS_REVERSIBLE_ENV)
}

/// 设备受控执行面配置 (状态账落位 + reversible 自动档)。
///
/// 组装根按 [`DeviceControlConfig::from_env`] 注入 (双闸注册即闸): 双闸未开 =
/// `tool.device_control` 不注册, 自述名册 `device_control` 行照实 false。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceControlConfig {
    /// 数据目录 (状态账 `device-actions.jsonl` 落位)。
    pub data_dir: PathBuf,
    /// reversible 档自动放行子档 (默认 false = 审批档; 只作用于 reversible)。
    pub auto_pass_reversible: bool,
}

impl DeviceControlConfig {
    /// 数据目录 + 默认审批档 (reversible 也每次人批)。
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
            auto_pass_reversible: false,
        }
    }

    /// 覆盖 reversible 自动档 (命名即承诺: 值即生效权力, 只此档可配)。
    pub fn with_auto_pass_reversible(mut self, on: bool) -> Self {
        self.auto_pass_reversible = on;
        self
    }

    /// 开关实况 (双闸 fail-closed)。
    pub fn enabled_from_env() -> bool {
        device_control_enabled_from_env()
    }

    /// 从 env 构造: 双闸齐开才 `Some` (默认关 → [`None`], fail-closed);
    /// 自动档随 [`DEVICE_AUTO_PASS_REVERSIBLE_ENV`] 取值。
    pub fn from_env(data_dir: impl Into<PathBuf>) -> Option<Self> {
        if !device_control_enabled_from_env() {
            return None;
        }
        Some(
            Self::new(data_dir)
                .with_auto_pass_reversible(env_is_one(DEVICE_AUTO_PASS_REVERSIBLE_ENV)),
        )
    }
}

// ---------------------------------------------------------------------------
// 错误 (拒绝即帧: 真因如实, 不静默降级)
// ---------------------------------------------------------------------------

/// 设备受控执行失败 (面向模型的稳定帧文本由 [`DeviceControlError::message`] 给出)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceControlError {
    /// 参数校验不过 (缺字段 / 越界取值 / 词表外取值 / 多余字段)。
    InvalidParam(String),
    /// 动作不在闭合分级词表 (词表外拒绝即帧)。
    UnknownAction(String),
    /// 设备白名单里没登记这个动作 (每设备动作词表闭合)。
    UnsupportedAction {
        /// 设备 id。
        device: String,
        /// 请求动作。
        action: String,
    },
    /// 设备未登记 (设备白名单: 未登记即拒, 默认零设备)。
    UnknownDevice(String),
    /// 设备已登记但未启用 (enabled 显式授权才可动)。
    DeviceDisabled(String),
    /// 三拍同计划绑定被改即拒 (请求 plan_id ≠ 现算 plan_id)。
    PlanMismatch {
        /// 请求携带的计划 id。
        expected: String,
        /// 现算计划 id。
        actual: String,
    },
    /// 先模拟后执行: 计划未预览, 拒绝执行。
    PreviewRequired(String),
    /// 需要人批 (分级审批证据缺位)。
    ApprovalRequired(String),
    /// 动钱动作缺 IM 审批卡 (money 档: 永远逐次人批 + IM 卡)。
    ImCardRequired,
    /// 设备白名单读取失败 (设备表拒开/坏表: fail-closed, 不静默回退空表)。
    Whitelist(String),
    /// 适配器失败 (读/写设备状态, 真因在描述里)。
    Adapter(String),
    /// 状态账写入/读取失败。
    Ledger(String),
}

impl DeviceControlError {
    /// 面向模型的拒绝帧 (稳定前缀 + 真因)。
    pub fn message(&self) -> String {
        match self {
            Self::InvalidParam(detail) => format!("拒绝设备动作: {detail}"),
            Self::UnknownAction(action) => format!(
                "拒绝设备动作: 动作 {action:?} 不在闭合分级词表 (reversible/irreversible/money), \
                 词表外拒绝即帧"
            ),
            Self::UnsupportedAction { device, action } => format!(
                "拒绝设备动作: 设备 {device} 的动作白名单里没有 {action:?} (每设备动作词表闭合, \
                 未登记动作拒绝即帧)"
            ),
            Self::UnknownDevice(device) => {
                format!("拒绝设备动作: 设备 {device} 未登记 (设备白名单: 未登记即拒, 默认零设备)")
            }
            Self::DeviceDisabled(device) => {
                format!("拒绝设备动作: 设备 {device} 已登记但未启用 (enabled 显式授权才可动)")
            }
            Self::PlanMismatch { expected, actual } => format!(
                "拒绝设备动作: 三拍同计划绑定不符 (载荷被改即拒): 请求 plan_id {expected} ≠ \
                 现算 plan_id {actual}"
            ),
            Self::PreviewRequired(plan_id) => {
                format!("拒绝设备动作: 先模拟后执行 —— 计划 {plan_id} 未预览, 不执行 (底线④)")
            }
            Self::ApprovalRequired(detail) => format!("拒绝设备动作: 需要人批 —— {detail}"),
            Self::ImCardRequired => format!(
                "拒绝设备动作: 动钱动作缺 IM 审批卡 (money 档永远逐次人批 + IM 卡, 无自动档)"
            ),
            Self::Whitelist(detail) => format!(
                "拒绝设备动作: 设备白名单读取失败 —— {detail} (fail-closed, 不静默回退空表)"
            ),
            Self::Adapter(detail) => format!("设备适配器失败: {detail}"),
            Self::Ledger(detail) => format!("设备动作账失败: {detail}"),
        }
    }
}

// ---------------------------------------------------------------------------
// 动作三档分级 (词表闭合, 底线②③)
// ---------------------------------------------------------------------------

/// 动作风险三档 (reversible / irreversible / money)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionGrade {
    /// 可逆档: 审批档 (默认每次人批, 可配自动档 —— 只此档可配)。
    Reversible,
    /// 不可逆档 (开火/开门/物理删除类): **永远逐次人批** (无自动档)。
    Irreversible,
    /// 动钱档 (购物/下单): **永远逐次人批 + IM 审批卡** (无自动档)。
    Money,
}

impl ActionGrade {
    /// 稳定档标签 (`reversible` / `irreversible` / `money`)。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Reversible => "reversible",
            Self::Irreversible => "irreversible",
            Self::Money => "money",
        }
    }

    /// 人读档名 (审批卡载荷同文)。
    pub const fn label(self) -> &'static str {
        match self {
            Self::Reversible => "可逆",
            Self::Irreversible => "不可逆",
            Self::Money => "动钱",
        }
    }

    /// 分级审批语义 (自动档只作用于 reversible, 其余两档无自动档)。
    pub const fn approval_requirement(self, auto_pass_reversible: bool) -> ApprovalRequirement {
        match self {
            Self::Reversible => ApprovalRequirement::Approval {
                auto_allowed: auto_pass_reversible,
            },
            Self::Irreversible => ApprovalRequirement::PerCallHuman,
            Self::Money => ApprovalRequirement::PerCallHumanImCard,
        }
    }
}

/// reversible 档闭合词表 (8 词: 开关/亮度/温度/模式/定时/启停)。
pub const REVERSIBLE_ACTIONS: &[&str] = &[
    "power_on",
    "power_off",
    "set_brightness",
    "set_temperature",
    "set_mode",
    "set_timer",
    "start",
    "stop",
];

/// irreversible 档闭合词表 (3 词: 开火/开门/物理删除类, 永远逐次人批)。
pub const IRREVERSIBLE_ACTIONS: &[&str] = &["ignite", "unlock_door", "physical_delete"];

/// money 档闭合词表 (2 词: 购物/下单, 动钱词表闭合, 永远逐次人批 + IM 卡)。
pub const MONEY_ACTIONS: &[&str] = &["purchase", "place_order"];

/// 动作 → 分级 (闭合词表判定; 词表外返回 [`None`] = 拒绝即帧)。
pub fn grade_of(action: &str) -> Option<ActionGrade> {
    if REVERSIBLE_ACTIONS.contains(&action) {
        Some(ActionGrade::Reversible)
    } else if IRREVERSIBLE_ACTIONS.contains(&action) {
        Some(ActionGrade::Irreversible)
    } else if MONEY_ACTIONS.contains(&action) {
        Some(ActionGrade::Money)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// 分级审批要求 + 审批证据
// ---------------------------------------------------------------------------

/// 一次动作的审批要求 (由分级推出, 构造后不可越权放宽)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalRequirement {
    /// 审批档 (reversible): 默认每次人批; 自动档可配 (`auto_allowed`)。
    Approval {
        /// 自动档是否放行 (只有 reversible 可为 true)。
        auto_allowed: bool,
    },
    /// 永远逐次人批 (irreversible, 无自动档)。
    PerCallHuman,
    /// 永远逐次人批 + IM 审批卡 (money, 无自动档)。
    PerCallHumanImCard,
}

impl ApprovalRequirement {
    /// 稳定档标签 (执行帧 / 账本 / 审批卡同文)。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Approval { auto_allowed: true } => "approval_auto_allowed",
            Self::Approval {
                auto_allowed: false,
            } => "approval",
            Self::PerCallHuman => "per_call_human",
            Self::PerCallHumanImCard => "per_call_human_im_card",
        }
    }

    /// 是否要求 IM 审批卡 (money 档恒 true)。
    pub const fn requires_im_card(self) -> bool {
        matches!(self, Self::PerCallHumanImCard)
    }

    /// 结构化审批要求 (计划帧 / 审批卡载荷同源)。
    pub fn to_json(self) -> Value {
        match self {
            Self::Approval { auto_allowed } => json!({
                "tier": "approval",
                "auto_allowed": auto_allowed,
                "im_card_required": false,
            }),
            Self::PerCallHuman => json!({
                "tier": "per_call_human",
                "auto_allowed": false,
                "im_card_required": false,
            }),
            Self::PerCallHumanImCard => json!({
                "tier": "per_call_human_im_card",
                "auto_allowed": false,
                "im_card_required": true,
            }),
        }
    }
}

/// 执行拍的审批证据 (谁拍的板 + money 档的 IM 卡标识)。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalEvidence {
    /// 谁批 (人名 / IM 卡回调标识; 自动档放行时可缺席, 如实记
    /// [`AUTO_PASS_APPROVER`])。
    pub approved_by: Option<String>,
    /// IM 审批卡标识 (money 档必填, 其余档可空)。
    pub im_card_id: Option<String>,
}

/// 非空裁剪 (空白视同缺位)。
fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|text| !text.is_empty())
}

// ---------------------------------------------------------------------------
// 设备白名单 (底线①: 默认零设备, 未登记/未启用拒绝即帧)
// ---------------------------------------------------------------------------

/// 白名单里的一个设备 (显式登记 + 显式启用才可动)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceEntry {
    /// 设备稳定 id (白名单键)。
    pub id: String,
    /// 设备人读名 (账本 / 审批卡同文)。
    pub name: String,
    /// 设备类别 (light / air_conditioner / microwave / ...; mock 投影按动作词表, 不按类别)。
    pub kind: String,
    /// 是否启用 (enabled 显式授权才可动; 默认 [`false`])。
    pub enabled: bool,
    /// 该设备的动作白名单 (闭合; 词表外动作拒绝即帧)。
    pub actions: Vec<String>,
}

/// 执行面设备白名单合同 (底线① 的单一语义: 登记 + 启用才可动)。
///
/// 两个只读消费口: [`DeviceRegistry`] (进程内注册表, mock fixture / 测试) 与
/// [`RegistryFileWhitelist`] (K 轴 L1 感知批的 `devices.json` 设备表**只读
/// 消费桥** —— 白名单复用: 感知面与执行面同一本设备表, 各自不另立白名单)。
pub trait DeviceWhitelist: Send + Sync {
    /// 授权判定 (未登记 → [`DeviceControlError::UnknownDevice`];
    /// 已登记未启用 → [`DeviceControlError::DeviceDisabled`])。
    fn authorize(&self, device_id: &str) -> Result<DeviceEntry, DeviceControlError>;

    /// 白名单计数 `(登记总数, 启用数)` (自述节真值; 读表失败照实报错不编数)。
    fn counts(&self) -> Result<(u64, u64), DeviceControlError>;
}

/// 每类别动作词表 (闭合; 从 L1 设备类别推 L3 可执行面)。
///
/// 现值 (保守, fail-closed): `appliance` (家电类) = reversible 家电面;
/// `sensor` / `camera` / `other` = 只读感知面, **零执行动作**。irreversible /
/// money 两档的动作不挂任何类别 —— 它们只随**显式逐设备登记**
/// ([`DeviceRegistry::register`] 的 `actions`) 挂接 (mock/测试harness 与未来
/// L4 渠道族注册面); L4 动钱/不可逆面的 kind 扩表归口 L4 渠道接入批。
pub fn actions_for_kind(kind: &str) -> &'static [&'static str] {
    match kind {
        "appliance" => REVERSIBLE_ACTIONS,
        _ => &[],
    }
}

/// 设备白名单合同的进程内实现 (mock fixture / 测试 harness 用)。
///
/// **默认零设备** (底线①): [`DeviceRegistry::new`] 不登记任何设备;
/// mock 内置假设备经 [`DeviceRegistry::with_mock_fixtures`] 显式登记启用。
#[derive(Debug, Default)]
pub struct DeviceRegistry {
    entries: Mutex<BTreeMap<String, DeviceEntry>>,
}

impl DeviceRegistry {
    /// 空注册表 (默认零设备)。
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// mock 内置假设备注册表: 灯 / 空调 / 微波炉三件**显式登记并启用**
    /// (仅 mock 假设备, 无真实设备面; 真实协议接入后置)。
    pub fn with_mock_fixtures() -> Arc<Self> {
        let registry = Self::new();
        for entry in mock_fixture_entries() {
            registry
                .register(entry)
                .expect("mock fixture ids are unique");
        }
        registry
    }

    /// 登记设备 (id 重复拒绝即帧)。
    pub fn register(&self, entry: DeviceEntry) -> Result<(), DeviceControlError> {
        let mut entries = self.lock();
        if entries.contains_key(&entry.id) {
            return Err(DeviceControlError::InvalidParam(format!(
                "设备 {} 已登记 (白名单不重复登记)",
                entry.id
            )));
        }
        entries.insert(entry.id.clone(), entry);
        Ok(())
    }

    /// 启用/停用设备 (未登记拒绝即帧)。
    pub fn set_enabled(&self, device_id: &str, enabled: bool) -> Result<(), DeviceControlError> {
        let mut entries = self.lock();
        let entry = entries
            .get_mut(device_id)
            .ok_or_else(|| DeviceControlError::UnknownDevice(device_id.to_string()))?;
        entry.enabled = enabled;
        Ok(())
    }

    /// 授权判定 (未登记 → [`DeviceControlError::UnknownDevice`];
    /// 已登记未启用 → [`DeviceControlError::DeviceDisabled`])。
    pub fn authorize(&self, device_id: &str) -> Result<DeviceEntry, DeviceControlError> {
        let entries = self.lock();
        let entry = entries
            .get(device_id)
            .ok_or_else(|| DeviceControlError::UnknownDevice(device_id.to_string()))?;
        if !entry.enabled {
            return Err(DeviceControlError::DeviceDisabled(device_id.to_string()));
        }
        Ok(entry.clone())
    }

    /// 全量清单 (自述节 / 列举用)。
    pub fn list(&self) -> Vec<DeviceEntry> {
        self.lock().values().cloned().collect()
    }

    /// 计数 (总登记数, 启用数)。
    pub fn counts(&self) -> (u64, u64) {
        let entries = self.lock();
        let total = entries.len() as u64;
        let enabled = entries.values().filter(|entry| entry.enabled).count() as u64;
        (total, enabled)
    }

    /// 锁定注册表 (内部互斥)。
    fn lock(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, DeviceEntry>> {
        self.entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl DeviceWhitelist for DeviceRegistry {
    fn authorize(&self, device_id: &str) -> Result<DeviceEntry, DeviceControlError> {
        DeviceRegistry::authorize(self, device_id)
    }

    fn counts(&self) -> Result<(u64, u64), DeviceControlError> {
        Ok(DeviceRegistry::counts(self))
    }
}

/// K 轴 L1 设备表 (`devices.json`) 的**只读消费桥** (白名单复用)。
///
/// 只读不写: 每次判定现读 [`crate::device::registry::DeviceRegistry`]
/// (StoredDoc 严格打开), 表拒开 / 坏表 = [`DeviceControlError::Whitelist`]
/// 拒绝即帧 (fail-closed, 不静默回退空表也不静默放行)。设备动作词表按
/// [`actions_for_kind`] 从 L1 类别推导 (闭合、保守)。
#[derive(Debug, Clone)]
pub struct RegistryFileWhitelist {
    registry: crate::device::registry::DeviceRegistry,
}

impl RegistryFileWhitelist {
    /// 数据目录落点 (`<data_dir>/devices.json`, 与 L1 感知面同一本设备表)。
    pub fn under_data_dir(data_dir: impl AsRef<Path>) -> Self {
        Self {
            registry: crate::device::registry::DeviceRegistry::under_data_dir(data_dir),
        }
    }

    /// 显式路径落点。
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self {
            registry: crate::device::registry::DeviceRegistry::at(path),
        }
    }

    /// 现读设备表 (StoredDoc 严格打开; 拒开即帧)。
    fn load(&self) -> Result<crate::device::registry::DevicesBody, DeviceControlError> {
        self.registry
            .load_strict()
            .map_err(|error| DeviceControlError::Whitelist(error.to_string()))
    }
}

impl DeviceWhitelist for RegistryFileWhitelist {
    fn authorize(&self, device_id: &str) -> Result<DeviceEntry, DeviceControlError> {
        let body = self.load()?;
        let device = body
            .find(device_id)
            .ok_or_else(|| DeviceControlError::UnknownDevice(device_id.to_string()))?;
        if !device.enabled {
            return Err(DeviceControlError::DeviceDisabled(device_id.to_string()));
        }
        Ok(DeviceEntry {
            id: device.id.clone(),
            name: device.name.clone(),
            kind: device.kind.as_str().to_string(),
            enabled: device.enabled,
            actions: actions_for_kind(device.kind.as_str())
                .iter()
                .map(|action| (*action).to_string())
                .collect(),
        })
    }

    fn counts(&self) -> Result<(u64, u64), DeviceControlError> {
        let body = self.load()?;
        let total = body.len() as u64;
        let enabled = body.whitelist().len() as u64;
        Ok((total, enabled))
    }
}

/// mock 内置假设备三件 (灯 / 空调 / 微波炉, 显式登记并启用)。
pub fn mock_fixture_entries() -> Vec<DeviceEntry> {
    vec![
        DeviceEntry {
            id: "lamp".to_string(),
            name: "灯".to_string(),
            kind: "light".to_string(),
            enabled: true,
            actions: vec![
                "power_on".to_string(),
                "power_off".to_string(),
                "set_brightness".to_string(),
            ],
        },
        DeviceEntry {
            id: "air_conditioner".to_string(),
            name: "空调".to_string(),
            kind: "air_conditioner".to_string(),
            enabled: true,
            actions: vec![
                "power_on".to_string(),
                "power_off".to_string(),
                "set_temperature".to_string(),
                "set_mode".to_string(),
            ],
        },
        DeviceEntry {
            id: "microwave".to_string(),
            name: "微波炉".to_string(),
            kind: "microwave".to_string(),
            enabled: true,
            actions: vec![
                "power_on".to_string(),
                "power_off".to_string(),
                "set_timer".to_string(),
                "start".to_string(),
                "stop".to_string(),
            ],
        },
    ]
}

// ---------------------------------------------------------------------------
// 设备状态 + 适配器 (mock 先行, 真实协议族后插)
// ---------------------------------------------------------------------------

/// 设备状态 (键值快照; mock 与真实适配器同一形状)。
pub type DeviceState = BTreeMap<String, Value>;

/// 设备适配器 (执行面唯一出口; mock 实现见 [`MockDeviceAdapter`])。
///
/// 真实协议族 (本地桥/云桥/直连) 落位时实现同一 trait 后插, 三拍语义不变。
pub trait DeviceAdapter: Send + Sync {
    /// 读设备状态快照 (只读)。
    fn snapshot(&self, device_id: &str) -> Result<DeviceState, DeviceControlError>;

    /// 应用动作 (真执行, 变更设备状态), 返回变更后状态。
    fn apply(
        &self,
        device_id: &str,
        action: &str,
        params: &Value,
    ) -> Result<DeviceState, DeviceControlError>;
}

/// mock 设备适配器 (内置假设备的进程内状态账; 零真实设备)。
#[derive(Debug, Default)]
pub struct MockDeviceAdapter {
    states: Mutex<BTreeMap<String, DeviceState>>,
}

impl MockDeviceAdapter {
    /// 空 mock 适配器 (设备须显式 [`MockDeviceAdapter::ensure_device`])。
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// 内置三件假设备 (灯 / 空调 / 微波炉) 的 mock 适配器。
    pub fn with_builtin_fixtures() -> Arc<Self> {
        let adapter = Self::new();
        for (id, state) in mock_fixture_initial_states() {
            adapter.ensure_device(id, state);
        }
        adapter
    }

    /// 登记/覆盖一台假设备的初始状态。
    pub fn ensure_device(&self, device_id: &str, initial: DeviceState) {
        self.lock().insert(device_id.to_string(), initial);
    }

    /// 读现值 (只读; 测试/自述用)。
    pub fn state_of(&self, device_id: &str) -> Option<DeviceState> {
        self.lock().get(device_id).cloned()
    }

    /// 锁定状态表 (内部互斥)。
    fn lock(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, DeviceState>> {
        self.states
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl DeviceAdapter for MockDeviceAdapter {
    fn snapshot(&self, device_id: &str) -> Result<DeviceState, DeviceControlError> {
        self.lock()
            .get(device_id)
            .cloned()
            .ok_or_else(|| DeviceControlError::Adapter(format!("mock 设备 {device_id} 无状态账")))
    }

    fn apply(
        &self,
        device_id: &str,
        action: &str,
        params: &Value,
    ) -> Result<DeviceState, DeviceControlError> {
        let mut states = self.lock();
        let before = states.get(device_id).cloned().ok_or_else(|| {
            DeviceControlError::Adapter(format!("mock 设备 {device_id} 无状态账"))
        })?;
        let after = project_action(&before, action, params)?;
        states.insert(device_id.to_string(), after.clone());
        Ok(after)
    }
}

/// mock 内置假设备初始状态 (灯 / 空调 / 微波炉)。
pub fn mock_fixture_initial_states() -> Vec<(&'static str, DeviceState)> {
    let mut lamp: DeviceState = BTreeMap::new();
    lamp.insert("power".to_string(), json!("off"));
    lamp.insert("brightness".to_string(), json!(0));
    let mut ac: DeviceState = BTreeMap::new();
    ac.insert("power".to_string(), json!("off"));
    ac.insert("mode".to_string(), json!("cool"));
    ac.insert("temperature".to_string(), json!(26));
    let mut microwave: DeviceState = BTreeMap::new();
    microwave.insert("power".to_string(), json!("off"));
    microwave.insert("timer_seconds".to_string(), json!(0));
    microwave.insert("running".to_string(), json!("off"));
    vec![
        ("lamp", lamp),
        ("air_conditioner", ac),
        ("microwave", microwave),
    ]
}

// ---------------------------------------------------------------------------
// 参数校验 + 动作投影 (纯函数: preview 与 execute 同一投影)
// ---------------------------------------------------------------------------

/// 参数取对象视图 (缺参/null = 空对象; 非对象拒绝即帧)。
fn params_map(params: &Value) -> Result<Map<String, Value>, DeviceControlError> {
    match params {
        Value::Null => Ok(Map::new()),
        Value::Object(map) => Ok(map.clone()),
        other => Err(DeviceControlError::InvalidParam(format!(
            "params 必须是 JSON 对象 (收到 {other})"
        ))),
    }
}

/// 键集闭合校验 (多余字段 / 缺字段都拒绝即帧)。
fn expect_keys(
    map: &Map<String, Value>,
    expected: &[&str],
    action: &str,
) -> Result<(), DeviceControlError> {
    let mut actual: Vec<&str> = map.keys().map(String::as_str).collect();
    actual.sort_unstable();
    let mut wanted: Vec<&str> = expected.to_vec();
    wanted.sort_unstable();
    if actual != wanted {
        return Err(DeviceControlError::InvalidParam(format!(
            "{action} 的参数键集必须恰为 {wanted:?} (收到 {actual:?}, 多余/缺失都拒)"
        )));
    }
    Ok(())
}

/// 取整数参数 (i64, 范围闭合)。
fn require_int(
    map: &Map<String, Value>,
    key: &str,
    action: &str,
    min: i64,
    max: i64,
) -> Result<i64, DeviceControlError> {
    let value = map
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| DeviceControlError::InvalidParam(format!("{action}.{key} 必须是整数")))?;
    if (min..=max).contains(&value) {
        Ok(value)
    } else {
        Err(DeviceControlError::InvalidParam(format!(
            "{action}.{key} 必须在 [{min}, {max}] (收到 {value})"
        )))
    }
}

/// 取正数金额参数 (f64, > 0)。
fn require_positive_number(
    map: &Map<String, Value>,
    key: &str,
    action: &str,
) -> Result<f64, DeviceControlError> {
    let value = map
        .get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| DeviceControlError::InvalidParam(format!("{action}.{key} 必须是数字")))?;
    if value > 0.0 {
        Ok(value)
    } else {
        Err(DeviceControlError::InvalidParam(format!(
            "{action}.{key} 必须 > 0 (收到 {value})"
        )))
    }
}

/// 取非空字符串参数。
fn require_nonempty_str(
    map: &Map<String, Value>,
    key: &str,
    action: &str,
) -> Result<String, DeviceControlError> {
    let value = map
        .get(key)
        .and_then(Value::as_str)
        .and_then(|text| non_empty(Some(text)))
        .ok_or_else(|| {
            DeviceControlError::InvalidParam(format!("{action}.{key} 必须是非空字符串"))
        })?;
    Ok(value.to_string())
}

/// 取闭合词表内字符串参数。
fn require_str_in(
    map: &Map<String, Value>,
    key: &str,
    action: &str,
    allowed: &[&str],
) -> Result<String, DeviceControlError> {
    let value = require_nonempty_str(map, key, action)?;
    if allowed.contains(&value.as_str()) {
        Ok(value)
    } else {
        Err(DeviceControlError::InvalidParam(format!(
            "{action}.{key} 必须在 {allowed:?} (收到 {value:?})"
        )))
    }
}

/// 动作参数校验 (逐动作键集闭合 + 取值范围闭合; 纯判定, 零副作用)。
pub fn validate_action_params(action: &str, params: &Value) -> Result<(), DeviceControlError> {
    let map = params_map(params)?;
    match action {
        "power_on" | "power_off" | "start" | "stop" | "ignite" | "unlock_door"
        | "physical_delete" => expect_keys(&map, &[], action),
        "set_brightness" => {
            expect_keys(&map, &["level"], action)?;
            require_int(&map, "level", action, 0, 100).map(|_| ())
        }
        "set_temperature" => {
            expect_keys(&map, &["celsius"], action)?;
            require_int(&map, "celsius", action, 16, 30).map(|_| ())
        }
        "set_mode" => {
            expect_keys(&map, &["mode"], action)?;
            require_str_in(&map, "mode", action, &["cool", "heat", "fan"]).map(|_| ())
        }
        "set_timer" => {
            expect_keys(&map, &["seconds"], action)?;
            require_int(&map, "seconds", action, 1, 3600).map(|_| ())
        }
        "purchase" => {
            expect_keys(&map, &["item", "amount", "currency"], action)?;
            require_nonempty_str(&map, "item", action)?;
            require_positive_number(&map, "amount", action)?;
            require_nonempty_str(&map, "currency", action).map(|_| ())
        }
        "place_order" => {
            expect_keys(&map, &["item", "quantity", "amount", "currency"], action)?;
            require_nonempty_str(&map, "item", action)?;
            require_int(&map, "quantity", action, 1, 99)?;
            require_positive_number(&map, "amount", action)?;
            require_nonempty_str(&map, "currency", action).map(|_| ())
        }
        other => Err(DeviceControlError::UnknownAction(other.to_string())),
    }
}

/// 动作状态投影 (确定性: preview 的"将发生什么" = execute 的"会发生什么";
/// 纯函数, 不触碰设备)。
pub fn project_action(
    before: &DeviceState,
    action: &str,
    params: &Value,
) -> Result<DeviceState, DeviceControlError> {
    validate_action_params(action, params)?;
    let map = params_map(params)?;
    let mut after = before.clone();
    let mut set = |key: &str, value: Value| {
        after.insert(key.to_string(), value);
    };
    match action {
        "power_on" => set("power", json!("on")),
        "power_off" => set("power", json!("off")),
        "start" => set("running", json!("on")),
        "stop" => set("running", json!("off")),
        "set_brightness" => set("brightness", map["level"].clone()),
        "set_temperature" => set("temperature", map["celsius"].clone()),
        "set_mode" => set("mode", map["mode"].clone()),
        "set_timer" => set("timer_seconds", map["seconds"].clone()),
        "ignite" => set("flame", json!("on")),
        "unlock_door" => set("door", json!("unlocked")),
        "physical_delete" => set("content", json!("deleted")),
        "purchase" => {
            set("last_purchase", map["item"].clone());
            set("charged_amount", map["amount"].clone());
        }
        "place_order" => {
            set("last_order", map["item"].clone());
            set("order_quantity", map["quantity"].clone());
            set("charged_amount", map["amount"].clone());
        }
        other => return Err(DeviceControlError::UnknownAction(other.to_string())),
    }
    Ok(after)
}

/// 状态差异行 ("key: 旧 → 新", 新增键旧值按 `-`)。
fn diff_effects(before: &DeviceState, after: &DeviceState) -> Vec<String> {
    let mut keys: BTreeSet<&String> = before.keys().collect();
    keys.extend(after.keys());
    keys.into_iter()
        .filter(|key| before.get(*key) != after.get(*key))
        .map(|key| {
            let old = before
                .get(key)
                .map(value_label)
                .unwrap_or_else(|| "-".to_string());
            let new = after
                .get(key)
                .map(value_label)
                .unwrap_or_else(|| "-".to_string());
            format!("{key}: {old} → {new}")
        })
        .collect()
}

/// 值标签 (字符串裸出, 其余走 JSON)。
fn value_label(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// JSON 规范化字符串 (键排序递归; plan_id 身份摘要与 serde 特性无关)。
fn canonical_json(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(text) => serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string()),
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().map(canonical_json).collect();
            format!("[{}]", parts.join(","))
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let parts: Vec<String> = keys
                .into_iter()
                .map(|key| {
                    let name = serde_json::to_string(key).unwrap_or_else(|_| "\"\"".to_string());
                    format!("{name}:{}", canonical_json(&map[key]))
                })
                .collect();
            format!("{{{}}}", parts.join(","))
        }
    }
}

// ---------------------------------------------------------------------------
// 三拍: 计划帧 / 预览帧 / 执行帧
// ---------------------------------------------------------------------------

/// 计划帧 (plan 拍产物: 设备+动作+参数 → 执行计划; 零副作用)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionPlan {
    /// 计划稳定身份 (三拍同计划绑定键; FNV-1a 64 hex, 非密码学)。
    pub plan_id: String,
    /// 设备 id。
    pub device: String,
    /// 设备人读名。
    pub device_name: String,
    /// 动作 (闭合词表内)。
    pub device_action: String,
    /// 分级 (reversible / irreversible / money)。
    pub grade: ActionGrade,
    /// 动作参数 (键集闭合)。
    pub params: Value,
    /// 分级审批要求 (构造后不可越权放宽)。
    pub approval: ApprovalRequirement,
}

/// 计划身份摘要 (设备|动作|分级|参数的规范串 → FNV-1a 64 hex, **非密码学**)。
///
/// 供审批卡 / 账本 / 三拍绑定做「同一计划」配对标识: 声明变则摘要必变。
pub fn plan_id_for(
    device: &str,
    device_action: &str,
    grade: ActionGrade,
    params: &Value,
) -> String {
    let canonical = format!(
        "{device}\n{device_action}\n{}\n{}",
        grade.as_str(),
        canonical_json(params)
    );
    format!(
        "{:016x}",
        crate::worktree_plan::fnv1a64(canonical.as_bytes())
    )
}

/// 预览帧 (preview 拍产物: "将发生什么" = before → after 投影; 零副作用)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewFrame {
    /// 计划稳定身份 (与 plan 拍同值)。
    pub plan_id: String,
    /// 设备 id。
    pub device: String,
    /// 设备人读名。
    pub device_name: String,
    /// 动作。
    pub device_action: String,
    /// 分级。
    pub grade: ActionGrade,
    /// 动作参数 (与 plan 拍同值; 审批卡载荷从这里取真参数, 不藏参数)。
    /// `#[serde(default)]`: 旧帧载荷无此列照读 (存量 = 空参数)。
    #[serde(default)]
    pub params: Value,
    /// 执行前状态快照 (只读)。
    pub before: DeviceState,
    /// 执行后状态投影 (未执行, 只是"将发生什么")。
    pub after: DeviceState,
    /// 人读变化行 ("key: 旧 → 新")。
    pub effects: Vec<String>,
}

/// 执行帧 (execute 拍产物: 真执行 + 状态账追加)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecuteFrame {
    /// 计划稳定身份。
    pub plan_id: String,
    /// 状态账记录 id (与账本行同值)。
    pub record_id: String,
    /// 设备 id。
    pub device: String,
    /// 动作。
    pub device_action: String,
    /// 分级。
    pub grade: ActionGrade,
    /// 执行前状态。
    pub before: DeviceState,
    /// 执行后状态 (真变更)。
    pub after: DeviceState,
    /// 谁批 (自动档如实记 [`AUTO_PASS_APPROVER`])。
    pub approved_by: String,
    /// IM 审批卡标识 (money 档必在)。
    pub im_card_id: Option<String>,
}

// ---------------------------------------------------------------------------
// 状态账 (device-actions.jsonl, append-only)
// ---------------------------------------------------------------------------

/// 状态账文件 (数据目录落点: `<data_dir>/device-actions.jsonl`)。
pub const DEVICE_ACTION_LEDGER_FILE: &str = "device-actions.jsonl";

/// 状态账 StoredDoc 的身份戳。
pub const DEVICE_ACTION_DOC_NAME: &str = "device-actions-ledger";

/// 状态账 StoredDoc 的格式版本戳。
pub const DEVICE_ACTION_DOC_VERSION: u32 = 1;

/// 状态账的一行 (append-only 的行主体: 何时/何设备/何动作/谁批)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceActionRecord {
    /// 记录稳定身份 (`dev-<时刻毫秒>-<进程内序号>`)。
    pub id: String,
    /// 执行时刻 (epoch 毫秒)。
    pub at_ms: i64,
    /// 计划 id (三拍同计划绑定)。
    pub plan_id: String,
    /// 设备 id。
    pub device: String,
    /// 设备人读名。
    pub device_name: String,
    /// 动作。
    pub device_action: String,
    /// 分级标签。
    pub grade: String,
    /// 动作参数。
    pub params: Value,
    /// 执行前状态。
    pub before: DeviceState,
    /// 执行后状态。
    pub after: DeviceState,
    /// 谁批 (自动档如实记 [`AUTO_PASS_APPROVER`])。
    pub approved_by: String,
    /// IM 审批卡标识 (money 档必在, 其余档 [`None`])。
    pub im_card_id: Option<String>,
}

/// 记录身份 (`dev-<时刻毫秒>-<进程内序号>`)。
pub fn device_action_record_id(at_ms: i64, ordinal: u64) -> String {
    format!("dev-{at_ms}-{ordinal}")
}

/// 状态账 (`<data_dir>/device-actions.jsonl`)。
///
/// **append-only**: 逐行一个 [`StoredDoc`] 信封, 追加写 + 落盘, 不重写、不删行;
/// 坏行读作不存在 (跳过并计入 `skipped`), 不砖化整本。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceActionLedger {
    path: PathBuf,
}

impl DeviceActionLedger {
    /// 数据目录落点 (`<data_dir>/device-actions.jsonl`)。
    pub fn under_data_dir(data_dir: impl AsRef<Path>) -> Self {
        Self {
            path: data_dir.as_ref().join(DEVICE_ACTION_LEDGER_FILE),
        }
    }

    /// 显式路径落点。
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// 账本路径。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 追加一行 (JSONL 单行 + 落盘; append-only, 不重写不删行)。
    pub fn append(&self, record: &DeviceActionRecord) -> Result<(), DeviceControlError> {
        use std::io::Write;
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|error| {
                    DeviceControlError::Ledger(format!("状态账目录创建失败 {parent:?}: {error}"))
                })?;
            }
        }
        let doc = StoredDoc::new(
            DEVICE_ACTION_DOC_NAME,
            DEVICE_ACTION_DOC_VERSION,
            vec![DEVICE_ACTION_DOC_VERSION],
            record,
        );
        // ⚠️ **2026-10-27 修同族缺陷**：原写法**两次写**（行体 + 换行）
        // ⇒ 并发写者会把两行挤成一行 ⇒ **状态记录丢失**。
        // ⇒ 改用**共用原语**（行体与行尾换行同在一次写）。
        crate::proposal::append_jsonl_row(&self.path, "状态账", &doc)
            .map_err(|error| DeviceControlError::Ledger(error))
    }

    /// 读回整本 (逐记录语义: 坏行读作不存在并计入 `skipped`)。
    pub fn scan(&self) -> Result<RecordScan<DeviceActionRecord>, DeviceControlError> {
        scan_records::<DeviceActionRecord>(&self.path, &ledger_compat()).map_err(|error| {
            DeviceControlError::Ledger(format!("状态账读取失败 {}: {error}", self.path.display()))
        })
    }
}

/// 账本读端契约 (仅当前格式版本可读)。
fn ledger_compat() -> DocCompat {
    DocCompat::exact(DEVICE_ACTION_DOC_NAME, DEVICE_ACTION_DOC_VERSION)
}

/// 当前时刻 (epoch 毫秒); 时钟回拨不编造, 如实取系统读数。
fn now_epoch_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// 审批卡载荷 (本地审批面板 / IM 审批卡同源)
// ---------------------------------------------------------------------------

/// 设备动作审批卡载荷 (IM 审批卡渲染面消费同一份载荷)。
///
/// 三拍纪律在卡面同文: 预览效果 = "将发生什么" (先模拟后执行);
/// money 档 [`DeviceApprovalCard::requires_im_card`] 恒 true (动钱 = IM 卡闭环)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceApprovalCard {
    /// 计划 id (与三拍绑定同值)。
    pub plan_id: String,
    /// 设备 id。
    pub device: String,
    /// 设备人读名。
    pub device_name: String,
    /// 动作。
    pub device_action: String,
    /// 分级标签。
    pub grade: String,
    /// 动作参数。
    pub params: Value,
    /// 预览效果行 ("key: 旧 → 新")。
    pub effects: Vec<String>,
    /// 是否要求 IM 审批卡 (money 档恒 true)。
    pub requires_im_card: bool,
    /// 分级审批语义说明 (人读)。
    pub approval_note: String,
}

impl DeviceApprovalCard {
    /// 从预览帧载荷成审批卡 (预览效果原样进卡, 动作参数随卡不藏 —— 卡载荷自足,
    /// IM 推送链按卡即可重建执行计划)。
    pub fn from_preview(preview: &PreviewFrame) -> Self {
        let grade = preview.grade;
        Self {
            plan_id: preview.plan_id.clone(),
            device: preview.device.clone(),
            device_name: preview.device_name.clone(),
            device_action: preview.device_action.clone(),
            grade: grade.as_str().to_string(),
            params: preview.params.clone(),
            effects: preview.effects.clone(),
            requires_im_card: grade.approval_requirement(false).requires_im_card(),
            approval_note: approval_note_for(grade).to_string(),
        }
    }

    /// 渲染呈现文本 (本地审批面板 / IM 审批卡共用的卡面正文)。
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str("[设备动作待审] ");
        out.push_str(&self.device_name);
        out.push_str(" · ");
        out.push_str(&self.device_action);
        out.push('\n');
        out.push_str(&format!(
            "设备: {} | 分级: {} | 计划: {}\n",
            self.device, self.grade, self.plan_id
        ));
        out.push_str("将发生什么 (先模拟后执行):\n");
        for (index, line) in self.effects.iter().enumerate() {
            out.push_str(&format!("{}. {line}\n", index + 1));
        }
        out.push_str(&self.approval_note);
        out.push('\n');
        if self.requires_im_card {
            out.push_str("动钱动作: 需要 IM 审批卡回执 (im_card_id) 才执行。\n");
        }
        out
    }

    /// 结构化卡载荷 (含分级风险标注 + 批准/拒绝两按钮, 与 IM 审批卡同构)。
    pub fn to_json(&self) -> Value {
        json!({
            "plan_id": self.plan_id,
            "device": self.device,
            "device_name": self.device_name,
            "device_action": self.device_action,
            "grade": self.grade,
            "params": self.params,
            "effects": self.effects,
            "im_card": self.requires_im_card,
            "requires_im_card": self.requires_im_card,
            "approval_note": self.approval_note,
            "risk": grade_risk_label(&self.grade),
            "buttons": ["approve", "reject"],
        })
    }
}

/// 分级审批语义说明 (审批卡 / 拒绝帧同文)。
pub fn approval_note_for(grade: ActionGrade) -> &'static str {
    match grade {
        ActionGrade::Reversible => "可逆动作 (审批档): 默认每次执行停人工审批; 自动档只此档可配。",
        ActionGrade::Irreversible => "不可逆动作: 永远逐次人批 (无自动档, 底线③)。",
        ActionGrade::Money => "动钱动作: 永远逐次人批 + IM 审批卡 (无自动档, 底线②)。",
    }
}

/// 分级风险标注 (审批卡载荷同文; IM 审批卡信封的 `risk_level` 同源取值)。
pub fn grade_risk_label(grade: &str) -> &'static str {
    match grade {
        "reversible" => "medium",
        "irreversible" | "money" => "high",
        _ => "high",
    }
}

// ---------------------------------------------------------------------------
// 三拍执行核心 (plan / preview / execute + 状态账)
// ---------------------------------------------------------------------------

/// 设备受控执行核心 (三拍 + 白名单 + 分级审批 + 状态账)。
///
/// **plan / preview 零副作用**: 不改设备、不落账 (预览票是进程内记忆);
/// **execute 真执行**: 适配器调用 + 状态账 append-only 追加。
pub struct DeviceControlRunner {
    config: DeviceControlConfig,
    whitelist: Arc<dyn DeviceWhitelist>,
    adapter: Arc<dyn DeviceAdapter>,
    ledger: DeviceActionLedger,
    /// 预览票 (进程内): plan_id → 已预览 (先模拟后执行的凭证, 不落盘)。
    previewed: Mutex<BTreeSet<String>>,
    /// 进程内记录序号 (身份唯一性用, 非安全边界)。
    ordinal: AtomicU64,
}

impl DeviceControlRunner {
    /// 组装执行核心 (白名单 + 适配器 + 状态账落位)。
    pub fn new(
        config: DeviceControlConfig,
        whitelist: Arc<dyn DeviceWhitelist>,
        adapter: Arc<dyn DeviceAdapter>,
    ) -> Self {
        let ledger = DeviceActionLedger::under_data_dir(&config.data_dir);
        Self {
            config,
            whitelist,
            adapter,
            ledger,
            previewed: Mutex::new(BTreeSet::new()),
            ordinal: AtomicU64::new(1),
        }
    }

    /// mock 组装口: 进程内 fixture 白名单 (灯/空调/微波炉) + mock 适配器 +
    /// 状态账落位 (零真实设备)。
    pub fn mock(config: DeviceControlConfig) -> Arc<Self> {
        Arc::new(Self::new(
            config,
            DeviceRegistry::with_mock_fixtures(),
            MockDeviceAdapter::with_builtin_fixtures(),
        ))
    }

    /// mock 执行面 + **L1 设备表只读消费** (白名单复用): 执行走 mock 适配器,
    /// 白名单判定现读 `devices.json` (与感知面同一本设备表)。
    pub fn mock_with_file_whitelist(
        config: DeviceControlConfig,
        devices_file: impl Into<PathBuf>,
    ) -> Arc<Self> {
        Arc::new(Self::new(
            config,
            Arc::new(RegistryFileWhitelist::at(devices_file.into())),
            MockDeviceAdapter::with_builtin_fixtures(),
        ))
    }

    /// 白名单 (只读消费口)。
    pub fn whitelist(&self) -> &Arc<dyn DeviceWhitelist> {
        &self.whitelist
    }

    /// 状态账落点 (只读消费口)。
    pub fn ledger(&self) -> &DeviceActionLedger {
        &self.ledger
    }

    /// **plan 拍** (零副作用): 白名单 + 分级词表 + 参数闭合校验 → 计划帧。
    pub fn plan(
        &self,
        device: &str,
        device_action: &str,
        params: &Value,
    ) -> Result<ExecutionPlan, DeviceControlError> {
        let grade = grade_of(device_action)
            .ok_or_else(|| DeviceControlError::UnknownAction(device_action.to_string()))?;
        let entry = self.whitelist.authorize(device)?;
        if !entry.actions.iter().any(|action| action == device_action) {
            return Err(DeviceControlError::UnsupportedAction {
                device: device.to_string(),
                action: device_action.to_string(),
            });
        }
        validate_action_params(device_action, params)?;
        let params = match params {
            Value::Null => json!({}),
            other => other.clone(),
        };
        Ok(ExecutionPlan {
            plan_id: plan_id_for(device, device_action, grade, &params),
            device: device.to_string(),
            device_name: entry.name,
            device_action: device_action.to_string(),
            grade,
            params,
            approval: grade.approval_requirement(self.config.auto_pass_reversible),
        })
    }

    /// **preview 拍** (零副作用): before → after 确定性投影, "将发生什么";
    /// 预览票只进进程内记忆 (不落盘、不改设备、不落账)。
    pub fn preview(
        &self,
        device: &str,
        device_action: &str,
        params: &Value,
        expected_plan_id: Option<&str>,
    ) -> Result<PreviewFrame, DeviceControlError> {
        let plan = self.plan(device, device_action, params)?;
        if let Some(expected) = expected_plan_id {
            if expected != plan.plan_id {
                return Err(DeviceControlError::PlanMismatch {
                    expected: expected.to_string(),
                    actual: plan.plan_id.clone(),
                });
            }
        }
        let before = self.adapter.snapshot(device)?;
        let after = project_action(&before, device_action, &plan.params)?;
        let effects = diff_effects(&before, &after);
        self.previewed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(plan.plan_id.clone());
        Ok(PreviewFrame {
            plan_id: plan.plan_id,
            device: device.to_string(),
            device_name: plan.device_name,
            device_action: device_action.to_string(),
            grade: plan.grade,
            params: plan.params,
            before,
            after,
            effects,
        })
    }

    /// **execute 拍** (真执行): 三拍同计划绑定 + 先模拟后执行 + 分级审批
    /// 证据 → 适配器调用 + 状态账 append-only 追加。
    pub fn execute(
        &self,
        device: &str,
        device_action: &str,
        params: &Value,
        expected_plan_id: Option<&str>,
        evidence: &ApprovalEvidence,
    ) -> Result<ExecuteFrame, DeviceControlError> {
        let plan = self.plan(device, device_action, params)?;
        if let Some(expected) = expected_plan_id {
            if expected != plan.plan_id {
                return Err(DeviceControlError::PlanMismatch {
                    expected: expected.to_string(),
                    actual: plan.plan_id.clone(),
                });
            }
        }
        // 分级审批证据先校验 (拒绝零副作用: 缺人批/缺 IM 卡不烧预览票)。
        let approved_by = self.approval_evidence(&plan, evidence)?;
        // 先模拟后执行 (底线④): 预览票一次性, 过证据校验后核销 (逐次语义)。
        let previewed = self
            .previewed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&plan.plan_id);
        if !previewed {
            return Err(DeviceControlError::PreviewRequired(plan.plan_id));
        }
        let before = self.adapter.snapshot(device)?;
        let after = self.adapter.apply(device, device_action, &plan.params)?;
        let at_ms = now_epoch_ms();
        let record = DeviceActionRecord {
            id: device_action_record_id(at_ms, self.ordinal.fetch_add(1, Ordering::Relaxed)),
            at_ms,
            plan_id: plan.plan_id.clone(),
            device: device.to_string(),
            device_name: plan.device_name.clone(),
            device_action: device_action.to_string(),
            grade: plan.grade.as_str().to_string(),
            params: plan.params.clone(),
            before: before.clone(),
            after: after.clone(),
            approved_by: approved_by.clone(),
            im_card_id: evidence.im_card_id.clone(),
        };
        self.ledger.append(&record)?;
        Ok(ExecuteFrame {
            plan_id: plan.plan_id,
            record_id: record.id,
            device: device.to_string(),
            device_action: device_action.to_string(),
            grade: plan.grade,
            before,
            after,
            approved_by,
            im_card_id: evidence.im_card_id.clone(),
        })
    }

    /// 分级审批证据校验 (返回「谁批」; 自动档放行如实记 [`AUTO_PASS_APPROVER`])。
    fn approval_evidence(
        &self,
        plan: &ExecutionPlan,
        evidence: &ApprovalEvidence,
    ) -> Result<String, DeviceControlError> {
        let approved_by = non_empty(evidence.approved_by.as_deref());
        let note = approval_note_for(plan.grade);
        match plan.approval {
            ApprovalRequirement::Approval { auto_allowed: true } => {
                Ok(approved_by.unwrap_or(AUTO_PASS_APPROVER).to_string())
            }
            ApprovalRequirement::Approval {
                auto_allowed: false,
            } => approved_by
                .map(str::to_string)
                .ok_or_else(|| DeviceControlError::ApprovalRequired(note.to_string())),
            ApprovalRequirement::PerCallHuman => approved_by.map(str::to_string).ok_or_else(|| {
                DeviceControlError::ApprovalRequired(format!("{note} 自动档不适用, 逐次人批"))
            }),
            ApprovalRequirement::PerCallHumanImCard => {
                let who = approved_by.map(str::to_string).ok_or_else(|| {
                    DeviceControlError::ApprovalRequired(format!("{note} 自动档不适用, 逐次人批"))
                })?;
                if non_empty(evidence.im_card_id.as_deref()).is_none() {
                    return Err(DeviceControlError::ImCardRequired);
                }
                Ok(who)
            }
        }
    }

    /// 自述节现探测 (设备计数 / 动作计数 / 最近动作, 真值如实)。
    pub fn status(&self) -> Result<DeviceControlStatus, String> {
        let (devices_total, devices_enabled) = self
            .whitelist
            .counts()
            .map_err(|error| format!("自述设备节读白名单失败: {}", error.message()))?;
        let scan = self
            .ledger
            .scan()
            .map_err(|error| format!("自述设备节读状态账失败: {}", error.message()))?;
        let mut status = DeviceControlStatus {
            devices_total,
            devices_enabled,
            actions_executed: 0,
            executed_reversible: 0,
            executed_irreversible: 0,
            executed_money: 0,
            last_action: None,
            note: None,
            reason: None,
        };
        if scan.skipped > 0 {
            status.note = Some(format!(
                "状态账 {} 行坏行读作不存在 (如实跳过, 不砖化整本)",
                scan.skipped
            ));
        }
        for record in &scan.records {
            status.actions_executed += 1;
            match record.body.grade.as_str() {
                "reversible" => status.executed_reversible += 1,
                "irreversible" => status.executed_irreversible += 1,
                "money" => status.executed_money += 1,
                other => {
                    return Err(format!("状态账分级标签越界: {other} (词表闭合, 不编数)"));
                }
            }
            status.last_action = Some(format!(
                "{} / {} @ {} (谁批: {})",
                record.body.device,
                record.body.device_action,
                record.body.at_ms,
                record.body.approved_by
            ));
        }
        if status.actions_executed == 0 && status.note.is_none() {
            status.note = Some("尚无动作记录".to_string());
        }
        Ok(status)
    }
}

/// 自述「设备受控执行」节 (与工具面同账同源, 真值如实)。
///
/// 只回计数与最近动作标签, 不回参数正文 —— 与自述面「有多少不说是什么」同口径。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceControlStatus {
    /// 白名单登记设备总数。
    pub devices_total: u64,
    /// 白名单启用设备数 (enabled 显式授权)。
    pub devices_enabled: u64,
    /// 已执行动作总数 (状态账行数)。
    pub actions_executed: u64,
    /// 已执行 reversible 档计数。
    pub executed_reversible: u64,
    /// 已执行 irreversible 档计数。
    pub executed_irreversible: u64,
    /// 已执行 money 档计数。
    pub executed_money: u64,
    /// 最近动作标签 (`设备 / 动作 @ 时刻 (谁批: ...)`)。
    pub last_action: Option<String>,
    /// 如实备注 (坏行跳过 / 尚无记录)。
    pub note: Option<String>,
    /// 缺位原因 (本面现探测恒 [`None`]; 缺探针时由上层补)。
    pub reason: Option<String>,
}

impl DeviceControlStatus {
    /// 节体 JSON (自述面 `device_control` 节的载荷)。
    pub fn to_json(&self) -> Value {
        json!({
            "devices_total": self.devices_total,
            "devices_enabled": self.devices_enabled,
            "actions_executed": self.actions_executed,
            "executed_reversible": self.executed_reversible,
            "executed_irreversible": self.executed_irreversible,
            "executed_money": self.executed_money,
            "last_action": self.last_action,
            "note": self.note,
            "reason": self.reason,
        })
    }
}

// ---------------------------------------------------------------------------
// 治理 (分级审批档 + 风险映射 + 单调边界 Guard)
// ---------------------------------------------------------------------------

/// 分级审批档钩子 (治理链, 与本地审批面板 / IM 审批卡同链)。
///
/// - `plan` / `preview` = 零副作用读面 → 放行;
/// - `execute` 按分级: reversible = 审批档 (可配自动档) / irreversible =
///   **永远逐次人批** / money = **永远逐次人批 + IM 卡**;
/// - 词表外动作 / 未知拍 fail-closed 拒绝即帧 ([`Decision::deny`])。
#[derive(Debug, Clone, Copy, Default)]
pub struct DeviceControlApprovalHook {
    /// reversible 自动档 (只此档可配; 默认 false = 审批档)。
    auto_pass_reversible: bool,
}

impl DeviceControlApprovalHook {
    /// 默认档构造 (reversible 也每次人批)。
    pub fn new() -> Self {
        Self::default()
    }

    /// 覆盖 reversible 自动档 (irreversible / money 不受影响, 永逐次人批)。
    pub fn with_auto_pass_reversible(mut self, on: bool) -> Self {
        self.auto_pass_reversible = on;
        self
    }
}

#[async_trait]
impl GovernanceHook for DeviceControlApprovalHook {
    fn name(&self) -> &str {
        "device_control_action_grades"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        let Action::CapabilityDispatch {
            capability,
            arguments,
        } = &request.action
        else {
            return Decision::Allow;
        };
        if capability.as_str() != DEVICE_CONTROL_CAPABILITY_ID {
            return Decision::Allow;
        }
        let verb = arguments
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or_default();
        match verb {
            // plan / preview 零副作用读面: 零审批 (三拍的前两拍不改世界)。
            "plan" | "preview" => Decision::Allow,
            "execute" => {
                let device_action = arguments
                    .get("device_action")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                match grade_of(device_action) {
                    None => Decision::deny(format!(
                        "设备动作 {device_action:?} 不在闭合分级词表: 拒绝即帧 (词表外无工具面)"
                    )),
                    Some(ActionGrade::Reversible) if self.auto_pass_reversible => Decision::Allow,
                    Some(grade) => Decision::require_approval(format!(
                        "设备动作 {device_action:?} ({} 档): {}",
                        grade.label(),
                        approval_note_for(grade)
                    )),
                }
            }
            other => Decision::deny(format!(
                "设备控制拍 {other:?} 不在闭合拍词表 (plan/preview/execute): fail-closed 拒绝即帧"
            )),
        }
    }
}

/// 分级风险映射 (pre-execute 瀑布): 未显式授权 = require-approval 档。
///
/// 与文档/图像工具风险映射同一语义: 显式授权 (grant 且未标审批) → 放行;
/// 授权被标人工审批 / 无授权 → `ask` (拒绝即帧 `pipeline.pre_ask`)。
pub struct DeviceControlRiskMappingHook {
    policy: Arc<Mutex<PermissionPolicy>>,
}

impl DeviceControlRiskMappingHook {
    /// 挂在共享授权策略上的风险映射钩子。
    pub fn new(policy: Arc<Mutex<PermissionPolicy>>) -> Self {
        Self { policy }
    }
}

impl PreExecuteHook for DeviceControlRiskMappingHook {
    fn name(&self) -> &str {
        "device_control_risk_mapping"
    }

    fn pre_verdict(&self, request: &PreExecuteRequest<'_>) -> PreVerdict {
        let decision = self
            .policy
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .decision_for_capability(request.capability.as_str());
        match decision {
            Decision::Allow => PreVerdict::Allow,
            Decision::RequireApproval { reason } => PreVerdict::Ask { reason },
            Decision::Deny { .. } => PreVerdict::Ask {
                reason: format!(
                    "device control tool {} sits at the controlled execution level: \
                     no explicit authorization for it",
                    request.capability.as_str()
                ),
            },
        }
    }
}

/// 单调边界 Guard (只拒不放): 白名单 / 词表 / 参数闭合。
///
/// 自动放行档放行的调用也照穿本 Guard —— 自动放行从不绕过硬边界; 任何后续
/// 阶段都无法把这里的拒绝翻回放行 (单调性)。
pub struct DeviceControlBoundaryGuard {
    whitelist: Arc<dyn DeviceWhitelist>,
}

impl DeviceControlBoundaryGuard {
    /// 绑定设备白名单的边界 Guard。
    pub fn new(whitelist: Arc<dyn DeviceWhitelist>) -> Self {
        Self { whitelist }
    }

    /// 硬边界判定 (纯校验, 与 plan 拍同一真源): 返回 `Some(拒绝帧)` 即拒绝。
    pub fn refusal(&self, arguments: &Value) -> Option<String> {
        let params: DeviceControlParams = serde_json::from_value(arguments.clone()).ok()?;
        // 只判执行面硬边界 (词表/白名单/参数); 拍词表归 [`DeviceControlApprovalHook`]。
        match grade_of(&params.device_action) {
            None => {
                return Some(
                    DeviceControlError::UnknownAction(params.device_action.clone()).message(),
                )
            }
            Some(_) => {}
        }
        let entry = match self.whitelist.authorize(&params.device) {
            Ok(entry) => entry,
            Err(error) => return Some(error.message()),
        };
        if !entry
            .actions
            .iter()
            .any(|action| *action == params.device_action)
        {
            return Some(
                DeviceControlError::UnsupportedAction {
                    device: params.device.clone(),
                    action: params.device_action.clone(),
                }
                .message(),
            );
        }
        let plan_params = if params.params.is_null() {
            json!({})
        } else {
            params.params.clone()
        };
        match validate_action_params(&params.device_action, &plan_params) {
            Ok(()) => None,
            Err(error) => Some(error.message()),
        }
    }
}

impl ToolGuard for DeviceControlBoundaryGuard {
    fn name(&self) -> &str {
        "device_control_boundary"
    }

    fn deny(&self, request: &ToolGuardRequest<'_>) -> Option<String> {
        if request.capability.as_str() != DEVICE_CONTROL_CAPABILITY_ID {
            return None;
        }
        self.refusal(request.arguments)
    }
}

/// 生产授权播种: 已注册的设备控制面即获显式授权 (授权 = 「这次调用有人拍板」)。
///
/// 分级风险档位不在工具策略里 —— 由 [`DeviceControlApprovalHook`] 唯一承载,
/// 与本地审批面板 / IM 审批卡同链。
pub fn authorized_device_control_policy() -> Arc<Mutex<PermissionPolicy>> {
    let policy = Arc::new(Mutex::new(PermissionPolicy::new()));
    policy
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .grant(Permission::ExecuteTool(
            DEVICE_CONTROL_CAPABILITY_ID.to_string(),
        ));
    policy
}

/// 组装设备控制五段流水线: 风险映射 → 单调边界 Guard → 超时 → 输出归一合同。
pub fn device_control_pipeline(
    whitelist: Arc<dyn DeviceWhitelist>,
) -> Arc<crate::exec_pipeline::ToolExecutionPipeline> {
    Arc::new(
        crate::exec_pipeline::ToolExecutionPipeline::new()
            .with_pre_hook(Arc::new(DeviceControlRiskMappingHook::new(
                authorized_device_control_policy(),
            )))
            .with_guard(Arc::new(DeviceControlBoundaryGuard::new(whitelist)))
            .with_around(
                AroundPolicy::new().with_timeout(DEVICE_CONTROL_TIMEOUT_MS, DEFAULT_MAX_TIMEOUT_MS),
            )
            .with_output_schema(DeviceControlTool::output_schema()),
    )
}

// ---------------------------------------------------------------------------
// 工具面 (模型侧一个能力)
// ---------------------------------------------------------------------------

/// 拍闭合词表: plan。
pub const ACTION_PLAN: &str = "plan";
/// 拍闭合词表: preview。
pub const ACTION_PREVIEW: &str = "preview";
/// 拍闭合词表: execute。
pub const ACTION_EXECUTE: &str = "execute";

/// 拍闭合词表 (3 拍)。
pub const DEVICE_CONTROL_BEATS: &[&str] = &[ACTION_PLAN, ACTION_PREVIEW, ACTION_EXECUTE];

/// 工具调用参数 (拍 + 设备 + 动作 + 参数 + 三拍绑定/审批证据)。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceControlParams {
    /// 拍 (plan / preview / execute)。
    pub action: String,
    /// 设备 id (白名单键)。
    pub device: String,
    /// 设备动作 (闭合分级词表内)。
    pub device_action: String,
    /// 动作参数 (键集闭合; 缺省 = 空参数)。
    #[serde(default)]
    pub params: Value,
    /// 计划 id (preview / execute 必带, 三拍同计划绑定)。
    pub plan_id: Option<String>,
    /// 谁批 (execute 的审批证据)。
    pub approved_by: Option<String>,
    /// IM 审批卡标识 (money 档 execute 必带)。
    pub im_card_id: Option<String>,
}

/// 工具描述全文 (注册声明与文档引用同一份文本)。
pub const DEVICE_CONTROL_TOOL_DESCRIPTION: &str = "Control whitelisted devices \
  through three beats: plan (device + action + parameters -> an execution plan \
  with a stable plan id and its approval tier), preview (read-only projection of \
  what will happen, before -> after, zero side effects), and execute (the real \
  adapter call plus an append-only action ledger). Actions are graded in a closed \
  vocabulary: reversible (approval tier, optional auto-pass), irreversible \
  (ignition / door unlock / physical delete words - every call needs a human \
  approval) and money (purchase / order words - every call needs a human approval \
  plus an IM approval card). Only registered and enabled devices may be touched; \
  unlisted devices, unlisted actions and off-vocabulary actions are refused with \
  an explicit reason. preview and execute must carry the plan id from the plan \
  beat, and execute refuses a plan that was never previewed.";

/// 设备受控执行工具 (`tool.device_control`, 声明名 `device_control`)。
pub struct DeviceControlTool {
    id: CapabilityId,
    runner: Arc<DeviceControlRunner>,
}

impl DeviceControlTool {
    /// 绑定三拍执行核心。
    pub fn new(runner: Arc<DeviceControlRunner>) -> Self {
        Self {
            id: CapabilityId::new(DEVICE_CONTROL_CAPABILITY_ID).expect("valid capability id"),
            runner,
        }
    }

    /// 执行核心 (测试直调口, 与工具面同一真源)。
    pub fn runner(&self) -> &Arc<DeviceControlRunner> {
        &self.runner
    }

    /// 输出归一合同: 结构化输出必须是对象且 `action` / `status` 恒在场。
    pub fn output_schema() -> OutputSchema {
        OutputSchema::new()
            .require("action", SchemaKind::String)
            .require("status", SchemaKind::String)
    }

    /// 拒绝即帧 (真因如实, 不静默降级)。
    fn refuse(&self, call: &ToolCall, error: DeviceControlError) -> ToolResult {
        ToolResult::permanent_error(&call.id, error.message()).with_name(DEVICE_CONTROL_TOOL_NAME)
    }
}

#[async_trait]
impl ToolCapability for DeviceControlTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let mut parameters = ToolParameters::new();
        parameters.extend(
            json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": DEVICE_CONTROL_BEATS,
                        "description": "Closed beat vocabulary: plan / preview / execute (three beats, simulate before you act)."
                    },
                    "device": {
                        "type": "string",
                        "description": "Device id from the whitelist (registered and enabled devices only)."
                    },
                    "device_action": {
                        "type": "string",
                        "description": "Closed graded action vocabulary (reversible / irreversible / money words); off-vocabulary actions are refused."
                    },
                    "params": {
                        "type": "object",
                        "description": "Action parameters (closed key set per action; omit for no-parameter actions)."
                    },
                    "plan_id": {
                        "type": "string",
                        "description": "preview / execute only: the plan id returned by the plan beat (payload drift is refused)."
                    },
                    "approved_by": {
                        "type": "string",
                        "description": "execute only: who approved this call (human name or IM card callback id)."
                    },
                    "im_card_id": {
                        "type": "string",
                        "description": "execute on money-grade actions only: the IM approval card id (required for purchase / order words)."
                    }
                },
                "required": ["action", "device", "device_action"],
                "additionalProperties": false
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        NormalizedTool::new(DEVICE_CONTROL_TOOL_NAME)
            .with_description(DEVICE_CONTROL_TOOL_DESCRIPTION.to_string())
            .with_parameters(parameters)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let arguments = match &call.arguments {
            Value::Null => json!({}),
            other => other.clone(),
        };
        let params = match serde_json::from_value::<DeviceControlParams>(arguments) {
            Ok(params) => params,
            Err(error) => {
                return self.refuse(
                    call,
                    DeviceControlError::InvalidParam(format!(
                        "invalid device_control parameters: {error}"
                    )),
                );
            }
        };
        let action_params = if params.params.is_null() {
            json!({})
        } else {
            params.params.clone()
        };
        match params.action.as_str() {
            ACTION_PLAN => {
                match self
                    .runner
                    .plan(&params.device, &params.device_action, &action_params)
                {
                    Ok(plan) => ToolResult::ok(
                        &call.id,
                        json!({
                            "action": ACTION_PLAN,
                            "status": "planned",
                            "plan_id": plan.plan_id,
                            "device": plan.device,
                            "device_name": plan.device_name,
                            "device_action": plan.device_action,
                            "grade": plan.grade.as_str(),
                            "params": plan.params,
                            "approval": plan.approval.to_json(),
                            "approval_note": approval_note_for(plan.grade),
                            "zero_side_effect": true,
                        }),
                    )
                    .with_name(DEVICE_CONTROL_TOOL_NAME),
                    Err(error) => self.refuse(call, error),
                }
            }
            ACTION_PREVIEW => {
                let Some(plan_id) = non_empty(params.plan_id.as_deref()) else {
                    return self.refuse(
                        call,
                        DeviceControlError::InvalidParam(
                            "preview 必须带 plan 拍的 plan_id (三拍同计划绑定)".to_string(),
                        ),
                    );
                };
                match self.runner.preview(
                    &params.device,
                    &params.device_action,
                    &action_params,
                    Some(plan_id),
                ) {
                    Ok(preview) => {
                        let card = DeviceApprovalCard::from_preview(&preview);
                        ToolResult::ok(
                            &call.id,
                            json!({
                                "action": ACTION_PREVIEW,
                                "status": "previewed",
                                "plan_id": preview.plan_id,
                                "device": preview.device,
                                "device_name": preview.device_name,
                                "device_action": preview.device_action,
                                "grade": preview.grade.as_str(),
                                "before": preview.before,
                                "after": preview.after,
                                "effects": preview.effects,
                                "zero_side_effect": true,
                                "approval_card": card.to_json(),
                            }),
                        )
                        .with_name(DEVICE_CONTROL_TOOL_NAME)
                    }
                    Err(error) => self.refuse(call, error),
                }
            }
            ACTION_EXECUTE => {
                let Some(plan_id) = non_empty(params.plan_id.as_deref()) else {
                    return self.refuse(
                        call,
                        DeviceControlError::InvalidParam(
                            "execute 必须带 plan 拍的 plan_id (三拍同计划绑定)".to_string(),
                        ),
                    );
                };
                let evidence = ApprovalEvidence {
                    approved_by: params.approved_by.clone(),
                    im_card_id: params.im_card_id.clone(),
                };
                match self.runner.execute(
                    &params.device,
                    &params.device_action,
                    &action_params,
                    Some(plan_id),
                    &evidence,
                ) {
                    Ok(executed) => ToolResult::ok(
                        &call.id,
                        json!({
                            "action": ACTION_EXECUTE,
                            "status": "executed",
                            "plan_id": executed.plan_id,
                            "record_id": executed.record_id,
                            "device": executed.device,
                            "device_action": executed.device_action,
                            "grade": executed.grade.as_str(),
                            "before": executed.before,
                            "after": executed.after,
                            "approved_by": executed.approved_by,
                            "im_card_id": executed.im_card_id,
                        }),
                    )
                    .with_name(DEVICE_CONTROL_TOOL_NAME),
                    Err(error) => self.refuse(call, error),
                }
            }
            other => self.refuse(
                call,
                DeviceControlError::InvalidParam(format!(
                    "拍 {other:?} 不在闭合拍词表 (plan/preview/execute)"
                )),
            ),
        }
    }
}

/// 组装生产工具面: 三拍执行 + 五段流水线 (模型侧一个能力)。
pub fn device_control_capability(runner: Arc<DeviceControlRunner>) -> Arc<dyn ToolCapability> {
    let tool: Arc<dyn ToolCapability> = Arc::new(DeviceControlTool::new(Arc::clone(&runner)));
    Arc::new(crate::exec_pipeline::PipelinedCapability::new(
        tool,
        device_control_pipeline(runner.whitelist().clone()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::{SessionId, TraceId};

    fn temp_runner() -> (tempfile::TempDir, Arc<DeviceControlRunner>) {
        let dir = tempfile::tempdir().expect("tempdir");
        let runner = DeviceControlRunner::mock(DeviceControlConfig::new(dir.path()));
        (dir, runner)
    }

    fn evidence(approved_by: &str) -> ApprovalEvidence {
        ApprovalEvidence {
            approved_by: Some(approved_by.to_string()),
            im_card_id: None,
        }
    }

    fn money_evidence() -> ApprovalEvidence {
        ApprovalEvidence {
            approved_by: Some("主人".to_string()),
            im_card_id: Some("im-card-1".to_string()),
        }
    }

    /// ① 三拍语义: plan 拍零副作用 (不建账、不改设备)。
    #[test]
    fn plan_beat_is_pure_and_binds_the_plan_id() {
        let (dir, runner) = temp_runner();
        let plan = runner
            .plan("lamp", "set_brightness", &json!({"level": 80}))
            .expect("plan");
        assert_eq!(plan.grade, ActionGrade::Reversible);
        assert_eq!(
            plan.plan_id,
            plan_id_for(
                "lamp",
                "set_brightness",
                ActionGrade::Reversible,
                &json!({"level": 80})
            ),
            "三拍同计划绑定键 = 声明内容的稳定摘要"
        );
        assert!(
            !dir.path().join(DEVICE_ACTION_LEDGER_FILE).exists(),
            "plan 零副作用: 不落账"
        );
        let state = runner.adapter.snapshot("lamp").expect("snapshot");
        assert_eq!(
            state.get("brightness"),
            Some(&json!(0)),
            "plan 零副作用: 设备状态不因 plan 变"
        );
    }

    /// ② 三拍语义: preview 拍零副作用 (投影"将发生什么", 不改设备、不落账)。
    #[test]
    fn preview_beat_projects_effects_with_zero_side_effects() {
        let (dir, runner) = temp_runner();
        let preview = runner
            .preview("lamp", "set_brightness", &json!({"level": 80}), None)
            .expect("preview");
        assert_eq!(preview.before.get("brightness"), Some(&json!(0)));
        assert_eq!(preview.after.get("brightness"), Some(&json!(80)));
        assert!(
            preview
                .effects
                .iter()
                .any(|line| line.contains("brightness: 0 → 80")),
            "效果行 = 人读 'key: 旧 → 新': {:?}",
            preview.effects
        );
        assert!(
            !dir.path().join(DEVICE_ACTION_LEDGER_FILE).exists(),
            "preview 零副作用: 不落账"
        );
        let state = runner.adapter.snapshot("lamp").expect("snapshot");
        assert_eq!(
            state.get("brightness"),
            Some(&json!(0)),
            "preview 零副作用: 投影不动设备状态"
        );
    }

    /// ③ 三拍语义: execute 拍真执行 (mock 状态变更 + 状态账追加)。
    #[test]
    fn execute_beat_changes_mock_state_and_appends_the_ledger() {
        let (_dir, runner) = temp_runner();
        let preview = runner
            .preview("lamp", "set_brightness", &json!({"level": 80}), None)
            .expect("preview");
        let executed = runner
            .execute(
                "lamp",
                "set_brightness",
                &json!({"level": 80}),
                Some(&preview.plan_id),
                &evidence("主人"),
            )
            .expect("execute");
        assert_eq!(executed.after.get("brightness"), Some(&json!(80)));
        let state = runner.adapter.snapshot("lamp").expect("snapshot");
        assert_eq!(
            state.get("brightness"),
            Some(&json!(80)),
            "execute = 真执行: mock 状态真变更"
        );
        let scan = runner.ledger().scan().expect("scan");
        assert_eq!(scan.records.len(), 1, "execute = 真执行 + 入账");
        let record = &scan.records[0].body;
        assert_eq!(record.device, "lamp");
        assert_eq!(record.device_action, "set_brightness");
        assert_eq!(record.approved_by, "主人", "账本记谁批");
        assert!(record.at_ms > 0, "账本记何时");
        assert_eq!(record.plan_id, executed.plan_id, "账本绑同一计划");
    }

    /// ④ 底线④: 先模拟后执行 —— 未预览的计划拒绝执行。
    #[test]
    fn execute_refuses_a_plan_that_was_never_previewed() {
        let (_dir, runner) = temp_runner();
        let error = runner
            .execute("lamp", "power_on", &json!({}), None, &evidence("主人"))
            .expect_err("未预览必须拒");
        assert!(
            error.message().contains("先模拟后执行"),
            "拒绝帧点名三拍纪律: {}",
            error.message()
        );
    }

    /// ⑤ 三拍同计划绑定: 载荷被改 (plan_id 不符) 拒绝即帧。
    #[test]
    fn execute_rejects_plan_id_drift() {
        let (_dir, runner) = temp_runner();
        let preview = runner
            .preview("lamp", "power_on", &json!({}), None)
            .expect("preview");
        let error = runner
            .execute(
                "lamp",
                "power_on",
                &json!({}),
                Some("deadbeefdeadbeef"),
                &evidence("主人"),
            )
            .expect_err("计划漂移必须拒");
        assert!(
            matches!(error, DeviceControlError::PlanMismatch { .. }),
            "{error:?}"
        );
        assert!(
            error.message().contains("载荷被改即拒"),
            "{}",
            error.message()
        );
        // 同一计划只可执行一次 (预览票一次性)。
        runner
            .execute(
                "lamp",
                "power_on",
                &json!({}),
                Some(&preview.plan_id),
                &evidence("主人"),
            )
            .expect("首次执行");
        let error = runner
            .execute(
                "lamp",
                "power_on",
                &json!({}),
                Some(&preview.plan_id),
                &evidence("主人"),
            )
            .expect_err("预览票一次性");
        assert!(
            matches!(error, DeviceControlError::PreviewRequired(_)),
            "{error:?}"
        );
    }

    /// ⑥ 分级矩阵 · reversible: 审批档 (默认人批), 自动档可配 (只此档)。
    #[test]
    fn reversible_grade_is_approval_tier_with_optional_auto_pass() {
        let (_dir, runner) = temp_runner();
        runner
            .preview("lamp", "power_on", &json!({}), None)
            .expect("preview");
        let error = runner
            .execute(
                "lamp",
                "power_on",
                &json!({}),
                None,
                &ApprovalEvidence::default(),
            )
            .expect_err("默认审批档: 无人批不执行");
        assert!(
            matches!(error, DeviceControlError::ApprovalRequired(_)),
            "{error:?}"
        );

        let dir = tempfile::tempdir().expect("tempdir");
        let auto_runner = Arc::new(DeviceControlRunner::new(
            DeviceControlConfig::new(dir.path()).with_auto_pass_reversible(true),
            DeviceRegistry::with_mock_fixtures(),
            MockDeviceAdapter::with_builtin_fixtures(),
        ));
        auto_runner
            .preview("lamp", "power_on", &json!({}), None)
            .expect("preview");
        let executed = auto_runner
            .execute(
                "lamp",
                "power_on",
                &json!({}),
                None,
                &ApprovalEvidence::default(),
            )
            .expect("自动档只放 reversible");
        assert_eq!(executed.approved_by, AUTO_PASS_APPROVER, "自动档如实记谁批");
    }

    /// ⑦ 分级矩阵 · irreversible: 永远逐次人批 (自动档开了也不放)。
    #[test]
    fn irreversible_grade_always_needs_per_call_human_approval() {
        let dir = tempfile::tempdir().expect("tempdir");
        let registry = DeviceRegistry::with_mock_fixtures();
        let adapter = MockDeviceAdapter::with_builtin_fixtures();
        adapter.ensure_device("front_door", DeviceState::new());
        let runner = Arc::new(DeviceControlRunner::new(
            DeviceControlConfig::new(dir.path()).with_auto_pass_reversible(true),
            registry.clone(),
            adapter,
        ));
        registry
            .register(DeviceEntry {
                id: "front_door".to_string(),
                name: "前门".to_string(),
                kind: "door".to_string(),
                enabled: true,
                actions: vec!["unlock_door".to_string()],
            })
            .expect("register");
        runner
            .preview("front_door", "unlock_door", &json!({}), None)
            .expect("preview");
        let error = runner
            .execute(
                "front_door",
                "unlock_door",
                &json!({}),
                None,
                &ApprovalEvidence::default(),
            )
            .expect_err("irreversible 无自动档");
        assert!(
            matches!(error, DeviceControlError::ApprovalRequired(_)),
            "{error:?}"
        );
        assert!(
            error.message().contains("永远逐次人批"),
            "{}",
            error.message()
        );
        let executed = runner
            .execute(
                "front_door",
                "unlock_door",
                &json!({}),
                None,
                &evidence("主人"),
            )
            .expect("人批后执行");
        assert_eq!(executed.grade, ActionGrade::Irreversible);
    }

    /// ⑧ 分级矩阵 · money: 永远逐次人批 + IM 卡 (缺卡拒绝, 自动档不适用)。
    #[test]
    fn money_grade_always_needs_per_call_human_approval_plus_im_card() {
        let dir = tempfile::tempdir().expect("tempdir");
        let registry = DeviceRegistry::with_mock_fixtures();
        let adapter = MockDeviceAdapter::with_builtin_fixtures();
        adapter.ensure_device("shop", DeviceState::new());
        let runner = Arc::new(DeviceControlRunner::new(
            DeviceControlConfig::new(dir.path()).with_auto_pass_reversible(true),
            registry.clone(),
            adapter,
        ));
        registry
            .register(DeviceEntry {
                id: "shop".to_string(),
                name: "商店".to_string(),
                kind: "service".to_string(),
                enabled: true,
                actions: vec!["purchase".to_string(), "place_order".to_string()],
            })
            .expect("register");
        let params = json!({"item": "牛奶", "amount": 12.5, "currency": "CNY"});
        runner
            .preview("shop", "purchase", &params, None)
            .expect("preview");
        let error = runner
            .execute(
                "shop",
                "purchase",
                &params,
                None,
                &ApprovalEvidence {
                    approved_by: Some("主人".to_string()),
                    im_card_id: None,
                },
            )
            .expect_err("缺 IM 卡必须拒");
        assert!(
            matches!(error, DeviceControlError::ImCardRequired),
            "{error:?}"
        );
        let error = runner
            .execute(
                "shop",
                "purchase",
                &params,
                None,
                &ApprovalEvidence::default(),
            )
            .expect_err("无人批必须拒");
        assert!(
            matches!(error, DeviceControlError::ApprovalRequired(_)),
            "{error:?}"
        );
        runner
            .preview("shop", "purchase", &params, None)
            .expect("preview again");
        let executed = runner
            .execute("shop", "purchase", &params, None, &money_evidence())
            .expect("人批 + IM 卡后执行");
        assert_eq!(executed.im_card_id.as_deref(), Some("im-card-1"));
        assert_eq!(executed.grade, ActionGrade::Money);
    }

    /// ⑨ 分级词表闭合: 三档词表互斥、词表外拒绝即帧。
    #[test]
    fn grading_vocabulary_is_closed_and_disjoint() {
        for action in REVERSIBLE_ACTIONS {
            assert_eq!(grade_of(action), Some(ActionGrade::Reversible), "{action}");
        }
        for action in IRREVERSIBLE_ACTIONS {
            assert_eq!(
                grade_of(action),
                Some(ActionGrade::Irreversible),
                "{action}"
            );
        }
        for action in MONEY_ACTIONS {
            assert_eq!(grade_of(action), Some(ActionGrade::Money), "{action}");
        }
        for off in ["nuke", "Purchase", "purchase ", "drop_database", ""] {
            assert_eq!(grade_of(off), None, "词表外必须拒: {off:?}");
        }
        let (_dir, runner) = temp_runner();
        let error = runner
            .plan("lamp", "nuke", &json!({}))
            .expect_err("词表外动作必须拒绝即帧");
        assert!(
            matches!(error, DeviceControlError::UnknownAction(_)),
            "{error:?}"
        );
    }

    /// ⑩ 白名单拒: 未登记设备 / 未启用设备都拒绝即帧 (默认零设备)。
    #[test]
    fn whitelist_refuses_unregistered_and_disabled_devices() {
        let registry = DeviceRegistry::new();
        assert_eq!(registry.counts(), (0, 0), "默认零设备 (底线①)");
        let error = registry.authorize("lamp").expect_err("未登记必须拒");
        assert!(error.message().contains("未登记"), "{}", error.message());

        let dir = tempfile::tempdir().expect("tempdir");
        let registry = DeviceRegistry::with_mock_fixtures();
        let runner = Arc::new(DeviceControlRunner::new(
            DeviceControlConfig::new(dir.path()),
            registry.clone(),
            MockDeviceAdapter::with_builtin_fixtures(),
        ));
        registry.set_enabled("lamp", false).expect("disable");
        let error = runner
            .plan("lamp", "power_on", &json!({}))
            .expect_err("未启用必须拒");
        assert!(
            matches!(error, DeviceControlError::DeviceDisabled(_)),
            "{error:?}"
        );
        let error = runner
            .plan("ghost", "power_on", &json!({}))
            .expect_err("未登记必须拒");
        assert!(
            matches!(error, DeviceControlError::UnknownDevice(_)),
            "{error:?}"
        );
    }

    /// ⑪ 每设备动作白名单闭合 + 参数键集闭合 (plan 拍拒绝即帧)。
    #[test]
    fn device_action_whitelist_and_params_are_closed() {
        let (_dir, runner) = temp_runner();
        let error = runner
            .plan("lamp", "set_temperature", &json!({"celsius": 24}))
            .expect_err("灯没有温度动作");
        assert!(
            matches!(error, DeviceControlError::UnsupportedAction { .. }),
            "{error:?}"
        );
        let error = runner
            .plan("lamp", "set_brightness", &json!({"level": 200}))
            .expect_err("越界参数必须拒");
        assert!(
            error.message().contains("拒绝设备动作"),
            "{}",
            error.message()
        );
        let error = runner
            .plan("lamp", "power_on", &json!({"surprise": 1}))
            .expect_err("多余参数必须拒");
        assert!(error.message().contains("键集"), "{}", error.message());
    }

    /// ⑫ 状态账 append-only: 追加保序、内容不重写不删行。
    #[test]
    fn ledger_is_append_only_and_ordered() {
        let (dir, runner) = temp_runner();
        for level in [10, 90] {
            runner
                .preview("lamp", "set_brightness", &json!({"level": level}), None)
                .expect("preview");
            runner
                .execute(
                    "lamp",
                    "set_brightness",
                    &json!({"level": level}),
                    None,
                    &evidence("主人"),
                )
                .expect("execute");
        }
        let path = dir.path().join(DEVICE_ACTION_LEDGER_FILE);
        let bytes_after_two = std::fs::read(&path).expect("read ledger");
        let scan = runner.ledger().scan().expect("scan");
        assert_eq!(scan.records.len(), 2);
        assert_eq!(
            scan.records[0].body.params["level"],
            json!(10),
            "保序第一条"
        );
        assert_eq!(
            scan.records[1].body.params["level"],
            json!(90),
            "保序第二条"
        );
        // 追加第三条后, 前两条字节原样 (append-only: 不重写不删行)。
        runner
            .preview("lamp", "power_off", &json!({}), None)
            .expect("preview");
        runner
            .execute("lamp", "power_off", &json!({}), None, &evidence("主人"))
            .expect("execute");
        let bytes_after_three = std::fs::read(&path).expect("read ledger");
        assert!(
            bytes_after_three.len() > bytes_after_two.len(),
            "append-only: 只增不减"
        );
        assert_eq!(
            &bytes_after_three[..bytes_after_two.len()],
            &bytes_after_two[..],
            "append-only: 既有行字节原样"
        );
    }

    /// ⑬ mock fixture 三件 (灯/空调/微波炉) 真走三拍 + 状态账。
    #[test]
    fn mock_fixtures_walk_the_three_beats() {
        let (_dir, runner) = temp_runner();
        let cases: Vec<(&str, &str, Value)> = vec![
            ("lamp", "set_brightness", json!({"level": 42})),
            ("air_conditioner", "set_temperature", json!({"celsius": 24})),
            ("microwave", "set_timer", json!({"seconds": 30})),
        ];
        for (device, action, params) in cases {
            let plan = runner.plan(device, action, &params).expect("plan");
            let preview = runner
                .preview(device, action, &params, Some(&plan.plan_id))
                .expect("preview");
            assert_eq!(preview.plan_id, plan.plan_id, "三拍同计划");
            let executed = runner
                .execute(
                    device,
                    action,
                    &params,
                    Some(&plan.plan_id),
                    &evidence("主人"),
                )
                .expect("execute");
            assert_ne!(executed.before, executed.after, "execute 真变更状态");
        }
        let scan = runner.ledger().scan().expect("scan");
        assert_eq!(scan.records.len(), 3, "三件各入一账");
        let lamp = runner.adapter.snapshot("lamp").expect("snapshot");
        assert_eq!(lamp.get("brightness"), Some(&json!(42)), "灯真变更");
        let ac = runner
            .adapter
            .snapshot("air_conditioner")
            .expect("snapshot");
        assert_eq!(ac.get("temperature"), Some(&json!(24)), "空调真变更");
        let microwave = runner.adapter.snapshot("microwave").expect("snapshot");
        assert_eq!(
            microwave.get("timer_seconds"),
            Some(&json!(30)),
            "微波炉真变更"
        );
    }

    /// ⑭ 审批卡载荷: 预览效果 + 分级语义 + money 档 IM 卡标注。
    #[test]
    fn approval_card_carries_preview_effects_and_im_card_flag() {
        let (_dir, runner) = temp_runner();
        let preview = runner
            .preview("lamp", "set_brightness", &json!({"level": 80}), None)
            .expect("preview");
        let card = DeviceApprovalCard::from_preview(&preview);
        assert_eq!(card.grade, "reversible");
        assert!(!card.requires_im_card, "reversible 无 IM 卡要求");
        let rendered = card.render();
        assert!(rendered.contains("将发生什么 (先模拟后执行)"), "{rendered}");
        assert!(rendered.contains("brightness: 0 → 80"), "{rendered}");
        assert!(
            rendered.contains(&format!("计划: {}", preview.plan_id)),
            "渲染含计划 id: {rendered}"
        );
        let payload = card.to_json();
        assert_eq!(payload["buttons"], json!(["approve", "reject"]));
        assert_eq!(payload["risk"], json!("medium"));
        assert_eq!(payload["im_card"], json!(false));

        // money 档: IM 卡标注恒 true + 卡面同文。
        let dir = tempfile::tempdir().expect("tempdir");
        let registry = DeviceRegistry::with_mock_fixtures();
        let adapter = MockDeviceAdapter::with_builtin_fixtures();
        adapter.ensure_device("shop", DeviceState::new());
        let runner = Arc::new(DeviceControlRunner::new(
            DeviceControlConfig::new(dir.path()),
            registry.clone(),
            adapter,
        ));
        registry
            .register(DeviceEntry {
                id: "shop".to_string(),
                name: "商店".to_string(),
                kind: "service".to_string(),
                enabled: true,
                actions: vec!["purchase".to_string()],
            })
            .expect("register");
        let preview = runner
            .preview(
                "shop",
                "purchase",
                &json!({"item": "牛奶", "amount": 12.5, "currency": "CNY"}),
                None,
            )
            .expect("preview");
        let card = DeviceApprovalCard::from_preview(&preview);
        assert!(card.requires_im_card, "money 档恒要 IM 卡");
        assert_eq!(card.to_json()["im_card"], json!(true));
        assert_eq!(card.to_json()["risk"], json!("high"));
        assert!(card.render().contains("IM 审批卡"), "{}", card.render());
    }

    /// ⑮ 开关 fail-closed: 双闸齐开才生效 (显式 `"1"`, 杂值全拒)。
    #[test]
    fn switch_is_fail_closed_on_the_double_gate() {
        // crate 级 env 锁（跨模块互斥，见 test_support::CRATE_ENV_LOCK）。
        let _guard = crate::test_support::lock_env();
        // 跨件一致性: 感知族开关常量与感知批同一把钥匙 (同名同值不漂移)。
        assert_eq!(
            crate::device::sense::DEVICE_SENSE_ENABLE_ENV,
            DEVICE_SENSE_ENABLE_ENV,
            "双闸的上游钥匙与感知批同一常量"
        );
        let name_sense = DEVICE_SENSE_ENABLE_ENV;
        let name_control = DEVICE_CONTROL_ENABLE_ENV;
        let name_auto = DEVICE_AUTO_PASS_REVERSIBLE_ENV;
        let saved = (
            std::env::var(name_sense).ok(),
            std::env::var(name_control).ok(),
            std::env::var(name_auto).ok(),
        );
        let restore = |saved: &(Option<String>, Option<String>, Option<String>)| {
            for (name, value) in [
                (name_sense, &saved.0),
                (name_control, &saved.1),
                (name_auto, &saved.2),
            ] {
                crate::test_support::set_env(name, value.as_deref());
            }
        };
        let cases: &[(Option<&str>, Option<&str>)] = &[
            (None, None),
            (Some("1"), None),
            (None, Some("1")),
            (Some("0"), Some("1")),
            (Some("1"), Some("0")),
            (Some("true"), Some("true")),
            (Some("yes"), Some("1")),
            (Some("1"), Some("garbage")),
        ];
        for (sense, control) in cases {
            crate::test_support::set_env(name_sense, *sense);
            crate::test_support::set_env(name_control, *control);
            assert!(
                !device_control_enabled_from_env(),
                "双闸未齐必须关: {sense:?} {control:?}"
            );
            assert!(
                DeviceControlConfig::from_env(".").is_none(),
                "双闸未齐 = 配置不构造 (fail-closed)"
            );
        }
        crate::test_support::set_env(name_sense, Some("1"));
        crate::test_support::set_env(name_control, Some(" 1 "));
        crate::test_support::set_env(name_auto, None);
        assert!(device_control_enabled_from_env(), "双闸齐开 (空白容忍)");
        let config = DeviceControlConfig::from_env(".").expect("双闸齐开 = 配置构造");
        assert!(!config.auto_pass_reversible, "自动档默认关");
        crate::test_support::set_env(name_auto, Some("1"));
        let config = DeviceControlConfig::from_env(".").expect("配置构造");
        assert!(config.auto_pass_reversible, "自动档随子开关");
        restore(&saved);
    }

    /// ⑯ 治理钩子分级矩阵: 零副作用拍放行 / 三档分级审批 / 词表外拒绝即帧。
    #[tokio::test]
    async fn governance_hook_grades_the_three_tiers() {
        fn dispatch(arguments: Value) -> GovernanceRequest<'static> {
            let capability: &'static CapabilityId = Box::leak(Box::new(
                CapabilityId::new(DEVICE_CONTROL_CAPABILITY_ID).expect("capability id"),
            ));
            let arguments: &'static Value = Box::leak(Box::new(arguments));
            GovernanceRequest::new(
                Action::CapabilityDispatch {
                    capability,
                    arguments,
                },
                SessionId::new(),
                TraceId::new(),
                1,
            )
        }

        let hook = DeviceControlApprovalHook::new();
        // plan / preview 零副作用读面: 零审批。
        for beat in [ACTION_PLAN, ACTION_PREVIEW] {
            let decision = hook
                .evaluate(&dispatch(
                    json!({"action": beat, "device": "lamp", "device_action": "power_on"}),
                ))
                .await;
            assert_eq!(
                decision,
                Decision::Allow,
                "{beat} 零副作用零审批: {decision:?}"
            );
        }
        // reversible: 默认审批档。
        let decision = hook
            .evaluate(&dispatch(
                json!({"action": "execute", "device": "lamp", "device_action": "power_on"}),
            ))
            .await;
        let Decision::RequireApproval { reason } = decision else {
            panic!("reversible 默认审批档: {decision:?}");
        };
        assert!(reason.contains("审批档"), "{reason}");
        // irreversible / money: 永远逐次人批 (自动档开了也不放)。
        let auto_hook = DeviceControlApprovalHook::new().with_auto_pass_reversible(true);
        for (action, needle) in [("unlock_door", "永远逐次人批"), ("purchase", "IM 审批卡")]
        {
            let decision = auto_hook
                .evaluate(&dispatch(
                    json!({"action": "execute", "device": "shop", "device_action": action}),
                ))
                .await;
            let Decision::RequireApproval { reason } = decision else {
                panic!("{action} 永逐次人批: {decision:?}");
            };
            assert!(reason.contains(needle), "{reason}");
        }
        // 自动档只放 reversible。
        let decision = auto_hook
            .evaluate(&dispatch(
                json!({"action": "execute", "device": "lamp", "device_action": "power_on"}),
            ))
            .await;
        assert_eq!(
            decision,
            Decision::Allow,
            "自动档只放 reversible: {decision:?}"
        );
        // 词表外动作 / 未知拍: fail-closed 拒绝即帧。
        let decision = hook
            .evaluate(&dispatch(
                json!({"action": "execute", "device": "lamp", "device_action": "nuke"}),
            ))
            .await;
        assert!(matches!(decision, Decision::Deny { .. }), "{decision:?}");
        let decision = hook
            .evaluate(&dispatch(
                json!({"action": "wish", "device": "lamp", "device_action": "power_on"}),
            ))
            .await;
        assert!(matches!(decision, Decision::Deny { .. }), "{decision:?}");
    }

    /// ⑰ 单调边界 Guard: 白名单/词表/参数硬边界只拒不放。
    #[test]
    fn boundary_guard_denies_whitelist_and_vocabulary_violations() {
        let (_dir, runner) = temp_runner();
        let guard = DeviceControlBoundaryGuard::new(runner.whitelist().clone());
        let ok = guard
            .refusal(&json!({"action": "execute", "device": "lamp", "device_action": "power_on"}));
        assert_eq!(ok, None, "合法调用不拒");
        let denied = guard
            .refusal(&json!({"action": "execute", "device": "ghost", "device_action": "power_on"}))
            .expect("未登记设备必须拒");
        assert!(denied.contains("未登记"), "{denied}");
        let denied = guard
            .refusal(&json!({"action": "execute", "device": "lamp", "device_action": "set_temperature", "params": {"celsius": 24}}))
            .expect("设备词表外动作必须拒");
        assert!(denied.contains("白名单"), "{denied}");
        let denied = guard
            .refusal(&json!({"action": "execute", "device": "lamp", "device_action": "set_brightness", "params": {"level": -1}}))
            .expect("越界参数必须拒");
        assert!(denied.contains("拒绝设备动作"), "{denied}");
    }

    /// ⑱ 自述节: 设备计数 / 分级计数 / 最近动作真值如实。
    #[test]
    fn status_section_reports_honest_counts() {
        let (_dir, runner) = temp_runner();
        let empty = runner.status().expect("status");
        assert_eq!(empty.devices_total, 3, "mock fixture 三件");
        assert_eq!(empty.devices_enabled, 3);
        assert_eq!(empty.actions_executed, 0);
        assert_eq!(empty.note.as_deref(), Some("尚无动作记录"), "无记录不编数");
        assert_eq!(empty.last_action, None, "无记录不编最近动作");

        runner
            .preview("lamp", "power_on", &json!({}), None)
            .expect("preview");
        runner
            .execute("lamp", "power_on", &json!({}), None, &evidence("主人"))
            .expect("execute");
        let status = runner.status().expect("status");
        assert_eq!(status.actions_executed, 1);
        assert_eq!(status.executed_reversible, 1);
        assert_eq!(status.executed_money, 0, "没动钱不编数");
        let last = status.last_action.as_ref().expect("最近动作");
        assert!(last.contains("lamp / power_on"), "{last}");
        assert!(last.contains("谁批: 主人"), "{last}");
        let section = status.to_json();
        assert_eq!(section["executed_money"], json!(0));
        assert_eq!(section["devices_enabled"], json!(3));
    }

    /// ⑲ 工具面: 三拍走通 + 缺 plan_id 即拒 + 输出归一字段恒在。
    #[tokio::test]
    async fn tool_face_walks_the_three_beats() {
        use apeireth_protocol::canonical::ToolOutcome;

        fn value_of(result: &ToolResult) -> Value {
            match &result.outcome {
                ToolOutcome::Ok { value } => value.clone(),
                other => panic!("必须成功帧: {other:?}"),
            }
        }

        let (_dir, runner) = temp_runner();
        let tool = DeviceControlTool::new(runner.clone());
        let call = |id: &str, arguments: Value| ToolCall {
            id: id.to_string(),
            name: DEVICE_CONTROL_TOOL_NAME.to_string(),
            arguments,
        };
        let planned = tool
            .invoke(&call(
                "c1",
                json!({"action": "plan", "device": "lamp", "device_action": "set_brightness", "params": {"level": 30}}),
            ))
            .await;
        assert!(planned.is_ok(), "{planned:?}");
        let planned = value_of(&planned);
        assert_eq!(planned["action"], json!(ACTION_PLAN));
        assert_eq!(planned["status"], json!("planned"));
        assert_eq!(planned["grade"], json!("reversible"));
        assert_eq!(
            planned["zero_side_effect"],
            json!(true),
            "plan 帧自报零副作用"
        );
        let plan_id = planned["plan_id"]
            .as_str()
            .expect("plan_id 进帧")
            .to_string();

        let previewed = tool
            .invoke(&call(
                "c2",
                json!({"action": "preview", "device": "lamp", "device_action": "set_brightness", "params": {"level": 30}, "plan_id": plan_id}),
            ))
            .await;
        assert!(previewed.is_ok(), "{previewed:?}");
        let previewed = value_of(&previewed);
        assert_eq!(previewed["status"], json!("previewed"));
        assert!(previewed["approval_card"].is_object(), "审批卡载荷随预览帧");

        let executed = tool
            .invoke(&call(
                "c3",
                json!({"action": "execute", "device": "lamp", "device_action": "set_brightness", "params": {"level": 30}, "plan_id": plan_id, "approved_by": "主人"}),
            ))
            .await;
        assert!(executed.is_ok(), "{executed:?}");
        let executed = value_of(&executed);
        assert_eq!(executed["status"], json!("executed"));
        assert_eq!(executed["approved_by"], json!("主人"));
        assert!(executed["record_id"].is_string(), "状态账记录 id 进帧");

        // 缺 plan_id 的 preview/execute 即拒。
        let refused = tool
            .invoke(&call(
                "c4",
                json!({"action": "execute", "device": "lamp", "device_action": "set_brightness", "params": {"level": 30}, "approved_by": "主人"}),
            ))
            .await;
        assert!(!refused.is_ok(), "{refused:?}");
        // 词表外拍即拒。
        let refused = tool
            .invoke(&call(
                "c5",
                json!({"action": "wish", "device": "lamp", "device_action": "power_on"}),
            ))
            .await;
        assert!(!refused.is_ok(), "{refused:?}");
    }

    /// ⑳ 白名单复用: L1 设备表 (`devices.json`) 只读消费桥 —— 同一本设备表,
    /// 登记/启用判定同源; 类目推动作词表 (闭合、保守); 坏表拒开不回退。
    #[test]
    fn file_registry_bridge_reuses_the_l1_whitelist_read_only() {
        use crate::device::registry::{Device, DeviceKind, DevicesBody};

        let dir = tempfile::tempdir().expect("tempdir");
        let store = crate::device::registry::DeviceRegistry::under_data_dir(dir.path());
        let body = DevicesBody {
            devices: vec![
                Device {
                    id: "lamp".to_string(),
                    kind: DeviceKind::Appliance,
                    name: "灯".to_string(),
                    adapter: "mock".to_string(),
                    enabled: true,
                },
                Device {
                    id: "thermometer".to_string(),
                    kind: DeviceKind::Sensor,
                    name: "温度计".to_string(),
                    adapter: "mock".to_string(),
                    enabled: true,
                },
                Device {
                    id: "oven".to_string(),
                    kind: DeviceKind::Appliance,
                    name: "烤箱".to_string(),
                    adapter: "mock".to_string(),
                    enabled: false,
                },
            ],
        };
        store.save(&body).expect("save devices.json");
        let whitelist = RegistryFileWhitelist::under_data_dir(dir.path());

        // 启用家电: 可授权, 动作词表 = reversible 家电面 (类别推导)。
        let entry = whitelist.authorize("lamp").expect("lamp 授权");
        assert_eq!(entry.actions, REVERSIBLE_ACTIONS);
        // 未登记 / 未启用: 拒绝即帧 (与进程内注册表同一语义)。
        let error = whitelist.authorize("ghost").expect_err("未登记必须拒");
        assert!(
            matches!(error, DeviceControlError::UnknownDevice(_)),
            "{error:?}"
        );
        let error = whitelist.authorize("oven").expect_err("未启用必须拒");
        assert!(
            matches!(error, DeviceControlError::DeviceDisabled(_)),
            "{error:?}"
        );
        // 只读感知类目: 零执行动作 (fail-closed, 保守推导)。
        let entry = whitelist.authorize("thermometer").expect("sensor 授权");
        assert!(entry.actions.is_empty(), "传感器只读, 无执行动作");
        assert_eq!(
            actions_for_kind("other"),
            &[] as &[&str],
            "兜底类目同样零执行动作 (fail-closed)"
        );
        assert_eq!(whitelist.counts().expect("counts"), (3, 2));

        // 端到端: 文件白名单 + mock 执行面真走三拍。
        let runner = DeviceControlRunner::mock_with_file_whitelist(
            DeviceControlConfig::new(dir.path()),
            dir.path().join(crate::device::registry::DEVICES_FILE),
        );
        let plan = runner.plan("lamp", "power_on", &json!({})).expect("plan");
        runner
            .preview("lamp", "power_on", &json!({}), Some(&plan.plan_id))
            .expect("preview");
        let executed = runner
            .execute(
                "lamp",
                "power_on",
                &json!({}),
                Some(&plan.plan_id),
                &evidence("主人"),
            )
            .expect("execute");
        assert_eq!(executed.after.get("power"), Some(&json!("on")));
        let error = runner
            .plan("thermometer", "power_on", &json!({}))
            .expect_err("传感器零执行动作");
        assert!(
            matches!(error, DeviceControlError::UnsupportedAction { .. }),
            "{error:?}"
        );

        // 坏表: 拒开即帧, fail-closed 不回退空表 (照实报错不编数)。
        std::fs::write(
            dir.path().join(crate::device::registry::DEVICES_FILE),
            "{{not json",
        )
        .expect("坏表落盘");
        let error = whitelist.authorize("lamp").expect_err("坏表必须拒开");
        assert!(
            matches!(error, DeviceControlError::Whitelist(_)),
            "{error:?}"
        );
        assert!(
            error.message().contains("fail-closed"),
            "{}",
            error.message()
        );
    }
}
