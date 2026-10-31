//! 实例引用槽位：组件 / 元素实例引用的绑定语义（设计文档 §5.4.1 `bind:this`）。
//!
//! ## 语义（设计文档 §5.4.1）
//!
//! `bind:this` **不是双向数据绑定**，是**引用绑定**：
//!
//! | 时机 | 动作 |
//! |---|---|
//! | 挂载后 | 把实例句柄**写入**槽位 |
//! | 卸载前 | 把槽位**清空** |
//!
//! 卸载前清空是硬要求：槽位里若留着已销毁的句柄，调用方会在「看起来还在」的
//! 实例上读出过期状态。
//!
//! ## 为什么这一层落在 `ui-core`
//!
//! 引用槽位本身是**生命周期**语义（与 [`crate::Scope`] 同族），不是 DOM 语义：
//! - DOM 侧把它特化成「元素句柄」（`ui-dom` 的 `ThisBinding<D: Dom>`）；
//! - 组件侧把它用作「组件实例引用」（本模块的通用形态）。
//!
//! ⇒ 槽位对**引用类型**是泛型的，`ui-core` 不需要知道任何平台句柄类型。

use std::cell::RefCell;
use std::rc::Rc;

/// 实例引用槽位：挂载时写入句柄、卸载前清空。
///
/// 克隆 = **共享同一个槽位**（与 [`crate::Signal`] 的克隆语义一致）⇒ 持有方
/// 可以把槽位交给子组件，子组件在挂载 / 卸载时写入 / 清空它。
pub struct InstanceRef<T> {
    slot: Rc<RefCell<Option<T>>>,
}

impl<T> std::fmt::Debug for InstanceRef<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstanceRef")
            .field("bound", &self.is_bound())
            .finish()
    }
}

impl<T> Clone for InstanceRef<T> {
    fn clone(&self) -> Self {
        Self {
            slot: self.slot.clone(),
        }
    }
}

impl<T> Default for InstanceRef<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> InstanceRef<T> {
    /// 建一个未绑定的槽位。
    pub fn new() -> Self {
        Self {
            slot: Rc::new(RefCell::new(None)),
        }
    }

    /// 读取引用（未绑定 ⇒ `None`）。
    pub fn get(&self) -> Option<T>
    where
        T: Clone,
    {
        self.slot.borrow().clone()
    }

    /// 槽位当前是否已绑定。
    pub fn is_bound(&self) -> bool {
        self.slot.borrow().is_some()
    }

    /// 挂载：写入实例句柄（覆盖旧值）。
    ///
    /// 幂等语义：**后写覆盖先写**（同一个槽位被两个实例依次挂载时，最后一个
    /// 生效 ⇒ 与「最后挂载者拥有该槽位」的直觉一致）。
    pub fn bind(&self, instance: T) {
        *self.slot.borrow_mut() = Some(instance);
    }

    /// 卸载：清空引用。
    ///
    /// **幂等**：未绑定时调用是空操作（不报错、不产生副作用）。
    pub fn unbind(&self) {
        self.slot.borrow_mut().take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_slot_is_unbound() {
        let slot: InstanceRef<u32> = InstanceRef::new();
        assert!(!slot.is_bound());
        assert_eq!(slot.get(), None, "挂载前引用必须为空");
    }

    #[test]
    fn bind_then_unbind_toggles_the_bound_state() {
        let slot: InstanceRef<u32> = InstanceRef::new();
        slot.bind(7);
        assert!(slot.is_bound());
        assert_eq!(slot.get(), Some(7));

        slot.unbind();
        assert!(!slot.is_bound());
        assert_eq!(slot.get(), None, "卸载前必须清空引用");
    }

    #[test]
    fn unbind_is_idempotent() {
        let slot: InstanceRef<u32> = InstanceRef::new();
        slot.bind(1);
        slot.unbind();
        slot.unbind(); // 幂等
        assert!(!slot.is_bound());
        assert_eq!(slot.get(), None);
    }

    #[test]
    fn cloned_slots_share_one_binding() {
        let slot: InstanceRef<u32> = InstanceRef::new();
        let clone = slot.clone();
        clone.bind(42);
        assert_eq!(slot.get(), Some(42), "克隆共享同一槽位");
        slot.unbind();
        assert!(!clone.is_bound(), "清空对所有持有方生效");
    }

    #[test]
    fn binding_twice_keeps_the_latest_instance() {
        let slot: InstanceRef<u32> = InstanceRef::new();
        slot.bind(1);
        slot.bind(2);
        assert_eq!(slot.get(), Some(2), "后挂载者拥有该槽位");
    }

    #[test]
    fn slot_can_carry_a_non_copy_handle() {
        // 真实 DOM 后端的句柄不是 `Copy` ⇒ 槽位只要求 `Clone`。
        let slot: InstanceRef<String> = InstanceRef::new();
        slot.bind(String::from("句柄"));
        assert_eq!(slot.get().as_deref(), Some("句柄"));
    }

    #[test]
    fn independent_slots_do_not_interfere() {
        let a: InstanceRef<u32> = InstanceRef::new();
        let b: InstanceRef<u32> = InstanceRef::new();
        a.bind(1);
        assert!(a.is_bound());
        assert!(!b.is_bound(), "两个槽位互不影响");
        b.bind(2);
        assert_eq!(a.get(), Some(1));
        assert_eq!(b.get(), Some(2));
    }
}
