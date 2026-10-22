//! 音频采集会话: 采集配置推导 + 有界帧队列 (背压) + 生命周期.
//!
//! 采集参数全部从 [`AudioConfig`] 推导 (字节率 / 单帧字节数), 推导结果
//! 与 [`crate::voice::config`] 的 K-1 校验面互为守门:
//! - [`bytes_per_second`]: `sample_rate × channels × (bit_depth / 8)`
//! - [`bytes_per_frame`]: 按帧时长折算, 非整数帧一律拒绝 (采样网格对齐)
//!
//! 帧队列有界: 打满即 [`VoiceError::Backpressure`] (调用方丢帧或降速),
//! 零无限缓冲 —— 流式采集的背压纪律与流协议信用窗口同源。

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::voice::config::AudioConfig;
use crate::voice::error::VoiceError;

/// 默认帧队列容量 (帧数): 覆盖约 2s @ 20ms 帧.
pub const DEFAULT_CAPTURE_QUEUE_FRAMES: usize = 100;

/// 采集帧时长上限 (毫秒).
pub const MAX_FRAME_DURATION_MS: u32 = 1_000;

/// 字节率 (bytes/秒): `sample_rate × channels × (bit_depth / 8)`.
pub fn bytes_per_second(config: &AudioConfig) -> u64 {
    let bytes_per_sample = u64::from(config.bit_depth) / 8;
    u64::from(config.sample_rate) * u64::from(config.channels) * bytes_per_sample
}

/// 单帧字节数: `bytes_per_second × frame_ms / 1000`, 必须整除 (采样网格对齐).
pub fn bytes_per_frame(config: &AudioConfig, frame_ms: u32) -> Result<usize, VoiceError> {
    if frame_ms == 0 || frame_ms > MAX_FRAME_DURATION_MS {
        return Err(VoiceError::InvalidArgument(format!(
            "frame duration {frame_ms}ms out of range 1..={MAX_FRAME_DURATION_MS}"
        )));
    }
    let numerator = bytes_per_second(config) * u64::from(frame_ms);
    if numerator % 1000 != 0 {
        return Err(VoiceError::InvalidArgument(format!(
            "frame duration {frame_ms}ms is not aligned to the sample grid at {} Hz",
            config.sample_rate
        )));
    }
    Ok((numerator / 1000) as usize)
}

/// 时长 → 帧数 (向上取整).
pub fn frame_count_for_duration(frame_ms: u32, duration_ms: u64) -> u64 {
    if frame_ms == 0 {
        return 0;
    }
    duration_ms.div_ceil(u64::from(frame_ms))
}

/// 未压缩 PCM 字节数 → 时长 (毫秒, 向下取整).
///
/// 公式: `bytes × 8 × 1000 / (sample_rate × channels × bit_depth)`;
/// 参数非法 (0 采样率 / 0 位深) 返 0, 不 panic。
pub fn duration_ms_for_bytes(bytes: usize, sample_rate: u32, bit_depth: u16, channels: u8) -> u64 {
    let denom = u64::from(sample_rate) * u64::from(channels) * u64::from(bit_depth);
    if denom == 0 {
        return 0;
    }
    (bytes as u64) * 8 * 1000 / denom
}

/// 采集会话生命周期状态.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureState {
    /// 空闲 (未采集).
    Idle,
    /// 采集中 (帧入队).
    Listening,
}

/// 采集统计快照.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureStats {
    /// 当前状态.
    pub state: CaptureState,
    /// 累计入队帧数.
    pub frames_pushed: u64,
    /// 累计入队字节数.
    pub bytes_pushed: u64,
    /// 当前队列深度.
    pub queued_frames: usize,
    /// 队列容量.
    pub capacity_frames: usize,
}

/// 采集会话: 有界帧队列 + 生命周期 + 采集配置纪律.
#[derive(Debug)]
pub struct CaptureSession {
    config: AudioConfig,
    frame_duration_ms: u32,
    expected_frame_bytes: usize,
    state: CaptureState,
    queue: VecDeque<Vec<u8>>,
    capacity_frames: usize,
    frames_pushed: u64,
    bytes_pushed: u64,
}

impl CaptureSession {
    /// 创建采集会话 (初始态 `Idle`).
    pub fn new(
        config: AudioConfig,
        frame_duration_ms: u32,
        capacity_frames: usize,
    ) -> Result<Self, VoiceError> {
        config.validate()?;
        if capacity_frames == 0 {
            return Err(VoiceError::InvalidArgument(
                "capture queue capacity must be non-zero".to_string(),
            ));
        }
        let expected_frame_bytes = bytes_per_frame(&config, frame_duration_ms)?;
        Ok(Self {
            config,
            frame_duration_ms,
            expected_frame_bytes,
            state: CaptureState::Idle,
            queue: VecDeque::new(),
            capacity_frames,
            frames_pushed: 0,
            bytes_pushed: 0,
        })
    }

    /// 从完整配置创建 (帧时长取 VAD 帧长, 容量取默认).
    pub fn from_voice_config(
        config: &crate::voice::config::VoiceConfig,
    ) -> Result<Self, VoiceError> {
        Self::new(
            config.audio.clone(),
            config.vad.frame_size_ms,
            DEFAULT_CAPTURE_QUEUE_FRAMES,
        )
    }

    /// 当前状态.
    pub fn state(&self) -> CaptureState {
        self.state
    }

    /// 期望单帧字节数.
    pub fn expected_frame_bytes(&self) -> usize {
        self.expected_frame_bytes
    }

    /// 帧时长 (毫秒).
    pub fn frame_duration_ms(&self) -> u32 {
        self.frame_duration_ms
    }

    /// 采集配置.
    pub fn config(&self) -> &AudioConfig {
        &self.config
    }

    /// 队列深度.
    pub fn queued_frames(&self) -> usize {
        self.queue.len()
    }

    /// 统计快照.
    pub fn stats(&self) -> CaptureStats {
        CaptureStats {
            state: self.state,
            frames_pushed: self.frames_pushed,
            bytes_pushed: self.bytes_pushed,
            queued_frames: self.queue.len(),
            capacity_frames: self.capacity_frames,
        }
    }

    /// 开始采集 (`Idle` → `Listening`).
    pub fn start(&mut self) -> Result<(), VoiceError> {
        match self.state {
            CaptureState::Idle => {
                self.state = CaptureState::Listening;
                Ok(())
            }
            CaptureState::Listening => Err(VoiceError::State(
                "capture session already listening".to_string(),
            )),
        }
    }

    /// 停止采集 (`Listening` → `Idle`), 返回累计统计.
    pub fn stop(&mut self) -> Result<CaptureStats, VoiceError> {
        match self.state {
            CaptureState::Listening => {
                self.state = CaptureState::Idle;
                self.queue.clear();
                Ok(self.stats())
            }
            CaptureState::Idle => Err(VoiceError::State(
                "capture session not listening".to_string(),
            )),
        }
    }

    /// 入队一帧 (必须精确等于 [`CaptureSession::expected_frame_bytes`]).
    ///
    /// 队列打满 → [`VoiceError::Backpressure`] (调用方降速或丢帧, 零无限缓冲)。
    pub fn push_frame(&mut self, frame: Vec<u8>) -> Result<(), VoiceError> {
        if self.state != CaptureState::Listening {
            return Err(VoiceError::State(
                "push_frame requires listening state".to_string(),
            ));
        }
        if frame.len() != self.expected_frame_bytes {
            return Err(VoiceError::InvalidArgument(format!(
                "frame size {} != expected {} bytes ({}ms @ {}Hz)",
                frame.len(),
                self.expected_frame_bytes,
                self.frame_duration_ms,
                self.config.sample_rate
            )));
        }
        if self.queue.len() >= self.capacity_frames {
            return Err(VoiceError::Backpressure(format!(
                "capture queue full: {} frames",
                self.capacity_frames
            )));
        }
        self.queue.push_back(frame);
        self.frames_pushed += 1;
        self.bytes_pushed += self.expected_frame_bytes as u64;
        Ok(())
    }

    /// 出队一帧 (消费侧).
    pub fn pop_frame(&mut self) -> Option<Vec<u8>> {
        self.queue.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav_config() -> AudioConfig {
        AudioConfig::default_wav()
    }

    #[test]
    fn byte_rate_math_matches_capture_config() {
        // 默认: 16kHz / 16bit / mono → 32000 bytes/s
        assert_eq!(bytes_per_second(&wav_config()), 32_000);
        // 20ms 帧 → 640 bytes
        assert_eq!(bytes_per_frame(&wav_config(), 20).expect("aligned"), 640);
        // 44.1kHz / 16bit / stereo → 176400 bytes/s, 10ms 帧 → 1764 bytes
        let cfg =
            AudioConfig::custom("wav".to_string(), 44_100, 16, 2, "en".to_string()).expect("valid");
        assert_eq!(bytes_per_second(&cfg), 176_400);
        assert_eq!(bytes_per_frame(&cfg, 10).expect("aligned"), 1_764);
    }

    #[test]
    fn byte_rate_math_rejects_unaligned_frame() {
        // 11025Hz × 1 byte × 3ms = 33.075 bytes → 非整数
        let cfg =
            AudioConfig::custom("wav".to_string(), 11_025, 8, 1, "en".to_string()).expect("valid");
        assert!(matches!(
            bytes_per_frame(&cfg, 3),
            Err(VoiceError::InvalidArgument(_))
        ));
        assert!(matches!(
            bytes_per_frame(&wav_config(), 0),
            Err(VoiceError::InvalidArgument(_))
        ));
        assert!(matches!(
            bytes_per_frame(&wav_config(), MAX_FRAME_DURATION_MS + 1),
            Err(VoiceError::InvalidArgument(_))
        ));
    }

    #[test]
    fn frame_count_rounds_up() {
        assert_eq!(frame_count_for_duration(20, 1_000), 50);
        assert_eq!(frame_count_for_duration(20, 1_001), 51);
        assert_eq!(frame_count_for_duration(20, 0), 0);
        assert_eq!(frame_count_for_duration(0, 100), 0);
    }

    #[test]
    fn duration_math_for_pcm_bytes() {
        // 32000 bytes/s @ 16k/16bit/mono → 1000ms
        assert_eq!(duration_ms_for_bytes(32_000, 16_000, 16, 1), 1_000);
        // 640 bytes → 20ms
        assert_eq!(duration_ms_for_bytes(640, 16_000, 16, 1), 20);
        // 非法参数 → 0
        assert_eq!(duration_ms_for_bytes(100, 0, 16, 1), 0);
        assert_eq!(duration_ms_for_bytes(0, 16_000, 16, 1), 0);
    }

    #[test]
    fn capture_session_lifecycle_and_frame_discipline() {
        let mut session = CaptureSession::new(wav_config(), 20, 2).expect("session");
        assert_eq!(session.state(), CaptureState::Idle);
        assert_eq!(session.expected_frame_bytes(), 640);

        // 未开始先推帧 → State
        assert!(matches!(
            session.push_frame(vec![0u8; 640]),
            Err(VoiceError::State(_))
        ));
        // 停止未开始的会话 → State
        assert!(matches!(session.stop(), Err(VoiceError::State(_))));

        session.start().expect("start");
        // 二次开始 → State
        assert!(matches!(session.start(), Err(VoiceError::State(_))));

        // 帧大小不合规 → InvalidArgument
        assert!(matches!(
            session.push_frame(vec![0u8; 639]),
            Err(VoiceError::InvalidArgument(_))
        ));

        // 正常入队
        session.push_frame(vec![1u8; 640]).expect("frame 1");
        session.push_frame(vec![2u8; 640]).expect("frame 2");
        // 队列打满 → Backpressure
        let err = session
            .push_frame(vec![3u8; 640])
            .expect_err("must backpressure");
        assert_eq!(
            crate::error_taxonomy::ClassifyError::category(&err),
            crate::error_taxonomy::ErrorCategory::Backpressure
        );
        assert!(crate::error_taxonomy::ClassifyError::is_retryable(&err));

        // 消费一帧 → 恢复入队
        assert_eq!(session.pop_frame(), Some(vec![1u8; 640]));
        session.push_frame(vec![3u8; 640]).expect("frame 3");

        let stats = session.stop().expect("stop");
        assert_eq!(stats.state, CaptureState::Idle);
        assert_eq!(stats.frames_pushed, 3);
        assert_eq!(stats.bytes_pushed, 3 * 640);
        assert_eq!(stats.queued_frames, 0, "stop 清空队列");
    }

    #[test]
    fn capture_session_constructor_guards() {
        assert!(matches!(
            CaptureSession::new(wav_config(), 20, 0),
            Err(VoiceError::InvalidArgument(_))
        ));
        // 非法采集配置被 K-1 拒绝
        let bad = AudioConfig {
            format: "wav".to_string(),
            sample_rate: 100,
            bit_depth: 16,
            channels: 1,
            language: "en".to_string(),
        };
        assert!(CaptureSession::new(bad, 20, 10).is_err());
    }

    #[test]
    fn capture_session_from_voice_config_uses_vad_frame() {
        let config = crate::voice::config::VoiceConfig::default();
        let session = CaptureSession::from_voice_config(&config).expect("session");
        assert_eq!(session.frame_duration_ms(), config.vad.frame_size_ms);
        assert_eq!(
            session.expected_frame_bytes(),
            bytes_per_frame(&config.audio, config.vad.frame_size_ms).expect("aligned")
        );
    }
}
