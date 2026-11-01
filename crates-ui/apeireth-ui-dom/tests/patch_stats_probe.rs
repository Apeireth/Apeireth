//! ⭐⭐ **patch 统计**（独立回归测试；不读生产源码文本）。
//!
//! ## 它钉住什么
//!
//! 实测（E-137）：4.7k 元素的树里，**改一个叶子**要 **91 ms**（构造仅 2 ms）
//! ⇒ ⇒ 瓶颈是**差分 patch 的比对范围** ⇒ ⇒ 要判断「收窄范围」是否有效，
//! ⇒ ⇒ 需要一个**不抖**的观测量 ⇒ ⇒ **计数**（`PatchStats`）。
//!
//! ## 核心断言
//!
//! 改**一个**叶子 ⇒ ⇒ **比对**了**整棵树**（这正是要优化的点）
//! ⇒ ⇒ 但**写入**只有**极少**次（差分是对的：没变的就不碰 DOM）。
//! ⇒ ⇒ 这两个数字合起来 = 「**该优化的是比对范围，不是写入**」。

use apeireth_ui_core::Runtime;
use apeireth_ui_dom::binding::Binding;
use apeireth_ui_dom::dom::{Dom, InMemoryDom};
use apeireth_ui_dom::renderer::Renderer;
use apeireth_ui_dom::vnode::VNode;

#[test]
fn patch_stats_separate_comparisons_from_writes() {
    let runtime = Runtime::new();
    let leaf = runtime.signal(String::from("初始"));
    let dom = InMemoryDom::new();
    let root = dom.create_element("main").expect("建根必须成功");
    let mut renderer = Renderer::new(dom, root);

    /// 一行：N 个 span + 第一个 span 后的**绑定** input。
    fn build(width: usize, leaf: &apeireth_ui_core::Signal<String>, first: bool) -> VNode {
        let mut children = Vec::with_capacity(width + 1);
        for index in 0..width {
            children.push(VNode::element("span").child(VNode::text("x")).build());
            if first && index == 0 {
                children.push(
                    VNode::element("input")
                        .bind(Binding::text(leaf.clone()))
                        .build(),
                );
            }
        }
        VNode::element("div")
            .child(VNode::Fragment(children))
            .build()
    }

    let view = || build(8, &leaf, true);
    renderer.mount(view()).expect("挂载必须成功");

    // ⭐ 只统计**这一次** patch。
    renderer.reset_patch_stats();
    leaf.set(String::from("改后"));
    renderer.patch(view()).expect("打补丁必须成功");
    let stats = renderer.patch_stats();

    assert!(
        stats.compared_nodes >= 8,
        "⭐ 差分 patch 必须**比对**了树里的节点（实际 {}）",
        stats.compared_nodes
    );
    assert!(
        stats.written_nodes >= 1,
        "改一个叶子 ⇒ 至少要写 1 次（实际 {}）",
        stats.written_nodes
    );
    assert!(
        stats.written_nodes < stats.compared_nodes,
        "⭐ **写入**必须远少于**比对**（比对 {} vs 写入 {}）⇒ ⇒ 该优化的是**比对范围**",
        stats.compared_nodes,
        stats.written_nodes
    );
}
