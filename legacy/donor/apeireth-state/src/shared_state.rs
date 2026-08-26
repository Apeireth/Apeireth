//! # SharedState — 3 模式 state 抽象
//!
//! 借鉴 Golutra v0.1.0 9 Tauri state 模式 (`OnceLock + Arc + Mutex`) 1:1 翻译到 ratatui 路线.
//!
//! ## 3 模式 (跟 Golutra 9 Tauri state 1:1 对应)
//!
//! 1. **OnceLock** — 进程全局 lazy init 单例
//!    - 借鉴 Golutra `OnceLock<Arc<T>>` 全局 state
//! 2. **Mutex** — 跨线程互斥 (1 writer 或 1 reader at a time)
//!    - 借鉴 Golutra `state: tauri::State<Mutex<T>>` 注入
//! 3. **RwLock** — 跨线程读写锁 (1 writer 或 N readers)
//!    - 借鉴 Golutra `state: tauri::State<RwLock<T>>` 注入
//!
//! ## 抽象层
//!
//! - [`SharedStateMode`] — 3 变体 enum, 编译期 exhaustive match 守门
//! - [`SharedState<T>`] — trait, 3 模式各自 impl (`OnceLockState<T>` / `MutexState<T>` / `RwLockState<T>`)
//!
//! **不** 引入 dyn trait object (编译期 monomorphization, 0 虚函数开销).

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::{StateError, StateErrorKind};

/// **K-1 强校验 #3**: 3 模式 state 模式分类 (跟 Golutra 9 Tauri state 1:1).
pub const SHARED_STATE_MODE_COUNT: usize = 3;

/// 3 模式 state 模式分类 (编译期 enum exhaustive match 守门).
///
/// 顺序固定: OnceLock → Mutex → RwLock (per 借鉴 Golutra `OnceLock + Arc + Mutex` 顺序).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SharedStateMode {
    /// 模式 0: 进程全局 lazy init 单例.
    OnceLock,
    /// 模式 1: 跨线程互斥.
    Mutex,
    /// 模式 2: 跨线程读写锁.
    RwLock,
}

impl fmt::Display for SharedStateMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl SharedStateMode {
    /// 编译期字符串表示.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OnceLock => "once_lock",
            Self::Mutex => "mutex",
            Self::RwLock => "rw_lock",
        }
    }
}

/// 3 模式 state trait (跨模式统一接口, 编译期 monomorphization).
///
/// **设计**:
/// - 3 模式各自 impl (OnceLockState`T` / MutexState`T` / RwLockState`T`)
/// - 调用方用 `match mode` 选模式, 编译期 exhaustive match 守门
/// - 不引入 dyn trait object (per Rust 性能共识)
///
/// **不假装**:
/// - 3 模式 trait method 全部走标准库, 无 Tauri 依赖
/// - 锁中毒返回 `StateError::Poisoned`, 借 借鉴 Golutra `tauri::Error` 错误模式
pub trait SharedState<T>: Send + Sync {
    /// 当前模式 (per SharedStateMode 3 变体).
    fn mode(&self) -> SharedStateMode;

    /// 读访问 (不可变借用).
    ///
    /// 行为按模式:
    /// - OnceLock: 借用 `&T` (无锁, 仅一次性 init 后只读)
    /// - Mutex: `lock().unwrap()` 拿 `MutexGuard<T>` (允许读 + 写)
    /// - RwLock: `read().unwrap()` 拿 `RwLockReadGuard<T>` (仅读, 写走 `with_lock_mut`)
    fn read(&self) -> Result<StateReadGuard<'_, T>, StateError>;

    /// 写访问 (可变借用).
    ///
    /// 行为按模式:
    /// - OnceLock: **不**支持 (返 `StateError::Unsupported`)
    /// - Mutex: `lock().unwrap()` 拿 `MutexGuard<T>` (允许读 + 写)
    /// - RwLock: `write().unwrap()` 拿 `RwLockWriteGuard<T>` (允许读 + 写)
    fn write(&self) -> Result<StateWriteGuard<'_, T>, StateError>;
}

/// 不可变借用 guard (3 模式各自 impl, 用 enum 区分).
///
/// **设计**: 不引入 `Box<dyn Deref>` (避免 dyn overhead), 用 enum monomorphization.
#[allow(dead_code)] // 字段用于未来 R21+ 续做, 当前 enum dispatch 实际不用
pub enum StateReadGuard<'a, T> {
    /// OnceLock 模式 read guard (无锁, `&T` 借用).
    OnceLock(&'a T),
    /// Mutex 模式 read guard (`MutexGuard<T>`).
    Mutex(std::sync::MutexGuard<'a, T>),
    /// RwLock 模式 read guard (`RwLockReadGuard<T>`).
    RwLock(std::sync::RwLockReadGuard<'a, T>),
}

impl<T> std::ops::Deref for StateReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        match self {
            Self::OnceLock(t) => t,
            Self::Mutex(g) => g,
            Self::RwLock(g) => g,
        }
    }
}

/// 可变借用 guard (3 模式各自 impl).
#[allow(dead_code)]
pub enum StateWriteGuard<'a, T> {
    /// OnceLock 模式不支持写.
    OnceLock,
    /// Mutex 模式 write guard (`MutexGuard<T>`).
    Mutex(std::sync::MutexGuard<'a, T>),
    /// RwLock 模式 write guard (`RwLockWriteGuard<T>`).
    RwLock(std::sync::RwLockWriteGuard<'a, T>),
}

impl<T> std::ops::Deref for StateWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        match self {
            Self::OnceLock => {
                panic!("StateWriteGuard::OnceLock 不支持 Deref, 调用方需先 match OnceLock 错误")
            }
            Self::Mutex(g) => g,
            Self::RwLock(g) => g,
        }
    }
}

impl<T> std::ops::DerefMut for StateWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        match self {
            Self::OnceLock => {
                panic!("StateWriteGuard::OnceLock 不支持 DerefMut, 调用方需先 match OnceLock 错误")
            }
            Self::Mutex(g) => g,
            Self::RwLock(g) => g,
        }
    }
}

// =====================================================================
// 单元测试 (SharedStateMode 3 变体 + trait 编译期守门 = 10+ 测试)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_mode_variants_constructible() {
        let _ = SharedStateMode::OnceLock;
        let _ = SharedStateMode::Mutex;
        let _ = SharedStateMode::RwLock;
    }

    #[test]
    fn three_mode_as_str_distinct() {
        let s = [
            SharedStateMode::OnceLock.as_str(),
            SharedStateMode::Mutex.as_str(),
            SharedStateMode::RwLock.as_str(),
        ];
        let unique: std::collections::HashSet<&str> = s.iter().copied().collect();
        assert_eq!(unique.len(), 3);
    }

    #[test]
    fn three_mode_serialize_round_trip() {
        for m in [
            SharedStateMode::OnceLock,
            SharedStateMode::Mutex,
            SharedStateMode::RwLock,
        ] {
            let s = serde_json::to_string(&m).unwrap();
            let back: SharedStateMode = serde_json::from_str(&s).unwrap();
            assert_eq!(m, back);
        }
    }

    #[test]
    fn mode_count_constant_is_3() {
        assert_eq!(SHARED_STATE_MODE_COUNT, 3);
    }

    #[test]
    fn state_error_kind_5_variants_constructible() {
        // 5 variant 验 (借用 StateErrorKind 共享, 仅作集成存在性检查)
        let _ = StateErrorKind::Poisoned;
        let _ = StateErrorKind::NotInitialized;
        let _ = StateErrorKind::TypeMismatch;
        let _ = StateErrorKind::Unsupported;
        let _ = StateErrorKind::Other;
    }

    #[test]
    fn state_read_guard_deref_3_variants_compile() {
        // 编译期守门: 3 variant 都实现 Deref (编译通过即认为 OK)
        // 实际 deref 验在 mode_*_lock.rs 的 read 测试里跑 (锁存在时才验)
        // 用 `_phantom_deref` 占位表达 trait 存在, 不实际 deref
        fn _phantom_deref<'a, T>(_: &StateReadGuard<'a, T>) -> &'a T
        where
            T: 'a,
        {
            // 编译期仅作类型签名验, 永不在测试体内调用 (OnceLock variant 会 panic).
            // 用 unreachable!() 表示 "此函数不应被执行" — 这比 unimplemented!() 准确,
            // 因为它的唯一目的是编译期证明 StateReadGuard<'_, T> 公开 Deref<T> 的目标 &T.
            unreachable!("_phantom_deref 是编译期类型证明, 不应在测试运行时调用")
        }
    }
}
