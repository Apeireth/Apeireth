//! ⭐ **宿主侧「时刻文本」能力**（浏览器 `Intl` 口径 ⇒ 与生产**逐字同源**）。
//!
//! ## 契约
//!
//! 引擎（`apeireth-ui-kit`）只声明「时刻文本由宿主给」；
//! 本 crate 提供**浏览器实现** ⇒ 宿主在启动时 `install()` 一次即可。
//!
//! ## 为什么逐字照抄生产
//!
//! 生产（`frontend/companion-desktop/src/lib/governance/format.ts`）：
//!
//! ```text
//! formatClockMs(ts)     ⇒ d.toLocaleTimeString('zh-CN', {hour:'2-digit', minute:'2-digit', second:'2-digit'})
//! formatDateTimeMs(ts)  ⇒ 同日 ? toLocaleTimeString('zh-CN', {hour, minute})
//!                          : `${toLocaleDateString('zh-CN', {month:'2-digit', day:'2-digit'})} ${hm}`
//! ```
//!
//! ⇒ 本实现**逐字照抄**（含「同日」判定用 `toDateString()`，它按**本地时区**）
//! ⇒ ⇒ 两侧的输出来自**同一个** `Intl` ⇒ ⇒ 连 ICU 版本差异都一致。

#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

#[cfg(target_arch = "wasm32")]
mod browser;

#[cfg(target_arch = "wasm32")]
pub use browser::BrowserTimeText;

/// 装上**浏览器时刻文本**（宿主启动时调用一次）。
///
/// - wasm ⇒ 装**浏览器 `Intl` 实现** ⇒ 与生产**逐字一致**；
/// - native ⇒ **不装** ⇒ 引擎保持默认的**固定 UTC** 实现（可测、可复现）。
pub fn install() {
    #[cfg(target_arch = "wasm32")]
    {
        apeireth_ui_kit::governance_tab::set_time_text_formatter(Some(std::rc::Rc::new(
            BrowserTimeText,
        )));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        // native ⇒ 保持默认（固定 UTC）⇒ 可测、可复现。
    }
}
