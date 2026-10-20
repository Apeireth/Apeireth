//! 流式转写 / 合成分块协议 + 背压 (流式协议层).
//!
//! 线格式: 长度前缀 + UTF-8 JSON 帧体 (共用 [`crate::frame_codec`] 纪律),
//! 帧体 `{"type": "<snake_case>", ...}`。
//!
//! ## 分块协议
//!
//! 一条流由 [`StreamFrame::Open`] 开启, `Ready` 协商窗口, 数据分块
//! (`AudioChunk` 上行 / `TranscriptChunk` / `SynthesisChunk` 下行) 以
//! `chunk_seq` 单调递增传输, `final_chunk` 标记末块, [`StreamFrame::Close`]
//! 关闭。分块序号违例 (跳号 / 重复 / 末块后再来 / 超限) 一律
//! [`VoiceError::Protocol`], 零静默修复。
//!
//! ## 背压 (信用窗口)
//!
//! 接收方以 [`StreamFrame::Credit`] 发放发送信用 (单位: 分块数):
//! - [`StreamSender`] 每发一个分块消耗 1 信用, 信用耗尽 →
//!   [`VoiceError::Backpressure`] (整批原子失败, 不发半批)。
//! - [`StreamReceiver`] 消费缓冲后经 [`StreamReceiver::grant_frames`] 补发信用,
//!   发送方 [`StreamSender::on_credit`] 收回。
//!
//! 测试零真实网络: 发送方 / 接收方直接对接, 窗口行为逐迁移钉死。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::frame_codec::{encode_frame, StreamDecoder};
use crate::voice::error::VoiceError;

/// 单分块载荷上界 (字节).
pub const MAX_CHUNK_BYTES: usize = 64 * 1024;

/// 默认发送窗口 (分块数).
pub const DEFAULT_WINDOW_CHUNKS: u32 = 8;

/// 信用累计上限 (防接收方无限发放把发送方信用撑爆).
pub const MAX_CREDITS: u32 = 1024;

/// 帧类型总数 (闭合词表规模).
pub const STREAM_FRAME_COUNT: usize = 8;

// ============================================================================
// §1 流帧
// ============================================================================

/// 流方向.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamKind {
    /// 流式转写 (音频上行, 文本下行).
    Transcription,
    /// 流式合成 (文本上行, 音频下行).
    Synthesis,
}

/// 流式协议帧 (8 变体, 闭合词表).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamFrame {
    /// 开启流 (请求).
    Open {
        /// 流 ID.
        stream_id: String,
        /// 流方向.
        kind: StreamKind,
        /// 模型标识.
        model: String,
        /// 语言 (可选, 空 = 自动推断).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
    },
    /// 流就绪 (响应, 协商窗口).
    Ready {
        /// 流 ID.
        stream_id: String,
        /// 单分块载荷上界 (字节).
        max_chunk_bytes: u32,
        /// 初始发送窗口 (分块数).
        window_chunks: u32,
    },
    /// 音频分块 (上行).
    AudioChunk {
        /// 流 ID.
        stream_id: String,
        /// 分块序号 (0 起, 单调).
        chunk_seq: u32,
        /// 分块载荷.
        payload: Vec<u8>,
        /// 是否末块.
        final_chunk: bool,
    },
    /// 转写分块 (下行).
    TranscriptChunk {
        /// 流 ID.
        stream_id: String,
        /// 分块序号 (0 起, 单调).
        chunk_seq: u32,
        /// 增量文本.
        text: String,
        /// 是否末块.
        final_chunk: bool,
    },
    /// 合成分块 (下行).
    SynthesisChunk {
        /// 流 ID.
        stream_id: String,
        /// 分块序号 (0 起, 单调).
        chunk_seq: u32,
        /// 音频载荷.
        payload: Vec<u8>,
        /// 是否末块.
        final_chunk: bool,
    },
    /// 发送信用 (背压窗口).
    Credit {
        /// 流 ID.
        stream_id: String,
        /// 新增信用 (分块数).
        credits: u32,
    },
    /// 流错误.
    StreamError {
        /// 流 ID.
        stream_id: String,
        /// 稳定错误码.
        code: String,
        /// 说明 (不得携带秘密).
        message: String,
        /// 限流建议退避 (毫秒, 可选).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        retry_after_ms: Option<u64>,
    },
    /// 关闭流.
    Close {
        /// 流 ID.
        stream_id: String,
        /// 关闭原因 (可选).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
}

impl StreamFrame {
    /// 帧类型稳定字符串.
    pub fn type_str(&self) -> &'static str {
        match self {
            StreamFrame::Open { .. } => "open",
            StreamFrame::Ready { .. } => "ready",
            StreamFrame::AudioChunk { .. } => "audio_chunk",
            StreamFrame::TranscriptChunk { .. } => "transcript_chunk",
            StreamFrame::SynthesisChunk { .. } => "synthesis_chunk",
            StreamFrame::Credit { .. } => "credit",
            StreamFrame::StreamError { .. } => "stream_error",
            StreamFrame::Close { .. } => "close",
        }
    }

    /// 流 ID.
    pub fn stream_id(&self) -> &str {
        match self {
            StreamFrame::Open { stream_id, .. }
            | StreamFrame::Ready { stream_id, .. }
            | StreamFrame::AudioChunk { stream_id, .. }
            | StreamFrame::TranscriptChunk { stream_id, .. }
            | StreamFrame::SynthesisChunk { stream_id, .. }
            | StreamFrame::Credit { stream_id, .. }
            | StreamFrame::StreamError { stream_id, .. }
            | StreamFrame::Close { stream_id, .. } => stream_id,
        }
    }
}

/// 流帧解码器.
pub type StreamFrameDecoder = StreamDecoder<StreamFrame>;

/// 编码一帧流协议帧.
pub fn encode_stream_frame(frame: &StreamFrame) -> Result<Vec<u8>, VoiceError> {
    encode_frame(frame).map_err(VoiceError::from)
}

// ============================================================================
// §2 发送侧 (分块 + 信用背压)
// ============================================================================

/// 流发送侧: 把载荷切分块并按信用窗口发送.
#[derive(Debug)]
pub struct StreamSender {
    stream_id: String,
    max_chunk_bytes: usize,
    credits: u32,
    next_seq: u32,
    closed: bool,
}

impl StreamSender {
    /// 创建发送侧 (`max_chunk_bytes` 1..=MAX_CHUNK_BYTES, 初始信用 0..=MAX_CREDITS).
    pub fn new(
        stream_id: impl Into<String>,
        max_chunk_bytes: usize,
        credits: u32,
    ) -> Result<Self, VoiceError> {
        if max_chunk_bytes == 0 || max_chunk_bytes > MAX_CHUNK_BYTES {
            return Err(VoiceError::InvalidArgument(format!(
                "max_chunk_bytes {max_chunk_bytes} out of range 1..={MAX_CHUNK_BYTES}"
            )));
        }
        if credits > MAX_CREDITS {
            return Err(VoiceError::InvalidArgument(format!(
                "credits {credits} exceeds {MAX_CREDITS}"
            )));
        }
        Ok(Self {
            stream_id: stream_id.into(),
            max_chunk_bytes,
            credits,
            next_seq: 0,
            closed: false,
        })
    }

    /// 当前可用信用.
    pub fn credits(&self) -> u32 {
        self.credits
    }

    /// 下一个分块序号.
    pub fn next_seq(&self) -> u32 {
        self.next_seq
    }

    /// 是否已关闭.
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// 推入一段载荷: 切分为分块帧 (信用不足整批原子失败).
    ///
    /// `final_chunk` 落在最后一块; 空载荷产生一个空末块 (保证对端收得到终结信号)。
    pub fn push_payload(
        &mut self,
        payload: &[u8],
        final_chunk: bool,
    ) -> Result<Vec<StreamFrame>, VoiceError> {
        if self.closed {
            return Err(VoiceError::State("stream already closed".to_string()));
        }
        let needed = if payload.is_empty() {
            1
        } else {
            payload.len().div_ceil(self.max_chunk_bytes) as u32
        };
        if needed > self.credits {
            return Err(VoiceError::Backpressure(format!(
                "send window exhausted: need {needed} chunk credits, have {}",
                self.credits
            )));
        }
        let mut frames = Vec::with_capacity(needed as usize);
        if payload.is_empty() {
            frames.push(StreamFrame::AudioChunk {
                stream_id: self.stream_id.clone(),
                chunk_seq: self.next_seq,
                payload: Vec::new(),
                final_chunk,
            });
            self.next_seq += 1;
            self.credits -= 1;
            return Ok(frames);
        }
        for i in 0..needed {
            let start = i as usize * self.max_chunk_bytes;
            let end = ((i as usize + 1) * self.max_chunk_bytes).min(payload.len());
            let is_last = i + 1 == needed;
            frames.push(StreamFrame::AudioChunk {
                stream_id: self.stream_id.clone(),
                chunk_seq: self.next_seq,
                payload: payload[start..end].to_vec(),
                final_chunk: final_chunk && is_last,
            });
            self.next_seq += 1;
            self.credits -= 1;
        }
        Ok(frames)
    }

    /// 收到信用发放 (封顶 [`MAX_CREDITS`], 超发即协议违例).
    pub fn on_credit(&mut self, credits: u32) -> Result<(), VoiceError> {
        if credits == 0 {
            return Err(VoiceError::Protocol(
                "zero credit grant is illegal".to_string(),
            ));
        }
        let total = self.credits.saturating_add(credits);
        if total > MAX_CREDITS {
            return Err(VoiceError::Protocol(format!(
                "credit overflow: {total} > {MAX_CREDITS}"
            )));
        }
        self.credits = total;
        Ok(())
    }

    /// 关闭流 (幂等保护: 二次关闭返 `State`).
    pub fn close(&mut self, reason: Option<String>) -> Result<StreamFrame, VoiceError> {
        if self.closed {
            return Err(VoiceError::State("stream already closed".to_string()));
        }
        self.closed = true;
        Ok(StreamFrame::Close {
            stream_id: self.stream_id.clone(),
            reason,
        })
    }
}

// ============================================================================
// §3 接收侧 (序号校验 + 组装 + 信用补发)
// ============================================================================

/// 流接收侧: 分块组装 + 序号纪律 + 信用补发.
#[derive(Debug)]
pub struct StreamReceiver {
    stream_id: String,
    max_chunk_bytes: usize,
    window_chunks: u32,
    expected_seq: u32,
    buffered: Vec<u8>,
    consumed: u32,
    finalized: bool,
    closed: bool,
}

impl StreamReceiver {
    /// 创建接收侧.
    pub fn new(
        stream_id: impl Into<String>,
        max_chunk_bytes: usize,
        window_chunks: u32,
    ) -> Result<Self, VoiceError> {
        if max_chunk_bytes == 0 || max_chunk_bytes > MAX_CHUNK_BYTES {
            return Err(VoiceError::InvalidArgument(format!(
                "max_chunk_bytes {max_chunk_bytes} out of range 1..={MAX_CHUNK_BYTES}"
            )));
        }
        if window_chunks == 0 || window_chunks > MAX_CREDITS {
            return Err(VoiceError::InvalidArgument(format!(
                "window_chunks {window_chunks} out of range 1..={MAX_CREDITS}"
            )));
        }
        Ok(Self {
            stream_id: stream_id.into(),
            max_chunk_bytes,
            window_chunks,
            expected_seq: 0,
            buffered: Vec::new(),
            consumed: 0,
            finalized: false,
            closed: false,
        })
    }

    /// 已缓存未消费字节数.
    pub fn buffered_len(&self) -> usize {
        self.buffered.len()
    }

    /// 是否已收末块.
    pub fn is_finalized(&self) -> bool {
        self.finalized
    }

    /// 喂入一个音频分块; 收到末块时返回 `Some(完整载荷)` (缓冲随之清空).
    pub fn on_audio_chunk(
        &mut self,
        chunk_seq: u32,
        payload: &[u8],
        final_chunk: bool,
    ) -> Result<Option<Vec<u8>>, VoiceError> {
        if self.closed {
            return Err(VoiceError::Protocol("chunk after stream close".to_string()));
        }
        if self.finalized {
            return Err(VoiceError::Protocol(format!(
                "chunk {chunk_seq} after final chunk"
            )));
        }
        if chunk_seq != self.expected_seq {
            return Err(VoiceError::Protocol(format!(
                "out-of-order chunk: expected {}, got {chunk_seq}",
                self.expected_seq
            )));
        }
        if payload.len() > self.max_chunk_bytes {
            return Err(VoiceError::Protocol(format!(
                "chunk {} payload {} bytes exceeds {MAX_CHUNK_BYTES}",
                chunk_seq,
                payload.len()
            )));
        }
        self.expected_seq += 1;
        self.buffered.extend_from_slice(payload);
        self.consumed += 1;
        if final_chunk {
            self.finalized = true;
            let done = std::mem::take(&mut self.buffered);
            return Ok(Some(done));
        }
        Ok(None)
    }

    /// 消费缓冲后补发信用 (每 `window_chunks` 个分块补一批, 保序).
    pub fn grant_frames(&mut self) -> Vec<StreamFrame> {
        if self.consumed == 0 {
            return Vec::new();
        }
        let batches = self.consumed / self.window_chunks;
        self.consumed %= self.window_chunks;
        if batches == 0 {
            return Vec::new();
        }
        vec![StreamFrame::Credit {
            stream_id: self.stream_id.clone(),
            credits: batches * self.window_chunks,
        }]
    }

    /// 标记流关闭 (之后的分块一律协议违例).
    pub fn mark_closed(&mut self) {
        self.closed = true;
    }
}

// ============================================================================
// §4 组装器 (转写文本 / 合成音频)
// ============================================================================

/// 流式转写组装器: 增量文本合并 + 末块判定 + 序号纪律.
#[derive(Debug)]
pub struct TranscriptAssembler {
    expected_seq: u32,
    parts: Vec<String>,
    finalized: bool,
}

impl TranscriptAssembler {
    /// 创建空组装器.
    pub fn new() -> Self {
        Self {
            expected_seq: 0,
            parts: Vec::new(),
            finalized: false,
        }
    }

    /// 已合并文本.
    pub fn text(&self) -> String {
        self.parts.concat()
    }

    /// 是否已收末块.
    pub fn is_finalized(&self) -> bool {
        self.finalized
    }

    /// 喂入一个转写分块; 末块时返回 `Some(完整文本)`.
    pub fn on_chunk(
        &mut self,
        chunk_seq: u32,
        text: &str,
        final_chunk: bool,
    ) -> Result<Option<String>, VoiceError> {
        if self.finalized {
            return Err(VoiceError::Protocol(format!(
                "transcript chunk {chunk_seq} after final chunk"
            )));
        }
        if chunk_seq != self.expected_seq {
            return Err(VoiceError::Protocol(format!(
                "out-of-order transcript chunk: expected {}, got {chunk_seq}",
                self.expected_seq
            )));
        }
        self.expected_seq += 1;
        self.parts.push(text.to_string());
        if final_chunk {
            self.finalized = true;
            return Ok(Some(self.text()));
        }
        Ok(None)
    }

    /// 收流结束却无末块 → 协议违例 (调用方在 `Close` 时调).
    pub fn finish(&mut self) -> Result<String, VoiceError> {
        if !self.finalized {
            return Err(VoiceError::Protocol(
                "stream closed without final transcript chunk".to_string(),
            ));
        }
        Ok(self.text())
    }
}

impl Default for TranscriptAssembler {
    fn default() -> Self {
        Self::new()
    }
}

/// 流式合成组装器: 音频分块合并 + 末块判定 + 序号纪律.
#[derive(Debug)]
pub struct SynthesisAssembler {
    expected_seq: u32,
    chunks: Vec<u8>,
    finalized: bool,
}

impl SynthesisAssembler {
    /// 创建空组装器.
    pub fn new() -> Self {
        Self {
            expected_seq: 0,
            chunks: Vec::new(),
            finalized: false,
        }
    }

    /// 已合并字节数.
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// 是否为空.
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// 是否已收末块.
    pub fn is_finalized(&self) -> bool {
        self.finalized
    }

    /// 喂入一个合成分块; 末块时返回 `Some(完整音频)`.
    pub fn on_chunk(
        &mut self,
        chunk_seq: u32,
        payload: &[u8],
        final_chunk: bool,
    ) -> Result<Option<Vec<u8>>, VoiceError> {
        if self.finalized {
            return Err(VoiceError::Protocol(format!(
                "synthesis chunk {chunk_seq} after final chunk"
            )));
        }
        if chunk_seq != self.expected_seq {
            return Err(VoiceError::Protocol(format!(
                "out-of-order synthesis chunk: expected {}, got {chunk_seq}",
                self.expected_seq
            )));
        }
        if payload.len() > MAX_CHUNK_BYTES {
            return Err(VoiceError::Protocol(format!(
                "synthesis chunk {} payload {} bytes exceeds {MAX_CHUNK_BYTES}",
                chunk_seq,
                payload.len()
            )));
        }
        self.expected_seq += 1;
        self.chunks.extend_from_slice(payload);
        if final_chunk {
            self.finalized = true;
            return Ok(Some(std::mem::take(&mut self.chunks)));
        }
        Ok(None)
    }

    /// 收流结束却无末块 → 协议违例.
    pub fn finish(&mut self) -> Result<Vec<u8>, VoiceError> {
        if !self.finalized {
            return Err(VoiceError::Protocol(
                "stream closed without final synthesis chunk".to_string(),
            ));
        }
        Ok(std::mem::take(&mut self.chunks))
    }
}

impl Default for SynthesisAssembler {
    fn default() -> Self {
        Self::new()
    }
}

/// 流错误帧 → 闭合词表分类.
pub fn classify_stream_error(code: &str, message: &str, retry_after_ms: Option<u64>) -> VoiceError {
    match code {
        "rate_limited" => VoiceError::RateLimited {
            retry_after_ms: retry_after_ms.unwrap_or(1_000),
        },
        "unauthorized" | "forbidden" | "auth" => VoiceError::Authentication(message.to_string()),
        "token_expired" => VoiceError::TokenExpired,
        "protocol" => VoiceError::Protocol(message.to_string()),
        "unavailable" | "network" => VoiceError::Network(message.to_string()),
        "timeout" => VoiceError::Timeout {
            operation: "stream_operation",
        },
        "backpressure" => VoiceError::Backpressure(message.to_string()),
        _ => VoiceError::Internal(format!("stream error code `{code}`: {message}")),
    }
}

/// 流表 (多路复用观测): 按流 ID 追踪进行中的流.
#[derive(Debug, Default)]
pub struct StreamTable {
    active: HashMap<String, StreamKind>,
}

impl StreamTable {
    /// 创建空表.
    pub fn new() -> Self {
        Self::default()
    }

    /// 登记新流 (重复 ID → 协议违例).
    pub fn open(&mut self, stream_id: &str, kind: StreamKind) -> Result<(), VoiceError> {
        if self.active.insert(stream_id.to_string(), kind).is_some() {
            return Err(VoiceError::Protocol(format!(
                "duplicate stream id {stream_id}"
            )));
        }
        Ok(())
    }

    /// 关闭流 (未知 ID → 协议违例).
    pub fn close(&mut self, stream_id: &str) -> Result<(), VoiceError> {
        if self.active.remove(stream_id).is_none() {
            return Err(VoiceError::Protocol(format!(
                "close for unknown stream {stream_id}"
            )));
        }
        Ok(())
    }

    /// 当前活跃流数.
    pub fn len(&self) -> usize {
        self.active.len()
    }

    /// 是否无活跃流.
    pub fn is_empty(&self) -> bool {
        self.active.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error_taxonomy::{ClassifyError, ErrorCategory};

    #[test]
    fn stream_frame_table_is_closed_at_8() {
        assert_eq!(STREAM_FRAME_COUNT, 8);
        let frames = vec![
            StreamFrame::Open {
                stream_id: "s1".into(),
                kind: StreamKind::Transcription,
                model: "m".into(),
                language: Some("en".into()),
            },
            StreamFrame::Ready {
                stream_id: "s1".into(),
                max_chunk_bytes: 1024,
                window_chunks: 4,
            },
            StreamFrame::AudioChunk {
                stream_id: "s1".into(),
                chunk_seq: 0,
                payload: vec![1],
                final_chunk: false,
            },
            StreamFrame::TranscriptChunk {
                stream_id: "s1".into(),
                chunk_seq: 0,
                text: "hi".into(),
                final_chunk: true,
            },
            StreamFrame::SynthesisChunk {
                stream_id: "s1".into(),
                chunk_seq: 0,
                payload: vec![2],
                final_chunk: true,
            },
            StreamFrame::Credit {
                stream_id: "s1".into(),
                credits: 2,
            },
            StreamFrame::StreamError {
                stream_id: "s1".into(),
                code: "rate_limited".into(),
                message: "m".into(),
                retry_after_ms: Some(10),
            },
            StreamFrame::Close {
                stream_id: "s1".into(),
                reason: None,
            },
        ];
        assert_eq!(frames.len(), STREAM_FRAME_COUNT);
    }

    #[test]
    fn stream_frame_codec_roundtrip_every_variant() {
        let frames = vec![
            StreamFrame::Open {
                stream_id: "s1".into(),
                kind: StreamKind::Synthesis,
                model: "m".into(),
                language: None,
            },
            StreamFrame::Ready {
                stream_id: "s1".into(),
                max_chunk_bytes: 4096,
                window_chunks: 8,
            },
            StreamFrame::AudioChunk {
                stream_id: "s1".into(),
                chunk_seq: 3,
                payload: vec![9, 8, 7],
                final_chunk: true,
            },
            StreamFrame::TranscriptChunk {
                stream_id: "s1".into(),
                chunk_seq: 1,
                text: "增量".into(),
                final_chunk: false,
            },
            StreamFrame::SynthesisChunk {
                stream_id: "s1".into(),
                chunk_seq: 2,
                payload: vec![1],
                final_chunk: false,
            },
            StreamFrame::Credit {
                stream_id: "s1".into(),
                credits: 8,
            },
            StreamFrame::StreamError {
                stream_id: "s1".into(),
                code: "timeout".into(),
                message: "slow".into(),
                retry_after_ms: None,
            },
            StreamFrame::Close {
                stream_id: "s1".into(),
                reason: Some("done".into()),
            },
        ];
        for frame in frames {
            let bytes = encode_stream_frame(&frame).expect("encode");
            let mut decoder = StreamFrameDecoder::new();
            decoder.push(&bytes);
            let decoded = decoder.next_frame().expect("decode").expect("one");
            assert_eq!(decoded, frame, "roundtrip mismatch: {}", frame.type_str());
        }
    }

    #[test]
    fn sender_splits_payload_and_marks_final() {
        let mut sender = StreamSender::new("s1", 4, 8).expect("sender");
        let frames = sender
            .push_payload(&[1, 2, 3, 4, 5, 6, 7, 8, 9], true)
            .expect("push");
        assert_eq!(frames.len(), 3);
        assert_eq!(sender.credits(), 5, "3 个分块消耗 3 信用");
        assert_eq!(sender.next_seq(), 3);
        match &frames[2] {
            StreamFrame::AudioChunk {
                chunk_seq,
                payload,
                final_chunk,
                ..
            } => {
                assert_eq!(*chunk_seq, 2);
                assert_eq!(payload, &vec![9]);
                assert!(*final_chunk);
            }
            other => panic!("unexpected {other:?}"),
        }
        assert!(!matches!(
            &frames[0],
            StreamFrame::AudioChunk {
                final_chunk: true,
                ..
            }
        ));
    }

    #[test]
    fn sender_backpressure_fails_atomically_and_recovers_on_credit() {
        let mut sender = StreamSender::new("s1", 2, 2).expect("sender");
        // 8 字节 / 2 = 4 分块 > 2 信用 → 整批失败
        let err = sender
            .push_payload(&[0u8; 8], true)
            .expect_err("must backpressure");
        assert_eq!(err.category(), ErrorCategory::Backpressure);
        assert!(err.is_retryable());
        assert_eq!(sender.credits(), 2, "失败不消耗信用");
        assert_eq!(sender.next_seq(), 0, "失败不推进序号");

        // 收回信用 → 恢复
        sender.on_credit(2).expect("credit");
        assert_eq!(sender.credits(), 4);
        let frames = sender.push_payload(&[0u8; 8], true).expect("push");
        assert_eq!(frames.len(), 4);
    }

    #[test]
    fn sender_empty_payload_yields_single_final_chunk() {
        let mut sender = StreamSender::new("s1", 1024, 2).expect("sender");
        let frames = sender.push_payload(&[], true).expect("push");
        assert_eq!(frames.len(), 1);
        assert!(matches!(
            &frames[0],
            StreamFrame::AudioChunk {
                payload,
                final_chunk: true,
                ..
            } if payload.is_empty()
        ));
    }

    #[test]
    fn sender_credit_discipline() {
        let mut sender = StreamSender::new("s1", 1024, 1).expect("sender");
        assert!(matches!(sender.on_credit(0), Err(VoiceError::Protocol(_))));
        sender.on_credit(MAX_CREDITS - 1).expect("credit");
        assert!(
            matches!(sender.on_credit(1), Err(VoiceError::Protocol(_))),
            "信用溢出必须拒绝"
        );
        // 关闭后再推 → State
        sender.close(Some("done".into())).expect("close");
        assert!(matches!(
            sender.push_payload(&[1], true),
            Err(VoiceError::State(_))
        ));
        assert!(matches!(sender.close(None), Err(VoiceError::State(_))));
    }

    #[test]
    fn sender_rejects_bad_window_config() {
        assert!(matches!(
            StreamSender::new("s1", 0, 1),
            Err(VoiceError::InvalidArgument(_))
        ));
        assert!(matches!(
            StreamSender::new("s1", MAX_CHUNK_BYTES + 1, 1),
            Err(VoiceError::InvalidArgument(_))
        ));
        assert!(matches!(
            StreamSender::new("s1", 1024, MAX_CREDITS + 1),
            Err(VoiceError::InvalidArgument(_))
        ));
    }

    #[test]
    fn receiver_reassembles_and_enforces_sequence() {
        let mut receiver = StreamReceiver::new("s1", 16, 2).expect("receiver");
        assert!(receiver
            .on_audio_chunk(0, &[1, 2], false)
            .expect("c0")
            .is_none());
        assert_eq!(receiver.buffered_len(), 2);
        let done = receiver
            .on_audio_chunk(1, &[3, 4], true)
            .expect("c1")
            .expect("final");
        assert_eq!(done, vec![1, 2, 3, 4]);
        assert!(receiver.is_finalized());

        // 末块后再来 → Protocol
        assert!(matches!(
            receiver.on_audio_chunk(2, &[5], true),
            Err(VoiceError::Protocol(_))
        ));
    }

    #[test]
    fn receiver_rejects_out_of_order_duplicate_and_oversize() {
        let mut receiver = StreamReceiver::new("s1", 4, 2).expect("receiver");
        // 跳号
        assert!(matches!(
            receiver.on_audio_chunk(1, &[1], false),
            Err(VoiceError::Protocol(_))
        ));
        let mut receiver = StreamReceiver::new("s1", 4, 2).expect("receiver");
        receiver.on_audio_chunk(0, &[1], false).expect("c0");
        // 重复
        assert!(matches!(
            receiver.on_audio_chunk(0, &[1], false),
            Err(VoiceError::Protocol(_))
        ));
        let mut receiver = StreamReceiver::new("s1", 4, 2).expect("receiver");
        // 超限载荷
        assert!(matches!(
            receiver.on_audio_chunk(0, &[0u8; 5], false),
            Err(VoiceError::Protocol(_))
        ));
    }

    #[test]
    fn receiver_grants_credit_as_buffer_drains() {
        let mut receiver = StreamReceiver::new("s1", 16, 2).expect("receiver");
        for i in 0..2 {
            receiver.on_audio_chunk(i, &[1], false).expect("chunk");
        }
        let grants = receiver.grant_frames();
        assert_eq!(grants.len(), 1);
        assert!(matches!(
            &grants[0],
            StreamFrame::Credit { credits, .. } if *credits == 2
        ));
        // 不足一批不发
        receiver.on_audio_chunk(2, &[1], false).expect("chunk");
        assert!(receiver.grant_frames().is_empty());
    }

    #[test]
    fn receiver_marks_closed_and_rejects_later_chunks() {
        let mut receiver = StreamReceiver::new("s1", 16, 2).expect("receiver");
        receiver.mark_closed();
        assert!(matches!(
            receiver.on_audio_chunk(0, &[1], false),
            Err(VoiceError::Protocol(_))
        ));
    }

    #[test]
    fn transcript_assembler_merges_and_requires_final() {
        let mut assembler = TranscriptAssembler::new();
        assert!(assembler.on_chunk(0, "he", false).expect("c0").is_none());
        assert!(assembler.on_chunk(1, "llo", false).expect("c1").is_none());
        assert_eq!(assembler.text(), "hello");
        let full = assembler
            .on_chunk(2, " world", true)
            .expect("c2")
            .expect("final");
        assert_eq!(full, "hello world");
        assert_eq!(assembler.finish().expect("finish"), "hello world");

        // 无末块收尾 → Protocol
        let mut assembler = TranscriptAssembler::new();
        assembler.on_chunk(0, "x", false).expect("c0");
        assert!(matches!(assembler.finish(), Err(VoiceError::Protocol(_))));

        // 序号违例 → Protocol
        let mut assembler = TranscriptAssembler::new();
        assert!(matches!(
            assembler.on_chunk(5, "x", false),
            Err(VoiceError::Protocol(_))
        ));
    }

    #[test]
    fn synthesis_assembler_merges_bytes_and_requires_final() {
        let mut assembler = SynthesisAssembler::new();
        assert!(assembler.on_chunk(0, &[1, 2], false).expect("c0").is_none());
        assert_eq!(assembler.len(), 2);
        let full = assembler
            .on_chunk(1, &[3], true)
            .expect("c1")
            .expect("final");
        assert_eq!(full, vec![1, 2, 3]);
        assert!(assembler.is_empty());

        let mut assembler = SynthesisAssembler::new();
        assembler.on_chunk(0, &[1], false).expect("c0");
        assert!(matches!(assembler.finish(), Err(VoiceError::Protocol(_))));

        let mut assembler = SynthesisAssembler::new();
        assert!(matches!(
            assembler.on_chunk(0, &[0u8; MAX_CHUNK_BYTES + 1], true),
            Err(VoiceError::Protocol(_))
        ));
    }

    #[test]
    fn stream_error_classification_closed() {
        assert!(matches!(
            classify_stream_error("rate_limited", "m", Some(7)),
            VoiceError::RateLimited { retry_after_ms: 7 }
        ));
        assert!(matches!(
            classify_stream_error("token_expired", "m", None),
            VoiceError::TokenExpired
        ));
        assert!(matches!(
            classify_stream_error("protocol", "m", None),
            VoiceError::Protocol(_)
        ));
        assert!(matches!(
            classify_stream_error("timeout", "m", None),
            VoiceError::Timeout { .. }
        ));
        assert!(matches!(
            classify_stream_error("backpressure", "m", None),
            VoiceError::Backpressure(_)
        ));
        assert!(matches!(
            classify_stream_error("bogus", "m", None),
            VoiceError::Internal(_)
        ));
    }

    #[test]
    fn stream_table_tracks_open_close() {
        let mut table = StreamTable::new();
        table.open("s1", StreamKind::Transcription).expect("open");
        assert!(matches!(
            table.open("s1", StreamKind::Synthesis),
            Err(VoiceError::Protocol(_))
        ));
        assert_eq!(table.len(), 1);
        table.close("s1").expect("close");
        assert!(matches!(table.close("s1"), Err(VoiceError::Protocol(_))));
        assert!(table.is_empty());
    }
}
