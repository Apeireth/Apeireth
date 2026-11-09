//! The fixed retrieval hook: an explicit switch that can open a **working** path only.
//!
//! # What this file pins
//!
//! `memory_assembly.rs` used to hard-code `with_automatic_retrieval(false)` ⇒ ⇒
//! "the fixed hook is off by default" was in fact "**there is no way to turn it
//! on**". The switch is now the data-root file `memory-hook.json`
//! (`{"automatic_retrieval": true}`).
//!
//! A switch is only worth having if the thing it switches **works** when opened.
//! Measured on the default desktop path with the hook armed, the hook's own
//! `memory_retrieve` call was refused by the dispatcher's scope gate
//! (`scope_denied / tool is outside this session's tool scope`) — **one failing
//! frame per turn**. So the module side carries a second gate: the hook speaks
//! only when the Host bound a namespace mapping, i.e. when the memory tool face
//! has an authority to speak with.
//!
//! * `the_fixed_hook_switch_opens_a_working_path` — switch on + Host identity +
//!   mapping ⇒ the hook fires and the recall lands in the transcript.
//! * `the_fixed_hook_switch_never_opens_a_failing_path` — switch on, **no**
//!   mapping ⇒ the hook stays **silent**: no failing frame, no transcript noise.
//!
//! # Counter-evidence
//!
//! Removing the mapping gate in `memory_module.rs` turns the second case red
//! (the hook fires and is refused); removing the switch reader in
//! `memory_assembly.rs` turns the first case red (the hook never fires).
//!
//! # Why this file is alone
//!
//! It moves the process-wide `APEIRETH_DATA_DIR` (the data root's single
//! source) ⇒ ⇒ it is its own test binary so no other test can observe it.
#[path = "support/convergence.rs"]
mod support;
use apeireth_core::kernel::{CapabilityId, NamespaceId, ProfileId, SessionId, SystemClock};
use apeireth_memory::{MemoryCoordinator, MemoryScope, SqliteMemoryStore};
use apeireth_plugin::agents::{AgentConfig, AgentPrincipal};
use apeireth_plugin::memory_backend::MemoryBackend;
use apeireth_protocol::canonical::{ContentPart, MessageRole};
use apeireth_runtime::{AgentHostPolicy, RuntimeBuilder};
use apeireth_runtime_assembly::{
    profile_memory::{ProfileMemoryAccess, ProfileMemoryMappings},
    CognitiveBackends, CognitiveModuleConfig, ProductionModules,
};
use std::sync::Arc;
use support::*;

/// 开关文件（与 `memory_assembly.rs` 的常量同形；信封格式照 `StoredDoc`）。
const SWITCH: &[u8] = br#"{"name":"apeireth.memory.hook","version":1,"compatible_versions":[1],"body":{"automatic_retrieval":true}}"#;

fn cid(id: &str) -> CapabilityId {
    CapabilityId::new(id).unwrap()
}

fn principal() -> AgentPrincipal {
    AgentPrincipal {
        profile: ProfileId::new(),
        namespace: NamespaceId::new(),
    }
}

fn memory() -> (Arc<MemoryCoordinator>, Arc<SqliteMemoryStore>) {
    let db = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    (
        Arc::new(MemoryCoordinator::new(db.clone(), db.clone()).with_scoped_backend(db.clone())),
        db,
    )
}

fn production(db: Arc<SqliteMemoryStore>) -> ProductionModules {
    ProductionModules::build(
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
    )
    .unwrap()
}

/// 把数据根指到临时目录并写好开关文件（进程级 env ⇒ 本文件独占一个测试二进制）。
fn arm_switch() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("apeireth-hook-e2e-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(dir.join("memory-hook.json"), SWITCH).expect("write");
    std::env::set_var("APEIRETH_DATA_DIR", &dir);
    dir
}

/// 转录里有没有固定检索 Hook 的帧。
fn hook_frames(provider: &Provider) -> Vec<String> {
    provider
        .requests()
        .iter()
        .flat_map(|request| request.messages.iter())
        .filter(|message| message.role == MessageRole::Tool)
        .map(|message| ContentPart::join_text(&message.content))
        .filter(|text| text.contains("before_model_call") && text.contains("module.memory"))
        .collect()
}

/// ① **反证**: 开关打开 + Host 身份 + 命名空间映射 ⇒ 固定检索 Hook **真的跑**
/// （转录里有 Hook 帧，且召回内容进了转录）。
///
/// 反证口径：把 `memory_assembly.rs` 里的开关读成常量 `false` ⇒ 本例红
/// （Hook 永不触发 ⇒ 「默认关 = 没有打开路径」回来了）。
#[tokio::test]
async fn the_fixed_hook_switch_opens_a_working_path() {
    arm_switch();
    let (coordinator, db) = memory();
    let owner = principal();
    let mappings = Arc::new(ProfileMemoryMappings::default());
    mappings
        .grant(
            &owner,
            Some(ProfileMemoryAccess {
                coordinator: coordinator.clone(),
                read_scopes: vec![MemoryScope::Global],
                writable_private: true,
            }),
            vec![],
        )
        .unwrap();
    let provider = Provider::new(vec![]);
    let runtime = production(db)
        .register_with_profile_mappings(builder(provider.clone()), mappings)
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let agent = runtime
        .spawn_agent(
            "main",
            AgentConfig {
                principal: Some(owner),
                tools: vec![cid("tool.memory.retrieve")],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    agent.run("what do you remember".into()).await.unwrap();

    let frames = hook_frames(&provider);
    assert!(
        !frames.is_empty(),
        "开关打开 + Host 身份 + 映射 ⇒ 固定检索 Hook 必须真的跑；转录里没有 Hook 帧"
    );
    assert!(
        !frames.iter().any(|frame| frame.contains("scope_denied")),
        "Hook 帧不许是失败帧；frames = {frames:?}"
    );
}

/// ② **反证**: 开关打开但**没有** Host 命名空间映射 ⇒ Hook **保持沉默**
/// （既不跑、也不留失败帧 ⇒ 「开了但兑现不了」的状态不存在）。
///
/// 反证口径：去掉 `memory_module.rs` 的映射闸 ⇒ 本例红（Hook 会发一次
/// `memory_retrieve` 并被分发器范围闸拒 ⇒ 每回合一条失败帧）。
#[tokio::test]
async fn the_fixed_hook_switch_never_opens_a_failing_path() {
    arm_switch();
    let (_coordinator, db) = memory();
    let owner = principal();
    let provider = Provider::new(vec![]);
    let runtime = production(db)
        .register_into(builder(provider.clone()))
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let agent = runtime
        .spawn_agent(
            "main",
            AgentConfig {
                principal: Some(owner),
                tools: vec![cid("tool.memory.retrieve")],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    agent.run("what do you remember".into()).await.unwrap();

    let frames = hook_frames(&provider);
    assert!(
        frames.is_empty(),
        "没有 Host 命名空间绑定 ⇒ 固定检索 Hook 必须沉默；frames = {frames:?}"
    );
    let failures: Vec<String> = provider
        .requests()
        .iter()
        .flat_map(|request| request.messages.iter())
        .filter(|message| message.role == MessageRole::Tool)
        .map(|message| ContentPart::join_text(&message.content))
        .filter(|text| text.contains("scope_denied") || text.contains("namespace_denied"))
        .collect();
    assert!(
        failures.is_empty(),
        "沉默 = 一条失败帧都不留；failures = {failures:?}"
    );
}
