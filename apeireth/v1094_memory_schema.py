"""V1094 Memory Module Schema — R8-TrackA3 (HotCold + WAL + Replay + Dream).

依据 (R8-TrackA3 任务描述):
  - HotCold/WAL → MemoryReplay → Dream 真生产底层 storage schema
  - 兼容现仓 memory.db (memory_store.py v0.3) + hqb.db (hqb/schema.py v0.1)
  - 借鉴 memoryos-rust crates/memoryos-core/src/memory/mod.rs STM/MTM/LTM
  - 借鉴 v1072_eternal_identity.py LTM/MTM/STM 三层语义
  - 借鉴 memory_replay_design.py IDEMPOTENT_OPS = {tag_set, anchor_link,
    anchor_unlink, score_record, phase_emit, trace_record}
  - 借鉴 R7-BE-01 dream-design (DreamState IDLE/DREAMING/CONSOLIDATING/
    FORGETTING/VERIFYING/INTERRUPTED)

设计原则 (主 22:33 ASI 北极星 + V3 守门 + 真生产不停):
  1. CREATE TABLE IF NOT EXISTS — 不破坏旧 schema
  2. _meta 单表存 schema_version + idempotent migrate
  3. 8 表职责单一, FK + INDEX 明确
  4. WAL 必 append + checksum, hot ↔ cold 迁移可审计
  5. _dream=true 标记产自 Dream 子系统, Replay 不修改 _dream=true 行

8 表职责:
  memory_hot         当前 session episode (rolling buffer; tier='hot')
  memory_cold        跨 session 持久 (tier='cold', long-term 锚点 + Note)
  memory_wal         写前日志 (append+checksum, applied=0/1 标志)
  memory_dream       dream 产出暂存 (_dream=true, status=pending/consumed/tombstoned)
  memory_snapshots   replay 状态快照 (scope+seq 唯一, 借鉴 letta/messages 自引用)
  stm_messages       STM 短期对话消息 (rolling N=50, session_id 关联)
  mtm_themes         MTM 中期主题聚合 (topic_id 唯一, last_updated DESC)
  ltm_facts          LTM 长期事实 (fingerprint UNIQUE, category+importance 索引)
  memory_meta        schema_version=0.1.0

对接点:
  - v1072_eternal_identity.IdentityManifest.add(source=LTM/MTM/STM) →
        ltm_facts / mtm_themes / stm_messages (source='eternal_identity')
  - hqb_integration.HQBIntegration.record_* →
        memory_snapshots(scope='hqb', snapshot_score)
  - memory_replay_design.IDEMPOTENT_OPS →
        memory_wal(kind IN {tag_set, anchor_link, ...}, applied=0/1)
  - R7-BE-01 DreamSubsystem → memory_dream(_dream=true)

执行迁移:
  from apeireth.v1094_memory_schema import MemorySchema, upgrade
  MemorySchema(":memory:").init()        # 一次性 init (CREATE IF NOT EXISTS)
  upgrade("apeireth.db")                  # 幂等升级
  downgrade("apeireth.db")                # 谨慎回退 (DROP TABLE)

V1094_VERSION = "0.1.0" — R8-TrackA3 初版
"""

from __future__ import annotations

import sqlite3
import time
import uuid
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional


V1094_VERSION = "0.1.0"


# ---------------------------------------------------------------------------
# SQL 常量 (借鉴 hqb/schema.py 单一 SCHEMA 字符串 + CREATE IF NOT EXISTS)
# ---------------------------------------------------------------------------


# 8 表 + meta — 顺序无关 (FK 全 ON DELETE CASCADE / SET NULL)
SCHEMA_V094 = f"""
-- V1094 Memory Schema v{V1094_VERSION} (R8-TrackA3)
-- 兼容现仓: memory_store.py v0.3 + hqb/schema.py v0.1 — 不 DROP 既有
-- 唯一新增 8 表, 表前缀 memory_/stm/mtm/ltm 不与 episodes/notes/hqb_* 冲突

-- 1) Hot tier — 当前 session episode (rolling buffer; TTL 由上层管理)
CREATE TABLE IF NOT EXISTS memory_hot (
    id              TEXT PRIMARY KEY,
    session_id      TEXT NOT NULL,
    actor           TEXT NOT NULL,                       -- 'master'/'apeireth'/'tool'
    content         TEXT NOT NULL,
    context         TEXT NOT NULL DEFAULT '',
    kind            TEXT NOT NULL DEFAULT 'utterance',   -- utterance/tool_call/observation/kickoff
    tier            TEXT NOT NULL DEFAULT 'hot',         -- always 'hot' (denormalized for index)
    ts              REAL NOT NULL,
    fingerprint     TEXT NOT NULL,                       -- sha256[:16] dedup key (借鉴 mem0)
    linked_identity_hash TEXT NOT NULL DEFAULT '',      -- 触发时 V1072 IdentityCard hash
    observation_date REAL                                -- mem0 temporal grounding (None=ts)
);

-- 2) Cold tier — 跨 session 持久 (LTM 锚点 + Note; 永不丢)
CREATE TABLE IF NOT EXISTS memory_cold (
    id              TEXT PRIMARY KEY,
    session_id      TEXT NOT NULL DEFAULT '',            -- 来源 session
    actor           TEXT NOT NULL DEFAULT 'apeireth',
    content         TEXT NOT NULL,
    context         TEXT NOT NULL DEFAULT '',
    kind            TEXT NOT NULL DEFAULT 'note',        -- note/anchor/fact/decision
    tier            TEXT NOT NULL DEFAULT 'cold',        -- always 'cold'
    category        TEXT NOT NULL DEFAULT 'fact',        -- identity/decision/value/event/fact (V1072)
    importance      INTEGER NOT NULL DEFAULT 5,          -- 0..10 (借鉴 memory_3tier LTM_ANCHOR_MIN_IMPORTANCE=8)
    confidence      REAL NOT NULL DEFAULT 0.5,           -- 0..1 (Bayesian)
    ts              REAL NOT NULL,
    fingerprint     TEXT NOT NULL,                       -- sha256[:16] dedup key
    linked_identity_hash TEXT NOT NULL DEFAULT '',
    observation_date REAL,
    superseded_by   TEXT                                 -- 旧版本被新版本替换的 id (借maz hole Parfit psychological continuity)
);

-- 3) WAL — 写前日志 (append+checksum, applied=0/1 标志; 借鉴 Tonbo LSM)
CREATE TABLE IF NOT EXISTS memory_wal (
    seq             INTEGER PRIMARY KEY AUTOINCREMENT,   -- 单调递增 (StateID.seq)
    scope           TEXT NOT NULL,                       -- 'stm'/'mtm'/'ltm'/'hot'/'cold'/'dream'/'snapshot'
    op              TEXT NOT NULL,                       -- IDEMPOTENT_OPS whitelist
    payload         TEXT NOT NULL,                       -- JSON-serializable
    event_id        TEXT NOT NULL,                       -- uuid (幂等键)
    checksum        TEXT NOT NULL,                       -- sha256(payload)[:16]
    applied         INTEGER NOT NULL DEFAULT 0,          -- 0=pending, 1=applied
    ts              REAL NOT NULL,
    applied_ts      REAL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_wal_event ON memory_wal(event_id);  -- 幂等键

-- 4) Dream candidates — dream 产出暂存 (_dream=true 标记, Replay 不修改)
CREATE TABLE IF NOT EXISTS memory_dream (
    id              TEXT PRIMARY KEY,
    source_episode_id TEXT,                              -- 来源 episode (FK→memory_hot.id 可空)
    summary         TEXT NOT NULL,
    confidence      REAL NOT NULL DEFAULT 0.5,
    importance      INTEGER NOT NULL DEFAULT 5,
    status          TEXT NOT NULL DEFAULT 'pending',     -- pending/consumed/tombstoned
    dream_state     TEXT NOT NULL DEFAULT 'CONSOLIDATING', -- IDLE/DREAMING/CONSOLIDATING/FORGETTING/VERIFYING/INTERRUPTED
    ts              REAL NOT NULL,
    consumed_ts     REAL
);

-- 5) Snapshots — replay 状态快照 (scope+seq 唯一; 借鉴 letta/messages self-ref)
CREATE TABLE IF NOT EXISTS memory_snapshots (
    id              TEXT PRIMARY KEY,
    scope           TEXT NOT NULL,                       -- 'stm'/'mtm'/'ltm'/'hot'/'cold'/'hqb'
    seq             INTEGER NOT NULL,                    -- StateID.seq 单调
    content_hash    TEXT NOT NULL,                       -- sha256[:16] snapshot 内容指纹
    rationale       TEXT NOT NULL DEFAULT '',
    identity_hash   TEXT NOT NULL DEFAULT '',            -- 触发时的 V1072 IdentityCard hash
    snapshot_score  REAL,                                -- 决策时刻 ASI V0.3 真测
    ts              REAL NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_snapshot_scope_seq ON memory_snapshots(scope, seq);

-- 6) STM messages — 短期对话消息 (rolling N=50 借鉴 memory_3tier.STM_MAX_SIZE)
CREATE TABLE IF NOT EXISTS stm_messages (
    id              TEXT PRIMARY KEY,
    session_id      TEXT NOT NULL,
    role            TEXT NOT NULL,                       -- 'user'/'assistant'/'system'/'tool'
    content         TEXT NOT NULL,
    embedding_ref   TEXT,                                -- 外部向量库引用 (本表不存 vector; 仅 reference)
    ts              REAL NOT NULL,
    fingerprint     TEXT NOT NULL
);

-- 7) MTM themes — 中期主题聚合 (topic_id 唯一; last_updated DESC 索引)
CREATE TABLE IF NOT EXISTS mtm_themes (
    topic_id        TEXT PRIMARY KEY,
    topic_label     TEXT NOT NULL,
    n_episodes      INTEGER NOT NULL DEFAULT 0,
    importance_avg  REAL NOT NULL DEFAULT 0.0,
    summary         TEXT NOT NULL DEFAULT '',
    last_updated    REAL NOT NULL,
    fingerprint     TEXT NOT NULL
);

-- 8) LTM facts — 长期事实 (fingerprint UNIQUE; 永不丢)
CREATE TABLE IF NOT EXISTS ltm_facts (
    id              TEXT PRIMARY KEY,
    category        TEXT NOT NULL,                       -- identity/decision/value/event/fact
    content         TEXT NOT NULL,
    importance      INTEGER NOT NULL DEFAULT 5,          -- 0..10
    confidence      REAL NOT NULL DEFAULT 0.5,
    master_quoted   TEXT NOT NULL DEFAULT '',            -- 主人原话引用 (V1072 MemoryAnchor)
    fingerprint     TEXT NOT NULL,                       -- sha256(category+content)[:16]
    ts              REAL NOT NULL,
    observation_date REAL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_ltm_fingerprint ON ltm_facts(fingerprint);

-- 9) Meta — schema_version + 自描述
CREATE TABLE IF NOT EXISTS memory_meta (
    k TEXT PRIMARY KEY,
    v TEXT NOT NULL
);
"""


# 索引 (单独执行; 部分表内已建 UNIQUE 索引, 这里集中重复建保证幂等)
INDEXES_V094 = """
-- Hot tier 索引: 按 session + 时间 + actor 多维查询
CREATE INDEX IF NOT EXISTS idx_hot_session_ts  ON memory_hot(session_id, ts DESC);
CREATE INDEX IF NOT EXISTS idx_hot_tier_ts     ON memory_hot(tier, ts DESC);
CREATE INDEX IF NOT EXISTS idx_hot_fingerprint ON memory_hot(fingerprint);
CREATE INDEX IF NOT EXISTS idx_hot_identity    ON memory_hot(linked_identity_hash);

-- Cold tier 索引: 按 category + importance + 时间
CREATE INDEX IF NOT EXISTS idx_cold_category_imp ON memory_cold(category, importance DESC, ts DESC);
CREATE INDEX IF NOT EXISTS idx_cold_tier_ts      ON memory_cold(tier, ts DESC);
CREATE INDEX IF NOT EXISTS idx_cold_fingerprint  ON memory_cold(fingerprint);
CREATE INDEX IF NOT EXISTS idx_cold_identity     ON memory_cold(linked_identity_hash);
CREATE INDEX IF NOT EXISTS idx_cold_superseded   ON memory_cold(superseded_by);

-- WAL 索引: 按 scope + applied + 时间 (Replay 按时间窗扫 pending)
CREATE INDEX IF NOT EXISTS idx_wal_scope_applied_ts ON memory_wal(scope, applied, ts);
CREATE INDEX IF NOT EXISTS idx_wal_applied_ts       ON memory_wal(applied, ts);
CREATE INDEX IF NOT EXISTS idx_wal_op               ON memory_wal(op);

-- Dream 索引: pending 优先 + state 分布
CREATE INDEX IF NOT EXISTS idx_dream_status_ts  ON memory_dream(status, ts DESC);
CREATE INDEX IF NOT EXISTS idx_dream_state_ts   ON memory_dream(dream_state, ts DESC);
CREATE INDEX IF NOT EXISTS idx_dream_source     ON memory_dream(source_episode_id);

-- Snapshots 索引: 按 scope+seq 唯一 (已在表内); 追加时间序
CREATE INDEX IF NOT EXISTS idx_snapshot_ts      ON memory_snapshots(ts DESC);

-- STM 索引: session + 时间 (rolling N=50 按时间窗裁剪)
CREATE INDEX IF NOT EXISTS idx_stm_session_ts   ON stm_messages(session_id, ts DESC);
CREATE INDEX IF NOT EXISTS idx_stm_role         ON stm_messages(role);

-- MTM 索引: 按 last_updated DESC (主题热度); label 用于搜索
CREATE INDEX IF NOT EXISTS idx_mtm_label        ON mtm_themes(topic_label);
CREATE INDEX IF NOT EXISTS idx_mtm_last_updated ON mtm_themes(last_updated DESC);

-- LTM 索引: category + importance DESC (优先级); 身份哈希
CREATE INDEX IF NOT EXISTS idx_ltm_category_imp ON ltm_facts(category, importance DESC);
CREATE INDEX IF NOT EXISTS idx_ltm_ts           ON ltm_facts(ts DESC);
CREATE INDEX IF NOT EXISTS idx_ltm_observation  ON ltm_facts(observation_date);
"""


# ---------------------------------------------------------------------------
# 工具函数: 借鉴 hqb/schema.py + memory_store.py
# ---------------------------------------------------------------------------


def _checksum(payload: str) -> str:
    """WAL payload 校验和: sha256(payload)[:16]."""
    import hashlib
    return hashlib.sha256(payload.encode("utf-8", errors="ignore")).hexdigest()[:16]


def _fingerprint(*parts: str) -> str:
    """Episodes / Notes / Facts 指纹: sha256(joined)[:16]."""
    import hashlib
    canonical = "|".join((p or "").strip() for p in parts)
    return hashlib.sha256(canonical.encode("utf-8", errors="ignore")).hexdigest()[:16]


# ---------------------------------------------------------------------------
# 真迁移脚本: upgrade() / downgrade()
# ---------------------------------------------------------------------------


def upgrade(conn: sqlite3.Connection) -> None:
    """幂等升级 — CREATE IF NOT EXISTS, 多次执行结果一致.

    Args:
        conn: 目标 SQLite 连接 (调用方负责生命周期 + PRAGMA)
    """
    conn.executescript(SCHEMA_V094)
    conn.executescript(INDEXES_V094)
    # schema_version seed (只在首次插入; 既有不动 — 真生产兼容)
    cur = conn.execute("SELECT v FROM memory_meta WHERE k='schema_version'")
    if cur.fetchone() is None:
        conn.execute(
            "INSERT INTO memory_meta(k, v) VALUES (?, ?)",
            ("schema_version", V1094_VERSION),
        )
        conn.execute(
            "INSERT INTO memory_meta(k, v) VALUES (?, ?)",
            ("v1094_initialized_ts", str(time.time())),
        )
    else:
        # 既有 schema_version, 不覆盖 (避免破坏下游契约)
        pass
    conn.commit()


def downgrade(conn: sqlite3.Connection, *, keep_meta: bool = True) -> None:
    """谨慎回退 — DROP 所有 V1094 表.

    Args:
        conn: 目标连接
        keep_meta: True=保留 memory_meta (默认; 记录回退事件); False=全清

    Warning: 现仓 episodes / notes / hqb_* 不在此列 — 不会破坏.
    """
    tables = [
        "memory_hot", "memory_cold", "memory_wal", "memory_dream",
        "memory_snapshots", "stm_messages", "mtm_themes", "ltm_facts",
    ]
    for t in tables:
        conn.execute(f"DROP TABLE IF EXISTS {t}")
    if not keep_meta:
        conn.execute("DROP TABLE IF EXISTS memory_meta")
    else:
        conn.execute(
            "INSERT OR REPLACE INTO memory_meta(k, v) VALUES (?, ?)",
            ("v1094_downgraded_ts", str(time.time())),
        )
    conn.commit()


def upgrade_path(path: str | Path) -> None:
    """路径快捷方式 — 创建 db + 升级."""
    p = Path(path)
    p.parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(str(p))
    try:
        conn.execute("PRAGMA journal_mode=WAL")
        conn.execute("PRAGMA synchronous=NORMAL")
        conn.execute("PRAGMA foreign_keys=ON")
        upgrade(conn)
    finally:
        conn.close()


def downgrade_path(path: str | Path, *, keep_meta: bool = True) -> None:
    """路径快捷方式 — 回退."""
    p = Path(path)
    conn = sqlite3.connect(str(p))
    try:
        downgrade(conn, keep_meta=keep_meta)
    finally:
        conn.close()


# ---------------------------------------------------------------------------
# 高层封装: MemorySchema — 借鉴 hqb/schema.py.HqbStore 模式
# ---------------------------------------------------------------------------


class MemorySchema:
    """V1094 Memory Schema 真生产 facade.

    Usage:
        store = MemorySchema(":memory:")           # 内存 db
        store = MemorySchema("apeireth.db")        # 文件 db
        # schema 自动初始化

    职责:
        - init / upgrade / downgrade
        - 验证 schema_version
        - 提供表名清单 (供文档 / 测试)
        - 提供 fingerprint / checksum 工具
    """

    TABLE_NAMES: tuple[str, ...] = (
        "memory_hot", "memory_cold", "memory_wal", "memory_dream",
        "memory_snapshots", "stm_messages", "mtm_themes", "ltm_facts",
        "memory_meta",
    )

    def __init__(self, path: str | Path):
        self._is_memory = str(path) == ":memory:"
        if not self._is_memory:
            p = Path(path)
            p.parent.mkdir(parents=True, exist_ok=True)
            self.path = p
        else:
            self.path = ":memory:"  # type: ignore[assignment]
        self._conn = sqlite3.connect(str(self.path))
        self._conn.execute("PRAGMA journal_mode=WAL")
        self._conn.execute("PRAGMA synchronous=NORMAL")
        self._conn.execute("PRAGMA foreign_keys=ON")
        self._conn.row_factory = sqlite3.Row
        self._init_schema()

    # ---------- schema mgmt ----------

    def _init_schema(self) -> None:
        upgrade(self._conn)

    def schema_version(self) -> str:
        cur = self._conn.execute("SELECT v FROM memory_meta WHERE k='schema_version'")
        row = cur.fetchone()
        return row[0] if row else "0.0.0"

    def meta_get(self, k: str, default: Optional[str] = None) -> Optional[str]:
        cur = self._conn.execute("SELECT v FROM memory_meta WHERE k=?", (k,))
        row = cur.fetchone()
        return row[0] if row else default

    def list_tables(self) -> List[str]:
        cur = self._conn.execute(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
        )
        return [r[0] for r in cur.fetchall()]

    def table_exists(self, name: str) -> bool:
        cur = self._conn.execute(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?", (name,)
        )
        return cur.fetchone() is not None

    def list_indexes(self) -> List[str]:
        cur = self._conn.execute(
            "SELECT name FROM sqlite_master WHERE type='index' AND name NOT LIKE 'sqlite_%' ORDER BY name"
        )
        return [r[0] for r in cur.fetchall()]

    def downgrade(self, *, keep_meta: bool = True) -> None:
        """就地回退 — 调用后需重新 init 才能使用."""
        downgrade(self._conn, keep_meta=keep_meta)

    # ---------- WAL 写入辅助 (供上层 R7-BE-02 Replay / Phase-1 Dream 调用) ----------

    def wal_append(
        self,
        scope: str,
        op: str,
        payload: Any,
        *,
        event_id: Optional[str] = None,
    ) -> str:
        """追加一条 WAL — 幂等 (同 event_id 重复调用返回相同 row)."""
        from json import dumps
        eid = event_id or uuid.uuid4().hex
        s = dumps(payload, ensure_ascii=False, separators=(",", ":")) if not isinstance(payload, str) else payload
        try:
            self._conn.execute(
                "INSERT INTO memory_wal(seq, scope, op, payload, event_id, checksum, applied, ts, applied_ts)"
                " VALUES (NULL, ?, ?, ?, ?, ?, 0, ?, NULL)",
                (scope, op, s, eid, _checksum(s), time.time()),
            )
        except sqlite3.IntegrityError:
            # 幂等 — 同 event_id 已存在, 不重复
            pass
        self._conn.commit()
        return eid

    def wal_mark_applied(self, event_id: str) -> int:
        """标记 WAL 已应用 (幂等)."""
        cur = self._conn.execute(
            "UPDATE memory_wal SET applied=1, applied_ts=? WHERE event_id=? AND applied=0",
            (time.time(), event_id),
        )
        self._conn.commit()
        return cur.rowcount

    def wal_pending(self, scope: Optional[str] = None, limit: int = 100) -> List[Dict[str, Any]]:
        """列出 pending WAL — 供 Replay worker 消费."""
        if scope:
            cur = self._conn.execute(
                "SELECT * FROM memory_wal WHERE applied=0 AND scope=? ORDER BY seq LIMIT ?",
                (scope, limit),
            )
        else:
            cur = self._conn.execute(
                "SELECT * FROM memory_wal WHERE applied=0 ORDER BY seq LIMIT ?",
                (limit,),
            )
        return [dict(r) for r in cur.fetchall()]

    # ---------- 通用 close ----------

    def close(self) -> None:
        self._conn.close()


# ---------------------------------------------------------------------------
# 对接点说明 (供 docs + 集成时引用, 不参与运行时)
# ---------------------------------------------------------------------------


INTEGRATION_POINTS: Dict[str, str] = {
    "v1072_eternal_identity": (
        "IdentityManifest.add(source='LTM'/'MTM'/'STM') → ltm_facts(category, "
        "importance>=8, fingerprint) / mtm_themes(topic_label) / "
        "stm_messages(session_id, role, content). identity_hash → "
        "linked_identity_hash (hot/cold)."
    ),
    "hqb_integration": (
        "HQBIntegration.record_v1074/v1082/v1083 → memory_snapshots(scope='hqb', "
        "seq=auto, snapshot_score=score, identity_hash=触发时 hash). "
        "schema_version 通过 memory_meta.k='v1094_compat_with_hqb_v0.1' 校验."
    ),
    "memory_replay_design": (
        "IDEMPOTENT_OPS = {tag_set, anchor_link, anchor_unlink, score_record, "
        "phase_emit, trace_record} → memory_wal.op 必 ∈ 该集合 (否则 wal_append "
        "运行时上层守门). StateID(scope, seq) ↔ memory_snapshots(scope, seq) "
        "UNIQUE. Event.event_id ↔ memory_wal.event_id UNIQUE (幂等键)."
    ),
    "r7_be_01_dream": (
        "DreamSubsystem.run_cycle → memory_dream(_dream 标记隐式 status='pending'"
        "/dream_state=CONSOLIDATING; 来源 = memory_hot.id 或 memory_cold.id). "
        "consumed_ts 写入 → status='consumed'. tombstone → status='tombstoned'."
    ),
}


__all__ = [
    "V1094_VERSION",
    "SCHEMA_V094",
    "INDEXES_V094",
    "MemorySchema",
    "upgrade",
    "downgrade",
    "upgrade_path",
    "downgrade_path",
    "_checksum",
    "_fingerprint",
    "INTEGRATION_POINTS",
]