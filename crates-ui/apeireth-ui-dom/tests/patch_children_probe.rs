//! ⭐ **独立复现**：`patch_children_at_path` 在「旧子级为空」时是否真的新增 DOM 子节点。
//!
//! 独立文件的好处：不与 `renderer.rs` 内部的锚点/格式化纠缠，结论无歧义。

use apeireth_ui_dom::dom::Dom;
use apeireth_ui_dom::vnode::VNode;
use apeireth_ui_dom::{InMemoryDom, Renderer};

#[test]
fn patch_children_adds_dom_children_when_old_is_empty() {
    let dom = InMemoryDom::new();
    let root = dom.create_element("main").expect("建节点必须成功");
    let mut renderer = Renderer::new(dom, root);
    renderer
        .mount(
            VNode::element("div")
                .child(VNode::element("span").build())
                .build(),
        )
        .expect("挂载必须成功");

    let tree_root = renderer.dom().children(&root).expect("读子节点必须成功")[0];
    let host = renderer
        .dom()
        .children(&tree_root)
        .expect("读子节点必须成功")[0];
    let before = renderer
        .dom()
        .children(&host)
        .expect("读子节点必须成功")
        .len();
    println!("before = {before}");

    renderer
        .patch_children_at_path(
            &[0],
            vec![
                VNode::element("span").build(),
                VNode::element("span").build(),
                VNode::element("span").build(),
            ],
        )
        .expect("子级替换必须成功");

    let after = renderer
        .dom()
        .children(&host)
        .expect("读子节点必须成功")
        .len();
    println!("after = {after}");
    assert_eq!(after, 3, "必须真的新增 3 个 DOM 子节点（实测 {after}）");
}

/// ⭐ **嵌套窗口化渲染**：留白由引擎算 ⇒ 宿主内容**真的能滚**。
///
/// 这条测的是 2026-10-28 浏览器集成探针发现的陷阱：
/// 只写窗口行（不留白）⇒ 宿主高度不足 ⇒ 浏览器把 `scrollTop` 夹回 0 ⇒ 窗口不动。
#[test]
fn nested_windowed_render_keeps_container_scrollable() {
    use apeireth_ui_dom::dom::{ElementBox, ScrollMetrics};

    let dom = InMemoryDom::new();
    let root = dom.create_element("main").expect("建节点必须成功");
    let mut renderer = Renderer::new(dom, root);
    renderer
        .mount(
            VNode::element("div")
                .child(VNode::element("div").build())
                .build(),
        )
        .expect("挂载必须成功");

    let tree_root = renderer.dom().children(&root).expect("读子节点必须成功")[0];
    let host = renderer
        .dom()
        .children(&tree_root)
        .expect("读子节点必须成功")[0];
    renderer.dom().script_measurement(
        host,
        ElementBox::new(0.0, 0.0, 300.0, 200.0),
        ScrollMetrics {
            scroll_top: 0.0,
            scroll_height: 4000.0,
            client_height: 200.0,
        },
    );

    let heights = vec![40.0_f64; 100];
    let plan = renderer
        .patch_windowed_at_path(&[0], &heights, 0, |index| {
            VNode::element("div")
                .attr("class", "row")
                .attr("style", "height: 40px")
                .child(VNode::text(format!("{index}")))
                .build()
        })
        .expect("嵌套窗口化必须成功");
    assert_eq!(plan.first, 0, "初始在顶部");

    let rows = renderer
        .dom()
        .children(&host)
        .expect("读子节点必须成功")
        .len();
    // 视口 200 / 行高 40 ⇒ 5 行可见 + 1 个后留白
    assert_eq!(rows, 6, "应含 5 个窗口行 + 1 个后留白（实测 {rows}）");
}
