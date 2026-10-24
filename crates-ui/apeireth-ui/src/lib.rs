//! # apeireth-ui — 门面（re-export 各层）
//!
//! 职责边界（设计文档 §3.3.6）：**零逻辑**。门面里出现任何一行非 `pub use` 的
//! 实现代码即违规 —— 门面只是「让试点应用不必写 5 条 `use`」。
//!
//! ## 各层与拓扑秩（ADR-0015 §3.3，单向下行无环）
//! | 层 | 拓扑秩 | 门面里的名字 |
//! |---|:--:|---|
//! | 响应式核心（信号 / 派生 / 依赖图 / 调度 / effect） | 1 | [`core`] |
//! | DOM 绑定（虚拟节点 / 差分 / 委托 / 绑定 / 渲染器） | 2 | [`dom`] |
//! | GPU 桥（状态 → 像素） | 2 | [`gpu`] |
//! | 组件库（共享组件 + 注册表） | 3 | [`kit`] |
//! | 过程宏（`#[component]` / `view!`） | 0 | [`component`] / [`view`] |
//!
//! 常用类型同时在门面根部具名导出（`Signal` / `VNode` / `UiComponent` …），
//! 试点应用通常只需要 `use apeireth_ui::…;`。
//!
//! ## 口径说明
//!
//! - **宏的运行时路径约定**：`#[component]` 与 `view!` 的展开物引用
//!   `::apeireth_ui_core::…` / `::apeireth_ui_dom::…`（**绝对路径**）⇒
//!   使用这两个宏的 crate 须把 `apeireth-ui-core` / `apeireth-ui-dom` 列为
//!   **直接依赖**（门面本身不参与展开物的路径解析）。
//!
//! - 真实 DOM 后端（`WebDom`）只在 **wasm32 目标**存在 ⇒ 门面的具名导出也
//!   同样按目标条件化（native 编译时**没有**这个名字，不是「导出了但不能用」）。
//! - `BindingSlot::property_name()` 返回 `Option<&'static str>`：`bind:this`
//!   不是 property 绑定 ⇒ `None`（P1 起的口径）。

#![forbid(unsafe_code)]

pub use apeireth_ui_core as core;
pub use apeireth_ui_dom as dom;
pub use apeireth_ui_gpu as gpu;
pub use apeireth_ui_kit as kit;
pub use apeireth_ui_macro::{component, view};

pub use apeireth_ui_core::{
    Callback, Derived, EffectHandle, FlushReport, HostScheduler, InstanceRef, NodeId,
    QueuedScheduler, ReactiveError, ReadSignal, Runtime, Scope, Signal, SyncScheduler, UiComponent,
};
pub use apeireth_ui_dom::{
    diff, diff_keyed, list_diff_mode, AttrValue, Binding, BindingSlot, Children, Dom, DomError,
    DomEvent, DomOp, ElementBuilder, ElementHandler, ElementVNode, EventHandler, EventKind,
    EventRegistry, HandlerId, InMemoryDom, InMemoryNodeId, Key, KeyedDiff, KeyedDiffError,
    KeyedSlot, ListDiffMode, MountedElement, MountedFragment, MountedNode, MountedText, Patch,
    RecordedOp, Renderer, TextVNode, ThisBinding, ThisRef, UnavailableDom, UnavailableNode, VNode,
};
pub use apeireth_ui_gpu::{
    BackendPreference, BindingError, BlackHoleParams, ColorProfile, Engine, EngineOptions,
    EngineReport, FrameReport, GlslError, GpuContext, GpuError, OutputEncoding, SceneDescriptor,
    SceneError, ScenePipeline, SceneUniforms, Tier, UnavailableGpuContext, UniformBufferLayout,
    UniformSlot, UniformValues, SHADER_UNIFORM_NAMES,
};
pub use apeireth_ui_kit::{
    Badge, BadgeProps, Button, ButtonProps, CardShell, CardShellProps, ComponentRegistry, Icon,
    IconData, IconNode, IconProps, RegistryError,
};

/// 真实 DOM 后端（web-sys）—— **仅 wasm32 目标**。
///
/// native 编译时该名字**不存在**（不是「导出了但不能用」）。
#[cfg(target_arch = "wasm32")]
pub use apeireth_ui_dom::WebDom;

#[cfg(test)]
mod tests {
    //! crate 级断言：门面只做转发 ⇒ 断言「各层可达 + 跨层类型一致」。

    use crate::core::HostScheduler as _;
    use crate::core::Runtime;
    use crate::dom::{Patch, VNode};
    use crate::gpu::{GpuContext as _, Tier};
    use crate::kit::ComponentRegistry;
    use crate::Dom as _;

    #[test]
    fn every_layer_is_reachable_from_the_facade() {
        let runtime = Runtime::new();
        let signal = runtime.signal(1u32);
        let view = signal.read_only();
        let derived = runtime.derived(move || view.get());
        let node = VNode::element("div").build();
        let registry = ComponentRegistry::new();

        assert_eq!(derived.get(), 1);
        assert_eq!(node.tag(), Some("div"));
        assert!(registry.is_empty());
        assert!(!Tier::Software.can_render());
        assert!(matches!(
            Patch::RemoveChildren {
                path: vec![],
                from: 0,
                count: 1
            },
            Patch::RemoveChildren { .. }
        ));
    }

    /// P1 遗留尾巴：响应式核心的新增类型（宿主调度器 / 实例引用）必须能从
    /// 门面取到。
    #[test]
    fn p1_core_types_are_reachable_from_the_facade() {
        let scheduler = crate::SyncScheduler;
        let ran = std::rc::Rc::new(std::cell::Cell::new(false));
        let flag = ran.clone();
        scheduler.schedule_flush(Box::new(move || flag.set(true)));
        assert!(ran.get(), "同步调度器必须当场执行");

        let slot: crate::InstanceRef<u32> = crate::InstanceRef::new();
        assert!(!slot.is_bound());
        slot.bind(7);
        assert_eq!(slot.get(), Some(7));

        let queued = crate::QueuedScheduler::new();
        assert!(queued.is_empty());
    }

    /// P1 遗留尾巴：DOM 绑定层的新增类型（keyed 差分 / `bind:this` / 未接线
    /// 后端）必须能从门面取到。
    #[test]
    fn p1_dom_types_are_reachable_from_the_facade() {
        let old = vec![
            VNode::element("li").key("a").build(),
            VNode::element("li").key("b").build(),
        ];
        let new = vec![
            VNode::element("li").key("b").build(),
            VNode::element("li").key("a").build(),
        ];
        let plan = crate::diff_keyed(&old, &new).expect("差分必须成功");
        assert_eq!(plan.mode(), crate::ListDiffMode::Keyed);
        assert_eq!(plan.move_count(), 1);

        let binding: crate::ThisBinding<crate::InMemoryDom> = crate::ThisBinding::new();
        assert!(!binding.is_bound());

        // 未接线后端：每个操作都显式报错（不许静默降级）。
        let unavailable = crate::UnavailableDom;
        assert!(unavailable.create_element("div").is_err());
    }

    /// P2 新增类型（渲染器 / 操作序列 / 擦除引用）必须能从门面取到。
    #[test]
    fn p2_types_are_reachable_from_the_facade() {
        let dom = crate::InMemoryDom::new();
        let root = dom.create_element("main").expect("建节点必须成功");
        let mut renderer = crate::Renderer::new(dom, root);
        renderer
            .mount(VNode::element("div").class("ap-a").build())
            .expect("挂载必须成功");
        assert!(renderer.is_mounted());
        assert_eq!(renderer.bound_slots(), 0);
        assert!(!renderer.dom().ops().is_empty(), "操作序列必须可见");

        let reference: crate::ThisRef = crate::ThisBinding::<crate::InMemoryDom>::new().erased();
        assert_eq!(reference, reference.clone(), "同一槽位 = 同一身份");
    }

    /// P5 新增类型（GLSL 前置变换 / uniform 绑定表 / 双后端引擎入口）必须能
    /// 从门面取到。
    #[test]
    fn p5_gpu_types_are_reachable_from_the_facade() {
        // 映射表：顺序 ⇒ binding 号。
        assert_eq!(crate::gpu::binding_of("uRes"), Some(0));
        assert_eq!(crate::gpu::binding_of("uTlGold"), Some(15));
        assert_eq!(crate::gpu::name_of(15), Some("uTlGold"));
        assert_eq!(crate::SHADER_UNIFORM_NAMES.len(), 16);

        // 引擎创建参数的中性默认值。
        let options = crate::EngineOptions::default();
        assert_eq!(options.backend, crate::BackendPreference::Auto);
        assert!(options.width > 0 && options.height > 0);

        // 未接线后端仍然诚实：档位是 Software（其**运行时**报错断言在浏览器侧 ——
        // native 造不出 `ScenePipeline`，见 ui-gpu 的 `context.rs` 说明）。
        let unavailable = crate::UnavailableGpuContext;
        assert_eq!(unavailable.tier(), Tier::Software);
    }
}
