//! 派生值：惰性求值 + 主动失效（设计文档 §4.3）。
//!
//! 决策：**失效是推的**（写信号 ⇒ 立刻沿订阅边把下游派生与 effect 标脏），
//! **求值是拉的**（派生只在有人读它时才重算，每次 flush 内至多重算一次）。
//!
//! 为什么选惰性：死枝不计算（条件块不显示时没人读它）、求值顺序不外泄、
//! 与 `ui-core` 的零依赖目标一致。

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::error::ReactiveError;
use crate::graph::NodeId;
use crate::runtime::{RuntimeInner, TrackingFrame};

/// 类型擦除的派生节点接口（运行期只负责「标脏」，求值由派生自身完成）。
pub(crate) trait DerivedOps {
    /// 把缓存标脏（下次读取时重算）。
    fn invalidate(&self);
}

/// 派生节点内部状态。
pub(crate) struct DerivedNode<T> {
    id: NodeId,
    compute: Rc<dyn Fn() -> T>,
    cache: RefCell<Option<T>>,
    dirty: Cell<bool>,
}
impl<T> DerivedOps for DerivedNode<T> {
    fn invalidate(&self) {
        self.dirty.set(true);
    }
}

/// 派生值：由其它信号算出的只读值。
pub struct Derived<T> {
    node: Rc<DerivedNode<T>>,
    inner: Rc<RuntimeInner>,
}

impl<T> std::fmt::Debug for Derived<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Derived")
            .field("id", &self.node.id)
            .finish()
    }
}

/// 派生的克隆 = 共享同一份缓存与依赖登记。
impl<T> Clone for Derived<T> {
    fn clone(&self) -> Self {
        Self {
            node: self.node.clone(),
            inner: self.inner.clone(),
        }
    }
}

impl<T: Clone + 'static> Derived<T> {
    /// 新建派生值（内部用；对外走 [`crate::Runtime::derived`]）。
    pub(crate) fn new(inner: Rc<RuntimeInner>, compute: impl Fn() -> T + 'static) -> Self {
        let id = inner.alloc_id();
        let node = Rc::new(DerivedNode {
            id,
            compute: Rc::new(compute),
            cache: RefCell::new(None),
            dirty: Cell::new(true),
        });
        inner.derived_nodes.borrow_mut().insert(id, node.clone());
        Self { node, inner }
    }

    /// 读派生值：脏则重算（求值期间登记依赖边），否则命中缓存。
    ///
    /// # Panics
    ///
    /// 求值形成环时 panic，见 [`ReactiveError::CycleDetected`]。
    pub fn get(&self) -> T {
        match self.try_get() {
            Ok(value) => value,
            Err(err) => panic!("{err}"),
        }
    }

    /// 读派生值（可失败版本）：求值环以错误返回，不 panic。
    pub fn try_get(&self) -> Result<T, ReactiveError> {
        if self.inner.tracking_contains(self.node.id) {
            let path = self.inner.tracking_path(self.node.id);
            return Err(ReactiveError::CycleDetected { path });
        }

        let needs_compute = {
            let cached = self.node.cache.borrow();
            self.node.dirty.get() || cached.is_none()
        };
        if needs_compute {
            // 重算前清空旧依赖 ⇒ 条件依赖正确。
            self.inner
                .graph
                .borrow_mut()
                .clear_dependencies(self.node.id);
            {
                let _guard = self.inner.push_frame(TrackingFrame::Derived(self.node.id));
                let value = (self.node.compute)();
                *self.node.cache.borrow_mut() = Some(value);
            }
            self.node.dirty.set(false);
        }

        // 登记「外层追踪域 依赖 本派生」⇒ 派生变脏时外层 effect 会重跑。
        self.inner.track(self.node.id);
        Ok(self
            .node
            .cache
            .borrow()
            .clone()
            .expect("派生缓存：重算后必定有值"))
    }

    /// 本派生的节点编号。
    pub fn id(&self) -> NodeId {
        self.node.id
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::Runtime;
    use crate::Derived;
    use std::cell::{Cell, RefCell};
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::rc::Rc;

    #[test]
    fn derived_is_lazy_and_caches_until_invalidated() {
        let runtime = Runtime::new();
        let calls = Rc::new(Cell::new(0));
        let base = runtime.signal(1u32);

        let c = calls.clone();
        let b = base.clone();
        let doubled = runtime.derived(move || {
            c.set(c.get() + 1);
            b.get() * 2
        });

        assert_eq!(calls.get(), 0, "没人读 ⇒ 不算（惰性）");
        assert_eq!(doubled.get(), 2);
        assert_eq!(calls.get(), 1);
        assert_eq!(doubled.get(), 2);
        assert_eq!(calls.get(), 1, "缓存命中 ⇒ 不重算");

        base.set(5);
        assert_eq!(doubled.get(), 10);
        assert_eq!(calls.get(), 2, "依赖变脏 ⇒ 重算一次");
    }

    #[test]
    fn effect_reading_derived_reruns_when_base_changes() {
        let runtime = Runtime::new();
        let seen = Rc::new(RefCell::new(Vec::<u32>::new()));
        let base = runtime.signal(1u32);
        let b = base.clone();
        let doubled = runtime.derived(move || b.get() * 2);

        let s = seen.clone();
        let d = doubled.clone();
        runtime.effect(move || {
            s.borrow_mut().push(d.get());
        });

        runtime.flush();
        base.set(4);
        runtime.flush();

        assert_eq!(&*seen.borrow(), &[2, 8]);
    }

    #[test]
    fn derived_cycle_is_detected() {
        let runtime = Runtime::new();
        let slot_a: Rc<RefCell<Option<Derived<u32>>>> = Rc::new(RefCell::new(None));
        let slot_b: Rc<RefCell<Option<Derived<u32>>>> = Rc::new(RefCell::new(None));

        let read_b = slot_b.clone();
        let a = runtime.derived(move || read_b.borrow().as_ref().expect("b 已装入").get() + 1);
        let read_a = slot_a.clone();
        let b = runtime.derived(move || read_a.borrow().as_ref().expect("a 已装入").get() * 2);

        *slot_a.borrow_mut() = Some(a);
        *slot_b.borrow_mut() = Some(b);

        // 求值环：内层检出后由 get() 以 panic 报出（错误类型 CycleDetected）。
        let result = catch_unwind(AssertUnwindSafe(|| {
            slot_a.borrow().as_ref().expect("a 已装入").get()
        }));
        assert!(result.is_err(), "求值环必须被检出");
    }
}
