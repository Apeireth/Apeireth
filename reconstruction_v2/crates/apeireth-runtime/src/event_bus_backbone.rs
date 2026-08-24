//! EventBusBackbone - 多 channel 路由事件总线
//!
//! 0 装 PASS: 从 UnifiedRuntimeHost (host.rs:63 event_bus: Arc<EventBus>) 抽取 + 升级。
//! 旧版 EventBus 是单 channel (broadcast), 所有 topic 共用一条通道。
//! Backbone 把 channel 按业务领域拆开: emotion / tool / governance / dream / session ...
//!
//! 行为兼容性:
//! - publish(topic, payload) 默认走 "default" channel (== 旧版 EventBus 的语义)
//! - channel("governance").publish(...) 走 governance channel
//! - 各 channel 独立 buffer, 独立订阅者

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use apeireth_core::bus::{EventBus, EventEnvelope};
use apeireth_protocol::normalized::NormalizedMessage;

/// 预定义 channel 名 (0 装 PASS: 业务领域约定, 不是代码强约束)
pub mod channel_names {
    pub const DEFAULT: &str = "default";
    pub const EMOTION: &str = "emotion";
    pub const TOOL: &str = "tool";
    pub const GOVERNANCE: &str = "governance";
    pub const DREAM: &str = "dream";
    pub const SESSION: &str = "session";
    pub const PRESENCE: &str = "presence";
    pub const TELEMETRY: &str = "telemetry";
}

/// Backbone: 多 EventBus channel, 按 channel 名寻址。
pub struct EventBusBackbone {
    channels: RwLock<HashMap<String, Arc<EventBus>>>,
    default_channel_capacity: usize,
}

impl EventBusBackbone {
    /// 0 装 PASS: 默认构造 1 个 "default" channel, 容量 128 (与旧版 host.rs:113 一致)。
    pub fn new() -> Self {
        Self::with_capacity(128)
    }

    pub fn with_capacity(default_channel_capacity: usize) -> Self {
        let mut channels = HashMap::new();
        channels.insert(
            channel_names::DEFAULT.to_string(),
            Arc::new(EventBus::new(default_channel_capacity)),
        );
        Self { channels: RwLock::new(channels), default_channel_capacity }
    }

    /// 取出 (or 懒创建) 指定 channel 的 Arc<EventBus>。
    pub fn channel(&self, name: &str) -> Arc<EventBus> {
        {
            let channels = self.channels.read().expect("EventBusBackbone channels poisoned");
            if let Some(bus) = channels.get(name) {
                return bus.clone();
            }
        }
        // 没找到则懒创建
        let mut channels = self.channels.write().expect("EventBusBackbone channels poisoned");
        channels
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(EventBus::new(self.default_channel_capacity)))
            .clone()
    }

    /// 预注册 channel (避免首次 publish 才创建的延迟)。
    pub fn register_channel(&self, name: &str) {
        let mut channels = self.channels.write().expect("EventBusBackbone channels poisoned");
        channels
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(EventBus::new(self.default_channel_capacity)));
    }

    /// 0 装 PASS: 默认 channel 发布 (兼容旧版 host.rs event_bus.publish)
    pub fn publish(&self, topic: impl Into<String>, payload: impl Into<String>) -> usize {
        self.channel(channel_names::DEFAULT).publish(topic, payload)
    }

    /// 0 装 PASS: 默认 channel 订阅
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<EventEnvelope> {
        self.channel(channel_names::DEFAULT).subscribe()
    }

    /// 指定 channel 发布 (返回订阅者数量)
    pub fn publish_to(&self, channel: &str, topic: impl Into<String>, payload: impl Into<String>) -> usize {
        self.channel(channel).publish(topic, payload)
    }

    /// 列出当前已注册的 channel 名 (供 /v1/panel/endpoints 之类观察面板用)
    pub fn channels(&self) -> Vec<String> {
        self.channels
            .read()
            .expect("EventBusBackbone channels poisoned")
            .keys()
            .cloned()
            .collect()
    }
}

impl Default for EventBusBackbone {
    fn default() -> Self { Self::new() }
}

impl std::fmt::Debug for EventBusBackbone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let channels = self.channels.read().expect("EventBusBackbone channels poisoned");
        write!(f, "EventBusBackbone {{ channels: {:?} }}", channels.keys().collect::<Vec<_>>())
    }
}

/// 兼容旧版 EventBus 类型 (re-export, 让 host.rs 用法 0 改动)
pub use apeireth_core::bus::{EventBus as LegacyEventBus, Topic};

/// 0 装 PASS helper: 给 NormalizedMessage 自动包装 event envelope (供 handle_chat_turn 复用)
pub fn message_envelope(msg: &NormalizedMessage) -> EventEnvelope {
    EventEnvelope {
        topic: Topic("chat.message".to_string()),
        payload: serde_json::to_string(msg).unwrap_or_default(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_backbone_default_channel_works() {
        // 0 装 PASS: 默认 channel 等价旧版 EventBus
        let bb = EventBusBackbone::new();
        assert_eq!(bb.channels(), vec!["default".to_string()]);
        let mut rx = bb.subscribe();
        bb.publish("test.topic", "hello");
        let env = rx.recv().await.unwrap();
        assert_eq!(env.topic.0, "test.topic");
        assert_eq!(env.payload, "hello");
    }

    #[tokio::test]
    async fn test_backbone_multi_channel_isolation() {
        // 0 装 PASS: governance channel 的事件不应出现在 emotion channel
        let bb = EventBusBackbone::new();
        bb.register_channel("emotion");
        let mut emotion_rx = bb.channel("emotion").subscribe();
        let mut gov_rx = bb.channel("governance").subscribe();

        bb.publish_to("governance", "audit.event", "blocked");

        let env = gov_rx.recv().await.unwrap();
        assert_eq!(env.topic.0, "audit.event");

        // emotion channel 应无消息 (timeout check)
        let timeout = tokio::time::Duration::from_millis(50);
        match tokio::time::timeout(timeout, emotion_rx.recv()).await {
            Err(_) => {} // timeout = 验证通过 (没收到)
            Ok(Ok(_)) => panic!("emotion channel 不应收到 governance 事件"),
            Ok(Err(_)) => {}
        }
    }

    #[tokio::test]
    async fn test_backbone_lazy_channel_creation() {
        // 0 装 PASS: 首次访问 channel() 时懒创建
        let bb = EventBusBackbone::new();
        assert_eq!(bb.channels().len(), 1); // 只有 default
        let _bus = bb.channel("newly-created");
        assert_eq!(bb.channels().len(), 2);
    }
}
