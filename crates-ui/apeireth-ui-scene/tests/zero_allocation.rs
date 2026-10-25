//! ⭐ **每帧零堆分配**的**实测**门禁（分配计数探针）。
//!
//! ## 为什么这个测试住在 `tests/` 而不是 `src/`
//!
//! 计数用的全局分配器必须写 `unsafe impl GlobalAlloc` ⇒ 而本 crate 的
//! `src/lib.rs` 有 **`#![forbid(unsafe_code)]`** ⇒ 探针**不能**放进库源码。
//!
//! ⇒ 集成测试是**独立的 crate** ⇒ `forbid` 不作用于它 ⇒ 探针放这里。
//! （且 `#[global_allocator]` 只作用于**本测试二进制** ⇒ 不影响其它目标。）
//!
//! ## 口径
//!
//! - 计数器是**线程局部**的（`const` 初始化 ⇒ 访问它本身**不分配** ⇒ 分配器
//!   里访问它不会递归）⇒ 其它测试线程的分配**不会**污染本测量；
//! - 测量的是**稳态**（先预热若干轮，让值集合的缓冲扩容到位）⇒ 之后每帧
//!   写入必须**零次分配**；
//! - 同时钉住**反例**：诊断视图（[`SceneUniforms::entries_f32`]）**确实**
//!   会分配 ⇒ 「零分配」是**改出来的**，不是本来就成立。

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use apeireth_ui_gpu::UniformValues;
use apeireth_ui_scene::SceneUniforms;

thread_local! {
    /// 本线程的分配计数。
    ///
    /// ⚠ `const` 初始化 ⇒ 访问它**不分配** ⇒ 分配器里访问它不会递归。
    static COUNT: Cell<u64> = const { Cell::new(0) };
}

/// 本线程至今的分配次数。
fn count() -> u64 {
    COUNT.with(|counter| counter.get())
}

/// 计数用的全局分配器（转发给系统分配器，只多记一次数）。
struct Counting;

// SAFETY: 纯转发 ⇒ 契约与 `System` 完全相同（分配 / 释放 / 重新分配的
// 尺寸与对齐约束一字不改），只是多记一次数。
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        COUNT.with(|counter| counter.set(counter.get() + 1));
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        COUNT.with(|counter| counter.set(counter.get() + 1));
        unsafe { System.realloc(pointer, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// 一份取值（覆盖 16 个 uniform，其中两个是双分量）。
fn sample() -> SceneUniforms {
    SceneUniforms {
        res: [1440.0, 900.0],
        center: [950.4, 495.0],
        scale: 86.4,
        time: 1.2,
        rot: 0.35,
        bright: 0.32,
        turb: 0.12,
        pulse: 0.0,
        incl: 0.3,
        azim: 0.0,
        disk_azim: 0.0,
        disk_incl: 0.3,
        amb_str: 0.0,
        eclipse: 0.0,
        tl_bright: 1.0,
        tl_gold: 1.0,
    }
}

#[test]
fn the_per_frame_write_path_allocates_nothing() {
    // ⭐ 实测「每帧零堆分配」。
    let mut values = UniformValues::new();
    let uniforms = sample();

    // 预热：第一次写入会让值集合的缓冲扩容（**一次性**）⇒ 稳态才是零分配。
    for _ in 0..8 {
        uniforms.write_into_values(&mut values).expect("写入成功");
    }

    let before = count();
    for _ in 0..512 {
        uniforms.write_into_values(&mut values).expect("写入成功");
    }
    let delta = count() - before;
    assert_eq!(
        delta, 0,
        "每帧写入必须**零堆分配**（512 轮实测 {delta} 次分配）"
    );
}

#[test]
fn the_diagnostic_entries_path_does_allocate() {
    // ⭐ 反例钉住「零分配」不是**本来就**成立：诊断视图确实会分配。
    // （若哪天它也不分配了，本测试会红 ⇒ 说明零分配已覆盖到诊断路径。）
    let uniforms = sample();
    for _ in 0..8 {
        let _ = uniforms.entries_f32();
    }
    let before = count();
    let _ = uniforms.entries_f32();
    let delta = count() - before;
    assert!(
        delta > 0,
        "诊断视图仍会分配（实测 {delta} 次）⇒ 热路径的零分配是**改出来的**"
    );
}
