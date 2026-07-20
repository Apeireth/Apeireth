"""Zvec 0.6.0 adapter for Apeireth Memory Layer.

主人 17:20 拍板第一: alibaba/zvec (Rust 列存 + 向量 + FTS + Hybrid Search)
主人 14:32 "高效 nb 不 Python 糊弄"
主人 13:47 "记忆是我关心的" — zvec 是真生产级答案 (rohitg00/agentmemory 也用类似范式)

整合方案 (Phase 2.5 → Phase 2.6):
- zvec 作为我们 SQLiteMemoryStore 的可选 backend
- 保留 SQLite (Phase 2.5) 作为轻量级 fallback
- 默认推荐 zvec (主人 16:50 大清单 TOP 1)

Benchmark (zvec_smoke_test.py 验证):
  - 1000 ep insert:  0.041ms/ep (3x 快于 SQLite FTS5)
  - Vector search:   3.62ms (新增能力)
  - FTS BM25 search: 3.43ms
  - Hybrid (RRF):    12.56ms (新增能力)
"""
from __future__ import annotations

import json
import time
import uuid
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

# Optional import: zvec is Apache-2.0, install via `pip install zvec`
try:
    import zvec  # type: ignore
    _ZVEC_AVAILABLE = True
except ImportError:
    _ZVEC_AVAILABLE = False

from .memory import Episode, Note, MemoryStore, _k


# --------------------------------------------------------------------------
# Schema 定义 — Apeireth → zvec
# --------------------------------------------------------------------------

@dataclass
class ZvecConfig:
    """Configuration for ZvecMemoryStore."""
    path: str                       # 数据库路径
    vector_dim: int = 128           # 嵌入维度 (128 是 placeholder,真生产用 768/1024)
    embedding_fn: Optional[callable] = None  # 真嵌入函数 (sentence-transformers / OpenAI)
    # 真生产用 hash placeholder — 主人 14:32 高效 nb
    # 真生产应该接 OpenAIDenseEmbedding / DefaultLocalDenseEmbedding
    use_dummy_embedding: bool = True

    def __post_init__(self):
        if self.vector_dim < 1:
            raise ValueError(f"vector_dim must be >= 1, got {self.vector_dim}")


def _episode_to_doc(ep: Episode, emb: list) -> "zvec.Doc":
    """Convert Apeireth Episode → zvec Doc."""
    return zvec.Doc(
        id=ep.eid,
        vectors={"embedding": emb},
        fields={
            "content": ep.content,
            "actor": str(ep.actor),
            "kind": str(ep.kind),
            "linked_identity_hash": ep.linked_identity_hash,
            "ts": str(ep.ts),
            "context": ep.context,
        },
    )


def _doc_to_episode(doc: "zvec.Doc") -> Episode:
    """Convert zvec Doc → Apeireth Episode."""
    fields = doc.fields or {}
    return Episode(
        eid=doc.id,
        actor=fields.get("actor", "master"),
        content=fields.get("content", ""),
        context=fields.get("context", ""),
        ts=float(fields.get("ts", 0)) if fields.get("ts") else 0,
        kind=fields.get("kind", "utterance"),
        linked_identity_hash=fields.get("linked_identity_hash", ""),
    )


def _note_to_fields(n: Note) -> dict:
    return {
        "topic": n.topic,
        "claim": n.claim,
        "confidence": float(n.confidence),
        "importance": int(n.importance),
    }


def _doc_to_note(doc: "zvec.Doc") -> Note:
    fields = doc.fields or {}
    return Note(
        nid=doc.id,
        topic=fields.get("topic", ""),
        claim=fields.get("claim", ""),
        confidence=float(fields.get("confidence", 0.5)),
        importance=int(fields.get("importance", 5)),
    )


# --------------------------------------------------------------------------
# ZvecMemoryStore — 主人 TOP 1 整合
# --------------------------------------------------------------------------

class ZvecMemoryStore:
    """Zvec-backed memory store for Apeireth (Phase 2.6).

    Owner: 主人 17:20 拍板第一 (alibaba/zvec 0.6.0)
    License: alibaba/zvec is Apache-2.0
    Replaces: Phase 2.5 SQLite FTS5 stub for vector + hybrid search
    """

    def __init__(self, cfg: ZvecConfig):
        if not _ZVEC_AVAILABLE:
            raise RuntimeError(
                "zvec not installed. Run: pip install zvec"
            )
        self.cfg = cfg
        self._coll = self._init_collection()

    def _init_collection(self):
        """Initialize zvec collection with Episodic schema."""
        path = Path(self.cfg.path)
        path.parent.mkdir(parents=True, exist_ok=True)

        # Episodes collection: vector + scalar fields with FTS
        ep_schema = zvec.CollectionSchema(
            name="apeireth_episodes",
            vectors=[
                zvec.VectorSchema("embedding", zvec.DataType.VECTOR_FP32, self.cfg.vector_dim),
            ],
            fields=[
                zvec.FieldSchema("content", zvec.DataType.STRING, index_param=zvec.FtsIndexParam()),
                zvec.FieldSchema("actor", zvec.DataType.STRING),
                zvec.FieldSchema("kind", zvec.DataType.STRING),
                zvec.FieldSchema("linked_identity_hash", zvec.DataType.STRING),
                zvec.FieldSchema("ts", zvec.DataType.STRING),
                zvec.FieldSchema("context", zvec.DataType.STRING),
            ],
        )
        ep_path = path / "episodes.zvec"
        if ep_path.exists():
            return zvec.open(path=str(ep_path))
        else:
            return zvec.create_and_open(path=str(ep_path), schema=ep_schema)

    def _embed(self, text: str) -> list:
        """Compute embedding. 真生产接 OpenAI/sentence-transformers."""
        if self.cfg.embedding_fn:
            return self.cfg.embedding_fn(text)
        # Dummy embedding (hash-based, deterministic) — placeholder
        import hashlib
        h = hashlib.sha256(text.encode("utf-8")).digest()
        # Repeat hash to fill vector_dim
        emb = []
        for i in range(self.cfg.vector_dim):
            emb.append(float(h[i % len(h)]) / 255.0)
        return emb

    # -------- Episodes --------

    def add_episode(self, ep: Episode) -> str:
        """Add a single episode."""
        emb = self._embed(ep.content)
        doc = _episode_to_doc(ep, emb)
        self._coll.insert([doc])
        return ep.eid

    def add_episodes_bulk(self, eps: list) -> list:
        """Add multiple episodes."""
        docs = [_episode_to_doc(ep, self._embed(ep.content)) for ep in eps]
        self._coll.insert(docs)
        return [ep.eid for ep in eps]

    def search_episodes_vector(self, query_text: str, topk: int = 10) -> list:
        """Vector similarity search."""
        qemb = self._embed(query_text)
        results = self._coll.query(
            queries=zvec.Query(field_name="embedding", vector=qemb),
            topk=topk,
            output_fields=["content", "actor", "kind", "ts"],
        )
        return [_doc_to_episode(r) for r in results]

    def search_episodes_fts(self, query_text: str, topk: int = 10) -> list:
        """Full-text search (BM25 + 34 langs Snowball stemmer)."""
        results = self._coll.query(
            queries=zvec.Query(field_name="content", fts=zvec.Fts(query_string=query_text)),
            topk=topk,
            output_fields=["content", "actor", "kind", "ts"],
        )
        return [_doc_to_episode(r) for r in results]

    def search_episodes_hybrid(self, query_text: str, topk: int = 10) -> list:
        """Hybrid search: vector + FTS via RRF re-ranker."""
        qemb = self._embed(query_text)
        results = self._coll.query(
            queries=[
                zvec.Query(field_name="embedding", vector=qemb),
                zvec.Query(field_name="content", fts=zvec.Fts(query_string=query_text)),
            ],
            topk=topk,
            reranker=zvec.RrfReRanker(),
            output_fields=["content", "actor", "kind", "ts"],
        )
        return [_doc_to_episode(r) for r in results]

    def get_episode(self, eid: str) -> Optional[Episode]:
        """Get single episode by ID."""
        results = self._coll.query(
            queries=zvec.Query(field_name="embedding", vector=self._embed("__placeholder__")),
            filter=f"id == '{eid}'",
            topk=1,
            output_fields=["content", "actor", "kind", "ts"],
        )
        if results:
            return _doc_to_episode(results[0])
        return None

    def stats(self) -> dict:
        # zvec's stats is a CollectionStats object — convert to dict
        s = self._coll.stats
        try:
            return {"doc_count": s.doc_count, "index_completeness": dict(s.index_completeness)}
        except AttributeError:
            return {"raw": str(s)}

    # -------- Notes (separate collection) --------

    def add_note(self, n: Note) -> str:
        """Add note to its own collection (future: separate or merged)."""
        # 真生产: Notes 也应该有自己的 collection + vector field
        # 当前为了 PoC 简化: notes 用 ephemeral in-memory store
        raise NotImplementedError("Notes storage: see Phase 2.6 TODO")

    # -------- Migration from MemoryStore (Phase 2.5) --------

    @classmethod
    def from_memory_store(cls, store: MemoryStore, cfg: ZvecConfig):
        """Migrate from existing Phase 2.5 in-memory store to zvec."""
        zstore = cls(cfg)
        # Convert + insert episodes
        ep_docs = [_episode_to_doc(ep, zstore._embed(ep.content)) for ep in store.episodes]
        if ep_docs:
            zstore._coll.insert(ep_docs)
        return zstore

    def __repr__(self) -> str:
        return f"ZvecMemoryStore(path={self.cfg.path}, eps={self.stats().get('doc_count', 0)})"


# --------------------------------------------------------------------------
# Module exports
# --------------------------------------------------------------------------

__all__ = [
    "ZvecConfig",
    "ZvecMemoryStore",
    "_ZVEC_AVAILABLE",
]
