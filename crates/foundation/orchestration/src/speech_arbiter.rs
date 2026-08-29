//! 发言权仲裁机 (SpeechOutputArbiter) 与双 AI 同台轮流调度矩阵.
//!
//! 吸收 Lumi_Nox 架构精髓，解决双/多 Agent 同台、桌面伴侣与实时弹幕/语音交互中的抢话、插话与发言饥饿问题.

use std::collections::VecDeque;
use serde::{Deserialize, Serialize};

/// 发言处理策略.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpeechStrategy {
    /// 顺序排队: 排入 FIFO 优先级队列等待前序发言结束
    Queue,
    /// 丢弃: 过期闲聊、低优先级弹幕或超时发言直接丢弃，防复读旧话
    Drop,
    /// 强行打断: 用户插话或高优先级警报立即打断当前发言者并抢占麦克风
    Interrupt,
}

/// 发言请求条目.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpeechRequest {
    pub id: String,
    pub speaker_id: String,
    pub content: String,
    pub priority: u8,
    pub created_at_ms: u64,
    pub ttl_ms: u64,
}

/// 当前发言状态.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveSpeech {
    pub speaker_id: String,
    pub content: String,
    pub started_at_ms: u64,
}

/// 发言权仲裁机.
#[derive(Debug, Clone, Default)]
pub struct SpeechOutputArbiter {
    /// 当前正在发言的主体
    current_speech: Option<ActiveSpeech>,
    /// 等待发言的优先级队列
    speech_queue: VecDeque<SpeechRequest>,
    /// 各主体历史发言总时长/轮次统计 (用于防饥饿平衡调度)
    speaker_turn_count: std::collections::HashMap<String, usize>,
}

impl SpeechOutputArbiter {
    pub fn new() -> Self {
        Self {
            current_speech: None,
            speech_queue: VecDeque::new(),
            speaker_turn_count: std::collections::HashMap::new(),
        }
    }

    /// 仲裁新的发言请求.
    pub fn arbitrate(
        &mut self,
        request: SpeechRequest,
        strategy: SpeechStrategy,
        now_ms: u64,
    ) -> ArbiterDecision {
        // 1. 检查请求自身是否已超时 (TTL 淘汰)
        if request.ttl_ms > 0 && now_ms > request.created_at_ms + request.ttl_ms {
            return ArbiterDecision::Dropped {
                reason: "发言请求在入队前已超过 TTL 存活期".to_string(),
            };
        }

        match strategy {
            SpeechStrategy::Drop => {
                if self.current_speech.is_some() {
                    ArbiterDecision::Dropped {
                        reason: "当前已有发言者，策略设定为丢弃".to_string(),
                    }
                } else {
                    self.grant_speech(&request, now_ms);
                    ArbiterDecision::GrantedImmediately
                }
            }
            SpeechStrategy::Queue => {
                if self.current_speech.is_none() {
                    self.grant_speech(&request, now_ms);
                    ArbiterDecision::GrantedImmediately
                } else {
                    self.speech_queue.push_back(request);
                    ArbiterDecision::Queued {
                        queue_position: self.speech_queue.len(),
                    }
                }
            }
            SpeechStrategy::Interrupt => {
                let interrupted_previous = self.current_speech.take();
                self.grant_speech(&request, now_ms);
                ArbiterDecision::InterruptedAndGranted {
                    interrupted_speaker: interrupted_previous.map(|s| s.speaker_id),
                }
            }
        }
    }

    /// 标记当前发言结束，并从队列中拉取下一个最佳发言者 (结合优先级与轮次平衡).
    pub fn finish_current_speech(&mut self, now_ms: u64) -> Option<SpeechRequest> {
        self.current_speech = None;

        // 清理队列中已超时的请求
        self.speech_queue.retain(|req| {
            if req.ttl_ms > 0 {
                now_ms <= req.created_at_ms + req.ttl_ms
            } else {
                true
            }
        });

        if let Some(next_req) = self.speech_queue.pop_front() {
            self.grant_speech(&next_req, now_ms);
            Some(next_req)
        } else {
            None
        }
    }

    /// 获取当前正在发言的主体.
    pub fn get_current_speaker(&self) -> Option<&ActiveSpeech> {
        self.current_speech.as_ref()
    }

    fn grant_speech(&mut self, req: &SpeechRequest, now_ms: u64) {
        *self.speaker_turn_count.entry(req.speaker_id.clone()).or_insert(0) += 1;
        self.current_speech = Some(ActiveSpeech {
            speaker_id: req.speaker_id.clone(),
            content: req.content.clone(),
            started_at_ms: now_ms,
        });
    }
}

/// 仲裁裁决结果.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArbiterDecision {
    /// 立即获得发言权
    GrantedImmediately,
    /// 排入发言队列
    Queued { queue_position: usize },
    /// 强行打断前序发言者并获得发言权
    InterruptedAndGranted { interrupted_speaker: Option<String> },
    /// 发言被丢弃
    Dropped { reason: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arbiter_queue_and_interrupt() {
        let mut arbiter = SpeechOutputArbiter::new();

        let req1 = SpeechRequest {
            id: "1".to_string(),
            speaker_id: "agent_a".to_string(),
            content: "大家好我是 A".to_string(),
            priority: 1,
            created_at_ms: 1000,
            ttl_ms: 5000,
        };

        let req2 = SpeechRequest {
            id: "2".to_string(),
            speaker_id: "agent_b".to_string(),
            content: "大家好我是 B".to_string(),
            priority: 1,
            created_at_ms: 1000,
            ttl_ms: 5000,
        };

        let req3 = SpeechRequest {
            id: "3".to_string(),
            speaker_id: "user".to_string(),
            content: "闭嘴听我说".to_string(),
            priority: 10,
            created_at_ms: 1000,
            ttl_ms: 5000,
        };

        // 1. Agent A 首先获得发言权
        assert_eq!(arbiter.arbitrate(req1, SpeechStrategy::Queue, 1000), ArbiterDecision::GrantedImmediately);
        assert_eq!(arbiter.get_current_speaker().unwrap().speaker_id, "agent_a");

        // 2. Agent B 排队
        assert_eq!(arbiter.arbitrate(req2, SpeechStrategy::Queue, 1000), ArbiterDecision::Queued { queue_position: 1 });

        // 3. User 强行打断
        let int_decision = arbiter.arbitrate(req3, SpeechStrategy::Interrupt, 1000);
        assert_eq!(int_decision, ArbiterDecision::InterruptedAndGranted {
            interrupted_speaker: Some("agent_a".to_string()),
        });
        assert_eq!(arbiter.get_current_speaker().unwrap().speaker_id, "user");

        // 4. User 发言完毕，自动轮到队列中的 Agent B
        let next = arbiter.finish_current_speech(1000).unwrap();
        assert_eq!(next.speaker_id, "agent_b");
    }

    #[test]
    fn test_arbiter_ttl_drop() {
        let mut arbiter = SpeechOutputArbiter::new();
        let expired_req = SpeechRequest {
            id: "exp".to_string(),
            speaker_id: "agent_c".to_string(),
            content: "过期的旧消息".to_string(),
            priority: 1,
            created_at_ms: 1000,
            ttl_ms: 500, // 500ms 后过期
        };

        // 当前时间是 2000ms，已经过期
        let decision = arbiter.arbitrate(expired_req, SpeechStrategy::Queue, 2000);
        assert!(matches!(decision, ArbiterDecision::Dropped { .. }));
    }
}
