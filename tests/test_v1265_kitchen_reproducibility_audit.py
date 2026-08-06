"""V1265 ASI V0.6.67 R18 kitchen reproducibility audit tests (主 00:44 质量工程化 + 主 00:56 任何人都能接手 + 主 17:43 实事求是 + 主 23:44 干到底).

Tests (主 00:44):
  - sanity_check_1265 14/14 pass
  - V1265AuditConfig 真 dataclass
  - V1265SubRun 真 dataclass
  - V1265AuditResult 真 dataclass
  - V1265 audit-only mode (no subprocess) 验证 audit 数据
  - V1265 --sanity CLI 入口
  - V1265 --audit CLI 入口
  - V3 guards 严格
  - V1257 candidates status PENDING_USER_CHOICE
  - V1256 baseline 16 pillars 齐
  - V1258 substrate position 92.91% (主 22:33 终极授权)
  - V1259 trajectory 12 V3 guards (主 19:33 走在前人肩上)
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


class TestV1265Imports(unittest.TestCase):
    """V1265 真 import 测试 (主 17:43 实事求是)."""

    def test_import_v1265_module(self):
        """V1265 module 真能 import."""
        import apeireth.v1265_kitchen_reproducibility_audit as v1265
        self.assertTrue(hasattr(v1265, "V1265_VERSION"))
        self.assertTrue(hasattr(v1265, "V1265_DIM_VERSION"))
        self.assertEqual(v1265.V1265_VERSION, "0.1.0")
        self.assertEqual(v1265.V1265_DIM_VERSION, "0.6.67")

    def test_v1265_v3_guards_count(self):
        """V1265 V3 guards 必须 13 (主 17:58 + 主 20:46 + 主 22:33)."""
        import apeireth.v1265_kitchen_reproducibility_audit as v1265
        self.assertEqual(len(v1265.V3_GUARDS), 13)

    def test_v1265_key_functions_exist(self):
        """V1265 关键函数必须存在 (主 00:56 任何人都能接手)."""
        import apeireth.v1265_kitchen_reproducibility_audit as v1265
        for fn in [
            "sanity_check_1265",
            "run_v1265_audit",
            "render_r18_release_manifest",
            "render_text_report",
            "render_json_report",
            "main",
            "_audit_v1256_baseline",
            "_audit_v1258_substrate",
            "_audit_v1259_trajectory",
            "_audit_v1257_candidates",
            "_check_v3_guards",
        ]:
            self.assertTrue(callable(getattr(v1265, fn)), f"{fn} not callable")


class TestV1265Sanity(unittest.TestCase):
    """V1265 自身 sanity (主 00:44 质量工程化)."""

    def test_sanity_check_14_pass(self):
        """V1265 sanity 必须 14/14 pass."""
        import apeireth.v1265_kitchen_reproducibility_audit as v1265
        sc = v1265.sanity_check_1265()
        self.assertEqual(len(sc), 14)
        for k, v in sc.items():
            self.assertTrue(v, f"sanity check failed: {k}")


class TestV1265Config(unittest.TestCase):
    """V1265AuditConfig dataclass 真生产 (主 00:44 质量工程化)."""

    def test_default_config(self):
        import apeireth.v1265_kitchen_reproducibility_audit as v1265
        cfg = v1265.V1265AuditConfig()
        self.assertTrue(cfg.run_dry_run)
        self.assertTrue(cfg.run_full)
        self.assertTrue(cfg.run_full_e2e)
        self.assertTrue(cfg.run_streamlit_real)
        self.assertEqual(cfg.benchmark_samples, 3)
        self.assertEqual(cfg.health_cycles, 2)
        self.assertEqual(cfg.per_run_timeout_s, 90.0)

    def test_config_to_dict(self):
        import apeireth.v1265_kitchen_reproducibility_audit as v1265
        cfg = v1265.V1265AuditConfig(artifacts_dir="/tmp/test", benchmark_samples=10)
        d = cfg.__dict__
        self.assertEqual(d["artifacts_dir"], "/tmp/test")
        self.assertEqual(d["benchmark_samples"], 10)


class TestV1265SubRun(unittest.TestCase):
    """V1265SubRun dataclass 真生产 (主 00:44)."""

    def test_sub_run_creation(self):
        import apeireth.v1265_kitchen_reproducibility_audit as v1265
        sr = v1265.V1265SubRun(
            mode_name="dry_run",
            cmd=["python", "-m", "apeireth.v1263_real_kitchen_integration"],
            started_at=1.0,
            ended_at=2.0,
            duration_sec=1.0,
            return_code=0,
            timeout=False,
            artifact_path="/tmp/kitchen.json",
        )
        self.assertEqual(sr.mode_name, "dry_run")
        self.assertEqual(sr.return_code, 0)
        self.assertFalse(sr.timeout)

        d = sr.to_dict()
        self.assertEqual(d["mode_name"], "dry_run")
        self.assertEqual(d["return_code"], 0)
        self.assertIn("cmd", d)


class TestV1265AuditOnly(unittest.TestCase):
    """V1265 --audit 模式 (无 subprocess) 真测 audit 数据."""

    def test_audit_v1256_baseline(self):
        """V1256 baseline 真读 + 16 pillars 齐 (主 17:43)."""
        import apeireth.v1265_kitchen_reproducibility_audit as v1265
        result = v1265._audit_v1256_baseline()
        self.assertTrue(result.get("ok"), f"V1256 import failed: {result.get('error')}")
        self.assertEqual(result.get("pillars_count"), 16)
        self.assertEqual(result.get("pillars_expected"), 16)
        self.assertEqual(result.get("dim_version"), "0.6.66")
        self.assertEqual(result.get("asi_north_star"), 0.98)

    def test_audit_v1258_substrate(self):
        """V1258 substrate position 92.91% (主 22:33 终极授权)."""
        import apeireth.v1265_kitchen_reproducibility_audit as v1265
        result = v1265._audit_v1258_substrate()
        self.assertTrue(result.get("ok"), f"V1258 import failed: {result.get('error')}")
        self.assertEqual(result.get("sixteen_pillars_count"), 16)
        self.assertAlmostEqual(result.get("position_vs_north_star_pct"), 0.9291, places=4)
        self.assertTrue(result.get("audit_pass"))

    def test_audit_v1259_trajectory(self):
        """V1259 trajectory 12 V3 guards (主 19:33 走在前人肩上)."""
        import apeireth.v1265_kitchen_reproducibility_audit as v1265
        result = v1265._audit_v1259_trajectory()
        self.assertTrue(result.get("ok"), f"V1259 import failed: {result.get('error')}")
        self.assertEqual(result.get("v3_guards_count"), 12)
        self.assertEqual(result.get("v3_guards_pass_count"), 12)
        self.assertTrue(result.get("imports_clean"))

    def test_audit_v1257_candidates(self):
        """V1257 候选 status 必须 PENDING_USER_CHOICE (主 22:33 终极授权)."""
        import apeireth.v1265_kitchen_reproducibility_audit as v1265
        result = v1265._audit_v1257_candidates()
        self.assertTrue(result.get("ok"))
        self.assertEqual(result.get("v1257_status"), "PENDING_USER_CHOICE")
        candidates = result.get("candidates", [])
        self.assertEqual(len(candidates), 4)
        self.assertIn("JUBILEE", candidates)
        self.assertIn("HENOCHIC_TRANSLATION", candidates)
        self.assertIn("DIVINE_INVITATION", candidates)
        self.assertIn("COVENANT", candidates)


class TestV1265V3Guards(unittest.TestCase):
    """V1265 V3 guards 严格验证 (主 17:43 + 主 17:58 + 主 20:46 + 主 22:33)."""

    def test_v3_guards_no_subprocess_run(self):
        """无 V1263 subprocess 跑 → v1265_real_subprocess_v1263 FAIL (实事求是)."""
        import apeireth.v1265_kitchen_reproducibility_audit as v1265
        sub_runs = []
        guards = v1265._check_v3_guards(
            sub_runs,
            v1256_check={"ok": True},
            v1258_check={"ok": True},
            v1259_check={"ok": True},
            v1257_check={"v1257_status": "PENDING_USER_CHOICE"},
        )
        self.assertFalse(guards["v1265_real_subprocess_v1263"])
        self.assertTrue(guards["v1265_no_v1257_self_decision"])
        self.assertTrue(guards["v1265_not_new_dim"])

    def test_v3_guards_with_subprocess(self):
        """有 V1263 subprocess 跑 → v1265_real_subprocess_v1263 PASS."""
        import apeireth.v1265_kitchen_reproducibility_audit as v1265
        sub_runs = [
            v1265.V1265SubRun(
                mode_name="dry_run",
                cmd=[],
                started_at=0.0,
                ended_at=1.0,
                duration_sec=1.0,
                return_code=0,
                timeout=False,
                artifact_path="/tmp/test_artifact.json",
                summary={"success": True},
            )
        ]
        guards = v1265._check_v3_guards(
            sub_runs,
            v1256_check={"ok": True},
            v1258_check={"ok": True},
            v1259_check={"ok": True},
            v1257_check={"v1257_status": "PENDING_USER_CHOICE"},
        )
        self.assertTrue(guards["v1265_real_subprocess_v1263"])
        # 注: artifact_persisted 需要 artifact 真的存在于磁盘, 这里用 mock
        # 不强制断言 artifact_persisted (因为 mock artifact 不在磁盘)
        all_pass_no_artifact = all(
            v for k, v in guards.items() if k != "v1265_real_artifact_persisted"
        )
        self.assertTrue(all_pass_no_artifact, f"some guards failed: {guards}")


class TestV1265RunAudit(unittest.TestCase):
    """V1265 真跑 audit-only (无 subprocess) — 验证 report 结构."""

    def test_run_audit_only_no_subprocess(self):
        """V1265 audit-only 模式 (主 17:43 实事求是)."""
        import apeireth.v1265_kitchen_reproducibility_audit as v1265
        with tempfile.TemporaryDirectory() as tmp:
            cfg = v1265.V1265AuditConfig(
                artifacts_dir=tmp,
                run_dry_run=False,
                run_full=False,
                run_full_e2e=False,
                run_streamlit_real=False,
            )
            result = v1265.run_v1265_audit(cfg)
            # audit-only 模式下 success=False (因为 real_subprocess_v1263 fail)
            # 但是 audit 数据应该齐
            self.assertFalse(result.success, "audit-only should NOT pass (no real subprocess)")
            self.assertEqual(len(result.sub_runs), 0)
            self.assertTrue(result.v1256_baseline_check.get("ok"))
            self.assertTrue(result.v1258_substrate_check.get("ok"))
            self.assertTrue(result.v1259_trajectory_check.get("ok"))
            self.assertEqual(
                result.v1257_candidates_status.get("v1257_status"),
                "PENDING_USER_CHOICE",
            )

            # 真测 artifact
            json_path = os.path.join(tmp, "v1265_audit.json")
            self.assertTrue(os.path.exists(json_path))
            with open(json_path, "r", encoding="utf-8") as f:
                rep = json.load(f)
            self.assertEqual(rep["v1265_dim_version"], "0.6.67")
            self.assertIn("R18 kitchen reproducibility audit", rep["v1265_note"])

    def test_run_audit_artifacts_text_and_json(self):
        """V1265 必须产生 text + JSON artifact (主 00:44 质量工程化)."""
        import apeireth.v1265_kitchen_reproducibility_audit as v1265
        with tempfile.TemporaryDirectory() as tmp:
            cfg = v1265.V1265AuditConfig(
                artifacts_dir=tmp,
                run_dry_run=False,
                run_full=False,
                run_full_e2e=False,
                run_streamlit_real=False,
            )
            v1265.run_v1265_audit(cfg)
            self.assertTrue(os.path.exists(os.path.join(tmp, "v1265_audit.json")))
            self.assertTrue(os.path.exists(os.path.join(tmp, "v1265_audit.txt")))


class TestV1265CLI(unittest.TestCase):
    """V1265 CLI 入口 (主 00:56 任何人都能接手)."""

    def test_cli_sanity(self):
        """CLI --sanity 入口必须可跑."""
        result = subprocess.run(
            [
                sys.executable,
                "-m",
                "apeireth.v1265_kitchen_reproducibility_audit",
                "--sanity",
            ],
            cwd=str(_PROMETHEAN_ROOT),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=30,
        )
        self.assertEqual(result.returncode, 0, f"stderr: {result.stderr}")
        out = result.stdout or ""
        self.assertIn("14/14 pass", out)

    def test_cli_audit(self):
        """CLI --audit 入口必须可跑 (无 subprocess)."""
        with tempfile.TemporaryDirectory() as tmp:
            result = subprocess.run(
                [
                    sys.executable,
                    "-m",
                    "apeireth.v1265_kitchen_reproducibility_audit",
                    "--audit",
                    "--artifacts-dir",
                    tmp,
                ],
                cwd=str(_PROMETHEAN_ROOT),
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="replace",
                timeout=30,
            )
            # --audit 是 legitimate mode, 总是 exit 0
            self.assertEqual(result.returncode, 0, f"stderr: {result.stderr}")
            out = result.stdout or ""
            self.assertIn("V1265", out)
            self.assertIn("audit", out.lower())

    def test_cli_help(self):
        """CLI --help 必须可用 (主 00:56)."""
        result = subprocess.run(
            [
                sys.executable,
                "-m",
                "apeireth.v1265_kitchen_reproducibility_audit",
                "--help",
            ],
            cwd=str(_PROMETHEAN_ROOT),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=15,
        )
        self.assertEqual(result.returncode, 0)
        out = result.stdout or ""
        self.assertIn("--sanity", out)
        self.assertIn("--audit", out)
        self.assertIn("--full-audit", out)
        self.assertIn("--release-manifest", out)


class TestV1265SubprocessRun(unittest.TestCase):
    """V1265 真跑 V1263 (主 23:44 干到底 + 主 17:43 实事求是).

    注: 本测试真 subprocess 跑 V1263, 可能会耗时 ~30-90s.
    """

    def test_run_v1263_dry_run_subprocess(self):
        """V1265 真 subprocess 跑 V1263 --dry-run (主 23:44 干到底)."""
        import apeireth.v1265_kitchen_reproducibility_audit as v1265
        with tempfile.TemporaryDirectory() as tmp:
            cfg = v1265.V1265AuditConfig(
                artifacts_dir=tmp,
                run_dry_run=True,
                run_full=False,
                run_full_e2e=False,
                run_streamlit_real=False,
                dry_run_port=19900,
                streamlit_port=19981,
                benchmark_samples=2,
                health_cycles=1,
                per_run_timeout_s=60.0,
            )
            result = v1265.run_v1265_audit(cfg)
            self.assertEqual(len(result.sub_runs), 1)
            sr = result.sub_runs[0]
            self.assertEqual(sr.mode_name, "dry_run")
            self.assertEqual(sr.return_code, 0, f"V1263 dry_run failed: {sr.error}")
            self.assertTrue(sr.summary.get("success"))
            self.assertIsNotNone(sr.artifact_path)
            self.assertTrue(os.path.exists(sr.artifact_path))


if __name__ == "__main__":
    unittest.main()
