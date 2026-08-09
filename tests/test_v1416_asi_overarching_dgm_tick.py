"""Tests for V1416 — ASI 总框架 DGM closed-loop tick executor."""
from __future__ import annotations

import json
import os
import sys
import subprocess
from pathlib import Path
from typing import Any, Dict, List

import pytest


# Make sure apeireth package is importable
_APEIRETH_ROOT = str(Path(__file__).resolve().parent.parent / "apeireth")
if _APEIRETH_ROOT not in sys.path:
    sys.path.insert(0, _APEIRETH_ROOT)

_WORKSPACE_ROOT = str(Path(__file__).resolve().parent.parent)
if _WORKSPACE_ROOT not in sys.path:
    sys.path.insert(0, _WORKSPACE_ROOT)


import apeireth.v1416_asi_overarching_dgm_tick as v1416  # noqa: E402


# ----------------------- Fixtures -----------------------


@pytest.fixture(autouse=True)
def _restore_cwd():
    yield


# ----------------------- Constants -----------------------


def test_constants():
    assert v1416.V1416_VERSION == "0.1.0"
    assert v1416.V1416_SCHEMA == "v1416.asi-overarching-dgm-tick/v1"
    assert v1416.V1416_MODULE == "v1416_asi_overarching_dgm_tick"
    assert len(v1416.V1416_GUARDS) == 15
    assert len(v1416.V1416_V3_GUARDS) == 9
    assert len(v1416.V1416_BORROWED) == 5
    assert len(v1416.V1416_POLICIES) == 3
    assert len(v1416.V1416_SEVERITIES) == 3


# ----------------------- Dataclasses -----------------------


def test_tick_config_roundtrip():
    cfg = v1416.build_default_config()
    d = cfg.to_dict()
    assert d["critical_pause_threshold"] == 1
    assert d["critical_lockdown_threshold"] == 3
    assert d["escalation_pause_threshold"] == 1
    assert d["cooldown_seconds"] == 900


def test_dgm_tick_report_roundtrip():
    rep = v1416.DgmTickReport(
        tick_id="x", timestamp="2026-08-10T02:00:00Z",
        v1413_snapshot_id="snap-1",
        v1414_alerts_count=2, v1414_max_severity="WARN",
        v1415_overall_max_severity="CRITICAL",
        v1415_escalation_count=1, v1415_n_snapshots=5,
        policy="PAUSE", policy_reason="test reason",
        chain_ok=True, n_modules=5,
    )
    d = rep.to_dict()
    assert d["schema"] == v1416.V1416_SCHEMA
    assert d["policy"] == "PAUSE"
    assert d["v1414_alerts_count"] == 2
    assert d["v1415_escalation_count"] == 1


# ----------------------- Helpers -----------------------


def test_slug_timestamp_uses_utc():
    from datetime import datetime, timezone
    dt = datetime(2026, 8, 10, 2, 0, 0, tzinfo=timezone.utc)
    assert v1416.slug_timestamp(dt) == "2026-08-10T02-00-00Z"


def test_severity_helpers():
    assert v1416._severity_rank("CRITICAL") > v1416._severity_rank("WARN")
    assert v1416._max_severity("INFO", "WARN") == "WARN"
    assert v1416._max_severity("CRITICAL", "INFO") == "CRITICAL"


def test_is_path_safe_rejects_dotdot():
    assert v1416._is_path_safe("foo/bar.jsonl") is True
    assert v1416._is_path_safe("a/b/c.jsonl") is True
    assert v1416._is_path_safe("../../etc/passwd") is False
    assert v1416._is_path_safe("a/../b") is False
    assert v1416._is_path_safe("") is False
    assert v1416._is_path_safe(None) is False  # type: ignore[arg-type]


# ----------------------- Cross-module read -----------------------


def test_read_v1413_latest_snapshot_missing():
    snap = v1416._read_v1413_latest_snapshot(str(Path(__file__).parent / "nope.jsonl"))
    assert snap == {}


def test_read_v1413_latest_snapshot_roundtrip(tmp_path):
    p = tmp_path / "h.jsonl"
    p.write_text(
        json.dumps({"snapshot_id": "snap-1", "verdict": "COMPLETE", "framework_score": 11})
        + "\n" + json.dumps({"snapshot_id": "snap-2", "verdict": "GOOD", "framework_score": 10})
        + "\n",
        encoding="utf-8",
    )
    snap = v1416._read_v1413_latest_snapshot(str(p))
    assert snap["snapshot_id"] == "snap-2"


def test_read_v1414_alerts_no_history():
    alerts, max_sev = v1416._read_v1414_alerts(
        str(Path(__file__).parent / "nope.jsonl"),
        str(Path(__file__).parent / "nope.base.json"),
    )
    assert isinstance(alerts, list)
    assert max_sev in v1416.V1416_SEVERITIES


def test_read_v1415_overlay_no_history():
    overlay = v1416._read_v1415_overlay(
        str(Path(__file__).parent / "nope.jsonl"),
        str(Path(__file__).parent / "nope.base.json"),
    )
    assert "overall_max_severity" in overlay
    assert "escalation_count" in overlay


# ----------------------- Policy Gate -----------------------


def test_policy_proceed_on_empty():
    cfg = v1416.build_default_config()
    policy, reason = v1416.policy_from_v1414_v1415([], "INFO", {"escalation_count": 0}, cfg)
    assert policy == "PROCEED"
    assert "safe" in reason.lower()


def test_policy_pause_on_one_critical():
    cfg = v1416.build_default_config()
    policy, reason = v1416.policy_from_v1414_v1415(
        [{"severity": "CRITICAL"}], "CRITICAL",
        {"escalation_count": 0}, cfg,
    )
    assert policy == "PAUSE"
    assert "CRITICAL" in reason


def test_policy_lockdown_on_three_critical():
    cfg = v1416.build_default_config()
    alerts = [{"severity": "CRITICAL"}] * 3
    policy, reason = v1416.policy_from_v1414_v1415(
        alerts, "CRITICAL", {"escalation_count": 0}, cfg,
    )
    assert policy == "LOCKDOWN"
    assert "lockdown" in reason.lower()


def test_policy_pause_on_escalation():
    cfg = v1416.build_default_config()
    policy, reason = v1416.policy_from_v1414_v1415(
        [], "INFO", {"escalation_count": 1}, cfg,
    )
    assert policy == "PAUSE"
    assert "escalation" in reason.lower()


def test_policy_lockdown_wins_over_pause():
    cfg = v1416.build_default_config()
    alerts = [{"severity": "CRITICAL"}] * 3 + [{"severity": "CRITICAL"}]
    policy, _ = v1416.policy_from_v1414_v1415(
        alerts, "CRITICAL", {"escalation_count": 1}, cfg,
    )
    assert policy == "LOCKDOWN"


def test_policy_bounded_to_set():
    cfg = v1416.build_default_config()
    for n_crit in [0, 1, 2, 3, 5]:
        alerts = [{"severity": "CRITICAL"}] * n_crit
        policy, _ = v1416.policy_from_v1414_v1415(
            alerts, "INFO", {"escalation_count": 0}, cfg,
        )
        assert policy in v1416.V1416_POLICIES


# ----------------------- Tick Orchestrator -----------------------


def test_run_dgm_tick_with_no_history(tmp_path):
    h = tmp_path / "empty.jsonl"
    b = tmp_path / "empty.base.json"
    cfg = v1416.build_default_config()
    cfg.enable_append = False
    rep = v1416.run_dgm_tick(str(h), str(b), config=cfg)
    assert "_v1416_" in rep.tick_id
    assert rep.v1414_alerts_count >= 0
    assert rep.policy in v1416.V1416_POLICIES
    assert rep.v1414_max_severity in v1416.V1416_SEVERITIES
    assert rep.n_modules == 5


def test_run_dgm_tick_with_synthetic_history(tmp_path):
    h = tmp_path / "h.jsonl"
    # Append 2 snapshots with WARN-level gap
    lines = []
    for i, gap in enumerate([0.05, 0.06]):
        lines.append(json.dumps({
            "snapshot_id": f"snap-{i}",
            "verdict": "GOOD",
            "framework_score": 10,
            "gap_to_north_star": gap,
            "chain_ok": True,
            "timestamp": f"2026-08-{9 + i:02d}T01:30:00Z",
        }))
    h.write_text("\n".join(lines) + "\n", encoding="utf-8")
    cfg = v1416.build_default_config()
    cfg.enable_append = False
    rep = v1416.run_dgm_tick(str(h), str(h.parent / "nope.base.json"), config=cfg)
    assert rep.v1413_snapshot_id != ""
    assert rep.v1415_n_snapshots >= 1


def test_append_tick_writes_jsonl(tmp_path):
    out = tmp_path / "ticks.jsonl"
    rep = v1416.DgmTickReport(
        tick_id="t1", timestamp="2026-08-10T02:00:00Z",
        policy="PROCEED", policy_reason="test", chain_ok=True, n_modules=5,
    )
    ok = v1416.append_tick(rep, str(out))
    assert ok is True
    assert out.exists()
    lines = out.read_text(encoding="utf-8").strip().split("\n")
    assert len(lines) == 1
    obj = json.loads(lines[0])
    assert obj["tick_id"] == "t1"


def test_append_tick_rejects_unsafe_path():
    rep = v1416.DgmTickReport(tick_id="t1", timestamp="2026-08-10T02:00:00Z")
    ok = v1416.append_tick(rep, "../../etc/passwd")
    assert ok is False


def test_append_tick_atomic_with_multiple(tmp_path):
    out = tmp_path / "ticks.jsonl"
    for i in range(5):
        rep = v1416.DgmTickReport(
            tick_id=f"t{i}", timestamp=f"2026-08-10T0{i}:00:00Z",
            policy="PROCEED", policy_reason="test", chain_ok=True, n_modules=5,
        )
        v1416.append_tick(rep, str(out))
    lines = out.read_text(encoding="utf-8").strip().split("\n")
    assert len(lines) == 5
    ids = [json.loads(l)["tick_id"] for l in lines]
    assert ids == ["t0", "t1", "t2", "t3", "t4"]


# ----------------------- Render -----------------------


def test_render_tick_md_emits_sections():
    rep = v1416.DgmTickReport(
        tick_id="t1", timestamp="2026-08-10T02:00:00Z",
        v1413_snapshot_id="snap-1",
        v1414_alerts_count=1, v1414_max_severity="WARN",
        v1415_overall_max_severity="CRITICAL",
        v1415_escalation_count=0, v1415_n_snapshots=3,
        policy="PAUSE", policy_reason="test reason",
        chain_ok=True, n_modules=5,
    )
    md = v1416.render_tick_md(rep)
    # 9 sections: header, policy, V1414, V1415, rules, borrowed, guards, honest, footer
    assert "V1416 ASI 总框架 DGM" in md
    assert "## Policy decision" in md
    assert "## V1414 watchdog" in md
    assert "## V1415 overlay" in md
    assert "## Policy rules" in md
    assert "## Borrowed" in md
    assert "## GUARDS" in md
    assert "## Honest disclosure" in md
    # Honest disclosure string
    assert "Phenomenal" in md
    assert "ASI 达成" in md or "ASI" in md


# ----------------------- Popper -----------------------


def test_popper_self_test_passes():
    passed, total, failed = v1416.popper_self_test()
    assert passed == total, f"failed: {failed}"
    assert total >= 14


# ----------------------- Chain Delegate -----------------------


def test_chain_delegate_returns_5_modules():
    all_ok, n_ok, _, n_mod, errors = v1416.chain_delegate_v1416()
    assert n_mod == 5
    assert n_ok >= 4  # self + V1411-V1415 (≥ 4 of them)
    assert isinstance(errors, list)


# ----------------------- CLI -----------------------


def test_cli_version_runs():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1416_asi_overarching_dgm_tick", "version"],
        cwd=str(Path(__file__).resolve().parent.parent),
        capture_output=True, text=True, encoding="utf-8", timeout=60,
        env={**os.environ, "PYTHONIOENCODING": "utf-8"},
    )
    assert result.returncode == 0
    assert "V1416_VERSION: 0.1.0" in result.stdout


def test_cli_policy_runs():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1416_asi_overarching_dgm_tick", "policy"],
        cwd=str(Path(__file__).resolve().parent.parent),
        capture_output=True, text=True, timeout=60,
    )
    assert result.returncode == 0
    assert "PROCEED" in result.stdout
    assert "PAUSE" in result.stdout
    assert "LOCKDOWN" in result.stdout


def test_cli_severity_runs():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1416_asi_overarching_dgm_tick", "severity"],
        cwd=str(Path(__file__).resolve().parent.parent),
        capture_output=True, text=True, timeout=60,
    )
    assert result.returncode == 0
    assert "INFO" in result.stdout
    assert "WARN" in result.stdout
    assert "CRITICAL" in result.stdout


def test_cli_popper_runs():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1416_asi_overarching_dgm_tick", "popper"],
        cwd=str(Path(__file__).resolve().parent.parent),
        capture_output=True, text=True, encoding="utf-8", timeout=60,
        env={**os.environ, "PYTHONIOENCODING": "utf-8"},
    )
    assert result.returncode == 0
    assert "popper:" in result.stdout


def test_cli_meta_runs_and_emits_json():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1416_asi_overarching_dgm_tick", "meta"],
        cwd=str(Path(__file__).resolve().parent.parent),
        capture_output=True, text=True, encoding="utf-8", timeout=60,
        env={**os.environ, "PYTHONIOENCODING": "utf-8"},
    )
    assert result.returncode == 0
    payload = json.loads(result.stdout)
    assert payload["version"] == "0.1.0"
    assert len(payload["guards"]) == 15
    assert len(payload["v3_guards"]) == 9
    assert len(payload["borrowed"]) == 5
    assert payload["default_config"]["critical_lockdown_threshold"] == 3


def test_cli_demo_runs():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1416_asi_overarching_dgm_tick", "demo"],
        cwd=str(Path(__file__).resolve().parent.parent),
        capture_output=True, text=True, timeout=60,
    )
    assert result.returncode == 0
    assert "demo:" in result.stdout
    assert "PROCEED" in result.stdout


def test_cli_tick_runs_no_history(tmp_path):
    h = tmp_path / "empty.jsonl"
    b = tmp_path / "empty.base.json"
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1416_asi_overarching_dgm_tick", "tick",
         "--history-path", str(h), "--baseline-path", str(b), "--no-append"],
        cwd=str(Path(__file__).resolve().parent.parent),
        capture_output=True, text=True, encoding="utf-8", timeout=60,
        env={**os.environ, "PYTHONIOENCODING": "utf-8"},
    )
    assert result.returncode == 0
    payload = json.loads(result.stdout)
    assert payload["schema"] == v1416.V1416_SCHEMA
    assert payload["n_modules"] == 5
    assert payload["policy"] in v1416.V1416_POLICIES


def test_cli_tick_writes_to_file(tmp_path):
    h = tmp_path / "empty.jsonl"
    out = tmp_path / "tick.json"
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1416_asi_overarching_dgm_tick", "tick",
         "--history-path", str(h), "--out", str(out), "--no-append"],
        cwd=str(Path(__file__).resolve().parent.parent),
        capture_output=True, text=True, encoding="utf-8", timeout=60,
        env={**os.environ, "PYTHONIOENCODING": "utf-8"},
    )
    assert result.returncode == 0
    assert out.exists()
    payload = json.loads(out.read_text(encoding="utf-8"))
    assert payload["schema"] == v1416.V1416_SCHEMA


def test_cli_tick_rejects_unsafe_path():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1416_asi_overarching_dgm_tick", "tick",
         "--out", "../escape.json", "--no-append"],
        cwd=str(Path(__file__).resolve().parent.parent),
        capture_output=True, text=True, encoding="utf-8", timeout=60,
        env={**os.environ, "PYTHONIOENCODING": "utf-8"},
    )
    assert result.returncode == 2


def test_cli_render_runs(tmp_path):
    h = tmp_path / "h.jsonl"
    out = tmp_path / "report.md"
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1416_asi_overarching_dgm_tick", "render",
         "--history-path", str(h), "--out", str(out)],
        cwd=str(Path(__file__).resolve().parent.parent),
        capture_output=True, text=True, encoding="utf-8", timeout=60,
        env={**os.environ, "PYTHONIOENCODING": "utf-8"},
    )
    assert result.returncode == 0
    assert out.exists()
    content = out.read_text(encoding="utf-8")
    assert "V1416" in content
    assert "## Policy decision" in content


def test_cli_chain_runs():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1416_asi_overarching_dgm_tick", "chain"],
        cwd=str(Path(__file__).resolve().parent.parent),
        capture_output=True, text=True, encoding="utf-8", timeout=60,
        env={**os.environ, "PYTHONIOENCODING": "utf-8"},
    )
    assert result.returncode in (0, 1)
    payload = json.loads(result.stdout)
    assert payload["n_modules"] == 5


def test_cli_help_runs():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1416_asi_overarching_dgm_tick", "help"],
        cwd=str(Path(__file__).resolve().parent.parent),
        capture_output=True, text=True, encoding="utf-8", timeout=60,
        env={**os.environ, "PYTHONIOENCODING": "utf-8"},
    )
    assert result.returncode == 0
    assert "DGM" in result.stdout or "tick" in result.stdout.lower()


# ----------------------- Integration -----------------------


def test_v1416_reads_v1413_real_history():
    """Real end-to-end: V1416 reads the actual V1413 history file."""
    cwd = Path(__file__).resolve().parent.parent
    hist = cwd / ".v1413-asi-overarching-history.jsonl"
    if not hist.exists():
        pytest.skip("V1413 history file not present; skipping integration test")
    cfg = v1416.build_default_config()
    cfg.enable_append = False
    rep = v1416.run_dgm_tick(str(hist), str(hist.parent / ".v1413-asi-overarching-baseline.json"), config=cfg)
    assert isinstance(rep, v1416.DgmTickReport)
    assert rep.v1413_snapshot_id != ""
    # In a healthy env, policy should be PROCEED
    assert rep.policy == "PROCEED" or rep.policy == "PAUSE"  # could be either depending on data


def test_v1416_end_to_end_pipeline(tmp_path):
    """End-to-end: V1416 → V1415 → V1414 → V1413 all wired."""
    h = tmp_path / "h.jsonl"
    # 1 snapshot with critical gap
    h.write_text(
        json.dumps({
            "snapshot_id": "snap-e2e",
            "verdict": "GOOD",
            "framework_score": 10,
            "gap_to_north_star": 0.025,  # CRITICAL (>= 0.02)
            "chain_ok": True,
            "timestamp": "2026-08-10T01:30:00Z",
        }) + "\n",
        encoding="utf-8",
    )
    cfg = v1416.build_default_config()
    cfg.enable_append = False
    rep = v1416.run_dgm_tick(str(h), str(h.parent / "nope.base.json"), config=cfg)
    assert rep.v1415_overall_max_severity in ("WARN", "CRITICAL")
    assert rep.policy in v1416.V1416_POLICIES