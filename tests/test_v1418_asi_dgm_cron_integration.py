"""Tests for V1418 — ASI 总框架 DGM cron integration (5min cron auto-tick → append → render).

Phase: 1418
Version: 0.1.0
Date: 2026-08-10 (cron tick 02:50, Asia/Shanghai deep night)
Post: V1417 (DGM tick history)

Test sections (11 — 主 17:43 实事求是):
1.  TestConstants (1)             — VERSION/SCHEMA/MODULE/GUARDS/V3_GUARDS/BORROWED/POLICIES
2.  TestDataclasses (2)          — CronTickOutcome + CronSessionSummary roundtrips
3.  TestConfig (3)               — DEFAULT_CRON_CONFIG + build_default_config + bounds rejection
4.  TestComputeNextDue (3)       — deterministic + jitter + bounds rejection
5.  TestPathSafety (2)           — _safe_path + dotdot rejection + _parse_iso_timestamp
6.  TestPopper (1)               — popper_self_test = 15/15
7.  TestRenderSessionMd (2)      — 4 markdown sections + honest disclosure + N outcomes
8.  TestTickOnce (4)             — chain_ok + history append + render + cycle_index propagation
9.  TestRunSession (3)           — N cycles + summary fields + summary json written
10. TestChainDelegate (1)        — V1416 + V1417 chain_ok probe
11. TestCLI (12)                 — version/popper/chain/tick-once/run-session/next-due/render-summary/show-outcomes/detect-policy/emit-shell/help

Total: ~33 tests
"""

from __future__ import annotations

import dataclasses
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

# Ensure workspace root is importable
WORKSPACE = Path(__file__).resolve().parents[1]
if str(WORKSPACE) not in sys.path:
    sys.path.insert(0, str(WORKSPACE))

import apeireth.v1418_asi_dgm_cron_integration as m


# ============================================================================
# 1. TestConstants
# ============================================================================


class TestConstants(unittest.TestCase):
    def test_constants_present(self):
        self.assertTrue(m.V1418_VERSION)
        self.assertTrue(m.V1418_SCHEMA)
        self.assertTrue(m.V1418_MODULE)
        self.assertEqual(len(m.V1418_GUARDS), 15)
        self.assertEqual(len(m.V1418_V3_GUARDS), 9)
        self.assertEqual(len(m.V1418_BORROWED), 4)
        self.assertEqual(set(m.V1418_POLICIES), {"PROCEED", "PAUSE", "LOCKDOWN"})


# ============================================================================
# 2. TestDataclasses
# ============================================================================


class TestDataclasses(unittest.TestCase):
    def test_cron_tick_outcome_roundtrip(self):
        o = m.CronTickOutcome(
            cycle_index=1, ran_at_iso="2026-08-10T00-00-00Z", tick_id="t1",
            policy="PROCEED", chain_ok=True, alerts_count=0, escalation_count=0,
            n_modules=5, appended_to_history=True, rendered_path="/tmp/r.md",
            render_ok=True, note="x",
        )
        o2 = m.CronTickOutcome(**dataclasses.asdict(o))
        self.assertEqual(o, o2)

    def test_cron_session_summary_roundtrip(self):
        s = m.CronSessionSummary(
            n_cycles=2, n_policies=1, policy_proceed_count=2,
            policy_pause_count=0, policy_lockdown_count=0,
            chain_ok_count=2, chain_ok_rate=1.0,
            first_tick="t1", last_tick="t2", span_seconds=10,
            session_started_iso="2026-08-10T00-00-00Z",
            session_ended_iso="2026-08-10T00-00-10Z",
            rendered_path="", note="y",
        )
        s2 = m.CronSessionSummary(**dataclasses.asdict(s))
        self.assertEqual(s, s2)


# ============================================================================
# 3. TestConfig
# ============================================================================


class TestConfig(unittest.TestCase):
    def test_default_config_in_bounds(self):
        cfg = m.DEFAULT_CRON_CONFIG
        self.assertGreaterEqual(cfg.cadence_seconds, cfg.min_cadence_seconds)
        self.assertLessEqual(cfg.cadence_seconds, cfg.max_cadence_seconds)
        self.assertGreaterEqual(cfg.max_cycles, 1)
        self.assertLessEqual(cfg.max_cycles, m.MAX_CYCLES_PER_SESSION)

    def test_build_default_config_applies_overrides(self):
        overrides = {"cadence_seconds": 60, "jitter_seconds": 5, "note": "test"}
        cfg = m.build_default_config(overrides)
        self.assertEqual(cfg.cadence_seconds, 60)
        self.assertEqual(cfg.jitter_seconds, 5)
        self.assertEqual(cfg.note, "test")

    def test_build_default_config_rejects_unknown(self):
        with self.assertRaises(ValueError):
            m.build_default_config({"never_set_this": True})

    def test_config_rejects_out_of_bounds_cadence(self):
        with self.assertRaises(ValueError):
            m.CronIntegrationConfig(
                cadence_seconds=999_999_999, jitter_seconds=0,
                auto_render=True, render_out=Path("/tmp/r.md"),
                max_cycles=10, min_cadence_seconds=1, max_cadence_seconds=86400,
                sleep_fn_name="time.sleep",
            )

    def test_config_rejects_out_of_bounds_max_cycles(self):
        with self.assertRaises(ValueError):
            m.CronIntegrationConfig(
                cadence_seconds=60, jitter_seconds=0,
                auto_render=True, render_out=Path("/tmp/r.md"),
                max_cycles=999_999_999, min_cadence_seconds=1, max_cadence_seconds=86400,
                sleep_fn_name="time.sleep",
            )


# ============================================================================
# 4. TestComputeNextDue
# ============================================================================


class TestComputeNextDue(unittest.TestCase):
    def test_compute_next_due_deterministic(self):
        last_iso = "2026-08-10T00-00-00Z"
        due_iso = m.compute_next_due(last_iso, 300, 0)
        self.assertEqual(due_iso, "2026-08-10T00-05-00Z")

    def test_compute_next_due_with_jitter(self):
        last_iso = "2026-08-10T00-00-00Z"
        due_iso = m.compute_next_due(last_iso, 300, 7)
        self.assertEqual(due_iso, "2026-08-10T00-05-07Z")

    def test_compute_next_due_rejects_out_of_bounds_cadence(self):
        with self.assertRaises(ValueError):
            m.compute_next_due("2026-08-10T00-00-00Z", 0, 0)


# ============================================================================
# 5. TestPathSafety + Helpers
# ============================================================================


class TestPathSafetyAndHelpers(unittest.TestCase):
    def test_path_safety_rejects_dotdot(self):
        with self.assertRaises(ValueError):
            m._safe_path(Path("a/../b"))

    def test_path_safety_accepts_absolute(self):
        p = m._safe_path(Path("C:/tmp/r.md"))
        self.assertTrue(str(p).endswith("r.md"))

    def test_parse_iso_timestamp_validates(self):
        self.assertIsNotNone(m._parse_iso_timestamp("2026-08-10T00-00-00Z"))
        self.assertIsNone(m._parse_iso_timestamp("not-an-iso"))
        self.assertIsNone(m._parse_iso_timestamp(""))

    def test_now_utc_iso_format(self):
        iso = m._now_utc_iso()
        self.assertRegex(iso, r"^\d{4}-\d{2}-\d{2}T\d{2}-\d{2}-\d{2}Z$")


# ============================================================================
# 6. TestPopper
# ============================================================================


class TestPopper(unittest.TestCase):
    def test_popper_self_test_passes(self):
        ok, msgs = m.popper_self_test()
        # 15/15 expected
        self.assertTrue(ok, msg=f"popper failed: {msgs}")
        first = msgs[0] if msgs else ""
        self.assertIn("15", first)


# ============================================================================
# 7. TestRenderSessionMd
# ============================================================================


class TestRenderSessionMd(unittest.TestCase):
    def test_render_session_md_contains_4_sections(self):
        s = m.CronSessionSummary(
            n_cycles=1, n_policies=1, policy_proceed_count=1,
            policy_pause_count=0, policy_lockdown_count=0,
            chain_ok_count=1, chain_ok_rate=1.0,
            first_tick="t1", last_tick="t1", span_seconds=0,
            session_started_iso="2026-08-10T00-00-00Z",
            session_ended_iso="2026-08-10T00-00-00Z",
            rendered_path="", note="",
        )
        o = m.CronTickOutcome(
            cycle_index=1, ran_at_iso="2026-08-10T00-00-00Z", tick_id="t1",
            policy="PROCEED", chain_ok=True, alerts_count=0, escalation_count=0,
            n_modules=5, appended_to_history=True, rendered_path="",
            render_ok=True, note="",
        )
        md = m.render_session_md(s, [o])
        for marker in [
            "# V1418",
            "## 1. Session summary",
            "## 2. 哲学守门",
            "## 3. Cycle outcomes",
            "## 4. Honest disclosure",
        ]:
            self.assertIn(marker, md, f"missing: {marker}")

    def test_render_session_md_with_zero_outcomes(self):
        s = m.CronSessionSummary(
            n_cycles=0, n_policies=0, policy_proceed_count=0,
            policy_pause_count=0, policy_lockdown_count=0,
            chain_ok_count=0, chain_ok_rate=0.0,
            first_tick="", last_tick="", span_seconds=0,
            session_started_iso="", session_ended_iso="",
            rendered_path="", note="",
        )
        md = m.render_session_md(s, [])
        self.assertIn("_no cycles executed_", md)


# ============================================================================
# 8. TestTickOnce (real integration with V1416 + V1417)
# ============================================================================


class TestTickOnce(unittest.TestCase):
    """These tests invoke real V1416.run_dgm_tick + V1417.append_tick_snapshot.

    They are tagged real-integration; isolated runs use temp dirs for
    V1417 history only — V1416 internally appends to V1416_DEFAULT_OUT_PATH
    (relative to CWD), which is fine because it's append-only JSONL.
    """

    def test_tick_once_chain_ok(self):
        with tempfile.TemporaryDirectory() as tmp:
            h = Path(tmp) / "h.jsonl"
            b = Path(tmp) / "b.json"
            outcome = m.tick_once(
                history_path=h,
                baseline_path=b,
                render=False,
                cycle_index=1,
            )
            self.assertTrue(outcome.chain_ok, f"tick_once failed: {outcome.note}")
            self.assertEqual(outcome.cycle_index, 1)

    def test_tick_once_appends_to_history(self):
        with tempfile.TemporaryDirectory() as tmp:
            h = Path(tmp) / "h.jsonl"
            b = Path(tmp) / "b.json"
            outcome = m.tick_once(
                history_path=h,
                baseline_path=b,
                render=False,
            )
            self.assertTrue(outcome.appended_to_history, f"history append failed: {outcome.note}")
            self.assertTrue(h.exists())

    def test_tick_once_render_produces_md(self):
        with tempfile.TemporaryDirectory() as tmp:
            h = Path(tmp) / "h.jsonl"
            b = Path(tmp) / "b.json"
            r = Path(tmp) / "render.md"
            outcome = m.tick_once(
                history_path=h,
                baseline_path=b,
                render_out=r,
                render=True,
            )
            self.assertTrue(outcome.render_ok, f"render failed: {outcome.note}")
            self.assertTrue(r.exists())
            content = r.read_text(encoding="utf-8")
            self.assertIn("# V1417", content)
            self.assertIn("## 1. Summary", content)

    def test_tick_once_cycle_index_propagation(self):
        with tempfile.TemporaryDirectory() as tmp:
            h = Path(tmp) / "h.jsonl"
            b = Path(tmp) / "b.json"
            outcome = m.tick_once(
                history_path=h,
                baseline_path=b,
                render=False,
                cycle_index=7,
            )
            self.assertEqual(outcome.cycle_index, 7)
            self.assertIn("cycle_index=7", outcome.note)


# ============================================================================
# 9. TestRunSession
# ============================================================================


class TestRunSession(unittest.TestCase):
    def test_run_session_with_2_cycles_pass_through_sleep(self):
        with tempfile.TemporaryDirectory() as tmp:
            h = Path(tmp) / "h.jsonl"
            b = Path(tmp) / "b.json"
            sj = Path(tmp) / "summary.json"
            summary = m.run_session(
                cycles=2,
                cadence_seconds=1,
                history_path=h,
                baseline_path=b,
                render=True,
                sleep_fn_name="pass-through",
                summary_json_path=sj,
            )
            self.assertEqual(summary.n_cycles, 2)
            self.assertGreater(summary.policy_proceed_count, 0)
            self.assertEqual(summary.policy_proceed_count + summary.policy_pause_count + summary.policy_lockdown_count, 2)
            self.assertTrue(sj.exists())

            # Inspect summary JSON
            rec = json.loads(sj.read_text(encoding="utf-8"))
            self.assertEqual(rec["n_cycles"], 2)
            self.assertEqual(len(rec["outcomes"]), 2)

    def test_run_session_rejects_out_of_bounds_cycles(self):
        with self.assertRaises(ValueError):
            m.run_session(cycles=999_999, cadence_seconds=1)

    def test_run_session_rejects_out_of_bounds_cadence(self):
        with self.assertRaises(ValueError):
            m.run_session(cycles=1, cadence_seconds=0)

    def test_run_session_summary_fields(self):
        with tempfile.TemporaryDirectory() as tmp:
            h = Path(tmp) / "h.jsonl"
            b = Path(tmp) / "b.json"
            sj = Path(tmp) / "summary.json"
            summary = m.run_session(
                cycles=1,
                cadence_seconds=1,
                history_path=h,
                baseline_path=b,
                render=False,
                sleep_fn_name="pass-through",
                summary_json_path=sj,
            )
            self.assertEqual(summary.n_cycles, 1)
            # On a real V1416 tick, first_tick should be a real tick_id (slug).
            self.assertTrue(summary.first_tick, "first_tick should not be empty after 1 cycle")
            self.assertGreaterEqual(summary.chain_ok_rate, 0.0)
            self.assertLessEqual(summary.chain_ok_rate, 1.0)


# ============================================================================
# 10. TestChainDelegate
# ============================================================================


class TestChainDelegate(unittest.TestCase):
    def test_chain_delegate_v1416_v1417_probe(self):
        chain = m.chain_delegate()
        self.assertTrue(chain["all_ok"], f"chain broken: {chain['errors']}")
        self.assertEqual(chain["n_modules"], 2)
        self.assertEqual(chain["n_modules_ok"], 2)


# ============================================================================
# 11. TestCLI (subprocess-driven against real CLI dispatcher)
# ============================================================================


def _run_cli(*args):
    """Run the V1418 CLI as a subprocess and return (returncode, stdout, stderr).

    Uses UTF-8 with errors='replace' so Chinese text in CLI output does
    not crash on Windows systems where the default codec is gbk.
    """
    cmd = [sys.executable, "-m", "apeireth.v1418_asi_dgm_cron_integration", *args]
    env = dict(os.environ)
    env["PYTHONIOENCODING"] = "utf-8"
    proc = subprocess.run(
        cmd, cwd=str(WORKSPACE),
        capture_output=True, text=True, timeout=60,
        encoding="utf-8", errors="replace", env=env,
    )
    return proc.returncode, proc.stdout or "", proc.stderr or ""


class TestCLI(unittest.TestCase):
    def test_cli_version(self):
        rc, out, _ = _run_cli("version")
        self.assertEqual(rc, 0)
        self.assertIn("V1418_VERSION", out)

    def test_cli_meta_json(self):
        rc, out, _ = _run_cli("meta", "--json")
        self.assertEqual(rc, 0)
        rec = json.loads(out)
        self.assertEqual(rec["version"], m.V1418_VERSION)
        self.assertEqual(len(rec["guards"]), 15)

    def test_cli_popper(self):
        rc, out, _ = _run_cli("popper")
        self.assertEqual(rc, 0)
        self.assertIn("15", out)

    def test_cli_chain(self):
        rc, out, _ = _run_cli("chain")
        self.assertEqual(rc, 0)
        rec = json.loads(out)
        self.assertTrue(rec["all_ok"])

    def test_cli_next_due(self):
        rc, out, _ = _run_cli(
            "next-due",
            "--last-iso", "2026-08-10T00-00-00Z",
            "--cadence-seconds", "300",
            "--jitter-seconds", "0",
        )
        self.assertEqual(rc, 0)
        self.assertEqual(out.strip(), "2026-08-10T00-05-00Z")

    def test_cli_tick_once(self):
        with tempfile.TemporaryDirectory() as tmp:
            h = Path(tmp) / "h.jsonl"
            b = Path(tmp) / "b.json"
            rc, out, err = _run_cli(
                "tick-once",
                "--history-path", str(h),
                "--baseline-path", str(b),
                "--no-render",
                "--cycle-index", "3",
            )
            if rc != 0:
                # Some envs may have missing V1416 deps; that's a noisy failure.
                # Print stderr for debug.
                self.fail(f"tick-once failed: rc={rc}; stderr={err}; out={out}")
            rec = json.loads(out)
            self.assertTrue(rec["chain_ok"])
            self.assertEqual(rec["cycle_index"], 3)

    def test_cli_run_session_pass_through(self):
        with tempfile.TemporaryDirectory() as tmp:
            h = Path(tmp) / "h.jsonl"
            b = Path(tmp) / "b.json"
            sj = Path(tmp) / "summary.json"
            rc, out, err = _run_cli(
                "run-session",
                "--cycles", "2",
                "--cadence-seconds", "1",
                "--jitter-seconds", "0",
                "--sleep-fn", "pass-through",
                "--history-path", str(h),
                "--baseline-path", str(b),
                "--summary-json-path", str(sj),
                "--no-render",
            )
            if rc != 0:
                self.fail(f"run-session failed: rc={rc}; stderr={err}; out={out}")
            rec = json.loads(out)
            self.assertEqual(rec["n_cycles"], 2)

    def test_cli_emit_shell(self):
        rc, out, _ = _run_cli(
            "emit-shell",
            "--render",
            "--cycles", "5",
            "--cadence-seconds", "60",
        )
        self.assertEqual(rc, 0)
        self.assertIn("python -m apeireth.v1418_asi_dgm_cron_integration run-session", out)
        self.assertIn("--render", out)

    def test_cli_show_outcomes_no_summary(self):
        rc, out, err = _run_cli(
            "show-outcomes",
            "--summary-json-path", "/tmp/nonexistent_v1418_summary.json",
        )
        # Should error if file is missing
        self.assertNotEqual(rc, 0)
        self.assertIn("no summary", err)

    def test_cli_detect_policy_no_last_n(self):
        rc, out, err = _run_cli(
            "detect-policy",
            "--summary-json-path", "/tmp/nonexistent_v1418_summary.json",
        )
        self.assertNotEqual(rc, 0)
        self.assertIn("--last-n", err)

    def test_cli_help(self):
        rc, out, _ = _run_cli("help")
        self.assertEqual(rc, 0)
        self.assertIn("Usage:", out)
        self.assertIn("tick-once", out)
        self.assertIn("run-session", out)

    def test_cli_demo(self):
        rc, out, _ = _run_cli("demo")
        self.assertEqual(rc, 0)
        self.assertIn("V1418", out)

    def test_cli_unknown_command(self):
        rc, _, err = _run_cli("totally_unknown_command_xyz")
        self.assertEqual(rc, 2)
        self.assertIn("unknown command", err)


# ============================================================================
# Test runner
# ============================================================================


if __name__ == "__main__":
    unittest.main(verbosity=2)
