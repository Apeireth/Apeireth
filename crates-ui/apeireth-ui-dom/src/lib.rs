//! # apeireth-ui-dom — DOM 绑定层（虚拟节点 / 后端抽象 / 差分 / 委托 / 绑定）
//!
//! 引擎里**唯一**接触 DOM 的一层。职责边界（设计文档 §3.3.2）：只回答
//! 「状态变成 DOM」，不回答状态从哪来（那是 `ui-core` 与数据层）。
//!
//! ## 为什么 DOM 操作必须被抽象在 trait 后面
//! 差分 / 委托 / 绑定逻辑全部写成泛型代码（`D: Dom`）⇒ 100% 可 native 单测，
//! **不需要浏览器、不需要 wasm target**。这是「消灭源码镜像测试」的结构性前提
//! （设计文档 §5.1.2 / §10.2）。
//!
//! ## 本批（P1）交付口径
//!
//! ### ✅ 已实现
//! - **虚拟节点**（元素 / 文本 / 分组）+ 元素节点的身份 key（[`Key`]）；
//! - **属性值语义**（布尔 `false` ⇒ 不写该属性）、class token 合并；
//! - **差分**：class 按 token + 属性键级 + 子节点按索引对齐（unkeyed 形态）；
//! - **keyed 列表差分**（[`keyed`]）：按 key 配对 + **最长递增子序列（LIS）**
//!   最小移动；移动次数 = 配对数 − LIS 长度（可机械核对的下界）；
//! - **事件委托表**（目标 → 祖先的冒泡顺序 + `stop_propagation`）；
//! - **双向绑定**（`bind:value` / `bind:checked` 的两条方向语义）；
//! - **`bind:this`**（[`ThisBinding`]）：挂载写引用 / 卸载前清引用；
//! - **内存 DOM 后端**（[`InMemoryDom`]，native 永远可用）；
//! - **失败显式化**：`Dom` 的每个操作都返回 `Result`（节点不存在 / 已摘除 /
//!   不适用 / 未接线 四类错误各自分明）⇒ **没有静默降级**。
//!
//! ### 🔵 未接线（等什么）
//! - **真实 DOM 后端**（web-sys / wasm-bindgen）——等 P0 探测结论落地
//!   （wasm-bindgen 0.2.129 ⟷ js-sys 0.3.106 ⟷ web-sys 0.3.106，精确硬钉 1:1）；
//!   该后端以 [`UnavailableDom`] **显式存在**：每个操作都返回
//!   `DomError::NotImplemented`，**不假装**自己在往真实 DOM 写东西。
//! - **渲染器**（`Renderer<D>`：挂载 / 打补丁 / 卸载的成对执行）——等 P2；
//!   因此 [`keyed::KeyedDiff`] 目前**只产出差分计划**（复用谁 / 移动谁 / 新建谁
//!   / 移除谁），**不产出 DOM 操作序列**；`Patch` 仍是 P0 的「按下标寻址」模型。
//! - **`bind:this` 的渲染器接线**——[`ThisBinding`] 的挂载 / 卸载语义已实现且
//!   可测，但「渲染器在挂载时调用 `mount`、卸载前调用 `unmount`」这一步要等
//!   P2 的渲染器；在那之前**不会**有静默降级（绑定槽位保持未绑定状态，
//!   `get()` 返回 `None`）。
//! - **wasm 侧微任务调度器**——`ui-core` 的 `HostScheduler` trait 边界已就位，
//!   「把 flush 排进浏览器微任务队列」的那一实现落在 **wasm 宿主侧**（与本 crate
//!   的真实 DOM 后端一起接）。
//!
//! ## 纪律
//! 本 crate 的源码里**不出现颜色字面量与时长字面量**（设计文档 §2.2）：那些归
//! CSS 资产与设计令牌，引擎只引用 class 与结构。

#![forbid(unsafe_code)]

pub mod binding;
pub mod diff;
pub mod dom;
pub mod events;
pub mod keyed;
pub mod vnode;

pub use binding::{Binding, BindingSlot, ThisBinding};
pub use diff::{diff, Patch};
pub use dom::{
    Dom, DomError, EventKind, InMemoryDom, InMemoryNodeId, UnavailableDom, UnavailableNode,
};
pub use events::{DomEvent, EventHandler, EventRegistry, HandlerId};
pub use keyed::{diff_keyed, list_diff_mode, KeyedDiff, KeyedDiffError, KeyedSlot, ListDiffMode};
pub use vnode::{AttrValue, ElementBuilder, ElementVNode, Key, TextVNode, VNode};

#[cfg(test)]
mod tests {
    //! crate 级断言：只放**跨模块**的组合行为（虚拟树 → 差分 → 内存 DOM）。

    use super::*;

    #[test]
    fn virtual_tree_renders_into_memory_backend() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("section").expect("建节点必须成功");
        dom.set_attribute(&root, "class", "ap-shell")
            .expect("写属性必须成功");
        let card = VNode::element("div")
            .class("ap-card")
            .attr("role", "group")
            .child(VNode::text("条目"))
            .build();
        let card_node = dom.create_element("div").expect("建节点必须成功");

        if let VNode::Element(element) = &card {
            for (name, value) in &element.attrs {
                if let Some(value) = value.to_attr_string() {
                    dom.set_attribute(&card_node, name, &value)
                        .expect("写属性必须成功");
                }
            }
            for child in &element.children {
                if let VNode::Text(text) = child {
                    let node = dom.create_text(&text.data).expect("建节点必须成功");
                    dom.append_child(&card_node, &node).expect("挂载必须成功");
                }
            }
        }
        dom.append_child(&root, &card_node).expect("挂载必须成功");

        assert_eq!(dom.tag(root), Some(String::from("section")));
        assert_eq!(dom.attribute(root, "class"), Some(String::from("ap-shell")));
        assert_eq!(
            dom.attribute(card_node, "class"),
            Some(String::from("ap-card"))
        );
        assert_eq!(dom.text_content(root), "条目");
    }

    #[test]
    fn diff_between_two_render_trees_is_empty() {
        let build = || {
            VNode::element("div")
                .class("a")
                .child(VNode::element("span").child(VNode::text("x")).build())
                .build()
        };
        assert!(diff(&build(), &build()).is_empty());
    }

    #[test]
    fn keyed_children_carry_their_identity_into_the_diff() {
        // 虚拟树 ⇒ keyed 差分 ⇒ 身份按 key 复用（跨模块组合）。
        let old = vec![
            VNode::element("li")
                .key("a")
                .child(VNode::text("1"))
                .build(),
            VNode::element("li")
                .key("b")
                .child(VNode::text("2"))
                .build(),
        ];
        let new = vec![
            VNode::element("li")
                .key("b")
                .child(VNode::text("2"))
                .build(),
            VNode::element("li")
                .key("a")
                .child(VNode::text("1"))
                .build(),
        ];

        let plan = diff_keyed(&old, &new).expect("差分必须成功");
        assert_eq!(plan.mode(), ListDiffMode::Keyed);
        assert_eq!(plan.matched(), &[Some(1), Some(0)]);
        assert_eq!(plan.move_count(), 1, "只移动一个节点（LIS 下界）");
    }

    #[test]
    fn bind_this_slot_travels_from_the_view_to_the_dom_backend() {
        // `bind:this` 槽位 ⇒ 内存 DOM 节点（跨模块组合）。
        let dom = InMemoryDom::new();
        let node = dom.create_element("canvas").expect("建节点必须成功");
        let binding: ThisBinding<InMemoryDom> = ThisBinding::new();

        assert_eq!(binding.get(), None);
        binding.mount(node);
        assert_eq!(binding.get(), Some(node));
        binding.unmount();
        assert_eq!(binding.get(), None);
    }
}
