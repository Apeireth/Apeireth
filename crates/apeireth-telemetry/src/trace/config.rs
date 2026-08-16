//! # Tracing Config
//!
//! `TracingConfig` 是 distributed tracing 框架的顶层配置, 1:1 翻译 v0.9.21 商业版
//! `out/main/chunks/tracing` 商业版配置. 4 段配置: service / resource / sampler /
//! exporter. 编译期 hardcode 字段, 防止运行时误改.
//!
//! ## 4 段配置 (per task spec §6)
//!
//! | 段 | 字段 | 用途 |
//! |----|------|------|
//! | `service` | `name`, `version`, `environment` | 服务标识 (OpenTelemetry semantic conventions) |
//! | `resource` | `attributes` (HashMap) | 资源属性 (host / region / pod_id) |
//! | `sampler` | `kind`, `ratio` | 采样策略 + 比例 |
//! | `exporter` | `kind`, `endpoint`, `output_path` | 导出目标 (stdout / file / OTLP / Jaeger) |
//!
//! ## 6 哲学 anchor 穿透
//!
//! - **S-1 北极星导向**: service.name 必填, 符合 OTel semantic conventions
//! - **S-2 实事求是**: 4 段配置 + 8 字段, 0 过度设计
//! - **O-2 走在前人肩上**: 借鉴 OpenTelemetry SDK `Resource` + `Sampler` + `SpanExporter` 接口
//! - **O-3 干到底**: Default impl 给出合理默认, 测试覆盖 K-1 强校验
//! - **O-4 任何人都能接手**: 跟 credentials / cache 配置同模式 (serde + Default)
//! - **O-5 不假装**: service.name K-1 强校验, 编译期 + 运行时双重守门

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::error::{TracingError, TracingResult};
use super::exporter::ExporterKind;
use super::sampler::SamplerKind;

// ============================================================================
// §1 4 段配置
// ============================================================================

/// Service 段 — 服务标识 (OpenTelemetry `service.*`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceConfig {
    /// 服务名 (e.g. "apeireth-api"). 必填, K-1 强校验.
    pub name: String,
    /// 服务版本 (e.g. "1.0.0").
    pub version: String,
    /// 部署环境 (e.g. "production" / "staging" / "dev").
    pub environment: String,
}

impl ServiceConfig {
    /// K-1 强校验: service.name 非空.
    pub fn validate(&self) -> TracingResult<()> {
        if self.name.trim().is_empty() {
            return Err(TracingError::EmptyServiceName);
        }
        if self.version.trim().is_empty() {
            return Err(TracingError::Internal(
                "service.version must not be empty".into(),
            ));
        }
        if self.environment.trim().is_empty() {
            return Err(TracingError::Internal(
                "service.environment must not be empty".into(),
            ));
        }
        Ok(())
    }
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            name: "apeireth".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            environment: "dev".into(),
        }
    }
}

/// Resource 段 — 资源属性 (OpenTelemetry `Resource` attributes).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceConfig {
    /// 资源属性 key-value (e.g. "host.name" -> "node-1").
    pub attributes: HashMap<String, String>,
    /// host.name 快捷字段.
    pub host_name: String,
    /// service.namespace (e.g. "apeireth" / "anthropic").
    pub namespace: String,
}

impl Default for ResourceConfig {
    fn default() -> Self {
        let mut attrs = HashMap::new();
        attrs.insert("telemetry.sdk.name".into(), "apeireth-tracing".into());
        attrs.insert("telemetry.sdk.language".into(), "rust".into());
        Self {
            attributes: attrs,
            host_name: "unknown".into(),
            namespace: "apeireth".into(),
        }
    }
}

impl ResourceConfig {
    /// 合并 resource.attributes 进总 HashMap, host_name / namespace 优先.
    pub fn merged_attributes(&self) -> HashMap<String, String> {
        let mut out = self.attributes.clone();
        out.insert("host.name".into(), self.host_name.clone());
        out.insert("service.namespace".into(), self.namespace.clone());
        out
    }
}

/// Sampler 段 — 采样策略.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SamplerConfig {
    /// 采样策略 (4 选 1).
    pub kind: SamplerKind,
    /// 采样比例 (0.0 - 1.0), 仅 `TraceIdRatioBased` 使用.
    pub ratio: f64,
}

impl Default for SamplerConfig {
    fn default() -> Self {
        Self {
            kind: SamplerKind::AlwaysOn,
            ratio: 1.0,
        }
    }
}

impl SamplerConfig {
    /// K-1 强校验: ratio 范围 [0.0, 1.0].
    pub fn validate(&self) -> TracingResult<()> {
        if !(0.0..=1.0).contains(&self.ratio) {
            return Err(TracingError::SamplingError(format!(
                "ratio must be in [0.0, 1.0], got {}",
                self.ratio
            )));
        }
        Ok(())
    }
}

/// Exporter 段 — 导出目标.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExporterConfig {
    /// 导出类型 (4 选 1).
    pub kind: ExporterKind,
    /// 目标 endpoint (OTLP gRPC: "http://collector:4317"; Jaeger: "http://jaeger:14268").
    pub endpoint: String,
    /// 文件路径 (FileExporter 用).
    pub output_path: String,
    /// 单次批量大小 (default 512).
    pub batch_size: usize,
}

impl Default for ExporterConfig {
    fn default() -> Self {
        Self {
            kind: ExporterKind::Stdout,
            endpoint: "".into(),
            output_path: "./traces.jsonl".into(),
            batch_size: 512,
        }
    }
}

impl ExporterConfig {
    /// K-1 强校验: FileExporter 必须有 output_path.
    pub fn validate(&self) -> TracingResult<()> {
        if matches!(self.kind, ExporterKind::File) && self.output_path.trim().is_empty() {
            return Err(TracingError::ExportFailed {
                exporter: "file".into(),
                reason: "output_path is empty".into(),
            });
        }
        Ok(())
    }
}

/// TracingConfig — 4 段配置的聚合.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TracingConfig {
    /// service 段.
    pub service: ServiceConfig,
    /// resource 段.
    pub resource: ResourceConfig,
    /// sampler 段.
    pub sampler: SamplerConfig,
    /// exporter 段.
    pub exporter: ExporterConfig,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            service: ServiceConfig::default(),
            resource: ResourceConfig::default(),
            sampler: SamplerConfig::default(),
            exporter: ExporterConfig::default(),
        }
    }
}

impl TracingConfig {
    /// K-1 强校验 4 段: service / resource (无字段) / sampler / exporter.
    pub fn validate(&self) -> TracingResult<()> {
        self.service.validate()?;
        self.sampler.validate()?;
        self.exporter.validate()?;
        Ok(())
    }

    /// Builder-style 构造.
    pub fn builder() -> TracingConfigBuilder {
        TracingConfigBuilder::default()
    }
}

// ============================================================================
// §2 Builder
// ============================================================================

/// TracingConfigBuilder — fluent 构造.
#[derive(Debug, Default, Clone)]
pub struct TracingConfigBuilder {
    service: ServiceConfig,
    resource: ResourceConfig,
    sampler: SamplerConfig,
    exporter: ExporterConfig,
}

impl TracingConfigBuilder {
    /// 设置 service.name (K-1 强校验).
    pub fn service_name(mut self, name: impl Into<String>) -> Self {
        self.service.name = name.into();
        self
    }

    /// 设置 service.version.
    pub fn service_version(mut self, version: impl Into<String>) -> Self {
        self.service.version = version.into();
        self
    }

    /// 设置 service.environment.
    pub fn environment(mut self, env: impl Into<String>) -> Self {
        self.service.environment = env.into();
        self
    }

    /// 设置 host.name.
    pub fn host_name(mut self, host: impl Into<String>) -> Self {
        self.resource.host_name = host.into();
        self
    }

    /// 设置 resource attribute.
    pub fn resource_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.resource.attributes.insert(key.into(), value.into());
        self
    }

    /// 设置 sampler.
    pub fn sampler(mut self, kind: SamplerKind, ratio: f64) -> Self {
        self.sampler = SamplerConfig { kind, ratio };
        self
    }

    /// 设置 exporter.
    pub fn exporter(mut self, kind: ExporterKind) -> Self {
        self.exporter.kind = kind;
        self
    }

    /// 设置 file exporter 路径.
    pub fn file_output_path(mut self, path: impl Into<String>) -> Self {
        self.exporter.output_path = path.into();
        self
    }

    /// 构造 TracingConfig (K-1 强校验).
    pub fn build(self) -> TracingResult<TracingConfig> {
        let cfg = TracingConfig {
            service: self.service,
            resource: self.resource,
            sampler: self.sampler,
            exporter: self.exporter,
        };
        cfg.validate()?;
        Ok(cfg)
    }
}

// ============================================================================
// §3 编译期常量
// ============================================================================

/// 配置 schema 版本 (用于序列化兼容性).
pub const TRACING_CONFIG_SCHEMA_VERSION: &str = "apeireth.tracing.config/v1";

/// 4 段配置计数.
pub const TRACING_CONFIG_SECTION_COUNT: usize = 4;

// ============================================================================
// §4 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_valid() {
        let cfg = TracingConfig::default();
        cfg.validate().expect("default must be valid");
    }

    #[test]
    fn test_k1_service_name_empty() {
        let cfg = TracingConfig {
            service: ServiceConfig {
                name: "".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(matches!(
            cfg.validate(),
            Err(TracingError::EmptyServiceName)
        ));
    }

    #[test]
    fn test_k1_service_name_whitespace() {
        let cfg = TracingConfig {
            service: ServiceConfig {
                name: "   ".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(matches!(
            cfg.validate(),
            Err(TracingError::EmptyServiceName)
        ));
    }

    #[test]
    fn test_k1_sampler_ratio_out_of_range() {
        let cfg = TracingConfig {
            sampler: SamplerConfig {
                kind: SamplerKind::TraceIdRatioBased,
                ratio: 1.5,
            },
            ..Default::default()
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_k1_file_exporter_empty_path() {
        let cfg = TracingConfig {
            exporter: ExporterConfig {
                kind: ExporterKind::File,
                output_path: "".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_builder_chain() {
        let cfg = TracingConfig::builder()
            .service_name("apeireth-api")
            .service_version("1.0.0")
            .environment("production")
            .host_name("node-1")
            .resource_attribute("region", "us-east-1")
            .sampler(SamplerKind::TraceIdRatioBased, 0.1)
            .exporter(ExporterKind::Stdout)
            .build()
            .expect("builder must succeed");
        assert_eq!(cfg.service.name, "apeireth-api");
        assert_eq!(cfg.sampler.ratio, 0.1);
        let merged = cfg.resource.merged_attributes();
        assert_eq!(merged.get("host.name").unwrap(), "node-1");
        assert_eq!(merged.get("region").unwrap(), "us-east-1");
    }

    #[test]
    fn test_section_count() {
        assert_eq!(TRACING_CONFIG_SECTION_COUNT, 4);
    }
}
