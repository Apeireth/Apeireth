//! wasm32 侧的装配（把状态条挂进**真实 DOM**）。
//!
//! ## 诚实边界
//!
//! - 本模块整体被 `#[cfg(target_arch = "wasm32")]` 隔离 ⇒ **native 编译不参与**
//!   ⇒ 它的行为**未经 native 侧验证**（浏览器内的验证由本面的对照 harness
//!   承担：像素 / DOM / 逐帧）。
//! - **真后端未接线**：数据是**固定 fixture**（引擎不碰 HTTP / SSE 契约）。
//! - 组件**不认识**业务语义：这一层只把固定数据**摆成**组件的 props。

use std::cell::RefCell;
use std::rc::Rc;

use apeireth_ui_core::{Callback, HostScheduler, Runtime, Signal, UiComponent};
use apeireth_ui_dom::{Renderer, VNode, WebDom};
use apeireth_ui_kit::status_bar::{
    StatusBadge, StatusBadgeProps, StatusBar, StatusBarItem, StatusBarProps, StatusIndicator,
};
use apeireth_ui_macro::view;
use wasm_bindgen::prelude::*;
use web_sys::Element;

use crate::fixture_generated::status_bar_cases;

#[wasm_bindgen]
extern "C" {
    /// 浏览器微任务队列（`queueMicrotask`）。
    #[wasm_bindgen(js_name = queueMicrotask)]
    fn queue_microtask(callback: &JsValue);
}

/// 微任务调度器：把 flush 排进浏览器的微任务队列。
struct MicrotaskScheduler;

impl HostScheduler for MicrotaskScheduler {
    fn schedule_flush(&self, flush: Box<dyn FnOnce()>) {
        let callback = Closure::once_into_js(move || {
            flush();
            JsValue::UNDEFINED
        });
        queue_microtask(&callback);
    }
}

/// 应用状态（响应式部分；**可共享** ⇒ effect 闭包能捕获它）。
#[derive(Clone)]
struct App {
    /// 运行时（信号宿主）。
    runtime: Runtime,
    /// 当前用例下标。
    case_index: Signal<usize>,
    /// 点击计数（**证据**：证明 Rust 侧的回调**真的**接上了）。
    clicks: Rc<RefCell<u32>>,
}

/// wasm 侧的单例应用。
struct Host {
    /// 应用状态。
    app: App,
    /// 渲染器（真实 DOM 后端）。
    renderer: Rc<RefCell<Renderer<WebDom>>>,
}

thread_local! {
    /// wasm 侧的单例应用。
    static HOST: RefCell<Option<Host>> = const { RefCell::new(None) };
}

/// 渲染整棵视图树（对照容器 + 状态条 + 徽标）。
fn render_tree(app: &App) -> VNode {
    let cases = status_bar_cases();
    let index = app.case_index.get();
    let Some(case) = cases.get(index) else {
        return VNode::fragment(Vec::new());
    };
    let items: Vec<StatusBarItem> = case
        .items
        .iter()
        .map(|cell| {
            let clicks = app.clicks.clone();
            StatusBarItem {
                key: cell.key.clone(),
                indicator: StatusIndicator {
                    text: cell.text.clone(),
                    tone: cell.tone,
                    title: cell.title.clone(),
                    clickable: cell.clickable,
                    sim: cell.sim,
                },
                // 每一格都带回调（与生产组件同构：四个回调**恒在**，只是不可点
                // 的格**不挂**监听器）⇒ 点击计数是「回调真的接上了」的证据。
                on_click: Some(Callback::new(move || {
                    let mut guard = clicks.borrow_mut();
                    *guard += 1;
                })),
            }
        })
        .collect();
    let bar = StatusBar::render(&StatusBarProps::new(items));
    let badge = StatusBadge::render(&StatusBadgeProps {
        variant: case.badge_variant,
        label: case.badge_label.clone(),
        size: case.badge_size,
    });
    view! {
        div(class = "statusbar-stage") {
            { bar }
            " "
            { badge }
        }
    }
}

/// 挂载状态条面。
fn mount(container: &Element) -> Result<(), String> {
    let dom = WebDom::new().map_err(|error| error.to_string())?;
    let root: web_sys::Node = container.clone().into();
    let mut renderer = Renderer::new(dom, root);

    let runtime = Runtime::new();
    runtime.set_host_scheduler(Rc::new(MicrotaskScheduler));
    let app = App {
        case_index: runtime.signal(0),
        clicks: Rc::new(RefCell::new(0)),
        runtime: runtime.clone(),
    };

    renderer
        .mount(render_tree(&app))
        .map_err(|error| error.to_string())?;

    let renderer = Rc::new(RefCell::new(renderer));
    let effect_renderer = renderer.clone();
    let effect_app = app.clone();
    runtime.effect(move || {
        let tree = render_tree(&effect_app);
        let mut guard = effect_renderer.borrow_mut();
        let _ = guard.patch(tree);
    });

    HOST.with(|slot| {
        *slot.borrow_mut() = Some(Host { app, renderer });
    });
    Ok(())
}

/// 引擎是否已就绪（挂载完成）。
#[wasm_bindgen]
pub fn is_ready() -> bool {
    HOST.with(|slot| slot.borrow().is_some())
}

/// 固定数据的用例数。
#[wasm_bindgen]
pub fn case_count() -> u32 {
    status_bar_cases().len() as u32
}

/// 切到第 `index` 组用例（越界 ⇒ 不切）。
#[wasm_bindgen]
pub fn select_case(index: u32) {
    HOST.with(|slot| {
        let slot = slot.borrow();
        let Some(host) = slot.as_ref() else {
            return;
        };
        let count = status_bar_cases().len();
        if (index as usize) < count {
            host.app.case_index.set(index as usize);
        }
    });
}

/// 点击计数（**证据**：Rust 侧回调被调用过几次）。
#[wasm_bindgen]
pub fn click_count() -> u32 {
    HOST.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(|host| *host.app.clicks.borrow())
            .unwrap_or(0)
    })
}

/// 渲染器是否已挂载（**证据**：引擎侧真的有 DOM 树）。
#[wasm_bindgen]
pub fn mounted() -> bool {
    HOST.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(|host| host.renderer.borrow().tree().is_some())
            .unwrap_or(false)
    })
}

/// 运行时里待处理的写（**证据**：响应式调度在工作）。
#[wasm_bindgen]
pub fn pending_writes() -> u32 {
    HOST.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(|host| host.app.runtime.pending() as u32)
            .unwrap_or(0)
    })
}

/// 启动（模块装载时跑一次）。
#[wasm_bindgen]
pub fn start() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Some(container) = document.get_element_by_id("app") else {
        return;
    };
    let _ = mount(&container);
}
