//! DOM 后端抽象：真实 DOM 与内存 DOM 共用同一套渲染逻辑。
//!
//! 设计文档 §5.1.2：**这是本设计能消灭源码镜像测试的结构性前提** —— 差分 /
//! 委托 / 绑定逻辑全部写成 `D: Dom` 泛型代码，因而 100% 可 native 单测。
//!
//! P0 骨架阶段：只落后端抽象与内存实现；真实后端（web-sys）等 P0 探测结论
//! 回来后再接（探测子线实测：wasm-bindgen 0.2.129 ⟷ js-sys 0.3.106 ⟷
//! web-sys 0.3.106，三者精确硬钉 1:1）。

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
}

impl std::fmt::Display for DomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DomError::NodeNotFound => write!(f, "NodeNotFound: 节点不存在"),
            DomError::NodeDetached => write!(f, "NodeDetached: 节点已摘除"),
        }
    }
}

impl std::error::Error for DomError {}

/// DOM 后端抽象：渲染层唯一允许触碰「真实节点」的地方。
pub trait Dom {
    /// 后端节点类型（真实后端是 web-sys 的元素句柄，内存后端是编号）。
    type Node: Clone + PartialEq;

    /// 建一个元素节点。
    fn create_element(&self, tag: &str) -> Self::Node;

    /// 建一个文本节点。
    fn create_text(&self, data: &str) -> Self::Node;

    /// 写属性。
    fn set_attribute(&self, node: &Self::Node, name: &str, value: &str);

    /// 删属性。
    fn remove_attribute(&self, node: &Self::Node, name: &str);

    /// 写文本内容（就地改，不重建文本节点，见设计 §5.2.4）。
    fn set_text(&self, node: &Self::Node, data: &str);

    /// 追加子节点。
    fn append_child(&self, parent: &Self::Node, child: &Self::Node);

    /// 移除子节点。
    fn remove_child(&self, parent: &Self::Node, child: &Self::Node);

    /// 读子节点列表。
    fn children(&self, parent: &Self::Node) -> Vec<Self::Node>;
}

/// 内存 DOM 的节点句柄（编号）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InMemoryNodeId(pub usize);

#[derive(Debug, Clone)]
enum InMemoryNodeKind {
    Element(String),
    Text(String),
}

#[derive(Debug, Clone)]
struct InMemoryNode {
    kind: InMemoryNodeKind,
    attrs: Vec<(String, String)>,
    children: Vec<InMemoryNodeId>,
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

    /// 追加一个节点记录并返回句柄。
    fn push(&self, kind: InMemoryNodeKind) -> InMemoryNodeId {
        let mut nodes = self.nodes.borrow_mut();
        nodes.push(InMemoryNode {
            kind,
            attrs: Vec::new(),
            children: Vec::new(),
        });
        InMemoryNodeId(nodes.len() - 1)
    }
}

impl Dom for InMemoryDom {
    type Node = InMemoryNodeId;

    fn create_element(&self, tag: &str) -> Self::Node {
        self.push(InMemoryNodeKind::Element(tag.to_owned()))
    }

    fn create_text(&self, data: &str) -> Self::Node {
        self.push(InMemoryNodeKind::Text(data.to_owned()))
    }

    fn set_attribute(&self, node: &Self::Node, name: &str, value: &str) {
        let mut nodes = self.nodes.borrow_mut();
        let Some(entry) = nodes.get_mut(node.0) else {
            return;
        };
        match entry.attrs.iter_mut().find(|(key, _)| key == name) {
            Some(slot) => slot.1 = value.to_owned(),
            None => entry.attrs.push((name.to_owned(), value.to_owned())),
        }
    }

    fn remove_attribute(&self, node: &Self::Node, name: &str) {
        let mut nodes = self.nodes.borrow_mut();
        if let Some(entry) = nodes.get_mut(node.0) {
            entry.attrs.retain(|(key, _)| key != name);
        }
    }

    fn set_text(&self, node: &Self::Node, data: &str) {
        let mut nodes = self.nodes.borrow_mut();
        if let Some(entry) = nodes.get_mut(node.0) {
            if let InMemoryNodeKind::Text(text) = &mut entry.kind {
                *text = data.to_owned();
            }
        }
    }

    fn append_child(&self, parent: &Self::Node, child: &Self::Node) {
        let mut nodes = self.nodes.borrow_mut();
        let Some(entry) = nodes.get_mut(parent.0) else {
            return;
        };
        if !entry.children.contains(child) {
            entry.children.push(*child);
        }
    }

    fn remove_child(&self, parent: &Self::Node, child: &Self::Node) {
        let mut nodes = self.nodes.borrow_mut();
        if let Some(entry) = nodes.get_mut(parent.0) {
            entry.children.retain(|node| node != child);
        }
    }

    fn children(&self, parent: &Self::Node) -> Vec<Self::Node> {
        let nodes = self.nodes.borrow();
        nodes
            .get(parent.0)
            .map(|entry| entry.children.clone())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_backend_builds_a_readable_tree() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("div");
        let text = dom.create_text("你好");
        dom.append_child(&root, &text);
        dom.set_attribute(&root, "class", "ap-root");

        assert_eq!(dom.tag(root), Some(String::from("div")));
        assert_eq!(dom.attribute(root, "class"), Some(String::from("ap-root")));
        assert_eq!(dom.text_content(root), "你好");
        assert_eq!(dom.children(&root), vec![text]);
    }

    #[test]
    fn attribute_removal_and_text_rewrite_are_in_place() {
        let dom = InMemoryDom::new();
        let node = dom.create_element("input");
        dom.set_attribute(&node, "value", "a");
        assert_eq!(dom.attribute(node, "value"), Some(String::from("a")));
        dom.set_attribute(&node, "value", "b");
        assert_eq!(dom.attribute(node, "value"), Some(String::from("b")));
        dom.remove_attribute(&node, "value");
        assert_eq!(dom.attribute(node, "value"), None);

        let text = dom.create_text("x");
        dom.set_text(&text, "y");
        assert_eq!(dom.text_content(text), "y");
        assert_eq!(dom.node_count(), 2, "改文本不得重建节点");
    }

    #[test]
    fn removing_a_child_detaches_it_from_the_parent() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("ul");
        let item = dom.create_element("li");
        dom.append_child(&root, &item);
        assert_eq!(dom.children(&root).len(), 1);

        dom.remove_child(&root, &item);
        assert!(dom.children(&root).is_empty());
    }
}
