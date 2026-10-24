//! 试点应用入口。
//!
//! ## 两个形态
//!
//! | 目标 | 入口 | 产出 |
//! |---|---|---|
//! | **native** | [`run_demo`] | 打印虚拟树（证明「组件 = Rust 函数」这条链跑得通） |
//! | **wasm32** | [`wasm_app`] | 把试点卡片挂进**真实 DOM**（浏览器内对照的引擎侧） |
//!
//! ## 诚实口径
//!
//! - native 打印的是**虚拟树文本**，不是真实 DOM —— 真实 DOM 后端（web-sys）的
//!   **运行时**验证由试点 harness 承担（像素对照 + 逐帧对照，见 `harness/`）。
//! - wasm 侧喂的是**固定 fixture**（不连真实后端）：引擎不碰 HTTP / SSE 契约
//!   （设计文档 §1.3 第 6 条）。

#[cfg(not(target_arch = "wasm32"))]
use apeireth_ui_core::UiComponent;
#[cfg(not(target_arch = "wasm32"))]
use apeireth_ui_demo::{
    pilot_callbacks, pilot_item, tree_to_text, ApprovalCard, ApprovalCardProps, PilotState,
};
#[cfg(not(target_arch = "wasm32"))]
use apeireth_ui_dom::Children;

/// 跑一遍试点流程，返回打印文本（入口与测试共用同一路径）。
#[cfg(not(target_arch = "wasm32"))]
fn run_demo() -> String {
    let state = PilotState::new();
    let (allow, reject, dismiss) = pilot_callbacks(&state);
    let props = ApprovalCardProps {
        item: pilot_item(),
        busy: state.busy().get(),
        copied: state.copied(),
        on_allow: allow,
        on_reject: reject,
        on_dismiss: Some(dismiss),
        children: Children::default(),
    };

    let mut out = String::new();
    out.push_str("== 渲染结果 ==\n");
    out.push_str(&tree_to_text(&ApprovalCard::render(&props)));

    out.push_str("\n\n== 关闭卡片（不做业务决策） ==\n");
    state.dismiss();
    state.flush();
    out.push_str(&format!(
        "可见：{} / 决策：{:?}\n",
        state.visible().get(),
        state.decision().get()
    ));

    out.push_str("\n== 允许 ==\n");
    state.allow();
    state.flush();
    out.push_str(&format!("决策：{:?}\n", state.decision().get()));
    out
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    print!("{}", run_demo());
}

#[cfg(target_arch = "wasm32")]
fn main() {
    wasm_app::start();
}

/// wasm32 侧的装配（把试点卡片挂进真实 DOM）。
#[cfg(target_arch = "wasm32")]
pub mod wasm_app {
    //! ## 诚实边界
    //!
    //! - 本模块整体被 `#[cfg(target_arch = "wasm32")]` 隔离 ⇒ **native 编译不参与**
    //!   ⇒ 它的行为**未经 native 侧验证**（浏览器内的验证由试点 harness 承担：
    //!   像素对照 + 逐帧对照，见 `crates-ui/apeireth-ui-demo/harness/`）。
    //! - **真后端未接线**：数据面是**固定 fixture**。
    //!
    //! ## 调度
    //!
    //! 写入 ⇒ 一次**微任务** flush（设计文档 §4.2 规则 1）⇒ 同一轮事件处理里的
    //! 多次写合并为一次 DOM 写入 ⇒ 界面在同一帧响应。

    use std::cell::RefCell;
    use std::rc::Rc;

    use apeireth_ui_core::{HostScheduler, UiComponent};
    use apeireth_ui_demo::{
        pilot_callbacks, pilot_item, ApprovalCard, ApprovalCardProps, PilotState,
    };
    use apeireth_ui_dom::{Children, Renderer, VNode, WebDom};
    use wasm_bindgen::prelude::*;
    use web_sys::Element;

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

    /// 试点应用（wasm 侧的运行时状态）。
    struct PilotApp {
        /// 试点状态（信号 + 回调）。
        state: PilotState,
        /// 渲染器（真实 DOM 后端）。
        renderer: Rc<RefCell<Renderer<WebDom>>>,
    }

    thread_local! {
        /// wasm 侧的单例应用。
        static APP: RefCell<Option<PilotApp>> = const { RefCell::new(None) };
    }

    /// 挂载试点卡片。
    fn mount_into(container: &Element) -> Result<(), String> {
        let dom = WebDom::new().map_err(|error| error.to_string())?;
        let root: web_sys::Node = container.clone().into();
        let mut renderer = Renderer::new(dom, root);

        let state = PilotState::new();
        state
            .runtime()
            .set_host_scheduler(Rc::new(MicrotaskScheduler));

        let (allow, reject, dismiss) = pilot_callbacks(&state);
        let props = ApprovalCardProps {
            item: pilot_item(),
            busy: state.busy().get(),
            copied: state.copied(),
            on_allow: allow,
            on_reject: reject,
            on_dismiss: Some(dismiss),
            children: Children::default(),
        };

        renderer
            .mount(ApprovalCard::render(&props))
            .map_err(|error| error.to_string())?;

        let renderer = Rc::new(RefCell::new(renderer));
        let props = Rc::new(RefCell::new(props));

        // 响应式重绘：effect 读到的信号变了 ⇒ 重算虚拟树 ⇒ 就地打补丁。
        let effect_renderer = renderer.clone();
        let effect_props = props.clone();
        let effect_state = state.clone();
        state.runtime().effect(move || {
            let tree = render_tree(&effect_state, &effect_props);
            let mut guard = effect_renderer.borrow_mut();
            let _ = guard.patch(tree);
        });

        APP.with(|slot| {
            *slot.borrow_mut() = Some(PilotApp { state, renderer });
        });
        Ok(())
    }

    /// 渲染整棵视图树（卡片可见 ⇒ 卡片；不可见 ⇒ 空树 ⇒ 节点被移除）。
    fn render_tree(state: &PilotState, props: &Rc<RefCell<ApprovalCardProps>>) -> VNode {
        if !state.visible().get() {
            return VNode::fragment(Vec::new());
        }
        let mut current = props.borrow_mut();
        current.busy = state.busy().get();
        ApprovalCard::render(&current)
    }

    /// 引擎是否已就绪（挂载完成）。
    #[wasm_bindgen]
    pub fn is_ready() -> bool {
        APP.with(|slot| slot.borrow().is_some())
    }

    /// 设置「处理中」状态（对照 harness 的逐帧触发点）。
    #[wasm_bindgen]
    pub fn set_busy(value: bool) {
        APP.with(|slot| {
            if let Some(app) = slot.borrow().as_ref() {
                app.state.busy().set(value);
            }
        });
    }

    /// 设置「已复制」状态。
    #[wasm_bindgen]
    pub fn set_copied(value: bool) {
        APP.with(|slot| {
            if let Some(app) = slot.borrow().as_ref() {
                app.state.copied().set(value);
            }
        });
    }

    /// 触发「允许」。
    #[wasm_bindgen]
    pub fn allow() {
        APP.with(|slot| {
            if let Some(app) = slot.borrow().as_ref() {
                app.state.allow();
            }
        });
    }

    /// 触发「拒绝」。
    #[wasm_bindgen]
    pub fn reject() {
        APP.with(|slot| {
            if let Some(app) = slot.borrow().as_ref() {
                app.state.reject();
            }
        });
    }

    /// 触发「关闭」（只关卡片，不做业务决策）。
    #[wasm_bindgen]
    pub fn dismiss() {
        APP.with(|slot| {
            if let Some(app) = slot.borrow().as_ref() {
                app.state.dismiss();
            }
        });
    }

    /// 业务决策的当前值（`None` / `Some(true)` / `Some(false)`）。
    #[wasm_bindgen]
    pub fn decision() -> JsValue {
        APP.with(|slot| {
            let value = slot
                .borrow()
                .as_ref()
                .map(|app| app.state.decision().get())
                .unwrap_or(None);
            match value {
                None => JsValue::NULL,
                Some(flag) => JsValue::from_bool(flag),
            }
        })
    }

    /// 卡片是否仍在文档里（关闭后应为 `false`）。
    #[wasm_bindgen]
    pub fn card_present() -> bool {
        APP.with(|slot| {
            let slot = slot.borrow();
            let Some(app) = slot.as_ref() else {
                return false;
            };
            let renderer = app.renderer.borrow();
            renderer.tree().is_some_and(|tree| {
                tree.children()
                    .iter()
                    .any(|child| child.dom_node().is_some())
            })
        })
    }

    /// 启动（模块装载时跑一次）。
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
        let _ = mount_into(&container);
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    #[test]
    fn demo_output_shows_the_state_transitions() {
        let text = super::run_demo();
        assert!(text.contains("读取卷宗目录"), "实测输出：{text}");
        assert!(text.contains("批准"), "实测输出：{text}");
        assert!(text.contains("决策：None"), "实测输出：{text}");
        assert!(text.contains("决策：Some(true)"), "实测输出：{text}");
    }
}
