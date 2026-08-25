//! SharedState trait + SharedStateMode.

/// 3 模式 state 模式选择.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedStateMode { OnceLock, Mutex, RwLock }

/// 3 模式抽象 trait.
pub trait SharedState<T: ?Sized>: Send + Sync {
    fn mode(&self) -> SharedStateMode;
    fn with_lock<F, R>(&self, f: F) -> R
    where F: FnOnce(&mut T) -> R;
}

impl SharedStateMode {
    pub fn as_str(&self) -> &'static str {
        match self { Self::OnceLock=>"once_lock", Self::Mutex=>"mutex", Self::RwLock=>"rw_lock" }
    }
}