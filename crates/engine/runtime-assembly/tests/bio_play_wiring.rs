//! 生物机制族 W3.3 · 游戏练习场工具面装配级 E2E (`tool.play_propose` /
//! `tool.play_start` / `tool.play_finish` / `tool.play_abort`)。
//!
//! 覆盖装配级契约 (不是 W2 内核单测的重复):
//! 1. env 开关 `APEIRETH_ENABLE_BIO_PLAY` 只认 `"1"`, 其余 fail-closed;
//! 2. 默认关: 四工具零注册 + `ModuleFamily::BioPlay` 零构造 (懒加载语义);
//! 3. env 开但缺工作区根: 不注册 (无法证明 `sandbox_dir` 包含性 = fail-closed);
//! 4. env 开 + 根注入: 恰好四个模块注册 (探针 4 次构造), 四能力齐;
//! 5. propose: 有提案 / 无提案 (结构化 null + 原因) 两态, 只提案不开场;
//! 6. start: 未显式 opt-in 拒绝 (`opt_in_required`), opt-in 后成功 + 注入时钟;
//! 7. sandbox_dir 越界 (绝对路径 / `..` / 根外 / symlink 逃逸) 拒绝即帧;
//! 8. finish 产出 `PlayOutcome` 且超预留拒绝; abort 幂等; 重复 session 拒绝;
//! 9. 全链路 (propose → start → finish) 工作区逐字节零变化, 沙箱目录不创建;
//! 10. 工具面源码扫描: 无主体记忆写入 / 无 shell / 无写文件调用.
//!
//! **0 装诚实**: env 默认关; 未 live E2E (无 gateway/CLI 手测); `sandbox_dir` 只是
//! 路径纪律标识, **不是 OS 级隔离**, 目录不创建; 账本只在内存 (重启丢 session);
//! 无 IM opt-in 卡 —— 工具面显式 `user_opt_in=true` 是唯一门槛; 内核零改动。

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use apeireth_core::clock::VirtualClock;
use apeireth_core::kernel::{Clock, Timestamp};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_runtime_assembly::canonical::{
    bio_play_enabled_from_env_value, bio_play_wiring_from_env_value, validate_play_sandbox_dir,
    BioPlayState, BIO_PLAY_ENABLE_ENV, PLAY_ABORT_CAPABILITY_ID, PLAY_ABORT_TOOL_NAME,
    PLAY_CAPABILITY_IDS, PLAY_FINISH_CAPABILITY_ID, PLAY_FINISH_TOOL_NAME,
    PLAY_PROPOSE_CAPABILITY_ID, PLAY_PROPOSE_TOOL_NAME, PLAY_START_CAPABILITY_ID,
    PLAY_START_TOOL_NAME,
};
use apeireth_runtime_assembly::{
    BuildInstrumentation, CountingBuildProbe, ModuleBuildProbe, ModuleFamily, ProductionBackends,
    ProductionModules, ProductionModulesConfig,
};
use serde_json::{json, Value};

/// 注入时钟的固定时刻 (测试永不读系统时钟; 内核 `now_ms` 由本钟现读)。
const NOW_MS: i64 = 1_700_000_000_123;

/// 进程 env 只在这三个装配测试里改; 全文件共用一把锁 (测试线程并行下仍确定)。
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn env_lock() -> MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// env 开关守卫: Drop 时清掉, panic 也不污染后续测试。
struct EnvSwitch;

impl EnvSwitch {
    fn set(value: &str) -> Self {
        std::env::set_var(BIO_PLAY_ENABLE_ENV, value);
        Self
    }

    fn unset() -> Self {
        std::env::remove_var(BIO_PLAY_ENABLE_ENV);
        Self
    }
}

impl Drop for EnvSwitch {
    fn drop(&mut self) {
        std::env::remove_var(BIO_PLAY_ENABLE_ENV);
    }
}

fn clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(NOW_MS)
            .expect("valid fixed timestamp")
            .as_datetime(),
    ))
}

fn workspace(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("apeireth-bio-play-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("创建测试工作区");
    dir
}

fn base_config() -> ProductionModulesConfig {
    ProductionModulesConfig {
        memory_recall: false,
        memory_writeback: false,
        preference_recall: false,
        self_assessment: false,
        ..ProductionModulesConfig::default()
    }
}

fn build(
    config: ProductionModulesConfig,
    workspace_root: Option<PathBuf>,
    probe: &Arc<CountingBuildProbe>,
) -> ProductionModules {
    let concrete = Arc::clone(probe);
    let probe: Arc<dyn ModuleBuildProbe> = concrete;
    let instrument = BuildInstrumentation::default().with_probe(probe);
    ProductionModules::build_instrumented(
        config,
        ProductionBackends {
            workspace_root,
            ..ProductionBackends::default()
        },
        clock(),
        &instrument,
    )
    .expect("装配必须成功")
}

fn find_capability<'a>(
    modules: &'a ProductionModules,
    id: &str,
) -> Option<&'a Arc<dyn ToolCapability>> {
    modules
        .capabilities()
        .iter()
        .find(|capability| capability.id().as_str() == id)
}

/// 一次装配的四件套 (共享同一本账本; 行为测试不依赖进程 env)。
struct PlayFace {
    state: Arc<BioPlayState>,
    propose: Arc<dyn ToolCapability>,
    start: Arc<dyn ToolCapability>,
    finish: Arc<dyn ToolCapability>,
    abort: Arc<dyn ToolCapability>,
}

fn face(root: &Path) -> PlayFace {
    let wiring = bio_play_wiring_from_env_value(Some("1"), Some(root), clock())
        .expect("env 开 + 根注入必须构造四件套");
    let state = Arc::clone(wiring.state());
    let capabilities: Vec<Arc<dyn ToolCapability>> = wiring
        .modules()
        .iter()
        .flat_map(|module| module.capabilities())
        .collect();
    assert_eq!(capabilities.len(), 4, "四个模块各贡献一个能力");
    PlayFace {
        state,
        propose: Arc::clone(&capabilities[0]),
        start: Arc::clone(&capabilities[1]),
        finish: Arc::clone(&capabilities[2]),
        abort: Arc::clone(&capabilities[3]),
    }
}

fn call(name: &str, arguments: Value) -> ToolCall {
    ToolCall {
        id: format!("call_{name}"),
        name: name.to_string(),
        arguments,
    }
}

async fn invoke_ok(tool: &Arc<dyn ToolCapability>, name: &str, arguments: Value) -> Value {
    let result = tool.invoke(&call(name, arguments)).await;
    assert!(result.is_ok(), "{name} 应成功, 实际: {}", result.render());
    serde_json::from_str(&result.render()).expect("输出应为结构化 JSON")
}

async fn invoke_err(tool: &Arc<dyn ToolCapability>, name: &str, arguments: Value) -> String {
    let result = tool.invoke(&call(name, arguments)).await;
    assert!(
        !result.is_ok(),
        "{name} 应拒绝即帧, 实际: {}",
        result.render()
    );
    result.render()
}

/// 一个默认 policy 下必然通过的提案入参 (2 项技能 = 5 000 tokens / 300 000 ms)。
fn candidates() -> Value {
    json!({
        "candidates": [{
            "topic": "预演一次捕猎路线",
            "skills": ["拆解", "复盘"],
            "risk": "sandboxed"
        }]
    })
}

async fn propose(face: &PlayFace) -> Value {
    let frame = invoke_ok(&face.propose, PLAY_PROPOSE_TOOL_NAME, candidates()).await;
    frame
        .get("proposal")
        .cloned()
        .expect("propose 帧必须有 proposal 字段")
}

fn start_args(
    proposal: &Value,
    session_id: &str,
    sandbox_dir: &str,
    user_opt_in: Option<bool>,
) -> Value {
    let mut arguments = json!({
        "session_id": session_id,
        "sandbox_dir": sandbox_dir,
        "proposal": proposal,
    });
    if let Some(user_opt_in) = user_opt_in {
        arguments["user_opt_in"] = json!(user_opt_in);
    }
    arguments
}

async fn start(
    face: &PlayFace,
    session_id: &str,
    sandbox_dir: &str,
    opt_in: Option<bool>,
) -> Value {
    let proposal = propose(face).await;
    let frame = invoke_ok(
        &face.start,
        PLAY_START_TOOL_NAME,
        start_args(&proposal, session_id, sandbox_dir, opt_in),
    )
    .await;
    frame
}

fn session_status(frame: &Value) -> &str {
    frame["session"]["status"]
        .as_str()
        .expect("session.status 字符串")
}

// ============================================
// ① env 开关 fail-closed
// ============================================

#[test]
fn the_env_switch_is_fail_closed_and_names_the_production_gate() {
    assert_eq!(BIO_PLAY_ENABLE_ENV, "APEIRETH_ENABLE_BIO_PLAY");
    assert!(!bio_play_enabled_from_env_value(None), "未设置 = 关");
    for raw in [
        "", " ", "0", "true", "TRUE", "yes", "on", "2", "01", "1.0", "-1", " 0 ",
    ] {
        assert!(
            !bio_play_enabled_from_env_value(Some(raw)),
            "{raw:?} 必须关 (只认 \"1\")"
        );
    }
    assert!(bio_play_enabled_from_env_value(Some("1")));
    assert!(bio_play_enabled_from_env_value(Some(" 1 ")));
}

// ============================================
// ② 默认关 = 不构造不注册 (懒加载语义)
// ============================================

#[test]
fn the_default_assembly_registers_no_play_tools_and_constructs_no_play_family() {
    let _lock = env_lock();
    let _env = EnvSwitch::unset();
    let root = workspace("default-off");
    let probe = CountingBuildProbe::new();

    let modules = build(base_config(), Some(root.clone()), &probe);

    assert_eq!(
        probe.count(ModuleFamily::BioPlay),
        0,
        "默认关 = 四族零构造 (探针有任何上报 = 懒加载被绕开)"
    );
    for id in PLAY_CAPABILITY_IDS {
        assert!(
            find_capability(&modules, id).is_none(),
            "默认关不得注册 {id}"
        );
    }
    // 装配其余部分不受影响 (对照能力在场, 证明不是整机空转)。
    assert!(
        find_capability(&modules, "tool.filesystem").is_some(),
        "既有工具族照常注册"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_env_switch_without_a_workspace_root_registers_nothing() {
    let _lock = env_lock();
    let _env = EnvSwitch::set("1");
    let probe = CountingBuildProbe::new();

    // 缺工作区根 = 无法证明 sandbox_dir 包含性 = 拒绝注册 (fail-closed, 不静默放行)。
    let modules = build(base_config(), None, &probe);

    assert_eq!(probe.count(ModuleFamily::BioPlay), 0);
    for id in PLAY_CAPABILITY_IDS {
        assert!(find_capability(&modules, id).is_none());
    }
}

#[test]
fn the_env_switch_registers_exactly_four_play_modules_with_a_root() {
    let _lock = env_lock();
    let _env = EnvSwitch::set("1");
    let root = workspace("env-on");
    let probe = CountingBuildProbe::new();

    let modules = build(base_config(), Some(root.clone()), &probe);

    assert_eq!(
        probe.count(ModuleFamily::BioPlay),
        4,
        "四个工具模块各构造一次"
    );
    for id in PLAY_CAPABILITY_IDS {
        assert!(
            find_capability(&modules, id).is_some(),
            "env 开 + 根注入必须注册 {id}"
        );
    }
    // 模型侧声明名可解析 (声明面完整, 不是只挂 id)。
    for (id, name) in [
        (PLAY_PROPOSE_CAPABILITY_ID, PLAY_PROPOSE_TOOL_NAME),
        (PLAY_START_CAPABILITY_ID, PLAY_START_TOOL_NAME),
        (PLAY_FINISH_CAPABILITY_ID, PLAY_FINISH_TOOL_NAME),
        (PLAY_ABORT_CAPABILITY_ID, PLAY_ABORT_TOOL_NAME),
    ] {
        let declaration = find_capability(&modules, id).expect("已注册").declaration();
        assert_eq!(declaration.name, name);
    }

    let _ = std::fs::remove_dir_all(&root);
}

// ============================================
// ③ propose: 两态 + 只提案不开场
// ============================================

#[tokio::test]
async fn propose_returns_a_full_proposal_and_starts_nothing() {
    let root = workspace("propose-ok");
    let face = face(&root);

    let frame = invoke_ok(&face.propose, PLAY_PROPOSE_TOOL_NAME, candidates()).await;
    let proposal = &frame["proposal"];
    assert_eq!(proposal["topic"], "预演一次捕猎路线");
    assert_eq!(proposal["skill_targets"], json!(["拆解", "复盘"]));
    assert_eq!(proposal["estimated_budget_tokens"], json!(5_000));
    assert_eq!(proposal["estimated_duration_ms"], json!(300_000));
    assert_eq!(proposal["needs_user_opt_in"], true);
    assert!(
        proposal["rationale"]
            .as_str()
            .is_some_and(|rationale| rationale.contains("opt-in")),
        "理由必须写明 opt-in 结论: {proposal:?}"
    );
    assert_eq!(frame["started"], false, "propose 绝不开场");
    assert_eq!(frame["memory_write"], false);
    assert_eq!(frame["policy"]["max_budget_tokens"], json!(20_000));
    assert_eq!(frame["policy"]["max_duration_ms"], json!(900_000));
    assert_eq!(frame["policy"]["allowed_risk"], "sandboxed");
    assert!(face.state.sessions().is_empty(), "提案不产生 session");
}

#[tokio::test]
async fn propose_returns_a_structured_null_when_policy_filters_every_candidate() {
    let root = workspace("propose-none");
    let face = face(&root);

    let mut many_skills = Vec::new();
    for index in 0..20 {
        many_skills.push(format!("技能{index}"));
    }
    let frame = invoke_ok(
        &face.propose,
        PLAY_PROPOSE_TOOL_NAME,
        json!({
            "candidates": [
                {"topic": "需要批准的真实环境练习", "skills": ["拆解"], "risk": "needs_approval"},
                {"topic": "超预算超时长", "skills": many_skills, "risk": "safe"}
            ]
        }),
    )
    .await;

    assert_eq!(frame["proposal"], Value::Null, "放不下必须返回 null");
    let reason = frame["reason"].as_str().expect("无提案必须给原因");
    assert!(reason.contains("no candidate passed"), "{reason}");
    assert!(reason.contains("nothing was started"), "{reason}");
    assert_eq!(frame["candidates_considered"], json!(2));
    assert!(face.state.sessions().is_empty());
    assert!(face.state.pending().is_empty());
}

#[tokio::test]
async fn propose_honors_caller_headroom_and_defaults_to_the_policy_ceiling() {
    let root = workspace("propose-headroom");
    let face = face(&root);

    // 缺省额度 = policy 上限 (20 000 / 900 000): 7 技能估算 12 500 / 900 000 压线通过。
    let seven_skills = json!(["观察", "拆解", "复述", "试错", "复盘", "迁移", "提问"]);
    let frame = invoke_ok(
        &face.propose,
        PLAY_PROPOSE_TOOL_NAME,
        json!({"candidates": [{"topic": "压线练习", "skills": seven_skills, "risk": "safe"}]}),
    )
    .await;
    assert_eq!(frame["proposal"]["estimated_budget_tokens"], json!(12_500));
    assert_eq!(frame["proposal"]["estimated_duration_ms"], json!(900_000));

    // 调用方额度低于 policy: 5 000 / 300 000 放不进 4 999 tokens → null。
    let frame = invoke_ok(
        &face.propose,
        PLAY_PROPOSE_TOOL_NAME,
        json!({
            "candidates": [{"topic": "预演", "skills": ["拆解", "复盘"], "risk": "safe"}],
            "budget_available_tokens": 4_999
        }),
    )
    .await;
    assert_eq!(frame["proposal"], Value::Null);

    let frame = invoke_ok(
        &face.propose,
        PLAY_PROPOSE_TOOL_NAME,
        json!({
            "candidates": [{"topic": "预演", "skills": ["拆解", "复盘"], "risk": "safe"}],
            "duration_available_ms": 299_999
        }),
    )
    .await;
    assert_eq!(frame["proposal"], Value::Null);

    // 刚好放下 → 有提案。
    let frame = invoke_ok(
        &face.propose,
        PLAY_PROPOSE_TOOL_NAME,
        json!({
            "candidates": [{"topic": "预演", "skills": ["拆解", "复盘"], "risk": "safe"}],
            "budget_available_tokens": 5_000,
            "duration_available_ms": 300_000
        }),
    )
    .await;
    assert_eq!(frame["proposal"]["estimated_budget_tokens"], json!(5_000));
}

#[tokio::test]
async fn propose_refuses_malformed_or_unbounded_input() {
    let root = workspace("propose-invalid");
    let face = face(&root);

    // 空候选 / 未知 risk 词 / 空主题 一律拒绝即帧 (不猜)。
    for invalid in [
        json!({"candidates": []}),
        json!({"candidates": [{"topic": "t", "skills": ["a"], "risk": "extreme"}]}),
        json!({"candidates": [{"topic": "  ", "skills": ["a"], "risk": "safe"}]}),
        json!({"candidates": [], "unknown_field": 1}),
    ] {
        let message = invoke_err(&face.propose, PLAY_PROPOSE_TOOL_NAME, invalid).await;
        assert!(message.starts_with("error: "), "{message}");
    }
    assert!(face.state.sessions().is_empty());
}

// ============================================
// ④ start: opt-in 唯一门槛 / 注入时钟 / 重复拒绝
// ============================================

#[tokio::test]
async fn start_without_an_explicit_opt_in_is_refused() {
    let root = workspace("start-no-optin");
    let face = face(&root);
    let proposal = propose(&face).await;

    // 缺字段 / false / null 一律 `opt_in_required`; 不留下半启动 session。
    for opt_in in [None, Some(false)] {
        let message = invoke_err(
            &face.start,
            PLAY_START_TOOL_NAME,
            start_args(&proposal, "s-gated", "sandbox/play-1", opt_in),
        )
        .await;
        assert!(message.contains("code=opt_in_required"), "{message}");
    }
    let message = invoke_err(
        &face.start,
        PLAY_START_TOOL_NAME,
        json!({
            "session_id": "s-gated",
            "sandbox_dir": "sandbox/play-1",
            "user_opt_in": null,
            "proposal": proposal,
        }),
    )
    .await;
    assert!(message.contains("code=opt_in_required"), "{message}");
    assert!(
        face.state.sessions().is_empty(),
        "被拒绝的启动不得留下 session"
    );
}

#[tokio::test]
async fn start_after_opt_in_records_a_running_session_with_the_injected_clock() {
    let root = workspace("start-ok");
    let face = face(&root);

    let frame = start(&face, "s-ok", "sandbox/play-1", Some(true)).await;
    assert_eq!(session_status(&frame), "running");
    assert_eq!(frame["session"]["id"], "s-ok");
    assert_eq!(frame["session"]["topic"], "预演一次捕猎路线");
    assert_eq!(frame["session"]["budget_tokens"], json!(5_000));
    assert_eq!(
        frame["session"]["started_ms"], NOW_MS,
        "now_ms 必须来自注入时钟"
    );
    assert_eq!(frame["session"]["outcome"], Value::Null);
    assert_eq!(frame["sandbox_dir"], "sandbox/play-1");
    assert_eq!(frame["user_opt_in"], true);
    assert_eq!(frame["persisted"], false, "内存账本, 不冒充持久");

    let sessions = face.state.sessions();
    assert_eq!(sessions.len(), 1);
    assert_eq!(
        sessions[0].started_ms,
        u64::try_from(NOW_MS).expect("fixed clock is positive")
    );
    assert_eq!(sessions[0].topic, "预演一次捕猎路线");
}

#[tokio::test]
async fn start_rejects_duplicate_session_ids() {
    let root = workspace("start-dup");
    let face = face(&root);
    start(&face, "s-dup", "sandbox/a", Some(true)).await;

    let proposal = propose(&face).await;
    let message = invoke_err(
        &face.start,
        PLAY_START_TOOL_NAME,
        start_args(&proposal, "s-dup", "sandbox/b", Some(true)),
    )
    .await;
    assert!(message.contains("code=duplicate_session"), "{message}");
    assert_eq!(face.state.sessions().len(), 1, "重复 id 不得新增 session");
}

// ============================================
// ⑤ sandbox_dir 越界拒绝
// ============================================

#[tokio::test]
async fn start_refuses_sandbox_dirs_that_escape_the_workspace() {
    let root = workspace("sandbox-refuse");
    let face = face(&root);
    let proposal = propose(&face).await;
    let absolute = root.join("inside-but-absolute");
    let absolute = absolute.to_string_lossy().to_string();

    for sandbox in [
        String::new(),
        "   ".to_string(),
        ".".to_string(),
        "..".to_string(),
        "../outside".to_string(),
        "sandbox/../../outside".to_string(),
        absolute,
    ] {
        let message = invoke_err(
            &face.start,
            PLAY_START_TOOL_NAME,
            start_args(&proposal, "s-escape", &sandbox, Some(true)),
        )
        .await;
        assert!(
            message.contains("sandbox_dir") || message.contains("workspace root"),
            "越界必须拒绝: {sandbox:?} → {message}"
        );
    }
    assert!(face.state.sessions().is_empty(), "越界拒绝不得开场");
    assert!(
        !root.join("outside").exists(),
        "拒绝路径不得被创建 (本面零落盘)"
    );
}

#[cfg(windows)]
fn symlink_dir(original: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(original, link)
}

#[cfg(unix)]
fn symlink_dir(original: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(original, link)
}

#[tokio::test]
async fn start_refuses_a_sandbox_whose_existing_ancestor_escapes_via_symlink() {
    let base = workspace("sandbox-symlink");
    let root = base.join("workspace");
    let outside = base.join("outside");
    std::fs::create_dir_all(&root).expect("create workspace");
    std::fs::create_dir_all(&outside).expect("create outside");
    let link = root.join("link");
    if symlink_dir(&outside, &link).is_err() {
        // 无符号链接权限 (Windows 非开发者模式) = 该平台测不了, 如实跳过并清场。
        let _ = std::fs::remove_dir_all(&base);
        return;
    }

    let face = face(&root);
    let proposal = propose(&face).await;
    for sandbox in ["link", "link/nested"] {
        let message = invoke_err(
            &face.start,
            PLAY_START_TOOL_NAME,
            start_args(&proposal, "s-link", sandbox, Some(true)),
        )
        .await;
        // 拒绝可能发生在两层：play 自身校验（英文 outside the workspace root），
        // 或更早的 pre-call 通用守卫（guardrail::OutsideWorkspace，中文「超出工作区边界」）。
        // 断言只钉「必须被拒绝」，不钉具体层与措辞 —— 否则 Windows 上因无符号链接权限
        // 被静默跳过，Linux/CI 才暴露（2026-10-22 并线发现：基座既有红）。
        assert!(
            message.contains("outside the workspace root") || message.contains("超出工作区边界"),
            "symlink 逃逸必须拒绝: {message}"
        );
    }
    assert!(face.state.sessions().is_empty());

    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn the_sandbox_validator_accepts_nested_relative_dirs_without_creating_them() {
    let root = workspace("sandbox-accept");
    assert_eq!(
        validate_play_sandbox_dir(&root, "sandbox/play-1").expect("合法相对路径"),
        "sandbox/play-1"
    );
    assert_eq!(
        validate_play_sandbox_dir(&root, "./sandbox").expect("./ 规范化"),
        "sandbox"
    );
    assert!(
        !root.join("sandbox").exists(),
        "路径纪律只校验不创建目录 (沙箱 = 受控目录标识, 非 OS 隔离)"
    );
    // 工作区根不存在 = 无法做包含性证明 = 拒绝 (fail-closed, 不猜)。
    assert!(
        validate_play_sandbox_dir(&root.join("missing-root"), "sandbox").is_err(),
        "缺工作区根必须拒绝"
    );
    let _ = std::fs::remove_dir_all(&root);
}

// ============================================
// ⑥ finish / abort 状态机
// ============================================

#[tokio::test]
async fn finish_records_the_outcome_and_guards_the_reserved_budget() {
    let root = workspace("finish");
    let face = face(&root);
    start(&face, "s-finish", "sandbox/play-1", Some(true)).await;

    // 超预留: 拒绝即帧, session 保持 running, 不产生半成品终态。
    let message = invoke_err(
        &face.finish,
        PLAY_FINISH_TOOL_NAME,
        json!({"session_id": "s-finish", "budget_used": 5_001}),
    )
    .await;
    assert!(
        message.contains("code=budget_used_exceeds_reserved"),
        "{message}"
    );
    assert_eq!(
        face.state.sessions()[0].status,
        apeireth_organ::play::PlayStatus::Running
    );

    let frame = invoke_ok(
        &face.finish,
        PLAY_FINISH_TOOL_NAME,
        json!({
            "session_id": "s-finish",
            "learned": ["拆解"],
            "artifacts": ["sandbox/play-1/demo.txt"],
            "budget_used": 1_234
        }),
    )
    .await;
    assert_eq!(session_status(&frame), "finished");
    assert_eq!(frame["outcome"]["learned"], json!(["拆解"]));
    assert_eq!(
        frame["outcome"]["artifacts"],
        json!(["sandbox/play-1/demo.txt"])
    );
    assert_eq!(frame["outcome"]["budget_used"], json!(1_234));
    assert_eq!(frame["memory_write"], false, "产出不写主体记忆");
    assert_eq!(frame["persisted"], false);

    let sessions = face.state.sessions();
    assert_eq!(sessions.len(), 1);
    assert_eq!(
        sessions[0].status,
        apeireth_organ::play::PlayStatus::Finished
    );
    assert_eq!(
        sessions[0].outcome.as_ref().expect("应有产出").budget_used,
        1_234
    );

    // 终态不可复活。
    let message = invoke_err(
        &face.finish,
        PLAY_FINISH_TOOL_NAME,
        json!({"session_id": "s-finish"}),
    )
    .await;
    assert!(message.contains("code=invalid_transition"), "{message}");
}

#[tokio::test]
async fn abort_is_idempotent_and_finish_after_abort_is_refused() {
    let root = workspace("abort");
    let face = face(&root);
    start(&face, "s-abort", "sandbox/play-1", Some(true)).await;

    let first = invoke_ok(
        &face.abort,
        PLAY_ABORT_TOOL_NAME,
        json!({"session_id": "s-abort"}),
    )
    .await;
    assert_eq!(session_status(&first), "aborted");
    assert_eq!(first["aborted"], true);
    assert_eq!(first["idempotent_replay"], false);
    assert_eq!(first["session"]["outcome"], Value::Null);

    let second = invoke_ok(
        &face.abort,
        PLAY_ABORT_TOOL_NAME,
        json!({"session_id": "s-abort"}),
    )
    .await;
    assert_eq!(second["idempotent_replay"], true, "重复 abort 必须幂等");
    assert_eq!(
        second["session"], first["session"],
        "幂等重放不得改变任何字段"
    );
    assert_eq!(face.state.sessions().len(), 1);

    let message = invoke_err(
        &face.finish,
        PLAY_FINISH_TOOL_NAME,
        json!({"session_id": "s-abort"}),
    )
    .await;
    assert!(message.contains("code=invalid_transition"), "{message}");

    let message = invoke_err(
        &face.abort,
        PLAY_ABORT_TOOL_NAME,
        json!({"session_id": "missing"}),
    )
    .await;
    assert!(message.contains("code=session_not_found"), "{message}");

    let message = invoke_err(
        &face.finish,
        PLAY_FINISH_TOOL_NAME,
        json!({"session_id": "missing"}),
    )
    .await;
    assert!(message.contains("code=session_not_found"), "{message}");
}

// ============================================
// ⑦ 全链路零落盘 + 共享账本
// ============================================

fn snapshot_dir(root: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        for entry in std::fs::read_dir(dir).expect("读目录") {
            let entry = entry.expect("目录项");
            let path = entry.path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                let relative = path
                    .strip_prefix(base)
                    .expect("相对路径")
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push((relative, std::fs::read(&path).expect("读文件")));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

#[tokio::test]
async fn the_four_tools_share_one_ledger_and_leave_the_workspace_untouched() {
    let root = workspace("e2e");
    std::fs::write(root.join("a.rs"), "fn a() {}\n").expect("write a.rs");
    std::fs::write(root.join("b.md"), "# b\n").expect("write b.md");
    let before = snapshot_dir(&root);
    let face = face(&root);

    let frame = invoke_ok(&face.propose, PLAY_PROPOSE_TOOL_NAME, candidates()).await;
    let proposal = frame["proposal"].clone();
    let started = invoke_ok(
        &face.start,
        PLAY_START_TOOL_NAME,
        start_args(&proposal, "s-e2e", "sandbox/play-1", Some(true)),
    )
    .await;
    assert_eq!(session_status(&started), "running");
    let finished = invoke_ok(
        &face.finish,
        PLAY_FINISH_TOOL_NAME,
        json!({
            "session_id": "s-e2e",
            "learned": ["拆解"],
            "artifacts": ["sandbox/play-1/demo.txt"],
            "budget_used": 4_321
        }),
    )
    .await;
    assert_eq!(session_status(&finished), "finished");

    // 跨能力共享同一本账本: propose 的提案 → start 开场 → finish 结束都落在同一账。
    assert_eq!(face.state.sessions().len(), 1);
    assert_eq!(
        face.state.sessions()[0].status,
        apeireth_organ::play::PlayStatus::Finished
    );
    assert!(face.state.pending().is_empty());

    // 工作区逐字节零变化 + 沙箱目录不创建 (产物只是元数据字符串)。
    assert_eq!(snapshot_dir(&root), before, "play 四件套不得落盘");
    assert!(
        !root.join("sandbox").exists(),
        "sandbox_dir 只是受控目录标识, 不得创建"
    );

    let _ = std::fs::remove_dir_all(&root);
}

// ============================================
// ⑧ 工具面源码扫描: 无记忆 / shell / 写文件面
// ============================================

#[test]
fn the_play_tool_surface_has_no_memory_shell_or_file_write_path() {
    // 结构断言: 工具面源码里不存在主体记忆写入 / shell / 写文件调用 (非运行时
    // 反射可用, 与内核 play.rs 的源码扫描同模式)。只扫生产段: `#[cfg(test)]`
    // 之后的单元测试用 `std::fs::write` 造 fixture, 不属于工具面行为。
    let source = include_str!("../src/canonical/play_tools.rs");
    let production = source.split("#[cfg(test)]").next().expect("源码必有生产段");
    for forbidden in [
        "fn remember",
        "write_memory",
        "MemoryStore",
        "EpisodeStore",
        "save_episode",
        "memory_store",
        "episode_store",
        "fs::write",
        "fs::create_dir",
        "fs::remove_file",
        "fs::remove_dir",
        "File::create",
        "OpenOptions",
        "write_all",
        "Command::new",
        "std::process",
    ] {
        assert!(
            !production.contains(forbidden),
            "play 工具面不得出现 {forbidden:?} (无主体记忆写入 / 无 shell / 无写文件面)"
        );
    }
    // 闭合词表: 四个能力都在 tool.play_ 命名空间下, 不借用既有写/执行面。
    for id in PLAY_CAPABILITY_IDS {
        assert!(id.starts_with("tool.play_"), "{id}");
    }
    assert_eq!(PLAY_CAPABILITY_IDS.len(), 4);
}
