//! `apeireth-perception::voice::emotion_voice` — 情感声学参数调制与语音情绪向导 (Firefly 模式 / 语调桥联动).
//!
//! ## 核心哲学 (S-1 北极星 + O-2 吸收 Firefly 标杆)
//! 机械的文本朗读缺乏生命力。本模块将 Apeireth 的认知情绪状态 (PAD 情感三维模型与 5 维性格特征)，
//! 确定性映射为 TTS 引擎可消费的底层连续声学特征参数：
//! - **音调 (Pitch)**: 喜悦/好奇时上升，悲伤/沉静时下降；
//! - **语速 (Speed)**: 兴奋/紧迫时加快，安抚/深思时放缓；
//! - **能量 (Energy / Volume)**: 情感共鸣时的响度与动态范围增益；
//! - **SSML 与情绪引导词**: 生成兼容 Edge-TTS、GPT-SoVITS 与 CosyVoice 的情感标记。
//!
//! ## 安全与纯粹性
//! - 纯 Safe Rust (`#![deny(unsafe_code)]`)，0 外部不可信 C-FFI 依赖；
//! - 边界收敛与数学 Clamp，防止极端情绪参数导致音频爆音或失真。

#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};

/// 主导情感分类 (TTS 渲染器使用).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmotionCategory {
    /// 欢欣 / 喜悦.
    Joy,
    /// 温暖 / 安抚 / 关怀.
    WarmCare,
    /// 好奇 / 探究.
    Curiosity,
    /// 专注 / 严谨.
    Focused,
    /// 疲惫 / 难过 / 低落.
    Sorrow,
    /// 中性 / 平静.
    Calm,
}

impl EmotionCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Joy => "cheerful",
            Self::WarmCare => "gentle",
            Self::Curiosity => "excited",
            Self::Focused => "serious",
            Self::Sorrow => "sad",
            Self::Calm => "calm",
        }
    }
}

/// 连续声学特征调制参数.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcousticParameters {
    /// 音调偏移 (半音单位 Semitones, 范围 [-6.0, +6.0]).
    pub pitch_semitones: f64,
    /// 语速倍率比 (范围 [0.70, 1.40], 1.0 为基准正常语速).
    pub speed_ratio: f64,
    /// 能量/音量分贝调整 (dB, 范围 [-6.0, +6.0]).
    pub volume_db: f64,
    /// 情感强度因子 (范围 [0.0, 1.0]).
    pub emotion_intensity: f64,
    /// 主导情感分类.
    pub primary_emotion: EmotionCategory,
}

impl Default for AcousticParameters {
    fn default() -> Self {
        Self {
            pitch_semitones: 0.0,
            speed_ratio: 1.0,
            volume_db: 0.0,
            emotion_intensity: 0.5,
            primary_emotion: EmotionCategory::Calm,
        }
    }
}

/// PAD 情感输入特征 (Pleasure, Arousal, Dominance).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PadEmotion {
    /// 愉悦度 [-1.0, +1.0] (负为沮丧，正为欢喜).
    pub pleasure: f64,
    /// 激活/兴奋度 [-1.0, +1.0] (负为困倦/平静，正为亢奋/紧张).
    pub arousal: f64,
    /// 支配/自信度 [-1.0, +1.0] (负为怯懦/遵从，正为坚定/果断).
    pub dominance: f64,
}

impl Default for PadEmotion {
    fn default() -> Self {
        Self {
            pleasure: 0.2, // 默认温和正面
            arousal: 0.0,
            dominance: 0.1,
        }
    }
}

/// 情感声学调制器.
pub struct EmotionVoiceSynthesizer;

impl EmotionVoiceSynthesizer {
    /// 根据 PAD 情绪与性格特征计算连续声学参数.
    pub fn compute_acoustic_params(pad: &PadEmotion, warmth: f64, liveliness: f64) -> AcousticParameters {
        let p = pad.pleasure.clamp(-1.0, 1.0);
        let a = pad.arousal.clamp(-1.0, 1.0);
        let _d = pad.dominance.clamp(-1.0, 1.0);
        let w = warmth.clamp(0.0, 1.0);
        let l = liveliness.clamp(0.0, 1.0);

        // 1. 计算音调: 愉悦度与活泼度提升音调，负愉悦与低激活降低音调
        let pitch = (p * 2.0 + a * 1.5 + (l - 0.5) * 1.5).clamp(-6.0, 6.0);

        // 2. 计算语速: 激活度越高语速越快，高温暖度适当放缓更加温柔
        let speed = (1.0 + a * 0.15 + (l - 0.5) * 0.1 - (w - 0.5) * 0.05).clamp(0.70, 1.40);

        // 3. 计算音量: 激活度提升音量
        let volume = (a * 2.5 + (l - 0.5) * 1.5).clamp(-6.0, 6.0);

        // 4. 判定主导情感分类
        let primary_emotion = if p > 0.4 && a > 0.2 {
            EmotionCategory::Joy
        } else if p > 0.1 && w > 0.6 {
            EmotionCategory::WarmCare
        } else if a > 0.4 {
            EmotionCategory::Curiosity
        } else if p < -0.3 {
            EmotionCategory::Sorrow
        } else if a < -0.2 && p.abs() < 0.3 {
            EmotionCategory::Calm
        } else {
            EmotionCategory::Focused
        };

        // 5. 情感强度
        let intensity = (p.abs().max(a.abs()) * 0.7 + (w - 0.5).abs() * 0.3).clamp(0.1, 1.0);

        AcousticParameters {
            pitch_semitones: pitch,
            speed_ratio: speed,
            volume_db: volume,
            emotion_intensity: intensity,
            primary_emotion,
        }
    }

    /// 将普通文本包裹为富情感 SSML (兼容微软 Edge-TTS / Azure Speech).
    pub fn wrap_ssml(text: &str, voice_name: &str, params: &AcousticParameters) -> String {
        let pitch_pct = (params.pitch_semitones * 5.0) as i32;
        let speed_pct = ((params.speed_ratio - 1.0) * 100.0) as i32;
        let volume_pct = (params.volume_db * 5.0) as i32;

        let style = params.primary_emotion.as_str();

        format!(
            r#"<speak version="1.0" xmlns="http://www.w3.org/2001/10/synthesis" xmlns:mstts="https://www.w3.org/2001/mstts" xml:lang="zh-CN"><voice name="{}"><mstts:express-as style="{}" styledegree="{:.1}"><prosody pitch="{:+}%" rate="{:+}%" volume="{:+}%">{}</prosody></mstts:express-as></voice></speak>"#,
            voice_name,
            style,
            params.emotion_intensity,
            pitch_pct,
            speed_pct,
            volume_pct,
            text
        )
    }
}

// ============================================================
// 单元测试集
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_acoustic_params_joy() {
        let pad = PadEmotion {
            pleasure: 0.8,
            arousal: 0.6,
            dominance: 0.4,
        };
        let params = EmotionVoiceSynthesizer::compute_acoustic_params(&pad, 0.7, 0.8);

        assert_eq!(params.primary_emotion, EmotionCategory::Joy);
        assert!(params.pitch_semitones > 1.5);
        assert!(params.speed_ratio > 1.0);
    }

    #[test]
    fn test_compute_acoustic_params_sorrow() {
        let pad = PadEmotion {
            pleasure: -0.8,
            arousal: -0.4,
            dominance: -0.5,
        };
        let params = EmotionVoiceSynthesizer::compute_acoustic_params(&pad, 0.3, 0.2);

        assert_eq!(params.primary_emotion, EmotionCategory::Sorrow);
        assert!(params.pitch_semitones < -1.0);
        assert!(params.speed_ratio < 1.0);
    }

    #[test]
    fn test_wrap_ssml_generates_valid_xml() {
        let params = AcousticParameters {
            pitch_semitones: 2.0,
            speed_ratio: 1.10,
            volume_db: 1.5,
            emotion_intensity: 0.8,
            primary_emotion: EmotionCategory::Joy,
        };

        let ssml = EmotionVoiceSynthesizer::wrap_ssml("主人，今天的工作全部圆满完成了！", "zh-CN-XiaoxiaoNeural", &params);
        assert!(ssml.starts_with("<speak"));
        assert!(ssml.contains("style=\"cheerful\""));
        assert!(ssml.contains("zh-CN-XiaoxiaoNeural"));
        assert!(ssml.contains("主人，今天的工作全部圆满完成了！"));
        assert!(ssml.ends_with("</speak>"));
    }
}
