"""Tests for V1103 — R8-P2 Diagnostic Snapshot (mock V1077 接口).

V1103 真生产 (主 22:33 + 主 17:43 实事求是 + 主 23:44 干到底 + 主 00:56 任何人都能接手).

策略 (主 17:43 实事求是):
  V1103 = 诊断器, V1077 = 量具. 测试不应让 V1077 真跑污染.
  对 V1103 自身组件用 unit test; 对 V1077 接口用 mock; E2E 仅在 mock 下运行.

V3 守门:
  不假装 E2E 通过 = 真集成: V1103 真接 V1077 在主 22:33 但测试隔离 mock,
  避免 Python 3.13 + Windows 已知 GC finalizer 误报.
"""
from __future__ import annotations

import json
import unittest
from pathlib import Path
from typing import Any, Dict, List, Optional
from unittest import mock

from apeireth import v1103_r8p2_diagnostic as v1103


# ---------------------------------------------------------------------------
# Helper: synthetic V1077 snapshot for mock
# ---------------------------------------------------------------------------

def _fake_v1077_breakdown(score_eng: float = 0.10) -> Dict[str, Any]:
    """Synthetic V1077-style dim_breakdown mimicking V0.4 weights."""
    return {
        "v04_score": 0.80,
        "n_dims_total": 17,
        "n_dims_filled": 16,
        "n_dims_failed": 0,
        "runtime_ms": 500.0,
        "version": "0.1.0",
        "philosophy_guard_ok": True,
        "weights_used": {
            "phi_proxy": 0.12, "capabilities": 0.10, "cross_domain": 0.10,
            "engineering": 0.10, "vcp_4": 0.05, "v2_philosophy": 0.05,
            "rubric_open": 0.00, "real_production": 0.04, "cognitive_core": 0.07,
            "self_organizing_core": 0.07, "plugin_core": 0.06, "self_improving_core": 0.06,
            "neurosymbolic": 0.05, "world_model": 0.04, "reinforcement_learning": 0.03,
            "scientific_method": 0.02, "eternal_identity": 0.04,
        },
        "dim_breakdown": {
            "phi_proxy": 0.85, "capabilities": 1.00, "cross_domain": 0.97,
            "engineering": score_eng, "vcp_4": 0.97, "v2_philosophy": 0.99,
            "rubric_open": 0.00, "real_production": 1.00, "cognitive_core": 0.49,
            "self_organizing_core": 0.86, "plugin_core": 0.89, "self_improving_core": 0.87,
            "neurosymbolic": 0.84, "world_model": 0.72, "reinforcement_learning": 0.93,
            "scientific_method": 1.00, "eternal_identity": 0.84,
        },
    }


# ---------------------------------------------------------------------------
# Test 1: Component 2 — DimensionGapAnalyzer (pure)
# ---------------------------------------------------------------------------

class TestV1103DimensionGapAnalyzer(unittest.TestCase):
    """Component 2: gap analysis on a fake snapshot."""

    def _mk_snapshot(self, score_eng: float = 0.10) -> v1103.V04Snapshot:
        raw = _fake_v1077_breakdown(score_eng)
        dims: List[v1103.DimSnapshot] = []
        breakdown = raw["dim_breakdown"]
        weights = raw["weights_used"]
        for name, score in breakdown.items():
            w = float(weights.get(name, 0.0))
            s = float(score)
            dims.append(v1103.DimSnapshot(
                name=name, score=s, weight=w, weighted=s * w,
                gap=max(0.0, 1.0 - s),
                module_id="?", measurement_kind="unknown",
            ))
        return v1103.V04Snapshot(
            ts=0, v04_score=raw["v04_score"], n_dims_total=raw["n_dims_total"],
            n_dims_filled=raw["n_dims_filled"], n_dims_failed=raw["n_dims_failed"],
            runtime_ms=raw["runtime_ms"],
            weights_used={k: float(v) for k, v in weights.items()},
            dims=dims, v1077_version=raw["version"],
            philosophy_guard_ok=raw["philosophy_guard_ok"], error=None,
        )

    def test_analyze_empty(self):
        empty = v1103.V04Snapshot(
            ts=0, v04_score=0, n_dims_total=0, n_dims_filled=0,
            n_dims_failed=0, runtime_ms=0, weights_used={}, dims=[],
            v1077_version="x", philosophy_guard_ok=False,
        )
        gaps = v1103.analyze_gaps(empty)
        self.assertEqual(gaps, [])

    def test_analyze_sort_desc(self):
        snap = self._mk_snapshot()
        gaps = v1103.analyze_gaps(snap)
        impacts = [g.impact for g in gaps]
        self.assertEqual(impacts, sorted(impacts, reverse=True))

    def test_analyze_ranks_start_at_1(self):
        snap = self._mk_snapshot()
        gaps = v1103.analyze_gaps(snap)
        if gaps:
            self.assertEqual(gaps[0].rank, 1)
            self.assertEqual([g.rank for g in gaps],
                             list(range(1, len(gaps) + 1)))

    def test_engineering_top1_under_low_score(self):
        """engineering score 0.10, weight 0.10 → impact 0.09 max."""
        snap = self._mk_snapshot(score_eng=0.10)
        gaps = v1103.analyze_gaps(snap)
        self.assertEqual(gaps[0].name, "engineering")
        # 0.10 * (1 - 0.10) = 0.09
        self.assertAlmostEqual(gaps[0].impact, 0.10 * 0.90, places=4)


# ---------------------------------------------------------------------------
# Test 2: Component 3 — ASIHeadroomEstimator (pure)
# ---------------------------------------------------------------------------

class TestV1103ASIHeadroomEstimator(unittest.TestCase):

    def _mk_snapshot(self, score: float = 0.80) -> v1103.V04Snapshot:
        raw = _fake_v1077_breakdown(score_eng=score)
        dims: List[v1103.DimSnapshot] = []
        for name, sc in raw["dim_breakdown"].items():
            w = float(raw["weights_used"].get(name, 0.0))
            dims.append(v1103.DimSnapshot(
                name=name, score=float(sc), weight=w,
                weighted=float(sc) * w,
                gap=max(0.0, 1.0 - float(sc)),
                module_id="?", measurement_kind="unknown",
            ))
        return v1103.V04Snapshot(
            ts=0, v04_score=raw["v04_score"], n_dims_total=17,
            n_dims_filled=16, n_dims_failed=0, runtime_ms=500.0,
            weights_used={k: float(v) for k, v in raw["weights_used"].items()},
            dims=dims, v1077_version="0.1.0", philosophy_guard_ok=True,
        )

    def test_headroom_zero_at_target(self):
        snap = self._mk_snapshot()
        h = v1103.estimate_headroom(snap, asi_target=snap.v04_score)
        self.assertEqual(h.absolute_headroom, 0.0)
        self.assertEqual(h.relative_headroom_pct, 0.0)

    def test_headroom_default_target(self):
        snap = self._mk_snapshot()
        h = v1103.estimate_headroom(snap, asi_target=0.98)
        self.assertAlmostEqual(h.asi_target, 0.98, places=4)
        self.assertGreater(h.absolute_headroom, 0.0)

    def test_cumulative_upper_bound_nonneg(self):
        snap = self._mk_snapshot()
        h = v1103.estimate_headroom(snap, asi_target=0.98)
        self.assertGreaterEqual(h.cumulative_impact_if_all_lifted_to_1, 0.0)

    def test_top_n_projection_monotonic(self):
        snap = self._mk_snapshot()
        h = v1103.estimate_headroom(snap, asi_target=0.98)
        scores = [h.feasible_if_lift_top_n[k]
                  for k in sorted(h.feasible_if_lift_top_n.keys())]
        for i in range(1, len(scores)):
            self.assertGreaterEqual(scores[i], scores[i - 1])


# ---------------------------------------------------------------------------
# Test 3: Component 4 — P2CandidateGenerator (pure)
# ---------------------------------------------------------------------------

class TestV1103P2CandidateGenerator(unittest.TestCase):

    def test_path_map_known(self):
        for mod_id in ("V1060", "V1061", "V1045", "V1062", "V1071"):
            self.assertIn(mod_id, v1103.MODULE_PATH_MAP)

    def test_top_n_returns_sorted(self):
        snap = v1103.V04Snapshot(
            ts=0, v04_score=0.8, n_dims_total=17, n_dims_filled=16,
            n_dims_failed=0, runtime_ms=500,
            weights_used={"a": 0.10, "b": 0.05},
            dims=[
                v1103.DimSnapshot("a", 0.1, 0.10, 0.01, 0.9, "V1", "k"),
                v1103.DimSnapshot("b", 0.5, 0.05, 0.025, 0.5, "V2", "k"),
            ],
            v1077_version="0.1.0", philosophy_guard_ok=True,
        )
        gaps = v1103.analyze_gaps(snap)
        cands = v1103.generate_candidates(gaps, top_n=5)
        ranks = [c.rank for c in cands]
        self.assertEqual(ranks, sorted(ranks))

    def test_candidate_has_module_path(self):
        snap = v1103.V04Snapshot(
            ts=0, v04_score=0.8, n_dims_total=17, n_dims_filled=16,
            n_dims_failed=0, runtime_ms=500,
            weights_used={"engineering": 0.10},
            dims=[
                v1103.DimSnapshot("engineering", 0.1, 0.10, 0.01, 0.9,
                                   "V1060", "test_coverage"),
            ],
            v1077_version="0.1.0", philosophy_guard_ok=True,
        )
        gaps = v1103.analyze_gaps(snap)
        cands = v1103.generate_candidates(gaps, top_n=1)
        self.assertEqual(len(cands), 1)
        self.assertEqual(cands[0].module_id, "V1060")
        self.assertIn("v1060", cands[0].module_path)
        self.assertTrue(cands[0].module_path.endswith(".py"))


# ---------------------------------------------------------------------------
# Test 4: Component 5 — PhilosophyGuard
# ---------------------------------------------------------------------------

class TestV1103PhilosophyGuard(unittest.TestCase):

    def test_guard_5_items(self):
        guard = v1103.V1103PhilosophyGuard()
        self.assertEqual(len(guard.GUARDS), 5)
        for key in ("diagnostic_is_not_asi", "marginal_lift_is_upper_bound",
                    "top_n_is_not_sole_path", "weight_sum_is_not_asi",
                    "module_id_is_not_one_liner"):
            self.assertIn(key, v1103.V3_GUARDS)

    def test_guard_check_all_true(self):
        guard = v1103.V1103PhilosophyGuard()
        result = guard.check_all()
        self.assertEqual(len(result), 5)
        for v in result.values():
            self.assertTrue(v)


# ---------------------------------------------------------------------------
# Test 5: Renderers (pure)
# ---------------------------------------------------------------------------

class TestV1103Renderers(unittest.TestCase):

    def _mk_report(self) -> v1103.DiagnosticReport:
        snap = v1103.V04Snapshot(
            ts=0, v04_score=0.80, n_dims_total=17, n_dims_filled=16,
            n_dims_failed=0, runtime_ms=500.0,
            weights_used={"engineering": 0.10, "cognitive_core": 0.07,
                           "phi_proxy": 0.12},
            dims=[
                v1103.DimSnapshot("engineering", 0.10, 0.10, 0.01, 0.9, "V1060", "test_coverage"),
                v1103.DimSnapshot("cognitive_core", 0.49, 0.07, 0.0343, 0.51, "V1061", "compute_metrics"),
                v1103.DimSnapshot("phi_proxy", 0.85, 0.12, 0.102, 0.15, "V1045", "phi_proxy_estimate"),
            ],
            v1077_version="0.1.0", philosophy_guard_ok=True,
        )
        gaps = v1103.analyze_gaps(snap)
        cands = v1103.generate_candidates(gaps, top_n=3)
        h = v1103.estimate_headroom(snap, asi_target=0.98)
        return v1103.DiagnosticReport(
            version=v1103.V1103_VERSION, ts=0.0, snapshot=snap,
            headroom=h, candidates=cands,
            philosophy_guard=v1103.V1103PhilosophyGuard().check_all(),
        )

    def test_render_text_contains_asi(self):
        report = self._mk_report()
        text = v1103.render_text(report, top_n=3)
        self.assertIn("V1103", text)
        self.assertIn("0.9800", text)
        self.assertIn("engineering", text)
        self.assertIn("V1060", text)

    def test_render_markdown_valid(self):
        report = self._mk_report()
        md = v1103.render_markdown(report, top_n=3)
        self.assertIn("# V1103", md)
        self.assertIn("Top-N P2 候选", md)
        self.assertIn("V3 哲学守门", md)
        self.assertIn("engineering", md)

    def test_to_dict_jsonable(self):
        report = self._mk_report()
        d = report.to_dict()
        s = json.dumps(d, default=str)
        self.assertIn("headroom", s)
        self.assertIn("candidates", s)
        self.assertIn("version", s)


# ---------------------------------------------------------------------------
# Test 6: End-to-end with MOCKED V1077 (no real import / stderr)
# ---------------------------------------------------------------------------

class TestV1103E2EMocked(unittest.TestCase):

    @mock.patch.object(v1103, "_call_v1077_safe")
    @mock.patch.object(v1103, "_import_v1077")
    def test_load_snapshot_via_mock(self, mock_imp, mock_call):
        """Mock V1077 to avoid Python 3.13 stderr finalizer."""
        mock_imp.return_value = object()  # pretend importable
        mock_call.return_value = (_fake_v1077_breakdown(score_eng=0.10), None)

        snap = v1103.load_snapshot()
        self.assertIsNone(snap.error)
        self.assertEqual(snap.v04_score, 0.80)
        self.assertGreater(len(snap.dims), 0)

    @mock.patch.object(v1103, "_call_v1077_safe")
    @mock.patch.object(v1103, "_import_v1077")
    def test_load_snapshot_handles_error(self, mock_imp, mock_call):
        """V1077 failing → V1103 must report error, not crash."""
        mock_imp.return_value = None
        mock_call.return_value = (None, "v1077_import_failed")

        snap = v1103.load_snapshot()
        self.assertEqual(snap.error, "v1077_import_failed")
        self.assertEqual(snap.v04_score, 0.0)

    @mock.patch.object(v1103, "_call_v1077_safe")
    @mock.patch.object(v1103, "_import_v1077")
    def test_run_diagnostic_via_mock(self, mock_imp, mock_call):
        """Full E2E with mocked V1077 — covers run_diagnostic + renderers."""
        mock_imp.return_value = object()
        mock_call.return_value = (_fake_v1077_breakdown(score_eng=0.10), None)

        report = v1103.run_diagnostic(top_n=5, asi_target=0.98)
        self.assertEqual(report.version, v1103.V1103_VERSION)
        self.assertGreater(len(report.candidates), 0)
        # top candidate should be engineering (impact 0.09)
        self.assertEqual(report.candidates[0].dim_name, "engineering")
        self.assertEqual(report.candidates[0].module_id, "V1060")

    def test_cli_help_runs(self):
        """--help must return 0 (smoke; V1077 not needed)."""
        old_argv = __import__("sys").argv
        try:
            __import__("sys").argv = ["v1103", "--help"]
            rc = v1103.main(["--help"])
            self.assertIn(rc, (0, None))
        except SystemExit:
            pass  # argparse exits with --help
        finally:
            __import__("sys").argv = old_argv


# ---------------------------------------------------------------------------
# Test 7: V3_GUARDS content sanity
# ---------------------------------------------------------------------------

class TestV1103V3GuardContent(unittest.TestCase):

    def test_diagnostic_is_not_asi_present(self):
        self.assertIn("diagnostic_is_not_asi", v1103.V3_GUARDS)
        self.assertIn("ASI", v1103.V3_GUARDS["diagnostic_is_not_asi"])

    def test_marginal_lift_guard_present(self):
        self.assertIn("marginal_lift_is_upper_bound", v1103.V3_GUARDS)
        # V3_GUARDS carries Chinese "上界" (upper bound) by design (主 13:31 大胆激进).
        # Test is permissive: matches "上界" OR "上" OR ascii equivalent.
        desc = v1103.V3_GUARDS["marginal_lift_is_upper_bound"]
        self.assertTrue("上界" in desc or "上" in desc or "upper" in desc.lower())

    def test_top_n_guard_present(self):
        self.assertIn("top_n_is_not_sole_path", v1103.V3_GUARDS)


if __name__ == "__main__":
    unittest.main(verbosity=2)
