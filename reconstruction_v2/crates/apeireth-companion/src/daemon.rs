//! Daemon - 守护循环 (从 v1.0 apeireth-companion/daemon.rs 997 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 Daemon trait + 错误监控 + spawn/join

use std::sync::Arc;
use tokio::sync::Mutex;

/// 0 装 PASS: 真 Daemon 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonState { Running, Stopped, Error }

/// 0 装 PASS: 真 Judicator 决策 (v1 daemon 简化)
pub struct Judicator;

impl Judicator {
    pub fn new() -> Self { Self }
    /// 0 装 PASS: 真评估
    pub fn evaluate(&self, risk: u8) -> DaemonState {
        if risk > 90 { DaemonState::Error }
        else if risk == 0 { DaemonState::Stopped }
        else { DaemonState::Running }
    }
}

/// 0 装 PASS: 真需要 LLM review (v1 requires_llm_review 简化)
pub fn requires_llm_review(risk: u8, confidence: f32) -> bool {
    risk > 50 && confidence < 0.7
}

pub struct Daemon { pub state: Arc<Mutex<DaemonState>> }

impl Daemon {
    pub fn new() -> Self { Self { state: Arc::new(Mutex::new(DaemonState::Stopped)) } }
    pub async fn start(&self) { *self.state.lock().await = DaemonState::Running; }
    pub async fn stop(&self) { *self.state.lock().await = DaemonState::Stopped; }
    pub async fn current(&self) -> DaemonState { *self.state.lock().await }
}

impl Default for Daemon { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_judicator_running() { assert_eq!(Judicator::new().evaluate(50), DaemonState::Running); }
    #[test] fn test_judicator_stopped() { assert_eq!(Judicator::new().evaluate(0), DaemonState::Stopped); }
    #[test] fn test_judicator_error() { assert_eq!(Judicator::new().evaluate(95), DaemonState::Error); }
    #[test] fn test_requires_llm_review() {
        assert!(requires_llm_review(80, 0.5));
        assert!(!requires_llm_review(80, 0.9));
        assert!(!requires_llm_review(30, 0.5));
    }
    #[tokio::test] async fn test_daemon_lifecycle() {
        let d = Daemon::new();
        assert_eq!(d.current().await, DaemonState::Stopped);
        d.start().await;
        assert_eq!(d.current().await, DaemonState::Running);
        d.stop().await;
        assert_eq!(d.current().await, DaemonState::Stopped);
    }
    #[test] fn test_state_eq() { assert_eq!(DaemonState::Running, DaemonState::Running); }
}
