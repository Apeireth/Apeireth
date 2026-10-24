//! 设置页三页 · `provider` 子面的组件模块（**并行迁移面 · 预置空桩**）。
//!
//! ⚠️ 预置空桩 ⇒ 目的是让 `ui-kit/src/lib.rs` 的 `pub mod settings_provider;` 提前存在
//! （从而**多个子面并行施工而互不碰 `lib.rs`**）⇒ 本文件由**该子面的施工线独占写入**。
//!
//! 组件风格照 `tool_card.rs` / `status_bar.rs`：`Props` + `render` + `view!` 构造。
