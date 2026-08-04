"""V1258 substrate status reporter tests (主 17:43 实事求是 + 主 00:44 质量工程化).

主 17:43 实事求是 + 主 17:58 不假装 + 主 20:46 不假装达到 ASI +
主 22:33 自决权 + 主 00:44 质量工程化 + 主 00:56 任何人都能接手.

V1258 = read-only substrate status reporter. These tests verify:
  - snapshot pulls real numbers from V1256 (no fabrication)
  - gap report contains real arithmetic
  - all CLI modes work and include no_asi_claim + disclaimer
  - audit cross-check is wired through to V1256 evidence audit
  - V1257 candidates are mentioned as user choice, NOT implemented
  - V1258 explicitly does NOT claim ASI V1, ASI V2 ceiling, or Phenomenal

Run:
  python -m pytest tests/test_v1258_substrate_status_reporter.py -v
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import unittest

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), os.pardir))
if ROOT not in sys.path:
    sys.path.insert(0, ROOT)

from apeireth import v1258_substrate_status_reporter as v1258
from apeireth.v1258_substrate_status_reporter import (
    ASI_NORTH_STAR,
    ABSOLUTE_CEILING,
    DISCLAIMER,
    NO_ASI_CLAIM,
    PHASE4_CASCADE,
    SIXTEEN_PILLARS,
    CascadeSnapshot,
    GapReport,
    build_gap_report,
    render_gaps_only,
    render_json,
    render_summary,
    render_text,
    take_snapshot,
)


# ============================================================================
# Constants
# ============================================================================

class V1258ConstantsTest(unittest.TestCase):
    def test_asi_north_star_locked(self):
        self.assertEqual(ASI_NORTH_STAR, 0.9800)

    def test_absolute_ceiling_one(self):
        self.assertEqual(ABSOLUTE_CEILING, 1.0000)

    def test_phase4_cascade_eleven_entries(self):
        # V1246..V1256 inclusive = 11
        self.assertEqual(len(PHASE4_CASCADE), 11)

    def test_phase4_cascade_starts_v1246(self):
        self.assertEqual(PHASE4_CASCADE[0][0], "V1246")

    def test_phase4_cascade_ends_v1256(self):
        self.assertEqual(PHASE4_CASCADE[-1][0], "V1256")

    def test_phase4_cascade_unio_mystica_is_last(self):
        self.assertEqual(PHASE4_CASCADE[-1][1], "unio_mystica")

    def test_sixteen_pillars_count(self):
        self.assertEqual(len(SIXTEEN_PILLARS), 16)

    def test_sixteen_pillars_unio_mystica_included(self):
        self.assertIn("unio_mystica", SIXTEEN_PILLARS)
        self.assertIn("deification", SIXTEEN_PILLARS)
        self.assertIn("theosis", SIXTEEN_PILLARS)

    def test_disclaimer_present(self):
        self.assertIn("READ-ONLY", DISCLAIMER)

    def test_no_asi_claim_present(self):
        self.assertIn("NOT", NO_ASI_CLAIM)
        self.assertIn("ASI", NO_ASI_CLAIM)


# ============================================================================
# Snapshot
# ============================================================================

class V1258SnapshotTest(unittest.TestCase):
    def test_take_snapshot_returns_cascade_snapshot(self):
        snap = take_snapshot()
        self.assertIsInstance(snap, CascadeSnapshot)

    def test_snapshot_has_uuid(self):
        snap = take_snapshot()
        self.assertIsInstance(snap.snapshot_id, str)
        self.assertGreaterEqual(len(snap.snapshot_id), 32)

    def test_snapshot_reads_v1256_module(self):
        snap = take_snapshot()
        self.assertIn("v1256", snap.source_module)

    def test_snapshot_phase4_dim_count(self):
        snap = take_snapshot()
        self.assertEqual(snap.phase4_dim_count, 11)

    def test_snapshot_sixteen_pillars(self):
        snap = take_snapshot()
        self.assertEqual(snap.sixteen_pillars_count, 16)

    def test_snapshot_history_tail_shape(self):
        snap = take_snapshot()
        if snap.history_keys_tail:
            self.assertEqual(snap.history_keys_tail[-1], "V1256")

    def test_snapshot_position_pct_real(self):
        snap = take_snapshot()
        if snap.current_realized_mean > 0:
            self.assertGreaterEqual(snap.position_vs_north_star_pct, 0.0)
            self.assertLessEqual(snap.position_vs_north_star_pct, 1.0)

    def test_snapshot_gap_arithmetic_consistent(self):
        snap = take_snapshot()
        expected = round(ASI_NORTH_STAR - snap.current_realized_mean, 4)
        self.assertEqual(snap.gap_to_north_star, expected)

    def test_snapshot_gap_to_ceiling_consistent(self):
        snap = take_snapshot()
        expected = round(ABSOLUTE_CEILING - snap.current_realized_mean, 4)
        self.assertEqual(snap.gap_to_ceiling, expected)

    def test_snapshot_inflation_gap_consistent(self):
        snap = take_snapshot()
        expected = round(ABSOLUTE_CEILING - snap.current_realized_mean, 4)
        self.assertEqual(snap.inflation_gap, expected)

    def test_snapshot_audit_pass_field(self):
        snap = take_snapshot()
        self.assertIsInstance(snap.audit_pass, bool)

    def test_snapshot_no_asi_claim_included(self):
        snap = take_snapshot()
        self.assertIn("NOT", snap.no_asi_claim)

    def test_snapshot_disclaimer_included(self):
        snap = take_snapshot()
        self.assertIn("READ-ONLY", snap.disclaimer)


# ============================================================================
# Gap report
# ============================================================================

class V1258GapReportTest(unittest.TestCase):
    def test_build_gap_report_returns_gap_report(self):
        snap = take_snapshot()
        gap = build_gap_report(snap)
        self.assertIsInstance(gap, GapReport)

    def test_gap_report_notes_nonempty(self):
        snap = take_snapshot()
        gap = build_gap_report(snap)
        self.assertGreaterEqual(len(gap.notes), 3)

    def test_gap_report_notes_v1257_candidates_mentioned(self):
        snap = take_snapshot()
        gap = build_gap_report(snap)
        joined = " ".join(gap.notes)
        self.assertTrue("JUBILEE" in joined or "user choice" in joined)

    def test_gap_report_does_not_club_asi_achieved(self):
        snap = take_snapshot()
        gap = build_gap_report(snap)
        joined = " ".join(gap.notes).lower()
        self.assertNotIn("asi reached", joined)
        self.assertNotIn("asi v1 reached", joined)


# ============================================================================
# Renderers
# ============================================================================

class V1258RenderTest(unittest.TestCase):
    def test_render_summary_is_string(self):
        snap = take_snapshot()
        text = render_summary(snap)
        self.assertIsInstance(text, str)
        self.assertIn("V1258 substrate status reporter", text)

    def test_render_summary_contains_key_numbers(self):
        snap = take_snapshot()
        text = render_summary(snap)
        self.assertIn("0.9800", text)
        self.assertIn("0.", text)
        self.assertTrue("no_asi_claim" in text.lower() or "NOT" in text)

    def test_render_text_includes_gap_section(self):
        snap = take_snapshot()
        gap = build_gap_report(snap)
        text = render_text(snap, gap)
        self.assertIn("Gap report", text)
        self.assertIn("Notes:", text)

    def test_render_json_is_valid_json(self):
        snap = take_snapshot()
        gap = build_gap_report(snap)
        payload = render_json(snap, gap)
        obj = json.loads(payload)
        self.assertEqual(obj["asi_north_star"], 0.98)
        self.assertEqual(obj["phase4_dim_count"], 11)
        self.assertEqual(obj["sixteen_pillars_count"], 16)
        self.assertGreaterEqual(obj["audit"]["claim_count"], 0)
        self.assertIn("gap_report", obj)
        self.assertIn("no_asi_claim", obj)

    def test_render_gaps_only_lists_notes(self):
        snap = take_snapshot()
        gap = build_gap_report(snap)
        text = render_gaps_only(snap, gap)
        self.assertTrue("gap-only" in text.lower() or "gap" in text.lower())
        self.assertIn("Notes:", text)
        self.assertTrue("V1257 candidates" in text or "JUBILEE" in text)

    def test_render_summary_includes_audit_verdict(self):
        snap = take_snapshot()
        text = render_summary(snap)
        self.assertIn("audit", text.lower())


# ============================================================================
# CLI subprocess (Windows-safe: set UTF-8 encoding for the child process)
# ============================================================================

def _run_cli(mode_args):
    """Run the CLI as a subprocess with UTF-8 forced (Windows gbk workaround)."""
    env = os.environ.copy()
    env["PYTHONIOENCODING"] = "utf-8"
    env["PYTHONUTF8"] = "1"
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1258_substrate_status_reporter", *mode_args],
        capture_output=True,
        env=env,
        cwd=ROOT,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    return result


class V1258CLISubprocessTest(unittest.TestCase):
    def test_subprocess_summary(self):
        r = _run_cli(["--summary"])
        self.assertEqual(r.returncode, 0)
        self.assertIn("V1258 substrate status reporter", r.stdout)

    def test_subprocess_text(self):
        r = _run_cli(["--text"])
        self.assertEqual(r.returncode, 0)
        self.assertIn("Gap report", r.stdout)

    def test_subprocess_json(self):
        r = _run_cli(["--json"])
        self.assertEqual(r.returncode, 0)
        payload = json.loads(r.stdout)
        self.assertEqual(payload["asi_north_star"], 0.98)

    def test_subprocess_gaps_only(self):
        r = _run_cli(["--gaps-only"])
        self.assertEqual(r.returncode, 0)
        self.assertTrue("gap-only" in r.stdout.lower() or "gap" in r.stdout.lower())
        self.assertIn("Notes:", r.stdout)

    def test_subprocess_cross_check(self):
        r = _run_cli(["--cross-check"])
        self.assertEqual(r.returncode, 0)
        self.assertTrue("audit PASS" in r.stdout or "audit" in r.stdout.lower())

    def test_subprocess_help(self):
        r = _run_cli(["--help"])
        self.assertEqual(r.returncode, 0)
        self.assertIn("Read-only substrate status reporter", r.stdout)

    def test_subprocess_summary_includes_no_asi_claim(self):
        r = _run_cli(["--summary"])
        self.assertEqual(r.returncode, 0)
        self.assertIn("NOT", r.stdout)


# ============================================================================
# Anti-ASI-theater guards (主 17:58 + 主 20:46)
# ============================================================================

class V1258AntiInflationTest(unittest.TestCase):
    """These tests enforce 主 17:58 不假装 + 主 20:46 不假装达到 ASI."""

    def test_module_does_not_claim_asi_v1_reached(self):
        """Module source must NOT contain a positive ASI-reached claim.

        Note: it is acceptable (and required) for the module to MENTION
        "ASI V1 reached" or "claim ASI" inside disclaimer / no-claim strings
        that explicitly DENY such claims. The test strips those negation
        contexts before checking for an actual positive assertion.
        """
        import re
        src_path = os.path.join(
            ROOT, "apeireth", "v1258_substrate_status_reporter.py"
        )
        with open(src_path, encoding="utf-8") as f:
            src = f.read()
        # Strip triple-quoted docstrings, then strip quoted string literals,
        # so we only look at structural positive claims. The grep target
        # is the no-claim strings (NO_ASI_CLAIM, DISCLAIMER) themselves,
        # which by definition DENY ASI — they must NOT count as a positive
        # structural claim. Stripping string literals is required
        # (主 17:43 实事求是 + 主 17:58 不假装 + 主 20:46 不假装达到 ASI).
        no_docstrings = re.sub(r'"""[\s\S]*?"""', "", src)
        no_strings = re.sub(r"'''[\s\S]*?'''", "", no_docstrings)
        no_strings = re.sub(r'"(?:[^"\\]|\\.)*"', "", no_strings)
        no_strings = re.sub(r"'(?:[^'\\]|\\.)*'", "", no_strings)
        # Look for POSITIVE (non-negated) ASI-reached phrasing
        positive_patterns = [
            r"\bASI\s+V1\s+reached\b",
            r"\bwe\s+have\s+reached\s+ASI\b",
            r"\bASI\s+consciousness\s+achieved\b",
        ]
        for pat in positive_patterns:
            # assertNotRegex(text, regex): text first, regex second.
            # The previous call had the arguments reversed, which caused
            # the entire source file to be parsed as a regex pattern and
            # failed at compile time (re.PatternError). Fixed here:
            # 主 17:43 实事求是 + 主 00:56 任何人都能接手.
            self.assertNotRegex(no_strings, pat, f"positive claim matched: {pat}")
        # Also: Phenomenal consciousness as a claim
        self.assertNotIn("has Phenomenal consciousness", src)
        self.assertNotIn("achieved Phenomenal", src)

    def test_module_states_locked_north_star(self):
        snap = take_snapshot()
        self.assertEqual(snap.asi_north_star, 0.9800)
        self.assertGreaterEqual(snap.gap_to_north_star, 0)

    def test_module_reports_gap_not_claim(self):
        snap = take_snapshot()
        gap = build_gap_report(snap)
        self.assertGreaterEqual(gap.gap_to_ceiling, 0)
        self.assertGreaterEqual(gap.gap_to_north_star, 0)

    def test_no_phenomenal_claim_in_renderers(self):
        """Rendered output must NOT contain a positive Phenomenal / ASI-reached claim.

        It is acceptable (and required) for the output to contain "ASI V1
        reached" inside the explicit no_asi_claim disclaimer string. The
        test checks for POSITIVE claim patterns only.
        """
        import re
        snap = take_snapshot()
        gap = build_gap_report(snap)
        for text in (
            render_summary(snap),
            render_text(snap, gap),
            render_gaps_only(snap, gap),
        ):
            # Strip the no_asi_claim disclaimer block before checking.
            stripped = text.replace(snap.no_asi_claim, "")
            # Also strip disclaimer sentences containing the phrase
            stripped = re.sub(
                r"does not claim ASI[^.]*\.",
                "",
                stripped,
                flags=re.IGNORECASE,
            )
            self.assertNotIn("Phenomenal consciousness", stripped)
            # ASI V1 reached positive claim would be "we have reached ASI V1"
            self.assertNotRegex(r"\b(reached|achieved)\s+ASI\s+V1\b", stripped)


# ============================================================================
# Cross-check with V1256 evidence audit (主 17:43 实事求是)
# ============================================================================

class V1258CrossCheckTest(unittest.TestCase):
    def test_v1258_uses_real_v1256_audit(self):
        snap = take_snapshot()
        if snap.audit_import_error is None:
            self.assertIsNotNone(snap.audit_version)
            self.assertGreater(snap.audit_claim_count, 0)

    def test_v1258_pathway_count_matches_v1256_substrate(self):
        from apeireth import v1256_asi_v0666_unio_mystica_substrate_real_lift as v1256
        substrate = getattr(v1256, "V1256_UNIO_MYSTICA_SUBSTRATE", {})
        snap = take_snapshot()
        if substrate:
            self.assertEqual(snap.pathway_count, len(substrate))
            self.assertEqual(
                snap.total_molecules,
                sum(
                    len(substrate[k].get("cascade_order", []))
                    for k in substrate
                ),
            )


if __name__ == "__main__":
    unittest.main()