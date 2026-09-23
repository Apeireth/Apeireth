//! 2026-09-08 用户旋钮回归：shell/fetch 注册 + 审批策略、organs /
//! preference_learning 模块装配、Judge/Council env 旋钮。

use std::sync::Mutex;

use apeireth_cli::build_canonical_runtime_with_sessions_from_env;
use apeireth_governance::Permission;

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// 进程级 env 隔离 (与 canonical_cli_bootstrap 同款纪律: 并行测试不串 env)。
struct EnvGuard {
    key: &'static str,
    prev: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: Option<&str>) -> Self {
        let prev = std::env::var(key).ok();
        match value {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
        Self { key, prev }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.prev {
            Some(v) => std::env::set_var(self.key, v),
            None => std::env::remove_var(self.key),
        }
    }
}

fn temp_db(tag: &str) -> String {
    std::env::temp_dir()
        .join(format!(
            "apeireth-knob-{tag}-{}.sqlite3",
            std::process::id()
        ))
        .to_string_lossy()
        .into_owned()
}

fn module_ids(runtime: &apeireth_runtime::canonical::Runtime) -> Vec<String> {
    runtime
        .modules()
        .iter()
        .map(|m| m.manifest().id.clone())
        .collect()
}

/// shell 旋钮: 注册 tool.shell + 策略 grant + require_approval (fail-closed 语义).
#[tokio::test]
async fn shell_knob_registers_tool_and_requires_approval() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _g_shell = EnvGuard::set("APEIRETH_ENABLE_SHELL", Some("1"));
    let _g_db = EnvGuard::set("APEIRETH_COGNITIVE_DB", Some(&temp_db("shell")));
    let _g_sdb = EnvGuard::set("APEIRETH_SESSION_DB", Some(&temp_db("shell-session")));

    let (runtime, _sessions, _memory, policy, _guard_hook) =
        build_canonical_runtime_with_sessions_from_env()
            .await
            .expect("bootstrap with shell knob");

    let tool_ids: Vec<String> = runtime
        .tools()
        .iter()
        .map(|t| t.id().as_str().to_string())
        .collect();
    assert!(
        tool_ids.contains(&"tool.shell".to_string()),
        "shell knob must register tool.shell, got {tool_ids:?}"
    );
    let guard = policy.lock().unwrap();
    assert!(
        guard.has(&Permission::ExecuteTool("tool.shell".to_string())),
        "shell must be granted"
    );
    let approval: Vec<&str> = guard.approval_capabilities().collect();
    assert!(
        approval.contains(&"tool.shell"),
        "shell must be approval-marked (每次调用走审批), got {approval:?}"
    );
}

/// fetch 旋钮: 注册 + 审批标记 (与 shell 同语义).
#[tokio::test]
async fn fetch_knob_registers_tool_and_requires_approval() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _g_fetch = EnvGuard::set("APEIRETH_ENABLE_FETCH", Some("1"));
    let _g_db = EnvGuard::set("APEIRETH_COGNITIVE_DB", Some(&temp_db("fetch")));
    let _g_sdb = EnvGuard::set("APEIRETH_SESSION_DB", Some(&temp_db("fetch-session")));

    let (runtime, _sessions, _memory, policy, _guard_hook) =
        build_canonical_runtime_with_sessions_from_env()
            .await
            .expect("bootstrap with fetch knob");

    let tool_ids: Vec<String> = runtime
        .tools()
        .iter()
        .map(|t| t.id().as_str().to_string())
        .collect();
    assert!(tool_ids.contains(&"tool.fetch".to_string()), "{tool_ids:?}");
    let guard = policy.lock().unwrap();
    let approval: Vec<&str> = guard.approval_capabilities().collect();
    assert!(approval.contains(&"tool.fetch"), "{approval:?}");
}

/// organs + preference_learning 旋钮: 模块装配进 runtime (默认关闭的对照由
/// production_slot_order_is_explicit 测试锚定).
#[tokio::test]
async fn organs_and_preference_learning_knobs_register_modules() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _g_organs = EnvGuard::set("APEIRETH_ENABLE_ORGANS", Some("1"));
    let _g_pl = EnvGuard::set("APEIRETH_ENABLE_PREFERENCE_LEARNING", Some("1"));
    let _g_db = EnvGuard::set("APEIRETH_COGNITIVE_DB", Some(&temp_db("organs")));
    let _g_sdb = EnvGuard::set("APEIRETH_SESSION_DB", Some(&temp_db("organs-session")));

    let (runtime, _sessions, _memory, _policy, _guard_hook) =
        build_canonical_runtime_with_sessions_from_env()
            .await
            .expect("bootstrap with organ knobs");

    let ids = module_ids(&runtime);
    assert!(
        ids.contains(&"cognitive.organs".to_string()),
        "organ module must be wired, got {ids:?}"
    );
    assert!(
        ids.contains(&"cognitive.preference_learning".to_string()),
        "preference learning must be wired, got {ids:?}"
    );
}

// ---- 2026-10-06 W2 接线批旋钮 (engineering-review-handoff-2026-10-06.md §5 W2) ----

/// proactive recall 旋钮: 默认关 (行为不变), `=1` 时给 enabled 策略
/// (已接线的 `compile_prompt_overlay_with_proactive_access` 路径由此可达)。
#[test]
fn proactive_recall_knob_is_opt_in() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _g_off = EnvGuard::set("APEIRETH_ENABLE_PROACTIVE_RECALL", None);
    assert!(
        apeireth_cli::proactive_recall_policy_from_env().is_none(),
        "default must stay off"
    );
    let _g_on = EnvGuard::set("APEIRETH_ENABLE_PROACTIVE_RECALL", Some("1"));
    let policy = apeireth_cli::proactive_recall_policy_from_env()
        .expect("knob =1 must produce a policy");
    assert!(policy.enabled, "policy must be enabled");
    assert_eq!(policy.budget, 2, "deterministic default budget");
}

/// typed 写读对称: 读侧默认开 + 逃生门; 身份默认稳定、可覆写、空白值不采纳。
#[test]
fn typed_recall_defaults_on_and_identity_is_stable() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _g_kill_off = EnvGuard::set("APEIRETH_DISABLE_TYPED_RECALL", None);
    let _g_p = EnvGuard::set("APEIRETH_PERSONA_ID", None);
    let _g_s = EnvGuard::set("APEIRETH_SUBJECT_ID", None);
    assert!(
        apeireth_cli::typed_recall_enabled_from_env(),
        "read side defaults on (write/read symmetry)"
    );
    let identity = apeireth_cli::typed_recall_identity_from_env();
    assert_eq!(identity.persona_id, "apeireth");
    assert_eq!(identity.subject_id, "local-user");

    let _g_kill_on = EnvGuard::set("APEIRETH_DISABLE_TYPED_RECALL", Some("1"));
    assert!(
        !apeireth_cli::typed_recall_enabled_from_env(),
        "escape hatch must kill the read side"
    );

    let _g_p2 = EnvGuard::set("APEIRETH_PERSONA_ID", Some(" persona-x "));
    let _g_s2 = EnvGuard::set("APEIRETH_SUBJECT_ID", Some("user-x"));
    let identity = apeireth_cli::typed_recall_identity_from_env();
    assert_eq!(identity.persona_id, "persona-x", "trimmed override");
    assert_eq!(identity.subject_id, "user-x");
}

/// 语义向量阶段旋钮: 双缺 = None (词法回退不变); 只设其一 = **大声报错**
/// (半配是配置事故, 不静默); 双全 = 构造成功 (不发网络)。
#[test]
fn embedding_knob_fails_loud_on_partial_config() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _g_url = EnvGuard::set("APEIRETH_EMBEDDING_URL", None);
    let _g_model = EnvGuard::set("APEIRETH_EMBEDDING_MODEL", None);
    let _g_key = EnvGuard::set("APEIRETH_EMBEDDING_KEY", None);

    let none = apeireth_cli::embedding_provider_from_env().expect("no config is Ok");
    assert!(none.is_none(), "double-missing must stay lexical fallback");

    let _g_url_only = EnvGuard::set("APEIRETH_EMBEDDING_URL", Some("http://127.0.0.1:9/v1"));
    assert!(
        apeireth_cli::embedding_provider_from_env().is_err(),
        "half-configured must fail loud"
    );

    let _g_model_too = EnvGuard::set("APEIRETH_EMBEDDING_MODEL", Some("emb-test"));
    let provider = apeireth_cli::embedding_provider_from_env()
        .expect("full config must construct")
        .expect("full config yields Some");
    assert_eq!(provider.model_id(), "emb-test");
}

/// W2 三组旋钮齐开时生产组装不回归 (typed 对称 + proactive + embedding 构造
/// 全走一遍; 效果级证据在 runtime-assembly 的 memory_provider_e2e /
/// cognitive_convergence_vertical T7-T9)。
#[tokio::test]
async fn bootstrap_succeeds_with_w2_knobs_enabled() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _g_proactive = EnvGuard::set("APEIRETH_ENABLE_PROACTIVE_RECALL", Some("1"));
    let _g_url = EnvGuard::set("APEIRETH_EMBEDDING_URL", Some("http://127.0.0.1:9/v1"));
    let _g_model = EnvGuard::set("APEIRETH_EMBEDDING_MODEL", Some("emb-test"));
    let _g_db = EnvGuard::set("APEIRETH_COGNITIVE_DB", Some(&temp_db("w2")));
    let _g_sdb = EnvGuard::set("APEIRETH_SESSION_DB", Some(&temp_db("w2-session")));

    let (_runtime, _sessions, _memory, _policy, _guard_hook) =
        build_canonical_runtime_with_sessions_from_env()
            .await
            .expect("bootstrap with W2 knobs must succeed");
}

/// 逃生门开着也必须能正常组装 (fail-closed 惯例: 关 = 行为回到修复前)。
#[tokio::test]
async fn bootstrap_succeeds_with_typed_recall_killed() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _g_kill = EnvGuard::set("APEIRETH_DISABLE_TYPED_RECALL", Some("1"));
    let _g_db = EnvGuard::set("APEIRETH_COGNITIVE_DB", Some(&temp_db("w2-kill")));
    let _g_sdb = EnvGuard::set("APEIRETH_SESSION_DB", Some(&temp_db("w2-kill-session")));

    let (_runtime, _sessions, _memory, _policy, _guard_hook) =
        build_canonical_runtime_with_sessions_from_env()
            .await
            .expect("bootstrap with typed recall killed must succeed");
}
