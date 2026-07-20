"""Memory Store v0.2 — 真持久化 + SQLite FTS5 全文搜索

借鉴来源 (主人 14:27 "聚集全人类智慧"):
- claude-mem (thedotmack, 87k stars): SQLite + FTS5 + 3-layer progressive disclosure
- letta (Berkeley): archival + recall 二层
- Letta research paper: archival memory + core memory + recall

设计原则 (主人原话):
- 主人 12:14 "中央 AI 是永恒身份" -> Episode 必须持久化跨 session
- 主人 12:14 "LLM 没历史就从主人学" -> Episode 必须永久 (除非 Forget)
- 主人 12:27 "platform 13:04 让 LLM 接入后超越自己" -> Layered memory

为什么用 SQLite 不是 sqlite-vec:
- SQLite 是 stdlib (Py 3.13 自带 sqlite3)
- FTS5 是 SQLite 自带的全文搜索 (无需额外依赖)
- sqlite-vec 也要 SQLite + 额外扩展 -> 稍后接
- 这次 v0.2 目标是 "真能跑 + 真能搜"

为什么 借鉴 claude-mem 的 3-layer:
- search -> timeline -> get_observations
- 这是 token-efficient 的渐进披露
- 我们 L3 Memory Layer 直接实现这个 API
"""

from __future__ import annotations
import json
import sqlite3
import time
import uuid
from pathlib import Path
from typing import Optional

from .memory import Episode, Note, MemoryStore, _k


SCHEMA = """
CREATE TABLE IF NOT EXISTS episodes (
    eid TEXT PRIMARY KEY,
    actor TEXT NOT NULL,
    content TEXT NOT NULL,
    context TEXT DEFAULT '',
    kind TEXT DEFAULT 'utterance',
    ts REAL NOT NULL,
    linked_identity_hash TEXT DEFAULT '',
    fingerprint TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_episodes_ts ON episodes(ts);
CREATE INDEX IF NOT EXISTS idx_episodes_fingerprint ON episodes(fingerprint);

CREATE TABLE IF NOT EXISTS notes (
    nid TEXT PRIMARY KEY,
    topic TEXT NOT NULL,
    claim TEXT NOT NULL,
    confidence REAL DEFAULT 0.5,
    importance INTEGER DEFAULT 5,
    evidence TEXT DEFAULT '[]',
    created_at REAL NOT NULL,
    last_consolidated REAL NOT NULL,
    supersedes TEXT DEFAULT '[]'
);

CREATE INDEX IF NOT EXISTS idx_notes_topic ON notes(topic);

CREATE VIRTUAL TABLE IF NOT EXISTS episodes_fts USING fts5(
    eid UNINDEXED,
    content,
    context
);

CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
    nid UNINDEXED,
    topic,
    claim
);

CREATE TABLE IF NOT EXISTS memory_meta (
    k TEXT PRIMARY KEY,
    v TEXT NOT NULL
);
"""


class SqliteMemoryStore:
    """SQLite + FTS5 backend for MemoryStore.

    Schema = 3 tables + 2 FTS5 indexes + meta table.
    Why SQLite + FTS5 (vs JSON file):
    - FTS5 是 SQLite 自带的 full-text search (BM25 ranking)
    - 不需额外依赖 (sqlite3 是 stdlib)
    - 0 失败模式 (文件锁, 冲突, race condition)
    - 跨 session 是默认行为 (file-based DB)

    Usage:
        store = SqliteMemoryStore("apeireth.db")
        store.append_episode(Episode(...))
        results = store.search_episodes("authentication bug", limit=5)
    """

    def __init__(self, path: str | Path):
        self.path = Path(path)
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self._conn = sqlite3.connect(str(self.path))
        self._conn.execute("PRAGMA journal_mode=WAL")  # 读写并发
        self._conn.execute("PRAGMA synchronous=NORMAL")  # speed
        self._init_schema()
        self._init_seed_meta()

    def _init_schema(self):
        self._conn.executescript(SCHEMA)
        self._conn.commit()

    def _init_seed_meta(self):
        cur = self._conn.execute("SELECT v FROM memory_meta WHERE k='schema_version'")
        if cur.fetchone() is None:
            self._conn.execute(
                "INSERT INTO memory_meta(k, v) VALUES (?, ?)",
                ("schema_version", "0.2.0"),
            )
            self._conn.commit()

    # ---------- Episode ----------

    def append_episode(self, ep: Episode) -> bool:
        """插入 Episode. 同 fingerprint 跳过 (de-dup)."""
        fp = integrity_hash_for_episode(ep)
        cur = self._conn.execute("SELECT 1 FROM episodes WHERE fingerprint = ? LIMIT 1", (fp,))
        if cur.fetchone():
            return False  # dup
        self._conn.execute(
            "INSERT INTO episodes(eid, actor, content, context, kind, ts, linked_identity_hash, fingerprint) VALUES (?,?,?,?,?,?,?,?)",
            (ep.eid, ep.actor, ep.content, ep.context, ep.kind, ep.ts, ep.linked_identity_hash, fp),
        )
        self._conn.execute(
            "INSERT INTO episodes_fts(eid, content, context) VALUES (?,?,?)",
            (ep.eid, ep.content, ep.context),
        )
        self._conn.commit()
        return True

    def episodes(self, limit: int = 100, since_ts: Optional[float] = None) -> list[Episode]:
        if since_ts is not None:
            rows = self._conn.execute(
                "SELECT eid, actor, content, context, kind, ts, linked_identity_hash FROM episodes WHERE ts >= ? ORDER BY ts DESC LIMIT ?",
                (since_ts, limit),
            ).fetchall()
        else:
            rows = self._conn.execute(
                "SELECT eid, actor, content, context, kind, ts, linked_identity_hash FROM episodes ORDER BY ts DESC LIMIT ?",
                (limit,),
            ).fetchall()
        return [Episode(eid=r[0], actor=r[1], content=r[2], context=r[3], kind=r[4], ts=r[5], linked_identity_hash=r[6]) for r in rows]

    def search_episodes(self, query: str, limit: int = 5) -> list[tuple[Episode, float]]:
        """3-layer Layer 1: compact index. Returns (Episode, bm25_score)."""
        sql = """
        SELECT e.eid, e.actor, e.content, e.context, e.kind, e.ts, e.linked_identity_hash,
               bm25(episodes_fts) AS score
        FROM episodes_fts
        JOIN episodes e ON e.eid = episodes_fts.eid
        WHERE episodes_fts MATCH ?
        ORDER BY score
        LIMIT ?
        """
        rows = self._conn.execute(sql, (query, limit)).fetchall()
        results = []
        for r in rows:
            ep = Episode(eid=r[0], actor=r[1], content=r[2], context=r[3], kind=r[4], ts=r[5], linked_identity_hash=r[6])
            results.append((ep, r[7]))
        return results

    # ---------- Note ----------

    def add_note(self, note: Note) -> bool:
        try:
            self._conn.execute(
                "INSERT INTO notes(nid, topic, claim, confidence, importance, evidence, created_at, last_consolidated, supersedes) VALUES (?,?,?,?,?,?,?,?,?)",
                (note.nid, note.topic, note.claim, note.confidence, note.importance,
                 json.dumps(note.evidence), note.created_at, note.last_consolidated,
                 json.dumps(note.supersedes)),
            )
            self._conn.execute(
                "INSERT INTO notes_fts(nid, topic, claim) VALUES (?,?,?)",
                (note.nid, note.topic, note.claim),
            )
            self._conn.commit()
            return True
        except sqlite3.IntegrityError:
            return False

    def notes(self, limit: int = 100) -> list[Note]:
        rows = self._conn.execute(
            "SELECT nid, topic, claim, confidence, importance, evidence, created_at, last_consolidated, supersedes FROM notes ORDER BY last_consolidated DESC LIMIT ?",
            (limit,),
        ).fetchall()
        return [deserialize_note(r) for r in rows]

    def update_note(self, note: Note) -> None:
        self._conn.execute(
            "UPDATE notes SET topic=?, claim=?, confidence=?, importance=?, evidence=?, last_consolidated=?, supersedes=? WHERE nid=?",
            (note.topic, note.claim, note.confidence, note.importance,
             json.dumps(note.evidence), note.last_consolidated,
             json.dumps(note.supersedes), note.nid),
        )
        # FTS5 不会自动更新 (这是 SQLite 限制), 先删后插
        self._conn.execute("DELETE FROM notes_fts WHERE nid = ?", (note.nid,))
        self._conn.execute(
            "INSERT INTO notes_fts(nid, topic, claim) VALUES (?,?,?)",
            (note.nid, note.topic, note.claim),
        )
        self._conn.commit()

    def search_notes(self, query: str, limit: int = 5) -> list[tuple[Note, float]]:
        sql = """
        SELECT n.nid, n.topic, n.claim, n.confidence, n.importance, n.evidence, n.created_at, n.last_consolidated, n.supersedes,
               bm25(notes_fts) AS score
        FROM notes_fts
        JOIN notes n ON n.nid = notes_fts.nid
        WHERE notes_fts MATCH ?
        ORDER BY score
        LIMIT ?
        """
        rows = self._conn.execute(sql, (query, limit)).fetchall()
        return [(deserialize_note(r), r[9]) for r in rows]

    # ---------- Forget ----------

    def forget_episode(self, eid: str) -> None:
        self._conn.execute("DELETE FROM episodes_fts WHERE eid = ?", (eid,))
        self._conn.execute("DELETE FROM episodes WHERE eid = ?", (eid,))
        self._conn.commit()

    def forget_note(self, nid: str) -> None:
        self._conn.execute("DELETE FROM notes_fts WHERE nid = ?", (nid,))
        self._conn.execute("DELETE FROM notes WHERE nid = ?", (nid,))
        self._conn.commit()

    # ---------- Stats ----------

    def stats(self) -> dict:
        episodes = self._conn.execute("SELECT COUNT(*) FROM episodes").fetchone()[0]
        notes = self._conn.execute("SELECT COUNT(*) FROM notes").fetchone()[0]
        schema = self._conn.execute("SELECT v FROM memory_meta WHERE k='schema_version'").fetchone()
        return {"episodes": episodes, "notes": notes, "schema_version": schema[0] if schema else "0.0.0"}

    def close(self) -> None:
        self._conn.close()


# ---------- helpers ----------

def deserialize_note(r) -> Note:
    return Note(
        nid=r[0], topic=r[1], claim=r[2], confidence=r[3], importance=r[4],
        evidence=json.loads(r[5]) if r[5] else [],
        created_at=r[6], last_consolidated=r[7],
        supersedes=json.loads(r[8]) if r[8] else [],
    )


def integrity_hash_for_episode(ep: Episode) -> str:
    """Stable hash of Episode 内容 (de-dup key)."""
    canonical = f"{ep.actor}|{ep.content}|{ep.context}|{ep.kind}"
    import hashlib
    return hashlib.sha256(canonical.encode("utf-8", errors="ignore")).hexdigest()[:16]


# ---------- bridge from in-memory to SQLite ----------

def migrate_from_memory_store(mem: MemoryStore, sqlite_store: SqliteMemoryStore) -> dict:
    """把 v0.1 in-memory MemoryStore 迁到 v0.2 SQLite backend.

    返回迁移统计.
    """
    episodes_added = 0
    episodes_skipped = 0
    for ep in mem.episodes:
        if sqlite_store.append_episode(ep):
            episodes_added += 1
        else:
            episodes_skipped += 1

    notes_added = 0
    notes_skipped = 0
    for n in mem.notes:
        if sqlite_store.add_note(n):
            notes_added += 1
        else:
            notes_skipped += 1

    return {
        "episodes_added": episodes_added,
        "episodes_skipped_dedup": episodes_skipped,
        "notes_added": notes_added,
        "notes_skipped_dedup": notes_skipped,
    }


__all__ = [
    "SqliteMemoryStore",
    "migrate_from_memory_store",
    "integrity_hash_for_episode",
]
