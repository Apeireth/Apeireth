//! 语音一元请求 / 响应帧 + 传输边界 (转写 / 合成协议层).
//!
//! 转写与合成为一元 RPC: [`VoiceFrame::TranscribeRequest`] /
//! [`VoiceFrame::SynthesisRequest`] 请求, 对应响应帧或 [`VoiceFrame::Error`]。
//! 流式面走 [`crate::voice::stream`] 的分块协议; 两条面共享同一闭合错误词表。
//!
//! 传输边界 [`VoiceTransport`] 是唯一 IO 面: 生产注入 HTTP / 流式传输,
//! 测试注入脚本化 mock (零真实网络)。

use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::frame_codec::{encode_frame, StreamDecoder};
use crate::voice::error::VoiceError;
use crate::voice::stream::StreamFrame;

/// 一元帧类型总数 (闭合词表规模).
pub const VOICE_FRAME_COUNT: usize = 5;

/// 模型名最大长度 (防超长标识进错误 / 日志).
pub const MAX_MODEL_NAME_BYTES: usize = 128;

/// 一元请求 / 响应帧 (5 变体, 闭合词表).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VoiceFrame {
    /// 转写请求.
    TranscribeRequest {
        /// 请求 ID (响应关联).
        request_id: u64,
        /// 模型标识.
        model: String,
        /// 音频格式.
        format: String,
        /// 采样率 (Hz).
        sample_rate: u32,
        /// 位深.
        bit_depth: u16,
        /// 通道数.
        channels: u8,
        /// 语言 (可选).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
        /// 音频字节.
        audio: Vec<u8>,
    },
    /// 转写响应.
    TranscribeResponse {
        /// 请求 ID.
        request_id: u64,
        /// 转写文本.
        text: String,
        /// 服务端报告的模型标识.
        model: String,
        /// 服务端报告的语言.
        language: String,
        /// 置信度 (0.0..=1.0, 可选).
        confidence: Option<f32>,
        /// 音频时长 (毫秒).
        duration_ms: u64,
    },
    /// 合成请求.
    SynthesisRequest {
        /// 请求 ID.
        request_id: u64,
        /// 模型标识.
        model: String,
        /// 音色标识.
        voice: String,
        /// 语言.
        language: String,
        /// 输出格式.
        output_format: String,
        /// 采样率 (Hz).
        sample_rate: u32,
        /// 待合成文本.
        text: String,
    },
    /// 合成响应.
    SynthesisResponse {
        /// 请求 ID.
        request_id: u64,
        /// 音频字节.
        data: Vec<u8>,
        /// 音频格式.
        format: String,
        /// 采样率 (Hz).
        sample_rate: u32,
        /// 位深.
        bit_depth: u16,
        /// 通道数.
        channels: u8,
        /// 合成时长 (毫秒).
        duration_ms: u64,
    },
    /// 错误响应.
    Error {
        /// 请求 ID.
        request_id: u64,
        /// 稳定错误码.
        code: String,
        /// 说明 (不得携带秘密).
        message: String,
        /// 限流建议退避 (毫秒, 可选).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        retry_after_ms: Option<u64>,
    },
}

impl VoiceFrame {
    /// 帧类型稳定字符串.
    pub fn type_str(&self) -> &'static str {
        match self {
            VoiceFrame::TranscribeRequest { .. } => "transcribe_request",
            VoiceFrame::TranscribeResponse { .. } => "transcribe_response",
            VoiceFrame::SynthesisRequest { .. } => "synthesis_request",
            VoiceFrame::SynthesisResponse { .. } => "synthesis_response",
            VoiceFrame::Error { .. } => "error",
        }
    }

    /// 请求 ID.
    pub fn request_id(&self) -> u64 {
        match self {
            VoiceFrame::TranscribeRequest { request_id, .. }
            | VoiceFrame::TranscribeResponse { request_id, .. }
            | VoiceFrame::SynthesisRequest { request_id, .. }
            | VoiceFrame::SynthesisResponse { request_id, .. }
            | VoiceFrame::Error { request_id, .. } => *request_id,
        }
    }
}

/// 一元帧解码器.
pub type VoiceFrameDecoder = StreamDecoder<VoiceFrame>;

/// 编码一帧一元帧.
pub fn encode_voice_frame(frame: &VoiceFrame) -> Result<Vec<u8>, VoiceError> {
    encode_frame(frame).map_err(VoiceError::from)
}

/// 请求 ID 生成器 (进程内单调, 响应关联用).
static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

/// 生成下一个请求 ID.
pub fn next_request_id() -> u64 {
    REQUEST_ID.fetch_add(1, Ordering::Relaxed)
}

/// 校验模型标识 (非空 + ≤ [`MAX_MODEL_NAME_BYTES`] 字节).
pub fn validate_model_name(model: &str) -> Result<(), VoiceError> {
    if model.trim().is_empty() {
        return Err(VoiceError::InvalidArgument(
            "model name is empty".to_string(),
        ));
    }
    if model.len() > MAX_MODEL_NAME_BYTES {
        return Err(VoiceError::InvalidArgument(format!(
            "model name too long: {} bytes (> {MAX_MODEL_NAME_BYTES})",
            model.len()
        )));
    }
    Ok(())
}

/// 一元错误帧 → 闭合词表分类 (与流错误帧同一分类口径).
pub fn classify_error_frame(code: &str, message: &str, retry_after_ms: Option<u64>) -> VoiceError {
    crate::voice::stream::classify_stream_error(code, message, retry_after_ms)
}

/// 语音传输边界: 协议层唯一 IO 面.
///
/// 实现约定:
/// - `exchange` 保序返回**同一 `request_id`** 的响应, ID 不匹配即协议违例;
/// - 失败必须返闭合词表错误 (网络类可重试判定交给上层), 不得静默返假响应。
#[async_trait::async_trait]
pub trait VoiceTransport: Send + Sync {
    /// 一元请求 / 响应.
    async fn exchange(&self, request: VoiceFrame) -> Result<VoiceFrame, VoiceError>;
    /// 流式上行帧.
    async fn send_stream(&self, frame: StreamFrame) -> Result<(), VoiceError>;
    /// 流式下行帧.
    async fn recv_stream(&self) -> Result<StreamFrame, VoiceError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice::stream::StreamKind;

    fn all_frames() -> Vec<VoiceFrame> {
        vec![
            VoiceFrame::TranscribeRequest {
                request_id: 1,
                model: "m1".to_string(),
                format: "wav".to_string(),
                sample_rate: 16_000,
                bit_depth: 16,
                channels: 1,
                language: Some("en".to_string()),
                audio: vec![1, 2, 3],
            },
            VoiceFrame::TranscribeResponse {
                request_id: 1,
                text: "hello".to_string(),
                model: "m1".to_string(),
                language: "en".to_string(),
                confidence: Some(0.9),
                duration_ms: 1_500,
            },
            VoiceFrame::SynthesisRequest {
                request_id: 2,
                model: "m2".to_string(),
                voice: "v1".to_string(),
                language: "zh-CN".to_string(),
                output_format: "mp3".to_string(),
                sample_rate: 24_000,
                text: "你好".to_string(),
            },
            VoiceFrame::SynthesisResponse {
                request_id: 2,
                data: vec![9, 9],
                format: "mp3".to_string(),
                sample_rate: 24_000,
                bit_depth: 16,
                channels: 1,
                duration_ms: 800,
            },
            VoiceFrame::Error {
                request_id: 3,
                code: "rate_limited".to_string(),
                message: "slow down".to_string(),
                retry_after_ms: Some(500),
            },
        ]
    }

    #[test]
    fn voice_frame_table_is_closed_at_5() {
        assert_eq!(VOICE_FRAME_COUNT, 5);
        assert_eq!(all_frames().len(), 5);
    }

    #[test]
    fn voice_frame_codec_roundtrip_every_variant() {
        for frame in all_frames() {
            let bytes = encode_voice_frame(&frame).expect("encode");
            let mut decoder = VoiceFrameDecoder::new();
            decoder.push(&bytes);
            let decoded = decoder.next_frame().expect("decode").expect("one");
            assert_eq!(decoded, frame, "roundtrip mismatch: {}", frame.type_str());
            assert_eq!(decoded.request_id(), frame.request_id());
        }
    }

    #[test]
    fn request_id_is_monotonic() {
        let a = next_request_id();
        let b = next_request_id();
        assert!(b > a);
    }

    #[test]
    fn model_name_validation() {
        assert!(validate_model_name("m1").is_ok());
        assert!(matches!(
            validate_model_name("  "),
            Err(VoiceError::InvalidArgument(_))
        ));
        let long = "x".repeat(MAX_MODEL_NAME_BYTES + 1);
        assert!(matches!(
            validate_model_name(&long),
            Err(VoiceError::InvalidArgument(_))
        ));
    }

    #[test]
    fn classify_error_frame_delegates_to_stream_classifier() {
        assert!(matches!(
            classify_error_frame("rate_limited", "m", Some(3)),
            VoiceError::RateLimited { retry_after_ms: 3 }
        ));
        assert!(matches!(
            classify_error_frame("unauthorized", "m", None),
            VoiceError::Authentication(_)
        ));
    }

    #[test]
    fn voice_frame_request_ids_are_preserved_on_wire() {
        let frame = VoiceFrame::Error {
            request_id: 42,
            code: "timeout".to_string(),
            message: "m".to_string(),
            retry_after_ms: None,
        };
        let bytes = encode_voice_frame(&frame).expect("encode");
        let json: serde_json::Value = serde_json::from_slice(&bytes[4..]).expect("json");
        assert_eq!(json["request_id"], 42);
        assert_eq!(json["type"], "error");
    }

    #[test]
    fn stream_frames_and_voice_frames_share_codec_discipline() {
        // 同一编码器族: 一元帧 + 流帧混喂, 半包不破坏帧边界
        let v = VoiceFrame::Error {
            request_id: 1,
            code: "protocol".into(),
            message: "m".into(),
            retry_after_ms: None,
        };
        let s = StreamFrame::Open {
            stream_id: "s".into(),
            kind: StreamKind::Transcription,
            model: "m".into(),
            language: None,
        };
        let mut bytes = encode_voice_frame(&v).expect("encode");
        let stream_bytes = crate::frame_codec::encode_frame(&s).expect("encode");
        // 第二帧只喂一半 (半包)
        bytes.extend_from_slice(&stream_bytes[..stream_bytes.len() / 2]);

        let mut vdec = VoiceFrameDecoder::new();
        vdec.push(&bytes);
        let got = vdec.next_frame().expect("decode").expect("one");
        assert_eq!(got, v);
        assert!(
            vdec.next_frame().expect("decode").is_none(),
            "半包必须留在缓存等待后续字节"
        );
    }
}
