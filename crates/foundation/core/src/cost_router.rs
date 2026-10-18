//! 成本分级路由 (`CostRouter`) —— 模型面轴 (H-P2「更省」) 的任务选型面。
//!
//! 任务按**成本/能力**自动选模型: 任务描述先定**成本档** (分级词表
//! [`CostTier`]: `frugal|standard|capable|premium`), 再按档位的筛选条件
//! (能力需求 × 单价上限, 事实全部来自 [`crate::model_profile`] 画像表) 过滤
//! 出候选模型并选定。省钱第一杠杆, 但**绝不编价**: 画像表没有的价格一个字
//! 都不补 (与 [`crate::model_profile`] 的「缺省 = 未提供」同一口径)。
//!
//! # 分级词表 (每档 = 模型筛选条件: 能力需求 × 单价上限)
//!
//! | 档位 | 任务语义 | 工具需求 | 窗口下限 (token) | 单价上限 (微元/千 token) |
//! |------|----------|----------|------------------|--------------------------|
//! | [`CostTier::Frugal`]   | 杂活/轻任务 | 不强制 | [`FRUGAL_MIN_CONTEXT_WINDOW`]   | [`FRUGAL_PRICE_CEILING_MICROS`]   |
//! | [`CostTier::Standard`] | 常规工作 (默认档) | 需工具 | [`STANDARD_MIN_CONTEXT_WINDOW`] | [`STANDARD_PRICE_CEILING_MICROS`] |
//! | [`CostTier::Capable`]  | 重活 (多步工具/大上下文) | 需工具 | [`CAPABLE_MIN_CONTEXT_WINDOW`] | [`CAPABLE_PRICE_CEILING_MICROS`] |
//! | [`CostTier::Premium`]  | 深思 (最难任务) | 需工具 | [`PREMIUM_MIN_CONTEXT_WINDOW`] | [`PREMIUM_PRICE_CEILING_MICROS`] |
//!
//! 表里的**单价上限是预算策略常量** (这一档最多想花多少), 不是任何模型的
//! 价格事实 —— 模型单价只来自画像表, 缺省就是缺省。
//!
//! # 路由规则表 (任务描述 → 档位 → 候选 → 选定)
//!
//! 1. **任务描述 → 档位**: 显式档 ([`RoutingRequest::explicit_tier`]) 最高;
//!    其次规则推断 (工具调用数/上下文规模启发式 [`infer_tier`]); 都没有 =
//!    默认 [`CostTier::Standard`]。推断口径: 重信号单证即升档 (能力风险
//!    优先), 轻档需双证 (缺信号不猜轻)。
//! 2. **档位 → 候选** ([`CostRouter::candidates`] 的过滤条件):
//!    支持工具 (档位或任务要求工具时必须 `supports_tools == Some(true)`,
//!    未知不等于支持) / 窗口够 (`context_window` 显式 ≥ 档位下限与任务上下文
//!    规模的较大者, 未知不算够) / 单价 ≤ 档上限 (比较单价见
//!    [`comparison_price_micros`]; **无价/缺价行不参与价格比较**, 按能力入
//!    候选, 价格标注照实「未提供」)。
//! 3. **候选 → 选定** (顺序即优先级):
//!    ① 档位映射配置的首选模型 (在候选内才生效, 不在候选如实注明后顺延);
//!    ② 有价候选里比较单价最低者 (等权合计口径; 平手取画像表顺序);
//!    ③ 默认模型在候选内 = 保持默认 (无价格证据不换模型, 稳定性优先);
//!    ④ 画像表顺序第一个候选 (能力合格, 价格照实未提供)。
//! 4. **无候选 = 回退默认模型 + 如实标注** ([`RoutingDecision::used_fallback`]):
//!    默认模型也不过档位过滤时照实注明「仅作显式回退」; 默认模型缺席则
//!    如实选不出模型, 不硬凑。
//!
//! # 配置面
//!
//! - [`COST_ROUTING_ENV`] (`APEIRETH_COST_ROUTING`): 主开关, **fail-closed**
//!   —— 只有精确 `1` 开, 其他一律关;
//! - [`COST_ROUTING_PREFERRED_ENV`] (`APEIRETH_COST_ROUTING_PREFERRED`):
//!   档位映射配置 (每档首选模型, JSON 对象 `{"frugal": "<model id>", ...}`),
//!   任一缺陷整体拒开, 值自由报错。
//!
//! # 诚实口径
//!
//! - **无价模型** (`input_price_micros`/`output_price_micros` 任一缺): 成本
//!   算不出就不参与价格比较 (与画像表 [`crate::model_profile`] 冻结的
//!   `CostRateTable` 换算口径同源: 缺一半回 `None`), 按能力入候选, 价格
//!   标注照实「未提供」——**绝不编价**;
//! - 路由决策可解释: [`RoutingDecision::selection_reason`] 说明为什么选定,
//!   回退带 [`RoutingDecision::fallback_reason`] 真值标注;
//! - 自述路由节 (当前档/选定模型/回退标注) 的真值来源是
//!   [`CostRouter::status`]: 开关真值 + 最近一次路由决策, 没路由过就照实
//!   说没路由过。

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Mutex;

use serde_json::Value;

use crate::model_profile::{normalize_model_id, ModelProfile, ModelProfileTable};

/// env 入口: 成本分级路由主开关 (`APEIRETH_COST_ROUTING`)。
///
/// **fail-closed**: 只有精确 `1` (去首尾空白) 开, 其他取值一律关。
pub const COST_ROUTING_ENV: &str = "APEIRETH_COST_ROUTING";

/// env 入口: 档位映射配置 (`APEIRETH_COST_ROUTING_PREFERRED`)。
///
/// JSON 对象 `{"<tier>": "<model id>", ...}` —— 覆盖每档首选模型; 空白值 =
/// 无覆盖。任一缺陷整体拒开 (fail-closed, 值自由报错)。
pub const COST_ROUTING_PREFERRED_ENV: &str = "APEIRETH_COST_ROUTING_PREFERRED";

/// 轻档窗口下限 (token)。routing floor, **不是**模型规格书。
pub const FRUGAL_MIN_CONTEXT_WINDOW: u64 = 8_192;

/// 轻档单价上限 (微元/千 token, 比较单价合计口径)。预算策略常量。
pub const FRUGAL_PRICE_CEILING_MICROS: u64 = 5_000;

/// 标准档窗口下限 (token)。routing floor, **不是**模型规格书。
pub const STANDARD_MIN_CONTEXT_WINDOW: u64 = 32_768;

/// 标准档单价上限 (微元/千 token, 比较单价合计口径)。预算策略常量。
pub const STANDARD_PRICE_CEILING_MICROS: u64 = 20_000;

/// 重活档窗口下限 (token)。routing floor, **不是**模型规格书。
pub const CAPABLE_MIN_CONTEXT_WINDOW: u64 = 65_536;

/// 重活档单价上限 (微元/千 token, 比较单价合计口径)。预算策略常量。
pub const CAPABLE_PRICE_CEILING_MICROS: u64 = 80_000;

/// 深思档窗口下限 (token)。routing floor, **不是**模型规格书。
pub const PREMIUM_MIN_CONTEXT_WINDOW: u64 = 131_072;

/// 深思档单价上限 (微元/千 token, 比较单价合计口径)。预算策略常量。
pub const PREMIUM_PRICE_CEILING_MICROS: u64 = 400_000;

/// 启发式: 期望工具调用数 ≥ 此值 = 深思档信号。
pub const HEURISTIC_PREMIUM_TOOL_CALLS: u64 = 32;

/// 启发式: 上下文规模 ≥ 此值 (token) = 深思档信号。
pub const HEURISTIC_PREMIUM_CONTEXT_TOKENS: u64 = 200_000;

/// 启发式: 期望工具调用数 ≥ 此值 = 重活档信号。
pub const HEURISTIC_CAPABLE_TOOL_CALLS: u64 = 8;

/// 启发式: 上下文规模 ≥ 此值 (token) = 重活档信号。
pub const HEURISTIC_CAPABLE_CONTEXT_TOKENS: u64 = 64_000;

/// 启发式: 上下文规模 ≤ 歜值 (token) 且零工具调用 = 轻活档信号
/// (与 [`FRUGAL_MIN_CONTEXT_WINDOW`] 同值: 轻任务装得进轻档窗口)。
pub const HEURISTIC_FRUGAL_CONTEXT_TOKENS: u64 = FRUGAL_MIN_CONTEXT_WINDOW;

/// 任务成本档 (分级词表)。四档闭合: `frugal|standard|capable|premium`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CostTier {
    /// 杂活/轻任务: 不强制工具, 最低预算档。
    Frugal,
    /// 常规工作 (默认档): 需工具支持。
    Standard,
    /// 重活 (多步工具/大上下文): 需工具支持 + 更大窗口。
    Capable,
    /// 深思 (最难任务): 需工具支持 + 最大窗口, 预算最高档。
    Premium,
}

impl CostTier {
    /// 分级词表全集 (顺序即档位阶梯)。
    pub const ALL: [CostTier; 4] = [
        CostTier::Frugal,
        CostTier::Standard,
        CostTier::Capable,
        CostTier::Premium,
    ];

    /// 词表拼写 (`frugal|standard|capable|premium`)。
    pub const fn as_str(self) -> &'static str {
        match self {
            CostTier::Frugal => "frugal",
            CostTier::Standard => "standard",
            CostTier::Capable => "capable",
            CostTier::Premium => "premium",
        }
    }

    /// 词表解析 (ASCII 大小写不敏感); 未知词 = `None` (词表闭合, 不猜档)。
    pub fn parse(name: &str) -> Option<CostTier> {
        CostTier::ALL
            .into_iter()
            .find(|tier| tier.as_str() == name.to_ascii_lowercase())
    }

    /// 档位筛选条件 (能力需求 × 单价上限 × 窗口下限, 全部为策略常量)。
    pub const fn spec(self) -> TierSpec {
        match self {
            CostTier::Frugal => TierSpec {
                tier: CostTier::Frugal,
                requires_tools: false,
                min_context_window: FRUGAL_MIN_CONTEXT_WINDOW,
                price_ceiling_micros: FRUGAL_PRICE_CEILING_MICROS,
            },
            CostTier::Standard => TierSpec {
                tier: CostTier::Standard,
                requires_tools: true,
                min_context_window: STANDARD_MIN_CONTEXT_WINDOW,
                price_ceiling_micros: STANDARD_PRICE_CEILING_MICROS,
            },
            CostTier::Capable => TierSpec {
                tier: CostTier::Capable,
                requires_tools: true,
                min_context_window: CAPABLE_MIN_CONTEXT_WINDOW,
                price_ceiling_micros: CAPABLE_PRICE_CEILING_MICROS,
            },
            CostTier::Premium => TierSpec {
                tier: CostTier::Premium,
                requires_tools: true,
                min_context_window: PREMIUM_MIN_CONTEXT_WINDOW,
                price_ceiling_micros: PREMIUM_PRICE_CEILING_MICROS,
            },
        }
    }
}

impl fmt::Display for CostTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 一档的模型筛选条件 (能力需求 × 单价上限)。
///
/// 这些数字是**档位语义的一部分 (预算策略常量)**, 不是任何模型的价格/规格
/// 事实 —— 模型事实只来自画像表。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TierSpec {
    /// 档位。
    pub tier: CostTier,
    /// 能力需求: 是否强制要求工具支持 (任务侧要求会再抬高, 见路由规则表)。
    pub requires_tools: bool,
    /// 窗口下限 (token, routing floor 不是规格书)。
    pub min_context_window: u64,
    /// 单价上限 (微元/千 token, 比较单价合计口径)。
    pub price_ceiling_micros: u64,
}

/// 档位来源 (可解释性: 档位是怎么定下来的)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TierSource {
    /// 显式档: 调用方点名 (最高优先)。
    Explicit,
    /// 规则推断: 工具调用数/上下文规模启发式。
    Inferred,
    /// 默认档: 信号缺位, 落 [`CostTier::Standard`]。
    Default,
}

impl TierSource {
    /// 来源拼写 (`explicit|inferred|default`)。
    pub const fn as_str(self) -> &'static str {
        match self {
            TierSource::Explicit => "explicit",
            TierSource::Inferred => "inferred",
            TierSource::Default => "default",
        }
    }
}

/// 一个任务的路由信号 (任务描述的路由投影)。
///
/// 信号缺位就照实 `None` —— **不猜** (缺信号不推轻档, 见 [`infer_tier`])。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RoutingRequest {
    /// 显式档 (调用方点名): 最高优先, 压过一切启发式。
    pub explicit_tier: Option<CostTier>,
    /// 期望工具调用数; `None` = 信号缺位。
    pub expected_tool_calls: Option<u64>,
    /// 上下文规模估计 (token); `None` = 信号缺位。
    pub estimated_context_tokens: Option<u64>,
    /// 任务本身是否需要工具 (能力需求): 需要时候选必须显式支持工具。
    pub requires_tools: bool,
}

impl RoutingRequest {
    /// 全信号缺位的任务描述 (路由结果 = 默认档 [`CostTier::Standard`])。
    pub fn new() -> Self {
        Self::default()
    }

    /// 点名显式档。
    #[must_use]
    pub fn with_explicit_tier(mut self, tier: CostTier) -> Self {
        self.explicit_tier = Some(tier);
        self
    }

    /// 声明期望工具调用数。
    #[must_use]
    pub fn with_expected_tool_calls(mut self, tool_calls: u64) -> Self {
        self.expected_tool_calls = Some(tool_calls);
        self
    }

    /// 声明上下文规模估计 (token)。
    #[must_use]
    pub fn with_estimated_context_tokens(mut self, tokens: u64) -> Self {
        self.estimated_context_tokens = Some(tokens);
        self
    }

    /// 声明任务是否需要工具。
    #[must_use]
    pub fn with_requires_tools(mut self, requires_tools: bool) -> Self {
        self.requires_tools = requires_tools;
        self
    }
}

/// 任务描述 → 成本档 (规则推断口径, 显式档由调用方先压)。
///
/// 推断阶梯 (重信号单证即升档, 轻档需双证):
/// 1. 工具调用数 ≥ [`HEURISTIC_PREMIUM_TOOL_CALLS`] 或上下文 ≥
///    [`HEURISTIC_PREMIUM_CONTEXT_TOKENS`] → [`CostTier::Premium`];
/// 2. 工具调用数 ≥ [`HEURISTIC_CAPABLE_TOOL_CALLS`] 或上下文 ≥
///    [`HEURISTIC_CAPABLE_CONTEXT_TOKENS`] → [`CostTier::Capable`];
/// 3. 零工具调用 **且** 上下文 ≤ [`HEURISTIC_FRUGAL_CONTEXT_TOKENS`]
///    (两个轻信号都在场) → [`CostTier::Frugal`];
/// 4. 其余 (含信号缺位) → [`CostTier::Standard`] ([`TierSource::Default`])。
pub fn infer_tier(request: &RoutingRequest) -> (CostTier, TierSource) {
    if let Some(tier) = request.explicit_tier {
        return (tier, TierSource::Explicit);
    }
    let premium_signal = request
        .expected_tool_calls
        .is_some_and(|calls| calls >= HEURISTIC_PREMIUM_TOOL_CALLS)
        || request
            .estimated_context_tokens
            .is_some_and(|tokens| tokens >= HEURISTIC_PREMIUM_CONTEXT_TOKENS);
    if premium_signal {
        return (CostTier::Premium, TierSource::Inferred);
    }
    let capable_signal = request
        .expected_tool_calls
        .is_some_and(|calls| calls >= HEURISTIC_CAPABLE_TOOL_CALLS)
        || request
            .estimated_context_tokens
            .is_some_and(|tokens| tokens >= HEURISTIC_CAPABLE_CONTEXT_TOKENS);
    if capable_signal {
        return (CostTier::Capable, TierSource::Inferred);
    }
    let frugal_signal = request.expected_tool_calls == Some(0)
        && request
            .estimated_context_tokens
            .is_some_and(|tokens| tokens <= HEURISTIC_FRUGAL_CONTEXT_TOKENS);
    if frugal_signal {
        return (CostTier::Frugal, TierSource::Inferred);
    }
    (CostTier::Standard, TierSource::Default)
}

/// 比较单价 (合计口径): `input + output`, **两列齐才有值**。
///
/// 缺任一半 = `None` = 不参与价格比较 —— 成本算不出就不猜 (与画像表冻结的
/// 换算口径同源: 缺一半回 `None`, **绝不编价**); 缓存读列不入比较 (同
/// `CostRateTable` 的形状口径)。
pub fn comparison_price_micros(profile: &ModelProfile) -> Option<u64> {
    match (profile.input_price_micros, profile.output_price_micros) {
        (Some(input), Some(output)) => Some(input.saturating_add(output)),
        _ => None,
    }
}

/// 一次路由决策 (可解释: 档位来源 / 候选全集 / 选定理由 / 回退真值标注)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingDecision {
    /// 定下的成本档。
    pub tier: CostTier,
    /// 档位来源 (显式/推断/默认)。
    pub tier_source: TierSource,
    /// 选定模型 id; `None` = 无候选且无默认模型在力 (如实选不出)。
    pub selected_model: Option<String>,
    /// 选定模型的画像行 (未知模型 = 显式 unknown 行, 不编造)。
    pub selected_profile: Option<ModelProfile>,
    /// 候选全集 (画像表顺序; 无价/缺价行按能力在场, 价格字段照实缺省)。
    pub candidates: Vec<ModelProfile>,
    /// 回退标注真值: 是否因**无候选**而回退默认模型。
    pub used_fallback: bool,
    /// 回退原因 (无候选时给出过滤条件真值; 非回退 = `None`)。
    pub fallback_reason: Option<String>,
    /// 选定理由 (为什么是这个模型, 可解释)。
    pub selection_reason: String,
}

/// 成本分级路由真值 (自述路由节的唯一来源)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CostRoutingStatus {
    /// 主开关真值 (fail-closed: 只有 `APEIRETH_COST_ROUTING=1` 才开)。
    pub enabled: bool,
    /// 最近一次路由决策; `None` = 本路由器尚无任务被路由 (照实, 不编造)。
    pub decision: Option<RoutingDecision>,
    /// 无决策的如实原因 (关闸/未路由); 有决策时 `None`。
    pub reason: Option<String>,
}

/// 档位映射配置 (每档首选模型覆盖) + 主开关。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CostRoutingConfig {
    /// 主开关 (`APEIRETH_COST_ROUTING=1` 才开)。
    pub enabled: bool,
    /// 每档首选模型覆盖 (缺档 = 无覆盖)。
    pub preferred_models: BTreeMap<CostTier, String>,
}

impl CostRoutingConfig {
    /// 关闸 + 无覆盖。
    pub fn disabled() -> Self {
        Self::default()
    }

    /// 主开关取值口径 (fail-closed): 只有精确 `1` (去首尾空白) 开。
    pub fn switch_enabled(raw: Option<&str>) -> bool {
        matches!(raw.map(str::trim), Some("1"))
    }

    /// 从两份 env 原始值装配 (纯函数): 开关原始值 + 档位映射 JSON。
    /// 任一层坏 = 整体拒开 (fail-closed, 值自由报错)。
    pub fn from_env_values(
        enabled_raw: Option<&str>,
        preferred_raw: Option<&str>,
    ) -> Result<Self, CostRoutingError> {
        let preferred_models = match preferred_raw.map(str::trim).filter(|raw| !raw.is_empty()) {
            Some(raw) => Self::preferred_from_env_value(raw)?,
            None => BTreeMap::new(),
        };
        Ok(Self {
            enabled: Self::switch_enabled(enabled_raw),
            preferred_models,
        })
    }

    /// 解析档位映射 env 值 (JSON 对象 `{"<tier>": "<model id>", ...}`)。
    ///
    /// fail-closed: 坏 JSON / 非对象 / 未知档位词 / 值非字符串 / 空 model id /
    /// 同档重复（大小写不敏感）全部拒收。报错**值自由**: 只报键名与期望形状,
    /// 从不回显值。
    pub fn preferred_from_env_value(
        raw: &str,
    ) -> Result<BTreeMap<CostTier, String>, CostRoutingError> {
        let value: Value =
            serde_json::from_str(raw).map_err(|error| CostRoutingError::InvalidShape {
                reason: format!(
                    "{COST_ROUTING_PREFERRED_ENV}: JSON parse failed at line {}",
                    error.line()
                ),
            })?;
        let map = value
            .as_object()
            .ok_or_else(|| CostRoutingError::InvalidShape {
                reason: format!(
                "the preferred-model mapping must be a JSON object of tier -> model id, found {}",
                value_kind(&value)
            ),
            })?;
        let mut preferred = BTreeMap::new();
        for (key, entry) in map {
            let tier =
                CostTier::parse(key).ok_or_else(|| CostRoutingError::InvalidTierMapping {
                    tier: key.clone(),
                    reason: "unknown tier name (accepted: frugal, standard, capable, premium)"
                        .to_string(),
                })?;
            let model_id = match entry {
                Value::String(text) => text.clone(),
                other => {
                    return Err(CostRoutingError::InvalidTierMapping {
                        tier: key.clone(),
                        reason: format!(
                            "value must be a model id string, found {}",
                            value_kind(other)
                        ),
                    })
                }
            };
            if model_id.trim().is_empty() {
                return Err(CostRoutingError::InvalidTierMapping {
                    tier: key.clone(),
                    reason: "model id must not be empty".to_string(),
                });
            }
            if preferred.contains_key(&tier) {
                return Err(CostRoutingError::InvalidTierMapping {
                    tier: key.clone(),
                    reason: "tier is configured twice (matching is ASCII case-insensitive)"
                        .to_string(),
                });
            }
            preferred.insert(tier, model_id);
        }
        Ok(preferred)
    }
}

/// 成本分级路由配置面的缺陷。所有错误**值自由**: 只报键名/期望形状/行号,
/// 从不回显被解析的值 (误粘贴的密钥绝不进错误帧)。
#[derive(Debug)]
pub enum CostRoutingError {
    /// JSON 形状或字段不符 (值自由报错)。
    InvalidShape {
        /// 只含键名与期望形状的说明。
        reason: String,
    },
    /// 档位映射的条目非法 (只报键名与语法要求)。
    InvalidTierMapping {
        /// 出错的档位键名 (键名是配置面, 不是值)。
        tier: String,
        /// 语法要求说明 (不含值)。
        reason: String,
    },
}

impl fmt::Display for CostRoutingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidShape { reason } => {
                write!(f, "cost routing configuration is malformed: {reason}")
            }
            Self::InvalidTierMapping { tier, reason } => {
                write!(
                    f,
                    "cost routing preferred-model entry {tier:?} is invalid: {reason}"
                )
            }
        }
    }
}

impl std::error::Error for CostRoutingError {}

/// 成本分级路由器: 画像表 (筛选来源) + 默认模型 (回退目标) + 配置 + 最近
/// 决策真值 (自述路由节来源)。
#[derive(Debug)]
pub struct CostRouter {
    profiles: ModelProfileTable,
    default_model: Option<String>,
    config: CostRoutingConfig,
    last_decision: Mutex<Option<RoutingDecision>>,
}

impl CostRouter {
    /// 一台路由器。
    ///
    /// `default_model` = 无候选时的回退目标 (生效模型 id 口径), `None` = 没有
    /// 默认模型在力 (回退也照实选不出, 不硬凑)。
    pub fn new(
        profiles: ModelProfileTable,
        default_model: Option<String>,
        config: CostRoutingConfig,
    ) -> Self {
        Self {
            profiles,
            default_model,
            config,
            last_decision: Mutex::new(None),
        }
    }

    /// 主开关真值 (fail-closed)。
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// 回退默认模型 (生效模型 id 口径)。
    pub fn default_model(&self) -> Option<&str> {
        self.default_model.as_deref()
    }

    /// 画像表 (筛选来源)。
    pub fn profiles(&self) -> &ModelProfileTable {
        &self.profiles
    }

    /// 路由一个任务: 档位 → 候选 → 选定 (含无候选回退)。
    ///
    /// 开关关 = `None` (**fail-closed**: 不路由, 不产生任何档位/选定); 开 =
    /// `Some(决策)`, 并把决策记为最近决策 ([`CostRouter::status`] 的真值)。
    pub fn route(&self, request: &RoutingRequest) -> Option<RoutingDecision> {
        if !self.config.enabled {
            return None;
        }
        let decision = self.decide(request);
        let mut last = self
            .last_decision
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *last = Some(decision.clone());
        Some(decision)
    }

    /// 最近一次路由决策; 本路由器尚无任务被路由 = `None` (照实)。
    pub fn last_decision(&self) -> Option<RoutingDecision> {
        let last = self
            .last_decision
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        last.clone()
    }

    /// 自述路由节真值: 开关真值 + 最近决策 (或缺席的如实原因)。
    pub fn status(&self) -> CostRoutingStatus {
        if !self.config.enabled {
            return CostRoutingStatus {
                enabled: false,
                decision: None,
                reason: Some(format!(
                    "cost-tier routing switch is off ({COST_ROUTING_ENV}=1 enables it); no tier is in force"
                )),
            };
        }
        match self.last_decision() {
            Some(decision) => CostRoutingStatus {
                enabled: true,
                decision: Some(decision),
                reason: None,
            },
            None => CostRoutingStatus {
                enabled: true,
                decision: None,
                reason: Some(
                    "cost-tier routing is enabled but no task has been routed yet".to_string(),
                ),
            },
        }
    }

    /// 档位过滤后的候选全集 (画像表顺序)。
    pub fn candidates(&self, tier: CostTier, request: &RoutingRequest) -> Vec<ModelProfile> {
        let (needs_tools, min_window) = Self::effective_requirements(tier, request);
        self.profiles
            .profiles()
            .iter()
            .filter(|profile| Self::passes_filters(profile, tier, needs_tools, min_window))
            .cloned()
            .collect()
    }

    /// 档位条件 + 任务需求的合成要求: 工具需求取 OR, 窗口下限取较大者。
    fn effective_requirements(tier: CostTier, request: &RoutingRequest) -> (bool, u64) {
        let spec = tier.spec();
        let needs_tools = spec.requires_tools
            || request.requires_tools
            || request.expected_tool_calls.is_some_and(|calls| calls > 0);
        let min_window = spec
            .min_context_window
            .max(request.estimated_context_tokens.unwrap_or(0));
        (needs_tools, min_window)
    }

    /// 画像行过不过档位过滤 (支持工具 / 窗口够 / 单价 ≤ 档上限)。
    ///
    /// 工具/窗口: 未知不算合格 (未知不等于支持/够); 单价: 无价/缺价行**不
    /// 参与价格比较** (按能力入候选, 绝不编价)。
    fn passes_filters(
        profile: &ModelProfile,
        tier: CostTier,
        needs_tools: bool,
        min_window: u64,
    ) -> bool {
        let tools_ok = !needs_tools || profile.supports_tools == Some(true);
        let window_ok = profile
            .context_window
            .is_some_and(|window| window >= min_window);
        let price_ok = comparison_price_micros(profile)
            .is_none_or(|price| price <= tier.spec().price_ceiling_micros);
        tools_ok && window_ok && price_ok
    }

    /// 决策核心 (纯逻辑; 记录由 [`CostRouter::route`] 负责)。
    fn decide(&self, request: &RoutingRequest) -> RoutingDecision {
        let (tier, tier_source) = infer_tier(request);
        let candidates = self.candidates(tier, request);
        let (needs_tools, min_window) = Self::effective_requirements(tier, request);
        let ceiling = tier.spec().price_ceiling_micros;

        if candidates.is_empty() {
            return self.fallback_decision(tier, tier_source, needs_tools, min_window, ceiling);
        }

        // ① 档位映射配置的首选模型 (在候选内才生效)。
        let mut preferred_note = String::new();
        if let Some(preferred_id) = self.config.preferred_models.get(&tier) {
            let key = normalize_model_id(preferred_id);
            let hit = candidates
                .iter()
                .find(|profile| normalize_model_id(&profile.model_id) == key)
                .cloned();
            if let Some(profile) = hit {
                return Self::selected_decision(
                    tier,
                    tier_source,
                    candidates,
                    profile,
                    format!(
                        "tier-preferred model for {tier} (configured override) passes the {tier} filter"
                    ),
                );
            }
            preferred_note = format!(
                "; tier-preferred model {preferred_id} is not a {tier} candidate (filter not satisfied), selection continued by rule"
            );
        }

        // ② 有价候选里比较单价最低者 (合计口径; 平手取画像表顺序)。
        let cheapest = candidates
            .iter()
            .filter(|profile| comparison_price_micros(profile).is_some())
            .min_by_key(|profile| comparison_price_micros(profile))
            .cloned();
        if let Some(cheapest) = cheapest {
            let price = comparison_price_micros(&cheapest).unwrap_or_default();
            return Self::selected_decision(
                tier,
                tier_source,
                candidates,
                cheapest,
                format!(
                    "cheapest priced candidate at {price} micros per 1k tokens (input+output, equal-weight comparison); \
                     the price comparison covers priced candidates only{preferred_note}"
                ),
            );
        }

        // ③ 默认模型在候选内 = 保持默认 (无价格证据不换模型)。
        let default_hit = self.default_model.as_ref().and_then(|default| {
            let key = normalize_model_id(default);
            candidates
                .iter()
                .find(|profile| normalize_model_id(&profile.model_id) == key)
                .cloned()
        });
        if let Some(profile) = default_hit {
            return Self::selected_decision(
                tier,
                tier_source,
                candidates,
                profile,
                format!(
                    "no comparable unit price among candidates (prices not provided); keeping the default model \
                     since it is a capability-qualified {tier} candidate{preferred_note}"
                ),
            );
        }

        // ④ 画像表顺序第一个候选 (能力合格, 价格照实未提供)。
        let first = candidates
            .first()
            .cloned()
            .expect("non-empty candidate list");
        Self::selected_decision(
            tier,
            tier_source,
            candidates,
            first,
            format!(
                "no comparable unit price among candidates (prices not provided); first capability-qualified \
                 candidate in profile-table order{preferred_note}"
            ),
        )
    }

    fn selected_decision(
        tier: CostTier,
        tier_source: TierSource,
        candidates: Vec<ModelProfile>,
        selected: ModelProfile,
        selection_reason: String,
    ) -> RoutingDecision {
        RoutingDecision {
            tier,
            tier_source,
            selected_model: Some(selected.model_id.clone()),
            selected_profile: Some(selected),
            candidates,
            used_fallback: false,
            fallback_reason: None,
            selection_reason,
        }
    }

    /// 无候选 = 回退默认模型 + 如实标注 (不硬凑, 不粉饰)。
    fn fallback_decision(
        &self,
        tier: CostTier,
        tier_source: TierSource,
        needs_tools: bool,
        min_window: u64,
        ceiling: u64,
    ) -> RoutingDecision {
        let tools_clause = if needs_tools {
            "tool support confirmed"
        } else {
            "tool support not required"
        };
        let mut fallback_reason = format!(
            "no model row in the profile table passes the {tier} filter \
             ({tools_clause}, context window >= {min_window} tokens, comparison unit price <= {ceiling} micros per 1k tokens)"
        );
        let default_profile = self.default_model.as_ref().map(|model| {
            let profile = self.profiles.resolve(model).into_owned();
            if !Self::passes_filters(&profile, tier, needs_tools, min_window) {
                fallback_reason.push_str(&format!(
                    "; default model {model} does not pass the filter either — kept only as the explicit fallback"
                ));
            }
            profile
        });
        let selection_reason = match &self.default_model {
            Some(model) => format!(
                "fallback: no candidate passed the {tier} filter; default model {model} kept as the explicit fallback"
            ),
            None => {
                fallback_reason.push_str("; no default model is in force either — nothing is selected");
                format!(
                    "fallback: no candidate passed the {tier} filter and no default model is in force — nothing is selected"
                )
            }
        };
        RoutingDecision {
            tier,
            tier_source,
            selected_model: self.default_model.clone(),
            selected_profile: default_profile,
            candidates: Vec::new(),
            used_fallback: true,
            fallback_reason: Some(fallback_reason),
            selection_reason,
        }
    }
}

/// JSON 值的类型名 (报错用; 只报类型, 绝不报值)。
fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(
        model_id: &str,
        supports_tools: Option<bool>,
        context_window: Option<u64>,
        input_price_micros: Option<u64>,
        output_price_micros: Option<u64>,
    ) -> ModelProfile {
        ModelProfile {
            model_id: model_id.to_string(),
            context_window,
            max_output: Some(8_192),
            supports_tools,
            supports_streaming: Some(true),
            input_price_micros,
            output_price_micros,
            cache_read_price_micros: None,
            capabilities_flags: Vec::new(),
        }
    }

    fn matrix_table() -> ModelProfileTable {
        ModelProfileTable::new(vec![
            profile(
                "chore-model",
                Some(false),
                Some(32_768),
                Some(1_000),
                Some(1_000),
            ),
            profile(
                "work-model",
                Some(true),
                Some(32_768),
                Some(3_000),
                Some(3_000),
            ),
            profile(
                "deep-model",
                Some(true),
                Some(128_000),
                Some(10_000),
                Some(10_000),
            ),
            profile(
                "huge-model",
                Some(true),
                Some(256_000),
                Some(50_000),
                Some(50_000),
            ),
            profile("unpriced-model", Some(true), Some(256_000), None, None),
        ])
    }

    fn candidate_ids(decision: &RoutingDecision) -> Vec<&str> {
        decision
            .candidates
            .iter()
            .map(|profile| profile.model_id.as_str())
            .collect()
    }

    fn make_router(table: ModelProfileTable, default_model: Option<&str>) -> CostRouter {
        CostRouter::new(
            table,
            default_model.map(str::to_string),
            CostRoutingConfig {
                enabled: true,
                preferred_models: BTreeMap::new(),
            },
        )
    }

    #[test]
    fn the_cost_tier_vocabulary_is_a_closed_four_step_ladder() {
        let spellings: Vec<&str> = CostTier::ALL.iter().map(|tier| tier.as_str()).collect();
        assert_eq!(
            spellings,
            vec!["frugal", "standard", "capable", "premium"],
            "分级词表: 四档闭合, 拼写冻结"
        );
        for tier in CostTier::ALL {
            assert_eq!(CostTier::parse(tier.as_str()), Some(tier), "{tier}");
            assert_eq!(
                CostTier::parse(&tier.as_str().to_ascii_uppercase()),
                Some(tier),
                "词表解析大小写不敏感"
            );
        }
        for unknown in ["cheap", "luxury", "", "standard-premium"] {
            assert_eq!(CostTier::parse(unknown), None, "词表闭合, 未知词不猜档");
        }
    }

    #[test]
    fn each_tier_is_capability_requirement_times_price_ceiling() {
        // 分级词表数值钉死: 能力需求 × 窗口下限 × 单价上限 (策略常量)。
        let frugal = CostTier::Frugal.spec();
        assert!(!frugal.requires_tools, "轻档不强制工具");
        assert_eq!(frugal.min_context_window, FRUGAL_MIN_CONTEXT_WINDOW);
        assert_eq!(frugal.price_ceiling_micros, FRUGAL_PRICE_CEILING_MICROS);
        let standard = CostTier::Standard.spec();
        assert!(standard.requires_tools);
        assert_eq!(standard.min_context_window, STANDARD_MIN_CONTEXT_WINDOW);
        assert_eq!(standard.price_ceiling_micros, STANDARD_PRICE_CEILING_MICROS);
        let capable = CostTier::Capable.spec();
        assert!(capable.requires_tools);
        assert_eq!(capable.min_context_window, CAPABLE_MIN_CONTEXT_WINDOW);
        assert_eq!(capable.price_ceiling_micros, CAPABLE_PRICE_CEILING_MICROS);
        let premium = CostTier::Premium.spec();
        assert!(premium.requires_tools);
        assert_eq!(premium.min_context_window, PREMIUM_MIN_CONTEXT_WINDOW);
        assert_eq!(premium.price_ceiling_micros, PREMIUM_PRICE_CEILING_MICROS);
        // 档位阶梯: 预算与窗口随档位单调不降。
        for pair in CostTier::ALL.windows(2) {
            let (lower, higher) = (pair[0].spec(), pair[1].spec());
            assert!(lower.price_ceiling_micros < higher.price_ceiling_micros);
            assert!(lower.min_context_window <= higher.min_context_window);
        }
    }

    #[test]
    fn the_four_tier_filter_matrix_selects_by_capability_window_and_price() {
        let router = make_router(matrix_table(), Some("work-model"));
        let request = RoutingRequest::new().with_requires_tools(false);

        let frugal = router
            .route(&request.with_explicit_tier(CostTier::Frugal))
            .expect("routing is on");
        assert_eq!(
            candidate_ids(&frugal),
            vec!["chore-model", "unpriced-model"],
            "轻档: 不强制工具, 窗口 ≥ 8_192, 比较单价 ≤ 5_000 或无价"
        );

        let standard = router
            .route(&request.with_explicit_tier(CostTier::Standard))
            .expect("routing is on");
        assert_eq!(
            candidate_ids(&standard),
            vec!["work-model", "deep-model", "unpriced-model"],
            "标准档: 需工具, 窗口 ≥ 32_768, 比较单价 ≤ 20_000 或无价"
        );

        let capable = router
            .route(&request.with_explicit_tier(CostTier::Capable))
            .expect("routing is on");
        assert_eq!(
            candidate_ids(&capable),
            vec!["deep-model", "unpriced-model"],
            "重活档: 需工具, 窗口 ≥ 65_536, 比较单价 ≤ 80_000 或无价"
        );

        let premium = router
            .route(&request.with_explicit_tier(CostTier::Premium))
            .expect("routing is on");
        assert_eq!(
            candidate_ids(&premium),
            vec!["huge-model", "unpriced-model"],
            "深思档: 需工具, 窗口 ≥ 131_072, 比较单价 ≤ 400_000 或无价"
        );
    }

    #[test]
    fn tier_inference_falls_from_heavy_signals_to_the_default_standard() {
        // 重信号单证即升档。
        let (tier, source) = infer_tier(&RoutingRequest::new().with_expected_tool_calls(32));
        assert_eq!((tier, source), (CostTier::Premium, TierSource::Inferred));
        let (tier, source) =
            infer_tier(&RoutingRequest::new().with_estimated_context_tokens(200_000));
        assert_eq!((tier, source), (CostTier::Premium, TierSource::Inferred));
        let (tier, source) = infer_tier(&RoutingRequest::new().with_expected_tool_calls(8));
        assert_eq!((tier, source), (CostTier::Capable, TierSource::Inferred));
        let (tier, source) =
            infer_tier(&RoutingRequest::new().with_estimated_context_tokens(64_000));
        assert_eq!((tier, source), (CostTier::Capable, TierSource::Inferred));
        // 轻档需双证。
        let (tier, source) = infer_tier(
            &RoutingRequest::new()
                .with_expected_tool_calls(0)
                .with_estimated_context_tokens(8_192),
        );
        assert_eq!((tier, source), (CostTier::Frugal, TierSource::Inferred));
        let (tier, source) = infer_tier(&RoutingRequest::new().with_expected_tool_calls(0));
        assert_eq!(
            (tier, source),
            (CostTier::Standard, TierSource::Default),
            "缺上下文信号不猜轻档"
        );
        let (tier, source) = infer_tier(&RoutingRequest::new().with_estimated_context_tokens(500));
        assert_eq!(
            (tier, source),
            (CostTier::Standard, TierSource::Default),
            "缺工具信号不猜轻档"
        );
        // 全缺位 = 默认 standard。
        let (tier, source) = infer_tier(&RoutingRequest::new());
        assert_eq!((tier, source), (CostTier::Standard, TierSource::Default));
        // 零工具但大上下文 = 重信号升档 (能力风险优先于省钱)。
        let (tier, source) = infer_tier(
            &RoutingRequest::new()
                .with_expected_tool_calls(0)
                .with_estimated_context_tokens(64_000),
        );
        assert_eq!((tier, source), (CostTier::Capable, TierSource::Inferred));
    }

    #[test]
    fn an_explicit_tier_overrides_the_heuristics() {
        let request = RoutingRequest::new()
            .with_explicit_tier(CostTier::Frugal)
            .with_expected_tool_calls(64)
            .with_estimated_context_tokens(400_000);
        let (tier, source) = infer_tier(&request);
        assert_eq!(
            (tier, source),
            (CostTier::Frugal, TierSource::Explicit),
            "显式档最高优先, 压过重信号"
        );
        let router = make_router(matrix_table(), Some("work-model"));
        let decision = router.route(&request).expect("routing is on");
        assert_eq!(decision.tier, CostTier::Frugal);
        assert_eq!(decision.tier_source, TierSource::Explicit);
    }

    #[test]
    fn no_candidate_falls_back_to_the_default_model_with_an_honest_annotation() {
        // deep-model 是唯一有价行且窗口够 standard, 但把窗口砍到 32k 后
        // capable 档零候选 → 回退默认模型 + 如实标注。
        let table = ModelProfileTable::new(vec![
            profile(
                "work-model",
                Some(true),
                Some(32_768),
                Some(3_000),
                Some(3_000),
            ),
            profile(
                "deep-model",
                Some(true),
                Some(65_536),
                Some(70_000),
                Some(70_000),
            ),
        ]);
        let router = make_router(table, Some("work-model"));
        let decision = router
            .route(&RoutingRequest::new().with_explicit_tier(CostTier::Premium))
            .expect("routing is on");
        assert!(decision.used_fallback, "无候选必须回退并标注");
        assert_eq!(
            decision.selected_model.as_deref(),
            Some("work-model"),
            "回退目标 = 默认模型"
        );
        let reason = decision.fallback_reason.as_deref().expect("回退原因真值");
        assert!(reason.contains("premium"), "{reason}");
        assert!(reason.contains("131072"), "如实报过滤条件: {reason}");
        assert!(reason.contains("400000"), "如实报档上限: {reason}");
        assert!(
            reason.contains("work-model does not pass the filter either"),
            "默认模型不过滤也如实标注: {reason}"
        );
        assert!(decision.candidates.is_empty(), "候选真值 = 空");
        assert_eq!(
            decision.selected_profile.as_ref().map(|p| &p.model_id),
            Some(&"work-model".to_string())
        );

        // 默认模型缺席: 如实选不出, 不硬凑。
        let router = make_router(
            ModelProfileTable::new(vec![profile("tiny", Some(false), Some(1_024), None, None)]),
            None,
        );
        let decision = router
            .route(&RoutingRequest::new().with_explicit_tier(CostTier::Standard))
            .expect("routing is on");
        assert!(decision.used_fallback);
        assert_eq!(decision.selected_model, None, "无默认模型 = 如实选不出");
        assert_eq!(decision.selected_profile, None);
        let reason = decision.fallback_reason.as_deref().expect("回退原因真值");
        assert!(reason.contains("no default model is in force"), "{reason}");
    }

    #[test]
    fn unpriced_models_join_candidates_by_capability_and_are_never_priced() {
        // 无价行: 按能力入候选, 不参与价格比较, 价格字段照实 None (绝不编价)。
        let unpriced = profile("unpriced-model", Some(true), Some(65_536), None, None);
        assert_eq!(
            comparison_price_micros(&unpriced),
            None,
            "无价 = 不参与价格比较"
        );
        // 缺一半也不补另一半: 同样算不出 = 不参与价格比较。
        let half_priced = profile("half-priced", Some(true), Some(65_536), Some(1_000), None);
        assert_eq!(
            comparison_price_micros(&half_priced),
            None,
            "缺一半不编另一半"
        );
        let table = ModelProfileTable::new(vec![
            unpriced,
            half_priced,
            profile(
                "priced-over-ceiling",
                Some(true),
                Some(65_536),
                Some(70_000),
                Some(70_000),
            ),
        ]);
        let router = make_router(table, Some("unpriced-model"));
        let decision = router
            .route(&RoutingRequest::new().with_explicit_tier(CostTier::Capable))
            .expect("routing is on");
        assert_eq!(
            candidate_ids(&decision),
            vec!["unpriced-model", "half-priced"],
            "无价/缺价行按能力入候选; 有价超上限行出局"
        );
        // 绝不编价: 候选行的价格字段照实保留, 缺的一律不补。
        for candidate in &decision.candidates {
            match candidate.model_id.as_str() {
                "unpriced-model" => assert_eq!(
                    (candidate.input_price_micros, candidate.output_price_micros),
                    (None, None),
                    "无价行不编价"
                ),
                "half-priced" => assert_eq!(
                    (candidate.input_price_micros, candidate.output_price_micros),
                    (Some(1_000), None),
                    "缺一半不补另一半"
                ),
                other => panic!("unexpected candidate {other}"),
            }
        }
        // 无价格证据时保持默认模型 (稳定性), 理由如实。
        assert_eq!(decision.selected_model.as_deref(), Some("unpriced-model"));
        assert!(
            decision.selection_reason.contains("not provided"),
            "{}",
            decision.selection_reason
        );
    }

    #[test]
    fn the_cheapest_priced_candidate_wins_the_comparison() {
        let table = ModelProfileTable::new(vec![
            profile(
                "expensive",
                Some(true),
                Some(65_536),
                Some(6_000),
                Some(6_000),
            ),
            profile(
                "cheapest",
                Some(true),
                Some(65_536),
                Some(1_000),
                Some(1_000),
            ),
            profile("middle", Some(true), Some(65_536), Some(3_000), Some(3_000)),
            profile("unpriced", Some(true), Some(65_536), None, None),
        ]);
        let router = make_router(table, Some("expensive"));
        let decision = router
            .route(&RoutingRequest::new().with_explicit_tier(CostTier::Capable))
            .expect("routing is on");
        assert_eq!(
            decision.selected_model.as_deref(),
            Some("cheapest"),
            "有价格证据就按省钱选"
        );
        assert!(
            decision.selection_reason.contains("priced candidates only"),
            "{}",
            decision.selection_reason
        );
        assert!(!decision.used_fallback);
    }

    #[test]
    fn the_tier_preferred_override_wins_when_it_qualifies_and_is_annotated_when_it_does_not() {
        let table = ModelProfileTable::new(vec![
            profile(
                "cheapest",
                Some(true),
                Some(65_536),
                Some(1_000),
                Some(1_000),
            ),
            profile(
                "preferred-model",
                Some(true),
                Some(65_536),
                Some(4_000),
                Some(4_000),
            ),
            profile(
                "too-expensive",
                Some(true),
                Some(65_536),
                Some(90_000),
                Some(90_000),
            ),
        ]);
        let mut preferred = BTreeMap::new();
        preferred.insert(CostTier::Capable, "PREFERRED-MODEL".to_string());
        let router = CostRouter::new(
            table,
            Some("cheapest".to_string()),
            CostRoutingConfig {
                enabled: true,
                preferred_models: preferred,
            },
        );
        let decision = router
            .route(&RoutingRequest::new().with_explicit_tier(CostTier::Capable))
            .expect("routing is on");
        assert_eq!(
            decision.selected_model.as_deref(),
            Some("preferred-model"),
            "配置覆盖胜出 (model id 匹配大小写不敏感)"
        );
        assert!(
            decision.selection_reason.contains("configured override"),
            "{}",
            decision.selection_reason
        );

        // 首选模型不过档位过滤: 如实注明后按规则顺延, 不硬选。
        let mut preferred = BTreeMap::new();
        preferred.insert(CostTier::Capable, "too-expensive".to_string());
        let router = CostRouter::new(
            router.profiles().clone(),
            Some("cheapest".to_string()),
            CostRoutingConfig {
                enabled: true,
                preferred_models: preferred,
            },
        );
        let decision = router
            .route(&RoutingRequest::new().with_explicit_tier(CostTier::Capable))
            .expect("routing is on");
        assert_eq!(decision.selected_model.as_deref(), Some("cheapest"));
        assert!(
            decision.selection_reason.contains("too-expensive")
                && decision
                    .selection_reason
                    .contains("not a capable candidate"),
            "首选落选要如实标注: {}",
            decision.selection_reason
        );
    }

    #[test]
    fn the_cost_routing_switch_fails_closed() {
        for off in [
            None,
            Some(""),
            Some("0"),
            Some("2"),
            Some("true"),
            Some("yes"),
            Some("on"),
            Some("  1  0"),
        ] {
            assert!(
                !CostRoutingConfig::switch_enabled(off.as_deref()),
                "fail-closed: 只有精确 1 开, {off:?} 必须关"
            );
        }
        assert!(CostRoutingConfig::switch_enabled(Some("1")));
        assert!(CostRoutingConfig::switch_enabled(Some(" 1 ")));

        // 开关关 = route 不路由 (None), 自述照实报关闸。
        let router = CostRouter::new(
            matrix_table(),
            Some("work-model".to_string()),
            CostRoutingConfig::disabled(),
        );
        assert!(!router.is_enabled());
        assert_eq!(
            router.route(&RoutingRequest::new()),
            None,
            "关闸 fail-closed: 不路由"
        );
        let status = router.status();
        assert!(!status.enabled);
        assert_eq!(status.decision, None);
        assert!(
            status
                .reason
                .as_deref()
                .is_some_and(|reason| reason.contains(COST_ROUTING_ENV)),
            "{:?}",
            status.reason
        );
    }

    #[test]
    fn the_preferred_tier_mapping_fails_closed_on_any_defect() {
        for raw in [
            "not json",
            "\"a string\"",
            "[]",
            r#"{"luxury": "model-x"}"#,
            r#"{"frugal": 7}"#,
            r#"{"frugal": ""}"#,
            r#"{"frugal": "   "}"#,
            r#"{"frugal": {"model": "x"}}"#,
            r#"{"frugal": "model-a", "Frugal": "model-b"}"#,
        ] {
            assert!(
                CostRoutingConfig::preferred_from_env_value(raw).is_err(),
                "must fail closed: {raw}"
            );
        }
        // model id 是接口事实, 形状合法的值照常收; 值自由报错只发生在拒收时。
        let parsed = CostRoutingConfig::preferred_from_env_value(r#"{"frugal": "model-x"}"#);
        assert!(parsed.is_ok(), "model id 是接口事实, 正常收");
        let error =
            CostRoutingConfig::preferred_from_env_value(r#"{"frugal": ["sk-live-never-echo-me"]}"#)
                .expect_err("类型不符拒开");
        let message = error.to_string();
        assert!(
            !message.contains("never-echo-me"),
            "值绝不能进错误帧: {message}"
        );
        let error = CostRoutingConfig::preferred_from_env_value(r#"{"frugal": 1234567890}"#)
            .expect_err("类型不符拒开");
        let message = error.to_string();
        assert!(
            !message.contains("1234567890"),
            "值绝不能进错误帧: {message}"
        );

        // 正常形状: 四档全覆盖 + 缺档 = 无覆盖; 空白 env = 无覆盖。
        let parsed = CostRoutingConfig::preferred_from_env_value(
            r#"{"frugal": "model-a", "Premium": "model-b"}"#,
        )
        .expect("parse");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[&CostTier::Frugal], "model-a");
        assert_eq!(parsed[&CostTier::Premium], "model-b");
        let config =
            CostRoutingConfig::from_env_values(Some("1"), Some("   ")).expect("blank mapping");
        assert!(config.enabled);
        assert!(config.preferred_models.is_empty());
        let config = CostRoutingConfig::from_env_values(Some("1"), None).expect("no mapping");
        assert!(config.enabled);
    }

    #[test]
    fn routing_status_reports_the_latest_decision_and_its_absence_honestly() {
        let router = make_router(matrix_table(), Some("work-model"));
        let before = router.status();
        assert!(before.enabled, "开关真值");
        assert_eq!(before.decision, None, "尚无任务被路由 = 如实 None");
        assert!(
            before
                .reason
                .as_deref()
                .is_some_and(|reason| reason.contains("no task has been routed")),
            "{:?}",
            before.reason
        );
        router.route(&RoutingRequest::new()).expect("routing is on");
        let after = router.status();
        assert_eq!(after.reason, None);
        let decision = after.decision.expect("最近决策真值");
        assert_eq!(decision.tier, CostTier::Standard);
        assert_eq!(decision.tier_source, TierSource::Default);
        assert_eq!(decision.selected_model.as_deref(), Some("work-model"));
        assert!(!decision.used_fallback);
    }
}
