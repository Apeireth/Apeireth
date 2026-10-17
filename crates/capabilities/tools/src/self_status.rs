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

use std::collections::BTreeMap;
use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, SessionId};
use apeireth_core::model_profile::ModelProfile;
use apeireth_core::RELEASE_VERSION;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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

/// 记忆账本计数 (纯计数元数据; 缺失的计数显式 `None` → JSON `null`)。
///
/// **永不回内容原文**: 只有数字, 没有记忆文本。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryLedgerStats {
    /// 会话数 (episode 流里的不同会话计数)。
    pub sessions: Option<u64>,
    /// 记忆件数 (episode 计数)。
    pub memories: Option<u64>,
    /// 保护件数 (治理层标记为保护的记忆计数)。
    pub protected: Option<u64>,
    /// 教训数 (反思沉淀的教训计数)。
    pub lessons: Option<u64>,
    /// 待摄取数 (V4-b 保鲜回路: 摄取队列 `pending` 真值; 缺源 = `None` 显式 null)。
    pub pending_ingest: Option<u64>,
    /// 最近折入真值 (V4-b: epoch 毫秒; 尚未折入 = `None`, 显式 null 不编时间)。
    pub last_fold_at_ms: Option<i64>,
    /// 去重计数 (V4-b: 散列增量门跳过重复摄取的累计真值; 缺源 = `None`)。
    pub dedup_skipped: Option<u64>,
    /// 局部缺失原因 (某计数探测失败时给出; 与字段级 null 配对)。
    pub reason: Option<String>,
}

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
}

impl SelfStatusSnapshot {
    /// 只有身份与名册的最小自述 (其余字段显式 null + 原因)。
    pub fn minimal(identity: SelfStatusIdentity, capabilities: Vec<CapabilitySwitch>) -> Self {
        Self {
            identity,
            capabilities,
            memory_ledger: None,
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
];

/// 把一份自述投影成结构化 JSON (字段合同的唯一实现)。
///
/// 合同要点: **所有字段恒在场**; 来源缺失/失败的字段显式 `null` 并在同节
/// `reason` 给出原因 —— 不是缺字段, 不是整帧失败。
pub fn render_snapshot(snapshot: &SelfStatusSnapshot) -> Value {
    let capabilities: serde_json::Map<String, Value> = snapshot
        .capabilities
        .iter()
        .map(|entry| (entry.name.clone(), Value::Bool(entry.enabled)))
        .collect();

    let memory_ledger = match &snapshot.memory_ledger {
        Some(stats) => json!({
            "sessions": stats.sessions,
            "memories": stats.memories,
            "protected": stats.protected,
            "lessons": stats.lessons,
            // V4-b 保鲜回路真值 (待摄取数 / 最近折入 / 去重计数); 缺源显式 null。
            "pending_ingest": stats.pending_ingest,
            "last_fold_at_ms": stats.last_fold_at_ms,
            "dedup_skipped": stats.dedup_skipped,
            "reason": snapshot
                .memory_ledger_reason
                .clone()
                .or_else(|| stats.reason.clone()),
        }),
        None => json!({
            "sessions": null,
            "memories": null,
            "protected": null,
            "lessons": null,
            "pending_ingest": null,
            "last_fold_at_ms": null,
            "dedup_skipped": null,
            "reason": snapshot
                .memory_ledger_reason
                .clone()
                .unwrap_or_else(|| DATA_PROBE_NOT_WIRED.to_string()),
        }),
    };

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
            "reason": snapshot.care_reason.clone().or_else(|| status.reason.clone()),
        }),
        None => json!({
            "rules": null,
            "next_trigger_at_ms": null,
            "last_care_at_ms": null,
            "last_care_note": null,
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
        "domain": domain,
        "content_feed": content_feed,
        "care": care,
        "backup": backup,
        "voice": voice,
        "task_board": task_board,
        "persona": persona,
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
    /// 用户域节随 W3-① (多户数据隔离) 入合同: 当前域 id/角色/域内计数自述化;
    /// 内容流节随 J1 (内容流接入) 入合同: 订阅数/最近拉取自述化;
    /// 关怀节随 V3-a (双引擎主动关怀) 入合同: 规则数/下次触发/最近关怀自述化;
    /// 备份节随 G5 (备份恢复) 入合同: 最近备份时间/清单件数自述化;
    /// 任务簿节随 V6-a (任务编排持久队列) 入合同: 分态计数自述化;
    /// 人格节随 V4-a (人格记忆层) 入合同: 信念/边界/偏好/张力计数 + 最后整编
    /// 真值自述化 (只计数不回内容)。
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
                 ledger counts (numbers only), tuning state, turn budgets, workspace facts, and \
                 the active model profile (context window, tool support, unit prices — missing \
                 prices read as not provided, never invented). Reports measured state only; \
                 credential values are never echoed.",
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
            memory_ledger: Some(MemoryLedgerStats {
                sessions: Some(2),
                memories: Some(5),
                protected: Some(1),
                lessons: Some(0),
                pending_ingest: Some(3),
                last_fold_at_ms: Some(1_700_000_000_000),
                dedup_skipped: Some(4),
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
        }
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
        // 缺节即红: fixture 的渲染必须带出冻结节表的每一节 (十四节合同终核)。
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
    /// 如实注, 不编时间不编判定。
    #[test]
    fn care_section_reports_rule_count_next_trigger_and_last_care_honestly() {
        let mut snapshot = fixture_snapshot();
        snapshot.care = Some(CareStatus {
            rules: Some(1),
            next_trigger_at_ms: Some(1_728_100_000_000),
            last_care_at_ms: None,
            last_care_note: Some("尚无关怀记录".to_string()),
            reason: None,
        });
        let value = render_snapshot(&snapshot);
        assert_eq!(value["care"]["rules"], 1);
        assert_eq!(value["care"]["next_trigger_at_ms"], 1_728_100_000_000i64);
        assert_eq!(value["care"]["last_care_at_ms"], Value::Null);
        assert_eq!(value["care"]["last_care_note"], "尚无关怀记录");
        assert_eq!(value["care"]["reason"], Value::Null);

        // 探测口失败 = 全维显式 null + 原因 (0 装诚实)。
        let unavailable = CareStatus::unavailable("care_rules.json refused to open");
        assert_eq!(unavailable.rules, None);
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

    /// 十四节合同终核: 节序显式、schema require 清单全、渲染缺一节即红 ——
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
            sessions: Some(1),
            memories: None,
            protected: None,
            lessons: None,
            pending_ingest: None,
            last_fold_at_ms: None,
            dedup_skipped: None,
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
}
