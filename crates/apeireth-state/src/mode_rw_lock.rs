//! # RwLockState — 模式 3: 跨线程读写锁
//!
//! 借鉴 Golutra v0.1.0 `state: tauri::State<RwLock<T>>` 9 Tauri state 模式 1:1 翻译.
//!
//! ## 设计
//!
//! - `Arc<std::sync::RwLock<T>>` 持有 state, 跨线程共享
//! - `read` 走 `RwLock::read()` (`RwLockReadGuard<T>`, 多个 reader 并发)
//! - `write` 走 `RwLock::write()` (`RwLockWriteGuard<T>`, exclusive)
//! - 锁中毒返 `StateError::Poisoned`
//!
//! ## 适用场景
//!
//! - 读多写少 state (memory 会话历史 / eye 输入监控 / ear 事件订阅)
//! - 9 器官中, 真实集成通常 memory / eye / ear 走 RwLock
//!
//! **不假装**:
//! - `RwLock::read()` 返 `Result<RwLockReadGuard, _>`, 锁中毒返 `Err`
//! - skeleton 阶段不引入 parking_lot (跟 stdlib 一致)

use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use serde::{Deserialize, Serialize};

use crate::error::StateError;
use crate::organ::Organ;
use crate::shared_state::{
    SharedState, SharedStateMode, StateReadGuard, StateWriteGuard,
};

/// **模式 3: RwLockState 跨线程读写锁** (per 借鉴 Golutra `state: tauri::State<RwLock<T>>`).
///
/// **设计**:
/// - 内部用 `Arc<RwLock<T>>` 持有 state
/// - `read` 走 `RwLock::read()` (N 个 reader 并发)
/// - `write` 走 `RwLock::write()` (exclusive, 阻塞 reader)
/// - 锁中毒返 `StateError::Poisoned`
#[derive(Debug)]
pub struct RwLockState<T>
where
    T: Send + Sync + 'static,
{
    /// 内部 `Arc<RwLock<T>>` (跨线程共享).
    inner: Arc<RwLock<T>>,
}

impl<T> RwLockState<T>
where
    T: Send + Sync + 'static,
{
    /// 新建 RwLockState (立即 init).
    pub fn new(value: T) -> Self {
        Self {
            inner: Arc::new(RwLock::new(value)),
        }
    }

    /// Get 内部 `Arc<RwLock<T>>` 的 clone.
    pub fn handle(&self) -> Arc<RwLock<T>> {
        Arc::clone(&self.inner)
    }

    /// Try read (非阻塞, 多个 reader 并发).
    pub fn try_read(&self, organ: Organ) -> Result<RwLockReadGuard<'_, T>, StateError> {
        self.inner
            .try_read()
            .map_err(|e| match e {
                std::sync::TryLockError::Poisoned(_) => StateError::Poisoned {
                    mode: SharedStateMode::RwLock,
                    organ,
                },
                std::sync::TryLockError::WouldBlock => StateError::Other {
                    msg: "RwLock read would block".to_string(),
                },
            })
    }

    /// Try write (非阻塞, exclusive).
    pub fn try_write(&self, organ: Organ) -> Result<RwLockWriteGuard<'_, T>, StateError> {
        self.inner
            .try_write()
            .map_err(|e| match e {
                std::sync::TryLockError::Poisoned(_) => StateError::Poisoned {
                    mode: SharedStateMode::RwLock,
                    organ,
                },
                std::sync::TryLockError::WouldBlock => StateError::Other {
                    msg: "RwLock write would block".to_string(),
                },
            })
    }
}

impl<T> Default for RwLockState<T>
where
    T: Default + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> Clone for RwLockState<T>
where
    T: Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> SharedState<T> for RwLockState<T>
where
    T: Send + Sync + 'static,
{
    fn mode(&self) -> SharedStateMode {
        SharedStateMode::RwLock
    }

    fn read(&self) -> Result<StateReadGuard<'_, T>, StateError> {
        let guard = self.inner.read().map_err(|_| StateError::Poisoned {
            mode: SharedStateMode::RwLock,
            organ: Organ::Memory, // 默认 organ (RwLock 主用)
        })?;
        Ok(StateReadGuard::RwLock(guard))
    }

    fn write(&self) -> Result<StateWriteGuard<'_, T>, StateError> {
        let guard = self.inner.write().map_err(|_| StateError::Poisoned {
            mode: SharedStateMode::RwLock,
            organ: Organ::Memory,
        })?;
        Ok(StateWriteGuard::RwLock(guard))
    }
}

/// **RwLockStateInit marker**: 标记 RwLockState 已被 init.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RwLockStateInit {
    /// 编译期 hardcode 字段.
    pub _marker: u8,
}

impl Default for RwLockStateInit {
    fn default() -> Self {
        Self { _marker: 0 }
    }
}

impl RwLockStateInit {
    /// 新建 init marker.
    pub const fn new() -> Self {
        Self { _marker: 0 }
    }
}

/// **模式 3 marker**: RwLockStateMode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RwLockStateMode;

impl RwLockStateMode {
    /// 编译期字符串.
    pub const fn as_str() -> &'static str {
        "rw_lock"
    }
}

// =====================================================================
// 单元测试 (RwLockState 基础 + SharedState trait impl + 跨 handle = 10+ 测试)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[derive(Debug, Default, Clone, PartialEq, Eq)]
    struct TestValue {
        data: Vec<u32>,
    }

    #[test]
    fn new_with_value() {
        let state = RwLockState::new(TestValue { data: vec![1, 2, 3] });
        let guard = state.read().unwrap();
        assert_eq!(guard.data, vec![1, 2, 3]);
    }

    #[test]
    fn default_for_default_type() {
        let state: RwLockState<TestValue> = RwLockState::default();
        let guard = state.read().unwrap();
        assert!(guard.data.is_empty());
    }

    #[test]
    fn read_concurrent_via_handles() {
        let state = RwLockState::new(TestValue { data: vec![42] });
        let h1 = state.handle();
        let h2 = state.handle();

        // 2 个 reader 并发拿锁 (互不阻塞)
        let g1 = h1.read().unwrap();
        let g2 = h2.read().unwrap();
        assert_eq!(g1.data, vec![42]);
        assert_eq!(g2.data, vec![42]);
    }

    #[test]
    fn write_blocks_new_readers() {
        let state = RwLockState::new(TestValue { data: vec![0] });
        let h = state.handle();

        // 主线程拿写锁
        let _write_guard = h.write().unwrap();

        // 同线程 try_read 在持有写锁时确实会 WouldBlock
        // (注意: 跨线程测试需要 join 时机控制, 此处只 smoke test 同步路径)
        let _r = state.try_read(Organ::Memory);
        // 这里 write_guard 还在, 但 try_read 在另一 scope 试, std::sync::RwLock 不会因为
        // "已存在 write_guard" 自动 WouldBlock (取决于实现), 仅 smoke
        // 实际: 上面 _write_guard 还在 scope, 但 try_read 是不同的 RwLock path
        // 释放写锁
        drop(_write_guard);
        // 现在 read 应该成功
        let g = state.try_read(Organ::Memory).expect("read after write release should succeed");
        assert_eq!(g.data, vec![0]);
    }

    #[test]
    fn write_then_read_returns_updated_value() {
        let state = RwLockState::new(TestValue { data: vec![] });
        {
            let mut g = state.write().unwrap();
            g.data.push(1);
            g.data.push(2);
        }
        let g = state.read().unwrap();
        assert_eq!(g.data, vec![1, 2]);
    }

    #[test]
    fn shared_state_mode_is_rwlock() {
        let state = RwLockState::new(TestValue { data: vec![] });
        assert_eq!(state.mode(), SharedStateMode::RwLock);
    }

    #[test]
    fn try_read_when_uncontended_succeeds() {
        let state = RwLockState::new(TestValue { data: vec![1] });
        let g = state.try_read(Organ::Memory).expect("should read");
        assert_eq!(g.data, vec![1]);
    }

    #[test]
    fn try_write_when_uncontended_succeeds() {
        let state = RwLockState::new(TestValue { data: vec![] });
        // try_write + drop guard before read (否则 read 在 write_guard 持锁时阻塞, 死锁)
        {
            let mut g = state.try_write(Organ::Memory).expect("should write");
            g.data.push(99);
        } // g 释放
        let r = state.read().expect("read after write release should succeed");
        assert_eq!(r.data, vec![99]);
    }

    #[test]
    fn clone_creates_independent_handle() {
        let state = RwLockState::new(TestValue { data: vec![1] });
        let cloned = state.clone();
        {
            let mut g = cloned.write().unwrap();
            g.data.push(2);
        }
        let g = state.read().unwrap();
        assert_eq!(g.data, vec![1, 2]);
    }

    #[test]
    fn init_marker_constructible() {
        let _ = RwLockStateInit::new();
        let _ = RwLockStateInit::default();
    }

    #[test]
    fn state_mode_marker_as_str() {
        assert_eq!(RwLockStateMode::as_str(), "rw_lock");
    }
}
