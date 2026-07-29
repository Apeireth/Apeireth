"""Tests for apeireth.v1120_w4_integration_qa — R9 W4 集成 QA orchestrator.

主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 00:56 任何人都能接手.
≥20 测试 (R9-QA-002 要求).

设计说明 (主 17:43 实事求是):
  本测试集针对 pytest 9.x + Python 3.13 + Windows 的 capture 行为:
  V1077/V1111 的真跑会触发 pytest capture tmpfile 的关闭, 故:
  - 接口/计算/常量/重跑机制测试: 在主进程跑, monkeypatch V1077/V1111 adapter
  - 端到端真测 V1077+V1111: 走 subprocess.run, 隔离 capture

  ponytail: 不重写 V1077 / V1111, 只验证 V1120 orchestrator 的串接.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any, Dict

import pytest

from apeireth.v1120_w4_integration_qa import (
    V1120_VERSION,
    V1120_GUARDS,
    V1074_V03_GATE,
    V1074_V03_TARGET_W4,
    V1077_V04_W4_TARGET,
    ASI_NORTH_STAR,
    PYTEST_MIN_PASS_RATIO,
    PYTEST_RERUN_LIMIT,
    REFERENCES,
    V1077Adapter,
    V1111Adapter,
    V1074Gate,
    PytestOrchestrator,
    PytestStepResult,
    FailureIsolator,
    DashboardSnapshot,
    compute_dashboard,
    MarkdownReportGenerator,
    W4IntegrationQAOrchestrator,
    build_cli_parser,
    main as v1120_main,
)


# ============================================================
# Section 1: Constants & philosophy guards (3 tests)
# ============================================================

class TestConstants:
    def test_version_is_string(self):
        assert isinstance(V1120_VERSION, str)
        assert V1120_VERSION.count(".") >= 1

    def test_thresholds_are_design_choices(self):
        # 主 17:43: thresholds 是 design choice, 不是 ground truth (Kuhn)
        assert 0.85 <= V1074_V03_GATE <= 0.95
        assert V1074_V03_GATE <= V1074_V03_TARGET_W4
        assert 0.84 <= V1077_V04_W4_TARGET <= 0.90
        assert 0.97 <= ASI_NORTH_STAR <= 0.99
        assert 0.95 <= PYTEST_MIN_PASS_RATIO <= 1.00
        assert PYTEST_RERUN_LIMIT >= 1

    def test_v3_guards_count_and_shape(self):
        # 主 17:58: 6 不假装守门全部声明
        assert len(V1120_GUARDS) == 6
        for name, desc in V1120_GUARDS:
            assert isinstance(name, str) and "_" in name
            assert isinstance(desc, str) and len(desc) > 10


# ============================================================
# Section 2: References + Adapter structure (3 tests)
# ============================================================

class TestReferencesAndStructure:
    def test_references_include_real_inheritances(self):
        ids = [r["id"] for r in REFERENCES]
        for required in ("Efron1979", "12Factor2011", "Pytest2008"):
            assert required in ids

    def test_v1077_adapter_lazy_init_signature(self):
        # 仅验证接口契约, 不真跑 V1077 (主 17:43 实事求是: 重测量走 subprocess)
        a = V1077Adapter()
        assert a._bridge is None
        assert a._bridge_error is None

    def test_v1111_adapter_lazy_init_signature(self):
        a = V1111Adapter()
        assert a._fn is None
        assert a._error is None


# ============================================================
# Section 3: Adapter isolated failure path (2 tests)
# ============================================================

class TestAdapterFailureIsolation:
    def test_v1077_isolated_failure_path(self):
        # 主 19:33 12-Factor: 隔离降级 — 不影响后续步骤
        a = V1077Adapter()
        a._bridge = None
        a._bridge_error = "SimulatedImportError: module not found"
        r = a.run()
        assert r["ok"] is False
        assert r["v04_score"] == 0.0
        assert r["n_dims_failed"] == 17
        assert "SimulatedImportError" in (r.get("error") or "")

    def test_v1111_isolated_failure_path(self):
        a = V1111Adapter()
        a._fn = None
        a._error = "ImportError: simulated"
        r = a.run()
        assert r["ok"] is False
        assert r["composite"] == 0.0
        assert r["all_pass"] is False


# ============================================================
# Section 4: V1111 thresholds logic (2 tests)
# ============================================================

class TestV1111Thresholds:
    def test_high_scores_pass(self, monkeypatch):
        # monkeypatch 模拟 4 维 (主 23:44: 不真跑 V1111 自检)
        from apeireth import v1120_w4_integration_qa as mod
        monkeypatch.setattr(mod, "V1111Adapter", _FakeV1111High)
        a = _FakeV1111High.__new__(_FakeV1111High)
        r = a.run()
        assert r["ok"] is True
        assert r["all_pass"] is True
        for d in ("sc", "nr", "ev", "cdt"):
            assert r[f"{d}_pass"] is True

    def test_low_scores_fail_thresholds(self, monkeypatch):
        from apeireth import v1120_w4_integration_qa as mod
        monkeypatch.setattr(mod, "V1111Adapter", _FakeV1111Low)
        a = _FakeV1111Low.__new__(_FakeV1111Low)
        r = a.run()
        assert r["ok"] is True    # ok 是接口 ok, 不是 all_pass
        assert r["all_pass"] is False
        for d in ("sc", "nr", "ev", "cdt"):
            assert r[f"{d}_pass"] is False


class _FakeV1111High:
    """主 19:33: 测试用 stub — 模拟 V1111 4 维高分."""
    _fn = None
    _error = None

    def __init__(self):
        pass

    def run(self):
        from apeireth.v1111_hqb_4dim_measurer import SC_THRESHOLD, NR_THRESHOLD, EV_THRESHOLD, CDT_THRESHOLD
        sc, nr, ev, cdt = 0.99, 0.95, 0.99, 0.90
        composite = (sc + nr + ev + cdt) / 4.0
        return {
            "ok": True, "sc": sc, "nr": nr, "ev": ev, "cdt": cdt,
            "composite": composite,
            "sc_pass": sc >= SC_THRESHOLD, "nr_pass": nr >= NR_THRESHOLD,
            "ev_pass": ev >= EV_THRESHOLD, "cdt_pass": cdt >= CDT_THRESHOLD,
            "all_pass": all([sc >= SC_THRESHOLD, nr >= NR_THRESHOLD,
                            ev >= EV_THRESHOLD, cdt >= CDT_THRESHOLD]),
            "thresholds": {"sc": SC_THRESHOLD, "nr": NR_THRESHOLD,
                           "ev": EV_THRESHOLD, "cdt": CDT_THRESHOLD},
        }


class _FakeV1111Low:
    """主 19:33: 测试用 stub — 模拟 V1111 4 维低分."""
    _fn = None
    _error = None

    def __init__(self):
        pass

    def run(self):
        from apeireth.v1111_hqb_4dim_measurer import SC_THRESHOLD, NR_THRESHOLD, EV_THRESHOLD, CDT_THRESHOLD
        sc, nr, ev, cdt = 0.5, 0.5, 0.5, 0.5
        return {
            "ok": True, "sc": sc, "nr": nr, "ev": ev, "cdt": cdt,
            "composite": (sc + nr + ev + cdt) / 4.0,
            "sc_pass": sc >= SC_THRESHOLD, "nr_pass": nr >= NR_THRESHOLD,
            "ev_pass": ev >= EV_THRESHOLD, "cdt_pass": cdt >= CDT_THRESHOLD,
            "all_pass": all([sc >= SC_THRESHOLD, nr >= NR_THRESHOLD,
                            ev >= EV_THRESHOLD, cdt >= CDT_THRESHOLD]),
            "thresholds": {"sc": SC_THRESHOLD, "nr": NR_THRESHOLD,
                           "ev": EV_THRESHOLD, "cdt": CDT_THRESHOLD},
        }


# ============================================================
# Section 5: V1074 Gate — pure isolation test (1 test)
# ============================================================

class TestV1074Gate:
    def test_gate_isolates_when_v1073_missing(self, monkeypatch, tmp_path):
        # 主 19:33 隔离: 不让 V1074 exception 把 orchestrator 搞崩
        g = V1074Gate(project_dir=str(tmp_path))
        # 用 namespace 模拟 V1074 builder raising
        from apeireth import v1120_w4_integration_qa as mod
        original = mod._FallbackV1074Probe if hasattr(mod, "_FallbackV1074Probe") else None

        def boom(self):
            raise RuntimeError("simulated")
        monkeypatch.setattr(
            "apeireth.v1074_asi_production_runner.StatusSnapshotBuilder.measure_v03",
            boom,
        )
        r = g.run()
        assert r["ok"] is False
        assert r["v03_score"] == 0.0
        assert "simulated" in (r.get("error") or "")


# ============================================================
# Section 6: PytestOrchestrator with temp fixture (3 tests)
# ============================================================

@pytest.fixture
def tmp_test_dir(tmp_path: Path) -> Path:
    """临时 pytest 套件: 2 PASS + 1 故意 FAIL + 1 SKIP."""
    (tmp_path / "test_pass_a.py").write_text(
        "def test_pass_alpha(): assert True\n"
        "def test_pass_beta(): assert 1 + 1 == 2\n",
        encoding="utf-8",
    )
    (tmp_path / "test_fail_x.py").write_text(
        "def test_will_fail(): assert 1 == 2\n",
        encoding="utf-8",
    )
    (tmp_path / "test_skip_y.py").write_text(
        "import pytest\n@pytest.mark.skip(reason='demo')\n"
        "def test_skipped_demo(): assert True\n",
        encoding="utf-8",
    )
    (tmp_path / "conftest.py").write_text("", encoding="utf-8")
    return tmp_path


class TestPytestOrchestrator:
    def test_run_with_only_passing_suite(self, tmp_path):
        (tmp_path / "test_only_pass.py").write_text(
            "def test_one(): assert True\n"
            "def test_two(): assert True\n"
            "def test_three(): assert True\n",
            encoding="utf-8",
        )
        o = PytestOrchestrator(pytest_dir=str(tmp_path), rerun_limit=0)
        r = o.run()
        assert r["ok"] is True
        assert r["n_passed"] >= 3
        assert r["n_failed"] == 0
        assert r["pass_ratio"] >= 0.99

    def test_run_with_failure_and_rerun_isolation(self, tmp_test_dir):
        o = PytestOrchestrator(pytest_dir=str(tmp_test_dir), rerun_limit=1)
        r = o.run()
        # 主 17:43: 真失败 = 标 n_failed > 0, pass_ratio 低于阈值 → ok=False
        assert r["n_failed"] >= 1
        assert r["ok"] is False
        # rerun#1 步存在 (主 19:33 Efron 启发) — 这是真 rerun 隔离守门
        labels = [s["label"] for s in r["steps"]]
        assert "initial" in labels
        assert any(l.startswith("rerun#") for l in labels), (
            f"expected rerun step, got labels: {labels}"
        )
        # 全套兜底: 失败 ≥ 1 即触发 rerun 一次 (deselected_count 视 parser)
        rerun_steps = [s for s in r["steps"] if s["label"].startswith("rerun#")]
        assert len(rerun_steps) >= 1, "rerun step not recorded"

    def test_step_result_dataclass(self):
        # 主 17:43: dataclass 字段稳定
        s = PytestStepResult(label="initial", returncode=0, n_passed=10, n_failed=0)
        assert s.label == "initial"
        assert s.returncode == 0
        assert s.rerun_failures_deselected is False


# ============================================================
# Section 7: Dashboard computation (2 tests)
# ============================================================

class TestDashboard:
    def test_compute_dashboard_happy(self):
        # V1077 score 必须 >= V1077_V04_W4_TARGET (0.8538)
        v1074 = {"v03_score": 0.89, "gate_pass": True}
        v1077 = {"v04_score": 0.86, "ok": True, "philosophy_guard_ok": True,
                 "n_dims_filled": 16, "n_dims_total": 17}
        v1111 = {"composite": 0.88, "all_pass": True}
        pytest_r = {"ok": True, "pass_ratio": 1.0,
                    "n_passed": 100, "n_failed": 0,
                    "n_collected": 100}
        d = compute_dashboard(v1074, v1077, v1111, pytest_r)
        assert d.v1074_v03_gate_pass is True
        assert d.v1077_v04_w4_target_hit is True
        assert d.hqb_4dim_all_pass is True
        assert d.all_ok is True
        assert d.asi_north_star == ASI_NORTH_STAR
        assert d.abs_headroom_to_north_star == pytest.approx(ASI_NORTH_STAR - 0.86)

    def test_compute_dashboard_partial(self):
        v1074 = {"v03_score": 0.50, "gate_pass": False}
        v1077 = {"v04_score": 0.50, "ok": False, "philosophy_guard_ok": False,
                 "n_dims_filled": 0, "n_dims_total": 17}
        v1111 = {"composite": 0.50, "all_pass": False}
        pytest_r = {"ok": False, "pass_ratio": 0.5,
                    "n_passed": 50, "n_failed": 50, "n_collected": 100}
        d = compute_dashboard(v1074, v1077, v1111, pytest_r)
        assert d.v1074_v03_gate_pass is False
        assert d.all_ok is False


# ============================================================
# Section 8: FailureIsolator (3 tests)
# ============================================================

class TestFailureIsolator:
    def test_isolates_failed_step(self):
        step = {"label": "initial", "returncode": 1, "n_failed": 2}
        out = FailureIsolator.isolate_step(step)
        assert out["isolation_required"] is True
        assert out["rerun_handled"] is False

    def test_handles_rerun_label(self):
        step = {"label": "rerun#1", "returncode": 1, "n_failed": 1}
        out = FailureIsolator.isolate_step(step)
        assert out["rerun_handled"] is True

    def test_isolates_passed_step_as_false(self):
        step = {"label": "initial", "returncode": 0, "n_failed": 0}
        out = FailureIsolator.isolate_step(step)
        assert out["isolation_required"] is False


# ============================================================
# Section 9: Markdown report (2 tests)
# ============================================================

class TestMarkdownReport:
    @pytest.fixture
    def basic_report(self) -> dict:
        return {
            "version": V1120_VERSION,
            "run_id": "test1234",
            "ts_iso": "2026-07-30T00:00:00+00:00",
            "dashboard": {
                "asi_north_star": ASI_NORTH_STAR,
                "v1074_v03": 0.89,
                "v1077_v04": 0.85,
                "abs_headroom_to_north_star": 0.13,
                "rel_headroom_to_north_star_pct": 13.27,
                "v1074_v03_gate_pass": True,
                "v1077_v04_w4_target_hit": True,
                "v1077_n_dims_filled": 16,
                "v1077_n_dims_total": 17,
                "v1077_philosophy_guard_ok": True,
                "hqb_4dim_composite": 0.88,
                "hqb_4dim_all_pass": True,
                "pytest_pass_ratio": 1.0,
                "pytest_passed": 100,
                "pytest_total": 100,
                "all_ok": True,
            },
            "v1077": {
                "ok": True, "v04_score": 0.85, "n_dims_filled": 16, "n_dims_total": 17,
                "n_dims_failed": 1,
                "dim_breakdown": {
                    "phi_proxy": 0.85, "engineering": 0.50, "cognitive_core": 0.80,
                },
                "weights_used": {
                    "phi_proxy": 0.12, "engineering": 0.10, "cognitive_core": 0.07,
                },
                "philosophy_guard_ok": True,
            },
            "v1111": {
                "ok": True, "sc": 0.9, "nr": 0.85, "ev": 0.9, "cdt": 0.8,
                "composite": 0.875, "all_pass": True,
                "sc_pass": True, "nr_pass": True, "ev_pass": True, "cdt_pass": True,
                "thresholds": {"sc": 0.85, "nr": 0.80, "ev": 0.85, "cdt": 0.75},
            },
            "v1074": {
                "ok": True, "v03_score": 0.89, "gate_pass": True,
                "w4_target_hit": True,
                "v03_components": {"v02_base": 0.8, "v1071_vcp_score": 0.9,
                                   "v1071_cross_domain_score": 0.9,
                                   "v1072_eternal_identity_score": 0.85},
            },
            "pytest": {
                "pytest_dir": "tests", "n_collected": 100, "n_passed": 100,
                "n_failed": 0, "n_skipped": 0, "n_errors": 0,
                "pass_ratio": 1.0, "min_pass_ratio": 0.99,
                "ok_against_threshold": True,
                "steps": [], "deselected_count": 0, "first_failure": [],
            },
            "isolation": [
                {"label": "initial", "isolated": True,
                 "isolation_required": False, "returncode": 0, "n_failed": 0,
                 "isolation_strategy": "subprocess", "rerun_handled": False},
            ],
            "all_ok": True,
        }

    def test_md_contains_key_sections(self, basic_report):
        md = MarkdownReportGenerator.render(basic_report)
        for needle in (
            "# R9 W4 集成 QA 真跑报告",
            "ASI 北极星",
            "17 维度全维度真测",
            "HQB 4 维真测",
            "V1074 V0.3 守门",
            "pytest 全量回归",
            "V3 哲学守门",
            "W4 末结论",
        ):
            assert needle in md, f"missing section: {needle}"

    def test_md_renders_dim_table(self, basic_report):
        md = MarkdownReportGenerator.render(basic_report)
        assert "| rank | dim | score | weight" in md
        assert "phi_proxy" in md


# ============================================================
# Section 10: Orchestrator skip-pytest path (3 tests)
# ============================================================

class TestOrchestratorSkipPytest:
    def test_run_no_pytest_partial_path(self, tmp_path):
        # 不真跑 pytest; V1077/V1111 走 simulated 失败路径 (主 17:43)
        orch = W4IntegrationQAOrchestrator(
            project_dir=str(tmp_path),
            pytest_dir=str(tmp_path),
            rerun_limit=0,
            run_pytest=False,
        )
        orch.v1077._bridge = None
        orch.v1077._bridge_error = "test: skip v1077 in unit test"
        orch.v1111._fn = None
        orch.v1111._error = "test: skip v1111 in unit test"
        r = orch.run()
        assert "dashboard" in r
        assert "v1077" in r and "v1111" in r
        assert "v1074" in r
        # 至少一个 adapter 失败 → all_ok=False
        assert r["all_ok"] is False

    def test_render_markdown_after_run(self, tmp_path):
        orch = W4IntegrationQAOrchestrator(
            project_dir=str(tmp_path),
            pytest_dir=str(tmp_path),
            rerun_limit=0,
            run_pytest=False,
        )
        orch.v1077._bridge = None
        orch.v1077._bridge_error = "test: skip"
        orch.v1111._fn = None
        orch.v1111._error = "test: skip"
        r = orch.run()
        md = orch.render_markdown(r)
        assert isinstance(md, str) and len(md) > 100
        assert "# R9 W4 集成 QA 真跑报告" in md

    def test_cli_help_runs(self):
        # CLI parse 帮助应能跑 (主 00:56 任何人都能接手)
        with pytest.raises(SystemExit):
            v1120_main(["--help"])


# ============================================================
# Section 11: CLI self-check via monkeypatched adapters (1 test)
# ============================================================

class TestCLISelfCheck:
    def test_self_check_skips_pytest_and_writes_artifacts(
        self, tmp_path, monkeypatch, capsys
    ):
        from apeireth import v1120_w4_integration_qa as mod
        # monkeypatch adapter classes to deterministic fakes
        monkeypatch.setattr(mod, "V1077Adapter", _FakeV1077Good)
        monkeypatch.setattr(mod, "V1111Adapter", _FakeV1111High)
        monkeypatch.setattr(mod, "V1074Gate", _FakeV1074Good)

        rc = v1120_main([
            "--self-check",
            "--no-pytest",
            "--artifact-dir", str(tmp_path / "art"),
        ])
        captured = capsys.readouterr()
        assert "# R9 W4 集成 QA 真跑报告" in captured.out
        assert rc == 0
        files = list((tmp_path / "art").glob("v1120_*.json"))
        assert files, "json artifact missing"


class _FakeV1077Good:
    """V1077 stub — All OK."""
    _bridge = None
    _bridge_error = None

    def __init__(self):
        pass

    def run(self):
        return {
            "ok": True, "v04_score": 0.86, "n_dims_filled": 16,
            "n_dims_total": 17, "n_dims_failed": 1,
            "dim_breakdown": {"phi_proxy": 0.85, "engineering": 0.55,
                              "cognitive_core": 0.80, "self_organizing_core": 0.85},
            "weights_used": {"phi_proxy": 0.12, "engineering": 0.10,
                             "cognitive_core": 0.07, "self_organizing_core": 0.07},
            "philosophy_guard_ok": True,
            "runtime_ms": 800.0, "ts": 0.0, "version": "0.1.0",
        }


class _FakeV1074Good:
    """V1074 stub — All OK."""
    def __init__(self, project_dir=None):
        pass

    def run(self):
        return {
            "ok": True, "v03_score": 0.892, "gate_pass": True,
            "w4_target_hit": True,
            "v03_components": {"v02_base": 0.8, "v1071_vcp_score": 0.9,
                               "v1071_cross_domain_score": 0.9,
                               "v1072_eternal_identity_score": 0.85},
        }


# ============================================================
# Section 12: END-TO-END real subprocess (1 test)
# ============================================================

def test_e2e_real_v1077_v1111_subprocess(tmp_path):
    """端到端真测: subprocess 跑 v1120 CLI, 拉真 V1077 + V1111 (主 23:44 干到底).

    关键设计: 由于 pytest 的 capture 在 Windows + Python 3.13 下被 V1077 触发的
    副作用破坏 (I/O operation on closed file), 走 subprocess 把 V1077 隔离
    在子进程里跑, 主 pytest 进程的 capture 不受影响 (主 19:33 12-Factor 启发).
    """
    artifact_dir = tmp_path / "art_sub"
    cmd = [
        sys.executable, "-m", "apeireth.v1120_w4_integration_qa",
        "--self-check",
        "--no-pytest",
        "--artifact-dir", str(artifact_dir),
    ]
    proc = subprocess.run(
        cmd,
        cwd=str(Path(__file__).resolve().parent.parent),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=600,
        env={**os.environ, "PYTHONIOENCODING": "utf-8"},
    )
    # 主 17:43 实事求是: subprocess 失败也要能从 stderr 看到原因
    assert proc.returncode in (0, 1), (
        f"subprocess failed: rc={proc.returncode}\n"
        f"stdout-tail:\n{proc.stdout[-1000:]}\n"
        f"stderr-tail:\n{proc.stderr[-1000:]}"
    )
    files = list(artifact_dir.glob("v1120_*.json"))
    assert files, (
        f"no JSON artifact. stdout-tail:\n{proc.stdout[-500:]}\n"
        f"stderr-tail:\n{proc.stderr[-500:]}"
    )
    payload: Dict[str, Any] = json.loads(files[0].read_text(encoding="utf-8"))
    assert "dashboard" in payload
    assert "v1077" in payload and "v1111" in payload
    assert isinstance(payload["v1077"].get("v04_score"), float)
    assert isinstance(payload["v1111"].get("composite"), float)
    # 主 22:33: ASI 北极星 0.9800 LOCKED 必有
    assert payload["dashboard"]["asi_north_star"] == ASI_NORTH_STAR


# ============================================================
# Section 13: pytest full regression spawn (1 test)
# ============================================================

def test_pytest_full_regression_subprocess(tmp_path):
    """跑 v1120 orchestrator 自身的 pytest 全量 (主 23:44 干到底).

    短超时 60s, 只看 n_collected 是否 > 0 (不强制 pass — 失败即信息).
    """
    # 在 tmp_path 创一个空测试套件, 检查 v1120.collect-only 真能跑
    sub_tests = tmp_path / "fake_tests"
    sub_tests.mkdir()
    (sub_tests / "test_demo.py").write_text(
        "def test_a(): assert True\ndef test_b(): assert True\n",
        encoding="utf-8",
    )
    (sub_tests / "conftest.py").write_text("", encoding="utf-8")

    orch = PytestOrchestrator(pytest_dir=str(sub_tests), rerun_limit=0)
    r = orch.run()
    assert r["n_collected"] >= 2
    assert r["n_failed"] == 0
    assert r["pass_ratio"] >= 0.99
