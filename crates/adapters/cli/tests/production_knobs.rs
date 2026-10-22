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

/// organs + preference_learning 旋钮: 模块装配进 runtime (organs 默认关的对照
/// 由 production_slot_order_is_explicit 测试锚定; preference_learning 属记忆
/// 核心族默认开, `=1` 显式开等价于默认态)。
#[tokio::test]
async fn organs_and_preference_learning_knobs_register_modules() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _g_organs = EnvGuard::set("APEIRETH_ENABLE_ORGANS", Some("1"));
    let _g_pl = EnvGuard::set("APEIRETH_ENABLE_PREFERENCE_LEARNING", Some("1"));
    let _g_pl_off = EnvGuard::set("APEIRETH_DISABLE_PREFERENCE_LEARNING", None);
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

/// proactive recall 旋钮 (记忆核心族, 默认开): 未设=开; `=0` / DISABLE 关
/// (DISABLE 优先); `=1` 显式开给 enabled 策略 (已接线的
/// `compile_prompt_overlay_with_proactive_access` 路径由此可达)。
#[test]
fn proactive_recall_knob_defaults_on_with_escape_hatch() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _g_enable = EnvGuard::set("APEIRETH_ENABLE_PROACTIVE_RECALL", None);
    let _g_disable = EnvGuard::set("APEIRETH_DISABLE_PROACTIVE_RECALL", None);
    let policy = apeireth_cli::proactive_recall_policy_from_env()
        .expect("default (unset) must be on for the memory-core family");
    assert!(policy.enabled, "policy must be enabled");
    assert_eq!(policy.budget, 2, "deterministic default budget");

    let _g_zero = EnvGuard::set("APEIRETH_ENABLE_PROACTIVE_RECALL", Some("0"));
    assert!(
        apeireth_cli::proactive_recall_policy_from_env().is_none(),
        "=0 must turn the knob off"
    );

    let _g_one = EnvGuard::set("APEIRETH_ENABLE_PROACTIVE_RECALL", Some("1"));
    let policy =
        apeireth_cli::proactive_recall_policy_from_env().expect("knob =1 must produce a policy");
    assert!(policy.enabled, "policy must be enabled");
    assert_eq!(policy.budget, 2, "deterministic default budget");

    let _g_disable_on = EnvGuard::set("APEIRETH_DISABLE_PROACTIVE_RECALL", Some("1"));
    assert!(
        apeireth_cli::proactive_recall_policy_from_env().is_none(),
        "DISABLE must win over ENABLE=1"
    );
}

/// 记忆核心族默认语义 (preference_learning / memory_injection): 未设=开、
/// `=0` 关、`APEIRETH_DISABLE_*=1` 关且优先于 `=1`。
#[test]
fn memory_core_knobs_default_on_and_disable_wins() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _g_pl = EnvGuard::set("APEIRETH_ENABLE_PREFERENCE_LEARNING", None);
    let _g_pl_off = EnvGuard::set("APEIRETH_DISABLE_PREFERENCE_LEARNING", None);
    let _g_inj = EnvGuard::set("APEIRETH_ENABLE_MEMORY_INJECTION", None);
    let _g_inj_off = EnvGuard::set("APEIRETH_DISABLE_MEMORY_INJECTION", None);

    assert!(
        apeireth_cli::preference_learning_enabled_from_env(),
        "preference learning defaults on (unset = on)"
    );
    assert!(
        apeireth_cli::memory_injection_enabled_from_env(),
        "memory injection defaults on (unset = on)"
    );

    let _g_pl_zero = EnvGuard::set("APEIRETH_ENABLE_PREFERENCE_LEARNING", Some("0"));
    let _g_inj_zero = EnvGuard::set("APEIRETH_ENABLE_MEMORY_INJECTION", Some("0"));
    assert!(
        !apeireth_cli::preference_learning_enabled_from_env(),
        "=0 off"
    );
    assert!(!apeireth_cli::memory_injection_enabled_from_env(), "=0 off");

    let _g_pl_one = EnvGuard::set("APEIRETH_ENABLE_PREFERENCE_LEARNING", Some("1"));
    let _g_inj_one = EnvGuard::set("APEIRETH_ENABLE_MEMORY_INJECTION", Some("1"));
    assert!(
        apeireth_cli::preference_learning_enabled_from_env(),
        "=1 on"
    );
    assert!(apeireth_cli::memory_injection_enabled_from_env(), "=1 on");

    let _g_pl_kill = EnvGuard::set("APEIRETH_DISABLE_PREFERENCE_LEARNING", Some("1"));
    let _g_inj_kill = EnvGuard::set("APEIRETH_DISABLE_MEMORY_INJECTION", Some("1"));
    assert!(
        !apeireth_cli::preference_learning_enabled_from_env(),
        "DISABLE must win over ENABLE=1"
    );
    assert!(
        !apeireth_cli::memory_injection_enabled_from_env(),
        "DISABLE must win over ENABLE=1"
    );
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

/// W2b 记忆闭环旋钮: memory_injection 属记忆核心族**默认开**;
/// consolidation / reflexion 默认关 (对照不变), reflexion 开 =
/// `cognitive.reflexion` 模块注册 (效果级证据: assembly 的 consolidation/reflexion
/// 测试 + memory 的注入格式测试)。
#[tokio::test]
async fn memory_loop_knobs_register_reflexion_module() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _g_inj = EnvGuard::set("APEIRETH_ENABLE_MEMORY_INJECTION", None);
    let _g_inj_off = EnvGuard::set("APEIRETH_DISABLE_MEMORY_INJECTION", None);
    let _g_con = EnvGuard::set("APEIRETH_ENABLE_CONSOLIDATION", None);
    let _g_ref = EnvGuard::set("APEIRETH_ENABLE_REFLEXION", None);
    assert!(apeireth_cli::memory_injection_enabled_from_env());
    assert!(!apeireth_cli::consolidation_enabled_from_env());
    assert!(!apeireth_cli::reflexion_enabled_from_env());

    let reflex_dir = std::env::temp_dir()
        .join(format!("apeireth-reflex-{}", std::process::id()))
        .to_string_lossy()
        .into_owned();
    let _g_ref_on = EnvGuard::set("APEIRETH_ENABLE_REFLEXION", Some("1"));
    let _g_dir = EnvGuard::set("APEIRETH_REFLEXION_DIR", Some(&reflex_dir));
    let _g_db = EnvGuard::set("APEIRETH_COGNITIVE_DB", Some(&temp_db("reflex")));
    let _g_sdb = EnvGuard::set("APEIRETH_SESSION_DB", Some(&temp_db("reflex-session")));

    let (runtime, _sessions, _memory, _policy, _guard_hook) =
        build_canonical_runtime_with_sessions_from_env()
            .await
            .expect("bootstrap with reflexion knob");
    let ids = module_ids(&runtime);
    assert!(
        ids.contains(&"cognitive.reflexion".to_string()),
        "reflexion module must be wired, got {ids:?}"
    );
    assert!(apeireth_cli::reflexion_enabled_from_env());
}

/// 「推荐配置」一键预设映射一致: 六个 env 与 frontend types.ts
/// `RECOMMENDED_CAPABILITY_PRESET` 同名镜像 —— 齐开时三个记忆核心旋钮解析全开
/// (proactive 策略 / preference_learning / memory_injection) + consolidation /
/// reflexion 开 + organs / preference_learning / reflexion 模块装配, 且**绝不**
/// 触及 shell/fetch (危险项不进推荐配置)。
#[tokio::test]
async fn recommended_preset_maps_to_core_memory_knobs() {
    const RECOMMENDED_PRESET_ENVS: &[&str] = &[
        "APEIRETH_ENABLE_PROACTIVE_RECALL",
        "APEIRETH_ENABLE_PREFERENCE_LEARNING",
        "APEIRETH_ENABLE_MEMORY_INJECTION",
        "APEIRETH_ENABLE_CONSOLIDATION",
        "APEIRETH_ENABLE_REFLEXION",
        "APEIRETH_ENABLE_ORGANS",
    ];
    let _lock = ENV_LOCK.lock().unwrap();
    let _guards: Vec<EnvGuard> = RECOMMENDED_PRESET_ENVS
        .iter()
        .map(|&key| EnvGuard::set(key, Some("1")))
        .collect();
    let _g_pr_off = EnvGuard::set("APEIRETH_DISABLE_PROACTIVE_RECALL", None);
    let _g_pl_off = EnvGuard::set("APEIRETH_DISABLE_PREFERENCE_LEARNING", None);
    let _g_inj_off = EnvGuard::set("APEIRETH_DISABLE_MEMORY_INJECTION", None);
    let _g_shell = EnvGuard::set("APEIRETH_ENABLE_SHELL", None);
    let _g_fetch = EnvGuard::set("APEIRETH_ENABLE_FETCH", None);
    let reflex_dir = std::env::temp_dir()
        .join(format!("apeireth-preset-reflex-{}", std::process::id()))
        .to_string_lossy()
        .into_owned();
    let _g_dir = EnvGuard::set("APEIRETH_REFLEXION_DIR", Some(&reflex_dir));
    let _g_db = EnvGuard::set("APEIRETH_COGNITIVE_DB", Some(&temp_db("preset")));
    let _g_sdb = EnvGuard::set("APEIRETH_SESSION_DB", Some(&temp_db("preset-session")));

    // 三个记忆核心旋钮与预设映射一致 (同一批 env 名驱动同一批解析函数)。
    assert!(apeireth_cli::proactive_recall_policy_from_env().is_some());
    assert!(apeireth_cli::preference_learning_enabled_from_env());
    assert!(apeireth_cli::memory_injection_enabled_from_env());
    assert!(apeireth_cli::consolidation_enabled_from_env());
    assert!(apeireth_cli::reflexion_enabled_from_env());

    let (runtime, _sessions, _memory, _policy, _guard_hook) =
        build_canonical_runtime_with_sessions_from_env()
            .await
            .expect("bootstrap with the recommended preset must succeed");
    let ids = module_ids(&runtime);
    for expected in [
        "cognitive.organs",
        "cognitive.preference_learning",
        "cognitive.reflexion",
    ] {
        assert!(
            ids.contains(&expected.to_string()),
            "{expected} must be wired under the recommended preset, got {ids:?}"
        );
    }

    // 推荐配置绝不包含 shell/fetch: 旋钮未设则工具不注册。
    let tool_ids: Vec<String> = runtime
        .tools()
        .iter()
        .map(|t| t.id().as_str().to_string())
        .collect();
    assert!(
        !tool_ids.contains(&"tool.shell".to_string()),
        "{tool_ids:?}"
    );
    assert!(
        !tool_ids.contains(&"tool.fetch".to_string()),
        "{tool_ids:?}"
    );
}

// ---- 上下文预算旋钮 (APEIRETH_CONTEXT_BUDGET_CHARS) ----

/// 上下文预算旋钮: `APEIRETH_CONTEXT_BUDGET_CHARS` 设小值可触发注入上下文截断
/// (核心块身份/系统约定/安全指令永不截断, 非核心长尾先砍); 未设 = 合理默认。
#[test]
fn context_budget_knob_triggers_truncation_when_small() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _g = EnvGuard::set("APEIRETH_CONTEXT_BUDGET_CHARS", Some("300"));

    let budget = apeireth_cli::context_budget_chars_from_env();
    assert_eq!(budget, 300, "knob must resolve the env value");

    // A small budget truncates the long-tail block while the core survives.
    let core =
        apeireth_orchestration::context_budget::ContextBlock::new("identity", "I".repeat(80))
            .core(true);
    let mem = apeireth_orchestration::context_budget::ContextBlock::new("memory", "M".repeat(400));
    let out = apeireth_runtime::canonical::budget_context_blocks(vec![core.clone(), mem], budget);

    let out_core = out.iter().find(|b| b.name == "identity").unwrap();
    assert_eq!(out_core.content, core.content, "core never truncated");
    let out_mem = out.iter().find(|b| b.name == "memory").unwrap();
    assert!(
        out_mem.content.chars().count() < 400,
        "long-tail block must be truncated by the small budget"
    );
}

/// 上下文预算旋钮: 未设 / 非法值 = 合理默认 (`DEFAULT_CONTEXT_BUDGET_CHARS`)。
#[test]
fn context_budget_knob_defaults_when_unset() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _g = EnvGuard::set("APEIRETH_CONTEXT_BUDGET_CHARS", None);
    assert_eq!(
        apeireth_cli::context_budget_chars_from_env(),
        apeireth_runtime::canonical::DEFAULT_CONTEXT_BUDGET_CHARS
    );

    let _g_bad = EnvGuard::set("APEIRETH_CONTEXT_BUDGET_CHARS", Some("not-a-number"));
    assert_eq!(
        apeireth_cli::context_budget_chars_from_env(),
        apeireth_runtime::canonical::DEFAULT_CONTEXT_BUDGET_CHARS,
        "invalid values fall back to the default"
    );
}

// ---- 轮数 / 单轮工具调用预算旋钮 (APEIRETH_MAX_TURN_ROUNDS / APEIRETH_MAX_TOOL_CALLS) ----

/// 预算显式化: 两个预算 env 旋钮装配进 runtime (调小轮数可触发真实上限帧)。
#[tokio::test]
async fn turn_budget_knobs_configure_the_runtime_from_env() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _g_rounds = EnvGuard::set("APEIRETH_MAX_TURN_ROUNDS", Some("2"));
    let _g_calls = EnvGuard::set("APEIRETH_MAX_TOOL_CALLS", Some("4"));
    let _g_db = EnvGuard::set("APEIRETH_COGNITIVE_DB", Some(&temp_db("turn-budget")));
    let _g_sdb = EnvGuard::set("APEIRETH_SESSION_DB", Some(&temp_db("turn-budget-session")));

    assert_eq!(apeireth_cli::turn_round_limit_from_env(), 2);
    assert_eq!(apeireth_cli::tool_call_limit_from_env(), 4);

    let (runtime, _sessions, _memory, _policy, _guard_hook) =
        build_canonical_runtime_with_sessions_from_env()
            .await
            .expect("bootstrap with budget knobs");
    assert_eq!(runtime.config().max_rounds, 2, "轮数预算 env 生效");
    assert_eq!(
        runtime.config().max_tool_calls_per_round,
        4,
        "单轮工具调用预算 env 生效"
    );
}

/// 预算旋钮取值纪律: 越界钳制、非法值回默认 (8 / 16)。
#[test]
fn turn_budget_knobs_clamp_and_fall_back_to_defaults() {
    let _lock = ENV_LOCK.lock().unwrap();

    let _g_bad = EnvGuard::set("APEIRETH_MAX_TURN_ROUNDS", Some("not-a-number"));
    let _g_bad2 = EnvGuard::set("APEIRETH_MAX_TOOL_CALLS", Some(""));
    assert_eq!(apeireth_cli::turn_round_limit_from_env(), 8);
    assert_eq!(apeireth_cli::tool_call_limit_from_env(), 16);

    let _g_big = EnvGuard::set("APEIRETH_MAX_TURN_ROUNDS", Some("999"));
    let _g_zero = EnvGuard::set("APEIRETH_MAX_TOOL_CALLS", Some("0"));
    assert_eq!(apeireth_cli::turn_round_limit_from_env(), 64);
    assert_eq!(apeireth_cli::tool_call_limit_from_env(), 1);

    let _g_none = EnvGuard::set("APEIRETH_MAX_TURN_ROUNDS", None);
    let _g_none2 = EnvGuard::set("APEIRETH_MAX_TOOL_CALLS", None);
    assert_eq!(apeireth_cli::turn_round_limit_from_env(), 8);
    assert_eq!(apeireth_cli::tool_call_limit_from_env(), 16);
}

// ---- 会话配额旋钮 (APEIRETH_QUOTA_MAX_TOKENS / APEIRETH_QUOTA_MAX_TOOL_STEPS
//      / APEIRETH_QUOTA_MAX_COST_MICROS) ----

/// 会话配额旋钮 (opt-in 烧钱闸): 正整数直通, env → RuntimeConfig 同源装配。
#[tokio::test]
async fn session_quota_knobs_configure_the_runtime_from_env() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _g_tokens = EnvGuard::set("APEIRETH_QUOTA_MAX_TOKENS", Some("5000"));
    let _g_steps = EnvGuard::set("APEIRETH_QUOTA_MAX_TOOL_STEPS", Some("250"));
    let _g_cost = EnvGuard::set("APEIRETH_QUOTA_MAX_COST_MICROS", Some("700000"));
    let _g_db = EnvGuard::set("APEIRETH_COGNITIVE_DB", Some(&temp_db("quota-knobs")));
    let _g_sdb = EnvGuard::set("APEIRETH_SESSION_DB", Some(&temp_db("quota-knobs-session")));

    assert_eq!(apeireth_cli::quota_max_tokens_from_env(), Some(5_000));
    assert_eq!(apeireth_cli::quota_max_tool_steps_from_env(), Some(250));
    assert_eq!(
        apeireth_cli::quota_max_cost_micros_from_env(),
        Some(700_000),
        "花费旋钮同一解析口径"
    );

    let (runtime, _sessions, _memory, _policy, _guard_hook) =
        build_canonical_runtime_with_sessions_from_env()
            .await
            .expect("bootstrap with quota knobs");
    assert_eq!(
        runtime.config().quota.max_tokens,
        Some(5_000),
        "token 配额 env 生效"
    );
    assert_eq!(
        runtime.config().quota.max_tool_steps,
        Some(250),
        "步数配额 env 生效"
    );
    assert_eq!(
        runtime.config().quota.max_cost_micros,
        Some(700_000),
        "花费配额 env 生效"
    );
}

/// 会话配额旋钮取值纪律: 未设 / 0 / 非法 = 不设限 (None, opt-in 显式),
/// 越界钳制到计量列宽。
#[test]
fn session_quota_knobs_are_opt_in_and_clamp_oversized_values() {
    let _lock = ENV_LOCK.lock().unwrap();

    let _g_none = EnvGuard::set("APEIRETH_QUOTA_MAX_TOKENS", None);
    let _g_none2 = EnvGuard::set("APEIRETH_QUOTA_MAX_TOOL_STEPS", None);
    let _g_none3 = EnvGuard::set("APEIRETH_QUOTA_MAX_COST_MICROS", None);
    assert_eq!(apeireth_cli::quota_max_tokens_from_env(), None);
    assert_eq!(apeireth_cli::quota_max_tool_steps_from_env(), None);
    assert_eq!(apeireth_cli::quota_max_cost_micros_from_env(), None);

    let _g_zero = EnvGuard::set("APEIRETH_QUOTA_MAX_TOKENS", Some("0"));
    let _g_bad = EnvGuard::set("APEIRETH_QUOTA_MAX_TOOL_STEPS", Some("nope"));
    let _g_cost_zero = EnvGuard::set("APEIRETH_QUOTA_MAX_COST_MICROS", Some("0"));
    assert_eq!(
        apeireth_cli::quota_max_tokens_from_env(),
        None,
        "0 = 不设限 (显式)"
    );
    assert_eq!(apeireth_cli::quota_max_tool_steps_from_env(), None);
    assert_eq!(
        apeireth_cli::quota_max_cost_micros_from_env(),
        None,
        "花费旋钮同族纪律: 0 = 不设限"
    );

    let _g_big = EnvGuard::set("APEIRETH_QUOTA_MAX_TOKENS", Some("99999999999999"));
    let _g_cost_big = EnvGuard::set("APEIRETH_QUOTA_MAX_COST_MICROS", Some("99999999999999"));
    assert_eq!(
        apeireth_cli::quota_max_tokens_from_env(),
        Some(apeireth_runtime::canonical::MAX_QUOTA_TOKENS),
        "越界钳制"
    );
    assert_eq!(
        apeireth_cli::quota_max_cost_micros_from_env(),
        Some(apeireth_runtime::canonical::MAX_QUOTA_COST_MICROS),
        "越界钳制"
    );
}

/// H-P1 模型面: 生效模型 id 的唯一解析口径 (与运行时 `with_default_model`
/// 同源) —— `APEIRETH_MODEL` 显式覆盖胜出; 空白覆盖按未设处理, 回族首默认
/// 模型; 族模型表 env (`APEIRETH_API_MODELS`) 与解析同源。
#[test]
fn active_model_id_is_the_single_resolution_for_the_model_plane() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _g_model = EnvGuard::set("APEIRETH_MODEL", Some("explicit-model"));
    let _g_models = EnvGuard::set("APEIRETH_API_MODELS", None);
    assert_eq!(
        apeireth_cli::active_model_id().as_deref(),
        Some("explicit-model"),
        "显式覆盖胜出"
    );

    let _g_blank = EnvGuard::set("APEIRETH_MODEL", Some("  "));
    assert_eq!(
        apeireth_cli::active_model_id().as_deref(),
        Some("minimax-m3"),
        "空白覆盖按未设处理, 回族首默认模型 (提供商面规范 id, 小写)"
    );

    let _g_list = EnvGuard::set("APEIRETH_API_MODELS", Some("custom-a,custom-b"));
    assert_eq!(
        apeireth_cli::active_model_id().as_deref(),
        Some("custom-a"),
        "族模型表 env 同源 (首个)"
    );

    let _g_empty_list = EnvGuard::set("APEIRETH_API_MODELS", Some(" "));
    let _g_no_model = EnvGuard::set("APEIRETH_MODEL", None);
    assert_eq!(
        apeireth_cli::active_model_id().as_deref(),
        Some("minimax-m3"),
        "模型表 env 空白 = 族默认模型表"
    );
}

/// H-P1 配置面 fail-closed: 坏 `APEIRETH_MODEL_PROFILES` = 装配拒开
/// (启动错误正式通道), 不静默回退基线。
#[tokio::test]
async fn bootstrap_refuses_a_broken_model_profiles_env() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _g_profiles = EnvGuard::set("APEIRETH_MODEL_PROFILES", Some("{ not json"));
    let _g_db = EnvGuard::set("APEIRETH_COGNITIVE_DB", Some(&temp_db("model-profiles")));
    let _g_sdb = EnvGuard::set(
        "APEIRETH_SESSION_DB",
        Some(&temp_db("model-profiles-session")),
    );
    let Err(error) = build_canonical_runtime_with_sessions_from_env().await else {
        panic!("坏画像配置必须拒开");
    };
    assert!(
        error.contains("model profile configuration refused to load"),
        "{error}"
    );
}

/// H-P2 成本分级路由开关 fail-closed: 只有精确 `1` 开, 其他一律关
/// (garbage 绝不误开); 档位映射坏形状整体拒开 (值自由报错)。
#[test]
fn cost_routing_switch_fails_closed_and_the_tier_mapping_refuses_broken_shapes() {
    let _lock = ENV_LOCK.lock().unwrap();
    for off in ["0", "true", "yes", "2", ""] {
        let _g_off = EnvGuard::set("APEIRETH_COST_ROUTING", Some(off));
        assert!(
            !apeireth_cli::cost_routing_enabled_from_env(),
            "fail-closed: {off:?} 必须关"
        );
    }
    let _g_on = EnvGuard::set("APEIRETH_COST_ROUTING", Some("1"));
    assert!(apeireth_cli::cost_routing_enabled_from_env());

    // 档位映射: 未知档位词拒开 (词表闭合)。
    let _g_bad = EnvGuard::set(
        "APEIRETH_COST_ROUTING_PREFERRED",
        Some(r#"{"luxury": "model-x"}"#),
    );
    let error = apeireth_cli::cost_routing_config_from_env().expect_err("坏映射必须拒开");
    assert!(
        error.contains("cost routing configuration refused to load"),
        "{error}"
    );

    // 正常映射: 每档首选模型覆盖; 空白 = 无覆盖。
    let _g_ok = EnvGuard::set(
        "APEIRETH_COST_ROUTING_PREFERRED",
        Some(r#"{"frugal": "model-a"}"#),
    );
    let config = apeireth_cli::cost_routing_config_from_env().expect("mapping parses");
    assert!(config.enabled, "开关随 env 真值");
    assert_eq!(config.preferred_models.len(), 1, "一档一覆盖");
    let _g_blank = EnvGuard::set("APEIRETH_COST_ROUTING_PREFERRED", Some("  "));
    let config = apeireth_cli::cost_routing_config_from_env().expect("blank mapping");
    assert!(config.preferred_models.is_empty(), "空白 = 无覆盖");
}

/// H-P2 配置面 fail-closed: 坏 `APEIRETH_COST_ROUTING_PREFERRED` = 装配拒开
/// (启动错误正式通道), 不静默忽略。
#[tokio::test]
async fn bootstrap_refuses_a_broken_cost_routing_tiers_env() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _g_tiers = EnvGuard::set("APEIRETH_COST_ROUTING_PREFERRED", Some("{ not json"));
    let _g_db = EnvGuard::set("APEIRETH_COGNITIVE_DB", Some(&temp_db("cost-routing")));
    let _g_sdb = EnvGuard::set(
        "APEIRETH_SESSION_DB",
        Some(&temp_db("cost-routing-session")),
    );
    let Err(error) = build_canonical_runtime_with_sessions_from_env().await else {
        panic!("坏档位映射必须拒开");
    };
    assert!(
        error.contains("cost routing configuration refused to load"),
        "{error}"
    );
}

/// H-P2 回合模型解析: 显式 `--model` > 成本路由选定 > 默认模型; 开关关 =
/// fail-closed 零行为变化。
#[test]
fn resolve_turn_model_prefers_the_explicit_model_then_routing_then_the_default() {
    let _lock = ENV_LOCK.lock().unwrap();
    let data_dir =
        std::env::temp_dir().join(format!("apeireth-knob-cost-routing-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data_dir);
    std::fs::create_dir_all(&data_dir).expect("create test data dir");
    let data_dir = data_dir.to_string_lossy().into_owned();
    let _g_data_dir = EnvGuard::set("APEIRETH_DATA_DIR", Some(&data_dir));
    let _g_model = EnvGuard::set("APEIRETH_MODEL", Some("default-model"));
    let _g_profiles = EnvGuard::set(
        "APEIRETH_MODEL_PROFILES",
        Some(
            r#"[
                {"model_id": "default-model", "context_window": 32768, "supports_tools": true},
                {"model_id": "cheap-model", "context_window": 32768, "supports_tools": true,
                 "input_price_micros": 100, "output_price_micros": 100}
            ]"#,
        ),
    );
    let _g_preferred = EnvGuard::set("APEIRETH_COST_ROUTING_PREFERRED", None);
    let _g_routing_off = EnvGuard::set("APEIRETH_COST_ROUTING", None);

    let router = apeireth_cli::cost_router_from_env().expect("router assembles");
    assert_eq!(
        apeireth_cli::resolve_turn_model(None, &router, "hello"),
        None,
        "开关关 fail-closed: 不路由, 回合仍走默认模型"
    );
    assert_eq!(
        apeireth_cli::resolve_turn_model(Some("pinned".to_string()), &router, "hello").as_deref(),
        Some("pinned"),
        "显式 --model 最高"
    );

    let _g_routing_on = EnvGuard::set("APEIRETH_COST_ROUTING", Some("1"));
    let router = apeireth_cli::cost_router_from_env().expect("router assembles");
    assert_eq!(
        apeireth_cli::resolve_turn_model(None, &router, "hello").as_deref(),
        Some("cheap-model"),
        "开闸 + 有价格证据: 按省钱选"
    );
    assert_eq!(
        apeireth_cli::resolve_turn_model(Some("pinned".to_string()), &router, "hello").as_deref(),
        Some("pinned"),
        "显式 --model 仍胜出 (显式 > 自动)"
    );
    let _ = std::fs::remove_dir_all(&data_dir);
}
