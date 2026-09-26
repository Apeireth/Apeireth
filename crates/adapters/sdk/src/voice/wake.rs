//! # Voice 唤醒词 (per 既有 Voice SDK)
//!
//! 唤醒词 (按既有实现 + R20 设计拍板):
//! 1. **Hardcoded** — 编译期 hardcode 唤醒词, 默认 `"apeireth"` (per R20 设计拍板, 1:1 翻译品牌一致)
//! 2. **Custom** — 用户自定义唤醒词字符串 (R21 续真接时估补)
//! 3. **Phonetic** — 音标匹配 (e.g. `[əˈpɪərɛθ]` 替代字符串, R21 续)
//! 4. **Semantic** — 语义匹配 (e.g. `"AI assistant"` 整段语义, R21 续)
//!
//! 领域模型 (4 类别 + 配置 / 检测结果) + 模板包络匹配检测引擎 (§6).

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::voice::error::{VoiceError, VoiceResult};

// ============================================================================
// §1 编译期 hardcode 常量 (K-1 强校验 #1 品牌一致)
// ============================================================================

/// 默认唤醒词 (K-1 强校验 #1 品牌一致: 编译期 hardcode `"apeireth"`).
///
/// 对齐既有实现 品牌一致 (R20 设计拍板).
pub const VOICE_DEFAULT_WAKE_WORD: &str = "apeireth";

/// 自定义唤醒词最大长度 (按既有实现估算 64 char, 防恶意长串).
pub const MAX_CUSTOM_WAKE_WORD_LENGTH: usize = 64;

/// 唤醒词最小长度 (按既有实现估算 3 char, 防过短误触).
pub const MIN_WAKE_WORD_LENGTH: usize = 3;

// ============================================================================
// §2 唤醒词类别 (4 variant, 1:1 翻译 既有 Voice SDK)
// ============================================================================

/// 唤醒词类别 (4 variant, 对齐既有实现 `WakeWordCategory` enum).
///
/// 4 类别 snake_case 字符串严格匹配 既有实现 API 规范.
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

    /// 字符串 (对齐既有实现 `category` 字段, snake_case 严格匹配).
    pub fn as_str(&self) -> &'static str {
        match self {
            WakeWordCategory::Hardcoded => "hardcoded",
            WakeWordCategory::Custom => "custom",
            WakeWordCategory::Phonetic => "phonetic",
            WakeWordCategory::Semantic => "semantic",
        }
    }

    /// 从字符串解析 (按既有实现响应 `category` 字段).
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
// §3 WakeWord struct (按既有实现 `wake_word` 字段)
// ============================================================================

/// 唤醒词配置 (按既有实现 `wake_word` 字段).
///
/// 字段对应既有实现 `WakeWordConfig` 对象:
/// - `category` (4 类别, 编译期 hardcode)
/// - `keyword` (字符串, 默认 `"apeireth"` per Hardcoded 类别)
/// - `sensitivity` (0.0..=1.0, 按既有实现默认 0.5)
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

    /// 校验唤醒词 (非空 + 长度 3..=64, 按既有实现估算).
    pub fn validate_keyword(keyword: &str) -> VoiceResult<()> {
        let trimmed = keyword.trim();
        if trimmed.is_empty() {
            return Err(VoiceError::InvalidArgument(
                "wake word is empty".to_string(),
            ));
        }
        if trimmed.len() < MIN_WAKE_WORD_LENGTH {
            return Err(VoiceError::InvalidArgument(format!(
                "wake word too short: {} < {} chars",
                trimmed.len(),
                MIN_WAKE_WORD_LENGTH
            )));
        }
        if trimmed.len() > MAX_CUSTOM_WAKE_WORD_LENGTH {
            return Err(VoiceError::InvalidArgument(format!(
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
            return Err(VoiceError::InvalidArgument(format!(
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

/// 唤醒词检测结果 (按既有实现 `detect_wake` 响应).
///
/// 字段对应既有实现 `WakeWordDetection` 对象:
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

    /// 是否达到唤醒判定: `confidence >= sensitivity` (置信度语义见 [`crate::voice::wake`]).
    pub fn is_wake(&self, sensitivity: f32) -> bool {
        self.confidence >= sensitivity
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
        assert_eq!(
            VOICE_DEFAULT_WAKE_WORD, "apeireth",
            "K-1 强校验: 默认唤醒词必须是 'apeireth'"
        );
    }

    #[test]
    fn k1_wake_word_length_bounds() {
        assert_eq!(MIN_WAKE_WORD_LENGTH, 3);
        assert_eq!(MAX_CUSTOM_WAKE_WORD_LENGTH, 64);
    }

    // ---- §2 4 WakeWordCategory 枚举守门 ----

    #[test]
    fn k1_wake_word_category_has_4_variants() {
        assert_eq!(
            SUPPORTED_WAKE_WORD_CATEGORIES.len(),
            4,
            "K-1 强校验: 必须 4 个类别"
        );
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
        assert_eq!(WakeWordCategory::Hardcoded.default_word(), Some("apeireth"));
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
        assert!(matches!(result, Err(VoiceError::InvalidArgument(_))));
    }

    #[test]
    fn k1_wake_word_custom_rejects_too_short() {
        let result = WakeWord::custom("ab".to_string());
        assert!(matches!(result, Err(VoiceError::InvalidArgument(_))));
    }

    #[test]
    fn k1_wake_word_custom_rejects_too_long() {
        let long = "a".repeat(65);
        let result = WakeWord::custom(long);
        assert!(matches!(result, Err(VoiceError::InvalidArgument(_))));
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
        let det = WakeWordDetection::new(WakeWordCategory::Hardcoded, "apeireth".to_string(), 0.95);
        assert!(det.is_apeireth());

        let det_upper =
            WakeWordDetection::new(WakeWordCategory::Hardcoded, "APEIRETH".to_string(), 0.95);
        assert!(det_upper.is_apeireth());

        let det_other =
            WakeWordDetection::new(WakeWordCategory::Custom, "hey buddy".to_string(), 0.95);
        assert!(!det_other.is_apeireth());
    }

    #[test]
    fn k1_wake_word_detection_default_confidence() {
        let det = WakeWordDetection::new(WakeWordCategory::Hardcoded, "apeireth".to_string(), 0.5);
        assert_eq!(det.category, WakeWordCategory::Hardcoded);
        assert_eq!(det.keyword, "apeireth");
        assert!(det.session_id.is_none());
    }
}

// ============================================================================
// §6 唤醒检测引擎 (模板包络匹配; 纯信号处理, 零外部模型)
// ============================================================================
//
// 算法语义 (如实文档化, 不夸大):
// 1. `extract_envelope`: PCM 分帧 → 帧 RMS 能量序列 → 峰值归一化到 0..=1。
// 2. `envelope_similarity`: 双方包络各自线性重采样到 ENVELOPE_BINS 个
//    采样点, 再算余弦相似度 (非负向量 → 0.0..=1.0)。
// 3. `WakeDetector::enroll` 用真实发音音频登记模板; `detect` 对候选音频算
//    与模板的相似度, 作为 `WakeWordDetection.confidence` 返回。
//
// 这是**模板包络匹配**, 不是统计学习模型: 置信度的含义是"能量包络与登记
// 发音的相似程度", 是否命中由 `WakeWord.sensitivity` 阈值判定
// ([`WakeWordDetection::is_wake`])。它对同一个人重复同一发音有效,
// 对跨人 / 变速发音的鲁棒性有限 —— 这是本层的真实能力边界。

/// 包络重采样点数 (相似度比较的固定维度).
pub const ENVELOPE_BINS: usize = 32;

/// 相似度计算的最小包络长度 (帧数): 过短输入不足以构成判定.
pub const MIN_ENVELOPE_FRAMES: usize = 4;

/// 登记模板所需的最少 PCM 采样点 (10ms @ 16kHz).
pub const MIN_ENROLL_SAMPLES: usize = 160;

/// 默认判定阈值 (与 `WakeWord.sensitivity` 默认 0.5 配套上调, 偏保守).
pub const DEFAULT_WAKE_THRESHOLD: f32 = 0.75;

/// 唤醒模板: 关键词 + 从真实音频提取的能量包络.
#[derive(Debug, Clone, PartialEq)]
pub struct WakeTemplate {
    /// 关键词.
    pub keyword: String,
    /// 类别.
    pub category: WakeWordCategory,
    /// 能量包络 (峰值归一化).
    pub envelope: Vec<f32>,
}

/// 提取短时能量包络 (帧 RMS, 峰值归一化到 0..=1).
///
/// 全静音输入返回全 0 包络 (相似度层面自然得 0 分)。
pub fn extract_envelope(samples: &[i16], frame_samples: usize) -> Vec<f32> {
    if samples.is_empty() || frame_samples == 0 {
        return Vec::new();
    }
    let mut envelope = Vec::new();
    for frame in samples.chunks(frame_samples) {
        let sum_sq: f64 = frame
            .iter()
            .map(|s| {
                let v = f64::from(*s);
                v * v
            })
            .sum();
        let rms = (sum_sq / frame.len() as f64).sqrt();
        envelope.push(rms as f32);
    }
    let peak = envelope.iter().cloned().fold(0.0f32, f32::max);
    if peak > 0.0 {
        for v in &mut envelope {
            *v /= peak;
        }
    }
    envelope
}

/// 线性重采样到 `bins` 个点 (长度 1 时恒定复制).
fn resample(envelope: &[f32], bins: usize) -> Vec<f32> {
    if envelope.is_empty() {
        return vec![0.0; bins];
    }
    if envelope.len() == 1 {
        return vec![envelope[0]; bins];
    }
    let last = (envelope.len() - 1) as f32;
    (0..bins)
        .map(|i| {
            let pos = i as f32 * last / (bins - 1) as f32;
            let lo = pos.floor() as usize;
            let hi = (lo + 1).min(envelope.len() - 1);
            let frac = pos - lo as f32;
            envelope[lo] * (1.0 - frac) + envelope[hi] * frac
        })
        .collect()
}

/// 包络相似度 (余弦相似度, 0.0..=1.0; 双零包络得 0.0).
pub fn envelope_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() < MIN_ENVELOPE_FRAMES || b.len() < MIN_ENVELOPE_FRAMES {
        return 0.0;
    }
    let ra = resample(a, ENVELOPE_BINS);
    let rb = resample(b, ENVELOPE_BINS);
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for (x, y) in ra.iter().zip(rb.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na <= 0.0 || nb <= 0.0 {
        return 0.0;
    }
    (dot / (na.sqrt() * nb.sqrt())).clamp(0.0, 1.0)
}

/// 唤醒检测器: 模板登记 + 包络匹配.
#[derive(Debug, Clone)]
pub struct WakeDetector {
    template: Option<WakeTemplate>,
    threshold: f32,
}

impl WakeDetector {
    /// 创建检测器 (`threshold` 0.0..=1.0).
    pub fn new(threshold: f32) -> Result<Self, VoiceError> {
        if !(0.0..=1.0).contains(&threshold) {
            return Err(VoiceError::InvalidArgument(format!(
                "wake threshold {threshold} out of range [0.0, 1.0]"
            )));
        }
        Ok(Self {
            template: None,
            threshold,
        })
    }

    /// 判定阈值.
    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    /// 是否已登记模板.
    pub fn has_template(&self) -> bool {
        self.template.is_some()
    }

    /// 清除模板.
    pub fn clear(&mut self) {
        self.template = None;
    }

    /// 登记模板 (以真实发音 PCM 提取包络; 静音音频拒绝登记).
    pub fn enroll(&mut self, wake: &WakeWord, samples: &[i16]) -> Result<(), VoiceError> {
        WakeWord::validate_keyword(&wake.keyword)?;
        if samples.len() < MIN_ENROLL_SAMPLES {
            return Err(VoiceError::InvalidArgument(format!(
                "enrollment audio too short: {} samples (< {MIN_ENROLL_SAMPLES})",
                samples.len()
            )));
        }
        let envelope = extract_envelope(samples, frame_samples_for(samples));
        if envelope.iter().all(|v| *v <= 0.0) {
            return Err(VoiceError::InvalidArgument(
                "enrollment audio is silent".to_string(),
            ));
        }
        self.template = Some(WakeTemplate {
            keyword: wake.keyword.clone(),
            category: wake.category,
            envelope,
        });
        Ok(())
    }

    /// 检测候选音频: 返回相似度得分 (是否命中由 `is_wake(sensitivity)` 判定).
    ///
    /// 未登记模板或模板关键词与当前配置不一致 → [`VoiceError::State`]
    /// (零假装: 没有模板就没有判定能力, 显式失败)。
    pub fn detect(
        &self,
        wake: &WakeWord,
        samples: &[i16],
    ) -> Result<WakeWordDetection, VoiceError> {
        if samples.is_empty() {
            return Err(VoiceError::InvalidArgument(
                "detection audio is empty".to_string(),
            ));
        }
        let template = self
            .template
            .as_ref()
            .ok_or_else(|| VoiceError::State("no wake template enrolled".to_string()))?;
        if template.keyword != wake.keyword {
            return Err(VoiceError::State(format!(
                "no wake template enrolled for keyword `{}`",
                wake.keyword
            )));
        }
        let envelope = extract_envelope(samples, frame_samples_for(samples));
        let score = envelope_similarity(&template.envelope, &envelope);
        Ok(WakeWordDetection::new(
            template.category,
            template.keyword.clone(),
            score,
        ))
    }
}

/// 帧长自适应: 约 20ms @ 16kHz 网格, 至少 1 个采样点.
fn frame_samples_for(samples: &[i16]) -> usize {
    (samples.len() / 8).max(1)
}

#[cfg(test)]
mod engine_tests {
    use super::*;

    /// 生成带能量包络的合成 PCM (包络形状 → 判定测试的确定性输入).
    fn synth(envelope: &[f32], frame_samples: usize) -> Vec<i16> {
        let mut out = Vec::new();
        for (i, amp) in envelope.iter().enumerate() {
            for j in 0..frame_samples {
                let phase = (i * frame_samples + j) % 2 == 0;
                let v = (*amp * 20_000.0) as i16;
                out.push(if phase { v } else { -v });
            }
        }
        out
    }

    #[test]
    fn envelope_extraction_normalizes_peak() {
        let audio = synth(&[0.25, 1.0, 0.5], 64);
        let env = extract_envelope(&audio, 64);
        assert_eq!(env.len(), 3);
        let peak = env.iter().cloned().fold(0.0f32, f32::max);
        assert!((peak - 1.0).abs() < 1e-6, "峰值必须归一化: {peak}");
        assert!(env[1] > env[0]);
        assert!(env[1] > env[2]);

        // 全静音 → 全 0
        let silent = vec![0i16; 256];
        let env = extract_envelope(&silent, 64);
        assert!(env.iter().all(|v| *v == 0.0));
    }

    #[test]
    fn envelope_similarity_bounds() {
        let a = vec![0.2, 0.9, 1.0, 0.4, 0.1, 0.0, 0.0, 0.0];
        // 相同 → 1.0
        let same = envelope_similarity(&a, &a);
        assert!((same - 1.0).abs() < 1e-6, "identical envelopes: {same}");
        // 全零 → 0.0
        let zeros = vec![0.0; 8];
        assert_eq!(envelope_similarity(&a, &zeros), 0.0);
        // 反向形状 → 明显低于 1.0
        let mut rev = a.clone();
        rev.reverse();
        let diff = envelope_similarity(&a, &rev);
        assert!(diff < 0.999, "different shapes must not match: {diff}");
        // 过短 → 0.0
        assert_eq!(envelope_similarity(&[1.0], &a), 0.0);
    }

    #[test]
    fn detector_enroll_rejects_silent_and_short_audio() {
        let mut detector = WakeDetector::new(DEFAULT_WAKE_THRESHOLD).expect("detector");
        let wake = WakeWord::default_apeireth();
        assert!(matches!(
            detector.enroll(&wake, &[0i16; 64]),
            Err(VoiceError::InvalidArgument(_))
        ));
        assert!(
            matches!(
                detector.enroll(&wake, &[0i16; MIN_ENROLL_SAMPLES]),
                Err(VoiceError::InvalidArgument(_))
            ),
            "静音登记必须拒绝"
        );
    }

    #[test]
    fn detector_requires_matching_enrolled_template() {
        let wake = WakeWord::default_apeireth();
        let detector = WakeDetector::new(DEFAULT_WAKE_THRESHOLD).expect("detector");
        let audio = synth(&[0.5, 1.0, 0.7, 0.4, 0.2], 64);
        // 未登记 → State
        assert!(matches!(
            detector.detect(&wake, &audio),
            Err(VoiceError::State(_))
        ));

        // 登记后换关键词 → State
        let mut detector = detector;
        detector.enroll(&wake, &audio).expect("enroll");
        let other = WakeWord::custom("hey buddy".to_string()).expect("valid");
        assert!(matches!(
            detector.detect(&other, &audio),
            Err(VoiceError::State(_))
        ));
    }

    #[test]
    fn detector_scores_matching_audio_high_and_silence_low() {
        let wake = WakeWord::default_apeireth();
        let mut detector = WakeDetector::new(DEFAULT_WAKE_THRESHOLD).expect("detector");
        let audio = synth(&[0.2, 0.6, 1.0, 0.7, 0.3], 64);
        detector.enroll(&wake, &audio).expect("enroll");

        // 同一音频 → 高分命中
        let det = detector.detect(&wake, &audio).expect("detect");
        assert_eq!(det.keyword, VOICE_DEFAULT_WAKE_WORD);
        assert_eq!(det.category, WakeWordCategory::Hardcoded);
        assert!(
            det.confidence > 0.99,
            "same audio must score high: {}",
            det.confidence
        );
        assert!(det.is_wake(wake.sensitivity));

        // 静音 → 低分不命中
        let det = detector
            .detect(&wake, &[0i16; 320])
            .expect("detect silence");
        assert!(
            det.confidence < 0.5,
            "silence must score low: {}",
            det.confidence
        );
        assert!(!det.is_wake(wake.sensitivity));

        // 空音频 → InvalidArgument
        assert!(matches!(
            detector.detect(&wake, &[]),
            Err(VoiceError::InvalidArgument(_))
        ));
    }

    #[test]
    fn detector_threshold_guards_and_clear() {
        assert!(matches!(
            WakeDetector::new(1.5),
            Err(VoiceError::InvalidArgument(_))
        ));
        let wake = WakeWord::default_apeireth();
        let mut detector = WakeDetector::new(0.5).expect("detector");
        let audio = synth(&[1.0, 1.0, 0.8, 0.6], 64);
        detector.enroll(&wake, &audio).expect("enroll");
        assert!(detector.has_template());
        detector.clear();
        assert!(!detector.has_template());
        assert!(matches!(
            detector.detect(&wake, &audio),
            Err(VoiceError::State(_))
        ));
    }
}
