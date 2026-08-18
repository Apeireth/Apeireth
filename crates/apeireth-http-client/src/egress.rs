//! `apeireth-gateway::egress` — S4 出站网络策略 (域名/协议白名单默认拒绝 + 审计哈希链).
//!
//! ## 定位 (安全调研 S4, 官方边界三线: 安全底线永不外包)
//!
//! gateway 是 LLM 出站网关 — 所有外发请求先过本策略:
//! - **默认拒绝**: 不在白名单的域名 → Err (0 装 PASS: 不静默放行).
//! - **协议约束**: 仅允许 https (http 拒绝; 本地回环例外需显式声明).
//! - **审计哈希链**: 每次外发 (允许或拒绝) 进链, 每条含 prev_hash (SHA-256) —
//!   不可篡改审计 (与 `apeireth-arbitration` HASH-SQL 同哲学, 此处内存链 + trait 口).
//! - **预算联动**: `budget_hook` trait 口 — 外发扣减与 spend 预算联动 (调用方接).
//!
//! ## 0 装 PASS
//!
//! - 本模块是**纯策略件** (无 IO): 解析/白名单/审计链全确定性可测.
//! - 真网络请求仍走既有 transport; 本模块只回答"该不该放行 + 留痕".
//! - 域名解析不做 DNS (host 从 URL 提取, 防 SSRF 由调用方在连接层配合).

use std::collections::HashMap;

/// 出站策略错误 (可行动信息).
#[derive(Debug, Clone, PartialEq)]
pub enum EgressError {
    /// 域名不在白名单 (默认拒绝).
    DomainNotAllowed(String),
    /// 协议非 https (或未显式放行).
    ProtocolNotAllowed(String),
    /// URL 无法解析.
    InvalidUrl(String),
    /// 预算不足 (与 spend 联动).
    BudgetExceeded { needed: f64, left: f64 },
}

/// 审计链条目 (append-only 哈希链).
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub seq: u64,
    pub url: String,
    pub allowed: bool,
    pub reason: String,
    pub at_ms: i64,
    pub prev_hash: String,
    pub hash: String,
}

/// 出站策略配置.
#[derive(Debug, Clone)]
pub struct EgressConfig {
    /// 允许的域名 (精确或后缀匹配: "api.example.com" 精确, "*.example.com" 后缀).
    pub allowlist: Vec<String>,
    /// 允许 http 的域名 (默认只 https; 本地开发例外显式声明).
    pub allow_http: Vec<String>,
}

impl Default for EgressConfig {
    fn default() -> Self {
        Self {
            allowlist: Vec::new(),
            allow_http: Vec::new(),
        }
    }
}

/// 预算钩子 (与 spend 预算联动 — 调用方接, 0 装: 默认不限额).
pub trait BudgetHook: Send + Sync + std::fmt::Debug {
    /// 外发前检查预算 (需要 = 该请求的成本估计).
    fn check(&self, url: &str, needed: f64) -> Result<(), EgressError>;
}

/// 默认无预算钩子 (诚实: 未接预算联动).
#[derive(Debug, Default)]
pub struct NoBudget;

impl BudgetHook for NoBudget {
    fn check(&self, _url: &str, _needed: f64) -> Result<(), EgressError> {
        Ok(())
    }
}

/// 出站网络策略 (确定性).
#[derive(Debug)]
pub struct EgressPolicy {
    config: EgressConfig,
    chain: Vec<AuditEntry>,
    budget: Box<dyn BudgetHook>,
    next_seq: u64,
}

impl EgressPolicy {
    pub fn new(config: EgressConfig) -> Self {
        Self {
            config,
            chain: Vec::new(),
            budget: Box::new(NoBudget),
            next_seq: 1,
        }
    }

    pub fn with_budget(mut self, hook: Box<dyn BudgetHook>) -> Self {
        self.budget = hook;
        self
    }

    /// 出站检查: 默认拒绝 + https 协议 + 预算联动. 通过 → Ok, 并记审计.
    pub fn check_outbound(&mut self, url: &str, cost: f64) -> Result<(), EgressError> {
        // 预算联动
        self.budget.check(url, cost)?;
        // URL 解析
        let (scheme, host) = parse_url(url).ok_or_else(|| EgressError::InvalidUrl(url.into()))?;
        // 协议
        if scheme != "https"
            && !self
                .config
                .allow_http
                .iter()
                .any(|h| host_matches(h.as_str(), &host))
        {
            self.audit(url, false, format!("协议 {scheme} 不允许"));
            return Err(EgressError::ProtocolNotAllowed(scheme.clone()));
        }
        // 域名白名单 (默认拒绝)
        let allowed = self
            .config
            .allowlist
            .iter()
            .any(|rule| host_matches(rule.as_str(), &host));
        if !allowed {
            self.audit(url, false, format!("域名 {host} 不在白名单 (默认拒绝)"));
            return Err(EgressError::DomainNotAllowed(host.clone()));
        }
        self.audit(url, true, "白名单命中");
        Ok(())
    }

    /// 审计链 (append-only, 每条约 128 hex 字符; 截断防无限增长由调用方接 event_log).
    pub fn audit(&mut self, url: &str, allowed: bool, reason: impl Into<String>) -> &AuditEntry {
        let at_ms = chrono::Utc::now().timestamp_millis();
        let reason = reason.into();
        let prev_hash = self
            .chain
            .last()
            .map(|e| e.hash.clone())
            .unwrap_or_default();
        let raw = format!("{url}|{allowed}|{reason}|{at_ms}|{prev_hash}");
        let hash = sha256_hex(&raw);
        let entry = AuditEntry {
            seq: self.next_seq,
            url: url.to_string(),
            allowed,
            reason,
            at_ms,
            prev_hash,
            hash,
        };
        self.next_seq += 1;
        self.chain.push(entry);
        self.chain.last().unwrap()
    }

    /// 审计链完整性验证 (自检: 每条 prev_hash == 前条 hash).
    pub fn verify_chain(&self) -> bool {
        for w in self.chain.windows(2) {
            if w[1].prev_hash != w[0].hash {
                return false;
            }
        }
        true
    }

    pub fn audit_len(&self) -> usize {
        self.chain.len()
    }
}

/// 简易 URL 解析 (scheme://host[:port]/path; 无外部依赖, 不 DNS).
fn parse_url(url: &str) -> Option<(String, String)> {
    let (scheme, rest) = url.split_once("://")?;
    if scheme.is_empty() || rest.is_empty() {
        return None;
    }
    let host_port = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = host_port.split(':').next().unwrap_or("").to_lowercase();
    if host.is_empty() {
        return None;
    }
    Some((scheme.to_lowercase(), host))
}

/// 白名单匹配: "api.x.com" 精确; "*.x.com" 后缀 (host 是 x.com 或 *.x.com 的子域).
fn host_matches(rule: &str, host: &str) -> bool {
    let rule = rule.trim().to_lowercase();
    if let Some(suffix) = rule.strip_prefix("*.") {
        host == suffix || host.ends_with(&format!(".{suffix}"))
    } else {
        host == rule
    }
}

/// SHA-256 hex (复用 workspace sha2; 失败理论不可达, 0 装: 不静默).
fn sha256_hex(raw: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    let out = hasher.finalize();
    out.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_deny_outside_allowlist() {
        let mut p = EgressPolicy::new(EgressConfig {
            allowlist: vec!["api.apeireth.ai".into(), "*.llm.example.com".into()],
            ..Default::default()
        });
        assert!(p
            .check_outbound("https://api.apeireth.ai/v1/chat", 1.0)
            .is_ok());
        assert!(p
            .check_outbound("https://sub.llm.example.com/x", 1.0)
            .is_ok());
        let err = p
            .check_outbound("https://evil.example.org", 1.0)
            .unwrap_err();
        assert!(matches!(err, EgressError::DomainNotAllowed(_)), "{err:?}");
        // 拒绝也留痕
        assert_eq!(p.audit_len(), 3);
    }

    #[test]
    fn https_only_by_default() {
        let mut p = EgressPolicy::new(EgressConfig {
            allowlist: vec!["api.apeireth.ai".into()],
            ..Default::default()
        });
        let err = p
            .check_outbound("http://api.apeireth.ai/x", 1.0)
            .unwrap_err();
        assert!(matches!(err, EgressError::ProtocolNotAllowed(_)));
        // 显式放行 http
        let mut p2 = EgressPolicy::new(EgressConfig {
            allowlist: vec!["api.apeireth.ai".into(), "localhost".into()],
            allow_http: vec!["localhost".into()],
            ..Default::default()
        });
        assert!(p2.check_outbound("http://localhost:8080/x", 1.0).is_ok());
    }

    #[test]
    fn audit_chain_verifies_and_tamper_detected() {
        let mut p = EgressPolicy::new(EgressConfig {
            allowlist: vec!["a.com".into()],
            ..Default::default()
        });
        p.check_outbound("https://a.com/1", 1.0).unwrap();
        p.check_outbound("https://a.com/2", 1.0).unwrap();
        p.check_outbound("https://b.com/3", 1.0).unwrap_err(); // 拒绝也进链
        assert!(p.verify_chain(), "链应完整");
        // 篡改中间条目 → 链断裂
        p.chain[1].hash = "deadbeef".into();
        assert!(!p.verify_chain(), "篡改应被检出");
    }

    #[test]
    fn budget_hook_blocks() {
        #[derive(Debug)]
        struct Tight;
        impl BudgetHook for Tight {
            fn check(&self, _url: &str, needed: f64) -> Result<(), EgressError> {
                if needed > 5.0 {
                    Err(EgressError::BudgetExceeded { needed, left: 5.0 })
                } else {
                    Ok(())
                }
            }
        }
        let mut p = EgressPolicy::new(EgressConfig {
            allowlist: vec!["a.com".into()],
            ..Default::default()
        })
        .with_budget(Box::new(Tight));
        assert!(p.check_outbound("https://a.com/x", 3.0).is_ok());
        let err = p.check_outbound("https://a.com/big", 10.0).unwrap_err();
        assert!(matches!(err, EgressError::BudgetExceeded { .. }));
    }

    #[test]
    fn wildcard_suffix_matching() {
        let mut p = EgressPolicy::new(EgressConfig {
            allowlist: vec!["*.example.com".into()],
            ..Default::default()
        });
        assert!(p.check_outbound("https://api.example.com/v1", 1.0).is_ok());
        assert!(p.check_outbound("https://example.com/v1", 1.0).is_ok());
        assert!(
            p.check_outbound("https://fakeexample.com/v1", 1.0).is_err(),
            "fakeexample.com 不应命中 *.example.com"
        );
    }
}
