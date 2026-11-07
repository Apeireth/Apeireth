//! 批次 3a: 凭据形状清扫在**转录汇聚点**的兜底 (默认关 + env 开关).
//!
//! 验收判据 (逐条对应):
//!
//! ① 开关**关闭** ⇒ transcript 里的内容**逐字节不变**, 且**不挂任何侧车**;
//! ② 开关**打开** ⇒ 含 `sk-` + ≥20 字符的消息 ⇒ transcript 里**无明文**且含
//!    内核既有标签 `[REDACTED_OPENAI_KEY]`;
//! ③ 开关**打开** ⇒ **用户消息**与**工具结果消息****都**被扫 (两个用例);
//! ④ 开关**打开** ⇒ **干净消息逐字节不变** (零命中不变形);
//! ⑤ 遮形**不影响**消息的其它属性 (角色 / id / 顺序 ⇒ transcript 长度与顺序不变);
//! ⑥ ⚠️ **默认关的判据**: 不设 env ⇒ 行为与「关闭」**完全一致**;
//! ⑦ (设计稿批次 3a 追加判据) **事件日志面**也不含明文 (不得只清消息面漏日志面).
//!
//! # env 纪律
//!
//! `std::env::set_var` 改的是**进程级**状态 ⇒ 同一测试二进制里任何另一个测试都
//! 可能同时读到它 ⇒ ⇒ 本文件所有改 env 的用例都持 `ENV_LOCK` (与
//! `crates/adapters/cli/tests/production_knobs.rs` 同款纪律), 且 `EnvGuard` 在
//! 退出 (含 panic 展开) 时**还原**原值 ⇒ 不污染同二进制的其它用例.

use std::sync::{Arc, Mutex, MutexGuard};

use apeireth_core::kernel::{Clock, SessionId, Timestamp, VirtualClock};
use apeireth_governance::credential_sweep::credential_sweep_env_name;
use apeireth_protocol::canonical::{ContentPart, MessageRole, NormalizedMessage};
use apeireth_runtime::{Session, SessionEventKind};

/// 进程级 env 的串行化闸.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// 进程级 env 隔离: 退出 (含 panic 展开) 时**还原**原值.
struct EnvGuard {
    key: String,
    prev: Option<String>,
}

impl EnvGuard {
    fn set(key: &str, value: Option<&str>) -> Self {
        let prev = std::env::var(key).ok();
        match value {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
        Self {
            key: key.to_string(),
            prev,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.prev {
            Some(v) => std::env::set_var(&self.key, v),
            None => std::env::remove_var(&self.key),
        }
    }
}

/// 持锁地把开关设成某个值 (值 = `None` ⇒ **未设**, 即默认关的判据面).
fn switch(value: Option<&str>) -> (MutexGuard<'static, ()>, EnvGuard) {
    let lock = ENV_LOCK.lock().unwrap();
    let guard = EnvGuard::set(&credential_sweep_env_name(), value);
    (lock, guard)
}

/// 测试时钟 (虚拟时钟 ⇒ 确定性).
fn clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(1_700_000_000_000)
            .unwrap()
            .as_datetime(),
    ))
}

/// 一条消息的文本面 (形状层的清扫面).
fn text_of(message: &NormalizedMessage) -> String {
    ContentPart::join_text(&message.content)
}

/// 一枚「真实形态」的样例凭据 (前缀 + 足够长的主体 ⇒ 形状层命中).
fn sample_key() -> String {
    format!("sk-{}", "A".repeat(24))
}

/// 开关名必须与登记表 (`docs/architecture/env-registry.toml`) 登记的那条一致.
///
/// 这一条把「运维面看到的名字」钉死: 名字是**拼**出来的 (前缀 + 后缀), 但拼出来
/// 的必须正是登记的那一个.
#[test]
fn the_switch_name_is_the_registered_one() {
    assert_eq!(
        credential_sweep_env_name(),
        "APEIRETH_ENABLE_CREDENTIAL_SWEEP",
        "开关名必须与登记表一致"
    );
}

/// ① 开关**关闭** ⇒ transcript 逐字节不变, 且不挂任何侧车.
#[test]
fn with_the_switch_off_the_transcript_is_byte_identical() {
    let (_lock, _env) = switch(Some("0"));
    let secret = sample_key();
    let raw = format!("my key is {secret} thanks");
    let original = NormalizedMessage::user(raw.clone());

    let mut session = Session::new(SessionId::new(), clock().as_ref());
    session.append(original.clone(), clock().as_ref());

    assert_eq!(session.len(), 1, "关闭时不得多出/少掉消息");
    assert_eq!(
        session.messages[0], original,
        "关闭时 transcript 必须逐字节不变"
    );
    assert_eq!(text_of(&session.messages[0]), raw);
    assert!(
        text_of(&session.messages[0]).contains(&secret),
        "关闭时明文原样保留 (本批不改变默认行为)"
    );
    // 不挂任何侧车: 消息形状与原消息全等 (无附加字段 / 无附加标记).
    assert!(session.messages[0].tool_calls.is_empty());
    assert!(session.messages[0].tool_call_id.is_none());
    assert!(session.messages[0].name.is_none());
}

/// ② 开关**打开** ⇒ 含 `sk-` + ≥20 字符的消息 ⇒ 无明文 + 带内核既有标签.
#[test]
fn with_the_switch_on_a_key_shaped_secret_leaves_no_plaintext() {
    let (_lock, _env) = switch(Some("1"));
    let secret = sample_key();
    let raw = format!("here is my key {secret} thanks");

    let mut session = Session::new(SessionId::new(), clock().as_ref());
    session.append(NormalizedMessage::user(raw), clock().as_ref());

    let text = text_of(&session.messages[0]);
    assert!(!text.contains(&secret), "明文不得留在 transcript: {text}");
    assert!(
        text.contains("[REDACTED_OPENAI_KEY]"),
        "必须带内核既有形状层标签: {text}"
    );
}

/// ③(其一) 开关**打开** ⇒ **用户消息**被扫 (用户自己粘进来的凭据不漏).
#[test]
fn with_the_switch_on_a_user_message_is_swept() {
    let (_lock, _env) = switch(Some("1"));
    let secret = sample_key();

    let mut session = Session::new(SessionId::new(), clock().as_ref());
    session.append(
        NormalizedMessage::user(format!("user pasted {secret}")),
        clock().as_ref(),
    );

    let text = text_of(&session.messages[0]);
    assert!(
        !text.contains(&secret),
        "用户消息里的明文不得留在 transcript: {text}"
    );
    assert!(
        text.contains("[REDACTED_OPENAI_KEY]"),
        "用户消息必须带内核既有形状层标签: {text}"
    );
}

/// ③(其二) 开关**打开** ⇒ **工具结果消息**被扫 (能力回吐的凭据不漏).
#[test]
fn with_the_switch_on_a_tool_result_message_is_swept() {
    let (_lock, _env) = switch(Some("1"));
    let secret = sample_key();

    let mut session = Session::new(SessionId::new(), clock().as_ref());
    session.append(
        NormalizedMessage::tool_result(
            "call-1",
            Some("tool.fs".to_string()),
            format!("tool echoed {secret}"),
        ),
        clock().as_ref(),
    );

    let text = text_of(&session.messages[0]);
    assert!(
        !text.contains(&secret),
        "工具结果消息里的明文不得留在 transcript: {text}"
    );
    assert!(
        text.contains("[REDACTED_OPENAI_KEY]"),
        "工具结果消息必须带内核既有形状层标签: {text}"
    );
}

/// ③(其三) 第二个接线点 (`extend`) 同样兜一道.
#[test]
fn with_the_switch_on_extend_sweeps_too() {
    let (_lock, _env) = switch(Some("1"));
    let secret = sample_key();

    let mut session = Session::new(SessionId::new(), clock().as_ref());
    session.extend(
        vec![
            NormalizedMessage::user(format!("u {secret}")),
            NormalizedMessage::tool_result("call-7", None, format!("t {secret}")),
        ],
        clock().as_ref(),
    );

    assert_eq!(session.len(), 2);
    for message in &session.messages {
        let text = text_of(message);
        assert!(!text.contains(&secret), "明文不得留在 transcript: {text}");
        assert!(
            text.contains("[REDACTED_OPENAI_KEY]"),
            "必须带内核既有形状层标签: {text}"
        );
    }
}

/// ④ 开关**打开** ⇒ 干净消息**逐字节不变** (零命中不变形).
#[test]
fn with_the_switch_on_a_clean_message_is_byte_identical() {
    let (_lock, _env) = switch(Some("1"));
    let raw = "nothing sensitive here: 12345 + abcdef";
    let original = NormalizedMessage::user(raw);

    let mut session = Session::new(SessionId::new(), clock().as_ref());
    session.append(original.clone(), clock().as_ref());

    assert_eq!(session.len(), 1);
    assert_eq!(session.messages[0], original, "干净消息必须逐字节不变");
    assert_eq!(text_of(&session.messages[0]), raw);
}

/// ⑤ 遮形**不影响**消息的其它属性 (角色 / id / 顺序 ⇒ 长度与顺序不变).
#[test]
fn masking_leaves_roles_ids_and_order_untouched() {
    let (_lock, _env) = switch(Some("1"));
    let secret = sample_key();

    let mut session = Session::new(SessionId::new(), clock().as_ref());
    session.append(
        NormalizedMessage::user(format!("u {secret}")),
        clock().as_ref(),
    );
    session.append(
        NormalizedMessage::assistant(format!("a {secret}")),
        clock().as_ref(),
    );
    session.append(
        NormalizedMessage::tool_result(
            "call-42",
            Some("tool.shell".to_string()),
            format!("t {secret}"),
        ),
        clock().as_ref(),
    );

    assert_eq!(session.len(), 3, "遮形不得改变 transcript 长度");
    assert_eq!(session.messages[0].role, MessageRole::User);
    assert_eq!(session.messages[1].role, MessageRole::Assistant);
    assert_eq!(session.messages[2].role, MessageRole::Tool);
    assert_eq!(
        session.messages[2].tool_call_id.as_deref(),
        Some("call-42"),
        "工具调用 id 不得变"
    );
    assert_eq!(
        session.messages[2].name.as_deref(),
        Some("tool.shell"),
        "工具名不得变"
    );
    // 顺序不变: 每条消息的**前缀**仍是它自己的角色标记.
    assert!(text_of(&session.messages[0]).starts_with("u "));
    assert!(text_of(&session.messages[1]).starts_with("a "));
    assert!(text_of(&session.messages[2]).starts_with("t "));
}

/// ⑥ ⚠️ **默认关的判据**: 不设 env ⇒ 行为与「关闭」**完全一致**.
#[test]
fn an_unset_switch_behaves_exactly_like_off() {
    let (_lock, _env) = switch(None);
    let secret = sample_key();
    let raw = format!("key {secret}");
    let original = NormalizedMessage::user(raw.clone());

    let mut session = Session::new(SessionId::new(), clock().as_ref());
    session.append(original.clone(), clock().as_ref());

    assert_eq!(session.len(), 1);
    assert_eq!(
        session.messages[0], original,
        "未设 env ⇒ 与「关闭」完全一致 (默认关)"
    );
    assert!(text_of(&session.messages[0]).contains(&secret));
}

/// ⑦ **事件日志面**也不含明文 (不得只清消息面漏日志面).
#[test]
fn the_event_log_carries_no_plaintext_either() {
    let (_lock, _env) = switch(Some("1"));
    let secret = sample_key();

    let mut session = Session::new(SessionId::new(), clock().as_ref());
    session.append(
        NormalizedMessage::user(format!("key {secret}")),
        clock().as_ref(),
    );

    let logged: Vec<NormalizedMessage> = session
        .log
        .iter()
        .filter_map(|entry| match &entry.event {
            SessionEventKind::MessageAppended { message, .. } => Some(message.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(logged.len(), 1, "日志面必须记着这一条追加");
    let text = text_of(&logged[0]);
    assert!(!text.contains(&secret), "日志面不得含明文: {text}");
    assert!(
        text.contains("[REDACTED_OPENAI_KEY]"),
        "日志面必须带内核既有形状层标签: {text}"
    );
}
