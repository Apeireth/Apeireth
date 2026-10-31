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
    /// 引擎渲染器（**共享**给自动渲染的 effect ⇒ ⇒ 用 `Rc<RefCell<…>>`）。
    renderer: Rc<RefCell<Renderer<WebDom>>>,
    /// 宿主状态（当前屏幕 + 脏标记）。
    state: Rc<RefCell<HostState>>,
    /// 响应式运行时（**自动渲染**的驱动：effect + flush）。
    runtime: Runtime,
    /// 当前屏幕（共享给 effect ⇒ effect 能读它的状态信号 ⇒ ⇒ 依赖被追踪）。
    screen: Rc<dyn Screen>,
    /// 自动渲染的 effect 句柄（**必须保留** ⇒ 否则效果被立刻撤销）。
    ///
    /// ⚠️ core 的 effect 语义：「创建时不执行、依赖标脏后重跑、重跑前清空旧依赖」。
    #[allow(dead_code)]
    auto_render: apeireth_ui_core::EffectHandle,
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
    let screen: Rc<dyn Screen> = Rc::new(DemoScreen::new(&runtime));
    state.borrow_mut().switch_to(screen.id());

    let mut renderer = Renderer::new(dom, root);
    let view = screen.view();
    renderer
        .render_guarded(view)
        .map_err(|error| JsValue::from_str(&format!("宿主薄壳：首屏渲染失败：{error}")))?;
    let renderer = Rc::new(RefCell::new(renderer));

    // ⭐ **自动渲染**：effect 体里读屏幕的状态信号 ⇒ ⇒ 依赖被 core 追踪
    // ⇒ ⇒ 状态变 ⇒ effect 被标脏 ⇒ ⇒ **flush 时**重建视图 + patch DOM。
    // ⇒ ⇒ 宿主**不需要**手动「重建视图」—— 那正是自动调度的意义。
    let auto_render = {
        let screen = screen.clone();
        let renderer = renderer.clone();
        runtime.effect(move || {
            let view = screen.view();
            // ⭐ **自动建立依赖**：遍历树里所有绑定并读一次 ⇒ ⇒ core 收集到依赖
            // ⇒ ⇒ 屏幕作者**不需要**手动 `signal.get()`（这正是本批要消除的易漏点）。
            view.touch_bindings();
            let mut guard = renderer.borrow_mut();
            let _ = guard.patch(view);
        })
    };

    HOST.with(|slot| {
        *slot.borrow_mut() = Some(Host {
            renderer,
            state,
            runtime,
            screen,
            auto_render,
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
            .borrow_mut()
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
        host.auto_render.dispose();
        host.renderer
            .borrow_mut()
            .dispose()
            .map_err(|error| JsValue::from_str(&format!("宿主薄壳：销毁失败：{error}")))?;
        host.state.borrow_mut().clear();
        Ok(())
    })
}

/// ⭐ **改演示屏的文本**（演示屏专用接口）。
///
/// ## 为什么暴露它
///
/// 演示屏的信号是**内部**的 ⇒ 外部改不到 ⇒ ⇒ 没有它就无法演示
/// 「信号变 ⇒ 重渲染 ⇒ DOM 更新」这条链 ⇒ ⇒ 这条链是宿主运行时最核心的契约。
///
/// ⚠️ 只影响**演示屏**；真实屏幕由自己的状态源驱动。
#[wasm_bindgen]
pub fn set_demo_text(text: &str) -> Result<(), JsValue> {
    HOST.with(|slot| {
        let guard = slot.borrow();
        let Some(host) = guard.as_ref() else {
            return Err(JsValue::from_str("宿主薄壳：还没 boot"));
        };
        // 演示屏的信号访问（内部可见性靠同 crate 的 `pub(crate)` 通道）。
        if let Some(demo) = host.screen.as_any().downcast_ref::<DemoScreen>() {
            demo.text.set(text.to_owned());
            // ⚠️ **不 patch** ⇒ ⇒ DOM 的更新由**自动渲染**（effect + flush）完成
            // ⇒ ⇒ 这正是「信号 ⇒ 自动重渲染」的证据点。
            host.state.borrow_mut().mark_dirty();
            return Ok(());
        }
        Err(JsValue::from_str("宿主薄壳：当前屏幕不是演示屏"))
    })
}

/// ⭐ **flush**：跑完当前所有待办（含**自动渲染**的 effect）。
///
/// ## 宿主该在什么时候调
///
/// ⚠️ **实测口径（2026-10-31 钉住）**：core 的 effect 是**同步重跑**的
/// ⇒ ⇒ 状态变 ⇒ effect **立即**跑 ⇒ ⇒ DOM **立刻**更新
/// ⇒ ⇒ **不需要**（也不能靠）flush 来「延后」更新。
///
/// ⇒ 所以 `flush` 的作用是**排空待办**（若有待办则跑完；无待办则**幂等无副作用**）
/// ⇒ ⇒ 宿主可在事件批次结束时调一次做**兜底排空**。
///
/// ⚠️ 「一帧内多次状态变更 ⇒ 只重渲染一次」**尚未实现**
/// ⇒ ⇒ 那需要**上层显式批处理**（core 不提供；下一批）。
#[wasm_bindgen]
pub fn flush() -> Result<bool, JsValue> {
    HOST.with(|slot| {
        let guard = slot.borrow();
        let Some(host) = guard.as_ref() else {
            return Err(JsValue::from_str("宿主薄壳：还没 boot"));
        };
        let report = host.runtime.flush();
        Ok(report.effects_run > 0)
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

    fn as_any(&self) -> &dyn std::any::Any {
        self
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
