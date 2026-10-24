//! `apeireth-ui-core` 集成冒烟：响应式核心的**跨模块**行为。
//!
//! 单元测试在各自实现文件内（证据就近）；本文件只放跨模块组合断言。

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use apeireth_ui_core::{
    HostScheduler, InstanceRef, QueuedScheduler, ReactiveError, Runtime, SyncScheduler,
};

#[test]
fn signal_change_propagates_through_derived_into_effect() {
    let runtime = Runtime::new();
    let base = runtime.signal(2u32);
    let doubled = {
        let base = base.clone();
        runtime.derived(move || base.get() * 2)
    };

    let seen = Rc::new(RefCell::new(Vec::<u32>::new()));
    {
        let seen = seen.clone();
        let doubled = doubled.clone();
        runtime.effect(move || {
            seen.borrow_mut().push(doubled.get());
        });
    }

    runtime.flush();
    base.set(5);
    runtime.flush();

    assert_eq!(&*seen.borrow(), &[4, 10]);
}

#[test]
fn scope_disposal_stops_effect_reruns() {
    let runtime = Runtime::new();
    let value = runtime.signal(0u32);
    let scope = runtime.scope();

    let runs = Rc::new(Cell::new(0usize));
    {
        let runs = runs.clone();
        let value = value.clone();
        scope.effect(move || {
            runs.set(runs.get() + 1);
            let _ = value.get();
        });
    }

    runtime.flush();
    assert_eq!(runs.get(), 1);

    scope.dispose();
    value.set(1);
    runtime.flush();
    assert_eq!(runs.get(), 1, "作用域销毁后 effect 不得再跑");
}

#[test]
fn self_triggering_effect_is_reported_as_flush_cycle() {
    let runtime = Runtime::new();
    let value = runtime.signal(0u32);
    {
        let value = value.clone();
        runtime.effect(move || {
            let current = value.get();
            value.set(current + 1);
        });
    }

    let err = runtime.try_flush().expect_err("自激循环必须报错");
    assert!(
        matches!(err, ReactiveError::FlushCycle { .. }),
        "实测：{err:?}"
    );
}

// ── 跨模块组合：宿主调度器注入 × 依赖传播 × 批次 ─────────────────────────

/// 宿主调度器注入了「异步」实现 ⇒ flush 时机完全由宿主决定。
#[test]
fn injected_host_scheduler_controls_the_flush_timing() {
    let runtime = Runtime::new();
    let host = Rc::new(QueuedScheduler::new());
    runtime.set_host_scheduler(host.clone());

    let value = runtime.signal(0u32);
    let runs = Rc::new(Cell::new(0));
    {
        let runs = runs.clone();
        let value = value.clone();
        runtime.effect(move || {
            runs.set(runs.get() + 1);
            let _ = value.get();
        });
    }

    value.set(1);
    value.set(2);
    assert_eq!(runs.get(), 0, "宿主未推进 ⇒ 一次都不得跑");
    assert_eq!(host.queued(), 1, "两 次写合并成一次 flush 请求");

    host.drain();
    assert_eq!(runs.get(), 1, "宿主推进 ⇒ 恰好跑一次，且看到最终值");
}

/// 默认实现是同步的 ⇒ 写后当场重算（native 侧默认口径）。
#[test]
fn default_host_scheduler_is_synchronous() {
    let runtime = Runtime::new();
    let value = runtime.signal(0u32);
    let runs = Rc::new(Cell::new(0));
    {
        let runs = runs.clone();
        let value = value.clone();
        runtime.effect(move || {
            runs.set(runs.get() + 1);
            let _ = value.get();
        });
    }

    value.set(1);
    assert_eq!(runs.get(), 1);
}

/// 同步实现也可被显式注入（宿主自持策略时用）。
#[test]
fn sync_scheduler_can_be_injected_explicitly() {
    let runtime = Runtime::new();
    runtime.set_host_scheduler(Rc::new(SyncScheduler));
    let value = runtime.signal(0u32);
    let runs = Rc::new(Cell::new(0));
    {
        let runs = runs.clone();
        let value = value.clone();
        runtime.effect(move || {
            runs.set(runs.get() + 1);
            let _ = value.get();
        });
    }
    value.set(1);
    assert_eq!(runs.get(), 1);
}

/// 批次 × 宿主调度器：批次内的写只向宿主请求**一次** flush。
#[test]
fn batch_coalesces_writes_into_one_host_request() {
    let runtime = Runtime::new();
    let host = Rc::new(QueuedScheduler::new());
    runtime.set_host_scheduler(host.clone());

    let value = runtime.signal(0u32);
    runtime.batch(|| {
        value.set(1);
        value.set(2);
        value.set(3);
    });

    assert_eq!(host.queued(), 1, "整批只请求一次 flush");
    host.drain();
    assert!(host.is_empty());
}

/// 宿主注入点必须是 **trait 边界**：宿主可以自带一份实现而不碰引擎内部。
#[test]
fn a_host_can_implement_the_scheduler_boundary_itself() {
    struct CountingHost {
        calls: Rc<Cell<usize>>,
    }
    impl HostScheduler for CountingHost {
        fn schedule_flush(&self, flush: Box<dyn FnOnce()>) {
            self.calls.set(self.calls.get() + 1);
            flush();
        }
    }

    let runtime = Runtime::new();
    let calls = Rc::new(Cell::new(0));
    runtime.set_host_scheduler(Rc::new(CountingHost {
        calls: calls.clone(),
    }));

    let value = runtime.signal(0u32);
    value.set(1);
    assert_eq!(calls.get(), 1, "宿主实现被调用了一次");
}

/// 实例引用槽位（`bind:this` 语义内核）跨模块可用：挂载写、卸载清。
#[test]
fn instance_reference_round_trips_through_mount_and_unmount() {
    let slot: InstanceRef<String> = InstanceRef::new();
    assert!(!slot.is_bound());

    slot.bind(String::from("句柄"));
    assert!(slot.is_bound());
    assert_eq!(slot.get().as_deref(), Some("句柄"));

    slot.unbind();
    assert!(!slot.is_bound(), "卸载前必须清空");
}

/// 派生 × effect × 作用域：销毁作用域后派生不再被求值。
#[test]
fn disposing_a_scope_stops_derived_recomputation() {
    let runtime = Runtime::new();
    let base = runtime.signal(1u32);
    let calls = Rc::new(Cell::new(0));
    let scope = runtime.scope();

    let derived = {
        let base = base.clone();
        let calls = calls.clone();
        runtime.derived(move || {
            calls.set(calls.get() + 1);
            base.get() * 2
        })
    };

    scope.effect({
        let derived = derived.clone();
        move || {
            let _ = derived.get();
        }
    });
    runtime.flush();
    assert_eq!(calls.get(), 1);

    scope.dispose();
    base.set(2);
    runtime.flush();
    assert_eq!(calls.get(), 1, "作用域销毁后派生不再被求值");
}

/// 条件分支下的依赖重收集（跨模块：effect → 依赖图）。
#[test]
fn conditional_dependency_edges_are_rebuilt_on_branch_switch() {
    let runtime = Runtime::new();
    let toggle = runtime.signal(true);
    let left = runtime.signal(1u32);
    let right = runtime.signal(2u32);

    let handle = {
        let toggle = toggle.clone();
        let left = left.clone();
        let right = right.clone();
        runtime.effect(move || {
            let _ = if toggle.get() {
                left.get()
            } else {
                right.get()
            };
        })
    };
    runtime.flush();
    let deps = runtime.graph().dependencies_of(handle.id());
    assert!(deps.contains(&left.id()), "当前分支的信号必须是依赖");
    assert!(!deps.contains(&right.id()), "未走的分支不得是依赖");

    toggle.set(false);
    runtime.flush();
    let deps = runtime.graph().dependencies_of(handle.id());
    assert!(!deps.contains(&left.id()), "旧分支的边必须被摘掉");
    assert!(deps.contains(&right.id()), "新分支的信号必须成为依赖");
}

/// 多个派生共享同一来源时，来源只登记一次依赖（订阅去重，跨模块）。
#[test]
fn shared_source_registers_one_edge_per_reader() {
    let runtime = Runtime::new();
    let base = runtime.signal(1u32);
    let seen = Rc::new(RefCell::new(Vec::<u32>::new()));

    let doubled = {
        let base = base.clone();
        runtime.derived(move || base.get() * 2)
    };

    let handle = {
        let doubled = doubled.clone();
        let seen = seen.clone();
        runtime.effect(move || {
            seen.borrow_mut().push(doubled.get());
        })
    };
    runtime.flush();

    assert_eq!(
        runtime.graph().dependents_of(base.id()),
        vec![doubled.id()],
        "来源的订阅方是派生本身（effect 依赖的是派生，不是来源）"
    );
    assert_eq!(
        runtime.graph().dependencies_of(handle.id()),
        vec![doubled.id()],
        "effect 只依赖派生一条边（重复读不累积）"
    );
    assert_eq!(&*seen.borrow(), &[2]);
}
