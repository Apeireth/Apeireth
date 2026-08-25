//! MutexState — mode 2: cross-thread mutex.
use std::sync::{Arc, Mutex};

/// Mutex state mode (3 variants — shared with SharedStateMode).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutexStateMode { Read, Write, Owned }

/// MutexState<T> — wraps `Arc<Mutex<T>>` for cross-thread shared access.
#[derive(Debug, Clone)]
pub struct MutexState<T: ?Sized>(Arc<Mutex<T>>);

impl<T> MutexState<T> {
    pub fn new(value: T) -> Self {
        Self(Arc::new(Mutex::new(value)))
    }

    pub fn with<F, R>(&self, f: F) -> R
    where F: FnOnce(&mut T) -> R, {
        let mut guard = self.0.lock().expect("MutexState poisoned");
        f(&mut *guard)
    }
}

impl<T: Default> Default for MutexState<T> {
    fn default() -> Self { Self::new(T::default()) }
}

/// MutexStateInit<T> — initialization helper.
#[derive(Debug, Clone)]
pub struct MutexStateInit<T>(T);

impl<T> MutexStateInit<T> {
    pub fn new(value: T) -> Self { Self(value) }
    pub fn into_state(self) -> MutexState<T> { MutexState::new(self.0) }
}