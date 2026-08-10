//! `v2_routes` — R20 阶段 6 (1.0 release) V2 路由模块入口
//!
//! **目的**: 给 `apeireth-api` 加 V2 路由分发目录, 跟 `v2_endpoints.rs` (LOCKED)
//! 并行, 不动 `v2_endpoints.rs` 79KB 大块.
//!
//! **子模块**:
//! - `observability` — 3 端点路由 (`/v2/observability/{metrics,health,status}`),
//!   走 `axum::Router`, 跟 `v2_endpoints::build_router` 同模式但分文件
//!
//! **架构**:
//! - 自包含, 0 循环依赖 (跟 v2_endpoints 一致)
//! - 公开 API: `pub fn router() -> axum::Router<()>` — 让 caller 决定如何 merge
//! - 不动 `v2_endpoints.rs` (24 LOCKED 守门)
//!
//! **8 项不修改承诺**:
//! - 1. ✅ 0 触碰 24 LOCKED crate
//! - 2. ✅ 0 改 v2_endpoints.rs 79KB
//! - 3. ✅ 0 改 workspace version (1.0.0)
//! - 4. ✅ 0 引 NewAPI
//! - 5. ✅ 0 重复造轮子 (复用 `crate::observability::*`)
//! - 6. ✅ 6 哲学锚穿透 (子模块顶部)
//! - 7. ✅ 不假装已接 observability 真依赖 (跟 status.rs 一致, stub 占位)
//! - 8. ✅ 诚实标缺 (5 R-Measure 占位 0.0)

#![deny(unsafe_code)]

pub mod observability;
