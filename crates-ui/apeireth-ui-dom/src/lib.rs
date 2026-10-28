//! # apeireth-ui-dom — DOM 绑定层（虚拟节点 / 后端抽象 / 差分 / 委托 / 绑定）
//!
//! 引擎里**唯一**接触 DOM 的一层。职责边界（设计文档 §3.3.2）：只回答
//! 「状态变成 DOM」，不回答状态从哪来（那是 `ui-core` 与数据层）。
//!
//! ## 为什么 DOM 操作必须被抽象在 trait 后面
//! 差分 / 委托 / 绑定 / 渲染逻辑全部写成泛型代码（`D: Dom`）⇒ 100% 可 native
//! 单测，**不需要浏览器、不需要 wasm target**。这是「消灭源码镜像测试」的
//! 结构性前提（设计文档 §5.1.2 / §10.2）。
//!
//! ## 本批（P2）交付口径
//!
//! ### ✅ 已实现
//! - **虚拟节点**（元素 / 文本 / 分组）+ 元素节点的身份 key（[`Key`]）；
//! - **属性值语义**（布尔 `false` ⇒ 不写该属性）、class token 合并；
//! - **差分**：class 按 token + 属性键级 + 子节点按索引对齐（unkeyed 形态）；
//! - **keyed 列表差分**（[`keyed`]）：按 key 配对 + **最长递增子序列（LIS）**
//!   最小移动；移动次数 = 配对数 − LIS 长度（可机械核对的下界）；
//!   「是否移动」的判定是 **O(1)**（集合查表 ⇒ 不是 O(N²)）；
//! - **事件委托表**（目标 → 祖先的冒泡顺序 + `stop_propagation`）；
//! - **双向绑定**（`bind:value` / `bind:checked` 的两条方向语义）；
//! - **`bind:this`**（[`ThisBinding`]）：挂载写引用 / 卸载前清引用；
//! - **内存 DOM 后端**（[`InMemoryDom`]，native 永远可用）+ **操作序列记录**
//!   （[`DomOp`]：T1–T6 的机械判据）+ **日志上界**（长跑不会吃光内存）；
//! - **真实 DOM 后端**（`WebDom`，web-sys）—— wasm32 目标专用；
//! - **事件处理器接线**（P3）：虚拟树带处理器槽位（[`ElementHandler`]），
//!   `Dom` trait 有 `add_listener` / `remove_listener` / `remove_listeners`，
//!   渲染器在**挂载时挂**、**差分时按事件类型增量替换**、**卸载时摘**
//!   （T6 的成对契约）；
//! - **命名空间建节点**（P3）：`Dom::create_element_ns` ⇒ SVG 元素建在 SVG
//!   命名空间里（HTML 命名空间里的 `<svg>` **不是** SVG 元素：浏览器不把它
//!   当矢量图渲染，且 `viewBox` 这类驼峰属性名会被小写化）；
//! - **渲染器**（[`Renderer`]）：把 P1 的 keyed 差分计划落成实际 DOM 操作序列，
//!   挂载 / 卸载**成对执行**，`bind:this` **已接线**（挂载写引用 / 卸载清引用）；
//! - **CSS 过渡触发时机的六条保护规则**（设计文档 §5.5.2 的 T1–T6）逐条
//!   有机械测试（见 [`renderer`] 模块）；
//! - **失败显式化**：`Dom` 的每个操作都返回 `Result`（节点不存在 / 已摘除 /
//!   不适用 / 未接线 / 不可判定 五类错误各自分明）⇒ **没有静默降级**。
//!
//! ### ⭐ 块锚点（2026-10-25 修）
//!
//! **块** = 一个「虚拟节点 ↔ DOM 节点集合」的对应关系（元素 / 文本是**一个**
//! DOM 节点；分组（Fragment）是**一组**兄弟 DOM 节点）。
//!
//! - **块级**按位置 / key 配对 ⇒ 某个块「出现 / 消失」**不会**让兄弟错位；
//! - **块内**的增删按**块锚点**落位（该块后继块的**首** DOM 节点）
//!   ⇒ ⇒ 条件块从「空」变「非空」时，内容**留在原位**
//!   （修前是追加到父节点末尾 ⇒ 条件卡出现在容器**底部**）；
//! - **摊平只做一层**：嵌套分组保持为**子块** ⇒ ⇒ 嵌套块的身份**独立**于
//!   兄弟序列（修前递归摊平 ⇒ 嵌套块的身份被融进兄弟 ⇒ 身份转移）。
//!
//! ### ⭐ 失败之后：**显式劈叉**（2026-10-25 修）
//!
//! 一次 patch 失败 ⇒ DOM 可能已改了一半 ⇒ 内存树与 DOM **劈叉** ⇒
//! ⇒ 渲染器**不再**拿过期旧树继续差分（那会让每帧都失败 ⇒ 界面永久冻结）⇒
//! ⇒ 标记 [`Renderer::is_desynced`] ⇒ **下一次 patch 走整树重建**；
//! ⇒ 失败原因可读（[`Renderer::last_error`]）⇒ 宿主能 log、能决定重建。
//!
//! ### 🔵 未接线（等什么）
//! - **委托根单监听器**（设计文档 §5.3.2 的形态）——**元素直挂已接线**
//!   （`Dom` trait 有监听器接口；渲染器在挂载 / 差分 / 卸载三处接线；
//!   真实 DOM 后端的监听器按 **(节点, 事件类型)** 增量替换）。
//!   未接线的只是「委托根**单**监听器」这一种形态；[`EventRegistry`]
//!   （按「事件类型 + 挂载深度」索引的委托表）是**独立**的机制，
//!   尚未与渲染器接线。
//! - **事件委托的单监听器形态**（设计文档 §5.3.2）：本批按**元素直挂**
//!   （每个带处理器的元素一个监听器，冒泡交给浏览器）。`currentTarget` 语义
//!   与设计 §5.3.3 的确定解**一致**（= 处理器注册在哪个元素上），差别只在
//!   监听器数量。**元素直挂本身是已接线的**（`Dom` trait 有监听器接口，
//!   渲染器在挂载 / 差分 / 卸载三处接线）；未接线的只是「**委托根单监听器**」
//!   这一种形态。
//! - **真实 DOM 后端的运行时验证**——`WebDom` 被 `#[cfg(target_arch = "wasm32")]`
//!   隔离 ⇒ **native 编译不参与**。浏览器内的验证（真实 DOM 行为 / 像素对照 /
//!   逐帧对照）由 **P3 的试点对照 harness** 承担（`apeireth-ui-demo/harness/`：
//!   Rust 引擎版 vs 生产 Svelte 版的逐像素 + 逐帧对照）。
//! - **wasm 侧微任务调度器**——`ui-core` 的 `HostScheduler` trait 边界已就位，
//!   「把 flush 排进浏览器微任务队列」的那一实现落在 **wasm 宿主侧**。
//! - **effect 作用域与渲染器的联动**——设计文档 §5.1.4 的「挂载 ⇒ 建作用域 /
//!   卸载 ⇒ 逆序销毁作用域」由 `ui-core` 的作用域机制承担；渲染器**不**直接
//!   操作作用域，两者**尚未串联**。
//! - **双向绑定的信号写回**——`Dom` 的 property 接口**已接线**（渲染器在
//!   挂载 / 差分两处写 property，property 消失时**真正移除**并回落到默认值），
//!   但「信号变化 ⇒ 自动触发一次差分」这一步由 `ui-core` 的响应式链承担，
//!   **本 crate 不直接订阅信号**。
//!
//! ### P2 的 `Dom` trait 形状变化（公开 API）
//!
//! | 操作 | 为什么加 | 设计文档依据 |
//! |---|---|---|
//! | `insert_before` | keyed 列表的**移动**必须是「换位置」，不是「先摘除再追加」（后者会让节点短暂离开文档树，打断 CSS 过渡 / 动画状态） | §5.2.3 / §5.1.2 |
//! | `set_class_tokens` | class 必须**按 token** 差分（整体覆写会连带抹掉别的 class，并产生额外中间态） | §5.1.1 / §5.2.1 |
//! | `set_property` / `get_property` | `bind:value` 走 **property** 不是 attribute（写 attribute 对 `input.value` 无效，且破坏受控输入语义） | §5.4.1 / §5.4.3 |
//! | `remove_property` | property 从虚拟树上**消失**是正常差分（条件渲染）⇒ 没有它 ⇒ **整帧中止** ⇒ 界面冻结 | §5.4.3 |
//! | `remove_listener` | 差分时「换处理器」必须**按事件类型**摘挂 ⇒ 摘全部会**误伤**同元素的其它事件类型 | §5.3.2 / §5.3.3 |
//! | `target_path` | 事件负载的路径口径必须**两个后端一致**（否则委托路由不可比） | §5.3.3 |
//!
//! ⇒ 三个内存后端 / 未接线桩 / 真实后端**同步**实现，`DomError` 另加
//! `Undecidable`（差分不可判定，与「后端做不了」的 `Unsupported` 分开）。
//!
//! ### 🔵 未接线（等什么）
//! - **委托根单监听器**（设计文档 §5.3.2 的形态）——**元素直挂已接线**
//!   （`Dom` trait 有监听器接口；渲染器在挂载 / 差分 / 卸载三处接线；
//!   真实 DOM 后端的监听器按 **(节点, 事件类型)** 增量替换）。
//!   未接线的只是「委托根**单**监听器」这一种形态；[`EventRegistry`]
//!   （按「事件类型 + 挂载深度」索引的委托表）是**独立**的机制，
//!   尚未与渲染器接线。
//! - **事件委托的单监听器形态**（设计文档 §5.3.2）：本批按**元素直挂**
//!   （每个带处理器的元素一个监听器，冒泡交给浏览器）。`currentTarget` 语义
//!   与设计 §5.3.3 的确定解**一致**（= 处理器注册在哪个元素上），差别只在
//!   监听器数量。
//! - **真实 DOM 后端的运行时验证**——`WebDom` 被 `#[cfg(target_arch = "wasm32")]`
//!   隔离 ⇒ **native 编译不参与**。浏览器内的验证（真实 DOM 行为 / 像素对照 /
//!   逐帧对照）由 **P3 的试点对照 harness** 承担（`apeireth-ui-demo/harness/`：
//!   Rust 引擎版 vs 生产 Svelte 版的逐像素 + 逐帧对照）。
//! - **wasm 侧微任务调度器**——`ui-core` 的 `HostScheduler` trait 边界已就位，
//!   「把 flush 排进浏览器微任务队列」的那一实现落在 **wasm 宿主侧**。
//! - **effect 作用域与渲染器的联动**——设计文档 §5.1.4 的「挂载 ⇒ 建作用域 /
//!   卸载 ⇒ 逆序销毁作用域」由 `ui-core` 的作用域机制承担；渲染器**不**直接
//!   操作作用域，两者**尚未串联**。
//!
//! ## 纪律
//! 本 crate 的源码里**不出现颜色字面量与时长字面量**（设计文档 §2.2）：那些归
//! CSS 资产与设计令牌，引擎只引用 class 与结构。

#![forbid(unsafe_code)]

pub mod binding;
pub mod children;
pub mod diff;
pub mod dom;
pub mod events;
/// ⭐ 焦点 / IME / a11y 通道（L1-G3）。
pub mod focus;
pub mod keyed;
pub mod listeners;
pub mod renderer;
/// ⭐ 窗口化（虚拟列表）原语（L1-G5 第一片）。
pub mod virtual_list;
pub mod vnode;

/// 真实 DOM 后端（web-sys）—— **仅 wasm32 目标**。
///
/// native 编译时该模块**不参与** ⇒ 其行为未经任何运行时验证。
#[cfg(target_arch = "wasm32")]
pub mod web;

pub use binding::{Binding, BindingSlot, ThisBinding, ThisRef};
pub use children::Children;
pub use diff::{diff, Patch};
pub use dom::{
    Dom, DomError, DomOp, EventKind, InMemoryDom, InMemoryNodeId, RecordedOp, UnavailableDom,
    UnavailableNode, DEFAULT_OP_CAPACITY,
};
pub use events::{
    visible_child_index, DomEvent, ElementHandler, EventHandler, EventRegistry, HandlerId,
};
pub use keyed::{diff_keyed, list_diff_mode, KeyedDiff, KeyedDiffError, KeyedSlot, ListDiffMode};
pub use listeners::ListenerTable;
pub use renderer::{MountedElement, MountedFragment, MountedNode, MountedText, Renderer};
pub use vnode::{AttrValue, ElementBuilder, ElementVNode, Key, PropValue, TextVNode, VNode};

#[cfg(target_arch = "wasm32")]
pub use web::WebDom;

#[cfg(test)]
mod tests {
    //! crate 级断言：只放**跨模块**的组合行为（虚拟树 → 差分 → 内存 DOM）。

    use super::*;

    #[test]
    fn virtual_tree_renders_into_memory_backend() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("section").expect("建节点必须成功");
        dom.set_attribute(&root, "class", "ap-shell")
            .expect("写属性必须成功");
        let card = VNode::element("div")
            .class("ap-card")
            .attr("role", "group")
            .child(VNode::text("条目"))
            .build();
        let card_node = dom.create_element("div").expect("建节点必须成功");

        if let VNode::Element(element) = &card {
            for (name, value) in &element.attrs {
                if let Some(value) = value.to_attr_string() {
                    dom.set_attribute(&card_node, name, &value)
                        .expect("写属性必须成功");
                }
            }
            for child in &element.children {
                if let VNode::Text(text) = child {
                    let node = dom.create_text(&text.data).expect("建节点必须成功");
                    dom.append_child(&card_node, &node).expect("挂载必须成功");
                }
            }
        }
        dom.append_child(&root, &card_node).expect("挂载必须成功");

        assert_eq!(dom.tag(root), Some(String::from("section")));
        assert_eq!(dom.attribute(root, "class"), Some(String::from("ap-shell")));
        assert_eq!(
            dom.attribute(card_node, "class"),
            Some(String::from("ap-card"))
        );
        assert_eq!(dom.text_content(root), "条目");
    }

    #[test]
    fn diff_between_two_render_trees_is_empty() {
        let build = || {
            VNode::element("div")
                .class("a")
                .child(VNode::element("span").child(VNode::text("x")).build())
                .build()
        };
        assert!(diff(&build(), &build()).is_empty());
    }

    #[test]
    fn keyed_children_carry_their_identity_into_the_diff() {
        // 虚拟树 ⇒ keyed 差分 ⇒ 身份按 key 复用（跨模块组合）。
        let old = vec![
            VNode::element("li")
                .key("a")
                .child(VNode::text("1"))
                .build(),
            VNode::element("li")
                .key("b")
                .child(VNode::text("2"))
                .build(),
        ];
        let new = vec![
            VNode::element("li")
                .key("b")
                .child(VNode::text("2"))
                .build(),
            VNode::element("li")
                .key("a")
                .child(VNode::text("1"))
                .build(),
        ];

        let plan = diff_keyed(&old, &new).expect("差分必须成功");
        assert_eq!(plan.mode(), ListDiffMode::Keyed);
        assert_eq!(plan.matched(), &[Some(1), Some(0)]);
        assert_eq!(plan.move_count(), 1, "只移动一个节点（LIS 下界）");
    }

    #[test]
    fn bind_this_slot_travels_from_the_view_to_the_dom_backend() {
        // `bind:this` 槽位 ⇒ 内存 DOM 节点（跨模块组合）。
        let dom = InMemoryDom::new();
        let node = dom.create_element("canvas").expect("建节点必须成功");
        let binding: ThisBinding<InMemoryDom> = ThisBinding::new();

        assert_eq!(binding.get(), None);
        binding.mount(node);
        assert_eq!(binding.get(), Some(node));
        binding.unmount();
        assert_eq!(binding.get(), None);
    }

    #[test]
    fn renderer_bind_this_wiring_reaches_the_typed_binding() {
        // 虚拟树 ⇒ 渲染器 ⇒ `bind:this` 槽位 ⇒ 类型化绑定（跨模块组合）。
        let dom = InMemoryDom::new();
        let root = dom.create_element("main").expect("建节点必须成功");
        let binding: ThisBinding<InMemoryDom> = ThisBinding::new();
        let view = VNode::element("canvas").this(binding.erased()).build();

        let mut renderer = Renderer::new(dom, root);
        renderer.mount(view).expect("挂载必须成功");

        assert!(binding.is_bound(), "挂载 ⇒ 写引用");
        let node = binding.get().expect("挂载后必须有节点");
        assert_eq!(renderer.dom().tag(node), Some(String::from("canvas")));
        assert_eq!(renderer.bound_slots(), 1);

        renderer.unmount().expect("卸载必须成功");
        assert!(!binding.is_bound(), "卸载 ⇒ 清引用");
        assert_eq!(renderer.bound_slots(), 0, "挂载 / 卸载必须配平");
    }
}
