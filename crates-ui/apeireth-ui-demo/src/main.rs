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
    pilot_callbacks, pilot_item, tool_card_cases, tree_to_text, ApprovalCard, ApprovalCardProps,
    PilotState,
};
#[cfg(not(target_arch = "wasm32"))]
use apeireth_ui_dom::Children;
#[cfg(not(target_arch = "wasm32"))]
use apeireth_ui_kit::tool_card::{
    ToolCallCard, ToolCallCardProps, ToolCallLifecycleCard, ToolCallLifecycleCardProps,
};

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

    out.push_str(&tool_card_demo());
    out
}

/// 跑一遍工具卡面流程，返回打印文本（**两张卡**各渲染一次）。
#[cfg(not(target_arch = "wasm32"))]
fn tool_card_demo() -> String {
    let runtime = apeireth_ui_core::Runtime::new();
    let mut out = String::new();
    out.push_str("\n== 工具卡面（固定数据的第一组用例） ==\n");
    let Some(case) = tool_card_cases().into_iter().next() else {
        return out;
    };
    let tool = ToolCallCard::render(&ToolCallCardProps::new(
        case.tool_call.clone(),
        runtime.signal(true),
        runtime.signal(true),
    ));
    out.push_str(&format!("--- {} ---\n", case.id));
    out.push_str(&tree_to_text(&tool));
    out.push('\n');
    let lifecycle = ToolCallLifecycleCard::render(&ToolCallLifecycleCardProps::new(
        case.lifecycle_call.clone(),
        runtime.signal(true),
    ));
    out.push_str(&tree_to_text(&lifecycle));
    out.push('\n');
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

    use apeireth_ui_core::{HostScheduler, Runtime, Signal, UiComponent};
    use apeireth_ui_demo::{
        pilot_callbacks, pilot_item, tool_card_cases, ApprovalCard, ApprovalCardProps, PilotState,
    };
    use apeireth_ui_dom::{Children, Renderer, VNode, WebDom};
    use apeireth_ui_kit::tool_card::{
        ToolCallCard, ToolCallCardProps, ToolCallLifecycleCard, ToolCallLifecycleCardProps,
    };
    use apeireth_ui_macro::view;
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
    ///
    /// ## 两种应用形态
    ///
    /// 同一个 wasm 产物挂两种应用（试点审批卡 / 工具卡面）⇒ 由
    /// `window.__APP_MODE__` 决定（**必须在 wasm 初始化之前**设定 ⇒ 初始化
    /// 即挂载）。取值未知 ⇒ 挂**试点审批卡**（默认形态）。
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
        if current_mode().as_deref() == Some(TOOLCARD_MODE) {
            let _ = mount_toolcards(&container);
            return;
        }
        let _ = mount_into(&container);
    }

    /// 应用形态的全局键名（**在 wasm 初始化之前**由页面设定）。
    const MODE_KEY: &str = "__APP_MODE__";

    /// 工具卡面形态的取值。
    const TOOLCARD_MODE: &str = "toolcard";

    /// 读当前应用形态（`None` ⇒ 默认形态）。
    fn current_mode() -> Option<String> {
        let window = web_sys::window()?;
        let value = js_sys::Reflect::get(window.as_ref(), &JsValue::from_str(MODE_KEY)).ok()?;
        if value.is_null() || value.is_undefined() {
            return None;
        }
        Some(value.as_string()?)
    }

    // ── 工具卡面应用 ──────────────────────────────────────────────────────

    /// 一组用例的交互信号（展开态 / 原始 JSON 态；**可共享**）。
    #[derive(Clone)]
    struct CaseSignals {
        /// 工具卡的展开态。
        tool_expanded: Signal<bool>,
        /// 工具卡的原始 JSON 态。
        tool_raw: Signal<bool>,
        /// 生命周期卡的展开态。
        lifecycle_expanded: Signal<bool>,
    }

    /// 工具卡面应用的状态（响应式部分；**可共享** ⇒ effect 闭包能捕获它）。
    #[derive(Clone)]
    struct ToolCardState {
        /// 运行时（信号宿主；`Runtime` 本身可共享）。
        runtime: Runtime,
        /// 当前用例下标。
        case_index: Signal<usize>,
        /// 每组用例的交互信号。
        cases: Vec<CaseSignals>,
    }

    impl ToolCardState {
        /// 运行时（信号宿主）。
        fn runtime(&self) -> &Runtime {
            &self.runtime
        }

        /// 建一份状态（每组用例一套信号）。
        fn new() -> Self {
            let runtime = Runtime::new();
            let count = tool_card_cases().len();
            let cases = (0..count)
                .map(|_| CaseSignals {
                    tool_expanded: runtime.signal(false),
                    tool_raw: runtime.signal(false),
                    lifecycle_expanded: runtime.signal(false),
                })
                .collect();
            Self {
                case_index: runtime.signal(0),
                cases,
                runtime,
            }
        }
    }

    /// 工具卡面应用（wasm 侧的运行时状态）。
    ///
    /// 渲染器由 **effect 闭包**持有（`Rc` 共享 ⇒ 渲染器随闭包存活）⇒ 本结构
    /// 只留状态。
    struct ToolCardApp {
        /// 应用状态。
        state: ToolCardState,
    }

    thread_local! {
        /// wasm 侧的单例应用（工具卡面形态）。
        static TOOLCARD_APP: RefCell<Option<ToolCardApp>> = const { RefCell::new(None) };
    }

    /// 挂载工具卡面（两张卡 + 对照容器）。
    fn mount_toolcards(container: &Element) -> Result<(), String> {
        let dom = WebDom::new().map_err(|error| error.to_string())?;
        let root: web_sys::Node = container.clone().into();
        let mut renderer = Renderer::new(dom, root);

        let state = ToolCardState::new();
        let scheduler_state = state.clone();
        scheduler_state
            .runtime()
            .set_host_scheduler(Rc::new(MicrotaskScheduler));

        renderer
            .mount(render_toolcards(&state))
            .map_err(|error| error.to_string())?;

        let renderer = Rc::new(RefCell::new(renderer));
        let effect_renderer = renderer.clone();
        let effect_state = state.clone();
        state.runtime().effect(move || {
            let tree = render_toolcards(&effect_state);
            let mut guard = effect_renderer.borrow_mut();
            let _ = guard.patch(tree);
        });

        TOOLCARD_APP.with(|slot| {
            *slot.borrow_mut() = Some(ToolCardApp { state });
        });
        Ok(())
    }

    /// 渲染工具卡面（对照容器 + 两张卡）。
    fn render_toolcards(state: &ToolCardState) -> VNode {
        let index = state.case_index.get();
        let cases = tool_card_cases();
        let Some(case) = cases.get(index) else {
            return VNode::fragment(Vec::new());
        };
        let Some(signals) = state.cases.get(index) else {
            return VNode::fragment(Vec::new());
        };
        let tool = ToolCallCard::render(&ToolCallCardProps::new(
            case.tool_call.clone(),
            signals.tool_expanded.clone(),
            signals.tool_raw.clone(),
        ));
        let lifecycle = ToolCallLifecycleCard::render(&ToolCallLifecycleCardProps::new(
            case.lifecycle_call.clone(),
            signals.lifecycle_expanded.clone(),
        ));
        view! {
            div(class = "toolcard-stage") {
                { tool }
                " "
                { lifecycle }
            }
        }
    }

    /// 固定数据的用例数。
    #[wasm_bindgen]
    pub fn case_count() -> u32 {
        tool_card_cases().len() as u32
    }

    /// 固定数据里全部 JSON 值的序列化结果（**浏览器差分测试**用）。
    ///
    /// 每项给出：紧凑序列化、缩进序列化（2 空格）、超长截断文本 ⇒ 浏览器侧用
    /// 同一批值跑脚本侧的序列化 ⇒ **逐字节**比对。
    #[wasm_bindgen]
    pub fn json_probe() -> String {
        use apeireth_ui_kit::json::JsonValue;
        use apeireth_ui_kit::tool_card::stringify_value;

        let mut values: Vec<(String, JsonValue)> = Vec::new();
        for case in tool_card_cases() {
            if let Some(args) = &case.tool_call.args {
                values.push((format!("{}:tool-args", case.id), args.clone()));
            }
            if let Some(result) = &case.tool_call.result_full {
                values.push((format!("{}:tool-result", case.id), result.clone()));
            }
            if let Some(args) = &case.lifecycle_call.arguments {
                values.push((format!("{}:lifecycle-args", case.id), args.clone()));
            }
            if let Some(result) = &case.lifecycle_call.result {
                values.push((format!("{}:lifecycle-result", case.id), result.clone()));
            }
        }
        let items: Vec<JsonValue> = values
            .iter()
            .map(|(id, value)| {
                JsonValue::Object(vec![
                    (String::from("id"), JsonValue::String(id.clone())),
                    (
                        String::from("compact"),
                        JsonValue::String(value.to_string_compact()),
                    ),
                    (
                        String::from("pretty"),
                        JsonValue::String(value.to_string_pretty()),
                    ),
                    (
                        String::from("truncated"),
                        JsonValue::String(stringify_value(value, 4000)),
                    ),
                ])
            })
            .collect();
        JsonValue::Array(items).to_string_compact()
    }

    /// 切到第 `index` 组用例（越界 ⇒ 不切）。
    #[wasm_bindgen]
    pub fn select_case(index: u32) {
        TOOLCARD_APP.with(|slot| {
            let slot = slot.borrow();
            let Some(app) = slot.as_ref() else {
                return;
            };
            let count = tool_card_cases().len();
            if (index as usize) < count {
                app.state.case_index.set(index as usize);
            }
        });
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

    #[test]
    fn demo_output_shows_the_tool_cards() {
        let text = super::run_demo();
        // 两张卡的根 class 都要出现（native 侧证明「组件 = Rust 函数」这条链跑得通）。
        assert!(text.contains("tool-card"), "实测输出：{text}");
        assert!(text.contains("tool-lifecycle-card"), "实测输出：{text}");
        // 展开态 ⇒ 展开体出现。
        assert!(text.contains("tool-body"), "实测输出：{text}");
    }
}
