"""Relation Graph v0.1 — 中央 AI 的关系图谱 (AriGraph 借鉴)

依据: TOP-DESIGN-V1 §4.3 (Component 3: Relation Graph)
文献: AriGraph (2407.04363) — 关系推理 + temporal knowledge graph
主人原话 (12:14): "中央 AI 是永恒身份, 但像人是一切社会关系的总和"

设计:
  节点 (Node.kind):
    master      主人 (唯一)
    ai_self     中央 AI (唯一, 中心节点)
    task        任务
    value       价值观 (来自 archetypes / remember_forever)
    agent       临时团 agent
    tool        工具/技能
    episode     历史事件 (Memory 层引用)
    note        抽象知识 (Memory 层引用)
  边 (Edge.kind):
    causal       X 引发 Y
    temporal     X 先于 Y
    part_of      X 属于 Y
    derived_from X 抽象自 Y (Note ← Episode)
    conflict     X 与 Y 冲突 (Reconsolidation 标记)
    supports     X 支持 Y (Note ↔ Value)
    assigned     任务分派

操作:
  add_node / add_edge / neighbors / traverse / find_path / to_dict / load_graph

中心节点 = ai_self。所有其它节点最终可达 ai_self (主人原话: "人是一切社会关系的总和")。
"""

from __future__ import annotations
import json
import hashlib
import time
import uuid
from dataclasses import dataclass, field, asdict
from pathlib import Path
from collections import deque
from typing import Optional

GRAPH_VERSION = "0.1.0"

# Node kind 限定 (allowlist)
NODE_KINDS = {"master", "ai_self", "task", "value", "agent", "tool", "episode", "note"}

# Edge kind 限定
EDGE_KINDS = {"causal", "temporal", "part_of", "derived_from", "conflict", "supports", "assigned"}


@dataclass
class Node:
    """关系图节点 — id 是稳定标识 (跨 session 引用)"""
    nid: str
    kind: str                                # NODE_KINDS 之一
    label: str = ""                          # 人类可读标签
    ref: str = ""                            # 跨层引用 (e.g. episode.eid / note.nid / task.uuid)
    weight: float = 1.0                      # 中心性 / 重要度
    meta: dict = field(default_factory=dict) # 扩展字段 (LLM 涌现的)
    created_at: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class Edge:
    """关系图边 — directed (单向语义)"""
    eid: str
    src: str                                 # source node nid
    dst: str                                 # dst node nid
    kind: str                                # EDGE_KINDS 之一
    weight: float = 1.0
    evidence: str = ""                       # 引用的 Episode / Note / 原话
    created_at: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class RelationGraph:
    """节点 + 边的容器 — 中心节点约定为 kind=ai_self 的节点 (唯一)"""
    nodes: dict[str, Node] = field(default_factory=dict)   # nid -> Node
    edges: list[Edge] = field(default_factory=list)
    version: str = GRAPH_VERSION
    created_at: float = field(default_factory=time.time)

    # ---------- construction ----------

    def add_node(self, kind: str, label: str, ref: str = "", nid: str = "", weight: float = 1.0, meta: dict | None = None) -> Node:
        """添加节点 — 同 nid 重复时更新 (去重)。"""
        if kind not in NODE_KINDS:
            raise ValueError(f"unknown node kind: {kind} (allow: {NODE_KINDS})")
        if not nid:
            nid = f"{kind[:3]}_{uuid.uuid4().hex[:8]}"
        if nid in self.nodes:
            existing = self.nodes[nid]
            existing.label = label or existing.label
            existing.weight = max(existing.weight, weight)
            if meta:
                existing.meta.update(meta)
            return existing
        node = Node(nid=nid, kind=kind, label=label, ref=ref, weight=weight, meta=meta or {})
        self.nodes[nid] = node
        return node

    def add_edge(self, src: str, dst: str, kind: str, weight: float = 1.0, evidence: str = "") -> Edge:
        """添加边 — 同 src+dst+kind 去重 (semantic key)"""
        if kind not in EDGE_KINDS:
            raise ValueError(f"unknown edge kind: {kind} (allow: {EDGE_KINDS})")
        if src not in self.nodes or dst not in self.nodes:
            raise KeyError(f"edge src/dst not in graph: src={src} dst={dst}")
        for e in self.edges:
            if e.src == src and e.dst == dst and e.kind == kind:
                e.weight = max(e.weight, weight)
                if evidence:
                    e.evidence = evidence
                return e
        edge = Edge(eid=uuid.uuid4().hex[:8], src=src, dst=dst, kind=kind, weight=weight, evidence=evidence)
        self.edges.append(edge)
        return edge

    # ---------- query ----------

    def central(self) -> Optional[Node]:
        """中心节点 (ai_self) — 唯一"""
        for n in self.nodes.values():
            if n.kind == "ai_self":
                return n
        return None

    def neighbors(self, nid: str, edge_kind: Optional[str] = None) -> list[tuple[Node, Edge]]:
        """邻接节点 — 返回 (neighbor_node, edge)。edge_kind 过滤可选。"""
        if nid not in self.nodes:
            return []
        out: list[tuple[Node, Edge]] = []
        for e in self.edges:
            if edge_kind and e.kind != edge_kind:
                continue
            if e.src == nid and e.dst in self.nodes:
                out.append((self.nodes[e.dst], e))
            elif e.dst == nid and e.src in self.nodes and e.kind in {"temporal", "part_of", "supports"}:
                # 反向语义 (少数边 kind 可逆)
                out.append((self.nodes[e.src], e))
        return out

    def traverse(self, start: str, depth: int = 2, edge_kind: Optional[str] = None) -> list[list[str]]:
        """BFS 有限深度 — 返回所有路径 (list of nid lists)"""
        if start not in self.nodes:
            return []
        paths: list[list[str]] = [[start]]
        frontier: list[list[str]] = [[start]]
        seen_paths: set[tuple] = {tuple([start])}
        for _ in range(depth):
            new_frontier: list[list[str]] = []
            for path in frontier:
                last = path[-1]
                for nb, _ in self.neighbors(last, edge_kind):
                    if nb.nid in path:  # avoid cycle
                        continue
                    new_path = path + [nb.nid]
                    key = tuple(new_path)
                    if key in seen_paths:
                        continue
                    seen_paths.add(key)
                    new_frontier.append(new_path)
                    paths.append(new_path)
            frontier = new_frontier
            if not frontier:
                break
        return paths

    def find_path(self, src: str, dst: str, max_depth: int = 5) -> list[str]:
        """最短路径 (BFS) — src 到 dst 的 nid 序列"""
        if src not in self.nodes or dst not in self.nodes:
            return []
        if src == dst:
            return [src]
        visited = {src}
        queue: deque[tuple[str, list[str]]] = deque([(src, [src])])
        while queue:
            cur, path = queue.popleft()
            for nb, _ in self.neighbors(cur):
                if nb.nid in visited:
                    continue
                new_path = path + [nb.nid]
                if nb.nid == dst:
                    return new_path
                visited.add(nb.nid)
                queue.append((nb.nid, new_path))
                if len(new_path) > max_depth:
                    continue
        return []

    # ---------- serialization ----------

    def to_dict(self) -> dict:
        return {
            "version": self.version,
            "created_at": self.created_at,
            "nodes": [n.to_dict() for n in self.nodes.values()],
            "edges": [e.to_dict() for e in self.edges],
        }

    def integrity_hash(self) -> str:
        canonical = json.dumps(self.to_dict(), sort_keys=True, ensure_ascii=False)
        return hashlib.sha256(canonical.encode("utf-8")).hexdigest()[:16]

    def stats(self) -> dict:
        kinds: dict[str, int] = {}
        for n in self.nodes.values():
            kinds[n.kind] = kinds.get(n.kind, 0) + 1
        ekinds: dict[str, int] = {}
        for e in self.edges:
            ekinds[e.kind] = ekinds.get(e.kind, 0) + 1
        return {
            "total_nodes": len(self.nodes),
            "total_edges": len(self.edges),
            "node_kinds": kinds,
            "edge_kinds": ekinds,
            "central": self.central().nid if self.central() else None,
            "hash": self.integrity_hash(),
        }


# ---------- I/O ----------

def save_graph(g: RelationGraph, path: str | Path) -> Path:
    p = Path(path)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(g.to_dict(), ensure_ascii=False, indent=2), encoding="utf-8")
    return p


def load_graph(path: str | Path) -> RelationGraph:
    raw = json.loads(Path(path).read_text(encoding="utf-8"))
    g = RelationGraph(version=raw.get("version", GRAPH_VERSION), created_at=raw.get("created_at", time.time()))
    for nd in raw.get("nodes", []):
        meta = nd.pop("meta", {})
        node = Node(meta=meta, **nd)
        g.nodes[node.nid] = node
    for ed in raw.get("edges", []):
        g.edges.append(Edge(**ed))
    return g


__all__ = [
    "NODE_KINDS",
    "EDGE_KINDS",
    "GRAPH_VERSION",
    "Node",
    "Edge",
    "RelationGraph",
    "save_graph",
    "load_graph",
]