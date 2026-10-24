//! `#[component]` 宏展开的端到端测试。
//!
//! 这是**展开快照之外**的行为断言：直接调用宏生成的组件类型，断言 props 结构、
//! 构造器与 `UiComponent::render` 的行为（设计文档 §6.4：组件 = 纯函数，
//! native 直接调 ⇒ 不需要浏览器）。

use apeireth_ui::core::UiComponent;
use apeireth_ui::dom::VNode;
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
