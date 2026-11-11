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

/// 目标在「**可见**子节点」序列里的下标（不可见的节点**跳过**）。
///
/// ## 为什么要有这个函数（2026-10-25 修）
///
/// 浏览器的子节点里有**注释节点** ⇒ 若把它们算进下标 ⇒ ⇒ 内存后端
/// （只有元素与文本）的下标空间与浏览器**不一致** ⇒ ⇒ 两端的
/// [`DomEvent::target_path`] **不可比** ⇒ ⇒ 委托路由（按路径定位）会错。
///
/// ⇒ ⇒ 统一：**只数元素与文本**（注释节点跳过）。
///
/// ## 参数
///
/// - `visible[i]` = 第 i 个**原始**子节点是否可见（元素 / 文本）；
/// - `target` = 目标是第几个**原始**子节点。
///
/// 返回目标在**可见**序列里的下标。
///
/// ⚠️ 若目标**本身不可见**（理论上不会发生：事件目标总是元素 / 文本）⇒
/// ⇒ 返回它**前面**可见子节点的个数（= 它「本该在」的下标）。
pub fn visible_child_index(visible: &[bool], target: usize) -> usize {
    let mut index = 0usize;
    for (position, is_visible) in visible.iter().enumerate() {
        if position == target {
            return index;
        }
        if *is_visible {
            index += 1;
        }
    }
    index
}

/// 事件处理器编号（注销用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HandlerId(pub usize);

/// 引擎自有事件对象。
#[derive(Debug)]
pub struct DomEvent {
    kind: EventKind,
    target_path: Vec<usize>,
    /// 失焦事件的**落点**路径（`relatedTarget`；`None` = 没有落点 / 不是失焦）。
    ///
    /// ## 为什么需要（2026-10-25 加）
    ///
    /// `focusout` 在浏览器里**也会**在「焦点从 A 移到 B（同一个组件内部）」
    /// 时冒泡 ⇒ ⇒ 只看 `focusout` **分不清**「真失焦」与「内部挪焦点」⇒
    /// ⇒ 表单的「离开字段 ⇒ 提交草稿」逻辑会在**内部挪焦点**时误触发。
    ///
    /// ⇒ 浏览器的 `FocusEvent.relatedTarget` 就是「焦点落到哪去了」⇒
    /// ⇒ 引擎把它折成**路径**（与 `target_path` 同一口径）⇒
    /// ⇒ 处理器可以判断「落点是不是还在自己内部」。
    related_target_path: Option<Vec<usize>>,
    key: Option<String>,
    target_value: Option<String>,
    /// 复选框/单选的 `checked`（`Change` 事件专用；`None` = 该事件没有这个载荷）。
    checked: Option<bool>,
    ctrl_key: bool,
    shift_key: bool,
    meta_key: bool,
    /// Alt 键（组合键判定必需；此前 **漏了** 这个修饰键）。
    alt_key: bool,
    /// 指针/鼠标的**视口坐标（CSS 像素）**，与 `ElementBox` 同一坐标系。
    client_x: Option<f64>,
    /// 见 [`Self::client_x`]。
    client_y: Option<f64>,
    /// 鼠标按键（0=主键 1=中键 2=次键；仅指针类事件有）。
    button: Option<u8>,
    /// 指针 id（多点触控时区分手指；仅 pointer 类事件有）。
    pointer_id: Option<i32>,
    /// IME 组合数据（`compositionend` 的 `data`；其它事件为 `None`）。
    composition_data: Option<String>,
    /// ⭐⭐⭐ **事件发生时是否处于 IME 组合期**（E-169 补）。
    ///
    /// ## 为什么必须有这个载荷（2026-11-04 补）
    ///
    /// 中文输入法在组合期里，用户按 `Enter` 是「**确认候选**」⇒ ⇒ **不是**「提交」；
    /// 而应用收到 `keydown(Enter)` 时，若**无法区分**这两种 Enter
    /// ⇒ ⇒ 就会在「选词确认」时**误发一次提交**（把半成品拼音发出去）。
    ///
    /// ⇒ ⇒ 引擎此前只做「**组合期不打断输入**」（`Dom::is_composing()` ⇒
    /// ⇒ 组合期不写回 `value`/`checked`，延后补写）⇒ ⇒ **输入不会被冲掉** ✓
    /// ⇒ ⇒ 但「**不误发送**」这一半**做不了** ⇒ ⇒ 因为**事件里没有这个信息**
    /// ⇒ ⇒ ⇒ 应用**无从判断** ⇒ ⇒⇒ r10 要求的「中文 IME 合成期不误发送」不成立。
    ///
    /// ⇒ ⇒ 现在把它放进**事件载荷** ⇒ ⇒ 应用可以直接判
    /// ⇒ ⇒ `event.is_composing()` ⇒ ⇒ 「这次 Enter 是选词还是提交」。
    ///
    /// ⚠️ 口径：`false` = **不在**组合期；`true` = **正在**组合。
    /// ⚠️ 非键盘事件（如 `pointerdown`）通常也是 `false`。
    is_composing: bool,
    propagation_stopped: Cell<bool>,
    default_prevented: Cell<bool>,
}

impl DomEvent {
    /// 建一个事件对象（目标路径 = 从根到目标的子节点下标序列）。
    pub fn new(kind: EventKind, target_path: Vec<usize>) -> Self {
        Self {
            kind,
            target_path,
            related_target_path: None,
            key: None,
            target_value: None,
            checked: None,
            ctrl_key: false,
            shift_key: false,
            meta_key: false,
            alt_key: false,
            client_x: None,
            client_y: None,
            button: None,
            pointer_id: None,
            composition_data: None,
            is_composing: false,
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

    /// 失焦事件的**落点**路径（`relatedTarget`）。
    ///
    /// `None` = 没有落点（不是失焦事件 / 焦点落到了**文档之外**）。
    ///
    /// 判「真失焦」的口径：落点**不在**本元素内部 ⇒ 真失焦。
    pub fn related_target_path(&self) -> Option<&[usize]> {
        self.related_target_path.as_deref()
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
    /// 复选框/单选的 `checked`（没有该载荷 ⇒ `None`，**不**用 `false` 冒充）。
    pub fn checked(&self) -> Option<bool> {
        self.checked
    }

    /// 指针/鼠标的**视口坐标**（CSS 像素）；没有该载荷 ⇒ `None`。
    pub fn client_position(&self) -> Option<(f64, f64)> {
        match (self.client_x, self.client_y) {
            (Some(x), Some(y)) => Some((x, y)),
            _ => None,
        }
    }

    /// 鼠标按键（0=主键 1=中键 2=次键）。
    pub fn button(&self) -> Option<u8> {
        self.button
    }

    /// 指针 id（多点触控）。
    pub fn pointer_id(&self) -> Option<i32> {
        self.pointer_id
    }

    /// Alt 键是否按下。
    pub fn alt_key(&self) -> bool {
        self.alt_key
    }

    /// 设置 `checked` 载荷。
    pub fn with_checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    /// 设置指针/鼠标的视口坐标（CSS 像素）。
    pub fn with_client_position(mut self, x: f64, y: f64) -> Self {
        self.client_x = Some(x);
        self.client_y = Some(y);
        self
    }

    /// 设置鼠标按键。
    pub fn with_button(mut self, button: u8) -> Self {
        self.button = Some(button);
        self
    }

    /// 设置指针 id。
    pub fn with_pointer_id(mut self, pointer_id: i32) -> Self {
        self.pointer_id = Some(pointer_id);
        self
    }

    /// IME 组合数据（`compositionend.data`）；没有该载荷 ⇒ `None`。
    pub fn composition_data(&self) -> Option<&str> {
        self.composition_data.as_deref()
    }

    /// 设置 IME 组合数据。
    pub fn with_composition_data(mut self, data: impl Into<String>) -> Self {
        self.composition_data = Some(data.into());
        self
    }

    /// ⭐⭐⭐ **事件发生时是否处于 IME 组合期**（`true` = 正在组合）。
    ///
    /// ## 怎么用（中文输入法的关键判据）
    ///
    /// ```text
    /// // ⭐ 组合期的 Enter 是「确认候选」，**不是**「提交」
    /// if event.key() == Some("Enter") && !event.is_composing() {
    ///     submit();
    /// }
    /// ```
    ///
    /// ⇒ ⇒ 不加这个判断 ⇒ ⇒ **选词时会误提交**（把半成品拼音发出去）。
    ///
    /// ⚠️ 与 [`crate::dom::Dom::is_composing`] 的区别：后者问的是「**现在**是不是在组合」
    /// （用于写回门控）⇒ ⇒ 本方法是「**这个事件**发生时是不是在组合」
    /// ⇒ ⇒ **两者口径不同，都要看**（事件是历史事实，`is_composing()` 是当前状态）。
    pub fn is_composing(&self) -> bool {
        self.is_composing
    }

    /// 设置 IME 组合期标记（构造器）。
    pub fn with_is_composing(mut self, composing: bool) -> Self {
        self.is_composing = composing;
        self
    }

    /// 设置 Alt 键状态。
    pub fn with_alt_key(mut self, pressed: bool) -> Self {
        self.alt_key = pressed;
        self
    }

    /// 设置事件目标取值（`input` / `change` 的控件当前值）。
    pub fn with_target_value(mut self, value: impl Into<String>) -> Self {
        self.target_value = Some(value.into());
        self
    }

    /// 带上失焦落点的路径（构造器）。
    pub fn with_related_target_path(mut self, path: Vec<usize>) -> Self {
        self.related_target_path = Some(path);
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

    /// ⭐ **`all()` 与 `index/from_index` 必须一一对应**。
    ///
    /// 为什么单独一条：上一版测试写的是 `names.len() == all().len()`
    /// ⇒ ⇒ 若有人**从 `all()` 里漏掉一个变体**，两侧同时变短 ⇒ ⇒ **测试照样通过**
    /// （判据自身的缺陷）。改成 `all()` 由 `from_index` 枚举得出 + 本测试双射自检，
    /// 漏项与提前截断都会红。
    /// **编译期护栏**：`match` 必须**穷尽**全部 `EventKind` 变体。
    ///
    /// 新增变体却忘了同步 `index()` / `from_index()` / `all()` ⇒ ⇒ 本函数**编译失败**
    /// ⇒ ⇒ 比任何运行时测试都早、都硬。
    #[allow(dead_code)]
    fn exhaustive_variant_guard(kind: EventKind) {
        match kind {
            EventKind::Click
            | EventKind::DoubleClick
            | EventKind::KeyDown
            | EventKind::KeyUp
            | EventKind::Input
            | EventKind::Change
            | EventKind::FocusIn
            | EventKind::FocusOut
            | EventKind::Submit
            | EventKind::PointerDown
            | EventKind::PointerUp
            | EventKind::PointerMove
            | EventKind::PointerEnter
            | EventKind::PointerLeave
            | EventKind::Scroll
            | EventKind::Wheel
            | EventKind::ContextMenu
            | EventKind::CompositionStart
            | EventKind::CompositionEnd => {}
        }
    }

    /// ⭐⭐⭐ **IME 组合期标记**在事件载荷里（E-169）。
    ///
    /// ## 它锁住什么
    ///
    /// 中文输入法组合期里用户按 `Enter` 是「**确认候选**」⇒ ⇒ **不是**「提交」
    /// ⇒ ⇒ 应用必须能判 ⇒ ⇒ 载荷里必须有「**这个事件发生时**是否在组合」。
    ///
    /// ⇒ ⇒ 没有这个载荷 ⇒ ⇒ 应用**无从判断** ⇒ ⇒ **选词时误提交**。
    #[test]
    fn composing_flag_is_carried_in_the_event_payload() {
        let event = DomEvent::new(EventKind::KeyDown, vec![0, 0])
            .with_key("Enter")
            .with_is_composing(true);
        assert!(
            event.is_composing(),
            "⭐ 组合期标记必须能进事件载荷（否则 IME 选词会误提交）"
        );
        // ⭐ 默认**不是**组合期（不能默认 true ⇒ 那会让所有 Enter 都不提交）
        let plain = DomEvent::new(EventKind::KeyDown, vec![0, 0]).with_key("Enter");
        assert!(
            !plain.is_composing(),
            "⭐ 默认必须是「不在组合期」（默认 true ⇒ 提交功能会全废）"
        );
        // ⭐ 可显式置否
        let cleared = DomEvent::new(EventKind::KeyDown, vec![0, 0])
            .with_key("Enter")
            .with_is_composing(true)
            .with_is_composing(false);
        assert!(!cleared.is_composing(), "⭐ 组合期标记必须可显式置否");
    }

    /// ⭐⭐⭐ **键盘事件的修饰键**必须能被填进载荷（E-169）。
    ///
    /// ## 它锁住什么
    ///
    /// 旧口径下修饰键**只在鼠标分支填** ⇒ ⇒ 键盘事件的 `shift_key()` 恒 `false`
    /// ⇒ ⇒ **Enter 与 Shift+Enter 不可区分** ⇒ ⇒ 「Enter 提交 / Shift+Enter 换行」做不了。
    #[test]
    fn keyboard_modifiers_are_carried_in_the_event_payload() {
        let event = DomEvent::new(EventKind::KeyDown, vec![0])
            .with_key("Enter")
            .with_modifiers(false, true, false)
            .with_alt_key(false);
        assert!(
            event.shift_key(),
            "⭐ 键盘事件的 shift 必须能被填进载荷（否则 Enter/Shift+Enter 不可区分）"
        );
        assert!(!event.ctrl_key(), "⭐ ctrl 必须独立于 shift");
        assert!(!event.meta_key(), "⭐ meta 必须独立于 shift");
        assert!(!event.alt_key(), "⭐ alt 必须独立于 shift");

        // ⭐ **Shift+Enter** 与 **Enter** 必须能区分开
        let plain_enter = DomEvent::new(EventKind::KeyDown, vec![0])
            .with_key("Enter")
            .with_modifiers(false, false, false)
            .with_alt_key(false);
        assert_ne!(
            plain_enter.shift_key(),
            event.shift_key(),
            "⭐ Enter 与 Shift+Enter 必须可区分"
        );
    }

    #[test]
    fn event_kind_enumeration_is_bijective() {
        for kind in EventKind::all() {
            assert_eq!(
                EventKind::from_index(kind.index()),
                Some(kind),
                "index/from_index 必须互逆：{kind:?}"
            );
        }
        assert_eq!(
            EventKind::from_index(EventKind::all().len()),
            None,
            "`all()` 必须恰好枚举到最后一个变体（不得提前截断或漏项）"
        );
        // 变体总数：新增变体时这里与上面的 `exhaustive_variant_guard` **同时**要求更新
        // ⇒ ⇒ 不可能「加了变体但没进 all()」而无人发现。
        assert_eq!(
            EventKind::all().len(),
            19,
            "变体总数须与 exhaustive_variant_guard 一致"
        );
    }

    // ── G2 事件面与载荷（台账 L1-G2）─────────────────────────────────────

    /// **事件名唯一且非空**：新增 `EventKind` 变体却忘了映射时，这里会立刻红。
    #[test]
    fn every_event_kind_has_a_unique_lowercase_name() {
        use std::collections::BTreeSet;
        let mut names = BTreeSet::new();
        for kind in EventKind::all() {
            let name = kind.as_str();
            assert!(!name.is_empty(), "事件名不得为空：{kind:?}");
            assert_eq!(
                name,
                name.to_ascii_lowercase(),
                "事件名必须小写（与 DOM 事件名同口径）：{kind:?} ⇒ {name}"
            );
            assert!(names.insert(name), "事件名重复：{name}");
        }
        assert_eq!(
            names.len(),
            EventKind::all().len(),
            "`all()` 必须覆盖全部变体"
        );
    }

    /// ⭐ **`scroll` 不冒泡** ⇒ 不得挂到委托根（挂了会静默收不到事件）。
    #[test]
    fn scroll_is_not_delegatable_to_root() {
        assert!(!EventKind::Scroll.bubbles_to_delegation_root());
        for kind in EventKind::all() {
            if kind != EventKind::Scroll {
                assert!(
                    kind.bubbles_to_delegation_root(),
                    "{kind:?} 应当可以委托到根"
                );
            }
        }
    }

    /// **载荷构造器与访问器一一对应**（含此前缺失的 `alt` / 坐标 / 按钮 / 指针 id / checked）。
    #[test]
    fn event_payload_builders_round_trip() {
        let event = DomEvent::new(EventKind::Change, vec![1, 2])
            .with_target_value("hello")
            .with_checked(true)
            .with_client_position(12.5, 34.25)
            .with_button(2)
            .with_pointer_id(7)
            .with_alt_key(true);
        assert_eq!(event.kind(), EventKind::Change);
        assert_eq!(event.target_path(), &[1, 2]);
        assert_eq!(event.target_value(), Some("hello"));
        assert_eq!(event.checked(), Some(true), "复选框 checked 必须可读");
        assert_eq!(event.client_position(), Some((12.5, 34.25)));
        assert_eq!(event.button(), Some(2));
        assert_eq!(event.pointer_id(), Some(7));
        assert!(event.alt_key(), "Alt 修饰键必须可读");
    }

    /// ⭐ **没有载荷就必须是 `None`**，不能用 `false` / `0` **冒充**
    /// （0 装纪律：不知道 ≠ 知道是 false）。
    #[test]
    fn absent_payload_is_none_not_a_default_value() {
        let event = DomEvent::new(EventKind::Click, vec![0]);
        assert_eq!(event.checked(), None, "Click 没有 checked 载荷 ⇒ None");
        assert_eq!(event.client_position(), None, "未填坐标 ⇒ None");
        assert_eq!(event.button(), None);
        assert_eq!(event.pointer_id(), None);
        assert!(!event.alt_key(), "未按 Alt ⇒ false（布尔无三态）");
    }
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

    /// **失焦落点**（`relatedTarget`）⇒ 判「真失焦」用。
    #[test]
    fn event_carries_the_related_target_path() {
        let event =
            DomEvent::new(EventKind::FocusOut, vec![0, 1]).with_related_target_path(vec![0, 2]);
        assert_eq!(event.related_target_path(), Some(&[0, 2][..]));
        assert_eq!(event.target_depth(), 1);

        let without = DomEvent::new(EventKind::FocusOut, vec![0]);
        assert_eq!(
            without.related_target_path(),
            None,
            "没有落点 ⇒ `None`（分不清真失焦 / 内部挪焦点）"
        );
    }

    // ── 目标路径：只数**可见**子节点（元素 / 文本） ─────────────────────────

    /// **注释节点跳过** ⇒ 与内存后端同一口径。
    #[test]
    fn the_visible_child_index_skips_invisible_children() {
        // 原始子节点：div（可见）、注释（不可见）、span（可见）。
        let visible = [true, false, true];
        assert_eq!(visible_child_index(&visible, 0), 0, "第一个可见子节点");
        assert_eq!(
            visible_child_index(&visible, 2),
            1,
            "注释**跳过** ⇒ span 的下标是 1（不是 2）"
        );
        assert_eq!(
            visible_child_index(&visible, 1),
            1,
            "目标本身不可见 ⇒ 返回它前面可见子节点的个数"
        );
    }

    /// **没有注释时**下标与原始下标一致（不误伤）。
    #[test]
    fn the_visible_child_index_matches_the_raw_index_without_comments() {
        let visible = [true, true, true];
        assert_eq!(visible_child_index(&visible, 0), 0);
        assert_eq!(visible_child_index(&visible, 1), 1);
        assert_eq!(visible_child_index(&visible, 2), 2);
    }
}
