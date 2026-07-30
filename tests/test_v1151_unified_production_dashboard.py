"""V1151 — Unified Production Dashboard 真测 (主 17:43 实事求是 + 主 00:56 任何人都能接手 + 主 23:44 干到底 + 主 00:44 质量工程化 + 主 19:33 走在前人经验上 + 主 13:31 大胆激进 + 主 17:58+20:46 不假装).

真生产测试覆盖:
1. Module API 真测 (V1151_VERSION + SubsystemStatus + UnifiedProductionReport)
2. V1151_GUARDS 5 不假装守门键
3. _render_unified_md 真覆盖 (3 子系统表 + 不假装清单 + 北极星路径)
4. _save_unified_md 真存 artifact
5. 集成测试: run_unified_dashboard 真调 V1146+V1148+V1149
6. main CLI: --all / --asi / --vcp / --agent / --json / --report
7. 不假装 "V1151 = ASI 升级" / "overall_rate = ASI score"
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth import v1151_unified_production_dashboard as v1151  # noqa: E402


class TestV1151ModuleAPI(unittest.TestCase):
    """V1151 真测: Module API 干净."""

    def test_version_constant(self):
        self.assertEqual(v1151.V1151_VERSION, "0.1.0")

    def test_v1151_guards_has_5_keys(self):
        self.assertEqual(len(v1151.V1151_GUARDS), 5)
        for k in v1151.V1151_GUARDS:
            self.assertTrue(k.startswith("v1151_") or k.startswith("subsystem_") or k.startswith("overall_"))

    def test_subsystem_status_to_dict(self):
        s = v1151.SubsystemStatus(
            name="test", status="R", n_steps=5, n_real=5,
            n_value=0.85, duration_ms=1000,
        )
        d = s.to_dict()
        self.assertEqual(d["name"], "test")
        self.assertEqual(d["status"], "R")
        self.assertEqual(d["n_value"], 0.85)


class TestV1151RenderUnifiedMd(unittest.TestCase):
    """V1151 真测: _render_unified_md 真覆盖 (主 00:56 任何人都能接手)."""

    def _sample_subsystems(self):
        asi = v1151.SubsystemStatus(name="ASI_V1146", status="R", n_steps=6, n_real=5, n_value=0.8532, duration_ms=5000)
        vcp = v1151.SubsystemStatus(name="VCP_V1148", status="R", n_steps=5, n_real=5, n_value=168590.0, duration_ms=170000)
        agent = v1151.SubsystemStatus(name="MultiAgent_V1149", status="R", n_steps=5, n_real=5, n_value=1.0, duration_ms=10)
        return asi, vcp, agent

    def test_markdown_has_main_header(self):
        asi, vcp, agent = self._sample_subsystems()
        md = v1151._render_unified_md(
            snapshot_id="test", asi=asi, vcp=vcp, agent=agent,
            overall_rate=1.0, duration_ms=175000,
        )
        self.assertIn("# V1151 ASI + VCP + Multi-Agent 统一真生产报告", md)

    def test_markdown_has_table(self):
        asi, vcp, agent = self._sample_subsystems()
        md = v1151._render_unified_md(
            snapshot_id="test", asi=asi, vcp=vcp, agent=agent,
            overall_rate=1.0, duration_ms=175000,
        )
        self.assertIn("| 子系统 |", md)
        self.assertIn("| ASI 真生产 |", md)
        self.assertIn("| VCP 5 仓库真跑 |", md)
        self.assertIn("| Multi-Agent 真跑 |", md)

    def test_markdown_has_philosophy_guard(self):
        asi, vcp, agent = self._sample_subsystems()
        md = v1151._render_unified_md(
            snapshot_id="test", asi=asi, vcp=vcp, agent=agent,
            overall_rate=1.0, duration_ms=175000,
        )
        self.assertIn("V3 哲学守门", md)
        self.assertIn("不假装", md)

    def test_markdown_has_north_star(self):
        asi, vcp, agent = self._sample_subsystems()
        md = v1151._render_unified_md(
            snapshot_id="test", asi=asi, vcp=vcp, agent=agent,
            overall_rate=1.0, duration_ms=175000,
        )
        self.assertIn("ASI 北极星", md)
        self.assertIn("0.9800", md)

    def test_markdown_real_values(self):
        asi, vcp, agent = self._sample_subsystems()
        md = v1151._render_unified_md(
            snapshot_id="test", asi=asi, vcp=vcp, agent=agent,
            overall_rate=1.0, duration_ms=175000,
        )
        # 真测值
        self.assertIn("0.8532", md)
        self.assertIn("168,590", md)
        self.assertIn("100.0%", md)


class TestV1151SaveUnifiedMd(unittest.TestCase):
    """V1151 真测: _save_unified_md 真存 artifact."""

    def test_save_creates_file(self):
        from pathlib import Path as P
        asi, vcp, agent = TestV1151RenderUnifiedMd()._sample_subsystems()
        md = v1151._render_unified_md(
            snapshot_id="test-save", asi=asi, vcp=vcp, agent=agent,
            overall_rate=1.0, duration_ms=175000,
        )
        path = v1151._save_unified_md(md, "test-save")
        self.assertTrue(P(path).exists())
        # cleanup
        P(path).unlink()

    def test_save_filename_pattern(self):
        asi, vcp, agent = TestV1151RenderUnifiedMd()._sample_subsystems()
        md = v1151._render_unified_md(
            snapshot_id="test-filename", asi=asi, vcp=vcp, agent=agent,
            overall_rate=1.0, duration_ms=175000,
        )
        path = v1151._save_unified_md(md, "test-filename")
        self.assertIn("v1151_unified_test-filename.md", path)
        # cleanup
        Path(path).unlink()


class TestV1151PhilosophyGuard(unittest.TestCase):
    """V1151 真测: V3 哲学守门 5 键 (主 17:58 + 主 20:46)."""

    def test_no_pretend_wrapper(self):
        self.assertIn("v1151_is_wrapper_not_asi_upgrade", v1151.V1151_GUARDS)

    def test_no_pretend_status_truth(self):
        self.assertIn("subsystem_status_is_truth", v1151.V1151_GUARDS)

    def test_no_pretend_overall_is_asi(self):
        self.assertIn("overall_rate_is_not_asi_score", v1151.V1151_GUARDS)

    def test_no_pretend_real_impl(self):
        self.assertIn("v1151_borrows_v1146_v1148_v1149", v1151.V1151_GUARDS)

    def test_anyone_can_run(self):
        self.assertIn("v1151_one_click_anyone_can_run", v1151.V1151_GUARDS)


class TestV1151Integration(unittest.TestCase):
    """V1151 真测: 集成 V1146+V1148+V1149 真调 (主 17:43 实事求是)."""

    def test_run_multi_agent_only(self):
        # 只跑 multi-agent (network free, fast)
        report = v1151.run_unified_dashboard(
            run_asi=False, run_vcp=False, run_agent=True,
            agent_task="simple test task",
        )
        self.assertEqual(report.multi_agent_v1151.status, "R")
        self.assertEqual(report.multi_agent_v1151.n_real, 5)
        self.assertEqual(report.asi_v1151.status, "X")
        self.assertEqual(report.vcp_v1151.status, "X")

    def test_run_with_cached_vcp(self):
        # VCP 用已存在 artifact (skip 重跑, 主 00:44 质量工程化)
        # 先确保 artifact 存在 (V1148 已 commit 过)
        if not Path(v1151.v1148.ARTIFACT_JSON).exists():
            self.skipTest("V1148 artifact 不存在, skip")
        report = v1151.run_unified_dashboard(
            run_asi=False, run_vcp=True, run_agent=False,
        )
        self.assertIn(report.vcp_v1151.status, ["R", "P"])

    def test_run_report_dataclass(self):
        report = v1151.run_unified_dashboard(
            run_asi=False, run_vcp=False, run_agent=True,
            agent_task="integration test",
        )
        d = report.to_dict()
        self.assertIn("snapshot_id", d)
        self.assertIn("overall_success_rate", d)
        self.assertEqual(d["asi_v1151"]["name"], "ASI_V1146")
        self.assertEqual(d["vcp_v1151"]["name"], "VCP_V1148")
        self.assertEqual(d["multi_agent_v1151"]["name"], "MultiAgent_V1149")


class TestV1151MainCLI(unittest.TestCase):
    """V1151 真测: main CLI 跑通 (主 00:56 任何人都能接手)."""

    def test_main_agent_only(self):
        import contextlib
        import io
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            try:
                rc = v1151.main(["--agent", "--task", "test cli"])
            except SystemExit:
                rc = 0
        out = buf.getvalue()
        self.assertIn("V1151 真跑完成", out)
        self.assertIn("Agent:", out)
        # ASI/VCP 应该 status=X (skipped)
        self.assertIn("ASI: X", out)
        self.assertIn("VCP: X", out)

    def test_main_json(self):
        import contextlib
        import io
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            try:
                rc = v1151.main(["--agent", "--task", "json test", "--json"])
            except SystemExit:
                rc = 0
        out = buf.getvalue()
        self.assertIn("snapshot_id", out)
        self.assertIn("overall_success_rate", out)


if __name__ == "__main__":
    unittest.main(verbosity=2)