//! 长度前缀帧编解码 (信令 / 语音流共用, 0 重复造轮子).
//!
//! 线格式 (大端):
//!
//! ```text
//! +------------------------+-------------------------------+
//! | u32 BE 帧体长度 (bytes) | 帧体 (UTF-8 JSON, serde 编解码) |
//! +------------------------+-------------------------------+
//! ```
//!
//! 纪律:
//! - 帧体长度上界 [`MAX_FRAME_BYTES`], 超限即 [`CodecErrorKind::Oversize`] —— 先验
//!   长度再分配, 拒绝恶意长度头导致的巨量分配。
//! - 长度为 0 的帧体一律非法 (没有"空帧"语义)。
//! - 流式解码器 [`StreamDecoder`] 在产出一次错误后进入**熔断**状态:
//!   帧边界已不可信, 后续字节一律拒收, 由上层重置连接。
//! - 半包缓存: `push` 只累积, `next_frame` 按需吐完整帧; 一次 `push` 可含多帧。

use std::marker::PhantomData;

use serde::de::DeserializeOwned;
use serde::Serialize;

/// 单帧帧体长度上界 (1 MiB): 覆盖最大信令帧 / 语音分块帧, 超限拒收.
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;

/// 长度前缀字节数 (u32 大端).
pub const LENGTH_PREFIX_BYTES: usize = 4;

/// 编解码错误类别 (有限词表).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecErrorKind {
    /// 帧体长度为 0 或超过 [`MAX_FRAME_BYTES`].
    Oversize,
    /// 帧体不是合法 UTF-8 JSON / 反序列化失败.
    Malformed,
    /// 序列化失败 (编码侧).
    Encode,
    /// 解码器已熔断 (先前产出过错误, 帧边界不可信).
    Poisoned,
}

impl CodecErrorKind {
    /// 稳定字符串 (日志维度).
    pub const fn as_str(self) -> &'static str {
        match self {
            CodecErrorKind::Oversize => "oversize",
            CodecErrorKind::Malformed => "malformed",
            CodecErrorKind::Encode => "encode",
            CodecErrorKind::Poisoned => "poisoned",
        }
    }
}

/// 编解码错误 (类别 + 脱敏细节; 细节不回显帧体内容, 防止把秘密打日志).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodecError {
    /// 错误类别.
    pub kind: CodecErrorKind,
    /// 脱敏细节 (只含长度 / 位置等数字, 不含帧体字节).
    pub detail: String,
}

impl CodecError {
    /// 构造.
    pub fn new(kind: CodecErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }
}

impl std::fmt::Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "frame codec error ({}): {}",
            self.kind.as_str(),
            self.detail
        )
    }
}

impl std::error::Error for CodecError {}

/// 编码一帧: serde 序列化 + 4 字节大端长度前缀.
pub fn encode_frame<T: Serialize>(frame: &T) -> Result<Vec<u8>, CodecError> {
    let body = serde_json::to_vec(frame)
        .map_err(|e| CodecError::new(CodecErrorKind::Encode, format!("serialize failed: {e}")))?;
    if body.is_empty() || body.len() > MAX_FRAME_BYTES {
        return Err(CodecError::new(
            CodecErrorKind::Oversize,
            format!(
                "frame body length {} out of range 1..={MAX_FRAME_BYTES}",
                body.len()
            ),
        ));
    }
    let mut out = Vec::with_capacity(LENGTH_PREFIX_BYTES + body.len());
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

/// 解码单个完整帧体 (不含长度前缀).
pub fn decode_body<T: DeserializeOwned>(body: &[u8]) -> Result<T, CodecError> {
    if body.is_empty() || body.len() > MAX_FRAME_BYTES {
        return Err(CodecError::new(
            CodecErrorKind::Oversize,
            format!(
                "frame body length {} out of range 1..={MAX_FRAME_BYTES}",
                body.len()
            ),
        ));
    }
    serde_json::from_slice(body).map_err(|e| {
        // 细节只保留 serde 错误类别文本 (不含输入内容), 防止帧体泄密进日志
        CodecError::new(
            CodecErrorKind::Malformed,
            format!("body[{}B]: {e}", body.len()),
        )
    })
}

/// 流式帧解码器: 半包缓存 + 多帧连吐 + 出错熔断.
#[derive(Debug)]
pub struct StreamDecoder<T> {
    buf: Vec<u8>,
    poisoned: bool,
    _marker: PhantomData<T>,
}

impl<T> Default for StreamDecoder<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> StreamDecoder<T> {
    /// 创建空解码器.
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            poisoned: false,
            _marker: PhantomData,
        }
    }

    /// 喂入一段传输字节 (只累积, 不消费).
    pub fn push(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// 是否处于熔断状态 (帧边界已不可信).
    pub fn is_poisoned(&self) -> bool {
        self.poisoned
    }

    /// 当前缓存的未消费字节数 (半包观测用).
    pub fn buffered(&self) -> usize {
        self.buf.len()
    }
}

impl<T: DeserializeOwned> StreamDecoder<T> {
    /// 取下一帧: `Ok(None)` = 字节还不够 (等更多输入); 出错即熔断.
    pub fn next_frame(&mut self) -> Result<Option<T>, CodecError> {
        if self.poisoned {
            return Err(CodecError::new(
                CodecErrorKind::Poisoned,
                "decoder poisoned after earlier error",
            ));
        }
        if self.buf.len() < LENGTH_PREFIX_BYTES {
            return Ok(None);
        }
        let len = u32::from_be_bytes([self.buf[0], self.buf[1], self.buf[2], self.buf[3]]) as usize;
        if len == 0 || len > MAX_FRAME_BYTES {
            self.poisoned = true;
            return Err(CodecError::new(
                CodecErrorKind::Oversize,
                format!("declared frame length {len} out of range 1..={MAX_FRAME_BYTES}"),
            ));
        }
        if self.buf.len() < LENGTH_PREFIX_BYTES + len {
            return Ok(None);
        }
        let body: Vec<u8> = self.buf[LENGTH_PREFIX_BYTES..LENGTH_PREFIX_BYTES + len].to_vec();
        self.buf.drain(..LENGTH_PREFIX_BYTES + len);
        match decode_body::<T>(&body) {
            Ok(frame) => Ok(Some(frame)),
            Err(e) => {
                self.poisoned = true;
                Err(e)
            }
        }
    }

    /// 排空当前缓存中的所有完整帧 (顺序保留).
    pub fn drain_frames(&mut self) -> Result<Vec<T>, CodecError> {
        let mut out = Vec::new();
        while let Some(frame) = self.next_frame()? {
            out.push(frame);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(tag = "type", rename_all = "snake_case")]
    enum DemoFrame {
        Ping { seq: u64, value: String },
        Pong { seq: u64, ok: bool },
    }

    #[test]
    fn encode_frame_prefixes_big_endian_length() {
        let frame = DemoFrame::Ping {
            seq: 7,
            value: "x".repeat(10),
        };
        let bytes = encode_frame(&frame).expect("encode");
        let len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        assert_eq!(len, bytes.len() - LENGTH_PREFIX_BYTES);
        // 帧体 = UTF-8 JSON, value 10 字节必须完整进帧体
        let body = &bytes[LENGTH_PREFIX_BYTES..];
        assert!(std::str::from_utf8(body)
            .expect("utf8")
            .contains("xxxxxxxxxx"));
    }

    #[test]
    fn frame_roundtrip_single() {
        let frame = DemoFrame::Pong { seq: 42, ok: true };
        let bytes = encode_frame(&frame).expect("encode");
        let mut decoder = StreamDecoder::<DemoFrame>::new();
        decoder.push(&bytes);
        let decoded = decoder.next_frame().expect("decode").expect("one frame");
        assert_eq!(decoded, frame);
        assert!(decoder.next_frame().expect("drain").is_none());
    }

    #[test]
    fn frame_roundtrip_partial_feed_byte_by_byte() {
        let frame = DemoFrame::Ping {
            seq: 3,
            value: "partial".to_string(),
        };
        let bytes = encode_frame(&frame).expect("encode");
        let mut decoder = StreamDecoder::<DemoFrame>::new();
        for (i, b) in bytes.iter().enumerate() {
            decoder.push(&[*b]);
            let got = decoder.next_frame().expect("decode");
            if i + 1 < bytes.len() {
                assert!(got.is_none(), "must not produce frame before full feed");
            } else {
                assert_eq!(got.expect("final byte completes frame"), frame);
            }
        }
    }

    #[test]
    fn decoder_emits_multiple_frames_from_one_push() {
        let f1 = DemoFrame::Ping {
            seq: 1,
            value: "a".into(),
        };
        let f2 = DemoFrame::Pong { seq: 2, ok: false };
        let f3 = DemoFrame::Ping {
            seq: 3,
            value: "b".into(),
        };
        let mut bytes = encode_frame(&f1).expect("encode");
        bytes.extend_from_slice(&encode_frame(&f2).expect("encode"));
        bytes.extend_from_slice(&encode_frame(&f3).expect("encode"));
        // 再喂一个半包 (只喂第一帧之外的前缀)
        bytes.extend_from_slice(&[0, 0]);

        let mut decoder = StreamDecoder::<DemoFrame>::new();
        decoder.push(&bytes);
        let frames = decoder.drain_frames().expect("drain");
        assert_eq!(frames, vec![f1, f2, f3]);
        assert_eq!(decoder.buffered(), 2, "半包必须留在缓存");
    }

    #[test]
    fn decoder_rejects_zero_length_frame() {
        let mut decoder = StreamDecoder::<DemoFrame>::new();
        decoder.push(&[0, 0, 0, 0]);
        let err = decoder.next_frame().expect_err("zero length is illegal");
        assert_eq!(err.kind, CodecErrorKind::Oversize);
        assert!(decoder.is_poisoned());
    }

    #[test]
    fn decoder_rejects_oversize_declared_length_before_alloc() {
        let mut decoder = StreamDecoder::<DemoFrame>::new();
        // 声明 0xFFFFFFFF 长度: 必须在读取帧体前拒绝
        decoder.push(&[0xFF, 0xFF, 0xFF, 0xFF]);
        let err = decoder.next_frame().expect_err("oversize must be rejected");
        assert_eq!(err.kind, CodecErrorKind::Oversize);
        assert!(decoder.is_poisoned());
    }

    #[test]
    fn decoder_poisons_after_malformed_body() {
        let mut decoder = StreamDecoder::<DemoFrame>::new();
        let body = b"{\"type\":\"ping\""; // 缺字段 + 截断 JSON
        let mut bytes = (body.len() as u32).to_be_bytes().to_vec();
        bytes.extend_from_slice(body);
        decoder.push(&bytes);
        let err = decoder.next_frame().expect_err("malformed body");
        assert_eq!(err.kind, CodecErrorKind::Malformed);
        // 熔断后即使有合法帧也拒收 (帧边界已不可信)
        decoder.push(&encode_frame(&DemoFrame::Pong { seq: 1, ok: true }).expect("encode"));
        let err2 = decoder.next_frame().expect_err("poisoned");
        assert_eq!(err2.kind, CodecErrorKind::Poisoned);
    }

    #[test]
    fn decode_body_rejects_unknown_variant() {
        let err =
            decode_body::<DemoFrame>(br#"{"type":"bogus","seq":1}"#).expect_err("unknown variant");
        assert_eq!(err.kind, CodecErrorKind::Malformed);
    }

    #[test]
    fn codec_error_display_contains_kind_not_body() {
        let mut decoder = StreamDecoder::<DemoFrame>::new();
        let body = b"{\"type\":\"ping\",\"seq\":1,\"value\":\"secret-token\"ZZZ";
        let mut bytes = (body.len() as u32).to_be_bytes().to_vec();
        bytes.extend_from_slice(body);
        decoder.push(&bytes);
        let err = decoder.next_frame().expect_err("malformed");
        let text = err.to_string();
        assert!(text.contains("malformed"));
        assert!(
            !text.contains("secret-token"),
            "错误文本不得回显帧体: {text}"
        );
    }

    #[test]
    fn encode_frame_of_serializable_is_length_prefixed() {
        let bytes = encode_frame(&serde_json::json!({"type":"ping","seq":1})).expect("encode");
        let len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        assert_eq!(len, bytes.len() - LENGTH_PREFIX_BYTES);
    }
}
