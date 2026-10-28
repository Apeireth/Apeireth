//! Cancellation and draining for one plugin's runtime-owned futures.
//! Closing a scope refuses new work; it never kills a thread or rolls back effects.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::TaskId;
use futures::future::{AbortHandle, Abortable};
use futures::task::AtomicWaker;

#[derive(Default)]
struct Tasks {
    closed: bool,
    handles: BTreeMap<TaskId, AbortHandle>,
}

#[derive(Default)]
struct Inner {
    tasks: Mutex<Tasks>,
    drained: AtomicWaker,
}

/// Cloneable cancellation port; all clones belong to the same module instance.
#[derive(Clone, Default)]
pub struct ModuleTaskScope(Arc<Inner>);

/// A future was refused or interrupted because its module is stopping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleCancelled;

struct Lease {
    scope: ModuleTaskScope,
    id: TaskId,
}

impl Drop for Lease {
    fn drop(&mut self) {
        self.scope
            .0
            .tasks
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .handles
            .remove(&self.id);
        self.scope.0.drained.wake();
    }
}

impl ModuleTaskScope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_open(&self) -> bool {
        !self
            .0
            .tasks
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .closed
    }

    pub fn active_tasks(&self) -> usize {
        self.0
            .tasks
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .handles
            .len()
    }

    /// Cancel on closure or on dropping the caller future; the lease always drains.
    pub async fn run<T>(&self, future: impl Future<Output = T>) -> Result<T, ModuleCancelled> {
        let id = TaskId::new();
        let (handle, registration) = AbortHandle::new_pair();
        {
            let mut tasks = self.0.tasks.lock().unwrap_or_else(|e| e.into_inner());
            if tasks.closed {
                return Err(ModuleCancelled);
            }
            tasks.handles.insert(id, handle);
        }
        let _lease = Lease {
            scope: self.clone(),
            id,
        };
        Abortable::new(future, registration)
            .await
            .map_err(|_| ModuleCancelled)
    }

    pub fn close(&self) {
        let handles = {
            let mut tasks = self.0.tasks.lock().unwrap_or_else(|e| e.into_inner());
            tasks.closed = true;
            tasks.handles.values().cloned().collect::<Vec<_>>()
        };
        for handle in handles {
            handle.abort();
        }
    }

    /// One lifecycle owner waits after closing. Never hold a registry lock here.
    pub async fn close_and_wait(&self) {
        self.close();
        futures::future::poll_fn(|cx| {
            self.0.drained.register(cx.waker());
            if self.active_tasks() == 0 {
                std::task::Poll::Ready(())
            } else {
                std::task::Poll::Pending
            }
        })
        .await
    }
}
