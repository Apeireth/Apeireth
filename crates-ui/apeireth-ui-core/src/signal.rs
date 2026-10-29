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
//!
//! ## 失败语义：写路径永不 panic
//!
//! wasm 侧 **panic = 整页 abort** ⇒ 写失败（派生期写入 / 自激环 / 读改写冲突）
//! 一律以 `Err` 返回给 `try_*` 变体；「无条件」变体（[`Signal::set`] /
//! [`Signal::update`] / [`Signal::set_if_changed`]）把错误**记录到运行期错误槽**
//! （[`crate::Runtime::take_flush_error`] 可取走）后继续执行 ⇒ 不 abort 页面。
//!
//! ## 代价与语义（写之前先知道）
//!
//! - [`Signal::get`] **每次都克隆一份值**（`T: Clone`）⇒ 大对象（`Vec` / `String`）
//!   的读取是 O(n) 拷贝；高频路径请用 [`Signal::read_only`] + 就地消费，或把
//!   派生值改成引用型包装。
//! - [`Signal::try_set_if_changed`] 用 `PartialEq` 判等 ⇒ `Vec` 是**深比较**
//!   （每次 O(n)）；`f64::NAN != NAN` ⇒ **NaN 永远判「变了」**（每次都触发）。
//!   需要自定义判等时用 [`Signal::try_set_if_changed_with`]。

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
    ///
    /// 写入成功后向**宿主调度器**请求一次 flush（设计文档 §4.2 规则 1）；
    /// 若该次 flush 失败（如自激循环），错误在此返回（**不许静默降级**）。
    ///
    /// 返回的错误只可能是**本次写入期间**产生的（按 flush 代次隔离）⇒ 上一次
    /// flush 留下的陈旧错误不会污染本次（与该错误毫无关系的）写入。
    pub fn try_set(&self, value: T) -> Result<(), ReactiveError> {
        self.check_writable()?;
        // 记下「本次写入开始时」的 flush 代次 ⇒ 只取代次更新的失败。
        let gen_before = self.inner.flush_gen.get();
        *self.cell.value.borrow_mut() = value;
        self.bump_version();
        // 失效传播（可能检出自激环 ⇒ Err；**其余节点照常标脏**，失败不中断整轮）。
        let propagation = self.inner.propagate_dirty(self.id);
        self.inner.request_flush();
        // 本次写入期间的 flush 失败（代次隔离 ⇒ 陈旧错误不污染本次写）。
        let flush_error = self.inner.error_after(gen_before);
        match (propagation, flush_error) {
            (Err(err), _) => Err(err),
            (_, Some(err)) => Err(err),
            (Ok(()), None) => Ok(()),
        }
    }

    /// 写信号（**不 panic**）：失败时把错误记入运行期错误槽后继续。
    ///
    /// 失败的可观测出口：[`crate::Runtime::take_flush_error`]。
    /// 需要在调用点直接拿到错误时用 [`Signal::try_set`]。
    pub fn set(&self, value: T) {
        if let Err(err) = self.try_set(value) {
            // 失败必须能被回传（不许静默降级）：记入运行期错误槽。
            self.inner.record_error(err);
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

    /// 等值不触发（**自定义相等判定**）：相等时不失效、不传播，返回 `false`。
    ///
    /// 为什么需要：`PartialEq` 的代价与语义都可能不合适 ——
    /// - `Vec` 的 `PartialEq` 是**深比较**（每次 O(n)）；
    /// - `f64::NAN != NAN` ⇒ NaN **永远**判「变了」（每次都触发失效）；
    /// - 业务上常需要「按业务键相等」（例如按 id 比较两条记录）。
    ///
    /// 本方法把判等交给调用方 ⇒ 代价与语义都可控。
    pub fn try_set_if_changed_with(
        &self,
        value: T,
        eq: impl Fn(&T, &T) -> bool,
    ) -> Result<bool, ReactiveError> {
        let unchanged = {
            let current = self.cell.value.borrow();
            eq(&current, &value)
        };
        if unchanged {
            return Ok(false);
        }
        self.try_set(value)?;
        Ok(true)
    }

    /// 等值不触发（自定义相等判定；**不 panic**）：失败时把错误记入运行期错误槽。
    pub fn set_if_changed_with(&self, value: T, eq: impl Fn(&T, &T) -> bool) -> bool {
        match self.try_set_if_changed_with(value, eq) {
            Ok(changed) => changed,
            Err(err) => {
                self.inner.record_error(err);
                false
            }
        }
    }
}

impl<T: Clone> Signal<T> {
    /// 读信号：在追踪域内会登记依赖边。
    ///
    /// **代价**：每次都**克隆一份值**（`T: Clone`）⇒ `Vec` / `String` 这类大对象
    /// 的读取是 O(n) 拷贝；高频路径请用只读视图并就地消费。
    pub fn get(&self) -> T {
        self.inner.track(self.id);
        self.cell.value.borrow().clone()
    }

    /// 读改写：把**旧值**交给闭包，闭包返回**新值**。
    ///
    /// 语义（为什么不是 `&mut T`）：
    /// - 旧值先被**取出来并释放借用** ⇒ 闭包里再读本信号**不会**二次借出
    ///   （`RefCell` 的双重借出是 panic ⇒ wasm 侧整页 abort）；
    /// - 闭包拿到的是**旧值快照** ⇒ 闭包期间对本信号的另一次写入会被闭包的
    ///   返回值覆盖 ⇒ 这种情况下本方法**显式拒绝**
    ///   （[`ReactiveError::ConcurrentWrite`]），不静默丢更新。
    ///
    /// 派生求值期间写入会返回错误（见 [`ReactiveError::MutationDuringDerivation`]）。
    pub fn try_update(&self, f: impl FnOnce(T) -> T) -> Result<(), ReactiveError> {
        self.check_writable()?;
        let gen_before = self.inner.flush_gen.get();
        let version_before = self.cell.version.get();
        // 先取出旧值并**释放借用** ⇒ 闭包里再读本信号是安全的。
        let old = { self.cell.value.borrow().clone() };
        let new = f(old);
        // 闭包期间若有人又写了本信号 ⇒ 本次读改写会静默覆盖那次写 ⇒ 显式拒绝。
        if self.cell.version.get() != version_before {
            return Err(ReactiveError::ConcurrentWrite { node: self.id });
        }
        *self.cell.value.borrow_mut() = new;
        self.bump_version();
        let propagation = self.inner.propagate_dirty(self.id);
        self.inner.request_flush();
        let flush_error = self.inner.error_after(gen_before);
        match (propagation, flush_error) {
            (Err(err), _) => Err(err),
            (_, Some(err)) => Err(err),
            (Ok(()), None) => Ok(()),
        }
    }

    /// 读改写（**不 panic**）：失败时把错误记入运行期错误槽后继续。
    pub fn update(&self, f: impl FnOnce(T) -> T) {
        if let Err(err) = self.try_update(f) {
            self.inner.record_error(err);
        }
    }
}

impl<T: Clone + PartialEq> Signal<T> {
    /// 等值不触发：值相等时不失效、不传播，返回 `false`。
    ///
    /// **代价与语义**（用前先知道）：
    /// - 判等走 `PartialEq` ⇒ `Vec` 是**深比较**（每次 O(n)）；
    /// - `f64::NAN != NAN` ⇒ **NaN 永远判「变了」**（每次都触发失效传播）。
    ///
    /// 需要自定义判等时用 [`Signal::try_set_if_changed_with`]。
    pub fn try_set_if_changed(&self, value: T) -> Result<bool, ReactiveError> {
        self.try_set_if_changed_with(value, |current, incoming| current == incoming)
    }

    /// 等值不触发（**不 panic**）：失败时把错误记入运行期错误槽，返回 `false`。
    pub fn set_if_changed(&self, value: T) -> bool {
        match self.try_set_if_changed(value) {
            Ok(changed) => changed,
            Err(err) => {
                self.inner.record_error(err);
                false
            }
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
    ///
    /// **代价**：每次都克隆一份值（同 [`Signal::get`]）。
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
        value.update(|v| v + 1);
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

    // ── 判别测试（钉行为不钉字面）──────────────────────────────────────────

    /// read-after-write 可见性：写完**立刻**读，必须是新值（不等 flush）。
    #[test]
    fn read_after_write_returns_the_new_value_without_a_flush() {
        let runtime = Runtime::new();
        let value = runtime.signal(1u32);
        value.set(2);
        assert_eq!(value.get(), 2, "写后读必须立刻可见");
        assert_eq!(value.version(), 1);
    }

    /// 同一追踪域内**重复读**同一个信号 ⇒ 只登记一条依赖边（订阅去重）。
    #[test]
    fn repeated_reads_in_one_effect_register_one_dependency() {
        let runtime = Runtime::new();
        let value = runtime.signal(1u32);

        let v = value.clone();
        let handle = runtime.effect(move || {
            let _ = v.get();
            let _ = v.get();
            let _ = v.get();
        });
        runtime.flush();

        assert_eq!(
            runtime.graph().dependencies_of(handle.id()),
            vec![value.id()],
            "重复读只登记一条边"
        );
        assert_eq!(
            runtime.graph().dependents_of(value.id()),
            vec![handle.id()],
            "反向表同样去重"
        );
    }

    /// 多次重跑不得让依赖边**累积**（每次重跑先清旧边）。
    #[test]
    fn repeated_runs_do_not_accumulate_dependency_edges() {
        let runtime = Runtime::new();
        let value = runtime.signal(1u32);
        let other = runtime.signal(2u32);

        let v = value.clone();
        let o = other.clone();
        let handle = runtime.effect(move || {
            let _ = v.get();
            let _ = o.get();
        });

        for _ in 0..5 {
            runtime.flush();
            value.set(value.get() + 1);
        }
        runtime.flush();

        assert_eq!(
            runtime.graph().dependencies_of(handle.id()).len(),
            2,
            "依赖边数必须恒定（不得累积）"
        );
        assert_eq!(runtime.graph().edge_count(), 2);
    }

    /// 只读视图与可写信号共享**同一份**状态（不是快照）。
    #[test]
    fn read_only_view_tracks_later_writes() {
        let runtime = Runtime::new();
        let value = runtime.signal(1u32);
        let view = value.read_only();
        value.set(2);
        assert_eq!(view.get(), 2);
        assert_eq!(view.version(), value.version());
    }

    // ── 本批重构新增的判别测试 ─────────────────────────────────────────────

    /// 读改写闭包里**读同一个信号** ⇒ 不得 panic（旧的 `borrow_mut` 双重借出）。
    #[test]
    fn update_closure_may_read_the_same_signal() {
        let runtime = Runtime::new();
        let value = runtime.signal(10u32);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            value.update(|old| {
                // 闭包里读同一信号：旧实现会二次借出 ⇒ RefCell panic ⇒ 整页 abort。
                let _ = value.get();
                old + 1
            });
        }));
        assert!(result.is_ok(), "闭包里读同一信号不得 panic");
        assert_eq!(value.get(), 11, "读改写必须真的生效");
    }

    /// 读改写的 `try_` 变体在**读改写期间被再次写入**时返回 `Err`（不 panic）。
    #[test]
    fn try_update_reports_concurrent_write_instead_of_panicking() {
        let runtime = Runtime::new();
        let value = runtime.signal(1u32);

        let err = value
            .try_update(|old| {
                value.set(99);
                old + 1
            })
            .expect_err("并发写必须显式报错");
        assert_eq!(
            err,
            crate::ReactiveError::ConcurrentWrite { node: value.id() }
        );
        assert_eq!(value.get(), 99, "被拒绝的读改写不得覆盖那次写");
    }

    /// 自定义判等：NaN 按「业务相等」处理时不得触发失效。
    #[test]
    fn set_if_changed_with_supports_custom_equality() {
        let runtime = Runtime::new();
        let counter = Rc::new(Cell::new(0));
        let value = runtime.signal(f64::NAN);

        let c = counter.clone();
        let v = value.clone();
        runtime.effect(move || {
            c.set(c.get() + 1);
            let _ = v.get();
        });
        runtime.flush();
        assert_eq!(counter.get(), 1);

        // 默认判等：NaN != NaN ⇒ 永远判「变了」。
        assert!(value.set_if_changed(f64::NAN), "NaN 默认永远判「变了」");
        runtime.flush();
        assert_eq!(counter.get(), 2);

        // 自定义判等：把两个 NaN 视为相等 ⇒ 不触发。
        assert!(
            !value.set_if_changed_with(f64::NAN, |a, b| a.is_nan() && b.is_nan()),
            "自定义判等 ⇒ 不触发"
        );
        runtime.flush();
        assert_eq!(counter.get(), 2, "自定义判等 ⇒ 不重跑");
    }

    /// 派生求值期间的写：`try_set` 返回 `Err`；`set` **不 panic**（错误入槽）。
    #[test]
    fn set_during_derivation_does_not_panic() {
        let runtime = Runtime::new();
        let value = runtime.signal(0u32);
        let v = value.clone();
        let derived = runtime.derived(move || {
            v.set(1);
            0u32
        });

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| derived.get()));
        assert!(
            result.is_ok(),
            "写路径不得 panic（wasm 侧 panic = 整页 abort）"
        );
        assert_eq!(value.get(), 0, "被拒绝的写不得改值");
        assert!(
            runtime.take_flush_error().is_some(),
            "失败必须被记录（不许静默降级）"
        );
    }
}
