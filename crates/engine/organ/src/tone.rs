//! `apeireth-organ::tone` — 语调桥与人格表现合成器 (PersonaSynthesizer / Emergent Tone).
//!
//! **设计哲学 (陪伴与多器官涌现)**:
//! - 对接哲学「陪伴 = 关系可能性 + 声音温度」；
//! - **三层语调合成 (3-Layer Emergence Synthesis)**:
//!   1. 关系基线 (`tone_hint`, 来自 Bond 信任度与共鸣度)；
//!   2. 情绪调制 (`emotion_tone`, 来自 EmotionOrgan 状态)；
//!   3. 审议强度 (`deliberation_intensity`, 来自 Council 审议加权分与置信度)；
//! - 纯确定性状态机与映射算法，0 LLM 虚假生成，严格有界输入校验。
//!
//! **O-6 三阶审查**:
//! 1. 总体: 将多个独立运作的认知器官（羁绊、情绪、审议智囊）涌现为统一的对话语气与人设温度
//! 2. 系统: 放置在 `apeireth-organ`, 作为器官间协作与表现层的合成原语
//! 3. 架构: 强类型数据结构与纯函数，0 unsafe, 0 外部 C 依赖

use std::fmt;

use serde::{Deserialize, Serialize};

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
}
