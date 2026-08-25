//! OnceLockState — mode 1: process-global lazy init.
use std::sync::{Arc, OnceLock};

/// OnceLock state mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnceLockStateMode { Read, Write, Owned }

/// OnceLockState wraps `OnceLock<Arc<T>>` so we can clone out without cloning T.
#[derive(Debug, Clone)]
pub struct OnceLockState<T>(Arc<OnceLock<Arc<T>>>);

impl<T> OnceLockState<T> {
    pub fn new() -> Self {
        Self(Arc::new(OnceLock::new()))
    }

    /// Get or initialize. Caller supplies an `fn() -> Arc<T>`.
    pub fn get_or_init<F>(&self, f: F) -> Arc<T>
    where F: FnOnce() -> Arc<T>,
    {
        self.0.get_or_init(f).clone()
    }

    pub fn get(&self) -> Option<Arc<T>> {
        self.0.get().cloned()
    }
}

impl<T> Default for OnceLockState<T> {
    fn default() -> Self { Self::new() }
}

/// OnceLockStateInit<T> — initialization helper.
#[derive(Debug, Clone)]
pub struct OnceLockStateInit<T>(pub fn() -> Arc<T>);

impl<T> OnceLockStateInit<T> {
    pub fn new(init: fn() -> Arc<T>) -> Self { Self(init) }
    pub fn build(self) -> OnceLockState<T> { OnceLockState::new() }
}
