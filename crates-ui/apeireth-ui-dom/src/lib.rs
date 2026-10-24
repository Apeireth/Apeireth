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
//! ## 本批（P0 骨架）交付口径
//! - ✅ **已实现**：虚拟节点（元素 / 文本 / 分组）、属性值语义（布尔 `false` ⇒
//!   不写该属性）、class token 合并、差分（class 按 token + 属性键级 + 子节点
//!   按索引对齐）、事件委托表（目标 → 祖先的冒泡顺序 + `stop_propagation`）、
//!   双向绑定（`bind:value` / `bind:checked` 的两条方向语义）、内存 DOM 后端。
//! - 🔵 **未接线**：真实 DOM 后端（web-sys / wasm-bindgen）——等 P0 探测结论
//!   （wasm-bindgen 0.2.129 ⟷ js-sys 0.3.106 ⟷ web-sys 0.3.106，精确硬钉 1:1）；
//!   keyed 列表的 LIS 最小移动 —— P2。
//!
//! ## 纪律
//! 本 crate 的源码里**不出现颜色字面量与时长字面量**（设计文档 §2.2）：那些归
//! CSS 资产与设计令牌，引擎只引用 class 与结构。

#![forbid(unsafe_code)]

pub mod binding;
pub mod diff;
pub mod dom;
pub mod events;
pub mod vnode;

pub use binding::{Binding, BindingSlot};
pub use diff::{diff, Patch};
pub use dom::{Dom, DomError, EventKind, InMemoryDom, InMemoryNodeId};
pub use events::{DomEvent, EventHandler, EventRegistry, HandlerId};
pub use vnode::{AttrValue, ElementBuilder, ElementVNode, TextVNode, VNode};

#[cfg(test)]
mod tests {
    //! crate 级断言：只放**跨模块**的组合行为（虚拟树 → 差分 → 内存 DOM）。

    use super::*;

    #[test]
    fn virtual_tree_renders_into_memory_backend() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("section");
        dom.set_attribute(&root, "class", "ap-shell");
        let card = VNode::element("div")
            .class("ap-card")
            .attr("role", "group")
            .child(VNode::text("审批"))
            .build();
        let card_node = dom.create_element("div");

        if let VNode::Element(element) = &card {
            for (name, value) in &element.attrs {
                if let Some(value) = value.to_attr_string() {
                    dom.set_attribute(&card_node, name, &value);
                }
            }
            for child in &element.children {
                if let VNode::Text(text) = child {
                    let node = dom.create_text(&text.data);
                    dom.append_child(&card_node, &node);
                }
            }
        }
        dom.append_child(&root, &card_node);

        assert_eq!(dom.tag(root), Some(String::from("section")));
        assert_eq!(dom.attribute(root, "class"), Some(String::from("ap-shell")));
        assert_eq!(
            dom.attribute(card_node, "class"),
            Some(String::from("ap-card"))
        );
        assert_eq!(dom.text_content(root), "审批");
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
}
