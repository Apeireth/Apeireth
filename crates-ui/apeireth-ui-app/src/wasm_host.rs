//! ⭐ **wasm 薄壳**：把契约（[`crate::Screen`] / [`crate::HostState`]）接到**真实 DOM**。
//!
//! ## 这个模块是什么
//!
//! **唯一**把「屏幕」变成真实 DOM 的地方。三条 wasm 导出：
//!
//! - [`boot`]：接管一个容器元素 ⇒ 建引擎 ⇒ 挂载首个屏幕；
//! - [`rerender`]：按宿主状态的脏标记决定要不要重渲染（**帧边界收口**）；
//! - [`dispose_screen`]：销毁当前屏幕（**必须**：否则旧屏的焦点陷阱/IME 队列会渗进新屏）。
//!
//! ## ⚠️ 「信号变 ⇒ 自动重渲染」**未做**（诚实标注）
//!
//! 当前 [`rerender`] 由宿主**显式**调用 ⇒ ⇒ 屏幕状态变了要自己调一次
//! ⇒ ⇒ 「信号变更 ⇒ 自动调度」属**自动调度层**，本批**不引入**（避免未验证的调度语义）。
//!
//! ## 为什么薄
//!
//! 契约在 [`crate`]（纯逻辑，native 可测）⇒ 这里只做三件事：
//! **建引擎 / 挂树 / 销毁** ⇒ ⇒ wasm 侧没有业务逻辑 ⇒ ⇒ 逻辑改动不需要重新验证浏览器。
//!
//! ## ⚠️ 口径（诚实）
//!
//! - 本模块**只在 wasm32 目标编译** ⇒ native 侧不参与；
//! - **WebView2 未验证**（真实 Windows 宿主未跑）⇒ 当前证据只覆盖
//!   **系统 Chromium**（浏览器探针）⇒ ⇒ 切到生产宿主前必须补 C2。

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;

use apeireth_ui_core::Runtime;
use apeireth_ui_dom::dom::Dom;
use apeireth_ui_dom::web::WebDom;
use apeireth_ui_dom::Renderer;

use crate::{HostState, Screen};

/// 宿主实例（wasm 单线程 ⇒ `thread_local`）。
struct Host {
    /// 引擎渲染器。
    renderer: Renderer<WebDom>,
    /// 宿主状态（当前屏幕 + 脏标记）。
    state: Rc<RefCell<HostState>>,
    /// 当前屏幕。
    screen: Box<dyn Screen>,
}

thread_local! {
    /// 当前宿主（`None` = 还没 boot）。
    static HOST: RefCell<Option<Host>> = const { RefCell::new(None) };
}

/// ⭐ **boot**：接管容器元素 ⇒ 建引擎 ⇒ 挂载首个屏幕。
///
/// # Errors
///
/// 找不到容器 / 建引擎失败 ⇒ 返回 JS 错误（宿主可捕获 ⇒ 不静默白屏）。
#[wasm_bindgen]
pub fn boot(container_selector: &str) -> Result<(), JsValue> {
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("宿主薄壳：没有 document"))?;
    let element = document
        .query_selector(container_selector)?
        .ok_or_else(|| JsValue::from_str(&format!("宿主薄壳：找不到容器 {container_selector}")))?;

    let dom = WebDom::new().map_err(|error| JsValue::from_str(&format!("宿主薄壳：{error}")))?;
    // WebDom 的节点是**它自己**的树 ⇒ ⇒ 需要把它挂到真实容器上：
    // 用 `create_element` 在真实 DOM 建根，再把引擎的树挂进去。
    let root = dom
        .create_element("div")
        .map_err(|error| JsValue::from_str(&format!("宿主薄壳：建根失败：{error}")))?;
    let _ = element.append_child(&root);

    let runtime = Runtime::new();
    let state = Rc::new(RefCell::new(HostState::new()));
    let screen = Box::new(DemoScreen::new(&runtime));
    state.borrow_mut().switch_to(screen.id());

    let mut renderer = Renderer::new(dom, root);
    let view = screen.view();
    renderer
        .render_guarded(view)
        .map_err(|error| JsValue::from_str(&format!("宿主薄壳：首屏渲染失败：{error}")))?;

    HOST.with(|slot| {
        *slot.borrow_mut() = Some(Host {
            renderer,
            state,
            screen,
        });
    });
    Ok(())
}

/// ⭐ **rerender**：按脏标记决定是否重渲染（帧边界收口 ⇒ 多次变更只渲染一次）。
///
/// # Errors
///
/// 未 boot ⇒ 返回 JS 错误。
#[wasm_bindgen]
pub fn rerender() -> Result<bool, JsValue> {
    HOST.with(|slot| {
        let mut guard = slot.borrow_mut();
        let Some(host) = guard.as_mut() else {
            return Err(JsValue::from_str("宿主薄壳：还没 boot"));
        };
        if !host.state.borrow_mut().take_dirty() {
            return Ok(false);
        }
        let view = host.screen.view();
        host.renderer
            .patch(view)
            .map_err(|error| JsValue::from_str(&format!("宿主薄壳：重渲染失败：{error}")))?;
        Ok(true)
    })
}

/// ⭐ **dispose_screen**：销毁当前屏幕（清焦点/陷阱/IME 队列/捕获错误）。
///
/// # Errors
///
/// 未 boot ⇒ 返回 JS 错误。
#[wasm_bindgen]
pub fn dispose_screen() -> Result<(), JsValue> {
    HOST.with(|slot| {
        let mut guard = slot.borrow_mut();
        let Some(host) = guard.as_mut() else {
            return Err(JsValue::from_str("宿主薄壳：还没 boot"));
        };
        host.renderer
            .dispose()
            .map_err(|error| JsValue::from_str(&format!("宿主薄壳：销毁失败：{error}")))?;
        host.state.borrow_mut().clear();
        Ok(())
    })
}

/// ⭐ **演示屏幕**：一个受控输入框（信号 ⇄ DOM 双向绑定）。
///
/// ## 为什么内置演示屏
///
/// 它是**最小可运行**的宿主证据：信号变 ⇒ DOM 更新；用户输入 ⇒ 信号更新
/// ⇒ ⇒ 一眼就能看出「契约 → 引擎 → 真实 DOM」这条链通不通。
pub struct DemoScreen {
    /// 输入框的值（信号）。
    text: apeireth_ui_core::Signal<String>,
}

impl DemoScreen {
    /// 建演示屏（`runtime` 提供信号与运行时）。
    pub fn new(runtime: &Runtime) -> Self {
        Self {
            text: runtime.signal(String::from("宿主已就绪")),
        }
    }
}

impl Screen for DemoScreen {
    fn id(&self) -> &str {
        "demo"
    }

    fn view(&self) -> apeireth_ui_dom::vnode::VNode {
        use apeireth_ui_dom::binding::Binding;
        use apeireth_ui_dom::vnode::VNode;

        VNode::element("section")
            .attr("class", "app-screen")
            .child(
                VNode::element("input")
                    .attr("id", "app-demo-input")
                    .bind(Binding::text(self.text.clone()))
                    .build(),
            )
            .build()
    }
}
