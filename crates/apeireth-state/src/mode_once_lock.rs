//! # OnceLockState — 模式 1: 进程全局 lazy init
//!
//! 借鉴 Golutra v0.1.0 `OnceLock<Arc<T>>` 9 Tauri state 模式 1:1 翻译.
//!
//! ## 设计
//!
//! - `OnceLock<Arc<T>>` 静态 / 实例字段, **lazy init** (首次访问时 `init`)
//! - 后续访问直接 clone `Arc<T>`, 0 锁开销
//! - 写 `init` 只能调 1 次, 后续 `init` 返 `Err(AlreadyInitialized)`
//!
//! ## 适用场景
//!
//! - 启动后只读的配置 / 路由表 / capability list
//! - 编译期 hardcode 的 6 哲学锚 (mind 器官)
//! - 进程级常量 (e.g. PLATFORM_NAME)
//!
//! **不假装**:
//! - `init` 是 stub impl, 当前 0 真接 (R21+ 续做)
//! - 9 器官中, 真实集成通常 mind 走 OnceLock (6 锚 + 成长阶段), 其他 8 器官走 Mutex / RwLock
//!
//! ## 与 LOCKED crate 边界
//!
//! - 0 触碰 24 LOCKED crate
//! - 0 改 workspace version
//! - 0 假装 "OnceLockState 是异步的" (它本质是 sync)

use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};

use crate::error::{StateError, StateErrorKind};
use crate::organ::Organ;
use crate::shared_state::{
    SharedState, SharedStateMode, StateReadGuard, StateWriteGuard,
};

/// **模式 1: OnceLockState 进程全局 lazy init** (per 借鉴 Golutra `OnceLock<Arc<T>>`).
///
/// **设计**:
/// - 内部用 `OnceLock<Arc<T>>` 持有 state
/// - `init(value)` 仅 1 次生效, 后续 init 返 `AlreadyInitialized` 错误
/// - `get()` clone `Arc<T>`, 0 锁开销
///
/// **不假装**: 当前 skeleton 阶段 `init` 走 `Ok(())` 直接覆盖, **不**做 `OnceLock::set` 内部 mutex 守门.
#[derive(Debug)]
pub struct OnceLockState<T>
where
    T: Send + Sync + 'static,
{
    /// 内部 `OnceLock<Arc<T>>` (lazy init).
    inner: OnceLock<Arc<T>>,
}

impl<T> Default for OnceLockState<T>
where
    T: Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> OnceLockState<T>
where
    T: Send + Sync + 'static,
{
    /// 新建空 OnceLockState (未 init).
    pub const fn new() -> Self {
        Self {
            inner: OnceLock::new(),
        }
    }

    /// Lazy init 一次.
    ///
    /// 行为 (per 借鉴 Golutra 9 Tauri state init 模式):
    /// - 首次调用: 把 `value` 包成 `Arc<T>` 存入 OnceLock, 返 `Ok(())`
    /// - 后续调用: 返 `Err(AlreadyInitialized)` (OnceLock::set 返 Err 表示已 set)
    pub fn init(&self, value: T) -> Result<(), StateError> {
        self.inner
            .set(Arc::new(value))
            .map_err(|_arc| StateError::Other {
                msg: "OnceLockState already initialized".to_string(),
            })
    }

    /// Get 当前 state (clone `Arc<T>`).
    ///
    /// 行为:
    /// - 已 init: clone `Arc<T>`, 返 `Some(Arc<T>)`
    /// - 未 init: 返 `None` (无错误, 仅 None, 跟 `OnceLock::get` 一致)
    pub fn get(&self) -> Option<Arc<T>> {
        self.inner.get().cloned()
    }

    /// Get 强制 unwrap (未 init 返 `StateError::NotInitialized`).
    pub fn get_unwrap(&self, organ: Organ) -> Result<Arc<T>, StateError> {
        self.inner.get().cloned().ok_or(StateError::NotInitialized {
            mode: SharedStateMode::OnceLock,
            organ,
        })
    }

    /// 是否已 init.
    pub fn is_initialized(&self) -> bool {
        self.inner.get().is_some()
    }
}

impl<T> SharedState<T> for OnceLockState<T>
where
    T: Send + Sync + 'static,
{
    fn mode(&self) -> SharedStateMode {
        SharedStateMode::OnceLock
    }

    fn read(&self) -> Result<StateReadGuard<'_, T>, StateError> {
        // OnceLock: 借用 `&T` (无锁, 但需要先 init)
        let arc = self.inner.get().ok_or(StateError::NotInitialized {
            mode: SharedStateMode::OnceLock,
            organ: Organ::Mind, // 默认 organ, 调用方应在更上层处理
        })?;
        Ok(StateReadGuard::OnceLock(arc.as_ref()))
    }

    fn write(&self) -> Result<StateWriteGuard<'_, T>, StateError> {
        // OnceLock 模式不支持写 (per 借鉴 Golutra 9 Tauri state 设计: OnceLock 是只读 single-instance)
        Err(StateError::Unsupported {
            mode: SharedStateMode::OnceLock,
            organ: Organ::Mind,
            reason: "OnceLockState 不支持写, 请用 MutexState 或 RwLockState".to_string(),
        })
    }
}

/// **OnceLockStateInit marker**: 标记 OnceLockState 已被 init (per 借鉴 Golutra `StateInit` 模式).
///
/// **不假装**: skeleton 阶段 0 业务字段, 仅作编译期存在性标记.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OnceLockStateInit {
    /// 编译期 hardcode 字段, 防 serde 默认空 struct 不被 derive 接受.
    pub _marker: u8,
}

impl Default for OnceLockStateInit {
    fn default() -> Self {
        Self { _marker: 0 }
    }
}

impl OnceLockStateInit {
    /// 新建 init marker.
    pub const fn new() -> Self {
        Self { _marker: 0 }
    }
}

/// **模式 1 marker**: OnceLockStateMode (per 借鉴 Golutra `StateMode::OnceLock`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OnceLockStateMode;

impl OnceLockStateMode {
    /// 编译期字符串.
    pub const fn as_str() -> &'static str {
        "once_lock"
    }
}

// =====================================================================
// 单元测试 (OnceLockState 基础 + SharedState trait impl = 10+ 测试)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestValue {
        counter: u32,
    }

    #[test]
    fn new_is_uninitialized() {
        let state: OnceLockState<TestValue> = OnceLockState::new();
        assert!(!state.is_initialized());
        assert!(state.get().is_none());
    }

    #[test]
    fn init_then_get() {
        let state: OnceLockState<TestValue> = OnceLockState::new();
        state.init(TestValue { counter: 42 }).unwrap();
        assert!(state.is_initialized());
        let got = state.get().expect("should be Some after init");
        assert_eq!(got.counter, 42);
    }

    #[test]
    fn default_equivalent_to_new() {
        let state: OnceLockState<TestValue> = OnceLockState::default();
        assert!(!state.is_initialized());
    }

    #[test]
    fn shared_state_mode_is_oncelock() {
        let state: OnceLockState<TestValue> = OnceLockState::new();
        assert_eq!(state.mode(), SharedStateMode::OnceLock);
    }

    #[test]
    fn read_uninit_returns_not_initialized_error() {
        let state: OnceLockState<TestValue> = OnceLockState::new();
        let r = state.read();
        assert!(matches!(
            r,
            Err(StateError::NotInitialized {
                mode: SharedStateMode::OnceLock,
                ..
            })
        ));
    }

    #[test]
    fn read_after_init_returns_value() {
        let state: OnceLockState<TestValue> = OnceLockState::new();
        state.init(TestValue { counter: 100 }).unwrap();
        let guard = state.read().expect("read after init should succeed");
        assert_eq!(guard.counter, 100);
    }

    #[test]
    fn write_returns_unsupported_error() {
        let state: OnceLockState<TestValue> = OnceLockState::new();
        state.init(TestValue { counter: 1 }).unwrap();
        let r = state.write();
        assert!(matches!(r, Err(StateError::Unsupported { .. })));
    }

    #[test]
    fn write_uninit_also_unsupported() {
        let state: OnceLockState<TestValue> = OnceLockState::new();
        let r = state.write();
        // write 在 OnceLock 永远 Unsupported (不管 init 与否)
        assert!(matches!(r, Err(StateError::Unsupported { .. })));
    }

    #[test]
    fn get_unwrap_uninit_returns_error() {
        let state: OnceLockState<TestValue> = OnceLockState::new();
        let r = state.get_unwrap(Organ::Mind);
        assert!(matches!(r, Err(StateError::NotInitialized { .. })));
    }

    #[test]
    fn get_unwrap_after_init_returns_value() {
        let state: OnceLockState<TestValue> = OnceLockState::new();
        state.init(TestValue { counter: 7 }).unwrap();
        let arc = state.get_unwrap(Organ::Mind).expect("should be Some after init");
        assert_eq!(arc.counter, 7);
    }

    #[test]
    fn init_marker_constructible() {
        let _ = OnceLockStateInit::new();
        let _ = OnceLockStateInit::default();
    }

    #[test]
    fn state_mode_marker_as_str() {
        assert_eq!(OnceLockStateMode::as_str(), "once_lock");
    }
}
