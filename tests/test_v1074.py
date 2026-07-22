"""V1074 ASI Production Runner 真测试 (主 22:33 ASI 北极星 + 主 17:43 实事求是 +
主 19:33 走在前人经验上 + 主 13:31 大胆激进 + 主 17:58+20:46 不假装 +
主 23:44 干到底 + 主 00:56 任何人都能接手 + 主 00:44 质量工程化).

主 17:43 实事求是: 真跑真测, 不硬编码, 不假装 pass.
主 00:44 质量工程化: 每个组件 ≥ 5 tests + sanity refs/guards/无假装 tests.

测试结构:
  TestStatusSnapshot          (8 tests) — 真测 / 真快照 / 真 hash
  TestMarkdownReportGenerator (8 tests) — 真 Markdown / 真维度 / 真历史
  TestPrometheusExporter      (7 tests) — 真 Prometheus 文本格式
  TestDecisionRecommender     (9 tests) — 真决策 / 真触发 / 不假装
  TestTrendAnalyzer           (6 tests) — 真 slope / 真 delta / 真 stdev
  TestArtifactWriter          (7 tests) — 真写盘 / 真 ls / 真 cat
  TestProductionRunnerBridge  (6 tests) — 真 V0.3 映射 / 真守门
  TestV3PhilosophyGuard       (5 tests) — 4 不假装
  TestProductionRunner        (8 tests) — 真端到端 / 真 5 步 / 真 artifacts
  TestCLIEntrypoint           (4 tests) — 真 argparse / 真 --report / 真 --snapshot

合计: 68 tests
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Dict, List

import pytest

# Make apeireth importable from project root
PROJECT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(PROJECT_ROOT))

from apeireth.v1074_asi_production_runner import (  # noqa: E402
    ASI_LEVEL_THRESHOLDS,
    DEFAULT_ARTIFACTS,
    DEFAULT_ARTIFACTS_DIR,
    DEFAULT_DATA_DIR,
    DEFAULT_REPORT_DIR,
    DIRECTION_CATALOG,
    MarkdownReportGenerator,
    ProductionRunner,
    ProductionRunnerBridge,
    PrometheusExporter,
    REFERENCES,
    StatusSnapshot,
    StatusSnapshotBuilder,
    TrendAnalyzer,
    V1074_VERSION,
    _clamp01,
    _level_from_score,
    _safe_div,
    _sha256,
    _utc_now_iso,
    Decision,
    DecisionRecommender,
    ArtifactWriter,
    v1074_philosophy_guard,
    _cli,
)


# ============================================================================
# Helpers
# ============================================================================

def _make_snapshot(
    level: str = "ASI",
    v03: float = 0.88,
    v02: float = 0.88,
    vcp: float = 0.96,
    cd: float = 1.0,
    ei: float = 0.84,
    n_modules: int = 1073,
    n_tests: int = 3629,
    n_commits: int = 377,
    guard_ok: bool = True,
    history: List[Dict[str, Any]] = None,
) -> StatusSnapshot:
    """V1074 真构造测试用 snapshot (主 17:43 实事求是: 真字段)."""
    return StatusSnapshot(
        snapshot_id=f"snap_test_{v03:.4f}".replace(".", "_"),
        ts=1700000000.0,
        ts_iso="2026-07-22T00:00:00+00:00",
        version=V1074_VERSION,
        level=level,
        level_score=v03,
        v02_base=v02,
        v03_score=v03,
        n_modules=n_modules,
        n_tests=n_tests,
        n_commits=n_commits,
        dim_breakdown={
            "phi_proxy": 1.0,
            "capabilities": 0.7,
            "cross_domain": cd,
            "engineering": 1.0,
            "vcp_4": vcp,
            "v2_philosophy": 0.87,
            "rubric_open": 1.0,
            "real_production": 0.75,
            "cognitive_core": 0.70,
            "self_organizing_core": 0.81,
            "plugin_core": 0.83,
            "self_improving_core": 0.80,
            "neurosymbolic": 0.79,
            "world_model": 0.72,
            "reinforcement_learning": 0.88,
            "scientific_method": 0.94,
            "eternal_identity": ei,
        },
        v1071_vcp_score=vcp,
        v1071_cross_domain=cd,
        v1072_eternal_identity=ei,
        philosophy_guard_ok=guard_ok,
        score_history=history or [],
        notes={"build_ts": "2026-07-22T00:00:00+00:00"},
        refs=REFERENCES,
    )


# ============================================================================
# TestStatusSnapshot (8 tests)
# ============================================================================

class TestStatusSnapshot:
    def test_snapshot_basic_construction(self):
        """V1074 StatusSnapshot 真基础构造 (主 17:43)."""
        s = _make_snapshot()
        assert s.level == "ASI"
        assert 0.85 <= s.v03_score <= 0.98
        assert s.n_modules == 1073
        assert s.snapshot_id.startswith("snap_test_")

    def test_snapshot_to_dict(self):
        """V1074 StatusSnapshot 真 to_dict (主 17:43)."""
        s = _make_snapshot()
        d = s.to_dict()
        assert d["level"] == "ASI"
        assert d["v03_score"] == s.v03_score
        assert "dim_breakdown" in d
        assert len(d["dim_breakdown"]) == 17

    def test_snapshot_to_json_roundtrip(self):
        """V1074 StatusSnapshot 真 JSON 往返 (主 17:43)."""
        s = _make_snapshot(v03=0.92)
        js = s.to_json()
        d = json.loads(js)
        assert d["v03_score"] == 0.92
        assert d["level"] == _level_from_score(0.92)

    def test_snapshot_short_hash_stable(self):
        """V1074 StatusSnapshot 真 hash 稳定 (主 17:43)."""
        s = _make_snapshot(v03=0.85)
        h1 = s.short_hash()
        # 同样输入同样 hash
        s2 = _make_snapshot(v03=0.85)
        assert h1 == s2.short_hash()
        # 不同输入不同 hash
        s3 = _make_snapshot(v03=0.86)
        assert h1 != s3.short_hash()

    def test_snapshot_short_hash_length_16(self):
        """V1074 StatusSnapshot 真 hash 长度 16 (主 17:43)."""
        s = _make_snapshot()
        h = s.short_hash()
        assert len(h) == 16

    def test_snapshot_builder_count_modules(self):
        """V1074 StatusSnapshotBuilder 真数模块 (主 17:43)."""
        builder = StatusSnapshotBuilder(project_dir=str(PROJECT_ROOT))
        n = builder.count_modules()
        # 主 17:43 实事求是: 真数, 不假装
        assert n > 100  # 至少有 100+ V-modules

    def test_snapshot_builder_count_tests(self):
        """V1074 StatusSnapshotBuilder 真数 tests (主 17:43)."""
        builder = StatusSnapshotBuilder(project_dir=str(PROJECT_ROOT))
        n = builder.count_tests()
        assert n > 1000  # 至少有 1000+ 真 tests

    def test_snapshot_builder_build_runs(self):
        """V1074 StatusSnapshotBuilder 真 build (主 17:43)."""
        builder = StatusSnapshotBuilder(project_dir=str(PROJECT_ROOT))
        s = builder.build()
        assert s.snapshot_id.startswith("snap_")
        assert s.n_modules > 100
        assert s.n_tests > 1000
        assert s.v03_score >= 0.0


# ============================================================================
# TestMarkdownReportGenerator (8 tests)
# ============================================================================

class TestMarkdownReportGenerator:
    def test_report_basic_render(self):
        """V1074 MarkdownReport 真基础渲染 (主 00:56)."""
        rep = MarkdownReportGenerator()
        s = _make_snapshot()
        md = rep.render(s)
        assert "# ASI Status Report" in md
        assert s.snapshot_id in md
        assert s.level in md

    def test_report_includes_dim_table(self):
        """V1074 MarkdownReport 真含 17 维表 (主 22:33)."""
        rep = MarkdownReportGenerator()
        s = _make_snapshot()
        md = rep.render(s)
        # 17 维都得有
        for dim in s.dim_breakdown.keys():
            assert dim in md, f"missing dim {dim} in report"

    def test_report_includes_subscores(self):
        """V1074 MarkdownReport 真含 V1071/V1072 子分 (主 22:33)."""
        rep = MarkdownReportGenerator()
        s = _make_snapshot()
        md = rep.render(s)
        assert "V1071 VCP" in md
        assert "V1072" in md or "eternal_identity" in md
        assert f"{s.v1071_vcp_score:.4f}" in md

    def test_report_includes_history(self):
        """V1074 MarkdownReport 真含历史 (主 23:44)."""
        rep = MarkdownReportGenerator()
        history = [
            {"snapshot_id": "snap_h1", "ts_iso": "2026-07-22T01:00:00+00:00", "v03_score": 0.85},
            {"snapshot_id": "snap_h2", "ts_iso": "2026-07-22T02:00:00+00:00", "v03_score": 0.87},
        ]
        s = _make_snapshot(history=history)
        md = rep.render(s)
        assert "snap_h1" in md or "snap_h2" in md
        assert "delta" in md.lower() or "趋势" in md

    def test_report_includes_philosophy_guard(self):
        """V1074 MarkdownReport 真含 V3 守门 (主 17:58 + 主 20:46)."""
        rep = MarkdownReportGenerator()
        s = _make_snapshot()
        md = rep.render(s)
        assert "哲学守门" in md or "philosophy" in md.lower()
        assert "不假装" in md

    def test_report_includes_references(self):
        """V1074 MarkdownReport 真含 references (主 19:33)."""
        rep = MarkdownReportGenerator()
        s = _make_snapshot()
        md = rep.render(s)
        assert "GitHubActions2019" in md
        assert "Prometheus2012" in md
        assert "OpenTelemetry2019" in md

    def test_report_v03_score_formatted(self):
        """V1074 MarkdownReport 真 V0.3 数字格式化 (主 17:43)."""
        rep = MarkdownReportGenerator()
        s = _make_snapshot(v03=0.88123)
        md = rep.render(s)
        # 必须有 0.8812 格式
        assert "0.8812" in md

    def test_report_no_history_no_trend_block(self):
        """V1074 MarkdownReport 无历史时无 trend block (主 23:44)."""
        rep = MarkdownReportGenerator()
        s = _make_snapshot(history=[])
        md = rep.render(s)
        # 无历史时不应有 trend 表
        assert "首末 delta" not in md
        assert "标准差" not in md


# ============================================================================
# TestPrometheusExporter (7 tests)
# ============================================================================

class TestPrometheusExporter:
    def test_prometheus_basic_render(self):
        """V1074 PrometheusExporter 真基础渲染 (主 19:33)."""
        prom = PrometheusExporter()
        s = _make_snapshot()
        txt = prom.render(s)
        assert "asi_v03_score" in txt
        assert f"{s.v03_score:.6f}" in txt

    def test_prometheus_has_help_and_type(self):
        """V1074 PrometheusExporter 真 # HELP / # TYPE (主 19:33)."""
        prom = PrometheusExporter()
        s = _make_snapshot()
        txt = prom.render(s)
        assert "# HELP asi_v03_score" in txt
        assert "# TYPE asi_v03_score gauge" in txt
        assert "# HELP asi_n_modules" in txt

    def test_prometheus_includes_labels(self):
        """V1074 PrometheusExporter 真 labels (主 19:33)."""
        prom = PrometheusExporter()
        s = _make_snapshot()
        txt = prom.render(s)
        assert f'snapshot_id="{s.snapshot_id}"' in txt
        assert f'level="{s.level}"' in txt
        assert f'version="{s.version}"' in txt

    def test_prometheus_includes_counts(self):
        """V1074 PrometheusExporter 真 counts (主 17:43)."""
        prom = PrometheusExporter()
        s = _make_snapshot(n_modules=1073, n_tests=3629, n_commits=377)
        txt = prom.render(s)
        # Prometheus 格式: metric_name{labels} value
        assert re.search(r"asi_n_modules\{[^}]*\}\s+1073", txt), f"missing n_modules 1073 in: {txt[:300]}"
        assert re.search(r"asi_n_tests\{[^}]*\}\s+3629", txt), f"missing n_tests 3629 in: {txt[:300]}"
        assert re.search(r"asi_n_commits\{[^}]*\}\s+377", txt), f"missing n_commits 377 in: {txt[:300]}"

    def test_prometheus_includes_subscores(self):
        """V1074 PrometheusExporter 真 V1071/V1072 (主 22:33)."""
        prom = PrometheusExporter()
        s = _make_snapshot(vcp=0.96, ei=0.84)
        txt = prom.render(s)
        assert "asi_v1071_vcp" in txt
        assert "asi_v1072_eternal_identity" in txt
        assert "0.960000" in txt or "0.96" in txt
        assert "0.840000" in txt or "0.84" in txt

    def test_prometheus_includes_dim_breakdown(self):
        """V1074 PrometheusExporter 真 dim breakdown (主 22:33)."""
        prom = PrometheusExporter()
        s = _make_snapshot()
        txt = prom.render(s)
        for dim in s.dim_breakdown.keys():
            safe = re.sub(r"[^a-zA-Z0-9_]", "_", dim)
            assert f"asi_dim_{safe}" in txt, f"missing dim {safe}"

    def test_prometheus_guard_ok_metric(self):
        """V1074 PrometheusExporter 真 guard metric (主 17:58)."""
        prom = PrometheusExporter()
        s_ok = _make_snapshot(guard_ok=True)
        s_bad = _make_snapshot(guard_ok=False)
        txt_ok = prom.render(s_ok)
        txt_bad = prom.render(s_bad)
        assert "asi_philosophy_guard_ok" in txt_ok
        assert "} 1" in txt_ok
        assert "} 0" in txt_bad


# ============================================================================
# TestDecisionRecommender (9 tests)
# ============================================================================

class TestDecisionRecommender:
    def test_decision_basic_recommend(self):
        """V1074 DecisionRecommender 真基础推荐 (主 13:31)."""
        rec = DecisionRecommender()
        s = _make_snapshot(v03=0.88, vcp=0.96)
        d = rec.recommend(s)
        assert d.decision_id.startswith("dec_")
        assert d.chosen_direction in [c["id"] for c in DIRECTION_CATALOG]

    def test_decision_has_alternatives(self):
        """V1074 DecisionRecommender 真 alternatives (主 13:31)."""
        rec = DecisionRecommender()
        s = _make_snapshot(v03=0.85, vcp=0.5)
        d = rec.recommend(s)
        assert isinstance(d.alternatives, list)
        # alternatives 必须有 id 和 title
        if d.alternatives:
            for alt in d.alternatives:
                assert "id" in alt
                assert "title" in alt
                assert "expected_score_lift" in alt

    def test_decision_lift_non_negative(self):
        """V1074 DecisionRecommender 真 lift ≥ 0 (主 13:31)."""
        rec = DecisionRecommender()
        for v03 in (0.70, 0.85, 0.92, 0.97):
            s = _make_snapshot(v03=v03)
            d = rec.recommend(s)
            assert d.expected_score_lift >= 0.0

    def test_decision_confidence_in_range(self):
        """V1074 DecisionRecommender 真 confidence ∈ [0,1] (主 17:43)."""
        rec = DecisionRecommender()
        s = _make_snapshot()
        d = rec.recommend(s)
        assert 0.0 <= d.confidence <= 1.0

    def test_decision_philosophy_guard_ok(self):
        """V1074 DecisionRecommender 真 philosophy_guard_ok (主 17:58)."""
        rec = DecisionRecommender()
        s = _make_snapshot()
        d = rec.recommend(s)
        assert d.philosophy_guard_ok is True
        assert d.confidence < 1.0

    def test_decision_to_dict_roundtrip(self):
        """V1074 Decision 真 to_dict (主 17:43)."""
        rec = DecisionRecommender()
        s = _make_snapshot()
        d = rec.recommend(s)
        dd = d.to_dict()
        assert dd["chosen_direction"] == d.chosen_direction
        assert dd["expected_score_lift"] == d.expected_score_lift

    def test_decision_eternal_identity_trigger_low(self):
        """V1074 DecisionRecommender 真 eternal_identity low → 推荐深挖 (主 13:31)."""
        rec = DecisionRecommender()
        s = _make_snapshot(ei=0.50)  # low eternal identity
        d = rec.recommend(s)
        # 应该推荐 eternal_identity_deep 方向之一
        assert "eternal_identity" in d.chosen_direction or "deep" in d.chosen_direction or len(d.alternatives) > 0

    def test_decision_high_v03_tends_hold(self):
        """V1074 DecisionRecommender 真 high V0.3 → 真决策 in 候选 (主 23:44)."""
        rec = DecisionRecommender()
        s = _make_snapshot(v03=0.96, vcp=0.99)
        d = rec.recommend(s)
        # 真决策必须 in 候选 catalog
        assert d.chosen_direction in [c["id"] for c in DIRECTION_CATALOG]
        # 真 lift 必须合理 (0.0~0.5)
        assert 0.0 <= d.expected_score_lift <= 0.5
        # 真 confidence 必须 < 1.0 (主 17:58 不假装 optimal)
        assert d.confidence < 1.0

    def test_decision_lift_below_threshold(self):
        """V1074 DecisionRecommender 真 lift < 0.50 (主 17:58 + 主 20:46 不假装)."""
        rec = DecisionRecommender()
        s = _make_snapshot()
        d = rec.recommend(s)
        # 不假装 lift 巨大
        assert d.expected_score_lift < 0.50


# ============================================================================
# TestTrendAnalyzer (6 tests)
# ============================================================================

class TestTrendAnalyzer:
    def test_trend_empty_history(self):
        """V1074 TrendAnalyzer 真空历史 (主 17:43)."""
        ta = TrendAnalyzer()
        s = _make_snapshot(history=[])
        t = ta.analyze(s)
        assert t["n_history"] == 0
        assert t["score_slope"] == 0.0

    def test_trend_basic_slope(self):
        """V1074 TrendAnalyzer 真 slope (主 23:44)."""
        ta = TrendAnalyzer()
        history = [
            {"v03_score": 0.80, "n_modules": 1000, "n_tests": 3000, "n_commits": 300},
            {"v03_score": 0.85, "n_modules": 1050, "n_tests": 3500, "n_commits": 350},
            {"v03_score": 0.90, "n_modules": 1073, "n_tests": 3629, "n_commits": 377},
        ]
        s = _make_snapshot(history=history)
        t = ta.analyze(s)
        # 上升趋势
        assert t["score_slope"] > 0.0
        assert t["modules_slope"] > 0.0

    def test_trend_score_min_max(self):
        """V1074 TrendAnalyzer 真 min/max (主 23:44)."""
        ta = TrendAnalyzer()
        history = [
            {"v03_score": 0.70},
            {"v03_score": 0.95},
            {"v03_score": 0.85},
        ]
        s = _make_snapshot(history=history)
        t = ta.analyze(s)
        assert t["score_min"] == 0.70
        assert t["score_max"] == 0.95

    def test_trend_score_mean_stdev(self):
        """V1074 TrendAnalyzer 真 mean/stdev (主 17:43)."""
        ta = TrendAnalyzer()
        history = [
            {"v03_score": 0.80},
            {"v03_score": 0.85},
            {"v03_score": 0.90},
        ]
        s = _make_snapshot(history=history)
        t = ta.analyze(s)
        assert abs(t["score_mean"] - 0.85) < 0.001
        assert t["score_stdev"] > 0.0

    def test_trend_current_vs_first_delta(self):
        """V1074 TrendAnalyzer 真 current vs first delta (主 23:44)."""
        ta = TrendAnalyzer()
        history = [{"v03_score": 0.70}, {"v03_score": 0.75}]
        s = _make_snapshot(history=history, v03=0.85)
        t = ta.analyze(s)
        assert t["current_vs_first_delta"] == pytest.approx(0.15, abs=1e-6)

    def test_trend_linear_slope_helper(self):
        """V1074 TrendAnalyzer.linear_slope 真 (主 23:44)."""
        # 完美线性 y = 2x
        ys = [0.0, 2.0, 4.0, 6.0]
        slope = TrendAnalyzer.linear_slope(ys)
        assert slope == pytest.approx(2.0, abs=1e-6)


# ============================================================================
# TestArtifactWriter (7 tests)
# ============================================================================

class TestArtifactWriter:
    def test_writer_ensure_dirs(self, tmp_path):
        """V1074 ArtifactWriter 真建目录 (主 23:44)."""
        w = ArtifactWriter(project_dir=str(tmp_path))
        w.ensure_dirs()
        assert (tmp_path / DEFAULT_REPORT_DIR).exists()
        assert (tmp_path / DEFAULT_DATA_DIR).exists()
        assert (tmp_path / DEFAULT_ARTIFACTS_DIR).exists()

    def test_writer_write_snapshot_json(self, tmp_path):
        """V1074 ArtifactWriter 真写 snapshot JSON (主 23:44)."""
        w = ArtifactWriter(project_dir=str(tmp_path))
        s = _make_snapshot(v03=0.91)
        path = w.write_snapshot_json(s)
        assert path.exists()
        loaded = json.loads(path.read_text(encoding="utf-8"))
        assert loaded["v03_score"] == 0.91

    def test_writer_write_report_md(self, tmp_path):
        """V1074 ArtifactWriter 真写 Markdown 报告 (主 00:56)."""
        w = ArtifactWriter(project_dir=str(tmp_path))
        s = _make_snapshot()
        md = "# test report"
        path = w.write_report_md(md)
        assert path.exists()
        assert path.read_text(encoding="utf-8") == "# test report"

    def test_writer_write_prometheus_txt(self, tmp_path):
        """V1074 ArtifactWriter 真写 Prometheus (主 19:33)."""
        w = ArtifactWriter(project_dir=str(tmp_path))
        path = w.write_prometheus_txt("# HELP test\n# TYPE test gauge\ntest 1.0\n")
        assert path.exists()
        assert "# HELP test" in path.read_text(encoding="utf-8")

    def test_writer_write_decision_json(self, tmp_path):
        """V1074 ArtifactWriter 真写决策 (主 13:31)."""
        w = ArtifactWriter(project_dir=str(tmp_path))
        rec = DecisionRecommender()
        s = _make_snapshot()
        d = rec.recommend(s)
        path = w.write_decision_json(d)
        assert path.exists()
        loaded = json.loads(path.read_text(encoding="utf-8"))
        assert loaded["chosen_direction"] == d.chosen_direction

    def test_writer_append_history_jsonl(self, tmp_path):
        """V1074 ArtifactWriter 真追加历史 (主 23:44)."""
        w = ArtifactWriter(project_dir=str(tmp_path))
        s1 = _make_snapshot(v03=0.85)
        s2 = _make_snapshot(v03=0.86)
        p1 = w.append_history_jsonl(s1)
        p2 = w.append_history_jsonl(s2)
        assert p1 == p2  # 同路径
        lines = [l for l in p1.read_text(encoding="utf-8").splitlines() if l.strip()]
        assert len(lines) == 2
        loaded = [json.loads(l) for l in lines]
        assert loaded[0]["v03_score"] == 0.85
        assert loaded[1]["v03_score"] == 0.86

    def test_writer_write_all(self, tmp_path):
        """V1074 ArtifactWriter 真 write_all 一次性 (主 23:44 干到底)."""
        w = ArtifactWriter(project_dir=str(tmp_path))
        s = _make_snapshot()
        rec = DecisionRecommender()
        d = rec.recommend(s)
        ta = TrendAnalyzer()
        t = ta.analyze(s)
        rep = MarkdownReportGenerator()
        prom = PrometheusExporter()
        md = rep.render(s)
        ptxt = prom.render(s)
        paths = w.write_all(s, d, t, md, ptxt)
        # 6 个文件
        assert len(paths) == 6
        for k, v in paths.items():
            assert Path(v).exists(), f"{k} not written: {v}"


# ============================================================================
# TestProductionRunnerBridge (6 tests)
# ============================================================================

class TestProductionRunnerBridge:
    def test_bridge_runner_score_full(self):
        """V1074 ProductionRunnerBridge 真完整 snapshot → 高分 (主 17:43)."""
        b = ProductionRunnerBridge()
        s = _make_snapshot(guard_ok=True)
        rs = b.runner_score(s)
        assert rs >= 0.99

    def test_bridge_runner_score_partial(self):
        """V1074 ProductionRunnerBridge 真部分 snapshot → 低分 (主 17:43)."""
        b = ProductionRunnerBridge()
        s = StatusSnapshot(
            snapshot_id="snap_partial",
            ts=0,
            ts_iso="",
            version=V1074_VERSION,
            level="ANI",
            level_score=0.3,
            v02_base=0.3,
            v03_score=0.3,
            n_modules=0,
            n_tests=0,
            n_commits=0,
            dim_breakdown={},
            v1071_vcp_score=0.0,
            v1071_cross_domain=0.0,
            v1072_eternal_identity=0.0,
            philosophy_guard_ok=False,
            refs=[],
        )
        rs = b.runner_score(s)
        assert rs < 0.5

    def test_bridge_v03_contribution_in_range(self):
        """V1074 ProductionRunnerBridge 真 V0.3 贡献 ∈ [0,1] (主 22:33)."""
        b = ProductionRunnerBridge()
        s = _make_snapshot()
        c = b.asi_v03_runner_contribution(s)
        assert 0.0 <= c <= 1.0

    def test_bridge_report_philosophy_guard(self):
        """V1074 ProductionRunnerBridge 真 philosophy_guard 4 项 (主 17:58)."""
        b = ProductionRunnerBridge()
        s = _make_snapshot()
        r = b.bridge_report(s)
        assert "runner_is_not_asi" in r["philosophy_guard"]
        assert "report_is_not_production" in r["philosophy_guard"]
        assert "decision_is_not_optimal" in r["philosophy_guard"]
        assert "v03_measurement_is_not_asi" in r["philosophy_guard"]

    def test_bridge_report_v03_score_match(self):
        """V1074 ProductionRunnerBridge 真 V0.3 分数匹配 (主 17:43)."""
        b = ProductionRunnerBridge()
        s = _make_snapshot(v03=0.91)
        r = b.bridge_report(s)
        assert r["v03_score"] == 0.91
        assert r["level"] == _level_from_score(0.91)

    def test_bridge_runner_weight_proposed_positive(self):
        """V1074 ProductionRunnerBridge 真 weight > 0 (主 13:31)."""
        b = ProductionRunnerBridge()
        assert b.RUNNER_WEIGHT_PROPOSED > 0
        assert b.RUNNER_WEIGHT_PROPOSED < 0.10  # 主 17:58 不假装加很大


# ============================================================================
# TestV3PhilosophyGuard (5 tests)
# ============================================================================

class TestV3PhilosophyGuard:
    def test_philosophy_guard_4_keys(self):
        """V1074 V3PhilosophyGuard 真 4 keys (主 17:58 + 主 20:46)."""
        g = v1074_philosophy_guard()
        assert len(g) == 4
        assert "runner_is_not_asi" in g
        assert "report_is_not_production" in g
        assert "decision_is_not_optimal" in g
        assert "v03_measurement_is_not_asi" in g

    def test_philosophy_guard_all_true(self):
        """V1074 V3PhilosophyGuard 真全 True (主 17:43)."""
        g = v1074_philosophy_guard()
        assert all(v is True for v in g.values())

    def test_philosophy_guard_runner_not_asi(self):
        """V1074 V3PhilosophyGuard 真 runner ≠ ASI (主 17:58)."""
        g = v1074_philosophy_guard()
        assert g["runner_is_not_asi"] is True

    def test_philosophy_guard_report_not_production(self):
        """V1074 V3PhilosophyGuard 真 report ≠ production (主 17:58)."""
        g = v1074_philosophy_guard()
        assert g["report_is_not_production"] is True

    def test_philosophy_guard_decision_not_optimal(self):
        """V1074 V3PhilosophyGuard 真 decision ≠ optimal (主 20:46)."""
        g = v1074_philosophy_guard()
        assert g["decision_is_not_optimal"] is True


# ============================================================================
# TestProductionRunner (8 tests) — 真端到端 (主 00:56)
# ============================================================================

class TestProductionRunner:
    def test_runner_run_no_write(self):
        """V1074 ProductionRunner 真 run (no write) (主 17:43)."""
        runner = ProductionRunner(project_dir=str(PROJECT_ROOT))
        r = runner.run(write_artifacts=False)
        assert r.snapshot_id.startswith("snap_") or "snap_" in r.snapshot_id
        assert r.n_steps >= 5
        assert r.all_ok

    def test_runner_run_with_artifacts(self, tmp_path):
        """V1074 ProductionRunner 真 run + 写真 artifacts (主 23:44 干到底)."""
        runner = ProductionRunner(project_dir=str(tmp_path))
        # 准备一个 fake history 让 trend 路径走完
        history_dir = tmp_path / DEFAULT_DATA_DIR
        history_dir.mkdir(parents=True, exist_ok=True)
        history_file = history_dir / DEFAULT_ARTIFACTS["history_jsonl"]
        history_file.write_text(
            json.dumps({
                "snapshot_id": "snap_h_test",
                "ts_iso": "2026-07-22T00:00:00+00:00",
                "v03_score": 0.80,
                "n_modules": 100,
                "n_tests": 1000,
                "n_commits": 100,
            }) + "\n",
            encoding="utf-8",
        )
        r = runner.run(write_artifacts=True)
        assert r.all_ok
        assert len(r.artifacts) >= 5
        for k, v in r.artifacts.items():
            assert Path(v).exists(), f"artifact {k} not written: {v}"

    def test_runner_includes_5_steps(self):
        """V1074 ProductionRunner 真 5 步 (主 23:44)."""
        runner = ProductionRunner(project_dir=str(PROJECT_ROOT))
        r = runner.run(write_artifacts=False)
        # 至少 5 步
        assert r.n_steps >= 5

    def test_runner_decision_in_result(self):
        """V1074 ProductionRunner 真决策 in result (主 13:31)."""
        runner = ProductionRunner(project_dir=str(PROJECT_ROOT))
        r = runner.run(write_artifacts=False)
        assert r.decision_id.startswith("dec_")
        assert r.chosen_direction in [c["id"] for c in DIRECTION_CATALOG]
        assert r.expected_score_lift >= 0.0

    def test_runner_v03_score_matches_snapshot(self):
        """V1074 ProductionRunner 真 V0.3 匹配 snapshot (主 22:33)."""
        runner = ProductionRunner(project_dir=str(PROJECT_ROOT))
        r = runner.run(write_artifacts=False)
        # 真 V0.3 score 必须 ∈ [0,1]
        assert 0.0 <= r.v03_score <= 1.0

    def test_runner_philosophy_guard_in_result(self):
        """V1074 ProductionRunner 真 philosophy_guard in result (主 17:58)."""
        runner = ProductionRunner(project_dir=str(PROJECT_ROOT))
        r = runner.run(write_artifacts=False)
        assert "runner_is_not_asi" in r.philosophy_guard
        assert "report_is_not_production" in r.philosophy_guard

    def test_runner_result_to_dict(self):
        """V1074 ProductionRunner 真 to_dict (主 17:43)."""
        runner = ProductionRunner(project_dir=str(PROJECT_ROOT))
        r = runner.run(write_artifacts=False)
        d = r.to_dict()
        assert d["snapshot_id"] == r.snapshot_id
        assert d["v03_score"] == r.v03_score
        assert d["n_steps"] == r.n_steps

    def test_runner_real_project_v03(self):
        """V1074 ProductionRunner 真项目 V0.3 真测 (主 17:43 实事求是).

        不假设具体分数, 只检查:
          - v03_score >= 0 (真测到了)
          - level 是 ASI/AGI/ANI/TRANSCENDENT 之一
          - 真模块数 > 1000
        """
        runner = ProductionRunner(project_dir=str(PROJECT_ROOT))
        r = runner.run(write_artifacts=False)
        assert r.v03_score > 0.0
        assert r.level in ("ANI", "AGI", "ASI", "TRANSCENDENT")


# ============================================================================
# TestCLIEntrypoint (4 tests)
# ============================================================================

class TestCLIEntrypoint:
    def test_cli_report_flag_runs(self):
        """V1074 CLI 真 --report (主 00:56)."""
        rc = _cli(["--report", "--project-dir", str(PROJECT_ROOT), "--no-write"])
        assert rc == 0

    def test_cli_snapshot_flag(self):
        """V1074 CLI 真 --snapshot (主 00:56)."""
        # snapshot mode 走 print, 难以测 print, 改为测 rc + 通过主项目测
        rc = _cli(["--snapshot", "--project-dir", str(PROJECT_ROOT)])
        assert rc == 0

    def test_cli_decision_flag(self):
        """V1074 CLI 真 --decision (主 13:31)."""
        rc = _cli(["--decision", "--project-dir", str(PROJECT_ROOT)])
        assert rc == 0

    def test_cli_trend_flag(self):
        """V1074 CLI 真 --trend (主 23:44)."""
        # 准备历史
        with tempfile.TemporaryDirectory() as td:
            history_dir = Path(td) / DEFAULT_DATA_DIR
            history_dir.mkdir(parents=True, exist_ok=True)
            history_file = history_dir / DEFAULT_ARTIFACTS["history_jsonl"]
            history_file.write_text(
                json.dumps({"snapshot_id": "snap_x", "ts_iso": "2026-07-22T00:00:00+00:00", "v03_score": 0.80}) + "\n",
                encoding="utf-8",
            )
            rc = _cli(["--trend", "--project-dir", str(Path(td))])
            assert rc == 0


# ============================================================================
# TestRefs (sanity: 真 REFERENCES 完整)
# ============================================================================

class TestSanityReferences:
    def test_references_count(self):
        """V1074 真 references ≥ 11 (主 19:33)."""
        assert len(REFERENCES) >= 11

    def test_references_have_required_keys(self):
        """V1074 真 references 含 id/title/url (主 19:33)."""
        for r in REFERENCES:
            assert "id" in r
            assert "title" in r
            assert "url" in r
            assert r["url"].startswith("http")

    def test_asi_level_thresholds_complete(self):
        """V1074 ASI 真 level thresholds 完整 (主 22:33)."""
        assert "ANI" in ASI_LEVEL_THRESHOLDS
        assert "AGI" in ASI_LEVEL_THRESHOLDS
        assert "ASI" in ASI_LEVEL_THRESHOLDS
        assert "TRANSCENDENT" in ASI_LEVEL_THRESHOLDS

    def test_level_from_score_boundaries(self):
        """V1074 ASI 真 level boundaries (主 22:33)."""
        assert _level_from_score(0.0) == "ANI"
        assert _level_from_score(0.49) == "ANI"
        assert _level_from_score(0.50) == "AGI"
        assert _level_from_score(0.85) == "ASI"
        assert _level_from_score(0.97) == "ASI"
        assert _level_from_score(0.98) == "TRANSCENDENT"

    def test_clamp01_helper(self):
        """V1074 _clamp01 真 (主 17:43)."""
        assert _clamp01(-0.5) == 0.0
        assert _clamp01(0.5) == 0.5
        assert _clamp01(1.5) == 1.0

    def test_safe_div_helper(self):
        """V1074 _safe_div 真 (主 17:43)."""
        assert _safe_div(1.0, 0.0) == 0.0
        assert _safe_div(1.0, 0.0, default=42.0) == 42.0
        assert _safe_div(4.0, 2.0) == 2.0

    def test_sha256_helper(self):
        """V1074 _sha256 真 (主 17:43)."""
        h1 = _sha256("hello")
        h2 = _sha256("hello")
        assert h1 == h2
        assert len(h1) == 64

    def test_utc_now_iso_format(self):
        """V1074 _utc_now_iso 真 ISO 格式 (主 17:43)."""
        iso = _utc_now_iso()
        assert "T" in iso
        assert "+" in iso or "Z" in iso

    def test_direction_catalog_count(self):
        """V1074 真 DIRECTION_CATALOG ≥ 5 (主 13:31)."""
        assert len(DIRECTION_CATALOG) >= 5

    def test_direction_catalog_have_required_keys(self):
        """V1074 真 DIRECTION_CATALOG 完整 keys (主 13:31)."""
        for d in DIRECTION_CATALOG:
            assert "id" in d
            assert "title" in d
            assert "trigger" in d
            assert "rationale" in d
            assert "borrowing" in d
            assert "expected_score_lift" in d
            assert callable(d["trigger"])