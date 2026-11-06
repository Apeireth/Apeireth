//! effect 与作用域：副作用的生命周期边界（设计文档 §4.5）。
//!
//! effect 语义：
//! - 创建时**不**同步执行 ⇒ 首次执行发生在 flush 的 effect 阶段（DOM 已就位）；
//! - 依赖被标脏 ⇒ 重跑；重跑前清空旧依赖 ⇒ 条件依赖正确；
//! - 作用域是销毁的组织单位：子作用域随父作用域一并销毁，且按 **LIFO 逆序**
//!   销毁（子组件的 effect 先于父组件销毁）。
//!
//! ## 销毁必须真的释放（长跑 SPA 的内存账）
//!
//! 销毁不是「打个标记」：effect 体（闭包）必须**从运行时表里摘掉**，闭包捕获的
//! DOM 节点 / 组件状态才可能被回收 ⇒ 否则运行时表单调增长，内存线性上涨。
//! 执行次数（`runs`）随句柄走（不随表走）⇒ 销毁后仍可读。

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::graph::NodeId;
use crate::runtime::{Runtime, RuntimeInner};

/// effect 体：可多次调用的闭包（内部用 `Rc<RefCell<..>>` 以便在 flush 中安全取出）。
pub(crate) type EffectBody = Rc<RefCell<dyn FnMut()>>;

/// effect 节点状态（内部用）。
pub(crate) struct EffectState {
    /// effect 体；销毁后置空 ⇒ 闭包（及其捕获物）可被回收。
    pub(crate) body: Option<EffectBody>,
    /// 是否被标脏（待重跑）。
    pub(crate) dirty: bool,
    /// 执行次数（与 [`EffectHandle`] 共享同一份计数 ⇒ 销毁后仍可读）。
    pub(crate) runs: Rc<Cell<usize>>,
    /// 清理回调（销毁时按逆序执行）。
    pub(crate) cleanups: Vec<Rc<dyn Fn()>>,
    /// 是否已销毁。
    pub(crate) disposed: bool,
    /// 本 effect 的阶段（渲染前 / 渲染后）⇒ 重排时回投对应队列。
    pub(crate) pre: bool,
}

/// effect 句柄：可销毁、可注册清理回调。
#[derive(Clone)]
pub struct EffectHandle {
    pub(crate) id: NodeId,
    pub(crate) inner: Rc<RuntimeInner>,
    /// 执行次数（与运行时表里的状态共享同一份计数 ⇒ 销毁后仍可读）。
    pub(crate) runs: Rc<Cell<usize>>,
}

impl std::fmt::Debug for EffectHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EffectHandle")
            .field("id", &self.id)
            .field("runs", &self.runs())
            .finish()
    }
}

impl EffectHandle {
    /// 本 effect 的节点编号。
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// 销毁 effect：解除全部依赖订阅、按逆序执行清理回调、**从运行时表摘除**。
    ///
    /// **幂等**：重复调用安全（第二次是空操作）。
    ///
    /// 销毁后：
    /// - effect 体被释放（闭包捕获的 DOM 节点 / 组件状态可被回收）；
    /// - 节点从运行时的 effect 表里移除（表不得单调增长）；
    /// - 依赖图里该节点的双向边与标签一并清掉；
    /// - [`EffectHandle::runs`] 在销毁后仍可读（计数随句柄走）。
    pub fn dispose(&self) {
        let cleanups = {
            let mut effects = self.inner.effects.borrow_mut();
            match effects.remove(&self.id) {
                Some(mut state) => {
                    state.disposed = true;
                    // 置空 body ⇒ 闭包（及其捕获物）在这里被释放。
                    state.body = None;
                    std::mem::take(&mut state.cleanups)
                }
                None => Vec::new(),
            }
        };
        // 依赖图：双向边 + 标签一并摘掉（否则图与标签表只增不减）。
        self.inner.graph.borrow_mut().remove_node(self.id);
        for cleanup in cleanups.iter().rev() {
            cleanup();
        }
    }

    /// 注册清理回调（销毁时按逆序执行）。
    ///
    /// **已销毁时立即执行该回调**：清理回调的职责是「释放资源」，若 effect 已经
    /// 销毁却把回调**静默丢弃**，那份资源就永远不会被释放 ⇒ 这里改为**当场执行**
    ///（不再有可销毁的 effect 体可清）。需要「显式报错」语义时用
    /// [`EffectHandle::try_on_cleanup`]。
    pub fn on_cleanup(&self, cleanup: impl Fn() + 'static) {
        // 已销毁 ⇒ 回调当场执行；未销毁 ⇒ 登记（两种情况都由 `try_on_cleanup` 落地）。
        let _ = self.try_on_cleanup(cleanup);
    }

    /// 注册清理回调（可失败版本）：effect 已销毁时返回
    /// [`crate::ReactiveError::DisposedScope`]。
    ///
    /// 注意：回调**仍会当场执行**（清理回调的职责是「释放资源」⇒ 不得静默丢弃
    /// ⇒ 资源必须被释放）；`Err` 只是告诉调用方「这个 effect 已经没了」。
    pub fn try_on_cleanup(&self, cleanup: impl Fn() + 'static) -> Result<(), crate::ReactiveError> {
        let mut effects = self.inner.effects.borrow_mut();
        match effects.get_mut(&self.id) {
            Some(state) if !state.disposed => {
                state.cleanups.push(Rc::new(cleanup));
                Ok(())
            }
            _ => {
                // 已销毁：把回调**当场执行**（不得静默丢弃），再把「已销毁」报给调用方。
                drop(effects);
                cleanup();
                Err(crate::ReactiveError::DisposedScope)
            }
        }
    }

    /// 本 effect 已执行次数（销毁后仍可读）。
    pub fn runs(&self) -> usize {
        self.runs.get()
    }

    /// 本 effect 当前是否被标脏（待重跑）。
    ///
    /// 语义是**实际脏状态**：effect 体运行期间若把自己重新标脏（例如自激），
    /// 运行结束后这里仍是 `true` ⇒ 不会「撒谎」说自己是干净的。
    pub fn is_dirty(&self) -> bool {
        self.inner
            .effects
            .borrow()
            .get(&self.id)
            .map(|state| state.dirty)
            .unwrap_or(false)
    }
}

/// 作用域：组件生命周期边界（effect 的销毁组织单位）。
#[derive(Clone)]
pub struct Scope {
    inner: Rc<ScopeInner>,
}

struct ScopeInner {
    runtime: Rc<Runtime>,
    children: RefCell<Vec<Rc<ScopeInner>>>,
    handles: RefCell<Vec<EffectHandle>>,
    disposed: Cell<bool>,
}

impl std::fmt::Debug for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scope")
            .field("disposed", &self.is_disposed())
            .field("effects", &self.inner.handles.borrow().len())
            .finish()
    }
}

impl Scope {
    /// 新建作用域（内部用；对外走 [`crate::Runtime::scope`]）。
    pub(crate) fn new(runtime: Rc<Runtime>) -> Self {
        Self {
            inner: Rc::new(ScopeInner {
                runtime,
                children: RefCell::new(Vec::new()),
                handles: RefCell::new(Vec::new()),
                disposed: Cell::new(false),
            }),
        }
    }

    /// 建子作用域（随本作用域一并销毁）。
    ///
    /// # Panics
    ///
    /// 作用域已销毁时 panic（与 [`Scope::effect`] 同口径：已销毁作用域不得再
    /// 产生新节点，否则那些节点**永远没人销毁** ⇒ 孤儿 effect 每次写信号都重跑）。
    /// 需要显式错误语义时用 [`Scope::try_child`]。
    pub fn child(&self) -> Scope {
        match self.try_child() {
            Ok(child) => child,
            Err(err) => panic!("{err}"),
        }
    }

    /// 建子作用域（可失败版本）：本作用域已销毁时返回
    /// [`crate::ReactiveError::DisposedScope`]。
    pub fn try_child(&self) -> Result<Scope, crate::ReactiveError> {
        if self.is_disposed() {
            return Err(crate::ReactiveError::DisposedScope);
        }
        let child = Scope::new(self.inner.runtime.clone());
        self.inner.children.borrow_mut().push(child.inner.clone());
        Ok(child)
    }

    /// 在本作用域内建一个 effect（渲染后执行）。
    ///
    /// # Panics
    ///
    /// 作用域已销毁时 panic（见 [`crate::ReactiveError::DisposedScope`]）。
    pub fn effect(&self, f: impl FnMut() + 'static) -> EffectHandle {
        self.assert_alive();
        let handle = self.inner.runtime.spawn_effect(f, false);
        self.inner.handles.borrow_mut().push(handle.clone());
        handle
    }

    /// 在本作用域内建一个渲染前 effect。
    ///
    /// # Panics
    ///
    /// 作用域已销毁时 panic。
    pub fn effect_pre(&self, f: impl FnMut() + 'static) -> EffectHandle {
        self.assert_alive();
        let handle = self.inner.runtime.spawn_effect(f, true);
        self.inner.handles.borrow_mut().push(handle.clone());
        handle
    }

    /// 销毁作用域：子作用域先销毁、本作用域的 effect 按逆序销毁。
    ///
    /// **幂等**：重复调用安全。
    pub fn dispose(&self) {
        if self.inner.disposed.get() {
            return;
        }
        self.inner.disposed.set(true);

        let children: Vec<Rc<ScopeInner>> = self.inner.children.borrow_mut().drain(..).collect();
        for child in children.iter().rev() {
            Scope {
                inner: child.clone(),
            }
            .dispose();
        }

        let handles: Vec<EffectHandle> = self.inner.handles.borrow_mut().drain(..).collect();
        for handle in handles.iter().rev() {
            handle.dispose();
        }
    }

    /// 本作用域是否已销毁。
    pub fn is_disposed(&self) -> bool {
        self.inner.disposed.get()
    }

    /// 本作用域直接持有的 effect 数。
    pub fn effect_count(&self) -> usize {
        self.inner.handles.borrow().len()
    }

    /// 作用域未销毁断言。
    fn assert_alive(&self) {
        assert!(
            !self.is_disposed(),
            "{}",
            crate::ReactiveError::DisposedScope
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::Runtime;
    use std::cell::{Cell, RefCell};
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::rc::Rc;

    #[test]
    fn dispose_is_idempotent_and_runs_cleanups_in_reverse_order() {
        let runtime = Runtime::new();
        let log = Rc::new(RefCell::new(Vec::<u32>::new()));
        let value = runtime.signal(0u32);

        let v = value.clone();
        let handle = runtime.effect(move || {
            let _ = v.get();
        });
        handle.on_cleanup({
            let l = log.clone();
            move || l.borrow_mut().push(1)
        });
        handle.on_cleanup({
            let l = log.clone();
            move || l.borrow_mut().push(2)
        });

        runtime.flush();
        handle.dispose();
        handle.dispose(); // 幂等
        assert_eq!(&*log.borrow(), &[2, 1], "清理回调按逆序执行且只执行一次");

        value.set(1);
        runtime.flush();
        assert_eq!(handle.runs(), 1, "销毁后不再重跑");
    }

    #[test]
    fn scope_disposes_children_before_parent() {
        let runtime = Runtime::new();
        let log = Rc::new(RefCell::new(Vec::<&'static str>::new()));
        let parent = runtime.scope();
        let child = parent.child();

        parent.effect(|| {}).on_cleanup({
            let l = log.clone();
            move || l.borrow_mut().push("parent")
        });
        child.effect(|| {}).on_cleanup({
            let l = log.clone();
            move || l.borrow_mut().push("child")
        });

        parent.dispose();
        assert_eq!(
            &*log.borrow(),
            &["child", "parent"],
            "子作用域先于父作用域销毁（LIFO）"
        );
        assert!(parent.is_disposed());
        assert!(child.is_disposed());
    }

    #[test]
    fn cleanup_callbacks_run_on_scope_dispose() {
        let runtime = Runtime::new();
        let log = Rc::new(RefCell::new(Vec::<&'static str>::new()));
        let scope = runtime.scope();

        let handle = scope.effect(|| {});
        handle.on_cleanup({
            let l = log.clone();
            move || l.borrow_mut().push("cleaned")
        });

        scope.dispose();
        assert_eq!(&*log.borrow(), &["cleaned"]);
        assert_eq!(scope.effect_count(), 0);
    }

    // ── 判别测试（钉行为不钉字面）──────────────────────────────────────────

    /// 作用域销毁时，effect 必须按**创建逆序**销毁（LIFO）。
    #[test]
    fn scope_disposes_its_effects_in_reverse_creation_order() {
        let runtime = Runtime::new();
        let log = Rc::new(RefCell::new(Vec::<u32>::new()));
        let scope = runtime.scope();

        for index in 0..3u32 {
            scope.effect(|| {}).on_cleanup({
                let l = log.clone();
                move || l.borrow_mut().push(index)
            });
        }

        scope.dispose();
        assert_eq!(&*log.borrow(), &[2, 1, 0], "清理必须按创建逆序（子先于父）");
    }

    /// 作用域销毁**幂等**：第二次销毁不得重复跑清理回调。
    #[test]
    fn scope_dispose_is_idempotent() {
        let runtime = Runtime::new();
        let count = Rc::new(Cell::new(0));
        let scope = runtime.scope();

        scope.effect(|| {}).on_cleanup({
            let c = count.clone();
            move || c.set(c.get() + 1)
        });

        scope.dispose();
        scope.dispose();
        scope.dispose();
        assert_eq!(count.get(), 1, "销毁幂等 ⇒ 清理只跑一次");
        assert!(scope.is_disposed());
    }

    /// 销毁后的 effect 不得再被标脏、不得再执行。
    #[test]
    fn disposed_effect_is_neither_dirty_nor_rerun() {
        let runtime = Runtime::new();
        let value = runtime.signal(0u32);
        let runs = Rc::new(Cell::new(0));

        let v = value.clone();
        let r = runs.clone();
        let handle = runtime.effect(move || {
            r.set(r.get() + 1);
            let _ = v.get();
        });

        runtime.flush();
        assert_eq!(runs.get(), 1);
        handle.dispose();
        assert!(!handle.is_dirty(), "销毁后不得被标脏");

        value.set(1);
        runtime.flush();
        assert_eq!(runs.get(), 1, "销毁后不得再执行");
        assert_eq!(handle.runs(), 1, "执行次数记录在销毁后仍可读");
    }

    /// 销毁必须**解除依赖订阅**：销毁后依赖图里不得残留该节点的边。
    #[test]
    fn dispose_removes_the_dependency_edges() {
        let runtime = Runtime::new();
        let value = runtime.signal(0u32);

        let v = value.clone();
        let handle = runtime.effect(move || {
            let _ = v.get();
        });
        runtime.flush();
        assert!(!runtime.graph().dependencies_of(handle.id()).is_empty());

        handle.dispose();
        assert!(
            runtime.graph().dependencies_of(handle.id()).is_empty(),
            "销毁后不得残留依赖边"
        );
        assert!(
            runtime.graph().dependents_of(value.id()).is_empty(),
            "反向边也必须解除"
        );
    }

    /// 子作用域的 effect 随父作用域一并销毁（且子先于父）。
    #[test]
    fn child_scope_effects_die_with_the_parent() {
        let runtime = Runtime::new();
        let log = Rc::new(RefCell::new(Vec::<&'static str>::new()));
        let parent = runtime.scope();
        let child = parent.child();

        parent.effect(|| {}).on_cleanup({
            let l = log.clone();
            move || l.borrow_mut().push("parent")
        });
        child.effect(|| {}).on_cleanup({
            let l = log.clone();
            move || l.borrow_mut().push("child")
        });

        parent.dispose();
        assert_eq!(&*log.borrow(), &["child", "parent"]);
        assert!(child.is_disposed());
    }

    /// 已销毁的作用域不得再创建 effect（显式报错，不静默降级）。
    #[test]
    fn disposed_scope_refuses_to_create_effects() {
        let runtime = Runtime::new();
        let scope = runtime.scope();
        scope.dispose();

        let result = catch_unwind(AssertUnwindSafe(|| {
            scope.effect(|| {});
        }));
        assert!(result.is_err(), "已销毁作用域必须显式报错");
    }

    // ── 本批重构新增的判别测试 ─────────────────────────────────────────────

    /// 销毁后再注册清理回调 ⇒ 回调**必须当场执行**（不得静默丢弃）。
    #[test]
    fn cleanup_registered_after_dispose_runs_immediately() {
        let runtime = Runtime::new();
        let count = Rc::new(Cell::new(0));
        let handle = runtime.effect(|| {});
        handle.dispose();

        handle.on_cleanup({
            let c = count.clone();
            move || c.set(c.get() + 1)
        });

        assert_eq!(count.get(), 1, "已销毁后注册的清理回调必须当场执行");
    }

    /// `try_on_cleanup` 在已销毁时**显式报错**（不静默降级）。
    #[test]
    fn try_on_cleanup_reports_disposed_scope() {
        let runtime = Runtime::new();
        let handle = runtime.effect(|| {});
        handle.dispose();

        let err = handle
            .try_on_cleanup(|| {})
            .expect_err("已销毁的 effect 必须显式报错");
        assert_eq!(err, crate::ReactiveError::DisposedScope);
    }

    /// 已销毁的作用域不得再创建子作用域（否则子作用域永远没人销毁）。
    #[test]
    fn disposed_scope_refuses_to_create_children() {
        let runtime = Runtime::new();
        let scope = runtime.scope();
        scope.dispose();

        assert!(scope.try_child().is_err(), "已销毁作用域必须显式报错");
        let result = catch_unwind(AssertUnwindSafe(|| {
            scope.child();
        }));
        assert!(result.is_err(), "child() 必须做存活断言");
    }

    /// 销毁 ⇒ effect 体被释放 ⇒ 闭包捕获的 `Rc` 计数回落（内存账）。
    #[test]
    fn dispose_releases_the_effect_body() {
        let runtime = Runtime::new();
        let captured = Rc::new(Cell::new(0u32));
        let handle = {
            let c = captured.clone();
            runtime.effect(move || {
                c.set(c.get() + 1);
            })
        };

        // 运行时表持有闭包 ⇒ 闭包里的 Rc 有两份（测试自己一份 + 闭包一份）。
        assert_eq!(Rc::strong_count(&captured), 2, "闭包持有捕获物");

        handle.dispose();
        assert_eq!(
            Rc::strong_count(&captured),
            1,
            "销毁后 effect 体必须被释放（否则长跑 SPA 内存线性增长）"
        );
    }
}
