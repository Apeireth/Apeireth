"""V1200 — ASI V0.6.10 dual dim lift tests (主 17:43 实事求是).

测试要点:
  - 主 17:43 实事求是: measure_v1200() 3-formula 真测
  - 主 22:33 北极星: ASI recompute lift 真算 (0.9148 → 0.9518)
  - 主 17:58+20:46 不假装: 6 不假装守门验证
  - 主 19:33 走在前人经验上: 站在 V1198 + V1199 + V1197 + V1194 肩上
  - 主 13:31 大胆激进: 一次 cron 双 dim 真 lift
  - 主 23:44 干到底: 真补 + 真测 + 真升
  - 主 00:56 任何人都能接手: 任何 cron 可调 measure_v1200()
  - 主 00:44 质量工程化: dataclass + 3-formula + inflation_gap
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

APEIRETH_DIR = Path(__file__).resolve().parent.parent
PROJ_DIR = APEIRETH_DIR.parent


def _import_v1200():
    sys.path.insert(0, str(PROJ_DIR))
    from apeireth import v1200_asi_v0610_dual_dim_lift as m
    return m


# 主 17:43 实事求是 — 3-formula 真测
def test_v1200_measure_returns_3formula_tuple():
    """V1200 measure_v1200() → 3-formula tuple (additive, recompute, corrected)."""
    m = _import_v1200()
    f1, f2, f3 = m.measure_v1200()
    assert isinstance(f1, float) and isinstance(f2, float) and isinstance(f3, float)
    # V1200 inflation_gap ≈ 0 (真 lift), 3-formula 一致
    assert abs(f1 - f2) < 0.001, f"V1200 additive={f1}, recompute={f2} 应 ≈ 一致"
    assert abs(f1 - f3) < 0.001, f"V1200 additive={f1}, corrected={f3} 应 ≈ 一致"
    assert abs(f2 - f3) < 0.001, f"V1200 recompute={f2}, corrected={f3} 应 ≈ 一致"
    # V1200 真 lift: 0.9148 + 0.008 + 0.029 = 0.9518
    assert abs(f2 - 0.9518) < 0.001, f"V1200 recompute = {f2}, expected 0.9518"


def test_v1200_lift_v1197_baseline():
    """V1200 ASI recompute: 0.9147625 (V1197) → 0.9518 (V1200)."""
    m = _import_v1200()
    report = m.compute_v1200_lift()
    assert abs(report.asi_recompute_baseline - 0.9147625) < 0.001
    assert report.asi_recompute_lifted == 0.9518
    # Δ = 0.008 (V1198) + 0.029 (V1199) = 0.037
    assert abs(report.asi_recompute_delta - 0.037) < 0.001


def test_v1200_two_dim_lifts():
    """V1200 2 dim lifts: v2_philosophy +0.008, real_llm_benchmark +0.029."""
    m = _import_v1200()
    report = m.compute_v1200_lift()
    assert "v2_philosophy" in report.dim_lifts
    assert "real_llm_benchmark" in report.dim_lifts
    # v2_philosophy
    vp = report.dim_lifts["v2_philosophy"]
    assert vp.baseline == 0.72
    assert vp.new_value == 0.88
    assert abs(vp.delta - 0.16) < 0.001
    assert vp.weight == 0.05
    assert abs(vp.lift_contribution - 0.008) < 0.001
    # real_llm_benchmark
    rl = report.dim_lifts["real_llm_benchmark"]
    assert rl.baseline == 0.416
    assert rl.new_value == 0.996
    assert abs(rl.delta - 0.58) < 0.001
    assert rl.weight == 0.05
    assert abs(rl.lift_contribution - 0.029) < 0.001


def test_v1200_inflation_gap_near_zero():
    """V1200 inflation_gap ≈ 0 (主 17:43 实事求是: 真 lift 没有 continuity inflation)."""
    m = _import_v1200()
    report = m.compute_v1200_lift()
    # V1197 additive=1.02, recompute=0.9148, gap=+0.1051 (continuity inflation)
    # V1200 additive ≈ recompute, gap ≈ 0 (真 lift)
    assert abs(report.inflation_gap_additive_vs_recompute) < 0.001, (
        f"V1200 inflation_gap = {report.inflation_gap_additive_vs_recompute}, expected ≈ 0"
    )
    assert abs(report.inflation_gap_additive_vs_corrected) < 0.001


def test_v1200_north_star_position_pct():
    """V1200 vs ASI 北极星 0.98: position ≈ 97.12%."""
    m = _import_v1200()
    report = m.compute_v1200_lift()
    assert report.asi_north_star == 0.9800
    assert report.gap_to_north_star_recompute > 0  # V1200 0.9518 < 0.98
    assert abs(report.gap_to_north_star_recompute - 0.0282) < 0.001
    # position_pct = 0.9518 / 0.98 * 100 = 97.12%
    assert abs(report.position_pct_recompute - 97.12) < 0.5


def test_v1200_dim_version_is_0_6_10():
    """V1200 dim_version = 0.6.10 (V0.6.9 = V1197 → V0.6.10 = V1198/V1199/V1200)."""
    m = _import_v1200()
    report = m.compute_v1200_lift()
    assert report.dim_version == "0.6.10"


def test_v1200_snapshot_id_unique():
    """V1200 snapshot_id 必须 uuid 唯一."""
    m = _import_v1200()
    r1 = m.compute_v1200_lift()
    r2 = m.compute_v1200_lift()
    assert r1.snapshot_id != r2.snapshot_id
    assert r1.snapshot_id.startswith("v1200-")


def test_v1200_baselines_v1197():
    """V1200 baselines 来自 V1197 3-formula honest recovery."""
    m = _import_v1200()
    report = m.compute_v1200_lift()
    assert report.v1197_additive == 1.0198699999999998
    assert report.v1197_recompute == 0.9147625
    assert report.v1197_corrected == 0.9147625


def test_v1200_dim_count():
    """V1200 2 dim lifts (n_dims_lifted=2)."""
    m = _import_v1200()
    report = m.compute_v1200_lift()
    assert report.n_dims_lifted == 2
    assert report.n_dims_pass == 2


# 主 19:33 走在前人经验上 — V1198 + V1199 模块验证
def test_v1200_v1198_module_importable():
    """V1200 引用的 V1198 v2_philosophy lift 模块可导入."""
    sys.path.insert(0, str(PROJ_DIR))
    from apeireth import v1198_v2_philosophy_lift as v1198
    assert hasattr(v1198, "measure_v1198")
    assert hasattr(v1198, "compute_v1198_lift")


def test_v1200_v1199_module_importable():
    """V1200 引用的 V1199 real_llm_benchmark lift 模块可导入."""
    sys.path.insert(0, str(PROJ_DIR))
    from apeireth import v1199_real_llm_benchmark_v1190 as v1199
    assert hasattr(v1199, "measure_v1199")
    assert hasattr(v1199, "compute_v1199_lift")
    # V1200 引用的 V1190 artifact 应可加载
    artifact = v1199._load_v1190_artifact()
    assert artifact is not None


# 主 17:58+20:46 — V3 哲学守门
def test_v1200_honest_notes_present():
    """V1200 report.notes 必须含主 17:43 实事求是 + V3 不假装守门."""
    m = _import_v1200()
    report = m.compute_v1200_lift()
    notes_text = " ".join(report.notes)
    assert "17:43" in notes_text or "实事求是" in notes_text
    assert "17:58" in notes_text or "20:46" in notes_text or "不假装" in notes_text
    assert "22:33" in notes_text or "北极星" in notes_text
    assert "19:33" in notes_text or "前人经验" in notes_text
    assert "13:31" in notes_text or "大胆激进" in notes_text


def test_v1200_artifact_writable(tmp_path):
    """V1200 artifact 可写 (主 00:56 任何人都能接手)."""
    m = _import_v1200()
    report = m.compute_v1200_lift()
    artifact_path = m.write_artifact(report, artifact_dir=str(tmp_path))
    p = Path(artifact_path)
    assert p.exists(), f"artifact {p} should exist"
    data = json.loads(p.read_text(encoding="utf-8"))
    assert "dim_lifts" in data
    assert "v2_philosophy" in data["dim_lifts"]
    assert "real_llm_benchmark" in data["dim_lifts"]
    assert "formula_1_additive" in data
    assert "formula_2_recompute" in data
    assert "formula_3_corrected" in data
    assert "inflation_gap_additive_vs_recompute" in data
    assert data["dim_version"] == "0.6.10"


def test_v1200_render_report_md():
    """V1200 render_report_md() 输出必须含 3-formula 表 + 2 dim lift 表 + V3 守门 + 历史."""
    m = _import_v1200()
    report = m.compute_v1200_lift()
    md = m.render_report_md(report)
    assert "# V1200" in md
    assert "3-formula" in md
    assert "formula_1" in md or "additive" in md
    assert "formula_2" in md or "recompute" in md
    assert "formula_3" in md or "corrected" in md
    assert "v2_philosophy" in md
    assert "real_llm_benchmark" in md
    assert "V1194" in md  # 历史对比
    assert "V1197" in md  # 历史对比
    assert "0.9148" in md  # V1197 baseline
    assert "0.9518" in md  # V1200 lifted
    assert "0.9800" in md or "0.98" in md  # north star
    assert "V3 哲学守门" in md
    assert "Honest note" in md or "不假装" in md
    assert len(md) > 2000, f"V1200 report md len={len(md)}, expected > 2000"


def test_v1200_cli_measure(monkeypatch, capsys):
    """V1200 CLI --measure 输出 0.9518."""
    m = _import_v1200()
    monkeypatch.setattr(sys, "argv", ["v1200", "--measure"])
    rc = m.main()
    assert rc == 0
    captured = capsys.readouterr()
    assert "0.9518" in captured.out, f"V1200 CLI --measure stdout = {captured.out}"


def test_v1200_cli_measure_additive(monkeypatch, capsys):
    """V1200 CLI --measure-additive 输出 0.9518 (与 recompute 一致, inflation_gap=0)."""
    m = _import_v1200()
    monkeypatch.setattr(sys, "argv", ["v1200", "--measure-additive"])
    rc = m.main()
    assert rc == 0
    captured = capsys.readouterr()
    assert "0.9518" in captured.out, f"V1200 CLI --measure-additive stdout = {captured.out}"


def test_v1200_cli_measure_corrected(monkeypatch, capsys):
    """V1200 CLI --measure-corrected 输出 0.9518."""
    m = _import_v1200()
    monkeypatch.setattr(sys, "argv", ["v1200", "--measure-corrected"])
    rc = m.main()
    assert rc == 0
    captured = capsys.readouterr()
    assert "0.9518" in captured.out, f"V1200 CLI --measure-corrected stdout = {captured.out}"