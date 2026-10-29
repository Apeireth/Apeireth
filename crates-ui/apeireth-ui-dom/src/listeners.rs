//! 事件监听器登记表：按 **(节点, 事件类型)** 索引。
//!
//! ## 为什么要有这一层（2026-10-25 修）
//!
//! 修前，两个后端都把监听器存在**线性表**里：
//!
//! - 内存后端：每个节点的记录里一个 `Vec<(EventKind, ElementHandler)>`；
//! - 真实后端：后端里一个 `Vec<WebListener>`，**每次摘挂都 drain 整个 Vec**。
//!
//! ⇒ 一次重渲染要给 N 个元素重挂 ⇒ 每个元素都扫一遍 N 条 ⇒ **O(N²)**。
//!
//! ## 本模块的口径
//!
//! - **按 kind 增量替换**：换掉「某节点的某个事件类型」的监听器是 **O(1)**；
//! - **摘某节点的部**：是 **O(该节点的事件类型数)**（不是 O(全表)）；
//! - **同元素多 kind 互不误伤**：表里按 (节点, kind) 分槽 ⇒ 摘 `click` 不会
//!   带走 `keydown`（这是既有契约，**不许破坏**）。
//!
//! ## 为什么节点键要是可哈希的
//!
//! 真实 DOM 后端的节点句柄（`web_sys::Node`）**不可哈希** ⇒ 后端给每个节点
//! 分配一个**引擎内编号**（弱引用映射，节点被回收后编号自动消失）⇒
//! ⇒ 表的键 = (编号, 事件类型) ⇒ 两个后端共用同一张表的实现。

use std::collections::HashMap;
use std::hash::Hash;

use crate::dom::EventKind;

/// 监听器登记表：节点 → (事件类型 → 监听器)。
///
/// ## 不变量
///
/// - 同一个 (节点, 事件类型) **至多一条**记录（挂第二个 ⇒ 覆盖第一个，
///   与 [`crate::vnode::ElementVNode::handler`] 的覆盖语义一致）；
/// - 摘掉某节点的**一个** kind **不许**影响该节点的其它 kind。
#[derive(Debug, Clone, Default)]
pub struct ListenerTable<K, V> {
    by_node: HashMap<K, HashMap<EventKind, V>>,
}

impl<K: Eq + Hash, V> ListenerTable<K, V> {
    /// 建一张空表。
    pub fn new() -> Self {
        Self {
            by_node: HashMap::new(),
        }
    }

    /// 挂一条监听器（**覆盖**同 (节点, kind) 的旧记录）⇒ 返回被覆盖的那条。
    pub fn insert(&mut self, node: K, kind: EventKind, value: V) -> Option<V> {
        self.by_node.entry(node).or_default().insert(kind, value)
    }

    /// 读某节点某 kind 的监听器。
    pub fn get(&self, node: &K, kind: EventKind) -> Option<&V> {
        self.by_node.get(node).and_then(|kinds| kinds.get(&kind))
    }

    /// 摘掉某节点某 kind 的监听器 ⇒ 返回被摘掉的那条。
    pub fn remove_kind(&mut self, node: &K, kind: EventKind) -> Option<V> {
        let kinds = self.by_node.get_mut(node)?;
        let removed = kinds.remove(&kind);
        if kinds.is_empty() {
            self.by_node.remove(node);
        }
        removed
    }

    /// 摘掉某节点的**全部**监听器 ⇒ 返回被摘掉的 (kind, 监听器) 列表。
    pub fn remove_node(&mut self, node: &K) -> Vec<(EventKind, V)> {
        match self.by_node.remove(node) {
            Some(kinds) => kinds.into_iter().collect(),
            None => Vec::new(),
        }
    }

    /// 监听器总条数。
    pub fn len(&self) -> usize {
        self.by_node.values().map(HashMap::len).sum()
    }

    /// 表是否为空。
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 监听器的占位值（节点编号 + 事件类型）。
    type Table = ListenerTable<u32, String>;

    #[test]
    fn inserting_the_same_kind_replaces_the_previous_entry() {
        let mut table = Table::new();
        assert_eq!(table.insert(1, EventKind::Click, String::from("a")), None);
        assert_eq!(
            table.insert(1, EventKind::Click, String::from("b")),
            Some(String::from("a")),
            "同 (节点, kind) 至多一条 ⇒ 后者覆盖前者"
        );
        assert_eq!(table.len(), 1);
        assert_eq!(table.get(&1, EventKind::Click), Some(&String::from("b")));
    }

    #[test]
    fn removing_one_kind_keeps_the_other_kinds_of_the_same_node() {
        let mut table = Table::new();
        table.insert(1, EventKind::Click, String::from("click"));
        table.insert(1, EventKind::KeyDown, String::from("key"));

        assert_eq!(
            table.remove_kind(&1, EventKind::Click),
            Some(String::from("click"))
        );
        assert_eq!(
            table.get(&1, EventKind::KeyDown),
            Some(&String::from("key")),
            "摘掉 click **不许**带走 keydown（既有契约）"
        );
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn removing_a_node_drops_all_of_its_kinds() {
        let mut table = Table::new();
        table.insert(1, EventKind::Click, String::from("click"));
        table.insert(1, EventKind::KeyDown, String::from("key"));
        table.insert(2, EventKind::Click, String::from("other"));

        let removed = table.remove_node(&1);
        assert_eq!(removed.len(), 2, "该节点的两个 kind 都被摘掉");
        assert_eq!(table.len(), 1, "别的节点不受影响");
        assert!(table.get(&1, EventKind::Click).is_none());
    }

    #[test]
    fn an_empty_node_entry_is_forgotten_entirely() {
        let mut table = Table::new();
        table.insert(1, EventKind::Click, String::from("click"));
        table.remove_kind(&1, EventKind::Click);
        assert!(table.is_empty(), "摘空之后不留空壳条目");
    }
}
