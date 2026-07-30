"""V1149 — Multi-Agent Role + DAG 真测 (主 17:43 实事求是 + 主 00:56 任何人都能接手 + 主 23:44 干到底 + 主 00:44 质量工程化 + 主 19:33 走在前人经验上 + 主 13:31 大胆激进 + 主 17:58+20:46 不假装).

真生产测试覆盖:
1. AgentRole 5 真角色真测
2. AgentTask 真构造 + to_dict
3. AgentDAG 真 topological sort + cycle detection
4. _plan_task_for_role 真按 role 拆
5. _execute_task 真 mock execute + duration_ms
6. _build_default_dag 真 5 节点 + 4 边
7. run_multi_agent 真入口 + topo order 真跑
8. V1149_GUARDS 5 守门键
9. main CLI 跑通
10. 不假装 "multi-agent = ASI" / "mock executor = real LLM"
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth import v1149_multi_agent_role_dag as v1149  # noqa: E402


class TestV1149ModuleAPI(unittest.TestCase):
    """V1149 真测: Module API 干净."""

    def test_version_constant(self):
        self.assertEqual(v1149.V1149_VERSION, "0.1.0")

    def test_agent_role_has_5(self):
        roles = v1149.AgentRole.all_roles()
        self.assertEqual(len(roles), 5)

    def test_agent_role_values(self):
        roles = [r.value for r in v1149.AgentRole.all_roles()]
        for expected in ["planner", "executor", "critic", "refiner", "synthesizer"]:
            self.assertIn(expected, roles)

    def test_v1149_guards_has_5_keys(self):
        self.assertEqual(len(v1149.V1149_GUARDS), 5)
        for k in v1149.V1149_GUARDS:
            self.assertTrue(k.startswith(("agent_role_", "dag_", "multi_agent_", "mock_", "v1149_")))


class TestV1149AgentTask(unittest.TestCase):
    """V1149 真测: AgentTask dataclass."""

    def test_agent_task_to_dict(self):
        t = v1149.AgentTask(id="t1", role=v1149.AgentRole.PLANNER, input="test")
        d = t.to_dict()
        self.assertEqual(d["id"], "t1")
        self.assertEqual(d["role"], "planner")
        self.assertEqual(d["input"], "test")
        self.assertEqual(d["status"], "pending")
        self.assertEqual(d["deps"], [])

    def test_agent_task_default_status(self):
        t = v1149.AgentTask(id="t1", role=v1149.AgentRole.EXECUTOR, input="x")
        self.assertEqual(t.status, "pending")
        self.assertEqual(t.output, "")
        self.assertEqual(t.error, "")
        self.assertEqual(t.duration_ms, 0.0)


class TestV1149DAG(unittest.TestCase):
    """V1149 真测: AgentDAG 真 topological sort + cycle detection."""

    def test_dag_add_task(self):
        dag = v1149.AgentDAG()
        t = v1149.AgentTask(id="t1", role=v1149.AgentRole.PLANNER, input="x")
        dag.add_task(t)
        self.assertIn("t1", dag.tasks)

    def test_dag_duplicate_id_raises(self):
        dag = v1149.AgentDAG()
        t = v1149.AgentTask(id="t1", role=v1149.AgentRole.PLANNER, input="x")
        dag.add_task(t)
        with self.assertRaises(ValueError):
            dag.add_task(v1149.AgentTask(id="t1", role=v1149.AgentRole.EXECUTOR, input="y"))

    def test_dag_missing_dep_raises(self):
        dag = v1149.AgentDAG()
        t = v1149.AgentTask(id="t1", role=v1149.AgentRole.EXECUTOR, input="x", deps=["missing"])
        with self.assertRaises(ValueError):
            dag.add_task(t)

    def test_dag_topo_sort_linear(self):
        dag = v1149.AgentDAG()
        a = v1149.AgentTask(id="a", role=v1149.AgentRole.PLANNER, input="x")
        b = v1149.AgentTask(id="b", role=v1149.AgentRole.EXECUTOR, input="y", deps=["a"])
        c = v1149.AgentTask(id="c", role=v1149.AgentRole.SYNTHESIZER, input="z", deps=["b"])
        for t in [a, b, c]:
            dag.add_task(t)
        order = dag.topo_sort()
        self.assertEqual(order, ["a", "b", "c"])

    def test_dag_topo_sort_diamond(self):
        #   a
        #  / \
        # b   c
        #  \ /
        #   d
        dag = v1149.AgentDAG()
        a = v1149.AgentTask(id="a", role=v1149.AgentRole.PLANNER, input="x")
        b = v1149.AgentTask(id="b", role=v1149.AgentRole.EXECUTOR, input="y", deps=["a"])
        c = v1149.AgentTask(id="c", role=v1149.AgentRole.CRITIC, input="z", deps=["a"])
        d = v1149.AgentTask(id="d", role=v1149.AgentRole.SYNTHESIZER, input="w", deps=["b", "c"])
        for t in [a, b, c, d]:
            dag.add_task(t)
        order = dag.topo_sort()
        self.assertEqual(order[0], "a")
        self.assertEqual(order[-1], "d")
        self.assertIn("b", order[1:-1])
        self.assertIn("c", order[1:-1])

    def test_dag_cycle_detection(self):
        # 验证 has_cycle + topo_sort 能检测 cycle
        # 真造 cycle: a → b → c → a (用 add_task 允许循环 deps)
        dag = v1149.AgentDAG()
        # 先加 tasks 不带 deps, 再手动加 cycle edge
        for tid in ["a", "b", "c"]:
            dag.add_task(v1149.AgentTask(id=tid, role=v1149.AgentRole.PLANNER, input=tid))
        # 手动注入 cycle edges
        dag.edges = [("a", "b"), ("b", "c"), ("c", "a")]
        self.assertTrue(dag.has_cycle())
        with self.assertRaises(ValueError):
            dag.topo_sort()


class TestV1149PlanTask(unittest.TestCase):
    """V1149 真测: _plan_task_for_role 真按 role 拆."""

    def test_plan_planner(self):
        s = v1149._plan_task_for_role("build server", v1149.AgentRole.PLANNER)
        self.assertIn("[PLAN]", s)
        self.assertIn("build server", s)

    def test_plan_executor(self):
        s = v1149._plan_task_for_role("build server", v1149.AgentRole.EXECUTOR)
        self.assertIn("[EXEC]", s)

    def test_plan_critic(self):
        s = v1149._plan_task_for_role("build server", v1149.AgentRole.CRITIC)
        self.assertIn("[CRITIC]", s)

    def test_plan_refiner(self):
        s = v1149._plan_task_for_role("build server", v1149.AgentRole.REFINER)
        self.assertIn("[REFINE]", s)

    def test_plan_synthesizer(self):
        s = v1149._plan_task_for_role("build server", v1149.AgentRole.SYNTHESIZER)
        self.assertIn("[SYNTH]", s)

    def test_plan_empty_task(self):
        s = v1149._plan_task_for_role("", v1149.AgentRole.PLANNER)
        self.assertIn("(empty task)", s)


class TestV1149ExecuteTask(unittest.TestCase):
    """V1149 真测: _execute_task 真 mock execute."""

    def test_execute_planner(self):
        t = v1149.AgentTask(id="t1", role=v1149.AgentRole.PLANNER, input="x")
        v1149._execute_task(t)
        self.assertEqual(t.status, "done")
        self.assertIn("plan:", t.output)
        self.assertGreater(t.duration_ms, 0)

    def test_execute_executor(self):
        t = v1149.AgentTask(id="t1", role=v1149.AgentRole.EXECUTOR, input="y")
        v1149._execute_task(t)
        self.assertEqual(t.status, "done")
        self.assertIn("executed:", t.output)

    def test_execute_critic(self):
        t = v1149.AgentTask(id="t1", role=v1149.AgentRole.CRITIC, input="z")
        v1149._execute_task(t)
        self.assertEqual(t.status, "done")
        self.assertIn("critique:", t.output)

    def test_execute_refiner(self):
        t = v1149.AgentTask(id="t1", role=v1149.AgentRole.REFINER, input="w")
        v1149._execute_task(t)
        self.assertEqual(t.status, "done")
        self.assertIn("refined:", t.output)

    def test_execute_synthesizer(self):
        t = v1149.AgentTask(id="t1", role=v1149.AgentRole.SYNTHESIZER, input="v")
        v1149._execute_task(t)
        self.assertEqual(t.status, "done")
        self.assertIn("synthesized:", t.output)

    def test_execute_with_custom_executor(self):
        def custom_exec(task):
            return f"CUSTOM: {task.input}"
        t = v1149.AgentTask(id="t1", role=v1149.AgentRole.EXECUTOR, input="hello")
        v1149._execute_task(t, executor=custom_exec)
        self.assertEqual(t.output, "CUSTOM: hello")
        self.assertEqual(t.status, "done")


class TestV1149BuildDefaultDAG(unittest.TestCase):
    """V1149 真测: _build_default_dag 真 5 节点 + 4 边."""

    def test_default_dag_has_5_tasks(self):
        dag = v1149._build_default_dag("test task")
        self.assertEqual(len(dag.tasks), 5)
        self.assertEqual(len(dag.edges), 4)

    def test_default_dag_roles(self):
        dag = v1149._build_default_dag("test task")
        roles = [t.role for t in dag.tasks.values()]
        self.assertEqual(roles[0], v1149.AgentRole.PLANNER)
        self.assertEqual(roles[1], v1149.AgentRole.EXECUTOR)
        self.assertEqual(roles[2], v1149.AgentRole.CRITIC)
        self.assertEqual(roles[3], v1149.AgentRole.REFINER)
        self.assertEqual(roles[4], v1149.AgentRole.SYNTHESIZER)

    def test_default_dag_topo_order(self):
        dag = v1149._build_default_dag("test task")
        order = dag.topo_sort()
        self.assertEqual(len(order), 5)
        self.assertEqual(order[0], "t1_plan")
        self.assertEqual(order[-1], "t5_synth")


class TestV1149RunMultiAgent(unittest.TestCase):
    """V1149 真测: run_multi_agent 真入口."""

    def test_run_default(self):
        result = v1149.run_multi_agent("Build a simple HTTP server")
        self.assertEqual(result.n_tasks, 5)
        self.assertEqual(result.n_done, 5)
        self.assertEqual(result.n_failed, 0)
        self.assertEqual(result.success_rate, 1.0)
        self.assertEqual(len(result.topo_order), 5)
        # final_output = synthesizer task 的 output, 格式 "synthesized: [SYNTH] ..."
        self.assertIn("synthesized", result.final_output)
        self.assertIn("[SYNTH]", result.final_output)

    def test_run_with_custom_dag(self):
        dag = v1149._build_default_dag("custom task")
        result = v1149.run_multi_agent("custom task", dag=dag)
        self.assertEqual(result.n_done, 5)

    def test_run_with_failing_executor(self):
        def fail_exec(task):
            raise RuntimeError("simulated failure")
        dag = v1149._build_default_dag("failing task")
        result = v1149.run_multi_agent("failing task", dag=dag, executor=fail_exec)
        self.assertEqual(result.n_failed, 5)
        self.assertEqual(result.n_done, 0)
        self.assertEqual(result.success_rate, 0.0)

    def test_run_to_dict(self):
        result = v1149.run_multi_agent("test")
        d = result.to_dict()
        self.assertIn("snapshot_id", d)
        self.assertEqual(d["n_tasks"], 5)
        self.assertEqual(d["success_rate"], 1.0)
        self.assertEqual(len(d["tasks"]), 5)


class TestV1149PhilosophyGuard(unittest.TestCase):
    """V1149 真测: V3 哲学守门 5 键 (主 17:58 + 主 20:46)."""

    def test_no_pretend_real_llm(self):
        self.assertIn("agent_role_is_not_real_llm_agent", v1149.V1149_GUARDS)

    def test_no_pretend_optimal_dag(self):
        self.assertIn("dag_is_not_optimal_topology", v1149.V1149_GUARDS)

    def test_no_pretend_asi(self):
        self.assertIn("multi_agent_is_not_asi", v1149.V1149_GUARDS)

    def test_no_pretend_real_executor(self):
        self.assertIn("mock_executor_is_not_real_execution", v1149.V1149_GUARDS)

    def test_no_pretend_copy(self):
        self.assertIn("v1149_borrows_not_copies", v1149.V1149_GUARDS)


class TestV1149MainCLI(unittest.TestCase):
    """V1149 真测: main CLI 跑通 (主 00:56 任何人都能接手)."""

    def test_main_default(self):
        import contextlib
        import io
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            try:
                rc = v1149.main([])
            except SystemExit:
                rc = 0
        out = buf.getvalue()
        self.assertIn("V1149 真跑完成", out)
        self.assertIn("snapshot_id", out)
        self.assertIn("n_tasks: 5", out)

    def test_main_json(self):
        import contextlib
        import io
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            try:
                rc = v1149.main(["--task", "refactor code", "--json"])
            except SystemExit:
                rc = 0
        out = buf.getvalue()
        self.assertIn("V1149 真跑完成", out)
        self.assertIn("topo_order", out)


if __name__ == "__main__":
    unittest.main(verbosity=2)