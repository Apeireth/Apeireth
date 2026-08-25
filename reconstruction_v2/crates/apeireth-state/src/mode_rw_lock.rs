//! RwLockState — mode 3: cross-thread read-write lock.
use std::sync::{Arc, RwLock};

/// RwLock state mode (3 variants).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RwLockStateMode { Read, Write, Owned }

/// RwLockState<T> — wraps `Arc<RwLock<T>>` for read-heavy scenarios.
#[derive(Debug, Clone)]
pub struct RwLockState<T: ?Sized>(Arc<RwLock<T>>);

impl<T> RwLockState<T> {
    pub fn new(value: T) -> Self {
        Self(Arc::new(RwLock::new(value)))
    }

    pub fn read<F, R>(&self, f: F) -> R
    where F: FnOnce(&T) -> R {
        let guard = self.0.read().expect("RwLockState poisoned");
        f(&*guard)
    }

    pub fn write<F, R>(&self, f: F) -> R
    where F: FnOnce(&mut T) -> R {
        let mut guard = self.0.write().expect("RwLockState poisoned");
        f(&mut *guard)
    }
}

impl<T: Default> Default for RwLockState<T> {
    fn default() -> Self { Self::new(T::default()) }
}

/// RwLockStateInit<T> — initialization helper.
#[derive(Debug, Clone)]
pub struct RwLockStateInit<T>(T);

impl<T> RwLockStateInit<T> {
    pub fn new(value: T) -> Self { Self(value) }
    pub fn into_state(self) -> RwLockState<T> { RwLockState::new(self.0) }
}