//! 自省通道的生产端到端证明: 真 CLI 生产装配 (keyless, 无网络)。
//!
//! 覆盖: `tool.self_status` 默认可用且零审批 (治理只 grant 不审批)、能力
//! 名册与 env 生效值一致、记忆账本计数对账真实存储、凭据只回存在性布尔
//! (绝不回显凭据本体)、既有内置只读工具零回归。`std::env` 是进程全局,
//! 全部用例串在一把锁后。

use std::sync::Mutex;

use apeireth_cli::build_canonical_runtime_from_env;
use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{Action, Decision, GovernanceRequest};
use apeireth_memory::{EpisodeStore, MemoryGovernanceStore, SqliteMemoryStore};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolResult};
use apeireth_runtime::canonical::Runtime;
use serde_json::Value;

static ENV_LOCK: Mutex<()> = Mutex::new(());

const GUARDED_KEYS: &[&str] = &[
    "APEIRETH_SESSION_DB",
    "APEIRETH_COGNITIVE_DB",
    "APEIRETH_DATA_DIR",
    "APEIRETH_API_KEY",
    "APEIRETH_ANTHROPIC_KEY",
    "APEIRETH_OPENAI_KEY",
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "APEIRETH_OPENAI_MODELS",
    "APEIRETH_MODEL",
    "APEIRETH_COGNITIVE_JUDGE",
    "APEIRETH_COGNITIVE_COUNCIL",
    "APEIRETH_ENABLE_ORGANS",
    "APEIRETH_ENABLE_PREFERENCE_LEARNING",
    "APEIRETH_DISABLE_PREFERENCE_LEARNING",
    "APEIRETH_ENABLE_SHELL",
    "APEIRETH_ENABLE_FETCH",
    "APEIRETH_ENABLE_MCP",
    "APEIRETH_ENABLE_SELF_TUNING",
    "APEIRETH_TUNE_MEMORY_FADE",
    "APEIRETH_TUNE_CURIOSITY_STRENGTH",
    "APEIRETH_TUNE_TONE_SATURATION",
    "APEIRETH_TUNE_CONSOLIDATION_CADENCE",
    "APEIRETH_ENABLE_LOCAL_READ_TOOLS",
    "APEIRETH_DISABLE_LOCAL_READ_TOOLS",
    "APEIRETH_ENABLE_MEMORY_INJECTION",
    "APEIRETH_DISABLE_MEMORY_INJECTION",
    "APEIRETH_ENABLE_PROACTIVE_RECALL",
    "APEIRETH_DISABLE_PROACTIVE_RECALL",
    "APEIRETH_ENABLE_CONSOLIDATION",
    "APEIRETH_ENABLE_REFLEXION",
    "APEIRETH_REFLEXION_DIR",
    "APEIRETH_CONTEXT_BUDGET_CHARS",
    "APEIRETH_MAX_TURN_ROUNDS",
    "APEIRETH_MAX_TOOL_CALLS",
    "APEIRETH_KEYRING_BACKEND",
    "APEIRETH_KEYRING_DIR",
];

struct EnvGuard {
    keys: &'static [&'static str],
    previous: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn guard() -> Self {
        let previous = GUARDED_KEYS
            .iter()
            .map(|key| (*key, std::env::var(key).ok()))
            .collect();
        Self {
            keys: GUARDED_KEYS,
            previous,
        }
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

fn temp_path(label: &str, name: &str) -> String {
    std::env::temp_dir()
        .join(format!(
            "apeireth-self-status-e2e-{label}-{}-{name}",
            std::process::id()
        ))
        .to_string_lossy()
        .into_owned()
}

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

async fn build_runtime(label: &str) -> Runtime {
    std::env::set_var("APEIRETH_SESSION_DB", temp_path(label, "session.sqlite3"));
    std::env::set_var(
        "APEIRETH_COGNITIVE_DB",
        temp_path(label, "cognitive.sqlite3"),
    );
    build_canonical_runtime_from_env()
        .await
        .expect("keyless production runtime builds without network")
}

fn find_tool(runtime: &Runtime, capability_id: &str) -> std::sync::Arc<dyn ToolCapability> {
    runtime
        .capability_registry()
        .entries()
        .into_iter()
        .map(|(_owner, capability)| capability)
        .find(|capability| capability.id().as_str() == capability_id)
        .unwrap_or_else(|| panic!("{capability_id} must be registered"))
}

async fn invoke_self_status(runtime: &Runtime) -> Value {
    let tool = find_tool(runtime, "tool.self_status");
    let call = ToolCall {
        id: "call_self_status".into(),
        name: "self_status".into(),
        arguments: serde_json::json!({}),
    };
    let result: ToolResult = tool.invoke(&call).await;
    assert!(result.is_ok(), "{}", result.render());
    serde_json::from_str(&result.render()).expect("structured self-report json")
}

fn episode(id: &str, session: &str) -> apeireth_core::Episode {
    apeireth_core::Episode {
        id: id.to_string(),
        timestamp: 1_700_000_000,
        role: "user".to_string(),
        content: format!("记忆内容原文-{id}"),
        session_id: session.to_string(),
    }
}

#[tokio::test]
async fn self_status_is_granted_by_default_with_zero_approval() {
    let _lock = lock_env();
    let guard = EnvGuard::guard();
    guard.clear_all();
    let runtime = build_runtime("grant").await;

    let capability = CapabilityId::new("tool.self_status").expect("capability id");
    let arguments = serde_json::json!({});
    let request = GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability: &capability,
            arguments: &arguments,
        },
        SessionId::new(),
        TraceId::new(),
        1,
    );
    let verdict = runtime.governance().evaluate_verbose(&request).await;
    assert!(
        verdict.decision.is_allowed(),
        "只读档自述面必须默认可用且零审批: {}",
        verdict.decision
    );
    assert!(
        !matches!(verdict.decision, Decision::RequireApproval { .. }),
        "self_status 不得要求审批: {}",
        verdict.decision
    );

    // 未授权能力仍被拒绝 (对照: 未知能力 fail-closed)。
    let unknown = CapabilityId::new("tool.unknown").expect("capability id");
    let arguments = serde_json::json!({});
    let request = GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability: &unknown,
            arguments: &arguments,
        },
        SessionId::new(),
        TraceId::new(),
        1,
    );
    let verdict = runtime.governance().evaluate_verbose(&request).await;
    assert!(
        matches!(verdict.decision, Decision::Deny { .. }),
        "{}",
        verdict.decision
    );
}

#[tokio::test]
async fn roster_matches_env_effective_switches_and_identity_is_real() {
    let _lock = lock_env();
    let guard = EnvGuard::guard();
    guard.clear_all();
    std::env::set_var("APEIRETH_ENABLE_ORGANS", "1");
    std::env::set_var("APEIRETH_ENABLE_CONSOLIDATION", "1");
    std::env::set_var("APEIRETH_ENABLE_PREFERENCE_LEARNING", "0");
    std::env::set_var("APEIRETH_DISABLE_MEMORY_INJECTION", "1");
    std::env::set_var("APEIRETH_DISABLE_LOCAL_READ_TOOLS", "1");
    std::env::set_var("APEIRETH_ENABLE_SELF_TUNING", "1");

    let runtime = build_runtime("roster").await;
    let report = invoke_self_status(&runtime).await;

    // 身份: 产品名 / workspace 版本 / 运行时角色。
    assert_eq!(report["identity"]["product_name"], "apeireth");
    assert_eq!(report["identity"]["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(report["identity"]["runtime_role"], "gateway-sidecar");

    // 能力名册 = 生效值 (装配真正用的开关), 非配置文本。
    let roster = report["capabilities"].as_object().expect("roster");
    assert_eq!(roster["organs"], true);
    assert_eq!(roster["consolidation"], true);
    assert_eq!(roster["preference_learning"], false);
    assert_eq!(roster["memory_injection"], false);
    assert_eq!(roster["local_read_tools"], false);
    assert_eq!(roster["self_tuning"], true);
    assert_eq!(roster["shell"], false);
    assert_eq!(roster["fetch"], false);
    assert_eq!(roster["memory_recall"], true);
    assert_eq!(roster["memory_writeback"], true);
}

#[tokio::test]
async fn memory_ledger_counts_track_the_cognitive_store() {
    let _lock = lock_env();
    let guard = EnvGuard::guard();
    guard.clear_all();

    // 先造数再装配: 探测口读同一认知库的可查元数据。
    let cognitive = temp_path("ledger", "cognitive.sqlite3");
    std::env::set_var("APEIRETH_COGNITIVE_DB", &cognitive);
    // 教训存储钉到临时目录: 计数断言不吃宿主环境的既有教训。
    std::env::set_var("APEIRETH_REFLEXION_DIR", temp_path("ledger", "reflexion"));
    {
        let store = SqliteMemoryStore::open(&cognitive).expect("store");
        for (id, session) in [("e1", "s1"), ("e2", "s1"), ("e3", "s2")] {
            store.put_episode(&episode(id, session)).expect("episode");
        }
        store.protect_episode("e1", 0).expect("protect");
    }

    let runtime = build_runtime("ledger").await;
    let report = invoke_self_status(&runtime).await;
    let ledger = &report["memory_ledger"];
    assert_eq!(ledger["sessions"], 2);
    assert_eq!(ledger["memories"], 3);
    assert_eq!(ledger["protected"], 1);
    assert_eq!(ledger["lessons"], 0);
    assert_eq!(ledger["reason"], Value::Null);

    // 计数不回内容原文。
    let rendered = report.to_string();
    for leaked in ["记忆内容原文-e1", "记忆内容原文-e2", "记忆内容原文-e3"] {
        assert!(
            !rendered.contains(leaked),
            "memory content leaked: {leaked}"
        );
    }
}

#[tokio::test]
async fn credential_presence_is_boolean_only_and_never_echoes_values() {
    let _lock = lock_env();
    let guard = EnvGuard::guard();
    guard.clear_all();

    let data_dir = temp_path("creds", "data");
    std::fs::create_dir_all(&data_dir).expect("data dir");
    std::env::set_var("APEIRETH_DATA_DIR", &data_dir);
    let secret = "sk-live-e2e-must-not-be-echoed";
    std::fs::write(
        std::path::PathBuf::from(&data_dir).join("creds.json"),
        format!("{{\"provider\": {{\"api_key\": \"{secret}\"}}}}"),
    )
    .expect("creds file");

    let runtime = build_runtime("creds-present").await;
    let report = invoke_self_status(&runtime).await;
    assert_eq!(report["workspace"]["credentials_present"], true);
    assert!(
        !report.to_string().contains(secret),
        "凭据本体绝不能回显: {}",
        report
    );

    // 凭据面文件本身仍然不可读 (安全契约, fail-closed)。
    let filesystem = find_tool(&runtime, "tool.filesystem");
    let call = ToolCall {
        id: "call_read".into(),
        name: "filesystem".into(),
        arguments: serde_json::json!({ "operation": "read", "path": "creds.json" }),
    };
    // 数据目录在工作区外时路径级拒绝 (工作区边界); 在工作区内时凭据面拒绝。
    // 两种拒绝都是失败帧, 且绝不吐出凭据内容。
    let denied = filesystem.invoke(&call).await;
    assert!(!denied.is_ok(), "凭据面必须拒绝读取");
    assert!(!denied.render().contains(secret), "{}", denied.render());

    // 无凭据 = false (布尔存在性), 依旧不回显。
    std::fs::remove_file(std::path::PathBuf::from(&data_dir).join("creds.json")).expect("remove");
    let runtime = build_runtime("creds-absent").await;
    let report = invoke_self_status(&runtime).await;
    assert_eq!(report["workspace"]["credentials_present"], false);
}

#[tokio::test]
async fn budget_reports_the_live_configurable_knobs() {
    let _lock = lock_env();
    let guard = EnvGuard::guard();
    guard.clear_all();
    // 刚拍的可配置预算口径: 自述预算与运行时装配走同一条旋钮解析路径。
    std::env::set_var("APEIRETH_MAX_TURN_ROUNDS", "2");
    std::env::set_var("APEIRETH_MAX_TOOL_CALLS", "4");

    let runtime = build_runtime("budget").await;
    let report = invoke_self_status(&runtime).await;
    assert_eq!(report["budget"]["max_rounds_per_turn"], 2);
    assert_eq!(report["budget"]["max_tool_calls_per_round"], 4);
    assert_eq!(report["budget"]["source"], "configured");
    assert!(
        report["budget"]["note"].as_str().unwrap().contains("knobs"),
        "{}",
        report["budget"]["note"]
    );
    assert_eq!(report["budget"]["reason"], Value::Null);
}

#[tokio::test]
async fn builtin_read_tools_keep_working_zero_regression() {
    let _lock = lock_env();
    let guard = EnvGuard::guard();
    guard.clear_all();
    let runtime = build_runtime("regression").await;

    // 旧工具照常可用: 配置文件读取 (非敏感字段不脱敏不改字节)。
    let filesystem = find_tool(&runtime, "tool.filesystem");
    let call = ToolCall {
        id: "call_read".into(),
        name: "filesystem".into(),
        arguments: serde_json::json!({ "operation": "read", "path": "Cargo.toml" }),
    };
    let result = filesystem.invoke(&call).await;
    assert!(result.is_ok(), "{}", result.render());
    let content = result.render();
    assert!(content.contains("[package]"), "旧读取回归: {content}");
    assert!(content.contains("apeireth-cli"), "旧读取回归: {content}");

    // 旧检索照常可用。
    let search = find_tool(&runtime, "tool.search");
    let call = ToolCall {
        id: "call_search".into(),
        name: "search".into(),
        arguments: serde_json::json!({ "query": "apeireth-cli", "path": "." }),
    };
    let result = search.invoke(&call).await;
    assert!(result.is_ok(), "{}", result.render());
    assert!(
        result.render().contains("Cargo.toml"),
        "{}",
        result.render()
    );
}
