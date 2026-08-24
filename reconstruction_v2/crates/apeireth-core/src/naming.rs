//! Naming - V0.5 命名空间 (从 v1.0 apeireth-naming-v05 5,153 LOC 收敛)
//!
//! 0 装 PASS: 重构版 naming 用简化 prefix + 4 维标签 (per right 图 "VCP 命名"):
//!   - PC: Pre-condition (前置条件)
//!   - RC: Re-validated Cost (重新验证成本)
//!   - HG: Human Guidance (人类指导强度)
//!   - GP: Goal Priority (目标优先级)

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// 4 维命名标签 (per VCP 命名, 决策 #22 §2.2)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NameAxis {
    PC,  // Pre-condition
    RC,  // Re-validated Cost
    HG,  // Human Guidance
    GP,  // Goal Priority
}

impl NameAxis {
    pub fn label(self) -> &'static str {
        match self {
            Self::PC => "PC",
            Self::RC => "RC",
            Self::HG => "HG",
            Self::GP => "GP",
        }
    }
}

/// 命名项 (0 装 PASS: name + 4 维标签 + 锁定的 24 维 LOCKED 状态)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub axes: HashMap<NameAxis, u8>,  // 0..3
    pub locked: bool,                 // 24 LOCKED 入口 (per decision-22 §2.2)
}

impl NamedItem {
    pub fn new(id: String, name: String, description: String) -> Self {
        Self { id, name, description, axes: HashMap::new(), locked: false }
    }

    /// 0 装 PASS: 锁定的命名项不能修改 axis (per decision-22 §2.2 "24 LOCKED 入口签名冻结降级")
    pub fn set_axis(&mut self, axis: NameAxis, value: u8) -> Result<(), String> {
        if self.locked { return Err("LOCKED naming entry cannot be modified".into()); }
        if value > 3 { return Err("axis value out of range 0..3".into()); }
        self.axes.insert(axis, value);
        Ok(())
    }
}

/// 命名空间注册表
pub struct NameRegistry {
    items: HashMap<String, NamedItem>,
}

impl NameRegistry {
    pub fn new() -> Self { Self { items: HashMap::new() } }

    pub fn register(&mut self, item: NamedItem) {
        self.items.insert(item.id.clone(), item);
    }

    pub fn get(&self, id: &str) -> Option<&NamedItem> {
        self.items.get(id)
    }

    pub fn list(&self) -> Vec<&NamedItem> {
        self.items.values().collect()
    }

    /// 按 axis 过滤 (例如: 找所有 GP>=2 的项)
    pub fn filter_by_axis(&self, axis: NameAxis, min_value: u8) -> Vec<&NamedItem> {
        self.items.values()
            .filter(|item| item.axes.get(&axis).map_or(0, |&v| v) >= min_value)
            .collect()
    }
}

impl Default for NameRegistry {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_4_axes_labels() {
        assert_eq!(NameAxis::PC.label(), "PC");
        assert_eq!(NameAxis::RC.label(), "RC");
        assert_eq!(NameAxis::HG.label(), "HG");
        assert_eq!(NameAxis::GP.label(), "GP");
    }

    #[test]
    fn test_locked_entry_cannot_modify() {
        let mut item = NamedItem::new("x".into(), "Test".into(), "desc".into());
        item.locked = true;
        assert!(item.set_axis(NameAxis::PC, 2).is_err());
    }

    #[test]
    fn test_axis_value_out_of_range() {
        let mut item = NamedItem::new("y".into(), "T".into(), "d".into());
        assert!(item.set_axis(NameAxis::GP, 4).is_err());
    }

    #[test]
    fn test_filter_by_axis() {
        let mut reg = NameRegistry::new();
        let mut a = NamedItem::new("a".into(), "A".into(), "".into());
        a.set_axis(NameAxis::GP, 3).unwrap();
        reg.register(a);
        let mut b = NamedItem::new("b".into(), "B".into(), "".into());
        b.set_axis(NameAxis::GP, 1).unwrap();
        reg.register(b);
        assert_eq!(reg.filter_by_axis(NameAxis::GP, 2).len(), 1);
    }
}
