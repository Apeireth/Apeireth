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

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use apeireth_ui_core::{Runtime, Signal};
use apeireth_ui_dom::dom::Dom;
use apeireth_ui_dom::vnode::VNode;
use apeireth_ui_dom::web::WebDom;
use apeireth_ui_dom::Renderer;

use crate::async_line::AsyncLine;
use crate::{HostState, Screen};

/// ⭐ E-182：演示屏的**异步数据线通道名**（按通道隔离 ⇒ ⇒ 不与别的数据线互相干扰）。
const DEMO_ASYNC_CHANNEL: &str = "demo-async";

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
    /// ⭐ 帧循环是否在跑（`dispose` 时置 false ⇒ 循环自然停下）。
    running: Rc<Cell<bool>>,
    /// ⭐ 当前排着的帧号（`dispose` 时用它 `cancel_animation_frame` ⇒ ⇒ 立刻停）。
    frame_id: Rc<Cell<i32>>,
    /// ⭐ 帧槽（`dispose` 时释放 ⇒ ⇒ 帧闭包随之被 drop ⇒ ⇒ **零残留**）。
    frame_slot: Rc<FrameSlot>,
    /// 自动渲染的 effect 句柄（**必须保留** ⇒ 否则效果被立刻撤销）。
    ///
    /// ⚠️ core 的 effect 语义：「创建时不执行、依赖标脏后重跑、重跑前清空旧依赖」。
    #[allow(dead_code)]
    auto_render: apeireth_ui_core::EffectHandle,
    /// ⭐⭐⭐ **E-182：异步数据线的定时器闭包**（**必须保活**）。
    ///
    /// ⇒ ⇒ `Closure` 一旦被 drop ⇒ ⇒ JS 侧回调**立刻失效** ⇒ ⇒ 定时器**永远不会到点**
    /// ⇒ ⇒⇒ 所以必须**由宿主持有**到它触发为止。
    ///
    /// ⚠️ 为什么不「发出新查询就丢掉旧的」：那等于**取消**上一个请求
    /// ⇒ ⇒⇒ 而「乱序返回」恰恰要求**旧请求仍在飞**（已发出、不可撤回）
    /// ⇒ ⇒⇒ ⇒ 丢掉闭包 = 悄悄取消 ⇒ ⇒ **仲裁就永远不会被触发** ⇒ ⇒ 证据就没了。
    /// ⇒ ⇒⇒ ⇒ 故：**攒着**，`dispose_screen` 时统一清空（drop ⇒ 回调失效 ⇒ 零残留）。
    timers: TimerSlot,
    /// ⭐ E-182：异步数据线的计数（**证据用**：解决几个 / 被接受几个 / 被驳回几个）。
    line_stats: Rc<LineStats>,
}

/// ⭐ E-182：**定时器闭包槽**（`Rc<RefCell<Vec<…>>>` 太长 ⇒ ⇒ 起名，避免 clippy 报「类型过于复杂」）。
type TimerSlot = Rc<RefCell<Vec<Closure<dyn FnMut()>>>>;

/// ⭐ E-182：异步数据线的计数（宿主侧的「账」⇒ ⇒ 证据读它）。
///
/// ## 为什么需要计数（而不是只看 DOM）
///
/// 「DOM 显示 beta」**不能**说明 alpha 被驳回了 —— 也可能是 alpha **根本没回来**。
/// ⇒ ⇒ 三个数一起看才成立：`resolved == 2`（两条都回来了）+ `rejected == 1`
/// （有一条被挡下）+ `accepted == 1`（只有一条上屏）。
#[derive(Debug, Default)]
struct LineStats {
    /// 已解决（回调已到）的请求数。
    resolved: Cell<u32>,
    /// 被接受（成功发布）的快照数。
    accepted: Cell<u32>,
    /// 被驳回（过期 ⇒ 不许覆盖新工作）的快照数。
    rejected: Cell<u32>,
    /// 还剩几次「慢查询」（模拟**冷缓存**：第一个请求慢 ⇒ ⇒ 后发的请求**先**回来）。
    slow_left: Cell<u32>,
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
    // ⭐⭐⭐ **E-182：屏幕与宿主必须共用同一份快照仓**
    //   ⇒ ⇒ 用 `snapshots().clone()` 拿（**共享**，不是各持一份）
    //   ⇒ ⇒⇒ 否则屏幕发的快照宿主**看不见** ⇒ ⇒ 机制形同虚设。
    let store = state.borrow().snapshots().clone();
    let screen: Rc<dyn Screen> = Rc::new(DemoScreen::new(&runtime, store));
    state.borrow_mut().switch_to(screen.id());

    let mut renderer = Renderer::new(dom, root);
    let view = screen.view();
    renderer
        .render_guarded(view)
        .map_err(|error| JsValue::from_str(&format!("宿主薄壳：首屏渲染失败：{error}")))?;
    state.borrow_mut().mark_rendered();
    let renderer = Rc::new(RefCell::new(renderer));

    // ⭐ **自动渲染**：effect 体里读屏幕的状态信号 ⇒ ⇒ 依赖被 core 追踪
    // ⇒ ⇒ 状态变 ⇒ effect 被标脏 ⇒ ⇒ **flush 时**重建视图 + patch DOM。
    // ⇒ ⇒ 宿主**不需要**手动「重建视图」—— 那正是自动调度的意义。
    let auto_render = {
        let screen = screen.clone();
        let state_for_effect = state.clone();
        runtime.effect(move || {
            let view = screen.view();
            // ⭐ **自动建立依赖**：遍历树里所有绑定并读一次 ⇒ ⇒ core 收集到依赖
            // ⇒ ⇒ 屏幕作者**不需要**手动 `signal.get()`（这正是 E-129 消除的易漏点）。
            view.touch_bindings();
            // ⭐⭐ **只标脏，不 patch**（批次边界收口）：
            //   core 的 effect 是**同步重跑**的 ⇒ ⇒ 一帧内 N 次变更 ⇒ N 次 effect
            //   ⇒ ⇒ 若在这里直接 patch ⇒ ⇒ **N 次渲染**（会抖）。
            //   ⇒ ⇒ 这里只**请求**渲染 ⇒ ⇒ 真正的 patch 由批次边界做**一次**。
            let _ = &view; // 树已构造（依赖已建立）；本批不 patch
            state_for_effect.borrow_mut().mark_dirty();
        })
    };

    let running = Rc::new(Cell::new(true));
    let frame_id = Rc::new(Cell::new(0i32));
    let frame_slot = Rc::new(FrameSlot {
        closure: RefCell::new(None),
    });
    // ⭐ E-182：异步数据线的接线（定时器槽 + 计数；**慢查询只剩 1 次** ⇒ 模拟冷缓存）。
    let timers = Rc::new(RefCell::new(Vec::new()));
    let line_stats = Rc::new(LineStats {
        slow_left: Cell::new(1),
        ..LineStats::default()
    });
    HOST.with(|slot| {
        *slot.borrow_mut() = Some(Host {
            renderer,
            state,
            runtime,
            screen,
            running: running.clone(),
            frame_id: frame_id.clone(),
            frame_slot: frame_slot.clone(),
            auto_render,
            timers,
            line_stats,
        });
    });

    // ⭐⭐ **帧驱动**：每帧跑一次**批次边界**（⇒ N 次状态变更 ⇒ 本帧只渲染 1 次）。
    //   ⇒ ⇒ 宿主**不需要**手动调 `flush` ⇒ 状态变 ⇒ 下一帧自动更新。
    start_frame_loop(running, frame_id, frame_slot);
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
        // ⭐⭐⭐ **渲染决策收口在 `HostState::render_plan`**（E-179）
        //   ⇒ ⇒ 「有效快照优先、否则标脏才渲染、否则不渲染」**一处**决定
        //   ⇒ ⇒ ⇒ 而**不是**散在 wasm 薄壳里 ⇒ ⇒⇒ 逻辑可 native 单测。
        //
        // ⭐⭐⭐ **为什么用「计划」而不是直接取视图**（实测教训，E-179）：
        //   若在**持有 `host.state` 借用**时调 `screen.view()` ⇒ ⇒ 视图构建
        //   ⇒ ⇒ 可能触发 flush ⇒ ⇒⇒ flush 里的 effect 会再借 `host.state`
        //   ⇒ ⇒⇒ ⇒ **RefCell 双重借用 panic** ⇒ ⇒⇒ ⇒ wasm 里表现为「unreachable」。
        //   ⇒ ⇒⇒ ⇒ 所以先取**计划**（借用在此结束）⇒ ⇒⇒ 再去取视图 ✓
        let channel = host.screen.channel();
        let plan = host.state.borrow_mut().render_plan(channel);
        let view = match plan {
            crate::RenderPlan::Snapshot(tree) => tree,
            crate::RenderPlan::ScreenView => host.screen.view(),
            crate::RenderPlan::Idle => return Ok(false),
        };
        host.renderer
            .borrow_mut()
            .patch(view)
            .map_err(|error| JsValue::from_str(&format!("宿主薄壳：重渲染失败：{error}")))?;
        host.state.borrow_mut().mark_rendered();
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
        // ⭐ 先停帧循环（`running` 标志 + **取消已排的那一帧**）⇒ ⇒ 循环立刻停止
        //   （不只是「下一帧起不再排」⇒ ⇒ 那一帧也不会再执行）。
        host.running.set(false);
        if let Some(window) = web_sys::window() {
            let _ = window.cancel_animation_frame(host.frame_id.get());
        }
        // ⭐ 释放帧闭包（槽被释放 ⇒ ⇒ 闭包被 drop ⇒ ⇒ JS 侧回调失效 ⇒ ⇒ **零残留**）。
        host.frame_slot.closure.borrow_mut().take();
        // ⭐ E-182：清空在飞的定时器（drop ⇒ ⇒ 回调失效 ⇒ ⇒ 不会再算完去发快照）
        //   ⇒ ⇒ 否则销毁后的异步结果会往一个**已销毁**的宿主里发快照（脏写）。
        host.timers.borrow_mut().clear();
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

/// ⭐ **帧槽**：持有那个唯一的帧回调闭包。
///
/// ## 为什么用 `Weak` 打破自引用（2026-10-31）
///
/// 帧回调要**自己排下一帧** ⇒ ⇒ 它必须能引用「装着自己的槽」
/// ⇒ ⇒ 若闭包**强引用**槽（`Rc`）⇒ ⇒ 槽 ⇒ 闭包 ⇒ 槽 ⇒ ⇒ **循环引用**
/// ⇒ ⇒ 引用计数**永远不归零** ⇒ ⇒ 每次 boot 残留一个闭包（E-132 的已知债）。
/// ⇒ ⇒ 改用 **`Weak`**：闭包只**弱**引用槽 ⇒ ⇒ 宿主释放槽 ⇒ ⇒ 弱引用失效
/// ⇒ ⇒ 闭包随之释放 ⇒ ⇒ **彻底回收**（不再有残留）。
pub struct FrameSlot {
    /// 帧回调闭包（宿主释放槽时它被 drop ⇒ JS 侧回调失效）。
    pub closure: RefCell<Option<FrameClosure>>,
}

/// 帧回调闭包的类型（每帧收到一个时间戳）。
type FrameClosure = Closure<dyn FnMut(f64)>;

/// ⭐⭐ **帧驱动循环**：每帧执行一次**批次边界渲染**。
///
/// ## 为什么需要它
///
/// core 的 effect 是**同步重跑**的 ⇒ ⇒ 状态变一次就标脏一次
/// ⇒ ⇒ 若**每次**都渲染 ⇒ ⇒ 一帧内 N 次变更 ⇒ N 次渲染（抖）
/// ⇒ ⇒ 帧循环把「渲染」收敛到**每帧一次** ⇒ ⇒ 一帧内 N 次变更 ⇒ **渲染 1 次**。
///
/// ## 停止
///
/// `dispose_screen` 会把 `running` 置 false ⇒ ⇒ 下一帧起**不再排下一帧** ⇒ ⇒ 循环自然停。
fn start_frame_loop(running: Rc<Cell<bool>>, frame_id: Rc<Cell<i32>>, slot: Rc<FrameSlot>) {
    let Some(window) = web_sys::window() else {
        return;
    };
    // ⭐⭐ **单一 Closure 自递归**（2026-10-31 修**真缺陷**）：
    //   修前：**每帧** `Closure::new` + `forget()` ⇒ ⇒ **每帧泄漏一个闭包 + 一个 JS 回调**
    //   （60 帧/秒 ⇒ 每分钟 3600 个 ⇒ 持续增长）⇒ ⇒ 那是**真缺陷**，不是一次性债。
    //   修后：**只建一个** Closure，它自己排下一帧 ⇒ ⇒ 泄漏降为「每次 boot 一个」。
    // ⭐ 闭包只**弱**引用槽 ⇒ ⇒ 宿主释放槽 ⇒ ⇒ 闭包随之释放 ⇒ ⇒ 无残留。
    let slot_for_closure = Rc::downgrade(&slot);
    let running_for_closure = running.clone();
    let frame_id_for_closure = frame_id.clone();
    let window_for_closure = window.clone();
    let closure = Closure::<dyn FnMut(f64)>::new(move |_timestamp| {
        if !running_for_closure.get() {
            return;
        }
        // 本帧的批次边界：把这一帧内积累的变更**合并成一次**渲染。
        let _ = render_if_dirty();
        // 排下一帧：**复用同一个 Closure**（不再每帧新建 ⇒ 不再每帧泄漏）。
        // 弱引用升级失败 ⇒ ⇒ 槽已被释放 ⇒ ⇒ 循环自然终止。
        let Some(slot) = slot_for_closure.upgrade() else {
            return;
        };
        let borrowed = slot.closure.borrow();
        if let Some(current) = borrowed.as_ref() {
            if let Ok(id) =
                window_for_closure.request_animation_frame(current.as_ref().unchecked_ref())
            {
                frame_id_for_closure.set(id);
            }
        }
    });
    *slot.closure.borrow_mut() = Some(closure);
    let borrowed = slot.closure.borrow();
    if let Some(current) = borrowed.as_ref() {
        if let Ok(id) = window.request_animation_frame(current.as_ref().unchecked_ref()) {
            frame_id.set(id);
        }
    }
    // ⭐ 无循环引用（闭包只持 `Weak`）⇒ ⇒ 宿主释放槽 ⇒ ⇒ 闭包被 drop
    //   ⇒ ⇒ **零残留**（E-132 的「每次 boot 泄漏一个」已消除）。
}

/// ⭐⭐ **批次边界渲染**：若被标脏 ⇒ **渲染一次**（并清脏 + 计数 +1）。
///
/// ## 为什么在批次边界而不是「变更时」
///
/// core 的 effect 是**同步重跑**的 ⇒ ⇒ 一帧内 N 次状态变更 ⇒ N 次 effect
/// ⇒ ⇒ 若每次都直接 patch ⇒ ⇒ **N 次渲染**（重排抖动、浪费）
/// ⇒ ⇒ 故：effect 只**标脏** ⇒ ⇒ 批次边界**合并成一次**渲染。
///
/// 返回 `true` = 本次真的渲染了。
#[wasm_bindgen]
pub fn render_if_dirty() -> Result<bool, JsValue> {
    HOST.with(|slot| {
        let mut guard = slot.borrow_mut();
        let Some(host) = guard.as_mut() else {
            return Err(JsValue::from_str("宿主薄壳：还没 boot"));
        };
        // ⭐⭐⭐ **渲染决策收口在 `HostState::render_plan`**（E-179）
        //   ⇒ ⇒ 「有效快照优先、否则标脏才渲染、否则不渲染」**一处**决定
        //   ⇒ ⇒ ⇒ 而**不是**散在 wasm 薄壳里 ⇒ ⇒⇒ 逻辑可 native 单测。
        //
        // ⭐⭐⭐ **为什么用「计划」而不是直接取视图**（实测教训，E-179）：
        //   若在**持有 `host.state` 借用**时调 `screen.view()` ⇒ ⇒ 视图构建
        //   ⇒ ⇒ 可能触发 flush ⇒ ⇒⇒ flush 里的 effect 会再借 `host.state`
        //   ⇒ ⇒⇒ ⇒ **RefCell 双重借用 panic** ⇒ ⇒⇒ ⇒ wasm 里表现为「unreachable」。
        //   ⇒ ⇒⇒ ⇒ 所以先取**计划**（借用在此结束）⇒ ⇒⇒ 再去取视图 ✓
        let channel = host.screen.channel();
        let plan = host.state.borrow_mut().render_plan(channel);
        let view = match plan {
            crate::RenderPlan::Snapshot(tree) => tree,
            crate::RenderPlan::ScreenView => host.screen.view(),
            crate::RenderPlan::Idle => return Ok(false),
        };
        host.renderer
            .borrow_mut()
            .patch(view)
            .map_err(|error| JsValue::from_str(&format!("宿主薄壳：重渲染失败：{error}")))?;
        host.state.borrow_mut().mark_rendered();
        Ok(true)
    })
}

/// ⭐ **已渲染次数**（用来观测「一帧内多次变更 ⇒ 渲染了几次」）。
#[wasm_bindgen]
pub fn render_count() -> Result<usize, JsValue> {
    HOST.with(|slot| {
        let guard = slot.borrow();
        let Some(host) = guard.as_ref() else {
            return Err(JsValue::from_str("宿主薄壳：还没 boot"));
        };
        let renders = host.state.borrow().renders();
        Ok(renders)
    })
}

/// ⭐ **flush**：跑完当前所有待办 + 执行**批次边界渲染**。
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
        // 批次边界：把 effect 请求的渲染**合并成一次**。
        // ⭐ 同上 ⇒ ⇒ 决策**一处**（不三份复制）⇒ ⇒ 且**先取计划**（避免重入）。
        let channel = host.screen.channel();
        let plan = host.state.borrow_mut().render_plan(channel);
        let rendered = if let Some(view) = match plan {
            crate::RenderPlan::Snapshot(tree) => Some(tree),
            crate::RenderPlan::ScreenView => Some(host.screen.view()),
            crate::RenderPlan::Idle => None,
        } {
            host.renderer
                .borrow_mut()
                .patch(view)
                .map_err(|error| JsValue::from_str(&format!("宿主薄壳：重渲染失败：{error}")))?;
            host.state.borrow_mut().mark_rendered();
            true
        } else {
            false
        };
        let _ = report;
        Ok(rendered)
    })
}

/// ⭐⭐⭐ **演示屏的树**（**唯一**的构建处 ⇒ ⇒ 屏幕路径与快照路径**必定一致**）。
///
/// ## 为什么抽成一个函数（E-182）
///
/// 异步数据线要发的是**整棵树**（快照**替换**整棵树，不是补丁）
/// ⇒ ⇒ 若屏幕自己建一棵、异步回调另建一棵 ⇒ ⇒ **两棵树会漂移**
/// （输入框的绑定、id、class 早晚对不上）⇒ ⇒ ⇒ 那些差异只会以「诡异的 DOM 行为」暴露。
/// ⇒ ⇒⇒ 故：**一处构建**、两条路径共用。
///
/// ⚠️ `text` / `query` 传的是**信号**（不是值）：输入框要**双向绑定**
/// ⇒ ⇒ 绑定必须拿到信号本身；`results` 传值（异步结果是一次性文本）。
fn demo_view(text: &Signal<String>, query: &Signal<String>, results: &str) -> VNode {
    use apeireth_ui_dom::binding::Binding;

    VNode::element("section")
        .attr("class", "app-screen")
        .child(
            VNode::element("input")
                .attr("id", "app-demo-input")
                .bind(Binding::text(text.clone()))
                .build(),
        )
        .child(
            VNode::element("input")
                .attr("id", "app-demo-query")
                .attr("placeholder", "异步查询")
                .bind(Binding::text(query.clone()))
                .build(),
        )
        .child(
            VNode::element("p")
                .attr("id", "app-demo-results")
                .child(VNode::text(results))
                .build(),
        )
        .build()
}

/// ⭐⭐ **演示屏幕**：受控输入框（信号 ⇄ DOM 双向绑定）+ **异步数据线**（E-182）。
///
/// ## 为什么内置演示屏
///
/// 它是**最小可运行**的宿主证据：信号变 ⇒ DOM 更新；用户输入 ⇒ 信号更新
/// ⇒ ⇒ 一眼就能看出「契约 → 引擎 → 真实 DOM」这条链通不通。
///
/// ## E-182：为什么要它带一条**异步数据线**
///
/// ⇒ ⇒ 宿主运行时里「旧回调不能覆盖新工作」这条**只有接了真异步才可能被观测**
/// ⇒ ⇒⇒ 而**参考宿主**是最合适的第一个消费者：它是**生产路径**（不是测试夹具）
/// ⇒ ⇒⇒⇒ 于是「快照仲裁」从「引擎有这能力」变成「**产品宿主在用它**」。
pub struct DemoScreen {
    /// 输入框的值（信号）。
    text: Signal<String>,
    /// ⭐ E-182：异步查询词（受控 ⇒ ⇒ 与输入框双向绑定）。
    query: Signal<String>,
    /// ⭐ E-182：**已公布（上屏）**的异步结果 ⇒ ⇒ 只有**过仲裁**的结果才写到这里。
    results: Signal<String>,
    /// ⭐ E-182：异步数据线（**与宿主共享**同一个快照仓）。
    line: AsyncLine,
}

impl DemoScreen {
    /// 建演示屏（`runtime` 提供信号与运行时；`store` 必须与宿主**共享**）。
    pub fn new(runtime: &Runtime, store: crate::snapshot::SnapshotStore) -> Self {
        Self {
            text: runtime.signal(String::from("宿主已就绪")),
            query: runtime.signal(String::new()),
            results: runtime.signal(String::from("（还没有异步结果）")),
            line: AsyncLine::new(store, DEMO_ASYNC_CHANNEL),
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

    /// ⭐ E-182：**声明通道** ⇒ ⇒ 宿主才会去取这条数据线的快照（否则永远走屏幕视图）。
    fn channel(&self) -> Option<&str> {
        Some(DEMO_ASYNC_CHANNEL)
    }

    fn view(&self) -> VNode {
        demo_view(&self.text, &self.query, &self.results.get())
    }
}

/// ⭐⭐⭐ **发起一次异步查询**（演示屏专用：真实定时器 + 真实仲裁）。
///
/// ## 它做什么
///
/// ① **领票**（`line.begin()`）⇒ ⇒ 拿到「本次查询」的代号；
/// ② **记下查询词**（受控输入 ⇒ ⇒ 屏幕显示它）；
/// ③ 排一个**真定时器**（`set_timeout` ⇒ ⇒ 真异步，不是同步假装）；
/// ④ 到点时算结果 ⇒ ⇒ **过仲裁**（`line.publish`）⇒ ⇒ **过期的一律丢掉**。
///
/// ## 为什么第一个请求慢（600ms）、之后快（150ms）
///
/// 为了**必然**造出**乱序**：先发「alpha」（慢）再发「beta」（快）
/// ⇒ ⇒ beta **先**回来 ⇒ ⇒ alpha **后**回来
/// ⇒ ⇒⇒ ⇒ 若没有仲裁 ⇒ alpha **会盖掉** beta（屏幕上出现过期结果）
/// ⇒ ⇒⇒ ⇒ **这就是要验的那件事**。
///
/// ## Errors
///
/// 未 boot / 当前屏幕不是演示屏 ⇒ 返回 JS 错误。
#[wasm_bindgen]
pub fn set_demo_query(query: &str) -> Result<(), JsValue> {
    HOST.with(|slot| {
        let guard = slot.borrow();
        let Some(host) = guard.as_ref() else {
            return Err(JsValue::from_str("宿主薄壳：还没 boot"));
        };
        let Some(demo) = host.screen.as_any().downcast_ref::<DemoScreen>() else {
            return Err(JsValue::from_str("宿主薄壳：当前屏幕不是演示屏"));
        };
        let Some(window) = web_sys::window() else {
            return Err(JsValue::from_str("宿主薄壳：没有 window"));
        };

        // ① 领票（**在请求发出之前**领）② 记下查询词③ 标记脏（输入框要显示新词）
        let ticket = demo.line.begin();
        demo.query.set(query.to_owned());
        host.state.borrow_mut().mark_dirty();

        // ④ 真定时器：第一个请求慢（冷缓存）⇒ ⇒ 后发先至
        let delay = if host.line_stats.slow_left.get() > 0 {
            host.line_stats
                .slow_left
                .set(host.line_stats.slow_left.get() - 1);
            600
        } else {
            150
        };

        // ⑤ 到点：算结果 ⇒ ⇒ **过仲裁** ⇒ ⇒ 过期的一律丢掉（不覆盖新工作）
        let text = demo.text.clone();
        let query_signal = demo.query.clone();
        let results = demo.results.clone();
        let line = demo.line.clone();
        let state = host.state.clone();
        let stats = host.line_stats.clone();
        let query_owned = query.to_owned();
        let closure = Closure::<dyn FnMut()>::new(move || {
            // ⭐ 诊断：回调入口打标记（若在下面某步冻住 ⇒ ⇒ 报告里能看到「卡在哪个回调」）
            let stage_key = JsValue::from_str("__P5B_ASYNC_STAGE__");
            if let Some(window) = web_sys::window() {
                let _ = js_sys::Reflect::set(
                    &window,
                    &stage_key,
                    &JsValue::from_str(&format!("cb-enter:{query_owned}")),
                );
            }
            let fetched = format!("{query_owned} 的结果");
            stats.resolved.set(stats.resolved.get() + 1);
            let tree = demo_view(&text, &query_signal, &fetched);
            // ⚠️ 票据 `clone()`：闭包要能**被重复调用**（`FnMut`）
            //   ⇒ ⇒ 把捕获的票**移出**会让闭包退化成 `FnOnce` ⇒ ⇒ 编译直接红。
            //   ⇒ ⇒ 票据是**值语义凭证**（`Clone`）⇒ ⇒ 克隆一份传进去，语义不变。
            if line.publish(tree, ticket.clone()) {
                // ⭐ 被接受：只有**最新**的那条能走到这里
                stats.accepted.set(stats.accepted.get() + 1);
                results.set(fetched);
                state.borrow_mut().mark_dirty();
            } else {
                // ⭐⭐ 被驳回：**过期结果** ⇒ ⇒ 不许覆盖新工作（这就是仲裁的意义）
                stats.rejected.set(stats.rejected.get() + 1);
            }
            if let Some(window) = web_sys::window() {
                let _ = js_sys::Reflect::set(
                    &window,
                    &stage_key,
                    &JsValue::from_str(&format!("cb-done:{query_owned}")),
                );
            }
        });
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
            closure.as_ref().unchecked_ref(),
            delay,
        );
        // 必须保活到触发（drop ⇒ ⇒ 回调立刻失效 ⇒ ⇒ 定时器永不到点）。
        host.timers.borrow_mut().push(closure);
        Ok(())
    })
}

/// ⭐⭐ **异步数据线的计数**（证据读它 ⇒ ⇒ 光看 DOM 不够）。
///
/// ## 为什么需要它
///
/// 「DOM 显示 beta」**不能**证明 alpha 被驳回了 —— 也可能 alpha **根本没回来**。
/// ⇒ ⇒ 三个数一起看才成立：`resolved == 2`（两条都回来了）
/// + `accepted == 1`（只上屏一条）+ `rejected == 1`（有一条被挡下）。
///
/// # Errors
///
/// 未 boot ⇒ 返回 JS 错误。
#[wasm_bindgen]
pub fn async_line_stats() -> Result<JsValue, JsValue> {
    HOST.with(|slot| {
        let guard = slot.borrow();
        let Some(host) = guard.as_ref() else {
            return Err(JsValue::from_str("宿主薄壳：还没 boot"));
        };
        let object = js_sys::Object::new();
        let stats = &host.line_stats;
        for (key, value) in [
            ("resolved", stats.resolved.get()),
            ("accepted", stats.accepted.get()),
            ("rejected", stats.rejected.get()),
        ] {
            js_sys::Reflect::set(
                &object,
                &JsValue::from_str(key),
                &JsValue::from_f64(f64::from(value)),
            )?;
        }
        Ok(object.into())
    })
}
