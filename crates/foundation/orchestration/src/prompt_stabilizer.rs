//! 大模型 Prompt Cache 字节级前缀稳定器与单点动态注入引擎.
//!
//! 吸收 NemesisBot 架构精髓，将 System Prompt、Persona 与已确认的历史对话构建为绝对不变的字节前缀，
//! 将高频变化的动态环境状态（时间、地点、电量、心流状态）严格限制在最新一条 User 消息之前单点注入，
//! 最大化 Anthropic / OpenAI / DeepSeek 等主流大模型厂商的 Prompt Cache 命中率 (80%+).

use serde::{Deserialize, Serialize};

/// 对话消息角色.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StabilizedRole {
    System,
    User,
    Assistant,
}

/// 规范化对话消息.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StabilizedMessage {
    pub role: StabilizedRole,
    pub content: String,
}

/// 动态环境状态快照 (仅在最新 User 消息前注入).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EphemeralContextSnapshot {
    pub current_time_str: String,
    pub device_status: String,
    pub active_mood: String,
}

impl EphemeralContextSnapshot {
    pub fn render_block(&self) -> String {
        format!(
            "[Ephemeral Context: Time={}, Device={}, Mood={}]",
            self.current_time_str, self.device_status, self.active_mood
        )
    }
}

/// Prompt 缓存稳定器.
#[derive(Debug, Clone, Default)]
pub struct PromptCacheStabilizer;

impl PromptCacheStabilizer {
    pub fn new() -> Self {
        Self
    }

    /// 装配具有极致 Prompt Cache 稳定性的消息流.
    ///
    /// 规则：
    /// 1. 不变的前缀 (System Prompt + 静态 Persona + 工具定义) 排在最前；
    /// 2. 历史轮次（除了最后一条 User 消息）完全保持原始字节不变，杜绝在历史消息中动态插值导致缓存雪崩；
    /// 3. 将 `EphemeralContextSnapshot` 精确追加到最新一条 User 消息的顶部.
    pub fn assemble_messages(
        system_prompt: &str,
        history: &[StabilizedMessage],
        latest_user_input: &str,
        ephemeral: &EphemeralContextSnapshot,
    ) -> Vec<StabilizedMessage> {
        let mut assembled = Vec::with_capacity(history.len() + 2);

        // 1. 固定前缀: System Prompt
        assembled.push(StabilizedMessage {
            role: StabilizedRole::System,
            content: system_prompt.to_string(),
        });

        // 2. 历史轮次原样注入 (保证字节级 100% 缓存命中)
        for msg in history {
            assembled.push(msg.clone());
        }

        // 3. 最新 User 消息: 单点注入动态环境状态
        let dynamic_block = ephemeral.render_block();
        let user_payload = if dynamic_block.is_empty() {
            latest_user_input.to_string()
        } else {
            format!("{}\n{}", dynamic_block, latest_user_input)
        };

        assembled.push(StabilizedMessage {
            role: StabilizedRole::User,
            content: user_payload,
        });

        assembled
    }

    /// 计算两次组装之间的公共稳定前缀字节比例 (用于观测 Prompt Cache 命中潜力).
    pub fn calculate_cache_prefix_ratio(
        previous: &[StabilizedMessage],
        current: &[StabilizedMessage],
    ) -> f32 {
        if previous.is_empty() || current.is_empty() {
            return 0.0;
        }

        let mut matching_chars = 0;
        let mut total_chars = 0;

        for msg in current {
            total_chars += msg.content.len();
        }

        let min_len = std::cmp::min(previous.len(), current.len());
        for i in 0..min_len {
            if previous[i] == current[i] {
                matching_chars += current[i].content.len();
            } else {
                break;
            }
        }

        if total_chars == 0 {
            1.0
        } else {
            matching_chars as f32 / total_chars as f32
        }
    }
}

/// Hydra-style tiered system-prompt assembly: sort by tier (lower first) and
/// concatenate. Recovered from canonical `prompt_cache::assemble_tiered`.
///
/// Typical tiers: `0` identity, `50` memory evidence, `100` tool guidance.
/// This is a string assembler only — it does not own a session or a cache.
pub fn assemble_tiered(parts: &[(u8, &str)]) -> String {
    let mut sorted: Vec<(u8, &str)> = parts.to_vec();
    sorted.sort_by_key(|(tier, _)| *tier);
    let mut s = String::new();
    for (_, content) in sorted {
        s.push_str(content);
        if !content.ends_with('\n') {
            s.push('\n');
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_cache_stabilization_assembly() {
        let sys = "You are Apeireth, an autonomous companion.";
        let history = vec![
            StabilizedMessage {
                role: StabilizedRole::User,
                content: "Hello!".to_string(),
            },
            StabilizedMessage {
                role: StabilizedRole::Assistant,
                content: "Hello! How can I help you today?".to_string(),
            },
        ];

        let ephemeral = EphemeralContextSnapshot {
            current_time_str: "2026-08-29 22:00".to_string(),
            device_status: "Battery 95%".to_string(),
            active_mood: "Happy".to_string(),
        };

        let assembled =
            PromptCacheStabilizer::assemble_messages(sys, &history, "What time is it?", &ephemeral);

        assert_eq!(assembled.len(), 4);
        assert_eq!(assembled[0].role, StabilizedRole::System);
        assert_eq!(assembled[1].content, "Hello!");
        assert_eq!(assembled[2].content, "Hello! How can I help you today?");
        // 验证动态信息被单点注入到最新 User 消息中
        assert!(assembled[3]
            .content
            .contains("[Ephemeral Context: Time=2026-08-29 22:00"));
        assert!(assembled[3].content.contains("What time is it?"));
    }

    #[test]
    fn test_cache_prefix_ratio() {
        let m1 = vec![
            StabilizedMessage {
                role: StabilizedRole::System,
                content: "System Prompt 1234567890".to_string(),
            },
            StabilizedMessage {
                role: StabilizedRole::User,
                content: "Old Message".to_string(),
            },
        ];
        let m2 = vec![
            StabilizedMessage {
                role: StabilizedRole::System,
                content: "System Prompt 1234567890".to_string(),
            },
            StabilizedMessage {
                role: StabilizedRole::User,
                content: "Old Message".to_string(),
            },
            StabilizedMessage {
                role: StabilizedRole::Assistant,
                content: "New reply".to_string(),
            },
        ];

        let ratio = PromptCacheStabilizer::calculate_cache_prefix_ratio(&m1, &m2);
        assert!(ratio > 0.6); // 前缀稳定命中率超过 60%
    }

    #[test]
    fn tiered_assembly_orders_by_tier() {
        let s = assemble_tiered(&[
            (100, "工具指引\n"),
            (0, "身份: 阿佩瑞斯\n"),
            (50, "记忆证据\n"),
        ]);
        let i0 = s.find("身份").unwrap();
        let i1 = s.find("记忆").unwrap();
        let i2 = s.find("工具").unwrap();
        assert!(i0 < i1 && i1 < i2, "tier 0 身份应最前: {s}");
    }

    #[test]
    fn tiered_assembly_adds_trailing_newline_when_missing() {
        let s = assemble_tiered(&[(1, "a"), (0, "b")]);
        assert_eq!(s, "b\na\n");
    }
}
