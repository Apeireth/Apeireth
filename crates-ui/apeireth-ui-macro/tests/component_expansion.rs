//! `#[component]` 宏展开的端到端测试。
//!
//! 这是**展开快照之外**的行为断言：直接调用宏生成的组件类型，断言 props 结构、
//! 构造器与 `UiComponent::render` 的行为（设计文档 §6.4：组件 = 纯函数，
//! native 直接调 ⇒ 不需要浏览器）。

use apeireth_ui_core::UiComponent;
use apeireth_ui_dom::{Children, VNode};
use apeireth_ui_macro::component;

/// 一个用宏声明的组件。
#[component]
fn Greeting(name: String, times: usize) -> VNode {
    VNode::element("span")
        .class("ap-greeting")
        .child(VNode::text(format!("{name}")))
        .build()
}

/// 另一个用宏声明的组件（零参数）。
#[component]
fn Separator() -> VNode {
    VNode::element("hr").build()
}

/// 带可选参数与子内容插槽的组件。
#[component]
fn Panel(
    title: String,
    /// 紧凑排版（可选，缺省 `false`）。
    #[prop(default = false)]
    compact: bool,
    /// 子内容插槽（永远可选，缺省为空子树）。
    children: Children,
) -> VNode {
    let mut panel = VNode::element("div")
        .class("ap-panel")
        .child(VNode::text(title))
        .child(children.render());
    if compact {
        panel = panel.class("ap-panel--compact");
    }
    panel.build()
}

#[test]
fn macro_generates_props_struct_and_component_type() {
    let props = GreetingProps {
        name: String::from("Apeireth"),
        times: 2,
    };
    let node = Greeting::render(&props);
    assert_eq!(node.tag(), Some("span"));

    let instance = Greeting::new(String::from("Apeireth"), 2);
    assert_eq!(instance.props.name, "Apeireth");
    assert_eq!(instance.props.times, 2);
}

#[test]
fn macro_generated_component_is_a_pure_function() {
    let props = GreetingProps {
        name: String::from("Apeireth"),
        times: 2,
    };
    assert_eq!(Greeting::render(&props), Greeting::render(&props));
}

#[test]
fn macro_supports_zero_parameter_components() {
    let node = Separator::render(&SeparatorProps {});
    assert_eq!(node.tag(), Some("hr"));
    let instance = Separator::new();
    assert_eq!(Separator::render(&instance.props), node);
}

#[test]
fn optional_props_fall_back_to_their_default() {
    let instance = Panel::new(String::from("标题"));
    assert!(
        !instance.props.compact,
        "缺省值来自 `#[prop(default = ...)]`"
    );
    assert!(instance.props.children.render().children().is_empty());
}

#[test]
fn optional_props_can_be_overridden_by_the_builder() {
    let instance = Panel::builder()
        .title(String::from("标题"))
        .compact(true)
        .build();
    assert!(instance.props.compact);
    assert_eq!(
        Panel::render(&instance.props).tag(),
        Some("div"),
        "紧凑态只加 class，不改结构"
    );
}

#[test]
fn children_slot_content_is_rendered_inside_the_component() {
    let instance = Panel::builder()
        .title(String::from("标题"))
        .children(Children::new(|| VNode::text("插槽内容")))
        .build();
    let node = Panel::render(&instance.props);
    let text: String = node
        .children()
        .iter()
        .filter_map(|child| match child {
            VNode::Text(text) => Some(text.data.clone()),
            _ => None,
        })
        .collect();
    assert!(text.contains("插槽内容"), "实测文本：{text}");
}
