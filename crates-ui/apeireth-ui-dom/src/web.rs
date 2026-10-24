//! 真实 DOM 后端（web-sys）—— **wasm32 目标专用**。
//!
//! ## 诚实边界（**必读**）
//!
//! - 本模块整体被 `#[cfg(target_arch = "wasm32")]` 隔离 ⇒ **native 编译时
//!   不参与** ⇒ 它的行为**未经任何运行时验证**（wasm 构建通过只证明
//!   「能编译」，**不证明**画面正确）。
//! - 浏览器内验证（真实 DOM 行为 / 像素对照 / 逐帧对照）属 P3 的浏览器内
//!   对照测试（试点 harness，见 `crates-ui/apeireth-ui-demo/harness/`）。
//! - **事件挂载形态**：本批按**元素直挂**（每个带处理器的元素一个监听器，
//!   冒泡交给浏览器）。设计文档 §5.3.2 写的是「委托根单监听器」形态；
//!   两者的 `currentTarget` 语义**完全一致**（§5.3.3 的确定解：= 处理器
//!   注册在哪个元素上），差别只在监听器数量 ⇒ 委托形态列为 🔵 未接线。
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
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{Document, Element, Event, KeyboardEvent, MouseEvent, Node, Text};

use crate::dom::{Dom, DomError, EventKind};
use crate::events::{DomEvent, ElementHandler};

/// 真实 DOM 后端上的一个监听器（闭包必须留在后端里，否则 JS 侧的回调悬空）。
#[derive(Debug)]
struct WebListener {
    node: Node,
    kind: EventKind,
    closure: Closure<dyn FnMut(Event)>,
}

/// 真实 DOM 后端（web-sys）。
#[derive(Debug)]
pub struct WebDom {
    document: Document,
    /// 已被引擎摘除的节点（弱引用集合 ⇒ 判据见模块文档）。
    detached: RefCell<WeakSet<JsValue>>,
    /// 已挂的监听器（闭包住在这里 ⇒ 生命周期与后端一致）。
    listeners: RefCell<Vec<WebListener>>,
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
            listeners: RefCell::new(Vec::new()),
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

    fn create_element_ns(&self, namespace: &str, tag: &str) -> Result<Self::Node, DomError> {
        self.document
            .create_element_ns(Some(namespace), tag)
            .map(Element::into)
            .map_err(|error| {
                DomError::Unsupported(format!(
                    "create_element_ns({namespace}, {tag}) 失败：{error:?}"
                ))
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

    fn add_listener(
        &self,
        node: &Self::Node,
        kind: EventKind,
        handler: ElementHandler,
    ) -> Result<(), DomError> {
        self.ensure_writable(node)?;
        // 同一节点同一事件类型只留一个监听器（覆盖语义）。
        self.remove_listeners(node)?;
        let closure = Closure::wrap(Box::new(move |event: Event| {
            let target = event
                .target()
                .and_then(|value| value.dyn_ref::<Node>().cloned());
            let mut dom_event = DomEvent::new(kind, target_path(target.as_ref()));
            if let Some(keyboard) = event.dyn_ref::<KeyboardEvent>() {
                dom_event = dom_event.with_key(keyboard.key());
            }
            if let Some(mouse) = event.dyn_ref::<MouseEvent>() {
                dom_event =
                    dom_event.with_modifiers(mouse.ctrl_key(), mouse.shift_key(), mouse.meta_key());
            }
            handler.call(&mut dom_event);
            if dom_event.is_default_prevented() {
                event.prevent_default();
            }
            if dom_event.is_propagation_stopped() {
                event.stop_propagation();
            }
        }) as Box<dyn FnMut(Event)>);
        node.add_event_listener_with_callback(kind.as_str(), closure.as_ref().unchecked_ref())
            .map_err(|error| {
                DomError::Unsupported(format!(
                    "add_event_listener({}) 失败：{error:?}",
                    kind.as_str()
                ))
            })?;
        self.listeners.borrow_mut().push(WebListener {
            node: node.clone(),
            kind,
            closure,
        });
        Ok(())
    }

    fn remove_listeners(&self, node: &Self::Node) -> Result<(), DomError> {
        let mut listeners = self.listeners.borrow_mut();
        let mut kept = Vec::with_capacity(listeners.len());
        for listener in listeners.drain(..) {
            if &listener.node == node {
                let _ = node.remove_event_listener_with_callback(
                    listener.kind.as_str(),
                    listener.closure.as_ref().unchecked_ref(),
                );
            } else {
                kept.push(listener);
            }
        }
        *listeners = kept;
        Ok(())
    }
}

/// 目标节点在文档里的**子节点下标路径**（从根到目标）。
///
/// 与内存后端的路径口径一致（`DomEvent::target_path`：从根到目标的子节点
/// 下标序列）⇒ 两端的事件对象形状可比。
fn target_path(target: Option<&Node>) -> Vec<usize> {
    let Some(target) = target else {
        return Vec::new();
    };
    let mut indices: Vec<usize> = Vec::new();
    let mut current = target.clone();
    while let Some(parent) = current.parent_node() {
        let list = parent.child_nodes();
        let mut index = 0usize;
        for candidate in 0..list.length() {
            let Some(child) = list.get(candidate) else {
                continue;
            };
            if child == current {
                index = candidate as usize;
                break;
            }
        }
        indices.push(index);
        current = parent;
    }
    indices.reverse();
    indices
}
