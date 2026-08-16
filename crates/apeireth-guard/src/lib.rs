//! apeireth-guard: Privacy Guard (VCP 模式 3/8 — 隐私卫士).
//!
//! **职责**: 检测 + 脱敏 + 审计 文本中的 PII.
//!
//! 借鉴 VCP PrivacyGuard (字段级: 5 类 PII + 4 类脱敏策略 + 审计日志).
//!
//! **不漂移**:
//! - 0 改 VCP 任何内部代码
//! - 0 副作用: 检测 / 脱敏是纯函数; 审计是内存 ring buffer
//!
//! **当前状态**: R173 阶段 5 VCP 模式 3 落地 (PII 检测 + 脱敏 + 审计最小骨架).

#![deny(unsafe_code)]

pub mod audit;
pub mod pii;
// R177: guard invariants (12 tests + 2 Kani proofs)
mod organ_kani_proofs;
pub mod redactor;

// Re-exports 公共 API
pub use audit::{hash_value_sha256, AuditLog, PrivacyAction, PrivacyEvent};
pub use pii::{detect_pii, PiiKind, PiiMatch};
pub use redactor::{redact_one, redact_text, RedactionStrategy};

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
}
