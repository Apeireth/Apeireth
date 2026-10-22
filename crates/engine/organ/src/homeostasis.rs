//! homeostasis —— SG-14 异稳态生命体征调节内核 v0（生物机制族 W1 批；机制内核，未接线）。
//!
//! # 生物原型（灵感源，非保真模拟）
//!
//! - **异稳态（allostasis）**：生命体按「预期」提前调节，而不是等指标越限才熔断；
//!   调节目标是围绕设定值（setpoint）的**预测性偏移**，不是单点恒定。对应到工程：
//!   [`HomeostasisRegulator::regulate_with_trend`] 用一窗口趋势外推，在下一窗口越限前动作。
//! - **适应负荷（allostatic load）**：长期调节失败会留下累积代价（慢性应激负荷意象）。
//!   对应到工程：[`AllostaticLoad`] 按调节计划的严重度做确定性线性记账。
//! - **生命体征 5 维**：上下文压力 / 记忆负荷 / 预算余额 / 错误率 / 关系新鲜度
//!   （[`VitalSign`]）。前三维与错误率是「越高越紧张」，预算余额与关系新鲜度是
//!   「越低越紧张」（方向由 [`VitalSign::higher_is_worse`] 固定）。
//!
//! # 工程边界（明确不做什么）
//!
//! - **纯函数、输入显式注入**：不读系统时钟、不读全局状态、不做 IO、无 RNG、无 LLM。
//! - **规则是确定性优先级表**：`PRIORITY_TABLE` 的行序即优先级（严重度降序 → 体征固定序 →
//!   体征内动作固定序），同一动作只保留最高优先级的一次出现（稳定去重）；不是 if 迷宫。
//! - **趋势 = 一窗口线性外推**：`next + (next - prev)`，夹到 `[0, 1]`；只回答「下一窗口
//!   会不会升到更严重档」，不建模长期曲线、不做统计学习、不预测多窗口。
//! - **阈值/设定值是调用方给的工程参数**，不是生理测量；本模块不做参数标定。
//! - **非有限观测值按 `0.0` 折叠**（NaN 与 −∞ 落 0，+∞ 收边到 1），见
//!   [`VitalSignReading::sanitized`]；输出不携带 NaN，理由字符串也只格式化净化后的数值。
//!   折叠后预算/关系新鲜度的 NaN 会落到最紧张侧（保守触发），上下文/记忆/错误率会落到
//!   最宽松侧 —— 这是显式的确定性取舍，不是静默容错。
//! - **非有限阈值视为该档不可用（跳过）**，见 [`VitalSign::crosses`]：配置缺失不应被
//!   放大成永久警报（也不会因折叠方向不同而忽严忽松）；其余有限阈值照常判定。
//!
//! # 规则表（唯一真源：本文件内 `PRIORITY_TABLE`）
//!
//! 行内动作按执行顺序排列；「整编」= [`RegulationAction::ConsolidateBeforeOverflow`]，
//! 「降召回」= [`RegulationAction::ReduceRecallDepth`]，「节流」=
//! [`RegulationAction::ThrottleProactiveMessages`]，「恢复」=
//! [`RegulationAction::RestoreBudget`]，「升级」= [`RegulationAction::EscalateToHuman`]。
//!
//! | 档位 | 上下文压力 | 记忆负荷 | 预算余额 | 错误率 | 关系新鲜度 |
//! |---|---|---|---|---|---|
//! | Critical | 整编 → 降召回 → 升级 | 整编 → 降召回 → 升级 | 节流 → 降召回 → 恢复 → 升级 | 节流 → 降召回 → 升级 | 降召回 → 升级 |
//! | High | 整编 → 降召回 | 整编 → 降召回 | 节流 → 恢复 | 节流 → 降召回 | 降召回 |
//! | Warn | 降召回 | 整编 | 节流 | 节流 | 降召回 |
//!
//! 预测阶段优先：若线性外推让某体征升档（且当前未到该档），该档动作整组排在
//! 当前状态动作之前；同一动作仍只保留第一次出现（归因给预测 / 更高优先级体征）。
//!
//! # 能力级别（四级口径）：IMPLEMENTED（已实现、未接线）
//!
//! 不是 PRODUCTION WIRED、不是 DEFAULT ENABLED、不是 HARDWARE VALIDATED：
//!
//! - **未接 runtime 回合循环**：没有任何器官在每轮采集 [`VitalSignSet`]；
//! - **无 `self_status` 节**：本模块既不产出也不消费自述状态；
//! - **未接 dreaming/care 调度**：`ConsolidateBeforeOverflow` 尚未路由到整编/做梦管线，
//!   `ThrottleProactiveMessages` 尚未接到关怀发起闸；
//! - 无持久化、无工具面、无遥测、无日志副作用。
//!
//! 接线点（W2 候选，不在本批）：上下文组装/记忆服务/预算账本/错误统计/关系模型 →
//! [`VitalSignSet`]；[`RegulationPlan::steps`] → 回合前置动作序列；
//! [`AllostaticLoad::level`] → `self_status` 一节；趋势输入 → 连续两轮的体征快照。

use serde::{Deserialize, Serialize};

/// 把任意 `f64` 折叠到 `[0, 1]`：NaN → `0.0`，±∞ 由 `clamp` 收边。
fn sanitize_unit(value: f64) -> f64 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, 1.0)
    }
}

/// 生命体征维度（声明顺序即遍历顺序，优先级表与汇总都以此为准）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VitalSign {
    /// 上下文压力：上下文窗口占用（越高越紧张）。
    ContextPressure,
    /// 记忆负荷：待整编 / 驻留记忆量（越高越紧张）。
    MemoryLoad,
    /// 预算余额：剩余调用预算（越低越紧张）。
    BudgetRemaining,
    /// 错误率：近期失败回合占比（越高越紧张）。
    ErrorRate,
    /// 关系新鲜度：关系上下文的陈旧程度（越低越紧张）。
    RelationshipFreshness,
}

impl VitalSign {
    /// 全部 5 维，顺序固定（优先级、汇总与测试都以此为准）。
    pub const ALL: [Self; 5] = [
        Self::ContextPressure,
        Self::MemoryLoad,
        Self::BudgetRemaining,
        Self::ErrorRate,
        Self::RelationshipFreshness,
    ];

    /// 中文名（用于理由字符串 / 日志）。
    pub const fn label(self) -> &'static str {
        match self {
            Self::ContextPressure => "上下文压力",
            Self::MemoryLoad => "记忆负荷",
            Self::BudgetRemaining => "预算余额",
            Self::ErrorRate => "错误率",
            Self::RelationshipFreshness => "关系新鲜度",
        }
    }

    /// 方向：`true` = 数值越高越紧张；`false` = 数值越低越紧张。
    pub const fn higher_is_worse(self) -> bool {
        match self {
            Self::ContextPressure | Self::MemoryLoad | Self::ErrorRate => true,
            Self::BudgetRemaining | Self::RelationshipFreshness => false,
        }
    }

    /// 该体征的 `value` 是否已触及 `threshold`（方向感知；**等于阈值即算触及**）。
    ///
    /// 非有限阈值（NaN / ±∞）表示该档阈值不可用：直接返回 `false`（跳过该档），
    /// 避免把配置缺失放大成永久警报；有限阈值先夹到 `[0, 1]` 再比较。
    pub fn crosses(self, value: f64, threshold: f64) -> bool {
        if !threshold.is_finite() {
            return false;
        }
        let threshold = sanitize_unit(threshold);
        if self.higher_is_worse() {
            value >= threshold
        } else {
            value <= threshold
        }
    }
}

/// 严重度档位（声明序即排序：`Normal < Warn < High < Critical`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// 阈值内。
    Normal,
    /// 触及 `warn` 阈值。
    Warn,
    /// 触及 `high` 阈值。
    High,
    /// 触及 `critical` 阈值。
    Critical,
}

impl Severity {
    /// 全部档位（从轻到重）。
    pub const ALL: [Self; 4] = [Self::Normal, Self::Warn, Self::High, Self::Critical];

    /// 中文名。
    pub const fn label(self) -> &'static str {
        match self {
            Self::Normal => "正常",
            Self::Warn => "警告",
            Self::High => "高",
            Self::Critical => "危急",
        }
    }

    /// 是否需要调节动作（`Normal` 以外都算警报）。
    pub const fn is_alarm(self) -> bool {
        !matches!(self, Self::Normal)
    }
}

/// 单项生命体征读数：观测值 + 设定值 + 三档阈值，统一在 `[0, 1]` 标度上。
///
/// 字段按原样保存（serde 可回放原始输入）；判定时观测值先过 [`Self::value_unit`]
/// 净化，阈值经 [`VitalSign::crosses`] 夹边（非有限阈值跳过该档），
/// 因此越界 / 非有限值不会让判定失控。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VitalSignReading {
    /// 当前观测值 `[0, 1]`。
    pub value: f64,
    /// 设定值（异稳态目标点，用于理由里的偏差说明）。
    pub setpoint: f64,
    /// `warn` 阈值。
    pub warn: f64,
    /// `high` 阈值。
    pub high: f64,
    /// `critical` 阈值。
    pub critical: f64,
}

impl VitalSignReading {
    /// 直接构造（不做净化；判定时统一走 [`Self::sanitized`]）。
    pub const fn new(value: f64, setpoint: f64, warn: f64, high: f64, critical: f64) -> Self {
        Self {
            value,
            setpoint,
            warn,
            high,
            critical,
        }
    }

    /// 按体征取工程默认设定值与阈值（方向已对齐，见各分支）。
    ///
    /// 默认值是工程起点，不是标定结果；调用方可直接改字段覆盖。
    pub const fn defaults_for(sign: VitalSign, value: f64) -> Self {
        match sign {
            // 越高越紧张：warn < high < critical。
            VitalSign::ContextPressure => Self::new(value, 0.20, 0.65, 0.80, 0.92),
            VitalSign::MemoryLoad => Self::new(value, 0.25, 0.70, 0.85, 0.95),
            VitalSign::ErrorRate => Self::new(value, 0.02, 0.05, 0.15, 0.30),
            // 越低越紧张：warn > high > critical。
            VitalSign::BudgetRemaining => Self::new(value, 0.80, 0.30, 0.15, 0.05),
            VitalSign::RelationshipFreshness => Self::new(value, 0.85, 0.50, 0.25, 0.10),
        }
    }

    /// 净化：逐字段折叠 NaN → `0.0`、越界 / ±∞ → `[0, 1]`。
    ///
    /// 用于回放 / 展示；严重度判定只对观测值取净化值，阈值保持原值以保留
    /// 「非有限阈值 = 该档不可用」的语义（见 [`VitalSign::crosses`]）。
    pub fn sanitized(self) -> Self {
        Self {
            value: sanitize_unit(self.value),
            setpoint: sanitize_unit(self.setpoint),
            warn: sanitize_unit(self.warn),
            high: sanitize_unit(self.high),
            critical: sanitize_unit(self.critical),
        }
    }

    /// 净化后的观测值 `[0, 1]`。
    pub fn value_unit(self) -> f64 {
        sanitize_unit(self.value)
    }

    /// 净化后的偏差：`value - setpoint`，范围 `[-1, 1]`。
    pub fn deviation(self) -> f64 {
        sanitize_unit(self.value) - sanitize_unit(self.setpoint)
    }

    /// 按体征方向判定严重度：三档独立比较，取触达的最高档。
    ///
    /// 观测值先净化；阈值保持原值传入 [`VitalSign::crosses`]（非有限阈值按该档跳过）。
    pub fn severity_for(self, sign: VitalSign) -> Severity {
        let value = self.value_unit();
        if sign.crosses(value, self.critical) {
            Severity::Critical
        } else if sign.crosses(value, self.high) {
            Severity::High
        } else if sign.crosses(value, self.warn) {
            Severity::Warn
        } else {
            Severity::Normal
        }
    }

    /// 某档对应的净化阈值；`Normal` 没有阈值，返回 `warn` 作占位（不用于判定）。
    pub fn threshold_for(self, level: Severity) -> f64 {
        let reading = self.sanitized();
        match level {
            Severity::Critical => reading.critical,
            Severity::High => reading.high,
            Severity::Warn | Severity::Normal => reading.warn,
        }
    }
}

/// 5 维生命体征集合（字段与 [`VitalSign::ALL`] 一一对应）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VitalSignSet {
    /// 上下文压力。
    pub context_pressure: VitalSignReading,
    /// 记忆负荷。
    pub memory_load: VitalSignReading,
    /// 预算余额。
    pub budget_remaining: VitalSignReading,
    /// 错误率。
    pub error_rate: VitalSignReading,
    /// 关系新鲜度。
    pub relationship_freshness: VitalSignReading,
}

impl VitalSignSet {
    /// 直接构造 5 维读数。
    pub const fn new(
        context_pressure: VitalSignReading,
        memory_load: VitalSignReading,
        budget_remaining: VitalSignReading,
        error_rate: VitalSignReading,
        relationship_freshness: VitalSignReading,
    ) -> Self {
        Self {
            context_pressure,
            memory_load,
            budget_remaining,
            error_rate,
            relationship_freshness,
        }
    }

    /// 健康基线：各维贴近设定值，5 维都不越限。
    pub fn healthy() -> Self {
        Self {
            context_pressure: VitalSignReading::defaults_for(VitalSign::ContextPressure, 0.20),
            memory_load: VitalSignReading::defaults_for(VitalSign::MemoryLoad, 0.25),
            budget_remaining: VitalSignReading::defaults_for(VitalSign::BudgetRemaining, 0.80),
            error_rate: VitalSignReading::defaults_for(VitalSign::ErrorRate, 0.01),
            relationship_freshness: VitalSignReading::defaults_for(
                VitalSign::RelationshipFreshness,
                0.85,
            ),
        }
    }

    /// 按体征取只读读数。
    pub fn get(&self, sign: VitalSign) -> &VitalSignReading {
        match sign {
            VitalSign::ContextPressure => &self.context_pressure,
            VitalSign::MemoryLoad => &self.memory_load,
            VitalSign::BudgetRemaining => &self.budget_remaining,
            VitalSign::ErrorRate => &self.error_rate,
            VitalSign::RelationshipFreshness => &self.relationship_freshness,
        }
    }

    /// 按体征取可变读数。
    pub fn get_mut(&mut self, sign: VitalSign) -> &mut VitalSignReading {
        match sign {
            VitalSign::ContextPressure => &mut self.context_pressure,
            VitalSign::MemoryLoad => &mut self.memory_load,
            VitalSign::BudgetRemaining => &mut self.budget_remaining,
            VitalSign::ErrorRate => &mut self.error_rate,
            VitalSign::RelationshipFreshness => &mut self.relationship_freshness,
        }
    }

    /// 写入某一维读数。
    pub fn set(&mut self, sign: VitalSign, reading: VitalSignReading) {
        *self.get_mut(sign) = reading;
    }

    /// 链式写入某一维读数。
    pub fn with(mut self, sign: VitalSign, reading: VitalSignReading) -> Self {
        self.set(sign, reading);
        self
    }

    /// 按体征判定严重度。
    pub fn severity(&self, sign: VitalSign) -> Severity {
        self.get(sign).severity_for(sign)
    }

    /// 5 维中最坏的严重度（空集不可能，兜底 `Normal`）。
    pub fn worst_severity(&self) -> Severity {
        VitalSign::ALL
            .iter()
            .map(|sign| self.severity(*sign))
            .max()
            .unwrap_or(Severity::Normal)
    }
}

/// 调节动作（有序计划里的原子动作；[`Self::None`] 只在无越限时单独出现）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegulationAction {
    /// 上下文将满前先整编：把驻留内容压成摘要，防溢出。
    ConsolidateBeforeOverflow,
    /// 节流主动消息：少花预算 / 少在故障期把错误外溢给用户。
    ThrottleProactiveMessages,
    /// 降低召回深度：缩短检索半径、减少上下文注入量。
    ReduceRecallDepth,
    /// 恢复预算：请求补充或等待额度回填。
    RestoreBudget,
    /// 升级人工：交给主人或上层决策，机制侧不擅自继续。
    EscalateToHuman,
    /// 无动作（系统健康，含一窗口趋势预测也未越限）。
    None,
}

impl RegulationAction {
    /// 中文名。
    pub const fn label(self) -> &'static str {
        match self {
            Self::ConsolidateBeforeOverflow => "提前整编",
            Self::ThrottleProactiveMessages => "节流主动消息",
            Self::ReduceRecallDepth => "降低召回深度",
            Self::RestoreBudget => "恢复预算",
            Self::EscalateToHuman => "升级人工",
            Self::None => "无动作",
        }
    }

    /// 是否为「无动作」哨兵。
    pub const fn is_none(self) -> bool {
        matches!(self, Self::None)
    }
}

/// 计划中的一步：动作 + 触发体征 + 档位 + 是否预测 + 中文理由。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegulationStep {
    /// 动作。
    pub action: RegulationAction,
    /// 触发体征；[`RegulationAction::None`] 时为 `None`。
    pub sign: Option<VitalSign>,
    /// 触发档位（预测步为外推后的档位）。
    pub severity: Severity,
    /// 是否来自「下一窗口会升档」的趋势外推。
    pub predicted: bool,
    /// 中文理由（只含净化后的数值，绝不出现 NaN / inf）。
    pub reason: String,
}

/// 一次调节决策：有序动作步骤 + 总体严重度 + 中文摘要。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegulationPlan {
    /// 有序动作步骤（健康时是单步 [`RegulationAction::None`]）。
    pub steps: Vec<RegulationStep>,
    /// 总体严重度 = 当前最坏档与趋势外推最坏档的较大者。
    pub severity: Severity,
    /// 中文摘要（越限体征 + 动作计数 + 预测计数）。
    pub summary: String,
}

impl RegulationPlan {
    /// 有序动作序列（含可能的 [`RegulationAction::None`]）。
    pub fn actions(&self) -> Vec<RegulationAction> {
        self.steps.iter().map(|step| step.action).collect()
    }

    /// 是否什么都不用做（空计划或仅 `None`）。
    pub fn is_noop(&self) -> bool {
        self.steps.iter().all(|step| step.action.is_none())
    }

    /// 预测步数量。
    pub fn predicted_count(&self) -> usize {
        self.steps.iter().filter(|step| step.predicted).count()
    }
}

/// 一条优先级规则：`(体征, 档位) → 动作`；表行序即优先级。
#[derive(Debug, Clone, Copy)]
struct Rule {
    sign: VitalSign,
    severity: Severity,
    action: RegulationAction,
}

/// 确定性优先级表（唯一规则源，改规则只改这张表）。
///
/// **行序语义**：严重度降序（`Critical → High → Warn`）→ 体征按
/// [`VitalSign::ALL`] 顺序 → 同一体征内按动作固定顺序。
/// 判定时只取「恰好等于该档」的行；高档行已显式包含低档动作（累积语义），
/// 因此不需要在运行时做「>= 档位」的二次展开。
/// 同一动作在整份计划里只保留第一次出现（稳定去重），归因给最高优先级的体征。
const PRIORITY_TABLE: &[Rule] = &[
    // ───── Critical ─────
    Rule {
        sign: VitalSign::ContextPressure,
        severity: Severity::Critical,
        action: RegulationAction::ConsolidateBeforeOverflow,
    },
    Rule {
        sign: VitalSign::ContextPressure,
        severity: Severity::Critical,
        action: RegulationAction::ReduceRecallDepth,
    },
    Rule {
        sign: VitalSign::ContextPressure,
        severity: Severity::Critical,
        action: RegulationAction::EscalateToHuman,
    },
    Rule {
        sign: VitalSign::MemoryLoad,
        severity: Severity::Critical,
        action: RegulationAction::ConsolidateBeforeOverflow,
    },
    Rule {
        sign: VitalSign::MemoryLoad,
        severity: Severity::Critical,
        action: RegulationAction::ReduceRecallDepth,
    },
    Rule {
        sign: VitalSign::MemoryLoad,
        severity: Severity::Critical,
        action: RegulationAction::EscalateToHuman,
    },
    Rule {
        sign: VitalSign::BudgetRemaining,
        severity: Severity::Critical,
        action: RegulationAction::ThrottleProactiveMessages,
    },
    Rule {
        sign: VitalSign::BudgetRemaining,
        severity: Severity::Critical,
        action: RegulationAction::ReduceRecallDepth,
    },
    Rule {
        sign: VitalSign::BudgetRemaining,
        severity: Severity::Critical,
        action: RegulationAction::RestoreBudget,
    },
    Rule {
        sign: VitalSign::BudgetRemaining,
        severity: Severity::Critical,
        action: RegulationAction::EscalateToHuman,
    },
    Rule {
        sign: VitalSign::ErrorRate,
        severity: Severity::Critical,
        action: RegulationAction::ThrottleProactiveMessages,
    },
    Rule {
        sign: VitalSign::ErrorRate,
        severity: Severity::Critical,
        action: RegulationAction::ReduceRecallDepth,
    },
    Rule {
        sign: VitalSign::ErrorRate,
        severity: Severity::Critical,
        action: RegulationAction::EscalateToHuman,
    },
    Rule {
        sign: VitalSign::RelationshipFreshness,
        severity: Severity::Critical,
        action: RegulationAction::ReduceRecallDepth,
    },
    Rule {
        sign: VitalSign::RelationshipFreshness,
        severity: Severity::Critical,
        action: RegulationAction::EscalateToHuman,
    },
    // ───── High ─────
    Rule {
        sign: VitalSign::ContextPressure,
        severity: Severity::High,
        action: RegulationAction::ConsolidateBeforeOverflow,
    },
    Rule {
        sign: VitalSign::ContextPressure,
        severity: Severity::High,
        action: RegulationAction::ReduceRecallDepth,
    },
    Rule {
        sign: VitalSign::MemoryLoad,
        severity: Severity::High,
        action: RegulationAction::ConsolidateBeforeOverflow,
    },
    Rule {
        sign: VitalSign::MemoryLoad,
        severity: Severity::High,
        action: RegulationAction::ReduceRecallDepth,
    },
    Rule {
        sign: VitalSign::BudgetRemaining,
        severity: Severity::High,
        action: RegulationAction::ThrottleProactiveMessages,
    },
    Rule {
        sign: VitalSign::BudgetRemaining,
        severity: Severity::High,
        action: RegulationAction::RestoreBudget,
    },
    Rule {
        sign: VitalSign::ErrorRate,
        severity: Severity::High,
        action: RegulationAction::ThrottleProactiveMessages,
    },
    Rule {
        sign: VitalSign::ErrorRate,
        severity: Severity::High,
        action: RegulationAction::ReduceRecallDepth,
    },
    Rule {
        sign: VitalSign::RelationshipFreshness,
        severity: Severity::High,
        action: RegulationAction::ReduceRecallDepth,
    },
    // ───── Warn ─────
    Rule {
        sign: VitalSign::ContextPressure,
        severity: Severity::Warn,
        action: RegulationAction::ReduceRecallDepth,
    },
    Rule {
        sign: VitalSign::MemoryLoad,
        severity: Severity::Warn,
        action: RegulationAction::ConsolidateBeforeOverflow,
    },
    Rule {
        sign: VitalSign::BudgetRemaining,
        severity: Severity::Warn,
        action: RegulationAction::ThrottleProactiveMessages,
    },
    Rule {
        sign: VitalSign::ErrorRate,
        severity: Severity::Warn,
        action: RegulationAction::ThrottleProactiveMessages,
    },
    Rule {
        sign: VitalSign::RelationshipFreshness,
        severity: Severity::Warn,
        action: RegulationAction::ReduceRecallDepth,
    },
];

/// 当前状态下的调节器入参快照：把 `next` 与阈值一起投影到下一窗口。
///
/// 只外推一窗口；阈值沿用 `next` 的净化阈值。返回值已净化，不会越出 `[0, 1]`。
fn project_reading(prev: &VitalSignReading, next: &VitalSignReading) -> VitalSignReading {
    let delta = next.value_unit() - prev.value_unit();
    let mut projected = next.sanitized();
    projected.value = sanitize_unit(next.value_unit() + delta);
    projected
}

/// 把某一 `(体征, 档位)` 的所有表行动作追加进计划（稳定去重）。
fn push_rules(
    steps: &mut Vec<RegulationStep>,
    seen: &mut Vec<RegulationAction>,
    sign: VitalSign,
    level: Severity,
    predicted: bool,
    reading: &VitalSignReading,
) {
    let threshold = reading.threshold_for(level);
    for rule in PRIORITY_TABLE
        .iter()
        .filter(|rule| rule.sign == sign && rule.severity == level)
    {
        if rule.action.is_none() || seen.contains(&rule.action) {
            continue;
        }
        seen.push(rule.action);
        let reason = if predicted {
            format!(
                "趋势外推：{} 下一窗口预计 {:.2} 触及 {} 档（阈值 {:.2}），提前执行「{}」",
                sign.label(),
                reading.value_unit(),
                level.label(),
                threshold,
                rule.action.label()
            )
        } else {
            format!(
                "{} 现值 {:.2} 触及 {} 档（阈值 {:.2}），执行「{}」",
                sign.label(),
                reading.value_unit(),
                level.label(),
                threshold,
                rule.action.label()
            )
        };
        steps.push(RegulationStep {
            action: rule.action,
            sign: Some(sign),
            severity: level,
            predicted,
            reason,
        });
    }
}

/// 异稳态调节器（无状态；两个纯函数入口）。
#[derive(Debug, Clone, Copy, Default)]
pub struct HomeostasisRegulator;

impl HomeostasisRegulator {
    /// 构造调节器。
    pub const fn new() -> Self {
        Self
    }

    /// 按当前生命体征给出调节计划。
    pub fn regulate(&self, vitals: &VitalSignSet) -> RegulationPlan {
        self.plan(vitals, None)
    }

    /// 预测性调节：`prev → next` 一窗口线性外推，若会升档则把动作提到当前计划里。
    ///
    /// 预测步排在当前状态步之前（提前量优先）；同一动作仍只出现一次。
    pub fn regulate_with_trend(&self, prev: &VitalSignSet, next: &VitalSignSet) -> RegulationPlan {
        self.plan(next, Some(prev))
    }

    fn plan(&self, current: &VitalSignSet, prev: Option<&VitalSignSet>) -> RegulationPlan {
        let current_severity = VitalSign::ALL.map(|sign| current.severity(sign));
        let projected_severity = prev.map(|prev| {
            VitalSign::ALL
                .map(|sign| project_reading(prev.get(sign), current.get(sign)).severity_for(sign))
        });

        let mut steps: Vec<RegulationStep> = Vec::new();
        let mut seen: Vec<RegulationAction> = Vec::new();

        // 阶段 A：预测动作（下一窗口升档且当前还没到该档）。
        if let Some(prev) = prev {
            let projected = projected_severity.unwrap_or([Severity::Normal; 5]);
            for level in [Severity::Critical, Severity::High, Severity::Warn] {
                for (index, sign) in VitalSign::ALL.iter().enumerate() {
                    if projected[index] == level && current_severity[index] < level {
                        let reading = project_reading(prev.get(*sign), current.get(*sign));
                        push_rules(&mut steps, &mut seen, *sign, level, true, &reading);
                    }
                }
            }
        }

        // 阶段 B：当前状态动作（严重度降序 → 体征固定序）。
        for level in [Severity::Critical, Severity::High, Severity::Warn] {
            for (index, sign) in VitalSign::ALL.iter().enumerate() {
                if current_severity[index] == level {
                    push_rules(
                        &mut steps,
                        &mut seen,
                        *sign,
                        level,
                        false,
                        current.get(*sign),
                    );
                }
            }
        }

        let worst_current = current.worst_severity();
        let worst_projected = projected_severity
            .map(|projected| projected.iter().copied().max().unwrap_or(Severity::Normal))
            .unwrap_or(Severity::Normal);
        let severity = worst_current.max(worst_projected);

        if steps.is_empty() {
            steps.push(RegulationStep {
                action: RegulationAction::None,
                sign: None,
                severity: Severity::Normal,
                predicted: false,
                reason: "全部生命体征在阈值内（含一窗口趋势外推），无需调节".to_string(),
            });
        }

        let mut summary_parts: Vec<String> = Vec::new();
        for (index, sign) in VitalSign::ALL.iter().enumerate() {
            if current_severity[index].is_alarm() {
                summary_parts.push(format!(
                    "{}={}",
                    sign.label(),
                    current_severity[index].label()
                ));
            }
        }
        if let Some(projected) = projected_severity {
            for (index, sign) in VitalSign::ALL.iter().enumerate() {
                if projected[index] > current_severity[index] {
                    summary_parts.push(format!(
                        "{}预测={}",
                        sign.label(),
                        projected[index].label()
                    ));
                }
            }
        }
        let action_count = steps.iter().filter(|step| !step.action.is_none()).count();
        let predicted_count = steps.iter().filter(|step| step.predicted).count();
        let summary = if summary_parts.is_empty() {
            format!("最坏严重度 {}；无越限体征，无调节动作", severity.label())
        } else {
            format!(
                "最坏严重度 {}；体征 [{}]；动作 {} 个（其中预测 {} 个）",
                severity.label(),
                summary_parts.join("，"),
                action_count,
                predicted_count
            )
        };

        RegulationPlan {
            steps,
            severity,
            summary,
        }
    }
}

/// 适应负荷记账：长期调节失败的累积代价（确定性线性模型）。
///
/// - `Critical` 计划按 [`Self::RISE_PER_SECOND_CRITICAL`] 累积；
/// - `High` 计划按 [`Self::RISE_PER_SECOND_HIGH`] 累积；
/// - `Warn` 计划**保持**（长期低度警报本身就是负荷，但不再加码）；
/// - `Normal` 计划按 [`Self::RECOVERY_PER_SECOND`] 恢复。
///
/// 饱和到 `[0, 1]`；达到 [`Self::CHRONIC_THRESHOLD`] 视为慢性适应负荷。
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct AllostaticLoad {
    level: f64,
}

impl AllostaticLoad {
    /// 危急档每秒累积速率。
    pub const RISE_PER_SECOND_CRITICAL: f64 = 0.02;
    /// 高档每秒累积速率。
    pub const RISE_PER_SECOND_HIGH: f64 = 0.01;
    /// 正常档每秒恢复速率。
    pub const RECOVERY_PER_SECOND: f64 = 0.005;
    /// 慢性适应负荷判定阈值。
    pub const CHRONIC_THRESHOLD: f64 = 0.70;

    /// 构造零负荷。
    pub const fn new() -> Self {
        Self { level: 0.0 }
    }

    /// 当前负荷 `[0, 1]`（净化后读取）。
    pub fn level(&self) -> f64 {
        sanitize_unit(self.level)
    }

    /// 是否已达慢性适应负荷阈值。
    pub fn is_chronic(&self) -> bool {
        self.level() >= Self::CHRONIC_THRESHOLD
    }

    /// 按一次调节计划与经过时长推进负荷，返回推进后的 `level`。
    ///
    /// `elapsed_ms` 由调用方注入（本模块不读时钟）；超大时长由饱和兜底。
    pub fn observe(&mut self, plan: &RegulationPlan, elapsed_ms: u64) -> f64 {
        let seconds = elapsed_ms as f64 / 1000.0;
        let rate = match plan.severity {
            Severity::Critical => Self::RISE_PER_SECOND_CRITICAL,
            Severity::High => Self::RISE_PER_SECOND_HIGH,
            Severity::Warn => 0.0,
            Severity::Normal => -Self::RECOVERY_PER_SECOND,
        };
        self.level = sanitize_unit(self.level() + rate * seconds);
        self.level()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_one(sign: VitalSign, value: f64) -> VitalSignSet {
        VitalSignSet::healthy().with(sign, VitalSignReading::defaults_for(sign, value))
    }

    #[test]
    fn severity_thresholds_are_inclusive_and_direction_aware() {
        let higher_is_worse = [
            (0.649_9, Severity::Normal),
            (0.65, Severity::Warn),
            (0.799_9, Severity::Warn),
            (0.80, Severity::High),
            (0.919_9, Severity::High),
            (0.92, Severity::Critical),
            (1.0, Severity::Critical),
        ];
        for (value, expected) in higher_is_worse {
            let reading = VitalSignReading::defaults_for(VitalSign::ContextPressure, value);
            assert_eq!(
                reading.severity_for(VitalSign::ContextPressure),
                expected,
                "context_pressure={value}"
            );
        }

        let lower_is_worse = [
            (0.300_1, Severity::Normal),
            (0.30, Severity::Warn),
            (0.150_1, Severity::Warn),
            (0.15, Severity::High),
            (0.050_1, Severity::High),
            (0.05, Severity::Critical),
            (0.0, Severity::Critical),
        ];
        for (value, expected) in lower_is_worse {
            let reading = VitalSignReading::defaults_for(VitalSign::BudgetRemaining, value);
            assert_eq!(
                reading.severity_for(VitalSign::BudgetRemaining),
                expected,
                "budget_remaining={value}"
            );
        }
    }

    #[test]
    fn healthy_set_is_noop_with_explicit_none_action() {
        let plan = HomeostasisRegulator::new().regulate(&VitalSignSet::healthy());
        assert_eq!(plan.actions(), vec![RegulationAction::None]);
        assert!(plan.is_noop());
        assert_eq!(plan.severity, Severity::Normal);
        assert_eq!(plan.predicted_count(), 0);
        assert!(plan.summary.contains("正常"));
        assert!(!plan.summary.contains("NaN"));
    }

    #[test]
    fn priority_table_is_stable_and_deduplicated() {
        let set = VitalSignSet::healthy()
            .with(
                VitalSign::ContextPressure,
                VitalSignReading::defaults_for(VitalSign::ContextPressure, 0.95),
            )
            .with(
                VitalSign::MemoryLoad,
                VitalSignReading::defaults_for(VitalSign::MemoryLoad, 0.99),
            )
            .with(
                VitalSign::BudgetRemaining,
                VitalSignReading::defaults_for(VitalSign::BudgetRemaining, 0.01),
            );
        let regulator = HomeostasisRegulator::new();
        let plan = regulator.regulate(&set);
        assert_eq!(
            plan.actions(),
            vec![
                RegulationAction::ConsolidateBeforeOverflow,
                RegulationAction::ReduceRecallDepth,
                RegulationAction::EscalateToHuman,
                RegulationAction::ThrottleProactiveMessages,
                RegulationAction::RestoreBudget,
            ]
        );
        assert_eq!(plan.severity, Severity::Critical);
        // 确定性：同一输入再跑一次，计划逐字段相等。
        assert_eq!(plan, regulator.regulate(&set));
    }

    #[test]
    fn critical_rows_precede_high_rows() {
        let set = VitalSignSet::healthy()
            .with(
                VitalSign::ContextPressure,
                VitalSignReading::defaults_for(VitalSign::ContextPressure, 0.85),
            )
            .with(
                VitalSign::BudgetRemaining,
                VitalSignReading::defaults_for(VitalSign::BudgetRemaining, 0.04),
            );
        let plan = HomeostasisRegulator::new().regulate(&set);
        assert_eq!(
            plan.actions(),
            vec![
                RegulationAction::ThrottleProactiveMessages,
                RegulationAction::ReduceRecallDepth,
                RegulationAction::RestoreBudget,
                RegulationAction::EscalateToHuman,
                RegulationAction::ConsolidateBeforeOverflow,
            ]
        );
        assert_eq!(plan.severity, Severity::Critical);
        assert_eq!(plan.steps[0].severity, Severity::Critical);
        assert_eq!(plan.steps[4].severity, Severity::High);
        assert_eq!(plan.steps[4].sign, Some(VitalSign::ContextPressure));
    }

    #[test]
    fn trend_extrapolation_triggers_preemptive_action() {
        let prev = set_one(VitalSign::ContextPressure, 0.55);
        let next = set_one(VitalSign::ContextPressure, 0.68);
        let plan = HomeostasisRegulator::new().regulate_with_trend(&prev, &next);
        assert_eq!(plan.severity, Severity::High);
        assert_eq!(
            plan.actions(),
            vec![
                RegulationAction::ConsolidateBeforeOverflow,
                RegulationAction::ReduceRecallDepth,
            ]
        );
        assert_eq!(plan.predicted_count(), 2);
        assert!(plan.steps.iter().all(|step| step.predicted));
        assert_eq!(
            plan.steps[0].sign,
            Some(VitalSign::ContextPressure),
            "预测步归因给升档体征"
        );
        assert_eq!(plan.steps[0].severity, Severity::High);
        assert!(plan.steps[0].reason.contains("趋势外推"));
        assert!(plan.steps[0].reason.contains("上下文压力"));
    }

    #[test]
    fn slow_trend_does_not_trigger_false_positive() {
        let regulator = HomeostasisRegulator::new();
        let prev = set_one(VitalSign::ContextPressure, 0.30);
        let next = set_one(VitalSign::ContextPressure, 0.34);
        let plan = regulator.regulate_with_trend(&prev, &next);
        assert_eq!(plan.severity, Severity::Normal);
        assert_eq!(plan.actions(), vec![RegulationAction::None]);
        assert!(plan.is_noop());
        assert_eq!(plan.predicted_count(), 0);

        // 慢速变化跨过 warn 阈值前的最后一段：0.60 → 0.62 外推 0.64，仍未越 0.65。
        let prev = set_one(VitalSign::ContextPressure, 0.60);
        let next = set_one(VitalSign::ContextPressure, 0.62);
        let plan = regulator.regulate_with_trend(&prev, &next);
        assert!(plan.is_noop());
    }

    #[test]
    fn budget_depletion_throttles_before_restoring() {
        let set = set_one(VitalSign::BudgetRemaining, 0.10);
        let plan = HomeostasisRegulator::new().regulate(&set);
        assert_eq!(
            plan.actions(),
            vec![
                RegulationAction::ThrottleProactiveMessages,
                RegulationAction::RestoreBudget,
            ]
        );
        assert!(plan
            .steps
            .iter()
            .all(|step| step.sign == Some(VitalSign::BudgetRemaining)));
        assert_eq!(plan.severity, Severity::High);
    }

    #[test]
    fn warn_level_never_escalates_to_human() {
        let plan = HomeostasisRegulator::new().regulate(&set_one(VitalSign::ContextPressure, 0.70));
        assert_eq!(plan.actions(), vec![RegulationAction::ReduceRecallDepth]);
        assert!(!plan.actions().contains(&RegulationAction::EscalateToHuman));
    }

    #[test]
    fn non_finite_inputs_are_sanitized_and_never_leak_nan() {
        let poisoned = VitalSignReading::new(
            f64::NAN,
            f64::INFINITY,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
        );
        assert_eq!(poisoned.sanitized().value, 0.0);
        assert_eq!(poisoned.sanitized().setpoint, 1.0);
        assert_eq!(poisoned.sanitized().warn, 0.0);
        // 非有限阈值 = 该档不可用（跳过）→ 上下文压力不因配置缺失报警。
        assert!(!VitalSign::ContextPressure.crosses(1.0, f64::NAN));
        assert!(!VitalSign::ContextPressure.crosses(1.0, f64::INFINITY));
        assert_eq!(
            poisoned.severity_for(VitalSign::ContextPressure),
            Severity::Normal
        );

        let plan = HomeostasisRegulator::new()
            .regulate(&VitalSignSet::healthy().with(VitalSign::ContextPressure, poisoned));
        assert!(!plan.summary.contains("NaN"));
        assert!(!plan.summary.contains("inf"));
        assert!(plan.steps.iter().all(|step| !step.reason.contains("NaN")));

        // 预算余额 NaN → 0.0（紧张侧）→ Critical（保守触发）。
        let budget_nan = VitalSignReading::new(f64::NAN, 0.80, 0.30, 0.15, 0.05);
        assert_eq!(
            budget_nan.severity_for(VitalSign::BudgetRemaining),
            Severity::Critical
        );
        let plan =
            HomeostasisRegulator::new().regulate(&set_one(VitalSign::BudgetRemaining, f64::NAN));
        assert!(plan.actions().contains(&RegulationAction::RestoreBudget));
        assert!(plan.actions().contains(&RegulationAction::EscalateToHuman));
        assert_eq!(plan.severity, Severity::Critical);
    }

    #[test]
    fn regulation_is_deterministic_across_calls() {
        let regulator = HomeostasisRegulator::new();
        let current = set_one(VitalSign::MemoryLoad, 0.90);
        let prev = set_one(VitalSign::MemoryLoad, 0.80);
        assert_eq!(regulator.regulate(&current), regulator.regulate(&current));
        assert_eq!(
            regulator.regulate_with_trend(&prev, &current),
            regulator.regulate_with_trend(&prev, &current)
        );
    }

    #[test]
    fn default_thresholds_are_ordered_per_direction() {
        for sign in VitalSign::ALL {
            let reading = VitalSignReading::defaults_for(sign, 0.5);
            if sign.higher_is_worse() {
                assert!(
                    reading.warn < reading.high && reading.high < reading.critical,
                    "{sign:?} 阈值必须 warn < high < critical"
                );
            } else {
                assert!(
                    reading.warn > reading.high && reading.high > reading.critical,
                    "{sign:?} 阈值必须 warn > high > critical"
                );
            }
            assert!((0.0..=1.0).contains(&reading.setpoint));
        }
    }

    #[test]
    fn allostatic_load_accumulates_saturates_and_recovers() {
        let regulator = HomeostasisRegulator::new();
        let critical = regulator.regulate(&set_one(VitalSign::ContextPressure, 0.99));
        let healthy = regulator.regulate(&VitalSignSet::healthy());
        let mut load = AllostaticLoad::new();
        assert_eq!(load.level(), 0.0);
        assert!(!load.is_chronic());

        let after_first = load.observe(&critical, 10_000);
        assert!((after_first - 0.2).abs() < 1e-12);
        let after_second = load.observe(&critical, 10_000);
        assert!((after_second - 0.4).abs() < 1e-12);

        let recovered = load.observe(&healthy, 10_000);
        assert!((recovered - 0.35).abs() < 1e-12);

        // Warn 档按文档保持，不加码也不恢复。
        let warn = regulator.regulate(&set_one(VitalSign::ContextPressure, 0.70));
        let held = load.observe(&warn, 60_000);
        assert!((held - recovered).abs() < 1e-12);

        load.observe(&critical, 600_000);
        assert_eq!(load.level(), 1.0);
        assert!(load.is_chronic());
    }
}
