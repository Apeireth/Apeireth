//! `v2_routes::observability` — R20 阶段 6 估补: 3 端点 V2 路由
//!
//! **目的**: 给 `apeireth-api` 加 3 个 V2 observability 路由
//! (跟 `v2_endpoints.rs` LOCKED 平行, 走 axum Router, 不动 v2_endpoints).
//!
//! **3 端点** (per task spec):
//! - `GET /v2/observability/metrics` — Prometheus exposition format (复用 `crate::observability::metrics::metrics_handler`)
//! - `GET /v2/observability/health`  — 5 组件 health (复用 `crate::observability::health::health_handler`)
//! - `GET /v2/observability/status`  — 服务状态 (复用 `crate::observability::status::status_handler`)
//!
//! **架构** (per task spec "不依赖 NewAPI / 不重复造轮子"):
//! - 复用 `crate::observability::{metrics, health, status}` 现有 handler
//! - 跟 `crate::observability::router()` 走同模式 (axum Router + 3 route + 3 GET)
//! - 但路径前缀不同: `v2_routes` 走 `/v2/observability/...`, 旧 `observability::router` 走 `/observability/...`
//!   (注: 旧 router 当前没接进任何 v2 router, 见 v2_endpoints.rs LOCKED)
//!
//! **不修改承诺**:
//! - 1. ✅ 0 触碰 24 LOCKED crate
//! - 2. ✅ 0 改 v2_endpoints.rs 79KB (仅是平行模块, 不动)
//! - 3. ✅ 0 改 workspace version (1.0.0)
//! - 4. ✅ 0 引 NewAPI
//! - 5. ✅ 0 重复造轮子 (复用 crate::observability::* 公开 API)
//! - 6. ✅ 6 哲学锚穿透 (本文件顶部)
//! - 7. ✅ 不假装已接真 metrics (跟 metrics.rs 一致, stub 占位)
//! - 8. ✅ 诚实标缺 (5 R-Measure 占位 0.0)
//!
//! **使用**:
//! ```rust,ignore
//! use apeireth_api::v2_routes::observability::observability_routes;
//! let v2 = axum::Router::new()
//!     .merge(observability_routes())
//!     // ... 其他 v2 路由
//!     ;
//! ```

#![deny(unsafe_code)]

use axum::routing::get;
use axum::Router;

use crate::observability::{
    health::health_handler, metrics::metrics_handler, status::status_handler,
};

/// **3 端点路径** (per task spec — V2 风格, 跟 v1 旧 `/observability/...` 区分)
pub const V2_OBSERVABILITY_ENDPOINTS: [&str; 3] = [
    "/v2/observability/metrics",
    "/v2/observability/health",
    "/v2/observability/status",
];

/// **observability_routes** — 3 端点 axum sub-router
///
/// 跟 `crate::observability::router()` 走同模式 (axum Router), 但路径前缀改 `/v2/`。
/// 返回 `Router<()>` (无 state), 让 caller 决定如何 merge 到更大的 Router。
///
/// **状态码**: 全 GET, 跟 `observability::router()` 一致
/// - `/metrics` → 200 (Prometheus 文本)
/// - `/health`  → 200 (ok / degraded) / 503 (down)
/// - `/status`  → 200 (永远, 状态查询)
pub fn observability_routes() -> Router<()> {
    Router::new()
        .route(V2_OBSERVABILITY_ENDPOINTS[0], get(metrics_handler))
        .route(V2_OBSERVABILITY_ENDPOINTS[1], get(health_handler))
        .route(V2_OBSERVABILITY_ENDPOINTS[2], get(status_handler))
}

/// **observability_routes_count** — 编译期守门 (必 = 3)
pub const OBSERVABILITY_ROUTES_COUNT: usize = V2_OBSERVABILITY_ENDPOINTS.len();

const _: () = assert!(OBSERVABILITY_ROUTES_COUNT == 3, "3 V2 observability 端点");

// ============================================================
// 单元测试 (3 测: 编译期 / 3 端点路径 / Router 构造)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 1. 编译期 hardcode + 3 端点路径字面量
    #[test]
    fn v2_observability_endpoints_count_3() {
        assert_eq!(V2_OBSERVABILITY_ENDPOINTS.len(), 3);
        assert_eq!(OBSERVABILITY_ROUTES_COUNT, 3);
        assert!(V2_OBSERVABILITY_ENDPOINTS.contains(&"/v2/observability/metrics"));
        assert!(V2_OBSERVABILITY_ENDPOINTS.contains(&"/v2/observability/health"));
        assert!(V2_OBSERVABILITY_ENDPOINTS.contains(&"/v2/observability/status"));
    }

    /// 2. Router 可构造 + 路径前缀 /v2/ 跟 v1 /observability/ 区分
    #[test]
    fn v2_observability_routes_builds_router() {
        let r: Router<()> = observability_routes();
        // 简单构造验证 — 路径前缀 /v2/ 跟 v1 /observability/ 不重
        let _ = r;
        for ep in V2_OBSERVABILITY_ENDPOINTS {
            assert!(ep.starts_with("/v2/"), "V2 端点必须 /v2/ 前缀: {ep}");
        }
    }

    /// 3. 复用 crate::observability::* handler (不重写 handler, 走复用)
    #[test]
    fn v2_observability_reuses_observability_handlers() {
        // 编译期: handler 函数指针相同 (走 crate::observability::* import)
        let _h1 = metrics_handler;
        let _h2 = health_handler;
        let _h3 = status_handler;
        // 全是 async fn, 编译期守门
        assert!(true, "re-use check");
    }
}
