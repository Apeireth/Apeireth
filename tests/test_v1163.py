"""test_v1163 — V1163 ASI real_production V0.6 真测模块 tests.

主 17:43 实事求是 + 主 23:44 干到底 + 主 00:44 质量工程化.
真测 5 sub-dim: compose_orchestration_real / k8s_dockerfile_real /
              subprocess_runtime_real / health_probe_real / canonical_bundle_real.

Usage:
    pytest tests/test_v1163.py -v
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from unittest.mock import MagicMock

import pytest

# 确保 apeireth 模块可导入
REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO_ROOT))

from apeireth.v1163_asi_real_production_v06_real_measure import (  # noqa: E402
    DEFAULT_ARTIFACT_DIR,
    TARGET_REAL_PRODUCTION_V06,
    V1132_REPORT_FIELDS,
    V1144_BASELINE_REAL_PRODUCTION,
    V1163_DIM_VERSION,
    V1163_SUBDIM_NAMES,
    V1163_VERSION,
    RealProductionReport,
    SubDimEvidence,
    _attr_first,
    _call_safely,
    _get_v1132_report,
    _measure_canonical_bundle,
    _measure_compose_orchestration,
    _measure_health_probe,
    _measure_k8s_dockerfile,
    _measure_subprocess_runtime,
    _safe_field,
    _safe_import,
    measure_real_production_full,
    measure_real_production_v06,
    render_report_md,
)


# ---------------------------------------------------------------------------
# Constants & locks
# ---------------------------------------------------------------------------


class TestV1163Constants:
    def test_version_present(self):
        assert isinstance(V1163_VERSION, str) and len(V1163_VERSION) > 0

    def test_dim_version(self):
        assert V1163_DIM_VERSION == "0.6"

    def test_subdim_names_locked(self):
        assert V1163_SUBDIM_NAMES == (
            "compose_orchestration_real",
            "k8s_dockerfile_real",
            "subprocess_runtime_real",
            "health_probe_real",
            "canonical_bundle_real",
        )

    def test_baseline_hardcoded(self):
        # 主 17:43 — baseline 写死, 不能根据当次运行改
        assert V1144_BASELINE_REAL_PRODUCTION == 0.9600

    def test_target_set(self):
        assert TARGET_REAL_PRODUCTION_V06 == 0.8500

    def test_v1132_fields_documented(self):
        # 9 真 field names should be documented
        assert "compose_files_parsed" in V1132_REPORT_FIELDS
        assert "services_seen" in V1132_REPORT_FIELDS
        assert "k8s_manifests_ok" in V1132_REPORT_FIELDS
        assert "dockerfile_valid" in V1132_REPORT_FIELDS
        assert "subprocess_runs_ok" in V1132_REPORT_FIELDS
        assert "subprocess_runs_failed" in V1132_REPORT_FIELDS
        assert "health_probes_ok" in V1132_REPORT_FIELDS
        assert "health_probes_failed" in V1132_REPORT_FIELDS
        assert "canonical_bundle_valid" in V1132_REPORT_FIELDS

    def test_artifact_dir_default(self):
        assert DEFAULT_ARTIFACT_DIR == "artifacts"


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


class TestSafeHelpers:
    def test_safe_import_returns_module(self):
        mod = _safe_import("apeireth.v1163_asi_real_production_v06_real_measure")
        assert mod is not None

    def test_safe_import_returns_none_on_missing(self):
        mod = _safe_import("apeireth.does_not_exist_xyz_123")
        assert mod is None

    def test_attr_first_picks_first_existing(self):
        class _Obj:
            alpha = "a"
            beta = "b"
        obj = _Obj()
        # First existing wins
        assert _attr_first(obj, ["alpha", "beta"]) == "a"

        class _ObjNoAlpha:
            beta = "b"
        obj_no_alpha = _ObjNoAlpha()
        # Falls back to second
        assert _attr_first(obj_no_alpha, ["alpha", "beta"]) == "b"

        class _ObjEmpty:
            pass
        obj_empty = _ObjEmpty()
        # Returns None if all missing
        assert _attr_first(obj_empty, ["x", "y"]) is None

    def test_call_safely_with_callable(self):
        fn = lambda: 42
        ok, val = _call_safely(fn, default=None)
        assert ok is True and val == 42

    def test_call_safely_with_none(self):
        ok, val = _call_safely(None, default="fb")
        assert ok is False and val == "fb"

    def test_call_safely_with_raising(self):
        def boom():
            raise RuntimeError("x")
        ok, val = _call_safely(boom, default="fb")
        assert ok is False and val == "fb"

    def test_safe_field_returns_default_on_missing(self):
        obj = MagicMock(spec=["alpha"])
        obj.alpha = 5
        assert _safe_field(obj, "alpha") == 5
        assert _safe_field(obj, "missing", default=99) == 99


class TestGetV1132Report:
    def test_real_call_succeeds_or_returns_reason(self):
        ok, rep, reason = _get_v1132_report()
        # In real env this should succeed since V1132 exists
        if not ok:
            assert reason != "ok"
        else:
            assert rep is not None
            assert reason == "ok"


# ---------------------------------------------------------------------------
# Mock report factory
# ---------------------------------------------------------------------------


def _mock_report(**overrides):
    """Build a MagicMock that mimics V1132DeploymentReport."""
    defaults = {
        "report_id": "rpt-test",
        "docker_daemon_available": False,
        "compose_files_parsed": 2,
        "services_seen": 14,
        "k8s_manifests_ok": 3,
        "dockerfile_valid": 2,
        "subprocess_runs_ok": 2,
        "subprocess_runs_failed": 0,
        "health_probes_ok": 0,
        "health_probes_failed": 1,
        "canonical_bundle_valid": True,
        "offline_valid": True,
        "runtime_valid": False,
        "passed": False,
    }
    defaults.update(overrides)
    rep = MagicMock()
    for k, v in defaults.items():
        setattr(rep, k, v)
    return rep


# ---------------------------------------------------------------------------
# R1 — compose_orchestration_real
# ---------------------------------------------------------------------------


class TestMeasureComposeOrchestration:
    def test_with_realistic_values(self):
        rep = _mock_report(compose_files_parsed=2, services_seen=14)
        score, ev = _measure_compose_orchestration(rep)
        assert 0.0 <= score <= 1.0
        assert ev.name == "compose_orchestration_real"
        assert ev.raw["compose_files_parsed"] == 2
        assert ev.raw["services_seen"] == 14

    def test_no_compose_files(self):
        rep = _mock_report(compose_files_parsed=0, services_seen=0)
        score, ev = _measure_compose_orchestration(rep)
        assert score < 0.5  # compose_files_parsed=0 → low score
        assert ev.checks["compose_files_at_least_2"] is False

    def test_high_volume(self):
        rep = _mock_report(compose_files_parsed=10, services_seen=50)
        score, ev = _measure_compose_orchestration(rep)
        assert score > 0.5

    def test_score_bounded(self):
        rep = _mock_report(compose_files_parsed=999, services_seen=9999)
        score, ev = _measure_compose_orchestration(rep)
        assert 0.0 <= score <= 1.0
        # saturation check
        assert ev.raw["compose_norm"] == 1.0
        assert ev.raw["services_norm"] == 1.0


# ---------------------------------------------------------------------------
# R2 — k8s_dockerfile_real
# ---------------------------------------------------------------------------


class TestMeasureK8sDockerfile:
    def test_baseline_realistic(self):
        rep = _mock_report(k8s_manifests_ok=3, dockerfile_valid=2)
        score, ev = _measure_k8s_dockerfile(rep)
        assert 0.0 <= score <= 1.0
        assert ev.name == "k8s_dockerfile_real"

    def test_no_k8s_no_dockerfile(self):
        rep = _mock_report(k8s_manifests_ok=0, dockerfile_valid=0)
        score, ev = _measure_k8s_dockerfile(rep)
        assert score == 0.0
        assert ev.checks["k8s_at_least_3"] is False
        assert ev.checks["dockerfile_at_least_2"] is False

    def test_one_only(self):
        rep = _mock_report(k8s_manifests_ok=5, dockerfile_valid=0)
        score, ev = _measure_k8s_dockerfile(rep)
        # k8s OK but no dockerfile → partial
        assert 0.0 < score < 1.0


# ---------------------------------------------------------------------------
# R3 — subprocess_runtime_real
# ---------------------------------------------------------------------------


class TestMeasureSubprocessRuntime:
    def test_perfect_runtime(self):
        rep = _mock_report(subprocess_runs_ok=5, subprocess_runs_failed=0)
        score, ev = _measure_subprocess_runtime(rep)
        assert score > 0.5
        assert ev.checks["no_failed_runs"] is True

    def test_with_failures(self):
        rep = _mock_report(subprocess_runs_ok=2, subprocess_runs_failed=3)
        score, ev = _measure_subprocess_runtime(rep)
        # ratio = 2/5 = 0.4, scaled → below perfect
        assert score < 0.6
        assert ev.checks["no_failed_runs"] is False

    def test_no_runs_at_all(self):
        rep = _mock_report(subprocess_runs_ok=0, subprocess_runs_failed=0)
        score, ev = _measure_subprocess_runtime(rep)
        # 主 17:43 — 没跑 ≠ 0 但接近 0; base=0, 只有极小 check bonus
        assert score < 0.15
        assert any("no subprocess" in n for n in ev.notes)

    def test_all_failed(self):
        rep = _mock_report(subprocess_runs_ok=0, subprocess_runs_failed=5)
        score, ev = _measure_subprocess_runtime(rep)
        assert score == 0.0


# ---------------------------------------------------------------------------
# R4 — health_probe_real
# ---------------------------------------------------------------------------


class TestMeasureHealthProbe:
    def test_no_probes_attempted(self):
        rep = _mock_report(health_probes_ok=0, health_probes_failed=0)
        score, ev = _measure_health_probe(rep)
        # 0.3 partial (untested)
        assert 0.2 <= score <= 0.5
        assert any("untested" in n or "no probe" in n for n in ev.notes)

    def test_all_probes_ok(self):
        rep = _mock_report(health_probes_ok=3, health_probes_failed=0)
        score, ev = _measure_health_probe(rep)
        assert score >= 0.7
        assert ev.checks["probe_no_failed"] is True

    def test_failed_probes(self):
        rep = _mock_report(health_probes_ok=1, health_probes_failed=5)
        score, ev = _measure_health_probe(rep)
        assert score < 0.7

    def test_perfect_score(self):
        rep = _mock_report(health_probes_ok=10, health_probes_failed=0)
        score, ev = _measure_health_probe(rep)
        assert score > 0.8


# ---------------------------------------------------------------------------
# R5 — canonical_bundle_real
# ---------------------------------------------------------------------------


class TestMeasureCanonicalBundle:
    def test_all_three(self):
        rep = _mock_report(canonical_bundle_valid=True, offline_valid=True, runtime_valid=True)
        score, ev = _measure_canonical_bundle(rep)
        assert score >= 0.9

    def test_canonical_and_offline_no_runtime(self):
        rep = _mock_report(canonical_bundle_valid=True, offline_valid=True, runtime_valid=False)
        score, ev = _measure_canonical_bundle(rep)
        assert 0.7 < score < 0.9

    def test_only_canonical(self):
        rep = _mock_report(canonical_bundle_valid=True, offline_valid=False, runtime_valid=False)
        score, ev = _measure_canonical_bundle(rep)
        # 0.4 partial
        assert score < 0.6

    def test_none(self):
        rep = _mock_report(canonical_bundle_valid=False, offline_valid=False, runtime_valid=False)
        score, ev = _measure_canonical_bundle(rep)
        assert score == 0.0


# ---------------------------------------------------------------------------
# Main entry + report
# ---------------------------------------------------------------------------


class TestMainEntry:
    def test_measure_v06_returns_float(self):
        score = measure_real_production_v06()
        assert isinstance(score, float)
        assert 0.0 <= score <= 1.0

    def test_full_returns_dataclass(self):
        rep = measure_real_production_full(write_artifact=False)
        assert isinstance(rep, RealProductionReport)
        assert 0.0 <= rep.total <= 1.0

    def test_full_populates_sub_dim_scores(self):
        rep = measure_real_production_full(write_artifact=False)
        assert set(rep.sub_dim_scores.keys()) == set(V1163_SUBDIM_NAMES)
        for name in V1163_SUBDIM_NAMES:
            assert name in rep.sub_dim_scores
            assert 0.0 <= rep.sub_dim_scores[name] <= 1.0

    def test_full_populates_evidence(self):
        rep = measure_real_production_full(write_artifact=False)
        assert set(rep.sub_dim_evidence.keys()) == set(V1163_SUBDIM_NAMES)
        for name in V1163_SUBDIM_NAMES:
            ev = rep.sub_dim_evidence[name]
            assert ev.name == name
            assert isinstance(ev, SubDimEvidence)
            assert 0.0 <= ev.score <= 1.0

    def test_full_writes_artifact(self, tmp_path):
        rep = measure_real_production_full(write_artifact=True, artifact_dir=str(tmp_path))
        # Either succeeds writing or has a note about failure
        if rep.artifact_path:
            p = Path(rep.artifact_path)
            assert p.exists()
            data = json.loads(p.read_text(encoding="utf-8"))
            assert data["snapshot_id"] == rep.snapshot_id
            assert data["version"] == V1163_VERSION
            assert data["dim_version"] == V1163_DIM_VERSION
            assert set(data["sub_dim_scores"].keys()) == set(V1163_SUBDIM_NAMES)
        else:
            # If write failed, a note should explain
            assert any("artifact write failed" in n for n in rep.notes)

    def test_no_write_skips_artifact(self, tmp_path):
        rep = measure_real_production_full(write_artifact=False)
        assert rep.artifact_path == ""

    def test_total_is_mean(self):
        rep = measure_real_production_full(write_artifact=False)
        if rep.sub_dim_scores:
            mean = sum(rep.sub_dim_scores.values()) / len(rep.sub_dim_scores)
            assert abs(rep.total - mean) < 1e-9

    def test_subdim_status_counting(self):
        rep = measure_real_production_full(write_artifact=False)
        total_categorized = rep.n_subdims_passed + rep.n_subdims_partial + rep.n_subdims_missing
        assert total_categorized == len(V1163_SUBDIM_NAMES)

    def test_summary_line_format(self):
        rep = measure_real_production_full(write_artifact=False)
        line = rep.summary_line()
        assert "V1163" in line
        assert "real_production V0.6" in line
        assert "snapshot=" in line


# ---------------------------------------------------------------------------
# Render report MD
# ---------------------------------------------------------------------------


class TestRenderReport:
    def test_md_contains_required_sections(self):
        rep = measure_real_production_full(write_artifact=False)
        md = render_report_md(rep)
        assert "# V1163 real_production V0.6 真补报告" in md
        assert "## Total" in md
        assert "## 5 sub-dim 真测" in md
        assert "## Sub-dim Evidence" in md
        assert "## Notes" in md
        assert V1163_VERSION in md

    def test_md_lists_all_subdims(self):
        rep = measure_real_production_full(write_artifact=False)
        md = render_report_md(rep)
        for name in V1163_SUBDIM_NAMES:
            assert name in md


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


class TestCLI:
    def test_default_run(self, capsys, monkeypatch):
        monkeypatch.setattr(sys, "argv", ["v1163"])
        from apeireth.v1163_asi_real_production_v06_real_measure import _cli
        rc = _cli()
        assert rc == 0
        captured = capsys.readouterr()
        assert "V1163 real_production V0.6" in captured.out

    def test_json_run(self, capsys, monkeypatch):
        monkeypatch.setattr(sys, "argv", ["v1163", "--json", "--no-write"])
        from apeireth.v1163_asi_real_production_v06_real_measure import _cli
        rc = _cli()
        assert rc == 0
        captured = capsys.readouterr()
        # Should be valid JSON
        data = json.loads(captured.out)
        assert "snapshot_id" in data
        assert "sub_dim_scores" in data

    def test_report_run_writes_md(self, monkeypatch, tmp_path):
        md_out = tmp_path / "report.md"
        monkeypatch.setattr(sys, "argv", ["v1163", "--report", "--no-write", "--md-out", str(md_out)])
        from apeireth.v1163_asi_real_production_v06_real_measure import _cli
        rc = _cli()
        assert rc == 0
        assert md_out.exists()
        content = md_out.read_text(encoding="utf-8")
        assert "V1163" in content


# ---------------------------------------------------------------------------
# Round-trip: to_dict / from_dict
# ---------------------------------------------------------------------------


class TestRoundTrip:
    def test_to_from_dict_preserves_scores(self):
        rep = measure_real_production_full(write_artifact=False)
        d = rep.to_dict()
        rep2 = RealProductionReport.from_dict(d)
        assert rep2.snapshot_id == rep.snapshot_id
        assert rep2.total == rep.total
        assert rep2.sub_dim_scores == rep.sub_dim_scores
        assert rep2.dim_version == rep.dim_version
        assert rep2.target == rep.target
        assert rep2.v1132_report_id == rep.v1132_report_id

    def test_from_dict_with_minimal_data(self):
        d = {"snapshot_id": "x", "total": 0.5}
        rep = RealProductionReport.from_dict(d)
        assert rep.snapshot_id == "x"
        assert rep.total == 0.5


# ---------------------------------------------------------------------------
# 主 17:43 实事求是 — 不刷 KPI: 失败 sub-dim 必须可见
# ---------------------------------------------------------------------------


class TestNoFaking:
    """主 17:43 — 不假装, 不刷 KPI. 测试 0 分路径."""

    def test_no_pretending_when_v1132_missing(self, monkeypatch):
        # Mock _get_v1132_report to fail
        import apeireth.v1163_asi_real_production_v06_real_measure as vmod
        original = vmod._get_v1132_report
        monkeypatch.setattr(vmod, "_get_v1132_report",
                           lambda: (False, None, "simulated_failure"))
        try:
            score = vmod.measure_real_production_v06()
            assert score == 0.0
        finally:
            monkeypatch.setattr(vmod, "_get_v1132_report", original)

    def test_zero_when_no_data(self):
        rep = _mock_report(compose_files_parsed=0, services_seen=0,
                          k8s_manifests_ok=0, dockerfile_valid=0,
                          subprocess_runs_ok=0, subprocess_runs_failed=0,
                          health_probes_ok=0, health_probes_failed=0,
                          canonical_bundle_valid=False, offline_valid=False,
                          runtime_valid=False)
        # Use full measure path with this mock
        import apeireth.v1163_asi_real_production_v06_real_measure as vmod
        # Manually compose
        s1, _ = vmod._measure_compose_orchestration(rep)
        s2, _ = vmod._measure_k8s_dockerfile(rep)
        s3, _ = vmod._measure_subprocess_runtime(rep)
        s4, _ = vmod._measure_health_probe(rep)
        s5, _ = vmod._measure_canonical_bundle(rep)
        # R4 has 0.3 partial even when no probes; R3 = 0
        # R1 + R2 + R5 = 0; R3 = 0; R4 = 0.3
        # Mean = 0.3 / 5 = 0.06
        mean = (s1 + s2 + s3 + s4 + s5) / 5.0
        assert mean < 0.2  # mostly missing → low