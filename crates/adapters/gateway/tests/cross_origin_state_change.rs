//! **跨源状态变更闸**（2026-10-31 · 静态安全审计复核）。
//!
//! # 背景
//!
//! `tower_http::cors::CorsLayer` 只**加响应头**，**不拒绝**非预检请求
//! ⇒ ⇒ 跨源的 **simple POST**（无预检）**照样抵达 handler**
//! ⇒ ⇒ 浏览器读不到响应，但**副作用照样发生**
//! ⇒ ⇒ 审计条目 `canonical_entry.rs:admin_shutdown` 指的正是这条。
//!
//! ⇒ 修法：加一层**跨源请求闸**（带 `Origin` 且不在本地白名单 ⇒ 403），
//! 且**放在令牌门之内** ⇒ 未过令牌门仍回 401（不借 403/401 差别泄露路由存在性）。
//!
//! # 覆盖
//!
//! * 跨源（`https://evil.example`）⇒ **403**；
//! * 本地源（`http://127.0.0.1:5173` / `tauri://localhost`）⇒ **放行**；
//! * **无 `Origin`**（CLI / 服务端客户端）⇒ **放行**。

use std::sync::Arc;

use apeireth_core::kernel::{Clock, VirtualClock};
use apeireth_gateway::canonical_router;
use apeireth_governance::AllowAll;
use apeireth_runtime::canonical::{InMemorySessionStore, Runtime};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

fn frozen_clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        apeireth_core::kernel::Timestamp::from_epoch_millis(1_700_000_000_000)
            .unwrap()
            .as_datetime(),
    ))
}

async fn runtime() -> Runtime {
    Runtime::builder()
        .with_clock(frozen_clock())
        .with_session_store(Arc::new(InMemorySessionStore::new()))
        .with_governance(Arc::new(AllowAll))
        .with_default_model("test-model")
        .build()
        .await
        .expect("runtime")
}

fn shutdown_request(origin: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/v1/admin/shutdown")
        .header("content-type", "application/json");
    if let Some(origin) = origin {
        builder = builder.header("origin", origin);
    }
    builder.body(Body::empty()).expect("request")
}

#[tokio::test]
async fn foreign_origin_cannot_trigger_state_change() {
    let router = canonical_router(Arc::new(runtime().await));
    let response = router
        .oneshot(shutdown_request(Some("https://evil.example")))
        .await
        .expect("response");
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "跨源 simple POST 必须被拒（CORS 层不拒绝 ⇒ 这一层必须拒）"
    );
}

#[tokio::test]
async fn local_origins_are_allowed_through_the_origin_gate() {
    for origin in [
        "http://127.0.0.1:5173",
        "http://localhost:3000",
        "tauri://localhost",
        "http://tauri.localhost",
    ] {
        let router = canonical_router(Arc::new(runtime().await));
        let response = router
            .oneshot(shutdown_request(Some(origin)))
            .await
            .expect("response");
        assert_ne!(
            response.status(),
            StatusCode::FORBIDDEN,
            "本地源 {origin} 不该被跨源闸拒"
        );
    }
}

#[tokio::test]
async fn requests_without_origin_header_are_allowed() {
    // CLI / 服务端 / 脚本等**非浏览器**客户端本来就没有 Origin 头 ⇒ 必须照常工作。
    let router = canonical_router(Arc::new(runtime().await));
    let response = router
        .oneshot(shutdown_request(None))
        .await
        .expect("response");
    assert_ne!(
        response.status(),
        StatusCode::FORBIDDEN,
        "无 Origin 头 ⇒ 不该被跨源闸拒"
    );
}
