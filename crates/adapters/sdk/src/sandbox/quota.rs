//! # Sandbox quota — 配额记账
//!
//! 多沙箱编排的资源准入面。客户端在**发出创建请求之前**做本地记账预留
//! ([`QuotaLedger::reserve`]), 创建失败 / 销毁 / 回收时归还 ([`QuotaLedger::release`]),
//! 超限一律 [`SandboxError::QuotaExceeded`](crate::sandbox::SandboxError::QuotaExceeded)
//! —— 不静默排队、不假装成功。
//!
//! 记账状态可选**原子落盘** (复用 `apeireth_core::storage_atomic`):
//! 每次预留/归还后以原子替换 + fsync 写出 JSON 快照, 进程崩溃后账目可核对。
//!
//! 记账器内部同步 (跨任务共享安全), 供并发编排共享同一份配额。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use apeireth_core::storage_atomic::{write_atomic_durable, OWNER_ONLY_MODE};

use crate::sandbox::error::{SandboxError, SandboxResult};
use crate::sandbox::resource::ResourceLimits;

/// 配额策略 (编排面准入上限, 3 字段)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuotaPolicy {
    /// 并发活跃沙箱数上限。
    pub max_sandboxes: usize,
    /// 并发 CPU 核数总和上限。
    pub max_cpu_cores: f32,
    /// 并发内存字节总和上限。
    pub max_memory_bytes: u64,
}

impl Default for QuotaPolicy {
    fn default() -> Self {
        Self {
            max_sandboxes: 8,
            max_cpu_cores: 16.0,
            max_memory_bytes: 8 * 1024 * 1024 * 1024,
        }
    }
}

impl QuotaPolicy {
    /// 校验策略自身合法 (上限为 0 视为配置错误, 不是"禁止一切"的静默语义;
    /// NaN 上限同样拒绝)。
    pub fn validate(&self) -> SandboxResult<()> {
        if self.max_sandboxes == 0 {
            return Err(SandboxError::InvalidConfig(
                "quota.max_sandboxes must be non-zero".into(),
            ));
        }
        if self.max_cpu_cores.is_nan() || self.max_cpu_cores <= 0.0 {
            return Err(SandboxError::InvalidConfig(
                "quota.max_cpu_cores must be positive (NaN rejected)".into(),
            ));
        }
        if self.max_memory_bytes == 0 {
            return Err(SandboxError::InvalidConfig(
                "quota.max_memory_bytes must be non-zero".into(),
            ));
        }
        Ok(())
    }
}

/// 记账快照 (可序列化, 供落盘 / 巡检报告)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuotaSnapshot {
    /// 策略。
    pub policy: QuotaPolicy,
    /// 当前预留数。
    pub active_sandboxes: usize,
    /// 当前预留 CPU 核数总和。
    pub cpu_cores_in_use: f32,
    /// 当前预留内存字节总和。
    pub memory_bytes_in_use: u64,
    /// 快照时间 (epoch millis)。
    pub recorded_at_ms: i64,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_millis()).unwrap_or(0))
        .unwrap_or(0)
}

#[derive(Debug, Default)]
struct LedgerInner {
    reservations: HashMap<Uuid, ResourceLimits>,
    cpu_in_use: f32,
    memory_in_use: u64,
}

/// 配额记账器: 内部同步, 可跨任务共享 (`clone()` 得到同一账本的另一引用)。
#[derive(Debug, Clone)]
pub struct QuotaLedger {
    policy: QuotaPolicy,
    state_path: Option<PathBuf>,
    inner: std::sync::Arc<Mutex<LedgerInner>>,
}

impl QuotaLedger {
    /// 新账本 (不落盘)。
    pub fn new(policy: QuotaPolicy) -> SandboxResult<Self> {
        policy.validate()?;
        Ok(Self {
            policy,
            state_path: None,
            inner: std::sync::Arc::new(Mutex::new(LedgerInner::default())),
        })
    }

    /// 绑定状态文件: 每次记账变更原子落盘 (权限 600)。
    pub fn with_state_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.state_path = Some(path.into());
        self
    }

    /// 当前策略。
    pub fn policy(&self) -> &QuotaPolicy {
        &self.policy
    }

    /// 状态文件路径 (None = 纯内存账本)。
    pub fn state_path(&self) -> Option<&Path> {
        self.state_path.as_deref()
    }

    /// 预留一份资源配额。超限返 [`SandboxError::QuotaExceeded`], 不留部分账。
    pub fn reserve(&self, id: Uuid, limits: &ResourceLimits) -> SandboxResult<()> {
        let mut inner = self.lock();
        if inner.reservations.contains_key(&id) {
            return Err(SandboxError::InvalidState(format!(
                "quota already reserved for {id}"
            )));
        }
        let active = inner.reservations.len() + 1;
        let cpu = inner.cpu_in_use + limits.cpu_cores;
        let memory = inner.memory_in_use + limits.memory_bytes;

        if active > self.policy.max_sandboxes {
            return Err(SandboxError::QuotaExceeded(format!(
                "sandbox count {active} exceeds limit {}",
                self.policy.max_sandboxes
            )));
        }
        if cpu > self.policy.max_cpu_cores {
            return Err(SandboxError::QuotaExceeded(format!(
                "cpu cores {cpu} exceed limit {}",
                self.policy.max_cpu_cores
            )));
        }
        if memory > self.policy.max_memory_bytes {
            return Err(SandboxError::QuotaExceeded(format!(
                "memory bytes {memory} exceed limit {}",
                self.policy.max_memory_bytes
            )));
        }

        inner.reservations.insert(id, limits.clone());
        inner.cpu_in_use = cpu;
        inner.memory_in_use = memory;
        drop(inner);
        self.persist()
    }

    /// 归还预留 (幂等: 未预留的 ID 返 [`SandboxError::NotFound`])。
    pub fn release(&self, id: &Uuid) -> SandboxResult<()> {
        let mut inner = self.lock();
        let limits = inner
            .reservations
            .remove(id)
            .ok_or_else(|| SandboxError::NotFound {
                sandbox_id: id.to_string(),
            })?;
        inner.cpu_in_use = (inner.cpu_in_use - limits.cpu_cores).max(0.0);
        inner.memory_in_use = inner.memory_in_use.saturating_sub(limits.memory_bytes);
        drop(inner);
        self.persist()
    }

    /// 当前账目快照。
    pub fn snapshot(&self) -> QuotaSnapshot {
        let inner = self.lock();
        QuotaSnapshot {
            policy: self.policy.clone(),
            active_sandboxes: inner.reservations.len(),
            cpu_cores_in_use: inner.cpu_in_use,
            memory_bytes_in_use: inner.memory_in_use,
            recorded_at_ms: now_ms(),
        }
    }

    /// 某 ID 是否已预留。
    pub fn is_reserved(&self, id: &Uuid) -> bool {
        self.lock().reservations.contains_key(id)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, LedgerInner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn persist(&self) -> SandboxResult<()> {
        let Some(path) = &self.state_path else {
            return Ok(());
        };
        let bytes = serde_json::to_vec_pretty(&self.snapshot())?;
        write_atomic_durable(path, &bytes, OWNER_ONLY_MODE).map_err(SandboxError::Io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits(cpu: f32, mem: u64) -> ResourceLimits {
        ResourceLimits {
            cpu_cores: cpu,
            memory_bytes: mem,
            ..ResourceLimits::default()
        }
    }

    /// 配额边界: 恰好用满允许, 超一分拒绝, 归还后恢复。
    #[test]
    fn quota_boundary_allows_exactly_at_limit_and_rejects_over() {
        let ledger = QuotaLedger::new(QuotaPolicy {
            max_sandboxes: 2,
            max_cpu_cores: 2.0,
            max_memory_bytes: 1024,
        })
        .unwrap();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        ledger.reserve(a, &limits(1.0, 512)).expect("first fits");
        ledger
            .reserve(b, &limits(1.0, 512))
            .expect("exactly at limit");
        let err = ledger.reserve(c, &limits(1.0, 512)).unwrap_err();
        assert!(matches!(err, SandboxError::QuotaExceeded(_)), "got {err:?}");

        ledger.release(&a).expect("release restores capacity");
        ledger
            .reserve(c, &limits(1.0, 512))
            .expect("capacity restored");
    }

    /// 单维超限 (CPU / 内存 / 数量) 各自独立拒绝。
    #[test]
    fn each_quota_dimension_rejects_independently() {
        let ledger = QuotaLedger::new(QuotaPolicy {
            max_sandboxes: 10,
            max_cpu_cores: 1.0,
            max_memory_bytes: 1024,
        })
        .unwrap();
        ledger
            .reserve(Uuid::new_v4(), &limits(0.5, 64))
            .expect("first fits");
        let err = ledger.reserve(Uuid::new_v4(), &limits(0.6, 64));
        assert!(matches!(err, Err(SandboxError::QuotaExceeded(_))));

        let ledger = QuotaLedger::new(QuotaPolicy {
            max_sandboxes: 10,
            max_cpu_cores: 8.0,
            max_memory_bytes: 256,
        })
        .unwrap();
        let err = ledger.reserve(Uuid::new_v4(), &limits(1.0, 512));
        assert!(matches!(err, Err(SandboxError::QuotaExceeded(_))));
    }

    /// 重复预留 / 未预留归还的错误分类。
    #[test]
    fn double_reserve_and_unknown_release_close_to_the_vocabulary() {
        let ledger = QuotaLedger::new(QuotaPolicy::default()).unwrap();
        let id = Uuid::new_v4();
        ledger.reserve(id, &limits(0.5, 64)).unwrap();
        let err = ledger.reserve(id, &limits(0.5, 64)).unwrap_err();
        assert!(matches!(err, SandboxError::InvalidState(_)));
        ledger.release(&id).unwrap();
        let err = ledger.release(&id).unwrap_err();
        assert!(matches!(err, SandboxError::NotFound { .. }));
    }

    /// 记账落盘: 原子写快照文件, 内容可反序列化且与账本一致。
    #[test]
    fn quota_snapshot_persists_atomically_to_state_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("quota.json");
        let ledger = QuotaLedger::new(QuotaPolicy::default())
            .unwrap()
            .with_state_file(&path);
        ledger.reserve(Uuid::new_v4(), &limits(2.0, 1024)).unwrap();

        let bytes = fs_err::read(&path).expect("state file written");
        let snapshot: QuotaSnapshot = serde_json::from_slice(&bytes).expect("valid json");
        assert_eq!(snapshot.active_sandboxes, 1);
        assert!((snapshot.cpu_cores_in_use - 2.0).abs() < 1e-6);
        assert_eq!(snapshot.memory_bytes_in_use, 1024);
        assert_eq!(snapshot.policy, ledger.policy().clone());
    }

    /// 策略自身校验: 0 值 / NaN 上限是配置错误。
    #[test]
    fn quota_policy_rejects_zero_and_nan_limits() {
        assert!(QuotaPolicy::default().validate().is_ok());
        let bad = QuotaPolicy {
            max_sandboxes: 0,
            ..QuotaPolicy::default()
        };
        assert!(matches!(
            bad.validate(),
            Err(SandboxError::InvalidConfig(_))
        ));
        let bad = QuotaPolicy {
            max_cpu_cores: f32::NAN,
            ..QuotaPolicy::default()
        };
        assert!(matches!(
            bad.validate(),
            Err(SandboxError::InvalidConfig(_))
        ));
    }
}
