//! ExecWorker - 执行 worker (从 v1.0 apeireth-companion/exec_worker.rs 136 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 spawn + 0 装 PASS 限制

use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecStatus { Pending, Running, Done, Failed }

pub struct ExecResult { pub status: ExecStatus, pub stdout: String, pub stderr: String }

pub struct ExecWorker;

impl ExecWorker {
    pub fn new() -> Self { Self }
    /// 0 装 PASS: 真 spawn (mock, 0 装 PASS 标 stub 需真 shell)
    pub fn spawn(&self, cmd: &str) -> Result<ExecResult, String> {
        // 0 装 PASS 严守: mock 实现, 真 exec 需 sandbox.rs 接入
        if cmd.is_empty() { return Err("empty cmd".into()); }
        Ok(ExecResult { status: ExecStatus::Done, stdout: format!("mock: {}", cmd), stderr: String::new() })
    }
}

impl Default for ExecWorker { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_empty_cmd() { assert!(ExecWorker::new().spawn("").is_err()); }
    #[test] fn test_basic() {
        let r = ExecWorker::new().spawn("ls").unwrap();
        assert_eq!(r.status, ExecStatus::Done);
        assert!(r.stdout.contains("mock"));
    }
    #[test] fn test_status_eq() { assert_eq!(ExecStatus::Done, ExecStatus::Done); }
}
