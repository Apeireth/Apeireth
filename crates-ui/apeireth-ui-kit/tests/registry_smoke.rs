//! `apeireth-ui-kit` 集成冒烟：组件注册 → 按名渲染的**跨模块**行为。

use apeireth_ui_core::UiComponent;
use apeireth_ui_dom::VNode;
use apeireth_ui_kit::{Button, ButtonProps, ComponentRegistry, RegistryError};

#[test]
fn button_component_registers_and_renders_by_name() {
    let mut registry = ComponentRegistry::new();
    registry
        .register_named::<Button>("button")
        .expect("首次注册成功");

    let props = ButtonProps::new("批准");
    let node = registry
        .render::<Button>("button", &props)
        .expect("渲染成功");
    assert_eq!(node.tag(), Some("button"));
    assert_eq!(node, Button::render(&props), "组件是纯函数");
}

#[test]
fn duplicate_and_missing_registrations_are_reported() {
    let mut registry = ComponentRegistry::new();
    registry
        .register_named::<Button>("button")
        .expect("首次注册成功");

    assert_eq!(
        registry.register_named::<Button>("button"),
        Err(RegistryError::Duplicate(String::from("button")))
    );

    let props = ButtonProps::new("批准");
    assert_eq!(
        registry.render::<Button>("nope", &props),
        Err(RegistryError::NotFound(String::from("nope")))
    );
}

#[test]
fn registry_renders_a_custom_component() {
    struct Panel;

    impl UiComponent for Panel {
        type Props = String;
        type Output = VNode;

        fn render(props: &Self::Props) -> Self::Output {
            VNode::element("aside")
                .child(VNode::text(props.clone()))
                .build()
        }
    }

    let mut registry = ComponentRegistry::new();
    registry
        .register_named::<Panel>("panel")
        .expect("首次注册成功");

    let node = registry
        .render::<Panel>("panel", &String::from("侧栏"))
        .expect("渲染成功");
    assert_eq!(node.tag(), Some("aside"));
}
