//! 真实 DOM 后端（web-sys）—— **wasm32 目标专用**。
//!
//! ## 诚实边界（**必读**）
//!
//! - 本模块整体被 `#[cfg(target_arch = "wasm32")]` 隔离 ⇒ **native 编译时
//!   不参与** ⇒ 它的行为**未经任何运行时验证**（wasm 构建通过只证明
//!   「能编译」，**不证明**画面正确）。
//! - 浏览器内验证（真实 DOM 行为 / 像素对照 / 逐帧对照）属 P5 的浏览器 job
//!   —— **待建**。
//! - 本后端**不实现事件委托**：`Dom` trait 目前没有监听器接口 ⇒ 事件由
//!   [`crate::events`] 的委托表独立承担，尚未接到真实 DOM。
//!
//! ## 「已摘除」的判定
//!
//! 真实 DOM 里，被摘除的节点**仍然是个合法对象**（写属性不会报错）⇒
//! 「已摘除」必须由引擎**显式记账**才能判定（设计文档 §5.5.2 规则 T6 的
//! 可判定性要求）。本后端用 [`js_sys::WeakSet`] 记账：
//! - **弱引用** ⇒ 节点被回收后记账自动消失，不泄漏；
//! - **按对象身份**（`===`）判定 ⇒ 不同节点不会互相误判。

use std::cell::RefCell;

use js_sys::{Reflect, WeakSet};
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{Document, Element, Node, Text};

use crate::dom::{Dom, DomError};

/// 真实 DOM 后端（web-sys）。
#[derive(Debug)]
pub struct WebDom {
    document: Document,
    /// 已被引擎摘除的节点（弱引用集合 ⇒ 判据见模块文档）。
    detached: RefCell<WeakSet<JsValue>>,
}

impl WebDom {
    /// 从当前窗口的文档建后端。
    ///
    /// 非浏览器环境（没有 `window`）⇒ 显式报错，**不假装**能工作。
    pub fn new() -> Result<Self, DomError> {
        let window = web_sys::window().ok_or_else(|| {
            DomError::Unsupported(String::from("当前环境没有 window（不是浏览器上下文？）"))
        })?;
        let document = window.document().ok_or_else(|| {
            DomError::Unsupported(String::from("window.document() 不可用（文档未就绪？）"))
        })?;
        Ok(Self::from_document(document))
    }

    /// 从给定文档建后端。
    pub fn from_document(document: Document) -> Self {
        Self {
            document,
            detached: RefCell::new(WeakSet::<JsValue>::new_typed()),
        }
    }

    /// 读文档。
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// 节点是否已被引擎摘除（规则 T6 的判据）。
    fn is_detached(&self, node: &Node) -> bool {
        let value: JsValue = node.clone().into();
        self.detached.borrow().has(&value)
    }

    /// 标记节点已被引擎摘除。
    fn mark_detached(&self, node: &Node) {
        let value: JsValue = node.clone().into();
        let _ = self.detached.borrow_mut().add(&value);
    }

    /// 节点可写性检查（被摘除 ⇒ [`DomError::NodeDetached`]）。
    fn ensure_writable(&self, node: &Node) -> Result<(), DomError> {
        if self.is_detached(node) {
            return Err(DomError::NodeDetached);
        }
        Ok(())
    }

    /// 取节点上的元素视图（非元素 ⇒ [`DomError::Unsupported`]）。
    fn as_element<'a>(&self, node: &'a Node) -> Result<&'a Element, DomError> {
        node.dyn_ref::<Element>().ok_or_else(|| {
            DomError::Unsupported(String::from("该节点不是元素节点（属性只能写在元素上）"))
        })
    }
}

impl Dom for WebDom {
    type Node = Node;

    fn create_element(&self, tag: &str) -> Result<Self::Node, DomError> {
        self.document
            .create_element(tag)
            .map(Element::into)
            .map_err(|error| {
                DomError::Unsupported(format!("create_element({tag}) 失败：{error:?}"))
            })
    }

    fn create_text(&self, data: &str) -> Result<Self::Node, DomError> {
        Ok(self.document.create_text_node(data).into())
    }

    fn set_attribute(&self, node: &Self::Node, name: &str, value: &str) -> Result<(), DomError> {
        self.ensure_writable(node)?;
        let element = self.as_element(node)?;
        element.set_attribute(name, value).map_err(|error| {
            DomError::Unsupported(format!("set_attribute({name}) 失败：{error:?}"))
        })
    }

    fn remove_attribute(&self, node: &Self::Node, name: &str) -> Result<(), DomError> {
        self.ensure_writable(node)?;
        let element = self.as_element(node)?;
        element.remove_attribute(name).map_err(|error| {
            DomError::Unsupported(format!("remove_attribute({name}) 失败：{error:?}"))
        })
    }

    fn set_class_tokens(
        &self,
        node: &Self::Node,
        added: &[String],
        removed: &[String],
    ) -> Result<(), DomError> {
        self.ensure_writable(node)?;
        let element = self.as_element(node)?;
        let class_list = element.class_list();
        for token in removed {
            class_list.remove_1(token).map_err(|error| {
                DomError::Unsupported(format!("class token 删除失败（{token}）：{error:?}"))
            })?;
        }
        for token in added {
            class_list.add_1(token).map_err(|error| {
                DomError::Unsupported(format!("class token 添加失败（{token}）：{error:?}"))
            })?;
        }
        Ok(())
    }

    fn set_property(&self, node: &Self::Node, name: &str, value: &str) -> Result<(), DomError> {
        self.ensure_writable(node)?;
        let element = self.as_element(node)?;
        let target: JsValue = element.clone().into();
        Reflect::set(&target, &JsValue::from_str(name), &JsValue::from_str(value))
            .map(|_| ())
            .map_err(|error| DomError::Unsupported(format!("set_property({name}) 失败：{error:?}")))
    }

    fn get_property(&self, node: &Self::Node, name: &str) -> Result<String, DomError> {
        self.ensure_writable(node)?;
        let element = self.as_element(node)?;
        let target: JsValue = element.clone().into();
        let value = Reflect::get(&target, &JsValue::from_str(name)).map_err(|error| {
            DomError::Unsupported(format!("get_property({name}) 失败：{error:?}"))
        })?;
        Ok(match value.as_string() {
            Some(text) => text,
            None => match value.as_bool() {
                Some(flag) => flag.to_string(),
                None => String::new(),
            },
        })
    }

    fn set_text(&self, node: &Self::Node, data: &str) -> Result<(), DomError> {
        self.ensure_writable(node)?;
        if node.dyn_ref::<Text>().is_none() {
            return Err(DomError::Unsupported(String::from(
                "set_text 只适用于文本节点",
            )));
        }
        // CharacterData 的 nodeValue 写入口（DOM 规范：文本节点的 nodeValue
        // 就是它的 data）——就地改文本，**不重建节点**（设计文档 §5.2.4）。
        node.set_node_value(Some(data));
        Ok(())
    }

    fn append_child(&self, parent: &Self::Node, child: &Self::Node) -> Result<(), DomError> {
        self.ensure_writable(parent)?;
        self.ensure_writable(child)?;
        parent
            .append_child(child)
            .map(|_| ())
            .map_err(|error| DomError::Unsupported(format!("append_child 失败：{error:?}")))
    }

    fn insert_before(
        &self,
        parent: &Self::Node,
        child: &Self::Node,
        anchor: Option<&Self::Node>,
    ) -> Result<(), DomError> {
        self.ensure_writable(parent)?;
        self.ensure_writable(child)?;
        parent
            .insert_before(child, anchor)
            .map(|_| ())
            .map_err(|error| DomError::Unsupported(format!("insert_before 失败：{error:?}")))
    }

    fn remove_child(&self, parent: &Self::Node, child: &Self::Node) -> Result<(), DomError> {
        self.ensure_writable(parent)?;
        self.ensure_writable(child)?;
        parent
            .remove_child(child)
            .map(|_| ())
            .map_err(|error| DomError::Unsupported(format!("remove_child 失败：{error:?}")))?;
        // 摘除是终态：记账（T6：被摘除的节点再写入 ⇒ NodeDetached）。
        self.mark_detached(child);
        Ok(())
    }

    fn children(&self, parent: &Self::Node) -> Result<Vec<Self::Node>, DomError> {
        self.ensure_writable(parent)?;
        let list = parent.child_nodes();
        let mut children = Vec::with_capacity(list.length() as usize);
        for index in 0..list.length() {
            let Some(child) = list.get(index) else {
                continue;
            };
            // 只认元素与文本（与内存后端的口径一致 ⇒ 两侧下标可对照）。
            if child.dyn_ref::<Element>().is_some() || child.dyn_ref::<Text>().is_some() {
                children.push(child);
            }
        }
        Ok(children)
    }
}
