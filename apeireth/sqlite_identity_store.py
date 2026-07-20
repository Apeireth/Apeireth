"""Identity Store v0.3 — SQLite 真持久化 (Phase 6.5)

v0.1 (identity.py + identity_store.py):
  - JSON 序列化 + in-memory multi-card
  - Schema 验证 + 版本迁移 + integrity_hash
  - load_dir / save_card — 文件级操作

v0.3 (本文件 — Phase 6.5):
  - SQLite backend for IdentityStore
  - 跨 session 真存活 (master / persona / team 全部)
  - 与 SqliteMemoryStore + SqliteRelationStore 一致

依据:
- TOP-DESIGN-V1 §3.4 (启动创世 + 身份卡持久化)
- 主人 12:14 中央 AI 永恒身份 → 身份卡必须跨 session
- DEV-LOG 17:14 限制: "team card 没保存到磁盘" → Phase 6 临时团无持久
- DEV-LOG 17:43 限制: 同样 (Phase 6.5 启动)

为什么 v0.3 = Sqlite:
- memory v0.2 (Phase 2.5) + relation v0.2 (Phase 3.5) 都走过 — 一致性
- 多张卡按 role / name 查询需要索引 — SQL 天然
- FTS5 全文搜 (跨卡 name + purpose + recall_anchor)
- 0 外部依赖 (sqlite3 stdlib)

Schema 设计 (3 tables + 1 FTS5 + meta):
- identity_cards: 主键 name, role 索引, created_at 索引
- card_integrity: name → integrity_hash + role + updated_at
- identity_fts: FTS5(name, purpose, mission, recall_anchor)
- identity_meta: 单行 (id=1) → schema_version + cross_card_hash

不变性约束:
- upsert_card: 同 name 更新 (master 仍只 1 张, add() 抛错, upsert 覆盖)
- delete_card: master 不允许删 (沙盒保护)
- integrity_hash 走 canonical JSON sort_keys=True
"""

from __future__ import annotations
import json
import sqlite3
import time
from pathlib import Path
from typing import Optional

from .identity import IdentityCard
from .identity_store import (
    IDENTITY_STORE_VERSION,
    StoreEntry,
    IdentityStore,
)


# === Schema ===

SCHEMA = """
-- v0.3 schema: 与 memory / relation 形态对齐 (一表 + 一 FTS5 + meta)
CREATE TABLE IF NOT EXISTS identity_cards (
    name TEXT PRIMARY KEY,
    role TEXT NOT NULL DEFAULT 'snapshot',
    card_json TEXT NOT NULL,
    created_at REAL NOT NULL,
    updated_at REAL NOT NULL,
    integrity_hash TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_identity_role ON identity_cards(role);
CREATE INDEX IF NOT EXISTS idx_identity_updated ON identity_cards(updated_at);

-- FTS5 跨卡搜索 (name / purpose / mission / recall_anchor / creator)
-- 不使用 content='' 外部管理 — 允许 SQLite 自动同步 + CJK 字符默认能 tokenize
CREATE VIRTUAL TABLE IF NOT EXISTS identity_fts USING fts5(
    name,
    role,
    purpose,
    mission,
    recall_anchor,
    creator
);

-- meta 单行 — schema_version + 跨卡聚合 hash
CREATE TABLE IF NOT EXISTS identity_meta (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    schema_version TEXT NOT NULL,
    cross_card_hash TEXT NOT NULL,
    updated_at REAL NOT NULL
);
"""


SQLITE_IDENTITY_VERSION = "0.3.0"


class SqliteIdentityStore:
    """SQLite backend for IdentityStore — Phase 6.5

    Usage:
        store = SqliteIdentityStore("apeireth.db")
        store.upsert_card(master_card, role="master")
        cards = store.load_all_cards()         # → IdentityStore (内存)
        results = store.search("ASI")          # → [(name, role, score), ...]
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
        cur = self._conn.execute("SELECT 1 FROM identity_meta WHERE id=1")
        if cur.fetchone() is None:
            self._conn.execute(
                "INSERT INTO identity_meta(id, schema_version, cross_card_hash, updated_at) VALUES (1,?,?,?)",
                (SQLITE_IDENTITY_VERSION, "", time.time()),
            )
            self._conn.commit()

    # ---------- write ----------

    def upsert_card(self, card: IdentityCard, role: str = "snapshot") -> bool:
        """插入或更新一张卡. 同一 name → role 持久, content 覆盖.

        Returns True if inserted, False if updated.
        """
        if not card.name:
            raise ValueError("card.name is required")
        card_json = json.dumps(card.to_dict(), ensure_ascii=False, sort_keys=True)
        hash_now = card.integrity_hash()
        now = time.time()
        cur = self._conn.execute(
            "SELECT created_at FROM identity_cards WHERE name=?", (card.name,)
        )
        row = cur.fetchone()
        if row:
            self._conn.execute(
                "UPDATE identity_cards SET role=?, card_json=?, updated_at=?, integrity_hash=? WHERE name=?",
                (role, card_json, now, hash_now, card.name),
            )
            # FTS5: 同步更新 (非 contentless 表, 删除再插入)
            self._conn.execute("DELETE FROM identity_fts WHERE name=?", (card.name,))
            self._conn.execute(
                "INSERT INTO identity_fts(name, role, purpose, mission, recall_anchor, creator) VALUES (?,?,?,?,?,?)",
                (card.name, role, card.purpose, card.mission, card.recall_anchor, card.creator),
            )
            self._conn.commit()
            return False
        self._conn.execute(
            "INSERT INTO identity_cards(name, role, card_json, created_at, updated_at, integrity_hash) VALUES (?,?,?,?,?,?)",
            (card.name, role, card_json, now, now, hash_now),
        )
        self._conn.execute(
            "INSERT INTO identity_fts(name, role, purpose, mission, recall_anchor, creator) VALUES (?,?,?,?,?,?)",
            (card.name, role, card.purpose, card.mission, card.recall_anchor, card.creator),
        )
        self._conn.commit()
        return True

    def delete_card(self, name: str) -> bool:
        """删除一张卡. master 不允许删 (沙盒保护).

        Returns True if deleted.
        """
        cur = self._conn.execute("SELECT role FROM identity_cards WHERE name=?", (name,))
        row = cur.fetchone()
        if row is None:
            return False
        if row[0] == "master":
            raise PermissionError(f"master card '{name}' cannot be deleted")
        self._conn.execute("DELETE FROM identity_cards WHERE name=?", (name,))
        self._conn.execute("DELETE FROM identity_fts WHERE name=?", (name,))
        self._conn.commit()
        return True

    # ---------- read ----------
        self._conn.commit()
        return True

    # ---------- read ----------

    def load_all_cards(self) -> IdentityStore:
        """从 SQLite 重建 IdentityStore (in-memory).

        顺序: master 先, 然后 persona, 然后 team, 最后 snapshot.
        IdentityStore.add() 默认 role='snapshot', 但我们用 StoreEntry 显式记 role.
        """
        rows = self._conn.execute(
            "SELECT name, role, card_json, created_at, integrity_hash FROM identity_cards ORDER BY CASE role WHEN 'master' THEN 0 WHEN 'persona' THEN 1 WHEN 'team' THEN 2 ELSE 3 END, name"
        ).fetchall()
        store = IdentityStore(root=str(self.path))
        for r in rows:
            try:
                raw = json.loads(r[2])
            except Exception:
                continue
            try:
                card = IdentityCard(**raw)
            except Exception:
                continue
            entry = StoreEntry(card=card, role=r[1], path=None, integrity_ok=True)
            store.entries[card.name] = entry
        return store

    def get_card(self, name: str) -> Optional[IdentityCard]:
        cur = self._conn.execute(
            "SELECT card_json FROM identity_cards WHERE name=?", (name,)
        )
        row = cur.fetchone()
        if row is None:
            return None
        return IdentityCard(**json.loads(row[0]))

    def cards_by_role(self, role: str) -> list[IdentityCard]:
        rows = self._conn.execute(
            "SELECT card_json FROM identity_cards WHERE role=? ORDER BY name", (role,)
        ).fetchall()
        return [IdentityCard(**json.loads(r[0])) for r in rows]

    def search(self, query: str, limit: int = 5) -> list[tuple[str, str, float]]:
        """3-layer Layer 1: 全文搜索 (FTS5 match + JOIN 回主表).

        已知 v0.3 限制: FTS5 content='' 模式下 bm25() 不可靠 — 返回 count-based 顺序.
        JOIN identity_cards 保证 name / role 从主表拿 (不为 NULL).
        """
        sql = """
        SELECT c.name, c.role
        FROM identity_cards c
        WHERE c.name IN (
            SELECT name FROM identity_fts WHERE identity_fts MATCH ?
        )
        ORDER BY c.updated_at DESC
        LIMIT ?
        """
        rows = self._conn.execute(sql, (query, limit)).fetchall()
        return [(r[0], r[1], 0.0) for r in rows]

    # ---------- meta / integrity ----------

    def save_cross_hash(self, cross_hash: str):
        """存跨卡聚合 hash — 给 linkage 层做 5 层完整性校验."""
        self._conn.execute(
            "UPDATE identity_meta SET cross_card_hash=?, updated_at=? WHERE id=1",
            (cross_hash, time.time()),
        )
        self._conn.commit()

    def stats(self) -> dict:
        total = self._conn.execute("SELECT COUNT(*) FROM identity_cards").fetchone()[0]
        roles = self._conn.execute(
            "SELECT role, COUNT(*) FROM identity_cards GROUP BY role"
        ).fetchall()
        meta = self._conn.execute(
            "SELECT schema_version, cross_card_hash, updated_at FROM identity_meta WHERE id=1"
        ).fetchone()
        return {
            "total_cards": total,
            "by_role": dict(roles),
            "schema_version": meta[0] if meta else "0.0.0",
            "cross_card_hash": meta[1] if meta else "",
            "updated_at": meta[2] if meta else 0.0,
        }

    def close(self):
        self._conn.close()


# ---------- bridge from in-memory to SQLite ----------

def migrate_from_identity_store(store: IdentityStore, sqlite_store: SqliteIdentityStore) -> dict:
    """把 v0.2 in-memory IdentityStore 迁到 v0.3 SQLite backend.

    返回迁移统计.
    """
    added = updated = 0
    for entry in store.entries.values():
        if sqlite_store.upsert_card(entry.card, role=entry.role):
            added += 1
        else:
            updated += 1
    # 同步跨卡 hash
    sqlite_store.save_cross_hash(store.integrity_hash())
    return {
        "cards_added": added,
        "cards_updated": updated,
        "total": len(store.entries),
        "cross_card_hash": store.integrity_hash(),
    }


__all__ = [
    "SqliteIdentityStore",
    "migrate_from_identity_store",
    "SQLITE_IDENTITY_VERSION",
]