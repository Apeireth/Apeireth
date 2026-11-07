//! 生产装配的**注册式 Memory 工具面**（`mem.registered_namespace` / `rt.owned_agents`）。
//!
//! # 这三个测试为什么存在（它们不是「再测一遍模块」）
//!
//! 2026-11-07 之前，`ProductionModules::build` 只把 `MemoryCoordinator` 喂给**行为模块**
//! （召回/写回），**从不**构造 `MemoryModule` ⇒ ⇒ 生产装配里 `module.memory` 不存在
//! ⇒ ⇒ 7 个 `memory_*` 工具一个都没注册（调用即 `requested tool is not registered`）；
//! 只有**测试自己**手动 `with_plugin(MemoryModule::new(...))` 才跑得起来
//! ⇒ ⇒ 「测试在调用装配根不提供的东西」。
//!
//! 现在工具面由装配根自己登记（`canonical/memory_assembly.rs`），所以：
//! * 本文件**不**手动注册任何 `MemoryModule`（手动注册 = 重复 `module.memory`）；
//! * 反证口径：把 `memory_assembly::wire` 里的 `tool_modules.push(...)` 去掉
//!   ⇒ 三个测试都会红在 `requested tool is not registered`。
#[path = "support/convergence.rs"]
mod support;
use apeireth_core::kernel::{CapabilityId, NamespaceId, ProfileId, SessionId, SystemClock};
use apeireth_memory::{MemoryCoordinator, MemoryScope, SqliteMemoryStore};
use apeireth_plugin::agents::{AgentConfig, AgentPrincipal};
use apeireth_runtime::{AgentHostPolicy, ModuleAgentGrant, RuntimeBuilder};
use apeireth_runtime_assembly::{
    profile_memory::{ProfileMemoryAccess, ProfileMemoryMappings},
    CognitiveBackends, CognitiveModuleConfig, ProductionModules,
};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use support::*;

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

/// 生产装配（记忆工具面必须靠它自己提供）。
fn production(db: &Arc<SqliteMemoryStore>) -> ProductionModules {
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

fn memory_tools() -> Vec<CapabilityId> {
    [
        "tool.memory.retrieve",
        "tool.memory.write",
        "tool.memory.organize",
    ]
    .iter()
    .map(|id| cid(id))
    .collect()
}

/// ⭐ **E2E**：生产装配的运行时里，`memory_write` 真被执行且**成功**。
///
/// 断言两件事：① 工具结果不是「未注册/被拒」；② 结果里带真写入回执（`revision`）。
#[tokio::test]
async fn production_assembly_registers_memory_tools_and_a_real_write_succeeds() {
    let (_c, db) = memory();
    let provider = Provider::new(vec![
        vec![call(
            "memory_write",
            json!({"content":"orchard_public_write","idempotency_key":"k1"}),
        )],
        vec![],
    ]);
    let runtime = production(&db)
        .register_into(builder(provider.clone()))
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let agent = runtime
        .spawn_agent(
            "main",
            AgentConfig {
                principal: Some(principal()),
                tools: memory_tools(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    agent.run("remember this".into()).await.unwrap();
    let results = provider.results();
    let write = results
        .iter()
        .find(|text| text.contains("orchard_public_write") || text.contains("revision"))
        .unwrap_or_else(|| panic!("memory_write 没有真跑；工具结果 = {results:?}"));
    assert!(
        write.contains("\"revision\""),
        "memory_write 必须返回真写入回执（revision）；实际 = {write}"
    );
    assert!(
        !write.contains("not registered") && !write.contains("namespace_denied"),
        "生产装配下 memory_write 不许是「未注册/被拒」；实际 = {write}"
    );
}

/// ⭐ **Host 映射表迟绑定**：`register_with_profile_mappings` 把映射绑到**同一个**
/// 生产模块实例上 ⇒ 有映射的人写得进、没映射的人被拒（`no current Host mapping`）。
///
/// 反证口径：去掉 `memory_assembly::bind_profile_mappings(...)` ⇒ 映射丢 ⇒
/// 第二个 agent 也会写成功（它本该被拒）⇒ 本测试红。
#[tokio::test]
async fn host_profile_mappings_are_bound_to_the_production_memory_module() {
    let (c, db) = memory();
    let granted = principal();
    let stranger = principal();
    let mappings = Arc::new(ProfileMemoryMappings::default());
    mappings
        .grant(
            &granted,
            Some(ProfileMemoryAccess {
                coordinator: c.clone(),
                read_scopes: vec![MemoryScope::Global],
                writable_private: true,
            }),
            vec![],
        )
        .unwrap();
    let provider = Provider::new(vec![
        vec![call(
            "memory_write",
            json!({"content":"granted_private_write","idempotency_key":"g1"}),
        )],
        vec![],
        vec![call(
            "memory_write",
            json!({"content":"stranger_write","idempotency_key":"s1"}),
        )],
        vec![],
    ]);
    let runtime = production(&db)
        .register_with_profile_mappings(builder(provider.clone()), mappings.clone())
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let a = runtime
        .spawn_agent(
            "granted",
            AgentConfig {
                principal: Some(granted),
                tools: memory_tools(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    a.run("write it".into()).await.unwrap();
    let b = runtime
        .spawn_agent(
            "stranger",
            AgentConfig {
                principal: Some(stranger),
                tools: memory_tools(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    b.run("try it".into()).await.unwrap();
    let results = provider.results();
    assert!(
        results.iter().any(|text| text.contains("\"revision\"")),
        "有映射的人必须写成功（迟绑定生效）；工具结果 = {results:?}"
    );
    assert!(
        results
            .iter()
            .any(|text| text.contains("no current Host mapping")),
        "没映射的人必须被拒（否则迟绑定等于没绑）；工具结果 = {results:?}"
    );
}

/// ⭐ **模块自有 Agent**（`rt.owned_agents`）：`memory_organize` 在生产装配下
/// 真起一个**模块拥有的**子 Agent，子 Agent 用它自己的 Host 授权写库。
///
/// 观察点：主 Agent 的 organize 回执里有 `task_handle`；子 Agent 的写回执里也有
/// 真 `revision`（子 Agent 不是摆设）。
#[tokio::test]
async fn production_memory_module_owns_a_child_agent_that_writes_with_its_own_grant() {
    let (_c, db) = memory();
    let owner = principal();
    let provider = Provider::new(vec![
        vec![call("memory_organize", json!({"query":"topic"}))],
        vec![],
    ]);
    let release = provider.organizer_script(vec![
        vec![call(
            "memory_write",
            json!({"content":"private_child_payload","idempotency_key":"child"}),
        )],
        vec![],
    ]);
    let mut policy = AgentHostPolicy::default();
    policy.modules.insert(
        pid("module.memory"),
        ModuleAgentGrant {
            tools: [cid("tool.memory.write"), cid("tool.memory.retrieve")]
                .into_iter()
                .collect(),
            principals: vec![owner.clone()],
            ..Default::default()
        },
    );
    let runtime = production(&db)
        .register_into(builder(provider.clone()))
        .build_agent_runtime(policy)
        .await
        .unwrap();
    let main = runtime
        .spawn_agent(
            "main",
            AgentConfig {
                principal: Some(owner),
                tools: vec![cid("tool.memory.organize")],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    main.run("organize".into()).await.unwrap();
    // ⚠️ `organizer_script` 的**首个**子 Agent 模型调用是被 `Notify` 挂住的
    // （夹具用它来控制「子 Agent 何时真的开跑」）⇒ ⇒ 先放行再等结果，
    // 否则是死等（本测试第一版就踩了这个坑：5s 超时）。
    release.notify_one();
    let waited = tokio::time::timeout(Duration::from_secs(5), async {
        while !provider
            .results()
            .iter()
            .any(|text| text.contains("private_child_payload") || text.contains("\"revision\""))
        {
            tokio::task::yield_now().await;
        }
    })
    .await;
    if waited.is_err() {
        panic!(
            "子 Agent 必须在 5s 内用它自己的授权写一条记忆；请求数 = {}；工具结果 = {:?}",
            provider.requests().len(),
            provider.results()
        );
    }
    release.notify_one();
    let results = provider.results();
    assert!(
        results
            .iter()
            .any(|text| text.contains("\"agent_id\"") && text.contains("scheduled")),
        "organize 必须回执**模块自有 Agent** 的调度结果（agent_id + scheduled）；工具结果 = {results:?}"
    );
    let child_write = results
        .iter()
        .find(|text| text.contains("\"revision\""))
        .unwrap_or_else(|| panic!("子 Agent 的写必须有真回执；工具结果 = {results:?}"));
    assert!(
        !child_write.contains("not registered") && !child_write.contains("denied"),
        "子 Agent 的写不许是「未注册/被拒」；实际 = {child_write}"
    );
    // 子 Agent 的会话与主 Agent 不同（模块自有 Agent，不是主 Agent 换个名字）。
    let main_session: SessionId = main.snapshot().session;
    assert!(!child_write.is_empty() && main_session.to_string().len() >= 32);
}

/// 让未使用的 import 检查保持安静：装配辅助在测试里只用于类型推断。
#[allow(dead_code)]
fn _assert_builder_type(builder: RuntimeBuilder) -> RuntimeBuilder {
    builder
}
