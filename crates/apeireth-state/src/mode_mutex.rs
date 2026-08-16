//! # MutexState — 模式 2: 跨线程互斥
//!
//! 借鉴 Golutra v0.1.0 `state: tauri::State<Mutex<T>>` 9 Tauri state 模式 1:1 翻译.
//!
//! ## 设计
//!
//! - `Arc<std::sync::Mutex<T>>` 持有 state, 跨线程共享
//! - `read` / `write` 都走 `Mutex::lock()` (Mutex 不区分读 / 写, 都是 exclusive lock)
//! - 锁中毒 (`Mutex::lock()` 返 `Err(Poisoned)`) 映射到 `StateError::Poisoned`
//!
//! ## 适用场景
//!
//! - 计数器 (heart 60Hz tick / brain LLM call 频率)
//! - 写多读少 state (hand 工具调用统计)
//! - 简单共享 (any organ, 写多场景)
//!
//! **不假装**:
//! - `Mutex::lock()` 在锁中毒时返 `Err`, 不 panic (per stdlib 行为)
//! - skeleton 阶段不引入 parking_lot (跟 stdlib 一致, 0 依赖扩张)

use std::sync::{Arc, Mutex, MutexGuard};

use serde::{Deserialize, Serialize};

use crate::error::StateError;
use crate::organ::Organ;
use crate::shared_state::{SharedState, SharedStateMode, StateReadGuard, StateWriteGuard};

/// **模式 2: MutexState 跨线程互斥** (per 借鉴 Golutra `state: tauri::State<Mutex<T>>`).
///
/// **设计**:
/// - 内部用 `Arc<Mutex<T>>` 持有 state
/// - `read` / `write` 都走 `Mutex::lock()` (exclusive)
/// - 锁中毒返 `StateError::Poisoned`
#[derive(Debug)]
pub struct MutexState<T>
where
    T: Send + 'static,
{
    /// 内部 `Arc<Mutex<T>>` (跨线程共享).
    inner: Arc<Mutex<T>>,
}

impl<T> MutexState<T>
where
    T: Send + 'static,
{
    /// 新建 MutexState (立即 init, value 包成 `Arc<Mutex<T>>`).
    pub fn new(value: T) -> Self {
        Self {
            inner: Arc::new(Mutex::new(value)),
        }
    }

    /// Get 内部 `Arc<Mutex<T>>` 的 clone (跨线程共享).
    pub fn handle(&self) -> Arc<Mutex<T>> {
        Arc::clone(&self.inner)
    }

    /// Try lock (非阻塞).
    ///
    /// 行为:
    /// - 拿到锁: 返 `Ok(MutexGuard<T>)`
    /// - 锁被占: 返 `Err(TryLockError::WouldBlock)`, 映射为 `StateError::Other`
    /// - 锁中毒: 返 `Err(TryLockError::Poisoned)`, 映射为 `StateError::Poisoned`
    pub fn try_lock(&self, organ: Organ) -> Result<MutexGuard<'_, T>, StateError> {
        self.inner.try_lock().map_err(|e| match e {
            std::sync::TryLockError::Poisoned(_) => StateError::Poisoned {
                mode: SharedStateMode::Mutex,
                organ,
            },
            std::sync::TryLockError::WouldBlock => StateError::Other {
                msg: "Mutex would block".to_string(),
            },
        })
    }
}

impl<T> Default for MutexState<T>
where
    T: Default + Send + 'static,
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> Clone for MutexState<T>
where
    T: Send + 'static,
{
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> SharedState<T> for MutexState<T>
where
    T: Send + 'static,
{
    fn mode(&self) -> SharedStateMode {
        SharedStateMode::Mutex
    }

    fn read(&self) -> Result<StateReadGuard<'_, T>, StateError> {
        // Mutex: lock + MutexGuard (可读 + 可写)
        // 锁中毒: 返 Poisoned
        let guard = self.inner.lock().map_err(|_| StateError::Poisoned {
            mode: SharedStateMode::Mutex,
            organ: Organ::Mind, // 默认 organ, 调用方应在更上层处理
        })?;
        Ok(StateReadGuard::Mutex(guard))
    }

    fn write(&self) -> Result<StateWriteGuard<'_, T>, StateError> {
        let guard = self.inner.lock().map_err(|_| StateError::Poisoned {
            mode: SharedStateMode::Mutex,
            organ: Organ::Mind,
        })?;
        Ok(StateWriteGuard::Mutex(guard))
    }
}

/// **MutexStateInit marker**: 标记 MutexState 已被 init.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MutexStateInit {
    /// 编译期 hardcode 字段.
    pub _marker: u8,
}

impl Default for MutexStateInit {
    fn default() -> Self {
        Self { _marker: 0 }
    }
}

impl MutexStateInit {
    /// 新建 init marker.
    pub const fn new() -> Self {
        Self { _marker: 0 }
    }
}

/// **模式 2 marker**: MutexStateMode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MutexStateMode;

impl MutexStateMode {
    /// 编译期字符串.
    pub const fn as_str() -> &'static str {
        "mutex"
    }
}

// =====================================================================
// 单元测试 (MutexState 基础 + SharedState trait impl + 跨 handle = 10+ 测试)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default, Clone, PartialEq, Eq)]
    struct TestValue {
        counter: u32,
    }

    #[test]
    fn new_with_value() {
        let state = MutexState::new(TestValue { counter: 42 });
        let guard = state.read().unwrap();
        assert_eq!(guard.counter, 42);
    }

    #[test]
    fn default_for_default_type() {
        let state: MutexState<TestValue> = MutexState::default();
        let guard = state.read().unwrap();
        assert_eq!(guard.counter, 0);
    }

    #[test]
    fn handle_clone_shares_state() {
        let state = MutexState::new(TestValue { counter: 1 });
        let handle = state.handle();

        // 通过 handle 改 counter
        {
            let mut g = handle.lock().unwrap();
            g.counter = 100;
        }

        // state 看到同样改动
        let guard = state.read().unwrap();
        assert_eq!(guard.counter, 100);
    }

    #[test]
    fn shared_state_mode_is_mutex() {
        let state = MutexState::new(TestValue { counter: 0 });
        assert_eq!(state.mode(), SharedStateMode::Mutex);
    }

    #[test]
    fn write_increments_counter() {
        let state = MutexState::new(TestValue { counter: 0 });
        {
            let mut g = state.write().unwrap();
            g.counter += 1;
        }
        let g = state.read().unwrap();
        assert_eq!(g.counter, 1);
    }

    #[test]
    fn try_lock_succeeds_when_uncontended() {
        let state = MutexState::new(TestValue { counter: 0 });
        let g = state.try_lock(Organ::Heart).expect("should lock");
        assert_eq!(g.counter, 0);
    }

    #[test]
    fn clone_creates_independent_handle() {
        let state = MutexState::new(TestValue { counter: 1 });
        let cloned = state.clone();
        // 同一个 inner (Arc clone, 共享 state)
        {
            let mut g = cloned.write().unwrap();
            g.counter = 999;
        }
        let g = state.read().unwrap();
        assert_eq!(g.counter, 999);
    }

    #[test]
    fn init_marker_constructible() {
        let _ = MutexStateInit::new();
        let _ = MutexStateInit::default();
    }

    #[test]
    fn state_mode_marker_as_str() {
        assert_eq!(MutexStateMode::as_str(), "mutex");
    }

    #[test]
    fn poison_after_panic_then_read_returns_poisoned() {
        // 先拿锁, 内部 panic 触发中毒, 后续 read 应该返 Poisoned 错误
        let state = MutexState::new(TestValue { counter: 0 });
        let handle = state.handle();

        // 子线程 panic 在锁内
        let handle_clone = Arc::clone(&handle);
        let join = std::thread::spawn(move || {
            let _guard = handle_clone.lock().unwrap();
            panic!("intentional panic to poison mutex");
        });
        let _ = join.join(); // 子线程已 panic, join 返 Err

        // 主线程 read: 锁中毒, 返 Poisoned
        let r = state.read();
        assert!(matches!(r, Err(StateError::Poisoned { .. })));
    }
}
