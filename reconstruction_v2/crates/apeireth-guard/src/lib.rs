//! apeireth-guard: Privacy Guard (VCP 模式 3/8 — 隐私卫士).
//!
//! **职责**: 检测 + 脱敏 + 审计 文本中的 PII.
//!
//! 借鉴 VCP PrivacyGuard (字段级: 5 类 PII + 4 类脱敏策略 + 审计日志).
//!
//! **v2 重构说明**:
//! - 与 v1 era 等价行为, 仅迁移到 v2 crate 形态.
//! - v1 的 `audit.rs` (审计日志) 与 `organ_kani_proofs.rs` 在 v2 不再作为独立模块,
//!   其核心类型 [PrivacyAction] / [PrivacyEvent] / [AuditLog] / [hash_value_sha256]
//!   内联在本 crate root, 对外 re-export 不变 (向后兼容 v1 调用方).
//! - v1 路径中所有可观测行为 (PII 类别 / 脱敏策略 / 审查启发式 / Untrusted 边界标记)
//!   保持一致, 不漂移.
//!
//! **不漂移**:
//! - 0 副作用: 检测 / 脱敏是纯函数; 审计是内存 ring buffer
//! - 接口与 v1 等价 (PrivacyGuard / detect_pii / redact_* / audit_tool_description / wrap_untrusted).

#![deny(unsafe_code)]

use std::collections::VecDeque;

use parking_lot::Mutex;
use sha2::{Digest, Sha256};

pub mod pii;
pub mod redactor;
pub mod tool_desc_audit;
pub mod untrusted_mark;

// Re-exports 公共 API
pub use pii::{detect_pii, PiiKind, PiiMatch};
pub use redactor::{hash_value_sha256, redact_one, redact_text, RedactionStrategy};
pub use tool_desc_audit::{
    audit_tool_description, description_changed, DefaultToolDescAuditor, DescAuditRecord,
    DescAuditReport, DescFinding, DescFindingKind, DescVerdict, ToolDescAuditLog,
    ToolDescriptionAuditor,
};
pub use untrusted_mark::{
    escape_untrusted_content, wrap_untrusted, DefaultUntrustedMarker, UntrustedMarker,
    UntrustedSource,
};

// ============================================
// v1 audit.rs 内容的内联 (v2 不再作为独立 module)
// - PrivacyAction / PrivacyEvent / AuditLog 是门面所需的依赖项.
// - 行为与 v1 完全一致: ring buffer (默认 1024 容量) + SHA256 哈希原值 + Mutex 保护.
// ============================================

/// 隐私事件动作.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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

/// 单条隐私事件.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PrivacyEvent {
    /// 事件时间戳 (epoch seconds, 由调用方注入)
    pub timestamp: i64,
    /// PII 类型
    pub kind: PiiKind,
    /// 动作
    pub action: PrivacyAction,
    /// 原值 SHA256 (前 16 字符, 不暴露原值)
    pub original_hash: String,
    /// 原值长度 (字节)
    pub length: usize,
    /// 备注 / 上下文 (eg. strategy=Mask)
    pub note: String,
}

/// 隐私审计日志 — 内存 ring buffer (固定上限, FIFO 淘汰).
#[derive(Debug)]
pub struct AuditLog {
    capacity: usize,
    entries: Mutex<VecDeque<PrivacyEvent>>,
}

impl AuditLog {
    /// 默认容量 1024.
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// 自定义容量.
    pub fn with_capacity(capacity: usize) -> Self {
        let cap = capacity.max(1);
        Self {
            capacity: cap,
            entries: Mutex::new(VecDeque::with_capacity(cap)),
        }
    }

    /// 当前事件数.
    pub fn len(&self) -> usize {
        self.entries.lock().len()
    }

    /// 是否为空.
    pub fn is_empty(&self) -> bool {
        self.entries.lock().is_empty()
    }

    /// 容量.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 追加一条事件 (超容量挤掉最旧).
    pub fn append(&self, event: PrivacyEvent) {
        let mut entries = self.entries.lock();
        if entries.len() >= self.capacity {
            entries.pop_front();
        }
        entries.push_back(event);
    }

    /// 便捷: 记录一条 PII 匹配事件.
    pub fn record_match(
        &self,
        action: PrivacyAction,
        m: &PiiMatch,
        timestamp: i64,
        note: String,
    ) {
        self.append(PrivacyEvent {
            timestamp,
            kind: m.kind,
            action,
            original_hash: hash_value_sha256(&m.value),
            length: m.value.len(),
            note,
        });
    }

    /// 快照 (旧→新).
    pub fn snapshot(&self) -> Vec<PrivacyEvent> {
        self.entries.lock().iter().cloned().collect()
    }

    /// 统计某动作的事件数.
    pub fn count_by_action(&self, action: PrivacyAction) -> usize {
        self.entries
            .lock()
            .iter()
            .filter(|e| e.action == action)
            .count()
    }

    /// 清空.
    pub fn clear(&self) {
        self.entries.lock().clear();
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

/// 脱敏结果 — PrivacyGuard 入口返回.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactionResult {
    /// 脱敏后文本
    pub redacted_text: String,
    /// 检测到的 PII 匹配 (按 start 升序)
    pub matches: Vec<PiiMatch>,
}

/// PrivacyGuard — 顶层门面, 协调检测 + 脱敏 + 审计.
#[derive(Debug)]
pub struct PrivacyGuard {
    /// 审计日志
    audit: AuditLog,
    /// 默认脱敏策略
    strategy: RedactionStrategy,
    /// 是否启用审计记录
    audit_enabled: bool,
}

impl PrivacyGuard {
    /// 构造默认 PrivacyGuard (Mask 策略 + 1024 容量审计).
    pub fn new() -> Self {
        Self {
            audit: AuditLog::new(),
            strategy: RedactionStrategy::Mask,
            audit_enabled: true,
        }
    }

    /// 自定义策略构造.
    pub fn with_strategy(strategy: RedactionStrategy) -> Self {
        Self {
            audit: AuditLog::new(),
            strategy,
            audit_enabled: true,
        }
    }

    /// 指定审计容量.
    pub fn with_audit_capacity(mut self, capacity: usize) -> Self {
        self.audit = AuditLog::with_capacity(capacity);
        self
    }

    /// 关闭审计 (per 选项).
    pub fn without_audit(mut self) -> Self {
        self.audit_enabled = false;
        self
    }

    /// 顶层入口: 检测 + 脱敏 + 审计一条记录.
    pub fn check_and_redact(&self, text: &str, timestamp: i64) -> RedactionResult {
        let matches = detect_pii(text);
        let redacted = redact_text(text, &matches, self.strategy);
        if self.audit_enabled {
            for m in &matches {
                self.audit.record_match(
                    PrivacyAction::Redacted,
                    m,
                    timestamp,
                    format!("strategy={:?}", self.strategy),
                );
            }
        }
        RedactionResult {
            redacted_text: redacted,
            matches,
        }
    }

    /// 仅检测 (不脱敏, 仅审计 "Detected" 事件).
    pub fn detect_only(&self, text: &str, timestamp: i64) -> Vec<PiiMatch> {
        let matches = detect_pii(text);
        if self.audit_enabled {
            for m in &matches {
                self.audit
                    .record_match(PrivacyAction::Detected, m, timestamp, "detect-only");
            }
        }
        matches
    }

    /// 审计日志引用.
    pub fn audit(&self) -> &AuditLog {
        &self.audit
    }

    /// 当前策略.
    pub fn strategy(&self) -> RedactionStrategy {
        self.strategy
    }

    /// 设置策略 (mutating).
    pub fn set_strategy(&mut self, strategy: RedactionStrategy) {
        self.strategy = strategy;
    }
}

impl Default for PrivacyGuard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn privacy_guard_redacts_email() {
        let g = PrivacyGuard::new();
        let r = g.check_and_redact("contact alice@example.com today", 1_700_000_000);
        assert!(r.matches.iter().any(|m| m.kind == PiiKind::Email));
        assert!(r.redacted_text.contains('*'));
    }

    #[test]
    fn privacy_guard_audit_records_events() {
        let g = PrivacyGuard::new();
        let _ = g.check_and_redact("alice@example.com and 192.168.1.1", 1_700_000_000);
        assert_eq!(g.audit().len(), 2);
    }

    #[test]
    fn privacy_guard_no_pii_clean_text() {
        let g = PrivacyGuard::new();
        let r = g.check_and_redact("the sky is blue", 1_700_000_000);
        assert_eq!(r.matches.len(), 0);
        assert_eq!(r.redacted_text, "the sky is blue");
        assert_eq!(g.audit().len(), 0);
    }

    #[test]
    fn privacy_guard_without_audit() {
        let g = PrivacyGuard::new().without_audit();
        let _ = g.check_and_redact("alice@example.com", 1_700_000_000);
        assert_eq!(g.audit().len(), 0);
    }

    #[test]
    fn privacy_guard_strategy_replace_label() {
        let g = PrivacyGuard::with_strategy(RedactionStrategy::ReplaceLabel);
        let r = g.check_and_redact("alice@example.com", 1_700_000_000);
        assert_eq!(r.redacted_text, "[EMAIL]");
    }

    #[test]
    fn detect_only_returns_matches_no_redaction() {
        let g = PrivacyGuard::new();
        let matches = g.detect_only("a@b.com here", 1_700_000_000);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].kind, PiiKind::Email);
        assert_eq!(g.audit().count_by_action(PrivacyAction::Detected), 1);
        assert_eq!(g.audit().count_by_action(PrivacyAction::Redacted), 0);
    }

    #[test]
    fn set_strategy_mutates() {
        let mut g = PrivacyGuard::new();
        assert_eq!(g.strategy(), RedactionStrategy::Mask);
        g.set_strategy(RedactionStrategy::Hash);
        assert_eq!(g.strategy(), RedactionStrategy::Hash);
    }

    #[test]
    fn privacy_guard_redacts_env_secret_and_token() {
        // ae12d9eb 增量: 门面级 env 行级 + 密钥 token 脱敏 (含审计)
        let g = PrivacyGuard::new();
        let text = "export OPENAI_API_KEY=sk-1234567890abcdefghijklmnopqrstuv";
        let r = g.check_and_redact(text, 1_700_000_000);
        assert!(
            r.redacted_text.starts_with("export OPENAI_API_KEY="),
            "KEY= 前缀保留"
        );
        assert!(
            !r.redacted_text.contains("1234567890"),
            "密钥主体不可见: {}",
            r.redacted_text
        );
        assert!(r.matches.iter().any(|m| m.kind == PiiKind::EnvSecret));
        assert!(g.audit().len() >= 1, "应写审计");
    }

    #[test]
    fn privacy_guard_normal_text_not_touched() {
        // ae12d9eb 增量: 正常文本不误伤 (误报控制证据)
        let g = PrivacyGuard::new();
        let text = "LOG_LEVEL=debug\nflask-mode is fine\nthe quick brown fox jumps";
        let r = g.check_and_redact(text, 1_700_000_000);
        assert_eq!(r.matches.len(), 0, "正常文本不应检出: {:?}", r.matches);
        assert_eq!(r.redacted_text, text);
    }

    // ============================================
    // v2 重构: 内联 AuditLog 的本地行为测试 (v1 audit.rs 等价)
    // ============================================

    #[test]
    fn audit_log_ring_buffer_overflow() {
        let log = AuditLog::with_capacity(2);
        log.append(PrivacyEvent {
            timestamp: 1,
            kind: PiiKind::Email,
            action: PrivacyAction::Redacted,
            original_hash: "abc".into(),
            length: 3,
            note: String::new(),
        });
        log.append(PrivacyEvent {
            timestamp: 2,
            kind: PiiKind::Email,
            action: PrivacyAction::Redacted,
            original_hash: "def".into(),
            length: 3,
            note: String::new(),
        });
        log.append(PrivacyEvent {
            timestamp: 3,
            kind: PiiKind::Email,
            action: PrivacyAction::Redacted,
            original_hash: "ghi".into(),
            length: 3,
            note: String::new(),
        });
        assert_eq!(log.len(), 2, "超容量应挤掉最旧");
        let snap = log.snapshot();
        assert_eq!(snap[0].timestamp, 2, "t=1 应被挤出");
        assert_eq!(snap[1].timestamp, 3);
    }

    #[test]
    fn audit_log_count_by_action_and_clear() {
        let log = AuditLog::new();
        log.append(PrivacyEvent {
            timestamp: 0,
            kind: PiiKind::Email,
            action: PrivacyAction::Detected,
            original_hash: String::new(),
            length: 0,
            note: String::new(),
        });
        log.append(PrivacyEvent {
            timestamp: 0,
            kind: PiiKind::Phone,
            action: PrivacyAction::Redacted,
            original_hash: String::new(),
            length: 0,
            note: String::new(),
        });
        log.append(PrivacyEvent {
            timestamp: 0,
            kind: PiiKind::Ssn,
            action: PrivacyAction::Redacted,
            original_hash: String::new(),
            length: 0,
            note: String::new(),
        });
        assert_eq!(log.count_by_action(PrivacyAction::Detected), 1);
        assert_eq!(log.count_by_action(PrivacyAction::Redacted), 2);
        log.clear();
        assert!(log.is_empty());
    }
}
