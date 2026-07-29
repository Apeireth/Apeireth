"""V1118 true performance optimizers for the V1074 production runner.

The module implements five independently switchable optimizers:

1. :class:`LazyImporter` defers the V1048/V1073 dependency chain and binds it once.
2. :class:`SnapshotCompressor` writes semantically identical compact JSON.
3. :class:`ParallelDimensionEvaluator` evaluates independent project dimensions in a
   persistent two-process pool.
4. :class:`SubmoduleResultCache` is a bounded, state-keyed 32-entry true LRU cache.
5. :class:`MarkdownTemplateCompiler` precompiles/caches static Markdown sections.

The orchestrator patches runner *instances* only.  It never replaces global
``subprocess.run`` and :meth:`V1118Optimizers.unwrap` restores every method.

Truth guard (主 17:43): the fast V1048 path is used only when its three injected
scores are formula-equivalent to the original measurements.  In particular, the
pytest collection subprocess is skipped only when the static test-function count is
at least 2,000, where both measurements are provably clamped to ``1.0``.  Otherwise
V1074's original measurement is called.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib
import json
import multiprocessing
import os
import re
import statistics
import subprocess
import sys
import threading
import time
from collections import OrderedDict
from concurrent.futures import ProcessPoolExecutor
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any, Callable, Dict, Hashable, Iterable, List, Mapping, Optional, Sequence, Tuple

V1118_VERSION = "0.1.0"
V1118_LRU_MAXSIZE = 32
V1118_PARALLEL_WORKERS = 2
V1074_REFERENCE_BASELINE_S = 3.05
V1074_TARGET_S = 2.50
V1074_MIN_SAVINGS_PCT = 20.0

_TEST_DEF_RE = re.compile(rb"def\s+test_")


# ---------------------------------------------------------------------------
# Shared evidence types and project metric workers (module-level = picklable)
# ---------------------------------------------------------------------------

@dataclass(frozen=True)
class MicroBenchmark:
    """A small, serializable before/after measurement."""

    name: str
    baseline_s: float
    optimized_s: float
    speedup: float
    savings_pct: float
    semantics_equal: bool
    trials: int
    notes: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class DimensionJob:
    """Picklable project-metric job for the two-worker process pool."""

    name: str
    operation: str
    project_dir: str


@dataclass(frozen=True)
class ProjectMetrics:
    modules: int
    tests: int
    commits: int

    def to_dict(self) -> Dict[str, int]:
        return {"modules": self.modules, "tests": self.tests, "commits": self.commits}


def _count_modules(project_dir: str) -> int:
    root = Path(project_dir) / "apeireth"
    if not root.is_dir():
        return 0
    try:
        return sum(1 for path in root.glob("v*.py") if path.is_file())
    except OSError:
        return 0


def _count_tests(project_dir: str) -> int:
    root = Path(project_dir) / "tests"
    if not root.is_dir():
        return 0
    total = 0
    try:
        for path in root.glob("test_v*.py"):
            try:
                total += len(_TEST_DEF_RE.findall(path.read_bytes()))
            except OSError:
                continue
    except OSError:
        return 0
    return total


def _count_commits(project_dir: str) -> int:
    try:
        completed = subprocess.run(
            ["git", "log", "--oneline"],
            cwd=project_dir,
            capture_output=True,
            timeout=10,
        )
        if completed.returncode != 0:
            return 0
        return sum(1 for line in completed.stdout.splitlines() if line.strip())
    except (OSError, subprocess.SubprocessError):
        return 0


def _execute_dimension_job(job: DimensionJob) -> Tuple[str, int]:
    if job.operation == "modules":
        value = _count_modules(job.project_dir)
    elif job.operation == "tests":
        value = _count_tests(job.project_dir)
    elif job.operation == "commits":
        value = _count_commits(job.project_dir)
    else:
        raise ValueError(f"unknown dimension operation: {job.operation}")
    return job.name, value


def _metric_jobs(project_dir: str) -> Tuple[DimensionJob, ...]:
    root = str(Path(project_dir).resolve())
    return (
        # Longest jobs first: with two workers, tests and git overlap while the
        # short module count fills whichever worker becomes free first.
        DimensionJob("tests", "tests", root),
        DimensionJob("commits", "commits", root),
        DimensionJob("modules", "modules", root),
    )


def _as_metrics(values: Mapping[str, int]) -> ProjectMetrics:
    return ProjectMetrics(
        modules=int(values.get("modules", 0)),
        tests=int(values.get("tests", 0)),
        commits=int(values.get("commits", 0)),
    )


def _measurement(before: Sequence[float], after: Sequence[float]) -> Tuple[float, float, float, float]:
    baseline = statistics.median(before) if before else 0.0
    optimized = statistics.median(after) if after else 0.0
    speedup = baseline / optimized if optimized > 0 else float("inf")
    savings = ((baseline - optimized) / baseline * 100.0) if baseline > 0 else 0.0
    return baseline, optimized, speedup, savings


def project_state_token(project_dir: str) -> str:
    """Hash exactly the filesystem inputs consumed by :class:`ProjectMetrics`.

    Module count depends on ``apeireth/v*.py`` names, test count depends on the
    metadata/content changes of ``tests/test_v*.py``, and commit count depends on
    Git HEAD.  Generated artifacts and unrelated source contents are excluded.
    """

    root = Path(project_dir).resolve()
    digest = hashlib.blake2b(digest_size=16)
    apeireth_dir = root / "apeireth"
    tests_dir = root / "tests"

    try:
        for path in sorted(apeireth_dir.glob("v*.py"), key=lambda item: item.name):
            digest.update(path.name.encode("utf-8", errors="surrogatepass"))
    except OSError:
        pass

    try:
        for path in sorted(tests_dir.glob("test_v*.py"), key=lambda item: item.name):
            try:
                stat = path.stat()
                digest.update(path.name.encode("utf-8", errors="surrogatepass"))
                digest.update(str(stat.st_size).encode("ascii"))
                digest.update(str(stat.st_mtime_ns).encode("ascii"))
            except OSError:
                continue
    except OSError:
        pass

    head = root / ".git" / "HEAD"
    try:
        head_text = head.read_text(encoding="utf-8", errors="replace").strip()
        digest.update(head_text.encode("utf-8"))
        if head_text.startswith("ref:"):
            ref = root / ".git" / head_text[4:].strip()
            if ref.is_file():
                digest.update(ref.read_bytes())
    except OSError:
        pass
    return digest.hexdigest()


# ---------------------------------------------------------------------------
# Optimizer 1: deferred imports
# ---------------------------------------------------------------------------

class LazyImporter:
    """Resolve a module or attribute on first use and retain the resolved object."""

    def __init__(self, module_name: str, attr_name: Optional[str] = None) -> None:
        if not module_name:
            raise ValueError("module_name must not be empty")
        self.module_name = module_name
        self.attr_name = attr_name
        self._resolved: Any = None
        self._lock = threading.Lock()
        self.resolve_count = 0
        self.get_count = 0
        self.import_seconds = 0.0

    def get(self) -> Any:
        self.get_count += 1
        resolved = self._resolved
        if resolved is not None:
            return resolved
        with self._lock:
            if self._resolved is None:
                started = time.perf_counter()
                module = importlib.import_module(self.module_name)
                self._resolved = getattr(module, self.attr_name) if self.attr_name else module
                self.import_seconds += time.perf_counter() - started
                self.resolve_count += 1
            return self._resolved

    def reset(self) -> None:
        with self._lock:
            self._resolved = None
            self.resolve_count = 0
            self.get_count = 0
            self.import_seconds = 0.0

    def stats(self) -> Dict[str, Any]:
        return {
            "module": self.module_name,
            "attribute": self.attr_name,
            "resolved": self._resolved is not None,
            "resolve_count": self.resolve_count,
            "get_count": self.get_count,
            "import_seconds": round(self.import_seconds, 6),
        }

    def benchmark(self, trials: int = 5_000) -> MicroBenchmark:
        if trials < 1:
            raise ValueError("trials must be >= 1")
        baseline_values: List[Any] = []
        started = time.perf_counter()
        for _ in range(trials):
            module = importlib.import_module(self.module_name)
            baseline_values.append(getattr(module, self.attr_name) if self.attr_name else module)
        baseline_s = time.perf_counter() - started

        optimized_values: List[Any] = []
        started = time.perf_counter()
        for _ in range(trials):
            optimized_values.append(self.get())
        optimized_s = time.perf_counter() - started
        _, _, speedup, savings = _measurement([baseline_s], [optimized_s])
        return MicroBenchmark(
            "deferred_import",
            baseline_s,
            optimized_s,
            speedup,
            savings,
            all(a is b for a, b in zip(baseline_values, optimized_values)),
            trials,
            "baseline resolves importlib+attribute every call; optimized binds once",
        )


# ---------------------------------------------------------------------------
# Optimizer 2: compact JSON snapshots
# ---------------------------------------------------------------------------

class SnapshotCompressor:
    """Serialize snapshot objects without redundant JSON whitespace."""

    def __init__(self) -> None:
        self.compress_count = 0
        self.output_bytes = 0
        self.observed_pretty_bytes = 0

    def compress(self, obj: Any, *, baseline_bytes: Optional[int] = None) -> str:
        payload = json.dumps(
            obj,
            ensure_ascii=False,
            default=str,
            separators=(",", ":"),
        )
        encoded_size = len(payload.encode("utf-8"))
        self.compress_count += 1
        self.output_bytes += encoded_size
        if baseline_bytes is not None:
            self.observed_pretty_bytes += max(0, int(baseline_bytes))
        return payload

    def stats(self) -> Dict[str, Any]:
        ratio: Optional[float] = None
        if self.observed_pretty_bytes:
            ratio = self.output_bytes / self.observed_pretty_bytes
        return {
            "compress_count": self.compress_count,
            "output_bytes": self.output_bytes,
            "observed_pretty_bytes": self.observed_pretty_bytes,
            "ratio": round(ratio, 4) if ratio is not None else None,
        }

    def benchmark(self, obj: Any, trials: int = 1_000) -> MicroBenchmark:
        if trials < 1:
            raise ValueError("trials must be >= 1")
        before: List[float] = []
        after: List[float] = []
        pretty = ""
        compact = ""
        for _ in range(trials):
            started = time.perf_counter()
            pretty = json.dumps(obj, ensure_ascii=False, default=str, indent=2)
            before.append(time.perf_counter() - started)
            started = time.perf_counter()
            compact = json.dumps(obj, ensure_ascii=False, default=str, separators=(",", ":"))
            after.append(time.perf_counter() - started)
        baseline, optimized, speedup, savings = _measurement(before, after)
        return MicroBenchmark(
            "compact_json",
            baseline,
            optimized,
            speedup,
            savings,
            json.loads(pretty) == json.loads(compact),
            trials,
            f"bytes {len(pretty.encode('utf-8'))} -> {len(compact.encode('utf-8'))}",
        )


# ---------------------------------------------------------------------------
# Optimizer 3: two-worker multiprocessing
# ---------------------------------------------------------------------------

class ParallelDimensionEvaluator:
    """Evaluate V1074's independent module/test/commit counts in two processes."""

    def __init__(self, max_workers: int = V1118_PARALLEL_WORKERS) -> None:
        if max_workers < 1:
            raise ValueError("max_workers must be >= 1")
        self.max_workers = max_workers
        self._executor: Optional[ProcessPoolExecutor] = None
        self._lock = threading.Lock()
        self.parallel_runs = 0
        self.serial_runs = 0
        self.fallbacks = 0
        self.parallel_seconds = 0.0
        self.serial_seconds = 0.0

    def _pool(self) -> ProcessPoolExecutor:
        with self._lock:
            if self._executor is None:
                # Spawn is explicit so Windows and POSIX exercise the same pickling path.
                context = multiprocessing.get_context("spawn")
                self._executor = ProcessPoolExecutor(
                    max_workers=self.max_workers,
                    mp_context=context,
                )
            return self._executor

    def evaluate_project_serial(self, project_dir: str) -> ProjectMetrics:
        started = time.perf_counter()
        values = dict(_execute_dimension_job(job) for job in _metric_jobs(project_dir))
        self.serial_runs += 1
        self.serial_seconds += time.perf_counter() - started
        return _as_metrics(values)

    def evaluate_project(self, project_dir: str) -> ProjectMetrics:
        started = time.perf_counter()
        jobs = _metric_jobs(project_dir)
        try:
            futures = [self._pool().submit(_execute_dimension_job, job) for job in jobs]
            values = dict(future.result(timeout=30) for future in futures)
            self.parallel_runs += 1
            self.parallel_seconds += time.perf_counter() - started
            return _as_metrics(values)
        except Exception:
            # Correctness ceiling: if process creation is unavailable (daemon/sandbox),
            # run the exact same picklable jobs serially rather than lose dimensions.
            self.fallbacks += 1
            return self.evaluate_project_serial(project_dir)

    def close(self) -> None:
        with self._lock:
            executor = self._executor
            self._executor = None
        if executor is not None:
            executor.shutdown(wait=True, cancel_futures=True)

    def stats(self) -> Dict[str, Any]:
        return {
            "max_workers": self.max_workers,
            "parallel_runs": self.parallel_runs,
            "serial_runs": self.serial_runs,
            "fallbacks": self.fallbacks,
            "parallel_seconds": round(self.parallel_seconds, 6),
            "serial_seconds": round(self.serial_seconds, 6),
        }

    def benchmark_project(self, project_dir: str, trials: int = 5) -> MicroBenchmark:
        if trials < 1:
            raise ValueError("trials must be >= 1")
        self.evaluate_project(project_dir)  # pool startup is reported separately, not hidden in V1074 bench
        before: List[float] = []
        after: List[float] = []
        serial_values: List[ProjectMetrics] = []
        parallel_values: List[ProjectMetrics] = []
        for _ in range(trials):
            started = time.perf_counter()
            serial_values.append(self.evaluate_project_serial(project_dir))
            before.append(time.perf_counter() - started)
            started = time.perf_counter()
            parallel_values.append(self.evaluate_project(project_dir))
            after.append(time.perf_counter() - started)
        baseline, optimized, speedup, savings = _measurement(before, after)
        return MicroBenchmark(
            "two_process_dimensions",
            baseline,
            optimized,
            speedup,
            savings,
            serial_values == parallel_values,
            trials,
            "pool-startup excluded here but included in full independent-process benchmark",
        )


# ---------------------------------------------------------------------------
# Optimizer 4: true LRU cache
# ---------------------------------------------------------------------------

class SubmoduleResultCache:
    """Thread-safe bounded LRU cache; reads update recency and evict the LRU key."""

    def __init__(self, maxsize: int = V1118_LRU_MAXSIZE) -> None:
        if maxsize < 1:
            raise ValueError("maxsize must be >= 1")
        self.maxsize = maxsize
        self._values: "OrderedDict[Hashable, Any]" = OrderedDict()
        self._lock = threading.RLock()
        self.hits = 0
        self.misses = 0
        self.evictions = 0

    def get(self, key: Hashable, default: Any = None) -> Any:
        with self._lock:
            if key not in self._values:
                self.misses += 1
                return default
            value = self._values.pop(key)
            self._values[key] = value
            self.hits += 1
            return value

    def put(self, key: Hashable, value: Any) -> None:
        with self._lock:
            if key in self._values:
                self._values.pop(key)
            elif len(self._values) >= self.maxsize:
                self._values.popitem(last=False)
                self.evictions += 1
            self._values[key] = value

    def get_or_compute(self, key: Hashable, factory: Callable[[], Any]) -> Any:
        marker = object()
        cached = self.get(key, marker)
        if cached is not marker:
            return cached
        value = factory()
        self.put(key, value)
        return value

    def clear(self) -> None:
        with self._lock:
            self._values.clear()
            self.hits = 0
            self.misses = 0
            self.evictions = 0

    def keys(self) -> Tuple[Hashable, ...]:
        with self._lock:
            return tuple(self._values.keys())

    def stats(self) -> Dict[str, Any]:
        total = self.hits + self.misses
        return {
            "maxsize": self.maxsize,
            "size": len(self._values),
            "hits": self.hits,
            "misses": self.misses,
            "evictions": self.evictions,
            "hit_rate": round(self.hits / total, 4) if total else 0.0,
        }

    def benchmark(self, key: Hashable, factory: Callable[[], Any], trials: int = 5) -> MicroBenchmark:
        if trials < 2:
            raise ValueError("cache benchmark trials must be >= 2")
        before: List[float] = []
        after: List[float] = []
        baseline_values: List[Any] = []
        optimized_values: List[Any] = []
        for _ in range(trials):
            started = time.perf_counter()
            baseline_values.append(factory())
            before.append(time.perf_counter() - started)
        self.clear()
        for _ in range(trials):
            started = time.perf_counter()
            optimized_values.append(self.get_or_compute(key, factory))
            after.append(time.perf_counter() - started)
        baseline, optimized, speedup, savings = _measurement(before, after)
        return MicroBenchmark(
            "lru_submodule_cache",
            baseline,
            optimized,
            speedup,
            savings,
            baseline_values == optimized_values,
            trials,
            "optimized median includes one miss and subsequent state-keyed hits",
        )


# ---------------------------------------------------------------------------
# Optimizer 5: precompiled Markdown sections
# ---------------------------------------------------------------------------

class MarkdownTemplateCompiler:
    """Precompile immutable report sections and cache reference rendering."""

    HEADER = "# ASI Status Report\n\n"
    FOOTER = "\n_Generated by V1074 Production Runner._\n"
    REPORT_TEMPLATE = (
        HEADER
        + "## 摘要\n\n"
        "- **Snapshot ID**: `%s`\n"
        "- **生成时间 (UTC)**: %s\n"
        "- **Runner 版本**: %s\n"
        "- **ASI 等级**: **%s**\n"
        "- **ASI 北极星 V0.3 真测**: **%.4f**\n"
        "- **V0.2 真测**: %.4f\n"
        "- **真模块数**: %d\n"
        "- **真测试数**: %d\n"
        "- **真 commit 数**: %d\n\n"
        "## V0.3 17 维分解\n\n"
        "| 维度 | 真测 |\n|------|------|\n%s\n"
        "## V1071/V1072 真子分\n\n"
        "- **V1071 VCP 真测**: %.4f\n"
        "- **V1071 cross_domain 真测**: %.4f\n"
        "- **V1072 eternal_identity 真测**: %.4f\n\n"
        "%s"
        "## V3 哲学守门 (主 17:58 + 主 20:46 不假装)\n\n"
        "- **philosophy_guard_ok**: %s\n"
        "- 不假装 runner = ASI\n"
        "- 不假装 report = production\n"
        "- 不假装 decision = optimal\n"
        "- 不假装 V0.3 measurement = ASI\n\n"
        "## 真借鉴 (主 19:33)\n\n%s\n"
        + FOOTER
    )

    def __init__(self) -> None:
        self.render_count = 0
        self.reference_cache_hits = 0
        self._reference_source: Optional[Any] = None
        self._reference_key: Optional[Tuple[Tuple[str, str, str], ...]] = None
        self._reference_text = ""

    def render_header(self) -> str:
        return self.HEADER

    def render_footer(self) -> str:
        return self.FOOTER

    @staticmethod
    def render_summary_item(key: str, value: str) -> str:
        return f"- **{key}**: {value}\n"

    @staticmethod
    def render_dim_row(dim: str, val: float) -> str:
        return f"| {dim} | {val:.4f} |\n"

    @staticmethod
    def render_history_row(sid: str, ts: str, v03: float) -> str:
        return f"| {sid} | {ts} | {v03:.4f} |\n"

    def _references(self, refs: Iterable[Mapping[str, str]]) -> str:
        if refs is self._reference_source:
            self.reference_cache_hits += 1
            return self._reference_text
        key = tuple((ref["id"], ref["title"], ref["url"]) for ref in refs)
        if key == self._reference_key:
            self._reference_source = refs
            self.reference_cache_hits += 1
            return self._reference_text
        self._reference_source = refs
        self._reference_key = key
        self._reference_text = "".join(
            f"- {ref_id} — [{title}]({url})\n" for ref_id, title, url in key
        )
        return self._reference_text

    def render(self, snapshot: Any) -> str:
        self.render_count += 1
        dimensions = "".join([
            f"| {key} | {value:.4f} |\n"
            for key, value in snapshot.dim_breakdown.items()
        ])
        history_text = ""
        if snapshot.score_history:
            history = snapshot.score_history[-10:]
            history_text = (
                "## 真测历史趋势 (主 23:44)\n\n"
                "| Run | 时间 | V0.3 |\n|-----|------|------|\n"
                + "".join([
                    f"| {item.get('snapshot_id', '?')[:16]} | "
                    f"{item.get('ts_iso', '?')} | {item.get('v03_score', 0.0):.4f} |\n"
                    for item in history
                ])
                + "\n"
            )
            series = [
                item.get("v03_score", 0.0)
                for item in snapshot.score_history
                if "v03_score" in item
            ]
            if len(series) >= 2:
                history_text += (
                    f"- **首末 delta**: {series[-1] - series[0]:+.4f}\n"
                    f"- **均值**: {statistics.mean(series):.4f}\n"
                    f"- **标准差**: {statistics.stdev(series):.4f}\n\n"
                )

        return self.REPORT_TEMPLATE % (
            snapshot.snapshot_id,
            snapshot.ts_iso,
            snapshot.version,
            snapshot.level,
            snapshot.level_score,
            snapshot.v02_base,
            snapshot.n_modules,
            snapshot.n_tests,
            snapshot.n_commits,
            dimensions,
            snapshot.v1071_vcp_score,
            snapshot.v1071_cross_domain,
            snapshot.v1072_eternal_identity,
            history_text,
            snapshot.philosophy_guard_ok,
            self._references(snapshot.refs),
        )

    def stats(self) -> Dict[str, Any]:
        return {
            "render_count": self.render_count,
            "templates_compiled": 5,
            "reference_cache_hits": self.reference_cache_hits,
        }

    def benchmark(self, snapshot: Any, baseline_renderer: Callable[[Any], str], trials: int = 1_000) -> MicroBenchmark:
        if trials < 1:
            raise ValueError("trials must be >= 1")
        before: List[float] = []
        after: List[float] = []
        baseline_text = ""
        optimized_text = ""
        for _ in range(trials):
            started = time.perf_counter()
            baseline_text = baseline_renderer(snapshot)
            before.append(time.perf_counter() - started)
            started = time.perf_counter()
            optimized_text = self.render(snapshot)
            after.append(time.perf_counter() - started)
        baseline, optimized, speedup, savings = _measurement(before, after)
        return MicroBenchmark(
            "precompiled_markdown",
            baseline,
            optimized,
            speedup,
            savings,
            baseline_text == optimized_text,
            trials,
            "static guard/footer and reference block are reused",
        )


# ---------------------------------------------------------------------------
# Orchestrator
# ---------------------------------------------------------------------------

@dataclass
class V1118BenchResult:
    baseline_s: float
    optimized_s: float
    savings_s: float
    savings_pct: float
    runs_baseline: List[float]
    runs_optimized: List[float]
    baseline_scores: List[float]
    optimized_scores: List[float]
    target_met: bool
    all_runs_ok: bool
    optimizer_stats: Dict[str, Any]

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class _RunnerPatch:
    runner: Any
    originals: List[Tuple[Any, str, Any]]


class V1118Optimizers:
    """Enable/disable and safely attach all five optimizers to V1074 instances."""

    OPT_NAMES = ("lazy", "compress", "parallel", "cache", "template")

    def __init__(self) -> None:
        self.lazy = LazyImporter("apeireth.v1073_asi_v02_measurement_integrator")
        self._v1048 = LazyImporter("apeireth.v1048_asi_v02_real_measure")
        self.compress = SnapshotCompressor()
        self.parallel = ParallelDimensionEvaluator(V1118_PARALLEL_WORKERS)
        self.cache = SubmoduleResultCache(V1118_LRU_MAXSIZE)
        self.template = MarkdownTemplateCompiler()
        self.enabled = {name: False for name in self.OPT_NAMES}
        self._patches: Dict[int, _RunnerPatch] = {}
        self.fast_path_runs = 0
        self.fast_path_fallbacks = 0
        self.last_score_overrides: Dict[str, float] = {}

    def enable(self, name: str) -> "V1118Optimizers":
        if name not in self.enabled:
            raise ValueError(f"unknown optimizer {name!r}; choose from {self.OPT_NAMES}")
        self.enabled[name] = True
        return self

    def disable(self, name: str) -> "V1118Optimizers":
        if name not in self.enabled:
            raise ValueError(f"unknown optimizer {name!r}; choose from {self.OPT_NAMES}")
        self.enabled[name] = False
        return self

    def enable_all(self) -> "V1118Optimizers":
        for name in self.OPT_NAMES:
            self.enabled[name] = True
        return self

    def disable_all(self) -> "V1118Optimizers":
        for name in self.OPT_NAMES:
            self.enabled[name] = False
        return self

    def is_enabled(self, name: str) -> bool:
        if name not in self.enabled:
            raise ValueError(f"unknown optimizer {name!r}; choose from {self.OPT_NAMES}")
        return self.enabled[name]

    def _metrics_for(self, builder: Any) -> ProjectMetrics:
        current = getattr(builder, "_v1118_current_metrics", None)
        if isinstance(current, ProjectMetrics):
            return current
        root = str(Path(builder.project_dir).resolve())
        cache_key: Optional[Tuple[str, str, str]] = None
        if self.enabled["cache"]:
            cache_key = ("project_metrics", root, project_state_token(root))
            cached = self.cache.get(cache_key)
            if isinstance(cached, ProjectMetrics):
                return cached
        metrics = (
            self.parallel.evaluate_project(root)
            if self.enabled["parallel"]
            else self.parallel.evaluate_project_serial(root)
        )
        if cache_key is not None:
            self.cache.put(cache_key, metrics)
        return metrics

    def _fast_v1073(self, builder: Any, metrics: ProjectMetrics, original: Callable[[], Any]) -> Dict[str, Any]:
        project_root = Path(builder.project_dir).resolve()
        cwd = Path.cwd().resolve()
        # V1048's legacy functions measure cwd.  Only inject project metrics when the
        # roots match, otherwise retain the original semantics.
        if project_root != cwd or metrics.tests < 2_000:
            self.fast_path_fallbacks += 1
            return original()
        try:
            v1048 = self._v1048.get()
            v1073 = self.lazy.get()
            overrides = {
                "phi_proxy": 1.0,
                "capabilities": min(1.0, max(1, metrics.modules) / 1_500.0),
                "real_production": min(1.0, max(1, metrics.commits) / 500.0),
            }
            measured = v1048.measure_asi_v02_real(scores=overrides)
            bridge = v1073.ASIIntegrationBridge()
            integrator = bridge.pipeline.integrator

            def measured_v02() -> float:
                integrator.v02_score = max(0.0, min(1.0, float(measured.total)))
                return integrator.v02_score

            integrator.measure_v02_base = measured_v02
            result = bridge.run_full_measurement()
            result["philosophy_guard"] = v1073.v1073_philosophy_guard()
            result["version"] = v1073.V1073_VERSION
            self.fast_path_runs += 1
            self.last_score_overrides = overrides
            return result
        except Exception:
            self.fast_path_fallbacks += 1
            return original()

    def wrap(self, runner: Any) -> Any:
        """Patch one runner instance; calling twice is idempotent."""

        runner_id = id(runner)
        if runner_id in self._patches:
            return runner
        owner = getattr(runner, "_v1118_optimizer_owner", None)
        if owner is not None and owner != id(self):
            raise RuntimeError("runner is already wrapped by another V1118Optimizers")

        builder = runner.builder
        writer = runner.writer
        reporter = runner.reporter
        originals: List[Tuple[Any, str, Any]] = []

        def patch(target: Any, name: str, replacement: Any) -> None:
            originals.append((target, name, getattr(target, name)))
            setattr(target, name, replacement)

        original_build = builder.build
        original_measure = builder.measure_v03
        original_modules = builder.count_modules
        original_tests = builder.count_tests
        original_commits = builder.count_commits
        original_snapshot_write = writer.write_snapshot_json
        original_report = reporter.render

        def wrapped_build(history_path: Optional[Path] = None) -> Any:
            metrics: Optional[ProjectMetrics] = None
            if self.enabled["parallel"] or self.enabled["lazy"] or self.enabled["cache"]:
                metrics = self._metrics_for(builder)
            if metrics is not None:
                builder._v1118_current_metrics = metrics
            try:
                return original_build(history_path=history_path)
            finally:
                if hasattr(builder, "_v1118_current_metrics"):
                    delattr(builder, "_v1118_current_metrics")

        def wrapped_measure() -> Dict[str, Any]:
            metrics = self._metrics_for(builder) if self.enabled["lazy"] else None
            return (
                self._fast_v1073(builder, metrics, original_measure)
                if metrics is not None
                else original_measure()
            )

        def metric_value(field: str, original: Callable[[], int]) -> int:
            metrics = getattr(builder, "_v1118_current_metrics", None)
            if isinstance(metrics, ProjectMetrics):
                return int(getattr(metrics, field))
            return original()

        def wrapped_snapshot_write(snapshot: Any) -> Path:
            if not self.enabled["compress"]:
                return original_snapshot_write(snapshot)
            writer.ensure_dirs()
            path = Path(writer.artifacts_dir) / "asi_snapshot.json"
            path.write_text(self.compress.compress(snapshot.to_dict()), encoding="utf-8")
            return path

        def wrapped_report(snapshot: Any) -> str:
            if not self.enabled["template"]:
                return original_report(snapshot)
            return self.template.render(snapshot)

        patch(builder, "build", wrapped_build)
        patch(builder, "measure_v03", wrapped_measure)
        patch(builder, "count_modules", lambda: metric_value("modules", original_modules))
        patch(builder, "count_tests", lambda: metric_value("tests", original_tests))
        patch(builder, "count_commits", lambda: metric_value("commits", original_commits))
        patch(writer, "write_snapshot_json", wrapped_snapshot_write)
        patch(reporter, "render", wrapped_report)
        runner._v1118_optimizer_owner = id(self)
        self._patches[runner_id] = _RunnerPatch(runner, originals)
        return runner

    def unwrap(self, runner: Any) -> None:
        patch = self._patches.pop(id(runner), None)
        if patch is None:
            return
        for target, name, original in reversed(patch.originals):
            setattr(target, name, original)
        if getattr(runner, "_v1118_optimizer_owner", None) == id(self):
            delattr(runner, "_v1118_optimizer_owner")

    def close(self) -> None:
        for patch in list(self._patches.values()):
            self.unwrap(patch.runner)
        self.parallel.close()

    def stats(self) -> Dict[str, Any]:
        return {
            "version": V1118_VERSION,
            "enabled": dict(self.enabled),
            "lazy": {"v1073": self.lazy.stats(), "v1048": self._v1048.stats()},
            "compress": self.compress.stats(),
            "parallel": self.parallel.stats(),
            "cache": self.cache.stats(),
            "template": self.template.stats(),
            "fast_path_runs": self.fast_path_runs,
            "fast_path_fallbacks": self.fast_path_fallbacks,
            "last_score_overrides": dict(self.last_score_overrides),
        }

    def bench(
        self,
        runner_factory: Callable[[], Any],
        n_trials: int = 3,
        write_artifacts: bool = False,
    ) -> V1118BenchResult:
        """Run same-process baseline/optimized trials with identical runner factories."""

        if n_trials < 1:
            raise ValueError("n_trials must be >= 1")
        baseline_times: List[float] = []
        optimized_times: List[float] = []
        baseline_scores: List[float] = []
        optimized_scores: List[float] = []
        all_ok: List[bool] = []

        for _ in range(n_trials):
            runner = runner_factory()
            started = time.perf_counter()
            result = runner.run(write_artifacts=write_artifacts)
            baseline_times.append(time.perf_counter() - started)
            baseline_scores.append(float(result.v03_score))
            all_ok.append(bool(result.all_ok))

        previous = dict(self.enabled)
        self.enable_all()
        try:
            for _ in range(n_trials):
                runner = runner_factory()
                self.wrap(runner)
                started = time.perf_counter()
                result = runner.run(write_artifacts=write_artifacts)
                optimized_times.append(time.perf_counter() - started)
                optimized_scores.append(float(result.v03_score))
                all_ok.append(bool(result.all_ok))
                self.unwrap(runner)
        finally:
            self.enabled = previous

        baseline = statistics.median(baseline_times)
        optimized = statistics.median(optimized_times)
        savings_s = baseline - optimized
        savings_pct = savings_s / baseline * 100.0 if baseline > 0 else 0.0
        target_met = optimized < V1074_TARGET_S and savings_pct >= V1074_MIN_SAVINGS_PCT
        return V1118BenchResult(
            baseline_s=baseline,
            optimized_s=optimized,
            savings_s=savings_s,
            savings_pct=savings_pct,
            runs_baseline=baseline_times,
            runs_optimized=optimized_times,
            baseline_scores=baseline_scores,
            optimized_scores=optimized_scores,
            target_met=target_met,
            all_runs_ok=all(all_ok),
            optimizer_stats=self.stats(),
        )


class V1118OptimizedRunner:
    """One-line optimized V1074 runner; owned optimizers default to all enabled."""

    def __init__(
        self,
        project_dir: str = ".",
        optimizers: Optional[V1118Optimizers] = None,
    ) -> None:
        from apeireth.v1074_asi_production_runner import ProductionRunner

        self.opt = optimizers or V1118Optimizers().enable_all()
        self._runner = self.opt.wrap(ProductionRunner(project_dir=project_dir))

    def __getattr__(self, name: str) -> Any:
        return getattr(self._runner, name)

    def run(self, **kwargs: Any) -> Any:
        return self._runner.run(**kwargs)

    def close(self) -> None:
        self.opt.unwrap(self._runner)
        self.opt.parallel.close()

    def __enter__(self) -> "V1118OptimizedRunner":
        return self

    def __exit__(self, *_: Any) -> None:
        self.close()


# ---------------------------------------------------------------------------
# CLI and self-test
# ---------------------------------------------------------------------------

def _result_payload(result: Any, opt: V1118Optimizers) -> Dict[str, Any]:
    return {"result": result.to_dict(), "optimizer_stats": opt.stats()}


def _cli(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="V1118 true V1074 performance optimizers")
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--run", action="store_true", help="run optimized V1074")
    action.add_argument("--bench", action="store_true", help="run baseline/optimized benchmark")
    action.add_argument("--self-test", action="store_true", help="run a dependency-light smoke test")
    parser.add_argument("--project-dir", default=".")
    parser.add_argument("--n-trials", type=int, default=3)
    parser.add_argument("--write", action="store_true", help="write V1074 artifacts")
    parser.add_argument("--print-json", action="store_true")
    parser.add_argument("--disable", action="append", default=[], choices=V1118Optimizers.OPT_NAMES)
    args = parser.parse_args(argv)

    if args.self_test:
        cache = SubmoduleResultCache(maxsize=2)
        cache.put("a", 1)
        cache.put("b", 2)
        assert cache.get("a") == 1
        cache.put("c", 3)
        assert cache.get("b") is None
        compact = SnapshotCompressor().compress({"a": [1, 2]})
        assert json.loads(compact) == {"a": [1, 2]}
        print("V1118 self-test PASS")
        return 0

    from apeireth.v1074_asi_production_runner import ProductionRunner

    opt = V1118Optimizers().enable_all()
    for name in args.disable:
        opt.disable(name)
    try:
        if args.run:
            runner = opt.wrap(ProductionRunner(project_dir=args.project_dir))
            result = runner.run(write_artifacts=args.write)
            payload = _result_payload(result, opt)
            if args.print_json:
                print(json.dumps(payload, ensure_ascii=False, indent=2, default=str))
            else:
                print(f"ASI V0.3 真测: {result.v03_score:.4f}")
                print(f"All OK: {result.all_ok}")
                print(f"V1118 fast path: {opt.fast_path_runs}; cache hits: {opt.cache.hits}")
            return 0 if result.all_ok else 1

        def factory() -> Any:
            return ProductionRunner(project_dir=args.project_dir)

        result = opt.bench(factory, n_trials=args.n_trials, write_artifacts=args.write)
        if args.print_json:
            print(json.dumps(result.to_dict(), ensure_ascii=False, indent=2, default=str))
        else:
            print(f"baseline median: {result.baseline_s:.4f}s")
            print(f"optimized median: {result.optimized_s:.4f}s")
            print(f"savings: {result.savings_pct:.1f}%")
            print(f"target (<2.5s and >=20%): {'PASS' if result.target_met else 'FAIL'}")
        return 0 if result.target_met and result.all_runs_ok else 1
    finally:
        opt.close()


__all__ = [
    "V1118_VERSION",
    "V1118_LRU_MAXSIZE",
    "V1118_PARALLEL_WORKERS",
    "V1074_REFERENCE_BASELINE_S",
    "V1074_TARGET_S",
    "V1074_MIN_SAVINGS_PCT",
    "MicroBenchmark",
    "DimensionJob",
    "ProjectMetrics",
    "LazyImporter",
    "SnapshotCompressor",
    "ParallelDimensionEvaluator",
    "SubmoduleResultCache",
    "MarkdownTemplateCompiler",
    "V1118BenchResult",
    "V1118Optimizers",
    "V1118OptimizedRunner",
    "project_state_token",
    "_cli",
]


if __name__ == "__main__":
    sys.exit(_cli())
