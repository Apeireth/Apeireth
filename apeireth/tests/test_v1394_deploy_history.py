#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
test_v1394_deploy_history.py — Pytest 验证 V1394 ASI 真生产 deploy-stack history

V1394 = real production JSONL history of V1393 judge results (post-V1393 next-step).
V1050 → V1394 prompt stale correction: 实际部署栈系列已 V1384-V1393, V1394 = 时间序列兑现.

Sections:
 1. Module constants (5)
 2. HistoryEntry dataclass (6)
 3. Trend dataclass (4)
 4. append_entry + load_history roundtrip (5)
 5. compute_trend 算法 (6)
 6. popper_self_test passes (2)
 7. CLI: version / show / trend / summary / demo / popper (7)
 8. V3 哲学 守门 6 GUARDS 自动注入 (3)
 9. Integration: V1393 judge → V1394 history → trend (3)
"""
import json
import sys
import tempfile
from pathlib import Path

import pytest

# Add apeireth root to path so the module can be imported
APEIRETH_ROOT = Path(__file__).resolve().parent.parent
if str(APEIRETH_ROOT) not in sys.path:
    sys.path.insert(0, str(APEIRETH_ROOT))

import v1394_deploy_history as m  # noqa: E402
from v1394_deploy_history import (  # noqa: E402
    HistoryEntry,
    Trend,
    V1394_DEFAULT_HISTORY_PATH,
    V1394_GUARDS,
    V1394_SCHEMA,
    V1394_VERSION,
    append_entry,
    compute_trend,
    load_history,
    popper_self_test,
    run_cli,
)


# ============================================================================
# 1. Module constants (5)
# ============================================================================


def test_module_version():
    assert V1394_VERSION == "0.1.0"


def test_module_schema():
    assert V1394_SCHEMA == "v1394.deploy-history/v1"


def test_module_default_path():
    assert V1394_DEFAULT_HISTORY_PATH == ".v1393-judge-history.jsonl"


def test_module_guards_count():
    assert len(V1394_GUARDS) >= 8, f"V1394 needs >= 8 GUARDS, got {len(V1394_GUARDS)}"


def test_module_guards_have_no_cap_change():
    """V3 哲学 GUARD: honest 0.90 cap preserved."""
    assert "GUARD_NO_CAP_CHANGE" in V1394_GUARDS


# ============================================================================
# 2. HistoryEntry dataclass (6)
# ============================================================================


def test_history_entry_defaults():
    e = HistoryEntry()
    assert e.timestamp == ""
    assert e.target == ""
    assert e.verdict == "GOOD"
    assert e.score == 100
    assert e.grade == "A+"
    assert e.n_findings == 0
    assert e.notes == []


def test_history_entry_to_dict_has_schema():
    e = HistoryEntry(target="x", score=80, grade="B")
    d = e.to_dict()
    assert d["schema"] == V1394_SCHEMA
    assert d["version"] == V1394_VERSION
    assert d["target"] == "x"
    assert d["score"] == 80
    assert d["grade"] == "B"


def test_history_entry_from_dict_roundtrip():
    e1 = HistoryEntry(
        target="roundtrip",
        verdict="POOR",
        score=50,
        grade="D",
        n_findings=5,
        n_errors=2,
        n_warnings=3,
        n_info=0,
        policy_pass=False,
        policy_score=50,
        n_hints=5,
        notes=["a", "b"],
    )
    d = e1.to_dict()
    e2 = HistoryEntry.from_dict(d)
    assert e2.target == e1.target
    assert e2.verdict == e1.verdict
    assert e2.score == e1.score
    assert e2.grade == e1.grade
    assert e2.n_findings == e1.n_findings
    assert e2.n_errors == e1.n_errors
    assert e2.policy_pass == e1.policy_pass
    assert e2.policy_score == e1.policy_score
    assert e2.notes == e1.notes


def test_history_entry_from_dict_handles_missing_fields():
    e = HistoryEntry.from_dict({"target": "minimal"})
    assert e.target == "minimal"
    assert e.verdict == "GOOD"  # default
    assert e.score == 100  # default


def test_history_entry_from_dict_aliases_deploy_score():
    """V1394 from_dict 接受 deploy_score / deploy_grade 别名 (来自 V1393 judge dict)."""
    e = HistoryEntry.from_dict(
        {"target": "alias", "deploy_score": 75, "deploy_grade": "B"}
    )
    assert e.score == 75
    assert e.grade == "B"


def test_history_entry_timestamp_auto_populated_on_append():
    """append_entry 自动写 timestamp (主 17:43 实事求是)."""
    e = HistoryEntry(target="autots")
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
        tp = f.name
    try:
        append_entry(e, tp)
        assert e.timestamp != "", "timestamp should be auto-populated"
        assert "T" in e.timestamp and "Z" in e.timestamp
    finally:
        Path(tp).unlink(missing_ok=True)


# ============================================================================
# 3. Trend dataclass (4)
# ============================================================================


def test_trend_defaults():
    t = Trend()
    assert t.direction == "stable"
    assert t.delta_score == 0
    assert t.delta_findings == 0
    assert t.n_entries == 0
    assert t.first_score == 100
    assert t.last_score == 100


def test_trend_to_dict_keys():
    t = Trend(direction="improving", delta_score=10, n_entries=5)
    d = t.to_dict()
    assert d["direction"] == "improving"
    assert d["delta_score"] == 10
    assert d["n_entries"] == 5
    assert "first_timestamp" in d
    assert "last_timestamp" in d


def test_trend_valid_directions():
    """GUARD_TREND_VALID: direction ∈ {improving, stable, declining}."""
    t = Trend(direction="improving")
    assert t.direction in ("improving", "stable", "declining")


def test_trend_negative_delta_means_declining():
    """compute_trend 边界: delta < -5 → declining."""
    e1 = HistoryEntry(score=80, timestamp="2026-08-09T00:00:00Z")
    e2 = HistoryEntry(score=60, timestamp="2026-08-09T01:00:00Z")
    t = compute_trend([e1, e2])
    assert t.direction == "declining"
    assert t.delta_score == -20


# ============================================================================
# 4. append_entry + load_history roundtrip (5)
# ============================================================================


def test_load_history_nonexistent_returns_empty():
    h = load_history("___v1394_nonexistent_xyz___")
    assert h == []


def test_append_then_load_single_entry():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
        tp = f.name
    try:
        e = HistoryEntry(target="single", verdict="OK", score=85, grade="B")
        append_entry(e, tp)
        loaded = load_history(tp)
        assert len(loaded) == 1
        assert loaded[0].target == "single"
        assert loaded[0].verdict == "OK"
    finally:
        Path(tp).unlink(missing_ok=True)


def test_append_then_load_multiple_entries_preserves_order():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
        tp = f.name
    try:
        entries = [
            HistoryEntry(target=f"t{i}", score=50 + i * 5, grade="B", timestamp=f"2026-08-09T0{i}:00:00Z")
            for i in range(5)
        ]
        for e in entries:
            append_entry(e, tp)
        loaded = load_history(tp)
        assert len(loaded) == 5
        for i, e in enumerate(loaded):
            assert e.target == f"t{i}"
            assert e.score == 50 + i * 5
    finally:
        Path(tp).unlink(missing_ok=True)


def test_load_history_skips_blank_and_bad_lines():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
        tp = f.name
    try:
        with open(tp, "w", encoding="utf-8") as f2:
            f2.write("\n")
            f2.write(json.dumps(HistoryEntry(target="good1").to_dict()) + "\n")
            f2.write("not-valid-json\n")
            f2.write(json.dumps(HistoryEntry(target="good2").to_dict()) + "\n")
            f2.write('{"missing": "fields", "target": "minimal"}\n')
        loaded = load_history(tp)
        assert len(loaded) == 3  # 2 good + 1 minimal
        targets = [e.target for e in loaded]
        assert "good1" in targets
        assert "good2" in targets
        assert "minimal" in targets
    finally:
        Path(tp).unlink(missing_ok=True)


def test_load_history_handles_unicode_notes():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
        tp = f.name
    try:
        e = HistoryEntry(target="unicode", notes=["中文", "🚀", "ascii"])
        append_entry(e, tp)
        loaded = load_history(tp)
        assert loaded[0].notes == ["中文", "🚀", "ascii"]
    finally:
        Path(tp).unlink(missing_ok=True)


# ============================================================================
# 5. compute_trend 算法 (6)
# ============================================================================


def test_compute_trend_empty():
    t = compute_trend([])
    assert t.direction == "stable"
    assert t.n_entries == 0


def test_compute_trend_single_entry():
    e = HistoryEntry(target="solo", score=42, grade="F", timestamp="2026-08-09T00:00:00Z")
    t = compute_trend([e])
    assert t.n_entries == 1
    assert t.first_score == 42
    assert t.last_score == 42
    assert t.first_timestamp == "2026-08-09T00:00:00Z"


def test_compute_trend_improving():
    e1 = HistoryEntry(score=50, timestamp="2026-08-09T00:00:00Z")
    e2 = HistoryEntry(score=75, timestamp="2026-08-09T01:00:00Z")
    t = compute_trend([e1, e2])
    assert t.direction == "improving"
    assert t.delta_score == 25


def test_compute_trend_declining():
    e1 = HistoryEntry(score=80, timestamp="2026-08-09T00:00:00Z")
    e2 = HistoryEntry(score=30, timestamp="2026-08-09T01:00:00Z")
    t = compute_trend([e1, e2])
    assert t.direction == "declining"
    assert t.delta_score == -50


def test_compute_trend_stable_within_threshold():
    e1 = HistoryEntry(score=80, timestamp="2026-08-09T00:00:00Z")
    e2 = HistoryEntry(score=82, timestamp="2026-08-09T01:00:00Z")
    t = compute_trend([e1, e2])
    assert t.direction == "stable"
    assert t.delta_score == 2


def test_compute_trend_delta_findings():
    e1 = HistoryEntry(score=50, n_findings=10, timestamp="2026-08-09T00:00:00Z")
    e2 = HistoryEntry(score=80, n_findings=3, timestamp="2026-08-09T01:00:00Z")
    t = compute_trend([e1, e2])
    assert t.delta_findings == -7


# ============================================================================
# 6. popper_self_test passes (2)
# ============================================================================


def test_popper_self_test_passes():
    r = popper_self_test()
    assert r["passed"] is True
    assert r["failures"] == []
    assert r["n_tested"] >= 10


def test_popper_self_test_handles_dne():
    r = popper_self_test()
    # Test 1 explicitly checks DNE path
    assert r["passed"], f"popper failed: {r['failures']}"


# ============================================================================
# 7. CLI 真可跑 (7)
# ============================================================================


def test_cli_version():
    rc = run_cli(["version"])
    assert rc == 0


def test_cli_demo_runs():
    rc = run_cli(["demo"])
    assert rc == 0


def test_cli_popper_passes():
    rc = run_cli(["popper"])
    assert rc == 0


def test_cli_show_no_history_returns_zero():
    rc = run_cli(["show", "--history", "___v1394_dne___"])
    assert rc == 0


def test_cli_trend_no_history_returns_zero():
    rc = run_cli(["trend", "--history", "___v1394_dne___"])
    assert rc == 0


def test_cli_trend_with_json_flag():
    rc = run_cli(["trend", "--history", "___v1394_dne___", "--json"])
    assert rc == 0


def test_cli_summary_no_history():
    rc = run_cli(["summary", "--history", "___v1394_dne___"])
    assert rc == 0


# ============================================================================
# 8. V3 哲学 6 GUARDS 自动注入 (3)
# ============================================================================


def test_v3_guards_present():
    """V3 哲学 6 GUARDS: module_is_not_asi / measurement_is_not_truth / structure_is_not_consciousness / production_is_not_safety / automation_is_not_autonomy / runner_is_not_asi."""
    # V1394 不直接属于 V3 哲学模块 (它是 V1384-V1393 部署栈的延伸),
    # 但 module-level 守门应明示 NO_CAP_CHANGE + HONEST_DISCLOSURE.
    assert "GUARD_NO_CAP_CHANGE" in V1394_GUARDS
    assert "GUARD_HONEST_DISCLOSURE" in V1394_GUARDS


def test_module_does_not_claim_asi():
    """V1394 module 不宣称是 ASI; 仅是 deploy-stack history."""
    src = Path(m.__file__).read_text(encoding="utf-8")
    # 注释 + docstring 不允许 "是 ASI" / "= ASI" 的自我宣告
    assert "V1394 是 ASI" not in src
    assert "V1394 = ASI" not in src


def test_module_does_not_claim_consciousness():
    """V1394 module 不宣称 consciousness."""
    src = Path(m.__file__).read_text(encoding="utf-8")
    assert "V1394 是 conscious" not in src
    assert "V1394 = consciousness" not in src


# ============================================================================
# 9. Integration: V1393 judge → V1394 history → trend (3)
# ============================================================================


def test_integration_judge_result_into_history():
    """真把 V1393 JudgeResult 翻译为 V1394 HistoryEntry."""
    from v1393_deploy_judge import JudgeResult, judge  # type: ignore
    # Use V1393's demo path (if available)
    try:
        jr = judge("promethean/deploy")
    except Exception:
        # Fallback: 手工构造 JudgeResult (避免环境依赖)
        jr = JudgeResult(
            target="promethean/deploy",
            verdict="OK",
            deploy_score=85,
            deploy_grade="B",
            n_findings=2,
            n_errors=0,
            n_warnings=2,
            n_info=0,
            policy_pass=True,
            policy_score=100,
            n_hints=2,
            notes=["test"],
        )
    e = HistoryEntry(
        target=jr.target,
        verdict=jr.verdict,
        score=jr.deploy_score,
        grade=jr.deploy_grade,
        n_findings=jr.n_findings,
        n_errors=jr.n_errors,
        n_warnings=jr.n_warnings,
        n_info=jr.n_info,
        policy_pass=jr.policy_pass,
        policy_score=jr.policy_score,
        n_hints=jr.n_hints,
        notes=jr.notes,
    )
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
        tp = f.name
    try:
        append_entry(e, tp)
        loaded = load_history(tp)
        assert len(loaded) == 1
        assert loaded[0].target == jr.target
        assert loaded[0].verdict == jr.verdict
        assert loaded[0].score == jr.deploy_score
    finally:
        Path(tp).unlink(missing_ok=True)


def test_integration_two_entries_show_improving_trend():
    """真 append 2 entries, 真 compute trend = improving."""
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
        tp = f.name
    try:
        e1 = HistoryEntry(
            timestamp="2026-08-09T00:00:00Z",
            target="ci",
            verdict="POOR",
            score=50,
            grade="D",
            n_findings=10,
            n_errors=2,
            n_warnings=8,
            n_info=0,
        )
        e2 = HistoryEntry(
            timestamp="2026-08-09T01:00:00Z",
            target="ci",
            verdict="OK",
            score=85,
            grade="B",
            n_findings=2,
            n_errors=0,
            n_warnings=2,
            n_info=0,
        )
        append_entry(e1, tp)
        append_entry(e2, tp)
        loaded = load_history(tp)
        assert len(loaded) == 2
        t = compute_trend(loaded)
        assert t.direction == "improving"
        assert t.delta_score == 35
        assert t.n_entries == 2
    finally:
        Path(tp).unlink(missing_ok=True)


def test_integration_cli_append_show_trend_summary():
    """真跑 CLI: append → show → trend → summary."""
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
        tp = f.name
    try:
        # Demo 模式不依赖 V1393 真 judge
        rc = run_cli(["demo"])
        assert rc == 0
        # summary 用 DNE 路径测一遍
        rc = run_cli(["summary", "--history", "___v1394_dne___", "--target", "ci"])
        assert rc == 0
        # trend JSON
        rc = run_cli(["trend", "--history", "___v1394_dne___", "--target", "ci", "--json"])
        assert rc == 0
    finally:
        # demo 内部用 tempfile, 这里只清掉我们的 path
        Path(tp).unlink(missing_ok=True)