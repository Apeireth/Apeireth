//! The structured self-report tool (`tool.self_status`).
//!
//! 产品哲学「不假装」要求模型能**实测自身状态再发言**。本模块把自省设计化为
//! 一个专用只读面: 一次调用返回一份**结构化自述** JSON —— 身份 / 能力名册 /
//! 记忆账本 / 调参状态 / 预算 / 工作区 / 模型画像。数据一律取自运行时**真实生效值**
//! (装配时的实际开关、存储可查元数据、进程内生效调参), 不复述配置文本。
//!
//! # 三条硬边界
//!
//! 1. **只读档**: 工具无写无执行, 无参数, 默认可用, 零审批 (治理侧只做 grant)。
//! 2. **计数不回内容**: 记忆账本只回计数元数据, 永不回放记忆原文。
//! 3. **凭据只回存在性**: 工作区一节只回「凭据是否存在」布尔, 不回显凭据本体。
//!
//! # 错误即帧, 缺失显式 null
//!
//! data 系来源 (记忆账本 / 工作区探测) 读取失败时, **该字段显式 `null` 并给出
//! 原因**, 整帧仍然成功返回 —— 自述宁可如实说"这块我没测到", 不整帧失败,
//! 更不编造数字。输出经既有 [`ToolOutcome`] 归一合同冻结
//! (见 [`SelfStatusTool::output_schema`]), 超时归 `timeout.*` 既有 code 族。
//!
//! # 生物机制族只读聚合 (W3.2)
//!
//! 14 条生物机制 (SG-13…SG-26) 不逐个新增顶层节 (21 节合同冻结): 统一挂在既有
//! `capabilities` 节内的 [`CAPABILITIES_BIO_MECHANISMS`] 只读子对象上, 每机制
//! 一格 `{stage, value, threshold, last_action, wired, reason}`。默认关
//! ([`BIO_SELF_STATUS_ENABLE_ENV`] 只认 `"1"`), 关时 `mechanisms` 显式 `null`
//! + 原因「未启用」; 未接线/采集失败的格显式 `null` + 原因, 绝不编数值。

use std::collections::BTreeMap;
use std::sync::Arc;

use apeireth_core::cost_router::CostRoutingStatus;
use apeireth_core::kernel::{CapabilityId, SessionId};
use apeireth_core::model_profile::ModelProfile;
use apeireth_core::RELEASE_VERSION;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::device::device_control::DeviceControlStatus;
use crate::exec_pipeline::{OutputSchema, SchemaKind};

/// 产品标识 (中性工程事实: 自述身份, 不引任何第三方标识)。
pub const PRODUCT_NAME: &str = "apeireth";

/// 运行时角色标签: 桌面形态下后端进程以侧车身份被宿主监督。
pub const RUNTIME_ROLE_GATEWAY_SIDECAR: &str = "gateway-sidecar";

/// 自述身份一节: 产品名 / 版本 (workspace 单轴版本) / 运行时角色。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelfStatusIdentity {
    /// 产品名。
    pub product_name: String,
    /// 版本 (workspace 单轴版本值)。
    pub version: String,
    /// 运行时角色。
    pub runtime_role: String,
}

impl Default for SelfStatusIdentity {
    fn default() -> Self {
        Self {
            product_name: PRODUCT_NAME.to_string(),
            version: RELEASE_VERSION.to_string(),
            runtime_role: RUNTIME_ROLE_GATEWAY_SIDECAR.to_string(),
        }
    }
}

/// 能力名册一行: 开关名 + 当前生效值 (装配真实值, 非配置文本)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySwitch {
    /// 开关名 (与装配旋钮同名, snake_case)。
    pub name: String,
    /// 当前生效值。
    pub enabled: bool,
}

impl CapabilitySwitch {
    /// 一行能力名册。
    pub fn new(name: impl Into<String>, enabled: bool) -> Self {
        Self {
            name: name.into(),
            enabled,
        }
    }
}

// 记忆账本视图层已拆到 `memory_ledger_view`（尺寸基线：本文件只减不增）。
pub use crate::memory_ledger_view::{MemoryLedgerStats, RetrievalTuningMove};

/// 四滑杆调参状态 (体验参数生效值 + 预设名 + 自学习开关 + 性格节真值)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TuningStatus {
    /// 遗忘衰减强度倍率当前生效值。
    pub memory_fade: f64,
    /// 好奇强度倍率当前生效值。
    pub curiosity_strength: f64,
    /// 语气情绪饱和倍率当前生效值。
    pub tone_saturation: f64,
    /// 整合间隔 (回合数) 当前生效值。
    pub consolidation_cadence: f64,
    /// 当前取值命中的预设名 (`effortless` / `balanced` / `deep_memory` / `custom`)。
    pub preset: String,
    /// 自学习 (自动调参) 开关的当前生效值。
    pub self_learning: bool,
    /// 四杆锁定态 (locked = 成长沉积不再移动该杆; 解锁是显式动作)。
    /// `None` = 锁定态来源未接线/读取失败 (帧内显式 null + 原因, 不编)。
    pub locked: Option<TraitLockStatus>,
    /// 最近移动 (成长计既有账的实际移动, 新 → 旧, 最多 4 条)。
    /// `None` = 来源未接线/读取失败 (同上)。
    pub recent_moves: Option<Vec<TraitMoveStatus>>,
}

/// 四杆锁定态 (与调参面板四滑杆同名同序)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraitLockStatus {
    /// 遗忘衰减强度滑杆锁定态。
    pub memory_fade: bool,
    /// 好奇强度滑杆锁定态。
    pub curiosity_strength: bool,
    /// 语气情绪饱和度滑杆锁定态。
    pub tone_saturation: bool,
    /// 整合节奏滑杆锁定态。
    pub consolidation_cadence: bool,
}

/// 一根滑杆的最近一次实际移动 (性格节「最近移动」的一行)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraitMoveStatus {
    /// 滑杆 id (snake_case: `memory_fade` / `curiosity_strength` /
    /// `tone_saturation` / `consolidation_cadence`)。
    pub param: String,
    /// 滑杆中文名 (与调参面板同名)。
    pub label: String,
    /// 移动时刻 (epoch 毫秒)。
    pub at_epoch_ms: i64,
    /// 移动量 (带符号)。
    pub delta: f64,
    /// 移动原因 (成长计既有账的沉积原因原文)。
    pub reason: String,
}

/// 性格节附加真值: 锁定态 + 最近移动 (由成长计数据面探测口现读)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraitStatus {
    /// 四杆锁定态。
    pub locked: TraitLockStatus,
    /// 最近移动 (新 → 旧)。
    pub recent_moves: Vec<TraitMoveStatus>,
}

/// 预设名: 省心档 (遗忘更强 / 好奇更收敛 / 语气更淡 / 整合更慢)。
pub const PRESET_EFFORTLESS: &str = "effortless";
/// 预设名: 均衡档 (全基线取值)。
pub const PRESET_BALANCED: &str = "balanced";
/// 预设名: 深度记忆档 (遗忘更弱 / 好奇更强 / 语气更浓 / 整合更快)。
pub const PRESET_DEEP_MEMORY: &str = "deep_memory";
/// 预设名: 取值不落在任何命名预设上。
pub const PRESET_CUSTOM: &str = "custom";

/// 命名预设的四滑杆取值表 (与调参面板的预设按钮同一取值集)。
pub const DISPOSITION_PRESETS: &[(&str, [f64; 4])] = &[
    (PRESET_EFFORTLESS, [1.5, 0.75, 0.8, 2.0]),
    (PRESET_BALANCED, [1.0, 1.0, 1.0, 1.0]),
    (PRESET_DEEP_MEMORY, [0.5, 1.5, 1.2, 1.0]),
];

/// 由四滑杆取值推导预设名: 命中命名取值集回预设名, 否则 `custom`。
pub fn derive_tuning_preset(values: [f64; 4]) -> String {
    for (name, preset) in DISPOSITION_PRESETS {
        if preset
            .iter()
            .zip(values.iter())
            .all(|(expected, actual)| (expected - actual).abs() < 1e-9)
        {
            return (*name).to_string();
        }
    }
    PRESET_CUSTOM.to_string()
}

/// 预算一节: 回合轮数上限 / 单轮工具上限 + 配额两维真值 + 取值口径注记。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetStatus {
    /// 单回合轮数上限。
    pub max_rounds_per_turn: u64,
    /// 单轮工具调用上限。
    pub max_tool_calls_per_round: u64,
    /// 会话配额一节: token / 步数 / 花费三维真值 (上限/已耗/余量), 深度维
    /// 如实"暂无接口" (见 [`QuotaStatus`])。
    pub quota: QuotaStatus,
    /// 取值口径: `constant` = 读编译期常量; `configured` = 读可配置值。
    pub source: String,
    /// 口径注记 (可配置口径未接线时如实注明)。
    pub note: String,
}

/// 会话配额计量读数 (只回数字, 不回内容)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QuotaMeterReading {
    /// 会话累计已耗 token (回合计量 usage 累加)。
    pub consumed_tokens: u64,
    /// 会话累计已耗工具步数 (工具调用计数)。
    pub consumed_tool_steps: u64,
    /// 会话累计已耗花费 (有价轮按 `usage × 单价` 换算累加, 每侧向上取整)。
    pub consumed_cost_micros: u64,
    /// 无价轮数: cost 换算不出来的计量轮 (无单价行, 绝不编价)。> 0 时
    /// `consumed_cost_micros` 是部分计量口径, 自述如实标"部分计量"。
    pub unpriced_rounds: u64,
}

/// 会话配额计量探测口: 传会话 id, 回该会话账本的已耗读数 (失败回 `Err(原因)`)。
///
/// 配额计量是**会话级**的, 所以读数必须带会话作用域; 无作用域的调用对应字段
/// 显式 `null` + 原因 (0 装诚实)。
pub type SessionQuotaProbe =
    Arc<dyn Fn(SessionId) -> Result<QuotaMeterReading, String> + Send + Sync>;

/// 一维配额的自述行: 上限 / 已耗 / 余量 + 显示串 + 缺位原因。
///
/// 显示串口径: `"{已耗}/{上限}"`, 取不到或不设限的一侧显 `"—"`
/// (不设限显 "—"); 暂无接口的维度整行显 `"—"`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuotaDimensionStatus {
    /// 上限; `None` = 不设限 (显示 "—")。
    pub limit: Option<u64>,
    /// 已耗; `None` = 该作用域取不到计量 (显式 null + 原因)。
    pub consumed: Option<u64>,
    /// 余量 = 上限 − 已耗 (饱和); `None` = 不设限或取不到计量。
    pub remaining: Option<u64>,
    /// 显示串 (`"{已耗}/{上限}"`, 取不到的一侧显 "—"; 暂无接口整行 "—")。
    pub display: String,
    /// 已耗取不到 / 该维暂无接口 / 花费维部分计量标注的原因; 齐全时为 `None`。
    pub reason: Option<String>,
}

impl QuotaDimensionStatus {
    /// 计量真值行: 已耗已知; 上限 `None` = 不设限 (显示 "—")。
    pub fn metered(limit: Option<u64>, consumed: u64) -> Self {
        Self {
            limit,
            consumed: Some(consumed),
            remaining: limit.map(|cap| cap.saturating_sub(consumed)),
            display: format!("{}/{}", consumed, dash(limit)),
            reason: None,
        }
    }

    /// 无读数行: 上限已知, 该作用域取不到已耗 (显式 null + 原因)。
    pub fn without_reading(limit: Option<u64>, reason: impl Into<String>) -> Self {
        Self {
            limit,
            consumed: None,
            remaining: None,
            display: format!("—/{}", dash(limit)),
            reason: Some(reason.into()),
        }
    }

    /// 暂无接口行: 该维计量未实施 (如实整行 "—" + 原因, 不硬造数字)。
    pub fn not_wired(reason: impl Into<String>) -> Self {
        Self {
            limit: None,
            consumed: None,
            remaining: None,
            display: "—".to_string(),
            reason: Some(reason.into()),
        }
    }

    /// 换上会话账本读数 (上限不变, 已耗/余量/显示串按真值重算)。
    pub fn with_reading(self, consumed: u64) -> Self {
        Self::metered(self.limit, consumed)
    }

    /// 换上花费维读数 (上限不变, 已耗/余量/显示串按真值重算), 并按无价轮数
    /// 如实标"部分计量"。
    ///
    /// `unpriced_rounds > 0` 时已耗是**部分计量**口径 (无价轮未计入, 绝不
    /// 编价), 标注落在 `reason`; 全量口径 (0 无价轮) 无标注。
    pub fn with_cost_reading(self, consumed: u64, unpriced_rounds: u64) -> Self {
        let mut row = self.with_reading(consumed);
        if unpriced_rounds > 0 {
            row.reason = Some(quota_cost_partial_note(unpriced_rounds));
        }
        row
    }

    /// 换成"取不到读数" (上限不变, 已耗显式 null + 原因)。
    pub fn with_missing_reading(self, reason: impl Into<String>) -> Self {
        Self::without_reading(self.limit, reason)
    }
}

/// 不设限的显示占位 (上限/余量取不到时显 "—")。
fn dash(value: Option<u64>) -> String {
    value
        .map(|v| v.to_string())
        .unwrap_or_else(|| "—".to_string())
}

/// 历史注记 (文案退役): 花费维曾以 `QUOTA_COST_NOT_WIRED`
/// ("not wired: the cost dimension is not metered at the execution points yet
/// (the unit-price table source is ready, installing the metering is a
/// separate batch)") 如实标"暂无接口" —— A-P2 计量点装表接线后该文案退役,
/// 花费维改报真值; 无价轮的诚实标注改走 [`QUOTA_COST_PARTIAL`]。
pub const QUOTA_COST_PARTIAL: &str = "部分计量";

/// 花费维"部分计量"标注 (累计含无价轮时如实标注; 绝不编价)。
///
/// 无价轮 (无单价行的计量轮) 不计入成本列, 因此 `unpriced_rounds > 0` 时
/// 已耗数字是**部分**口径 —— 缺口照实说, 不把部分当全量。
pub fn quota_cost_partial_note(unpriced_rounds: u64) -> String {
    format!("{QUOTA_COST_PARTIAL}: {unpriced_rounds} 个计量轮无单价未计入 (绝不编价)")
}

/// 深度维"暂无接口"的原因 (0 装诚实: 调用树深度跟踪未实施)。
pub const QUOTA_DEPTH_NOT_WIRED: &str =
    "not wired: the depth dimension has no call-tree depth tracking yet";

/// 已耗取不到的原因: 配额计量是会话级, 本调用没有会话作用域。
pub const QUOTA_METER_SCOPE_REQUIRED: &str =
    "quota metering is session-scoped: this invocation carried no session scope";

/// 会话配额一节: 三维真值 + 深度维如实标注。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuotaStatus {
    /// token 维 (真值: 上限/已耗/余量)。
    pub tokens: QuotaDimensionStatus,
    /// 工具步数维 (真值: 上限/已耗/余量)。
    pub tool_steps: QuotaDimensionStatus,
    /// 花费维 (真值: 上限/已耗/余量; 含无价轮时已耗标"部分计量")。
    pub cost_micros: QuotaDimensionStatus,
    /// 深度维 —— 暂无接口 (调用树深度跟踪未实施)。
    pub depth: QuotaDimensionStatus,
}

/// 口径注记: 预算读编译期常量, 逐回合可配置口径尚未接线。
pub const BUDGET_SOURCE_CONSTANT_NOTE: &str =
    "read from compile-time constants; per-turn configurable override is not wired yet";

/// 工作区一节: 根路径 / 凭据存在性 (布尔, 不回显凭据本体)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceStatus {
    /// 工作区根路径。
    pub root: Option<String>,
    /// 凭据是否存在 (只回布尔; `None` = 探测缺位/失败 → 显式 null)。
    pub credentials_present: Option<bool>,
}

/// 单价「未提供」的显示标注 (零假价格: 缺省即明说未提供, 绝不编价)。
pub const PRICE_NOT_PROVIDED: &str = "未提供";

/// 内容流面一节 (J1+J2 自述): 订阅数 / 最近拉取 + 生命周期真值, 如实。
///
/// 「最近拉取」逐维 `None` = 尚无拉取 (显式 null + `last_pull_note` 如实注
/// 「尚无拉取记录」), 绝不编造时间或条数; 探测口缺位/失败 = 全维显式 null +
/// `reason` (0 装诚实)。
///
/// J2 生命周期字段 (字段级 `None` = 缺源显式 null): 条目级去重数 / 投摄取队列
/// 数 / 注意力截断数 / 订阅健康 (degraded / 退避)。**只报数字不回内容**。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentFeedStatus {
    /// 订阅数 (`feeds.json` 真值计数)。
    pub feeds: Option<u64>,
    /// 最近拉取时刻 (epoch 毫秒); `None` = 尚无拉取。
    pub last_pull_at_ms: Option<i64>,
    /// 最近拉取成败 (true = 全部订阅拉取成功)。
    pub last_pull_ok: Option<bool>,
    /// 最近拉取入记忆流的条数。
    pub last_pull_ingested: Option<u64>,
    /// 最近拉取备注 (成败真因 / 「尚无拉取记录」)。
    pub last_pull_note: Option<String>,
    /// 条目级去重累计 (J2: 同条目重复拉取跳过, `None` = 缺源)。
    pub dedup_skipped: Option<u64>,
    /// 投摄取队列累计 (J2: 交进待摄取队列的条目真值, `None` = 缺源)。
    pub ingest_queued: Option<u64>,
    /// 注意力上限截断累计 (J2: 超单次/每日上限如实截断, `None` = 缺源)。
    pub attention_truncated: Option<u64>,
    /// 标 degraded 的订阅数 (J2: 连续失败达阈值, `None` = 缺源)。
    pub degraded_feeds: Option<u64>,
    /// 退避中的订阅数 (J2: 拉取失败退避窗口内, `None` = 缺源)。
    pub backing_off_feeds: Option<u64>,
    /// 探测缺位/失败的原因 (全维就位时 `None`)。
    pub reason: Option<String>,
}

impl ContentFeedStatus {
    /// 不可用 (探测口缺位/失败): 全维显式 null + 原因。
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            feeds: None,
            last_pull_at_ms: None,
            last_pull_ok: None,
            last_pull_ingested: None,
            last_pull_note: None,
            dedup_skipped: None,
            ingest_queued: None,
            attention_truncated: None,
            degraded_feeds: None,
            backing_off_feeds: None,
            reason: Some(reason.into()),
        }
    }
}

/// 双引擎关怀一节 (V3-a 自述): 规则数 / 下次触发 / 最近关怀真值, 如实。
///
/// 「最近关怀」`None` = 尚无关怀 (显式 null + `last_care_note` 如实注「尚无
/// 关怀记录」), 绝不编造时间或判定; 探测口缺位/失败 = 全维显式 null +
/// `reason` (0 装诚实)。
///
/// 心跳态两列 (V3-a 收尾一针: 常驻心跳关怀生产者): `None` = 心跳生产者
/// 未接线/未运行 (如实), 绝不冒充在跑。
///
/// 天气提醒计数 (K 轴 L2 提醒侧消费端): `weather-signals.jsonl` 真值计数
/// (只交信号的天气提醒信号总数), 与提醒侧消费面同一本信号账。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CareStatus {
    /// 关怀规则数 (`care_rules.json` 真值计数)。
    pub rules: Option<u64>,
    /// 下次触发时刻 (epoch 毫秒); `None` = 无启用规则 (如实)。
    pub next_trigger_at_ms: Option<i64>,
    /// 最近关怀时刻 (epoch 毫秒); `None` = 尚无关怀。
    pub last_care_at_ms: Option<i64>,
    /// 最近关怀判定真值 (发起/说不 + 判定档 + 理由)。
    pub last_care_note: Option<String>,
    /// 心跳态: 下次 poll 时刻 (epoch 毫秒); `None` = 心跳生产者未运行/未接线。
    pub heartbeat_next_poll_at_ms: Option<i64>,
    /// 心跳态: 最近心跳命中 (关怀发起) 时刻; `None` = 尚无心跳命中。
    pub heartbeat_last_hit_at_ms: Option<i64>,
    /// 天气提醒计数 (`weather-signals.jsonl` 信号真值计数; `None` = 信号账读不到)。
    pub weather_reminders: Option<u64>,
    /// 探测缺位/失败的原因 (全维就位时 `None`)。
    pub reason: Option<String>,
}

impl CareStatus {
    /// 不可用 (探测口缺位/失败): 全维显式 null + 原因。
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            rules: None,
            next_trigger_at_ms: None,
            last_care_at_ms: None,
            last_care_note: None,
            heartbeat_next_poll_at_ms: None,
            heartbeat_last_hit_at_ms: None,
            weather_reminders: None,
            reason: Some(reason.into()),
        }
    }
}

/// 语音一节 (V3-b 发声面自述): 账本数 / 最近说话 / 嗓名, 真值如实。
///
/// 「最近说话」`None` = 尚无语句 (显式 null + `note` 如实注「尚无语句记录」),
/// 绝不编造时刻或文本; `last_text_excerpt` 是**有界摘录** (脱敏口径: 全文
/// 只在语句账本 `spoken.jsonl`, 自述只给摘录); 探测口缺位/失败 = 全维
/// 显式 null + `reason` (0 装诚实)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceStatus {
    /// 语句账本数 (`spoken.jsonl` 真值计数)。
    pub utterances: Option<u64>,
    /// 最近说话时刻 (epoch 毫秒); `None` = 尚无语句。
    pub last_spoken_at_ms: Option<i64>,
    /// 最近说话摘录 (有界; 全文只在语句账本)。
    pub last_text_excerpt: Option<String>,
    /// 最近用嗓 (系统默认嗓记 `system-default`)。
    pub voice: Option<String>,
    /// 备注 (「尚无语句记录」/ 坏行跳过如实注)。
    pub note: Option<String>,
    /// 探测缺位/失败的原因 (全维就位时 `None`)。
    pub reason: Option<String>,
}

impl VoiceStatus {
    /// 不可用 (探测口缺位/失败): 全维显式 null + 原因。
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            utterances: None,
            last_spoken_at_ms: None,
            last_text_excerpt: None,
            voice: None,
            note: None,
            reason: Some(reason.into()),
        }
    }
}

/// 备份一节 (G5 备份恢复): 最近备份时间 / 清单件数真值, 如实。
///
/// 「最近备份」`None` = 尚无备份 (显式 null + `reason` 如实注「尚无备份记录」),
/// 绝不编造时间或件数; 探测口缺位/失败 = 全维显式 null + `reason` (0 装诚实)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupStatus {
    /// 最近备份时刻 (epoch 毫秒); `None` = 尚无备份。
    pub last_backup_at_ms: Option<i64>,
    /// 该次备份的清单件数; `None` = 尚无备份。
    pub manifest_entries: Option<u64>,
    /// 缺位/失败/尚无备份的原因 (真值齐全时 `None`)。
    pub reason: Option<String>,
}

impl BackupStatus {
    /// 不可用 (探测口缺位/失败): 全维显式 null + 原因。
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            last_backup_at_ms: None,
            manifest_entries: None,
            reason: Some(reason.into()),
        }
    }
}

/// 任务簿一节 (V6-a 任务编排持久队列): 任务队列分态计数真值。
///
/// 计数与 `tool.task_board` 工具面同一队列同一真值源 (只回计数不回任务正文);
/// 探测口缺位/失败 = 全维显式 null + `reason` (0 装诚实, 不编数)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskBoardStatus {
    /// `pending` (入队待派) 计数。
    pub pending: u64,
    /// `assigned` (已指派并已唤醒) 计数。
    pub assigned: u64,
    /// `running` (执行段在跑) 计数。
    pub running: u64,
    /// `done` (完成) 计数。
    pub done: u64,
    /// `failed` (人判失败) 计数。
    pub failed: u64,
    /// `blocked` (外部等待/熔断) 计数。
    pub blocked: u64,
}

/// 自述提案节的「最近提案」一行 (只回 id / kind / 标题 / 状态, 不回正文)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalStatusEntry {
    /// 提案 id。
    pub id: String,
    /// 提案 kind 标签 (`tidy` / `fix_suggest` / `summarize` / `list`)。
    pub kind: String,
    /// 标题首行 (人读)。
    pub title: String,
    /// 状态标签 (`pending` / `accepted` / `rejected`)。
    pub state: String,
}

/// 自述提案节的「最近推送」一行 (只回提案 id / 渠道 / 时刻 / 推送结果, 不回卡正文)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalPushStatusEntry {
    /// 提案 id (推送幂等键)。
    pub proposal_id: String,
    /// 推送到的 IM 渠道 id。
    pub channel_id: String,
    /// 推送时刻 (epoch 毫秒)。
    pub at_ms: i64,
    /// 推送结果标签 (`pushed` / `failed`; `failed` = 留账待重试, 如实)。
    pub outcome: String,
}

/// 提案一节 (自主度第一小步 · 边角料提案面): 待审数 / 分态计数 / 最近提案。
///
/// 计数与 `tool.proposal_draft` 工具面同一本提案账 (`proposals.jsonl`) 同一
/// 真值源; 「最近提案」有界 (5 条), 只回 id/kind/标题/状态不回 detail/payload
/// 正文; 探测口缺位/失败 = 全维显式 null + `reason` (0 装诚实, 不编数)。
///
/// **推送列** (自主度提案面接线批): `pushed` / `recent_pushes` 与提案卡推送账
/// (`proposal-pushes.jsonl`) 同一真值源 —— 已推送数 / 最近推送随自述上报,
/// 失败推送照实标 `failed` (留账待重试, 不藏)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalStatus {
    /// 待审数 (已入账还没点头/摇头)。
    pub pending: u64,
    /// 人点头 (标记 accepted) 计数 —— accepted 只是账面标记, 不是执行。
    pub accepted: u64,
    /// 人摇头 (标记 rejected) 计数。
    pub rejected: u64,
    /// 提案总数 (草稿行计数)。
    pub total: u64,
    /// 最近提案 (有界, 新在前)。
    pub recent: Vec<ProposalStatusEntry>,
    /// 已推送数 (成功推到 IM 渠道的去重提案 id 计数; 幂等键 = 提案 id)。
    /// `#[serde(default)]`: 旧自述载荷无此列照读 (存量 = 0)。
    #[serde(default)]
    pub pushed: u64,
    /// 最近推送 (有界, 新在前; 含 `failed` 留账行, 如实)。
    /// `#[serde(default)]`: 旧自述载荷无此列照读 (存量 = 空)。
    #[serde(default)]
    pub recent_pushes: Vec<ProposalPushStatusEntry>,
    /// 备注 (「尚无提案记录」/ 坏行跳过如实注)。
    pub note: Option<String>,
    /// 探测缺位/失败的原因 (全维就位时 `None`)。
    pub reason: Option<String>,
}

impl ProposalStatus {
    /// 不可用 (探测口缺位/失败): 全维显式 null + 原因。
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            pending: 0,
            accepted: 0,
            rejected: 0,
            total: 0,
            recent: Vec::new(),
            pushed: 0,
            recent_pushes: Vec::new(),
            note: None,
            reason: Some(reason.into()),
        }
    }
}

/// 创作节逐操作计数一行 (闭合词表成员名 + 计数; 0 = 本进程未跑过该操作, 如实)。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageOpsOpCount {
    /// 操作词表成员名 (`resize` / `crop` / `rotate` / ...)。
    pub operation: String,
    /// 应用计数。
    pub count: u64,
}

/// 创作节 (创作工具面 W1 · 图像操作): 最近操作计数, 只计数不回路径, 如实。
///
/// 计数口径与 `tool.image_ops` 工具面同一实例同一真值源: **进程内计数**
/// (本进程生命周期内的最近操作, 重启清零), 不落账不造账 —— 0 装诚实,
/// 不拿磁盘账冒充。`last_op_at_ms` / `last_call_ops` 为 `None` = 尚无操作
/// (显式 null + `note` 如实注「尚无操作记录」), 绝不编造时刻或序列;
/// `denied` = 拒绝即帧次数 (错误即帧也计数, 不藏拒绝)。脱敏口径: 本节
/// **零路径零文件名**, 只回计数与操作名。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageOpsStatus {
    /// 工具调用次数 (成败都计)。
    pub calls: u64,
    /// 成功应用的操作步数累计 (管道内每一步都计)。
    pub ops_applied: u64,
    /// 拒绝即帧次数。
    pub denied: u64,
    /// 闭合词表逐操作计数 (行序 = 词表序; 0 = 未跑, 不省行)。
    pub by_operation: Vec<ImageOpsOpCount>,
    /// 最近一次成功操作时刻 (epoch 毫秒); `None` = 尚无操作。
    pub last_op_at_ms: Option<i64>,
    /// 最近一调用的操作序列 (`resize -> grayscale` 词表名, 有界)。
    pub last_call_ops: Option<String>,
    /// 备注 (「尚无操作记录」/ 计数口径注记)。
    pub note: Option<String>,
    /// 探测缺位/失败的原因 (真值齐全时 `None`)。
    pub reason: Option<String>,
}

impl ImageOpsStatus {
    /// 不可用 (探测口缺位/失败): 全维显式 null/零 + 原因。
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            calls: 0,
            ops_applied: 0,
            denied: 0,
            by_operation: Vec::new(),
            last_op_at_ms: None,
            last_call_ops: None,
            note: None,
            reason: Some(reason.into()),
        }
    }
}

/// 创作节逐操作计数一行 (W2 文档操作: 闭合词表成员名 + 计数; 0 = 本进程未跑过,
/// 如实不省行)。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocOpsOpCount {
    /// 操作词表成员名 (`pdf_info` / `pdf_extract_text` / ...)。
    pub operation: String,
    /// 成功执行计数。
    pub count: u64,
}

/// 创作节 (创作工具面 W2 · 文档操作): 进程内计数, 只计数不回路径, 如实。
///
/// 计数口径与 `tool.doc_ops` 工具面同一实例同一真值源: **进程内计数**
/// (重启清零), 不落账不造账。`last_op_at_ms` / `last_action` 为 `None` =
/// 尚无操作 (显式 null + `note` 如实注「尚无操作记录」), 绝不编造时刻或动作;
/// `denied` = 拒绝即帧次数 (错误即帧也计数, 不藏拒绝); `pages_touched` =
/// 读+写处理页数累计。脱敏口径: 本节**零路径零文件名**, 只回计数与操作名。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocOpsStatus {
    /// 工具调用次数 (成败都计)。
    pub calls: u64,
    /// 读+写处理页数累计。
    pub pages_touched: u64,
    /// 拒绝即帧次数。
    pub denied: u64,
    /// 闭合词表逐操作成功计数 (行序 = 词表序; 0 = 未跑, 不省行)。
    pub by_operation: Vec<DocOpsOpCount>,
    /// 最近一次成功操作时刻 (epoch 毫秒); `None` = 尚无操作。
    pub last_op_at_ms: Option<i64>,
    /// 最近一次成功动作词表名; `None` = 尚无操作。
    pub last_action: Option<String>,
    /// 备注 (「尚无操作记录」/ 计数口径注记)。
    pub note: Option<String>,
    /// 探测缺位/失败的原因 (真值齐全时 `None`)。
    pub reason: Option<String>,
}

impl DocOpsStatus {
    /// 不可用 (探测口缺位/失败): 全维显式 null/零 + 原因。
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            calls: 0,
            pages_touched: 0,
            denied: 0,
            by_operation: Vec::new(),
            last_op_at_ms: None,
            last_action: None,
            note: None,
            reason: Some(reason.into()),
        }
    }
}

/// 创作节逐操作计数一行 (K3 表格操作: 闭合词表动作名 + 计数; 0 = 本进程未
/// 跑过, 如实不省行)。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SheetOpsOpCount {
    /// 操作词表动作名 (`read` / `summary` / `filter` / `sort` / `aggregate` /
    /// `formula` / `export_csv` / `export_xlsx`)。
    pub operation: String,
    /// 成功执行计数。
    pub count: u64,
}

/// 创作节 (创作工具面 K3 · 表格操作): 进程内计数, 只计数不回路径, 如实。
///
/// 计数口径与 `tool.sheet_ops` 工具面同一实例同一真值源: **进程内计数**
/// (重启清零), 不落账不造账。`last_op_at_ms` / `last_action` 为 `None` =
/// 尚无操作 (显式 null + `note` 如实注「尚无操作记录」), 绝不编造时刻或动作;
/// `denied` = 拒绝即帧次数 (错误即帧也计数, 不藏拒绝); `rows_touched` =
/// 读+写处理行数累计。脱敏口径: 本节**零路径零文件名**, 只回计数与操作名。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SheetOpsStatus {
    /// 工具调用次数 (成败都计)。
    pub calls: u64,
    /// 读+写处理行数累计。
    pub rows_touched: u64,
    /// 拒绝即帧次数。
    pub denied: u64,
    /// 闭合词表逐动作成功计数 (行序 = 词表序; 0 = 未跑, 不省行)。
    pub by_operation: Vec<SheetOpsOpCount>,
    /// 最近一次成功操作时刻 (epoch 毫秒); `None` = 尚无操作。
    pub last_op_at_ms: Option<i64>,
    /// 最近一次成功动作词表名; `None` = 尚无操作。
    pub last_action: Option<String>,
    /// 备注 (「尚无操作记录」/ 计数口径注记)。
    pub note: Option<String>,
    /// 探测缺位/失败的原因 (真值齐全时 `None`)。
    pub reason: Option<String>,
}

impl SheetOpsStatus {
    /// 不可用 (探测口缺位/失败): 全维显式 null/零 + 原因。
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            calls: 0,
            rows_touched: 0,
            denied: 0,
            by_operation: Vec::new(),
            last_op_at_ms: None,
            last_action: None,
            note: None,
            reason: Some(reason.into()),
        }
    }
}

/// 在场节 (K 轴 L1 在场感知): 设备数 / 白名单数 / 最近感知, 真值如实。
///
/// 计数与 `tool.device_sense` 工具面同一本设备表 (`devices.json`) 同一真值源:
/// `devices` = 表内设备总数 (含停用), `whitelisted` = 白名单数 (enabled 才可
/// 感知, 白名单制默认零设备)。「最近感知」两列 `None` = 尚无感知 (显式 null,
/// 不编时间); 来源是**进程内计数** (重启清零, `note` 如实注明)。设备表拒开
/// (StoredDoc 语义) = 全维显式 null + `reason` (0 装诚实, 不编数)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DevicePresenceStatus {
    /// 设备数 (`devices.json` 表内行数, 含停用)。
    pub devices: Option<u64>,
    /// 白名单数 (表内且 `enabled`, 底线①: 显式启用才可感知)。
    pub whitelisted: Option<u64>,
    /// 最近感知时刻 (epoch 毫秒); `None` = 尚无感知。
    pub last_sense_at_ms: Option<i64>,
    /// 最近感知摘要 (有界: 几台/什么源; `None` = 尚无感知)。
    pub last_sense_note: Option<String>,
    /// 备注 (缺省表注 / 进程内计数口径注记)。
    pub note: Option<String>,
    /// 探测缺位/失败的原因 (真值齐全时 `None`)。
    pub reason: Option<String>,
}

impl DevicePresenceStatus {
    /// 不可用 (探测口缺位/失败): 全维显式 null + 原因。
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            devices: None,
            whitelisted: None,
            last_sense_at_ms: None,
            last_sense_note: None,
            note: None,
            reason: Some(reason.into()),
        }
    }
}

/// 人格层一节 (V4-a 人格记忆自述): 信念/边界/偏好/张力计数 + 最后整编真值,
/// 只计数不回内容 (与自述面「有多少不说是什么」同口径)。
///
/// 计数与人格层落库面同一真值源 (`persona_memory_*` 分表的自述节); 「最后整编」
/// `None` = 尚未整编 (显式 null, 不编时间, epoch 秒口径与人格层真值一致);
/// 探测口缺位/失败 = 全维显式 null + `reason` (0 装诚实, 不编数)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaStatusSection {
    /// 信念数 (core_beliefs 真值计数)。
    pub core_beliefs: u64,
    /// 边界数 (boundaries 真值计数)。
    pub boundaries: u64,
    /// 偏好数 (preferences 真值计数)。
    pub preferences: u64,
    /// 张力数 (已标注人格张力计数)。
    pub tensions: u64,
    /// 最后整编真值 (epoch 秒, 与人格层真值同口径); `None` = 尚未整编。
    pub last_consolidation_at: Option<i64>,
    /// 缺位/失败的原因 (真值齐全时 `None`)。
    pub reason: Option<String>,
}

/// 域内计数 (W3-① 域节): 会话/记忆/受保护记忆的真值计数。
///
/// 逐维 `None` = 该维探测口缺位/失败 → 显式 null + `reason` (不编数)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainCounts {
    /// 域内会话数 (该域会话库的真值计数)。
    pub sessions: Option<u64>,
    /// 域内记忆数 (该域记忆库的真值计数)。
    pub memories: Option<u64>,
    /// 域内受保护记忆数。
    pub protected: Option<u64>,
    /// 缺位/失败的原因 (全维就位时 `None`)。
    pub reason: Option<String>,
}

impl DomainCounts {
    /// 计数不可用 (探测口缺位/失败): 全维显式 null + 原因。
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            sessions: None,
            memories: None,
            protected: None,
            reason: Some(reason.into()),
        }
    }
}

/// 用户域一节 (W3-① 自述域节): 当前域 id / 显示名 / 角色 / 创建时刻 + 域内计数。
///
/// 「当前域」= 本自述来源所组装的那个用户域的数据面 (星群形态下每户伙伴实例
/// 自述自己的域), 如实报告, 不跨域代答。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainStatus {
    /// 域 id (默认用户域 = `default`)。
    pub user_id: String,
    /// 域显示名。
    pub display_name: String,
    /// 域角色 (`owner` / `member`)。
    pub role: String,
    /// 域创建时刻 (epoch 毫秒)。
    pub created_at: i64,
    /// 域内计数 (真值; 缺探测口的维显式 null + 原因)。
    pub counts: DomainCounts,
}

/// 模型画像事实显式 unknown 的原因 (未知模型: 不编造任何事实)。
pub const MODEL_PROFILE_UNKNOWN: &str = "model is not in the profile table: its facts are \
     explicitly unknown (never invented)";

/// 一份完整结构化自述的来源数据 (工具只负责投影成 JSON)。
#[derive(Debug, Clone, PartialEq)]
pub struct SelfStatusSnapshot {
    /// 身份一节。
    pub identity: SelfStatusIdentity,
    /// 能力名册 (当前生效开关全表)。
    pub capabilities: Vec<CapabilitySwitch>,
    /// 记忆账本计数; `None` = 来源不可用 (帧内显式 null + 原因)。
    pub memory_ledger: Option<MemoryLedgerStats>,
    /// 问责计数（只读子对象数据；**不新增顶层节**）。
    pub accountability: Option<crate::accountability_view::AccountabilityStats>,
    /// 问责计数缺失原因。
    pub accountability_reason: Option<String>,
    /// 记忆账本不可用的原因; 可用时为 `None`。
    pub memory_ledger_reason: Option<String>,
    /// 调参状态 (含性格节真值: 四杆锁定态 + 最近移动)。
    pub tuning: Option<TuningStatus>,
    /// 调参节缺源/局部缺失的原因 (来源缺席或性格节锁定态/最近移动没测到);
    /// 齐全时为 `None`。
    pub tuning_reason: Option<String>,
    /// 预算。
    pub budget: Option<BudgetStatus>,
    /// 预算不可用的原因; 可用时为 `None`。
    pub budget_reason: Option<String>,
    /// 工作区。
    pub workspace: Option<WorkspaceStatus>,
    /// 工作区不可用的原因; 可用时为 `None`。
    pub workspace_reason: Option<String>,
    /// 模型画像一节 (生效模型的能力事实 + 单价, 缺省如实未提供)。
    pub model: Option<ModelProfile>,
    /// 模型画像不可用的原因; 可用时为 `None` (未知模型的显式 unknown 说明
    /// 在渲染时按 [`MODEL_PROFILE_UNKNOWN`] 给出)。
    pub model_reason: Option<String>,
    /// 用户域一节 (W3-①: 当前域 id/角色/域内计数)。
    pub domain: Option<DomainStatus>,
    /// 用户域一节不可用的原因; 可用时为 `None`。
    pub domain_reason: Option<String>,
    /// 内容流面一节 (J1: 订阅数/最近拉取)。
    pub content_feed: Option<ContentFeedStatus>,
    /// 内容流面一节不可用的原因; 可用时为 `None`。
    pub content_feed_reason: Option<String>,
    /// 双引擎关怀一节 (V3-a: 规则数/下次触发/最近关怀真值)。
    pub care: Option<CareStatus>,
    /// 关怀一节不可用的原因; 可用时为 `None`。
    pub care_reason: Option<String>,
    /// 备份一节 (G5: 最近备份时间/清单件数真值)。
    pub backup: Option<BackupStatus>,
    /// 备份一节不可用的原因; 可用时为 `None`。
    pub backup_reason: Option<String>,
    /// 语音一节 (V3-b: 账本数/最近说话/嗓名)。
    pub voice: Option<VoiceStatus>,
    /// 语音一节不可用的原因; 可用时为 `None`。
    pub voice_reason: Option<String>,
    /// 任务簿一节 (V6-a: 任务队列分态计数真值)。
    pub task_board: Option<TaskBoardStatus>,
    /// 任务簿一节不可用的原因; 可用时为 `None`。
    pub task_board_reason: Option<String>,
    /// 人格层一节 (V4-a: 信念/边界/偏好/张力计数 + 最后整编真值)。
    pub persona: Option<PersonaStatusSection>,
    /// 人格层一节不可用的原因; 可用时为 `None`。
    pub persona_reason: Option<String>,
    /// 提案一节 (自主度第一小步: 待审数/分态计数/最近提案)。
    pub proposal: Option<ProposalStatus>,
    /// 提案一节不可用的原因; 可用时为 `None`。
    pub proposal_reason: Option<String>,
    /// 成本分级路由一节 (H-P2: 当前档/选定模型/回退标注真值)。
    pub cost_routing: Option<CostRoutingStatus>,
    /// 成本分级路由一节不可用的原因; 可用时为 `None`。
    pub cost_routing_reason: Option<String>,
    /// 创作节 (创作工具面 W1: 最近操作计数, 只计数不回路径)。
    pub image_ops: Option<ImageOpsStatus>,
    /// 创作节不可用的原因; 可用时为 `None`。
    pub image_ops_reason: Option<String>,
    /// 创作节 (创作工具面 W2 · 文档操作: 进程内计数, 只计数不回路径)。
    pub doc_ops: Option<DocOpsStatus>,
    /// 创作文档操作节不可用的原因; 可用时为 `None`。
    pub doc_ops_reason: Option<String>,
    /// 在场节 (K 轴 L1 在场感知: 设备数/白名单数/最近感知)。
    pub device_presence: Option<DevicePresenceStatus>,
    /// 在场节不可用的原因; 可用时为 `None`。
    pub device_presence_reason: Option<String>,
    /// 创作节 (创作工具面 K3 · 表格操作: 进程内计数, 只计数不回路径)。
    pub sheet_ops: Option<SheetOpsStatus>,
    /// 创作表格操作节不可用的原因; 可用时为 `None`。
    pub sheet_ops_reason: Option<String>,
    /// 设备受控执行节 (K 轴 L3/L4: 控制计数/最近动作真值, 与 `tool.device_control`
    /// 工具面同账同源)。
    pub device_control: Option<DeviceControlStatus>,
    /// 设备受控执行节不可用的原因; 可用时为 `None`。
    pub device_control_reason: Option<String>,
    /// 生物机制族只读聚合 (W3.2 装配面): 挂在既有 `capabilities` 节内的
    /// `bio_mechanisms` 子对象 (14 条机制逐格读数; **不新增顶层节**)。
    ///
    /// `None` = 本组装面未挂该观测面 → 渲染面**省略**该键, 旧帧形状不变
    /// (兼容旧消费者); `Some` = 观测面已挂, 未启用时子对象恒在场并显式给出
    /// 「未启用」原因。
    pub bio_mechanisms: Option<BioMechanismsStatus>,
}

impl SelfStatusSnapshot {
    /// 只有身份与名册的最小自述 (其余字段显式 null + 原因)。
    pub fn minimal(identity: SelfStatusIdentity, capabilities: Vec<CapabilitySwitch>) -> Self {
        Self {
            identity,
            capabilities,
            memory_ledger: None,
            accountability: None,
            accountability_reason: None,
            memory_ledger_reason: Some(DATA_PROBE_NOT_WIRED.to_string()),
            tuning: None,
            tuning_reason: Some(DATA_PROBE_NOT_WIRED.to_string()),
            budget: None,
            budget_reason: Some(DATA_PROBE_NOT_WIRED.to_string()),
            workspace: None,
            workspace_reason: Some(DATA_PROBE_NOT_WIRED.to_string()),
            model: None,
            model_reason: Some(DATA_PROBE_NOT_WIRED.to_string()),
            domain: None,
            domain_reason: Some(DATA_PROBE_NOT_WIRED.to_string()),
            content_feed: None,
            content_feed_reason: Some(DATA_PROBE_NOT_WIRED.to_string()),
            care: None,
            care_reason: Some(DATA_PROBE_NOT_WIRED.to_string()),
            backup: None,
            backup_reason: Some(DATA_PROBE_NOT_WIRED.to_string()),
            voice: None,
            voice_reason: Some(DATA_PROBE_NOT_WIRED.to_string()),
            task_board: None,
            task_board_reason: Some(DATA_PROBE_NOT_WIRED.to_string()),
            persona: None,
            persona_reason: Some(DATA_PROBE_NOT_WIRED.to_string()),
            proposal: None,
            proposal_reason: Some(DATA_PROBE_NOT_WIRED.to_string()),
            cost_routing: None,
            cost_routing_reason: Some(DATA_PROBE_NOT_WIRED.to_string()),
            image_ops: None,
            image_ops_reason: Some(DATA_PROBE_NOT_WIRED.to_string()),
            doc_ops: None,
            doc_ops_reason: Some(DATA_PROBE_NOT_WIRED.to_string()),
            device_presence: None,
            device_presence_reason: Some(DATA_PROBE_NOT_WIRED.to_string()),
            sheet_ops: None,
            sheet_ops_reason: Some(DATA_PROBE_NOT_WIRED.to_string()),
            device_control: None,
            device_control_reason: Some(DATA_PROBE_NOT_WIRED.to_string()),
            // 生物机制观测面未挂: 子对象缺省 (旧帧形状不变); 挂上观测面由装配根
            // 注入 [`BioMechanismsStatus`] (W3.2)。
            bio_mechanisms: None,
        }
    }
}

/// 统一的「来源未接线」原因 (0 装诚实: 显式说没测到, 不编造)。
pub const DATA_PROBE_NOT_WIRED: &str = "status source not wired in this composition";

/// 自述数据来源: 组装根注入真实生效值与探测口, 工具只做只读投影。
pub trait SelfStatusSource: Send + Sync {
    /// 采集一份自述快照 (调用时的生效值)。
    fn snapshot(&self) -> SelfStatusSnapshot;

    /// 会话作用域内的自述采集: `session` = 派发本调用的回合所属会话。
    ///
    /// 只有会话作用域的读数 (配额计量的已耗列) 依赖它; 默认实现退化为
    /// [`SelfStatusSource::snapshot`], 对应字段显式 `null` + 原因。
    fn snapshot_in_session(&self, session: Option<SessionId>) -> SelfStatusSnapshot {
        let _ = session;
        self.snapshot()
    }
}

/// data 系探测口 (记忆账本 / 凭据存在性): 每次自述现探测, 失败回 `Err(原因)`。
///
/// 失败**不**让整帧失败: 对应字段显式 null + 原因 (见 [`render_snapshot`])。
pub type StatusProbe<T> = Arc<dyn Fn() -> Result<T, String> + Send + Sync>;

/// 自述帧冻结节合同: 节的全集与顺序 (显式节序), 演进须注明理由。
///
/// [`render_snapshot`] 的渲染面、[`SelfStatusTool::output_schema`] 的 require
/// 清单、fixture 的在场面三侧同表 —— 缺任何一节 = 合同违约 (断言见
/// `the_frozen_section_contract_is_the_single_source_for_render_and_schema`)。
///
/// **W3.2 生物机制族**不新增顶层节: 14 条机制的只读聚合挂在既有
/// `capabilities` 节内的 [`CAPABILITIES_BIO_MECHANISMS`] 子对象上, 本表仍 21 节。
pub const SELF_STATUS_SECTIONS: &[&str] = &[
    "identity",
    "capabilities",
    "memory_ledger",
    "tuning",
    "budget",
    "workspace",
    // H-P1: 模型画像节入七节合同 (形状冻结的演进, 附理由: 路由/成本依据面)。
    "model",
    // W3-①: 用户域节入八节合同 (多户数据隔离的自述域面)。
    "domain",
    // J1: 内容流节入九节合同 (订阅数/最近拉取的如实自述)。
    "content_feed",
    // V3-a: 关怀节入十节合同 (规则数/下次触发/最近关怀真值)。
    "care",
    // G5: 备份节入十一节合同 (最近备份时间/清单件数真值)。
    "backup",
    // V3-b: 语音节入十二节合同 (账本数/最近说话/嗓名真值)。
    "voice",
    // V6-a: 任务簿节入十三节合同 (任务队列分态计数真值)。
    "task_board",
    // V4-a: 人格节入十四节合同 (形状冻结的演进, 附理由: 人格记忆层的计数
    // 真值与最后整编真值, 只计数不回内容)。
    "persona",
    // 自主度第一小步: 提案节入十五节合同 (形状冻结的演进, 附理由: 边角料
    // 提案面的待审数/分态计数/最近提案真值 —— 人审面要知道有多少提案待看)。
    "proposal",
    // H-P2: 成本分级路由节入十六节合同 (形状冻结的演进, 附理由: 成本分级
    // 路由的当前档/选定模型/回退标注真值自述化 —— 选型可解释、回退不粉饰)。
    "cost_routing",
    // 创作工具面 W1: 创作节入十七节合同 (形状冻结的演进, 附理由: 图像操作
    // 的最近操作计数自述化 —— agent 创作面的用量如实可见; 只计数不回路径)。
    "image_ops",
    // 创作工具面 W2: 文档操作节入十八节合同 (形状冻结的演进, 附理由: 文档
    // 读写族的进程内计数自述化 —— 读了/写了多少页、拒了多少次如实可见;
    // 只计数不回路径, 与 image_ops 创作节同口径)。
    "doc_ops",
    // K 轴 L1 在场感知: 在场节入十九节合同 (形状冻结的演进, 附理由: 物理在场
    // 骨架的设备数/白名单数/最近感知真值自述化 —— 白名单制默认零设备的实况
    // 要让 agent 自己看得见; 最近感知为进程内计数口径, 如实注明)。
    "presence",
    // 创作工具面 K3: 表格操作节入二十节合同 (形状冻结的演进, 附理由: 表格
    // 读写族的进程内计数自述化 —— 读了/写了多少行、拒了多少次如实可见;
    // 只计数不回路径, 与 image_ops/doc_ops 创作节同口径)。
    "sheet_ops",
    // K 轴 L3/L4 受控执行: 设备受控执行节入二十一节合同 (形状冻结的演进,
    // 附理由: 设备动作的控制计数/最近动作真值自述化 —— 执行了多少动作、
    // 最近动了什么/谁批的要让 agent 自己看得见; 只回计数与动作标签, 不回
    // 参数正文, 与在场节同目录同口径)。
    "device_control",
];

/// `capabilities` 节内的生物机制只读子对象键名 (W3.2 装配面)。
///
/// **不是顶层节**: 21 节冻结合同不变, 14 条机制统一挂在既有 `capabilities`
/// 节内（台账 W3 计划「观测聚合」策略）。
pub const CAPABILITIES_BIO_MECHANISMS: &str = "bio_mechanisms";

/// 问责计数子对象（`capabilities` 节内；**不新增顶层节**，顶层仍 21 节）。
pub const CAPABILITIES_ACCOUNTABILITY: &str = "accountability";

/// 生物机制自述开关的 env 名: 只认显式 `"1"`，其余任何取值 = 关 (fail-closed)。
pub const BIO_SELF_STATUS_ENABLE_ENV: &str = "APEIRETH_ENABLE_BIO_SELF_STATUS";

/// 生物机制子对象的**读数标注**（D32 建议②：键位属冻结合同 ⇒ 改在节旁显式标注；
/// 14 条机制里 5 条的 wire 名仍是体验名 ⇒ 用户看到本行即知那是**检测读数**）。
pub const BIO_MECHANISMS_READING_NOTE: &str =
    "以下为检测读数（分类/估计），不是内在体验；每格的接线状态见 wired/stage/reason";

/// 开关关时的整节原因（0 装诚实: 明说未启用，不冒充已采集）。
pub const BIO_SELF_STATUS_DISABLED_REASON: &str = "未启用: APEIRETH_ENABLE_BIO_SELF_STATUS != 1 \
     (默认关, fail-closed; 启用后才采集已接线机制的只读值)";

/// 机制未接线时的逐行原因（不编数值）。
pub const BIO_MECHANISM_NOT_WIRED_REASON: &str = "未接线: 本装配面未注册该机制的只读数据源 \
     (W3.2 只读聚合, 不编数值)";

/// 机制装配阶段: 未接线（本组装面无运行时数据源）。
pub const BIO_STAGE_NOT_WIRED: &str = "not_wired";

/// 机制装配阶段: 已接线（本组装面注册了只读探测口）。
pub const BIO_STAGE_WIRED: &str = "wired";

/// 解析 [`BIO_SELF_STATUS_ENABLE_ENV`] 的原始值: 只认 `"1"` (fail-closed)。
///
/// 纯函数（不读进程 env），与既有 `APEIRETH_ENABLE_*` 开关同一惯例:
/// 未设 / `"0"` / `"true"` / 垃圾值一律为关。
pub fn bio_self_status_enabled_from_env_value(raw: Option<&str>) -> bool {
    raw == Some("1")
}

/// 生物机制族 14 条机制的闭合词表（W3.2 装配面）。
///
/// 与台账 SG-13…SG-26 同名一一对应；枚举声明序 = 自述行序（聚合按此序逐格
/// 采集，不随注册表迭代顺序漂移）。**不是**新的顶层自述节。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BioMechanismKind {
    /// SG-13 神经调质慢变量。
    Neuromodulation,
    /// SG-14 异稳态 / 生命体征。
    Homeostasis,
    /// SG-23 节律 + 睡眠压力。
    Circadian,
    /// SG-15 免疫记忆 + 炎症收口。
    ImmuneMemory,
    /// SG-16 痛觉学习。
    Nociception,
    /// SG-17 再生四期。
    Recovery,
    /// SG-19 依恋 / 分离-重聚。
    Attachment,
    /// SG-20 社会认知 / 交互修复。
    SocialRepair,
    /// SG-24 群体感应升级门。
    Quorum,
    /// SG-18 凋亡 / 神经发生（资产生命周期）。
    Lifecycle,
    /// SG-21 游戏 / 幼态延续。
    Play,
    /// SG-22 仪式。
    Ritual,
    /// SG-25 共生菌群（工具生态健康度）。
    EcosystemHealth,
    /// SG-26 承载力 / 演替 / 关键种。
    Capacity,
}

impl BioMechanismKind {
    /// 14 条机制的规范顺序（自述行序 = 此序；闭合词表，不增不减不漂移）。
    pub const ALL: [Self; 14] = [
        Self::Neuromodulation,
        Self::Homeostasis,
        Self::Circadian,
        Self::ImmuneMemory,
        Self::Nociception,
        Self::Recovery,
        Self::Attachment,
        Self::SocialRepair,
        Self::Quorum,
        Self::Lifecycle,
        Self::Play,
        Self::Ritual,
        Self::EcosystemHealth,
        Self::Capacity,
    ];

    /// 稳定 snake_case 名（自述 JSON 键；与台账英文名同名）。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Neuromodulation => "neuromodulation",
            Self::Homeostasis => "homeostasis",
            Self::Circadian => "circadian",
            Self::ImmuneMemory => "immune_memory",
            Self::Nociception => "nociception",
            Self::Recovery => "recovery",
            Self::Attachment => "attachment",
            Self::SocialRepair => "social_repair",
            Self::Quorum => "quorum",
            Self::Lifecycle => "lifecycle",
            Self::Play => "play",
            Self::Ritual => "ritual",
            Self::EcosystemHealth => "ecosystem_health",
            Self::Capacity => "capacity",
        }
    }

    /// 由稳定名解析；词表外 = `None`（不隐式创建机制）。
    pub fn from_str_name(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == raw)
    }
}

/// 一格机制的只读读数（`capabilities.bio_mechanisms` 子对象的一行）。
///
/// 六字段冻结合同: `stage` / `value` / `threshold` / `last_action` / `wired` /
/// `reason`。未接线或采集失败的格一律显式 `null` + 原因，绝不编数值。
///
/// serde: 容器级 `#[serde(default)]` 让缺字段的旧/局部载荷按「未接线」缺省，
/// 不因新增字段读崩（向前兼容）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BioMechanismReading {
    /// 装配阶段标签（[`BIO_STAGE_NOT_WIRED`] / [`BIO_STAGE_WIRED`]）。
    pub stage: String,
    /// 当前读数（机制自定的只读 JSON 值）；取不到 = 显式 `null`。
    pub value: Option<Value>,
    /// 机制当前生效的判定阈值；无阈值 / 未接线 = 显式 `null`。
    pub threshold: Option<f64>,
    /// 最近动作的有界摘要；尚无动作 = 显式 `null`。
    pub last_action: Option<String>,
    /// 接线事实: 本组装面是否注册了该机制的只读探测口。
    ///
    /// 与采集成败分开: 注册但读失败 = `wired: true` + `value: null` + 原因；
    /// 未注册 = `wired: false` + `stage: "not_wired"` + 原因。
    pub wired: bool,
    /// 未接线 / 未启用 / 采集失败的原因；正常采集 = `None`。
    pub reason: Option<String>,
}

impl Default for BioMechanismReading {
    fn default() -> Self {
        Self::not_wired(BIO_MECHANISM_NOT_WIRED_REASON)
    }
}

impl BioMechanismReading {
    /// 未接线一格: `stage = not_wired`、`wired = false`、值/阈值/动作全
    /// `null` + 原因（0 装诚实，不编数值）。
    pub fn not_wired(reason: impl Into<String>) -> Self {
        Self {
            stage: BIO_STAGE_NOT_WIRED.to_string(),
            value: None,
            threshold: None,
            last_action: None,
            wired: false,
            reason: Some(reason.into()),
        }
    }

    /// 采集成功一格: `stage = wired`、`wired = true`；值/阈值/动作按真值
    /// （机制无阈值 / 尚无动作时可显式 `None`）。
    pub fn wired(
        value: Option<Value>,
        threshold: Option<f64>,
        last_action: Option<String>,
    ) -> Self {
        Self {
            stage: BIO_STAGE_WIRED.to_string(),
            value,
            threshold,
            last_action,
            wired: true,
            reason: None,
        }
    }

    /// 采集失败一格（探测口已注册但读失败）: 接线事实 `wired = true`、
    /// `stage = wired`、值/阈值/动作全 `null` + 真因（不整帧失败）。
    pub fn probe_failed(reason: impl Into<String>) -> Self {
        Self {
            stage: BIO_STAGE_WIRED.to_string(),
            value: None,
            threshold: None,
            last_action: None,
            wired: true,
            reason: Some(reason.into()),
        }
    }
}

/// `capabilities` 节内的生物机制只读聚合子对象（W3.2 装配面）。
///
/// **合同（兼容旧消费者）**:
/// - 观测面未挂载（[`SelfStatusSnapshot::bio_mechanisms`] = `None`）: 渲染面
///   **省略**该键，旧帧形状逐键不变；
/// - 观测面已挂载且开关关: `{"enabled": false, "mechanisms": null, "reason":
///   "<未启用>"}` —— 一格也不采集，`mechanisms` 显式 `null`（不是"采集到 0"）；
/// - 观测面已挂载且开关开: `mechanisms` = 14 条机制逐格读数（未注册的格
///   `wired: false` + 原因「未接线」；注册但读失败 = `null` + 真因；
///   正常 = 真值）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BioMechanismsStatus {
    /// 采集开关的生效值（`APEIRETH_ENABLE_BIO_SELF_STATUS=1` 才为 `true`）。
    pub enabled: bool,
    /// 逐机制读数（行序 = [`BioMechanismKind::ALL`]）；`None` = 开关关，
    /// 一格也不采集。
    pub mechanisms: Option<BTreeMap<BioMechanismKind, BioMechanismReading>>,
    /// 整节未启用 / 不可用的原因；正常采集 = `None`。
    pub reason: Option<String>,
    /// **读数标注**（D32 建议②）：属**类型的一部分** ⇒ serde 往返**逐字段稳定**。
    pub note: Option<String>,
}

impl Default for BioMechanismsStatus {
    fn default() -> Self {
        Self::disabled(BIO_SELF_STATUS_DISABLED_REASON)
    }
}

impl BioMechanismsStatus {
    /// 开关关（或观测面未启用）: `mechanisms` 显式 `null` + 原因
    /// （不冒充采集，也不把「没采」写成 0）。
    pub fn disabled(reason: impl Into<String>) -> Self {
        Self {
            enabled: false,
            mechanisms: None,
            reason: Some(reason.into()),
            note: Some(BIO_MECHANISMS_READING_NOTE.to_string()),
        }
    }

    /// 开关开且已采集: 14 格读数（未注册的格由调用方填
    /// [`BioMechanismReading::not_wired`]）。
    pub fn enabled_with(mechanisms: BTreeMap<BioMechanismKind, BioMechanismReading>) -> Self {
        Self {
            enabled: true,
            mechanisms: Some(mechanisms),
            reason: None,
            note: Some(BIO_MECHANISMS_READING_NOTE.to_string()),
        }
    }

    /// 一格读数（缺格 = `None`，不隐式造格）。
    pub fn reading(&self, kind: BioMechanismKind) -> Option<&BioMechanismReading> {
        self.mechanisms.as_ref().and_then(|rows| rows.get(&kind))
    }
}

/// 生物机制只读探测口（一格机制一次现读）。
///
/// `now_ms` = 采集时刻（epoch 毫秒，由装配面时钟注入，机制内核零系统时钟）；
/// 失败回 `Err(真因)` —— 自述格显式 `null` + 原因，**不**让整帧失败。
pub trait BioMechanismProbe: Send + Sync {
    /// 现读一格机制读数。
    fn snapshot(&self, now_ms: i64) -> Result<BioMechanismReading, String>;
}

/// 把一份自述投影成结构化 JSON (字段合同的唯一实现)。
///
/// 合同要点: **所有字段恒在场**; 来源缺失/失败的字段显式 `null` 并在同节
/// `reason` 给出原因 —— 不是缺字段, 不是整帧失败。
pub fn render_snapshot(snapshot: &SelfStatusSnapshot) -> Value {
    let mut capabilities: serde_json::Map<String, Value> = snapshot
        .capabilities
        .iter()
        .map(|entry| (entry.name.clone(), Value::Bool(entry.enabled)))
        .collect();
    // W3.2 生物机制族只读聚合: 只挂在既有 `capabilities` 节内 (非顶层节)。
    // 观测面未挂载 = 省略该键 (旧帧形状不变); 挂载后未启用 = 子对象显式
    // `mechanisms: null` + 「未启用」原因。
    if let Some(bio) = &snapshot.bio_mechanisms {
        capabilities.insert(
            CAPABILITIES_BIO_MECHANISMS.to_string(),
            render_bio_mechanisms(bio),
        );
    }
    // 问责计数（只读子对象；未挂载 = 省略该键，旧帧形状不变）。
    if let Some((stats, reason)) = crate::accountability_view::snapshot_accountability(snapshot) {
        capabilities.insert(
            CAPABILITIES_ACCOUNTABILITY.to_string(),
            crate::accountability_view::render_accountability(&stats, reason),
        );
    }

    let memory_ledger = crate::memory_ledger_view::render_memory_ledger(snapshot);

    let tuning = match &snapshot.tuning {
        Some(status) => json!({
            "values": {
                "memory_fade": status.memory_fade,
                "curiosity_strength": status.curiosity_strength,
                "tone_saturation": status.tone_saturation,
                "consolidation_cadence": status.consolidation_cadence,
            },
            "preset": status.preset,
            "self_learning": status.self_learning,
            // 性格节真值 (裁定 ④⑤): 锁定态 + 最近移动; 来源缺失 = 显式 null。
            "locked": status.locked,
            "recent_moves": status.recent_moves,
            "reason": snapshot.tuning_reason,
        }),
        None => json!({
            "values": {
                "memory_fade": null,
                "curiosity_strength": null,
                "tone_saturation": null,
                "consolidation_cadence": null,
            },
            "preset": null,
            "self_learning": null,
            "locked": null,
            "recent_moves": null,
            "reason": snapshot
                .tuning_reason
                .clone()
                .unwrap_or_else(|| DATA_PROBE_NOT_WIRED.to_string()),
        }),
    };

    let budget = match &snapshot.budget {
        Some(status) => json!({
            "max_rounds_per_turn": status.max_rounds_per_turn,
            "max_tool_calls_per_round": status.max_tool_calls_per_round,
            "quota": {
                "tokens": render_quota_dimension(&status.quota.tokens),
                "tool_steps": render_quota_dimension(&status.quota.tool_steps),
                "cost_micros": render_quota_dimension(&status.quota.cost_micros),
                "depth": render_quota_dimension(&status.quota.depth),
            },
            "source": status.source,
            "note": status.note,
            "reason": snapshot.budget_reason,
        }),
        None => json!({
            "max_rounds_per_turn": null,
            "max_tool_calls_per_round": null,
            "quota": null,
            "source": null,
            "note": null,
            "reason": snapshot
                .budget_reason
                .clone()
                .unwrap_or_else(|| DATA_PROBE_NOT_WIRED.to_string()),
        }),
    };

    let workspace = match &snapshot.workspace {
        Some(status) => json!({
            "root": status.root,
            "credentials_present": status.credentials_present,
            "reason": snapshot.workspace_reason,
        }),
        None => json!({
            "root": null,
            "credentials_present": null,
            "reason": snapshot
                .workspace_reason
                .clone()
                .unwrap_or_else(|| DATA_PROBE_NOT_WIRED.to_string()),
        }),
    };

    let model = match &snapshot.model {
        Some(profile) => json!({
            "model_id": profile.model_id,
            "context_window": profile.context_window,
            "max_output": profile.max_output,
            "supports_tools": profile.supports_tools,
            "supports_streaming": profile.supports_streaming,
            "capabilities_flags": profile.capabilities_flags,
            "input_price_micros": profile.input_price_micros,
            "output_price_micros": profile.output_price_micros,
            "cache_read_price_micros": profile.cache_read_price_micros,
            "prices_display": prices_display(profile),
            "reason": snapshot.model_reason.clone().or_else(|| {
                profile.is_unknown().then(|| MODEL_PROFILE_UNKNOWN.to_string())
            }),
        }),
        None => json!({
            "model_id": null,
            "context_window": null,
            "max_output": null,
            "supports_tools": null,
            "supports_streaming": null,
            "capabilities_flags": [],
            "input_price_micros": null,
            "output_price_micros": null,
            "cache_read_price_micros": null,
            "prices_display": PRICE_NOT_PROVIDED,
            "reason": snapshot
                .model_reason
                .clone()
                .unwrap_or_else(|| DATA_PROBE_NOT_WIRED.to_string()),
        }),
    };

    let cost_routing = match &snapshot.cost_routing {
        Some(status) => {
            let decision = status.decision.as_ref();
            json!({
                "enabled": status.enabled,
                "tier": decision.map(|decision| decision.tier.as_str()),
                "tier_source": decision.map(|decision| decision.tier_source.as_str()),
                "selected_model": decision.and_then(|decision| decision.selected_model.clone()),
                "price_display": decision
                    .and_then(|decision| decision.selected_profile.as_ref())
                    .map(prices_display)
                    .unwrap_or_else(|| PRICE_NOT_PROVIDED.to_string()),
                "used_fallback": decision.map(|decision| decision.used_fallback),
                "fallback_reason": decision.and_then(|decision| decision.fallback_reason.clone()),
                "selected_reason": decision.map(|decision| decision.selection_reason.clone()),
                "candidates": decision
                    .map(|decision| {
                        decision
                            .candidates
                            .iter()
                            .map(|profile| {
                                json!({
                                    "model_id": profile.model_id,
                                    "price_display": prices_display(profile),
                                })
                            })
                            .collect::<Vec<Value>>()
                    })
                    .unwrap_or_default(),
                "reason": status.reason.clone(),
            })
        }
        None => json!({
            "enabled": null,
            "tier": null,
            "tier_source": null,
            "selected_model": null,
            "price_display": PRICE_NOT_PROVIDED,
            "used_fallback": null,
            "fallback_reason": null,
            "selected_reason": null,
            "candidates": [],
            "reason": snapshot
                .cost_routing_reason
                .clone()
                .unwrap_or_else(|| DATA_PROBE_NOT_WIRED.to_string()),
        }),
    };

    let domain = match &snapshot.domain {
        Some(status) => json!({
            "user_id": status.user_id,
            "display_name": status.display_name,
            "role": status.role,
            "created_at": status.created_at,
            "counts": render_domain_counts(&status.counts),
        }),
        None => json!({
            "user_id": null,
            "display_name": null,
            "role": null,
            "created_at": null,
            "counts": render_domain_counts(&DomainCounts::unavailable(
                snapshot
                    .domain_reason
                    .clone()
                    .unwrap_or_else(|| DATA_PROBE_NOT_WIRED.to_string()),
            )),
        }),
    };

    let content_feed = match &snapshot.content_feed {
        Some(status) => json!({
            "feeds": status.feeds,
            "last_pull_at_ms": status.last_pull_at_ms,
            "last_pull_ok": status.last_pull_ok,
            "last_pull_ingested": status.last_pull_ingested,
            "last_pull_note": status.last_pull_note,
            "dedup_skipped": status.dedup_skipped,
            "ingest_queued": status.ingest_queued,
            "attention_truncated": status.attention_truncated,
            "degraded_feeds": status.degraded_feeds,
            "backing_off_feeds": status.backing_off_feeds,
            "reason": snapshot
                .content_feed_reason
                .clone()
                .or_else(|| status.reason.clone()),
        }),
        None => json!({
            "feeds": null,
            "last_pull_at_ms": null,
            "last_pull_ok": null,
            "last_pull_ingested": null,
            "last_pull_note": null,
            "dedup_skipped": null,
            "ingest_queued": null,
            "attention_truncated": null,
            "degraded_feeds": null,
            "backing_off_feeds": null,
            "reason": snapshot
                .content_feed_reason
                .clone()
                .unwrap_or_else(|| DATA_PROBE_NOT_WIRED.to_string()),
        }),
    };

    let care = match &snapshot.care {
        Some(status) => json!({
            "rules": status.rules,
            "next_trigger_at_ms": status.next_trigger_at_ms,
            "last_care_at_ms": status.last_care_at_ms,
            "last_care_note": status.last_care_note,
            // 心跳态 (V3-a 收尾一针): 下次 poll 时刻 / 最近心跳命中;
            // 未接线/未运行 = 显式 null (如实)。
            "heartbeat_next_poll_at_ms": status.heartbeat_next_poll_at_ms,
            "heartbeat_last_hit_at_ms": status.heartbeat_last_hit_at_ms,
            // 天气提醒计数 (K 轴 L2 提醒侧消费端): 信号账真值计数。
            "weather_reminders": status.weather_reminders,
            "reason": snapshot.care_reason.clone().or_else(|| status.reason.clone()),
        }),
        None => json!({
            "rules": null,
            "next_trigger_at_ms": null,
            "last_care_at_ms": null,
            "last_care_note": null,
            "heartbeat_next_poll_at_ms": null,
            "heartbeat_last_hit_at_ms": null,
            "weather_reminders": null,
            "reason": snapshot
                .care_reason
                .clone()
                .unwrap_or_else(|| DATA_PROBE_NOT_WIRED.to_string()),
        }),
    };

    let backup = match &snapshot.backup {
        Some(status) => json!({
            "last_backup_at_ms": status.last_backup_at_ms,
            "manifest_entries": status.manifest_entries,
            "reason": snapshot
                .backup_reason
                .clone()
                .or_else(|| status.reason.clone()),
        }),
        None => json!({
            "last_backup_at_ms": null,
            "manifest_entries": null,
            "reason": snapshot
                .backup_reason
                .clone()
                .unwrap_or_else(|| DATA_PROBE_NOT_WIRED.to_string()),
        }),
    };

    let voice = match &snapshot.voice {
        Some(status) => json!({
            "utterances": status.utterances,
            "last_spoken_at_ms": status.last_spoken_at_ms,
            "last_text_excerpt": status.last_text_excerpt,
            "voice": status.voice,
            "note": status.note,
            "reason": snapshot.voice_reason.clone().or_else(|| status.reason.clone()),
        }),
        None => json!({
            "utterances": null,
            "last_spoken_at_ms": null,
            "last_text_excerpt": null,
            "voice": null,
            "note": null,
            "reason": snapshot
                .voice_reason
                .clone()
                .unwrap_or_else(|| DATA_PROBE_NOT_WIRED.to_string()),
        }),
    };

    let task_board = match &snapshot.task_board {
        Some(status) => json!({
            "pending": status.pending,
            "assigned": status.assigned,
            "running": status.running,
            "done": status.done,
            "failed": status.failed,
            "blocked": status.blocked,
            "reason": snapshot.task_board_reason,
        }),
        None => json!({
            "pending": null,
            "assigned": null,
            "running": null,
            "done": null,
            "failed": null,
            "blocked": null,
            "reason": snapshot
                .task_board_reason
                .clone()
                .unwrap_or_else(|| DATA_PROBE_NOT_WIRED.to_string()),
        }),
    };

    let persona = match &snapshot.persona {
        Some(status) => json!({
            "core_beliefs": status.core_beliefs,
            "boundaries": status.boundaries,
            "preferences": status.preferences,
            "tensions": status.tensions,
            "last_consolidation_at": status.last_consolidation_at,
            "reason": snapshot
                .persona_reason
                .clone()
                .or_else(|| status.reason.clone()),
        }),
        None => json!({
            "core_beliefs": null,
            "boundaries": null,
            "preferences": null,
            "tensions": null,
            "last_consolidation_at": null,
            "reason": snapshot
                .persona_reason
                .clone()
                .unwrap_or_else(|| DATA_PROBE_NOT_WIRED.to_string()),
        }),
    };

    let proposal = match &snapshot.proposal {
        Some(status) => json!({
            "pending": status.pending,
            "accepted": status.accepted,
            "rejected": status.rejected,
            "total": status.total,
            "recent": status.recent,
            // 推送列 (自主度提案面接线): 已推送数 / 最近推送, 与推送账同源。
            "pushed": status.pushed,
            "recent_pushes": status.recent_pushes,
            "note": status.note,
            "reason": snapshot
                .proposal_reason
                .clone()
                .or_else(|| status.reason.clone()),
        }),
        None => json!({
            "pending": null,
            "accepted": null,
            "rejected": null,
            "total": null,
            "recent": null,
            "pushed": null,
            "recent_pushes": null,
            "note": null,
            "reason": snapshot
                .proposal_reason
                .clone()
                .unwrap_or_else(|| DATA_PROBE_NOT_WIRED.to_string()),
        }),
    };

    // 创作节 (创作工具面 W1): 最近操作计数, 只计数不回路径 (脱敏口径)。
    let image_ops = match &snapshot.image_ops {
        Some(status) => json!({
            "calls": status.calls,
            "ops_applied": status.ops_applied,
            "denied": status.denied,
            "by_operation": status.by_operation,
            "last_op_at_ms": status.last_op_at_ms,
            "last_call_ops": status.last_call_ops,
            "note": status.note,
            "reason": snapshot
                .image_ops_reason
                .clone()
                .or_else(|| status.reason.clone()),
        }),
        None => json!({
            "calls": null,
            "ops_applied": null,
            "denied": null,
            "by_operation": null,
            "last_op_at_ms": null,
            "last_call_ops": null,
            "note": null,
            "reason": snapshot
                .image_ops_reason
                .clone()
                .unwrap_or_else(|| DATA_PROBE_NOT_WIRED.to_string()),
        }),
    };

    // 创作节 (创作工具面 W2 · 文档操作): 进程内计数, 只计数不回路径 (脱敏口径)。
    let doc_ops = match &snapshot.doc_ops {
        Some(status) => json!({
            "calls": status.calls,
            "pages_touched": status.pages_touched,
            "denied": status.denied,
            "by_operation": status.by_operation,
            "last_op_at_ms": status.last_op_at_ms,
            "last_action": status.last_action,
            "note": status.note,
            "reason": snapshot
                .doc_ops_reason
                .clone()
                .or_else(|| status.reason.clone()),
        }),
        None => json!({
            "calls": null,
            "pages_touched": null,
            "denied": null,
            "by_operation": null,
            "last_op_at_ms": null,
            "last_action": null,
            "note": null,
            "reason": snapshot
                .doc_ops_reason
                .clone()
                .unwrap_or_else(|| DATA_PROBE_NOT_WIRED.to_string()),
        }),
    };

    // 在场节 (K 轴 L1 在场感知): 设备数/白名单数/最近感知, 真值如实 (白名单制
    // 默认零设备); 最近感知为进程内计数口径, 缺省表/尚无感知都不编造。
    let presence = match &snapshot.device_presence {
        Some(status) => json!({
            "devices": status.devices,
            "whitelisted": status.whitelisted,
            "last_sense_at_ms": status.last_sense_at_ms,
            "last_sense_note": status.last_sense_note,
            "note": status.note,
            "reason": snapshot
                .device_presence_reason
                .clone()
                .or_else(|| status.reason.clone()),
        }),
        None => json!({
            "devices": null,
            "whitelisted": null,
            "last_sense_at_ms": null,
            "last_sense_note": null,
            "note": null,
            "reason": snapshot
                .device_presence_reason
                .clone()
                .unwrap_or_else(|| DATA_PROBE_NOT_WIRED.to_string()),
        }),
    };

    // 创作节 (创作工具面 K3 · 表格操作): 进程内计数, 只计数不回路径 (脱敏口径)。
    let sheet_ops = match &snapshot.sheet_ops {
        Some(status) => json!({
            "calls": status.calls,
            "rows_touched": status.rows_touched,
            "denied": status.denied,
            "by_operation": status.by_operation,
            "last_op_at_ms": status.last_op_at_ms,
            "last_action": status.last_action,
            "note": status.note,
            "reason": snapshot
                .sheet_ops_reason
                .clone()
                .or_else(|| status.reason.clone()),
        }),
        None => json!({
            "calls": null,
            "rows_touched": null,
            "denied": null,
            "by_operation": null,
            "last_op_at_ms": null,
            "last_action": null,
            "note": null,
            "reason": snapshot
                .sheet_ops_reason
                .clone()
                .unwrap_or_else(|| DATA_PROBE_NOT_WIRED.to_string()),
        }),
    };

    // 设备受控执行节 (K 轴 L3/L4 受控执行): 控制计数/最近动作, 真值如实 (与
    // `tool.device_control` 工具面同一执行核心同一状态账); 只回计数与动作
    // 标签, 不回参数正文 (脱敏口径)。
    let device_control = match &snapshot.device_control {
        Some(status) => json!({
            "devices_total": status.devices_total,
            "devices_enabled": status.devices_enabled,
            "actions_executed": status.actions_executed,
            "executed_reversible": status.executed_reversible,
            "executed_irreversible": status.executed_irreversible,
            "executed_money": status.executed_money,
            "last_action": status.last_action,
            "note": status.note,
            "reason": snapshot
                .device_control_reason
                .clone()
                .or_else(|| status.reason.clone()),
        }),
        None => json!({
            "devices_total": null,
            "devices_enabled": null,
            "actions_executed": null,
            "executed_reversible": null,
            "executed_irreversible": null,
            "executed_money": null,
            "last_action": null,
            "note": null,
            "reason": snapshot
                .device_control_reason
                .clone()
                .unwrap_or_else(|| DATA_PROBE_NOT_WIRED.to_string()),
        }),
    };

    json!({
        "identity": {
            "product_name": snapshot.identity.product_name,
            "version": snapshot.identity.version,
            "runtime_role": snapshot.identity.runtime_role,
        },
        "capabilities": Value::Object(capabilities),
        "memory_ledger": memory_ledger,
        "tuning": tuning,
        "budget": budget,
        "workspace": workspace,
        "model": model,
        "cost_routing": cost_routing,
        "domain": domain,
        "content_feed": content_feed,
        "care": care,
        "backup": backup,
        "voice": voice,
        "task_board": task_board,
        "persona": persona,
        "proposal": proposal,
        "image_ops": image_ops,
        "doc_ops": doc_ops,
        "presence": presence,
        "sheet_ops": sheet_ops,
        "device_control": device_control,
    })
}

/// 单价显示串: 三项全缺 = 「未提供」; 有值则逐项标注 (缺的单项也标「未提供」)。
fn prices_display(profile: &ModelProfile) -> String {
    let item = |price: Option<u64>| match price {
        Some(value) => value.to_string(),
        None => PRICE_NOT_PROVIDED.to_string(),
    };
    if profile.input_price_micros.is_none()
        && profile.output_price_micros.is_none()
        && profile.cache_read_price_micros.is_none()
    {
        return PRICE_NOT_PROVIDED.to_string();
    }
    format!(
        "input={}, output={}, cache_read={}",
        item(profile.input_price_micros),
        item(profile.output_price_micros),
        item(profile.cache_read_price_micros)
    )
}

/// 把生物机制只读聚合投影成结构化 JSON (子对象合同的唯一实现)。
///
/// 行序 = [`BioMechanismKind::ALL`]（`BTreeMap` 键序即词表序）；开关关时
/// `mechanisms` 显式 `null`，一格也不采集。
fn render_bio_mechanisms(status: &BioMechanismsStatus) -> Value {
    let mechanisms = status
        .mechanisms
        .as_ref()
        .map(|rows| {
            Value::Object(
                rows.iter()
                    .map(|(kind, reading)| {
                        (
                            kind.as_str().to_string(),
                            json!({
                                "stage": reading.stage,
                                "value": reading.value,
                                "threshold": reading.threshold,
                                "last_action": reading.last_action,
                                "wired": reading.wired,
                                "reason": reading.reason,
                            }),
                        )
                    })
                    .collect(),
            )
        })
        .unwrap_or(Value::Null);
    json!({
        "enabled": status.enabled,
        "mechanisms": mechanisms,
        "reason": status.reason,
        // D32 建议②：键位保留（冻结合同），但**在节旁显式标注是读数**。
        // 优先用**类型里的** `note`（serde 往返稳定）；缺失时回落常量 ⇒ 标注永不失。
        // ⇒ **标注永不失**（且常量仍在渲染面 ⇒ 词表检查器守得住这条）。
        "note": status
            .note
            .clone()
            .unwrap_or_else(|| BIO_MECHANISMS_READING_NOTE.to_string()),
    })
}

/// 把一维配额行投影成结构化 JSON (字段合同的唯一实现)。
fn render_quota_dimension(row: &QuotaDimensionStatus) -> Value {
    json!({
        "limit": row.limit,
        "consumed": row.consumed,
        "remaining": row.remaining,
        "display": row.display,
        "reason": row.reason,
    })
}

/// 把域内计数投影成结构化 JSON (字段合同的唯一实现)。
fn render_domain_counts(counts: &DomainCounts) -> Value {
    json!({
        "sessions": counts.sessions,
        "memories": counts.memories,
        "protected": counts.protected,
        "reason": counts.reason,
    })
}

/// 空参数 (工具无参数; 未知字段拒绝)。
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct SelfStatusParams {}

/// 结构化自述工具 (`tool.self_status`, 声明名 `self_status`)。
pub struct SelfStatusTool {
    id: CapabilityId,
    source: Arc<dyn SelfStatusSource>,
}

impl SelfStatusTool {
    /// 绑定一个自述来源。
    pub fn new(source: Arc<dyn SelfStatusSource>) -> Self {
        Self {
            id: CapabilityId::new("tool.self_status").unwrap(),
            source,
        }
    }

    /// 输出归一合同: 结构化输出必须是对象且**每一节恒在场** (每节为对象;
    /// 节的全集与顺序 = [`SELF_STATUS_SECTIONS`] 冻结表, 随能力面叠加而增,
    /// 计数词免钉防漂移)。
    ///
    /// 具体字段允许显式 null (来源缺失), 但**节**不可缺席 —— 自述的形状冻结。
    /// 模型画像节随 H-P1 (模型画像表) 入合同: 路由/成本的依据数据面自述化;
    /// 成本分级路由节随 H-P2 (成本分级路由) 入合同: 当前档/选定模型/回退标注
    /// 真值自述化 (选型可解释, 无价模型价格照实「未提供」);
    /// 用户域节随 W3-① (多户数据隔离) 入合同: 当前域 id/角色/域内计数自述化;
    /// 内容流节随 J1 (内容流接入) 入合同: 订阅数/最近拉取自述化;
    /// 关怀节随 V3-a (双引擎主动关怀) 入合同: 规则数/下次触发/最近关怀自述化;
    /// 备份节随 G5 (备份恢复) 入合同: 最近备份时间/清单件数自述化;
    /// 任务簿节随 V6-a (任务编排持久队列) 入合同: 分态计数自述化;
    /// 人格节随 V4-a (人格记忆层) 入合同: 信念/边界/偏好/张力计数 + 最后整编
    /// 真值自述化 (只计数不回内容);
    /// 提案节随自主度第一小步 (边角料提案面) 入合同: 待审数/分态计数/最近提案
    /// 自述化 (accepted 只是账面标记, 不是执行);
    /// 创作节随创作工具面 W1 (图像操作) 入合同: 最近操作计数自述化 (只计数不回
    /// 路径, 进程内计数口径如实注明);
    /// 在场节随 K 轴 L1 (在场感知骨架) 入合同: 设备数/白名单数/最近感知自述化
    /// (白名单制默认零设备的实况可见, 最近感知为进程内计数口径);
    /// 设备受控执行节随 K 轴 L3/L4 (受控执行骨架) 入合同: 控制计数/最近动作
    /// 自述化 (执行了多少动作、最近动了什么/谁批的可见, 只回计数与动作标签)。
    ///
    /// **W3.2 生物机制族**: 不新增顶层节 —— 14 条机制的只读聚合挂在既有
    /// `capabilities` 节内的 [`CAPABILITIES_BIO_MECHANISMS`] 子对象上
    /// （未挂观测面时该键缺省；挂上后未启用 = `mechanisms: null` + 原因）。
    /// 本 schema 的 require 清单仍是 [`SELF_STATUS_SECTIONS`] 的 21 节。
    pub fn output_schema() -> OutputSchema {
        let mut schema = OutputSchema::new();
        for section in SELF_STATUS_SECTIONS {
            schema = schema.require(*section, SchemaKind::Object);
        }
        schema
    }

    /// 采集并投影当前自述 (结构化值)。
    pub fn collect(&self) -> Value {
        render_snapshot(&self.source.snapshot())
    }

    /// 采集并投影一份自述, 可带会话作用域 (配额计量列的真值来源)。
    async fn collect_scoped(&self, call: &ToolCall, session: Option<SessionId>) -> ToolResult {
        let arguments = match &call.arguments {
            Value::Null => json!({}),
            other => other.clone(),
        };
        if let Err(error) = serde_json::from_value::<SelfStatusParams>(arguments) {
            return ToolResult::permanent_error(
                &call.id,
                format!("invalid self_status parameters: {error}"),
            );
        }
        // 采集跑在阻塞池上: data 系探测 (库查询/文件读) 卡住时, 执行链的
        // 到期计时器仍能触发, 超时归既有 `timeout.*` code 族 (可重试帧),
        // 而不是把回合一起挂死。
        let source = Arc::clone(&self.source);
        match tokio::task::spawn_blocking(move || {
            render_snapshot(&source.snapshot_in_session(session))
        })
        .await
        {
            Ok(value) => ToolResult::ok(&call.id, value),
            Err(error) => ToolResult::permanent_error(
                &call.id,
                format!("self_status collection task failed: {error}"),
            ),
        }
    }
}

#[async_trait]
impl ToolCapability for SelfStatusTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let mut params = ToolParameters::new();
        params.extend(
            serde_json::json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        NormalizedTool::new("self_status")
            .with_description(
                "Read-only structured self-report: identity, capability switches in force, memory \
                 ledger counts (numbers only), tuning state, turn budgets, workspace facts, the \
                 active model profile (context window, tool support, unit prices — missing \
                 prices read as not provided, never invented), cost-tier routing state \
                 (current tier, selected model, honest fallback annotation), and physical \
                 device presence (device count, whitelist count, last sensing - whitelist \
                 with zero devices by default) plus controlled device execution (action \
                 counts per grade and the last action, from the append-only action ledger). \
                 Reports measured \
                 state only; credential values are never echoed.",
            )
            .with_parameters(params)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.collect_scoped(call, None).await
    }

    /// 会话作用域入口: 回合派发带会话 id, 配额计量列因此读到该会话账本的
    /// 真值 (见 [`SelfStatusSource::snapshot_in_session`])。
    async fn invoke_in_session(
        &self,
        call: &ToolCall,
        _frozen: Option<&apeireth_plugin::FrozenInvocation>,
        session: Option<SessionId>,
    ) -> ToolResult {
        self.collect_scoped(call, session).await
    }
}

/// 能力名册投影辅助: 把 (名, 值) 迭代器收成稳定排序的名册。
pub fn capability_roster<I, S>(entries: I) -> Vec<CapabilitySwitch>
where
    I: IntoIterator<Item = (S, bool)>,
    S: Into<String>,
{
    let mut map: BTreeMap<String, bool> = BTreeMap::new();
    for (name, enabled) in entries {
        map.insert(name.into(), enabled);
    }
    map.into_iter()
        .map(|(name, enabled)| CapabilitySwitch::new(name, enabled))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_snapshot() -> SelfStatusSnapshot {
        SelfStatusSnapshot {
            identity: SelfStatusIdentity::default(),
            capabilities: capability_roster([("shell", false), ("search", true)]),
            accountability: None,
            accountability_reason: None,
            memory_ledger: Some(MemoryLedgerStats {
                verified_lessons: Some(2),
                candidate_lessons: Some(1),
                sessions: Some(2),
                memories: Some(5),
                protected: Some(1),
                lessons: Some(0),
                pending_ingest: Some(3),
                last_fold_at_ms: Some(1_700_000_000_000),
                dedup_skipped: Some(4),
                retrieval_tuning_count: Some(2),
                last_retrieval_tuning: Some(RetrievalTuningMove {
                    target: "dimension:semantic".to_string(),
                    signal: "positive".to_string(),
                    previous: 0.25,
                    next: 0.26,
                    at_epoch_ms: 1_700_000_100_000,
                    evidence: "注入后命中回执: 正 1 / 负 0 (mem-a)".to_string(),
                }),
                reason: None,
            }),
            memory_ledger_reason: None,
            tuning: Some(TuningStatus {
                memory_fade: 1.5,
                curiosity_strength: 0.75,
                tone_saturation: 0.8,
                consolidation_cadence: 2.0,
                preset: derive_tuning_preset([1.5, 0.75, 0.8, 2.0]),
                self_learning: false,
                locked: Some(TraitLockStatus {
                    memory_fade: true,
                    curiosity_strength: false,
                    tone_saturation: false,
                    consolidation_cadence: false,
                }),
                recent_moves: Some(vec![TraitMoveStatus {
                    param: "memory_fade".to_string(),
                    label: "遗忘衰减强度".to_string(),
                    at_epoch_ms: 1_700_000_000_000,
                    delta: 0.25,
                    reason: "检索命中 4/未命中 0: 召回供给充裕, 遗忘衰减上调一档".to_string(),
                }]),
            }),
            tuning_reason: None,
            budget: Some(BudgetStatus {
                max_rounds_per_turn: 8,
                max_tool_calls_per_round: 16,
                quota: QuotaStatus {
                    tokens: QuotaDimensionStatus::metered(Some(5_000), 120),
                    tool_steps: QuotaDimensionStatus::metered(None, 3),
                    cost_micros: QuotaDimensionStatus::metered(Some(10_000), 2_000),
                    depth: QuotaDimensionStatus::not_wired(QUOTA_DEPTH_NOT_WIRED),
                },
                source: "constant".to_string(),
                note: BUDGET_SOURCE_CONSTANT_NOTE.to_string(),
            }),
            budget_reason: None,
            workspace: Some(WorkspaceStatus {
                root: Some("C:\\work".to_string()),
                credentials_present: Some(true),
            }),
            workspace_reason: None,
            model: Some(ModelProfile {
                model_id: "model-a".to_string(),
                context_window: Some(32_768),
                max_output: Some(8_192),
                supports_tools: Some(true),
                supports_streaming: Some(false),
                input_price_micros: None,
                output_price_micros: None,
                cache_read_price_micros: None,
                capabilities_flags: vec!["system_prompt".to_string()],
            }),
            model_reason: None,
            domain: Some(DomainStatus {
                user_id: "alice".to_string(),
                display_name: "Alice".to_string(),
                role: "owner".to_string(),
                created_at: 1_728_000_000_000,
                counts: DomainCounts {
                    sessions: Some(3),
                    memories: Some(12),
                    protected: Some(1),
                    reason: None,
                },
            }),
            domain_reason: None,
            content_feed: Some(ContentFeedStatus {
                feeds: Some(2),
                last_pull_at_ms: Some(1_728_000_500_000),
                last_pull_ok: Some(true),
                last_pull_ingested: Some(4),
                last_pull_note: Some("2 feeds, 4 items".to_string()),
                dedup_skipped: Some(1),
                ingest_queued: Some(4),
                attention_truncated: Some(0),
                degraded_feeds: Some(0),
                backing_off_feeds: Some(0),
                reason: None,
            }),
            content_feed_reason: None,
            care: Some(CareStatus {
                rules: Some(3),
                next_trigger_at_ms: Some(1_728_000_900_000),
                last_care_at_ms: Some(1_728_000_400_000),
                last_care_note: Some("morning: spoke (rule_fallback)".to_string()),
                // V3-a 收尾一针: 心跳态两列入 fixture (下次 poll 时刻 / 最近心跳命中)。
                heartbeat_next_poll_at_ms: Some(1_728_001_000_000),
                heartbeat_last_hit_at_ms: Some(1_728_000_400_000),
                // K 轴 L2 提醒侧消费端: 天气提醒计数入 fixture。
                weather_reminders: Some(4),
                reason: None,
            }),
            care_reason: None,
            backup: Some(BackupStatus {
                last_backup_at_ms: Some(1_728_000_600_000),
                manifest_entries: Some(7),
                reason: None,
            }),
            backup_reason: None,
            voice: Some(VoiceStatus {
                utterances: Some(5),
                last_spoken_at_ms: Some(1_728_000_700_000),
                last_text_excerpt: Some("最近说的一句".to_string()),
                voice: Some("system-default".to_string()),
                note: None,
                reason: None,
            }),
            voice_reason: None,
            task_board: Some(TaskBoardStatus {
                pending: 2,
                assigned: 1,
                running: 1,
                done: 4,
                failed: 1,
                blocked: 0,
            }),
            task_board_reason: None,
            persona: Some(PersonaStatusSection {
                core_beliefs: 2,
                boundaries: 1,
                preferences: 3,
                tensions: 1,
                last_consolidation_at: Some(1_728_000_800),
                reason: None,
            }),
            persona_reason: None,
            proposal: Some(ProposalStatus {
                pending: 1,
                accepted: 2,
                rejected: 1,
                total: 4,
                recent: vec![ProposalStatusEntry {
                    id: "prop-1728000900000-1".to_string(),
                    kind: "tidy".to_string(),
                    title: "边角料: 三处临时注释待清理".to_string(),
                    state: "pending".to_string(),
                }],
                pushed: 1,
                recent_pushes: vec![ProposalPushStatusEntry {
                    proposal_id: "prop-1728000900000-1".to_string(),
                    channel_id: "primary".to_string(),
                    at_ms: 1_728_000_950_000,
                    outcome: "pushed".to_string(),
                }],
                note: None,
                reason: None,
            }),
            proposal_reason: None,
            cost_routing: Some(CostRoutingStatus {
                enabled: true,
                decision: Some(apeireth_core::cost_router::RoutingDecision {
                    tier: apeireth_core::cost_router::CostTier::Standard,
                    tier_source: apeireth_core::cost_router::TierSource::Default,
                    selected_model: Some("model-a".to_string()),
                    selected_profile: Some(ModelProfile {
                        model_id: "model-a".to_string(),
                        context_window: Some(32_768),
                        max_output: Some(8_192),
                        supports_tools: Some(true),
                        supports_streaming: Some(false),
                        input_price_micros: None,
                        output_price_micros: None,
                        cache_read_price_micros: None,
                        capabilities_flags: vec!["system_prompt".to_string()],
                    }),
                    // 候选真值: 无价行按能力在场 (价格标注「未提供」), 缺一半
                    // 行逐项标注 (缺的不补, 绝不编价)。
                    candidates: vec![
                        ModelProfile {
                            model_id: "model-a".to_string(),
                            context_window: Some(32_768),
                            max_output: Some(8_192),
                            supports_tools: Some(true),
                            supports_streaming: Some(false),
                            input_price_micros: None,
                            output_price_micros: None,
                            cache_read_price_micros: None,
                            capabilities_flags: vec!["system_prompt".to_string()],
                        },
                        ModelProfile {
                            model_id: "model-b".to_string(),
                            context_window: Some(65_536),
                            max_output: Some(8_192),
                            supports_tools: Some(true),
                            supports_streaming: Some(true),
                            input_price_micros: Some(2_000),
                            output_price_micros: None,
                            cache_read_price_micros: None,
                            capabilities_flags: Vec::new(),
                        },
                    ],
                    used_fallback: false,
                    fallback_reason: None,
                    selection_reason:
                        "no comparable unit price among candidates (prices not provided); keeping \
                         the default model since it is a capability-qualified standard candidate"
                            .to_string(),
                }),
                reason: None,
            }),
            cost_routing_reason: None,
            image_ops: Some(ImageOpsStatus {
                calls: 3,
                ops_applied: 5,
                denied: 1,
                by_operation: vec![
                    ImageOpsOpCount {
                        operation: "resize".to_string(),
                        count: 2,
                    },
                    ImageOpsOpCount {
                        operation: "grayscale".to_string(),
                        count: 1,
                    },
                    ImageOpsOpCount {
                        operation: "watermark_text".to_string(),
                        count: 2,
                    },
                ],
                last_op_at_ms: Some(1_728_000_960_000),
                last_call_ops: Some("resize -> grayscale".to_string()),
                note: Some(
                    "进程内计数: 本进程生命周期内的最近操作计数 (重启清零), 不落账不造账"
                        .to_string(),
                ),
                reason: None,
            }),
            image_ops_reason: None,
            doc_ops: Some(DocOpsStatus {
                calls: 5,
                pages_touched: 9,
                denied: 1,
                by_operation: vec![
                    DocOpsOpCount {
                        operation: "pdf_info".to_string(),
                        count: 2,
                    },
                    DocOpsOpCount {
                        operation: "pdf_extract_text".to_string(),
                        count: 1,
                    },
                    DocOpsOpCount {
                        operation: "docx_extract_text".to_string(),
                        count: 1,
                    },
                    DocOpsOpCount {
                        operation: "docx_outline".to_string(),
                        count: 0,
                    },
                    DocOpsOpCount {
                        operation: "pdf_merge".to_string(),
                        count: 0,
                    },
                    DocOpsOpCount {
                        operation: "pdf_split".to_string(),
                        count: 0,
                    },
                    DocOpsOpCount {
                        operation: "pdf_extract_pages".to_string(),
                        count: 1,
                    },
                    DocOpsOpCount {
                        operation: "pdf_watermark_text".to_string(),
                        count: 0,
                    },
                ],
                last_op_at_ms: Some(1_728_000_970_000),
                last_action: Some("pdf_extract_pages".to_string()),
                note: Some(
                    "进程内计数: 本进程生命周期内的最近操作计数 (重启清零), 不落账不造账"
                        .to_string(),
                ),
                reason: None,
            }),
            doc_ops_reason: None,
            device_presence: Some(DevicePresenceStatus {
                devices: Some(3),
                whitelisted: Some(2),
                last_sense_at_ms: Some(1_728_000_980_000),
                last_sense_note: Some(
                    "2 台白名单设备已感知 (mock 档 = 固定 fixture 数据)".to_string(),
                ),
                note: Some(
                    "设备表已载入 (devices.json); 最近感知为进程内计数 (重启清零)".to_string(),
                ),
                reason: None,
            }),
            device_presence_reason: None,
            sheet_ops: Some(SheetOpsStatus {
                calls: 4,
                rows_touched: 12,
                denied: 1,
                by_operation: vec![
                    SheetOpsOpCount {
                        operation: "read".to_string(),
                        count: 1,
                    },
                    SheetOpsOpCount {
                        operation: "summary".to_string(),
                        count: 1,
                    },
                    SheetOpsOpCount {
                        operation: "filter".to_string(),
                        count: 1,
                    },
                    SheetOpsOpCount {
                        operation: "sort".to_string(),
                        count: 0,
                    },
                    SheetOpsOpCount {
                        operation: "aggregate".to_string(),
                        count: 1,
                    },
                    SheetOpsOpCount {
                        operation: "formula".to_string(),
                        count: 0,
                    },
                    SheetOpsOpCount {
                        operation: "export_csv".to_string(),
                        count: 0,
                    },
                ],
                last_op_at_ms: Some(1_728_000_990_000),
                last_action: Some("aggregate".to_string()),
                note: Some(
                    "进程内计数: 本进程生命周期内的最近操作计数 (重启清零), 不落账不造账"
                        .to_string(),
                ),
                reason: None,
            }),
            sheet_ops_reason: None,
            device_control: Some(DeviceControlStatus {
                devices_total: 3,
                devices_enabled: 2,
                actions_executed: 2,
                executed_reversible: 1,
                executed_irreversible: 1,
                executed_money: 0,
                last_action: Some(
                    "lamp / power_off @ 1728001000000 (谁批: im:u_demo@primary)".to_string(),
                ),
                note: None,
                reason: None,
            }),
            device_control_reason: None,
            bio_mechanisms: Some(fixture_bio_mechanisms()),
        }
    }

    /// 生物机制只读聚合的 fixture: 14 格全在场, 仅 `capacity` 一格有真值
    /// (其余 13 格如实「未接线」), 钉住逐格六字段合同。
    fn fixture_bio_mechanisms() -> BioMechanismsStatus {
        let mut rows = BTreeMap::new();
        for kind in BioMechanismKind::ALL {
            let reading = if kind == BioMechanismKind::Capacity {
                BioMechanismReading::wired(
                    Some(json!({
                        "active_tasks": 2,
                        "pending": 1,
                        "observed_at_ms": 1_728_001_000_000i64,
                    })),
                    Some(8.0),
                    Some("任务队列：2 个执行槽在跑".to_string()),
                )
            } else {
                BioMechanismReading::not_wired(BIO_MECHANISM_NOT_WIRED_REASON)
            };
            rows.insert(kind, reading);
        }
        BioMechanismsStatus::enabled_with(rows)
    }

    struct FixedSource {
        snapshot: SelfStatusSnapshot,
    }

    impl SelfStatusSource for FixedSource {
        fn snapshot(&self) -> SelfStatusSnapshot {
            self.snapshot.clone()
        }
    }

    async fn invoke_tool(snapshot: SelfStatusSnapshot) -> ToolResult {
        let tool = SelfStatusTool::new(Arc::new(FixedSource { snapshot }));
        let call = ToolCall {
            id: "call_1".into(),
            name: "self_status".into(),
            arguments: json!({}),
        };
        tool.invoke(&call).await
    }

    #[tokio::test]
    async fn every_section_arrives_with_real_values() {
        let result = invoke_tool(fixture_snapshot()).await;
        assert!(result.is_ok(), "{}", result.render());
        let value: Value = serde_json::from_str(&result.render()).expect("structured json");
        // 缺节即红: fixture 的渲染必须带出冻结节表的每一节 (节合同终核)。
        for section in SELF_STATUS_SECTIONS {
            assert!(value.get(section).is_some(), "missing section {section}");
        }
        assert_eq!(value["care"]["rules"], 3);
        assert_eq!(value["care"]["next_trigger_at_ms"], 1_728_000_900_000i64);
        assert_eq!(value["care"]["last_care_at_ms"], 1_728_000_400_000i64);
        assert_eq!(
            value["care"]["last_care_note"],
            "morning: spoke (rule_fallback)"
        );
        // V3-a 收尾一针: 心跳态两列 (下次 poll 时刻 / 最近心跳命中) 真值入帧。
        assert_eq!(
            value["care"]["heartbeat_next_poll_at_ms"],
            1_728_001_000_000i64
        );
        assert_eq!(
            value["care"]["heartbeat_last_hit_at_ms"],
            1_728_000_400_000i64
        );
        // K 轴 L2 提醒侧消费端: 天气提醒计数真值入帧。
        assert_eq!(value["care"]["weather_reminders"], 4);
        assert_eq!(value["care"]["reason"], Value::Null);
        assert_eq!(value["content_feed"]["feeds"], 2);
        assert_eq!(
            value["content_feed"]["last_pull_at_ms"],
            1_728_000_500_000i64
        );
        assert_eq!(value["content_feed"]["last_pull_ok"], true);
        assert_eq!(value["content_feed"]["last_pull_ingested"], 4);
        assert_eq!(value["identity"]["product_name"], PRODUCT_NAME);
        assert_eq!(value["identity"]["version"], RELEASE_VERSION);
        assert_eq!(
            value["identity"]["runtime_role"],
            RUNTIME_ROLE_GATEWAY_SIDECAR
        );
        assert_eq!(value["memory_ledger"]["sessions"], 2);
        assert_eq!(value["memory_ledger"]["memories"], 5);
        assert_eq!(value["memory_ledger"]["protected"], 1);
        assert_eq!(value["memory_ledger"]["lessons"], 0);
        // 教训对象（Lesson）与「反思沉淀计数」是两个东西，分列上报：
        // 已验证 = 真学到；候选 = 尚未验证（不注入上下文）。
        assert_eq!(value["memory_ledger"]["verified_lessons"], 2);
        assert_eq!(value["memory_ledger"]["candidate_lessons"], 1);
        // V4-b 保鲜回路三真值随记忆节如实上报 (待摄取数 / 最近折入 / 去重计数)。
        assert_eq!(value["memory_ledger"]["pending_ingest"], 3);
        assert_eq!(
            value["memory_ledger"]["last_fold_at_ms"],
            1_700_000_000_000i64
        );
        assert_eq!(value["memory_ledger"]["dedup_skipped"], 4);
        assert_eq!(value["tuning"]["preset"], PRESET_EFFORTLESS);
        assert_eq!(value["budget"]["max_rounds_per_turn"], 8);
        assert_eq!(value["budget"]["max_tool_calls_per_round"], 16);
        assert_eq!(value["workspace"]["root"], "C:\\work");
        assert_eq!(value["workspace"]["credentials_present"], true);
        assert_eq!(value["domain"]["user_id"], "alice");
        assert_eq!(value["domain"]["display_name"], "Alice");
        assert_eq!(value["domain"]["role"], "owner");
        assert_eq!(value["domain"]["created_at"], 1_728_000_000_000i64);
        assert_eq!(value["domain"]["counts"]["sessions"], 3);
        assert_eq!(value["domain"]["counts"]["memories"], 12);
        assert_eq!(value["domain"]["counts"]["protected"], 1);
        assert_eq!(value["persona"]["core_beliefs"], 2);
        assert_eq!(value["persona"]["boundaries"], 1);
        assert_eq!(value["persona"]["preferences"], 3);
        assert_eq!(value["persona"]["tensions"], 1);
        assert_eq!(value["persona"]["last_consolidation_at"], 1_728_000_800i64);
        assert_eq!(value["persona"]["reason"], Value::Null);
        // 提案节真值 (自主度第一小步): 待审数/分态计数/最近提案入帧。
        assert_eq!(value["proposal"]["pending"], 1);
        assert_eq!(value["proposal"]["accepted"], 2);
        assert_eq!(value["proposal"]["rejected"], 1);
        assert_eq!(value["proposal"]["total"], 4);
        let recent = value["proposal"]["recent"]
            .as_array()
            .expect("recent 提案在场");
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0]["id"], "prop-1728000900000-1");
        assert_eq!(recent[0]["kind"], "tidy");
        assert_eq!(recent[0]["state"], "pending");
        // 推送列 (自主度提案面接线): 已推送数 / 最近推送随自述上报。
        assert_eq!(value["proposal"]["pushed"], 1);
        let recent_pushes = value["proposal"]["recent_pushes"]
            .as_array()
            .expect("recent_pushes 在场");
        assert_eq!(recent_pushes.len(), 1);
        assert_eq!(recent_pushes[0]["proposal_id"], "prop-1728000900000-1");
        assert_eq!(recent_pushes[0]["channel_id"], "primary");
        assert_eq!(recent_pushes[0]["outcome"], "pushed");
        assert_eq!(recent_pushes[0]["at_ms"], 1_728_000_950_000i64);
        assert_eq!(value["proposal"]["reason"], Value::Null);
        // 在场节真值 (K 轴 L1): 设备数/白名单数/最近感知入帧。
        assert_eq!(value["presence"]["devices"], 3);
        assert_eq!(value["presence"]["whitelisted"], 2);
        assert_eq!(value["presence"]["last_sense_at_ms"], 1_728_000_980_000i64);
        assert!(value["presence"]["last_sense_note"]
            .as_str()
            .unwrap()
            .contains("2 台白名单设备已感知"));
        assert!(value["presence"]["note"]
            .as_str()
            .unwrap()
            .contains("进程内计数"));
        assert_eq!(value["presence"]["reason"], Value::Null);
        // 设备受控执行节真值 (K 轴 L3/L4): 控制计数/最近动作入帧。
        assert_eq!(value["device_control"]["devices_total"], 3);
        assert_eq!(value["device_control"]["devices_enabled"], 2);
        assert_eq!(value["device_control"]["actions_executed"], 2);
        assert_eq!(value["device_control"]["executed_reversible"], 1);
        assert_eq!(value["device_control"]["executed_irreversible"], 1);
        assert_eq!(value["device_control"]["executed_money"], 0);
        assert_eq!(
            value["device_control"]["last_action"],
            "lamp / power_off @ 1728001000000 (谁批: im:u_demo@primary)"
        );
        assert_eq!(value["device_control"]["reason"], Value::Null);
    }

    /// 设备受控执行节缺源 = 显式 null + 原因 (0 装诚实, 不编控制计数)。
    #[test]
    fn device_control_section_without_a_source_is_explicit_null_with_a_reason() {
        let mut snapshot = fixture_snapshot();
        snapshot.device_control = None;
        snapshot.device_control_reason =
            Some("device control probe failed: 状态账读取失败".to_string());
        let value = render_snapshot(&snapshot);
        assert_eq!(value["device_control"]["actions_executed"], Value::Null);
        assert_eq!(value["device_control"]["executed_money"], Value::Null);
        assert_eq!(value["device_control"]["last_action"], Value::Null);
        assert!(value["device_control"]["reason"]
            .as_str()
            .unwrap()
            .contains("device control probe failed"));
    }

    /// 在场节缺源 = 显式 null + 原因 (0 装诚实, 不编设备数)。
    #[test]
    fn presence_section_without_a_source_is_explicit_null_with_a_reason() {
        let mut snapshot = fixture_snapshot();
        snapshot.device_presence = None;
        snapshot.device_presence_reason =
            Some("device table refused to open: bad devices.json".to_string());
        let value = render_snapshot(&snapshot);
        assert_eq!(value["presence"]["devices"], Value::Null);
        assert_eq!(value["presence"]["whitelisted"], Value::Null);
        assert_eq!(value["presence"]["last_sense_at_ms"], Value::Null);
        assert!(value["presence"]["reason"]
            .as_str()
            .unwrap()
            .contains("device table refused to open"));
    }

    /// 提案节缺席 = 显式 null + 原因 (0 装诚实, 不编待审数)。
    #[tokio::test]
    async fn missing_proposal_section_is_explicit_null_with_a_reason() {
        let mut snapshot = fixture_snapshot();
        snapshot.proposal = None;
        snapshot.proposal_reason = Some("proposal ledger probe failed: bad json".to_string());
        let result = invoke_tool(snapshot).await;
        assert!(result.is_ok(), "a missing probe must not fail the frame");
        let value: Value = serde_json::from_str(&result.render()).expect("structured json");
        assert_eq!(value["proposal"]["pending"], Value::Null);
        assert_eq!(value["proposal"]["accepted"], Value::Null);
        assert_eq!(value["proposal"]["recent"], Value::Null);
        assert_eq!(value["proposal"]["pushed"], Value::Null);
        assert_eq!(value["proposal"]["recent_pushes"], Value::Null);
        assert!(
            value["proposal"]["reason"]
                .as_str()
                .unwrap()
                .contains("proposal ledger probe failed"),
            "{}",
            value["proposal"]["reason"]
        );
        // 其余节不受牵连。
        assert_eq!(value["persona"]["core_beliefs"], 2);
    }

    /// 性格节真值 (裁定 ④⑤): 四杆锁定态 + 最近移动随调参节如实上报。
    #[tokio::test]
    async fn tuning_section_reports_lock_state_and_recent_moves() {
        let result = invoke_tool(fixture_snapshot()).await;
        assert!(result.is_ok(), "{}", result.render());
        let value: Value = serde_json::from_str(&result.render()).expect("structured json");
        assert_eq!(value["tuning"]["locked"]["memory_fade"], true);
        assert_eq!(value["tuning"]["locked"]["curiosity_strength"], false);
        assert_eq!(value["tuning"]["locked"]["tone_saturation"], false);
        assert_eq!(value["tuning"]["locked"]["consolidation_cadence"], false);
        let moves = value["tuning"]["recent_moves"]
            .as_array()
            .expect("recent moves 在场");
        assert_eq!(moves.len(), 1);
        assert_eq!(moves[0]["param"], "memory_fade");
        assert_eq!(moves[0]["label"], "遗忘衰减强度");
        assert_eq!(moves[0]["delta"], 0.25);
        assert_eq!(moves[0]["at_epoch_ms"], 1_700_000_000_000i64);
        assert!(
            moves[0]["reason"].as_str().is_some_and(|r| !r.is_empty()),
            "最近移动必须带原因"
        );
    }

    /// 性格节来源缺席 = 显式 null + 原因 (0 装诚实, 不编锁定态/最近移动)。
    #[tokio::test]
    async fn missing_trait_status_is_explicit_null_with_a_reason() {
        let mut snapshot = fixture_snapshot();
        snapshot.tuning_reason = Some("trait growth probe failed: bad json".to_string());
        if let Some(tuning) = snapshot.tuning.as_mut() {
            tuning.locked = None;
            tuning.recent_moves = None;
        }
        let result = invoke_tool(snapshot).await;
        let value: Value = serde_json::from_str(&result.render()).expect("structured json");
        assert_eq!(value["tuning"]["locked"], Value::Null);
        assert_eq!(value["tuning"]["recent_moves"], Value::Null);
        assert_eq!(
            value["tuning"]["reason"],
            "trait growth probe failed: bad json"
        );
        // 生效值与锁定态不同源: 缺一不编另一, 值照实在场。
        assert_eq!(value["tuning"]["values"]["memory_fade"], 1.5);
    }

    #[tokio::test]
    async fn missing_data_sections_are_explicit_null_with_a_reason() {
        let mut snapshot = fixture_snapshot();
        snapshot.memory_ledger = None;
        snapshot.memory_ledger_reason = Some("memory store unavailable: disk error".to_string());
        snapshot.workspace = None;
        snapshot.workspace_reason = Some("workspace probe failed".to_string());
        snapshot.domain = None;
        snapshot.domain_reason = Some("domain registry unavailable: bad config".to_string());
        snapshot.content_feed = None;
        snapshot.content_feed_reason = Some("feeds.json refused to open: bad config".to_string());
        snapshot.care = None;
        snapshot.care_reason = Some("care_rules.json refused to open: bad config".to_string());

        let result = invoke_tool(snapshot).await;
        assert!(result.is_ok(), "a missing source must not fail the frame");
        let value: Value = serde_json::from_str(&result.render()).expect("structured json");

        assert_eq!(value["memory_ledger"]["sessions"], Value::Null);
        assert_eq!(value["memory_ledger"]["memories"], Value::Null);
        assert_eq!(value["memory_ledger"]["protected"], Value::Null);
        assert_eq!(value["memory_ledger"]["lessons"], Value::Null);
        assert!(
            value["memory_ledger"]["reason"]
                .as_str()
                .unwrap()
                .contains("memory store unavailable"),
            "{}",
            value["memory_ledger"]["reason"]
        );
        assert_eq!(value["workspace"]["root"], Value::Null);
        assert_eq!(value["workspace"]["credentials_present"], Value::Null);
        assert!(
            value["workspace"]["reason"]
                .as_str()
                .unwrap()
                .contains("workspace probe failed"),
            "{}",
            value["workspace"]["reason"]
        );
        // 域节缺席同样显式 null + 原因 (不丢节, 不编域)。
        assert_eq!(value["domain"]["user_id"], Value::Null);
        assert_eq!(value["domain"]["role"], Value::Null);
        assert_eq!(value["domain"]["counts"]["sessions"], Value::Null);
        assert!(
            value["domain"]["counts"]["reason"]
                .as_str()
                .unwrap()
                .contains("domain registry unavailable"),
            "{}",
            value["domain"]["counts"]["reason"]
        );
        // 其余节不受牵连: 不整帧失败。
        assert_eq!(value["identity"]["product_name"], PRODUCT_NAME);
        assert!(!value["memory_ledger"]["reason"].is_null());
        // 内容流节缺席同样显式 null + 原因 (不丢节, 不编订阅数)。
        assert_eq!(value["content_feed"]["feeds"], Value::Null);
        assert_eq!(value["content_feed"]["last_pull_at_ms"], Value::Null);
        assert!(
            value["content_feed"]["reason"]
                .as_str()
                .unwrap()
                .contains("feeds.json refused to open"),
            "{}",
            value["content_feed"]["reason"]
        );
        // 关怀节缺席同样显式 null + 原因 (不丢节, 不编规则数)。
        assert_eq!(value["care"]["rules"], Value::Null);
        assert_eq!(value["care"]["next_trigger_at_ms"], Value::Null);
        assert_eq!(value["care"]["last_care_at_ms"], Value::Null);
        assert_eq!(value["care"]["weather_reminders"], Value::Null);
        assert!(
            value["care"]["reason"]
                .as_str()
                .unwrap()
                .contains("care_rules.json refused to open"),
            "{}",
            value["care"]["reason"]
        );
    }

    /// V3-a 自述关怀行: 规则数/下次触发/最近关怀如实 —— 尚无关怀 = 显式 null +
    /// 如实注, 不编时间不编判定。V3-a 收尾一针: 心跳态两列 (下次 poll 时刻 /
    /// 最近心跳命中) 同口径 —— 未接线/未运行 = 显式 null, 不冒充在跑。
    #[test]
    fn care_section_reports_rule_count_next_trigger_and_last_care_honestly() {
        let mut snapshot = fixture_snapshot();
        snapshot.care = Some(CareStatus {
            rules: Some(1),
            next_trigger_at_ms: Some(1_728_100_000_000),
            last_care_at_ms: None,
            last_care_note: Some("尚无关怀记录".to_string()),
            heartbeat_next_poll_at_ms: None,
            heartbeat_last_hit_at_ms: None,
            weather_reminders: Some(2),
            reason: None,
        });
        let value = render_snapshot(&snapshot);
        assert_eq!(value["care"]["rules"], 1);
        assert_eq!(value["care"]["next_trigger_at_ms"], 1_728_100_000_000i64);
        assert_eq!(value["care"]["last_care_at_ms"], Value::Null);
        assert_eq!(value["care"]["last_care_note"], "尚无关怀记录");
        assert_eq!(value["care"]["heartbeat_next_poll_at_ms"], Value::Null);
        assert_eq!(value["care"]["heartbeat_last_hit_at_ms"], Value::Null);
        // K 轴 L2: 天气提醒计数照信号账真值报 (只交信号的提醒侧消费面同账)。
        assert_eq!(value["care"]["weather_reminders"], 2);
        assert_eq!(value["care"]["reason"], Value::Null);

        // 心跳态真值在场 = 照真值报 (下次 poll 时刻 / 最近心跳命中)。
        snapshot.care = Some(CareStatus {
            heartbeat_next_poll_at_ms: Some(1_728_100_300_000),
            heartbeat_last_hit_at_ms: Some(1_728_090_000_000),
            ..snapshot.care.clone().expect("care fixture")
        });
        let value = render_snapshot(&snapshot);
        assert_eq!(
            value["care"]["heartbeat_next_poll_at_ms"],
            1_728_100_300_000i64
        );
        assert_eq!(
            value["care"]["heartbeat_last_hit_at_ms"],
            1_728_090_000_000i64
        );

        // 探测口失败 = 全维显式 null + 原因 (0 装诚实)。
        let unavailable = CareStatus::unavailable("care_rules.json refused to open");
        assert_eq!(unavailable.rules, None);
        assert_eq!(unavailable.heartbeat_next_poll_at_ms, None);
        assert_eq!(unavailable.heartbeat_last_hit_at_ms, None);
        assert_eq!(
            unavailable.reason.as_deref(),
            Some("care_rules.json refused to open")
        );
    }

    /// V6-a 自述行: 任务簿分态计数真值 (pending/running/failed 计数不编造)。
    #[test]
    fn task_board_section_reports_state_counts_truthfully() {
        let mut snapshot = fixture_snapshot();
        snapshot.task_board = Some(TaskBoardStatus {
            pending: 3,
            assigned: 1,
            running: 2,
            done: 7,
            failed: 1,
            blocked: 0,
        });
        let value = render_snapshot(&snapshot);
        assert_eq!(value["task_board"]["pending"], 3);
        assert_eq!(value["task_board"]["assigned"], 1);
        assert_eq!(value["task_board"]["running"], 2);
        assert_eq!(value["task_board"]["done"], 7);
        assert_eq!(value["task_board"]["failed"], 1);
        assert_eq!(value["task_board"]["blocked"], 0);
        assert_eq!(value["task_board"]["reason"], Value::Null);

        // 探测口缺位 = 全维显式 null + 原因 (不编数)。
        snapshot.task_board = None;
        snapshot.task_board_reason = Some(DATA_PROBE_NOT_WIRED.to_string());
        let value = render_snapshot(&snapshot);
        assert_eq!(value["task_board"]["pending"], Value::Null);
        assert_eq!(value["task_board"]["failed"], Value::Null);
        assert_eq!(
            value["task_board"]["reason"].as_str(),
            Some(DATA_PROBE_NOT_WIRED)
        );
    }

    /// V4-a 自述人格节: 信念/边界/偏好/张力计数 + 最后整编真值如实 (只计数不
    /// 回内容; 尚未整编 = 显式 null, 不编时间)。
    #[test]
    fn persona_section_reports_counts_and_last_consolidation_truthfully() {
        let mut snapshot = fixture_snapshot();
        snapshot.persona = Some(PersonaStatusSection {
            core_beliefs: 4,
            boundaries: 2,
            preferences: 5,
            tensions: 1,
            last_consolidation_at: Some(1_728_200_000),
            reason: None,
        });
        let value = render_snapshot(&snapshot);
        assert_eq!(value["persona"]["core_beliefs"], 4);
        assert_eq!(value["persona"]["boundaries"], 2);
        assert_eq!(value["persona"]["preferences"], 5);
        assert_eq!(value["persona"]["tensions"], 1);
        assert_eq!(value["persona"]["last_consolidation_at"], 1_728_200_000i64);
        assert_eq!(value["persona"]["reason"], Value::Null);

        // 尚未整编 = 最后整编显式 null (不编时间), 计数照常如实。
        snapshot.persona = Some(PersonaStatusSection {
            core_beliefs: 0,
            boundaries: 0,
            preferences: 0,
            tensions: 0,
            last_consolidation_at: None,
            reason: None,
        });
        let value = render_snapshot(&snapshot);
        assert_eq!(value["persona"]["core_beliefs"], 0);
        assert_eq!(value["persona"]["last_consolidation_at"], Value::Null);
        assert_eq!(value["persona"]["reason"], Value::Null);
    }

    /// 人格节缺源 = 全维显式 null + 原因 (0 装诚实, 不编数)。
    #[test]
    fn persona_section_without_a_source_is_explicit_null_with_a_reason() {
        let mut snapshot = fixture_snapshot();
        snapshot.persona = None;
        snapshot.persona_reason = Some("persona store refused to open: bad config".to_string());
        let value = render_snapshot(&snapshot);
        assert_eq!(value["persona"]["core_beliefs"], Value::Null);
        assert_eq!(value["persona"]["boundaries"], Value::Null);
        assert_eq!(value["persona"]["preferences"], Value::Null);
        assert_eq!(value["persona"]["tensions"], Value::Null);
        assert_eq!(value["persona"]["last_consolidation_at"], Value::Null);
        assert!(
            value["persona"]["reason"]
                .as_str()
                .unwrap()
                .contains("persona store refused to open"),
            "{}",
            value["persona"]["reason"]
        );
    }

    /// 节合同终核: 节序显式、schema require 清单全、渲染缺一节即红 ——
    /// [`SELF_STATUS_SECTIONS`] 是 render / output_schema / fixture 三侧的
    /// 单一真源, 任一侧漂移都红。
    #[test]
    fn the_frozen_section_contract_is_the_single_source_for_render_and_schema() {
        // ① 节序显式: schema require 清单与冻结表逐项同名同序。
        let schema = SelfStatusTool::output_schema();
        let declared: Vec<&str> = schema
            .fields()
            .iter()
            .map(|field| field.name.as_str())
            .collect();
        assert_eq!(declared, SELF_STATUS_SECTIONS, "节序必须逐项显式一致");

        // ② 渲染面全集 = 冻结表 (不缺节, 不多节)。
        let value = render_snapshot(&fixture_snapshot());
        let rendered: std::collections::BTreeSet<&str> = value
            .as_object()
            .expect("object frame")
            .keys()
            .map(String::as_str)
            .collect();
        let frozen: std::collections::BTreeSet<&str> =
            SELF_STATUS_SECTIONS.iter().copied().collect();
        assert_eq!(rendered, frozen, "渲染面必须恰好是冻结节表的全集");
        assert!(SelfStatusTool::output_schema().validate(&value).is_ok());

        // ③ 渲染缺一节即红: 逐节拆除都要撞 output_contract.schema_mismatch。
        for section in SELF_STATUS_SECTIONS {
            let mut drifted = render_snapshot(&fixture_snapshot());
            drifted.as_object_mut().unwrap().remove(*section);
            let error = SelfStatusTool::output_schema()
                .validate(&drifted)
                .expect_err("missing section must be a contract violation");
            assert_eq!(
                error.code(),
                "output_contract.schema_mismatch",
                "缺节 {section} 必须红"
            );
        }
    }

    /// J1 自述行: 订阅数/最近拉取如实 —— 尚无拉取 = 显式 null + 如实注, 不编时间。
    #[test]
    fn content_feed_section_reports_subscription_count_and_last_pull_honestly() {
        let mut snapshot = fixture_snapshot();
        snapshot.content_feed = Some(ContentFeedStatus {
            feeds: Some(3),
            last_pull_at_ms: None,
            last_pull_ok: None,
            last_pull_ingested: None,
            last_pull_note: Some("尚无拉取记录".to_string()),
            reason: None,
            ..Default::default()
        });
        let value = render_snapshot(&snapshot);
        assert_eq!(value["content_feed"]["feeds"], 3);
        assert_eq!(value["content_feed"]["last_pull_at_ms"], Value::Null);
        assert_eq!(value["content_feed"]["last_pull_ok"], Value::Null);
        assert_eq!(value["content_feed"]["last_pull_ingested"], Value::Null);
        assert_eq!(value["content_feed"]["last_pull_note"], "尚无拉取记录");
        assert_eq!(value["content_feed"]["reason"], Value::Null);
    }

    #[test]
    fn missing_leaf_counts_stay_null_without_losing_the_section() {
        let mut snapshot = fixture_snapshot();
        snapshot.memory_ledger = Some(MemoryLedgerStats {
            verified_lessons: None,
            candidate_lessons: None,
            sessions: Some(1),
            memories: None,
            protected: None,
            lessons: None,
            pending_ingest: None,
            last_fold_at_ms: None,
            dedup_skipped: None,
            retrieval_tuning_count: None,
            last_retrieval_tuning: None,
            reason: Some("lesson store not wired".to_string()),
        });
        snapshot.memory_ledger_reason = None;
        let value = render_snapshot(&snapshot);
        assert_eq!(value["memory_ledger"]["sessions"], 1);
        assert_eq!(value["memory_ledger"]["memories"], Value::Null);
        assert_eq!(value["memory_ledger"]["lessons"], Value::Null);
        // V4-b 保鲜三真值缺源同样显式 null (不编数)。
        assert_eq!(value["memory_ledger"]["pending_ingest"], Value::Null);
        assert_eq!(value["memory_ledger"]["last_fold_at_ms"], Value::Null);
        assert_eq!(value["memory_ledger"]["dedup_skipped"], Value::Null);
        assert!(value["memory_ledger"]["reason"]
            .as_str()
            .unwrap()
            .contains("lesson store not wired"));
    }

    /// V4-c 记忆节扩: 进化次数 / 最近调权真值如实 (何时/因何证据/何维度/
    /// 前后权重); 尚未调权 = 显式 null 不编移动, 缺源 = 显式 null + 原因。
    #[test]
    fn memory_ledger_retrieval_tuning_fields_report_truth_or_explicit_null() {
        let mut snapshot = fixture_snapshot();
        let value = render_snapshot(&snapshot);
        assert_eq!(value["memory_ledger"]["retrieval_tuning_count"], 2);
        assert_eq!(
            value["memory_ledger"]["last_retrieval_tuning"]["target"],
            "dimension:semantic"
        );
        assert_eq!(
            value["memory_ledger"]["last_retrieval_tuning"]["signal"],
            "positive"
        );
        assert_eq!(
            value["memory_ledger"]["last_retrieval_tuning"]["previous"],
            0.25
        );
        assert_eq!(
            value["memory_ledger"]["last_retrieval_tuning"]["next"],
            0.26
        );
        assert_eq!(
            value["memory_ledger"]["last_retrieval_tuning"]["at_epoch_ms"],
            1_700_000_100_000i64
        );
        assert!(value["memory_ledger"]["last_retrieval_tuning"]["evidence"]
            .as_str()
            .unwrap()
            .contains("注入后命中回执"));
        assert!(SelfStatusTool::output_schema().validate(&value).is_ok());

        // 有账未调权: 计数 0 + 最近调权显式 null (不编移动)。
        if let Some(stats) = snapshot.memory_ledger.as_mut() {
            stats.retrieval_tuning_count = Some(0);
            stats.last_retrieval_tuning = None;
        }
        let zero = render_snapshot(&snapshot);
        assert_eq!(zero["memory_ledger"]["retrieval_tuning_count"], 0);
        assert_eq!(zero["memory_ledger"]["last_retrieval_tuning"], Value::Null);

        // 缺源: 全维显式 null + 原因 (0 装诚实, 不编数)。
        snapshot.memory_ledger = None;
        snapshot.memory_ledger_reason = Some("retrieval tuning ledger probe not wired".to_string());
        let missing = render_snapshot(&snapshot);
        assert_eq!(
            missing["memory_ledger"]["retrieval_tuning_count"],
            Value::Null
        );
        assert_eq!(
            missing["memory_ledger"]["last_retrieval_tuning"],
            Value::Null
        );
        assert!(missing["memory_ledger"]["reason"]
            .as_str()
            .unwrap()
            .contains("not wired"));
    }

    /// V4-b 记忆节扩: 待摄取数 / 最近折入 / 去重计数真值如实; 尚未折入 =
    /// 显式 null (不编时间)。
    #[test]
    fn memory_ledger_freshness_fields_report_truth_or_explicit_null() {
        let mut snapshot = fixture_snapshot();
        let value = render_snapshot(&snapshot);
        assert_eq!(value["memory_ledger"]["pending_ingest"], 3);
        assert_eq!(
            value["memory_ledger"]["last_fold_at_ms"],
            1_700_000_000_000i64
        );
        assert_eq!(value["memory_ledger"]["dedup_skipped"], 4);

        // 尚未折入: 最近折入显式 null (不编时间), 其余真值照常。
        if let Some(stats) = snapshot.memory_ledger.as_mut() {
            stats.pending_ingest = Some(0);
            stats.last_fold_at_ms = None;
            stats.dedup_skipped = Some(0);
        }
        let unfilled = render_snapshot(&snapshot);
        assert_eq!(unfilled["memory_ledger"]["pending_ingest"], 0);
        assert_eq!(unfilled["memory_ledger"]["last_fold_at_ms"], Value::Null);
        assert_eq!(unfilled["memory_ledger"]["dedup_skipped"], 0);
        assert!(SelfStatusTool::output_schema().validate(&unfilled).is_ok());
    }

    #[test]
    fn a_partial_domain_counts_probe_stays_honest_per_dimension() {
        // 域计数缺维逐维显式 null + 原因, 域节不丢、域身份仍如实。
        let mut snapshot = fixture_snapshot();
        let domain = snapshot.domain.as_mut().expect("fixture carries a domain");
        domain.counts = DomainCounts {
            sessions: Some(4),
            memories: None,
            protected: None,
            reason: Some("memory ledger probe not wired".to_string()),
        };
        let value = render_snapshot(&snapshot);
        assert_eq!(value["domain"]["user_id"], "alice");
        assert_eq!(value["domain"]["counts"]["sessions"], 4);
        assert_eq!(value["domain"]["counts"]["memories"], Value::Null);
        assert_eq!(value["domain"]["counts"]["protected"], Value::Null);
        assert!(value["domain"]["counts"]["reason"]
            .as_str()
            .unwrap()
            .contains("memory ledger probe not wired"));
        assert!(SelfStatusTool::output_schema().validate(&value).is_ok());
    }

    #[test]
    fn output_schema_accepts_the_rendered_shape() {
        let value = render_snapshot(&fixture_snapshot());
        assert!(SelfStatusTool::output_schema().validate(&value).is_ok());
    }

    #[test]
    fn output_schema_rejects_a_drifted_shape() {
        let mut value = render_snapshot(&fixture_snapshot());
        value.as_object_mut().unwrap().remove("memory_ledger");
        let error = SelfStatusTool::output_schema()
            .validate(&value)
            .expect_err("missing section must be a contract violation");
        assert_eq!(error.code(), "output_contract.schema_mismatch");
    }

    #[test]
    fn capability_roster_is_sorted_and_lossless() {
        let roster = capability_roster([("shell", true), ("fetch", false), ("search", true)]);
        let names: Vec<&str> = roster.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(names, ["fetch", "search", "shell"]);
        assert!(!roster[0].enabled);
        assert!(roster[1].enabled);
    }

    #[test]
    fn preset_derivation_matches_the_named_value_sets() {
        assert_eq!(
            derive_tuning_preset([1.5, 0.75, 0.8, 2.0]),
            PRESET_EFFORTLESS
        );
        assert_eq!(derive_tuning_preset([1.0, 1.0, 1.0, 1.0]), PRESET_BALANCED);
        assert_eq!(
            derive_tuning_preset([0.5, 1.5, 1.2, 1.0]),
            PRESET_DEEP_MEMORY
        );
        assert_eq!(derive_tuning_preset([2.0, 2.0, 2.0, 3.0]), PRESET_CUSTOM);
    }

    #[tokio::test]
    async fn unknown_arguments_are_refused() {
        let tool = SelfStatusTool::new(Arc::new(FixedSource {
            snapshot: fixture_snapshot(),
        }));
        let call = ToolCall {
            id: "call_1".into(),
            name: "self_status".into(),
            arguments: json!({ "surprise": 1 }),
        };
        let result = tool.invoke(&call).await;
        assert!(!result.is_ok());
        assert!(
            result.render().contains("invalid self_status parameters"),
            "{}",
            result.render()
        );
    }

    #[tokio::test]
    async fn pipeline_freezes_the_frame_into_the_output_contract() {
        use crate::exec_pipeline::{ExecutionStage, ToolExecutionPipeline};

        let tool: Arc<dyn ToolCapability> = Arc::new(SelfStatusTool::new(Arc::new(FixedSource {
            snapshot: fixture_snapshot(),
        })));
        let pipeline =
            ToolExecutionPipeline::new().with_output_schema(SelfStatusTool::output_schema());
        let call = ToolCall {
            id: "call_1".into(),
            name: "self_status".into(),
            arguments: json!({}),
        };
        let executed = pipeline.run(tool.as_ref(), &call).await;
        assert!(executed.failure.is_none(), "{:?}", executed.failure);
        assert!(executed.outcome.ok);
        let structured = executed.outcome.structured().expect("structured sidecar");
        // 缺节即红: 冻结节表的每一节都必须穿过执行管道到场。
        for section in SELF_STATUS_SECTIONS {
            assert!(
                structured.get(section).is_some(),
                "missing section {section}"
            );
        }
        assert!(executed
            .record
            .stages()
            .contains(&ExecutionStage::Normalize));
    }

    #[tokio::test]
    async fn a_hanging_collection_closes_as_a_timeout_frame() {
        use crate::exec_pipeline::{AroundPolicy, PipelineFailure, ToolExecutionPipeline};

        struct HangingSource;

        impl SelfStatusSource for HangingSource {
            fn snapshot(&self) -> SelfStatusSnapshot {
                std::thread::sleep(std::time::Duration::from_millis(600));
                fixture_snapshot()
            }
        }

        let tool: Arc<dyn ToolCapability> = Arc::new(SelfStatusTool::new(Arc::new(HangingSource)));
        let pipeline =
            ToolExecutionPipeline::new().with_around(AroundPolicy::new().with_timeout(50, 120_000));
        let call = ToolCall {
            id: "call_1".into(),
            name: "self_status".into(),
            arguments: json!({}),
        };
        let executed = pipeline.run(tool.as_ref(), &call).await;
        assert!(
            matches!(executed.failure, Some(PipelineFailure::Timeout { .. })),
            "{:?}",
            executed.failure
        );
        let rendered = executed.result.render();
        assert!(rendered.contains("timeout."), "{rendered}");
        assert!(
            rendered.contains("timeout.deadline_expired"),
            "超时必须归 timeout.* code 族: {rendered}"
        );
    }

    #[test]
    fn quota_rows_report_true_limits_consumed_and_remaining_with_dash_for_unlimited() {
        let value = render_snapshot(&fixture_snapshot());
        let tokens = &value["budget"]["quota"]["tokens"];
        assert_eq!(tokens["limit"], 5_000);
        assert_eq!(tokens["consumed"], 120);
        assert_eq!(tokens["remaining"], 4_880);
        assert_eq!(tokens["display"], "120/5000");
        assert!(tokens["reason"].is_null());

        // 不设限显 "—": 上限/余量为显式 null, 显示串的上限侧是 "—"。
        let steps = &value["budget"]["quota"]["tool_steps"];
        assert_eq!(steps["limit"], Value::Null);
        assert_eq!(steps["consumed"], 3);
        assert_eq!(steps["remaining"], Value::Null);
        assert_eq!(steps["display"], "3/—");
        assert!(steps["reason"].is_null());
    }

    #[test]
    fn the_depth_dimension_is_honestly_marked_not_wired() {
        let value = render_snapshot(&fixture_snapshot());
        let row = &value["budget"]["quota"]["depth"];
        assert_eq!(row["display"], "—", "深度维暂无接口整行显 —");
        assert_eq!(row["limit"], Value::Null);
        assert_eq!(row["consumed"], Value::Null);
        assert_eq!(row["remaining"], Value::Null);
        assert!(
            row["reason"].as_str().unwrap().contains("not wired"),
            "深度维必须如实标注暂无接口: {row}"
        );
    }

    #[test]
    fn the_cost_dimension_reports_true_values_and_marks_partial_metering() {
        // 演进自「花费维暂无接口整行 —」钉死 (演进理由: A-P2 计量点装表接线,
        // 花费维转真值 —— 上限/已耗/余量; 含无价轮时如实标"部分计量")。
        let mut snapshot = fixture_snapshot();
        snapshot.budget.as_mut().expect("budget").quota.cost_micros =
            QuotaDimensionStatus::without_reading(Some(10_000), DATA_PROBE_NOT_WIRED)
                .with_cost_reading(2_000, 2);
        let value = render_snapshot(&snapshot);
        let row = &value["budget"]["quota"]["cost_micros"];
        assert_eq!(row["limit"], 10_000);
        assert_eq!(row["consumed"], 2_000);
        assert_eq!(row["remaining"], 8_000);
        assert_eq!(row["display"], "2000/10000");
        assert_eq!(
            row["reason"],
            quota_cost_partial_note(2),
            "含无价轮 = 部分计量标注, 缺口照实说"
        );
    }

    #[test]
    fn a_fully_priced_cost_total_carries_no_partial_note() {
        let mut snapshot = fixture_snapshot();
        snapshot.budget.as_mut().expect("budget").quota.cost_micros =
            QuotaDimensionStatus::without_reading(None, DATA_PROBE_NOT_WIRED)
                .with_cost_reading(2_000, 0);
        let value = render_snapshot(&snapshot);
        let row = &value["budget"]["quota"]["cost_micros"];
        assert_eq!(row["consumed"], 2_000);
        assert_eq!(row["limit"], Value::Null, "不设限显 —");
        assert_eq!(row["display"], "2000/—");
        assert!(row["reason"].is_null(), "全量口径无标注: {row}");
    }

    #[test]
    fn the_partial_note_copy_is_frozen_and_names_the_gap() {
        // 用户可见文案是合同 (演进须注明理由): 标注必须说清"部分计量"口径、
        // 无价轮真实计数、以及绝不编价的立场。
        assert_eq!(
            quota_cost_partial_note(0),
            "部分计量: 0 个计量轮无单价未计入 (绝不编价)"
        );
        assert_eq!(
            quota_cost_partial_note(3),
            "部分计量: 3 个计量轮无单价未计入 (绝不编价)"
        );
    }

    #[test]
    fn quota_readings_without_a_session_scope_are_explicit_null_with_a_reason() {
        let row = QuotaDimensionStatus::without_reading(Some(10), QUOTA_METER_SCOPE_REQUIRED);
        assert_eq!(row.limit, Some(10));
        assert_eq!(row.consumed, None);
        assert_eq!(row.remaining, None);
        assert_eq!(row.display, "—/10");
        assert_eq!(row.reason.as_deref(), Some(QUOTA_METER_SCOPE_REQUIRED));

        // 换上真值后数字恢复, 上限不变。
        let row = row.with_reading(4);
        assert_eq!(row.consumed, Some(4));
        assert_eq!(row.remaining, Some(6));
        assert_eq!(row.display, "4/10");
        assert!(row.reason.is_none());
    }

    #[test]
    fn the_session_scoped_snapshot_entry_defaults_to_the_scope_free_snapshot() {
        struct BareSource;
        impl SelfStatusSource for BareSource {
            fn snapshot(&self) -> SelfStatusSnapshot {
                fixture_snapshot()
            }
        }

        let source = BareSource;
        let scoped = source.snapshot_in_session(Some(SessionId::new()));
        assert_eq!(scoped, source.snapshot(), "默认实现不改变读数");
    }

    #[tokio::test]
    async fn the_pipelined_entry_forwards_the_session_scope_to_the_source() {
        use crate::exec_pipeline::{PipelinedCapability, ToolExecutionPipeline};

        /// 记住绑定会话并只对它给出真值的来源 (钉住作用域透传链)。
        struct ScopedSource {
            bound: SessionId,
            reading: u64,
        }

        impl SelfStatusSource for ScopedSource {
            fn snapshot(&self) -> SelfStatusSnapshot {
                fixture_snapshot()
            }

            fn snapshot_in_session(&self, session: Option<SessionId>) -> SelfStatusSnapshot {
                let mut snapshot = fixture_snapshot();
                if session == Some(self.bound) {
                    if let Some(budget) = snapshot.budget.as_mut() {
                        budget.quota.tokens =
                            QuotaDimensionStatus::metered(Some(100), self.reading);
                    }
                }
                snapshot
            }
        }

        let bound = SessionId::new();
        let inner: Arc<dyn ToolCapability> =
            Arc::new(SelfStatusTool::new(Arc::new(ScopedSource {
                bound,
                reading: 7,
            })));
        let tool = PipelinedCapability::new(inner, Arc::new(ToolExecutionPipeline::new()));
        let call = ToolCall {
            id: "call_1".into(),
            name: "self_status".into(),
            arguments: json!({}),
        };

        // 会话作用域穿过 `invoke_in_session` → 管道 → 来源 (真值列)。
        let scoped = tool.invoke_in_session(&call, None, Some(bound)).await;
        let value: Value = serde_json::from_str(&scoped.render()).expect("structured json");
        assert_eq!(
            value["budget"]["quota"]["tokens"]["consumed"], 7,
            "会话作用域必须透传到来源: {value}"
        );

        // 无作用域走基线快照 (fixture 的 120), 不是绑定会话的读数。
        let unscoped = tool.invoke(&call).await;
        let value: Value = serde_json::from_str(&unscoped.render()).expect("structured json");
        assert_eq!(value["budget"]["quota"]["tokens"]["consumed"], 120);
    }

    #[test]
    fn the_model_section_projects_the_profile_with_honest_missing_prices() {
        let value = render_snapshot(&fixture_snapshot());
        let model = &value["model"];
        assert_eq!(model["model_id"], "model-a");
        assert_eq!(model["context_window"], 32_768);
        assert_eq!(model["max_output"], 8_192);
        assert_eq!(model["supports_tools"], true);
        assert_eq!(model["supports_streaming"], false);
        assert_eq!(model["capabilities_flags"], json!(["system_prompt"]));
        // 单价缺省 = 未提供: 数字侧显式 null, 显示侧明说「未提供」。
        assert_eq!(model["input_price_micros"], Value::Null);
        assert_eq!(model["output_price_micros"], Value::Null);
        assert_eq!(model["cache_read_price_micros"], Value::Null);
        assert_eq!(model["prices_display"], PRICE_NOT_PROVIDED);
        assert!(model["reason"].is_null());
    }

    #[test]
    fn provided_prices_render_each_item_and_missing_items_stay_not_provided() {
        let mut snapshot = fixture_snapshot();
        let profile = snapshot.model.as_mut().expect("model profile");
        profile.input_price_micros = Some(2_000);
        profile.cache_read_price_micros = Some(500);
        let value = render_snapshot(&snapshot);
        let model = &value["model"];
        assert_eq!(model["input_price_micros"], 2_000);
        assert_eq!(model["output_price_micros"], Value::Null);
        assert_eq!(
            model["prices_display"],
            format!("input=2000, output={PRICE_NOT_PROVIDED}, cache_read=500")
        );
    }

    #[test]
    fn the_model_section_without_a_source_is_explicit_null_with_a_reason() {
        let mut snapshot = fixture_snapshot();
        snapshot.model = None;
        snapshot.model_reason = Some("model profile source not wired".to_string());
        let value = render_snapshot(&snapshot);
        let model = &value["model"];
        assert_eq!(model["model_id"], Value::Null);
        assert_eq!(model["context_window"], Value::Null);
        assert_eq!(model["supports_tools"], Value::Null);
        assert_eq!(model["prices_display"], PRICE_NOT_PROVIDED);
        assert!(
            model["reason"]
                .as_str()
                .unwrap()
                .contains("model profile source not wired"),
            "{}",
            model["reason"]
        );
    }

    #[test]
    fn an_unknown_model_reports_explicit_unknown_instead_of_invented_facts() {
        let mut snapshot = fixture_snapshot();
        snapshot.model = Some(ModelProfile::unknown("mystery-model"));
        snapshot.model_reason = None;
        let value = render_snapshot(&snapshot);
        let model = &value["model"];
        assert_eq!(model["model_id"], "mystery-model", "id 如实");
        assert_eq!(model["context_window"], Value::Null, "不编窗口");
        assert_eq!(model["supports_tools"], Value::Null, "未知不等于不支持");
        assert_eq!(model["prices_display"], PRICE_NOT_PROVIDED, "绝不编价");
        assert!(
            model["reason"]
                .as_str()
                .unwrap()
                .contains("explicitly unknown"),
            "{}",
            model["reason"]
        );
    }

    #[test]
    fn the_model_section_carries_no_credential_surface() {
        // 画像面无密钥: 自述帧字段全表里没有任何凭据面字段 (既有
        // `credentials_present` 是存在性布尔, 不是凭据本体)。
        fn keys(value: &Value, out: &mut Vec<String>) {
            match value {
                Value::Object(map) => {
                    for (key, child) in map {
                        out.push(key.to_ascii_lowercase());
                        keys(child, out);
                    }
                }
                Value::Array(items) => {
                    for item in items {
                        keys(item, out);
                    }
                }
                _ => {}
            }
        }
        const FORBIDDEN: &[&str] = &[
            "api_key",
            "apikey",
            "key",
            "token",
            "auth",
            "bearer",
            "secret",
            "password",
            "passwd",
            "credential",
            "credentials",
            "signature",
        ];
        let value = render_snapshot(&fixture_snapshot());
        let mut all_keys = Vec::new();
        keys(&value, &mut all_keys);
        for name in &all_keys {
            assert!(
                !FORBIDDEN.contains(&name.as_str()),
                "自述必须无密钥面字段: {name}"
            );
        }
    }

    #[test]
    fn the_cost_routing_section_projects_tier_selected_and_fallback_truth() {
        // 自述路由节 (H-P2): 当前档 / 选定模型 / 回退标注, 全部真值投影。
        let value = render_snapshot(&fixture_snapshot());
        let routing = &value["cost_routing"];
        assert_eq!(routing["enabled"], true, "开关真值");
        assert_eq!(routing["tier"], "standard", "当前档真值");
        assert_eq!(routing["tier_source"], "default", "档位来源可解释");
        assert_eq!(routing["selected_model"], "model-a", "选定模型真值");
        assert_eq!(routing["used_fallback"], false, "回退标注真值");
        assert_eq!(routing["price_display"], PRICE_NOT_PROVIDED, "绝不编价");
        assert!(routing["reason"].is_null());
        // 候选真值: 无价行价格标注「未提供」, 缺一半行逐项标注 (缺的不补)。
        let candidates = routing["candidates"].as_array().expect("candidate list");
        assert_eq!(candidates[0]["model_id"], "model-a");
        assert_eq!(candidates[0]["price_display"], PRICE_NOT_PROVIDED);
        assert_eq!(
            candidates[1]["price_display"],
            format!("input=2000, output={PRICE_NOT_PROVIDED}, cache_read={PRICE_NOT_PROVIDED}"),
            "缺一半如实逐项标注, 不编另一半"
        );

        // 回退决策: 回退标注 true + 如实原因进帧 (不粉饰)。
        let mut snapshot = fixture_snapshot();
        snapshot.cost_routing = Some(CostRoutingStatus {
            enabled: true,
            decision: Some(apeireth_core::cost_router::RoutingDecision {
                tier: apeireth_core::cost_router::CostTier::Capable,
                tier_source: apeireth_core::cost_router::TierSource::Explicit,
                selected_model: Some("default-model".to_string()),
                selected_profile: Some(ModelProfile::unknown("default-model")),
                candidates: Vec::new(),
                used_fallback: true,
                fallback_reason: Some("no model row passes the capable filter".to_string()),
                selection_reason: "fallback: default model kept as the explicit fallback"
                    .to_string(),
            }),
            reason: None,
        });
        let value = render_snapshot(&snapshot);
        let routing = &value["cost_routing"];
        assert_eq!(routing["tier"], "capable");
        assert_eq!(routing["tier_source"], "explicit");
        assert_eq!(routing["used_fallback"], true, "回退标注真值");
        assert_eq!(
            routing["fallback_reason"],
            "no model row passes the capable filter"
        );
        assert_eq!(routing["selected_model"], "default-model");
        assert_eq!(
            routing["price_display"], PRICE_NOT_PROVIDED,
            "未知模型绝不编价"
        );
    }

    #[test]
    fn the_cost_routing_section_without_a_decision_is_explicit_null_with_a_reason() {
        // 开关关 (fail-closed): 档位/选定显式 null + 如实原因, 不冒充在路由。
        let mut snapshot = fixture_snapshot();
        snapshot.cost_routing = Some(CostRoutingStatus {
            enabled: false,
            decision: None,
            reason: Some(
                "cost-tier routing switch is off (APEIRETH_COST_ROUTING=1 enables it); no tier is in force"
                    .to_string(),
            ),
        });
        let value = render_snapshot(&snapshot);
        let routing = &value["cost_routing"];
        assert_eq!(routing["enabled"], false);
        assert_eq!(routing["tier"], Value::Null);
        assert_eq!(routing["selected_model"], Value::Null);
        assert_eq!(routing["used_fallback"], Value::Null, "没决策不谎报 false");
        assert!(routing["reason"]
            .as_str()
            .unwrap()
            .contains("APEIRETH_COST_ROUTING"));

        // 无来源: 整节显式 null + 原因 (0 装诚实)。
        snapshot.cost_routing = None;
        snapshot.cost_routing_reason = Some("routing source not wired".to_string());
        let value = render_snapshot(&snapshot);
        let routing = &value["cost_routing"];
        assert_eq!(routing["enabled"], Value::Null);
        assert_eq!(routing["tier"], Value::Null);
        assert_eq!(routing["candidates"], json!([]));
        assert_eq!(routing["price_display"], PRICE_NOT_PROVIDED);
        assert!(routing["reason"]
            .as_str()
            .unwrap()
            .contains("routing source not wired"));
    }

    /// 创作节 (创作工具面 W1): 最近操作计数如实, 只计数不回路径 (脱敏口径);
    /// 缺来源 = 整节显式 null + 原因 (0 装诚实, 不编数)。
    #[test]
    fn image_ops_creation_section_reports_operation_counts_without_paths() {
        let value = render_snapshot(&fixture_snapshot());
        let creation = &value["image_ops"];
        assert_eq!(creation["calls"], 3);
        assert_eq!(creation["ops_applied"], 5);
        assert_eq!(creation["denied"], 1);
        assert_eq!(creation["by_operation"][0]["operation"], "resize");
        assert_eq!(creation["by_operation"][0]["count"], 2);
        assert_eq!(creation["last_call_ops"], "resize -> grayscale");
        // 脱敏口径: 创作节零路径零文件名 (节内不出现路径分隔符/扩展名文件名)。
        let rendered = creation.to_string();
        assert!(!rendered.contains('/'), "创作节零路径: {rendered}");
        assert!(!rendered.contains('\\'), "创作节零路径: {rendered}");
        assert!(!rendered.contains(".png"), "创作节零文件名: {rendered}");
        assert!(creation["last_op_at_ms"].is_number());

        // 尚无操作: last_op_at_ms / last_call_ops 显式 null + 如实注, 不编时刻。
        let mut snapshot = fixture_snapshot();
        snapshot.image_ops = Some(ImageOpsStatus {
            calls: 0,
            ops_applied: 0,
            denied: 0,
            by_operation: vec![],
            last_op_at_ms: None,
            last_call_ops: None,
            note: Some("尚无操作记录".to_string()),
            reason: None,
        });
        let value = render_snapshot(&snapshot);
        assert_eq!(value["image_ops"]["last_op_at_ms"], Value::Null);
        assert_eq!(value["image_ops"]["last_call_ops"], Value::Null);
        assert_eq!(value["image_ops"]["note"], "尚无操作记录");

        // 缺来源: 整节显式 null + 原因 (0 装诚实)。
        let mut snapshot = fixture_snapshot();
        snapshot.image_ops = None;
        snapshot.image_ops_reason = Some("image ops source not wired".to_string());
        let value = render_snapshot(&snapshot);
        assert_eq!(value["image_ops"]["calls"], Value::Null);
        assert_eq!(value["image_ops"]["by_operation"], Value::Null);
        assert!(value["image_ops"]["reason"]
            .as_str()
            .unwrap()
            .contains("image ops source not wired"));
    }

    /// 创作节 (创作工具面 W2 · 文档操作): 进程内计数如实 (页数/拒绝/逐操作),
    /// 只计数不回路径; 缺来源 = 整节显式 null + 原因 (0 装诚实, 不编数)。
    #[test]
    fn doc_ops_creation_section_reports_page_counts_without_paths() {
        let value = render_snapshot(&fixture_snapshot());
        let section = &value["doc_ops"];
        assert_eq!(section["calls"], 5);
        assert_eq!(section["pages_touched"], 9);
        assert_eq!(section["denied"], 1);
        assert_eq!(section["by_operation"][0]["operation"], "pdf_info");
        assert_eq!(section["by_operation"][0]["count"], 2);
        assert_eq!(section["by_operation"].as_array().unwrap().len(), 8);
        assert_eq!(section["last_action"], "pdf_extract_pages");
        assert!(section["last_op_at_ms"].is_number());

        // 尚无操作: 显式 null + 如实注, 不编时刻不编动作。
        let mut snapshot = fixture_snapshot();
        snapshot.doc_ops = Some(DocOpsStatus {
            calls: 0,
            pages_touched: 0,
            denied: 0,
            by_operation: vec![],
            last_op_at_ms: None,
            last_action: None,
            note: Some("尚无操作记录".to_string()),
            reason: None,
        });
        let value = render_snapshot(&snapshot);
        assert_eq!(value["doc_ops"]["last_op_at_ms"], Value::Null);
        assert_eq!(value["doc_ops"]["last_action"], Value::Null);
        assert_eq!(value["doc_ops"]["note"], "尚无操作记录");

        // 缺来源: 整节显式 null + 原因 (0 装诚实)。
        let mut snapshot = fixture_snapshot();
        snapshot.doc_ops = None;
        snapshot.doc_ops_reason = Some("doc ops source not wired".to_string());
        let value = render_snapshot(&snapshot);
        assert_eq!(value["doc_ops"]["calls"], Value::Null);
        assert_eq!(value["doc_ops"]["by_operation"], Value::Null);
        assert!(value["doc_ops"]["reason"]
            .as_str()
            .unwrap()
            .contains("doc ops source not wired"));
    }

    /// 表格操作创作节 (K3): 逐动作计数如实, 零路径零文件名 (脱敏口径)。
    #[test]
    fn sheet_ops_creation_section_reports_action_counts_without_paths() {
        let value = render_snapshot(&fixture_snapshot());
        let section = &value["sheet_ops"];
        assert_eq!(section["calls"], 4);
        assert_eq!(section["rows_touched"], 12);
        assert_eq!(section["denied"], 1);
        assert_eq!(section["by_operation"][0]["operation"], "read");
        assert_eq!(section["by_operation"][0]["count"], 1);
        assert_eq!(section["by_operation"].as_array().unwrap().len(), 7);
        assert_eq!(section["last_action"], "aggregate");
        assert!(section["last_op_at_ms"].is_number());

        // 脱敏口径: 本节零路径零文件名。
        let rendered = value.to_string();
        assert!(!rendered.contains(".csv"), "{rendered}");

        // 尚无操作: 显式 null + 如实注, 不编时刻不编动作。
        let mut snapshot = fixture_snapshot();
        snapshot.sheet_ops = Some(SheetOpsStatus {
            calls: 0,
            rows_touched: 0,
            denied: 0,
            by_operation: vec![],
            last_op_at_ms: None,
            last_action: None,
            note: Some("尚无操作记录".to_string()),
            reason: None,
        });
        let value = render_snapshot(&snapshot);
        assert_eq!(value["sheet_ops"]["last_op_at_ms"], Value::Null);
        assert_eq!(value["sheet_ops"]["last_action"], Value::Null);
        assert_eq!(value["sheet_ops"]["note"], "尚无操作记录");

        // 缺来源: 整节显式 null + 原因 (0 装诚实)。
        let mut snapshot = fixture_snapshot();
        snapshot.sheet_ops = None;
        snapshot.sheet_ops_reason = Some("sheet ops source not wired".to_string());
        let value = render_snapshot(&snapshot);
        assert_eq!(value["sheet_ops"]["calls"], Value::Null);
        assert_eq!(value["sheet_ops"]["by_operation"], Value::Null);
        assert!(value["sheet_ops"]["reason"]
            .as_str()
            .unwrap()
            .contains("sheet ops source not wired"));
    }

    /// W3.2 生物机制族: 14 条机制的闭合词表 (与台账 SG-13…SG-26 同名同序,
    /// 不增不减不漂移; 名字唯一)。
    #[test]
    fn bio_mechanism_kind_vocabulary_is_the_closed_fourteen() {
        assert_eq!(BioMechanismKind::ALL.len(), 14);
        let names: Vec<&str> = BioMechanismKind::ALL
            .iter()
            .map(|kind| kind.as_str())
            .collect();
        assert_eq!(
            names,
            [
                "neuromodulation",
                "homeostasis",
                "circadian",
                "immune_memory",
                "nociception",
                "recovery",
                "attachment",
                "social_repair",
                "quorum",
                "lifecycle",
                "play",
                "ritual",
                "ecosystem_health",
                "capacity",
            ]
        );
        let unique: std::collections::BTreeSet<&str> = names.iter().copied().collect();
        assert_eq!(unique.len(), names.len(), "词表名必须唯一");
        for (kind, name) in BioMechanismKind::ALL.iter().zip(names) {
            assert_eq!(BioMechanismKind::from_str_name(name), Some(*kind));
        }
        assert_eq!(BioMechanismKind::from_str_name("unknown"), None);
        // serde 名与稳定名同轴 (JSON 键不随 Debug 派生漂移)。
        assert_eq!(
            serde_json::to_value(BioMechanismKind::ImmuneMemory).unwrap(),
            json!("immune_memory")
        );
    }

    /// W3.2 env 开关惯例: 只认显式 "1", 其余取值一律关 (fail-closed)。
    #[test]
    fn bio_self_status_switch_only_accepts_explicit_one() {
        assert!(!bio_self_status_enabled_from_env_value(None));
        assert!(bio_self_status_enabled_from_env_value(Some("1")));
        for raw in ["0", "true", "TRUE", "yes", "on", "", "01", " 1 "] {
            assert!(
                !bio_self_status_enabled_from_env_value(Some(raw)),
                "{raw:?} 不等于 \"1\", 必须为关"
            );
        }
        assert_eq!(
            BIO_SELF_STATUS_ENABLE_ENV,
            "APEIRETH_ENABLE_BIO_SELF_STATUS"
        );
    }

    /// W3.2 serde 兼容: 局部/旧载荷缺字段 = 按「未接线」缺省, 不读崩;
    /// 六字段合同序列化完整。
    #[test]
    fn bio_mechanism_reading_serde_is_forward_compatible() {
        let missing: BioMechanismReading = serde_json::from_value(json!({})).expect("defaults");
        assert_eq!(missing.stage, BIO_STAGE_NOT_WIRED);
        assert!(!missing.wired);
        assert!(missing.value.is_none());
        assert!(missing.threshold.is_none());
        assert!(missing.last_action.is_none());
        assert_eq!(
            missing.reason.as_deref(),
            Some(BIO_MECHANISM_NOT_WIRED_REASON)
        );

        let wired: BioMechanismReading =
            serde_json::from_value(json!({ "wired": true, "value": 3 })).expect("partial");
        assert!(wired.wired);
        assert_eq!(wired.value, Some(json!(3)));
        assert!(wired.threshold.is_none(), "缺字段 = 显式 null, 不编 0");

        // 六字段合同: 序列化后一个不少。
        let encoded = serde_json::to_value(BioMechanismReading::wired(
            Some(json!({"active_tasks": 1})),
            Some(4.0),
            Some("一档".to_string()),
        ))
        .expect("serialize");
        for field in [
            "stage",
            "value",
            "threshold",
            "last_action",
            "wired",
            "reason",
        ] {
            assert!(encoded.get(field).is_some(), "缺字段 {field}: {encoded}");
        }
        assert_eq!(encoded["stage"], BIO_STAGE_WIRED);
        assert_eq!(encoded["wired"], true);
        assert!(encoded["reason"].is_null());

        // 聚合子对象同样向前兼容: 空载荷 = 未启用缺省。
        let status: BioMechanismsStatus = serde_json::from_value(json!({})).expect("defaults");
        assert!(!status.enabled);
        assert!(status.mechanisms.is_none());
        assert!(status
            .reason
            .as_deref()
            .unwrap()
            .contains(BIO_SELF_STATUS_ENABLE_ENV));
    }

    /// W3.2 兼容旧消费者: 观测面未挂载 = capabilities 逐键仍是布尔, 一个键不多
    /// （既有 e2e 的「名册全为布尔」断言不被生物聚合破）。
    #[test]
    fn bio_mechanisms_absent_keeps_the_legacy_capabilities_shape() {
        let mut snapshot = fixture_snapshot();
        snapshot.bio_mechanisms = None;
        let value = render_snapshot(&snapshot);
        let capabilities = value["capabilities"].as_object().expect("roster object");
        assert!(
            capabilities.get(CAPABILITIES_BIO_MECHANISMS).is_none(),
            "未挂观测面不得出现子对象键"
        );
        assert_eq!(capabilities.len(), snapshot.capabilities.len());
        for (name, switch) in capabilities {
            assert!(switch.is_boolean(), "{name} 必须仍是布尔: {switch}");
        }
        assert!(SelfStatusTool::output_schema().validate(&value).is_ok());
    }

    /// D32 建议②：**键位保留 + 节旁显式标注是读数**（两条都钉住）。
    /// D32 建议②：**键位保留 + 节旁显式标注是读数**（两条都钉住）。
    /// ① `note` 在场且措辞在顾问词表内；② 5 条体验名键位**必须仍在**（守卫「不越界」）。
    #[test]
    fn bio_mechanisms_note_declares_readings_without_changing_keys() {
        let value = render_snapshot(&fixture_snapshot());
        let bio = &value["capabilities"][CAPABILITIES_BIO_MECHANISMS];
        let note = bio["note"].as_str().expect("读数标注文案在场");
        assert!(
            note.contains("检测读数"),
            "标注文案必须说明这是检测读数: {note}"
        );
        assert!(
            note.contains("不是内在体验"),
            "标注文案必须否定「体验」读法: {note}"
        );
        assert!(
            !note.contains("我有") && !note.contains("我感到"),
            "不得反过来声称体验: {note}"
        );
        let rows = bio["mechanisms"].as_object().expect("mechanisms object");
        for wire in ["nociception", "attachment", "ritual", "quorum", "play"] {
            assert!(
                rows.contains_key(wire),
                "键位 {wire} 本波不得改动（冻结合同）"
            );
        }
        assert!(SelfStatusTool::output_schema().validate(&value).is_ok());
    }

    #[test]
    fn bio_mechanisms_disabled_renders_null_mechanisms_with_a_reason() {
        let mut snapshot = fixture_snapshot();
        snapshot.bio_mechanisms = Some(BioMechanismsStatus::disabled(
            BIO_SELF_STATUS_DISABLED_REASON,
        ));
        let value = render_snapshot(&snapshot);
        let bio = &value["capabilities"][CAPABILITIES_BIO_MECHANISMS];
        assert_eq!(bio["enabled"], false);
        assert_eq!(bio["mechanisms"], Value::Null);
        assert!(bio["reason"].as_str().unwrap().contains("未启用"));
        assert!(bio["reason"]
            .as_str()
            .unwrap()
            .contains(BIO_SELF_STATUS_ENABLE_ENV));
        assert!(SelfStatusTool::output_schema().validate(&value).is_ok());
    }

    /// W3.2 启用面: 14 格逐格六字段合同 —— 已接线格带真值/阈值/最近动作,
    /// 未接线格 `wired:false` + 值 null + 原因「未接线」(不编数值)。
    #[test]
    fn bio_mechanisms_wired_and_unwired_rows_carry_the_frozen_fields() {
        let value = render_snapshot(&fixture_snapshot());
        let bio = &value["capabilities"][CAPABILITIES_BIO_MECHANISMS];
        assert_eq!(bio["enabled"], true);
        assert!(bio["reason"].is_null());
        let rows = bio["mechanisms"].as_object().expect("mechanisms object");
        assert_eq!(rows.len(), BioMechanismKind::ALL.len());

        let capacity = &rows["capacity"];
        assert_eq!(capacity["stage"], BIO_STAGE_WIRED);
        assert_eq!(capacity["wired"], true);
        assert_eq!(capacity["value"]["active_tasks"], 2);
        assert_eq!(capacity["threshold"], 8.0);
        assert_eq!(capacity["last_action"], "任务队列：2 个执行槽在跑");
        assert!(capacity["reason"].is_null());

        for kind in BioMechanismKind::ALL {
            let row = &rows[kind.as_str()];
            for field in [
                "stage",
                "value",
                "threshold",
                "last_action",
                "wired",
                "reason",
            ] {
                assert!(row.get(field).is_some(), "{kind:?} 缺字段 {field}: {row}");
            }
            if kind != BioMechanismKind::Capacity {
                assert_eq!(row["wired"], false, "{kind:?}");
                assert_eq!(row["stage"], BIO_STAGE_NOT_WIRED, "{kind:?}");
                assert_eq!(row["value"], Value::Null, "{kind:?}");
                assert_eq!(row["threshold"], Value::Null, "{kind:?}");
                assert_eq!(row["last_action"], Value::Null, "{kind:?}");
                assert!(
                    row["reason"].as_str().unwrap().contains("未接线"),
                    "{kind:?}: {row}"
                );
            }
        }
    }

    /// W3.2 合同边界: 生物聚合**不新增顶层节** —— 21 节不变, schema 仍是
    /// 冻结节表, 渲染面顶层键集一致; 同一快照重复渲染逐字节一致 (确定性)。
    #[test]
    fn bio_mechanisms_adds_no_top_level_section_and_renders_deterministically() {
        assert_eq!(SELF_STATUS_SECTIONS.len(), 21, "冻结合同仍是二十一节");
        assert!(!SELF_STATUS_SECTIONS.contains(&CAPABILITIES_BIO_MECHANISMS));
        let schema = SelfStatusTool::output_schema();
        let declared: Vec<&str> = schema
            .fields()
            .iter()
            .map(|field| field.name.as_str())
            .collect();
        assert_eq!(declared, SELF_STATUS_SECTIONS);

        let snapshot = fixture_snapshot();
        let first = render_snapshot(&snapshot);
        let second = render_snapshot(&snapshot);
        assert_eq!(first, second, "同输入必须同输出");
        assert!(first.get(CAPABILITIES_BIO_MECHANISMS).is_none());
        let rendered: std::collections::BTreeSet<&str> = first
            .as_object()
            .expect("object frame")
            .keys()
            .map(String::as_str)
            .collect();
        let frozen: std::collections::BTreeSet<&str> =
            SELF_STATUS_SECTIONS.iter().copied().collect();
        assert_eq!(rendered, frozen);
        assert!(schema.validate(&first).is_ok());
    }
}
