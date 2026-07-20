"""Memory ↔ Graph Linker v0.1 — 跨层自动绑定 (Phase 3.6)

依据: TOP-DESIGN-V1 §4.3 (Relation Graph) — 跨层引用 ref
主人原话: "像人是一切社会关系的总和" — Episode/Note 也是关系之一
DEV-LOG 14:50 next step #1+#2: Episode→GraphNode auto-sync + Note→derived_from auto-link

为什么需要 linker:
- SqliteMemoryStore (Phase 2.5) 和 SqliteRelationStore (Phase 3.5) 是两个独立 DB
- 没有自动绑定 = 两个孤岛, 主人问"为什么某 note 是某 episode 来的" 答不上来
- linker 关闭 loop: Episode 进 memory → 自动 add_node(kind='episode') + causal edge → ai_self
-                   Note   进 memory → 找 evidence 里的 eid → derived_from edge
- 幂等: 重复 link 走 upsert, 不污染图

设计原则:
- linker 不写业务逻辑 — 只是"转换器" — 把 memory 节点形态翻译成 graph 节点形态
- 跨层 ref 字段承担 lookup (episode.eid / note.nid)
- 中心 ai_self 节点由 ensure_central_ai_node 创建并复用
- 不动 memory / graph 自身 schema — 只读 + 写 graph
"""

from __future__ import annotations
import hashlib
import time
import uuid
from pathlib import Path
from typing import Optional

from .memory import Episode, Note
from .memory_store import SqliteMemoryStore
from .relation import Node as RNode, Edge as REdge, NODE_KINDS, EDGE_KINDS
from .relation_store import SqliteRelationStore

LINKER_VERSION = "0.1.0"


def _fingerprint(ep: Episode) -> str:
    """Episode 跨层稳定 fingerprint — 与 memory_store.de-dup 一致"""
    canonical = f"{ep.actor}|{ep.content}|{ep.context}|{ep.kind}"
    return hashlib.sha256(canonical.encode("utf-8", errors="ignore")).hexdigest()[:16]


def ensure_central_ai_node(gstore: SqliteRelationStore, label: str = "楚零 (中央 AI)") -> str:
    """确保图中存在且仅存在一个 ai_self 节点 — 返回 nid.

    - 查找现有 ai_self 节点复用
    - 不存在则新建 + meta.seed=True 标记首次创建
    """
    cur_nodes = gstore.nodes_by_kind("ai_self")
    if cur_nodes:
        return cur_nodes[0].nid
    nid = "ai_self_central"
    node = RNode(
        nid=nid,
        kind="ai_self",
        label=label,
        ref="",
        weight=10.0,
        meta={"seed": True, "ts": time.time()},
        created_at=time.time(),
    )
    gstore.upsert_node(node)
    return nid


def link_episode(ep: Episode, gstore: SqliteRelationStore, ai_nid: str) -> tuple[bool, bool]:
    """绑定 Episode → GraphNode + causal edge.

    返回 (episode_node_added, edge_added) — True 表示新加, False 表示去重命中.
    同 fingerprint 的 episode 节点不走 upsert (avoid pollution), 但 central 边 weight 累加.
    """
    if ep.kind not in ("utterance", "tool_call", "observation", "kickoff"):
        return (False, False)

    # 节点 nid 用 ref = eid (跨层引用)
    ep_nid = f"epi_{ep.eid}"
    is_new_node = gstore.upsert_node(RNode(
        nid=ep_nid,
        kind="episode",
        label=f"[{ep.actor}] {ep.content[:30]}",
        ref=ep.eid,
        weight=1.0,
        meta={"context": ep.context[:50], "fingerprint": _fingerprint(ep), "actor": ep.actor},
        created_at=ep.ts,
    ))
    # causal edge: episode → ai_self (episode 影响了中央 AI 的演化)
    is_new_edge = gstore.upsert_edge(REdge(
        eid=uuid.uuid4().hex[:8],
        src=ep_nid,
        dst=ai_nid,
        kind="causal",
        weight=1.0,
        evidence=ep.content[:60],
        created_at=ep.ts,
    ))
    # 反向 supports: ai_self supports episode (中央 AI 容纳这个事件)
    gstore.upsert_edge(REdge(
        eid=uuid.uuid4().hex[:8],
        src=ai_nid,
        dst=ep_nid,
        kind="supports",
        weight=0.5,
        evidence=f"contained at ts={ep.ts:.0f}",
        created_at=ep.ts,
    ))
    return (is_new_node, is_new_edge)


def link_note(n: Note, gstore: SqliteRelationStore, ai_nid: str) -> tuple[bool, list[str]]:
    """绑定 Note → GraphNode + derived_from edge (Note ← Episode.evidence).

    返回 (note_node_added, new_edge_ids). new_edge_ids 只记 derived_from, supports 边入图但不上报 (噪音).
    """
    note_nid = f"note_{n.nid}"
    is_new_node = gstore.upsert_node(RNode(
        nid=note_nid,
        kind="note",
        label=f"{n.topic}: {n.claim[:40]}",
        ref=n.nid,
        weight=max(1.0, n.confidence * (n.importance / 10.0) * 10.0),  # 0-10 scale from confidence * importance
        meta={"topic": n.topic, "confidence": n.confidence, "importance": n.importance},
        created_at=n.created_at,
    ))

    new_edge_ids: list[str] = []
    # derived_from: note ← episode (n.evidence 里列出的 eid)
    for eid in n.evidence or []:
        ep_nid = f"epi_{eid}"
        # 检查 episode 节点是否在图中 — 如果不在, 补一个 placeholder
        existing_ref_nodes = gstore.nodes_by_ref(eid)
        if not existing_ref_nodes:
            gstore.upsert_node(RNode(
                nid=ep_nid,
                kind="episode",
                label=f"[lazy] {eid}",
                ref=eid,
                weight=0.5,
                meta={"lazy_link": True, "source_note": n.nid},
                created_at=time.time(),
            ))
        added = gstore.upsert_edge(REdge(
            eid=uuid.uuid4().hex[:8],
            src=note_nid,
            dst=ep_nid,
            kind="derived_from",
            weight=n.confidence,
            evidence=f"note={n.nid} claim={n.claim[:50]}",
            created_at=n.created_at,
        ))
        if added:
            new_edge_ids.append(f"derived_from:{ep_nid}")

    # supports: note ← ai_self (中央 AI 支持这个 note 作为知识)
    gstore.upsert_edge(REdge(
        eid=uuid.uuid4().hex[:8],
        src=ai_nid,
        dst=note_nid,
        kind="supports",
        weight=0.5,
        evidence=f"note topic={n.topic}",
        created_at=n.created_at,
    ))
    return (is_new_node, new_edge_ids)


def sync_all(mstore: SqliteMemoryStore, gstore: SqliteRelationStore,
             ai_label: str = "楚零 (中央 AI)") -> dict:
    """全量同步 — Memory episodes + notes → Graph nodes + edges.

    幂等: 重复调用不会污染图 (走 upsert).
    返回同步统计.
    """
    ai_nid = ensure_central_ai_node(gstore, ai_label)

    ep_nodes_added = ep_edges_added = 0
    for ep in mstore.episodes(limit=10000):
        node_new, edge_new = link_episode(ep, gstore, ai_nid)
        ep_nodes_added += int(node_new)
        ep_edges_added += int(edge_new)

    note_nodes_added = 0
    derived_edges_added = 0
    for n in mstore.notes(limit=10000):
        node_new, derived_ids = link_note(n, gstore, ai_nid)
        note_nodes_added += int(node_new)
        derived_edges_added += len(derived_ids)

    return {
        "version": LINKER_VERSION,
        "ai_self": ai_nid,
        "episodes_linked": ep_nodes_added,
        "episode_causal_edges_added": ep_edges_added,
        "notes_linked": note_nodes_added,
        "derived_from_edges_added": derived_edges_added,
        "ts": time.time(),
    }


class Linker:
    """增量 Linker — 持有两 store, 增量加新 Episode/Note 时同步.

    Usage:
        linker = Linker(mstore, gstore)
        ep = Episode(eid=uuid.uuid4().hex[:8], actor='master', content='...')
        mstore.append_episode(ep)
        linker.link_one_episode(ep)
    """
    def __init__(self, mstore: SqliteMemoryStore, gstore: SqliteRelationStore,
                 ai_label: str = "楚零 (中央 AI)"):
        self.mstore = mstore
        self.gstore = gstore
        self.ai_nid = ensure_central_ai_node(gstore, ai_label)
        self.session_added_nodes: int = 0
        self.session_added_edges: int = 0

    def link_one_episode(self, ep: Episode) -> tuple[bool, bool]:
        node_new, edge_new = link_episode(ep, self.gstore, self.ai_nid)
        self.session_added_nodes += int(node_new)
        self.session_added_edges += int(edge_new)
        return (node_new, edge_new)

    def link_one_note(self, n: Note) -> tuple[bool, list[str]]:
        node_new, derived_ids = link_note(n, self.gstore, self.ai_nid)
        self.session_added_nodes += int(node_new)
        self.session_added_edges += len(derived_ids)
        return (node_new, derived_ids)

    def stats(self) -> dict:
        g = self.gstore.stats()
        return {
            "session_added_nodes": self.session_added_nodes,
            "session_added_edges": self.session_added_edges,
            "ai_self_nid": self.ai_nid,
            "graph_total_nodes": g.get("nodes", 0),
            "graph_total_edges": g.get("edges", 0),
            "linker_version": LINKER_VERSION,
        }


__all__ = [
    "LINKER_VERSION",
    "ensure_central_ai_node",
    "link_episode",
    "link_note",
    "sync_all",
    "Linker",
]
