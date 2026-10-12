//! IM 快捷接入: 出站传输 (trait + HTTP 实现 + 断线重连预算)。
//!
//! - [`ImSender`] 是出站面的唯一抽象 (文本分段消息 / 审批卡片);
//! - [`ImHttpSender`] 把 kind 信封 POST 到渠道配置的 webhook / endpoint:
//!   2xx = 接受, 429/5xx/网络失败 = [`crate::im::error::ImError::Transport`]
//!   (可重试), 其余 4xx = `Rejected` (永久);
//! - [`ImReconnectPolicy`] 是断线重连预算 (指数退避 + 次数上限), 退避计划
//!   纯函数可测, 0 隐式无限重试。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::im::card::ImApprovalCard;
use crate::im::channel::{ImChannelKind, ImChannelTarget};
use crate::im::error::{ImError, ImResult};
use crate::im::message::{build_card_body, build_text_body, ImOutboundText, ImSendReceipt};

/// 出站面 (文本 + 卡片)。
#[async_trait]
pub trait ImSender: Send + Sync {
    /// 发一段已分段文本到 IM 会话。
    async fn send_text(
        &self,
        target: &ImChannelTarget,
        message: &ImOutboundText,
    ) -> ImResult<ImSendReceipt>;

    /// 发一张审批卡片到 IM 会话。
    async fn send_card(
        &self,
        target: &ImChannelTarget,
        conversation_id: &str,
        card: &ImApprovalCard,
    ) -> ImResult<ImSendReceipt>;
}

/// 断线重连预算 (指数退避, 有上限)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImReconnectPolicy {
    /// 总尝试次数 (含首次; ≥ 1)。
    pub max_attempts: u32,
    /// 退避基数 (毫秒)。
    pub backoff_base_ms: u64,
    /// 退避上限 (毫秒)。
    pub backoff_cap_ms: u64,
}

impl Default for ImReconnectPolicy {
    fn default() -> Self {
        Self {
            max_attempts: Self::DEFAULT_MAX_ATTEMPTS,
            backoff_base_ms: Self::DEFAULT_BACKOFF_BASE_MS,
            backoff_cap_ms: Self::DEFAULT_BACKOFF_CAP_MS,
        }
    }
}

impl ImReconnectPolicy {
    /// 默认尝试次数 (1 次发送 + 2 次重连)。
    pub const DEFAULT_MAX_ATTEMPTS: u32 = 3;
    /// 默认退避基数 (毫秒)。
    pub const DEFAULT_BACKOFF_BASE_MS: u64 = 200;
    /// 默认退避上限 (毫秒)。
    pub const DEFAULT_BACKOFF_CAP_MS: u64 = 5_000;

    /// 第 `attempt` 次重试前的退避毫秒 (attempt 从 1 计, 指数退避 + 封顶)。
    pub fn delay_ms(&self, attempt: u32) -> u64 {
        if attempt == 0 {
            return 0;
        }
        let exponent = attempt.saturating_sub(1).min(16);
        let delay = self.backoff_base_ms.saturating_mul(1u64 << exponent);
        delay.min(self.backoff_cap_ms)
    }

    /// 第 `attempt` 次失败后是否还重连 (只有可重试错误才重连)。
    pub fn should_retry(&self, attempt: u32, error: &ImError) -> bool {
        attempt < self.max_attempts && error.is_retryable()
    }
}

/// HTTP 出站实现 (reqwest; 信封由 [`build_text_body`] / [`build_card_body`] 组装)。
#[derive(Debug, Clone)]
pub struct ImHttpSender {
    client: reqwest::Client,
    reconnect: ImReconnectPolicy,
}

impl ImHttpSender {
    /// 构造 HTTP 发送器 (自带重连预算)。
    pub fn new(reconnect: ImReconnectPolicy) -> Self {
        Self {
            client: reqwest::Client::new(),
            reconnect,
        }
    }

    /// 重连预算。
    pub fn reconnect(&self) -> &ImReconnectPolicy {
        &self.reconnect
    }

    /// POST 一个信封体到渠道端点 (含重连)。
    pub async fn post_body(
        &self,
        target: &ImChannelTarget,
        body: &serde_json::Value,
    ) -> ImResult<ImSendReceipt> {
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            match self.post_once(target, body).await {
                Ok(receipt) => return Ok(receipt),
                Err(error) => {
                    if self.reconnect.should_retry(attempt, &error) {
                        let delay = self.reconnect.delay_ms(attempt);
                        if delay > 0 {
                            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                        }
                        continue;
                    }
                    return Err(error);
                }
            }
        }
    }

    async fn post_once(
        &self,
        target: &ImChannelTarget,
        body: &serde_json::Value,
    ) -> ImResult<ImSendReceipt> {
        let response = self
            .client
            .post(&target.endpoint)
            .json(body)
            .send()
            .await
            .map_err(|e| ImError::Transport {
                reason: format!("endpoint unreachable: {e}"),
            })?;
        let status = response.status();
        if status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Ok(ImSendReceipt {
                message_ref: message_ref_of(&text),
            });
        }
        if status.as_u16() == 429 || status.is_server_error() {
            return Err(ImError::Transport {
                reason: format!("endpoint returned {status}"),
            });
        }
        Err(ImError::Rejected {
            reason: format!("endpoint returned {status}"),
        })
    }
}

#[async_trait]
impl ImSender for ImHttpSender {
    async fn send_text(
        &self,
        target: &ImChannelTarget,
        message: &ImOutboundText,
    ) -> ImResult<ImSendReceipt> {
        let body = build_text_body(target.kind, message)?;
        self.post_body(target, &body).await
    }

    async fn send_card(
        &self,
        target: &ImChannelTarget,
        conversation_id: &str,
        card: &ImApprovalCard,
    ) -> ImResult<ImSendReceipt> {
        let body = build_card_body(target.kind, conversation_id, card)?;
        self.post_body(target, &body).await
    }
}

/// 从对端响应体取消息引用 (没有就用本地接受标记)。
fn message_ref_of(response: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(response) {
        for key in ["message_id", "msgid", "id"] {
            if let Some(reference) = value.get(key).and_then(serde_json::Value::as_str) {
                return reference.to_string();
            }
        }
    }
    "accepted".to_string()
}

/// 按 kind 取该渠道的默认消息预算 (便捷入口)。
pub fn outbound_budget(kind: ImChannelKind) -> usize {
    kind.max_text_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::im::error::ImErrorClass;

    #[test]
    fn reconnect_budget_is_exponential_and_capped() {
        let policy = ImReconnectPolicy {
            max_attempts: 4,
            backoff_base_ms: 100,
            backoff_cap_ms: 250,
        };
        assert_eq!(policy.delay_ms(0), 0);
        assert_eq!(policy.delay_ms(1), 100);
        assert_eq!(policy.delay_ms(2), 200);
        assert_eq!(policy.delay_ms(3), 250);
        assert_eq!(policy.delay_ms(9), 250);
    }

    #[test]
    fn reconnect_only_follows_retryable_failures_and_respects_the_budget() {
        let policy = ImReconnectPolicy::default();
        let transport = ImError::Transport {
            reason: "reset".into(),
        };
        let rejected = ImError::Rejected {
            reason: "endpoint returned 400".into(),
        };
        assert!(policy.should_retry(1, &transport));
        assert!(!policy.should_retry(policy.max_attempts, &transport));
        assert!(!policy.should_retry(1, &rejected));
        assert_eq!(transport.class(), ImErrorClass::Transport);
        assert!(!rejected.is_retryable());
    }
}
