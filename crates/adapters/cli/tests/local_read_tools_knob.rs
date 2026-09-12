//! End-to-end knob tests for the local read-tools privacy escape hatch.
//!
//! These build the real production runtime (keyless, no network) and assert
//! the governance decisions for `tool.filesystem` / `tool.search` /
//! `tool.repo` under each environment combination. `std::env` is
//! process-global, so the tests are serialized behind one lock.

use std::sync::Mutex;

use apeireth_cli::{
    build_canonical_runtime_from_env, DISABLE_LOCAL_READ_TOOLS_ENV, ENABLE_LOCAL_READ_TOOLS_ENV,
};
use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest};
use apeireth_runtime::canonical::Runtime;

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Everything that influences the production bootstrap must be pinned, so a
/// developer's shell cannot leak a knob or a key into an otherwise
/// deterministic test.
const GUARDED_KEYS: &[&str] = &[
    ENABLE_LOCAL_READ_TOOLS_ENV,
    DISABLE_LOCAL_READ_TOOLS_ENV,
    "APEIRETH_SESSION_DB",
    "APEIRETH_COGNITIVE_DB",
    "APEIRETH_API_KEY",
    "APEIRETH_ANTHROPIC_KEY",
    "OPENAI_API_KEY",
    "APEIRETH_OPENAI_MODELS",
    "APEIRETH_MODEL",
    "APEIRETH_COGNITIVE_JUDGE",
    "APEIRETH_COGNITIVE_COUNCIL",
    "APEIRETH_ENABLE_ORGANS",
    "APEIRETH_ENABLE_PREFERENCE_LEARNING",
    "APEIRETH_ENABLE_SHELL",
    "APEIRETH_ENABLE_FETCH",
];

struct EnvGuard {
    keys: &'static [&'static str],
    previous: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn guard(keys: &'static [&'static str]) -> Self {
        let previous = keys
            .iter()
            .map(|key| (*key, std::env::var(key).ok()))
            .collect();
        Self { keys, previous }
    }

    fn clear_all(&self) {
        for key in self.keys {
            std::env::remove_var(key);
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, previous) in &self.previous {
            match previous {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

fn temp_db_path(label: &str) -> String {
    std::env::temp_dir()
        .join(format!(
            "apeireth-cli-knob-{label}-{}.sqlite3",
            std::process::id()
        ))
        .to_string_lossy()
        .into_owned()
}

async fn build_runtime(label: &str) -> Runtime {
    std::env::set_var(
        "APEIRETH_SESSION_DB",
        temp_db_path(&format!("{label}-session")),
    );
    std::env::set_var(
        "APEIRETH_COGNITIVE_DB",
        temp_db_path(&format!("{label}-cognitive")),
    );
    build_canonical_runtime_from_env()
        .await
        .expect("keyless production runtime builds without network")
}

async fn decision(runtime: &Runtime, capability: &str) -> Decision {
    let capability = CapabilityId::new(capability).expect("valid test capability");
    let request = GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability: &capability,
            arguments: &serde_json::Value::Null,
        },
        SessionId::new(),
        TraceId::new(),
        1,
    );
    runtime.governance().evaluate_verbose(&request).await.decision
}

async fn assert_allow(runtime: &Runtime, capability: &str) {
    let actual = decision(runtime, capability).await;
    assert!(actual.is_allowed(), "{capability}: {actual}");
}

async fn assert_deny(runtime: &Runtime, capability: &str) {
    let actual = decision(runtime, capability).await;
    assert!(
        matches!(actual, Decision::Deny { .. }),
        "{capability}: {actual}"
    );
}

/// Assert the local read tools match `allowed`, and `tool.repo` stays granted.
async fn assert_local_read_tools(runtime: &Runtime, allowed: bool) {
    for capability in ["tool.filesystem", "tool.search"] {
        if allowed {
            assert_allow(runtime, capability).await;
        } else {
            assert_deny(runtime, capability).await;
        }
    }
    assert_allow(runtime, "tool.repo").await;
}

#[tokio::test]
async fn local_read_tools_default_to_granted() {
    let _lock = ENV_LOCK.lock().unwrap();
    let guard = EnvGuard::guard(GUARDED_KEYS);
    guard.clear_all();

    let runtime = build_runtime("default").await;
    assert_local_read_tools(&runtime, true).await;
}

#[tokio::test]
async fn disable_env_rejects_local_read_tools() {
    let _lock = ENV_LOCK.lock().unwrap();
    let guard = EnvGuard::guard(GUARDED_KEYS);
    guard.clear_all();
    std::env::set_var(DISABLE_LOCAL_READ_TOOLS_ENV, "1");

    let runtime = build_runtime("disable").await;
    assert_local_read_tools(&runtime, false).await;
}

#[tokio::test]
async fn legacy_enable_env_still_grants_local_read_tools() {
    let _lock = ENV_LOCK.lock().unwrap();
    let guard = EnvGuard::guard(GUARDED_KEYS);
    guard.clear_all();
    std::env::set_var(ENABLE_LOCAL_READ_TOOLS_ENV, "1");

    let runtime = build_runtime("enable").await;
    assert_local_read_tools(&runtime, true).await;
}

#[tokio::test]
async fn disable_wins_over_enable() {
    let _lock = ENV_LOCK.lock().unwrap();
    let guard = EnvGuard::guard(GUARDED_KEYS);
    guard.clear_all();
    std::env::set_var(ENABLE_LOCAL_READ_TOOLS_ENV, "1");
    std::env::set_var(DISABLE_LOCAL_READ_TOOLS_ENV, "1");

    let runtime = build_runtime("both").await;
    assert_local_read_tools(&runtime, false).await;
}
