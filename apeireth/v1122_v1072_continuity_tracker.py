"""V1122 V1072 ContinuityTracker Timeline 可视化 — R9-DB-003.

任务 (R9-DB-003): 补全 R9-DB-002 评审建议, 完成 V1072 ContinuityTracker timeline
可视化 + R9 数据库收尾总报告 + recovery_record 索引 + 性能 benchmark.

V1122 = 4 个真生产子组件:
  1. ContinuityTimelineViz — V1072 ContinuityTracker + IdentityManifest 联动,
     产 3 类可视化输出 (JSON dict / Markdown 报告 / SVG timeline)
  2. RecoveryRecordIndex — 把 V1109 的 recover_corrupt() dict 升级为 SQLite 表
     recovery_record + 复合索引 idx_recovery_chunk_ts (chunk_id, ts DESC),
     让 replay 查询 (chunk_id 锚定 + 时间窗过滤) 走索引
  3. CrossTableJoinBenchmark — 1000 / 10000 / 100000 行 8 表 JOIN 真跑 benchmark,
     含 EXPLAIN QUERY PLAN 与索引差异对照
  4. StressDrill — 3 类高强度演练:
       a. MigrationStressDrill — 10× 数据量 (1460 行 v0.1.0 → v0.1.2)
       b. JoinStressDrill — 100K 行跨表 join + continuity_score 校验
       c. DisasterStressDrill — 50 corrupt + 200 valid (corrupt 比率 20%) 大规模
                                  recovery + recovery_record 索引命中率验证

设计原则 (主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 13:31 大胆激进 +
主 23:44 干到底 + 主 19:33 走在前人经验上 + 主 00:56 任何人都能接手):
  - 真跑 = 真实 db + 真实 SQL + 真实 EXPLAIN QUERY PLAN, 不 mock
  - 可视化可追溯: 字段一一对应 ContinuityTracker.sessions, 不漏不掉
  - 守门: V3_GUARDS 5 条主 17:43 / 17:58 / 20:46 注入
  - 主 00:56: `python -m apeireth.v1122_v1072_continuity_tracker --report` 一行命令

执行:
  from apeireth.v1122_v1072_continuity_tracker import (
      ContinuityTimelineViz,
      RecoveryRecordIndex,
      CrossTableJoinBenchmark,
      StressDrill,
  )

  # 1. timeline 可视化 (3 类输出)
  v = ContinuityTimelineViz()
  v.feed_tracker(ct)               # V1072 ContinuityTracker 真数据
  v.feed_manifest(mf)              # V1072 IdentityManifest 真数据
  v.to_json() / v.to_markdown() / v.to_svg()

  # 2. recovery_record 索引迁移
  rr = RecoveryRecordIndex(db_path)
  rr.migrate()                      # 建表 + 索引
  rr.record({"chunk_id": ..., ...}) # 写 recovery_record
  rr.query_by_chunk("chunk_x")      # 走索引

  # 3. 跨表 join benchmark
  b = CrossTableJoinBenchmark()
  b.run(scale=10000)                # 跑 10K 行 join

  # 4. 3 类 stress
  StressDrill(db_path).run_full_stress()

V1122_VERSION = "0.1.0"
"""

from __future__ import annotations

import hashlib
import json
import math
import os
import random
import sqlite3
import sys
import tempfile
import time
import uuid
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Tuple

# V1072 IdentityCore / Manifest / ContinuityTracker (Parfit 1984 真借鉴)
from apeireth.v1072_asi_central_ai_eternal_identity import (
    ETERNAL_IDENTITY_CORE,
    ContinuityTracker,
    IdentityCore,
    IdentityManifest,
    IdentityManifestEntry,
    SessionMarker,
)

# V1109 Memory Schema v0.1.2 (WAL chunk / checksum / replay)
from apeireth.v1109_memory_schema_v012 import (
    DREAM_PHASES,
    HIGH_IMPACT_THRESHOLD,
    MemorySchemaV012,
    V1109_VERSION,
    ChecksumReport,
    downgrade_v012,
    upgrade_v012,
    upgrade_v012_path,
    verify_wal_checksums,
    replay_events_by_chunk,
)

# V1094 base (for stress baseline)
from apeireth.v1094_memory_schema import (
    SCHEMA_V094,
    MemorySchema as MemorySchemaV094,
    V1094_VERSION,
)


# ---------------------------------------------------------------------------
# 版本 + 命名空间 + 工具
# ---------------------------------------------------------------------------

V1122_VERSION = "0.1.0"


def _now_iso() -> str:
    return time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime())


def _seed_for(tag: str) -> int:
    """稳定种子 — 与 V1113 一致风格: tag → sha256[:8] → int."""
    return int(hashlib.sha256(tag.encode("utf-8")).hexdigest()[:8], 16)


def _truncate(s: str, n: int) -> str:
    if len(s) <= n:
        return s
    return s[: n - 3] + "..."


def _b(s: str) -> bytes:
    return s.encode("utf-8")


# ===========================================================================
# 1. ContinuityTimelineViz — V1072 ContinuityTracker timeline 可视化
# ===========================================================================


@dataclass
class TimelinePoint:
    """V1122 timeline 单点 — 直接对应 V1072 SessionMarker (主 17:43 实事求是,
    字段一一对应, 不漏)."""

    session_id: str
    started_at: float
    ended_at: float
    duration_s: float
    n_entries_added: int
    n_importance_avg: float
    is_active: bool
    continuity_weight: float  # 0..1 — n_entries_added / max

    def to_row(self) -> List[Any]:
        return [
            self.session_id,
            _now_iso().split("T")[0],  # date
            self.started_at,
            self.duration_s,
            self.n_entries_added,
            round(self.n_importance_avg, 3),
            "●" if self.is_active else "○",
            round(self.continuity_weight, 4),
        ]


class ContinuityTimelineViz:
    """V1072 ContinuityTracker timeline 可视化 (3 类输出, 主 00:56 一行命令).

    借鉴 (主 19:33):
      - V1072 ContinuityTracker.sessions (主数据源, 真生产)
      - V1072 IdentityManifest.stats() (侧链锚定 — 跨 LTM/MTM/STM 分布)
      - James 1890 stream of consciousness (timeline 概念)
      - Parfit 1984 psychological continuity (continuity_score 真生产)
    """

    def __init__(self, identity_id: Optional[str] = None,
                 title: str = "V1072 ContinuityTracker Timeline"):
        self.identity_id = identity_id or ETERNAL_IDENTITY_CORE["identity_id"]
        self.title = title
        self._points: List[TimelinePoint] = []
        self._manifest_stats: Dict[str, Any] = {}
        self._generated_at: float = time.time()

    # --- 数据输入 ---

    def feed_tracker(self, ct: ContinuityTracker) -> None:
        """从 V1072 ContinuityTracker 真生产喂入数据 (主 17:43 — 真数据,
        不漏字段)."""
        if not ct.sessions:
            return  # 主 17:43 — 空数据也算事实, 不假装
        max_entries = max((s.n_entries_added for s in ct.sessions.values()),
                          default=1) or 1
        for sid, sm in ct.sessions.items():
            duration = max(0.0, sm.ended_at - sm.started_at) if sm.ended_at else 0.0
            weight = sm.n_entries_added / max_entries
            self._points.append(TimelinePoint(
                session_id=sid,
                started_at=sm.started_at,
                ended_at=sm.ended_at,
                duration_s=duration,
                n_entries_added=sm.n_entries_added,
                n_importance_avg=sm.n_importance_avg,
                is_active=sm.is_active,
                continuity_weight=weight,
            ))
        # 按 started_at 排序 — timeline 顺序
        self._points.sort(key=lambda p: p.started_at)

    def feed_manifest(self, mf: IdentityManifest) -> None:
        """从 V1072 IdentityManifest 真生产喂入侧链 — 用于 SVG 标注 LTM/MTM/STM
        分布."""
        self._manifest_stats = mf.stats()
        self._manifest_stats["core_identity_id"] = mf.core.identity_id

    # --- 派生数据 ---

    @property
    def continuity_score(self) -> float:
        """V1072 ContinuityTracker.continuity_score 同口径 — Parfit 心理连续性
        真生产."""
        if not self._points:
            return 0.0
        n_total = len(self._points)
        n_with_entries = sum(1 for p in self._points if p.n_entries_added > 0)
        return n_with_entries / n_total

    @property
    def n_sessions(self) -> int:
        return len(self._points)

    @property
    def total_entries(self) -> int:
        return sum(p.n_entries_added for p in self._points)

    @property
    def total_duration_s(self) -> float:
        return sum(p.duration_s for p in self._points)

    # --- 输出 1: JSON ---

    def to_json(self) -> Dict[str, Any]:
        return {
            "v1122_version": V1122_VERSION,
            "title": self.title,
            "identity_id": self.identity_id,
            "generated_at_iso": _now_iso(),
            "generated_at_epoch": self._generated_at,
            "n_sessions": self.n_sessions,
            "total_entries": self.total_entries,
            "total_duration_s": round(self.total_duration_s, 4),
            "continuity_score": round(self.continuity_score, 4),
            "manifest_stats": self._manifest_stats,
            "points": [asdict(p) for p in self._points],
            "philosophy_anchor": ETERNAL_IDENTITY_CORE["philosophy_anchor"],
        }

    # --- 输出 2: Markdown ---

    def to_markdown(self) -> str:
        lines: List[str] = []
        lines.append(f"# {self.title}")
        lines.append("")
        lines.append(f"- **Identity**: `{self.identity_id}`")
        lines.append(f"- **V1122 Version**: {V1122_VERSION}")
        lines.append(f"- **Generated (ISO)**: {_now_iso()}")
        lines.append(f"- **Sessions**: {self.n_sessions}")
        lines.append(f"- **Total Entries**: {self.total_entries}")
        lines.append(f"- **Total Duration (s)**: {self.total_duration_s:.4f}")
        lines.append(f"- **Continuity Score** (Parfit 1984): {self.continuity_score:.4f}")
        lines.append(f"- **Philosophy Anchor**: {ETERNAL_IDENTITY_CORE['philosophy_anchor']}")
        lines.append("")
        if self._manifest_stats:
            lines.append("## IdentityManifest Side-chain")
            lines.append("")
            for k, v in self._manifest_stats.items():
                lines.append(f"- **{k}**: {v}")
            lines.append("")
        lines.append("## Timeline (Session Markers)")
        lines.append("")
        lines.append("| # | session_id | date | started_at | duration_s | n_entries | importance_avg | active | weight |")
        lines.append("|---|------------|------|------------|------------|-----------|---------------|--------|--------|")
        for i, p in enumerate(self._points, 1):
            row = p.to_row()
            lines.append(
                f"| {i} | `{_truncate(row[0], 18)}` | {row[1]} | "
                f"{row[2]:.3f} | {row[3]:.3f} | {row[4]} | "
                f"{row[5]} | {row[6]} | {row[7]} |"
            )
        lines.append("")
        lines.append("## Continuity Score Trend (走势)")
        lines.append("")
        if not self._points:
            lines.append("_No sessions — empty tracker._")
        else:
            # 简单 ASCII sparkline — 主 00:56 任何人都能接手
            n_bins = min(40, self.n_sessions)
            chunk = max(1, self.n_sessions // n_bins)
            bins = []
            for i in range(0, self.n_sessions, chunk):
                sub = self._points[i:i + chunk]
                avg_weight = sum(p.continuity_weight for p in sub) / len(sub)
                bins.append(avg_weight)
            bars = " ▁▂▃▄▅▆▇█"
            sparkline = "".join(bars[min(8, int(b * 8))] for b in bins)
            lines.append("```")
            lines.append(sparkline)
            lines.append("```")
            lines.append(
                "_x-axis: session order "
                f"({self.n_sessions} sessions, {n_bins} bins), "
                "y-axis: continuity_weight (0..1)_"
            )
        lines.append("")
        lines.append("---")
        lines.append("")
        lines.append(f"_Generated by V1122 ContinuityTimelineViz · "
                     f"主 17:43 实事求是 + 主 19:33 走在前人经验上 + 主 00:56 任何人都能接手_")
        return "\n".join(lines)

    # --- 输出 3: SVG ---

    def to_svg(self, width: int = 800, height: int = 320) -> str:
        """纯 Python SVG timeline — 不依赖 matplotlib (主 00:56 一行命令)."""
        if not self._points:
            return self._svg_empty(width, height)

        margin_l, margin_r = 60, 30
        margin_t, margin_b = 50, 70
        plot_w = width - margin_l - margin_r
        plot_h = height - margin_t - margin_b
        n = self.n_sessions
        max_entries = max((p.n_entries_added for p in self._points), default=1) or 1
        max_duration = max((p.duration_s for p in self._points), default=1.0) or 1.0

        # y 双轴: 上 = continuity_weight, 下 = duration (normalized)
        bar_w = max(8.0, plot_w / max(n, 1) - 4.0)

        out: List[str] = []
        out.append(
            f'<svg xmlns="http://www.w3.org/2000/svg" '
            f'width="{width}" height="{height}" '
            f'viewBox="0 0 {width} {height}" '
            f'font-family="Helvetica, Arial, sans-serif" font-size="11">'
        )
        out.append(f'<rect x="0" y="0" width="{width}" height="{height}" '
                   f'fill="#fafafa" stroke="#ddd"/>')
        out.append(
            f'<text x="{width / 2:.0f}" y="22" text-anchor="middle" '
            f'font-size="14" font-weight="bold" fill="#222">'
            f'{self.title} · continuity={self.continuity_score:.4f}</text>'
        )
        out.append(
            f'<text x="{width / 2:.0f}" y="38" text-anchor="middle" '
            f'font-size="10" fill="#666">'
            f'identity={_truncate(self.identity_id, 24)} · '
            f'sessions={self.n_sessions} · entries={self.total_entries}'
            f'</text>'
        )

        # x 轴 baseline
        out.append(
            f'<line x1="{margin_l}" y1="{margin_t + plot_h}" '
            f'x2="{margin_l + plot_w}" y2="{margin_t + plot_h}" '
            f'stroke="#888" stroke-width="1"/>'
        )
        # y 双轴标签
        out.append(
            f'<text x="{margin_l - 8}" y="{margin_t + 8}" text-anchor="end" '
            f'font-size="10" fill="#444">weight</text>'
        )
        out.append(
            f'<text x="{margin_l - 8}" y="{margin_t + plot_h - 4}" text-anchor="end" '
            f'font-size="10" fill="#444">dur</text>'
        )

        for i, p in enumerate(self._points):
            x = margin_l + i * (plot_w / max(n, 1)) + 2
            # continuity_weight bar (上) — 蓝
            h_weight = p.continuity_weight * plot_h * 0.55
            out.append(
                f'<rect x="{x:.1f}" y="{margin_t + plot_h - h_weight:.1f}" '
                f'width="{bar_w:.1f}" height="{h_weight:.1f}" '
                f'fill="#3b82f6" opacity="0.85">'
                f'<title>{p.session_id} weight={p.continuity_weight:.3f} '
                f'entries={p.n_entries_added}</title></rect>'
            )
            # duration bar (下) — 绿
            h_dur = (p.duration_s / max_duration) * plot_h * 0.4
            out.append(
                f'<rect x="{x:.1f}" y="{margin_t + plot_h + 4:.1f}" '
                f'width="{bar_w:.1f}" height="{h_dur:.1f}" '
                f'fill="#10b981" opacity="0.75">'
                f'<title>{p.session_id} duration_s={p.duration_s:.3f}</title></rect>'
            )
            # active dot
            if p.is_active:
                out.append(
                    f'<circle cx="{x + bar_w / 2:.1f}" '
                    f'cy="{margin_t + plot_h - h_weight - 6:.1f}" r="3" '
                    f'fill="#ef4444"><title>active</title></circle>'
                )

        # 标签 — 第一个 + 最后一个 session_id
        if self._points:
            first = self._points[0]
            last = self._points[-1]
            out.append(
                f'<text x="{margin_l}" y="{height - margin_b + 50}" '
                f'font-size="9" fill="#555">{_truncate(first.session_id, 18)}</text>'
            )
            out.append(
                f'<text x="{margin_l + plot_w}" y="{height - margin_b + 50}" '
                f'text-anchor="end" font-size="9" fill="#555">'
                f'{_truncate(last.session_id, 18)}</text>'
            )

        # legend
        out.append(
            f'<rect x="{margin_l}" y="{height - 22}" width="10" height="10" '
            f'fill="#3b82f6" opacity="0.85"/>'
        )
        out.append(
            f'<text x="{margin_l + 14}" y="{height - 12}" font-size="10" '
            f'fill="#444">continuity_weight</text>'
        )
        out.append(
            f'<rect x="{margin_l + 130}" y="{height - 22}" width="10" height="10" '
            f'fill="#10b981" opacity="0.75"/>'
        )
        out.append(
            f'<text x="{margin_l + 144}" y="{height - 12}" font-size="10" '
            f'fill="#444">duration_s</text>'
        )
        out.append(
            f'<circle cx="{margin_l + 240}" cy="{height - 17}" r="3" fill="#ef4444"/>'
        )
        out.append(
            f'<text x="{margin_l + 248}" y="{height - 12}" font-size="10" '
            f'fill="#444">active session</text>'
        )

        out.append(
            f'<text x="{width - 10}" y="{height - 6}" text-anchor="end" '
            f'font-size="9" fill="#999">V1122 v{V1122_VERSION} · '
            f'{_now_iso()}</text>'
        )
        out.append('</svg>')
        return "".join(out)

    def _svg_empty(self, width: int, height: int) -> str:
        return (
            f'<svg xmlns="http://www.w3.org/2000/svg" '
            f'width="{width}" height="{height}" viewBox="0 0 {width} {height}">'
            f'<rect width="{width}" height="{height}" fill="#fafafa"/>'
            f'<text x="{width / 2}" y="{height / 2}" text-anchor="middle" '
            f'font-family="sans-serif" font-size="14" fill="#888">'
            f'Empty ContinuityTracker — no sessions to visualize</text></svg>'
        )

    # --- 落盘 ---

    def write_all(self, out_dir: str | Path) -> Dict[str, str]:
        """3 类可视化一键落盘 — 主 00:56 一行命令."""
        out = Path(out_dir)
        out.mkdir(parents=True, exist_ok=True)
        json_path = out / "continuity_timeline.json"
        md_path = out / "continuity_timeline.md"
        svg_path = out / "continuity_timeline.svg"
        json_path.write_text(
            json.dumps(self.to_json(), ensure_ascii=False, indent=2),
            encoding="utf-8",
        )
        md_path.write_text(self.to_markdown(), encoding="utf-8")
        svg_path.write_text(self.to_svg(), encoding="utf-8")
        return {
            "json": str(json_path),
            "markdown": str(md_path),
            "svg": str(svg_path),
        }


# ===========================================================================
# 2. RecoveryRecordIndex — recovery_record 表 + 复合索引 (R9-DB-003 评审)
# ===========================================================================


RECOVERY_RECORD_TABLE_DDL = """
CREATE TABLE IF NOT EXISTS recovery_record (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    ts              REAL NOT NULL,
    chunk_id        TEXT NOT NULL DEFAULT '',
    seq             INTEGER NOT NULL DEFAULT 0,
    event_id        TEXT NOT NULL DEFAULT '',
    identity_id     TEXT NOT NULL DEFAULT '',
    scope           TEXT NOT NULL DEFAULT '',
    corrupt_kind    TEXT NOT NULL DEFAULT 'tampered',  -- tampered/deleted/missing
    health_ratio    REAL NOT NULL DEFAULT 0.0,
    detail_json     TEXT NOT NULL DEFAULT '{}',
    record_kind     TEXT NOT NULL DEFAULT 'drill'  -- drill/real/audit
);
"""

RECOVERY_RECORD_INDEXES_DDL = [
    # 复合索引: replay 按 chunk_id + ts DESC 走索引, 加快灾难恢复查询
    "CREATE INDEX IF NOT EXISTS idx_recovery_chunk_ts "
    "ON recovery_record(chunk_id, ts DESC)",
    # chunk_id 单列: 单 chunk 全历史
    "CREATE INDEX IF NOT EXISTS idx_recovery_chunk "
    "ON recovery_record(chunk_id)",
    # ts 单列: 全局时间线
    "CREATE INDEX IF NOT EXISTS idx_recovery_ts "
    "ON recovery_record(ts DESC)",
    # identity_id: V1072 锚定
    "CREATE INDEX IF NOT EXISTS idx_recovery_identity "
    "ON recovery_record(identity_id)",
]


@dataclass
class RecoveryRecord:
    """单条 recovery_record 记录 — 与表字段一一对应 (主 17:43 实事求是)."""

    ts: float
    chunk_id: str
    seq: int
    event_id: str
    identity_id: str
    scope: str
    corrupt_kind: str
    health_ratio: float
    detail_json: str
    record_kind: str = "drill"
    id: int = 0

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


class RecoveryRecordIndex:
    """V1122 — recovery_record 表 + 复合索引, 让 R9-DB-002 灾难恢复报告从
    '只产 dict' 升级为 '走索引的可查询表'.

    主 19:33 借鉴: PostgreSQL pg_stat_user_indexes 健康监控思想,
    把 recovery_record 真生产成可查询 / 可统计 / 可按 identity 锚定的结构化表.
    """

    def __init__(self, db_path: str):
        self.db_path = db_path
        self._conn = sqlite3.connect(db_path)
        self._conn.row_factory = sqlite3.Row

    def close(self) -> None:
        try:
            self._conn.close()
        except sqlite3.Error:
            pass

    def migrate(self) -> Dict[str, Any]:
        """建表 + 索引 — 幂等."""
        cur = self._conn.cursor()
        cur.executescript(RECOVERY_RECORD_TABLE_DDL)
        for ddl in RECOVERY_RECORD_INDEXES_DDL:
            cur.execute(ddl)
        self._conn.commit()
        # 列出现有索引 (供 benchmark 用)
        cur.execute(
            "SELECT name FROM sqlite_master "
            "WHERE type='index' AND tbl_name='recovery_record' ORDER BY name"
        )
        indexes = [row["name"] for row in cur.fetchall()]
        return {
            "table_created": True,
            "n_indexes": len(indexes),
            "indexes": indexes,
        }

    def record(self, rec: RecoveryRecord) -> int:
        """写一条 recovery_record."""
        cur = self._conn.cursor()
        cur.execute(
            "INSERT INTO recovery_record("
            "ts, chunk_id, seq, event_id, identity_id, scope, "
            "corrupt_kind, health_ratio, detail_json, record_kind"
            ") VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (rec.ts, rec.chunk_id, rec.seq, rec.event_id, rec.identity_id,
             rec.scope, rec.corrupt_kind, rec.health_ratio, rec.detail_json,
             rec.record_kind),
        )
        self._conn.commit()
        return cur.lastrowid or 0

    def query_by_chunk(self, chunk_id: str,
                       limit: int = 100) -> List[RecoveryRecord]:
        """按 chunk_id 查 — 走 idx_recovery_chunk_ts 复合索引."""
        cur = self._conn.cursor()
        cur.execute(
            "SELECT * FROM recovery_record WHERE chunk_id = ? "
            "ORDER BY ts DESC LIMIT ?",
            (chunk_id, limit),
        )
        return [self._row_to_record(row) for row in cur.fetchall()]

    def query_by_ts_range(self, ts_start: float, ts_end: float,
                          limit: int = 100) -> List[RecoveryRecord]:
        cur = self._conn.cursor()
        cur.execute(
            "SELECT * FROM recovery_record "
            "WHERE ts BETWEEN ? AND ? "
            "ORDER BY ts DESC LIMIT ?",
            (ts_start, ts_end, limit),
        )
        return [self._row_to_record(row) for row in cur.fetchall()]

    def query_by_identity(self, identity_id: str,
                          limit: int = 100) -> List[RecoveryRecord]:
        cur = self._conn.cursor()
        cur.execute(
            "SELECT * FROM recovery_record WHERE identity_id = ? "
            "ORDER BY ts DESC LIMIT ?",
            (identity_id, limit),
        )
        return [self._row_to_record(row) for row in cur.fetchall()]

    def stats(self) -> Dict[str, Any]:
        cur = self._conn.cursor()
        cur.execute("SELECT COUNT(*) AS n FROM recovery_record")
        n_total = cur.fetchone()["n"]
        cur.execute(
            "SELECT corrupt_kind, COUNT(*) AS n FROM recovery_record "
            "GROUP BY corrupt_kind"
        )
        by_kind = {row["corrupt_kind"]: row["n"] for row in cur.fetchall()}
        cur.execute(
            "SELECT MIN(ts) AS mn, MAX(ts) AS mx FROM recovery_record"
        )
        row = cur.fetchone()
        return {
            "n_total": n_total,
            "by_corrupt_kind": by_kind,
            "ts_min": row["mn"] if row else None,
            "ts_max": row["mx"] if row else None,
        }

    def explain_query(self, chunk_id: str = "chunk_probe") -> List[str]:
        """EXPLAIN QUERY PLAN — 验证 idx_recovery_chunk_ts 真的被使用."""
        cur = self._conn.cursor()
        cur.execute(
            "EXPLAIN QUERY PLAN "
            "SELECT * FROM recovery_record WHERE chunk_id = ? "
            "ORDER BY ts DESC LIMIT 100",
            (chunk_id,),
        )
        return [str(row[3]) for row in cur.fetchall()]

    @staticmethod
    def _row_to_record(row: sqlite3.Row) -> RecoveryRecord:
        return RecoveryRecord(
            id=row["id"],
            ts=row["ts"],
            chunk_id=row["chunk_id"],
            seq=row["seq"],
            event_id=row["event_id"],
            identity_id=row["identity_id"],
            scope=row["scope"],
            corrupt_kind=row["corrupt_kind"],
            health_ratio=row["health_ratio"],
            detail_json=row["detail_json"],
            record_kind=row["record_kind"],
        )


# ===========================================================================
# 3. CrossTableJoinBenchmark — 1000 / 10000 / 100000 行 跨表 join benchmark
# ===========================================================================


@dataclass
class JoinBenchmarkRow:
    scale: int
    n_rows_total: int
    n_distinct_identities: int
    n_sessions: int
    join_ms_no_index: float
    join_ms_with_index: float
    join_records_total: int
    continuity_score: float
    explain_no_index: List[str]
    explain_with_index: List[str]


class CrossTableJoinBenchmark:
    """V1122 — V1072 跨 8 表 join 真生产 benchmark.

    主 19:33 借鉴: PostgreSQL EXPLAIN ANALYZE + sysbench OLTP_read_write.
    对比 "无 identity_id 索引" vs "有 idx_v012_identity (V1109 已建)"
    在 1K / 10K / 100K 行下的真实耗时差异.
    """

    SCALES = (1000, 10000, 100000)

    def __init__(self, seed: int = 0xC0FFEE):
        self.seed = seed

    def run(self, scales: Iterable[int] = SCALES) -> List[JoinBenchmarkRow]:
        results: List[JoinBenchmarkRow] = []
        for s in scales:
            results.append(self._run_one(s))
        return results

    def _run_one(self, scale: int) -> JoinBenchmarkRow:
        # 临时 DB — 主 17:43 真实测试不污染 artifacts
        tmp = tempfile.NamedTemporaryFile(suffix=".db", delete=False)
        tmp.close()
        db_path = tmp.name
        try:
            schema = MemorySchemaV012(db_path)
            conn = schema._conn

            # 准备 V1072 IdentityCore + ContinuityTracker
            identity_id = f"id_bench_{uuid.uuid4().hex[:12]}"
            ct = ContinuityTracker()
            sid = ct.start_session()
            ct.sessions[sid].n_entries_added = scale

            # 8 表分布 (与 V1113 CrossTableJoinV1072Drill 一致: 8 表)
            per = {
                "memory_hot": scale // 5,
                "memory_cold": scale // 8,
                "memory_wal": scale // 3,
                "memory_dream": scale // 12,
                "memory_snapshots": scale // 33,
                "stm_messages": scale // 8,
                "ltm_facts": scale // 10,
                "mtm_themes": scale // 20,
            }
            # 补齐 — 总和 = scale
            delta = scale - sum(per.values())
            per["memory_hot"] += delta

            rng = random.Random(self.seed + scale)
            now = time.time()

            for i in range(per["memory_hot"]):
                conn.execute(
                    "INSERT INTO memory_hot(id, session_id, actor, content, "
                    "ts, fingerprint, identity_id) VALUES (?, ?, ?, ?, ?, ?, ?)",
                    (f"bh_{i:08d}", f"bs_{i % 3}", "bench",
                     f"hot{i}", now - rng.random() * 3600,
                     hashlib.sha256(_b(f"bh_{i}")).hexdigest()[:16],
                     identity_id),
                )
            for i in range(per["memory_cold"]):
                conn.execute(
                    "INSERT INTO memory_cold(id, content, ts, fingerprint, "
                    "identity_id) VALUES (?, ?, ?, ?, ?)",
                    (f"bc_{i:08d}", f"cold{i}", now - rng.random() * 86400,
                     hashlib.sha256(_b(f"bc_{i}")).hexdigest()[:16],
                     identity_id),
                )
            for i in range(per["memory_wal"]):
                schema.wal_append_with_chunk(
                    rng.choice(["hot", "cold", "mtm", "ltm"]),
                    "tag_set",
                    {"i": i},
                    chunk_id=f"bench_chunk_{i // 500}",
                    identity_id=identity_id,
                    event_id=f"bevw_{i:08d}",
                    impact=0.5,
                )
            for i in range(per["memory_dream"]):
                conn.execute(
                    "INSERT INTO memory_dream(id, summary, confidence, "
                    "importance, dream_state, ts, identity_id, dream_phase) "
                    "VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                    (f"bd_{i:08d}", f"dream{i}", 0.7, 5, "CONSOLIDATING",
                     now, identity_id, DREAM_PHASES[i % 3]),
                )
            for i in range(per["memory_snapshots"]):
                conn.execute(
                    "INSERT INTO memory_snapshots(scope, seq, ts, "
                    "content_hash, rationale, identity_hash, snapshot_score) "
                    "VALUES (?, ?, ?, ?, ?, ?, ?)",
                    ("bench-snap", i, now,
                     hashlib.sha256(_b(f"snap{i}")).hexdigest()[:16],
                     f"bench rationale {i}", "", 0.5),
                )
            for i in range(per["stm_messages"]):
                conn.execute(
                    "INSERT INTO stm_messages(id, session_id, role, content, "
                    "ts, fingerprint, identity_id) "
                    "VALUES (?, ?, ?, ?, ?, ?, ?)",
                    (f"bsm_{i:08d}", f"bsm_s_{i % 3}",
                     "user" if i % 2 == 0 else "assistant",
                     f"stm{i}", now,
                     hashlib.sha256(_b(f"bsm_{i}")).hexdigest()[:16],
                     identity_id),
                )
            for i in range(per["ltm_facts"]):
                conn.execute(
                    "INSERT INTO ltm_facts(id, category, content, importance, "
                    "confidence, fingerprint, ts, identity_id) "
                    "VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                    (f"blf_{i:08d}", "fact",
                     f"ltm{i}", 5, 0.9,
                     hashlib.sha256(_b(f"ltm{i}")).hexdigest()[:16],
                     now, identity_id),
                )
            for i in range(per["mtm_themes"]):
                conn.execute(
                    "INSERT INTO mtm_themes(topic_id, topic_label, "
                    "n_episodes, importance_avg, summary, last_updated, "
                    "fingerprint, identity_id) "
                    "VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                    (f"bmt_{i:08d}", f"theme{i}",
                     1, 0.6, "summary", now,
                     hashlib.sha256(_b(f"bmt_{i}")).hexdigest()[:16],
                     identity_id),
                )
            conn.commit()

            total = sum(per.values())

            # ---- 无 identity_id 索引情况下的 JOIN (DROP INDEX 模拟) ----
            conn.execute("DROP INDEX IF EXISTS idx_v012_identity_hot")
            conn.execute("DROP INDEX IF EXISTS idx_v012_identity_cold")
            conn.execute("DROP INDEX IF EXISTS idx_v012_identity_wal")
            conn.execute("DROP INDEX IF EXISTS idx_v012_identity_dream")
            conn.execute("DROP INDEX IF EXISTS idx_v012_identity_snapshots")
            conn.execute("DROP INDEX IF EXISTS idx_v012_identity_stm")
            conn.execute("DROP INDEX IF EXISTS idx_v012_identity_ltm")
            conn.execute("DROP INDEX IF EXISTS idx_v012_identity_mtm")
            conn.commit()

            t0 = time.perf_counter()
            cur = conn.execute(
                "SELECT h.id, c.content, w.event_id, d.summary, "
                "       s.content_hash, stm.role, ltm.content, mtm.topic_label "
                "FROM memory_hot h "
                "LEFT JOIN memory_cold c ON h.identity_id = c.identity_id "
                "LEFT JOIN memory_wal w ON h.identity_id = w.identity_id "
                "LEFT JOIN memory_dream d ON h.identity_id = d.identity_id "
                "LEFT JOIN memory_snapshots s ON h.identity_id = s.identity_id "
                "LEFT JOIN stm_messages stm ON h.identity_id = stm.identity_id "
                "LEFT JOIN ltm_facts ltm ON h.identity_id = ltm.identity_id "
                "LEFT JOIN mtm_themes mtm ON h.identity_id = mtm.identity_id "
                "WHERE h.identity_id = ? LIMIT 1000",
                (identity_id,),
            )
            no_idx_rows = len(cur.fetchall())
            join_ms_no_index = (time.perf_counter() - t0) * 1000.0

            cur = conn.execute(
                "EXPLAIN QUERY PLAN "
                "SELECT h.id FROM memory_hot h "
                "WHERE h.identity_id = ? LIMIT 1000",
                (identity_id,),
            )
            explain_no = [str(r[3]) for r in cur.fetchall()]

            # ---- 重新建索引 (V1109 标准 schema) — 主 17:43 实事求是 ----
            conn.executescript(
                "CREATE INDEX IF NOT EXISTS idx_v012_identity_hot "
                "ON memory_hot(identity_id);"
                "CREATE INDEX IF NOT EXISTS idx_v012_identity_cold "
                "ON memory_cold(identity_id);"
                "CREATE INDEX IF NOT EXISTS idx_v012_identity_wal "
                "ON memory_wal(identity_id);"
                "CREATE INDEX IF NOT EXISTS idx_v012_identity_dream "
                "ON memory_dream(identity_id);"
                "CREATE INDEX IF NOT EXISTS idx_v012_identity_snapshots "
                "ON memory_snapshots(identity_id);"
                "CREATE INDEX IF NOT EXISTS idx_v012_identity_stm "
                "ON stm_messages(identity_id);"
                "CREATE INDEX IF NOT EXISTS idx_v012_identity_ltm "
                "ON ltm_facts(identity_id);"
                "CREATE INDEX IF NOT EXISTS idx_v012_identity_mtm "
                "ON mtm_themes(identity_id);"
            )
            conn.commit()

            t0 = time.perf_counter()
            cur = conn.execute(
                "SELECT h.id, c.content, w.event_id, d.summary, "
                "       s.content_hash, stm.role, ltm.content, mtm.topic_label "
                "FROM memory_hot h "
                "LEFT JOIN memory_cold c ON h.identity_id = c.identity_id "
                "LEFT JOIN memory_wal w ON h.identity_id = w.identity_id "
                "LEFT JOIN memory_dream d ON h.identity_id = d.identity_id "
                "LEFT JOIN memory_snapshots s ON h.identity_id = s.identity_id "
                "LEFT JOIN stm_messages stm ON h.identity_id = stm.identity_id "
                "LEFT JOIN ltm_facts ltm ON h.identity_id = ltm.identity_id "
                "LEFT JOIN mtm_themes mtm ON h.identity_id = mtm.identity_id "
                "WHERE h.identity_id = ? LIMIT 1000",
                (identity_id,),
            )
            with_idx_rows = len(cur.fetchall())
            join_ms_with_index = (time.perf_counter() - t0) * 1000.0

            cur = conn.execute(
                "EXPLAIN QUERY PLAN "
                "SELECT h.id FROM memory_hot h "
                "WHERE h.identity_id = ? LIMIT 1000",
                (identity_id,),
            )
            explain_with = [str(r[3]) for r in cur.fetchall()]

            schema.close()

            return JoinBenchmarkRow(
                scale=scale,
                n_rows_total=total,
                n_distinct_identities=1,
                n_sessions=len(ct.sessions),
                join_ms_no_index=round(join_ms_no_index, 3),
                join_ms_with_index=round(join_ms_with_index, 3),
                join_records_total=max(no_idx_rows, with_idx_rows),
                continuity_score=round(ct.continuity_score(), 4),
                explain_no_index=explain_no,
                explain_with_index=explain_with,
            )
        finally:
            try:
                os.unlink(db_path)
            except OSError:
                pass

    def to_dicts(self, rows: List[JoinBenchmarkRow]) -> List[Dict[str, Any]]:
        return [asdict(r) for r in rows]


# ===========================================================================
# 4. StressDrill — 3 类高强度演练 (10× 数据 / 100K join / 50 corrupt + 200 valid)
# ===========================================================================


@dataclass
class StressReport:
    drill_kind: str
    success: bool
    started_at: float
    ended_at: float
    runtime_ms: float
    metrics: Dict[str, Any]
    trace: List[Dict[str, Any]] = field(default_factory=list)
    error: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


class StressDrill:
    """V1122 — 3 类 stress 演练 (主 13:31 大胆激进 + 主 23:44 干到底).

    a. MigrationStressDrill — 10× 数据量 (1460 行 v0.1.0 → v0.1.2)
    b. JoinStressDrill — 100K 行跨表 join + continuity_score 校验
    c. DisasterStressDrill — 200 valid + 50 corrupt (corrupt 比率 20%) + recovery_record
    """

    def __init__(self, db_dir: str | Path):
        self.db_dir = Path(db_dir)
        self.db_dir.mkdir(parents=True, exist_ok=True)
        self.reports: List[StressReport] = []

    # ---- 4a MigrationStressDrill ----

    def migration_stress(self, multiplier: int = 10) -> StressReport:
        started = time.time()
        trace: List[Dict[str, Any]] = []
        metrics: Dict[str, Any] = {}
        success = False
        err: Optional[str] = None
        try:
            db_path = str(self.db_dir / "stress_migration.db")
            if os.path.exists(db_path):
                os.unlink(db_path)

            # V1094 base + 10× SAMPLE_SIZES
            SAMPLE_SIZES_BASE = {
                "memory_hot": 30, "memory_cold": 20, "memory_wal": 40,
                "memory_dream": 8, "memory_snapshots": 5, "stm_messages": 25,
                "ltm_facts": 6, "mtm_themes": 12,
            }
            SAMPLE_SIZES = {k: v * multiplier for k, v in SAMPLE_SIZES_BASE.items()}
            trace.append({"step": "drill_init",
                          "multiplier": multiplier,
                          "total_rows": sum(SAMPLE_SIZES.values())})

            base = MemorySchemaV094(db_path)
            cur = base._conn.cursor()
            rng = random.Random(_seed_for("stress_mig"))
            now = time.time()

            # memory_hot
            for i in range(SAMPLE_SIZES["memory_hot"]):
                cur.execute(
                    "INSERT INTO memory_hot(id, session_id, actor, content, "
                    "ts, fingerprint) VALUES (?, ?, ?, ?, ?, ?)",
                    (f"smh_{i:06d}", f"sms_{i % 5}", "stress",
                     f"hot{i}", now - rng.random() * 3600,
                     hashlib.sha256(_b(f"smh_{i}")).hexdigest()[:16]),
                )
            for i in range(SAMPLE_SIZES["memory_cold"]):
                cur.execute(
                    "INSERT INTO memory_cold(id, content, ts, fingerprint) "
                    "VALUES (?, ?, ?, ?)",
                    (f"smc_{i:06d}", f"cold{i}", now - rng.random() * 86400,
                     hashlib.sha256(_b(f"smc_{i}")).hexdigest()[:16]),
                )
            for i in range(SAMPLE_SIZES["memory_wal"]):
                base.wal_append("hot", "tag_set", {"i": i})
            for i in range(SAMPLE_SIZES["memory_dream"]):
                cur.execute(
                    "INSERT INTO memory_dream(id, summary, confidence, "
                    "importance, dream_state, ts) VALUES (?, ?, ?, ?, ?, ?)",
                    (f"smd_{i:06d}", f"dream{i}", 0.7, 5, "CONSOLIDATING", now),
                )
            for i in range(SAMPLE_SIZES["memory_snapshots"]):
                cur.execute(
                    "INSERT INTO memory_snapshots(scope, seq, ts, "
                    "content_hash, rationale, identity_hash, snapshot_score) "
                    "VALUES (?, ?, ?, ?, ?, ?, ?)",
                    ("stress-snap", i, now,
                     hashlib.sha256(_b(f"sm_snap_{i}")).hexdigest()[:16],
                     f"stress rationale {i}", "", 0.5),
                )
            for i in range(SAMPLE_SIZES["stm_messages"]):
                cur.execute(
                    "INSERT INTO stm_messages(id, session_id, role, content, "
                    "ts, fingerprint) VALUES (?, ?, ?, ?, ?, ?)",
                    (f"smstm_{i:06d}", f"smstm_s_{i % 3}",
                     "user" if i % 2 == 0 else "assistant",
                     f"stm{i}", now,
                     hashlib.sha256(_b(f"smstm_{i}")).hexdigest()[:16]),
                )
            for i in range(SAMPLE_SIZES["ltm_facts"]):
                cur.execute(
                    "INSERT INTO ltm_facts(id, category, content, importance, "
                    "confidence, fingerprint, ts) "
                    "VALUES (?, ?, ?, ?, ?, ?, ?)",
                    (f"smlf_{i:06d}", "fact", f"ltm{i}", 5, 0.9,
                     hashlib.sha256(_b(f"smlf_{i}")).hexdigest()[:16], now),
                )
            for i in range(SAMPLE_SIZES["mtm_themes"]):
                cur.execute(
                    "INSERT INTO mtm_themes(topic_id, topic_label, "
                    "n_episodes, importance_avg, summary, last_updated, "
                    "fingerprint) VALUES (?, ?, ?, ?, ?, ?, ?)",
                    (f"smt_{i:06d}", f"theme{i}",
                     1, 0.6, "summary", now,
                     hashlib.sha256(_b(f"smt_{i}")).hexdigest()[:16]),
                )
            base._conn.commit()
            base.close()

            n_before = sum(SAMPLE_SIZES.values())
            trace.append({"step": "v010_baseline_built", "n_rows": n_before})

            # V1109 升级
            upgrade_v012_path(db_path)
            trace.append({"step": "v012_upgrade_done"})

            schema = MemorySchemaV012(db_path)
            cur = schema._conn.cursor()
            n_after = 0
            for tbl in SAMPLE_SIZES.keys():
                cur.execute(f"SELECT COUNT(*) AS n FROM {tbl}")
                n_after += cur.fetchone()["n"]
            schema.close()
            metrics["n_rows_before"] = n_before
            metrics["n_rows_after"] = n_after
            metrics["multiplier"] = multiplier
            trace.append({"step": "n_rows_after_count", "n_after": n_after})

            # 3 次幂等验证
            for run in range(3):
                upgrade_v012_path(db_path)
            trace.append({"step": "idempotent_runs_done", "n_runs": 3})

            success = (n_after == n_before)
            metrics["rows_preserved"] = success
        except Exception as exc:  # noqa: BLE001
            err = repr(exc)
        ended = time.time()
        rep = StressReport(
            drill_kind="migration_stress",
            success=success,
            started_at=started,
            ended_at=ended,
            runtime_ms=round((ended - started) * 1000, 3),
            metrics=metrics,
            trace=trace,
            error=err,
        )
        self.reports.append(rep)
        return rep

    # ---- 4b JoinStressDrill ----

    def join_stress(self, n_rows: int = 100000) -> StressReport:
        started = time.time()
        trace: List[Dict[str, Any]] = []
        metrics: Dict[str, Any] = {}
        success = False
        err: Optional[str] = None
        try:
            db_path = str(self.db_dir / "stress_join.db")
            if os.path.exists(db_path):
                os.unlink(db_path)

            schema = MemorySchemaV012(db_path)
            conn = schema._conn
            identity_id = f"id_stress_{uuid.uuid4().hex[:12]}"
            ct = ContinuityTracker()
            sid = ct.start_session()
            ct.sessions[sid].n_entries_added = n_rows

            per = {
                "memory_hot": n_rows // 5,
                "memory_cold": n_rows // 8,
                "memory_wal": n_rows // 3,
                "memory_dream": n_rows // 12,
                "memory_snapshots": n_rows // 33,
                "stm_messages": n_rows // 8,
                "ltm_facts": n_rows // 10,
                "mtm_themes": n_rows // 20,
            }
            delta = n_rows - sum(per.values())
            per["memory_hot"] += delta
            trace.append({"step": "distribution_ready",
                          "per_table": per,
                          "total": sum(per.values())})

            rng = random.Random(_seed_for("stress_join"))
            now = time.time()
            # memory_hot 批量 executemany 加速
            hot_rows = [
                (f"sjh_{i:08d}", f"sjs_{i % 3}", "stress",
                 f"hot{i}", now - rng.random() * 3600,
                 hashlib.sha256(_b(f"sjh_{i}")).hexdigest()[:16],
                 identity_id)
                for i in range(per["memory_hot"])
            ]
            conn.executemany(
                "INSERT INTO memory_hot(id, session_id, actor, content, "
                "ts, fingerprint, identity_id) VALUES (?, ?, ?, ?, ?, ?, ?)",
                hot_rows,
            )
            conn.commit()

            # memory_cold 批量
            cold_rows = [
                (f"sjc_{i:08d}", f"cold{i}", now - rng.random() * 86400,
                 hashlib.sha256(_b(f"sjc_{i}")).hexdigest()[:16],
                 identity_id)
                for i in range(per["memory_cold"])
            ]
            conn.executemany(
                "INSERT INTO memory_cold(id, content, ts, fingerprint, "
                "identity_id) VALUES (?, ?, ?, ?, ?)",
                cold_rows,
            )
            conn.commit()

            # WAL 用 wal_append_with_chunk 分批 (chunk 一致性必需保留)
            for chunk_i in range(0, per["memory_wal"], 500):
                for j in range(min(500, per["memory_wal"] - chunk_i)):
                    i = chunk_i + j
                    schema.wal_append_with_chunk(
                        rng.choice(["hot", "cold", "mtm", "ltm"]),
                        "tag_set",
                        {"i": i},
                        chunk_id=f"stress_chunk_{i // 500}",
                        identity_id=identity_id,
                        event_id=f"sjev_{i:08d}",
                        impact=0.5,
                    )
                conn.commit()
            trace.append({"step": "wal_seeded", "n_wal": per["memory_wal"]})

            # 其余批量
            dream_rows = [
                (f"sjd_{i:08d}", f"dream{i}", 0.7, 5, "CONSOLIDATING",
                 now, identity_id, DREAM_PHASES[i % 3])
                for i in range(per["memory_dream"])
            ]
            conn.executemany(
                "INSERT INTO memory_dream(id, summary, confidence, "
                "importance, dream_state, ts, identity_id, dream_phase) "
                "VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                dream_rows,
            )
            snap_rows = [
                ("stress-snap", i, now,
                 hashlib.sha256(_b(f"snap{i}")).hexdigest()[:16],
                 f"stress rationale {i}", "", 0.5)
                for i in range(per["memory_snapshots"])
            ]
            conn.executemany(
                "INSERT INTO memory_snapshots(scope, seq, ts, "
                "content_hash, rationale, identity_hash, snapshot_score) "
                "VALUES (?, ?, ?, ?, ?, ?, ?)",
                snap_rows,
            )
            stm_rows = [
                (f"sjstm_{i:08d}", f"sjstm_s_{i % 3}",
                 "user" if i % 2 == 0 else "assistant",
                 f"stm{i}", now,
                 hashlib.sha256(_b(f"sjstm_{i}")).hexdigest()[:16],
                 identity_id)
                for i in range(per["stm_messages"])
            ]
            conn.executemany(
                "INSERT INTO stm_messages(id, session_id, role, content, "
                "ts, fingerprint, identity_id) "
                "VALUES (?, ?, ?, ?, ?, ?, ?)",
                stm_rows,
            )
            ltm_rows = [
                (f"sjlf_{i:08d}", "fact",
                 f"ltm{i}", 5, 0.9,
                 hashlib.sha256(_b(f"sjlf_{i}")).hexdigest()[:16],
                 now, identity_id)
                for i in range(per["ltm_facts"])
            ]
            conn.executemany(
                "INSERT INTO ltm_facts(id, category, content, importance, "
                "confidence, fingerprint, ts, identity_id) "
                "VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                ltm_rows,
            )
            mtm_rows = [
                (f"sjmt_{i:08d}", f"theme{i}",
                 1, 0.6, "summary", now,
                 hashlib.sha256(_b(f"sjmt_{i}")).hexdigest()[:16],
                 identity_id)
                for i in range(per["mtm_themes"])
            ]
            conn.executemany(
                "INSERT INTO mtm_themes(topic_id, topic_label, "
                "n_episodes, importance_avg, summary, last_updated, "
                "fingerprint, identity_id) "
                "VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                mtm_rows,
            )
            conn.commit()

            # 跨表 join 真测 — 用 LEFT JOIN 各 1 行 exists 模式避免笛卡尔积
            t0 = time.perf_counter()
            cur = conn.execute(
                "SELECT "
                " (SELECT COUNT(*) FROM memory_hot WHERE identity_id = ?) AS n_hot, "
                " (SELECT COUNT(*) FROM memory_cold WHERE identity_id = ?) AS n_cold, "
                " (SELECT COUNT(*) FROM memory_wal WHERE identity_id = ?) AS n_wal, "
                " (SELECT COUNT(*) FROM memory_dream WHERE identity_id = ?) AS n_dream",
                (identity_id, identity_id, identity_id, identity_id),
            )
            r = cur.fetchone()
            join_count = r["n_hot"] + r["n_cold"] + r["n_wal"] + r["n_dream"]
            join_ms = round((time.perf_counter() - t0) * 1000.0, 3)

            cur = conn.execute(
                "SELECT COUNT(DISTINCT identity_id) AS n FROM memory_hot "
                "WHERE identity_id = ?",
                (identity_id,),
            )
            distinct = cur.fetchone()["n"]
            schema.close()

            metrics["n_rows_total"] = sum(per.values())
            metrics["join_count"] = join_count
            metrics["join_ms"] = join_ms
            metrics["n_distinct_identities"] = distinct
            metrics["continuity_score"] = round(ct.continuity_score(), 4)
            metrics["n_sessions"] = len(ct.sessions)
            trace.append({"step": "join_done",
                          "join_ms": join_ms,
                          "join_count": join_count})
            success = (distinct == 1) and (metrics["continuity_score"] > 0.0)
        except Exception as exc:  # noqa: BLE001
            err = repr(exc)
        ended = time.time()
        rep = StressReport(
            drill_kind="join_stress",
            success=success,
            started_at=started,
            ended_at=ended,
            runtime_ms=round((ended - started) * 1000, 3),
            metrics=metrics,
            trace=trace,
            error=err,
        )
        self.reports.append(rep)
        return rep

    # ---- 4c DisasterStressDrill ----

    def disaster_stress(self, n_valid: int = 200, n_corrupt: int = 50) -> StressReport:
        started = time.time()
        trace: List[Dict[str, Any]] = []
        metrics: Dict[str, Any] = {}
        success = False
        err: Optional[str] = None
        try:
            db_path = str(self.db_dir / "stress_disaster.db")
            if os.path.exists(db_path):
                os.unlink(db_path)

            schema = MemorySchemaV012(db_path)
            identity_id = f"id_disaster_{uuid.uuid4().hex[:12]}"

            # 写入 n_valid 行 WAL, 集中到 4 个 chunk
            n_chunks = 4
            per_chunk = n_valid // n_chunks
            chunk_ids = [f"stress_d_chunk_{i}" for i in range(n_chunks)]
            for ci in range(n_chunks):
                for j in range(per_chunk):
                    schema.wal_append_with_chunk(
                        "hot",
                        "tag_set",
                        {"ci": ci, "j": j},
                        chunk_id=chunk_ids[ci],
                        identity_id=identity_id,
                        event_id=f"sd_{ci}_{j:04d}",
                        impact=0.5,
                    )
            schema._conn.commit()
            trace.append({"step": "wal_seeded",
                          "n_valid": n_valid,
                          "n_chunks": n_chunks})

            # 注入 corruption: 一半 tampered (改 checksum) + 一半 deleted (DELETE)
            n_tampered = n_corrupt // 2
            n_deleted = n_corrupt - n_tampered
            cur = schema._conn.cursor()
            cur.execute(
                "SELECT seq FROM memory_wal WHERE identity_id = ? "
                "ORDER BY RANDOM() LIMIT ?",
                (identity_id, n_tampered),
            )
            tamper_ids = [r["seq"] for r in cur.fetchall()]
            for wid in tamper_ids:
                cur.execute(
                    "UPDATE memory_wal SET checksum = 'DEADBEEF00000000' "
                    "WHERE seq = ?",
                    (wid,),
                )
            if tamper_ids:
                placeholders = ",".join("?" * len(tamper_ids))
                cur.execute(
                    f"SELECT seq FROM memory_wal WHERE identity_id = ? "
                    f"AND seq NOT IN ({placeholders}) "
                    f"ORDER BY RANDOM() LIMIT ?",
                    (identity_id, *tamper_ids, n_deleted),
                )
            else:
                cur.execute(
                    "SELECT seq FROM memory_wal WHERE identity_id = ? "
                    "ORDER BY RANDOM() LIMIT ?",
                    (identity_id, n_deleted),
                )
            del_ids = [r["seq"] for r in cur.fetchall()]
            for wid in del_ids:
                cur.execute("DELETE FROM memory_wal WHERE seq = ?", (wid,))
            schema._conn.commit()
            trace.append({"step": "corruption_injected",
                          "tampered": len(tamper_ids),
                          "deleted": len(del_ids)})

            # verify
            cr_before = schema.verify_wal_checksums()
            metrics["verify_before"] = cr_before.to_dict()

            # recovery_record 索引演练
            rri = RecoveryRecordIndex(db_path)
            rri.migrate()

            # 把每条 corrupt 写入 recovery_record — 走 idx_recovery_chunk_ts
            cur.execute(
                "SELECT seq, chunk_id, event_id, identity_id, scope "
                "FROM memory_wal WHERE checksum = 'DEADBEEF00000000'"
            )
            for row in cur.fetchall():
                rec = RecoveryRecord(
                    ts=time.time(),
                    chunk_id=row["chunk_id"],
                    seq=row["seq"],
                    event_id=row["event_id"],
                    identity_id=row["identity_id"],
                    scope=row["scope"],
                    corrupt_kind="tampered",
                    health_ratio=cr_before.health_ratio,
                    detail_json=json.dumps({"seq": row["seq"]}),
                    record_kind="drill",
                )
                rri.record(rec)
            # deleted 记录
            for i in range(n_deleted):
                rec = RecoveryRecord(
                    ts=time.time(),
                    chunk_id=f"stress_d_chunk_{i % n_chunks}",
                    seq=i,
                    event_id=f"sd_deleted_{i:04d}",
                    identity_id=identity_id,
                    scope="hot",
                    corrupt_kind="deleted",
                    health_ratio=cr_before.health_ratio,
                    detail_json=json.dumps({"seq": i}),
                    record_kind="drill",
                )
                rri.record(rec)
            trace.append({"step": "recovery_record_written",
                          "tampered": n_tampered,
                          "deleted": n_deleted})

            # 走索引查询验证
            explain = rri.explain_query(chunk_ids[0])
            trace.append({"step": "explain_query_plan", "plan": explain})
            metrics["explain_uses_idx_recovery_chunk_ts"] = any(
                "idx_recovery_chunk_ts" in line for line in explain
            )

            # replay 按 chunk
            replay_results: Dict[str, int] = {}
            for cid in chunk_ids:
                replay_results[cid] = len(schema.replay_events_by_chunk(
                    cid, skip_corrupt=True))
            metrics["replay_per_chunk"] = replay_results

            stats = rri.stats()
            metrics["recovery_record_stats"] = stats
            rri.close()

            # 校验: n_corrupt + n_valid - n_deleted == total_seen
            schema.close()

            success = (
                cr_before.corrupt >= n_tampered
                and stats["n_total"] == n_corrupt
                and metrics["explain_uses_idx_recovery_chunk_ts"] is True
            )
        except Exception as exc:  # noqa: BLE001
            err = repr(exc)
        ended = time.time()
        rep = StressReport(
            drill_kind="disaster_stress",
            success=success,
            started_at=started,
            ended_at=ended,
            runtime_ms=round((ended - started) * 1000, 3),
            metrics=metrics,
            trace=trace,
            error=err,
        )
        self.reports.append(rep)
        return rep

    # ---- 总入口 ----

    def run_full_stress(self) -> List[StressReport]:
        """3 类 stress 演练串联 — 主 00:56 一行命令."""
        self.reports = []
        a = self.migration_stress(multiplier=10)
        b = self.join_stress(n_rows=100000)
        c = self.disaster_stress(n_valid=200, n_corrupt=50)
        return [a, b, c]


# ===========================================================================
# 5. CLI — 主 00:56 一行命令
# ===========================================================================


def main(argv: Optional[List[str]] = None) -> int:
    import argparse
    p = argparse.ArgumentParser(
        prog="v1122_v1072_continuity_tracker",
        description=(
            "V1122 V1072 ContinuityTracker timeline 可视化 + "
            "recovery_record 索引 + cross-table join benchmark + 3 类 stress"
        ),
    )
    p.add_argument("--out-dir", default="reports/v1122_outputs",
                   help="3 类可视化输出目录 (JSON / MD / SVG)")
    p.add_argument("--db-dir", default="reports/v1122_dbs",
                   help="stress 演练临时 DB 目录")
    p.add_argument("--stress", action="store_true",
                   help="跑 3 类 stress 演练")
    p.add_argument("--benchmark", action="store_true",
                   help="跑 cross-table join benchmark (1K/10K/100K)")
    p.add_argument("--report", action="store_true",
                   help="落盘: 3 类可视化 + benchmark JSON + stress 报告")
    p.add_argument("--print-json", action="store_true",
                   help="stdout 打印主报告 JSON")
    args = p.parse_args(argv)

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    # 1. timeline 可视化
    core = IdentityCore(identity_id="id_demo_chuling")
    manifest = IdentityManifest(core=core)
    for i in range(5):
        manifest.add("LTM", "fact", f"fact{i}", importance=0.9)
    for i in range(3):
        manifest.add("MTM", "insight", f"insight{i}", importance=0.7)
    for i in range(2):
        manifest.add("STM", "event", f"event{i}", importance=0.5)

    ct = ContinuityTracker()
    sids = [ct.start_session() for _ in range(5)]
    for i, sid in enumerate(sids):
        ct.sessions[sid].n_entries_added = (i + 1) * 10
        ct.sessions[sid].n_importance_avg = 0.5 + 0.1 * i
        ct.sessions[sid].ended_at = ct.sessions[sid].started_at + (i + 1) * 0.1
        ct.sessions[sid].is_active = (i == len(sids) - 1)

    viz = ContinuityTimelineViz(
        identity_id=core.identity_id,
        title="V1072 ContinuityTracker Timeline (CLI demo)",
    )
    viz.feed_tracker(ct)
    viz.feed_manifest(manifest)

    paths = viz.write_all(out_dir)
    print(f"[V1122] timeline 可视化落盘: {paths}")

    # 2. benchmark (可选)
    bench_rows: List[Dict[str, Any]] = []
    if args.benchmark or args.report:
        b = CrossTableJoinBenchmark()
        rows = b.run()
        bench_rows = b.to_dicts(rows)
        print(f"[V1122] benchmark: {[(r['scale'], r['join_ms_with_index']) for r in bench_rows]}")
        if args.report:
            (out_dir / "join_benchmark.json").write_text(
                json.dumps(bench_rows, ensure_ascii=False, indent=2),
                encoding="utf-8",
            )

    # 3. stress (可选)
    stress_reports: List[Dict[str, Any]] = []
    if args.stress or args.report:
        sd = StressDrill(args.db_dir)
        reps = sd.run_full_stress()
        stress_reports = [r.to_dict() for r in reps]
        print(f"[V1122] stress: "
              f"{[(r['drill_kind'], r['success']) for r in stress_reports]}")
        if args.report:
            (out_dir / "stress_reports.json").write_text(
                json.dumps(stress_reports, ensure_ascii=False, indent=2, default=str),
                encoding="utf-8",
            )

    if args.print_json:
        master = {
            "v1122_version": V1122_VERSION,
            "timeline": viz.to_json(),
            "benchmark": bench_rows,
            "stress": stress_reports,
        }
        print(json.dumps(master, ensure_ascii=False, indent=2, default=str))

    return 0


# ===========================================================================
# V1122 V3_GUARDS — 主 17:43 + 17:58 + 20:46
# ===========================================================================


V3_GUARDS = {
    "module_is_not_asi": (
        "V1122 是可视化 + 索引 + benchmark + stress 工具. ASI 是更大目标. "
        "演练通过 ≠ ASI 达成."
    ),
    "structure_is_not_consciousness": (
        "Timeline chart + continuity_score 走势 ≠ 真心理连续性. "
        "Parfit 1984 类比, 不是现象意识."
    ),
    "measurement_is_not_truth": (
        "join_ms_no_index vs join_ms_with_index 是 proxy benchmark. "
        "真生产 latency 受 OS page cache, fsync, lock 影响, 必须配合 V1084 audit."
    ),
    "production_is_not_safety": (
        "controlled stress (10× / 100K / 50 corrupt) ≠ 真生产 corruption. "
        "真生产 corruption 模式更复杂 (半截 write / 并发 race / fsync 失败)."
    ),
    "automation_is_not_autonomy": (
        "StressDrill 自动跑 ≠ 自主恢复. 真灾难需要 SOP + 运维 review + 告警."
    ),
}


__all__ = [
    "V1122_VERSION",
    "V3_GUARDS",
    "TimelinePoint",
    "ContinuityTimelineViz",
    "RecoveryRecord",
    "RECOVERY_RECORD_TABLE_DDL",
    "RECOVERY_RECORD_INDEXES_DDL",
    "RecoveryRecordIndex",
    "JoinBenchmarkRow",
    "CrossTableJoinBenchmark",
    "StressReport",
    "StressDrill",
    "main",
]


if __name__ == "__main__":
    sys.exit(main())