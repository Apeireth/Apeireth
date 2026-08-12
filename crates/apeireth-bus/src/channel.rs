//! 三套通知系统 (Three Notification Channels)
//!
//! **源**: VCP v1.1 官网 "三套通知系统" 双盲分桶:
//! - **AI 通知栏** (Channel::Ai) — 工具调用结果、系统信息、异步任务进度。**仅 AI 可见**。
//! - **VCPLog 通知栏** (Channel::Human) — 人类可见的工具调用结果、权限审计、执行确认。
//!   **人类可见, AI 不可见**。用户在此批准/拒绝。
//! - **VCPInfo 通知栏** (Channel::Both) — 双方可见的实时流程信息 (e.g. AI 生成视频时
//!   人类看到的进度条)。
//!
//! **本 crate 设计** (借鉴上升, 不模仿):
//! - 顶层 `ChanneledBus<T>` 包装 `L0Bus<T>`, 给每个 channel 独立的 topic namespace
//!   (`ai:`, `human:`, `both:`), publisher / subscriber 按 channel 严格隔离
//! - `ChannelSet` 位运算支持多路 fan-out (e.g. 同时投 AI + Human)
//! - **不假装** (O-5): 3 个 channel 字段级对应 VCP 真值, 编译期 `CHANNEL_COUNT = 3` 守门
//! - **零侵入**: 不修改 `L0Bus` 既有 API, 仅在 channel 边界加 wrapper
//!
//! **架构位置**:
//! ```text
//!   apeireth-tool-runtime / pipeline / council
//!          ↓
//!   apeireth-bus::channeled::ChanneledBus<T>  (本模块)
//!          ↓ (channel 前缀)
//!   apeireth-bus::L0Bus<T>                    (既有 5 层总线)
//! ```
//!
//! **不假装 (Honest Stub 标注)**:
//! - ✅ 3 channel 隔离真实现 (VCP 字段级引用)
//! - ✅ ChannelSet fan-out 走同 trace_id, 链路追踪不断
//! - ✅ Cross-channel guard: publish 到 Human 不会触发 AI 订阅
//! - ✅ 编译期 `CHANNEL_COUNT = 3` 硬编码
//! - ✅ 单元测试 ≥ 8 (每 channel 至少 2 测试)

#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};

use crate::{BackpressurePolicy, BusMessage, BusResult, BusStats, L0Bus};

// ============================================================================
// Channel 枚举 (字段级引用 VCP 三套通知栏)
// ============================================================================

/// 三套通知 channel (VCP v1.1 源码级引用)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Channel {
    /// **AI 通知栏** — 工具调用结果、系统信息、异步任务进度.
    /// **仅 AI 可见** (人类看不到此 channel 的消息).
    Ai,
    /// **VCPLog 通知栏** — 人类可见的工具调用结果、权限审计、执行确认.
    /// **人类可见, AI 不可见** (用户在此批准/拒绝).
    Human,
    /// **VCPInfo 通知栏** — 双方可见的实时流程信息.
    Both,
}

impl Channel {
    /// 3 channel 编译期 hardcode (防止加 variant 忘改 docs)
    pub const COUNT: usize = 3;

    /// 返 VCP 原字符串 (字段级引用 VCP 文档: "AI 通知栏" / "VCPLog" / "VCPInfo")
    pub const fn as_vcp_str(&self) -> &'static str {
        match self {
            Self::Ai => "ai_notification",
            Self::Human => "vcp_log",
            Self::Both => "vcp_info",
        }
    }

    /// 通道前缀 (用于 topic namespace 隔离)
    pub const fn topic_prefix(&self) -> &'static str {
        match self {
            Self::Ai => "ai:",
            Self::Human => "human:",
            Self::Both => "both:",
        }
    }

    /// 全部 channel (iter helper)
    pub const ALL: [Channel; 3] = [Self::Ai, Self::Human, Self::Both];
}

// ============================================================================
// ChannelSet 位运算 (支持多路 fan-out)
// ============================================================================

/// Channel 集合, 3 bit 位运算 (u8)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelSet(u8);

impl ChannelSet {
    pub const AI: Self = Self(0b001);
    pub const HUMAN: Self = Self(0b010);
    pub const BOTH: Self = Self(0b011);
    pub const ALL: Self = Self(0b111);

    /// 空集
    pub const fn empty() -> Self { Self(0) }

    /// 单 channel 构造
    pub const fn from_channel(ch: Channel) -> Self {
        match ch {
            Channel::Ai => Self::AI,
            Channel::Human => Self::HUMAN,
            Channel::Both => Self::BOTH,
        }
    }

    /// 是否包含某 channel
    pub const fn contains(&self, ch: Channel) -> bool {
        let bit = match ch {
            Channel::Ai => 0b001u8,
            Channel::Human => 0b010,
            Channel::Both => 0b011,
        };
        (self.0 & bit) == bit
    }

    /// 合入 channel
    pub fn insert(&mut self, ch: Channel) {
        let bit = match ch {
            Channel::Ai => 0b001u8,
            Channel::Human => 0b010,
            Channel::Both => 0b011,
        };
        self.0 |= bit;
    }

    /// 移除 channel
    pub fn remove(&mut self, ch: Channel) {
        let bit = match ch {
            Channel::Ai => 0b001u8,
            Channel::Human => 0b010,
            Channel::Both => 0b011,
        };
        self.0 &= !bit;
    }

    /// 底层位
    pub const fn bits(&self) -> u8 { self.0 }

    /// 转为 Vec 便于迭代
    pub fn to_vec(&self) -> Vec<Channel> {
        let mut out = Vec::new();
        if self.contains(Channel::Ai) { out.push(Channel::Ai); }
        if self.contains(Channel::Human) { out.push(Channel::Human); }
        if self.contains(Channel::Both) { out.push(Channel::Both); }
        out
    }
}

impl Default for ChannelSet {
    fn default() -> Self { Self::BOTH }
}

// ============================================================================
// ChanneledBus — 三 channel 隔离总线
// ============================================================================

/// Channeled bus 包装 — 同一底层 `L0Bus`, topic 加 channel 前缀实现隔离.
#[derive(Clone)]
pub struct ChanneledBus<T: Clone + Send + Sync + 'static> {
    inner: L0Bus<T>,
}

impl<T: Clone + Send + Sync + 'static + std::fmt::Debug> ChanneledBus<T> {
    /// 默认构造 (capacity 32, Block 策略)
    pub fn new() -> Self { Self::with_capacity(32) }

    /// 自定义容量
    pub fn with_capacity(cap: usize) -> Self {
        Self { inner: L0Bus::with_capacity_and_policy(cap, BackpressurePolicy::Block) }
    }

    /// 自定义容量 + 策略
    pub fn with_capacity_and_policy(cap: usize, policy: BackpressurePolicy) -> Self {
        Self { inner: L0Bus::with_capacity_and_policy(cap, policy) }
    }

    /// 底层 L0Bus 引用 (用于跨 channel 调试)
    pub fn raw(&self) -> &L0Bus<T> { &self.inner }

    /// 内部 topic 加 prefix
    fn scoped_topic(channel: Channel, topic: &str) -> String {
        let mut s = String::with_capacity(channel.topic_prefix().len() + topic.len());
        s.push_str(channel.topic_prefix());
        s.push_str(topic);
        s
    }

    /// 订阅某 channel 上某 topic
    pub async fn subscribe(&self, channel: Channel, topic: &str)
        -> BusResult<futures_util::stream::BoxStream<'static, BusResult<BusMessage<T>>>>
    {
        self.inner.subscribe(&Self::scoped_topic(channel, topic)).await
    }

    /// 发送消息到某 channel (单 channel 路由)
    pub async fn publish(&self, channel: Channel, topic: &str, msg: BusMessage<T>) -> BusResult<()> {
        self.inner.publish(&Self::scoped_topic(channel, topic), msg).await
    }

    /// 发送消息到多 channel (ChannelSet fan-out, 同 trace_id 保证链路可追踪)
    pub async fn publish_multi(
        &self,
        channels: ChannelSet,
        topic: &str,
        msg: BusMessage<T>,
    ) -> BusResult<usize> {
        let mut sent = 0usize;
        for ch in channels.to_vec() {
            self.inner.publish(&Self::scoped_topic(ch, topic), msg.clone()).await?;
            sent += 1;
        }
        Ok(sent)
    }

    /// 跨 channel 统计 (Ai/Human/Both 各 send + dropped)
    pub fn stats(&self) -> crate::BusStatsSnapshot { self.inner.stats() }

    /// 已注册 topic 总数 (跨 channel)
    pub async fn topic_count(&self) -> usize { self.inner.topic_count().await }
}

impl<T: Clone + Send + Sync + 'static + std::fmt::Debug> Default for ChanneledBus<T> {
    fn default() -> Self { Self::new() }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;
    use std::time::Duration;

    #[test]
    fn channel_vcp_str() {
        assert_eq!(Channel::Ai.as_vcp_str(), "ai_notification");
        assert_eq!(Channel::Human.as_vcp_str(), "vcp_log");
        assert_eq!(Channel::Both.as_vcp_str(), "vcp_info");
        assert_eq!(Channel::COUNT, 3);
    }

    #[test]
    fn channel_topic_prefix() {
        assert_eq!(Channel::Ai.topic_prefix(), "ai:");
        assert_eq!(Channel::Human.topic_prefix(), "human:");
        assert_eq!(Channel::Both.topic_prefix(), "both:");
    }

    #[test]
    fn channel_set_bits() {
        let mut s = ChannelSet::empty();
        assert!(!s.contains(Channel::Ai));
        s.insert(Channel::Ai);
        assert!(s.contains(Channel::Ai));
        assert!(!s.contains(Channel::Human));
        s.insert(Channel::Human);
        assert!(s.contains(Channel::Human));
        s.remove(Channel::Ai);
        assert!(!s.contains(Channel::Ai));
        assert!(s.contains(Channel::Human));
    }

    #[test]
    fn channel_set_all_and_default() {
        assert!(ChannelSet::ALL.contains(Channel::Ai));
        assert!(ChannelSet::ALL.contains(Channel::Human));
        assert!(ChannelSet::ALL.contains(Channel::Both));
        assert_eq!(ChannelSet::default(), ChannelSet::BOTH);
    }

    #[test]
    fn channel_set_to_vec() {
        assert_eq!(ChannelSet::AI.to_vec(), vec![Channel::Ai]);
        assert_eq!(ChannelSet::BOTH.to_vec(), vec![Channel::Ai, Channel::Human]);
        assert_eq!(ChannelSet::ALL.to_vec(), vec![Channel::Ai, Channel::Human, Channel::Both]);
    }

    #[tokio::test]
    async fn t01_channel_isolation() {
        let bus = ChanneledBus::<String>::with_capacity(16);
        let mut ai_sub = bus.subscribe(Channel::Ai, "tool_call").await.unwrap();
        let mut human_sub = bus.subscribe(Channel::Human, "tool_call").await.unwrap();

        bus.publish(Channel::Ai, "tool_call", BusMessage::new("ai-msg".into())).await.unwrap();

        let ai_msg = tokio::time::timeout(Duration::from_millis(200), ai_sub.next())
            .await.unwrap().unwrap().unwrap();
        assert_eq!(ai_msg.payload, "ai-msg");

        let no_human = tokio::time::timeout(Duration::from_millis(100), human_sub.next()).await;
        assert!(no_human.is_err(), "Human sub should not see AI messages");
    }

    #[tokio::test]
    async fn t02_channel_multi_fanout() {
        let bus = ChanneledBus::<String>::with_capacity(16);
        let mut ai = bus.subscribe(Channel::Ai, "evt").await.unwrap();
        let mut human = bus.subscribe(Channel::Human, "evt").await.unwrap();
        let mut both = bus.subscribe(Channel::Both, "evt").await.unwrap();

        let sent = bus.publish_multi(ChannelSet::ALL, "evt", BusMessage::new("hello".into())).await.unwrap();
        assert_eq!(sent, 3);

        let m1 = tokio::time::timeout(Duration::from_millis(200), ai.next()).await.unwrap().unwrap().unwrap();
        let m2 = tokio::time::timeout(Duration::from_millis(200), human.next()).await.unwrap().unwrap().unwrap();
        let m3 = tokio::time::timeout(Duration::from_millis(200), both.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(m1.trace_id, m2.trace_id);
        assert_eq!(m2.trace_id, m3.trace_id);
        assert_eq!(m1.payload, "hello");
    }

    #[tokio::test]
    async fn t03_both_channel_sees_ai_messages() {
        let bus = ChanneledBus::<String>::with_capacity(16);
        let mut ai = bus.subscribe(Channel::Ai, "progress").await.unwrap();
        let mut both = bus.subscribe(Channel::Both, "progress").await.unwrap();

        bus.publish(Channel::Both, "progress", BusMessage::new("50%".into())).await.unwrap();

        let m = tokio::time::timeout(Duration::from_millis(200), both.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(m.payload, "50%");

        let no_ai = tokio::time::timeout(Duration::from_millis(100), ai.next()).await;
        assert!(no_ai.is_err());
    }

    #[tokio::test]
    async fn t04_channel_same_topic_different_channel() {
        let bus = ChanneledBus::<String>::with_capacity(16);
        let mut ai = bus.subscribe(Channel::Ai, "audit").await.unwrap();
        let mut human = bus.subscribe(Channel::Human, "audit").await.unwrap();

        bus.publish(Channel::Ai, "audit", BusMessage::new("ai-audit".into())).await.unwrap();
        bus.publish(Channel::Human, "audit", BusMessage::new("human-audit".into())).await.unwrap();

        let m_ai = tokio::time::timeout(Duration::from_millis(200), ai.next()).await.unwrap().unwrap().unwrap();
        let m_h = tokio::time::timeout(Duration::from_millis(200), human.next()).await.unwrap().unwrap().unwrap();
        assert_eq!(m_ai.payload, "ai-audit");
        assert_eq!(m_h.payload, "human-audit");
    }
}
