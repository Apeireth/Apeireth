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
/// ## 失败之后：**显式劈叉**，不装作没事（2026-10-25 修）
///
/// 一次 patch 失败 ⇒ DOM 可能已经改了一半 ⇒ 渲染器的内存树与 DOM **劈叉**
/// ⇒ ⇒ 修前会把**过期**的旧树继续当差分基准 ⇒ ⇒ 每帧都失败 ⇒
/// ⇒ **界面永久冻结，且没有任何日志**。
///
/// ⇒ 现在：失败 ⇒ 标记 [`Renderer::is_desynced`] ⇒ **下一次 patch 走整树重建**
/// ⇒ ⇒ 宿主能查 [`Renderer::last_error`] ⇒ 能 log、能决定重建。
///
/// ## 诚实边界
///
/// - **事件委托的单监听器形态**：本 crate 按**元素直挂**（每个带处理器的元素
///   一个监听器，冒泡交给浏览器）；委托表在 [`crate::events`] 里独立存在。
/// - **effect 作用域未接线**：设计文档 §5.1.4 的「挂载 ⇒ 建作用域 / 卸载 ⇒
///   逆序销毁作用域」由 `ui-core` 的作用域机制承担，渲染器**不**直接操作它。
#[derive(Debug)]
pub struct Renderer<D: Dom> {
    dom: D,
    root: D::Node,
    tree: Option<MountedNode<D>>,
    /// 当前**已绑定**的 `bind:this` 槽位数（挂载 +1 / 卸载 −1）。
    bound_slots: usize,
    /// 渲染器与 DOM 是否**劈叉**（一次失败的 patch 之后为 `true`）。
    ///
    /// 劈叉 ⇒ 内存树**不可信** ⇒ 下一次 patch **整树重建**（不再拿旧树差分）。
    desynced: bool,
    /// 最近一次失败的原因（**成功后不清** ⇒ 宿主可随时取走写日志）。
    last_error: Option<DomError>,
    /// 降级告警（例如「后端不支持移除 property ⇒ 已写回默认值」）。
    warnings: Vec<String>,
}

impl<D: Dom> Renderer<D> {
    /// 建一个渲染器（挂载容器 = `root`）。
    pub fn new(dom: D, root: D::Node) -> Self {
        Self {
            dom,
            root,
            tree: None,
            bound_slots: 0,
            desynced: false,
            last_error: None,
            warnings: Vec::new(),
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

    /// 渲染器与 DOM 是否**劈叉**（一次失败的 patch 之后为 `true`）。
    ///
    /// ## 为什么宿主必须知道
    ///
    /// 劈叉意味着「内存里的树**不再代表**真实的 DOM」⇒ ⇒ 此时的任何差分都
    /// 不可信 ⇒ ⇒ 渲染器会在下一次 patch 时**整树重建**⇒ 但宿主也**应当**
    /// 知道（要 log、要决定是否重建界面）。
    pub fn is_desynced(&self) -> bool {
        self.desynced
    }

    /// 最近一次失败的原因（**成功后不清** ⇒ 宿主可随时取走写日志）。
    pub fn last_error(&self) -> Option<&DomError> {
        self.last_error.as_ref()
    }

    /// **取走**最近一次失败的原因（取走后清空 ⇒ 适合「日志队列」式消费）。
    pub fn take_last_error(&mut self) -> Option<DomError> {
        self.last_error.take()
    }

    /// 降级告警（例：「后端不支持移除 property ⇒ 已写回默认值」）。
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// **取走**全部降级告警。
    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
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
    ///
    /// ⚠️ 若渲染器已**劈叉**（上一次 patch 失败）⇒ 本次挂载走**整树重建**
    /// ⇒ ⇒ 宿主不必自己清理 DOM。
    pub fn mount(&mut self, vnode: VNode) -> Result<MountedNode<D>, DomError> {
        if self.tree.is_some() {
            if !self.desynced {
                return Err(DomError::Undecidable(String::from(
                    "渲染器已挂载：挂载 / 卸载必须成对（先卸载再挂载）",
                )));
            }
            // 已劈叉 ⇒ 旧树不可信 ⇒ 整树重建。
            return self.rebuild(vnode);
        }
        let root = self.root.clone();
        match self.mount_node(&vnode, &root, None) {
            Ok(mounted) => {
                self.tree = Some(mounted.clone());
                Ok(mounted)
            }
            Err(error) => {
                // 挂载失败也可能留下半截 DOM ⇒ 同样标记劈叉（下次整树重建）。
                self.desynced = true;
                self.last_error = Some(error.clone());
                Err(error)
            }
        }
    }

    /// 打补丁：用新虚拟树替换当前已挂载的树。
    ///
    /// ## 失败之后会发生什么（2026-10-25 修）
    ///
    /// DOM 可能已经改了一半 ⇒ 内存树与 DOM **劈叉** ⇒ 此时**绝不能**再拿过期
    /// 的旧树继续差分（那会让每一帧都失败 ⇒ 界面永久冻结）⇒ ⇒
    /// 失败 ⇒ 标记 [`Self::is_desynced`] ⇒ **下一次 patch 整树重建**。
    ///
    /// 失败**照样返回 `Err`**（不吞掉）⇒ 宿主能 log、能决定重建。
    pub fn patch(&mut self, vnode: VNode) -> Result<MountedNode<D>, DomError> {
        if self.desynced {
            // 已劈叉 ⇒ 不拿过期旧树差分 ⇒ 整树重建。
            return self.rebuild(vnode);
        }
        let Some(old) = self.tree.take() else {
            return Err(DomError::Undecidable(String::from(
                "渲染器未挂载：先 mount 再 patch",
            )));
        };
        let root = self.root.clone();
        match self.patch_node(&old, &vnode, &root, None) {
            Ok(mounted) => {
                self.tree = Some(mounted.clone());
                Ok(mounted)
            }
            Err(error) => {
                // 旧树**只**留作「卸载时的清理清单」⇒ **不再**作为差分基准。
                self.tree = Some(old);
                // ⚠️ **只有「可能已经改了 DOM」的错误才标记劈叉**：
                // `Undecidable`（差分不可判定 / 挂载状态不对）发生在**触碰 DOM
                // 之前** ⇒ 内存树仍可信 ⇒ 不标记（否则一次重复 key 就会让
                // 渲染器误判成劈叉）。
                if !matches!(error, DomError::Undecidable(_)) {
                    self.desynced = true;
                }
                self.last_error = Some(error.clone());
                Err(error)
            }
        }
    }

    /// **整树重建**：清掉旧产物，重新挂载。
    ///
    /// ## 为什么必须「连旧产物一起清掉」
    ///
    /// 失败的 patch 可能已经建了**新节点**（它们不在旧树里 ⇒ 旧树卸载时
    /// 摘不掉）⇒ ⇒ 只卸载旧树会留下**孤儿节点**⇒ ⇒ DOM 里出现重复内容。
    ///
    /// ⇒ ⇒ 重建 = ① 尽力卸载旧树 → ② 清空容器（容器**归引擎独占**，
    /// 见本类型文档）→ ③ 重新挂载。
    fn rebuild(&mut self, vnode: VNode) -> Result<MountedNode<D>, DomError> {
        let root = self.root.clone();
        // ① 尽力卸载旧树（可能与 DOM 已经不一致 ⇒ 失败**不致命**）。
        if let Some(old) = self.tree.take() {
            let _ = self.unmount_node(&old, &root);
        }
        // ② 清空容器里的残留（劈叉留下的孤儿节点）。
        self.clear_children(&root)?;
        // ③ 重新挂载。
        let mounted = self.mount_node(&vnode, &root, None)?;
        self.tree = Some(mounted.clone());
        self.desynced = false;
        Ok(mounted)
    }

    /// 清空挂载容器的子节点（**重建**时用；容器归引擎独占）。
    fn clear_children(&self, parent: &D::Node) -> Result<(), DomError> {
        let children = self.dom.children(parent)?;
        for child in children {
            // 一个节点摘不掉不该让整次重建失败 ⇒ 尽力而为。
            let _ = self.dom.remove_child(parent, &child);
        }
        Ok(())
    }

    /// 卸载整棵树（解绑 `bind:this` ⇒ 摘除 DOM 节点）。
    ///
    /// 幂等：未挂载时是空操作。
    pub fn unmount(&mut self) -> Result<(), DomError> {
        let Some(old) = self.tree.take() else {
            return Ok(());
        };
        let root = self.root.clone();
        let result = self.unmount_node(&old, &root);
        self.desynced = false;
        result
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
    ///
    /// ## ⭐ `after` = **块锚点**（2026-10-25 修）
    ///
    /// `after` 是「本块之后**紧邻**的那个 DOM 节点」（`None` = 本块是最后一块）。
    /// 它由**块级**配对算出来（不是由 DOM 反查）⇒ ⇒
    /// - 本块**变大**（条件块从空变非空）⇒ 新内容插在 `after` **之前**
    ///   ⇒ **位置不丢**（修前是追加到父节点末尾 ⇒ 条件卡跑到容器底部）；
    /// - 本块被**替换**（类型 / 标签变了）⇒ 新节点插在 `after` **之前**
    ///   ⇒ 落回原位。
    ///
    /// ## 块 = 一个「虚拟节点 ↔ DOM 节点集合」的对应关系
    ///
    /// - 元素块 / 文本块 ⇒ 对应**一个** DOM 节点；
    /// - 分组块（Fragment）⇒ 对应**一组**兄弟 DOM 节点（它在 DOM 上被摊平）。
    ///
    /// ⇒ 块级配对（谁对谁）⇒ 块内配对（块里的内容怎么变）⇒ **两层分开**。
    /// ⭐⭐⭐ **先定状态、再动作**：判断「本节点的 keyed 子节点差分是否可判定」。
    ///
    /// ## 为什么需要它（台账 B1 的根因）
    ///
    /// `patch_node` 原顺序是「① 写 DOM 属性 → ② `patch_children` →
    /// `patch_keyed_children` 里 `diff_keyed` 才判定可判定性」。
    /// 一旦不可判定（典型：**重复 key**）：
    ///
    /// - 属性**已经写进 DOM**（`:515`），而内存树仍是旧 vnode（`:364` 的 `self.tree = Some(old)`）；
    /// - `Undecidable` 被显式**排除**在「标 `desynced`」之外（`:370-372`）
    ///   ⇒ ⇒ 渲染器**自称同步**、且**不会**走整树重建（`:388-401`）自愈
    ///   ⇒ ⇒ **静默分叉**，下一帧仍拿旧树差分。
    ///
    /// ⇒ ⇒ 把可判定性**提到写属性之前**：失败时 DOM **一个字节都没动**，
    /// `:367-369` 那条注释（「`Undecidable` 发生在触碰 DOM 之前」）**才真正成立**，
    /// 而不是像修之前那样只在「父元素属性恰好没变」的夹具下侥幸成立。
    ///
    /// ## 代价
    ///
    /// 只在会走 keyed 路径时预检，且只算**可判定性**（`patch_children` 里会重算一次
    /// 真正的计划）；n = **同一父节点下的直接子节点数**，通常个位数。
    fn ensure_children_keyed_decidable(
        &self,
        old: &[MountedNode<D>],
        new: &[VNode],
    ) -> Result<(), DomError> {
        let old_vnodes: Vec<VNode> = old.iter().filter_map(|child| child.to_vnode()).collect();
        if old_vnodes.len() != old.len() {
            // 有子节点取不到 vnode（文本 / 空块）⇒ ⇒ 一定不走 keyed 路径。
            return Ok(());
        }
        if crate::keyed::list_diff_mode(&old_vnodes, new) != crate::keyed::ListDiffMode::Keyed {
            return Ok(());
        }
        diff_keyed(&old_vnodes, new)
            .map(|_| ())
            .map_err(|error| DomError::Undecidable(format!("keyed 差分不可判定：{error}")))
    }

    fn patch_node(
        &mut self,
        old: &MountedNode<D>,
        new: &VNode,
        parent: &D::Node,
        after: Option<&D::Node>,
    ) -> Result<MountedNode<D>, DomError> {
        match (old, new) {
            (MountedNode::Element(old_element), VNode::Element(new_element)) => {
                if old_element.vnode.tag != new_element.tag {
                    // 标签变了 ⇒ 必须重建（T1 只保护「同标签」的情形）。
                    self.unmount_node(old, parent)?;
                    return self.mount_node(new, parent, after);
                }
                // ⭐⭐⭐ **先定状态、再动作**（台账 B1，2026-10-27 修）：
                //   必须先确定「本节点的子节点差分**能不能做**」，**再**写任何 DOM 属性。
                //   修之前的顺序是「先写属性 → 再在 keyed 差分里判定可判定性」
                //   ⇒ ⇒ 不可判定时属性**已经落进 DOM**，而内存树仍留旧值；
                //   更糟的是 `Undecidable` 被显式排除在「标劈叉」之外
                //   ⇒ ⇒ 渲染器**自称同步**且**不走**整树重建自愈 ⇒ ⇒ **静默分叉**。
                self.ensure_children_keyed_decidable(&old_element.children, &new_element.children)?;
                // T1：同标签 ⇒ 复用同一个真实 DOM 节点，只改属性与子节点。
                self.patch_attributes(&old_element.dom, &old_element.vnode, new_element)?;
                self.patch_handlers(&old_element.dom, &old_element.vnode, new_element)?;
                self.rebind_this(&old_element.vnode, new_element, &old_element.dom)?;
                // 元素的子节点 = **该元素的全部**子节点 ⇒ 尾部锚点 = `None`
                // （子节点都挂在元素内部）。
                let children = self.patch_children(
                    &old_element.dom,
                    &old_element.children,
                    &new_element.children,
                    None,
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
                // ⭐ **块内按「块」配对，摊平只做一层**（2026-10-25 修）。
                //
                // ## 为什么**不能**再摊平（修前的缺陷，实测）
                //
                // 修前 `flatten_mounted` / `flatten_vnode` **递归**摊平所有嵌套
                // Fragment ⇒ 嵌套块的身份被**融进兄弟序列** ⇒
                // ⇒ 实测：`div > Fragment[span(x), Fragment([]), span(z)]`，
                // 条件块由假变真 ⇒ 摊平后 old = [x, z]、new = [x, y, z]
                // ⇒ ⇒ 按位置配对 ⇒ **id4 那颗节点从「z」变成了「y」**
                // ⇒ ⇒ 「z」被**重建**成新节点（id6）⇒
                // ⇒ ⇒ `<input>` 的值 / 焦点跟错逻辑项、CSS 过渡在错节点上重启、
                // `bind:this` 指向错元素。
                //
                // ## 现在的做法（**语义正确**的那一套）
                //
                // - **摊平只做一层**：嵌套 Fragment 保持为**子块** ⇒
                //   ⇒ 它的身份**独立**于兄弟序列 ⇒ ⇒ 兄弟不会错位；
                // - 嵌套块**自己**按块级规则配对（位置 / key）⇒
                //   ⇒ 条件块从「空」变「非空」时，它的内容插在**自己的后继**
                //   （块锚点）之前 ⇒ ⇒ **位置不丢**。
                //
                // ## 为什么这是**语义正确**的方案（而不是「让摊平后的配对带身份」）
                //
                // 「摊平 + 带身份」需要给**每个**嵌套块发明一个身份（key 或序号）
                // ⇒ ⇒ 身份得**跨帧稳定** ⇒ ⇒ 而嵌套块在虚拟树里**没有**稳定
                // 身份（`view!` 每次生成的 Fragment 都没有 key）⇒ ⇒ 只能靠
                // 「位置」⇒ ⇒ 位置一变，身份就变 ⇒ ⇒ **还是**会错位。
                // ⇒ ⇒ 保留「嵌套块」这个**层级**才是唯一能保住身份的办法。
                let children =
                    self.patch_children(parent, &old_fragment.children, new_children, after)?;
                Ok(MountedNode::Fragment(MountedFragment { children }))
            }
            _ => {
                // 类型不匹配 ⇒ 替换子树（落在**原位**：锚点 = 本块的后继）。
                self.unmount_node(old, parent)?;
                self.mount_node(new, parent, after)
            }
        }
    }

    /// **块级**配对：old 的每一项与 new 的每一项都是**块**。
    ///
    /// ## ⭐ 两条规则（合起来才是「块语义」）
    ///
    /// 1. **块的身份按位置 / key 配对**（`{ expr }` 这种插值恒占一个位置）
    ///    ⇒ ⇒ 某个块「出现 / 消失」**不会**让后面的兄弟错位
    ///    （这与生产侧 Svelte 的 `{#if}` 块锚点语义一致）。
    /// 2. **块内的增删按块锚点落位**（见 [`Self::patch_node`] 的 `after`）
    ///    ⇒ ⇒ 条件块从空变非空时，内容**留在原位**（不追加到父节点末尾）。
    ///
    /// ## `after` = 块列表**之后**的锚点
    ///
    /// - 元素的子节点列表 = 该元素的**全部**子节点 ⇒ `after = None`；
    /// - 分组的子节点列表 = 父节点子列表的**一段** ⇒ `after` = 该分组的后继
    ///   ⇒ ⇒ 分组里新增的内容插在**分组原来的位置**（而不是父节点末尾）。
    fn patch_children(
        &mut self,
        parent: &D::Node,
        old: &[MountedNode<D>],
        new: &[VNode],
        after: Option<&D::Node>,
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
            self.patch_keyed_children(parent, old, new, &old_vnodes, after)
        } else if has_key {
            self.patch_mixed_children(parent, old, new, after)
        } else {
            self.patch_positional_children(parent, old, new, after)
        }
    }

    /// **位置配对**（unkeyed）：old[i] ↔ new[i]。
    ///
    /// ## ⭐ 块锚点的落位算法（2026-10-25 修）
    ///
    /// **自后向前**处理 ⇒ 维护一个「后继块的**首** DOM 节点」作为锚点 ⇒
    /// ⇒ 新增的块 / 块内新增的内容**插在该锚点之前** ⇒ ⇒ **位置不丢**。
    ///
    /// ⚠️ **为什么是「首」节点而不是「尾」节点**：块 i 的后继是块 i+1 ⇒
    /// ⇒ 块 i 的内容必须排在块 i+1 的**第一个** DOM 节点**之前** ⇒
    /// ⇒ 锚点 = 后继块的首节点。
    ///
    /// ⚠️ **空块怎么办**：空 Fragment 没有 DOM 节点 ⇒ ⇒ 锚点**继续往回找**
    /// （不更新）⇒ ⇒ 后面的块仍然排在正确的位置。
    fn patch_positional_children(
        &mut self,
        parent: &D::Node,
        old: &[MountedNode<D>],
        new: &[VNode],
        after: Option<&D::Node>,
    ) -> Result<Vec<MountedNode<D>>, DomError> {
        let common = old.len().min(new.len());
        // ① 多余的旧块：移除（自后向前 ⇒ 保持前面的下标有效）。
        //    移除的都是**尾部**的块 ⇒ 不会影响前面块的锚点。
        for index in (common..old.len()).rev() {
            self.unmount_node(&old[index], parent)?;
        }
        // ② 按新顺序落位（自后向前 ⇒ 锚点 = 已就位的后继块的首节点）。
        let mut slots: Vec<Option<MountedNode<D>>> = vec![None; new.len()];
        let mut anchor: Option<D::Node> = after.cloned();
        for index in (0..new.len()).rev() {
            let mounted = match old.get(index) {
                Some(old_child) => {
                    self.patch_node(old_child, &new[index], parent, anchor.as_ref())?
                }
                None => self.mount_node(&new[index], parent, anchor.as_ref())?,
            };
            // 空块（空 Fragment）没有 DOM 节点 ⇒ 锚点**不变**。
            if let Some(first) = first_dom_node(&mounted) {
                anchor = Some(first.clone());
            }
            slots[index] = Some(mounted);
        }
        Ok(slots.into_iter().flatten().collect())
    }

    /// keyed 子节点对位：按 key 配对 + LIS 最小移动（设计文档 §5.2.3）。
    fn patch_keyed_children(
        &mut self,
        parent: &D::Node,
        old: &[MountedNode<D>],
        new: &[VNode],
        old_vnodes: &[VNode],
        after: Option<&D::Node>,
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
        let mut anchor: Option<D::Node> = after.cloned();
        for new_index in (0..new.len()).rev() {
            match plan.matched()[new_index] {
                Some(old_index) => {
                    let Some(old_child) = old.get(old_index) else {
                        continue;
                    };
                    let patched =
                        self.patch_node(old_child, &new[new_index], parent, anchor.as_ref())?;
                    let dom = patched.dom_node().cloned();
                    // 不在 LIS 里的子节点要换位置（LIS 之外的必须移动 ⇒ 移动次数
                    // = 配对数 − LIS 长度 = 下界）。
                    //
                    // ⚠️ `is_retained` 是 **O(1)**（集合查表）⇒ 不是 O(N²)。
                    if !plan.is_retained(old_index) {
                        if let Some(node) = dom.as_ref() {
                            self.dom.insert_before(parent, node, anchor.as_ref())?;
                        }
                    }
                    // 锚点 = 该块的**首** DOM 节点（分组块 ⇒ 它的第一个子节点）。
                    anchor = first_dom_node(&patched).cloned();
                    slots[new_index] = Some(patched);
                }
                None => {
                    // 新建：属性先写、节点后进文档树（T3）。
                    let mounted = self.mount_node(&new[new_index], parent, anchor.as_ref())?;
                    anchor = first_dom_node(&mounted).cloned();
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
    /// [`Self::patch_positional_children`]（**既有契约完全不变**）。
    fn patch_mixed_children(
        &mut self,
        parent: &D::Node,
        old: &[MountedNode<D>],
        new: &[VNode],
        after: Option<&D::Node>,
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
        let mut anchor: Option<D::Node> = after.cloned();
        for new_index in (0..new.len()).rev() {
            match plan[new_index] {
                Some(old_index) => {
                    let Some(old_child) = old.get(old_index) else {
                        continue;
                    };
                    let patched =
                        self.patch_node(old_child, &new[new_index], parent, anchor.as_ref())?;
                    if !order_preserved {
                        // `insert_before` 对**已在文档里**的节点 ⇒ **移动**它
                        // ⇒ ⇒ DOM 顺序恒等于 new 顺序。
                        //
                        // ⚠️ 分组块（Fragment）没有**单个** DOM 节点 ⇒
                        // ⇒ 要整块移动（逐个插到锚点之前 ⇒ 块内顺序不变）。
                        self.move_block(parent, &patched, anchor.as_ref())?;
                    }
                    if let Some(first) = first_dom_node(&patched) {
                        anchor = Some(first.clone());
                    }
                    slots[new_index] = Some(patched);
                }
                None => {
                    let mounted = self.mount_node(&new[new_index], parent, anchor.as_ref())?;
                    if let Some(first) = first_dom_node(&mounted) {
                        anchor = Some(first.clone());
                    }
                    slots[new_index] = Some(mounted);
                }
            }
        }
        Ok(slots.into_iter().flatten().collect())
    }

    /// 把一个**块**整体移动到锚点之前（块内顺序保持）。
    ///
    /// 为什么需要：分组块（Fragment）在 DOM 上是**一组**兄弟节点 ⇒
    /// ⇒ 「移动一个块」= 「把它的每个 DOM 节点都插到锚点之前」⇒
    /// ⇒ 逐个插入 ⇒ 块内的相对顺序不变 ✓。
    fn move_block(
        &mut self,
        parent: &D::Node,
        block: &MountedNode<D>,
        anchor: Option<&D::Node>,
    ) -> Result<(), DomError> {
        for node in dom_nodes_of(block) {
            self.dom.insert_before(parent, &node, anchor)?;
        }
        Ok(())
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
    /// ## ⭐ 跳过条件（2026-10-25 重新审视，实时语义下）
    ///
    /// 后端的 [`Dom::get_property`] 是**实时**语义（浏览器读的是真实的
    /// `input.value` ⇒ 可能是**用户刚敲进去**的值）⇒ ⇒
    /// ⇒ **绝不能**拿「实时值」当「上一帧的目标值」来比 ⇒
    /// ⇒ 否则用户刚敲的字符会被引擎覆盖回去。
    ///
    /// ⇒ 跳过条件分两层：
    ///
    /// 1. **目标值没变**（`old` 与 `new` 的取值相同）⇒ **不写**
    ///    ⇒ 这一条**保护**用户的输入与输入法组合态（**不看**实时值）；
    /// 2. 目标值变了、但**实时值已经等于目标值** ⇒ 不写
    ///    ⇒ 避免一次**无谓**的重写（重写会推光标）。
    fn patch_props(
        &mut self,
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
                    // 当前值已是目标值 ⇒ 不写（避免无谓重写 / 光标跳动）。
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
                    // ⭐ property **从虚拟树上消失**（2026-10-25 修）。
                    //
                    // ## 修前的缺陷
                    //
                    // 修前直接 `Err(Unsupported)` ⇒ ⇒ **整帧中止** ⇒
                    // ⇒ DOM 已经改了一半 ⇒ 界面冻结（且宿主常常把错误吞掉）。
                    //
                    // ## 现在的口径
                    //
                    // 1. **首选真正移除**（[`Dom::remove_property`]）
                    //    ⇒ 之后读回来是**默认值** ⇒ 与浏览器一致；
                    // 2. 后端**做不了** ⇒ **降级**：写回**默认值** + **记警告**
                    //    ⇒ ⇒ **绝不中止整帧**（0 装：降级必须**看得见**）。
                    self.remove_property_or_default(dom, &name, old_value)?;
                }
            }
        }
        Ok(())
    }

    /// property 消失的处理：**优先真正移除**，后端做不了 ⇒ 写回默认值 + 记警告。
    fn remove_property_or_default(
        &mut self,
        dom: &D::Node,
        name: &str,
        old_value: Option<&PropValue>,
    ) -> Result<(), DomError> {
        match self.dom.remove_property(dom, name) {
            Ok(()) => Ok(()),
            Err(DomError::Unsupported(_)) => {
                // 后端做不了 ⇒ 写回**默认值**（字符串 ⇒ 空串；布尔 ⇒ false）。
                match old_value {
                    Some(PropValue::Bool(_)) => self.dom.set_property_bool(dom, name, false)?,
                    _ => self.dom.set_property(dom, name, "")?,
                }
                self.warnings.push(format!(
                    "property `{name}` 已从虚拟树消失，但后端不支持移除 ⇒ 已写回默认值（未真正移除）"
                ));
                Ok(())
            }
            Err(error) => Err(error),
        }
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
    ///
    /// ## ⭐ **按事件类型增量替换**（2026-10-25 修）
    ///
    /// 修前：只要处理器集合变了 ⇒ `remove_listeners`（摘**全部**）⇒
    /// ⇒ 再把**全部**处理器重挂 ⇒ ⇒ 每次重渲染、每个元素都**全量拆装**
    /// ⇒ ⇒ 真实 DOM 后端侧是 **O(N²)**（每个元素都 drain 整个监听器表）。
    ///
    /// 现在：**逐个事件类型**比较 ⇒ 只换**变了的那一个** ⇒
    /// ⇒ 其它事件类型的监听器**原地不动**（既有契约：同元素多 kind 互不误伤）
    /// ⇒ ⇒ 真实 DOM 后端侧是 **O(变化的那个 kind)**。
    ///
    /// ⚠️ **该 kind 从有变无**（新虚拟树里这个事件类型没了）⇒
    /// ⇒ 必须把旧的那个摘掉 ⇒ ⇒ 走 [`Dom::remove_listener`]（**按 kind**）。
    fn patch_handlers(
        &mut self,
        dom: &D::Node,
        old: &ElementVNode,
        new: &ElementVNode,
    ) -> Result<(), DomError> {
        if old.handlers == new.handlers {
            return Ok(());
        }
        // ① 逐 kind 比较：变了 ⇒ 先摘旧的那个（**只摘这个 kind**），再挂新的。
        for (kind, handler) in &new.handlers {
            let unchanged = old
                .handlers
                .iter()
                .find(|(old_kind, _)| old_kind == kind)
                .is_some_and(|(_, old_handler)| old_handler == handler);
            if unchanged {
                continue;
            }
            self.dom.remove_listener(dom, *kind)?;
            self.dom.add_listener(dom, *kind, handler.clone())?;
        }
        // ② 新虚拟树里**没有**的事件类型 ⇒ 把旧的那个摘掉。
        for (kind, _) in &old.handlers {
            if new.handlers.iter().any(|(new_kind, _)| new_kind == kind) {
                continue;
            }
            self.dom.remove_listener(dom, *kind)?;
        }
        Ok(())
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

/// 一个**块**的**首**个 DOM 节点（空分组块 ⇒ `None`）。
///
/// ## 为什么需要「首」而不是「尾」
///
/// 块 i 的后继是块 i+1 ⇒ ⇒ 块 i 的内容必须排在块 i+1 的**第一个** DOM 节点
/// **之前** ⇒ ⇒ 块锚点 = 后继块的**首**节点。
fn first_dom_node<D: Dom>(node: &MountedNode<D>) -> Option<&D::Node> {
    match node {
        MountedNode::Element(element) => Some(&element.dom),
        MountedNode::Text(text) => Some(&text.dom),
        MountedNode::Fragment(fragment) => fragment.children.first().and_then(first_dom_node),
    }
}

/// 一个**块**的**全部** DOM 节点（按文档顺序；分组块摊平后的那一段）。
///
/// 用于「整块移动」：分组块没有**单个** DOM 节点 ⇒ ⇒ 逐个插到锚点之前。
fn dom_nodes_of<D: Dom>(node: &MountedNode<D>) -> Vec<D::Node> {
    match node {
        MountedNode::Element(element) => vec![element.dom.clone()],
        MountedNode::Text(text) => vec![text.dom.clone()],
        MountedNode::Fragment(fragment) => {
            fragment.children.iter().flat_map(dom_nodes_of).collect()
        }
    }
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

    // ── 块锚点：条件块的出现 / 消失**不许**让兄弟错位（2026-10-25 修） ──────

    /// ⭐ **条件块从「空」变「非空」⇒ 内容留在原位**（2026-10-25 修）。
    ///
    /// ## 缺陷现象（实测）
    ///
    /// `div > [Fragment([Fragment([])]), footer]`，条件由假变真 ⇒
    /// ⇒ 修前 `before: [footer]` ⇒ `after: [footer, span("条件")]` ⇒
    /// ⇒ **条件卡出现在容器底部**（而不是它自己的位置）。
    ///
    /// ## 根因
    ///
    /// 修前「新增的子项」一律走 `mount_node(child, parent, None)` ⇒
    /// ⇒ **锚点 = None ⇒ 追加到父节点末尾** ⇒ 位置丢失。
    ///
    /// ## 判据
    ///
    /// 条件块的内容必须落在 **footer 之前**（块锚点 = 该块后继块的首节点）。
    #[test]
    fn a_conditional_block_that_appears_lands_in_its_own_slot() {
        let (mut renderer, _root) = renderer_with_dom();
        let before = VNode::element("div")
            .child(VNode::fragment(vec![VNode::fragment(Vec::new())]))
            .child(VNode::element("footer").build())
            .build();
        renderer.mount(before).expect("挂载必须成功");
        let div = child_order(&renderer)[0];
        let children = renderer.dom().children(&div).expect("读子节点必须成功");
        assert_eq!(children.len(), 1, "条件块为空 ⇒ 容器里只有 footer");
        let footer = children[0];

        // 条件由假变真。
        let after = VNode::element("div")
            .child(VNode::fragment(vec![VNode::fragment(vec![
                VNode::element("span").child(VNode::text("条件")).build(),
            ])]))
            .child(VNode::element("footer").build())
            .build();
        renderer.patch(after).expect("打补丁必须成功");

        let children = renderer.dom().children(&div).expect("读子节点必须成功");
        assert_eq!(children.len(), 2, "条件块出现了 ⇒ 容器里两个节点");
        assert_eq!(
            renderer.dom().tag(children[0]),
            Some(String::from("span")),
            "条件块必须落在 footer **之前**（位置不许丢）"
        );
        assert_eq!(children[1], footer, "footer 必须还在原位");
        assert_eq!(renderer.dom().text_content(children[0]), "条件");
    }

    /// ⭐ **块锚点：条件块在兄弟**之间**出现时，位置也不丢**。
    ///
    /// 判据：条件块的内容必须落在它的**后继兄弟**之前（而不是父节点末尾）。
    #[test]
    fn a_conditional_block_between_two_siblings_stays_between_them() {
        let (mut renderer, _root) = renderer_with_dom();
        let before = VNode::element("div")
            .child(VNode::element("header").build())
            .child(VNode::fragment(vec![VNode::fragment(Vec::new())]))
            .child(VNode::element("footer").build())
            .build();
        renderer.mount(before).expect("挂载必须成功");
        let div = child_order(&renderer)[0];
        let children = renderer.dom().children(&div).expect("读子节点必须成功");
        assert_eq!(children.len(), 2);
        let header = children[0];
        let footer = children[1];

        let after = VNode::element("div")
            .child(VNode::element("header").build())
            .child(VNode::fragment(vec![VNode::fragment(vec![
                VNode::element("span").attr("id", "cond").build(),
            ])]))
            .child(VNode::element("footer").build())
            .build();
        renderer.patch(after).expect("打补丁必须成功");

        let children = renderer.dom().children(&div).expect("读子节点必须成功");
        assert_eq!(children.len(), 3);
        assert_eq!(children[0], header, "header 不动");
        assert_eq!(children[2], footer, "footer 不动");
        assert_eq!(
            renderer.dom().attribute(children[1], "id"),
            Some(String::from("cond")),
            "条件块必须落在 header 与 footer **之间**"
        );
    }

    /// ⭐ **嵌套条件块不许顶替兄弟的身份**（2026-10-25 修）。
    ///
    /// ## 缺陷现象（实测）
    ///
    /// `div > Fragment[span(x), Fragment([]), span(z)]`，条件由假变真 ⇒
    /// ⇒ 修前 `before: [id2="x", id4="z"]` ⇒ `after: [id2="x", id4="y", id6="z"]`
    /// ⇒ ⇒ **id4 那颗节点从「z」变成了「y」**（身份转移），「z」被重建为 id6。
    ///
    /// ## 根因
    ///
    /// 修前摊平**递归**摊平所有嵌套 Fragment ⇒ 嵌套块的身份被融进兄弟序列。
    ///
    /// ## 判据
    ///
    /// 「z」那颗节点**必须还是同一颗**（身份不变 ⇒ 输入值 / 焦点 / 过渡不错位）。
    #[test]
    fn a_nested_conditional_block_does_not_take_over_its_sibling_identity() {
        let (mut renderer, _root) = renderer_with_dom();
        let before = VNode::element("div")
            .child(VNode::fragment(vec![
                VNode::element("span").attr("id", "x").build(),
                VNode::fragment(Vec::new()),
                VNode::element("span").attr("id", "z").build(),
            ]))
            .build();
        renderer.mount(before).expect("挂载必须成功");
        let div = child_order(&renderer)[0];
        let children = renderer.dom().children(&div).expect("读子节点必须成功");
        assert_eq!(children.len(), 2, "条件块为空 ⇒ 两个节点");
        let x = children[0];
        let z = children[1];
        assert_eq!(renderer.dom().attribute(x, "id"), Some(String::from("x")));
        assert_eq!(renderer.dom().attribute(z, "id"), Some(String::from("z")));

        // 条件由假变真。
        let after = VNode::element("div")
            .child(VNode::fragment(vec![
                VNode::element("span").attr("id", "x").build(),
                VNode::fragment(vec![VNode::element("span").attr("id", "y").build()]),
                VNode::element("span").attr("id", "z").build(),
            ]))
            .build();
        renderer.patch(after).expect("打补丁必须成功");

        let children = renderer.dom().children(&div).expect("读子节点必须成功");
        assert_eq!(children.len(), 3);
        assert_eq!(children[0], x, "x 必须还是同一颗节点");
        assert_eq!(
            children[2], z,
            "z 必须还是同一颗节点（**不许**被条件块顶替 ⇒ 身份转移会让输入值 / 焦点跟错逻辑项）"
        );
        assert_eq!(
            renderer.dom().attribute(children[1], "id"),
            Some(String::from("y"))
        );
    }

    /// **嵌套条件块变空** ⇒ 它的内容被摘掉，兄弟身份**不变**。
    #[test]
    fn a_nested_conditional_block_that_empties_leaves_its_siblings_alone() {
        let (mut renderer, _root) = renderer_with_dom();
        let before = VNode::element("div")
            .child(VNode::fragment(vec![
                VNode::element("span").attr("id", "x").build(),
                VNode::fragment(vec![VNode::element("span").attr("id", "y").build()]),
                VNode::element("span").attr("id", "z").build(),
            ]))
            .build();
        renderer.mount(before).expect("挂载必须成功");
        let div = child_order(&renderer)[0];
        let children = renderer.dom().children(&div).expect("读子节点必须成功");
        let x = children[0];
        let z = children[2];

        let after = VNode::element("div")
            .child(VNode::fragment(vec![
                VNode::element("span").attr("id", "x").build(),
                VNode::fragment(Vec::new()),
                VNode::element("span").attr("id", "z").build(),
            ]))
            .build();
        renderer.patch(after).expect("打补丁必须成功");

        let children = renderer.dom().children(&div).expect("读子节点必须成功");
        assert_eq!(children.len(), 2, "条件块空了 ⇒ 它的内容被摘掉");
        assert_eq!(children[0], x, "x 身份不变");
        assert_eq!(children[1], z, "z 身份不变");
    }

    // ── 失败之后：**显式劈叉**，不许拿过期旧树继续 diff（2026-10-25 修） ─────

    /// 一个「写 property 时故意失败」的内存后端（制造**半途失败**的 patch）。
    #[derive(Debug)]
    struct FragileDom {
        inner: InMemoryDom,
        /// 写 property 时是否失败。
        fail_property: Cell<bool>,
        /// 移除 property 时是否**不支持**（制造「后端做不了」的降级）。
        fail_remove_property: Cell<bool>,
    }

    impl FragileDom {
        fn new() -> Self {
            Self {
                inner: InMemoryDom::new(),
                fail_property: Cell::new(false),
                fail_remove_property: Cell::new(false),
            }
        }

        /// 让「写 property」开始（或停止）失败。
        fn set_fail_property(&self, flag: bool) {
            self.fail_property.set(flag);
        }

        /// 让「移除 property」开始（或停止）报「后端不支持」。
        fn set_fail_remove_property(&self, flag: bool) {
            self.fail_remove_property.set(flag);
        }
    }

    impl Dom for FragileDom {
        type Node = InMemoryNodeId;

        fn create_element(&self, tag: &str) -> Result<Self::Node, DomError> {
            self.inner.create_element(tag)
        }

        fn create_text(&self, data: &str) -> Result<Self::Node, DomError> {
            self.inner.create_text(data)
        }

        fn set_attribute(
            &self,
            node: &Self::Node,
            name: &str,
            value: &str,
        ) -> Result<(), DomError> {
            self.inner.set_attribute(node, name, value)
        }

        fn remove_attribute(&self, node: &Self::Node, name: &str) -> Result<(), DomError> {
            self.inner.remove_attribute(node, name)
        }

        fn set_class_tokens(
            &self,
            node: &Self::Node,
            added: &[String],
            removed: &[String],
        ) -> Result<(), DomError> {
            self.inner.set_class_tokens(node, added, removed)
        }

        fn set_property(&self, node: &Self::Node, name: &str, value: &str) -> Result<(), DomError> {
            if self.fail_property.get() {
                return Err(DomError::Unsupported(String::from(
                    "测试替身：写 property 失败",
                )));
            }
            self.inner.set_property(node, name, value)
        }

        fn set_property_bool(
            &self,
            node: &Self::Node,
            name: &str,
            value: bool,
        ) -> Result<(), DomError> {
            if self.fail_property.get() {
                return Err(DomError::Unsupported(String::from(
                    "测试替身：写 property 失败",
                )));
            }
            self.inner.set_property_bool(node, name, value)
        }

        fn get_property(&self, node: &Self::Node, name: &str) -> Result<String, DomError> {
            self.inner.get_property(node, name)
        }

        fn remove_property(&self, node: &Self::Node, name: &str) -> Result<(), DomError> {
            if self.fail_remove_property.get() {
                return Err(DomError::Unsupported(String::from(
                    "测试替身：后端不支持移除 property",
                )));
            }
            self.inner.remove_property(node, name)
        }

        fn set_text(&self, node: &Self::Node, data: &str) -> Result<(), DomError> {
            self.inner.set_text(node, data)
        }

        fn append_child(&self, parent: &Self::Node, child: &Self::Node) -> Result<(), DomError> {
            self.inner.append_child(parent, child)
        }

        fn insert_before(
            &self,
            parent: &Self::Node,
            child: &Self::Node,
            anchor: Option<&Self::Node>,
        ) -> Result<(), DomError> {
            self.inner.insert_before(parent, child, anchor)
        }

        fn remove_child(&self, parent: &Self::Node, child: &Self::Node) -> Result<(), DomError> {
            self.inner.remove_child(parent, child)
        }

        fn children(&self, parent: &Self::Node) -> Result<Vec<Self::Node>, DomError> {
            self.inner.children(parent)
        }

        fn add_listener(
            &self,
            node: &Self::Node,
            kind: crate::dom::EventKind,
            handler: crate::events::ElementHandler,
        ) -> Result<(), DomError> {
            self.inner.add_listener(node, kind, handler)
        }

        fn remove_listeners(&self, node: &Self::Node) -> Result<(), DomError> {
            self.inner.remove_listeners(node)
        }

        fn remove_listener(
            &self,
            node: &Self::Node,
            kind: crate::dom::EventKind,
        ) -> Result<(), DomError> {
            self.inner.remove_listener(node, kind)
        }

        fn target_path(&self, node: &Self::Node) -> Result<Vec<usize>, DomError> {
            self.inner.target_path(node)
        }
    }

    /// ⭐ **一次失败的 patch 之后，渲染器必须**显式劈叉**（不许悄悄吞掉）。
    ///
    /// ## 缺陷现象（实测）
    ///
    /// 修前：出错 ⇒ `self.tree = Some(old)`（**过期**的旧树）⇒ DOM 已改了一半
    /// ⇒ ⇒ 之后每次 patch 都拿过期旧树做基准 ⇒ ⇒ 可能每帧都失败 ⇒
    /// ⇒ **界面永久停在旧状态，且无任何日志**。
    ///
    /// ## 判据
    ///
    /// ① 失败**照样返回 `Err`**（不吞）；
    /// ② 失败之后 [`Renderer::is_desynced`] 为 `true`（宿主能查、能 log）；
    /// ③ [`Renderer::last_error`] 有内容（宿主能决定重建）；
    /// ④ **下一次** patch 走**整树重建** ⇒ DOM 与新树一致 ⇒ 界面恢复。
    #[test]
    fn a_failed_patch_marks_the_renderer_desynced_and_the_next_patch_rebuilds() {
        let dom = FragileDom::new();
        let root = dom.create_element("main").expect("建节点必须成功");
        let mut renderer = Renderer::new(dom, root);
        renderer
            .mount(VNode::element("div").prop("value", "旧").build())
            .expect("挂载必须成功");
        // 让下一次 patch 在写 property 时失败（此时属性已经改了一半）。
        renderer.dom().set_fail_property(true);

        let err = renderer
            .patch(VNode::element("div").prop("value", "新").build())
            .expect_err("写 property 失败必须显式报错");
        assert!(matches!(err, DomError::Unsupported(_)), "实测：{err:?}");
        assert!(
            renderer.is_desynced(),
            "失败之后必须标记劈叉（宿主要能 log、要能决定重建）"
        );
        assert!(
            renderer.last_error().is_some(),
            "失败原因必须能被宿主读到（不许悄悄吞掉）"
        );

        // 后端恢复正常 ⇒ 下一次 patch 必须**整树重建**（不许拿过期旧树差分）。
        renderer.dom().set_fail_property(false);
        renderer
            .patch(VNode::element("section").child(VNode::text("重建")).build())
            .expect("重建后的 patch 必须成功（界面不许永久冻结）");
        assert!(!renderer.is_desynced(), "重建成功 ⇒ 不再劈叉");
        let children = renderer.dom().children(&root).expect("读子节点必须成功");
        assert_eq!(
            children.len(),
            1,
            "重建后容器里**只**剩新树（不许有孤儿节点）"
        );
        assert_eq!(
            renderer.dom().inner.tag(children[0]),
            Some(String::from("section"))
        );
    }

    /// **失败不许被吞掉**：错误必须**原样**回到调用方。
    #[test]
    fn a_failed_patch_returns_the_error_to_the_caller() {
        let dom = FragileDom::new();
        let root = dom.create_element("main").expect("建节点必须成功");
        let mut renderer = Renderer::new(dom, root);
        renderer
            .mount(VNode::element("div").prop("value", "旧").build())
            .expect("挂载必须成功");
        renderer.dom().set_fail_property(true);

        let err = renderer
            .patch(VNode::element("div").prop("value", "新").build())
            .expect_err("失败必须回到调用方");
        assert!(matches!(err, DomError::Unsupported(_)), "实测：{err:?}");
        // 错误**可取走**（宿主写日志用）。
        assert!(renderer.take_last_error().is_some());
    }

    /// **差分不可判定**（重复 key）⇒ DOM 一个字节都没动 ⇒ **不算**劈叉。
    ///
    /// （判据落在「错误发生在触碰 DOM 之前」⇒ 内存树仍可信。）
    #[test]
    fn an_undecidable_diff_does_not_desync_the_renderer() {
        let (mut renderer, _root) = renderer_with_dom();
        renderer
            .mount(VNode::element("ul").child(keyed_item("a", "1")).build())
            .expect("挂载必须成功");
        let err = renderer
            .patch(
                VNode::element("ul")
                    .child(keyed_item("x", "1"))
                    .child(keyed_item("x", "2"))
                    .build(),
            )
            .expect_err("重复 key 必须显式报错");
        assert!(matches!(err, DomError::Undecidable(_)), "实测：{err:?}");
        assert!(
            !renderer.is_desynced(),
            "差分不可判定发生在触碰 DOM 之前 ⇒ 内存树仍可信 ⇒ 不算劈叉"
        );
    }

    /// ⭐⭐⭐ **回归（台账 B1，2026-10-27 修）**：`Undecidable` 必须发生在
    /// **写任何 DOM 属性之前**。
    ///
    /// ## 为什么需要这条（上一条测试的「侥幸」）
    ///
    /// 上一条测试的夹具里**父元素一个属性都没有** ⇒ ⇒ `patch_attributes`
    /// 一个字节都没写 ⇒ ⇒ 「DOM 未被触碰」只是**夹具侥幸**，不是构造保证。
    /// 修前的真实顺序是：先写属性（`:515`）→ 再在 keyed 差分里判定可判定性
    /// （`:681-683`）⇒ ⇒ 属性已落 DOM、内存树仍是旧值，且 `Undecidable`
    /// 被排除在「标劈叉」之外 ⇒ ⇒ **静默分叉且不自愈**。
    ///
    /// ## 判据
    ///
    /// 让父元素属性**真的会变**（`class="a"` → `"b"`）同时子节点重复 key：
    /// ① 报 `Undecidable`；② **DOM 里的 `class` 仍是 `"a"`**
    /// ⇒ ⇒ 「先定状态、再动作」由**构造**保证（不是靠夹具）。
    #[test]
    fn an_undecidable_diff_happens_before_any_attribute_is_written() {
        let (mut renderer, root) = renderer_with_dom();
        renderer
            .mount(
                VNode::element("ul")
                    .class("a")
                    .child(keyed_item("a", "1"))
                    .build(),
            )
            .expect("挂载必须成功");
        let ul = renderer.dom().children(&root).expect("读子节点必须成功")[0];
        assert_eq!(
            renderer.dom().attribute(ul, "class"),
            Some(String::from("a")),
            "前提：挂载后 class 应为 a"
        );

        let err = renderer
            .patch(
                VNode::element("ul")
                    .class("b")
                    .child(keyed_item("x", "1"))
                    .child(keyed_item("x", "2"))
                    .build(),
            )
            .expect_err("重复 key 必须显式报错");
        assert!(matches!(err, DomError::Undecidable(_)), "实测：{err:?}");
        assert_eq!(
            renderer.dom().attribute(ul, "class"),
            Some(String::from("a")),
            "`Undecidable` 必须发生在**写属性之前** ⇒ class 仍应是 a（修前会是 b）"
        );
        assert!(!renderer.is_desynced(), "内存在树仍可信 ⇒ 不算劈叉");
    }

    // ── property 消失：**不许中止整帧**（2026-10-25 修） ─────────────────────

    /// ⭐ **property 从虚拟树上消失 ⇒ 真正移除，不中止整帧**（2026-10-25 修）。
    ///
    /// ## 缺陷现象（实测）
    ///
    /// 同一个元素这帧有 `prop("value", …)`、下帧没有 ⇒
    /// ⇒ 修前 `Err(Unsupported("移除 property `value` 未支持"))` ⇒ **整帧中止**。
    ///
    /// ## 判据
    ///
    /// ① patch **成功**；
    /// ② 移除之后读回来的值是**默认值**（空串）⇒ 与浏览器一致；
    /// ③ 操作序列里有 `RemoveProperty`（**真的**移除了，不是悄悄跳过）。
    #[test]
    fn a_property_that_disappears_is_removed_instead_of_aborting_the_frame() {
        let (mut renderer, _root) = renderer_with_dom();
        renderer
            .mount(VNode::element("input").prop("value", "旧").build())
            .expect("挂载必须成功");
        let node = child_order(&renderer)[0];
        renderer.dom().clear_ops();

        renderer
            .patch(VNode::element("input").build())
            .expect("property 消失**不许**中止整帧");

        assert_eq!(
            renderer
                .dom()
                .get_property(&node, "value")
                .expect("读 property 必须成功"),
            String::new(),
            "移除之后回到默认值（与浏览器一致）"
        );
        assert!(
            renderer
                .dom()
                .ops()
                .iter()
                .any(|recorded| matches!(recorded.op, DomOp::RemoveProperty { .. })),
            "必须真的移除（不是悄悄跳过）"
        );
    }

    /// **后端做不了移除** ⇒ **降级**（写回默认值 + 记警告），**不许**中止整帧。
    #[test]
    fn a_backend_without_property_removal_degrades_instead_of_aborting() {
        let dom = FragileDom::new();
        let root = dom.create_element("main").expect("建节点必须成功");
        let mut renderer = Renderer::new(dom, root);
        renderer
            .mount(VNode::element("input").prop("value", "旧").build())
            .expect("挂载必须成功");
        let node = renderer.dom().children(&root).expect("读子节点必须成功")[0];
        renderer.dom().set_fail_remove_property(true);

        renderer
            .patch(VNode::element("input").build())
            .expect("后端做不了移除 ⇒ 降级，**不许**中止整帧");
        assert_eq!(
            renderer
                .dom()
                .get_property(&node, "value")
                .expect("读 property 必须成功"),
            String::new(),
            "降级 ⇒ 写回默认值"
        );
        let warnings = renderer.warnings().to_vec();
        assert!(
            !warnings.is_empty(),
            "降级必须**看得见**（记警告，宿主才道）：{warnings:?}"
        );
    }

    // ── 事件处理器：**按 kind 增量替换**（2026-10-25 修） ───────────────────

    /// ⭐ **换掉一种事件处理器，不许牵连另一种**（2026-10-25 修）。
    ///
    /// ## 缺陷现象（实测）
    ///
    /// 修前：`patch_handlers` 一律 `remove_listeners`（摘**全部**）⇒
    /// ⇒ 再全量重挂 ⇒ ⇒ 每次重渲染、每个元素都**全量拆装**（真实 DOM 侧 O(N²)）。
    ///
    /// ## 判据
    ///
    /// ① 只换 `click` ⇒ 操作序列里**只有** `click` 的摘挂；
    /// ② **不许**出现 `RemoveListeners`（全量摘挂的痕迹）；
    /// ③ `keydown` 的处理器**仍然**能触发（没被误伤）。
    #[test]
    fn replacing_one_event_kind_leaves_the_other_kinds_alone() {
        let (mut renderer, _root) = renderer_with_dom();
        let clicks = Rc::new(Cell::new(0u32));
        let keys = Rc::new(Cell::new(0u32));
        let click_counter = clicks.clone();
        let key_counter = keys.clone();
        // `keydown` 的处理器**两次都是同一个闭包实例**（处理器按指针身份比较）
        // ⇒ ⇒ 它**没变** ⇒ ⇒ 不该被重挂。
        let key_handler = crate::events::ElementHandler::new(move |_event| {
            key_counter.set(key_counter.get() + 1);
        });
        renderer
            .mount(
                VNode::element("div")
                    .handler(
                        crate::dom::EventKind::Click,
                        crate::events::ElementHandler::new(move |_event| {
                            click_counter.set(click_counter.get() + 1);
                        }),
                    )
                    .handler(crate::dom::EventKind::KeyDown, key_handler.clone())
                    .build(),
            )
            .expect("挂载必须成功");
        let node = child_order(&renderer)[0];
        renderer.dom().clear_ops();

        // 只换 `click` 的处理器。
        let second = Rc::new(Cell::new(0u32));
        let second_counter = second.clone();
        renderer
            .patch(
                VNode::element("div")
                    .handler(
                        crate::dom::EventKind::Click,
                        crate::events::ElementHandler::new(move |_event| {
                            second_counter.set(second_counter.get() + 1);
                        }),
                    )
                    .handler(crate::dom::EventKind::KeyDown, key_handler.clone())
                    .build(),
            )
            .expect("打补丁必须成功");

        let ops = renderer.dom().ops();
        assert!(
            !ops.iter()
                .any(|recorded| matches!(recorded.op, DomOp::RemoveListeners)),
            "不许全量摘挂（那是 O(N²) 的根源）"
        );
        assert_eq!(
            ops.iter()
                .filter(|recorded| matches!(
                    recorded.op,
                    DomOp::RemoveListener {
                        kind: crate::dom::EventKind::Click
                    }
                ))
                .count(),
            1,
            "只摘 `click` 那个"
        );
        assert_eq!(
            ops.iter()
                .filter(|recorded| matches!(
                    recorded.op,
                    DomOp::RemoveListener {
                        kind: crate::dom::EventKind::KeyDown
                    }
                ))
                .count(),
            0,
            "`keydown` **不许**被牵连"
        );
        // `keydown` 处理器仍然活着（没被误伤）。
        assert_eq!(
            renderer
                .dom()
                .dispatch(node, crate::dom::EventKind::KeyDown),
            1
        );
    }

    /// **该 kind 从有变无** ⇒ 旧的那个必须被摘掉。
    #[test]
    fn a_handler_kind_that_disappears_is_unhooked() {
        let (mut renderer, _root) = renderer_with_dom();
        let counter = Rc::new(Cell::new(0u32));
        let handler_counter = counter.clone();
        renderer
            .mount(
                VNode::element("button")
                    .handler(
                        crate::dom::EventKind::Click,
                        crate::events::ElementHandler::new(move |_event| {
                            handler_counter.set(handler_counter.get() + 1);
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

        renderer
            .patch(VNode::element("button").build())
            .expect("打补丁必须成功");
        assert_eq!(
            renderer.dom().dispatch(node, crate::dom::EventKind::Click),
            0,
            "事件类型没了 ⇒ 旧处理器必须被摘掉"
        );
    }

    // ── 与浏览器**同语义**：class / property（2026-10-25 修） ──────────────

    /// ⭐ **class token 全删之后留 `class=""`**（与浏览器一致）。
    ///
    /// ## 缺陷现象（实测）
    ///
    /// 内存后端把 class 属性**整个删掉** ⇒ 浏览器侧 `classList` 删完是
    /// `class=""` ⇒ ⇒ 两侧属性对照**差一项** ⇒ native 绿 ≠ 浏览器对。
    #[test]
    fn removing_every_class_token_leaves_an_empty_class_attribute() {
        let (mut renderer, _root) = renderer_with_dom();
        renderer
            .mount(VNode::element("div").class("ap-a").build())
            .expect("挂载必须成功");
        let node = child_order(&renderer)[0];

        renderer
            .patch(VNode::element("div").build())
            .expect("打补丁必须成功");
        assert_eq!(
            renderer.dom().attribute(node, "class"),
            Some(String::new()),
            "与浏览器一致：留 `class=\"\"`（属性**仍在**）"
        );
    }

    /// ⭐ **用户敲进去的值不许被覆盖**（实时语义下的跳过条件）。
    ///
    /// ## 为什么这条重要
    ///
    /// 后端读 property 是**实时**语义（浏览器读的是真实的 `input.value`
    /// ⇒ 可能是**用户刚敲进去**的值）⇒ ⇒ 若把「实时值」当「上一帧的目标值」
    /// 来比 ⇒ ⇒ 用户刚敲的字符会被引擎**覆盖回去**（幽灵输入的另一半）。
    ///
    /// ## 判据
    ///
    /// 目标值**没变** ⇒ 引擎**不许**写 property（用户的输入必须活着）。
    #[test]
    fn a_user_typed_value_is_not_overwritten_when_the_target_did_not_change() {
        let (mut renderer, _root) = renderer_with_dom();
        renderer
            .mount(VNode::element("input").prop("value", "a").build())
            .expect("挂载必须成功");
        let node = child_order(&renderer)[0];
        // 模拟用户敲入（宿主直接写后端 ⇒ 引擎**没有**写它）。
        renderer
            .dom()
            .set_property(&node, "value", "ab")
            .expect("模拟用户输入必须成功");
        renderer.dom().clear_ops();

        renderer
            .patch(VNode::element("input").prop("value", "a").build())
            .expect("重渲染必须成功");

        assert!(
            !renderer
                .dom()
                .ops()
                .iter()
                .any(|recorded| matches!(recorded.op, DomOp::SetProperty { .. })),
            "目标值没变 ⇒ **不许**写 property（否则覆盖用户输入）"
        );
        assert_eq!(
            renderer
                .dom()
                .get_property(&node, "value")
                .expect("读 property 必须成功"),
            String::from("ab"),
            "用户敲进去的值必须活着"
        );
    }

    /// **目标值变了** ⇒ 引擎**必须**写（受控输入的语义：状态说了算）。
    #[test]
    fn a_changed_target_value_is_written_back() {
        let (mut renderer, _root) = renderer_with_dom();
        renderer
            .mount(VNode::element("input").prop("value", "a").build())
            .expect("挂载必须成功");
        let node = child_order(&renderer)[0];
        renderer
            .dom()
            .set_property(&node, "value", "ab")
            .expect("模拟用户输入必须成功");
        renderer.dom().clear_ops();

        renderer
            .patch(VNode::element("input").prop("value", "c").build())
            .expect("重渲染必须成功");
        assert!(
            renderer
                .dom()
                .ops()
                .iter()
                .any(|recorded| matches!(recorded.op, DomOp::SetProperty { .. })),
            "目标变了 ⇒ 必须写（受控输入语义）"
        );
        assert_eq!(
            renderer
                .dom()
                .get_property(&node, "value")
                .expect("读 property 必须成功"),
            String::from("c")
        );
    }
}
