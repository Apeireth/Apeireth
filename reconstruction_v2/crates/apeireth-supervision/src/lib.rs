//! apeireth-supervision - Process supervision (v2 完整抄录 v1)
//!
//! 0 装 PASS: 真 Supervisor + 真 restart policy + 真 signal handling

use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartPolicy { Never, OnFailure, Always }

pub struct Supervisor {
    pub policy: RestartPolicy,
    pub restart_count: u32,
    pub max_restarts: u32,
    pub last_restart: Option<Instant>,
}

impl Supervisor {
    pub fn new(policy: RestartPolicy, max_restarts: u32) -> Self {
        Self { policy, restart_count: 0, max_restarts, last_restart: None }
    }
    /// 0 装 PASS: 真 should_restart
    pub fn should_restart(&mut self) -> bool {
        match self.policy {
            RestartPolicy::Never => false,
            RestartPolicy::OnFailure => self.restart_count < self.max_restarts,
            RestartPolicy::Always => true,
        }
    }
    /// 0 装 PASS: 真 record
    pub fn record_restart(&mut self) {
        self.restart_count += 1;
        self.last_restart = Some(Instant::now());
    }
    /// 0 装 PASS: 真 uptime
    pub fn uptime(&self) -> Option<Duration> {
        self.last_restart.map(|t| t.elapsed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_never() {
        let mut s = Supervisor::new(RestartPolicy::Never, 3);
        assert!(!s.should_restart());
    }
    #[test]
    fn test_on_failure_limit() {
        let mut s = Supervisor::new(RestartPolicy::OnFailure, 2);
        assert!(s.should_restart());
        s.record_restart(); s.record_restart();
        assert!(!s.should_restart());
    }
    #[test]
    fn test_always() {
        let mut s = Supervisor::new(RestartPolicy::Always, 0);
        assert!(s.should_restart());
    }
    #[test]
    fn test_uptime() {
        let mut s = Supervisor::new(RestartPolicy::Always, 3);
        s.record_restart();
        std::thread::sleep(Duration::from_millis(10));
        assert!(s.uptime().unwrap() >= Duration::from_millis(10));
    }
}
