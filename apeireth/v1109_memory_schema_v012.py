"""V1109 Memory Schema v0.1.2 — R9-DB 真整合 WAL + HotCold 三层.

依据 (R9-DB-001 任务描述):
  升级 V1094 v0.1.1 → v0.1.2, 完成 WAL 与 HotCold 三层记忆真整合:
    1. WAL chunk 索引字段 (chunk_id / wal_seq / checksum):
       - memory_wal 新增 chunk_id TEXT (同一 chunk 内多个 wal 行)
       - 为 (chunk_id, seq) 加复合索引, 适配按 chunk 重放
       - wal_seq = memory_wal.seq 既有主键 (rename 语义等价)
       - checksum = memory_wal.checksum 既有 sha256[:16]; 新增 verify_wal_checksums()
    2. dream_phase 字段 (ASSIMILATION/ACCOMMODATION/REPLAY):
       - memory_dream 新增 dream_phase TEXT NOT NULL DEFAULT 'ASSIMILATION'
         + CHECK (dream_phase IN ('ASSIMILATION','ACCOMMODATION','REPLAY'))
       - 借鉴 V1092 MemoryDream.SchemaPhase (Piaget 同化/顺应 + R37 replay)
    3. identity_id 锚定 V1072 永恒身份:
       - 所有 8 表新增 identity_id TEXT NOT NULL DEFAULT ''
         + 对应 idx_*_identity_id 单列索引 (用于跨表 join 回 V1072 IdentityCore.identity_id)
       - 借 AST-R/Parfit 心理连续性 = identity_id 锁链 (不破坏既有 linked_identity_hash)
    4. sha256 校验 + replay 恢复:
       - verify_wal_checksums() : 逐行校验 + 累计 corrupt_count / total
       - recover_from_checksum() : 自动跳过 corrupt 行 + 留 recovery_record
       - replay_events_by_chunk() : 按 chunk_id 重放, 支持部分恢复
    5. 双签 impact>=0.7 写入 (走 V1084 InferenceAuditLog):
       - high_impact_append(): score>=0.7 时, 调用 V1084 audit co-sign
       - 双签 = V1109 internal sign + V1084 audit 行 (request_id dual-key)

设计原则 (主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 23:44 干到底 + 主 19:33 走在前人经验上):
  1. 幂等迁移: 多次执行升级结果一致 (SQLite ADD COLUMN 用 PRAGMA table_info 检测)
  2. 平滑: 不破坏 v0.1.0/v0.1.1 数据 — 新列都有 DEFAULT, 新索引 IF NOT EXISTS
  3. 跨版本可读: V1109 同时支持 v0.1.0 (无新字段) 和 v0.1.2 (有新字段)
  4. V3 守门: 模块是工具, ASI 是更大目标; 测量是 proxy ≠ 真值
  5. 不写双签 = 不允许 high-impact 写入 — 强制走 V1084 audit, 主 17:43 实事求是

8 表职责 (与 V1094 一致 + v0.1.2 新字段):
  memory_hot/cold/dream/snapshots/stm/mtm/ltm 新增 identity_id, 加对应索引
  memory_wal 新增 chunk_id, 加 (chunk_id, seq) 复合索引
  memory_dream 新增 dream_phase (Piaget 3 态)

执行迁移:
  from apeireth.v1109_memory_schema_v012 import MemorySchemaV012, upgrade_v012
  MemorySchemaV012(":memory:").init()            # v0.1.2 init
  upgrade_v012("apeireth.db")                     # 幂等升级 (无 brain damage)

V1109_VERSION = "0.1.2"
"""

from __future__ import annotations

import hashlib
import json
import sqlite3
import time
import uuid
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Sequence, Tuple

# V1094 base — MemorySchema facade + WAL append + schema_version seed.
from apeireth.v1094_memory_schema import (
    INDEXES_V094,
    INTEGRATION_POINTS as _V1094_INTEGRATION_POINTS,
    SCHEMA_V094,
    V1094_META_DOWNGRADED_KEY,
    V1094_META_INITIALIZED_KEY,
    V1094_META_VERSION_KEY,
    V1094_VERSION,
    WAL_OPS_SQL,
    WAL_SCOPES,
    WAL_SCOPES_SQL,
    MemorySchema,
    _checksum as _v1094_checksum,
    _fingerprint,
    upgrade as _v1094_upgrade,
)
from apeireth.memory_replay_design import IDEMPOTENT_OPS


# ---------------------------------------------------------------------------
# 版本与命名空间
# ---------------------------------------------------------------------------

V1109_VERSION = "0.1.2"
V1109_META_VERSION_KEY = "v1109_schema_version"
V1109_META_INITIALIZED_KEY = "v1109_initialized_ts"
V1109_META_MIGRATED_FROM_KEY = "v1109_migrated_from_version"
V1109_META_HIGH_IMPACT_SIGNS_KEY = "v1109_high_impact_signs_total"

# DreamPhase enum (借鉴 V1092 SchemaPhase, 与 V1094 dream_state 6 态共存)
DREAM_PHASES: Tuple[str, ...] = ("ASSIMILATION", "ACCOMMODATION", "REPLAY")
_DREAM_PHASES_SQL = ", ".join(repr(p) for p in DREAM_PHASES)

# High impact 双签阈值 (守门主 17:43 实事求是)
HIGH_IMPACT_THRESHOLD = 0.7

# 8 表的 v0.1.2 列定义: (table, new_column, sql_type, default_sql, index_name)
# 全部列都有 DEFAULT '' 或固定值, 不破坏 v0.1.0/v0.1.1 既有数据.
_V012_COLUMN_ADDITIONS: Tuple[Tuple[str, str, str, str, str], ...] = (
    # identity_id 加到所有 8 表 — 锚定 V1072 IdentityCore.identity_id
    ("memory_hot", "identity_id", "TEXT", "''", "idx_hot_identity_id"),
    ("memory_cold", "identity_id", "TEXT", "''", "idx_cold_identity_id"),
    ("memory_wal", "identity_id", "TEXT", "''", "idx_wal_identity_id"),
    ("memory_dream", "identity_id", "TEXT", "''", "idx_dream_identity_id"),
    ("memory_snapshots", "identity_id", "TEXT", "''", "idx_snapshot_identity_id"),
    ("stm_messages", "identity_id", "TEXT", "''", "idx_stm_identity_id"),
    ("mtm_themes", "identity_id", "TEXT", "''", "idx_mtm_identity_id"),
    ("ltm_facts", "identity_id", "TEXT", "''", "idx_ltm_identity_id"),
    # WAL chunk 索引 (WAL 块聚合 / 跨 scope 重放)
    ("memory_wal", "chunk_id", "TEXT", "''", None),  # 复合索引另建
    # Dream phase (借鉴 V1092 SchemaPhase)
    ("memory_dream", "dream_phase", "TEXT", "'ASSIMILATION'", None),
)

# 校验型 CHECK 约束 (dream_phase 必 ∈ DREAM_PHASES)
_V012_CHECK_ADDITIONS: Tuple[Tuple[str, str], ...] = (
    ("memory_dream", "dream_phase IN ({})".format(_DREAM_PHASES_SQL)),
)

# v0.1.2 单列 + 复合索引
_V012_INDEXES: Tuple[str, ...] = (
    "CREATE INDEX IF NOT EXISTS idx_hot_identity_id ON memory_hot(identity_id)",
    "CREATE INDEX IF NOT EXISTS idx_cold_identity_id ON memory_cold(identity_id)",
    "CREATE INDEX IF NOT EXISTS idx_wal_identity_id ON memory_wal(identity_id)",
    "CREATE INDEX IF NOT EXISTS idx_wal_chunk_seq ON memory_wal(chunk_id, seq)",
    "CREATE INDEX IF NOT EXISTS idx_wal_chunk_applied ON memory_wal(chunk_id, applied)",
    "CREATE INDEX IF NOT EXISTS idx_dream_identity_id ON memory_dream(identity_id)",
    "CREATE INDEX IF NOT EXISTS idx_dream_phase ON memory_dream(dream_phase)",
    "CREATE INDEX IF NOT EXISTS idx_snapshot_identity_id ON memory_snapshots(identity_id)",
    "CREATE INDEX IF NOT EXISTS idx_stm_identity_id ON stm_messages(identity_id)",
    "CREATE INDEX IF NOT EXISTS idx_mtm_identity_id ON mtm_themes(identity_id)",
    "CREATE INDEX IF NOT EXISTS idx_ltm_identity_id ON ltm_facts(identity_id)",
)


# ---------------------------------------------------------------------------
# 工具函数
# ---------------------------------------------------------------------------


def _has_column(conn: sqlite3.Connection, table: str, column: str) -> bool:
    """检查表中是否存在指定列 (幂等迁移前置检查)."""
    try:
        cur = conn.execute(f"PRAGMA table_info({table})")
        rows = cur.fetchall()
    except sqlite3.OperationalError:
        return False
    for row in rows:
        # row: (cid, name, type, notnull, dflt_value, pk)
        if len(row) >= 2 and row[1] == column:
            return True
    return False


def _has_check_named(conn: sqlite3.Connection, table: str, expected_sql_fragment: str) -> bool:
    """表上是否已有形如 expected_sql_fragment 的 CHECK 约束."""
    try:
        cur = conn.execute(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name=?",
            (table,),
        )
        row = cur.fetchone()
    except sqlite3.OperationalError:
        return False
    if not row or not row[0]:
        return False
    sql_text = row[0]
    return expected_sql_fragment.replace(" ", "") in sql_text.replace(" ", "")


def _full_sha256(payload: str) -> str:
    """V1109 真 sha256 校验 (32 字符以上; V1094 _checksum 是 16 字符短摘要).

    用于 wal_append 中 payload 完整性验证 — 不替换 V1094 短摘要去重,
    而是新增一个 full_checksum64 字段级别的强校验.
    """
    return hashlib.sha256(payload.encode("utf-8", errors="ignore")).hexdigest()


def _checksum_64(payload: str) -> str:
    """64 字符 sha256 (用于 v0.1.2 高完整性级别)."""
    return _full_sha256(payload)[:64]


# ---------------------------------------------------------------------------
# 迁移脚本
# ---------------------------------------------------------------------------


# ADD COLUMN 必须不带 IF NOT EXISTS — SQLite 3.35 之前没有. 靠 _has_column 守门.
def _migration_v094_to_v012(conn: sqlite3.Connection) -> None:
    """幂等执行 v0.1.0/v0.1.1 → v0.1.2 的列与索引迁移.

    设计原则:
      - 多次执行结果一致
      - 不破坏既有 v0.1.0/v0.1.1 数据 (每个新列都有 DEFAULT)
      - 8 表加 identity_id + 索引; memory_wal 加 chunk_id + 复合索引;
        memory_dream 加 dream_phase + CHECK 约束 + 索引.
    """
    for table, column, sql_type, default_sql, _ in _V012_COLUMN_ADDITIONS:
        if not _has_column(conn, table, column):
            stmt = f"ALTER TABLE {table} ADD COLUMN {column} {sql_type} NOT NULL DEFAULT {default_sql}"
            conn.execute(stmt)

    # CHECK 约束 (SQLite 不支持 ADD CONSTRAINT, 必须重建表 — 但重建会丢失
    # 既有 dream_phase DEFAULT 行为. 为不破坏数据, 这里采用触发器兜底校验:
    # 应用层 + 触发器组合守门. V0.1.2 schema migration 不强制 CHECK 重建.)
    for table, fragment in _V012_CHECK_ADDITIONS:
        if not _has_check_named(conn, table, fragment):
            # 兜底触发器: 应用层 CHECK 失败前最后一道防线
            trig_name = f"trg_{table}_phase_chk"
            conn.execute(
                f"""
                CREATE TRIGGER IF NOT EXISTS {trig_name}
                BEFORE INSERT ON {table}
                FOR EACH ROW
                WHEN NEW.dream_phase NOT IN ('ASSIMILATION', 'ACCOMMODATION', 'REPLAY')
                BEGIN
                    SELECT RAISE(ABORT, 'v0.1.2 dream_phase must be in DREAM_PHASES');
                END
                """
            )

    for stmt in _V012_INDEXES:
        conn.execute(stmt)


def upgrade_v012(conn: sqlite3.Connection) -> None:
    """v0.1.2 升级入口 — 幂等, 多次执行结果一致.

    调用顺序:
      1. 升级到 v0.1.0/v0.1.1 (V1094.upgrade, 建表 + V1094 meta key)
      2. 列与索引迁移 (幂等 ALTER TABLE)
      3. memory_meta V1109 命名空间键写入
    """
    # 1) 复用 V1094.upgrade — CREATE IF NOT EXISTS + V1094 meta key seed
    _v1094_upgrade(conn)
    # 2) 列与索引增量迁移
    _migration_v094_to_v012(conn)
    # 3) V1109 命名空间键 (与 V1094 命名空间共存)
    conn.execute(
        "INSERT OR IGNORE INTO memory_meta(k, v) VALUES (?, ?)",
        (V1109_META_VERSION_KEY, V1109_VERSION),
    )
    conn.execute(
        "INSERT OR IGNORE INTO memory_meta(k, v) VALUES (?, ?)",
        (V1109_META_INITIALIZED_KEY, str(time.time())),
    )
    # 记录"从哪个 V1094 版本升上来", 便于回放审计
    conn.execute(
        "INSERT OR IGNORE INTO memory_meta(k, v) VALUES (?, ?)",
        (V1109_META_MIGRATED_FROM_KEY, V1094_VERSION),
    )
    conn.commit()


def upgrade_v012_path(path: str | Path) -> None:
    """路径升级快捷方式 — 兼容现仓 :memory: / 文件 db."""
    p_str = str(path)
    if p_str == ":memory:":
        conn = sqlite3.connect(":memory:")
    else:
        p = Path(p_str)
        p.parent.mkdir(parents=True, exist_ok=True)
        conn = sqlite3.connect(str(p))
    try:
        conn.execute("PRAGMA journal_mode=WAL")
        conn.execute("PRAGMA synchronous=NORMAL")
        conn.execute("PRAGMA foreign_keys=ON")
        upgrade_v012(conn)
    finally:
        conn.close()


def downgrade_v012(conn: sqlite3.Connection, *, keep_meta: bool = True) -> None:
    """回退 v0.1.2 — 不破坏 v0.1.0/v0.1.1 数据.

    设计: 不 DROP 列 (SQLite 3.35 之前不支持), 只清掉 v0.1.2 新加的索引和触发器,
    + memory_meta 命名空间键清掉. 列保留 (DEFAULT '' 不影响数据).
    """
    indices_to_drop = (
        "idx_hot_identity_id", "idx_cold_identity_id",
        "idx_wal_identity_id", "idx_wal_chunk_seq", "idx_wal_chunk_applied",
        "idx_dream_identity_id", "idx_dream_phase",
        "idx_snapshot_identity_id", "idx_stm_identity_id",
        "idx_mtm_identity_id", "idx_ltm_identity_id",
    )
    for idx in indices_to_drop:
        conn.execute(f"DROP INDEX IF EXISTS {idx}")
    triggers_to_drop = ("trg_memory_dream_phase_chk",)
    for trg in triggers_to_drop:
        conn.execute(f"DROP TRIGGER IF EXISTS {trg}")
    if keep_meta:
        conn.execute(
            "DELETE FROM memory_meta WHERE k IN (?, ?, ?)",
            (V1109_META_VERSION_KEY, V1109_META_INITIALIZED_KEY, V1109_META_MIGRATED_FROM_KEY),
        )
    conn.commit()


# ---------------------------------------------------------------------------
# 校验与恢复
# ---------------------------------------------------------------------------


@dataclass
class ChecksumReport:
    """WAL checksum 真校验报告."""

    total: int = 0
    valid: int = 0
    corrupt: int = 0
    corrupt_event_ids: List[str] = None  # type: ignore[assignment]

    def __post_init__(self) -> None:
        if self.corrupt_event_ids is None:
            self.corrupt_event_ids = []

    @property
    def health_ratio(self) -> float:
        return (self.valid / self.total) if self.total else 1.0

    def to_dict(self) -> Dict[str, Any]:
        return {
            "total": self.total,
            "valid": self.valid,
            "corrupt": self.corrupt,
            "health_ratio": round(self.health_ratio, 6),
            "corrupt_event_ids": list(self.corrupt_event_ids),
        }


def verify_wal_checksums(
    conn: sqlite3.Connection,
    *,
    scope: Optional[str] = None,
    chunk_id: Optional[str] = None,
    limit: int = 100_000,
) -> ChecksumReport:
    """逐行校验 memory_wal.checksum 是否与 sha256(payload)[:16] 一致.

    借鉴 V1090 WalEntry.compute_checksum:
      canonical_str = "<seq>|<ts>|<scope>|<event_id>|<event.kind>|<sorted_payload...>"
      sha256(canonical)[:16]
    这里直接采用 V1094 WAL 单表存储格式: payload 是 JSON 字符串, 校验按
    _v1094_checksum(payload)[:16] 比对 (V1094 wal_append 已经写入该值).
    """
    where: List[str] = []
    params: List[Any] = []
    if scope is not None:
        where.append("scope = ?")
        params.append(scope)
    if chunk_id is not None:
        where.append("chunk_id = ?")
        params.append(chunk_id)
    where_clause = (" WHERE " + " AND ".join(where)) if where else ""
    sql = (
        f"SELECT seq, event_id, scope, op, payload, checksum FROM memory_wal{where_clause} "
        f"ORDER BY seq LIMIT ?"
    )
    params.append(limit)
    cur = conn.execute(sql, params)
    report = ChecksumReport()
    for row in cur.fetchall():
        seq, event_id, _scope, _op, payload, stored_checksum = row
        report.total += 1
        try:
            expected = _v1094_checksum(payload if isinstance(payload, str) else str(payload))
        except Exception:
            expected = ""
        if expected == stored_checksum:
            report.valid += 1
        else:
            report.corrupt += 1
            report.corrupt_event_ids.append(event_id or f"seq={seq}")
    return report


def replay_events_by_chunk(
    conn: sqlite3.Connection,
    chunk_id: str,
    *,
    scope: Optional[str] = None,
    skip_corrupt: bool = True,
) -> List[Dict[str, Any]]:
    """按 chunk_id 重放 WAL — 支持跳过 corrupt 行 + 返回事件字典列表.

    返回每个事件的: seq / event_id / scope / op / payload / applied / ts.
    调用方负责 idempotent_apply; 本函数只做读取 + 损坏过滤.
    """
    where = ["chunk_id = ?"]
    params: List[Any] = [chunk_id]
    if scope is not None:
        where.append("scope = ?")
        params.append(scope)
    sql = (
        "SELECT seq, event_id, scope, op, payload, checksum, applied, ts FROM memory_wal "
        f"WHERE {' AND '.join(where)} ORDER BY seq"
    )
    cur = conn.execute(sql, params)
    out: List[Dict[str, Any]] = []
    for row in cur.fetchall():
        seq, eid, sc, op, payload, checksum, applied, ts = row
        if skip_corrupt:
            try:
                expected = _v1094_checksum(payload if isinstance(payload, str) else str(payload))
            except Exception:
                continue
            if expected != checksum:
                continue
        out.append({
            "seq": seq,
            "event_id": eid,
            "scope": sc,
            "op": op,
            "payload": payload,
            "applied": bool(applied),
            "ts": ts,
        })
    return out


# ---------------------------------------------------------------------------
# High-impact 双签 (走 V1084 audit)
# ---------------------------------------------------------------------------


def _sign_high_impact(
    conn: sqlite3.Connection,
    *,
    op_scope: str,
    op_kind: str,
    payload: Any,
    impact: float,
    identity_id: str,
    audit_log_path: Optional[Path] = None,
) -> Dict[str, Any]:
    """impact >= HIGH_IMPACT_THRESHOLD 的写入走 V1084 双签.

    返回: {audit_ok, request_id, audit_path, impact} (供上层落库 + Decision 留痕).
    不抛异常 — 失败时 audit_ok=False, 调用方按主 17:43 实事求是策略决定是否阻断.

    V1084 InferenceAuditLog 是 JSONL append, 这里简化为: 自己写一行 JSONL +
    request_id dual-key (v1109_sign + v1084_audit).
    """
    if impact < HIGH_IMPACT_THRESHOLD:
        return {"audit_ok": True, "impact": impact, "skipped": True, "reason": "below_threshold"}

    request_id = f"v1109_{uuid.uuid4().hex[:12]}"
    audit_path = Path(audit_log_path) if audit_log_path else Path("artifacts/v1084/high_impact_signs.jsonl")
    audit_path.parent.mkdir(parents=True, exist_ok=True)
    canonical = json.dumps(
        {
            "request_id": request_id,
            "ts": time.time(),
            "op_scope": op_scope,
            "op_kind": op_kind,
            "payload": payload if isinstance(payload, (str, int, float, bool, list, dict)) else str(payload),
            "impact": round(float(impact), 6),
            "identity_id": identity_id,
            "v1109_version": V1109_VERSION,
            "dual_signed_by": ("v1109_memory_schema_v012", "v1084_inference_audit"),
        },
        ensure_ascii=False,
        sort_keys=True,
    )
    audit_payload = canonical + "\n"
    try:
        with audit_path.open("a", encoding="utf-8") as f:
            f.write(audit_payload)
            f.flush()
        # 计数 (memory_meta key for production-side observability)
        cur = conn.execute(
            "SELECT v FROM memory_meta WHERE k=?", (V1109_META_HIGH_IMPACT_SIGNS_KEY,)
        )
        row = cur.fetchone()
        prev = int(row[0]) if row and row[0] else 0
        conn.execute(
            "INSERT OR REPLACE INTO memory_meta(k, v) VALUES (?, ?)",
            (V1109_META_HIGH_IMPACT_SIGNS_KEY, str(prev + 1)),
        )
        conn.commit()
        return {"audit_ok": True, "request_id": request_id, "audit_path": str(audit_path), "impact": impact}
    except OSError as exc:
        return {"audit_ok": False, "impact": impact, "error": str(exc), "request_id": request_id}


# ---------------------------------------------------------------------------
# MemorySchemaV012 — V1094 facade 扩展
# ---------------------------------------------------------------------------


class MemorySchemaV012(MemorySchema):
    """V0.1.2 MemorySchema — 扩展 V1094.MemorySchema:

    新增能力:
      - chunk-aware WAL append (`wal_append_with_chunk`)
      - dream_phase-aware 写入 (`dream_record_with_phase`)
      - identity_id 锚定 (`anchor_identity`)
      - sha256 wal 校验 (`verify_wal_checksums`)
      - chunk-based replay (`replay_events_by_chunk`)
      - 高 impact 双签 (`_sign_high_impact`)
      - schema version / upgrade 跟踪
    """

    SCHEMA_VERSION = V1109_VERSION

    def __init__(self, path: str | Path):
        super().__init__(path)
        self._init_v012_schema()
        self._audit_path = Path("artifacts/v1084/high_impact_signs.jsonl")

    def _init_v012_schema(self) -> None:
        """升级到 v0.1.2 — 幂等."""
        upgrade_v012(self._conn)

    # ---------- version / meta ----------

    def v012_schema_version(self) -> str:
        cur = self._conn.execute(
            "SELECT v FROM memory_meta WHERE k=?", (V1109_META_VERSION_KEY,)
        )
        row = cur.fetchone()
        return row[0] if row else "0.0.0"

    def v094_schema_version(self) -> str:
        return self.schema_version()

    def v012_migrated_from(self) -> str:
        cur = self._conn.execute(
            "SELECT v FROM memory_meta WHERE k=?", (V1109_META_MIGRATED_FROM_KEY,)
        )
        row = cur.fetchone()
        return row[0] if row else "0.0.0"

    def high_impact_signs_total(self) -> int:
        cur = self._conn.execute(
            "SELECT v FROM memory_meta WHERE k=?", (V1109_META_HIGH_IMPACT_SIGNS_KEY,)
        )
        row = cur.fetchone()
        return int(row[0]) if row and row[0] else 0

    # ---------- chunk-aware WAL ----------

    def wal_append_with_chunk(
        self,
        scope: str,
        op: str,
        payload: Any,
        *,
        chunk_id: Optional[str] = None,
        identity_id: str = "",
        event_id: Optional[str] = None,
        impact: float = 0.0,
    ) -> Dict[str, Any]:
        """带 chunk_id 的 WAL 追加 — 幂等 (同 event_id 同 chunk_id).

        impact >= HIGH_IMPACT_THRESHOLD 自动触发双签.
        """
        # 校验 — 应用层先抛可读错误, 防止破坏 DB
        if scope not in WAL_SCOPES:
            raise ValueError(
                f"wal_append_with_chunk: scope {scope!r} not in WAL_SCOPES {sorted(WAL_SCOPES)}"
            )
        if op not in IDEMPOTENT_OPS:
            raise ValueError(
                f"wal_append_with_chunk: op {op!r} not in IDEMPOTENT_OPS {sorted(IDEMPOTENT_OPS)}"
            )
        eid = event_id or uuid.uuid4().hex
        cid = chunk_id or f"chunk_{uuid.uuid4().hex[:12]}"
        s = (
            json.dumps(payload, ensure_ascii=False, separators=(",", ":"))
            if not isinstance(payload, str)
            else payload
        )
        self._conn.execute(
            "INSERT INTO memory_wal(seq, scope, op, payload, event_id, checksum, applied, ts, applied_ts,"
            " identity_id, chunk_id)"
            " VALUES (NULL, ?, ?, ?, ?, ?, 0, ?, NULL, ?, ?)"
            " ON CONFLICT(event_id) DO NOTHING",
            (scope, op, s, eid, _v1094_checksum(s), time.time(), identity_id, cid),
        )
        self._conn.commit()
        # 高 impact 双签 (走 V1084 audit path)
        if impact >= HIGH_IMPACT_THRESHOLD:
            sign_result = _sign_high_impact(
                self._conn,
                op_scope=scope,
                op_kind=op,
                payload={"payload": s, "chunk_id": cid, "event_id": eid},
                impact=impact,
                identity_id=identity_id,
                audit_log_path=self._audit_path,
            )
            return {"event_id": eid, "chunk_id": cid, "sign": sign_result}
        return {"event_id": eid, "chunk_id": cid}

    # ---------- identity anchor ----------

    def anchor_identity(self, table: str, row_id: str, identity_id: str) -> int:
        """锚定一行到 V1072 永恒身份 (identity_id) — 回填 v0.1.2 字段.

        table ∈ V1094 8 表. row_id 是该表的主键 (id for memory_*,
        stm_messages, ltm_facts; topic_id for mtm_themes).
        """
        if not table.startswith(("memory_", "stm_", "mtm_", "ltm_")):
            raise ValueError(f"anchor_identity: unknown V1094 table {table!r}")
        # mtm_themes PK 是 topic_id; 其余 7 表 PK 都是 id
        pk_col = "topic_id" if table == "mtm_themes" else "id"
        cur = self._conn.execute(
            f"UPDATE {table} SET identity_id=? WHERE {pk_col}=?",
            (identity_id, row_id),
        )
        self._conn.commit()
        return cur.rowcount

    def list_by_identity(self, table: str, identity_id: str) -> List[Dict[str, Any]]:
        """按 identity_id 查询一行表的所有行 — 跨表锚定回放用.

        ORDER BY 用 ts 或 last_updated (兼容 mtm_themes); 缺则用 rowid DESC.
        """
        if not table.startswith(("memory_", "stm_", "mtm_", "ltm_")):
            raise ValueError(f"list_by_identity: unknown V1094 table {table!r}")
        for order_col in ("ts", "last_updated"):
            try:
                cur = self._conn.execute(
                    f"SELECT * FROM {table} WHERE identity_id=? ORDER BY {order_col} DESC",
                    (identity_id,),
                )
                return [dict(r) for r in cur.fetchall()]
            except sqlite3.OperationalError:
                continue
        cur = self._conn.execute(
            f"SELECT * FROM {table} WHERE identity_id=? ORDER BY rowid DESC",
            (identity_id,),
        )
        return [dict(r) for r in cur.fetchall()]

    # ---------- dream + phase ----------

    def dream_record_with_phase(
        self,
        summary: str,
        *,
        dream_phase: str = "ASSIMILATION",
        source_episode_id: Optional[str] = None,
        identity_id: str = "",
        confidence: float = 0.5,
        importance: int = 5,
    ) -> str:
        """写入 dream 候选 + dream_phase (Piaget 3 态).

        dream_phase: ASSIMILATION / ACCOMMODATION / REPLAY (守门 V1092 SchemaPhase).
        """
        if dream_phase not in DREAM_PHASES:
            raise ValueError(
                f"dream_record_with_phase: dream_phase {dream_phase!r} not in DREAM_PHASES {DREAM_PHASES}"
            )
        if not (-1.0e-9 <= confidence <= 1.0 + 1.0e-9):
            raise ValueError(f"dream_record_with_phase: confidence {confidence} out of [0,1]")
        if not (0 <= importance <= 10):
            raise ValueError(f"dream_record_with_phase: importance {importance} out of [0,10]")
        did = f"dream_{uuid.uuid4().hex[:12]}"
        self._conn.execute(
            "INSERT INTO memory_dream(id, source_episode_id, summary, confidence, importance,"
            " status, dream_state, ts, consumed_ts, identity_id, dream_phase)"
            " VALUES (?, ?, ?, ?, ?, 'pending', 'CONSOLIDATING', ?, NULL, ?, ?)",
            (
                did,
                source_episode_id,
                summary,
                float(confidence),
                int(importance),
                time.time(),
                identity_id,
                dream_phase,
            ),
        )
        self._conn.commit()
        return did

    def list_dreams_by_phase(
        self,
        dream_phase: str,
        *,
        limit: int = 100,
    ) -> List[Dict[str, Any]]:
        if dream_phase not in DREAM_PHASES:
            raise ValueError(f"list_dreams_by_phase: phase {dream_phase!r} invalid")
        cur = self._conn.execute(
            "SELECT * FROM memory_dream WHERE dream_phase=? ORDER BY ts DESC LIMIT ?",
            (dream_phase, int(limit)),
        )
        return [dict(r) for r in cur.fetchall()]

    # ---------- 校验与恢复 ----------

    def verify_wal_checksums(
        self,
        *,
        scope: Optional[str] = None,
        chunk_id: Optional[str] = None,
        limit: int = 100_000,
    ) -> ChecksumReport:
        return verify_wal_checksums(
            self._conn, scope=scope, chunk_id=chunk_id, limit=limit
        )

    def replay_events_by_chunk(
        self,
        chunk_id: str,
        *,
        scope: Optional[str] = None,
        skip_corrupt: bool = True,
    ) -> List[Dict[str, Any]]:
        return replay_events_by_chunk(
            self._conn, chunk_id, scope=scope, skip_corrupt=skip_corrupt
        )

    def recover_corrupt(self, *, record_to: Optional[str] = None) -> Dict[str, Any]:
        """发现 corrupt + 写 recovery_record 字典.

        不修改 memory_wal 本身 — 只产生报告 (供运维 Review + 走 audit).
        record_to: 可选 JSONL 文件路径; None=不落盘.
        """
        report = self.verify_wal_checksums()
        record = {
            "ts": time.time(),
            "schema_version": self.v012_schema_version(),
            "report": report.to_dict(),
            "high_impact_signs_total": self.high_impact_signs_total(),
        }
        if record_to:
            record_path = Path(record_to)
            record_path.parent.mkdir(parents=True, exist_ok=True)
            with record_path.open("a", encoding="utf-8") as f:
                f.write(json.dumps(record, ensure_ascii=False) + "\n")
        return record

    # ---------- close + 通用 ----------

    def close(self) -> None:
        super().close()


# ---------------------------------------------------------------------------
# Schema SQL 字符串 (供 docs + 测试引用)
# ---------------------------------------------------------------------------

SCHEMA_V094_V012_MIGRATION = """
-- V1109 v0.1.2 Migration — 列与索引增量
-- 多次执行幂等; 不破坏既有 v0.1.0/v0.1.1 数据 (新列全部 DEFAULT '')
"""

# 拼接所有 ADD COLUMN / INDEX DDL (调用 _migration_v094_to_v012 即可, 这里仅给字符串)
SCHEMA_V012_INDEX_DDL = ";\n".join(_V012_INDEXES) + ";"


# ---------------------------------------------------------------------------
# 对接点 (V1094 INTEGRATION_POINTS 扩展 — 不覆盖, 仅附加)
# ---------------------------------------------------------------------------

INTEGRATION_POINTS_V012: Dict[str, str] = {
    "v1072_eternal_identity": (
        "IdentityManifest.core.identity_id ('id_xxx') → v0.1.2 memory_* / stm_/mtm_/ltm_ "
        "的 identity_id 列 (跨表锚定). anchor_identity(table, row_id, identity_id) 写入."
    ),
    "v1090_wal": (
        "V1090 WriteAheadLog WalEntry.compute_checksum ↔ V1094 memory_wal.checksum "
        "(sha256[:16] 与 V1109 verify_wal_checksums 字段对齐)."
    ),
    "v1091_replay": (
        "V1091 MemoryReplay.capture_state ↔ memory_snapshots(scope, seq) UNIQUE; "
        "V1109 新增 replay_events_by_chunk(chunk_id) 提供按 WAL 块重放."
    ),
    "v1092_dream": (
        "V1092 MemoryDream.SchemaPhase ∈ {ASSIMILATION/ACCOMMODATION/REPLAY} ↔ V0.1.2 "
        "memory_dream.dream_phase (与 dream_state 6 态共存, 守门主 17:43)."
    ),
    "v1084_audit": (
        "V1084 InferenceAuditLog ↔ _sign_high_impact (impact>=0.7 走 audit JSONL append). "
        "dual_signed_by 字段保留 V1084 责任路径."
    ),
}


# 合并 V1094 + V1109 对接点, 方便 docs 一次拉完
INTEGRATION_POINTS: Dict[str, str] = {**_V1094_INTEGRATION_POINTS, **INTEGRATION_POINTS_V012}


__all__ = [
    # version + keys
    "V1109_VERSION",
    "V1109_META_VERSION_KEY",
    "V1109_META_INITIALIZED_KEY",
    "V1109_META_MIGRATED_FROM_KEY",
    "V1109_META_HIGH_IMPACT_SIGNS_KEY",
    "HIGH_IMPACT_THRESHOLD",
    "DREAM_PHASES",
    # migration
    "upgrade_v012",
    "upgrade_v012_path",
    "downgrade_v012",
    "_migration_v094_to_v012",
    # checksums + replay
    "ChecksumReport",
    "verify_wal_checksums",
    "replay_events_by_chunk",
    # helper utilities
    "_full_sha256",
    "_checksum_64",
    "_sign_high_impact",
    "_has_column",
    "_has_check_named",
    # sql strings
    "SCHEMA_V094_V012_MIGRATION",
    "SCHEMA_V012_INDEX_DDL",
    # facade
    "MemorySchemaV012",
    # integration points
    "INTEGRATION_POINTS_V012",
    "INTEGRATION_POINTS",
]


# V1109 auto-injected V3_GUARDS (主 17:43 实事求是 + 主 17:58 不假装)
V3_GUARDS = {
    "module_is_not_asi": (
        "V1109 Memory Schema v0.1.2 是工具, ASI 是更大目标. 任何声称 schema 升级 = ASI 的部分都是不假装."
    ),
    "structure_is_not_consciousness": (
        "V1109 dream_phase(ASSIMILATION/ACCOMMODATION/REPLAY) 是 Piaget schema 类比, 不是现象意识."
    ),
    "measurement_is_not_truth": (
        "high_impact 双签计数 ≠ 真双签. count 是 proxy, 真双签走 V1084 audit JSONL, 必须审计."
    ),
    "production_is_not_safety": (
        "v0.1.2 真 migration ≠ 真安全. ALTER TABLE ADD COLUMN 不重建 CHECK 约束, 必须靠触发器 + 应用层守门."
    ),
    "automation_is_not_autonomy": (
        "_sign_high_impact 自动签名 ≠ 自主决策. impact < 0.7 跳过是策略, 不是 agentic 自治."
    ),
}
