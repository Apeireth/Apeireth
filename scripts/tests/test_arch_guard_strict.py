#!/usr/bin/env python3
"""`scripts/arch_guard.py --strict` 退出码语义回归测试（stdlib unittest，0 外部依赖）。

判据（点对点升格 2026-11-04 之后的门禁语义）：

1. **strict 下有违规 ⇒ 退出码非 0**（CI 与 commit.ps1 才会真正拦截）；
2. **strict 下无违规 ⇒ 退出码 0**（干净基线不误伤）；
3. **report 模式下即使有违规也退出码 0**（显式降级路径：只报告、不阻断）；
4. **`--strict` 是被 CI workflow 依赖的确切参数名**：拼错必须被 argparse 拒掉
   （否则 workflow 里的参数名漂移会静默退化成 report-only）。

隔离口径（不污染工作树）：本测试**不改仓内任何文件**，也不调用 `cargo metadata`——
把 `arch_guard.ROOT` 指向临时目录，并 mock 掉 `workspace_metadata()`，
用一个 2~3 crate 的合成 workspace 驱动真实守卫代码路径。

因此本测试：
- 不依赖仓库当前状态（并行施工线的改动不会让它误红）；
- 不在工作树留下任何文件（临时目录在 tearDown 里清掉）；
- 仍然跑的是真守卫代码（`check()` + `main()` 的真实退出码判定），不是复刻逻辑。

跑法（仓库根目录）::

    python3 -m unittest discover -s scripts/tests -v
"""

from __future__ import annotations

import contextlib
import importlib.util
import io
import tempfile
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS_DIR = Path(__file__).resolve().parent.parent


def _load_arch_guard():
    """按路径加载 scripts/arch_guard.py（不把仓内模块名塞进 sys.path）。"""
    spec = importlib.util.spec_from_file_location(
        "arch_guard_under_test", SCRIPTS_DIR / "arch_guard.py"
    )
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


arch_guard = _load_arch_guard()


# 合成契约：2 个 crate、层方向 L0 -> L1 允许、无概念/扩展点/例外登记。
# 只覆盖守卫读到的键；未知段不影响判定。
CONTRACT_TOML = """\
[meta]
version = "0.0-test"
status = "test"
current_wave = 1

[layers]
L0 = ["crate_a"]
L1 = ["crate_b"]

[deps]
allowed_layer_edges = ["L1->*"]

[budgets]
max_workspace_crates = 10
max_production_edges = 10
max_file_loc = 100000

[env]
max_distinct_names = 100

[exceptions]
register_file = "exceptions.toml"
per_wave_weight_budget = 0
active_exceptions_max = 0
strict_requires_adr_above_weight = 10
"""


def _make_fake_workspace(root: Path, crate_names: list[str]) -> tuple[dict, dict, dict]:
    """造一个最小合成 workspace：(roots, prod_deps, all_deps)。

    crate 落在 root 之下 ⇒ 守卫里的 `path.relative_to(ROOT)` 可用（ROOT 被 patch 成 root）。
    每个 crate 只有一个极小的 lib.rs（无 APEIRETH_* 开关、无超大文件、无同名概念）。
    """
    roots: dict[str, Path] = {}
    for name in crate_names:
        crate_root = root / name
        src = crate_root / "src"
        src.mkdir(parents=True, exist_ok=True)
        (src / "lib.rs").write_text("pub fn placeholder() {}\n", encoding="utf-8")
        roots[name] = crate_root
    prod: dict[str, set[str]] = {name: set() for name in crate_names}
    all_deps: dict[str, set[str]] = {name: set() for name in crate_names}
    return roots, prod, all_deps


class ArchGuardStrictExitCodeTest(unittest.TestCase):
    """arch_guard 的 strict/report 退出码语义（真代码路径 + 合成 workspace）。"""

    def setUp(self) -> None:
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)

        # 契约 / 基线 / 例外登记 —— 全部落在临时目录内，仓内文件零改动
        (self.root / "exceptions.toml").write_text("exceptions = []\n", encoding="utf-8")
        self.contract = self.root / "contract.toml"
        self.contract.write_text(CONTRACT_TOML, encoding="utf-8")
        self.baseline = self.root / "baseline.txt"
        self.baseline.write_text("# 空基线（合成 workspace 无任何已登记指纹）\n", encoding="utf-8")
        self.log = self.root / "arch-guard.log"

    def _run(self, *extra_args: str, crates: tuple[str, ...] = ("crate_a", "crate_b")):
        """跑一次真实 main()，返回 (退出码, stdout)。"""
        roots, prod, all_deps = _make_fake_workspace(self.root / "ws", list(crates))
        out, err = io.StringIO(), io.StringIO()
        with mock.patch.object(arch_guard, "ROOT", self.root), mock.patch.object(
            arch_guard, "workspace_metadata", lambda: (roots, prod, all_deps)
        ), contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = arch_guard.main(
                [
                    "--contract",
                    str(self.contract),
                    "--baseline",
                    str(self.baseline),
                    "--log",
                    str(self.log),
                    *extra_args,
                ]
            )
        return code, out.getvalue()

    # ---------------------------------------------------------------- 判据

    def test_strict_clean_exits_zero(self) -> None:
        """② strict + 无违规 ⇒ exit 0（干净基线不误伤）。"""
        code, out = self._run("--strict")
        self.assertEqual(code, 0, out)
        self.assertIn("VIOLATIONS: 0", out)

    def test_strict_with_violation_exits_nonzero(self) -> None:
        """① strict + 有违规 ⇒ exit 非 0（这才是「能拦」的判据）。"""
        # crate_c 未登记在 [layers] ⇒ 结构性违规（权重 10），与仓库当前状态无关。
        code, out = self._run("--strict", crates=("crate_a", "crate_b", "crate_c"))
        self.assertNotEqual(code, 0, out)
        self.assertIn("VIOLATIONS: 1", out)
        self.assertIn("layer crate_c", out)

    def test_report_with_violation_exits_zero(self) -> None:
        """③ report + 有违规 ⇒ exit 0（显式降级路径只报告、不阻断）。"""
        code, out = self._run("--report", crates=("crate_a", "crate_b", "crate_c"))
        self.assertEqual(code, 0, out)
        self.assertIn("VIOLATIONS: 1", out)

    def test_report_clean_exits_zero(self) -> None:
        """③ report + 无违规 ⇒ exit 0。"""
        code, out = self._run("--report")
        self.assertEqual(code, 0, out)
        self.assertIn("VIOLATIONS: 0", out)

    def test_strict_flag_name_is_exact(self) -> None:
        """④ `--strict` 参数名必须精确：拼错要被 argparse 拒掉（exit 2）。

        CI workflow 与 commit.ps1 都按字面量传 `--strict`；参数名一旦漂移，
        这里必须先红，而不是让门禁静默退化成 report-only。
        """
        with self.assertRaises(SystemExit) as ctx:
            self._run("--strictt")
        self.assertEqual(ctx.exception.code, 2)


if __name__ == "__main__":
    unittest.main(verbosity=2)
