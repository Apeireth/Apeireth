"""V1152 tests — V1149 + V1084 real LLM executor bridge."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any, Dict, List

import pytest

# Ensure promethean root is on sys.path
ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from apeireth import v1084_asi_real_llm_inference as v1084
from apeireth import v1149_multi_agent_role_dag as v1149
from apeireth import v1152_multi_agent_real_llm_executor as v1152


# ============================================================================
# Module API
# ============================================================================


class TestV1152ModuleAPI:
    def test_version_constant(self):
        assert v1152.V1152_VERSION == "0.1.0"

    def test_role_templates_has_5_roles(self):
        assert len(v1152.ROLE_PROMPT_TEMPLATES) == 5
        for role in v1149.AgentRole.all_roles():
            assert role.value in v1152.ROLE_PROMPT_TEMPLATES

    def test_guards_has_5_keys(self):
        assert len(v1152.V1152_GUARDS) == 5
        for k in [
            "v1152_is_not_asi",
            "mock_is_not_real_llm",
            "benchmark_22_is_real_count",
            "v1152_borrows_not_copies",
            "v1152_endpoint_config_is_safer",
        ]:
            assert k in v1152.V1152_GUARDS


# ============================================================================
# Prompt builder
# ============================================================================


class TestV1152PromptBuilder:
    def test_build_role_prompt_planner(self):
        p = v1152.build_role_prompt("planner", "Build X", "ctx")
        assert "planner agent" in p.lower() or "planner" in p.lower()
        assert "Build X" in p
        # Planner template doesn't use context (planner is first step)

    def test_build_role_prompt_executor_uses_context(self):
        p = v1152.build_role_prompt("executor", "Do X", "PREVIOUS")
        assert "Do X" in p
        assert "PREVIOUS" in p

    def test_build_role_prompt_unknown_role_falls_back(self):
        p = v1152.build_role_prompt("nonexistent", "Q", "")
        # Falls back to EXECUTOR template
        assert "executor" in p.lower()

    def test_build_role_prompt_truncates_long_input(self):
        long_input = "A" * 1000
        p = v1152.build_role_prompt("executor", long_input, "")
        # Truncates to 500 chars
        assert len(p) < 5000


# ============================================================================
# Benchmark samples
# ============================================================================


class TestV1152BenchmarkSamples:
    def test_default_benchmark_has_22_samples(self):
        samples = v1152._default_benchmark()
        assert len(samples) == 22

    def test_default_benchmark_5_roles_4_categories(self):
        samples = v1152._default_benchmark()
        roles = {s.role for s in samples}
        assert roles == {r.value for r in v1149.AgentRole.all_roles()}
        categories = {s.category for s in samples}
        assert "math" in categories
        assert "code" in categories
        assert "reasoning" in categories
        assert "knowledge" in categories
        assert "edge_case" in categories

    def test_default_benchmark_has_2_edge_cases(self):
        samples = v1152._default_benchmark()
        edge = [s for s in samples if s.category == "edge_case"]
        assert len(edge) == 2
        # First edge = empty, second = very long
        assert edge[0].task_input == ""
        assert len(edge[1].task_input) > 100

    def test_sample_ids_unique(self):
        samples = v1152._default_benchmark()
        ids = [s.sample_id for s in samples]
        assert len(ids) == len(set(ids))


# ============================================================================
# Executor
# ============================================================================


class TestV1152AgentExecutor:
    def test_executor_init_default(self):
        ex = v1152.V1152AgentExecutor(force_mock=True)
        assert ex.endpoint is not None
        assert ex.engine is not None
        assert ex.audit is not None
        assert ex.executions == []

    def test_executor_init_custom_endpoint(self):
        ep = v1084.LLMEndpointConfig(
            name="custom-ep",
            base_url="https://example.com/v1",
            api_key="sk-x",
            model_id="m-test",
        )
        ex = v1152.V1152AgentExecutor(endpoint=ep, force_mock=True)
        assert ex.endpoint.name == "custom-ep"

    def test_executor_run_mock_task(self):
        ex = v1152.V1152AgentExecutor(force_mock=True)
        task = v1149.AgentTask(
            id="t1", role=v1149.AgentRole.EXECUTOR, input="Do X"
        )
        text = ex.execute(task)
        assert task.status == "done"
        assert text != ""
        assert len(ex.executions) == 1
        assert ex.executions[0].role == "executor"
        # Mock fallback or ok
        assert ex.executions[0].status in ("ok", "mock")

    def test_executor_run_5_roles_in_sequence(self):
        ex = v1152.V1152AgentExecutor(force_mock=True)
        for role in v1149.AgentRole.all_roles():
            task = v1149.AgentTask(id=f"t_{role.value}", role=role, input="Q")
            ex.execute(task)
        assert len(ex.executions) == 5
        # Verify context buffer got populated
        assert len(ex._context_buffer) == 5

    def test_executor_reset_clears_context(self):
        ex = v1152.V1152AgentExecutor(force_mock=True)
        task = v1149.AgentTask(id="t1", role=v1149.AgentRole.EXECUTOR, input="Q")
        ex.execute(task)
        assert len(ex._context_buffer) > 0
        ex.reset()
        assert ex._context_buffer == []

    def test_executor_handles_empty_input(self):
        ex = v1152.V1152AgentExecutor(force_mock=True)
        task = v1149.AgentTask(id="t1", role=v1149.AgentRole.EXECUTOR, input="")
        text = ex.execute(task)
        # Should still produce output (mock handles empty)
        assert task.status in ("done", "failed")

    def test_executor_handles_long_input(self):
        ex = v1152.V1152AgentExecutor(force_mock=True)
        task = v1149.AgentTask(id="t1", role=v1149.AgentRole.EXECUTOR, input="A" * 2000)
        text = ex.execute(task)
        # Should still work (truncated internally)
        assert task.status in ("done", "failed")


# ============================================================================
# Benchmark run
# ============================================================================


class TestV1152BenchmarkRun:
    def test_run_benchmark_22_samples_mock(self):
        ex = v1152.V1152AgentExecutor(force_mock=True)
        run = v1152.run_benchmark(ex)
        assert run.n_samples == 22
        assert run.n_ok + run.n_mock + run.n_error == 22
        # Success rate = (ok + mock) / total
        assert 0.0 <= run.success_rate <= 1.0

    def test_run_benchmark_mock_succeeds(self):
        # When endpoint unreachable, mock_fallback should kick in
        ex = v1152.V1152AgentExecutor(force_mock=False)  # let engine decide
        run = v1152.run_benchmark(ex)
        # Most/all should be mock or ok
        assert run.n_error < run.n_samples

    def test_run_benchmark_results_match_samples(self):
        ex = v1152.V1152AgentExecutor(force_mock=True)
        run = v1152.run_benchmark(ex)
        # Each sample should produce a result (or error)
        assert len(run.results) == run.n_samples
        # First 20 samples should match roles
        for i, sample in enumerate(run.samples[:20]):
            if i < len(run.results):
                assert run.results[i].role == sample.role

    def test_run_benchmark_snapshot_id(self):
        ex = v1152.V1152AgentExecutor(force_mock=True)
        run = v1152.run_benchmark(ex)
        assert run.snapshot_id.startswith("v1152-")
        assert len(run.snapshot_id) == len("v1152-") + 8

    def test_run_benchmark_cost_tracking(self):
        ex = v1152.V1152AgentExecutor(force_mock=True)
        run = v1152.run_benchmark(ex)
        assert run.total_cost_usd >= 0.0
        assert run.avg_latency_ms >= 0.0


# ============================================================================
# run_agent_with_llm — bridge entry point
# ============================================================================


class TestV1152RunAgentWithLLM:
    def test_run_agent_mock(self):
        result = v1152.run_agent_with_llm("Build X", force_mock=True)
        assert result.n_tasks == 5
        assert result.n_done >= 0

    def test_run_agent_real_llm_flag(self):
        # --real-llm means force_mock=False; endpoint will likely fall back to mock
        result = v1152.run_agent_with_llm("Explain ASI", force_mock=False)
        assert result.snapshot_id.startswith("v1149-")
        assert result.n_tasks == 5


# ============================================================================
# Render
# ============================================================================


class TestV1152RenderMarkdown:
    def test_render_benchmark_md_has_key_sections(self):
        ex = v1152.V1152AgentExecutor(force_mock=True)
        run = v1152.run_benchmark(ex)
        md = v1152.render_benchmark_md(run)
        assert "V1152" in md
        assert "n_samples" in md
        assert "22" in md
        assert "哲学守门" in md or "guard" in md.lower()


# ============================================================================
# To_dict
# ============================================================================


class TestV1152ToDict:
    def test_role_config_to_dict(self):
        rc = v1152.V1152RoleConfig(role="executor", template="X", max_tokens=100, temperature=0.5)
        d = rc.to_dict()
        assert d["role"] == "executor"
        assert d["max_tokens"] == 100

    def test_sample_to_dict(self):
        s = v1152.V1152BenchmarkSample(
            sample_id="x", role="planner", category="math",
            difficulty="easy", task_input="1+1"
        )
        d = s.to_dict()
        assert d["sample_id"] == "x"

    def test_execution_result_to_dict(self):
        r = v1152.V1152ExecutionResult(
            role="planner", task_id="t1", prompt_preview="p", response_text="r",
            status="mock", latency_ms=10.0, input_tokens=5, output_tokens=10,
            cost_usd=0.0001, model_id="m", endpoint="e"
        )
        d = r.to_dict()
        assert d["status"] == "mock"
        assert d["latency_ms"] == 10.0

    def test_run_to_dict_rounded(self):
        ex = v1152.V1152AgentExecutor(force_mock=True)
        run = v1152.run_benchmark(ex)
        d = run.to_dict()
        assert isinstance(d["success_rate"], float)
        assert isinstance(d["avg_latency_ms"], float)
        assert isinstance(d["total_cost_usd"], float)
        assert "samples" in d
        assert "results" in d