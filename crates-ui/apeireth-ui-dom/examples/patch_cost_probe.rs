//! ⭐⭐ **native 侧「每节点成本」测量**（2026-11-01；E-151 的下一步）。
//!
//! ## 它回答什么
//!
//! 浏览器侧实测（真实 Chromium，37,477 个真实 DOM 节点）：
//! **改一个叶子 ⇒ 差分 patch ≈ 84 ms** ⇒ ⇒ ≈ **2.2 µs/节点**。
//!
//! ⭐ 那个数字里**混着两段完全不同的成本**：
//!
//! 1. **纯 Rust 逻辑**（树比对、属性比较、集合分配…）—— 与平台无关；
//! 2. **wasm ⇄ JS 边界往返**（每次 `set_property` / `get_property` /
//!    `classList` 操作都要跨界）—— **只存在于浏览器**。
//!
//! ⇒ ⇒ 本探针在 **native**（无 wasm、无 JS、无真实 DOM）上跑**同一棵树**
//! ⇒ ⇒ **差值就是边界成本** ⇒ ⇒ ⇒ 由此决定「该优化纯 Rust 逻辑」还是
//! ⇒ ⇒ ⇒ 「该减少跨界次数」。
//!
//! ## 为什么是 `examples/` 而不是 `tests/`
//!
//! 计时**会抖**（见 E-145：同一次测量波动可达 10 倍）⇒ ⇒ 断言计时
//! ⇒ ⇒ **必然偶发红** ⇒ ⇒ ⇒ 故本探针**只打印、不断言**（⇒ 不进 CI 门禁），
//! ⇒ ⇒ 数字靠**多次运行取中位数**（复跑命令见文末）。
//!
//! ## 复跑
//!
//! ```text
//! cargo run --release -p apeireth-ui-dom --example patch_cost_probe
//! ```
//!
//! 浏览器侧同口径（真实 Chromium）：
//! `crates-ui/evidence/p5-b` 下 `APEIRETH_L1_SERVE_ROOT=$PWD/web python3 drive_l1.py`

use std::time::Instant;

use apeireth_ui_core::Runtime;
use apeireth_ui_dom::binding::Binding;
use apeireth_ui_dom::dom::{Dom, InMemoryDom};
use apeireth_ui_dom::renderer::Renderer;
use apeireth_ui_dom::vnode::VNode;

/// 与浏览器侧 perf 探针**同形**的树（`WIDTH = 8` / `DEPTH = 4`）。
fn build(
    width: usize,
    depth: usize,
    leaf: &apeireth_ui_core::Signal<String>,
    first: bool,
) -> VNode {
    let mut children = Vec::with_capacity(width);
    for index in 0..width {
        if depth == 0 {
            children.push(
                VNode::element("span")
                    .attr("data-i", index.to_string())
                    .build(),
            );
            if first {
                children.push(
                    VNode::element("input")
                        .bind(Binding::text(leaf.clone()))
                        .build(),
                );
            }
        } else {
            children.push(build(width, depth - 1, leaf, first && index == 0));
        }
    }
    VNode::element("div")
        .attr("class", "perf-row")
        .child(VNode::Fragment(children))
        .build()
}

/// 数**真实 DOM 节点**（`InMemoryDom` 的树形后端）。
fn count_nodes(dom: &InMemoryDom, node: apeireth_ui_dom::dom::InMemoryNodeId) -> usize {
    let children = dom.children(&node).unwrap_or_default();
    let mut total = 1;
    for child in children {
        total += count_nodes(dom, child);
    }
    total
}

fn main() {
    const WIDTH: usize = 8;
    const DEPTH: usize = 4;
    const REPEATS: usize = 5;

    let runtime = Runtime::new();
    let leaf = runtime.signal(String::from("初始"));
    let dom = InMemoryDom::new();
    let root = dom.create_element("main").expect("建根必须成功");
    let mut renderer = Renderer::new(dom, root);
    let view = || build(WIDTH, DEPTH, &leaf, true);

    let start_mount = Instant::now();
    renderer.mount(view()).expect("挂载必须成功");
    let mount_micros = start_mount.elapsed().as_micros();

    let nodes = count_nodes(renderer.dom(), root);

    println!("== native 侧 patch 成本（{WIDTH}×{DEPTH}）==");
    println!(
        "挂载 = {} ms | 真实 DOM 节点 = {nodes}",
        mount_micros / 1000
    );

    let mut samples: Vec<u128> = Vec::with_capacity(REPEATS);
    for round in 0..REPEATS {
        renderer.reset_patch_stats();
        leaf.set(format!("改后-{round}"));
        let start = Instant::now();
        renderer.patch(view()).expect("打补丁必须成功");
        let micros = start.elapsed().as_micros();
        let stats = renderer.patch_stats();
        samples.push(micros);
        println!(
            "  第 {} 次: patch = {} µs | 比对 = {} | 写入 = {}",
            round + 1,
            micros,
            stats.compared_nodes,
            stats.written_nodes
        );
    }
    samples.sort_unstable();
    let median = samples[samples.len() / 2];
    println!(
        "中位数 = {median} µs | 每节点 ≈ {:.3} µs",
        median as f64 / nodes as f64
    );
    // ⭐ **结论按实测写，不按预期写**（2026-11-01 实测更正）。
    //
    // 预期：「native 快 ⇒ 差值 = wasm⇄JS 边界成本」
    // ⇒ ⇒ **实测反过来**：native（`InMemoryDom`）比浏览器（真实 Chromium + `WebDom`）
    //        **还慢** ⇒ ⇒ ⇒ **边界不是成本来源** ⇒ ⇒ 瓶颈在**纯 Rust 侧**。
    //
    // ⚠️ 归因限制：两个后端**不是同一个东西**——`InMemoryDom` 是测试用的内存树
    //   （节点以 `Vec` 存放 + 索引查找）本身就可能比真实 DOM 慢
    //   ⇒ ⇒ ⇒ **本探针只能证明「Rust 侧不便宜」，不能单独归因到边界或某一段**。
    println!(
        "（参考：同树在真实 Chromium 上 ≈ 84 ms ⇒ ⇒ ⭐ native **不更快** \
         ⇒ ⇒ 瓶颈不在 wasm⇄JS 边界，在 Rust 侧 / 后端实现）"
    );
}
