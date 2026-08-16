//! `apeireth-telemetry` - 1.1 升级: 4 老 crate (cache/metrics/tracing/observability) 真合并
//!
//! **R35 facade + 1.1 真合并**: 4 老 crate 源码 1:1 搬到本 crate 4 module
//! (cache / metric / trace / observability). 0 改 4 老 crate 公开 API.
//!
//! **兼容层**: 旧 import path `apeireth_cache::X` 仍可工作 (通过 `_compat_*` re-export).
//! 推荐新代码用 `apeireth_telemetry::cache::X` 直接路径.
//!
//! **不重复声明**: 4 个 module 全是从原 crate 1:1 搬过来, 0 增加代码体积.
//!
//! **6 哲学锚穿透**:
//! - S-1 走在前人肩上: Prometheus client / OpenTelemetry / W3C trace spec 字段级 1:1
//! - S-2 实事求是: 真搬源码 0 改, 0 重写, 0 facade 假象
//! - O-3 干到底: 一波 1.1 release 4 合并全收
//! - O-4 任何人都能接手: 1:1 文件结构, 看 telemetry/src/cache 跟看 apeireth-cache/src 一样
//! - O-5 不假装: 4 module 完整可读, 0 隐藏
//! - S-1 不漂移: workspace 1.0 -> 1.1, 8 项承诺 / 24 LOCKED crate **主人 1.1 授权可重构**

#![warn(missing_docs)]

/// 1.1 cache module - LRU/LFU/FIFO/ARC/TinyLFU + 4 backend (1:1 from apeireth-cache)
pub mod cache;
// R177: organ invariants (5 tests + 2 Kani)
mod organ_kani_proofs;

// 1.1 兼容层 (R128 续): 过洋 doctests 参考老 crate 名.
// R35+1.1 合并 apeireth-{cache,metrics,tracing,observability} 到 telemetry/src.
// 老 crate 已 frozen, 但 doctest 仍引 `crate::trace::quick_trace`.
// 在 lib.rs 加 4 个 compat module 别名, 让旧 import 路径通过 pub use 归丛.
// 新码推荐 `apeireth_telemetry::trace::*`. per S-2/O-4.

/// 1.1 兼容: `apeireth_cache::*` 归丛 `cache::*`.
#[doc(hidden)]
pub mod apeireth_cache {
    pub use crate::cache::*;
}

/// 1.1 兼容: `apeireth_metrics::*` 归丛 `metric::*`.
#[doc(hidden)]
pub mod apeireth_metrics {
    pub use crate::metric::*;
}

/// 1.1 metric module - Counter/Gauge/Histogram/Summary + Prometheus (1:1 from apeireth-metrics)
pub mod metric;

/// 1.1 兼容: `apeireth_tracing::*` 归丛 `trace::*`.
#[doc(hidden)]
pub mod apeireth_tracing {
    pub use crate::trace::*;
}

/// 1.1 trace module - W3C trace_id + 4 SpanKind + 4 exporter (1:1 from apeireth-tracing)
pub mod trace;

/// 1.1 兼容: `apeireth_observability::*` 归丛 `observability::*`.
#[doc(hidden)]
pub mod apeireth_observability {
    pub use crate::observability::*;
}

/// 1.1 observability module - 5 component health + 3 endpoint + tui_dashboard (1:1 from apeireth-observability)
pub mod observability;

/// R122-7: VCP vcpLogReplayManager.js 19KB 字段级借鉴 (R122-7-VCP-LogReplay-2026-08-10)
/// 1.1 升级新增 module: 12 unit test + ReplaySpeed 3 模式 + LogStats 聚合
pub mod log_replay;

// 1.1 兼容层: 旧 import path 透明 re-export
/// 1.1: 6 module 名 1:1 对应 (R122-7 加 log_replay; OTel 审计加 otlp)
pub const ALL_MODS: [&str; 6] = [
    "cache",
    "metric",
    "trace",
    "observability",
    "log_replay",
    "otlp",
];

/// 可选 OTLP 导出接口 (trait + Noop 默认 + JSON 行实现, 0 opentelemetry 重依赖).
/// 详见 `crates/apeireth-telemetry/src/otlp.rs` 头部审计结论.
pub mod otlp;

#[cfg(test)]
mod r35_umbrella_tests {
    use super::*;
    #[test]
    fn r35_4_modules_all_present() {
        use crate::cache::policy::EvictionPolicy;
        let _ = EvictionPolicy::Lru;
        use crate::metric::config::MetricsConfig;
        let _cfg = std::mem::size_of::<MetricsConfig>();
        use crate::trace::span::SpanKind;
        let _ = SpanKind::Client;
        use crate::observability::TuiDashboardError;
        let _ = std::mem::size_of::<TuiDashboardError>();
    }
    #[test]
    fn r35_facade_reexports_compile() {
        assert_eq!(ALL_MODS.len(), 6); // R122-7 加 log_replay; OTel 审计加 otlp
        let _ = std::any::type_name::<crate::cache::policy::EvictionPolicy>();
        let _ = std::any::type_name::<crate::metric::config::MetricsConfig>();
        let _ = std::any::type_name::<crate::trace::span::SpanKind>();
        let _ = std::any::type_name::<crate::observability::TuiDashboardError>();
        let _ = std::any::type_name::<crate::log_replay::LogReplay>();
        let _ = std::any::type_name::<crate::otlp::NoopOtlpSink>();
    }
}
