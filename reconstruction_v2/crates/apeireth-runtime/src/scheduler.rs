use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub struct Scheduler {
    running: Arc<AtomicBool>,
    tasks: Vec<JoinHandle<()>>,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(true)),
            tasks: Vec::new(),
        }
    }

    /// Spawns a periodic background task with a given interval
    pub fn schedule_periodic<F>(&mut self, name: &'static str, interval_secs: u64, mut action: F)
    where
        F: FnMut() -> BoxFuture<'static, ()> + Send + 'static,
    {
        let is_running = self.running.clone();
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
            // First tick fires immediately, skip it
            interval.tick().await;

            while is_running.load(Ordering::Relaxed) {
                interval.tick().await;
                if !is_running.load(Ordering::Relaxed) {
                    break;
                }
                tracing::debug!(task = name, "Executing scheduled background tick");
                action().await;
            }
            tracing::info!(task = name, "Scheduled background task stopped");
        });

        self.tasks.push(handle);
    }

    /// Stops all running scheduled tasks
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        for task in self.tasks.drain(..) {
            task.abort();
        }
    }
}

impl Drop for Scheduler {
    fn drop(&mut self) {
        self.stop();
    }
}
