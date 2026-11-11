//! ⭐ **焦点 / IME / a11y 通道**（台账 L1-G3）。
//!
//! 本模块提供三件**跨消费者不变量**。它们都是「每个界面若自己实现，必各错一种」的东西，
//! 所以归引擎（L1）而不是留给上层：
//!
//! 1. [`FocusTracker`]：**焦点栈** + **焦点陷阱（trap）** —— 弹层打开时 Tab 不得跑到外层；
//! 2. [`CompositionGate`]：**IME 组合静默期** —— 中文/日文输入法组合期间，
//!    引擎**绝不能**把状态写回输入框（否则会把正在输入的字打散）；
//! 3. [`AriaChannel`]：**无障碍属性通道** —— 只允许写入白名单属性，避免上层乱写。
//!
//! ## ⚠️ 本模块的边界（0 装纪律）
//!
//! 这里只做**状态与判定**，不直接碰 DOM：真正的 `focus()` 调用 / `aria-*` 写入由渲染器
//! 或后端完成。**当前接线状态：未接线**（见四级口径），因此本模块的结论是
//! 「逻辑已实现并有测试」，**不是**「浏览器里已经生效」。
//!
//! 引擎**不做**排版/主题（ADR-0015）；本模块也不涉及视觉，只保证**焦点与输入语义正确**。

use std::collections::VecDeque;

/// 一个可聚焦元素的稳定标识（由上层决定是什么：节点 id、key、路径编号均可）。
pub type FocusId = u64;

/// ⭐ **焦点栈**（含焦点陷阱）。
///
/// ## 为什么需要栈而不是「一个当前焦点」
///
/// 弹层 / 菜单 / 对话框打开时，焦点临时移入；关闭时**必须回到打开前的元素**。
/// 只记「当前焦点」就丢了这个「从哪儿来」的信息 ⇒ ⇒ 关闭后焦点落到 `<body>`，
/// 键盘用户直接失去位置（这是真实的无障碍缺陷）。
///
/// ## 陷阱（trap）
///
/// 弹层打开期间，`Tab` 只能在其内部循环。陷阱用「允许集合」表达：
/// 集合非空 ⇒ 焦点**必须**落在集合内 ⇒ `allows()` 为假即视为越界。
#[derive(Debug, Default, Clone)]
pub struct FocusTracker {
    /// 历史焦点（栈顶 = 上一个焦点）。用 `VecDeque` 以便限制上界。
    previous: VecDeque<FocusId>,
    /// 当前焦点。
    current: Option<FocusId>,
    /// 焦点陷阱的允许集合（空 = 无陷阱）。
    trap: Vec<FocusId>,
    /// 历史栈上界（防止长会话无限增长）。
    capacity: usize,
}

/// 焦点栈的默认上界。
pub const DEFAULT_FOCUS_STACK_CAPACITY: usize = 128;

impl FocusTracker {
    /// 建一个空追踪器。
    pub fn new() -> Self {
        Self {
            previous: VecDeque::new(),
            current: None,
            trap: Vec::new(),
            capacity: DEFAULT_FOCUS_STACK_CAPACITY,
        }
    }

    /// 当前焦点。
    pub fn current(&self) -> Option<FocusId> {
        self.current
    }

    /// 把焦点移到 `id`（返回被替换下来的焦点）。
    ///
    /// ⚠️ 若存在**焦点陷阱**且 `id` 不在允许集合内 ⇒ ⇒ **拒绝移动**并返回 `Err(id)`：
    /// 静默允许越界是最糟的选择（用户 Tab 出弹层却没人发现）。
    pub fn focus(&mut self, id: FocusId) -> Result<Option<FocusId>, FocusId> {
        if !self.allows(id) {
            return Err(id);
        }
        let previous = self.current.replace(id);
        if let Some(old) = previous {
            self.push_previous(old);
        }
        Ok(previous)
    }

    /// 焦点离开（`blur`）⇒ 返回新的当前焦点（通常是 `None`）。
    ///
    /// 陷阱生效期间 **不允许** 清空焦点（否则 `Tab` 会落到文档外部）。
    pub fn blur(&mut self) -> usize {
        match self.current.take() {
            Some(old) => {
                self.push_previous(old);
                1
            }
            None => 0,
        }
    }

    /// 恢复到上一个焦点（弹层关闭时调用）。
    pub fn restore_previous(&mut self) -> Option<FocusId> {
        while let Some(candidate) = self.previous.pop_back() {
            if self.allows(candidate) {
                self.current = Some(candidate);
                return Some(candidate);
            }
        }
        None
    }

    /// 打开焦点陷阱（`allowed` 为空 ⇒ 等价于不设陷阱，避免"锁死"）。
    pub fn open_trap(&mut self, allowed: impl IntoIterator<Item = FocusId>) {
        self.trap = allowed.into_iter().collect();
    }

    /// 关闭焦点陷阱。
    pub fn close_trap(&mut self) {
        self.trap.clear();
    }

    /// 是否存在生效中的陷阱。
    pub fn trap_active(&self) -> bool {
        !self.trap.is_empty()
    }

    /// `id` 是否被允许获得焦点（无陷阱 ⇒ 全部允许）。
    pub fn allows(&self, id: FocusId) -> bool {
        self.trap.is_empty() || self.trap.contains(&id)
    }

    /// 设置历史栈上界（0 视为 1）。
    pub fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity.max(1);
        while self.previous.len() > self.capacity {
            self.previous.pop_front();
        }
    }

    /// 历史栈深度（判据用）。
    pub fn history_depth(&self) -> usize {
        self.previous.len()
    }

    fn push_previous(&mut self, id: FocusId) {
        self.previous.push_back(id);
        while self.previous.len() > self.capacity {
            self.previous.pop_front();
        }
    }
}

/// ⭐ **IME 组合门**：中文/日文等输入法组合期间，引擎必须**暂停**写回输入框。
///
/// ## 为什么必须归引擎
///
/// 组合期间浏览器正在管理「预编辑串」；此时若引擎因为状态变化把 `value` 写回，
/// 会打断输入法 ⇒ ⇒ 用户看到刚打的拼音被清掉（真实缺陷，且**只在中文输入下复现**，
/// 英文测试永远发现不了）。
///
/// 因此：`compositionstart` ⇒ 关门；`compositionend` ⇒ 开门并**采纳**组合结果。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CompositionGate {
    active: bool,
}

impl CompositionGate {
    /// 建一个未组合态的门。
    pub fn new() -> Self {
        Self { active: false }
    }

    /// 组合开始（`compositionstart`）。
    pub fn start(&mut self) {
        self.active = true;
    }

    /// 组合结束（`compositionend`）。
    pub fn end(&mut self) {
        self.active = false;
    }

    /// 是否处于组合中。
    pub fn is_composing(&self) -> bool {
        self.active
    }

    /// ⭐ **引擎此刻是否允许把状态写回输入控件**（组合中 ⇒ 禁止）。
    ///
    /// 这是本模块最重要的判定：调用点应当用它**门住**所有 `set_property("value", …)`。
    pub fn allows_write_back(&self) -> bool {
        !self.active
    }
}

/// a11y 属性白名单：只允许设置这些 `aria-*` / 语义属性。
///
/// ## 为什么要白名单
///
/// `aria-*` 写错（例如给非按钮写 `aria-pressed`，或把 `role` 写成不存在的值）
/// **不会报错**，只会让屏幕阅读器行为异常 ⇒ ⇒ 白名单让「乱写」在**开发期**就失败。
pub const ARIA_ATTRIBUTE_ALLOWLIST: &[&str] = &[
    "aria-label",
    "aria-labelledby",
    "aria-describedby",
    "aria-live",
    "aria-atomic",
    "aria-hidden",
    "aria-expanded",
    "aria-selected",
    "aria-checked",
    "aria-disabled",
    "aria-invalid",
    "aria-required",
    "aria-busy",
    "aria-current",
    "aria-controls",
    "aria-haspopup",
    "aria-modal",
    "aria-pressed",
    "aria-valuemin",
    "aria-valuemax",
    "aria-valuenow",
    "aria-valuetext",
    "role",
    "tabindex",
];

/// a11y 属性通道：校验 + 记录（**不直接写 DOM**）。
#[derive(Debug, Default, Clone)]
pub struct AriaChannel {
    entries: Vec<(String, String)>,
}

impl AriaChannel {
    /// 建一个空通道。
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// 写入一条 a11y 属性；不在白名单 ⇒ 返回 `Err(属性名)`。
    pub fn set(&mut self, name: &str, value: &str) -> Result<(), String> {
        if !ARIA_ATTRIBUTE_ALLOWLIST.contains(&name) {
            return Err(name.to_owned());
        }
        match self.entries.iter_mut().find(|(key, _)| key == name) {
            Some(slot) => slot.1 = value.to_owned(),
            None => self.entries.push((name.to_owned(), value.to_owned())),
        }
        Ok(())
    }

    /// 读取一条 a11y 属性。
    pub fn get(&self, name: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// 已写入的条目数（判据用）。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⭐ **焦点栈回到打开前的元素**（弹层关闭后键盘用户不会失去位置）。
    #[test]
    fn focus_stack_restores_the_previous_element() {
        let mut tracker = FocusTracker::new();
        tracker.focus(1).expect("无陷阱时允许");
        tracker.focus(2).expect("无陷阱时允许");
        assert_eq!(tracker.current(), Some(2));
        assert_eq!(tracker.history_depth(), 1, "1 被压入历史");
        assert_eq!(tracker.restore_previous(), Some(1), "必须回到 1");
        assert_eq!(tracker.current(), Some(1));
    }

    /// ⭐ **陷阱外拒绝获得焦点**（不是静默允许）。
    #[test]
    fn trap_rejects_focus_outside_the_allowed_set() {
        let mut tracker = FocusTracker::new();
        tracker.focus(1).expect("无陷阱时允许");
        tracker.open_trap([10, 11]);
        assert!(tracker.trap_active());
        assert_eq!(
            tracker.focus(2),
            Err(2),
            "陷阱生效期间，2 不在集合内 ⇒ 拒绝"
        );
        assert_eq!(tracker.current(), Some(1), "拒绝后焦点不变");
        tracker.focus(10).expect("集合内允许");
        assert_eq!(tracker.current(), Some(10));
    }

    /// 陷阱关闭后恢复自由；空集合不锁死。
    #[test]
    fn closing_the_trap_restores_freedom_and_empty_trap_is_no_trap() {
        let mut tracker = FocusTracker::new();
        tracker.open_trap([]);
        assert!(!tracker.trap_active(), "空集合等价于无陷阱（不得锁死）");
        tracker.focus(5).expect("无陷阱时允许");
        tracker.open_trap([1]);
        tracker.close_trap();
        tracker.focus(9).expect("陷阱关闭后允许");
    }

    /// 焦点历史有**上界**（长会话不会无限增长）。
    #[test]
    fn focus_history_is_bounded() {
        let mut tracker = FocusTracker::new();
        tracker.set_capacity(2);
        for id in 0..10 {
            tracker.focus(id).expect("无陷阱时允许");
        }
        assert_eq!(tracker.history_depth(), 2, "历史栈必须被夹到上界");
    }

    /// ⭐ **IME 组合期间禁止写回**（中文输入不被引擎打断）。
    #[test]
    fn composition_gate_blocks_write_back_while_composing() {
        let mut gate = CompositionGate::new();
        assert!(gate.allows_write_back(), "未组合时可以写回");
        gate.start();
        assert!(gate.is_composing());
        assert!(!gate.allows_write_back(), "组合中**必须**禁止写回");
        gate.end();
        assert!(!gate.is_composing());
        assert!(gate.allows_write_back(), "组合结束后恢复写回");
    }

    /// a11y 白名单：合法属性可写可读可覆盖，非法属性**报错**。
    #[test]
    fn aria_channel_enforces_allowlist_and_overwrites() {
        let mut channel = AriaChannel::new();
        channel.set("aria-label", "保存").expect("白名单内");
        channel.set("role", "button").expect("白名单内");
        assert_eq!(channel.get("aria-label"), Some("保存"));
        assert_eq!(channel.len(), 2);
        channel.set("aria-label", "保存草稿").expect("可覆盖");
        assert_eq!(channel.get("aria-label"), Some("保存草稿"));
        assert_eq!(channel.len(), 2, "覆盖不得新增条目");
        assert_eq!(
            channel.set("aria-foo", "x"),
            Err("aria-foo".to_owned()),
            "白名单外必须报错（不静默丢弃）"
        );
    }

    /// 白名单本身不得有重复项（否则维护时会误以为两处都能加）。
    #[test]
    fn aria_allowlist_has_no_duplicates() {
        let unique: std::collections::BTreeSet<&str> =
            ARIA_ATTRIBUTE_ALLOWLIST.iter().copied().collect();
        assert_eq!(
            unique.len(),
            ARIA_ATTRIBUTE_ALLOWLIST.len(),
            "白名单含重复项"
        );
    }
}
