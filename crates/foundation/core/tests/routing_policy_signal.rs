//! 隐私**信号口**的行为钉死 (W2 · 单元 C)。
//!
//! 覆盖面:
//!
//! 1. 信号缺失 ⇒ 普通档 + **候选集与上轮逐条相同** (零行为变化对照);
//! 2. restricted ⇒ 只能落到**可承接受限档**的候选 (不合格者被排除且**理由可见**);
//! 3. confidential ⇒ 更严一档;
//! 4. 无合格候选 ⇒ **明确失败** (不回退默认模型);
//! 5. 信号缺失 ⇒ 按 ordinary 处理且**显式标注** (不静默);
//! 6. 显式声明但值不认识 ⇒ **拒路由** (不静默降级);
//! 7. 对抗: 声明 confidential 但候选不支持 ⇒ 越级声明无效;
//! 8. 每个被排除候选都带**理由**;
//! 9. 信号口读取 (保留键 / trim / 大小写不敏感 / 缺位);
//! 10. 负向用例: 候选表 = 画像表真子集 ⇒ 选出的模型**可与成本档层不同** (候选表闸
//!    的策略语义, 非语义漂移);
//! 11. 退路排序**不等于**选定阶梯顺序 (措辞钉死)。

use std::collections::BTreeMap;

use apeireth_core::cost_router::{CostRouter, CostRoutingConfig, CostTier};
use apeireth_core::model_profile::{ModelProfile, ModelProfileTable};
use apeireth_core::routing_policy::{
    privacy_signal_from_metadata, privacy_signal_from_str, CandidateOutcome, CapabilityRequirement,
    PolicyCandidate, PrivacyLevel, PrivacySignal, PrivacySignalError, PrivacySource, RoutingPolicy,
    RoutingPolicyError, TaskProfile, PRIVACY_SIGNAL_KEY,
};

fn profile(
    model_id: &str,
    supports_tools: Option<bool>,
    context_window: Option<u64>,
    input_price_micros: Option<u64>,
    output_price_micros: Option<u64>,
) -> ModelProfile {
    ModelProfile {
        model_id: model_id.to_string(),
        context_window,
        max_output: Some(8_192),
        supports_tools,
        supports_streaming: Some(true),
        input_price_micros,
        output_price_micros,
        cache_read_price_micros: None,
        capabilities_flags: Vec::new(),
    }
}

/// 画像表: 三个有价行 (单价 2_000 / 6_000 / 20_000 合计口径) + 一个无价行。
fn table() -> ModelProfileTable {
    ModelProfileTable::new(vec![
        profile(
            "cheap-model",
            Some(true),
            Some(65_536),
            Some(1_000),
            Some(1_000),
        ),
        profile(
            "work-model",
            Some(true),
            Some(32_768),
            Some(3_000),
            Some(3_000),
        ),
        profile(
            "deep-model",
            Some(true),
            Some(131_072),
            Some(10_000),
            Some(10_000),
        ),
        profile("unpriced-model", Some(true), Some(131_072), None, None),
    ])
}

fn policy(
    candidates: Vec<PolicyCandidate>,
    profiles: ModelProfileTable,
    default_model: Option<&str>,
) -> RoutingPolicy {
    RoutingPolicy::new(
        candidates,
        profiles,
        default_model.map(str::to_string),
        CostRoutingConfig {
            enabled: true,
            preferred_models: BTreeMap::new(),
        },
    )
}

/// 三档隐私各就各位的候选表。
fn graded_candidate_table() -> Vec<PolicyCandidate> {
    vec![
        PolicyCandidate::new("cheap-model", PrivacyLevel::Ordinary),
        PolicyCandidate::new("work-model", PrivacyLevel::Restricted),
        PolicyCandidate::new("deep-model", PrivacyLevel::Confidential),
        PolicyCandidate::new("unpriced-model", PrivacyLevel::Ordinary),
    ]
}

/// 标准档 + 需工具的能力需求。
fn capability() -> CapabilityRequirement {
    CapabilityRequirement::new().with_requires_tools(true)
}

/// 标准档任务画像 (隐私信号由参数给)。
fn task(signal: PrivacySignal) -> TaskProfile {
    TaskProfile::from_signal(capability(), Some(CostTier::Standard), signal)
}

#[test]
fn a_missing_signal_is_ordinary_and_leaves_the_candidate_set_row_by_row_unchanged() {
    // 零行为变化对照: 信号缺位 ⇒ 普通档 ⇒ **候选集与上轮逐条相同**。
    let candidates = graded_candidate_table();
    let defaulted = policy(candidates.clone(), table(), Some("work-model"))
        .decide(&task(PrivacySignal::Defaulted))
        .expect("缺省兜底可选路");
    let declared = policy(candidates, table(), Some("work-model"))
        .decide(&task(PrivacySignal::Declared(PrivacyLevel::Ordinary)))
        .expect("显式普通档可选路");

    assert_eq!(defaulted.privacy, PrivacyLevel::Ordinary);
    assert_eq!(declared.privacy, PrivacyLevel::Ordinary);
    assert_eq!(
        defaulted.selected_model, declared.selected_model,
        "缺省兜底与显式普通档选出同一个模型"
    );
    assert_eq!(defaulted.fallback_chain, declared.fallback_chain);
    assert_eq!(
        defaulted.comparisons, declared.comparisons,
        "逐行比较逐条相同 (模型 id / 行状态 / 排除理由 / 单价 / 是否参与比较)"
    );
}

#[test]
fn a_missing_signal_is_annotated_explicitly_and_never_silently() {
    let policy = policy(graded_candidate_table(), table(), Some("work-model"));
    let decision = policy
        .decide(&task(PrivacySignal::Defaulted))
        .expect("缺省兜底可选路");

    assert_eq!(
        decision.privacy_source,
        PrivacySource::Defaulted,
        "来源必须看得见"
    );
    assert!(
        decision.selection_reason.contains("缺省兜底"),
        "缺省兜底要在选定理由里写明: {}",
        decision.selection_reason
    );
    assert!(
        !decision.selection_reason.contains("显式声明"),
        "缺省兜底不得自称显式声明: {}",
        decision.selection_reason
    );

    // 显式声明普通档 ⇒ 来源是 declared, 措辞也换过来。
    let declared = policy
        .decide(&task(PrivacySignal::Declared(PrivacyLevel::Ordinary)))
        .expect("显式普通档可选路");
    assert_eq!(declared.privacy_source, PrivacySource::Declared);
    assert!(
        declared.selection_reason.contains("显式声明"),
        "{}",
        declared.selection_reason
    );
}

#[test]
fn a_restricted_signal_only_reaches_candidates_that_can_take_restricted_data() {
    let policy = policy(graded_candidate_table(), table(), Some("work-model"));
    let decision = policy
        .decide(&task(PrivacySignal::Declared(PrivacyLevel::Restricted)))
        .expect("受限档可选路");

    assert_eq!(
        decision.selected_model, "work-model",
        "只有可承接受限档的候选才合格"
    );
    let cheap = decision
        .comparisons
        .iter()
        .find(|row| row.model_id == "cheap-model")
        .expect("被排除的行在场");
    assert_eq!(cheap.outcome, CandidateOutcome::Excluded);
    let reason = cheap.exclusion_reason.as_deref().expect("排除理由可见");
    assert!(reason.contains("隐私等级 restricted"), "{reason}");
    assert!(reason.contains("上限 ordinary"), "{reason}");
    assert!(
        !decision.fallback_chain.contains(&"cheap-model".to_string()),
        "越闸候选连退路都不许留"
    );
}

#[test]
fn a_confidential_signal_is_stricter_than_a_restricted_one() {
    let policy = policy(graded_candidate_table(), table(), Some("work-model"));
    let decision = policy
        .decide(&task(PrivacySignal::Declared(PrivacyLevel::Confidential)))
        .expect("机密档可选路");

    assert_eq!(
        decision.selected_model, "deep-model",
        "机密档只落到可承接机密档的候选"
    );
    for excluded in ["cheap-model", "work-model", "unpriced-model"] {
        let row = decision
            .comparisons
            .iter()
            .find(|row| row.model_id == excluded)
            .expect("被排除的行在场");
        assert_eq!(row.outcome, CandidateOutcome::Excluded, "{excluded}");
        assert!(
            row.exclusion_reason
                .as_deref()
                .is_some_and(|reason| reason.contains("隐私等级 confidential")),
            "{excluded} 的排除理由必须点名隐私闸: {:?}",
            row.exclusion_reason
        );
    }
}

#[test]
fn a_signal_without_any_qualified_candidate_fails_instead_of_falling_back() {
    // 所有候选只承接普通档, 机密档任务 ⇒ 无合格候选 ⇒ 明确失败 (不回退默认模型)。
    let candidates = vec![
        PolicyCandidate::new("cheap-model", PrivacyLevel::Ordinary),
        PolicyCandidate::new("work-model", PrivacyLevel::Ordinary),
    ];
    let policy = policy(candidates, table(), Some("work-model"));
    let error = policy
        .decide(&task(PrivacySignal::Declared(PrivacyLevel::Confidential)))
        .expect_err("无合格候选 ⇒ 明确失败");

    match &error {
        RoutingPolicyError::NoEligibleCandidate {
            tier,
            privacy,
            privacy_source,
            reason,
            comparisons,
            ..
        } => {
            assert_eq!(*tier, CostTier::Standard);
            assert_eq!(*privacy, PrivacyLevel::Confidential);
            assert_eq!(*privacy_source, PrivacySource::Declared);
            assert!(reason.contains("全部被策略层排除"), "{reason}");
            assert!(
                comparisons
                    .iter()
                    .all(|row| row.outcome == CandidateOutcome::Excluded),
                "错误帧里没有「过了闸」的行 (行状态与「已被闸排除」一致)"
            );
            assert!(
                comparisons.iter().all(|row| row.exclusion_reason.is_some()),
                "每一行都带排除理由"
            );
        }
        other => panic!("预期 NoEligibleCandidate, 实得 {other:?}"),
    }
    assert!(
        !error.to_string().contains("work-model"),
        "默认模型不得被当成回退目标写进错误帧: {error}"
    );
}

#[test]
fn an_unknown_declared_value_is_refused_instead_of_downgraded() {
    // 显式声明了但看不懂 ⇒ 拒路由 (fail-closed): 静默降级成普通档 = 静默放宽。
    let error = privacy_signal_from_str(Some("secret")).expect_err("不认识的档 ⇒ 报错");
    match &error {
        PrivacySignalError::InvalidValue { key, reason } => {
            assert_eq!(key, PRIVACY_SIGNAL_KEY);
            assert!(reason.contains("ordinary"), "{reason}");
            assert!(
                !error.to_string().contains("secret"),
                "值绝不能进错误帧: {error}"
            );
        }
    }

    // 空白 ⇒ 缺位 ⇒ 兜底 (不是「声明了但看不懂」)。
    assert_eq!(
        privacy_signal_from_str(Some("   ")).expect("空白 = 缺位"),
        PrivacySignal::Defaulted
    );
    assert_eq!(
        privacy_signal_from_str(None).expect("缺位 = 兜底"),
        PrivacySignal::Defaulted
    );
}

#[test]
fn the_adversarial_case_a_confidential_declaration_cannot_reach_an_ordinary_candidate() {
    // 对抗: 调用方声明机密档, 但候选只支持普通档 ⇒ 越级声明**无效** ⇒ 明确失败。
    let candidates = vec![PolicyCandidate::new("cheap-model", PrivacyLevel::Ordinary)];
    let policy = policy(candidates, table(), Some("work-model"));
    let error = policy
        .decide(&task(PrivacySignal::Declared(PrivacyLevel::Confidential)))
        .expect_err("越级声明 ⇒ 明确失败");

    assert!(
        matches!(error, RoutingPolicyError::NoEligibleCandidate { .. }),
        "实得 {error:?}"
    );
}

#[test]
fn the_signal_port_reads_the_reserved_metadata_key() {
    // 端口: 保留键 + 大小写不敏感 + 去首尾空白 + 缺位兜底。
    let mut metadata = BTreeMap::new();
    metadata.insert(PRIVACY_SIGNAL_KEY.to_string(), " Restricted ".to_string());
    assert_eq!(
        privacy_signal_from_metadata(&metadata).expect("受限档"),
        PrivacySignal::Declared(PrivacyLevel::Restricted)
    );

    let mut upper = BTreeMap::new();
    upper.insert(PRIVACY_SIGNAL_KEY.to_string(), "CONFIDENTIAL".to_string());
    assert_eq!(
        privacy_signal_from_metadata(&upper).expect("机密档"),
        PrivacySignal::Declared(PrivacyLevel::Confidential)
    );

    assert_eq!(
        privacy_signal_from_metadata(&BTreeMap::new()).expect("缺位"),
        PrivacySignal::Defaulted
    );

    // 信号口的等级与闸门读的是同一个等级 (接线一致性)。
    let signal = privacy_signal_from_metadata(&metadata).expect("受限档");
    assert_eq!(signal.level(), PrivacyLevel::Restricted);
    assert!(signal.is_declared());
    assert_eq!(signal.source(), PrivacySource::Declared);
}

#[test]
fn every_excluded_candidate_carries_a_reason() {
    let policy = policy(graded_candidate_table(), table(), Some("work-model"));
    let decision = policy
        .decide(&task(PrivacySignal::Declared(PrivacyLevel::Confidential)))
        .expect("机密档可选路");

    for row in &decision.comparisons {
        match row.outcome {
            CandidateOutcome::Excluded => {
                assert!(
                    row.exclusion_reason.is_some(),
                    "{} 被排除却没有理由",
                    row.model_id
                );
            }
            CandidateOutcome::Selected | CandidateOutcome::Eligible => {
                assert!(
                    row.exclusion_reason.is_none(),
                    "{} 没被排除却有排除理由",
                    row.model_id
                );
            }
        }
    }
}

#[test]
fn a_candidate_table_that_is_a_proper_subset_may_select_a_different_model() {
    // 负向用例 (零行为变化的成立条件): 候选表 = 画像表真子集 ⇒ 幸存集 =
    // 候选表 ∩ 成本候选集 ⇒ 选出的模型**可与成本档层不同** (候选表闸的策略语义,
    // 不是语义漂移)。普通档下隐私闸恒不过滤, 但候选表闸仍在。
    let profiles = table();
    let cost_router = CostRouter::new(
        profiles.clone(),
        Some("work-model".to_string()),
        CostRoutingConfig {
            enabled: true,
            preferred_models: BTreeMap::new(),
        },
    );
    let cost_decision = cost_router
        .route(
            &apeireth_core::cost_router::RoutingRequest::new()
                .with_explicit_tier(CostTier::Standard)
                .with_requires_tools(true),
        )
        .expect("成本档层可路由");

    let policy = policy(
        vec![PolicyCandidate::new("work-model", PrivacyLevel::Ordinary)],
        profiles,
        Some("work-model"),
    );
    let decision = policy
        .decide(&task(PrivacySignal::Declared(PrivacyLevel::Ordinary)))
        .expect("子集候选表可选路");

    assert_eq!(
        cost_decision.selected_model.as_deref(),
        Some("cheap-model"),
        "成本档层在完整画像表上选最便宜的"
    );
    assert_eq!(
        decision.selected_model, "work-model",
        "策略层的候选表闸把它挡在外面 ⇒ 选出不同模型 (候选表闸的策略语义)"
    );
    let cheap = decision
        .comparisons
        .iter()
        .find(|row| row.model_id == "cheap-model")
        .expect("未登记的行也在逐行比较里");
    assert_eq!(cheap.outcome, CandidateOutcome::Excluded);
    assert!(
        cheap
            .exclusion_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("未登记在策略候选表")),
        "排除理由要点名候选表闸: {:?}",
        cheap.exclusion_reason
    );
}

#[test]
fn the_fallback_order_is_explicitly_not_the_selection_ladder() {
    let policy = policy(graded_candidate_table(), table(), Some("work-model"));
    let decision = policy
        .decide(&task(PrivacySignal::Declared(PrivacyLevel::Ordinary)))
        .expect("可选路");

    let fallback_reason = decision.fallback_reason.as_deref().expect("退路说明真值");
    assert!(
        fallback_reason.contains("不等于") && fallback_reason.contains("选定阶梯"),
        "退路说明必须写明「退路排序 ≠ 选定阶梯顺序」: {fallback_reason}"
    );
    assert!(
        fallback_reason.contains("单价升序"),
        "退路说明要写明自己的排序口径: {fallback_reason}"
    );
}
