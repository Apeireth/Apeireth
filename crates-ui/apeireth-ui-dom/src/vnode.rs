//! 虚拟节点（VNode）：DOM 的**纯数据结构**表示。
//!
//! 设计文档 §5.1.1：VNode 是普通 Rust 类型（可 `PartialEq`、可 `Debug`）⇒ 断言
//! 写成 `assert_eq!(vnode, expected)`，而不是「读源码文本断言」——这是消灭 38 处
//! 源码镜像测试的结构性前提。
//!
//! ## 四个分区（设计文档 §5.1.1）
//!
//! | 分区 | 载体 | 差分策略 |
//! |---|---|---|
//! | `class` | `attrs` 里名为 `class` 的项 | **按 token 差分**（增删单个 class） |
//! | `style` | `attrs` 里名为 `style` 的项 | 整体字符串比对，不同才写 |
//! | `attrs` | 其余属性 | 键级差分（新增 / 改值 / 消失） |
//! | `bindings` | [`ElementVNode::this_ref`]（`bind:this`） | 键级差分（槽位换了才重新接线） |

use crate::binding::ThisRef;
use crate::dom::EventKind;
use crate::events::ElementHandler;

/// 列表项身份（key）：不透明的可比较值（设计文档 §5.2.3）。
///
/// 由 `view!` 宏从 `key = expr` 推出（字符串 / 整数）。
/// **不支持复合 key**（本仓 0 处需要）。
///
/// > **与设计文档的一处刻意差异**：设计文档写的是 `Key::Str(&'static str)`；
/// > 本实现用**拥有所有权**的 `String`。理由：key 是**运行期**值（列表项的
/// > 标识），不可能是 `'static`；`'static str` 只够描述「编译期字面量 key」
/// > 这一种用法。`String` 是 `&'static str` 的严格超集 ⇒ 不丢能力。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Key {
    /// 字符串 key。
    Str(String),
    /// 有符号整数 key。
    Int(i64),
    /// 无符号整数 key。
    Uint(u64),
}

impl Key {
    /// 从字符串构造 key。
    pub fn str(value: impl Into<String>) -> Self {
        Key::Str(value.into())
    }

    /// 从有符号整数构造 key。
    pub fn int(value: i64) -> Self {
        Key::Int(value)
    }

    /// 从无符号整数构造 key。
    pub fn uint(value: u64) -> Self {
        Key::Uint(value)
    }
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Key::Str(text) => write!(f, "{text}"),
            Key::Int(value) => write!(f, "{value}"),
            Key::Uint(value) => write!(f, "{value}"),
        }
    }
}

impl From<&str> for Key {
    fn from(value: &str) -> Self {
        Key::Str(value.to_owned())
    }
}

impl From<String> for Key {
    fn from(value: String) -> Self {
        Key::Str(value)
    }
}

impl From<i64> for Key {
    fn from(value: i64) -> Self {
        Key::Int(value)
    }
}

impl From<u64> for Key {
    fn from(value: u64) -> Self {
        Key::Uint(value)
    }
}

/// 属性值。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttrValue {
    /// 文本值。
    Text(String),
    /// 布尔值（`false` ⇒ **不写该属性**，而非写 `false`）。
    Bool(bool),
}

impl AttrValue {
    /// 转成「要写入 DOM 的属性值」；布尔 `false` 返回 `None`（表示不写）。
    pub fn to_attr_string(&self) -> Option<String> {
        match self {
            AttrValue::Text(text) => Some(text.clone()),
            AttrValue::Bool(flag) => {
                if *flag {
                    Some(String::from("true"))
                } else {
                    None
                }
            }
        }
    }
}

impl From<&str> for AttrValue {
    fn from(value: &str) -> Self {
        AttrValue::Text(value.to_owned())
    }
}

impl From<String> for AttrValue {
    fn from(value: String) -> Self {
        AttrValue::Text(value)
    }
}

impl From<bool> for AttrValue {
    fn from(value: bool) -> Self {
        AttrValue::Bool(value)
    }
}

/// 虚拟节点。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VNode {
    /// 元素节点。
    Element(ElementVNode),
    /// 文本节点。
    Text(TextVNode),
    /// 分组节点（无宿主节点，对应列表 / 条件块容器）。
    Fragment(Vec<VNode>),
}

impl VNode {
    /// 建一个元素节点（返回构造器）。
    pub fn element(tag: impl Into<String>) -> ElementBuilder {
        ElementBuilder::new(tag)
    }

    /// 建一个文本节点。
    pub fn text(data: impl Into<String>) -> Self {
        VNode::Text(TextVNode { data: data.into() })
    }

    /// 建一个分组节点。
    pub fn fragment(children: impl IntoIterator<Item = VNode>) -> Self {
        VNode::Fragment(children.into_iter().collect())
    }

    /// 本节点的标签（元素节点才有）。
    pub fn tag(&self) -> Option<&str> {
        match self {
            VNode::Element(element) => Some(element.tag.as_str()),
            _ => None,
        }
    }

    /// 本节点的子节点（元素节点才有）。
    pub fn children(&self) -> &[VNode] {
        match self {
            VNode::Element(element) => element.children.as_slice(),
            VNode::Fragment(children) => children.as_slice(),
            _ => &[],
        }
    }

    /// 本节点的 key（只有带 key 的元素节点有）。
    pub fn key(&self) -> Option<&Key> {
        match self {
            VNode::Element(element) => element.key.as_ref(),
            _ => None,
        }
    }

    /// 本节点的属性值（非元素节点 ⇒ `None`）。
    pub fn attr_value(&self, name: &str) -> Option<&AttrValue> {
        match self {
            VNode::Element(element) => element.attr_value(name),
            _ => None,
        }
    }

    /// 本节点的 class token 集合（保序；非元素节点 ⇒ 空）。
    pub fn class_tokens(&self) -> Vec<String> {
        match self.attr_value("class") {
            Some(AttrValue::Text(text)) => text.split_whitespace().map(str::to_owned).collect(),
            _ => Vec::new(),
        }
    }
}

/// 元素节点。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementVNode {
    /// 标签名。
    pub tag: String,
    /// 列表项身份（keyed 差分用；`None` ⇒ 身份由位置决定）。
    pub key: Option<Key>,
    /// 属性（保持写入顺序 ⇒ 差分可逐字对比）。
    pub attrs: Vec<(String, AttrValue)>,
    /// `bind:this` 槽位（类型擦除；见 [`ThisRef`]）。
    ///
    /// `None` ⇒ 该元素没有元素引用绑定。
    pub this_ref: Option<ThisRef>,
    /// 事件处理器（按事件类型索引）。
    ///
    /// 处理器**按指针身份**比较（见 [`ElementHandler`]）⇒ 差分能判断
    /// 「处理器换了没有」（换了 ⇒ 重挂监听器）。
    pub handlers: Vec<(EventKind, ElementHandler)>,
    /// 子节点。
    pub children: Vec<VNode>,
}

impl ElementVNode {
    /// 建一个元素节点。
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            tag: tag.into(),
            key: None,
            attrs: Vec::new(),
            this_ref: None,
            handlers: Vec::new(),
            children: Vec::new(),
        }
    }

    /// 设置列表项身份（keyed 差分用）。
    pub fn key(mut self, key: impl Into<Key>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// 挂上 `bind:this` 槽位（元素引用绑定）。
    ///
    /// 槽位是**类型擦除**的（[`ThisRef`]）⇒ 虚拟树不必知道后端的节点类型。
    pub fn this(mut self, this_ref: ThisRef) -> Self {
        self.this_ref = Some(this_ref);
        self
    }

    /// 读 key。
    pub fn key_value(&self) -> Option<&Key> {
        self.key.as_ref()
    }

    /// 追加一个属性（同名属性覆盖，保持原位置）。
    pub fn attr(mut self, name: impl Into<String>, value: impl Into<AttrValue>) -> Self {
        let name = name.into();
        let value = value.into();
        match self.attrs.iter_mut().find(|(key, _)| *key == name) {
            Some(slot) => slot.1 = value,
            None => self.attrs.push((name, value)),
        }
        self
    }

    /// 追加一个 class token（保序去重；class 必须**按 token** 差分，见设计 §5.2.1）。
    pub fn class(mut self, token: impl Into<String>) -> Self {
        let token = token.into();
        let existing = self
            .attrs
            .iter()
            .find(|(key, _)| key == "class")
            .map(|(_, value)| value.clone());
        match existing {
            Some(AttrValue::Text(mut text)) => {
                if text.split_whitespace().any(|item| item == token) {
                    return self;
                }
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(&token);
                let attrs = std::mem::take(&mut self.attrs);
                self.attrs = attrs
                    .into_iter()
                    .map(|(key, value)| {
                        if key == "class" {
                            (key, AttrValue::Text(text.clone()))
                        } else {
                            (key, value)
                        }
                    })
                    .collect();
                self
            }
            Some(_) => self,
            None => {
                self.attrs
                    .push((String::from("class"), AttrValue::Text(token)));
                self
            }
        }
    }

    /// 挂一个事件处理器（同一事件类型挂第二个 ⇒ 覆盖第一个）。
    pub fn handler(mut self, kind: EventKind, handler: ElementHandler) -> Self {
        match self.handlers.iter_mut().find(|(key, _)| *key == kind) {
            Some(slot) => slot.1 = handler,
            None => self.handlers.push((kind, handler)),
        }
        self
    }

    /// 追加子节点。
    pub fn child(mut self, child: VNode) -> Self {
        self.children.push(child);
        self
    }
    /// 追加多个子节点。
    pub fn children(mut self, children: impl IntoIterator<Item = VNode>) -> Self {
        self.children.extend(children);
        self
    }

    /// 读属性值。
    pub fn attr_value(&self, name: &str) -> Option<&AttrValue> {
        self.attrs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    }

    /// 产出虚拟节点。
    pub fn build(self) -> VNode {
        VNode::Element(self)
    }
}

/// 文本节点。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextVNode {
    /// 文本内容。
    pub data: String,
}

/// 元素节点构造器。
#[derive(Debug, Clone)]
pub struct ElementBuilder {
    node: ElementVNode,
}

impl ElementBuilder {
    /// 建一个元素节点构造器。
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            node: ElementVNode::new(tag),
        }
    }

    /// 设置列表项身份（keyed 差分用）。
    pub fn key(mut self, key: impl Into<Key>) -> Self {
        self.node = self.node.key(key);
        self
    }

    /// 挂上 `bind:this` 槽位（元素引用绑定）。
    pub fn this(mut self, this_ref: ThisRef) -> Self {
        self.node = self.node.this(this_ref);
        self
    }

    /// 追加属性。
    pub fn attr(mut self, name: impl Into<String>, value: impl Into<AttrValue>) -> Self {
        self.node = self.node.attr(name, value);
        self
    }

    /// 追加 class token。
    pub fn class(mut self, token: impl Into<String>) -> Self {
        self.node = self.node.class(token);
        self
    }

    /// 挂一个事件处理器。
    pub fn handler(mut self, kind: EventKind, handler: ElementHandler) -> Self {
        self.node = self.node.handler(kind, handler);
        self
    }

    /// 追加子节点。
    pub fn child(mut self, child: VNode) -> Self {
        self.node = self.node.child(child);
        self
    }

    /// 追加多个子节点。
    pub fn children(mut self, children: impl IntoIterator<Item = VNode>) -> Self {
        self.node = self.node.children(children);
        self
    }

    /// 产出虚拟节点。
    pub fn build(self) -> VNode {
        VNode::Element(self.node)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_tokens_are_merged_and_deduplicated() {
        let node = ElementVNode::new("div")
            .class("a")
            .class("b")
            .class("a")
            .build();

        match node {
            VNode::Element(element) => {
                assert_eq!(
                    element.attr_value("class"),
                    Some(&AttrValue::Text(String::from("a b")))
                );
            }
            other => panic!("实测节点形态：{other:?}"),
        }
    }

    #[test]
    fn bool_false_attribute_is_not_written() {
        assert_eq!(AttrValue::Bool(false).to_attr_string(), None);
        assert_eq!(
            AttrValue::Bool(true).to_attr_string(),
            Some(String::from("true"))
        );
        assert_eq!(
            AttrValue::Text(String::from("x")).to_attr_string(),
            Some(String::from("x"))
        );
    }

    #[test]
    fn element_handlers_compare_by_pointer_identity() {
        let handler = ElementHandler::new(|_event| {});
        let same = handler.clone();
        let other = ElementHandler::new(|_event| {});

        assert_eq!(handler, same, "同一个闭包 ⇒ 同一个处理器");
        assert_ne!(handler, other, "不同闭包 ⇒ 不同处理器");
    }

    #[test]
    fn a_second_handler_of_the_same_kind_replaces_the_first() {
        let node = ElementVNode::new("button")
            .handler(EventKind::Click, ElementHandler::new(|_event| {}))
            .handler(EventKind::Click, ElementHandler::new(|_event| {}));
        assert_eq!(node.handlers.len(), 1, "同一事件类型只留一个处理器");
    }

    #[test]
    fn same_name_attribute_is_overwritten_in_place() {
        let element = ElementVNode::new("input")
            .attr("type", "text")
            .attr("value", "a")
            .attr("type", "email")
            .build();
        match element {
            VNode::Element(element) => {
                assert_eq!(element.attrs.len(), 2);
                assert_eq!(
                    element.attr_value("type"),
                    Some(&AttrValue::Text(String::from("email")))
                );
            }
            other => panic!("实测节点形态：{other:?}"),
        }
    }
}
