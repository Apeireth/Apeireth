//! # Voice Activity Detection (per @anthropic-ai/voice v0.9.21 商业版 1:1 翻译)
//!
//! 3 VAD 算法 (per v0.9.21 商业版 + task spec §3):
//! 1. **Energy** — 基于能量阈值 (RMS 简易, 离线)
//! 2. **Silence** — 基于静音时长阈值 (per 商业版 silence detection)
//! 3. **WebRtc** — WebRTC VAD 集成 (per Chromium WebRTC VAD, 离线)
//!
//! **STUB**: 3 算法枚举保留 1:1 翻译, 但 detect() 内部返 `VoiceError::NotImplemented`.
//!
//! ## 引用文档
//!
//! 1. `@anthropic-ai/voice v0.9.21` `client/vad_engine.js` (VAD 1:1 翻译源)
//! 2. WebRTC VAD 官方文档 (per Google WebRTC project)

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::voice::error::{VoiceError, VoiceResult};

// ============================================================================
// §1 3 VAD 算法 enum (K-1 强校验守门, 编译期 hardcode 3 variant)
// ============================================================================

/// VAD 算法 (3 variant, 1:1 翻译 @anthropic-ai/voice v0.9.21 商业版 `VadAlgorithm` enum).
///
/// 3 算法 snake_case 字符串严格匹配 v0.9.21 商业版 API 规范.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VadAlgorithm {
    /// **基于能量阈值** (RMS 简易, 离线, per v0.9.21 商业版估 1:1).
    #[default]
    Energy,
    /// **基于静音时长阈值** (silence detection, per v0.9.21 商业版估 1:1).
    Silence,
    /// **WebRTC VAD** (Chromium WebRTC VAD 集成, 离线, per v0.9.21 商业版估 1:1).
    WebRtc,
}

impl VadAlgorithm {
    /// 3 算法 hardcode 常量.
    pub const COUNT: usize = 3;

    /// 字符串 (1:1 翻译 v0.9.21 商业版 `algorithm` 字段, snake_case 严格匹配).
    pub fn as_str(&self) -> &'static str {
        match self {
            VadAlgorithm::Energy => "energy",
            VadAlgorithm::Silence => "silence",
            VadAlgorithm::WebRtc => "webrtc",
        }
    }

    /// 从字符串解析 (per v0.9.21 商业版响应 `algorithm` 字段).
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "energy" => Some(VadAlgorithm::Energy),
            "silence" => Some(VadAlgorithm::Silence),
            "webrtc" => Some(VadAlgorithm::WebRtc),
            _ => None,
        }
    }

    /// 是否 offline (Energy / WebRtc 本地推理).
    pub fn is_offline(&self) -> bool {
        matches!(self, VadAlgorithm::Energy | VadAlgorithm::WebRtc)
    }

    /// 默认能量阈值 (per Energy 算法, 0.0..=1.0, 默认 0.05).
    pub fn default_energy_threshold(&self) -> f32 {
        match self {
            VadAlgorithm::Energy => 0.05,
            VadAlgorithm::Silence => 0.0, // silence 不用 energy
            VadAlgorithm::WebRtc => 0.5,  // WebRTC VAD aggressiveness 0-3, 映射到 0.0-1.0
        }
    }

    /// 默认静音时长阈值 (毫秒, per Silence 算法, 默认 500ms).
    pub fn default_silence_threshold_ms(&self) -> u32 {
        match self {
            VadAlgorithm::Energy => 0, // energy 不用 silence
            VadAlgorithm::Silence => 500,
            VadAlgorithm::WebRtc => 300,
        }
    }
}

impl std::fmt::Display for VadAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 编译期守门: SUPPORTED_VAD_ALGORITHMS 长度 == 3 (K-1 强校验守门, 3 算法 hardcode).
pub const SUPPORTED_VAD_ALGORITHMS: &[VadAlgorithm] = &[
    VadAlgorithm::Energy,
    VadAlgorithm::Silence,
    VadAlgorithm::WebRtc,
];
const _: () = assert!(SUPPORTED_VAD_ALGORITHMS.len() == 3);

// ============================================================================
// §2 VadConfig VAD 配置 (per v0.9.21 商业版 1:1 翻译)
// ============================================================================

/// VAD 配置 (per v0.9.21 商业版 `vad_config` 字段 1:1 翻译).
///
/// 字段对应 v0.9.21 商业版 `VadConfig` 对象:
/// - `algorithm` (per `VadAlgorithm`)
/// - `energy_threshold` (0.0..=1.0, per Energy 算法)
/// - `silence_threshold_ms` (静音时长阈值, per Silence 算法)
/// - `min_speech_duration_ms` (最小语音长度, 过滤短促噪声)
/// - `frame_size_ms` (VAD 帧长度, 10/20/30 ms per WebRTC VAD)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VadConfig {
    /// VAD 算法
    pub algorithm: VadAlgorithm,
    /// 能量阈值 (0.0..=1.0, per Energy 算法)
    pub energy_threshold: f32,
    /// 静音时长阈值 (毫秒)
    pub silence_threshold_ms: u32,
    /// 最小语音长度 (毫秒, 过滤短促噪声)
    pub min_speech_duration_ms: u32,
    /// VAD 帧长度 (毫秒, 10/20/30 per WebRTC VAD)
    pub frame_size_ms: u32,
}

impl VadConfig {
    /// 创建默认 Energy VAD 配置.
    pub fn default_energy() -> Self {
        Self {
            algorithm: VadAlgorithm::Energy,
            energy_threshold: VadAlgorithm::Energy.default_energy_threshold(),
            silence_threshold_ms: 0,
            min_speech_duration_ms: 100,
            frame_size_ms: 20,
        }
    }

    /// 创建默认 Silence VAD 配置.
    pub fn default_silence() -> Self {
        Self {
            algorithm: VadAlgorithm::Silence,
            energy_threshold: 0.0,
            silence_threshold_ms: VadAlgorithm::Silence.default_silence_threshold_ms(),
            min_speech_duration_ms: 100,
            frame_size_ms: 20,
        }
    }

    /// 创建默认 WebRTC VAD 配置.
    pub fn default_webrtc() -> Self {
        Self {
            algorithm: VadAlgorithm::WebRtc,
            energy_threshold: VadAlgorithm::WebRtc.default_energy_threshold(),
            silence_threshold_ms: VadAlgorithm::WebRtc.default_silence_threshold_ms(),
            min_speech_duration_ms: 100,
            frame_size_ms: 20,
        }
    }

    /// 创建自定义 VAD 配置 (per K-1 强校验守门).
    pub fn custom(
        algorithm: VadAlgorithm,
        energy_threshold: f32,
        silence_threshold_ms: u32,
        min_speech_duration_ms: u32,
        frame_size_ms: u32,
    ) -> VoiceResult<Self> {
        // 能量阈值 0.0..=1.0
        if !(0.0..=1.0).contains(&energy_threshold) {
            return Err(VoiceError::Other(format!(
                "energy_threshold {} out of range [0.0, 1.0]",
                energy_threshold
            )));
        }
        // 静音阈值 0..=10000ms (10s)
        if silence_threshold_ms > 10_000 {
            return Err(VoiceError::Other(format!(
                "silence_threshold_ms {} out of range [0, 10000]",
                silence_threshold_ms
            )));
        }
        // 最小语音长度 0..=10000ms
        if min_speech_duration_ms > 10_000 {
            return Err(VoiceError::Other(format!(
                "min_speech_duration_ms {} out of range [0, 10000]",
                min_speech_duration_ms
            )));
        }
        // 帧长度 10/20/30 ms (per WebRTC VAD)
        if !matches!(frame_size_ms, 10 | 20 | 30) {
            return Err(VoiceError::Other(format!(
                "frame_size_ms {} invalid, expected 10/20/30",
                frame_size_ms
            )));
        }
        Ok(Self {
            algorithm,
            energy_threshold,
            silence_threshold_ms,
            min_speech_duration_ms,
            frame_size_ms,
        })
    }
}

impl Default for VadConfig {
    fn default() -> Self {
        Self::default_energy()
    }
}

// ============================================================================
// §3 VadResult VAD 检测结果 (per v0.9.21 商业版 1:1 翻译)
// ============================================================================

/// VAD 检测结果 (per v0.9.21 商业版 `vad_detect` 响应 1:1 翻译).
///
/// 字段对应 v0.9.21 商业版 `VadResult` 对象:
/// - `is_speech` (是否语音)
/// - `algorithm` (per `VadAlgorithm`)
/// - `confidence` (0.0..=1.0, 置信度)
/// - `speech_duration` (语音时长)
/// - `silence_duration` (静音时长)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VadResult {
    /// 是否语音
    pub is_speech: bool,
    /// VAD 算法
    pub algorithm: VadAlgorithm,
    /// 置信度 (0.0..=1.0)
    pub confidence: f32,
    /// 语音时长
    pub speech_duration: Duration,
    /// 静音时长
    pub silence_duration: Duration,
}

impl VadResult {
    /// 创建新 VAD 结果 (STUB 模式由调用方构造, R21 续真接时由 detect 返).
    pub fn new(
        is_speech: bool,
        algorithm: VadAlgorithm,
        confidence: f32,
        speech_duration: Duration,
        silence_duration: Duration,
    ) -> Self {
        Self {
            is_speech,
            algorithm,
            confidence,
            speech_duration,
            silence_duration,
        }
    }

    /// 语音占比 (speech / (speech + silence), 0.0..=1.0)
    pub fn speech_ratio(&self) -> f32 {
        let total = self.speech_duration.as_millis() + self.silence_duration.as_millis();
        if total == 0 {
            return 0.0;
        }
        self.speech_duration.as_millis() as f32 / total as f32
    }
}

// ============================================================================
// §4 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- §1 3 VadAlgorithm 枚举守门 ----

    #[test]
    fn k1_vad_algorithm_has_3_variants() {
        assert_eq!(
            SUPPORTED_VAD_ALGORITHMS.len(),
            3,
            "K-1 强校验: 必须 3 个 VAD 算法"
        );
        assert_eq!(VadAlgorithm::COUNT, 3);
        assert_eq!(VadAlgorithm::Energy.as_str(), "energy");
        assert_eq!(VadAlgorithm::Silence.as_str(), "silence");
        assert_eq!(VadAlgorithm::WebRtc.as_str(), "webrtc");
    }

    #[test]
    fn k1_vad_algorithm_default_is_energy() {
        let default = VadAlgorithm::default();
        assert_eq!(default, VadAlgorithm::Energy);
    }

    #[test]
    fn k1_vad_algorithm_parse_roundtrip() {
        for algo in SUPPORTED_VAD_ALGORITHMS {
            assert_eq!(VadAlgorithm::parse(algo.as_str()), Some(*algo));
        }
        assert_eq!(VadAlgorithm::parse("unknown"), None);
    }

    #[test]
    fn k1_vad_algorithm_offline_check() {
        assert!(VadAlgorithm::Energy.is_offline());
        assert!(!VadAlgorithm::Silence.is_offline()); // Silence 算 hybrid
        assert!(VadAlgorithm::WebRtc.is_offline());
    }

    #[test]
    fn k1_vad_algorithm_default_thresholds() {
        assert!((VadAlgorithm::Energy.default_energy_threshold() - 0.05).abs() < 0.001);
        assert_eq!(VadAlgorithm::Silence.default_silence_threshold_ms(), 500);
        assert_eq!(VadAlgorithm::WebRtc.default_silence_threshold_ms(), 300);
    }

    // ---- §2 VadConfig ----

    #[test]
    fn k1_vad_config_default_trait_is_energy() {
        let config = VadConfig::default();
        assert_eq!(config.algorithm, VadAlgorithm::Energy);
    }

    #[test]
    fn k1_vad_config_default_energy() {
        let config = VadConfig::default_energy();
        assert_eq!(config.algorithm, VadAlgorithm::Energy);
        assert!((config.energy_threshold - 0.05).abs() < 0.001);
    }

    #[test]
    fn k1_vad_config_default_silence() {
        let config = VadConfig::default_silence();
        assert_eq!(config.algorithm, VadAlgorithm::Silence);
        assert_eq!(config.silence_threshold_ms, 500);
    }

    #[test]
    fn k1_vad_config_default_webrtc() {
        let config = VadConfig::default_webrtc();
        assert_eq!(config.algorithm, VadAlgorithm::WebRtc);
        assert_eq!(config.frame_size_ms, 20);
    }

    #[test]
    fn k1_vad_config_custom_valid() {
        let config =
            VadConfig::custom(VadAlgorithm::Energy, 0.1, 1000, 200, 20).expect("valid custom");
        assert!((config.energy_threshold - 0.1).abs() < 0.001);
    }

    #[test]
    fn k1_vad_config_rejects_invalid_energy_threshold() {
        let result = VadConfig::custom(VadAlgorithm::Energy, 1.5, 1000, 200, 20);
        assert!(matches!(result, Err(VoiceError::Other(_))));
        let result = VadConfig::custom(VadAlgorithm::Energy, -0.1, 1000, 200, 20);
        assert!(matches!(result, Err(VoiceError::Other(_))));
    }

    #[test]
    fn k1_vad_config_rejects_invalid_frame_size() {
        let result = VadConfig::custom(VadAlgorithm::Energy, 0.1, 1000, 200, 50);
        assert!(matches!(result, Err(VoiceError::Other(_))));
    }

    #[test]
    fn k1_vad_config_rejects_invalid_silence_threshold() {
        let result = VadConfig::custom(VadAlgorithm::Silence, 0.0, 20000, 200, 20);
        assert!(matches!(result, Err(VoiceError::Other(_))));
    }

    // ---- §3 VadResult ----

    #[test]
    fn k1_vad_result_speech_ratio() {
        let result = VadResult::new(
            true,
            VadAlgorithm::Energy,
            0.95,
            Duration::from_millis(3000),
            Duration::from_millis(1000),
        );
        assert!(result.is_speech);
        assert!((result.speech_ratio() - 0.75).abs() < 0.001);
    }

    #[test]
    fn k1_vad_result_speech_ratio_empty() {
        let result = VadResult::new(
            false,
            VadAlgorithm::Energy,
            0.0,
            Duration::from_millis(0),
            Duration::from_millis(0),
        );
        assert_eq!(result.speech_ratio(), 0.0);
    }
}
