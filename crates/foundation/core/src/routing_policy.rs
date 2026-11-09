//! 统一路由策略层 (`routing_policy`) —— 能力创造线 C-H1: 任务画像 × 隐私 × 成本
//! ⇒ **可解释**的选路决策 (选谁 / 为什么 / 退路是谁)。
//!
//! # 本层是什么: **统一策略层, 不造第三套路由语义**
//!
//! 仓内已有两层路由语义, 本层是它们**上面**的一张统一决策表, **不替代、不重写**:
//!
//! | 层 | 位置 | 回答的问题 |
//! |---|---|---|
//! | 成本档层 ([`crate::cost_router`]) | `foundation/core/src/cost_router.rs` | 任务属**哪一档**? 哪些模型过得了档位过滤 (能力 × 窗口 × 单价上限)? 选定阶梯? |
//! | 提供方层 (`ProviderRouter`) | `engine/runtime/src/canonical/provider.rs` | 这次调用**先打谁**? 失败了**退到谁**? (健康度 + fallback, 只读不改) |
//! | **本层 (策略层)** | `foundation/core/src/routing_policy.rs` | 任务画像 (能力 × 隐私 × 成本) ⇒ **选谁 / 为什么 / 退路是谁**? |
//!
//! **复用点 (本层调它, 不重写它)**:
//!
//! - 档位判定: [`infer_tier`] (成本档层的启发式, 显式档 > 推断 > 默认);
//! - 档位过滤 (成本矩阵): [`CostRouter::candidates`] (能力需求 × 窗口下限 × 单价上限);
//! - 选定阶梯 (档位首选 > 最低单价 > 默认模型 > 画像表顺序): **整条阶梯由成本档层
//!   在存活集上跑** ([`CostRouter::route`]), 本层只做**隐私闸 + 候选表闸 + 可解释聚合**;
//! - 价格比较口径: [`comparison_price_micros`] (合计口径, **缺价不编价**)。
//!
//! ⇒ 同一档位、同一候选集下, 本层选出的模型与成本档层**同源一致** (见集成测试
//! `the_policy_layer_agrees_with_the_cost_router_on_the_same_tier`)。
//!
//! # 决策表 (画像 × 隐私 × 成本)
//!
//! | 维度 | 取值 | 来源 | 闸位 |
//! |---|---|---|---|
//! | 能力需求 | [`CapabilityRequirement`] (需工具 / 期望工具调用数 / 上下文规模) | 调用方声明 | 硬闸 (工具/窗口不达标 ⇒ 出局) |
//! | 成本档 | [`CostTier`] (`frugal|standard|capable|premium`) | 显式档, 否则按能力信号推断 | 硬闸 (档位过滤 + 单价上限) |
//! | 隐私等级 | [`PrivacyLevel`] (`ordinary|restricted|confidential`) | 调用方声明 | **硬闸** (安全 > 功能 > 性能) |
//!
//! **闸位顺序 = 安全序**: 隐私闸 → 候选表闸 → 成本档闸 → 价格比较。
//! ⇒ 一个「更便宜但隐私不合规」的候选**永远赢不了**一个「更贵但隐私合规」的候选
//! (对抗用例 `privacy_beats_price_in_an_adversarial_conflict` 钉死)。
//!
//! # 决策规则 (顺序即优先级)
//!
//! 1. **开关**: 复用 [`CostRoutingConfig`] 的 fail-closed 口径
//!    (`APEIRETH_COST_ROUTING=1` 才开, **0 新 env 名**); 关 = 直接失败
//!    ([`RoutingPolicyError::Disabled`]), 不产生任何决策。
//! 2. **成本档候选**: [`CostRouter::candidates`] —— 档位矩阵口径**原样复用**。
//!    零候选 ⇒ [`RoutingPolicyError::NoCostCandidate`] (明确失败, 不替成本档层造候选)。
//! 3. **隐私闸**: 候选的 [`PolicyCandidate::max_privacy`] 必须 ≥ 任务的
//!    [`TaskProfile::privacy`]; 不达标 ⇒ **排除并给出排除理由** (绝不静默过滤)。
//! 4. **候选表闸**: 候选须登记在策略候选表; 未登记 ⇒ 排除并说明 (不默认放行)。
//!    ⚠️ **零行为变化的成立条件**: 普通档下隐私闸恒不过滤, 但**候选表闸仍在**
//!    ⇒ 幸存集 = 候选表 ∩ 成本候选集 ⇒ **候选表若是画像表的真子集, 选出的模型可与
//!    成本档层不同 (或直接失败)** —— 这是**候选表闸的策略语义**, 不是语义漂移。
//! 5. **选定**: 存活集交给成本档层跑既定阶梯 ⇒ 成本口径与成本档层**同源**。
//! 6. **退路链**: 存活集按比较单价升序 (有价优先, **缺价候选不参与价格比较**、
//!    排在其后) ⇒ 退路是谁、按什么顺序, 写进 [`RoutingDecision::fallback_chain`]。
//!    ⚠️ **退路排序 ≠ 选定阶梯顺序**: 阶梯是「档位首选 > 最便宜 > 默认模型 >
//!    画像表顺序」, 退路链是**单价升序** ⇒ 退路第一名**未必**是阶梯第二名,
//!    消费方不可把退路链当重试顺序 (见 [`RoutingDecision::fallback_reason`])。
//!
//! # 诚实口径
//!
//! - **缺价不编价**: 价格只有两列齐才有值 ([`comparison_price_micros`]); 缺价/缺半
//!   的候选**不参与价格比较**, 价格字段照实缺省, 比较行标 `price_compared = false`
//!   —— 与成本档层的「绝不编价」同一纪律;
//! - **候选全被排除 ⇒ 明确失败** ([`RoutingPolicyError::NoEligibleCandidate`]),
//!   错误帧带**逐行排除理由**; **不随便挑一个**、**不静默回退默认模型**
//!   (默认模型若也过不了隐私闸, 宁可不选 —— 隐私闸优先于成本档层的默认回退语义,
//!   这是本层**收紧**而非替代成本档层的那一处, 已在 §边界 里写明);
//! - **未知模型 = 显式 unknown 行**: 画像表没有的行照实回 unknown (不编造能力事实);
//! - 逐行比较 ([`RoutingDecision::comparisons`]) 覆盖「策略候选表全部行」+
//!   「过了成本档但未登记的行」, 被排除的行**带理由在场**, 不静默消失。
//!
//! # 与既有两层的边界 (写明, 防后人误读)
//!
//! - 对**成本档层**: 本层是**上层**。成本矩阵、档位阶梯、选定阶梯、单价换算
//!   **全部复用**, 本层不持有第二份成本语义; 本层新增的只有「隐私维度」+
//!   「候选表」+「可解释聚合」。**一处收紧**: 隐私闸否决时本层**不**继承成本档层的
//!   「无候选 ⇒ 回退默认模型」, 改为明确失败 (默认模型可能本身就过不了隐私闸)。
//! - 对**提供方层** (`ProviderRouter`): 本层**只选模型**, 不管健康度、不管
//!   调用失败后的 provider fallback。两者是**先后**关系 (先选模型, 再由提供方层
//!   在 provider 之间退避), 语义不重叠; 本层在 `core` (foundation 层),
//!   **不依赖 engine/runtime**。
//!
//! # 隐私信号的来源 (口径事实 · 2026-11-08 实测)
//!
//! 仓内**没有**任务级的隐私真源, 逐条查证如下 (证据行号见 §证据):
//!
//! - **插件/请求层**: 插件 crate 的请求类型 (provider 请求 = `NormalizedRequest`,
//!   协议层所有) 上**没有**任何敏感度/分类/隐私字段; 唯一可挂载处是请求既有的
//!   `metadata` 透传通道 —— 但该通道**会被转发到厂商线**
//!   (`protocol/src/adapters/openai_chat.rs:188-193` 把全部 metadata 写进出网 body)
//!   ⇒ **隐私声明不放这里** (否则声明本身外泄);
//! - **记忆域** (`engine/memory/src/scope.rs`): [`MemoryScope`] 是**可见性**边界
//!   (谁能看), 不是**敏感度**属性 ⇒ 不作隐私信号;
//! - **CLI 会话/工作区**: 只有 `workspace_root = current_dir()`
//!   (`adapters/cli/src/lib.rs:2144`), 仓内**没有**「家目录 vs 公共目录」的既有
//!   敏感度口径 ⇒ 推导它等于**新造语义** (凭空造), 不做;
//! - **仓内既有的敏感度词表**: 只有 `apeireth_guard::observation::DataSensitivity`
//!   (`engine/guard/src/observation.rs:39-70`) —— 那是**动作级污点**分类 (每个工具
//!   调用一次), 且在 **engine 层**, core 依赖方向是 `core ← protocol ← plugin`
//!   (core 无内部 crate 依赖) ⇒ **core 不可消费**, 也不是任务级输入。
//!
//! ⇒ **结论: 显式声明**。信号由**调用方显式提供** ([`PrivacySignal::Declared`]),
//! 缺失时按普通档兜底 ([`PrivacySignal::Defaulted`]) 且在决策里**显式标注**
//! (不静默); 显式给了但值不认识 ⇒ **报错拒路由**, 不静默降级 (fail-closed)。
//!
//! **信号口** (装配批接这三个函数即可, 0 新 env 名):
//!
//! - [`privacy_signal_from_str`] —— 从**原始值**读信号 (`None` ⇒ 缺位兜底;
//!   空白/不认识 ⇒ **报错**, 不静默降级); 载体字段名 = [`PRIVACY_SIGNAL_KEY`]
//!   (本地载体: 会话设置 / CLI 参数 ⇒ **绝不**进会外发的请求元数据);
//! - [`TaskProfile::from_signal`] —— 把信号接进任务画像 (⇒ [`TaskProfile::privacy`]
//!   读生效等级, [`RoutingDecision::privacy_source`] 读来源)。
//!
//! ⚠️ 载体由装配批选, 但**不要**选「会外发到厂商」的通道 (例如规范请求的
//! `metadata`: `protocol/src/adapters/openai_chat.rs:188-193` 会把它整包写进出网
//! body ⇒ 声明本身会外泄)。
//!
//! # 接线面 (面名 `routing_policy`, **不是** `tool.*` 能力 id)
//!
//! - 开关口径: **复用** [`crate::cost_router::COST_ROUTING_ENV`]
//!   (`APEIRETH_COST_ROUTING`) 与 [`crate::cost_router::COST_ROUTING_PREFERRED_ENV`]
//!   ⇒ **0 新 env 名**; 语义沿用成本档层的 fail-closed 口径 (只有精确 `1` 开);
//! - 隐私事实的落点: 画像行的能力标记 ([`PRIVACY_FLAG_RESTRICTED`] /
//!   [`PRIVACY_FLAG_CONFIDENTIAL`]) 或调用方自建的候选表
//!   ([`PolicyCandidate::table_from_profiles`]) —— **不新设 env 名**;
//! - 自述面: **两个口分开** —— [`RoutingPolicy::last_decision`] (最近一次**成功**
//!   决策) + [`RoutingPolicy::last_attempt`] (最近一次**尝试**: 成功**或**被拒 ⇒
//!   「因隐私被拒」这种最该被看见的事件不会因为「没选出模型」而消失)。

use std::fmt;
use std::sync::Mutex;

use crate::cost_router::{
    comparison_price_micros, infer_tier, CostRouter, CostRoutingConfig, CostRoutingError, CostTier,
    RoutingRequest, TierSource,
};
use crate::model_profile::{normalize_model_id, ModelProfile, ModelProfileTable};

/// 画像行能力标记: 该模型**可承接受限档**数据 (上限 = [`PrivacyLevel::Restricted`])。
///
/// 画像行的 `capabilities_flags` 是开放词表 (snake_case); 本层读这两个标记作为
/// **隐私事实**: 标记在 ⇒ 可承接对应敏感度; 标记缺位 ⇒ **只承接普通档**
/// (缺省 = 未提供 ⇒ 按最严口径处理, fail-closed)。
pub const PRIVACY_FLAG_RESTRICTED: &str = "privacy_restricted";

/// 画像行能力标记: 该模型**可承接机密档**数据 (上限 = [`PrivacyLevel::Confidential`])。
///
/// 与 [`PRIVACY_FLAG_RESTRICTED`] 同时出现 ⇒ 取**低档**
/// ([`PrivacyLevel::Restricted`], 保守: 两个标记矛盾时按**更小的可承接上限**处理,
/// 不靠标记放宽)。
pub const PRIVACY_FLAG_CONFIDENTIAL: &str = "privacy_confidential";

/// 隐私等级 (数据敏感度三档闭合词表)。
///
/// 阶梯 (低 → 高): [`PrivacyLevel::Ordinary`] < [`PrivacyLevel::Restricted`]
/// < [`PrivacyLevel::Confidential`]。
///
/// - **任务侧** ([`TaskProfile::privacy`]): 本次任务数据的敏感度;
/// - **候选侧** ([`PolicyCandidate::max_privacy`]): 该候选**可承接**的最高敏感度。
///
/// 闸位口径: `候选上限 >= 任务等级` 才放行 —— 隐私是**硬闸**, 宁可无候选也不越闸
/// (安全 > 功能 > 性能, 照项目 O-1 纪律)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PrivacyLevel {
    /// 普通档: 一般数据, 可发给一般外部服务。
    Ordinary,
    /// 受限档: 受控数据, 只可发给受控/自有部署的服务。
    Restricted,
    /// 机密档: 敏感数据, 只可发给端侧/私有部署。
    Confidential,
}

impl PrivacyLevel {
    /// 词表全集 (顺序即敏感度阶梯)。
    pub const ALL: [PrivacyLevel; 3] = [
        PrivacyLevel::Ordinary,
        PrivacyLevel::Restricted,
        PrivacyLevel::Confidential,
    ];

    /// 词表拼写 (`ordinary|restricted|confidential`)。
    pub const fn as_str(self) -> &'static str {
        match self {
            PrivacyLevel::Ordinary => "ordinary",
            PrivacyLevel::Restricted => "restricted",
            PrivacyLevel::Confidential => "confidential",
        }
    }

    /// 词表解析 (ASCII 大小写不敏感, **去首尾空白** —— 与成本档层
    /// [`CostRoutingConfig::switch_enabled`] 的 trim 口径一致); 未知词 = `None`
    /// (词表闭合, 不猜档)。
    pub fn parse(name: &str) -> Option<PrivacyLevel> {
        let normalized = name.trim().to_ascii_lowercase();
        PrivacyLevel::ALL
            .into_iter()
            .find(|level| level.as_str() == normalized)
    }
}

impl fmt::Display for PrivacyLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 隐私信号的来源 (可解释字段: 这一档是**显式声明**的, 还是**缺省兜底**的)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrivacySource {
    /// 调用方显式声明的隐私等级。
    Declared,
    /// 信号缺位 ⇒ 按普通档兜底 (**显式标注**, 不静默)。
    #[default]
    Defaulted,
}

impl PrivacySource {
    /// 来源拼写 (`declared|defaulted`)。
    pub const fn as_str(self) -> &'static str {
        match self {
            PrivacySource::Declared => "declared",
            PrivacySource::Defaulted => "defaulted",
        }
    }
}

impl fmt::Display for PrivacySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 隐私信号: 一条**显式声明**或一次**缺省兜底**。
///
/// 两种形态的差别**必须**在决策里看得见 (缺省兜底 = 「没人声明过」, 与「声明了
/// 普通档」不是一回事) ⇒ 决策同时带 [`PrivacyLevel`] 与 [`PrivacySource`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrivacySignal {
    /// 调用方显式声明的隐私等级。
    Declared(PrivacyLevel),
    /// 信号缺位 ⇒ 按普通档兜底 (**显式标注**, 不静默)。
    #[default]
    Defaulted,
}

impl PrivacySignal {
    /// 生效的隐私等级 (缺省兜底 = 普通档 —— 特性默认关 ⇒ 逐字节零行为变化)。
    pub const fn level(self) -> PrivacyLevel {
        match self {
            PrivacySignal::Declared(level) => level,
            PrivacySignal::Defaulted => PrivacyLevel::Ordinary,
        }
    }

    /// 信号来源 (显式声明 / 缺省兜底)。
    pub const fn source(self) -> PrivacySource {
        match self {
            PrivacySignal::Declared(_) => PrivacySource::Declared,
            PrivacySignal::Defaulted => PrivacySource::Defaulted,
        }
    }

    /// 是否是**显式声明**的信号。
    pub const fn is_declared(self) -> bool {
        matches!(self, PrivacySignal::Declared(_))
    }

    /// 决策里的可解释标注 (缺省兜底必须写明「没人声明过」)。
    pub fn annotation(self) -> String {
        match self {
            PrivacySignal::Declared(level) => {
                format!("隐私等级 {level} (调用方显式声明)")
            }
            PrivacySignal::Defaulted => {
                format!(
                    "隐私等级 {} (缺省兜底: 调用方未提供隐私信号 ⇒ 按最不敏感档处理)",
                    PrivacyLevel::Ordinary
                )
            }
        }
    }
}

/// 隐私档在本族**本地载体**里的字段名 (`privacy_level`)。
///
/// ⚠️ **只用于本地载体** (会话设置 / 本地表 / CLI 参数) ⇒ **绝不**放进会外发到
/// 厂商的请求元数据通道 (`NormalizedRequest.metadata` 会被整包写进出网 body ⇒
/// 声明本身会外泄)。
pub const PRIVACY_SIGNAL_KEY: &str = "privacy_level";

/// 隐私信号解析失败。**值自由报错**: 只报键名与期望词表, 从不回显被解析的值。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrivacySignalError {
    /// 显式声明的值不在词表内 ⇒ **拒路由** (fail-closed: 不静默降级为普通档,
    /// 那等于把「声明了但看不懂」当成「没人声明」= 静默放宽)。
    InvalidValue {
        /// 出错的键名 (键名是配置面, 不是值)。
        key: String,
        /// 期望词表说明 (不含值)。
        reason: String,
    },
}

impl fmt::Display for PrivacySignalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidValue { key, reason } => {
                write!(f, "privacy signal entry {key:?} is invalid: {reason}")
            }
        }
    }
}

impl std::error::Error for PrivacySignalError {}

/// 隐私信号端口 (纯函数): 从**原始值**读一条信号。
///
/// - `None` (**缺位**) ⇒ [`PrivacySignal::Defaulted`] (信号缺位 ⇒ 普通档兜底,
///   **由调用方在决策里显式标注**, 不静默);
/// - 认识的词 ⇒ [`PrivacySignal::Declared`] (ASCII 大小写不敏感, 去首尾空白);
/// - **其它一切** (不认识 / 空白 / 空串) ⇒ [`PrivacySignalError::InvalidValue`]
///   ⇒ **拒路由**, 不静默降级 —— 「显式传了空」与「没人声明」必须可区分
///   (当缺位处理 = 静默放宽)。
pub fn privacy_signal_from_str(value: Option<&str>) -> Result<PrivacySignal, PrivacySignalError> {
    let Some(raw) = value else {
        return Ok(PrivacySignal::Defaulted);
    };
    match PrivacyLevel::parse(raw) {
        Some(level) => Ok(PrivacySignal::Declared(level)),
        None => Err(PrivacySignalError::InvalidValue {
            key: PRIVACY_SIGNAL_KEY.to_string(),
            reason: format!(
                "unknown privacy level (accepted: {})",
                PrivacyLevel::ALL
                    .iter()
                    .map(|level| level.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }),
    }
}

/// 任务的能力需求 (投影成成本档层的路由信号)。
///
/// 字段语义与成本档层的 [`RoutingRequest`] **同源** (本层只是换个形状, 不换口径):
/// 信号缺位就照实 `None` —— **不猜** (缺信号不推轻档, 规则在成本档层里)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CapabilityRequirement {
    /// 任务本身是否需要工具 (需要时候选必须显式支持工具; 未知不等于支持)。
    pub requires_tools: bool,
    /// 期望工具调用数; `None` = 信号缺位。
    pub expected_tool_calls: Option<u64>,
    /// 上下文规模估计 (token); `None` = 信号缺位。
    pub estimated_context_tokens: Option<u64>,
}

impl CapabilityRequirement {
    /// 不要求工具、信号全缺位的能力需求 (零假设)。
    pub fn new() -> Self {
        Self::default()
    }

    /// 声明任务是否需要工具。
    #[must_use]
    pub fn with_requires_tools(mut self, requires_tools: bool) -> Self {
        self.requires_tools = requires_tools;
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

    /// 投影成成本档层的路由请求 (成本档位判定与档位过滤都吃它)。
    pub fn to_routing_request(&self, tier: Option<CostTier>) -> RoutingRequest {
        RoutingRequest {
            explicit_tier: tier,
            expected_tool_calls: self.expected_tool_calls,
            estimated_context_tokens: self.estimated_context_tokens,
            requires_tools: self.requires_tools,
        }
    }
}

/// 任务画像 = **能力需求 × 隐私等级 × 成本档**。
///
/// - 能力需求: [`CapabilityRequirement`];
/// - 隐私信号: [`PrivacySignal`] —— `Declared(等级)` = 调用方显式声明;
///   `Defaulted` = 信号缺位 ⇒ 按普通档兜底 (**决策里显式标注**, 不静默);
/// - 成本档: [`Option<CostTier>`] —— `Some` = 显式档 (最高优先); `None` = 交给
///   成本档层的启发式按能力信号推断 (口径复用 [`infer_tier`], 不在本层重写)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskProfile {
    /// 能力需求 (工具 / 上下文规模 / 期望工具调用数)。
    pub capability: CapabilityRequirement,
    /// 显式成本档; `None` = 按能力信号推断。
    pub cost_tier: Option<CostTier>,
    /// 隐私信号 (显式声明 / 缺省兜底)。
    pub privacy_signal: PrivacySignal,
}

impl TaskProfile {
    /// 一张任务画像 (隐私等级 = **显式声明**)。
    ///
    /// 传入等级即视为调用方**显式声明** ⇒ 信号来源 = [`PrivacySource::Declared`];
    /// 要走「信号缺位 ⇒ 普通档兜底」的口径, 用 [`TaskProfile::from_signal`]。
    pub fn new(
        capability: CapabilityRequirement,
        cost_tier: Option<CostTier>,
        privacy: PrivacyLevel,
    ) -> Self {
        Self::from_signal(capability, cost_tier, PrivacySignal::Declared(privacy))
    }

    /// 一张任务画像 (隐私信号由信号口给)。
    pub fn from_signal(
        capability: CapabilityRequirement,
        cost_tier: Option<CostTier>,
        privacy_signal: PrivacySignal,
    ) -> Self {
        Self {
            capability,
            cost_tier,
            privacy_signal,
        }
    }

    /// 生效的隐私等级 (缺省兜底 = 普通档)。
    pub fn privacy(&self) -> PrivacyLevel {
        self.privacy_signal.level()
    }

    /// 点名显式成本档 (压过启发式推断)。
    #[must_use]
    pub fn with_cost_tier(mut self, tier: CostTier) -> Self {
        self.cost_tier = Some(tier);
        self
    }

    /// 显式声明任务的隐私等级 (信号来源随之变成「显式声明」)。
    #[must_use]
    pub fn with_privacy(mut self, privacy: PrivacyLevel) -> Self {
        self.privacy_signal = PrivacySignal::Declared(privacy);
        self
    }

    /// 显式声明「信号缺位」⇒ 普通档兜底 (决策里会标注出来)。
    #[must_use]
    pub fn without_privacy_signal(mut self) -> Self {
        self.privacy_signal = PrivacySignal::Defaulted;
        self
    }

    /// 投影成成本档层的路由请求 (成本档位判定的唯一入口)。
    pub fn routing_request(&self) -> RoutingRequest {
        self.capability.to_routing_request(self.cost_tier)
    }
}

/// 策略候选表的一行: 一个**可路由**模型 + 它**可承接的最高隐私等级**。
///
/// 隐私上限是**候选侧的策略事实**: 模型能承接多敏感的数据, 由部署形态与合规
/// 边界决定, 不是模型画像里的能力事实 ⇒ 本层用候选表承载, **不改画像表结构**。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyCandidate {
    /// 模型 id (匹配大小写不敏感, 与画像表同一口径)。
    pub model_id: String,
    /// 该候选可承接的最高隐私等级 (任务等级高于它 ⇒ 排除并给理由)。
    pub max_privacy: PrivacyLevel,
}

impl PolicyCandidate {
    /// 一行候选 (模型 id + 隐私上限)。
    pub fn new(model_id: impl Into<String>, max_privacy: PrivacyLevel) -> Self {
        Self {
            model_id: model_id.into(),
            max_privacy,
        }
    }

    /// 匹配键 (model id 归一, 大小写不敏感)。
    pub fn key(&self) -> String {
        normalize_model_id(&self.model_id)
    }

    /// 从画像表派生候选表: 隐私事实读画像行的能力标记
    /// ([`PRIVACY_FLAG_RESTRICTED`] / [`PRIVACY_FLAG_CONFIDENTIAL`])。
    ///
    /// - 标记缺位 ⇒ 上限 = [`PrivacyLevel::Ordinary`] (**缺省 = 未提供 ⇒ 只承接
    ///   普通档**, fail-closed);
    /// - 两个标记同时在场 ⇒ 取**低档** (保守: 按更小的可承接上限处理, 标记不放宽);
    /// - 画像表**每一行**都登记 ⇒ 不存在「未登记 ⇒ 不可路由」的情形 (要那种语义
    ///   就自建候选表)。
    pub fn table_from_profiles(profiles: &ModelProfileTable) -> Vec<PolicyCandidate> {
        profiles
            .profiles()
            .iter()
            .map(|profile| PolicyCandidate {
                model_id: profile.model_id.clone(),
                max_privacy: Self::privacy_from_flags(&profile.capabilities_flags),
            })
            .collect()
    }

    /// 画像行的能力标记 ⇒ 隐私上限 (缺省 = 普通档; 双标取**低档** (保守))。
    fn privacy_from_flags(flags: &[String]) -> PrivacyLevel {
        let restricted = flags.iter().any(|flag| flag == PRIVACY_FLAG_RESTRICTED);
        let confidential = flags.iter().any(|flag| flag == PRIVACY_FLAG_CONFIDENTIAL);
        match (restricted, confidential) {
            (true, false) => PrivacyLevel::Restricted,
            (false, true) => PrivacyLevel::Confidential,
            (true, true) => PrivacyLevel::Restricted,
            (false, false) => PrivacyLevel::Ordinary,
        }
    }
}

/// 候选逐行比较的结论。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateOutcome {
    /// 选中者。
    Selected,
    /// 通过全部闸门 (可作退路)。
    Eligible,
    /// 被排除 (理由见 [`CandidateComparison::exclusion_reason`])。
    Excluded,
}

impl CandidateOutcome {
    /// 结论拼写 (`selected|eligible|excluded`)。
    pub const fn as_str(self) -> &'static str {
        match self {
            CandidateOutcome::Selected => "selected",
            CandidateOutcome::Eligible => "eligible",
            CandidateOutcome::Excluded => "excluded",
        }
    }
}

impl fmt::Display for CandidateOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 候选逐行比较 (可解释: 每个候选过没过、为什么、单价多少)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateComparison {
    /// 模型 id。
    pub model_id: String,
    /// 比较结论 (选中 / 备选 / 排除)。
    pub outcome: CandidateOutcome,
    /// 排除理由 (被排除时给出, 多条以 `; ` 连接; 入选/备选为 `None`)。
    pub exclusion_reason: Option<String>,
    /// 比较单价 (微元/千 token, 合计口径); `None` = **未提供 ⇒ 不参与价格比较**。
    pub comparison_price_micros: Option<u64>,
    /// 该行是否参与了价格比较 (缺价 / 被排除 ⇒ `false`)。
    pub price_compared: bool,
}

/// 策略层的一次选路决策 (可解释: 选谁 / 为什么 / 退路是谁 / 逐行比较)。
///
/// ⚠️ 与成本档层的决策 (成本档层的 `RoutingDecision`) 是**两个不同层的类型**:
/// 本类型多出**隐私维度**、**逐行比较**与**退路链**。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingDecision {
    /// 定下的成本档 (口径复用成本档层)。
    pub tier: CostTier,
    /// 档位来源 (显式 / 推断 / 默认)。
    pub tier_source: TierSource,
    /// 生效的隐私等级 (任务画像声明; 缺省兜底 = 普通档)。
    pub privacy: PrivacyLevel,
    /// 隐私信号的来源 (显式声明 / 缺省兜底) —— 缺省兜底必须看得见。
    pub privacy_source: PrivacySource,
    /// 选中的模型 id。
    pub selected_model: String,
    /// 选中模型的画像行 (画像表无该行 = **显式 unknown 行**, 不编造能力事实)。
    pub selected_profile: ModelProfile,
    /// 选定理由 (哪条规则命中 —— 含成本档层的阶梯理由与本层的画像上下文)。
    pub selection_reason: String,
    /// 退路链: 选中者之后**依次**可顶替的候选 (有序, 不含选中者)。
    pub fallback_chain: Vec<String>,
    /// 退路说明 (退路是谁 / 按什么口径排 / 无退路时如实说明)。
    pub fallback_reason: Option<String>,
    /// 候选逐行比较 (含被排除者的排除理由 —— **不静默过滤**)。
    pub comparisons: Vec<CandidateComparison>,
}

/// 策略层决策失败的形态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingPolicyError {
    /// 策略层未开 (fail-closed): 不路由, 不产生任何决策。
    Disabled,
    /// 成本档过滤后**零候选** (画像表在该档没有过得了的模型)。
    NoCostCandidate {
        /// 该次判定的成本档。
        tier: CostTier,
        /// 档位来源 (显式 / 推断 / 默认)。
        tier_source: TierSource,
        /// 如实说明 (含档位过滤条件的真值)。
        reason: String,
    },
    /// 候选**全部被排除** ⇒ 明确失败, **不随便挑一个**、不静默回退默认模型。
    NoEligibleCandidate {
        /// 该次判定的成本档。
        tier: CostTier,
        /// 档位来源 (显式 / 推断 / 默认)。
        tier_source: TierSource,
        /// 生效的隐私等级。
        privacy: PrivacyLevel,
        /// 隐私信号的来源 (显式声明 / 缺省兜底)。
        privacy_source: PrivacySource,
        /// 如实说明 (为什么全军覆没)。
        reason: String,
        /// 逐行比较 (每行带排除理由 —— 排除**可见**; 错误帧里**没有**「过了闸」
        ///   的行, 行状态与「已被闸排除」一致)。
        comparisons: Vec<CandidateComparison>,
    },
}

impl fmt::Display for RoutingPolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled => {
                write!(
                    f,
                    "routing policy is off ({} = 1 enables it); no model is selected",
                    crate::cost_router::COST_ROUTING_ENV
                )
            }
            Self::NoCostCandidate {
                tier,
                tier_source,
                reason,
            } => {
                write!(
                    f,
                    "routing policy found no candidate at the {tier} tier ({tier_source:?}): {reason}"
                )
            }
            Self::NoEligibleCandidate {
                tier,
                privacy,
                reason,
                ..
            } => {
                write!(
                    f,
                    "routing policy found no eligible candidate at the {tier} tier under privacy level {privacy}: {reason}"
                )
            }
        }
    }
}

impl std::error::Error for RoutingPolicyError {}

/// 一次路由**尝试**的结论。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttemptOutcome {
    /// 选出了模型。
    Selected,
    /// 被拒 (哪道闸拒的见 [`RoutingAttempt::rejected_by`]).
    Rejected,
}

impl AttemptOutcome {
    /// 结论拼写 (`selected|rejected`)。
    pub const fn as_str(self) -> &'static str {
        match self {
            AttemptOutcome::Selected => "selected",
            AttemptOutcome::Rejected => "rejected",
        }
    }
}

impl fmt::Display for AttemptOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 被拒的**维度** (哪一道闸拒的) —— 自述面要能说清「因隐私被拒」而不是
/// 「路由失败」。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttemptDimension {
    /// **隐私闸**: 候选可承接的上限低于任务的隐私等级。
    Privacy,
    /// **候选表闸**: 候选未登记在策略候选表 (或画像行事实全缺)。
    CandidateTable,
    /// **成本档闸**: 画像表里没有模型通过该档过滤。
    CostTier,
}

impl AttemptDimension {
    /// 维度拼写 (`privacy|candidate_table|cost_tier`)。
    pub const fn as_str(self) -> &'static str {
        match self {
            AttemptDimension::Privacy => "privacy",
            AttemptDimension::CandidateTable => "candidate_table",
            AttemptDimension::CostTier => "cost_tier",
        }
    }
}

impl fmt::Display for AttemptDimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 一次路由尝试的事实 (成功**或**被拒)。
///
/// ⚠️ 与 [`RoutingDecision`] 是**两个口**: 决策口只记**成功**
/// ([`RoutingPolicy::last_decision`]), 尝试口**成功与被拒都记**
/// ([`RoutingPolicy::last_attempt`]) ⇒ 「因隐私被拒」这种最该被看见的事件
/// 不会因为「没选出模型」而消失。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingAttempt {
    /// 结论 (选定 / 被拒)。
    pub outcome: AttemptOutcome,
    /// 被拒的维度; 选定 = `None`.
    pub rejected_by: Option<AttemptDimension>,
    /// 理由 (被拒 = 为什么; 选定 = 选定理由)。
    pub reason: String,
    /// 参与评估的候选数 (成本档候选数; 被拒时 = 被评估后全被排掉的那些)。
    pub candidate_count: usize,
    /// 生效的成本档。
    pub tier: CostTier,
    /// 生效的隐私等级。
    pub privacy: PrivacyLevel,
    /// 隐私信号来源 (显式声明 / 缺省兜底)。
    pub privacy_source: PrivacySource,
}

impl RoutingAttempt {
    /// 这次尝试是不是**被拒**了。
    pub fn is_rejected(&self) -> bool {
        self.outcome == AttemptOutcome::Rejected
    }
}

/// 统一路由策略层: 候选表 + 成本档层复用 + 可解释决策。
///
/// 生命周期: 策略层**不持锁状态**以外的东西 —— 成本档层实例 + 候选表 + 配置面。
/// 最近一次决策记在内部 ([`RoutingPolicy::last_decision`]), 供自述路由节读取;
/// 最近一次**尝试** (含被拒) 记在 [`RoutingPolicy::last_attempt`]。
#[derive(Debug)]
pub struct RoutingPolicy {
    /// 策略候选表 (可路由模型 + 隐私上限; 同 id 重复登记取更严的一档)。
    candidates: Vec<PolicyCandidate>,
    /// 成本档层实例 (**复用**: 档位判定 + 档位过滤矩阵, 语义原样)。
    cost_router: CostRouter,
    /// 画像表 (筛选来源; 与成本档层同源)。
    profiles: ModelProfileTable,
    /// 回退目标模型 (生效模型 id 口径; 与成本档层同源)。
    default_model: Option<String>,
    /// 配置面 (开关 + 档位首选映射; 与成本档层同源 ⇒ **0 新 env 名**)。
    config: CostRoutingConfig,
    /// 最近一次**成功**的决策 (失败不记 —— 照实)。
    last_decision: Mutex<Option<RoutingDecision>>,
    /// 最近一次**尝试** (成功或被拒都记 —— 被拒的事实也必须看得见)。
    last_attempt: Mutex<Option<RoutingAttempt>>,
}

impl RoutingPolicy {
    /// 装配一台策略层。
    ///
    /// - `candidates`: 策略候选表 (可路由模型 + 隐私上限);
    /// - `profiles`: 模型画像表 (成本档层与本层同源);
    /// - `default_model`: 默认模型 (回退目标口径, 与成本档层同源);
    /// - `config`: 配置面 (**复用** [`CostRoutingConfig`]: 开关 =
    ///   `APEIRETH_COST_ROUTING`, 档位首选 = `APEIRETH_COST_ROUTING_PREFERRED`) ⇒
    ///   **0 新 env 名**。
    pub fn new(
        candidates: Vec<PolicyCandidate>,
        profiles: ModelProfileTable,
        default_model: Option<String>,
        config: CostRoutingConfig,
    ) -> Self {
        let cost_router = CostRouter::new(profiles.clone(), default_model.clone(), config.clone());
        Self {
            candidates,
            cost_router,
            profiles,
            default_model,
            config,
            last_decision: Mutex::new(None),
            last_attempt: Mutex::new(None),
        }
    }

    /// 从两份 env 原始值装配 (纯函数, 不直接读环境): 开关原始值 + 档位映射 JSON。
    ///
    /// 口径完全复用成本档层的 fail-closed 解析 (**0 新 env 名**); 任一层坏 =
    /// 整体拒开 (值自由报错, 由 [`CostRoutingError`] 承担)。
    pub fn from_env_values(
        candidates: Vec<PolicyCandidate>,
        profiles: ModelProfileTable,
        default_model: Option<String>,
        enabled_raw: Option<&str>,
        preferred_raw: Option<&str>,
    ) -> Result<Self, CostRoutingError> {
        let config = CostRoutingConfig::from_env_values(enabled_raw, preferred_raw)?;
        Ok(Self::new(candidates, profiles, default_model, config))
    }

    /// 开关真值 (fail-closed: 只有 `APEIRETH_COST_ROUTING=1` 才开)。
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// 策略候选表 (同 model id 重复登记时**取更严的一档**, fail-closed 不放宽)。
    pub fn candidate_table(&self) -> Vec<PolicyCandidate> {
        let mut merged: Vec<PolicyCandidate> = Vec::new();
        for candidate in &self.candidates {
            let key = candidate.key();
            match merged.iter_mut().find(|row| row.key() == key) {
                Some(existing) => {
                    if candidate.max_privacy < existing.max_privacy {
                        existing.max_privacy = candidate.max_privacy;
                    }
                }
                None => merged.push(candidate.clone()),
            }
        }
        merged
    }

    /// 最近一次**成功**的决策; 本策略层尚未成功路由过 = `None` (照实, 不编造)。
    pub fn last_decision(&self) -> Option<RoutingDecision> {
        let last = self
            .last_decision
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        last.clone()
    }

    /// 最近一次**尝试** (成功**或**被拒); 本策略层尚未尝试过 = `None` (照实)。
    ///
    /// ⚠️ 与 [`RoutingPolicy::last_decision`] **两个口分开**: 决策口只记成功
    /// (「最近一次决策」的口径不被拒绝污染), 尝试口**被拒也记** ⇒ 自述面能显示
    /// 「最近一次被隐私闸拒绝」(否则最该被看见的事件不可见)。
    pub fn last_attempt(&self) -> Option<RoutingAttempt> {
        let last = self
            .last_attempt
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        last.clone()
    }

    /// 记一次尝试 (成功与被拒都记)。
    fn record_attempt(&self, attempt: RoutingAttempt) {
        let mut last = self
            .last_attempt
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *last = Some(attempt);
    }

    /// 决策: 任务画像 ⇒ 候选 ⇒ 选定 (含逐行比较与退路链)。
    ///
    /// 失败形态 (见 [`RoutingPolicyError`]): 开关关 ⇒ `Disabled`; 成本档零候选 ⇒
    /// `NoCostCandidate`; 候选全被排除 ⇒ `NoEligibleCandidate` (带逐行理由)。
    pub fn decide(&self, profile: &TaskProfile) -> Result<RoutingDecision, RoutingPolicyError> {
        if !self.config.enabled {
            return Err(RoutingPolicyError::Disabled);
        }

        let request = profile.routing_request();
        // 隐私信号: 生效等级 (缺省兜底 = 普通档) + 来源 (显式声明 / 缺省兜底)。
        let privacy_level = profile.privacy();
        let privacy_source = profile.privacy_signal.source();
        // ① 成本档判定 (复用成本档层的显式档 > 启发式推断 > 默认档口径)。
        let (tier, tier_source) = infer_tier(&request);
        // ② 档位过滤 (复用成本档层的成本矩阵: 能力需求 × 窗口下限 × 单价上限)。
        let cost_candidates = self.cost_router.candidates(tier, &request);
        if cost_candidates.is_empty() {
            let spec = tier.spec();
            let reason = format!(
                "画像表里没有模型通过 {tier} 档过滤 (能力需求 × 窗口下限 {} token × 比较单价上限 {} 微元/千 token); 策略层不替成本档层造候选",
                spec.min_context_window, spec.price_ceiling_micros
            );
            self.record_attempt(RoutingAttempt {
                outcome: AttemptOutcome::Rejected,
                rejected_by: Some(AttemptDimension::CostTier),
                reason: reason.clone(),
                candidate_count: 0,
                tier,
                privacy: privacy_level,
                privacy_source,
            });
            return Err(RoutingPolicyError::NoCostCandidate {
                tier,
                tier_source,
                reason,
            });
        }

        // ③ 逐行评估: 隐私闸 (硬闸) → 候选表闸 → 成本档闸。
        let evaluation = self.evaluate_candidates(profile, tier, &cost_candidates);
        if evaluation.survivors.is_empty() {
            let reason = format!(
                "{tier} 档的 {} 个候选全部被策略层排除 (隐私等级 {privacy} 闸 / 候选表闸 / 档位过滤); 宁可不选也不越闸",
                cost_candidates.len(),
                privacy = privacy_level
            );
            self.record_attempt(RoutingAttempt {
                outcome: AttemptOutcome::Rejected,
                rejected_by: Some(evaluation.dominant_rejection_dimension()),
                reason: reason.clone(),
                candidate_count: cost_candidates.len(),
                tier,
                privacy: privacy_level,
                privacy_source,
            });
            return Err(RoutingPolicyError::NoEligibleCandidate {
                tier,
                tier_source,
                privacy: privacy_level,
                reason,
                privacy_source,
                comparisons: evaluation.comparisons,
            });
        }

        // ④ 选定: 存活集交给成本档层跑既定阶梯 (档位首选 > 最低单价 > 默认模型 >
        //    画像表顺序) ⇒ 成本口径与成本档层**同源**, 本层不重写。
        let survivor_keys: Vec<String> = evaluation
            .survivors
            .iter()
            .map(|profile| normalize_model_id(&profile.model_id))
            .collect();
        let restricted_profiles = ModelProfileTable::new(
            self.profiles
                .profiles()
                .iter()
                .filter(|profile| survivor_keys.contains(&normalize_model_id(&profile.model_id)))
                .cloned()
                .collect(),
        );
        // 退路排序用**画像表顺序**作平手口径 (与成本档层的「平手取画像表顺序」同源)。
        let ordered = fallback_order(restricted_profiles.profiles());
        let restricted = CostRouter::new(
            restricted_profiles,
            self.default_model.clone(),
            self.config.clone(),
        );
        let cost_decision = restricted
            .route(&request)
            .ok_or(RoutingPolicyError::Disabled)?;
        let Some(selected_model) = cost_decision.selected_model else {
            // 错误帧里**没有**「过了闸」的行: 存活集的行在此一律记为落选 (带理由),
            // 行状态与「已被闸排除」一致 (不出现「全军覆没」却写着 eligible 的矛盾帧)。
            let mut comparisons = evaluation.comparisons;
            for row in &mut comparisons {
                if row.outcome == CandidateOutcome::Eligible {
                    row.outcome = CandidateOutcome::Excluded;
                    row.exclusion_reason =
                        Some("未进入选定: 成本档层在存活集上未选出模型 (默认模型缺席)".to_string());
                    row.price_compared = false;
                }
            }
            let reason =
                "存活集非空但成本档层未选出模型 (默认模型缺席); 如实选不出, 不硬凑".to_string();
            self.record_attempt(RoutingAttempt {
                outcome: AttemptOutcome::Rejected,
                rejected_by: Some(AttemptDimension::CostTier),
                reason: reason.clone(),
                candidate_count: evaluation.survivors.len(),
                tier,
                privacy: privacy_level,
                privacy_source,
            });
            return Err(RoutingPolicyError::NoEligibleCandidate {
                tier,
                tier_source,
                privacy: privacy_level,
                privacy_source,
                reason,
                comparisons,
            });
        };

        // ⑤ 退路链: 存活集按比较单价升序 (有价优先; 缺价候选不参与价格比较、排其后)。
        let selected_key = normalize_model_id(&selected_model);
        let fallback_chain: Vec<String> = ordered
            .iter()
            .filter(|model_id| normalize_model_id(model_id) != selected_key)
            .cloned()
            .collect();
        let fallback_reason = if fallback_chain.is_empty() {
            Some(format!(
                "无退路: 策略层筛选后只有 {selected_model} 这一个候选 (选中者), 没有其它可顶替的候选; \
                 退路排序口径 = 比较单价升序 (与选定阶梯不是同一顺序)"
            ))
        } else {
            Some(format!(
                "退路 {} 个 (选中者不可用时依次顶替): {}; 排序口径: 比较单价升序 (有价候选优先), 缺价候选不参与价格比较、排在其后; \
                 ⚠️ 退路排序 (单价升序) **不等于**选定阶梯顺序 (档位首选 > 最便宜 > 默认模型 > 画像表顺序) ⇒ 退路第一名未必是阶梯第二名, 不可当重试顺序用",
                fallback_chain.len(),
                fallback_chain.join(" → ")
            ))
        };

        // ⑥ 逐行比较: 把选中者标出来 (排除行**保持带理由在场**)。
        let mut comparisons = evaluation.comparisons;
        for row in &mut comparisons {
            if normalize_model_id(&row.model_id) == selected_key {
                row.outcome = CandidateOutcome::Selected;
                row.exclusion_reason = None;
            }
        }

        let selection_reason = self.selection_reason(
            &cost_decision.selection_reason,
            tier,
            tier_source,
            profile.privacy_signal,
            evaluation.survivors.len(),
            &comparisons,
        );

        let decision = RoutingDecision {
            tier,
            tier_source,
            privacy: privacy_level,
            privacy_source,
            selected_profile: self.profiles.resolve(&selected_model).into_owned(),
            selected_model,
            selection_reason,
            fallback_chain,
            fallback_reason,
            comparisons,
        };
        let mut last = self
            .last_decision
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *last = Some(decision.clone());
        // 尝试口: 成功也记 (与决策口**两个口分开**; 被拒的事实见上面的拒绝分支)。
        self.record_attempt(RoutingAttempt {
            outcome: AttemptOutcome::Selected,
            rejected_by: None,
            reason: decision.selection_reason.clone(),
            candidate_count: evaluation.survivors.len(),
            tier,
            privacy: privacy_level,
            privacy_source,
        });
        Ok(decision)
    }

    /// 逐行评估候选 (隐私闸 → 候选表闸 → 成本档闸)。
    fn evaluate_candidates(
        &self,
        profile: &TaskProfile,
        tier: CostTier,
        cost_candidates: &[ModelProfile],
    ) -> Evaluation {
        let cost_keys: Vec<String> = cost_candidates
            .iter()
            .map(|candidate| normalize_model_id(&candidate.model_id))
            .collect();
        let mut comparisons: Vec<CandidateComparison> = Vec::new();
        let mut survivors: Vec<ModelProfile> = Vec::new();
        let mut registered: Vec<String> = Vec::new();
        // 各闸的排除计数 (「被哪道闸拒的」的事实来源, 不靠解析理由文本)。
        let mut privacy_excluded = 0usize;
        let mut table_excluded = 0usize;
        let mut tier_excluded = 0usize;

        for candidate in self.candidate_table() {
            let key = candidate.key();
            registered.push(key.clone());
            let model_profile = self.profiles.resolve(&candidate.model_id).into_owned();
            let price = comparison_price_micros(&model_profile);
            let mut reasons: Vec<String> = Vec::new();

            // 隐私闸 (硬闸): 候选可承接的上限低于任务等级 ⇒ 排除 (安全 > 功能 > 性能)。
            if candidate.max_privacy < profile.privacy() {
                privacy_excluded += 1;
                reasons.push(format!(
                    "隐私等级 {} 超过该候选允许的上限 {} (隐私闸硬排除; 宁可无候选也不越闸)",
                    profile.privacy(),
                    candidate.max_privacy
                ));
            }
            // 候选表闸: 未登记 / 事实缺位 / 没过档位过滤 ⇒ 排除 (理由可见)。
            if self.profiles.get(&candidate.model_id).is_none() {
                table_excluded += 1;
                reasons.push("画像表无该模型行 (模型事实未提供, 不编造)".to_string());
            } else if model_profile.is_unknown() {
                table_excluded += 1;
                reasons.push("画像表的该模型行事实全缺 (能力事实未提供, 不编造)".to_string());
            } else if !cost_keys.contains(&key) {
                tier_excluded += 1;
                reasons.push(format!(
                    "未通过 {tier} 档过滤 (工具支持 / 上下文窗口 / 比较单价 三条件之一未满足)"
                ));
            }

            if reasons.is_empty() {
                survivors.push(model_profile.clone());
                comparisons.push(CandidateComparison {
                    model_id: candidate.model_id.clone(),
                    outcome: CandidateOutcome::Eligible,
                    exclusion_reason: None,
                    comparison_price_micros: price,
                    price_compared: price.is_some(),
                });
            } else {
                comparisons.push(CandidateComparison {
                    model_id: candidate.model_id.clone(),
                    outcome: CandidateOutcome::Excluded,
                    exclusion_reason: Some(reasons.join("; ")),
                    comparison_price_micros: price,
                    price_compared: false,
                });
            }
        }

        // 过了成本档但**未登记**在候选表的行: 排除并说明 (不静默消失)。
        for model_profile in cost_candidates {
            let key = normalize_model_id(&model_profile.model_id);
            if registered.contains(&key) {
                continue;
            }
            comparisons.push(CandidateComparison {
                model_id: model_profile.model_id.clone(),
                outcome: CandidateOutcome::Excluded,
                exclusion_reason: Some(
                    "未登记在策略候选表 (策略层候选闸: 未登记 = 不可路由, 不默认放行)".to_string(),
                ),
                comparison_price_micros: comparison_price_micros(model_profile),
                price_compared: false,
            });
        }

        Evaluation {
            survivors,
            comparisons,
            privacy_excluded,
            table_excluded,
            tier_excluded,
        }
    }

    /// 选定理由 = 成本档层的阶梯理由 + 本层的画像上下文 (哪条规则命中)。
    fn selection_reason(
        &self,
        cost_reason: &str,
        tier: CostTier,
        tier_source: TierSource,
        privacy_signal: PrivacySignal,
        survivor_count: usize,
        comparisons: &[CandidateComparison],
    ) -> String {
        let mut reason = format!(
            "{cost_reason}; 策略层上下文: 成本档 {tier} (来源 {}) × {}, 通过全部闸门的候选 {survivor_count} 个",
            tier_source.as_str(),
            privacy_signal.annotation()
        );
        // 档位首选模型若被隐私闸/候选表闸排除, 如实说明 (不静默跳过)。
        if let Some(preferred) = self.config.preferred_models.get(&tier) {
            let preferred_key = normalize_model_id(preferred);
            if let Some(row) = comparisons
                .iter()
                .find(|row| normalize_model_id(&row.model_id) == preferred_key)
            {
                if row.outcome == CandidateOutcome::Excluded {
                    reason.push_str(&format!(
                        "; 档位首选模型 {preferred} 未进入候选 ({})",
                        row.exclusion_reason.as_deref().unwrap_or("被策略层排除")
                    ));
                }
            }
        }
        reason
    }
}

/// 一次评估的中间结果 (内部)。
#[derive(Debug)]
struct Evaluation {
    /// 通过全部闸门的候选 (画像表顺序)。
    survivors: Vec<ModelProfile>,
    /// 逐行比较 (含被排除者的理由)。
    comparisons: Vec<CandidateComparison>,
    /// 被隐私闸排除的行数。
    privacy_excluded: usize,
    /// 被候选表闸排除的行数。
    table_excluded: usize,
    /// 被成本档闸排除的行数。
    tier_excluded: usize,
}

impl Evaluation {
    /// 排掉候选最多的那道闸 (平手取**更严**的一档: 隐私 → 候选表 → 成本档)。
    fn dominant_rejection_dimension(&self) -> AttemptDimension {
        if self.privacy_excluded > 0
            && self.privacy_excluded >= self.table_excluded
            && self.privacy_excluded >= self.tier_excluded
        {
            AttemptDimension::Privacy
        } else if self.table_excluded > 0 && self.table_excluded >= self.tier_excluded {
            AttemptDimension::CandidateTable
        } else {
            AttemptDimension::CostTier
        }
    }
}

/// 退路排序口径: 比较单价升序 (有价候选优先, 平手取画像表顺序), 缺价候选
/// **不参与价格比较**、排在其后 (缺价不编价, 与成本档层同一纪律)。
fn fallback_order(survivors: &[ModelProfile]) -> Vec<String> {
    let mut priced: Vec<&ModelProfile> = survivors
        .iter()
        .filter(|profile| comparison_price_micros(profile).is_some())
        .collect();
    priced.sort_by_key(|profile| comparison_price_micros(profile).unwrap_or_default());
    let unpriced: Vec<&ModelProfile> = survivors
        .iter()
        .filter(|profile| comparison_price_micros(profile).is_none())
        .collect();
    priced
        .into_iter()
        .chain(unpriced)
        .map(|profile| profile.model_id.clone())
        .collect()
}
