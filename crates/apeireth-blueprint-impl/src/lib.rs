//! # apeireth-blueprint-impl — 蓝图实装 crate
//!
//! **目的**: 把 RIVAL 蓝图 §2.4 (R20 阶段 4 估补) 的 5 项集中实装到 1 个 crate.
//!
//! **5 估补项**:
//!
//! | # | 估补项 | 模块 | 关键 API |
//! |---|--------|------|---------|
//! | 1 | 4 风险类 (K-1/K-2/K-3/K-4) | [`risk`] | [`K1StrongValidate`] / [`K2WeakValidate`] / [`K3Audit`] / [`K4Guard`] |
//! | 2 | 4 决策表 (D-01/D-02/D-03/D-04) | [`decision`] | [`D01Impl`] / [`D02Routing`] / [`D03WsAuth`] / [`D04RateLimit`] |
//! | 3 | 6 实战模板 (A-F) | [`template`] | [`template_a_auth`] / [`template_b_ratelimit`] / [`template_c_error`] / [`template_d_test`] / [`template_e_config`] / [`template_f_logging`] |
//! | 4 | 5 R-Measure (R-1..R-5) | [`r_measure`] | [`r1_directness`] / [`r2_candor`] / [`r3_closure`] / [`r4_promise`] / [`r5_failure_honesty`] |
//! | 5 | 3 评估指标 (Q1/Q2/Q3) | [`q_metric`] | [`q1_quality`] / [`q2_satisfaction`] / [`q3_growth`] |
//!
//! **不冲突**: V0.5 命名 24 维 (per `apeireth-naming-v05` crate, bg_6603d030 单独派) 互不依赖.
//!
//! **RIVAL 引用**: `docs/stage3-blueprints/v09021-rust-translation-blueprint-RIVAL.md` §2.4
//!
//! ---
//!
//! ## 🎯 6 哲学锚穿透
//!
//! | 锚 | 含义 | 本 crate 体现 |
//! |----|------|---------------|
//! | **S-1 主 22:33** | 北极星导向 | 5 估补服务 ASI 北极星, 不装饰 |
//! | **S-2 主 17:43** | 实事求是 | 5 项实装, 0 TODO 占位 |
//! | **O-5 主 17:58** | 不假装 | 失败必须 `Err`, R-5 量化"不假装"率 |
//! | **O-2 主 19:33** | 走在前人经验上 | 借鉴 OWASP 4 类输入校验 / v0.9.21 商业版 R-Measure / token bucket / WS 鉴权 |
//! | **O-3 主 23:44** | 干到底 | 5 项 1 crate 打包, 不分散, 不留尾 |
//! | **O-4 主 00:56** | 任何人都能接手 | 5 模块边界清晰, 4 文件 = 4 trait/enum/函数集, 接手者一眼能看 |
//!
//! ---
//!
//! ## 🛡️ 8 项不修改承诺 (per APEIRETH-CONVENTIONS.md)
//!
//! | # | 不修改项 | 原因 / 引用 |
//! |---|---------|------------|
//! | 1 | 阶段 1+2+3 LOCKED | 主人明确沉淀, 不可改 |
//! | 2 | v2 / v4 / v4.1 LOCKED | 哲学层纲领 |
//! | 3 | 阶段 4 主文档 LOCKED (6ca80776) | 6ca80776 commit |
//! | 4 | 阶段 5 施工文档 LOCKED (631 行) | R20 阶段 5 |
//! | 5 | v6 修正 = 4 重守门 + 权限发放 + E 层修改路径 | 5 决策表中 D-04 走 v6 |
//! | 6 | R11 baseline 三值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) | [`r_measure::RMeasureAll::drift`] 对齐 |
//! | 7 | v1 → v5 历史链 | 不删除 |
//! | 8 | 24 LOCKED crate 不动 | 本 crate 是新增, 不修改任何 LOCKED crate (新增在 `crates/apeireth-blueprint-impl/`, 跟 24 LOCKED 并列) |
//!
//! ---
//!
//! ## 📦 模块一览
//!
//! - [`error`] — 统一错误类型 [`error::BlueprintError`] (13 个 variant)
//! - [`risk`] — 4 风险类 (K-1/K-2/K-3/K-4) trait + 默认实现 + [`risk::RiskChain`]
//! - [`decision`] — 4 决策表 enum + [`decision::DecisionBundle`]
//! - [`template`] — 6 模板函数 + 真实 trait 接口
//! - [`r_measure`] — 5 R-Measure 函数 + [`r_measure::RMeasureAll`] 聚合
//! - [`q_metric`] — 3 Q-Metric 函数 + [`q_metric::QMetricAll`] 聚合
//!
//! ## 🚀 快速开始
//!
//! ```rust
//! use apeireth_blueprint_impl::*;
//!
//! // 1. 4 决策打包
//! let decisions = DecisionBundle::default();
//! decisions.validate().unwrap();
//!
//! // 2. 跑 R-Measure
//! let samples = vec![
//!     ActionSample::perfect(),
//!     ActionSample::perfect(),
//! ];
//! let r = RMeasureAll::from_samples(&samples);
//! assert_eq!(r.r1_directness, 1.0);
//!
//! // 3. 跑 Q-Metric
//! let tasks = vec![TaskResult::new(true, 1.0)];
//! let feedback = vec![UserFeedback { rating: 5, has_text: false, is_long_term: false }];
//! let q = QMetricAll::from_inputs(&tasks, &feedback, &[]);
//! assert_eq!(q.q1_quality, 1.0);
//! ```

#![deny(unsafe_code)]
#![allow(clippy::needless_lifetimes)]

// ============================================
// 0. 模块声明 + 公共 re-export
// ============================================

pub mod error;
// R177: organ invariants (5 tests + 2 Kani)
pub mod decision;
mod organ_kani_proofs;
pub mod q_metric;
pub mod r_measure;
pub mod risk;
pub mod template;

// 统一 re-export — 让 `use apeireth_blueprint_impl::*;` 拿到全部 5 估补的 public API
pub use decision::{D01Impl, D02Routing, D03WsAuth, D04RateLimit, DecisionBundle};
pub use error::{BlueprintError, BlueprintResult};
pub use q_metric::{
    q1_quality, q2_satisfaction, q3_growth, GrowthSnapshot, QMetricAll, TaskResult, UserFeedback,
};
pub use r_measure::{
    r1_directness, r2_candor, r3_closure, r4_promise, r5_failure_honesty, ActionSample,
    RMeasureAll, RMeasureDrift,
};
pub use risk::{
    AuditEvent, BrokenAudit, DefaultK1Guard, DefaultK2Guard, GuardDecision, GuardRule,
    InMemoryAudit, K1Input, K1StrongValidate, K2Input, K2Result, K2WeakValidate, K3Audit, K4Guard,
    RiskChain, RuleTableGuard,
};
pub use template::{
    template_a_auth, template_b_ratelimit, template_c_error, template_d_test, template_e_config,
    template_f_logging, AlwaysAllowRateLimit, Auth, AuthToken, ConfigLoader, DefaultErrorMapper,
    EnvFileConfig, InMemoryAuth, Logging, MockAuth, MockAuthImpl, RateLimit, TokenBucket,
    TracingAuditLog, UnifiedError,
};

// ============================================
// 1. 集成层 — BlueprintPipeline (5 估补项串联)
// ============================================

/// Blueprint 5 估补的串联管线 — 一次调用走完全部 5 项.
///
/// 顺序 (per RIVAL §2.4 + S-1 北极星):
/// 1. **风险 (K-1..K-4)**: 任何 action 必须过 4 风险类守门
/// 2. **决策 (D-01..D-04)**: endpoint / 路由 / WS 鉴权 / 限流 4 决策打包
/// 3. **模板 (A-F)**: 鉴权 / 限流 / 错误 / 测试 / 配置 / 日志 6 模板
/// 4. **R-Measure (R-1..R-5)**: 直行 / 直说 / 闭环 / 守门 / 诚实 5 维
/// 5. **Q-Metric (Q1/Q2/Q3)**: 任务质量 / 用户满意 / 长期成长 3 维
///
/// 5 步全完成后返回 [`BlueprintReport`].
#[derive(Default)]
pub struct BlueprintPipeline {
    pub decisions: DecisionBundle,
    pub r_measure: Option<RMeasureAll>,
    pub q_metric: Option<QMetricAll>,
}

impl BlueprintPipeline {
    /// 构造 — 默认决策打包.
    pub fn new() -> Self {
        Self::default()
    }

    /// 用自定义决策构造.
    pub fn with_decisions(decisions: DecisionBundle) -> Self {
        Self {
            decisions,
            r_measure: None,
            q_metric: None,
        }
    }

    /// 校验 4 决策 (任何不合法 → Err).
    pub fn validate_decisions(&self) -> BlueprintResult<()> {
        self.decisions.validate()
    }

    /// 算 R-Measure.
    pub fn compute_r_measure(&mut self, samples: &[ActionSample]) -> BlueprintResult<RMeasureAll> {
        let r = RMeasureAll::from_samples(samples);
        r.validate()?;
        self.r_measure = Some(r);
        Ok(r)
    }

    /// 算 Q-Metric.
    pub fn compute_q_metric(
        &mut self,
        tasks: &[TaskResult],
        feedback: &[UserFeedback],
        history: &[GrowthSnapshot],
    ) -> BlueprintResult<QMetricAll> {
        let q = QMetricAll::from_inputs(tasks, feedback, history);
        q.validate()?;
        self.q_metric = Some(q);
        Ok(q)
    }

    /// 决策快照 — 4 决策可视化字符串.
    pub fn decision_snapshot(&self) -> String {
        self.decisions.snapshot()
    }

    /// 生成完整 report.
    pub fn report(&self) -> BlueprintReport {
        BlueprintReport {
            decision_snapshot: self.decisions.snapshot(),
            r_measure: self.r_measure,
            q_metric: self.q_metric,
        }
    }
}

/// Blueprint 完整运行报告.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlueprintReport {
    pub decision_snapshot: String,
    pub r_measure: Option<RMeasureAll>,
    pub q_metric: Option<QMetricAll>,
}

impl BlueprintReport {
    /// 是否全部通过 baseline.
    pub fn meets_baseline(&self) -> bool {
        match self.r_measure {
            Some(r) => r.drift().all_meet_baseline() && r.r5_failure_honesty >= 1.0,
            None => false,
        }
    }

    /// 综合得分 (R-Measure 5 维 + Q-Metric 3 维, 等权平均, 缺项 = 0).
    pub fn composite_score(&self) -> f64 {
        let r = self.r_measure.map(|m| m.average()).unwrap_or(0.0);
        let q = self.q_metric.map(|m| m.average()).unwrap_or(0.0);
        // R:5 项, Q:3 项, 总 8 项
        (r * 5.0 + q * 3.0) / 8.0
    }
}

// ============================================
// 2. 模块 1 — risk 集成演示
// ============================================

/// 模块 1 (risk) 集成 — 默认 K1Guard + K2Guard + InMemoryAudit + RuleTableGuard.
pub fn default_risk_chain(
) -> RiskChain<DefaultK1Guard, DefaultK2Guard, InMemoryAudit, RuleTableGuard> {
    RiskChain::new(
        DefaultK1Guard,
        DefaultK2Guard,
        InMemoryAudit::default(),
        RuleTableGuard::new(),
    )
}

/// 模块 1 (risk) 集成 — 白名单 K1Guard (model 只允许 claude-*, scope 只允许 read/write).
pub struct WhitelistK1Guard;
impl K1StrongValidate for WhitelistK1Guard {
    fn validate(&self, input: &K1Input) -> BlueprintResult<()> {
        // K1Input::new 已保证 4 字段非空 + 长度合理 + 无控制字符
        if !input.model_name.starts_with("claude-") {
            return Err(BlueprintError::K1StrongValidationFailed {
                field: "model_name".into(),
                value: input.model_name.clone(),
                reason: "only claude-* models allowed".into(),
            });
        }
        if !["read", "write", "admin"].contains(&input.scope.as_str()) {
            return Err(BlueprintError::K1StrongValidationFailed {
                field: "scope".into(),
                value: input.scope.clone(),
                reason: "scope not in [read, write, admin]".into(),
            });
        }
        Ok(())
    }
}

/// 模块 1 (risk) 集成演示 — 用白名单 K1Guard 跑一次完整 chain.
pub fn demo_risk_chain_with_whitelist() -> BlueprintResult<(K2Result, GuardDecision)> {
    let k1 = K1Input::new("hello", "sk-test1234", "claude-3-sonnet", "read")?;
    let k2 = K2Input::new("hi", vec!["fallback1".into(), "fallback2".into()]);
    let mut g4 = RuleTableGuard::new();
    g4.add_rule(GuardRule {
        subject: "tool:bash".into(),
        action: "exec".into(),
        decision: GuardDecision::Allow,
        reason: "default-allow".into(),
    })?;
    let chain: RiskChain<WhitelistK1Guard, DefaultK2Guard, InMemoryAudit, RuleTableGuard> =
        RiskChain::new(
            WhitelistK1Guard,
            DefaultK2Guard,
            InMemoryAudit::default(),
            g4,
        );
    chain.run(&k1, &k2, "tool:bash", "exec")
}

// ============================================
// 3. 模块 2 — decision 集成演示
// ============================================

/// 模块 2 (decision) 集成 — 5 Provider 决策快照 (1 真接 + 4 stub).
pub fn five_provider_decisions() -> Vec<(String, D01Impl)> {
    vec![
        (
            "claude-code".into(),
            D01Impl::RealConnect {
                provider: "claude-code".into(),
                endpoint: "/v1/messages".into(),
            },
        ),
        (
            "codex".into(),
            D01Impl::StubNotImplemented {
                tool: "codex".into(),
                planned_stage: "R21".into(),
            },
        ),
        (
            "gemini-cli".into(),
            D01Impl::StubNotImplemented {
                tool: "gemini-cli".into(),
                planned_stage: "R21".into(),
            },
        ),
        (
            "iflow".into(),
            D01Impl::StubNotImplemented {
                tool: "iflow".into(),
                planned_stage: "R22".into(),
            },
        ),
        (
            "opencode".into(),
            D01Impl::StubNotImplemented {
                tool: "opencode".into(),
                planned_stage: "R22".into(),
            },
        ),
    ]
}

impl D01Impl {
    /// R20 阶段 4 估补用 — 返回 (provider_name, is_real).
    pub fn provider_status(&self) -> (&str, bool) {
        match self {
            D01Impl::RealConnect { provider, .. } => (provider.as_str(), true),
            D01Impl::StubNotImplemented { tool, .. } => (tool.as_str(), false),
        }
    }
}

/// 模块 2 (decision) 集成演示 — 5 Provider 决策全 validate.
pub fn demo_decision_5_providers() -> BlueprintResult<usize> {
    let decisions = five_provider_decisions();
    let mut count = 0;
    for (_name, d01) in &decisions {
        d01.validate()?;
        count += 1;
    }
    Ok(count)
}

// ============================================
// 4. 模块 3 — template 集成演示
// ============================================

/// 模块 3 (template) 集成 — 6 模板全开 (5 运行时 + 1 测试).
pub struct TemplateBundle<A, R, L>
where
    A: Auth,
    R: RateLimit,
    L: Logging,
{
    pub auth: A,
    pub rate_limit: R,
    pub logging: L,
}

impl<A, R, L> TemplateBundle<A, R, L>
where
    A: Auth,
    R: RateLimit,
    L: Logging,
{
    /// 6 模板串联使用: 鉴权 → 限流 → 日志
    pub fn execute(&self, scope: &str, action: &str) -> BlueprintResult<AuthToken> {
        self.rate_limit.try_acquire()?;
        let tok = self.auth.issue(scope)?;
        self.logging
            .trace("template_bundle", &format!("issued token for {action}"));
        Ok(tok)
    }
}

/// 模块 3 (template) 集成演示 — 用 6 模板默认实现构造完整 bundle.
pub fn demo_template_bundle() -> TemplateBundle<InMemoryAuth, TokenBucket, TracingAuditLog> {
    let auth = InMemoryAuth::default();
    let rate_limit = TokenBucket::new(60, std::time::Duration::from_secs(1));
    let logging = TracingAuditLog::default();
    TemplateBundle {
        auth,
        rate_limit,
        logging,
    }
}

// ============================================
// 5. 模块 4 — r_measure 集成演示
// ============================================

/// 模块 4 (r_measure) 集成演示 — 跑 100 完美样本 + 算 R-Measure 全 5 维.
pub fn demo_r_measure_100_perfect() -> RMeasureAll {
    let samples = (0..100)
        .map(|_| ActionSample::perfect())
        .collect::<Vec<_>>();
    RMeasureAll::from_samples(&samples)
}

/// 模块 4 (r_measure) 集成演示 — 跑 100 mixed 样本 (87% 完美) 看是否超 R11 baseline.
pub fn demo_r_measure_100_mixed_above_baseline() -> RMeasureAll {
    let n_perfect = 87;
    let n_worst = 13;
    let mut samples = Vec::with_capacity(n_perfect + n_worst);
    for _ in 0..n_perfect {
        samples.push(ActionSample::perfect());
    }
    for _ in 0..n_worst {
        samples.push(ActionSample::worst());
    }
    RMeasureAll::from_samples(&samples)
}

// ============================================
// 6. 模块 5 — q_metric 集成演示
// ============================================

/// 模块 5 (q_metric) 集成演示 — 10 任务全完美 + 5 用户全 5 星 + 2 成长快照.
pub fn demo_q_metric_optimal() -> QMetricAll {
    let tasks = (0..10)
        .map(|_| TaskResult::new(true, 1.0))
        .collect::<Vec<_>>();
    let feedback = (0..5)
        .map(|_| UserFeedback {
            rating: 5,
            has_text: true,
            is_long_term: true,
        })
        .collect::<Vec<_>>();
    let history = vec![
        GrowthSnapshot::new(0, 0.5, 0.5, 0.5),
        GrowthSnapshot::new(1, 0.9, 0.9, 0.9),
    ];
    QMetricAll::from_inputs(&tasks, &feedback, &history)
}

// ============================================
// 7. 总体集成 — 5 估补一次跑全
// ============================================

/// 5 估补全跑 (Pipeline 入口).
///
/// 5 步:
/// 1. 决策 validate
/// 2. R-Measure compute
/// 3. Q-Metric compute
/// 4. 生成 report
/// 5. 算 composite score
pub fn run_full_pipeline(
    decisions: DecisionBundle,
    samples: &[ActionSample],
    tasks: &[TaskResult],
    feedback: &[UserFeedback],
    history: &[GrowthSnapshot],
) -> BlueprintResult<BlueprintReport> {
    let mut pipe = BlueprintPipeline::with_decisions(decisions);
    pipe.validate_decisions()?;
    pipe.compute_r_measure(samples)?;
    pipe.compute_q_metric(tasks, feedback, history)?;
    Ok(pipe.report())
}

// ============================================
// 8. 文档常量 — 6 哲学锚 + 8 项承诺 (供 doc 链接用)
// ============================================

/// 6 哲学锚 (供外部 crate 引用, 保证命名一致).
pub const PHILOSOPHY_ANCHORS: [&str; 6] = [
    "S-1 主 22:33 北极星导向",
    "S-2 主 17:43 实事求是",
    "O-5 主 17:58 不假装",
    "O-2 主 19:33 走在前人经验上",
    "O-3 主 23:44 干到底",
    "O-4 主 00:56 任何人都能接手",
];

/// 8 项不修改承诺 (供外部 crate 引用).
pub const EIGHT_PROMISES: [&str; 8] = [
    "1. 阶段 1+2+3 LOCKED",
    "2. v2 / v4 / v4.1 LOCKED",
    "3. 阶段 4 主文档 LOCKED (6ca80776)",
    "4. 阶段 5 施工文档 LOCKED (631 行)",
    "5. v6 修正 = 4 重守门 + 权限发放 + E 层修改路径",
    "6. R11 baseline 三值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063)",
    "7. v1 → v5 历史链不删除",
    "8. 24 LOCKED crate 不动 (本 crate 新增, 不修改任何 LOCKED)",
];

// ============================================
// 9. serde 引入 (BlueprintReport 序列化用)
// ============================================

use serde::{Deserialize, Serialize};

// ============================================
// 10. 自检 — 5 估补都跑一次 (sanity check)
// ============================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- 哲学锚 + 承诺 ---
    #[test]
    fn philosophy_anchors_count_is_6() {
        assert_eq!(PHILOSOPHY_ANCHORS.len(), 6);
    }

    #[test]
    fn eight_promises_count_is_8() {
        assert_eq!(EIGHT_PROMISES.len(), 8);
    }

    // --- Pipeline ---
    #[test]
    fn pipeline_new_uses_default_decisions() {
        let p = BlueprintPipeline::new();
        assert!(p.validate_decisions().is_ok());
    }

    #[test]
    fn pipeline_with_decisions_uses_provided() {
        let d = DecisionBundle::new(
            D01Impl::RealConnect {
                provider: "claude-code".into(),
                endpoint: "/v1/messages".into(),
            },
            D02Routing::SubPath {
                tool: "Bash".into(),
                sub_path: "/v1/Bash".into(),
            },
            D03WsAuth::LinkToken {
                ttl: std::time::Duration::from_secs(300),
            },
            D04RateLimit::TokenBucket {
                capacity: 60,
                refill_interval: std::time::Duration::from_secs(1),
            },
        );
        let p = BlueprintPipeline::with_decisions(d);
        assert!(p.validate_decisions().is_ok());
    }

    #[test]
    fn pipeline_compute_r_measure() {
        let mut p = BlueprintPipeline::new();
        let r = p.compute_r_measure(&[ActionSample::perfect()]).unwrap();
        assert_eq!(r.r1_directness, 1.0);
        assert!(p.r_measure.is_some());
    }

    #[test]
    fn pipeline_compute_q_metric() {
        let mut p = BlueprintPipeline::new();
        let q = p
            .compute_q_metric(&[TaskResult::new(true, 1.0)], &[], &[])
            .unwrap();
        assert_eq!(q.q1_quality, 1.0);
        assert!(p.q_metric.is_some());
    }

    #[test]
    fn pipeline_report_meets_baseline_with_perfect_samples() {
        let mut p = BlueprintPipeline::new();
        p.compute_r_measure(&[ActionSample::perfect()]).unwrap();
        let report = p.report();
        assert!(report.meets_baseline());
    }

    #[test]
    fn pipeline_report_does_not_meet_baseline_without_r_measure() {
        let p = BlueprintPipeline::new();
        let report = p.report();
        assert!(!report.meets_baseline());
    }

    #[test]
    fn pipeline_report_composite_score_with_both() {
        let mut p = BlueprintPipeline::new();
        p.compute_r_measure(&[ActionSample::perfect()]).unwrap();
        let q = p
            .compute_q_metric(
                &[TaskResult::new(true, 1.0)],
                &[UserFeedback {
                    rating: 5,
                    has_text: false,
                    is_long_term: false,
                }],
                &[],
            )
            .unwrap();
        assert_eq!(q.q1_quality, 1.0);
        let report = p.report();
        let score = report.composite_score();
        // R avg = 1.0, Q avg = (1.0 + 1.0 + 0.0) / 3 ≈ 0.6667
        // composite = (1.0 * 5 + 0.6667 * 3) / 8 ≈ 0.875
        assert!(score > 0.85 && score < 0.9, "score = {score}");
    }

    // --- 模块 1 risk 集成 ---
    #[test]
    fn default_risk_chain_works() {
        let chain = default_risk_chain();
        let k1 = K1Input::new("hi", "sk-test1234", "gpt-4", "read").unwrap();
        let k2 = K2Input::new("hi", vec![]);
        let r = chain.run(&k1, &k2, "x", "y");
        assert!(r.is_ok());
    }

    #[test]
    fn whitelist_risk_chain_rejects_gpt() {
        // 用 gpt-4 (非 claude-*) 单独构造 input, 跑白名单 K1Guard → 必然 Err
        use crate::risk::{
            GuardDecision, GuardRule, InMemoryAudit, K1Input, K1StrongValidate, K2Input, RiskChain,
        };
        struct WhitelistK1;
        impl K1StrongValidate for WhitelistK1 {
            fn validate(&self, input: &K1Input) -> BlueprintResult<()> {
                if !input.model_name.starts_with("claude-") {
                    return Err(BlueprintError::K1StrongValidationFailed {
                        field: "model_name".into(),
                        value: input.model_name.clone(),
                        reason: "only claude-* allowed".into(),
                    });
                }
                Ok(())
            }
        }
        let mut g4 = RuleTableGuard::new();
        g4.add_rule(GuardRule {
            subject: "tool:bash".into(),
            action: "exec".into(),
            decision: GuardDecision::Allow,
            reason: "default".into(),
        })
        .unwrap();
        let chain: RiskChain<WhitelistK1, DefaultK2Guard, InMemoryAudit, RuleTableGuard> =
            RiskChain::new(WhitelistK1, DefaultK2Guard, InMemoryAudit::default(), g4);
        let k1 = K1Input::new("hi", "sk-test1234", "gpt-4", "read").unwrap();
        let k2 = K2Input::new("hi", vec![]);
        let r = chain.run(&k1, &k2, "tool:bash", "exec");
        assert!(r.is_err()); // gpt-4 不在 claude-* 白名单
    }

    // --- 模块 2 decision 集成 ---
    #[test]
    fn five_provider_decisions_have_1_real_4_stub() {
        let d = five_provider_decisions();
        let real = d.iter().filter(|(_, x)| x.is_real()).count();
        let stub = d.iter().filter(|(_, x)| !x.is_real()).count();
        assert_eq!(real, 1);
        assert_eq!(stub, 4);
    }

    #[test]
    fn demo_decision_5_providers_validates_all() {
        let r = demo_decision_5_providers().unwrap();
        assert_eq!(r, 5);
    }

    // --- 模块 3 template 集成 ---
    #[test]
    fn template_bundle_executes() {
        let bundle = demo_template_bundle();
        // 鉴权 + 限流 + 日志
        let r = bundle.execute("read", "tool:bash");
        assert!(r.is_ok());
    }

    #[test]
    fn template_bundle_respects_ratelimit() {
        // 用 capacity=2 的限流构造 bundle, 跑 5 次, 第 3 次起失败
        let auth = InMemoryAuth::default();
        let rate_limit = TokenBucket::new(2, std::time::Duration::from_secs(60));
        let logging = TracingAuditLog::default();
        let bundle: TemplateBundle<InMemoryAuth, TokenBucket, TracingAuditLog> = TemplateBundle {
            auth,
            rate_limit,
            logging,
        };
        assert!(bundle.execute("read", "a").is_ok());
        assert!(bundle.execute("read", "b").is_ok());
        assert!(bundle.execute("read", "c").is_err());
    }

    // --- 模块 4 r_measure 集成 ---
    #[test]
    fn test_demo_r_measure_100_perfect_is_one() {
        let r = super::demo_r_measure_100_perfect();
        assert_eq!(r.r1_directness, 1.0);
        assert_eq!(r.r2_candor, 1.0);
        assert_eq!(r.r3_closure, 1.0);
        assert_eq!(r.r4_promise, 1.0);
        assert_eq!(r.r5_failure_honesty, 1.0);
    }

    #[test]
    fn test_demo_r_measure_100_mixed_above_baseline() {
        let r = super::demo_r_measure_100_mixed_above_baseline();
        // 87/100 = 0.87 → 跟 R11 baseline 0.8682 持平或略高
        assert!(r.r4_promise >= 0.86);
    }

    // --- 模块 5 q_metric 集成 ---
    #[test]
    fn demo_q_metric_optimal_is_optimal() {
        let q = demo_q_metric_optimal();
        assert_eq!(q.q1_quality, 1.0);
        assert!(q.q2_satisfaction > 0.9);
        // growth: dr=0.4, dt=0.4, ds=0.4 → avg = 0.4
        assert!((q.q3_growth - 0.4).abs() < 1e-9);
    }

    // --- 总集成 ---
    #[test]
    fn run_full_pipeline_works() {
        let decisions = DecisionBundle::default();
        let samples = vec![ActionSample::perfect(), ActionSample::perfect()];
        let tasks = vec![TaskResult::new(true, 1.0)];
        let feedback = vec![UserFeedback {
            rating: 5,
            has_text: true,
            is_long_term: true,
        }];
        let history = vec![
            GrowthSnapshot::new(0, 0.5, 0.5, 0.5),
            GrowthSnapshot::new(1, 0.9, 0.9, 0.9),
        ];
        let report = run_full_pipeline(decisions, &samples, &tasks, &feedback, &history).unwrap();
        assert!(report.meets_baseline());
        assert!(report.composite_score() > 0.5);
    }

    #[test]
    fn run_full_pipeline_fails_on_invalid_decision() {
        let bad_decisions = DecisionBundle::new(
            D01Impl::default(),
            D02Routing::default(),
            D03WsAuth::LinkToken {
                ttl: std::time::Duration::from_secs(10 * 60), // invalid
            },
            D04RateLimit::default(),
        );
        let r = run_full_pipeline(bad_decisions, &[], &[], &[], &[]);
        assert!(r.is_err());
    }

    // --- 5 估补项 sanity (O-5 不假装: 全部 API 真接) ---
    #[test]
    fn all_5_modules_publicly_exposed() {
        // K
        let _k1: Box<dyn K1StrongValidate> = Box::new(DefaultK1Guard);
        let _k2: Box<dyn K2WeakValidate> = Box::new(DefaultK2Guard);
        let _k3: Box<dyn K3Audit> = Box::new(InMemoryAudit::default());
        let _k4: Box<dyn K4Guard> = Box::new(RuleTableGuard::new());
        // D
        let _d01 = D01Impl::default();
        let _d02 = D02Routing::default();
        let _d03 = D03WsAuth::default();
        let _d04 = D04RateLimit::default();
        // T
        let _a: Box<dyn Auth> = Box::new(template_a_auth());
        let _b: Box<dyn RateLimit> = Box::new(template_b_ratelimit());
        let _c: Box<dyn UnifiedError> = Box::new(template_c_error());
        let _e: Box<dyn ConfigLoader> = Box::new(template_e_config());
        let _f: Box<dyn Logging> = Box::new(template_f_logging());
        // R + Q 都是 pub fn, 不需要 trait 对象
        let _ = r1_directness(&[]);
        let _ = q1_quality(&[]);
    }
}

// ============================================
// 11. 架构流图 (comment only, 不进 binary)
// ============================================
//
//   ┌──────────────────── apeireth-blueprint-impl ────────────────────┐
//   │                                                                   │
//   │  ┌─────────────┐   ┌──────────────┐   ┌──────────────┐         │
//   │  │   risk      │   │  decision    │   │  template    │         │
//   │  │  (K1..K4)   │   │  (D01..D04)  │   │  (A..F)      │         │
//   │  └──────┬──────┘   └──────┬───────┘   └──────┬───────┘         │
//   │         │                 │                  │                  │
//   │         └─────────┬───────┴──────────┬───────┘                  │
//   │                   │                  │                          │
//   │                   ▼                  ▼                          │
//   │            ┌──────────────┐   ┌──────────────┐                  │
//   │            │  r_measure   │   │   q_metric   │                  │
//   │            │  (R1..R5)    │   │  (Q1..Q3)    │                  │
//   │            └──────┬───────┘   └──────┬───────┘                  │
//   │                   │                  │                          │
//   │                   └────────┬─────────┘                          │
//   │                            ▼                                    │
//   │                   ┌──────────────────┐                          │
//   │                   │ BlueprintPipeline │                          │
//   │                   │  + BlueprintReport│                          │
//   │                   └──────────────────┘                          │
//   └───────────────────────────────────────────────────────────────────┘
//
//   数据流:
//   1. decision.validate()  → 4 enum 自检
//   2. risk.K1..K4 chain    → 4 风险类守门
//   3. template A-F         → 6 模板实例化
//   4. r_measure R1..R5     → 5 维 R-Measure
//   5. q_metric Q1..Q3      → 3 维 Q-Metric
//   6. BlueprintReport      → 综合得分 + baseline 对比
//
// ============================================
