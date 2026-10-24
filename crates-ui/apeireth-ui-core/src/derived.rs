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

    /// 本派生当前是否被标脏（缓存已失效、待重算）。
    pub fn is_dirty(&self) -> bool {
        self.node.dirty.get()
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

    // ── 判别测试（钉行为不钉字面）──────────────────────────────────────────

    /// 跨派生的环：错误必须**带路径**（首尾同节点 + 覆盖环上全部节点）。
    #[test]
    fn cross_derived_cycle_reports_the_full_path() {
        let runtime = Runtime::new();
        let slot_a: Rc<RefCell<Option<Derived<u32>>>> = Rc::new(RefCell::new(None));
        let slot_b: Rc<RefCell<Option<Derived<u32>>>> = Rc::new(RefCell::new(None));
        let captured: Rc<RefCell<Option<crate::ReactiveError>>> = Rc::new(RefCell::new(None));

        let read_b = slot_b.clone();
        let sink = captured.clone();
        let a =
            runtime.derived(
                move || match read_b.borrow().as_ref().expect("b 已装入").try_get() {
                    Ok(value) => value + 1,
                    Err(err) => {
                        *sink.borrow_mut() = Some(err);
                        0
                    }
                },
            );
        let read_a = slot_a.clone();
        let sink = captured.clone();
        let b =
            runtime.derived(
                move || match read_a.borrow().as_ref().expect("a 已装入").try_get() {
                    Ok(value) => value * 2,
                    Err(err) => {
                        *sink.borrow_mut() = Some(err);
                        0
                    }
                },
            );

        *slot_a.borrow_mut() = Some(a);
        *slot_b.borrow_mut() = Some(b);

        let a_id = slot_a.borrow().as_ref().expect("a 已装入").id();
        let b_id = slot_b.borrow().as_ref().expect("b 已装入").id();
        let _ = slot_a.borrow().as_ref().expect("a 已装入").try_get();

        let err = captured
            .borrow()
            .clone()
            .expect("跨派生求值环必须被检出并报出路径");
        match err {
            crate::ReactiveError::CycleDetected { path } => {
                assert_eq!(path.len(), 3, "路径 = a -> b -> a，实测：{path:?}");
                assert_eq!(path[0], path[2], "首尾必须是同一节点");
                assert!(path.contains(&a_id), "路径必须含 a");
                assert!(path.contains(&b_id), "路径必须含 b");
            }
            other => panic!("实测错误类型：{other:?}"),
        }
    }

    /// 环路径必须能渲染成**人可读**的一行（设计文档 §4.4 的硬要求）。
    #[test]
    fn cycle_path_renders_labels_when_available() {
        let runtime = Runtime::new();
        let slot_a: Rc<RefCell<Option<Derived<u32>>>> = Rc::new(RefCell::new(None));
        let slot_b: Rc<RefCell<Option<Derived<u32>>>> = Rc::new(RefCell::new(None));

        let read_b = slot_b.clone();
        let a = runtime.derived(move || read_b.borrow().as_ref().expect("b 已装入").get() + 1);
        let read_a = slot_a.clone();
        let b = runtime.derived(move || read_a.borrow().as_ref().expect("a 已装入").get() * 2);
        *slot_a.borrow_mut() = Some(a);
        *slot_b.borrow_mut() = Some(b);

        let a_id = slot_a.borrow().as_ref().expect("a 已装入").id();
        let b_id = slot_b.borrow().as_ref().expect("b 已装入").id();
        runtime.set_label(a_id, "视图::计数");
        runtime.set_label(b_id, "视图::翻倍");

        let described = runtime.describe_cycle_path(&[a_id, b_id, a_id]);
        assert!(described.contains("视图::计数"), "实测：{described}");
        assert!(described.contains("视图::翻倍"), "实测：{described}");
        assert!(
            described.contains("->"),
            "路径必须可读地串起来：{described}"
        );
    }

    /// **不误报**合法图：菱形依赖（A 读 B、C；B、C 都读 D）不是环。
    #[test]
    fn diamond_dependency_is_not_mistaken_for_a_cycle() {
        let runtime = Runtime::new();
        let base = runtime.signal(2u32);

        let d = {
            let base = base.clone();
            runtime.derived(move || base.get() + 1)
        };
        let b = {
            let d = d.clone();
            runtime.derived(move || d.get() * 10)
        };
        let c = {
            let d = d.clone();
            runtime.derived(move || d.get() * 100)
        };
        let a = {
            let b = b.clone();
            let c = c.clone();
            runtime.derived(move || b.get() + c.get())
        };

        assert_eq!(a.get(), 330, "菱形图必须正常求值");
        base.set(3);
        assert_eq!(a.get(), 440, "菱形图必须正常重算");
    }

    /// 派生缓存**只**被自己的依赖失效（无关写入不得让缓存失效）。
    #[test]
    fn unrelated_write_does_not_invalidate_the_derived_cache() {
        let runtime = Runtime::new();
        let base = runtime.signal(1u32);
        let unrelated = runtime.signal(0u32);
        let calls = Rc::new(Cell::new(0));

        let b = base.clone();
        let counter = calls.clone();
        let doubled = runtime.derived(move || {
            counter.set(counter.get() + 1);
            b.get() * 2
        });

        assert_eq!(doubled.get(), 2);
        assert_eq!(calls.get(), 1);

        unrelated.set(1);
        assert!(!doubled.is_dirty(), "无关写入不得让派生缓存失效");
        assert_eq!(doubled.get(), 2);
        assert_eq!(calls.get(), 1, "无关写入后不得重算");

        base.set(5);
        assert!(doubled.is_dirty(), "自己的依赖变了 ⇒ 缓存失效");
        assert_eq!(doubled.get(), 10);
        assert_eq!(calls.get(), 2);
    }

    /// 一次失效只重算一次（多次读命中同一份缓存）。
    #[test]
    fn derived_recomputes_at_most_once_per_invalidation() {
        let runtime = Runtime::new();
        let base = runtime.signal(1u32);
        let calls = Rc::new(Cell::new(0));

        let b = base.clone();
        let counter = calls.clone();
        let derived = runtime.derived(move || {
            counter.set(counter.get() + 1);
            b.get()
        });

        derived.get();
        derived.get();
        derived.get();
        assert_eq!(calls.get(), 1, "缓存命中不得重复求值");

        base.set(2);
        derived.get();
        derived.get();
        assert_eq!(calls.get(), 2, "一次失效只重算一次");
    }

    /// read-after-write 可见性：写完立刻读派生，必须拿到**新值**（不是旧缓存）。
    #[test]
    fn derived_reflects_the_value_written_just_now() {
        let runtime = Runtime::new();
        let base = runtime.signal(1u32);
        let b = base.clone();
        let derived = runtime.derived(move || b.get() * 3);

        assert_eq!(derived.get(), 3);
        base.set(4);
        assert_eq!(derived.get(), 12, "写后读派生必须立刻是新值");
    }
}
