//! 事件委托：委托根上一套监听器，按冒泡路径自目标向祖先调用。
//!
//! 设计文档 §5.3：
//! - 委托根 = 引擎的挂载容器（所有引擎产出的 DOM 的唯一公共祖先）；
//! - 调用顺序：目标节点上的处理器先执行，然后沿祖先链向上；
//! - `stop_propagation()` ⇒ 停止向上传播；`prevent_default()` 由处理器显式调用，
//!   引擎**不自动**阻止默认行为。
//!
//! 事件对象 API 面按仓库实测用量裁剪（不照抄浏览器 API 全集）。

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::dom::EventKind;

/// 事件处理器（接收可变引用以便 `stop_propagation` / `prevent_default`）。
pub type EventHandler = Rc<dyn Fn(&mut DomEvent)>;

/// 元素上的事件处理器（可放进虚拟树的那一份）。
///
/// 与 [`EventHandler`] 的差别：它**自带 `Debug` / `PartialEq` 实现**，因而能
/// 放进 `ElementVNode`（虚拟树要可 `PartialEq`、可 `Debug`，设计文档 §6.4）。
///
/// **相等性按指针身份**（`Rc::ptr_eq`）：闭包没有结构相等可言 ⇒ 「同一个
/// 处理器」= 「同一个闭包实例」。这条语义让差分能判断「处理器换了没有」
/// （换了 ⇒ 重挂监听器）。
#[derive(Clone)]
pub struct ElementHandler {
    inner: EventHandler,
}

impl ElementHandler {
    /// 包装一个处理器闭包。
    pub fn new(handler: impl Fn(&mut DomEvent) + 'static) -> Self {
        Self {
            inner: Rc::new(handler),
        }
    }

    /// 调用处理器。
    pub fn call(&self, event: &mut DomEvent) {
        (self.inner)(event);
    }
}

impl std::fmt::Debug for ElementHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ElementHandler(..)")
    }
}

impl PartialEq for ElementHandler {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for ElementHandler {}

/// 处理器编号（注销用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HandlerId(pub usize);

/// 引擎自有事件对象。
#[derive(Debug)]
pub struct DomEvent {
    kind: EventKind,
    target_path: Vec<usize>,
    key: Option<String>,
    target_value: Option<String>,
    ctrl_key: bool,
    shift_key: bool,
    meta_key: bool,
    propagation_stopped: Cell<bool>,
    default_prevented: Cell<bool>,
}

impl DomEvent {
    /// 建一个事件对象（目标路径 = 从根到目标的子节点下标序列）。
    pub fn new(kind: EventKind, target_path: Vec<usize>) -> Self {
        Self {
            kind,
            target_path,
            key: None,
            target_value: None,
            ctrl_key: false,
            shift_key: false,
            meta_key: false,
            propagation_stopped: Cell::new(false),
            default_prevented: Cell::new(false),
        }
    }

    /// 事件类型。
    pub fn kind(&self) -> EventKind {
        self.kind
    }

    /// 目标路径（从根到目标的子节点下标序列）。
    pub fn target_path(&self) -> &[usize] {
        &self.target_path
    }

    /// 目标深度（根 = 0）。
    pub fn target_depth(&self) -> usize {
        self.target_path.len().saturating_sub(1)
    }

    /// 键盘事件的按键名。
    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }

    /// 目标元素的取值（`bind:value` 路径自动完成取值）。
    pub fn target_value(&self) -> Option<&str> {
        self.target_value.as_deref()
    }

    /// Ctrl 键是否按下。
    pub fn ctrl_key(&self) -> bool {
        self.ctrl_key
    }

    /// Shift 键是否按下。
    pub fn shift_key(&self) -> bool {
        self.shift_key
    }

    /// Meta 键是否按下。
    pub fn meta_key(&self) -> bool {
        self.meta_key
    }

    /// 阻止事件继续向上冒泡。
    pub fn stop_propagation(&self) {
        self.propagation_stopped.set(true);
    }

    /// 是否已请求阻止冒泡。
    pub fn is_propagation_stopped(&self) -> bool {
        self.propagation_stopped.get()
    }

    /// 阻止默认行为（引擎不自动调用）。
    pub fn prevent_default(&self) {
        self.default_prevented.set(true);
    }

    /// 是否已请求阻止默认行为。
    pub fn is_default_prevented(&self) -> bool {
        self.default_prevented.get()
    }

    /// 带上按键名（构造器）。
    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// 带上目标取值（构造器）。
    pub fn with_target_value(mut self, value: impl Into<String>) -> Self {
        self.target_value = Some(value.into());
        self
    }

    /// 带上修饰键状态（构造器）。
    pub fn with_modifiers(mut self, ctrl: bool, shift: bool, meta: bool) -> Self {
        self.ctrl_key = ctrl;
        self.shift_key = shift;
        self.meta_key = meta;
        self
    }
}

/// 已挂载的处理器。
struct MountedHandler {
    id: HandlerId,
    kind: EventKind,
    depth: usize,
    handler: EventHandler,
}

/// 事件委托表：委托根上按「事件类型 + 挂载深度」索引处理器。
#[derive(Default)]
pub struct EventRegistry {
    handlers: RefCell<Vec<MountedHandler>>,
    next_id: Cell<usize>,
}

impl EventRegistry {
    /// 建一个空的委托表。
    pub fn new() -> Self {
        Self::default()
    }

    /// 在指定深度挂一个处理器（深度 = 距委托根的层数，根 = 0）。
    pub fn register(&self, kind: EventKind, depth: usize, handler: EventHandler) -> HandlerId {
        let id = HandlerId(self.next_id.get());
        self.next_id.set(self.next_id.get() + 1);
        self.handlers.borrow_mut().push(MountedHandler {
            id,
            kind,
            depth,
            handler,
        });
        id
    }

    /// 注销一个处理器（幂等）。
    pub fn unregister(&self, id: HandlerId) {
        self.handlers
            .borrow_mut()
            .retain(|mounted| mounted.id != id);
    }

    /// 已挂载处理器数。
    pub fn len(&self) -> usize {
        self.handlers.borrow().len()
    }

    /// 委托表是否为空。
    pub fn is_empty(&self) -> bool {
        self.handlers.borrow().is_empty()
    }

    /// 派发事件：自目标向祖先链调用处理器，返回实际调用次数。
    ///
    /// 调用顺序 = 深度从大到小（目标先、祖先后）；任一处理器调用
    /// `stop_propagation()` 即停止向上传播。
    pub fn dispatch(&self, event: &mut DomEvent) -> usize {
        let target_depth = event.target_depth();
        let mut candidates: Vec<(usize, HandlerId, EventHandler)> = self
            .handlers
            .borrow()
            .iter()
            .filter(|mounted| mounted.kind == event.kind && mounted.depth <= target_depth)
            .map(|mounted| (mounted.depth, mounted.id, mounted.handler.clone()))
            .collect();
        // 深度降序 ⇒ 目标先执行；同深度按挂载顺序。
        candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.0));

        let mut called = 0;
        for (_, _, handler) in candidates {
            handler(event);
            called += 1;
            if event.is_propagation_stopped() {
                break;
            }
        }
        called
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn handlers_run_from_target_up_to_ancestors() {
        let registry = EventRegistry::new();
        let order = Rc::new(RefCell::new(Vec::<&'static str>::new()));

        let o = order.clone();
        registry.register(
            EventKind::Click,
            0,
            Rc::new(move |_event: &mut DomEvent| o.borrow_mut().push("root")),
        );
        let o = order.clone();
        registry.register(
            EventKind::Click,
            2,
            Rc::new(move |_event: &mut DomEvent| o.borrow_mut().push("target")),
        );

        let mut event = DomEvent::new(EventKind::Click, vec![0, 1, 2]);
        assert_eq!(registry.dispatch(&mut event), 2);
        assert_eq!(&*order.borrow(), &["target", "root"]);
    }

    #[test]
    fn stop_propagation_halts_the_bubble_chain() {
        let registry = EventRegistry::new();
        let order = Rc::new(RefCell::new(Vec::<&'static str>::new()));

        let o = order.clone();
        registry.register(
            EventKind::Click,
            2,
            Rc::new(move |event: &mut DomEvent| {
                o.borrow_mut().push("target");
                event.stop_propagation();
            }),
        );
        let o = order.clone();
        registry.register(
            EventKind::Click,
            0,
            Rc::new(move |_event: &mut DomEvent| o.borrow_mut().push("root")),
        );

        let mut event = DomEvent::new(EventKind::Click, vec![0, 1, 2]);
        assert_eq!(registry.dispatch(&mut event), 1);
        assert_eq!(&*order.borrow(), &["target"]);
        assert!(event.is_propagation_stopped());
    }

    #[test]
    fn unregister_removes_the_handler() {
        let registry = EventRegistry::new();
        let id = registry.register(EventKind::Input, 0, Rc::new(|_event: &mut DomEvent| {}));
        assert_eq!(registry.len(), 1);
        registry.unregister(id);
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }

    #[test]
    fn event_carries_key_and_value_payloads() {
        let event = DomEvent::new(EventKind::KeyDown, vec![0])
            .with_key("k")
            .with_target_value("文本")
            .with_modifiers(true, false, false);
        assert_eq!(event.key(), Some("k"));
        assert_eq!(event.target_value(), Some("文本"));
        assert!(event.ctrl_key());
        assert!(!event.shift_key());
        assert_eq!(event.target_depth(), 0);
    }
}
