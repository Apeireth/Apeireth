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
    /// ⭐ **IME 组合态**（L1-G3）：与事件闭包**共享**（`Rc<Cell<bool>>`）
    /// ⇒ ⇒ 组合事件到达时置位，渲染器经 `Dom::is_composing()` 读到
    /// ⇒ ⇒ **不必依赖宿主转告**（否则宿主忘一处就回到「中文输入被打散」）。
    composing: std::rc::Rc<Cell<bool>>,
    /// ⭐ **document 级组合事件监听器**（持有所有权，避免被回收后静默失效）。
    ///
    /// 为什么必须由后端自己装：若只在「应用监听了 composition 事件」时才更新组合态
    /// ⇒ ⇒ 不监听的应用会**静默失去 IME 保护**（中文输入被打散，且无任何报错）。
    // ⚠️ **「持有」本身就是目的**（clippy 会误报「never read」）：
    //   这两个 `Closure` 是 **JS 侧回调的 Rust 端所有者** ⇒ ⇒ 一旦被 drop
    //   ⇒ ⇒ JS 侧监听器回调失效 ⇒ ⇒ **IME 组合态跟踪静默失效**
    //   ⇒ ⇒ 字段**不读**是对的（读了反而多余），但必须**留着**。
    #[allow(dead_code)]
    #[allow(clippy::type_complexity)]
    composition_closures: (Closure<dyn FnMut(Event)>, Closure<dyn FnMut(Event)>),
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
        let composing = std::rc::Rc::new(Cell::new(false));
        // ⭐ **document 级**跟踪组合态：与「应用是否监听了 composition 事件」无关
        //   ⇒ ⇒ 否则不监听的应用会静默失去 IME 保护。
        let start_flag = std::rc::Rc::clone(&composing);
        let start = Closure::wrap(
            Box::new(move |_event: Event| start_flag.set(true)) as Box<dyn FnMut(Event)>
        );
        let end_flag = std::rc::Rc::clone(&composing);
        let end = Closure::wrap(
            Box::new(move |_event: Event| end_flag.set(false)) as Box<dyn FnMut(Event)>
        );
        for (name, callback) in [
            ("compositionstart", start.as_ref().unchecked_ref()),
            ("compositionend", end.as_ref().unchecked_ref()),
        ] {
            if let Err(error) = document.add_event_listener_with_callback(name, callback) {
                // 显式告警（不静默）：拿不到 document 级监听 ⇒ IME 保护会退化。
                web_sys::console::warn_1(
                    &format!(
                        "引擎：无法安装 document 级 `{name}` 监听（IME 保护将退化）：{error:?}"
                    )
                    .into(),
                );
            }
        }
        Self {
            document,
            detached: RefCell::new(WeakSet::<JsValue>::new_typed()),
            node_ids: RefCell::new(WeakMap::<Object, JsValue>::new_typed()),
            composing,
            composition_closures: (start, end),
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
    /// 摘除**整棵子树**（引擎侧记账：被摘除的节点再写入 ⇒ `NodeDetached`）。
    ///
    /// ## ⚠️ 为什么**这里不摘监听器**（2026-11-05 的一次**实测教训**）
    ///
    /// 曾试过「摘除时一并 `remove_listeners`」⇒ ⇒ **WebKitGTK 侧直接把页面跑挂了**
    /// ⇒ ⇒ 机制：`remove_listeners` 会 **drop** 掉 wasm 侧的 `Closure`
    /// ⇒ ⇒ 若该闭包**正在执行**（事件处理过程中移除节点）
    /// ⇒ ⇒ ⇒ wasm-bindgen 直接 **panic**：
    /// ⇒ ⇒ ⇒ 「closure invoked recursively or after being dropped」
    /// ⇒ ⇒ ⇒ wasm 实例 trap ⇒ ⇒ 页面**停在探针中途**（实测 `stage=wasm-ready`）。
    ///
    /// ⇒ ⇒ ⇒ **就地 drop 闭包是不安全的** ⇒ ⇒ 任何「摘除时清理监听器」的方案
    /// ⇒ ⇒ ⇒ 都必须**延后 drop**（等事件派发结束后再释放）⇒ ⇒ ⇒ **未做**。
    ///
    /// ⇒ ⇒ ⇒ ⇒ 当前口径：监听器**随 `WebDom` 一起释放**（`WebDom` drop ⇒ ⇒ 闭包 drop）
    /// ⇒ ⇒ ⇒ ⇒ 而 `WebDom` 的生命周期由**调用方**管 ⇒ ⇒ ⇒ 这是**调用方责任**。
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

impl Drop for WebDom {
    fn drop(&mut self) {
        // ① document 级：composition 监听器（**必须先摘**，否则 document 上留死闭包）。
        for (name, callback) in [
            (
                "compositionstart",
                self.composition_closures.0.as_ref().unchecked_ref(),
            ),
            (
                "compositionend",
                self.composition_closures.1.as_ref().unchecked_ref(),
            ),
        ] {
            let _ = self
                .document
                .remove_event_listener_with_callback(name, callback);
        }
        // ② 元素级：把表里所有监听器摘掉（JS 侧）⇒ ⇒ 随表 drop 的 `Closure` 才安全。
        let drained = self.listeners.borrow_mut().drain_all();
        for (_kind, listener) in drained {
            let _ = listener.node.remove_event_listener_with_callback(
                listener.kind.as_str(),
                listener.closure.as_ref().unchecked_ref(),
            );
        }
    }
}

impl Dom for WebDom {
    fn focus(&self, node: &Self::Node) -> Result<(), DomError> {
        self.ensure_writable(node)?;
        let element = self.as_element(node)?;
        let html = element.dyn_ref::<web_sys::HtmlElement>().ok_or_else(|| {
            DomError::Unsupported(String::from("节点不是可聚焦元素（非 HtmlElement）"))
        })?;
        html.focus()
            .map_err(|error| DomError::Unsupported(format!("focus() 失败：{error:?}")))
    }

    fn blur(&self, node: &Self::Node) -> Result<(), DomError> {
        self.ensure_writable(node)?;
        let element = self.as_element(node)?;
        let html = element.dyn_ref::<web_sys::HtmlElement>().ok_or_else(|| {
            DomError::Unsupported(String::from("节点不是可失焦元素（非 HtmlElement）"))
        })?;
        html.blur()
            .map_err(|error| DomError::Unsupported(format!("blur() 失败：{error:?}")))
    }

    fn is_composing(&self) -> bool {
        self.composing.get()
    }

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
        if let Ok(object) = element.clone().dyn_into::<Object>() {
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
        // ⭐ 组合态与闭包共享（`Rc<Cell<bool>>`）：事件一到达就更新，渲染器立刻读到。
        let composing = std::rc::Rc::clone(&self.composing);
        let closure = Closure::wrap(Box::new(move |event: Event| {
            match kind {
                EventKind::CompositionStart => composing.set(true),
                EventKind::CompositionEnd => composing.set(false),
                _ => {}
            }
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
            // ⭐ **IME 组合数据**（G3）：`compositionend.data` = 本次组合出来的字符串
            //   ⇒ 上层可据此区分「组合提交」与普通 `input`。
            if matches!(
                kind,
                EventKind::CompositionEnd | EventKind::CompositionStart
            ) {
                if let Ok(data) = Reflect::get(event.as_ref(), &JsValue::from_str("data")) {
                    if let Some(text) = data.as_string() {
                        dom_event = dom_event.with_composition_data(text);
                    }
                }
            }
            if let Some(keyboard) = event.dyn_ref::<KeyboardEvent>() {
                dom_event = dom_event.with_key(keyboard.key());
                // ⭐⭐⭐ **键盘事件的修饰键**（E-169 补）。
                //
                // ## 为什么必须在这里填（2026-11-04 修）
                //
                // 此前修饰键**只在 `MouseEvent` 分支内**填（见下方）
                // ⇒ ⇒ **纯键盘事件的 `shift_key()` / `ctrl_key()` 恒为 `false`**
                // ⇒ ⇒ ⇒ 应用**无法区分 `Enter` 与 `Shift+Enter`**
                // ⇒ ⇒ ⇒⇒ r10 要求的「Enter 与 Shift+Enter 的区分」在引擎层**不可实现**
                // ⇒ ⇒ ⇒ （典型用途：输入框「Enter 提交 / Shift+Enter 换行」）
                //
                // ⇒ ⇒ 现在键盘分支**自己**填 ⇒ ⇒ 两类事件**各自**取自己的修饰键。
                dom_event = dom_event
                    .with_modifiers(
                        keyboard.ctrl_key(),
                        keyboard.shift_key(),
                        keyboard.meta_key(),
                    )
                    .with_alt_key(keyboard.alt_key());
            }
            // ⭐⭐⭐ **IME 组合期标记**（E-169 补）：「这次事件发生时是否在组合」。
            //
            // ⇒ ⇒ 中文输入法组合期里用户按 `Enter` 是「**确认候选**」⇒ ⇒ **不是**「提交」
            // ⇒ ⇒ 应用要判 `event.is_composing()` 才不会**误发**（r10 §5.2 硬要求）。
            //
            // ⚠️ 取值口径：`KeyboardEvent.isComposing` 是**事件发生那一刻**的组合态
            // ⇒ ⇒ 正是我们要的「历史事实」⇒ ⇒ 与 `Dom::is_composing()`（当前状态）互补。
            if let Some(keyboard) = event.dyn_ref::<KeyboardEvent>() {
                dom_event = dom_event.with_is_composing(keyboard.is_composing());
            }
            // ⭐ **复选框/单选的 `checked`**（G2）：`change` 事件里 `checked` 是布尔 property
            // ⇒ ⇒ 填进载荷，处理器不必回读 DOM（与 `target_value` 同一理由）。
            if let Some(element) = target.as_ref().and_then(|node| node.dyn_ref::<Element>()) {
                if let Ok(checked) = Reflect::get(element.as_ref(), &JsValue::from_str("checked")) {
                    if let Some(flag) = checked.as_bool() {
                        dom_event = dom_event.with_checked(flag);
                    }
                }
            }
            if let Some(mouse) = event.dyn_ref::<MouseEvent>() {
                dom_event = dom_event
                    .with_modifiers(mouse.ctrl_key(), mouse.shift_key(), mouse.meta_key())
                    // ⭐ **Alt 键**（G2）：此前漏了这个修饰键 ⇒ 组合键判定做不了。
                    .with_alt_key(mouse.alt_key())
                    // ⭐ **视口坐标（CSS 像素）**（G2）：与 `ElementBox` 同一坐标系
                    // ⇒ 命中测试可以直接和 `bounding_box()` 的返回值比较。
                    .with_client_position(f64::from(mouse.client_x()), f64::from(mouse.client_y()));
                // 鼠标按键：`-1` 表示「本事件没有按键」（如无按键的 `pointermove`）
                // ⇒ ⇒ 不填（`None`），**不要**强转成 255。
                let button = mouse.button();
                if button >= 0 {
                    dom_event = dom_event.with_button(button as u8);
                }
            }
            // ⭐ **指针 id**（G2）：只有 pointer 类事件带 `pointerId`
            // ⇒ ⇒ 用通用属性读取，避免为它单开 `web-sys` feature。
            if let Ok(value) = Reflect::get(event.as_ref(), &JsValue::from_str("pointerId")) {
                if let Some(id) = value.as_f64() {
                    dom_event = dom_event.with_pointer_id(id as i32);
                }
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
            let outcome = listener.node.remove_event_listener_with_callback(
                listener.kind.as_str(),
                listener.closure.as_ref().unchecked_ref(),
            );
            // ⭐⭐⭐ **诊断**（E-176）：摘除失败 ⇒ ⇒ JS 侧监听器**还挂着**
            //   ⇒ ⇒ 而 wasm 侧的 `Closure` 会被 drop ⇒ ⇒ ⇒ 之后事件派发
            //   ⇒ ⇒ ⇒ wasm-bindgen 抛「closure invoked recursively or after being dropped」。
            //   ⇒ ⇒ ⇒ **跨内核实测**：Chromium 下**从不**失败；WebKitGTK 下**会**失败。
            if let Err(error) = outcome {
                let message = format!(
                    "⭐ 监听器摘除失败（事件类型={}）：{error:?}",
                    listener.kind.as_str()
                );
                web_sys::console::error_1(&JsValue::from_str(&message));
            }
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
            let outcome = node.remove_event_listener_with_callback(
                kind.as_str(),
                listener.closure.as_ref().unchecked_ref(),
            );
            // ⭐⭐⭐ **诊断**（E-176）：同上 ⇒ ⇒ 失败必须**看得见**，不能静默。
            if let Err(error) = outcome {
                let message = format!("⭐ 监听器摘除失败（事件类型={}）：{error:?}", kind.as_str());
                web_sys::console::error_1(&JsValue::from_str(&message));
            }
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
