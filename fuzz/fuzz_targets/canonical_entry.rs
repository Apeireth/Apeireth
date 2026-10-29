#![no_main]
//! fuzz: **网关请求入口面** (`crates/adapters/gateway/src/canonical_entry.rs`)
//! —— 参数解码 + 入口校验 + 令牌门。
//!
//! 本 target 只打**被测模块自己写明的契约**；每条断言的出处写在断言旁的注释里。
//!
//! ## 面 1: 参数解码面 (`CanonicalChatRequest` / `CanonicalApprovalRequest`)
//!
//! 两个请求体是网关的第一道门 (模块文档: "This module owns decoding,
//! transport validation, ...")。契约:
//!
//! - `input` 是**必填** (`pub input: String`, 无 `serde(default)`) —— 缺字段必须拒;
//! - `input` / `model` / `system` 是 `String` —— **类型强制 = 0** (数字/对象/数组/null 一律拒);
//! - `session` / `approval` 是 UUID 包装 (`apeireth_core::kernel::ids`:
//!   "Generated ids ... wrap a UUID") —— 非 UUID 必须拒;
//! - 请求体只有这 4 个字段 —— **未知字段不得改变这一回合** (未知参数不进执行路径)。
//!
//! ## 面 2: 入口校验面 (`execute_chat` / `resolve_approval`)
//!
//! - `execute_chat`: "回合输入的最小校验 (空输入不成回合)" —— 空输入必须被拒成
//!   `InvalidRequest`, 且**不得**跑进运行时 (否则会落一个空会话);
//! - `resolve_approval`: 裁决词表**闭合** (`decision` 字段文档: "`approve`, `reject`,
//!   or `cancel`.") —— 词表外一律拒 (fail-closed), 理由要点明词表。
//!   **不得**把词表外的裁决默认成 approve (那是审批绕过)。
//!
//! ## 面 3: 令牌门面 (`GatewayTokenGate::parse` / `verify`)
//!
//! `verify` 的过门裁决必须**由真实凭证支撑**: `Pass` ⇒ 凭证真的命中;
//! `PassUnprotected` ⇒ 只可能出现在「env 未设 + 表空」或「显式逃生阀」两种口径;
//! `RejectInvalid` ⇒ 凭证确实没命中。逃生阀哨兵 `none` 永远不得成为真令牌。
//!
//! ## harness 侧的诚实边界
//!
//! - 运行时面 (`execute_chat` / `resolve_approval`) 需要一个 `Runtime`;
//!   fuzz crate 不引 tokio (不引新的外部依赖), 故用 std 手写一个**有限次轮询**的
//!   迷你执行器: 轮询预算耗尽 = 该次调用**放弃断言** (不是失败)。
//! - 运行时面有副作用 (会话车道 / 会话账本会累积), 故每进程调用预算封顶
//!   (`RUNTIME_CALL_BUDGET`); 超预算后只跑解码面与令牌门面。
//! - 运行时面用 `catch_unwind` 包住: **无 tokio 运行时**导致的 panic 属 harness 环境
//!   (不是库缺陷), 捕获后放弃断言, 不当成 crash。

use std::future::Future;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::pin::Pin;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::task::{Context, Poll, Wake, Waker};

use apeireth_core::kernel::{ApprovalId, SessionId};
use apeireth_gateway::canonical_entry::{
    CanonicalApprovalRequest, CanonicalChatRequest, CanonicalEntryError, GateVerdict,
    GatewayTokenGate,
};
use apeireth_gateway::client_tokens::{ClientTokenConfig, ClientTokenSpec, ClientTokenTable};
use apeireth_runtime::canonical::Runtime;
use libfuzzer_sys::fuzz_target;
use serde_json::Value;

/// 固定会话 / 审批 id: fuzz 只扰动**参数面**, 不扰动 id 面。
const SESSION_UUID: &str = "11111111-1111-4111-8111-111111111111";
const APPROVAL_UUID: &str = "22222222-2222-4222-8222-222222222222";

/// 运行时面每进程的调用预算 (会话车道 / 会话账本会累积, 封顶防内存无界)。
const RUNTIME_CALL_BUDGET: usize = 256;

/// 迷你执行器的轮询预算 (耗尽 = 放弃断言, 不是失败)。
const POLL_BUDGET: usize = 64;

// ---------------------------------------------------------------------------
// 迷你执行器 (std only)
// ---------------------------------------------------------------------------

struct NoopWake;

impl Wake for NoopWake {
    fn wake(self: Arc<Self>) {}
}

/// 有限次轮询一个 future: `None` = 预算内没跑完 (放弃断言, 不算失败)。
fn poll_bounded<F: Future>(mut future: Pin<&mut F>, max_polls: usize) -> Option<F::Output> {
    let waker = Waker::from(Arc::new(NoopWake));
    let mut context = Context::from_waker(&waker);
    for _ in 0..max_polls {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return Some(value),
            Poll::Pending => std::thread::yield_now(),
        }
    }
    None
}

/// 共享运行时 (零 provider / 零插件): 只够跑入口校验段。
fn shared_runtime() -> Option<Arc<Runtime>> {
    static RUNTIME: OnceLock<Option<Arc<Runtime>>> = OnceLock::new();
    RUNTIME
        .get_or_init(|| {
            let mut future = Box::pin(Runtime::builder().build());
            poll_bounded(future.as_mut(), POLL_BUDGET)?
                .ok()
                .map(Arc::new)
        })
        .clone()
}

/// 运行时面调用预算是否还有余额。
fn runtime_budget_left() -> bool {
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    CALLS.fetch_add(1, Ordering::Relaxed) < RUNTIME_CALL_BUDGET
}

fn session_id() -> SessionId {
    SessionId::from_str(SESSION_UUID).expect("固定会话 id 必须是 UUID")
}

fn approval_id() -> ApprovalId {
    ApprovalId::from_str(APPROVAL_UUID).expect("固定审批 id 必须是 UUID")
}

// ---------------------------------------------------------------------------
// 面 1: 参数解码面
// ---------------------------------------------------------------------------

/// `CanonicalChatRequest` 解码契约。
///
/// 出处: `canonical_entry.rs:42-54` (字段与 `serde(default)` 标注) +
/// `docs/gateway-api-contract.md` §3 (`session` 可选 / `input` 用户输入 /
/// `model`、`system` 可选) + `apeireth_core::kernel::ids` (generated id = UUID 包装)。
fn chat_decode_face(payload: &[u8]) {
    let decoded = serde_json::from_slice::<CanonicalChatRequest>(payload);
    let Ok(Value::Object(object)) = serde_json::from_slice::<Value>(payload) else {
        // 不是 JSON 对象体: 契约只约束对象体 (解码面直接拒, 无 panic)。
        return;
    };

    // ① `input` 必填 (无 `serde(default)`) ⇒ 缺字段必须解码失败。
    if !object.contains_key("input") {
        assert!(
            decoded.is_err(),
            "缺 `input` 的请求体必须解码失败 (`input: String` 无默认值 = 必填)"
        );
    }

    // ② `input` 是 String ⇒ 类型强制 = 0 (数字/布尔/对象/数组/null 一律拒)。
    if let Some(input) = object.get("input") {
        if !input.is_string() {
            assert!(
                decoded.is_err(),
                "`input` 非字符串必须解码失败 (`input: String`, 类型强制 = 0), 实际: {input:?}"
            );
        }
    }

    // ③ `model` / `system` 同为 String ⇒ 非字符串必须拒。
    for key in ["model", "system"] {
        if let Some(value) = object.get(key) {
            if !value.is_string() {
                assert!(
                    decoded.is_err(),
                    "`{key}` 非字符串必须解码失败 (`{key}: Option<String>`), 实际: {value:?}"
                );
            }
        }
    }

    // ④ `session` 是 UUID 包装 ⇒ 字符串形必须是可解析的 UUID, 且解码值与之一致。
    if let Some(Value::String(raw)) = object.get("session") {
        let parsed = SessionId::from_str(raw);
        assert!(
            parsed.is_ok(),
            "`session` 必须是 UUID 格式 (generated id wrap a UUID), 实际: {raw:?}"
        );
        if let (Ok(parsed), Ok(request)) = (parsed, &decoded) {
            assert_eq!(
                request.session,
                Some(parsed),
                "`session` 解码值与给定 UUID 不一致"
            );
        }
    }

    // ⑤ 未知字段不得改变这一回合 (未知参数不进执行路径)。
    let known = ["session", "input", "model", "system"];
    let mut lean = object.clone();
    lean.retain(|key, _| known.contains(&key.as_str()));
    if let (Ok(full), Ok(lean)) = (
        serde_json::from_value::<CanonicalChatRequest>(Value::Object(object.clone())),
        serde_json::from_value::<CanonicalChatRequest>(Value::Object(lean)),
    ) {
        assert_eq!(full.input, lean.input, "未知字段改写了 `input`");
        assert_eq!(full.session, lean.session, "未知字段改写了 `session`");
        assert_eq!(full.model, lean.model, "未知字段改写了 `model`");
        assert_eq!(full.system, lean.system, "未知字段改写了 `system`");
    }
}

/// `CanonicalApprovalRequest` 解码契约。
///
/// 出处: `canonical_entry.rs:247-255` (字段与 `serde(default)` 标注) +
/// `docs/gateway-api-contract.md` §6 (`{session, approval, decision, reason?}`)。
fn approval_decode_face(payload: &[u8]) {
    let decoded = serde_json::from_slice::<CanonicalApprovalRequest>(payload);
    let Ok(Value::Object(object)) = serde_json::from_slice::<Value>(payload) else {
        return;
    };

    // ① 必填字段: `session` / `approval` / `decision` 均无 `serde(default)`。
    for key in ["session", "approval", "decision"] {
        if !object.contains_key(key) {
            assert!(
                decoded.is_err(),
                "缺 `{key}` 的请求体必须解码失败 (`{key}` 无默认值 = 必填)"
            );
        }
    }

    // ② `decision` 是 String ⇒ 非字符串必须拒。
    if let Some(value) = object.get("decision") {
        if !value.is_string() {
            assert!(
                decoded.is_err(),
                "`decision` 非字符串必须解码失败 (`decision: String`), 实际: {value:?}"
            );
        }
    }

    // ③ `session` / `approval` 是 UUID 包装 ⇒ 字符串形必须是可解析的 UUID。
    if let Some(Value::String(raw)) = object.get("session") {
        assert!(
            SessionId::from_str(raw).is_ok(),
            "`session` 必须是 UUID 格式, 实际: {raw:?}"
        );
    }
    if let Some(Value::String(raw)) = object.get("approval") {
        assert!(
            ApprovalId::from_str(raw).is_ok(),
            "`approval` 必须是 UUID 格式, 实际: {raw:?}"
        );
    }

    // ④ 未知字段不得改变这次审批请求。
    let known = ["session", "approval", "decision", "reason"];
    let mut lean = object.clone();
    lean.retain(|key, _| known.contains(&key.as_str()));
    if let (Ok(full), Ok(lean)) = (
        serde_json::from_value::<CanonicalApprovalRequest>(Value::Object(object.clone())),
        serde_json::from_value::<CanonicalApprovalRequest>(Value::Object(lean)),
    ) {
        assert_eq!(full.session, lean.session, "未知字段改写了 `session`");
        assert_eq!(full.approval, lean.approval, "未知字段改写了 `approval`");
        assert_eq!(full.decision, lean.decision, "未知字段改写了 `decision`");
        assert_eq!(full.reason, lean.reason, "未知字段改写了 `reason`");
    }
}

// ---------------------------------------------------------------------------
// 面 2: 入口校验面
// ---------------------------------------------------------------------------

/// `execute_chat` 的入口校验契约。
///
/// 出处: `canonical_entry.rs:1601-1610` ("回合输入的最小校验 (空输入不成回合)";
/// 校验先于会话立案 ("400 请求不落空会话")) + `canonical_entry.rs:238-240`
/// (`InvalidRequest` = "Transport input was not meaningful enough to form a turn")。
fn chat_entry_face(payload: &[u8], flags: u8) {
    let Some(runtime) = shared_runtime() else {
        return;
    };
    if !runtime_budget_left() {
        return;
    }

    // 三种输入形态: 空 / 纯空白 / 任意字节 (都必须是「不成回合」或「成回合」,
    // 绝不能出现「空白输入被当成正常输入」或「正常输入被当成空」)。
    let input = match flags & 0b11 {
        0 => String::new(),
        1 => " \t\n\u{3000} ".to_string(),
        _ => String::from_utf8_lossy(payload).into_owned(),
    };
    let request = CanonicalChatRequest {
        session: Some(session_id()),
        input: input.clone(),
        model: None,
        system: None,
    };

    let mut future = Box::pin(apeireth_gateway::canonical_entry::execute_chat(
        runtime.as_ref(),
        request,
    ));
    let polled = catch_unwind(AssertUnwindSafe(|| {
        poll_bounded(future.as_mut(), POLL_BUDGET)
    }));
    let Ok(Some(result)) = polled else {
        // 没跑完 / harness 环境 panic (无 tokio 运行时): 放弃断言。
        return;
    };

    let blank = input.trim().is_empty();
    match result {
        Err(CanonicalEntryError::InvalidRequest(message)) => {
            assert!(
                blank,
                "非空输入被当成空输入拒了: {message:?} (input={input:?})"
            );
            assert!(
                message.contains("empty"),
                "拒绝理由必须说清事实 (空输入不成回合), 实际: {message:?}"
            );
        }
        Err(CanonicalEntryError::Runtime(_)) => {
            assert!(!blank, "空输入竟跑进了运行时 (空请求会落一个空会话)");
        }
        Ok(_) => {
            assert!(!blank, "空输入竟成了回合 (空输入不成回合)");
        }
    }
}

/// `resolve_approval` 的入口校验契约。
///
/// 出处: `canonical_entry.rs:251-252` (`decision` 字段文档: "`approve`, `reject`,
/// or `cancel`.") + `canonical_entry.rs:429-441` (词表外 → `InvalidRequest`,
/// 理由点名词表) + `canonical_entry.rs:402-404` (不存在的审批 → `InvalidRequest`)。
fn approval_entry_face(payload: &[u8], flags: u8) {
    let Some(runtime) = shared_runtime() else {
        return;
    };
    if !runtime_budget_left() {
        return;
    }

    let raw = String::from_utf8_lossy(payload).into_owned();
    let decision = match flags & 0b11 {
        0 => raw,
        1 => "approve".to_string(),
        2 => "reject".to_string(),
        3 => "cancel".to_string(),
        _ => raw,
    };
    let request = CanonicalApprovalRequest {
        session: session_id(),
        approval: approval_id(),
        decision: decision.clone(),
        reason: None,
    };

    let mut future = Box::pin(apeireth_gateway::canonical_entry::resolve_approval(
        runtime.as_ref(),
        request,
    ));
    let polled = catch_unwind(AssertUnwindSafe(|| {
        poll_bounded(future.as_mut(), POLL_BUDGET)
    }));
    let Ok(Some(result)) = polled else {
        return;
    };

    // 裁决词表闭合: 归一化 (trim + 小写) 后不在词表内 ⇒ 必须被拒成 `InvalidRequest`。
    let normalized = decision.trim().to_ascii_lowercase();
    let in_vocabulary = matches!(normalized.as_str(), "approve" | "reject" | "cancel");

    match result {
        Err(CanonicalEntryError::InvalidRequest(message)) => {
            if !in_vocabulary {
                assert!(
                    message.contains("unknown approval decision"),
                    "词表外裁决的拒绝理由必须点名裁决非法: {message:?} (decision={decision:?})"
                );
                assert!(
                    message.contains("approve")
                        && message.contains("reject")
                        && message.contains("cancel"),
                    "拒绝理由必须点明闭合词表 (approve/reject/cancel): {message:?}"
                );
            } else {
                // 词表内但审批不存在 ⇒ 事实理由「查无此审批」, 不得 500。
                assert!(
                    message.contains("not found"),
                    "不存在的审批必须给出「查无此审批」的事实理由, 实际: {message:?}"
                );
            }
        }
        Err(CanonicalEntryError::Runtime(_)) => {
            assert!(
                !in_vocabulary,
                "词表外的裁决 `{decision:?}` 竟被当成有效裁决执行"
            );
        }
        Ok(_) => {
            assert!(
                !in_vocabulary,
                "词表外的裁决 `{decision:?}` 竟解出了结果 (审批绕过)"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 面 3: 令牌门面
// ---------------------------------------------------------------------------

/// `GatewayTokenGate::parse` / `verify` 的契约。
///
/// 出处: `canonical_entry.rs:770-778` (`parse` 三态口径) + `canonical_entry.rs:794-844`
/// (`verify` 过门裁决口径) + `canonical_entry.rs:757-763` (逃生阀哨兵字面值;
/// "真实令牌不得取此值") + `client_tokens.rs:83-92` (`ClientTokenSpec::validate`:
/// 令牌不冒充逃生阀哨兵)。
fn token_gate_face(payload: &[u8], flags: u8) {
    let raw = String::from_utf8_lossy(payload).into_owned();

    // env 面: 三态口径 (缺失/空白 = 未设防默认态; 哨兵 = 显式逃生阀; 其余 = 强制令牌)。
    let gate = GatewayTokenGate::parse(Some(raw.clone()));
    if let GatewayTokenGate::Enforced(token) = &gate {
        assert_ne!(
            token.as_str(),
            GatewayTokenGate::UNPROTECTED_SENTINEL,
            "逃生阀哨兵不得成为真令牌"
        );
    }

    // 令牌表面: 一枚条目 (名/令牌来自同一段字节的两半)。
    //
    // ⚠️ 切点必须落在**字符边界**上：`raw.len() / 2` 是**字节**下标，
    // 多字节字符（中文/emoji/U+FFFD）会让它落在字符中间 ⇒ `split_at` 直接 panic
    // （`core::str::slice_error_fail_rt`）。这是 **harness 自身的 bug**，不是产品缺陷。
    // 修法：按 `char_indices` 找最近的合法切点；不足 2 个字符时按整体处理。
    let split = raw
        .char_indices()
        .map(|(index, _)| index)
        .find(|index| *index >= raw.len() / 2)
        .unwrap_or(raw.len());
    let (name, token) = raw.split_at(split);
    let table = ClientTokenTable::from_config(ClientTokenConfig {
        clients: vec![ClientTokenSpec {
            name: name.to_string(),
            token: token.to_string(),
            user_id: None,
        }],
    });
    if let Ok(table) = &table {
        for entry in table.entries() {
            assert_ne!(
                entry.token.trim(),
                GatewayTokenGate::UNPROTECTED_SENTINEL,
                "令牌表里出现了逃生阀哨兵 (「无防护」被当成可登录身份)"
            );
        }
    }
    let table = table.unwrap_or_default();

    let bearer = if flags & 0b1 != 0 {
        None
    } else {
        Some(raw.clone())
    };
    let verdict = gate.verify(bearer.as_deref(), &table);

    match verdict {
        GateVerdict::Pass { client } => {
            let env_hit = matches!(
                &gate,
                GatewayTokenGate::Enforced(token) if Some(token.as_str()) == bearer.as_deref()
            );
            let table_hit =
                table.authenticate(bearer.as_deref().unwrap_or("")) == Some(client.as_str());
            assert!(
                env_hit || table_hit,
                "过门放行却没有凭证命中 (client={client}, bearer={:?})",
                bearer
            );
            assert_ne!(
                bearer.as_deref(),
                Some(GatewayTokenGate::UNPROTECTED_SENTINEL),
                "逃生阀哨兵被当成真令牌放行"
            );
        }
        GateVerdict::PassUnprotected { mode } => match &gate {
            GatewayTokenGate::UnprotectedDefault => {
                assert!(table.is_empty(), "令牌表在场时不得走未设防放行口径");
                assert_eq!(mode, "env_unset", "未设防默认态的放行口径记号不对");
            }
            GatewayTokenGate::UnprotectedDeclared => {
                assert_eq!(mode, "declared_none", "逃生阀口径的放行口径记号不对");
            }
            GatewayTokenGate::Enforced(_) => {
                panic!("强制态令牌门不得放行未设防口径");
            }
        },
        GateVerdict::RejectMissing => {
            assert!(bearer.is_none(), "没带 Bearer 竟判成缺令牌");
        }
        GateVerdict::RejectInvalid => {
            let bearer = bearer.expect("无效裁决必带 Bearer");
            let env_hit = matches!(
                &gate,
                GatewayTokenGate::Enforced(token) if token.as_str() == bearer
            );
            assert!(
                !env_hit && table.authenticate(&bearer).is_none(),
                "判成无效令牌, 但凭证其实命中了 (bearer={bearer:?})"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 入口
// ---------------------------------------------------------------------------

fuzz_target!(|data: &[u8]| {
    let Some((mode, rest)) = data.split_first() else {
        return;
    };
    let Some((flags, payload)) = rest.split_first() else {
        return;
    };

    match mode % 5 {
        0 => chat_decode_face(payload),
        1 => approval_decode_face(payload),
        2 => chat_entry_face(payload, *flags),
        3 => approval_entry_face(payload, *flags),
        _ => token_gate_face(payload, *flags),
    }
});
