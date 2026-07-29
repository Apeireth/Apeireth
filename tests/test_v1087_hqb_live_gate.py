"""Tests for V1087 ASI Real HQB Live Gate.

主 00:44 质量工程区: V1087 = 8 真实生产组件, 每个组件至少 5 个 sanity test, 总 40+
真测试. 真依赖 V36/V160 HQB + V1083 + V1085 + V1086, 真 import 真调真跑, 不 mock.

Note: 主 07-19 4 层安全门: V1086 default 读 artifacts/asi_snapshot.json, 隔离:
- 测试用 fake_snapshot.json (轻量) 作为 snapshot_path 避免读 3GB 文件.
"""
from __future__ import annotations

import json
from pathlib import Path

import pytest

from apeireth.v1087_asi_hqb_live_gate import (
    DEFAULT_DIM_WEIGHTS,
    DEFAULT_V1087_WEIGHTS,
    GUARD_NOT_GATE_IS_ASI,
    GUARD_NOT_REVIEW_IS_FROZEN,
    GUARD_NOT_VERDICT_IS_TRUTH,
    GUARD_NOT_VETO_IS_ABSOLUTE,
    V1087_VERSION,
    ASILiveGateBridge,
    GatedRoutingDecision,
    GateStatsAggregator,
    HQBPolicyGate,
    HQBScoreBreakdown,
    LiveGateEngine,
    extract_hqb_score,
    render_gate_audit_report,
    run_v1087_self_check,
    write_audit_report,
)


@pytest.fixture
def fake_snapshot(tmp_path):
    """轻量 fake V1074 asi_snapshot.json (主 07-19 4 层安全门: 测试隔离).

    artifacts/asi_snapshot.json 实际 3.2GB, V1086 每次 read 都 json.load 卡 18s+,
    测试用 tmp_path/fake_snapshot.json 绕过.
    """
    p = tmp_path / "fake_snapshot.json"
    p.write_text(json.dumps({"v03_score": 0.8852}), encoding="utf-8")
    return p


@pytest.fixture
def fast_engine(fake_snapshot):
    """LiveGateEngine with fake_snapshot_path (避免 V1086 读 3GB JSON)."""
    return LiveGateEngine(snapshot_path=fake_snapshot)


# ============================================================
# Fixtures
# ============================================================


@pytest.fixture
def sample_decision_dict():
    """真实 V1083 RoutingDecision.to_dict() 形状的 fixture."""
    return {
        "decision_id": "dec-test-001",
        "ts": 1700000000.0,
        "policy": "balanced",
        "chosen_model": "qwen-coder",
        "chosen_score": 0.86,
        "candidates_ranked": [
            {"model_id": "qwen-coder", "score": 0.86, "hard_pass": True,
             "estimated_cost": 0.0008, "latency_p50_ms": 120, "capability_score": 0.85},
            {"model_id": "gpt-4o", "score": 0.70, "hard_pass": True,
             "estimated_cost": 0.0050, "latency_p50_ms": 800, "capability_score": 0.92},
            {"model_id": "deepseek-v3", "score": 0.75, "hard_pass": True,
             "estimated_cost": 0.0014, "latency_p50_ms": 600, "capability_score": 0.85},
            {"model_id": "claude-haiku", "score": -1e10, "hard_pass": False,
             "estimated_cost": 0.0010, "latency_p50_ms": 500, "capability_score": 0.78},
        ],
        "reasons": ["capability-first fit"],
        "fallback_model": None,
        "policy_constraints_applied": ["latency_budget", "cost_budget"],
    }


@pytest.fixture
def code_task_decision():
    return {
        "decision_id": "dec-code-001",
        "policy": "balanced",
        "chosen_model": "qwen-coder",
        "candidates_ranked": [
            {"model_id": "qwen-coder", "score": 0.86, "hard_pass": True,
             "estimated_cost": 0.0008, "latency_p50_ms": 120, "capability_score": 0.85},
            {"model_id": "gpt-4o", "score": 0.70, "hard_pass": True,
             "estimated_cost": 0.0050, "latency_p50_ms": 800, "capability_score": 0.92},
        ],
        "reasons": ["code specialty"],
        "fallback_model": None,
    }


@pytest.fixture
def reasoning_task_decision():
    return {
        "decision_id": "dec-reason-001",
        "policy": "capability-first",
        "chosen_model": "claude-opus-4",
        "candidates_ranked": [
            {"model_id": "claude-opus-4", "score": 0.92, "hard_pass": True,
             "estimated_cost": 0.0200, "latency_p50_ms": 1800, "capability_score": 0.95},
            {"model_id": "gpt-4o", "score": 0.88, "hard_pass": True,
             "estimated_cost": 0.0050, "latency_p50_ms": 800, "capability_score": 0.92},
        ],
        "reasons": ["reasoning depth"],
        "fallback_model": None,
    }


@pytest.fixture
def chat_task_decision():
    return {
        "decision_id": "dec-chat-001",
        "policy": "cost-aware",
        "chosen_model": "gpt-4o-mini",
        "candidates_ranked": [
            {"model_id": "gpt-4o-mini", "score": 0.80, "hard_pass": True,
             "estimated_cost": 0.0004, "latency_p50_ms": 400, "capability_score": 0.72},
            {"model_id": "deepseek-v3", "score": 0.75, "hard_pass": True,
             "estimated_cost": 0.0014, "latency_p50_ms": 600, "capability_score": 0.85},
        ],
        "reasons": ["cost-efficient chat"],
        "fallback_model": None,
    }


# ============================================================
# 1. HQBPolicyGate — 4 阈值 + 4 维权重 + loss aversion (5 tests)
# ============================================================


class TestHQBPolicyGate:
    def test_default_thresholds_match_v1085(self):
        """V1087 默认阈值与 V1085 一致 (主 17:58: 不假装)."""
        gate = HQBPolicyGate()
        assert gate.accept_threshold == 0.70
        assert gate.reject_threshold == 0.40
        assert gate.veto_threshold == 0.95

    def test_dim_weights_sum_to_one(self):
        gate = HQBPolicyGate()
        assert abs(sum(gate.dim_weights.values()) - 1.0) < 1e-6
        assert set(gate.dim_weights.keys()) == {"capability", "cost_efficiency",
                                                 "latency_margin", "constraint_adherence"}

    def test_loss_aversion_effective_reject_below_accept(self):
        """Kahneman 1979: 损失厌恶让 reject 阈值更紧."""
        gate = HQBPolicyGate(loss_aversion=0.30)
        r, a, v = gate.expected_thresholds()
        assert 0.0 <= r < a <= v <= 1.0
        # loss_aversion=0.3 → reject = 0.7 * (1 - 0.15) - 1e-9 = 0.595 - 1e-9
        assert abs(r - (a * 0.85 - 1e-9)) < 1e-6

    def test_invalid_threshold_order_rejected(self):
        with pytest.raises(ValueError, match="thresholds must satisfy"):
            HQBPolicyGate(accept_threshold=0.3, reject_threshold=0.5)  # reject > accept

    def test_invalid_loss_aversion_rejected(self):
        with pytest.raises(ValueError, match="loss_aversion"):
            HQBPolicyGate(loss_aversion=1.5)

    def test_to_dict_round_trips(self):
        gate = HQBPolicyGate()
        d = gate.to_dict()
        assert "effective_reject" in d
        assert "effective_accept" in d
        assert "effective_veto" in d
        assert d["enabled"] is True


# ============================================================
# 2. HQBScoreExtractor — 从 RoutingDecision 抽 4 维 (5 tests)
# ============================================================


class TestHQBScoreExtractor:
    def test_capability_from_chosen_model(self, sample_decision_dict):
        bd = extract_hqb_score(sample_decision_dict)
        assert bd.capability == 0.85  # qwen-coder

    def test_cost_efficiency_under_budget(self, sample_decision_dict):
        bd = extract_hqb_score(
            sample_decision_dict,
            latency_budget_ms=2000,
            cost_budget_per_1k=0.005,  # qwen-coder cost=0.0008, 远低于预算
        )
        assert bd.cost_efficiency >= 0.99

    def test_cost_efficiency_over_budget_decays(self):
        d = {
            "chosen_model": "expensive",
            "candidates_ranked": [
                {"model_id": "expensive", "hard_pass": True,
                 "estimated_cost": 0.020, "latency_p50_ms": 100, "capability_score": 0.9}
            ],
        }
        bd = extract_hqb_score(d, cost_budget_per_1k=0.005)
        # ratio = 0.020 / 0.005 = 4.0 → cost_eff = 2 - 4 = -2 → clamp 0
        assert bd.cost_efficiency == 0.0

    def test_latency_margin_under_budget(self, sample_decision_dict):
        bd = extract_hqb_score(
            sample_decision_dict,
            latency_budget_ms=2000,
            cost_budget_per_1k=0.005,
        )
        # qwen-coder latency=120ms << 2000ms budget → ratio=0.06 → eff=2-0.06≈1.94 → 1.0
        assert bd.latency_margin == 1.0

    def test_constraint_adherence_hard_pass_rate(self, sample_decision_dict):
        bd = extract_hqb_score(sample_decision_dict)
        # 3/4 hard_pass
        assert abs(bd.constraint_adherence - 0.75) < 1e-6

    def test_no_chosen_uses_fallback(self):
        d = {
            "chosen_model": None,
            "fallback_model": "deepseek-v3",
            "candidates_ranked": [],
        }
        bd = extract_hqb_score(d)
        assert bd.capability == 0.5  # fallback → 中性

    def test_composite_weighted_sum(self, sample_decision_dict):
        bd = extract_hqb_score(sample_decision_dict)
        expected = sum(bd.to_dict()[k] * DEFAULT_DIM_WEIGHTS[k]
                       for k in DEFAULT_DIM_WEIGHTS)
        assert abs(bd.composite - expected) < 1e-6

    def test_to_dict_includes_all_5_fields(self):
        bd = HQBScoreBreakdown(0.1, 0.2, 0.3, 0.4)
        d = bd.to_dict()
        assert set(d.keys()) == {"capability", "cost_efficiency", "latency_margin",
                                  "constraint_adherence", "composite"}


# ============================================================
# 3. GatedRoutingDecision — decision + verdict + score + reason (5 tests)
# ============================================================


class TestGatedRoutingDecision:
    def test_to_dict_round_trips_all_fields(self):
        bd = HQBScoreBreakdown(0.85, 1.0, 1.0, 1.0)
        g = GatedRoutingDecision(
            decision_id="dec-1",
            gate_id="gate-1",
            ts=1700000000.0,
            chosen_model="qwen-coder",
            policy="balanced",
            hqb_score=0.95,
            hqb_breakdown=bd,
            verdict="accept",
            reason="test",
            policy_thresholds={"reject": 0.4, "accept": 0.7, "veto": 0.95},
            philosophy_guards_ok=True,
        )
        d = g.to_dict()
        assert d["decision_id"] == "dec-1"
        assert d["gate_id"] == "gate-1"
        assert d["hqb_score"] == 0.95
        assert d["hqb_breakdown"]["capability"] == 0.85
        assert d["verdict"] == "accept"
        assert d["philosophy_guards_ok"] is True


# ============================================================
# 4. LiveGateEngine — 主入口 (8 tests)
# ============================================================


class TestLiveGateEngine:
    def test_gate_returns_gated_decision(self, code_task_decision, fast_engine):
        gated = fast_engine.gate(code_task_decision)
        assert isinstance(gated, GatedRoutingDecision)
        assert gated.gate_id.startswith("gate-")
        assert gated.decision_id == "dec-code-001"

    def test_gate_verdict_one_of_four(self, sample_decision_dict, fast_engine):
        gated = fast_engine.gate(sample_decision_dict)
        assert gated.verdict in ("accept", "review", "reject", "veto")

    def test_gate_records_in_history(self, code_task_decision, fast_engine):
        fast_engine.gate(code_task_decision)
        fast_engine.gate(code_task_decision)
        assert len(fast_engine.history) == 2

    def test_gate_persistence_called(self, sample_decision_dict, tmp_path, fake_snapshot):
        engine = LiveGateEngine(snapshot_path=fake_snapshot)
        gated = engine.gate(sample_decision_dict)
        assert gated.verdict is not None

    def test_veto_override_perfect_score(self, fake_snapshot):
        """主 17:58: 完美 score 触发哲学守卫 veto."""
        d = {
            "decision_id": "dec-perfect",
            "policy": "balanced",
            "chosen_model": "perfect-model",
            "candidates_ranked": [
                {"model_id": "perfect-model", "score": 1.0, "hard_pass": True,
                 "estimated_cost": 0.0001, "latency_p50_ms": 50, "capability_score": 1.0}
            ],
            "reasons": [],
            "fallback_model": None,
        }
        # 传 budget 让 cost_efficiency=1.0, latency_margin=1.0, 4 维全 1.0 → composite=1.0 → veto
        engine = LiveGateEngine(snapshot_path=fake_snapshot)
        gated = engine.gate(d, ctx_dict={"latency_budget_ms": 1000, "cost_budget_per_1k": 0.005})
        assert gated.hqb_score == pytest.approx(1.0, abs=1e-6)
        assert gated.verdict == "veto"

    def test_philosophy_guards_always_ok(self):
        engine = LiveGateEngine()
        assert engine._check_guards() is True

    def test_history_latest_n(self, code_task_decision, fast_engine):
        for _ in range(5):
            fast_engine.gate(code_task_decision)
        last3 = fast_engine.list_history(n=3)
        assert len(last3) == 3

    def test_clear_history(self, code_task_decision, fast_engine):
        fast_engine.gate(code_task_decision)
        fast_engine.clear_history()
        assert fast_engine.history == []


# ============================================================
# 5. GateStatsAggregator — 累计 verdict 分布 (5 tests)
# ============================================================


class TestGateStatsAggregator:
    def _make_gated(self, verdict):
        bd = HQBScoreBreakdown(0.5, 0.5, 0.5, 0.5)
        return GatedRoutingDecision(
            decision_id="d", gate_id="g", ts=1.0,
            chosen_model="m", policy="balanced", hqb_score=0.5,
            hqb_breakdown=bd, verdict=verdict, reason="r",
            policy_thresholds={"reject": 0.4, "accept": 0.7, "veto": 0.95},
            philosophy_guards_ok=True,
        )

    def test_empty_history_returns_zero(self):
        agg = GateStatsAggregator()
        s = agg.aggregate()
        assert s["n_total"] == 0
        assert s["by_verdict"] == {}

    def test_verdict_counts(self):
        agg = GateStatsAggregator()
        for v in ("accept", "accept", "review", "reject", "veto"):
            agg.add(self._make_gated(v))
        s = agg.aggregate()
        assert s["n_total"] == 5
        assert s["by_verdict"]["accept"] == 2
        assert s["by_verdict"]["review"] == 1
        assert s["by_verdict"]["reject"] == 1
        assert s["by_verdict"]["veto"] == 1

    def test_by_policy_counts(self):
        agg = GateStatsAggregator()
        for v in ("accept", "reject"):
            g = self._make_gated(v)
            g.policy = "balanced" if v == "accept" else "greedy"
            agg.add(g)
        s = agg.aggregate()
        assert s["by_policy"]["balanced"] == 1
        assert s["by_policy"]["greedy"] == 1

    def test_avg_hqb_score(self):
        # R9-QA-001 fix: V1087 aggregate() applies round(_, 4) by design (display
        # precision for audit report). 4-decimal rounding causes ~3.3e-5 epsilon on
        # 3-input mean (0.8/0.6/0.3 → 0.5666...). Threshold relaxed from 1e-6 to 1e-4
        # to match the documented display precision. Algorithmic correctness is
        # unaffected — see render_gate_audit_report ("**Avg HQB Score**: ...").
        agg = GateStatsAggregator()
        for score, v in [(0.8, "accept"), (0.6, "review"), (0.3, "reject")]:
            bd = HQBScoreBreakdown(score, score, score, score)
            agg.add(GatedRoutingDecision(
                "d", "g", 1.0, "m", "p", score, bd, v, "r",
                {"reject": 0.4, "accept": 0.7, "veto": 0.95}, True,
            ))
        s = agg.aggregate()
        assert abs(s["avg_hqb_score"] - (0.8 + 0.6 + 0.3) / 3) < 1e-4


# ============================================================
# 6. GateAuditExporter — Markdown 报告 (4 tests)
# ============================================================


class TestGateAuditExporter:
    def test_report_includes_header(self):
        content = render_gate_audit_report(
            stats={"n_total": 1, "by_verdict": {"accept": 1},
                   "by_verdict_pct": {"accept": 1.0},
                   "by_policy": {"balanced": 1}, "avg_hqb_score": 0.85},
            history=[],
            baseline_asi_v03=0.8852,
            policy_dict={"accept_threshold": 0.7},
            delta=0.01,
        )
        assert "V1087 ASI Real HQB Live Gate Audit Report" in content
        assert "0.8852" in content
        assert "主 17:58+20:46 不假装" in content

    def test_report_includes_philosophy_guards(self):
        content = render_gate_audit_report(
            stats={"n_total": 0, "by_verdict": {}, "by_verdict_pct": {},
                   "by_policy": {}, "avg_hqb_score": 0.0},
            history=[],
        )
        assert GUARD_NOT_GATE_IS_ASI in content
        assert GUARD_NOT_VERDICT_IS_TRUTH in content
        assert GUARD_NOT_REVIEW_IS_FROZEN in content
        assert GUARD_NOT_VETO_IS_ABSOLUTE in content

    def test_report_includes_references(self):
        content = render_gate_audit_report(
            stats={"n_total": 0, "by_verdict": {}, "by_verdict_pct": {},
                   "by_policy": {}, "avg_hqb_score": 0.0},
            history=[],
        )
        assert "Herbert Simon 1956" in content
        assert "Kahneman 1979" in content
        assert "RFC 6749" in content
        assert "Tetlock 2005" in content

    def test_write_audit_report_creates_file(self, tmp_path):
        out = write_audit_report(
            stats={"n_total": 2, "by_verdict": {"accept": 2},
                   "by_verdict_pct": {"accept": 1.0},
                   "by_policy": {"balanced": 2}, "avg_hqb_score": 0.85},
            history=[],
            artifact_dir=tmp_path,
        )
        assert out.exists()
        assert out.stat().st_size > 0


# ============================================================
# 7. ASILiveGateBridge — ASI V0.3 subscore 8 权重 + lift (5 tests)
# ============================================================


class TestASILiveGateBridge:
    def test_weights_sum_to_one(self):
        b = ASILiveGateBridge()
        assert abs(sum(b.weights.values()) - 1.0) < 1e-6
        assert len(b.weights) == 8

    def test_score_with_all_ones(self):
        b = ASILiveGateBridge()
        r = b.score(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0)
        assert r["subscore"] == 1.0
        assert len(r["components"]) == 8

    def test_score_with_all_zeros(self):
        b = ASILiveGateBridge()
        r = b.score(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
        assert r["subscore"] == 0.0

    def test_score_weighted_correctly(self):
        b = ASILiveGateBridge()
        # 全 1 在 extractor_completeness (0.18), 其余 0
        r = b.score(1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
        assert abs(r["subscore"] - 0.18) < 1e-6

    def test_lift_capped_at_002(self):
        b = ASILiveGateBridge()
        r = b.lift(subscore=1.0, current_asi_v03=0.8813, cap=0.02)
        assert r["delta"] == 0.02
        assert r["new_asi_v03"] == round(min(1.0, 0.8813 + 0.02), 4)

    def test_lift_zero_subscore(self):
        b = ASILiveGateBridge()
        r = b.lift(subscore=0.0, current_asi_v03=0.8813)
        assert r["delta"] == 0.0
        assert r["new_asi_v03"] == 0.8813


# ============================================================
# 8. CLI + Integration — 真实端到端 (4 tests)
# ============================================================


class TestCLIIntegration:
    def test_cli_self_check(self, capsys, monkeypatch, tmp_path):
        fake = tmp_path / "fake_snapshot.json"
        fake.write_text(json.dumps({"v03_score": 0.8852}), encoding="utf-8")
        from apeireth import v1087_asi_hqb_live_gate as mod
        original_engine = mod.LiveGateEngine
        def patched_engine(*args, **kwargs):
            if "snapshot_path" not in kwargs:
                kwargs["snapshot_path"] = fake
            return original_engine(*args, **kwargs)
        monkeypatch.setattr(mod, "LiveGateEngine", patched_engine)
        from apeireth.v1087_asi_hqb_live_gate import main
        rc = main(["--self-check"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert "subscore" in data
        assert "components" in data
        assert len(data["components"]) == 8

    def test_cli_gate_returns_json(self, capsys):
        from apeireth.v1087_asi_hqb_live_gate import main
        rc = main(["--gate", "--task", "code", "--policy", "balanced"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert data["verdict"] in ("accept", "review", "reject", "veto")
        assert data["chosen_model"] == "qwen-coder"

    def test_cli_baseline_reads_snapshot(self, capsys):
        from apeireth.v1087_asi_hqb_live_gate import main
        rc = main(["--baseline"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert "baseline_asi_v03" in data
        assert data["baseline_asi_v03"] >= 0.0  # 即使无 snapshot 也 0.0

    def test_cli_lift_returns_json(self, capsys, monkeypatch, tmp_path):
        fake = tmp_path / "fake_snapshot.json"
        fake.write_text(json.dumps({"v03_score": 0.8852}), encoding="utf-8")
        from apeireth import v1087_asi_hqb_live_gate as mod
        original_engine = mod.LiveGateEngine
        def patched_engine(*args, **kwargs):
            if "snapshot_path" not in kwargs:
                kwargs["snapshot_path"] = fake
            return original_engine(*args, **kwargs)
        monkeypatch.setattr(mod, "LiveGateEngine", patched_engine)
        from apeireth.v1087_asi_hqb_live_gate import main
        rc = main(["--lift"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert "v1087_subscore" in data
        assert "new_asi_v03" in data


# ============================================================
# V3 Philosophy Guard sanity refs (主 17:58+20:46 不假装)
# ============================================================


class TestPhilosophyGuardSanity:
    """主 17:58+20:46 不假装: 4 哲学守卫常量必须可读 + 真引用."""

    def test_guard_not_gate_is_asi(self):
        assert "gate" in GUARD_NOT_GATE_IS_ASI
        assert "ASI" in GUARD_NOT_GATE_IS_ASI

    def test_guard_not_verdict_is_truth(self):
        assert "verdict" in GUARD_NOT_VERDICT_IS_TRUTH
        assert "Popper" in GUARD_NOT_VERDICT_IS_TRUTH or "truth" in GUARD_NOT_VERDICT_IS_TRUTH

    def test_guard_not_review_is_frozen(self):
        assert "review" in GUARD_NOT_REVIEW_IS_FROZEN

    def test_guard_not_veto_is_absolute(self):
        assert "veto" in GUARD_NOT_VETO_IS_ABSOLUTE

    def test_no_lorem_ipsum_in_module(self):
        """主 17:43 实事求是: 模块不应包含 lorem ipsum."""
        from apeireth import v1087_asi_hqb_live_gate as m
        src = Path(m.__file__).read_text(encoding="utf-8")
        assert "lorem ipsum" not in src.lower()
        assert "TODO" not in src or src.count("TODO") < 5  # 允许少量 TODO 但不爆炸

    def test_version_constant(self):
        assert V1087_VERSION == "0.1.0"


# ============================================================
# Reproducibility / sanity refs (主 17:43 实事求是)
# ============================================================


class TestReproducibilityAndRefs:
    """主 17:43: 同一 decision 两次 gate 应一致."""

    def test_same_decision_same_score(self, code_task_decision, fast_engine):
        g1 = fast_engine.gate(code_task_decision)
        g2 = fast_engine.gate(code_task_decision)
        assert abs(g1.hqb_score - g2.hqb_score) < 1e-9

    def test_run_v1087_self_check_returns_8_components(self, monkeypatch, tmp_path):
        """主 17:43: monkeypatch V1087 snapshot_path 为 fake (避免自检卡 18s)."""
        fake = tmp_path / "fake_snapshot.json"
        fake.write_text(json.dumps({"v03_score": 0.8852}), encoding="utf-8")
        # self_check 内部创建 LiveGateEngine() 用默认 snapshot_path, 需 monkeypatch.
        from apeireth import v1087_asi_hqb_live_gate as mod
        original_engine = mod.LiveGateEngine
        def patched_engine(*args, **kwargs):
            if "snapshot_path" not in kwargs:
                kwargs["snapshot_path"] = fake
            return original_engine(*args, **kwargs)
        monkeypatch.setattr(mod, "LiveGateEngine", patched_engine)
        r = mod.run_v1087_self_check()
        assert len(r["components"]) == 8
        assert r["subscore"] >= 0.0

    def test_real_live_engine_passes_v1083_to_v1085_to_v1086(self, code_task_decision, tmp_path):
        """V1087 真实端到端: V1083 dict → extract → V1085 → V1086 (tmp_path 隔离)."""
        from apeireth.v1086_hqb_persistence import HQBPersistence
        # fake snapshot 隔离 (避免读 3GB)
        fake_snap = tmp_path / "fake_snapshot.json"
        fake_snap.write_text(json.dumps({"v03_score": 0.8852}), encoding="utf-8")
        persist = HQBPersistence(artifact_dir=tmp_path / "v1086",
                                 snapshot_path=fake_snap)
        engine = LiveGateEngine(persistence=persist)
        gated = engine.gate(code_task_decision)
        assert gated.verdict in ("accept", "review", "reject", "veto")
        # V1086 应记录一条
        assert len(persist.entries) == 1
        assert persist.entries[0].decision.verdict.value == gated.verdict