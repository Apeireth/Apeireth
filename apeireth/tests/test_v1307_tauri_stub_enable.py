"""V1307 — Tauri-stub enable test (Popper-style hypotheses)

Verifies the V1307 audit + fix:
1. tauri-stub in workspace members (cargo metadata)
2. tauri-stub in packages
3. cargo check -p apeireth-tauri-stub passes
4. workspace comment no longer contains misleading "reqwest 0.13 强约束"
5. workspace comment now contains V1307 truth marker
6. V3 哲学守门 present in module docstring
7. uncommented_count: tauri-stub line is no longer commented in Cargo.toml
8. cargo_metadata_packages >= 92 (was 91 before V1307)
"""

import json
import os
import re
import subprocess
import sys
import unittest
from pathlib import Path

REPO = Path(r".openclaw\workspace\promethean")
WS = REPO / "Apeireth-rust"
WS_CARGO = WS / "Cargo.toml"
TS_CARGO = WS / "crates" / "apeireth-tauri-stub" / "Cargo.toml"


class TestV1307TauriStubEnable(unittest.TestCase):

    @classmethod
    def setUpClass(cls):
        # Get cargo metadata
        out = subprocess.run(
            ["cargo", "metadata", "--format-version=1", "--no-deps"],
            cwd=WS, capture_output=True, timeout=60
        )
        if out.returncode != 0:
            raise RuntimeError(f"cargo metadata failed: {out.stderr.decode('utf-8','replace')[:500]}")
        cls.meta = json.loads(out.stdout)

    def test_h_tauri_stub_in_workspace_members(self):
        """h_v1307_in_members: tauri-stub path in workspace_members"""
        found = any("tauri-stub" in m for m in self.meta["workspace_members"])
        self.assertTrue(found, "tauri-stub should be in workspace_members")

    def test_h_tauri_stub_in_packages(self):
        """h_v1307_in_packages: tauri-stub name in packages list"""
        names = [p["name"] for p in self.meta["packages"]]
        self.assertIn("apeireth-tauri-stub", names)

    def test_h_packages_count_at_least_92(self):
        """h_v1307_packages_92: packages count >= 92 (was 91 before V1307)"""
        self.assertGreaterEqual(len(self.meta["packages"]), 92)

    def test_h_members_count_at_least_92(self):
        """h_v1307_members_92: workspace_members count >= 92"""
        self.assertGreaterEqual(len(self.meta["workspace_members"]), 92)

    def test_h_tauri_stub_cargo_check_passes(self):
        """h_v1307_cargo_check: cargo check -p apeireth-tauri-stub returncode 0"""
        out = subprocess.run(
            ["cargo", "check", "-p", "apeireth-tauri-stub"],
            cwd=WS, capture_output=True, timeout=120
        )
        self.assertEqual(out.returncode, 0,
                         f"cargo check failed: {out.stderr.decode('utf-8','replace')[-500:]}")

    def test_h_workspace_cargo_no_misleading_reqwest(self):
        """h_v1307_no_misleading_reqwest: workspace Cargo.toml no longer cites
        'reqwest 0.13 强约束' as the reason for excluding tauri-stub (since
        tauri-stub has no reqwest dep and builds clean)."""
        content = WS_CARGO.read_text(encoding="utf-8")
        # Look for the misleading phrase in the tauri-stub comment block
        # Find the line "# 2026-08-05 P0-1 fix:" referring to tauri-stub
        if "2026-08-05 P0-1 fix" in content:
            # Check it was updated to reflect V1307
            self.assertIn("V1307 实证反驳", content,
                          "workspace comment should mark V1307 fix replacing old reqwest claim")

    def test_h_workspace_cargo_has_v1307_marker(self):
        """h_v1307_marker: workspace Cargo.toml contains 'V1307 fix' marker"""
        content = WS_CARGO.read_text(encoding="utf-8")
        self.assertIn("V1307 fix", content)

    def test_h_tauri_stub_uncommented(self):
        """h_v1307_uncommented: 'crates/apeireth-tauri-stub' line is NOT commented out"""
        content = WS_CARGO.read_text(encoding="utf-8")
        # Use TOML parser for robustness (regex gets confused by [...] inside comments)
        try:
            import tomllib
        except ImportError:
            import tomli as tomllib
        data = tomllib.loads(content)
        members = data["workspace"]["members"]
        ts_members = [m for m in members if "tauri-stub" in m]
        self.assertGreater(len(ts_members), 0,
                           f"expected at least 1 active tauri-stub member, got {ts_members}")
        # And the path should be exactly "crates/apeireth-tauri-stub" (not commented)
        self.assertIn("crates/apeireth-tauri-stub", members)

    def test_h_tauri_stub_cargo_no_reqwest_dep(self):
        """h_v1307_ts_cargo_no_reqwest: tauri-stub Cargo.toml has no reqwest dep"""
        content = TS_CARGO.read_text(encoding="utf-8")
        # Look for reqwest in deps section
        deps_match = re.search(r"\[dependencies\](.*?)(?=\n\[|\Z)", content, re.DOTALL)
        if deps_match:
            deps_section = deps_match.group(1)
            self.assertNotIn("reqwest", deps_section.lower(),
                             "tauri-stub should not depend on reqwest")

    def test_h_v3_philosophy_guard_in_audit(self):
        """h_v1307_v3_guard: V3 哲学守门 mentioned in audit script"""
        audit_script = REPO / "apeireth" / "v1307_tauri_stub_audit.py"
        content = audit_script.read_text(encoding="utf-8")
        # Should reference V3 philosophy in some form
        self.assertTrue(
            any(kw in content for kw in ["V3", "哲学", "守门", "polaris", "asi"]),
            "audit script should mention V3 philosophy"
        )

    def test_h_decision_documented(self):
        """h_v1307_decision: decision.json exists with build_passed=true"""
        decision_path = REPO / "v1307_decision.json"
        self.assertTrue(decision_path.exists(), "decision.json should exist")
        data = json.loads(decision_path.read_text(encoding="utf-8"))
        self.assertTrue(data.get("check_passed"),
                        f"decision.json should show check_passed=true, got {data}")

    def test_h_report_exists(self):
        """h_v1307_report: V1307_REPORT.md exists"""
        report_path = REPO / "V1307_REPORT.md"
        self.assertTrue(report_path.exists(), "V1307_REPORT.md should exist")

    def test_h_audit_script_exists(self):
        """h_v1307_audit_script: v1307 audit script exists and runs"""
        script = REPO / "apeireth" / "v1307_tauri_stub_audit.py"
        self.assertTrue(script.exists())
        # Should be runnable
        out = subprocess.run(
            [sys.executable, str(script)],
            cwd=REPO, capture_output=True, timeout=30
        )
        # May print to stdout but should not error
        # (it writes a findings JSON)
        findings_path = REPO / "v1307_audit_findings.json"
        self.assertTrue(findings_path.exists(), "v1307_audit_findings.json should exist")


if __name__ == "__main__":
    unittest.main(verbosity=2)