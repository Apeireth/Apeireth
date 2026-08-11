//! # Span
//!
//! `Span` 是 distributed tracing 的核心单元, 1:1 翻译 v0.9.21 商业版
//! `out/main/chunks/tracing` Span + Event + Status.
//!
//! ## 4 SpanKind (per task spec §4)
//!
//! | Kind | 用途 | 1:1 翻译 |
//! |------|------|----------|
//! | `Client` | 同步 HTTP/DB 客户端 | `SpanKind.CLIENT` |
//! | `Server` | 同步 HTTP/RPC 服务端 | `SpanKind.SERVER` |
//! | `Producer` | 异步消息生产 | `SpanKind.PRODUCER` |
//! | `Consumer` | 异步消息消费 | `SpanKind.CONSUMER` |
//!
//! ## 4 SpanEvent (per task spec §4)
//!
//! | Event | 用途 | 1:1 翻译 |
//! |-------|------|----------|
//! | `Log` | 自由文本日志 | `Event` (name + attrs) |
//! | `Exception` | 错误 + stack | `Exception` (error + stacktrace) |
//! | `Event` | 命名事件 + attrs | `Event` (named) |
//! | `Message` | 队列消息 (topic + body) | `Message` (queue/topic) |
//!
//! ## SpanStatus
//!
//! 3 状态: Unset / Ok / Error (message).
//!
//! ## 6 哲学 anchor 穿透
//!
//! - **S-1 北极星导向**: 1:1 翻译 OpenTelemetry Span, 0 业务重设计
//! - **S-2 实事求是**: 4 kind + 4 event + 3 status, 0 过度设计
//! - **O-2 走在前人肩上**: 借鉴 OpenTelemetry SDK Span + Event + Status
//! - **O-3 干到底**: 4 kind + 4 event 全实现, K-1 强校验
//! - **O-4 任何人都能接手**: 跟 credentials / cache Span 同模式
//! - **O-5 不假装**: 字段编译期 hardcode enum, 0 假装已对接

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::context::TraceContext;
use crate::error::{TracingError, TracingResult};

// ============================================================================
// §1 SpanKind 枚举 (4 变体)
// ============================================================================

/// 4 种 Span 类型.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SpanKind {
    /// 客户端 (发起请求).
    Client,
    /// 服务端 (接收请求).
    Server,
    /// 生产者 (发消息).
    Producer,
    /// 消费者 (收消息).
    Consumer,
}

impl SpanKind {
    /// 字符串名.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Client => "client",
            Self::Server => "server",
            Self::Producer => "producer",
            Self::Consumer => "consumer",
        }
    }
}

// ============================================================================
// §2 SpanEventKind 枚举 (4 变体)
// ============================================================================

/// 4 种 span 事件类型.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SpanEventKind {
    /// 自由文本日志.
    Log,
    /// 异常 (error + stack).
    Exception,
    /// 命名事件 + attrs.
    Event,
    /// 队列消息.
    Message,
}

impl SpanEventKind {
    /// 字符串名.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Log => "log",
            Self::Exception => "exception",
            Self::Event => "event",
            Self::Message => "message",
        }
    }
}

// ============================================================================
// §3 SpanStatus 枚举
// ============================================================================

/// Span 状态.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SpanStatus {
    /// 未设置 (默认).
    Unset,
    /// 成功.
    Ok,
    /// 错误.
    Error {
        /// 错误消息.
        message: String,
    },
}

impl Default for SpanStatus {
    fn default() -> Self {
        Self::Unset
    }
}

// ============================================================================
// §4 SpanEvent — span 内部事件
// ============================================================================

/// Span 内部事件.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpanEvent {
    /// 事件名.
    pub name: String,
    /// 事件类型.
    pub kind: SpanEventKind,
    /// 事件时间戳 (epoch nanos).
    pub timestamp_nanos: u128,
    /// 事件属性.
    pub attributes: HashMap<String, String>,
}

impl SpanEvent {
    /// Log 事件 (自由文本).
    pub fn log(message: impl Into<String>) -> Self {
        let msg = message.into();
        let mut attrs = HashMap::new();
        attrs.insert("message".to_string(), msg);
        Self {
            name: "log".to_string(),
            kind: SpanEventKind::Log,
            timestamp_nanos: now_nanos(),
            attributes: attrs,
        }
    }

    /// Exception 事件 (error + stack).
    pub fn exception(error: impl Into<String>, stack: impl Into<String>) -> Self {
        let mut attrs = HashMap::new();
        attrs.insert("error".to_string(), error.into());
        attrs.insert("stack".to_string(), stack.into());
        Self {
            name: "exception".to_string(),
            kind: SpanEventKind::Exception,
            timestamp_nanos: now_nanos(),
            attributes: attrs,
        }
    }

    /// Event 事件 (命名 + attrs).
    pub fn event(name: impl Into<String>, attrs: HashMap<String, String>) -> Self {
        Self {
            name: name.into(),
            kind: SpanEventKind::Event,
            timestamp_nanos: now_nanos(),
            attributes: attrs,
        }
    }

    /// Message 事件 (队列消息).
    pub fn message(topic: impl Into<String>, body: impl Into<String>) -> Self {
        let mut attrs = HashMap::new();
        attrs.insert("topic".to_string(), topic.into());
        attrs.insert("body".to_string(), body.into());
        Self {
            name: "message".to_string(),
            kind: SpanEventKind::Message,
            timestamp_nanos: now_nanos(),
            attributes: attrs,
        }
    }
}

// ============================================================================
// §5 Span
// ============================================================================

/// Span — 分布式追踪单元.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    /// trace context.
    pub context: TraceContext,
    /// parent span_id (None = root).
    pub parent_span_id: Option<String>,
    /// 操作名 (e.g. "http.get" / "db.query").
    pub name: String,
    /// Span 类型.
    pub kind: SpanKind,
    /// 开始时间 (epoch nanos).
    pub start_time_nanos: u128,
    /// 结束时间 (epoch nanos).
    pub end_time_nanos: u128,
    /// Span 属性.
    pub attributes: HashMap<String, String>,
    /// Span 事件列表.
    pub events: Vec<SpanEvent>,
    /// Span 状态.
    pub status: SpanStatus,
}

impl Span {
    /// 构造新 Span (root).
    pub fn new(name: impl Into<String>, kind: SpanKind, context: TraceContext) -> TracingResult<Self> {
        context.validate()?;
        Ok(Self {
            context,
            parent_span_id: None,
            name: name.into(),
            kind,
            start_time_nanos: now_nanos(),
            end_time_nanos: 0,
            attributes: HashMap::new(),
            events: Vec::new(),
            status: SpanStatus::Unset,
        })
    }

    /// 构造 child Span.
    pub fn child(
        name: impl Into<String>,
        kind: SpanKind,
        parent: &Span,
        new_span_id: String,
    ) -> TracingResult<Self> {
        let child_ctx = parent.context.child(new_span_id);
        let mut s = Self::new(name, kind, child_ctx)?;
        s.parent_span_id = Some(parent.context.span_id.clone());
        Ok(s)
    }

    /// 设置属性.
    pub fn set_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    /// 添加事件.
    pub fn add_event(&mut self, event: SpanEvent) {
        self.events.push(event);
    }

    /// 记录 Log 事件.
    pub fn log(&mut self, message: impl Into<String>) {
        self.add_event(SpanEvent::log(message));
    }

    /// 记录 Exception 事件.
    pub fn exception(&mut self, error: impl Into<String>, stack: impl Into<String>) {
        self.add_event(SpanEvent::exception(error, stack));
    }

    /// 记录 Event 事件.
    pub fn event(&mut self, name: impl Into<String>, attrs: HashMap<String, String>) {
        self.add_event(SpanEvent::event(name, attrs));
    }

    /// 记录 Message 事件.
    pub fn message(&mut self, topic: impl Into<String>, body: impl Into<String>) {
        self.add_event(SpanEvent::message(topic, body));
    }

    /// 标记状态 Ok.
    pub fn set_ok(&mut self) {
        self.status = SpanStatus::Ok;
    }

    /// 标记状态 Error.
    pub fn set_error(&mut self, message: impl Into<String>) {
        self.status = SpanStatus::Error {
            message: message.into(),
        };
    }

    /// 结束 span.
    pub fn end(&mut self) -> TracingResult<()> {
        if self.end_time_nanos != 0 {
            return Err(TracingError::Internal(
                "span already ended".into(),
            ));
        }
        self.end_time_nanos = now_nanos();
        if self.end_time_nanos < self.start_time_nanos {
            return Err(TracingError::Internal(
                "end_time < start_time".into(),
            ));
        }
        Ok(())
    }

    /// 持续时间 (millis).
    pub fn duration_millis(&self) -> u64 {
        if self.end_time_nanos < self.start_time_nanos {
            0
        } else {
            ((self.end_time_nanos - self.start_time_nanos) / 1_000_000) as u64
        }
    }

    /// 序列化为 JSON 字符串 (供 exporter 使用).
    pub fn to_json(&self) -> TracingResult<String> {
        serde_json::to_string(self).map_err(|e| {
            TracingError::Internal(format!("span serialize failed: {}", e))
        })
    }

    /// 人类可读的起始时间.
    pub fn start_time_iso(&self) -> String {
        let dt: DateTime<Utc> = DateTime::<Utc>::from_timestamp_nanos(self.start_time_nanos as i64);
        dt.to_rfc3339()
    }
}

// ============================================================================
// §6 编译期常量
// ============================================================================

/// Span 类型计数.
pub const SPAN_KIND_COUNT: usize = 4;

/// SpanEvent 类型计数.
pub const SPAN_EVENT_KIND_COUNT: usize = 4;

// ============================================================================
// §7 工具函数
// ============================================================================

/// 当前时间 (epoch nanos).
pub fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

// ============================================================================
// §8 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn root_ctx() -> TraceContext {
        TraceContext::new(
            "0af7651916cd43dd8448eb211c80319c".into(),
            "b7ad6b7169203331".into(),
            true,
        )
    }

    #[test]
    fn test_span_4_kinds() {
        assert_eq!(SpanKind::Client.as_str(), "client");
        assert_eq!(SpanKind::Server.as_str(), "server");
        assert_eq!(SpanKind::Producer.as_str(), "producer");
        assert_eq!(SpanKind::Consumer.as_str(), "consumer");
        assert_eq!(SPAN_KIND_COUNT, 4);
    }

    #[test]
    fn test_event_4_kinds() {
        assert_eq!(SpanEventKind::Log.as_str(), "log");
        assert_eq!(SpanEventKind::Exception.as_str(), "exception");
        assert_eq!(SpanEventKind::Event.as_str(), "event");
        assert_eq!(SpanEventKind::Message.as_str(), "message");
        assert_eq!(SPAN_EVENT_KIND_COUNT, 4);
    }

    #[test]
    fn test_span_new() {
        let s = Span::new("http.get", SpanKind::Client, root_ctx()).unwrap();
        assert_eq!(s.name, "http.get");
        assert_eq!(s.kind, SpanKind::Client);
        assert!(s.parent_span_id.is_none());
    }

    #[test]
    fn test_span_attribute_set_get() {
        let s = Span::new("db.query", SpanKind::Client, root_ctx())
            .unwrap()
            .set_attribute("db.statement", "SELECT 1")
            .set_attribute("db.system", "postgresql");
        assert_eq!(s.attributes.get("db.statement").unwrap(), "SELECT 1");
        assert_eq!(s.attributes.get("db.system").unwrap(), "postgresql");
    }

    #[test]
    fn test_span_event_log() {
        let mut s = Span::new("op", SpanKind::Client, root_ctx()).unwrap();
        s.log("starting");
        assert_eq!(s.events.len(), 1);
        assert_eq!(s.events[0].kind, SpanEventKind::Log);
    }

    #[test]
    fn test_span_event_exception() {
        let mut s = Span::new("op", SpanKind::Client, root_ctx()).unwrap();
        s.exception("io timeout", "at foo() at bar()");
        assert_eq!(s.events.len(), 1);
        assert_eq!(s.events[0].kind, SpanEventKind::Exception);
        assert_eq!(s.events[0].attributes.get("error").unwrap(), "io timeout");
    }

    #[test]
    fn test_span_event_named() {
        let mut s = Span::new("op", SpanKind::Client, root_ctx()).unwrap();
        let mut attrs = HashMap::new();
        attrs.insert("k".to_string(), "v".to_string());
        s.event("checkpoint", attrs);
        assert_eq!(s.events[0].kind, SpanEventKind::Event);
    }

    #[test]
    fn test_span_event_message() {
        let mut s = Span::new("op", SpanKind::Producer, root_ctx()).unwrap();
        s.message("orders", "{\"id\":1}");
        assert_eq!(s.events[0].kind, SpanEventKind::Message);
        assert_eq!(s.events[0].attributes.get("topic").unwrap(), "orders");
    }

    #[test]
    fn test_span_status_ok() {
        let mut s = Span::new("op", SpanKind::Client, root_ctx()).unwrap();
        s.set_ok();
        assert_eq!(s.status, SpanStatus::Ok);
    }

    #[test]
    fn test_span_status_error() {
        let mut s = Span::new("op", SpanKind::Client, root_ctx()).unwrap();
        s.set_error("oops");
        assert!(matches!(s.status, SpanStatus::Error { .. }));
    }

    #[test]
    fn test_span_end() {
        let mut s = Span::new("op", SpanKind::Client, root_ctx()).unwrap();
        s.end().unwrap();
        assert!(s.end_time_nanos >= s.start_time_nanos);
        // 重复 end 应失败
        assert!(s.end().is_err());
    }

    #[test]
    fn test_child_span() {
        let parent = Span::new("parent", SpanKind::Server, root_ctx()).unwrap();
        let child = Span::child(
            "child",
            SpanKind::Client,
            &parent,
            "aaaaaaaaaaaaaaaa".into(),
        )
        .unwrap();
        assert_eq!(child.context.trace_id, parent.context.trace_id);
        assert_eq!(child.parent_span_id.as_deref(), Some(parent.context.span_id.as_str()));
    }

    #[test]
    fn test_span_to_json() {
        let s = Span::new("op", SpanKind::Client, root_ctx()).unwrap();
        let j = s.to_json().unwrap();
        assert!(j.contains("http.get") || j.contains("op"));
        assert!(j.contains("trace_id"));
    }
}
