//! 设置页三页 · **子面宿主分发**（共享模块；各子面的挂载实现在**各自的模块**里）。
//!
//! ## 为什么是「一个共享分发 + 各自的实现」
//!
//! `window.__APP_MODE__` 的取值形如 `settings:<面 id>`（`wizard` / `theme` / `frame` / …）。
//! 分发**必须**在共享的 `main.rs` 里（wasm 只有一个入口）⇒ 若各子面自己往 `main.rs`
//! 里加分支 ⇒ **并行时必然撞车**（本次已实测到共享文件被覆盖）。
//!
//! ⇒ 处置：**分发集中在此文件**（共享、只由主线改），**实现分散在各子面模块**
//! （各子面施工线独占）⇒ 共享点归零。
//!
//! ## 诚实边界
//!
//! - 本模块整体被 `#[cfg(target_arch = "wasm32")]` 隔离 ⇒ **native 编译不参与**
//!   ⇒ 其行为**未经 native 侧验证**。
//! - **未接线的面 id ⇒ 显式返回 `None`**（调用方据此**不挂载任何东西**）
//!   ⇒ 页面显示为空白 ⇒ **不假装已接线**。

/// 挂载某个子面。返回 `None` ⇒ **该面 id 未接线**（调用方不挂载）。
#[cfg(target_arch = "wasm32")]
pub fn mount(
    id: &str,
    container: &web_sys::Element,
) -> Option<Result<(), String>> {
    match id {
        "wizard" => Some(crate::settings_wizard::wasm::mount(container)),
        "theme" => Some(crate::settings_theme::wasm::mount(container)),
        "frame" => Some(crate::settings_frame::wasm::mount(container)),
        "budget" => Some(crate::settings_budget::wasm::mount(container)),
        "data" => Some(crate::settings_data::wasm::mount(container)),
        "character" => Some(crate::settings_character::wasm::mount(container)),
        "capability" => Some(crate::settings_capability::wasm::mount(container)),
        "provider" => Some(crate::settings_provider::wasm::mount(container)),
        _ => None,
    }
}
