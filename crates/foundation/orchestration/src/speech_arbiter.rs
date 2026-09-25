//! 发言权仲裁机 (SpeechOutputArbiter) 与双 AI 同台轮流调度矩阵.
//!
//! 吸收 Lumi_Nox 架构精髓，解决双/多 Agent 同台、桌面伴侣与实时弹幕/语音交互中的抢话、插话与发言饥饿问题.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

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

/// 发言队列上限 (M7: 防无界增长 — 插话风暴下旧实现无界 push_back).
pub const MAX_SPEECH_QUEUE: usize = 64;

/// `ttl_ms == 0` 时的默认存活期 (M7: 旧实现把 0 当"永不过期",
/// 一条排队请求可以无限期滞留队列; 现在统一吃默认 TTL 上限).
pub const DEFAULT_SPEECH_TTL_MS: u64 = 60_000;

impl SpeechOutputArbiter {
    pub fn new() -> Self {
        Self {
            current_speech: None,
            speech_queue: VecDeque::new(),
            speaker_turn_count: std::collections::HashMap::new(),
        }
    }

    /// 有效 TTL: `ttl_ms == 0` 不再意味着永不过期, 回落默认上限 (M7).
    fn effective_ttl_ms(ttl_ms: u64) -> u64 {
        if ttl_ms == 0 {
            DEFAULT_SPEECH_TTL_MS
        } else {
            ttl_ms
        }
    }

    /// 请求是否已过期 (M7: saturating_add 防 `created_at_ms + ttl_ms` 溢出回绕).
    fn is_expired(req: &SpeechRequest, now_ms: u64) -> bool {
        now_ms
            > req
                .created_at_ms
                .saturating_add(Self::effective_ttl_ms(req.ttl_ms))
    }

    /// 仲裁新的发言请求.
    pub fn arbitrate(
        &mut self,
        request: SpeechRequest,
        strategy: SpeechStrategy,
        now_ms: u64,
    ) -> ArbiterDecision {
        // 1. 检查请求自身是否已超时 (TTL 淘汰)
        if Self::is_expired(&request, now_ms) {
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
                    // M7: 队列有界 — 超限丢最旧 (排队最久者), 防插话风暴无界堆积.
                    while self.speech_queue.len() > MAX_SPEECH_QUEUE {
                        self.speech_queue.pop_front();
                    }
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

        // 清理队列中已超时的请求 (M7: saturating_add + 默认 TTL 口径与 arbitrate 一致)
        self.speech_queue
            .retain(|req| !Self::is_expired(req, now_ms));

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
        *self
            .speaker_turn_count
            .entry(req.speaker_id.clone())
            .or_insert(0) += 1;
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
        assert_eq!(
            arbiter.arbitrate(req1, SpeechStrategy::Queue, 1000),
            ArbiterDecision::GrantedImmediately
        );
        assert_eq!(arbiter.get_current_speaker().unwrap().speaker_id, "agent_a");

        // 2. Agent B 排队
        assert_eq!(
            arbiter.arbitrate(req2, SpeechStrategy::Queue, 1000),
            ArbiterDecision::Queued { queue_position: 1 }
        );

        // 3. User 强行打断
        let int_decision = arbiter.arbitrate(req3, SpeechStrategy::Interrupt, 1000);
        assert_eq!(
            int_decision,
            ArbiterDecision::InterruptedAndGranted {
                interrupted_speaker: Some("agent_a".to_string()),
            }
        );
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

    /// M7: `ttl_ms == 0` 不再永不过期 (旧实现: ttl=0 → retain 恒真 → 队列里永久滞留).
    #[test]
    fn m7_zero_ttl_uses_default_not_immortal() {
        let mut arbiter = SpeechOutputArbiter::new();
        let req = SpeechRequest {
            id: "z".to_string(),
            speaker_id: "agent_z".to_string(),
            content: "没有显式 TTL 的请求".to_string(),
            priority: 1,
            created_at_ms: 1000,
            ttl_ms: 0,
        };

        // 默认 TTL (60s) 内 → 正常排队 (先占住发言权).
        let holder = SpeechRequest {
            id: "h".to_string(),
            speaker_id: "agent_h".to_string(),
            content: "占麦".to_string(),
            priority: 1,
            created_at_ms: 1000,
            ttl_ms: 0,
        };
        arbiter.arbitrate(holder, SpeechStrategy::Queue, 1000);
        assert!(matches!(
            arbiter.arbitrate(req.clone(), SpeechStrategy::Queue, 61_000),
            ArbiterDecision::Queued { .. }
        ));
        // 超过默认 TTL → 入队前即被丢弃 (旧实现会永不过期).
        assert!(matches!(
            arbiter.arbitrate(req, SpeechStrategy::Queue, 61_001),
            ArbiterDecision::Dropped { .. }
        ));
    }

    /// M7: 队列有界 — 插话风暴超过 MAX_SPEECH_QUEUE 时丢最旧, 不得无界增长.
    #[test]
    fn m7_speech_queue_bounded() {
        let mut arbiter = SpeechOutputArbiter::new();
        // 先占住发言权, 后续请求才会排队.
        let holder = SpeechRequest {
            id: "holder".to_string(),
            speaker_id: "agent_h".to_string(),
            content: "长发言".to_string(),
            priority: 1,
            created_at_ms: 1000,
            ttl_ms: 0,
        };
        arbiter.arbitrate(holder, SpeechStrategy::Queue, 1000);

        let total = MAX_SPEECH_QUEUE + 20;
        for i in 0..total {
            let req = SpeechRequest {
                id: format!("q{i}"),
                speaker_id: "agent_q".to_string(),
                content: "插话".to_string(),
                priority: 1,
                created_at_ms: 1000,
                ttl_ms: 0,
            };
            let d = arbiter.arbitrate(req, SpeechStrategy::Queue, 1000);
            let ArbiterDecision::Queued { queue_position } = d else {
                panic!("应排队, 得到 {d:?}");
            };
            assert!(
                queue_position <= MAX_SPEECH_QUEUE,
                "队列长度越界: {queue_position}"
            );
        }
        // 只保留最后 MAX_SPEECH_QUEUE 个: 队首 = q{total - MAX} = q20.
        let next = arbiter.finish_current_speech(1000).expect("有排队者");
        assert_eq!(
            next.id,
            format!("q{}", total - MAX_SPEECH_QUEUE),
            "更早的插入应被淘汰"
        );
    }
}
