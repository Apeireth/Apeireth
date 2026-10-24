//! `apeireth-ui-core` 集成冒烟：响应式核心的**跨模块**行为。
//!
//! 单元测试在各自实现文件内（证据就近）；本文件只放跨模块组合断言。

use apeireth_ui_core::{ReactiveError, Runtime};

#[test]
fn signal_change_propagates_through_derived_into_effect() {
    let runtime = Runtime::new();
    let base = runtime.signal(2u32);
    let doubled = {
        let base = base.clone();
        runtime.derived(move || base.get() * 2)
    };

    let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u32>::new()));
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

    let runs = std::rc::Rc::new(std::cell::Cell::new(0usize));
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
