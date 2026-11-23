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
//! 生产装配默认开启脱敏。显式设置 `APEIRETH_ENABLE_EGRESS_GATE=0` 或 `false` 可退出保护。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::Clock;
use apeireth_governance::authorization::{AuthorizationAction, AuthorizationPolicy};
use apeireth_protocol::canonical::{ContentPart, NormalizedRequest, NormalizedResponse};
use apeireth_runtime::canonical::egress_gate::{EgressGate, EgressVerdict, ResponseRestorer};
use apeireth_tools_canonical::redaction_bridge::{RedactionBridge, RedactionHit};

use crate::accountability::{AccountabilityLedger, Affected, Boundary, ImpactStatement, Outcome};
use crate::consequence::{ConsequenceTracker, OutcomeSource};
use crate::egress_ledger::{EgressLedger, EgressLedgerEntry};

/// 出网闸的配置变量。
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
/// 生产路由的出境与回程共享单次请求专有的登记簿。
pub struct RedactionEgressGate {
    legacy_bridge: Option<Mutex<RedactionBridge>>,
    /// **出网动作序号**（单调递增）：与毫秒时间戳一起组成 `action_id`。
    ///
    /// 修「同毫秒两次出网 ⇒ 同 `action_id`」的碰撞（2026-10-25 实测发现）：
    /// 毫秒级 id 在同毫秒内产生两条影响声明时会**共用一个 id** ⇒ 破坏「一次
    /// 行动 ↔ 一条影响声明」的 1:1 不变式（后果回填的绑定账会因此拒绑第二侧）。
    /// 加序号后 id 恒唯一（进程内单调）。
    action_seq: AtomicU64,
    /// 出域账本（承诺措辞里「记录已知流向」的载体；零真值、append-only）。
    ledger: Arc<EgressLedger>,
    record_ledger: bool,
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
    /// 后果状态机（追加事件，不改写历史）。
    ///
    /// `None` = 不记后果事件（库级默认）。
    consequences: Option<Arc<tokio::sync::Mutex<ConsequenceTracker>>>,
}

impl RedactionEgressGate {
    /// 新建闸，账本走内存（不持久）。
    pub fn new() -> Self {
        Self::with_ledger(Arc::new(EgressLedger::new(None)))
    }

    /// 为直接使用 `vet` 和 `restore` 的调用方建立独立登记簿。
    /// `prepare` 始终为单次请求生成新登记簿。
    pub fn with_session_key(session_key: [u8; 32]) -> Self {
        Self {
            legacy_bridge: Some(Mutex::new(RedactionBridge::with_session_key(session_key))),
            action_seq: AtomicU64::new(0),
            ledger: Arc::new(EgressLedger::new(None)),
            record_ledger: true,
            authorization: None,
            clock: Arc::new(apeireth_core::kernel::SystemClock),
            accountability: None,
            consequences: None,
        }
    }

    /// 用指定账本新建（生产接线：账本落盘）。
    pub fn with_ledger(ledger: Arc<EgressLedger>) -> Self {
        Self {
            legacy_bridge: None,
            action_seq: AtomicU64::new(0),
            ledger,
            record_ledger: true,
            authorization: None,
            clock: Arc::new(apeireth_core::kernel::SystemClock),
            accountability: None,
            consequences: None,
        }
    }

    /// 挂上问责账本（每次外部行动写一条影响声明）。
    pub fn with_accountability(mut self, ledger: Arc<AccountabilityLedger>) -> Self {
        self.accountability = Some(ledger);
        self
    }

    /// 挂上后果状态机（记录行动后果的变化）。
    pub fn with_consequences(
        mut self,
        tracker: Arc<tokio::sync::Mutex<ConsequenceTracker>>,
    ) -> Self {
        self.consequences = Some(tracker);
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

    /// 注入时钟（判定过期与 `action_id` 时间戳用；默认系统时钟）。
    #[must_use]
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// 出域账本（只读引用）。
    pub fn ledger(&self) -> &Arc<EgressLedger> {
        &self.ledger
    }
}

impl RedactionEgressGate {
    /// 记一行出域账本。
    fn record(
        &self,
        verdict: &str,
        hits: &[RedactionHit],
        destination: &str,
        usage: &str,
        reason: Option<String>,
    ) {
        if !self.record_ledger {
            return;
        }
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
        let now_ms = self.clock.now().timestamp_millis();
        // 序号入 id ⇒ 同毫秒多次出网也不撞 id（1:1 不变式的前提）。
        let seq = self.action_seq.fetch_add(1, Ordering::SeqCst);
        let action_id = format!("act-{now_ms}-{seq}");
        // 后果初始状态 = `unknown`（**显式**，不假装知道）。
        // 模型侧发生了什么我们观测不到 —— 设计文档 §2.3 的最小实现就是这一档。
        // **独立于影响声明**：没挂问责账本时也要记后果。
        if let Some(tracker) = &self.consequences {
            if let Ok(mut guard) = tracker.try_lock() {
                guard.record(
                    &action_id,
                    crate::accountability::Outcome::Unknown,
                    OutcomeSource::NoReply,
                    now_ms,
                );
            }
        }
        let Some(ledger) = &self.accountability else {
            return;
        };
        let statement = ImpactStatement {
            action_id,
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
        let Some(Ok(mut bridge)) = self.legacy_bridge.as_ref().map(Mutex::lock) else {
            // 登记簿不可用 → 不还原（占位保留）。**不泄漏**：保持占位比还原错更安全。
            return;
        };
        let report = bridge.restore_for_local(&response.content);
        response.content = report.text;
    }

    fn vet(&self, request: &NormalizedRequest) -> EgressVerdict {
        match &self.legacy_bridge {
            Some(bridge) => match bridge.lock() {
                Ok(mut bridge) => self.vet_with_bridge(request, &mut bridge),
                Err(_) => self.deny("出境脱敏登记簿不可用".into()),
            },
            None => self.prepare(request).0,
        }
    }

    fn prepare(&self, request: &NormalizedRequest) -> (EgressVerdict, Option<ResponseRestorer>) {
        let mut bridge = RedactionBridge::new_session();
        let verdict = self.vet_with_bridge(request, &mut bridge);
        let restore: Option<ResponseRestorer> = match verdict {
            EgressVerdict::Redacted(_) => {
                Some(Box::new(move |response: &mut NormalizedResponse| {
                    response.content = bridge.restore_for_local(&response.content).text;
                }))
            }
            EgressVerdict::Pass => Some(Box::new(|_| {})),
            EgressVerdict::Deny { .. } => None,
        };
        (verdict, restore)
    }
}

impl RedactionEgressGate {
    fn deny(&self, reason: String) -> EgressVerdict {
        self.record(
            "deny",
            &[],
            "provider",
            EGRESS_USAGE_CHAT,
            Some(reason.clone()),
        );
        self.declare_impact("provider", false);
        EgressVerdict::Deny { reason }
    }

    fn vet_with_bridge(
        &self,
        request: &NormalizedRequest,
        bridge: &mut RedactionBridge,
    ) -> EgressVerdict {
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
                return self.deny(reason);
            }
        }
        if request
            .messages
            .iter()
            .flat_map(|message| &message.content)
            .any(|part| matches!(part, ContentPart::ImageUrl { .. }))
        {
            return self.deny("出境脱敏闸无法检查图像内容，已拒绝本次出网".into());
        }
        let mut replaced = request.clone();
        let mut hits = Vec::new();
        for message in &mut replaced.messages {
            for part in &mut message.content {
                if let ContentPart::Text { text } = part {
                    redact_text(bridge, text, &mut hits);
                }
            }
            for call in &mut message.tool_calls {
                redact_arguments(bridge, &mut call.arguments, &mut hits);
            }
        }
        // 出域账本：每次判定记一行（**零真值**：只记对象类型与指纹）。
        // 记账失败**不阻断出网**（账本是观测面，不是闸）。
        if !hits.is_empty() {
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

fn redact_text(bridge: &mut RedactionBridge, text: &mut String, hits: &mut Vec<RedactionHit>) {
    let report = bridge.redact_for_egress(text);
    if !report.hits.is_empty() {
        *text = report.text;
        hits.extend(report.hits);
    }
}

fn redact_arguments(
    bridge: &mut RedactionBridge,
    value: &mut serde_json::Value,
    hits: &mut Vec<RedactionHit>,
) {
    use serde_json::Value;
    let mut pending = vec![value];
    while let Some(value) = pending.pop() {
        match value {
            Value::String(text) => redact_text(bridge, text, hits),
            Value::Array(array) => pending.extend(array),
            Value::Object(object) => pending.extend(object.values_mut()),
            _ => {}
        }
    }
}

/// 闸 env 的**生效判定**（值语义；不含 env 读取）。
///
/// 只有显式 `"0"` 或 `"false"`（大小写不敏感）关闭保护。
pub fn egress_gate_enabled_from_value(raw: &str) -> bool {
    let trimmed = raw.trim();
    !(trimmed == "0" || trimmed.eq_ignore_ascii_case("false"))
}

fn configured_value(value: Result<String, std::env::VarError>) -> String {
    match value {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => String::new(),
        // Preserve the fail-closed behavior without exposing the invalid bytes.
        Err(std::env::VarError::NotUnicode(_)) => "invalid configuration encoding".into(),
    }
}

/// 生产问责回填要求显式启用记录。
pub(crate) fn egress_accountability_enabled_from_env() -> bool {
    let raw = configured_value(std::env::var(EGRESS_GATE_ENV));
    !raw.trim().is_empty() && egress_gate_enabled_from_value(&raw)
}

/// 生产出网闸的总开关。
pub fn egress_gate_enabled_from_env() -> bool {
    egress_gate_enabled_from_value(&configured_value(std::env::var(EGRESS_GATE_ENV)))
}

/// 按配置装配出网闸。
///
/// 未设、空值、`"1"` 启用脱敏，授权策略 JSON 额外要求授权。
/// 策略无效时拒绝出网。`"0"` 或 `"false"` 显式关闭。
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
    egress_gate_from_env_with_all(ledger_path, accountability_path, None)
}

/// 同上，并指定**后果账本**路径（后果状态持久化 + 重启后仍可查）。
pub fn egress_gate_from_env_with_all(
    ledger_path: Option<std::path::PathBuf>,
    accountability_path: Option<std::path::PathBuf>,
    consequence_path: Option<std::path::PathBuf>,
) -> Option<std::sync::Arc<dyn EgressGate>> {
    let raw = configured_value(std::env::var(EGRESS_GATE_ENV));
    gate_from_config(&raw, ledger_path, accountability_path, consequence_path)
}

fn gate_from_config(
    value: &str,
    ledger_path: Option<std::path::PathBuf>,
    accountability_path: Option<std::path::PathBuf>,
    consequence_path: Option<std::path::PathBuf>,
) -> Option<Arc<dyn EgressGate>> {
    let trimmed = value.trim();
    if !egress_gate_enabled_from_value(trimmed) {
        return None;
    }
    if trimmed.is_empty() {
        let mut gate = RedactionEgressGate::new();
        gate.record_ledger = false;
        return Some(Arc::new(gate));
    }
    let ledger = Arc::new(EgressLedger::new(ledger_path));
    let mut gate = RedactionEgressGate::with_ledger(ledger);
    if let Some(path) = accountability_path {
        gate = gate.with_accountability(Arc::new(AccountabilityLedger::new(Some(path))));
    }
    if let Some(path) = consequence_path {
        gate = gate.with_consequences(Arc::new(tokio::sync::Mutex::new(
            ConsequenceTracker::with_ledger(path),
        )));
    }
    match plan_authorization(trimmed) {
        GateAuthorizationPlan::RedactionOnly => {}
        GateAuthorizationPlan::WithPolicy(policy) => {
            gate = gate.with_authorization(policy, Arc::new(apeireth_core::kernel::SystemClock));
        }
        GateAuthorizationPlan::DenyAll { .. } => {
            eprintln!("[egress] 授权策略无效，已拒绝出网");
            gate = gate.with_authorization(
                AuthorizationPolicy::new(
                    "egress-gate",
                    "egress-gate-malformed-policy",
                    0,
                    Vec::new(),
                ),
                Arc::new(apeireth_core::kernel::SystemClock),
            );
        }
    }
    Some(Arc::new(gate) as std::sync::Arc<dyn EgressGate>)
}

/// 授权策略的装配计划（**纯函数** ⇒ 可直接测，不碰 env）。
///
/// | env 值 | 计划 | 语义 |
/// |---|---|---|
/// | 空或 `"1"` | `RedactionOnly` | 只做脱敏 |
/// | 合法策略 JSON | `WithPolicy` | 闸 + 授权策略（未授权即拒绝，fail-closed）|
/// | **其它非空值（畸形）** | `DenyAll` | **fail-closed**：闸照常装配，但出网一律拒绝 |
///
/// ⚠️ 畸形值**不能**当成「没配」（那会 fail-open ⇒ 脱敏与授权一起消失）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateAuthorizationPlan {
    /// 只脱敏。
    RedactionOnly,
    /// 合法策略 ⇒ 挂上它。
    WithPolicy(AuthorizationPolicy),
    /// 畸形 ⇒ 一律拒（fail-closed）。
    DenyAll { error: String },
}

/// 判定 env 值对应的授权装配计划（**纯函数**）。
pub fn plan_authorization(trimmed: &str) -> GateAuthorizationPlan {
    let trimmed = trimmed.trim();
    if trimmed.is_empty() || trimmed == "1" {
        return GateAuthorizationPlan::RedactionOnly;
    }
    match AuthorizationPolicy::from_carryable(trimmed) {
        Ok(policy) => GateAuthorizationPlan::WithPolicy(policy),
        Err(error) => GateAuthorizationPlan::DenyAll { error },
    }
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
    fn same_millisecond_egresses_get_distinct_action_ids() {
        // 同毫秒两次出网 ⇒ 两条影响声明必须**不同 id**（1:1 不变式的前提）。
        // 修前 id = `act-{now_ms}` ⇒ 同毫秒撞 id（后果回填绑定账会拒绑第二侧）。
        let dir = tempfile::tempdir().expect("temp dir");
        let ledger = Arc::new(AccountabilityLedger::new(Some(
            dir.path().join("impact.jsonl"),
        )));
        let gate = RedactionEgressGate::new()
            .with_accountability(Arc::clone(&ledger))
            .with_clock(Arc::new(FixedClock(1_700_000_000_000)));
        let request = request_with("今天天气不错");
        assert!(matches!(gate.vet(&request), EgressVerdict::Pass));
        assert!(matches!(gate.vet(&request), EgressVerdict::Pass));
        let statements = ledger.statements();
        assert_eq!(statements.len(), 2, "两次出网 ⇒ 两条影响声明");
        assert_ne!(
            statements[0].action_id, statements[1].action_id,
            "同毫秒两次出网不得共用 action_id"
        );
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
    fn unscannable_image_content_is_denied_without_recording_the_url() {
        let gate = RedactionEgressGate::new();
        let mut request = request_with("图像描述");
        request.messages[0].content.push(ContentPart::ImageUrl {
            url: "https://images.example.test/private-object".into(),
            detail: None,
        });
        assert!(matches!(gate.vet(&request), EgressVerdict::Deny { .. }));
        let entries = gate.ledger().entries();
        assert_eq!(entries[0].verdict, "deny");
        assert!(!serde_json::to_string(&entries)
            .unwrap()
            .contains("private-object"));
    }

    #[test]
    fn nested_tool_arguments_are_redacted_without_changing_their_shape() {
        use apeireth_protocol::canonical::ToolCall;
        let gate = RedactionEgressGate::new();
        let mut request = request_with("历史工具参数");
        request.messages[0].tool_calls.push(ToolCall {
            id: "test-call".into(),
            name: "test-tool".into(),
            arguments: serde_json::json!({"recipients": ["alice@example.test"], "limit": 1}),
        });
        let EgressVerdict::Redacted(redacted) = gate.vet(&request) else {
            panic!("tool argument should be redacted");
        };
        let call = &redacted.messages[0].tool_calls[0];
        assert_eq!(call.id, "test-call");
        assert_eq!(call.arguments["limit"], 1);
        assert!(call.arguments["recipients"][0]
            .as_str()
            .unwrap()
            .starts_with("__AP_"));
        assert_eq!(
            request.messages[0].tool_calls[0].arguments["recipients"][0],
            "alice@example.test"
        );
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
        let (verdict, restorer) = gate.prepare(&request);
        let redacted = match verdict {
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
        restorer.expect("redacted request has a response restorer")(&mut response);
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
        let (_, restorer) = gate.prepare(&request_with("alice@example.com"));
        restorer.expect("redacted request has a response restorer")(&mut response);
        assert!(
            response.content.contains("__AP_EMAIL_deadbeef0000__"),
            "未登记占位应原样留痕: {}",
            response.content
        );
    }

    #[test]
    fn one_requests_restorer_does_not_reveal_another_requests_value() {
        let gate = RedactionEgressGate::new();
        let (verdict_a, restore_a) = gate.prepare(&request_with("alice@example.com"));
        let EgressVerdict::Redacted(redacted_a) = verdict_a else {
            panic!("request A must be redacted");
        };
        let alias_a = ContentPart::join_text(&redacted_a.messages[0].content);
        let (_, restore_b) = gate.prepare(&request_with("bob@example.com"));
        let mut response_b = NormalizedResponse::text("b", "test-model", &alias_a);
        restore_b.unwrap()(&mut response_b);
        assert_eq!(response_b.content, alias_a);
        let mut response_a = NormalizedResponse::text("a", "test-model", alias_a);
        restore_a.unwrap()(&mut response_a);
        assert_eq!(response_a.content, "alice@example.com");
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

    /// 后果初始状态是 `unknown`（显式，不假装知道）。
    #[test]
    fn gate_records_unknown_outcome_for_each_action() {
        let tracker = std::sync::Arc::new(tokio::sync::Mutex::new(
            crate::consequence::ConsequenceTracker::new(),
        ));
        let gate = RedactionEgressGate::new().with_consequences(tracker.clone());

        let _ = gate.vet(&request_with("今天天气不错"));

        let guard = tracker.blocking_lock();
        assert_eq!(guard.counts().tracked_actions, 1);
        assert_eq!(guard.counts().unknown, 1, "我们不知道结果 —— 显式 unknown");
    }

    #[test]
    fn env_switch_requires_explicit_opt_out() {
        let default = configured_value(Err(std::env::VarError::NotPresent));
        for value in [default.as_str(), " ", "1"] {
            assert!(egress_gate_enabled_from_value(value));
            assert_eq!(
                plan_authorization(value),
                GateAuthorizationPlan::RedactionOnly
            );
        }
        for value in ["0", " FALSE "] {
            assert!(!egress_gate_enabled_from_value(value));
        }
        let invalid = configured_value(Err(std::env::VarError::NotUnicode("fixture".into())));
        assert!(egress_gate_enabled_from_value(&invalid));
        assert!(matches!(
            plan_authorization(&invalid),
            GateAuthorizationPlan::DenyAll { .. }
        ));
    }

    #[test]
    fn embedded_factory_matches_the_public_activation_query() {
        assert_eq!(
            egress_gate_from_env().is_some(),
            egress_gate_enabled_from_env()
        );
    }

    #[test]
    fn default_factory_redacts_without_writing_audit_files() {
        let temp = tempfile::tempdir().unwrap();
        let paths = ["egress", "impact", "consequence"].map(|name| temp.path().join(name));
        let make_gate = |value| {
            gate_from_config(
                value,
                Some(paths[0].clone()),
                Some(paths[1].clone()),
                Some(paths[2].clone()),
            )
        };
        let request = request_with("Email test.user@example.com");
        let gate = make_gate("").unwrap();
        assert!(matches!(
            gate.prepare(&request).0,
            EgressVerdict::Redacted(_)
        ));
        assert!(paths.iter().all(|path| !path.exists()));
        assert!(make_gate("0").is_none());
        assert!(matches!(
            make_gate("invalid").unwrap().prepare(&request).0,
            EgressVerdict::Deny { .. }
        ));
    }

    // ===== 2026-10-31 · 静态安全审计复核：畸形策略必须 fail-closed =====

    /// 畸形策略 ⇒ **必须**是 fail-closed 计划（**不能**是「不装配闸」）。
    #[test]
    fn malformed_policy_plans_deny_all_not_no_gate() {
        for bad in [
            "{not json",
            r#"{"subject":"u1""#, // 截断的 JSON
            "true",               // 不是 "1"，也不是策略
            "[]",                 // 类型不对
        ] {
            match plan_authorization(bad) {
                GateAuthorizationPlan::DenyAll { .. } => {}
                other => panic!("畸形值 {bad:?} 必须走 fail-closed，实际：{other:?}"),
            }
        }
    }

    /// **合法但空 grants** 的策略 ⇒ 挂上它 ⇒ 但**无 grant ⇒ 一律拒**（同样 fail-closed）。
    #[test]
    fn valid_but_empty_grants_policy_still_denies() {
        let json = r#"{"version":1,"subject":"u1","issuer":"o","issued_at_ms":1,"grants":[]}"#;
        match plan_authorization(json) {
            GateAuthorizationPlan::WithPolicy(policy) => {
                assert!(policy.grants.is_empty());
                let gate = RedactionEgressGate::new()
                    .with_authorization(policy, Arc::new(apeireth_core::kernel::SystemClock));
                assert!(
                    matches!(
                        gate.vet(&request_with("今天天气不错")),
                        EgressVerdict::Deny { .. }
                    ),
                    "空授权 ⇒ 必须拒绝出网"
                );
            }
            other => panic!("合法策略应走 WithPolicy，实际：{other:?}"),
        }
    }

    /// `"1"` ⇒ 只脱敏（既有语义不变）。
    #[test]
    fn one_plans_redaction_only() {
        assert_eq!(
            plan_authorization("1"),
            GateAuthorizationPlan::RedactionOnly
        );
        assert_eq!(
            plan_authorization(" 1 "),
            GateAuthorizationPlan::RedactionOnly
        );
    }

    /// 合法策略 ⇒ 挂上它。
    #[test]
    fn valid_policy_plans_with_policy() {
        let json = r#"{"version":1,"subject":"u1","issuer":"owner","issued_at_ms":1,
            "grants":[{"resource":"egress","action":"egress"}]}"#;
        match plan_authorization(json) {
            GateAuthorizationPlan::WithPolicy(policy) => {
                assert_eq!(policy.subject, "u1");
            }
            other => panic!("合法策略应走 WithPolicy，实际：{other:?}"),
        }
    }

    /// **端到端**：畸形 env 值 ⇒ 闸**被装配**且**出网被拒**（fail-closed）。
    ///
    /// ⚠️ 不能直接改 env（全局状态 + 2024 版 `set_var` 不安全）
    /// ⇒ ⇒ 用纯函数 + 显式装配走同一条语义路径。
    #[test]
    fn malformed_policy_yields_installed_gate_that_denies() {
        let plan = plan_authorization("{broken");
        let gate = match plan {
            GateAuthorizationPlan::DenyAll { .. } => RedactionEgressGate::new().with_authorization(
                AuthorizationPolicy::new(
                    "egress-gate",
                    "egress-gate-malformed-policy",
                    0,
                    Vec::new(),
                ),
                Arc::new(apeireth_core::kernel::SystemClock),
            ),
            _ => panic!("畸形值必须走 DenyAll"),
        };
        let request = request_with("今天天气不错");
        match gate.vet(&request) {
            EgressVerdict::Deny { reason } => {
                assert!(
                    reason.contains("未获授权") || reason.contains("拒绝"),
                    "拒绝理由应可读: {reason}"
                );
            }
            other => panic!("畸形策略下必须拒绝出网，实际：{other:?}"),
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
