"""V1094 Memory Schema tests — R8-TrackA3.

≥20 真测试覆盖:
  - schema 创建 (T01..T04): 9 表 + 索引 + meta + 版本
  - 索引效率 (T05..T08): EXPLAIN QUERY PLAN 命中索引
  - 真插入查询 (T09..T15): hot/cold/wal/dream/snapshot/stm/mtm/ltm
  - 迁移幂等 (T16..T18): upgrade 多次 + downgrade + 重建
  - 对接点 (T19..T20): fingerprint/checksum/wal_append 幂等
  - 约束完整性 (T21..T23): UNIQUE / FK / CHECK 行为

执行:
  pytest tests/test_v1094_memory_schema.py -v
"""
from __future__ import annotations

import sqlite3
import sys
import tempfile
import time
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth.v1094_memory_schema import (  # noqa: E402
    INDEXES_V094,
    INTEGRATION_POINTS,
    SCHEMA_V094,
    V1094_VERSION,
    MemorySchema,
    _checksum,
    _fingerprint,
    downgrade,
    downgrade_path,
    upgrade,
    upgrade_path,
)


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture
def mem_store() -> MemorySchema:
    """内存 db — 每个 test 干净."""
    s = MemorySchema(":memory:")
    yield s
    s.close()


@pytest.fixture
def file_store(tmp_path: Path) -> MemorySchema:
    """文件 db — 测 upgrade/downgrade 路径."""
    p = tmp_path / "v1094_test.db"
    s = MemorySchema(p)
    yield s
    s.close()


# ===========================================================================
# Group 1: schema 创建 (T01..T04) — 4 用例
# ===========================================================================


def test_t01_schema_version_seeded(mem_store: MemorySchema) -> None:
    """T01: 首次 init → V1094 命名空间键 memory_meta.v1094_schema_version=V1094_VERSION.

    旧 ``schema_version`` 键 (memory_store.py v0.3 拥有) 永不被 V1094 写入,
    以保证真生产兼容; 新 reader 通过 ``v1094_schema_version`` 读进度.
    """
    assert mem_store.schema_version() == V1094_VERSION
    # 命名空间键 (V1094 拥有)
    assert mem_store.meta_get("v1094_schema_version") == V1094_VERSION
    assert mem_store.meta_get("v1094_initialized_ts") is not None
    # 旧 key 不应被 V1094 写入 (新库情况下)
    assert mem_store.meta_get("schema_version") is None


def test_t02_all_tables_created(mem_store: MemorySchema) -> None:
    """T02: 9 表 (8 业务 + memory_meta) 全部 CREATE."""
    tables = set(mem_store.list_tables())
    expected = {
        "memory_hot", "memory_cold", "memory_wal", "memory_dream",
        "memory_snapshots", "stm_messages", "mtm_themes", "ltm_facts",
        "memory_meta",
    }
    assert expected.issubset(tables), f"missing: {expected - tables}"
    assert len(tables) == 9


def test_t03_all_indexes_created(mem_store: MemorySchema) -> None:
    """T03: 索引齐全 — ≥20 条 (8 表 + 集中索引 26 条)."""
    idx = mem_store.list_indexes()
    assert len(idx) >= 20, f"too few indexes: {len(idx)}"
    # 关键索引点名验证
    must_have = {
        "idx_hot_session_ts", "idx_cold_category_imp",  # 普通 INDEX
        "idx_snapshot_scope_seq", "idx_ltm_fingerprint",  # UNIQUE INDEX (sqlite_master 也登记)
    }
    assert must_have.issubset(set(idx)), f"missing: {must_have - set(idx)}"


def test_t04_pragmas_set(file_store: MemorySchema) -> None:
    """T04: PRAGMA journal_mode=WAL + foreign_keys=ON (file db; :memory: 走 memory 模式)."""
    cur = file_store._conn.execute("PRAGMA journal_mode")
    mode = cur.fetchone()[0].lower()
    # 文件 db → 'wal'; :memory: → 'memory' (SQLite 限制); 两者都合法, code path 一致
    assert mode in ("wal", "memory"), f"unexpected journal mode: {mode}"
    cur = file_store._conn.execute("PRAGMA foreign_keys")
    assert cur.fetchone()[0] == 1


# ===========================================================================
# Group 2: 索引效率 (T05..T08) — 4 用例
# ===========================================================================


def test_t05_hot_session_ts_index_hit(mem_store: MemorySchema) -> None:
    """T05: hot tier 按 session+ts 排序 — EXPLAIN QUERY PLAN 命中 idx_hot_session_ts."""
    # 灌 200 行制造真实数据
    for i in range(200):
        mem_store._conn.execute(
            "INSERT INTO memory_hot VALUES (?,?,?,?,?,?,?,?,?,?,?)",
            (f"h{i}", f"sess{i % 5}", "apeireth", f"msg{i}", "",
             "utterance", "hot", time.time() + i, _fingerprint(f"h{i}"),
             "idhash", None),
        )
    mem_store._conn.commit()
    plan = mem_store._conn.execute(
        "EXPLAIN QUERY PLAN SELECT * FROM memory_hot WHERE session_id=? ORDER BY ts DESC",
        ("sess1",),
    ).fetchall()
    plan_str = " ".join(str(p[3]) for p in plan)
    assert "idx_hot_session_ts" in plan_str, f"no index hit: {plan_str}"


def test_t06_ltm_category_importance_index(mem_store: MemorySchema) -> None:
    """T06: ltm_facts 按 category + importance DESC — 命中 idx_ltm_category_imp."""
    for i in range(150):
        mem_store._conn.execute(
            "INSERT INTO ltm_facts VALUES (?,?,?,?,?,?,?,?,?)",
            (f"l{i}", "identity" if i % 2 else "decision", f"fact{i}", 5 + (i % 6),
             0.5 + (i % 5) * 0.1, "", _fingerprint(f"l{i}", "identity"), time.time(), None),
        )
    mem_store._conn.commit()
    plan = mem_store._conn.execute(
        "EXPLAIN QUERY PLAN SELECT * FROM ltm_facts WHERE category=? ORDER BY importance DESC",
        ("identity",),
    ).fetchall()
    plan_str = " ".join(str(p[3]) for p in plan)
    assert "idx_ltm_category_imp" in plan_str


def test_t07_wal_pending_scan_uses_index(mem_store: MemorySchema) -> None:
    """T07: WAL pending 扫表 — 命中 idx_wal_applied_ts (或 idx_wal_scope_applied_ts)."""
    for i in range(100):
        mem_store.wal_append("hot", "tag_set", {"i": i})
    plan = mem_store._conn.execute(
        "EXPLAIN QUERY PLAN SELECT * FROM memory_wal WHERE applied=0 ORDER BY ts"
    ).fetchall()
    plan_str = " ".join(str(p[3]) for p in plan)
    assert "idx_wal" in plan_str, f"no wal index hit: {plan_str}"


def test_t08_snapshot_scope_seq_unique(mem_store: MemorySchema) -> None:
    """T08: memory_snapshots (scope, seq) UNIQUE — EXPLAIN 显示 USING COVERING INDEX."""
    for i in range(50):
        mem_store._conn.execute(
            "INSERT INTO memory_snapshots VALUES (?,?,?,?,?,?,?,?)",
            (f"sn{i}", "hqb", i, _fingerprint(f"sn{i}"), "r", "ih", 0.85, time.time()),
        )
    mem_store._conn.commit()
    plan = mem_store._conn.execute(
        "EXPLAIN QUERY PLAN SELECT * FROM memory_snapshots WHERE scope=? AND seq=?",
        ("hqb", 1),
    ).fetchall()
    plan_str = " ".join(str(p[3]) for p in plan)
    assert "idx_snapshot_scope_seq" in plan_str


# ===========================================================================
# Group 3: 真插入查询 (T09..T15) — 7 用例
# ===========================================================================


def test_t09_hot_insert_and_query(mem_store: MemorySchema) -> None:
    """T09: memory_hot 真插入 + 按 session_id 查询."""
    sid = "session_alpha"
    for i in range(10):
        mem_store._conn.execute(
            "INSERT INTO memory_hot VALUES (?,?,?,?,?,?,?,?,?,?,?)",
            (f"h{i}", sid, "master", f"hello-{i}", "ctx",
             "utterance", "hot", 1700000000.0 + i,
             _fingerprint(f"h{i}"), "idhash1", None),
        )
    mem_store._conn.commit()
    rows = mem_store._conn.execute(
        "SELECT id, content FROM memory_hot WHERE session_id=? ORDER BY ts DESC", (sid,)
    ).fetchall()
    assert len(rows) == 10
    assert rows[0]["content"] == "hello-9"


def test_t10_cold_with_superseded_chain(mem_store: MemorySchema) -> None:
    """T10: memory_cold — supersede 链 + Parfit 心理连续性."""
    for i in range(3):
        mem_store._conn.execute(
            "INSERT INTO memory_cold VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            (f"c{i}", "sess0", "apeireth", f"v{i}", "", "note", "cold",
             "decision", 9, 0.9, 1700000000.0 + i,
             _fingerprint(f"c{i}", "decision"), "ih", None,
             f"c{i+1}" if i < 2 else None),
        )
    mem_store._conn.commit()
    rows = mem_store._conn.execute(
        "SELECT id, superseded_by FROM memory_cold ORDER BY ts"
    ).fetchall()
    assert rows[0]["superseded_by"] == "c1"
    assert rows[1]["superseded_by"] == "c2"
    assert rows[2]["superseded_by"] is None


def test_t11_wal_append_and_apply(mem_store: MemorySchema) -> None:
    """T11: wal_append 真写入 + wal_mark_applied + 幂等."""
    eid = mem_store.wal_append("hot", "tag_set", {"tag": "phi", "k": 1})
    pending = mem_store.wal_pending(scope="hot")
    assert len(pending) == 1 and pending[0]["event_id"] == eid
    # mark applied
    n = mem_store.wal_mark_applied(eid)
    assert n == 1
    # 二次 mark 幂等
    n2 = mem_store.wal_mark_applied(eid)
    assert n2 == 0
    # 同 event_id 二次 append 幂等 (UNIQUE 冲突捕获)
    eid2 = mem_store.wal_append("hot", "tag_set", {"x": 1}, event_id=eid)
    assert eid2 == eid  # 返回相同 event_id


def test_t12_dream_pending_and_consumed(mem_store: MemorySchema) -> None:
    """T12: memory_dream — pending → consumed 流程."""
    for i in range(5):
        mem_store._conn.execute(
            "INSERT INTO memory_dream VALUES (?,?,?,?,?,?,?,?,?)",
            (f"d{i}", f"h{i}", f"summary{i}", 0.7, 6,
             "pending", "CONSOLIDATING", 1700000000.0 + i, None),
        )
    mem_store._conn.commit()
    cur = mem_store._conn.execute("SELECT COUNT(*) FROM memory_dream WHERE status='pending'")
    assert cur.fetchone()[0] == 5
    mem_store._conn.execute(
        "UPDATE memory_dream SET status='consumed', consumed_ts=? WHERE id='d0'",
        (time.time(),),
    )
    mem_store._conn.commit()
    cur = mem_store._conn.execute("SELECT COUNT(*) FROM memory_dream WHERE status='consumed'")
    assert cur.fetchone()[0] == 1


def test_t13_snapshot_uniqueness(mem_store: MemorySchema) -> None:
    """T13: memory_snapshots (scope, seq) UNIQUE — 重复 (scope, seq) 抛 IntegrityError."""
    mem_store._conn.execute(
        "INSERT INTO memory_snapshots VALUES (?,?,?,?,?,?,?,?)",
        ("sn1", "hqb", 1, "h1", "r", "ih", 0.85, time.time()),
    )
    mem_store._conn.commit()
    with pytest.raises(sqlite3.IntegrityError):
        mem_store._conn.execute(
            "INSERT INTO memory_snapshots VALUES (?,?,?,?,?,?,?,?)",
            ("sn2", "hqb", 1, "h2", "r", "ih", 0.86, time.time()),
        )


def test_t14_stm_messages_rolling_query(mem_store: MemorySchema) -> None:
    """T14: stm_messages 按 session 滚动 (模拟 STM_MAX_SIZE=50)."""
    for i in range(50):
        mem_store._conn.execute(
            "INSERT INTO stm_messages VALUES (?,?,?,?,?,?,?)",
            (f"sm{i}", "sess_x", "user" if i % 2 else "assistant",
             f"msg-{i}", None, 1700000000.0 + i, _fingerprint(f"sm{i}")),
        )
    mem_store._conn.commit()
    rows = mem_store._conn.execute(
        "SELECT id FROM stm_messages WHERE session_id=? ORDER BY ts DESC LIMIT 50", ("sess_x",)
    ).fetchall()
    assert len(rows) == 50
    assert rows[0]["id"] == "sm49"  # 最新在前


def test_t15_ltm_fingerprint_unique_dedup(mem_store: MemorySchema) -> None:
    """T15: ltm_facts.fingerprint UNIQUE — 重复 fingerprint 抛 IntegrityError (去重)."""
    fp = _fingerprint("phi", "0.1.0")
    mem_store._conn.execute(
        "INSERT INTO ltm_facts VALUES (?,?,?,?,?,?,?,?,?)",
        ("l1", "fact", "phi 0.1.0", 9, 0.9, "", fp, time.time(), None),
    )
    mem_store._conn.commit()
    with pytest.raises(sqlite3.IntegrityError):
        mem_store._conn.execute(
            "INSERT INTO ltm_facts VALUES (?,?,?,?,?,?,?,?,?)",
            ("l2", "fact", "phi 0.1.0 dup", 9, 0.9, "", fp, time.time(), None),
        )


# ===========================================================================
# Group 4: 迁移幂等 (T16..T18) — 3 用例
# ===========================================================================


def test_t16_upgrade_idempotent(mem_store: MemorySchema) -> None:
    """T16: upgrade() 多次执行 — 表数 + 索引数不变, schema_version 不变."""
    conn = mem_store._conn
    upgrade(conn)
    upgrade(conn)
    upgrade(conn)
    tables_before = set(mem_store.list_tables())
    idx_before = set(mem_store.list_indexes())
    sv_before = mem_store.schema_version()
    assert sv_before == V1094_VERSION
    assert len(tables_before) == 9
    # 重复 upgrade 不破坏
    upgrade(conn)
    assert set(mem_store.list_tables()) == tables_before
    assert set(mem_store.list_indexes()) == idx_before
    assert mem_store.schema_version() == sv_before


def test_t17_upgrade_then_downgrade_then_upgrade(tmp_path: Path) -> None:
    """T17: 真文件迁移 — upgrade → downgrade → upgrade 重建完整."""
    p = tmp_path / "v1094_cycle.db"
    # 1st upgrade
    upgrade_path(p)
    c1 = sqlite3.connect(str(p))
    tables1 = [r[0] for r in c1.execute("SELECT name FROM sqlite_master WHERE type='table'").fetchall()]
    assert "memory_hot" in tables1
    c1.close()
    # downgrade (保留 meta)
    downgrade_path(p, keep_meta=True)
    c2 = sqlite3.connect(str(p))
    tables2 = [r[0] for r in c2.execute("SELECT name FROM sqlite_master WHERE type='table'").fetchall()]
    assert "memory_hot" not in tables2
    assert "memory_meta" in tables2  # keep_meta=True
    c2.close()
    # 重新 upgrade 重建
    upgrade_path(p)
    c3 = sqlite3.connect(str(p))
    tables3 = [r[0] for r in c3.execute("SELECT name FROM sqlite_master WHERE type='table'").fetchall()]
    assert "memory_hot" in tables3
    assert "memory_meta" in tables3
    c3.close()


def test_t18_path_persistence(tmp_path: Path) -> None:
    """T18: 关闭 + 重开 — schema 与数据均存活 (真持久化)."""
    p = tmp_path / "v1094_persist.db"
    s1 = MemorySchema(p)
    s1._conn.execute(
        "INSERT INTO ltm_facts VALUES (?,?,?,?,?,?,?,?,?)",
        ("lx", "fact", "persistent fact", 9, 0.9, "",
         _fingerprint("lx"), time.time(), None),
    )
    s1._conn.commit()
    s1.close()
    # 重开
    s2 = MemorySchema(p)
    row = s2._conn.execute("SELECT content FROM ltm_facts WHERE id='lx'").fetchone()
    assert row is not None and row[0] == "persistent fact"
    assert s2.schema_version() == V1094_VERSION
    s2.close()


# ===========================================================================
# Group 5: 对接点 + 工具 (T19..T20) — 2 用例
# ===========================================================================


def test_t19_fingerprint_and_checksum_deterministic() -> None:
    """T19: _fingerprint / _checksum — 同输入同输出 (无随机)."""
    fp1 = _fingerprint("a", "b", "c")
    fp2 = _fingerprint("a", "b", "c")
    assert fp1 == fp2
    assert len(fp1) == 16
    assert _fingerprint("a") != _fingerprint("a", "b")  # 顺序敏感
    cs1 = _checksum("payload-x")
    cs2 = _checksum("payload-x")
    assert cs1 == cs2
    assert _checksum("payload-x") != _checksum("payload-y")


def test_t20_integration_points_declared() -> None:
    """T20: INTEGRATION_POINTS 覆盖 4 个对接点 (v1072/hqb/replay/dream)."""
    assert set(INTEGRATION_POINTS.keys()) == {
        "v1072_eternal_identity", "hqb_integration",
        "memory_replay_design", "r7_be_01_dream",
    }
    for k, v in INTEGRATION_POINTS.items():
        assert isinstance(v, str) and len(v) > 50


# ===========================================================================
# Group 6: 约束完整性 + 业务语义 (T21..T23) — 3 用例
# ===========================================================================


def test_t21_mtm_themes_unique_topic_id(mem_store: MemorySchema) -> None:
    """T21: mtm_themes.topic_id PK — 重复 topic_id 抛 IntegrityError."""
    mem_store._conn.execute(
        "INSERT INTO mtm_themes VALUES (?,?,?,?,?,?,?)",
        ("t1", "phi", 1, 5.0, "sum", time.time(), _fingerprint("t1")),
    )
    mem_store._conn.commit()
    with pytest.raises(sqlite3.IntegrityError):
        mem_store._conn.execute(
            "INSERT INTO mtm_themes VALUES (?,?,?,?,?,?,?)",
            ("t1", "phi-dup", 2, 6.0, "sum", time.time(), _fingerprint("t1-dup")),
        )


def test_t22_meta_protected_from_overwrite() -> None:
    """T22: V1094 命名空间键不会被 upgrade 覆盖 — 真生产兼容."""
    s = MemorySchema(":memory:")
    # 手动篡改 V1094 命名空间键 (模拟外部 reader 写入了别的进度)
    s._conn.execute(
        "UPDATE memory_meta SET v='99.99.99' WHERE k='v1094_schema_version'"
    )
    s._conn.commit()
    # 再次 upgrade — 不应被覆盖
    upgrade(s._conn)
    assert s.schema_version() == "99.99.99"  # 用户/迁移自定义值不被吞
    s.close()


def test_t23_v94_constants_consistent() -> None:
    """T23: SCHEMA_V094 / INDEXES_V094 字符串与运行时一致 (供审计/diff)."""
    # SCHEMA 必须包含所有 9 表
    for t in ("memory_hot", "memory_cold", "memory_wal", "memory_dream",
              "memory_snapshots", "stm_messages", "mtm_themes", "ltm_facts",
              "memory_meta"):
        assert f"CREATE TABLE IF NOT EXISTS {t}" in SCHEMA_V094
    # INDEXES 必须包含至少 8 个核心索引
    # 注: idx_wal_event / idx_snapshot_scope_seq / idx_ltm_fingerprint 是 UNIQUE 索引,
    #     在 SCHEMA_V094 表内创建, 不在 INDEXES_V094 集中
    for idx in ("idx_hot_session_ts", "idx_cold_category_imp",
                "idx_dream_status_ts", "idx_stm_session_ts",
                "idx_mtm_last_updated"):
        assert f"CREATE INDEX IF NOT EXISTS {idx}" in INDEXES_V094, f"missing in INDEXES_V094: {idx}"
    # UNIQUE 索引应在 SCHEMA_V094 表内
    for idx in ("idx_wal_event", "idx_snapshot_scope_seq", "idx_ltm_fingerprint"):
        assert f"CREATE UNIQUE INDEX IF NOT EXISTS {idx}" in SCHEMA_V094, f"missing in SCHEMA_V094: {idx}"
    # 版本号一致
    assert V1094_VERSION in SCHEMA_V094


# ===========================================================================
# Group 7: 真生产兼容 (R8-DB-COMPAT, 2026-07-29 数据库工程师补) — 4 用例
#
# 目标:
#   - 旧 memory_store.py v0.3 库升级到 V1094: 旧 episodes/notes 仍可读,
#     旧 schema_version 永不被覆盖.
#   - 非法 op / scope 在 wal_append 应用层即被拒, 不污染 WAL.
#   - downgrade 不会破坏 memory_store 的旧数据.
#   - wal_append 同 event_id 重复调用仍是幂等 (UNIQUE 索引真理之源).
# ===========================================================================


def test_t24_legacy_v030_db_upgrade(tmp_path: Path) -> None:
    """T24: 模拟旧 memory_store.py v0.3 库 — 升级后旧 episodes/notes 仍在, 旧 schema_version=0.3.0 不被覆盖."""
    p = tmp_path / "legacy_v030.db"
    # 1) 用 memory_store 的 SCHEMA 模拟 v0.3 库
    legacy = sqlite3.connect(str(p))
    legacy.executescript(
        """
        CREATE TABLE memory_meta (
            k TEXT PRIMARY KEY,
            v TEXT NOT NULL
        );
        CREATE TABLE episodes (
            eid TEXT PRIMARY KEY, actor TEXT NOT NULL, content TEXT NOT NULL,
            context TEXT DEFAULT '', kind TEXT DEFAULT 'utterance',
            ts REAL NOT NULL, linked_identity_hash TEXT DEFAULT '',
            fingerprint TEXT NOT NULL
        );
        CREATE TABLE notes (
            nid TEXT PRIMARY KEY, topic TEXT NOT NULL, claim TEXT NOT NULL,
            confidence REAL DEFAULT 0.5, importance INTEGER DEFAULT 5,
            evidence TEXT DEFAULT '[]', created_at REAL NOT NULL,
            last_consolidated REAL NOT NULL, supersedes TEXT DEFAULT '[]'
        );
        INSERT INTO episodes(eid, actor, content, ts, fingerprint)
            VALUES ('old-e1', 'master', '历史会话内容', 1700000000.0, 'fp-old-1');
        INSERT INTO notes(nid, topic, claim, created_at, last_consolidated)
            VALUES ('old-n1', '历史主题', '历史事实', 1700000000.0, 1700000000.0);
        INSERT INTO memory_meta(k, v) VALUES ('schema_version', '0.3.0');
        """
    )
    legacy.commit()
    legacy.close()

    # 2) 在同一文件上跑 V1094 upgrade
    upgrade_path(p)

    # 3) 旧表 + 旧行 + 旧 schema_version 必须完整保留
    c = sqlite3.connect(str(p))
    try:
        ep = c.execute("SELECT content FROM episodes WHERE eid='old-e1'").fetchone()
        assert ep is not None and ep[0] == "历史会话内容"
        nt = c.execute("SELECT claim FROM notes WHERE nid='old-n1'").fetchone()
        assert nt is not None and nt[0] == "历史事实"
        sv = c.execute(
            "SELECT v FROM memory_meta WHERE k='schema_version'"
        ).fetchone()
        assert sv is not None and sv[0] == "0.3.0", (
            "V1094 不得覆盖 memory_store v0.3 拥有的 schema_version"
        )
        # 4) V1094 命名空间键被注入
        v94 = c.execute(
            "SELECT v FROM memory_meta WHERE k='v1094_schema_version'"
        ).fetchone()
        assert v94 is not None and v94[0] == V1094_VERSION
        # 5) V1094 自身表全部建好
        for t in ("memory_hot", "memory_cold", "memory_wal", "memory_dream",
                  "memory_snapshots", "stm_messages", "mtm_themes", "ltm_facts"):
            row = c.execute(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?", (t,)
            ).fetchone()
            assert row is not None, f"V1094 table missing: {t}"
    finally:
        c.close()


def test_t25_wal_append_rejects_invalid_op() -> None:
    """T25: wal_append 非法 op / scope 在应用层抛 ValueError, 不污染 WAL."""
    s = MemorySchema(":memory:")
    with pytest.raises(ValueError, match="not in IDEMPOTENT_OPS"):
        s.wal_append("hot", "drop_database", {"k": 1})
    with pytest.raises(ValueError, match="not in WAL_SCOPES"):
        s.wal_append("rogue_scope", "tag_set", {"k": 1})
    # WAL 应保持空 (无非法行落入)
    cur = s._conn.execute("SELECT COUNT(*) FROM memory_wal")
    assert cur.fetchone()[0] == 0
    s.close()


def test_t26_downgrade_preserves_legacy_meta(tmp_path: Path) -> None:
    """T26: downgrade(keep_meta=True) 仅清 V1094 命名空间键 + V1094 表, 旧 memory_store 数据不动."""
    p = tmp_path / "downgrade_keep.db"
    # 1) 旧 v0.3 + V1094 共存
    legacy = sqlite3.connect(str(p))
    legacy.executescript(
        """
        CREATE TABLE memory_meta (k TEXT PRIMARY KEY, v TEXT NOT NULL);
        CREATE TABLE episodes (eid TEXT PRIMARY KEY, actor TEXT NOT NULL,
            content TEXT NOT NULL, ts REAL NOT NULL, fingerprint TEXT NOT NULL);
        INSERT INTO episodes VALUES ('ep-keep', 'master', 'old', 1.0, 'fp');
        INSERT INTO memory_meta(k, v) VALUES ('schema_version', '0.3.0');
        INSERT INTO memory_meta(k, v) VALUES ('legacy_key', 'legacy_value');
        """
    )
    legacy.commit()
    legacy.close()
    upgrade_path(p)
    # 2) downgrade (keep_meta=True)
    downgrade_path(p, keep_meta=True)
    # 3) 旧表 + 旧 meta 完整; V1094 表已清; 命名空间键清掉
    c = sqlite3.connect(str(p))
    try:
        row = c.execute("SELECT content FROM episodes WHERE eid='ep-keep'").fetchone()
        assert row is not None and row[0] == "old"
        legacy_sv = c.execute(
            "SELECT v FROM memory_meta WHERE k='schema_version'"
        ).fetchone()
        assert legacy_sv is not None and legacy_sv[0] == "0.3.0"
        legacy_kv = c.execute(
            "SELECT v FROM memory_meta WHERE k='legacy_key'"
        ).fetchone()
        assert legacy_kv is not None and legacy_kv[0] == "legacy_value"
        # V1094 命名空间键全清
        v94 = c.execute(
            "SELECT 1 FROM memory_meta WHERE k IN "
            "('v1094_schema_version', 'v1094_initialized_ts', 'v1094_downgraded_ts')"
        ).fetchall()
        assert v94 == []
        # V1094 表已 DROP
        for t in ("memory_hot", "memory_wal", "ltm_facts"):
            present = c.execute(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?", (t,)
            ).fetchone()
            assert present is None, f"downgrade 未清 {t}"
    finally:
        c.close()


def test_t27_wal_append_idempotent_on_duplicate_event() -> None:
    """T27: wal_append 同 event_id 二次调用幂等 (SQLite UNIQUE 真理之源)."""
    s = MemorySchema(":memory:")
    eid = s.wal_append("hot", "tag_set", {"x": 1}, event_id="dup-eid")
    # 二次 append: 不抛错, 仍返回同一 event_id, 行数 = 1
    eid2 = s.wal_append("hot", "tag_set", {"x": 1}, event_id="dup-eid")
    assert eid2 == eid
    cur = s._conn.execute("SELECT COUNT(*) FROM memory_wal")
    assert cur.fetchone()[0] == 1
    s.close()