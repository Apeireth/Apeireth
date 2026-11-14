//! Construction-time wiring of the vendor-private request fields into the
//! **production** OpenAI-compatible LLM factory.
//!
//! ⭐ These tests drive the production path
//! ([`OpenAiCompatibleLlmFactory::from_env_with_resolver`]), not a hand-built
//! capability: the whole point of the change is that an operator can turn a
//! vendor-private switch on **without touching code**, and that a wrong value
//! fails **at construction** instead of at the first real request.
//!
//! ⭐ The process environment is global state: every test that mutates it
//! takes the one lock of this binary (`ENV_LOCK`) and restores the previous
//! values afterwards.

use std::sync::{Arc, Mutex, MutexGuard};

use apeireth_plugin::{CredentialResolver, ProviderCapability, StaticCredentials};
use apeireth_protocol::canonical::{NormalizedMessage, NormalizedRequest};
use apeireth_provider::credentials::OPENAI_COMPATIBLE_API_KEY;
use apeireth_provider::openai_compatible_llm_factory::{FactoryError, OpenAiCompatibleLlmFactory};
use apeireth_provider::vendor_body;

const FAKE_KEY: &str = "sk-openai-fake-test-key";
const MODEL: &str = "gpt-4o-mini";

/// The one environment lock of this test binary.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn env_lock() -> MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Put the environment back the way it was when this guard drops.
struct EnvRestore(Vec<(String, Option<String>)>);

impl EnvRestore {
    fn apply(vars: &[(&str, Option<&str>)]) -> Self {
        let mut saved = Vec::new();
        for (name, value) in vars {
            saved.push(((*name).to_string(), std::env::var(name).ok()));
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
        Self(saved)
    }
}

impl Drop for EnvRestore {
    fn drop(&mut self) {
        for (name, value) in &self.0 {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}

/// Build the production factory with the vendor-private switches cleared,
/// then with `vendor_env` applied on top.
///
/// ⭐ The switches are cleared first on purpose: the test process may inherit
/// a deployment's environment, and "no switch set" has to mean exactly that.
fn build_factory(
    base_url: &str,
    vendor_env: &[(&str, Option<&str>)],
) -> Result<OpenAiCompatibleLlmFactory, FactoryError> {
    let _guard = env_lock();
    let mut vars: Vec<(&str, Option<&str>)> = vec![
        ("APEIRETH_OPENAI_URL", Some(base_url)),
        ("APEIRETH_OPENAI_MODELS", Some(MODEL)),
        (vendor_body::THINKING_ENV, None),
    ];
    vars.extend_from_slice(vendor_env);
    let _restore = EnvRestore::apply(&vars);
    OpenAiCompatibleLlmFactory::from_env_with_resolver(fake_resolver())
}

fn fake_resolver() -> Arc<dyn CredentialResolver> {
    Arc::new(StaticCredentials::new().with(OPENAI_COMPATIBLE_API_KEY, FAKE_KEY))
}

fn request() -> NormalizedRequest {
    NormalizedRequest::new(MODEL, vec![NormalizedMessage::user("say hello")])
}

// ── a minimal one-shot HTTP mock (no new dependency) ──

struct MockServer {
    base_url: String,
    received: Arc<Mutex<Option<serde_json::Value>>>,
}

impl MockServer {
    async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let received = Arc::new(Mutex::new(None));
        let received_clone = Arc::clone(&received);
        tokio::spawn(async move {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let body = read_body(&mut socket).await;
            *received_clone.lock().unwrap() = body;
            write_ok(&mut socket).await;
        });
        Self {
            base_url: format!("http://{addr}"),
            received,
        }
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn received(&self) -> Option<serde_json::Value> {
        self.received.lock().unwrap().clone()
    }
}

async fn read_body<S>(socket: &mut S) -> Option<serde_json::Value>
where
    S: tokio::io::AsyncRead + Unpin,
{
    use tokio::io::AsyncReadExt;
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    let header_len = loop {
        let Ok(n) = socket.read(&mut tmp).await else {
            return None;
        };
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(idx) = buf.windows(4).position(|w| w == b"\r\n\r\n".as_slice()) {
            break idx + 4;
        }
    };
    let headers = String::from_utf8_lossy(&buf[..header_len]).to_string();
    let content_length = headers
        .lines()
        .find_map(|line| {
            let line = line.to_ascii_lowercase();
            line.strip_prefix("content-length: ")
                .and_then(|value| value.trim().parse::<usize>().ok())
        })
        .unwrap_or(0);
    while buf.len() < header_len + content_length {
        let Ok(n) = socket.read(&mut tmp).await else {
            return None;
        };
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
    }
    let text = String::from_utf8_lossy(&buf).to_string();
    let body_text = text.split("\r\n\r\n").nth(1).unwrap_or("");
    serde_json::from_str(body_text).ok()
}

async fn write_ok<S>(socket: &mut S)
where
    S: tokio::io::AsyncWrite + Unpin,
{
    use tokio::io::AsyncWriteExt;
    let body = serde_json::json!({
        "id": "chatcmpl-test",
        "model": MODEL,
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "hello world"},
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 12, "completion_tokens": 3, "total_tokens": 15}
    })
    .to_string();
    let head = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
        body.len()
    );
    let _ = socket.write_all(head.as_bytes()).await;
    let _ = socket.write_all(body.as_bytes()).await;
    let _ = socket.flush().await;
}

// ── the behaviour contract ──

/// ⭐ No switch set ⇒ the factory leaves the extra body **unset** ⇒ the
/// request body stays byte-identical to the one before this feature.
#[tokio::test]
async fn without_any_switch_the_factory_sets_no_extra_body() {
    let server = MockServer::start().await;
    let factory = build_factory(server.base_url(), &[]).expect("factory builds");

    assert!(
        factory.capability().extra_body().is_none(),
        "no switch ⇒ no vendor-private field may be attached"
    );
}

/// ⭐ The switch is a **production** knob: it must reach the wire, not just
/// sit in the capability.
#[tokio::test]
async fn the_reasoning_switch_reaches_the_wire() {
    let server = MockServer::start().await;
    let factory = build_factory(
        server.base_url(),
        &[(vendor_body::THINKING_ENV, Some("disabled"))],
    )
    .expect("factory builds");

    assert_eq!(
        factory.capability().extra_body(),
        Some(serde_json::json!({"thinking": {"type": "disabled"}})),
        "the switch must be attached to the capability"
    );

    factory
        .capability()
        .complete(&request())
        .await
        .expect("completes");
    let recorded = server.received().expect("server saw the request");
    assert_eq!(
        recorded["thinking"]["type"], "disabled",
        "the vendor field must reach the wire: {recorded}"
    );
}

/// ⭐ The factory sends **exactly** the fields it owns — and nothing else.
///
/// ⭐ The production factory owns one vendor knob (the reasoning-mode
/// switch). The long-range memory evaluation owns two more (its own
/// reasoning-effort and extra-body switches). A tool-local setting must
/// never change what the shipped product sends, so the factory ignores
/// switches it does not own — and the body it builds carries exactly one
/// key.
#[tokio::test]
async fn the_factory_sends_only_the_field_it_owns() {
    /// The evaluation's reasoning-effort switch (not the factory's).
    const EVAL_REASONING_EFFORT_ENV: &str = "APEIRETH_OPENAI_REASONING_EFFORT";
    /// The evaluation's extra-body switch (not the factory's).
    const EVAL_EXTRA_BODY_ENV: &str = "APEIRETH_OPENAI_EXTRA_BODY";

    let server = MockServer::start().await;
    let factory = build_factory(
        server.base_url(),
        &[
            (vendor_body::THINKING_ENV, Some("enabled")),
            (EVAL_REASONING_EFFORT_ENV, Some("low")),
            (EVAL_EXTRA_BODY_ENV, Some(r#"{"vendor_flag": true}"#)),
        ],
    )
    .expect("factory builds");

    assert_eq!(
        factory.capability().extra_body(),
        Some(serde_json::json!({"thinking": {"type": "enabled"}})),
        "the factory must attach only the switch it owns"
    );

    factory
        .capability()
        .complete(&request())
        .await
        .expect("completes");
    let recorded = server.received().expect("server saw the request");
    assert!(
        recorded.get("reasoning_effort").is_none() && recorded.get("vendor_flag").is_none(),
        "a switch the factory does not own must not reach the wire: {recorded}"
    );
}

/// ⭐ An unrecognised switch value fails **at construction**: it is never
/// silently ignored, and never treated as "off".
#[tokio::test]
async fn an_unknown_reasoning_switch_fails_at_construction() {
    let server = MockServer::start().await;
    let Err(error) = build_factory(
        server.base_url(),
        &[(vendor_body::THINKING_ENV, Some("maybe"))],
    ) else {
        panic!("an unknown switch value must be refused")
    };
    let message = error.to_string();
    assert!(
        matches!(error, FactoryError::VendorBody(_)),
        "the error must name the vendor-body source: {message}"
    );
    assert!(
        message.contains(vendor_body::THINKING_ENV),
        "the error must name the variable: {message}"
    );
    assert!(
        message.contains("maybe"),
        "the error must quote the offending value: {message}"
    );
    assert!(
        server.received().is_none(),
        "a refused switch must not produce a request"
    );
}

/// ⭐ The production factory owns **one** vendor knob: the reasoning-mode
/// switch. A variable that belongs to another surface (the long-range memory
/// evaluation owns its own extra-body switch) must not leak into a
/// production request body — otherwise a tool-local setting would silently
/// change what the shipped product sends.
#[tokio::test]
async fn a_vendor_switch_the_factory_does_not_own_never_reaches_the_wire() {
    /// The extra-body switch of the memory evaluation (not the factory's).
    const EVAL_EXTRA_BODY_ENV: &str = "APEIRETH_OPENAI_EXTRA_BODY";

    let server = MockServer::start().await;
    let factory = build_factory(
        server.base_url(),
        &[(EVAL_EXTRA_BODY_ENV, Some(r#"{"vendor_flag": true}"#))],
    )
    .expect("a foreign switch must not break the factory");

    assert!(
        factory.capability().extra_body().is_none(),
        "the factory must not attach a switch it does not own"
    );

    factory
        .capability()
        .complete(&request())
        .await
        .expect("completes");
    let recorded = server.received().expect("server saw the request");
    assert!(
        recorded.get("vendor_flag").is_none(),
        "a foreign switch must not reach the wire: {recorded}"
    );
}
