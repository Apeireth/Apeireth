//! `apeireth-ui` 集成冒烟：门面转发各层（试点应用只需依赖门面一个 crate）。

use apeireth_ui::core::Runtime;
use apeireth_ui::dom::VNode;
use apeireth_ui::kit::{Button, ButtonProps};
use apeireth_ui::{ComponentRegistry, UiComponent};

#[test]
fn facade_exposes_a_usable_vertical_slice() {
    let runtime = Runtime::new();
    let count = runtime.signal(0u32);
    assert_eq!(count.get(), 0);
    count.set(1);
    assert_eq!(count.get(), 1);

    let node = Button::render(&ButtonProps::new("批准"));
    assert_eq!(node.tag(), Some("button"));

    let element = VNode::element("div").class("ap-root").build();
    assert_eq!(element.tag(), Some("div"));

    let registry = ComponentRegistry::new();
    assert!(registry.is_empty());
}

#[test]
fn facade_component_macro_is_usable_end_to_end() {
    use apeireth_ui::component;

    #[component]
    fn Badge(text: String) -> VNode {
        VNode::element("span")
            .class("ap-badge")
            .child(VNode::text(text))
            .build()
    }

    let node = Badge::render(&BadgeProps {
        text: String::from("新"),
    });
    assert_eq!(node.tag(), Some("span"));
}
