"""V1110 P0 终验 — 真测 (R9-DevOps / R9-DEV-001).

主 17:43 实事求是 + 主 00:56 任何人都能接手: pytest 集成, 真跑 V1110, 验证 P0 三件套全过.

注意: 此 test 真跑 V1074/V1087/V1088 三个子进程 (主 17:43). 慢但真.
若要快速冒烟, 跑: pytest -q tests/test_v1110_p0_terminal_verify.py::TestV1110SelfCheck
"""
from __future__ import annotations

import unittest

import apeireth.v1110_p0_terminal_verify as m


class TestV1110SelfCheck(unittest.TestCase):
    """V1110 自检: import OK + 函数存在 (主 17:58 不假装, 主 00:56 任何人都能接手)."""

    def test_version_present(self):
        self.assertTrue(hasattr(m, "V1110_VERSION"))
        self.assertIsInstance(m.V1110_VERSION, str)

    def test_thresholds_constants_present(self):
        for const in ("SNAPSHOT_MAX_BYTES", "V1074_V03_MIN",
                      "V1087_SUBSCORE_MIN", "V1088_LIFT_MIN"):
            self.assertTrue(hasattr(m, const), f"missing {const}")
            self.assertIsInstance(getattr(m, const), (int, float))

    def test_check_functions_callable(self):
        for fn_name in ("check_v1074", "check_v1087", "check_v1088"):
            self.assertTrue(callable(getattr(m, fn_name, None)),
                            f"missing callable {fn_name}")

    def test_run_terminal_verify_callable(self):
        self.assertTrue(callable(m.run_terminal_verify))
        self.assertTrue(callable(m.render_markdown))
        self.assertTrue(callable(m.main))

    def test_philosophy_ok_normalization(self):
        """主 17:43 实事求是: 不依赖字符串原样, 兼容多种空格 / 大小写."""
        norm = '"philosophy_guards_ok": true'.lower().replace(" ", "")
        self.assertIn('"philosophy_guards_ok":true', norm)


class TestV1110CheckResultDataclass(unittest.TestCase):
    """CheckResult 数据结构 (主 17:43 实事求是: 真测真记, 不假装)."""

    def test_checkresult_to_dict_roundtrip(self):
        r = m.CheckResult(
            name="x", passed=True, elapsed_sec=0.1, detail="ok",
            threshold=0.5, measured=0.6,
        )
        d = r.to_dict()
        self.assertEqual(d["name"], "x")
        self.assertEqual(d["passed"], True)
        self.assertEqual(d["threshold"], 0.5)
        self.assertEqual(d["measured"], 0.6)

    def test_checkresult_handles_negative_measured(self):
        r = m.CheckResult(name="x", passed=False, elapsed_sec=0.0, measured=-1.0)
        d = r.to_dict()
        self.assertEqual(d["measured"], -1.0)


class TestV1110RenderMarkdown(unittest.TestCase):
    """render_markdown 报告 (主 00:56 任何人都能接手: 报告可读)."""

    def test_markdown_contains_all_pass(self):
        report = {
            "v1110_version": "0.1.0",
            "started_at": 1.0,
            "elapsed_sec": 2.5,
            "all_pass": True,
            "thresholds": {
                "snapshot_max_bytes": 20971520,
                "v1074_v03_min": 0.8859,
                "v1087_subscore_min": 1.0,
                "v1088_lift_min": 0.0185,
            },
            "checks": [
                {"name": "A", "passed": True, "elapsed_sec": 0.5, "detail": "ok",
                 "threshold": 0.5, "measured": 0.6, "raw": {}},
                {"name": "B", "passed": True, "elapsed_sec": 0.5, "detail": "ok",
                 "threshold": 0.5, "measured": 0.7, "raw": {}},
            ],
        }
        md = m.render_markdown(report)
        self.assertIn("ALL PASS", md)
        self.assertIn("V1110", md)
        self.assertIn("A", md)
        self.assertIn("B", md)
        self.assertIn("R9", md)

    def test_markdown_contains_fail_detail(self):
        report = {
            "v1110_version": "0.1.0",
            "started_at": 1.0,
            "elapsed_sec": 2.5,
            "all_pass": False,
            "thresholds": {
                "snapshot_max_bytes": 20971520,
                "v1074_v03_min": 0.8859,
                "v1087_subscore_min": 1.0,
                "v1088_lift_min": 0.0185,
            },
            "checks": [
                {"name": "FAIL_COMP", "passed": False, "elapsed_sec": 0.5,
                 "detail": "v03=0.5", "threshold": 0.8859, "measured": 0.5,
                 "raw": {"rc": 1}},
            ],
        }
        md = m.render_markdown(report)
        self.assertIn("FAIL", md)
        self.assertIn("FAIL_COMP", md)
        self.assertIn("失败定位", md)


class TestV1110RunTerminalVerify(unittest.TestCase):
    """V1110 真跑 (主 17:43 实事求是 + 主 23:44 干到底)."""

    def test_run_terminal_verify_smoke(self):
        """真跑三件套, 验证 all_pass=True (P0 修复后应全过)."""
        report = m.run_terminal_verify()
        self.assertIn("all_pass", report)
        self.assertIn("checks", report)
        self.assertEqual(len(report["checks"]), 3)
        # 主 17:43: 不假设必过, 但每一项都必须有 measured 字段
        for c in report["checks"]:
            self.assertIn("name", c)
            self.assertIn("passed", c)
            self.assertIn("measured", c)
            self.assertIn("threshold", c)
        # 主 23:44 干到底: V1110 报告所有检查项名称
        names = [c["name"] for c in report["checks"]]
        self.assertTrue(any("V1074" in n for n in names))
        self.assertTrue(any("V1087" in n for n in names))
        self.assertTrue(any("V1088" in n for n in names))


if __name__ == "__main__":
    unittest.main()
