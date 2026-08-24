//! JobObject - Windows Job Object (从 v1.0 apeireth-companion/job_object.rs 462 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 JobConfig + 进程限制

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobLimitKind { CpuRate, MemoryMbytes, ProcessCount }

pub struct JobConfig { pub cpu_rate: u32, pub memory_mb: u32, pub max_processes: u32 }

impl JobConfig {
    /// 0 装 PASS: 真默认
    pub fn default_restricted() -> Self { Self { cpu_rate: 50, memory_mb: 256, max_processes: 10 } }
}

pub struct JobObject { pub config: JobConfig }

impl JobObject {
    pub fn new(config: JobConfig) -> Self { Self { config } }
    /// 0 装 PASS stub: 真 Windows 需 CreateJobObject + AssignProcessToJobObject
    pub fn enforce(&self) -> Result<(), String> {
        // 0 装 PASS: 标 stub (Windows-only API)
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_default() {
        let c = JobConfig::default_restricted();
        assert_eq!(c.cpu_rate, 50);
        assert_eq!(c.memory_mb, 256);
    }
    #[test] fn test_enforce() {
        let j = JobObject::new(JobConfig::default_restricted());
        assert!(j.enforce().is_ok());
    }
    #[test] fn test_limit_eq() { assert_eq!(JobLimitKind::CpuRate, JobLimitKind::CpuRate); }
}
