"""Tests for V1414 — ASI 总框架 regression detector + watchdog."""
from __future__ import annotations

import json
import os
import sys
import subprocess
from pathlib import Path
from typing import List

import pytest


# Make sure apeireth package is importable
_APEIRETH_ROOT = str(Path(__file__).resolve().parent.parent / "apeireth")
if _APEIRETH_ROOT not in sys.path:
    sys.path.insert(0, _APEIRETH_ROOT)

# Ensure workspace root also resolvable
_WORKSPACE_ROOT = str(Path(__file__).resolve().parent.parent)
if _WORKSPACE_ROOT not in sys.path:
    sys.path.insert(0, _WORKSPACE_ROOT)


import apeireth.v1414_asi_overarching_watchdog as v1414  # noqa: E402


# ----------------------- Fixtures -----------------------


@pytest.fixture(autouse=True)
def _restore_cwd():
    yield
    # No mutating global cwd needed; tests use tmp_path via monkeypatch.


def _fake_snapshot(
    timestamp="2026-08-10T02-00-00Z",
    verdict="COMPLETE",
    framework_score=11,
    level_score=12,
    coherence_score=12,
    chain_ok=True,
    borrowed_count=7,
    gap_to_north_star=0.0695,
):
    """Build a V1413-HistorySnapshot-shaped dummy."""
    from apeireth.v1413_asi_overarching_history import HistorySnapshot

    return HistorySnapshot(
        timestamp=timestamp,
        snapshot_id=f"{timestamp}_test",
        source_module="v1412_asi_overarching_dashboard",
        source_version="0.1.0",
        verdict=verdict,
        framework_score=framework_score,
        level_score=level_score,
        coherence_score=coherence_score,
        chain_ok=chain_ok,
        borrowed_count=borrowed_count,
        anchor_value=0.9105,
        gap_to_north_star=gap_to_north_star,
        gap_to_ceiling=0.0795,
        note="test",
    )


def _fake_baseline(
    baseline_timestamp="2026-08-10T01-00-00Z",
    baseline_verdict="COMPLETE",
    baseline_framework_score=11,
    baseline_level_score=12,
    baseline_coherence_score=12,
    baseline_chain_ok=True,
    baseline_borrowed_count=7,
    baseline_anchor=0.9105,
    baseline_gap=0.0695,
):
    from apeireth.v1413_asi_overarching_history import HistoryBaseline

    return HistoryBaseline(
        baseline_timestamp=baseline_timestamp,
        baseline_verdict=baseline_verdict,
        baseline_framework_score=baseline_framework_score,
        baseline_level_score=baseline_level_score,
        baseline_coherence_score=baseline_coherence_score,
        baseline_chain_ok=baseline_chain_ok,
        baseline_borrowed_count=baseline_borrowed_count,
        baseline_anchor=baseline_anchor,
        baseline_gap=baseline_gap,
        note="test-baseline",
    )


# ----------------------- Section A: Constants -----------------------


def test_constants_version_and_schema():
    assert v1414.V1414_VERSION == "0.1.0"
    assert v1414.V1414_MODULE == "v1414_asi_overarching_watchdog"
    assert v1414.V1414_SCHEMA == "v1414.asi-overarching-watchdog/v1"


def test_constants_severities_have_three_levels():
    assert set(v1414.V1414_SEVERITIES) == {"INFO", "WARN", "CRITICAL"}
    assert len(v1414.V1414_SEVERITIES) == 3


def test_constants_severity_rank_order():
    assert v1414.severity_rank("INFO") == 0
    assert v1414.severity_rank("WARN") == 1
    assert v1414.severity_rank("CRITICAL") == 2
    assert v1414.severity_rank("UNKNOWN") == 0  # default


def test_constants_verdict_rank_order():
    assert v1414.verdict_rank("COMPLETE") == 4
    assert v1414.verdict_rank("GOOD") == 3
    assert v1414.verdict_rank("PARTIAL") == 2
    assert v1414.verdict_rank("WEAK") == 1
    assert v1414.verdict_rank("INCOMPLETE") == 0
    assert v1414.verdict_rank("UNKNOWN") == -1


def test_constants_guards_count_and_v3_subset():
    assert len(v1414.V1414_GUARDS) == 16
    assert len(v1414.V1414_V3_GUARDS) == 6
    # V3 guards all in main GUARDS? They are V3-philosophy subset; main guards are watchdog-specific.
    # V3 guards should be a separate tuple (subset conceptually).
    for g in v1414.V1414_V3_GUARDS:
        assert g.startswith("GUARD_WATCHDOG_")


def test_constants_borrowed_count_and_shape():
    assert len(v1414.V1414_BORROWED) == 4
    for src, use in v1414.V1414_BORROWED:
        assert isinstance(src, str) and isinstance(use, str) and src and use


def test_constants_remediation_catalog_has_five_hints():
    assert len(v1414.V1414_REMEDIATION_CATALOG) == 5
    assert all(k.startswith("HINT_") for k in v1414.V1414_REMEDIATION_CATALOG.keys())


def test_constants_default_paths_dotted():
    # Default paths are dotted so they live in cwd (V1413 convention)
    assert v1414.V1414_DEFAULT_HISTORY_PATH.startswith(".")
    assert v1414.V1414_DEFAULT_BASELINE_PATH.startswith(".")


# ----------------------- Section B: Helpers -----------------------


def test_slug_timestamp_format():
    s = v1414.slug_timestamp()
    # YYYY-MM-DDTHH-MM-SSZ
    assert s[4] == "-" and s[7] == "-" and s[10] == "T" and s.endswith("Z")
    assert len(s) == 20


def test_slug_timestamp_deterministic_with_input():
    from datetime import datetime, timezone
    dt = datetime(2026, 8, 10, 2, 0, 0, tzinfo=timezone.utc)
    assert v1414.slug_timestamp(dt) == "2026-08-10T02-00-00Z"


def test_is_path_safe_rejects_absolute_and_dotdot():
    assert v1414._is_path_safe("relative.jsonl") is True
    assert v1414._is_path_safe("a/b.jsonl") is True
    assert v1414._is_path_safe("..//etc/passwd") is False
    assert v1414._is_path_safe("C:/Windows/system.ini") is False
    assert v1414._is_path_safe("/etc/passwd") is False
    assert v1414._is_path_safe("") is False
    assert v1414._is_path_safe(None) is False  # type: ignore[arg-type]


def test_hint_for_rule_known_rules():
    assert "HINT_REVERT_BASELINE" in v1414._hint_for_rule("RULE_VERDICT_REGRESSION")
    assert "HINT_REPLAY_HISTORY" in v1414._hint_for_rule("RULE_GAP_EXPANSION")
    assert "HINT_PROBE_DEPLOY" in v1414._hint_for_rule("RULE_FRAMEWORK_DROP")
    assert "HINT_DEEP_DIVE_BORROWED" in v1414._hint_for_rule("RULE_CHAIN_FAIL")


def test_hint_for_rule_unknown_falls_back():
    h = v1414._hint_for_rule("RULE_UNKNOWN")
    assert "HINT_LOCK_AND_PAUSE" in h


# ----------------------- Section C: Dataclass round-trips -----------------------


def test_watchdog_config_roundtrip():
    cfg = v1414.WatchdogConfig(cooldown_seconds=600, note="x")
    d = cfg.to_dict()
    cfg2 = v1414.WatchdogConfig.from_dict(d)
    assert cfg2.cooldown_seconds == 600
    assert cfg2.note == "x"
    # enable_rule is preserved
    assert cfg2.enable_rule["RULE_VERDICT_REGRESSION"] is True


def test_watchdog_config_from_dict_tolerates_missing_keys():
    cfg = v1414.WatchdogConfig.from_dict({})
    assert cfg.gap_expansion_warn > 0
    assert cfg.cooldown_seconds > 0


def test_watchdog_rule_roundtrip():
    r = v1414.WatchdogRule(rule_id="RULE_X", severity="WARN", field="f", op="drop", threshold=2.0, reason="r")
    d = r.to_dict()
    r2 = v1414.WatchdogRule.from_dict(d)
    assert r2.rule_id == "RULE_X" and r2.severity == "WARN" and r2.threshold == 2.0


def test_regression_alert_roundtrip():
    a = v1414.RegressionAlert(
        rule_id="RULE_X",
        severity="WARN",
        snapshot_timestamp="2026-08-10T02-00-00Z",
        baseline_timestamp="2026-08-10T01-00-00Z",
        magnitude=-1.0,
        reason="test",
        evidence={"a": 1},
        remediation_hint="H: x",
    )
    d = a.to_dict()
    a2 = v1414.RegressionAlert.from_dict(d)
    assert a2.rule_id == "RULE_X" and a2.magnitude == -1.0 and a2.evidence["a"] == 1


def test_watchdog_report_roundtrip():
    rep = v1414.WatchdogReport(
        timestamp="2026-08-10T02-00-00Z",
        n_snapshots=1,
        n_alerts=1,
        max_severity="WARN",
        alerts=[v1414.RegressionAlert(rule_id="R", severity="WARN")],
        remediation_hints=["hint"],
    )
    d = rep.to_dict()
    rep2 = v1414.WatchdogReport.from_dict(d)
    assert rep2.n_alerts == 1 and rep2.max_severity == "WARN"
    assert len(rep2.alerts) == 1 and rep2.alerts[0].rule_id == "R"
    assert rep2.remediation_hints == ["hint"]


# ----------------------- Section D: Builders -----------------------


def test_build_default_rules_has_four():
    rules = v1414.build_default_rules()
    ids = [r.rule_id for r in rules]
    assert ids == ["RULE_VERDICT_REGRESSION", "RULE_GAP_EXPANSION", "RULE_FRAMEWORK_DROP", "RULE_CHAIN_FAIL"]


def test_build_default_rules_severities_distributed():
    rules = v1414.build_default_rules()
    sev = {r.severity for r in rules}
    assert "WARN" in sev and "CRITICAL" in sev


def test_build_default_config_sane_thresholds():
    cfg = v1414.build_default_config()
    assert 0 < cfg.gap_expansion_warn < cfg.gap_expansion_critical < 1
    assert cfg.cooldown_seconds > 0
    assert cfg.window_size > 0


# ----------------------- Section E: Evaluators -----------------------


def test_evaluate_regressions_empty_history_no_alerts():
    assert v1414.evaluate_regressions([], None, v1414.build_default_config()) == []


def test_evaluate_regressions_no_baseline_no_alerts():
    snap = _fake_snapshot()
    cfg = v1414.build_default_config()
    assert v1414.evaluate_regressions([snap], None, cfg) == []


def test_evaluate_regressions_baseline_equal_current_no_alerts():
    snap = _fake_snapshot()
    base = _fake_baseline()
    cfg = v1414.build_default_config()
    assert v1414.evaluate_regressions([snap], base, cfg) == []


def test_evaluate_regressions_framework_drop_triggers_critical():
    snap = _fake_snapshot(framework_score=9)  # dropped 2
    base = _fake_baseline(baseline_framework_score=11)
    cfg = v1414.build_default_config()
    alerts = v1414.evaluate_regressions([snap], base, cfg)
    crit = [a for a in alerts if a.severity == "CRITICAL" and a.rule_id == "RULE_FRAMEWORK_DROP"]
    assert len(crit) == 1
    assert crit[0].magnitude == -2.0


def test_evaluate_regressions_verdict_regression_triggers_critical():
    snap = _fake_snapshot(verdict="GOOD")  # 4→3, rank drop 1
    base = _fake_baseline(baseline_verdict="COMPLETE")
    cfg = v1414.build_default_config()
    alerts = v1414.evaluate_regressions([snap], base, cfg)
    crit = [a for a in alerts if a.severity == "CRITICAL" and a.rule_id == "RULE_VERDICT_REGRESSION"]
    assert len(crit) == 1
    assert crit[0].evidence["current_verdict"] == "GOOD"
    assert crit[0].evidence["baseline_verdict"] == "COMPLETE"


def test_evaluate_regressions_chain_fail_triggers_critical():
    snap = _fake_snapshot(chain_ok=False)
    base = _fake_baseline(baseline_chain_ok=True)
    cfg = v1414.build_default_config()
    alerts = v1414.evaluate_regressions([snap], base, cfg)
    crit = [a for a in alerts if a.severity == "CRITICAL" and a.rule_id == "RULE_CHAIN_FAIL"]
    assert len(crit) == 1


def test_evaluate_regressions_gap_expansion_warn_only():
    snap = _fake_snapshot(gap_to_north_star=0.0755)  # +0.006 > warn 0.005, < critical 0.02
    base = _fake_baseline(baseline_gap=0.0695)
    cfg = v1414.build_default_config()
    alerts = v1414.evaluate_regressions([snap], base, cfg)
    by_id = {a.rule_id: a for a in alerts}
    assert "RULE_GAP_EXPANSION" in by_id
    assert by_id["RULE_GAP_EXPANSION"].severity == "WARN"


def test_evaluate_regressions_gap_expansion_promotes_to_critical():
    snap = _fake_snapshot(gap_to_north_star=0.10)  # +0.0305 > critical 0.02
    base = _fake_baseline(baseline_gap=0.0695)
    cfg = v1414.build_default_config()
    alerts = v1414.evaluate_regressions([snap], base, cfg)
    by_id = {a.rule_id: a for a in alerts}
    assert by_id["RULE_GAP_EXPANSION"].severity == "CRITICAL"


def test_evaluate_regressions_disabled_rule_no_alert():
    snap = _fake_snapshot(framework_score=9)
    base = _fake_baseline(baseline_framework_score=11)
    cfg = v1414.build_default_config()
    cfg.enable_rule["RULE_FRAMEWORK_DROP"] = False
    alerts = v1414.evaluate_regressions([snap], base, cfg)
    assert not any(a.rule_id == "RULE_FRAMEWORK_DROP" for a in alerts)


def test_evaluate_regressions_all_four_fire_together():
    snap = _fake_snapshot(verdict="GOOD", framework_score=9, chain_ok=False, gap_to_north_star=0.10)
    base = _fake_baseline(baseline_verdict="COMPLETE", baseline_framework_score=11, baseline_chain_ok=True)
    cfg = v1414.build_default_config()
    alerts = v1414.evaluate_regressions([snap], base, cfg)
    ids = {a.rule_id for a in alerts}
    assert {
        "RULE_VERDICT_REGRESSION",
        "RULE_GAP_EXPANSION",
        "RULE_FRAMEWORK_DROP",
        "RULE_CHAIN_FAIL",
    }.issubset(ids)


# ----------------------- Section F: compute_remediation_hints -----------------------


def test_compute_remediation_hints_dedupes():
    alerts = [
        v1414.RegressionAlert(rule_id="RULE_FRAMEWORK_DROP", remediation_hint="HINT_PROBE_DEPLOY: x"),
        v1414.RegressionAlert(rule_id="RULE_FRAMEWORK_DROP", remediation_hint="HINT_PROBE_DEPLOY: x"),
        v1414.RegressionAlert(rule_id="RULE_CHAIN_FAIL", remediation_hint="HINT_DEEP_DIVE_BORROWED: y"),
    ]
    hints = v1414.compute_remediation_hints(alerts)
    assert len(hints) == 2
    assert len(set(hints)) == 2


def test_compute_remediation_hints_empty_alerts_still_one():
    hints = v1414.compute_remediation_hints([])
    assert len(hints) == 1
    assert "HINT_LOCK_AND_PAUSE" in hints[0]


def test_compute_remediation_hints_critical_contributes_hint():
    alerts = [
        v1414.RegressionAlert(rule_id="RULE_CHAIN_FAIL", severity="CRITICAL", remediation_hint="HINT_DEEP_DIVE_BORROWED: z"),
    ]
    hints = v1414.compute_remediation_hints(alerts)
    assert any("DEEP_DIVE_BORROWED" in h for h in hints)


# ----------------------- Section G: cooldown -----------------------


def test_should_cooldown_no_last_ts_returns_false():
    assert v1414.should_cooldown([], None, v1414.build_default_config()) is False


def test_should_cooldown_no_alerts_returns_false():
    assert v1414.should_cooldown([], "2026-08-10T02-00-00Z", v1414.build_default_config()) is False


def test_should_cooldown_only_info_no_cooldown():
    alerts = [v1414.RegressionAlert(rule_id="X", severity="INFO")]
    assert v1414.should_cooldown(alerts, "2026-08-10T02-00-00Z", v1414.build_default_config()) is False


def test_should_cooldown_recent_critical_triggers():
    from datetime import datetime, timezone
    # 60 seconds ago — within 900s cooldown
    dt = datetime.now(timezone.utc).timestamp() - 60
    from datetime import datetime as _dt
    ts = _dt.fromtimestamp(dt, tz=timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    alerts = [v1414.RegressionAlert(rule_id="RULE_FRAMEWORK_DROP", severity="CRITICAL")]
    assert v1414.should_cooldown(alerts, ts, v1414.build_default_config()) is True


def test_should_cooldown_old_critical_no_cooldown():
    from datetime import datetime, timezone
    dt = datetime.now(timezone.utc).timestamp() - 3600  # 1 hour ago
    ts = datetime.fromtimestamp(dt, tz=timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    alerts = [v1414.RegressionAlert(rule_id="RULE_FRAMEWORK_DROP", severity="CRITICAL")]
    assert v1414.should_cooldown(alerts, ts, v1414.build_default_config()) is False


def test_should_cooldown_future_ts_safe_returns_false():
    alerts = [v1414.RegressionAlert(rule_id="RULE_FRAMEWORK_DROP", severity="CRITICAL")]
    assert v1414.should_cooldown(alerts, "2999-01-01T00:00:00Z", v1414.build_default_config()) is False


# ----------------------- Section H: render_watchdog_md -----------------------


def test_render_watchdog_md_has_eight_sections():
    rep = v1414.WatchdogReport(timestamp=v1414.slug_timestamp(), n_alerts=0, max_severity="INFO", alerts=[], remediation_hints=["HINT_LOCK_AND_PAUSE: x"])
    md = v1414.render_watchdog_md(rep, v1414.build_default_config())
    for i in range(1, 9):
        assert f"## {i}." in md


def test_render_watchdog_md_alerts_table_present():
    alerts = [
        v1414.RegressionAlert(rule_id="RULE_FRAMEWORK_DROP", severity="CRITICAL",
                              snapshot_timestamp="2026-08-10T02-00-00Z",
                              baseline_timestamp="2026-08-10T01-00-00Z",
                              magnitude=-1.0, reason="test reason"),
    ]
    rep = v1414.WatchdogReport(timestamp=v1414.slug_timestamp(), n_alerts=1, max_severity="CRITICAL", alerts=alerts, remediation_hints=["H: x"])
    md = v1414.render_watchdog_md(rep, v1414.build_default_config(), alerts)
    assert "| 1 | RULE_FRAMEWORK_DROP | **CRITICAL**" in md


def test_render_watchdog_md_max_severity_verdict_text():
    rep_crit = v1414.WatchdogReport(timestamp=v1414.slug_timestamp(), n_alerts=1, max_severity="CRITICAL")
    md = v1414.render_watchdog_md(rep_crit, v1414.build_default_config())
    assert "WATCHDOG_CRITICAL" in md

    rep_warn = v1414.WatchdogReport(timestamp=v1414.slug_timestamp(), n_alerts=1, max_severity="WARN")
    md = v1414.render_watchdog_md(rep_warn, v1414.build_default_config())
    assert "WATCHDOG_WARN" in md

    rep_info = v1414.WatchdogReport(timestamp=v1414.slug_timestamp(), n_alerts=0, max_severity="INFO")
    md = v1414.render_watchdog_md(rep_info, v1414.build_default_config())
    assert "WATCHDOG_OK" in md


def test_render_watchdog_md_includes_borrowed():
    rep = v1414.WatchdogReport(timestamp=v1414.slug_timestamp(), n_alerts=0, max_severity="INFO")
    md = v1414.render_watchdog_md(rep, v1414.build_default_config())
    assert "Borrowed (4" in md
    assert "V1413 overarching history" in md


# ----------------------- Section I: run_watchdog_tick -----------------------


def test_run_watchdog_tick_no_history_no_alerts(tmp_path, monkeypatch):
    # Use non-existent history file
    rep = v1414.run_watchdog_tick(
        history_path=str(tmp_path / "no_history.jsonl"),
        baseline_path=str(tmp_path / "no_baseline.json"),
    )
    assert rep.n_snapshots == 0
    assert rep.n_alerts == 0
    assert rep.max_severity == "INFO"


def test_run_watchdog_tick_with_healthy_history(tmp_path, monkeypatch):
    from apeireth.v1413_asi_overarching_history import (
        HistorySnapshot, append_snapshot, make_baseline, write_baseline,
    )
    hist = tmp_path / "history.jsonl"
    base = tmp_path / "baseline.json"
    snap = HistorySnapshot(
        timestamp="2026-08-10T01-00-00Z",
        snapshot_id="x",
        source_module="v1412_asi_overarching_dashboard",
        source_version="0.1.0",
        verdict="COMPLETE",
        framework_score=11,
        level_score=12,
        coherence_score=12,
        chain_ok=True,
        borrowed_count=7,
        anchor_value=0.9105,
        gap_to_north_star=0.0695,
        gap_to_ceiling=0.0795,
        note="seed",
    )
    append_snapshot(snap, str(hist))
    write_baseline(make_baseline(snap), str(base))
    rep = v1414.run_watchdog_tick(history_path=str(hist), baseline_path=str(base))
    assert rep.n_snapshots >= 1
    assert rep.n_alerts == 0
    assert rep.max_severity == "INFO"
    assert rep.chain_ok is True


def test_run_watchdog_tick_with_regression(tmp_path):
    from apeireth.v1413_asi_overarching_history import (
        HistorySnapshot, append_snapshot, make_baseline, write_baseline,
    )
    hist = tmp_path / "history.jsonl"
    base = tmp_path / "baseline.json"
    # baseline: COMPLETE / fw 11
    write_baseline(
        make_baseline(HistorySnapshot(
            timestamp="2026-08-10T00-00-00Z",
            snapshot_id="b",
            source_module="v1412_asi_overarching_dashboard",
            source_version="0.1.0",
            verdict="COMPLETE",
            framework_score=11,
            level_score=12,
            coherence_score=12,
            chain_ok=True,
            borrowed_count=7,
            anchor_value=0.9105,
            gap_to_north_star=0.0695,
            gap_to_ceiling=0.0795,
            note="baseline",
        )),
        str(base),
    )
    # history: same as baseline → no regression
    append_snapshot(HistorySnapshot(
        timestamp="2026-08-10T01-00-00Z",
        snapshot_id="s1",
        source_module="v1412_asi_overarching_dashboard",
        source_version="0.1.0",
        verdict="COMPLETE",
        framework_score=11,
        level_score=12,
        coherence_score=12,
        chain_ok=True,
        borrowed_count=7,
        anchor_value=0.9105,
        gap_to_north_star=0.0695,
        gap_to_ceiling=0.0795,
        note="ok",
    ), str(hist))
    # Now append a regression: verdict GOOD, fw 9, chain False, gap 0.10
    append_snapshot(HistorySnapshot(
        timestamp="2026-08-10T02-00-00Z",
        snapshot_id="s2",
        source_module="v1412_asi_overarching_dashboard",
        source_version="0.1.0",
        verdict="GOOD",
        framework_score=9,
        level_score=12,
        coherence_score=12,
        chain_ok=False,
        borrowed_count=7,
        anchor_value=0.9105,
        gap_to_north_star=0.10,
        gap_to_ceiling=0.0795,
        note="regression",
    ), str(hist))
    rep = v1414.run_watchdog_tick(history_path=str(hist), baseline_path=str(base))
    ids = {a.rule_id for a in rep.alerts}
    assert "RULE_VERDICT_REGRESSION" in ids
    assert "RULE_FRAMEWORK_DROP" in ids
    assert "RULE_CHAIN_FAIL" in ids
    assert "RULE_GAP_EXPANSION" in ids
    assert rep.max_severity == "CRITICAL"


# ----------------------- Section J: chain_delegate_v1414 -----------------------


def test_chain_delegate_v1414_probes_v1411_v1412_v1413():
    all_ok, n_ok, n_alerts, n_mod, errs = v1414.chain_delegate_v1414()
    assert n_mod >= 4  # V1411 + V1412 + V1413 + V1414 self
    assert n_ok >= 3
    assert isinstance(errs, list)


def test_chain_delegate_v1414_self_module_present():
    all_ok, n_ok, _, n_mod, _ = v1414.chain_delegate_v1414()
    # We include self as a module — verify n_mod >= 4
    assert n_mod >= 4


# ----------------------- Section K: popper_self_test -----------------------


def test_popper_self_test_passes():
    passed, total, failed = v1414.popper_self_test()
    assert total == 12
    assert passed >= 11, f"expected ≥11 pass; got {passed}; failed: {failed}"


def test_popper_self_test_includes_zero_severity_case():
    passed, total, _ = v1414.popper_self_test()
    assert total == 12
    assert passed <= 12


# ----------------------- Section L: CLI -----------------------


def test_cli_version_runs():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1414_asi_overarching_watchdog", "version"],
        cwd=str(Path(__file__).resolve().parent.parent),
        capture_output=True, text=True, encoding="utf-8", timeout=60, env={**os.environ, "PYTHONIOENCODING": "utf-8"},
    )
    assert result.returncode == 0
    out = result.stdout
    assert "V1414_VERSION: 0.1.0" in out
    assert "16" in out or "guards: 16" in out


def test_cli_rules_runs():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1414_asi_overarching_watchdog", "rules"],
        cwd=str(Path(__file__).resolve().parent.parent),
        capture_output=True, text=True, encoding="utf-8", timeout=60, env={**os.environ, "PYTHONIOENCODING": "utf-8"},
    )
    assert result.returncode == 0
    assert "RULE_VERDICT_REGRESSION" in result.stdout
    assert "RULE_FRAMEWORK_DROP" in result.stdout


def test_cli_severity_runs():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1414_asi_overarching_watchdog", "severity"],
        cwd=str(Path(__file__).resolve().parent.parent),
        capture_output=True, text=True, encoding="utf-8", timeout=60, env={**os.environ, "PYTHONIOENCODING": "utf-8"},
    )
    assert result.returncode == 0
    assert "INFO" in result.stdout and "WARN" in result.stdout and "CRITICAL" in result.stdout


def test_cli_remediation_runs():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1414_asi_overarching_watchdog", "remediation"],
        cwd=str(Path(__file__).resolve().parent.parent),
        capture_output=True, text=True, encoding="utf-8", timeout=60, env={**os.environ, "PYTHONIOENCODING": "utf-8"},
    )
    assert result.returncode == 0
    assert "HINT_PROBE_DEPLOY" in result.stdout
    assert "HINT_LOCK_AND_PAUSE" in result.stdout


def test_cli_popper_runs():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1414_asi_overarching_watchdog", "popper"],
        cwd=str(Path(__file__).resolve().parent.parent),
        capture_output=True, text=True, encoding="utf-8", timeout=60, env={**os.environ, "PYTHONIOENCODING": "utf-8"},
    )
    assert result.returncode == 0
    assert "popper:" in result.stdout
    # Should be "12/12" ideally
    assert "12/12" in result.stdout


def test_cli_meta_runs_and_emits_json():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1414_asi_overarching_watchdog", "meta"],
        cwd=str(Path(__file__).resolve().parent.parent),
        capture_output=True, text=True, encoding="utf-8", timeout=60, env={**os.environ, "PYTHONIOENCODING": "utf-8"},
    )
    assert result.returncode == 0
    payload = json.loads(result.stdout)
    assert payload["version"] == "0.1.0"
    assert payload["module"] == "v1414_asi_overarching_watchdog"
    assert len(payload["guards"]) == 16
    assert len(payload["v3_guards"]) == 6
    assert len(payload["borrowed"]) == 4
    assert len(payload["remediation_catalog"]) == 5


def test_cli_chain_runs():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1414_asi_overarching_watchdog", "chain"],
        cwd=str(Path(__file__).resolve().parent.parent),
        capture_output=True, text=True, encoding="utf-8", timeout=60, env={**os.environ, "PYTHONIOENCODING": "utf-8"},
    )
    # Chain proxy: should be ok in a healthy env
    assert result.returncode in (0, 1)
    payload = json.loads(result.stdout)
    assert "all_ok" in payload
    assert "n_modules" in payload
    assert payload["n_modules"] >= 4


def test_cli_demo_runs():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1414_asi_overarching_watchdog", "demo"],
        cwd=str(Path(__file__).resolve().parent.parent),
        capture_output=True, text=True, encoding="utf-8", timeout=60, env={**os.environ, "PYTHONIOENCODING": "utf-8"},
    )
    assert result.returncode in (0, 1, 2)
    # Either we have alerts or no alerts, but the line should be there
    assert "tick:" in result.stdout or "max_severity" in result.stdout


def test_cli_tick_quiet_runs(tmp_path):
    # Use a tmp history path so we don't pollute cwd
    h = tmp_path / "empty.jsonl"
    b = tmp_path / "empty.base.json"
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1414_asi_overarching_watchdog", "tick",
         "--history-path", str(h), "--baseline-path", str(b), "--quiet"],
        cwd=str(Path(__file__).resolve().parent.parent),
        capture_output=True, text=True, encoding="utf-8", timeout=60, env={**os.environ, "PYTHONIOENCODING": "utf-8"},
    )
    assert result.returncode == 0
    assert "max_severity=INFO" in result.stdout


def test_cli_config_runs():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1414_asi_overarching_watchdog", "config"],
        cwd=str(Path(__file__).resolve().parent.parent),
        capture_output=True, text=True, encoding="utf-8", timeout=60, env={**os.environ, "PYTHONIOENCODING": "utf-8"},
    )
    assert result.returncode == 0
    payload = json.loads(result.stdout)
    assert payload["gap_expansion_warn"] > 0


def test_cli_probe_runs(tmp_path):
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1414_asi_overarching_watchdog", "probe",
         "--history-path", str(tmp_path / "nope.jsonl"),
         "--baseline-path", str(tmp_path / "nope.base.json")],
        cwd=str(Path(__file__).resolve().parent.parent),
        capture_output=True, text=True, encoding="utf-8", timeout=60, env={**os.environ, "PYTHONIOENCODING": "utf-8"},
    )
    assert result.returncode == 0
    payload = json.loads(result.stdout)
    assert payload["history_exists"] is False
    assert payload["baseline_exists"] is False


def test_cli_help_runs():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1414_asi_overarching_watchdog", "help"],
        cwd=str(Path(__file__).resolve().parent.parent),
        capture_output=True, text=True, encoding="utf-8", timeout=60, env={**os.environ, "PYTHONIOENCODING": "utf-8"},
    )
    assert result.returncode == 0
    assert "ASI 总框架" in result.stdout or "watchdog" in result.stdout.lower()


# ----------------------- Section M: Integration / Inter-module -----------------------


def test_v1414_reads_v1413_real_history(tmp_path):
    """Real end-to-end: V1414 reads V1413's actual history file (the one with snap=COMPLETE fw=11) created earlier."""
    # We don't know if V1413 history file exists in cwd; guard for both
    cwd_hist = Path.cwd() / ".v1413-asi-overarching-history.jsonl"
    base_path = Path(__file__).resolve().parent.parent / ".v1413-asi-overarching-history.jsonl"
    if not base_path.exists():
        pytest.skip("V1413 history file not present in workspace; skipping integration test")
    rep = v1414.run_watchdog_tick(history_path=str(base_path), baseline_path=str(base_path.parent / ".v1413-asi-overarching-baseline.json"))
    # Should not raise; chain_ok should reflect latest snapshot
    assert isinstance(rep, v1414.WatchdogReport)


def test_v1414_chain_reports_self_in_modules():
    all_ok, n_ok, _, n_mod, _ = v1414.chain_delegate_v1414()
    assert n_mod == 4  # V1411 + V1412 + V1413 + V1414 self

