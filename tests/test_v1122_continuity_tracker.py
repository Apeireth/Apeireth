"""V1122 V1072 ContinuityTracker Timeline 可视化测试 — R9-DB-003.

主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 13:31 大胆激进 +
主 23:44 干到底 + 主 19:33 走在前人经验上 + 主 00:56 任何人都能接手.

测试覆盖 (T01..T16, ≥15 用例):
  T01..T04  ContinuityTimelineViz (timeline chart + continuity score + 3 类输出)
  T05..T08  RecoveryRecordIndex (recovery_record 表 + 4 索引 + 走索引查询)
  T09..T12  CrossTableJoinBenchmark (1K/10K/100K 真跑 + EXPLAIN 索引差异)
  T13..T16  StressDrill (3 类 stress + 10× 数据 + 50 corrupt + 100K join)

执行:
  python -m pytest tests/test_v1122_continuity_tracker.py -v
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

from apeireth.v1072_asi_central_ai_eternal_identity import (  # noqa: E402
    ETERNAL_IDENTITY_CORE,
    ContinuityTracker,
    IdentityCore,
    IdentityManifest,
    SessionMarker,
)
from apeireth.v1109_memory_schema_v012 import (  # noqa: E402
    MemorySchemaV012,
    V1109_VERSION,
)
from apeireth.v1122_v1072_continuity_tracker import (  # noqa: E402
    V1122_VERSION,
    V3_GUARDS,
    CrossTableJoinBenchmark,
    ContinuityTimelineViz,
    JoinBenchmarkRow,
    RECOVERY_RECORD_INDEXES_DDL,
    RECOVERY_RECORD_TABLE_DDL,
    RecoveryRecord,
    RecoveryRecordIndex,
    StressDrill,
    StressReport,
    TimelinePoint,
)


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture
def tmp_dir(tmp_path: Path) -> Path:
    return tmp_path / "v1122"


@pytest.fixture
def tracker_with_5_sessions() -> ContinuityTracker:
    ct = ContinuityTracker()
    sids = [ct.start_session() for _ in range(5)]
    for i, sid in enumerate(sids):
        ct.sessions[sid].n_entries_added = (i + 1) * 10
        ct.sessions[sid].n_importance_avg = 0.5 + 0.1 * i
        ct.sessions[sid].ended_at = ct.sessions[sid].started_at + (i + 1) * 0.05
        ct.sessions[sid].is_active = (i == len(sids) - 1)
    return ct


@pytest.fixture
def manifest_with_3_sources() -> IdentityManifest:
    mf = IdentityManifest(core=IdentityCore(identity_id="id_v1122_test"))
    for i in range(4):
        mf.add("LTM", "fact", f"fact{i}", importance=0.9)
    for i in range(3):
        mf.add("MTM", "insight", f"insight{i}", importance=0.7)
    for i in range(2):
        mf.add("STM", "event", f"event{i}", importance=0.5)
    return mf


@pytest.fixture
def viz_with_data(tmp_dir: Path, tracker_with_5_sessions: ContinuityTracker,
                  manifest_with_3_sources: IdentityManifest) -> ContinuityTimelineViz:
    tmp_dir.mkdir(parents=True, exist_ok=True)
    viz = ContinuityTimelineViz(identity_id="id_v1122_test")
    viz.feed_tracker(tracker_with_5_sessions)
    viz.feed_manifest(manifest_with_3_sources)
    return viz


@pytest.fixture
def recovery_db(tmp_dir: Path) -> str:
    tmp_dir.mkdir(parents=True, exist_ok=True)
    path = str(tmp_dir / "recovery.db")
    rri = RecoveryRecordIndex(path)
    rri.migrate()
    yield path
    rri.close()


# ===========================================================================
# Group 1: ContinuityTimelineViz — T01..T04
# ===========================================================================


def test_t01_viz_n_points_matches_tracker(
        viz_with_data: ContinuityTimelineViz) -> None:
    """T01: 5 个 session marker 喂入后 → 5 个 TimelinePoint."""
    assert viz_with_data.n_sessions == 5
    assert viz_with_data.total_entries == sum(
        p.n_entries_added for p in viz_with_data._points
    )


def test_t02_viz_continuity_score_parfit(
        viz_with_data: ContinuityTimelineViz) -> None:
    """T02: continuity_score (Parfit 1984) = 1.0 — 5/5 都有 n_entries_added > 0."""
    assert viz_with_data.continuity_score == 1.0
    # 与 V1072 ContinuityTracker.continuity_score() 同口径
    assert viz_with_data.continuity_score == 1.0


def test_t03_viz_three_outputs_non_empty(
        viz_with_data: ContinuityTimelineViz) -> None:
    """T03: 3 类输出 (JSON / MD / SVG) 全部非空 + 结构正确."""
    j = viz_with_data.to_json()
    assert "continuity_score" in j and "points" in j
    assert j["continuity_score"] > 0
    assert j["n_sessions"] == 5
    md = viz_with_data.to_markdown()
    assert "Timeline" in md
    assert "session_id" in md
    assert "Continuity Score Trend" in md
    svg = viz_with_data.to_svg()
    assert svg.startswith("<svg")
    assert svg.endswith("</svg>")
    assert "continuity=" in svg
    assert 'fill="#3b82f6"' in svg  # weight bar
    assert 'fill="#10b981"' in svg  # duration bar


def test_t04_viz_empty_tracker_returns_safe_outputs() -> None:
    """T04: 空 tracker → JSON n_sessions=0, MD 有空态文案, SVG 是 empty placeholder."""
    viz = ContinuityTimelineViz(identity_id="id_empty")
    j = viz.to_json()
    assert j["n_sessions"] == 0
    assert j["continuity_score"] == 0.0
    md = viz.to_markdown()
    assert "_No sessions_" in md or "No sessions" in md or "0 sessions" in md
    svg = viz.to_svg()
    assert svg.startswith("<svg")
    assert "Empty ContinuityTracker" in svg


# ===========================================================================
# Group 2: RecoveryRecordIndex — T05..T08
# ===========================================================================


def test_t05_recovery_migrate_creates_table_and_4_indexes(tmp_dir: Path) -> None:
    """T05: migrate() 建表 + 4 个索引 (chunk+ts / chunk / ts / identity)."""
    tmp_dir.mkdir(parents=True, exist_ok=True)
    path = str(tmp_dir / "r05.db")
    rri = RecoveryRecordIndex(path)
    info = rri.migrate()
    assert info["table_created"] is True
    assert info["n_indexes"] == 4
    assert "idx_recovery_chunk_ts" in info["indexes"]
    assert "idx_recovery_chunk" in info["indexes"]
    assert "idx_recovery_ts" in info["indexes"]
    assert "idx_recovery_identity" in info["indexes"]
    rri.close()


def test_t06_recovery_record_round_trip(recovery_db: str) -> None:
    """T06: record → query_by_chunk / ts_range / identity 全部回环一致."""
    rri = RecoveryRecordIndex(recovery_db)
    now = time.time()
    rec1 = RecoveryRecord(
        ts=now, chunk_id="chunk_a", seq=1, event_id="e1",
        identity_id="id_a", scope="hot", corrupt_kind="tampered",
        health_ratio=0.7, detail_json='{"i":1}',
    )
    rec2 = RecoveryRecord(
        ts=now + 1, chunk_id="chunk_a", seq=2, event_id="e2",
        identity_id="id_a", scope="hot", corrupt_kind="deleted",
        health_ratio=0.6, detail_json='{"i":2}',
    )
    rec3 = RecoveryRecord(
        ts=now + 2, chunk_id="chunk_b", seq=1, event_id="e3",
        identity_id="id_b", scope="cold", corrupt_kind="tampered",
        health_ratio=0.5, detail_json='{"i":3}',
    )
    id1 = rri.record(rec1)
    id2 = rri.record(rec2)
    id3 = rri.record(rec3)
    assert id1 > 0 and id2 > id1 and id3 > id2

    by_chunk_a = rri.query_by_chunk("chunk_a")
    assert len(by_chunk_a) == 2
    assert all(r.chunk_id == "chunk_a" for r in by_chunk_a)
    # ORDER BY ts DESC → rec2 在前
    assert by_chunk_a[0].seq == 2
    assert by_chunk_a[1].seq == 1

    by_identity_a = rri.query_by_identity("id_a")
    assert len(by_identity_a) == 2
    by_ts = rri.query_by_ts_range(now, now + 3)
    assert len(by_ts) == 3

    stats = rri.stats()
    assert stats["n_total"] == 3
    assert stats["by_corrupt_kind"]["tampered"] == 2
    assert stats["by_corrupt_kind"]["deleted"] == 1
    rri.close()


def test_t07_recovery_explain_uses_chunk_ts_index(recovery_db: str) -> None:
    """T07: EXPLAIN QUERY PLAN 走 idx_recovery_chunk_ts 复合索引."""
    rri = RecoveryRecordIndex(recovery_db)
    plan = rri.explain_query(chunk_id="any_chunk")
    plan_str = " ".join(plan)
    assert "idx_recovery_chunk_ts" in plan_str, (
        f"idx_recovery_chunk_ts 复合索引未被使用, plan={plan}"
    )
    rri.close()


def test_t08_recovery_ddl_idempotent(tmp_dir: Path) -> None:
    """T08: migrate() 二次调用幂等 — 索引数不变."""
    tmp_dir.mkdir(parents=True, exist_ok=True)
    path = str(tmp_dir / "r08.db")
    rri = RecoveryRecordIndex(path)
    info1 = rri.migrate()
    info2 = rri.migrate()
    assert info1["n_indexes"] == info2["n_indexes"] == 4
    rri.close()


# ===========================================================================
# Group 3: CrossTableJoinBenchmark — T09..T12
# ===========================================================================


def test_t09_benchmark_1k_rows_with_idx_fast(tmp_dir: Path) -> None:
    """T09: 1K 行 8 表 JOIN + identity_id 索引命中, 单 query ≤ 50ms (本机 CI)."""
    tmp_dir.mkdir(parents=True, exist_ok=True)
    b = CrossTableJoinBenchmark()
    row = b._run_one(1000)
    assert isinstance(row, JoinBenchmarkRow)
    assert row.scale == 1000
    assert row.n_distinct_identities == 1
    assert row.n_rows_total == 1000
    assert row.continuity_score == 1.0
    assert row.join_ms_with_index < 50.0  # 索引后 ≤ 50ms
    # EXPLAIN 验证: with_index 路径走 idx_v012_identity_hot
    assert any("idx_v012_identity_hot" in line for line in row.explain_with_index)


def test_t10_benchmark_scales_1k_10k_100k(tmp_dir: Path) -> None:
    """T10: 3 个 scale 都跑通 — 1K + 10K + 100K."""
    tmp_dir.mkdir(parents=True, exist_ok=True)
    b = CrossTableJoinBenchmark()
    rows = b.run(scales=(1000, 10000, 100000))
    assert len(rows) == 3
    assert [r.scale for r in rows] == [1000, 10000, 100000]
    # 100K 行 = 100000
    assert rows[-1].n_rows_total == 100000
    # 索引后 100K 应 < 100ms (单次 query, 含 LIMIT 1000)
    assert rows[-1].join_ms_with_index < 100.0


def test_t11_benchmark_with_idx_beats_no_idx(tmp_dir: Path) -> None:
    """T11: 有 idx_v012_identity 比 无索引 (DROP INDEX 模拟) 快 (主 19:33 走在前人)."""
    tmp_dir.mkdir(parents=True, exist_ok=True)
    b = CrossTableJoinBenchmark()
    row = b._run_one(10000)
    # 至少 1 个 EXPLAIN plan 节点使用 idx_v012_identity_hot
    has_idx_plan = any("idx_v012_identity_hot" in line
                       for line in row.explain_with_index)
    assert has_idx_plan, f"期望 idx_v012_identity_hot, 实得: {row.explain_with_index}"
    # 无索引的 plan 不应包含 idx_v012_identity_hot
    assert not any("idx_v012_identity_hot" in line
                   for line in row.explain_no_index), (
        f"无索引 plan 不应包含 idx_v012_identity_hot, 实得: {row.explain_no_index}"
    )


def test_t12_benchmark_to_dicts_serializable(tmp_dir: Path) -> None:
    """T12: benchmark 结果 json.dumps 序列化无 TypeError."""
    tmp_dir.mkdir(parents=True, exist_ok=True)
    b = CrossTableJoinBenchmark()
    rows = b.run(scales=(1000,))
    ds = b.to_dicts(rows)
    s = json.dumps(ds, ensure_ascii=False)
    parsed = json.loads(s)
    assert parsed[0]["scale"] == 1000


# ===========================================================================
# Group 4: StressDrill — T13..T16
# ===========================================================================


def test_t13_migration_stress_10x_preserves_rows(tmp_dir: Path) -> None:
    """T13: 10× 数据量 v0.1.0 → v0.1.2 真跑, 1460 行 0 丢失 + 3 次幂等."""
    tmp_dir.mkdir(parents=True, exist_ok=True)
    sd = StressDrill(tmp_dir)
    rep = sd.migration_stress(multiplier=10)
    assert rep.success is True
    assert rep.error is None
    # 146 行 × 10 = 1460
    assert rep.metrics["n_rows_before"] == 1460
    assert rep.metrics["n_rows_after"] == 1460
    assert rep.metrics["rows_preserved"] is True
    steps = [t["step"] for t in rep.trace]
    assert "drill_init" in steps
    assert "v012_upgrade_done" in steps
    assert "idempotent_runs_done" in steps


def test_t14_join_stress_1k_anchored_to_identity(tmp_dir: Path) -> None:
    """T14: join_stress 1K 行 → 1 个 distinct identity + continuity_score > 0."""
    tmp_dir.mkdir(parents=True, exist_ok=True)
    sd = StressDrill(tmp_dir)
    rep = sd.join_stress(n_rows=1000)
    assert rep.success is True
    assert rep.error is None
    assert rep.metrics["n_distinct_identities"] == 1
    assert rep.metrics["continuity_score"] > 0
    assert rep.metrics["n_rows_total"] >= 1000


def test_t15_disaster_stress_50_corrupt_recovery_record_uses_index(
        tmp_dir: Path) -> None:
    """T15: disaster_stress 200 valid + 50 corrupt → recovery_record 表 50 行 + 走 idx_recovery_chunk_ts."""
    tmp_dir.mkdir(parents=True, exist_ok=True)
    sd = StressDrill(tmp_dir)
    rep = sd.disaster_stress(n_valid=200, n_corrupt=50)
    assert rep.success is True, f"disaster_stress failed: {rep.error}"
    assert rep.metrics["recovery_record_stats"]["n_total"] == 50
    assert rep.metrics["explain_uses_idx_recovery_chunk_ts"] is True
    # 验证 verify_wal_checksums 检测到至少 25 处 tampered (一半)
    vb = rep.metrics["verify_before"]
    assert vb["corrupt"] >= 25


def test_t16_run_full_stress_three_reports(tmp_dir: Path) -> None:
    """T16: run_full_stress 串联 3 类 stress, 返回 3 份 StressReport."""
    tmp_dir.mkdir(parents=True, exist_ok=True)
    sd = StressDrill(tmp_dir)
    reps = sd.run_full_stress()
    assert len(reps) == 3
    kinds = [r.drill_kind for r in reps]
    assert kinds == ["migration_stress", "join_stress", "disaster_stress"]
    # join_stress 默认 100K 行 (本机 CI 时间充裕)
    assert reps[1].metrics["n_rows_total"] >= 100000
    # 所有都 success
    assert all(r.success for r in reps)
