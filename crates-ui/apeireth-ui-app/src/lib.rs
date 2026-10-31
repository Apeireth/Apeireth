//! ⭐ **引擎产品工程**（宿主运行时契约）—— 交接包 §0「由引擎线指定路径与工程基线」的落点。
//!
//! ## 这个 crate 是什么
//!
//! 它是**新前端的实现目标工程**：把「屏幕」装配成引擎能渲染的树，并管住
//! 「挂载 → 重渲染 → 销毁」这条生命周期。
//!
//! ## 它不是什么（边界，硬）
//!
//! - **不是试点**：[`apeireth-ui-demo`](../apeireth-ui-demo/index.html) 是试点载体兼
//!   装配级冒烟（ADR-0015 §4），且被证据装置与 CI 引用 ⇒ ⇒ 职责不同，不合并。
//! - **不是引擎**：`core` / `dom` / `gpu` / `kit` 是引擎层（ADR-0015 增补的三层边界），
//!   本 crate 只做**装配与宿主** ⇒ ⇒ 组件与能力一律不塞进来。
//!
//! ## 为什么契约放在这里、wasm 薄壳放在 `main.rs`
//!
//! 契约（[`Screen`] / [`HostState`]）是**纯逻辑** ⇒ ⇒ **native 可单测**
//! （与 `apeireth-ui-core` 的「永远能 native 单测」纪律一致）；
//! wasm 入口（`main.rs`）只做「把契约接到真实 DOM」这一件事 ⇒ ⇒ 薄。

use apeireth_ui_core::Signal;
use apeireth_ui_dom::vnode::VNode;

/// ⭐ **wasm 薄壳**（接真实 DOM 的入口）—— **仅 wasm32 目标编译**。
///
/// ⇒ native 侧**不参与编译** ⇒ ⇒ 契约层永远可 native 单测（与 core 同纪律）。
#[cfg(target_arch = "wasm32")]
pub mod wasm_host;

/// ⭐ **屏幕契约**：一个屏幕 = 一个身份 + 一棵由引擎渲染的视图树。
///
/// ## 为什么是 trait 而不是闭包
///
/// 屏幕要能被**路由**（按 id 找屏幕）、要能被**测试**（不碰 DOM 就能拿到视图树）
/// ⇒ ⇒ trait 能同时满足两者；闭包只能满足其一。
pub trait Screen: 'static {
    /// 屏幕身份（路由 / 日志 / 恢复现场用；**稳定** ⇒ 不要塞会变的文案）。
    fn id(&self) -> &str;

    /// ⭐ 供宿主做**类型识别**（默认实现即可）。
    ///
    /// ## 为什么需要
    ///
    /// 宿主要能对具体屏幕做**专用操作**（例如演示屏的「改文本」接口）
    /// ⇒ ⇒ 而 trait 对象默认不能 `downcast` ⇒ ⇒ 需要一条**类型擦除的窄口**。
    fn as_any(&self) -> &dyn std::any::Any;

    /// 当前状态的视图树。
    ///
    /// ⚠️ **每次调用都应返回当前状态的树**（状态在信号里 ⇒ 读信号即可）
    /// ⇒ ⇒ 宿主在需要重渲染时调它 ⇒ ⇒ 树是「状态的投影」，不持有状态。
    fn view(&self) -> VNode;
}

/// ⭐ **宿主状态**：当前在哪个屏幕、要不要重渲染。
///
/// ## 为什么宿主自己持有「脏」标记
///
/// 屏幕的变更来源很多（信号、事件、路由、数据到达）⇒ ⇒ 若每个来源都自己触发重渲染
/// ⇒ ⇒ 同一帧内可能重渲染多次 ⇒ ⇒ 抖动与浪费。
/// ⇒ ⇒ 宿主收口：**来源只标脏**，由宿主在**帧边界**统一决定是否重渲染一次。
#[derive(Debug, Default, Clone)]
pub struct HostState {
    current: Option<String>,
    dirty: bool,
}

impl HostState {
    /// 建空宿主状态（无当前屏幕）。
    pub fn new() -> Self {
        Self::default()
    }

    /// 当前屏幕 id（`None` = 还没上屏）。
    pub fn current(&self) -> Option<&str> {
        self.current.as_deref()
    }

    /// 是否标脏（需要重渲染）。
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// ⭐ 切屏：换当前屏幕并**标脏**（新屏幕必须重渲染一次）。
    ///
    /// ⚠️ 切屏时**旧屏幕必须先被销毁**（`dispose`）⇒ ⇒ 否则旧屏的焦点陷阱、
    /// IME 待写队列会渗进新屏（见 `Renderer::dispose`）。
    pub fn switch_to(&mut self, screen_id: impl Into<String>) {
        self.current = Some(screen_id.into());
        self.dirty = true;
    }

    /// 标脏（任何变更来源都调它：信号、事件、数据…）。
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// ⭐ 取走「脏」标记（宿主在帧边界调它 ⇒ ⇒ 一次变更只触发一次重渲染）。
    ///
    /// 返回 `true` = 需要重渲染；`false` = 本帧无需重渲染。
    pub fn take_dirty(&mut self) -> bool {
        std::mem::replace(&mut self.dirty, false)
    }

    /// 清空（屏幕被销毁时）。
    pub fn clear(&mut self) {
        self.current = None;
        self.dirty = false;
    }
}

/// ⭐ **把一个信号接到宿主**（屏幕状态 ⇒ 标脏 ⇒ 重渲染）。
///
/// ## 为什么在这里做（而不是让每个屏幕自己接）
///
/// 漏接 ⇒ 改了状态**界面不动**（最难查的一类 bug）⇒ ⇒ 收口在宿主里做一次，
/// 所有屏幕**自动**获得「状态变 ⇒ 重渲染」。
///
/// ⚠️ 返回的 `EffectHandle` **必须被保留**（或显式 drop）⇒ ⇒ 否则效果会被立刻撤销
/// （见 `apeireth_ui_core::effect`）。
pub fn wire_signal_to_host(
    runtime: &apeireth_ui_core::Runtime,
    host: &Signal<HostState>,
    source: &Signal<String>,
) -> apeireth_ui_core::EffectHandle {
    // ⭐ core 的 effect 是**依赖追踪式**的：闭包里**读了哪个 signal** ⇒ 就订阅哪个
    // ⇒ ⇒ 这里读 `source` ⇒ 订阅它 ⇒ 它变了 ⇒ 本 effect 重跑 ⇒ 宿主标脏。
    let source = source.clone();
    let host = host.clone();
    runtime.effect(move || {
        let _ = source.get(); // ← 这一行**就是**订阅（依赖收集点）
        host.update(|mut state| {
            state.mark_dirty();
            state
        });
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 切屏 ⇒ 标脏 ⇒ 取走后不再脏（一次变更只触发一次重渲染）。
    #[test]
    fn switching_marks_dirty_once() {
        let mut state = HostState::new();
        assert!(state.current().is_none());
        state.switch_to("settings");
        assert_eq!(state.current(), Some("settings"));
        assert!(state.is_dirty());
        assert!(state.take_dirty(), "切屏后必须重渲染一次");
        assert!(!state.is_dirty(), "取走后不应再脏");
        assert!(!state.take_dirty(), "同一帧内不应重复重渲染");
    }

    /// 多次标脏 ⇒ 仍然只触发一次重渲染（帧边界收口）。
    #[test]
    fn many_marks_collapse_into_one_render() {
        let mut state = HostState::new();
        for _ in 0..5 {
            state.mark_dirty();
        }
        assert!(state.take_dirty());
        assert!(!state.take_dirty(), "5 次标脏 ⇒ 帧边界只重渲染 1 次");
    }

    /// 清空 ⇒ 无当前屏幕、也不脏（屏幕被销毁后）。
    #[test]
    fn clear_resets_state() {
        let mut state = HostState::new();
        state.switch_to("settings");
        state.mark_dirty();
        state.clear();
        assert!(state.current().is_none());
        assert!(!state.is_dirty());
    }
}
