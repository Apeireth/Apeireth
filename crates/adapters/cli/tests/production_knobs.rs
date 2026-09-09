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

    let (runtime, _sessions, _memory, policy) = build_canonical_runtime_with_sessions_from_env()
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

    let (runtime, _sessions, _memory, policy) = build_canonical_runtime_with_sessions_from_env()
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

    let (runtime, _sessions, _memory, _policy) = build_canonical_runtime_with_sessions_from_env()
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
