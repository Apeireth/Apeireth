//! `apeireth-ui-dom` 集成冒烟：虚拟树 → 差分 → 内存 DOM 的**跨模块**行为。

use apeireth_ui_dom::{
    diff, diff_keyed, Dom, DomError, InMemoryDom, KeyedSlot, ListDiffMode, Patch, ThisBinding,
    UnavailableDom, VNode,
};

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
    let root = dom.create_element("main").expect("建节点必须成功");

    let node = VNode::element("section")
        .class("ap-panel")
        .child(VNode::text("内容"))
        .build();

    if let VNode::Element(element) = &node {
        dom.set_attribute(&root, "class", "ap-root")
            .expect("写属性必须成功");
        for (name, value) in &element.attrs {
            if let Some(value) = value.to_attr_string() {
                dom.set_attribute(&root, name, &value)
                    .expect("写属性必须成功");
            }
        }
        for child in &element.children {
            if let VNode::Text(text) = child {
                let text_node = dom.create_text(&text.data).expect("建节点必须成功");
                dom.append_child(&root, &text_node).expect("挂载必须成功");
            }
        }
    }

    assert_eq!(dom.tag(root), Some(String::from("main")));
    assert_eq!(dom.text_content(root), "内容");
    assert_eq!(dom.children(&root).expect("读子节点必须成功").len(), 1);
}

// ── keyed 列表：身份按 key 复用，移动次数达到 LIS 下界 ──────────────────

#[test]
fn keyed_reorder_keeps_node_identity_and_moves_the_minimum() {
    let old: Vec<VNode> = ["a", "b", "c", "d"]
        .iter()
        .map(|key| VNode::element("li").key(*key).build())
        .collect();
    let new: Vec<VNode> = ["d", "a", "b", "c"]
        .iter()
        .map(|key| VNode::element("li").key(*key).build())
        .collect();

    let plan = diff_keyed(&old, &new).expect("差分必须成功");
    assert_eq!(plan.mode(), ListDiffMode::Keyed);
    assert_eq!(plan.matched(), &[Some(3), Some(0), Some(1), Some(2)]);
    assert_eq!(
        plan.move_count(),
        1,
        "只有 d 需要移动（LIS 下界 = 配对数 4 − LIS 长度 3）"
    );
    assert_eq!(plan.retained_count(), 3);
}

#[test]
fn keyed_plan_describes_the_new_order_as_slots() {
    let old = vec![
        VNode::element("li").key("a").build(),
        VNode::element("li").key("b").build(),
    ];
    let new = vec![
        VNode::element("li").key("b").build(),
        VNode::element("li").key("c").build(),
    ];

    let plan = diff_keyed(&old, &new).expect("差分必须成功");
    let slots = plan.slots();
    assert_eq!(slots[0], KeyedSlot::Reuse(1), "b 复用旧位置 1");
    assert!(matches!(slots[1], KeyedSlot::New(_)), "c 是新建的");
    assert_eq!(plan.removed(), &[0], "a 被移除");
}

// ── bind:this：挂载写引用 / 卸载前清引用 ─────────────────────────────────

#[test]
fn bind_this_slot_is_cleared_before_the_node_leaves_the_tree() {
    let dom = InMemoryDom::new();
    let root = dom.create_element("div").expect("建节点必须成功");
    let canvas = dom.create_element("canvas").expect("建节点必须成功");
    dom.append_child(&root, &canvas).expect("挂载必须成功");

    let binding: ThisBinding<InMemoryDom> = ThisBinding::new();
    binding.mount(canvas);
    assert_eq!(binding.get(), Some(canvas));

    // 卸载：先清引用，再摘节点（设计文档 §5.1.4 的成对契约）。
    binding.unmount();
    dom.remove_child(&root, &canvas).expect("摘除必须成功");

    assert!(!binding.is_bound(), "卸载后引用必须为空");
    assert_eq!(binding.get(), None);
    assert!(dom.is_detached(canvas));
}

// ── 诚实边界：失败必须显式（不许静默降级）──────────────────────────────

#[test]
fn writing_to_a_removed_node_fails_loudly() {
    let dom = InMemoryDom::new();
    let root = dom.create_element("div").expect("建节点必须成功");
    let item = dom.create_element("span").expect("建节点必须成功");
    dom.append_child(&root, &item).expect("挂载必须成功");
    dom.remove_child(&root, &item).expect("摘除必须成功");

    assert_eq!(
        dom.set_attribute(&item, "class", "ap-row")
            .expect_err("写入已摘除节点必须报错"),
        DomError::NodeDetached
    );
}

#[test]
fn the_unwired_dom_backend_reports_not_implemented() {
    let dom = UnavailableDom;
    assert_eq!(
        dom.create_element("div").expect_err("未接线后端必须报错"),
        DomError::NotImplemented
    );
}
