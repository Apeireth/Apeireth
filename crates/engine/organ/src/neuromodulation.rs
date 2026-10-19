//! neuromodulation —— 神经调质慢变量内核 v0（生物机制族 W1 批 SG-13, 机制内核, 未接线）。
//!
//! **状态：IMPLEMENTED（库级机制内核）/ 未接线。** 本模块只提供确定性的数据结构与纯计算：
//! `lib.rs` 已登记 `pub mod neuromodulation;`，但**没有** self_status 节、没有接 persona /
//! 语调合成、没有改 runtime 装配、没有持久化与遥测。谁消费 [`BehaviorParams`]，由 W2 接线批
//! 在模块之外决定。
//!
//! # 生物原型是灵感源，不是生理模拟
//! 六维对应多巴胺（奖赏预测误差 RPE）/ 血清素（基线稳定与耐心）/ 催产素（联结与信任）/
//! 皮质醇（压力与警觉）/ 去甲肾上腺素（唤醒）/ 乙酰胆碱（注意与学习增益）。这些名字是
//! **工程标签**：调质值是 `[0, 1]` 区间内的标量参数，**不宣称**主观情绪、意识或生理真实性；
//! [`BehaviorParams`] 的映射是可审阅的工程近似，不是内分泌模型。
//!
//! # 慢变量与快事件分层
//! 对话 / 工具 / 感知产生**快事件**（[`NeuromodulatorEvent`]），事件只以有符号脉冲注入；
//! 调质本身是**慢、全局、有惯性**的变量：每维按指数率向基线衰减，跨事件保留。快层与慢层
//! 不共享状态；本模块不读事件流、不读时钟、不读存储。
//!
//! # 时间与确定性
//! 时间一律由调用方注入 `now_ms: u64`（毫秒，单调时钟语义）。本模块不调用
//! `SystemTime::now()`，不使用随机数、线程、`HashMap` 迭代等非确定性来源；完全相同的
//! 输入序列必然产生逐位相同的输出。
//!
//! # 不变量（单测钉死）
//! - 六维值恒为有限 `f32` 且落在配置 clamp 界内（默认 `[0, 1]`）；
//! - `NaN` / `±∞` 输入按安全回落处理：事件载荷 → 0，状态量 → 对应基线；
//! - 无事件时向基线**单调**收敛；同一时间戳重复衰减幂等；时间回拨视为 0 流逝；
//! - 事件方向有符号（成功抬多巴胺、威胁抬皮质醇、社交正抬催产素）；
//! - 同一事件多次 `apply` 产生累计效应，但任何时刻都不越过 clamp 界；
//! - [`BehaviorParams`] 每个字段恒在各自文档化的合法区间内。
//!
//! # 与产品面的唯一接口
//! [`NeuromodulatorState::behavior_params`] → [`BehaviorParams`]（语气温度、耐心倍率、
//! 主动频率倍率、好奇增益、警觉阈值、详尽度）。这是后续 persona / 主动关怀接线的**唯一**
//! 消费点；本模块不生成文案、不决定是否发消息、不持有会话状态。
//!
//! # 未做项（0 装诚实）
//! - 不做 RPE 学习策略：RPE 只是一个带符号注入脉冲，不更新任何预测器；
//! - 不做事件采集、调度 tick、持久化、快照迁移、遥测与自述节；
//! - 不做多智能体间共享调质池；每实例状态独立；
//! - 不做与情绪记忆 / 生物节律 / 内稳态模块的交叉耦合（留给 W2+）。

use serde::{Deserialize, Serialize};

/// 调质维数（固定 6）。
pub const DIMENSION_COUNT: usize = 6;

/// 调质标量统一量程下界。
pub const MIN_LEVEL: f32 = 0.0;

/// 调质标量统一量程上界。
pub const MAX_LEVEL: f32 = 1.0;

/// 六维基线（无事件稳态的工程先验），下标顺序同 [`NeuromodulatorKind::ALL`]。
///
/// `[多巴胺 0.50, 血清素 0.55, 催产素 0.50, 皮质醇 0.30, 去甲肾上腺素 0.40,
/// 乙酰胆碱 0.50]`。基线是常量，不被配置改写；配置的 clamp 界只会把它截断成
/// 「有效基线」（见 [`NeuromodulatorState::baseline_vector`]）。
pub const BASELINE_LEVELS: [f32; DIMENSION_COUNT] = [0.50, 0.55, 0.50, 0.30, 0.40, 0.50];

/// 有符号增量的幅值上限（事件脉冲与耦合步进都截断到 `[-1, 1]`）。
const MAX_DELTA_MAGNITUDE: f32 = 1.0;

/// 默认每维指数衰减率（单位 `1/ms`），下标顺序同 [`NeuromodulatorKind::ALL`]。
///
/// 对应半衰期约：多巴胺 90 s、血清素 8 min、催产素 5 min、皮质醇 12 min、
/// 去甲肾上腺素 45 s、乙酰胆碱 30 s。慢变量语义：单次事件之后需要数十秒到数分钟
/// 才回落，避免调质被单条消息瞬间抹平。
pub const DEFAULT_DECAY_PER_MS: [f32; DIMENSION_COUNT] = [
    7.70e-6, // 多巴胺: ln(2) / 90_000 ms
    1.44e-6, // 血清素: ln(2) / 480_000 ms
    2.31e-6, // 催产素: ln(2) / 300_000 ms
    9.63e-7, // 皮质醇: ln(2) / 720_000 ms
    1.54e-5, // 去甲肾上腺素: ln(2) / 45_000 ms
    2.31e-5, // 乙酰胆碱: ln(2) / 30_000 ms
];

/// 默认事件增益（九类事件全部 1.0）。
pub const DEFAULT_EVENT_GAIN: f32 = 1.0;

/// 事件增益允许上限（安全化时截断；防止配置病态放大）。
pub const MAX_EVENT_GAIN: f32 = 4.0;

/// 耦合系数允许的绝对值上限（安全化时截断）。
pub const MAX_COUPLING_MAGNITUDE: f32 = 1.0;

/// 默认 6×6 一阶耦合矩阵：`coupling[source][target]`（行 = 源维、列 = 目标维）。
///
/// 默认只启用两条文档化耦合，其余 34 项为 0：
/// - 皮质醇↑ → 血清素↓（`-0.35`）：压力压低基线稳定与耐心；
/// - 多巴胺↑ → 乙酰胆碱↑（`+0.30`）：奖赏/趋近信号放大注意与学习增益。
///
/// 耦合只对**本次事件实际造成的源维变化**传导一次；不按存量偏离反复计费，也不做
/// 级联迭代（见 [`NeuromodulatorConfig::coupling`]）。
pub const DEFAULT_COUPLING: [[f32; DIMENSION_COUNT]; DIMENSION_COUNT] = [
    // 源 = 多巴胺：多巴胺↑ → 乙酰胆碱↑（+0.30）
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.30],
    // 源 = 血清素：默认无耦合
    [0.0; DIMENSION_COUNT],
    // 源 = 催产素：默认无耦合
    [0.0; DIMENSION_COUNT],
    // 源 = 皮质醇：皮质醇↑ → 血清素↓（-0.35）
    [0.0, -0.35, 0.0, 0.0, 0.0, 0.0],
    // 源 = 去甲肾上腺素：默认无耦合
    [0.0; DIMENSION_COUNT],
    // 源 = 乙酰胆碱：默认无耦合
    [0.0; DIMENSION_COUNT],
];

/// 神经调质种类（六维慢变量，枚举顺序即所有六元数组的下标顺序）。
///
/// 名字是工程标签，只承载六个可调标的工程角色：奖赏预测误差 / 基线稳定 / 联结 /
/// 压力 / 唤醒 / 注意与学习增益。六维一律非负，`[0, 1]` 量程。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NeuromodulatorKind {
    /// 多巴胺：奖赏预测误差（RPE）与趋近动机。
    Dopamine,
    /// 血清素：基线稳定与耐心。
    Serotonin,
    /// 催产素：联结与信任。
    Oxytocin,
    /// 皮质醇：压力与警觉。
    Cortisol,
    /// 去甲肾上腺素：唤醒与突发响应。
    Noradrenaline,
    /// 乙酰胆碱：注意与学习增益。
    Acetylcholine,
}

impl NeuromodulatorKind {
    /// 六维固定顺序，是所有数组下标语义的单一真源。
    pub const ALL: [Self; DIMENSION_COUNT] = [
        Self::Dopamine,
        Self::Serotonin,
        Self::Oxytocin,
        Self::Cortisol,
        Self::Noradrenaline,
        Self::Acetylcholine,
    ];

    /// 维下标（`0..6`），与 [`Self::ALL`] 及所有六元数组一致。
    pub const fn index(self) -> usize {
        match self {
            Self::Dopamine => 0,
            Self::Serotonin => 1,
            Self::Oxytocin => 2,
            Self::Cortisol => 3,
            Self::Noradrenaline => 4,
            Self::Acetylcholine => 5,
        }
    }

    /// 由维下标还原（越界返 `None`）。
    pub const fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::Dopamine),
            1 => Some(Self::Serotonin),
            2 => Some(Self::Oxytocin),
            3 => Some(Self::Cortisol),
            4 => Some(Self::Noradrenaline),
            5 => Some(Self::Acetylcholine),
            _ => None,
        }
    }

    /// 中性 ASCII 标签（日志 / 遥测用，不含第三方产品名）。
    pub const fn code(self) -> &'static str {
        match self {
            Self::Dopamine => "dopamine",
            Self::Serotonin => "serotonin",
            Self::Oxytocin => "oxytocin",
            Self::Cortisol => "cortisol",
            Self::Noradrenaline => "noradrenaline",
            Self::Acetylcholine => "acetylcholine",
        }
    }

    /// 该维的全局基线值（见 [`BASELINE_LEVELS`]）。
    pub const fn baseline(self) -> f32 {
        BASELINE_LEVELS[self.index()]
    }
}

/// 六维调质状态向量，每个分量都是 `[0, 1]` 内的有限 `f32`。
///
/// 约定：分量越高表示该调质越强（例如皮质醇越高表示压力越大），不使用负值；
/// 有符号脉冲请用 [`NeuromodulatorDelta`]。直接写 public 字段会绕过安全化，
/// 需要约束的写入请走 [`Self::new`] / [`Self::from_array`] / [`Self::set`]；
/// [`NeuromodulatorState`] 在每次更新后都会按配置重新 clamp。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NeuromodulatorVector {
    /// 多巴胺 `[0, 1]`。
    pub dopamine: f32,
    /// 血清素 `[0, 1]`。
    pub serotonin: f32,
    /// 催产素 `[0, 1]`。
    pub oxytocin: f32,
    /// 皮质醇 `[0, 1]`。
    pub cortisol: f32,
    /// 去甲肾上腺素 `[0, 1]`。
    pub noradrenaline: f32,
    /// 乙酰胆碱 `[0, 1]`。
    pub acetylcholine: f32,
}

impl Default for NeuromodulatorVector {
    /// 默认值 = [`Self::baseline`]。
    fn default() -> Self {
        Self::baseline()
    }
}

impl NeuromodulatorVector {
    /// 无事件稳态基线（见 [`BASELINE_LEVELS`]）。
    pub fn baseline() -> Self {
        Self {
            dopamine: BASELINE_LEVELS[0],
            serotonin: BASELINE_LEVELS[1],
            oxytocin: BASELINE_LEVELS[2],
            cortisol: BASELINE_LEVELS[3],
            noradrenaline: BASELINE_LEVELS[4],
            acetylcholine: BASELINE_LEVELS[5],
        }
    }

    /// 由六维构造：非有限分量回落到对应基线，有限分量截断到 `[MIN_LEVEL, MAX_LEVEL]`。
    pub fn new(
        dopamine: f32,
        serotonin: f32,
        oxytocin: f32,
        cortisol: f32,
        noradrenaline: f32,
        acetylcholine: f32,
    ) -> Self {
        Self {
            dopamine,
            serotonin,
            oxytocin,
            cortisol,
            noradrenaline,
            acetylcholine,
        }
        .sanitized()
    }

    /// 由数组构造，下标顺序同 [`NeuromodulatorKind::ALL`]；安全化规则同 [`Self::new`]。
    pub fn from_array(values: [f32; DIMENSION_COUNT]) -> Self {
        Self {
            dopamine: values[0],
            serotonin: values[1],
            oxytocin: values[2],
            cortisol: values[3],
            noradrenaline: values[4],
            acetylcholine: values[5],
        }
        .sanitized()
    }

    /// 展开为数组，下标顺序同 [`NeuromodulatorKind::ALL`]。
    pub fn as_array(&self) -> [f32; DIMENSION_COUNT] {
        [
            self.dopamine,
            self.serotonin,
            self.oxytocin,
            self.cortisol,
            self.noradrenaline,
            self.acetylcholine,
        ]
    }

    /// 读取某一维。
    pub fn get(&self, kind: NeuromodulatorKind) -> f32 {
        match kind {
            NeuromodulatorKind::Dopamine => self.dopamine,
            NeuromodulatorKind::Serotonin => self.serotonin,
            NeuromodulatorKind::Oxytocin => self.oxytocin,
            NeuromodulatorKind::Cortisol => self.cortisol,
            NeuromodulatorKind::Noradrenaline => self.noradrenaline,
            NeuromodulatorKind::Acetylcholine => self.acetylcholine,
        }
    }

    /// 写入某一维：有限值按 `[MIN_LEVEL, MAX_LEVEL]` 截断，非有限值忽略（保留原值）。
    pub fn set(&mut self, kind: NeuromodulatorKind, value: f32) {
        if value.is_finite() {
            *component_mut(self, kind) = value.clamp(MIN_LEVEL, MAX_LEVEL);
        }
    }

    /// 六维是否全为有限值。
    pub fn all_finite(&self) -> bool {
        self.as_array().iter().all(|value| value.is_finite())
    }

    /// 六维是否全部落在 `[min, max]`（边界本身也需有限且 `min <= max`）。
    pub fn is_within(&self, min: f32, max: f32) -> bool {
        if !min.is_finite() || !max.is_finite() || min > max {
            return false;
        }
        self.all_finite() && self.as_array().iter().all(|v| *v >= min && *v <= max)
    }

    /// 与另一向量的 L1 距离（六维绝对差之和），用于收敛性度量。
    pub fn l1_distance(&self, other: &Self) -> f32 {
        let a = self.as_array();
        let b = other.as_array();
        a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum()
    }

    /// 逐维安全化到 `[MIN_LEVEL, MAX_LEVEL]`，非有限分量回落对应基线。
    fn sanitized(self) -> Self {
        Self {
            dopamine: sanitize_level(self.dopamine, BASELINE_LEVELS[0], MIN_LEVEL, MAX_LEVEL),
            serotonin: sanitize_level(self.serotonin, BASELINE_LEVELS[1], MIN_LEVEL, MAX_LEVEL),
            oxytocin: sanitize_level(self.oxytocin, BASELINE_LEVELS[2], MIN_LEVEL, MAX_LEVEL),
            cortisol: sanitize_level(self.cortisol, BASELINE_LEVELS[3], MIN_LEVEL, MAX_LEVEL),
            noradrenaline: sanitize_level(
                self.noradrenaline,
                BASELINE_LEVELS[4],
                MIN_LEVEL,
                MAX_LEVEL,
            ),
            acetylcholine: sanitize_level(
                self.acetylcholine,
                BASELINE_LEVELS[5],
                MIN_LEVEL,
                MAX_LEVEL,
            ),
        }
    }

    /// 逐维截断到 `[min, max]`，非有限分量回落对应基线（`min <= max` 由调用方保证）。
    fn clamped_to(self, min: f32, max: f32) -> Self {
        Self {
            dopamine: sanitize_level(self.dopamine, BASELINE_LEVELS[0], min, max),
            serotonin: sanitize_level(self.serotonin, BASELINE_LEVELS[1], min, max),
            oxytocin: sanitize_level(self.oxytocin, BASELINE_LEVELS[2], min, max),
            cortisol: sanitize_level(self.cortisol, BASELINE_LEVELS[3], min, max),
            noradrenaline: sanitize_level(self.noradrenaline, BASELINE_LEVELS[4], min, max),
            acetylcholine: sanitize_level(self.acetylcholine, BASELINE_LEVELS[5], min, max),
        }
    }
}

impl std::ops::Index<NeuromodulatorKind> for NeuromodulatorVector {
    type Output = f32;

    fn index(&self, kind: NeuromodulatorKind) -> &f32 {
        match kind {
            NeuromodulatorKind::Dopamine => &self.dopamine,
            NeuromodulatorKind::Serotonin => &self.serotonin,
            NeuromodulatorKind::Oxytocin => &self.oxytocin,
            NeuromodulatorKind::Cortisol => &self.cortisol,
            NeuromodulatorKind::Noradrenaline => &self.noradrenaline,
            NeuromodulatorKind::Acetylcholine => &self.acetylcholine,
        }
    }
}

/// 六维有符号增量（事件脉冲 / 耦合步进），每分量有限且截断到 `[-1, 1]`。
///
/// 与 [`NeuromodulatorVector`]（状态量，`[0, 1]`）区分：增量可以为负，表示压低某维；
/// 事件的基础脉冲见 [`NeuromodulatorEvent::base_impulse`]。
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct NeuromodulatorDelta {
    /// 多巴胺增量 `[-1, 1]`。
    pub dopamine: f32,
    /// 血清素增量 `[-1, 1]`。
    pub serotonin: f32,
    /// 催产素增量 `[-1, 1]`。
    pub oxytocin: f32,
    /// 皮质醇增量 `[-1, 1]`。
    pub cortisol: f32,
    /// 去甲肾上腺素增量 `[-1, 1]`。
    pub noradrenaline: f32,
    /// 乙酰胆碱增量 `[-1, 1]`。
    pub acetylcholine: f32,
}

impl NeuromodulatorDelta {
    /// 全零增量。
    pub fn zero() -> Self {
        Self::default()
    }

    /// 由六维构造：非有限分量回落 0，有限分量截断到 `[-1, 1]`。
    pub fn new(
        dopamine: f32,
        serotonin: f32,
        oxytocin: f32,
        cortisol: f32,
        noradrenaline: f32,
        acetylcholine: f32,
    ) -> Self {
        Self {
            dopamine,
            serotonin,
            oxytocin,
            cortisol,
            noradrenaline,
            acetylcholine,
        }
        .sanitized()
    }

    /// 由数组构造，下标顺序同 [`NeuromodulatorKind::ALL`]。
    pub fn from_array(values: [f32; DIMENSION_COUNT]) -> Self {
        Self {
            dopamine: values[0],
            serotonin: values[1],
            oxytocin: values[2],
            cortisol: values[3],
            noradrenaline: values[4],
            acetylcholine: values[5],
        }
        .sanitized()
    }

    /// 展开为数组，下标顺序同 [`NeuromodulatorKind::ALL`]。
    pub fn as_array(&self) -> [f32; DIMENSION_COUNT] {
        [
            self.dopamine,
            self.serotonin,
            self.oxytocin,
            self.cortisol,
            self.noradrenaline,
            self.acetylcholine,
        ]
    }

    /// 读取某一维增量。
    pub fn get(&self, kind: NeuromodulatorKind) -> f32 {
        self[kind]
    }

    /// 逐维安全化到 `[-1, 1]`，非有限分量回落 0。
    fn sanitized(self) -> Self {
        Self {
            dopamine: sanitize_delta(self.dopamine),
            serotonin: sanitize_delta(self.serotonin),
            oxytocin: sanitize_delta(self.oxytocin),
            cortisol: sanitize_delta(self.cortisol),
            noradrenaline: sanitize_delta(self.noradrenaline),
            acetylcholine: sanitize_delta(self.acetylcholine),
        }
    }
}

impl std::ops::Index<NeuromodulatorKind> for NeuromodulatorDelta {
    type Output = f32;

    fn index(&self, kind: NeuromodulatorKind) -> &f32 {
        match kind {
            NeuromodulatorKind::Dopamine => &self.dopamine,
            NeuromodulatorKind::Serotonin => &self.serotonin,
            NeuromodulatorKind::Oxytocin => &self.oxytocin,
            NeuromodulatorKind::Cortisol => &self.cortisol,
            NeuromodulatorKind::Noradrenaline => &self.noradrenaline,
            NeuromodulatorKind::Acetylcholine => &self.acetylcholine,
        }
    }
}

/// 快事件：驱动慢调质变量的离散输入。
///
/// 事件只描述「发生了什么」，不含时间戳；时间由调用方在
/// [`NeuromodulatorState::apply`] 注入。所有事件载荷按有限 `f32` 处理：非有限值按 0
/// 回落，RPE 截断到 `[-1, 1]`。事件本身无副作用，可安全复制 / 序列化 / 排队。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum NeuromodulatorEvent {
    /// 奖赏预测误差：`RPE = actual - expected`。
    ///
    /// 正向 RPE 抬多巴胺（趋近 / 学习），负向 RPE 压低多巴胺；`abs(RPE)` 额外抬
    /// 乙酰胆碱（显著误差放大注意与学习增益）。
    RewardPredictionError {
        /// 预期值（非有限按 0）；不要求落在 `[0, 1]`。
        expected: f32,
        /// 实际值（非有限按 0）；不要求落在 `[0, 1]`。
        actual: f32,
    },
    /// 任务成功：多巴胺 / 血清素小升，皮质醇下降。
    TaskSuccess,
    /// 任务失败：多巴胺 / 血清素下降，皮质醇上升（失败也是学习信号，乙酰胆碱小升）。
    TaskFailure,
    /// 社交正反馈：催产素 / 血清素上升，皮质醇下降。
    SocialPositive,
    /// 社交负反馈：催产素 / 血清素下降，皮质醇 / 去甲肾上腺素上升。
    SocialNegative,
    /// 威胁：皮质醇 / 去甲肾上腺素大幅上升，多巴胺 / 血清素下降。
    Threat,
    /// 新异刺激：多巴胺 / 去甲肾上腺素 / 乙酰胆碱上升。
    Novelty,
    /// 安静期：去甲肾上腺素 / 皮质醇下降，血清素小升。
    Quiet,
    /// 用户纠错：乙酰胆碱大幅上升（注意重定向），皮质醇 / 去甲肾上腺素小升。
    UserCorrection,
}

impl NeuromodulatorEvent {
    /// 事件种类数（= [`NeuromodulatorConfig::event_gains`] 的数组长度）。
    pub const KIND_COUNT: usize = 9;

    /// 事件下标（`0..9`），与 [`NeuromodulatorConfig::event_gains`] 一致。
    pub const fn index(self) -> usize {
        match self {
            Self::RewardPredictionError { .. } => 0,
            Self::TaskSuccess => 1,
            Self::TaskFailure => 2,
            Self::SocialPositive => 3,
            Self::SocialNegative => 4,
            Self::Threat => 5,
            Self::Novelty => 6,
            Self::Quiet => 7,
            Self::UserCorrection => 8,
        }
    }

    /// 中性 ASCII 标签（日志 / 遥测用）。
    pub const fn code(self) -> &'static str {
        match self {
            Self::RewardPredictionError { .. } => "reward_prediction_error",
            Self::TaskSuccess => "task_success",
            Self::TaskFailure => "task_failure",
            Self::SocialPositive => "social_positive",
            Self::SocialNegative => "social_negative",
            Self::Threat => "threat",
            Self::Novelty => "novelty",
            Self::Quiet => "quiet",
            Self::UserCorrection => "user_correction",
        }
    }

    /// 仅 RPE 事件返回 `actual - expected`（载荷先安全化，结果截断到 `[-1, 1]`）；
    /// 其余事件返回 `None`。
    pub fn reward_prediction_error(self) -> Option<f32> {
        match self {
            Self::RewardPredictionError { expected, actual } => {
                let expected = if expected.is_finite() { expected } else { 0.0 };
                let actual = if actual.is_finite() { actual } else { 0.0 };
                Some((actual - expected).clamp(-1.0, 1.0))
            }
            _ => None,
        }
    }

    /// 事件基础脉冲（未乘 [`NeuromodulatorConfig::event_gains`]、未计耦合）。
    ///
    /// | 事件 | 多巴胺 | 血清素 | 催产素 | 皮质醇 | 去甲肾上腺素 | 乙酰胆碱 |
    /// | --- | --- | --- | --- | --- | --- | --- |
    /// | `RewardPredictionError` | `0.12 * RPE` | 0 | 0 | 0 | 0 | `0.03 * abs(RPE)` |
    /// | `TaskSuccess` | `+0.08` | `+0.04` | `+0.01` | `-0.05` | `-0.01` | `+0.02` |
    /// | `TaskFailure` | `-0.06` | `-0.03` | 0 | `+0.07` | `+0.03` | `+0.03` |
    /// | `SocialPositive` | `+0.02` | `+0.05` | `+0.08` | `-0.04` | 0 | 0 |
    /// | `SocialNegative` | `-0.01` | `-0.04` | `-0.05` | `+0.06` | `+0.02` | 0 |
    /// | `Threat` | `-0.04` | `-0.03` | 0 | `+0.12` | `+0.10` | `+0.02` |
    /// | `Novelty` | `+0.06` | 0 | 0 | 0 | `+0.04` | `+0.06` |
    /// | `Quiet` | `-0.01` | `+0.03` | 0 | `-0.05` | `-0.08` | 0 |
    /// | `UserCorrection` | `-0.02` | `-0.01` | 0 | `+0.03` | `+0.03` | `+0.08` |
    ///
    /// 表中未列出的耦合效应由 [`NeuromodulatorConfig::coupling`] 另行叠加。
    pub fn base_impulse(self) -> NeuromodulatorDelta {
        match self {
            Self::RewardPredictionError { .. } => {
                let rpe = self.reward_prediction_error().unwrap_or(0.0);
                NeuromodulatorDelta::new(0.12 * rpe, 0.0, 0.0, 0.0, 0.0, 0.03 * rpe.abs())
            }
            Self::TaskSuccess => NeuromodulatorDelta::new(0.08, 0.04, 0.01, -0.05, -0.01, 0.02),
            Self::TaskFailure => NeuromodulatorDelta::new(-0.06, -0.03, 0.0, 0.07, 0.03, 0.03),
            Self::SocialPositive => NeuromodulatorDelta::new(0.02, 0.05, 0.08, -0.04, 0.0, 0.0),
            Self::SocialNegative => NeuromodulatorDelta::new(-0.01, -0.04, -0.05, 0.06, 0.02, 0.0),
            Self::Threat => NeuromodulatorDelta::new(-0.04, -0.03, 0.0, 0.12, 0.10, 0.02),
            Self::Novelty => NeuromodulatorDelta::new(0.06, 0.0, 0.0, 0.0, 0.04, 0.06),
            Self::Quiet => NeuromodulatorDelta::new(-0.01, 0.03, 0.0, -0.05, -0.08, 0.0),
            Self::UserCorrection => NeuromodulatorDelta::new(-0.02, -0.01, 0.0, 0.03, 0.03, 0.08),
        }
    }
}

/// 调质内核配置：衰减率、事件增益、耦合矩阵与 clamp 界。
///
/// 所有字段在 [`NeuromodulatorState::new`] 中经 [`Self::sanitized`] 安全化：
/// 非有限值、负衰减率、负增益、超限耦合、倒置 / 越界边界都不会进入运行态。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NeuromodulatorConfig {
    /// 每维指数衰减率（单位 `1/ms`，非负有限；非法值安全化为 0 = 不衰减）。
    ///
    /// 下标顺序同 [`NeuromodulatorKind::ALL`]。默认值见 [`DEFAULT_DECAY_PER_MS`]，
    /// 半衰期从 30 s 到 12 min 不等。
    pub decay_per_ms: [f32; DIMENSION_COUNT],

    /// 九类事件的注入增益，下标同 [`NeuromodulatorEvent::index`]。
    ///
    /// 有限值截断到 `[0, MAX_EVENT_GAIN]`；非有限值按 0（该事件失效）。默认全 1.0。
    pub event_gains: [f32; NeuromodulatorEvent::KIND_COUNT],

    /// 6×6 一阶耦合矩阵：`coupling[source][target]`（行 = 源维、列 = 目标维），
    /// 下标顺序同 [`NeuromodulatorKind::ALL`]。
    ///
    /// 语义：先叠加本次事件脉冲，再把**本次实际造成的源维变化** `d[source]` 按
    /// `coupling[source][target] * d[source]` 一次性传导到目标维（基于同一快照，
    /// 不做迭代 / 级联）。存量偏离不会反复触发耦合；同一事件多次 `apply` 会多次累计。
    /// 系数有限值截断到 `[-MAX_COUPLING_MAGNITUDE, MAX_COUPLING_MAGNITUDE]`，非有限按 0。
    /// 默认只启用两条文档化耦合，见 [`DEFAULT_COUPLING`]。
    pub coupling: [[f32; DIMENSION_COUNT]; DIMENSION_COUNT],

    /// clamp 下界（安全化时截断到 `[MIN_LEVEL, MAX_LEVEL]`）。
    ///
    /// 与 [`Self::max_level`] 构成闭区间；倒置时自动交换，非法 / 相等时整对回落
    /// `[MIN_LEVEL, MAX_LEVEL]`。有效基线 = 全局基线按该区间截断。
    pub min_level: f32,

    /// clamp 上界（安全化规则同 [`Self::min_level`]）。
    pub max_level: f32,
}

impl Default for NeuromodulatorConfig {
    /// 默认配置：默认衰减率 + 全 1 事件增益 + 两条文档化耦合 + `[0, 1]` clamp 界。
    fn default() -> Self {
        Self {
            decay_per_ms: DEFAULT_DECAY_PER_MS,
            event_gains: [DEFAULT_EVENT_GAIN; NeuromodulatorEvent::KIND_COUNT],
            coupling: DEFAULT_COUPLING,
            min_level: MIN_LEVEL,
            max_level: MAX_LEVEL,
        }
    }
}

impl NeuromodulatorConfig {
    /// 返回安全化副本（不修改 `self`）。
    ///
    /// 规则：
    /// - `decay_per_ms`：非有限或负数 → 0；
    /// - `event_gains`：非有限 → 0，有限值截断到 `[0, MAX_EVENT_GAIN]`；
    /// - `coupling`：非有限 → 0，有限值截断到 `[-MAX_COUPLING_MAGNITUDE,
    ///   MAX_COUPLING_MAGNITUDE]`；
    /// - `min_level` / `max_level`：先各自截断到 `[MIN_LEVEL, MAX_LEVEL]`，倒置则交换，
    ///   相等或非法则整对回落 `[MIN_LEVEL, MAX_LEVEL]`。
    pub fn sanitized(&self) -> Self {
        let mut min_level = sanitize_bound(self.min_level, MIN_LEVEL);
        let mut max_level = sanitize_bound(self.max_level, MAX_LEVEL);
        if min_level > max_level {
            std::mem::swap(&mut min_level, &mut max_level);
        }
        if !(max_level > min_level) {
            min_level = MIN_LEVEL;
            max_level = MAX_LEVEL;
        }

        let mut decay_per_ms = [0.0f32; DIMENSION_COUNT];
        for (slot, value) in decay_per_ms.iter_mut().zip(self.decay_per_ms) {
            *slot = if value.is_finite() && value >= 0.0 {
                value
            } else {
                0.0
            };
        }

        let mut event_gains = [0.0f32; NeuromodulatorEvent::KIND_COUNT];
        for (slot, value) in event_gains.iter_mut().zip(self.event_gains) {
            *slot = if value.is_finite() {
                value.clamp(0.0, MAX_EVENT_GAIN)
            } else {
                0.0
            };
        }

        let mut coupling = [[0.0f32; DIMENSION_COUNT]; DIMENSION_COUNT];
        for (source, row) in coupling.iter_mut().enumerate() {
            for (target, slot) in row.iter_mut().enumerate() {
                let value = self.coupling[source][target];
                *slot = if value.is_finite() {
                    value.clamp(-MAX_COUPLING_MAGNITUDE, MAX_COUPLING_MAGNITUDE)
                } else {
                    0.0
                };
            }
        }

        Self {
            decay_per_ms,
            event_gains,
            coupling,
            min_level,
            max_level,
        }
    }
}

/// 产品面行为参数：六维调质向量 → 语气 / 节奏 / 注意旋钮的唯一映射出口。
///
/// 所有映射先对输入分量做「非有限 → 对应基线、有限值截断 `[0, 1]`」的安全处理，再按
/// 下列公式计算，最后按各字段合法区间 clamp。记 `d` = 多巴胺、`s` = 血清素、
/// `o` = 催产素、`c` = 皮质醇、`n` = 去甲肾上腺素、`a` = 乙酰胆碱，公式逐条写在
/// 各字段 doc 上。基线向量对应的参考值：`warmth` 0.555、`patience_factor` 1.135、
/// `initiative_frequency` 0.87、`curiosity_gain` 0.49、`caution_threshold` 0.505、
/// `verbosity` 0.5375。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BehaviorParams {
    /// 语气温度 = `clamp(0.15 + 0.60*o + 0.30*s - 0.20*c, 0, 1)`。
    ///
    /// 0 = 克制中性，1 = 温暖。催产素（联结）与血清素（稳定）抬升，皮质醇（压力）压低。
    pub warmth: f32,

    /// 追问前等待倍率 = `clamp(0.50 + 1.50*s + 0.50*o - 0.80*c - 0.50*n, 0.5, 2.0)`。
    ///
    /// 1.0 = 基线等待；> 1 更愿意等用户回应，< 1 更急。血清素 / 催产素抬升，
    /// 皮质醇 / 去甲肾上腺素压低。合法区间 `[0.5, 2.0]`。
    pub patience_factor: f32,

    /// 主动消息频率倍率 = `clamp(0.20 + 0.90*d + 0.50*o + 0.30*a - 0.60*c, 0, 2.0)`。
    ///
    /// 1.0 = 基线频率；0 = 完全不自发。多巴胺（趋近）/ 催产素（联结）/ 乙酰胆碱
    /// （注意）抬升，皮质醇（压力）压低。合法区间 `[0.0, 2.0]`。
    pub initiative_frequency: f32,

    /// 好奇增益 = `clamp(0.10 + 0.45*d + 0.45*a - 0.20*c, 0, 1)`。
    ///
    /// 多巴胺（奖赏预期）与乙酰胆碱（注意）抬升，皮质醇（压力）压低。合法区间 `[0, 1]`。
    pub curiosity_gain: f32,

    /// 警觉阈值 = `clamp(0.55 - 0.35*c - 0.25*n + 0.20*s + 0.10*o, 0.05, 0.95)`。
    ///
    /// 语义是**风险阈值**：输入风险分超过该阈值即走谨慎路径，因此**值越低越易触发谨慎**。
    /// 皮质醇 / 去甲肾上腺素压低阈值（压力与唤醒提高警觉），血清素 / 催产素抬升阈值
    /// （稳定与联结提高容忍度）。合法区间 `[0.05, 0.95]`。
    pub caution_threshold: f32,

    /// 表达详尽度 = `clamp(0.20 + 0.35*a + 0.25*s + 0.20*d - 0.25*c, 0, 1)`。
    ///
    /// 乙酰胆碱 / 血清素 / 多巴胺抬升，皮质醇（压力）压低。合法区间 `[0, 1]`。
    pub verbosity: f32,
}

impl BehaviorParams {
    /// `patience_factor` 的合法区间。
    pub const PATIENCE_RANGE: (f32, f32) = (0.5, 2.0);

    /// `initiative_frequency` 的合法区间。
    pub const INITIATIVE_RANGE: (f32, f32) = (0.0, 2.0);

    /// `caution_threshold` 的合法区间。
    pub const CAUTION_RANGE: (f32, f32) = (0.05, 0.95);

    /// `warmth` / `curiosity_gain` / `verbosity` 共用的 `[0, 1]` 合法区间。
    pub const UNIT_RANGE: (f32, f32) = (0.0, 1.0);

    /// 基线调质向量对应的行为参数（产品面缺省值 / 回退点）。
    pub fn baseline() -> Self {
        Self::from_vector(&NeuromodulatorVector::baseline())
    }

    /// 把调质向量映射为行为参数；映射公式逐条见本类型字段 doc。
    ///
    /// 输入分量先安全化（非有限 → 对应全局基线，有限值截断 `[0, 1]`），因此本函数
    /// 对任意 `f32` 输入都返回合法结果，不会产生 `NaN`。
    pub fn from_vector(vector: &NeuromodulatorVector) -> Self {
        let d = sanitize_level(vector.dopamine, BASELINE_LEVELS[0], MIN_LEVEL, MAX_LEVEL);
        let s = sanitize_level(vector.serotonin, BASELINE_LEVELS[1], MIN_LEVEL, MAX_LEVEL);
        let o = sanitize_level(vector.oxytocin, BASELINE_LEVELS[2], MIN_LEVEL, MAX_LEVEL);
        let c = sanitize_level(vector.cortisol, BASELINE_LEVELS[3], MIN_LEVEL, MAX_LEVEL);
        let n = sanitize_level(
            vector.noradrenaline,
            BASELINE_LEVELS[4],
            MIN_LEVEL,
            MAX_LEVEL,
        );
        let a = sanitize_level(
            vector.acetylcholine,
            BASELINE_LEVELS[5],
            MIN_LEVEL,
            MAX_LEVEL,
        );

        Self {
            warmth: (0.15 + 0.60 * o + 0.30 * s - 0.20 * c)
                .clamp(Self::UNIT_RANGE.0, Self::UNIT_RANGE.1),
            patience_factor: (0.50 + 1.50 * s + 0.50 * o - 0.80 * c - 0.50 * n)
                .clamp(Self::PATIENCE_RANGE.0, Self::PATIENCE_RANGE.1),
            initiative_frequency: (0.20 + 0.90 * d + 0.50 * o + 0.30 * a - 0.60 * c)
                .clamp(Self::INITIATIVE_RANGE.0, Self::INITIATIVE_RANGE.1),
            curiosity_gain: (0.10 + 0.45 * d + 0.45 * a - 0.20 * c)
                .clamp(Self::UNIT_RANGE.0, Self::UNIT_RANGE.1),
            caution_threshold: (0.55 - 0.35 * c - 0.25 * n + 0.20 * s + 0.10 * o)
                .clamp(Self::CAUTION_RANGE.0, Self::CAUTION_RANGE.1),
            verbosity: (0.20 + 0.35 * a + 0.25 * s + 0.20 * d - 0.25 * c)
                .clamp(Self::UNIT_RANGE.0, Self::UNIT_RANGE.1),
        }
    }

    /// 六个字段是否全部有限且落在各自合法区间内。
    pub fn is_within_ranges(&self) -> bool {
        within_range(self.warmth, Self::UNIT_RANGE)
            && within_range(self.patience_factor, Self::PATIENCE_RANGE)
            && within_range(self.initiative_frequency, Self::INITIATIVE_RANGE)
            && within_range(self.curiosity_gain, Self::UNIT_RANGE)
            && within_range(self.caution_threshold, Self::CAUTION_RANGE)
            && within_range(self.verbosity, Self::UNIT_RANGE)
    }
}

/// 调质状态机：配置 + 当前向量 + 上次衰减时间戳。
///
/// 纯确定性：所有时间由调用方注入，内部不读时钟、不做 IO。`new` 会先安全化配置，
/// 因此即使传入 `NaN` / 倒置边界也不会产生非法状态。构造时逻辑时间为 0；首次
/// `apply` / `decay` 前调用方应保证 `now_ms` 单调（回拨按 0 流逝处理，不倒退状态）。
#[derive(Debug, Clone, PartialEq)]
pub struct NeuromodulatorState {
    /// 已安全化的配置。
    config: NeuromodulatorConfig,
    /// 当前六维向量。
    vector: NeuromodulatorVector,
    /// 有效基线（全局基线按 `config` clamp 界截断）。
    baseline: NeuromodulatorVector,
    /// 上次状态更新的逻辑毫秒时间戳。
    last_update_ms: u64,
}

impl NeuromodulatorState {
    /// 以基线为初始向量构造状态机；配置先经 [`NeuromodulatorConfig::sanitized`]。
    pub fn new(config: NeuromodulatorConfig) -> Self {
        let config = config.sanitized();
        let initial =
            NeuromodulatorVector::baseline().clamped_to(config.min_level, config.max_level);
        Self {
            config,
            vector: initial,
            baseline: initial,
            last_update_ms: 0,
        }
    }

    /// 已安全化的配置（只读）。
    pub fn config(&self) -> &NeuromodulatorConfig {
        &self.config
    }

    /// 当前六维向量（拷贝）。
    pub fn vector(&self) -> NeuromodulatorVector {
        self.vector
    }

    /// 有效基线（全局基线按配置 clamp 界截断后的值，衰减的收敛目标）。
    pub fn baseline_vector(&self) -> NeuromodulatorVector {
        self.baseline
    }

    /// 上次状态更新的逻辑毫秒时间戳。
    pub fn last_update_ms(&self) -> u64 {
        self.last_update_ms
    }

    /// 复位到有效基线，并把逻辑时钟设为 `now_ms`。
    pub fn reset(&mut self, now_ms: u64) {
        self.vector = self.baseline;
        self.last_update_ms = now_ms;
    }

    /// 按流逝时间向基线做指数衰减，幂等。
    ///
    /// `dt = to_ms.saturating_sub(last_update_ms)`；`dt == 0`（含时间回拨）不做任何
    /// 改动，也不回退逻辑时钟。每维：`x' = base + (x - base) * exp(-rate * dt)`，
    /// 结果按配置 clamp 界安全化。指数衰减满足半群性质，因此「先 decay(t1) 再
    /// decay(t2)」与「一次 decay(t2)」在无事件时等价（浮点舍入内）。
    pub fn decay(&mut self, to_ms: u64) {
        if to_ms <= self.last_update_ms {
            return;
        }
        let elapsed_ms = to_ms - self.last_update_ms;
        self.last_update_ms = to_ms;
        let elapsed = elapsed_ms as f32;

        for (index, kind) in NeuromodulatorKind::ALL.iter().enumerate() {
            let rate = self.config.decay_per_ms[index];
            if rate == 0.0 {
                continue;
            }
            let factor = (-rate * elapsed).exp();
            let baseline = self.baseline[*kind];
            let next = baseline + (self.vector[*kind] - baseline) * factor;
            *component_mut(&mut self.vector, *kind) =
                sanitize_level(next, baseline, self.config.min_level, self.config.max_level);
        }
    }

    /// 应用一个快事件：先按 `now_ms` 衰减到当前时刻，再注入事件脉冲与一阶耦合。
    ///
    /// - 事件增益为 0 时该事件不产生任何状态变化（但仍推进逻辑时钟）；
    /// - 耦合只对本次事件**实际造成的源维变化**传导一次（基于同一快照，不迭代）；
    /// - 同一事件多次调用会累计，最终由 clamp 界与后续衰减约束，不会越界。
    pub fn apply(&mut self, event: &NeuromodulatorEvent, now_ms: u64) {
        self.decay(now_ms);

        let gain = self.config.event_gains[event.index()];
        if gain == 0.0 {
            return;
        }

        let impulse = event.base_impulse();
        let before = self.vector.as_array();
        for kind in NeuromodulatorKind::ALL {
            let next = self.vector[kind] + impulse[kind] * gain;
            *component_mut(&mut self.vector, kind) = sanitize_level(
                next,
                self.baseline[kind],
                self.config.min_level,
                self.config.max_level,
            );
        }

        let after = self.vector.as_array();
        let mut coupled = [0.0f32; DIMENSION_COUNT];
        for (source, row) in self.config.coupling.iter().enumerate() {
            let applied = after[source] - before[source];
            if applied == 0.0 {
                continue;
            }
            for (target, coefficient) in row.iter().enumerate() {
                coupled[target] += coefficient * applied;
            }
        }
        for (index, kind) in NeuromodulatorKind::ALL.iter().enumerate() {
            let next = self.vector[*kind] + sanitize_delta(coupled[index]);
            *component_mut(&mut self.vector, *kind) = sanitize_level(
                next,
                self.baseline[*kind],
                self.config.min_level,
                self.config.max_level,
            );
        }
    }

    /// 当前向量 → 产品面行为参数（唯一映射出口，公式见 [`BehaviorParams`]）。
    pub fn behavior_params(&self) -> BehaviorParams {
        BehaviorParams::from_vector(&self.vector)
    }
}

/// 把任意 `f32` 收敛到 `[min, max]`：非有限值回落 `fallback`，有限值 clamp。
///
/// 调用方需保证 `fallback` 与边界有限且 `min <= max`（状态机侧均已安全化）。
fn sanitize_level(value: f32, fallback: f32, min: f32, max: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback
    }
}

/// 把有符号增量安全化到 `[-MAX_DELTA_MAGNITUDE, MAX_DELTA_MAGNITUDE]`，非有限值回落 0。
fn sanitize_delta(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(-MAX_DELTA_MAGNITUDE, MAX_DELTA_MAGNITUDE)
    } else {
        0.0
    }
}

/// 把配置边界安全化到 `[MIN_LEVEL, MAX_LEVEL]`，非有限值回落给定默认。
fn sanitize_bound(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(MIN_LEVEL, MAX_LEVEL)
    } else {
        fallback
    }
}

/// 值是否有限且落在闭区间内。
fn within_range(value: f32, range: (f32, f32)) -> bool {
    value.is_finite() && value >= range.0 && value <= range.1
}

/// 取某一维的可变引用（供 [`NeuromodulatorVector::set`] 使用）。
fn component_mut(vector: &mut NeuromodulatorVector, kind: NeuromodulatorKind) -> &mut f32 {
    match kind {
        NeuromodulatorKind::Dopamine => &mut vector.dopamine,
        NeuromodulatorKind::Serotonin => &mut vector.serotonin,
        NeuromodulatorKind::Oxytocin => &mut vector.oxytocin,
        NeuromodulatorKind::Cortisol => &mut vector.cortisol,
        NeuromodulatorKind::Noradrenaline => &mut vector.noradrenaline,
        NeuromodulatorKind::Acetylcholine => &mut vector.acetylcholine,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// 近似比较容差（公式回归用）。
    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1e-6
    }

    /// 默认配置的初始状态。
    fn default_state() -> NeuromodulatorState {
        NeuromodulatorState::new(NeuromodulatorConfig::default())
    }

    /// 关闭全部耦合的配置（对照实验用）。
    fn uncoupled_config() -> NeuromodulatorConfig {
        NeuromodulatorConfig {
            coupling: [[0.0; DIMENSION_COUNT]; DIMENSION_COUNT],
            ..NeuromodulatorConfig::default()
        }
    }

    /// 断言行为参数六项全部在各自合法区间内。
    fn assert_params_legal(params: &BehaviorParams) {
        assert!(
            params.is_within_ranges(),
            "behavior_params 越界: {params:?}"
        );
        assert!(params.warmth >= 0.0 && params.warmth <= 1.0);
        assert!(params.patience_factor >= 0.5 && params.patience_factor <= 2.0);
        assert!(params.initiative_frequency >= 0.0 && params.initiative_frequency <= 2.0);
        assert!(params.curiosity_gain >= 0.0 && params.curiosity_gain <= 1.0);
        assert!(params.caution_threshold >= 0.05 && params.caution_threshold <= 0.95);
        assert!(params.verbosity >= 0.0 && params.verbosity <= 1.0);
    }

    #[test]
    fn kind_all_index_and_baseline_are_consistent() {
        assert_eq!(NeuromodulatorKind::ALL.len(), DIMENSION_COUNT);
        for (index, kind) in NeuromodulatorKind::ALL.iter().enumerate() {
            assert_eq!(kind.index(), index);
            assert_eq!(NeuromodulatorKind::from_index(index), Some(*kind));
            assert_eq!(kind.baseline(), BASELINE_LEVELS[index]);
            assert!(kind.baseline() >= MIN_LEVEL && kind.baseline() <= MAX_LEVEL);
            assert!(!kind.code().is_empty());
        }
        assert_eq!(NeuromodulatorKind::from_index(DIMENSION_COUNT), None);
        let codes: HashSet<&'static str> = NeuromodulatorKind::ALL
            .iter()
            .map(|kind| kind.code())
            .collect();
        assert_eq!(codes.len(), DIMENSION_COUNT, "六维 code 必须唯一");
    }

    #[test]
    fn vector_baseline_and_constructor_are_clamped_and_finite() {
        let baseline = NeuromodulatorVector::baseline();
        assert_eq!(NeuromodulatorVector::default(), baseline);
        assert_eq!(baseline.as_array(), BASELINE_LEVELS);
        assert!(baseline.is_within(MIN_LEVEL, MAX_LEVEL));
        assert!(baseline.all_finite());

        let created = NeuromodulatorVector::new(2.0, -1.0, f32::NAN, f32::INFINITY, 0.25, 0.75);
        assert_eq!(created.dopamine, MAX_LEVEL);
        assert_eq!(created.serotonin, MIN_LEVEL);
        assert_eq!(created.oxytocin, BASELINE_LEVELS[2], "NaN 分量回落基线");
        assert_eq!(created.cortisol, BASELINE_LEVELS[3], "∞ 分量回落基线");
        assert_eq!(created.noradrenaline, 0.25);
        assert_eq!(created.acetylcholine, 0.75);
        assert!(created.is_within(MIN_LEVEL, MAX_LEVEL));
    }

    #[test]
    fn vector_set_ignores_non_finite_and_clamps() {
        let mut vector = NeuromodulatorVector::baseline();
        vector.set(NeuromodulatorKind::Dopamine, 0.75);
        assert_eq!(vector.get(NeuromodulatorKind::Dopamine), 0.75);
        vector.set(NeuromodulatorKind::Dopamine, f32::NAN);
        assert_eq!(vector.get(NeuromodulatorKind::Dopamine), 0.75);
        vector.set(NeuromodulatorKind::Dopamine, 9.0);
        assert_eq!(vector.get(NeuromodulatorKind::Dopamine), MAX_LEVEL);
        vector.set(NeuromodulatorKind::Dopamine, -9.0);
        assert_eq!(vector.get(NeuromodulatorKind::Dopamine), MIN_LEVEL);
        assert_eq!(vector[NeuromodulatorKind::Dopamine], MIN_LEVEL);
    }

    #[test]
    fn reward_prediction_error_is_signed_sanitized_and_clamped() {
        let positive = NeuromodulatorEvent::RewardPredictionError {
            expected: 0.2,
            actual: 0.8,
        };
        let rpe = positive
            .reward_prediction_error()
            .expect("RPE 事件必须返回 Some");
        assert!(approx(rpe, 0.6), "RPE 应等于 actual - expected, 实得 {rpe}");

        let negative = NeuromodulatorEvent::RewardPredictionError {
            expected: 0.8,
            actual: 0.2,
        };
        let rpe = negative
            .reward_prediction_error()
            .expect("RPE 事件必须返回 Some");
        assert!(approx(rpe, -0.6), "负 RPE, 实得 {rpe}");

        let nan = NeuromodulatorEvent::RewardPredictionError {
            expected: f32::NAN,
            actual: f32::INFINITY,
        };
        assert_eq!(nan.reward_prediction_error(), Some(0.0));

        let saturated = NeuromodulatorEvent::RewardPredictionError {
            expected: -3.0e38,
            actual: 3.0e38,
        };
        assert_eq!(saturated.reward_prediction_error(), Some(1.0));

        assert_eq!(
            NeuromodulatorEvent::TaskSuccess.reward_prediction_error(),
            None
        );
    }

    #[test]
    fn reward_prediction_error_moves_dopamine_in_the_right_direction() {
        let baseline = NeuromodulatorVector::baseline();

        let mut positive = default_state();
        positive.apply(
            &NeuromodulatorEvent::RewardPredictionError {
                expected: 0.2,
                actual: 0.8,
            },
            0,
        );
        assert!(positive.vector().dopamine > baseline.dopamine);

        let mut negative = default_state();
        negative.apply(
            &NeuromodulatorEvent::RewardPredictionError {
                expected: 0.8,
                actual: 0.2,
            },
            0,
        );
        assert!(negative.vector().dopamine < baseline.dopamine);

        let mut strong = default_state();
        strong.apply(
            &NeuromodulatorEvent::RewardPredictionError {
                expected: 0.0,
                actual: 1.0,
            },
            0,
        );
        let mut weak = default_state();
        weak.apply(
            &NeuromodulatorEvent::RewardPredictionError {
                expected: 0.45,
                actual: 0.55,
            },
            0,
        );
        assert!(
            strong.vector().dopamine > weak.vector().dopamine,
            "|RPE| 越大, 多巴胺脉冲越大"
        );
        assert!(
            strong.vector().acetylcholine > baseline.acetylcholine,
            "显著 RPE 应抬乙酰胆碱 (学习增益)"
        );
    }

    #[test]
    fn task_outcomes_move_dopamine_serotonin_and_cortisol() {
        let baseline = NeuromodulatorVector::baseline();

        let mut success = default_state();
        success.apply(&NeuromodulatorEvent::TaskSuccess, 0);
        let v = success.vector();
        assert!(v.dopamine > baseline.dopamine);
        assert!(v.serotonin > baseline.serotonin);
        assert!(v.cortisol < baseline.cortisol);

        let mut failure = default_state();
        failure.apply(&NeuromodulatorEvent::TaskFailure, 0);
        let v = failure.vector();
        assert!(v.dopamine < baseline.dopamine);
        assert!(v.serotonin < baseline.serotonin);
        assert!(v.cortisol > baseline.cortisol);
    }

    #[test]
    fn threat_raises_cortisol_and_noradrenaline_and_lowers_serotonin() {
        let baseline = NeuromodulatorVector::baseline();
        let mut state = default_state();
        state.apply(&NeuromodulatorEvent::Threat, 0);
        let v = state.vector();
        assert!(v.cortisol > baseline.cortisol);
        assert!(v.noradrenaline > baseline.noradrenaline);
        assert!(v.serotonin < baseline.serotonin);
        assert!(v.dopamine < baseline.dopamine);
    }

    #[test]
    fn social_feedback_moves_oxytocin_oppositely() {
        let baseline = NeuromodulatorVector::baseline();

        let mut positive = default_state();
        positive.apply(&NeuromodulatorEvent::SocialPositive, 0);
        assert!(positive.vector().oxytocin > baseline.oxytocin);
        assert!(positive.vector().cortisol < baseline.cortisol);

        let mut negative = default_state();
        negative.apply(&NeuromodulatorEvent::SocialNegative, 0);
        assert!(negative.vector().oxytocin < baseline.oxytocin);
        assert!(negative.vector().cortisol > baseline.cortisol);
    }

    #[test]
    fn novelty_correction_and_quiet_move_acetylcholine_and_arousal() {
        let baseline = NeuromodulatorVector::baseline();

        let mut novelty = default_state();
        novelty.apply(&NeuromodulatorEvent::Novelty, 0);
        assert!(novelty.vector().acetylcholine > baseline.acetylcholine);
        assert!(novelty.vector().dopamine > baseline.dopamine);
        assert!(novelty.vector().noradrenaline > baseline.noradrenaline);

        let mut correction = default_state();
        correction.apply(&NeuromodulatorEvent::UserCorrection, 0);
        assert!(correction.vector().acetylcholine > baseline.acetylcholine);
        assert!(correction.vector().cortisol > baseline.cortisol);

        let mut quiet = default_state();
        quiet.apply(&NeuromodulatorEvent::Quiet, 0);
        assert!(quiet.vector().noradrenaline < baseline.noradrenaline);
        assert!(quiet.vector().cortisol < baseline.cortisol);
        assert!(quiet.vector().serotonin > baseline.serotonin);
    }

    #[test]
    fn cortisol_coupling_suppresses_serotonin() {
        let mut coupled = default_state();
        let mut uncoupled = NeuromodulatorState::new(uncoupled_config());
        coupled.apply(&NeuromodulatorEvent::Threat, 0);
        uncoupled.apply(&NeuromodulatorEvent::Threat, 0);

        assert_eq!(
            coupled.vector().cortisol,
            uncoupled.vector().cortisol,
            "耦合矩阵没有指向皮质醇的项"
        );
        assert!(
            coupled.vector().serotonin < uncoupled.vector().serotonin,
            "皮质醇↑ → 血清素↓ 耦合必须生效"
        );
        assert_eq!(coupled.vector().oxytocin, uncoupled.vector().oxytocin);
    }

    #[test]
    fn dopamine_coupling_raises_acetylcholine() {
        let mut coupled = default_state();
        let mut uncoupled = NeuromodulatorState::new(uncoupled_config());
        coupled.apply(&NeuromodulatorEvent::Novelty, 0);
        uncoupled.apply(&NeuromodulatorEvent::Novelty, 0);

        assert_eq!(coupled.vector().dopamine, uncoupled.vector().dopamine);
        assert!(
            coupled.vector().acetylcholine > uncoupled.vector().acetylcholine,
            "多巴胺↑ → 乙酰胆碱↑ 耦合必须生效"
        );
        assert_eq!(coupled.vector().cortisol, uncoupled.vector().cortisol);
    }

    #[test]
    fn decay_converges_monotonically_to_baseline() {
        let mut state = default_state();
        state.apply(&NeuromodulatorEvent::Novelty, 0);
        let baseline = state.baseline_vector();
        assert!(state.vector().dopamine > baseline.dopamine);

        let mut previous = state.vector().l1_distance(&baseline);
        assert!(previous > 0.0, "事件后必须偏离基线");
        for at in [10_000u64, 60_000, 600_000, 3_600_000] {
            state.decay(at);
            let distance = state.vector().l1_distance(&baseline);
            assert!(distance < previous, "t={at}ms 应更接近基线");
            assert!(
                state.vector().dopamine >= baseline.dopamine,
                "高于基线的维度不应下穿基线"
            );
            previous = distance;
        }
        assert!(previous < 1e-4, "1 小时后应基本收敛, 实得 {previous}");
        assert!(approx(state.vector().dopamine, baseline.dopamine));
    }

    #[test]
    fn decay_is_idempotent_and_ignores_clock_rewind() {
        let mut state = default_state();
        state.apply(&NeuromodulatorEvent::SocialPositive, 1_000);
        state.decay(5_000);
        let settled = state.vector();

        state.decay(5_000);
        assert_eq!(state.vector(), settled, "同一时间戳重复衰减必须幂等");

        state.decay(3_000);
        assert_eq!(state.vector(), settled, "时间回拨按 0 流逝处理");
        assert_eq!(state.last_update_ms(), 5_000, "逻辑时钟不回退");

        state.decay(5_001);
        assert_ne!(state.vector(), settled, "1ms 也应产生衰减");
    }

    #[test]
    fn repeated_events_accumulate_then_clamp() {
        let mut state = default_state();
        let event = NeuromodulatorEvent::SocialPositive;
        let baseline = state.vector();

        state.apply(&event, 0);
        let once = state.vector();
        assert!(once.oxytocin > baseline.oxytocin);

        state.apply(&event, 0);
        let twice = state.vector();
        assert!(
            twice.oxytocin > once.oxytocin,
            "同一事件两次应累计, 不是严格幂等"
        );

        for _ in 0..100 {
            state.apply(&event, 0);
        }
        assert!(state.vector().oxytocin <= MAX_LEVEL);
        assert_eq!(
            state.vector().oxytocin,
            MAX_LEVEL,
            "累计后应顶到 clamp 上界"
        );
        assert!(state.vector().is_within(MIN_LEVEL, MAX_LEVEL));
    }

    #[test]
    fn nan_and_infinite_inputs_never_escape() {
        let bad_config = NeuromodulatorConfig {
            decay_per_ms: [f32::NAN, -1.0, f32::INFINITY, 0.0, 0.0, 1.0e-5],
            event_gains: [
                f32::NAN,
                -2.0,
                1.0e30,
                f32::INFINITY,
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
            ],
            coupling: [[f32::NAN; DIMENSION_COUNT]; DIMENSION_COUNT],
            min_level: 2.0,
            max_level: -1.0,
        };
        let mut state = NeuromodulatorState::new(bad_config);

        assert!(state.vector().is_within(MIN_LEVEL, MAX_LEVEL));
        assert_eq!(
            state.config().min_level,
            MIN_LEVEL,
            "倒置越界边界应回落 [0,1]"
        );
        assert_eq!(state.config().max_level, MAX_LEVEL);
        assert!(state
            .config()
            .decay_per_ms
            .iter()
            .all(|rate| rate.is_finite() && *rate >= 0.0));
        assert!(state
            .config()
            .event_gains
            .iter()
            .all(|gain| gain.is_finite() && *gain >= 0.0 && *gain <= MAX_EVENT_GAIN));
        assert!(state
            .config()
            .coupling
            .iter()
            .flatten()
            .all(|coefficient| coefficient.is_finite()));

        state.apply(
            &NeuromodulatorEvent::RewardPredictionError {
                expected: f32::NAN,
                actual: f32::INFINITY,
            },
            1_000_000,
        );
        state.apply(&NeuromodulatorEvent::TaskSuccess, 2_000_000);
        assert!(state.vector().is_within(MIN_LEVEL, MAX_LEVEL));
        assert!(state.vector().all_finite());
        assert_params_legal(&state.behavior_params());

        let delta =
            NeuromodulatorDelta::new(f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 5.0, -5.0, 0.25);
        assert_eq!(delta.dopamine, 0.0);
        assert_eq!(delta.serotonin, 0.0);
        assert_eq!(delta.oxytocin, 0.0);
        assert_eq!(delta.cortisol, 1.0);
        assert_eq!(delta.noradrenaline, -1.0);
        assert_eq!(delta.acetylcholine, 0.25);
    }

    #[test]
    fn behavior_params_stay_in_legal_ranges_at_extremes() {
        let mut state = default_state();
        for _ in 0..200 {
            state.apply(&NeuromodulatorEvent::Threat, 0);
        }
        assert_params_legal(&state.behavior_params());

        let mut state = default_state();
        for _ in 0..200 {
            state.apply(&NeuromodulatorEvent::SocialPositive, 0);
        }
        assert_params_legal(&state.behavior_params());

        let mut state = default_state();
        for _ in 0..200 {
            state.apply(&NeuromodulatorEvent::Quiet, 0);
        }
        assert_params_legal(&state.behavior_params());

        assert_params_legal(&BehaviorParams::from_vector(
            &NeuromodulatorVector::from_array([0.0; DIMENSION_COUNT]),
        ));
        assert_params_legal(&BehaviorParams::from_vector(
            &NeuromodulatorVector::from_array([1.0; DIMENSION_COUNT]),
        ));
        assert_params_legal(&BehaviorParams::from_vector(
            &NeuromodulatorVector::from_array([
                f32::NAN,
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::NAN,
                0.5,
                0.5,
            ]),
        ));
        assert_params_legal(&BehaviorParams::baseline());
    }

    #[test]
    fn behavior_mapping_directions_are_intuitive() {
        let baseline = BehaviorParams::baseline();

        let mut positive = default_state();
        positive.apply(&NeuromodulatorEvent::SocialPositive, 0);
        let p = positive.behavior_params();
        assert!(p.warmth > baseline.warmth, "社交正反馈应更温暖");
        assert!(
            p.patience_factor > baseline.patience_factor,
            "社交正反馈应更有耐心"
        );
        assert!(p.verbosity > baseline.verbosity);
        assert!(
            p.caution_threshold > baseline.caution_threshold,
            "血清素/催产素上升应提高警觉阈值 (更不易触发谨慎)"
        );

        let mut threat = default_state();
        threat.apply(&NeuromodulatorEvent::Threat, 0);
        let t = threat.behavior_params();
        assert!(t.warmth < baseline.warmth);
        assert!(t.patience_factor < baseline.patience_factor);
        assert!(
            t.caution_threshold < baseline.caution_threshold,
            "压力/唤醒应降低警觉阈值 (更易触发谨慎)"
        );
        assert!(t.verbosity < baseline.verbosity);

        let mut reward = default_state();
        reward.apply(
            &NeuromodulatorEvent::RewardPredictionError {
                expected: 0.0,
                actual: 0.8,
            },
            0,
        );
        let r = reward.behavior_params();
        assert!(r.curiosity_gain > baseline.curiosity_gain);
        assert!(r.initiative_frequency > baseline.initiative_frequency);
    }

    #[test]
    fn behavior_params_baseline_mapping_matches_documented_formulas() {
        let params = BehaviorParams::from_vector(&NeuromodulatorVector::baseline());
        assert!(approx(params.warmth, 0.555), "warmth={}", params.warmth);
        assert!(
            approx(params.patience_factor, 1.135),
            "patience={}",
            params.patience_factor
        );
        assert!(
            approx(params.initiative_frequency, 0.87),
            "initiative={}",
            params.initiative_frequency
        );
        assert!(
            approx(params.curiosity_gain, 0.49),
            "curiosity={}",
            params.curiosity_gain
        );
        assert!(
            approx(params.caution_threshold, 0.505),
            "caution={}",
            params.caution_threshold
        );
        assert!(
            approx(params.verbosity, 0.5375),
            "verbosity={}",
            params.verbosity
        );
        assert_eq!(params, BehaviorParams::baseline());
    }

    #[test]
    fn identical_inputs_produce_identical_outputs() {
        let events = [
            (NeuromodulatorEvent::TaskSuccess, 100u64),
            (NeuromodulatorEvent::Novelty, 5_000),
            (NeuromodulatorEvent::Threat, 9_000),
            (NeuromodulatorEvent::SocialPositive, 12_000),
            (NeuromodulatorEvent::Quiet, 60_000),
        ];
        let run = || {
            let mut state = default_state();
            for &(event, at) in &events {
                state.apply(&event, at);
            }
            state.decay(120_000);
            (
                state.vector(),
                state.baseline_vector(),
                state.behavior_params(),
                state.last_update_ms(),
            )
        };
        let first = run();
        let second = run();
        assert_eq!(first, second, "同样输入序列必须逐位一致");
    }

    #[test]
    fn serde_roundtrip_for_public_types() {
        let vector = NeuromodulatorVector::new(0.1, 0.2, 0.3, 0.4, 0.5, 0.6);
        let json = serde_json::to_string(&vector).expect("vector 序列化");
        let back: NeuromodulatorVector = serde_json::from_str(&json).expect("vector 反序列化");
        assert_eq!(vector, back);

        let event = NeuromodulatorEvent::RewardPredictionError {
            expected: 0.25,
            actual: 0.75,
        };
        let json = serde_json::to_string(&event).expect("event 序列化");
        assert!(json.contains("reward_prediction_error"));
        let back: NeuromodulatorEvent = serde_json::from_str(&json).expect("event 反序列化");
        assert_eq!(event, back);

        let config = NeuromodulatorConfig::default();
        let json = serde_json::to_string(&config).expect("config 序列化");
        let back: NeuromodulatorConfig = serde_json::from_str(&json).expect("config 反序列化");
        assert_eq!(config, back);

        let params = BehaviorParams::from_vector(&vector);
        let json = serde_json::to_string(&params).expect("params 序列化");
        let back: BehaviorParams = serde_json::from_str(&json).expect("params 反序列化");
        assert_eq!(params, back);

        for kind in NeuromodulatorKind::ALL {
            let json = serde_json::to_string(&kind).expect("kind 序列化");
            let back: NeuromodulatorKind = serde_json::from_str(&json).expect("kind 反序列化");
            assert_eq!(kind, back);
        }
    }

    #[test]
    fn event_indices_are_unique_and_gains_are_slot_addressed() {
        let events = [
            NeuromodulatorEvent::RewardPredictionError {
                expected: 0.0,
                actual: 0.0,
            },
            NeuromodulatorEvent::TaskSuccess,
            NeuromodulatorEvent::TaskFailure,
            NeuromodulatorEvent::SocialPositive,
            NeuromodulatorEvent::SocialNegative,
            NeuromodulatorEvent::Threat,
            NeuromodulatorEvent::Novelty,
            NeuromodulatorEvent::Quiet,
            NeuromodulatorEvent::UserCorrection,
        ];
        assert_eq!(events.len(), NeuromodulatorEvent::KIND_COUNT);

        let mut seen = HashSet::new();
        for (expected, event) in events.iter().enumerate() {
            assert_eq!(event.index(), expected);
            assert!(seen.insert(event.index()), "事件下标必须唯一");
            assert!(!event.code().is_empty());
        }
        assert_eq!(seen.len(), NeuromodulatorEvent::KIND_COUNT);

        let mut config = NeuromodulatorConfig::default();
        config.event_gains[NeuromodulatorEvent::TaskSuccess.index()] = 0.0;
        let mut state = NeuromodulatorState::new(config);
        let before = state.vector();
        state.apply(&NeuromodulatorEvent::TaskSuccess, 1_000);
        assert_eq!(state.vector(), before, "增益 0 的事件不得改变状态");
        assert_eq!(state.last_update_ms(), 1_000, "但逻辑时钟仍推进");
    }
}
