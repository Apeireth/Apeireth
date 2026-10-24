//! # apeireth-ui-core — 响应式核心（信号 / 派生 / 依赖图 / 调度 / effect）
//!
//! 纯 Rust 响应式内核：**0 外部依赖**（含 0 依赖 web-sys）⇒ 永远能 native
//! 单测，不需浏览器、不需 wasm target（ADR-0015 §4 P1 验收门）。
//!
//! ## 职责边界（设计文档 §3.3.1）
//! 只回答「**什么时候、哪些东西需要重新算**」，不回答「算出来之后放到哪」
//! —— 那是 `ui-dom`（DOM 投影）与 `ui-gpu`（像素投影）的事，两条路平级。
//!
//! ## 引擎里不出现业务名词（设计文档 §1.3 第 12 条）
//! 本 crate 的公开类型里没有会话 / 审批 / presence 之类的概念。一旦出现，
//! 引擎就退化成「带业务的项目」，可复用性与可测试性同时崩掉。
//!
//! ## 交付口径（P1 建核，P3 补回调）
//!
//! ### ✅ 已实现
//! - **信号**：读登记依赖 / 写主动失效 / 版本号 / 只读视图 / 等值不触发；
//! - **派生**：惰性求值 + 主动失效 + 求值环检测（带人名路径）；
//! - **依赖图**：双向边 + 节点标签 + 条件依赖重收集；
//! - **调度**：内部待办双队列（pre / post）+ 去重 + 延迟队列；
//! - **宿主调度器注入**：[`HostScheduler`] trait + 同步默认实现
//!   ([`SyncScheduler`]) + 手动队列实现 ([`QueuedScheduler`])；**注入是 trait
//!   边界，不是依赖** ⇒ 本 crate 的 `[dependencies]` 段仍为 0 条；
//! - **effect**：生命周期 + 清理回调（逆序）+ 幂等销毁 + 作用域 LIFO 销毁；
//! - **批次**：多次写合并为一次 flush（可嵌套）；
//! - **实例引用槽位**：[`InstanceRef`]（`bind:this` 的语义内核：挂载写引用、
//!   卸载前清空）；
//! - **组件 trait 边界**：[`UiComponent`]；
//! - **组件回调**：[`Callback`]（类型擦除的 `Fn()`；`Clone` + `Debug` + 按指针
//!   身份比较 ⇒ 能当 props 字段放进宏生成的结构体）。
//!
//! ### 🔵 未接线（等什么）
//! - **wasm 侧微任务调度器**：`HostScheduler` 的 trait 边界已就位，但「把 flush
//!   排进浏览器微任务队列」的那一实现落在 **wasm 宿主侧**（`ui-dom` 的
//!   `web-sys` 后端 / 试点宿主）⇒ **本 crate 不含该实现**，等 P2 的真实 DOM
//!   后端一起接；在此之前 native 侧用同步默认实现。
//! - **DOM 写入批次挂载点**：flush 的「渲染前」队列（`effect_pre` 所在的那一
//!   段）就是 DOM 写入批次的落点，但**真正的 DOM 写入在 `ui-dom`** ⇒ 本 crate
//!   只提供队列与顺序保证，不含写入动作。
//! - **派生闭包的自引用环**：派生闭包若捕获**自身所在运行时的信号**，会形成
//!   `Rc` 引用环 ⇒ 运行时不被释放（P0 遗留的内存账，**已登记、未修**；修法是
//!   把信号对运行时的持有改成弱引用，属独立批次）。
//!
//! ## 契约登记
//! `docs/architecture/contract.toml` `[extension_points]`：
//! `ui_component_trait = "crates-ui/apeireth-ui-core/src/component.rs::UiComponent"`。
//! 命名约束：trait 名**不得**叫 `Component`（全仓已被图像家族两处占用）。

#![forbid(unsafe_code)]

pub mod callback;
pub mod component;
pub mod derived;
pub mod effect;
pub mod error;
pub mod graph;
pub mod host_scheduler;
pub mod instance;
pub mod runtime;
pub mod scheduler;
pub mod signal;

pub use callback::Callback;
pub use component::UiComponent;
pub use derived::Derived;
pub use effect::{EffectHandle, Scope};
pub use error::ReactiveError;
pub use graph::{DependencyGraph, NodeId};
pub use host_scheduler::{HostScheduler, QueuedScheduler, SyncScheduler};
pub use instance::InstanceRef;
pub use runtime::{FlushReport, Runtime, MAX_NODE_RUNS_PER_FLUSH};
pub use scheduler::Scheduler;
pub use signal::{ReadSignal, Signal};

use std::cell::RefCell;

thread_local! {
    /// 线程本地的默认运行时：模块级自由函数的落点。
    ///
    /// 显式 [`Runtime`] 才是推荐用法（测试可各自持有独立运行时）；自由函数只是
    /// 语法糖，语义与显式运行时完全一致。
    static DEFAULT_RUNTIME: RefCell<Option<Runtime>> = const { RefCell::new(None) };
}

/// 取线程本地的默认运行时（不存在则建一个）。
fn default_runtime() -> Runtime {
    DEFAULT_RUNTIME.with(|slot| {
        let mut guard = slot.borrow_mut();
        if guard.is_none() {
            *guard = Some(Runtime::new());
        }
        guard
            .as_ref()
            .expect("默认运行时：初始化后必定存在")
            .clone()
    })
}

/// 建一个信号（默认运行时）。
pub fn signal<T: Clone>(value: T) -> Signal<T> {
    default_runtime().signal(value)
}

/// 建一个派生值（默认运行时）。
pub fn derived<T: Clone + 'static>(compute: impl Fn() -> T + 'static) -> Derived<T> {
    default_runtime().derived(compute)
}

/// 建一个 effect（默认运行时，渲染后执行）。
pub fn effect<F: FnMut() + 'static>(f: F) -> EffectHandle {
    default_runtime().effect(f)
}

/// 建一个渲染前 effect（默认运行时）。
pub fn effect_pre<F: FnMut() + 'static>(f: F) -> EffectHandle {
    default_runtime().effect_pre(f)
}

/// 批次：把多次写入合并成一次 flush（默认运行时）。
pub fn batch<F: FnOnce()>(f: F) {
    default_runtime().batch(f);
}

/// 同步跑完当前所有待办（默认运行时；测试用）。
pub fn flush_now() -> FlushReport {
    default_runtime().flush()
}

#[cfg(test)]
mod tests {
    //! crate 级断言：只放**跨模块**的组合行为；单模块行为断言在各自实现文件内
    //! （测试与被测实现同处一地 ⇒ 证据就近，模块拆分时测试不会漂）。

    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn free_functions_share_one_thread_local_runtime() {
        let counter = Rc::new(Cell::new(0));
        let value = signal(1u32);

        let c = counter.clone();
        let v = value.clone();
        effect(move || {
            c.set(c.get() + 1);
            let _ = v.get();
        });

        flush_now();
        assert_eq!(counter.get(), 1);

        value.set(2);
        flush_now();
        assert_eq!(counter.get(), 2, "自由函数与显式运行时语义一致");
    }

    #[test]
    fn crate_level_reexports_are_reachable() {
        // 门面 crate 依赖本 crate 的这些具名导出（跨 crate 契约面）。
        let runtime: Runtime = Runtime::new();
        let sig: Signal<u32> = runtime.signal(3);
        let view: ReadSignal<u32> = sig.read_only();
        let node: NodeId = sig.id();
        let derived: Derived<u32> = runtime.derived(move || view.get());
        let handle: EffectHandle = runtime.effect(|| {});
        let scope: Scope = runtime.scope();
        let err: ReactiveError = ReactiveError::DisposedScope;

        assert_eq!(derived.get(), 3);
        assert_eq!(node, sig.id());
        assert_eq!(handle.runs(), 0);
        assert!(!scope.is_disposed());
        assert_eq!(err, ReactiveError::DisposedScope);
    }

    #[test]
    fn host_scheduler_types_are_reachable_from_the_crate_root() {
        // 宿主调度器是**跨 crate 契约面**（wasm 侧注入）⇒ 必须从 crate 根部可达。
        let runtime = Runtime::new();
        let host = Rc::new(QueuedScheduler::new());
        runtime.set_host_scheduler(host.clone());

        let value = runtime.signal(0u32);
        value.set(1);
        assert_eq!(host.queued(), 1, "写入 ⇒ 向宿主请求一次 flush");
        assert_eq!(host.drain(), 1);
    }

    #[test]
    fn instance_ref_is_reachable_from_the_crate_root() {
        let slot: InstanceRef<u32> = InstanceRef::new();
        assert!(!slot.is_bound());
        slot.bind(5);
        assert_eq!(slot.get(), Some(5));
    }
}
