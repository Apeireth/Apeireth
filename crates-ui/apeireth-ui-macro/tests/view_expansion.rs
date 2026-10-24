//! `view!` 宏的端到端测试：宏展开后**真的能构造出虚拟树**。
//!
//! 断言落在**行为**上（虚拟树的形状、class token 集合、属性值），不是落在
//! 「生成文本长什么样」上（那属源码镜像断言，设计文档 §10.2 禁止）。
//!
//! 生成文本层面的断言在 `src/view.rs` 的单元测试里（那里比的是**宏自己生成**
//! 的代码，属 §10.2 的 E1 例外：宏展开快照）。

use apeireth_ui_dom::{AttrValue, EventKind, VNode};

/// 读元素的 class token 集合（保序）。
fn class_tokens(node: &VNode) -> Vec<String> {
    match node {
        VNode::Element(element) => match element.attr_value("class") {
            Some(AttrValue::Text(text)) => text.split_whitespace().map(str::to_owned).collect(),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}

/// 读元素的属性值。
fn attr<'a>(node: &'a VNode, name: &str) -> Option<&'a AttrValue> {
    match node {
        VNode::Element(element) => element.attr_value(name),
        _ => None,
    }
}

/// 取子节点。
fn children(node: &VNode) -> &[VNode] {
    node.children()
}

#[test]
fn plain_element_becomes_a_virtual_element() {
    let node = apeireth_ui_macro::view! { div };
    assert_eq!(node.tag(), Some("div"));
    assert!(children(&node).is_empty());
}

#[test]
fn literal_attributes_land_on_the_element() {
    let node = apeireth_ui_macro::view! {
        div(class = "approval-card", role = "group", "aria-label" = "待批准操作")
    };
    assert_eq!(class_tokens(&node), vec!["approval-card"]);
    assert_eq!(
        attr(&node, "role"),
        Some(&AttrValue::Text(String::from("group")))
    );
    assert_eq!(
        attr(&node, "aria-label"),
        Some(&AttrValue::Text(String::from("待批准操作")))
    );
}

#[test]
fn conditional_class_token_is_added_only_when_true() {
    let on = apeireth_ui_macro::view! { span(class = "badge", class:copied = true) };
    assert_eq!(class_tokens(&on), vec!["badge", "copied"]);

    let off = apeireth_ui_macro::view! { span(class = "badge", class:copied = false) };
    assert_eq!(class_tokens(&off), vec!["badge"]);
}

#[test]
fn literal_text_becomes_a_text_node() {
    let node = apeireth_ui_macro::view! { span { "拒绝" } };
    assert_eq!(children(&node).len(), 1);
    match &children(&node)[0] {
        VNode::Text(text) => assert_eq!(text.data, "拒绝"),
        other => panic!("实测节点形态：{other:?}"),
    }
}

#[test]
fn dynamic_text_is_evaluated_at_render_time() {
    let text_value = String::from("批准");
    let node = apeireth_ui_macro::view! { span { text_value } };
    match &children(&node)[0] {
        VNode::Text(text) => assert_eq!(text.data, "批准"),
        other => panic!("实测节点形态：{other:?}"),
    }
}

#[test]
fn dynamic_child_node_is_spliced_into_the_tree() {
    let inner = VNode::text("动态");
    let node = apeireth_ui_macro::view! { div { { inner } } };
    assert_eq!(children(&node).len(), 1);
    match &children(&node)[0] {
        VNode::Text(text) => assert_eq!(text.data, "动态"),
        other => panic!("实测节点形态：{other:?}"),
    }
}

#[test]
fn if_block_selects_the_branch() {
    let on =
        apeireth_ui_macro::view! { div { if true { span { "命中" } } else { span { "未中" } } } };
    assert_eq!(children(&on).len(), 1);
    let fragment = match &children(&on)[0] {
        VNode::Fragment(items) => items,
        other => panic!("实测节点形态：{other:?}"),
    };
    assert_eq!(fragment.len(), 1);

    let off =
        apeireth_ui_macro::view! { div { if false { span { "命中" } } else { span { "未中" } } } };
    let fragment = match &children(&off)[0] {
        VNode::Fragment(items) => items,
        other => panic!("实测节点形态：{other:?}"),
    };
    assert_eq!(fragment.len(), 1);
}

#[test]
fn for_block_maps_over_the_iterator() {
    let items = vec![String::from("甲"), String::from("乙")];
    let node = apeireth_ui_macro::view! {
        ul { for item in items { li(key = item.clone()) { item } } }
    };
    let list = match &children(&node)[0] {
        VNode::Fragment(items) => items,
        other => panic!("实测节点形态：{other:?}"),
    };
    assert_eq!(list.len(), 2);
    match &list[0] {
        VNode::Element(element) => {
            assert_eq!(element.tag, "li");
            assert!(element.key.is_some(), "列表项必须带 key");
        }
        other => panic!("实测节点形态：{other:?}"),
    }
}

#[test]
fn event_handlers_are_attached_to_the_element() {
    let node = apeireth_ui_macro::view! {
        button(onclick = move |_event| { /* 空实现 */ })
    };
    match &node {
        VNode::Element(element) => {
            assert_eq!(element.handlers.len(), 1);
            assert_eq!(element.handlers[0].0, EventKind::Click);
        }
        other => panic!("实测节点形态：{other:?}"),
    }
}

#[test]
fn nested_structure_keeps_its_shape() {
    let node = apeireth_ui_macro::view! {
        div(class = "card") {
            div(class = "header") { h3(class = "title") { "标题" } }
            div(class = "body") { p { "正文" } }
        }
    };
    assert_eq!(class_tokens(&node), vec!["card"]);
    assert_eq!(children(&node).len(), 2);
    assert_eq!(children(&node)[0].tag(), Some("div"));
    assert_eq!(class_tokens(&children(&node)[0]), vec!["header"]);
    assert_eq!(children(&children(&node)[0]).len(), 1);
    assert_eq!(children(&children(&node)[0])[0].tag(), Some("h3"));
    assert_eq!(
        class_tokens(&children(&children(&node)[0])[0]),
        vec!["title"]
    );
}
