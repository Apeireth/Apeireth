use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::oneshot;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Error, Debug)]
pub enum TaskError {
    #[error("Task not found")]
    NotFound,
    #[error("Invalid state transition")]
    InvalidTransition,
    #[error("Task already exists")]
    AlreadyExists,
}

pub struct AsyncTaskStore {
    states: Mutex<HashMap<String, TaskState>>,
    notifiers: Mutex<HashMap<String, oneshot::Sender<TaskState>>>,
}

impl Default for AsyncTaskStore {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncTaskStore {
    pub fn new() -> Self {
        Self {
            states: Mutex::new(HashMap::new()),
            notifiers: Mutex::new(HashMap::new()),
        }
    }

    pub fn insert_task(&self, id: &str) -> Result<(), TaskError> {
        let mut states = self.states.lock().unwrap();
        if states.contains_key(id) {
            return Err(TaskError::AlreadyExists);
        }
        states.insert(id.to_string(), TaskState::Pending);
        Ok(())
    }

    /// Double-checked TOCTOU-safe transition
    pub fn transition(&self, id: &str, expected: TaskState, new_state: TaskState) -> Result<(), TaskError> {
        let mut states = self.states.lock().unwrap();
        let current = states.get_mut(id).ok_or(TaskError::NotFound)?;
        
        if *current == expected {
            *current = new_state;
            
            // If completed or failed, notify waiters
            if new_state == TaskState::Completed || new_state == TaskState::Failed {
                let mut notifiers = self.notifiers.lock().unwrap();
                if let Some(tx) = notifiers.remove(id) {
                    let _ = tx.send(new_state);
                }
            }
            Ok(())
        } else {
            Err(TaskError::InvalidTransition)
        }
    }

    pub fn wait_for_completion(&self, id: &str) -> Result<oneshot::Receiver<TaskState>, TaskError> {
        let states = self.states.lock().unwrap();
        let current = states.get(id).ok_or(TaskError::NotFound)?;
        
        if *current == TaskState::Completed || *current == TaskState::Failed {
            // Already done, return immediately via an already resolved oneshot
            let (tx, rx) = oneshot::channel();
            let _ = tx.send(*current);
            return Ok(rx);
        }

        let mut notifiers = self.notifiers.lock().unwrap();
        let (tx, rx) = oneshot::channel();
        notifiers.insert(id.to_string(), tx);
        Ok(rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_transition() {
        let store = AsyncTaskStore::new();
        store.insert_task("task1").unwrap();
        assert!(store.transition("task1", TaskState::Pending, TaskState::Running).is_ok());
        assert!(store.transition("task1", TaskState::Pending, TaskState::Completed).is_err());
    }

    #[tokio::test]
    async fn test_wait_for_completion() {
        let store = Arc::new(AsyncTaskStore::new());
        store.insert_task("task2").unwrap();
        
        let rx = store.wait_for_completion("task2").unwrap();
        
        let store_clone = store.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            store_clone.transition("task2", TaskState::Pending, TaskState::Completed).unwrap();
        });

        let final_state = rx.await.unwrap();
        assert_eq!(final_state, TaskState::Completed);
    }
}
