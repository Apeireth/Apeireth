//! # Structured JSON logging
//!
//! 1:1 翻译 v0.9.21 商业版 `out/main` observability 集成 (per blueprint §2.5.3).
//! 商业版 JSON log 输出到 stdout + 文件, 我们 skeleton 阶段走 `tracing-subscriber` 0.3 json feature
//! + 自家 `LogEntry` 渲染.
//!
//! ## LogEntry 字段
//!
//! - `timestamp` (UTC RFC 3339)
//! - `level` (`debug` / `info` / `warn` / `error`)
//! - `target` (crate / module path, 例: `apeireth_observability::tracing`)
//! - `message` (PII 自动脱敏, 走 `redact_pii`)
//! - `trace_id` (32 hex, 当前 span 的 trace_id, 可选)
//! - `span_id` (16 hex, 当前 span 的 span_id, 可选)
//! - `fields` (任意 key-value, structured)

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use super::{redact_pii, PLATFORM_NAME};

/// 日志级别 (5 档, per tracing 标准).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// 调试 (verbose)
    Debug,
    /// 信息 (默认)
    Info,
    /// 警告
    Warn,
    /// 错误
    Error,
    /// 严重 (panic 级)
    Critical,
}

impl LogLevel {
    /// 字符串 (per K-1 强校验).
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
            LogLevel::Critical => "critical",
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 单条日志条目 (1:1 翻译 OpenTelemetry log record).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogEntry {
    /// UTC RFC 3339 时间戳
    pub timestamp: DateTime<Utc>,
    /// 日志级别
    pub level: LogLevel,
    /// 模块路径 (crate::module, 例: `apeireth_observability::tracing`)
    pub target: String,
    /// 消息 (PII 自动脱敏)
    pub message: String,
    /// Trace ID (32 hex, 可选, 来自活跃 span)
    pub trace_id: Option<String>,
    /// Span ID (16 hex, 可选, 来自活跃 span)
    pub span_id: Option<String>,
    /// 平台名 (K-1 强校验, 永远 = "apeireth")
    pub platform: String,
    /// Schema 版本
    pub schema_version: String,
    /// 结构化字段
    pub fields: HashMap<String, serde_json::Value>,
}

impl LogEntry {
    /// 新建 (PII 自动脱敏 message).
    pub fn new(level: LogLevel, target: impl Into<String>, message: impl Into<String>) -> Self {
        let raw = message.into();
        let redacted = redact_pii(&raw).unwrap_or_else(|| {
            warn!("logging: PII redaction returned None, using original");
            raw.clone()
        });
        Self {
            timestamp: Utc::now(),
            level,
            target: target.into(),
            message: redacted,
            trace_id: None,
            span_id: None,
            platform: PLATFORM_NAME.to_string(),
            schema_version: "1".to_string(),
            fields: HashMap::new(),
        }
    }

    /// 加 trace context (链式).
    pub fn with_trace(mut self, trace_id: impl Into<String>, span_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self.span_id = Some(span_id.into());
        self
    }

    /// 加 field (链式, 任意 JSON value).
    pub fn with_field(mut self, k: impl Into<String>, v: impl Into<serde_json::Value>) -> Self {
        self.fields.insert(k.into(), v.into());
        self
    }

    /// 序列化为 JSON 字符串 (1:1 翻译商业版 JSON log line).
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// 记录结构化日志 (PII 脱敏 + JSON 序列化 + tracing info 输出).
///
/// 1:1 翻译 OpenTelemetry `logger.emit()`.
pub fn log_structured(
    level: LogLevel,
    target: &str,
    message: &str,
    fields: Option<HashMap<String, serde_json::Value>>,
) -> LogEntry {
    let mut entry = LogEntry::new(level, target, message);
    if let Some(f) = fields {
        entry.fields.extend(f);
    }

    // tracing 输出 (让 tracing-subscriber json feature 接管实际 sink)
    // 注意: tracing macro 的 `target` 字段必须是字符串字面值, 不能用变量. 把 target 放 entry 已含.
    let target_owned = target.to_string();
    match level {
        LogLevel::Debug => {
            info!(target = "apeireth_observability", module = %target_owned, log_entry = ?entry, "structured log")
        }
        LogLevel::Info => {
            info!(target = "apeireth_observability", module = %target_owned, log_entry = ?entry, "structured log")
        }
        LogLevel::Warn => {
            warn!(target = "apeireth_observability", module = %target_owned, log_entry = ?entry, "structured log")
        }
        LogLevel::Error => {
            tracing::error!(target = "apeireth_observability", module = %target_owned, log_entry = ?entry, "structured log")
        }
        LogLevel::Critical => {
            tracing::error!(target = "apeireth_observability", module = %target_owned, critical = true, "structured log")
        }
    }

    entry
}

// ============================================================================
// 单元测试 (in-module)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_entry_redacts_pii() {
        let entry = LogEntry::new(LogLevel::Info, "test", "password=secret123");
        assert!(entry.message.contains("password=***"));
        assert!(!entry.message.contains("secret123"), "明文严禁出现");
    }

    #[test]
    fn log_entry_redacts_token() {
        let entry = LogEntry::new(LogLevel::Info, "test", "api_token=abc-def");
        assert!(entry.message.contains("api_token=***"));
        assert!(!entry.message.contains("abc-def"));
    }

    #[test]
    fn log_entry_with_trace() {
        let entry = LogEntry::new(LogLevel::Info, "test", "hello")
            .with_trace("0af7651916cd43dd8448eb211c80319c", "b7ad6b7169203331");
        assert_eq!(
            entry.trace_id.as_deref(),
            Some("0af7651916cd43dd8448eb211c80319c")
        );
        assert_eq!(entry.span_id.as_deref(), Some("b7ad6b7169203331"));
    }

    #[test]
    fn log_entry_with_field() {
        let entry = LogEntry::new(LogLevel::Info, "test", "msg")
            .with_field("user_id", serde_json::json!(42))
            .with_field("endpoint", serde_json::json!("/api/v1"));
        assert_eq!(entry.fields.get("user_id").unwrap(), &serde_json::json!(42));
    }

    #[test]
    fn log_entry_serde_roundtrip() {
        let entry = LogEntry::new(LogLevel::Warn, "apeireth_observability::test", "test msg")
            .with_trace("0af7651916cd43dd8448eb211c80319c", "b7ad6b7169203331")
            .with_field("k", serde_json::json!("v"));
        let json = entry.to_json().expect("serialize");
        let back: LogEntry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.level, LogLevel::Warn);
        assert_eq!(back.target, "apeireth_observability::test");
        assert_eq!(
            back.trace_id.as_deref(),
            Some("0af7651916cd43dd8448eb211c80319c")
        );
    }

    #[test]
    fn log_level_as_str() {
        assert_eq!(LogLevel::Debug.as_str(), "debug");
        assert_eq!(LogLevel::Info.as_str(), "info");
        assert_eq!(LogLevel::Warn.as_str(), "warn");
        assert_eq!(LogLevel::Error.as_str(), "error");
        assert_eq!(LogLevel::Critical.as_str(), "critical");
    }

    #[test]
    fn log_structured_does_not_panic() {
        let mut fields = HashMap::new();
        fields.insert("count".to_string(), serde_json::json!(42));
        let entry = log_structured(LogLevel::Info, "test", "hello", Some(fields));
        assert_eq!(entry.level, LogLevel::Info);
    }
}
