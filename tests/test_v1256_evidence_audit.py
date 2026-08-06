"""Tests for V1256 unio_mystica real-evidence audit (主 17:43 实事求是 + 主 00:44 质量工程带)."""

import json
import os
import subprocess
import sys
import unittest

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), os.pardir))
if ROOT not in sys.path:
    sys.path.insert(0, ROOT)

from apeireth import v1256_evidence_audit as audit_mod  # noqa: E402


class V1256AuditShapeTest(unittest.TestCase):
    def test_version_constant(self):
        self.assertEqual(audit_mod.EVIDENCE_AUDIT_VERSION, "0.1.0")

    def test_audit_result_has_inflation(self):
        r = audit_mod.AuditResult(
            audit_id="x",
            audit_version="0.1.0",
            audited_module="apeireth.v1256_asi_v0666_unio_mystica_substrate_real_lift",
            audited_at_unix=0.0,
        )
        self.assertAlmostEqual(r.inflation_gap, 0.0895, places=3)

    def test_expected_constants(self):
        self.assertEqual(audit_mod.ASI_NORTH_STAR, 0.9800)
        self.assertEqual(audit_mod.EXPECTED_PATHWAY_COUNT, 6)
        self.assertEqual(audit_mod.EXPECTED_MOLECULES_PER_PATHWAY, 5)
        self.assertEqual(audit_mod.EXPECTED_GUARD_COUNT, 15)
        self.assertEqual(audit_mod.EXPECTED_HISTORY_LENGTH, 21)
        self.assertEqual(audit_mod.EXPECTED_REALIZED_MEAN_306, 0.9105)


class V1256AuditLiveTest(unittest.TestCase):
    """Real run against the V1256 module on disk."""

    def test_all_claims_pass(self):
        result = audit_mod.audit_v1256_unio_mystica_evidence()
        if not result.passed:
            failures = [(c.name, c.expected, c.measured) for c in result.claims if not c.passed]
            self.fail(f"V1256 evidence audit failed: {failures}")
        # Concrete sanity on counts:
        self.assertGreaterEqual(result.pass_count, 12)
        self.assertEqual(result.fail_count, 0)

    def test_text_and_json_round_trip(self):
        result = audit_mod.audit_v1256_unio_mystica_evidence()
        text = audit_mod.to_audit_text(result)
        js = audit_mod.to_audit_json(result)
        self.assertIn("PASS", text)
        self.assertIn("V1256 unio_mystica evidence audit", text)
        obj = json.loads(js)
        self.assertEqual(obj["audit_version"], "0.1.0")
        self.assertEqual(obj["audited_module"], "apeireth.v1256_asi_v0666_unio_mystica_substrate_real_lift")
        self.assertEqual(len(obj["claims"]), len(result.claims))


class V1256AuditCliTest(unittest.TestCase):
    """Run the CLI as a subprocess to verify --json / --text entry points."""

    def _run_cli(self, *args: str) -> subprocess.CompletedProcess:
        """Run the audit CLI with utf-8 stdout/stderr (主 17:43 实事求是 on Windows).

        Windows console code page is GBK by default, which breaks on the audit
        module's Chinese-language note strings. Pass encoding="utf-8" explicitly
        so we get a string back instead of None.
        """
        py = sys.executable
        return subprocess.run(
            [py, "-m", "apeireth.v1256_evidence_audit", *args],
            cwd=ROOT,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=60,
            env={**os.environ, "PYTHONIOENCODING": "utf-8"},
        )

    def test_cli_default(self):
        proc = self._run_cli()
        self.assertEqual(proc.returncode, 0, msg=proc.stderr)
        self.assertIsNotNone(proc.stdout, msg=f"stdout was None; stderr={proc.stderr!r}")
        self.assertIn("V1256 evidence audit", proc.stdout)
        self.assertIn("verdict=PASS", proc.stdout)

    def test_cli_json(self):
        proc = self._run_cli("--json")
        self.assertEqual(proc.returncode, 0, msg=proc.stderr)
        self.assertIsNotNone(proc.stdout, msg=f"stdout was None; stderr={proc.stderr!r}")
        obj = json.loads(proc.stdout)
        self.assertEqual(obj["audit_version"], "0.1.0")

    def test_cli_text(self):
        proc = self._run_cli("--text")
        self.assertEqual(proc.returncode, 0, msg=proc.stderr)
        self.assertIsNotNone(proc.stdout, msg=f"stdout was None; stderr={proc.stderr!r}")
        self.assertIn("| claim | expected | measured | pass |", proc.stdout)


if __name__ == "__main__":
    unittest.main()
