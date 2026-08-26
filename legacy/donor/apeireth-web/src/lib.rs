//! `apeireth-web` 库入口 (R18)
//!
//! 提供:
//! - `api` — Council 7 advisor 数据结构 (跨 SSR/客户端 共享)
//! - `app` / `council` / `verdict` — 组件 stub (R19+ 升级到 Leptos view! 时填)
//! - `memory` — Memory UI (Episode 时间线 + IdentityCard), 端到端接通 apeireth-memory
//! - `council_history` — Council 历史 (R18 sub-agent #2, TBD)
//! - `sovereignty` — Self-Disable 5 大机制控制台 (R18 sub-agent #3, 端到端接通 apeireth-sovereignty)
//! - `asi` — ASI 24 维测量可视化 (R18 sub-agent #4, 端到端接通 apeireth-asi, 雷达图 + ML 校准状态)
//! - `api_endpoints` — 综合 Dashboard (R18 sub-agent #5, 6 器官状态汇总, SSR only)
//! - `templates` — 共享 HTML 模板 helpers (html_escape, render_error_page)

pub mod api;
// R177: organ invariants (5 tests + 2 Kani)
#[cfg(feature = "ssr")]
pub mod api_endpoints;
pub mod app;
#[cfg(feature = "ssr")]
pub mod asi;
pub mod council;
pub mod council_history;
pub mod memory;
mod organ_kani_proofs;
pub mod sovereignty;
pub mod templates;
pub mod tool_loop_adapter;
pub mod verdict;
