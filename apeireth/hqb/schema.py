"""HQB (Harness Quality Benchmark) SQLite Schema — R3-DB-01.

依据:
- R2-REQ-01 V1085/V1086 HQB 真生产化方向 (V160 HQB 4 维 SC/NR/EV/CDT 接入 V1074 runner)
- HARNESS.md §2.3/§4/§5 (Harness 修改必须 HQB 量化)
- 现仓模式 (memory_store.py / relation_store.py):
  * sqlite3 stdlib
  * SCHEMA 字符串 + CREATE TABLE IF NOT EXISTS
  * <name>_meta 单表存 schema_version + idempotent migrate

为什么新增而非合并 memory_store:
- HQB 决策/守门/Δ/trace 是独立审计轨迹, 与 memory episode/relation graph 关注点不同
- 独立 DB 文件 (hqb.db) 让审计可单独 export/rotate (主 21:15 "最细颗粒度审计")

4 表职责:
- hqb_decisions: 每次 HQB 评分决策 (id + task_id + score + philosophy_guard_status + snapshot_score)
- hqb_guard_events: 决策触发的守门事件 (哲学/安全/evidence 三类)
- hqb_asi_deltas: 决策前后的 ASI V0.3 变化 (归因 lift_value)
- hqb_trace: 决策树 (parent_id 串成追溯链) — 借鉴 code-deep-study/letta 的 messages 自引用
"""

from __future__ import annotations

import sqlite3
import time
import uuid
from pathlib import Path
from typing import Any, Dict, List, Optional


SCHEMA_VERSION = "0.1.0"


SCHEMA = f"""
-- HQB 真生产 schema v{SCHEMA_VERSION} (R3-DB-01)
-- 仅 CREATE IF NOT EXISTS — 不破坏现仓 memory.db / graph.db / identity.db

CREATE TABLE IF NOT EXISTS hqb_decisions (
    id              TEXT PRIMARY KEY,        -- uuid
    task_id         TEXT NOT NULL,           -- 关联 V1074 runner 的 task
    decision        TEXT NOT NULL,           -- 'allow' | 'block' | 'flag'
    score           REAL NOT NULL,           -- HQB 4 维聚合分 (0..1)
    philosophy_guard_status TEXT NOT NULL,    -- 'pass' | 'fail' | 'skip'
    snapshot_score  REAL NOT NULL,           -- 决策时刻 ASI V0.3 真测
    ts              REAL NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_hqb_decisions_task ON hqb_decisions(task_id);
CREATE INDEX IF NOT EXISTS idx_hqb_decisions_ts   ON hqb_decisions(ts);

CREATE TABLE IF NOT EXISTS hqb_guard_events (
    id          TEXT PRIMARY KEY,
    decision_id TEXT NOT NULL,
    guard_type  TEXT NOT NULL,               -- 'philosophy' | 'safety' | 'evidence' | 'cost'
    passed      INTEGER NOT NULL,            -- 0 | 1 (SQLite 无 bool)
    reason      TEXT NOT NULL DEFAULT '',
    ts          REAL NOT NULL,
    FOREIGN KEY (decision_id) REFERENCES hqb_decisions(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_hqb_guard_decision ON hqb_guard_events(decision_id);
CREATE INDEX IF NOT EXISTS idx_hqb_guard_type     ON hqb_guard_events(guard_type);

CREATE TABLE IF NOT EXISTS hqb_asi_deltas (
    id            TEXT PRIMARY KEY,
    decision_id   TEXT NOT NULL,
    asiv0_before  REAL NOT NULL,
    asiv0_after   REAL NOT NULL,
    lift_value    REAL NOT NULL,             -- after - before
    ts            REAL NOT NULL,
    FOREIGN KEY (decision_id) REFERENCES hqb_decisions(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_hqb_delta_decision ON hqb_asi_deltas(decision_id);
CREATE INDEX IF NOT EXISTS idx_hqb_delta_ts       ON hqb_asi_deltas(ts);

CREATE TABLE IF NOT EXISTS hqb_trace (
    id        TEXT PRIMARY KEY,
    parent_id TEXT,                          -- 树形追溯链 (根节点 NULL)
    action    TEXT NOT NULL,
    rationale TEXT NOT NULL DEFAULT '',
    ts        REAL NOT NULL,
    FOREIGN KEY (parent_id) REFERENCES hqb_trace(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_hqb_trace_parent ON hqb_trace(parent_id);
CREATE INDEX IF NOT EXISTS idx_hqb_trace_ts     ON hqb_trace(ts);

CREATE TABLE IF NOT EXISTS hqb_meta (
    k TEXT PRIMARY KEY,
    v TEXT NOT NULL
);
"""


class HqbStore:
    """SQLite backend for HQB 真生产审计. Usage: HqbStore('hqb.db') | HqbStore(':memory:')."""

    def __init__(self, path: str | Path):
        self._is_memory = path == ":memory:"
        if not self._is_memory:
            p = Path(path); p.parent.mkdir(parents=True, exist_ok=True)
            self.path = p
        else:
            self.path = ":memory:"  # type: ignore[assignment]
        self._conn = sqlite3.connect(str(self.path))
        self._conn.execute("PRAGMA journal_mode=WAL")
        self._conn.execute("PRAGMA foreign_keys=ON")
        self._conn.row_factory = sqlite3.Row
        self._init_schema()
        self._seed_meta()

    def _init_schema(self) -> None:
        self._conn.executescript(SCHEMA); self._conn.commit()

    def _seed_meta(self) -> None:
        if self._conn.execute("SELECT v FROM hqb_meta WHERE k='schema_version'").fetchone() is None:
            self._conn.execute("INSERT INTO hqb_meta(k,v) VALUES (?,?)", ("schema_version", SCHEMA_VERSION))
            self._conn.commit()

    # ---- writes ----

    def record_decision(self, task_id: str, decision: str, score: float,
                        philosophy_guard_status: str, snapshot_score: float,
                        ts: Optional[float] = None) -> str:
        did = str(uuid.uuid4())
        self._conn.execute("INSERT INTO hqb_decisions VALUES (?,?,?,?,?,?,?)",
            (did, task_id, decision, score, philosophy_guard_status, snapshot_score, ts or time.time()))
        self._conn.commit(); return did

    def record_guard(self, decision_id: str, guard_type: str, passed: bool, reason: str = "") -> str:
        gid = str(uuid.uuid4())
        self._conn.execute("INSERT INTO hqb_guard_events VALUES (?,?,?,?,?,?)",
            (gid, decision_id, guard_type, 1 if passed else 0, reason, time.time()))
        self._conn.commit(); return gid

    def record_delta(self, decision_id: str, asiv0_before: float, asiv0_after: float,
                     ts: Optional[float] = None) -> str:
        did = str(uuid.uuid4())
        self._conn.execute("INSERT INTO hqb_asi_deltas VALUES (?,?,?,?,?,?)",
            (did, decision_id, asiv0_before, asiv0_after, asiv0_after - asiv0_before, ts or time.time()))
        self._conn.commit(); return did

    def record_trace(self, action: str, rationale: str = "", parent_id: Optional[str] = None) -> str:
        tid = str(uuid.uuid4())
        self._conn.execute("INSERT INTO hqb_trace VALUES (?,?,?,?,?)",
            (tid, parent_id, action, rationale, time.time()))
        self._conn.commit(); return tid

    # ---- reads ----

    def get_decision(self, decision_id: str) -> Optional[Dict[str, Any]]:
        cur = self._conn.execute("SELECT * FROM hqb_decisions WHERE id=?", (decision_id,))
        row = cur.fetchone()
        return dict(row) if row else None

    def list_decisions(self, limit: int = 100) -> List[Dict[str, Any]]:
        cur = self._conn.execute(
            "SELECT * FROM hqb_decisions ORDER BY ts DESC LIMIT ?", (limit,)
        )
        return [dict(r) for r in cur.fetchall()]

    def list_guards_for(self, decision_id: str) -> List[Dict[str, Any]]:
        cur = self._conn.execute(
            "SELECT * FROM hqb_guard_events WHERE decision_id=? ORDER BY ts", (decision_id,)
        )
        return [dict(r) for r in cur.fetchall()]

    def list_deltas_for(self, decision_id: str) -> List[Dict[str, Any]]:
        cur = self._conn.execute(
            "SELECT * FROM hqb_asi_deltas WHERE decision_id=?", (decision_id,)
        )
        return [dict(r) for r in cur.fetchall()]

    def schema_version(self) -> str:
        cur = self._conn.execute("SELECT v FROM hqb_meta WHERE k='schema_version'")
        row = cur.fetchone()
        return row[0] if row else "0.0.0"

    # ---- delete ----

    def delete_decision(self, decision_id: str) -> int:
        # ON DELETE CASCADE 自动清 guards + deltas
        cur = self._conn.execute("DELETE FROM hqb_decisions WHERE id=?", (decision_id,))
        self._conn.commit()
        return cur.rowcount

    def close(self) -> None:
        self._conn.close()