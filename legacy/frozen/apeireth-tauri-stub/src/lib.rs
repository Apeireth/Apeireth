//! `apeireth-tauri-stub` — ⚠️ DEPRECATED Tauri 2 参考实现 (V2 Day 1 Step 1.3)
//!
//! 原名 `apeireth-desktop`,R17 战役 2-3 创建,作为最小 stub 让 workspace 可 build.
//! R17 砍 Tauri 前端战役已过,R19 战役计划用真前端,本 crate **不在产品里**,
//! 仅作为 Tauri 2 集成参考样例保留 (`src/main.rs` 26KB 真 Tauri 代码,可复用).
//!
//! **当前状态**: 仅保留 `R19_DESKTOP_STUB = true` 常量 + `src/main.rs` 26KB Tauri 代码.
//! **不动承诺**: 本 stub 不参与 LOCKED 检查,不进 CI artifact.
//! **R19 worker 接管路径**: 见 `README.md`.
//!
//! ⚠️ Tech-Review 2026-08-05: 本 stub 是 reqwest 0.13 + hyper 0.14 双版本共存的
//! 唯一引入者（通过 `tauri = "2"` 传递依赖）。R19 worker 接管时建议：
//!   (a) 把本 crate 从 `[workspace] members` 移除（`Cargo.toml:35`），
//!   (b) 改为 `crates/apeireth-tauri-stub/Cargo.toml` 独立 `[workspace]`，
//!   (c) 或保留为 opt-in `cargo build -p apeireth-tauri-stub` 显式构建。
//! 见 `docs/tech-review-2026-08-05.md` §P0-1。

#![deny(unsafe_code)]

/// R19 战役待实现. 此常量供 R19 worker 检测 stub 是否被覆盖.
pub const R19_DESKTOP_STUB: bool = true;

/// V2 Day 1: crate 已重命名 + 标 DEPRECATED. R19 接管前请勿移除.
pub const V2_DAY1_DEPRECATED: bool = true;

/// B2: tool_loop adapter (R32-2 + R35 follow-up, 0 漂移 TUI)
pub mod tool_loop_adapter;
