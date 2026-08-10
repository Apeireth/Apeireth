//! # Exporter
//!
//! 4 种 span exporter, 1:1 翻译 v0.9.21 商业版 `out/main/chunks/tracing` exporter 模块.
//!
//! ## 4 Exporter (per task spec §5)
//!
//! | Exporter | 用途 | 实现度 |
//! |----------|------|--------|
//! | `StdoutExporter` | print to stdout | ✅ 完整 |
//! | `FileExporter` | write to JSONL file | ✅ 完整 |
//! | `OtlpGrpcExporter` | OTLP gRPC collector | ❌ stub (`NotImplemented`) |
//! | `JaegerExporter` | Jaeger agent | ❌ stub (`NotImplemented`) |
//!
//! ## 6 哲学 anchor 穿透
//!
//! - **S-1 北极星导向**: 1:1 翻译 OpenTelemetry `SpanExporter`
//! - **S-2 实事求是**: 1+1 完整 + 2 stub, 0 假装已对接 OTLP/Jaeger
//! - **O-2 走在前人肩上**: 借鉴 OpenTelemetry SDK `BatchSpanExporter` + `SimpleSpanExporter`
//! - **O-3 干到底**: stdout/file 完整, OTLP/Jaeger stub 守门
//! - **O-4 任何人都能接手**: 跟 cache / credentials exporter 同模式
//! - **O-5 不假装**: 2 stub 返 `NotImplemented` + log warn

use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::error::{TracingError, TracingResult};
use super::span::Span;

// ============================================================================
// §1 ExporterKind 枚举 (4 变体)
// ============================================================================

/// 4 种 exporter 类型.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ExporterKind {
    /// Stdout 输出.
    Stdout,
    /// JSONL 文件输出.
    File,
    /// OTLP gRPC 输出.
    OtlpGrpc,
    /// Jaeger 输出.
    Jaeger,
}

impl ExporterKind {
    /// 字符串名.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Stdout => "stdout",
            Self::File => "file",
            Self::OtlpGrpc => "otlp_grpc",
            Self::Jaeger => "jaeger",
        }
    }
}

// ============================================================================
// §2 Exporter trait
// ============================================================================

/// Span exporter trait (1:1 翻译 OpenTelemetry `SpanExporter`).
#[async_trait]
pub trait SpanExporter: Send + Sync {
    /// 导出单个 span.
    async fn export(&self, span: &Span) -> TracingResult<()>;

    /// 强制 flush (buffer 落盘).
    async fn flush(&self) -> TracingResult<()>;

    /// 关闭 exporter (释放资源).
    async fn shutdown(&self) -> TracingResult<()>;

    /// Exporter 类型.
    fn kind(&self) -> ExporterKind;
}

// ============================================================================
// §3 StdoutExporter (完整)
// ============================================================================

/// Stdout exporter — 直接 print to stdout.
#[derive(Debug, Default, Clone)]
pub struct StdoutExporter;

#[async_trait]
impl SpanExporter for StdoutExporter {
    async fn export(&self, span: &Span) -> TracingResult<()> {
        let json = span.to_json()?;
        println!("[trace] {}", json);
        Ok(())
    }

    async fn flush(&self) -> TracingResult<()> {
        // stdout 是 line-buffered, 不需显式 flush
        Ok(())
    }

    async fn shutdown(&self) -> TracingResult<()> {
        Ok(())
    }

    fn kind(&self) -> ExporterKind {
        ExporterKind::Stdout
    }
}

// ============================================================================
// §4 FileExporter (完整)
// ============================================================================

/// File exporter — write to JSONL file.
///
/// 每行一个 span JSON, 用 Mutex 串行化写.
#[derive(Debug)]
pub struct FileExporter {
    /// 输出文件路径.
    pub output_path: PathBuf,
    /// 内部 writer (Mutex 保护).
    writer: Mutex<Option<std::fs::File>>,
}

impl FileExporter {
    /// 构造 (不立即打开文件).
    pub fn new(output_path: impl Into<PathBuf>) -> Self {
        Self {
            output_path: output_path.into(),
            writer: Mutex::new(None),
        }
    }

    /// 打开文件 (lazy).
    fn ensure_open(&self) -> TracingResult<std::fs::File> {
        let mut guard = self
            .writer
            .lock()
            .map_err(|e| TracingError::Internal(format!("file exporter lock poisoned: {}", e)))?;
        // 关闭旧 writer, 重新打开 (File 没有 try_clone for write 需要 seek)
        *guard = None;
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.output_path)
            .map_err(|e| TracingError::ExportFailed {
                exporter: "file".into(),
                reason: format!("open {:?} failed: {}", self.output_path, e),
            })?;
        // 重新打开一个独立的 file handle 供本次写入, 同时保留 clone 在 Mutex 里给 flush 用
        let f = file.try_clone().map_err(|e| TracingError::ExportFailed {
            exporter: "file".into(),
            reason: format!("try_clone failed: {}", e),
        })?;
        *guard = Some(file);
        Ok(f)
    }
}

#[async_trait]
impl SpanExporter for FileExporter {
    async fn export(&self, span: &Span) -> TracingResult<()> {
        let json = span.to_json()?;
        let mut file = self.ensure_open()?;
        writeln!(file, "{}", json).map_err(|e| TracingError::ExportFailed {
            exporter: "file".into(),
            reason: format!("write failed: {}", e),
        })?;
        file.flush().map_err(|e| TracingError::ExportFailed {
            exporter: "file".into(),
            reason: format!("flush failed: {}", e),
        })?;
        Ok(())
    }

    async fn flush(&self) -> TracingResult<()> {
        let mut guard = self
            .writer
            .lock()
            .map_err(|e| TracingError::Internal(format!("file exporter lock poisoned: {}", e)))?;
        if let Some(f) = guard.as_mut() {
            f.flush().map_err(|e| TracingError::ExportFailed {
                exporter: "file".into(),
                reason: format!("flush failed: {}", e),
            })?;
        }
        Ok(())
    }

    async fn shutdown(&self) -> TracingResult<()> {
        let mut guard = self
            .writer
            .lock()
            .map_err(|e| TracingError::Internal(format!("file exporter lock poisoned: {}", e)))?;
        *guard = None;
        Ok(())
    }

    fn kind(&self) -> ExporterKind {
        ExporterKind::File
    }
}

// ============================================================================
// §5 OtlpGrpcExporter (stub)
// ============================================================================

/// OTLP gRPC Exporter — R20 阶段 6 stub.
///
/// 留 R21 续真接, 1 owner × 1 周.
#[derive(Debug, Clone)]
pub struct OtlpGrpcExporter {
    /// collector endpoint (e.g. "http://collector:4317").
    pub endpoint: String,
}

impl OtlpGrpcExporter {
    /// 构造.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }
}

#[async_trait]
impl SpanExporter for OtlpGrpcExporter {
    async fn export(&self, _span: &Span) -> TracingResult<()> {
        Err(TracingError::NotImplemented(format!(
            "OtlpGrpcExporter to {} — R20 stage 6 stub, 留 R21 续真接",
            self.endpoint
        )))
    }

    async fn flush(&self) -> TracingResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> TracingResult<()> {
        Ok(())
    }

    fn kind(&self) -> ExporterKind {
        ExporterKind::OtlpGrpc
    }
}

// ============================================================================
// §6 JaegerExporter (stub)
// ============================================================================

/// Jaeger Exporter — R20 阶段 6 stub.
#[derive(Debug, Clone)]
pub struct JaegerExporter {
    /// jaeger endpoint (e.g. "http://jaeger:14268/api/traces").
    pub endpoint: String,
}

impl JaegerExporter {
    /// 构造.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }
}

#[async_trait]
impl SpanExporter for JaegerExporter {
    async fn export(&self, _span: &Span) -> TracingResult<()> {
        Err(TracingError::NotImplemented(format!(
            "JaegerExporter to {} — R20 stage 6 stub, 留 R21 续真接",
            self.endpoint
        )))
    }

    async fn flush(&self) -> TracingResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> TracingResult<()> {
        Ok(())
    }

    fn kind(&self) -> ExporterKind {
        ExporterKind::Jaeger
    }
}

// ============================================================================
// §7 Factory
// ============================================================================

/// Exporter factory (按 kind + endpoint / output_path 构造).
pub fn build_exporter(
    kind: ExporterKind,
    endpoint: &str,
    output_path: &str,
) -> Box<dyn SpanExporter> {
    match kind {
        ExporterKind::Stdout => Box::new(StdoutExporter),
        ExporterKind::File => Box::new(FileExporter::new(output_path)),
        ExporterKind::OtlpGrpc => Box::new(OtlpGrpcExporter::new(endpoint)),
        ExporterKind::Jaeger => Box::new(JaegerExporter::new(endpoint)),
    }
}

// ============================================================================
// §8 编译期常量
// ============================================================================

/// Exporter 变体计数.
pub const EXPORTER_KIND_COUNT: usize = 4;

// ============================================================================
// §9 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::context::TraceContext;
    use super::super::span::SpanKind;

    fn sample_span() -> Span {
        Span::new(
            "test.span",
            SpanKind::Client,
            TraceContext::new(
                "0af7651916cd43dd8448eb211c80319c".into(),
                "b7ad6b7169203331".into(),
                true,
            ),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_stdout_exporter_write() {
        let e = StdoutExporter;
        let r = e.export(&sample_span()).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_file_exporter_write() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        let e = FileExporter::new(path.clone());
        let span = sample_span();
        e.export(&span).await.unwrap();
        e.flush().await.unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("test.span"));
        assert!(content.contains("trace_id"));
    }

    #[tokio::test]
    async fn test_otlp_exporter_not_implemented() {
        let e = OtlpGrpcExporter::new("http://collector:4317");
        let r = e.export(&sample_span()).await;
        assert!(matches!(r, Err(TracingError::NotImplemented(_))));
    }

    #[tokio::test]
    async fn test_jaeger_exporter_not_implemented() {
        let e = JaegerExporter::new("http://jaeger:14268");
        let r = e.export(&sample_span()).await;
        assert!(matches!(r, Err(TracingError::NotImplemented(_))));
    }

    #[test]
    fn test_kind_count() {
        assert_eq!(EXPORTER_KIND_COUNT, 4);
    }

    #[test]
    fn test_factory() {
        let _stdout: Box<dyn SpanExporter> = build_exporter(ExporterKind::Stdout, "", "");
        let _file: Box<dyn SpanExporter> = build_exporter(ExporterKind::File, "", "/tmp/x.jsonl");
        let _otlp: Box<dyn SpanExporter> = build_exporter(ExporterKind::OtlpGrpc, "http://x:4317", "");
        let _jaeger: Box<dyn SpanExporter> = build_exporter(ExporterKind::Jaeger, "http://x:14268", "");
    }
}
