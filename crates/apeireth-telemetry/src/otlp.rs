//! # 可选 OTLP 导出接口 (轻量, 0 重依赖)
//!
//! ## 审计结论 (2026-08)
//!
//! `apeireth-telemetry` 已有事件/指标基础设施:
//! - `metric::exporter::OtlpExporter` (metric 层) — 诚实 stub, 返 `ExporterNotImplemented`
//! - `trace::exporter::OtlpGrpcExporter` (trace 层) — 诚实 stub, 返 `NotImplemented`
//! - workspace `[workspace.dependencies]` **0 opentelemetry 依赖**, 本模块同样不引入
//!
//! 本模块补一个**可选 OTLP 导出器接口**: trait + 默认 Noop 实现 + JSON 行实现,
//! 给调用方一个在"接真 OTLP collector"之前的稳定扩展点 (0 新依赖:
//! std + serde_json + thiserror + async-trait 均为现有依赖)。
//!
//! ## 不假装 (0 假装)
//! - `NoopOtlpSink` 是默认实现: `emit` 返 `Ok(())` 但**丢弃事件**,
//!   `is_implemented() == false` — 调用方应先用 `is_implemented()` 判断,
//!   不要以为事件已导出。
//! - `JsonLinesOtlpSink` 是本地结构化输出 (console / file), **不是** OTLP 协议格式,
//!   仅用于开发期观察。
//! - 真 OTLP gRPC/HTTP 导出 (需 opentelemetry-proto) 留专门迁移, 走本 trait 接入。

use std::io::Write;
use std::sync::Mutex;

use serde::Serialize;
use thiserror::Error;

// ============================================================================
// §1 错误类型
// ============================================================================

/// OTLP sink 错误 (轻量 3 variant, 0 重依赖).
#[derive(Debug, Error)]
pub enum OtlpError {
    /// 底层 writer I/O 失败.
    #[error("otlp sink io error: {0}")]
    Io(String),
    /// 事件序列化失败.
    #[error("otlp event serialization error: {0}")]
    Serialize(String),
    /// sink 未实现 (stub 占位, 0 假装).
    #[error("otlp sink not implemented: {0}")]
    NotImplemented(String),
}

/// 统一 Result 别名.
pub type OtlpResult<T> = std::result::Result<T, OtlpError>;

// ============================================================================
// §2 事件类型
// ============================================================================

/// 一个可导出的 OTLP 事件 (与 OTLP 日志/事件模型对齐的轻量子集).
#[derive(Debug, Clone, Serialize)]
pub struct OtlpEvent {
    /// 事件名 (e.g. "llm.request").
    pub name: String,
    /// 事件时间戳 (unix millis).
    pub timestamp_unix_ms: u64,
    /// 属性键值对 (resource/attribute 语义).
    #[serde(default)]
    pub attributes: Vec<(String, String)>,
    /// 可选结构化 payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

impl OtlpEvent {
    /// 构造事件 (时间戳 = now).
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            timestamp_unix_ms: now_unix_ms(),
            attributes: Vec::new(),
            payload: None,
        }
    }

    /// 链式: 追加一个属性.
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.push((key.into(), value.into()));
        self
    }

    /// 链式: 设置结构化 payload.
    pub fn with_payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = Some(payload);
        self
    }
}

/// 当前 unix epoch millis.
fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ============================================================================
// §3 OtlpSink trait
// ============================================================================

/// 可选 OTLP 导出器接口 (轻量, 0 opentelemetry 重依赖).
///
/// 实现方可以是:
/// - `NoopOtlpSink` — 默认关闭态 (丢弃事件, `is_implemented() == false`)
/// - `JsonLinesOtlpSink` — 本地结构化 JSON 行输出 (开发期)
/// - 未来真 OTLP 导出器 (opentelemetry-proto) — 走本 trait 接入
#[async_trait::async_trait]
pub trait OtlpSink: Send + Sync {
    /// 导出/记录一个事件.
    async fn emit(&self, event: &OtlpEvent) -> OtlpResult<()>;

    /// sink 名 (e.g. "noop", "json_lines").
    fn name(&self) -> &'static str;

    /// 是否真导出 (false = noop/stub — 0 假装, 调用方据此判断).
    fn is_implemented(&self) -> bool;

    /// 强制 flush (默认 no-op, 有缓冲的实现覆写).
    async fn flush(&self) -> OtlpResult<()> {
        Ok(())
    }

    /// 关闭 sink (默认 no-op, 释放资源).
    async fn shutdown(&self) -> OtlpResult<()> {
        Ok(())
    }
}

// ============================================================================
// §4 NoopOtlpSink — 默认实现 (0 假装)
// ============================================================================

/// 默认 Noop sink: `emit` 返 `Ok(())` 但**丢弃事件**, `is_implemented() == false`.
///
/// 这是"OTel 关闭"的默认态 — 调用方不应以为事件已导出。
#[derive(Debug, Default)]
pub struct NoopOtlpSink;

impl NoopOtlpSink {
    /// 构造.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl OtlpSink for NoopOtlpSink {
    async fn emit(&self, _event: &OtlpEvent) -> OtlpResult<()> {
        // 有意丢弃 (0 假装): Noop 不导出任何数据, 用 is_implemented() 识别.
        Ok(())
    }

    fn name(&self) -> &'static str {
        "noop"
    }

    fn is_implemented(&self) -> bool {
        false
    }
}

// ============================================================================
// §5 JsonLinesOtlpSink — 本地结构化 JSON 行输出
// ============================================================================

/// JSON 行 sink: 每事件一行结构化 JSON, 写到底层 `Write` (console/file).
///
/// **不是** OTLP 协议格式 — 仅开发期本地观察用; 真 OTLP 走专门实现。
pub struct JsonLinesOtlpSink {
    writer: Mutex<Box<dyn Write + Send>>,
}

impl JsonLinesOtlpSink {
    /// 构造 (接管 writer, e.g. `Box::new(std::io::stdout())` 或文件).
    pub fn new(writer: Box<dyn Write + Send>) -> Self {
        Self {
            writer: Mutex::new(writer),
        }
    }
}

#[async_trait::async_trait]
impl OtlpSink for JsonLinesOtlpSink {
    async fn emit(&self, event: &OtlpEvent) -> OtlpResult<()> {
        let line = serde_json::to_string(event)
            .map_err(|e| OtlpError::Serialize(e.to_string()))?;
        let mut w = self
            .writer
            .lock()
            .map_err(|e| OtlpError::Io(format!("lock poisoned: {e}")))?;
        writeln!(w, "{line}").map_err(|e| OtlpError::Io(e.to_string()))?;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "json_lines"
    }

    fn is_implemented(&self) -> bool {
        true
    }

    async fn flush(&self) -> OtlpResult<()> {
        let mut w = self
            .writer
            .lock()
            .map_err(|e| OtlpError::Io(format!("lock poisoned: {e}")))?;
        w.flush().map_err(|e| OtlpError::Io(e.to_string()))
    }
}

// ============================================================================
// §6 便捷构造
// ============================================================================

/// 默认 sink: Noop (OTel 关闭态).
pub fn default_sink() -> NoopOtlpSink {
    NoopOtlpSink::new()
}

/// JSON 行 sink (本地开发观察).
pub fn json_lines_sink(writer: Box<dyn Write + Send>) -> JsonLinesOtlpSink {
    JsonLinesOtlpSink::new(writer)
}

// ============================================================================
// §7 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn sample_event() -> OtlpEvent {
        OtlpEvent::new("llm.request")
            .with_attribute("model", "MiniMax-M3")
            .with_payload(serde_json::json!({ "tokens": 128 }))
    }

    // ---- Noop ----

    #[tokio::test]
    async fn noop_emits_ok_but_is_not_implemented() {
        let sink = NoopOtlpSink::new();
        assert_eq!(sink.name(), "noop");
        assert!(!sink.is_implemented(), "Noop 必须标未实现 (0 假装)");
        let e = sample_event();
        assert!(sink.emit(&e).await.is_ok(), "Noop emit 返 Ok (丢弃事件)");
        assert!(sink.flush().await.is_ok());
        assert!(sink.shutdown().await.is_ok());
    }

    #[tokio::test]
    async fn default_sink_is_noop() {
        let sink = default_sink();
        assert_eq!(sink.name(), "noop");
        assert!(!sink.is_implemented());
    }

    // ---- JsonLines ----

    /// 测试 writer: 共享 buffer.
    struct TestWriter {
        buf: Arc<Mutex<Vec<u8>>>,
    }

    impl Write for TestWriter {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            self.buf.lock().unwrap().extend_from_slice(data);
            Ok(data.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn json_lines_writes_structured_json_line() {
        let shared = Arc::new(Mutex::new(Vec::<u8>::new()));
        let sink = JsonLinesOtlpSink::new(Box::new(TestWriter {
            buf: Arc::clone(&shared),
        }));
        assert!(sink.is_implemented());
        assert_eq!(sink.name(), "json_lines");

        sink.emit(&sample_event()).await.unwrap();
        sink.emit(&OtlpEvent::new("llm.response")).await.unwrap();
        sink.flush().await.unwrap();

        let bytes = shared.lock().unwrap().clone();
        let text = String::from_utf8(bytes).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2, "每事件一行: {text}");

        let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first["name"], "llm.request");
        assert_eq!(first["attributes"][0], serde_json::json!(["model", "MiniMax-M3"]));
        assert_eq!(first["payload"]["tokens"], 128);
        assert!(first["timestamp_unix_ms"].as_u64().unwrap() > 0);
    }

    #[tokio::test]
    async fn json_lines_shutdown_is_ok() {
        let shared = Arc::new(Mutex::new(Vec::<u8>::new()));
        let sink = JsonLinesOtlpSink::new(Box::new(TestWriter {
            buf: Arc::clone(&shared),
        }));
        assert!(sink.shutdown().await.is_ok());
    }

    // ---- 降级行为: writer 失败返明确错误 (0 假装) ----

    /// 总是失败的 writer.
    struct FailWriter;

    impl Write for FailWriter {
        fn write(&mut self, _data: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "boom"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn json_lines_propagates_io_error_honestly() {
        let sink = JsonLinesOtlpSink::new(Box::new(FailWriter));
        let e = sample_event();
        let err = sink.emit(&e).await.unwrap_err();
        assert!(matches!(err, OtlpError::Io(_)), "IO 失败必须返明确错误: {err}");
        assert!(format!("{err}").contains("boom"));
    }

    // ---- 事件构造 ----

    #[test]
    fn event_chaining_sets_fields() {
        let e = OtlpEvent::new("x")
            .with_attribute("k", "v")
            .with_payload(serde_json::json!({"n": 1}));
        assert_eq!(e.name, "x");
        assert_eq!(e.attributes, vec![("k".to_string(), "v".to_string())]);
        assert_eq!(e.payload, Some(serde_json::json!({"n": 1})));
        assert!(e.timestamp_unix_ms > 0);
    }
}
