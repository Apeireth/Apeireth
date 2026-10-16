// pet-render-pause.ts — 渲染循环的第二道停帧闩（真隐藏口径）
//
// 背景（2026-10-11 真窗实测发现）：Tauri `window.hide()` **不翻**
// `document.visibilityState`（WebView2 实测 hidden:false）——`document.hidden`
// 路径只覆盖最小化/遮挡，真隐藏停帧靠浏览器原生节流（黑箱，不可验收）。
// 补法：壳层在隐藏桌宠前发 `pet-host-visibility` 事件，窗根把本闩置位，
// 所有形象层渲染循环在 `document.hidden || petRenderPause.paused` 时停帧。
// 双保险：原生节流 + 显式闩，后者是可验收的那道。
export const petRenderPause = {paused: false};
