//! W2 · 单元 D 的行为证据套件:
//!
//! * **C-H2 深度维** —— 自述面的深度格不再写「暂无接口」: 上限是真值, 已耗
//!   读不到时**显式 null + 原因**（不报 0）; 上限大于当前调用树可达深度时
//!   **必须**带口径注记（否则看着在计量、实际是死的）。
//! * **C-H1 路由策略层** —— 自述面新增一节: 策略层开关 + 最近一次决策真值
//!   （含**排除理由**）; 信号来源（declared / defaulted）**露出来** ⇒ 缺省
//!   必须看起来像缺省。
//! * **C-D1 对象授权面的加固** —— 主体键归一 + 调用者校验（可见性矩阵 oracle
//!   的封口）+ 撤销归属校验 + 审计守卫。
//!
//! 纪律: 只测**已接线**的事实; 未接线的面（如对象授权面的生产激活口）**不**
//! 在本文件里假装已接线。

use std::sync::Arc;

use apeireth_core::routing_policy::{
    PolicyCandidate, PrivacyLevel, PrivacySignal, RoutingPolicy, TaskProfile,
};
use apeireth_runtime_assembly::canonical::object_grant::{
    normalize_subject_key, SUBJECT_SHAPE_REFUSED,
};
use apeireth_runtime_assembly::canonical::self_status_source::{
    budget_status_from_configured, routing_policy_probe,
};
use apeireth_tools_canonical::self_status::{
    QuotaDimensionStatus, QUOTA_DEPTH_NOT_ENABLED, QUOTA_DEPTH_READING_MISSING,
};

// ---------------------------------------------------------------------------
// C-H2 深度维: 上限是真值 + 口径注记 + 未启用照实说
// ---------------------------------------------------------------------------

#[test]
fn the_depth_dimension_is_off_unless_a_ceiling_is_configured() {
    let budget = budget_status_from_configured(8, 16, None, None, None, None);
    assert_eq!(
        budget.quota.depth.limit, None,
        "深度维未设上限 ⇒ 上限不显示成数字"
    );
    assert_eq!(budget.quota.depth.consumed, None, "未启用 ⇒ 已耗不编造");
    assert_eq!(
        budget.quota.depth.reason.as_deref(),
        Some(QUOTA_DEPTH_NOT_ENABLED),
        "未启用 ⇒ 如实标「未启用」, 不装成在计量"
    );
    assert_eq!(
        budget.quota.depth.display, "—",
        "未启用 ⇒ 整行显「—」(未接线 / 不适用), 不显示成真数字"
    );
}

#[test]
fn a_configured_depth_ceiling_carries_its_calibration_note() {
    let budget = budget_status_from_configured(8, 16, None, None, None, Some(8));
    assert_eq!(budget.quota.depth.limit, Some(8), "上限是真值");
    assert_eq!(
        budget.quota.depth.consumed, None,
        "会话账本暂无深度列 ⇒ 已耗显式 null (不报 0)"
    );
    let reason = budget
        .quota
        .depth
        .reason
        .as_deref()
        .expect("深度格必须带原因");
    assert!(
        reason.contains(QUOTA_DEPTH_READING_MISSING),
        "已耗取不到 ⇒ 如实说明: {reason}"
    );
    assert!(
        reason.contains("可达深度"),
        "上限口径必须带注记（当前调用树可达深度 ≤ 1 ⇒ 更大的上限不可达）: {reason}"
    );
    assert_eq!(budget.quota.depth.display, "—/8", "取不到的一侧显「—」");
}

#[test]
fn the_depth_dimension_never_reports_zero_consumption() {
    let budget = budget_status_from_configured(8, 16, None, None, None, Some(1));
    assert_ne!(
        budget.quota.depth.consumed,
        Some(0),
        "读不到 ≠ 消耗 0 ⇒ 绝不报 0"
    );
    assert!(matches!(
        budget.quota.depth,
        QuotaDimensionStatus { consumed: None, .. }
    ));
}

// ---------------------------------------------------------------------------
// C-H1 路由策略层: 探针 + 信号来源可见 + 缺省不过滤
// ---------------------------------------------------------------------------

fn policy_with_one_model() -> RoutingPolicy {
    let profiles = apeireth_core::model_profile::ModelProfileTable::new(vec![
        apeireth_core::model_profile::ModelProfile {
            model_id: "model-a".to_string(),
            context_window: Some(32_768),
            max_output: Some(4_096),
            supports_tools: Some(true),
            supports_streaming: Some(true),
            input_price_micros: Some(1_000),
            output_price_micros: Some(2_000),
            cache_read_price_micros: None,
            capabilities_flags: Vec::new(),
        },
    ]);
    RoutingPolicy::from_env_values(
        PolicyCandidate::table_from_profiles(&profiles),
        profiles,
        Some("model-a".to_string()),
        Some("1"),
        None,
    )
    .expect("策略层装配")
}

#[test]
fn the_routing_policy_probe_reports_the_last_decision_and_its_exclusions() {
    let policy = Arc::new(policy_with_one_model());
    // 先真跑一次选路（探针读的是**策略层自己**的最近决策 ⇒ 同源）。
    policy
        .decide(&TaskProfile::from_signal(
            apeireth_core::routing_policy::CapabilityRequirement::new().with_requires_tools(true),
            None,
            PrivacySignal::Defaulted,
        ))
        .expect("决策");
    let probe = routing_policy_probe(Arc::clone(&policy));
    let status = probe().expect("探针可读");
    assert!(status.enabled, "策略层开关为开");
    let decision = status.last_decision.expect("刚决策过 ⇒ 有决策");
    assert_eq!(decision.selected_model, "model-a");
    assert_eq!(
        decision.privacy_source, "defaulted",
        "缺省必须看起来像缺省（declared / defaulted）"
    );
    assert_eq!(decision.privacy, "ordinary");
    assert!(
        !decision.comparisons.is_empty(),
        "候选逐行比较要露出来（含排除理由）"
    );
}

#[test]
fn a_disabled_policy_reports_no_decision_rather_than_an_invented_one() {
    let profiles = apeireth_core::model_profile::ModelProfileTable::new(vec![
        apeireth_core::model_profile::ModelProfile {
            model_id: "model-a".to_string(),
            context_window: Some(32_768),
            max_output: Some(4_096),
            supports_tools: Some(true),
            supports_streaming: Some(true),
            input_price_micros: Some(1_000),
            output_price_micros: Some(2_000),
            cache_read_price_micros: None,
            capabilities_flags: Vec::new(),
        },
    ]);
    let policy = RoutingPolicy::from_env_values(
        PolicyCandidate::table_from_profiles(&profiles),
        profiles,
        Some("model-a".to_string()),
        None,
        None,
    )
    .expect("策略层装配");
    let probe = routing_policy_probe(Arc::new(policy));
    let status = probe().expect("探针可读");
    assert!(!status.enabled, "开关关 ⇒ 报关");
    assert!(
        status.last_decision.is_none(),
        "开关关 ⇒ 没有决策可报, 不编造"
    );
}

#[test]
fn the_defaulted_signal_never_filters_candidates() {
    let policy = policy_with_one_model();
    let profile = TaskProfile::from_signal(
        apeireth_core::routing_policy::CapabilityRequirement::new().with_requires_tools(true),
        None,
        PrivacySignal::Defaulted,
    );
    let decision = policy.decide(&profile).expect("决策");
    assert_eq!(decision.privacy, PrivacyLevel::Ordinary);
    assert_eq!(
        decision.privacy_source.as_str(),
        "defaulted",
        "缺省来源如实标 defaulted"
    );
    assert_eq!(decision.selected_model, "model-a");
}

// ---------------------------------------------------------------------------
// C-D1 对象授权面: 主体键归一（形状 + trim 口径统一）
// ---------------------------------------------------------------------------

#[test]
fn the_subject_key_is_normalized_before_it_reaches_the_ledger() {
    let key = normalize_subject_key("  profile:11111111-1111-4111-8111-111111111111/namespace:22222222-2222-4222-8222-222222222222  ")
        .expect("命名空间键形态接受");
    assert_eq!(
        key,
        "profile:11111111-1111-4111-8111-111111111111/namespace:22222222-2222-4222-8222-222222222222",
        "trim 后落账 ⇒ 同一主体的两种写法不再分家"
    );
    assert_eq!(
        normalize_subject_key("  ns-alice  ").expect("短键接受"),
        "ns-alice",
        "短键形态也要 trim"
    );
    assert_eq!(
        normalize_subject_key("   ").unwrap_err(),
        "subject must not be empty",
        "空主体拒绝"
    );
    assert_eq!(
        normalize_subject_key("profile:11111111-1111-4111-8111-111111111111/namespace:zz")
            .unwrap_err(),
        SUBJECT_SHAPE_REFUSED,
        "形状不合 ⇒ 统一文案 (值自由报错)"
    );
    assert_eq!(
        normalize_subject_key("namespace:22222222-2222-4222-8222-222222222222/profile:11111111-1111-4111-8111-111111111111")
            .unwrap_err(),
        SUBJECT_SHAPE_REFUSED,
        "段序颠倒 ⇒ 拒绝 (否则同一主体两种写法)"
    );
}

// ---------------------------------------------------------------------------
// C-D1 加固: 模型面必须校验调用主体（可见性矩阵 oracle 的封口）
// ---------------------------------------------------------------------------

/// 加固用的最小台架：一个私有会话域 + 一个模型面上下文。
mod grant_harness {
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use apeireth_core::kernel::{
        AgentId, Clock, NamespaceId, PluginId, ProfileId, SessionId, SystemClock, TraceId,
    };
    use apeireth_memory::{MemoryCoordinator, MemoryProvenance, MemoryScope, SqliteMemoryStore};
    use apeireth_plugin::agents::AgentPrincipal;
    use apeireth_plugin::task_scope::ModuleTaskScope;
    use apeireth_plugin::{ToolAuthorization, ToolCaller, ToolCancellation, ToolExecutionContext};

    use apeireth_runtime_assembly::canonical::object_grant::{
        object_grant_registry, MemoryGrantResolver, ObjectGrantService, ObjectGrantTool,
    };

    /// 被授主体。
    pub const SUBJECT: &str = "ns-subject";

    pub struct Harness {
        pub service: Arc<ObjectGrantService>,
        pub tool: ObjectGrantTool,
    }

    pub fn harness() -> Harness {
        let db = Arc::new(SqliteMemoryStore::open_in_memory().expect("in-memory store"));
        let coordinator = Arc::new(
            MemoryCoordinator::new(db.clone(), db.clone()).with_scoped_backend(db.clone()),
        );
        coordinator
            .writeback_scoped_episode(
                &apeireth_core::kernel::Episode {
                    id: "ep-1".to_string(),
                    timestamp: SystemClock.now().timestamp(),
                    role: "user".to_string(),
                    content: "memory owned by subject".to_string(),
                    session_id: SUBJECT.to_string(),
                },
                &MemoryScope::Session {
                    session_id: SUBJECT.to_string(),
                },
                &MemoryProvenance {
                    source_session: Some(SUBJECT.to_string()),
                    ..Default::default()
                },
            )
            .expect("seed episode");
        let resolver = Arc::new(MemoryGrantResolver::new(coordinator.clone()));
        let registry = object_grant_registry(None, resolver).expect("in-memory ledger");
        let service = Arc::new(ObjectGrantService::with_default_clock(registry));
        let tool = ObjectGrantTool::new(Arc::clone(&service));
        Harness { service, tool }
    }

    pub fn context(caller: ToolCaller) -> ToolExecutionContext {
        ToolExecutionContext {
            agent: AgentId::default(),
            principal: Some(AgentPrincipal {
                profile: ProfileId::new(),
                namespace: NamespaceId::new(),
            }),
            session: None,
            turn: None,
            call_id: "call_1".to_string(),
            caller,
            module: PluginId::new("test.object_grant").expect("valid plugin id"),
            module_instance: None,
            internal_review: None,
            message_support: false,
            message_route: None,
            delegated_tools: None,
            delegation: None,
            tool_scope: None,
            authorization: ToolAuthorization::default(),
            cancellation: ToolCancellation::default(),
            deadline: Instant::now() + Duration::from_secs(30),
            trace: None,
            module_tasks: ModuleTaskScope::default(),
            resource_lease: None,
        }
    }

    /// 宿主自己的主体键（`namespace_key` 口径）。
    pub fn host_key() -> String {
        apeireth_runtime_assembly::canonical::memory_module::namespace_key(&AgentPrincipal {
            profile: ProfileId::new(),
            namespace: NamespaceId::new(),
        })
    }
}

use apeireth_plugin::{ToolCaller, ToolCapability};
use apeireth_protocol::canonical::{ToolCall, ToolOutcome};
use serde_json::json;

#[tokio::test]
async fn the_model_face_cannot_probe_someone_elses_visibility_matrix() {
    let h = grant_harness::harness();
    // 宿主给被授主体登一条授权并落章。
    let entry = h
        .service
        .request_grant(
            &grant_harness::host_key(),
            "ep-1",
            grant_harness::SUBJECT,
            None,
        )
        .expect("授权请求");
    h.service
        .approve_grant("owner", &entry.grant_id)
        .expect("人批通过");

    let call = ToolCall {
        id: "call_check".to_string(),
        name: "object_grant".to_string(),
        arguments: json!({
            "action": "check",
            "subject": grant_harness::SUBJECT,
            "object_id": "ep-1",
        }),
    };
    // **模型面**: 调用者既不是主体本人、也不是该授权的授予人 ⇒ 拒绝。
    let refused = h
        .tool
        .invoke_with_context(&call, None, &grant_harness::context(ToolCaller::Model))
        .await;
    assert!(
        matches!(refused.outcome, ToolOutcome::Error { .. }),
        "模型面探测他人可见性 ⇒ 必须拒绝（可见性矩阵 oracle）"
    );

    // **宿主面**: 面板的落章口 ⇒ 放行（宿主才是 owner）。
    let allowed = h
        .tool
        .invoke_with_context(&call, None, &grant_harness::context(ToolCaller::Host))
        .await;
    assert!(
        matches!(allowed.outcome, ToolOutcome::Ok { .. }),
        "宿主面照常可查（审批面板的落章口）"
    );
}
