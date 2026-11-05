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

/// ⭐⭐⭐ **异步数据线**（E-182）—— 「领票 / 发布 / 作废」的**参考接线**。
///
/// ⇒ ⇒ 宿主只管**算树**，**不碰票据** ⇒ ⇒⇒ 漏接的可能最小 ⇒ ⇒⇒⇒
/// ⇒ ⇒ 它与 [`HostState`] **共享**同一份 [`snapshot::SnapshotStore`] ⇒ ⇒⇒
/// ⇒ ⇒⇒ 屏幕**发**快照、宿主**取**快照 ⇒ ⇒⇒ ⇒ 机制才成立。
pub mod async_line;

/// ⭐⭐⭐ **视图快照**（application-layer 地基，E-178）—— 「旧回调不能覆盖新工作」。
pub mod snapshot;

/// ⭐ **wasm 薄壳**（接真实 DOM 的入口）—— **仅 wasm32 目标编译**。
///
/// ⇒ native 侧**不参与编译** ⇒ ⇒ 契约层永远可 native 单测（与 core 同纪律）。
#[cfg(target_arch = "wasm32")]
pub mod wasm_host;

/// ⭐⭐⭐⭐ **渲染计划**（E-179）：宿主每帧的**决策结果**。
///
/// ## 为什么是「计划」而不是「树」
///
/// ⇒ ⇒ 决策**不能**在持有 `HostState` 借用时去调 `screen.view()`
/// ⇒ ⇒⇒ （重入会触发 flush ⇒ ⇒⇒ effect 里再借 `HostState` ⇒ ⇒⇒⇒ 双重借用 panic）
/// ⇒ ⇒⇒ ⇒ 所以决策只**说明「该渲染什么」**，**不代劳**取视图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderPlan {
    /// 渲染这棵树（来自**有效快照** ⇒ ⇒ 异步结果比屏幕视图更新鲜）。
    Snapshot(VNode),
    /// 渲染**屏幕当前视图**（由调用方在**借用释放之后**去取）。
    ScreenView,
    /// ⭐ **本帧不渲染**（保留上一帧 DOM）。
    Idle,
}

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

    /// ⭐⭐⭐ 本屏的**异步通道名**（`None` ⇒ ⇒ 本屏没有异步数据 ⇒ ⇒ 宿主不查快照）。
    ///
    /// ## 为什么需要它
    ///
    /// ⇒ ⇒ 宿主渲染前要问「这一屏有没有**待渲染的异步结果**」
    /// ⇒ ⇒⇒ 而「有没有」是**本屏**的事 ⇒ ⇒⇒ ⇒ 不能由宿主猜。
    ///
    /// ## 默认
    ///
    /// `None` ⇒ ⇒ 绝大多数屏幕（纯本地状态）**不需要**任何改动 ✓
    /// ⇒ ⇒⇒ ⇒ 这是一个**默认实现**的加法 ⇒ ⇒⇒ 不破坏既有实现。
    fn channel(&self) -> Option<&str> {
        None
    }
}

/// ⭐ **宿主状态**：当前在哪个屏幕、要不要重渲染。
///
/// ## ⚠️ 「一次变更只触发一次重渲染」的**准确边界**（2026-10-31 更正）
///
/// 本类型把**多次标脏合并成一次 `take_dirty`** ⇒ ⇒ **在这一层**是对的
/// ⇒ ⇒ 但 **端到端不成立**：core 的 effect 是**同步重跑**的（`signal.set` ⇒ effect 立即跑）
/// ⇒ ⇒ 状态变一次 ⇒ DOM 就重渲染一次 ⇒ ⇒ 「帧边界只渲染一次」需要**上层显式批处理**
/// ⇒ ⇒ **不是**本类型自动提供的。
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
    /// ⭐ 已渲染次数（**批次边界**的实际渲染次数 ⇒ ⇒ 用来观测「一帧多次变更 ⇒ 渲染几次」）。
    renders: usize,
    /// ⭐⭐⭐ **快照仓**（E-179）⇒ ⇒ 宿主与屏幕**共用**同一份。
    ///
    /// ## 为什么宿主要持有它
    ///
    /// ⇒ ⇒ 屏幕**发**快照（异步结果算好的视图）⇒ ⇒ 宿主**取**快照去渲染
    /// ⇒ ⇒⇒ 两者必须看到**同一份**状态 ⇒ ⇒⇒ ⇒ 而 `SnapshotStore` 的克隆是**共享**的
    /// ⇒ ⇒⇒ ⇒ 所以宿主与屏幕各持一份克隆即可 ⇒ ⇒⇒ ⇒ 不必引入 `Rc<RefCell<…>>` 的额外层。
    snapshots: crate::snapshot::SnapshotStore,
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

    /// ⭐ 取走「脏」标记（**本层**合并多次标脏 ⇒ 只回报一次「需要重渲染」）。
    ///
    /// 返回 `true` = 需要重渲染；`false` = 本帧无需重渲染。
    pub fn take_dirty(&mut self) -> bool {
        std::mem::replace(&mut self.dirty, false)
    }

    /// ⭐ 记一次**实际渲染**（批次边界真正 patch 了才调）。
    ///
    /// ⇒ 与「标脏次数」分开 ⇒ ⇒ 才能观测「多次变更 ⇒ 渲染**几次**」。
    pub fn mark_rendered(&mut self) {
        self.renders += 1;
    }

    /// 已渲染次数。
    pub fn renders(&self) -> usize {
        self.renders
    }

    /// ⭐⭐⭐ **快照仓**（与屏幕**共享**同一份）⇒ ⇒ 屏幕用它发布、宿主用它取用。
    pub fn snapshots(&self) -> &crate::snapshot::SnapshotStore {
        &self.snapshots
    }

    /// ⭐⭐⭐⭐ **本帧渲染什么**（**仲裁后**的决策，E-179）。
    ///
    /// ## ⭐⭐⭐ 为什么返回「计划」而不是「直接给出树」
    ///
    /// ⇒ ⇒ 早先的实现是 `render_target(channel, || screen.view())`
    /// ⇒ ⇒⇒ **闭包在借用未释放时被调用** ⇒ ⇒⇒ 若 `screen.view()` 触发 flush
    /// ⇒ ⇒⇒ ⇒ flush 里的 effect 会 `host.state.borrow_mut()` ⇒ ⇒⇒⇒
    /// ⇒ ⇒⇒⇒ **RefCell 双重借用 panic** ⇒ ⇒⇒ ⇒ wasm 里表现为「unreachable」。
    ///
    /// ⇒ ⇒⇒ ⇒ 所以决策**只产出计划**，**不代劳**取视图 ⇒ ⇒⇒
    /// ⇒ ⇒⇒ ⇒ 调用方在**借用释放之后**再去取视图 ⇒ ⇒⇒ ⇒ 重入**不可能**。
    ///
    /// ## 口径（三条，**按顺序**）
    ///
    /// ① 若该通道有**有效**快照 ⇒ ⇒ [`RenderPlan::Snapshot`]（它比屏幕视图**更新鲜**）
    ///    ⇒ ⇒ ⇒ 并**消费掉**该快照（取出即消费 ⇒ ⇒⇒ 不会渲染两次）；
    /// ② 否则若**标脏** ⇒ ⇒ [`RenderPlan::ScreenView`]；
    /// ③ 否则 ⇒ ⇒ [`RenderPlan::Idle`] ⇒ ⇒ **本帧不渲染**（保留上一帧 DOM）。
    ///
    /// ## ⚠️ 「标脏」在选了快照时**不**被消费
    ///
    /// ⇒ ⇒ 快照只覆盖「这个通道的那份数据」⇒ ⇒⇒ 屏幕自身的其它状态变更**仍需**渲染
    /// ⇒ ⇒⇒ ⇒ 所以**保留**脏标记 ⇒ ⇒⇒ ⇒ 下一帧会渲染屏幕视图 ⇒ ⇒⇒ ⇒ 不丢状态。
    pub fn render_plan(&mut self, channel: Option<&str>) -> RenderPlan {
        // ① 有效快照优先。
        if let Some(channel) = channel {
            if let Some(snapshot) = self.snapshots.take_current(channel) {
                return RenderPlan::Snapshot(snapshot.tree);
            }
        }
        // ② 标脏 ⇒ ⇒ 渲染屏幕视图。
        if self.take_dirty() {
            return RenderPlan::ScreenView;
        }
        // ③ 本帧不渲染（保留上一帧 DOM）。
        RenderPlan::Idle
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
///
/// ## ⭐⭐ 为什么 `source` 是**泛型**（E-180）
///
/// 早先这个签名写死 `&Signal<String>` ⇒ ⇒ 而真实的屏幕状态**什么类型都有**
/// （`bool` / `Vec<String>` / 自定义结构）⇒ ⇒⇒ 那些屏幕**接不上**
/// ⇒ ⇒⇒ 只能自己在宿主里**抄一遍**这段逻辑 ⇒ ⇒⇒ ⇒ 抄一份就多一处「忘了接」的坑。
/// ⇒ ⇒ ⇒ 泛型化后**一处**逻辑覆盖**所有**信号类型 ⇒ ⇒⇒ 宿主不必再抄。
///
/// ## ⭐⭐⭐ 为什么**返回 `Result`**（E-180：把静默失效变成显式报错）
///
/// 「信号属于哪个运行时」决定依赖边**能不能**建立。跨运行时的信号写它
/// **不会**触发任何重渲染，现象是「状态变了界面不动」，且**全程无任何报错** ——
/// 这正是本 crate 文档里记过的「**幽灵输入**」。
///
/// 类型层（`ReactiveScope`）已经挡住「状态持有者自建运行时」那条路，但
/// `Runtime::new` 仍是 `pub`（native 测试与独立运行时的需要），所以接线处
/// 必须**当场复核归属**：复核不过就返回
/// [`apeireth_ui_core::ReactiveError::ForeignSignal`]，而不是「接上了、
/// 其实什么都不会发生」。
///
/// 判据：`runtime.owns(host)` 且 `runtime.owns(source)`（**指针身份**）。
pub fn wire_signal_to_host<T: Clone + 'static>(
    runtime: &apeireth_ui_core::Runtime,
    host: &Signal<HostState>,
    source: &Signal<T>,
) -> Result<apeireth_ui_core::EffectHandle, apeireth_ui_core::ReactiveError> {
    // ⭐ 守卫：宿主状态与被接的信号必须**同属这一个运行时**。
    // 错位时**当场**报错，而不是「接上了、其实什么都不会发生」。
    if !runtime.owns(host) {
        return Err(apeireth_ui_core::ReactiveError::ForeignSignal { node: host.id() });
    }
    if !runtime.owns(source) {
        return Err(apeireth_ui_core::ReactiveError::ForeignSignal { node: source.id() });
    }
    // ⭐ core 的 effect 是**依赖追踪式**的：闭包里**读了哪个 signal** ⇒ 就订阅哪个
    // ⇒ ⇒ 这里读 `source` ⇒ 订阅它 ⇒ 它变了 ⇒ 本 effect 重跑 ⇒ 宿主标脏。
    let source = source.clone();
    let host = host.clone();
    Ok(runtime.effect(move || {
        let _ = source.get(); // ← 这一行**就是**订阅（依赖收集点）
        host.update(|mut state| {
            state.mark_dirty();
            state
        });
    }))
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

    /// 多次标脏 ⇒ **本层**只回报一次（⚠️ 不等于端到端只渲染一次 —— 见类型文档）。
    #[test]
    fn many_marks_collapse_into_one_render() {
        let mut state = HostState::new();
        for _ in 0..5 {
            state.mark_dirty();
        }
        assert!(state.take_dirty());
        assert!(!state.take_dirty(), "5 次标脏 ⇒ 帧边界只重渲染 1 次");
    }

    /// ⭐⭐ **批次语义**：多次变更 ⇒ 批次边界**只渲染一次**。
    ///
    /// 这是「上层显式批处理」的核心口径：
    /// - N 次标脏 ⇒ 脏标记仍是**一个**（合并）；
    /// - 批次边界渲染**一次** ⇒ 计数 +1；
    /// - 渲染后不再脏 ⇒ 再判定时**不**该再渲染（幂等）。
    #[test]
    fn batch_renders_once_regardless_of_change_count() {
        let mut state = HostState::new();
        state.switch_to("settings");
        for _ in 0..5 {
            state.mark_dirty();
        }
        assert!(state.is_dirty(), "多次变更后必须脏");
        // 批次边界：渲染一次
        state.mark_rendered();
        state.take_dirty();
        assert_eq!(state.renders(), 1, "5 次变更 ⇒ 只该渲染 1 次");
        assert!(!state.is_dirty(), "渲染后不再脏");
        // 幂等：再判定时不该再渲染
        assert!(!state.take_dirty(), "渲染后不得重复渲染");
        assert_eq!(state.renders(), 1, "渲染次数不得增加");
    }

    /// ⭐ 收集树里的**全部文本**（测试用；`VNode` 没有内置的文本访问器）。
    fn text_of(node: &VNode) -> String {
        match node {
            VNode::Text(text) => text.data.clone(),
            _ => node.children().iter().map(text_of).collect(),
        }
    }

    /// ⭐⭐⭐⭐ **宿主真的渲染「有效快照」**（真接线的证明，E-179）。
    ///
    /// ## 它锁住什么
    ///
    /// ⇒ ⇒ 前面的测试只验到「`render_target` 返回什么」
    /// ⇒ ⇒⇒ 而这一条验的是**整条链**：屏幕**发**快照 ⇒ ⇒ 宿主**取** ⇒ ⇒ **渲染它**
    /// ⇒ ⇒⇒ 且**过期**的那份**不得**上屏。
    ///
    /// ## 为什么这条重要
    ///
    /// ⇒ ⇒ 「有 API」≠「真在跑」⇒ ⇒⇒ 宿主若没接上去 ⇒ ⇒⇒ 机制形同虚设
    /// ⇒ ⇒⇒ ⇒ 而这种「写了但没接线」**最难查**（代码都在、测试都过、就是不生效）。
    #[test]
    fn the_host_really_renders_a_valid_snapshot() {
        use crate::snapshot::SnapshotStore;

        // ① 一个「异步屏幕」：它持有**共享**的快照仓（与宿主同一份）。
        struct AsyncScreen {
            store: SnapshotStore,
            view_text: String,
        }
        impl Screen for AsyncScreen {
            fn id(&self) -> &str {
                "async-screen"
            }
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
            fn view(&self) -> VNode {
                VNode::element("div")
                    .child(VNode::text(self.view_text.clone()))
                    .build()
            }
            fn channel(&self) -> Option<&str> {
                Some("async")
            }
        }

        let mut host = HostState::new();
        // ② 屏幕与宿主**共用**同一个仓（克隆是共享的 ⇒ ⇒ 看到同一份）。
        let screen_store = host.snapshots().clone();
        let screen = AsyncScreen {
            store: screen_store.clone(),
            view_text: String::from("本地视图"),
        };

        // ③ 屏幕发了一份**有效**快照（异步结果）。
        let ticket = screen.store.begin("async");
        let published = screen.store.publish(
            "async",
            crate::snapshot::Snapshot::new(
                VNode::element("div").child(VNode::text("异步结果")).build(),
                ticket,
            ),
        );
        assert!(published, "⭐ 有效快照必须被收下");

        // ④ 宿主渲染 ⇒ ⇒ **渲染的是快照**（不是本地视图）。
        let plan = host.render_plan(screen.channel());
        let tree = match plan {
            RenderPlan::Snapshot(tree) => tree,
            other => panic!("⭐ 应当选快照，实际是 {other:?}"),
        };
        assert!(
            text_of(&tree).contains("异步结果"),
            "⭐ 宿主必须渲染**快照**（异步结果比本地视图更新鲜）"
        );

        // ⑤ 再取 ⇒ ⇒ 快照**已消费** ⇒ ⇒ 不再渲染它。
        assert_eq!(
            host.render_plan(screen.channel()),
            RenderPlan::Idle,
            "⭐ 快照取出即消费（不会渲染两次）"
        );
    }

    /// ⭐⭐⭐⭐ **过期快照上不了屏**（真接线的证明，E-179）。
    #[test]
    fn a_stale_snapshot_never_reaches_the_host_render() {
        use crate::snapshot::Snapshot;

        let mut host = HostState::new();
        let store = host.snapshots().clone();
        // ① 旧工作 ⇒ ⇒ 旧票据。
        let stale = store.begin("async");
        // ② 新工作开始 ⇒ ⇒ 旧票据过期。
        let _fresh = store.begin("async");
        // ③ 旧结果回来 ⇒ ⇒ **发不出去**。
        assert!(
            !store.publish(
                "async",
                Snapshot::new(
                    VNode::element("div").child(VNode::text("旧结果")).build(),
                    stale,
                ),
            ),
            "⭐ 过期快照必须被拒收"
        );
        // ④ 宿主渲染 ⇒ ⇒ **不渲染**它（没有快照 ⇒ ⇒ 且没标脏 ⇒ ⇒ 本帧不渲染）。
        assert_eq!(
            host.render_plan(Some("async")),
            RenderPlan::Idle,
            "⭐ 过期快照**不得**触发渲染"
        );
    }

    /// ⭐⭐⭐ **标脏 ⇒ ⇒ 渲染屏幕视图**（无快照时的老路不变）。
    #[test]
    fn a_dirty_host_renders_the_screen_view() {
        let mut host = HostState::new();
        host.switch_to("plain");
        assert_eq!(
            host.render_plan(None),
            RenderPlan::ScreenView,
            "⭐ 标脏时必须渲染屏幕视图（老路不变）"
        );
    }

    /// ⭐⭐⭐ **没标脏且无快照 ⇒ ⇒ 本帧不渲染**（保留上一帧 DOM）。
    #[test]
    fn a_clean_host_does_not_render() {
        let mut host = HostState::new();
        assert_eq!(
            host.render_plan(None),
            RenderPlan::Idle,
            "⭐ 没标脏且无快照 ⇒ ⇒ 不渲染"
        );
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

    /// ⭐⭐⭐ **信号变了 ⇒ ⇒ 宿主标脏**（E-180：这条链此前**零测试**）。
    ///
    /// ## 为什么这条必须钉住
    ///
    /// 「状态变了界面不动」是**最难查**的一类 bug ⇒ ⇒ 而它的成败**全在这根线上**
    /// ⇒ ⇒ 若哪天这根线断了 ⇒ ⇒⇒ 现象是「界面静默不动」⇒ ⇒⇒ 没有任何报错。
    ///
    /// ## ⭐ 为什么用 `Signal<bool>`（而不是 `String`）
    ///
    /// 早先签名写死 `Signal<String>` ⇒ ⇒ 真实屏幕的状态**什么类型都有**
    /// ⇒ ⇒⇒ E-180 把它**泛型化** ⇒ ⇒⇒ 这条测试**顺带**证明泛型那一路也通。
    ///
    /// ## ⭐ 为什么每步都要 `flush()`
    ///
    /// core 的 effect 是**延迟执行**的（「渲染后执行」）⇒ ⇒ ⇒ 它**不**在
    /// `set` 的那一刻跑 ⇒ ⇒⇒ 而是在宿主调度器驱动的那次 `flush` 里跑
    /// ⇒ ⇒⇒ 所以「信号变了 ⇒ ⇒ 宿主标脏」这条链**跨一次 flush**才成立
    /// ⇒ ⇒⇒ ⇒ 不 flush 就断言 ⇒ ⇒⇒ 会得到「没标脏」的**假阴性**。
    #[test]
    fn a_signal_change_marks_the_host_dirty() {
        let runtime = apeireth_ui_core::Runtime::new();
        // ⭐⭐ 信号必须**建在同一个 runtime 上**（`runtime.signal`）⇒ ⇒ 自由函数
        // `apeireth_ui_core::signal(...)` 落的是**线程局部默认运行时** ⇒ ⇒⇒ 那是
        // **另一张图** ⇒ ⇒⇒ 依赖边根本不会建立 ⇒ ⇒⇒ 「信号变了宿主不脏」会**假红**。
        let host = runtime.signal(HostState::new());
        let flag = runtime.signal(false);
        // ⭐ 保留 handle ⇒ ⇒ 否则效果会被立刻撤销（见 `core::effect` 的约定）。
        let _handle =
            wire_signal_to_host(&runtime, &host, &flag).expect("同属一个运行时的接线不得失败");
        // ⭐ effect 的**首次**执行发生在 flush ⇒ ⇒ flush 后宿主才脏（首帧要渲染）
        runtime.flush();
        assert!(
            host.get().is_dirty(),
            "⭐ 接线并 flush 后必须先脏一次（首帧要渲染）"
        );
        // 清脏 ⇒ ⇒ 只看「**变化之后**」的效果
        host.update(|mut state| {
            state.take_dirty();
            state
        });
        assert!(!host.get().is_dirty());
        // ⭐⭐ 改信号 ⇒ ⇒ flush ⇒ ⇒ 宿主必须**再次**标脏
        flag.set(true);
        runtime.flush();
        assert!(
            host.get().is_dirty(),
            "⭐ 信号变了 ⇒ ⇒ 宿主必须标脏 ⇒ ⇒ 否则界面静默不动"
        );
    }

    /// ⭐⭐⭐ **错位接线必须当场报错**（E-180：把「幽灵输入」从静默变成显式）。
    ///
    /// ## 为什么这条必须钉住
    ///
    /// 上一条测试写出来时**当场踩中**了这个坑：信号用**自由函数**
    /// `apeireth_ui_core::signal(...)` 建（落线程局部**默认运行时**），
    /// effect 却挂在另一个 `Runtime::new()` 上 ⇒ ⇒⇒ 那是**两张图**
    /// ⇒ ⇒⇒ 依赖边根本不会建立 ⇒ ⇒⇒ 「信号变了宿主不脏」**没有任何报错**。
    ///
    /// ⇒ ⇒ 所以本条锁住：**错位必须显式报错**，不许「接上了、其实什么都不会发生」。
    #[test]
    fn a_signal_from_another_runtime_is_rejected() {
        let runtime = apeireth_ui_core::Runtime::new();
        let other = apeireth_ui_core::Runtime::new();
        let host = runtime.signal(HostState::new());
        // 这个信号属于**另一个**运行时 ⇒ ⇒ 依赖边永远建不起来。
        let alien = other.signal(false);
        let err = match wire_signal_to_host(&runtime, &host, &alien) {
            Ok(_) => panic!("跨运行时的信号不得被接上"),
            Err(err) => err,
        };
        assert_eq!(
            err,
            apeireth_ui_core::ReactiveError::ForeignSignal { node: alien.id() },
            "⭐ 必须报「信号不属于该运行时」⇒ ⇒ 且点名是**哪一个**节点"
        );
        // 宿主状态本身来自别的运行时 ⇒ ⇒ 同样当场报错。
        let alien_host = other.signal(HostState::new());
        let err = match wire_signal_to_host(&runtime, &alien_host, &host) {
            Ok(_) => panic!("跨运行时的宿主状态不得被接上"),
            Err(err) => err,
        };
        assert_eq!(
            err,
            apeireth_ui_core::ReactiveError::ForeignSignal {
                node: alien_host.id()
            }
        );
    }
}
