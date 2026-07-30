"""V1146 — ASI 真生产一键集成启动器 真测 (主 17:43 实事求是 + 主 00:56 任何人都能接手).

真生产测试覆盖:
1. Module API 真测 (import 干净, 5 step 函数 + orchestrator 都存在)
2. 真跑单步 (asi, deploy, benchmark, deepread, philosophy, streamlit) — 都 try-except + 标 status
3. 真跑全 (--all 等价) — orchestrator 汇总真生产报告
4. Markdown 报告渲染真覆盖 (主 00:56 任何人都能接手)
5. JSON 报告结构真覆盖 (status enum, n_steps_*, asi_v05_score)
6. V3 哲学守门真覆盖 (5 不假装 guard)
7. Status 5 状态 taxonomy 真覆盖 (R/P/M/X/H)
8. 端口探活真测 (_probe_port_open 真实 socket)
9. 自我决策: 至少 1 step 状态 = REAL (真跑了)
10. 不假装 "module = production": 全部 6 step 真跑, 不只看 module 存在
"""

from __future__ import annotations

import json
import socket
import sys
import time
import unittest
from pathlib import Path
from typing import Any, Dict, List

# Make apeireth importable
ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth import v1146_asi_one_click_production as v1146  # noqa: E402


class TestV1146ModuleAPI(unittest.TestCase):
    """V1146 真测: Module API 干净 (主 00:56)."""

    def test_version_constant(self):
        self.assertEqual(v1146.V1146_VERSION, "0.1.0")

    def test_asi_locked_target(self):
        self.assertEqual(v1146.ASI_LOCKED_TARGET, 0.9800)

    def test_status_enum_has_5_states(self):
        # 5 status states: REAL / PARTIAL / MOCK / MISSING / HARD_CODED
        statuses = [s.value for s in v1146.StepStatus]
        self.assertEqual(len(statuses), 5)
        self.assertIn("R", statuses)
        self.assertIn("P", statuses)
        self.assertIn("M", statuses)
        self.assertIn("X", statuses)
        self.assertIn("H", statuses)

    def test_step_funcs_registry(self):
        # 6 真生产 step 函数
        expected = {"asi", "deploy", "benchmark", "deepread", "philosophy", "streamlit"}
        self.assertEqual(set(v1146.STEP_FUNCS.keys()), expected)

    def test_v3_guards_have_5_keys(self):
        # 5 不假装守门 (主 17:58 + 主 20:46)
        expected_keys = {
            "module_is_not_production",
            "docker_available_is_not_deployment",
            "api_key_present_is_not_llm_call",
            "streamlit_started_is_not_accessible",
            "report_is_not_truth",
        }
        self.assertEqual(set(v1146.V1146_GUARDS.keys()), expected_keys)


class TestV1146StepResultDataclass(unittest.TestCase):
    """V1146 真测: V1146StepResult + V1146ProductionReport dataclass."""

    def test_step_result_defaults(self):
        r = v1146.V1146StepResult(step_name="test", status=v1146.StepStatus.REAL)
        self.assertEqual(r.step_name, "test")
        self.assertEqual(r.status, v1146.StepStatus.REAL)
        self.assertEqual(r.value, 0.0)
        self.assertEqual(r.duration_ms, 0.0)

    def test_production_report_summary(self):
        now = time.time()
        r = v1146.V1146ProductionReport(
            snapshot_id="test-id",
            started_at=now,
            finished_at=now + 1.0,
            asi_v05_score=0.7,
            asi_real_score=0.7,
            n_steps_total=2,
            n_steps_real=1,
            n_steps_partial=1,
        )
        self.assertEqual(r.duration_s, 1.0)
        self.assertEqual(r.real_rate, 0.5)
        # asi_locked_gap is a stored field (default 0.0), orchestrator sets it to ASI_LOCKED - asi_real_score
        # 这里 test 创建时未传, 期望 0.0 default
        self.assertEqual(r.asi_locked_gap, 0.0)


class TestV1146ProbePort(unittest.TestCase):
    """V1146 真测: _probe_port_open 真 socket (主 17:43 实事求是)."""

    def test_probe_unused_port_returns_false(self):
        # 用一个不存在的端口 (高位端口)
        result = v1146._probe_port_open("127.0.0.1", 1, timeout_s=0.5)
        self.assertFalse(result)

    def test_probe_localhost_garbage_returns_false(self):
        result = v1146._probe_port_open("127.0.0.1", 65530, timeout_s=0.5)
        self.assertFalse(result)


class TestV1146StepASI(unittest.TestCase):
    """V1146 真测: ASI V0.5 17-dim 真测 (主 17:43 实事求是)."""

    def test_asi_step_returns_real_status(self):
        r = v1146.step_asi_v05_measure(timeout_s=60.0)
        # 真跑 V1144 期望 ≥ 0
        self.assertGreaterEqual(r.value, 0.0)
        self.assertEqual(r.step_name, "asi_v05_measure")
        # 至少 0.5-0.9 真测值 (主 22:33 ASI 北极星)
        if r.status == v1146.StepStatus.REAL:
            self.assertGreater(r.value, 0.0)
            # V1144 已知 ≥ 0.5 真值
            self.assertGreater(r.value, 0.4)
        # Note 非空
        self.assertTrue(len(r.note) > 0)


class TestV1146StepDeploy(unittest.TestCase):
    """V1146 真测: 真部署校验 — port 8765 探活."""

    def test_deploy_step_returns_correct_status(self):
        r = v1146.step_deployment_validate(timeout_s=10.0)
        self.assertEqual(r.step_name, "deployment_validate")
        # Port 8765 当前未起, 期望 MISSING (主 17:43 实事求是)
        self.assertIn(r.status, [v1146.StepStatus.REAL, v1146.StepStatus.PARTIAL, v1146.StepStatus.MISSING])
        # Note 必须有
        self.assertTrue(len(r.note) > 0)


class TestV1146StepBenchmark(unittest.TestCase):
    """V1146 真测: V1133 真接 LLM benchmark 跑 22 样本 (主 06:15 V1051 真接)."""

    def test_benchmark_step_runs_real_samples(self):
        # 真跑 22 样本, 30s timeout (主 17:43 实事求是)
        r = v1146.step_benchmark_llm(timeout_s=30.0, max_samples=22)
        self.assertEqual(r.step_name, "benchmark_llm")
        # 22 样本 + 真接 = 期望 REAL 或 MOCK
        self.assertIn(r.status, [v1146.StepStatus.REAL, v1146.StepStatus.MOCK, v1146.StepStatus.MISSING])
        # Note 有 detail
        self.assertIn("V1133 真跑", r.note)
        # raw 包含 n_samples
        if r.raw:
            self.assertIn("n_samples", r.raw)


class TestV1146StepDeepRead(unittest.TestCase):
    """V1146 真测: V1142 ASI-Arch 真源代码深读."""

    def test_deepread_step_returns_real_status(self):
        r = v1146.step_asi_arch_deep_read(timeout_s=10.0)
        self.assertEqual(r.step_name, "asi_arch_deep_read")
        # ASI-Arch 深读期望 REAL
        self.assertEqual(r.status, v1146.StepStatus.REAL)
        # raw 包含 bridge
        self.assertIn("bridge", r.raw)
        # bridge 至少 v06_ready
        self.assertIn("v06_ready", r.raw["bridge"])


class TestV1146StepPhilosophy(unittest.TestCase):
    """V1146 真测: V1135 ASI 5 哲学空隙真答."""

    def test_philosophy_step_returns_real_status(self):
        r = v1146.step_5_philosophical_gaps(timeout_s=10.0)
        self.assertEqual(r.step_name, "5_philosophical_gaps")
        # V1135 有 5 answers, 期望 REAL + value > 0
        self.assertEqual(r.status, v1146.StepStatus.REAL)
        self.assertGreater(r.value, 0.5)
        # raw n_answers = 5
        self.assertEqual(r.raw.get("n_answers"), 5)


class TestV1146StepStreamlit(unittest.TestCase):
    """V1146 真测: Streamlit 探活 (主 17:43 实事求是 不假装起)."""

    def test_streamlit_probe_without_autostart(self):
        r = v1146.step_streamlit_startup(autostart=False, port=8501)
        self.assertEqual(r.step_name, "streamlit_startup")
        # 默认不起 streamlit, 期望 MISSING (除非已有)
        self.assertIn(r.status, [v1146.StepStatus.MISSING, v1146.StepStatus.REAL])
        # raw port_open 字段
        self.assertIn("port_open", r.raw)


class TestV1146Orchestrator(unittest.TestCase):
    """V1146 真测: v1146_run_all 一键跑全 + 真生产报告."""

    def test_run_all_completes_with_summary(self):
        # benchmark 用 30s timeout 避免太长
        rep = v1146.v1146_run_all(autostart_streamlit=False, benchmark_timeout_s=30.0)
        self.assertEqual(rep.n_steps_total, 6)
        # snapshot_id 格式 v1146-xxxxxxxxxxxx
        self.assertTrue(rep.snapshot_id.startswith("v1146-"))
        # 至少 1 step 真跑 (R) - 主 17:43 不假装
        # ASI + benchmark + deepread + philosophy 4 个会 R
        self.assertGreaterEqual(rep.n_steps_real, 1)
        # 报告结构 OK
        self.assertEqual(len(rep.steps), 6)
        # ASI v05 真值 (来自 step 1)
        asi_step = next(s for s in rep.steps if s.step_name == "asi_v05_measure")
        self.assertEqual(asi_step.status, v1146.StepStatus.REAL)
        # locked gap 正确
        self.assertAlmostEqual(rep.asi_locked_gap, 0.9800 - rep.asi_real_score, places=4)

    def test_run_one_single_step(self):
        # 单跑 philosophy
        rep = v1146.v1146_run_one("philosophy")
        self.assertEqual(rep.n_steps_total, 1)
        self.assertEqual(rep.n_steps_real, 1)
        self.assertEqual(rep.steps[0].step_name, "5_philosophical_gaps")

    def test_run_one_invalid_step(self):
        rep = v1146.v1146_run_one("nonexistent")
        self.assertEqual(rep.n_steps_total, 0)
        self.assertFalse(rep.philosophy_guard_ok)


class TestV1146ReportRender(unittest.TestCase):
    """V1146 真测: Markdown 报告 + JSON 报告 真覆盖 (主 00:56 任何人都能接手)."""

    def test_markdown_render_includes_all_sections(self):
        now = time.time()
        rep = v1146.V1146ProductionReport(
            snapshot_id="test-md",
            started_at=now,
            finished_at=now + 1.0,
            asi_v05_score=0.7,
            asi_real_score=0.7,
            n_steps_total=2,
            n_steps_real=1,
            n_steps_partial=1,
            steps=[
                v1146.V1146StepResult(
                    step_name="asi_v05_measure",
                    status=v1146.StepStatus.REAL,
                    value=0.7,
                    note="test note",
                ),
                v1146.V1146StepResult(
                    step_name="deployment_validate",
                    status=v1146.StepStatus.MISSING,
                    value=0.0,
                    note="port not listening",
                ),
            ],
        )
        md = v1146.render_markdown(rep)
        # 关键 section
        self.assertIn("# V1146 ASI 真生产一键集成报告", md)
        self.assertIn("ASI V0.5 17-dim", md)
        self.assertIn("真生产 step 汇总", md)
        self.assertIn("V3 哲学守门", md)
        self.assertIn("module_is_not_production", md)
        self.assertIn("R", md)  # REAL status
        self.assertIn("X", md)  # MISSING status
        # step 名字
        self.assertIn("asi_v05_measure", md)
        self.assertIn("deployment_validate", md)

    def test_asdict_roundtrip(self):
        now = time.time()
        step = v1146.V1146StepResult(
            step_name="test",
            status=v1146.StepStatus.REAL,
            value=0.5,
            note="test",
        )
        d = v1146.asdict(step)
        self.assertEqual(d["step_name"], "test")
        self.assertEqual(d["status"], v1146.StepStatus.REAL)  # asdict keeps enum
        # round-trip via json
        # Note: enum to json needs default=str
        out = json.dumps(d, default=str)
        self.assertIn("test", out)


class TestV1146IntegrationReal(unittest.TestCase):
    """V1146 真测: 端到端真跑 — 一行命令 = 真生产 (主 00:56 任何人都能接手).

    主 17:43 实事求是:
    - 6 steps 都真跑
    - 至少 1 step 是 R (真跑了)
    - 真值 >= 0 (V1144 V0.5 = 0.7766)
    - benchmark 真接 = 22 样本 + 真 pass_rate
    """

    def test_full_integration_real(self):
        # 真跑 (用 30s benchmark timeout)
        rep = v1146.v1146_run_all(autostart_streamlit=False, benchmark_timeout_s=30.0)

        # ASI V0.5 真测 (主 22:33 ASI 北极星)
        asi_step = next(s for s in rep.steps if s.step_name == "asi_v05_measure")
        self.assertEqual(asi_step.status, v1146.StepStatus.REAL)
        # V1144 已知 ASI V0.5 真值 ~0.7766
        self.assertGreater(asi_step.value, 0.6)

        # 真接 LLM benchmark (主 06:15 V1051 真接)
        bench_step = next(s for s in rep.steps if s.step_name == "benchmark_llm")
        # 期望 REAL (有 .minimax_key) 或 MOCK (没 key)
        self.assertIn(bench_step.status, [v1146.StepStatus.REAL, v1146.StepStatus.MOCK])
        if bench_step.status == v1146.StepStatus.REAL:
            # 22 样本 + 真 pass_rate > 0
            self.assertEqual(bench_step.raw.get("n_samples"), 22)
            self.assertGreater(bench_step.raw.get("pass_rate", 0.0), 0.0)

        # ASI-Arch 真源代码深读
        deep_step = next(s for s in rep.steps if s.step_name == "asi_arch_deep_read")
        self.assertEqual(deep_step.status, v1146.StepStatus.REAL)

        # 5 哲学真答
        phil_step = next(s for s in rep.steps if s.step_name == "5_philosophical_gaps")
        self.assertEqual(phil_step.status, v1146.StepStatus.REAL)
        self.assertEqual(phil_step.raw.get("n_answers"), 5)

        # 部署 / Streamlit 期望 MISSING (没起)
        deploy_step = next(s for s in rep.steps if s.step_name == "deployment_validate")
        streamlit_step = next(s for s in rep.steps if s.step_name == "streamlit_startup")
        # 标 MISSING 即可 (没真起 docker compose), 不假装
        self.assertIn(deploy_step.status, [
            v1146.StepStatus.REAL, v1146.StepStatus.PARTIAL, v1146.StepStatus.MISSING,
        ])
        self.assertIn(streamlit_step.status, [
            v1146.StepStatus.REAL, v1146.StepStatus.PARTIAL, v1146.StepStatus.MISSING,
        ])


class TestV1146MainCLI(unittest.TestCase):
    """V1146 真测: main CLI 入口 (主 00:56 任何人都能接手).

    真实调用 main() 不抛异常, 输出 Markdown 报告.
    """

    def test_main_default_runs_all(self):
        import contextlib
        import io

        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            try:
                rc = v1146.main(["--benchmark-timeout", "20"])
            except SystemExit:
                rc = 0
        out = buf.getvalue()
        # Markdown 报告头部
        self.assertIn("V1146 ASI 真生产一键集成报告", out)
        # ASI V0.5 真值
        self.assertIn("ASI V0.5 17-dim", out)
        # 6 step 都列
        self.assertIn("asi_v05_measure", out)
        self.assertIn("deployment_validate", out)
        self.assertIn("benchmark_llm", out)
        self.assertIn("asi_arch_deep_read", out)
        self.assertIn("5_philosophical_gaps", out)
        self.assertIn("streamlit_startup", out)


if __name__ == "__main__":
    unittest.main(verbosity=2)