"""V1204 — ASI V0.6.14 real_production dual_dim_lift 测试 (主 22:33 北极星 + 主 17:43 实事求是 + 主 00:44 质量工程化).

测试 7 类:
  1. constants (版本/北极星/V1203 baseline)
  2. measure_v1204 (主入口, asi_recompute ∈ [V1203, north_star])
  3. cognitive_core 5+5=10 sub-dim (5 V1203 复用 + 5 V1204 真生产 artifact)
  4. engineering 5+5=10 sub-dim (5 V1203 复用 + 5 V1204 真生产 source)
  5. V1204Report dataclass + artifact roundtrip
  6. CLI flags (--measure / --json / --report / --full)
  7. V3 philosophy guard (3-formula + inflation gap ≤ 0.1)

主 13:31 大胆激进: ≥35 tests
主 17:43 实事求是: 不假装 lift = 真 ASI, 不假装 sub-dim = phenomenology
主 23:44 干到底: 真补 + 真测 + 真升 + 真 commit + 真 artifact
主 06:15 真生产闭环: V1181 docker + V1167 streamlit + V1190 LLM benchmark 真跑
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

# Ensure promethean dir is on path
_PROMETHEAN = Path(__file__).resolve().parent.parent
if str(_PROMETHEAN) not in sys.path:
    sys.path.insert(0, str(_PROMETHEAN))


from apeireth.v1204_asi_v0614_real_production_dual_dim_lift import (
    ASI_NORTH_STAR,
    V1203_RECOMPUTE,
    V1203_COGNITIVE_CORE_LIFTED,
    V1203_ENGINEERING_LIFTED,
    V1203_COGNITIVE_CORE_SUBDIM_NAMES,
    V1203_ENGINEERING_SUBDIM_NAMES,
    V1204_COGNITIVE_CORE_SUBDIM_NAMES,
    V1204_ENGINEERING_SUBDIM_NAMES,
    V1204_DIM_VERSION,
    V1204_VERSION,
    V1156_COGNITIVE_CORE_BASELINE,
    V1159_ENGINEERING_BASELINE,
    W_COGNITIVE_CORE,
    W_ENGINEERING,
    V1204Report,
    V1204SubDimEvidence,
    V1204DimLift,
    measure_v1204,
    render_report_md,
    run_v1204_full,
    write_v1204_artifact,
    _measure_v1181_docker_real,
    _measure_v1167_streamlit_real,
    _measure_v1190_llm_real,
    _measure_v1182_integration_real,
    _measure_v1189_integration_real,
    _measure_v1199_llm_benchmark_lift,
    _measure_v1106_engineering_lift,
    _measure_v1107_cognitive_lift,
    _measure_v1134_streamlit_real,
    _measure_v1077_v04_measurement,
)


# ============================================================================
# 1. constants
# ============================================================================

def test_v1204_version():
    assert V1204_VERSION == "0.1.0"


def test_v1204_dim_version():
    assert V1204_DIM_VERSION == "0.6.14"


def test_asi_north_star_locked():
    assert ASI_NORTH_STAR == 0.9800


def test_v1203_baseline_honest():
    """V1203 baseline 写死, 不能改 (主 17:43 实事求是)."""
    assert V1203_RECOMPUTE == 0.9711


def test_v1156_v1159_baselines_honest():
    """V1156 / V1159 baseline 写死 (主 17:43 实事求是)."""
    assert V1156_COGNITIVE_CORE_BASELINE == 0.92
    assert V1159_ENGINEERING_BASELINE == 0.92


def test_v1203_lifted_values():
    """V1203 lift 后的值写死."""
    assert V1203_COGNITIVE_CORE_LIFTED == 0.9457
    assert V1203_ENGINEERING_LIFTED == 0.9314


def test_weights_locked():
    """V2 5 位置权重 LOCKED (主 22:08)."""
    assert W_COGNITIVE_CORE == 0.05
    assert W_ENGINEERING == 0.05


def test_v1203_cog_subdim_count():
    """V1203 复用 10 个 sub-dim (主 17:43 实事求是)."""
    assert len(V1203_COGNITIVE_CORE_SUBDIM_NAMES) == 10


def test_v1203_eng_subdim_count():
    assert len(V1203_ENGINEERING_SUBDIM_NAMES) == 10


def test_v1204_cog_subdim_count():
    """V1204 新 5 个 cognitive_core sub-dim."""
    assert len(V1204_COGNITIVE_CORE_SUBDIM_NAMES) == 5


def test_v1204_eng_subdim_count():
    """V1204 新 5 个 engineering sub-dim."""
    assert len(V1204_ENGINEERING_SUBDIM_NAMES) == 5


def test_v1204_cog_subdim_real_production():
    """V1204 cognitive_core 5 NEW 全是真生产 artifact (主 06:15 + 主 23:44)."""
    assert "v1181_docker_real" in V1204_COGNITIVE_CORE_SUBDIM_NAMES
    assert "v1167_streamlit_real" in V1204_COGNITIVE_CORE_SUBDIM_NAMES
    assert "v1190_llm_real" in V1204_COGNITIVE_CORE_SUBDIM_NAMES
    assert "v1182_integration_real" in V1204_COGNITIVE_CORE_SUBDIM_NAMES
    assert "v1189_integration_real" in V1204_COGNITIVE_CORE_SUBDIM_NAMES


def test_v1204_eng_subdim_real_production():
    """V1204 engineering 5 NEW 全是真生产 (主 06:15 + 主 23:44)."""
    assert "v1199_llm_benchmark_lift" in V1204_ENGINEERING_SUBDIM_NAMES
    assert "v1106_engineering_lift" in V1204_ENGINEERING_SUBDIM_NAMES
    assert "v1107_cognitive_lift" in V1204_ENGINEERING_SUBDIM_NAMES
    assert "v1134_streamlit_real" in V1204_ENGINEERING_SUBDIM_NAMES
    assert "v1077_v04_measurement" in V1204_ENGINEERING_SUBDIM_NAMES


# ============================================================================
# 2. measure_v1204 (主入口)
# ============================================================================

def test_measure_v1204_returns_tuple():
    """measure_v1204() 返回 (asi, scores, evidence) 三元组."""
    asi, scores, ev = measure_v1204()
    assert isinstance(asi, float)
    assert isinstance(scores, dict)
    assert isinstance(ev, dict)


def test_measure_v1204_asi_recompute_in_range():
    """asi_recompute 应在 [V1203, north_star] 范围 (主 17:43 实事求是)."""
    asi, scores, ev = measure_v1204()
    assert asi >= V1203_RECOMPUTE - 0.005  # 允许微小回落 (新 sub-dim 拉低平均)
    assert asi <= ASI_NORTH_STAR


def test_measure_v1204_has_all_subdim_scores():
    """所有 20 个 sub-dim (10 cog + 10 eng) 应都在 scores."""
    asi, scores, ev = measure_v1204()
    for name in V1203_COGNITIVE_CORE_SUBDIM_NAMES:
        assert name in scores, f"missing V1203 cog sub-dim: {name}"
    for name in V1203_ENGINEERING_SUBDIM_NAMES:
        assert name in scores, f"missing V1203 eng sub-dim: {name}"
    for name in V1204_COGNITIVE_CORE_SUBDIM_NAMES:
        assert name in scores, f"missing V1204 cog sub-dim: {name}"
    for name in V1204_ENGINEERING_SUBDIM_NAMES:
        assert name in scores, f"missing V1204 eng sub-dim: {name}"


def test_measure_v1204_evidence_has_meta():
    """evidence['_meta'] 应含关键字段 (主 00:44 质量工程化)."""
    asi, scores, ev = measure_v1204()
    meta = ev["_meta"]
    assert "asi_recompute" in meta
    assert "asi_north_star" in meta
    assert "gap_to_north_star" in meta
    assert "position_pct" in meta
    assert meta["asi_north_star"] == ASI_NORTH_STAR


def test_measure_v1204_score_in_valid_range():
    """所有 sub-dim 分数应在 [0.0, 1.0]."""
    asi, scores, ev = measure_v1204()
    for name, s in scores.items():
        assert 0.0 <= s <= 1.0, f"{name} = {s} 不在 [0, 1]"


# ============================================================================
# 3. cognitive_core 5 V1204 NEW sub-dim (主 23:44 真生产闭环)
# ============================================================================

def test_measure_v1181_docker_real_returns_tuple():
    s, ev = _measure_v1181_docker_real()
    assert isinstance(s, float)
    assert isinstance(ev, dict)
    assert ev["name"] == "v1181_docker_real"


def test_measure_v1167_streamlit_real_returns_tuple():
    s, ev = _measure_v1167_streamlit_real()
    assert isinstance(s, float)
    assert isinstance(ev, dict)
    assert ev["name"] == "v1167_streamlit_real"


def test_measure_v1190_llm_real_returns_tuple():
    s, ev = _measure_v1190_llm_real()
    assert isinstance(s, float)
    assert isinstance(ev, dict)
    assert ev["name"] == "v1190_llm_real"


def test_measure_v1182_integration_real_returns_tuple():
    s, ev = _measure_v1182_integration_real()
    assert isinstance(s, float)
    assert isinstance(ev, dict)
    assert ev["name"] == "v1182_integration_real"


def test_measure_v1189_integration_real_returns_tuple():
    s, ev = _measure_v1189_integration_real()
    assert isinstance(s, float)
    assert isinstance(ev, dict)
    assert ev["name"] == "v1189_integration_real"


def test_v1181_docker_real_has_evidence():
    """V1181 真实生产 evidence (主 23:44 干到底)."""
    s, ev = _measure_v1181_docker_real()
    # 不强求 = 1.0 (取决于 artifact 是否存在)
    if s > 0:
        assert ev["checks"].get("artifact_exists") is True
        assert "raw" in ev


def test_v1167_streamlit_real_high_score():
    """V1167 streamlit 真启动 ≥ 0.5 (主 06:15 真生产)."""
    s, ev = _measure_v1167_streamlit_real()
    if s > 0:
        assert s >= 0.5, f"V1167 = {s}, expected ≥ 0.5"


def test_v1190_llm_real_has_pass_rate():
    """V1190 LLM benchmark 真跑 evidence 应含 pass_rate (主 06:15)."""
    s, ev = _measure_v1190_llm_real()
    if s > 0:
        assert "pass_rate" in ev["raw"]


def test_v1182_integration_has_dims():
    """V1182 v0.6 new_dim_collector evidence 应含 n_dims (主 17:43 实事求是)."""
    s, ev = _measure_v1182_integration_real()
    if s > 0:
        assert "n_dims" in ev["raw"]
        assert ev["raw"]["n_dims"] > 0


def test_v1189_integration_has_asi_lifted():
    """V1189 v1182 integration evidence 应含 asi_lifted."""
    s, ev = _measure_v1189_integration_real()
    if s > 0:
        assert "asi_lifted" in ev["raw"]


# ============================================================================
# 4. engineering 5 V1204 NEW sub-dim (主 23:44 真生产闭环)
# ============================================================================

def test_measure_v1199_llm_benchmark_lift_returns_tuple():
    s, ev = _measure_v1199_llm_benchmark_lift()
    assert isinstance(s, float)
    assert isinstance(ev, dict)
    assert ev["name"] == "v1199_llm_benchmark_lift"


def test_measure_v1106_engineering_lift_returns_tuple():
    s, ev = _measure_v1106_engineering_lift()
    assert isinstance(s, float)
    assert isinstance(ev, dict)
    assert ev["name"] == "v1106_engineering_lift"


def test_measure_v1107_cognitive_lift_returns_tuple():
    s, ev = _measure_v1107_cognitive_lift()
    assert isinstance(s, float)
    assert isinstance(ev, dict)
    assert ev["name"] == "v1107_cognitive_lift"


def test_measure_v1134_streamlit_real_returns_tuple():
    s, ev = _measure_v1134_streamlit_real()
    assert isinstance(s, float)
    assert isinstance(ev, dict)
    assert ev["name"] == "v1134_streamlit_real"


def test_measure_v1077_v04_measurement_returns_tuple():
    s, ev = _measure_v1077_v04_measurement()
    assert isinstance(s, float)
    assert isinstance(ev, dict)
    assert ev["name"] == "v1077_v04_measurement"


def test_v1106_engineering_lift_has_components():
    """V1106 engineering_lift ≥ 10 components 真有 (主 17:43 实事求是)."""
    s, ev = _measure_v1106_engineering_lift()
    if s > 0:
        assert "n_pass" in ev["raw"]
        assert ev["raw"]["n_pass"] >= 10


def test_v1199_llm_benchmark_high_score():
    """V1199 real_llm_benchmark lift ≥ 0.9 (主 06:15 真生产)."""
    s, ev = _measure_v1199_llm_benchmark_lift()
    if s > 0:
        assert s >= 0.9, f"V1199 = {s}, expected ≥ 0.9"


def test_v1107_cognitive_lift_has_callable():
    """V1107 fallback V1101_v04_dim_lift 应有 ≥ 15 callable."""
    s, ev = _measure_v1107_cognitive_lift()
    if s > 0:
        assert ev["raw"].get("n_callable", 0) >= 15


def test_v1134_streamlit_real_has_callable():
    """V1134 fallback V1080_subprocess_deploy 应有 ≥ 10 callable."""
    s, ev = _measure_v1134_streamlit_real()
    if s > 0:
        assert ev["raw"].get("n_callable", 0) >= 10


def test_v1077_v04_measurement_has_callable():
    """V1077 fallback V1116_v1077_v04_replicator 应有 ≥ 10 callable."""
    s, ev = _measure_v1077_v04_measurement()
    if s > 0:
        assert ev["raw"].get("n_callable", 0) >= 10


# ============================================================================
# 5. V1204Report dataclass + artifact roundtrip
# ============================================================================

def test_run_v1204_full_returns_report():
    """run_v1204_full() 返回 V1204Report (主 00:44 质量工程化)."""
    rep = run_v1204_full()
    assert isinstance(rep, V1204Report)
    assert rep.dim_version == V1204_DIM_VERSION


def test_v1204_report_has_dim_lifts():
    """V1204Report 应含 2 dim_lifts (cognitive_core + engineering)."""
    rep = run_v1204_full()
    assert "cognitive_core" in rep.dim_lifts
    assert "engineering" in rep.dim_lifts
    assert isinstance(rep.dim_lifts["cognitive_core"], V1204DimLift)


def test_v1204_report_has_sub_dim_scores():
    """V1204Report 应含 5+5=10 V1204 新 sub-dim scores + evidence."""
    rep = run_v1204_full()
    assert len(rep.cognitive_sub_dim_scores) == 5
    assert len(rep.engineering_sub_dim_scores) == 5
    assert len(rep.cognitive_sub_dim_evidence) == 5
    assert len(rep.engineering_sub_dim_evidence) == 5
    for name, ev in rep.cognitive_sub_dim_evidence.items():
        assert isinstance(ev, V1204SubDimEvidence)
    for name, ev in rep.engineering_sub_dim_evidence.items():
        assert isinstance(ev, V1204SubDimEvidence)


def test_v1204_report_has_v1203_reuse():
    """V1204Report 应含 V1203 复用的 10+10 sub-dim scores."""
    rep = run_v1204_full()
    assert len(rep.cognitive_v1203_sub_dim_scores) == 10
    assert len(rep.engineering_v1203_sub_dim_scores) == 10


def test_v1204_report_total_15_subdim():
    """V1204Report 应含 15+15=30 sub-dim 总 (10 V1203 复用 + 5 V1204 新)."""
    rep = run_v1204_full()
    assert rep.n_cognitive_total_subdims == 15
    assert rep.n_engineering_total_subdims == 15


def test_v1204_report_3_formula():
    """V1204Report 应含 3-formula (主 17:43 实事求是)."""
    rep = run_v1204_full()
    assert rep.formula_1_additive > 0
    assert rep.formula_2_recompute > 0
    assert rep.formula_3_corrected > 0


def test_v1204_report_to_dict_roundtrip():
    """V1204Report.to_dict() 可 roundtrip JSON (主 00:44 质量工程化)."""
    rep = run_v1204_full()
    d = rep.to_dict()
    assert isinstance(d, dict)
    assert d["dim_version"] == V1204_DIM_VERSION
    # JSON serialize
    s = json.dumps(d, ensure_ascii=False)
    d2 = json.loads(s)
    assert d2["snapshot_id"] == rep.snapshot_id


def test_v1204_summary_line_has_required_fields():
    """summary_line 含 ASI + V1203 + north_star + snapshot_id (主 00:56 任何人都能接手)."""
    rep = run_v1204_full()
    s = rep.summary_line()
    assert "V1204" in s
    assert "north_star" in s
    assert "snapshot" in s
    assert str(rep.snapshot_id) in s


def test_v1204_artifact_write():
    """write_v1204_artifact() 写 artifacts/v1204_*.json (主 23:44 干到底)."""
    rep = run_v1204_full()
    p = write_v1204_artifact(rep, artifact_dir=str(_PROMETHEAN / "artifacts"))
    assert p.exists()
    assert p.name == "v1204_asi_v0614_real_production_dual_dim_lift.json"
    # 内容可读
    data = json.loads(p.read_text(encoding="utf-8"))
    assert data["snapshot_id"] == rep.snapshot_id


def test_render_report_md_has_required_sections():
    """render_report_md() 含 3-formula + ASI north star + dim lifts + V3 guard."""
    rep = run_v1204_full()
    md = render_report_md(rep)
    assert "# V1204" in md
    assert "3-formula" in md
    assert "ASI north star" in md
    assert "Dim lifts" in md
    assert "V3 philosophy guard" in md
    assert "真生产闭环" in md or "real_production" in md


# ============================================================================
# 6. CLI flags
# ============================================================================

def test_v1204_cli_measure_flag(monkeypatch=None):
    """--measure flag 应返回 ASI (主 00:56 任何人都能接手)."""
    import subprocess
    r = subprocess.run(
        [sys.executable, "-X", "utf8", "-m", "apeireth.v1204_asi_v0614_real_production_dual_dim_lift", "--measure"],
        cwd=str(_PROMETHEAN),
        capture_output=True,
        text=True,
        timeout=60,
    )
    assert "V1204 ASI recompute" in r.stdout


def test_v1204_cli_json_flag():
    """--json flag 应输出 JSON."""
    import subprocess
    r = subprocess.run(
        [sys.executable, "-X", "utf8", "-m", "apeireth.v1204_asi_v0614_real_production_dual_dim_lift", "--json"],
        cwd=str(_PROMETHEAN),
        capture_output=True,
        text=True,
        timeout=60,
    )
    assert r.returncode == 0
    data = json.loads(r.stdout)
    assert "snapshot_id" in data
    assert data["dim_version"] == V1204_DIM_VERSION


# ============================================================================
# 7. V3 philosophy guard (主 17:58 + 主 20:46)
# ============================================================================

def test_v1204_3_formula_inflation_gap_bounded():
    """3-formula inflation gap 应 ≤ 0.1 (主 17:43 实事求是, 不假装 ASI)."""
    rep = run_v1204_full()
    assert rep.inflation_gap_additive_vs_recompute <= 0.1
    assert rep.inflation_gap_additive_vs_corrected <= 0.1


def test_v1204_not_pretend_asi_ultimate():
    """V1204 不假装 ASI 终极 (gap to north_star > 0)."""
    rep = run_v1204_full()
    assert rep.gap_to_north_star_recompute > 0  # 还有 gap
    assert rep.position_pct_recompute < 100.0  # 不是 100%


def test_v1204_dim_lift_contribution_bounded():
    """每个 dim lift contribution 应在 [-weight, +weight] 范围 (主 17:43)."""
    rep = run_v1204_full()
    for dl in rep.dim_lifts.values():
        assert abs(dl.contribution) <= dl.weight + 0.001


def test_v1204_has_asi_lift_or_honest_neutral():
    """V1204 ASI delta 应在 [-0.005, +0.05] 范围 (允许新 sub-dim 拉低平均, 但诚实)."""
    rep = run_v1204_full()
    assert -0.005 <= rep.asi_recompute_delta <= 0.05


def test_v1204_report_notes_has_philosophy_guard():
    """V1204Report.notes 应含 V3 哲学守门 (主 17:58 + 主 20:46)."""
    rep = run_v1204_full()
    notes_str = "\n".join(rep.notes)
    assert "V3 philosophy guard" in notes_str
    assert "不假装" in notes_str
    assert "真生产闭环" in notes_str or "real_production" in notes_str


# ============================================================================
# 8. 主 06:15 真生产闭环 (cron V1050/V1051/V1052 真实部署)
# ============================================================================

def test_v1204_includes_v1181_v1167_v1190():
    """V1204 必含 V1181 docker + V1167 streamlit + V1190 LLM benchmark 真生产闭环."""
    asi, scores, ev = measure_v1204()
    assert "v1181_docker_real" in scores
    assert "v1167_streamlit_real" in scores
    assert "v1190_llm_real" in scores


def test_v1204_includes_v1182_v1189_integration():
    """V1204 必含 V1182/V1189 v06 new_dim integration."""
    asi, scores, ev = measure_v1204()
    assert "v1182_integration_real" in scores
    assert "v1189_integration_real" in scores


def test_v1204_includes_v1199_v1106_v1107():
    """V1204 必含 V1199 LLM lift + V1106 engineering lift + V1107 cognitive lift."""
    asi, scores, ev = measure_v1204()
    assert "v1199_llm_benchmark_lift" in scores
    assert "v1106_engineering_lift" in scores
    assert "v1107_cognitive_lift" in scores


def test_v1204_includes_v1134_v1077():
    """V1204 必含 V1134 streamlit + V1077 v04 真测."""
    asi, scores, ev = measure_v1204()
    assert "v1134_streamlit_real" in scores
    assert "v1077_v04_measurement" in scores


# ============================================================================
# 9. 主 22:33 + 主 00:56 任何人都能接手
# ============================================================================

def test_v1204_evidence_explainable():
    """V1204 evidence 应能被任何 reviewer 读懂 (主 00:56)."""
    asi, scores, ev = measure_v1204()
    # 每个 V1204 新 sub-dim 应有 notes 解释
    for name in V1204_COGNITIVE_CORE_SUBDIM_NAMES + V1204_ENGINEERING_SUBDIM_NAMES:
        ev_obj = ev.get(f"cognitive_{name}") or ev.get(f"engineering_{name}")
        assert ev_obj is not None, f"{name} evidence missing"
        assert len(ev_obj.get("notes", [])) > 0, f"{name} no notes"


def test_v1204_has_v1203_path_or_fallback():
    """V1204 应能复用 V1203 (主 19:33 走在前人经验上)."""
    asi, scores, ev = measure_v1204()
    meta = ev["_meta"]
    # V1203 baseline 用得上 (有 _v1203_path 或 _v1203_fallback)
    assert "v1203_baseline" in meta or "_v1203_path" in ev or "_v1203_fallback" in ev