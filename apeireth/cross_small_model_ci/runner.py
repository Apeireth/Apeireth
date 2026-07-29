"""Cross-small-model CI: runner (R9-DevOps / R9-DEV-001).

主 19:33 走在前人经验上: 借鉴 pytest 2008 parametrize + GitHub Actions matrix 2020
+ EleutherAI LM-Eval 2021 跨模型评测.
主 00:56 任何人都能接手: `run_ci()` 一行 = 全模型 × HQB 4 维.
"""
from __future__ import annotations

import time
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Sequence

from .harness import HarnessResult, HQBHarness
from .models import DEFAULT_REGISTRY, FixtureAdapter, ModelAdapter, ModelRegistry


@dataclass
class CIRunner:
    """CI 编排器: 拿 registry + harness → 逐个模型跑 → 收集结果."""

    registry: ModelRegistry
    harness: HQBHarness
    skip_unavailable: bool = True   # 跳过真模型 (7B 没下载), fixture 必跑

    def __init__(self, registry: Optional[ModelRegistry] = None,
                 harness: Optional[HQBHarness] = None,
                 skip_unavailable: bool = True):
        self.registry = registry or DEFAULT_REGISTRY
        self.harness = harness or HQBHarness()
        self.skip_unavailable = skip_unavailable

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


def run_one_model(adapter: ModelAdapter, harness: Optional[HQBHarness] = None) -> HarnessResult:
    """单模型跑 (主 00:56 任何人都能接手)."""
    h = harness or HQBHarness()
    return h.run(adapter)


def run_ci(registry: Optional[ModelRegistry] = None,
           harness: Optional[HQBHarness] = None,
           only_families: Optional[Sequence[str]] = None,
           skip_unavailable: bool = True) -> List[HarnessResult]:
    """CI 一行入口 (主 00:56 任何人都能接手)."""
    runner = CIRunner(registry=registry, harness=harness, skip_unavailable=skip_unavailable)
    return runner.run(only_families=only_families)


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
