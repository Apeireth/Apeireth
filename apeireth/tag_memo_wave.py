"""Phase 53 VCP TagMemo 浪潮算法 Python 复刻.

主 23:18 + 23:50 抓紧干: 真生产 VCP TagMemo 借鉴 + Python 实现.
借鉴 VCP TagMemoEngine.js (主 23:18 真研究 82KB 真读源码).

主 23:10 真研究代码 + 主 23:18 VCP 真研究 + 主 22:40 自决 + 主 17:43 实事求是.

VCP TagMemo 浪潮算法 (V7.1 / V8 / V8.2 / V8.3):
  V7.1 - 短矩阵增量更新 (accumulatedTagChanges + accumulatedNewTagIds)
  V8 - 能量场缓存 (lastEnergyField)
  V8.2 - 持久化 Tag 对语义距离 (tagPairSimilarities Map)
  V8.3 - 阈值触发增量 (intrinsic_residual recompute on threshold)

主 17:43 不假装 Phenomenal, 仅 engineering approximation.
主 17:58 Phenomenal 终极目标, 不是已达成.

Karpathy 准则:
  1. Think Before Coding: 借鉴算法, 不模仿代码
  2. Simplicity First: 简单 Wave + Residual + EnergyField
  3. Surgical Changes: 不改 Memory3-Tier
  4. Goal-Driven Execution: 可验证
"""
from __future__ import annotations

import math
import time
import uuid
from collections import deque
from dataclasses import dataclass, field, asdict
from typing import Optional, Dict, List


TAG_MEMO_WAVE_VERSION = "0.1.0"


@dataclass
class TagNode:
    """Tag 节点 — VCP TagMemo 浪潮算法的基本单位."""
    tag_id: str
    label: str
    weight: float = 1.0
    residual: float = 0.0           # intrinsic residual (V8.2)
    first_seen: float = field(default_factory=time.time)
    last_seen: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class TagPair:
    """Tag 对 — V8.2 持久化语义距离."""
    a: str
    b: str
    similarity: float = 0.0         # 0-1 cosine-like
    last_updated: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


class TagMemoWave:
    """VCP TagMemo 浪潮算法 Python 复刻 — 主 23:18 真研究真生产.

    借鉴 VCP TagMemoEngine.js 真生产 (82KB / 1810 lines).
    不复制 VCP 代码, 用 Python 真生产模式.
    """

    def __init__(self, threshold: float = 0.5, matrix_rebuild_quiet_ms: int = 300000):
        self.tags: Dict[str, TagNode] = {}
        self.pairs: Dict[str, TagPair] = {}
        self.energy_field: float = 0.0       # V8 能量场缓存
        self.last_energy_field: Optional[float] = None
        self.accumulated_tag_changes: int = 0  # V7.1 增量更新
        self.accumulated_new_tag_ids: set = set()
        self._matrix_rebuild_quiet_ms = matrix_rebuild_quiet_ms
        self.threshold = threshold
        self.history: list = []

    def observe_tag(self, label: str) -> TagNode:
        """观察一个 tag — 增量更新."""
        now = time.time()
        if label not in self.tags:
            tid = uuid.uuid4().hex[:12]
            node = TagNode(tag_id=tid, label=label, first_seen=now, last_seen=now)
            self.tags[label] = node
            self.accumulated_new_tag_ids.add(label)
            self.accumulated_tag_changes += 1
        else:
            self.tags[label].last_seen = now
            self.accumulated_tag_changes += 1
        return self.tags[label]

    def cooccurrence(self, tag_a: str, tag_b: str, weight: float = 1.0):
        """记 co-occurrence 一次 — 更新 tag pair 相似度."""
        self.observe_tag(tag_a)
        self.observe_tag(tag_b)
        key = self._pair_key(tag_a, tag_b)
        if key not in self.pairs:
            self.pairs[key] = TagPair(a=tag_a, b=tag_b, similarity=0.0, last_updated=time.time())
        p = self.pairs[key]
        # 简化的 VCP-style 相似度更新: 增量加权
        old = p.similarity
        p.similarity = min(1.0, old + weight * 0.1)
        p.last_updated = time.time()

    def _pair_key(self, a: str, b: str) -> str:
        return f"{min(a,b)}::{max(a,b)}"

    def pair_similarity(self, a: str, b: str) -> float:
        return self.pairs.get(self._pair_key(a, b), TagPair(a, b, 0.0)).similarity

    def should_rebuild_matrix(self) -> bool:
        """V7.1 短矩阵增量更新: 阈值触发."""
        return self.accumulated_tag_changes >= self.threshold * 10

    def rebuild_matrix(self):
        """V8.2 真生产: 重建矩阵 + 能量场缓存."""
        if not self.tags:
            return
        # 简化: 平均 residual = sum(weight) / count
        n = len(self.tags)
        if n == 0:
            return
        total = sum(t.weight for t in self.tags.values())
        for t in self.tags.values():
            t.residual = t.weight / total
        # 能量场
        self.last_energy_field = self.energy_field
        self.energy_field = -math.log(1.0 / n) if n > 1 else 0.0
        # 重置
        self.accumulated_tag_changes = 0
        self.accumulated_new_tag_ids.clear()
        self.history.append({
            "ts": time.time(),
            "n_tags": n,
            "n_pairs": len(self.pairs),
            "energy_field": self.energy_field,
        })

    def tag_pair_observation(self) -> list:
        """V8.2 观测: 返回所有 tag pair 相似度."""
        return [p.to_dict() for p in self.pairs.values()]

    def stats(self) -> dict:
        return {
            "version": TAG_MEMO_WAVE_VERSION,
            "n_tags": len(self.tags),
            "n_pairs": len(self.pairs),
            "accumulated_changes": self.accumulated_tag_changes,
            "energy_field": self.energy_field,
            "last_energy_field": self.last_energy_field,
            "philosophy_isomorphy": (
                "VCP TagMemo 浪潮算法 Python 复刻, "
                "**借鉴模式不模仿代码**, 主 23:18 真研究"
            ),
        }


__all__ = ["TAG_MEMO_WAVE_VERSION", "TagNode", "TagPair", "TagMemoWave"]