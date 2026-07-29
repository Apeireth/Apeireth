"""Cross-small-model CI: runner + real-model attempt (R9-DevOps / R9-DEV-002).

主 19:33 走在前人经验上: 借鉴 pytest 2008 parametrize + GitHub Actions matrix 2020
+ EleutherAI LM-Eval 2021 跨模型评测 + Ollama 2023 本地 LLM 简化.
主 00:56 任何人都能接手: `run_ci()` 一行 = 全模型 × HQB 4 维.
主 17:58+20:46 不假装 (W3 增强): 真模型加载失败 → 显式记录, 不混入 PASS 数据.
主 13:31 大胆激进 (W3 增强): 至少 1 个真模型真接入尝试.
"""
from __future__ import annotations

import os
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence

from .harness import HarnessResult, HQBHarness
from .models import DEFAULT_REGISTRY, FixtureAdapter, ModelAdapter, ModelRegistry


# 真模型环境变量约定 (主 00:56 任何人都能接手: 通过 env 显式注入路径, 不 hardcode)
# key = family (qwen/llama/hermes/gemma), value = env var name
REAL_MODEL_ENV = {
    "qwen": "APEIRETH_QWEN35_PATH",
    "llama": "APEIRETH_LLAMA31_PATH",
    "hermes": "APEIRETH_HERMES_PATH",
    "gemma": "APEIRETH_GEMMA4_PATH",
}


@dataclass
class CIRunner:
    """CI 编排器: 拿 registry + harness → 逐个模型跑 → 收集结果."""

    registry: ModelRegistry
    harness: HQBHarness
    skip_unavailable: bool = True   # 跳过真模型 (7B 没下载), fixture 必跑
    honor_real_model_env: bool = True   # W3 增强: 从 env 注入 local_path, 真尝试加载

    def __init__(self, registry: Optional[ModelRegistry] = None,
                 harness: Optional[HQBHarness] = None,
                 skip_unavailable: bool = True,
                 honor_real_model_env: bool = True):
        self.registry = registry or DEFAULT_REGISTRY
        self.harness = harness or HQBHarness()
        self.skip_unavailable = skip_unavailable
        self.honor_real_model_env = honor_real_model_env

    def select_adapters(self, only_families: Optional[Sequence[str]] = None) -> List[ModelAdapter]:
        """选 adapter: 默认含 fixture; 真模型若 available 也含."""
        out: List[ModelAdapter] = []
        for a in self.registry.all():
            if only_families and a.family not in only_families:
                continue
            if a.family == "fixture":
                out.append(a)
                continue
            if a.is_available():
                out.append(a)
            elif not self.skip_unavailable:
                out.append(a)
        return out

    def run(self, only_families: Optional[Sequence[str]] = None) -> List[HarnessResult]:
        """真跑全部选中 adapter. 异常 adapter 也返回 (passed=False, error=...)."""
        results: List[HarnessResult] = []
        for a in self.select_adapters(only_families):
            try:
                r = self.harness.run(a)
            except Exception as e:
                r = HarnessResult(
                    model_name=a.name,
                    family=a.family,
                    available=a.is_available(),
                    passed=False,
                    error=f"runner exception: {e!r}",
                )
            results.append(r)
        return results

    # W3 增强: 真模型 best-effort 接入
    def attempt_real_model(self, family: str) -> HarnessResult:
        """W3 增强: 真尝试加载 1 个真模型 (主 13:31 大胆激进 + 主 17:58 不假装).

        流程:
          1. 从 env 拿 local_path; 若无 → 立刻返回 unavailable (不假装)
          2. 用对应 adapter 加载; 失败 → HarnessResult(available=False, error=...)
          3. 加载成功 → 跑 HQB 4 维
        """
        if not self.honor_real_model_env:
            return HarnessResult(
                model_name=f"real-{family}",
                family=family,
                available=False,
                passed=False,
                error="honor_real_model_env disabled",
            )
        env_key = REAL_MODEL_ENV.get(family)
        if env_key is None:
            return HarnessResult(
                model_name=f"real-{family}",
                family=family,
                available=False,
                passed=False,
                error=f"unknown family {family!r}",
            )
        local_path = os.environ.get(env_key)
        if not local_path:
            return HarnessResult(
                model_name=f"real-{family}",
                family=family,
                available=False,
                passed=False,
                error=f"env {env_key} not set (主 17:58 不假装: 未提供 local_path → 跳过真模型)",
            )
        # 找到对应 adapter 类
        adapter = self._make_real_adapter(family, local_path)
        if adapter is None:
            return HarnessResult(
                model_name=f"real-{family}",
                family=family,
                available=False,
                passed=False,
                error=f"no adapter registered for family={family!r}",
            )
        if not Path(local_path).exists():
            return HarnessResult(
                model_name=adapter.name,
                family=family,
                available=False,
                passed=False,
                error=f"local_path {local_path!r} does not exist (主 17:58 不假装)",
            )
        # 真跑 HQB
        return self.harness.run(adapter)

    def _make_real_adapter(self, family: str, local_path: str) -> Optional[ModelAdapter]:
        """构造真模型 adapter. 走 registry 的类 (避免硬编码)."""
        for a in self.registry.all():
            if a.family == family and not isinstance(a, FixtureAdapter):
                # 用同类的实例 + 注入 local_path
                try:
                    new_a = type(a)(local_path=local_path)  # type: ignore[call-arg]
                except TypeError:
                    new_a = type(a)()  # type: ignore[call-arg]
                new_a.local_path = local_path
                return new_a
        return None


def run_one_model(adapter: ModelAdapter, harness: Optional[HQBHarness] = None) -> HarnessResult:
    """单模型跑 (主 00:56 任何人都能接手)."""
    h = harness or HQBHarness()
    return h.run(adapter)


def run_ci(registry: Optional[ModelRegistry] = None,
           harness: Optional[HQBHarness] = None,
           only_families: Optional[Sequence[str]] = None,
           skip_unavailable: bool = True,
           include_real_model_attempts: bool = False,
           real_model_families: Optional[Sequence[str]] = None) -> List[HarnessResult]:
    """CI 一行入口 (主 00:56 任何人都能接手 + W3 增强: 可选真模型接入).

    include_real_model_attempts=True → 额外跑 1+ 真模型 attempt, 结果混入 (unavailable 也返回).
    """
    runner = CIRunner(registry=registry, harness=harness, skip_unavailable=skip_unavailable)
    results = runner.run(only_families=only_families)
    if include_real_model_attempts:
        # 主 13:31: 默认尝试 Qwen 3.5-7B + Llama 3.1-8B 两个真模型
        families = list(real_model_families or ["qwen", "llama"])
        for f in families:
            results.append(runner.attempt_real_model(f))
    return results


def summarize(results: Sequence[HarnessResult]) -> Dict[str, Any]:
    """聚合 CI 结果."""
    if not results:
        return {"n_models": 0, "n_passed": 0, "n_available": 0, "avg_subscore": 0.0, "all_pass": False}
    n_models = len(results)
    n_passed = sum(1 for r in results if r.passed)
    n_available = sum(1 for r in results if r.available)
    avg_sub = sum(r.subscore for r in results) / n_models
    return {
        "n_models": n_models,
        "n_passed": n_passed,
        "n_available": n_available,
        "avg_subscore": round(avg_sub, 4),
        "all_pass": n_passed >= 1 and n_passed == n_models,
    }
