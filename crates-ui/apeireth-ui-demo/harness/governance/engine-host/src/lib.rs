//! 治理卷宗面对照的 wasm 宿主（**自带 `[workspace]`** ⇒ 不进 crates-ui 依赖图）。
//!
//! ## 职责
//!
//! 把 `apeireth-ui-kit` 的四个治理卷宗 Tab 组件挂进**真实 DOM**，数据来自
//! **固定 fixture**（`fixture_generated.rs`，由抽取器从 JSON 派生）⇒ 引擎侧
//! 与参考侧喂**同一份**数据。
//!
//! ## 诚实边界
//!
//! - 本模块整体被 `#[cfg(target_arch = "wasm32")]` 隔离 ⇒ **native 编译不参与**
//!   ⇒ 它的行为**未经 native 侧验证**（浏览器内的验证由本 harness 承担：
//!   像素对照 + DOM 全等 + 逐帧对照，见 `tools/compare_governance.py`）。
//! - **真后端未接线**：数据面是**固定 fixture**；**变更调用不迁**（签批 /
//!   撤销 / 试算都是宿主侧的事）⇒ 引擎不碰 HTTP / SSE 契约。
//! - 试算表单的初值（`bind:value` 的 property 写回）在引擎侧**未接线**
//!   （`apeireth-ui-macro` 对 `bind:value` 显式报错）⇒ 由宿主在挂载后写入
//!   property（与 Svelte 侧的 `bind:value` 初值等效）。

#[cfg(target_arch = "wasm32")]
mod wasm_app;

#[cfg(target_arch = "wasm32")]
pub use wasm_app::*;

#[cfg(target_arch = "wasm32")]
mod fixture_generated;
