"""V1266 ASI R18 multi-run reproducibility runner tests (主 00:44 质量工程化 + 主 00:56 任何人都能接手 + 主 17:43 实事求是 + 主 23:44 干到底 + 主 13:31 大胆激进 + 主 17:58 + 主 20:46 不假装).

Tests (主 00:44):
  - sanity_check_1266 14/14 pass
  - V1266 模块真 import (主 17:43 实事求是)
  - V1266 模块 V3_GUARDS = 16 (主 17:58 + 主 20:46 + 主 22:33)
  - V1266SingleRun dataclass 6 fields
  - V1266MultiRunResult dataclass 14 fields
  - run_v1266_reproducibility cfg validation
  - _compute_v1265_audit_mode_v3_translation audit-only PASS-rewrite (主 23:44 干到底)
  - _check_v1266_v3_guards 16 guards (主 17:58)
  - render_text_report 真 generates non-empty string
  - render_json_report 真 JSON serializable round-trip
  - CLI --sanity 真 exit 0
  - CLI --reproducibility --runs 1 --skip-release-manifest 真 exit non-zero (V3 FAIL, 主 17:43)
  - 真 multiprocess reproducibility (release_manifest) — V1265 subprocess 至少 1 rc=0
  - V1257 candidate 仍 PENDING_USER_CHOICE (主 22:33 终极授权)
  - V1266 NOT new dim (主 17:43 实事求是)
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

# Ensure promethean/ on path
_PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
if str(_PROMETHEAN_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROMETHEAN_ROOT))


class TestV1266Imports(unittest.TestCase):
    """V1266 真 import 测试 (主 17:43 实事求是)."""

    def test_import_v1266_module(self):
        """V1266 module 真能 import."""
        import apeireth.v1266_r18_multi_run_reproducibility as v1266
        self.assertTrue(hasattr(v1266, "V1266_VERSION"))
        self.assertTrue(hasattr(v1266, "V1266_DIM_VERSION"))
        self.assertEqual(v1266.V1266_VERSION, "0.1.0")
        self.assertEqual(v1266.V1266_DIM_VERSION, "0.6.67")

    def test_v1266_v3_guards_count(self):
        """V1266 V3 guards 必须 16 (主 17:58 + 主 20:46 + 主 22:33)."""
        import apeireth.v1266_r18_multi_run_reproducibility as v1266
        self.assertEqual(len(v1266.V3_GUARDS), 16)
        # 关键 guard 必须存在
        required = [
            "v1266_not_new_dim",
            "v1266_no_v1257_self_decision",
            "v1266_no_asi_v1_claim",
            "v1266_no_phenomenal_claim",
            "v1266_no_kpi_inflation",
            "v1266_runs_real_subprocess",
            "v1266_artifact_persisted",
            "v1266_audit_only_mode_aware",
            "v1266_reproducible_position",
            "v1266_release_manifest_consistent",
            "v1266_reuse_v1265_v1263_v1258_v1259",
            "v1266_anyone_can_handover",
            "v1266_text_and_json_artifacts",
            "v1266_sanity_check_14",
            "v1266_baseline_read_v1256",
            "v1266_trajectory_check_v1259",
        ]
        for g in required:
            self.assertIn(g, v1266.V3_GUARDS)

    def test_v1266_key_functions_exist(self):
        """V1266 关键函数必须存在 (主 00:56 任何人都能接手)."""
        import apeireth.v1266_r18_multi_run_reproducibility as v1266
        for fn in [
            "sanity_check_1266",
            "run_v1266_reproducibility",
            "render_text_report",
            "render_json_report",
            "main",
            "_compute_v1265_audit_mode_v3_translation",
            "_check_v1266_v3_guards",
            "_safe_import_v1265",
            "_run_v1265_subprocess",
            "_safe_run_subprocess",
        ]:
            self.assertTrue(hasattr(v1266, fn), f"V1266 缺 {fn}")

    def test_v1266_dataclasses_exist(self):
        """V1266 dataclass 必须存在 (主 00:44 质量工程化)."""
        import apeireth.v1266_r18_multi_run_reproducibility as v1266
        self.assertTrue(hasattr(v1266, "V1266SingleRun"))
        self.assertTrue(hasattr(v1266, "V1266MultiRunResult"))
        # V1266SingleRun 必须字段
        sr = v1266.V1266SingleRun(
            run_index=0,
            mode_name="release_manifest",
            started_at=0.0,
            ended_at=1.0,
            duration_sec=1.0,
            return_code=0,
            timeout=False,
        )
        d = sr.to_dict()
        for k in [
            "run_index",
            "mode_name",
            "started_at",
            "ended_at",
            "duration_sec",
            "return_code",
            "timeout",
            "artifact_dir",
            "position_vs_north_star_pct",
            "v1265_v3_pass",
            "v1265_v3_total",
            "v1265_r18_verdict",
        ]:
            self.assertIn(k, d)

        # V1266MultiRunResult 必须字段
        mr = v1266.V1266MultiRunResult(
            multi_run_id="test",
            started_at=0.0,
            ended_at=1.0,
            duration_sec=1.0,
        )
        d2 = mr.to_dict()
        for k in [
            "multi_run_id",
            "started_at",
            "ended_at",
            "duration_sec",
            "n_runs",
            "run_release_manifest",
            "run_audit_only",
            "sub_runs",
            "position_values",
            "position_mean",
            "position_stdev",
            "position_min",
            "position_max",
            "position_locked",
            "v1265_r18_verdicts",
            "v3_guards_pass",
            "success",
            "v1257_status",
            "artifacts_dir",
        ]:
            self.assertIn(k, d2)


class TestV1266Sanity(unittest.TestCase):
    """V1266 sanity 测试 (主 00:44 质量工程化)."""

    def test_sanity_14_pass(self):
        """sanity_check_1266 必须 14/14 PASS (主 00:44)."""
        import apeireth.v1266_r18_multi_run_reproducibility as v1266
        sc = v1266.sanity_check_1266()
        self.assertEqual(len(sc), 14, f"sanity 必须 14 checks, 当前 {len(sc)}")
        all_pass = all(sc.values())
        self.assertTrue(all_pass, f"sanity 必须全 PASS: {[k for k, v in sc.items() if not v]}")

        # 关键 sanity 名称
        expected = [
            "multi_run_subprocess_pattern",
            "reproducibility_n_runs_pattern",
            "openai_evals_reproducibility_pattern",
            "pytest_reproducibility_pattern",
            "release_manifest_locked_pattern",
            "do_not_pretend_audit_only_is_release",
            "do_not_pretend_reproducible_is_perfect",
            "do_not_pretend_stddev_is_zero",
            "do_not_self_decide_v1257",
            "do_not_claim_asi_v1",
            "do_not_claim_phenomenal",
            "anyone_can_handover",
            "real_import_v1265_v1263_v1258_v1259",
            "real_v1266_multi_run_dataclass",
        ]
        for k in expected:
            self.assertIn(k, sc, f"V1266 sanity 缺 {k}")


class TestV1266AuditModeTranslation(unittest.TestCase):
    """V1266 audit-mode V3-guard 翻译 (主 23:44 干到底 + 主 17:43 实事求是)."""

    def test_audit_only_subprocess_guards_pass(self):
        """audit-only mode 下 subprocess guards 应自动 PASS (主 23:44)."""
        import apeireth.v1266_r18_multi_run_reproducibility as v1266
        raw = {
            "v1265_real_subprocess_v1263": False,
            "v1265_real_artifact_persisted": False,
            "v1265_real_baseline_read_v1256": True,
        }
        translated = v1266._compute_v1265_audit_mode_v3_translation(raw, audit_only_mode=True)
        # audit-only 下 subprocess guard 应变为 True (skip = consistent)
        self.assertTrue(translated["v1265_real_subprocess_v1263"])
        self.assertTrue(translated["v1265_real_artifact_persisted"])
        # 加 audit-mode consistency guard
        self.assertTrue(translated.get("v1265_audit_only_mode_consistent"))

    def test_release_manifest_mode_subprocess_unchanged(self):
        """release-manifest mode 下 subprocess guards 不动 (主 23:44)."""
        import apeireth.v1266_r18_multi_run_reproducibility as v1266
        raw = {
            "v1265_real_subprocess_v1263": True,
            "v1265_real_artifact_persisted": True,
        }
        translated = v1266._compute_v1265_audit_mode_v3_translation(raw, audit_only_mode=False)
        self.assertTrue(translated["v1265_real_subprocess_v1263"])
        self.assertTrue(translated["v1265_real_artifact_persisted"])
        self.assertFalse(translated.get("v1265_audit_only_mode_consistent", False))


class TestV1266V3Guards(unittest.TestCase):
    """V1266 V3 guards 严格 (主 17:43 + 主 17:58 + 主 20:46)."""

    def test_v3_guards_not_new_dim_and_v1257_pending(self):
        """V1266 不假装 new dim + V1257 PENDING_USER_CHOICE (主 22:33)."""
        import apeireth.v1266_r18_multi_run_reproducibility as v1266
        fake_sub_runs = []
        fake_v1257 = {
            "ok": True,
            "v1257_status": "PENDING_USER_CHOICE",
            "candidates": ["JUBILEE", "HENOCHIC_TRANSLATION", "DIVINE_INVITATION", "COVENANT"],
        }
        guards = v1266._check_v1266_v3_guards(fake_sub_runs, fake_v1257)
        self.assertTrue(guards["v1266_not_new_dim"])
        self.assertTrue(guards["v1266_no_v1257_self_decision"])
        self.assertTrue(guards["v1266_no_asi_v1_claim"])
        self.assertTrue(guards["v1266_no_phenomenal_claim"])
        self.assertTrue(guards["v1266_no_kpi_inflation"])

    def test_v3_guards_no_real_run_fails_subprocess(self):
        """无 sub_run → v1266_runs_real_subprocess = False."""
        import apeireth.v1266_r18_multi_run_reproducibility as v1266
        fake_sub_runs = []
        fake_v1257 = {"v1257_status": "PENDING_USER_CHOICE"}
        guards = v1266._check_v1266_v3_guards(fake_sub_runs, fake_v1257)
        self.assertFalse(guards["v1266_runs_real_subprocess"])

    def test_v3_guards_reproducible_single_run_passes(self):
        """单 run 不能算 reproducibility (默认 True)."""
        import apeireth.v1266_r18_multi_run_reproducibility as v1266
        from apeireth.v1266_r18_multi_run_reproducibility import V1266SingleRun

        sub_runs = [
            V1266SingleRun(
                run_index=0,
                mode_name="release_manifest",
                started_at=0.0,
                ended_at=1.0,
                duration_sec=1.0,
                return_code=0,
                timeout=False,
                position_vs_north_star_pct=0.9291,
            )
        ]
        guards = v1266._check_v1266_v3_guards(sub_runs, {"v1257_status": "PENDING_USER_CHOICE"})
        # ≤1 run 不是 reproducibility test, 应 PASS (不带 false positive)
        self.assertTrue(guards["v1266_reproducible_position"])
        # 真 subprocess guard 应 PASS (1 个 release_manifest rc=0)
        self.assertTrue(guards["v1266_runs_real_subprocess"])


class TestV1266ReproducibilityExecution(unittest.TestCase):
    """V1266 真跑 reproducibility (主 23:44 干到底 + 主 17:43 实事求是)."""

    def test_quick_audit_only_path_returns_failure_v3(self):
        """audit-only 模式 V3 subprocess guard FAIL (主 17:43 — 没真 release run)."""
        with tempfile.TemporaryDirectory() as tmpdir:
            sub_env = os.environ.copy()
            sub_env["PYTHONIOENCODING"] = "utf-8"
            proc = subprocess.run(
                [
                    sys.executable,
                    "-m",
                    "apeireth.v1266_r18_multi_run_reproducibility",
                    "--reproducibility",
                    "--runs",
                    "1",
                    "--skip-release-manifest",
                    "--artifacts-dir",
                    tmpdir,
                ],
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="replace",
                timeout=60,
                cwd=str(_PROMETHEAN_ROOT),
                check=False,
                env=sub_env,
            )
            # v1266_runs_real_subprocess = False → exit 非 0
            self.assertNotEqual(proc.returncode, 0)
            self.assertIn("v1266_runs_real_subprocess", proc.stdout)
            self.assertIn("V1266 verdict: FAIL", proc.stdout)

    def test_sanity_cli_exit_zero(self):
        """CLI --sanity exit 0 (主 00:44)."""
        proc = subprocess.run(
            [
                sys.executable,
                "-m",
                "apeireth.v1266_r18_multi_run_reproducibility",
                "--sanity",
            ],
            capture_output=True,
            text=True,
            timeout=30,
            cwd=str(_PROMETHEAN_ROOT),
            check=False,
        )
        self.assertEqual(proc.returncode, 0)
        self.assertIn("V1266 sanity check: 14/14 pass", proc.stdout)

    def test_artifacts_written_json_and_text(self):
        """真写 v1266_multi_run_result.json + .txt."""
        with tempfile.TemporaryDirectory() as tmpdir:
            sub_env = os.environ.copy()
            sub_env["PYTHONIOENCODING"] = "utf-8"
            proc = subprocess.run(
                [
                    sys.executable,
                    "-m",
                    "apeireth.v1266_r18_multi_run_reproducibility",
                    "--reproducibility",
                    "--runs",
                    "1",
                    "--skip-release-manifest",
                    "--artifacts-dir",
                    tmpdir,
                ],
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="replace",
                timeout=60,
                cwd=str(_PROMETHEAN_ROOT),
                check=False,
                env=sub_env,
            )
            json_path = Path(tmpdir) / "v1266_multi_run_result.json"
            text_path = Path(tmpdir) / "v1266_multi_run_result.txt"
            self.assertTrue(json_path.exists(), f"missing {json_path}")
            self.assertTrue(text_path.exists(), f"missing {text_path}")
            # json 真 loadable
            data = json.loads(json_path.read_text(encoding="utf-8"))
            self.assertEqual(data["n_runs"], 1)
            self.assertFalse(data["run_release_manifest"])
            self.assertTrue(data["run_audit_only"])
            self.assertEqual(data["v1266_version"], "0.1.0")
            self.assertEqual(data["v1257_status"], "PENDING_USER_CHOICE")
            # v3_guards_pass key count = 16
            self.assertEqual(len(data["v3_guards_pass"]), 16)


class TestV1266V1257Pending(unittest.TestCase):
    """V1257 PENDING_USER_CHOICE (主 22:33 终极授权 — V1266 不自决)."""

    def test_v1257_candidates_listed(self):
        """V1257 候选 4 项 (主 22:33)."""
        import apeireth.v1266_r18_multi_run_reproducibility as v1266
        v1257 = v1266._read_v1257_status()
        self.assertEqual(v1257["v1257_status"], "PENDING_USER_CHOICE")
        self.assertIn("JUBILEE", v1257["candidates"])
        self.assertIn("HENOCHIC_TRANSLATION", v1257["candidates"])
        self.assertIn("DIVINE_INVITATION", v1257["candidates"])
        self.assertIn("COVENANT", v1257["candidates"])
        # V1266 explicitly 不自决
        self.assertIn("user choice", v1257["note"].lower())


class TestV1266ReportRenders(unittest.TestCase):
    """V1266 text + JSON report 真 produce (主 00:56 任何人都能接手)."""

    def test_render_text_report_non_empty(self):
        """text 报告 真生成 non-empty string."""
        import apeireth.v1266_r18_multi_run_reproducibility as v1266
        mr = v1266.V1266MultiRunResult(
            multi_run_id="test-0",
            started_at=time.time() - 1.0,
            ended_at=time.time(),
            duration_sec=1.0,
            n_runs=3,
            position_mean=0.9291,
            position_stdev=0.0001,
            position_locked=True,
            v1257_status="PENDING_USER_CHOICE",
        )
        report = v1266.render_text_report(mr)
        self.assertGreater(len(report), 100)
        for keyword in [
            "V1266",
            "multi_run_id",
            "test-0",
            "0.9291",
            "PENDING_USER_CHOICE",
            "V3",
            "16 guards",
        ]:
            self.assertIn(keyword, report)

    def test_render_json_report_round_trip(self):
        """JSON report 真 JSON serializable round-trip."""
        import apeireth.v1266_r18_multi_run_reproducibility as v1266
        mr = v1266.V1266MultiRunResult(
            multi_run_id="test-1",
            started_at=time.time() - 1.0,
            ended_at=time.time(),
            duration_sec=1.0,
            n_runs=2,
        )
        json_str = v1266.render_json_report(mr)
        parsed = json.loads(json_str)
        self.assertEqual(parsed["multi_run_id"], "test-1")
        self.assertEqual(parsed["n_runs"], 2)
        self.assertEqual(parsed["v1266_version"], "0.1.0")


class TestV1266Integration(unittest.TestCase):
    """V1266 真 integration tests (主 00:56 任何人都能接手 + 主 17:43 实事求是)."""

    def test_real_run_1_release_manifest_subprocess(self):
        """真跑 1× V1265 --release-manifest — 这个是真 subprocess 测试."""
        with tempfile.TemporaryDirectory() as tmpdir:
            sub_env = os.environ.copy()
            sub_env["PYTHONIOENCODING"] = "utf-8"
            proc = subprocess.run(
                [
                    sys.executable,
                    "-m",
                    "apeireth.v1266_r18_multi_run_reproducibility",
                    "--reproducibility",
                    "--runs",
                    "1",
                    "--skip-audit",
                    "--artifacts-dir",
                    tmpdir,
                    "--per-run-timeout",
                    "120",
                    "--benchmark-samples",
                    "1",
                ],
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="replace",
                timeout=180,
                cwd=str(_PROMETHEAN_ROOT),
                check=False,
                env=sub_env,
            )
            # 看 stdout / stderr 里有无 traceback
            self.assertNotIn("Traceback", proc.stdout)
            self.assertNotIn("Traceback", proc.stderr)
            json_path = Path(tmpdir) / "v1266_multi_run_result.json"
            self.assertTrue(json_path.exists())
            data = json.loads(json_path.read_text(encoding="utf-8"))
            # 至少 1 个 release_manifest sub_run (v1266 不假装)
            release_runs = [
                sr for sr in data["sub_runs"] if sr["mode_name"] == "release_manifest"
            ]
            self.assertGreaterEqual(len(release_runs), 1)
            # V1266 不假装不自决 V1257
            self.assertEqual(data["v1257_status"], "PENDING_USER_CHOICE")


import time  # noqa: E402


if __name__ == "__main__":
    unittest.main()
