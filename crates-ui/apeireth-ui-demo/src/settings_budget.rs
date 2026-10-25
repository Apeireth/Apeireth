//! 预算与配额（**设置页 · 预算面**）的 wasm 宿主装配。
//!
//! ## 本模块是什么
//!
//! **宿主侧**：持有本面的**分区**（`budget` / `runtime`）、**旋钮草稿 / 反馈**
//! 与**固定数据**（会话消耗累计 / 三枚旋钮的配置值 / 生产常量表），把
//! `ui_kit::settings_budget` 的组件挂到页面的 `#app` 容器里。
//!
//! ## 固定数据的来源
//!
//! | 数据 | 来源 | 备注 |
//! |---|---|---|
//! | 会话消耗累计 | `harness/settings/fixtures/budget.json` 的 `sessionUsage` | **两侧同一份** |
//! | 三枚旋钮的配置值 | `harness/settings/fixtures/capabilities.json` 的 `maxTurnRounds` / `maxToolCalls` / `contextBudgetChars` | **两侧同一份**（参考侧经 `config.capabilities` 喂给生产） |
//! | 旋钮规格表 / 配额维度表 / 耗尽语义表 | 生产 `lib/budget.ts`（**逐字**） | ⚠️ 生产侧仍是唯一真源；这里是**喂给引擎的固定数据**（生产常量表**不迁**进 Rust，见 `SPLIT.md` §2.5 注意点④） |
//!
//! ## 派生值：**引擎算**（宿主只喂输入）
//!
//! 会话消耗显示串 / 生效值徽标 / 余量行 / 余量条百分比 ⇒ 全部由
//! `ui_kit::settings_budget` 里**照抄生产**的纯函数算出（宿主**不重算**）。
//!
//! ## 诚实边界
//!
//! | 事项 | 口径 |
//! |---|
//! | 数据 | **固定 fixture**（宿主注入）⇒ 引擎不碰 HTTP / SSE 契约（设计 §1.3 第 6 条） |
//! | 深度诊断（`checkHealthDetailed`） | **HTTP ⇒ 宿主能力** ⇒ 本批**不接线** ⇒ 点按钮**只记录意图**（`window.__BUDGET__.getIntents()` 可读）⇒ **不假装已接线** |
//! | 诊断结果区 | 外部组件 `StatusBadge`（带自己的 scoped CSS）⇒ **未迁**（见 `ui_kit::settings_budget` 的模块文档） |
//! | 运行指标卡（`MetricsPanel`） | 外部组件 + HTTP（`/v1/panel/metrics`）⇒ **未迁** ⇒ 本面**不渲染**它 |
//! | 旋钮提交 | 引擎只发**意图**；宿主算反馈（用迁过来的解析函数）⇒ **真正的 apply（写配置 / 注 env / 热应用）在宿主之外** |

use std::cell::RefCell;
use std::rc::Rc;

use apeireth_ui_core::{Runtime, Signal};
use apeireth_ui_kit::settings_budget::{
    self as kit, BudgetPanelProps, BudgetSection, BudgetState, KnobDraft, KnobFlags, KnobKey,
    KnobRow, KnobValues, OutOfRange, QuotaDimension, SessionUsageTotals,
};
use apeireth_ui_kit::settings_frame::icons;

/// 固定数据（**生产常量表的逐字副本** + **两侧同源的 fixture 值**）。
///
/// ## 为什么不写进 `ui-kit`
///
/// `BUDGET_KNOB_SPECS` / `QUOTA_DIMENSIONS` / `BUDGET_EXHAUSTION` 在生产里是
/// **带文案与环境名的常量表** ⇒ 迁它们会打断生产的源码镜像测试
/// ⇒ **留生产侧**，由**宿主**作为固定数据喂进引擎（见 `SPLIT.md` §2.5）。
pub mod data {
    use super::*;

    /// 会话消耗累计（**两侧同源**：`fixtures/budget.json` 的 `sessionUsage`）。
    pub fn session_usage() -> SessionUsageTotals {
        SessionUsageTotals {
            turns: 7,
            prompt_tokens: Some(18420),
            completion_tokens: Some(3120),
            cache_hit_tokens: Some(9120),
            duration_ms: Some(214000),
            tool_calls: 12,
            last_turn_tool_calls: Some(3),
        }
    }

    /// 三枚旋钮的**配置值**（**两侧同源**：`fixtures/capabilities.json`）。
    pub fn knob_values() -> KnobValues {
        KnobValues {
            max_turn_rounds: Some(8),
            max_tool_calls: Some(16),
            context_budget_chars: Some(24000),
        }
    }

    /// 旋钮草稿（**原始字面量**；生产 `budgetDraftFromConfig` ⇒ `String(value)`）。
    pub fn knob_draft() -> KnobDraft {
        KnobDraft {
            max_turn_rounds: String::from("8"),
            max_tool_calls: String::from("16"),
            context_budget_chars: String::from("24000"),
        }
    }

    /// 旋钮行注册表（**顺序 = 生产顺序**；文案与环境名**逐字**取自生产）。
    pub fn knob_rows() -> Vec<KnobRow> {
        vec![
            KnobRow {
                key: KnobKey::MaxTurnRounds,
                label: String::from("轮数上限（每回合）"),
                env: String::from("APEIRETH_MAX_TURN_ROUNDS"),
                desc: String::from(
                    "单回合逻辑轮数上限（含模块重试占位）。轮预算耗尽收束为 turn_not_converged 错误帧，带真实数字。",
                ),
                min: 1,
                max: Some(64),
                fallback: 8,
                out_of_range: OutOfRange::Clamp,
                icon: &kit::ICON_TIMER,
                hint: String::from("1..=64；越界钳制，非法回默认 8。"),
            },
            KnobRow {
                key: KnobKey::MaxToolCalls,
                label: String::from("单轮工具上限"),
                env: String::from("APEIRETH_MAX_TOOL_CALLS"),
                desc: String::from(
                    "单轮最多派发多少个工具调用；超出部分截断并补合成结果（不整轮失败）。",
                ),
                min: 1,
                max: Some(64),
                fallback: 16,
                out_of_range: OutOfRange::Clamp,
                icon: &icons::ICON_WRENCH,
                hint: String::from("1..=64；越界钳制，非法回默认 16。"),
            },
            KnobRow {
                key: KnobKey::ContextBudgetChars,
                label: String::from("上下文字符预算"),
                env: String::from("APEIRETH_CONTEXT_BUDGET_CHARS"),
                desc: String::from(
                    "组装期注入上下文块（记忆注入 / 器官产物 / 教训等）的总字符预算；核心块永不截断，超出按长尾贪心截断。",
                ),
                min: 1,
                // ⚠️ **无上界** ⇒ 步进器**不写 `max` 属性**（生产 `max={spec.max ?? undefined}`）。
                max: None,
                fallback: 24000,
                // ⚠️ 上下文字符预算是 **`reject` 规格**（越界回默认，不钳制）。
                out_of_range: OutOfRange::Reject,
                icon: &kit::ICON_LAYERS,
                hint: String::from("正整数（字符）；越界/非法回默认 24000。"),
            },
        ]
    }

    /// 多维配额维度（**逐字**取自生产 `QUOTA_DIMENSIONS`；四维**都没有接口**）。
    pub fn quota_dimensions() -> Vec<QuotaDimension> {
        vec![
            QuotaDimension {
                key: String::from("tokens"),
                label: String::from("Token 上限"),
                reason: String::from(
                    "配额类型带该上限字段，但没有配置/env 接线（仅调度器单测构造，生产无调用点）——暂无接口。",
                ),
                evidence: String::from("cognitive_quota_scheduler.rs :: CognitiveQuota.max_tokens"),
            },
            QuotaDimension {
                key: String::from("steps"),
                label: String::from("步数上限"),
                reason: String::from(
                    "同上：步数上限字段无配置/env 接线。单轮工具调用上限属回合预算旋钮（见上），不是本维配额——暂无接口。",
                ),
                evidence: String::from("cognitive_quota_scheduler.rs :: CognitiveQuota.max_tool_steps"),
            },
            QuotaDimension {
                key: String::from("cost"),
                label: String::from("花费上限"),
                reason: String::from("同上：花费上限字段无配置/env 接线——暂无接口。"),
                evidence: String::from("cognitive_quota_scheduler.rs :: CognitiveQuota.max_cost_micros"),
            },
            QuotaDimension {
                key: String::from("depth"),
                label: String::from("深度上限"),
                reason: String::from(
                    "深度维度未实施：不跟踪调用深度，该上限字段已按「删除而非留空」移除——暂无接口。",
                ),
                evidence: String::from("cognitive_quota_scheduler.rs 模块诚实注记（无递归深度）"),
            },
        ]
    }

    /// 预算耗尽的**固定语义**（**逐字**取自生产 `BUDGET_EXHAUSTION.behaviors`）。
    pub fn exhaustion_behaviors() -> Vec<String> {
        vec![
            String::from(
                "轮预算耗尽：回合收束为 turn_not_converged 错误帧，带真实数字（limit / rounds / pending_tool）。",
            ),
            String::from(
                "同轮待批：审批冻结即收束为待批准态，不烧剩余轮预算；同回合重复提议同一待批项直接收束。",
            ),
            String::from("单轮工具调用超限：超出的调用被截断并补合成结果，不整轮失败。"),
            String::from(
                "多维配额耗尽中断：调度器定义了耗尽中断信号，但调度器未接入生产运行时，无可选行为。",
            ),
        ]
    }
}

/// 宿主状态（**分区 + 旋钮草稿 / 反馈**）。
#[derive(Clone)]
pub struct BudgetHost {
    /// 运行时（信号宿主）。
    runtime: Runtime,
    /// 当前分区。
    section: Signal<BudgetSection>,
    /// 旋钮草稿与反馈（`ui-kit` 的**状态容器**）。
    state: BudgetState,
    /// 已发出的意图（**可观察** ⇒ 对照脚本能断言「点了什么」）。
    intents: Rc<RefCell<Vec<String>>>,
}

impl BudgetHost {
    /// 建一份宿主状态（**生产同口径**：初始分区 = `budget`）。
    pub fn new() -> Self {
        let runtime = Runtime::new();
        // ⚠️ **状态与渲染必须同属一个 runtime**（2026-10-25 修）：
        // 此前 `BudgetState` **自建** runtime ⇒ ⇒ 草稿信号不在宿主的 runtime 里
        // ⇒ ⇒ effect **不依赖**它们 ⇒ ⇒ 改草稿**不重渲染** ⇒ ⇒ **幽灵输入**。
        let state = BudgetState::new(runtime.clone());
        // 草稿初值 = 配置值（生产 `budgetDraftFromConfig`）。
        state.set_draft(KnobKey::MaxTurnRounds, String::from("8"));
        state.set_draft(KnobKey::MaxToolCalls, String::from("16"));
        state.set_draft(KnobKey::ContextBudgetChars, String::from("24000"));
        Self {
            runtime: runtime.clone(),
            section: runtime.signal(BudgetSection::Budget),
            state,
            intents: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// 运行时（信号宿主）。
    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    /// 当前分区。
    pub fn section(&self) -> BudgetSection {
        self.section.get()
    }

    /// 切分区（**未知 id ⇒ 不切** ⇒ 不假装已接线）。
    pub fn select_section(&self, id: &str) {
        let next = match id {
            "budget" => BudgetSection::Budget,
            "runtime" => BudgetSection::Runtime,
            _ => return,
        };
        self.section.set(next);
    }

    /// 已发出的意图（**顺序 = 发出顺序**）。
    pub fn intents(&self) -> Vec<String> {
        self.intents.borrow().clone()
    }

    /// 记一条意图。
    fn record(&self, intent: String) {
        self.intents.borrow_mut().push(intent);
    }

    /// 旋钮草稿变更（**宿主侧** ⇒ 组件不自己改状态）。
    fn on_knob_input(&self, key: KnobKey, value: String) {
        self.record(format!("knob-input:{}", key.config_key()));
        self.state.set_draft(key, value);
    }

    /// 旋钮提交（失焦 / 回车）：算反馈 + 归一草稿。
    ///
    /// ## 边界
    ///
    /// 生产在这里还会走「整条 apply 缝」（写配置 → 注 env → 侧车热应用）
    /// ⇒ 那是**宿主之外**的能力 ⇒ 本批**只算反馈 + 归一草稿**
    /// ⇒ **不假装已接线**。
    fn on_knob_commit(&self, key: KnobKey) {
        self.record(format!("knob-commit:{}", key.config_key()));
        let rows = data::knob_rows();
        let Some(row) = rows.iter().find(|row| row.key == key) else {
            return;
        };
        let draft = self.state.draft();
        let resolution = kit::resolve_budget_limit_input(draft.get(key), row);
        self.state.set_feedback(key, resolution.feedback.clone());
        let normalized = match resolution.value {
            Some(value) => value.to_string(),
            None => String::new(),
        };
        self.state.set_draft(key, normalized);
    }

    /// 立即执行深度诊断（**HTTP ⇒ 宿主能力 ⇒ 本批不接线**）。
    fn on_check_runtime(&self) {
        // ⚠️ 深度诊断 = `checkHealthDetailed`（**HTTP**）⇒ 本批**不接线**
        // ⇒ 这里**只记录意图**（可观察）⇒ **不假装已接线**。
        self.record(String::from("check-diagnostics"));
    }

    /// 建一份组件 props（**状态归宿主** ⇒ 组件是纯函数）。
    pub fn props(&self) -> BudgetPanelProps {
        let input_host = self.clone();
        let commit_host = self.clone();
        let check_host = self.clone();
        BudgetPanelProps {
            section: self.section.get(),
            totals: data::session_usage(),
            knobs: data::knob_values(),
            rows: data::knob_rows(),
            draft: self.state.draft(),
            feedback: self.state.feedback(),
            flags: KnobFlags::default(),
            quota_dimensions: data::quota_dimensions(),
            exhaustion_behaviors: data::exhaustion_behaviors(),
            checking_runtime: self.state.checking_runtime(),
            on_knob_input: Rc::new(move |key, value| input_host.on_knob_input(key, value)),
            on_knob_commit: Rc::new(move |key| commit_host.on_knob_commit(key)),
            on_check_runtime: Rc::new(move || check_host.on_check_runtime()),
        }
    }
}

impl Default for BudgetHost {
    fn default() -> Self {
        Self::new()
    }
}

/// wasm32 侧的装配（**native 编译不参与** ⇒ 行为由浏览器内对照钉住）。
#[cfg(target_arch = "wasm32")]
pub mod wasm {
    //! ## 诚实边界
    //!
    //! - 本模块整体被 `#[cfg(target_arch = "wasm32")]` 隔离 ⇒ **native 编译不参与**
    //!   ⇒ 它的行为**未经 native 侧验证**（浏览器内的验证由设置页 harness 承担：
    //!   像素对照 + DOM 全等 + 宽度同源断言）。
    //! - **真后端未接线**：数据面是**固定 fixture**。
    //!
    //! ## 调度
    //!
    //! 写入 ⇒ 一次**微任务** flush ⇒ 同一轮事件处理里的多次写合并为一次
    //! DOM 写入 ⇒ 界面在同一帧响应。
    //!
    //! ## DOM property 回写（**为什么需要**）
    //!
    //! 生产的数字步进器用 `bind:value` ⇒ 值是 **property**（DOM 上**没有**
    //! `value` 属性）⇒ 引擎侧若只写属性，DOM 对照会多出属性、像素对照会看到
    //! **空输入框** ⇒ 本模块在每次渲染后**显式回写** property
    //! （**只在当前值 ≠ 目标值时写** ⇒ 不打断输入法组合态与光标位置）。
    //!
    //! ## 内联样式归一化（**为什么需要**）
    //!
    //! 生产用样式指令 ⇒ 浏览器把取值**序列化**成自己的口径（**尾分号**）⇒
    //! 引擎侧把同一段取值**再交给 CSSOM 走一遍**（`style.cssText = 原文`）
    //! ⇒ **由浏览器自己序列化** ⇒ 两侧同口径、逐字相等。

    use std::cell::RefCell;
    use std::rc::Rc;

    use apeireth_ui_core::{HostScheduler, UiComponent};
    use apeireth_ui_dom::{Renderer, VNode, WebDom};
    use apeireth_ui_kit::settings_budget::BudgetPanel;
    use apeireth_ui_macro::view;
    use js_sys::Reflect;
    use wasm_bindgen::prelude::*;
    use web_sys::Element;

    use super::BudgetHost;

    /// 微任务调度器：把 flush 排进浏览器的微任务队列。
    ///
    /// ⚠️ **不用** `#[wasm_bindgen] extern "C"` 声明 `queueMicrotask`：本模块
    /// 在 **lib 与 bin 里各编译一份**（共享的 `main.rs` 与 `lib.rs` 都声明了
    /// 它）⇒ wasm-bindgen 的导出/描述符号会**重复** ⇒ 链接失败。
    /// ⇒ 这里用 `js_sys::Reflect` 直接调全局函数（**零导出** ⇒ 零冲突）。
    struct MicrotaskScheduler;

    impl HostScheduler for MicrotaskScheduler {
        fn schedule_flush(&self, flush: Box<dyn FnOnce()>) {
            let queue = Reflect::get(&js_sys::global(), &JsValue::from_str("queueMicrotask"))
                .ok()
                .and_then(|value| value.dyn_into::<js_sys::Function>().ok());
            match queue {
                Some(queue) => {
                    let callback = Closure::once_into_js(move || {
                        flush();
                        JsValue::UNDEFINED
                    });
                    let _ = queue.call1(&JsValue::UNDEFINED, &callback);
                }
                None => {
                    // ⇒ 没有微任务队列 ⇒ **同步**跑（不静默丢刷新）。
                    flush();
                }
            }
        }
    }

    /// 已挂载的应用（**单例**）。
    struct BudgetApp {
        /// 宿主状态。
        host: BudgetHost,
    }

    thread_local! {
        /// wasm 侧的单例应用。
        static APP: RefCell<Option<BudgetApp>> = RefCell::new(None);
        /// 控制 API 的闭包（**必须活到页面结束** ⇒ 存在这里）。
        static CONTROL_CLOSURES: RefCell<Vec<Closure<dyn FnMut(JsValue) -> JsValue>>> =
            RefCell::new(Vec::new());
    }

    /// 渲染整棵视图树。
    ///
    /// ## 舞台与作用域锚点
    ///
    /// - `.settings-stage`：**对照专用样式表**里的 828px 舞台（参考侧靠生产
    ///   自己的布局得到同样的宽度 ⇒ 两侧裁剪框宽度相等）。
    /// - `.sp-budget`：**子面 scoped CSS 的作用域前缀**（抽取器按它前缀化）。
    fn render_tree(host: &BudgetHost) -> VNode {
        let panel = BudgetPanel::render(&host.props());
        view! {
            div(class = "settings-stage") {
                div(class = "sp-budget") {
                    { panel }
                }
            }
        }
    }

    /// 挂载预算面。
    ///
    /// 失败 ⇒ **既返回错误、又发布到页面**（`window.__SETTINGS_ERROR__`）⇒
    /// 调用方忽略返回值时**仍然看得见**（不静默）。
    pub fn mount(container: &Element) -> Result<(), String> {
        match mount_inner(container) {
            Ok(()) => Ok(()),
            Err(error) => {
                report_error(&error);
                Err(error)
            }
        }
    }

    /// 挂载预算面（内部实现）。
    fn mount_inner(container: &Element) -> Result<(), String> {
        let dom = WebDom::new().map_err(|error| error.to_string())?;
        let root: web_sys::Node = container.clone().into();
        let mut renderer = Renderer::new(dom, root);

        let host = BudgetHost::new();
        host.runtime()
            .set_host_scheduler(Rc::new(MicrotaskScheduler));

        renderer
            .mount(render_tree(&host))
            .map_err(|error| error.to_string())?;

        let renderer = Rc::new(RefCell::new(renderer));
        let effect_renderer = renderer.clone();
        let effect_host = host.clone();
        let effect_container = container.clone();
        host.runtime().effect(move || {
            let tree = render_tree(&effect_host);
            let mut guard = effect_renderer.borrow_mut();
            let _ = guard.patch(tree);
            drop(guard);
            // 补丁会把属性写回**原文** ⇒ 回写与归一化必须**跟着补丁**跑。
            let _ = sync_properties(&effect_host, &effect_container);
            let _ = normalize_inline_styles(&effect_container);
        });

        // 首次渲染后的 property 回写与内联样式归一化。
        sync_properties(&host, container)?;
        normalize_inline_styles(container)?;

        APP.with(|slot| {
            *slot.borrow_mut() = Some(BudgetApp { host: host.clone() });
        });

        install_control_api();
        patch_settings_api();
        mark_ready();
        Ok(())
    }

    // ⚠️ **本面不导出 `set_settings_section`**：wasm-bindgen 的导出符号取
    // **Rust 函数名** ⇒ 向导面已经占了 `set_settings_section` ⇒ 本面再导出
    // 同名函数 ⇒ wasm 链接阶段**重复符号**失败（实测报错：`already defined`）。
    // ⇒ 本面靠两条**不依赖导出名**的路径驱动分区：
    //   ① `patch_settings_api()` 包装 `window.__SETTINGS__.selectSection`
    //      （**共享契约** ⇒ 幂等 ⇒ 与导出重复设置也无副作用）；
    //   ② `window.__BUDGET__.selectSection(id)`（**本面控制 API** ⇒ 对照脚本
    //      直接调用 ⇒ 不依赖任何导出名）。

    /// 包裹共享 API 的 `selectSection`（**兜底**：让共享契约对本面**也**生效）。
    fn patch_settings_api() {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Ok(api) = Reflect::get(window.as_ref(), &JsValue::from_str("__SETTINGS__")) else {
            return;
        };
        if api.is_null() || api.is_undefined() {
            return;
        }
        let Ok(original) = Reflect::get(&api, &JsValue::from_str("selectSection")) else {
            return;
        };
        if original.is_null() || original.is_undefined() {
            return;
        }
        let Ok(original) = original.dyn_into::<js_sys::Function>() else {
            return;
        };
        let wrapped = Closure::wrap(Box::new(move |args: JsValue| -> JsValue {
            let _ = original.call1(&JsValue::NULL, &args);
            let id = args.as_string().unwrap_or_default();
            APP.with(|slot| {
                let slot = slot.borrow();
                let Some(app) = slot.as_ref() else {
                    return;
                };
                app.host.select_section(&id);
            });
            JsValue::UNDEFINED
        }) as Box<dyn FnMut(JsValue) -> JsValue>);
        let function: &js_sys::Function = wrapped.as_ref().unchecked_ref();
        let _ = Reflect::set(&api, &JsValue::from_str("selectSection"), function.as_ref());
        CONTROL_CLOSURES.with(|slot| slot.borrow_mut().push(wrapped));
    }

    /// 装控制 API（`window.__BUDGET__`）—— 对照脚本用它读状态与意图。
    fn install_control_api() {
        let Some(window) = web_sys::window() else {
            return;
        };

        let get_state = Closure::wrap(Box::new(|_args: JsValue| -> JsValue {
            let state = APP.with(|slot| {
                slot.borrow()
                    .as_ref()
                    .map(|app| (app.host.section(), app.host.intents()))
                    .unwrap_or_default()
            });
            let object = js_sys::Object::new();
            let set = |key: &str, value: JsValue| {
                let _ = Reflect::set(object.as_ref(), &JsValue::from_str(key), &value);
            };
            set(
                "section",
                JsValue::from_str(match state.0 {
                    apeireth_ui_kit::settings_budget::BudgetSection::Budget => "budget",
                    apeireth_ui_kit::settings_budget::BudgetSection::Runtime => "runtime",
                }),
            );
            let intents = js_sys::Array::new();
            for intent in state.1 {
                intents.push(&JsValue::from_str(&intent));
            }
            set("intents", intents.into());
            object.into()
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let select_section = Closure::wrap(Box::new(|args: JsValue| -> JsValue {
            let id = args.as_string().unwrap_or_default();
            APP.with(|slot| {
                let slot = slot.borrow();
                let Some(app) = slot.as_ref() else {
                    return;
                };
                app.host.select_section(&id);
            });
            JsValue::UNDEFINED
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let api = js_sys::Object::new();
        let function: &js_sys::Function = get_state.as_ref().unchecked_ref();
        let _ = Reflect::set(
            api.as_ref(),
            &JsValue::from_str("getState"),
            function.as_ref(),
        );
        let select: &js_sys::Function = select_section.as_ref().unchecked_ref();
        let _ = Reflect::set(
            api.as_ref(),
            &JsValue::from_str("selectSection"),
            select.as_ref(),
        );
        let _ = Reflect::set(
            api.as_ref(),
            &JsValue::from_str("ready"),
            &JsValue::from_bool(true),
        );
        let _ = Reflect::set(
            window.as_ref(),
            &JsValue::from_str("__BUDGET__"),
            api.as_ref(),
        );
        CONTROL_CLOSURES.with(|slot| {
            let mut slot = slot.borrow_mut();
            slot.push(get_state);
            slot.push(select_section);
        });
    }

    /// 回写 DOM property（**数字步进器的取值**）。
    ///
    /// ⚠️ **只在当前值 ≠ 目标值时写** ⇒ 不打断输入法组合态与光标位置。
    fn sync_properties(host: &BudgetHost, container: &Element) -> Result<(), String> {
        let draft = host.state.draft();
        let values = [
            draft.max_turn_rounds.clone(),
            draft.max_tool_calls.clone(),
            draft.context_budget_chars.clone(),
        ];
        let inputs = container
            .query_selector_all("input[type='number']")
            .map_err(|error| format!("查数字步进器失败：{error:?}"))?;
        for index in 0..inputs.length() {
            let Some(value) = values.get(index as usize) else {
                break;
            };
            let Some(item) = inputs.item(index) else {
                continue;
            };
            let Some(element) = item.dyn_ref::<Element>() else {
                continue;
            };
            let current = Reflect::get(element.as_ref(), &JsValue::from_str("value"))
                .map(|value| value.as_string().unwrap_or_default())
                .unwrap_or_default();
            if current == *value {
                continue;
            }
            Reflect::set(
                element.as_ref(),
                &JsValue::from_str("value"),
                &JsValue::from_str(value),
            )
            .map_err(|error| format!("写数字步进器的 value 失败：{error:?}"))?;
        }
        Ok(())
    }

    /// 把挂载出来的**内联样式**过一遍浏览器的 CSSOM（**归一化**）。
    fn normalize_inline_styles(container: &Element) -> Result<(), String> {
        let list = container
            .query_selector_all("[style]")
            .map_err(|error| format!("查内联样式元素失败：{error:?}"))?;
        for index in 0..list.length() {
            let Some(item) = list.item(index) else {
                continue;
            };
            let Some(element) = item.dyn_ref::<Element>() else {
                continue;
            };
            let Some(raw) = element.get_attribute("style") else {
                continue;
            };
            let style = Reflect::get(element.as_ref(), &JsValue::from_str("style"))
                .map_err(|error| format!("取样式对象失败：{error:?}"))?;
            Reflect::set(
                &style,
                &JsValue::from_str("cssText"),
                &JsValue::from_str(&raw),
            )
            .map_err(|error| format!("写样式对象失败：{error:?}"))?;
        }
        Ok(())
    }

    /// 标记「画面已就绪」（**共享 harness 的 `ready()` 读它**）。
    fn mark_ready() {
        let Some(window) = web_sys::window() else {
            return;
        };
        let _ = Reflect::set(
            window.as_ref(),
            &JsValue::from_str("__SETTINGS_READY__"),
            &JsValue::TRUE,
        );
    }

    /// 记录一条**显式错误**（对照脚本可读；不静默）。
    fn report_error(message: &str) {
        let Some(window) = web_sys::window() else {
            return;
        };
        let _ = Reflect::set(
            window.as_ref(),
            &JsValue::from_str("__SETTINGS_ERROR__"),
            &JsValue::from_str(message),
        );
    }
}

#[cfg(test)]
mod tests {
    //! 宿主侧的**行为**断言（状态转移 ⇒ 与生产同口径）。

    use super::*;

    #[test]
    fn the_fixed_data_matches_the_two_sides() {
        let totals = data::session_usage();
        assert_eq!(totals.turns, 7);
        assert_eq!(totals.prompt_tokens, Some(18420));
        assert_eq!(totals.last_turn_tool_calls, Some(3));
        let knobs = data::knob_values();
        assert_eq!(knobs.max_turn_rounds, Some(8));
        assert_eq!(knobs.max_tool_calls, Some(16));
        assert_eq!(knobs.context_budget_chars, Some(24000));
    }

    #[test]
    fn the_knob_rows_keep_the_production_order_and_domain() {
        let rows = data::knob_rows();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].key, KnobKey::MaxTurnRounds);
        assert_eq!(rows[1].key, KnobKey::MaxToolCalls);
        assert_eq!(rows[2].key, KnobKey::ContextBudgetChars);
        assert_eq!(rows[0].max, Some(64));
        assert_eq!(rows[2].max, None, "上下文字符预算**无上界**");
        assert_eq!(rows[2].out_of_range, OutOfRange::Reject);
        assert_eq!(rows[0].out_of_range, OutOfRange::Clamp);
    }

    #[test]
    fn the_quota_dimensions_all_say_no_interface() {
        let dims = data::quota_dimensions();
        assert_eq!(dims.len(), 4);
        for dim in &dims {
            assert!(dim.reason.contains("暂无接口"), "{}", dim.reason);
            assert!(!dim.evidence.is_empty());
        }
    }

    #[test]
    fn the_initial_section_is_the_budget_one() {
        let host = BudgetHost::new();
        assert_eq!(host.section(), BudgetSection::Budget);
    }

    #[test]
    fn selecting_an_unknown_section_changes_nothing() {
        let host = BudgetHost::new();
        host.select_section("nope");
        assert_eq!(host.section(), BudgetSection::Budget);
        host.select_section("runtime");
        assert_eq!(host.section(), BudgetSection::Runtime);
    }

    #[test]
    fn the_knob_input_updates_the_draft_and_records_an_intent() {
        let host = BudgetHost::new();
        host.on_knob_input(KnobKey::MaxTurnRounds, String::from("12"));
        assert_eq!(host.state.draft().max_turn_rounds, "12");
        assert_eq!(
            host.intents(),
            vec![String::from("knob-input:maxTurnRounds")]
        );
    }

    #[test]
    fn the_knob_commit_normalizes_the_draft_and_writes_the_feedback() {
        let host = BudgetHost::new();
        host.on_knob_input(KnobKey::MaxTurnRounds, String::from("100"));
        host.on_knob_commit(KnobKey::MaxTurnRounds);
        assert_eq!(host.state.draft().max_turn_rounds, "64");
        assert_eq!(
            host.state.feedback().max_turn_rounds,
            "越界：已钳制到 64（1..=64）"
        );
        assert_eq!(
            host.intents(),
            vec![
                String::from("knob-input:maxTurnRounds"),
                String::from("knob-commit:maxTurnRounds"),
            ]
        );
    }

    #[test]
    fn the_reject_knob_falls_back_to_the_default() {
        let host = BudgetHost::new();
        host.on_knob_input(KnobKey::ContextBudgetChars, String::from("0"));
        host.on_knob_commit(KnobKey::ContextBudgetChars);
        assert_eq!(host.state.draft().context_budget_chars, "");
        assert_eq!(
            host.state.feedback().context_budget_chars,
            "越界（1 起）：已回默认 24000"
        );
    }

    #[test]
    fn the_diagnostics_button_only_records_an_intent() {
        // 深度诊断是 **HTTP** ⇒ 本批**不接线** ⇒ 只记录意图。
        let host = BudgetHost::new();
        host.on_check_runtime();
        assert_eq!(host.intents(), vec![String::from("check-diagnostics")]);
        assert!(!host.state.checking_runtime(), "未接线 ⇒ 不假装在诊断");
    }

    #[test]
    fn the_rendered_tree_carries_the_budget_panel() {
        let host = BudgetHost::new();
        let props = host.props();
        assert_eq!(props.section, BudgetSection::Budget);
        assert_eq!(props.rows.len(), 3);
        assert_eq!(props.quota_dimensions.len(), 4);
        assert_eq!(props.exhaustion_behaviors.len(), 4);
        assert_eq!(props.draft.max_turn_rounds, "8");
    }
}
