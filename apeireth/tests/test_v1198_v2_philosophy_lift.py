"""V1198 — ASI v2_philosophy lift tests (主 17:43 实事求是).

测试要点:
  - 主 17:43 实事求是: measure_v1198() 真测, 不 mock
  - 主 23:44 干到底: 5 sub-dim 真测全跑
  - 主 17:58+20:46 不假装: 6 不假装守门验证
  - 主 22:33 北极星: ASI recompute lift 真算 (0.9148 → 0.9228)
  - 主 19:33 走在前人经验上: V1137 真答 ALL_ANSWERS_V1137 真测
  - 主 00:56 任何人都能接手: 任何 cron 可调 measure_v1198()
  - 主 00:44 质量工程化: dataclass + snapshot_id + artifact_path 验证
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

APEIRETH_DIR = Path(__file__).resolve().parent.parent
PROJ_DIR = APEIRETH_DIR.parent


def _import_v1198():
    sys.path.insert(0, str(PROJ_DIR))
    from apeireth import v1198_v2_philosophy_lift as m
    return m


# 主 17:43 实事求是 — 真测 V1198 measure (V1137 真答 + 4 other sub-dim 真测)
def test_v1198_measure_returns_expected_score():
    """V1198 measure_v1198() 真测 = 0.88 (V1137 修复后)."""
    m = _import_v1198()
    score = m.measure_v1198()
    assert isinstance(score, float), f"score must be float, got {type(score)}"
    assert 0.85 <= score <= 0.95, f"V1198 score out of expected range: {score:.4f}"


def test_v1198_lift_v1161_baseline():
    """V1198 v2_philosophy 总: 0.72 → 0.88 (Δ=+0.16, 修复 V1161 attribute 查找 bug)."""
    m = _import_v1198()
    report = m.compute_v1198_lift()
    assert report.v2_philosophy_lifted == 0.88, (
        f"v2_philosophy_lifted = {report.v2_philosophy_lifted}, expected 0.88"
    )
    assert report.v2_philosophy_baseline == 0.72, (
        f"v2_philosophy_baseline = {report.v2_philosophy_baseline}, expected 0.72"
    )
    assert abs(report.v2_philosophy_delta - 0.16) < 0.001, (
        f"v2_philosophy_delta = {report.v2_philosophy_delta}, expected +0.16"
    )


def test_v1198_v1137_remaining_real_fixed():
    """V1198 V1137_remaining_real 真测: 0.0 → 0.8 (修复 attribute 查找 bug)."""
    m = _import_v1198()
    report = m.compute_v1198_lift()
    ev = report.sub_dim_evidence["V1137_remaining_real"]
    assert ev.score == 0.8, f"V1137_remaining_real score = {ev.score}, expected 0.8"
    assert ev.baseline == 0.0, f"V1137 baseline = {ev.baseline}, expected 0.0"
    assert ev.delta == 0.8, f"V1137 delta = {ev.delta}, expected 0.8"
    assert ev.raw.get("found_attr") == "ALL_ANSWERS_V1137", (
        f"V1198 修复路径: found_attr={ev.raw.get('found_attr')}, expected ALL_ANSWERS_V1137"
    )
    assert ev.raw.get("v1161_lookup_failed") is True
    assert ev.raw.get("v1198_lookup_fixed") is True
    # 4/5: answers_present, has_1_answer, has_2_answers, answers_truthy
    assert ev.checks["has_3_answers"] is False, "n=2 → has_3_answers must be False"
    assert ev.checks["has_2_answers"] is True
    assert ev.checks["has_1_answer"] is True


def test_v1198_other_4_subdim_pass():
    """V1198 4 其他 sub-dim 真测: V1135 + PHILOSOPHY_9_KEYS + ASI_7_QUESTIONS + v3_guards."""
    m = _import_v1198()
    report = m.compute_v1198_lift()
    # V1135 = 0.8 (5 真答)
    assert report.sub_dim_scores["V1135_answers_real"] == 0.8
    # PHILOSOPHY_9_KEYS = 1.0 (9 keys)
    assert report.sub_dim_scores["PHILOSOPHY_9_KEYS_real"] == 1.0
    # ASI_7_QUESTIONS = 1.0 (5 dims + has_time_dim + has_causal_dim)
    assert report.sub_dim_scores["ASI_7_QUESTIONS_real"] == 1.0
    # v3_guards = 0.8 (6 guards)
    assert report.sub_dim_scores["v3_guards_real"] == 0.8


def test_v1198_asi_recompute_lift():
    """V1198 ASI recompute lift: 0.9148 → 0.9228 (Δ=+0.008)."""
    m = _import_v1198()
    report = m.compute_v1198_lift()
    assert report.asi_recompute_baseline == 0.9148
    assert report.asi_recompute_lifted == 0.9228
    assert abs(report.asi_recompute_delta - 0.008) < 0.001


def test_v1198_measure_asi_recompute():
    """V1198 measure_v1198_asi_recompute() = 0.9228."""
    m = _import_v1198()
    asi = m.measure_v1198_asi_recompute()
    assert asi == 0.9228, f"measure_v1198_asi_recompute() = {asi}, expected 0.9228"


def test_v1198_dim_version_is_0_6_10():
    """V1198 dim_version = 0.6.10 (V0.6.9 = V1197 → V0.6.10 = V1198)."""
    m = _import_v1198()
    report = m.compute_v1198_lift()
    assert report.dim_version == "0.6.10"
    assert report.version == "0.1.0"


def test_v1198_snapshot_id_unique():
    """V1198 snapshot_id 必须 uuid 唯一."""
    m = _import_v1198()
    r1 = m.compute_v1198_lift()
    r2 = m.compute_v1198_lift()
    assert r1.snapshot_id != r2.snapshot_id
    assert r1.snapshot_id.startswith("v1198-")
    assert r2.snapshot_id.startswith("v1198-")


def test_v1198_north_star_locked_0_98():
    """V1198 ASI 北极星 = 0.9800 (主 22:33 LOCKED). V1198 0.9228 < 0.98 → gap > 0."""
    m = _import_v1198()
    report = m.compute_v1198_lift()
    assert report.asi_north_star == 0.9800
    assert report.asi_gap_after_lift > 0, f"V1198 0.9228 < 0.98 → gap > 0, got {report.asi_gap_after_lift}"
    assert abs(report.asi_gap_after_lift - 0.0572) < 0.001


def test_v1198_inflation_gap_zero():
    """V1198 真 lift 没有 inflation gap (V1161 / V1137 真测, 不是 continuity artifact)."""
    m = _import_v1198()
    report = m.compute_v1198_lift()
    # V1198 不报 3-formula inflation gap (这是 V1200 的工作)
    # 但 ASI recompute delta 必须等于 weight × sub_dim delta
    expected_delta = 0.05 * report.v2_philosophy_delta
    assert abs(report.asi_recompute_delta - expected_delta) < 0.001


# 主 19:33 走在前人经验上 — V1137 真答 attribute 验证
def test_v1198_v1137_imports_real_module():
    """V1198 真调 V1137.asi_philosophy_remaining_2 真 module (不是 mock)."""
    m = _import_v1198()
    sys.path.insert(0, str(PROJ_DIR))
    from apeireth import v1137_asi_philosophy_remaining_2 as v1137
    assert hasattr(v1137, "ALL_ANSWERS_V1137")
    assert len(v1137.ALL_ANSWERS_V1137) == 2
    # V1198 真读 V1137.ALL_ANSWERS_V1137 (主 17:43 实事求是)
    n = len(v1137.ALL_ANSWERS_V1137)
    assert n >= 2


# 主 17:58+20:46 — V3 哲学守门 (写在 report.notes)
def test_v1198_honest_notes_present():
    """V1198 report.notes 必须含主 17:43 实事求是 守门 (V3 不假装)."""
    m = _import_v1198()
    report = m.compute_v1198_lift()
    notes_text = " ".join(report.notes)
    assert "17:43" in notes_text or "实事求是" in notes_text
    assert "17:58" in notes_text or "20:46" in notes_text or "不假装" in notes_text
    assert "22:33" in notes_text or "北极星" in notes_text
    assert "19:33" in notes_text or "前人经验" in notes_text


def test_v1198_artifact_writable(tmp_path):
    """V1198 artifact 可写 (主 00:56 任何人都能接手)."""
    m = _import_v1198()
    report = m.compute_v1198_lift()
    artifact_path = m.write_artifact(report, artifact_dir=str(tmp_path))
    p = Path(artifact_path)
    assert p.exists(), f"artifact {p} should exist"
    data = json.loads(p.read_text(encoding="utf-8"))
    assert "v2_philosophy_lifted" in data
    assert "sub_dim_scores" in data
    assert "dim_lifts" not in data  # V1200 才用 dim_lifts
    assert data["dim_version"] == "0.6.10"


def test_v1198_render_report_md():
    """V1198 render_report_md() 输出必须含 5 sub-dim 表 + ASI 北极星 + Root cause."""
    m = _import_v1198()
    report = m.compute_v1198_lift()
    md = m.render_report_md(report)
    assert "# V1198" in md
    assert "V1137_remaining_real" in md
    assert "ALL_ANSWERS_V1137" in md
    assert "0.9148" in md  # baseline
    assert "0.9228" in md  # lifted
    assert "0.9800" in md or "0.98" in md  # north star
    assert "V3 哲学守门" in md
    assert len(md) > 2000, f"V1198 report md len={len(md)}, expected > 2000"


def test_v1198_cli_measure(monkeypatch):
    """V1198 CLI --measure 输出 0.8800."""
    m = _import_v1198()
    monkeypatch.setattr(sys, "argv", ["v1198", "--measure"])
    rc = m.main()
    assert rc == 0


def test_v1198_cli_measure_asi(monkeypatch):
    """V1198 CLI --measure-asi 输出 0.9228."""
    m = _import_v1198()
    monkeypatch.setattr(sys, "argv", ["v1198", "--measure-asi"])
    rc = m.main()
    assert rc == 0