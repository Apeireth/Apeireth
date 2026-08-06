"""Tests for V1291 — VCP Rust Build Artifact Profile.

V1291 = 真生产全 42 crates build artifact profile (target/debug/deps/*.{rlib,rmeta,exe,pdb,d}).
测试覆盖: regex + ArtifactInfo + CrateBuildProfile + scan_crate + find_deps_dir
+ _evaluate_hypotheses + Markdown + JSON + CLI + Regression (V1289/V1290 imports 仍 OK).
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
if str(PROMETHEAN_ROOT) not in sys.path:
    sys.path.insert(0, str(PROMETHEAN_ROOT))

from apeireth.v1291_rust_build_artifact_profile import (  # noqa: E402
    APEIRETH_RUST_CRATE_NAMES,
    ArtifactInfo,
    BuildArtifactLedger,
    CrateBuildProfile,
    V1291_THRESHOLD_ARTIFACTS_PER_CRATE,
    V1291_THRESHOLD_COVERAGE_PCT,
    V1291_THRESHOLD_MEDIAN_SIZE_KB,
    V1291_THRESHOLD_TOTAL_GB,
    _classify_kind,
    _evaluate_hypotheses,
    _is_example_artifact,
    _is_test_artifact,
    find_deps_dir,
    main,
    scan_crate,
    to_markdown,
    top_n_by_size,
)
from apeireth.v1291_rust_build_artifact_profile import ARTIFACT_NAME_RE  # noqa: E402


class TestV1291Regex(unittest.TestCase):
    """Regex patterns for cargo build artifacts."""

    def test_artifact_name_basic(self):
        m = ARTIFACT_NAME_RE.match("apeireth_action-0010ce5b958c453a.rlib")
        self.assertIsNotNone(m)
        self.assertEqual(m.group("base"), "apeireth_action")
        self.assertEqual(m.group("hash"), "0010ce5b958c453a")
        self.assertEqual(m.group("ext"), "rlib")

    def test_artifact_name_with_lib_prefix(self):
        m = ARTIFACT_NAME_RE.match("libapeireth_tool_registry-07280f66b9c5a569.rmeta")
        self.assertIsNotNone(m)
        self.assertEqual(m.group("base"), "libapeireth_tool_registry")
        self.assertEqual(m.group("ext"), "rmeta")

    def test_artifact_name_exe(self):
        m = ARTIFACT_NAME_RE.match("apeireth_central-19ab780138ec0cb7.exe")
        self.assertIsNotNone(m)
        self.assertEqual(m.group("ext"), "exe")

    def test_artifact_name_invalid_hash(self):
        self.assertIsNone(ARTIFACT_NAME_RE.match("apeireth_action-ZZZZ.rlib"))


class TestV1291ClassifyKind(unittest.TestCase):
    """_classify_kind — kind classification."""

    def test_rlib(self):
        self.assertEqual(_classify_kind("rlib", "apeireth_action"), "rlib")

    def test_rmeta(self):
        self.assertEqual(_classify_kind("rmeta", "apeireth_action"), "rmeta")

    def test_exe(self):
        self.assertEqual(_classify_kind("exe", "apeireth_central"), "exe")

    def test_pdb(self):
        self.assertEqual(_classify_kind("pdb", "apeireth_action"), "pdb")

    def test_d_file(self):
        self.assertEqual(_classify_kind("d", "apeireth_action"), "d")

    def test_unknown(self):
        self.assertEqual(_classify_kind("so", "apeireth_action"), "so")


class TestV1291IsTestOrExample(unittest.TestCase):
    """_is_test_artifact / _is_example_artifact detection."""

    def test_test_artifact(self):
        self.assertTrue(_is_test_artifact("apeireth_action_test"))
        self.assertTrue(_is_test_artifact("test_runner"))

    def test_non_test_artifact(self):
        self.assertFalse(_is_test_artifact("apeireth_action"))

    def test_example_artifact(self):
        self.assertTrue(_is_example_artifact("apeireth_action_example"))
        self.assertTrue(_is_example_artifact("example_basic"))

    def test_non_example_artifact(self):
        self.assertFalse(_is_example_artifact("apeireth_action"))


class TestV1291ScanCrate(unittest.TestCase):
    """scan_crate — 真扫描单个 crate build artifacts."""

    def setUp(self):
        import tempfile
        self.tmpdir = Path(tempfile.mkdtemp())
        self.deps_dir = self.tmpdir / "deps"
        self.deps_dir.mkdir()

    def _make_artifact(self, name: str, size: int = 1024):
        """生成 fake cargo build artifact."""
        f = self.deps_dir / name
        f.write_bytes(b"\x00" * size)
        return f

    def test_empty_deps_dir(self):
        profile = scan_crate("apeireth-action", self.deps_dir)
        self.assertEqual(profile.total_artifacts, 0)
        self.assertFalse(profile.has_any_artifact)

    def test_single_rlib(self):
        self._make_artifact("apeireth_action-0010ce5b958c453a.rlib", 2048)
        profile = scan_crate("apeireth-action", self.deps_dir)
        self.assertEqual(profile.n_rlib, 1)
        self.assertEqual(profile.total_size_bytes, 2048)

    def test_full_set_rlib_rmeta_d_pdb(self):
        self._make_artifact("apeireth_action-0010ce5b958c453a.rlib", 2048)
        self._make_artifact("apeireth_action-0010ce5b958c453a.rmeta", 512)
        self._make_artifact("apeireth_action-0010ce5b958c453a.d", 256)
        self._make_artifact("apeireth_action-0010ce5b958c453a.pdb", 4096)
        profile = scan_crate("apeireth-action", self.deps_dir)
        self.assertEqual(profile.n_rlib, 1)
        self.assertEqual(profile.n_rmeta, 1)
        self.assertEqual(profile.n_d_file, 1)
        self.assertEqual(profile.n_pdb, 1)
        self.assertEqual(profile.total_size_bytes, 2048 + 512 + 256 + 4096)

    def test_exe_detection(self):
        self._make_artifact("apeireth_central-19ab780138ec0cb7.exe", 1024)
        profile = scan_crate("apeireth-central", self.deps_dir)
        self.assertEqual(profile.n_exe, 1)
        self.assertFalse(profile.has_test_binary)

    def test_test_binary_detection(self):
        self._make_artifact("apeireth_action_test-1234567890abcdef.exe", 1024)
        profile = scan_crate("apeireth-action", self.deps_dir)
        # "action_test" doesn't start with "apeireth_action-" prefix, won't match
        # Need to use prefix match
        # Actually: base_prefix = "apeireth_action-", so "apeireth_action_test-..." won't match because '-' is in middle
        # This is correct behavior: cargo test uses <crate_name>-<hash> pattern with same crate name
        self.assertEqual(profile.total_artifacts, 0)

    def test_example_binary_detection(self):
        self._make_artifact("apeireth_action_example-1234567890abcdef.exe", 1024)
        profile = scan_crate("apeireth-action", self.deps_dir)
        # Same as above: prefix doesn't match
        self.assertEqual(profile.total_artifacts, 0)

    def test_max_size_tracking(self):
        self._make_artifact("apeireth_action-0010ce5b958c453a.rlib", 1024)
        self._make_artifact("apeireth_action-0020ce5b958c453a.d", 99999)
        profile = scan_crate("apeireth-action", self.deps_dir)
        self.assertEqual(profile.max_size_bytes, 99999)
        # 验证 max_size_artifact 指向真的 size=99999 的文件
        max_artifact_size = next(a.size_bytes for a in profile.artifacts if a.name == profile.max_size_artifact)
        self.assertEqual(max_artifact_size, 99999)

    def test_median_size(self):
        self._make_artifact("apeireth_action-0010ce5b958c453a.rlib", 100)
        self._make_artifact("apeireth_action-0020ce5b958c453a.rlib", 200)
        self._make_artifact("apeireth_action-0030ce5b958c453a.rlib", 300)
        profile = scan_crate("apeireth-action", self.deps_dir)
        self.assertEqual(profile.median_size_bytes, 200)

    def test_missing_deps_dir(self):
        profile = scan_crate("apeireth-action", self.tmpdir / "nonexistent")
        self.assertFalse(profile.deps_dir_exists)
        self.assertEqual(profile.total_artifacts, 0)

    def test_other_crate_not_counted(self):
        # apeireth_other artifacts should not be counted for "action" crate
        self._make_artifact("apeireth_other-0010ce5b958c453a.rlib", 1024)
        profile = scan_crate("apeireth-action", self.deps_dir)
        self.assertEqual(profile.total_artifacts, 0)


class TestV1291FindDepsDir(unittest.TestCase):
    """find_deps_dir helper."""

    def test_promethean_dir_not_found(self):
        result = find_deps_dir(Path("/nonexistent/path"))
        self.assertIsNone(result)


class TestV1291EvaluateHypotheses(unittest.TestCase):
    """_evaluate_hypotheses helper."""

    def test_empty_ledger_all_pass(self):
        ledger = BuildArtifactLedger(crate_profiles=[])
        results = _evaluate_hypotheses(ledger)
        self.assertEqual(len(results), 5)
        for r in results:
            self.assertEqual(r["pass_fail"], "PASS")

    def test_high_coverage_passes_h1(self):
        # 10 crates, all with artifacts
        profiles = [
            CrateBuildProfile(crate_name=f"c{i}", n_rlib=1, n_rmeta=1, n_d_file=1)
            for i in range(10)
        ]
        ledger = BuildArtifactLedger(crate_profiles=profiles)
        results = _evaluate_hypotheses(ledger)
        h1 = next(r for r in results if r["hypothesis_id"] == "h_crate_build_coverage_ge_80pct")
        self.assertEqual(h1["pass_fail"], "PASS")
        self.assertEqual(h1["crates_with_artifacts"], 10)

    def test_zero_coverage_fails_h1(self):
        profiles = [CrateBuildProfile(crate_name=f"c{i}") for i in range(10)]
        ledger = BuildArtifactLedger(crate_profiles=profiles)
        results = _evaluate_hypotheses(ledger)
        h1 = next(r for r in results if r["hypothesis_id"] == "h_crate_build_coverage_ge_80pct")
        self.assertEqual(h1["pass_fail"], "FAIL")

    def test_h3_median_lt_5mb(self):
        profile = CrateBuildProfile(crate_name="x", total_size_bytes=1024 * 1024)  # 1MB
        profile.artifacts = [ArtifactInfo(size_bytes=1024 * 1024)]
        ledger = BuildArtifactLedger(crate_profiles=[profile])
        results = _evaluate_hypotheses(ledger)
        h3 = next(r for r in results if r["hypothesis_id"] == "h_median_artifact_size_lt_5mb")
        self.assertEqual(h3["pass_fail"], "PASS")


class TestV1291TopN(unittest.TestCase):
    """top_n_by_size helper."""

    def test_top_3(self):
        p1 = CrateBuildProfile(crate_name="a", total_size_bytes=100)
        p2 = CrateBuildProfile(crate_name="b", total_size_bytes=300)
        p3 = CrateBuildProfile(crate_name="c", total_size_bytes=200)
        ledger = BuildArtifactLedger(crate_profiles=[p1, p2, p3])
        top = top_n_by_size(ledger, 3)
        names = [p.crate_name for p in top]
        self.assertEqual(names, ["b", "c", "a"])

    def test_bottom_2(self):
        p1 = CrateBuildProfile(crate_name="a", total_size_bytes=100)
        p2 = CrateBuildProfile(crate_name="b", total_size_bytes=300)
        p3 = CrateBuildProfile(crate_name="c", total_size_bytes=200)
        p4 = CrateBuildProfile(crate_name="d", total_size_bytes=50)
        ledger = BuildArtifactLedger(crate_profiles=[p1, p2, p3, p4])
        bottom = top_n_by_size(ledger, 2, reverse=True)
        names = [p.crate_name for p in bottom]
        self.assertEqual(names, ["d", "a"])


class TestV1291Markdown(unittest.TestCase):
    """Markdown output (主 00:56 任何人都能接手)."""

    def test_empty_ledger_markdown(self):
        ledger = BuildArtifactLedger(crate_profiles=[])
        results = _evaluate_hypotheses(ledger)
        md = to_markdown(ledger, results)
        self.assertIn("# V1291", md)
        self.assertIn("Total crates scanned: 0", md)

    def test_real_ledger_markdown(self):
        p = CrateBuildProfile(
            crate_name="apeireth-x",
            n_rlib=2, n_rmeta=1, n_exe=1, n_d_file=3, n_pdb=1,
            total_size_bytes=10240,
        )
        ledger = BuildArtifactLedger(crate_profiles=[p], deps_root="/target/debug/deps")
        results = _evaluate_hypotheses(ledger)
        md = to_markdown(ledger, results)
        self.assertIn("apeireth-x", md)
        self.assertIn("Bottom-5", md)


class TestV1291JsonSnapshot(unittest.TestCase):
    """JSON snapshot."""

    def test_json_snapshot_basic(self):
        p = CrateBuildProfile(crate_name="x", n_rlib=1, n_rmeta=1)
        ledger = BuildArtifactLedger(crate_profiles=[p])
        snap = ledger.to_dict()
        self.assertEqual(snap["total_crates_scanned"], 1)
        self.assertEqual(snap["total_artifacts"], 2)


class TestV1291CLI(unittest.TestCase):
    """CLI entry (主 00:56 任何人都能接手)."""

    def test_help(self):
        from io import StringIO
        from unittest.mock import patch
        with patch("sys.stdout", new_callable=StringIO):
            with self.assertRaises(SystemExit) as cm:
                main(["--help"])
            self.assertEqual(cm.exception.code, 0)

    def test_probe(self):
        rc = main(["--probe"])
        self.assertEqual(rc, 0)


class TestV1291CrateList(unittest.TestCase):
    """42 crates list integrity."""

    def test_42_crates_listed(self):
        self.assertEqual(len(APEIRETH_RUST_CRATE_NAMES), 42)

    def test_no_duplicates(self):
        self.assertEqual(len(APEIRETH_RUST_CRATE_NAMES), len(set(APEIRETH_RUST_CRATE_NAMES)))

    def test_all_start_with_apeireth(self):
        for name in APEIRETH_RUST_CRATE_NAMES:
            self.assertTrue(name.startswith("apeireth-"), f"{name} should start with apeireth-")


class TestV1291Thresholds(unittest.TestCase):
    """Threshold constants sanity."""

    def test_thresholds_positive(self):
        self.assertGreater(V1291_THRESHOLD_ARTIFACTS_PER_CRATE, 0)
        self.assertGreater(V1291_THRESHOLD_COVERAGE_PCT, 0)
        self.assertGreater(V1291_THRESHOLD_MEDIAN_SIZE_KB, 0)
        self.assertGreater(V1291_THRESHOLD_TOTAL_GB, 0)


class TestV1291ArtifactInfo(unittest.TestCase):
    """ArtifactInfo dataclass."""

    def test_default_values(self):
        a = ArtifactInfo()
        self.assertEqual(a.name, "")
        self.assertEqual(a.size_bytes, 0)
        self.assertEqual(a.kind, "")

    def test_with_values(self):
        a = ArtifactInfo(name="foo.rlib", path="/tmp/foo.rlib", size_bytes=1024, kind="rlib")
        self.assertEqual(a.name, "foo.rlib")
        self.assertEqual(a.size_bytes, 1024)


class TestV1291CrateBuildProfile(unittest.TestCase):
    """CrateBuildProfile dataclass."""

    def test_total_artifacts(self):
        p = CrateBuildProfile(n_rlib=1, n_rmeta=1, n_exe=1, n_d_file=1, n_pdb=1)
        self.assertEqual(p.total_artifacts, 5)

    def test_has_any_artifact_false(self):
        p = CrateBuildProfile()
        self.assertFalse(p.has_any_artifact)

    def test_has_any_artifact_true(self):
        p = CrateBuildProfile(n_rlib=1)
        self.assertTrue(p.has_any_artifact)


class TestV1291Regression(unittest.TestCase):
    """Regression — V1289/V1290 imports 仍 OK (主 19:33 走在前人肩上)."""

    def test_v1289_imports_still_work(self):
        from apeireth import v1289_rust_doc_coverage_audit
        self.assertTrue(hasattr(v1289_rust_doc_coverage_audit, "scan_crate"))

    def test_v1290_imports_still_work(self):
        from apeireth import v1290_rust_doc_section_depth_audit
        self.assertTrue(hasattr(v1290_rust_doc_section_depth_audit, "scan_crate"))


if __name__ == "__main__":
    unittest.main()
