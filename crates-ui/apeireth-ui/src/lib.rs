//! # apeireth-ui — 门面（re-export 各层）
//!
//! 职责边界（设计文档 §3.3.6）：**零逻辑**。门面里出现任何一行非 `pub use` 的
//! 实现代码即违规 —— 门面只是「让试点应用不必写 5 条 `use`」。
//!
//! ## 各层与拓扑秩（ADR-0015 §3.3，单向下行无环）
//! | 层 | 拓扑秩 | 门面里的名字 |
//! |---|:--:|---|
//! | 响应式核心（信号 / 派生 / 依赖图 / 调度 / effect） | 1 | [`core`] |
//! | DOM 绑定（虚拟节点 / 差分 / 委托 / 绑定） | 2 | [`dom`] |
//! | GPU 桥（状态 → 像素） | 2 | [`gpu`] |
//! | 组件库（共享组件 + 注册表） | 3 | [`kit`] |
//! | 过程宏（`#[component]`） | 0 | [`component`] |
//!
//! 常用类型同时在门面根部具名导出（`Signal` / `VNode` / `UiComponent` …），
//! 试点应用通常只需要 `use apeireth_ui::…;`。

#![forbid(unsafe_code)]

pub use apeireth_ui_core as core;
pub use apeireth_ui_dom as dom;
pub use apeireth_ui_gpu as gpu;
pub use apeireth_ui_kit as kit;
pub use apeireth_ui_macro::component;

pub use apeireth_ui_core::{
    Derived, EffectHandle, FlushReport, ReactiveError, ReadSignal, Runtime, Scope, Signal,
    UiComponent,
};
pub use apeireth_ui_dom::{Binding, BindingSlot, Dom, DomEvent, EventKind, Patch, VNode};
pub use apeireth_ui_gpu::{BlackHoleParams, GpuContext, GpuError, SceneUniforms, Tier};
pub use apeireth_ui_kit::{Button, ButtonProps, ComponentRegistry, RegistryError};

#[cfg(test)]
mod tests {
    //! crate 级断言：门面只做转发 ⇒ 断言「各层可达 + 跨层类型一致」。

    use crate::core::Runtime;
    use crate::dom::{Patch, VNode};
    use crate::gpu::Tier;
    use crate::kit::ComponentRegistry;

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
}
