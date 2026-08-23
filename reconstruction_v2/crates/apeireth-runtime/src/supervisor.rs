use std::time::Duration;
use tokio::time::sleep;
use async_trait::async_trait;

#[derive(Debug, Clone, Copy)]
pub enum RestartStrategy {
    OneForOne,
    OneForAll,
    RestForOne,
}

#[async_trait]
pub trait Worker: Send + Sync {
    async fn run(&self) -> Result<(), String>;
}

pub struct Supervisor {
    pub strategy: RestartStrategy,
    max_restarts: usize,
    base_backoff: Duration,
}

impl Default for Supervisor {
    fn default() -> Self {
        Self::new(RestartStrategy::OneForOne)
    }
}

impl Supervisor {
    pub fn new(strategy: RestartStrategy) -> Self {
        Self {
            strategy,
            max_restarts: 3,
            base_backoff: Duration::from_millis(100),
        }
    }

    pub fn with_retries(mut self, max: usize) -> Self {
        self.max_restarts = max;
        self
    }

    pub async fn supervise(&self, worker: Box<dyn Worker>) -> Result<(), String> {
        let mut retries = 0;
        let mut backoff = self.base_backoff;

        loop {
            match worker.run().await {
                Ok(_) => return Ok(()),
                Err(e) => {
                    if retries >= self.max_restarts {
                        return Err(format!("Worker failed after {} retries: {}", retries, e));
                    }
                    retries += 1;
                    sleep(backoff).await;
                    backoff *= 2; // exponential backoff
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct FlakyWorker {
        attempts: Arc<AtomicUsize>,
        fail_until: usize,
    }

    #[async_trait]
    impl Worker for FlakyWorker {
        async fn run(&self) -> Result<(), String> {
            let attempt = self.attempts.fetch_add(1, Ordering::SeqCst);
            if attempt < self.fail_until {
                Err("Random crash".into())
            } else {
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn test_supervisor_recovers() {
        let sup = Supervisor::new(RestartStrategy::OneForOne).with_retries(5);
        let attempts = Arc::new(AtomicUsize::new(0));
        let worker = Box::new(FlakyWorker {
            attempts: attempts.clone(),
            fail_until: 2,
        });

        let res = sup.supervise(worker).await;
        assert!(res.is_ok());
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_supervisor_gives_up() {
        let sup = Supervisor::new(RestartStrategy::OneForOne).with_retries(2);
        let attempts = Arc::new(AtomicUsize::new(0));
        let worker = Box::new(FlakyWorker {
            attempts: attempts.clone(),
            fail_until: 5, // Will fail more times than retries
        });

        let res = sup.supervise(worker).await;
        assert!(res.is_err());
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }
}
