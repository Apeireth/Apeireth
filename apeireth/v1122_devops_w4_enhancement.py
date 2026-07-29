"""V1122 DevOps W4 enhancement: matrix batching + retry + cache + DAG + lint.

R9-DEV-003 / R9-DevOps W4 收尾.

主 22:33 ASI 北极星 (终极梦想: 任何 LLM 接入即获 AGI/ASI 能力 — W4 让 CI 编排
  能承接 ASI 全栈 CI: matrix 切批 + 缓存 + 重试 + DAG 并行)
主 17:43 实事求是 (所有调度决策看真数据: cache key 真哈希, retry 真退避, 不刷KPI)
主 13:31 大胆激进 (增量矩阵优化 — 缓存命中则缩减 + DAG 并行 + lint 早失败)
主 23:44 干到底 (≥5 个真功能, 真测真产)
主 19:33 走在前人经验上:
  - GitHub Actions matrix 2020 (跨 os × python × model 笛卡尔)
  - AWS retry with jitter 2018 (full-jitter exponential backoff)
  - tenacity 2016 (retry decorator)
  - Apache Airflow DAG 2015 (有向无环图 + 拓扑排序)
  - pytest cache 2008 (.pytest_cache 内容哈希复用)
  - ESLint 2013 (lint 早失败)
主 00:56 任何人都能接手 (`build_matrix_plan` / `topo_sort` / `retry_with_policy` 一行可调)
主 17:58+20:46 不假装 (cache miss 显式记录, 不假装命中; DAG 环显式 ValueError)

Public API:
    MatrixJob, MatrixPlan, build_matrix_plan, partition_matrix_plan
    RetryPolicy, compute_backoff_ms, retry_with_policy
    CIArtifactCache (内容哈希 + TTL + LRU)
    CIWorkflowDAG (有向无环图 + 拓扑排序 + 环检测)
    W4LintIssue, lint_matrix_plan, lint_workflow_yaml_text
    optimize_matrix_plan (增量优化: 缓存命中 → 缩减)
"""
from __future__ import annotations

import hashlib
import random
import re
import time
from collections import OrderedDict, deque
from dataclasses import dataclass, field
from typing import Any, Callable, Dict, Iterable, List, Optional, Sequence, Tuple, TypeVar


T = TypeVar("T")


# ---------------------------------------------------------------------------
# 1. GitHub Actions matrix 切批 (主 13:31 大胆激进)
# ---------------------------------------------------------------------------
@dataclass(frozen=True)
class MatrixJob:
    """单 matrix job (family × dim × task 三元组)."""

    family: str
    dim: str
    task_id: str
    timeout_sec: float = 60.0

    def job_id(self) -> str:
        return f"{self.family}__{self.dim}__{self.task_id}"

    def cache_key(self) -> str:
        # 内容哈希: family|dim|task|timeout
        h = hashlib.sha256(
            f"{self.family}|{self.dim}|{self.task_id}|{self.timeout_sec}".encode("utf-8")
        ).hexdigest()
        return h[:16]


@dataclass
class MatrixPlan:
    """matrix 编排结果: jobs 列表 + 派生 cache keys."""

    jobs: List[MatrixJob]
    created_at: float = field(default_factory=time.time)

    def __len__(self) -> int:
        return len(self.jobs)

    def by_family(self) -> Dict[str, List[MatrixJob]]:
        out: Dict[str, List[MatrixJob]] = OrderedDict()
        for j in self.jobs:
            out.setdefault(j.family, []).append(j)
        return out


def build_matrix_plan(families: Sequence[str],
                      dims: Sequence[str],
                      tasks: Sequence[str],
                      timeout_sec: float = 60.0) -> MatrixPlan:
    """构造 matrix 计划 (GitHub Actions 风格笛卡尔积).

    主 00:56 任何人都能接手: 3 序列 → 1 计划; 不必手填 jobs.
    """
    jobs = [
        MatrixJob(family=f, dim=d, task_id=t, timeout_sec=timeout_sec)
        for f in families for d in dims for t in tasks
    ]
    return MatrixPlan(jobs=jobs)


def partition_matrix_plan(plan: MatrixPlan, max_concurrent: int = 4) -> List[List[MatrixJob]]:
    """切批: 把 plan 切成多个 batch, 每个 batch ≤ max_concurrent jobs.

    主 13:31 大胆激进: 控制并发上限 (避免真模型同时加载 N 个 OOM).
    主 17:43 实事求是: 按 job_id 稳定排序, 同一 plan 切批确定.
    """
    if max_concurrent <= 0:
        raise ValueError("max_concurrent must be >= 1")
    sorted_jobs = sorted(plan.jobs, key=lambda j: j.job_id())
    batches: List[List[MatrixJob]] = []
    for i in range(0, len(sorted_jobs), max_concurrent):
        batches.append(sorted_jobs[i:i + max_concurrent])
    return batches


# ---------------------------------------------------------------------------
# 2. RetryPolicy (借鉴 AWS retry with jitter 2018 + tenacity 2016)
# ---------------------------------------------------------------------------
@dataclass(frozen=True)
class RetryPolicy:
    """重试策略 (主 17:58 不假装 + 主 19:33 走在前人经验上).

    AWS Architecture Blog 2018 "Exponential Backoff and Jitter":
      sleep = random(0, min(cap, base * 2 ** (attempt - 1)))
    tenacity 2016 默认: max_attempt + wait_random_exponential.
    """

    max_attempts: int = 3
    base_ms: float = 100.0     # 第 1 次重试前等待 base ms
    cap_ms: float = 5000.0     # 单次最大等待
    jitter: str = "full"       # full / equal / none (借鉴 AWS 三种 jitter)
    retry_on: Tuple[type, ...] = (TimeoutError, ConnectionError, OSError)

    def __post_init__(self) -> None:
        if self.max_attempts < 1:
            raise ValueError("max_attempts must be >= 1")
        if self.base_ms <= 0 or self.cap_ms <= 0:
            raise ValueError("base_ms and cap_ms must be > 0")
        if self.jitter not in ("full", "equal", "none"):
            raise ValueError(f"jitter must be full/equal/none, got {self.jitter}")


def compute_backoff_ms(policy: RetryPolicy, attempt: int, rng: Optional[random.Random] = None) -> float:
    """计算第 attempt 次重试前的退避时间 (ms).

    Args:
        policy: RetryPolicy
        attempt: 1..max_attempts (1 = 第 1 次重试前等待)
        rng: 可选 Random 实例 (注入以保证可测性); None → 内部随机.

    Returns:
        等待时长 (ms, float). attempt 超界返回 0.
    """
    if attempt < 1 or attempt > policy.max_attempts:
        return 0.0
    rng = rng or random.Random()
    expo = policy.base_ms * (2 ** (attempt - 1))
    cap = min(policy.cap_ms, expo)
    if policy.jitter == "none":
        return cap
    if policy.jitter == "equal":
        half = cap / 2.0
        return half + rng.uniform(0.0, half)
    # full jitter: random(0, cap)
    return rng.uniform(0.0, cap)


def retry_with_policy(policy: RetryPolicy,
                      fn: Callable[[], T],
                      sleep_fn: Optional[Callable[[float], None]] = None,
                      rng: Optional[random.Random] = None) -> T:
    """带策略跑 fn (主 23:44 干到底 + 主 17:58 不假装).

    Args:
        policy: RetryPolicy
        fn: 真正干活的可调用对象 (无参, 返 T)
        sleep_fn: 测试时注入 (e.g. lambda s: None), 默认 time.sleep
        rng: 测试时注入 random.Random(seed), 默认新随机

    Raises:
        最后一次失败时的异常 (原异常类型, 不包装).
    """
    sleep_fn = sleep_fn or (lambda s: time.sleep(s / 1000.0))
    last_err: Optional[BaseException] = None
    for attempt in range(1, policy.max_attempts + 1):
        try:
            return fn()
        except policy.retry_on as e:  # type: ignore[misc]
            last_err = e
            if attempt >= policy.max_attempts:
                break
            wait_ms = compute_backoff_ms(policy, attempt, rng=rng)
            if wait_ms > 0:
                sleep_fn(wait_ms)
    assert last_err is not None
    raise last_err


# ---------------------------------------------------------------------------
# 3. CIArtifactCache (内容哈希 + TTL + LRU 上限)
# ---------------------------------------------------------------------------
@dataclass
class _CacheEntry:
    value: Any
    expires_at: float
    key: str


class CIArtifactCache:
    """CI 产物缓存 (主 00:44 质量工程化 + 主 17:43 实事求是).

    设计:
      - key = 内容哈希 (调用方决定 hash 什么; 我们提供 compute_key)
      - TTL: set 时指定秒数, 过期显式 miss (主 17:58 不假装)
      - LRU 上限: 超过 → 淘汰最旧
      - 命中次数统计: hit/miss/set/expire/evict
    """

    def __init__(self, max_entries: int = 256, default_ttl_sec: float = 300.0):
        self.max_entries = int(max_entries)
        self.default_ttl_sec = float(default_ttl_sec)
        self._store: "OrderedDict[str, _CacheEntry]" = OrderedDict()
        self.hits = 0
        self.misses = 0
        self.sets = 0
        self.expirations = 0
        self.evictions = 0

    @staticmethod
    def compute_key(*parts: Any) -> str:
        """内容哈希 (借鉴 pytest .pytest_cache 2008 内容指纹)."""
        h = hashlib.sha256()
        for p in parts:
            h.update(repr(p).encode("utf-8"))
            h.update(b"|")
        return h.hexdigest()[:16]

    def get(self, key: str) -> Any:
        ent = self._store.get(key)
        if ent is None:
            self.misses += 1
            return None
        if time.time() > ent.expires_at:
            del self._store[key]
            self.expirations += 1
            self.misses += 1
            return None
        self._store.move_to_end(key)  # LRU touch
        self.hits += 1
        return ent.value

    def set(self, key: str, value: Any, ttl_sec: Optional[float] = None) -> None:
        ttl = self.default_ttl_sec if ttl_sec is None else float(ttl_sec)
        self._store[key] = _CacheEntry(value=value, expires_at=time.time() + ttl, key=key)
        self._store.move_to_end(key)
        self.sets += 1
        while len(self._store) > self.max_entries:
            self._store.popitem(last=False)
            self.evictions += 1

    def invalidate(self, key: str) -> bool:
        return self._store.pop(key, None) is not None

    def stats(self) -> Dict[str, int]:
        return {
            "hits": self.hits,
            "misses": self.misses,
            "sets": self.sets,
            "expirations": self.expirations,
            "evictions": self.evictions,
            "size": len(self._store),
            "max_entries": self.max_entries,
        }


# ---------------------------------------------------------------------------
# 4. CIWorkflowDAG (有向无环图 + 拓扑排序 + 环检测)
# ---------------------------------------------------------------------------
class CIWorkflowDAGError(ValueError):
    """DAG 错误 (主 17:58 不假装 + 主 17:43 实事求是)."""


class CIWorkflowDAG:
    """CI 工作流 DAG (借鉴 Apache Airflow 2015).

    节点 = task_id; 边 = upstream -> downstream.
    topo_sort: Kahn's algorithm (BFS, 借鉴 Wikipedia 1962 → Airflow 2015).
    环检测: 拓扑排序过程中若剩余节点 > 0 → 显式 raise (主 17:58).
    """

    def __init__(self) -> None:
        self._nodes: "OrderedDict[str, List[str]]" = OrderedDict()  # node -> [downstream]

    def add_node(self, task_id: str) -> None:
        self._nodes.setdefault(task_id, [])

    def add_edge(self, upstream: str, downstream: str) -> None:
        if upstream == downstream:
            raise CIWorkflowDAGError(f"self-loop not allowed: {upstream}")
        if upstream not in self._nodes:
            self._nodes[upstream] = []
        if downstream not in self._nodes:
            self._nodes[downstream] = []
        if downstream not in self._nodes[upstream]:
            self._nodes[upstream].append(downstream)

    @property
    def nodes(self) -> List[str]:
        return list(self._nodes.keys())

    @property
    def edges(self) -> List[Tuple[str, str]]:
        return [(u, v) for u, vs in self._nodes.items() for v in vs]

    def topo_sort(self) -> List[str]:
        """Kahn 拓扑排序 (BFS).

        Returns:
            节点列表 (满足所有 upstream 在 downstream 之前).

        Raises:
            CIWorkflowDAGError: 存在环.
        """
        in_degree: Dict[str, int] = {n: 0 for n in self._nodes}
        for u, vs in self._nodes.items():
            for v in vs:
                in_degree[v] = in_degree.get(v, 0) + 1
        # 稳定排序: 按节点插入顺序
        queue: deque[str] = deque(n for n, d in in_degree.items() if d == 0)
        order: List[str] = []
        while queue:
            n = queue.popleft()
            order.append(n)
            for v in self._nodes.get(n, []):
                in_degree[v] -= 1
                if in_degree[v] == 0:
                    queue.append(v)
        if len(order) != len(self._nodes):
            remaining = [n for n, d in in_degree.items() if d > 0]
            raise CIWorkflowDAGError(f"cycle detected; unresolved nodes: {remaining}")
        return order


# ---------------------------------------------------------------------------
# 5. W4Lint (主 13:31 大胆激进 + 主 17:58 不假装: 早失败)
# ---------------------------------------------------------------------------
@dataclass(frozen=True)
class W4LintIssue:
    """lint 问题 (借鉴 ESLint 2013)."""

    level: str     # "error" | "warning" | "info"
    rule: str      # 规则 ID (e.g. "matrix_too_large")
    message: str

    def to_dict(self) -> Dict[str, str]:
        return {"level": self.level, "rule": self.rule, "message": self.message}


def lint_matrix_plan(plan: MatrixPlan,
                     max_jobs: int = 64,
                     default_timeout_sec: float = 60.0) -> List[W4LintIssue]:
    """lint matrix plan (主 17:43 实事求是).

    Rules:
      - matrix_too_large: jobs > max_jobs → error
      - timeout_too_low: timeout < 5s → warning (大模型加载不够)
      - duplicate_job_id: 重复 job_id → error (主 17:58 不假装)
    """
    issues: List[W4LintIssue] = []
    if len(plan.jobs) > max_jobs:
        issues.append(W4LintIssue(
            level="error",
            rule="matrix_too_large",
            message=f"matrix has {len(plan.jobs)} jobs > max {max_jobs}; "
                    f"split into multiple workflows.",
        ))
    seen: Dict[str, int] = {}
    for j in plan.jobs:
        seen[j.job_id()] = seen.get(j.job_id(), 0) + 1
        if j.timeout_sec < 5.0:
            issues.append(W4LintIssue(
                level="warning",
                rule="timeout_too_low",
                message=f"job {j.job_id()} timeout={j.timeout_sec}s < 5s "
                        f"(大模型加载可能不够).",
            ))
    for jid, n in seen.items():
        if n > 1:
            issues.append(W4LintIssue(
                level="error",
                rule="duplicate_job_id",
                message=f"duplicate job_id {jid} appears {n} times.",
            ))
    return issues


_GH_MATRIX_RE = re.compile(
    r"^\s*matrix:\s*$(?P<body>(?:^\s{4,}\S.*\n?)+)", re.MULTILINE
)


def lint_workflow_yaml_text(text: str,
                            max_matrix_jobs: int = 64) -> List[W4LintIssue]:
    """极简 lint: 从 GitHub Actions YAML 文本里抠出 `matrix:` 段, 检查.

    主 17:43: 仅做轻量结构校验, 不引入 PyYAML 依赖 (ponytail: 够用就行).
    主 19:33 走在前人经验上: GitHub Actions 2020 matrix schema.

    Rules:
      - missing_timeout: strategy matrix 段没提 timeout → warning
      - matrix_too_large: 估算 jobs > max → error (粗估: 矩阵维度笛卡尔)
    """
    issues: List[W4LintIssue] = []
    if "matrix:" not in text:
        issues.append(W4LintIssue(
            level="info",
            rule="no_matrix",
            message="workflow has no 'matrix:' strategy.",
        ))
        return issues
    if "timeout-minutes" not in text:
        issues.append(W4LintIssue(
            level="warning",
            rule="missing_timeout",
            message="workflow has no 'timeout-minutes'; jobs may hang forever.",
        ))
    # 估算 matrix 大小: 找含 "include" / "exclude" 之外的列表项数量 (粗估)
    m = _GH_MATRIX_RE.search(text)
    if m:
        body = m.group("body")
        # 数顶层 list 项 (e.g. "    - python-version: '3.12'")
        items = re.findall(r"^\s{6,}-\s\S", body, re.MULTILINE)
        if len(items) > max_matrix_jobs:
            issues.append(W4LintIssue(
                level="error",
                rule="matrix_too_large",
                message=f"workflow matrix has ~{len(items)} entries > {max_matrix_jobs}.",
            ))
    return issues


# ---------------------------------------------------------------------------
# 6. optimize_matrix_plan (主 13:31 大胆激进: 缓存命中 → 缩减)
# ---------------------------------------------------------------------------
def optimize_matrix_plan(plan: MatrixPlan,
                        cache: CIArtifactCache,
                        ttl_sec: float = 600.0) -> Tuple[MatrixPlan, List[MatrixJob]]:
    """根据 cache 缩减 matrix: 已缓存 job 不再执行.

    主 17:43 实事求是: cache 命中靠真哈希, 不 hardcode "都通过".

    Returns:
        (remaining_plan, cached_jobs)
    """
    remaining: List[MatrixJob] = []
    cached: List[MatrixJob] = []
    for j in plan.jobs:
        key = j.cache_key()
        if cache.get(key) is not None:
            cached.append(j)
        else:
            remaining.append(j)
    return MatrixPlan(jobs=remaining), cached


# ---------------------------------------------------------------------------
# __all__
# ---------------------------------------------------------------------------
__all__ = [
    # 1. matrix
    "MatrixJob", "MatrixPlan", "build_matrix_plan", "partition_matrix_plan",
    # 2. retry
    "RetryPolicy", "compute_backoff_ms", "retry_with_policy",
    # 3. cache
    "CIArtifactCache",
    # 4. DAG
    "CIWorkflowDAG", "CIWorkflowDAGError",
    # 5. lint
    "W4LintIssue", "lint_matrix_plan", "lint_workflow_yaml_text",
    # 6. optimize
    "optimize_matrix_plan",
    # module
    "__version__",
]

__version__ = "0.1.0"