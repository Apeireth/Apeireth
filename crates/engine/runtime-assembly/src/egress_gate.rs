//! 出网闸的**生产实现**（assembly 层）——把已有的出境脱敏闸接上电。
//!
//! 上位：`docs/04-internal/backend-enhancement/egress-gate-wiring-design-2026-10-23.md`
//! 与 ADR-0014 §1.2。端口在 runtime（`apeireth_runtime::canonical::egress_gate`），
//! 实现在这里 —— 方向纪律：runtime 不依赖 tools/governance 的具体实现。
//!
//! # 语义
//!
//! - **Pass**：本次请求没有可脱敏对象 → 原样发出；
//! - **Redacted**：检出 PII → 以占位符替换后发出（占位对出境面不可逆，
//!   回程还原由 `RedactionBridge::restore_for_local` 在受控回程面负责）；
//! - **Deny**：闸自身出错（读不到策略、桥内部失败）→ **拒绝即帧**（fail-closed）。
//!
//! # 为什么默认关
//!
//! 脱敏会改写请求内容 —— 那是**行为变化**，不是「更安全的默认值」。
//! 因此闸由 env 显式开启（`APEIRETH_ENABLE_EGRESS_GATE`），关时逐字节零变化。

use std::sync::Mutex;

use apeireth_protocol::canonical::{ContentPart, NormalizedRequest, NormalizedResponse};
use apeireth_runtime::canonical::egress_gate::{EgressGate, EgressVerdict};
use apeireth_tools_canonical::redaction_bridge::RedactionBridge;

/// 出网闸开关 env（未设 / 非 `"1"` = 关，fail-closed 于「开启侧」）。
pub const EGRESS_GATE_ENV: &str = "APEIRETH_ENABLE_EGRESS_GATE";

/// 生产出网闸：用 `RedactionBridge` 把请求里的 PII 换成不可逆占位。
///
/// 线程安全：`RedactionBridge` 内部有本会话登记簿（真值驻留处），用互斥量串行化。
pub struct RedactionEgressGate {
    bridge: Mutex<RedactionBridge>,
}

impl RedactionEgressGate {
    /// 新建闸（新会话键 = 新登记簿）。
    pub fn new() -> Self {
        Self {
            bridge: Mutex::new(RedactionBridge::new_session()),
        }
    }

    /// 用指定会话键新建（同一会话的占位恒同、跨会话换别名）。
    pub fn with_session_key(session_key: [u8; 32]) -> Self {
        Self {
            bridge: Mutex::new(RedactionBridge::with_session_key(session_key)),
        }
    }

    /// 本次判定是否改写了内容。
    pub fn replaced_any(&self) -> bool {
        self.bridge
            .lock()
            .map(|bridge| bridge.registered_values() > 0)
            .unwrap_or(false)
    }
}

impl Default for RedactionEgressGate {
    fn default() -> Self {
        Self::new()
    }
}

impl EgressGate for RedactionEgressGate {
    /// 受控回程还原：把回信里的占位符换回本地真值。
    ///
    /// 双闸的闸二 —— 还原只发生在**受控回程面**（本地落盘前），
    /// 且只还原本会话登记簿登记过的指纹；未登记占位**不还原、照实留痕**。
    /// 对出境面不可逆：出去的请求里只有占位，没有真值。
    fn restore(&self, response: &mut NormalizedResponse) {
        let Ok(mut bridge) = self.bridge.lock() else {
            // 登记簿不可用 → 不还原（占位保留）。**不泄漏**：保持占位比还原错更安全。
            return;
        };
        let report = bridge.restore_for_local(&response.content);
        response.content = report.text;
    }

    fn vet(&self, request: &NormalizedRequest) -> EgressVerdict {
        let Ok(mut bridge) = self.bridge.lock() else {
            // 登记簿锁被 poison = 无法保证「真值不出境」→ 拒绝即帧。
            return EgressVerdict::Deny {
                reason: "出境脱敏闸不可用（登记簿锁被污染）—— 已拒绝本次出网".to_string(),
            };
        };
        let mut replaced = request.clone();
        let mut any_hit = false;
        for message in &mut replaced.messages {
            for part in &mut message.content {
                if let ContentPart::Text { text } = part {
                    let redaction = bridge.redact_for_egress(text);
                    if !redaction.hits.is_empty() {
                        any_hit = true;
                        *text = redaction.text;
                    }
                }
            }
        }
        if any_hit {
            EgressVerdict::Redacted(replaced)
        } else {
            EgressVerdict::Pass
        }
    }
}

/// 按 env 决定是否装配闸。
///
/// - 未设 / 非 `"1"` → `None`（runtime 侧默认放行，零行为变化）；
/// - `"1"` → 装配闸。
pub fn egress_gate_from_env() -> Option<std::sync::Arc<dyn EgressGate>> {
    let enabled = std::env::var(EGRESS_GATE_ENV).ok().as_deref() == Some("1");
    enabled
        .then(|| std::sync::Arc::new(RedactionEgressGate::new()) as std::sync::Arc<dyn EgressGate>)
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_protocol::canonical::{MessageRole, NormalizedMessage};

    fn request_with(text: &str) -> NormalizedRequest {
        NormalizedRequest::new(
            "test-model",
            vec![NormalizedMessage {
                role: MessageRole::User,
                content: vec![ContentPart::Text {
                    text: text.to_string(),
                }],
                tool_calls: Vec::new(),
                tool_call_id: None,
                name: None,
            }],
        )
    }

    #[test]
    fn plain_text_passes_through_unchanged() {
        let gate = RedactionEgressGate::new();
        let request = request_with("今天天气不错");
        match gate.vet(&request) {
            EgressVerdict::Pass => {}
            other => panic!("应原样放行: {other:?}"),
        }
    }

    #[test]
    fn pii_is_replaced_by_placeholder() {
        let gate = RedactionEgressGate::new();
        // 一个能被 PII 检出的样本：邮箱。
        let request = request_with("联系 alice@example.com 就行");
        match gate.vet(&request) {
            EgressVerdict::Redacted(redacted) => {
                let text = match &redacted.messages[0].content[0] {
                    ContentPart::Text { text } => text.clone(),
                    other => panic!("应为文本: {other:?}"),
                };
                assert!(!text.contains("alice@example.com"), "真值不得出境: {text}");
                // 占位符形态由脱敏闸自有格式决定（此处为 `__AP_<类别>_<指纹>__`），
                // 断言「有占位」而不是硬编格式（格式会演进，硬编会假红）。
                assert!(text.contains("__AP_"), "应有占位: {text}");
            }
            other => panic!("应脱敏: {other:?}"),
        }
    }

    /// 回程还原：占位出境 → 回信带占位 → 受控回程换回真值。
    ///
    /// 这是「双闸」的闸二：出境面只有占位（不可逆），回程面才还原。
    #[test]
    fn egress_restore_returns_placeholder_to_real_value_locally() {
        use apeireth_protocol::canonical::NormalizedResponse;

        let gate = RedactionEgressGate::new();
        let request = request_with("联系 alice@example.com 就行");
        let redacted = match gate.vet(&request) {
            EgressVerdict::Redacted(redacted) => redacted,
            other => panic!("应脱敏: {other:?}"),
        };
        // 出去的那份只含占位。
        let outgoing = match &redacted.messages[0].content[0] {
            ContentPart::Text { text } => text.clone(),
            other => panic!("应为文本: {other:?}"),
        };
        assert!(!outgoing.contains("alice@example.com"));

        // 模型回信里带回了占位（它只能看到占位）。
        let mut response = NormalizedResponse {
            id: "resp-1".into(),
            model: "test-model".into(),
            content: format!("我看到的是 {outgoing}"),
            finish_reason: None,
            usage: Default::default(),
            tool_calls: Vec::new(),
            raw_metadata: Default::default(),
        };
        gate.restore(&mut response);
        // 回程面换回真值。
        assert!(
            response.content.contains("alice@example.com"),
            "回程应还原: {}",
            response.content
        );
    }

    /// 未知占位不还原（照实留痕）—— 还原错比不还原更危险。
    #[test]
    fn egress_restore_leaves_unknown_placeholders_alone() {
        use apeireth_protocol::canonical::NormalizedResponse;

        let gate = RedactionEgressGate::new();
        let mut response = NormalizedResponse {
            id: "resp-1".into(),
            model: "test-model".into(),
            content: "含 __AP_EMAIL_deadbeef0000__ 这个未登记占位".into(),
            finish_reason: None,
            usage: Default::default(),
            tool_calls: Vec::new(),
            raw_metadata: Default::default(),
        };
        gate.restore(&mut response);
        assert!(
            response.content.contains("__AP_EMAIL_deadbeef0000__"),
            "未登记占位应原样留痕: {}",
            response.content
        );
    }

    #[test]
    fn env_switch_is_off_by_default() {
        // 不设置 env → 不装配闸（runtime 侧默认放行）。
        // 注意：测试进程里可能已被别处设置，故只断言「非 1 时不装配」。
        let value = std::env::var(EGRESS_GATE_ENV).ok();
        if value.as_deref() != Some("1") {
            assert!(egress_gate_from_env().is_none());
        }
    }
}
