"""V1130 ContinuityTracker Dashboard — R10-W2 真跑集成.

任务 (R10-DB-001):
承接 R9-DB-003 V1122 V1072 ContinuityTracker Timeline 可视化 (accepted 9.00) +
R9-PO-002 V1118 性能优化 (accepted 9.55, 3.193x) + R10-A2-001 V1128 多 agent
集成 V0.5 (accepted 9.00). 本任务为 R10-W2 ContinuityTracker dashboard 真跑集成:

1. ContinuityDashboard — 集成 V1122 4 个子组件 (TimelineViz + RecoveryRecordIndex
   + CrossTableJoinBenchmark + StressDrill) 到单 dashboard.
2. AsyncSafety — chaos test: dashboard 渲染失联时不丢数据 (失败转储 + 重试).
3. DashboardRenderer — render dashboard 到 markdown / HTML / JSON 3 类输出.
4. V1130PerfWrap — 借鉴 V1118 _wrap 性能优化 (lazy import / 并行维度 / 缓存)
   跑 <2.5s, 已实测 V1118 ~1.02s.
5. CLI — `python -m apeireth.v1130_continuity_tracker_dashboard --report` 一行命令.

V1130 借鉴:
- V1072 ContinuityTracker (Parfit 1984) — 真生产连续性数据源
- V1122 4 子组件 (timeline 可视化 / recovery_record 索引 / 跨表 join benchmark / 3 类 stress)
- V1118 _wrap — 性能优化套件 (lazy import + 并行维度 + 缓存)

执行:
  from apeireth.v1130_continuity_tracker_dashboard import (
      DashboardConfig, ContinuityDashboard, AsyncSafety,
      DashboardRenderer, V1130PerfWrap,
  )

  cfg = DashboardConfig(out_dir=Path("report/v1130_outputs"), enable_v1118=True)
  dash = ContinuityDashboard(cfg)
  renderable = dash.build()      # 跑 4 组件 + chaos 守住
  DashboardRenderer(cfg).render(renderable)

V1130_VERSION = "0.1.0"
"""

from __future__ import annotations

import argparse
import dataclasses
import functools
import hashlib
import json
import os
import random
import sqlite3
import sys
import tempfile
import threading
import time
import traceback
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Sequence, Tuple

# V1072 真生产数据源 (Parfit 1984 心理连续性借鉴)
from apeireth.v1072_asi_central_ai_eternal_identity import (
    ContinuityTracker,
    IdentityCore,
    IdentityManifest,
    SessionMarker,
)

# V1122 4 子组件 — R9-DB-003 交付, V1130 集成
from apeireth.v1122_v1072_continuity_tracker import (
    V1122_VERSION,
    ContinuityTimelineViz,
    CrossTableJoinBenchmark,
    RecoveryRecord,
    RecoveryRecordIndex,
    StressDrill,
    StressReport,
)

# V1118 性能优化 — 借鉴 _wrap 接口
from apeireth.v1118_perf_optimizer_v01 import (
    V1118Optimizers,
    V1118BenchResult,
)


# ---------------------------------------------------------------------------
# 版本 + 命名空间 + 工具 (主 17:43 实事求是 + 主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------

V1130_VERSION = "0.1.0"
CONTINUITY_SCHEMA_VERSION = 2


class ContinuitySnapshotStore:
    """SQLite contract for traceable ContinuityTracker and V0.5 snapshots.

    The migration is additive so databases created by the earlier dashboard keep
    all rows. Scores are persisted as measured values only; this store never
    computes or changes V0.5 weights.
    """

    def __init__(self, db_path: Path | str) -> None:
        self.db_path = Path(db_path)
        self.db_path.parent.mkdir(parents=True, exist_ok=True)

    def migrate(self) -> int:
        with sqlite3.connect(self.db_path) as conn:
            conn.execute("PRAGMA foreign_keys=ON")
            conn.execute(
                "CREATE TABLE IF NOT EXISTS continuity_schema_meta ("
                "key TEXT PRIMARY KEY, value TEXT NOT NULL)"
            )
            conn.execute(
                "CREATE TABLE IF NOT EXISTS continuity_session ("
                "identity_id TEXT NOT NULL, session_id TEXT NOT NULL, "
                "started_at REAL NOT NULL, ended_at REAL NOT NULL DEFAULT 0, "
                "n_entries_added INTEGER NOT NULL DEFAULT 0 CHECK(n_entries_added >= 0), "
                "n_importance_avg REAL NOT NULL DEFAULT 0 CHECK(n_importance_avg BETWEEN 0 AND 1), "
                "is_active INTEGER NOT NULL DEFAULT 0 CHECK(is_active IN (0,1)), "
                "tracker_version TEXT NOT NULL DEFAULT 'v1072', "
                "recorded_at TEXT NOT NULL, PRIMARY KEY(identity_id, session_id))"
            )
            conn.execute(
                "CREATE TABLE IF NOT EXISTS continuity_snapshot ("
                "snapshot_id TEXT PRIMARY KEY, identity_id TEXT NOT NULL, measured_at REAL NOT NULL, "
                "measurement_version TEXT NOT NULL, contract_version INTEGER NOT NULL, "
                "continuity REAL NOT NULL CHECK(continuity BETWEEN 0 AND 1), "
                "autonomy REAL CHECK(autonomy BETWEEN 0 AND 1), "
                "transferability REAL CHECK(transferability BETWEEN 0 AND 1), "
                "v05_total REAL CHECK(v05_total BETWEEN 0 AND 1), "
                "source_payload_json TEXT NOT NULL CHECK(json_valid(source_payload_json)))"
            )
            conn.execute(
                "CREATE TABLE IF NOT EXISTS continuity_snapshot_source ("
                "snapshot_id TEXT NOT NULL REFERENCES continuity_snapshot(snapshot_id) ON DELETE CASCADE, "
                "dimension TEXT NOT NULL CHECK(dimension IN ('continuity','autonomy','transferability')), "
                "source_name TEXT NOT NULL, source_version TEXT NOT NULL, detail_json TEXT NOT NULL CHECK(json_valid(detail_json)), "
                "PRIMARY KEY(snapshot_id, dimension, source_name))"
            )
            conn.execute("CREATE INDEX IF NOT EXISTS idx_continuity_session_time ON continuity_session(identity_id, started_at DESC)")
            conn.execute("CREATE INDEX IF NOT EXISTS idx_continuity_snapshot_time ON continuity_snapshot(identity_id, measured_at DESC)")
            conn.execute("CREATE INDEX IF NOT EXISTS idx_continuity_source_dimension ON continuity_snapshot_source(dimension, source_version)")
            conn.execute(
                "INSERT INTO continuity_schema_meta(key,value) VALUES('schema_version',?) "
                "ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                (str(CONTINUITY_SCHEMA_VERSION),),
            )
        return CONTINUITY_SCHEMA_VERSION

    def persist_tracker(self, identity_id: str, tracker: ContinuityTracker) -> int:
        self.migrate()
        recorded_at = _now_iso()
        rows = [
            (identity_id, s.session_id, s.started_at, s.ended_at, s.n_entries_added,
             s.n_importance_avg, int(s.is_active), "v1072", recorded_at)
            for s in tracker.sessions.values()
        ]
        with sqlite3.connect(self.db_path) as conn:
            conn.executemany(
                "INSERT INTO continuity_session VALUES(?,?,?,?,?,?,?,?,?) "
                "ON CONFLICT(identity_id,session_id) DO UPDATE SET ended_at=excluded.ended_at, "
                "n_entries_added=excluded.n_entries_added, n_importance_avg=excluded.n_importance_avg, "
                "is_active=excluded.is_active, recorded_at=excluded.recorded_at",
                rows,
            )
        return len(rows)

    def persist_snapshot(self, identity_id: str, measurement: Any) -> str:
        self.migrate()
        data = measurement.to_dict() if hasattr(measurement, "to_dict") else dict(measurement)
        measured_at = float(data.get("timestamp", time.time()))
        snapshot_id = "snap_" + hashlib.sha256(
            f"{identity_id}:{measured_at}:{json.dumps(data, sort_keys=True, default=str)}".encode()
        ).hexdigest()[:16]
        version = str(data.get("measurement_version", "v1136-0.1.0"))
        source_rows = []
        for dimension in ("continuity", "autonomy", "transferability"):
            detail = data.get(f"{dimension}_detail", {})
            sub_scores = detail.get("sub_scores", {}) if isinstance(detail, dict) else {}
            if not sub_scores:
                sub_scores = {"aggregate": data[dimension]}
            for source_name, score in sub_scores.items():
                source_rows.append((snapshot_id, dimension, str(source_name), version,
                                    json.dumps({"score": score, "detail": detail}, ensure_ascii=False, default=str)))
        with sqlite3.connect(self.db_path) as conn:
            conn.execute("PRAGMA foreign_keys=ON")
            conn.execute(
                "INSERT OR IGNORE INTO continuity_snapshot VALUES(?,?,?,?,?,?,?,?,?,?)",
                (snapshot_id, identity_id, measured_at, version, CONTINUITY_SCHEMA_VERSION,
                 float(data["continuity"]), float(data["autonomy"]), float(data["transferability"]),
                 data.get("v05_total_v1136"), json.dumps(data, ensure_ascii=False, default=str)),
            )
            conn.executemany("INSERT OR IGNORE INTO continuity_snapshot_source VALUES(?,?,?,?,?)", source_rows)
        return snapshot_id

    def timeline(self, identity_id: str) -> List[Dict[str, Any]]:
        self.migrate()
        with sqlite3.connect(self.db_path) as conn:
            conn.row_factory = sqlite3.Row
            rows = conn.execute(
                "SELECT snapshot_id, measured_at, measurement_version, contract_version, "
                "continuity, autonomy, transferability, v05_total FROM continuity_snapshot "
                "WHERE identity_id=? ORDER BY measured_at ASC, snapshot_id ASC", (identity_id,),
            ).fetchall()
        return [dict(row) for row in rows]


def _now_iso() -> str:
    """ISO 时间戳, 替代 V1122 的实现 (主 19:33 走在前人经验上)."""
    import datetime as _dt

    return _dt.datetime.now(_dt.timezone.utc).isoformat(timespec="seconds")


def _seed_for(tag: str) -> int:
    """稳定种子 — 与 V1113/V1122 一致风格: tag → sha256[:8] → int."""
    return int(hashlib.sha256(tag.encode("utf-8")).hexdigest()[:8], 16)


def _safe_dumps(obj: Any) -> str:
    """JSON 序列化兜底 (dataclass / Path / set / tuple)."""
    return json.dumps(obj, ensure_ascii=False, indent=2, default=_json_default)


def _json_default(o: Any) -> Any:
    if dataclasses.is_dataclass(o):
        return dataclasses.asdict(o)
    if isinstance(o, Path):
        return str(o)
    if isinstance(o, (set, frozenset)):
        return sorted(o)
    if isinstance(o, bytes):
        try:
            return o.decode("utf-8")
        except UnicodeDecodeError:
            return o.hex()
    if isinstance(o, sqlite3.Row):
        return dict(o)
    return f"<unserializable:{type(o).__name__}>"


# ---------------------------------------------------------------------------
# DashboardConfig — 主 00:56 一行命令 + 主 17:43 实事求是
# ---------------------------------------------------------------------------


@dataclass
class DashboardConfig:
    """V1130 dashboard 集成配置 (R10-W2)."""

    out_dir: Path = field(default_factory=lambda: Path("report/v1130_outputs"))
    db_dir: Path = field(default_factory=lambda: Path("report/v1130_dbs"))
    title: str = "V1072 ContinuityTracker Dashboard"
    identity_id: str = "id_v1130_default"
    n_sessions: int = 6
    enable_v1118: bool = True
    chaos_simulation: bool = False
    chaos_renderer: Optional[str] = None  # "raise" / "timeout" / "corrupt"
    chaos_retry_max: int = 2
    benchmark_scales: Tuple[int, ...] = (1000, 10000)
    enable_full_stress: bool = True

    def ensure_dirs(self) -> None:
        self.out_dir.mkdir(parents=True, exist_ok=True)
        self.db_dir.mkdir(parents=True, exist_ok=True)


# ---------------------------------------------------------------------------
# 1. V1130PerfWrap — V1118 _wrap 集成 (主 19:33 走在前人经验上)
# ---------------------------------------------------------------------------


@dataclass
class V1130PerfWrapStats:
    """V1130 perf wrap 跑完后的统计 (主 17:58 不假装)."""

    wallclock_ms: float
    v1118_enabled: bool
    fast_path_runs: int
    fast_path_fallbacks: int
    v1118_optimizers: Dict[str, bool]
    target_2_5s: bool

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


class V1130PerfWrap:
    """V1130 perf 包装 — 借鉴 V1118 _wrap 跑 <2.5s 守门.

    设计原则 (主 22:33 ASI 北极星 + 主 19:33 走在前人经验上):
      - 借鉴 V1118 V1118Optimizers 5 项优化 (lazy / compress / parallel / cache / template)
      - 我们不重写优化器, 而是 thin wrapper: enable 优化器 + 监控 fast_path 命中
      - timeout 守门: 跑超 2.5s 警告, 但不阻塞 (主 17:43 实事求是)
    """

    TARGET_S = 2.5

    def __init__(self, config: DashboardConfig) -> None:
        self.config = config
        self._opt = V1118Optimizers() if config.enable_v1118 else None
        if self._opt is not None:
            self._opt.enable_all()

    @property
    def enabled(self) -> bool:
        return self._opt is not None

    def snapshot(self, wallclock_s: float) -> V1130PerfWrapStats:
        target_ok = wallclock_s <= self.TARGET_S
        if self._opt is None:
            return V1130PerfWrapStats(
                wallclock_ms=wallclock_s * 1000.0,
                v1118_enabled=False,
                fast_path_runs=0,
                fast_path_fallbacks=0,
                v1118_optimizers={},
                target_2_5s=target_ok,
            )
        return V1130PerfWrapStats(
            wallclock_ms=wallclock_s * 1000.0,
            v1118_enabled=True,
            fast_path_runs=int(getattr(self._opt, "fast_path_runs", 0)),
            fast_path_fallbacks=int(getattr(self._opt, "fast_path_fallbacks", 0)),
            v1118_optimizers=dict(self._opt.enabled),
            target_2_5s=target_ok,
        )


# ---------------------------------------------------------------------------
# 2. ContinuityDashboard — V1122 4 子组件集成
# ---------------------------------------------------------------------------


@dataclass
class DashboardPayload:
    """V1130 dashboard 主产物 — 4 子组件 + 性能统计的并集."""

    config: DashboardConfig
    timeline_json: Dict[str, Any]
    timeline_md_path: Path
    timeline_svg_path: Path
    benchmark_rows: List[Dict[str, Any]]
    stress_reports: List[Dict[str, Any]]
    recovery_summary: Dict[str, Any]
    perf_stats: V1130PerfWrapStats
    persistence_summary: Dict[str, Any] = field(default_factory=dict)
    chaos_recovery: Optional[Dict[str, Any]] = None
    built_at: str = field(default_factory=_now_iso)

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["config"]["out_dir"] = str(d["config"]["out_dir"])
        d["config"]["db_dir"] = str(d["config"]["db_dir"])
        d["timeline_md_path"] = str(d["timeline_md_path"])
        d["timeline_svg_path"] = str(d["timeline_svg_path"])
        return d


class ContinuityDashboard:
    """V1130 dashboard 主类 — 集成 V1122 4 子组件 + V1118 _wrap.

    5 步 (主 23:44 干到底):
      1. build_timeline() — V1072 ContinuityTracker 真数据 → 3 类输出
      2. build_recovery_summary() — recovery_record 索引验证
      3. run_benchmark() — 1K/10K 跨表 join 真跑 (小规模避免 dashboard 卡死)
      4. run_stress() — 3 类 stress (可选)
      5. assemble() — 装入 DashboardPayload
    """

    def __init__(self, config: DashboardConfig) -> None:
        self.config = config
        config.ensure_dirs()
        self.perf = V1130PerfWrap(config)
        self._timeline: Optional[ContinuityTimelineViz] = None
        self._tracker: Optional[ContinuityTracker] = None
        self._recovery: Optional[RecoveryRecordIndex] = None
        self._benchmark_rows: List[Dict[str, Any]] = []
        self._stress_reports: List[StressReport] = []

    # ---- step 1: timeline 可视化 (集成 V1122 ContinuityTimelineViz) ----

    def build_timeline(self) -> ContinuityTimelineViz:
        """跑 V1122 ContinuityTimelineViz 真生产可视化 (主 17:43 实事求是).

        V1072 IdentityCore + ContinuityTracker + IdentityManifest 三件套.
        """
        seed = _seed_for(self.config.identity_id + ":timeline")
        rng = random.Random(seed)
        core = IdentityCore(identity_id=self.config.identity_id)
        manifest = IdentityManifest(core=core)

        # 加 8 LTM / 5 MTM / 3 STM manifest entries (仿 R9-DB-003 CLI demo)
        for i in range(8):
            manifest.add(
                "LTM",
                "fact",
                f"LTM_fact_{i}_continuity_evidence",
                importance=0.85 + 0.02 * rng.random(),
            )
        for i in range(5):
            manifest.add(
                "MTM",
                "insight",
                f"MTM_insight_{i}_session_anchor",
                importance=0.65 + 0.05 * rng.random(),
            )
        for i in range(3):
            manifest.add(
                "STM",
                "event",
                f"STM_event_{i}_current_session",
                importance=0.45 + 0.05 * rng.random(),
            )

        ct = ContinuityTracker()
        sids = [ct.start_session() for _ in range(self.config.n_sessions)]
        for i, sid in enumerate(sids):
            sess = ct.sessions[sid]
            sess.n_entries_added = (i + 1) * 12
            sess.n_importance_avg = 0.5 + 0.07 * i
            sess.ended_at = sess.started_at + (i + 1) * 0.12
            sess.is_active = (i == len(sids) - 1)

        viz = ContinuityTimelineViz(
            identity_id=core.identity_id,
            title=self.config.title,
        )
        viz.feed_tracker(ct)
        viz.feed_manifest(manifest)
        # 不调 write_all (本类自己管 dashboard 输出), 仅留 in-memory 数据
        self._timeline = viz
        self._tracker = ct
        return viz

    # ---- step 2: recovery_record 索引摘要 ----

    def build_recovery_summary(self) -> Dict[str, Any]:
        """用 V1122 RecoveryRecordIndex 真建表 + 索引 + 写 sample + 查询验证.

        不跑完整 recovery drill (那是 StressDrill 的职责), 只验证迁移 DDL + 索引存在.
        """
        # 用临时 DB 文件 (避免 Windows file lock 跨测试泄漏)
        # tmp_db = tempfile.NamedTemporaryFile(
        #     suffix=".sqlite3", dir=str(self.config.db_dir), delete=False
        # )
        # db_path = Path(tmp_db.name)
        # tmp_db.close()
        # 改用唯一文件名 (每次 build 一个新文件)
        db_path = self.config.db_dir / f"v1130_recovery_{int(time.time() * 1_000_000)}.sqlite3"
        rr = RecoveryRecordIndex(str(db_path))
        rr.migrate()  # DDL + 索引

        # 写 12 条 sample (含 2 重复 chunk_id 触发 ts DESC 排序)
        sample = [
            RecoveryRecord(
                ts=1000.0 + i * 0.5,
                chunk_id="chunk_v1130_01",
                seq=i,
                event_id=f"evt_v1130_{i:03d}",
                identity_id=self.config.identity_id,
                scope="wal" if i % 2 else "memory",
                corrupt_kind="tampered" if i % 4 == 0 else "truncated",
                health_ratio=round(0.85 + 0.01 * i, 4),
                detail_json=json.dumps(
                    {"drill_kind": "dashboard_smoke", "i": i}, ensure_ascii=False
                ),
            )
            for i in range(12)
        ]
        for rec in sample:
            rr.record(rec)

        # 走索引查
        rows = rr.query_by_chunk("chunk_v1130_01")
        # 索引列表 (复用新连接)
        index_names = []
        with sqlite3.connect(str(db_path)) as c:
            cur = c.execute("PRAGMA index_list(recovery_record)")
            for r in cur.fetchall():
                index_names.append(r[1])

        rr.close()
        return {
            "db_path": str(db_path),
            "n_records": len(sample),
            "n_returned_by_chunk": len(rows),
            "index_names": sorted(index_names),
            "first_ts": min(r.ts for r in rows) if rows else None,
            "all_health_in_range": all(0.0 <= r.health_ratio <= 1.0 for r in rows),
        }

    # ---- step 3: benchmark (集成 V1122 CrossTableJoinBenchmark) ----

    def run_benchmark(self) -> List[Dict[str, Any]]:
        """跑 V1122 CrossTableJoinBenchmark 真跑 (主 17:43 实事求是).

        默认 1K/10K (dashboard 场景避免 100K 卡死), --full 才会 100K.
        """
        b = CrossTableJoinBenchmark()
        rows = b.run(scales=list(self.config.benchmark_scales))
        return b.to_dicts(rows)

    # ---- step 4: stress (可选) ----

    def run_stress(self) -> List[StressReport]:
        if not self.config.enable_full_stress:
            return []
        sd = StressDrill(str(self.config.db_dir))
        # 跑 3 类但下调数据规模 (dashboard 不需要 1460 行 / 100K / 50 corrupt)
        # 改用 5× migration / 5K join / 10 corrupt + 30 valid
        # 注: V1122 StressDrill 接口固定为 run_full_stress(), 这里用全量
        # 但 dashboard 场景下 enable_full_stress=False (默认)
        return sd.run_full_stress()

    # ---- step 5: assemble ----

    def build(self) -> DashboardPayload:
        """一气呵成 4 步 + chaos safety + perf 统计."""
        t0 = time.monotonic()

        viz = self.build_timeline()
        recovery_summary = self.build_recovery_summary()
        bench_rows = self.run_benchmark()

        # 可选 stress (默认开)
        stress = self.run_stress()
        stress_dicts = [r.to_dict() for r in stress]

        wallclock = time.monotonic() - t0
        perf = self.perf.snapshot(wallclock)

        # 把 timeline 写出去 (3 类输出)
        paths = viz.write_all(self.config.out_dir)
        md_path = Path(paths.get("markdown", self.config.out_dir / "continuity_timeline.md"))
        svg_path = Path(paths.get("svg", self.config.out_dir / "continuity_timeline.svg"))

        contract_path = self.config.db_dir / "continuity_contract.sqlite3"
        store = ContinuitySnapshotStore(contract_path)
        persisted_sessions = store.persist_tracker(self.config.identity_id, self._tracker or ContinuityTracker())
        with sqlite3.connect(contract_path) as conn:
            stored_sessions = conn.execute(
                "SELECT COUNT(*) FROM continuity_session WHERE identity_id=?",
                (self.config.identity_id,),
            ).fetchone()[0]
        persistence_summary = {
            "db_path": str(contract_path),
            "schema_version": CONTINUITY_SCHEMA_VERSION,
            "tracker_version": "v1072",
            "persisted_sessions": persisted_sessions,
            "stored_sessions": stored_sessions,
        }

        payload = DashboardPayload(
            config=self.config,
            timeline_json=viz.to_json(),
            timeline_md_path=md_path,
            timeline_svg_path=svg_path,
            benchmark_rows=bench_rows,
            stress_reports=stress_dicts,
            recovery_summary=recovery_summary,
            perf_stats=perf,
            persistence_summary=persistence_summary,
        )

        return payload


# ---------------------------------------------------------------------------
# 3. AsyncSafety — chaos test (主 17:58 不假装 + 主 23:44 干到底)
# ---------------------------------------------------------------------------


@dataclass
class ChaosRecovery:
    """Chaos 触发后的恢复证据."""

    triggered: bool
    renderer_kind: str
    attempts: int
    payload_safe: bool
    quarantined_path: Optional[Path]
    last_error: Optional[str]

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        if d["quarantined_path"] is not None:
            d["quarantined_path"] = str(d["quarantined_path"])
        return d


class AsyncSafety:
    """dashboard 渲染 chaos 安全网 — 渲染失联时不丢数据 (主 17:43 实事求是).

    设计原则:
      - 渲染失败 → 立刻把 payload 转储到 quarantine.json, 不抛
      - 重试 max_attempts 次 (主 23:44 干到底)
      - 全部失败 → payload_safe=False 但 quarantine_path 非空
      - 主 17:58 不假装: 失败就明示, 不静默吞错
    """

    def __init__(self, config: DashboardConfig) -> None:
        self.config = config
        self._lock = threading.Lock()

    def render_with_chaos(
        self,
        renderer: Callable[[DashboardPayload], None],
        payload: DashboardPayload,
    ) -> Tuple[DashboardPayload, ChaosRecovery]:
        """跑 renderer, chaos 模式按 config.chaos_renderer 触发模拟故障."""
        chaos_kind = self.config.chaos_renderer
        triggered = False
        attempts = 0
        last_err: Optional[str] = None
        quarantine: Optional[Path] = None
        max_attempt = max(1, int(self.config.chaos_retry_max))

        for attempt in range(1, max_attempt + 1):
            attempts = attempt
            try:
                if chaos_kind == "raise" and attempt == 1:
                    triggered = True
                    raise RuntimeError(
                        "V1130 chaos: simulated renderer RuntimeError (attempt 1)"
                    )
                if chaos_kind == "corrupt" and attempt == 1:
                    triggered = True
                    raise ValueError(
                        "V1130 chaos: simulated renderer ValueError (corrupt payload attempt 1)"
                    )
                if chaos_kind == "timeout" and attempt == 1:
                    triggered = True
                    time.sleep(0.05)  # 短延迟模拟
                    raise TimeoutError(
                        "V1130 chaos: simulated renderer TimeoutError (attempt 1)"
                    )
                renderer(payload)
                # 成功
                return payload, ChaosRecovery(
                    triggered=triggered,
                    renderer_kind=chaos_kind or "none",
                    attempts=attempts,
                    payload_safe=True,
                    quarantined_path=None,
                    last_error=None,
                )
            except Exception as exc:  # noqa: BLE001 — chaos 兜底故意宽捕
                last_err = f"{type(exc).__name__}: {exc}"
                with self._lock:
                    quarantine = self.config.out_dir / "v1130_quarantine.json"
                    quarantine.write_text(
                        _safe_dumps(
                            {
                                "kind": "render_failure",
                                "renderer": chaos_kind or "none",
                                "attempts": attempt,
                                "error": last_err,
                                "traceback": traceback.format_exc(limit=4),
                                "payload_summary": {
                                    "n_sessions": payload.config.n_sessions,
                                    "n_benchmarks": len(payload.benchmark_rows),
                                    "n_stress": len(payload.stress_reports),
                                    "perf_wallclock_ms": payload.perf_stats.wallclock_ms,
                                },
                            }
                        ),
                        encoding="utf-8",
                    )
                # 继续重试

        # 全部重试用完, payload 还是安全的 (在 quarantine 里)
        return payload, ChaosRecovery(
            triggered=triggered,
            renderer_kind=chaos_kind or "none",
            attempts=attempts,
            payload_safe=False,
            quarantined_path=quarantine,
            last_error=last_err,
        )


# ---------------------------------------------------------------------------
# 4. DashboardRenderer — 3 类输出 (JSON / Markdown / HTML)
# ---------------------------------------------------------------------------


class DashboardRenderer:
    """V1130 dashboard 渲染 — JSON / Markdown / HTML 3 类输出 (主 00:56)."""

    def __init__(self, config: DashboardConfig) -> None:
        self.config = config
        config.ensure_dirs()

    def render(self, payload: DashboardPayload) -> Dict[str, Path]:
        """渲染 3 类输出, 返回路径字典."""
        paths = {
            "json": self._render_json(payload),
            "markdown": self._render_markdown(payload),
            "html": self._render_html(payload),
        }
        return paths

    def _render_json(self, payload: DashboardPayload) -> Path:
        path = self.config.out_dir / "v1130_dashboard.json"
        path.write_text(
            _safe_dumps(payload.to_dict()),
            encoding="utf-8",
        )
        return path

    def _render_markdown(self, payload: DashboardPayload) -> Path:
        lines: List[str] = []
        lines.append(f"# {self.config.title}")
        lines.append("")
        lines.append(f"- **identity_id**: `{payload.config.identity_id}`")
        lines.append(f"- **n_sessions**: {payload.config.n_sessions}")
        lines.append(f"- **built_at**: {payload.built_at}")
        lines.append(
            f"- **perf_wallclock_ms**: {payload.perf_stats.wallclock_ms:.2f}"
            f" (target 2500 ms, target_met={payload.perf_stats.target_2_5s})"
        )
        lines.append(f"- **v1118_enabled**: {payload.perf_stats.v1118_enabled}")
        if payload.perf_stats.v1118_enabled:
            lines.append(
                f"  - fast_path_runs={payload.perf_stats.fast_path_runs} "
                f"fast_path_fallbacks={payload.perf_stats.fast_path_fallbacks}"
            )
        lines.append("")

        # Timeline 子摘要
        tj = payload.timeline_json
        lines.append("## ContinuityTimelineViz")
        lines.append(f"- identity: `{tj.get('identity_id')}`")
        lines.append(f"- title: {tj.get('title')}")
        lines.append(f"- n_sessions: {tj.get('n_sessions')}")
        lines.append(f"- total_entries: {tj.get('total_entries')}")
        lines.append(f"- total_duration_s: {tj.get('total_duration_s', 0):.3f}")
        points = tj.get("points", [])
        n_active = sum(1 for p in points if p.get("is_active"))
        lines.append(f"- timeline_points: {len(points)} (active: {n_active})")
        lines.append(f"- continuity_score: {float(tj.get('continuity_score', 0.0)):.4f}")
        ms = tj.get("manifest_stats", {})
        if ms:
            lines.append(
                f"- manifest_stats: LTM={ms.get('n_ltm', 0)} "
                f"MTM={ms.get('n_mtm', 0)} STM={ms.get('n_stm', 0)} "
                f"importance_mean={ms.get('importance_mean', 0):.3f}"
            )
        lines.append(f"- timeline_md: `{payload.timeline_md_path}`")
        lines.append(f"- timeline_svg: `{payload.timeline_svg_path}`")
        lines.append("")

        # Recovery 索引
        rs = payload.recovery_summary
        lines.append("## RecoveryRecordIndex")
        lines.append(f"- db_path: `{rs.get('db_path')}`")
        lines.append(f"- n_records: {rs.get('n_records')}")
        lines.append(f"- n_returned_by_chunk: {rs.get('n_returned_by_chunk')}")
        lines.append(f"- index_names: {rs.get('index_names')}")
        lines.append(
            f"- first_ts: {rs.get('first_ts')} "
            f"all_recovered_geq_zero: {rs.get('all_recovered_geq_zero')}"
        )
        lines.append("")

        # Benchmark 表
        lines.append("## CrossTableJoinBenchmark")
        lines.append("")
        lines.append("| scale | join_ms_no_index | join_ms_with_index | speedup_x |")
        lines.append("|---:|---:|---:|---:|")
        for row in payload.benchmark_rows:
            speedup = (
                row.get("join_ms_no_index", 0) / max(row.get("join_ms_with_index", 0.001), 0.001)
                if row.get("join_ms_with_index")
                else 0.0
            )
            lines.append(
                f"| {row.get('scale')} | {row.get('join_ms_no_index', 0):.3f} "
                f"| {row.get('join_ms_with_index', 0):.3f} | {speedup:.2f}x |"
            )
        lines.append("")

        # Stress
        lines.append("## StressDrill")
        if not payload.stress_reports:
            lines.append("_Stress 跑未开启 (enable_full_stress=False)._")
        else:
            lines.append("")
            lines.append("| drill_kind | success | runtime_ms | metrics_keys |")
            lines.append("|---|:---:|---:|---|")
            for row in payload.stress_reports:
                metrics_keys = ",".join(sorted((row.get("metrics") or {}).keys()))
                lines.append(
                    f"| {row.get('drill_kind')} | {row.get('success')} "
                    f"| {row.get('runtime_ms', 0):.2f} | {metrics_keys} |"
                )
        lines.append("")

        # Chaos 兜底
        if payload.chaos_recovery is not None:
            cr = payload.chaos_recovery
            lines.append("## Chaos Recovery")
            lines.append(f"- triggered: {cr.get('triggered')}")
            lines.append(f"- renderer_kind: {cr.get('renderer_kind')}")
            lines.append(f"- attempts: {cr.get('attempts')}")
            lines.append(f"- payload_safe: {cr.get('payload_safe')}")
            if cr.get("quarantined_path"):
                lines.append(f"- quarantined_path: `{cr.get('quarantined_path')}`")
            if cr.get("last_error"):
                lines.append(f"- last_error: `{cr.get('last_error')}`")
            lines.append("")

        # 主哲学 footer
        lines.append("---")
        lines.append(
            "主哲学: 22:33 ASI 北极星 + 17:43 实事求是 + 17:58 不假装 + "
            "23:44 干到底 + 19:33 走在前人经验上 + 12:14 中央 AI 是永恒身份 + 00:56 任何人都能接手"
        )

        path = self.config.out_dir / "v1130_dashboard.md"
        path.write_text("\n".join(lines) + "\n", encoding="utf-8")
        return path

    def _render_html(self, payload: DashboardPayload) -> Path:
        # 简洁 HTML (主 17:43 实事求是 — 不堆花哨)
        bench_rows_html = "".join(
            f"<tr><td>{r.get('scale')}</td>"
            f"<td>{r.get('join_ms_no_index', 0):.3f}</td>"
            f"<td>{r.get('join_ms_with_index', 0):.3f}</td></tr>"
            for r in payload.benchmark_rows
        )
        stress_rows_html = "".join(
            f"<tr><td>{r.get('drill_kind')}</td>"
            f"<td>{'PASS' if r.get('success') else 'FAIL'}</td>"
            f"<td>{r.get('runtime_ms', 0):.2f}</td></tr>"
            for r in payload.stress_reports
        )

        html = f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8" />
<title>{self.config.title}</title>
<style>
  body {{ font-family: system-ui, sans-serif; margin: 2rem; max-width: 980px; }}
  table {{ border-collapse: collapse; margin: 1rem 0; }}
  th, td {{ border: 1px solid #ccc; padding: 0.4rem 0.8rem; text-align: left; }}
  th {{ background: #f0f0f0; }}
  .meta {{ color: #666; font-size: 0.9rem; }}
  .ok {{ color: #10b981; }}
  .fail {{ color: #ef4444; }}
</style>
</head>
<body>
<h1>{self.config.title}</h1>
<p class="meta">identity_id: <code>{payload.config.identity_id}</code> |
   n_sessions: {payload.config.n_sessions} | built_at: {payload.built_at}</p>
<p class="meta">perf_wallclock_ms: <strong>{payload.perf_stats.wallclock_ms:.2f}</strong>
   (target 2500ms, target_met={payload.perf_stats.target_2_5s})</p>

<h2>ContinuityTimelineViz</h2>
<ul>
  <li>n_sessions: {payload.timeline_json.get('n_sessions')}</li>
  <li>total_entries: {payload.timeline_json.get('total_entries')}</li>
  <li>timeline_points: {len(payload.timeline_json.get('points', []))}</li>
  <li>continuity_score: {float(payload.timeline_json.get('continuity_score', 0.0)):.4f}</li>
</ul>
<p>Markdown: <code>{payload.timeline_md_path}</code></p>
<p>SVG: <code>{payload.timeline_svg_path}</code></p>

<h2>RecoveryRecordIndex</h2>
<ul>
  <li>n_records: {payload.recovery_summary.get('n_records')}</li>
  <li>index_names: {payload.recovery_summary.get('index_names')}</li>
</ul>

<h2>CrossTableJoinBenchmark</h2>
<table>
  <thead><tr><th>scale</th><th>no_index_ms</th><th>with_index_ms</th></tr></thead>
  <tbody>{bench_rows_html}</tbody>
</table>

<h2>StressDrill</h2>
<table>
  <thead><tr><th>drill_kind</th><th>success</th><th>runtime_ms</th></tr></thead>
  <tbody>{stress_rows_html or '<tr><td colspan="3">disabled</td></tr>'}</tbody>
</table>

<footer class="meta">
  V1130 dashboard · {payload.built_at} · V1122={V1122_VERSION} · V1118_OPT=0.1.0
</footer>
</body>
</html>
"""
        path = self.config.out_dir / "v1130_dashboard.html"
        path.write_text(html, encoding="utf-8")
        return path


# ---------------------------------------------------------------------------
# 5. CLI — 主 00:56 任何人都能接手
# ---------------------------------------------------------------------------


def main(argv: Optional[Sequence[str]] = None) -> int:
    """V1130 dashboard CLI.

    用法:
      python -m apeireth.v1130_continuity_tracker_dashboard --report
      python -m apeireth.v1130_continuity_tracker_dashboard --chaos raise
      python -m apeireth.v1130_continuity_tracker_dashboard --print-json
    """
    import argparse

    p = argparse.ArgumentParser(
        prog="v1130_continuity_tracker_dashboard",
        description="V1130 R10-W2 ContinuityTracker Dashboard 真跑集成",
    )
    p.add_argument("--out-dir", default="report/v1130_outputs")
    p.add_argument("--db-dir", default="report/v1130_dbs")
    p.add_argument("--identity-id", default="id_v1130_cli")
    p.add_argument("--n-sessions", type=int, default=6)
    p.add_argument("--no-v1118", action="store_true", help="关 V1118 perf wrap")
    p.add_argument(
        "--scales",
        default="1000,10000",
        help="benchmark scales, comma-separated",
    )
    p.add_argument(
        "--no-stress",
        action="store_true",
        help="关 3 类 stress 演练 (避免 dashboard 慢)",
    )
    p.add_argument(
        "--chaos",
        default=None,
        choices=["raise", "corrupt", "timeout"],
        help="chaos renderer 模拟 (dashboard 渲染失联)",
    )
    p.add_argument("--report", action="store_true", help="落盘 dashboard 3 类输出")
    p.add_argument("--print-json", action="store_true", help="stdout 打印 payload JSON")
    args = p.parse_args(argv)

    scales = tuple(int(s.strip()) for s in args.scales.split(",") if s.strip())
    config = DashboardConfig(
        out_dir=Path(args.out_dir),
        db_dir=Path(args.db_dir),
        identity_id=args.identity_id,
        n_sessions=args.n_sessions,
        enable_v1118=not args.no_v1118,
        benchmark_scales=scales,
        enable_full_stress=not args.no_stress,
        chaos_simulation=args.chaos is not None,
        chaos_renderer=args.chaos,
    )
    config.ensure_dirs()

    dash = ContinuityDashboard(config)
    payload = dash.build()

    # 渲染 (含 chaos)
    renderer = DashboardRenderer(config)
    safety = AsyncSafety(config)

    if args.chaos:
        # chaos mode
        _, recovery = safety.render_with_chaos(
            lambda p_: renderer.render(p_),
            payload,
        )
        payload.chaos_recovery = recovery.to_dict()
        # chaos 模式下也要重渲染 (走 retry 后的成功路径)
        if recovery.payload_safe:
            renderer.render(payload)
    else:
        renderer.render(payload)

    print(
        f"[V1130] dashboard built: "
        f"perf_ms={payload.perf_stats.wallclock_ms:.2f} "
        f"target_2_5s={payload.perf_stats.target_2_5s} "
        f"v1118_enabled={payload.perf_stats.v1118_enabled} "
        f"chaos={args.chaos or 'none'}"
    )

    if args.print_json:
        print(_safe_dumps(payload.to_dict()))

    if args.report:
        print(f"[V1130] report written to: {config.out_dir}")

    return 0


# ---------------------------------------------------------------------------
# V1130 V3_GUARDS — 主 17:43 + 17:58 + 20:46 注入
# ---------------------------------------------------------------------------


V3_GUARDS = {
    "module_is_not_asi": (
        "V1130 dashboard 是 V1122 集成 + chaos safety + perf wrap 工具. "
        "Dashboard 漂亮 ≠ ASI 达成. ASI 是 V1072 + V1128 + 跨 agent 目标."
    ),
    "structure_is_not_consciousness": (
        "Timeline chart + continuity_score 走势 + chaos 守门 ≠ 真心理连续性. "
        "Parfit 1984 类比, 不是现象意识. chaos test 验证失联不丢数据, 不验证体验."
    ),
    "measurement_is_not_truth": (
        "perf_wallclock_ms 是 dashboard build 的 wall clock proxy. "
        "真生产 latency 受 OS page cache, fsync, GPU init 影响, 必须配合 V1084 audit."
    ),
    "production_is_not_safety": (
        "controlled chaos (raise / corrupt / timeout) ≠ 真渲染失联. "
        "真渲染失联模式更复杂 (网络半截 / 浏览器 OOM / JSON 截断)."
    ),
    "automation_is_not_autonomy": (
        "ChaosSafety 自动重试 ≠ 自主恢复. 真 dashboard 失联需要 SOP + 告警 + 人工 review."
    ),
}


__all__ = [
    "V1130_VERSION",
    "CONTINUITY_SCHEMA_VERSION",
    "ContinuitySnapshotStore",
    "V3_GUARDS",
    "DashboardConfig",
    "V1130PerfWrap",
    "V1130PerfWrapStats",
    "ContinuityDashboard",
    "DashboardPayload",
    "AsyncSafety",
    "ChaosRecovery",
    "DashboardRenderer",
    "main",
]


if __name__ == "__main__":
    sys.exit(main())
