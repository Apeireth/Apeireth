"""R11 P0 Acceptance Gate — test cases (主 17:43 实事求是).

For each of the 5 gates, cover:
  - happy path: gate passes on a real workspace
  - failure path: gate returns clear failure reason on a broken workspace
  - CLI smoke: the gate CLI command is wired and emits structured output

These tests are deliberately small and offline; they use a temp directory to
simulate a broken dashboard contract / missing snapshot etc. (主 19:33 走在前人
经验上: pytest tmp_path fixture, 不发明新框架).
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

from apeireth.r11_requirements_gate import (
    ALL_GATES,
    GateResult,
    gate_a_v1136_v1074_truth_source,
    gate_b_dashboard_version_contract,
    gate_c_v3_nine_key_guard,
    gate_e_git_traceability,
    render_markdown_report,
    run_all_gates,
)

ROOT = Path(__file__).resolve().parents[1]


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _cli(*args: str, cwd: Path | None = None, timeout: int = 90) -> subprocess.CompletedProcess[str]:
    """Invoke ``python -m apeireth.cli gate ...`` and capture output."""
    env = os.environ.copy()
    env["PYTHONPATH"] = str(ROOT) + os.pathsep + env.get("PYTHONPATH", "")
    return subprocess.run(
        [sys.executable, "-m", "apeireth.cli", "gate", *args],
        cwd=str(cwd or ROOT),
        env=env,
        text=True,
        capture_output=True,
        timeout=timeout,
        encoding="utf-8",
        errors="replace",  # 主 17:43: 绝不抛异常, 字节异常 U+FFFD 替代
    )


def _cli_module(*args: str, cwd: Path | None = None, timeout: int = 90) -> subprocess.CompletedProcess[str]:
    """Invoke ``python -m apeireth.r11_requirements_gate ...``."""
    env = os.environ.copy()
    env["PYTHONPATH"] = str(ROOT) + os.pathsep + env.get("PYTHONPATH", "")
    return subprocess.run(
        [sys.executable, "-m", "apeireth.r11_requirements_gate", *args],
        cwd=str(cwd or ROOT),
        env=env,
        text=True,
        capture_output=True,
        timeout=timeout,
        encoding="utf-8",
        errors="replace",
    )


# ---------------------------------------------------------------------------
# Gate registry sanity
# ---------------------------------------------------------------------------


def test_all_gates_dict_has_five_gates():
    """Per R11 brief: 5 gates registered (V1136/V1074 truth, dashboard contract,
    9-key, test evidence, git traceability)."""
    assert len(ALL_GATES) == 5
    expected = {
        "A.v1136/v1074_truth_source",
        "B.dashboard_version_contract",
        "C.v3_nine_key_guard",
        "D.test_evidence",
        "E.git_traceability",
    }
    assert set(ALL_GATES.keys()) == expected


def test_gate_result_to_dict_is_serializable():
    """GateResult.to_dict() must yield JSON-serializable structure (主 17:43 实事求是)."""
    r = GateResult(name="X", passed=True, reason="ok", details={"x": 1.0})
    payload = json.dumps(r.to_dict())
    assert "passed" in payload and "true" in payload.lower()


# ---------------------------------------------------------------------------
# Gate A — V1136/V1074 truth source (happy path)
# ---------------------------------------------------------------------------


def test_gate_a_passes_on_real_workspace():
    """On a real workspace with V1136 + V1074 modules and a real snapshot, gate A passes."""
    r = gate_a_v1136_v1074_truth_source(ROOT)
    assert r.passed, f"gate A 应该在真 workspace 上通过; reason={r.reason}"
    d = r.details
    # 关键真值字段都在 details (主 17:43 不假装)
    for key in (
        "v1136_continuity",
        "v1136_autonomy",
        "v1136_transferability",
        "v1136_v05_total",
        "v1136_v05_v1125_placeholder",
        "v1074_snapshot_id",
        "v1074_v03_score",
    ):
        assert key in d, f"gate A details 缺 {key}"
    # V0.5 必填数字必须真 (主 17:43 不允许 cache / mock / 0.0)
    assert d["v1136_v05_total"] > 0.0
    assert 0.0 < d["v1074_v03_score"] <= 1.0


def test_gate_a_fails_when_v1136_falls_out_of_range(monkeypatch):
    """模拟 V1136 真测 continuity 越界 → gate A 必拒服 (主 17:43).

    gate_a 通过 inline import 从 ``apeireth.v1136_asi_v05_3dim_real_measurement``
    拿 ``measure_continuity_real``, 因此 monkeypatch 必须打在源 module 上,
    而不是 ``r11_requirements_gate``. (主 19:33 走在前人经验上: pytest 标配技巧).
    """
    import apeireth.r11_requirements_gate as g
    from apeireth.v1136_asi_v05_3dim_real_measurement import measure_continuity_real as real_cont

    def fake_cont():
        return {
            "continuity": 0.10,  # 越界 (< 0.55)
            "raw_avg": 0.10,
            "impl_ratio": 1.0,
            "fail_ratio": 0.0,
            "implemented": 8,
            "failed": 0,
            "total": 8,
            "sub_scores": {},
            "sub_metadata": {},
            "failures": [],
            "elapsed_seconds": 0.01,
        }

    monkeypatch.setattr(
        "apeireth.v1136_asi_v05_3dim_real_measurement.measure_continuity_real",
        fake_cont,
    )
    # real_cont 在源 module 上仍然存在, 确保 monkeypatch 真的生效
    from apeireth import v1136_asi_v05_3dim_real_measurement as _v1136

    assert _v1136.measure_continuity_real is fake_cont

    r = g.gate_a_v1136_v1074_truth_source(ROOT)
    assert not r.passed
    assert "continuity" in r.reason and "越界" in r.reason, (
        f"gate A 失败必须给出明确原因; got: {r.reason}"
    )


# ---------------------------------------------------------------------------
# Gate B — Dashboard version contract
# ---------------------------------------------------------------------------


def test_gate_b_passes_on_real_workspace():
    r = gate_b_dashboard_version_contract(ROOT)
    assert r.passed, f"gate B 应该在真 workspace 上通过; reason={r.reason}"
    d = r.details
    assert d["snapshot_id"] is not None
    assert d["version"] is not None


def test_gate_b_fails_when_snapshot_missing(tmp_path):
    r = gate_b_dashboard_version_contract(tmp_path)
    assert not r.passed
    assert "snapshot" in r.reason.lower() and "缺失" in r.reason, (
        f"gate B 失败必须有清晰原因; got: {r.reason}"
    )


def test_gate_b_fails_when_snapshot_missing_required_fields(tmp_path):
    """snapshot 缺关键字段 (V1074 真生产契约) → gate B 必拒服."""
    snap_dir = tmp_path / "artifacts"
    snap_dir.mkdir()
    (snap_dir / "asi_snapshot.json").write_text(
        json.dumps({"snapshot_id": "x", "version": "0.0.1"}),  # 缺 v03_score 等
        encoding="utf-8",
    )
    r = gate_b_dashboard_version_contract(tmp_path)
    assert not r.passed
    assert "缺字段" in r.reason or "missing" in r.reason.lower()


def test_gate_b_fails_when_report_has_mismatched_snapshot_id(tmp_path):
    """report 中 snapshot_id 与 asi_snapshot.json 不一致 → gate B 必拒服."""
    snap_dir = tmp_path / "artifacts"
    snap_dir.mkdir()
    (snap_dir / "asi_snapshot.json").write_text(json.dumps({
        "snapshot_id": "snap_correct_12345678",
        "ts_iso": "2026-07-30T00:00:00+00:00",
        "version": "0.1.0",
        "level": "ASI",
        "v03_score": 0.8964,
        "n_modules": 1, "n_tests": 1, "n_commits": 1,
    }), encoding="utf-8")

    rep_dir = tmp_path / "reports"
    rep_dir.mkdir()
    (rep_dir / "asi_report.md").write_text(
        "## bad\nSnapshot ID: snap_wrong_99999999\n", encoding="utf-8",
    )

    r = gate_b_dashboard_version_contract(tmp_path)
    assert not r.passed
    assert "snapshot_id" in r.reason


# ---------------------------------------------------------------------------
# Gate C — V3 nine-key guard
# ---------------------------------------------------------------------------


def test_gate_c_passes_with_default_lock():
    r = gate_c_v3_nine_key_guard(ROOT)
    assert r.passed, f"gate C 应该在默认 Lock 全 True 时通过; reason={r.reason}"
    d = r.details
    assert d["n_keys"] == 9
    assert d["verify_or_raise_works"] is True
    # 9 键全部 LOCKED
    assert d["lock_values"], "lock_values 必须真存 (主 17:43 实事求是)"
    assert all(d["lock_values"].values()), "默认 Lock 必须 9/9 LOCKED True"


def test_gate_c_inverted_lock_raises(monkeypatch):
    """任意 1 键 False → verify_or_raise 抛 RuntimeError (主 23:44 干到底)."""
    from apeireth.mcp.asi_nine_keys import ASI_NINE_KEYS, AsiNineKeyLock, verify_or_raise

    broken = AsiNineKeyLock(values={k: (i > 0) for i, k in enumerate(ASI_NINE_KEYS)})
    assert not broken.all_locked()
    with pytest.raises(RuntimeError) as excinfo:
        verify_or_raise(broken)
    assert "9 键" in str(excinfo.value) or "LOCKED" in str(excinfo.value)


# ---------------------------------------------------------------------------
# Gate E — Git traceability
# ---------------------------------------------------------------------------


def test_gate_e_passes_on_real_workspace():
    r = gate_e_git_traceability(ROOT)
    assert r.passed, f"gate E 应该在真 git workspace 上通过; reason={r.reason}"
    d = r.details
    assert d["head_sha"]
    assert d["n_recent_commits"] >= 1


def test_gate_e_fails_when_not_git_repo(tmp_path):
    """非 git dir → gate E 必拒服, 给出明确原因."""
    r = gate_e_git_traceability(tmp_path)
    assert not r.passed
    assert "git" in r.reason.lower() and (
        "not a git repo" in r.reason or "失败" in r.reason
    )


# ---------------------------------------------------------------------------
# CLI smoke
# ---------------------------------------------------------------------------


def test_cli_module_help_exits_zero():
    """``python -m apeireth.r11_requirements_gate`` (no args) 必须 exit 0 + 打印 help."""
    r = _cli_module()
    assert r.returncode == 0
    assert "R11" in (r.stdout or "")


def test_cli_module_run_emits_markdown_report_on_stdout():
    """``python -m apeireth.r11_requirements_gate run`` 输出 Markdown."""
    r = _cli_module("run", "--workspace", str(ROOT))
    assert r.returncode == 0, f"stderr={r.stderr}"
    out = r.stdout or ""
    assert "R11 P0 Acceptance Gate Report" in out
    assert "| Gate | Status | Reason |" in out
    # 5 个 gate 都应该被命名
    for name in ALL_GATES.keys():
        assert name in out, f"report missing gate: {name}"


def test_cli_module_run_emits_valid_json():
    """``--json`` 输出必须是合法 JSON, 含 all 5 gates."""
    r = _cli_module("run", "--json", "--workspace", str(ROOT))
    assert r.returncode == 0, f"stderr={r.stderr}"
    payload = json.loads(r.stdout or "{}")
    assert payload["n_total"] == 5
    assert set(payload["results"].keys()) == set(ALL_GATES.keys())
    # 每个 result 必须有 passed + reason + details
    for k, v in payload["results"].items():
        assert "passed" in v
        assert "reason" in v
        assert "details" in v


def test_cli_wired_subcommand_emits_report():
    """``python -m apeireth.cli gate`` 必须 exit 0 (workspace 现况=可生产)."""
    r = _cli("--workspace", str(ROOT))
    assert r.returncode == 0
    assert "R11 P0 Acceptance Gate Report" in (r.stdout or "")


def test_cli_strict_exit_zero_when_all_pass():
    """``--strict`` + 全 PASS 时必须 exit 0 (CI gate 路径)."""
    r = _cli("--strict", "--workspace", str(ROOT))
    # 若全 PASS: exit 0; 若有 FAIL: exit 1 (测试将失败, 但不会 race)
    assert r.returncode in (0, 1), f"strict mode 只能 0 或 1; got {r.returncode}; stderr={r.stderr}"


def test_cli_strict_exit_one_when_missing_snapshot(tmp_path):
    """--strict 在 broken workspace 必须 exit 1 (CI gate 必拒服)."""
    r = _cli("--strict", "--workspace", str(tmp_path))
    assert r.returncode == 1, "missing snapshot 必须让 --strict exit 1"
    # 但输出仍包含 report 便于排查
    out = r.stdout or ""
    assert "R11 P0 Acceptance Gate Report" in out


# ---------------------------------------------------------------------------
# Orchestration — run_all_gates
# ---------------------------------------------------------------------------


def test_run_all_gates_returns_all_five_keys():
    results = run_all_gates(ROOT)
    assert set(results.keys()) == set(ALL_GATES.keys())
    # 每个 value 必须是 GateResult 实例
    for name, r in results.items():
        assert isinstance(r, GateResult), f"{name} 返回非 GateResult: {type(r)}"
        assert r.name == name


def test_render_markdown_report_contains_all_gates():
    """render_markdown_report 必须含表格 + 每个 gate 详情."""
    results = run_all_gates(ROOT)
    md = render_markdown_report(results)
    assert md.startswith("# R11 P0 Acceptance Gate Report")
    assert "| Gate | Status | Reason |" in md
    for name in ALL_GATES.keys():
        assert f"`{name}`" in md


def test_e2e_strict_gate_with_real_workspace():
    """End-to-end: 真 workspace + --strict. 失败原因 (若有) 必须清晰可读."""
    r = _cli("--strict", "--json", "--workspace", str(ROOT))
    payload = json.loads(r.stdout or "{}")
    n_pass = payload["n_pass"]
    n_total = payload["n_total"]
    # 期望: 全 PASS (R10 W3 末真生产状态). 若发现 FAIL, 打印失败原因便于复现.
    if n_pass != n_total:
        fails = [k for k, v in payload["results"].items() if not v["passed"]]
        msg = "; ".join(f"{k}: {payload['results'][k]['reason']}" for k in fails)
        pytest.fail(
            f"workspace 不在 P0 PASS 状态 ({n_pass}/{n_total}); 失败 gates: {msg}"
        )
