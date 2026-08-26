//! # Voice 唤醒词 (per @anthropic-ai/voice v0.9.21 商业版 1:1 翻译)
//!
//! 唤醒词 (per v0.9.21 商业版 + R20 设计拍板):
//! 1. **Hardcoded** — 编译期 hardcode 唤醒词, 默认 `"apeireth"` (per R20 设计拍板, 1:1 翻译品牌一致)
//! 2. **Custom** — 用户自定义唤醒词字符串 (R21 续真接时估补)
//! 3. **Phonetic** — 音标匹配 (e.g. `[əˈpɪərɛθ]` 替代字符串, R21 续)
//! 4. **Semantic** — 语义匹配 (e.g. `"AI assistant"` 整段语义, R21 续)
//!
//! **STUB**: 4 类别 1:1 翻译, 但 detect_wake() 内部返 `VoiceError::NotImplemented`.

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::error::{VoiceError, VoiceResult};

// ============================================================================
// §1 编译期 hardcode 常量 (K-1 强校验 #1 品牌一致)
// ============================================================================

/// 默认唤醒词 (K-1 强校验 #1 品牌一致: 编译期 hardcode `"apeireth"`, 不写 HeySpectrAI).
///
/// 1:1 翻译 v0.9.21 商业版品牌一致 (R20 设计拍板).
pub const VOICE_DEFAULT_WAKE_WORD: &str = "apeireth";

/// 自定义唤醒词最大长度 (per v0.9.21 商业版估 64 char, 防恶意长串).
pub const MAX_CUSTOM_WAKE_WORD_LENGTH: usize = 64;

/// 唤醒词最小长度 (per v0.9.21 商业版估 3 char, 防过短误触).
pub const MIN_WAKE_WORD_LENGTH: usize = 3;

// ============================================================================
// §2 唤醒词类别 (4 variant, 1:1 翻译 @anthropic-ai/voice v0.9.21)
// ============================================================================

/// 唤醒词类别 (4 variant, 1:1 翻译 v0.9.21 商业版 `WakeWordCategory` enum).
///
/// 4 类别 snake_case 字符串严格匹配 v0.9.21 商业版 API 规范.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WakeWordCategory {
    /// **编译期 hardcode** 唤醒词 (默认 `"apeireth"`, per R20 设计拍板).
    #[default]
    Hardcoded,
    /// **用户自定义** 唤醒词字符串 (e.g. `"hey buddy"`, R21 续真接时用).
    Custom,
    /// **音标匹配** 唤醒词 (e.g. `[əˈpɪərɛθ]`, R21 续真接时估补).
    Phonetic,
    /// **语义匹配** 唤醒词 (e.g. `"AI assistant"`, R21 续真接时估补).
    Semantic,
}

impl WakeWordCategory {
    /// 4 类别 hardcode 常量.
    pub const COUNT: usize = 4;

    /// 字符串 (1:1 翻译 v0.9.21 商业版 `category` 字段, snake_case 严格匹配).
    pub fn as_str(&self) -> &'static str {
        match self {
            WakeWordCategory::Hardcoded => "hardcoded",
            WakeWordCategory::Custom => "custom",
            WakeWordCategory::Phonetic => "phonetic",
            WakeWordCategory::Semantic => "semantic",
        }
    }

    /// 从字符串解析 (per v0.9.21 商业版响应 `category` 字段).
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "hardcoded" => Some(WakeWordCategory::Hardcoded),
            "custom" => Some(WakeWordCategory::Custom),
            "phonetic" => Some(WakeWordCategory::Phonetic),
            "semantic" => Some(WakeWordCategory::Semantic),
            _ => None,
        }
    }

    /// 默认唤醒词 (per category, Hardcoded 返 "apeireth", 其他返 None).
    pub fn default_word(&self) -> Option<&'static str> {
        match self {
            WakeWordCategory::Hardcoded => Some(VOICE_DEFAULT_WAKE_WORD),
            _ => None, // Custom/Phonetic/Semantic 必须用户显式提供
        }
    }
}

impl std::fmt::Display for WakeWordCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 编译期守门: SUPPORTED_WAKE_WORD_CATEGORIES 长度 == 4 (K-1 强校验守门, 4 类别 hardcode).
pub const SUPPORTED_WAKE_WORD_CATEGORIES: &[WakeWordCategory] = &[
    WakeWordCategory::Hardcoded,
    WakeWordCategory::Custom,
    WakeWordCategory::Phonetic,
    WakeWordCategory::Semantic,
];
const _: () = assert!(SUPPORTED_WAKE_WORD_CATEGORIES.len() == 4);

// ============================================================================
// §3 WakeWord struct (per v0.9.21 商业版 `wake_word` 字段 1:1 翻译)
// ============================================================================

/// 唤醒词配置 (per v0.9.21 商业版 `wake_word` 字段 1:1 翻译).
///
/// 字段对应 v0.9.21 商业版 `WakeWordConfig` 对象:
/// - `category` (4 类别, 编译期 hardcode)
/// - `keyword` (字符串, 默认 `"apeireth"` per Hardcoded 类别)
/// - `sensitivity` (0.0..=1.0, per v0.9.21 商业版默认 0.5)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WakeWord {
    /// 类别 (编译期 hardcode 4 类别)
    pub category: WakeWordCategory,
    /// 关键词字符串 (Hardcoded 时默认 `"apeireth"`)
    pub keyword: String,
    /// 灵敏度 (0.0..=1.0, 默认 0.5)
    pub sensitivity: f32,
}

impl WakeWord {
    /// 创建默认 Hardcoded 唤醒词 `"apeireth"`.
    pub fn default_apeireth() -> Self {
        Self {
            category: WakeWordCategory::Hardcoded,
            keyword: VOICE_DEFAULT_WAKE_WORD.to_string(),
            sensitivity: 0.5,
        }
    }

    /// 创建自定义唤醒词 (Custom 类别).
    pub fn custom(keyword: String) -> VoiceResult<Self> {
        Self::validate_keyword(&keyword)?;
        Ok(Self {
            category: WakeWordCategory::Custom,
            keyword,
            sensitivity: 0.5,
        })
    }

    /// 创建音标唤醒词 (Phonetic 类别, R21 续真接时估补).
    pub fn phonetic(phonetic: String) -> VoiceResult<Self> {
        Self::validate_keyword(&phonetic)?;
        Ok(Self {
            category: WakeWordCategory::Phonetic,
            keyword: phonetic,
            sensitivity: 0.5,
        })
    }

    /// 创建语义唤醒词 (Semantic 类别, R21 续真接时估补).
    pub fn semantic(semantic: String) -> VoiceResult<Self> {
        Self::validate_keyword(&semantic)?;
        Ok(Self {
            category: WakeWordCategory::Semantic,
            keyword: semantic,
            sensitivity: 0.5,
        })
    }

    /// 校验唤醒词 (非空 + 长度 3..=64, per v0.9.21 商业版估).
    pub fn validate_keyword(keyword: &str) -> VoiceResult<()> {
        let trimmed = keyword.trim();
        if trimmed.is_empty() {
            return Err(VoiceError::Other("wake word is empty".to_string()));
        }
        if trimmed.len() < MIN_WAKE_WORD_LENGTH {
            return Err(VoiceError::Other(format!(
                "wake word too short: {} < {} chars",
                trimmed.len(),
                MIN_WAKE_WORD_LENGTH
            )));
        }
        if trimmed.len() > MAX_CUSTOM_WAKE_WORD_LENGTH {
            return Err(VoiceError::Other(format!(
                "wake word too long: {} > {} chars",
                trimmed.len(),
                MAX_CUSTOM_WAKE_WORD_LENGTH
            )));
        }
        Ok(())
    }

    /// 设置灵敏度 (0.0..=1.0).
    pub fn set_sensitivity(&mut self, sensitivity: f32) -> VoiceResult<()> {
        if !(0.0..=1.0).contains(&sensitivity) {
            return Err(VoiceError::Other(format!(
                "sensitivity {} out of range [0.0, 1.0]",
                sensitivity
            )));
        }
        self.sensitivity = sensitivity;
        Ok(())
    }
}

impl Default for WakeWord {
    fn default() -> Self {
        Self::default_apeireth()
    }
}

// ============================================================================
// §4 WakeWordDetection 唤醒词检测结果
// ============================================================================

/// 唤醒词检测结果 (per v0.9.21 商业版 `detect_wake` 响应 1:1 翻译).
///
/// 字段对应 v0.9.21 商业版 `WakeWordDetection` 对象:
/// - `category` (per `WakeWordCategory`)
/// - `keyword` (命中的关键词)
/// - `confidence` (0.0..=1.0, 检测置信度)
/// - `detected_at` (检测时间戳, SystemTime)
/// - `session_id` (触发的 audio session ID, R21 续真接时用)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WakeWordDetection {
    /// 类别
    pub category: WakeWordCategory,
    /// 命中的关键词
    pub keyword: String,
    /// 置信度 (0.0..=1.0)
    pub confidence: f32,
    /// 检测时间戳
    pub detected_at: SystemTime,
    /// 触发的 audio session ID (R21 续真接时由 detect_wake 返)
    pub session_id: Option<String>,
}

impl WakeWordDetection {
    /// 创建新的检测结果 (STUB 模式由调用方构造, R21 续真接时由 detect_wake 返).
    pub fn new(category: WakeWordCategory, keyword: String, confidence: f32) -> Self {
        Self {
            category,
            keyword,
            confidence,
            detected_at: SystemTime::now(),
            session_id: None,
        }
    }

    /// 检查是否命中默认唤醒词 `"apeireth"`.
    pub fn is_apeireth(&self) -> bool {
        self.keyword.eq_ignore_ascii_case(VOICE_DEFAULT_WAKE_WORD)
    }
}

// ============================================================================
// §5 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- §1 编译期 hardcode ----

    #[test]
    fn k1_default_wake_word_is_apeireth() {
        assert_eq!(VOICE_DEFAULT_WAKE_WORD, "apeireth", "K-1 强校验: 默认唤醒词必须是 'apeireth'");
    }

    #[test]
    fn k1_wake_word_length_bounds() {
        assert_eq!(MIN_WAKE_WORD_LENGTH, 3);
        assert_eq!(MAX_CUSTOM_WAKE_WORD_LENGTH, 64);
    }

    // ---- §2 4 WakeWordCategory 枚举守门 ----

    #[test]
    fn k1_wake_word_category_has_4_variants() {
        assert_eq!(SUPPORTED_WAKE_WORD_CATEGORIES.len(), 4, "K-1 强校验: 必须 4 个类别");
        assert_eq!(WakeWordCategory::COUNT, 4);
        assert_eq!(WakeWordCategory::Hardcoded.as_str(), "hardcoded");
        assert_eq!(WakeWordCategory::Custom.as_str(), "custom");
        assert_eq!(WakeWordCategory::Phonetic.as_str(), "phonetic");
        assert_eq!(WakeWordCategory::Semantic.as_str(), "semantic");
    }

    #[test]
    fn k1_wake_word_category_default_is_hardcoded() {
        let default = WakeWordCategory::default();
        assert_eq!(default, WakeWordCategory::Hardcoded);
    }

    #[test]
    fn k1_wake_word_category_default_word() {
        assert_eq!(
            WakeWordCategory::Hardcoded.default_word(),
            Some("apeireth")
        );
        assert_eq!(WakeWordCategory::Custom.default_word(), None);
        assert_eq!(WakeWordCategory::Phonetic.default_word(), None);
        assert_eq!(WakeWordCategory::Semantic.default_word(), None);
    }

    #[test]
    fn k1_wake_word_category_parse_roundtrip() {
        for cat in SUPPORTED_WAKE_WORD_CATEGORIES {
            assert_eq!(WakeWordCategory::parse(cat.as_str()), Some(*cat));
        }
        assert_eq!(WakeWordCategory::parse("unknown"), None);
    }

    // ---- §3 WakeWord ----

    #[test]
    fn k1_wake_word_default_is_apeireth() {
        let wake = WakeWord::default_apeireth();
        assert_eq!(wake.category, WakeWordCategory::Hardcoded);
        assert_eq!(wake.keyword, "apeireth");
        assert!((wake.sensitivity - 0.5).abs() < 0.001);
    }

    #[test]
    fn k1_wake_word_default_trait() {
        let wake = WakeWord::default();
        assert_eq!(wake, WakeWord::default_apeireth());
    }

    #[test]
    fn k1_wake_word_custom_valid() {
        let wake = WakeWord::custom("hey buddy".to_string()).expect("valid custom");
        assert_eq!(wake.category, WakeWordCategory::Custom);
        assert_eq!(wake.keyword, "hey buddy");
    }

    #[test]
    fn k1_wake_word_custom_rejects_empty() {
        let result = WakeWord::custom(String::new());
        assert!(matches!(result, Err(VoiceError::Other(_))));
    }

    #[test]
    fn k1_wake_word_custom_rejects_too_short() {
        let result = WakeWord::custom("ab".to_string());
        assert!(matches!(result, Err(VoiceError::Other(_))));
    }

    #[test]
    fn k1_wake_word_custom_rejects_too_long() {
        let long = "a".repeat(65);
        let result = WakeWord::custom(long);
        assert!(matches!(result, Err(VoiceError::Other(_))));
    }

    #[test]
    fn k1_wake_word_phonetic() {
        let wake = WakeWord::phonetic("[əˈpɪərɛθ]".to_string()).expect("valid phonetic");
        assert_eq!(wake.category, WakeWordCategory::Phonetic);
    }

    #[test]
    fn k1_wake_word_semantic() {
        let wake = WakeWord::semantic("AI assistant".to_string()).expect("valid semantic");
        assert_eq!(wake.category, WakeWordCategory::Semantic);
    }

    #[test]
    fn k1_wake_word_sensitivity_valid() {
        let mut wake = WakeWord::default_apeireth();
        assert!(wake.set_sensitivity(0.0).is_ok());
        assert!(wake.set_sensitivity(1.0).is_ok());
        assert!(wake.set_sensitivity(0.5).is_ok());
    }

    #[test]
    fn k1_wake_word_sensitivity_out_of_range() {
        let mut wake = WakeWord::default_apeireth();
        assert!(wake.set_sensitivity(-0.1).is_err());
        assert!(wake.set_sensitivity(1.1).is_err());
    }

    // ---- §4 WakeWordDetection ----

    #[test]
    fn k1_wake_word_detection_apeireth_check() {
        let det = WakeWordDetection::new(
            WakeWordCategory::Hardcoded,
            "apeireth".to_string(),
            0.95,
        );
        assert!(det.is_apeireth());

        let det_upper = WakeWordDetection::new(
            WakeWordCategory::Hardcoded,
            "APEIRETH".to_string(),
            0.95,
        );
        assert!(det_upper.is_apeireth());

        let det_other = WakeWordDetection::new(
            WakeWordCategory::Custom,
            "hey buddy".to_string(),
            0.95,
        );
        assert!(!det_other.is_apeireth());
    }

    #[test]
    fn k1_wake_word_detection_default_confidence() {
        let det = WakeWordDetection::new(
            WakeWordCategory::Hardcoded,
            "apeireth".to_string(),
            0.5,
        );
        assert_eq!(det.category, WakeWordCategory::Hardcoded);
        assert_eq!(det.keyword, "apeireth");
        assert!(det.session_id.is_none());
    }
}
