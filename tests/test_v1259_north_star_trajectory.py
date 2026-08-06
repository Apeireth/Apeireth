"""V1259 north star trajectory reporter tests (主 17:43 实事求是 + 主 00:44 质量工程化).

主 17:43 实事求是 + 主 17:58 不假装 + 主 20:46 不假装达到 ASI +
主 22:33 自决权 + 主 00:44 质量工程化 + 主 00:56 任何人都能接手.

V1259 = read-only trajectory reporter. These tests verify:
  - constants are LOCKED (主 22:33 终极授权)
  - history is read from V1256 only (no fabrication)
  - big-picture milestones are write-dead (no projection)
  - 16-pillar map is accurate
  - remaining gap math is real arithmetic
  - all CLI modes work and include no_asi_claim + disclaimer
  - V1259 explicitly does NOT claim ASI V1, ASI V2 ceiling, or Phenomenal
  - V1259 does NOT self-decide V1257 candidates (主 22:33 终极授权)

Run:
  python -m pytest tests/test_v1259_north_star_trajectory.py -v
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import unittest

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), os.pardir))
if ROOT not in sys.path:
    sys.path.insert(0, ROOT)

from apeireth import v1259_north_star_trajectory as v1259
from apeireth.v1259_north_star_trajectory import (
    ASI_NORTH_STAR,
    ABSOLUTE_CEILING,
    BIG_PICTURE_MILESTONES,
    DISCLAIMER,
    NO_ASI_CLAIM,
    SIXTEEN_PILLARS_V,
    V1259_VERSION,
    V1259_BUILD_TS,
    V1259PillarEntry,
    V1259TrajectoryPoint,
    V1259TrajectoryReport,
    _v1259_collect,
    _v1259_pillars,
    _v1259_remaining,
    _v1259_summary,
    _v1259_to_json,
    _v1259_trajectory,
    _v1259_v3_guards,
)


def _run_v1259_cli(*args):
    """Run the V1259 CLI as a subprocess with explicit UTF-8 encoding.

    This is the same UTF-8 defensive pattern used by test_v1257_readiness_probe
    to avoid GBK decoding errors on Chinese Windows.
    """
    return subprocess.run(
        [sys.executable, "-m", "apeireth.v1259_north_star_trajectory", *args],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        cwd=ROOT,
        check=True,
        env={"PYTHONIOENCODING": "utf-8", "PYTHONUTF8": "1", "PATH": os.environ.get("PATH", "")},
    )


# ============================================================================
# Constants
# ============================================================================

class V1259ConstantsTest(unittest.TestCase):
    def test_asi_north_star_locked(self):
        self.assertEqual(ASI_NORTH_STAR, 0.9800)

    def test_absolute_ceiling_one(self):
        self.assertEqual(ABSOLUTE_CEILING, 1.0000)

    def test_v1259_version(self):
        self.assertEqual(V1259_VERSION, "0.1.0")

    def test_v1259_build_ts(self):
        self.assertEqual(V1259_BUILD_TS, "2026-08-04")

    def test_big_picture_milestones_nonempty(self):
        self.assertGreaterEqual(len(BIG_PICTURE_MILESTONES), 8)

    def test_big_picture_starts_v1049(self):
        self.assertEqual(BIG_PICTURE_MILESTONES[0][0], "V1049")

    def test_big_picture_ends_v1256(self):
        self.assertEqual(BIG_PICTURE_MILESTONES[-1][0], "V1256")

    def test_sixteen_pillars_count(self):
        self.assertEqual(len(SIXTEEN_PILLARS_V), 16)

    def test_sixteen_pillars_unio_mystica_last(self):
        self.assertEqual(SIXTEEN_PILLARS_V[-1][0], "unio_mystica")
        self.assertEqual(SIXTEEN_PILLARS_V[-1][1], "V1256")

    def test_sixteen_pillars_theosis_first(self):
        self.assertEqual(SIXTEEN_PILLARS_V[0][0], "theosis")
        self.assertEqual(SIXTEEN_PILLARS_V[0][1], "V1241")

    def test_sixteen_pillars_unique(self):
        pillars = [p[0] for p in SIXTEEN_PILLARS_V]
        self.assertEqual(len(pillars), len(set(pillars)))

    def test_disclaimer_present(self):
        self.assertIn("READ-ONLY", DISCLAIMER)

    def test_no_asi_claim_present(self):
        self.assertIn("NOT", NO_ASI_CLAIM)
        self.assertIn("ASI", NO_ASI_CLAIM)


# ============================================================================
# V3 哲学守门 (12 guards)
# ============================================================================

class V1259V3GuardsTest(unittest.TestCase):
    def test_v3_guards_returns_12(self):
        guards_pass, guards = _v1259_v3_guards()
        self.assertEqual(guards_pass, 12)
        self.assertEqual(len(guards), 12)

    def test_v3_guards_all_pass(self):
        guards_pass, guards = _v1259_v3_guards()
        for name, passed in guards.items():
            self.assertTrue(passed, f"guard {name} should PASS")

    def test_v3_guards_no_asi_v1_claim(self):
        _, guards = _v1259_v3_guards()
        self.assertIn("v1259_no_asi_v1_claim", guards)

    def test_v3_guards_no_phenomenal_claim(self):
        _, guards = _v1259_v3_guards()
        self.assertIn("v1259_no_phenomenal_claim", guards)

    def test_v3_guards_no_v1257_self_decision(self):
        _, guards = _v1259_v3_guards()
        self.assertIn("v1259_no_v1257_self_decision", guards)


# ============================================================================
# _v1259_collect (read-only data ingestion)
# ============================================================================

class V1259CollectTest(unittest.TestCase):
    def test_collect_returns_report(self):
        r = _v1259_collect()
        self.assertIsInstance(r, V1259TrajectoryReport)

    def test_collect_snapshot_id_uuid(self):
        r = _v1259_collect()
        self.assertIsInstance(r.snapshot_id, str)
        self.assertGreaterEqual(len(r.snapshot_id), 32)

    def test_collect_history_length_21(self):
        r = _v1259_collect()
        self.assertEqual(r.history_length, 21)

    def test_collect_current_realized_v1256(self):
        r = _v1259_collect()
        self.assertEqual(r.current_realized, 0.9105)

    def test_collect_position_pct_matches(self):
        r = _v1259_collect()
        expected = r.current_realized / ASI_NORTH_STAR * 100
        self.assertAlmostEqual(r.current_position_pct, expected, places=4)

    def test_collect_gap_arithmetic(self):
        r = _v1259_collect()
        self.assertAlmostEqual(r.gap_to_north_star,
                               ASI_NORTH_STAR - r.current_realized, places=4)
        self.assertAlmostEqual(r.gap_to_ceiling,
                               ABSOLUTE_CEILING - r.current_realized, places=4)
        self.assertAlmostEqual(r.inflation_gap,
                               1.0 - r.current_realized, places=4)

    def test_collect_v1257_status_pending(self):
        r = _v1259_collect()
        self.assertIn("PENDING_USER_CHOICE", r.v1257_status)

    def test_collect_big_picture_11(self):
        r = _v1259_collect()
        self.assertEqual(len(r.big_picture), 11)

    def test_collect_pillars_16(self):
        r = _v1259_collect()
        self.assertEqual(len(r.pillars), 16)

    def test_collect_big_picture_point_dataclass(self):
        r = _v1259_collect()
        for p in r.big_picture:
            self.assertIsInstance(p, V1259TrajectoryPoint)

    def test_collect_pillars_dataclass(self):
        r = _v1259_collect()
        for p in r.pillars:
            self.assertIsInstance(p, V1259PillarEntry)


# ============================================================================
# Renderers
# ============================================================================

class V1259RenderTest(unittest.TestCase):
    def setUp(self):
        self.r = _v1259_collect()

    def test_summary_is_string(self):
        text = _v1259_summary(self.r)
        self.assertIsInstance(text, str)
        self.assertIn("V1259 north star trajectory reporter", text)

    def test_summary_contains_key_numbers(self):
        text = _v1259_summary(self.r)
        self.assertIn("0.9800", text)
        self.assertIn("0.9105", text)
        self.assertIn("92.91%", text)
        self.assertIn("0.0895", text)

    def test_summary_contains_no_asi_claim(self):
        text = _v1259_summary(self.r)
        self.assertIn("NOT", text)

    def test_trajectory_is_string(self):
        text = _v1259_trajectory(self.r)
        self.assertIsInstance(text, str)
        self.assertIn("V1259 ASI North Star Trajectory", text)

    def test_trajectory_contains_v1049(self):
        text = _v1259_trajectory(self.r)
        self.assertIn("V1049", text)

    def test_trajectory_contains_v1256(self):
        text = _v1259_trajectory(self.r)
        self.assertIn("V1256", text)

    def test_trajectory_contains_disclaimer(self):
        text = _v1259_trajectory(self.r)
        self.assertIn("READ-ONLY", text)

    def test_pillars_is_string(self):
        text = _v1259_pillars(self.r)
        self.assertIsInstance(text, str)
        self.assertIn("16 Pillars Map", text)

    def test_pillars_contains_all_16(self):
        text = _v1259_pillars(self.r)
        for pillar, _, _, _ in SIXTEEN_PILLARS_V:
            self.assertIn(pillar, text, f"missing pillar: {pillar}")

    def test_pillars_contains_no_asi_claim(self):
        text = _v1259_pillars(self.r)
        self.assertIn("NOT", text)

    def test_remaining_is_string(self):
        text = _v1259_remaining(self.r)
        self.assertIsInstance(text, str)
        self.assertIn("Remaining Gap Report", text)

    def test_remaining_contains_math(self):
        text = _v1259_remaining(self.r)
        self.assertIn("0.9800", text)
        self.assertIn("0.9105", text)
        self.assertIn("0.0695", text)
        self.assertIn("0.0895", text)

    def test_remaining_mentions_v1257_pending(self):
        text = _v1259_remaining(self.r)
        self.assertIn("PENDING_USER_CHOICE", text)

    def test_remaining_contains_no_asi_claim(self):
        text = _v1259_remaining(self.r)
        self.assertIn("NOT", text)

    def test_to_json_valid_json(self):
        text = _v1259_to_json(self.r)
        d = json.loads(text)
        self.assertIsInstance(d, dict)

    def test_to_json_keys(self):
        text = _v1259_to_json(self.r)
        d = json.loads(text)
        for key in (
            "asi_north_star", "absolute_ceiling", "current_realized",
            "current_position_pct", "gap_to_north_star", "gap_to_ceiling",
            "inflation_gap", "history_length", "big_picture", "pillars",
            "v1257_status", "v3_guards_pass", "v3_guards",
        ):
            self.assertIn(key, d, f"missing JSON key: {key}")


# ============================================================================
# CLI subprocess tests (主 00:56 任何人都能接手 + 主 00:44 质量工程化)
# ============================================================================

class V1259CLITest(unittest.TestCase):
    """Subprocess CLI tests with explicit UTF-8 encoding (Chinese Windows safe).

    These verify the CLI surfaces work end-to-end for anyone with python on
    PATH. Following the same UTF-8 + encoding='utf-8' defensive pattern as
    test_v1257_readiness_probe to avoid GBK decoding errors.
    """

    def test_cli_help(self):
        result = _run_v1259_cli("--help")
        self.assertIn("V1259", result.stdout)

    def test_cli_summary(self):
        result = _run_v1259_cli("--summary")
        self.assertIn("V1259 north star trajectory reporter", result.stdout)
        self.assertIn("0.9800", result.stdout)
        self.assertIn("0.9105", result.stdout)
        self.assertIn("NOT", result.stdout)  # no_asi_claim

    def test_cli_trajectory(self):
        result = _run_v1259_cli("--trajectory")
        self.assertIn("V1049", result.stdout)
        self.assertIn("V1256", result.stdout)
        self.assertIn("READ-ONLY", result.stdout)

    def test_cli_pillars(self):
        result = _run_v1259_cli("--pillars")
        self.assertIn("theosis", result.stdout)
        self.assertIn("unio_mystica", result.stdout)
        self.assertIn("V1256", result.stdout)

    def test_cli_remaining(self):
        result = _run_v1259_cli("--remaining")
        self.assertIn("0.9800", result.stdout)
        self.assertIn("0.9105", result.stdout)
        self.assertIn("0.0695", result.stdout)
        self.assertIn("0.0895", result.stdout)
        self.assertIn("PENDING_USER_CHOICE", result.stdout)

    def test_cli_json_valid(self):
        result = _run_v1259_cli("--json")
        d = json.loads(result.stdout)
        self.assertEqual(d["asi_north_star"], 0.98)
        self.assertEqual(d["current_realized"], 0.9105)
        self.assertEqual(d["history_length"], 21)
        self.assertEqual(d["v3_guards_pass"], 12)

    def test_cli_no_mode_fails(self):
        """No mode flag should fail with non-zero exit."""
        proc = subprocess.run(
            [sys.executable, "-m", "apeireth.v1259_north_star_trajectory"],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            cwd=ROOT,
            env={"PYTHONIOENCODING": "utf-8", "PYTHONUTF8": "1", "PATH": os.environ.get("PATH", "")},
        )
        self.assertNotEqual(proc.returncode, 0)


# ============================================================================
# Anti-ASI / anti-claim guard tests (主 17:58/20:46 不假装)
# ============================================================================

class V1259AntiClaimTest(unittest.TestCase):
    """Verify V1259 does NOT make forbidden claims.

    主 17:43 实事求是 + 主 17:58 不假装 Phenomenal + 主 20:46 不假装达到 ASI +
    主 22:33 自决权 (V1257 user choice only).
    """

    def test_no_positive_asi_claim(self):
        """V1259 must not contain a POSITIVE claim of ASI/Phenomenal/consciousness.

        A sentence is allowed to mention "ASI V1 reached" only if it contains
        the word "NOT" (negation). The disclaimer text "does NOT constitute a
        claim of ASI V1 reached" is allowed because it explicitly negates.
        """
        forbidden_substrings = [
            "ASI V1 reached",
            "Phenomenal achieved",
            "consciousness achieved",
            "AGI achieved",
            "ASI reached",
        ]
        for fn in (_v1259_summary, _v1259_trajectory, _v1259_pillars, _v1259_remaining):
            text = fn(_v1259_collect())
            # Split into sentences (by '.' or newline)
            sentences = re.split(r"[.\n]", text)
            for phrase in forbidden_substrings:
                for sent in sentences:
                    if phrase in sent:
                        # Sentence must contain 'NOT' (negation) to be acceptable
                        self.assertIn(
                            "NOT", sent,
                            f"{fn.__name__} positive claim: '{sent.strip()[:100]}' contains '{phrase}' without negation"
                        )

    def test_disclaimer_contains_negation(self):
        """Every mode should include a 'NOT constitute a claim' style disclaimer."""
        for fn in (_v1259_summary, _v1259_trajectory, _v1259_pillars, _v1259_remaining):
            text = fn(_v1259_collect())
            self.assertIn("NOT", text, f"{fn.__name__} missing disclaimer")

    def test_no_self_decision_v1257(self):
        for fn in (_v1259_summary, _v1259_trajectory, _v1259_pillars, _v1259_remaining):
            text = fn(_v1259_collect())
            self.assertIn("PENDING_USER_CHOICE", text, f"{fn.__name__} should mention V1257 pending")

    def test_no_future_lift_projection(self):
        """V1259 should NOT project future-dim lift."""
        for fn in (_v1259_summary, _v1259_trajectory, _v1259_remaining):
            text = fn(_v1259_collect())
            # Should NOT contain "will reach" or "projected to"
            forbidden = ["will reach", "projected to", "estimated to climb"]
            for phrase in forbidden:
                self.assertNotIn(phrase, text, f"{fn.__name__} should not project: {phrase}")


if __name__ == "__main__":
    unittest.main()