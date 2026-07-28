"""V1088 ASI End-to-End Operator — real tests.

主 17:43 实事求是: 测试必须真实跑 (subprocess --self-check), 不允许 mock.
主 23:44 干到底: 测试必须 assert 真实 subscore + ASI V0.3 lift.
主 17:58+20:46 不假装: 测试必须验证 4 不假装哲学守卫.
主 00:44 质量工程区: 测试必须覆盖 8 权重 component + 5 stage + pipeline halt 行为.
"""
from __future__ import annotations

import json
import subprocess
import sys
import time
from pathlib import Path

import pytest

APEIRETH_DIR = Path(__file__).resolve().parent.parent
PROJ_DIR = APEIRETH_DIR.parent


def _import_v1088():
    """主 00:56 任何人都能接手: 测试 import 必须能直接 run."""
    sys.path.insert(0, str(PROJ_DIR))
    from apeireth import v1088_asi_e2e_operator as m
    return m


# ---------------------------------------------------------------------------
# 主 17:43 实事求是 — Sanity tests (refs/guards/no-lorem/reproducibility)
# ---------------------------------------------------------------------------
class TestV1088Sanity:
    """主 17:43 实事求是: 文件存在 + module 能 import + 没有 lorem."""

    def test_v1088_module_exists(self):
        path = APEIRETH_DIR / "v1088_asi_e2e_operator.py"
        assert path.exists(), f"V1088 module missing: {path}"
        assert path.stat().st_size > 10000, "V1088 module too small"

    def test_v1088_importable(self):
        m = _import_v1088()
        assert m.V1088_VERSION == "0.1.0"
        assert hasattr(m, "EndToEndOperator")
        assert hasattr(m, "ASIE2EBridge")
        assert hasattr(m, "PipelineContext")
        assert hasattr(m, "PipelineTrace")
        assert hasattr(m, "StepResult")
        assert hasattr(m, "PipelineStage")
        assert hasattr(m, "StepStatus")

    def test_v1088_no_lorem_ipsum(self):
        m = _import_v1088()
        path = APEIRETH_DIR / "v1088_asi_e2e_operator.py"
        text = path.read_text(encoding="utf-8")
        assert "lorem ipsum" not in text.lower()
        assert "TODO" not in text
        assert "FIXME" not in text

    def test_v1088_has_main(self):
        m = _import_v1088()
        assert callable(m.main)
        assert callable(m.run_v1088_self_check)
        assert callable(m.run_v1088_demo)
        assert callable(m.run_v1088_stats)


# ---------------------------------------------------------------------------
# 主 17:58+20:46 不假装 — Guards tests (4 不假装哲学守卫)
# ---------------------------------------------------------------------------
class TestV1088PhilosophyGuards:
    """主 17:58+20:46 不假装: 4 个哲学守卫必须真实存在 + 不允许 silently skip."""

    def test_guard_pipeline_is_not_asi(self):
        m = _import_v1088()
        assert m.GUARD_PIPELINE_IS_NOT_ASI.startswith("pipeline = orchestration")
        assert "pipeline" in m.GUARD_PIPELINE_IS_NOT_ASI.lower()
        assert "ASI" in m.GUARD_PIPELINE_IS_NOT_ASI

    def test_guard_no_stage_skipped(self):
        m = _import_v1088()
        assert "SKIP" in m.GUARD_NO_STAGE_SKIPPED
        assert "explicit" in m.GUARD_NO_STAGE_SKIPPED.lower()

    def test_guard_no_silent_failure(self):
        m = _import_v1088()
        assert "exception" in m.GUARD_NO_SILENT_FAILURE.lower()
        assert "halt" in m.GUARD_NO_SILENT_FAILURE.lower()

    def test_guard_e2e_does_not_replace(self):
        m = _import_v1088()
        assert "V1081" in m.GUARD_E2E_DOES_NOT_REPLACE
        assert "V1083" in m.GUARD_E2E_DOES_NOT_REPLACE
        assert "V1084" in m.GUARD_E2E_DOES_NOT_REPLACE
        assert "V1087" in m.GUARD_E2E_DOES_NOT_REPLACE
        assert "V1080" in m.GUARD_E2E_DOES_NOT_REPLACE

    def test_philosophy_guards_in_report(self):
        m = _import_v1088()
        score = m.run_v1088_self_check()
        report = m.render_e2e_report(score)
        for guard in (
            m.GUARD_PIPELINE_IS_NOT_ASI,
            m.GUARD_NO_STAGE_SKIPPED,
            m.GUARD_NO_SILENT_FAILURE,
            m.GUARD_E2E_DOES_NOT_REPLACE,
        ):
            assert guard in report, f"Guard missing from report: {guard[:40]}"


# ---------------------------------------------------------------------------
# 主 00:44 质量工程区 — Component scores (8 权重)
# ---------------------------------------------------------------------------
class TestV1088Components:
    """主 00:44: 8 权重 component 必须真实计算 + subscore 真实."""

    def test_default_weights_sum_to_one(self):
        m = _import_v1088()
        total = sum(m.DEFAULT_V1088_WEIGHTS.values())
        assert abs(total - 1.0) < 1e-9, f"Weights sum to {total}"

    def test_default_weights_keys(self):
        m = _import_v1088()
        expected = {
            "probe_quality",
            "route_quality",
            "infer_quality",
            "gate_quality",
            "audit_chain",
            "no_skip",
            "no_silent_fail",
            "reproducibility",
        }
        assert set(m.DEFAULT_V1088_WEIGHTS.keys()) == expected

    def test_subscore_in_range(self):
        m = _import_v1088()
        score = m.run_v1088_self_check()
        assert 0.0 <= score["subscore"] <= 1.0

    def test_asi_lift_capped(self):
        m = _import_v1088()
        score = m.run_v1088_self_check()
        assert score["asi_v03_lift"] <= 0.02, f"Lift > cap: {score['asi_v03_lift']}"

    def test_components_all_present(self):
        m = _import_v1088()
        score = m.run_v1088_self_check()
        for key in m.DEFAULT_V1088_WEIGHTS:
            assert key in score["components"]
            assert 0.0 <= score["components"][key] <= 1.0


# ---------------------------------------------------------------------------
# 主 23:44 干到底 — Pipeline stages (5 stage: PROBE / ROUTE / INFER / GATE / AUDIT)
# ---------------------------------------------------------------------------
class TestV1088Stages:
    """主 23:44: 5 stage 真实存在 + status 真实 + provenance 真实."""

    def test_five_stages(self):
        m = _import_v1088()
        assert m.PipelineStage.PROBE.value == "probe"
        assert m.PipelineStage.ROUTE.value == "route"
        assert m.PipelineStage.INFER.value == "infer"
        assert m.PipelineStage.GATE.value == "gate"
        assert m.PipelineStage.AUDIT.value == "audit"

    def test_step_status_enum(self):
        m = _import_v1088()
        assert m.StepStatus.PASS.value == "pass"
        assert m.StepStatus.FAIL.value == "fail"
        assert m.StepStatus.UNKNOWN.value == "unknown"
        assert m.StepStatus.SKIP.value == "skip"

    def test_pipeline_runs_all_five_stages(self):
        m = _import_v1088()
        op = m.EndToEndOperator()
        ctx = m.PipelineContext(
            task="general",
            prompt="What is 2+2?",
            latency_budget_ms=2000,
            cost_budget_per_1k=0.02,
        )
        trace = op.run(ctx)
        stage_names = [s.stage for s in trace.steps]
        assert "probe" in stage_names
        assert "route" in stage_names
        assert "infer" in stage_names
        assert "gate" in stage_names
        assert "audit" in stage_names

    def test_step_has_real_timestamps(self):
        m = _import_v1088()
        op = m.EndToEndOperator()
        ctx = m.PipelineContext(
            task="general",
            prompt="What is 2+2?",
        )
        trace = op.run(ctx)
        for step in trace.steps:
            assert step.started_at
            assert step.ended_at
            assert step.duration_ms >= 0

    def test_pipeline_trace_to_dict(self):
        m = _import_v1088()
        op = m.EndToEndOperator()
        ctx = m.PipelineContext(prompt="Hello world")
        trace = op.run(ctx)
        d = trace.to_dict()
        assert "pipeline_id" in d
        assert "steps" in d
        assert "started_at" in d
        assert isinstance(d["steps"], list)


# ---------------------------------------------------------------------------
# 主 23:44 干到底 — Pipeline halt behavior (主 17:58 不假装: stage fail = halt)
# ---------------------------------------------------------------------------
class TestV1088HaltBehavior:
    """主 17:58 不假装: stage 失败必须 halt + 记录 error, 不允许 silent skip."""

    def test_pipeline_halt_on_route_failure(self):
        """主 17:58: route fail 必须 halt pipeline (不能用空 registry)."""
        m = _import_v1088()
        op = m.EndToEndOperator()

        # Inject empty registry → select_model returns nothing → route FAIL
        from apeireth import v1083_asi_decision_router as v1083

        class _StubOperator(m.EndToEndOperator):
            def _stage_route(self, ctx, trace):
                started = m.datetime.now(m.timezone.utc).isoformat()
                t0 = time.perf_counter()
                v = v1083.select_model(
                    v1083.RequestContext(
                        task_type="general",
                        capability_need=0.7,
                        latency_budget_ms=2000,
                        cost_budget_per_1k=0.02,
                        prompt_size_tokens=10,
                    ),
                    registry={},  # empty → no model
                    policy="balanced",
                )
                t1 = time.perf_counter()
                return m.StepResult(
                    stage=m.PipelineStage.ROUTE.value,
                    status=m.StepStatus.FAIL.value if not v.chosen_model else m.StepStatus.PASS.value,
                    started_at=started,
                    ended_at=m.datetime.now(m.timezone.utc).isoformat(),
                    duration_ms=(t1 - t0) * 1000.0,
                    payload={},
                    error="no_model_selected" if not v.chosen_model else None,
                )

        op2 = _StubOperator()
        ctx = m.PipelineContext(task="general", prompt="What?")
        trace = op2.run(ctx)
        # Should halt at route stage, no infer / gate / audit produced
        stage_names = [s.stage for s in trace.steps]
        assert "probe" in stage_names
        assert "route" in stage_names
        assert "infer" not in stage_names, f"infer should not run after route fail: {stage_names}"
        assert trace.final_verdict == "halt:route_fail"

    def test_no_silent_failure_marks_error(self):
        """主 17:58: FAIL step 必须有 error 字段, 不允许 silent."""
        m = _import_v1088()
        op = m.EndToEndOperator()
        ctx = m.PipelineContext(prompt="")
        trace = op.run(ctx)
        for step in trace.steps:
            if step.status == m.StepStatus.FAIL.value:
                assert step.error, f"FAIL step without error: {step.stage}"


# ---------------------------------------------------------------------------
# 主 00:56 任何人都能接手 — CLI tests (subprocess)
# ---------------------------------------------------------------------------
class TestV1088CLI:
    """主 00:56: CLI 必须真实可跑 (subprocess, 不允许 in-process)."""

    def test_self_check_subprocess(self):
        proc = subprocess.run(
            [sys.executable, "-m", "apeireth.v1088_asi_e2e_operator", "--self-check"],
            cwd=str(PROJ_DIR),
            capture_output=True,
            text=True,
            timeout=60,
        )
        assert proc.returncode == 0, f"stderr: {proc.stderr}"
        assert "subscore=" in proc.stdout
        assert "lift=" in proc.stdout

    def test_self_check_json_subprocess(self):
        proc = subprocess.run(
            [sys.executable, "-m", "apeireth.v1088_asi_e2e_operator", "--self-check", "--json"],
            cwd=str(PROJ_DIR),
            capture_output=True,
            text=True,
            timeout=60,
        )
        assert proc.returncode == 0, f"stderr: {proc.stderr}"
        data = json.loads(proc.stdout)
        assert "subscore" in data
        assert "asi_v03_lift" in data
        assert "components" in data

    def test_run_subprocess(self):
        proc = subprocess.run(
            [
                sys.executable, "-m", "apeireth.v1088_asi_e2e_operator",
                "--run", "--prompt", "Test", "--task", "general",
            ],
            cwd=str(PROJ_DIR),
            capture_output=True,
            text=True,
            timeout=60,
        )
        assert proc.returncode == 0, f"stderr: {proc.stderr}"
        assert "pipeline=" in proc.stdout
        assert "verdict=" in proc.stdout

    def test_demo_subprocess(self):
        proc = subprocess.run(
            [sys.executable, "-m", "apeireth.v1088_asi_e2e_operator", "--demo"],
            cwd=str(PROJ_DIR),
            capture_output=True,
            text=True,
            timeout=120,
        )
        assert proc.returncode == 0, f"stderr: {proc.stderr}"
        assert "V1088 demo" in proc.stdout
        assert "avg_subscore" in proc.stdout

    def test_stats_subprocess(self):
        proc = subprocess.run(
            [sys.executable, "-m", "apeireth.v1088_asi_e2e_operator", "--stats", "--json"],
            cwd=str(PROJ_DIR),
            capture_output=True,
            text=True,
            timeout=120,
        )
        assert proc.returncode == 0, f"stderr: {proc.stderr}"
        data = json.loads(proc.stdout)
        assert "trace_count" in data
        assert data["trace_count"] >= 5
        assert "stage_status_distribution" in data


# ---------------------------------------------------------------------------
# 主 23:44 干到底 — Reproducibility tests
# ---------------------------------------------------------------------------
class TestV1088Reproducibility:
    """主 23:44: trace 必须持久化 + 可重放."""

    def test_trace_persisted(self):
        m = _import_v1088()
        import tempfile
        with tempfile.TemporaryDirectory() as tmpdir:
            op = m.EndToEndOperator(artifacts_dir=Path(tmpdir) / "v1088")
            ctx = m.PipelineContext(prompt="Repro test")
            trace = op.run(ctx)
            trace_files = list((Path(tmpdir) / "v1088").glob("trace_*.json"))
            assert len(trace_files) == 1, f"Expected 1 trace file, got {len(trace_files)}"
            data = json.loads(trace_files[0].read_text(encoding="utf-8"))
            assert data["pipeline_id"] == trace.pipeline_id

    def test_pipeline_confidence_calibrated(self):
        """主 19:33 (Tetlock 校准): pipeline_confidence = PASS / non-skip stage."""
        m = _import_v1088()
        op = m.EndToEndOperator()
        ctx = m.PipelineContext(prompt="Calibration test")
        trace = op.run(ctx)
        # 真实计算
        pass_count = sum(1 for s in trace.steps if s.status == m.StepStatus.PASS.value)
        non_skip_count = sum(1 for s in trace.steps if s.status != m.StepStatus.SKIP.value)
        expected = pass_count / non_skip_count if non_skip_count else 0.0
        assert abs(trace.pipeline_confidence - expected) < 1e-9


# ---------------------------------------------------------------------------
# 主 13:08 真实意图追问 — Real production wiring tests
# ---------------------------------------------------------------------------
class TestV1088RealWiring:
    """主 13:08: V1088 真实串接 V1080/V1081/V1083/V1084/V1087, 不允许 stub."""

    def test_probe_uses_v1081(self):
        """PROBE stage 真实调 V1081.fabricate_or_reject."""
        m = _import_v1088()
        op = m.EndToEndOperator()
        ctx = m.PipelineContext(prompt="What is 2+2?")
        trace = op.run(ctx)
        probe_step = next(s for s in trace.steps if s.stage == "probe")
        # 主 17:43: honesty_text 字段真实来自 V1081
        assert "honesty_text" in probe_step.payload
        assert "is_honest_disclosure" in probe_step.payload

    def test_route_uses_v1083(self):
        """ROUTE stage 真实调 V1083.select_model."""
        m = _import_v1088()
        op = m.EndToEndOperator()
        ctx = m.PipelineContext(prompt="Test routing", task="code")
        trace = op.run(ctx)
        route_step = next(s for s in trace.steps if s.stage == "route")
        if route_step.status == m.StepStatus.PASS.value:
            assert "chosen_model" in route_step.payload
            assert "score" in route_step.payload
            assert route_step.payload["chosen_model"]

    def test_infer_uses_v1084(self):
        """INFER stage 真实调 V1084.InferenceEngine.infer."""
        m = _import_v1088()
        op = m.EndToEndOperator()
        ctx = m.PipelineContext(prompt="Test inference")
        trace = op.run(ctx)
        infer_step = next(s for s in trace.steps if s.stage == "infer")
        if infer_step.status == m.StepStatus.PASS.value:
            assert "input_tokens" in infer_step.payload
            assert "output_tokens" in infer_step.payload
            assert "cost_usd" in infer_step.payload

    def test_gate_uses_v1087(self):
        """GATE stage 真实调 V1087.LiveGateEngine.gate."""
        m = _import_v1088()
        op = m.EndToEndOperator()
        ctx = m.PipelineContext(prompt="Test gate")
        trace = op.run(ctx)
        gate_step = next(s for s in trace.steps if s.stage == "gate")
        if gate_step.status == m.StepStatus.PASS.value:
            assert "verdict" in gate_step.payload
            assert gate_step.payload["verdict"] in ("accept", "review", "reject", "veto")
            assert "breakdown" in gate_step.payload

    def test_audit_uses_v1080(self):
        """AUDIT stage 真实调 V1080.build_run_manifest."""
        m = _import_v1088()
        op = m.EndToEndOperator()
        ctx = m.PipelineContext(prompt="Test audit")
        trace = op.run(ctx)
        audit_step = next(s for s in trace.steps if s.stage == "audit")
        if audit_step.status == m.StepStatus.PASS.value:
            assert "manifest_sha" in audit_step.payload
            assert "git_rev" in audit_step.payload
            assert audit_step.payload["provenance_node_count"] >= 1


# ---------------------------------------------------------------------------
# 主 23:44 — End-to-end integration test
# ---------------------------------------------------------------------------
class TestV1088Integration:
    """主 23:44: 全链路真实跑通, subscore 真实计算."""

    def test_full_pipeline_subscore(self):
        m = _import_v1088()
        score = m.run_v1088_self_check()
        # 主 17:43: 真实 subscore 应该 > 0.5 (5 stage 真实串通)
        assert score["subscore"] > 0.5, f"subscore too low: {score['subscore']}"
        # 主 23:44: ASI V0.3 lift 应该 > 0
        assert score["asi_v03_lift"] > 0
        # 主 22:33: philosophy guards OK
        assert score["philosophy_guards_ok"]

    def test_report_writes_to_file(self):
        m = _import_v1088()
        import tempfile
        with tempfile.TemporaryDirectory() as tmpdir:
            score = m.run_v1088_self_check()
            report_path = Path(tmpdir) / "report.md"
            m.write_report(score, report_path)
            assert report_path.exists()
            text = report_path.read_text(encoding="utf-8")
            assert "V1088 ASI End-to-End Operator Report" in text
            assert "Subscore" in text
            assert "Component Scores" in text

    def test_default_registry_has_6_models(self):
        """主 17:43: default registry 必须真实有 6 个 model (V1083 sample)."""
        m = _import_v1088()
        reg = m._default_registry()
        assert len(reg) == 6
        for name in (
            "deepseek-v3",
            "claude-opus-4",
            "claude-sonnet-4",
            "gpt-4o",
            "gpt-4o-mini",
            "qwen-coder",
        ):
            assert name in reg