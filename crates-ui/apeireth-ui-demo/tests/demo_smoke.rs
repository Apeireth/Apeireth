//! `apeireth-ui-demo` 集成冒烟：试点组件的**行为**断言（不读源码文本）。
//!
//! 设计文档 §10.2：禁���「读源文件 + 文本断言」⇒ 这里全部是**调用组件**、
//! **断言产物**。

use apeireth_ui_core::{Runtime, UiComponent};
use apeireth_ui_demo::{pilot_callbacks, pilot_item, ApprovalCard, ApprovalCardProps, PilotState};
use apeireth_ui_dom::{Children, Dom, InMemoryDom, Renderer, VNode};

/// 建一份试点 props。
fn pilot_props(state: &PilotState) -> ApprovalCardProps {
    let (allow, reject, dismiss) = pilot_callbacks(state);
    ApprovalCardProps {
        item: pilot_item(),
        busy: state.busy().get(),
        copied: state.copied(),
        on_allow: allow,
        on_reject: reject,
        on_dismiss: Some(dismiss),
        children: Children::default(),
    }
}

/// 收集虚拟树里出现的全部 class token。
fn collect_classes(node: &VNode, out: &mut Vec<String>) {
    for token in node.class_tokens() {
        if !out.contains(&token) {
            out.push(token);
        }
    }
    for child in node.children() {
        collect_classes(child, out);
    }
}

#[test]
fn the_pilot_card_renders_the_same_class_vocabulary_as_the_reference() {
    let state = PilotState::new(Runtime::new().reactive_scope());
    let node = ApprovalCard::render(&pilot_props(&state));
    let mut classes = Vec::new();
    collect_classes(&node, &mut classes);

    for expected in [
        "approval-card",
        "card-header",
        "card-body",
        "card-footer",
        "title-wrap",
        "title",
        "time",
        "section",
        "section-label",
        "section-text",
        "command-row",
        "command",
        "sandbox-row",
        "sandbox-badge",
        "sandbox-meta",
        "allow-btn",
        "reject-btn",
        "close-btn",
        "copy-btn",
        "mono-path",
        "reason",
    ] {
        assert!(
            classes.iter().any(|token| token == expected),
            "class `{expected}` 未出现；实测：{classes:?}"
        );
    }
}

#[test]
fn the_pilot_card_mounts_into_the_memory_backend() {
    let state = PilotState::new(Runtime::new().reactive_scope());
    let dom = InMemoryDom::new();
    let root = dom.create_element("main").expect("建节点必须成功");
    let mut renderer = Renderer::new(dom, root);
    renderer
        .mount(ApprovalCard::render(&pilot_props(&state)))
        .expect("挂载必须成功");

    assert!(renderer.is_mounted());
    assert_eq!(renderer.bound_slots(), 0);
}

#[test]
fn the_copy_button_switches_its_icon_and_class_when_copied() {
    let runtime = Runtime::new();
    let state = PilotState::new(runtime.reactive_scope());
    let before = ApprovalCard::render(&pilot_props(&state));
    let before_text = apeireth_ui_demo::tree_to_text(&before);
    assert!(!before_text.contains("copied"), "复制前没有 `copied` class");

    state.mark_copied();
    runtime.flush();

    let after = ApprovalCard::render(&pilot_props(&state));
    let after_text = apeireth_ui_demo::tree_to_text(&after);
    assert!(
        after_text.contains("copy-btn copied"),
        "复制后必须带上 `copied` class：{after_text}"
    );
}
