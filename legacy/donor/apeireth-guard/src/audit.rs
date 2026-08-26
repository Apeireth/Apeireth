//! 隐私审计日志 — 记录每次 PII 检测 / 脱敏事件.
//!
//! 借鉴 VCP PrivacyGuard audit log.
//!
//! 设计:
//! - `PrivacyEvent` 单条事件 (timestamp, kind, action, original_hash, length)
//! - `AuditLog` 内存 ring buffer (固定上限, FIFO 淘汰)
//! - 0 副作用: append 是纯逻辑 (无 IO), 序列化由调用方决定

#![deny(unsafe_code)]

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::pii::{PiiKind, PiiMatch};

/// 隐私事件动作.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrivacyAction {
    /// 检测出 PII (未脱敏)
    Detected,
    /// 已脱敏
    Redacted,
    /// 放行 (检测但未脱敏)
    Allowed,
    /// 拒绝 (含 PII 且策略不允许)
    Denied,
}

impl PrivacyAction {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Detected => "detected",
            Self::Redacted => "redacted",
            Self::Allowed => "allowed",
            Self::Denied => "denied",
        }
    }
}

/// 单条隐私事件.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyEvent {
    /// 事件时间戳 (epoch seconds)
    pub timestamp: i64,
    /// 事件动作
    pub action: PrivacyAction,
    /// PII 类型 (None = 通用)
    pub pii_kind: Option<PiiKind>,
    /// 原始值 SHA256 哈希 (32 字节 hex)
    pub original_hash: String,
    /// 原始值长度
    pub original_length: usize,
    /// 备注 (自由文本)
    pub note: String,
}

/// SHA256 哈希 (64 字符 hex).
pub fn hash_value_sha256(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// 审计日志 — 内存 ring buffer (上限 1024 条, FIFO).
#[derive(Debug)]
pub struct AuditLog {
    capacity: usize,
    events: Mutex<Vec<PrivacyEvent>>,
}

impl AuditLog {
    /// 构造指定容量的审计日志.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            events: Mutex::new(Vec::with_capacity(capacity)),
        }
    }

    /// 默认容量 1024.
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// 追加事件,超出容量 FIFO 淘汰.
    pub fn append(&self, event: PrivacyEvent) {
        let mut guard = self.events.lock();
        if guard.len() >= self.capacity {
            guard.remove(0);
        }
        guard.push(event);
    }

    /// 从 PII 匹配 + 动作生成事件.
    pub fn record_match(
        &self,
        action: PrivacyAction,
        m: &PiiMatch,
        timestamp: i64,
        note: impl Into<String>,
    ) {
        self.append(PrivacyEvent {
            timestamp,
            action,
            pii_kind: Some(m.kind),
            original_hash: hash_value_sha256(&m.value),
            original_length: m.length(),
            note: note.into(),
        });
    }

    /// 当前事件数.
    pub fn len(&self) -> usize {
        self.events.lock().len()
    }

    /// 容量.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 全部事件 (快照).
    pub fn snapshot(&self) -> Vec<PrivacyEvent> {
        self.events.lock().clone()
    }

    /// 按 PII 类型统计.
    pub fn count_by_kind(&self, kind: PiiKind) -> usize {
        self.events
            .lock()
            .iter()
            .filter(|e| e.pii_kind == Some(kind))
            .count()
    }

    /// 按动作统计.
    pub fn count_by_action(&self, action: PrivacyAction) -> usize {
        self.events
            .lock()
            .iter()
            .filter(|e| e.action == action)
            .count()
    }

    /// 清空日志.
    pub fn clear(&self) {
        self.events.lock().clear();
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_match(kind: PiiKind, value: &str, start: usize) -> PiiMatch {
        PiiMatch {
            kind,
            value: value.to_string(),
            start,
            end: start + value.len(),
        }
    }

    #[test]
    fn audit_log_appends_events() {
        let log = AuditLog::new();
        assert_eq!(log.len(), 0);
        let m = make_match(PiiKind::Email, "alice@example.com", 0);
        log.record_match(PrivacyAction::Detected, &m, 100, "test");
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn audit_log_ring_buffer_evicts_oldest() {
        let log = AuditLog::with_capacity(3);
        let m = make_match(PiiKind::Email, "alice@example.com", 0);
        for i in 0..5 {
            log.record_match(PrivacyAction::Detected, &m, 100 + i, "test");
        }
        assert_eq!(log.len(), 3, "ring buffer should cap at 3");
        // 最早 2 条被淘汰
        assert_eq!(log.snapshot()[0].timestamp, 102);
    }

    #[test]
    fn audit_log_filter_by_kind() {
        let log = AuditLog::new();
        let m_email = make_match(PiiKind::Email, "a@b.com", 0);
        let m_ip = make_match(PiiKind::IpAddress, "1.2.3.4", 0);
        log.record_match(PrivacyAction::Detected, &m_email, 1, "");
        log.record_match(PrivacyAction::Detected, &m_email, 2, "");
        log.record_match(PrivacyAction::Detected, &m_ip, 3, "");
        assert_eq!(log.count_by_kind(PiiKind::Email), 2);
        assert_eq!(log.count_by_kind(PiiKind::IpAddress), 1);
        assert_eq!(log.count_by_kind(PiiKind::Phone), 0);
    }

    #[test]
    fn audit_log_filter_by_action() {
        let log = AuditLog::new();
        let m = make_match(PiiKind::Email, "a@b.com", 0);
        log.record_match(PrivacyAction::Detected, &m, 1, "");
        log.record_match(PrivacyAction::Redacted, &m, 2, "");
        log.record_match(PrivacyAction::Redacted, &m, 3, "");
        assert_eq!(log.count_by_action(PrivacyAction::Detected), 1);
        assert_eq!(log.count_by_action(PrivacyAction::Redacted), 2);
    }

    #[test]
    fn hash_sha256_deterministic_64_hex() {
        let h1 = hash_value_sha256("alice@example.com");
        let h2 = hash_value_sha256("alice@example.com");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
        // 全 hex 字符
        assert!(h1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn snapshot_is_clone() {
        let log = AuditLog::new();
        let m = make_match(PiiKind::Email, "a@b.com", 0);
        log.record_match(PrivacyAction::Redacted, &m, 1, "test");
        let snap = log.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].note, "test");
    }
}
