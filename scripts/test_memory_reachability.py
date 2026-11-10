"""`check_memory_reachability` 的自测（防**门禁本身**被改坏）。

# 为什么守卫也要有自测
这个守卫是「**未解释缺口**」的唯一尺子 ⇒ ⇒ 它一旦**变松**（「空理由也放过」
「过期解释也放过」「缺口涨了也放过」）⇒ ⇒ **数字立刻开始骗人** ⇒ ⇒ 而
「52% / 46」这种数字一旦骗人，**行动清单就是假的** ⇒ ⇒ 比没有门禁更坏。

⇒ ⇒ 所以必须钉住它**真的有牙**。

# 钉住什么
1. **解释已过期**（已可达 / 已不存在）⇒ ⇒ 必报 ⇒ ⇒ 「已解释」不会变成永久豁免池；
2. **未解释缺口超上限** ⇒ ⇒ 必报（棘轮 · 只减不增）；
3. **已解释条目无理由** ⇒ ⇒ 必报（不许空口豁免）；
4. **理由未指向未接线台账** ⇒ ⇒ 必报（口径唯一：台账）；
5. **缺上限 / 重复登记 / 格式错** ⇒ ⇒ 必报；
6. **过期条目不抵扣缺口**（否则一条过期豁免就能把缺口算小 ⇒ ⇒ 数字骗人）；
7. **`--update` 只许把上限往下压**（不许自动抬 ⇒ ⇒ 棘轮只减不增）；
8. **本仓自身是干净的**（这条守卫的**现实对象**）。
"""

import importlib.util
import re
import sys
import tempfile
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location(
    "check_memory_reachability", Path(__file__).with_name("check_memory_reachability.py")
)
MODULE = importlib.util.module_from_spec(SPEC)
# ⚠️ 必须先登记进 `sys.modules` ⇒ ⇒ `@dataclass` 解析注解时要在那里找模块。
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)

LEDGER = "docs/status/unwired-ledger.md"


def explained_block(module: str, reason: str = f"台账 {LEDGER} §5.1 L1：已解释") -> str:
    return f"explained {module}  # {reason}\n"


class ParseExplainedTests(unittest.TestCase):
    """「已解释」基线的解析判据。"""

    def test_empty_reason_is_rejected(self):
        """③ 已解释条目**无理由** ⇒ ⇒ 红。"""
        reg = MODULE.parse_explained("explained foo\nmax_unexplained_gap = 3\n")
        self.assertTrue(reg.problems, "无理由的已解释条目必须被报")
        self.assertTrue(
            any("理由为空" in p for p in reg.problems),
            f"报错必须说清是「理由为空」，实得：{reg.problems}",
        )

    def test_reason_must_cite_the_ledger(self):
        """④ 理由**未指向台账** ⇒ ⇒ 红（口径唯一：台账）。"""
        reg = MODULE.parse_explained(
            "explained foo  # 我觉得它不重要\nmax_unexplained_gap = 3\n"
        )
        self.assertTrue(reg.problems, "不指向台账的理由必须被报")
        self.assertTrue(
            any("未接线台账" in p for p in reg.problems),
            f"报错必须说清是「未指向台账」，实得：{reg.problems}",
        )

    def test_missing_cap_is_rejected(self):
        """⑤ 缺 `max_unexplained_gap` ⇒ ⇒ 红（没有棘轮就等于没有门禁）。"""
        reg = MODULE.parse_explained(explained_block("foo"))
        self.assertTrue(reg.problems, "缺上限必须被报")
        self.assertTrue(
            any("max_unexplained_gap" in p for p in reg.problems),
            f"报错必须点出上限缺失，实得：{reg.problems}",
        )

    def test_duplicate_module_is_rejected(self):
        reg = MODULE.parse_explained(
            explained_block("foo") + explained_block("foo") + "max_unexplained_gap = 3\n"
        )
        self.assertTrue(reg.problems, "重复登记必须被报")
        self.assertTrue(
            any("重复登记" in p for p in reg.problems),
            f"报错必须说清是重复登记，实得：{reg.problems}",
        )

    def test_unparsable_line_is_rejected(self):
        reg = MODULE.parse_explained("garbage line\nmax_unexplained_gap = 3\n")
        self.assertTrue(reg.problems, "无法识别的行必须被报")

    def test_cap_is_parsed(self):
        reg = MODULE.parse_explained(explained_block("foo") + "max_unexplained_gap = 46\n")
        self.assertEqual(46, reg.cap)
        self.assertEqual([], reg.problems)
        self.assertEqual({"foo"}, reg.modules)


class EvaluateTests(unittest.TestCase):
    """闸门判据（纯函数）。"""

    def setUp(self):
        self.names = {"a", "b", "c", "d"}
        self.reachable = {"a"}

    def reg(self, entries: dict[str, str], cap: int) -> MODULE.ExplainedRegistry:
        text = "".join(
            f"explained {m}  # {r}\n" for m, r in entries.items()
        ) + f"max_unexplained_gap = {cap}\n"
        return MODULE.parse_explained(text)

    def test_stale_explanation_when_module_became_reachable(self):
        """① 已解释里有**已可达**模块 ⇒ ⇒ 报「解释已过期，请移除」⇒ ⇒ 红。"""
        # `a` 在 `reachable` 里 ⇒ ⇒ 它不该再留在「已解释」里。
        reg = self.reg({"a": f"台账 {LEDGER} §5.2 L90：有意不接"}, cap=5)
        problems = MODULE.evaluate(self.names, self.reachable, reg)
        self.assertTrue(problems, "已可达的已解释条目必须被报")
        self.assertTrue(
            any("解释已过期" in p and "a" in p for p in problems),
            f"报错必须是「解释已过期」且点名模块，实得：{problems}",
        )

    def test_stale_explanation_when_module_vanished(self):
        """①' 模块已不存在 ⇒ ⇒ 同样报过期（登记作废）。"""
        reg = self.reg({"zzz": f"台账 {LEDGER} §5.2 L90：有意不接"}, cap=5)
        problems = MODULE.evaluate(self.names, self.reachable, reg)
        self.assertTrue(problems, "已不存在的已解释条目必须被报")
        self.assertTrue(
            any("解释已过期" in p for p in problems),
            f"报错必须是「解释已过期」，实得：{problems}",
        )

    def test_gap_over_cap_is_reported(self):
        """② 未解释缺口超上限 ⇒ ⇒ 红。"""
        # 不可达 = {b, c, d} = 3 条；已解释 1 条 ⇒ 缺口 2；上限 1 ⇒ ⇒ 超。
        reg = self.reg({"b": f"台账 {LEDGER} §5.2 L90：有意不接"}, cap=1)
        problems = MODULE.evaluate(self.names, self.reachable, reg)
        self.assertTrue(problems, "缺口超上限必须被报")
        self.assertTrue(
            any("未解释缺口" in p for p in problems),
            f"报错必须点名「未解释缺口」，实得：{problems}",
        )

    def test_gap_within_cap_is_clean(self):
        reg = self.reg({"b": f"台账 {LEDGER} §5.2 L90：有意不接"}, cap=2)
        self.assertEqual([], MODULE.evaluate(self.names, self.reachable, reg))

    def test_stale_entry_does_not_credit_the_gap(self):
        """⑥ 过期条目（已可达）**不抵扣**缺口 ⇒ ⇒ 否则一条过期豁免就能把缺口算小。"""
        reg = self.reg({"a": f"台账 {LEDGER} §5.2 L90：有意不接"}, cap=99)
        # 不可达 = {b, c, d} = 3 条；`a` 已可达 ⇒ 不抵扣 ⇒ 缺口必须是 3。
        self.assertEqual(3, MODULE.unexplained_gap(self.names, self.reachable, reg))

    def test_gap_counts_only_unreachable_modules(self):
        reg = self.reg({"b": f"台账 {LEDGER} §5.2 L90：有意不接"}, cap=99)
        # 不可达 = {b, c, d} = 3 条，已解释且不可达 = {b} ⇒ 缺口 = 2。
        self.assertEqual(2, MODULE.unexplained_gap(self.names, self.reachable, reg))


class WriteCapTests(unittest.TestCase):
    """`--update` 只许把上限**往下压**（棘轮只减不增）。

    ⚠️ **2026-11-07 改成 hermetic（用临时夹具，不读仓库真值）**：
    真实 `max_unexplained_gap` 已经压到**棘轮地板 0**（未解释缺口清零后），
    旧版拿真文件算 `max(cap-1, 0)` 会变成「0 → 0」⇒ 走的是「拒绝抬上限」分支
    ⇒ 测试不再是「能压下去」的证据。改成夹具后既恢复原意，又补一条**地板行为**。
    """

    FIXTURE = (
        "explained fixture_a  # 台账 {ledger} §5.2 L1：有意不接\n"
        "max_unexplained_gap = {cap}\n"
    )

    def _use_fixture(self, cap: int) -> None:
        text = self.FIXTURE.format(ledger=LEDGER, cap=cap)
        handle = tempfile.NamedTemporaryFile(
            "w", suffix=".txt", delete=False, encoding="utf-8"
        )
        handle.write(text)
        handle.close()
        original = MODULE.EXPLAINED
        MODULE.EXPLAINED = Path(handle.name)
        self.addCleanup(setattr, MODULE, "EXPLAINED", original)
        self.addCleanup(Path(handle.name).unlink)

    def _cap(self) -> int:
        reg = MODULE.parse_explained(MODULE.EXPLAINED.read_text(encoding="utf-8"))
        assert reg.cap is not None
        return reg.cap

    def test_cap_is_never_raised(self):
        self._use_fixture(cap=5)
        reg = MODULE.parse_explained(MODULE.EXPLAINED.read_text(encoding="utf-8"))
        message = MODULE.write_cap(reg, 10)
        self.assertIn("拒绝自动抬上限", message)
        self.assertEqual(5, self._cap(), "文件不许被改")

    def test_cap_is_lowered_when_gap_shrinks(self):
        self._use_fixture(cap=5)
        reg = MODULE.parse_explained(MODULE.EXPLAINED.read_text(encoding="utf-8"))
        message = MODULE.write_cap(reg, 2)
        self.assertIn("上限已压到", message)
        self.assertEqual(2, self._cap())

    def test_cap_at_floor_zero_refuses_to_go_lower(self):
        """棘轮地板：上限已经是 0 ⇒ ⇒ 既不许抬、也不许变成负数/半截。"""
        self._use_fixture(cap=0)
        reg = MODULE.parse_explained(MODULE.EXPLAINED.read_text(encoding="utf-8"))
        message = MODULE.write_cap(reg, 0)
        self.assertIn("拒绝自动抬上限", message)
        self.assertEqual(0, self._cap())

    def test_human_written_reasons_are_preserved(self):
        """`--update` **不许**冲掉人工资产（理由是判断依据）。"""
        self._use_fixture(cap=5)
        reg = MODULE.parse_explained(MODULE.EXPLAINED.read_text(encoding="utf-8"))
        MODULE.write_cap(reg, 2)
        text = MODULE.EXPLAINED.read_text(encoding="utf-8")
        for entry in reg.entries.values():
            self.assertIn(entry.reason, text, "人工写的理由被冲掉了")


class RepositoryTests(unittest.TestCase):
    """这条守卫的**现实对象**：本仓自己。"""

    @classmethod
    def setUpClass(cls):
        cls.names, cls.reachable, _entries, _edges = MODULE.compute()
        cls.reg = MODULE.read_explained()

    def test_registry_parses_clean(self):
        self.assertEqual(
            [],
            self.reg.problems,
            f"「已解释」基线有解析问题：{self.reg.problems}",
        )

    def test_every_entry_cites_the_ledger(self):
        for module, entry in self.reg.entries.items():
            self.assertIn(
                MODULE.LEDGER_REF,
                entry.reason,
                f"`{module}` 的理由未指向未接线台账 ⇒ ⇒ 口径唯一",
            )

    def test_every_entry_is_a_real_unreachable_module(self):
        """每条登记都必须是「真实存在且当前不可达」的模块（否则就是过期豁免）。"""
        for module in self.reg.entries:
            self.assertIn(
                module,
                self.names,
                f"`{module}` 不在记忆 crate 的顶层模块里 ⇒ ⇒ 登记作废",
            )
            self.assertNotIn(
                module,
                self.reachable,
                f"`{module}` 已可达 ⇒ ⇒ 解释已过期 ⇒ ⇒ 请从基线移除",
            )

    def test_gate_is_green(self):
        """本仓跑本门禁 ⇒ ⇒ 必须绿（这正是验收口径）。"""
        problems = MODULE.evaluate(self.names, self.reachable, self.reg)
        self.assertEqual(
            [],
            problems,
            f"本仓未解释缺口 / 已解释登记有问题：{problems}\n"
            f"（若并行线刚新增了不可达模块 ⇒ ⇒ 这是**真信号** ⇒ ⇒ "
            f"请在 `docs/status/unwired-ledger.md` 补一条，或先接线）",
        )

    def test_reachability_ratchet_is_not_broken(self):
        base = MODULE.read_baseline()
        self.assertIsNotNone(base, "可达率基线缺失")
        self.assertGreaterEqual(
            len(self.reachable),
            base,
            f"可达模块数 {len(self.reachable)} < 基线 {base} ⇒ ⇒ 有能力掉出生产链路",
        )

    def test_explained_file_is_the_only_source(self):
        """「已解释」的判据来源**单一** ⇒ ⇒ 脚本里不许硬编码分类。"""
        text = Path(MODULE.__file__).read_text(encoding="utf-8")
        for forbidden in ("golden_", "corpus", "research_", "_store_sqlite"):
            self.assertNotIn(
                forbidden,
                text.split("# 「已解释」是什么")[-1],
                f"脚本里出现了硬编码的分类词 `{forbidden}` ⇒ ⇒ 口径必须唯一（台账）",
            )


if __name__ == "__main__":
    unittest.main()
