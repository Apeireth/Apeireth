//! 信号：可写状态单元（设计文档 §4.1）。
//!
//! 读语义：
//! - 在**追踪域**（effect / 派生执行栈内）读取 ⇒ 登记依赖边；
//! - 在追踪域外读取 ⇒ 只取值，不登记任何依赖。
//!
//! 写语义：
//! - `set` **总是**使能失效并沿依赖图传播（不做「值相等就不触发」的隐式判断）；
//! - 需要等值不触发时显式用 [`Signal::try_set_if_changed`]。
//!
//! 版本号：每次写自增，供下游判断「值是否真的变过」。

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::error::ReactiveError;
use crate::graph::NodeId;
use crate::runtime::RuntimeInner;

/// 信号内部状态（信号与只读视图共享同一份）。
#[derive(Debug)]
pub(crate) struct SignalCell<T> {
    pub(crate) value: RefCell<T>,
    pub(crate) version: Cell<u64>,
}

/// 可写信号：响应式状态的基本单元。
pub struct Signal<T> {
    id: NodeId,
    inner: Rc<RuntimeInner>,
    cell: Rc<SignalCell<T>>,
}

impl<T> std::fmt::Debug for Signal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Signal")
            .field("id", &self.id)
            .field("version", &self.cell.version.get())
            .finish()
    }
}

/// 信号的克隆 = **共享同一份状态**（不是深拷贝）：`T` 本身不必可克隆。
impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            inner: self.inner.clone(),
            cell: self.cell.clone(),
        }
    }
}

impl<T> Signal<T> {
    /// 新建信号（内部用；对外走 [`crate::Runtime::signal`]）。
    pub(crate) fn new(inner: Rc<RuntimeInner>, value: T) -> Self {
        let id = inner.alloc_id();
        Self {
            id,
            inner,
            cell: Rc::new(SignalCell {
                value: RefCell::new(value),
                version: Cell::new(0),
            }),
        }
    }

    /// 写信号；派生求值期间写入会返回错误。
    pub fn try_set(&self, value: T) -> Result<(), ReactiveError> {
        self.check_writable()?;
        *self.cell.value.borrow_mut() = value;
        self.bump_version();
        self.inner.propagate_dirty(self.id);
        Ok(())
    }

    /// 写信号（派生求值期间写入会 panic —— 派生必须是纯函数）。
    ///
    /// # Panics
    ///
    /// 派生求值期间写入本信号时 panic，见 [`ReactiveError::MutationDuringDerivation`]。
    pub fn set(&self, value: T) {
        if let Err(err) = self.try_set(value) {
            panic!("{err}");
        }
    }

    /// 读改写：把当前值交给调用方就地修改，然后照常失效传播。
    pub fn try_update(&self, f: impl FnOnce(&mut T)) -> Result<(), ReactiveError> {
        self.check_writable()?;
        {
            let mut guard = self.cell.value.borrow_mut();
            f(&mut guard);
        }
        self.bump_version();
        self.inner.propagate_dirty(self.id);
        Ok(())
    }

    /// 读改写（派生求值期间会 panic）。
    ///
    /// # Panics
    ///
    /// 派生求值期间写入本信号时 panic。
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        if let Err(err) = self.try_update(f) {
            panic!("{err}");
        }
    }

    /// 本信号的节点编号。
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// 降级为只读视图（可跨层传递、不可写）。
    pub fn read_only(&self) -> ReadSignal<T> {
        ReadSignal {
            id: self.id,
            inner: self.inner.clone(),
            cell: self.cell.clone(),
        }
    }

    /// 版本号（每次写自增）。
    pub fn version(&self) -> u64 {
        self.cell.version.get()
    }

    /// 版本号自增并触发失效传播。
    fn bump_version(&self) {
        self.cell.version.set(self.cell.version.get() + 1);
    }

    /// 派生求值期间禁止写信号。
    fn check_writable(&self) -> Result<(), ReactiveError> {
        if self.inner.in_derivation() {
            let node = self.inner.current_frame().unwrap_or(self.id);
            return Err(ReactiveError::MutationDuringDerivation { node });
        }
        Ok(())
    }
}

impl<T: Clone> Signal<T> {
    /// 读信号：在追踪域内会登记依赖边。
    pub fn get(&self) -> T {
        self.inner.track(self.id);
        self.cell.value.borrow().clone()
    }
}

impl<T: Clone + PartialEq> Signal<T> {
    /// 等值不触发：值相等时不失效、不传播，返回 `false`。
    pub fn try_set_if_changed(&self, value: T) -> Result<bool, ReactiveError> {
        let unchanged = {
            let current = self.cell.value.borrow();
            *current == value
        };
        if unchanged {
            return Ok(false);
        }
        self.try_set(value)?;
        Ok(true)
    }

    /// 等值不触发（派生求值期间会 panic）。
    ///
    /// # Panics
    ///
    /// 派生求值期间写入本信号时 panic。
    pub fn set_if_changed(&self, value: T) -> bool {
        match self.try_set_if_changed(value) {
            Ok(changed) => changed,
            Err(err) => panic!("{err}"),
        }
    }
}

/// 只读信号视图：可跨层传递、不可写。
pub struct ReadSignal<T> {
    id: NodeId,
    inner: Rc<RuntimeInner>,
    cell: Rc<SignalCell<T>>,
}

impl<T> std::fmt::Debug for ReadSignal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReadSignal")
            .field("id", &self.id)
            .field("version", &self.cell.version.get())
            .finish()
    }
}

/// 只读视图的克隆 = 共享同一份状态。
impl<T> Clone for ReadSignal<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            inner: self.inner.clone(),
            cell: self.cell.clone(),
        }
    }
}

impl<T: Clone> ReadSignal<T> {
    /// 读信号：在追踪域内会登记依赖边。
    pub fn get(&self) -> T {
        self.inner.track(self.id);
        self.cell.value.borrow().clone()
    }

    /// 本信号的节点编号。
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// 版本号（每次写自增）。
    pub fn version(&self) -> u64 {
        self.cell.version.get()
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::Runtime;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn read_outside_tracking_registers_no_dependency() {
        let runtime = Runtime::new();
        let counter = Rc::new(Cell::new(0));
        let value = runtime.signal(7u32);

        // effect 体**不读**信号 ⇒ 无依赖边。
        let c = counter.clone();
        runtime.effect(move || {
            c.set(c.get() + 1);
        });

        runtime.flush();
        assert_eq!(counter.get(), 1, "effect 首次执行一次");

        // 追踪域外读取：不登记任何依赖 ⇒ 改值不触发重跑。
        assert_eq!(value.get(), 7);
        value.set(8);
        runtime.flush();
        assert_eq!(counter.get(), 1, "追踪域外读取 ⇒ 无依赖 ⇒ 不重跑");
    }

    #[test]
    fn read_inside_tracking_registers_dependency() {
        let runtime = Runtime::new();
        let counter = Rc::new(Cell::new(0));
        let value = runtime.signal(7u32);

        let c = counter.clone();
        let v = value.clone();
        runtime.effect(move || {
            c.set(c.get() + 1);
            let _ = v.get();
        });

        runtime.flush();
        value.set(8);
        runtime.flush();
        assert_eq!(counter.get(), 2, "追踪域内读取 ⇒ 登记依赖 ⇒ 改值即重跑");
    }

    #[test]
    fn set_if_changed_skips_equal_values() {
        let runtime = Runtime::new();
        let counter = Rc::new(Cell::new(0));
        let value = runtime.signal(1u32);

        let c = counter.clone();
        let v = value.clone();
        runtime.effect(move || {
            c.set(c.get() + 1);
            let _ = v.get();
        });
        runtime.flush();

        assert!(!value.set_if_changed(1), "等值 ⇒ 不触发");
        runtime.flush();
        assert_eq!(counter.get(), 1);

        assert!(value.set_if_changed(2), "不等 ⇒ 触发");
        runtime.flush();
        assert_eq!(counter.get(), 2);
    }

    #[test]
    fn version_increments_on_every_write() {
        let runtime = Runtime::new();
        let value = runtime.signal(0u32);
        assert_eq!(value.version(), 0);
        value.set(1);
        assert_eq!(value.version(), 1);
        value.update(|v| *v += 1);
        assert_eq!(value.version(), 2);
        assert_eq!(value.get(), 2);
    }

    #[test]
    fn read_only_view_shares_value_but_cannot_write() {
        let runtime = Runtime::new();
        let value = runtime.signal(5u32);
        let view = value.read_only();
        value.set(6);
        assert_eq!(view.get(), 6);
        assert_eq!(view.version(), 1);
    }
}
