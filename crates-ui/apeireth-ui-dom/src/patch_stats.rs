//! ⭐⭐ **patch 统计**（差分**比对**了多少节点 / 实际**写入**了多少次）。
//!
//! ## 为什么要有它（E-137 的数字结论）
//!
//! 实测：4.7k 元素的树里，**改一个叶子**要 **94 ms**，其中**构造仅 3 ms**
//! ⇒ ⇒ 89+ ms 在「差分 patch」⇒ ⇒ **写入只有 8 次**、**比对却有 42,138 次**
//! ⇒ ⇒ 差分**写得对**，但**比得太多** ⇒ ⇒ 该优化的是**比对范围**。
//!
//! ## 为什么用**计数**而不是计时
//!
//! 时间**会抖**（机器 / 负载 / 浏览器版本）⇒ ⇒ 不能当门禁；
//! ⇒ ⇒ **计数是确定的** ⇒ ⇒ 优化前后可直接对比 ⇒ ⇒ 「收窄比对范围」**可验证**。

/// ⭐⭐ **patch 统计**。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PatchStats {
    /// 差分遍历**比对**过的节点数（元素 + 文本）。
    pub compared_nodes: usize,
    /// 实际**写入** DOM 的次数（属性 / property / 文本 / 绑定写回）。
    pub written_nodes: usize,
}

/// ⭐ **段级耗时**（可选；默认**关闭** ⇒ 零开销）。
///
/// ## 为什么需要
///
/// 实测（E-153）：同一棵树 native patch ≈ **143~150 ms**、浏览器 ≈ **84 ms**
/// ⇒ ⇒ 已排除 6 个候选（读 DOM / 重复遍历 / 子树克隆 / 属性分配 / class 分配 /
/// wasm⇄JS 边界）⇒ ⇒ **但还不知道时间花在哪一段**
/// ⇒ ⇒ 本结构把 `patch_node` 的 6 段**分开累加**。
///
/// ## 为什么默认关闭
///
/// 取时间本身有成本（`clock()` 两次调用 / 每次节点）
/// ⇒ ⇒ 生产路径**不能**一直开着 ⇒ ⇒ 只在测量时由宿主注入时钟
/// （[`crate::renderer::Renderer::set_segment_clock`]）。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SegmentTimings {
    /// `patch_attributes`：class token + 属性比对/写入。
    pub attrs: u128,
    /// `patch_handlers`：事件处理器比对。
    pub handlers: u128,
    /// `rebind_this`：`bind:this` 槽位重绑。
    pub rebind: u128,
    /// `patch_children`：子节点差分（**含递归** ⇒ 天然是大头候选）。
    pub children: u128,
    /// `patch_props`：property 比对/写入。
    pub props: u128,
    /// `apply_bindings`：绑定写回（信号 → DOM）。
    pub bindings: u128,
    /// 重建 `MountedNode`（结构体 + `Vec` 分配）。
    pub rebuild: u128,
    /// `ensure_children_keyed_decidable`（keyed 差分**可判定性**校验）。
    ///
    /// ⚠️ 这一段**在 `patch_node` 的计时段之外**被调用（2026-11-02 补计时）
    /// ⇒ ⇒ 此前它被**误算进**「children 自身」⇒ ⇒ 本字段把它**单列**出来。
    pub ensure: u128,
}
