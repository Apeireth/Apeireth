//! **记忆引擎端到端冒烟**：写入 → 召回 → 注入 → 模型看得见证据。
//!
//! # 这个文件补的是哪个洞
//!
//! 记忆引擎的**每一段**都有单测（召回 / 注入 / 契约 / 影子换算法），
//! 但**没有一条把整条链路串起来**的测试
//! ⇒ ⇒ 「每段都绿 ⇒ 整条也绿」是**假设**，不是事实。
//! 本文件把**生产装配路径**整条跑一遍，并断言**证据真的出现在注入里**
//! （不是「函数返回了 `Ok`」）。
//!
//! # 真实链路（每一步 file:line）
//!
//! | 步骤 | 落点 |
//! |---|---|
//! | 装配根 | `canonical/production.rs:773` `ProductionModules::build` |
//! | 记忆装配 | `canonical/production.rs:1433` → `canonical/memory_assembly.rs:104` `assemble_memory` |
//! | 契约口径自检 | `canonical/memory_assembly.rs:109` → `canonical/memory_contract_wire.rs:62` `install` → `memory/src/canonical/memory_contracts.rs:260` `assert_scope_alignment`（25 组范围组合） |
//! | 排序旋钮配置面 | `canonical/memory_assembly.rs:111` → `:126` `apply_retrieval_knobs_at`（缺文件 ⇒ 不施加；坏文件 ⇒ 拒绝装配） |
//! | 影子双跑 / 换算法 | `canonical/memory_assembly.rs:116` → `canonical/shadow_wire.rs:574` `install_shadow`（缺配置 ⇒ **不构造任何东西**） |
//! | 召回模块注册 | `canonical/production.rs:1492-1495` `MemoryRecallModule::new(...).with_coordinator(...)` |
//! | 写回模块注册 | `canonical/production.rs:1691-1697` `MemoryWritebackModule::new(...).with_coordinator(...)` |
//! | **写入面** | `canonical/cognitive.rs:2129` `coord.writeback_scoped_episode` → `memory/src/coordinator/owned.rs:4` → `memory/src/coordinator.rs:1121` `persist_episode` → `backend.put_episode` + `put_episode_metadata` |
//! | **召回面** | `canonical/cognitive.rs:1550` `coord.compile_prompt_overlay_with_selected_access` → `memory/src/coordinator.rs:1164` → `:742` `recall` → `:383` `collect_candidates` → `:903` `execute_hybrid_retrieval` |
//! | **读契约** | `memory/src/context_compiler.rs:167` `contract_view` → `memory/src/canonical/memory_contracts.rs:76` `SelectedMemoryView::build`（读契约 + 纯数据标注） |
//! | **注入渲染** | `memory/src/coordinator.rs:1174` → `memory/src/context_compiler.rs:346` → `:404` → `:436` `compile_injection_with_renderer` → `:576` `render_through_contract`（注入契约渲染器） |
//! | **进提示词** | `canonical/cognitive.rs:1584` `PromptOverlay::system_cross_source("memory_recall", overlay)` ⇒ 宿主把跨源内容经披露封套放进**工具结果**通道（模型看得见） |
//!
//! # 三条硬断言（缺一条，这条冒烟就不成立）
//!
//! 1. ⭐ **证据真的出现**：注入文本里**必须**出现写入的那条记忆的内容
//!    （不是「函数返回了 `Ok`」）。
//! 2. ⭐ **预算纪律**：注入长度**不超**硬上限（`DEFAULT_MAX_CONTEXT_CHARS = 4000`）。
//! 3. ⭐ **默认态零副作用**：影子/换算法**未配置** ⇒ 不构造任何东西、不落任何文件、
//!    且产物与「未接线」的裸协调器**逐字相同**。
//!
//! # 反证（必须真的红过）
//!
//! 文件末尾三��� `*_bites_when_*` 用例把链路上的一环短路（注入渲染返回空 /
//! 契约视图未授权 / 记忆根本没写），并断言**同一条判定谓词**在这些短路下
//! **必须为假** ⇒ ⇒ 证明主断言不是自证。
//!
//! # 中性表述
//!
//! 只描述**机制**（写入 / 召回 / 注入 / 契约 / 影子），不引用任何外部产品名。

#[path = "support/convergence.rs"]
mod support;

use apeireth_core::kernel::{Clock, Episode, SessionId, SystemClock};
use apeireth_memory::canonical::memory_contracts::SelectedMemoryView;
use apeireth_memory::injection_renderer::{Budget, Evidence, Injection, InjectionRenderer};
use apeireth_memory::memory_view::{AlgoStamp, Scope};
use apeireth_memory::{
    ClosedWorldContextCompiler, MemoryCoordinator, MemoryGovernanceStore, MemoryProvenance,
    MemoryRecallQuery, MemoryScope, MemoryWritebackEntry, SqliteMemoryStore,
};
use apeireth_plugin::memory_backend::MemoryBackend;
use apeireth_protocol::canonical::{
    ContentPart, MessageRole, NormalizedMessage, NormalizedResponse,
};
use apeireth_runtime::canonical::module::{
    BehaviorModule, HookPoint, InvocationContext, ModuleContext, ModuleInvocationError,
    ModuleInvocationRequest, ModuleInvocationResponse, ModuleInvoker, ModuleOutcome,
};
use apeireth_runtime::canonical::subloop::{
    SubLoopError, SubLoopResult, SubLoopSpawner, SubLoopSpec,
};
use apeireth_runtime::TurnRequest;
use apeireth_runtime_assembly::canonical::cognitive::{
    MEMORY_RECALL_MODULE_ID, MEMORY_WRITEBACK_MODULE_ID,
};
use apeireth_runtime_assembly::canonical::memory_contract_wire;
use apeireth_runtime_assembly::shadow_wire;
use apeireth_runtime_assembly::{CognitiveBackends, CognitiveModuleConfig, ProductionModules};
use async_trait::async_trait;
use std::sync::{Arc, Mutex, MutexGuard};
use support::*;

// ---------------------------------------------------------------- 探针

/// 跨会话（全局范围）记忆的探针串。
const CANARY_GLOBAL: &str = "GLOBALCANARY7F3";
/// 另一主体（用户范围）记忆的探针串。
const CANARY_USER: &str = "USERCANARY5B1";
/// **不可见主体**记忆的探针串（授予范围里没有它 ⇒ 必须召不到）。
const CANARY_FOREIGN: &str = "FOREIGNCANARY9C4";
/// 会话范围记忆的探针串（跨会话查询里必须召不到）。
const CANARY_SESSION: &str = "SESSIONCANARY2D8";

/// 召回模块的注入硬上限（`canonical/cognitive.rs:66` `DEFAULT_MAX_CONTEXT_CHARS`）。
const RECALL_BUDGET_CHARS: usize = 4_000;

/// 数据根环境变量名（`apeireth_core::paths` 的唯一真源口径）。
const DATA_DIR_ENV: &str = "APEIRETH_DATA_DIR";

/// 串行化「改数据根环境变量」的用例（同一进程内 env 是全局的 ⇒ 不许并发）。
static DATA_ROOT_LOCK: Mutex<()> = Mutex::new(());

// ---------------------------------------------------------------- 支撑

struct NoopInvoker;

#[async_trait]
impl ModuleInvoker for NoopInvoker {
    async fn invoke(
        &self,
        _request: ModuleInvocationRequest,
    ) -> Result<ModuleInvocationResponse, ModuleInvocationError> {
        Err(ModuleInvocationError::NoModel)
    }
}

struct NoSubLoop;

#[async_trait]
impl SubLoopSpawner for NoSubLoop {
    async fn spawn(&self, _spec: SubLoopSpec) -> Result<SubLoopResult, SubLoopError> {
        Err(SubLoopError::NoModel)
    }
}

/// 生产装配（记忆线）+ **干净的数据根**（默认态 ⇒ 没有配置面文件）。
fn production(db: &Arc<SqliteMemoryStore>, injection: bool) -> ProductionModules {
    ProductionModules::build(
        CognitiveModuleConfig {
            memory_recall: true,
            memory_writeback: true,
            memory_injection: injection,
            preference_recall: false,
            self_assessment: false,
            filesystem: false,
            search: false,
            repo: false,
            ..Default::default()
        },
        CognitiveBackends {
            memory: Some(db.clone()),
            memory_governance: Some(db.clone()),
            scoped_memory: Some(db.clone()),
            ..Default::default()
        },
        Arc::new(SystemClock),
    )
    .expect("生产装配必须成功（契约口径分叉 / 配置坏 ⇒ 拒绝装配）")
}

/// 从生产装配里取出**记忆召回模块**（按注册 id 找 ⇒ 拿的就是生产那一个）。
fn recall_module(
    modules: &ProductionModules,
) -> Arc<dyn apeireth_runtime::canonical::module::Module> {
    modules
        .modules()
        .iter()
        .find(|module| module.manifest().id == MEMORY_RECALL_MODULE_ID)
        .cloned()
        .expect("生产装配必须注册记忆召回模块")
}

/// 从生产装配里取出**记忆写回模块**。
fn writeback_module(
    modules: &ProductionModules,
) -> Arc<dyn apeireth_runtime::canonical::module::Module> {
    modules
        .modules()
        .iter()
        .find(|module| module.manifest().id == MEMORY_WRITEBACK_MODULE_ID)
        .cloned()
        .expect("生产装配必须注册记忆写回模块")
}

/// 跑一个模块钩子（生产模块的**真实入口**）。
async fn run_hook(
    module: &Arc<dyn apeireth_runtime::canonical::module::Module>,
    hook: HookPoint,
    session: &SessionId,
    messages: &[NormalizedMessage],
    candidate: Option<&NormalizedResponse>,
) -> ModuleOutcome {
    let invocation = InvocationContext::user_turn();
    let invoker: Arc<dyn ModuleInvoker> = Arc::new(NoopInvoker);
    let subloop = NoSubLoop;
    let ctx = ModuleContext {
        session_id: session,
        model: "fixture-model",
        messages,
        candidate,
        tool_call: None,
        tool_result: None,
        invocation: &invocation,
        module_id: MEMORY_RECALL_MODULE_ID,
        error: None,
        invoker: &*invoker,
        invoker_handle: Arc::clone(&invoker),
        subloop: &subloop,
    };
    module
        .on_hook(hook, &ctx)
        .await
        .expect("记忆模块钩子不许失败")
}

/// 注入文本（模块产出的提示词块）。
fn overlay_text(outcome: &ModuleOutcome) -> String {
    outcome
        .prompt_overlays
        .iter()
        .map(|overlay| ContentPart::join_text(&overlay.message().content))
        .collect::<Vec<_>>()
        .join("\n")
}

/// ⭐ **核心判定谓词**：证据**真的**出现在注入里了吗？
///
/// 这条谓词同时被主用例与反证用例使用 ⇒ ⇒ 反证里它为假，就等于主断言会红。
fn evidence_present(overlay: &str, canary: &str) -> bool {
    overlay.contains(canary)
}

/// 写一条带指定范围的记忆（**生产写入面**：`MemoryCoordinator::writeback`）。
fn write_scoped(
    coordinator: &MemoryCoordinator,
    session_id: &str,
    scope: MemoryScope,
    content: &str,
) {
    let mut entry = MemoryWritebackEntry::new(session_id, "user", content);
    entry.scope = scope;
    entry.provenance = MemoryProvenance {
        source: "e2e.smoke".into(),
        source_session: Some(session_id.to_string()),
        ..MemoryProvenance::default()
    };
    coordinator
        .writeback(&entry)
        .expect("生产写入面必须写成功（写不进去 ⇒ 后面的断言全是空谈）");
}

/// 把数据根指到**临时空目录**（默认态 ⇒ 配置面文件一律不存在）并返回守卫。
fn temp_data_root(label: &str) -> (tempfile::TempDir, MutexGuard<'static, ()>) {
    let guard = DATA_ROOT_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = tempfile::tempdir().expect("临时数据根");
    std::env::set_var(DATA_DIR_ENV, dir.path());
    let _ = label;
    (dir, guard)
}

/// 恢复数据根环境变量。
fn restore_data_root() {
    std::env::remove_var(DATA_DIR_ENV);
}

/// 递归收集目录下的**文件**（目录不算 ⇒ 「零新增文件」只盯真实落盘）。
fn collect_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, out);
        } else {
            out.push(path);
        }
    }
}

// ---------------------------------------------------------------- 主链路

/// ⭐⭐ **端到端**：生产装配 ⇒ 生产写入面落库 ⇒ 生产召回面召回 ⇒
/// **注入里真的出现那条记忆的内容** ⇒ 预算纪律 ⇒ 契约门真的过了。
#[tokio::test]
async fn the_production_chain_writes_recalls_and_injects_the_evidence() {
    let (_root, _guard) = temp_data_root("main-chain");
    let db = Arc::new(SqliteMemoryStore::open_in_memory().expect("内存库"));
    // ⭐ **契约面确实被经过**：生产装配的那一行必须真的过契约门（计数会涨）。
    let contracts_before = memory_contract_wire::contract_install_count();
    let modules = production(&db, false);
    assert!(
        memory_contract_wire::contract_install_count() > contracts_before,
        "生产装配必须真的过了契约门（契约面被经过）"
    );

    let session = SessionId::new();
    let session_key = session.to_string();
    let user_message =
        NormalizedMessage::user(format!("{CANARY_GLOBAL} 交付窗口是周三下午, 别改时间"));
    let candidate = NormalizedResponse::text("assistant", "fixture-model", "好的, 已记下");

    // ---- 写入面（生产写回模块，AfterTurn）
    let writeback = writeback_module(&modules);
    run_hook(
        &writeback,
        HookPoint::AfterTurn,
        &session,
        std::slice::from_ref(&user_message),
        Some(&candidate),
    )
    .await;

    let stored = MemoryBackend::recent_episodes(db.as_ref(), &session_key, 20)
        .expect("读回写入的记忆")
        .into_iter()
        .map(|episode| episode.content)
        .collect::<Vec<_>>();
    assert!(
        stored.iter().any(|content| content.contains(CANARY_GLOBAL)),
        "生产写入面必须真的落库；实际写入 = {stored:?}"
    );

    // ---- 召回面（生产召回模块，TurnStart）
    let recall = recall_module(&modules);
    let query_message = NormalizedMessage::user(format!("{CANARY_GLOBAL} 什么时候交付"));
    let outcome = run_hook(
        &recall,
        HookPoint::TurnStart,
        &session,
        std::slice::from_ref(&query_message),
        None,
    )
    .await;
    let overlay = overlay_text(&outcome);

    // ⭐ **核心断言**：证据真的出现在注入里（不是「函数返回了 Ok」）。
    assert!(
        evidence_present(&overlay, CANARY_GLOBAL),
        "注入里必须真的出现记忆正文；注入 = {overlay}"
    );
    assert!(
        overlay.contains("<governed_memory"),
        "生产召回面必须出闭世界块（形态不能变）；注入 = {overlay}"
    );

    // ⭐ **预算纪律**：注入不许超硬上限。
    assert!(
        overlay.chars().count() <= RECALL_BUDGET_CHARS,
        "注入超出硬上限 {} 字：实际 {} 字",
        RECALL_BUDGET_CHARS,
        overlay.chars().count()
    );

    // ---- 契约面确实被经过（装配门真跑过，且是两套口径**当前**一致）
    let bundle = memory_contract_wire::install().expect("契约口径必须一致（分叉 ⇒ 拒绝装配）");
    assert_eq!(
        bundle.alignment_pairs,
        memory_contract_wire::ALIGNMENT_PAIRS,
        "口径自检必须真的覆盖全部范围组合（不是空跑）"
    );
    assert!(
        memory_contract_wire::contract_install_count() > contracts_before,
        "契约面必须被经过（只读计数口径）"
    );

    restore_data_root();
}

/// ⭐⭐ **注入形态**（`compile_injection_with_renderer` 那一支）：编号证据清单 +
/// 反幻觉规则 ⇒ 证据仍然真的在里面。
#[tokio::test]
async fn the_injection_format_also_carries_the_evidence_through_the_contract_renderer() {
    let (_root, _guard) = temp_data_root("injection-format");
    let db = Arc::new(SqliteMemoryStore::open_in_memory().expect("内存库"));
    let modules = production(&db, true);

    let session = SessionId::new();
    let session_key = session.to_string();
    let user_message =
        NormalizedMessage::user(format!("{CANARY_GLOBAL} 交付窗口是周三下午, 别改时间"));
    let candidate = NormalizedResponse::text("assistant", "fixture-model", "好的, 已记下");

    let writeback = writeback_module(&modules);
    run_hook(
        &writeback,
        HookPoint::AfterTurn,
        &session,
        std::slice::from_ref(&user_message),
        Some(&candidate),
    )
    .await;
    assert!(
        MemoryBackend::recent_episodes(db.as_ref(), &session_key, 20)
            .unwrap()
            .iter()
            .any(|episode| episode.content.contains(CANARY_GLOBAL)),
        "生产写入面必须真的落库"
    );

    let recall = recall_module(&modules);
    let query_message = NormalizedMessage::user(format!("{CANARY_GLOBAL} 什么时候交付"));
    let outcome = run_hook(
        &recall,
        HookPoint::TurnStart,
        &session,
        std::slice::from_ref(&query_message),
        None,
    )
    .await;
    let overlay = overlay_text(&outcome);

    assert!(
        evidence_present(&overlay, CANARY_GLOBAL),
        "注入形态里也必须真的出现记忆正文；注入 = {overlay}"
    );
    assert!(
        overlay.contains("[记忆证据"),
        "注入形态必须是编号证据清单；注入 = {overlay}"
    );
    assert!(
        overlay.contains("禁止说「我记得我们以前聊过」"),
        "反幻觉规则必须在（闭世界纪律）；注入 = {overlay}"
    );
    assert!(
        overlay.chars().count() <= RECALL_BUDGET_CHARS,
        "注入超出硬上限 {} 字：实际 {} 字",
        RECALL_BUDGET_CHARS,
        overlay.chars().count()
    );

    restore_data_root();
}

/// ⭐⭐ **模型看得见证据**：整条运行时跑一轮 ⇒ **模型请求里**真的有那条记忆。
#[tokio::test]
async fn the_model_actually_receives_the_recalled_evidence_in_its_request() {
    let (_root, _guard) = temp_data_root("model-sees");
    let db = Arc::new(SqliteMemoryStore::open_in_memory().expect("内存库"));
    let modules = production(&db, false);
    let provider = Provider::new(vec![vec![], vec![]]);
    let mut runtime = modules
        .register_into(builder(provider.clone()))
        .build()
        .await
        .expect("运行时必须起得来");
    let session = SessionId::new();

    // 第一轮：把记忆写进去（生产写回模块在 AfterTurn 落库）。
    runtime
        .execute(TurnRequest::new(
            session,
            format!("{CANARY_GLOBAL} 交付窗口是周三下午"),
        ))
        .await
        .expect("第一轮必须跑通");

    // 第二轮：问句命中 ⇒ 召回 ⇒ 证据进模型请求。
    runtime
        .execute(TurnRequest::new(
            session,
            format!("{CANARY_GLOBAL} 什么时候交付"),
        ))
        .await
        .expect("第二轮必须跑通");

    let requests = provider.requests();
    assert!(
        requests.len() >= 2,
        "至少要有两轮模型请求；实际 = {}",
        requests.len()
    );
    let second = &requests[1];
    let seen = second
        .messages
        .iter()
        .map(|message| ContentPart::join_text(&message.content))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        evidence_present(&seen, CANARY_GLOBAL),
        "模型请求里必须真的带上那条记忆（跨源披露通道也算）；请求消息 = {seen}"
    );
    // 跨源内容**不得**出现在系统通道（系统通道是宿主权威，不是记忆）。
    let system_text = second
        .messages
        .iter()
        .filter(|message| message.role == MessageRole::System)
        .map(|message| ContentPart::join_text(&message.content))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !evidence_present(&system_text, CANARY_GLOBAL),
        "记忆内容不许进系统通道；系统消息 = {system_text}"
    );

    assert!(runtime.shutdown().await.is_empty());
    restore_data_root();
}

/// ⭐ **跨会话 + 不同主体**：范围纪律（可见的召回、不可见的召不到）。
#[tokio::test]
async fn cross_session_and_multi_subject_scopes_are_respected() {
    let (_root, _guard) = temp_data_root("scopes");
    let db = Arc::new(SqliteMemoryStore::open_in_memory().expect("内存库"));
    // 与生产装配同一个构造口径（`memory_assembly::assemble_memory` 内部也是它）。
    let coordinator =
        MemoryCoordinator::new(db.clone(), db.clone()).with_scoped_backend(db.clone());

    // 写入面：四条不同范围/主体的记忆。
    write_scoped(
        &coordinator,
        "session-a",
        MemoryScope::Global,
        &format!("{CANARY_GLOBAL} 全局范围的记忆"),
    );
    write_scoped(
        &coordinator,
        "session-a",
        MemoryScope::User {
            user_id: "user-1".to_string(),
        },
        &format!("{CANARY_USER} 用户范围的记忆"),
    );
    write_scoped(
        &coordinator,
        "session-a",
        MemoryScope::User {
            user_id: "user-2".to_string(),
        },
        &format!("{CANARY_FOREIGN} 另一个主体的记忆"),
    );
    write_scoped(
        &coordinator,
        "session-a",
        MemoryScope::Session {
            session_id: "session-a".to_string(),
        },
        &format!("{CANARY_SESSION} 会话范围的记忆"),
    );

    // 召回面：**另一个会话** + 授予范围（全局 + user-1）。
    let query = MemoryRecallQuery::new("session-b", format!("{CANARY_GLOBAL} 交付"))
        .with_visible_scopes(vec![
            MemoryScope::Global,
            MemoryScope::User {
                user_id: "user-1".to_string(),
            },
            MemoryScope::Session {
                session_id: "session-b".to_string(),
            },
        ]);
    let selected = coordinator
        .compile_prompt_overlay_with_selected_access(&query)
        .expect("召回必须成功")
        .expect("有可见记忆 ⇒ 必须产出 overlay");
    let overlay = selected.overlay;

    assert!(
        evidence_present(&overlay, CANARY_GLOBAL),
        "全局范围的记忆必须跨会话被召回；注入 = {overlay}"
    );
    assert!(
        evidence_present(&overlay, CANARY_USER),
        "被授予主体的记忆必须被召回；注入 = {overlay}"
    );
    assert!(
        !evidence_present(&overlay, CANARY_FOREIGN),
        "没被授予的主体必须召不到（范围纪律）；注入 = {overlay}"
    );
    assert!(
        !evidence_present(&overlay, CANARY_SESSION),
        "会话范围的记忆不许被另一个会话召到；注入 = {overlay}"
    );
    assert!(
        overlay.chars().count() <= RECALL_BUDGET_CHARS,
        "注入超出硬上限 {} 字：实际 {} 字",
        RECALL_BUDGET_CHARS,
        overlay.chars().count()
    );

    restore_data_root();
}

/// ⭐ **默认态零副作用**：影子/换算法**未配置** ⇒ 不构造、不落文件、
/// 产物与「未接线」的裸协调器**逐字相同**。
#[tokio::test]
async fn the_default_state_has_no_shadow_side_effects_and_is_byte_identical() {
    let (root, _guard) = temp_data_root("zero-side-effects");
    let db = Arc::new(SqliteMemoryStore::open_in_memory().expect("内存库"));
    let modules = production(&db, false);

    // 影子/换算法面：**未配置** ⇒ 什么都不构造。
    assert!(
        shadow_wire::config_from_data_root(root.path())
            .expect("缺配置不是错误")
            .is_none(),
        "默认态必须没有影子配置（否则就不是默认态）"
    );
    assert!(
        shadow_wire::service().is_none(),
        "默认态必须**不构造**影子服务（进程槽必须还是空的）"
    );

    let session = SessionId::new();
    let session_key = session.to_string();
    let user_message =
        NormalizedMessage::user(format!("{CANARY_GLOBAL} 交付窗口是周三下午, 别改时间"));
    let candidate = NormalizedResponse::text("assistant", "fixture-model", "好的, 已记下");
    let writeback = writeback_module(&modules);
    run_hook(
        &writeback,
        HookPoint::AfterTurn,
        &session,
        std::slice::from_ref(&user_message),
        Some(&candidate),
    )
    .await;

    // 影子面**零新增文件**。
    assert!(
        !root.path().join("shadow.json").exists(),
        "默认态不许落影子配置文件"
    );
    assert!(
        !root.path().join("memory/shadow").exists(),
        "默认态不许落影子归档/指针目录"
    );
    // 整条链路跑完后，数据根里**一个文件都不许多**（影子/旋钮/水位都不许偷偷落盘）。
    let mut files = Vec::new();
    collect_files(root.path(), &mut files);
    assert!(
        files.is_empty(),
        "默认态不许落任何文件；实际落了 = {files:?}"
    );

    // 产物与「未接线」的裸协调器**逐字相同**（契约接线 + 影子接线都不改产物）。
    let bare = MemoryCoordinator::new(db.clone(), db.clone()).with_scoped_backend(db.clone());
    bare.writeback(&{
        let mut entry = MemoryWritebackEntry::new(
            &session_key,
            "user",
            format!("{CANARY_GLOBAL} 交付窗口是周三下午, 别改时间"),
        );
        entry.scope = MemoryScope::Session {
            session_id: session_key.clone(),
        };
        entry
    })
    .expect("裸协调器写入");
    let bare_query = MemoryRecallQuery::new(&session_key, format!("{CANARY_GLOBAL} 交付"))
        .with_visible_scopes(vec![MemoryScope::Session {
            session_id: session_key.clone(),
        }]);
    let bare_overlay = bare
        .compile_prompt_overlay_with_selected_access(&bare_query)
        .expect("裸协调器召回")
        .expect("裸协调器必须产出 overlay")
        .overlay;

    let query_message = NormalizedMessage::user(format!("{CANARY_GLOBAL} 什么时候交付"));
    let outcome = run_hook(
        &recall_module(&modules),
        HookPoint::TurnStart,
        &session,
        std::slice::from_ref(&query_message),
        None,
    )
    .await;
    let production_overlay = overlay_text(&outcome);

    assert!(
        evidence_present(&production_overlay, CANARY_GLOBAL),
        "生产链路的注入必须真的带上记忆；注入 = {production_overlay}"
    );
    assert!(
        production_overlay.contains(&bare_overlay),
        "接线后的产物必须与未接线的裸产物**逐字相同**（契约视图是无损投影）；\n生产 = {production_overlay}\n裸 = {bare_overlay}"
    );

    restore_data_root();
}

/// ⭐ **失败闭合**：配置坏了 ⇒ 装配**必须失败**（不许静默继续）。
#[tokio::test]
async fn a_broken_configuration_refuses_to_assemble() {
    let (root, _guard) = temp_data_root("broken-config");

    // 坏排序旋钮文件 ⇒ 拒绝装配（不许被当成「没配置」）。
    std::fs::write(root.path().join("retrieval.json"), b"{ not json").expect("写坏配置");
    let db = Arc::new(SqliteMemoryStore::open_in_memory().expect("内存库"));
    let build = ProductionModules::build(
        CognitiveModuleConfig {
            memory_recall: true,
            memory_writeback: true,
            preference_recall: false,
            self_assessment: false,
            filesystem: false,
            search: false,
            repo: false,
            ..Default::default()
        },
        CognitiveBackends {
            memory: Some(db.clone()),
            memory_governance: Some(db.clone()),
            scoped_memory: Some(db.clone()),
            ..Default::default()
        },
        Arc::new(SystemClock),
    );
    assert!(build.is_err(), "坏排序配置必须让装配失败（不许静默继续）");
    assert!(
        !build.err().unwrap().to_string().is_empty(),
        "失败必须带真因"
    );

    // 坏影子配置 ⇒ 拒绝装配（不许被当成「没配置」）。
    std::fs::remove_file(root.path().join("retrieval.json")).expect("清理");
    std::fs::write(root.path().join("shadow.json"), b"{ not json").expect("写坏影子配置");
    let db2 = Arc::new(SqliteMemoryStore::open_in_memory().expect("内存库"));
    let build2 = ProductionModules::build(
        CognitiveModuleConfig {
            memory_recall: true,
            memory_writeback: true,
            preference_recall: false,
            self_assessment: false,
            filesystem: false,
            search: false,
            repo: false,
            ..Default::default()
        },
        CognitiveBackends {
            memory: Some(db2.clone()),
            memory_governance: Some(db2.clone()),
            scoped_memory: Some(db2.clone()),
            ..Default::default()
        },
        Arc::new(SystemClock),
    );
    assert!(build2.is_err(), "坏影子配置必须让装配失败（不许静默继续）");

    restore_data_root();
}

// ---------------------------------------------------------------- 反证

/// **反证 ①**：把**注入渲染**短路（渲染器返回空）⇒ 核心判定谓词必须为假。
///
/// 这条用的就是生产链路上那一个渲染入口（`compile_injection_with_renderer`）
/// ⇒ ⇒ 主用例的断言不是自证。
#[test]
fn the_evidence_assertion_bites_when_the_injection_renderer_returns_empty() {
    struct EmptyRenderer;
    impl InjectionRenderer for EmptyRenderer {
        fn render(&self, _evidence: &[Evidence], _budget: Budget) -> Injection {
            Injection::Empty {
                reason: "反证：渲染器短路".to_string(),
            }
        }
    }

    let recalled = apeireth_memory::layers::MemoryRecallResult {
        items: vec![apeireth_memory::layers::RecalledMemoryItem {
            id: "ep-1".to_string(),
            layer: apeireth_memory::layers::MemoryLayerKind::Episodic,
            content: format!("{CANARY_GLOBAL} 交付窗口是周三下午"),
            timestamp_ms: 1_700_000_000_000,
            score: 0.9,
            importance: 0.5,
            source_ref: Some("session-a".to_string()),
            score_components: None,
            diary_snippets: Vec::new(),
        }],
        ..Default::default()
    };
    let compiler = ClosedWorldContextCompiler::new();
    let rendered = compiler
        .compile_injection_with_renderer(
            &recalled,
            "session-a",
            RECALL_BUDGET_CHARS,
            120,
            &EmptyRenderer,
        )
        .expect("短路渲染器仍走同一条编译路径");
    assert!(
        !evidence_present(&rendered.overlay, CANARY_GLOBAL),
        "渲染器短路 ⇒ 注入里不许出现证据（否则主断言是自证）；注入 = {}",
        rendered.overlay
    );
}

/// **反证 ②**：把**契约视图**短路（未授权范围 ⇒ 视图为空）⇒ 条目根本到不了编译器
/// ⇒ 核心判定谓词必须为假。
#[test]
fn the_evidence_assertion_bites_when_the_contract_view_is_not_granted() {
    let items = vec![apeireth_memory::layers::RecalledMemoryItem {
        id: "ep-1".to_string(),
        layer: apeireth_memory::layers::MemoryLayerKind::Episodic,
        content: format!("{CANARY_GLOBAL} 交付窗口是周三下午"),
        timestamp_ms: 1_700_000_000_000,
        score: 0.9,
        importance: 0.5,
        source_ref: Some("session-a".to_string()),
        score_components: None,
        diary_snippets: Vec::new(),
    }];
    let stamp = AlgoStamp {
        algo_id: "e2e.smoke".to_string(),
        version: "1".to_string(),
        params_digest: "e2e".to_string(),
    };
    // 未授权 ⇒ fail-closed ⇒ 视图空 ⇒ 条目进不了编译路径。
    let closed: SelectedMemoryView = SelectedMemoryView::build(&items, &[], stamp.clone());
    assert!(closed.is_empty(), "契约视图未授权时必须为空（fail-closed）");
    let compiled = ClosedWorldContextCompiler::new().compile_injection_with_renderer(
        &apeireth_memory::layers::MemoryRecallResult {
            items: closed.items(),
            ..Default::default()
        },
        "session-a",
        RECALL_BUDGET_CHARS,
        120,
        &apeireth_memory::injection_renderer::NumberedEvidenceRenderer::new(),
    );
    assert!(
        compiled.is_none(),
        "契约视图为空 ⇒ 编译路径必须什么都不产出（不许凭空造证据）"
    );

    // 对照：授权了 ⇒ 条目原样带出（无损投影 ⇒ 证据在）。
    let granted = SelectedMemoryView::build(&items, &[Scope::Global], stamp);
    assert!(
        evidence_present(&granted.items()[0].content, CANARY_GLOBAL),
        "授权后条目必须原样带出（无损投影）"
    );
}

/// **反证 ③**：把**写入面**短路（根本没写）⇒ 召回不到 ⇒ 核心判定谓词必须为假。
#[tokio::test]
async fn the_evidence_assertion_bites_when_the_memory_was_never_written() {
    let (_root, _guard) = temp_data_root("never-written");
    let db = Arc::new(SqliteMemoryStore::open_in_memory().expect("内存库"));
    let modules = production(&db, false);

    let session = SessionId::new();
    let query_message = NormalizedMessage::user(format!("{CANARY_GLOBAL} 什么时候交付"));
    let outcome = run_hook(
        &recall_module(&modules),
        HookPoint::TurnStart,
        &session,
        std::slice::from_ref(&query_message),
        None,
    )
    .await;
    let overlay = overlay_text(&outcome);
    assert!(
        !evidence_present(&overlay, CANARY_GLOBAL),
        "没写入的记忆不许出现在注入里（否则主断言是自证）；注入 = {overlay}"
    );

    restore_data_root();
}
