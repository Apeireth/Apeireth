//! ⭐⭐⭐ **「绑定信号 → 元素路径」映射**（独立回归测试；不读生产源码文本）。
//!
//! ## 它钉住什么
//!
//! 实测（E-137）：4.7k 元素的树里，**改一个叶子**的「差分 patch」要 **89 ms**
//! ⇒ ⇒ 瓶颈是 **patch 的逐节点比对范围** ⇒ ⇒ 要收窄它，就必须知道
//! 「**哪个信号变 ⇒ 哪些 DOM 子树要重画**」⇒ ⇒ 本测试钉住那个映射的**正确性**。
//!
//! ## 路径语义（关键）
//!
//! - 路径 = 从根到目标元素的**子节点索引**序列（与 `Renderer::node_for_path` 同语义）；
//! - ⚠️ `VNode::Fragment` **不产生 DOM 节点** ⇒ ⇒ 它的子节点**上提**到父级序列
//!   ⇒ ⇒ 索引按**展平后**的位置计（**最容易搞错的一点**）。

use apeireth_ui_core::Runtime;
use apeireth_ui_dom::binding::Binding;
use apeireth_ui_dom::dom::Dom;
use apeireth_ui_dom::renderer::Renderer;
use apeireth_ui_dom::vnode::VNode;

#[test]
fn binding_paths_point_at_the_bound_elements() {
    let runtime = Runtime::new();
    let first = runtime.signal(String::from("一"));
    let second = runtime.signal(String::from("二"));
    let (mut renderer, _root) = renderer_with_dom();

    // 结构：root(main)
    //   └ div#row
    //       ├ span
    //       ├ Fragment[ input(first), span ]   ← Fragment 不占索引
    //       └ input(second)
    let view = VNode::element("div")
        .attr("id", "row")
        .child(VNode::element("span").build())
        .child(VNode::Fragment(vec![
            VNode::element("input")
                .bind(Binding::text(first.clone()))
                .build(),
            VNode::element("span").build(),
        ]))
        .child(
            VNode::element("input")
                .bind(Binding::text(second.clone()))
                .build(),
        )
        .build();

    let paths = view.collect_binding_paths();
    assert_eq!(paths.len(), 2, "两个绑定 ⇒ 两条路径");

    // ① 第一个绑定（Fragment 里的 input）：路径 = [0]（div 的第 0 个子节点）
    let first_path = paths
        .iter()
        .find(|entry| entry.signal_id == first.id())
        .expect("必须能找到第一个绑定的路径");
    assert_eq!(
        first_path.path,
        vec![1usize, 0usize],
        "⚠️ 路径 = **挂载树路径**（Fragment 自己占一个索引）
\
         ⇒ ⇒ Fragment 里的 input = [Fragment(1), input(0)]
\
         ⚠️ 与 `node_for_path` **同根定义**（tree 根 = 挂载的 VNode 根）⇒ ⇒ 可直接互喂"
    );

    // ② 第二个绑定（div 的第三个子节点）：Fragment 展平后占 2 个位置 ⇒ ⇒ 索引 = 2
    let second_path = paths
        .iter()
        .find(|entry| entry.signal_id == second.id())
        .expect("必须能找到第二个绑定的路径");
    assert_eq!(
        second_path.path,
        vec![2usize],
        "⚠️ 第二个绑定在 div 的**第三个** child（Fragment 之后）⇒ ⇒ 路径 [2]"
    );

    // ③ 路径必须**真的指向**那个绑定元素（用引擎的路径定位核对 ⇒ 不是自说自话）
    renderer.mount(view).expect("挂载必须成功");
    // ⭐ `node_for_path` 的路径根 = **tree 根 = 挂载的 VNode 根**
    //   ⇒ ⇒ 与 `collect_binding_paths` 的路径**同一个根定义** ⇒ ⇒ **可直接互喂**（无需换算）。
    let node = renderer
        .node_for_path(&second_path.path)
        .expect("路径必须能定位到节点");
    assert_eq!(
        renderer.dom().property(node, "value").as_deref(),
        Some("二"),
        "⭐ 路径定位到的节点必须就是那个绑定元素（property 值即其信号值）"
    );
}

/// 建一个渲染器与根节点（内存后端 ⇒ native 可跑）。
fn renderer_with_dom() -> (
    Renderer<apeireth_ui_dom::dom::InMemoryDom>,
    apeireth_ui_dom::dom::InMemoryNodeId,
) {
    let dom = apeireth_ui_dom::dom::InMemoryDom::new();
    let root = dom.create_element("main").expect("建根必须成功");
    let renderer = Renderer::new(dom, root);
    (renderer, root)
}
