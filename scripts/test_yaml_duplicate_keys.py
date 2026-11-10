"""`check_yaml_duplicate_keys` 的自测（防门禁本身被改坏）。

# 为什么守卫也要有自测
这个守卫是**唯一**能拦住「YAML 重复键 ⇒ GitHub 拒解析整个 workflow ⇒ 0 job 秒红」的东西
（该病**已复发三轮**）⇒ ⇒ **守卫自己坏掉**（比如有人为了「让它安静」把 loader 改回宽松的）
⇒ ⇒ 就等于**把病放回去** ⇒ ⇒ 必须有测试钉住它**真的能抓到**。

# 钉住三件事
1. **能抓到**：同层重复键 ⇒ 报错（并带行/列/键名）；
2. **不误报**：不同层的同名键（合法的嵌套）⇒ 不报错；
3. **语法错误也报**：GitHub 同样拒解析 ⇒ 门禁必须一并拦下。
"""

import importlib.util
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location(
    "check_yaml_duplicate_keys", Path(__file__).with_name("check_yaml_duplicate_keys.py")
)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class YamlDuplicateKeyTests(unittest.TestCase):
    def write(self, text: str) -> Path:
        directory = tempfile.mkdtemp(prefix="yaml-dup-key-")
        path = Path(directory) / "wf.yml"
        path.write_text(text, encoding="utf-8")
        return path

    def test_same_level_duplicate_key_is_caught(self):
        path = self.write(
            "name: demo\n"
            "jobs:\n"
            "  a:\n"
            "    steps:\n"
            "      - name: one\n"
            "        if: always()\n"
            "        run: echo hi\n"
            "        if: always()\n"
        )
        problems = MODULE.scan(path)
        self.assertTrue(problems, "同层重复键必须被抓到")
        self.assertIn("'if'", problems[0])

    def test_nested_same_name_is_not_a_duplicate(self):
        # 同名键在不同层 = 合法嵌套 ⇒ **不得**误报。
        path = self.write(
            "name: demo\n"
            "on:\n"
            "  push:\n"
            "    branches:\n"
            "      - main\n"
            "jobs:\n"
            "  a:\n"
            "    if: always()\n"
            "    steps:\n"
            "      - run: echo hi\n"
        )
        self.assertEqual([], MODULE.scan(path))

    def test_syntax_error_is_reported(self):
        # 未闭合的流（`[` 少 `]`）⇒ PyYAML 报 scanner/parser error ⇒ GitHub 同样拒解析。
        path = self.write("name: demo\njobs:\n  a: [1, 2\n")
        problems = MODULE.scan(path)
        self.assertTrue(problems, "语法错误必须被报（GitHub 同样拒解析）")
        self.assertIn("语法错误", problems[0])

    def test_clean_file_has_no_problems(self):
        path = self.write(
            "name: demo\n"
            "on:\n"
            "  push:\n"
            "    branches: [main]\n"
            "jobs:\n"
            "  a:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - uses: actions/checkout@v4\n"
        )
        self.assertEqual([], MODULE.scan(path))

    def test_repository_workflows_are_clean(self):
        """仓库自己的 workflow 必须是干净的（这条守卫的**现实对象**）。"""
        files = sorted(MODULE.WORKFLOW_DIR.glob("*.yml")) + sorted(
            MODULE.WORKFLOW_DIR.glob("*.yaml")
        )
        self.assertTrue(files, "应当能扫到 workflow 文件")
        problems: list[str] = []
        for path in files:
            problems.extend(MODULE.scan(path))
        self.assertEqual([], problems, f"仓库 workflow 有重复键/语法错误：{problems}")


if __name__ == "__main__":
    unittest.main()
