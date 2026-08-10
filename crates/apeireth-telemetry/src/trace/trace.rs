//! # Trace
//!
//! `Trace` 是 distributed tracing 的根容器, 1:1 翻译 v0.9.21 商业版
//! `out/main/chunks/tracing` 的 Trace + Tracer.
//!
//! ## 字段
//!
//! - `trace_id` — 32 lowercase hex char (W3C TraceContext)
//! - `spans` — 全部 span (按时间序)
//! - `root_span_id` — root span 的 span_id
//! - `config` — TracingConfig (service + resource + sampler + exporter)
//!
//! ## 行为
//!
//! - 构造时按 `Sampler.should_sample` 决策
//! - 决策为采 → 启 exporter, end 时 export
//! - 决策为不采 → 跳过 export, 只记 metrics
//!
//! ## 6 哲学 anchor 穿透
//!
//! - **S-1 北极星导向**: 1:1 翻译 OpenTelemetry Tracer
//! - **S-2 实事求是**: trace_id 随机生成 (UUIDv4 → hex), 0 业务重设计
//! - **O-2 走在前人肩上**: 借鉴 OpenTelemetry SDK `Tracer` + `trace_id` 生成
//! - **O-3 干到底**: 5 字段 + 4 公开方法, 测试覆盖
//! - **O-4 任何人都能接手**: 跟 cache / credentials Trace 同模式
//! - **O-5 不假装**: trace_id K-1 强校验, sampled 决策后真接 sampler

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::config::TracingConfig;
use super::context::TraceContext;
use super::error::{TracingError, TracingResult};
use super::exporter::{build_exporter, SpanExporter};
use super::propagation::is_valid_span_id;
use super::propagation::is_valid_trace_id;
use super::sampler::Sampler;
use super::span::{Span, SpanKind};

// ============================================================================
// §1 Trace
// ============================================================================

/// Trace — 分布式追踪根容器.
pub struct Trace {
    /// trace_id (32 lowercase hex char).
    pub trace_id: String,
    /// 全部 spans (按时间序).
    pub spans: Vec<Span>,
    /// root span_id.
    pub root_span_id: Option<String>,
    /// 采样决策.
    pub sampled: bool,
    /// 配置.
    pub config: TracingConfig,
    /// Sampler (引用).
    sampler: Arc<dyn Sampler>,
    /// Exporter (引用).
    exporter: Box<dyn SpanExporter>,
}

impl Trace {
    /// 构造新 trace (按 sampler 决策).
    pub async fn new(config: TracingConfig, sampler: Arc<dyn Sampler>) -> TracingResult<Self> {
        config.validate()?;
        let trace_id = generate_trace_id();
        let sampled = sampler.should_sample(&trace_id, None).await;
        let exporter = build_exporter(
            config.exporter.kind,
            &config.exporter.endpoint,
            &config.exporter.output_path,
        );
        Ok(Self {
            trace_id,
            spans: Vec::new(),
            root_span_id: None,
            sampled,
            config,
            sampler,
            exporter,
        })
    }

    /// 派生 child trace (从 parent context 继承 trace_id + sampled).
    pub async fn child(
        parent_ctx: &TraceContext,
        config: TracingConfig,
        sampler: Arc<dyn Sampler>,
    ) -> TracingResult<Self> {
        config.validate()?;
        parent_ctx.validate()?;
        let sampled = sampler
            .should_sample(&parent_ctx.trace_id, Some(parent_ctx.sampled))
            .await;
        let exporter = build_exporter(
            config.exporter.kind,
            &config.exporter.endpoint,
            &config.exporter.output_path,
        );
        Ok(Self {
            trace_id: parent_ctx.trace_id.clone(),
            spans: Vec::new(),
            root_span_id: None,
            sampled,
            config,
            sampler,
            exporter,
        })
    }

    /// 启动 root span.
    pub async fn start_root(
        &mut self,
        name: impl Into<String>,
        kind: SpanKind,
    ) -> TracingResult<&Span> {
        if self.root_span_id.is_some() {
            return Err(TracingError::Internal(
                "root span already started".into(),
            ));
        }
        let span_id = generate_span_id();
        let ctx = TraceContext::new(self.trace_id.clone(), span_id.clone(), self.sampled);
        let mut span = Span::new(name, kind, ctx)?;
        span.attributes.insert(
            "service.name".to_string(),
            self.config.service.name.clone(),
        );
        self.root_span_id = Some(span_id);
        self.spans.push(span);
        Ok(self.spans.last().unwrap())
    }

    /// 启动 child span.
    pub async fn start_child(
        &mut self,
        name: impl Into<String>,
        kind: SpanKind,
        parent: &Span,
    ) -> TracingResult<&Span> {
        let span_id = generate_span_id();
        let child = Span::child(name, kind, parent, span_id)?;
        self.spans.push(child);
        Ok(self.spans.last().unwrap())
    }

    /// 结束 span (并 export 如果被采样).
    pub async fn end_span(&mut self, name: &str) -> TracingResult<()> {
        let pos = self
            .spans
            .iter()
            .position(|s| s.name == name && s.end_time_nanos == 0);
        let Some(idx) = pos else {
            return Err(TracingError::Internal(format!(
                "span not found or already ended: {}",
                name
            )));
        };
        self.spans[idx].end()?;
        if self.sampled {
            self.exporter.export(&self.spans[idx]).await?;
        }
        Ok(())
    }

    /// 导出所有 spans (调试用).
    pub async fn export_all(&self) -> TracingResult<()> {
        if !self.sampled {
            return Ok(());
        }
        for s in &self.spans {
            self.exporter.export(s).await?;
        }
        self.exporter.flush().await?;
        Ok(())
    }

    /// Flush exporter.
    pub async fn flush(&self) -> TracingResult<()> {
        self.exporter.flush().await
    }

    /// Shutdown trace (flush + close).
    pub async fn shutdown(self) -> TracingResult<()> {
        if self.sampled {
            self.exporter.flush().await?;
        }
        self.exporter.shutdown().await
    }

    /// Span 数.
    pub fn span_count(&self) -> usize {
        self.spans.len()
    }
}

/// 手动 Debug impl (跳过 sampler/exporter trait object).
impl std::fmt::Debug for Trace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Trace")
            .field("trace_id", &self.trace_id)
            .field("spans", &self.spans.len())
            .field("root_span_id", &self.root_span_id)
            .field("sampled", &self.sampled)
            .field("config", &self.config)
            .finish()
    }
}

// ============================================================================
// §2 ID 生成
// ============================================================================

/// 生成 trace_id (32 lowercase hex char).
///
/// 用 2 个 UUIDv4 拼成 32 hex char.
pub fn generate_trace_id() -> String {
    let u1 = Uuid::new_v4();
    let u2 = Uuid::new_v4();
    let mut s = format!("{}{}", u1.simple(), u2.simple());
    s.truncate(32);
    s
}

/// 生成 span_id (16 lowercase hex char).
pub fn generate_span_id() -> String {
    let u = Uuid::new_v4();
    let mut s = u.simple().to_string();
    s.truncate(16);
    s
}

// ============================================================================
// §3 编译期常量
// ============================================================================

/// Trace ID hex 长度.
pub const TRACE_ID_LEN: usize = 32;

/// Span ID hex 长度.
pub const SPAN_ID_LEN: usize = 16;

// ============================================================================
// §4 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::sampler::AlwaysOnSampler;
    use tempfile::NamedTempFile;

    fn test_config_file(path: std::path::PathBuf) -> TracingConfig {
        TracingConfig {
            exporter: super::super::config::ExporterConfig {
                kind: super::super::exporter::ExporterKind::File,
                output_path: path.to_string_lossy().to_string(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_trace_new() {
        let tmp = NamedTempFile::new().unwrap();
        let cfg = test_config_file(tmp.path().to_path_buf());
        let sampler = Arc::new(AlwaysOnSampler);
        let t = Trace::new(cfg, sampler).await.unwrap();
        assert!(is_valid_trace_id(&t.trace_id));
        assert_eq!(t.trace_id.len(), 32);
        assert!(t.sampled);
    }

    #[tokio::test]
    async fn test_root_child_spans() {
        let tmp = NamedTempFile::new().unwrap();
        let cfg = test_config_file(tmp.path().to_path_buf());
        let sampler = Arc::new(AlwaysOnSampler);
        let mut t = Trace::new(cfg, sampler).await.unwrap();
        let trace_id = t.trace_id.clone();

        let root = t.start_root("http.root", SpanKind::Server).await.unwrap();
        let root_id = root.context.span_id.clone();
        let root_clone = root.clone();

        let child = t
            .start_child("db.query", SpanKind::Client, &root_clone)
            .await
            .unwrap();
        assert_eq!(child.context.trace_id, trace_id);
        assert_eq!(child.parent_span_id.as_deref(), Some(root_id.as_str()));
        assert_eq!(t.span_count(), 2);
    }

    #[tokio::test]
    async fn test_end_span_exports() {
        let tmp = NamedTempFile::new().unwrap();
        let cfg = test_config_file(tmp.path().to_path_buf());
        let sampler = Arc::new(AlwaysOnSampler);
        let mut t = Trace::new(cfg, sampler).await.unwrap();

        t.start_root("op", SpanKind::Client).await.unwrap();
        t.end_span("op").await.unwrap();
        t.flush().await.unwrap();

        let content = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(content.contains("\"name\":\"op\""));
    }

    #[tokio::test]
    async fn test_root_already_started() {
        let tmp = NamedTempFile::new().unwrap();
        let cfg = test_config_file(tmp.path().to_path_buf());
        let sampler = Arc::new(AlwaysOnSampler);
        let mut t = Trace::new(cfg, sampler).await.unwrap();
        t.start_root("op1", SpanKind::Client).await.unwrap();
        let r = t.start_root("op2", SpanKind::Client).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn test_sampled_false_no_export() {
        let tmp = NamedTempFile::new().unwrap();
        let cfg = test_config_file(tmp.path().to_path_buf());
        let sampler = Arc::new(super::super::sampler::AlwaysOffSampler);
        let mut t = Trace::new(cfg, sampler).await.unwrap();
        assert!(!t.sampled);
        t.start_root("op", SpanKind::Client).await.unwrap();
        t.end_span("op").await.unwrap();
        t.flush().await.unwrap();
        // 文件不存在或为空
        let meta = std::fs::metadata(tmp.path()).unwrap();
        assert_eq!(meta.len(), 0);
    }

    #[test]
    fn test_generate_ids() {
        for _ in 0..100 {
            let tid = generate_trace_id();
            assert!(is_valid_trace_id(&tid));
            assert_eq!(tid.len(), 32);

            let sid = generate_span_id();
            assert!(is_valid_span_id(&sid));
            assert_eq!(sid.len(), 16);
        }
    }

    #[test]
    fn test_id_uniqueness() {
        // 1000 个 id 应该几乎不重复
        let mut ids: Vec<String> = (0..1000).map(|_| generate_trace_id()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 1000, "trace_id should be unique");
    }
}
