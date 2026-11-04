#!/usr/bin/env python3
"""`scripts/verify_changed.py` 回归测试（stdlib unittest，0 外部依赖）。

覆盖 fast gate 的三条影响面口径：

1. **seed 为空**（改动全落在根 workspace 之外）⇒ 受影响集合为空集，且不含任何
   非 member 包名——不得退化成「全 workspace + registry 第三方包」。
2. **显式指定一个 member crate 的文件** ⇒ 受影响集合只含根 workspace member。
3. **`all_workspace`**（根 Cargo.toml / Cargo.lock 改动）⇒ 受影响集合 == 根
   workspace 全部 member（当前 31 个，口径见 AGENTS.md §3）。

附带覆盖：反向依赖闭包途中遇到的非 member 包必须被剔除；改动在根 workspace 外时
`main()` 须明确说明并跳过 cargo 阶段、退出码 0。

跑法（仓库根目录）::

    python3 -m unittest discover -s scripts/tests -v
"""

from __future__ import annotations

import io
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parent.parent
ROOT = SCRIPTS_DIR.parent
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

import verify_changed  # noqa: E402  （需先把 scripts/ 放进 sys.path）

# 根 workspace member 数口径（AGENTS.md §3：31 members = 30 crates + bench harness）。
# workspace 成员增删时，本常量与 AGENTS.md §3 一并更新。
EXPECTED_MEMBER_COUNT = 31


def _fake_metadata() -> dict:
    """构造一份最小 metadata：2 个 member + 1 个非 member 包。

    图：member-b → member-a → 非 member 包；非 member 包 → member-a
    （即「非 member 包反向依赖 member」——闭包会走到它，但结果必须剔除它）。
    """
    return {
        "packages": [
            {
                "id": "member-a",
                "name": "apeireth-fake-a",
                "manifest_path": str(ROOT / "crates" / "fake" / "a" / "Cargo.toml"),
            },
            {
                "id": "member-b",
                "name": "apeireth-fake-b",
                "manifest_path": str(ROOT / "crates" / "fake" / "b" / "Cargo.toml"),
            },
            {
                "id": "non-member",
                "name": "third-party-fake",
                "manifest_path": "/registry/third-party-fake/Cargo.toml",
            },
        ],
        "workspace_members": ["member-a", "member-b"],
        "resolve": {
            "nodes": [
                {"id": "member-a", "dependencies": ["non-member"]},
                {"id": "member-b", "dependencies": ["member-a", "non-member"]},
                {"id": "non-member", "dependencies": ["member-a"]},
            ]
        },
    }


class FakeMetadataTest(unittest.TestCase):
    """不依赖 cargo 的纯逻辑测试（确定性、快）。"""

    def setUp(self) -> None:
        self.meta = _fake_metadata()
        self.member_ids = list(self.meta["workspace_members"])
        self.member_names = {"apeireth-fake-a", "apeireth-fake-b"}

    def test_empty_seed_yields_empty_set(self) -> None:
        """① seed 为空 ⇒ 空集（不退化为全量）。"""
        result = verify_changed.affected_packages([], self.meta, False, self.member_ids)
        self.assertEqual(result, [])
        self.assertNotIn("third-party-fake", result)

    def test_all_workspace_returns_members_only(self) -> None:
        """③ all_workspace ⇒ 全部 member，不含非 member 包。"""
        result = verify_changed.affected_packages([], self.meta, True, self.member_ids)
        self.assertEqual(result, sorted(self.member_names))
        self.assertNotIn("third-party-fake", result)

    def test_reverse_closure_filters_non_members(self) -> None:
        """④ 反向依赖闭包走到非 member 包时必须剔除。"""
        result = verify_changed.affected_packages(["member-a"], self.meta, False, self.member_ids)
        self.assertEqual(result, sorted(self.member_names))
        self.assertNotIn("third-party-fake", result)

    def test_member_names_helper(self) -> None:
        self.assertEqual(verify_changed.member_names(self.meta, self.member_ids), self.member_names)


class RootWorkspaceTest(unittest.TestCase):
    """对着本仓真实根 workspace 的测试（需要 cargo metadata）。"""

    @classmethod
    def setUpClass(cls) -> None:
        try:
            cls.meta = verify_changed.load_metadata()
        except SystemExit as exc:  # pragma: no cover - 环境无 cargo 时跳过
            raise unittest.SkipTest(f"cargo metadata 不可用：{exc}") from exc
        cls.by_manifest, cls.member_ids = verify_changed.build_index(cls.meta)
        cls.member_names = verify_changed.member_names(cls.meta, cls.member_ids)
        cls.non_member_names = {p["name"] for p in cls.meta["packages"]} - cls.member_names

    def test_member_count_matches_agreed_caliber(self) -> None:
        """③ 根 workspace member 数 == 口径值（当前 31）。"""
        self.assertEqual(
            len(self.member_names),
            EXPECTED_MEMBER_COUNT,
            "根 workspace member 数与 AGENTS.md §3 口径不一致；"
            "成员增删时须一并更新 AGENTS.md §3 与本文件的 EXPECTED_MEMBER_COUNT。",
        )

    def test_all_workspace_returns_all_members(self) -> None:
        """③ all_workspace ⇒ 返回全部 member 名。"""
        result = verify_changed.affected_packages([], self.meta, True, self.member_ids)
        self.assertEqual(result, sorted(self.member_names))
        self.assertEqual(len(result), len(self.meta["workspace_members"]))
        self.assertFalse(set(result) & self.non_member_names)

    def test_empty_seed_yields_empty_set(self) -> None:
        """① seed 为空 ⇒ 空集，且不含任何非 member 包名。"""
        result = verify_changed.affected_packages([], self.meta, False, self.member_ids)
        self.assertEqual(result, [])
        self.assertFalse(set(result) & self.non_member_names)

    def test_explicit_member_file_yields_members_only(self) -> None:
        """② 显式指定一个 member crate 的文件 ⇒ 只含 workspace member。"""
        path = "crates/capabilities/tools/src/filesystem.rs"
        pkg = verify_changed.crate_for_path(path, self.by_manifest)
        self.assertIsNotNone(pkg, f"{path} 应能映射到根 workspace member")
        result = verify_changed.affected_packages([pkg["id"]], self.meta, False, self.member_ids)
        self.assertIn("apeireth-tools-canonical", result)
        self.assertTrue(set(result) <= self.member_names)
        self.assertFalse(set(result) & self.non_member_names)

    def test_files_outside_root_workspace_yield_empty_set(self) -> None:
        """独立 workspace 目录下的改动 ⇒ 映射不到 member ⇒ 空集。"""
        path = "crates-ui/apeireth-ui-demo/src/lib.rs"
        pkg = verify_changed.crate_for_path(path, self.by_manifest)
        self.assertIsNone(pkg, "独立 workspace 的 crate 不应被当作根 workspace member")
        result = verify_changed.affected_packages([], self.meta, False, self.member_ids)
        self.assertEqual(result, [])

    def test_main_reports_outside_workspace_and_skips_cargo(self) -> None:
        """改动在根 workspace 外 ⇒ 明确说明 + 跳过 cargo 阶段 + 退出码 0。"""
        buf = io.StringIO()
        with tempfile.TemporaryDirectory() as tmp:
            log = Path(tmp) / "verify-changed.log"
            with redirect_stdout(buf):
                code = verify_changed.main(
                    [
                        "--dry-run",
                        "--no-arch",
                        "--log",
                        str(log),
                        "--files",
                        "crates-ui/apeireth-ui-demo/src/lib.rs",
                    ]
                )
        out = buf.getvalue()
        self.assertEqual(code, 0)
        self.assertIn("no affected workspace crate", out)
        self.assertIn("changes are outside the root workspace", out)
        self.assertIn("affected crates: 0", out)
        # 非 member 包名一个都不许出现
        for name in self.non_member_names:
            self.assertNotIn(f"  - {name}\n", out)


if __name__ == "__main__":
    unittest.main(verbosity=2)
