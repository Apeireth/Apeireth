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

/// ⭐ **单调微秒时钟**（native）。
///
/// 为什么不写在引擎里：本 workspace 的 **wasm** 环境里 `std::time::Instant::now()`
/// **会 panic**（E-137 实测）⇒ ⇒ 时钟必须由**宿主/探针**注入
/// ⇒ ⇒ 引擎只管调用（见 `Renderer::set_segment_clock`）。
fn micros_clock() -> u128 {
    use std::sync::OnceLock;
    use std::time::Instant;
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_micros()
}

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
    // ⭐ 打开**段级计时**（默认关闭 ⇒ 零开销；打开后每次 `patch_node` 会累加到 `SegmentTimings`）。
    renderer.set_segment_clock(Some(micros_clock));
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
        renderer.reset_segment_timings();
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
    // ⭐⭐ **段级归因**（E-154）：把中位数拆成 7 段 ⇒ ⇒ 看谁是**大头**。
    //
    // ⚠️ `children` 这一段**含递归** ⇒ ⇒ 它必然吃掉子树的时间
    // ⇒ ⇒ 所以「children 占比高」本身不是结论，要看**剩下的直接段**谁大。
    let seg = renderer.segment_timings();
    let total = seg.attrs
        + seg.handlers
        + seg.rebind
        + seg.children
        + seg.props
        + seg.bindings
        + seg.rebuild;
    let pct = |v: u128| {
        if total == 0 {
            0.0
        } else {
            v as f64 * 100.0 / total as f64
        }
    };
    println!("── 段级归因（最后一次，累计 µs / 占比）──");
    println!(
        "   attrs     {:>10} µs  {:>5.1}%",
        seg.attrs,
        pct(seg.attrs)
    );
    println!(
        "   handlers  {:>10} µs  {:>5.1}%",
        seg.handlers,
        pct(seg.handlers)
    );
    println!(
        "   rebind    {:>10} µs  {:>5.1}%",
        seg.rebind,
        pct(seg.rebind)
    );
    println!(
        "   children  {:>10} µs  {:>5.1}%  ⚠️ 含递归",
        seg.children,
        pct(seg.children)
    );
    println!(
        "   props     {:>10} µs  {:>5.1}%",
        seg.props,
        pct(seg.props)
    );
    println!(
        "   bindings  {:>10} µs  {:>5.1}%",
        seg.bindings,
        pct(seg.bindings)
    );
    println!(
        "   rebuild   {:>10} µs  {:>5.1}%",
        seg.rebuild,
        pct(seg.rebuild)
    );
    println!(
        "   ── 段合计 {:>10} µs（⚠️ **大于** patch 时间 = 嵌套造成的，见下）",
        total
    );

    // ⭐⭐ **嵌套陷阱与正确读法**（必读，否则会误判）
    //
    // `children` 段包的是 `patch_children`，而它**递归**调回 `patch_node`
    // ⇒ ⇒ 同一棵子树的耗时会被它的**每个祖先各累加一次**
    // ⇒ ⇒ 所以「段合计 > patch 总时间」是**必然的**，`children` 的占比**不可直接读**。
    //
    // ✅ 正确读法：
    //   ① 把**直接段**求和（不含 `children`）⇒ 这是「真正逐节点做的细活」；
    //   ② 用 `patch 中位数 − Σ直接段` ⇒ 得到 **`patch_children` 自身逻辑**的耗时
    //      （key 扫描 / 位置配对 / 块锚点计算 / `Vec` 构建）。
    let direct = seg.attrs + seg.handlers + seg.rebind + seg.props + seg.bindings + seg.rebuild;
    let patch_total = samples[samples.len() / 2];
    let children_self = patch_total.saturating_sub(direct);
    println!("── ⭐ 正确读法（去嵌套）──");
    println!(
        "   直接段合计 {:>10} µs  {:>5.1}%  （attrs/handlers/rebind/props/bindings/rebuild）",
        direct,
        direct as f64 * 100.0 / patch_total as f64
    );
    println!(
        "   children 自身 {:>10} µs  {:>5.1}%  ⭐ ← **真大头**（key 扫描 / 位置配对 / 锚点 / Vec）",
        children_self,
        children_self as f64 * 100.0 / patch_total as f64
    );
    println!("   （patch 中位数 {patch_total} µs）");

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
