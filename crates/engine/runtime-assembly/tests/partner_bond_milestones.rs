//! 关系里程碑在**羁绊模块里真的会被记下来**（不只是库级可用）。
//!
//! # 这条测试补的是什么
//!
//! `milestone` 模块早就有契约与内存实现，`MilestoneRecorder` 也有单测 ——
//! 但那证明的是「函数能跑」，不是「生产路径会走到它」。
//!
//! 本测试从**行为模块的 hook** 出发（`TurnStart` / `AfterTurn`），断言：
//!
//! 1. `TurnStart` 记「初见」，且**每回合都调也只记一条**（幂等）；
//! 2. `AfterTurn` 演化跨过阶段阈值 ⇒ 记一条「阶段跃迁」（带 from/to）；
//! 3. 没注入里程碑存储时 ⇒ 一条都不记（零行为变化）。

use std::sync::Arc;

use apeireth_core::clock::SystemClock;
use apeireth_core::kernel::CapabilityId;
use apeireth_core::kernel::SessionId;
use apeireth_memory::milestone::{
    InMemoryMilestoneStore, MilestoneKind, MilestonePayload, MilestoneRecorder, MilestoneStore,
};
use apeireth_memory::partner::InMemoryPartnerStore;
use apeireth_protocol::canonical::{NormalizedMessage, NormalizedResponse};
use apeireth_runtime_assembly::canonical::cognitive::PartnerBondModule;
use apeireth_runtime_assembly::canonical::module::{
    BehaviorModule, HookPoint, InvocationContext, ModuleContext, ModuleInvocationError,
    ModuleInvocationRequest, ModuleInvocationResponse, ModuleInvoker, ModuleOutcome,
};
use apeireth_runtime_assembly::canonical::subloop::{
    SubLoopError, SubLoopResult, SubLoopSpawner, SubLoopSpec,
};
use async_trait::async_trait;

struct NoSubLoop;

#[async_trait]
impl SubLoopSpawner for NoSubLoop {
    async fn spawn(&self, _spec: SubLoopSpec) -> Result<SubLoopResult, SubLoopError> {
        Err(SubLoopError::NoModel)
    }
}

static NO_SUB_LOOP: NoSubLoop = NoSubLoop;

struct NoopInvoker;

#[async_trait]
impl ModuleInvoker for NoopInvoker {
    async fn invoke(
        &self,
        _request: ModuleInvocationRequest,
    ) -> Result<ModuleInvocationResponse, ModuleInvocationError> {
        Ok(ModuleInvocationResponse {
            response: NormalizedResponse::text("assistant", "test-model", ""),
            served_by: CapabilityId::new("provider.test").expect("capability id"),
        })
    }
}

async fn run(module: &PartnerBondModule, hook: HookPoint, session: &SessionId) -> ModuleOutcome {
    let invocation = InvocationContext::user_turn();
    let invoker: Arc<dyn ModuleInvoker> = Arc::new(NoopInvoker);
    let messages: Vec<NormalizedMessage> = Vec::new();
    let ctx = ModuleContext {
        session_id: session,
        model: "test-model",
        messages: &messages,
        candidate: None,
        tool_call: None,
        tool_result: None,
        invocation: &invocation,
        module_id: "module.cognitive.partner_bond",
        error: None,
        invoker: &*invoker,
        invoker_handle: Arc::clone(&invoker),
        subloop: &NO_SUB_LOOP,
    };
    module.on_hook(hook, &ctx).await.expect("hook 不应失败")
}

fn module(depth_increment: f64) -> (PartnerBondModule, Arc<dyn MilestoneStore>) {
    let milestones: Arc<dyn MilestoneStore> = Arc::new(InMemoryMilestoneStore::new());
    let module = PartnerBondModule::new(
        Arc::new(InMemoryPartnerStore::new()),
        "test-user",
        Arc::new(SystemClock),
    )
    .with_depth_increment(depth_increment)
    .with_milestones(Arc::new(MilestoneRecorder::new(Arc::clone(&milestones))));
    (module, milestones)
}

/// `TurnStart` 记初见，且重复调用**不会**写出第二条。
#[tokio::test]
async fn turn_start_records_a_single_first_meeting() {
    let (module, milestones) = module(0.0);
    let session = SessionId::new();

    // 三个回合：真实装配每回合都会走 TurnStart
    for _ in 0..3 {
        run(&module, HookPoint::TurnStart, &session).await;
    }

    let list = milestones
        .query(&session, Some(MilestoneKind::FirstMeeting))
        .expect("query");
    assert_eq!(list.len(), 1, "初见只能有一条: {list:?}");
    assert!(matches!(&list[0].payload, MilestonePayload::Text(_)));
}

/// 阶段跃迁跨阈值 ⇒ 记一条带 from/to 的里程碑。
///
/// 深度增量 0.5 一次就把 Initial(0) 推过 0.40(Trusted) 与 0.15(Familiar) 的阈值 ——
/// 阶段从 Initial 直接变到 Trusted，记一条。
#[tokio::test]
async fn after_turn_records_a_stage_transition() {
    let (module, milestones) = module(0.5);
    let session = SessionId::new();

    run(&module, HookPoint::AfterTurn, &session).await;

    let transitions = milestones
        .query(&session, Some(MilestoneKind::StageTransition))
        .expect("query");
    assert_eq!(transitions.len(), 1, "应记一条阶段跃迁: {transitions:?}");
    match &transitions[0].payload {
        MilestonePayload::Stage { from, to } => {
            assert_eq!(from, "Initial", "起始阶段");
            assert_ne!(to, "Initial", "必须真的跃迁了: {to}");
        }
        other => panic!("阶段跃迁载荷应为 Stage: {other:?}"),
    }
}

/// 阶段没变（增量 0）⇒ **不记**噪声。
#[tokio::test]
async fn after_turn_without_a_stage_change_records_nothing() {
    let (module, milestones) = module(0.0);
    let session = SessionId::new();

    for _ in 0..3 {
        run(&module, HookPoint::AfterTurn, &session).await;
    }

    let transitions = milestones
        .query(&session, Some(MilestoneKind::StageTransition))
        .expect("query");
    assert!(
        transitions.is_empty(),
        "阶段没变不该记里程碑: {transitions:?}"
    );
}

/// 未注入里程碑存储 ⇒ 一条都不记（缺注入 = 零行为变化）。
#[tokio::test]
async fn without_a_milestone_store_nothing_is_recorded() {
    let milestones: Arc<dyn MilestoneStore> = Arc::new(InMemoryMilestoneStore::new());
    // 注意：**不**调 with_milestones
    let module = PartnerBondModule::new(
        Arc::new(InMemoryPartnerStore::new()),
        "test-user",
        Arc::new(SystemClock),
    )
    .with_depth_increment(0.5);
    let session = SessionId::new();

    run(&module, HookPoint::TurnStart, &session).await;
    run(&module, HookPoint::AfterTurn, &session).await;

    assert_eq!(
        milestones.query(&session, None).expect("query").len(),
        0,
        "未注入存储时不得写任何里程碑"
    );
}
