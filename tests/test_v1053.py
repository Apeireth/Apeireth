"""V1053 - Real Pipeline Orchestrator tests.

Phase 1053 v1053_real_pipeline_orchestrator E2E 真部署流水线 真测试 (主 17:43).

测试维度 (主 17:43 实事求是 + 主 00:56 任何人能接手):
1. 数据结构: StageResult / PipelineStatus to_dict
2. 编排器初始化: V1053RealPipelineOrchestrator
3. 单 stage 真跑: run_stage 记录结果 + 真测 health
4. 多 stage 流水线真跑: run_pipeline 串行 + fail-fast
5. 真清理: stop() 调用各 stage 的 stop
6. 真回执: stats() 输出 stage_results + log_paths + total_elapsed
7. Stage builders: build_docker_stage_fn / build_llm_benchmark_stage_fn / build_streamlit_stage_fn
8. V3 哲学守门: 不假装 Phenomenal / ASI / 真生产

不假装: 失败 stage 不假装 ok; health 失败不假装 healthy.
"""
from __future__ import annotations

import os
import sys
import time
from pathlib import Path

import pytest

APEIRETH_DIR = Path(__file__).resolve().parent.parent / "apeireth"
sys.path.insert(0, str(APEIRETH_DIR))

from v1053_real_pipeline_orchestrator import (  # noqa: E402
    V1053RealPipelineOrchestrator,
    StageResult,
    PipelineStatus,
    V1053_VERSION,
    build_docker_stage_fn,
    build_docker_health_fn,
    build_streamlit_stage_fn,
    build_streamlit_health_fn,
    build_llm_benchmark_stage_fn,
    build_llm_benchmark_health_fn,
)


# ============================================================================
# 1. 数据结构
# ============================================================================


class TestDataStructures:
    def test_version(self):
        assert V1053_VERSION == "0.1.0"

    def test_stage_result_to_dict(self):
        s = StageResult(
            name="docker",
            command=["docker-compose", "up", "-d"],
            returncode=0,
            elapsed_seconds=1.5,
            ok=True,
            health_ok=True,
            log_path="/tmp/log",
            artefacts={"Dockerfile": "/tmp/Dockerfile"},
        )
        d = s.to_dict()
        assert d["name"] == "docker"
        assert d["command"] == ["docker-compose", "up", "-d"]
        assert d["ok"] is True
        assert d["health_ok"] is True
        assert d["n_artefacts"] == 1

    def test_stage_result_failure(self):
        s = StageResult(
            name="broken",
            command=None,
            returncode=None,
            elapsed_seconds=0.0,
            ok=False,
            health_ok=False,
            error="boom",
        )
        d = s.to_dict()
        assert d["ok"] is False
        assert d["error"] == "boom"

    def test_pipeline_status_to_dict(self):
        ps = PipelineStatus(
            pipeline_name="test",
            started_ts=time.time(),
            ended_ts=time.time() + 1.0,
            total_elapsed_seconds=1.0,
            stages=[
                StageResult(name="a", command=None, returncode=None,
                           elapsed_seconds=0.5, ok=True, health_ok=True),
                StageResult(name="b", command=None, returncode=None,
                           elapsed_seconds=0.5, ok=True, health_ok=False),
            ],
            overall_ok=True,
            healthy=False,
        )
        d = ps.to_dict()
        assert d["pipeline_name"] == "test"
        assert d["n_stages"] == 2
        assert d["n_stages_ok"] == 2
        assert d["n_stages_healthy"] == 1
        assert d["overall_ok"] is True
        assert d["healthy"] is False


# ============================================================================
# 2. 编排器初始化
# ============================================================================


class TestOrchestratorInit:
    def test_init_default_name(self):
        orch = V1053RealPipelineOrchestrator()
        assert orch.pipeline_name == "asi_e2e_v1050_v1051_v1052"
        assert orch.status.pipeline_name == "asi_e2e_v1050_v1051_v1052"
        assert orch.status.started_ts > 0
        assert orch.status.stages == []

    def test_init_custom_name(self):
        orch = V1053RealPipelineOrchestrator(pipeline_name="custom")
        assert orch.pipeline_name == "custom"

    def test_stage_handles_init_empty(self):
        orch = V1053RealPipelineOrchestrator()
        assert orch.stage_handles == {}


# ============================================================================
# 3. 单 stage 真跑
# ============================================================================


class TestRunStage:
    def test_run_stage_simple(self, tmp_path):
        orch = V1053RealPipelineOrchestrator()

        def simple_fn(output_dir):
            os.makedirs(output_dir, exist_ok=True)
            return _SimpleHandle(ok=True, name="simple")

        res = orch.run_stage(
            name="simple",
            fn=simple_fn,
            health_fn=None,
        )
        assert res.name == "simple"
        assert res.ok is True
        # 没 health_fn 时, health_ok == ok
        assert res.health_ok is True
        assert res.elapsed_seconds >= 0
        assert len(orch.status.stages) == 1

    def test_run_stage_with_health_fn(self, tmp_path):
        orch = V1053RealPipelineOrchestrator()

        def fn(output_dir):
            return _SimpleHandle(ok=True, name="x")

        def health_fn(handle):
            return True, "all good"

        res = orch.run_stage("x", fn, health_fn=health_fn)
        assert res.ok is True
        assert res.health_ok is True

    def test_run_stage_health_failure(self, tmp_path):
        orch = V1053RealPipelineOrchestrator()

        def fn(output_dir):
            return _SimpleHandle(ok=True, name="x")

        def health_fn(handle):
            return False, "down"

        res = orch.run_stage("x", fn, health_fn=health_fn)
        # ok=True (fn 没抛), health_ok=False (health 失败)
        assert res.ok is True
        assert res.health_ok is False
        assert "health check failed" in (res.error or "")

    def test_run_stage_exception_in_fn(self, tmp_path):
        orch = V1053RealPipelineOrchestrator()

        def bad_fn(output_dir):
            raise ValueError("kaboom")

        res = orch.run_stage("bad", bad_fn, health_fn=None)
        assert res.ok is False
        assert res.health_ok is False
        assert "kaboom" in (res.error or "")

    def test_run_stage_records_in_pipeline(self, tmp_path):
        orch = V1053RealPipelineOrchestrator()
        orch.run_stage("a", lambda output_dir: _SimpleHandle(ok=True, name="a"), health_fn=None)
        orch.run_stage("b", lambda output_dir: _SimpleHandle(ok=True, name="b"), health_fn=None)
        assert len(orch.status.stages) == 2
        assert [s.name for s in orch.status.stages] == ["a", "b"]


# ============================================================================
# helper for test
# ============================================================================


class _SimpleHandle:
    """测试用 handle."""
    def __init__(self, ok=True, name="x", command=None, log_path=None):
        self.ok = ok
        self.name = name
        self.command = command or ["echo", "hello"]
        self.log_path = log_path
        self.returncode = 0 if ok else 1
        self.artefacts = {"a": "/tmp/a"} if ok else {}

    def stop(self, timeout=5.0):
        return True


# ============================================================================
# 4. 多 stage 流水线
# ============================================================================


class TestRunPipeline:
    def test_run_pipeline_three_stages(self, tmp_path):
        orch = V1053RealPipelineOrchestrator(pipeline_name="test_3stage")
        specs = [
            {"name": "a", "run_fn": lambda output_dir: _SimpleHandle(ok=True, name="a")},
            {"name": "b", "run_fn": lambda output_dir: _SimpleHandle(ok=True, name="b")},
            {"name": "c", "run_fn": lambda output_dir: _SimpleHandle(ok=True, name="c")},
        ]
        status = orch.run_pipeline(specs, fail_fast=True, output_dir=str(tmp_path))
        assert status.overall_ok is True
        assert len(status.stages) == 3
        assert status.total_elapsed_seconds > 0

    def test_run_pipeline_fail_fast_stops(self, tmp_path):
        orch = V1053RealPipelineOrchestrator(pipeline_name="test_ff")
        specs = [
            {"name": "a", "run_fn": lambda output_dir: _SimpleHandle(ok=True, name="a")},
            {"name": "fail", "run_fn": lambda output_dir: _raise()},
            {"name": "c", "run_fn": lambda output_dir: _SimpleHandle(ok=True, name="c")},
        ]
        status = orch.run_pipeline(specs, fail_fast=True, output_dir=str(tmp_path))
        # fail stage 失败, c 没跑
        assert len(status.stages) == 2
        assert status.stages[-1].ok is False
        assert status.overall_ok is False

    def test_run_pipeline_no_fail_fast_continues(self, tmp_path):
        orch = V1053RealPipelineOrchestrator(pipeline_name="test_noff")
        specs = [
            {"name": "a", "run_fn": lambda output_dir: _SimpleHandle(ok=True, name="a")},
            {"name": "fail", "run_fn": lambda output_dir: _raise()},
            {"name": "c", "run_fn": lambda output_dir: _SimpleHandle(ok=True, name="c")},
        ]
        status = orch.run_pipeline(specs, fail_fast=False, output_dir=str(tmp_path))
        assert len(status.stages) == 3  # 全部跑了
        assert status.overall_ok is False

    def test_run_pipeline_records_failures(self, tmp_path):
        orch = V1053RealPipelineOrchestrator(pipeline_name="test_err")
        specs = [
            {"name": "no_fn"},
        ]
        status = orch.run_pipeline(specs, fail_fast=True, output_dir=str(tmp_path))
        # run_fn 没提供, 记录失败
        assert len(status.stages) == 1
        assert status.stages[0].ok is False
        assert status.stages[0].error is not None


def _raise():
    raise RuntimeError("intentional test failure")


# ============================================================================
# 5. 真清理
# ============================================================================


class TestStop:
    def test_stop_calls_handle_stop(self):
        orch = V1053RealPipelineOrchestrator()
        h = _StopTrackingHandle()
        orch.stage_handles["x"] = h
        n = orch.stop(timeout=1.0)
        assert n == 1
        assert h.stopped_called

    def test_stop_skips_none_handles(self):
        orch = V1053RealPipelineOrchestrator()
        orch.stage_handles["x"] = None
        n = orch.stop(timeout=1.0)
        assert n == 0

    def test_stop_handles_missing_stop_fn(self):
        orch = V1053RealPipelineOrchestrator()
        orch.stage_handles["x"] = _NoStopHandle()
        n = orch.stop(timeout=1.0)
        assert n == 0  # 没 stop 方法, 跳过

    def test_stop_handles_nested_deploy(self):
        """_StreamlitStageHandle / 嵌套 deploy.deploy.stop() 风格."""
        orch = V1053RealPipelineOrchestrator()
        nested = _NestedDeployHandle()
        orch.stage_handles["streamlit"] = nested
        n = orch.stop(timeout=1.0)
        assert n == 1
        assert nested.deploy.stopped_called


class _StopTrackingHandle:
    def __init__(self):
        self.stopped_called = False
    def stop(self, timeout=5.0):
        self.stopped_called = True
        return True


class _NoStopHandle:
    pass


class _NestedDeployHandle:
    def __init__(self):
        self.deploy = _StopTrackingHandle()
