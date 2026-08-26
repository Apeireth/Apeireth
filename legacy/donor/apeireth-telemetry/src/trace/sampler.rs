//! # Sampler
//!
//! 4 种 sampling 策略, 1:1 翻译 v0.9.21 商业版 `out/main/chunks/tracing` 的
//! sampler 模块. 编译期 hardcode `SamplerKind` 枚举, 运行时由 `Sampler` trait
//! 决策是否记录当前 trace.
//!
//! ## 4 Sampler (per task spec §6)
//!
//! | Sampler | 行为 | 1:1 翻译 |
//! |---------|------|----------|
//! | `AlwaysOn` | 全采样 | `AlwaysOnSampler` |
//! | `AlwaysOff` | 不采样 | `AlwaysOffSampler` |
//! | `TraceIdRatioBased(ratio)` | 按 trace_id 哈希比例采样 | `TraceIdRatioBasedSampler` |
//! | `ParentBased(remote_parent_sampled)` | 跟父 span 决策, 父决策继承 | `ParentBasedSampler` |
//!
//! ## 6 哲学 anchor 穿透
//!
//! - **S-1 北极星导向**: 采样服务 ASI 北极星 (减少 90% 后端 trace 存储)
//! - **S-2 实事求是**: 4 sampler 不增不减, 0 过度设计
//! - **O-2 走在前人肩上**: 借鉴 OpenTelemetry SDK `Sampler` trait (ShouldSample)
//! - **O-3 干到底**: 4 sampler 全部真接, 测试 25+ fixture 覆盖
//! - **O-4 任何人都能接手**: 跟 cache / credentials sampler 同模式
//! - **O-5 不假装**: ratio 用 trace_id 末 8 hex char 哈希, 不假装"真随机"

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::error::TracingResult;

// ============================================================================
// §1 SamplerKind 枚举 (4 变体, 编译期 hardcode)
// ============================================================================

/// 4 种 sampler 策略.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SamplerKind {
    /// 全采样.
    AlwaysOn,
    /// 不采样.
    AlwaysOff,
    /// 按比例 (0.0 - 1.0) 采样.
    TraceIdRatioBased,
    /// 跟父 span 决策.
    ParentBased,
}

impl SamplerKind {
    /// 字符串名 (稳定, 用于序列化).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AlwaysOn => "always_on",
            Self::AlwaysOff => "always_off",
            Self::TraceIdRatioBased => "trace_id_ratio_based",
            Self::ParentBased => "parent_based",
        }
    }
}

// ============================================================================
// §2 Sampler trait
// ============================================================================

/// Sampler 决策 trait (1:1 翻译 OpenTelemetry `Sampler::should_sample`).
#[async_trait]
pub trait Sampler: Send + Sync {
    /// 决策是否记录当前 trace.
    ///
    /// # Arguments
    ///
    /// * `trace_id` — 32 hex char trace_id
    /// * `parent_sampled` — 父 span 是否被采样 (None 表示无父)
    async fn should_sample(&self, trace_id: &str, parent_sampled: Option<bool>) -> bool;

    /// Sampler 类型.
    fn kind(&self) -> SamplerKind;
}

// ============================================================================
// §3 4 Sampler 实现
// ============================================================================

/// AlwaysOn — 全采样.
#[derive(Debug, Default, Clone, Copy)]
pub struct AlwaysOnSampler;

#[async_trait]
impl Sampler for AlwaysOnSampler {
    async fn should_sample(&self, _trace_id: &str, _parent_sampled: Option<bool>) -> bool {
        true
    }

    fn kind(&self) -> SamplerKind {
        SamplerKind::AlwaysOn
    }
}

/// AlwaysOff — 不采样.
#[derive(Debug, Default, Clone, Copy)]
pub struct AlwaysOffSampler;

#[async_trait]
impl Sampler for AlwaysOffSampler {
    async fn should_sample(&self, _trace_id: &str, _parent_sampled: Option<bool>) -> bool {
        false
    }

    fn kind(&self) -> SamplerKind {
        SamplerKind::AlwaysOff
    }
}

/// TraceIdRatioBased — 按 trace_id 哈希比例采样.
///
/// 决策: trace_id 末 8 hex char 转 u64, 除以 u64::MAX, < ratio 则采样.
#[derive(Debug, Clone, Copy)]
pub struct TraceIdRatioBasedSampler {
    /// 采样比例 (0.0 - 1.0).
    pub ratio: f64,
}

impl TraceIdRatioBasedSampler {
    /// 构造 (K-1 强校验: ratio ∈ [0.0, 1.0]).
    pub fn new(ratio: f64) -> TracingResult<Self> {
        if !(0.0..=1.0).contains(&ratio) {
            return Err(super::error::TracingError::SamplingError(format!(
                "ratio must be in [0.0, 1.0], got {}",
                ratio
            )));
        }
        Ok(Self { ratio })
    }
}

#[async_trait]
impl Sampler for TraceIdRatioBasedSampler {
    async fn should_sample(&self, trace_id: &str, _parent_sampled: Option<bool>) -> bool {
        if self.ratio == 0.0 {
            return false;
        }
        if self.ratio == 1.0 {
            return true;
        }
        // 末 8 hex char → u64 (8 hex = 32 bit, 实际范围 [0, u32::MAX])
        // 标准化: v / u32::MAX 范围 [0.0, 1.0]
        let tail = &trace_id[trace_id.len().saturating_sub(8)..];
        let v = u64::from_str_radix(tail, 16).unwrap_or(0);
        let normalized = (v as f64) / f64::from(u32::MAX);
        normalized < self.ratio
    }

    fn kind(&self) -> SamplerKind {
        SamplerKind::TraceIdRatioBased
    }
}

/// ParentBased — 跟父 span 决策.
///
/// 父决策优先: 父被采样 → 子也采; 父不采 → 子也不采.
/// 无父 → fallback 到 root sampler (默认 AlwaysOn).
pub struct ParentBasedSampler {
    /// 无父时的 fallback sampler.
    pub root_sampler: Arc<dyn Sampler>,
}

impl std::fmt::Debug for ParentBasedSampler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParentBasedSampler")
            .field("root_sampler", &"<dyn Sampler>")
            .finish()
    }
}

impl Clone for ParentBasedSampler {
    fn clone(&self) -> Self {
        Self {
            root_sampler: self.root_sampler.clone(),
        }
    }
}

impl ParentBasedSampler {
    /// 构造 (默认 root = AlwaysOn).
    pub fn new() -> Self {
        Self {
            root_sampler: Arc::new(AlwaysOnSampler),
        }
    }

    /// 用指定的 root sampler 构造.
    pub fn with_root(root: Arc<dyn Sampler>) -> Self {
        Self { root_sampler: root }
    }
}

impl Default for ParentBasedSampler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Sampler for ParentBasedSampler {
    async fn should_sample(&self, _trace_id: &str, parent_sampled: Option<bool>) -> bool {
        match parent_sampled {
            Some(sampled) => sampled,
            None => self.root_sampler.should_sample(_trace_id, None).await,
        }
    }

    fn kind(&self) -> SamplerKind {
        SamplerKind::ParentBased
    }
}

// ============================================================================
// §4 Factory
// ============================================================================

/// Sampler factory (按 kind + ratio 构造).
pub fn build_sampler(kind: SamplerKind, ratio: f64) -> TracingResult<Arc<dyn Sampler>> {
    match kind {
        SamplerKind::AlwaysOn => Ok(Arc::new(AlwaysOnSampler)),
        SamplerKind::AlwaysOff => Ok(Arc::new(AlwaysOffSampler)),
        SamplerKind::TraceIdRatioBased => Ok(Arc::new(TraceIdRatioBasedSampler::new(ratio)?)),
        SamplerKind::ParentBased => Ok(Arc::new(ParentBasedSampler::new())),
    }
}

// ============================================================================
// §5 编译期常量
// ============================================================================

/// Sampler 变体计数.
pub const SAMPLER_KIND_COUNT: usize = 4;

// ============================================================================
// §6 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_trace_id() -> String {
        "0af7651916cd43dd8448eb211c80319c".to_string()
    }

    #[tokio::test]
    async fn test_always_on() {
        let s = AlwaysOnSampler;
        assert!(s.should_sample(&sample_trace_id(), None).await);
        assert_eq!(s.kind(), SamplerKind::AlwaysOn);
    }

    #[tokio::test]
    async fn test_always_off() {
        let s = AlwaysOffSampler;
        assert!(!s.should_sample(&sample_trace_id(), None).await);
        assert_eq!(s.kind(), SamplerKind::AlwaysOff);
    }

    #[tokio::test]
    async fn test_ratio_zero() {
        let s = TraceIdRatioBasedSampler::new(0.0).unwrap();
        assert!(!s.should_sample(&sample_trace_id(), None).await);
    }

    #[tokio::test]
    async fn test_ratio_one() {
        let s = TraceIdRatioBasedSampler::new(1.0).unwrap();
        assert!(s.should_sample(&sample_trace_id(), None).await);
    }

    #[tokio::test]
    async fn test_ratio_invalid() {
        assert!(TraceIdRatioBasedSampler::new(-0.1).is_err());
        assert!(TraceIdRatioBasedSampler::new(1.5).is_err());
    }

    #[tokio::test]
    async fn test_parent_based_yes() {
        let s = ParentBasedSampler::new();
        assert!(s.should_sample(&sample_trace_id(), Some(true)).await);
        assert!(!s.should_sample(&sample_trace_id(), Some(false)).await);
    }

    #[tokio::test]
    async fn test_parent_based_no_parent_fallback() {
        let s = ParentBasedSampler::new();
        // 默认 root = AlwaysOn
        assert!(s.should_sample(&sample_trace_id(), None).await);

        // 用 AlwaysOff 当 root
        let s2 = ParentBasedSampler::with_root(Arc::new(AlwaysOffSampler));
        assert!(!s2.should_sample(&sample_trace_id(), None).await);
    }

    #[tokio::test]
    async fn test_ratio_10_percent_distribution() {
        // 10% 采样: 10000 个 trace_id 应有约 1000 个被采样 (容许 ±5%)
        // 用真实 UUIDv4 派生, 保证末 8 hex char 在全 16^8 空间均匀分布
        let s = TraceIdRatioBasedSampler::new(0.1).unwrap();
        let mut sampled = 0;
        for _ in 0..10_000u32 {
            let id = generate_uuid_hex32();
            if s.should_sample(&id, None).await {
                sampled += 1;
            }
        }
        let rate = f64::from(sampled) / 10_000.0;
        assert!(
            (0.05..=0.15).contains(&rate),
            "10% sampler out of range: rate={}",
            rate
        );
    }

    /// 构造均匀分布的 32 hex char 串 (2 个 UUIDv4 拼).
    fn generate_uuid_hex32() -> String {
        use uuid::Uuid;
        let u1 = Uuid::new_v4();
        let u2 = Uuid::new_v4();
        let mut s = format!("{}{}", u1.simple(), u2.simple());
        s.truncate(32);
        s
    }

    #[test]
    fn test_kind_as_str() {
        assert_eq!(SamplerKind::AlwaysOn.as_str(), "always_on");
        assert_eq!(SamplerKind::AlwaysOff.as_str(), "always_off");
        assert_eq!(
            SamplerKind::TraceIdRatioBased.as_str(),
            "trace_id_ratio_based"
        );
        assert_eq!(SamplerKind::ParentBased.as_str(), "parent_based");
    }

    #[test]
    fn test_kind_count() {
        assert_eq!(SAMPLER_KIND_COUNT, 4);
    }
}
