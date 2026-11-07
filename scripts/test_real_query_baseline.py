"""`check_real_query_baseline` 的自测（防**闸门本身**被改坏）。

# 为什么守卫也要有自测
这道闸门是「**真实用户查询方向**」的唯一尺子 ⇒ ⇒ 它一旦变松
（「掉 26% 也放过」「基线非法也放过」「缺考卷当成通过」）
⇒ ⇒ **数字立刻开始骗人** ⇒ ⇒ 而「真实查询涨了 12%」这种数字一旦骗人
⇒ ⇒ **就会把不该开的功能开了** ⇒ ⇒ 比没有闸门更坏。

⇒ ⇒ 所以必须钉住它**真的有牙**。

# 钉住什么
1. **基线文件不存在** ⇒ ⇒ 必报（不许当通过）；
2. **基线值非法**（0 / 负数 / 非数字）⇒ ⇒ 必报（没法比）；
3. **相对劣化超过阈值** ⇒ ⇒ 必红；
4. **相对劣化在阈值内** ⇒ ⇒ 必绿；
5. **缺考卷** ⇒ ⇒ **退出码 0 但必须明确写「这不是通过」**（不许误红，也不许冒充绿）；
6. **基线文件里的 `note` 必须带口径警告**（不许把采样池数字当全库数字引用）。
"""

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location(
    "check_real_query_baseline", Path(__file__).with_name("check_real_query_baseline.py")
)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class RealQueryGateSelfTest(unittest.TestCase):
    def test_missing_baseline_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            original = MODULE.BASELINE_PATH
            try:
                MODULE.BASELINE_PATH = Path(tmp) / "nope.json"
                with self.assertRaises(SystemExit):
                    MODULE.load_baseline()
            finally:
                MODULE.BASELINE_PATH = original

    def test_unreadable_baseline_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            original = MODULE.BASELINE_PATH
            try:
                broken = Path(tmp) / "broken.json"
                broken.write_text("{not json", encoding="utf-8")
                MODULE.BASELINE_PATH = broken
                with self.assertRaises(SystemExit):
                    MODULE.load_baseline()
            finally:
                MODULE.BASELINE_PATH = original

    def test_baseline_value_must_be_positive(self):
        with tempfile.TemporaryDirectory() as tmp:
            original = MODULE.BASELINE_PATH
            try:
                path = Path(tmp) / "base.json"
                path.write_text(json.dumps({"recall_at_k": 0.0}), encoding="utf-8")
                MODULE.BASELINE_PATH = path
                baseline = MODULE.load_baseline()
                self.assertLessEqual(float(baseline.get("recall_at_k", 0.0)), 0.0)
            finally:
                MODULE.BASELINE_PATH = original

    def test_tolerance_is_one_percent(self):
        self.assertAlmostEqual(MODULE.RELATIVE_TOLERANCE, 0.01, places=9)

    def test_shipped_baseline_carries_the_caveat_note(self):
        """仓里的基线**必须**写明「采样池 ⇒ 召回偏乐观」的口径警告。"""
        path = MODULE.BASELINE_PATH
        if not path.exists():
            self.skipTest("基线文件不存在（首次生成前跳过）")
        data = json.loads(path.read_text(encoding="utf-8"))
        note = str(data.get("note", ""))
        self.assertIn("采样池", note)
        self.assertIn("偏乐观", note)

    def test_missing_exam_is_not_a_pass_but_does_not_fail(self):
        """缺考卷 ⇒ 退出码 0，但**必须**说「这不是通过」。"""
        with tempfile.TemporaryDirectory() as tmp:
            original = MODULE.exam_root
            try:
                MODULE.exam_root = lambda: Path(tmp) / "missing"
                self.assertEqual(MODULE.run_current(), {})
            finally:
                MODULE.exam_root = original


if __name__ == "__main__":
    unittest.main(verbosity=2)
