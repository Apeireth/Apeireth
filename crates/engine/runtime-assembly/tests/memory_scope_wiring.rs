//! **记忆作用域写回接线**（B3b-2）：合同受管作用域真的落到写回上。
//!
//! # 这个文件补的是哪个洞
//!
//! 翻译层（`canonical/memory_scope_wire.rs`）能把 `MemoryScopePlan` 忠实翻成
//! `MemoryScope`，**翻译对不等于接上了**：写回链路里原来 7 处硬编码
//! `MemoryScope::Session`，已绑定会话即使合同要求 `Persona`，记忆仍会落进
//! 会话作用域。本文件从**生产装配**（`ProductionModules::build` +
//! `CognitiveBackends.memory_scope_binding`）出发，断言落库元数据里的
//! `scope` 就是合同要求的那一个。
//!
//! # 覆盖的判定面（每条都对应一个会真出错的短路）
//!
//! | 用例 | 期望 |
//! |---|---|
//! | 已绑定 + `Contact` 合同（**真实 runtime 回合**） | 全部落库 `Persona{contact_id, subject_id}`，同回合无 `Session` 行 |
//! | 未绑定会话 | 逐字节沿用今日 `Session` 路径（0 回归） |
//! | 已绑定 + `Session` 合同 | `Session{session_id}` |
//! | 已绑定 + `None` 合同 | **零写入**（`none` ≠ `Global`，也 ≠ `Session`） |
//! | 已绑定、联系人已归档 | **零写入**（fail-closed，绝不回退 `Session`） |
//! | 已绑定、但装配面缺位 | **零写入**（缺位 = 显式跳过，不是回退） |
//!
//! # 反证纪律
//!
//! 断言只认**落库元数据里的 scope**（不是「函数返回 Ok」）。反向验伪时逐条
//! 把翻译/分派短路，本文件的相应用例必须真的红（红/绿输出见交付记录）。
//!
//! # 为什么归档用例直接调 hook
//!
//! 已绑定会话的联系人一旦归档，`turn_admission` 会在 `runtime.execute` **之前**
//! 拒绝整个回合 ⇒ 走不到写回模块。要测写回自己的 fail-closed 分支，只能直接
//! 调生产写回模块的 `AfterTurn` 钩子（`ModuleContext.agent_binding` 仍带绑定）。

#[path = "support/convergence.rs"]
mod support;

use apeireth_core::agent_contact::{
    Contact, ContactCreateRequest, ContactRegistry, ContractMemoryScope, InMemoryContactRegistry,
    PersonaText,
};
use apeireth_core::kernel::{Clock, SessionId, SystemClock};
use apeireth_memory::{MemoryScope, SqliteMemoryStore};
use apeireth_plugin::memory_backend::MemoryBackend;
use apeireth_protocol::canonical::{NormalizedMessage, NormalizedResponse};
use apeireth_runtime::canonical::agent_binding::{
    resolve_agent_binding, AgentBinding, AgentBindingRequest,
};
use apeireth_runtime::canonical::module::{
    HookPoint, InvocationContext, ModuleContext, ModuleInvocationError, ModuleInvocationRequest,
    ModuleInvocationResponse, ModuleInvoker, ModuleOutcome,
};
use apeireth_runtime::canonical::session::Session;
use apeireth_runtime::canonical::subloop::{
    SubLoopError, SubLoopResult, SubLoopSpawner, SubLoopSpec,
};
use apeireth_runtime::canonical::workspace::{
    InMemoryWorkspaceRegistry, WorkspaceCreateRequest, WorkspacePermissionScope, WorkspaceRegistry,
};
use apeireth_runtime::TurnRequest;
use apeireth_runtime_assembly::canonical::cognitive::{
    MemoryScopeWiring, MEMORY_RECALL_MODULE_ID, MEMORY_WRITEBACK_MODULE_ID,
};
use apeireth_runtime_assembly::canonical::desktop_host::build_desktop_main_runtime_at;
use apeireth_runtime_assembly::{CognitiveBackends, CognitiveModuleConfig, ProductionModules};
use async_trait::async_trait;
use std::sync::Arc;
use support::*;

/// 主体身份（与 CLI 默认 `APEIRETH_SUBJECT_ID` 同口径；这里显式钉住，不读环境）。
const SUBJECT_ID: &str = "local-user";
/// 固定时间戳：注册表的「当前」时刻由测试钉住，不依赖墙钟。
const NOW_MS: u64 = 1_767_000_000_000;

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

fn config() -> CognitiveModuleConfig {
    CognitiveModuleConfig {
        memory_recall: false,
        memory_writeback: true,
        preference_recall: false,
        self_assessment: false,
        filesystem: false,
        search: false,
        repo: false,
        ..Default::default()
    }
}

/// 生产装配（只开写回），装配面按需带上记忆作用域接线。
fn modules(db: &Arc<SqliteMemoryStore>, wiring: Option<MemoryScopeWiring>) -> ProductionModules {
    ProductionModules::build(
        config(),
        CognitiveBackends {
            memory: Some(db.clone()),
            memory_governance: Some(db.clone()),
            scoped_memory: Some(db.clone()),
            memory_scope_binding: wiring,
            ..Default::default()
        },
        Arc::new(SystemClock),
    )
    .expect("生产装配必须成功（契约口径分叉 / 配置坏 ⇒ 拒绝装配）")
}

/// 从生产装配里取出**记忆写回模块**（按注册 id 找 ⇒ 拿的就是生产那一个）。
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

/// 跑一次生产写回模块的 `AfterTurn` 钩子（`agent_binding` 就是宿主事实）。
async fn run_after_turn(
    module: &Arc<dyn apeireth_runtime::canonical::module::Module>,
    session: &SessionId,
    text: &str,
    binding: Option<&AgentBinding>,
) -> ModuleOutcome {
    let user_message = NormalizedMessage::user(text.to_owned());
    let messages = std::slice::from_ref(&user_message);
    let candidate = NormalizedResponse::text("assistant", "fixture-model", "好的, 已记下");
    let invocation = InvocationContext::user_turn();
    let invoker: Arc<dyn ModuleInvoker> = Arc::new(NoopInvoker);
    let subloop = NoSubLoop;
    let ctx = ModuleContext {
        session_id: session,
        model: "fixture-model",
        messages,
        candidate: Some(&candidate),
        tool_call: None,
        tool_result: None,
        invocation: &invocation,
        module_id: MEMORY_RECALL_MODULE_ID,
        error: None,
        invoker: &*invoker,
        invoker_handle: Arc::clone(&invoker),
        subloop: &subloop,
        agent_binding: binding,
    };
    module
        .on_hook(HookPoint::AfterTurn, &ctx)
        .await
        .expect("记忆写回模块钩子不许失败")
}

/// 读回本会话全部落库记忆的作用域（元数据里的 canonical `MemoryScope`）。
fn scopes_of(db: &SqliteMemoryStore, session_key: &str) -> Vec<MemoryScope> {
    MemoryBackend::recent_episodes(db, session_key, 50)
        .expect("读回写入的记忆")
        .into_iter()
        .map(|episode| {
            let metadata = MemoryBackend::get_episode_metadata(db, &episode.id)
                .expect("读回记忆元数据")
                .unwrap_or_else(|| panic!("落库记忆 {} 必须带元数据", episode.id));
            serde_json::from_value::<MemoryScope>(metadata["scope"].clone()).unwrap_or_else(|e| {
                panic!("元数据 scope 必须是 canonical MemoryScope: {e}；metadata={metadata}")
            })
        })
        .collect()
}

/// 联系人 + 工作区 + 由两者铸造的绑定（全部是生产 DTO，不伪造身份）。
struct Fixture {
    contacts: Arc<InMemoryContactRegistry>,
    workspaces: Arc<InMemoryWorkspaceRegistry>,
    contact: Contact,
    binding: AgentBinding,
    /// 工作区根必须真实存在，且活到绑定校验之后。
    _root: tempfile::TempDir,
}

fn fixture(scope: ContractMemoryScope) -> Fixture {
    let root = tempfile::tempdir().expect("临时工作区根");
    let contacts = Arc::new(InMemoryContactRegistry::new());
    let contact = contacts
        .create(ContactCreateRequest {
            display_name: "B3b-2 联系人".into(),
            subtitle: None,
            avatar_ref: None,
            persona: Some(PersonaText::new("B3b-2 测试人设").expect("合法人设")),
            memory_scope: scope,
            now_ms: NOW_MS,
        })
        .expect("建联系人");
    let workspaces = Arc::new(InMemoryWorkspaceRegistry::new());
    let workspace = workspaces
        .create(WorkspaceCreateRequest::managed(
            root.path(),
            "B3b-2 工作区",
            WorkspacePermissionScope::Write,
            NOW_MS,
        ))
        .expect("建工作区");
    let request = AgentBindingRequest::for_current(&contact, &workspace);
    let binding = resolve_agent_binding(&request, contacts.as_ref(), workspaces.as_ref(), NOW_MS)
        .expect("铸造绑定");
    Fixture {
        contacts,
        workspaces,
        contact,
        binding,
        _root: root,
    }
}

fn wiring(fixture: &Fixture) -> MemoryScopeWiring {
    MemoryScopeWiring {
        contacts: fixture.contacts.clone() as Arc<dyn ContactRegistry>,
        workspaces: fixture.workspaces.clone() as Arc<dyn WorkspaceRegistry>,
        subject_id: SUBJECT_ID.into(),
    }
}

// ---------------------------------------------------------------- 用例

/// ⭐⭐ 核心验收：**真实 runtime 回合**里，已绑定 Persona 合同的会话
/// 写回的每一条记忆都落在 `Persona` 作用域，且同回合没有 `Session` 行。
#[tokio::test]
async fn a_real_runtime_turn_writes_persona_scope_for_a_bound_contact() {
    let db = Arc::new(SqliteMemoryStore::open_in_memory().expect("内存库"));
    let fixture = fixture(ContractMemoryScope::Contact);
    let contact_id = fixture.contact.contact_id();
    let modules = modules(&db, Some(wiring(&fixture)));
    let provider = Provider::new(vec![]);
    let mut policy = modules.desktop_agent_policy().expect("桌面策略");
    policy.concurrent_turns = 1;
    let data_root = tempfile::tempdir().expect("临时数据根");
    let runtime = build_desktop_main_runtime_at(
        modules
            .register_into(builder(provider))
            .with_contact_registry(Some(fixture.contacts.clone() as Arc<dyn ContactRegistry>))
            .with_workspace_registry(Some(
                fixture.workspaces.clone() as Arc<dyn WorkspaceRegistry>
            )),
        policy,
        data_root.path(),
    )
    .await
    .expect("构建桌面主运行时");

    let clock = SystemClock;
    let session_id = SessionId::new();
    let mut session = Session::new(session_id, &clock);
    session.set_agent_binding(fixture.binding.clone(), &clock);
    runtime
        .sessions()
        .save(&session)
        .await
        .expect("预存带绑定的会话");

    runtime
        .execute(TurnRequest::new(session_id, "记住：交付窗口是周三下午"))
        .await
        .expect("已绑定会话的回合必须成功");

    let scopes = scopes_of(db.as_ref(), &session_id.to_string());
    assert!(!scopes.is_empty(), "已绑定会话必须真的写回记忆");
    let expected = MemoryScope::Persona {
        persona_id: contact_id.to_string(),
        user_id: SUBJECT_ID.into(),
    };
    assert!(
        scopes.iter().all(|scope| *scope == expected),
        "每一条落库记忆都必须是 Persona 作用域；实际 = {scopes:?}"
    );
    assert!(
        !scopes
            .iter()
            .any(|scope| matches!(scope, MemoryScope::Session { .. })),
        "已绑定 Persona 会话同回合不许再落 session 作用域行；实际 = {scopes:?}"
    );
}

/// 未绑定会话：今日行为逐字节不变（`Session{session_id}`）。
#[tokio::test]
async fn an_unbound_session_keeps_the_session_scope() {
    let db = Arc::new(SqliteMemoryStore::open_in_memory().expect("内存库"));
    let fixture = fixture(ContractMemoryScope::Contact);
    let modules = modules(&db, Some(wiring(&fixture)));
    let writeback = writeback_module(&modules);
    let session = SessionId::new();

    run_after_turn(&writeback, &session, "记住：未绑定会话", None).await;

    let scopes = scopes_of(db.as_ref(), &session.to_string());
    assert!(!scopes.is_empty(), "未绑定会话必须真的写回记忆");
    let expected = MemoryScope::Session {
        session_id: session.to_string(),
    };
    assert!(
        scopes.iter().all(|scope| *scope == expected),
        "未绑定会话必须逐字节沿用 Session 路径；实际 = {scopes:?}"
    );
}

/// 已绑定 + `Session` 合同 ⇒ 落 `Session{session_id}`（三变体里最容易漏的一支）。
#[tokio::test]
async fn a_bound_session_contract_writes_the_session_scope() {
    let db = Arc::new(SqliteMemoryStore::open_in_memory().expect("内存库"));
    let fixture = fixture(ContractMemoryScope::Session);
    let modules = modules(&db, Some(wiring(&fixture)));
    let writeback = writeback_module(&modules);
    let session = SessionId::new();

    run_after_turn(
        &writeback,
        &session,
        "记住：合同要求会话域",
        Some(&fixture.binding),
    )
    .await;

    let scopes = scopes_of(db.as_ref(), &session.to_string());
    assert!(!scopes.is_empty(), "已绑定会话必须真的写回记忆");
    let expected = MemoryScope::Session {
        session_id: session.to_string(),
    };
    assert!(
        scopes.iter().all(|scope| *scope == expected),
        "Session 合同必须落 Session 作用域；实际 = {scopes:?}"
    );
}

/// 已绑定 + `None` 合同 ⇒ **零写入**（`none` 既不是 `Global` 也不是 `Session`）。
#[tokio::test]
async fn a_bound_none_contract_writes_nothing() {
    let db = Arc::new(SqliteMemoryStore::open_in_memory().expect("内存库"));
    let fixture = fixture(ContractMemoryScope::None);
    let modules = modules(&db, Some(wiring(&fixture)));
    let writeback = writeback_module(&modules);
    let session = SessionId::new();

    run_after_turn(
        &writeback,
        &session,
        "记住：合同要求不写",
        Some(&fixture.binding),
    )
    .await;

    let scopes = scopes_of(db.as_ref(), &session.to_string());
    assert!(
        scopes.is_empty(),
        "None 合同必须零写入（不是落 session）；实际 = {scopes:?}"
    );
}

/// 已绑定、联系人已归档 ⇒ **零写入**（fail-closed；绝不回退 `Session`）。
#[tokio::test]
async fn an_archived_contact_fails_closed_without_any_write() {
    let db = Arc::new(SqliteMemoryStore::open_in_memory().expect("内存库"));
    let fixture = fixture(ContractMemoryScope::Contact);
    let contact_id = fixture.contact.contact_id();
    fixture
        .contacts
        .archive(&contact_id, fixture.contact.revision(), NOW_MS)
        .expect("归档联系人（软删除）");
    let modules = modules(&db, Some(wiring(&fixture)));
    let writeback = writeback_module(&modules);
    let session = SessionId::new();

    run_after_turn(
        &writeback,
        &session,
        "记住：归档联系人不许写",
        Some(&fixture.binding),
    )
    .await;

    let scopes = scopes_of(db.as_ref(), &session.to_string());
    assert!(
        scopes.is_empty(),
        "绑定失效必须整段不写（绝不回退 Session）；实际 = {scopes:?}"
    );
}

/// 已绑定、但装配面缺位 ⇒ **零写入**（缺位 = 显式跳过，不是回退 `Session`）。
#[tokio::test]
async fn a_bound_session_without_the_wiring_surface_writes_nothing() {
    let db = Arc::new(SqliteMemoryStore::open_in_memory().expect("内存库"));
    let fixture = fixture(ContractMemoryScope::Contact);
    let modules = modules(&db, None);
    let writeback = writeback_module(&modules);
    let session = SessionId::new();

    run_after_turn(
        &writeback,
        &session,
        "记住：装配面缺位",
        Some(&fixture.binding),
    )
    .await;

    let scopes = scopes_of(db.as_ref(), &session.to_string());
    assert!(
        scopes.is_empty(),
        "装配面缺位必须显式跳过（不是回退 Session）；实际 = {scopes:?}"
    );
}
