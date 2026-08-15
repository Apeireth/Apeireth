//! # apeireth-state
//!
//! **Apeireth R21 借鉴 Golutra #6: 9 Tauri state 模式转 TUI 等价物**
//! (per `analysis\golutra\BORROW_FROM_GOLUTRA.md` §8 P1 第 9/10 项 +
//! 主 2026-08-06 01:00 拍板 "只干 TUI").
//!
//! ## 借鉴背景
//!
//! Golutra v0.1.0 的 9 个 Tauri state 采用 `OnceLock<Arc<T>>` 全局 + 内部 `Mutex`
//! 共享模式 (`tauri::State<T>` 注入), 1:1 翻译到 ratatui 路线:
//!
//! | Golutra (Tauri 2) | 本 crate (TUI / ratatui) |
//! |---|---|
//! | `static STATE: OnceLock<Arc<T>> = OnceLock::new();` | [`OnceLockState<T>`] 模式 (lazy init 进程全局) |
//! | `state: tauri::State<Mutex<T>>` 注入 | [`MutexState<T>`] 模式 (`Arc<Mutex<T>>` 跨线程互斥) |
//! | `state: tauri::State<RwLock<T>>` 注入 | [`RwLockState<T>`] 模式 (`Arc<RwLock<T>>` 跨线程读写锁) |
//!
//! **1:1 翻译**:
//! - Golutra 9 个 `OnceLock<Arc<Mutex<...>>>` state → 本 crate 3 模式 trait + 3 模式具体类型
//! - Golutra `tauri::State<T>` 自动注入 → 本 crate `SharedState<T>` trait + 显式 `.with_lock()` / `.read()`
//! - Golutra 9 state 装配 (main.rs 启动时) → 本 crate [`OrganStateRegistry`] 9 器官聚合
//!
//! **不**抄 VCP / Golutra 业务代码 (per 借鉴 §0 哲学审计), **只**借鉴字段 + 行为模式.
//!
//! ## 3 模式 (跟 Golutra 9 Tauri state 1:1 对应)
//!
//! 1. **[`OnceLockState<T>`]** — 进程全局, 启动时 init 一次, 后续只读 clone
//!    - 借鉴 Golutra `OnceLock<Arc<T>>` 单例
//!    - 适用: 启动后只读的配置 / 路由表 / capability list
//! 2. **[`MutexState<T>`]** — 跨线程互斥, 同一时刻 1 writer 或 1 reader
//!    - 借鉴 Golutra `state: tauri::State<Mutex<T>>` 注入
//!    - 适用: 计数器 / LLM call 频率 / 工具调用历史
//! 3. **[`RwLockState<T>`]** — 跨线程读写锁, 1 writer 或 N readers
//!    - 借鉴 Golutra `state: tauri::State<RwLock<T>>` 注入
//!    - 适用: 内存历史 / 6 哲学锚 / organ state (读多写少)
//!
//! ## 9 器官 state 共享 (跟借鉴 Golutra #1 集成)
//!
//! 借鉴 Golutra #1 (9 器官 TUI command 模式, 1 owner 完成) 已在
//! `crates/apeireth-tui/src/organ/command/` 实现 9 organ State.
//! 本 crate 提供 9 器官 state 共享框架 ([`OrganStateRegistry`]),
//! 9 organ State 字段 (heart / brain / hand / eye / ear / memory / voice / body / mind)
//! 编译期 hardcode, 改 1 器官 = 改 1 字段 + 1 match arm.
//!
//! **集成方式** (per 借鉴 §0.3 中央 AI 主体性):
//! - `apeireth-tui` 的 `organ::command::heart::State` 等 9 类型 **保留为内部细节**
//! - 本 crate [`OrganStateRegistry`] 的 9 字段是 **新**的共享框架入口
//! - 集成由 R25.3 续做 (在 TUI `app.rs` LOCKED 边界外, 加 1 行 `let shared = OrganStateRegistry::new();`)
//!
//! ## 公开 API (100% 文档化)
//!
//! | 模块 | 公开类型 | 用途 |
//! |---|---|---|
//! | [`organ`] | `Organ` (9 变体) / `ORGAN_COUNT` / `OrganStub` 系列 (9 类型) | 9 器官编译期 hardcode |
//! | [`shared_state`] | `SharedState<T>` trait / `SharedStateMode` (3 变体) | 3 模式抽象 + 模式选择 |
//! | [`mode_once_lock`] | `OnceLockState<T>` / `OnceLockStateInit<T>` | 模式 1: 进程全局 lazy init |
//! | [`mode_mutex`] | `MutexState<T>` / `MutexStateInit<T>` | 模式 2: 跨线程互斥 |
//! | [`mode_rw_lock`] | `RwLockState<T>` / `RwLockStateInit<T>` | 模式 3: 跨线程读写锁 |
//! | [`registry`] | `OrganStateRegistry` (9 字段) / `OrganStateRegistry::new()` | 9 器官 state 聚合 |
//! | [`error`] | `StateError` (5 variant) / `StateErrorKind` | 错误类型 + 序列化摘要 |
//!
//! ## 6 哲学锚穿透 (per APEIRETH-CONVENTIONS §9)
//!
//! - **S-1 北极星导向** — 9 器官 state 服务 ASI 北极星 (heart 60Hz / brain LLM / mind 6 哲学锚 1:1 镜像)
//! - **S-2 实事求是** — 3 模式 trait + 3 模式具体类型全部 stub impl (R21 续真接), 0 假装 async/Tokio
//! - **O-2 走在前人肩上** — 借鉴 Golutra 9 Tauri state 模式, 借 `std::sync::{Mutex, RwLock, OnceLock}` 业界标准
//! - **O-3 干到底** — 9 器官 × 3 模式 = 27 hardcode, 25+ 集成测试, 1 完整例子
//! - **O-4 任何人都能接手** — 7 src 模块 + 1 example + 1 tests + 顶部 §0-§10 完整
//! - **O-5 不假装** — 3 模式全部 stub impl, 标 `// TODO R21: 真接 tokio::sync::Mutex`, 0 编造"已实现 async"
//!
//! ## 8 项不修改承诺 (per APEIRETH-CONVENTIONS §10 + 8-locked-unified §2)
//!
//! 8 项详见 [`docs/stage4/8-locked-unified-2026-08-05.md`](https://github.com/apeireth/apeireth-rust/blob/main/docs/stage4/8-locked-unified-2026-08-05.md) §2 (本指南统一版).
//!
//! | # | 承诺 | 本 crate 守门 |
//! |---|------|--------------|
//! | 1 | 阶段 1+2+3 LOCKED 文档 | ✅ 不动 |
//! | 2 | v2 / v4 / v4.1 LOCKED | ✅ 不动 |
//! | 3 | 阶段 4 主文档 LOCKED (6ca80776) | ✅ 不动 |
//! | 4 | 阶段 5 施工文档 LOCKED (631 行) | ✅ 不动 |
//! | 5 | v6 基础架构 (4 重守门 + 权限发放 + E 层修改路径) | ✅ 不动 |
//! | 6 | R11 baseline 三值 (V1141/V1131/V1136) | ✅ 不动 |
//! | 7 | 顶层 3 规范文件 (CONVENTIONS/VERSIONING/GLOSSARY) | ✅ 不动 |
//! | 8 | workspace version 1.0.0 (semver 严格) | ✅ 不动 (本 crate 0.1.0 + workspace.members 新增) |
//!
//! **0 触碰 24 LOCKED crate** (per `git diff`):
//! - `apeireth-tui/` LOCKED (8 src files mtime 16:34:11 baseline 严守, 0 改)
//! - 23 其他 LOCKED crate 0 触碰
//! - workspace `[workspace.package] version = "1.0.0"` 0 改
//!
//! ## 状态
//!
//! ⚠️ **skeleton (R21 借鉴 Golutra #6 估补, 主 2026-08-06 01:55 派活)**.
//! 3 模式 trait + 3 模式具体类型全部 stub impl, 9 器官 state 注册表 1:1 编译期 hardcode.
//! R21+ 续做: 真接 tokio::sync::Mutex / 真接 9 organ State 类型 (跟借鉴 #1 sister 报告集成) /
//! 真接 `apeireth-tui` app.rs 装配 (LOCKED 边界外, 加 1 行).
//!
//! ## 引用文档 (4 份)
//!
//! 1. `analysis\golutra\BORROW_FROM_GOLUTRA.md` §8 P1 第 9/10 项 (借鉴优先级 P1)
//! 2. `.openclaw\workspace\promethean\Apeireth-rust\reports\organ-command-borrow-golutra-report-2026-08-06.md` (借鉴 #1 sister 报告, 9 organ TUI command 模式)
//! 3. `.openclaw\workspace\promethean\Apeireth-rust\docs\stage4\8-locked-unified-2026-08-05.md` §2 (8 项不修改承诺唯一引用源)
//! 4. `.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-tui\src\organ\command\mod.rs` (借鉴 #1 sister 9 organ State, 0 改, **仅参考**)

#![warn(missing_docs)]
#![deny(unsafe_code)]
// sync 框架: 0 引 tokio (留 R21+ 续做 async 集成), 0 引 NewAPI, 0 引 parking_lot (用 std::sync 守边),
//            0 引 pyo3/qt/GDI (per 主 2026-08-06 01:00 拍板"纯 Rust")

// ============================================================================
// 子模块 (7 个, 每个 100% 文档化)
// ============================================================================

/// 9 器官 enum + 9 OrganStub 类型 (编译期 hardcode).
pub mod organ;
// R177: organ invariants (5 tests + 2 Kani)
mod organ_kani_proofs;
/// 错误类型 (5 variant StateError + 序列化摘要 StateErrorKind).
pub mod error;
/// SharedState trait + SharedStateMode (3 变体) — 3 模式抽象.
pub mod shared_state;
/// 模式 1: `OnceLockState<T>` 进程全局 lazy init.
pub mod mode_once_lock;
/// 模式 2: `MutexState<T>` 跨线程互斥.
pub mod mode_mutex;
/// 模式 3: `RwLockState<T>` 跨线程读写锁.
pub mod mode_rw_lock;
/// 9 器官 state 共享注册表 (9 字段, 1:1 跟借鉴 #1 sister 报告 9 organ 对齐).
pub mod registry;
// R150 P1 #8: XState-style statechart 引擎 (借鉴 statelyco/xstate 28K stars)
pub mod statechart;

// ============================================================================
// 公共 re-export (顶层级 API, 不需要 `apeireth_state::organ::Organ`)
// ============================================================================

pub use crate::error::{StateError, StateErrorKind, STATE_ERROR_VARIANT_COUNT};
pub use crate::mode_mutex::{MutexState, MutexStateInit, MutexStateMode};
pub use crate::mode_once_lock::{OnceLockState, OnceLockStateInit, OnceLockStateMode};
pub use crate::mode_rw_lock::{RwLockState, RwLockStateInit, RwLockStateMode};
pub use crate::organ::{
    BodyStub, BrainStub, EarStub, EyeStub, HandStub, HeartStub, MemoryStub, MindStub, Organ,
    ORGAN_ASCII_CHARS, ORGAN_COUNT, ORGAN_NAMES_ZH, VoiceStub,
};
pub use crate::registry::{
    OrganStateRegistry, OrganStateRegistryBuilder, REGISTRY_ORGAN_COUNT,
};
pub use crate::shared_state::{SharedState, SharedStateMode};

// ============================================================================
// 编译期 hardcode (5 项, 跨模块共享守门)
// ============================================================================

/// **Hardcode #1**: 平台名 (K-1 必含, per supervisor-prompt-818 §5.3 模式).
///
/// 跨 crate 通信用 platform 字段标识来源, 防 m3 幻觉把别平台的 state 调进来.
pub const PLATFORM_NAME: &str = "apeireth";

/// **Hardcode #2**: Schema 版本号 (向前兼容字段, R21+ 改格式时 bump).
pub const APEIRETH_STATE_SCHEMA_VERSION: &str = "1";

/// **Hardcode #3**: Golutra v0.1.0 借鉴的 Tauri state 数 (9 个, 1:1 翻译源).
pub const BORROWED_GOLUTRA_STATE_COUNT: usize = 9;

/// **Hardcode #4**: 3 模式 state 类型总数 (OnceLock / Mutex / RwLock).
pub const STATE_MODE_COUNT: usize = 3;

/// **Hardcode #5**: StateError variant 总数 (5 variant, 跟 PIPELINE_G5 6 variant 对齐风格).
pub const STATE_ERROR_COUNT: usize = 5;

/// 编译期字符串相等比较 (per std::str::eq 不是 const-stable, 自实现字节比较).
const fn const_str_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    let mut i = 0;
    while i < ab.len() {
        if ab[i] != bb[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// 编译期守门: PLATFORM_NAME == "apeireth" (K-1 强校验).
const _: () = assert!(const_str_eq(PLATFORM_NAME, "apeireth"));
/// 编译期守门: APEIRETH_STATE_SCHEMA_VERSION == "1".
const _: () = assert!(const_str_eq(APEIRETH_STATE_SCHEMA_VERSION, "1"));
/// 编译期守门: BORROWED_GOLUTRA_STATE_COUNT == 9.
const _: () = assert!(BORROWED_GOLUTRA_STATE_COUNT == 9);
/// 编译期守门: STATE_MODE_COUNT == 3.
const _: () = assert!(STATE_MODE_COUNT == 3);
/// 编译期守门: STATE_ERROR_COUNT == 5.
const _: () = assert!(STATE_ERROR_COUNT == 5);
/// 编译期守门: ORGAN_COUNT == 9.
const _: () = assert!(ORGAN_COUNT == 9);
/// 编译期守门: REGISTRY_ORGAN_COUNT == 9.
const _: () = assert!(REGISTRY_ORGAN_COUNT == 9);
/// 编译期守门: STATE_ERROR_VARIANT_COUNT == 5.
const _: () = assert!(STATE_ERROR_VARIANT_COUNT == 5);
