//! `apeireth-web` 库入口 (v2 reconstruction)
//!
//! 提供:
//! - `api` — Council 7 advisor 数据结构 (跨 SSR/客户端 共享)
//! - `app` / `council` / `verdict` — 组件 stub (R19+ 升级到 Leptos view! 时填)
//! - `memory` — Memory UI (Episode 时间线 + IdentityCard), 端到端接通 apeireth-storage::memory_episode
//! - `council_history` — Council 历史 (R18 sub-agent #2, TBD)
//! - `sovereignty` — Self-Disable 5 大机制控制台 (R18 sub-agent #3, 端到端接通 apeireth-sovereignty)
//! - `asi` — ASI 24 维测量可视化 (R18 sub-agent #4, 端到端接通 apeireth-asi, 雷达图 + ML 校准状态)
//! - `api_endpoints` — 综合 Dashboard (R18 sub-agent #5, 6 器官状态汇总, SSR only)
//! - `templates` — 共享 HTML 模板 helpers (html_escape, render_error_page)
//!
//! **v2 适配**:
//! - v1 依赖 apeireth-memory (独立 crate); v2 把 memory 子系统整合到 apeireth-storage::memory_episode
//!   (Episode 字段 id/timestamp/role/content/session_id 跟 v1 字段对齐, 直接复用).
//! - v1 依赖 apeireth-core::Episode / IdentityCard; v2 core 重构后字段不一样, web 这层直接用
//!   storage::memory_episode::Episode, IdentityCard 部分保留 v1 兼容字段 (continuity_id/birth_time/carriers/migration_history)
//!   在 memory.rs 内本地定义 in-memory 存储, 不依赖 core.
//! - v1 sovereignty SelfDisableSignal/Guard API 跟 v2 不完全一样; web 这层用 v2 check_no_* 系列重写 attack.

pub mod api;
#[cfg(feature = "ssr")]
pub mod api_endpoints;
pub mod app;
#[cfg(feature = "ssr")]
pub mod asi;
pub mod council;
pub mod council_history;
pub mod memory;
pub mod sovereignty;
pub mod templates;
pub mod verdict;
