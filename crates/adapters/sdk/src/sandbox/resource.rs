//! # Sandbox resource limits (per @anthropic-ai/sandbox 商业版 v0.9.21, 1:1 翻译)
//!
//! **STUB MODE**: 5 资源限制字段保留 1:1 翻译, 实际 cgroup v2 / blockio / net_cls 下发
//! 留 R21+ 真接 bollard / firecracker-rs / runsc 时实现.
//!
//! 5 资源限制 (K-1 强校验 #1):
//! 1. CPU 核数 (单位: 核, 范围 0.1 ~ 64)
//! 2. 内存字节 (单位: bytes, 范围 16 MiB ~ 256 GiB)
//! 3. IO 带宽 (单位: bytes/sec, 范围 1 MiB/s ~ 10 GiB/s)
//! 4. 网络带宽 (单位: bytes/sec, 范围 1 MiB/s ~ 10 GiB/s)
//! 5. 临时目录大小 (单位: bytes, 范围 1 MiB ~ 100 GiB)

use serde::{Deserialize, Serialize};

use crate::sandbox::error::{SandboxError, SandboxResult};

// ============================================================================
// §1 编译期常量 (K-1 强校验 #1: 5 资源限制上下限)
// ============================================================================

/// 最小 CPU 核数 (per v0.9.21 商业版估 0.1, 防止过度限制导致进程无法启动).
pub const MIN_CPU_CORES: f32 = 0.1;
/// 最大 CPU 核数 (per v0.9.21 商业版估 64, 防止独占宿主机).
pub const MAX_CPU_CORES: f32 = 64.0;
/// 最小内存 (16 MiB, per v0.9.21 商业版估, 防止进程无法启动).
pub const MIN_MEMORY_BYTES: u64 = 16 * 1024 * 1024;
/// 最大内存 (256 GiB, per v0.9.21 商业版估, 防止 OOM 宿主机).
pub const MAX_MEMORY_BYTES: u64 = 256 * 1024 * 1024 * 1024;
/// 最小 IO 带宽 (1 MiB/s, per v0.9.21 商业版估).
pub const MIN_IO_BANDWIDTH_BPS: u64 = 1024 * 1024;
/// 最大 IO 带宽 (10 GiB/s, per v0.9.21 商业版估).
pub const MAX_IO_BANDWIDTH_BPS: u64 = 10 * 1024 * 1024 * 1024;
/// 最小网络带宽 (1 MiB/s, per v0.9.21 商业版估).
pub const MIN_NET_BANDWIDTH_BPS: u64 = 1024 * 1024;
/// 最大网络带宽 (10 GiB/s, per v0.9.21 商业版估).
pub const MAX_NET_BANDWIDTH_BPS: u64 = 10 * 1024 * 1024 * 1024;
/// 最小临时目录大小 (1 MiB, per v0.9.21 商业版估).
pub const MIN_TMP_BYTES: u64 = 1024 * 1024;
/// 最大临时目录大小 (100 GiB, per v0.9.21 商业版估).
pub const MAX_TMP_BYTES: u64 = 100 * 1024 * 1024 * 1024;

// ============================================================================
// §2 ResourceLimits (5 字段, 1:1 翻译 v0.9.21 商业版)
// ============================================================================

/// 沙箱资源限制 (5 字段, K-1 强校验 #1: 编译期 hardcode, 不可运行时增删字段).
///
/// 字段对应 v0.9.21 商业版 `resourceLimits` 对象:
/// - `cpuCores`: CPU 核数
/// - `memoryBytes`: 内存字节
/// - `ioBandwidthBps`: IO 带宽 (bytes/sec)
/// - `networkBandwidthBps`: 网络带宽 (bytes/sec)
/// - `tmpBytes`: 临时目录大小 (bytes)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// CPU 核数 (单位: 核, 范围 [0.1, 64.0]).
    pub cpu_cores: f32,
    /// 内存 (单位: bytes, 范围 [16 MiB, 256 GiB]).
    pub memory_bytes: u64,
    /// IO 带宽 (单位: bytes/sec, 范围 [1 MiB/s, 10 GiB/s]).
    pub io_bandwidth_bps: u64,
    /// 网络带宽 (单位: bytes/sec, 范围 [1 MiB/s, 10 GiB/s]).
    pub network_bandwidth_bps: u64,
    /// 临时目录大小 (单位: bytes, 范围 [1 MiB, 100 GiB]).
    pub tmp_bytes: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            cpu_cores: 1.0,
            memory_bytes: 512 * 1024 * 1024,          // 512 MiB
            io_bandwidth_bps: 100 * 1024 * 1024,      // 100 MiB/s
            network_bandwidth_bps: 100 * 1024 * 1024, // 100 MiB/s
            tmp_bytes: 1024 * 1024 * 1024,            // 1 GiB
        }
    }
}

impl ResourceLimits {
    /// 校验所有 5 资源字段是否在合法范围 (K-1 强校验 #1).
    pub fn validate(&self) -> SandboxResult<()> {
        if self.cpu_cores < MIN_CPU_CORES || self.cpu_cores > MAX_CPU_CORES {
            return Err(SandboxError::InvalidConfig(format!(
                "cpu_cores {} out of range [{}, {}]",
                self.cpu_cores, MIN_CPU_CORES, MAX_CPU_CORES
            )));
        }
        if self.memory_bytes < MIN_MEMORY_BYTES || self.memory_bytes > MAX_MEMORY_BYTES {
            return Err(SandboxError::InvalidConfig(format!(
                "memory_bytes {} out of range [{}, {}]",
                self.memory_bytes, MIN_MEMORY_BYTES, MAX_MEMORY_BYTES
            )));
        }
        if self.io_bandwidth_bps < MIN_IO_BANDWIDTH_BPS
            || self.io_bandwidth_bps > MAX_IO_BANDWIDTH_BPS
        {
            return Err(SandboxError::InvalidConfig(format!(
                "io_bandwidth_bps {} out of range",
                self.io_bandwidth_bps
            )));
        }
        if self.network_bandwidth_bps < MIN_NET_BANDWIDTH_BPS
            || self.network_bandwidth_bps > MAX_NET_BANDWIDTH_BPS
        {
            return Err(SandboxError::InvalidConfig(format!(
                "network_bandwidth_bps {} out of range",
                self.network_bandwidth_bps
            )));
        }
        if self.tmp_bytes < MIN_TMP_BYTES || self.tmp_bytes > MAX_TMP_BYTES {
            return Err(SandboxError::InvalidConfig(format!(
                "tmp_bytes {} out of range",
                self.tmp_bytes
            )));
        }
        Ok(())
    }

    /// 计算总内存人类可读字符串 (per v0.9.21 商业版 `humanReadableMemory`, UI 显示用).
    pub fn human_memory(&self) -> String {
        humanize_bytes(self.memory_bytes)
    }

    /// 计算 CPU 核数人类可读字符串 (per v0.9.21 商业版 `humanReadableCpu`).
    pub fn human_cpu(&self) -> String {
        format!("{:.2} cores", self.cpu_cores)
    }
}

/// 字节数转人类可读 (B / KiB / MiB / GiB / TiB, per IEC binary).
fn humanize_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * KIB;
    const GIB: u64 = 1024 * MIB;
    const TIB: u64 = 1024 * GIB;
    if bytes >= TIB {
        format!("{:.2} TiB", bytes as f64 / TIB as f64)
    } else if bytes >= GIB {
        format!("{:.2} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.2} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.2} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// 资源使用快照 (R21+ 真接时由 cgroup / container stats 读, STUB 模式空).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// 当前 CPU 使用率 (0.0 ~ cpu_cores).
    pub cpu_used: f32,
    /// 当前 RSS 内存 (bytes).
    pub memory_used: u64,
    /// 当前 IO 速率 (bytes/sec, 滑动平均).
    pub io_bps: u64,
    /// 当前网络速率 (bytes/sec, 滑动平均).
    pub net_bps: u64,
    /// 临时目录当前大小 (bytes).
    pub tmp_used: u64,
    /// 采样时间戳.
    pub sampled_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Default 资源限制合法.
    #[test]
    fn resource_limits_default_valid() {
        let limits = ResourceLimits::default();
        assert!(limits.validate().is_ok());
    }

    /// CPU 核数 = 0 触发 K-1 校验 (K-1 强校验 #1).
    #[test]
    fn resource_limits_k1_cpu_cores_zero() {
        let limits = ResourceLimits {
            cpu_cores: 0.0,
            ..Default::default()
        };
        assert!(matches!(
            limits.validate(),
            Err(SandboxError::InvalidConfig(_))
        ));
    }

    /// 内存 = 0 触发 K-1 校验 (K-1 强校验 #1).
    #[test]
    fn resource_limits_k1_memory_zero() {
        let limits = ResourceLimits {
            memory_bytes: 0,
            ..Default::default()
        };
        assert!(matches!(
            limits.validate(),
            Err(SandboxError::InvalidConfig(_))
        ));
    }

    /// 内存超 MAX 触发 K-1 校验.
    #[test]
    fn resource_limits_k1_memory_too_large() {
        let limits = ResourceLimits {
            memory_bytes: MAX_MEMORY_BYTES + 1,
            ..Default::default()
        };
        assert!(matches!(
            limits.validate(),
            Err(SandboxError::InvalidConfig(_))
        ));
    }

    /// IO 带宽超 MAX 触发 K-1 校验.
    #[test]
    fn resource_limits_k1_io_too_large() {
        let limits = ResourceLimits {
            io_bandwidth_bps: MAX_IO_BANDWIDTH_BPS + 1,
            ..Default::default()
        };
        assert!(matches!(
            limits.validate(),
            Err(SandboxError::InvalidConfig(_))
        ));
    }

    /// human_memory 格式化 (K-1 守门: UI 显示必须可读).
    #[test]
    fn resource_limits_human_memory_gi() {
        let limits = ResourceLimits {
            memory_bytes: 2 * 1024 * 1024 * 1024, // 2 GiB
            ..Default::default()
        };
        let s = limits.human_memory();
        assert!(
            s.contains("GiB"),
            "human_memory should contain 'GiB', got {s}"
        );
    }
}
