//! 统一路由策略层 (`routing_policy`) 的行为钉死。
//!
//! 覆盖面 (每条一个可解释的决策语义):
//!
//! 1. 隐私词表闭合 (三档阶梯);
//! 2. 正常选路 (成本档 → 候选 → 选定 + 退路链);
//! 3. **与 `CostRouter` 口径一致** (同一档位下选出的模型一致);
//! 4. 隐私排除 (**排除理由可见**, 不静默过滤);
//! 5. **对抗用例**: 伪造「高隐私 + 低价」冲突候选 ⇒ 隐私必须赢 (安全 > 功能 > 性能);
//! 6. 候选全被排除 ⇒ 明确失败 (**不随便挑一个**);
//! 7. 缺价候选**不参与价格比较** (**绝不编价**);
//! 8. 无候选 ⇒ 错误形态可辨;
//! 9. 开关 fail-closed;
//! 10. 档位首选模型被隐私排除 ⇒ 如实说明。

use std::collections::BTreeMap;

use apeireth_core::cost_router::{
    comparison_price_micros, CostRouter, CostRoutingConfig, CostTier, RoutingRequest,
};
use apeireth_core::model_profile::{ModelProfile, ModelProfileTable};
use apeireth_core::routing_policy::{
    CandidateOutcome, PolicyCandidate, PrivacyLevel, RoutingPolicy, RoutingPolicyError,
    TaskProfile, PRIVACY_FLAG_CONFIDENTIAL, PRIVACY_FLAG_RESTRICTED,
};

/// 造一行画像 (缺省即未提供, 绝不编价)。
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

/// 画像行 + 隐私事实标记 (标记即隐私上限的事实来源)。
fn profile_with_privacy(
    model_id: &str,
    supports_tools: Option<bool>,
    context_window: Option<u64>,
    input_price_micros: Option<u64>,
    output_price_micros: Option<u64>,
    flags: &[&str],
) -> ModelProfile {
    let mut row = profile(
        model_id,
        supports_tools,
        context_window,
        input_price_micros,
        output_price_micros,
    );
    row.capabilities_flags = flags.iter().map(|flag| (*flag).to_string()).collect();
    row
}

/// 测试用画像表: 一个杂活行 (不支持工具) + 三个有价行 + 一个无价行。
fn table() -> ModelProfileTable {
    ModelProfileTable::new(vec![
        profile(
            "chore-model",
            Some(false),
            Some(32_768),
            Some(1_000),
            Some(1_000),
        ),
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

/// 一台开启的策略层 (候选表 + 画像表 + 默认模型 + 配置面)。
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

/// 候选表: 每个模型都只承接普通档 (隐私闸全开时的基线)。
fn open_candidate_table() -> Vec<PolicyCandidate> {
    vec![
        PolicyCandidate::new("chore-model", PrivacyLevel::Ordinary),
        PolicyCandidate::new("cheap-model", PrivacyLevel::Ordinary),
        PolicyCandidate::new("work-model", PrivacyLevel::Ordinary),
        PolicyCandidate::new("deep-model", PrivacyLevel::Ordinary),
        PolicyCandidate::new("unpriced-model", PrivacyLevel::Ordinary),
    ]
}

/// 标准档 + 需工具的任务画像。
fn standard_task(privacy: PrivacyLevel) -> TaskProfile {
    TaskProfile::new(
        apeireth_core::routing_policy::CapabilityRequirement::new().with_requires_tools(true),
        Some(CostTier::Standard),
        privacy,
    )
}

#[test]
fn the_privacy_vocabulary_is_a_closed_three_step_ladder() {
    let spellings: Vec<&str> = PrivacyLevel::ALL
        .iter()
        .map(|level| level.as_str())
        .collect();
    assert_eq!(
        spellings,
        vec!["ordinary", "restricted", "confidential"],
        "隐私词表: 三档闭合, 拼写冻结"
    );
    for level in PrivacyLevel::ALL {
        assert_eq!(PrivacyLevel::parse(level.as_str()), Some(level), "{level}");
        assert_eq!(
            PrivacyLevel::parse(&level.as_str().to_ascii_uppercase()),
            Some(level),
            "词表解析大小写不敏感"
        );
    }
    for unknown in ["secret", "", "ordinary-restricted", "none"] {
        assert_eq!(PrivacyLevel::parse(unknown), None, "词表闭合, 未知词不猜档");
    }
    // 阶梯单调: 普通 < 受限 < 机密 (数值序即敏感度序)。
    assert!(PrivacyLevel::Ordinary < PrivacyLevel::Restricted);
    assert!(PrivacyLevel::Restricted < PrivacyLevel::Confidential);
}

#[test]
fn a_normal_task_selects_the_cheapest_qualified_candidate_and_lists_a_fallback_chain() {
    let policy = policy(open_candidate_table(), table(), Some("work-model"));
    let decision = policy
        .decide(&standard_task(PrivacyLevel::Ordinary))
        .expect("选路成立");

    assert_eq!(decision.tier, CostTier::Standard);
    assert_eq!(decision.privacy, PrivacyLevel::Ordinary);
    assert_eq!(
        decision.selected_model, "cheap-model",
        "标准档: 有价候选里单价最低者胜出"
    );
    assert!(
        decision
            .selection_reason
            .contains("cheapest priced candidate"),
        "选谁要说清命中哪条规则: {}",
        decision.selection_reason
    );
    assert!(
        decision.selection_reason.contains("成本档 standard"),
        "理由要带上画像上下文: {}",
        decision.selection_reason
    );

    // 退路链: 有序、不含选中者、缺价候选排最后 (缺价不参与价格比较)。
    assert_eq!(
        decision.fallback_chain,
        vec!["work-model", "deep-model", "unpriced-model"],
        "退路 = 单价升序, 缺价候选排其后"
    );
    let fallback_reason = decision.fallback_reason.as_deref().expect("退路说明真值");
    assert!(fallback_reason.contains("退路 3 个"), "{fallback_reason}");

    // 逐行比较: 被排除的行也在场, 且带理由 (不静默过滤)。
    let chore = decision
        .comparisons
        .iter()
        .find(|row| row.model_id == "chore-model")
        .expect("逐行比较覆盖候选表全部行");
    assert_eq!(chore.outcome, CandidateOutcome::Excluded);
    let reason = chore.exclusion_reason.as_deref().expect("排除理由可见");
    assert!(reason.contains("未通过 standard 档过滤"), "{reason}");
    assert!(!chore.price_compared, "被排除的行不参与价格比较");

    let selected = decision
        .comparisons
        .iter()
        .find(|row| row.model_id == "cheap-model")
        .expect("选中者在逐行比较里");
    assert_eq!(selected.outcome, CandidateOutcome::Selected);
    assert_eq!(selected.comparison_price_micros, Some(2_000));
    assert!(selected.price_compared);

    // 最近决策真值 (自述面来源); 决策成功才记。
    assert_eq!(
        policy.last_decision().map(|d| d.selected_model),
        Some("cheap-model".to_string())
    );
}

#[test]
fn the_policy_layer_agrees_with_the_cost_router_on_the_same_tier() {
    // 同一档位、同一候选集 ⇒ 策略层与成本档层选出**同一个模型** (口径同源)。
    let profiles = table();
    let cost_router = CostRouter::new(
        profiles.clone(),
        Some("work-model".to_string()),
        CostRoutingConfig {
            enabled: true,
            preferred_models: BTreeMap::new(),
        },
    );
    let request = RoutingRequest::new()
        .with_explicit_tier(CostTier::Standard)
        .with_requires_tools(true);
    let cost_decision = cost_router.route(&request).expect("成本档层可路由");

    let policy = policy(open_candidate_table(), profiles, Some("work-model"));
    let policy_decision = policy
        .decide(&standard_task(PrivacyLevel::Ordinary))
        .expect("选路成立");

    assert_eq!(
        policy_decision.selected_model,
        cost_decision.selected_model.expect("成本档层选出模型"),
        "同一档位下选出的模型族一致 (不造第三套路由语义)"
    );
    assert_eq!(policy_decision.tier, cost_decision.tier);
}

#[test]
fn the_privacy_gate_excludes_candidates_and_the_reason_is_visible() {
    let candidates = vec![
        PolicyCandidate::new("cheap-model", PrivacyLevel::Ordinary),
        PolicyCandidate::new("work-model", PrivacyLevel::Restricted),
        PolicyCandidate::new("deep-model", PrivacyLevel::Restricted),
    ];
    let policy = policy(candidates, table(), Some("work-model"));
    let decision = policy
        .decide(&standard_task(PrivacyLevel::Restricted))
        .expect("选路成立");

    assert_eq!(
        decision.selected_model, "work-model",
        "普通档候选被隐私闸排除后, 受限档候选接管"
    );
    let cheap = decision
        .comparisons
        .iter()
        .find(|row| row.model_id == "cheap-model")
        .expect("逐行比较里有被排除的行");
    assert_eq!(cheap.outcome, CandidateOutcome::Excluded);
    let reason = cheap.exclusion_reason.as_deref().expect("排除理由可见");
    assert!(reason.contains("隐私等级 restricted"), "{reason}");
    assert!(reason.contains("上限 ordinary"), "{reason}");
    assert!(
        !decision.fallback_chain.contains(&"cheap-model".to_string()),
        "被隐私闸排除的候选绝不进退路链"
    );
}

#[test]
fn privacy_beats_price_in_an_adversarial_conflict() {
    // 对抗用例: 伪造「更便宜但隐私不合规」与「更贵但隐私合规」两个候选 ——
    // 安全 > 功能 > 性能: 隐私必须赢, 便宜的那个永远不能被选中。
    let candidates = vec![
        PolicyCandidate::new("cheap-model", PrivacyLevel::Ordinary),
        PolicyCandidate::new("deep-model", PrivacyLevel::Confidential),
    ];
    let policy = policy(candidates, table(), Some("work-model"));
    let decision = policy
        .decide(&standard_task(PrivacyLevel::Confidential))
        .expect("选路成立");

    assert_eq!(
        decision.selected_model, "deep-model",
        "隐私赢: 更便宜的普通档候选不得被机密档任务选中"
    );
    let cheap = decision
        .comparisons
        .iter()
        .find(|row| row.model_id == "cheap-model")
        .expect("逐行比较里有被排除的行");
    assert_eq!(cheap.outcome, CandidateOutcome::Excluded);
    assert!(
        cheap
            .exclusion_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("隐私等级 confidential")),
        "排除理由必须点名隐私闸: {:?}",
        cheap.exclusion_reason
    );
    assert!(
        !decision.fallback_chain.contains(&"cheap-model".to_string()),
        "越闸候选连退路都不许留"
    );
}

#[test]
fn when_every_candidate_is_excluded_the_policy_fails_loudly() {
    let policy = policy(open_candidate_table(), table(), Some("work-model"));
    let error = policy
        .decide(&standard_task(PrivacyLevel::Confidential))
        .expect_err("候选全被排除 ⇒ 明确失败, 不随便挑一个");

    match &error {
        RoutingPolicyError::NoEligibleCandidate {
            tier,
            privacy,
            comparisons,
            reason,
            ..
        } => {
            assert_eq!(*tier, CostTier::Standard);
            assert_eq!(*privacy, PrivacyLevel::Confidential);
            assert!(!comparisons.is_empty(), "失败帧要带逐行比较 (排除可见)");
            assert!(
                comparisons
                    .iter()
                    .all(|row| row.outcome == CandidateOutcome::Excluded),
                "全部行都是排除态"
            );
            assert!(
                comparisons.iter().all(|row| row.exclusion_reason.is_some()),
                "每一行的排除理由都在场"
            );
            assert!(reason.contains("全部被策略层排除"), "{reason}");
        }
        other => panic!("预期 NoEligibleCandidate, 实得 {other:?}"),
    }
    assert!(policy.last_decision().is_none(), "失败不记决策 (照实)");
}

#[test]
fn unpriced_candidates_never_enter_the_price_comparison() {
    let candidates = vec![
        PolicyCandidate::new("unpriced-model", PrivacyLevel::Ordinary),
        PolicyCandidate::new("work-model", PrivacyLevel::Ordinary),
    ];
    let policy = policy(candidates, table(), Some("work-model"));
    let decision = policy
        .decide(&standard_task(PrivacyLevel::Ordinary))
        .expect("选路成立");

    assert_eq!(
        decision.selected_model, "work-model",
        "有价证据在 ⇒ 按省钱选"
    );
    let unpriced = decision
        .comparisons
        .iter()
        .find(|row| row.model_id == "unpriced-model")
        .expect("逐行比较里有无价行");
    assert_eq!(
        unpriced.comparison_price_micros, None,
        "绝不编价: 无价行价格字段缺省"
    );
    assert!(!unpriced.price_compared, "无价候选不参与价格比较");
    assert!(
        decision.selection_reason.contains("priced candidates only"),
        "理由如实说明价格比较只覆盖有价候选: {}",
        decision.selection_reason
    );
    // 无价候选仍在候选/退路里 (按能力在场, 价格照实未提供)。
    assert!(
        decision
            .fallback_chain
            .contains(&"unpriced-model".to_string()),
        "无价候选按能力留作退路"
    );
    // 口径同源: 比较单价两列齐才有值。
    let table = table();
    let row = table.resolve("unpriced-model").into_owned();
    assert_eq!(comparison_price_micros(&row), None, "缺价 = 算不出");
}

#[test]
fn an_empty_cost_candidate_set_reports_its_own_error_shape() {
    // 只有一行不支持工具的模型 ⇒ 标准档 (需工具) 零候选。
    let profiles = ModelProfileTable::new(vec![profile(
        "chore-model",
        Some(false),
        Some(32_768),
        Some(1_000),
        Some(1_000),
    )]);
    let policy = policy(
        vec![PolicyCandidate::new("chore-model", PrivacyLevel::Ordinary)],
        profiles,
        Some("chore-model"),
    );
    let error = policy
        .decide(&standard_task(PrivacyLevel::Ordinary))
        .expect_err("零候选 ⇒ 明确失败");

    match &error {
        RoutingPolicyError::NoCostCandidate { tier, reason, .. } => {
            assert_eq!(*tier, CostTier::Standard);
            assert!(reason.contains("standard"), "{reason}");
            assert!(reason.contains("32768"), "如实报窗口下限: {reason}");
            assert!(reason.contains("20000"), "如实报单价上限: {reason}");
        }
        other => panic!("预期 NoCostCandidate, 实得 {other:?}"),
    }
    assert!(
        !matches!(error, RoutingPolicyError::NoEligibleCandidate { .. }),
        "零候选与全被排除是**两个**错误形态, 不混用"
    );
}

#[test]
fn the_policy_switch_fails_closed() {
    let policy = RoutingPolicy::new(
        open_candidate_table(),
        table(),
        Some("work-model".to_string()),
        CostRoutingConfig::disabled(),
    );
    assert!(!policy.is_enabled());
    let error = policy
        .decide(&standard_task(PrivacyLevel::Ordinary))
        .expect_err("关闸 = 不路由");
    assert!(
        matches!(error, RoutingPolicyError::Disabled),
        "关闸 fail-closed: 实得 {error:?}"
    );
    assert!(
        error.to_string().contains("APEIRETH_COST_ROUTING"),
        "错误帧要指出开关口径 (0 新 env 名): {error}"
    );
    assert!(policy.last_decision().is_none(), "关闸不记决策");
}

#[test]
fn the_tier_preferred_model_is_skipped_and_annotated_when_privacy_excludes_it() {
    // 档位首选模型 (配置覆盖) 被隐私闸排除 ⇒ 不得被选中, 且要如实说明为什么没选它。
    let mut preferred = BTreeMap::new();
    preferred.insert(CostTier::Standard, "cheap-model".to_string());
    let policy = RoutingPolicy::new(
        vec![
            PolicyCandidate::new("cheap-model", PrivacyLevel::Ordinary),
            PolicyCandidate::new("work-model", PrivacyLevel::Restricted),
        ],
        table(),
        Some("work-model".to_string()),
        CostRoutingConfig {
            enabled: true,
            preferred_models: preferred,
        },
    );
    let decision = policy
        .decide(&standard_task(PrivacyLevel::Restricted))
        .expect("选路成立");

    assert_eq!(
        decision.selected_model, "work-model",
        "隐私闸优先于档位首选覆盖"
    );
    assert!(
        decision
            .selection_reason
            .contains("档位首选模型 cheap-model 未进入候选"),
        "首选落选要如实标注: {}",
        decision.selection_reason
    );
}

#[test]
fn the_cost_tier_decides_which_candidates_even_enter_the_comparison() {
    // 成本档优先: 深思档 (窗口下限 131_072) 把便宜但窗口小的模型挡在比较之外。
    let candidates = vec![
        PolicyCandidate::new("cheap-model", PrivacyLevel::Ordinary),
        PolicyCandidate::new("deep-model", PrivacyLevel::Ordinary),
        PolicyCandidate::new("unpriced-model", PrivacyLevel::Ordinary),
    ];
    let policy = policy(candidates, table(), Some("work-model"));

    let standard = policy
        .decide(&standard_task(PrivacyLevel::Ordinary))
        .expect("标准档选路成立");
    assert_eq!(
        standard.selected_model, "cheap-model",
        "标准档: 更便宜的模型够格"
    );

    let premium = policy
        .decide(&TaskProfile::new(
            apeireth_core::routing_policy::CapabilityRequirement::new().with_requires_tools(true),
            Some(CostTier::Premium),
            PrivacyLevel::Ordinary,
        ))
        .expect("深思档选路成立");
    assert_eq!(
        premium.selected_model, "deep-model",
        "深思档: 窗口不够的便宜模型出局, 档位决定候选集"
    );
    let cheap = premium
        .comparisons
        .iter()
        .find(|row| row.model_id == "cheap-model")
        .expect("逐行比较里有被档位挡下的行");
    assert_eq!(cheap.outcome, CandidateOutcome::Excluded);
    let reason = cheap.exclusion_reason.as_deref().expect("排除理由可见");
    assert!(reason.contains("未通过 premium 档过滤"), "{reason}");
}

#[test]
fn the_candidate_table_can_be_derived_from_the_profile_privacy_flags() {
    // 隐私事实落点: 画像行的能力标记 (缺省 = 未提供 ⇒ 只承接普通档, 取更严)。
    let profiles = ModelProfileTable::new(vec![
        profile_with_privacy(
            "plain-model",
            Some(true),
            Some(32_768),
            Some(1_000),
            Some(1_000),
            &[],
        ),
        profile_with_privacy(
            "controlled-model",
            Some(true),
            Some(32_768),
            Some(2_000),
            Some(2_000),
            &[PRIVACY_FLAG_RESTRICTED],
        ),
        profile_with_privacy(
            "private-model",
            Some(true),
            Some(32_768),
            Some(3_000),
            Some(3_000),
            &[PRIVACY_FLAG_CONFIDENTIAL],
        ),
        profile_with_privacy(
            "contradicting-model",
            Some(true),
            Some(32_768),
            Some(4_000),
            Some(4_000),
            &[PRIVACY_FLAG_RESTRICTED, PRIVACY_FLAG_CONFIDENTIAL],
        ),
    ]);
    let table = PolicyCandidate::table_from_profiles(&profiles);
    let ceiling = |model_id: &str| {
        table
            .iter()
            .find(|candidate| candidate.model_id == model_id)
            .map(|candidate| candidate.max_privacy)
            .expect("派生表含该行")
    };
    assert_eq!(
        ceiling("plain-model"),
        PrivacyLevel::Ordinary,
        "缺省 = 只承接普通档"
    );
    assert_eq!(ceiling("controlled-model"), PrivacyLevel::Restricted);
    assert_eq!(ceiling("private-model"), PrivacyLevel::Confidential);
    assert_eq!(
        ceiling("contradicting-model"),
        PrivacyLevel::Restricted,
        "双标冲突取更严的一档 (不靠标记放宽)"
    );

    // 派生表可直接喂给策略层: 机密档任务只落到标记齐全的模型上。
    let policy = policy(table, profiles, Some("plain-model"));
    let decision = policy
        .decide(&standard_task(PrivacyLevel::Confidential))
        .expect("选路成立");
    assert_eq!(
        decision.selected_model, "private-model",
        "机密档任务只可落到标记可承接机密的模型"
    );
}
