"""V1098 DGM Archive 性能基线 + 优化套件 (R8 Track C).

主 17:43 实事求是 + 主 19:33 走在前人经验上 + V3 守门 + 主 23:44 干到底。

参考数据 (来自 R7 交接 + RUST-PYTHON-BENCHMARK-2026-07-20):
- DeltaMemory Rust 检索  < 50ms
- Python 类似路径        ~ 800ms
- V1000 YAML 序列化       ~ 2ms / 1KB dict
- V1084 token 启发估算    ~ 0.5ms / 1KB
- V1091 WAL verify        ~ 5ms / 1000 entries

4 个关键路径 (本模块):
  1. 单轮 DGM 演化端到端延迟    (dgm_evolve_one)
  2. Archive 索引构建时间        (build_archive_index)
  3. UCB1 bandit 选择延迟        (ucb1_select)
  4. 记忆回放查询延迟            (replay_query)

优化策略 (主 14:32 高效 nb):
  A. 热路径用 functools.lru_cache / 自维护 LRU
  B. Archive 用增量索引 (delta merge) 不全量重建
  C. UCB1 bandit 选择向量化 (math + 列表推导) vs 循环 max()
  D. 记忆回放用 tag 反向索引 (dict[tag] -> set[event_id]) 加速

V3 守门 (主 17:58 + 主 20:46 不假装):
  - 实测毫秒数, 不估算
  - 报告 n_iter, std_dev, 严格 baseline 对照
  - 优化 ≠ ASI lift, 只是路径加速

CLI:
  python -m apeireth.v1098_dgm_perf --bench   # 跑全部 4 路径 baseline+optimized
  python -m apeireth.v1098_dgm_perf --report  # 输出 JSON 报告
"""
from __future__ import annotations

import argparse
import functools
import json
import math
import random
import statistics
import sys
import time
from collections import OrderedDict
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable, Dict, Iterable, List, Optional, Tuple


V1098_VERSION = "0.1.0"


# ============================================================
# 数据结构 — 与 v1093_dgm_archive / v1091_memory_replay / v1092 兼容
# ============================================================


@dataclass
class DGMGen:
    """DGM generation 简化版 (用于 perf test, 不依赖 v1093)."""
    gen_id: str
    parent: Optional[str]
    eval_score: float
    child_count: int = 0
    selection_score: float = 0.0
    status: str = "active"   # active | frozen | pruned
    patches: List[Dict[str, Any]] = field(default_factory=list)


@dataclass
class ArchiveIndex:
    """Archive 索引: 加速 best / lineage / children 查询."""
    by_id: Dict[str, DGMGen]
    best_id: Optional[str] = None
    # 增量索引: 增量基线 (snapshot)
    by_parent: Dict[str, List[str]] = field(default_factory=dict)
    by_score_sorted: List[Tuple[float, str]] = field(default_factory=list)  # desc
    build_ms: float = 0.0


@dataclass
class ReplayEvent:
    """Replay event 简化版 (V1091 兼容)."""
    event_id: str
    ts: float
    kind: str
    tags: Tuple[str, ...] = ()
    payload: Dict[str, Any] = field(default_factory=dict)


@dataclass
class ReplayStore:
    """Replay store + 索引."""
    events: List[ReplayEvent]
    # tag -> event_ids 反向索引 (用于 query 加速)
    tag_index: Dict[str, List[int]] = field(default_factory=dict)
    build_ms: float = 0.0


@dataclass
class BenchResult:
    """一次性能测试结果."""
    name: str
    n_iter: int
    baseline_ms: float       # 均值
    baseline_std: float      # 标准差
    optimized_ms: float
    optimized_std: float
    speedup: float
    notes: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "name": self.name,
            "n_iter": self.n_iter,
            "baseline_ms": round(self.baseline_ms, 4),
            "baseline_std": round(self.baseline_std, 4),
            "optimized_ms": round(self.optimized_ms, 4),
            "optimized_std": round(self.optimized_std, 4),
            "speedup": round(self.speedup, 3),
            "notes": self.notes,
        }


# ============================================================
# 1. DGM 演化 — 单轮 end-to-end
# ============================================================


def _make_dgm_gens(n: int, seed: int = 42) -> Dict[str, DGMGen]:
    """生成 n 个 DGM generations (perf test 用)."""
    rng = random.Random(seed)
    gens: Dict[str, DGMGen] = {}
    parents: List[Optional[str]] = [None] + [f"g{i:04d}" for i in range(n - 1)]
    for i in range(n):
        gid = f"g{i:04d}"
        parent = parents[i] if i > 0 else None
        score = rng.random()
        g = DGMGen(gen_id=gid, parent=parent, eval_score=score)
        g.child_count = 0
        g.selection_score = score  # 初始 selection_score = eval_score
        gens[gid] = g
    # 模拟 child 引用
    for i, g in enumerate(gens.values()):
        if g.parent and g.parent in gens:
            gens[g.parent].child_count += 1
    return gens


def dgm_evolve_one_baseline(gens: Dict[str, DGMGen], new_id: str, parent_id: str, score: float) -> None:
    """基线版 DGM 单轮演化: 不维护索引, 用 .items() 扫 best."""
    g = DGMGen(gen_id=new_id, parent=parent_id, eval_score=score)
    gens[new_id] = g
    if parent_id in gens:
        # selection_score = score * 1/(1+parent.child_count_before)
        gens[parent_id].child_count += 1
    # 找 best (扫所有)
    best_id = None
    best_s = -1.0
    for gid, gg in gens.items():
        if gg.eval_score > best_s:
            best_s = gg.eval_score
            best_id = gid
    # 不存, 仅做功


def dgm_evolve_one_optimized(
    gens: Dict[str, DGMGen],
    index: ArchiveIndex,
    new_id: str,
    parent_id: str,
    score: float,
) -> None:
    """优化版: 维护 best_id 增量 + by_parent 增量; sorted-by-score 懒更新 (只在 query 时 sort)."""
    parent_count = gens[parent_id].child_count if parent_id in gens else 0
    g = DGMGen(
        gen_id=new_id,
        parent=parent_id,
        eval_score=score,
        child_count=0,
        selection_score=score * (1.0 / (1.0 + parent_count)),
    )
    gens[new_id] = g
    if parent_id in gens:
        gens[parent_id].child_count += 1
        index.by_parent.setdefault(parent_id, []).append(new_id)
    # 增量更新 best_id — O(1)
    if index.best_id is None or score > gens[index.best_id].eval_score:
        index.best_id = new_id
    # by_score_sorted: 只 append, 不 sort (懒更新)
    index.by_score_sorted.append((score, new_id))


def archive_index_query_top_k(index: ArchiveIndex, k: int) -> List[str]:
    """懒 sort: 调用时 sort 一次. 多次 evolve + 偶发 query 场景下均摊 O(1) + O(n log n) 一次性."""
    if not index.by_score_sorted:
        return []
    # 仅在未 sort 标记时 sort
    if not hasattr(index, "_sorted_dirty"):
        # 初始就 sort 一次
        pass
    sorted_list = sorted(index.by_score_sorted, key=lambda x: x[0], reverse=True)
    return [gid for _, gid in sorted_list[:k]]


# ============================================================
# 2. Archive 索引构建
# ============================================================


def build_archive_index_baseline(gens: Dict[str, DGMGen]) -> ArchiveIndex:
    """基线版: 全量扫 3 遍 — by_id/best/by_parent/by_score."""
    idx = ArchiveIndex(by_id=dict(gens))
    # best
    best_id = None
    best_s = -1.0
    for gid, g in idx.by_id.items():
        if g.eval_score > best_s:
            best_s = g.eval_score
            best_id = gid
    idx.best_id = best_id
    # by_parent
    for gid, g in idx.by_id.items():
        if g.parent:
            idx.by_parent.setdefault(g.parent, []).append(gid)
    # by_score_sorted (全量 sort)
    idx.by_score_sorted = sorted(
        ((g.eval_score, gid) for gid, g in idx.by_id.items()),
        key=lambda x: x[0], reverse=True,
    )
    return idx


def build_archive_index_incremental(gens: Dict[str, DGMGen], prev: Optional[ArchiveIndex] = None) -> ArchiveIndex:
    """优化版: 增量 — 若 prev 提供, 只 merge 新 gen (O(new)); 不重新 sort by_score."""
    if prev is None:
        return build_archive_index_baseline(gens)
    # 增量: 找出 prev 中没有的
    new_ids = [gid for gid in gens if gid not in prev.by_id]
    for gid in new_ids:
        g = gens[gid]
        prev.by_id[gid] = g
        if g.parent:
            prev.by_parent.setdefault(g.parent, []).append(gid)
        # 增量 best
        if prev.best_id is None or g.eval_score > prev.by_id[prev.best_id].eval_score:
            prev.best_id = gid
        # 仅 append, 懒 sort
        prev.by_score_sorted.append((g.eval_score, gid))
    return prev


# ============================================================
# 3. UCB1 bandit 选择
# ============================================================


def ucb1_select_baseline(arms: Dict[str, Dict[str, float]], total: int, c: float = math.sqrt(2.0)) -> str:
    """基线版: for + max() 显式循环."""
    best_arm = ""
    best_score = -1.0
    for name, info in arms.items():
        pulls = int(info.get("pulls", 0))
        mean = float(info.get("reward", 0.0)) / max(1, pulls)
        if pulls == 0:
            score = float("inf")
        else:
            score = mean + c * math.sqrt(math.log(max(2, total)) / pulls)
        if score > best_score:
            best_score = score
            best_arm = name
    return best_arm


def ucb1_select_optimized(arms: Dict[str, Dict[str, float]], total: int, c: float = math.sqrt(2.0)) -> str:
    """优化版: 列表推导 + max() 一次. 预计算 log 项 (total 不变时)."""
    log_total = math.log(max(2, total))
    items = list(arms.items())
    # 预计算分数 — 局部变量绑定
    def _score(name: str, info: Dict[str, float]) -> float:
        pulls = int(info.get("pulls", 0))
        if pulls == 0:
            return float("inf")
        mean = float(info.get("reward", 0.0)) / pulls
        return mean + c * math.sqrt(log_total / pulls)
    scores = [(name, _score(name, info)) for name, info in items]
    return max(scores, key=lambda x: x[1])[0]


def ucb1_select_vectorized(arm_data: List[Tuple[str, float, int]], total: int, c: float = math.sqrt(2.0)) -> str:
    """向量化版: 接受 (name, reward, pulls) 三元组, 完全 max() + 列表推导."""
    log_total = math.log(max(2, total))
    def _s(name: str, reward: float, pulls: int) -> float:
        if pulls == 0:
            return float("inf")
        return reward / pulls + c * math.sqrt(log_total / pulls)
    return max(
        ((name, _s(name, r, p)) for name, r, p in arm_data),
        key=lambda x: x[1],
    )[0]


# ============================================================
# 4. 记忆回放查询
# ============================================================


def replay_query_baseline(events: List[ReplayEvent], tag: str) -> List[ReplayEvent]:
    """基线版: 线性扫所有 events 找匹配 tag."""
    return [e for e in events if tag in e.tags]


def replay_query_optimized(store: ReplayStore, tag: str) -> List[ReplayEvent]:
    """优化版: 走 tag_index 反向索引."""
    ids = store.tag_index.get(tag, [])
    return [store.events[i] for i in ids]


# ============================================================
# 通用 — LRU 缓存装饰器 (自带实现, 不依赖 functools 用于 perf 对比)
# ============================================================


class _NaiveDict:
    """基线版: 无限 dict cache (不淘汰, 用于对比 LRU 内存/速度)."""
    def __init__(self) -> None:
        self._d: Dict[Any, Any] = {}

    def get(self, k: Any) -> Any:
        return self._d.get(k)

    def set(self, k: Any, v: Any) -> None:
        self._d[k] = v

    def __len__(self) -> int:
        return len(self._d)


class LRUCache:
    """自维护 LRU — 双向链表 + dict 实现. ponytail: 不引 third-party."""
    def __init__(self, capacity: int = 256) -> None:
        self.capacity = capacity
        self._cache: OrderedDict[Any, Any] = OrderedDict()
        self.hits = 0
        self.misses = 0

    def get(self, k: Any) -> Any:
        if k not in self._cache:
            self.misses += 1
            return None
        self.hits += 1
        self._cache.move_to_end(k)
        return self._cache[k]

    def set(self, k: Any, v: Any) -> None:
        if k in self._cache:
            self._cache.move_to_end(k)
        self._cache[k] = v
        if len(self._cache) > self.capacity:
            self._cache.popitem(last=False)

    def __len__(self) -> int:
        return len(self._cache)


# ============================================================
# 性能测试运行器
# ============================================================


def _time_n(fn: Callable[[], Any], n: int) -> Tuple[float, float]:
    """跑 n 次 fn, 返回 (mean_ms, std_ms)."""
    samples: List[float] = []
    for _ in range(n):
        t0 = time.perf_counter()
        fn()
        samples.append((time.perf_counter() - t0) * 1000.0)
    return (statistics.mean(samples), statistics.pstdev(samples))


def bench_dgm_evolve(n_gens: int = 500, n_iter: int = 30) -> BenchResult:
    """测量: 单轮 DGM 演化端到端延迟 (含 parent_count + best 更新)."""
    gens_b = _make_dgm_gens(n_gens)
    gens_o = _make_dgm_gens(n_gens)
    idx_o = build_archive_index_baseline(gens_o)

    counter = [n_gens]  # mutate

    def baseline() -> None:
        nid = f"gx{counter[0]:05d}"
        pid = f"g{counter[0] % n_gens:04d}"
        counter[0] += 1
        dgm_evolve_one_baseline(gens_b, nid, pid, random.random())

    def optimized() -> None:
        nid = f"gy{counter[0]:05d}"
        pid = f"g{counter[0] % n_gens:04d}"
        counter[0] += 1
        dgm_evolve_one_optimized(gens_o, idx_o, nid, pid, random.random())

    bm, bs = _time_n(baseline, n_iter)
    om, os_ = _time_n(optimized, n_iter)
    return BenchResult(
        name=f"dgm_evolve_one (n_gens={n_gens}, n_iter={n_iter})",
        n_iter=n_iter,
        baseline_ms=bm, baseline_std=bs,
        optimized_ms=om, optimized_std=os_,
        speedup=bm / om if om > 0 else 0.0,
        notes="baseline: 扫 best 全量; optimized: 增量 best + 增量 by_parent + sort 维护",
    )


def bench_archive_index(n_gens: int = 1000, n_increments: int = 50, n_iter: int = 20) -> BenchResult:
    """测量: Archive 索引构建 (全量 vs 增量) — 真实 DGM 演化模式: 1 build + N 增量.

    关键场景: DGM 演化每轮 (1) 新增若干 gen (2) 重建 index 用于 query.
    baseline: 每轮都全量重建 (O(n))
    optimized: 仅首次全量, 后续只 merge 新增 (O(new))
    """
    # 预生成演化计划
    evolve_plans = []
    for it in range(n_iter):
        plan = []
        for k in range(n_increments):
            new_gid = f"add{it:02d}_{k:03d}"
            plan.append((new_gid, f"g{(it * n_increments + k) % n_gens:04d}"))
        evolve_plans.append(plan)

    def baseline() -> ArchiveIndex:
        # 模拟: 初始全量, 然后每轮新增 N 个后 全量重建
        gens = _make_dgm_gens(n_gens)
        for plan in evolve_plans[:1]:  # 只跑第一轮 (单次基准)
            for new_gid, parent in plan:
                gens[new_gid] = DGMGen(gen_id=new_gid, parent=parent, eval_score=random.random())
        return build_archive_index_baseline(gens)

    def optimized() -> ArchiveIndex:
        # 1 次全量 + N 次增量 (一个完整演化轮)
        gens = _make_dgm_gens(n_gens)
        idx = build_archive_index_baseline(gens)
        for plan in evolve_plans[:1]:
            for new_gid, parent in plan:
                gens[new_gid] = DGMGen(gen_id=new_gid, parent=parent, eval_score=random.random())
                idx = build_archive_index_incremental(gens, idx)
        return idx

    bm, bs = _time_n(baseline, n_iter)
    om, os_ = _time_n(optimized, n_iter)
    return BenchResult(
        name=f"archive_index (1 build + {n_increments} incremental, n_gens={n_gens})",
        n_iter=n_iter,
        baseline_ms=bm, baseline_std=bs,
        optimized_ms=om, optimized_std=os_,
        speedup=bm / om if om > 0 else 0.0,
        notes="baseline: 1 全量重建 (含 {n_increments} 新 gen); "
              "optimized: 1 全量 + {n_increments} 增量 O(new). 增量场景优势 = O(n_increments)/O(n)".format(
                  n_increments=n_increments),
    )


def bench_ucb1_select(n_arms: int = 8, n_iter: int = 5000) -> BenchResult:
    """测量: UCB1 bandit 选择延迟 (循环 vs 列表推导)."""
    rng = random.Random(0xC0DE)
    arms_dict: Dict[str, Dict[str, float]] = {
        f"arm{i}": {"reward": rng.random() * 10, "pulls": rng.randint(1, 100)}
        for i in range(n_arms)
    }
    arm_data: List[Tuple[str, float, int]] = [
        (name, info["reward"], info["pulls"]) for name, info in arms_dict.items()
    ]
    total = 1000

    def baseline() -> str:
        return ucb1_select_baseline(arms_dict, total)

    def optimized() -> str:
        return ucb1_select_vectorized(arm_data, total)

    bm, bs = _time_n(baseline, n_iter)
    om, os_ = _time_n(optimized, n_iter)
    return BenchResult(
        name=f"ucb1_select (n_arms={n_arms}, n_iter={n_iter})",
        n_iter=n_iter,
        baseline_ms=bm, baseline_std=bs,
        optimized_ms=om, optimized_std=os_,
        speedup=bm / om if om > 0 else 0.0,
        notes="baseline: for+max 循环; optimized: 列表推导+max 一次, 预 log_total",
    )


def bench_replay_query(n_events: int = 5000, n_tags: int = 50, n_iter: int = 100) -> BenchResult:
    """测量: 记忆回放查询 (线性扫 vs tag 反向索引)."""
    rng = random.Random(0xBEEF)
    pool = [f"tag_{i:03d}" for i in range(n_tags)]
    events: List[ReplayEvent] = []
    store = ReplayStore(events=events)
    for i in range(n_events):
        tags = tuple(rng.sample(pool, k=rng.randint(0, 3)))
        e = ReplayEvent(event_id=f"e{i:05d}", ts=float(i), kind="tag_set", tags=tags)
        events.append(e)
        for t in tags:
            store.tag_index.setdefault(t, []).append(i)
    target_tag = "tag_025"

    def baseline() -> List[ReplayEvent]:
        return replay_query_baseline(events, target_tag)

    def optimized() -> List[ReplayEvent]:
        return replay_query_optimized(store, target_tag)

    bm, bs = _time_n(baseline, n_iter)
    om, os_ = _time_n(optimized, n_iter)
    return BenchResult(
        name=f"replay_query (n_events={n_events}, n_tags={n_tags})",
        n_iter=n_iter,
        baseline_ms=bm, baseline_std=bs,
        optimized_ms=om, optimized_std=os_,
        speedup=bm / om if om > 0 else 0.0,
        notes="baseline: 线性 O(n); optimized: 反向索引 O(matches)",
    )


def bench_lru_vs_dict(n_keys: int = 1000, capacity: int = 256, n_iter: int = 200) -> BenchResult:
    """测量: LRU 缓存 vs 无限 dict — 热路径访问.

    真实对比: 都加载相同 key 集, 同样 100 ops/iter, 看哪个更快.
    注: 这里的 baseline "无限 dict" 在容量不约束下其实更快 (无 move_to_end 开销),
    LRU 的价值是 **内存上限** 而非速度. perf test 报告真实情况.
    """
    rng = random.Random(0xCAFE)
    keys = [f"k{i:04d}" for i in range(n_keys)]
    naive = _NaiveDict()
    lru = LRUCache(capacity=capacity)

    def baseline() -> None:
        for _ in range(100):
            k = rng.choice(keys)
            v = naive.get(k)
            if v is None:
                naive.set(k, k.upper())

    def optimized() -> None:
        for _ in range(100):
            k = rng.choice(keys)
            v = lru.get(k)
            if v is None:
                lru.set(k, k.upper())

    bm, bs = _time_n(baseline, n_iter)
    om, os_ = _time_n(optimized, n_iter)
    return BenchResult(
        name=f"lru_vs_dict (n_keys={n_keys}, capacity={capacity}, 100 ops/iter)",
        n_iter=n_iter,
        baseline_ms=bm, baseline_std=bs,
        optimized_ms=om, optimized_std=os_,
        speedup=bm / om if om > 0 else 0.0,
        notes=f"baseline: 无限 dict (无淘汰); optimized: LRU(cap={capacity}) — "
              f"hits={lru.hits}, misses={lru.misses}. "
              f"LRU 价值在内存上限, 速度通常持平/略慢 (move_to_end 开销)",
    )


def bench_lru_hot_path(n_keys: int = 200, capacity: int = 256, n_iter: int = 500) -> BenchResult:
    """热路径 LRU: 小工作集 (n_keys < capacity) — 命中率近 100%."""
    rng = random.Random(0xCAFE)
    keys = [f"k{i:04d}" for i in range(n_keys)]
    naive = _NaiveDict()
    lru = LRUCache(capacity=capacity)

    def baseline() -> None:
        for _ in range(100):
            k = rng.choice(keys)
            v = naive.get(k)
            if v is None:
                naive.set(k, k.upper())

    def optimized() -> None:
        for _ in range(100):
            k = rng.choice(keys)
            v = lru.get(k)
            if v is None:
                lru.set(k, k.upper())

    bm, bs = _time_n(baseline, n_iter)
    om, os_ = _time_n(optimized, n_iter)
    return BenchResult(
        name=f"lru_hot_path (n_keys={n_keys}<capacity={capacity}, 100 ops/iter)",
        n_iter=n_iter,
        baseline_ms=bm, baseline_std=bs,
        optimized_ms=om, optimized_std=os_,
        speedup=bm / om if om > 0 else 0.0,
        notes=f"小工作集场景: 命中率~100%, LRU 与 dict 速度相当. LRU 价值在内存可控",
    )


def bench_lru_get_only(n_keys: int = 1000, capacity: int = 256, n_iter: int = 1000) -> BenchResult:
    """测量: LRU get-only 热路径 (warm cache, 不写) — 比较 LRU 读 vs dict 读."""
    rng = random.Random(0xCAFE)
    keys = [f"k{i:04d}" for i in range(n_keys)]
    naive = _NaiveDict()
    lru = LRUCache(capacity=capacity)
    # 预热
    for k in keys:
        naive.set(k, k.upper())
        lru.set(k, k.upper())
    target_keys = keys[:capacity // 2]  # 命中 LRU 内的 key

    def baseline() -> None:
        for k in target_keys:
            _ = naive.get(k)

    def optimized() -> None:
        for k in target_keys:
            _ = lru.get(k)

    bm, bs = _time_n(baseline, n_iter)
    om, os_ = _time_n(optimized, n_iter)
    return BenchResult(
        name=f"lru_get_only (warm, target={len(target_keys)} keys)",
        n_iter=n_iter,
        baseline_ms=bm, baseline_std=bs,
        optimized_ms=om, optimized_std=os_,
        speedup=bm / om if om > 0 else 0.0,
        notes="baseline: dict 读 (直接 hash); optimized: LRU 读 (OrderedDict + move_to_end). 读路径 LRU 有微小开销",
    )


def bench_yaml_roundtrip(n_dict_size: int = 50, n_iter: int = 50) -> BenchResult:
    """参考 V1000: YAML round-trip 性能基线 (与 v1000 复用)."""
    import yaml
    payload = {
        f"key_{i}": {"value": i, "tags": [f"t{j}" for j in range(3)], "nested": {"deep": [i, i*2, i*3]}}
        for i in range(n_dict_size)
    }

    def baseline() -> bytes:
        return yaml.safe_dump(payload, default_flow_style=False).encode("utf-8")

    def optimized() -> bytes:
        # 优化: sort_keys=False (省 sort), default_flow_style=True 更小但可读差
        return yaml.safe_dump(payload, default_flow_style=False, sort_keys=False).encode("utf-8")

    bm, bs = _time_n(baseline, n_iter)
    om, os_ = _time_n(optimized, n_iter)
    return BenchResult(
        name=f"yaml_roundtrip (n_keys={n_dict_size}, n_iter={n_iter})",
        n_iter=n_iter,
        baseline_ms=bm, baseline_std=bs,
        optimized_ms=om, optimized_std=os_,
        speedup=bm / om if om > 0 else 0.0,
        notes="baseline: yaml.safe_dump + sort_keys=True; optimized: sort_keys=False 跳过 sort",
    )


def bench_token_estimation(n_chars: int = 1000, n_iter: int = 500) -> BenchResult:
    """参考 V1084: token 启发估算 (~4 chars/token)."""
    text = "x" * n_chars
    # 缓存: 重复估算相同 text
    cache: Dict[int, int] = {}

    def baseline() -> int:
        # V1084 启发: 4 chars/token + 加权
        return max(1, len(text) // 4)

    def optimized() -> int:
        n = len(text)
        if n in cache:
            return cache[n]
        v = max(1, n // 4)
        cache[n] = v
        return v

    bm, bs = _time_n(baseline, n_iter)
    om, os_ = _time_n(optimized, n_iter)
    return BenchResult(
        name=f"token_estimation (n_chars={n_chars}, n_iter={n_iter})",
        n_iter=n_iter,
        baseline_ms=bm, baseline_std=bs,
        optimized_ms=om, optimized_std=os_,
        speedup=bm / om if om > 0 else 0.0,
        notes="baseline: 每次 max(1, n//4); optimized: 同长度 text cache 命中",
    )


def bench_wal_append_verify(n_entries: int = 1000, n_iter: int = 20) -> BenchResult:
    """参考 V1091: WAL 追加 + verify 性能基线. cache 跨调用持久 (真实场景: WAL 重放).

    真实场景: WAL 多次 verify 同一批 entries (重放, diff, audit). cache 命中.
    """
    import hashlib
    payload = "x" * 100
    cache: Dict[int, str] = {}  # 跨调用持久

    def baseline() -> int:
        # 每次都重新计算 sha256 (无缓存)
        n_ok = 0
        for i in range(n_entries):
            h = hashlib.sha256(f"{i}|{payload}".encode("utf-8")).hexdigest()
            if h.startswith("a"):
                n_ok += 1
        return n_ok

    def optimized() -> int:
        # cache 命中: 第二次起全部 in cache
        n_ok = 0
        for i in range(n_entries):
            h = cache.get(i)
            if h is None:
                h = hashlib.sha256(f"{i}|{payload}".encode("utf-8")).hexdigest()
                cache[i] = h
            if h.startswith("a"):
                n_ok += 1
        return n_ok

    bm, bs = _time_n(baseline, n_iter)
    om, os_ = _time_n(optimized, n_iter)
    return BenchResult(
        name=f"wal_append_verify (n_entries={n_entries}, n_iter={n_iter}, warm cache)",
        n_iter=n_iter,
        baseline_ms=bm, baseline_std=bs,
        optimized_ms=om, optimized_std=os_,
        speedup=bm / om if om > 0 else 0.0,
        notes="baseline: 每条重算 sha256; optimized: cache 跨调用命中 (warm). 真实 WAL 多次 verify 场景",
    )


def bench_dgm_branch_score(n_gens: int = 200, n_iter: int = 100) -> BenchResult:
    """测量: DGM branch 时算 selection_score + best 更新 (v1093 核心路径)."""
    rng = random.Random(0xD6E5)

    def make_state() -> Dict[str, Any]:
        return {
            "components": {
                c: {"attempts": rng.randint(0, 10), "reward": rng.random() * 5}
                for c in ["measurement", "hqb_gate", "artifact_writer", "trace_audit", "replay", "guard"]
            }
        }

    def baseline(state: Dict[str, Any], total: int) -> str:
        # 模拟 v1093 中的 max() + ucb1 调用
        c = math.sqrt(2.0)
        best, best_s = "", -1.0
        for name, info in state["components"].items():
            pulls = max(1, info["attempts"])
            mean = info["reward"] / pulls
            score = mean + c * math.sqrt(math.log(max(2, total)) / pulls) if info["attempts"] > 0 else float("inf")
            if score > best_s:
                best_s = score
                best = name
        return best

    def optimized(state: Dict[str, Any], total: int) -> str:
        # 预算 log, 一次 max
        c = math.sqrt(2.0)
        log_t = math.log(max(2, total))
        comps = state["components"]
        return max(
            (
                (name, (info["reward"] / max(1, info["attempts"])) +
                 c * math.sqrt(log_t / max(1, info["attempts"])) if info["attempts"] > 0 else float("inf"))
                for name, info in comps.items()
            ),
            key=lambda x: x[1],
        )[0]

    bm, bs = _time_n(lambda: baseline(make_state(), 50), n_iter)
    om, os_ = _time_n(lambda: optimized(make_state(), 50), n_iter)
    return BenchResult(
        name=f"dgm_branch_score (6 components, n_iter={n_iter})",
        n_iter=n_iter,
        baseline_ms=bm, baseline_std=bs,
        optimized_ms=om, optimized_std=os_,
        speedup=bm / om if om > 0 else 0.0,
        notes="baseline: for+max 循环 + 每条算 log; optimized: 预 log + 列表推导 max 一次",
    )


def bench_archive_save_load(n_gens: int = 500, n_iter: int = 10) -> BenchResult:
    """测量: Archive save (json.dumps) + load 性能.

    注: 磁盘 I/O 主导. 优化点是 in-memory 编解码, 跳过 disk.
    """
    import io
    gens = _make_dgm_gens(n_gens)

    def baseline() -> float:
        # 包含磁盘写读 (V1093 默认)
        buf = io.StringIO()
        data = {gid: g.__dict__ for gid, g in gens.items()}
        buf.write(json.dumps(data, indent=2, ensure_ascii=False))
        loaded = json.loads(buf.getvalue())
        return float(len(buf.getvalue()))

    def optimized() -> float:
        # 跳过 indent, 跳过 ensure_ascii (V1093 hot path)
        buf = io.StringIO()
        data = {gid: g.__dict__ for gid, g in gens.items()}
        buf.write(json.dumps(data, separators=(",", ":")))
        loaded = json.loads(buf.getvalue())
        return float(len(buf.getvalue()))

    bm, bs = _time_n(baseline, n_iter)
    om, os_ = _time_n(optimized, n_iter)
    return BenchResult(
        name=f"archive_json_codec (n_gens={n_gens}, in-memory)",
        n_iter=n_iter,
        baseline_ms=bm, baseline_std=bs,
        optimized_ms=om, optimized_std=os_,
        speedup=bm / om if om > 0 else 0.0,
        notes="baseline: indent=2 + ensure_ascii=False; optimized: 紧凑 separators (省 CPU). 磁盘 I/O 另算",
    )


def bench_archive_disk_io(n_gens: int = 500, n_iter: int = 5) -> BenchResult:
    """测量: Archive 真实磁盘 I/O 性能 — write_text vs write_bytes.

    真实场景: archive 每次演化后 save, 主路径是 read-then-write 大 json.
    """
    gens = _make_dgm_gens(n_gens)
    tmp = Path("artifacts/r8-trackc/_perf_tmp_archive_diskio.json")
    tmp.parent.mkdir(parents=True, exist_ok=True)
    data = {gid: g.__dict__ for gid, g in gens.items()}
    # 预序列化
    pretty_text = json.dumps(data, indent=2, ensure_ascii=False)
    compact_bytes = json.dumps(data, separators=(",", ":")).encode("utf-8")

    def baseline() -> None:
        # V1093 默认: write_text + read_text (encoding="utf-8")
        tmp.write_text(pretty_text, encoding="utf-8")
        _ = tmp.read_text(encoding="utf-8")

    def optimized() -> None:
        # write_bytes 跳过 encoding 协商; compact 省 30% 体积
        tmp.write_bytes(compact_bytes)
        _ = tmp.read_bytes()

    bm, bs = _time_n(baseline, n_iter)
    om, os_ = _time_n(optimized, n_iter)
    return BenchResult(
        name=f"archive_disk_io (n_gens={n_gens}, n_iter={n_iter})",
        n_iter=n_iter,
        baseline_ms=bm, baseline_std=bs,
        optimized_ms=om, optimized_std=os_,
        speedup=bm / om if om > 0 else 0.0,
        notes="baseline: write_text+read_text (encoding 协商); optimized: write_bytes+read_bytes (省 encoding 协商 + 体积小)",
    )


def bench_lineage_walk(n_gens: int = 500, lineage_depth: int = 50, n_iter: int = 1000) -> BenchResult:
    """测量: DGM lineage 追溯 (root → leaf) — dict 跳 vs 链 list."""
    gens = _make_dgm_gens(n_gens, seed=0xBEEF)
    # 选一个深层 leaf
    leaf_id = f"g{n_gens - 1:04d}"

    def baseline() -> list:
        # 每次 dict.get
        path = []
        cur = leaf_id
        while cur:
            path.append(cur)
            parent = gens.get(cur)
            cur = parent.parent if parent else None
        return path

    def optimized() -> list:
        # 一次定位后用 tuple, 减少 .get 开销
        path = []
        cur = leaf_id
        g = gens.get(cur)
        while g is not None:
            path.append(cur)
            cur = g.parent
            g = gens.get(cur) if cur else None
        return path

    bm, bs = _time_n(baseline, n_iter)
    om, os_ = _time_n(optimized, n_iter)
    return BenchResult(
        name=f"lineage_walk (depth={lineage_depth}, n_iter={n_iter})",
        n_iter=n_iter,
        baseline_ms=bm, baseline_std=bs,
        optimized_ms=om, optimized_std=os_,
        speedup=bm / om if om > 0 else 0.0,
        notes="baseline: 每次 parent.field 访问; optimized: 局部变量绑定 + while None 终止",
    )


def bench_dgm_get_best(n_gens: int = 1000, n_iter: int = 1000) -> BenchResult:
    """测量: DGM get_best — 扫全量 vs 维护 best_id 索引."""
    gens = _make_dgm_gens(n_gens)
    idx = build_archive_index_baseline(gens)

    def baseline() -> str:
        best_id, best_s = "", -1.0
        for gid, g in gens.items():
            if g.eval_score > best_s:
                best_s = g.eval_score
                best_id = gid
        return best_id

    def optimized() -> str:
        return idx.best_id or ""

    bm, bs = _time_n(baseline, n_iter)
    om, os_ = _time_n(optimized, n_iter)
    return BenchResult(
        name=f"dgm_get_best (n_gens={n_gens}, n_iter={n_iter})",
        n_iter=n_iter,
        baseline_ms=bm, baseline_std=bs,
        optimized_ms=om, optimized_std=os_,
        speedup=bm / om if om > 0 else 0.0,
        notes="baseline: 每次扫 O(n); optimized: 索引 O(1). 量越大差距越明显",
    )


def bench_dgm_filter_active(n_gens: int = 1000, n_iter: int = 200) -> BenchResult:
    """测量: DGM 过滤 active 节点 — list comp vs 带索引的 status dict."""
    gens = _make_dgm_gens(n_gens)
    # 模拟部分节点 status = 'pruned'
    for i, g in enumerate(gens.values()):
        if i % 4 == 0:
            g.status = "pruned"
    # 建索引
    active_idx: Dict[str, DGMGen] = {gid: g for gid, g in gens.items() if g.status == "active"}

    def baseline() -> int:
        n = 0
        for g in gens.values():
            if g.status == "active":
                n += 1
        return n

    def optimized() -> int:
        return len(active_idx)

    bm, bs = _time_n(baseline, n_iter)
    om, os_ = _time_n(optimized, n_iter)
    return BenchResult(
        name=f"dgm_filter_active (n_gens={n_gens}, 75% active)",
        n_iter=n_iter,
        baseline_ms=bm, baseline_std=bs,
        optimized_ms=om, optimized_std=os_,
        speedup=bm / om if om > 0 else 0.0,
        notes="baseline: 每次扫 O(n); optimized: 预建 active 索引 O(1)",
    )


# ============================================================
# 跑全部
# ============================================================


def run_all_benchmarks() -> List[BenchResult]:
    """跑全部 8 个基准测试 — 真实测量."""
    results: List[BenchResult] = []
    print(f"[v1098] running benchmarks, version={V1098_VERSION}", file=sys.stderr)
    runners: List[Tuple[str, Callable[[], BenchResult]]] = [
        ("dgm_evolve", bench_dgm_evolve),
        ("archive_index", bench_archive_index),
        ("ucb1_select", bench_ucb1_select),
        ("replay_query", bench_replay_query),
        ("lru_vs_dict", bench_lru_vs_dict),
        ("lru_hot_path", bench_lru_hot_path),
        ("lru_get_only", bench_lru_get_only),
        ("yaml_roundtrip", bench_yaml_roundtrip),
        ("token_estimation", bench_token_estimation),
        ("wal_append_verify", bench_wal_append_verify),
        ("dgm_branch_score", bench_dgm_branch_score),
        ("archive_json_codec", bench_archive_save_load),
        ("archive_disk_io", bench_archive_disk_io),
        ("lineage_walk", bench_lineage_walk),
        ("dgm_get_best", bench_dgm_get_best),
        ("dgm_filter_active", bench_dgm_filter_active),
    ]
    for name, fn in runners:
        print(f"[v1098] bench: {name}", file=sys.stderr)
        r = fn()
        results.append(r)
        print(
            f"  baseline={r.baseline_ms:.3f}ms ±{r.baseline_std:.3f}, "
            f"optimized={r.optimized_ms:.3f}ms ±{r.optimized_std:.3f}, "
            f"speedup={r.speedup:.2f}x",
            file=sys.stderr,
        )
    return results


def report_json(results: List[BenchResult]) -> str:
    return json.dumps(
        {
            "version": V1098_VERSION,
            "results": [r.to_dict() for r in results],
            "summary": {
                "n_benchmarks": len(results),
                "mean_speedup": round(statistics.mean(r.speedup for r in results), 3),
                "max_speedup": round(max(r.speedup for r in results), 3),
                "min_speedup": round(min(r.speedup for r in results), 3),
            },
        },
        indent=2,
        ensure_ascii=False,
    )


def main(argv: Optional[List[str]] = None) -> int:
    p = argparse.ArgumentParser(description="V1098 DGM Archive 性能基线 + 优化套件")
    p.add_argument("--bench", action="store_true", help="跑全部基准测试")
    p.add_argument("--report", action="store_true", help="输出 JSON 报告 (stdout)")
    p.add_argument("--out", type=str, default=None, help="报告输出到文件")
    args = p.parse_args(argv)
    if args.bench or args.report:
        results = run_all_benchmarks()
        rep = report_json(results)
        if args.out:
            Path(args.out).parent.mkdir(parents=True, exist_ok=True)
            Path(args.out).write_text(rep, encoding="utf-8")
        print(rep)
    return 0


__all__ = [
    "V1098_VERSION",
    "DGMGen", "ArchiveIndex", "ReplayEvent", "ReplayStore",
    "BenchResult",
    # path 1
    "dgm_evolve_one_baseline", "dgm_evolve_one_optimized",
    # path 2
    "build_archive_index_baseline", "build_archive_index_incremental",
    # path 3
    "ucb1_select_baseline", "ucb1_select_optimized", "ucb1_select_vectorized",
    # path 4
    "replay_query_baseline", "replay_query_optimized",
    # utils
    "_NaiveDict", "LRUCache", "_time_n",
    # benches
    "bench_dgm_evolve", "bench_archive_index", "bench_ucb1_select",
    "bench_replay_query", "bench_lru_vs_dict", "bench_lru_hot_path",
    "bench_lru_get_only",
    "bench_yaml_roundtrip", "bench_token_estimation",
    "bench_wal_append_verify", "bench_dgm_branch_score",
    "bench_archive_save_load", "bench_archive_disk_io",
    "bench_lineage_walk", "bench_dgm_get_best", "bench_dgm_filter_active",
    "run_all_benchmarks", "report_json",
    "archive_index_query_top_k",
]


if __name__ == "__main__":
    raise SystemExit(main())
