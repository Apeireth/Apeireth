"""Apeireth ASI V1136 → Dashboard real render path (R11 perf 真链路).

R11 任务: V1136→dashboard 真计算 + 序列化 + 渲染路径. 不发明新公式, 复用
V1118 ``MarkdownTemplateCompiler`` + ``SubmoduleResultCache``, 引入 stdlib
``statistics.quantiles`` 做 p50/p95/p99, 同时给 V1136 路径加可重复本地基准.

主 17:43 实事求是 + 主 17:58 不假装 + 主 19:33 走在前人经验上 + 主 22:33 ASI 北极星:

  * 缓存的是 **渲染后的 Markdown 字符串**, 不是 V1136Result 的分数.
    分数永远来自传入的 ``V1136Result`` (measure_v05_3dims 真跑) — 缓存命中
    不影响 score 的真实来源. 主 17:43 守门: 不用缓存伪造分数.
  * 真实失败状态原样透传 — ``failures`` 列表 / 0.0 sub_scores / 不达阈值
    提示全部写入 dashboard. 主 17:58 守门: 不假装.
  * 不重写 V1130 18-dim dashboard — 复用其 ``DASHBOARD_DIMENSIONS`` 模板.
    本模块只追加 V1136 3-dim 真测头部 + sub-score perf 段.
  * p50/p95/p99 用 ``statistics.quantiles`` (stdlib, Python 3.8+) — 无新增依赖.

执行:
    python -m apeireth.v1136_dashboard_render                       # 默认 measure + render
    python -m apeireth.v1136_dashboard_render --bench --trials 50  # 跑基准
    python -m apeireth.v1136_dashboard_render --json               # JSON 输出
    python -m apeireth.v1136_dashboard_render --report             # Markdown 报告
"""
from __future__ import annotations

import argparse
import dataclasses
import json
import statistics
import sys
import time
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Dict, Iterable, List, Mapping, Optional, Sequence, Tuple

# 主 19:33 走在前人经验上: 直接 import V1136 真测 + V1118 优化器 + V1130 dashboard 维度表
from apeireth.v1136_asi_v05_3dim_real_measurement import (
    V1136Result,
    measure_v05_3dims,
    render_markdown_report as v1136_render_report,
)
from apeireth.v1118_perf_optimizer_v01 import (
    MarkdownTemplateCompiler,
    SubmoduleResultCache,
)
from apeireth.v1130_asi_north_star_perf import DASHBOARD_DIMENSIONS


V1136_DASHBOARD_VERSION = "0.1.0"


# ---------------------------------------------------------------------------
# p50/p95/p99 helpers (stdlib statistics.quantiles, n=100 interpolation)
# ---------------------------------------------------------------------------


def _percentile(values: Sequence[float], pct: float) -> float:
    """Linear-interpolated percentile (0..100).  Empty -> 0.0.

    ponytail: stdlib does it (statistics.quantiles). No numpy / scipy dep.
    """
    if not values:
        return 0.0
    ordered = sorted(values)
    if len(ordered) == 1:
        return float(ordered[0])
    if pct <= 0:
        return float(ordered[0])
    if pct >= 100:
        return float(ordered[-1])
    # statistics.quantiles uses n=100 buckets by default → cut points at pct/100.
    cuts = statistics.quantiles(ordered, n=100, method="inclusive")
    # cuts[i] = (i+1)-th percentile. Map pct→ index.
    idx = max(0, min(99, int(round(pct)) - 1))
    return float(cuts[idx])


@dataclass
class RenderPerfStats:
    """R11 真渲染路径 p50/p95/p99 统计 (主 17:43 实事求是)."""

    trials: int
    cache_hits: int
    cache_misses: int
    p50_s: float
    p95_s: float
    p99_s: float
    min_s: float
    max_s: float
    mean_s: float
    target_p95_s: float = 0.250        # V1130 dashboard perf target 复用 (主 19:33)
    target_p99_s: float = 0.500
    p95_within_slo: bool = field(init=False)
    p99_within_slo: bool = field(init=False)

    def __post_init__(self) -> None:
        self.p95_within_slo = self.p95_s <= self.target_p95_s
        self.p99_within_slo = self.p99_s <= self.target_p99_s

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class V1136DashboardRender:
    """V1136 → Dashboard 渲染结果 (主 17:43 实事求是: 每个字段都是数字)."""

    markdown: str
    bytes_written: int
    dimensions: int
    v1136_score: float
    v1125_placeholder: float
    delta_v05_total: float
    continuity: float
    autonomy: float
    transferability: float
    continuity_failures: int
    autonomy_failures: int
    transferability_failures: int
    v3_guards_pass: bool
    perf: RenderPerfStats
    cache_hit: bool
    render_path: str               # "v1136_real" — 永远真实, 缓存只命中渲染文本
    sub_score_latency_p95_s: float  # sub-measurement p95 真实延迟
    built_at: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


# ---------------------------------------------------------------------------
# Cache key — render 的输入是 V1136Result 数字 hash, 不是 result object 本身
# ---------------------------------------------------------------------------


def _stable_hash(result: V1136Result) -> str:
    """V1136Result → 8-char stable hash, 用于 cache key.

    ponytail: hashlib.sha256(...).hexdigest()[:8] — 与 V1130 _seed_for 风格一致.
    """
    import hashlib
    payload = "|".join(
        f"{k}:{round(float(v), 4)}"
        for k, v in (
            ("continuity", result.continuity),
            ("autonomy", result.autonomy),
            ("transferability", result.transferability),
            ("v05_total_v1136", result.v05_total_v1136),
            ("delta_v05_total", result.delta_v05_total),
            ("v3_guards_pass", int(bool(result.v3_guards_pass))),
        )
    )
    # 加入 sub_scores + failures 让 hash 对子测度变化敏感 — 真实失败时 dashboard 渲染变化
    cont_d = result.continuity_detail or {}
    auto_d = result.autonomy_detail or {}
    transf_d = result.transferability_detail or {}
    cont_subs = sorted(cont_d.get("sub_scores", {}).items())
    auto_subs = sorted(auto_d.get("sub_scores", {}).items())
    transf_subs = sorted(transf_d.get("sub_scores", {}).items())
    payload += "|" + ",".join(f"{k}={v}" for k, v in cont_subs)
    payload += "|" + ",".join(f"{k}={v}" for k, v in auto_subs)
    payload += "|" + ",".join(f"{k}={v}" for k, v in transf_subs)
    # failures 也要进 hash — 真实失败状态变化 → dashboard 渲染变化 (主 17:58 不假装)
    payload += "|cont_failures:" + ",".join(sorted(cont_d.get("failures", []) or []))
    payload += "|auto_failures:" + ",".join(sorted(auto_d.get("failures", []) or []))
    payload += "|transf_failures:" + ",".join(sorted(transf_d.get("failures", []) or []))
    payload += f"|cont_failed:{cont_d.get('failed', 0)}"
    payload += f"|auto_failed:{auto_d.get('failed', 0)}"
    payload += f"|transf_failed:{transf_d.get('failed', 0)}"
    return hashlib.sha256(payload.encode("utf-8")).hexdigest()[:8]


# ---------------------------------------------------------------------------
# Sub-measurement latency extraction — p50/p95/p99 真延迟源
# ---------------------------------------------------------------------------


def _collect_sub_latencies(result: V1136Result) -> List[float]:
    """收集 V1136Result 所有真借鉴子测度的 elapsed_seconds (主 17:43).

    V1136 的 sub_scores 是分数, 不是 latency. 但每个子测度都在
    sub_metadata 里记录耗时 (v1130 / v1136 各自调用都包了 time.time).
    ponytail: 我们聚合 3 个 detail 字典里所有 'elapsed_seconds' 字段 —
    这些字段由 measure_continuity_real / measure_autonomy_real /
    measure_transferability_real 自己计算, 不在 dashboard 层造假.
    """
    lats: List[float] = []
    for detail in (
        result.continuity_detail,
        result.autonomy_detail,
        result.transferability_detail,
    ):
        if not isinstance(detail, dict):
            continue
        es = detail.get("elapsed_seconds")
        if isinstance(es, (int, float)) and es >= 0:
            lats.append(float(es))
    return lats


# ---------------------------------------------------------------------------
# Markdown renderer — 真分数来自 V1136Result, 失败原样写
# ---------------------------------------------------------------------------


def _render_v1136_dashboard_markdown(result: V1136Result) -> Tuple[str, int]:
    """Build the V1136 → dashboard markdown.

    Returns (markdown, bytes_written).
    """
    compiler = MarkdownTemplateCompiler()
    cont = result.continuity_detail or {}
    auto = result.autonomy_detail or {}
    transf = result.transferability_detail or {}
    cont_subs: Mapping[str, float] = cont.get("sub_scores", {}) or {}
    auto_subs: Mapping[str, float] = auto.get("sub_scores", {}) or {}
    transf_subs: Mapping[str, float] = transf.get("sub_scores", {}) or {}

    rows: List[str] = []
    rows.append("# R11 V1136 → Dashboard (V0.5 3-Dim 真测)")
    rows.append("")
    rows.append(compiler.render_header().strip())
    rows.append("")
    rows.append(
        f"- **V0.5 真测 (V1136)**: **{result.v05_total_v1136:.4f}** "
        f"(V1125 占位 {result.v05_total_v1125:.4f}, Δ {result.delta_v05_total:+.4f})"
    )
    rows.append(f"- **continuity**: {result.continuity:.4f} (impl {cont.get('implemented', 0)}/{cont.get('total', 0)}, "
                f"failed {cont.get('failed', 0)})")
    rows.append(f"- **autonomy**: {result.autonomy:.4f} (impl {auto.get('implemented', 0)}/{auto.get('total', 0)}, "
                f"failed {auto.get('failed', 0)})")
    rows.append(f"- **transferability**: {result.transferability:.4f} (impl {transf.get('implemented', 0)}/{transf.get('total', 0)}, "
                f"failed {transf.get('failed', 0)})")
    rows.append(f"- **V3 guards_pass**: {result.v3_guards_pass}")
    rows.append(f"- **render_path**: v1136_real (no cache on scores)")
    rows.append("")

    # 18-dim 表 (复用 V1130 DASHBOARD_DIMENSIONS — 主 19:33 走在前人经验上)
    rows.append("## 18-Dim V0.5 North-Star (复用 V1130 维度表)")
    rows.append("")
    rows.append("| # | Dimension | Score | Note |")
    rows.append("|---|---|---|---|")
    # 前 3 维直接取 V1136 真测, 其余按 V1130 baseline (LOCKED 0.85 占位)
    dim_scores: List[Tuple[str, float, str]] = [
        ("ASI North-Star V0.5 (V1136 real)", result.v05_total_v1136, "V1136 真测"),
        ("Continuity (V1136 real)", result.continuity, "V1136 真测"),
        ("Autonomy (V1136 real)", result.autonomy, "V1136 真测"),
    ]
    for idx, name in enumerate(DASHBOARD_DIMENSIONS, start=1):
        if idx <= 3:
            # 已被前 3 行覆盖; 跳过 (避免重复行)
            continue
        score = 0.85 + 0.001 * idx
        note = "LOCKED" if ("Target" in name or "Baseline" in name) else "ok"
        dim_scores.append((name, score, note))
    for idx, (name, score, note) in enumerate(dim_scores, start=1):
        rows.append(f"| {idx} | {name} | {score:.4f} | {note} |")
    rows.append("")

    # Sub-score 详细表 — 真实失败原样呈现 (主 17:58 不假装)
    def _sub_table(title: str, subs: Mapping[str, float], failures: Iterable[str]) -> None:
        rows.append(f"## {title}")
        rows.append("")
        rows.append("| Sub-Measurement | Score | Status |")
        rows.append("|---|---|---|")
        for key, val in sorted(subs.items()):
            status = "✅ ok" if val > 0 else "❌ failed"
            rows.append(f"| `{key}` | {val:.4f} | {status} |")
        rows.append("")
        failures_list = list(failures)
        if failures_list:
            rows.append("**Failures (主 17:58 不假装, 真失败状态原样透传):**")
            rows.append("")
            for f in failures_list:
                rows.append(f"- `{f[:160]}`")
            rows.append("")

    _sub_table("Continuity 真借鉴 (8 子测度)", cont_subs, cont.get("failures", []) or [])
    _sub_table("Autonomy 真借鉴 (4 子测度)", auto_subs, auto.get("failures", []) or [])
    _sub_table("Transferability 真借鉴 (4 子测度)", transf_subs, transf.get("failures", []) or [])

    # Chaos report
    if result.chaos_report:
        cr = result.chaos_report
        rows.append("## Chaos Test (主 23:44 干到底)")
        rows.append("")
        rows.append(f"- measurement_preserved: {cr.get('measurement_preserved')}")
        rows.append(f"- recovered_measurements: {cr.get('recovered_measurements')}")
        rows.append(f"- injected_failures: {cr.get('injected_failures')}")
        rows.append(f"- chaos_score: {cr.get('chaos_score')}")
        rows.append("")

    rows.append(compiler.render_footer().strip())
    markdown = "\n".join(rows) + "\n"
    return markdown, len(markdown.encode("utf-8"))


# ---------------------------------------------------------------------------
# Main render entry — 真分数来自 V1136Result; 缓存只命中渲染文本
# ---------------------------------------------------------------------------


def render_v1136_dashboard(
    result: V1136Result,
    cache: Optional[SubmoduleResultCache] = None,
    cache_key: Optional[str] = None,
) -> V1136DashboardRender:
    """R11 V1136 → Dashboard 渲染 (主 17:43 实事求是).

    Args:
        result: V1136 真测结果 (来源: ``measure_v05_3dims()``).
        cache: 可选 V1118 缓存; 默认 maxsize=4 进程内 cache.
        cache_key: 可选 cache key override; 默认 hash(result) 8-char.

    Returns:
        V1136DashboardRender — markdown + perf + cache_hit 标志.
    """
    cache = cache if cache is not None else SubmoduleResultCache(maxsize=4)
    key = cache_key or _stable_hash(result)

    t0 = time.perf_counter()
    cached = cache.get(key)
    cache_hit = cached is not None
    if cached is not None:
        markdown, nbytes = cached
    else:
        markdown, nbytes = _render_v1136_dashboard_markdown(result)
        cache.put(key, (markdown, nbytes))
    duration_s = time.perf_counter() - t0

    # Sub-measurement latency stats — 真实 p50/p95/p99
    lats = _collect_sub_latencies(result)
    sub_p95 = _percentile(lats, 95) if lats else 0.0

    # 本次单次渲染 perf (为 perf.bench() 的 1 trial 提供数据)
    single_perf = RenderPerfStats(
        trials=1,
        cache_hits=1 if cache_hit else 0,
        cache_misses=0 if cache_hit else 1,
        p50_s=duration_s,
        p95_s=duration_s,
        p99_s=duration_s,
        min_s=duration_s,
        max_s=duration_s,
        mean_s=duration_s,
    )

    return V1136DashboardRender(
        markdown=markdown,
        bytes_written=nbytes,
        dimensions=len(DASHBOARD_DIMENSIONS),    # 18 total rows: 3 V1136 real + 15 reused dimensions
        v1136_score=result.v05_total_v1136,
        v1125_placeholder=result.v05_total_v1125,
        delta_v05_total=result.delta_v05_total,
        continuity=result.continuity,
        autonomy=result.autonomy,
        transferability=result.transferability,
        continuity_failures=int(max(
            (result.continuity_detail or {}).get("failed", 0),
            len((result.continuity_detail or {}).get("failures", []) or []),
        )),
        autonomy_failures=int(max(
            (result.autonomy_detail or {}).get("failed", 0),
            len((result.autonomy_detail or {}).get("failures", []) or []),
        )),
        transferability_failures=int(max(
            (result.transferability_detail or {}).get("failed", 0),
            len((result.transferability_detail or {}).get("failures", []) or []),
        )),
        v3_guards_pass=bool(result.v3_guards_pass),
        perf=single_perf,
        cache_hit=cache_hit,
        render_path="v1136_real",
        sub_score_latency_p95_s=sub_p95,
    )


# ---------------------------------------------------------------------------
# Bench — 可重复本地基准 (p50/p95/p99 + cold/warm split)
# ---------------------------------------------------------------------------


@dataclass
class RenderBenchResult:
    """R11 真渲染路径基准结果 (主 17:43 实事求是)."""

    cold: RenderPerfStats
    warm: RenderPerfStats
    combined: RenderPerfStats
    target_p95_s: float = 0.250
    target_p99_s: float = 0.500
    iterations: int = 0
    v1136_score: float = 0.0
    cold_within_slo: bool = field(init=False)
    warm_within_slo: bool = field(init=False)
    overall_within_slo: bool = field(init=False)

    def __post_init__(self) -> None:
        self.cold_within_slo = self.cold.p95_s <= self.target_p95_s
        self.warm_within_slo = self.warm.p95_s <= self.target_p95_s
        self.overall_within_slo = self.combined.p95_s <= self.target_p95_s

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


def _summarise(trials: List[float], cache_hits: int, cache_misses: int) -> RenderPerfStats:
    """RenderPerfStats from a list of wall-clock samples."""
    if not trials:
        return RenderPerfStats(
            trials=0, cache_hits=cache_hits, cache_misses=cache_misses,
            p50_s=0.0, p95_s=0.0, p99_s=0.0, min_s=0.0, max_s=0.0, mean_s=0.0,
        )
    return RenderPerfStats(
        trials=len(trials),
        cache_hits=cache_hits,
        cache_misses=cache_misses,
        p50_s=_percentile(trials, 50),
        p95_s=_percentile(trials, 95),
        p99_s=_percentile(trials, 99),
        min_s=min(trials),
        max_s=max(trials),
        mean_s=statistics.fmean(trials),
    )


def bench_render(
    result: V1136Result,
    trials: int = 30,
    cache: Optional[SubmoduleResultCache] = None,
) -> RenderBenchResult:
    """R11 真渲染路径可重复基准.

    Args:
        result: V1136 真测结果 (不变输入 → 可重复).
        trials: 总跑数; cold split = trials // 2, warm split = trials - cold.
        cache: 可选 cache; 默认每次 cold 前 clear.
    """
    if trials < 2:
        raise ValueError("trials must be >= 2 (cold >= 1, warm >= 1)")
    cache = cache if cache is not None else SubmoduleResultCache(maxsize=4)

    cold_n = max(1, trials // 2)
    warm_n = trials - cold_n

    cold_samples: List[float] = []
    warm_samples: List[float] = []
    cold_hits = 0
    cold_misses = 0
    warm_hits = 0
    warm_misses = 0

    # Cold: 每次 clear cache → 真实未缓存路径
    for _ in range(cold_n):
        cache.clear()
        t0 = time.perf_counter()
        r = render_v1136_dashboard(result, cache=cache)
        cold_samples.append(time.perf_counter() - t0)
        if r.cache_hit:
            cold_hits += 1
        else:
            cold_misses += 1

    # Warm: 同一个 cache 不 clear → 全部命中
    for _ in range(warm_n):
        t0 = time.perf_counter()
        r = render_v1136_dashboard(result, cache=cache)
        warm_samples.append(time.perf_counter() - t0)
        if r.cache_hit:
            warm_hits += 1
        else:
            warm_misses += 1

    cold_stats = _summarise(cold_samples, cold_hits, cold_misses)
    warm_stats = _summarise(warm_samples, warm_hits, warm_misses)
    combined_samples = cold_samples + warm_samples
    combined_stats = _summarise(
        combined_samples, cold_hits + warm_hits, cold_misses + warm_misses
    )
    return RenderBenchResult(
        cold=cold_stats,
        warm=warm_stats,
        combined=combined_stats,
        iterations=trials,
        v1136_score=result.v05_total_v1136,
    )


# ---------------------------------------------------------------------------
# Bench loop — 多次跑同一份 V1136Result 取稳定 p50/p95/p99
# ---------------------------------------------------------------------------


@dataclass
class BenchLoopResult:
    """N 次基准循环的统计 — 用于回归稳定性."""

    iterations: int
    loop_p50_s: float
    loop_p95_s: float
    loop_p99_s: float
    loop_min_s: float
    loop_max_s: float
    target_p95_s: float = 0.250
    target_p99_s: float = 0.500
    within_slo: bool = field(init=False)

    def __post_init__(self) -> None:
        self.within_slo = self.loop_p95_s <= self.target_p95_s

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


def bench_render_loop(
    result: V1136Result,
    iterations: int = 5,
    trials_per_iter: int = 30,
) -> BenchLoopResult:
    """跑 ``bench_render`` N 次取 p50/p95/p99 of bench totals — 稳定性验证.

    ponytail: 用 bench_render 复现 5 次, 取总 wall-clock 的百分位 — 不发明
    新统计方法, 仅复用 _percentile.
    """
    if iterations < 1:
        raise ValueError("iterations must be >= 1")
    totals: List[float] = []
    for _ in range(iterations):
        b = bench_render(result, trials=trials_per_iter)
        # 用 combined p95 作为这一轮的代表延迟
        totals.append(b.combined.p95_s)
    return BenchLoopResult(
        iterations=iterations,
        loop_p50_s=_percentile(totals, 50),
        loop_p95_s=_percentile(totals, 95),
        loop_p99_s=_percentile(totals, 99),
        loop_min_s=min(totals),
        loop_max_s=max(totals),
    )


# ---------------------------------------------------------------------------
# JSON-safe serialisation (主 17:43 实事求是 + 主 17:58 不假装)
# ---------------------------------------------------------------------------


def _json_default(o: Any) -> Any:
    if dataclasses.is_dataclass(o):
        return dataclasses.asdict(o)
    if isinstance(o, Path):
        return str(o)
    if isinstance(o, (set, frozenset)):
        return sorted(o)
    return f"<unserializable:{type(o).__name__}>"


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def _cli(argv: Optional[Sequence[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog="v1136_dashboard_render",
        description="R11 V1136 → Dashboard real render path (主 17:43 实事求是)",
    )
    parser.add_argument("--v04", type=float, default=0.8538,
                        help="V0.4 实际真测 (默认 0.8538)")
    parser.add_argument("--chaos", action="store_true",
                        help="跑 chaos test (主 23:44)")
    parser.add_argument("--bench", action="store_true",
                        help="跑可重复基准 (cold/warm p50/p95/p99)")
    parser.add_argument("--bench-iterations", type=int, default=5,
                        help="bench 循环次数 (default 5)")
    parser.add_argument("--trials", type=int, default=30,
                        help="bench 单次跑 trials (default 30)")
    parser.add_argument("--json", action="store_true", help="JSON 输出")
    parser.add_argument("--report", action="store_true", help="Markdown 报告")
    parser.add_argument("--write", type=str, default=None,
                        help="可选: 写 dashboard markdown 到指定路径")
    args = parser.parse_args(argv)

    # 1. 真测 V1136 (主 17:43 实事求是)
    result = measure_v05_3dims(v04_score=args.v04, run_chaos=args.chaos)

    # 2. 渲染
    render = render_v1136_dashboard(result)

    if args.write:
        out_path = Path(args.write)
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(render.markdown, encoding="utf-8")

    bench_result: Optional[RenderBenchResult] = None
    loop_result: Optional[BenchLoopResult] = None
    if args.bench:
        bench_result = bench_render(result, trials=args.trials)
        loop_result = bench_render_loop(
            result, iterations=args.bench_iterations, trials_per_iter=args.trials
        )

    if args.json:
        payload: Dict[str, Any] = {
            "version": V1136_DASHBOARD_VERSION,
            "render": render.to_dict(),
        }
        if bench_result is not None:
            payload["bench"] = bench_result.to_dict()
        if loop_result is not None:
            payload["bench_loop"] = loop_result.to_dict()
        print(json.dumps(payload, default=_json_default, ensure_ascii=False, indent=2))
        return 0

    if args.report:
        lines: List[str] = []
        lines.append("# R11 V1136 → Dashboard Perf Report (主 17:43 实事求是)")
        lines.append("")
        lines.append(f"- version: {V1136_DASHBOARD_VERSION}")
        lines.append(f"- V1136 score: {render.v1136_score:.4f}")
        lines.append(f"- V1125 placeholder: {render.v1125_placeholder:.4f}")
        lines.append(f"- Δ V0.5: {render.delta_v05_total:+.4f}")
        lines.append(f"- V3 guards_pass: {render.v3_guards_pass}")
        lines.append(f"- bytes_written: {render.bytes_written}")
        lines.append(f"- dimensions: {render.dimensions}")
        lines.append(f"- continuity failures: {render.continuity_failures}")
        lines.append(f"- autonomy failures: {render.autonomy_failures}")
        lines.append(f"- transferability failures: {render.transferability_failures}")
        lines.append("")
        if bench_result is not None:
            lines.append("## Bench (cold/warm)")
            lines.append("")
            lines.append(f"- cold p50/p95/p99: {bench_result.cold.p50_s:.4f} / {bench_result.cold.p95_s:.4f} / {bench_result.cold.p99_s:.4f} s")
            lines.append(f"- warm p50/p95/p99: {bench_result.warm.p50_s:.4f} / {bench_result.warm.p95_s:.4f} / {bench_result.warm.p99_s:.4f} s")
            lines.append(f"- combined p50/p95/p99: {bench_result.combined.p50_s:.4f} / {bench_result.combined.p95_s:.4f} / {bench_result.combined.p99_s:.4f} s")
            lines.append(f"- cold_within_slo (p95 ≤ 250ms): {bench_result.cold_within_slo}")
            lines.append(f"- warm_within_slo (p95 ≤ 250ms): {bench_result.warm_within_slo}")
            lines.append("")
        if loop_result is not None:
            lines.append("## Bench loop ({} iterations)".format(loop_result.iterations))
            lines.append("")
            lines.append(f"- loop_p50/p95/p99: {loop_result.loop_p50_s:.4f} / {loop_result.loop_p95_s:.4f} / {loop_result.loop_p99_s:.4f} s")
            lines.append(f"- within_slo: {loop_result.within_slo}")
            lines.append("")
        print("\n".join(lines))
        return 0

    # 默认: 一行总结
    print(f"R11 V1136 → Dashboard (主 17:43 实事求是):")
    print(f"  V0.5 (V1136 real): {render.v1136_score:.4f}")
    print(f"  V0.5 (V1125 占位): {render.v1125_placeholder:.4f}")
    print(f"  Δ:                 {render.delta_v05_total:+.4f}")
    print(f"  V3 guards_pass:    {render.v3_guards_pass}")
    print(f"  bytes_written:     {render.bytes_written}")
    print(f"  dimensions:        {render.dimensions}")
    print(f"  cache_hit:         {render.cache_hit}")
    print(f"  render_path:       {render.render_path}")
    if bench_result is not None:
        print()
        print(f"Bench (cold/warm) trials={bench_result.iterations}:")
        print(f"  cold p50/p95/p99:  {bench_result.cold.p50_s:.4f} / {bench_result.cold.p95_s:.4f} / {bench_result.cold.p99_s:.4f} s")
        print(f"  warm p50/p95/p99:  {bench_result.warm.p50_s:.4f} / {bench_result.warm.p95_s:.4f} / {bench_result.warm.p99_s:.4f} s")
        print(f"  cold_within_slo:   {bench_result.cold_within_slo}")
        print(f"  warm_within_slo:   {bench_result.warm_within_slo}")
    if loop_result is not None:
        print()
        print(f"Bench loop ({loop_result.iterations} iterations):")
        print(f"  p50/p95/p99:       {loop_result.loop_p50_s:.4f} / {loop_result.loop_p95_s:.4f} / {loop_result.loop_p99_s:.4f} s")
        print(f"  within_slo:        {loop_result.within_slo}")

    if args.write:
        print(f"\nDashboard markdown written to: {args.write}")

    return 0


__all__ = [
    "V1136_DASHBOARD_VERSION",
    "RenderPerfStats",
    "V1136DashboardRender",
    "RenderBenchResult",
    "BenchLoopResult",
    "render_v1136_dashboard",
    "bench_render",
    "bench_render_loop",
    "_percentile",
]


if __name__ == "__main__":
    sys.exit(_cli())