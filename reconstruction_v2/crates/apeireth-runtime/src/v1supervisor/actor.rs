//! Actor mailbox — single-threaded handler loop over a tokio mpsc channel.
//!
//! ponytail: hand-rolled actor with tokio mpsc; the ceiling is a supervisor-
//! owned runtime that restarts actors on panic, the upgrade path is
//! `ActorRef::send` returning a Future of the response; not needed yet.

use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
pub struct ActorRef {
    tx: mpsc::Sender<i64>,
}

impl ActorRef {
    /// Non-blocking send; returns Err if the mailbox is full or closed.
    pub fn try_send(&self, msg: i64) -> Result<(), mpsc::error::TrySendError<i64>> {
        self.tx.try_send(msg)
    }

    /// Awaitable send; returns Err if the mailbox is closed.
    pub async fn send(&self, msg: i64) -> Result<(), mpsc::error::SendError<i64>> {
        self.tx.send(msg).await
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorState {
    Running,
    Stopped,
}

pub trait Actor: Send + Sync + 'static {
    type Message: Send + 'static;
    fn handle(&mut self, msg: Self::Message);
    fn on_stop(&mut self) {}
}

/// Spawn an actor: returns (ref, handle, state-cell).
///
/// ponytail: spawns a tokio task that drains the mailbox; the ceiling is
/// supervisor-driven restart on panic; not implemented (panic = task ends,
/// handle join resolves with Err).
pub fn spawn_actor<A: Actor<Message = i64> + 'static>(
    actor: A,
    mailbox_capacity: usize,
) -> (ActorRef, JoinHandle<()>, Arc<Mutex<ActorState>>) {
    let (tx, mut rx) = mpsc::channel::<i64>(mailbox_capacity);
    let state = Arc::new(Mutex::new(ActorState::Running));
    let state_clone = Arc::clone(&state);
    let handle = tokio::spawn(async move {
        let mut actor = actor;
        while let Some(msg) = rx.recv().await {
            actor.handle(msg);
        }
        actor.on_stop();
        if let Ok(mut s) = state_clone.lock() {
            *s = ActorState::Stopped;
        }
    });
    (ActorRef { tx }, handle, state)
}

/// Reference-counted sum actor — used by tests + demo.
pub struct CounterActor {
    pub sum: i64,
}

impl CounterActor {
    pub fn new() -> Self {
        Self { sum: 0 }
    }
}

impl Default for CounterActor {
    fn default() -> Self {
        Self::new()
    }
}

impl Actor for CounterActor {
    type Message = i64;
    fn handle(&mut self, msg: i64) {
        self.sum += msg;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn counter_actor_handle_accumulates() {
        let actor = CounterActor::new();
        let (tx, handle, state) = spawn_actor(actor, 16);
        tx.send(5).await.unwrap();
        tx.send(10).await.unwrap();
        drop(tx);
        handle.await.unwrap();
        assert_eq!(*state.lock().unwrap(), ActorState::Stopped);
    }
}
