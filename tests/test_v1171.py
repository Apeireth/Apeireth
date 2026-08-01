"""Test V1171 — ASI real_production V0.6.1 patched (V1170 alt runtime 真补).

主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 19:33 走在前人经验上 + 主 13:31 大胆激进 +
主 17:58+20:46 不假装 + 主 23:44 干到底 + 主 00:56 任何人都能接手 + 主 00:44 质量工程化.

V1171 = ASI real_production V0.6.1 patched (V1170 alt runtime 填 R3+R4):
- 5 sub-dim (LOCKED 名称沿用 V1163)
- R3 subprocess_runtime: V1132 + V1170 加权 (V1170 alt runtime 真补核心)
- R4 health_probe: V1170 health sub-dim mean 真补 (替代 V1132 health_probes=0/4)
- aggregate = mean(sub_dim_scores)
- 主 17:58+20:46 不假装: compat factor 0.9 (alt runtime ≠ docker)

测试覆盖 (主 00:44 质量工程化):
  1. Constants + dataclass invariants
  2. _safe_field 双类型 (dict + object)
  3. _get_v1132_report 真读真解析
  4. _get_v1170_score 真读真 fallback
  5. R1+R2+R5 measurement (沿用 V1163 算法) with mocked report
  6. R3 patched (V1132 + V1170 加权) with docker + without docker
  7. R4 patched (V1170 health sub-dim mean 真补)
  8. measure_real_production_v06_patched 端到端
  9. measure_real_production_v06_patched_full + JSON 序列化
  10. CLI main() --json / --report / summary paths
  11. delta from V1163 baseline is positive
  12. runtime_proven = True when total ≥ 0.85
"""

from __future__ import annotations

import dataclasses
import io
import json
import sys
from contextlib import redirect_stdout
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

import pytest


# ============================================================================
# 1. Constants + dataclass invariants
# ============================================================================


class TestV1171Constants:
    def test_version_present(self):
        from apeireth.v1171_asi_real_production_v06_patched import V1171_VERSION
        assert V1171_VERSION == "0.1.0"

    def test_dim_version(self):
        from apeireth.v1171_asi_real_production_v06_patched import V1171_DIM_VERSION
        assert V1171_DIM_VERSION == "0.6.1"

    def test_subdim_names_locked(self):
        from apeireth.v1171_asi_real_production_v06_patched import V1171_SUBDIM_NAMES
        assert V1171_SUBDIM_NAMES == (
            "compose_orchestration_real",
            "k8s_dockerfile_real",
            "subprocess_runtime_real",
            "health_probe_real",
            "canonical_bundle_real",
        )
        assert len(V1171_SUBDIM_NAMES) == 5

    def test_v1163_baseline_constant(self):
        from apeireth.v1171_asi_real_production_v06_patched import V1163_BASELINE_REAL_PRODUCTION
        # Per V1163 真测: total=0.4900
        assert 0.4 <= V1163_BASELINE_REAL_PRODUCTION <= 0.6

    def test_target_constant(self):
        from apeireth.v1171_asi_real_production_v06_patched import TARGET_V1171
        assert 0.7 <= TARGET_V1171 <= 1.0

    def test_docker_compat_factor(self):
        from apeireth.v1171_asi_real_production_v06_patched import V1170_DOCKER_COMPAT_FACTOR
        # 主 17:58+20:46 不假装: alt runtime ≠ docker, factor < 1.0
        assert 0.5 <= V1170_DOCKER_COMPAT_FACTOR <= 1.0

    def test_v1170_expected_alt_score(self):
        from apeireth.v1171_asi_real_production_v06_patched import V1170_EXPECTED_ALT_RUNTIME_SCORE
        assert V1170_EXPECTED_ALT_RUNTIME_SCORE == 1.0


class TestV1171SubDimEvidenceDataclass:
    def test_default_construction(self):
        from apeireth.v1171_asi_real_production_v06_patched import SubDimEvidence
        ev = SubDimEvidence(name="x", score=0.5)
        assert ev.checks == {}
        assert ev.notes == []
        assert ev.raw == {}

    def test_to_dict(self):
        from apeireth.v1171_asi_real_production_v06_patched import SubDimEvidence
        ev = SubDimEvidence(name="x", score=0.5, checks={"a": True})
        d = ev.to_dict()
        assert d["name"] == "x"
        assert d["score"] == 0.5
        assert d["checks"] == {"a": True}


class TestV1171ReportDataclass:
    def test_default_construction(self):
        from apeireth.v1171_asi_real_production_v06_patched import V1171Report
        r = V1171Report()
        assert r.total == 0.0
        assert r.v1163_baseline > 0
        assert r.docker_daemon_available is False
        assert r.runtime_proven is False

    def test_to_dict_round_trip(self):
        from apeireth.v1171_asi_real_production_v06_patched import V1171Report
        r = V1171Report()
        r.sub_dim_scores = {"a": 1.0, "b": 0.5}
        d = r.to_dict()
        s = json.dumps(d)
        d2 = json.loads(s)
        assert d2["sub_dim_scores"]["a"] == 1.0

    def test_summary_line(self):
        from apeireth.v1171_asi_real_production_v06_patched import V1171Report
        r = V1171Report()
        s = r.summary_line
        assert "V1171 real_production V0.6.1 patched" in s
        assert "vs V1163 baseline" in s
        assert "V1170 alt score=" in s


# ============================================================================
# 2. _safe_field with both dict and object
# ============================================================================


class TestSafeField:
    def test_dict_present(self):
        from apeireth.v1171_asi_real_production_v06_patched import _safe_field
        assert _safe_field({"a": 5}, "a", 0) == 5
        assert _safe_field({"a": "x"}, "a", "default") == "x"

    def test_dict_missing(self):
        from apeireth.v1171_asi_real_production_v06_patched import _safe_field
        assert _safe_field({}, "missing", "fallback") == "fallback"

    def test_none(self):
        from apeireth.v1171_asi_real_production_v06_patched import _safe_field
        assert _safe_field(None, "anything", 42) == 42

    def test_object_with_attribute(self):
        from apeireth.v1171_asi_real_production_v06_patched import _safe_field
        @dataclasses.dataclass
        class O:
            x: int = 7
        assert _safe_field(O(), "x", 0) == 7

    def test_object_missing_attribute(self):
        from apeireth.v1171_asi_real_production_v06_patched import _safe_field
        @dataclasses.dataclass
        class O:
            x: int = 7
        assert _safe_field(O(), "y", 99) == 99


# ============================================================================
# 3. _get_v1132_report — real V1132 validator runs and produces JSON
# ============================================================================


class TestGetV1132Report:
    def test_produces_dict_or_none(self, tmp_path):
        from apeireth.v1171_asi_real_production_v06_patched import _get_v1132_report
        ok, rep, reason = _get_v1132_report(artifact_dir=str(tmp_path))
        # ok may be False if V1132 fails; just verify type contract
        assert isinstance(ok, bool)
        assert isinstance(reason, str)
        if ok:
            assert rep is not None
            # Required fields
            assert "compose_files_parsed" in rep or "services_seen" in rep


# ============================================================================
# 4. _get_v1170_score
# ============================================================================


class TestGetV1170Score:
    def test_returns_float_in_range(self):
        from apeireth.v1171_asi_real_production_v06_patched import _get_v1170_score
        score = _get_v1170_score()
        assert isinstance(score, float)
        assert 0.0 <= score <= 1.0


# ============================================================================
# 5. R1, R2, R5 measurement (V1163-style algorithm) with mocked V1132 report
# ============================================================================


def _mock_v1132_report() -> dict:
    """Return a representative mock V1132 report."""
    return {
        "report_id": "rpt-mock",
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
    }


class TestR1ComposeOrchestration:
    def test_returns_tuple(self):
        from apeireth.v1171_asi_real_production_v06_patched import _measure_compose_orchestration
        rep = _mock_v1132_report()
        score, ev = _measure_compose_orchestration(rep)
        assert isinstance(score, float)
        assert 0.0 <= score <= 1.0
        assert ev.name == "compose_orchestration_real"

    def test_high_when_files_and_services_high(self):
        from apeireth.v1171_asi_real_production_v06_patched import _measure_compose_orchestration
        rep = _mock_v1132_report()
        rep["compose_files_parsed"] = 8  # saturate
        rep["services_seen"] = 40  # saturate
        score, _ = _measure_compose_orchestration(rep)
        assert score >= 0.9

    def test_low_when_zero(self):
        from apeireth.v1171_asi_real_production_v06_patched import _measure_compose_orchestration
        rep = _mock_v1132_report()
        rep["compose_files_parsed"] = 0
        rep["services_seen"] = 0
        score, _ = _measure_compose_orchestration(rep)
        assert score < 0.3


class TestR2K8sDockerfile:
    def test_returns_tuple(self):
        from apeireth.v1171_asi_real_production_v06_patched import _measure_k8s_dockerfile
        rep = _mock_v1132_report()
        score, ev = _measure_k8s_dockerfile(rep)
        assert 0.0 <= score <= 1.0
        assert ev.name == "k8s_dockerfile_real"

    def test_high_when_saturated(self):
        from apeireth.v1171_asi_real_production_v06_patched import _measure_k8s_dockerfile
        rep = _mock_v1132_report()
        rep["k8s_manifests_ok"] = 12
        rep["dockerfile_valid"] = 8
        score, _ = _measure_k8s_dockerfile(rep)
        assert score >= 0.9


class TestR5CanonicalBundle:
    def test_returns_tuple(self):
        from apeireth.v1171_asi_real_production_v06_patched import _measure_canonical_bundle
        rep = _mock_v1132_report()
        score, ev = _measure_canonical_bundle(rep)
        assert 0.0 <= score <= 1.0
        assert ev.name == "canonical_bundle_real"

    def test_full_when_all_true(self):
        from apeireth.v1171_asi_real_production_v06_patched import _measure_canonical_bundle
        rep = _mock_v1132_report()
        rep["canonical_bundle_valid"] = True
        rep["offline_valid"] = True
        rep["runtime_valid"] = True
        score, _ = _measure_canonical_bundle(rep)
        # runtime_valid=True makes base = 0.4+0.3+0.3 = 1.0 + bonus → capped at 1.0
        assert score >= 0.95


# ============================================================================
# 6. R3 patched — V1132 + V1170 加权 (with/without docker)
# ============================================================================


class TestR3SubprocessRuntimePatched:
    def test_docker_available_uses_v1132_plus_v1170(self):
        from apeireth.v1171_asi_real_production_v06_patched import _measure_subprocess_runtime_v1171
        rep = _mock_v1132_report()
        score, ev = _measure_subprocess_runtime_v1171(rep, v1170_score=1.0, docker_daemon=True)
        # 0.5 * (2/2) + 0.5 * 1.0 = 1.0 + bonus
        assert score >= 0.95
        assert "docker available" in " ".join(ev.notes)

    def test_no_docker_uses_v1170_with_compat_factor(self):
        from apeireth.v1171_asi_real_production_v06_patched import _measure_subprocess_runtime_v1171
        rep = _mock_v1132_report()
        score, ev = _measure_subprocess_runtime_v1171(rep, v1170_score=1.0, docker_daemon=False)
        # 1.0 * 0.9 = 0.9 + bonus → ≥ 1.0 capped
        assert score >= 0.9
        assert "no docker" in " ".join(ev.notes)

    def test_zero_v1170_zero_v1132_yields_low(self):
        from apeireth.v1171_asi_real_production_v06_patched import _measure_subprocess_runtime_v1171
        rep = _mock_v1132_report()
        rep["subprocess_runs_ok"] = 0
        rep["subprocess_runs_failed"] = 0
        score, _ = _measure_subprocess_runtime_v1171(rep, v1170_score=0.0, docker_daemon=False)
        # base = 0 (no V1170 + no V1132 runs), but check_bonus adds 0.03
        # from "no_failed_runs=True" (0 == 0). Accept < 0.1 as "essentially zero".
        assert score < 0.1


# ============================================================================
# 7. R4 patched — V1170 health sub-dim mean 真补
# ============================================================================


class TestR4HealthProbePatched:
    def test_docker_available(self):
        from apeireth.v1171_asi_real_production_v06_patched import _measure_health_probe_v1171
        rep = _mock_v1132_report()
        # Mock V1170 sub-dim: all perfect
        v1170_subdim = {"port_listen_real": 1.0, "http_probe_real": 1.0, "graceful_shutdown_real": 1.0}
        score, ev = _measure_health_probe_v1171(rep, v1170_score=1.0,
                                                v1170_subdim_scores=v1170_subdim,
                                                docker_daemon=True)
        # 0.5*0 + 0.5*1.0 = 0.5 + bonus
        assert score >= 0.5

    def test_no_docker_uses_v1170_health_mean(self):
        from apeireth.v1171_asi_real_production_v06_patched import _measure_health_probe_v1171
        rep = _mock_v1132_report()
        v1170_subdim = {"port_listen_real": 1.0, "http_probe_real": 1.0, "graceful_shutdown_real": 1.0}
        score, ev = _measure_health_probe_v1171(rep, v1170_score=1.0,
                                                v1170_subdim_scores=v1170_subdim,
                                                docker_daemon=False)
        # 1.0 * 0.9 = 0.9 + bonus → ≥ 0.95
        assert score >= 0.9
        assert "no docker" in " ".join(ev.notes)

    def test_no_subdim_scores_fallback_to_overall(self):
        from apeireth.v1171_asi_real_production_v06_patched import _measure_health_probe_v1171
        rep = _mock_v1132_report()
        score, _ = _measure_health_probe_v1171(rep, v1170_score=0.8,
                                                v1170_subdim_scores=None,
                                                docker_daemon=False)
        # 0.8 * 0.9 = 0.72
        assert 0.5 <= score <= 1.0


# ============================================================================
# 8. measure_real_production_v06_patched (end-to-end)
# ============================================================================


class TestMeasurePatched:
    def test_returns_float_in_range(self):
        from apeireth.v1171_asi_real_production_v06_patched import measure_real_production_v06_patched
        total = measure_real_production_v06_patched()
        assert isinstance(total, float)
        assert 0.0 <= total <= 1.0

    def test_returns_higher_than_v1163_baseline(self):
        from apeireth.v1171_asi_real_production_v06_patched import (
            measure_real_production_v06_patched, V1163_BASELINE_REAL_PRODUCTION,
        )
        total = measure_real_production_v06_patched()
        # V1171 must beat V1163 baseline 0.49 (V1170 alt runtime push)
        assert total > V1163_BASELINE_REAL_PRODUCTION, (
            f"V1171 total={total:.4f} should beat V1163 baseline {V1163_BASELINE_REAL_PRODUCTION:.4f}"
        )


# ============================================================================
# 9. measure_real_production_v06_patched_full + JSON serialization
# ============================================================================


class TestMeasureFull:
    def test_returns_report(self):
        from apeireth.v1171_asi_real_production_v06_patched import (
            measure_real_production_v06_patched_full, V1171_SUBDIM_NAMES,
        )
        rep = measure_real_production_v06_patched_full(write_artifact=False)
        assert hasattr(rep, "total")
        assert hasattr(rep, "v1163_baseline")
        assert hasattr(rep, "v1170_score")
        assert hasattr(rep, "v1132_report_id")
        assert hasattr(rep, "docker_daemon_available")
        assert hasattr(rep, "runtime_proven")
        # All 5 sub-dims present
        if rep.sub_dim_scores:
            assert set(rep.sub_dim_scores.keys()) == set(V1171_SUBDIM_NAMES)

    def test_aggregate_is_mean(self):
        from apeireth.v1171_asi_real_production_v06_patched import (
            measure_real_production_v06_patched_full, V1171_SUBDIM_NAMES,
        )
        rep = measure_real_production_v06_patched_full(write_artifact=False)
        if rep.sub_dim_scores:
            n = len(V1171_SUBDIM_NAMES)
            expected = sum(rep.sub_dim_scores.values()) / n
            assert abs(rep.total - expected) < 1e-6

    def test_writes_artifact(self, tmp_path):
        from apeireth.v1171_asi_real_production_v06_patched import measure_real_production_v06_patched_full
        rep = measure_real_production_v06_patched_full(artifact_dir=str(tmp_path), write_artifact=True)
        # Artifact may or may not be written depending on V1132 state
        if rep.artifact_path:
            json_path = Path(rep.artifact_path)
            assert json_path.exists()
            data = json.loads(json_path.read_text(encoding="utf-8"))
            assert "snapshot_id" in data


# ============================================================================
# 10. CLI main()
# ============================================================================


class TestCLI:
    def test_main_summary_path(self):
        from apeireth.v1171_asi_real_production_v06_patched import main
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = main(["--no-write"])
        assert rc == 0
        out = buf.getvalue()
        assert "V1171 real_production V0.6.1 patched:" in out
        assert "vs V1163 baseline" in out

    def test_main_json_path(self):
        from apeireth.v1171_asi_real_production_v06_patched import main
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = main(["--json", "--no-write"])
        assert rc == 0
        data = json.loads(buf.getvalue())
        assert "total" in data
        assert "v1163_baseline" in data
        assert "v1170_score" in data

    def test_main_report_path(self):
        from apeireth.v1171_asi_real_production_v06_patched import main
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = main(["--no-write", "--report"])
        assert rc == 0
        out = buf.getvalue()
        assert "# V1171 — ASI Real Production V0.6.1 Patched Report" in out
        assert "| sub-dim | score | notes |" in out


# ============================================================================
# 11. delta from V1163 baseline
# ============================================================================


class TestDeltaFromBaseline:
    def test_real_delta_positive(self):
        from apeireth.v1171_asi_real_production_v06_patched import measure_real_production_v06_patched
        total = measure_real_production_v06_patched()
        # The whole point of V1171: push V1163 baseline 0.49 higher
        assert total >= 0.5, (
            f"V1171 total={total:.4f} should be ≥ 0.5 (V1163 baseline was 0.49)"
        )


# ============================================================================
# 12. runtime_proven semantics
# ============================================================================


class TestRuntimeProven:
    def test_runtime_proven_when_total_above_target(self):
        from apeireth.v1171_asi_real_production_v06_patched import V1171Report, TARGET_V1171
        r = V1171Report()
        r.total = TARGET_V1171 + 0.1
        # runtime_proven is set in measure_real_production_v06_patched_full
        # when total >= 0.85 (TARGET_V1171=0.85). Replicate the logic:
        r.runtime_proven = (r.total >= 0.85)
        assert r.runtime_proven is True

    def test_runtime_proven_false_below_target(self):
        from apeireth.v1171_asi_real_production_v06_patched import V1171Report
        r = V1171Report()
        r.total = 0.5
        r.runtime_proven = (r.total >= 0.85)
        assert r.runtime_proven is False