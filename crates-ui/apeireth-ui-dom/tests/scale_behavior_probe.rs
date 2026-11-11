//! ⭐ **规模下的行为正确性**（独立回归测试；不读生产源码文本）。
//!
//! ## 它钉住什么
//!
//! 引擎的更新路径是「整树重建 → 差分 patch」⇒ ⇒ 树越大成本越高
//! ⇒ ⇒ **规模下的行为**（不是性能数字）必须仍然正确：
//! 改**一个叶子** ⇒ ⇒ **只有那个叶子**变，DOM **不增删节点**。
//!
//! ⚠️ 这里**不断言时间**（时间会抖 ⇒ ⇒ 不是门禁）；
//! ⇒ ⇒ 性能数字由浏览器侧的规模基线探针给出（`l1_perf_probe`）。

use apeireth_ui_core::Runtime;
use apeireth_ui_dom::binding::Binding;
use apeireth_ui_dom::dom::{Dom, InMemoryDom, InMemoryNodeId};
use apeireth_ui_dom::renderer::Renderer;
use apeireth_ui_dom::vnode::VNode;

/// 递归数节点（差分 patch 若整棵重刷，节点数会变 ⇒ ⇒ 可观测）。
fn count_nodes(dom: &InMemoryDom, node: InMemoryNodeId) -> usize {
    let children = dom.children(&node).unwrap_or_default();
    let mut total = 1;
    for child in children {
        total += count_nodes(dom, child);
    }
    total
}

/// ⭐ 改一个叶子 ⇒ 只有它变（规模：6 个兄弟节点 + 1 个绑定输入框）。
#[test]
fn updating_one_leaf_leaves_the_rest_untouched() {
    let runtime = Runtime::new();
    let leaf = runtime.signal(String::from("初始"));
    let dom = InMemoryDom::new();
    let root = dom.create_element("main").expect("建根必须成功");
    let mut renderer = Renderer::new(dom, root);

    /// 一行：N 个 span（各带文本 "x"）+ 第一个 span 后跟一个**绑定**的 input。
    fn build(width: usize, leaf: &apeireth_ui_core::Signal<String>, first: bool) -> VNode {
        let mut children = Vec::with_capacity(width + 1);
        for index in 0..width {
            children.push(
                VNode::element("span")
                    .attr("data-i", index.to_string())
                    .child(VNode::text("x"))
                    .build(),
            );
            if first && index == 0 {
                children.push(
                    VNode::element("input")
                        .bind(Binding::text(leaf.clone()))
                        .build(),
                );
            }
        }
        VNode::element("div")
            .attr("class", "row")
            .child(VNode::Fragment(children))
            .build()
    }

    let view = || build(6, &leaf, true);
    renderer.mount(view()).expect("挂载必须成功");

    // 找到那个带绑定的 input：root > div.row > (span×N, input)
    let row = renderer.dom().children(&root).expect("读子节点必须成功")[0];
    let row_children = renderer.dom().children(&row).expect("读行子节点必须成功");
    // ⭐ 找那个**带 value property** 的节点（`Fragment` 不产生节点 ⇒ ⇒ input 不在末尾）。
    let input = *row_children
        .iter()
        .find(|node| renderer.dom().property(**node, "value").is_some())
        .expect("行内必须有带 value property 的 input");

    // ① 首屏：property 写入生效
    //    ⚠️ `value` 是 **property**，**不在** `text_content` 里（观测手段要对）。
    assert_eq!(
        renderer.dom().property(input, "value").as_deref(),
        Some("初始"),
        "首屏必须把信号值写进 property"
    );

    let before_nodes = count_nodes(renderer.dom(), root);

    // ② 改一个叶子 ⇒ **只有它**变
    leaf.set(String::from("改后"));
    renderer.patch(view()).expect("打补丁必须成功");

    assert_eq!(
        renderer.dom().property(input, "value").as_deref(),
        Some("改后"),
        "更新后 property 必须是新值"
    );
    assert_eq!(
        before_nodes,
        count_nodes(renderer.dom(), root),
        "⭐ 改一个叶子 ⇒ **DOM 节点数不变**（差分 patch 不是整棵重刷 ⇒ 不增删节点）"
    );
}
