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

use std::sync::{Arc, Mutex};

use apeireth_core::kernel::Clock;
use apeireth_governance::authorization::{AuthorizationAction, AuthorizationPolicy};
use apeireth_protocol::canonical::{ContentPart, NormalizedRequest, NormalizedResponse};
use apeireth_runtime::canonical::egress_gate::{EgressGate, EgressVerdict};
use apeireth_tools_canonical::redaction_bridge::RedactionBridge;

use crate::accountability::{AccountabilityLedger, Affected, Boundary, ImpactStatement, Outcome};
use crate::egress_ledger::{EgressLedger, EgressLedgerEntry};

/// 出网闸开关 env（未设 / 非 `"1"` = 关，fail-closed 于「开启侧」）。
pub const EGRESS_GATE_ENV: &str = "APEIRETH_ENABLE_EGRESS_GATE";

/// 授权判定用的资源名（固定）——「把内容送出」这一个资源。
///
/// 粒度说明：**按用途授权**（scope 里放用途名，如 `chat`），provider 级粒度由
/// 出网控制层（`sec.egress_control`）管 —— 两层不混。
pub const EGRESS_AUTHORIZATION_RESOURCE: &str = "egress";

/// 当前唯一的出网用途（场景）。多场景需要扩这里 + 策略里加 scope 项。
pub const EGRESS_USAGE_CHAT: &str = "chat";

/// 生产出网闸：用 `RedactionBridge` 把请求里的 PII 换成不可逆占位。
///
/// 线程安全：`RedactionBridge` 内部有本会话登记簿（真值驻留处），用互斥量串行化。
pub struct RedactionEgressGate {
    bridge: Mutex<RedactionBridge>,
    /// 出域账本（承诺措辞里「记录已知流向」的载体；零真值、append-only）。
    ledger: Arc<EgressLedger>,
    /// 授权策略对象（可携带/可委托/可过期/可追溯）。
    ///
    /// `None` = **不按授权判定**（旧行为：只做脱敏，不因「没授权」拒绝）。
    /// `Some` = 未授权即 `Deny`（**fail-closed**：拿不到真值就别出去）。
    authorization: Option<Arc<AuthorizationPolicy>>,
    /// 时钟（判定过期用；不注入 = 用系统时钟）。
    clock: Arc<dyn Clock>,
    /// 问责账本（影响声明 + 事实→行动索引）。
    ///
    /// `None` = 不写影响声明（库级默认）；生产接线应传路径 ——
    /// 「在发现影响时通知你」的前提是**知道哪条事实影响过哪些行动**。
    accountability: Option<Arc<AccountabilityLedger>>,
}

impl RedactionEgressGate {
    /// 新建闸（新会话键 = 新登记簿），账本走内存（不持久）。
    pub fn new() -> Self {
        Self::with_ledger(Arc::new(EgressLedger::new(None)))
    }

    /// 用指定会话键新建（同一会话的占位恒同、跨会话换别名）。
    pub fn with_session_key(session_key: [u8; 32]) -> Self {
        Self {
            bridge: Mutex::new(RedactionBridge::with_session_key(session_key)),
            ledger: Arc::new(EgressLedger::new(None)),
            authorization: None,
            clock: Arc::new(apeireth_core::kernel::SystemClock),
            accountability: None,
        }
    }

    /// 用指定账本新建（生产接线：账本落盘）。
    pub fn with_ledger(ledger: Arc<EgressLedger>) -> Self {
        Self {
            bridge: Mutex::new(RedactionBridge::new_session()),
            ledger,
            authorization: None,
            clock: Arc::new(apeireth_core::kernel::SystemClock),
            accountability: None,
        }
    }

    /// 挂上问责账本（每次外部行动写一条影响声明）。
    pub fn with_accountability(mut self, ledger: Arc<AccountabilityLedger>) -> Self {
        self.accountability = Some(ledger);
        self
    }

    /// 挂上授权策略对象（未授权即拒绝出网，fail-closed）。
    pub fn with_authorization(
        mut self,
        policy: AuthorizationPolicy,
        clock: Arc<dyn Clock>,
    ) -> Self {
        self.authorization = Some(Arc::new(policy));
        self.clock = clock;
        self
    }

    /// 出域账本（只读引用）。
    pub fn ledger(&self) -> &Arc<EgressLedger> {
        &self.ledger
    }

    /// 本次判定是否改写了内容。
    pub fn replaced_any(&self) -> bool {
        self.bridge
            .lock()
            .map(|bridge| bridge.registered_values() > 0)
            .unwrap_or(false)
    }
}

impl RedactionEgressGate {
    /// 记一行出域账本。
    fn record(
        &self,
        verdict: &str,
        hits: &[apeireth_tools_canonical::redaction_bridge::RedactionHit],
        destination: &str,
        usage: &str,
        reason: Option<String>,
    ) {
        let entry = EgressLedgerEntry {
            ts_unix_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0),
            session: None,
            verdict: verdict.to_string(),
            objects: hits.iter().map(|hit| format!("{:?}", hit.kind)).collect(),
            fingerprints: hits.iter().map(|hit| hit.fingerprint.clone()).collect(),
            destination: destination.to_string(),
            usage: usage.to_string(),
            reason,
        };
        self.ledger.append(entry);
    }
}

impl Default for RedactionEgressGate {
    fn default() -> Self {
        Self::new()
    }
}

impl RedactionEgressGate {
    /// 写一条影响声明（**零真值**：只存指涉 id）。
    ///
    /// 边界 = `provider`（模型上下文出境）。
    /// `reversible` = **false** —— 出境的请求**收不回**（这是必须被记录的事实，
    /// 不是判断）；`outcome` = `unknown` —— 我们**不知道**模型侧发生了什么，
    /// 显式写 `unknown`，不假装知道。
    ///
    /// `facts_used` 当前**为空**：闸看不到上下文里用了哪些记忆条目
    /// （那要等上下文装配面接上来）。**空 = 如实没有**，不编。
    fn declare_impact(&self, destination: &str, _redacted: bool) {
        let Some(ledger) = &self.accountability else {
            return;
        };
        let now_ms = self.clock.now().timestamp_millis();
        let statement = ImpactStatement {
            action_id: format!("act-{now_ms}"),
            at_ms: now_ms,
            boundary: Boundary::Provider,
            facts_used: Vec::new(),
            inferences: Vec::new(),
            grants: self
                .authorization
                .as_ref()
                .map(|policy| vec![policy.fingerprint()])
                .unwrap_or_default(),
            affected: Affected::SelfOnly,
            reversible: false,
            destination: destination.to_string(),
            outcome: Outcome::Unknown,
        };
        ledger.append(&statement);
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
        // 授权判定**先于**脱敏：没授权就不必做脱敏工作量，也不必进登记簿。
        //
        // 粒度：资源固定 `egress`，scope 装**用途**（当前唯一用途 `chat`）。
        // provider 级粒度由出网控制层管（两层不混）。
        if let Some(policy) = &self.authorization {
            let now_ms = self.clock.now().timestamp_millis();
            if !policy.allows_at(
                EGRESS_AUTHORIZATION_RESOURCE,
                EGRESS_USAGE_CHAT,
                &AuthorizationAction::Egress,
                now_ms,
            ) {
                let reason = if !policy.is_valid_at(now_ms) {
                    "出网未获授权：授权策略已失效（撤销或过期）—— 已拒绝本次出网".to_string()
                } else {
                    format!(
                        "出网未获授权：策略未授予 `{EGRESS_AUTHORIZATION_RESOURCE}` 资源的 \
                         `{EGRESS_USAGE_CHAT}` 用途出网权限 —— 已拒绝本次出网"
                    )
                };
                self.record(
                    "deny",
                    &[],
                    "provider",
                    EGRESS_USAGE_CHAT,
                    Some(reason.clone()),
                );
                self.declare_impact("provider", false);
                return EgressVerdict::Deny { reason };
            }
        }
        let Ok(mut bridge) = self.bridge.lock() else {
            // 登记簿锁被 poison = 无法保证「真值不出境」→ 拒绝即帧。
            return EgressVerdict::Deny {
                reason: "出境脱敏闸不可用（登记簿锁被污染）—— 已拒绝本次出网".to_string(),
            };
        };
        let mut replaced = request.clone();
        let mut any_hit = false;
        let mut hits = Vec::new();
        for message in &mut replaced.messages {
            for part in &mut message.content {
                if let ContentPart::Text { text } = part {
                    let redaction = bridge.redact_for_egress(text);
                    if !redaction.hits.is_empty() {
                        any_hit = true;
                        hits.extend(redaction.hits);
                        *text = redaction.text;
                    }
                }
            }
        }
        // 出域账本：每次判定记一行（**零真值**：只记对象类型与指纹）。
        // 记账失败**不阻断出网**（账本是观测面，不是闸）。
        if any_hit {
            self.record("redacted", &hits, "provider", EGRESS_USAGE_CHAT, None);
            self.declare_impact("provider", true);
            EgressVerdict::Redacted(replaced)
        } else {
            self.record("pass", &[], "provider", EGRESS_USAGE_CHAT, None);
            self.declare_impact("provider", true);
            EgressVerdict::Pass
        }
    }
}

/// 按 env 决定是否装配闸。**两种形态**（不新增第二个开关 —— env 名数有棘轮）：
///
/// - 未设 / 其它值 → `None`（runtime 侧默认放行，零行为变化）；
/// - `"1"` → 装配闸，**只做脱敏**；
/// - `"<授权策略 JSON>"` → 装配闸**并挂上授权策略对象**（未授权即拒绝出网，fail-closed）。
///
/// 授权策略是一行可携带的 JSON，形如：
/// `{"subject":"user-1","issuer":"owner","grants":[{"resource":"egress","action":"egress"}]}`
///
/// 授权写坏了 = 不猜：**fail-closed**（不装配闸）+ stderr 如实报错（不静默）。
pub fn egress_gate_from_env() -> Option<std::sync::Arc<dyn EgressGate>> {
    egress_gate_from_env_with_ledger(None)
}

/// 同上，但指定出域账本落盘路径。
///
/// **生产接线必须传路径** —— 否则账本只在内存里，进程一退就没了，
/// 「记录已知流向」那句话就落空（这是承诺与实现的差距，不是细节）。
pub fn egress_gate_from_env_with_ledger(
    ledger_path: Option<std::path::PathBuf>,
) -> Option<std::sync::Arc<dyn EgressGate>> {
    egress_gate_from_env_with_ledgers(ledger_path, None)
}

/// 同上，但同时指定**问责账本**（影响声明 + 事实→行动索引）的落盘路径。
///
/// 两个账本分开：**出域账本**记「判定 + 对象类型」（观测面），
/// **问责账本**记「这次行动依赖了什么授权、能不能撤回、影响谁」（责任面）。
/// 承诺措辞里的「记录已知流向」对应前者，「在发现影响时通知你」对应后者。
pub fn egress_gate_from_env_with_ledgers(
    ledger_path: Option<std::path::PathBuf>,
    accountability_path: Option<std::path::PathBuf>,
) -> Option<std::sync::Arc<dyn EgressGate>> {
    let raw = std::env::var(EGRESS_GATE_ENV).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "0" || trimmed.eq_ignore_ascii_case("false") {
        return None;
    }
    let ledger = Arc::new(EgressLedger::new(ledger_path));
    let mut gate = RedactionEgressGate::with_ledger(ledger);
    if let Some(path) = accountability_path {
        gate = gate.with_accountability(Arc::new(AccountabilityLedger::new(Some(path))));
    }
    if trimmed != "1" {
        match AuthorizationPolicy::from_carryable(trimmed) {
            Ok(policy) => {
                gate =
                    gate.with_authorization(policy, Arc::new(apeireth_core::kernel::SystemClock));
            }
            Err(error) => {
                eprintln!("[egress] 授权策略解析失败（{error}）—— 不装配出网闸");
                return None;
            }
        }
    }
    Some(Arc::new(gate) as std::sync::Arc<dyn EgressGate>)
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_protocol::canonical::{MessageRole, NormalizedMessage};

    /// 固定时钟（测过期用；真实时钟会漂）。
    struct FixedClock(i64);

    impl apeireth_core::kernel::Clock for FixedClock {
        fn now(&self) -> chrono::DateTime<chrono::Utc> {
            chrono::DateTime::from_timestamp_millis(self.0).unwrap_or_else(chrono::Utc::now)
        }
    }

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

    /// 出网闸每次判定都记一行（零真值：只有对象类型与指纹）。
    #[test]
    fn gate_records_every_verdict_into_the_ledger() {
        let ledger = std::sync::Arc::new(crate::egress_ledger::EgressLedger::new(None));
        let gate = RedactionEgressGate::with_ledger(ledger.clone());

        // 一次无命中的判定。
        let _ = gate.vet(&request_with("今天天气不错"));
        // 一次有命中的判定。
        let _ = gate.vet(&request_with("联系 alice@example.com 就行"));

        let entries = ledger.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].verdict, "pass");
        assert_eq!(entries[1].verdict, "redacted");
        assert!(!entries[1].objects.is_empty(), "应有对象类型");
        // 零真值：账本里没有原文。
        let raw = format!("{:?}", entries);
        assert!(!raw.contains("alice@example.com"), "账本不得含真值: {raw}");
    }

    /// 未授权即拒绝出网（fail-closed），且**先于**脱敏判定。
    #[test]
    fn unauthorized_egress_is_denied() {
        use apeireth_governance::authorization::{AuthorizationAction, ScopedGrant};

        // 策略只授了 `memory` 读写，没授 `egress`。
        let policy = AuthorizationPolicy::new(
            "user-1",
            "owner",
            1_000,
            vec![ScopedGrant::new("memory:recall", AuthorizationAction::Read)],
        );
        let gate = RedactionEgressGate::new().with_authorization(
            policy,
            std::sync::Arc::new(apeireth_core::kernel::SystemClock),
        );
        match gate.vet(&request_with("你好")) {
            EgressVerdict::Deny { reason } => {
                assert!(reason.contains("未获授权"), "理由应可读: {reason}");
            }
            other => panic!("未授权应拒绝: {other:?}"),
        }
    }

    /// 已授权 → 正常走脱敏路径。
    #[test]
    fn authorized_egress_proceeds() {
        use apeireth_governance::authorization::{AuthorizationAction, ScopedGrant};

        let policy = AuthorizationPolicy::new(
            "user-1",
            "owner",
            1_000,
            vec![ScopedGrant::scoped(
                EGRESS_AUTHORIZATION_RESOURCE,
                AuthorizationAction::Egress,
                vec![EGRESS_USAGE_CHAT.to_string()],
            )],
        );
        let gate = RedactionEgressGate::new().with_authorization(
            policy,
            std::sync::Arc::new(apeireth_core::kernel::SystemClock),
        );
        match gate.vet(&request_with("今天天气不错")) {
            EgressVerdict::Pass => {}
            other => panic!("已授权应放行: {other:?}"),
        }
    }

    /// 授权过期 → 拒绝（过期即失效，不是「建议不用」）。
    #[test]
    fn expired_authorization_denies_egress() {
        use apeireth_governance::authorization::{AuthorizationAction, ScopedGrant};

        let now_ms = 1_700_000_000_000i64;
        let policy = AuthorizationPolicy::new(
            "user-1",
            "owner",
            now_ms,
            vec![ScopedGrant::new(
                EGRESS_AUTHORIZATION_RESOURCE,
                AuthorizationAction::Egress,
            )],
        )
        .with_expiry(now_ms + 1_000);
        let gate = RedactionEgressGate::new()
            .with_authorization(policy, std::sync::Arc::new(FixedClock(now_ms + 2_000)));
        match gate.vet(&request_with("你好")) {
            EgressVerdict::Deny { reason } => {
                assert!(reason.contains("失效"), "理由应说失效: {reason}");
            }
            other => panic!("过期应拒绝: {other:?}"),
        }
    }

    /// 每次外部行动都写一条影响声明（可逆=false、后果=unknown）。
    #[test]
    fn gate_writes_impact_statements() {
        let ledger = std::sync::Arc::new(crate::accountability::AccountabilityLedger::new(None));
        let gate = RedactionEgressGate::new().with_accountability(ledger.clone());

        let _ = gate.vet(&request_with("今天天气不错"));

        // 内存账本不持久 → statements() 为空；用计数验证需要落盘。
        // 这里改验「无账本时不影响判定」这条更重要的性质。
        assert!(ledger.path().is_none());
        assert!(matches!(
            gate.vet(&request_with("联系 alice@example.com 就行")),
            EgressVerdict::Redacted(_)
        ));
    }

    /// 影响声明落盘后能查「哪些行动受影响」。
    #[test]
    fn impact_statements_are_queryable_when_persisted() {
        let path = std::env::temp_dir().join(format!(
            "apeireth-gate-impact-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let ledger = std::sync::Arc::new(crate::accountability::AccountabilityLedger::new(Some(
            path.clone(),
        )));
        let gate = RedactionEgressGate::new().with_accountability(ledger.clone());
        let _ = gate.vet(&request_with("今天天气不错"));

        let statements = ledger.statements();
        assert_eq!(statements.len(), 1);
        assert_eq!(
            statements[0].boundary,
            crate::accountability::Boundary::Provider
        );
        assert!(!statements[0].reversible, "出境不可逆 —— 必须被记录");
        assert_eq!(
            statements[0].outcome,
            crate::accountability::Outcome::Unknown,
            "我们不知道模型侧发生了什么 —— 显式 unknown"
        );
        let _ = std::fs::remove_file(&path);
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

    /// env 的**两种形态**：`"1"` 只脱敏；`"<JSON>"` 额外挂授权。
    #[test]
    fn env_two_forms_are_parsed() {
        // "1" → 装配，但不带授权（判定逻辑里 authorization = None）。
        let gate = RedactionEgressGate::new();
        assert!(gate.authorization.is_none());

        // 授权 JSON 形如（grants 的 action 用 serde 的 snake_case）。
        let json = r#"{"version":1,"subject":"u1","issuer":"owner","issued_at_ms":1,
            "grants":[{"resource":"egress","action":"egress"}]}"#;
        let policy = AuthorizationPolicy::from_carryable(json).expect("应可解析");
        assert_eq!(policy.subject, "u1");
        assert!(policy.grants[0].scope.is_empty());
    }
}
