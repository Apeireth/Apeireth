"""Relation Graph v0.2 — SQLite 真持久化 (Phase 3 续)

v0.1 (relation.py): JSON 序列化 + in-memory graph
v0.2 (本文件): SQLite 持久化 + 跨 session 真存活 + 跨层引用 (ref)

依据:
- TOP-DESIGN-V1 §3.3 中心 + 临时团双层架构 — 图必须跨 session 持久
- 主人 12:14 中央 AI 是永恒身份 — 图是其关系总和
- AriGraph (2407.04363) temporal KG 借鉴

为什么 SQLite 而不是 JSON:
- memory v0.2 (d597171) 已经走过这条路 — 一致性
- 节点会增长 (每次交互加 epi/note 节点), JSON 重写慢
- 边按 kind 查询需要索引 — SQL 天然适合
- SQLite stdlib, 0 依赖

Schema 设计:
- nodes 表: 主键 nid, 加 ref 索引 (跨层引用 eid/nid/task.uuid)
- edges 表: 主键 eid, 加 (src,kind) / (dst,kind) 索引
- graph_meta 表: 单行 (id=1), 存 schema_version + integrity_hash

不变性约束:
- add_node 同 nid 更新 (upsert) — 不抛错
- add_edge 同 (src,dst,kind) 去重 (semantic key)
- integrity_hash 走 canonical JSON sort_keys=True
"""

from __future__ import annotations
import json
import sqlite3
import time
from pathlib import Path
from typing import Optional

from .relation import (
    RelationGraph, Node, Edge,
    NODE_KINDS, EDGE_KINDS,
)


SCHEMA = """
-- v0.2 schema — graph_meta 加 graph_created_at 字段, 还原 RelationGraph.created_at
CREATE TABLE IF NOT EXISTS graph_meta (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    schema_version TEXT NOT NULL,
    integrity_hash TEXT NOT NULL,
    graph_created_at REAL NOT NULL DEFAULT 0.0,
    updated_at REAL NOT NULL
);

CREATE TABLE IF NOT EXISTS graph_nodes (
    nid TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    label TEXT DEFAULT '',
    ref TEXT DEFAULT '',
    weight REAL DEFAULT 1.0,
    meta TEXT DEFAULT '{}',
    created_at REAL NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_graph_nodes_kind ON graph_nodes(kind);
CREATE INDEX IF NOT EXISTS idx_graph_nodes_ref  ON graph_nodes(ref);

CREATE TABLE IF NOT EXISTS graph_edges (
    eid TEXT PRIMARY KEY,
    src TEXT NOT NULL,
    dst TEXT NOT NULL,
    kind TEXT NOT NULL,
    weight REAL DEFAULT 1.0,
    evidence TEXT DEFAULT '',
    created_at REAL NOT NULL,
    UNIQUE(src, dst, kind)
);
CREATE INDEX IF NOT EXISTS idx_graph_edges_src  ON graph_edges(src);
CREATE INDEX IF NOT EXISTS idx_graph_edges_dst  ON graph_edges(dst);
CREATE INDEX IF NOT EXISTS idx_graph_edges_kind ON graph_edges(kind);


"""


class SqliteRelationStore:
    """SQLite backend for RelationGraph.

    Usage:
        store = SqliteRelationStore("graph.db")
        store.upsert_node(Node(...))
        store.upsert_edge(Edge(...))
        g = store.load_graph()
    """

    def __init__(self, path: str | Path):
        self.path = Path(path)
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self._conn = sqlite3.connect(str(self.path))
        self._conn.execute("PRAGMA journal_mode=WAL")
        self._conn.execute("PRAGMA synchronous=NORMAL")
        self._conn.execute("PRAGMA foreign_keys = ON")
        self._init_schema()
        self._init_seed_meta()

    def _init_schema(self):
        self._conn.executescript(SCHEMA)
        self._conn.commit()

    def _init_seed_meta(self):
        cur = self._conn.execute("SELECT 1 FROM graph_meta WHERE id=1")
        if cur.fetchone() is None:
            self._conn.execute(
                "INSERT INTO graph_meta(id, schema_version, integrity_hash, graph_created_at, updated_at) VALUES (1,?,?,?,?)",
                ("0.2.0", "", 0.0, time.time()),
            )
            self._conn.commit()

    # ---------- write ----------

    def upsert_node(self, n: Node) -> bool:
        """插入或更新节点. nid 已存在则 weight/meta 更新."""
        cur = self._conn.execute(
            "SELECT weight, meta FROM graph_nodes WHERE nid = ?", (n.nid,)
        )
        row = cur.fetchone()
        if row:
            old_w, old_meta = row
            meta = json.loads(old_meta) if old_meta else {}
            meta.update(n.meta or {})
            self._conn.execute(
                "UPDATE graph_nodes SET label=?, weight=MAX(?,?), meta=?, ref=? WHERE nid=?",
                (n.label, n.weight, old_w, json.dumps(meta, ensure_ascii=False), n.ref, n.nid),
            )
            self._conn.commit()
            return False  # updated, not new
        self._conn.execute(
            "INSERT INTO graph_nodes(nid, kind, label, ref, weight, meta, created_at) VALUES (?,?,?,?,?,?,?)",
            (n.nid, n.kind, n.label, n.ref, n.weight, json.dumps(n.meta or {}, ensure_ascii=False), n.created_at),
        )
        self._conn.commit()
        return True  # new

    def upsert_edge(self, e: Edge) -> bool:
        """插入或更新边. UNIQUE(src,dst,kind) 保证 semantic 去重."""
        try:
            self._conn.execute(
                "INSERT INTO graph_edges(eid, src, dst, kind, weight, evidence, created_at) VALUES (?,?,?,?,?,?,?)",
                (e.eid, e.src, e.dst, e.kind, e.weight, e.evidence, e.created_at),
            )
            self._conn.commit()
            return True
        except sqlite3.IntegrityError:
            self._conn.execute(
                "UPDATE graph_edges SET weight=MAX(weight,?), evidence=CASE WHEN ? != '' THEN ? ELSE evidence END WHERE src=? AND dst=? AND kind=?",
                (e.weight, e.evidence, e.evidence, e.src, e.dst, e.kind),
            )
            self._conn.commit()
            return False

    def remove_node(self, nid: str) -> int:
        """删除节点 + 级联删除关联边. 返回删除的边数."""
        cur = self._conn.execute("SELECT COUNT(*) FROM graph_edges WHERE src=? OR dst=?", (nid, nid))
        edge_count = cur.fetchone()[0]
        self._conn.execute("DELETE FROM graph_edges WHERE src=? OR dst=?", (nid, nid))
        self._conn.execute("DELETE FROM graph_nodes WHERE nid=?", (nid,))
        self._conn.commit()
        return edge_count

    # ---------- read ----------

    def load_graph(self) -> RelationGraph:
        """从 SQLite 重建 RelationGraph. 顺序: 先 nodes 再 edges (边依赖节点).

        重要: graph_meta.graph_created_at 还原 graph.created_at — 否则 integrity_hash 漂移.
        """
        meta = self._conn.execute(
            "SELECT graph_created_at FROM graph_meta WHERE id=1"
        ).fetchone()
        g_created = meta[0] if meta and meta[0] else time.time()
        g = RelationGraph(created_at=g_created)
        rows = self._conn.execute(
            "SELECT nid, kind, label, ref, weight, meta, created_at FROM graph_nodes"
        ).fetchall()
        for r in rows:
            meta = json.loads(r[5]) if r[5] else {}
            g.nodes[r[0]] = Node(
                nid=r[0], kind=r[1], label=r[2], ref=r[3],
                weight=r[4], meta=meta, created_at=r[6],
            )
        rows = self._conn.execute(
            "SELECT eid, src, dst, kind, weight, evidence, created_at FROM graph_edges"
        ).fetchall()
        for r in rows:
            g.edges.append(Edge(
                eid=r[0], src=r[1], dst=r[2], kind=r[3],
                weight=r[4], evidence=r[5], created_at=r[6],
            ))
        return g

    def nodes_by_kind(self, kind: str) -> list[Node]:
        rows = self._conn.execute(
            "SELECT nid, kind, label, ref, weight, meta, created_at FROM graph_nodes WHERE kind=? ORDER BY weight DESC",
            (kind,),
        ).fetchall()
        return [
            Node(nid=r[0], kind=r[1], label=r[2], ref=r[3],
                 weight=r[4], meta=json.loads(r[5]) if r[5] else {}, created_at=r[6])
            for r in rows
        ]

    def nodes_by_ref(self, ref: str) -> list[Node]:
        """跨层引用查找 (e.g. ref='demo_e1' 找所有引用该 episode 的节点)."""
        rows = self._conn.execute(
            "SELECT nid, kind, label, ref, weight, meta, created_at FROM graph_nodes WHERE ref=?",
            (ref,),
        ).fetchall()
        return [
            Node(nid=r[0], kind=r[1], label=r[2], ref=r[3],
                 weight=r[4], meta=json.loads(r[5]) if r[5] else {}, created_at=r[6])
            for r in rows
        ]

    def edges_by_kind(self, kind: str) -> list[Edge]:
        rows = self._conn.execute(
            "SELECT eid, src, dst, kind, weight, evidence, created_at FROM graph_edges WHERE kind=?",
            (kind,),
        ).fetchall()
        return [Edge(eid=r[0], src=r[1], dst=r[2], kind=r[3], weight=r[4], evidence=r[5], created_at=r[6]) for r in rows]

    # ---------- meta / integrity ----------

    def save_meta(self, integrity_hash: str, graph_created_at: Optional[float] = None):
        """存 hash + 可选 graph-level created_at (保证 round-trip 后 hash 一致)."""
        if graph_created_at is not None:
            self._conn.execute(
                "UPDATE graph_meta SET integrity_hash=?, graph_created_at=?, updated_at=? WHERE id=1",
                (integrity_hash, graph_created_at, time.time()),
            )
        else:
            self._conn.execute(
                "UPDATE graph_meta SET integrity_hash=?, updated_at=? WHERE id=1",
                (integrity_hash, time.time()),
            )
        self._conn.commit()

    def stats(self) -> dict:
        nodes = self._conn.execute("SELECT COUNT(*) FROM graph_nodes").fetchone()[0]
        edges = self._conn.execute("SELECT COUNT(*) FROM graph_edges").fetchone()[0]
        kinds = self._conn.execute("SELECT kind, COUNT(*) FROM graph_nodes GROUP BY kind").fetchall()
        ekinds = self._conn.execute("SELECT kind, COUNT(*) FROM graph_edges GROUP BY kind").fetchall()
        meta = self._conn.execute("SELECT schema_version, integrity_hash FROM graph_meta WHERE id=1").fetchone()
        return {
            "nodes": nodes,
            "edges": edges,
            "node_kinds": dict(kinds),
            "edge_kinds": dict(ekinds),
            "schema_version": meta[0] if meta else "0.0.0",
            "stored_hash": meta[1] if meta else "",
        }

    def close(self):
        self._conn.close()


# ---------- bridge from in-memory to SQLite ----------

def migrate_from_relation_graph(g: RelationGraph, store: SqliteRelationStore) -> dict:
    """把 v0.1 in-memory RelationGraph 迁到 v0.2 SQLite backend.

    先 nodes 后 edges — 边依赖节点存在.
    返回迁移统计.
    """
    nodes_added = nodes_updated = 0
    for n in g.nodes.values():
        if store.upsert_node(n):
            nodes_added += 1
        else:
            nodes_updated += 1
    edges_added = edges_updated = 0
    for e in g.edges:
        if store.upsert_edge(e):
            edges_added += 1
        else:
            edges_updated += 1
    # 写 meta (含 graph-level created_at, 保证 round-trip hash 一致)
    store.save_meta(g.integrity_hash(), graph_created_at=g.created_at)
    return {
        "nodes_added": nodes_added,
        "nodes_updated": nodes_updated,
        "edges_added": edges_added,
        "edges_updated": edges_updated,
    }


__all__ = [
    "SqliteRelationStore",
    "migrate_from_relation_graph",
]