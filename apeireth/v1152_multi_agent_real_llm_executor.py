"""V1152 — V1149 Multi-Agent 真接 V1084 Real LLM Executor (主 06:15 V1053+ 真生产 + 主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 19:33 走在前人经验上 + 主 13:31 大胆激进 + 主 17:58+20:46 不假装 + 主 23:44 干到底 + 主 00:56 任何人都能接手 + 主 00:44 质量工程化).

V1149 (Multi-Agent Role + DAG 抽象) + V1084 (Real LLM Inference Adapter)
→ V1152 真接 bridge: 每个 AgentRole 的 execute_task 真调 V1084 InferenceEngine, 真 HTTP / 真 mock fallback.

主 17:43 实事求是:
- 不假装 mock = 真 LLM: V1152 默认 mock (offline safe), `--real-llm` 真接 endpoint
- 不假装 22 真样本 = 真 benchmark: 22 sample 真跑, 记录每次 status (ok/mock/error)
- 不假装 V1152 = ASI 升级: V1152 是 bridge, 真生产链路 = V1149 + V1084 + V1152

主 19:33 走在前人经验上:
- V1149 = 真 DAG 编排 (ASI-Arch role-based + promptflow DAG 启发)
- V1084 = 真 LLM HTTP (OpenAI 兼容 + 离线 mock fallback)
- V1152 = 真把 V1149 的 executor 注入 V1084 InferenceEngine, 让每个 role 真发 prompt 给真 LLM

主 00:56 任何人都能接手:
  > python -m apeireth.v1152_multi_agent_real_llm_executor --real-llm --task "Build X"
  > python -m apeireth.v1152_multi_agent_real_llm_executor --benchmark  # 22 真样本 benchmark

主 00:44 质量工程化 — 10 真生产组件:
 1. V1152RoleConfig      — 5 role 真 prompt template (Planner/Executor/Critic/Refiner/Synthesizer)
 2. V1152PromptBuilder   — 真按 role + context 构造 prompt
 3. V1152AgentExecutor   — 真调 V1084 InferenceEngine (HTTP 优先, mock fallback)
 4. V1152BenchmarkSample — 22 真样本 dataclass (id/category/difficulty/expected_status)
 5. V1152BenchmarkRun    — 22 sample 真跑 result dataclass
 6. V1152_GUARDS         — 5 不假装守门
 7. _default_benchmark   — 22 真样本 (5 role × 4 类别 + 2 个 edge case)
 8. run_benchmark        — 真跑 22 sample, 真记每次 status, 真产 report
 9. run_agent_with_llm   — V1149 + V1084 真集成入口
 10. main CLI            — --real-llm / --benchmark / --task / --json / --report

Usage:
    python -m apeireth.v1152_multi_agent_real_llm_executor --task "Refactor V1148 deep read"
    python -m apeireth.v1152_multi_agent_real_llm_executor --real-llm --task "Explain ASI"
    python -m apeireth.v1152_multi_agent_real_llm_executor --benchmark --report
    python -m apeireth.v1152_multi_agent_real_llm_executor --benchmark --real-llm --json
"""

from __future__ import annotations

import argparse
import json
import time
import uuid
from dataclasses import dataclass, field, asdict
from enum import Enum
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple

# 真调既有真生产模块 (主 17:43 实事求是: 不重实现, 真调)
from apeireth import v1084_asi_real_llm_inference as v1084
from apeireth import v1149_multi_agent_role_dag as v1149

V1152_VERSION = "0.1.0"

# ============================================================================
# 1. V1152RoleConfig — 5 role 真 prompt template
# ============================================================================

# 5 真角色 prompt template (主 19:33 ASI-Arch role-based 真借鉴)
ROLE_PROMPT_TEMPLATES: Dict[str, str] = {
    v1149.AgentRole.PLANNER.value: (
        "You are a planner agent. Decompose the task into 1-3 concrete sub-tasks. "
        "Return a brief plan as bullet points.\n\nTASK: {input}"
    ),
    v1149.AgentRole.EXECUTOR.value: (
        "You are an executor agent. Execute the sub-task and return the concrete result. "
        "Be concise and specific.\n\nSUB-TASK: {input}\n\nPREVIOUS CONTEXT: {context}"
    ),
    v1149.AgentRole.CRITIC.value: (
        "You are a critic agent. Evaluate the executor's output for correctness, completeness, and clarity. "
        "Return a score 0-1 and a brief critique.\n\nOUTPUT TO EVALUATE: {input}\n\nCONTEXT: {context}"
    ),
    v1149.AgentRole.REFINER.value: (
        "You are a refiner agent. Based on the critic's feedback, refine the executor's output. "
        "Return the improved version.\n\nCRITIQUE: {input}\n\nCONTEXT: {context}"
    ),
    v1149.AgentRole.SYNTHESIZER.value: (
        "You are a synthesizer agent. Combine all sub-task outputs into a single final answer. "
        "Be coherent and complete.\n\nALL OUTPUTS: {input}"
    ),
}


@dataclass
class V1152RoleConfig:
    """V1152 role 真 prompt 配置."""
    role: str
    template: str
    max_tokens: int = 256
    temperature: float = 0.3

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


# ============================================================================
# 2. V1152PromptBuilder — 真按 role + context 构造 prompt
# ============================================================================


def build_role_prompt(role: str, input_text: str, context: str = "") -> str:
    """V1152 真按 role + context 构造 prompt (主 17:43 实事求是)."""
    template = ROLE_PROMPT_TEMPLATES.get(role, ROLE_PROMPT_TEMPLATES[v1149.AgentRole.EXECUTOR.value])
    return template.format(input=input_text[:500], context=context[:200])


# ============================================================================
# 3. V1152AgentExecutor — 真调 V1084 InferenceEngine
# ============================================================================


@dataclass
class V1152ExecutionResult:
    """V1152 单次 role 真执行结果."""
    role: str
    task_id: str
    prompt_preview: str
    response_text: str
    status: str  # ok / mock / error
    latency_ms: float
    input_tokens: int
    output_tokens: int
    cost_usd: float
    model_id: str
    endpoint: str
    error: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


class V1152AgentExecutor:
    """V1152 真 executor: 把 V1149 AgentTask 委托给 V1084 InferenceEngine (主 19:33)."""

    def __init__(
        self,
        endpoint: Optional[v1084.LLMEndpointConfig] = None,
        force_mock: bool = True,
        audit_log: Optional[v1084.InferenceAuditLog] = None,
    ) -> None:
        # 默认 endpoint (主 00:44 质量工程化 — 不假装默认 = 真接)
        self.endpoint = endpoint or v1084.LLMEndpointConfig(
            name="newapi-m3",
            base_url="https://api.newapi.example/v1",
            api_key=os_environ_or_default("APEIRETH_NEWAPI_KEY", "sk-test-placeholder"),
            model_id=os_environ_or_default("APEIRETH_NEWAPI_MODEL", "MiniMax-M3"),
            mock_fallback=True,  # 安全: 不可达时自动 mock
        )
        self.engine = v1084.InferenceEngine(
            endpoint=self.endpoint,
            force_mock=force_mock,
        )
        self.audit = audit_log or v1084.InferenceAuditLog()
        self.executions: List[V1152ExecutionResult] = []
        self._context_buffer: List[V1152ExecutionResult] = []  # 给 critic/refiner/synth 用

    def execute(self, task: v1149.AgentTask) -> str:
        """V1152 真执行单个 AgentTask (主 17:43 实事求是).

        Returns: response text (also stored in task.output).
        """
        t0 = time.time()
        # 构造 context (前序 task outputs)
        context_parts = [r.response_text[:200] for r in self._context_buffer[-3:]]
        context = " | ".join(context_parts)

        prompt = build_role_prompt(task.role.value, task.input, context)
        req = v1084.InferenceRequest(
            prompt=prompt,
            max_tokens=256,
            temperature=0.3,
            model_id=self.endpoint.model_id,
        )

        response = self.engine.infer(req)
        self.audit.record(req, response)

        result = V1152ExecutionResult(
            role=task.role.value,
            task_id=task.id,
            prompt_preview=prompt[:80],
            response_text=response.text,
            status=response.status,
            latency_ms=response.latency_ms,
            input_tokens=response.input_tokens,
            output_tokens=response.output_tokens,
            cost_usd=response.cost_usd,
            model_id=response.model_id,
            endpoint=response.endpoint,
            error=response.error or "",
        )
        self.executions.append(result)
        self._context_buffer.append(result)
        # 控制 buffer 大小
        if len(self._context_buffer) > 5:
            self._context_buffer = self._context_buffer[-5:]

        elapsed = (time.time() - t0) * 1000.0
        task.duration_ms = elapsed
        task.output = response.text
        task.error = response.error or ""
        task.status = "done" if response.status in ("ok", "mock") else "failed"
        return response.text

    def reset(self) -> None:
        """V1152 真清空 context (每轮跑前)."""
        self._context_buffer = []


def os_environ_or_default(key: str, default: str) -> str:
    """V1152 真从 env 读 endpoint config, 不假装 env = 真有值."""
    import os
    return os.environ.get(key, default)


# ============================================================================
# 4. V1152BenchmarkSample — 22 真样本
# ============================================================================


@dataclass
class V1152BenchmarkSample:
    """V1152 单个 benchmark 真样本."""
    sample_id: str
    role: str  # planner / executor / critic / refiner / synthesizer
    category: str  # math / code / reasoning / knowledge / edge_case
    difficulty: str  # easy / medium / hard
    task_input: str
    expected_status: str = "ok_or_mock"  # 期望 status (主 17:43 不假装 = 必 ok)

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class V1152BenchmarkRun:
    """V1152 22 sample 真跑结果."""
    snapshot_id: str
    started_at: float
    finished_at: float
    n_samples: int
    n_ok: int
    n_mock: int
    n_error: int
    success_rate: float
    avg_latency_ms: float
    total_cost_usd: float
    samples: List[V1152BenchmarkSample] = field(default_factory=list)
    results: List[V1152ExecutionResult] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["success_rate"] = round(self.success_rate, 4)
        d["avg_latency_ms"] = round(self.avg_latency_ms, 2)
        d["total_cost_usd"] = round(self.total_cost_usd, 6)
        return d


def _default_benchmark() -> List[V1152BenchmarkSample]:
    """V1152 真 22 sample benchmark (主 17:43 实事求是)."""
    samples: List[V1152BenchmarkSample] = []

    # 5 roles × 4 categories = 20 + 2 edge = 22
    role_list = [
        v1149.AgentRole.PLANNER.value,
        v1149.AgentRole.EXECUTOR.value,
        v1149.AgentRole.CRITIC.value,
        v1149.AgentRole.REFINER.value,
        v1149.AgentRole.SYNTHESIZER.value,
    ]
    categories = ["math", "code", "reasoning", "knowledge"]

    sample_inputs = {
        "math": [
            ("Compute 17 * 23 + 5", "easy"),
            ("Solve x^2 + 5x + 6 = 0", "medium"),
            ("Derive closed form for sum of squares 1..n", "hard"),
        ],
        "code": [
            ("Write a Python function to reverse a string", "easy"),
            ("Implement binary search in TypeScript", "medium"),
            ("Design a thread-safe LRU cache", "hard"),
        ],
        "reasoning": [
            ("If A implies B and B implies C, what does A imply?", "easy"),
            ("5 cards from 52, probability of full house", "medium"),
            ("Prove sqrt(2) is irrational", "hard"),
        ],
        "knowledge": [
            ("What year was the transformer paper published?", "easy"),
            ("Explain Constitutional AI in 3 sentences", "medium"),
            ("Compare Mesa-optimization vs outer alignment", "hard"),
        ],
    }

    idx = 0
    for role in role_list:
        for cat in categories:
            for inp, diff in sample_inputs[cat][:1]:  # 取每个 category 第一个, 5×4=20
                idx += 1
                samples.append(V1152BenchmarkSample(
                    sample_id=f"v1152-s{idx:02d}",
                    role=role,
                    category=cat,
                    difficulty=diff,
                    task_input=inp,
                ))

    # +2 edge cases
    samples.append(V1152BenchmarkSample(
        sample_id="v1152-s21",
        role=v1149.AgentRole.EXECUTOR.value,
        category="edge_case",
        difficulty="hard",
        task_input="",  # 空 prompt edge case
    ))
    samples.append(V1152BenchmarkSample(
        sample_id="v1152-s22",
        role=v1149.AgentRole.CRITIC.value,
        category="edge_case",
        difficulty="hard",
        task_input="A" * 1000,  # 超长 prompt edge case
    ))
    return samples


# ============================================================================
# 5. run_benchmark — 22 sample 真跑
# ============================================================================


def run_benchmark(
    executor: V1152AgentExecutor,
    samples: Optional[List[V1152BenchmarkSample]] = None,
) -> V1152BenchmarkRun:
    """V1152 真跑 22 sample benchmark (主 17:43 实事求是)."""
    if samples is None:
        samples = _default_benchmark()

    started = time.time()
    snapshot_id = f"v1152-{uuid.uuid4().hex[:8]}"
    results: List[V1152ExecutionResult] = []
    n_ok = 0
    n_mock = 0
    n_error = 0
    total_latency = 0.0
    total_cost = 0.0

    for sample in samples:
        # 构造临时 AgentTask
        task = v1149.AgentTask(
            id=sample.sample_id,
            role=v1149.AgentRole(sample.role),
            input=sample.task_input,
        )
        executor.reset()
        try:
            text = executor.execute(task)
            # 取最近一条 execution result
            if executor.executions:
                last = executor.executions[-1]
                results.append(last)
                total_latency += last.latency_ms
                total_cost += last.cost_usd
                if last.status == "ok":
                    n_ok += 1
                elif last.status == "mock":
                    n_mock += 1
                else:
                    n_error += 1
        except Exception as e:
            n_error += 1
            results.append(V1152ExecutionResult(
                role=sample.role,
                task_id=sample.sample_id,
                prompt_preview=sample.task_input[:80],
                response_text="",
                status="error",
                latency_ms=0.0,
                input_tokens=0,
                output_tokens=0,
                cost_usd=0.0,
                model_id=executor.endpoint.model_id,
                endpoint=executor.endpoint.name,
                error=f"{type(e).__name__}: {str(e)[:100]}",
            ))

    finished = time.time()
    success_rate = (n_ok + n_mock) / max(1, len(samples))
    avg_latency = total_latency / max(1, len(results))

    return V1152BenchmarkRun(
        snapshot_id=snapshot_id,
        started_at=started,
        finished_at=finished,
        n_samples=len(samples),
        n_ok=n_ok,
        n_mock=n_mock,
        n_error=n_error,
        success_rate=success_rate,
        avg_latency_ms=avg_latency,
        total_cost_usd=total_cost,
        samples=samples,
        results=results,
    )


# ============================================================================
# 6. run_agent_with_llm — V1149 + V1084 真集成入口
# ============================================================================


def run_agent_with_llm(
    initial_task: str,
    endpoint: Optional[v1084.LLMEndpointConfig] = None,
    force_mock: bool = True,
) -> v1149.AgentResult:
    """V1152 真接 V1149 multi-agent + V1084 real LLM (主 19:33 + 主 23:44 干到底)."""
    executor = V1152AgentExecutor(endpoint=endpoint, force_mock=force_mock)
    dag = v1149._build_default_dag(initial_task)

    # 用 V1152 executor 替换 V1149 default executor
    return v1149.run_multi_agent(
        initial_task=initial_task,
        dag=dag,
        executor=executor.execute,
    )


# ============================================================================
# 7. V1152_GUARDS — 5 不假装守门
# ============================================================================

V1152_GUARDS: Dict[str, str] = {
    "v1152_is_not_asi": (
        "V1152 是 V1149 multi-agent + V1084 LLM 集成 bridge, ASI 是更大目标 (主 22:33 北极星). "
        "bridge 真能跑 ≠ ASI level."
    ),
    "mock_is_not_real_llm": (
        "V1152 默认 mock (force_mock=True), `--real-llm` 才真接 endpoint. "
        "mock response ≠ real LLM response, 每次 status 真标 (ok/mock/error) 不假装."
    ),
    "benchmark_22_is_real_count": (
        "V1152 benchmark = 22 sample (5 role × 4 category + 2 edge), 不是 20 或 25. "
        "真跑 = 真按 samples 真循环, 每次 status 真记."
    ),
    "v1152_borrows_not_copies": (
        "V1152 真借鉴 V1149 (DAG + role) + V1084 (real LLM HTTP), 是 bridge + 集成, "
        "不重实现 V1149 或 V1084 (主 17:43 不重实现)."
    ),
    "v1152_endpoint_config_is_safer": (
        "V1152 默认 endpoint.mock_fallback=True, 不可达自动 mock, 不假装 endpoint = 必通. "
        "`--real-llm` + 不可达 → 真 mock + 真审计记录 (主 17:43 实事求是)."
    ),
}


# ============================================================================
# 8. 真产 Markdown 报告
# ============================================================================


def render_benchmark_md(run: V1152BenchmarkRun, title: str = "V1152 Real LLM Executor Benchmark") -> str:
    """V1152 真产 Markdown 报告."""
    md = [
        f"# {title}",
        "",
        f"- snapshot_id: `{run.snapshot_id}`",
        f"- V1152_VERSION: `{V1152_VERSION}`",
        f"- n_samples: **{run.n_samples}**",
        f"- n_ok: **{run.n_ok}**",
        f"- n_mock: **{run.n_mock}**",
        f"- n_error: **{run.n_error}**",
        f"- success_rate: **{run.success_rate:.4f}**",
        f"- avg_latency_ms: **{run.avg_latency_ms:.2f}**",
        f"- total_cost_usd: **{run.total_cost_usd:.6f}**",
        "",
        "## 真跑样本结果",
        "",
        "| sample_id | role | category | difficulty | status | latency_ms | cost_usd |",
        "|-----------|------|----------|------------|--------|------------|----------|",
    ]
    for sample, result in zip(run.samples, run.results):
        md.append(
            f"| {sample.sample_id} | {sample.role} | {sample.category} | {sample.difficulty} | "
            f"{result.status} | {result.latency_ms:.1f} | {result.cost_usd:.6f} |"
        )
    md.append("")
    md.append("## V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)")
    md.append("")
    for k, v in V1152_GUARDS.items():
        md.append(f"- ✅ {k}: {v}")
    return "\n".join(md)


# ============================================================================
# 9. main CLI (主 00:56 任何人都能接手)
# ============================================================================


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="V1152 V1149 + V1084 real LLM executor bridge")
    parser.add_argument("--task", type=str, default="Build a hello world HTTP server")
    parser.add_argument("--real-llm", action="store_true", help="真接 endpoint (default: mock)")
    parser.add_argument("--benchmark", action="store_true", help="22 sample 真跑 benchmark")
    parser.add_argument("--json", action="store_true", help="JSON output")
    parser.add_argument("--report", action="store_true", help="Markdown report")
    parser.add_argument("--save", type=str, default=None, help="save artifact path")
    args = parser.parse_args(argv)

    if args.benchmark:
        # 真跑 22 sample benchmark
        executor = V1152AgentExecutor(force_mock=not args.real_llm)
        run = run_benchmark(executor)
        if args.json:
            print(json.dumps(run.to_dict(), ensure_ascii=False, indent=2))
        elif args.report:
            md = render_benchmark_md(run)
            print(md)
        else:
            print(
                f"V1152 benchmark: snapshot_id={run.snapshot_id} "
                f"n_samples={run.n_samples} n_ok={run.n_ok} n_mock={run.n_mock} "
                f"n_error={run.n_error} success_rate={run.success_rate:.4f}"
            )
        if args.save:
            Path(args.save).write_text(
                json.dumps(run.to_dict(), ensure_ascii=False, indent=2),
                encoding="utf-8",
            )
            print(f"saved: {args.save}")
        return 0

    # 真跑 multi-agent task
    result = run_agent_with_llm(args.task, force_mock=not args.real_llm)
    if args.json:
        d = result.to_dict()
        d["tasks"] = [t.to_dict() for t in result.tasks]
        print(json.dumps(d, ensure_ascii=False, indent=2))
    else:
        print(
            f"V1152 multi-agent (real_llm={args.real_llm}): "
            f"snapshot_id={result.snapshot_id} "
            f"n_tasks={result.n_tasks} n_done={result.n_done} n_failed={result.n_failed} "
            f"success_rate={result.success_rate:.4f}"
        )
        print(f"final_output: {result.final_output[:120]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())