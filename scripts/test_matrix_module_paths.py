"""`check_matrix_module_paths` 的自测（防门禁本身被改坏 ⇒ 把病放回去）。

# 为什么守卫也要有自测
这个守卫是**唯一**能拦住「能力矩阵宣称了**不存在的模块**」的东西
（2026-11-03 当场抓到 2 处：`apeireth-orchestration::prompt_stabilizer`、
`apeireth-perception::voice::minimax_tts` —— **两个都已归档**）
⇒ ⇒ 门禁自己坏掉 ⇒ 等于**把病放回去** ⇒ 必须钉住它**真的能抓到**。

# 钉住三件事
1. **仓里的矩阵当前是干净的**（现实对象）；
2. **塞一个假模块 ⇒ 必须红**；
3. **写在删除线里 ⇒ 放行**（已如实标注的旧项不该反复报错）。
"""

import importlib.util
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location(
    "check_matrix_module_paths", Path(__file__).with_name("check_matrix_module_paths.py")
)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class MatrixModulePathTests(unittest.TestCase):
    def test_repository_matrix_is_clean(self):
        """仓里的矩阵必须干净（这条守卫的**现实对象**）。"""
        self.assertEqual([], MODULE.problems(MODULE.MATRIX.read_text(encoding="utf-8")))

    def test_a_fake_module_reference_is_caught(self):
        line = (
            "| **编排 (Orchestration)** | `orch.x` | `apeireth-orchestration::ghost_module` |"
            " `f()` | 说明 |\n"
        )
        problems = MODULE.problems(line)
        self.assertTrue(problems, "假模块引用必须被抓到")
        self.assertIn("ghost_module", problems[0])

    def test_a_deleted_module_reference_is_allowed(self):
        """删除线 = 已如实标注 ⇒ 放行（否则修好的旧项会反复报错）。"""
        line = (
            "| **编排 (Orchestration)** | ~~`orch.x`~~ | **⚠️ 已归档**"
            " ~~`apeireth-orchestration::ghost_module`~~ | ~~`f()`~~ | 说明 |\n"
        )
        self.assertEqual([], MODULE.problems(line))

    def test_a_real_module_reference_passes(self):
        line = (
            "| **编排 (Orchestration)** | `orch.x` | `apeireth-orchestration::council` |"
            " `f()` | 说明 |\n"
        )
        self.assertEqual([], MODULE.problems(line))


if __name__ == "__main__":
    unittest.main()
