//! quorum —— 群体感应升级门内核（生物机制族 W1 批，2026-10-19；IMPLEMENTED，未接线）。
//!
//! # 生物原型
//!
//! 细菌并不随时"开会"：它们持续分泌小分子信号（autoinducer），信号随距离扩散、
//! 随时间衰减；只有局部浓度越过**阈值**时，整个菌群才同步表达某项行为（生物膜、
//! 毒素、群集运动）。集体行动之后还有一段**不应期**（refractory period）：此间
//! 即使浓度仍高也不重复触发，避免把同一波信号反复兑现成行动。
//!
//! 本模块把这套机制缩成一个纯内核：
//!
//! - 信号（[`QuorumSignal`]）按类型带默认权重，表示"这条证据有多重"；
//! - 浓度随流逝时间线性衰减（[`QuorumConfig::decay_per_second`]），旧证据自然
//!   稀释，会议理由由**当下**仍站得住的信号支撑；
//! - 浓度累加有上限（[`QuorumConfig::max_accumulation`]），单波狂刷不能把分数
//!   推到无意义的量级；
//! - 只有 `score >= threshold` 才升议；升议被 [`QuorumAccumulator::consume`]
//!   兑现后立即清零并进入冷却（[`QuorumConfig::cooldown_ms`]），保证同一波信号
//!   只升议一次，不刷屏。
//!
//! # 工程边界（0 装诚实口径）
//!
//! - **IMPLEMENTED，未接线**：不读时钟、不做 I/O、不 spawn、不持有全局状态；
//!   时间一律由调用方以 `u64` 毫秒注入（同 crate 的 `self_tuning` / `task_queue`
//!   惯例），因此可重放、可确定性测试。
//! - **未接 council**：本模块不调用 `Council`、不触发任何 subagent 审议；也
//!   **不写自述节**（不生成模型侧自我描述、不产生面向用户的说明）。是否开会仍由
//!   现有硬编码/语义规则决定，本内核只把"为什么这次开会、上次不开"变成一笔可
//!   解释的浓度账。
//! - **不新增依赖**：只用 `serde` 序列化配置与升议记录；不用 `chrono`、不引入
//!   时钟抽象、不落盘。
//! - **quorum 只是升级门的浓度模型，不替代审议本身**：它只回答"证据够不够启动
//!   一次集体审议"，不回答"审议该得出什么结论"，也不改任何治理 / 审计判定。
//!
//! # 确定性与不变量
//!
//! 分数是「信号列表 + 注入时间」的纯函数：按插入顺序求和，逐信号线性衰减后截到
//! `[0, max_accumulation]`。所有输入在入口做有限性消毒（非有限 / 非正权重直接
//! 丢弃，非法配置回退默认值），因此分数永不为 NaN、永不越界；排序使用全序比较
//! （当前权重降序 → 时间升序 → 来源升序 → 类型声明序），同分同序稳定。
//! 时间倒流（`now_ms` 早于信号发生时刻）按"零流逝"处理，不产生负衰减。

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

/// 默认升议阈值：累计浓度达到 `1.0` 才启动一次集体审议。
///
/// 取 1.0 是让"单条中等信号不足以开会、一小簇同向信号刚好组成一次行动"：
/// 默认权重下约 2 条风险升级、2 条用户不信任、或 3~5 条轻信号才够一档。
pub const DEFAULT_THRESHOLD: f32 = 1.0;

/// 默认衰减速度：每秒 `0.05`（线性）。
///
/// 单条 0.2~0.6 权重的信号在 4~12 秒内显著稀释，跨分钟的信号基本归零；既允许
/// 一次事件风暴内的信号叠加，又不会让昨天的异常替今天的会议投票。
pub const DEFAULT_DECAY_PER_SECOND: f32 = 0.05;

/// 默认冷却（不应期）：10 分钟（600 000 毫秒）。
///
/// 一次升议兑现后 10 分钟内不再升议：同一波问题的后续余震不应反复召集智囊团，
/// 也给刚开完的会议留出消化结论的时间。
pub const DEFAULT_COOLDOWN_MS: u64 = 600_000;

/// 默认浓度上限：`2.0`（阈值的两倍）。
///
/// 留出高于阈值的两档余量用于信号排序（"这波有多严重"），同时给分数一个上界，
/// 防止连续注入把 f32 分数推到失去意义的量级。
pub const DEFAULT_MAX_ACCUMULATION: f32 = 2.0;

/// 一次升议快照最多携带的信号条数（`Escalate` 的 `top_signals` 与
/// [`QuorumEscalation::reasons`] 同源同长），使解释文本有界、不随风暴线性膨胀。
pub const TOP_SIGNALS_LIMIT: usize = 5;

/// 单条理由里来源字段最多保留的字符数，超出以 `…` 收尾（只截理由，不截信号本身）。
const MAX_REASON_SOURCE_CHARS: usize = 48;

/// 信号类别：不同来源的证据类型，各自带默认权重。
///
/// 权重表达"单条证据有多重"，不是概率；同类型信号的默认权重相同，调用方可用
/// [`QuorumSignal::weighted`] 覆盖。严重度次序（轻 → 重）为
/// `Uncertainty < Anomaly < Conflict < UserDistrust < RiskEscalation`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuorumSignalKind {
    /// 异常：单点观测偏离预期（工具报错、指标抖动），最轻的一类证据。
    Anomaly,
    /// 冲突：多源结论互相矛盾（评审与实现不一致、两条证据对不上）。
    Conflict,
    /// 不确定：模型或规则自己拿不准，单条不足以惊动集体（默认最低权重）。
    Uncertainty,
    /// 风险升级：风险等级被显式抬升（安全 / 权限 / 不可逆操作），默认最重。
    RiskEscalation,
    /// 用户不信任：用户显式表达怀疑或不信任，关系面证据，权重仅次于风险升级。
    UserDistrust,
}

impl QuorumSignalKind {
    /// 该类信号在未衰减时的默认满权重。
    ///
    /// 取值理由（阈值 1.0 下）：`RiskEscalation` 0.6（两条即够：1.2）、
    /// `UserDistrust` 0.5（两条）、`Conflict` 0.4、`Anomaly` 0.3、
    /// `Uncertainty` 0.2（五条）——越重的证据越少条数即可触发，但不允许单条
    /// 轻信号独自开会。
    pub const fn default_weight(self) -> f32 {
        match self {
            Self::Anomaly => 0.3,
            Self::Conflict => 0.4,
            Self::Uncertainty => 0.2,
            Self::RiskEscalation => 0.6,
            Self::UserDistrust => 0.5,
        }
    }

    /// 稳定的 snake_case 名称，用于理由文本与序列化键（与 serde 重命名一致）。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Anomaly => "anomaly",
            Self::Conflict => "conflict",
            Self::Uncertainty => "uncertainty",
            Self::RiskEscalation => "risk_escalation",
            Self::UserDistrust => "user_distrust",
        }
    }
}

/// 一条群体感应信号：类型、权重、发生时刻、来源。
///
/// 字段公开以便调用方原样记录/回放；权重在 [`QuorumAccumulator::add`] 入口被
/// 消毒，时刻倒流按零流逝处理，非法值不会污染浓度账。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuorumSignal {
    /// 信号类别（决定默认权重与理由标签）。
    pub kind: QuorumSignalKind,
    /// 该信号在发生时刻的满权重；实际贡献按流逝时间衰减。
    pub weight: f32,
    /// 信号发生时刻（毫秒，由调用方注入的单调时钟）。
    pub at_ms: u64,
    /// 来源标识（模块名 / 角色名 / 会话 id 等自由文本，仅用于解释）。
    pub source: String,
}

impl QuorumSignal {
    /// 用该类别的默认权重构造信号。
    pub fn new(kind: QuorumSignalKind, at_ms: u64, source: impl Into<String>) -> Self {
        Self {
            kind,
            weight: kind.default_weight(),
            at_ms,
            source: source.into(),
        }
    }

    /// 用显式权重构造信号（覆盖类别默认值，例如按观测强度缩放）。
    pub fn weighted(
        kind: QuorumSignalKind,
        weight: f32,
        at_ms: u64,
        source: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            weight,
            at_ms,
            source: source.into(),
        }
    }

    /// 该信号在 `now_ms` 的当前有效权重（线性衰减，下限 0）。
    ///
    /// - `now_ms` 早于 `at_ms`（时间倒流 / 未来信号）按零流逝处理，返回满权重；
    /// - `weight` 非有限或非正、`decay_per_second` 非有限或非正时按 0 处理，
    ///   因此结果总是有限且落在 `[0, weight]`。
    pub fn decayed_weight(&self, now_ms: u64, decay_per_second: f32) -> f32 {
        let weight = if self.weight.is_finite() && self.weight > 0.0 {
            self.weight
        } else {
            0.0
        };
        let decay = if decay_per_second.is_finite() && decay_per_second > 0.0 {
            decay_per_second
        } else {
            0.0
        };
        let elapsed_seconds = now_ms.saturating_sub(self.at_ms) as f32 / 1_000.0;
        (weight - decay * elapsed_seconds).max(0.0)
    }
}

/// 群体感应升级门的配置。所有字段公开，可由配置文件直接反序列化。
///
/// 非法值在 [`QuorumAccumulator::new`] 入口回退默认值（见下文规范化规则），
/// 而不是让 NaN / 负衰减流进浓度账：
///
/// - `threshold` 必须有限且大于 0，否则回退 [`DEFAULT_THRESHOLD`]（阈值 0 会让
///   空仓也升议，属于配置错误，按 fail-closed 方向收拾）；
/// - `decay_per_second` 必须有限且不小于 0，否则回退
///   [`DEFAULT_DECAY_PER_SECOND`]；
/// - `max_accumulation` 必须有限且不小于 `threshold`，否则回退
///   [`DEFAULT_MAX_ACCUMULATION`] 并抬到不低于 `threshold`（上限低于阈值意味着
///   闸门永不开启，配置错误不应被静默放大）；
/// - `cooldown_ms` 是 `u64`，0 表示无不应期，合法。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct QuorumConfig {
    /// 升议阈值：浓度 `>= threshold` 才升议。
    pub threshold: f32,
    /// 每秒线性衰减量（0 表示不衰减，仅测试 / 特殊场景使用）。
    pub decay_per_second: f32,
    /// 升议兑现后的不应期长度（毫秒），0 表示无冷却。
    pub cooldown_ms: u64,
    /// 浓度上限：累加与 `score` 都截到该值。
    pub max_accumulation: f32,
}

impl Default for QuorumConfig {
    fn default() -> Self {
        Self {
            threshold: DEFAULT_THRESHOLD,
            decay_per_second: DEFAULT_DECAY_PER_SECOND,
            cooldown_ms: DEFAULT_COOLDOWN_MS,
            max_accumulation: DEFAULT_MAX_ACCUMULATION,
        }
    }
}

impl QuorumConfig {
    /// 把配置收拾成可计算的有效值：有限性检查 + 上限不低于阈值。
    ///
    /// 这是 [`QuorumAccumulator::new`] 唯一的入口消毒点；调用方拿到的
    /// [`QuorumAccumulator::config`] 永远是已规范化的配置。
    fn normalized(self) -> Self {
        let threshold = if self.threshold.is_finite() && self.threshold > 0.0 {
            self.threshold
        } else {
            DEFAULT_THRESHOLD
        };
        let decay_per_second = if self.decay_per_second.is_finite() && self.decay_per_second >= 0.0
        {
            self.decay_per_second
        } else {
            DEFAULT_DECAY_PER_SECOND
        };
        let max_accumulation =
            if self.max_accumulation.is_finite() && self.max_accumulation >= threshold {
                self.max_accumulation
            } else {
                DEFAULT_MAX_ACCUMULATION.max(threshold)
            };
        Self {
            threshold,
            decay_per_second,
            cooldown_ms: self.cooldown_ms,
            max_accumulation,
        }
    }
}

/// 升级门对外的三态判定。
///
/// `Hold` / `Cooldown` 是两种"这次不开会"：前者是浓度不够（并给出还差多少），
/// 后者是不应期（并给出何时解禁）。判定本身无副作用，只有
/// [`QuorumAccumulator::consume`] 会改变状态。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuorumDecision {
    /// 达到阈值、不在冷却、未被消费：够格升议。
    Escalate {
        /// 当前浓度（已截到上限）。
        score: f32,
        /// 贡献最大的若干信号，权重为当前有效权重（降序、稳定同分序）。
        top_signals: Vec<QuorumSignal>,
    },
    /// 未达阈值：给出当前浓度与还差多少。
    Hold {
        /// 当前浓度。
        score: f32,
        /// 距离阈值还差多少（`threshold - score`，恒为正）。
        need: f32,
    },
    /// 处于升议后的不应期：给出解禁时刻与当前浓度。
    Cooldown {
        /// 冷却结束时刻（毫秒）；`now_ms >= until_ms` 即解禁。
        until_ms: u64,
        /// 当前浓度（冷却期仍继续记账，只是不升议）。
        score: f32,
    },
}

/// 一次被兑现的升议：供记录 / 日志 / 后续接线解释"为什么这次开会"。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuorumEscalation {
    /// 兑现时的浓度（已截到上限）。
    pub score: f32,
    /// 有界理由列表：每条对应一个 top signal，稳定机器可读格式
    /// `kind weight=x.xxx source=... at_ms=...`（来源超长时截断加 `…`）。
    pub reasons: Vec<String>,
    /// 兑现时刻（毫秒，等于 `consume(now_ms)` 的入参）。
    pub triggered_at_ms: u64,
}

/// 群体感应累加器：信号 → 衰减 → 累加 → 阈值 / 冷却判定。
///
/// 不持有系统时间：所有方法都要求调用方传入 `now_ms`，因此同一串输入可在测试与
/// 重放中逐位复现。内部只保留信号列表（原始权重与发生时刻），浓度是纯函数。
///
/// 状态机只有一处迁移：`consume` 在判定为 `Escalate` 时清空信号并写入冷却起点。
/// 因此"同一波信号只升议一次"由清零 + 冷却共同保证，而不是靠调用方自觉。
#[derive(Debug, Clone)]
pub struct QuorumAccumulator {
    config: QuorumConfig,
    signals: Vec<QuorumSignal>,
    cooldown_until_ms: Option<u64>,
}

impl QuorumAccumulator {
    /// 用给定配置构造累加器；配置先经 `QuorumConfig` 的规范化规则消毒。
    pub fn new(config: QuorumConfig) -> Self {
        Self {
            config: config.normalized(),
            signals: Vec::new(),
            cooldown_until_ms: None,
        }
    }

    /// 已规范化的配置（只读）。
    pub fn config(&self) -> &QuorumConfig {
        &self.config
    }

    /// 当前保留的原始信号（插入顺序）。
    ///
    /// 已衰减到 0 的信号会在下一次 [`Self::add`] 时被清理，因此本视图在"只读
    /// 时间推进"之后仍可能包含零权重条目；有效权重请按 `now_ms` 重新计算。
    pub fn signals(&self) -> &[QuorumSignal] {
        &self.signals
    }

    /// 注入一条信号：先把已有信号按 `now_ms` 衰减（并清理已归零的），再累加。
    ///
    /// 非有限或非正的 `weight` 会被整条丢弃（不记、不报错），保证浓度账无 NaN；
    /// 时刻不单调（`now_ms` 回退、或信号 `at_ms` 在未来）按零流逝处理，不产生
    /// 负衰减。
    ///
    /// 保留条目只由衰减清理、没有容量上限：注入速率由接线方约束（本内核不吞掉
    /// 任何有效证据，也不替调用方做采样决策）。
    pub fn add(&mut self, signal: QuorumSignal, now_ms: u64) {
        if !signal.weight.is_finite() || signal.weight <= 0.0 {
            return;
        }
        let decay = self.config.decay_per_second;
        self.signals
            .retain(|existing| existing.decayed_weight(now_ms, decay) > 0.0);
        self.signals.push(signal);
    }

    /// 当前浓度：逐信号线性衰减后求和，再截到 `[0, max_accumulation]`。
    ///
    /// 纯查询，不改变状态；同一 `now_ms` 重复调用结果逐位相同。
    pub fn score(&self, now_ms: u64) -> f32 {
        let decay = self.config.decay_per_second;
        let total = self.signals.iter().fold(0.0_f32, |sum, signal| {
            sum + signal.decayed_weight(now_ms, decay)
        });
        total.clamp(0.0, self.config.max_accumulation)
    }

    /// 升级门判定（无副作用）。
    ///
    /// 优先级：冷却（不应期）优先于浓度——冷却期内即使浓度已过阈值也只报
    /// [`QuorumDecision::Cooldown`]，不报升议；冷却结束后浓度仍过阈值则恢复
    /// [`QuorumDecision::Escalate`]。
    pub fn should_escalate(&self, now_ms: u64) -> QuorumDecision {
        let score = self.score(now_ms);
        if let Some(until_ms) = self.cooldown_until_ms {
            if now_ms < until_ms {
                return QuorumDecision::Cooldown { until_ms, score };
            }
        }
        if score >= self.config.threshold {
            QuorumDecision::Escalate {
                score,
                top_signals: self.top_signals(now_ms),
            }
        } else {
            QuorumDecision::Hold {
                score,
                need: self.config.threshold - score,
            }
        }
    }

    /// 兑现一次升议：仅当 [`Self::should_escalate`] 返回 `Escalate` 时生效。
    ///
    /// 生效时一次性完成三件事：清空已消费的信号（同一波信号不再升值）、把冷却
    /// 起点设为 `now_ms`（不应期到 `now_ms + cooldown_ms`）、返回到期快照。
    /// 其余情况（未达阈值 / 冷却中）一律返回 `None`，因此对同一波信号连续调用
    /// 第二次必然是 `None`（幂等）。
    pub fn consume(&mut self, now_ms: u64) -> Option<QuorumEscalation> {
        let QuorumDecision::Escalate { score, top_signals } = self.should_escalate(now_ms) else {
            return None;
        };
        self.signals.clear();
        self.cooldown_until_ms = Some(now_ms.saturating_add(self.config.cooldown_ms));
        Some(QuorumEscalation {
            score,
            reasons: top_signals.iter().map(signal_reason).collect(),
            triggered_at_ms: now_ms,
        })
    }

    /// 最近一次升议的冷却解禁时刻（从未升议时为 `None`），供观测面解释。
    pub fn cooldown_until_ms(&self) -> Option<u64> {
        self.cooldown_until_ms
    }

    /// 取当前有效权重最大的若干信号，权重替换为衰减后的值。
    ///
    /// 排序键（全序、稳定）：有效权重降序 → `at_ms` 升序 → `source` 升序 →
    /// 类型声明序。最多 [`TOP_SIGNALS_LIMIT`] 条；权重为各信号自身有效权重，
    /// 不按浓度上限截断（便于看清"哪条证据最重"）。
    fn top_signals(&self, now_ms: u64) -> Vec<QuorumSignal> {
        let decay = self.config.decay_per_second;
        let mut ranked = self
            .signals
            .iter()
            .map(|signal| (signal.decayed_weight(now_ms, decay), signal))
            .filter(|(weight, _)| *weight > 0.0)
            .collect::<Vec<_>>();
        ranked.sort_by(|(left_weight, left), (right_weight, right)| {
            right_weight
                .partial_cmp(left_weight)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.at_ms.cmp(&right.at_ms))
                .then_with(|| left.source.cmp(&right.source))
                .then_with(|| left.kind.cmp(&right.kind))
        });
        ranked
            .into_iter()
            .take(TOP_SIGNALS_LIMIT)
            .map(|(weight, signal)| QuorumSignal {
                kind: signal.kind,
                weight,
                at_ms: signal.at_ms,
                source: signal.source.clone(),
            })
            .collect()
    }
}

impl Default for QuorumAccumulator {
    fn default() -> Self {
        Self::new(QuorumConfig::default())
    }
}

/// 把一条信号渲染成稳定、有界的理由行。
///
/// 格式固定为 `kind weight=x.xxx source=... at_ms=...`；来源只用于解释，超长时
/// 截到 [`MAX_REASON_SOURCE_CHARS`] 个字符并以 `…` 标注，避免日志被单条来源刷爆。
fn signal_reason(signal: &QuorumSignal) -> String {
    let mut source = signal
        .source
        .chars()
        .take(MAX_REASON_SOURCE_CHARS)
        .collect::<String>();
    if signal.source.chars().nth(MAX_REASON_SOURCE_CHARS).is_some() {
        source.push('…');
    }
    format!(
        "{} weight={:.3} source={} at_ms={}",
        signal.kind.as_str(),
        signal.weight,
        source,
        signal.at_ms
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signal(kind: QuorumSignalKind, weight: f32, at_ms: u64, source: &str) -> QuorumSignal {
        QuorumSignal::weighted(kind, weight, at_ms, source)
    }

    fn config(threshold: f32, decay_per_second: f32, cooldown_ms: u64, max: f32) -> QuorumConfig {
        QuorumConfig {
            threshold,
            decay_per_second,
            cooldown_ms,
            max_accumulation: max,
        }
    }

    #[test]
    fn signal_kind_default_weights_are_distinct_and_ordered_by_severity() {
        let kinds = [
            QuorumSignalKind::Anomaly,
            QuorumSignalKind::Conflict,
            QuorumSignalKind::Uncertainty,
            QuorumSignalKind::RiskEscalation,
            QuorumSignalKind::UserDistrust,
        ];
        let weights = kinds
            .iter()
            .map(|kind| kind.default_weight())
            .collect::<Vec<_>>();
        for weight in &weights {
            assert!(
                weight.is_finite() && *weight > 0.0 && *weight < 1.0,
                "default weight must be a finite fraction, got {weight}"
            );
        }
        let mut distinct = weights.clone();
        distinct.sort_by(|left, right| left.partial_cmp(right).unwrap_or(Ordering::Equal));
        distinct.dedup();
        assert_eq!(distinct.len(), kinds.len(), "weights must be distinct");
        assert!(
            QuorumSignalKind::Uncertainty.default_weight()
                < QuorumSignalKind::Anomaly.default_weight()
        );
        assert!(
            QuorumSignalKind::Anomaly.default_weight()
                < QuorumSignalKind::Conflict.default_weight()
        );
        assert!(
            QuorumSignalKind::Conflict.default_weight()
                < QuorumSignalKind::UserDistrust.default_weight()
        );
        assert!(
            QuorumSignalKind::UserDistrust.default_weight()
                < QuorumSignalKind::RiskEscalation.default_weight()
        );
    }

    #[test]
    fn hold_reports_exact_shortfall_and_exact_threshold_escalates() {
        let mut accumulator = QuorumAccumulator::new(config(1.0, 0.0, 0, 10.0));
        accumulator.add(signal(QuorumSignalKind::Anomaly, 0.5, 0, "watchdog"), 0);
        accumulator.add(signal(QuorumSignalKind::Conflict, 0.25, 0, "reviewer"), 0);
        match accumulator.should_escalate(0) {
            QuorumDecision::Hold { score, need } => {
                assert_eq!(score, 0.75);
                assert_eq!(need, 0.25);
            }
            other => panic!("expected Hold, got {other:?}"),
        }
        accumulator.add(signal(QuorumSignalKind::Uncertainty, 0.25, 0, "reader"), 0);
        assert_eq!(accumulator.score(0), 1.0);
        match accumulator.should_escalate(0) {
            QuorumDecision::Escalate { score, top_signals } => {
                assert_eq!(score, 1.0);
                assert_eq!(top_signals.len(), 3);
            }
            other => panic!("expected Escalate exactly at the threshold, got {other:?}"),
        }
    }

    #[test]
    fn decay_is_linear_in_elapsed_seconds_and_floors_at_zero() {
        let mut accumulator = QuorumAccumulator::new(config(1.0, 0.125, 0, 2.0));
        accumulator.add(signal(QuorumSignalKind::Anomaly, 0.5, 0, "watchdog"), 0);
        assert_eq!(accumulator.score(0), 0.5);
        assert_eq!(accumulator.score(1_000), 0.375);
        assert_eq!(accumulator.score(2_000), 0.25);
        assert_eq!(accumulator.score(4_000), 0.0);
        assert_eq!(accumulator.score(60_000), 0.0);
    }

    #[test]
    fn adding_a_signal_first_decays_the_existing_pond() {
        let mut accumulator = QuorumAccumulator::new(config(1.0, 0.125, 0, 4.0));
        accumulator.add(signal(QuorumSignalKind::Anomaly, 0.5, 0, "first"), 0);
        accumulator.add(
            signal(QuorumSignalKind::Conflict, 0.25, 2_000, "second"),
            2_000,
        );
        // 旧信号 0.5 - 0.125 * 2 = 0.25，再加新信号 0.25。
        assert_eq!(accumulator.score(2_000), 0.5);
        assert_eq!(accumulator.signals().len(), 2);
    }

    #[test]
    fn accumulation_clamps_at_the_configured_ceiling() {
        let mut accumulator = QuorumAccumulator::new(config(1.0, 0.0, 0, 1.5));
        accumulator.add(
            signal(QuorumSignalKind::RiskEscalation, 0.6, 0, "risk-a"),
            0,
        );
        accumulator.add(
            signal(QuorumSignalKind::RiskEscalation, 0.6, 0, "risk-b"),
            0,
        );
        accumulator.add(signal(QuorumSignalKind::UserDistrust, 0.5, 0, "user"), 0);
        assert_eq!(accumulator.score(0), 1.5);
        match accumulator.should_escalate(0) {
            QuorumDecision::Escalate { score, top_signals } => {
                assert_eq!(score, 1.5);
                // 排名仍按各信号自身有效权重，不因总分截断而失真。
                assert_eq!(top_signals[0].weight, 0.6);
                assert_eq!(top_signals[1].weight, 0.6);
            }
            other => panic!("expected clamped Escalate, got {other:?}"),
        }
    }

    #[test]
    fn cooldown_blocks_reescalation_and_reports_until() {
        let mut accumulator = QuorumAccumulator::new(config(1.0, 0.0, 10_000, 2.0));
        accumulator.add(signal(QuorumSignalKind::Anomaly, 1.0, 0, "watchdog"), 0);
        let escalation = accumulator.consume(0).expect("first wave escalates");
        assert_eq!(escalation.score, 1.0);
        assert_eq!(escalation.triggered_at_ms, 0);
        assert_eq!(escalation.reasons.len(), 1);
        assert!(escalation.reasons[0].starts_with("anomaly weight=1.000 source=watchdog"));
        assert_eq!(accumulator.cooldown_until_ms(), Some(10_000));
        assert_eq!(accumulator.score(0), 0.0, "consumed signals are cleared");

        accumulator.add(
            signal(QuorumSignalKind::Conflict, 1.0, 1_000, "reviewer"),
            1_000,
        );
        match accumulator.should_escalate(1_000) {
            QuorumDecision::Cooldown { until_ms, score } => {
                assert_eq!(until_ms, 10_000);
                assert_eq!(score, 1.0);
            }
            other => panic!("expected Cooldown despite high score, got {other:?}"),
        }
        assert!(
            accumulator.consume(1_000).is_none(),
            "cooldown must not be consumed twice"
        );
        assert!(matches!(
            accumulator.should_escalate(9_999),
            QuorumDecision::Cooldown {
                until_ms: 10_000,
                ..
            }
        ));
    }

    #[test]
    fn cooldown_expiry_reopens_the_gate_for_the_next_wave() {
        let mut accumulator = QuorumAccumulator::new(config(1.0, 0.0, 10_000, 2.0));
        accumulator.add(signal(QuorumSignalKind::Anomaly, 1.0, 0, "watchdog"), 0);
        assert!(accumulator.consume(0).is_some());
        accumulator.add(
            signal(QuorumSignalKind::Conflict, 1.0, 1_000, "reviewer"),
            1_000,
        );

        // 边界：解禁时刻本身（now == until）即恢复升议。
        match accumulator.should_escalate(10_000) {
            QuorumDecision::Escalate { score, .. } => assert_eq!(score, 1.0),
            other => panic!("expected Escalate at cooldown expiry, got {other:?}"),
        }
        let second = accumulator.consume(10_000).expect("second wave escalates");
        assert_eq!(second.triggered_at_ms, 10_000);
        assert_eq!(accumulator.cooldown_until_ms(), Some(20_000));
        assert!(accumulator.consume(10_000).is_none());
    }

    #[test]
    fn consume_is_idempotent_within_one_wave() {
        let mut accumulator = QuorumAccumulator::new(config(1.0, 0.0, 5_000, 2.0));
        accumulator.add(signal(QuorumSignalKind::RiskEscalation, 1.0, 0, "risk"), 0);
        assert!(accumulator.consume(0).is_some());
        assert!(accumulator.consume(0).is_none());
        assert!(accumulator.consume(4_999).is_none());

        let mut quiet = QuorumAccumulator::new(config(1.0, 0.0, 5_000, 2.0));
        quiet.add(signal(QuorumSignalKind::Uncertainty, 0.2, 0, "reader"), 0);
        assert!(
            quiet.consume(0).is_none(),
            "below threshold never escalates"
        );
    }

    #[test]
    fn top_signals_rank_by_current_weight_with_stable_ties() {
        let mut accumulator = QuorumAccumulator::new(config(1.0, 0.125, 0, 10.0));
        accumulator.add(signal(QuorumSignalKind::Anomaly, 0.5, 0, "beta"), 0);
        accumulator.add(signal(QuorumSignalKind::Conflict, 0.5, 0, "alpha"), 0);
        accumulator.add(
            signal(QuorumSignalKind::RiskEscalation, 0.75, 500, "gamma"),
            500,
        );
        accumulator.add(signal(QuorumSignalKind::Uncertainty, 0.5, 0, "alpha"), 0);
        match accumulator.should_escalate(1_000) {
            QuorumDecision::Escalate { top_signals, .. } => {
                let kinds = top_signals
                    .iter()
                    .map(|signal| signal.kind)
                    .collect::<Vec<_>>();
                // 0.6875 > 0.375（三条同分：时间同 → 来源 alpha 先 → 类型声明序）。
                assert_eq!(
                    kinds,
                    vec![
                        QuorumSignalKind::RiskEscalation,
                        QuorumSignalKind::Conflict,
                        QuorumSignalKind::Uncertainty,
                        QuorumSignalKind::Anomaly,
                    ]
                );
                assert_eq!(top_signals[0].weight, 0.6875);
                assert_eq!(top_signals[1].weight, 0.375);
                assert_eq!(top_signals[3].source, "beta");
            }
            other => panic!("expected Escalate for ranking snapshot, got {other:?}"),
        }
    }

    #[test]
    fn top_signals_snapshot_is_bounded() {
        let mut accumulator = QuorumAccumulator::new(config(0.5, 0.0, 0, 10.0));
        for index in 0..(TOP_SIGNALS_LIMIT + 3) {
            accumulator.add(
                signal(QuorumSignalKind::Anomaly, 0.1, index as u64, "burst"),
                index as u64,
            );
        }
        match accumulator.should_escalate(10) {
            QuorumDecision::Escalate { top_signals, .. } => {
                assert_eq!(top_signals.len(), TOP_SIGNALS_LIMIT);
            }
            other => panic!("expected Escalate with bounded top signals, got {other:?}"),
        }
    }

    #[test]
    fn identical_inputs_produce_identical_decisions_and_reasons() {
        let replay = || {
            let mut accumulator = QuorumAccumulator::new(config(0.9, 0.05, 2_000, 2.0));
            accumulator.add(
                QuorumSignal::new(QuorumSignalKind::Anomaly, 0, "sensor-a"),
                0,
            );
            accumulator.add(
                QuorumSignal::new(QuorumSignalKind::UserDistrust, 0, "user"),
                0,
            );
            let first = accumulator.should_escalate(100);
            let escalation = accumulator.consume(100);
            let second = accumulator.should_escalate(500);
            (first, escalation, second)
        };
        assert_eq!(replay(), replay());
    }

    #[test]
    fn invalid_config_falls_back_to_documented_defaults() {
        let accumulator = QuorumAccumulator::new(QuorumConfig {
            threshold: 0.0,
            decay_per_second: f32::NAN,
            cooldown_ms: 42,
            max_accumulation: 0.25,
        });
        assert_eq!(accumulator.config().threshold, DEFAULT_THRESHOLD);
        assert_eq!(
            accumulator.config().decay_per_second,
            DEFAULT_DECAY_PER_SECOND
        );
        assert_eq!(
            accumulator.config().max_accumulation,
            DEFAULT_MAX_ACCUMULATION
        );
        assert_eq!(accumulator.config().cooldown_ms, 42);
    }

    #[test]
    fn hostile_inputs_cannot_produce_nan_or_out_of_range_scores() {
        let mut accumulator = QuorumAccumulator::new(QuorumConfig {
            threshold: f32::NAN,
            decay_per_second: -1.0,
            cooldown_ms: 0,
            max_accumulation: f32::INFINITY,
        });
        accumulator.add(signal(QuorumSignalKind::Anomaly, f32::NAN, 0, "nan"), 0);
        accumulator.add(
            signal(QuorumSignalKind::Conflict, f32::INFINITY, 0, "inf"),
            0,
        );
        accumulator.add(
            signal(QuorumSignalKind::Uncertainty, -0.5, 0, "negative"),
            0,
        );
        assert_eq!(accumulator.score(0), 0.0);
        assert_eq!(accumulator.signals().len(), 0);

        // 未来信号按零流逝计满权重；随后时间倒流也不产生负衰减或 NaN。
        accumulator.add(
            signal(QuorumSignalKind::RiskEscalation, 0.6, 10_000, "future"),
            0,
        );
        assert_eq!(accumulator.score(0), 0.6);
        accumulator.add(signal(QuorumSignalKind::Conflict, 0.3, 0, "past"), 0);
        let score = accumulator.score(0);
        assert!(score.is_finite());
        assert!((0.0..=accumulator.config().max_accumulation).contains(&score));
        match accumulator.should_escalate(0) {
            QuorumDecision::Hold { score: held, need } => {
                assert!(held.is_finite() && need.is_finite() && need > 0.0);
            }
            other => panic!("expected Hold below the sanitized threshold, got {other:?}"),
        }
    }

    #[test]
    fn serde_roundtrip_keeps_config_signal_and_escalation() {
        let config = QuorumConfig::default();
        let signal = QuorumSignal::new(QuorumSignalKind::RiskEscalation, 7, "risk");
        let escalation = QuorumEscalation {
            score: 1.25,
            reasons: vec!["risk_escalation weight=0.600 source=risk at_ms=7".into()],
            triggered_at_ms: 9,
        };
        let decoded_config: QuorumConfig =
            serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        let decoded_signal: QuorumSignal =
            serde_json::from_str(&serde_json::to_string(&signal).unwrap()).unwrap();
        let decoded_escalation: QuorumEscalation =
            serde_json::from_str(&serde_json::to_string(&escalation).unwrap()).unwrap();
        assert_eq!(decoded_config, config);
        assert_eq!(decoded_signal, signal);
        assert_eq!(decoded_escalation, escalation);
    }
}
