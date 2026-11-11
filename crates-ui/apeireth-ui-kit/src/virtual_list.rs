//! 基础组件：虚拟列表（窗口化列表）。
//!
//! ## 为什么要有这个组件
//!
//! 长列表（会话记录 / 工具清单 / 设置项）要能滚动 ⇒ ⇒ 但「**只渲染可见的那几行**」
//! 是**引擎的窗口化原语**（[`apeireth_ui_dom::virtual_list::plan_virtual_window`]
//! + `Renderer::patch_windowed_at_path`）的活 ⇒ ⇒ 组件层**不重算窗口**，而是：
//!
//! 1. **出结构**：滚动容器（class 由调用方给）+ 内容层（当前窗口的行 / 空态）；
//! 2. **出行**：把 `render_item` 的产物包进「行 class + key」的行节点
//!    （**key 是 keyed 差分的地基** —— 没有它，窗口平移时整片行按位置重建）；
//! 3. **交参数**：把行高 / overscan / 高度向量整理好，递给引擎那一侧。
//!
//! ## ⭐ 边界：组件与引擎原语的分工
//!
//! | 在本组件 | 在引擎（`apeireth-ui-dom`） |
//! |---|---|
//! | 容器结构、class 契约、行节点（class + key + 内容）、空态 | 窗口计算（`plan_virtual_window`）、前后留白、写入窗口内容、`virtualized` 标记 |
//!
//! ⇒ ⇒ **组件不调 `plan_virtual_window`**（三条理由）：
//!
//! - ① **窗口取决于真实滚动状态**（`scrollTop` / `clientHeight`）⇒ ⇒ 只有渲染器
//!   读得到（`Renderer::virtual_window_for` 链到宿主的真实 DOM 度量）
//!   ⇒ ⇒ 纯函数组件拿不到 ⇒ ⇒ 硬算就是「猜滚动位置」；
//! - ② **窗口内容必须由引擎写**（`patch_windowed_at_path` 一次做完三件事：
//!   写**前后留白** + 写窗口行 + 给容器打 `virtualized` 标记）
//!   ⇒ ⇒ 组件自己再算一遍 ⇒ ⇒ 树里的窗口与 DOM 里的窗口可能不一致
//!   ⇒ ⇒ 那正是 E-161（窗口塌陷）/ E-164（滚动位置丢失）的根；
//! - ③ **组件层不做布局**（ADR-0015）：留白高度是布局值 ⇒ ⇒ 归引擎算
//!   ⇒ ⇒ 组件**不写内联样式**（组件契约 §2.3 C3）。
//!
//! ## 结构
//!
//! ```text
//! div                       ← 滚动容器（class = 调用方给的 class）
//! │  data-scroll-region="" ← 滚动区钩子（宿主 / 探针定位用；**非视觉**）
//! │
//! ├ div                     ← 当前窗口的行（class = 调用方给的行 class；带 key）
//! ├ div
//! └ …
//! ```
//!
//! ⚠️ **行的父级就是滚动容器本身**（不套中间层）—— 因为引擎的窗口化
//! （`patch_windowed_at_path`）会把**滚动容器的子级**整层替换成
//! 「前留白 + 窗口行 + 后留白」⇒ ⇒ 中间多一层 ⇒ 那层会被替换掉
//! ⇒ ⇒ CSS 的子选择器（`.容器 > .行`）会在窗口化之后失效。
//!
//! ## class 名由调用方给
//!
//! 设计文档 §9.1：class 名是**视觉契约的一部分**（CSS 资产按这些名字写死，
//! 改名即改视觉）⇒ ⇒ 组件库**不发明** class 名（容器 class 与行 class
//! 都由调用方给）。
//!
//! ## 刻意不加的
//!
//! - **不写内联样式**（设计文档 §9.2 C3）⇒ ⇒ 留白的高度由引擎写，不由组件写；
//! - **不设 `id`**（设计文档 §9.2 C4）⇒ ⇒ 定位靠 class 与 `data-*` 钩子；
//! - **不自动打 `virtualized` 标记** ⇒ ⇒ 引擎的窗口化会**自动**打
//!   （E-165）⇒ ⇒ 组件手动打反而会让「整树 patch 跳过子级」提前生效
//!   ⇒ ⇒ 静态列表（不走窗口化）的行就再也更新不了。
//!
//! ## 🔵 未接线
//!
//! - 本组件**尚无生产消费方**（没有任何屏幕用它）⇒ ⇒ 结构与行节点已由
//!   native 单测钉住，但**未在真实浏览器里上屏**；
//! - **不等高行**未支持（`row_height` 是**一个**高度 ⇒ 引擎那一侧按等高算）；
//! - **sticky header** 未支持（引擎的窗口化只写「留白 + 行」）。

use std::marker::PhantomData;

use apeireth_ui_core::UiComponent;
use apeireth_ui_dom::dom::Dom;
use apeireth_ui_dom::virtual_list::VirtualPlan;
use apeireth_ui_dom::{DomError, Renderer, VNode};

/// ⭐ **默认预渲染余量**（上下各多渲染几行）。
///
/// 为什么是 2：窗口平移时多渲染两行 ⇒ ⇒ 滚动条一落稳，新行已经画好
/// ⇒ ⇒ 快速滚动不闪白。**这是行数，不是时长 / 尺寸** ⇒ ⇒ 不违反
/// 「组件层不出现时长与颜色字面量」的纪律。
pub const DEFAULT_OVERSCAN: usize = 2;

/// 当前窗口：行下标的**闭区间**。
///
/// ## 为什么组件要「知道窗口」
///
/// 组件是纯函数（`fn(&Props) -> VNode`）⇒ ⇒ 它**不知道**此刻滚到了哪里
/// ⇒ ⇒ 窗口必须由**调用方**用引擎算好后**递进来**（[`WindowRange::from_plan`]
/// 就是从引擎的 [`VirtualPlan`] 取可见区间的那一步）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowRange {
    /// 第一个可见行下标（**含**）。
    pub first: usize,
    /// 最后一个可见行下标（**含**）。
    pub last: usize,
}

impl WindowRange {
    /// 建一个窗口（调用方保证 `first <= last`）。
    pub fn new(first: usize, last: usize) -> Self {
        Self { first, last }
    }

    /// 窗口里的行数（空窗口 ⇒ 0）。
    pub fn len(&self) -> usize {
        if self.is_empty() {
            0
        } else {
            self.last - self.first + 1
        }
    }

    /// 窗口是否为空（`last < first`）。
    pub fn is_empty(&self) -> bool {
        self.last < self.first
    }

    /// ⭐ **从引擎的窗口计划取可见区间**（**空窗口 ⇒ `None`**）。
    ///
    /// 用法：`WindowRange::from_plan(&plan)` —— `plan` 来自
    /// `Renderer::virtual_window_for` / `patch_windowed_at_path`。
    pub fn from_plan(plan: &VirtualPlan) -> Option<Self> {
        if plan.is_empty() {
            None
        } else {
            Some(Self {
                first: plan.first,
                last: plan.last,
            })
        }
    }
}

/// 虚拟列表组件的 props（**泛型**：`T` = 行的数据类型）。
///
/// ## 为什么是泛型（而不是 `Vec<VNode>`）
///
/// - 组件层**不含业务语义**（设计文档 §1.3）⇒ ⇒ `T` 由调用方定
///   （会话行 / 工具行 / 设置行……）⇒ ⇒ 组件不需要「先渲染成 VNode 再交进来」
///   的中转，也就不必把业务数据提前变成节点；
/// - **`key_of` 需要业务身份**（哪一行的稳定标识）⇒ ⇒ 泛型才能让调用方
///   从 `T` 里取身份（组件自己猜不出「这一行的 key 是什么」）。
///
/// ## 为什么 `key_of` / `render_item` 是**函数指针**（不是闭包）
///
/// - props 要保持**纯数据**（可 `Clone` / `Debug` / `PartialEq`）⇒ ⇒ 函数指针
///   三件套齐全，闭包（捕获了状态）就不行；
/// - 函数指针**不能捕获状态** ⇒ ⇒ 需要状态的渲染方把状态**放进 `T`**
///   （这正是「数据归调用方、组件只管长什么样」的纪律）。
// ⚠️ **不 derive `PartialEq` / `Eq`**：
// - `row_height` 是 `f64` ⇒ ⇒ `f64: !Eq`；
// - props 里有**函数指针** ⇒ ⇒ 函数指针的比较**没有意义**（同一函数在不同
//   编译单元里地址可能不同 ⇒ rustc 的 `unpredictable_function_pointer_comparisons`
//   直接报错）⇒ ⇒ 「同样的 props ⇒ 同样的产物」这条纯函数纪律
// ⇒ ⇒ 改由**产物**（`VNode`）比较来断言（见 `the_component_is_pure`）。
#[derive(Debug, Clone)]
pub struct VirtualListProps<T> {
    /// 滚动容器的 class 名（调用方给；**视觉契约**）。
    pub class_name: String,
    /// 每一行的 class 名（调用方给；**视觉契约**）。
    pub item_class_name: String,
    /// 行数据（**全量**；窗口只决定「渲染哪几行」，不决定「有哪些行」）。
    pub items: Vec<T>,
    /// 行高（CSS 像素）—— **交给引擎的窗口计算那一侧**（`plan_virtual_window`）。
    pub row_height: f64,
    /// 预渲染余量（上下各多渲染几行）—— 同样交给引擎那一侧。
    pub overscan: usize,
    /// 行 key（**keyed 差分的地基**）。
    pub key_of: fn(&T) -> String,
    /// 行内容。
    pub render_item: fn(&T) -> VNode,
    /// 空态节点（`items` 为空时渲染；`None` ⇒ 渲染**空容器**）。
    pub empty: Option<VNode>,
    /// 当前窗口（行下标闭区间）—— 调用方用引擎算好后传入。
    ///
    /// `None` ⇒ **只出结构、不出行**（组件**不猜**窗口）。
    pub window: Option<WindowRange>,
}

impl<T> VirtualListProps<T> {
    /// 建一组虚拟列表 props。
    ///
    /// ⚠️ **两个 class 名必给**（容器 + 行）⇒ ⇒ 组件不发明 class 名；
    /// ⚠️ **`key_of` / `render_item` 必给** ⇒ ⇒ 编译期就挡住「忘了给 key」
    /// （忘了 ⇒ keyed 差分退化成按位置重建 ⇒ 窗口平移时行内的焦点 / 过渡态会丢）。
    pub fn new(
        class_name: impl Into<String>,
        item_class_name: impl Into<String>,
        row_height: f64,
        key_of: fn(&T) -> String,
        render_item: fn(&T) -> VNode,
    ) -> Self {
        Self {
            class_name: class_name.into(),
            item_class_name: item_class_name.into(),
            items: Vec::new(),
            row_height,
            overscan: DEFAULT_OVERSCAN,
            key_of,
            render_item,
            empty: None,
            window: None,
        }
    }

    /// 设行数据（builder 风格）。
    pub fn items(mut self, items: Vec<T>) -> Self {
        self.items = items;
        self
    }

    /// 设预渲染余量（builder 风格）。
    pub fn overscan(mut self, overscan: usize) -> Self {
        self.overscan = overscan;
        self
    }

    /// 设行 key（builder 风格）。
    pub fn key_of(mut self, key_of: fn(&T) -> String) -> Self {
        self.key_of = key_of;
        self
    }

    /// 设行内容（builder 风格）。
    pub fn render_item(mut self, render_item: fn(&T) -> VNode) -> Self {
        self.render_item = render_item;
        self
    }

    /// 设空态节点（builder 风格）。
    pub fn empty(mut self, empty: Option<VNode>) -> Self {
        self.empty = empty;
        self
    }

    /// 设当前窗口（builder 风格）。
    pub fn window(mut self, window: Option<WindowRange>) -> Self {
        self.window = window;
        self
    }

    /// 就地改当前窗口（builder 风格；**接线时每帧都要用**）。
    ///
    /// ⚠️ 为什么要有「就地改」：滚动后引擎算出的新窗口要**回灌**给下一次
    /// 整树重建（E-164 的正解：「树里的列表 = 当前窗口」）⇒ ⇒ 否则树里的
    /// 列表与 DOM 里的窗口会不一致 ⇒ ⇒ 窗口会来回跳。
    pub fn set_window(&mut self, window: Option<WindowRange>) {
        self.window = window;
    }

    /// ⭐ **一行的虚拟节点**（**行 class + key + 内容**）。
    ///
    /// 越界下标 ⇒ `None`（**不 panic**：组件不许把渲染栈搞崩）。
    pub fn row(&self, index: usize) -> Option<VNode> {
        let item = self.items.get(index)?;
        let row = VNode::element("div")
            .class(self.item_class_name.clone())
            .key((self.key_of)(item))
            .child((self.render_item)(item))
            .build();
        Some(row)
    }

    /// 窗口内的所有行（越界的下标**跳过**）。
    pub fn rows(&self, window: WindowRange) -> Vec<VNode> {
        (window.first..=window.last)
            .filter_map(|index| self.row(index))
            .collect()
    }

    /// ⭐ **等高行的高度向量**（喂引擎的窗口计算那一侧）。
    ///
    /// ⚠️ 这里**不做净化**（NaN / 负高度照原样给）⇒ ⇒ 净化由引擎的
    /// `plan_virtual_window` 负责（它把非法值按 0 处理）⇒ ⇒ **一份真源**。
    pub fn heights(&self) -> Vec<f64> {
        vec![self.row_height; self.items.len()]
    }
}

/// ⭐ **虚拟列表组件**（纯函数 `fn(&Props) -> VNode`）。
///
/// ## 产出什么
///
/// - **滚动容器**：`div` + 调用方给的 class + `data-scroll-region` 钩子；
/// - **内容层**（容器的子级）：
///   - `items` 为空 ⇒ **空态**（给了 [`VirtualListProps::empty`] 才渲染；
///     没给 ⇒ **空容器**）；
///   - `items` 非空 ⇒ **当前窗口的行**（窗口未知 ⇒ **只出结构，不出行**）。
///
/// ## 为什么「窗口未知就不出行」
///
/// 虚拟列表的意义就是「不渲染看不见的行」⇒ ⇒ 组件宁可**少画**也不**多画**
/// ⇒ ⇒ 窗口没算出来之前不猜（猜 ⇒ 长列表一次性建出上万个节点 ⇒ 正是
/// 虚拟化要躲开的事）。
pub fn virtual_list<T>(props: &VirtualListProps<T>) -> VNode {
    let mut container = VNode::element("div")
        .class(props.class_name.clone())
        // ⭐ 滚动区钩子：**非视觉**（不违反「class 名是视觉契约」的纪律）
        // ⇒ ⇒ 宿主 / 探针可以按它定位滚动容器（`scroll` 不冒泡 ⇒ 必须能定位）。
        .attr("data-scroll-region", "");

    if props.items.is_empty() {
        // 空态：有就渲染，没有就留空容器。
        if let Some(empty) = &props.empty {
            container = container.child(empty.clone());
        }
        return container.build();
    }

    if let Some(window) = props.window {
        for row in props.rows(window) {
            container = container.child(row);
        }
    }
    container.build()
}

/// ⭐ **接线辅助**（**不属于纯组件**）：用引擎的窗口化渲染把「当前窗口」写进滚动容器。
///
/// ## 为什么单独一个函数（而不是塞进组件）
///
/// 组件是纯函数（设计文档 §3.3.5 / 组件契约 §2.1）⇒ ⇒ **不许**在渲染里碰
/// 渲染器 ⇒ ⇒ 接线（读真实滚动状态 → 算窗口 → 写窗口内容 → 打 `virtualized`
/// 标记）归**引擎**（`Renderer::patch_windowed_at_path`）
/// ⇒ ⇒ 本函数只做一件事：**把组件的三样东西递给引擎** ——
/// 高度向量、overscan、行工厂。
///
/// ## 用法
///
/// ```ignore
/// let mut props = VirtualListProps::new("会话滚动区", "会话行", 40.0, row_key, row_content)
///     .items(rows);
/// renderer.render_guarded(virtual_list(&props))?;      // 首次挂载（容器在、行待算）
/// // 滚动后：
/// let plan = patch_windowed(&mut renderer, &[0], &props)?;   // 引擎按真实滚动状态算窗口
/// props.set_window(WindowRange::from_plan(&plan));           // 回灌 ⇒ 下一帧的树带同一个窗口
/// ```
///
/// ## 返回什么
///
/// 引擎实际用的 [`VirtualPlan`]（窗口区间 + 偏移）—— 调用方据此回灌窗口。
///
/// # Errors
///
/// 引擎侧失败（度量未定 / 后端不支持滚动度量 / 路径不存在）⇒ ⇒ **原样返回**
/// （**不吞错**：窗口化失败必须让调用方看见，否则列表会静默塌成一行）。
pub fn patch_windowed<D, T>(
    renderer: &mut Renderer<D>,
    path: &[usize],
    props: &VirtualListProps<T>,
) -> Result<VirtualPlan, DomError>
where
    D: Dom,
{
    renderer.patch_windowed_at_path(path, &props.heights(), props.overscan, |index| {
        // 越界理论上不会发生（引擎给的窗口下标必在数据范围内）⇒ ⇒ 但仍不 panic。
        props
            .row(index)
            .unwrap_or_else(|| VNode::fragment(Vec::new()))
    })
}

/// 虚拟列表组件（**可注册进组件注册表**；注册时带上行类型）。
///
/// ⭐ **为什么是泛型组件**：注册表按「名字 → 擦除后的 props」索引
/// ⇒ ⇒ 行类型不同 ⇒ props 类型不同 ⇒ ⇒ 注册名相同但类型不同 ⇒ ⇒
/// 必须由调用方显式指定 `T`（`registry.register_named::<VirtualList<Row>>("…")`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualList<T>(PhantomData<fn() -> T>);

impl<T> Default for VirtualList<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T> UiComponent for VirtualList<T> {
    type Props = VirtualListProps<T>;
    type Output = VNode;

    fn render(props: &Self::Props) -> Self::Output {
        virtual_list(props)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_ui_dom::dom::{ElementBox, InMemoryDom, ScrollMetrics};
    use apeireth_ui_dom::{AttrValue, Dom, Key};

    /// 测试用的行数据（**中性形状**：id + 文本）。
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Row {
        id: String,
        text: String,
    }

    fn row_key(row: &Row) -> String {
        row.id.clone()
    }

    fn row_content(row: &Row) -> VNode {
        VNode::text(row.text.clone())
    }

    fn rows(count: usize) -> Vec<Row> {
        (0..count)
            .map(|index| Row {
                id: format!("k{index}"),
                text: format!("行 {index}"),
            })
            .collect()
    }

    fn props(items: Vec<Row>) -> VirtualListProps<Row> {
        VirtualListProps::new("vl-scroll", "vl-row", 40.0, row_key, row_content).items(items)
    }

    /// 容器结构：外层 class + 滚动区钩子。
    #[test]
    fn container_carries_the_given_class_and_the_scroll_region_hook() {
        let node = virtual_list(&props(rows(3)).window(Some(WindowRange::new(0, 2))));
        assert_eq!(node.tag(), Some("div"));
        assert_eq!(
            node.attr_value("class"),
            Some(&AttrValue::Text(String::from("vl-scroll")))
        );
        assert_eq!(
            node.attr_value("data-scroll-region"),
            Some(&AttrValue::Text(String::new())),
            "滚动区钩子必须挂上（scroll 不冒泡 ⇒ 宿主要能定位容器）"
        );
    }

    /// 行结构：行 class + **行 key**（keyed 差分的地基）。
    #[test]
    fn rows_carry_the_row_class_and_the_key_from_the_key_function() {
        let node = virtual_list(&props(rows(3)).window(Some(WindowRange::new(0, 2))));
        let children = node.children();
        assert_eq!(children.len(), 3);
        for (index, child) in children.iter().enumerate() {
            assert_eq!(child.tag(), Some("div"));
            assert_eq!(
                child.attr_value("class"),
                Some(&AttrValue::Text(String::from("vl-row")))
            );
            assert_eq!(
                child.key(),
                Some(&Key::Str(format!("k{index}"))),
                "行 key 必须来自 key_of（keyed 差分靠它认人）"
            );
        }
    }

    /// `render_item` 的产物**被放在行里**（行 = class + key + 内容）。
    #[test]
    fn render_item_output_is_nested_inside_the_row() {
        let node = virtual_list(&props(rows(1)).window(Some(WindowRange::new(0, 0))));
        let row = &node.children()[0];
        assert_eq!(row.children().len(), 1);
        assert_eq!(row.children()[0], VNode::text(String::from("行 0")));
    }

    /// 空态：没有项 ⇒ 渲染空态节点。
    #[test]
    fn empty_list_renders_the_given_empty_state() {
        let node =
            virtual_list(&props(Vec::new()).empty(Some(VNode::text(String::from("还没有内容")))));
        assert_eq!(node.children().len(), 1);
        assert_eq!(node.children()[0], VNode::text(String::from("还没有内容")));
    }

    /// 空态**没给** ⇒ 渲染空容器（不留残骸）。
    #[test]
    fn empty_list_without_empty_state_renders_an_empty_container() {
        let node = virtual_list(&props(Vec::new()));
        assert_eq!(node.tag(), Some("div"));
        assert!(
            node.children().is_empty(),
            "没有空态节点 ⇒ 容器为空（不留半截结构）"
        );
    }

    /// 窗口只渲染窗口内的行（虚拟化的核心语义）。
    #[test]
    fn window_limits_the_rendered_rows() {
        let node = virtual_list(&props(rows(100)).window(Some(WindowRange::new(10, 19))));
        let children = node.children();
        assert_eq!(children.len(), 10, "窗口 10 行 ⇒ 只出 10 行");
        assert_eq!(children[0].key(), Some(&Key::Str(String::from("k10"))));
        assert_eq!(children[9].key(), Some(&Key::Str(String::from("k19"))));
    }

    /// 窗口未知 ⇒ **只出结构、不出行**（组件不猜窗口）。
    #[test]
    fn without_a_window_only_the_structure_is_rendered() {
        let node = virtual_list(&props(rows(1000)));
        assert_eq!(node.tag(), Some("div"));
        assert!(
            node.children().is_empty(),
            "窗口未知 ⇒ 不猜（长列表一次性建满 ⇒ 正是虚拟化要躲开的事）"
        );
    }

    /// 边界：0 项 / 1 项 / 很多项 ⇒ **结构不塌**。
    #[test]
    fn edge_cases_keep_the_structure_intact() {
        let empty = virtual_list(&props(Vec::new()));
        assert_eq!(empty.tag(), Some("div"));
        assert!(empty.children().is_empty());

        let one = virtual_list(&props(rows(1)).window(Some(WindowRange::new(0, 0))));
        assert_eq!(one.children().len(), 1);

        let many = virtual_list(&props(rows(1000)).window(Some(WindowRange::new(0, 19))));
        assert_eq!(
            many.children().len(),
            20,
            "1000 项 ⇒ DOM 里仍只有窗口那 20 行"
        );
    }

    /// class 名**可配置**且**逐字**出现在树上（视觉契约）。
    #[test]
    fn class_names_are_verbatim_on_the_tree() {
        let props = VirtualListProps::new("会话滚动区", "会话行", 32.0, row_key, row_content)
            .items(rows(2))
            .window(Some(WindowRange::new(0, 1)));
        let node = virtual_list(&props);
        assert_eq!(
            node.attr_value("class"),
            Some(&AttrValue::Text(String::from("会话滚动区")))
        );
        assert_eq!(
            node.children()[0].attr_value("class"),
            Some(&AttrValue::Text(String::from("会话行")))
        );
    }

    /// 纪律：组件**不写内联样式**、**不设 id**（设计文档 §9.2 C3 / C4）。
    #[test]
    fn the_component_never_writes_inline_styles_or_ids() {
        let node = virtual_list(&props(rows(3)).window(Some(WindowRange::new(0, 2))));
        assert!(node.attr_value("style").is_none(), "组件不写内联样式");
        assert!(node.attr_value("id").is_none(), "组件不设 id");
        for child in node.children() {
            assert!(child.attr_value("style").is_none());
            assert!(child.attr_value("id").is_none());
        }
    }

    /// 组件是**纯函数**：同样的 props ⇒ 同样的树。
    #[test]
    fn the_component_is_pure() {
        let props = props(rows(5)).window(Some(WindowRange::new(1, 3)));
        assert_eq!(virtual_list(&props), virtual_list(&props));
    }

    /// 高度向量：等高行 ⇒ `row_height × 行数`（喂引擎那一侧）。
    #[test]
    fn heights_match_the_row_height_and_the_item_count() {
        assert_eq!(props(rows(7)).heights(), vec![40.0; 7]);
        assert!(props(Vec::new()).heights().is_empty());
    }

    /// ⭐ **接线端到端（native，不开浏览器）**：组件的行工厂 + 高度向量
    /// 喂给引擎的窗口化 ⇒ ⇒ DOM 里只挂窗口内的行。
    #[test]
    fn the_engine_window_writes_only_the_window_rows_into_the_container() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("section").expect("建根必须成功");
        let mut renderer = Renderer::new(dom, root);
        let props = props(rows(100)).window(Some(WindowRange::new(0, 9)));
        renderer
            .render_guarded(virtual_list(&props))
            .expect("首次挂载必须成功");

        // 容器 = 树的根（组件的产物就是容器）⇒ 注入脚本化滚动度量
        // （引擎不做布局 ⇒ 度量必须显式注入）。
        let container = renderer.node_for_path(&[]).expect("容器必须可定位");
        renderer.dom().script_measurement(
            container,
            ElementBox::new(0.0, 0.0, 200.0, 200.0),
            ScrollMetrics {
                scroll_top: 400.0,
                scroll_height: 4000.0,
                client_height: 200.0,
            },
        );

        // 引擎按**真实滚动状态**算窗口并写进容器（行高 40 ⇒ 滚到 400 ⇒ 窗口从第 10 行起）。
        let plan = patch_windowed(&mut renderer, &[], &props).expect("窗口化必须成功");
        assert_eq!(plan.first, 8, "可见第 10 行 − overscan 2");
        assert_eq!(plan.last, 16, "可见到第 14 行 + overscan 2");
        assert_eq!(plan.count, 9);
        assert_eq!(plan.offset, 320.0);

        let text = renderer.dom().text_content(container);
        assert!(text.contains("行 8"), "窗口首行必须在 DOM 里：{text}");
        assert!(text.contains("行 16"), "窗口末行必须在 DOM 里：{text}");
        assert!(!text.contains("行 7"), "窗口外的行不得渲染：{text}");
        assert!(!text.contains("行 17"), "窗口外的行不得渲染：{text}");
    }

    /// 接线失败**原样报错**（不吞错、不 panic）—— 宿主必须看得见。
    #[test]
    fn the_wiring_helper_reports_engine_errors_instead_of_panicking() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("section").expect("建根必须成功");
        let mut renderer = Renderer::new(dom, root);
        let props = props(rows(10)).window(Some(WindowRange::new(0, 9)));
        renderer
            .render_guarded(virtual_list(&props))
            .expect("首次挂载必须成功");

        // 未注入滚动度量 ⇒ 内存后端**显式**不支持 ⇒ 错误必须冒出来。
        let error = patch_windowed(&mut renderer, &[], &props).expect_err("未注入度量必须报错");
        assert!(
            matches!(error, DomError::NotImplemented | DomError::Undecidable(_)),
            "实测错误形态：{error:?}"
        );
    }

    /// 注册表路径：泛型组件按名渲染（props 类型必须一致）。
    #[test]
    fn the_component_renders_through_the_registry() {
        use crate::registry::ComponentRegistry;

        let mut registry = ComponentRegistry::new();
        registry
            .register_named::<VirtualList<Row>>("virtual-list")
            .expect("首次注册成功");
        let props = props(rows(2)).window(Some(WindowRange::new(0, 1)));
        let node = registry
            .render::<VirtualList<Row>>("virtual-list", &props)
            .expect("渲染成功");
        assert_eq!(node.tag(), Some("div"));
        assert_eq!(node.children().len(), 2);
    }
}
