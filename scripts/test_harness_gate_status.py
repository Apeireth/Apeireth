#!/usr/bin/env python3
"""`scripts/harness.py` 的**门禁状态判定**自测（防「门禁本身被改坏」）。

# 为什么守卫也要有自测

2026-11-08 实测事故：本地闸门 `check_golden_baseline.py` 在**语料路径指错**时
**跳过并退出 0** ⇒ 人只看到 ✅ ⇒ 「准确度不许掉」这道闸门**一直没在跑**。

⇒ ⇒ 根因有两层：
1. 闸门脚本把「跳过」和「通过」**用同一个退出码**表达；
2. harness 把「**退出码 0**」直接当成「**跑过且通过**」。

⇒ ⇒ 本批修了两端（闸门脚本区分「没有语料」与「语料残缺」；harness 识别
`GATE_SKIP:` 标记 ⇒ 记 **SKIPPED** 而不是 PASS）⇒ ⇒ 所以**必须钉住它真的有牙**：

1. 打印了 `GATE_SKIP:` 且退出 0 ⇒ ⇒ 状态**必须**是 `SKIPPED`（**不是 PASS**）；
2. 退出 0 且没有该标记 ⇒ ⇒ 状态**必须**是 `PASS`；
3. 退出非 0 ⇒ ⇒ 状态**必须**是 `FAIL`；
4. **环境跳过**（`skip_kind = "env"`）**不阻断**裁决（环境本来就没有那些前置条件）
   ⇒ ⇒ 但**必须出现在理由里**（否则「没跑」在报告里就消失了 ⇒ 又变成「只看 ✅」）；
5. **真·scope 跳过**（本次 diff 不触发该门禁）⇒ ⇒ 不阻断；
6. **适用却没跑**的 required 门禁（既不是 env 跳过、也没有 scope 不相交）⇒ ⇒ **必须 BLOCKED**。

# 用法

```sh
python3 scripts/test_harness_gate_status.py
```
"""

from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def _load_harness():
    spec = importlib.util.spec_from_file_location("harness_under_test", ROOT / "scripts" / "harness.py")
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


HARNESS = _load_harness()


def _stub(directory: Path, name: str, body: str) -> list[str]:
    path = directory / f"{name}.py"
    path.write_text(body, encoding="utf-8")
    return [sys.executable, str(path)]


class GateStatusTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)

    def _run(self, cmd: list[str]) -> dict:
        gate = {"id": "t", "cmd": cmd, "scope": ["rust"], "required": True}
        return HARNESS.run_gate(gate, 60)

    def test_a_gate_that_skipped_is_not_recorded_as_pass(self):
        """① 打印了跳过标记 ⇒ 状态必须是 SKIPPED（**不是 PASS**）。"""
        cmd = _stub(self.dir, "skipper", 'print("GATE_SKIP: 语料目录不存在")\n')
        result = self._run(cmd)
        self.assertEqual(HARNESS.S_SKIPPED, result["status"], "跳过的门禁不得记成 PASS")
        self.assertEqual("env", result["skip_kind"])
        self.assertIn("不是通过", result["reason"] or "")

    def test_a_gate_that_ran_and_passed_is_pass(self):
        """② 退出 0 且没有跳过标记 ⇒ PASS。"""
        cmd = _stub(self.dir, "passer", 'print("all good")\n')
        result = self._run(cmd)
        self.assertEqual(HARNESS.S_PASS, result["status"])
        self.assertIsNone(result["skip_kind"])

    def test_a_gate_that_failed_is_fail(self):
        """③ 退出非 0 ⇒ FAIL。"""
        cmd = _stub(self.dir, "failing", "import sys\nsys.exit(3)\n")
        result = self._run(cmd)
        self.assertEqual(HARNESS.S_FAIL, result["status"])


class VerdictTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)

    def _skipped_gate(self) -> dict:
        cmd = _stub(self.dir, "skipper", 'print("GATE_SKIP: 语料目录不存在")\n')
        return HARNESS.run_gate(
            {"id": "t-skip", "cmd": cmd, "scope": ["rust"], "required": True}, 60
        )

    def test_environment_skip_does_not_block_but_is_visible(self):
        """④ 环境跳过 ⇒ 裁决仍 PASS，但**理由里必须出现「跳过」**。"""
        gate = self._skipped_gate()
        verdict, reasons = HARNESS.evaluate_verdict([gate], [], [])
        self.assertEqual(HARNESS.VERDICT_PASS, verdict, "环境缺前置条件不该阻断")
        self.assertTrue(
            any("跳过" in reason for reason in reasons),
            f"跳过必须在理由里出现（否则报告里只剩 ✅）：{reasons}",
        )

    def test_scope_skip_does_not_block(self):
        """⑤ 真·scope 跳过（本次 diff 不触发）⇒ 不阻断。"""
        gate = dict(self._skipped_gate())
        gate["scope_applies"] = False
        verdict, _ = HARNESS.evaluate_verdict([gate], [], [])
        self.assertEqual(HARNESS.VERDICT_PASS, verdict)

    def test_applicable_but_not_run_required_gate_blocks(self):
        """⑥ **适用却没跑**的 required 门禁 ⇒ 必须 BLOCKED（该查的没查）。"""
        gate = dict(self._skipped_gate())
        gate["skip_kind"] = None  # 不是环境跳过 ⇒ 就是「适用却没跑」
        gate["scope_applies"] = True
        verdict, reasons = HARNESS.evaluate_verdict([gate], [], [])
        self.assertEqual(HARNESS.VERDICT_BLOCKED, verdict, "适用却没跑必须阻断")
        self.assertTrue(any("skipped" in reason for reason in reasons), reasons)


if __name__ == "__main__":
    unittest.main(verbosity=2)
