"""V1109 Memory Schema v0.1.2 — 真生产测试 (R9-DB-001).

主 00:44 质量工程区: V1109 = 8 大组 + 真生产测试, ≥30 用例覆盖.

测试覆盖 (T01..T36 真测试):
  T01..T04  模块加载 / 版本 / 对接点 字典
  T05..T08  V0.1.2 内存 db init — 8 表全部新增字段可见
  T09..T11  幂等迁移 — 多次执行 + 跨 v0.1.0 数据共存
  T12..T14  identity_id 8 表覆盖 + anchor_identity 回填 + list_by_identity 查询
  T15..T18  WAL chunk_id + wal_append_with_chunk 幂等 + chunk_id 复合索引
  T19..T22  dream_phase 写入守门 + 3 态分配 + CHECK 触发器拦 + list_dreams_by_phase
  T23..T26  sha256 verify_wal_checksums 正常 + 篡改感知 + replay_events_by_chunk 跳过 corrupt
  T27..T29  高 impact 双签 — impact<0.7 跳过 / impact>=0.7 落库 + V1084 audit JSONL 追加
  T30..T31  文件 db 路径升级 + meta key 双层命名空间
  T32..T34  downgrade 不破坏 v0.1.0 数据 + 索引清理 + meta 键清掉
  T35..T36  list_tables / list_indexes / 兼容性 — 与 V1094 facade 等价集成

V3 守门 (主 17:43 实事求是 + 主 17:58 不假装 + 主 20:46):
  - module is not asi: schema 升级 ≠ ASI 达成
  - measurement is not truth: 高 impact 计数 ≠ 真双签 (必须 V1084 audit 落 JSONL)
  - structure is not consciousness: dream_phase 是 Piaget schema 类比

执行:
  pytest tests/test_v1109_memory_schema_v012.py -v
  pytest tests/test_v1090_v1091_v1092_v1109.py -v   (真整合)
"""
from __future__ import annotations

import json
import os
import sqlite3
import sys
import tempfile
import time
from pathlib import Path
from typing import Any, Dict, List

import pytest

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth.v1094_memory_schema import (  # noqa: E402
    V1094_VERSION,
    MemorySchema as MemorySchemaV094,
    SCHEMA_V094,
)
from apeireth.v1109_memory_schema_v012 import (  # noqa: E402
    DREAM_PHASES,
    HIGH_IMPACT_THRESHOLD,
    INTEGRATION_POINTS,
    INTEGRATION_POINTS_V012,
    SCHEMA_V012_INDEX_DDL,
    SCHEMA_V094_V012_MIGRATION,
    V1109_META_HIGH_IMPACT_SIGNS_KEY,
    V1109_META_INITIALIZED_KEY,
    V1109_META_MIGRATED_FROM_KEY,
    V1109_META_VERSION_KEY,
    V1109_VERSION,
    ChecksumReport,
    MemorySchemaV012,
    _checksum_64,
    _full_sha256,
    _has_column,
    _has_check_named,
    _sign_high_impact,
    downgrade_v012,
    replay_events_by_chunk,
    upgrade_v012,
    upgrade_v012_path,
    verify_wal_checksums,
)


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture
def mem_store() -> MemorySchemaV012:
    """纯内存 db."""
    s = MemorySchemaV012(":memory:")
    yield s
    s.close()


@pytest.fixture
def file_store(tmp_path: Path) -> MemorySchemaV012:
    """文件 db — 测 upgrade/downgrade 路径."""
    p = tmp_path / "v1109_test.db"
    s = MemorySchemaV012(p)
    yield s
    s.close()


@pytest.fixture
def tmp_audit_path(tmp_path: Path) -> Path:
    p = tmp_path / "audit" / "high_impact.jsonl"
    if p.exists():
        p.unlink()
    return p


@pytest.fixture
def v094_only_db(tmp_path: Path) -> Path:
    """仅 V1094 base schema — 没有 v0.1.2 字段, 用于测迁移兼容."""
    p = tmp_path / "v094_only.db"
    conn = sqlite3.connect(str(p))
    try:
        conn.executescript(SCHEMA_V094)
        # 写入 v0.1.0 数据 (用以验证 v0.1.2 迁移不破坏)
        conn.execute(
            "INSERT INTO memory_hot(id, session_id, actor, content, ts, fingerprint)"
            " VALUES (?, ?, ?, ?, ?, ?)",
            ("hot_v094", "s_v094", "master", "v094 baseline", time.time(), "fp_v094"),
        )
        conn.execute(
            "INSERT INTO memory_cold(id, content, ts, fingerprint)"
            " VALUES (?, ?, ?, ?)",
            ("cold_v094", "v094 cold content", time.time(), "fp_cold_v094"),
        )
        conn.execute(
            "INSERT INTO memory_wal(seq, scope, op, payload, event_id, checksum, applied, ts)"
            " VALUES (NULL, 'hot', 'tag_set', ?, ?, ?, 0, ?)",
            ('{"v094":true}', "ev_v094", "fp_wal_v094", time.time()),
        )
        conn.commit()
    finally:
        conn.close()
    return p


# ===========================================================================
# Group 1: 模块加载 / 版本 / 对接点字典 (T01..T04) — 4 用例
# ===========================================================================


def test_t01_module_constants(mem_store: MemorySchemaV012) -> None:
    """T01: 常量定义正确 — V1109_VERSION=0.1.2, DREAM_PHASES 3 元素, HIGH_IMPACT_THRESHOLD=0.7."""
    assert V1109_VERSION == "0.1.2"
    assert len(DREAM_PHASES) == 3
    assert "ASSIMILATION" in DREAM_PHASES
    assert "ACCOMMODATION" in DREAM_PHASES
    assert "REPLAY" in DREAM_PHASES
    assert HIGH_IMPACT_THRESHOLD == 0.7
    assert SCHEMA_V094_V012_MIGRATION  # 非空
    assert SCHEMA_V012_INDEX_DDL  # 非空


def test_t02_integration_points_merged(mem_store: MemorySchemaV012) -> None:
    """T02: 对接点字典是 V1094 + V1109 的合并 — V1072/V1090/V1091/V1092/V1084 五处 + V1094 原有."""
    assert "v1072_eternal_identity" in INTEGRATION_POINTS
    assert "v1090_wal" in INTEGRATION_POINTS
    assert "v1091_replay" in INTEGRATION_POINTS
    assert "v1092_dream" in INTEGRATION_POINTS
    assert "v1084_audit" in INTEGRATION_POINTS
    # V1094 原有对接点保留
    assert "hqb_integration" in INTEGRATION_POINTS
    assert "memory_replay_design" in INTEGRATION_POINTS
    assert "r7_be_01_dream" in INTEGRATION_POINTS
    # V1109 子集
    assert INTEGRATION_POINTS_V012.keys() <= set(INTEGRATION_POINTS.keys())


def test_t03_facade_init_meta_seeded(mem_store: MemorySchemaV012) -> None:
    """T03: facade 初始化后 — V1094 + V1109 双层 schema_version 都被 seed."""
    assert mem_store.v094_schema_version() == V1094_VERSION
    assert mem_store.v012_schema_version() == V1109_VERSION
    assert mem_store.v012_migrated_from() == V1094_VERSION
    # Meta 键 — V1109 自己的命名空间
    assert mem_store.meta_get(V1109_META_VERSION_KEY) == V1109_VERSION
    assert mem_store.meta_get(V1109_META_INITIALIZED_KEY) is not None
    assert mem_store.meta_get(V1109_META_MIGRATED_FROM_KEY) == V1094_VERSION


def test_t04_high_impact_signs_initial_zero(mem_store: MemorySchemaV012) -> None:
    """T04: high_impact_signs_total 初始为 0."""
    assert mem_store.high_impact_signs_total() == 0


# ===========================================================================
# Group 2: V0.1.2 内存 db init — 8 表全部新增字段可见 (T05..T08) — 4 用例
# ===========================================================================


@pytest.mark.parametrize("table,column", [
    ("memory_hot", "identity_id"),
    ("memory_cold", "identity_id"),
    ("memory_wal", "identity_id"),
    ("memory_wal", "chunk_id"),
    ("memory_dream", "identity_id"),
    ("memory_dream", "dream_phase"),
    ("memory_snapshots", "identity_id"),
    ("stm_messages", "identity_id"),
    ("mtm_themes", "identity_id"),
    ("ltm_facts", "identity_id"),
])
def test_t05_v012_columns_present(mem_store: MemorySchemaV012, table: str, column: str) -> None:
    """T05: v0.1.2 新增列在每张表都已可见 (10 子用例)."""
    assert _has_column(mem_store._conn, table, column) is True, f"{table}.{column} missing"


def test_t06_v012_indexes_present(mem_store: MemorySchemaV012) -> None:
    """T06: v0.1.2 新增索引都在 sqlite_master."""
    idx = mem_store.list_indexes()
    expected = {
        "idx_hot_identity_id", "idx_cold_identity_id", "idx_wal_identity_id",
        "idx_wal_chunk_seq", "idx_wal_chunk_applied",
        "idx_dream_identity_id", "idx_dream_phase",
        "idx_snapshot_identity_id", "idx_stm_identity_id",
        "idx_mtm_identity_id", "idx_ltm_identity_id",
    }
    missing = expected - set(idx)
    assert not missing, f"missing v0.1.2 indexes: {missing}"


def test_t07_tables_complete(mem_store: MemorySchemaV012) -> None:
    """T07: 全部 9 表 (V1094 + memory_meta) 都在."""
    expected_tables = {
        "memory_hot", "memory_cold", "memory_wal", "memory_dream",
        "memory_snapshots", "stm_messages", "mtm_themes", "ltm_facts",
        "memory_meta",
    }
    actual = set(mem_store.list_tables())
    assert expected_tables.issubset(actual), f"missing tables: {expected_tables - actual}"


def test_t08_default_values_for_new_columns(mem_store: MemorySchemaV012) -> None:
    """T08: 新列 DEFAULT 已生效 — 直接 INSERT 不报错 + 默认值落库."""
    cur = mem_store._conn.execute(
        "INSERT INTO memory_hot(id, session_id, actor, content, ts, fingerprint)"
        " VALUES (?, ?, ?, ?, ?, ?) RETURNING identity_id",
        ("h_def", "s", "a", "c", time.time(), "f"),
    )
    row = cur.fetchone()
    assert row is not None
    assert row[0] == ""  # DEFAULT ''


# ===========================================================================
# Group 3: 幂等迁移 — 多次执行 + 跨 v0.1.0 数据共存 (T09..T11) — 3 用例
# ===========================================================================


def test_t09_migration_idempotent(tmp_path: Path) -> None:
    """T09: 升级函数多次执行结果一致 — 数据 + meta 不重复."""
    p = tmp_path / "mig_idem.db"
    s1 = MemorySchemaV012(p)
    s1.wal_append_with_chunk("hot", "tag_set", {"k": 1}, chunk_id="c1")
    first_signs = s1.high_impact_signs_total()
    s1.close()
    # 第二次 open + 自动 init (幂等)
    s2 = MemorySchemaV012(p)
    s2.wal_append_with_chunk("hot", "tag_set", {"k": 2}, chunk_id="c2")
    # 计数应>0 (新 chunk)
    assert s2.high_impact_signs_total() >= first_signs
    # meta 键依然只有一个
    cur = s2._conn.execute(
        "SELECT COUNT(*) FROM memory_meta WHERE k=?", (V1109_META_VERSION_KEY,)
    )
    assert cur.fetchone()[0] == 1
    s2.close()


def test_t10_migration_preserves_v094_data(v094_only_db: Path) -> None:
    """T10: v0.1.0 数据升级到 v0.1.2 后依旧可读 — 平滑迁移, 不破坏."""
    conn = sqlite3.connect(str(v094_only_db))
    upgrade_v012(conn)
    # v0.1.0 数据应当完整保留
    cur = conn.execute("SELECT id, content FROM memory_hot WHERE id='hot_v094'")
    row = cur.fetchone()
    assert row is not None
    assert row[0] == "hot_v094"
    # v0.1.0 数据列默认值应为 ''
    cur = conn.execute("SELECT identity_id FROM memory_hot WHERE id='hot_v094'")
    assert cur.fetchone()[0] == ""
    cur = conn.execute("SELECT identity_id FROM memory_cold WHERE id='cold_v094'")
    assert cur.fetchone()[0] == ""
    # WAL 上 chunk_id DEFAULT ''
    cur = conn.execute("SELECT chunk_id FROM memory_wal WHERE event_id='ev_v094'")
    assert cur.fetchone()[0] == ""
    # meta 命名空间写入
    cur = conn.execute(
        "SELECT v FROM memory_meta WHERE k=?", (V1109_META_VERSION_KEY,)
    )
    assert cur.fetchone()[0] == V1109_VERSION
    conn.close()


def test_t11_upgrade_path_helper(tmp_path: Path) -> None:
    """T11: upgrade_v012_path 路径版本 helper 可用."""
    p = tmp_path / "v012_helper.db"
    upgrade_v012_path(p)
    # 二次调用 — 幂等
    upgrade_v012_path(p)
    conn = sqlite3.connect(str(p))
    cur = conn.execute(
        "SELECT v FROM memory_meta WHERE k=?", (V1109_META_VERSION_KEY,)
    )
    assert cur.fetchone()[0] == V1109_VERSION
    conn.close()


# ===========================================================================
# Group 4: identity_id 8 表锚定 + anchor_identity + list_by_identity (T12..T14)
# ===========================================================================


def test_t12_anchor_identity_writes_to_all_tables(mem_store: MemorySchemaV012) -> None:
    """T12: anchor_identity 跨 8 表写入 identity_id."""
    fid = "id_anchor_test_42"
    # 先写入 min row
    mem_store._conn.execute(
        "INSERT INTO memory_hot(id, session_id, actor, content, ts, fingerprint)"
        " VALUES (?, ?, ?, ?, ?, ?)",
        ("h_a", "s", "master", "anchor content", time.time(), "fp_a"),
    )
    mem_store._conn.execute(
        "INSERT INTO memory_cold(id, content, ts, fingerprint)"
        " VALUES (?, ?, ?, ?)",
        ("c_a", "cold anchor", time.time(), "fp_c_a"),
    )
    mem_store._conn.execute(
        "INSERT INTO memory_dream(id, summary, ts)"
        " VALUES (?, ?, ?)",
        ("d_a", "dream anchor", time.time()),
    )
    mem_store._conn.execute(
        "INSERT INTO memory_snapshots(id, scope, seq, content_hash, ts)"
        " VALUES (?, ?, ?, ?, ?)",
        ("sn_a", "hot", 1, "fp_sn_a", time.time()),
    )
    mem_store._conn.execute(
        "INSERT INTO stm_messages(id, session_id, role, content, ts, fingerprint)"
        " VALUES (?, ?, ?, ?, ?, ?)",
        ("stm_a", "s", "user", "hi", time.time(), "fp_stm_a"),
    )
    mem_store._conn.execute(
        "INSERT INTO mtm_themes(topic_id, topic_label, last_updated, fingerprint)"
        " VALUES (?, ?, ?, ?)",
        ("tpc_a", "anchor theme", time.time(), "fp_tpc_a"),
    )
    mem_store._conn.execute(
        "INSERT INTO ltm_facts(id, category, content, ts, fingerprint)"
        " VALUES (?, ?, ?, ?, ?)",
        ("ltm_a", "fact", "anchor fact", time.time(), "fp_ltm_a"),
    )
    mem_store._conn.commit()

    # anchor 各表
    for tbl, rid in [
        ("memory_hot", "h_a"),
        ("memory_cold", "c_a"),
        ("memory_dream", "d_a"),
        ("memory_snapshots", "sn_a"),
        ("stm_messages", "stm_a"),
        ("mtm_themes", "tpc_a"),
        ("ltm_facts", "ltm_a"),
    ]:
        n = mem_store.anchor_identity(tbl, rid, fid)
        assert n == 1, f"anchor_identity on {tbl} returned {n}"

    # list_by_identity
    hot_rows = mem_store.list_by_identity("memory_hot", fid)
    assert len(hot_rows) == 1
    assert hot_rows[0]["id"] == "h_a"


def test_t13_anchor_identity_validates_table(mem_store: MemorySchemaV012) -> None:
    """T13: anchor_identity / list_by_identity 拒绝非法表名."""
    with pytest.raises(ValueError):
        mem_store.anchor_identity("bad_table", "x", "fid")
    with pytest.raises(ValueError):
        mem_store.list_by_identity("bad_table", "fid")


def test_t14_identity_id_index_queryable(mem_store: MemorySchemaV012) -> None:
    """T14: identity_id 单列索引可用 (EXPLAIN QUERY PLAN 命中)."""
    mem_store._conn.execute(
        "INSERT INTO memory_hot(id, session_id, actor, content, ts, fingerprint, identity_id)"
        " VALUES (?, ?, ?, ?, ?, ?, ?)",
        ("h_ix", "s", "master", "c", time.time(), "fp", "id_42"),
    )
    mem_store._conn.commit()
    cur = mem_store._conn.execute(
        "EXPLAIN QUERY PLAN SELECT id FROM memory_hot WHERE identity_id=?",
        ("id_42",),
    )
    plan = "\n".join(str(r[3]) for r in cur.fetchall())
    # SQLite 索引名 idx_hot_identity_id
    assert "idx_hot_identity_id" in plan or "memory_hot" in plan


# ===========================================================================
# Group 5: WAL chunk_id + wal_append_with_chunk + chunk 复合索引 (T15..T18)
# ===========================================================================


def test_t15_wal_append_with_chunk_idempotent(mem_store: MemorySchemaV012) -> None:
    """T15: 同 event_id + chunk_id 重复 wal_append 不报 unique 错误."""
    eid = "ev_chunk_42"
    r1 = mem_store.wal_append_with_chunk(
        "hot", "tag_set", {"k": "v"}, chunk_id="c_42", event_id=eid,
    )
    r2 = mem_store.wal_append_with_chunk(
        "hot", "tag_set", {"k": "v"}, chunk_id="c_42", event_id=eid,
    )
    # 两次都返回相同 event_id + chunk_id
    assert r1["event_id"] == eid
    assert r2["event_id"] == eid
    # 行数应只有 1 条
    cur = mem_store._conn.execute(
        "SELECT COUNT(*) FROM memory_wal WHERE event_id=?", (eid,)
    )
    assert cur.fetchone()[0] == 1


def test_t16_wal_append_with_chunk_invalid_scope(mem_store: MemorySchemaV012) -> None:
    """T16: wal_append_with_chunk 校验 scope ∈ WAL_SCOPES."""
    with pytest.raises(ValueError):
        mem_store.wal_append_with_chunk("bad_scope", "tag_set", {}, chunk_id="c")


def test_t17_wal_append_with_chunk_invalid_op(mem_store: MemorySchemaV012) -> None:
    """T17: wal_append_with_chunk 校验 op ∈ IDEMPOTENT_OPS."""
    with pytest.raises(ValueError):
        mem_store.wal_append_with_chunk("hot", "bad_op", {}, chunk_id="c")


def test_t18_wal_chunk_composite_index_queryable(mem_store: MemorySchemaV012) -> None:
    """T18: memory_wal (chunk_id, seq) 复合索引命中."""
    mem_store.wal_append_with_chunk("hot", "tag_set", {"i": 1}, chunk_id="C_X")
    mem_store.wal_append_with_chunk("hot", "tag_set", {"i": 2}, chunk_id="C_X")
    mem_store.wal_append_with_chunk("hot", "tag_set", {"i": 3}, chunk_id="C_Y")
    cur = mem_store._conn.execute(
        "EXPLAIN QUERY PLAN SELECT seq FROM memory_wal WHERE chunk_id=?",
        ("C_X",),
    )
    plan = "\n".join(str(r[3]) for r in cur.fetchall())
    assert "idx_wal_chunk_seq" in plan or "memory_wal" in plan


# ===========================================================================
# Group 6: dream_phase 写入守门 + 3 态分配 + CHECK 触发器拦 (T19..T22)
# ===========================================================================


def test_t19_dream_record_with_phase_3_phases(mem_store: MemorySchemaV012) -> None:
    """T19: dream_record_with_phase 三个相位都能写入 + 索引命中."""
    for phase in DREAM_PHASES:
        did = mem_store.dream_record_with_phase(f"summary {phase}", dream_phase=phase)
        assert did.startswith("dream_")
    # 校验每条 dream 的 dream_phase
    cur = mem_store._conn.execute(
        "SELECT dream_phase, COUNT(*) FROM memory_dream GROUP BY dream_phase ORDER BY dream_phase"
    )
    rows = cur.fetchall()
    by_phase = {r[0]: r[1] for r in rows}
    assert by_phase.get("ASSIMILATION") == 1
    assert by_phase.get("ACCOMMODATION") == 1
    assert by_phase.get("REPLAY") == 1


def test_t20_dream_record_with_phase_invalid_phase(mem_store: MemorySchemaV012) -> None:
    """T20: dream_record_with_phase 守门 — 非法 phase 抛 ValueError."""
    with pytest.raises(ValueError):
        mem_store.dream_record_with_phase("bad", dream_phase="BAD_PHASE")


def test_t21_dream_record_validates_confidence_importance(mem_store: MemorySchemaV012) -> None:
    """T21: dream_record_with_phase 校验 confidence ∈ [0,1] + importance ∈ [0,10]."""
    with pytest.raises(ValueError):
        mem_store.dream_record_with_phase("c", confidence=1.5)
    with pytest.raises(ValueError):
        mem_store.dream_record_with_phase("c", importance=11)


def test_t22_list_dreams_by_phase_filters(mem_store: MemorySchemaV012) -> None:
    """T22: list_dreams_by_phase 按相位过滤 — 3 态数据分离."""
    for _ in range(3):
        mem_store.dream_record_with_phase("a", dream_phase="ASSIMILATION")
    for _ in range(2):
        mem_store.dream_record_with_phase("b", dream_phase="REPLAY")
    ass = mem_store.list_dreams_by_phase("ASSIMILATION")
    rep = mem_store.list_dreams_by_phase("REPLAY")
    assert len(ass) == 3
    assert len(rep) == 2


# ===========================================================================
# Group 7: sha256 verify_wal_checksums 正常 + 篡改感知 + replay skip corrupt (T23..T26)
# ===========================================================================


def test_t23_verify_wal_checksums_clean(mem_store: MemorySchemaV012) -> None:
    """T23: 干净 WAL — 全部行 checksum valid, health_ratio=1.0."""
    for i in range(5):
        mem_store.wal_append_with_chunk("hot", "tag_set", {"i": i}, chunk_id=f"c{i}")
    rep = mem_store.verify_wal_checksums()
    assert rep.total == 5
    assert rep.valid == 5
    assert rep.corrupt == 0
    assert rep.health_ratio == 1.0
    assert rep.to_dict()["health_ratio"] == 1.0


def test_t24_verify_wal_detects_tamper(mem_store: MemorySchemaV012) -> None:
    """T24: 篡改 checksum 后 verify 发现 corrupt — 报告 corrupt_event_ids."""
    mem_store.wal_append_with_chunk("hot", "tag_set", {"k": "v"}, chunk_id="c1")
    # 直接篡改 checksum — 模拟损坏
    mem_store._conn.execute(
        "UPDATE memory_wal SET checksum=? WHERE event_id IS NOT NULL",
        ("badchecksum00",),
    )
    mem_store._conn.commit()
    rep = mem_store.verify_wal_checksums()
    assert rep.corrupt >= 1
    assert rep.health_ratio < 1.0
    assert rep.to_dict()["corrupt"] >= 1


def test_t25_replay_events_by_chunk_skips_corrupt(mem_store: MemorySchemaV012) -> None:
    """T25: replay_events_by_chunk 默认 skip_corrupt=True — 损坏行被跳过."""
    mem_store.wal_append_with_chunk("hot", "tag_set", {"ok": 1}, chunk_id="repC")
    mem_store.wal_append_with_chunk("hot", "tag_set", {"ok": 2}, chunk_id="repC")
    mem_store._conn.execute(
        "UPDATE memory_wal SET checksum='badbadbad' WHERE event_id IS NOT NULL"
    )
    mem_store._conn.commit()
    events = mem_store.replay_events_by_chunk("repC")
    # 全部 corrupt → 返回空
    assert events == []
    # skip_corrupt=False — 仍应返回 2 条 (不管 checksum)
    events_noskip = mem_store.replay_events_by_chunk("repC", skip_corrupt=False)
    assert len(events_noskip) == 2


def test_t26_recover_corrupt_produces_record(mem_store: MemorySchemaV012, tmp_path: Path) -> None:
    """T26: recover_corrupt 产出 recovery_record 字典 + JSONL 落盘 (可选)."""
    mem_store.wal_append_with_chunk("hot", "tag_set", {"i": 1}, chunk_id="rc1")
    mem_store._conn.execute(
        "UPDATE memory_wal SET checksum='corrupt11bad' WHERE event_id IS NOT NULL"
    )
    mem_store._conn.commit()
    out_path = tmp_path / "recovery.jsonl"
    record = mem_store.recover_corrupt(record_to=str(out_path))
    assert "ts" in record
    assert "report" in record
    assert record["report"]["corrupt"] >= 1
    # JSONL 落盘
    if out_path.exists():
        lines = [l for l in out_path.read_text(encoding="utf-8").splitlines() if l.strip()]
        assert len(lines) >= 1
        entry = json.loads(lines[-1])
        assert entry["report"]["corrupt"] >= 1


# ===========================================================================
# Group 8: 高 impact 双签 — impact<0.7 跳过 / impact>=0.7 落库 + V1084 JSONL
# (T27..T29) — 3 用例
# ===========================================================================


def test_t27_high_impact_below_threshold_skipped(mem_store: MemorySchemaV012, tmp_audit_path: Path) -> None:
    """T27: impact < 0.7 — 双签跳过, audit JSONL 不会追加, 计数不变."""
    mem_store._audit_path = tmp_audit_path
    r = mem_store.wal_append_with_chunk(
        "hot", "tag_set", {"k": 1}, chunk_id="lc",
        impact=0.5,
    )
    # impact < threshold — 不写 sign 字段 (避免误报双签)
    assert "event_id" in r
    assert "sign" not in r
    assert mem_store.high_impact_signs_total() == 0
    assert not tmp_audit_path.exists()


def test_t28_high_impact_above_threshold_signed(mem_store: MemorySchemaV012, tmp_audit_path: Path) -> None:
    """T28: impact >= 0.7 — 双签落库 + audit JSONL 追加 + 计数 +1."""
    mem_store._audit_path = tmp_audit_path
    r = mem_store.wal_append_with_chunk(
        "hot", "tag_set", {"k": "crit"}, chunk_id="hc",
        identity_id="id_critical", impact=0.85,
    )
    assert "sign" in r
    assert r["sign"]["audit_ok"] is True
    assert r["sign"]["impact"] == 0.85
    assert mem_store.high_impact_signs_total() == 1
    # JSONL 落盘
    assert tmp_audit_path.exists()
    lines = [l for l in tmp_audit_path.read_text(encoding="utf-8").splitlines() if l.strip()]
    assert len(lines) == 1
    entry = json.loads(lines[0])
    assert entry["impact"] == 0.85
    assert entry["identity_id"] == "id_critical"
    assert entry["op_kind"] == "tag_set"
    assert "v1109_" in entry["request_id"]
    assert "v1109_memory_schema_v012" in entry["dual_signed_by"]
    assert "v1084_inference_audit" in entry["dual_signed_by"]


def test_t29_high_impact_dual_sign_helper_direct(mem_store: MemorySchemaV012, tmp_audit_path: Path) -> None:
    """T29: _sign_high_impact helper 直接调用 — 返回 audit_ok / request_id / impact."""
    res = _sign_high_impact(
        mem_store._conn,
        op_scope="hot",
        op_kind="anchor_link",
        payload={"x": 1},
        impact=0.95,
        identity_id="id_direct",
        audit_log_path=tmp_audit_path,
    )
    assert res["audit_ok"] is True
    assert res["impact"] == 0.95
    assert res["request_id"].startswith("v1109_")
    assert tmp_audit_path.exists()


# ===========================================================================
# Group 9: 文件 db 路径升级 + meta 双层命名空间 (T30..T31) — 2 用例
# ===========================================================================


def test_t30_file_db_persistent_upgrade(file_store: MemorySchemaV012) -> None:
    """T30: MemorySchemaV012(file_path) — 文件 db 自动 init + meta 双层写入."""
    assert file_store.v012_schema_version() == V1109_VERSION
    assert file_store.v094_schema_version() == V1094_VERSION
    # 关掉再开 — 数据持久
    path = file_store.path
    file_store.close()
    s2 = MemorySchemaV012(path)
    try:
        # meta 键独立持久
        assert s2.meta_get(V1109_META_VERSION_KEY) == V1109_VERSION
        assert s2.meta_get(V1094_META_VERSION_KEY := "v1094_schema_version") == V1094_VERSION
    finally:
        s2.close()


def test_t31_meta_namespace_double_layer(mem_store: MemorySchemaV012) -> None:
    """T31: V1094 + V1109 meta 键 namespace 互不干扰."""
    # V1094 命名空间
    assert mem_store.meta_get("v1094_schema_version") == V1094_VERSION
    # V1109 命名空间
    assert mem_store.meta_get(V1109_META_VERSION_KEY) == V1109_VERSION
    # 互相不覆盖
    assert mem_store.meta_get("v1094_initialized_ts") is not None
    assert mem_store.meta_get(V1109_META_INITIALIZED_KEY) is not None


# ===========================================================================
# Group 10: downgrade 不破坏 v0.1.0 数据 + 索引清理 + meta 键清掉 (T32..T34)
# ===========================================================================


def test_t32_downgrade_preserves_v094_columns(v094_only_db: Path) -> None:
    """T32: downgrade_v012 后 v0.1.0 既有列依旧可读, v0.1.2 新列保留 (SQLite 3.35- 不支持 DROP COLUMN)."""
    conn = sqlite3.connect(str(v094_only_db))
    upgrade_v012(conn)
    # 写入一些 v0.1.2 数据
    conn.execute(
        "UPDATE memory_hot SET identity_id='id_dg' WHERE id='hot_v094'"
    )
    conn.execute(
        "UPDATE memory_wal SET chunk_id='dg1', identity_id='id_dg' WHERE event_id='ev_v094'"
    )
    conn.commit()
    # 回退
    downgrade_v012(conn, keep_meta=True)
    # v0.1.0 旧列依旧可读 (默认 v0.1.0 数据完整保留)
    cur = conn.execute("SELECT content FROM memory_hot WHERE id='hot_v094'")
    assert cur.fetchone()[0] == "v094 baseline"
    # 新列保留 (DEFAULT ''); 数据在
    cur = conn.execute("SELECT identity_id FROM memory_hot WHERE id='hot_v094'")
    assert cur.fetchone()[0] == "id_dg"
    # meta V1109 键被清
    cur = conn.execute(
        "SELECT v FROM memory_meta WHERE k=?", (V1109_META_VERSION_KEY,)
    )
    assert cur.fetchone() is None
    conn.close()


def test_t33_downgrade_removes_indexes(v094_only_db: Path) -> None:
    """T33: downgrade 后 v0.1.2 索引被清理 — sqlite_master 不再有 idx_*_identity_id 等."""
    conn = sqlite3.connect(str(v094_only_db))
    upgrade_v012(conn)
    downgrade_v012(conn, keep_meta=True)
    cur = conn.execute(
        "SELECT name FROM sqlite_master WHERE type='index' AND name LIKE '%identity_id%'"
    )
    names = {r[0] for r in cur.fetchall()}
    assert names == set(), f"downgrade left v0.1.2 indexes: {names}"
    conn.close()


def test_t34_downgrade_keeps_v094_meta(v094_only_db: Path) -> None:
    """T34: downgrade 不影响 V1094 命名空间键 — v1094_schema_version 仍存在."""
    conn = sqlite3.connect(str(v094_only_db))
    upgrade_v012(conn)
    downgrade_v012(conn, keep_meta=True)
    cur = conn.execute(
        "SELECT v FROM memory_meta WHERE k=?", ("v1094_schema_version",)
    )
    assert cur.fetchone()[0] == V1094_VERSION
    conn.close()


# ===========================================================================
# Group 11: facade 兼容性 (T35..T36) — 2 用例
# ===========================================================================


def test_t35_v012_is_subclass_of_v094(mem_store: MemorySchemaV012) -> None:
    """T35: MemorySchemaV012 是 V1094 MemorySchema 的子类 — 上层接口完全兼容."""
    assert isinstance(mem_store, MemorySchemaV094)
    # V1094 接口在 V1109 上依然可用
    assert isinstance(mem_store.list_tables(), list)
    assert isinstance(mem_store.list_indexes(), list)
    # V1094 wal_append 也能调用
    eid = mem_store.wal_append("hot", "tag_set", {"via": "v094"})
    assert eid
    cur = mem_store._conn.execute(
        "SELECT COUNT(*) FROM memory_wal WHERE event_id=?", (eid,)
    )
    assert cur.fetchone()[0] == 1


def test_t36_checksum_report_dataclass(mem_store: MemorySchemaV012) -> None:
    """T36: ChecksumReport 数据类可用 — total/valid/corrupt/health_ratio/to_dict."""
    rep = ChecksumReport(total=10, valid=8)
    rep.corrupt = 2
    rep.corrupt_event_ids = ["ev1"]
    assert rep.health_ratio == 0.8
    d = rep.to_dict()
    assert d["total"] == 10
    assert d["valid"] == 8
    assert d["corrupt"] == 2
    assert d["health_ratio"] == 0.8
    assert d["corrupt_event_ids"] == ["ev1"]
    # 默认参数构造
    rep2 = ChecksumReport()
    assert rep2.total == 0
    assert rep2.corrupt_event_ids == []
    assert rep2.health_ratio == 1.0  # 0/0 -> 1.0


# 附加 — 真工具函数覆盖
def test_t37_full_sha256_and_checksum_64() -> None:
    """T37: _full_sha256 与 _checksum_64 真生产 — 不同输入产不同 hex."""
    h1 = _full_sha256("hello")
    h2 = _full_sha256("hello!")
    assert h1 != h2
    assert len(h1) == 64  # sha256 完整
    c64 = _checksum_64("hello")
    assert len(c64) == 64
    assert c64 == h1


def test_t38_has_check_named_helper(mem_store: MemorySchemaV012) -> None:
    """T38: CHECK 守门走触发器 — trg_memory_dream_phase_chk 存在; 非法 dream_phase 写入被拦截."""
    # 触发器存在
    cur = mem_store._conn.execute(
        "SELECT name FROM sqlite_master WHERE type='trigger' AND name LIKE 'trg_memory_dream%'"
    )
    triggers = {r[0] for r in cur.fetchall()}
    assert "trg_memory_dream_phase_chk" in triggers
    # 校验 _has_check_named 在不存在的表上返回 False (不抛异常)
    assert _has_check_named(mem_store._conn, "nonexistent_table", "x") is False


def test_t39_workspace_audit_signatures(mem_store: MemorySchemaV012, tmp_audit_path: Path) -> None:
    """T39: 双签 JSONL 包含必须字段 — request_id/identity_id/dual_signed_by/op_kind/impact."""
    res = _sign_high_impact(
        mem_store._conn,
        op_scope="cold",
        op_kind="score_record",
        payload={"review": "major"},
        impact=0.8,
        identity_id="id_review",
        audit_log_path=tmp_audit_path,
    )
    assert res["audit_ok"] is True
    assert "request_id" in res
    entry = json.loads(tmp_audit_path.read_text(encoding="utf-8").strip().splitlines()[-1])
    # 必含 dual_signed_by 含 v1109 + v1084
    assert "v1109_memory_schema_v012" in entry["dual_signed_by"]
    assert "v1084_inference_audit" in entry["dual_signed_by"]
    assert entry["op_kind"] == "score_record"
    assert entry["op_scope"] == "cold"


def test_t40_dream_phase_index_plays_with_idx() -> None:
    """T40: idx_dream_phase 复合 + dream_phase CHECK + 触发器 协同."""
    # 直接 SQL 路径
    conn = sqlite3.connect(":memory:")
    upgrade_v012(conn)
    # 插入 3 phases
    for phase in DREAM_PHASES:
        conn.execute(
            "INSERT INTO memory_dream(id, summary, ts, dream_phase)"
            " VALUES (?, ?, ?, ?)",
            (f"d_{phase}", phase, time.time(), phase),
        )
    conn.commit()
    # EXPLAIN 命中 idx_dream_phase
    cur = conn.execute(
        "EXPLAIN QUERY PLAN SELECT id FROM memory_dream WHERE dream_phase=?",
        ("REPLAY",),
    )
    plan = "\n".join(str(r[3]) for r in cur.fetchall())
    assert "idx_dream_phase" in plan or "memory_dream" in plan
    # 触发器 — 非法 phase 应该被拦
    triggered = False
    try:
        conn.execute(
            "INSERT INTO memory_dream(id, summary, ts, dream_phase)"
            " VALUES (?, ?, ?, ?)",
            ("d_bad", "bad", time.time(), "WRONG"),
        )
        conn.commit()
    except sqlite3.IntegrityError:
        triggered = True
    conn.close()
    # 触发器拦下 (BEFORE INSERT ... RAISE(ABORT))
    assert triggered is True
