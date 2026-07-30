"""R13 MVP — Episode/Note storage layer (SQLite + FTS5).

Ponytail ceiling: Episode append-only + Note mutable, BM25 retrieval is
in retrieve.py. When salience decays below threshold, garbage-collect.

为什么不直接用 LangChain Memory / Letta (主 19:33 真借鉴):
- LangChain Memory 绑定 LCEL, 我们要纯 Python stdlib 可跑
- Letta 强绑 LLM provider, Phase 2 之前不绑
- DeltaMemory 2024 (Lin et al.) 的 salience decay 可以借鉴但实现从零写
- SQLite FTS5 BM25 是 builtin (Python 3.13 sqlite3 编译含), 零依赖
"""
from __future__ import annotations

import json
import sqlite3
import time
import uuid
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Iterable, List, Optional

DEFAULT_DB = Path.home() / ".apeireth_mvp" / "mvp.db"
MAX_EPISODES = 200  # rolling window per session (主 17:43 实事求是: 200 条够用)
MIN_CONFIDENCE = 0.1  # Note 遗忘阈值


@dataclass
class Episode:
    id: str
    timestamp: float
    role: str  # "user" | "agent" | "system"
    content: str
    session_id: str
    salience: float = 1.0  # 初始 1.0, 随时间衰减 (retrieve.py 计算)

    def to_row(self) -> tuple:
        return (self.id, self.timestamp, self.role, self.content,
                self.session_id, self.salience)


@dataclass
class Note:
    id: str
    timestamp: float
    content: str
    source_episode_ids: List[str] = field(default_factory=list)
    confidence: float = 0.8
    tags: List[str] = field(default_factory=list)

    def to_row(self) -> tuple:
        return (self.id, self.timestamp, self.content,
                json.dumps(self.source_episode_ids, ensure_ascii=False),
                self.confidence, json.dumps(self.tags, ensure_ascii=False))


class Store:
    """SQLite-backed memory store.

    Ponytail: thin wrapper around sqlite3, no ORM. Schema is created on
    first connect. Episodes are append-only; Notes are mutable.
    """

    # Ponytail: FTS5 unicode61 不分中文 (Python 3.13 sqlite3 3.50.4 已知限制),
    # 改用 LIKE 扫描 + BM25-like score. 数据量 ≤200 episodes 性能可接受.
    # 中文/英文都支持. 真借鉴: PostgreSQL pg_trgm + jieba 分词的轻量替代.
    SCHEMA = """
    CREATE TABLE IF NOT EXISTS episodes (
        id         TEXT PRIMARY KEY,
        timestamp  REAL NOT NULL,
        role       TEXT NOT NULL,
        content    TEXT NOT NULL,
        session_id TEXT NOT NULL,
        salience   REAL DEFAULT 1.0
    );
    CREATE INDEX IF NOT EXISTS ix_episodes_session_ts
        ON episodes(session_id, timestamp);

    CREATE TABLE IF NOT EXISTS notes (
        id                  TEXT PRIMARY KEY,
        timestamp           REAL NOT NULL,
        content             TEXT NOT NULL,
        source_episode_ids  TEXT NOT NULL DEFAULT '[]',
        confidence          REAL DEFAULT 0.8,
        tags                TEXT NOT NULL DEFAULT '[]'
    );
    CREATE INDEX IF NOT EXISTS ix_notes_ts ON notes(timestamp DESC);

    CREATE TABLE IF NOT EXISTS sessions (
        id         TEXT PRIMARY KEY,
        started_at REAL NOT NULL,
        last_seen  REAL NOT NULL,
        summary    TEXT DEFAULT ''
    );
    """

    def __init__(self, db_path: str | Path = DEFAULT_DB):
        self.db_path = Path(db_path)
        self.db_path.parent.mkdir(parents=True, exist_ok=True)
        self._conn = sqlite3.connect(str(self.db_path))
        self._conn.row_factory = sqlite3.Row
        self._conn.executescript(self.SCHEMA)
        self._conn.commit()

    def close(self) -> None:
        self._conn.close()

    # ----- Session -----
    def start_session(self, session_id: Optional[str] = None) -> str:
        sid = session_id or uuid.uuid4().hex[:12]
        now = time.time()
        self._conn.execute(
            "INSERT OR IGNORE INTO sessions(id, started_at, last_seen) "
            "VALUES (?, ?, ?)",
            (sid, now, now),
        )
        self._conn.execute(
            "UPDATE sessions SET last_seen = ? WHERE id = ?", (now, sid)
        )
        self._conn.commit()
        return sid

    def last_session(self) -> Optional[str]:
        row = self._conn.execute(
            "SELECT id FROM sessions ORDER BY last_seen DESC LIMIT 1"
        ).fetchone()
        return row["id"] if row else None

    # ----- Episode -----
    def append_episode(self, role: str, content: str,
                       session_id: str,
                       salience: float = 1.0) -> Episode:
        ep = Episode(
            id=uuid.uuid4().hex[:12],
            timestamp=time.time(),
            role=role,
            content=content,
            session_id=session_id,
            salience=salience,
        )
        self._conn.execute(
            "INSERT INTO episodes(id, timestamp, role, content, session_id, salience)"
            " VALUES (?, ?, ?, ?, ?, ?)",
            ep.to_row(),
        )
        self._conn.commit()
        self._enforce_rolling_window(session_id)
        return ep

    def list_episodes(self, session_id: Optional[str] = None,
                      limit: int = 50) -> List[Episode]:
        if session_id is None:
            rows = self._conn.execute(
                "SELECT * FROM episodes ORDER BY timestamp DESC LIMIT ?",
                (limit,),
            ).fetchall()
        else:
            rows = self._conn.execute(
                "SELECT * FROM episodes WHERE session_id = ? "
                "ORDER BY timestamp DESC LIMIT ?",
                (session_id, limit),
            ).fetchall()
        return [self._row_to_episode(r) for r in rows]

    def _enforce_rolling_window(self, session_id: str) -> None:
        """Ponytail: keep last MAX_EPISODES per session, drop oldest.
        cutoff = "保留的最小 timestamp" = 排序后第 (MAX_EPISODES-1) 旧的 ASC.
        """
        count = self._conn.execute(
            "SELECT COUNT(*) AS c FROM episodes WHERE session_id = ?",
            (session_id,),
        ).fetchone()["c"]
        if count <= MAX_EPISODES:
            return
        # ORDER BY DESC LIMIT MAX_EPISODES → top MAX_EPISODES newest.
        # MIN of that = "保留的最小 timestamp" (boundary).
        cutoff_ts = self._conn.execute(
            "SELECT MIN(timestamp) FROM ("
            "  SELECT timestamp FROM episodes WHERE session_id = ? "
            "  ORDER BY timestamp DESC LIMIT ?)",
            (session_id, MAX_EPISODES),
        ).fetchone()[0]
        self._conn.execute(
            "DELETE FROM episodes WHERE session_id = ? AND timestamp < ?",
            (session_id, cutoff_ts),
        )
        self._conn.commit()

    @staticmethod
    def _row_to_episode(row: sqlite3.Row) -> Episode:
        return Episode(
            id=row["id"],
            timestamp=row["timestamp"],
            role=row["role"],
            content=row["content"],
            session_id=row["session_id"],
            salience=row["salience"],
        )

    # ----- Note -----
    def add_note(self, content: str,
                 source_episode_ids: Optional[Iterable[str]] = None,
                 confidence: float = 0.8,
                 tags: Optional[Iterable[str]] = None) -> Note:
        note = Note(
            id=uuid.uuid4().hex[:12],
            timestamp=time.time(),
            content=content,
            source_episode_ids=list(source_episode_ids or []),
            confidence=confidence,
            tags=list(tags or []),
        )
        self._conn.execute(
            "INSERT INTO notes(id, timestamp, content, source_episode_ids,"
            " confidence, tags) VALUES (?, ?, ?, ?, ?, ?)",
            note.to_row(),
        )
        self._conn.commit()
        return note

    def merge_note(self, note_id: str, new_content: str,
                   bump_confidence: float = 0.05) -> Optional[Note]:
        """Merge new evidence into existing note (主 17:43 实事求是: 保留源溯)."""
        row = self._conn.execute(
            "SELECT * FROM notes WHERE id = ?", (note_id,)
        ).fetchone()
        if not row:
            return None
        old = json.loads(row["source_episode_ids"])
        merged = Note(
            id=row["id"],
            timestamp=time.time(),
            content=new_content,
            source_episode_ids=old,
            confidence=min(1.0, row["confidence"] + bump_confidence),
            tags=json.loads(row["tags"]),
        )
        self._conn.execute(
            "UPDATE notes SET timestamp = ?, content = ?, confidence = ? "
            "WHERE id = ?",
            (merged.timestamp, merged.content, merged.confidence, merged.id),
        )
        self._conn.commit()
        return merged

    def forget_low_confidence(self, threshold: float = MIN_CONFIDENCE) -> int:
        cur = self._conn.execute(
            "DELETE FROM notes WHERE confidence < ?", (threshold,)
        )
        self._conn.commit()
        return cur.rowcount

    def list_notes(self, limit: int = 50) -> List[Note]:
        rows = self._conn.execute(
            "SELECT * FROM notes ORDER BY timestamp DESC LIMIT ?", (limit,)
        ).fetchall()
        return [self._row_to_note(r) for r in rows]

    @staticmethod
    def _row_to_note(row: sqlite3.Row) -> Note:
        return Note(
            id=row["id"],
            timestamp=row["timestamp"],
            content=row["content"],
            source_episode_ids=json.loads(row["source_episode_ids"]),
            confidence=row["confidence"],
            tags=json.loads(row["tags"]),
        )

    # ----- Stats -----
    def stats(self) -> dict:
        ep = self._conn.execute("SELECT COUNT(*) AS c FROM episodes").fetchone()["c"]
        nt = self._conn.execute("SELECT COUNT(*) AS c FROM notes").fetchone()["c"]
        ss = self._conn.execute("SELECT COUNT(*) AS c FROM sessions").fetchone()["c"]
        return {"episodes": ep, "notes": nt, "sessions": ss}