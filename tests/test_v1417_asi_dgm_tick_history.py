"""Tests for V1417 — ASI 总框架 DGM tick history (JSONL log + trend + digest + baseline + compare).

Phase: 1417
Version: 0.1.0
Date: 2026-08-10 (cron tick 02:40, Asia/Shanghai deep night)
Post: V1416 (DGM closed-loop tick executor)

Test sections (12 — 主 17:43 实事求是):
1.  TestConstants (1)              — VERSION/SCHEMA/MODULE/GUARDS/V3_GUARDS/BORROWED/POLICIES/TRENDS
2.  TestDataclasses (2)           — TickSnapshot + TickTrend + TickDigest + TickBaseline + TickCompare roundtrips
3.  TestHelpers (3)               — _slug_timestamp + _parse_iso_timestamp + _safe_path + _now_utc_iso + _atomic_append_jsonl
4.  TestLoadAndAppend (4)         — load_tick_history + load_v1416_ticks + append_tick_snapshot (roundtrip + missing)
5.  TestComputeTrend (5)          — INSUFFICIENT/IMPROVING/STABLE/DEGRADING/INSUFFICIENT (5 branches)
6.  TestComputeDigest (6)         — n_ticks=0 + n_ticks>0 + policy dist + averages + chain_ok rate
7.  TestBaselineCompare (6)       — make + save + load + compare REGRESSION/UNCHANGED/IMPROVEMENT
8.  TestRender (1)                — 8 markdown sections + honest disclosure
9.  TestPopper (1)                — 15/15 self-test pass
10. TestChainDelegate (1)         — V1416 chain_delegate_v1416 probe (read-only)
11. TestCLI (10)                  — version/policy/popper/meta/demo/snapshot/list/trend/digest/baseline/compare/render/chain/help
12. TestIntegration (2)           — reads real V1416 tick log + end-to-end pipeline

Total: ~42 tests
"""

from __future__ import annotations

import dataclasses
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

# Ensure workspace root is importable
WORKSPACE = Path(__file__).resolve().parents[1]
if str(WORKSPACE) not in sys.path:
    sys.path.insert(0, str(WORKSPACE))

import apeireth.v1417_asi_dgm_tick_history as m


# ============================================================================
# 1. TestConstants
# ============================================================================


class TestConstants(unittest.TestCase):
    def test_constants_present(self):
        self.assertTrue(m.V1417_VERSION)
        self.assertTrue(m.V1417_SCHEMA)
        self.assertTrue(m.V1417_MODULE)
        self.assertEqual(len(m.V1417_GUARDS), 15)
        self.assertEqual(len(m.V1417_V3_GUARDS), 9)
        self.assertEqual(len(m.V1417_BORROWED), 4)
        self.assertEqual(set(m.V1417_POLICIES), {"PROCEED", "PAUSE", "LOCKDOWN"})
        self.assertEqual(set(m.V1417_TRENDS), {"IMPROVING", "STABLE", "DEGRADING", "INSUFFICIENT"})


# ============================================================================
# 2. TestDataclasses
# ============================================================================


class TestDataclasses(unittest.TestCase):
    def test_tick_snapshot_roundtrip(self):
        s = m.TickSnapshot(
            timestamp="2026-08-10T00-00-00Z",
            tick_id="t1",
            policy="PROCEED",
            chain_ok=True,
            alerts_count=0,
            max_severity="INFO",
            escalation_count=0,
            n_modules=5,
            n_snapshots_v1415=0,
            note="x",
        )
        s2 = m.TickSnapshot(**dataclasses.asdict(s))
        self.assertEqual(s, s2)

    def test_all_dataclass_roundtrips(self):
        snap = m.TickSnapshot("2026-08-10T00-00-00Z", "t", "PROCEED", True, 0, "INFO", 0, 5, 0, "n")
        trend = m.TickTrend(
            direction="STABLE",
            n_snapshots=1,
            delta_alerts=0,
            delta_escalation=0,
            first_policy="PROCEED",
            last_policy="PROCEED",
            first_timestamp="2026-08-10T00-00-00Z",
            last_timestamp="2026-08-10T00-00-00Z",
            proceed_ratio=1.0,
            chain_ok_rate=1.0,
            degradation_flag=False,
            reason="x",
        )
        digest = m.TickDigest(
            n_ticks=1, policy_proceed=1, policy_pause=0, policy_lockdown=0,
            proceed_ratio=1.0, pause_ratio=0.0, lockdown_ratio=0.0,
            alerts_total=0, alerts_avg=0.0, escalation_total=0, escalation_avg=0.0,
            chain_ok_count=1, chain_ok_rate=1.0,
            first_timestamp="2026-08-10T00-00-00Z", last_timestamp="2026-08-10T00-00-00Z",
            span_seconds=0, note="x",
        )
        baseline = m.TickBaseline(
            baseline_timestamp="2026-08-10T00-00-00Z",
            policy="PROCEED",
            chain_ok=True,
            alerts_count=0,
            escalation_count=0,
            note="x",
            source_snapshot_index=0,
            created_at="2026-08-10T00-00-00Z",
        )
        cmp_res = m.TickCompare(
            baseline_timestamp="2026-08-10T00-00-00Z",
            current_timestamp="2026-08-10T00-00-00Z",
            delta_alerts=0,
            delta_escalation=0,
            policy_regressed=False,
            policy_improved=False,
            chain_ok_regressed=False,
            verdict="UNCHANGED",
            reasons="none",
            note="x",
        )
        for d in (snap, trend, digest, baseline, cmp_res):
            cls = type(d)
            d2 = cls(**dataclasses.asdict(d))
            self.assertEqual(d, d2)


# ============================================================================
# 3. TestHelpers
# ============================================================================


class TestHelpers(unittest.TestCase):
    def test_slug_timestamp(self):
        self.assertEqual(m._slug_timestamp("2026:08:10"), "2026-08-10")
        self.assertEqual(m._slug_timestamp("2026/08/10"), "2026-08-10")
        self.assertEqual(m._slug_timestamp("2026-08-10"), "2026-08-10")

    def test_parse_iso_timestamp(self):
        from datetime import datetime
        dt = m._parse_iso_timestamp("2026-08-10T12-34-56Z")
        self.assertIsNotNone(dt)
        self.assertEqual(dt.year, 2026)
        self.assertEqual(dt.month, 8)
        self.assertEqual(dt.day, 10)
        self.assertIsNone(m._parse_iso_timestamp(""))
        self.assertIsNone(m._parse_iso_timestamp("garbage"))

    def test_safe_path_rejects_dotdot(self):
        with self.assertRaises(ValueError):
            m._safe_path(Path("a/../b"))
        # absolute paths OK
        p = m._safe_path(Path("/tmp/x"))
        self.assertEqual(str(p), str(Path("/tmp/x")))


# ============================================================================
# 4. TestLoadAndAppend
# ============================================================================


class TestLoadAndAppend(unittest.TestCase):
    def test_load_tick_history_missing(self):
        snaps = m.load_tick_history(Path("/nonexistent/path.jsonl"))
        self.assertEqual(snaps, [])

    def test_load_tick_history_skips_invalid(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp) / "h.jsonl"
            p.write_text(
                '{"timestamp":"t1","tick_id":"a","policy":"PROCEED","chain_ok":true,"alerts_count":0,"max_severity":"INFO","escalation_count":0,"n_modules":5,"n_snapshots_v1415":0,"note":""}\n'
                "this is not json\n"
                '{"timestamp":"t2","tick_id":"b","policy":"PAUSE","chain_ok":false,"alerts_count":1,"max_severity":"WARN","escalation_count":0,"n_modules":5,"n_snapshots_v1415":0,"note":"x"}\n'
                "\n"
            )
            snaps = m.load_tick_history(p)
            self.assertEqual(len(snaps), 2)
            self.assertEqual(snaps[0].tick_id, "a")
            self.assertEqual(snaps[1].policy, "PAUSE")

    def test_append_tick_snapshot_roundtrip(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp) / "h.jsonl"
            snap = m.TickSnapshot("2026-08-10T00-00-00Z", "t1", "PROCEED", True, 0, "INFO", 0, 5, 0, "n")
            self.assertTrue(m.append_tick_snapshot(snap, p))
            self.assertTrue(p.exists())
            snaps = m.load_tick_history(p)
            self.assertEqual(len(snaps), 1)
            self.assertEqual(snaps[0], snap)

    def test_load_v1416_ticks(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp) / "v1416.jsonl"
            p.write_text(
                json.dumps({
                    "schema": "v1416.asi-overarching-dgm-tick/v1",
                    "tick_id": "abc",
                    "timestamp": "2026-08-10T00-00-00Z",
                    "v1413_snapshot_id": "s1",
                    "v1414_alerts_count": 0,
                    "v1414_max_severity": "INFO",
                    "v1415_overall_max_severity": "INFO",
                    "v1415_escalation_count": 0,
                    "v1415_n_snapshots": 3,
                    "policy": "PROCEED",
                    "policy_reason": "ok",
                    "chain_ok": True,
                    "n_modules": 5,
                    "note": "x",
                }) + "\n"
            )
            snaps = m.load_v1416_ticks(p)
            self.assertEqual(len(snaps), 1)
            self.assertEqual(snaps[0].tick_id, "abc")
            self.assertEqual(snaps[0].policy, "PROCEED")
            self.assertEqual(snaps[0].n_snapshots_v1415, 3)


# ============================================================================
# 5. TestComputeTrend
# ============================================================================


class TestComputeTrend(unittest.TestCase):
    def test_trend_insufficient_on_empty(self):
        t = m.compute_tick_trend([])
        self.assertEqual(t.direction, "INSUFFICIENT")
        self.assertEqual(t.n_snapshots, 0)

    def test_trend_insufficient_on_one(self):
        s = m.TickSnapshot("2026-08-10T00-00-00Z", "t", "PROCEED", True, 0, "INFO", 0, 5, 0, "")
        t = m.compute_tick_trend([s])
        self.assertEqual(t.direction, "INSUFFICIENT")

    def test_trend_improving_on_all_proceed(self):
        snaps = [m.TickSnapshot(f"2026-08-10T00-0{i}-00Z", f"t{i}", "PROCEED", True, 0, "INFO", 0, 5, 0, "") for i in range(5)]
        t = m.compute_tick_trend(snaps)
        self.assertEqual(t.direction, "IMPROVING")
        self.assertEqual(t.proceed_ratio, 1.0)
        self.assertEqual(t.chain_ok_rate, 1.0)
        self.assertFalse(t.degradation_flag)

    def test_trend_degrading_on_lockdown(self):
        snaps = [m.TickSnapshot(f"2026-08-10T00-0{i}-00Z", f"t{i}", "LOCKDOWN", False, 5, "CRITICAL", 1, 5, 0, "") for i in range(5)]
        t = m.compute_tick_trend(snaps)
        self.assertEqual(t.direction, "DEGRADING")
        self.assertTrue(t.degradation_flag)
        self.assertLess(t.proceed_ratio, 0.5)

    def test_trend_stable_or_better_on_mixed(self):
        snaps = (
            [m.TickSnapshot(f"2026-08-10T00-0{i}-00Z", f"t{i}", "PROCEED", True, 0, "INFO", 0, 5, 0, "") for i in range(4)]
            + [m.TickSnapshot("2026-08-10T00-04-00Z", "t5", "PAUSE", True, 1, "WARN", 0, 5, 0, "")]
        )
        t = m.compute_tick_trend(snaps)
        self.assertIn(t.direction, ("STABLE", "IMPROVING"))


# ============================================================================
# 6. TestComputeDigest
# ============================================================================


class TestComputeDigest(unittest.TestCase):
    def test_digest_empty(self):
        d = m.compute_tick_digest([])
        self.assertEqual(d.n_ticks, 0)
        self.assertEqual(d.policy_proceed, 0)
        self.assertEqual(d.chain_ok_rate, 0.0)

    def test_digest_aggregates(self):
        snaps = (
            [m.TickSnapshot("2026-08-10T00-00-00Z", f"t{i}", "PROCEED", True, 1, "INFO", 0, 5, 0, "") for i in range(3)]
            + [m.TickSnapshot("2026-08-10T00-03-00Z", "t4", "PAUSE", False, 2, "WARN", 1, 5, 0, "")]
        )
        d = m.compute_tick_digest(snaps)
        self.assertEqual(d.n_ticks, 4)
        self.assertEqual(d.policy_proceed, 3)
        self.assertEqual(d.policy_pause, 1)
        self.assertEqual(d.policy_lockdown, 0)
        self.assertEqual(d.alerts_total, 5)
        self.assertEqual(d.alerts_avg, 1.25)
        self.assertEqual(d.escalation_total, 1)
        self.assertEqual(d.chain_ok_count, 3)
        self.assertEqual(d.chain_ok_rate, 0.75)


# ============================================================================
# 7. TestBaselineCompare
# ============================================================================


class TestBaselineCompare(unittest.TestCase):
    def test_make_baseline_from_index(self):
        snaps = [
            m.TickSnapshot("2026-08-10T00-00-00Z", "t1", "PROCEED", True, 0, "INFO", 0, 5, 0, ""),
            m.TickSnapshot("2026-08-10T00-01-00Z", "t2", "PROCEED", True, 1, "INFO", 0, 5, 0, ""),
        ]
        b = m.make_tick_baseline(snaps, snapshot_index=0, note="first")
        self.assertEqual(b.baseline_timestamp, "2026-08-10T00-00-00Z")
        self.assertEqual(b.source_snapshot_index, 0)

    def test_make_baseline_default_last(self):
        snaps = [
            m.TickSnapshot("2026-08-10T00-00-00Z", "t1", "PROCEED", True, 0, "INFO", 0, 5, 0, ""),
            m.TickSnapshot("2026-08-10T00-01-00Z", "t2", "PROCEED", True, 1, "INFO", 0, 5, 0, ""),
        ]
        b = m.make_tick_baseline(snaps, note="last")
        self.assertEqual(b.source_snapshot_index, 1)

    def test_make_baseline_empty_raises(self):
        with self.assertRaises(ValueError):
            m.make_tick_baseline([])

    def test_save_load_baseline_roundtrip(self):
        with tempfile.TemporaryDirectory() as tmp:
            bp = Path(tmp) / "b.json"
            b = m.TickBaseline(
                baseline_timestamp="2026-08-10T00-00-00Z",
                policy="PROCEED",
                chain_ok=True,
                alerts_count=0,
                escalation_count=0,
                note="x",
                source_snapshot_index=0,
                created_at="2026-08-10T00-00-00Z",
            )
            self.assertTrue(m.save_baseline(b, bp))
            b2 = m.load_baseline(bp)
            self.assertEqual(b, b2)

    def test_compare_regression_on_lockdown(self):
        baseline = m.TickBaseline(
            baseline_timestamp="2026-08-10T00-00-00Z",
            policy="PROCEED",
            chain_ok=True,
            alerts_count=0,
            escalation_count=0,
            note="x",
            source_snapshot_index=0,
            created_at="2026-08-10T00-00-00Z",
        )
        snap = m.TickSnapshot("2026-08-10T00-01-00Z", "t1", "LOCKDOWN", False, 5, "CRITICAL", 1, 5, 0, "")
        cmp_res = m.compare_to_baseline(snap, baseline)
        self.assertEqual(cmp_res.verdict, "REGRESSION")
        self.assertTrue(cmp_res.policy_regressed)

    def test_compare_unchanged_on_identical(self):
        baseline = m.TickBaseline(
            baseline_timestamp="2026-08-10T00-00-00Z",
            policy="PROCEED",
            chain_ok=True,
            alerts_count=0,
            escalation_count=0,
            note="x",
            source_snapshot_index=0,
            created_at="2026-08-10T00-00-00Z",
        )
        snap = m.TickSnapshot("2026-08-10T00-01-00Z", "t1", "PROCEED", True, 0, "INFO", 0, 5, 0, "")
        cmp_res = m.compare_to_baseline(snap, baseline)
        self.assertEqual(cmp_res.verdict, "UNCHANGED")
        self.assertFalse(cmp_res.policy_regressed)

    def test_compare_improvement_on_lower_alerts(self):
        baseline = m.TickBaseline(
            baseline_timestamp="2026-08-10T00-00-00Z",
            policy="PAUSE",
            chain_ok=True,
            alerts_count=5,
            escalation_count=0,
            note="x",
            source_snapshot_index=0,
            created_at="2026-08-10T00-00-00Z",
        )
        snap = m.TickSnapshot("2026-08-10T00-01-00Z", "t1", "PROCEED", True, 0, "INFO", 0, 5, 0, "")
        cmp_res = m.compare_to_baseline(snap, baseline)
        self.assertEqual(cmp_res.verdict, "IMPROVEMENT")
        self.assertEqual(cmp_res.delta_alerts, -5)
        self.assertTrue(cmp_res.policy_improved)


# ============================================================================
# 8. TestRender
# ============================================================================


class TestRender(unittest.TestCase):
    def test_render_markdown_sections(self):
        snaps = [m.TickSnapshot(f"2026-08-10T00-0{i}-00Z", f"t{i}", "PROCEED", True, 0, "INFO", 0, 5, 0, "") for i in range(3)]
        trend = m.compute_tick_trend(snaps)
        digest = m.compute_tick_digest(snaps)
        md = m.render_tick_history_md(snaps, trend, digest)
        for section in [
            "# V1417",
            "## 1. Summary",
            "## 2. 哲学守门",
            "## 3. Tick list",
            "## 4. Trend",
            "## 5. Digest",
            "## 8. Honest disclosure",
        ]:
            self.assertIn(section, md)


# ============================================================================
# 9. TestPopper
# ============================================================================


class TestPopper(unittest.TestCase):
    def test_popper_15_of_15(self):
        ok, msgs = m.popper_self_test()
        self.assertTrue(ok, msg=f"popper failed: {msgs}")
        # First message should be "popper: 15/15"
        self.assertIn("popper: 15/15", msgs[0])


# ============================================================================
# 10. TestChainDelegate
# ============================================================================


class TestChainDelegate(unittest.TestCase):
    def test_chain_probe(self):
        probe = m.chain_delegate()
        self.assertTrue(probe["all_ok"])
        self.assertEqual(probe["n_modules"], 1)
        self.assertEqual(probe["n_modules_ok"], 1)
        self.assertEqual(probe["errors"], [])


# ============================================================================
# 11. TestCLI
# ============================================================================


class TestCLI(unittest.TestCase):
    def test_version(self):
        from io import StringIO
        old = sys.stdout
        sys.stdout = StringIO()
        try:
            rc = m.run_cli(["version"])
        finally:
            sys.stdout = old
        self.assertEqual(rc, 0)

    def test_meta(self):
        from io import StringIO
        old = sys.stdout
        sys.stdout = StringIO()
        try:
            rc = m.run_cli(["meta", "--json"])
        finally:
            sys.stdout = old
        self.assertEqual(rc, 0)

    def test_meta_no_json(self):
        from io import StringIO
        old = sys.stdout
        sys.stdout = StringIO()
        try:
            rc = m.run_cli(["meta"])
        finally:
            sys.stdout = old
        self.assertEqual(rc, 0)

    def test_demo(self):
        from io import StringIO
        old = sys.stdout
        sys.stdout = StringIO()
        try:
            rc = m.run_cli(["demo"])
        finally:
            sys.stdout = old
        self.assertEqual(rc, 0)

    def test_help(self):
        from io import StringIO
        old = sys.stdout
        sys.stdout = StringIO()
        try:
            rc = m.run_cli(["help"])
        finally:
            sys.stdout = old
        self.assertEqual(rc, 0)

    def test_popper(self):
        from io import StringIO
        old = sys.stdout
        sys.stdout = StringIO()
        try:
            rc = m.run_cli(["popper"])
        finally:
            sys.stdout = old
        self.assertEqual(rc, 0)

    def test_chain(self):
        from io import StringIO
        old = sys.stdout
        sys.stdout = StringIO()
        try:
            rc = m.run_cli(["chain"])
        finally:
            sys.stdout = old
        self.assertEqual(rc, 0)

    def test_trend_and_digest(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp) / "h.jsonl"
            s = m.TickSnapshot("2026-08-10T00-00-00Z", "t1", "PROCEED", True, 0, "INFO", 0, 5, 0, "")
            m.append_tick_snapshot(s, p)
            from io import StringIO
            old = sys.stdout
            sys.stdout = StringIO()
            try:
                rc1 = m.run_cli(["trend", "--history-path", str(p)])
                rc2 = m.run_cli(["digest", "--history-path", str(p)])
                rc3 = m.run_cli(["list", "--history-path", str(p)])
            finally:
                sys.stdout = old
            self.assertEqual(rc1, 0)
            self.assertEqual(rc2, 0)
            self.assertEqual(rc3, 0)

    def test_render(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp) / "h.jsonl"
            out = Path(tmp) / "report.md"
            s = m.TickSnapshot("2026-08-10T00-00-00Z", "t1", "PROCEED", True, 0, "INFO", 0, 5, 0, "")
            m.append_tick_snapshot(s, p)
            from io import StringIO
            old = sys.stdout
            sys.stdout = StringIO()
            try:
                rc = m.run_cli(["render", "--history-path", str(p), "--out", str(out)])
            finally:
                sys.stdout = old
            self.assertEqual(rc, 0)
            self.assertTrue(out.exists())
            self.assertIn("V1417", out.read_text(encoding="utf-8"))

    def test_unknown_command(self):
        from io import StringIO
        old = sys.stdout
        sys.stdout = StringIO()
        old_err = sys.stderr
        sys.stderr = StringIO()
        try:
            rc = m.run_cli(["bogus"])
        finally:
            sys.stdout = old
            sys.stderr = old_err
        self.assertEqual(rc, 2)


# ============================================================================
# 12. TestIntegration
# ============================================================================


class TestIntegration(unittest.TestCase):
    def test_e2e_pipeline(self):
        """End-to-end: synthetic V1416 JSONL → load → append → trend → digest → baseline → compare → render."""
        with tempfile.TemporaryDirectory() as tmp:
            # 1. Build synthetic V1416 JSONL
            v1416_path = Path(tmp) / "v1416.jsonl"
            v1416_records = [
                {"timestamp": "2026-08-10T00-00-00Z", "tick_id": "t1", "v1414_alerts_count": 0, "v1414_max_severity": "INFO", "v1415_escalation_count": 0, "v1415_n_snapshots": 0, "policy": "PROCEED", "policy_reason": "ok", "chain_ok": True, "n_modules": 5, "note": "first"},
                {"timestamp": "2026-08-10T00-01-00Z", "tick_id": "t2", "v1414_alerts_count": 0, "v1414_max_severity": "INFO", "v1415_escalation_count": 0, "v1415_n_snapshots": 1, "policy": "PROCEED", "policy_reason": "ok", "chain_ok": True, "n_modules": 5, "note": "second"},
                {"timestamp": "2026-08-10T00-02-00Z", "tick_id": "t3", "v1414_alerts_count": 1, "v1414_max_severity": "WARN", "v1415_escalation_count": 0, "v1415_n_snapshots": 2, "policy": "PAUSE", "policy_reason": "warn", "chain_ok": True, "n_modules": 5, "note": "third"},
            ]
            v1416_path.write_text("\n".join(json.dumps(r) for r in v1416_records) + "\n", encoding="utf-8")

            # 2. Load V1416 ticks
            snaps = m.load_v1416_ticks(v1416_path)
            self.assertEqual(len(snaps), 3)

            # 3. Append to V1417 history
            history_path = Path(tmp) / "v1417.jsonl"
            for s in snaps:
                m.append_tick_snapshot(s, history_path)
            history = m.load_tick_history(history_path)
            self.assertEqual(len(history), 3)

            # 4. Trend + digest
            trend = m.compute_tick_trend(history)
            digest = m.compute_tick_digest(history)
            self.assertIn(trend.direction, ("IMPROVING", "STABLE", "DEGRADING"))
            self.assertEqual(digest.n_ticks, 3)
            self.assertEqual(digest.policy_proceed, 2)
            self.assertEqual(digest.policy_pause, 1)

            # 5. Baseline + compare
            baseline = m.make_tick_baseline(history, snapshot_index=0, note="e2e")
            baseline_path = Path(tmp) / "b.json"
            m.save_baseline(baseline, baseline_path)
            baseline_loaded = m.load_baseline(baseline_path)
            self.assertEqual(baseline, baseline_loaded)
            cmp_res = m.compare_to_baseline(history[-1], baseline)
            self.assertEqual(cmp_res.delta_alerts, 1)

            # 6. Render
            md = m.render_tick_history_md(history, trend, digest, baseline=baseline, compare=cmp_res)
            self.assertIn("V1417", md)
            self.assertIn("Trend", md)
            self.assertIn("Digest", md)
            self.assertIn("REGRESSION", md)  # baseline PROCEED → current PAUSE = REGRESSION

    def test_reads_real_v1416_tick_log(self):
        """If .v1416-dgm-ticks.jsonl exists at workspace root, V1417 can read it."""
        real_path = WORKSPACE / ".v1416-dgm-ticks.jsonl"
        if not real_path.exists():
            self.skipTest("no real V1416 tick log found")
        snaps = m.load_v1416_ticks(real_path)
        self.assertGreater(len(snaps), 0)
        for s in snaps[:5]:
            self.assertIn(s.policy, m.V1417_POLICIES)


if __name__ == "__main__":
    unittest.main()