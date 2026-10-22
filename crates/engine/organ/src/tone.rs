//! `apeireth-organ::tone` — 语调桥与人格表现合成器 (PersonaSynthesizer / Emergent Tone).
//!
//! **设计哲学 (陪伴与多器官涌现)**:
//! - 对接哲学「陪伴 = 关系可能性 + 声音温度」；
//! - **三层语调合成 (3-Layer Emergence Synthesis)**:
//!   1. 关系基线 (`tone_hint`, 来自 Bond 信任度与共鸣度)；
//!   2. 情绪调制 (`emotion_tone`, 来自 EmotionOrgan 状态)；
//!   3. 审议强度 (`deliberation_intensity`, 来自 Council 审议加权分与置信度)；
//! - 纯确定性状态机与映射算法，0 LLM 虚假生成，严格有界输入校验。
//! - **调质调制 (W3.1 crate 内接线, 默认关)**: [`behavior_tone_style`] 把 SG-13
//!   [`BehaviorParams`] 六维映射为离散语气档位，[`tone_hint_with_params`] 在既有
//!   [`tone_hint`] 上可选叠加固定措辞片段；基线参数下逐字回退既有输出。装配面
//!   (runtime-assembly 注入 `NeuromodulatorState` 实例) 留给 W3.2，本批不宣称已接线。
//!
//! **O-6 三阶审查**:
//! 1. 总体: 将多个独立运作的认知器官（羁绊、情绪、审议智囊）涌现为统一的对话语气与人设温度
//! 2. 系统: 放置在 `apeireth-organ`, 作为器官间协作与表现层的合成原语
//! 3. 架构: 强类型数据结构与纯函数，0 unsafe, 0 外部 C 依赖

use std::fmt;

use apeireth_orchestration::self_tuning::TunableParam;
use serde::{Deserialize, Serialize};

use crate::neuromodulation::BehaviorParams;

/// 情绪注入 EMA 混合项基线系数 (基线 `BondCharacter::apply_emotion` 写死的
/// resonance 情绪项 0.1) —— 体验旋钮 [`TunableParam::ToneSaturation`] 的接线点:
/// 生效系数 = 本值 × 饱和倍率 (1.0 = 现行为, 零变化)。
pub const EMOTION_MIX_RESONANCE: f64 = 0.1;

/// trust 情绪项基线系数 (基线写死的 0.2; 与 [`EMOTION_MIX_RESONANCE`] 同受
/// [`TunableParam::ToneSaturation`] 缩放)。
pub const EMOTION_MIX_TRUST: f64 = 0.2;

/// 关系特征快照 (用于语调基线判定).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BondCharacterSnapshot {
    /// 信任度 `[0.0, 1.0]`
    pub trust: f64,
    /// 共鸣度 `[0.0, 1.0]`
    pub resonance: f64,
    /// 互依度 `[0.0, 1.0]`
    pub interdependency: f64,
    /// 韧性 `[0.0, 1.0]`
    pub resilience: f64,
    /// 创造性 `[0.0, 1.0]`
    pub creativity: f64,
}

impl Default for BondCharacterSnapshot {
    fn default() -> Self {
        Self {
            trust: 0.2,
            resonance: 0.2,
            interdependency: 0.1,
            resilience: 0.5,
            creativity: 0.2,
        }
    }
}

impl BondCharacterSnapshot {
    /// Plutchik-style emotion injection into relational character (baseline `BondCharacter::apply_emotion`).
    ///
    /// Joy / trust / anticipation raise resonance (EMA 0.7 / 0.1 mix). Trust
    /// itself is EMA 0.8 / 0.2. Fear / surprise / sadness / disgust / anger
    /// are accepted for API parity and currently unused (baseline discarded them
    /// the same way). Does **not** own `memory::partner::Bond` — this is a
    /// tone-layer snapshot helper.
    ///
    /// **语气情绪饱和度** (体验旋钮 [`TunableParam::ToneSaturation`], env
    /// `APEIRETH_TUNE_TONE_SATURATION`): 情绪注入 EMA 混合项的饱和倍率 ——
    /// 即基线写死的情绪项权重 (resonance 情绪项 0.1 / trust 情绪项 0.2) 的
    /// 缩放系数。1.0 = 现行为 (零变化); 0 = 情绪不入语气表征 (纯关系基线);
    /// 2 = 双倍情绪浓度。非法值回默认 1.0。
    #[allow(clippy::too_many_arguments)]
    pub fn apply_emotion(
        &mut self,
        joy: f64,
        trust: f64,
        fear: f64,
        surprise: f64,
        sadness: f64,
        disgust: f64,
        anger: f64,
        anticipation: f64,
    ) {
        let saturation = TunableParam::ToneSaturation.effective();
        self.resonance = (self.resonance * 0.7
            + (joy + trust + anticipation) * EMOTION_MIX_RESONANCE * saturation)
            .clamp(0.0, 1.0);
        self.trust = (self.trust * 0.8 + trust * EMOTION_MIX_TRUST * saturation).clamp(0.0, 1.0);
        let _ = (fear, surprise, sadness, disgust, anger);
    }
}

/// 情绪风格枚举 (7 档全覆盖).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmotionToneStyle {
    /// 明朗温暖
    Warm,
    /// 轻松随和
    Friendly,
    /// 轻柔舒缓
    Gentle,
    /// 沉稳谨慎
    Cautious,
    /// 平稳客观
    Diplomatic,
    /// 好奇探索
    Curious,
    /// 简洁专业
    Professional,
}

/// 审议结果回声 (来自 Council 智囊团审议结果).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DeliberationEcho {
    /// 智囊团加权总分 (归一化 `[-1.0, 1.0]`)
    pub weighted_score: f64,
    /// 综合置信度 `[0.0, 1.0]`
    pub confidence: f64,
}

/// 语调层的输入校验错误.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToneError {
    /// weighted_score 为 NaN 或超出 `[-1.0, 1.0]`
    InvalidScore,
    /// confidence 为 NaN 或超出 `[0.0, 1.0]`
    InvalidConfidence,
}

impl fmt::Display for ToneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidScore => write!(f, "审议加权分非法: 应在 [-1.0, 1.0] 范围内且非 NaN"),
            Self::InvalidConfidence => write!(f, "审议置信度非法: 应在 [0.0, 1.0] 范围内且非 NaN"),
        }
    }
}

impl std::error::Error for ToneError {}

/// 关系基线 → 语调提示 (确定性映射).
pub fn tone_hint(character: &BondCharacterSnapshot) -> &'static str {
    if character.trust >= 0.6 && character.resonance >= 0.6 {
        "轻松亲切，像老朋友一样自然"
    } else if character.trust >= 0.4 {
        "温暖自然，带一点熟稔"
    } else if character.resonance >= 0.4 {
        "温和关切，保持合适的分寸"
    } else {
        "礼貌克制，谨慎而友好"
    }
}

/// 情绪风格 → 语气措辞 (确定性映射, 7 档全覆盖).
pub fn emotion_tone(style: EmotionToneStyle) -> &'static str {
    match style {
        EmotionToneStyle::Warm => "明朗温暖，情绪自然流动",
        EmotionToneStyle::Friendly => "轻松友好，像近邻般随和",
        EmotionToneStyle::Gentle => "轻柔舒缓，带着关照",
        EmotionToneStyle::Cautious => "沉稳谨慎，字斟句酌",
        EmotionToneStyle::Diplomatic => "平稳客观，不走极端",
        EmotionToneStyle::Curious => "好奇探索，喜欢追问",
        EmotionToneStyle::Professional => "简洁专业，情绪收敛",
    }
}

/// 审议加权分 → 措辞强度 (确定性映射).
pub fn deliberation_intensity(echo: DeliberationEcho) -> Result<&'static str, ToneError> {
    if !echo.weighted_score.is_finite() || echo.weighted_score < -1.0 || echo.weighted_score > 1.0 {
        return Err(ToneError::InvalidScore);
    }
    if !echo.confidence.is_finite() || echo.confidence < 0.0 || echo.confidence > 1.0 {
        return Err(ToneError::InvalidConfidence);
    }

    if echo.weighted_score >= 0.5 && echo.confidence >= 0.6 {
        Ok("共识强烈，语气坚定明确")
    } else if echo.weighted_score >= 0.0 {
        Ok("倾向支持，语气从容平和")
    } else if echo.weighted_score > -0.5 {
        Ok("存在保留，语气留有余地")
    } else {
        Ok("分歧明显，语气谨慎克制")
    }
}

/// 三层语调综合合成 (关系基线 + 情绪调制 + 审议强度).
pub fn organ_tone(
    character: &BondCharacterSnapshot,
    emotion: EmotionToneStyle,
    deliberation: Option<DeliberationEcho>,
) -> Result<String, ToneError> {
    let base = tone_hint(character);
    let emo = emotion_tone(emotion);

    if let Some(echo) = deliberation {
        let intensity = deliberation_intensity(echo)?;
        Ok(format!(
            "【表达语调】基线：{}；情绪：{}；决策倾向：{}。",
            base, emo, intensity
        ))
    } else {
        Ok(format!("【表达语调】基线：{}；情绪：{}。", base, emo))
    }
}

// ============================================
// 调质调制 (SG-13 → 语气层, W3.1 crate 内接线; 默认关)
// ============================================

/// 语气温度档 (声明顺序 = 温度升序, `Ord` 与语义一致).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WarmthLevel {
    /// 克制中性 (`warmth < 0.40`).
    Cool,
    /// 温暖自然 (`0.40 <= warmth < 0.70`; 基线 `0.555` 落在此档).
    Warm,
    /// 更暖 (`warmth >= 0.70`).
    Affectionate,
}

/// 表达详尽度档 (声明顺序 = 详尽度升序).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerbosityLevel {
    /// 更简洁 (`verbosity < 0.35`).
    Terse,
    /// 适中 (`0.35 <= verbosity < 0.70`; 基线 `0.5375` 落在此档).
    Balanced,
    /// 更详尽 (`verbosity >= 0.70`).
    Detailed,
}

/// 谨慎档 (声明顺序 = 风险容忍度升序; **注意 `caution_threshold` 值越低越谨慎**).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CautionLevel {
    /// 更谨慎保守 (`caution_threshold <= 0.40`).
    Cautious,
    /// 适中 (`0.40 < caution_threshold <= 0.65`; 基线 `0.505` 落在此档).
    Measured,
    /// 更果断 (`caution_threshold > 0.65`).
    Bold,
}

/// 耐心档 (声明顺序 = 耐心升序).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatienceLevel {
    /// 更干脆 (`patience_factor < 0.85`).
    Brisk,
    /// 适中 (`0.85 <= patience_factor < 1.45`; 基线 `1.135` 落在此档).
    Steady,
    /// 更耐心 (`patience_factor >= 1.45`).
    Patient,
}

/// 主动性档 (声明顺序 = 主动频率升序).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InitiativeLevel {
    /// 更少主动 (`initiative_frequency < 0.60`).
    Reserved,
    /// 适中 (`0.60 <= initiative_frequency < 1.20`; 基线 `0.87` 落在此档).
    Balanced,
    /// 更主动 (`initiative_frequency >= 1.20`).
    Proactive,
}

/// 好奇档 (声明顺序 = 好奇增益升序).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CuriosityLevel {
    /// 少些追问 (`curiosity_gain < 0.30`).
    Low,
    /// 适中 (`0.30 <= curiosity_gain < 0.65`; 基线 `0.49` 落在此档).
    Moderate,
    /// 更爱追问 (`curiosity_gain >= 0.65`).
    High,
}

/// 调质语气风格: 六个离散档位 + 一个有限片段预算.
///
/// 由 [`behavior_tone_style`] 从 SG-13 的 [`BehaviorParams`] 确定性映射而来。**输出是
/// 结构化风格, 不是自由文本**: 六个枚举只承载档位, `clause_budget` 是有限数值
/// (`Terse` 1 / `Balanced` 3 / `Detailed` 6), 供 [`Self::tone_clauses`] 截断固定措辞
/// 片段表的长度。
///
/// # 档位映射 (输入先安全化: 非有限回基线, 有限值截断到字段合法区间)
///
/// - `warmth`: `< 0.40` → `Cool`, `< 0.70` → `Warm`, `>= 0.70` → `Affectionate`;
/// - `patience_factor`: `< 0.85` → `Brisk`, `< 1.45` → `Steady`, `>= 1.45` → `Patient`;
/// - `initiative_frequency`: `< 0.60` → `Reserved`, `< 1.20` → `Balanced`, `>= 1.20` → `Proactive`;
/// - `curiosity_gain`: `< 0.30` → `Low`, `< 0.65` → `Moderate`, `>= 0.65` → `High`;
/// - `caution_threshold` (**值越低越谨慎**): `<= 0.40` → `Cautious`, `<= 0.65` → `Measured`,
///   `> 0.65` → `Bold`;
/// - `verbosity`: `< 0.35` → `Terse`, `< 0.70` → `Balanced`, `>= 0.70` → `Detailed`.
///
/// 基线参数 (`BehaviorParams::baseline()`) 六维全部落在中间档, 等于 [`Self::baseline`]。
///
/// # 默认关 + 回退
///
/// 既有 [`tone_hint`] / [`emotion_tone`] / [`deliberation_intensity`] / [`organ_tone`]
/// **零改动**; 本类型只有被调用方显式消费时才生效。基线参数下
/// [`tone_hint_with_params`] 逐字回退 [`tone_hint`] (测试钉死)。调质是工程标签,
/// 不宣称主观情绪、意识或生理真实性。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BehaviorToneStyle {
    /// 温度档 (来自 `warmth`).
    pub warmth: WarmthLevel,
    /// 耐心档 (来自 `patience_factor`).
    pub patience: PatienceLevel,
    /// 主动性档 (来自 `initiative_frequency`).
    pub initiative: InitiativeLevel,
    /// 好奇档 (来自 `curiosity_gain`).
    pub curiosity: CuriosityLevel,
    /// 谨慎档 (来自 `caution_threshold`; 阈值越低越谨慎).
    pub caution: CautionLevel,
    /// 详尽度档 (来自 `verbosity`).
    pub verbosity: VerbosityLevel,
    /// 有限数值: 固定措辞片段预算, 取值 `1 | 3 | 6`, 由 `verbosity` 档位决定.
    ///
    /// 不生成任意文本, 只约束 [`Self::tone_clauses`] 从固定词表取多少条。
    pub clause_budget: u8,
}

impl BehaviorToneStyle {
    /// 基线风格: 六维全部中间档 + `Balanced` 预算。
    ///
    /// 与 `behavior_tone_style(&BehaviorParams::baseline())` 逐字段相等 (测试钉死),
    /// 是默认关 / 回退路径的参照点。
    pub fn baseline() -> Self {
        Self {
            warmth: WarmthLevel::Warm,
            patience: PatienceLevel::Steady,
            initiative: InitiativeLevel::Balanced,
            curiosity: CuriosityLevel::Moderate,
            caution: CautionLevel::Measured,
            verbosity: VerbosityLevel::Balanced,
            clause_budget: 3,
        }
    }

    /// 六维是否全部落在基线中间档。
    ///
    /// 只比较六个档位, 不看 [`Self::clause_budget`]: 手工构造的结构体即使预算不同,
    /// 只要六维都在中间档也返回 `true`。
    pub fn is_baseline(&self) -> bool {
        self.warmth == WarmthLevel::Warm
            && self.patience == PatienceLevel::Steady
            && self.initiative == InitiativeLevel::Balanced
            && self.curiosity == CuriosityLevel::Moderate
            && self.caution == CautionLevel::Measured
            && self.verbosity == VerbosityLevel::Balanced
    }

    /// 固定措辞片段: 按固定顺序取前 `clause_budget` 条, 基线风格返回空表。
    ///
    /// 顺序固定为: 详尽度 → 温度 → 谨慎 → 耐心 → 主动 → 好奇 (详尽度决定片段条数,
    /// 因此它自己的片段排在最前, 不会被自己的预算截掉)。片段全部来自固定词表, 不含
    /// 任何运行时生成的自由文本:
    ///
    /// - 详尽度: `更简洁` / `更详尽`;
    /// - 温度: `更克制中性` / `更温暖`;
    /// - 谨慎: `更谨慎` / `更果断`;
    /// - 耐心: `更耐心` / `更干脆`;
    /// - 主动: `更少主动` / `更主动`;
    /// - 好奇: `少些追问` / `更爱追问`。
    pub fn tone_clauses(&self) -> Vec<&'static str> {
        let mut clauses = Vec::new();
        match self.verbosity {
            VerbosityLevel::Terse => clauses.push("更简洁"),
            VerbosityLevel::Balanced => {}
            VerbosityLevel::Detailed => clauses.push("更详尽"),
        }
        match self.warmth {
            WarmthLevel::Cool => clauses.push("更克制中性"),
            WarmthLevel::Warm => {}
            WarmthLevel::Affectionate => clauses.push("更温暖"),
        }
        match self.caution {
            CautionLevel::Cautious => clauses.push("更谨慎"),
            CautionLevel::Measured => {}
            CautionLevel::Bold => clauses.push("更果断"),
        }
        match self.patience {
            PatienceLevel::Brisk => clauses.push("更干脆"),
            PatienceLevel::Steady => {}
            PatienceLevel::Patient => clauses.push("更耐心"),
        }
        match self.initiative {
            InitiativeLevel::Reserved => clauses.push("更少主动"),
            InitiativeLevel::Balanced => {}
            InitiativeLevel::Proactive => clauses.push("更主动"),
        }
        match self.curiosity {
            CuriosityLevel::Low => clauses.push("少些追问"),
            CuriosityLevel::Moderate => {}
            CuriosityLevel::High => clauses.push("更爱追问"),
        }
        clauses.truncate(usize::from(self.clause_budget));
        clauses
    }
}

/// 六维调质行为参数 → 离散语气风格 (确定性, 无 IO, 无随机, 无时钟)。
///
/// 输入先安全化: 非有限分量回落 [`BehaviorParams::baseline`] 对应值, 有限值截断到
/// 字段文档化的合法区间 (`UNIT_RANGE` / `PATIENCE_RANGE` / `INITIATIVE_RANGE` /
/// `CAUTION_RANGE`), 因此任意 `f32` 输入都返回合法结果且不 panic。档位映射表见
/// [`BehaviorToneStyle`] 文档。
///
/// # 方向 (测试钉死)
///
/// `warmth` ↑ → 更暖; `verbosity` ↑ → 更详尽且 `clause_budget` ↑; `caution_threshold` ↓
/// → 更谨慎保守; `patience_factor` ↑ → 更耐心; `initiative_frequency` ↑ → 更主动;
/// `curiosity_gain` ↑ → 更爱追问。
///
/// # 默认关 + 回退
///
/// 本函数是**新增纯函数**, 既有语气函数零改动; 基线参数输出 [`BehaviorToneStyle::baseline`]。
/// 装配面 (runtime-assembly 注入 `NeuromodulatorState` 并显式选调) 属 W3.2, 本批未接线。
pub fn behavior_tone_style(params: &BehaviorParams) -> BehaviorToneStyle {
    let baseline = BehaviorParams::baseline();
    let warmth = clamp_param(params.warmth, baseline.warmth, BehaviorParams::UNIT_RANGE);
    let patience = clamp_param(
        params.patience_factor,
        baseline.patience_factor,
        BehaviorParams::PATIENCE_RANGE,
    );
    let initiative = clamp_param(
        params.initiative_frequency,
        baseline.initiative_frequency,
        BehaviorParams::INITIATIVE_RANGE,
    );
    let curiosity = clamp_param(
        params.curiosity_gain,
        baseline.curiosity_gain,
        BehaviorParams::UNIT_RANGE,
    );
    let caution = clamp_param(
        params.caution_threshold,
        baseline.caution_threshold,
        BehaviorParams::CAUTION_RANGE,
    );
    let verbosity = clamp_param(
        params.verbosity,
        baseline.verbosity,
        BehaviorParams::UNIT_RANGE,
    );
    let verbosity_tier = verbosity_level(verbosity);

    BehaviorToneStyle {
        warmth: warmth_level(warmth),
        patience: patience_level(patience),
        initiative: initiative_level(initiative),
        curiosity: curiosity_level(curiosity),
        caution: caution_level(caution),
        verbosity: verbosity_tier,
        clause_budget: match verbosity_tier {
            VerbosityLevel::Terse => 1,
            VerbosityLevel::Balanced => 3,
            VerbosityLevel::Detailed => 6,
        },
    }
}

/// 既有 [`tone_hint`] 的可选叠加版: 关系基线 + 调质语气片段。
///
/// - **默认关 / 回退**: `params == BehaviorParams::baseline()` 时逐字返回 [`tone_hint`]
///   的结果 (含任意 `BondCharacterSnapshot`, 测试钉死); 调用方不使用本函数时行为零变化。
/// - **叠加**: 非基线参数时在既有基线上追加 `【调质语气】` + 固定片段 (最多
///   `clause_budget` 条, 顺序固定, 见 [`BehaviorToneStyle::tone_clauses`]), 不生成任意
///   文本、不改变既有 [`tone_hint`] / [`organ_tone`] 的签名与输出。
/// - **返回 `String`**: 需要拼接; 既有 `tone_hint` 保持 `&'static str` 零改动, 本函数是
///   与之并存的叠加版, 不是替换。
///
/// 本函数不读时钟 / 不持有状态 / 不做 IO; 调质是工程标签, 不宣称主观情绪。装配面
/// (runtime-assembly 提供 `NeuromodulatorState` 实例并注入) 待 W3.2, 本批仅 crate 内 API。
pub fn tone_hint_with_params(character: &BondCharacterSnapshot, params: &BehaviorParams) -> String {
    let base = tone_hint(character);
    let style = behavior_tone_style(params);
    let clauses = style.tone_clauses();
    if clauses.is_empty() {
        return base.to_string();
    }
    format!("{base}；【调质语气】{}", clauses.join("、"))
}

/// 调质参数安全化: 非有限回落基线, 有限值截断到字段合法区间。
fn clamp_param(value: f32, fallback: f32, range: (f32, f32)) -> f32 {
    if value.is_finite() {
        value.clamp(range.0, range.1)
    } else {
        fallback
    }
}

/// `warmth` → 温度档 (阈值即 [`BehaviorToneStyle`] 文档中的映射表).
fn warmth_level(value: f32) -> WarmthLevel {
    if value < 0.40 {
        WarmthLevel::Cool
    } else if value < 0.70 {
        WarmthLevel::Warm
    } else {
        WarmthLevel::Affectionate
    }
}

/// `verbosity` → 详尽度档.
fn verbosity_level(value: f32) -> VerbosityLevel {
    if value < 0.35 {
        VerbosityLevel::Terse
    } else if value < 0.70 {
        VerbosityLevel::Balanced
    } else {
        VerbosityLevel::Detailed
    }
}

/// `caution_threshold` → 谨慎档 (阈值越低越谨慎).
fn caution_level(value: f32) -> CautionLevel {
    if value <= 0.40 {
        CautionLevel::Cautious
    } else if value <= 0.65 {
        CautionLevel::Measured
    } else {
        CautionLevel::Bold
    }
}

/// `patience_factor` → 耐心档.
fn patience_level(value: f32) -> PatienceLevel {
    if value < 0.85 {
        PatienceLevel::Brisk
    } else if value < 1.45 {
        PatienceLevel::Steady
    } else {
        PatienceLevel::Patient
    }
}

/// `initiative_frequency` → 主动性档.
fn initiative_level(value: f32) -> InitiativeLevel {
    if value < 0.60 {
        InitiativeLevel::Reserved
    } else if value < 1.20 {
        InitiativeLevel::Balanced
    } else {
        InitiativeLevel::Proactive
    }
}

/// `curiosity_gain` → 好奇档.
fn curiosity_level(value: f32) -> CuriosityLevel {
    if value < 0.30 {
        CuriosityLevel::Low
    } else if value < 0.65 {
        CuriosityLevel::Moderate
    } else {
        CuriosityLevel::High
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tone_hint_thresholds() {
        let mut char_snapshot = BondCharacterSnapshot::default();
        assert_eq!(tone_hint(&char_snapshot), "礼貌克制，谨慎而友好");

        char_snapshot.trust = 0.5;
        assert_eq!(tone_hint(&char_snapshot), "温暖自然，带一点熟稔");

        char_snapshot.resonance = 0.7;
        char_snapshot.trust = 0.8;
        assert_eq!(tone_hint(&char_snapshot), "轻松亲切，像老朋友一样自然");
    }

    #[test]
    fn emotion_tone_covers_all_variants() {
        assert_eq!(
            emotion_tone(EmotionToneStyle::Warm),
            "明朗温暖，情绪自然流动"
        );
        assert_eq!(
            emotion_tone(EmotionToneStyle::Curious),
            "好奇探索，喜欢追问"
        );
        assert_eq!(
            emotion_tone(EmotionToneStyle::Professional),
            "简洁专业，情绪收敛"
        );
    }

    #[test]
    fn deliberation_intensity_validation_and_mapping() {
        let echo = DeliberationEcho {
            weighted_score: 0.8,
            confidence: 0.9,
        };
        assert_eq!(
            deliberation_intensity(echo).unwrap(),
            "共识强烈，语气坚定明确"
        );

        let invalid_echo = DeliberationEcho {
            weighted_score: f64::NAN,
            confidence: 0.5,
        };
        assert_eq!(
            deliberation_intensity(invalid_echo),
            Err(ToneError::InvalidScore)
        );
    }

    #[test]
    fn organ_tone_synthesizes_prompt() {
        let char_snapshot = BondCharacterSnapshot {
            trust: 0.7,
            resonance: 0.7,
            ..Default::default()
        };
        let echo = DeliberationEcho {
            weighted_score: 0.4,
            confidence: 0.8,
        };
        let prompt = organ_tone(&char_snapshot, EmotionToneStyle::Warm, Some(echo)).unwrap();
        assert!(prompt.contains("轻松亲切"));
        assert!(prompt.contains("明朗温暖"));
        assert!(prompt.contains("倾向支持"));
    }

    #[test]
    fn apply_emotion_raises_resonance_and_trust() {
        let mut snap = BondCharacterSnapshot {
            trust: 0.2,
            resonance: 0.2,
            ..Default::default()
        };
        snap.apply_emotion(1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
        assert!((snap.resonance - (0.2 * 0.7 + 0.3)).abs() < 1e-12);
        assert!((snap.trust - (0.2 * 0.8 + 0.2)).abs() < 1e-12);
        assert!(snap.resonance > 0.2 && snap.trust > 0.2);
    }

    #[test]
    fn apply_emotion_clamps() {
        let mut snap = BondCharacterSnapshot {
            trust: 1.0,
            resonance: 1.0,
            ..Default::default()
        };
        snap.apply_emotion(1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
        assert_eq!(snap.resonance, 1.0);
        assert_eq!(snap.trust, 1.0);
    }

    /// 旋钮推导锁定: 生效情绪混合项 = 基线系数 × ToneSaturation 倍率
    /// (未设旋钮 = 倍率 1.0 = 现行为, 零变化)。
    #[test]
    fn tone_saturation_knob_scales_emotion_mix() {
        apeireth_orchestration::self_tuning::clear_effective_overrides();
        let saturation = TunableParam::ToneSaturation.effective();
        let mut snap = BondCharacterSnapshot {
            trust: 0.2,
            resonance: 0.2,
            ..Default::default()
        };
        snap.apply_emotion(1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
        assert!(
            (snap.resonance - (0.2 * 0.7 + 3.0 * EMOTION_MIX_RESONANCE * saturation)).abs() < 1e-12,
            "resonance 情绪项必须按 基线系数×饱和倍率 缩放"
        );
        assert!(
            (snap.trust - (0.2 * 0.8 + 1.0 * EMOTION_MIX_TRUST * saturation)).abs() < 1e-12,
            "trust 情绪项必须按 基线系数×饱和倍率 缩放"
        );
        assert_eq!(
            TunableParam::ToneSaturation.parse_env_value(None),
            1.0,
            "未设旋钮 = 倍率 1.0 = 现状"
        );
    }

    // ============================================
    // 调质 → 语气 (W3.1 crate 内接线, 默认关)
    // ============================================

    /// 默认关 / 回退证据: 基线参数下叠加版与既有 `tone_hint` 逐字一致。
    #[test]
    fn baseline_params_keep_existing_tone_hint_verbatim() {
        let params = BehaviorParams::baseline();
        let snapshots = [
            BondCharacterSnapshot::default(),
            BondCharacterSnapshot {
                trust: 0.5,
                ..Default::default()
            },
            BondCharacterSnapshot {
                trust: 0.8,
                resonance: 0.7,
                ..Default::default()
            },
            BondCharacterSnapshot {
                resonance: 0.5,
                ..Default::default()
            },
        ];

        for snapshot in snapshots {
            assert_eq!(
                tone_hint_with_params(&snapshot, &params),
                tone_hint(&snapshot),
                "基线参数必须逐字回退既有 tone_hint"
            );
        }
        assert_eq!(behavior_tone_style(&params), BehaviorToneStyle::baseline());
        assert!(behavior_tone_style(&params).is_baseline());
    }

    /// `warmth` ↑ → 更暖 (方向断言 + 固定片段)。
    #[test]
    fn warmth_level_rises_with_warmth() {
        let baseline = BehaviorParams::baseline();
        let cool = behavior_tone_style(&BehaviorParams {
            warmth: 0.2,
            ..baseline
        });
        let warm = behavior_tone_style(&BehaviorParams {
            warmth: 0.55,
            ..baseline
        });
        let hot = behavior_tone_style(&BehaviorParams {
            warmth: 0.9,
            ..baseline
        });

        assert_eq!(cool.warmth, WarmthLevel::Cool);
        assert_eq!(warm.warmth, WarmthLevel::Warm);
        assert_eq!(hot.warmth, WarmthLevel::Affectionate);
        assert!(cool.warmth < warm.warmth && warm.warmth < hot.warmth);
        assert!(cool.tone_clauses().contains(&"更克制中性"));
        assert!(hot.tone_clauses().contains(&"更温暖"));
    }

    /// `verbosity` ↑ → 更详尽, 且有限片段预算 1 < 3 < 6。
    #[test]
    fn verbosity_level_and_clause_budget_rise_together() {
        let baseline = BehaviorParams::baseline();
        let terse = behavior_tone_style(&BehaviorParams {
            verbosity: 0.1,
            ..baseline
        });
        let balanced = behavior_tone_style(&baseline);
        let detailed = behavior_tone_style(&BehaviorParams {
            verbosity: 0.9,
            ..baseline
        });

        assert_eq!(terse.verbosity, VerbosityLevel::Terse);
        assert_eq!(balanced.verbosity, VerbosityLevel::Balanced);
        assert_eq!(detailed.verbosity, VerbosityLevel::Detailed);
        assert_eq!(
            (
                terse.clause_budget,
                balanced.clause_budget,
                detailed.clause_budget
            ),
            (1, 3, 6)
        );
        assert!(terse.clause_budget < balanced.clause_budget);
        assert!(balanced.clause_budget < detailed.clause_budget);
        assert_eq!(terse.tone_clauses(), vec!["更简洁"]);
        assert!(detailed.tone_clauses().contains(&"更详尽"));
    }

    /// `caution_threshold` ↓ → 更谨慎保守 (方向断言)。
    #[test]
    fn low_caution_threshold_is_more_cautious() {
        let baseline = BehaviorParams::baseline();
        let cautious = behavior_tone_style(&BehaviorParams {
            caution_threshold: 0.1,
            ..baseline
        });
        let measured = behavior_tone_style(&baseline);
        let bold = behavior_tone_style(&BehaviorParams {
            caution_threshold: 0.9,
            ..baseline
        });

        assert_eq!(cautious.caution, CautionLevel::Cautious);
        assert_eq!(measured.caution, CautionLevel::Measured);
        assert_eq!(bold.caution, CautionLevel::Bold);
        // `Ord` = 风险容忍度升序: 阈值越低 -> 档位越靠前 (越谨慎)。
        assert!(cautious.caution < measured.caution && measured.caution < bold.caution);
        assert!(cautious.tone_clauses().contains(&"更谨慎"));
        assert!(bold.tone_clauses().contains(&"更果断"));
    }

    /// `patience_factor` ↑ → 更耐心 (方向断言)。
    #[test]
    fn patience_level_rises_with_patience_factor() {
        let baseline = BehaviorParams::baseline();
        let brisk = behavior_tone_style(&BehaviorParams {
            patience_factor: 0.5,
            ..baseline
        });
        let steady = behavior_tone_style(&baseline);
        let patient = behavior_tone_style(&BehaviorParams {
            patience_factor: 2.0,
            ..baseline
        });

        assert_eq!(brisk.patience, PatienceLevel::Brisk);
        assert_eq!(steady.patience, PatienceLevel::Steady);
        assert_eq!(patient.patience, PatienceLevel::Patient);
        assert!(brisk.patience < steady.patience && steady.patience < patient.patience);
        assert!(brisk.tone_clauses().contains(&"更干脆"));
        assert!(patient.tone_clauses().contains(&"更耐心"));
    }

    /// `initiative_frequency` ↑ → 更主动; `curiosity_gain` ↑ → 更爱追问。
    #[test]
    fn initiative_and_curiosity_levels_follow_their_params() {
        let baseline = BehaviorParams::baseline();
        let reserved = behavior_tone_style(&BehaviorParams {
            initiative_frequency: 0.0,
            ..baseline
        });
        let proactive = behavior_tone_style(&BehaviorParams {
            initiative_frequency: 1.5,
            ..baseline
        });
        let low = behavior_tone_style(&BehaviorParams {
            curiosity_gain: 0.0,
            ..baseline
        });
        let high = behavior_tone_style(&BehaviorParams {
            curiosity_gain: 0.9,
            ..baseline
        });

        assert_eq!(reserved.initiative, InitiativeLevel::Reserved);
        assert_eq!(proactive.initiative, InitiativeLevel::Proactive);
        assert!(reserved.initiative < proactive.initiative);
        assert!(reserved.tone_clauses().contains(&"更少主动"));
        assert!(proactive.tone_clauses().contains(&"更主动"));

        assert_eq!(low.curiosity, CuriosityLevel::Low);
        assert_eq!(high.curiosity, CuriosityLevel::High);
        assert!(low.curiosity < high.curiosity);
        assert!(low.tone_clauses().contains(&"少些追问"));
        assert!(high.tone_clauses().contains(&"更爱追问"));
    }

    /// 越界 / 非有限输入: 非有限回基线, 有限值 clamp, 不 panic。
    #[test]
    fn out_of_range_and_non_finite_params_are_clamped_without_panic() {
        let wild = BehaviorParams {
            warmth: f32::NAN,
            patience_factor: f32::INFINITY,
            initiative_frequency: -1.0,
            curiosity_gain: 2.0,
            caution_threshold: f32::NEG_INFINITY,
            verbosity: 99.0,
        };
        let style = behavior_tone_style(&wild);

        // NaN / ±∞ 回落基线 0.555 / 0.505 -> 中间档; +∞ 的 patience 同样回基线。
        assert_eq!(style.warmth, WarmthLevel::Warm);
        assert_eq!(style.caution, CautionLevel::Measured);
        assert_eq!(style.patience, PatienceLevel::Steady);
        // -1.0 截断到 0.0 -> 更少主动。
        assert_eq!(style.initiative, InitiativeLevel::Reserved);
        // 2.0 截断到 1.0 -> 高好奇; 99.0 截断到 1.0 -> 更详尽。
        assert_eq!(style.curiosity, CuriosityLevel::High);
        assert_eq!(style.verbosity, VerbosityLevel::Detailed);
        assert_eq!(style.clause_budget, 6);

        // 边界值不应落在错误档位。
        let cautious_edge = behavior_tone_style(&BehaviorParams {
            caution_threshold: 0.40,
            ..BehaviorParams::baseline()
        });
        assert_eq!(cautious_edge.caution, CautionLevel::Cautious);
    }

    /// 调质内核 -> 行为参数 -> 语气风格: 联结抬温暖/耐心, 压力压温暖/耐心并抬谨慎。
    #[test]
    fn neuromodulator_derived_params_move_tone_in_expected_directions() {
        use crate::neuromodulation::NeuromodulatorVector;

        let mut bonding = NeuromodulatorVector::baseline();
        bonding.oxytocin = 1.0;
        bonding.serotonin = 1.0;
        let bonding_style = behavior_tone_style(&BehaviorParams::from_vector(&bonding));

        let mut threat = NeuromodulatorVector::baseline();
        threat.cortisol = 1.0;
        threat.noradrenaline = 1.0;
        let threat_style = behavior_tone_style(&BehaviorParams::from_vector(&threat));

        assert_eq!(bonding_style.warmth, WarmthLevel::Affectionate);
        assert_eq!(threat_style.caution, CautionLevel::Cautious);
        assert!(bonding_style.warmth > threat_style.warmth);
        assert!(bonding_style.patience > threat_style.patience);
        // 谨慎档 `Ord` = 风险容忍度升序: 压力侧 (Cautious) 比联结侧 (Measured) 更谨慎。
        assert!(threat_style.caution < bonding_style.caution);
    }

    /// 确定性 + 结构序列化 / 反序列化稳定。
    #[test]
    fn tone_style_is_deterministic_and_serializes_stably() {
        let params = BehaviorParams {
            warmth: 0.9,
            patience_factor: 1.8,
            initiative_frequency: 1.3,
            curiosity_gain: 0.8,
            caution_threshold: 0.2,
            verbosity: 0.8,
        };
        let first = behavior_tone_style(&params);
        let second = behavior_tone_style(&params);
        assert_eq!(first, second, "同一输入必须逐位稳定");
        assert_eq!(first.tone_clauses(), second.tone_clauses());

        let json = serde_json::to_string(&first).expect("风格结构必须可序列化");
        assert_eq!(
            json,
            r#"{"warmth":"affectionate","patience":"patient","initiative":"proactive","curiosity":"high","caution":"cautious","verbosity":"detailed","clause_budget":6}"#
        );
        let round_trip: BehaviorToneStyle =
            serde_json::from_str(&json).expect("风格结构必须可反序列化");
        assert_eq!(round_trip, first);

        let snapshot = BondCharacterSnapshot::default();
        assert_eq!(
            tone_hint_with_params(&snapshot, &params),
            tone_hint_with_params(&snapshot, &params),
            "叠加版输出必须确定"
        );
    }

    /// 片段只来自固定词表、数量不超过预算, 且叠加版保留既有基线前缀。
    #[test]
    fn tone_clauses_stay_within_budget_and_fixed_vocabulary() {
        const FIXED_CLAUSES: [&str; 12] = [
            "更简洁",
            "更详尽",
            "更克制中性",
            "更温暖",
            "更谨慎",
            "更果断",
            "更耐心",
            "更干脆",
            "更少主动",
            "更主动",
            "少些追问",
            "更爱追问",
        ];

        let params = BehaviorParams {
            warmth: 0.95,
            patience_factor: 2.0,
            initiative_frequency: 2.0,
            curiosity_gain: 1.0,
            caution_threshold: 0.05,
            verbosity: 0.95,
        };
        let style = behavior_tone_style(&params);
        let clauses = style.tone_clauses();

        assert_eq!(
            clauses,
            vec!["更详尽", "更温暖", "更谨慎", "更耐心", "更主动", "更爱追问"],
            "固定顺序: 详尽度 -> 温度 -> 谨慎 -> 耐心 -> 主动 -> 好奇"
        );
        assert_eq!(clauses.len(), usize::from(style.clause_budget));
        for clause in &clauses {
            assert!(FIXED_CLAUSES.contains(clause), "片段必须来自固定词表");
        }

        let snapshot = BondCharacterSnapshot::default();
        let rendered = tone_hint_with_params(&snapshot, &params);
        assert!(
            rendered.starts_with(tone_hint(&snapshot)),
            "叠加版必须保留既有基线前缀"
        );
        assert!(rendered.contains("【调质语气】"));
        assert!(rendered.contains("更详尽"));
    }
}
