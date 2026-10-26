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
//! - **运行时能力的两形态**：[`Runtime`]（所有者形态：可挂 effect / flush）
//!   与 [`ReactiveScope`]（使用者形态：只能建信号 / 派生）⇒ ⇒ 「状态与渲染
//!   同属一个 runtime」在**类型层面**可强制；
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
//!
//! ## 失败语义（wasm 侧 panic = 整页 abort ⇒ 引擎不得 abort 页面）
//!
//! - **写路径永不 panic**：`Signal::set` / `update` / `set_if_changed` 与
//!   `Runtime::flush` 遇到失败时，把错误**记录到运行期错误槽**
//!   （`Runtime::take_flush_error` 可取走）后继续执行；需要显式错误时用
//!   `try_*` 变体。
//! - **错误绑定到「本次写入期间」**：每次 flush 自增一个**代次**，写路径只取
//!   代次更新的错误 ⇒ 上一轮的陈旧错误不会污染下一次写入。
//! - **自激尽早判定**：同一次 flush 内被自己（或自己下游）的写入反复标脏的
//!   effect ⇒ 显式 `ReactiveError::CycleDetected`（带环路径），**不靠跑满次数
//!   上限才发现**；失败**不中断整轮调度**。
//! - **销毁即释放**：effect `dispose()` 会把 effect 体从运行时表里摘掉（闭包
//!   捕获的 DOM 节点 / 组件状态可被回收）；派生用**弱引用**登记，并提供
//!   `Derived::dispose()` 提前释放。
//!
//! ## 运行时能力的两形态（**架构不变量：状态与渲染同属一个 runtime**）
//!
//! 2026-10-25 实测到一个**架构级缺陷**：状态对象**自建**了一个 `Runtime`
//! ⇒ ⇒ 它的草稿 / 反馈信号**不属于**宿主挂 effect 的那个 runtime
//! ⇒ ⇒ **effect 根本不依赖这些信号** ⇒ ⇒ 改草稿**不触发重渲染**
//! ⇒ ⇒ 实测表现为「**幽灵输入**」（用户敲进去的值**引擎不知道**）。
//!
//! ⇒ ⇒ 2026-10-26 起，本 crate 用**两种能力形态**把这条不变量**落到类型层**：
//!
//! | 形态 | 类型 | 能力 | 谁拿 |
//! |---|---|---|---|
//! | **所有者形态** | [`Runtime`] | 建信号 / 派生 / **挂 effect** / flush / 注入调度器 | **应用根**（宿主入口） |
//! | **使用者形态** | [`ReactiveScope`] | **只**能建信号 / 派生 | **状态持有者** |
//!
//! ⇒ ⇒ 状态持有者**在类型层面**拿不到 `effect` / `flush`
//! ⇒ ⇒ 它**不可能**成为渲染链的另一端
//! ⇒ ⇒ 「信号与 effect 分属两个 runtime」这个**结构条件**被切断。
//!
//! ⇒ ⇒ 证明方式 = [`ReactiveScope`] 上的三个 `compile_fail` doc-test
//! （`ReactiveScope` 上没有 `effect` / `flush` / `new`）。
//!
//! ⚠️ **诚实登记**：[`Runtime::new`] 仍是 `pub`（native 测试 / 独立运行时 /
//! 既有 crate 的调用点不可改）⇒ ⇒ 「显式自建运行时」这条路径在**编译期**仍
//! 可写出来 ⇒ ⇒ 它由 `crates-ui/apeireth-ui-demo/harness/scripts/
//! check_runtime_ownership.py` 以**精确判据**守住（双保险）。
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
pub use runtime::{FlushReport, ReactiveScope, Runtime, MAX_NODE_RUNS_PER_FLUSH};
pub use scheduler::{Phase, Scheduler};
pub use signal::{ReadSignal, Signal};

use std::cell::RefCell;

thread_local! {
    /// 线程本地的默认运行时：模块级自由函数的落点。
    ///
    /// 显式 [`Runtime`] 才是推荐用法（测试可各自持有独立运行时）；自由函数只是
    /// 语法糖，语义与显式运行时完全一致。
    ///
    /// **口径**：本运行时**只给原型 / 测试用**。它是线程局部且**永不自动释放**
    /// （进程活多久，它就活多久）⇒ 长跑进程里它会一直持有信号 / 派生 / effect
    /// （内存只增不减）。需要干净起点时显式调用 [`reset_default_runtime`]。
    static DEFAULT_RUNTIME: RefCell<Option<Runtime>> = const { RefCell::new(None) };
}

/// 取线程本地的默认运行时（不存在则建一个）。
fn default_runtime() -> Runtime {
    DEFAULT_RUNTIME.with(|slot| {
        let mut guard = slot.borrow_mut();
        if guard.is_none() {
            *guard = Some(Runtime::new());
        }
        // 取不到只可能是「初始化时 panic」⇒ 那时进程已经在 unwind，不必再报。
        match guard.as_ref() {
            Some(runtime) => runtime.clone(),
            None => Runtime::new(),
        }
    })
}

/// 重置线程本地的默认运行时（丢弃它的全部状态）。
///
/// **为什么需要**：默认运行时是**常驻**的（线程局部 ⇒ 永不自动释放）⇒ 长跑
/// 进程里它持有的信号 / 派生 / effect 只增不减。原型与测试用它，但**测试之间**
/// 需要干净起点 ⇒ 本函数显式丢弃它（下次调用自由函数时重建）。
///
/// 显式 [`Runtime`] 的持有者不受影响（它们与本运行时无关）。
pub fn reset_default_runtime() {
    DEFAULT_RUNTIME.with(|slot| {
        *slot.borrow_mut() = None;
    });
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

    /// 线程本地的默认运行时可被**显式重置**（它是常驻的 ⇒ 内存只增不减）。
    #[test]
    fn default_runtime_can_be_reset() {
        let counter = Rc::new(Cell::new(0));
        {
            let c = counter.clone();
            effect(move || {
                c.set(c.get() + 1);
            });
        }
        // 故意**不 flush** ⇒ 旧运行时里留着待办。

        reset_default_runtime();

        // 重置后默认运行时是**新的**：旧运行时的待办不得再跑。
        flush_now();
        assert_eq!(
            counter.get(),
            0,
            "重置后旧运行时的待办不得再跑（否则它的 effect 还会跑）"
        );

        // 新运行时照常工作。
        {
            let c = counter.clone();
            effect(move || {
                c.set(c.get() + 1);
            });
        }
        flush_now();
        assert_eq!(counter.get(), 1, "重置后的默认运行时是干净的");
    }
}
