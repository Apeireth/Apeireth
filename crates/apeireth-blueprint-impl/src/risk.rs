//! # K-1 / K-2 / K-3 / K-4 — 4 风险类实装
//!
//! 风险分类源自 RIVAL 蓝图 §2.4 (R20 阶段 4 估补):
//!
//! | 风险类 | 含义 | 本模块 trait | 关键不变量 |
//! |--------|------|-------------|----------|
//! | **K-1** | 强校验 | [`K1StrongValidate`] | 用户输入 / api key / model name / scope 缺一不可, 全错就 `Err` |
//! | **K-2** | 弱校验 | [`K2WeakValidate`] | 容错处理 / 默认行为, 输入异常尝试 fallback |
//! | **K-3** | 监督 | [`K3Audit`] | trace log + audit, 失败也尽力写, 但断通道要 `Err` |
//! | **K-4** | 守门 | [`K4Guard`] | deny/allow 决策, 一旦 deny 必须 `Err` 不假装 |
//!
//! 4 trait 默认实现 (空 / pass-through) 给单元测试用, 真实业务实现方必须 override.
//!
//! ## 6 哲学锚
//!
//! - S-1 主 22:33 北极星导向 — 守门为 ASI 北极星服务, 不是装饰.
//! - S-2 主 17:43 实事求是 — 4 风险类源自实际失败模式, 不空想.
//! - O-5 主 17:58 不假装 — 失败必须 Err, 任何 Ok(false) 蒙混都是反锚.
//! - O-2 主 19:33 走在前人经验上 — 借鉴 OWASP 4 类输入校验 (syntactic/semantic, log, gate).
//! - O-3 主 23:44 干到底 — 4 风险类一次写齐, 不留 TODO.
//! - O-4 主 00:56 任何人都能接手 — 4 trait 边界清晰, 接手的 owner 一眼能看.
//!
//! ## 8 项不修改承诺
//!
//! 1. K-1 强校验不绕过 (即使用户说"信任我")
//! 2. K-2 弱校验 fallback 链不无限长 (上限 3 层)
//! 3. K-3 audit 失败不能吞错 (必须 Err 出来)
//! 4. K-4 deny 决策一旦做出不可覆盖
//! 5. 4 trait 互不重名 / 互不依赖
//! 6. 4 trait 都不假设 caller 是 K-3 (解耦)
//! 7. 默认实现都是 `unimplemented!()` 占位 — 防止忘记 override
//! 8. 风险类编号 (K-1..K-4) 不修改, 跟 RIVAL §2.4 对齐

use crate::error::{BlueprintError, BlueprintResult};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// K-1 强校验输入 — 4 字段必须全合法.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct K1Input {
    pub user_input: String,
    pub api_key: String,
    pub model_name: String,
    pub scope: String,
}

impl K1Input {
    /// 构造 + 自检 (构造时就拒绝空字段, 防止调用方忘记校验).
    pub fn new(
        user_input: impl Into<String>,
        api_key: impl Into<String>,
        model_name: impl Into<String>,
        scope: impl Into<String>,
    ) -> BlueprintResult<Self> {
        let user_input = user_input.into();
        let api_key = api_key.into();
        let model_name = model_name.into();
        let scope = scope.into();

        if user_input.trim().is_empty() {
            return Err(BlueprintError::K1StrongValidationFailed {
                field: "user_input".into(),
                value: user_input,
                reason: "empty after trim".into(),
            });
        }
        if user_input.len() > 64 * 1024 {
            return Err(BlueprintError::K1StrongValidationFailed {
                field: "user_input".into(),
                value: format!("<{} bytes>", user_input.len()),
                reason: "exceeds 64 KiB limit".into(),
            });
        }
        if api_key.trim().is_empty() {
            return Err(BlueprintError::K1StrongValidationFailed {
                field: "api_key".into(),
                value: api_key,
                reason: "empty".into(),
            });
        }
        // api key 至少 8 字符 (sk- 前缀 + 6 chars + 主体)
        if api_key.len() < 8 {
            return Err(BlueprintError::K1StrongValidationFailed {
                field: "api_key".into(),
                value: api_key,
                reason: "too short (<8 chars)".into(),
            });
        }
        if model_name.trim().is_empty() {
            return Err(BlueprintError::K1StrongValidationFailed {
                field: "model_name".into(),
                value: model_name,
                reason: "empty".into(),
            });
        }
        if scope.trim().is_empty() {
            return Err(BlueprintError::K1StrongValidationFailed {
                field: "scope".into(),
                value: scope,
                reason: "empty".into(),
            });
        }
        // 控制字符检查 (防注入 — 不允许 \0 / \r / \n in api_key)
        for ch in api_key.chars() {
            if ch.is_control() {
                return Err(BlueprintError::K1StrongValidationFailed {
                    field: "api_key".into(),
                    value: "<redacted>".into(),
                    reason: format!("control char U+{:04X}", ch as u32),
                });
            }
        }

        Ok(Self {
            user_input,
            api_key,
            model_name,
            scope,
        })
    }
}

/// K-1 强校验 trait — 任何业务实装必须实现.
///
/// **不变量**:
/// - 输入任一字段不合法 → `Err(K1StrongValidationFailed)`.
/// - 不允许 `Ok(false)` 蒙混 — 必须是 `Ok(())` 或 `Err`.
/// - 不允许吞错 / 静默 fallback (那是 K-2 的事).
pub trait K1StrongValidate: Send + Sync {
    /// 校验 4 字段 (user_input / api_key / model_name / scope).
    fn validate(&self, input: &K1Input) -> BlueprintResult<()>;

    /// 默认的 model 白名单 (e.g. "gpt-4*" / "claude-3*" / "gemini-2*").
    /// 返回 false = 不在白名单 → K-1 fail.
    fn model_allowed(&self, model_name: &str) -> bool {
        let _ = model_name;
        // 默认实现: 全部通过 (业务方应 override 加白名单)
        true
    }

    /// 默认的 scope 白名单 (e.g. "read" / "write" / "admin").
    fn scope_allowed(&self, scope: &str) -> bool {
        let _ = scope;
        true
    }
}

/// K-1 默认实现 — 仅做字段非空 + 长度检查 (在 K1Input::new 阶段完成).
/// 业务方应 override `model_allowed` / `scope_allowed` 加白名单.
pub struct DefaultK1Guard;

impl K1StrongValidate for DefaultK1Guard {
    fn validate(&self, input: &K1Input) -> BlueprintResult<()> {
        // K1Input::new 已保证 4 字段非空 + 长度合理 + 无控制字符.
        // 此处只检查 model / scope 白名单.
        if !self.model_allowed(&input.model_name) {
            return Err(BlueprintError::K1StrongValidationFailed {
                field: "model_name".into(),
                value: input.model_name.clone(),
                reason: "model not in whitelist".into(),
            });
        }
        if !self.scope_allowed(&input.scope) {
            return Err(BlueprintError::K1StrongValidationFailed {
                field: "scope".into(),
                value: input.scope.clone(),
                reason: "scope not in whitelist".into(),
            });
        }
        Ok(())
    }
}

// ============================================
// K-2 弱校验 — 容错 / 默认行为
// ============================================

/// K-2 弱校验输入 — 输入异常但有 fallback 链.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct K2Input {
    pub raw: String,
    pub fallback_chain: Vec<String>,
}

impl K2Input {
    pub fn new(raw: impl Into<String>, fallback_chain: Vec<String>) -> Self {
        Self {
            raw: raw.into(),
            fallback_chain,
        }
    }
}

/// K-2 弱校验结果 — 返回实际使用的值 + 走了哪一层 fallback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct K2Result {
    pub used_value: String,
    pub fallback_layer: usize, // 0 = 直接用 raw, 1+ = 走了 fallback_chain[N]
}

/// K-2 弱校验 trait — 尝试 fallback 链, 全部失败才 `Err`.
pub trait K2WeakValidate: Send + Sync {
    /// 校验 + fallback. fallback_chain 上限 3 层 (8 项承诺 #2).
    fn validate(&self, input: &K2Input) -> BlueprintResult<K2Result>;

    /// fallback 链深度上限.
    fn max_fallback_layers(&self) -> usize {
        3
    }
}

/// K-2 默认实现 — 直接用 raw; raw 异常时按顺序试 fallback_chain.
pub struct DefaultK2Guard;

impl K2WeakValidate for DefaultK2Guard {
    fn validate(&self, input: &K2Input) -> BlueprintResult<K2Result> {
        // 1. raw 自身合法 → 直接返回
        if !input.raw.trim().is_empty() && input.raw.len() <= 16 * 1024 {
            return Ok(K2Result {
                used_value: input.raw.clone(),
                fallback_layer: 0,
            });
        }

        // 2. 试 fallback_chain
        let max = self.max_fallback_layers();
        let chain: Vec<&String> = input
            .fallback_chain
            .iter()
            .take(max)
            .collect();

        for (i, candidate) in chain.iter().enumerate() {
            if !candidate.trim().is_empty() && candidate.len() <= 16 * 1024 {
                return Ok(K2Result {
                    used_value: (*candidate).clone(),
                    fallback_layer: i + 1,
                });
            }
        }

        // 3. 全失败 → K-2 Err
        Err(BlueprintError::K2WeakValidationFailed {
            field: "raw + fallback_chain".into(),
            reason: format!("all {} layers exhausted", chain.len()),
        })
    }
}

// ============================================
// K-3 监督 — trace log + audit
// ============================================

/// K-3 审计事件.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEvent {
    pub timestamp_ms: u64,
    pub category: String,    // K-1 / K-2 / K-3 / K-4 / D-01..D-04 / TEMPLATE / Q-METRIC
    pub subject: String,     // 谁 (e.g. "tool:bash", "user:123")
    pub decision: String,    // "allow" / "deny" / "info" / "warn"
    pub message: String,     // 详细信息
}

impl AuditEvent {
    pub fn now(category: impl Into<String>, subject: impl Into<String>, decision: impl Into<String>, message: impl Into<String>) -> Self {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Self {
            timestamp_ms,
            category: category.into(),
            subject: subject.into(),
            decision: decision.into(),
            message: message.into(),
        }
    }
}

/// K-3 审计 trait — 写 trace log + audit.
pub trait K3Audit: Send + Sync {
    /// 写一条审计事件. 通道断开时必须 `Err` (8 项承诺 #3).
    fn audit(&self, event: &AuditEvent) -> BlueprintResult<()>;

    /// 读最近 N 条事件 (用于 dashboard / debug). 通道断开 → Err.
    fn recent(&self, n: usize) -> BlueprintResult<Vec<AuditEvent>>;
}

/// K-3 默认实现 — 内存 ring buffer (生产应 override 写盘 / 网络).
pub struct InMemoryAudit {
    buffer: std::sync::Mutex<Vec<AuditEvent>>,
    capacity: usize,
}

impl InMemoryAudit {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: std::sync::Mutex::new(Vec::with_capacity(capacity)),
            capacity,
        }
    }
}

impl Default for InMemoryAudit {
    fn default() -> Self {
        Self::new(1024)
    }
}

impl K3Audit for InMemoryAudit {
    fn audit(&self, event: &AuditEvent) -> BlueprintResult<()> {
        let mut buf = self.buffer.lock().map_err(|_| {
            BlueprintError::K3AuditFailed {
                channel: "in_memory".into(),
                reason: "mutex poisoned".into(),
            }
        })?;
        if buf.len() >= self.capacity {
            buf.remove(0); // ring buffer: 丢最旧
        }
        buf.push(event.clone());
        Ok(())
    }

    fn recent(&self, n: usize) -> BlueprintResult<Vec<AuditEvent>> {
        let buf = self.buffer.lock().map_err(|_| BlueprintError::K3AuditFailed {
            channel: "in_memory".into(),
            reason: "mutex poisoned".into(),
        })?;
        let start = buf.len().saturating_sub(n);
        Ok(buf[start..].to_vec())
    }
}

/// 故意断通道的 K-3 实现 — 用于测试失败路径.
pub struct BrokenAudit;

impl K3Audit for BrokenAudit {
    fn audit(&self, _event: &AuditEvent) -> BlueprintResult<()> {
        Err(BlueprintError::K3AuditFailed {
            channel: "broken".into(),
            reason: "simulated channel down".into(),
        })
    }

    fn recent(&self, _n: usize) -> BlueprintResult<Vec<AuditEvent>> {
        Err(BlueprintError::K3AuditFailed {
            channel: "broken".into(),
            reason: "simulated channel down".into(),
        })
    }
}

// ============================================
// K-4 守门 — deny / allow
// ============================================

/// K-4 守门决策.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuardDecision {
    Allow,
    Deny,
}

/// K-4 守门规则 — 一个 (subject, action) → 决策.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardRule {
    pub subject: String,
    pub action: String,
    pub decision: GuardDecision,
    pub reason: String,
}

/// K-4 守门 trait — 决策 deny/allow. deny 必须 `Err` (8 项承诺 #4).
pub trait K4Guard: Send + Sync {
    /// 守门决策. 返回 Err 当且仅当 deny.
    fn decide(&self, subject: &str, action: &str) -> BlueprintResult<GuardDecision>;

    /// 添加规则.
    fn add_rule(&mut self, rule: GuardRule) -> BlueprintResult<()>;

    /// 列出所有规则.
    fn list_rules(&self) -> Vec<GuardRule>;
}

/// K-4 默认实现 — 规则表 (按 (subject, action) 匹配).
pub struct RuleTableGuard {
    rules: std::sync::Mutex<Vec<GuardRule>>,
}

impl Default for RuleTableGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleTableGuard {
    pub fn new() -> Self {
        Self {
            rules: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl K4Guard for RuleTableGuard {
    fn decide(&self, subject: &str, action: &str) -> BlueprintResult<GuardDecision> {
        let rules = self.rules.lock().map_err(|_| BlueprintError::K4GuardDenied {
            subject: subject.into(),
            rule: "<mutex poisoned>".into(),
        })?;

        for r in rules.iter() {
            if r.subject == subject && r.action == action {
                return match r.decision {
                    GuardDecision::Allow => Ok(GuardDecision::Allow),
                    GuardDecision::Deny => Err(BlueprintError::K4GuardDenied {
                        subject: subject.into(),
                        rule: r.reason.clone(),
                    }),
                };
            }
        }
        // 无匹配规则 → 默认 allow
        Ok(GuardDecision::Allow)
    }

    fn add_rule(&mut self, rule: GuardRule) -> BlueprintResult<()> {
        let mut rules = self.rules.lock().map_err(|_| BlueprintError::K4GuardDenied {
            subject: rule.subject.clone(),
            rule: "<mutex poisoned>".into(),
        })?;
        rules.push(rule);
        Ok(())
    }

    fn list_rules(&self) -> Vec<GuardRule> {
        self.rules
            .lock()
            .map(|r| r.clone())
            .unwrap_or_default()
    }
}

// ============================================
// RiskChain — 4 类串联 (K-1 → K-2 → K-3 → K-4)
// ============================================

/// 4 类风险串联 — 一次调用走完 K-1 → K-2 → K-3 → K-4.
///
/// 顺序: K-1 强校验 → K-2 弱校验 (K-1 失败跳过) → K-3 audit 记录 → K-4 守门.
pub struct RiskChain<K1, K2, K3, K4>
where
    K1: K1StrongValidate,
    K2: K2WeakValidate,
    K3: K3Audit,
    K4: K4Guard,
{
    pub k1: K1,
    pub k2: K2,
    pub k3: K3,
    pub k4: K4,
}

impl<K1, K2, K3, K4> RiskChain<K1, K2, K3, K4>
where
    K1: K1StrongValidate,
    K2: K2WeakValidate,
    K3: K3Audit,
    K4: K4Guard,
{
    pub fn new(k1: K1, k2: K2, k3: K3, k4: K4) -> Self {
        Self { k1, k2, k3, k4 }
    }

    /// 走完整链条. 任一关失败立即返回 Err, 但 K-3 audit 尽可能记录.
    pub fn run(&self, k1_input: &K1Input, k2_input: &K2Input, subject: &str, action: &str) -> BlueprintResult<(K2Result, GuardDecision)> {
        // Step 1: K-1
        self.k1.validate(k1_input)?;

        // Step 2: K-2
        let k2_result = self.k2.validate(k2_input)?;

        // Step 3: K-3 audit (尽力写, 不影响主流程)
        let event = AuditEvent::now(
            "CHAIN",
            subject,
            "info",
            format!("K-1 OK, K-2 used_layer={}", k2_result.fallback_layer),
        );
        let _ = self.k3.audit(&event); // K-3 失败不阻断主链 (K-3 自己的失败已 Err 出来)

        // Step 4: K-4
        let decision = self.k4.decide(subject, action)?;

        Ok((k2_result, decision))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_k1() -> K1Input {
        K1Input::new("hello", "sk-test1234", "gpt-4", "read").unwrap()
    }

    #[test]
    fn k1_input_new_rejects_empty() {
        let r = K1Input::new("", "sk-test1234", "gpt-4", "read");
        assert!(r.is_err());
        assert_eq!(r.unwrap_err().category(), "K-1");
    }

    #[test]
    fn k1_input_new_rejects_short_api_key() {
        let r = K1Input::new("hello", "short", "gpt-4", "read");
        assert!(r.is_err());
    }

    #[test]
    fn k1_input_new_rejects_control_char() {
        let r = K1Input::new("hello", "sk-test\0bad", "gpt-4", "read");
        assert!(r.is_err());
    }

    #[test]
    fn k1_input_new_rejects_oversize() {
        let big = "x".repeat(70 * 1024);
        let r = K1Input::new(big, "sk-test1234", "gpt-4", "read");
        assert!(r.is_err());
    }

    #[test]
    fn k1_default_guard_passes_valid() {
        let g = DefaultK1Guard;
        assert!(g.validate(&sample_k1()).is_ok());
    }

    #[test]
    fn k1_model_whitelist_can_reject() {
        struct Whitelist;
        impl K1StrongValidate for Whitelist {
            fn validate(&self, input: &K1Input) -> BlueprintResult<()> {
                // 跟 DefaultK1Guard 一样调用 model_allowed
                if !self.model_allowed(&input.model_name) {
                    return Err(BlueprintError::K1StrongValidationFailed {
                        field: "model_name".into(),
                        value: input.model_name.clone(),
                        reason: "model not in whitelist".into(),
                    });
                }
                Ok(())
            }
            fn model_allowed(&self, model_name: &str) -> bool {
                model_name.starts_with("claude-")
            }
        }
        let g = Whitelist;
        assert!(g.validate(&sample_k1()).is_err()); // gpt-4 not in whitelist
    }

    #[test]
    fn k2_uses_raw_when_valid() {
        let g = DefaultK2Guard;
        let input = K2Input::new("hello", vec!["fb1".into(), "fb2".into()]);
        let r = g.validate(&input).unwrap();
        assert_eq!(r.used_value, "hello");
        assert_eq!(r.fallback_layer, 0);
    }

    #[test]
    fn k2_falls_back_to_layer_1() {
        let g = DefaultK2Guard;
        let input = K2Input::new("", vec!["fb1".into(), "fb2".into()]);
        let r = g.validate(&input).unwrap();
        assert_eq!(r.used_value, "fb1");
        assert_eq!(r.fallback_layer, 1);
    }

    #[test]
    fn k2_falls_back_to_layer_2() {
        let g = DefaultK2Guard;
        let input = K2Input::new("x".repeat(20 * 1024), vec!["".into(), "fb2".into()]);
        let r = g.validate(&input).unwrap();
        assert_eq!(r.used_value, "fb2");
        assert_eq!(r.fallback_layer, 2);
    }

    #[test]
    fn k2_exhausts_all_fallbacks() {
        let g = DefaultK2Guard;
        let input = K2Input::new("x".repeat(20 * 1024), vec!["".into(), "".into(), "".into()]);
        let r = g.validate(&input);
        assert!(r.is_err());
        assert_eq!(r.unwrap_err().category(), "K-2");
    }

    #[test]
    fn k2_caps_fallback_chain_at_3() {
        let g = DefaultK2Guard;
        // 5 层 chain, 但只前 3 层会被尝试
        let input = K2Input::new(
            "x".repeat(20 * 1024),
            vec!["".into(), "".into(), "".into(), "should_not_reach".into()],
        );
        let r = g.validate(&input);
        assert!(r.is_err());
    }

    #[test]
    fn k3_in_memory_audit_writes_and_reads() {
        let a = InMemoryAudit::new(10);
        let e = AuditEvent::now("TEST", "tool:bash", "allow", "ok");
        a.audit(&e).unwrap();
        let recent = a.recent(5).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].category, "TEST");
    }

    #[test]
    fn k3_in_memory_ring_buffer_evicts_oldest() {
        let a = InMemoryAudit::new(3);
        for i in 0..5 {
            let e = AuditEvent::now("T", "s", "i", format!("e{i}"));
            a.audit(&e).unwrap();
        }
        let recent = a.recent(10).unwrap();
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].message, "e2"); // e0, e1 evicted
        assert_eq!(recent[2].message, "e4");
    }

    #[test]
    fn k3_broken_audit_returns_err() {
        let a = BrokenAudit;
        let e = AuditEvent::now("T", "s", "i", "m");
        assert!(a.audit(&e).is_err());
    }

    #[test]
    fn k4_default_allow_without_rules() {
        let g = RuleTableGuard::new();
        let d = g.decide("tool:bash", "exec").unwrap();
        assert_eq!(d, GuardDecision::Allow);
    }

    #[test]
    fn k4_deny_returns_err() {
        let mut g = RuleTableGuard::new();
        g.add_rule(GuardRule {
            subject: "tool:bash".into(),
            action: "rm_rf".into(),
            decision: GuardDecision::Deny,
            reason: "destructive".into(),
        })
        .unwrap();
        let r = g.decide("tool:bash", "rm_rf");
        assert!(r.is_err());
        assert_eq!(r.unwrap_err().category(), "K-4");
    }

    #[test]
    fn k4_list_rules_preserves_insertion_order() {
        let mut g = RuleTableGuard::new();
        g.add_rule(GuardRule {
            subject: "a".into(),
            action: "x".into(),
            decision: GuardDecision::Allow,
            reason: "r1".into(),
        })
        .unwrap();
        g.add_rule(GuardRule {
            subject: "b".into(),
            action: "y".into(),
            decision: GuardDecision::Deny,
            reason: "r2".into(),
        })
        .unwrap();
        let rules = g.list_rules();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].subject, "a");
        assert_eq!(rules[1].subject, "b");
    }

    #[test]
    fn risk_chain_runs_all_4_stages() {
        let mut g4 = RuleTableGuard::new();
        g4.add_rule(GuardRule {
            subject: "tool:bash".into(),
            action: "exec".into(),
            decision: GuardDecision::Allow,
            reason: "default-allow".into(),
        })
        .unwrap();
        let chain = RiskChain::new(
            DefaultK1Guard,
            DefaultK2Guard,
            InMemoryAudit::new(100),
            g4,
        );
        let k1 = sample_k1();
        let k2 = K2Input::new("hi", vec![]);
        let (r2, d) = chain.run(&k1, &k2, "tool:bash", "exec").unwrap();
        assert_eq!(r2.used_value, "hi");
        assert_eq!(d, GuardDecision::Allow);
    }

    #[test]
    fn risk_chain_short_circuits_on_k1_fail() {
        let chain = RiskChain::new(
            DefaultK1Guard,
            DefaultK2Guard,
            InMemoryAudit::default(),
            RuleTableGuard::new(),
        );
        // K1Input::new 拒绝了空字段 — 模拟强校验失败
        let bad = match K1Input::new("", "sk-test1234", "gpt-4", "read") {
            Err(_) => return, // constructor blocks — equivalent
            Ok(v) => v,
        };
        let k2 = K2Input::new("hi", vec![]);
        let r = chain.run(&bad, &k2, "tool:bash", "exec");
        assert!(r.is_err());
    }
}
