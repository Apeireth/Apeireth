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
//! - **按对象身份**（`===`）判定 ⇒ 不同节点不会互相误判；
//! - **整棵子树**都记账（2026-10-25 修）⇒ 与内存后端一致 ⇒
//!   ⇒ 规则 T6 在浏览器里才被**真正强制**（修前只标记顶层节点 ⇒ 子节点
//!   仍可被写入 ⇒ T6 在浏览器里是**假的**）。

use std::cell::{Cell, RefCell};

use js_sys::{Object, Reflect, WeakMap, WeakSet};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{Document, Element, Event, KeyboardEvent, MouseEvent, Node, Text};

use crate::dom::{Dom, DomError, ElementBox, EventKind, ScrollMetrics};
use crate::events::{DomEvent, ElementHandler};
use crate::listeners::ListenerTable;

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
    /// 节点 → 引擎内编号（弱引用 ⇒ 节点被回收后编号自动消失）。
    ///
    /// ## 为什么要有编号
    ///
    /// 监听器登记表按 **(节点, 事件类型)** 索引 ⇒ 键必须**可哈希**⇒
    /// ⇒ 而 `web_sys::Node` **不可哈希**（JS 对象没有内容哈希）⇒
    /// ⇒ 用弱引用映射给每个节点发一个编号 ⇒ ⇒ 表的键 = (编号, 事件类型)。
    node_ids: RefCell<WeakMap<Object, JsValue>>,
    /// 下一个节点编号。
    next_node_id: Cell<u32>,
    /// 已挂的监听器（按 **(节点编号, 事件类型)** 索引 ⇒ 增量替换）。
    listeners: RefCell<ListenerTable<u32, WebListener>>,
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
            node_ids: RefCell::new(WeakMap::<Object, JsValue>::new_typed()),
            next_node_id: Cell::new(0),
            listeners: RefCell::new(ListenerTable::new()),
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

    /// 标记**整棵子树**已被引擎摘除（与内存后端同口径 ⇒ T6 真被强制）。
    fn mark_subtree_detached(&self, node: &Node) {
        self.mark_detached(node);
        let children = node.child_nodes();
        for index in 0..children.length() {
            let Some(child) = children.get(index) else {
                continue;
            };
            self.mark_subtree_detached(&child);
        }
    }

    /// 节点可写性检查（被摘除 ⇒ [`DomError::NodeDetached`]）。
    fn ensure_writable(&self, node: &Node) -> Result<(), DomError> {
        if self.is_detached(node) {
            return Err(DomError::NodeDetached);
        }
        Ok(())
    }

    /// 节点的引擎内编号（**没有**则 `None`；不分配）。
    fn lookup_node_id(&self, node: &Node) -> Option<u32> {
        let key = node_object(node)?;
        let existing = self.node_ids.borrow().get_checked(&key)?;
        existing.as_f64().map(|value| value as u32)
    }

    /// 节点的引擎内编号（没有 ⇒ 分配一个）。
    fn node_id(&self, node: &Node) -> u32 {
        if let Some(id) = self.lookup_node_id(node) {
            return id;
        }
        let id = self.next_node_id.get();
        self.next_node_id.set(id.saturating_add(1));
        if let Some(key) = node_object(node) {
            self.node_ids
                .borrow_mut()
                .set(&key, &JsValue::from_f64(f64::from(id)));
        }
        id
    }

    /// 取节点上的元素视图（非元素 ⇒ [`DomError::Unsupported`]）。
    fn as_element<'a>(&self, node: &'a Node) -> Result<&'a Element, DomError> {
        node.dyn_ref::<Element>().ok_or_else(|| {
            DomError::Unsupported(String::from("该节点不是元素节点（属性只能写在元素上）"))
        })
    }
}

impl Dom for WebDom {
    fn bounding_box(&self, node: &Self::Node) -> Result<ElementBox, DomError> {
        let element = self.as_element(node)?;
        let rect = element.get_bounding_client_rect();
        Ok(ElementBox::new(
            rect.x(),
            rect.y(),
            rect.width(),
            rect.height(),
        ))
    }

    fn scroll_metrics(&self, node: &Self::Node) -> Result<ScrollMetrics, DomError> {
        let element = self.as_element(node)?;
        Ok(ScrollMetrics {
            scroll_top: f64::from(element.scroll_top()),
            scroll_height: f64::from(element.scroll_height()),
            client_height: f64::from(element.client_height()),
        })
    }

    fn set_scroll_top(&self, node: &Self::Node, top: f64) -> Result<(), DomError> {
        self.ensure_writable(node)?;
        let element = self.as_element(node)?;
        // 浏览器侧 `scrollTop` 是整数像素；先按真实度量**夹住**再写，
        // ⇒ 避免 `f64::NAN` / 负值 / 超界写成非法状态。
        let metrics = ScrollMetrics {
            scroll_top: 0.0,
            scroll_height: f64::from(element.scroll_height()),
            client_height: f64::from(element.client_height()),
        };
        element.set_scroll_top(metrics.clamp_scroll_top(top).round() as i32);
        Ok(())
    }

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

    fn set_property_bool(
        &self,
        node: &Self::Node,
        name: &str,
        value: bool,
    ) -> Result<(), DomError> {
        self.ensure_writable(node)?;
        let element = self.as_element(node)?;
        let target: JsValue = element.into();
        // ⚠️ 必须写**真布尔**（不是 `"true"` / `"false"` 字符串）—— 见 trait 上的说明。
        Reflect::set(
            &target,
            &JsValue::from_str(name),
            &JsValue::from_bool(value),
        )
        .map(|_| ())
        .map_err(|error| {
            DomError::Unsupported(format!("set_property_bool({name}) 失败：{error:?}"))
        })
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

    /// **移除** property ⇒ 回到**默认值**（浏览器语义）。
    ///
    /// ## 为什么要分「已知」与「未知」
    ///
    /// DOM 的 IDL 属性**没有**通用的「删除」语义 ⇒
    /// ⇒ 能识别的那几个（`value` / `checked` / …）写回**默认值**；
    /// ⇒ 识别不了的 ⇒ **显式失败**（`Unsupported`）⇒ 渲染器据此**降级**
    /// （写回默认值 + 记警告）⇒ **不假装支持**（0 装）。
    fn remove_property(&self, node: &Self::Node, name: &str) -> Result<(), DomError> {
        self.ensure_writable(node)?;
        let element = self.as_element(node)?;
        let target: JsValue = element.clone().into();
        // ① 自有属性（引擎自己挂上去的展开属性）⇒ 真的删掉。
        if let Some(object) = element.clone().dyn_into::<Object>().ok() {
            let _ = Reflect::delete_property(&object, &JsValue::from_str(name));
        }
        // ② 反射型 IDL 属性 ⇒ 写回**默认值**（浏览器语义：没有该属性时的取值）。
        let Some(default) = default_property_value(name) else {
            return Err(DomError::Unsupported(format!(
                "remove_property({name})：浏览器没有通用的删除语义（该属性无法回落到默认值）"
            )));
        };
        Reflect::set(&target, &JsValue::from_str(name), &default)
            .map(|_| ())
            .map_err(|error| {
                DomError::Unsupported(format!("remove_property({name}) 失败：{error:?}"))
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
        // 摘除是终态：**整棵子树**记账（T6：被摘除的节点再写入 ⇒ NodeDetached）。
        self.mark_subtree_detached(child);
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
            if is_visible(&child) {
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
        // 同一节点同一事件类型只留一个监听器（覆盖语义）⇒ 只摘**同事件类型**的。
        //
        // ⚠️ 不能摘「全部」：一个元素上挂**多个事件类型**的处理器时（例：标题
        // 同时挂 `click` 与 `keydown`），摘全部会把先挂的那个一起带走 ⇒ 先挂的
        // 处理器**静默失效**（点击不再触发）。
        self.remove_listener(node, kind)?;
        let closure = Closure::wrap(Box::new(move |event: Event| {
            let target = event
                .target()
                .and_then(|value| value.dyn_ref::<Node>().cloned());
            let mut dom_event = DomEvent::new(kind, target_path(target.as_ref()));
            // ⭐ **带上目标取值**（2026-10-25 修）：`input` / `change` 事件里
            // 控件的当前值（`value` property）⇒ 处理器**不必**再回读 DOM。
            //
            // ## 为什么必须有
            //
            // 生产的原生控件用 `bind:value` ⇒ 取值随 `input` 事件推进
            // ⇒ ⇒ **事件负载里就带着值**。
            // ⇒ 引擎侧此前**从不填** ⇒ `event.target_value()` 恒为 `None`
            // ⇒ ⇒ 消费方（`ui-kit/settings_budget.rs` 的旋钮）只能拿到 `""`
            // ⇒ ⇒ **用户敲进去的字符被丢掉** ⇒ ⇒ 实测：键入 `87` 后
            // 引擎侧草稿是 `""`、DOM 仍是浏览器自己画的 `87`
            // ⇒ ⇒ **幽灵输入**：用户看得见，引擎不知道。
            //
            // ⚠️ 只在目标**确实是表单控件**时填（`value` 是字符串 property）
            // ⇒ 非控件元素读不到 `value` ⇒ 跳过（不留 `undefined` 语义）。
            if let Some(element) = target.as_ref().and_then(|node| node.dyn_ref::<Element>()) {
                if let Ok(value) = Reflect::get(element.as_ref(), &JsValue::from_str("value")) {
                    if let Some(text) = value.as_string() {
                        dom_event = dom_event.with_target_value(text);
                    }
                }
            }
            // ⭐ **失焦的落点**（`relatedTarget`）⇒ 判「真失焦」用。
            //
            // `focusout` 在焦点从 A 挪到 B（同组件内部）时**也会**冒泡 ⇒
            // ⇒ 没有落点就分不清「真失焦」与「内部挪焦点」。
            if kind == EventKind::FocusOut {
                if let Some(related) = related_target(&event) {
                    dom_event = dom_event.with_related_target_path(target_path(Some(&related)));
                }
            }
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
        // 登记表按 **(节点编号, 事件类型)** 索引 ⇒ 增量替换是 O(1)。
        let id = self.node_id(node);
        self.listeners.borrow_mut().insert(
            id,
            kind,
            WebListener {
                node: node.clone(),
                kind,
                closure,
            },
        );
        Ok(())
    }

    fn remove_listeners(&self, node: &Self::Node) -> Result<(), DomError> {
        let Some(id) = self.lookup_node_id(node) else {
            // 从没挂过监听器 ⇒ 无事可做（幂等）。
            return Ok(());
        };
        let removed = self.listeners.borrow_mut().remove_node(&id);
        for (_kind, listener) in removed {
            let _ = listener.node.remove_event_listener_with_callback(
                listener.kind.as_str(),
                listener.closure.as_ref().unchecked_ref(),
            );
        }
        Ok(())
    }

    /// 摘掉某节点上**指定事件类型**的监听器（其余类型的**保留**）。
    ///
    /// ⇒ 差分时「换了处理器 ⇒ 重挂」只动**那一个** kind ⇒
    /// ⇒ 同元素的其它 kind **不受影响**（既有契约）⇒
    /// ⇒ 真实 DOM 侧也不再是「每次重渲染全量拆装」（O(N²)）。
    fn remove_listener(&self, node: &Self::Node, kind: EventKind) -> Result<(), DomError> {
        let Some(id) = self.lookup_node_id(node) else {
            return Ok(());
        };
        let removed = self.listeners.borrow_mut().remove_kind(&id, kind);
        if let Some(listener) = removed {
            let _ = node.remove_event_listener_with_callback(
                kind.as_str(),
                listener.closure.as_ref().unchecked_ref(),
            );
        }
        Ok(())
    }

    fn target_path(&self, node: &Self::Node) -> Result<Vec<usize>, DomError> {
        Ok(target_path(Some(node)))
    }
}

/// 反射型 IDL 属性的**默认值**（浏览器语义：没有该属性时的取值）。
///
/// ## 为什么要有这张表
///
/// DOM 的 IDL 属性**没有**通用的「删除」语义 ⇒ ⇒ 只能对**认得出**的属性
/// 写回默认值 ⇒ ⇒ 认不出的 ⇒ 后端**显式失败**（`Unsupported`）⇒
/// ⇒ 渲染器降级（写回默认值 + 记警告）⇒ **不假装支持**（0 装）。
fn default_property_value(name: &str) -> Option<JsValue> {
    match name {
        // 字符串型（DOMString）⇒ 默认值是**空串**。
        "value" | "placeholder" | "title" | "alt" | "name" | "id" | "defaultValue" => {
            Some(JsValue::from_str(""))
        }
        // 布尔型 ⇒ 默认值是 `false`。
        "checked" | "selected" | "disabled" | "multiple" | "required" | "readOnly" | "hidden"
        | "indeterminate" | "open" => Some(JsValue::from_bool(false)),
        _ => None,
    }
}

/// 失焦事件的**落点**（`relatedTarget`；没有 ⇒ `None`）。
///
/// ⚠️ 用 `Reflect` 读（而不是 `FocusEvent`）⇒ **不必**给 web-sys 额外开
/// feature（依赖面不变）⇒ ⇒ 也就能在只有 `Event` 的情况下工作。
fn related_target(event: &Event) -> Option<Node> {
    let value = Reflect::get(event.as_ref(), &JsValue::from_str("relatedTarget")).ok()?;
    if value.is_null() || value.is_undefined() {
        return None;
    }
    value.dyn_ref::<Node>().cloned()
}

/// 节点 → `Object`（弱引用映射的**键**必须是对象；不是对象 ⇒ `None`）。
fn node_object(node: &Node) -> Option<Object> {
    node.clone().dyn_into::<Object>().ok()
}

/// 节点是否是「**可见子节点**」（元素 / 文本；注释节点**不算**）。
///
/// ## 为什么注释节点不算
///
/// 内存后端里**只有**元素与文本 ⇒ 它的子节点下标空间里没有注释 ⇒
/// ⇒ 浏览器的下标若把注释算进去 ⇒ ⇒ 两侧的 `target_path` **不可比**
/// ⇒ ⇒ 委托路由（按路径定位）会错 ⇒ ⇒ 统一：**只数元素与文本**。
fn is_visible(node: &Node) -> bool {
    node.dyn_ref::<Element>().is_some() || node.dyn_ref::<Text>().is_some()
}

/// 目标节点在文档里的**子节点下标路径**（从根到目标）。
///
/// 与内存后端的路径口径一致（`DomEvent::target_path`：从根到目标的子节点
/// 下标序列）⇒ 两端的事件对象形状可比。
///
/// ⚠️ 下标**只数元素与文本**（注释节点跳过）⇒ 与内存后端同一口径。
fn target_path(target: Option<&Node>) -> Vec<usize> {
    let Some(target) = target else {
        return Vec::new();
    };
    let mut indices: Vec<usize> = Vec::new();
    let mut current = target.clone();
    while let Some(parent) = current.parent_node() {
        indices.push(visible_child_index(&parent, &current));
        current = parent;
    }
    indices.reverse();
    indices
}

/// 目标在父节点的**可见子节点**（元素 / 文本）里的下标。
///
/// ⚠️ **注释节点跳过** ⇒ 与内存后端同一口径 ⇒ 两端的 `target_path` 可比
/// （口径由 [`crate::events::visible_child_index`] 统一，两侧共用同一份实现）。
fn visible_child_index(parent: &Node, child: &Node) -> usize {
    let list = parent.child_nodes();
    let mut visible: Vec<bool> = Vec::with_capacity(list.length() as usize);
    let mut target: Option<usize> = None;
    for candidate in 0..list.length() {
        let Some(node) = list.get(candidate) else {
            // 拿不到 ⇒ 记成「不可见」（保持下标空间对齐）。
            visible.push(false);
            continue;
        };
        visible.push(is_visible(&node));
        if node == *child {
            target = Some(candidate as usize);
        }
    }
    match target {
        Some(index) => crate::events::visible_child_index(&visible, index),
        None => visible.iter().filter(|is_visible| **is_visible).count(),
    }
}
