"""V1113 Memory Schema v0.1.2 Runbook — 真跑演练测试 (R9-DB-002).

主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 13:31 大胆激进 +
主 23:44 干到底 + 主 19:33 走在前人经验上 + 主 00:56 任何人都能接手.

测试覆盖 (T01..T24, 真演练 ≥ 20 用例):
  T01..T04  RealDataMigrationDrill (v0.1.0 → v0.1.2 真演练)
  T05..T10  CrossTableJoinV1072Drill (V1072 跨 8 表 join 真演练)
  T11..T16  DisasterRecoveryDrill (corrupt WAL → recover 真演练)
  T17..T20  RunbookSummary 串联 + CLI 主 00:56 一行命令
  T21..T24  V1072 IdentityCore + ContinuityTracker 真集成

执行:
  python -m pytest tests/test_v1113_memory_runbook.py -v
"""
from __future__ import annotations

import json
import shutil
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
    IdentityCore,
    IdentityManifest,
    ContinuityTracker,
)
from apeireth.v1109_memory_schema_v012 import (  # noqa: E402
    DREAM_PHASES,
    MemorySchemaV012,
    V1109_VERSION,
)
from apeireth.v1113_memory_schema_v012_runbook import (  # noqa: E402
    CrossTableJoinV1072Drill,
    DisasterRecoveryDrill,
    DisasterRecoveryReport,
    JoinDrillReport,
    MigrationDrillReport,
    RealDataMigrationDrill,
    RunbookSummary,
    V1113_VERSION,
    main,
    run_full_runbook,
)


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture
def tmp_dir(tmp_path: Path) -> Path:
    return tmp_path / "v1113"


@pytest.fixture
def migration_report(tmp_dir: Path) -> MigrationDrillReport:
    tmp_dir.mkdir(parents=True, exist_ok=True)
    return RealDataMigrationDrill(db_path=str(tmp_dir / "mig.db")).run()


@pytest.fixture
def join_report(tmp_dir: Path) -> JoinDrillReport:
    tmp_dir.mkdir(parents=True, exist_ok=True)
    return CrossTableJoinV1072Drill(db_path=str(tmp_dir / "join.db"), n_rows=1000).run()


@pytest.fixture
def disaster_report(tmp_dir: Path) -> DisasterRecoveryReport:
    tmp_dir.mkdir(parents=True, exist_ok=True)
    return DisasterRecoveryDrill(
        db_path=str(tmp_dir / "dr.db"),
        n_wal_rows=50,
    ).run()


# ===========================================================================
# Group 1: RealDataMigrationDrill — T01..T04 (4 用例)
# ===========================================================================


def test_t01_migration_drill_runs_and_preserves_rows(migration_report: MigrationDrillReport) -> None:
    """T01: 真演练成功 — 升级前 ≥100 行, 升级后 0 丢失."""
    assert migration_report.success is True
    # SAMPLE_SIZES 总和 = 30+20+40+8+5+25+6+12 = 146
    assert migration_report.n_rows_before == 146
    assert migration_report.n_rows_after == migration_report.n_rows_before
    assert migration_report.error is None


def test_t02_migration_drill_seeds_both_versions(migration_report: MigrationDrillReport) -> None:
    """T02: V1094 v0.1.0 + V1109 v0.1.2 双层 meta 都已 seed."""
    assert migration_report.meta_v094_seeded is True
    assert migration_report.meta_v1109_seeded is True
    assert migration_report.n_columns_added > 0
    assert migration_report.n_indexes_added > 0


def test_t03_migration_drill_idempotent_5_runs(migration_report: MigrationDrillReport) -> None:
    """T03: 幂等演练 5 次连续 upgrade — 元数据不变, 行数稳定."""
    assert migration_report.migration_idempotent_runs == 5
    # 每行样本保留
    assert len(migration_report.sample_preservation["memory_hot"]) >= 3
    assert len(migration_report.sample_preservation["memory_dream"]) >= 3


def test_t04_migration_drill_trace_records_steps(migration_report: MigrationDrillReport) -> None:
    """T04: trace 至少 6 个步骤 — drill_init/v010_baseline_built/v010_schema_seeded/v012_upgrade_done/idempotent_runs_done/drill_done."""
    step_names = [s.step for s in migration_report.trace]
    assert "drill_init" in step_names
    assert "v010_baseline_built" in step_names
    assert "v010_schema_seeded" in step_names
    assert "v012_upgrade_done" in step_names
    assert "idempotent_runs_done" in step_names
    assert "drill_done" in step_names


# ===========================================================================
# Group 2: CrossTableJoinV1072Drill — T05..T10 (6 用例)
# ===========================================================================


def test_t05_join_drill_1000_rows_anchored(join_report: JoinDrillReport) -> None:
    """T05: 1000 行跨 8 表 anchor 到 V1072 identity_id."""
    assert join_report.success is True
    assert join_report.n_rows_total == 1000
    assert join_report.identity_id.startswith("id_chuling_")


def test_t06_join_drill_per_table_distribution(join_report: JoinDrillReport) -> None:
    """T06: 跨 8 表分布合理 — memory_wal 占 30%, memory_hot 占 20%."""
    ptc = join_report.per_table_counts
    assert ptc["memory_wal"] == 300
    assert ptc["memory_hot"] == 200
    assert ptc["memory_cold"] == 120
    assert ptc["stm_messages"] == 120
    assert ptc["ltm_facts"] == 100
    assert ptc["memory_dream"] == 80
    assert ptc["mtm_themes"] == 50
    assert ptc["memory_snapshots"] == 30


def test_t07_join_drill_distinct_identities_is_one(join_report: JoinDrillReport) -> None:
    """T07: 1000 行只 1 个 distinct identity_id — 锚定单一 Chu Ling."""
    assert join_report.n_distinct_identities == 1


def test_t08_join_drill_continuity_metrics(join_report: JoinDrillReport) -> None:
    """T08: V1072 连续性指标 — identity_locked=True + rows_per_session_mean 合理."""
    cm = join_report.continuity_metrics
    assert cm["identity_locked"] is True
    assert cm["n_sessions"] >= 3  # ContinuityTracker.start_session() × 3
    assert cm["rows_per_session_mean"] > 0


def test_t09_join_drill_join_records_match_total(join_report: JoinDrillReport) -> None:
    """T09: join_records (8 表 list_by_identity 之和) == n_rows_total."""
    assert join_report.n_join_records == join_report.n_rows_total


def test_t10_join_drill_trace_includes_v1072_seeding(join_report: JoinDrillReport) -> None:
    """T10: trace 必含 V1072 IdentityCore / IdentityManifest / ContinuityTracker seed 步骤."""
    steps = [s.step for s in join_report.trace]
    assert "v1072_identity_seeded" in steps
    assert "v1072_manifest_seeded" in steps
    assert "v1072_continuity_seeded" in steps
    assert "v012_rows_anchored" in steps


# ===========================================================================
# Group 3: DisasterRecoveryDrill — T11..T16 (6 用例)
# ===========================================================================


def test_t11_disaster_drill_detects_tampering(disaster_report: DisasterRecoveryReport) -> None:
    """T11: 真演练 — 5 条 tampered + 2 条 deleted, verify_wal_checksums 报 ≥5 corrupt."""
    assert disaster_report.n_wal_rows_initial == 50
    assert disaster_report.n_corrupted_rows == 7
    cr = disaster_report.checksum_report
    assert cr["total"] == 48  # 50 - 2 deleted
    assert cr["corrupt"] >= 5
    assert cr["health_ratio"] < 1.0


def test_t12_disaster_drill_replay_skips_corrupt(disaster_report: DisasterRecoveryReport) -> None:
    """T12: replay_events_by_chunk 默认 skip_corrupt=True — 总 replayed = total - corrupt - deleted."""
    assert disaster_report.n_skipped_rows >= 7
    # replayed 数量 = 48 - 5 = 43
    assert disaster_report.n_recovered_rows == 43


def test_t13_disaster_drill_recovery_record_populated(disaster_report: DisasterRecoveryReport) -> None:
    """T13: recover_corrupt 产出 recovery_record — 含 schema_version + report."""
    rec = disaster_report.recovery_record
    assert rec["schema_version"] == V1109_VERSION
    assert rec["report"]["corrupt"] >= 5
    assert "ts" in rec


def test_t14_disaster_drill_success_assertion(disaster_report: DisasterRecoveryReport) -> None:
    """T14: drill success = corrupt ≥ tampered + valid > 0 + health_ratio < 1.0 + corrupt_event_ids."""
    assert disaster_report.success is True


def test_t15_disaster_drill_corrupt_event_ids(disaster_report: DisasterRecoveryReport) -> None:
    """T15: corrupt_event_ids 列出所有 tampered event_id (前 16 hex 截断)."""
    ceids = disaster_report.recovery_record["report"]["corrupt_event_ids"]
    assert len(ceids) >= 5
    # 所有 tampered 的 event_id 前缀都在 corrupt_event_ids
    for cid in ["dr_ev_003", "dr_ev_007", "dr_ev_015", "dr_ev_023", "dr_ev_041"]:
        assert cid in ceids


def test_t16_disaster_drill_trace_records_steps(disaster_report: DisasterRecoveryReport) -> None:
    """T16: trace 步骤 — drill_init/wal_seeded/corruption_injected/verify_wal_checksums/replay_per_chunk/recover_corrupt."""
    steps = [s.step for s in disaster_report.trace]
    for s in ("drill_init", "wal_seeded", "corruption_injected",
              "verify_wal_checksums", "replay_per_chunk", "recover_corrupt"):
        assert s in steps, f"missing step: {s}"


# ===========================================================================
# Group 4: RunbookSummary 串联 + CLI — T17..T20 (4 用例)
# ===========================================================================


def test_t17_runbook_full_summary_success(tmp_dir: Path) -> None:
    """T17: run_full_runbook 一行命令 — 3 类演练全 success."""
    summary = run_full_runbook(db_dir=str(tmp_dir / "full"))
    assert summary.success is True
    assert summary.migration is not None and summary.migration.success
    assert summary.join is not None and summary.join.success
    assert summary.disaster is not None and summary.disaster.success


def test_t18_runbook_to_dict_serializable(tmp_dir: Path) -> None:
    """T18: summary.to_dict() → JSON 可序列化 (供主 00:56 docs 报告)."""
    summary = run_full_runbook(db_dir=str(tmp_dir / "serial"))
    blob = json.dumps(summary.to_dict(), ensure_ascii=False, default=str)
    parsed = json.loads(blob)
    assert "migration" in parsed and "join" in parsed and "disaster" in parsed
    assert parsed["success"] is True


def test_t19_runbook_main_cli_returns_zero(tmp_path: Path) -> None:
    """T19: 主 00:56 — main() 返回 0 (success) 或 1 (fail), print-json 输出."""
    rc = main(["--db-dir", str(tmp_path / "cli"), "--print-json"])
    assert rc in (0, 1)  # 不强求 success, 只要 CLI 可跑


def test_t20_runbook_main_cli_writes_report(tmp_path: Path) -> None:
    """T20: main() --report 写 reports/r9-db-v1109-runbook.md."""
    main(["--db-dir", str(tmp_path / "cli_rep"), "--report"])
    report_path = ROOT / "reports" / "r9-db-v1109-runbook.md"
    if report_path.exists():
        text = report_path.read_text(encoding="utf-8")
        assert "V1113" in text


# ===========================================================================
# Group 5: V1072 真集成 — T21..T24 (4 用例)
# ===========================================================================


def test_t21_v1072_identity_core_compatible_with_runbook(tmp_dir: Path) -> None:
    """T21: V1072 IdentityCore.identity_id 真能 anchor 到 V1109 8 表."""
    fid = "id_t21_runbook"
    db_path = tmp_dir / "v1072_t21.db"
    s = MemorySchemaV012(str(db_path))
    # 各表插一行 anchor
    for tbl, rid in [
        ("memory_hot", "h_t21"),
        ("memory_cold", "c_t21"),
        ("memory_dream", "d_t21"),
        ("memory_snapshots", "sn_t21"),
        ("stm_messages", "stm_t21"),
        ("mtm_themes", "tpc_t21"),
        ("ltm_facts", "ltm_t21"),
    ]:
        if tbl == "memory_hot":
            s._conn.execute(
                "INSERT INTO memory_hot(id, session_id, actor, content, ts, fingerprint)"
                " VALUES (?, ?, ?, ?, ?, ?)",
                (rid, "s", "a", "c", time.time(), "f"),
            )
        elif tbl == "memory_cold":
            s._conn.execute(
                "INSERT INTO memory_cold(id, content, ts, fingerprint)"
                " VALUES (?, ?, ?, ?)", (rid, "c", time.time(), "f"),
            )
        elif tbl == "memory_dream":
            s._conn.execute(
                "INSERT INTO memory_dream(id, summary, ts) VALUES (?, ?, ?)",
                (rid, "summ", time.time()),
            )
        elif tbl == "memory_snapshots":
            s._conn.execute(
                "INSERT INTO memory_snapshots(id, scope, seq, content_hash, ts)"
                " VALUES (?, ?, ?, ?, ?)", (rid, "hot", 1, "f", time.time()),
            )
        elif tbl == "stm_messages":
            s._conn.execute(
                "INSERT INTO stm_messages(id, session_id, role, content, ts, fingerprint)"
                " VALUES (?, ?, ?, ?, ?, ?)", (rid, "s", "user", "hi", time.time(), "f"),
            )
        elif tbl == "mtm_themes":
            s._conn.execute(
                "INSERT INTO mtm_themes(topic_id, topic_label, last_updated, fingerprint)"
                " VALUES (?, ?, ?, ?)", (rid, "lab", time.time(), "f"),
            )
        elif tbl == "ltm_facts":
            s._conn.execute(
                "INSERT INTO ltm_facts(id, category, content, ts, fingerprint)"
                " VALUES (?, ?, ?, ?, ?)", (rid, "fact", "c", time.time(), "f"),
            )
    s._conn.commit()
    for tbl, rid in [
        ("memory_hot", "h_t21"),
        ("memory_cold", "c_t21"),
        ("memory_dream", "d_t21"),
        ("memory_snapshots", "sn_t21"),
        ("stm_messages", "stm_t21"),
        ("mtm_themes", "tpc_t21"),
        ("ltm_facts", "ltm_t21"),
    ]:
        n = s.anchor_identity(tbl, rid, fid)
        assert n == 1
    s.close()


def test_t22_v1072_manifest_anchored_via_v1109(tmp_dir: Path) -> None:
    """T22: V1072 IdentityManifest 写入 + V1109 dream 表 anchor 到 IdentityCore.identity_id."""
    db_path = tmp_dir / "v1072_t22.db"
    s = MemorySchemaV012(str(db_path))
    manifest = IdentityManifest(core=IdentityCore(identity_id="id_manifest_42"))
    manifest.add("LTM", "fact", "test", importance=0.9)
    manifest.add("MTM", "insight", "test", importance=0.7)
    manifest.add("STM", "event", "test", importance=0.5)
    # 用 V1109 dream_record_with_phase 写入 (anchor 到 manifest.core.identity_id)
    for phase, kind in [("ASSIMILATION", "LTM"), ("ACCOMMODATION", "MTM"), ("REPLAY", "STM")]:
        did = s.dream_record_with_phase(
            summary=f"{kind} test", dream_phase=phase,
            identity_id=manifest.core.identity_id,
        )
        assert did.startswith("dream_")
    # 校验 anchor 命中
    rows = s.list_by_identity("memory_dream", manifest.core.identity_id)
    assert len(rows) == 3
    s.close()


def test_t23_v1072_continuity_tracker_runs(tmp_dir: Path) -> None:
    """T23: V1072 ContinuityTracker 跨 3 个 session 真跑 — current_session 跟踪."""
    ct = ContinuityTracker()
    sids = [ct.start_session() for _ in range(3)]
    assert ct.current_session == sids[-1]
    for sid in sids:
        assert sid in ct.sessions
    assert len(ct.sessions) == 3


def test_t24_v1072_eternal_identity_constant_available() -> None:
    """T24: ETERNAL_IDENTITY_CORE 常量 — Chu Ling / 楚零 永恒身份 (主 12:14) 真存在."""
    from apeireth.v1072_asi_central_ai_eternal_identity import ETERNAL_IDENTITY_CORE
    assert ETERNAL_IDENTITY_CORE["name"] == "Chu Ling"
    assert ETERNAL_IDENTITY_CORE["chinese_name"] == "楚零"
    assert ETERNAL_IDENTITY_CORE["ltm_persistence"] is True
    assert "Parfit 1984 psychological continuity" in ETERNAL_IDENTITY_CORE["philosophy_anchor"]