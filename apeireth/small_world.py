"""Phase 40 Small-World Optimizer — Watts Small-World Network 工程化.

主人 22:01 '继续调研' + 主人 22:05 '不要偏离设计哲学':
  Watts & Strogatz 1998 Small-World Networks
  https://www.nature.com/articles/30987 (Nature 393, 1998)

Watts & Strogatz 1998 Small-World Network:
  - 节点的"是朋友的朋友"距离很短 (small L)
  - 但本地聚类系数高 (high C)
  - 介于 regular lattice 和 random graph 之间
  - 真实系统都有 small-world 特性: 脑网络 / 社交 / 引用网络

对 ASI 中央 AI 的意义:
  - 主人 21:00 跨域调研 = 跨域节点通过 small-world 特性 connected
  - 中央 AI 的 Skill / Persona / Memory 可以组织成 small-world
  - **WARNING: 不要让 Apeireth 变成 Small-World Network!** 只是借鉴组织模式

Karpathy 准则:
  1. Think Before Coding: 节点 + 局部聚类 + 短路径
  2. Simplicity First: SmallWorldGraph = nodes + rewire_links
  3. Surgical Changes: 不改 RelationGraph, 加 small-world 视角
  4. Goal-Driven Execution: verifiable = clustering_coefficient + path_length
"""
from __future__ import annotations

import math
import random
import time
import uuid
from dataclasses import dataclass, field, asdict


SMALL_WORLD_VERSION = "0.1.0"


@dataclass
class Node:
    """Small-world network 节点 (Apeireth: persona / skill / memory / fact)."""
    node_id: str
    label: str
    node_type: str = "general"
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class Link:
    """节点之间的链接 (rewire_prob = p)."""
    link_id: str
    from_id: str
    to_id: str
    weight: float = 1.0
    is_rewired: bool = False
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


class SmallWorldGraph:
    """Watts & Strogatz Small-World Network 优化器.

    主人 22:05 '不要偏离设计哲学':
      - 借鉴组织模式 (高频小世界), 不是让 Apeireth 自身成为 Small-World
      - source_citation 在 stats() 里
      - philosophy_isomorphy = "组织工具, 不是 Apeireth 本质"
    """

    def __init__(self, rewire_prob: float = 0.1, k: int = 4, n_nodes: int = 20):
        self.rewire_prob = rewire_prob
        self.k = k
        self.n_nodes = n_nodes
        self.nodes: dict[str, Node] = {}
        self.links: list[Link] = []
        self.philosophy_isomorphy = (
            "Watts-Strogatz 1998 借鉴的是**网络组织模式**, "
            "**不是让 Apeireth 成为 Small-World Network**"
            "(主人 22:05 不偏离哲学)"
        )

    def build_ring_lattice(self):
        """先构造 regular ring lattice (Watts-Strogatz 第一步)."""
        for i in range(self.n_nodes):
            n = Node(
                node_id=f"node_{i:03d}",
                label=f"persona_{i}",
                node_type="general",
            )
            self.nodes[n.node_id] = n
        # 每个 node 连 k nearest (k/2 left, k/2 right)
        for i in range(self.n_nodes):
            src = f"node_{i:03d}"
            for j in range(1, self.k // 2 + 1):
                tgt = f"node_{(i + j) % self.n_nodes:03d}"
                l = Link(link_id=uuid.uuid4().hex[:8], from_id=src, to_id=tgt, weight=1.0, is_rewired=False)
                self.links.append(l)

    def rewire_links(self, seed: int = 42):
        """Watts-Strogatz 第二步: 用概率 p 重连边."""
        random.seed(seed)
        nodes_list = list(self.nodes.keys())
        rewired_count = 0
        for link in self.links[:]:
            if random.random() < self.rewire_prob:
                # 重连到 random node (避免自环 + 重复边)
                candidate = random.choice([n for n in nodes_list if n != link.from_id])
                link.to_id = candidate
                link.is_rewired = True
                rewired_count += 1
        return rewired_count

    def shortest_path_length(self, src: str, dst: str) -> int:
        """计算从 src 到 dst 的最短路径长度 (BFS)."""
        if src == dst:
            return 0
        visited = {src}
        queue = [(src, 0)]
        while queue:
            current, dist = queue.pop(0)
            for link in self.links:
                if link.from_id == current and link.to_id not in visited:
                    if link.to_id == dst:
                        return dist + 1
                    visited.add(link.to_id)
                    queue.append((link.to_id, dist + 1))
        return -1  # unreachable

    def compute_avg_path_length(self) -> float:
        """计算网络平均路径长度 L."""
        if len(self.nodes) < 2:
            return 0.0
        total = 0
        count = 0
        nodes_list = list(self.nodes.keys())
        for i, src in enumerate(nodes_list):
            for j, dst in enumerate(nodes_list):
                if i != j:
                    d = self.shortest_path_length(src, dst)
                    if d > 0:
                        total += d
                        count += 1
        return total / count if count > 0 else 0.0

    def compute_clustering_coefficient(self) -> float:
        """简化 clustering coefficient C — 邻居间链接占比."""
        # 构建 adjacency dict
        adj = {n: set() for n in self.nodes}
        for link in self.links:
            if link.from_id in adj:
                adj[link.from_id].add(link.to_id)
        coefs = []
        for node, neighbors in adj.items():
            if len(neighbors) < 2:
                coefs.append(0.0)
                continue
            # 邻居对数
            possible_pairs = len(neighbors) * (len(neighbors) - 1) / 2
            actual_pairs = 0
            neighbors_list = list(neighbors)
            for i in range(len(neighbors_list)):
                for j in range(i + 1, len(neighbors_list)):
                    n1, n2 = neighbors_list[i], neighbors_list[j]
                    if n2 in adj.get(n1, set()) or n1 in adj.get(n2, set()):
                        actual_pairs += 1
            coefs.append(actual_pairs / possible_pairs if possible_pairs > 0 else 0.0)
        return sum(coefs) / len(coefs) if coefs else 0.0

    def stats(self) -> dict:
        L = self.compute_avg_path_length()
        C = self.compute_clustering_coefficient()
        return {
            "n_nodes": len(self.nodes),
            "n_links": len(self.links),
            "n_rewired": sum(1 for link in self.links if link.is_rewired),
            "rewire_prob": self.rewire_prob,
            "avg_path_L": round(L, 3),
            "clustering_C": round(C, 3),
            "small_world_signature": L < math.log10(self.n_nodes) and C > 0.3,
            "philosophy_isomorphy": self.philosophy_isomorphy,
            "source_citation": "Watts & Strogatz 1998 Nature 393 'Collective dynamics of small-world networks'",
        }


__all__ = ["SMALL_WORLD_VERSION", "Node", "Link", "SmallWorldGraph"]
