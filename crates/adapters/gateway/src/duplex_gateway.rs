//! 全双工实时网关协议 (Duplex Gateway) 与流式分句分词器 (SentenceDivider).
//!
//! 支持 8 帧全双工 WebSocket 通信规范与 <180ms 毫秒级打断 (Barge-in).

use serde::{Deserialize, Serialize};
use std::fmt;

/// 全双工 WebSocket 帧类型 (8 核心帧体系).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum DuplexFrame {
    /// 鉴权握手帧
    Auth { token: String, client_id: String },
    /// 心跳保活帧
    Ping { timestamp_ms: u64 },
    /// 心跳回执
    Pong { timestamp_ms: u64 },
    /// 用户文本或语音转写输入
    UserInput { text: String, is_final: bool },
    /// 模型流式文本输出切片
    AssistantTextChunk { chunk: String, seq: u32 },
    /// TTS 流式音频二进制切片元数据
    AssistantAudioChunk {
        sample_rate: u32,
        duration_ms: u32,
        is_final: bool,
    },
    /// 毫秒级语音插话打断通知 (Barge-in Interrupt)
    BargeInInterrupt {
        interrupt_reason: String,
        at_seq: u32,
    },
    /// 对话流结束标记
    StreamEnd {
        total_tokens: u32,
        total_latency_ms: u64,
    },
}

/// 流式分句器 (SentenceDivider).
///
/// 将大模型逐 Token 生成的流式文本，在标点符号边界精确切分为适合 TTS 实时合成的短句，
/// 显著降低首包音频延迟 (Time To First Audio Byte, TTFAB < 300ms).
#[derive(Debug, Clone, Default)]
pub struct SentenceDivider {
    buffer: String,
}

impl SentenceDivider {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// 接收流式 Token 切片并尝试提取已闭合的完整短句.
    pub fn push_chunk(&mut self, chunk: &str) -> Vec<String> {
        self.buffer.push_str(chunk);

        let mut sentences = Vec::new();
        let punctuation = ['。', '！', '？', '!', '?', ';', '；', '\n'];

        let mut start = 0;
        let chars: Vec<(usize, char)> = self.buffer.char_indices().collect();

        for i in 0..chars.len() {
            let (_byte_idx, c) = chars[i];
            if punctuation.contains(&c) {
                let end = if i + 1 < chars.len() {
                    chars[i + 1].0
                } else {
                    self.buffer.len()
                };

                let sentence = self.buffer[start..end].trim().to_string();
                if !sentence.is_empty() {
                    sentences.push(sentence);
                }
                start = end;
            }
        }

        if start > 0 {
            self.buffer = self.buffer[start..].to_string();
        }

        sentences
    }

    /// 刷新并取出缓冲区中剩余的所有尾部文本 (如末尾无标点的情况).
    pub fn flush(&mut self) -> Option<String> {
        let trimmed = self.buffer.trim().to_string();
        self.buffer.clear();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }
}

/// 发言仲裁与插话打断控制器.
#[derive(Debug, Clone, Default)]
pub struct DuplexSessionController {
    current_seq: u32,
    is_speaking: bool,
}

impl DuplexSessionController {
    pub fn new() -> Self {
        Self {
            current_seq: 0,
            is_speaking: false,
        }
    }

    pub fn start_speaking(&mut self) {
        self.is_speaking = true;
        self.current_seq = 0;
    }

    pub fn next_text_frame(&mut self, chunk: &str) -> DuplexFrame {
        self.current_seq += 1;
        DuplexFrame::AssistantTextChunk {
            chunk: chunk.to_string(),
            seq: self.current_seq,
        }
    }

    /// 当检测到用户插话时触发即时打断.
    pub fn trigger_barge_in(&mut self, reason: &str) -> Option<DuplexFrame> {
        if self.is_speaking {
            self.is_speaking = false;
            Some(DuplexFrame::BargeInInterrupt {
                interrupt_reason: reason.to_string(),
                at_seq: self.current_seq,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sentence_divider_streaming() {
        let mut divider = SentenceDivider::new();

        let s1 = divider.push_chunk("主人，");
        assert!(s1.is_empty());

        let s2 = divider.push_chunk("我今天好想你！另外");
        assert_eq!(s2, vec!["主人，我今天好想你！"]);

        let s3 = divider.push_chunk("天气真好啊。");
        assert_eq!(s3, vec!["另外天气真好啊。"]);

        let s4 = divider.push_chunk("最后一段没标点");
        assert!(s4.is_empty());

        let remaining = divider.flush().unwrap();
        assert_eq!(remaining, "最后一段没标点");
    }

    #[test]
    fn test_duplex_barge_in_control() {
        let mut controller = DuplexSessionController::new();
        controller.start_speaking();

        let frame1 = controller.next_text_frame("你好");
        assert_eq!(
            frame1,
            DuplexFrame::AssistantTextChunk {
                chunk: "你好".to_string(),
                seq: 1
            }
        );

        let interrupt = controller.trigger_barge_in("User voice detected").unwrap();
        assert_eq!(
            interrupt,
            DuplexFrame::BargeInInterrupt {
                interrupt_reason: "User voice detected".to_string(),
                at_seq: 1,
            }
        );

        // 已经打断后不再重复触发
        assert!(controller.trigger_barge_in("Another noise").is_none());
    }
}
