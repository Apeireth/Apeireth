//! 工具卡面批二（**工具栏 + 工具网格**）的 wasm 装配。
//!
//! ## 诚实边界
//!
//! | 事项 | 口径 |
//! |---|---|
//! | 数据 | **固定 fixture**（宿主注入）⇒ 引擎不碰 HTTP / SSE 契约（设计 §1.3 第 6 条） |
//! | 能力门控 | **不迁**（清单判定在宿主侧） |
//! | 详情弹窗 | 治理卷宗面 ⇒ **不迁** ⇒ 引擎侧「打开详情」**未接线**（点击无副作用） |
//! | 空态 / 加载态 / 错误态 | 状态条面 ⇒ **不迁** ⇒ 命中为空时**不渲染网格** |
//! | 搜索框双向绑定 | **未接线**（`view!` 的 `bind:value` 显式报错；VDOM 层无 property 通道）⇒ 搜索词由宿主写入信号 |
//!
//! ## 调度
//!
//! 用**同步** flush（默认调度器）⇒ 状态写入后同帧落 DOM。

#[cfg(target_arch = "wasm32")]
pub mod wasm {
    //! wasm32 侧的装配（**native 编译不参与** ⇒ 行为由浏览器内对照钉住）。

    use std::cell::RefCell;
    use std::rc::Rc;

    use apeireth_ui_core::{Runtime, Signal, UiComponent};
    use apeireth_ui_demo::toolgrid_fixture::{toolgrid_cases, toolgrid_tools};
    use apeireth_ui_dom::{Renderer, VNode, WebDom};
    use apeireth_ui_kit::tool_grid::{
        filter_tools, ToolGrid, ToolGridProps, ToolToolbar, ToolToolbarProps,
    };
    use apeireth_ui_macro::view;
    use wasm_bindgen::prelude::*;
    use web_sys::Element;

    /// 工具卡面应用的状态（响应式部分；**可共享** ⇒ effect 闭包能捕获它）。
    #[derive(Clone)]
    struct ToolGridState {
        /// 运行时（信号宿主）。
        runtime: Runtime,
        /// 搜索词（宿主写入 ⇒ 网格重算）。
        query: Signal<String>,
    }

    impl ToolGridState {
        /// 建一份状态。
        fn new() -> Self {
            let runtime = Runtime::new();
            let query = runtime.signal(String::new());
            Self { runtime, query }
        }
    }

    thread_local! {
        /// wasm 侧的单例应用（工具卡面形态）。
        static TOOLGRID_APP: RefCell<Option<ToolGridState>> = const { RefCell::new(None) };
    }

    /// 渲染工具卡面（舞台 = 工具栏 + 网格）。
    fn render_toolgrid(state: &ToolGridState) -> VNode {
        let tools = toolgrid_tools();
        let query = state.query.get();
        let filtered = filter_tools(&tools, &query);
        let toolbar = ToolToolbar::render(&ToolToolbarProps::new(tools.len(), state.query.clone()));
        // 命中为空 ⇒ 生产走**空态**（状态条面）⇒ 本批不迁 ⇒ **不渲染网格**。
        let grid = if filtered.is_empty() {
            VNode::fragment(Vec::new())
        } else {
            ToolGrid::render(&ToolGridProps::new(filtered))
        };
        view! {
            div(class = "toolgrid-stage") {
                { toolbar }
                " "
                { grid }
            }
        }
    }

    /// 挂载工具卡面。
    pub fn mount(container: &Element) -> Result<(), String> {
        let dom = WebDom::new().map_err(|error| error.to_string())?;
        let root: web_sys::Node = container.clone().into();
        let mut renderer = Renderer::new(dom, root);

        let state = ToolGridState::new();
        renderer
            .mount(render_toolgrid(&state))
            .map_err(|error| error.to_string())?;

        let renderer = Rc::new(RefCell::new(renderer));
        let effect_renderer = renderer.clone();
        let effect_state = state.clone();
        state.runtime.effect(move || {
            let tree = render_toolgrid(&effect_state);
            let mut guard = effect_renderer.borrow_mut();
            let _ = guard.patch(tree);
        });

        TOOLGRID_APP.with(|slot| {
            *slot.borrow_mut() = Some(state);
        });
        set_ready();
        Ok(())
    }

    /// 标记「数据已到」（对照脚本的 `ready()` 判据；两侧同名同义）。
    fn set_ready() {
        let Some(window) = web_sys::window() else {
            return;
        };
        let _ = js_sys::Reflect::set(
            window.as_ref(),
            &JsValue::from_str("__TOOLGRID_READY__"),
            &JsValue::TRUE,
        );
    }

    /// 固定数据的用例数。
    #[wasm_bindgen]
    pub fn toolgrid_case_count() -> u32 {
        toolgrid_cases().len() as u32
    }

    /// 写搜索词（宿主注入 ⇒ 网格重算）。
    #[wasm_bindgen]
    pub fn set_search(query: String) {
        TOOLGRID_APP.with(|slot| {
            if let Some(state) = slot.borrow().as_ref() {
                state.query.set(query);
            }
        });
    }
}
