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
//! P0 骨架阶段：只落后端抽象与内存实现；真实后端（web-sys）等 P0 探测结论
//! 回来后再接（探测子线实测：wasm-bindgen 0.2.129 ⟷ js-sys 0.3.106 ⟷
//! web-sys 0.3.106，三者精确硬钉 1:1）。未接线的后端以
//! [`UnavailableDom`] 显式存在（每个操作都报错，**不假装**自己在工作）。

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
    /// **未接线**：真实 DOM 后端（web-sys）尚未接入（等 P0 探测结论）。
    NotImplemented,
    /// 不支持的操作。
    Unsupported(String),
}

impl std::fmt::Display for DomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DomError::NodeNotFound => write!(f, "NodeNotFound: 节点不存在"),
            DomError::NodeDetached => write!(f, "NodeDetached: 节点已摘除"),
            DomError::NotImplemented => {
                write!(
                    f,
                    "NotImplemented: 真实 DOM 后端未接线（P0 探测结论落地后接入）"
                )
            }
            DomError::Unsupported(detail) => write!(f, "Unsupported: {detail}"),
        }
    }
}

impl std::error::Error for DomError {}

/// DOM 后端抽象：渲染层唯一允许触碰「真实节点」的地方。
pub trait Dom {
    /// 后端节点类型（真实后端是 web-sys 的元素句柄，内存后端是编号）。
    type Node: Clone + PartialEq;

    /// 建一个元素节点。
    fn create_element(&self, tag: &str) -> Result<Self::Node, DomError>;

    /// 建一个文本节点。
    fn create_text(&self, data: &str) -> Result<Self::Node, DomError>;

    /// 写属性。
    fn set_attribute(&self, node: &Self::Node, name: &str, value: &str) -> Result<(), DomError>;

    /// 删属性。
    fn remove_attribute(&self, node: &Self::Node, name: &str) -> Result<(), DomError>;

    /// 写文本内容（就地改，不重建文本节点，见设计 §5.2.4）。
    fn set_text(&self, node: &Self::Node, data: &str) -> Result<(), DomError>;

    /// 追加子节点。
    fn append_child(&self, parent: &Self::Node, child: &Self::Node) -> Result<(), DomError>;

    /// 移除子节点（节点被摘除后再写入 ⇒ [`DomError::NodeDetached`]）。
    fn remove_child(&self, parent: &Self::Node, child: &Self::Node) -> Result<(), DomError>;

    /// 读子节点列表。
    fn children(&self, parent: &Self::Node) -> Result<Vec<Self::Node>, DomError>;
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
    children: Vec<InMemoryNodeId>,
    /// 是否已被从父节点摘除（摘除后禁止再写入 —— 设计文档 §5.5.2 规则 T6）。
    detached: bool,
}

/// 内存 DOM 后端：native 单测用（永远可用，不需浏览器、不需 wasm target）。
#[derive(Debug, Default)]
pub struct InMemoryDom {
    nodes: RefCell<Vec<InMemoryNode>>,
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

    /// 节点是否已被摘除（设计文档 §5.5.2 规则 T6 的可判定性）。
    pub fn is_detached(&self, node: InMemoryNodeId) -> bool {
        self.nodes
            .borrow()
            .get(node.0)
            .map(|entry| entry.detached)
            .unwrap_or(false)
    }

    /// 追加一个节点记录并返回句柄。
    fn push(&self, kind: InMemoryNodeKind) -> InMemoryNodeId {
        let mut nodes = self.nodes.borrow_mut();
        nodes.push(InMemoryNode {
            kind,
            attrs: Vec::new(),
            children: Vec::new(),
            detached: false,
        });
        InMemoryNodeId(nodes.len() - 1)
    }
}

impl Dom for InMemoryDom {
    type Node = InMemoryNodeId;

    fn create_element(&self, tag: &str) -> Result<Self::Node, DomError> {
        Ok(self.push(InMemoryNodeKind::Element(tag.to_owned())))
    }

    fn create_text(&self, data: &str) -> Result<Self::Node, DomError> {
        Ok(self.push(InMemoryNodeKind::Text(data.to_owned())))
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
        Ok(())
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
                Ok(())
            }
            InMemoryNodeKind::Element(_) => Err(DomError::Unsupported(String::from(
                "set_text 只适用于文本节点",
            ))),
        }
    }

    fn append_child(&self, parent: &Self::Node, child: &Self::Node) -> Result<(), DomError> {
        let mut nodes = self.nodes.borrow_mut();
        // 先判子节点（不可变读），再动父节点（可变写）——避免同一借用的重叠。
        let child_state = match nodes.get(child.0) {
            None => Err(DomError::NodeNotFound),
            Some(entry) if entry.detached => Err(DomError::NodeDetached),
            Some(_) => Ok(()),
        };
        child_state?;

        let Some(parent_entry) = nodes.get_mut(parent.0) else {
            return Err(DomError::NodeNotFound);
        };
        if parent_entry.detached {
            return Err(DomError::NodeDetached);
        }
        if !parent_entry.children.contains(child) {
            parent_entry.children.push(*child);
        }
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
        let Some(child_entry) = nodes.get_mut(child.0) else {
            return Err(DomError::NodeNotFound);
        };
        child_entry.detached = true;
        Ok(())
    }

    fn children(&self, parent: &Self::Node) -> Result<Vec<Self::Node>, DomError> {
        let nodes = self.nodes.borrow();
        let Some(entry) = nodes.get(parent.0) else {
            return Err(DomError::NodeNotFound);
        };
        Ok(entry.children.clone())
    }
}

/// **未接线**的 DOM 后端节点句柄（占位；没有任何实例能被造出来）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnavailableNode(pub usize);

/// **未接线**的 DOM 后端（P1 桩）。
///
/// 诚实口径：所有操作都**显式报错**（[`DomError::NotImplemented`]），
/// **不假装**「已经在往真实 DOM 写东西」。真实后端（web-sys）接入后由它替换。
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

    fn set_text(&self, _node: &Self::Node, _data: &str) -> Result<(), DomError> {
        Err(DomError::NotImplemented)
    }

    fn append_child(&self, _parent: &Self::Node, _child: &Self::Node) -> Result<(), DomError> {
        Err(DomError::NotImplemented)
    }

    fn remove_child(&self, _parent: &Self::Node, _child: &Self::Node) -> Result<(), DomError> {
        Err(DomError::NotImplemented)
    }

    fn children(&self, _parent: &Self::Node) -> Result<Vec<Self::Node>, DomError> {
        Err(DomError::NotImplemented)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            dom.set_text(&node, "x").expect_err("未接线后端必须报错"),
            DomError::NotImplemented
        );
        assert_eq!(
            dom.append_child(&node, &node)
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
