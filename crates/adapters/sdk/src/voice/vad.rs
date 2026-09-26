//! # Voice Activity Detection (per 既有 Voice SDK)
//!
//! 3 VAD 算法 (按既有实现 + task spec §3):
//! 1. **Energy** — 基于能量阈值 (RMS 简易, 离线)
//! 2. **Silence** — 基于静音时长阈值 (per 上游 silence detection)
//! 3. **WebRtc** — WebRTC VAD 集成 (per Chromium WebRTC VAD, 离线)
//!
//! 领域模型 (3 算法枚举 + 配置 / 结果) + 流式折叠状态机 (§5).
//!
//! ## 引用文档
//!
//! 1. `既有 Voice SDK` `client/vad_engine.js` (VAD 参考)
//! 2. WebRTC VAD 官方文档 (per Google WebRTC project)

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::voice::error::{VoiceError, VoiceResult};

// ============================================================================
// §1 3 VAD 算法 enum (K-1 强校验守门, 编译期 hardcode 3 variant)
// ============================================================================

/// VAD 算法 (3 variant, 1:1 翻译 既有 Voice SDK `VadAlgorithm` enum).
///
/// 3 算法 snake_case 字符串严格匹配 既有实现 API 规范.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VadAlgorithm {
    /// **基于能量阈值** (RMS 简易, 离线, 按既有实现估算 1:1).
    #[default]
    Energy,
    /// **基于静音时长阈值** (silence detection, 按既有实现估算 1:1).
    Silence,
    /// **WebRTC VAD** (Chromium WebRTC VAD 集成, 离线, 按既有实现估算 1:1).
    WebRtc,
}

impl VadAlgorithm {
    /// 3 算法 hardcode 常量.
    pub const COUNT: usize = 3;

    /// 字符串 (对齐既有实现 `algorithm` 字段, snake_case 严格匹配).
    pub fn as_str(&self) -> &'static str {
        match self {
            VadAlgorithm::Energy => "energy",
            VadAlgorithm::Silence => "silence",
            VadAlgorithm::WebRtc => "webrtc",
        }
    }

    /// 从字符串解析 (按既有实现响应 `algorithm` 字段).
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
// §2 VadConfig VAD 配置 (per 对齐既有实现)
// ============================================================================

/// VAD 配置 (按既有实现 `vad_config` 字段).
///
/// 字段对应既有实现 `VadConfig` 对象:
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
            return Err(VoiceError::InvalidArgument(format!(
                "energy_threshold {} out of range [0.0, 1.0]",
                energy_threshold
            )));
        }
        // 静音阈值 0..=10000ms (10s)
        if silence_threshold_ms > 10_000 {
            return Err(VoiceError::InvalidArgument(format!(
                "silence_threshold_ms {} out of range [0, 10000]",
                silence_threshold_ms
            )));
        }
        // 最小语音长度 0..=10000ms
        if min_speech_duration_ms > 10_000 {
            return Err(VoiceError::InvalidArgument(format!(
                "min_speech_duration_ms {} out of range [0, 10000]",
                min_speech_duration_ms
            )));
        }
        // 帧长度 10/20/30 ms (per WebRTC VAD)
        if !matches!(frame_size_ms, 10 | 20 | 30) {
            return Err(VoiceError::InvalidArgument(format!(
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
// §3 VadResult VAD 检测结果 (per 对齐既有实现)
// ============================================================================

/// VAD 检测结果 (按既有实现 `vad_detect` 响应).
///
/// 字段对应既有实现 `VadResult` 对象:
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
        assert!(matches!(result, Err(VoiceError::InvalidArgument(_))));
        let result = VadConfig::custom(VadAlgorithm::Energy, -0.1, 1000, 200, 20);
        assert!(matches!(result, Err(VoiceError::InvalidArgument(_))));
    }

    #[test]
    fn k1_vad_config_rejects_invalid_frame_size() {
        let result = VadConfig::custom(VadAlgorithm::Energy, 0.1, 1000, 200, 50);
        assert!(matches!(result, Err(VoiceError::InvalidArgument(_))));
    }

    #[test]
    fn k1_vad_config_rejects_invalid_silence_threshold() {
        let result = VadConfig::custom(VadAlgorithm::Silence, 0.0, 20000, 200, 20);
        assert!(matches!(result, Err(VoiceError::InvalidArgument(_))));
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

// ============================================================================
// §5 流式 VAD 折叠状态机 (语音段门限 + 静音挂留 + 置信度平滑)
// ============================================================================
//
// 语义 (如实文档化):
// - `fold` 逐观测折叠: 语音观测先进**候选段**, 累计到
//   `min_speech_duration_ms` 才计入语音 (短促噪声门限); 未达门限的候选段
//   在静音到来或 `finish` 时降级为静音。
// - 静音观测时长 < `silence_threshold_ms` 视为句中停顿 (挂留), 不清除
//   `in_speech`; 达到阈值才判定语音段结束。
// - 置信度按 EMA (α=0.5) 平滑, 抑制单帧抖动。
// 这是纯确定性折叠逻辑, 零模型推理; 每条迁移都在测试里钉死。

/// 置信度 EMA 系数.
pub const CONFIDENCE_EMA_ALPHA: f32 = 0.5;

/// 流式 VAD 折叠状态.
#[derive(Debug, Clone, PartialEq)]
pub struct VadStreamState {
    algorithm: VadAlgorithm,
    min_speech_duration_ms: u64,
    hangover_ms: u64,
    committed_speech_ms: u64,
    committed_silence_ms: u64,
    candidate_speech_ms: u64,
    confidence_ema: Option<f32>,
    in_speech: bool,
}

impl VadStreamState {
    /// 从 VAD 配置创建折叠状态.
    pub fn new(config: &VadConfig) -> Self {
        Self {
            algorithm: config.algorithm,
            min_speech_duration_ms: u64::from(config.min_speech_duration_ms),
            hangover_ms: u64::from(config.silence_threshold_ms),
            committed_speech_ms: 0,
            committed_silence_ms: 0,
            candidate_speech_ms: 0,
            confidence_ema: None,
            in_speech: false,
        }
    }

    /// 当前是否处于语音段 (门限后).
    pub fn in_speech(&self) -> bool {
        self.in_speech
    }

    /// 未过门限的候选语音时长 (毫秒).
    pub fn candidate_speech_ms(&self) -> u64 {
        self.candidate_speech_ms
    }

    /// 复位.
    pub fn reset(&mut self) {
        self.committed_speech_ms = 0;
        self.committed_silence_ms = 0;
        self.candidate_speech_ms = 0;
        self.confidence_ema = None;
        self.in_speech = false;
    }

    /// 折叠一个观测, 返回折叠后的聚合结果.
    pub fn fold(&mut self, obs: &VadResult) -> VadResult {
        // 置信度 EMA 平滑
        self.confidence_ema = Some(match self.confidence_ema {
            Some(prev) => {
                prev * (1.0 - CONFIDENCE_EMA_ALPHA) + obs.confidence * CONFIDENCE_EMA_ALPHA
            }
            None => obs.confidence,
        });

        if obs.is_speech {
            self.committed_silence_ms += obs.silence_duration.as_millis() as u64;
            self.candidate_speech_ms += obs.speech_duration.as_millis() as u64;
            if self.candidate_speech_ms >= self.min_speech_duration_ms {
                self.committed_speech_ms += self.candidate_speech_ms;
                self.candidate_speech_ms = 0;
                self.in_speech = true;
            }
        } else {
            // 未达门限的候选语音降级为静音 (短促噪声门限)
            let downgrade = self.candidate_speech_ms;
            self.candidate_speech_ms = 0;
            self.committed_silence_ms += obs.silence_duration.as_millis() as u64
                + obs.speech_duration.as_millis() as u64
                + downgrade;
            let silence_run = obs.silence_duration.as_millis() as u64;
            if silence_run >= self.hangover_ms {
                self.in_speech = false;
            }
        }
        self.snapshot()
    }

    /// 流结束: 尾部未达门限的候选语音降级为静音.
    pub fn finish(&mut self) -> VadResult {
        self.committed_silence_ms += self.candidate_speech_ms;
        self.candidate_speech_ms = 0;
        self.in_speech = false;
        self.snapshot()
    }

    fn snapshot(&self) -> VadResult {
        VadResult {
            is_speech: self.in_speech,
            algorithm: self.algorithm,
            confidence: self.confidence_ema.unwrap_or(0.0),
            speech_duration: Duration::from_millis(self.committed_speech_ms),
            silence_duration: Duration::from_millis(self.committed_silence_ms),
        }
    }
}

#[cfg(test)]
mod stream_tests {
    use super::*;

    fn config(min_speech_ms: u32, hangover_ms: u32) -> VadConfig {
        VadConfig {
            algorithm: VadAlgorithm::Energy,
            energy_threshold: 0.05,
            silence_threshold_ms: hangover_ms,
            min_speech_duration_ms: min_speech_ms,
            frame_size_ms: 20,
        }
    }

    fn speech_obs(ms: u64, confidence: f32) -> VadResult {
        VadResult {
            is_speech: true,
            algorithm: VadAlgorithm::Energy,
            confidence,
            speech_duration: Duration::from_millis(ms),
            silence_duration: Duration::from_millis(0),
        }
    }

    fn silence_obs(ms: u64, confidence: f32) -> VadResult {
        VadResult {
            is_speech: false,
            algorithm: VadAlgorithm::Energy,
            confidence,
            speech_duration: Duration::from_millis(0),
            silence_duration: Duration::from_millis(ms),
        }
    }

    #[test]
    fn short_speech_burst_is_gated_to_silence() {
        let mut state = VadStreamState::new(&config(100, 500));
        // 60ms 语音 < 100ms 门限 → 候选, 不算语音
        let out = state.fold(&speech_obs(60, 0.9));
        assert!(!out.is_speech);
        assert_eq!(state.candidate_speech_ms(), 60);
        assert_eq!(out.speech_duration, Duration::from_millis(0));

        // 静音到来 → 候选降级为静音
        let out = state.fold(&silence_obs(20, 0.1));
        assert!(!out.is_speech);
        assert_eq!(state.candidate_speech_ms(), 0);
        assert_eq!(out.speech_duration, Duration::from_millis(0));
        assert_eq!(
            out.silence_duration,
            Duration::from_millis(80),
            "60 降级 + 20 静音"
        );
    }

    #[test]
    fn sustained_speech_commits_after_min_duration() {
        let mut state = VadStreamState::new(&config(100, 500));
        state.fold(&speech_obs(40, 0.8));
        state.fold(&speech_obs(40, 0.8));
        assert!(!state.in_speech(), "未达门限不算语音");
        let out = state.fold(&speech_obs(40, 0.8));
        assert!(out.is_speech, "累计 120ms ≥ 100ms 门限");
        assert_eq!(out.speech_duration, Duration::from_millis(120));
        assert_eq!(state.candidate_speech_ms(), 0);
    }

    #[test]
    fn hangover_keeps_speech_through_short_pauses() {
        let mut state = VadStreamState::new(&config(100, 500));
        state.fold(&speech_obs(120, 0.9));
        assert!(state.in_speech());
        // 短停顿 200ms < 500ms 挂留 → 仍在语音段
        let out = state.fold(&silence_obs(200, 0.2));
        assert!(out.is_speech, "短停顿不应清除语音段");
        // 长停顿 600ms ≥ 500ms → 语音段结束
        let out = state.fold(&silence_obs(600, 0.1));
        assert!(!out.is_speech, "长停顿必须结束语音段");
    }

    #[test]
    fn confidence_is_smoothed_by_ema() {
        let mut state = VadStreamState::new(&config(10, 100));
        let out = state.fold(&speech_obs(20, 0.0));
        assert!((out.confidence - 0.0).abs() < 1e-6);
        let out = state.fold(&speech_obs(0, 1.0));
        // EMA: 0*0.5 + 1*0.5 = 0.5
        assert!(
            (out.confidence - 0.5).abs() < 1e-6,
            "EMA 平滑值必须 0.5: {}",
            out.confidence
        );
    }

    #[test]
    fn finish_downgrades_trailing_candidate() {
        let mut state = VadStreamState::new(&config(200, 500));
        state.fold(&speech_obs(150, 0.9));
        let out = state.finish();
        assert!(!out.is_speech);
        assert_eq!(out.speech_duration, Duration::from_millis(0));
        assert_eq!(
            out.silence_duration,
            Duration::from_millis(150),
            "尾部候选降级"
        );
    }

    #[test]
    fn reset_clears_everything() {
        let mut state = VadStreamState::new(&config(100, 500));
        state.fold(&speech_obs(300, 0.9));
        state.reset();
        assert!(!state.in_speech());
        assert_eq!(state.candidate_speech_ms(), 0);
        let out = state.fold(&silence_obs(0, 0.0));
        assert_eq!(out.speech_duration, Duration::from_millis(0));
        assert_eq!(out.silence_duration, Duration::from_millis(0));
    }

    #[test]
    fn folded_result_propagates_algorithm_and_ratio() {
        let mut cfg = config(100, 500);
        cfg.algorithm = VadAlgorithm::WebRtc;
        let mut state = VadStreamState::new(&cfg);
        state.fold(&speech_obs(120, 0.5));
        let out = state.fold(&silence_obs(80, 0.5));
        assert_eq!(out.algorithm, VadAlgorithm::WebRtc);
        let total = out.speech_duration.as_millis() + out.silence_duration.as_millis();
        assert_eq!(total, 200);
        assert!((out.speech_ratio() - 0.6).abs() < 1e-6);
    }
}
