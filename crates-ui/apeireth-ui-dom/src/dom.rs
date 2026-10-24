//! DOM 后端抽象：真实 DOM 与内存 DOM 共用同一套渲染逻辑。
//!
//! 设计文档 §5.1.2：**这是本设计能消灭源码镜像测试的结构性前提** —— 差分 /
//! 委托 / 绑定逻辑全部写成 `D: Dom` 泛型代码，因而 100% 可 native 单测。
//!
//! ## 诚实边界：失败必须显式，不许静默降级
//!
//! `Dom` 的**每一个**操作都返回 [`Result`]：
//! - 节点不存在 ⇒ [`DomError::NodeNotFound`]；
//! - 节点已被摘除 ⇒ [`DomError::NodeDetached`]（设计文档 §5.5.2 规则 T6：
//!   被移除的节点不参与本批次的属性写入 ⇒ **必须可判定**，不能静默吞掉）；
//! - 操作不适用 ⇒ [`DomError::Unsupported`]；
//! - 后端未接线 ⇒ [`DomError::NotImplemented`]。
//!
//! 先例：`ui-gpu` 的 `GpuContext::submit_frame -> Result<(), GpuError>`。
//!
//! ## 后端清单（P2 实况）
//!
//! | 后端 | 目标 | 状态 |
//! |---|---|---|
//! | [`InMemoryDom`] | native（永远可用） | ✅ 已实现，且**记录操作序列**（T1–T6 的机械判据） |
//! | `WebDom`（web-sys） | wasm32 only | ✅ 已实现；**未经任何运行时验证**（见 crate 文档头） |
//! | [`UnavailableDom`] | —— | 🔵 未接线桩：每个操作都报 [`DomError::NotImplemented`] |

use std::cell::RefCell;

/// DOM 事件类型（按仓库实测使用面裁剪，不照抄浏览器 API 全集）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventKind {
    /// 点击（冒泡，委托根监听）。
    Click,
    /// 键盘按下（冒泡）。
    KeyDown,
    /// 输入（冒泡；双向绑定的取值事件）。
    Input,
    /// 变更（冒泡）。
    Change,
    /// 失焦（冒泡）。
    FocusOut,
}

impl EventKind {
    /// 事件名（与仓内事件属性同名，降低迁移摩擦）。
    pub fn as_str(&self) -> &'static str {
        match self {
            EventKind::Click => "click",
            EventKind::KeyDown => "keydown",
            EventKind::Input => "input",
            EventKind::Change => "change",
            EventKind::FocusOut => "focusout",
        }
    }
}

/// DOM 后端错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomError {
    /// 节点不存在。
    NodeNotFound,
    /// 节点已从树上摘除（禁止再写入，见设计 §5.5.2 规则 T6）。
    NodeDetached,
    /// **未接线**：后端能力尚未接入。
    NotImplemented,
    /// 不支持的操作。
    Unsupported(String),
    /// **不可判定**：差分无法得出唯一结果（例：同一列表出现重复 key ⇒ 身份不唯一）。
    ///
    /// 与 [`DomError::Unsupported`] 分开：前者是「这个后端做不了」，
    /// 后者是「这个输入没有唯一答案」——两者的处置方式不同。
    Undecidable(String),
}

impl std::fmt::Display for DomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DomError::NodeNotFound => write!(f, "NodeNotFound: 节点不存在"),
            DomError::NodeDetached => write!(f, "NodeDetached: 节点已摘除"),
            DomError::NotImplemented => {
                write!(f, "NotImplemented: 该 DOM 后端能力未接线")
            }
            DomError::Unsupported(detail) => write!(f, "Unsupported: {detail}"),
            DomError::Undecidable(detail) => write!(f, "Undecidable: {detail}"),
        }
    }
}

impl std::error::Error for DomError {}

/// DOM 后端抽象：渲染层唯一允许触碰「真实节点」的地方。
///
/// ## 节点句柄的约束
///
/// `Node` 要求 `'static`：节点句柄是**拥有所有权**的句柄（内存后端 = 编号，
/// 真实后端 = JS 句柄），不借用外部数据。`'static` 也是 `bind:this` 槽位
/// 能把句柄**类型擦除**后仍可还原的前提（见 [`crate::binding::ThisRef`]）。
///
/// 后端本身同样要求 `'static`（`Dom: 'static`）：后端由渲染器**拥有**，
/// 且 `bind:this` 的擦除视图要求后端类型可被放进 `'static` 的 trait object。
pub trait Dom: 'static {
    /// 后端节点类型（真实后端是 web-sys 的节点句柄，内存后端是编号）。
    type Node: Clone + PartialEq + 'static;

    /// 建一个元素节点。
    fn create_element(&self, tag: &str) -> Result<Self::Node, DomError>;

    /// 建一个**命名空间**内的元素节点（SVG 等）。
    ///
    /// 默认实现 = [`Dom::create_element`]：内存后端不渲染，命名空间对它没有
    /// 意义（它只记录标签名）⇒ 默认实现不是「假装支持」，而是「本后端确实
    /// 不需要」。
    ///
    /// 真实 DOM 后端**必须**覆写：HTML 命名空间里的 `<svg>` **不是** SVG 元素
    /// （浏览器不会把它当矢量图渲染，`viewBox` 这类驼峰属性名还会被小写化）
    /// ⇒ SVG 元素必须走 `createElementNS`。
    fn create_element_ns(&self, namespace: &str, tag: &str) -> Result<Self::Node, DomError> {
        let _ = namespace;
        self.create_element(tag)
    }

    /// 建一个文本节点。
    fn create_text(&self, data: &str) -> Result<Self::Node, DomError>;

    /// 写属性。
    fn set_attribute(&self, node: &Self::Node, name: &str, value: &str) -> Result<(), DomError>;

    /// 删属性。
    fn remove_attribute(&self, node: &Self::Node, name: &str) -> Result<(), DomError>;

    /// 增删 class token（**token 级**差分；设计文档 §5.2.1）。
    ///
    /// 为什么不是整体覆写 `class` 字符串：整体覆写会连带抹掉别的 class，
    /// 且在有 CSS `transition` 时会产生额外的中间态。
    ///
    /// 语义：先删 `removed` 里的 token，再加 `added` 里的 token（去重保序）。
    fn set_class_tokens(
        &self,
        node: &Self::Node,
        added: &[String],
        removed: &[String],
    ) -> Result<(), DomError>;

    /// 写 property（`value` / `checked` 等）。
    ///
    /// **不是属性**：`bind:value` 走 property（设计文档 §5.4.1）——写 attribute
    /// 对 `input.value` 无效，且会破坏浏览器的受控输入语义。
    fn set_property(&self, node: &Self::Node, name: &str, value: &str) -> Result<(), DomError>;

    /// 读 property。
    fn get_property(&self, node: &Self::Node, name: &str) -> Result<String, DomError>;

    /// 写文本内容（就地改，不重建文本节点，见设计 §5.2.4）。
    fn set_text(&self, node: &Self::Node, data: &str) -> Result<(), DomError>;

    /// 追加子节点。
    fn append_child(&self, parent: &Self::Node, child: &Self::Node) -> Result<(), DomError>;

    /// 在锚点之前插入子节点（`anchor = None` ⇒ 追加到末尾）。
    ///
    /// keyed 列表的**移动**必须走本操作（设计文档 §5.2.3）：移动 = 换位置，
    /// **不是**「先摘除再追加」——后者会让节点短暂离开文档树，CSS 过渡 /
    /// 动画状态会被打断。
    fn insert_before(
        &self,
        parent: &Self::Node,
        child: &Self::Node,
        anchor: Option<&Self::Node>,
    ) -> Result<(), DomError>;

    /// 移除子节点（节点被摘除后再写入 ⇒ [`DomError::NodeDetached`]）。
    fn remove_child(&self, parent: &Self::Node, child: &Self::Node) -> Result<(), DomError>;

    /// 读子节点列表（按文档顺序）。
    fn children(&self, parent: &Self::Node) -> Result<Vec<Self::Node>, DomError>;

    /// 挂事件处理器（**元素直挂**：浏览器负责冒泡；`currentTarget` 语义 =
    /// 「处理器注册在哪个元素上」，与设计文档 §5.3.3 的确定解一致）。
    ///
    /// 同一节点同一事件类型挂两次 ⇒ 后者覆盖前者（与虚拟树
    /// [`crate::vnode::ElementVNode::handler`] 的覆盖语义一致）。
    fn add_listener(
        &self,
        node: &Self::Node,
        kind: EventKind,
        handler: crate::events::ElementHandler,
    ) -> Result<(), DomError>;

    /// 摘掉某节点上的**全部**事件处理器（卸载时必做；差分时重挂前也做）。
    fn remove_listeners(&self, node: &Self::Node) -> Result<(), DomError>;
}

/// 内存 DOM 的节点句柄（编号）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InMemoryNodeId(pub usize);

/// 内存 DOM 内部节点。
#[derive(Debug, Clone)]
enum InMemoryNodeKind {
    Element(String),
    Text(String),
}

/// 内存 DOM 内部节点记录。
#[derive(Debug, Clone)]
struct InMemoryNode {
    kind: InMemoryNodeKind,
    attrs: Vec<(String, String)>,
    props: Vec<(String, String)>,
    children: Vec<InMemoryNodeId>,
    /// 当前父节点（`None` = 尚未挂到任何树上）。
    parent: Option<InMemoryNodeId>,
    /// 事件处理器（挂在**本元素**上；冒泡由 [`InMemoryDom::dispatch`] 模拟）。
    listeners: Vec<(EventKind, crate::events::ElementHandler)>,
    /// 是否已被从树上摘除（摘除后禁止再写入 —— 设计文档 §5.5.2 规则 T6）。
    detached: bool,
}

/// 内存 DOM 记录的一次操作。
///
/// 这是**机械判定**设计文档 §5.5.2 六条保护规则（T1–T6）的依据：规则问的
/// 是「引擎按什么顺序写了什么」，操作序列正是那个问题的答案。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomOp {
    /// 建元素节点。
    CreateElement {
        /// 标签名。
        tag: String,
    },
    /// 建文本节点。
    CreateText {
        /// 文本内容。
        data: String,
    },
    /// 写属性。
    SetAttribute {
        /// 属性名。
        name: String,
        /// 属性值。
        value: String,
    },
    /// 删属性。
    RemoveAttribute {
        /// 属性名。
        name: String,
    },
    /// 写 property。
    SetProperty {
        /// property 名。
        name: String,
        /// property 值。
        value: String,
    },
    /// 读 property（不改树；记录它是为了让「读」在序列里可见）。
    GetProperty {
        /// property 名。
        name: String,
    },
    /// 写文本内容。
    SetText {
        /// 新文本。
        data: String,
    },
    /// 增删 class token（token 级差分）。
    SetClassTokens {
        /// 要加的 token。
        added: Vec<String>,
        /// 要删的 token。
        removed: Vec<String>,
    },
    /// 追加子节点。
    AppendChild {
        /// 父节点。
        parent: InMemoryNodeId,
    },
    /// 在锚点前插入子节点（`anchor = None` ⇒ 追加到末尾）。
    InsertBefore {
        /// 父节点。
        parent: InMemoryNodeId,
        /// 锚点节点（`None` = 末尾）。
        anchor: Option<InMemoryNodeId>,
    },
    /// 移除子节点。
    RemoveChild {
        /// 父节点。
        parent: InMemoryNodeId,
    },
    /// 挂事件处理器。
    AddListener {
        /// 事件类型。
        kind: EventKind,
    },
    /// 摘掉某节点上的全部事件处理器。
    RemoveListeners,
}

/// 一次操作 + 它作用的节点。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedOp {
    /// 操作内容。
    pub op: DomOp,
    /// 作用的节点。
    ///
    /// 树操作（`AppendChild` / `InsertBefore` / `RemoveChild`）记的是**子节点**
    /// （父节点记在 `DomOp` 里）。
    pub node: InMemoryNodeId,
}

/// 内存 DOM 后端：native 单测用（永远可用，不需浏览器、不需 wasm target）。
///
/// ## 操作序列记录
///
/// 后端**始终**记录自己执行过的操作（[`InMemoryDom::ops`]）⇒ T1–T6 可以被
/// 机械判定（而不必读源码文本 —— 设计文档 §10.2 明令禁止源码镜像断言）。
#[derive(Debug, Default)]
pub struct InMemoryDom {
    nodes: RefCell<Vec<InMemoryNode>>,
    ops: RefCell<Vec<RecordedOp>>,
}

impl InMemoryDom {
    /// 建一棵空的内存 DOM。
    pub fn new() -> Self {
        Self::default()
    }

    /// 读节点标签（元素节点才有）。
    pub fn tag(&self, node: InMemoryNodeId) -> Option<String> {
        match &self.nodes.borrow().get(node.0)?.kind {
            InMemoryNodeKind::Element(tag) => Some(tag.clone()),
            InMemoryNodeKind::Text(_) => None,
        }
    }

    /// 读属性值。
    pub fn attribute(&self, node: InMemoryNodeId, name: &str) -> Option<String> {
        let nodes = self.nodes.borrow();
        let entry = nodes.get(node.0)?;
        entry
            .attrs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
    }

    /// 读 property 值。
    pub fn property(&self, node: InMemoryNodeId, name: &str) -> Option<String> {
        let nodes = self.nodes.borrow();
        let entry = nodes.get(node.0)?;
        entry
            .props
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
    }

    /// 读文本内容（文本节点自身或子树拼接）。
    pub fn text_content(&self, node: InMemoryNodeId) -> String {
        let nodes = self.nodes.borrow();
        let Some(entry) = nodes.get(node.0) else {
            return String::new();
        };
        match &entry.kind {
            InMemoryNodeKind::Text(data) => data.clone(),
            InMemoryNodeKind::Element(_) => entry
                .children
                .iter()
                .map(|child| self.text_content(*child))
                .collect::<Vec<_>>()
                .join(""),
        }
    }

    /// 节点总数（用于断言「没有多余节点产生」）。
    pub fn node_count(&self) -> usize {
        self.nodes.borrow().len()
    }

    /// 派发事件：自目标节点起，沿祖先链**自下而上**调用处理器。
    ///
    /// 冒泡语义与真实浏览器一致（设计文档 §5.3.3）：目标先执行，祖先后执行；
    /// 任一处理器调用 `stop_propagation()` ⇒ 停止向上传播。
    ///
    /// 返回实际调用次数。
    pub fn dispatch(&self, node: InMemoryNodeId, kind: EventKind) -> usize {
        // 目标路径（从根到目标的子节点下标序列）。
        let mut indices: Vec<usize> = Vec::new();
        let mut current = node;
        while let Some(parent) = self.parent(current) {
            let siblings = self
                .nodes
                .borrow()
                .get(parent.0)
                .map(|entry| entry.children.clone())
                .unwrap_or_default();
            let index = siblings
                .iter()
                .position(|child| *child == current)
                .unwrap_or_default();
            indices.push(index);
            current = parent;
        }
        indices.reverse();

        // 祖先链（目标在前）。
        let mut chain: Vec<InMemoryNodeId> = vec![node];
        while let Some(parent) = self.parent(*chain.last().expect("链非空")) {
            chain.push(parent);
        }

        let mut event = crate::events::DomEvent::new(kind, indices);
        let mut called = 0;
        for current in chain {
            let handlers: Vec<crate::events::ElementHandler> = self
                .nodes
                .borrow()
                .get(current.0)
                .map(|entry| {
                    entry
                        .listeners
                        .iter()
                        .filter(|(key, _)| *key == kind)
                        .map(|(_, handler)| handler.clone())
                        .collect()
                })
                .unwrap_or_default();
            for handler in handlers {
                handler.call(&mut event);
                called += 1;
                if event.is_propagation_stopped() {
                    return called;
                }
            }
        }
        called
    }

    /// 节点是否已被摘除（设计文档 §5.5.2 规则 T6 的可判定性）。
    pub fn is_detached(&self, node: InMemoryNodeId) -> bool {
        self.nodes
            .borrow()
            .get(node.0)
            .map(|entry| entry.detached)
            .unwrap_or(false)
    }

    /// 节点当前的父节点（`None` = 未挂到树上，或已被摘除）。
    pub fn parent(&self, node: InMemoryNodeId) -> Option<InMemoryNodeId> {
        self.nodes
            .borrow()
            .get(node.0)
            .and_then(|entry| entry.parent)
    }

    /// 已执行的操作序列（按发生顺序）。
    pub fn ops(&self) -> Vec<RecordedOp> {
        self.ops.borrow().clone()
    }

    /// 清空操作序列（让下一次断言只看「这一批」的操作）。
    pub fn clear_ops(&self) {
        self.ops.borrow_mut().clear();
    }

    /// 追加一个节点记录并返回句柄。
    fn push(&self, kind: InMemoryNodeKind) -> InMemoryNodeId {
        let mut nodes = self.nodes.borrow_mut();
        nodes.push(InMemoryNode {
            kind,
            attrs: Vec::new(),
            props: Vec::new(),
            children: Vec::new(),
            parent: None,
            listeners: Vec::new(),
            detached: false,
        });
        InMemoryNodeId(nodes.len() - 1)
    }

    /// 记一次操作。
    fn record(&self, op: DomOp, node: InMemoryNodeId) {
        self.ops.borrow_mut().push(RecordedOp { op, node });
    }

    /// 把子树整体标记为已摘除（T6：被移除的节点不参与本批次的写入）。
    fn mark_subtree_detached(&self, node: InMemoryNodeId) {
        let mut nodes = self.nodes.borrow_mut();
        let mut pending = vec![node];
        while let Some(current) = pending.pop() {
            let Some(entry) = nodes.get_mut(current.0) else {
                continue;
            };
            entry.detached = true;
            entry.parent = None;
            pending.extend(entry.children.iter().copied());
        }
    }
}

impl Dom for InMemoryDom {
    type Node = InMemoryNodeId;

    fn create_element(&self, tag: &str) -> Result<Self::Node, DomError> {
        let node = self.push(InMemoryNodeKind::Element(tag.to_owned()));
        self.record(
            DomOp::CreateElement {
                tag: tag.to_owned(),
            },
            node,
        );
        Ok(node)
    }

    fn create_text(&self, data: &str) -> Result<Self::Node, DomError> {
        let node = self.push(InMemoryNodeKind::Text(data.to_owned()));
        self.record(
            DomOp::CreateText {
                data: data.to_owned(),
            },
            node,
        );
        Ok(node)
    }

    fn set_attribute(&self, node: &Self::Node, name: &str, value: &str) -> Result<(), DomError> {
        let mut nodes = self.nodes.borrow_mut();
        let Some(entry) = nodes.get_mut(node.0) else {
            return Err(DomError::NodeNotFound);
        };
        if entry.detached {
            return Err(DomError::NodeDetached);
        }
        if matches!(entry.kind, InMemoryNodeKind::Text(_)) {
            return Err(DomError::Unsupported(String::from(
                "属性只能写在元素节点上",
            )));
        }
        match entry.attrs.iter_mut().find(|(key, _)| key == name) {
            Some(slot) => slot.1 = value.to_owned(),
            None => entry.attrs.push((name.to_owned(), value.to_owned())),
        }
        drop(nodes);
        self.record(
            DomOp::SetAttribute {
                name: name.to_owned(),
                value: value.to_owned(),
            },
            *node,
        );
        Ok(())
    }

    fn remove_attribute(&self, node: &Self::Node, name: &str) -> Result<(), DomError> {
        let mut nodes = self.nodes.borrow_mut();
        let Some(entry) = nodes.get_mut(node.0) else {
            return Err(DomError::NodeNotFound);
        };
        if entry.detached {
            return Err(DomError::NodeDetached);
        }
        entry.attrs.retain(|(key, _)| key != name);
        drop(nodes);
        self.record(
            DomOp::RemoveAttribute {
                name: name.to_owned(),
            },
            *node,
        );
        Ok(())
    }

    fn set_class_tokens(
        &self,
        node: &Self::Node,
        added: &[String],
        removed: &[String],
    ) -> Result<(), DomError> {
        let mut nodes = self.nodes.borrow_mut();
        let Some(entry) = nodes.get_mut(node.0) else {
            return Err(DomError::NodeNotFound);
        };
        if entry.detached {
            return Err(DomError::NodeDetached);
        }
        if matches!(entry.kind, InMemoryNodeKind::Text(_)) {
            return Err(DomError::Unsupported(String::from(
                "class 只能设在元素节点上",
            )));
        }
        let current: Vec<String> = entry
            .attrs
            .iter()
            .find(|(key, _)| key == "class")
            .map(|(_, value)| value.split_whitespace().map(str::to_owned).collect())
            .unwrap_or_default();
        let mut next: Vec<String> = current
            .into_iter()
            .filter(|token| !removed.contains(token))
            .collect();
        for token in added {
            if !next.contains(token) {
                next.push(token.clone());
            }
        }
        // token 集合为空 ⇒ 整个 class 属性不保留（不留 `class=""` 残渣）。
        if next.is_empty() {
            entry.attrs.retain(|(key, _)| key != "class");
        } else {
            let value = next.join(" ");
            match entry.attrs.iter_mut().find(|(key, _)| key == "class") {
                Some(slot) => slot.1 = value,
                None => entry.attrs.push((String::from("class"), value)),
            }
        }
        drop(nodes);
        self.record(
            DomOp::SetClassTokens {
                added: added.to_vec(),
                removed: removed.to_vec(),
            },
            *node,
        );
        Ok(())
    }

    fn set_property(&self, node: &Self::Node, name: &str, value: &str) -> Result<(), DomError> {
        let mut nodes = self.nodes.borrow_mut();
        let Some(entry) = nodes.get_mut(node.0) else {
            return Err(DomError::NodeNotFound);
        };
        if entry.detached {
            return Err(DomError::NodeDetached);
        }
        if matches!(entry.kind, InMemoryNodeKind::Text(_)) {
            return Err(DomError::Unsupported(String::from(
                "property 只能设在元素节点上",
            )));
        }
        match entry.props.iter_mut().find(|(key, _)| key == name) {
            Some(slot) => slot.1 = value.to_owned(),
            None => entry.props.push((name.to_owned(), value.to_owned())),
        }
        drop(nodes);
        self.record(
            DomOp::SetProperty {
                name: name.to_owned(),
                value: value.to_owned(),
            },
            *node,
        );
        Ok(())
    }

    fn get_property(&self, node: &Self::Node, name: &str) -> Result<String, DomError> {
        let nodes = self.nodes.borrow();
        let Some(entry) = nodes.get(node.0) else {
            return Err(DomError::NodeNotFound);
        };
        if entry.detached {
            return Err(DomError::NodeDetached);
        }
        let value = entry
            .props
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
            .unwrap_or_default();
        drop(nodes);
        self.record(
            DomOp::GetProperty {
                name: name.to_owned(),
            },
            *node,
        );
        Ok(value)
    }

    fn set_text(&self, node: &Self::Node, data: &str) -> Result<(), DomError> {
        let mut nodes = self.nodes.borrow_mut();
        let Some(entry) = nodes.get_mut(node.0) else {
            return Err(DomError::NodeNotFound);
        };
        if entry.detached {
            return Err(DomError::NodeDetached);
        }
        match &mut entry.kind {
            InMemoryNodeKind::Text(text) => {
                *text = data.to_owned();
            }
            InMemoryNodeKind::Element(_) => {
                return Err(DomError::Unsupported(String::from(
                    "set_text 只适用于文本节点",
                )))
            }
        }
        drop(nodes);
        self.record(
            DomOp::SetText {
                data: data.to_owned(),
            },
            *node,
        );
        Ok(())
    }

    fn append_child(&self, parent: &Self::Node, child: &Self::Node) -> Result<(), DomError> {
        self.insert_before(parent, child, None)
    }

    fn insert_before(
        &self,
        parent: &Self::Node,
        child: &Self::Node,
        anchor: Option<&Self::Node>,
    ) -> Result<(), DomError> {
        let mut nodes = self.nodes.borrow_mut();
        // 先判子节点（不可变读），再动父节点（可变写）——避免同一借用的重叠。
        match nodes.get(child.0) {
            None => return Err(DomError::NodeNotFound),
            Some(entry) if entry.detached => return Err(DomError::NodeDetached),
            Some(_) => {}
        }
        match nodes.get(parent.0) {
            None => return Err(DomError::NodeNotFound),
            Some(entry) if entry.detached => return Err(DomError::NodeDetached),
            Some(_) => {}
        }
        if parent.0 == child.0 {
            return Err(DomError::Unsupported(String::from(
                "节点不能成为自己的子节点",
            )));
        }

        // 从原父节点摘下（真实 DOM 的 insertBefore 语义：移动而非复制）。
        let previous_parent = nodes.get(child.0).and_then(|entry| entry.parent);
        if let Some(previous) = previous_parent {
            if let Some(previous_entry) = nodes.get_mut(previous.0) {
                previous_entry.children.retain(|node| node != child);
            }
        }

        // 定位锚点位置。
        let position = {
            let parent_entry = nodes.get(parent.0).ok_or(DomError::NodeNotFound)?;
            match anchor {
                None => parent_entry.children.len(),
                Some(anchor) => parent_entry
                    .children
                    .iter()
                    .position(|node| node == anchor)
                    .ok_or(DomError::NodeNotFound)?,
            }
        };

        {
            let parent_entry = nodes.get_mut(parent.0).ok_or(DomError::NodeNotFound)?;
            parent_entry.children.insert(position, *child);
        }
        if let Some(child_entry) = nodes.get_mut(child.0) {
            child_entry.parent = Some(*parent);
        }
        drop(nodes);

        let op = match anchor {
            None => DomOp::AppendChild { parent: *parent },
            Some(anchor) => DomOp::InsertBefore {
                parent: *parent,
                anchor: Some(*anchor),
            },
        };
        self.record(op, *child);
        Ok(())
    }

    fn remove_child(&self, parent: &Self::Node, child: &Self::Node) -> Result<(), DomError> {
        let mut nodes = self.nodes.borrow_mut();
        if nodes.get(child.0).is_none() {
            return Err(DomError::NodeNotFound);
        }
        {
            let Some(parent_entry) = nodes.get_mut(parent.0) else {
                return Err(DomError::NodeNotFound);
            };
            if parent_entry.detached {
                return Err(DomError::NodeDetached);
            }
            let position = parent_entry
                .children
                .iter()
                .position(|node| node == child)
                .ok_or(DomError::NodeNotFound)?;
            parent_entry.children.remove(position);
        }
        drop(nodes);
        // 摘除是终态：整棵子树标记为已脱离（T6：被移除的节点不参与本批次写入）。
        self.mark_subtree_detached(*child);
        self.record(DomOp::RemoveChild { parent: *parent }, *child);
        Ok(())
    }

    fn children(&self, parent: &Self::Node) -> Result<Vec<Self::Node>, DomError> {
        let nodes = self.nodes.borrow();
        let Some(entry) = nodes.get(parent.0) else {
            return Err(DomError::NodeNotFound);
        };
        Ok(entry.children.clone())
    }

    fn add_listener(
        &self,
        node: &Self::Node,
        kind: EventKind,
        handler: crate::events::ElementHandler,
    ) -> Result<(), DomError> {
        let mut nodes = self.nodes.borrow_mut();
        let Some(entry) = nodes.get_mut(node.0) else {
            return Err(DomError::NodeNotFound);
        };
        if entry.detached {
            return Err(DomError::NodeDetached);
        }
        match entry.listeners.iter_mut().find(|(key, _)| *key == kind) {
            Some(slot) => slot.1 = handler,
            None => entry.listeners.push((kind, handler)),
        }
        drop(nodes);
        self.record(DomOp::AddListener { kind }, *node);
        Ok(())
    }

    fn remove_listeners(&self, node: &Self::Node) -> Result<(), DomError> {
        let mut nodes = self.nodes.borrow_mut();
        let Some(entry) = nodes.get_mut(node.0) else {
            return Err(DomError::NodeNotFound);
        };
        entry.listeners.clear();
        drop(nodes);
        self.record(DomOp::RemoveListeners, *node);
        Ok(())
    }
}

/// **未接线**的 DOM 后端节点句柄（占位；没有任何实例能被造出来）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnavailableNode(pub usize);

/// **未接线**的 DOM 后端（P1 桩）。
///
/// 诚实口径：所有操作都**显式报错**（[`DomError::NotImplemented`]），
/// **不假装**「已经在往真实 DOM 写东西」。
///
/// 先例：`ui-gpu` 的 `UnavailableGpuContext`（`GpuError::NotImplemented`）。
#[derive(Debug, Clone, Copy, Default)]
pub struct UnavailableDom;

impl Dom for UnavailableDom {
    type Node = UnavailableNode;

    fn create_element(&self, _tag: &str) -> Result<Self::Node, DomError> {
        Err(DomError::NotImplemented)
    }

    fn create_text(&self, _data: &str) -> Result<Self::Node, DomError> {
        Err(DomError::NotImplemented)
    }

    fn set_attribute(&self, _node: &Self::Node, _name: &str, _value: &str) -> Result<(), DomError> {
        Err(DomError::NotImplemented)
    }

    fn remove_attribute(&self, _node: &Self::Node, _name: &str) -> Result<(), DomError> {
        Err(DomError::NotImplemented)
    }

    fn set_property(&self, _node: &Self::Node, _name: &str, _value: &str) -> Result<(), DomError> {
        Err(DomError::NotImplemented)
    }

    fn set_class_tokens(
        &self,
        _node: &Self::Node,
        _added: &[String],
        _removed: &[String],
    ) -> Result<(), DomError> {
        Err(DomError::NotImplemented)
    }

    fn get_property(&self, _node: &Self::Node, _name: &str) -> Result<String, DomError> {
        Err(DomError::NotImplemented)
    }

    fn set_text(&self, _node: &Self::Node, _data: &str) -> Result<(), DomError> {
        Err(DomError::NotImplemented)
    }

    fn append_child(&self, _parent: &Self::Node, _child: &Self::Node) -> Result<(), DomError> {
        Err(DomError::NotImplemented)
    }

    fn insert_before(
        &self,
        _parent: &Self::Node,
        _child: &Self::Node,
        _anchor: Option<&Self::Node>,
    ) -> Result<(), DomError> {
        Err(DomError::NotImplemented)
    }

    fn remove_child(&self, _parent: &Self::Node, _child: &Self::Node) -> Result<(), DomError> {
        Err(DomError::NotImplemented)
    }

    fn children(&self, _parent: &Self::Node) -> Result<Vec<Self::Node>, DomError> {
        Err(DomError::NotImplemented)
    }

    fn add_listener(
        &self,
        _node: &Self::Node,
        _kind: EventKind,
        _handler: crate::events::ElementHandler,
    ) -> Result<(), DomError> {
        Err(DomError::NotImplemented)
    }

    fn remove_listeners(&self, _node: &Self::Node) -> Result<(), DomError> {
        Err(DomError::NotImplemented)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    // ── 事件派发（内存后端的冒泡语义） ──────────────────────────────────

    /// 建一个带处理器的按钮节点（挂在 `parent` 下）。
    fn button_with_handler(
        dom: &InMemoryDom,
        parent: InMemoryNodeId,
        kind: EventKind,
        handler: crate::events::ElementHandler,
    ) -> InMemoryNodeId {
        let node = dom.create_element("button").expect("建节点必须成功");
        dom.add_listener(&node, kind, handler)
            .expect("挂处理器必须成功");
        dom.append_child(&parent, &node).expect("挂载必须成功");
        node
    }

    #[test]
    fn dispatch_calls_handlers_from_the_target_up_to_the_ancestors() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("main").expect("建节点必须成功");
        let middle = dom.create_element("div").expect("建节点必须成功");
        dom.append_child(&root, &middle).expect("挂载必须成功");

        let order = Rc::new(RefCell::new(Vec::<&'static str>::new()));
        let root_order = order.clone();
        dom.add_listener(
            &root,
            EventKind::Click,
            crate::events::ElementHandler::new(move |_event| {
                root_order.borrow_mut().push("root");
            }),
        )
        .expect("挂处理器必须成功");
        let target_order = order.clone();
        let target = button_with_handler(
            &dom,
            middle,
            EventKind::Click,
            crate::events::ElementHandler::new(move |_event| {
                target_order.borrow_mut().push("target");
            }),
        );

        assert_eq!(dom.dispatch(target, EventKind::Click), 2);
        assert_eq!(&*order.borrow(), &["target", "root"], "目标先、祖先后");
    }

    /// **反例**：处理器调用 `stop_propagation()` ⇒ 祖先后不再被调用。
    #[test]
    fn dispatch_stops_at_the_handler_that_asks_for_it() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("main").expect("建节点必须成功");
        let middle = dom.create_element("div").expect("建节点必须成功");
        dom.append_child(&root, &middle).expect("挂载必须成功");

        let order = Rc::new(RefCell::new(Vec::<&'static str>::new()));
        let root_order = order.clone();
        dom.add_listener(
            &root,
            EventKind::Click,
            crate::events::ElementHandler::new(move |_event| {
                root_order.borrow_mut().push("root");
            }),
        )
        .expect("挂处理器必须成功");
        let target_order = order.clone();
        let target = button_with_handler(
            &dom,
            middle,
            EventKind::Click,
            crate::events::ElementHandler::new(move |event| {
                target_order.borrow_mut().push("target");
                event.stop_propagation();
            }),
        );

        assert_eq!(dom.dispatch(target, EventKind::Click), 1);
        assert_eq!(&*order.borrow(), &["target"]);
    }

    /// **不误报**：另一类事件不会被同类处理器捕获。
    #[test]
    fn dispatch_only_calls_handlers_of_the_same_event_kind() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("main").expect("建节点必须成功");
        let clicked = Rc::new(Cell::new(0u32));
        let counter = clicked.clone();
        let node = button_with_handler(
            &dom,
            root,
            EventKind::Click,
            crate::events::ElementHandler::new(move |_event| {
                counter.set(counter.get() + 1);
            }),
        );

        assert_eq!(dom.dispatch(node, EventKind::KeyDown), 0);
        assert_eq!(clicked.get(), 0);
        assert_eq!(dom.dispatch(node, EventKind::Click), 1);
        assert_eq!(clicked.get(), 1);
    }

    #[test]
    fn remove_listeners_clears_the_handlers_of_that_node() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("main").expect("建节点必须成功");
        let hits = Rc::new(Cell::new(0u32));
        let counter = hits.clone();
        let node = button_with_handler(
            &dom,
            root,
            EventKind::Click,
            crate::events::ElementHandler::new(move |_event| {
                counter.set(counter.get() + 1);
            }),
        );

        assert_eq!(dom.dispatch(node, EventKind::Click), 1);
        dom.remove_listeners(&node).expect("摘处理器必须成功");
        assert_eq!(dom.dispatch(node, EventKind::Click), 0);
        assert_eq!(hits.get(), 1, "摘掉之后不再触发");
    }

    #[test]
    fn memory_backend_builds_a_readable_tree() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("div").expect("建节点必须成功");
        let text = dom.create_text("你好").expect("建节点必须成功");
        dom.append_child(&root, &text).expect("挂载必须成功");
        dom.set_attribute(&root, "class", "ap-root")
            .expect("写属性必须成功");

        assert_eq!(dom.tag(root), Some(String::from("div")));
        assert_eq!(dom.attribute(root, "class"), Some(String::from("ap-root")));
        assert_eq!(dom.text_content(root), "你好");
        assert_eq!(dom.children(&root).expect("读子节点必须成功"), vec![text]);
    }

    #[test]
    fn attribute_removal_and_text_rewrite_are_in_place() {
        let dom = InMemoryDom::new();
        let node = dom.create_element("input").expect("建节点必须成功");
        dom.set_attribute(&node, "value", "a")
            .expect("写属性必须成功");
        assert_eq!(dom.attribute(node, "value"), Some(String::from("a")));
        dom.set_attribute(&node, "value", "b")
            .expect("写属性必须成功");
        assert_eq!(dom.attribute(node, "value"), Some(String::from("b")));
        dom.remove_attribute(&node, "value")
            .expect("删属性必须成功");
        assert_eq!(dom.attribute(node, "value"), None);

        let text = dom.create_text("x").expect("建节点必须成功");
        dom.set_text(&text, "y").expect("写文本必须成功");
        assert_eq!(dom.text_content(text), "y");
        assert_eq!(dom.node_count(), 2, "改文本不得重建节点");
    }

    #[test]
    fn removing_a_child_detaches_it_from_the_parent() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("ul").expect("建节点必须成功");
        let item = dom.create_element("li").expect("建节点必须成功");
        dom.append_child(&root, &item).expect("挂载必须成功");
        assert_eq!(dom.children(&root).expect("读子节点必须成功").len(), 1);

        dom.remove_child(&root, &item).expect("摘除必须成功");
        assert!(dom.children(&root).expect("读子节点必须成功").is_empty());
        assert!(dom.is_detached(item), "摘除后节点被标记为已脱离");
        assert_eq!(dom.parent(item), None, "摘除后不再有父节点");
    }

    // ── P2 新增：操作序列记录 ─────────────────────────────────────────────

    #[test]
    fn recorded_operations_follow_the_execution_order() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("div").expect("建节点必须成功");
        let child = dom.create_element("span").expect("建节点必须成功");
        dom.set_attribute(&child, "class", "ap-x")
            .expect("写属性必须成功");
        dom.append_child(&root, &child).expect("挂载必须成功");

        let ops = dom.ops();
        assert_eq!(
            ops,
            vec![
                RecordedOp {
                    op: DomOp::CreateElement {
                        tag: String::from("div")
                    },
                    node: root
                },
                RecordedOp {
                    op: DomOp::CreateElement {
                        tag: String::from("span")
                    },
                    node: child
                },
                RecordedOp {
                    op: DomOp::SetAttribute {
                        name: String::from("class"),
                        value: String::from("ap-x")
                    },
                    node: child
                },
                RecordedOp {
                    op: DomOp::AppendChild { parent: root },
                    node: child
                },
            ]
        );
    }

    #[test]
    fn clearing_the_operation_log_keeps_the_tree_intact() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("div").expect("建节点必须成功");
        dom.clear_ops();
        assert!(dom.ops().is_empty());
        assert_eq!(dom.tag(root), Some(String::from("div")), "树不受影响");
    }

    // ── P2 新增：property 与插入语义 ──────────────────────────────────────

    #[test]
    fn properties_are_stored_separately_from_attributes() {
        let dom = InMemoryDom::new();
        let node = dom.create_element("input").expect("建节点必须成功");
        dom.set_property(&node, "value", "abc")
            .expect("写 property 必须成功");
        dom.set_attribute(&node, "value", "abc")
            .expect("写属性必须成功");

        assert_eq!(dom.property(node, "value"), Some(String::from("abc")));
        assert_eq!(dom.attribute(node, "value"), Some(String::from("abc")));
        assert_eq!(
            dom.get_property(&node, "value")
                .expect("读 property 必须成功"),
            String::from("abc")
        );
    }

    #[test]
    fn reading_an_unset_property_yields_an_empty_string() {
        let dom = InMemoryDom::new();
        let node = dom.create_element("input").expect("建节点必须成功");
        assert_eq!(
            dom.get_property(&node, "checked")
                .expect("读 property 必须成功"),
            String::new()
        );
    }

    #[test]
    fn insert_before_places_the_child_at_the_anchor_position() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("ul").expect("建节点必须成功");
        let first = dom.create_element("li").expect("建节点必须成功");
        let second = dom.create_element("li").expect("建节点必须成功");
        let inserted = dom.create_element("li").expect("建节点必须成功");
        dom.append_child(&root, &first).expect("挂载必须成功");
        dom.append_child(&root, &second).expect("挂载必须成功");

        dom.insert_before(&root, &inserted, Some(&first))
            .expect("插入必须成功");

        assert_eq!(
            dom.children(&root).expect("读子节点必须成功"),
            vec![inserted, first, second]
        );
    }

    #[test]
    fn insert_before_with_no_anchor_appends_to_the_end() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("ul").expect("建节点必须成功");
        let first = dom.create_element("li").expect("建节点必须成功");
        let last = dom.create_element("li").expect("建节点必须成功");
        dom.append_child(&root, &first).expect("挂载必须成功");

        dom.insert_before(&root, &last, None).expect("插入必须成功");

        assert_eq!(
            dom.children(&root).expect("读子节点必须成功"),
            vec![first, last]
        );
    }

    #[test]
    fn moving_a_node_between_parents_detaches_it_from_the_old_one() {
        let dom = InMemoryDom::new();
        let old_parent = dom.create_element("ul").expect("建节点必须成功");
        let new_parent = dom.create_element("ol").expect("建节点必须成功");
        let item = dom.create_element("li").expect("建节点必须成功");
        dom.append_child(&old_parent, &item).expect("挂载必须成功");

        dom.append_child(&new_parent, &item).expect("移动必须成功");

        assert!(dom
            .children(&old_parent)
            .expect("读子节点必须成功")
            .is_empty());
        assert_eq!(
            dom.children(&new_parent).expect("读子节点必须成功"),
            vec![item]
        );
        assert_eq!(dom.parent(item), Some(new_parent));
        assert!(
            !dom.is_detached(item),
            "移动不是摘除：节点仍在树上，不该被标记为脱离"
        );
    }

    #[test]
    fn inserting_before_a_missing_anchor_is_reported_as_not_found() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("ul").expect("建节点必须成功");
        let child = dom.create_element("li").expect("建节点必须成功");
        let ghost = InMemoryNodeId(999);

        assert_eq!(
            dom.insert_before(&root, &child, Some(&ghost))
                .expect_err("锚点不存在必须报错"),
            DomError::NodeNotFound
        );
    }

    // ── 判别测试：失败必须显式（不许静默降级）──────────────────────────

    /// 写入**已摘除**的节点必须报错（设计文档 §5.5.2 规则 T6 的可判定性）。
    #[test]
    fn writing_to_a_removed_node_is_reported_as_detached() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("ul").expect("建节点必须成功");
        let item = dom.create_element("li").expect("建节点必须成功");
        dom.append_child(&root, &item).expect("挂载必须成功");
        dom.remove_child(&root, &item).expect("摘除必须成功");

        let err = dom
            .set_attribute(&item, "class", "ap-row")
            .expect_err("写入已摘除节点必须报错");
        assert_eq!(err, DomError::NodeDetached);
        assert_eq!(dom.attribute(item, "class"), None, "被拒绝的写不得留下痕迹");
    }

    /// 挂回**已摘除**的节点必须报错（摘除是终态）。
    #[test]
    fn reattaching_a_removed_node_is_refused() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("ul").expect("建节点必须成功");
        let item = dom.create_element("li").expect("建节点必须成功");
        dom.append_child(&root, &item).expect("挂载必须成功");
        dom.remove_child(&root, &item).expect("摘除必须成功");

        let err = dom
            .append_child(&root, &item)
            .expect_err("重新挂回已摘除节点必须报错");
        assert_eq!(err, DomError::NodeDetached);
    }

    /// 不存在的节点必须报错，不得静默吞掉。
    #[test]
    fn operating_on_a_missing_node_is_reported_as_not_found() {
        let dom = InMemoryDom::new();
        let ghost = InMemoryNodeId(999);
        assert_eq!(
            dom.set_attribute(&ghost, "class", "x")
                .expect_err("不存在的节点必须报错"),
            DomError::NodeNotFound
        );
        assert_eq!(
            dom.children(&ghost).expect_err("不存在的节点必须报错"),
            DomError::NodeNotFound
        );
        assert_eq!(
            dom.get_property(&ghost, "value")
                .expect_err("不存在的节点必须报错"),
            DomError::NodeNotFound
        );
    }

    /// 不适用的操作必须报错（属性写到文本节点上）。
    #[test]
    fn unsupported_operations_are_reported_explicitly() {
        let dom = InMemoryDom::new();
        let text = dom.create_text("x").expect("建节点必须成功");
        let err = dom
            .set_attribute(&text, "class", "x")
            .expect_err("文本节点不接受属性");
        assert!(matches!(err, DomError::Unsupported(_)), "实测：{err:?}");

        let err = dom
            .set_property(&text, "value", "x")
            .expect_err("文本节点不接受 property");
        assert!(matches!(err, DomError::Unsupported(_)), "实测：{err:?}");
    }

    /// 未接线的后端：**每个**操作都显式报 `NotImplemented`。
    #[test]
    fn unavailable_backend_reports_not_implemented_for_every_operation() {
        let dom = UnavailableDom;
        let node = UnavailableNode(0);
        assert_eq!(
            dom.create_element("div").expect_err("未接线后端必须报错"),
            DomError::NotImplemented
        );
        assert_eq!(
            dom.create_text("x").expect_err("未接线后端必须报错"),
            DomError::NotImplemented
        );
        assert_eq!(
            dom.set_attribute(&node, "class", "x")
                .expect_err("未接线后端必须报错"),
            DomError::NotImplemented
        );
        assert_eq!(
            dom.remove_attribute(&node, "class")
                .expect_err("未接线后端必须报错"),
            DomError::NotImplemented
        );
        assert_eq!(
            dom.set_property(&node, "value", "x")
                .expect_err("未接线后端必须报错"),
            DomError::NotImplemented
        );
        assert_eq!(
            dom.get_property(&node, "value")
                .expect_err("未接线后端必须报错"),
            DomError::NotImplemented
        );
        assert_eq!(
            dom.set_text(&node, "x").expect_err("未接线后端必须报错"),
            DomError::NotImplemented
        );
        assert_eq!(
            dom.append_child(&node, &node)
                .expect_err("未接线后端必须报错"),
            DomError::NotImplemented
        );
        assert_eq!(
            dom.insert_before(&node, &node, None)
                .expect_err("未接线后端必须报错"),
            DomError::NotImplemented
        );
        assert_eq!(
            dom.remove_child(&node, &node)
                .expect_err("未接线后端必须报错"),
            DomError::NotImplemented
        );
        assert_eq!(
            dom.children(&node).expect_err("未接线后端必须报错"),
            DomError::NotImplemented
        );
    }

    /// 错误消息必须可读（0 装纪律：报错要能定位）。
    #[test]
    fn dom_errors_render_readable_messages() {
        assert_eq!(
            DomError::NodeNotFound.to_string(),
            "NodeNotFound: 节点不存在"
        );
        assert_eq!(
            DomError::NodeDetached.to_string(),
            "NodeDetached: 节点已摘除"
        );
        assert!(DomError::NotImplemented
            .to_string()
            .contains("NotImplemented"));
        assert_eq!(
            DomError::Unsupported(String::from("x")).to_string(),
            "Unsupported: x"
        );
    }
}
