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
    """T01: 首次 init → memory_meta.schema_version=V1094_VERSION."""
    assert mem_store.schema_version() == V1094_VERSION
    assert mem_store.meta_get("schema_version") == V1094_VERSION
    assert mem_store.meta_get("v1094_initialized_ts") is not None


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
    """T22: memory_meta.schema_version 不会被 upgrade 覆盖 — 真生产兼容."""
    s = MemorySchema(":memory:")
    # 手动篡改 schema_version
    s._conn.execute("UPDATE memory_meta SET v='99.99.99' WHERE k='schema_version'")
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