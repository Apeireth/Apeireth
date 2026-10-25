//! 渲染器：把虚拟树落成**真实 DOM 操作序列**（设计文档 §5.1.3 / §5.1.4）。
//!
//! ## 核心数据结构：已挂载树
//!
//! 差分的核心是「**旧 VNode ↔ DOM 节点**」的对应关系（设计文档 §5.1.3）：
//! 引擎保留上一帧的虚拟树，比对时逐节点对位，**就地**改真实 DOM。
//! [`MountedNode`] 就是这份对应关系。
//!
//! ## 挂载 / 卸载的成对契约（设计文档 §5.1.4）
//!
//! - **挂载**：建节点 → **写完全部属性** → 挂子节点 → 进文档树 → 接线
//!   `bind:this`。
//! - **卸载**：**先解绑 `bind:this`（清引用）**，再摘 DOM 节点。
//!
//! 顺序不可颠倒：先解绑 ⇒ 槽位里不会留下已销毁的节点句柄（调用方在
//! 「看起来还在」的节点上读出过期状态）。
//!
//! ## 六条 CSS 过渡保护规则（设计文档 §5.5.2）
//!
//! 渲染器是这些规则的**唯一执行点**，故规则可被机械判定（测试见本文件末尾）：
//!
//! | 规则 | 渲染器里的落点 |
//! |---|---|
//! | **T1** 同标签 ⇒ 原地打补丁 | [`Renderer::patch_node`]：标签相同 ⇒ 复用同一个 DOM 节点 |
//! | **T2** 属性写入连续完成 | [`Renderer::patch_attributes`]：一个节点的属性写在一次调用里连续完成 |
//! | **T3** 插入先写属性后进文档树 | [`Renderer::mount_node`]：属性先写，最后才 `insert_before` / `append_child` |
//! | **T4** 引擎零自主动画 | 只写虚拟树里**逐字存在**的值；同一批次内不产生「挂载即卸载」的瞬态节点 |
//! | **T5** 移除不加延迟 | 移除在**同一批次内立即执行**（无延迟、无排队） |
//! | **T6** 移除节点不参与本批次写入 | 被移除的子节点在移除前**从不被写属性**（差分阶段就排除了它们） |

use std::fmt;

use crate::dom::{Dom, DomError};
use crate::keyed::diff_keyed;
use crate::vnode::{AttrValue, ElementVNode, PropValue, TextVNode, VNode};

/// 已挂载的元素节点。
pub struct MountedElement<D: Dom> {
    /// 挂载时的虚拟节点（下一轮差分的「旧态」）。
    pub vnode: ElementVNode,
    /// 对应的真实 DOM 节点。
    pub dom: D::Node,
    /// 子节点（已挂载）。
    pub children: Vec<MountedNode<D>>,
}

impl<D: Dom> Clone for MountedElement<D> {
    fn clone(&self) -> Self {
        Self {
            vnode: self.vnode.clone(),
            dom: self.dom.clone(),
            children: self.children.clone(),
        }
    }
}

/// 已挂载的文本节点。
pub struct MountedText<D: Dom> {
    /// 挂载时的虚拟节点。
    pub vnode: TextVNode,
    /// 对应的真实 DOM 节点。
    pub dom: D::Node,
}

impl<D: Dom> Clone for MountedText<D> {
    fn clone(&self) -> Self {
        Self {
            vnode: self.vnode.clone(),
            dom: self.dom.clone(),
        }
    }
}

/// 已挂载的分组节点（无宿主节点；对应列表 / 条件块容器）。
pub struct MountedFragment<D: Dom> {
    /// 子节点（已挂载；宿主节点 = 它们的父节点）。
    pub children: Vec<MountedNode<D>>,
}

impl<D: Dom> Clone for MountedFragment<D> {
    fn clone(&self) -> Self {
        Self {
            children: self.children.clone(),
        }
    }
}

/// 已挂载的节点：虚拟树 ↔ 真实 DOM 的对应关系。
pub enum MountedNode<D: Dom> {
    /// 元素节点。
    Element(MountedElement<D>),
    /// 文本节点。
    Text(MountedText<D>),
    /// 分组节点（无宿主节点）。
    Fragment(MountedFragment<D>),
}

impl<D: Dom> Clone for MountedNode<D> {
    fn clone(&self) -> Self {
        match self {
            MountedNode::Element(element) => MountedNode::Element(element.clone()),
            MountedNode::Text(text) => MountedNode::Text(text.clone()),
            MountedNode::Fragment(fragment) => MountedNode::Fragment(fragment.clone()),
        }
    }
}

impl<D: Dom> MountedNode<D> {
    /// 对应的真实 DOM 节点（分组节点没有宿主节点 ⇒ `None`）。
    pub fn dom_node(&self) -> Option<&D::Node> {
        match self {
            MountedNode::Element(element) => Some(&element.dom),
            MountedNode::Text(text) => Some(&text.dom),
            MountedNode::Fragment(_) => None,
        }
    }

    /// 本节点的列表身份（keyed 差分用）。
    pub fn key(&self) -> Option<&crate::vnode::Key> {
        match self {
            MountedNode::Element(element) => element.vnode.key.as_ref(),
            _ => None,
        }
    }

    /// 子节点（已挂载）。
    pub fn children(&self) -> &[MountedNode<D>] {
        match self {
            MountedNode::Element(element) => &element.children,
            MountedNode::Fragment(fragment) => &fragment.children,
            MountedNode::Text(_) => &[],
        }
    }

    /// 重建虚拟节点（供 keyed 差分复用 P1 的差分计划）。
    fn to_vnode(&self) -> Option<VNode> {
        match self {
            MountedNode::Element(element) => Some(VNode::Element(element.vnode.clone())),
            MountedNode::Text(text) => Some(VNode::Text(text.vnode.clone())),
            MountedNode::Fragment(_) => None,
        }
    }
}

impl<D: Dom> fmt::Debug for MountedNode<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MountedNode::Element(element) => f
                .debug_struct("MountedNode::Element")
                .field("tag", &element.vnode.tag)
                .field("children", &element.children.len())
                .finish(),
            MountedNode::Text(text) => f
                .debug_struct("MountedNode::Text")
                .field("data", &text.vnode.data)
                .finish(),
            MountedNode::Fragment(fragment) => f
                .debug_struct("MountedNode::Fragment")
                .field("children", &fragment.children.len())
                .finish(),
        }
    }
}

impl<D: Dom> PartialEq for MountedNode<D> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MountedNode::Element(left), MountedNode::Element(right)) => {
                left.vnode == right.vnode
                    && left.dom == right.dom
                    && left.children == right.children
            }
            (MountedNode::Text(left), MountedNode::Text(right)) => {
                left.vnode == right.vnode && left.dom == right.dom
            }
            (MountedNode::Fragment(left), MountedNode::Fragment(right)) => {
                left.children == right.children
            }
            _ => false,
        }
    }
}

impl<D: Dom> Eq for MountedNode<D> {}

/// 渲染器：持有一棵「已挂载树」，把虚拟树的变化落成真实 DOM 操作。
///
/// ## 生命周期
///
/// ```text
/// new ──▶ mount ──▶ patch ──▶ patch ──▶ unmount
///           │                     ▲                    │
///           └─────────────────────┴────────────────────┘
///                （mount / unmount 成对；patch 只在已挂载时可用）
/// ```
///
/// ## 挂载容器归引擎独占
///
/// 渲染器把容器当作**自己独占**的子树：新节点追加到末尾、锚点按容器内的
/// 兄弟顺序算。依据：设计文档 §5.3.2「委托根 = 引擎的挂载容器（所有引擎产出
/// 的 DOM 的唯一公共祖先）」。宿主若在容器里塞自己的节点，两侧下标会错位。
///
/// ## 诚实边界
///
/// - **事件委托未接线**：`Dom` trait 目前**没有**监听器接口 ⇒ 渲染器不负责
///   挂 / 摘事件监听（委托表在 [`crate::events`] 里独立存在，尚未接到真实
///   DOM 后端）。
/// - **effect 作用域未接线**：设计文档 §5.1.4 的「挂载 ⇒ 建作用域 / 卸载 ⇒
///   逆序销毁作用域」由 `ui-core` 的作用域机制承担，渲染器**不**直接操作它。
#[derive(Debug)]
pub struct Renderer<D: Dom> {
    dom: D,
    root: D::Node,
    tree: Option<MountedNode<D>>,
    /// 当前**已绑定**的 `bind:this` 槽位数（挂载 +1 / 卸载 −1）。
    bound_slots: usize,
}

impl<D: Dom> Renderer<D> {
    /// 建一个渲染器（挂载容器 = `root`）。
    pub fn new(dom: D, root: D::Node) -> Self {
        Self {
            dom,
            root,
            tree: None,
            bound_slots: 0,
        }
    }

    /// 读后端。
    pub fn dom(&self) -> &D {
        &self.dom
    }

    /// 读挂载容器。
    pub fn root(&self) -> &D::Node {
        &self.root
    }

    /// 读当前已挂载的树（未挂载 ⇒ `None`）。
    pub fn tree(&self) -> Option<&MountedNode<D>> {
        self.tree.as_ref()
    }

    /// 当前是否已挂载。
    pub fn is_mounted(&self) -> bool {
        self.tree.is_some()
    }

    /// 当前**已绑定**的 `bind:this` 槽位数。
    ///
    /// 挂载后 − 卸载后 = 0 ⇒ 「mount / unmount 成对」的机械判据。
    pub fn bound_slots(&self) -> usize {
        self.bound_slots
    }

    /// 挂载一棵虚拟树（追加到挂载容器末尾）。
    ///
    /// 幂等前提：同一渲染器**只挂一次**；重复挂载会报错（见 [`DomError`]）。
    pub fn mount(&mut self, vnode: VNode) -> Result<MountedNode<D>, DomError> {
        if self.tree.is_some() {
            return Err(DomError::Undecidable(String::from(
                "渲染器已挂载：挂载 / 卸载必须成对（先卸载再挂载）",
            )));
        }
        let root = self.root.clone();
        let mounted = self.mount_node(&vnode, &root, None)?;
        self.tree = Some(mounted.clone());
        Ok(mounted)
    }

    /// 打补丁：用新虚拟树替换当前已挂载的树。
    ///
    /// 失败时**保留旧树**（不留下半吊子状态）：DOM 已可能被部分改动，
    /// 但已挂载树的引用仍在调用方手里，便于继续排查。
    pub fn patch(&mut self, vnode: VNode) -> Result<MountedNode<D>, DomError> {
        let Some(old) = self.tree.take() else {
            return Err(DomError::Undecidable(String::from(
                "渲染器未挂载：先 mount 再 patch",
            )));
        };
        let root = self.root.clone();
        match self.patch_node(&old, &vnode, &root) {
            Ok(mounted) => {
                self.tree = Some(mounted.clone());
                Ok(mounted)
            }
            Err(error) => {
                self.tree = Some(old);
                Err(error)
            }
        }
    }

    /// 卸载整棵树（解绑 `bind:this` ⇒ 摘除 DOM 节点）。
    ///
    /// 幂等：未挂载时是空操作。
    pub fn unmount(&mut self) -> Result<(), DomError> {
        let Some(old) = self.tree.take() else {
            return Ok(());
        };
        let root = self.root.clone();
        self.unmount_node(&old, &root)
    }

    // ── 挂载 ──────────────────────────────────────────────────────────────

    /// 挂载一个节点（子节点先建好并写完属性，宿主节点最后进文档树 —— T3）。
    fn mount_node(
        &mut self,
        vnode: &VNode,
        parent: &D::Node,
        anchor: Option<&D::Node>,
    ) -> Result<MountedNode<D>, DomError> {
        match vnode {
            VNode::Element(element) => {
                let dom = match svg_namespace(&element.tag) {
                    Some(namespace) => self.dom.create_element_ns(namespace, &element.tag)?,
                    None => self.dom.create_element(&element.tag)?,
                };
                // T3：属性先写 —— 节点进文档树时看到的已经是最终态。
                self.write_attributes(&dom, element)?;
                // 事件处理器：节点进文档树前挂好（与属性同一批准备完）。
                self.write_handlers(&dom, element)?;
                let mut children = Vec::with_capacity(element.children.len());
                for child in &element.children {
                    children.push(self.mount_node(child, &dom, None)?);
                }
                // property 在**子节点之后**写（`select.value` 依赖 `option` 已存在），
                // 且**入树之前**（用户看不到默认值 → 目标值的中间态）。
                self.write_props(&dom, element)?;
                self.insert_into(parent, &dom, anchor)?;
                if let Some(this_ref) = &element.this_ref {
                    this_ref.bind_node(Box::new(dom.clone()))?;
                    self.bound_slots += 1;
                }
                Ok(MountedNode::Element(MountedElement {
                    vnode: element.clone(),
                    dom,
                    children,
                }))
            }
            VNode::Text(text) => {
                let dom = self.dom.create_text(&text.data)?;
                self.insert_into(parent, &dom, anchor)?;
                Ok(MountedNode::Text(MountedText {
                    vnode: text.clone(),
                    dom,
                }))
            }
            VNode::Fragment(children) => {
                let mut mounted = Vec::with_capacity(children.len());
                for child in children {
                    mounted.push(self.mount_node(child, parent, anchor)?);
                }
                Ok(MountedNode::Fragment(MountedFragment { children: mounted }))
            }
        }
    }

    // ── 打补丁 ────────────────────────────────────────────────────────────

    /// 打补丁：旧已挂载节点 ↔ 新虚拟节点。
    fn patch_node(
        &mut self,
        old: &MountedNode<D>,
        new: &VNode,
        parent: &D::Node,
    ) -> Result<MountedNode<D>, DomError> {
        match (old, new) {
            (MountedNode::Element(old_element), VNode::Element(new_element)) => {
                if old_element.vnode.tag != new_element.tag {
                    // 标签变了 ⇒ 必须重建（T1 只保护「同标签」的情形）。
                    let anchor = self.anchor_after(parent, &old_element.dom)?;
                    self.unmount_node(old, parent)?;
                    return self.mount_node(new, parent, anchor.as_ref());
                }
                // T1：同标签 ⇒ 复用同一个真实 DOM 节点，只改属性与子节点。
                self.patch_attributes(&old_element.dom, &old_element.vnode, new_element)?;
                self.patch_handlers(&old_element.dom, &old_element.vnode, new_element)?;
                self.rebind_this(&old_element.vnode, new_element, &old_element.dom)?;
                let children = self.patch_children(
                    &old_element.dom,
                    &old_element.children,
                    &new_element.children,
                )?;
                // property 在**子节点之后**打（同挂载时的理由）。
                self.patch_props(&old_element.dom, &old_element.vnode, new_element)?;
                Ok(MountedNode::Element(MountedElement {
                    vnode: new_element.clone(),
                    dom: old_element.dom.clone(),
                    children,
                }))
            }
            (MountedNode::Text(old_text), VNode::Text(new_text)) => {
                if old_text.vnode.data != new_text.data {
                    // 就地改文本（不重建文本节点 —— 设计文档 §5.2.4）。
                    self.dom.set_text(&old_text.dom, &new_text.data)?;
                }
                Ok(MountedNode::Text(MountedText {
                    vnode: new_text.clone(),
                    dom: old_text.dom.clone(),
                }))
            }
            (MountedNode::Fragment(old_fragment), VNode::Fragment(new_children)) => {
                // ⭐ **块内摊平配对**（2026-10-25 修）：
                // Fragment **没有宿主节点** ⇒ 它在 DOM 上就是它 children 的一组兄弟
                // ⇒ ⇒ 块内按**真实 DOM 节点**配对 ⇒ **元素被复用**
                // ⇒ ⇒ **受控输入可用**（修前：每次重渲染都重建整棵 Fragment 子树
                // ⇒ ⇒ 输入框被换掉 ⇒ 用户输入与焦点一起丢）。
                //
                // ⚠️ **块级**由 `patch_children` 负责（按位置 ⇒ 兄弟块不错位）；
                // 这里只管**块内**。
                let old_flat = flatten_mounted(&old_fragment.children);
                let new_flat = flatten_vnode(new_children);
                let old_vnodes: Vec<VNode> = old_flat
                    .iter()
                    .filter_map(|child| child.to_vnode())
                    .collect();
                let keyed = old_vnodes.len() == old_flat.len()
                    && crate::keyed::list_diff_mode(&old_vnodes, &new_flat)
                        == crate::keyed::ListDiffMode::Keyed;
                // ⭐ 混合配对：**有 key 的**按身份，**无 key 的**按位置
                // ⇒ ⇒ 条件渲染导致的「长度变化」不再让兄弟错位。
                let has_key = old_vnodes.iter().any(|vnode| vnode.key().is_some())
                    || new_flat.iter().any(|vnode| vnode.key().is_some());
                let children = if keyed {
                    self.patch_keyed_children(parent, &old_flat, &new_flat, &old_vnodes)?
                } else if has_key {
                    self.patch_mixed_children(parent, &old_flat, &new_flat)?
                } else {
                    self.patch_unkeyed_children(parent, &old_flat, &new_flat)?
                };
                Ok(MountedNode::Fragment(MountedFragment { children }))
            }
            _ => {
                // 类型不匹配 ⇒ 替换子树。
                let anchor = match last_dom_node(old) {
                    Some(dom) => self.anchor_after(parent, dom)?,
                    None => None,
                };
                self.unmount_node(old, parent)?;
                self.mount_node(new, parent, anchor.as_ref())
            }
        }
    }

    /// 子节点对位：keyed ⇒ 按身份（key）配对；否则 ⇒ 按位置。
    ///
    /// ## ⭐ **两侧都先「摊平」Fragment**（2026-10-25 修）
    ///
    /// ### 为什么
    ///
    /// `view!` 宏把「内插一个变量」编译成 **Fragment** ⇒ 组件树里到处都有。
    /// 而 Fragment **没有宿主节点** ⇒ 它在 DOM 上是**摊平**的一组兄弟。
    /// ⇒ ⇒ **配对的单位必须是「真实 DOM 节点」**，不能是「VNode 节点」。
    ///
    /// ### 摊平前的三个后果（**全部实测过**）
    ///
    /// 1. **每次重渲染都把整棵 Fragment 子树重建** ⇒ `patch_node` 没有
    ///    `(Fragment, Fragment)` 分支 ⇒ Fragment 落到「类型不匹配 ⇒ 替换子树」
    ///    ⇒ ⇒ **输入框元素被换掉** ⇒ 用户输入与焦点一起丢
    ///    ⇒ ⇒ **受控输入在引擎侧不可用**（实测：键入 1 个字符 ⇒ 取值 `''`）。
    /// 2. **keyed 差分被废掉**：`MountedNode::to_vnode()` 对 Fragment 返回 `None`
    ///    ⇒ `patch_children` 的 keyed 判定 `old_vnodes.len() == old.len()`
    ///    ⇒ **含 Fragment 的列表永远走不到 keyed** ⇒ 顺序变化只能靠重建。
    /// 3. **配对错位**：old 侧一个 Fragment 对 new 侧一个 Fragment ⇒ 但两者的
    ///    children 数量/顺序可能不同 ⇒ 按索引配对会把 `old[i]` 复用给语义不同的
    ///    `new[i]` ⇒ **内容出现在错误的位置**（实测：审批项的「会话行」跑到
    ///    「输入框」的位置 ⇒ 逐像素差 27.4%）。
    ///
    /// ### 现在的做法：**块级按位置 + 块内摊平配对**（2026-10-25 定案）
    ///
    /// - **块级**（本函数）：old / new 的每一项都是**块** ⇒ 按**位置**配对
    ///   ⇒ 块与块**一一对应**（`{ expr }` 这种插值恒占一个位置）
    ///   ⇒ ⇒ **兄弟块不会因为某个块「出现 / 消失」而错位**
    ///   ⇒ 这与生产侧 **Svelte 的 `{#if}` 块锚点**语义一致
    ///   （Svelte 给每个块一个独立锚点 ⇒ 块增删**不**影响其它块）。
    /// - **块内**（`patch_node` 的 `(Fragment, Fragment)` 分支）：摊平后配对
    ///   ⇒ 块内元素**被复用** ⇒ ⇒ **受控输入可用**；
    ///   ⇒ 块内若全部有 `key` ⇒ 按**身份**配对（可处理块内换序）。
    ///
    /// ### ⚠️ **为什么不是「整层摊平」**（2026-10-25 实测否掉的中间方案）
    ///
    /// 曾试过「把**整层**的 Fragment 摊平后再配对」⇒ **实测引入新的错位**：
    /// 摊平后，条件渲染的「生效配置」卡片**出现** ⇒ 该层长度 5→6
    /// ⇒ 位置配对 ⇒ **后面全部错位** ⇒ 搜索框仍被换掉（实测：元素标记丢失）。
    /// ⇒ 根因：摊平把**块级身份**融进了兄弟序列 ⇒ 块增删 ⇒ 兄弟错位。
    /// ⇒ ⇒ 正确做法：**块级保留身份**（按位置）＋ **块内摊平**（按真实节点）。
    fn patch_children(
        &mut self,
        parent: &D::Node,
        old: &[MountedNode<D>],
        new: &[VNode],
    ) -> Result<Vec<MountedNode<D>>, DomError> {
        // ① **块级**按位置配对（old / new 的每一项都是「块」）
        //    ⇒ `{ expr }` 这种插值**恒占一个位置** ⇒ 某个块「出现 / 消失」
        //    ⇒ **不会**让后面的兄弟错位
        //    （这与生产侧 Svelte 的 `{#if}` 块锚点语义一致）。
        let old_vnodes: Vec<VNode> = old.iter().filter_map(|child| child.to_vnode()).collect();
        let keyed = old_vnodes.len() == old.len()
            && crate::keyed::list_diff_mode(&old_vnodes, new) == crate::keyed::ListDiffMode::Keyed;
        // ⭐ 混合配对（见 `patch_mixed_children` 的说明）⇒ 条件渲染的块
        // 「出现 / 消失」时，**兄弟不再错位**。
        let has_key = old_vnodes.iter().any(|vnode| vnode.key().is_some())
            || new.iter().any(|vnode| vnode.key().is_some());
        if keyed {
            self.patch_keyed_children(parent, old, new, &old_vnodes)
        } else if has_key {
            self.patch_mixed_children(parent, old, new)
        } else {
            self.patch_unkeyed_children(parent, old, new)
        }
    }

    /// keyed 子节点对位：按 key 配对 + LIS 最小移动（设计文档 §5.2.3）。
    fn patch_keyed_children(
        &mut self,
        parent: &D::Node,
        old: &[MountedNode<D>],
        new: &[VNode],
        old_vnodes: &[VNode],
    ) -> Result<Vec<MountedNode<D>>, DomError> {
        // 差分计划**先**算出来（重复 key ⇒ 直接报错，此时 DOM 一个字节都没动）。
        let plan = diff_keyed(old_vnodes, new)
            .map_err(|error| DomError::Undecidable(format!("keyed 差分不可判定：{error}")))?;

        // ① 移除：key 消失的旧子节点 —— 同一批次内**立即**移除，不加延迟（T5）。
        //    被移除的子节点在本批次内**从未**被写属性（差分阶段就排除了它们 ⇒ T6）。
        for &old_index in plan.removed() {
            let Some(child) = old.get(old_index) else {
                continue;
            };
            self.unmount_node(child, parent)?;
        }

        // ② 按新顺序重建子节点序列（自后向前；锚点 = 已就位的后继）。
        let mut slots: Vec<Option<MountedNode<D>>> = vec![None; new.len()];
        let mut anchor: Option<D::Node> = None;
        for new_index in (0..new.len()).rev() {
            match plan.matched()[new_index] {
                Some(old_index) => {
                    let Some(old_child) = old.get(old_index) else {
                        continue;
                    };
                    let patched = self.patch_node(old_child, &new[new_index], parent)?;
                    let dom = patched.dom_node().cloned();
                    // 不在 LIS 里的子节点要换位置（LIS 之外的必须移动 ⇒ 移动次数
                    // = 配对数 − LIS 长度 = 下界）。
                    if !plan.retained().contains(&old_index) {
                        if let Some(node) = dom.as_ref() {
                            self.dom.insert_before(parent, node, anchor.as_ref())?;
                        }
                    }
                    anchor = dom;
                    slots[new_index] = Some(patched);
                }
                None => {
                    // 新建：属性先写、节点后进文档树（T3）。
                    let mounted = self.mount_node(&new[new_index], parent, anchor.as_ref())?;
                    anchor = mounted.dom_node().cloned();
                    slots[new_index] = Some(mounted);
                }
            }
        }
        Ok(slots.into_iter().flatten().collect())
    }

    /// ⭐ **混合配对**（2026-10-25）：**有 `key` 的按身份**，**无 `key` 的按位置**。
    ///
    /// ## 为什么需要（实测出来的）
    ///
    /// 组件树里**条件渲染**很常见（`{#if}`）：某个子块「出现 / 消失」
    /// ⇒ 该层**长度变化** ⇒ 纯位置配对 ⇒ **后面的兄弟全部错位**
    /// ⇒ ⇒ 实测：provider 面的「生效配置」卡片一出现 ⇒ 搜索框被换掉
    /// ⇒ ⇒ 治理面的 `show_created` 一变 ⇒ 「会话行」与按钮错位
    /// ⇒ ⇒ 逐像素差 27.4%。
    ///
    /// ## 为什么不能直接用 keyed 差分
    ///
    /// `list_diff_mode` 要求**整层全部有 `key`** ⇒ 而生产模板里元素之间有
    /// **空格文本节点**（Svelte 编译产物）⇒ 空格**没有 key** ⇒
    /// ⇒ **keyed 永远不成立** ⇒ ⇒ 「给列表项加 key」这条路在有空格时**无效**。
    ///
    /// ## 本函数的口径
    ///
    /// - new 侧**有 `key`** 且 old 侧**有同 key** ⇒ **按身份复用**（不按位置）；
    /// - 其余（无 key / key 对不上）⇒ **按位置兜底**（取下一个未使用的 old 下标）；
    /// - 复用后**按 new 顺序重排**（`insert_before` 会移动已有节点）
    ///   ⇒ ⇒ DOM 顺序恒等于 new 顺序。
    ///
    /// ⚠️ **只在「这一层存在 key」时启用** ⇒ ⇒ 绝大多数层仍走
    /// [`Self::patch_unkeyed_children`]（**既有契约完全不变**）。
    fn patch_mixed_children(
        &mut self,
        parent: &D::Node,
        old: &[MountedNode<D>],
        new: &[VNode],
    ) -> Result<Vec<MountedNode<D>>, DomError> {
        // ① old 侧的 key → 下标（**重复 key 取第一个**；重复 key 的可判定性
        //    由 keyed 差分负责报「不可判定」⇒ 这里不重复报错）。
        let mut by_key: std::collections::HashMap<crate::vnode::Key, usize> =
            std::collections::HashMap::new();
        for (index, child) in old.iter().enumerate() {
            if let Some(key) = child.to_vnode().and_then(|vnode| vnode.key().cloned()) {
                by_key.entry(key).or_insert(index);
            }
        }
        // ② **两阶段**配对（2026-10-25 修正）：
        //
        // ⚠️ **必须先做完全部 key 配对，再让「无 key 的」按位置兜底**。
        // ⇒ 否则「位置兜底」会**抢走**后面 key 匹配项 ⇒ 错位。
        // ⇒ 实测：old = [A, B, C]、new = [A, X, B, C]（X 是条件渲染出现的新块）
        // ⇒ 若边走边兜底 ⇒ X 抢走 B ⇒ B 抢走 C ⇒ **三个全错位**
        // ⇒ ⇒ 实测表现为「`code` 与 `footer` 的标签互换」⇒ 逐像素差 27.4%。
        //
        // 规则：
        // - new **有 key** 且 old **有同 key** ⇒ **按身份复用**；
        // - new **有 key** 但 old **没有** ⇒ **新建**（**不**参与位置兜底
        //   ⇒ 否则它会抢掉后面的位置）；
        // - new **无 key** ⇒ 按位置兜底（取下一个未使用的 old 下标）。
        let mut used = vec![false; old.len()];
        let mut plan: Vec<Option<usize>> = vec![None; new.len()];
        // 阶段 ①：key 配对。
        for (new_index, child) in new.iter().enumerate() {
            let Some(key) = child.key() else {
                continue;
            };
            let Some(&old_index) = by_key.get(key) else {
                continue;
            };
            if used[old_index] {
                continue;
            }
            used[old_index] = true;
            plan[new_index] = Some(old_index);
        }
        // 阶段 ②：无 key 的按位置兜底。
        let mut cursor = 0usize;
        for new_index in 0..new.len() {
            if plan[new_index].is_some() || new[new_index].key().is_some() {
                continue;
            }
            let mut found = None;
            for (probe, is_used) in used.iter().enumerate().skip(cursor) {
                if !is_used {
                    found = Some(probe);
                    cursor = probe + 1;
                    break;
                }
            }
            if let Some(index) = found {
                used[index] = true;
                plan[new_index] = Some(index);
            }
        }
        // ③ 没被认领的旧子节点：移除（自后向前 ⇒ 保持前面的下标有效）。
        for index in (0..old.len()).rev() {
            if !used[index] {
                self.unmount_node(&old[index], parent)?;
            }
        }
        // ④ 复用节点的**相对顺序**是否已经与 new 一致？
        //    ⇒ 若一致 ⇒ **不需要移动** ⇒ ⚠️ **移动会重启动 CSS 动画**
        //    ⇒ ⇒ 实测：每次渲染都重排 ⇒ 顶层 harness 的**动画相位**对照
        //    ⇒ ⇒ 在 150ms 相位差 3.38% ⇒ ⇒ 只在**真的换序**时才移动。
        let reused: Vec<usize> = plan.iter().filter_map(|slot| *slot).collect();
        let order_preserved = reused.windows(2).all(|pair| pair[0] < pair[1]);
        // ⑤ 按 new 顺序落位：复用（必要时重排）+ 新建。
        let mut slots: Vec<Option<MountedNode<D>>> = vec![None; new.len()];
        let mut anchor: Option<D::Node> = None;
        for new_index in (0..new.len()).rev() {
            match plan[new_index] {
                Some(old_index) => {
                    let Some(old_child) = old.get(old_index) else {
                        continue;
                    };
                    let patched = self.patch_node(old_child, &new[new_index], parent)?;
                    let dom = patched.dom_node().cloned();
                    if !order_preserved {
                        // `insert_before` 对**已在文档里**的节点 ⇒ **移动**它
                        // ⇒ ⇒ DOM 顺序恒等于 new 顺序。
                        if let Some(node) = dom.as_ref() {
                            self.dom.insert_before(parent, node, anchor.as_ref())?;
                        }
                    }
                    anchor = dom;
                    slots[new_index] = Some(patched);
                }
                None => {
                    let mounted = self.mount_node(&new[new_index], parent, anchor.as_ref())?;
                    anchor = mounted.dom_node().cloned();
                    slots[new_index] = Some(mounted);
                }
            }
        }
        Ok(slots.into_iter().flatten().collect())
    }

    /// unkeyed 子节点对位：按位置对齐（设计文档 §5.2.2）。
    fn patch_unkeyed_children(
        &mut self,
        parent: &D::Node,
        old: &[MountedNode<D>],
        new: &[VNode],
    ) -> Result<Vec<MountedNode<D>>, DomError> {
        let common = old.len().min(new.len());
        let mut mounted = Vec::with_capacity(new.len());
        for index in 0..common {
            mounted.push(self.patch_node(&old[index], &new[index], parent)?);
        }
        // 多余的旧子节点：自后向前移除（保持前面的下标有效）。
        for index in (common..old.len()).rev() {
            self.unmount_node(&old[index], parent)?;
        }
        // 新增的子节点：追加到末尾。
        for child in new.iter().skip(common) {
            mounted.push(self.mount_node(child, parent, None)?);
        }
        Ok(mounted)
    }

    // ── 属性 ──────────────────────────────────────────────────────────────

    /// 新建节点的属性写入（全部属性一次写完 —— T3）。
    fn write_attributes(&self, dom: &D::Node, element: &ElementVNode) -> Result<(), DomError> {
        let mut class_tokens: Vec<String> = Vec::new();
        for (name, value) in &element.attrs {
            if name == "class" {
                if let Some(text) = value.to_attr_string() {
                    class_tokens.extend(text.split_whitespace().map(str::to_owned));
                }
                continue;
            }
            // 布尔 `false` ⇒ 该属性**不写**（设计文档 §5.1.1 的属性值语义）。
            if let Some(text) = value.to_attr_string() {
                self.dom.set_attribute(dom, name, &text)?;
            }
        }
        if !class_tokens.is_empty() {
            self.dom.set_class_tokens(dom, &class_tokens, &[])?;
        }
        Ok(())
    }

    /// 新建节点：写完**全部 property**。
    ///
    /// ## 时机（不可挪）
    ///
    /// 在**子节点挂完之后**、**节点进文档树之前**调用：
    /// - 子节点之后 ⇒ `<select>` 的 `value` 依赖 `<option>` 已存在（否则静默无效）；
    /// - 入树之前 ⇒ 用户看不到「默认值 → 目标值」的中间态（`range` 会看到拇指跳一下）。
    fn write_props(&self, dom: &D::Node, element: &ElementVNode) -> Result<(), DomError> {
        for (name, value) in &element.props {
            match value {
                PropValue::Text(text) => self.dom.set_property(dom, name, text)?,
                PropValue::Bool(flag) => self.dom.set_property_bool(dom, name, *flag)?,
            }
        }
        Ok(())
    }

    /// 已有节点：property **变了才写**。
    ///
    /// ## 为什么必须「变了才写」
    ///
    /// 每次重渲染都无条件写 `value` ⇒ **打断输入法组合态**（拼音未上屏时被清空）
    /// 且会把光标推到末尾。这与各面宿主侧回写用的是同一条口径
    /// （「只在当前值 ≠ 目标值时写」）。
    ///
    /// ⚠️ 读回比较用 [`Dom::get_property`]；后端读不出（返回空串）时
    /// **不写**——宁可不动，也不要在读不到当前值时反复覆盖用户的输入。
    fn patch_props(
        &self,
        dom: &D::Node,
        old: &ElementVNode,
        new: &ElementVNode,
    ) -> Result<(), DomError> {
        let mut names: Vec<String> = old.props.iter().map(|(name, _)| name.clone()).collect();
        for (name, _) in &new.props {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
        for name in names {
            let old_value = old.prop_value(&name);
            let new_value = new.prop_value(&name);
            if old_value == new_value {
                continue;
            }
            match new_value {
                Some(PropValue::Text(text)) => {
                    // 当前值已是目标值 ⇒ 不写（保护输入法组合态）。
                    match self.dom.get_property(dom, &name) {
                        Ok(current) if current == *text => continue,
                        _ => {}
                    }
                    self.dom.set_property(dom, &name, text)?;
                }
                Some(PropValue::Bool(flag)) => {
                    self.dom.set_property_bool(dom, &name, *flag)?;
                }
                None => {
                    // property 被移除：**没有**「移除 property」的通用操作
                    // ⇒ 不静默清空（那会破坏受控输入）；登记为未支持。
                    return Err(DomError::Unsupported(format!(
                        "移除 property `{name}` 未支持（property 无通用删除操作）"
                    )));
                }
            }
        }
        Ok(())
    }

    /// 已有节点的属性差分（class 按 token；其余按键）。
    ///
    /// **T2**：一个节点的**全部**属性写入在这一次调用里连续完成，中间不插入
    /// 任何节点增删操作。
    fn patch_attributes(
        &self,
        dom: &D::Node,
        old: &ElementVNode,
        new: &ElementVNode,
    ) -> Result<(), DomError> {
        let old_tokens = class_tokens(old);
        let new_tokens = class_tokens(new);
        let added: Vec<String> = new_tokens
            .iter()
            .filter(|token| !old_tokens.contains(token))
            .cloned()
            .collect();
        let removed: Vec<String> = old_tokens
            .iter()
            .filter(|token| !new_tokens.contains(token))
            .cloned()
            .collect();
        if !added.is_empty() || !removed.is_empty() {
            self.dom.set_class_tokens(dom, &added, &removed)?;
        }

        let mut names: Vec<String> = old
            .attrs
            .iter()
            .map(|(name, _)| name.clone())
            .filter(|name| name != "class")
            .collect();
        for (name, _) in &new.attrs {
            if name != "class" && !names.contains(name) {
                names.push(name.clone());
            }
        }
        for name in names {
            let old_value = old.attr_value(&name).and_then(AttrValue::to_attr_string);
            let new_value = new.attr_value(&name).and_then(AttrValue::to_attr_string);
            if old_value != new_value {
                match new_value {
                    Some(value) => self.dom.set_attribute(dom, &name, &value)?,
                    None => self.dom.remove_attribute(dom, &name)?,
                }
            }
        }
        Ok(())
    }

    // ── 事件处理器 ────────────────────────────────────────────────────────

    /// 新建节点：挂上全部事件处理器。
    fn write_handlers(&self, dom: &D::Node, element: &ElementVNode) -> Result<(), DomError> {
        for (kind, handler) in &element.handlers {
            self.dom.add_listener(dom, *kind, handler.clone())?;
        }
        Ok(())
    }

    /// 已有节点：处理器变了才重挂（没变 ⇒ 不动监听器）。
    ///
    /// 处理器**按指针身份**比较（见 [`crate::events::ElementHandler`]）：
    /// 同一闭包实例 ⇒ 视为同一个处理器（不重挂）；换了闭包 ⇒ 重挂。
    fn patch_handlers(
        &self,
        dom: &D::Node,
        old: &ElementVNode,
        new: &ElementVNode,
    ) -> Result<(), DomError> {
        if old.handlers == new.handlers {
            return Ok(());
        }
        self.dom.remove_listeners(dom)?;
        self.write_handlers(dom, new)
    }

    // ── `bind:this` 接线 ──────────────────────────────────────────────────
    /// `bind:this` 槽位变了 ⇒ 先解绑旧的，再接新的。
    fn rebind_this(
        &mut self,
        old: &ElementVNode,
        new: &ElementVNode,
        dom: &D::Node,
    ) -> Result<(), DomError> {
        if old.this_ref == new.this_ref {
            return Ok(());
        }
        if let Some(old_ref) = &old.this_ref {
            old_ref.unbind();
            self.bound_slots = self.bound_slots.saturating_sub(1);
        }
        if let Some(new_ref) = &new.this_ref {
            new_ref.bind_node(Box::new(dom.clone()))?;
            self.bound_slots += 1;
        }
        Ok(())
    }

    // ── 卸载 ──────────────────────────────────────────────────────────────

    /// 卸载一个已挂载节点（先解绑引用，再摘 DOM —— 设计文档 §5.1.4）。
    fn unmount_node(&mut self, node: &MountedNode<D>, parent: &D::Node) -> Result<(), DomError> {
        match node {
            MountedNode::Element(element) => {
                // 先摘监听器（节点离树之前，处理器不再能被触发）。
                self.dom.remove_listeners(&element.dom)?;
                // 再解绑 `bind:this`（清引用）—— 槽位里不留已销毁的句柄。
                if let Some(this_ref) = &element.vnode.this_ref {
                    this_ref.unbind();
                    self.bound_slots = self.bound_slots.saturating_sub(1);
                }
                for child in &element.children {
                    self.unmount_node(child, &element.dom)?;
                }
                self.dom.remove_child(parent, &element.dom)?;
            }
            MountedNode::Text(text) => {
                self.dom.remove_child(parent, &text.dom)?;
            }
            MountedNode::Fragment(fragment) => {
                for child in &fragment.children {
                    self.unmount_node(child, parent)?;
                }
            }
        }
        Ok(())
    }

    // ── 小工具 ────────────────────────────────────────────────────────────

    /// 把节点放进父节点（`anchor = None` ⇒ 追加到末尾）。
    fn insert_into(
        &self,
        parent: &D::Node,
        child: &D::Node,
        anchor: Option<&D::Node>,
    ) -> Result<(), DomError> {
        match anchor {
            None => self.dom.append_child(parent, child),
            Some(anchor) => self.dom.insert_before(parent, child, Some(anchor)),
        }
    }

    /// 某个节点**之后**的那个兄弟（替换子树时用它当锚点）。
    fn anchor_after(&self, parent: &D::Node, node: &D::Node) -> Result<Option<D::Node>, DomError> {
        let children = self.dom.children(parent)?;
        Ok(children
            .iter()
            .position(|child| child == node)
            .and_then(|index| children.get(index + 1).cloned()))
    }
}

/// SVG 命名空间（`createElementNS` 用的那个）。
pub const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// SVG 标签表（这些标签必须建在 SVG 命名空间里）。
///
/// 理由：HTML 命名空间里的 `<svg>` **不是** SVG 元素 —— 浏览器不把它当矢量图
/// 渲染，且 `viewBox` 这类驼峰属性名会被小写化 ⇒ 与参考实现的 DOM 逐字不一致
/// （画面也会不同）。
const SVG_TAGS: &[&str] = &[
    "circle",
    "clipPath",
    "defs",
    "ellipse",
    "foreignObject",
    "g",
    "image",
    "line",
    "linearGradient",
    "marker",
    "mask",
    "path",
    "pattern",
    "polygon",
    "polyline",
    "radialGradient",
    "rect",
    "stop",
    "svg",
    "symbol",
    "text",
    "tspan",
    "use",
];

/// 标签是否属于 SVG 命名空间（是 ⇒ 返回命名空间）。
pub fn svg_namespace(tag: &str) -> Option<&'static str> {
    if SVG_TAGS.contains(&tag) {
        Some(SVG_NAMESPACE)
    } else {
        None
    }
}

/// 子树里**最后一个**带宿主节点的句柄（分组节点自己没有宿主节点 ⇒ 往里找）。
///
/// 用于「替换子树」时定位插入锚点：分组节点的锚点是它最后一个子节点的锚点。
fn last_dom_node<D: Dom>(node: &MountedNode<D>) -> Option<&D::Node> {
    match node {
        MountedNode::Element(element) => Some(&element.dom),
        MountedNode::Text(text) => Some(&text.dom),
        MountedNode::Fragment(fragment) => fragment.children.last().and_then(last_dom_node),
    }
}

/// 把一组**已挂载**子节点里的 **Fragment 就地摊平**（Fragment 无宿主节点
/// ⇒ 它在 DOM 上就是它 children 的一组兄弟）。
///
/// ⇒ 摊平后序列里的每一项都**有**宿主节点 ⇒
/// ⇒ 差分的配对单位与 DOM 的实际结构**一致**。
fn flatten_mounted<D: Dom>(nodes: &[MountedNode<D>]) -> Vec<MountedNode<D>> {
    let mut out = Vec::with_capacity(nodes.len());
    for node in nodes {
        match node {
            MountedNode::Fragment(fragment) => {
                out.extend(flatten_mounted(&fragment.children));
            }
            other => out.push(other.clone()),
        }
    }
    out
}

/// 把一组**虚拟**子节点里的 **Fragment 就地摊平**（与 [`flatten_mounted`] 同口径）。
fn flatten_vnode(nodes: &[VNode]) -> Vec<VNode> {
    let mut out = Vec::with_capacity(nodes.len());
    for node in nodes {
        match node {
            VNode::Fragment(children) => {
                out.extend(flatten_vnode(children));
            }
            other => out.push(other.clone()),
        }
    }
    out
}

/// 读元素的 class token 集合（保序）。
fn class_tokens(element: &ElementVNode) -> Vec<String> {
    match element.attr_value("class") {
        Some(AttrValue::Text(text)) => text.split_whitespace().map(str::to_owned).collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    //! 渲染器的行为断言 + **T1–T6 六条保护规则的机械判定**。
    //!
    //! 判据全部落在**可观测的行为**上（操作序列 / 节点身份 / 树形态），
    //! **不读源码文本**（设计文档 §10.2 明令禁止源码镜像断言）。

    use super::*;
    use crate::binding::ThisBinding;
    use crate::dom::{DomOp, InMemoryDom, InMemoryNodeId, RecordedOp};
    use std::cell::Cell;
    use std::rc::Rc;

    // ── 测试脚手架 ────────────────────────────────────────────────────────

    /// 建渲染器，并保留一份后端句柄（`InMemoryDom` 不是 `Clone` ⇒ 用
    /// `Renderer::dom()` 读回去）。
    fn renderer_with_dom() -> (Renderer<InMemoryDom>, InMemoryNodeId) {
        let dom = InMemoryDom::new();
        let root = dom.create_element("main").expect("建节点必须成功");
        let renderer = Renderer::new(dom, root);
        (renderer, root)
    }

    /// 带 key 的列表项。
    fn keyed_item(key: &str, text: &str) -> VNode {
        VNode::element("li")
            .key(key)
            .child(VNode::text(text))
            .build()
    }

    /// 是不是「属性写入」类操作。
    fn is_attribute_op(op: &DomOp) -> bool {
        matches!(
            op,
            DomOp::SetAttribute { .. }
                | DomOp::RemoveAttribute { .. }
                | DomOp::SetClassTokens { .. }
                | DomOp::SetProperty { .. }
        )
    }

    /// 是不是「树结构」类操作（增 / 删 / 移动节点）。
    fn is_tree_op(op: &DomOp) -> bool {
        matches!(
            op,
            DomOp::AppendChild { .. } | DomOp::InsertBefore { .. } | DomOp::RemoveChild { .. }
        )
    }

    /// 某节点的全部「属性写入」操作下标。
    fn attribute_op_indices(ops: &[RecordedOp], node: InMemoryNodeId) -> Vec<usize> {
        ops.iter()
            .enumerate()
            .filter(|(_, recorded)| recorded.node == node && is_attribute_op(&recorded.op))
            .map(|(index, _)| index)
            .collect()
    }

    /// 某节点「进入文档树」的操作下标（`append_child` / `insert_before`）。
    fn insertion_op_index(ops: &[RecordedOp], node: InMemoryNodeId) -> Option<usize> {
        ops.iter().position(|recorded| {
            recorded.node == node
                && matches!(
                    recorded.op,
                    DomOp::AppendChild { .. } | DomOp::InsertBefore { .. }
                )
        })
    }

    /// 某节点「被摘除」的操作下标。
    fn removal_op_index(ops: &[RecordedOp], node: InMemoryNodeId) -> Option<usize> {
        ops.iter().position(|recorded| {
            recorded.node == node && matches!(recorded.op, DomOp::RemoveChild { .. })
        })
    }

    /// 本批次里被创建过的节点（`create_element` / `create_text` 的对象）。
    fn created_nodes(ops: &[RecordedOp]) -> Vec<InMemoryNodeId> {
        ops.iter()
            .filter(|recorded| {
                matches!(
                    recorded.op,
                    DomOp::CreateElement { .. } | DomOp::CreateText { .. }
                )
            })
            .map(|recorded| recorded.node)
            .collect()
    }

    /// 挂载容器当前的子节点顺序。
    fn child_order(renderer: &Renderer<InMemoryDom>) -> Vec<InMemoryNodeId> {
        renderer
            .dom()
            .children(renderer.root())
            .expect("读子节点必须成功")
    }

    // ── 渲染器基础行为 ────────────────────────────────────────────────────

    #[test]
    fn mount_builds_the_tree_into_the_backend() {
        let (mut renderer, root) = renderer_with_dom();
        let view = VNode::element("section")
            .class("ap-shell")
            .attr("role", "group")
            .child(VNode::element("h3").child(VNode::text("标题")).build())
            .build();

        renderer.mount(view).expect("挂载必须成功");

        let children = renderer.dom().children(&root).expect("读子节点必须成功");
        assert_eq!(children.len(), 1);
        let section = children[0];
        assert_eq!(renderer.dom().tag(section), Some(String::from("section")));
        assert_eq!(
            renderer.dom().attribute(section, "class"),
            Some(String::from("ap-shell"))
        );
        assert_eq!(renderer.dom().text_content(section), "标题");
    }

    #[test]
    fn patch_updates_attributes_in_place() {
        let (mut renderer, _root) = renderer_with_dom();
        renderer
            .mount(
                VNode::element("div")
                    .class("ap-a")
                    .attr("title", "旧")
                    .build(),
            )
            .expect("挂载必须成功");
        let node = child_order(&renderer)[0];

        renderer
            .patch(
                VNode::element("div")
                    .class("ap-b")
                    .attr("title", "新")
                    .build(),
            )
            .expect("打补丁必须成功");

        assert_eq!(child_order(&renderer)[0], node, "节点身份必须不变");
        assert_eq!(
            renderer.dom().attribute(node, "class"),
            Some(String::from("ap-b"))
        );
        assert_eq!(
            renderer.dom().attribute(node, "title"),
            Some(String::from("新"))
        );
    }

    #[test]
    fn patch_updates_text_without_rebuilding_the_text_node() {
        let (mut renderer, _root) = renderer_with_dom();
        renderer
            .mount(VNode::element("p").child(VNode::text("旧")).build())
            .expect("挂载必须成功");
        let node = child_order(&renderer)[0];
        let text_node = renderer.dom().children(&node).expect("读子节点必须成功")[0];

        renderer
            .patch(VNode::element("p").child(VNode::text("新")).build())
            .expect("打补丁必须成功");

        assert_eq!(
            renderer.dom().children(&node).expect("读子节点必须成功")[0],
            text_node,
            "文本节点必须就地改，不重建"
        );
        assert_eq!(renderer.dom().text_content(node), "新");
    }

    #[test]
    fn unmount_removes_the_tree_and_clears_the_bindings() {
        let (mut renderer, root) = renderer_with_dom();
        let binding: ThisBinding<InMemoryDom> = ThisBinding::new();
        let view = VNode::element("canvas").this(binding.erased()).build();

        renderer.mount(view).expect("挂载必须成功");
        let node = child_order(&renderer)[0];
        assert!(binding.is_bound());

        renderer.unmount().expect("卸载必须成功");

        assert!(renderer
            .dom()
            .children(&root)
            .expect("读子节点必须成功")
            .is_empty());
        assert!(renderer.dom().is_detached(node));
        assert!(!binding.is_bound(), "卸载 ⇒ 清引用");
        assert_eq!(renderer.bound_slots(), 0, "挂载 / 卸载必须配平");
        assert!(!renderer.is_mounted());
    }

    #[test]
    fn mounting_twice_is_refused_instead_of_silently_replacing() {
        let (mut renderer, _root) = renderer_with_dom();
        renderer
            .mount(VNode::element("div").build())
            .expect("挂载必须成功");

        let err = renderer
            .mount(VNode::element("div").build())
            .expect_err("重复挂载必须报错");
        assert!(matches!(err, DomError::Undecidable(_)), "实测：{err:?}");
    }

    #[test]
    fn patching_without_mounting_is_refused() {
        let (mut renderer, _root) = renderer_with_dom();
        let err = renderer
            .patch(VNode::element("div").build())
            .expect_err("未挂载就打补丁必须报错");
        assert!(matches!(err, DomError::Undecidable(_)), "实测：{err:?}");
    }

    #[test]
    fn mount_and_unmount_can_be_repeated_in_pairs() {
        let (mut renderer, _root) = renderer_with_dom();
        for _ in 0..3 {
            renderer
                .mount(VNode::element("div").child(VNode::text("x")).build())
                .expect("挂载必须成功");
            assert!(renderer.is_mounted());
            renderer.unmount().expect("卸载必须成功");
            assert!(!renderer.is_mounted());
            assert_eq!(renderer.bound_slots(), 0);
        }
    }

    #[test]
    fn unkeyed_children_are_paired_by_position() {
        let (mut renderer, _root) = renderer_with_dom();
        renderer
            .mount(
                VNode::element("ul")
                    .child(VNode::element("li").child(VNode::text("1")).build())
                    .child(VNode::element("li").child(VNode::text("2")).build())
                    .build(),
            )
            .expect("挂载必须成功");
        let list = child_order(&renderer)[0];
        let before = renderer.dom().children(&list).expect("读子节点必须成功");

        renderer
            .patch(
                VNode::element("ul")
                    .child(VNode::element("li").child(VNode::text("x")).build())
                    .child(VNode::element("li").child(VNode::text("y")).build())
                    .build(),
            )
            .expect("打补丁必须成功");

        let after = renderer.dom().children(&list).expect("读子节点必须成功");
        assert_eq!(before, after, "按位置复用 ⇒ 节点身份不变");
        assert_eq!(renderer.dom().text_content(list), "xy");
    }

    #[test]
    fn keyed_reorder_moves_only_the_lis_lower_bound_of_nodes() {
        let (mut renderer, _root) = renderer_with_dom();
        let old: Vec<VNode> = ["a", "b", "c", "d"]
            .iter()
            .map(|key| keyed_item(key, key))
            .collect();
        renderer
            .mount(VNode::element("ul").children(old).build())
            .expect("挂载必须成功");
        let list = child_order(&renderer)[0];
        let before = renderer.dom().children(&list).expect("读子节点必须成功");

        let new: Vec<VNode> = ["d", "a", "b", "c"]
            .iter()
            .map(|key| keyed_item(key, key))
            .collect();
        renderer.dom().clear_ops();
        renderer
            .patch(VNode::element("ul").children(new).build())
            .expect("打补丁必须成功");

        let after = renderer.dom().children(&list).expect("读子节点必须成功");
        assert_eq!(after.len(), 4);
        assert_eq!(after[0], before[3], "d 移到最前（身份不变）");
        assert_eq!(after[1], before[0]);
        assert_eq!(after[2], before[1]);
        assert_eq!(after[3], before[2]);

        let moves = renderer
            .dom()
            .ops()
            .iter()
            .filter(|recorded| matches!(recorded.op, DomOp::InsertBefore { .. }))
            .count();
        assert_eq!(moves, 1, "移动次数 = 配对数 − LIS 长度 = 1（下界）");
    }

    #[test]
    fn keyed_list_removes_the_nodes_whose_key_disappeared() {
        let (mut renderer, _root) = renderer_with_dom();
        renderer
            .mount(
                VNode::element("ul")
                    .children(vec![keyed_item("a", "1"), keyed_item("b", "2")])
                    .build(),
            )
            .expect("挂载必须成功");
        let list = child_order(&renderer)[0];
        let removed = renderer.dom().children(&list).expect("读子节点必须成功")[0];

        renderer
            .patch(
                VNode::element("ul")
                    .children(vec![keyed_item("b", "2")])
                    .build(),
            )
            .expect("打补丁必须成功");

        let after = renderer.dom().children(&list).expect("读子节点必须成功");
        assert_eq!(after.len(), 1);
        assert!(renderer.dom().is_detached(removed), "被移除的节点必须摘除");
    }

    #[test]
    fn keyed_list_creates_the_nodes_for_new_keys() {
        let (mut renderer, _root) = renderer_with_dom();
        renderer
            .mount(
                VNode::element("ul")
                    .children(vec![keyed_item("a", "1")])
                    .build(),
            )
            .expect("挂载必须成功");
        let list = child_order(&renderer)[0];

        renderer
            .patch(
                VNode::element("ul")
                    .children(vec![keyed_item("a", "1"), keyed_item("c", "3")])
                    .build(),
            )
            .expect("打补丁必须成功");

        let after = renderer.dom().children(&list).expect("读子节点必须成功");
        assert_eq!(after.len(), 2);
        assert_eq!(renderer.dom().text_content(after[1]), "3");
    }

    #[test]
    fn duplicate_keys_are_reported_as_undecidable_and_touch_nothing() {
        let (mut renderer, _root) = renderer_with_dom();
        renderer
            .mount(
                VNode::element("ul")
                    .children(vec![keyed_item("a", "1")])
                    .build(),
            )
            .expect("挂载必须成功");
        let list = child_order(&renderer)[0];
        renderer.dom().clear_ops();

        let err = renderer
            .patch(
                VNode::element("ul")
                    .children(vec![keyed_item("x", "1"), keyed_item("x", "2")])
                    .build(),
            )
            .expect_err("重复 key 必须显式报错");
        assert!(matches!(err, DomError::Undecidable(_)), "实测：{err:?}");
        assert!(
            renderer.dom().ops().is_empty(),
            "差分不可判定 ⇒ DOM 一个字节都不许动"
        );
        assert_eq!(
            renderer
                .dom()
                .children(&list)
                .expect("读子节点必须成功")
                .len(),
            1
        );
    }

    #[test]
    fn fragment_children_are_mounted_into_the_parent() {
        let (mut renderer, root) = renderer_with_dom();
        let view = VNode::element("div")
            .child(VNode::fragment(vec![
                VNode::element("span").child(VNode::text("a")).build(),
                VNode::element("span").child(VNode::text("b")).build(),
            ]))
            .build();

        renderer.mount(view).expect("挂载必须成功");

        let div = renderer.dom().children(&root).expect("读子节点必须成功")[0];
        assert_eq!(
            renderer
                .dom()
                .children(&div)
                .expect("读子节点必须成功")
                .len(),
            2,
            "分组节点没有宿主节点 ⇒ 子节点直接落在父节点上"
        );
        assert_eq!(renderer.dom().text_content(div), "ab");
    }

    #[test]
    fn a_fragment_replaced_by_an_element_lands_in_the_right_slot() {
        let (mut renderer, root) = renderer_with_dom();
        renderer
            .mount(
                VNode::element("div")
                    .child(VNode::element("span").child(VNode::text("前")).build())
                    .child(VNode::fragment(vec![VNode::element("em").build()]))
                    .child(VNode::element("span").child(VNode::text("后")).build())
                    .build(),
            )
            .expect("挂载必须成功");
        let div = renderer.dom().children(&root).expect("读子节点必须成功")[0];
        assert_eq!(
            renderer
                .dom()
                .children(&div)
                .expect("读子节点必须成功")
                .len(),
            3
        );

        // 分组节点 ⇒ 换成元素：必须落在原来的位置（不是末尾）。
        renderer
            .patch(
                VNode::element("div")
                    .child(VNode::element("span").child(VNode::text("前")).build())
                    .child(VNode::element("strong").build())
                    .child(VNode::element("span").child(VNode::text("后")).build())
                    .build(),
            )
            .expect("打补丁必须成功");

        let children = renderer.dom().children(&div).expect("读子节点必须成功");
        assert_eq!(children.len(), 3);
        assert_eq!(
            renderer.dom().tag(children[1]),
            Some(String::from("strong")),
            "替换出来的节点必须落在原位"
        );
    }

    #[test]
    fn bind_this_slot_change_rebinds_the_reference() {
        let (mut renderer, _root) = renderer_with_dom();
        let first: ThisBinding<InMemoryDom> = ThisBinding::new();
        let second: ThisBinding<InMemoryDom> = ThisBinding::new();

        renderer
            .mount(VNode::element("canvas").this(first.erased()).build())
            .expect("挂载必须成功");
        let node = child_order(&renderer)[0];
        assert_eq!(first.get(), Some(node));
        assert!(!second.is_bound());

        renderer
            .patch(VNode::element("canvas").this(second.erased()).build())
            .expect("打补丁必须成功");

        assert!(!first.is_bound(), "旧槽位必须清引用");
        assert_eq!(second.get(), Some(node), "新槽位必须拿到节点");
        assert_eq!(renderer.bound_slots(), 1, "绑定数必须配平");
    }

    // ── 事件处理器：挂载 / 差分 / 卸载的成对契约 ─────────────────────────

    /// 建一个带点击处理器的按钮（处理器把计数器加一）。
    fn clickable_button(counter: Rc<Cell<u32>>) -> VNode {
        VNode::element("button")
            .handler(
                crate::dom::EventKind::Click,
                crate::events::ElementHandler::new(move |_event| {
                    counter.set(counter.get() + 1);
                }),
            )
            .build()
    }

    #[test]
    fn handlers_are_attached_on_mount_and_removed_on_unmount() {
        let (mut renderer, _root) = renderer_with_dom();
        let counter = Rc::new(Cell::new(0u32));
        renderer
            .mount(clickable_button(counter.clone()))
            .expect("挂载必须成功");

        let node = child_order(&renderer)[0];
        assert_eq!(
            renderer.dom().dispatch(node, crate::dom::EventKind::Click),
            1
        );
        assert_eq!(counter.get(), 1, "挂载后处理器必须能触发");

        renderer.unmount().expect("卸载必须成功");
        // 卸载后节点已摘除 ⇒ 派发不到任何处理器（成对契约）。
        assert!(renderer.dom().is_detached(node));
    }

    /// 一个元素上**多个事件类型**的处理器 ⇒ 都要挂上、都要能触发。
    ///
    /// 契约：挂载时每个事件类型各挂一个监听器；挂第二个**不得**把先挂的带走
    /// （真实 DOM 后端曾因此让「同时挂点击与键盘」的标题**点不动**）。
    #[test]
    fn several_event_kinds_on_one_element_all_fire() {
        let (mut renderer, _root) = renderer_with_dom();
        let clicks = Rc::new(Cell::new(0u32));
        let keys = Rc::new(Cell::new(0u32));

        let click_counter = clicks.clone();
        let key_counter = keys.clone();
        renderer
            .mount(
                VNode::element("div")
                    .handler(
                        crate::dom::EventKind::Click,
                        crate::events::ElementHandler::new(move |_event| {
                            click_counter.set(click_counter.get() + 1);
                        }),
                    )
                    .handler(
                        crate::dom::EventKind::KeyDown,
                        crate::events::ElementHandler::new(move |_event| {
                            key_counter.set(key_counter.get() + 1);
                        }),
                    )
                    .build(),
            )
            .expect("挂载必须成功");

        let node = child_order(&renderer)[0];
        assert_eq!(
            renderer.dom().dispatch(node, crate::dom::EventKind::Click),
            1
        );
        assert_eq!(
            renderer
                .dom()
                .dispatch(node, crate::dom::EventKind::KeyDown),
            1
        );
        assert_eq!(clicks.get(), 1, "点击处理器必须仍能触发");
        assert_eq!(keys.get(), 1, "键盘处理器必须能触发");
    }

    /// **反例**：换了处理器 ⇒ 重挂监听器（新的处理器生效、旧的失效）。
    #[test]
    fn patching_with_another_handler_rebinds_the_listener() {
        let (mut renderer, _root) = renderer_with_dom();
        let first = Rc::new(Cell::new(0u32));
        let second = Rc::new(Cell::new(0u32));

        renderer
            .mount(clickable_button(first.clone()))
            .expect("挂载必须成功");
        let node = child_order(&renderer)[0];

        let second_counter = second.clone();
        renderer
            .patch(
                VNode::element("button")
                    .handler(
                        crate::dom::EventKind::Click,
                        crate::events::ElementHandler::new(move |_event| {
                            second_counter.set(second_counter.get() + 1);
                        }),
                    )
                    .build(),
            )
            .expect("打补丁必须成功");

        assert_eq!(
            renderer.dom().dispatch(node, crate::dom::EventKind::Click),
            1
        );
        assert_eq!(first.get(), 0, "旧处理器必须已被换掉");
        assert_eq!(second.get(), 1, "新处理器必须已挂上");
    }

    /// **不误报**：处理器没换 ⇒ 不重挂（监听器不被反复拆装）。
    #[test]
    fn patching_with_the_same_handler_keeps_the_listener() {
        let (mut renderer, _root) = renderer_with_dom();
        let counter = Rc::new(Cell::new(0u32));

        // 同一个闭包 ⇒ 同一个处理器（按指针身份）。
        let handler = crate::events::ElementHandler::new(move |_event| {
            counter.set(counter.get() + 1);
        });
        let view = VNode::element("button")
            .handler(crate::dom::EventKind::Click, handler.clone())
            .build();

        renderer.mount(view.clone()).expect("挂载必须成功");
        let node = child_order(&renderer)[0];
        renderer.dom().clear_ops();

        renderer.patch(view).expect("打补丁必须成功");
        let removals = renderer
            .dom()
            .ops()
            .iter()
            .filter(|recorded| matches!(recorded.op, DomOp::RemoveListeners))
            .count();
        assert_eq!(removals, 0, "处理器没换 ⇒ 不该重挂监听器");
        assert_eq!(
            renderer.dom().dispatch(node, crate::dom::EventKind::Click),
            1
        );
    }

    // ── T1：同标签 ⇒ 原地打补丁，绝不重建元素 ──────────────────────────

    #[test]
    fn t1_same_tag_is_patched_in_place_and_never_rebuilt() {
        let (mut renderer, _root) = renderer_with_dom();
        renderer
            .mount(
                VNode::element("div")
                    .class("ap-a")
                    .child(VNode::text("x"))
                    .build(),
            )
            .expect("挂载必须成功");
        let node = child_order(&renderer)[0];
        renderer.dom().clear_ops();

        renderer
            .patch(
                VNode::element("div")
                    .class("ap-b")
                    .child(VNode::text("y"))
                    .build(),
            )
            .expect("打补丁必须成功");

        assert_eq!(child_order(&renderer)[0], node, "节点身份必须不变");
        let created = created_nodes(&renderer.dom().ops());
        assert!(
            created.is_empty(),
            "同标签 ⇒ 一个新节点都不许建：{created:?}"
        );
    }

    /// **T1 的反例**：标签变了就必须重建（否则属性会串）。
    #[test]
    fn t1_counter_example_a_different_tag_is_rebuilt() {
        let (mut renderer, _root) = renderer_with_dom();
        renderer
            .mount(VNode::element("div").build())
            .expect("挂载必须成功");
        let node = child_order(&renderer)[0];
        renderer.dom().clear_ops();

        renderer
            .patch(VNode::element("span").build())
            .expect("打补丁必须成功");

        assert_ne!(child_order(&renderer)[0], node, "标签变了 ⇒ 必须换节点");
        assert_eq!(created_nodes(&renderer.dom().ops()).len(), 1);
    }

    // ── T2：同一节点的所有属性写入连续完成 ───────────────────────────────

    #[test]
    fn t2_attribute_writes_of_one_node_are_contiguous() {
        let (mut renderer, _root) = renderer_with_dom();
        renderer
            .mount(
                VNode::element("div")
                    .attr("id", "a")
                    .attr("title", "b")
                    .attr("role", "c")
                    .child(VNode::element("span").attr("id", "d").build())
                    .build(),
            )
            .expect("挂载必须成功");

        let ops = renderer.dom().ops();
        let mut checked = 0;
        for recorded in &ops {
            if !matches!(
                recorded.op,
                DomOp::CreateElement { .. } | DomOp::CreateText { .. }
            ) {
                continue;
            }
            let node = recorded.node;
            let indices = attribute_op_indices(&ops, node);
            if indices.is_empty() {
                continue;
            }
            checked += 1;
            let first = indices[0];
            let last = indices[indices.len() - 1];
            for (index, recorded) in ops.iter().enumerate().take(last + 1).skip(first) {
                assert!(
                    !is_tree_op(&recorded.op),
                    "节点 {node:?} 的属性写入之间混进了树操作（第 {index} 步）"
                );
            }
        }
        assert!(checked >= 2, "至少两个节点带属性写入（否则断言是空的）");
    }

    /// **T2 的反例（不误报）**：不同节点各自的属性写入**允许**交错存在，
    /// 只要**同一个节点**的写入是连续的。
    #[test]
    fn t2_counter_example_two_nodes_may_each_write_attributes() {
        let (mut renderer, _root) = renderer_with_dom();
        renderer
            .mount(
                VNode::element("div")
                    .attr("id", "a")
                    .child(VNode::element("span").attr("id", "b").build())
                    .build(),
            )
            .expect("挂载必须成功");

        let ops = renderer.dom().ops();
        let writers: Vec<InMemoryNodeId> = ops
            .iter()
            .filter(|recorded| is_attribute_op(&recorded.op))
            .map(|recorded| recorded.node)
            .collect();
        assert!(
            writers.len() >= 2,
            "两个节点各写一次属性 ⇒ 不是误报：{writers:?}"
        );
    }

    // ── T3：新插入节点先写完全部属性，再进入文档树 ──────────────────────

    #[test]
    fn t3_new_node_gets_all_attributes_before_entering_the_document() {
        let (mut renderer, _root) = renderer_with_dom();
        renderer
            .mount(
                VNode::element("div")
                    .attr("id", "a")
                    .attr("title", "b")
                    .class("ap-c")
                    .child(VNode::element("span").attr("id", "d").build())
                    .build(),
            )
            .expect("挂载必须成功");

        let ops = renderer.dom().ops();
        for node in created_nodes(&ops) {
            let Some(insertion) = insertion_op_index(&ops, node) else {
                continue;
            };
            for attribute in attribute_op_indices(&ops, node) {
                assert!(
                    attribute < insertion,
                    "节点 {node:?} 在进文档树之后才写属性（属性第 {attribute} 步 / 进树第 {insertion} 步）"
                );
            }
        }
    }

    /// **T3 的反例（不误报）**：已存在的节点写属性不受「进树顺序」约束
    /// （它早就在树里了）。
    #[test]
    fn t3_counter_example_an_existing_node_may_be_written_later() {
        let (mut renderer, _root) = renderer_with_dom();
        renderer
            .mount(VNode::element("div").build())
            .expect("挂载必须成功");
        let node = child_order(&renderer)[0];
        renderer.dom().clear_ops();

        renderer
            .patch(VNode::element("div").attr("id", "late").build())
            .expect("打补丁必须成功");

        let ops = renderer.dom().ops();
        let attributes = attribute_op_indices(&ops, node);
        assert_eq!(attributes.len(), 1, "属性确实写了");
        assert!(
            insertion_op_index(&ops, node).is_none(),
            "已存在的节点不会重新进树 ⇒ 不该有插入操作"
        );
    }

    // ── T4：引擎零自主动画 ────────────────────────────────────────────────

    #[test]
    fn t4_engine_never_synthesizes_attribute_values() {
        let (mut renderer, _root) = renderer_with_dom();
        let view = VNode::element("div")
            .attr("title", "标题")
            .class("ap-a ap-b")
            .child(
                VNode::element("span")
                    .attr("style", "opacity: 0.5")
                    .child(VNode::text("x"))
                    .build(),
            )
            .build();

        renderer.mount(view.clone()).expect("挂载必须成功");

        // 虚拟树里出现过的全部属性值（逐字）。
        let mut allowed: Vec<String> = Vec::new();
        collect_attr_values(&view, &mut allowed);
        let ops = renderer.dom().ops();
        for recorded in &ops {
            match &recorded.op {
                DomOp::SetAttribute { value, .. } => {
                    assert!(
                        allowed.contains(value),
                        "引擎写了一个虚拟树里不存在的值：{value:?}"
                    );
                }
                DomOp::SetClassTokens { added, .. } => {
                    for token in added {
                        assert!(
                            allowed.contains(token),
                            "引擎写了一个虚拟树里不存在的 class token：{token:?}"
                        );
                    }
                }
                _ => {}
            }
        }
    }

    /// **T4 的反例（不误报）**：虚拟树里**本来就有**的内联样式必须**原样**
    /// 透传（引擎不加工、不插值、不丢弃）。
    #[test]
    fn t4_counter_example_inline_style_is_passed_through_verbatim() {
        let (mut renderer, _root) = renderer_with_dom();
        renderer
            .mount(VNode::element("span").attr("style", "opacity: 0.5").build())
            .expect("挂载必须成功");

        let ops = renderer.dom().ops();
        let styles: Vec<&String> = ops
            .iter()
            .filter_map(|recorded| match &recorded.op {
                DomOp::SetAttribute { name, value } if name == "style" => Some(value),
                _ => None,
            })
            .collect();
        assert_eq!(
            styles,
            vec![&String::from("opacity: 0.5")],
            "内联样式必须逐字透传，且只写一次"
        );
    }

    #[test]
    fn t4_engine_creates_no_transient_node_in_one_batch() {
        let (mut renderer, _root) = renderer_with_dom();
        renderer
            .mount(
                VNode::element("div")
                    .child(VNode::element("span").child(VNode::text("x")).build())
                    .build(),
            )
            .expect("挂载必须成功");
        renderer.dom().clear_ops();

        // 一次批次里既换标签（建新节点）又删旧节点。
        renderer
            .patch(
                VNode::element("div")
                    .child(VNode::element("p").child(VNode::text("y")).build())
                    .build(),
            )
            .expect("打补丁必须成功");

        let ops = renderer.dom().ops();
        for node in created_nodes(&ops) {
            assert!(
                removal_op_index(&ops, node).is_none(),
                "节点 {node:?} 在同一批次里被建出来又被摘掉 ⇒ CSS 过渡必然丢"
            );
        }
    }

    // ── T5：移除时机与参考实现一致，不加延迟 ────────────────────────────

    #[test]
    fn t5_removal_happens_in_the_same_batch_without_delay() {
        let (mut renderer, _root) = renderer_with_dom();
        renderer
            .mount(
                VNode::element("div")
                    .child(VNode::element("span").child(VNode::text("留")).build())
                    .child(VNode::element("span").child(VNode::text("走")).build())
                    .build(),
            )
            .expect("挂载必须成功");
        let div = child_order(&renderer)[0];
        let leaving = renderer.dom().children(&div).expect("读子节点必须成功")[1];
        renderer.dom().clear_ops();

        renderer
            .patch(
                VNode::element("div")
                    .child(VNode::element("span").child(VNode::text("留")).build())
                    .build(),
            )
            .expect("打补丁必须成功");

        assert!(
            renderer.dom().is_detached(leaving),
            "移除必须在这一批次里立即发生（不加延迟）"
        );
        let ops = renderer.dom().ops();
        assert!(
            removal_op_index(&ops, leaving).is_some(),
            "移除操作必须落在本批次里"
        );
    }

    /// **T5 的反例（不误报）**：没被移除的节点必须**仍然挂在树上**。
    #[test]
    fn t5_counter_example_a_kept_node_stays_attached() {
        let (mut renderer, _root) = renderer_with_dom();
        renderer
            .mount(
                VNode::element("div")
                    .child(VNode::element("span").child(VNode::text("留")).build())
                    .build(),
            )
            .expect("挂载必须成功");
        let div = child_order(&renderer)[0];
        let kept = renderer.dom().children(&div).expect("读子节点必须成功")[0];

        renderer
            .patch(
                VNode::element("div")
                    .child(VNode::element("span").child(VNode::text("留")).build())
                    .build(),
            )
            .expect("打补丁必须成功");

        assert!(!renderer.dom().is_detached(kept), "留下的节点不许被摘");
    }

    // ── T6：被移除的节点不参与本批次的写入 ──────────────────────────────

    #[test]
    fn t6_removed_node_receives_no_writes_in_the_same_batch() {
        let (mut renderer, _root) = renderer_with_dom();
        renderer
            .mount(
                VNode::element("div")
                    .child(VNode::element("span").child(VNode::text("留")).build())
                    .child(VNode::element("span").child(VNode::text("走")).build())
                    .build(),
            )
            .expect("挂载必须成功");
        let div = child_order(&renderer)[0];
        let leaving = renderer.dom().children(&div).expect("读子节点必须成功")[1];
        renderer.dom().clear_ops();

        renderer
            .patch(
                VNode::element("div")
                    .child(VNode::element("span").child(VNode::text("留")).build())
                    .build(),
            )
            .expect("打补丁必须成功");

        let ops = renderer.dom().ops();
        assert!(
            attribute_op_indices(&ops, leaving).is_empty(),
            "被移除的节点不许再收到属性写入"
        );
        assert_eq!(
            renderer
                .dom()
                .set_attribute(&leaving, "class", "ap-x")
                .expect_err("写入已摘除节点必须报错"),
            DomError::NodeDetached
        );
    }

    /// **T6 的反例（不误报）**：留下来的节点**照常**收到属性写入。
    #[test]
    fn t6_counter_example_a_kept_node_is_written_normally() {
        let (mut renderer, _root) = renderer_with_dom();
        renderer
            .mount(
                VNode::element("div")
                    .child(VNode::element("span").child(VNode::text("留")).build())
                    .child(VNode::element("span").child(VNode::text("走")).build())
                    .build(),
            )
            .expect("挂载必须成功");
        let div = child_order(&renderer)[0];
        let kept = renderer.dom().children(&div).expect("读子节点必须成功")[0];
        renderer.dom().clear_ops();

        renderer
            .patch(
                VNode::element("div")
                    .child(
                        VNode::element("span")
                            .attr("class", "ap-kept")
                            .child(VNode::text("留"))
                            .build(),
                    )
                    .build(),
            )
            .expect("打补丁必须成功");

        let ops = renderer.dom().ops();
        assert_eq!(
            attribute_op_indices(&ops, kept).len(),
            1,
            "留下来的节点照常写属性"
        );
    }

    // ── 工具 ──────────────────────────────────────────────────────────────

    /// 收集虚拟树里出现过的全部属性值 / class token（逐字）。
    fn collect_attr_values(node: &VNode, out: &mut Vec<String>) {
        if let VNode::Element(element) = node {
            for (name, value) in &element.attrs {
                if let Some(text) = value.to_attr_string() {
                    if name == "class" {
                        out.extend(text.split_whitespace().map(str::to_owned));
                    } else {
                        out.push(text);
                    }
                }
            }
        }
        for child in node.children() {
            collect_attr_values(child, out);
        }
    }
    // ── property 通道（`value` / `checked` 等 —— 与属性两条独立通道） ──────────

    /// **T-p1**：新建节点时 property 被写进去（`input` 的 `value` 不再依赖宿主回写）。
    #[test]
    fn a_new_nodes_properties_are_written() {
        let (mut renderer, _root) = renderer_with_dom();
        let tree = VNode::element("input")
            .attr("type", "text")
            .prop("value", "hello")
            .build();
        let mounted = renderer.mount(tree).expect("挂载必须成功");
        let element = *mounted.dom_node().expect("元素节点");
        let dom = renderer.dom();
        assert_eq!(dom.property(element, "value").as_deref(), Some("hello"));
        // ⚠️ **属性通道仍然没有 `value`** —— property 不等于属性（否则属性对照会多一项）。
        assert!(
            renderer.dom().attribute(element, "value").is_none(),
            "property 不应变成属性"
        );
    }

    /// ⭐ **Fragment 里的元素在重渲染后**不重建**（2026-10-25 修的核心缺陷）。
    ///
    /// ## 缺陷现象（实测，浏览器里）
    ///
    /// `view!` 把「内插一个变量」编译成 **Fragment** ⇒ 几乎每棵组件树里都有。
    /// 修前 `patch_children` **按「VNode 节点」配对** ⇒ old 侧一个 Fragment 对
    /// new 侧一个 Fragment ⇒ 而 `patch_node` 没有 `(Fragment, Fragment)` 分支
    /// ⇒ Fragment 落到「类型不匹配 ⇒ 替换子树」
    /// ⇒ ⇒ **每次重渲染都把整棵 Fragment 子树重建**
    /// ⇒ ⇒ **输入框元素被换掉** ⇒ 用户输入与焦点一起丢
    /// ⇒ ⇒ 实测：键入 1 个字符 ⇒ 取值 `''`、`activeElement` 掉到 `body`
    /// ⇒ ⇒ **受控输入在引擎侧不可用**（而像素对照看不出来 ⇒ 绿）。
    ///
    /// ## 判据（**可观测的节点身份**，不读源码文本）
    ///
    /// 同一个 Fragment 里的 `input` 节点，在第二次渲染后**必须是同一个节点**
    /// ⇒ 节点身份不变 ⇒ **没有重建**。
    #[test]
    fn a_fragment_subtree_is_not_rebuilt_on_rerender() {
        let (mut renderer, _root) = renderer_with_dom();
        let tree = VNode::element("div")
            .child(
                VNode::element("section")
                    .child(VNode::Fragment(vec![VNode::element("input")
                        .attr("type", "text")
                        .build()]))
                    .build(),
            )
            .build();
        let mounted = renderer.mount(tree).expect("挂载必须成功");
        let first = mounted.children().first().and_then(find_input);
        assert!(first.is_some(), "片段里应有 input");

        // 第二次渲染：结构相同 ⇒ **必须复用**同一个 input 节点。
        let tree2 = VNode::element("div")
            .child(
                VNode::element("section")
                    .child(VNode::Fragment(vec![VNode::element("input")
                        .attr("type", "text")
                        .build()]))
                    .build(),
            )
            .build();
        let mounted2 = renderer.patch(tree2).expect("重渲染必须成功");
        let second = mounted2.children().first().and_then(find_input);
        assert!(second.is_some(), "重渲染后仍应有 input");
        assert_eq!(
            first, second,
            "Fragment 里的节点在重渲染后**被重建了** ⇒ 输入框元素会被换掉 ⇒ 受控输入不可用"
        );
    }

    /// ⭐ **Fragment 里的顺序变化按位置对齐**（摊平后的既有语义）。
    ///
    /// 摊平后两侧序列都是「真实 DOM 节点」⇒ 位置配对才正确
    /// ⇒ 断言：第二个 Fragment 里的节点**不会**跑到第一个的位置。
    #[test]
    fn a_fragment_keeps_the_positions_of_its_children() {
        let (mut renderer, _root) = renderer_with_dom();
        let tree = VNode::element("div")
            .child(
                VNode::element("section")
                    .child(VNode::Fragment(vec![
                        VNode::element("span").attr("class", "a").build(),
                        VNode::element("span").attr("class", "b").build(),
                    ]))
                    .build(),
            )
            .build();
        let mounted = renderer.mount(tree).expect("挂载必须成功");
        let classes = mounted
            .children()
            .first()
            .map(collect_classes)
            .unwrap_or_default();
        assert_eq!(classes, vec!["a".to_string(), "b".to_string()]);

        // 第二次渲染：顺序**颠倒** ⇒ 位置配对 ⇒ 第一个位置仍是 `a`。
        let tree2 = VNode::element("div")
            .child(
                VNode::element("section")
                    .child(VNode::Fragment(vec![
                        VNode::element("span").attr("class", "b").build(),
                        VNode::element("span").attr("class", "a").build(),
                    ]))
                    .build(),
            )
            .build();
        let mounted2 = renderer.patch(tree2).expect("重渲染必须成功");
        let classes2 = mounted2
            .children()
            .first()
            .map(collect_classes)
            .unwrap_or_default();
        assert_eq!(
            classes2,
            vec!["b".to_string(), "a".to_string()],
            "Fragment 摊平后应按**位置**对齐 ⇒ 顺序变化要如实反映（**身份配对要靠 key**）"
        );
    }

    /// 在已挂载的子树里找第一个 `input` 节点（深度优先）。
    fn find_input(node: &MountedNode<InMemoryDom>) -> Option<InMemoryNodeId> {
        match node {
            MountedNode::Element(element) => {
                if element.vnode.tag == "input" {
                    return Some(element.dom);
                }
                element.children.iter().find_map(find_input)
            }
            MountedNode::Fragment(fragment) => fragment.children.iter().find_map(find_input),
            MountedNode::Text(_) => None,
        }
    }

    /// 收集子树里所有元素的 class（深度优先，保序）。
    fn collect_classes(node: &MountedNode<InMemoryDom>) -> Vec<String> {
        let mut out = Vec::new();
        fn walk(node: &MountedNode<InMemoryDom>, out: &mut Vec<String>) {
            match node {
                MountedNode::Element(element) => {
                    if let Some(class) = element.vnode.attr_value("class") {
                        if let Some(text) = class.to_attr_string() {
                            out.push(text);
                        }
                    }
                    for child in &element.children {
                        walk(child, out);
                    }
                }
                MountedNode::Fragment(fragment) => {
                    for child in &fragment.children {
                        walk(child, out);
                    }
                }
                MountedNode::Text(_) => {}
            }
        }
        walk(node, &mut out);
        out
    }

    /// **T-p2**：属性先写、property 后写 —— `<select>` 的 `value` 依赖 `option` 已存在。
    ///
    /// 判据落在**可观测的操作序列**上（不读源码文本）：
    /// 目标 `option` 的出现必须**早于** `select` 的 property 写入。
    #[test]
    fn properties_are_written_after_the_children_exist() {
        let (mut renderer, _root) = renderer_with_dom();
        let tree = VNode::element("select")
            .prop("value", "b")
            .child(VNode::element("option").attr("value", "a").build())
            .child(VNode::element("option").attr("value", "b").build())
            .build();
        renderer.mount(tree).expect("挂载必须成功");
        let ops = renderer.dom().ops();
        let select_value_at = ops
            .iter()
            .position(|op| matches!(op.op, DomOp::SetProperty { ref name, .. } if name == "value"))
            .expect("必须写 select 的 value");
        let last_option_at = ops
            .iter()
            .rposition(|op| matches!(op.op, DomOp::CreateElement { .. }))
            .expect("必须建 option");
        assert!(
            last_option_at < select_value_at,
            "property 必须在子节点之后写（select.value 依赖 option 已存在）"
        );
    }

    /// **T-p3**：property 变化 ⇒ 重写；未变化 ⇒ **不写**（保护输入法组合态与光标）。
    #[test]
    fn an_unchanged_property_is_not_rewritten() {
        let (mut renderer, _root) = renderer_with_dom();
        let first = VNode::element("input").prop("value", "same").build();
        renderer.mount(first.clone()).expect("挂载必须成功");
        renderer.dom().clear_ops();
        // 重渲染同一取值 ⇒ 不应产生 SetProperty。
        renderer.patch(first.clone()).expect("重渲染必须成功");
        let ops = renderer.dom().ops();
        assert!(
            !ops.iter()
                .any(|op| matches!(op.op, DomOp::SetProperty { .. })),
            "取值未变 ⇒ 不得重写 property（否则打断输入法组合态）"
        );
        // 取值变了 ⇒ 必须写。
        let second = VNode::element("input").prop("value", "changed").build();
        renderer.patch(second).expect("重渲染必须成功");
        let ops = renderer.dom().ops();
        assert!(
            ops.iter()
                .any(|op| matches!(op.op, DomOp::SetProperty { .. })),
            "取值变了 ⇒ 必须写 property"
        );
    }

    /// **T-p4**：布尔 property 走**布尔**通道（不是写字符串 `"true"` / `"false"`）。
    ///
    /// ⚠️ 这是本批最容易静默写错的一条：`Reflect::set(el, "checked", "false")`
    /// 写进去是**非空字符串** ⇒ 真值判断为真 ⇒ 语义反了。
    #[test]
    fn a_boolean_property_uses_the_boolean_channel() {
        let (mut renderer, _root) = renderer_with_dom();
        let tree = VNode::element("input")
            .attr("type", "checkbox")
            .prop("checked", true)
            .build();
        renderer.mount(tree).expect("挂载必须成功");
        let ops = renderer.dom().ops();
        assert!(
            ops.iter().any(|op| matches!(
                op.op,
                DomOp::SetPropertyBool { ref name, value } if name == "checked" && value
            )),
            "布尔 property 必须走 SetPropertyBool"
        );
        assert!(
            !ops.iter()
                .any(|op| matches!(op.op, DomOp::SetProperty { .. })),
            "布尔 property **不得**走字符串通道"
        );
    }
}
