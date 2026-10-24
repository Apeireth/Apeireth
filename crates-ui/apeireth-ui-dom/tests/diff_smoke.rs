//! `apeireth-ui-dom` 集成冒烟：虚拟树 → 差分 → 内存 DOM 的**跨模块**行为。

use apeireth_ui_dom::{diff, Dom, InMemoryDom, Patch, VNode};

#[test]
fn changed_tree_produces_minimal_patches() {
    let old = VNode::element("div")
        .class("ap-card")
        .child(VNode::text("旧"))
        .build();
    let new = VNode::element("div")
        .class("ap-card")
        .child(VNode::text("新"))
        .build();

    let patches = diff(&old, &new);
    assert_eq!(
        patches,
        vec![Patch::SetText {
            path: vec![0],
            text: String::from("新")
        }],
        "只该有一条文本补丁"
    );
}

#[test]
fn virtual_tree_maps_into_memory_backend() {
    let dom = InMemoryDom::new();
    let root = dom.create_element("main");

    let node = VNode::element("section")
        .class("ap-panel")
        .child(VNode::text("内容"))
        .build();

    if let VNode::Element(element) = &node {
        dom.set_attribute(&root, "class", "ap-root");
        for (name, value) in &element.attrs {
            if let Some(value) = value.to_attr_string() {
                dom.set_attribute(&root, name, &value);
            }
        }
        for child in &element.children {
            if let VNode::Text(text) = child {
                let text_node = dom.create_text(&text.data);
                dom.append_child(&root, &text_node);
            }
        }
    }

    assert_eq!(dom.tag(root), Some(String::from("main")));
    assert_eq!(dom.text_content(root), "内容");
    assert_eq!(dom.children(&root).len(), 1);
}
