//! Channel abstraction (LangGraph pub/sub).

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;

/// Channel error.
#[derive(Debug, Error)]
pub enum ChannelError {
    #[error("channel `{0}` not registered")]
    NotFound(String),
    #[error("type mismatch on channel `{0}`")]
    TypeMismatch(String),
}

/// Channel types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChannelType {
    LastValue,
    Topic,
    BinaryOperator,
    NamedBarrier,
}

/// LastValue channel — stores one value, last write wins.
pub struct LastValue<T: Clone + Send + Sync + 'static> {
    value: RwLock<Option<T>>,
}

impl<T: Clone + Send + Sync + 'static> LastValue<T> {
    pub fn new() -> Self { Self { value: RwLock::new(None) } }
    pub fn set(&self, v: T) { *self.value.write().unwrap() = Some(v); }
    pub fn get(&self) -> Option<T> { self.value.read().unwrap().clone() }
}

impl<T: Clone + Send + Sync + 'static + std::fmt::Debug> Channel for LastValue<T> {
    fn channel_type(&self) -> ChannelType { ChannelType::LastValue }
    fn name(&self) -> &str { "last_value" }
}

/// Topic channel — pub/sub.
pub struct Topic<T: Clone + Send + Sync + 'static> {
    subs: RwLock<Vec<Arc<dyn Fn(T) + Send + Sync>>>,
}

impl<T: Clone + Send + Sync + 'static> Topic<T> {
    pub fn new() -> Self { Self { subs: RwLock::new(Vec::new()) } }
    pub fn publish(&self, v: T) {
        let s = self.subs.read().unwrap().clone();
        for cb in s { cb(v.clone()); }
    }
    pub fn subscribe(&self, cb: Arc<dyn Fn(T) + Send + Sync>) {
        self.subs.write().unwrap().push(cb);
    }
}

/// Binary operator applied on update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    Add, Sub, Mul, Max, Min,
}

/// Binary operator value.
#[derive(Debug, Clone)]
pub struct BinaryOperatorValue {
    pub op: BinaryOperator,
    pub rhs: i64,
}

/// Named barrier — synchronisation barrier between branches.
pub struct NamedBarrier {
    name: String,
    waiting: RwLock<u32>,
    needed: u32,
}

impl NamedBarrier {
    pub fn new(name: impl Into<String>, needed: u32) -> Self {
        Self { name: name.into(), waiting: RwLock::new(0), needed }
    }
    pub fn arrive(&self) -> bool {
        let mut w = self.waiting.write().unwrap();
        *w += 1;
        *w >= self.needed
    }
    pub fn reset(&self) { *self.waiting.write().unwrap() = 0; }
    pub fn waiting(&self) -> u32 { *self.waiting.read().unwrap() }
    pub fn name(&self) -> &str { &self.name }
}

/// Channel trait — read/write abstraction.
pub trait Channel: Send + Sync {
    fn channel_type(&self) -> ChannelType;
    fn name(&self) -> &str;
}

/// Channel registry — named collection of channels.
#[derive(Default)]
pub struct ChannelRegistry {
    inner: RwLock<HashMap<String, Arc<dyn Channel>>>,
}

impl ChannelRegistry {
    pub fn new() -> Self { Self::default() }
    pub fn register(&self, name: impl Into<String>, ch: Arc<dyn Channel>) {
        self.inner.write().unwrap().insert(name.into(), ch);
    }
    pub fn get(&self, name: &str) -> Option<Arc<dyn Channel>> {
        self.inner.read().unwrap().get(name).cloned()
    }
    pub fn list(&self) -> Vec<String> {
        self.inner.read().unwrap().keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicI64, Ordering};

    #[test]
    fn last_value_basic() {
        let lv = LastValue::<i64>::new();
        lv.set(10);
        lv.set(20);
        assert_eq!(lv.get(), Some(20));
    }

    #[test]
    fn topic_pubsub() {
        let t = Topic::<i64>::new();
        let sum = Arc::new(AtomicI64::new(0));
        let s2 = sum.clone();
        t.subscribe(Arc::new(move |v| { s2.fetch_add(v, Ordering::Relaxed); }));
        t.publish(5);
        t.publish(10);
        assert_eq!(sum.load(Ordering::Relaxed), 15);
    }

    #[test]
    fn barrier_arrive() {
        let b = NamedBarrier::new("wait", 3);
        assert!(!b.arrive());
        assert!(!b.arrive());
        assert!(b.arrive());
        b.reset();
        assert_eq!(b.waiting(), 0);
    }

    #[test]
    fn registry_register_get() {
        let reg = ChannelRegistry::new();
        let lv: Arc<dyn Channel> = Arc::new(LastValue::<i64>::new());
        reg.register("x", lv);
        assert!(reg.get("x").is_some());
        assert!(reg.get("missing").is_none());
        assert_eq!(reg.list(), vec!["x".to_string()]);
    }

    #[test]
    fn binary_operator_value_eq() {
        let v1 = BinaryOperatorValue { op: BinaryOperator::Add, rhs: 5 };
        let v2 = BinaryOperatorValue { op: BinaryOperator::Add, rhs: 5 };
        assert_eq!(v1.op, v2.op);
        assert_eq!(v1.rhs, v2.rhs);
    }
}
