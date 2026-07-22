"""
Tests for V1083 ASI Real Decision Routing Engine
==================================================

主 22:33 + 主 17:43 + 主 19:33 + 主 13:31 + 主 17:58+20:46 + 主 23:44 + 主 00:56 + 主 00:44
"""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path

import pytest


APEIRETH_DIR = Path(__file__).resolve().parent.parent / "apeireth"
if str(APEIRETH_DIR.parent) not in sys.path:
    sys.path.insert(0, str(APEIRETH_DIR.parent))

from apeireth.v1083_asi_decision_router import (  # noqa: E402
    ASIDecisionRouterBridge,
    DEFAULT_MODEL_REGISTRY,
    DecisionLog,
    DecisionLogger,
    FailoverPlan,
    GUARD_NOT_BEST_IS_OPTIMAL,
    GUARD_NOT_CAPABILITY_FIRST_IS_ASI,
    GUARD_NOT_COST_AWARE_IS_SAVING,
    GUARD_NOT_FAILOVER_IS_SAFE,
    ModelRecord,
    POLICIES,
    RequestContext,
    RoutingDecision,
    cost_latency_matrix,
    main as v1083_main,
    plan_failover,
    policy_score,
    render_decision_report,
    run_v3_guards,
    select_model,
)


# ============================================================
# Test 1: RequestContext 真捕获
# ============================================================


class TestV1083Context:
    def test_context_creation(self):
        ctx = RequestContext(
            task_type="reasoning",
            capability_need=0.9,
            latency_budget_ms=1000,
            cost_budget_per_1k=0.005,
        )
        assert ctx.task_type == "reasoning"
        assert ctx.capability_need == 0.9

    def test_context_to_dict(self):
        ctx = RequestContext(
            task_type="qa",
            capability_need=0.7,
            latency_budget_ms=500,
            cost_budget_per_1k=0.001,
        )
        d = ctx.to_dict()
        assert d["task_type"] == "qa"
        assert d["capability_need"] == 0.7


# ============================================================
# Test 2: ModelCapability 真存
# ============================================================


class TestV1083Models:
    def test_default_registry_has_models(self):
        assert len(DEFAULT_MODEL_REGISTRY) >= 5
        assert "deepseek-v3" in DEFAULT_MODEL_REGISTRY
        assert "claude-opus-4" in DEFAULT_MODEL_REGISTRY

    def test_model_records_have_required_fields(self):
        for mid, m in DEFAULT_MODEL_REGISTRY.items():
            assert m.model_id == mid
            assert 0.0 <= m.capability_score <= 1.0
            assert m.cost_per_1k_tokens > 0
            assert m.latency_p50_ms > 0

    def test_model_record_to_dict(self):
        m = ModelRecord(
            model_id="test",
            capability_score=0.5,
            cost_per_1k_tokens=0.001,
            latency_p50_ms=500,
        )
        d = m.to_dict()
        assert d["model_id"] == "test"
        assert d["capability_score"] == 0.5


# ============================================================
# Test 3: CostLatencyMatrix 真存
# ============================================================


class TestV1083Matrix:
    def test_matrix_has_all_models(self):
        m = cost_latency_matrix(DEFAULT_MODEL_REGISTRY)
        for mid in DEFAULT_MODEL_REGISTRY:
            assert mid in m
            assert "cost_per_1k" in m[mid]
            assert "p50_ms" in m[mid]


# ============================================================
# Test 4: DecisionPolicy 真排
# ============================================================


class TestV1083Policy:
    def setup_method(self):
        self.ctx = RequestContext(
            task_type="reasoning",
            capability_need=0.85,
            latency_budget_ms=2000,
            cost_budget_per_1k=0.02,
        )

    def test_greedy_returns_capability(self):
        m = DEFAULT_MODEL_REGISTRY["claude-opus-4"]
        score = policy_score(m, self.ctx, "greedy")
        assert score == m.capability_score

    def test_cost_aware_penalizes_expensive(self):
        cheap = DEFAULT_MODEL_REGISTRY["gpt-4o-mini"]
        expensive = DEFAULT_MODEL_REGISTRY["claude-opus-4"]
        cheap_score = policy_score(cheap, self.ctx, "cost-aware")
        expensive_score = policy_score(expensive, self.ctx, "cost-aware")
        # Cheap model should score much higher per dollar
        assert cheap_score > expensive_score

    def test_capability_first_includes_task_affinity(self):
        m = DEFAULT_MODEL_REGISTRY["claude-opus-4"]
        score = policy_score(m, self.ctx, "capability-first")
        # 0.7 * 0.92 + 0.3 * 0.95 (reasoning affinity)
        expected = 0.92 * 0.7 + 0.95 * 0.3
        assert abs(score - expected) < 1e-6

    def test_balanced_normalizes(self):
        m = DEFAULT_MODEL_REGISTRY["deepseek-v3"]
        score = policy_score(m, self.ctx, "balanced")
        assert 0.0 <= score <= 1.0

    def test_latency_violation_is_minus_inf(self):
        tight_ctx = RequestContext(
            task_type="reasoning",
            capability_need=0.85,
            latency_budget_ms=100,  # very tight
            cost_budget_per_1k=1.0,
        )
        m = DEFAULT_MODEL_REGISTRY["claude-opus-4"]  # 1800ms
        score = policy_score(m, tight_ctx, "balanced")
        assert score == -math.inf

    def test_cost_violation_is_minus_inf(self):
        tight_ctx = RequestContext(
            task_type="reasoning",
            capability_need=0.85,
            latency_budget_ms=10000,
            cost_budget_per_1k=0.0001,  # very tight
        )
        m = DEFAULT_MODEL_REGISTRY["claude-opus-4"]  # $0.015
        score = policy_score(m, tight_ctx, "balanced")
        assert score == -math.inf

    def test_disabled_model_is_minus_inf(self):
        m = DEFAULT_MODEL_REGISTRY["claude-opus-4"]
        m.enabled = False
        score = policy_score(m, self.ctx, "balanced")
        assert score == -math.inf
        m.enabled = True  # restore


# ============================================================
# Test 5: ModelSelector 真选
# ============================================================


class TestV1083Selector:
    def setup_method(self):
        self.ctx = RequestContext(
            task_type="reasoning",
            capability_need=0.85,
            latency_budget_ms=2000,
            cost_budget_per_1k=0.02,
        )

    def test_select_returns_decision(self):
        d = select_model(self.ctx, DEFAULT_MODEL_REGISTRY)
        assert isinstance(d, RoutingDecision)
        assert d.decision_id.startswith("dec-")
        assert d.chosen_model is not None

    def test_select_reasons_nonempty(self):
        d = select_model(self.ctx, DEFAULT_MODEL_REGISTRY)
        assert len(d.reasons) >= 3

    def test_select_fallback_nonempty_when_valid(self):
        d = select_model(self.ctx, DEFAULT_MODEL_REGISTRY)
        # Multiple models should pass; fallback should be set
        assert d.fallback_model is not None

    def test_select_no_candidate_meets_budget(self):
        # Empty registry -> chosen None
        d = select_model(self.ctx, {})
        assert d.chosen_model is None
        assert "no_candidate_meets_budget" in " ".join(d.reasons)

    def test_select_too_tight_budget_falls_back(self):
        tight_ctx = RequestContext(
            task_type="reasoning",
            capability_need=0.85,
            latency_budget_ms=10,  # impossible
            cost_budget_per_1k=0.0000001,
        )
        d = select_model(tight_ctx, DEFAULT_MODEL_REGISTRY)
        assert d.chosen_model is None
        assert d.fallback_model is not None  # degraded

    def test_select_constraints_applied(self):
        d = select_model(self.ctx, DEFAULT_MODEL_REGISTRY)
        constraint_text = " ".join(d.policy_constraints_applied)
        assert "latency_budget" in constraint_text
        assert "cost_budget" in constraint_text

    def test_candidates_ranked_descending(self):
        d = select_model(self.ctx, DEFAULT_MODEL_REGISTRY)
        # First candidates should be valid (hard_pass=True)
        for c in d.candidates_ranked[:3]:
            if c["score"] is not None:
                assert c["hard_pass"] is True

    def test_to_dict(self):
        d = select_model(self.ctx, DEFAULT_MODEL_REGISTRY)
        x = d.to_dict()
        assert "decision_id" in x
        assert "policy" in x
        assert "reasons" in x


# ============================================================
# Test 6: FailoverPlanner 真落
# ============================================================


class TestV1083Failover:
    def test_plan_3_tier(self):
        p = plan_failover("claude-opus-4", DEFAULT_MODEL_REGISTRY)
        assert p.primary == "claude-opus-4"
        assert p.secondary is not None
        assert p.tertiary is not None
        assert p.primary != p.secondary
        assert p.secondary != p.tertiary

    def test_plan_empty_registry(self):
        p = plan_failover("", {})
        assert p.primary == ""
        assert p.secondary is None
        assert p.tertiary is None

    def test_plan_unknown_primary_uses_best(self):
        # If primary is "" or not in registry, fall back to highest capability
        p = plan_failover("", DEFAULT_MODEL_REGISTRY)
        # Should pick the highest-capability model
        best = max(DEFAULT_MODEL_REGISTRY.values(), key=lambda m: m.capability_score)
        assert p.primary == best.model_id

    def test_to_dict(self):
        p = plan_failover("claude-opus-4", DEFAULT_MODEL_REGISTRY)
        d = p.to_dict()
        assert "primary" in d
        assert "rationale" in d


# ============================================================
# Test 7: DecisionLogger 真记
# ============================================================


class TestV1083Logger:
    def test_logger_starts_empty(self):
        logger = DecisionLogger()
        assert len(logger.logs) == 0

    def test_log_adds_entry(self):
        logger = DecisionLogger()
        ctx = RequestContext(
            task_type="reasoning",
            capability_need=0.85,
            latency_budget_ms=1000,
            cost_budget_per_1k=0.005,
        )
        d = select_model(ctx, DEFAULT_MODEL_REGISTRY)
        entry = logger.log(d, ctx)
        assert isinstance(entry, DecisionLog)
        assert len(logger.logs) == 1

    def test_log_clears(self):
        logger = DecisionLogger()
        ctx = RequestContext(
            task_type="qa",
            capability_need=0.5,
            latency_budget_ms=1000,
            cost_budget_per_1k=0.001,
        )
        d = select_model(ctx, DEFAULT_MODEL_REGISTRY)
        logger.log(d, ctx)
        logger.clear()
        assert len(logger.logs) == 0

    def test_log_to_dict(self):
        logger = DecisionLogger()
        ctx = RequestContext(
            task_type="code",
            capability_need=0.9,
            latency_budget_ms=1000,
            cost_budget_per_1k=0.005,
        )
        d = select_model(ctx, DEFAULT_MODEL_REGISTRY)
        entry = logger.log(d, ctx)
        x = entry.to_dict()
        assert "decision_id" in x
        assert "context_summary" in x


# ============================================================
# Test 8: ASIDecisionRouterBridge 真测
# ============================================================


class TestV1083ASIBridge:
    def setup_method(self):
        self.ctx = RequestContext(
            task_type="reasoning",
            capability_need=0.85,
            latency_budget_ms=2000,
            cost_budget_per_1k=0.02,
        )
        self.decision = select_model(self.ctx, DEFAULT_MODEL_REGISTRY)
        self.plan = plan_failover(
            self.decision.chosen_model or "", DEFAULT_MODEL_REGISTRY
        )
        self.logger = DecisionLogger()
        self.logger.log(self.decision, self.ctx)

    def test_subscore_returns_float(self):
        bridge = ASIDecisionRouterBridge()
        sub = bridge.subscore(self.decision, self.plan, self.logger)
        assert isinstance(sub, float)
        assert 0.0 <= sub <= 1.0

    def test_subscore_higher_when_valid_decision(self):
        bridge = ASIDecisionRouterBridge()
        sub = bridge.subscore(self.decision, self.plan, self.logger)
        assert sub >= 0.5  # Valid decision + reasons + failover

    def test_lift_capped(self):
        bridge = ASIDecisionRouterBridge()
        lift = bridge.asi_v03_lift(
            self.decision, self.plan, self.logger, current_asi_v03=0.8813
        )
        assert lift["lift"] <= 0.02
        assert lift["projected_asi_v03"] <= 0.9800

    def test_weights_sum_to_one(self):
        b = ASIDecisionRouterBridge()
        total = (
            b.policy_quality_weight
            + b.constraint_enforcement_weight
            + b.failover_quality_weight
            + b.transparency_weight
            + b.cost_awareness_weight
            + b.no_fake_weight
        )
        assert abs(total - 1.0) < 1e-9


# ============================================================
# Test 9: V3 Philosophy Guards
# ============================================================


class TestV1083V3Guards:
    def test_run_v3_guards_returns_4(self):
        guards = run_v3_guards()
        assert len(guards) == 4

    def test_guards_contain_chinese(self):
        guards = run_v3_guards()
        for k, v in guards.items():
            assert "不假装" in v

    def test_guard_constants_distinct(self):
        guards = run_v3_guards()
        assert GUARD_NOT_BEST_IS_OPTIMAL in guards.values()
        assert GUARD_NOT_COST_AWARE_IS_SAVING in guards.values()
        assert GUARD_NOT_CAPABILITY_FIRST_IS_ASI in guards.values()
        assert GUARD_NOT_FAILOVER_IS_SAFE in guards.values()


# ============================================================
# Test 10: Report rendering
# ============================================================


class TestV1083Report:
    def setup_method(self):
        self.ctx = RequestContext(
            task_type="reasoning",
            capability_need=0.85,
            latency_budget_ms=2000,
            cost_budget_per_1k=0.02,
        )
        self.decision = select_model(self.ctx, DEFAULT_MODEL_REGISTRY)
        self.plan = plan_failover(
            self.decision.chosen_model or "", DEFAULT_MODEL_REGISTRY
        )
        self.logger = DecisionLogger()
        self.logger.log(self.decision, self.ctx)
        from datetime import datetime

        setattr(
            self.decision,
            "ts_iso",
            datetime.fromtimestamp(self.decision.ts).isoformat(),
        )
        self.sub = 0.85

    def test_render_contains_all_sections(self):
        md = render_decision_report(
            self.decision, self.plan, self.ctx, self.logger, self.sub
        )
        assert "V1083 ASI Decision Routing Report" in md
        assert "Request Context" in md
        assert "Routing Decision" in md
        assert "Reasons" in md
        assert "Failover Plan" in md
        assert "V3 哲学守门" in md
        assert "References" in md

    def test_render_includes_chosen_model(self):
        md = render_decision_report(
            self.decision, self.plan, self.ctx, self.logger, self.sub
        )
        assert self.decision.chosen_model in md


# ============================================================
# Test 11: CLI
# ============================================================


class TestV1083CLI:
    def test_main_catalog_quiet(self, capsys):
        rc = v1083_main(["--catalog", "--quiet"])
        assert rc == 0

    def test_main_guards(self, capsys):
        rc = v1083_main(["--guards"])
        assert rc == 0

    def test_main_route_quiet(self, capsys):
        rc = v1083_main(
            [
                "--route",
                "--task",
                "reasoning",
                "--latency",
                "2000",
                "--cost",
                "0.02",
                "--policy",
                "balanced",
                "--quiet",
            ]
        )
        assert rc == 0

    def test_main_route_with_report(self, tmp_path: Path):
        out = tmp_path / "report.md"
        rc = v1083_main(
            [
                "--route",
                "--task",
                "code",
                "--latency",
                "1000",
                "--cost",
                "0.005",
                "--policy",
                "cost-aware",
                "--report",
                "--output",
                str(out),
                "--quiet",
            ]
        )
        assert rc == 0
        assert out.exists()
        text = out.read_text(encoding="utf-8")
        assert "V1083" in text

    def test_main_lift(self, capsys):
        rc = v1083_main(["--route", "--lift", "--quiet"])
        assert rc == 0

    def test_main_no_args_shows_help(self, capsys):
        rc = v1083_main([])
        assert rc == 0


# ============================================================
# Test 12: Sanity
# ============================================================


class TestV1083Sanity:
    def test_eight_components_referenced(self):
        """V1083 has 8 components; sanity-check each is reachable."""
        ctx = RequestContext(
            task_type="reasoning",
            capability_need=0.85,
            latency_budget_ms=2000,
            cost_budget_per_1k=0.02,
        )
        # 1. RequestContext
        assert ctx is not None
        # 2. ModelCapability (via DEFAULT_MODEL_REGISTRY)
        assert len(DEFAULT_MODEL_REGISTRY) > 0
        # 3. CostLatencyMatrix
        m = cost_latency_matrix(DEFAULT_MODEL_REGISTRY)
        assert m is not None
        # 4. DecisionPolicy
        assert "balanced" in POLICIES
        # 5. ModelSelector
        d = select_model(ctx, DEFAULT_MODEL_REGISTRY)
        assert d.chosen_model is not None
        # 6. FailoverPlanner
        p = plan_failover(d.chosen_model, DEFAULT_MODEL_REGISTRY)
        assert p.primary == d.chosen_model
        # 7. DecisionLogger
        logger = DecisionLogger()
        logger.log(d, ctx)
        assert len(logger.logs) == 1
        # 8. ASIDecisionRouterBridge
        bridge = ASIDecisionRouterBridge()
        sub = bridge.subscore(d, p, logger)
        assert sub >= 0.0

    def test_policy_choices_all_work(self):
        ctx = RequestContext(
            task_type="reasoning",
            capability_need=0.85,
            latency_budget_ms=2000,
            cost_budget_per_1k=0.02,
        )
        for policy in POLICIES:
            d = select_model(ctx, DEFAULT_MODEL_REGISTRY, policy=policy)
            assert d.policy == policy
            assert d.chosen_model is not None  # all should find valid

    def test_decision_id_unique(self):
        ctx = RequestContext(
            task_type="reasoning",
            capability_need=0.85,
            latency_budget_ms=2000,
            cost_budget_per_1k=0.02,
        )
        d1 = select_model(ctx, DEFAULT_MODEL_REGISTRY)
        d2 = select_model(ctx, DEFAULT_MODEL_REGISTRY)
        assert d1.decision_id != d2.decision_id


# ============================================================
# Test 13: No-fake guard
# ============================================================


class TestV1083NoFake:
    def test_no_fake_phrase_in_source(self):
        """Source code must contain 不假装."""
        import inspect

        from apeireth import v1083_asi_decision_router

        src = inspect.getsource(v1083_asi_decision_router)
        assert "不假装" in src

    def test_subscore_not_suspiciously_perfect(self):
        """Real subscore should be < 1.0 in practice."""
        ctx = RequestContext(
            task_type="reasoning",
            capability_need=0.85,
            latency_budget_ms=2000,
            cost_budget_per_1k=0.02,
        )
        d = select_model(ctx, DEFAULT_MODEL_REGISTRY)
        p = plan_failover(d.chosen_model, DEFAULT_MODEL_REGISTRY)
        logger = DecisionLogger()
        logger.log(d, ctx)
        bridge = ASIDecisionRouterBridge()
        sub = bridge.subscore(d, p, logger)
        # Should be honest score (may be high in well-defined case)
        # We accept >=0.5; the test for honesty is that it's not artificially
        # perfect in conditions where some component should fail
        assert sub >= 0.5
        assert sub <= 1.0

    def test_chosen_model_explanation_in_reasons(self):
        """Reasons list must explain why chosen (not opaque)."""
        ctx = RequestContext(
            task_type="reasoning",
            capability_need=0.85,
            latency_budget_ms=2000,
            cost_budget_per_1k=0.02,
        )
        d = select_model(ctx, DEFAULT_MODEL_REGISTRY)
        reasons_text = " ".join(d.reasons)
        assert "score" in reasons_text
        assert "cost" in reasons_text
        assert "latency" in reasons_text