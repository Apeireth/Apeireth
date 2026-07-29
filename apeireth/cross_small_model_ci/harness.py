"""Cross-small-model CI: HQB 4-dim harness (R9-DevOps / R9-DEV-001).

主 18:52 HARNESS.md §2.3 HQB 4 维真借鉴:
  - SC 自洽性 (Self-Consistency):  同一 task N 次 → score 方差
  - NR 抗噪性 (Noise-Resistance):  同 prompt 不同扰动 → score 稳定性
  - EV 可演化性 (Evolvability):     prev → next score lift
  - CDT 跨域迁移 (Cross-Domain):    跨 4 域 → 跨域均值

主 17:43 实事求是: 真测真记, 不假装.
主 19:33 走在前人经验上: 借鉴 V36 HQB benchmark + V160 HQB 4 dims + V1085 HQB core.
"""
from __future__ import annotations

import statistics
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Sequence

from .models import ModelAdapter, ModelResult
from .tasks import DEFAULT_TASKS, HQBTask, TaskDomain, get_tasks_by_domain, nr_variants


# ---------------------------------------------------------------------------
# 配置
# ---------------------------------------------------------------------------
@dataclass
class SCConfig:
    n_trials: int = 3             # 自洽性重复次数
    sample_n: int = 2             # 抽样 task 数


@dataclass
class NRConfig:
    n_variants: int = 5           # 扰动版数
    sample_n: int = 2             # 抽样 task 数


@dataclass
class EVConfig:
    # EV 借用同 task 上 SC 的 first-run score 作为 "prev", last-run 作为 "next"
    # 真生产: 应接入历史 score, 这里 CI 用同 task 多轮近似
    sample_n: int = 2


@dataclass
class CDTConfig:
    domains: List[TaskDomain] = field(default_factory=lambda: [
        TaskDomain.CODE, TaskDomain.MATH, TaskDomain.REASONING, TaskDomain.CREATIVE,
    ])
    sample_per_domain: int = 1   # 每域抽样数 (够用即可)


# ---------------------------------------------------------------------------
# 单模型 HQB 4 维结果
# ---------------------------------------------------------------------------
@dataclass
class HarnessResult:
    model_name: str
    family: str
    available: bool
    sc: float = 0.0
    nr: float = 0.0
    ev: float = 0.0
    cdt: float = 0.0
    cdt_per_domain: Dict[str, float] = field(default_factory=dict)
    subscore: float = 0.0
    passed: bool = False
    error: Optional[str] = None
    elapsed_sec: float = 0.0
    n_inferences: int = 0
    detail: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "model_name": self.model_name,
            "family": self.family,
            "available": self.available,
            "sc": round(self.sc, 4),
            "nr": round(self.nr, 4),
            "ev": round(self.ev, 4),
            "cdt": round(self.cdt, 4),
            "cdt_per_domain": {k: round(v, 4) for k, v in self.cdt_per_domain.items()},
            "subscore": round(self.subscore, 4),
            "passed": self.passed,
            "error": self.error,
            "elapsed_sec": round(self.elapsed_sec, 3),
            "n_inferences": self.n_inferences,
            "detail": self.detail,
        }


# ---------------------------------------------------------------------------
# 4 维单测
# ---------------------------------------------------------------------------
def measure_sc(adapter: ModelAdapter, tasks: Sequence[HQBTask], cfg: SCConfig) -> Dict[str, Any]:
    """SC 自洽性: 同 task N 次 score → 1 - variance/mean² (借鉴 V160 measure_sc)."""
    sample = tasks[: max(1, cfg.sample_n)]
    per_task: Dict[str, float] = {}
    for t in sample:
        scores: List[float] = []
        for _ in range(cfg.n_trials):
            r = adapter.infer(t.prompt)
            if r.ok:
                scores.append(adapter.score(t.prompt, r.output))
        if not scores:
            per_task[t.task_id] = 0.0
            continue
        mean = sum(scores) / len(scores)
        if abs(mean) < 1e-9:
            per_task[t.task_id] = 1.0 if all(s == scores[0] for s in scores) else 0.0
        else:
            var = sum((s - mean) ** 2 for s in scores) / len(scores)
            per_task[t.task_id] = max(0.0, 1.0 - var / (mean ** 2 + 1e-9))
    overall = sum(per_task.values()) / max(1, len(per_task))
    return {"sc": overall, "per_task": per_task, "n_inferences": sum(cfg.n_trials for _ in sample)}


def measure_nr(adapter: ModelAdapter, tasks: Sequence[HQBTask], cfg: NRConfig) -> Dict[str, Any]:
    """NR 抗噪性: 同 prompt 不同扰动 → 1 - 平均相对差 (借鉴 V160 measure_nr)."""
    sample = tasks[: max(1, cfg.sample_n)]
    per_task: Dict[str, float] = {}
    total_inf = 0
    for t in sample:
        variants = nr_variants(t.prompt)[: cfg.n_variants]
        results = [adapter.infer(v) for v in variants]
        total_inf += len(results)
        scores = [adapter.score(t.prompt, r.output) for r in results if r.ok]
        if len(scores) < 2:
            per_task[t.task_id] = 0.0
            continue
        baseline = scores[0]
        if abs(baseline) < 1e-9:
            per_task[t.task_id] = 1.0 if all(abs(s - baseline) < 1e-9 for s in scores) else 0.0
            continue
        diffs = [abs(s - baseline) / abs(baseline) for s in scores[1:]]
        avg_diff = sum(diffs) / len(diffs)
        per_task[t.task_id] = max(0.0, 1.0 - avg_diff)
    overall = sum(per_task.values()) / max(1, len(per_task))
    return {"nr": overall, "per_task": per_task, "n_inferences": total_inf}


def measure_ev(adapter: ModelAdapter, tasks: Sequence[HQBTask], cfg: EVConfig) -> Dict[str, Any]:
    """EV 可演化性: 借用同 task 上 2 次 (prev → next) score lift (借鉴 V160 measure_ev)."""
    sample = tasks[: max(1, cfg.sample_n)]
    per_task: Dict[str, float] = {}
    total_inf = 0
    for t in sample:
        r1 = adapter.infer(t.prompt)
        r2 = adapter.infer(t.prompt)
        total_inf += 2
        if not (r1.ok and r2.ok):
            per_task[t.task_id] = 0.0
            continue
        s1 = adapter.score(t.prompt, r1.output)
        s2 = adapter.score(t.prompt, r2.output)
        if abs(s1) < 1e-9:
            per_task[t.task_id] = 0.0
            continue
        delta = (s2 - s1) / abs(s1)
        per_task[t.task_id] = max(0.0, min(1.0, 0.5 + delta))
    overall = sum(per_task.values()) / max(1, len(per_task))
    return {"ev": overall, "per_task": per_task, "n_inferences": total_inf}


def measure_cdt(adapter: ModelAdapter, cfg: CDTConfig) -> Dict[str, Any]:
    """CDT 跨域迁移: 跨 4 域 → 每域 1 task → 跨域均值 (借鉴 V160 measure_cdt)."""
    per_domain: Dict[str, float] = {}
    total_inf = 0
    for d in cfg.domains:
        tasks = get_tasks_by_domain(d)[: cfg.sample_per_domain]
        if not tasks:
            per_domain[d.value] = 0.0
            continue
        scores: List[float] = []
        for t in tasks:
            r = adapter.infer(t.prompt)
            total_inf += 1
            if r.ok:
                scores.append(adapter.score(t.prompt, r.output))
        per_domain[d.value] = (sum(scores) / len(scores)) if scores else 0.0
    overall = sum(per_domain.values()) / max(1, len(per_domain))
    return {"cdt": overall, "per_domain": per_domain, "n_inferences": total_inf}


# ---------------------------------------------------------------------------
# Harness 总入口
# ---------------------------------------------------------------------------
class HQBHarness:
    """HQB 4 维 harness (主 18:52 + HARNESS.md §2.3 真借鉴)."""

    PASS_THRESHOLD = 0.50  # 主 17:43 实事求是: ≥0.5 即算 PASS, 不假装完美

    def __init__(self, sc_cfg: Optional[SCConfig] = None,
                 nr_cfg: Optional[NRConfig] = None,
                 ev_cfg: Optional[EVConfig] = None,
                 cdt_cfg: Optional[CDTConfig] = None,
                 pass_threshold: float = PASS_THRESHOLD):
        self.sc_cfg = sc_cfg or SCConfig()
        self.nr_cfg = nr_cfg or NRConfig()
        self.ev_cfg = ev_cfg or EVConfig()
        self.cdt_cfg = cdt_cfg or CDTConfig()
        self.pass_threshold = pass_threshold

    def run(self, adapter: ModelAdapter) -> HarnessResult:
        """真跑 4 维 + 聚合 subscore + 判定 pass/fail."""
        import time
        t0 = time.time()
        if not adapter.is_available():
            return HarnessResult(
                model_name=adapter.name,
                family=adapter.family,
                available=False,
                passed=False,
                error=f"adapter not available (local_path={adapter.local_path!r})",
                elapsed_sec=time.time() - t0,
            )
        try:
            adapter.load()
        except Exception as e:
            return HarnessResult(
                model_name=adapter.name,
                family=adapter.family,
                available=True,
                passed=False,
                error=f"load failed: {e!r}",
                elapsed_sec=time.time() - t0,
            )

        tasks = list(DEFAULT_TASKS)
        sc = measure_sc(adapter, tasks, self.sc_cfg)
        nr = measure_nr(adapter, tasks, self.nr_cfg)
        ev = measure_ev(adapter, tasks, self.ev_cfg)
        cdt = measure_cdt(adapter, self.cdt_cfg)
        n_inf = sc["n_inferences"] + nr["n_inferences"] + ev["n_inferences"] + cdt["n_inferences"]
        subscore = (sc["sc"] + nr["nr"] + ev["ev"] + cdt["cdt"]) / 4.0
        passed = subscore >= self.pass_threshold
        return HarnessResult(
            model_name=adapter.name,
            family=adapter.family,
            available=True,
            sc=sc["sc"],
            nr=nr["nr"],
            ev=ev["ev"],
            cdt=cdt["cdt"],
            cdt_per_domain=cdt["per_domain"],
            subscore=subscore,
            passed=passed,
            elapsed_sec=time.time() - t0,
            n_inferences=n_inf,
            detail={
                "sc_per_task": sc["per_task"],
                "nr_per_task": nr["per_task"],
                "ev_per_task": ev["per_task"],
                "cdt_per_domain": cdt["per_domain"],
            },
        )
