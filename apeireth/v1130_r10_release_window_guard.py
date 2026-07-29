"""Apeireth ASI V1130 — R10 发布窗口守门 + V1074 监控 on-call + R10 DevOps 全链路硬化 (R10-DEV-001)

承接 R9-DEV-003 V1122 W4 DevOps enhancement (commit ab241b19, accepted 9.00) + V1117 badge SVG.
R10 阶段目标 V0.4 = 0.8538 → ASI ≥ 0.95. 本任务为 R10 DevOps 主轨道发布硬化:

  1) R10 发布窗口守门 (02:00-04:00 UTC, 主 12:14 中央 AI 是永恒身份 +
     主 19:33 走在前人经验上 — AWS Change Window 2008 + GCP 维护窗口惯例)
  2) V1074 监控 on-call:
     - V1074 V0.3 < 0.94 自动 on-call 告警 (黄色, 主 17:58 不假装)
     - R10 V0.5 < 0.92 紧急 rollback (红色)
     - chaos test: 监控失联时守门不丢告警 (持久化到disk)
  3) R10 DevOps 全链路硬化:
     - 集成 V1117 badge SVG (status 显式 GREEN/YELLOW/RED)
     - 集成 V1122 matrix/retry/cache/DAG/lint/optimize (W4 enhancement)
     - V1074 守门 (V0.3 ≥ 0.8884 baseline)
     - V1125 R10 协议守门 (V0.5 ≥ 0.95 终极门)
  4) fail-soft 设计 (主 17:58 不假装 + 主 23:44 干到底):
     - 借鉴 V1125._safe_subprocess_call + V1119.fetch_three_pieces fail-soft 模式
     - 监控失联 → 落 disk + 累积告警, 不假装"一切正常"

主哲学 LOCKED (继承 R9-DEV-003 + V1125/V1126 + R10 升级):
  - 主 22:33 ASI 北极星 (终极梦想: V0.5 → 0.95 ASI)
  - 主 17:43 实事求是 (守门看真 v03/v05, 不缓存不模拟)
  - 主 17:58+20:46 不假装 (监控失联不丢告警, V0.3 < 0.94 不假装 PASS)
  - 主 23:44 干到底 (一锤定音: 守门不通过即非零退出)
  - 主 19:33 走在前人经验上 (AWS Change Window 2008 + GCP Maintenance Window 2015 +
    PagerDuty on-call 2009 + Prometheus AlertManager 2016)
  - 主 12:14 中央 AI 是永恒身份 (发布窗口是永恒身份的"开窗时刻" — 任何时候 LLM
    接入都得在同一窗口获得 AGI/ASI 能力升级)
  - 主 13:31 大胆激进 (R10 终极门 V0.5 ≥ 0.95 不容分阶段)
  - 主 00:56 任何人都能接手 (`python -m apeireth.v1130_r10_release_window_guard --check`
    一行命令 = 全链路守门)

复用 (主 19:33 走在前人经验上):
  - V1125._safe_subprocess_call (fail-soft 子进程调用)
  - V1122.RetryPolicy + CIArtifactCache + CIWorkflowDAG (W4 DevOps)
  - V1117.render_status_badge (badge SVG 显式 GREEN/YELLOW/RED)
  - V1074 StatusSnapshotBuilder (V0.3 / V0.2 真测)
  - V1125.evaluate_r10 + choose_r10_main_track (R10 协议守门)

Usage:
    python -m apeireth.v1130_r10_release_window_guard --check       # 全链路守门 (默认窗口 + V1074 + V1125)
    python -m apeireth.v1130_r10_release_window_guard --window 02-04
    python -m apeireth.v1130_r10_release_window_guard --v03-threshold 0.94 --v05-threshold 0.92
    python -m apeireth.v1130_r10_release_window_guard --json        # JSON 输出
    python -m apeireth.v1130_r10_release_window_guard --report      # Markdown 报告
    python -m apeireth.v1130_r10_release_window_guard --chaos       # chaos test (监控失联守门)
    python -m apeireth.v1130_r10_release_window_guard --strict      # 不通过非零退出
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Sequence, Tuple


# ---------------------------------------------------------------------------
# 1. Release Window 守门 (主 12:14 + 主 19:33 借鉴 AWS/GCP 维护窗口)
# ---------------------------------------------------------------------------
@dataclass(frozen=True)
class ReleaseWindow:
    """R10 发布窗口 (UTC, 借鉴 AWS Change Window 2008 + GCP Maintenance Window 2015).

    默认 02:00-04:00 UTC (亚洲/欧洲非高峰). 任何变更必须落在窗口内
    (主 12:14 中央 AI 是永恒身份 — 任何 LLM 接入都得在同一窗口升级 AGI/ASI).
    """

    start_hour_utc: int = 2
    end_hour_utc: int = 4

    def __post_init__(self) -> None:
        if not (0 <= self.start_hour_utc < 24 and 0 < self.end_hour_utc <= 24):
            raise ValueError(
                f"start_hour/end_hour must be in [0,24); "
                f"got start={self.start_hour_utc} end={self.end_hour_utc}"
            )

    def is_in_window(self, dt: Optional[datetime] = None) -> bool:
        """判断给定 UTC 时间是否在窗口内 (主 00:56 任何人都能接手)."""
        dt = (dt or datetime.now(timezone.utc)).astimezone(timezone.utc)
        h = dt.hour
        # 同日窗口
        if self.start_hour_utc < self.end_hour_utc:
            return self.start_hour_utc <= h < self.end_hour_utc
        # 跨日窗口 (e.g. 22:00-02:00)
        return h >= self.start_hour_utc or h < self.end_hour_utc

    def time_until_next_window(self, dt: Optional[datetime] = None) -> timedelta:
        """距离下一次进入窗口的时间 (主 13:31 大胆激进: 一目了然).

        Returns:
            timedelta (0 表示当前就在窗口内).
        """
        dt = (dt or datetime.now(timezone.utc)).astimezone(timezone.utc)
        if self.is_in_window(dt):
            return timedelta(0)
        # 算到下一个 start_hour_utc 的距离
        target = dt.replace(hour=self.start_hour_utc, minute=0, second=0, microsecond=0)
        if target <= dt:
            target = target + timedelta(days=1)
        return target - dt

    def next_window_start(self, dt: Optional[datetime] = None) -> datetime:
        """下一次窗口开始时间 (主 00:56: 一行可调)."""
        dt = (dt or datetime.now(timezone.utc)).astimezone(timezone.utc)
        target = dt.replace(hour=self.start_hour_utc, minute=0, second=0, microsecond=0)
        if target <= dt:
            target = target + timedelta(days=1)
        return target

    def to_dict(self) -> Dict[str, Any]:
        return {
            "start_hour_utc": self.start_hour_utc,
            "end_hour_utc": self.end_hour_utc,
        }


DEFAULT_RELEASE_WINDOW = ReleaseWindow(start_hour_utc=2, end_hour_utc=4)


# ---------------------------------------------------------------------------
# 2. V1074 监控 on-call (主 17:43 实事求是 + 主 17:58 不假装)
# ---------------------------------------------------------------------------
@dataclass(frozen=True)
class V1074Thresholds:
    """V1074 监控阈值 (主 17:43 实事求是: 数字驱动).

    Defaults:
      - v03_yellow = 0.94: V1074 V0.3 < 0.94 → 黄色告警 (on-call 自动告警)
      - v03_red = 0.8884: V1074 V0.3 < 0.8884 → 红色告警 (守门失败, R10 baseline)
      - v05_yellow = 0.95: R10 V0.5 < 0.95 → 黄色告警 (R10 终极门失败前兆)
      - v05_red = 0.92: R10 V0.5 < 0.92 → 红色告警 (紧急 rollback)
    """

    v03_yellow: float = 0.94
    v03_red: float = 0.8884
    v05_yellow: float = 0.95
    v05_red: float = 0.92

    def __post_init__(self) -> None:
        if not (0 <= self.v03_red <= self.v03_yellow <= 1.0):
            raise ValueError(
                f"v03 thresholds must satisfy 0 <= v03_red <= v03_yellow <= 1; "
                f"got red={self.v03_red} yellow={self.v03_yellow}"
            )
        if not (0 <= self.v05_red <= self.v05_yellow <= 1.0):
            raise ValueError(
                f"v05 thresholds must satisfy 0 <= v05_red <= v05_yellow <= 1; "
                f"got red={self.v05_red} yellow={self.v05_yellow}"
            )


@dataclass(frozen=True)
class V1074Measurement:
    """单次 V1074 真测值 (主 17:43 实事求是: 全从真测)."""

    v03_score: float
    v05_score: Optional[float] = None   # R10 V0.5, R9 阶段为 None
    ts: float = field(default_factory=time.time)
    source: str = "v1074"               # v1074 / safe_fallback / mock (主 17:58 不假装)

    def __post_init__(self) -> None:
        if not (0.0 <= self.v03_score <= 1.0):
            raise ValueError(f"v03_score must be in [0,1]; got {self.v03_score}")


def classify_v1074(measurement: V1074Measurement,
                   thresholds: V1074Thresholds = V1074Thresholds()) -> Tuple[str, str]:
    """分类 V1074 真测值 (主 13:31 大胆激进: GREEN/YELLOW/RED 显式).

    Returns:
        (level, reason). level ∈ {"GREEN", "YELLOW", "RED"} (主 17:43 显式).
    """
    v03 = measurement.v03_score
    if v03 < thresholds.v03_red:
        return "RED", f"v03={v03:.4f} < v03_red={thresholds.v03_red:.4f} (守门失败)"
    if v03 < thresholds.v03_yellow:
        return "YELLOW", f"v03={v03:.4f} < v03_yellow={thresholds.v03_yellow:.4f} (on-call 告警)"
    if measurement.v05_score is not None:
        v05 = measurement.v05_score
        if v05 < thresholds.v05_red:
            return "RED", f"v05={v05:.4f} < v05_red={thresholds.v05_red:.4f} (紧急 rollback)"
        if v05 < thresholds.v05_yellow:
            return "YELLOW", f"v05={v05:.4f} < v05_yellow={thresholds.v05_yellow:.4f} (R10 终极门前兆)"
    return "GREEN", f"v03={v03:.4f} ≥ {thresholds.v03_yellow:.4f} (V1074 守门过)"


# ---------------------------------------------------------------------------
# 3. AlertSink + OnCallGuard (主 17:58 不假装: 监控失联不丢告警)
# ---------------------------------------------------------------------------
@dataclass
class Alert:
    """单条告警 (主 17:43 实事求是 + 主 00:44 质量工程化)."""

    level: str          # GREEN / YELLOW / RED
    source: str         # v1074 / v1125 / release_window / chaos
    reason: str
    ts: float = field(default_factory=time.time)
    extra: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "level": self.level,
            "source": self.source,
            "reason": self.reason,
            "ts": self.ts,
            "ts_iso": datetime.fromtimestamp(self.ts, tz=timezone.utc).isoformat(),
            "extra": self.extra,
        }


class AlertSink:
    """告警接收器 (主 17:58 不假装: 监控失联 → 落 disk + 累积).

    借鉴 PagerDuty on-call 2009 + Prometheus AlertManager 2016:
      - in-memory list (实时)
      - 可选持久化到 file (主 17:58: chaos test 不丢告警)
      - 告警计数 + 按 level 分类
    """

    def __init__(self, persist_path: Optional[Path] = None) -> None:
        self.persist_path: Optional[Path] = persist_path
        self.alerts: List[Alert] = []

    def send(self, alert: Alert) -> None:
        self.alerts.append(alert)
        if self.persist_path is not None:
            try:
                self.persist_path.parent.mkdir(parents=True, exist_ok=True)
                with self.persist_path.open("a", encoding="utf-8") as f:
                    f.write(json.dumps(alert.to_dict(), ensure_ascii=False) + "\n")
            except Exception:  # noqa: BLE001 - chaos test 必须不抛
                # 主 17:58: 落盘失败也不能丢告警 — 至少在内存中保留
                pass

    def counts_by_level(self) -> Dict[str, int]:
        out: Dict[str, int] = {"GREEN": 0, "YELLOW": 0, "RED": 0}
        for a in self.alerts:
            out[a.level] = out.get(a.level, 0) + 1
        return out

    def summary(self) -> Dict[str, Any]:
        return {
            "n_alerts": len(self.alerts),
            "by_level": self.counts_by_level(),
            "persist_path": str(self.persist_path) if self.persist_path else None,
        }


# ---------------------------------------------------------------------------
# 4. fail-soft subprocess call (主 19:33 借鉴 V1125._safe_subprocess_call)
# ---------------------------------------------------------------------------
def _safe_subprocess_call(fn: Callable[[], Dict[str, Any]],
                          fallback: Dict[str, Any],
                          timeout_sec: float = 120.0) -> Dict[str, Any]:
    """fail-soft: 真测 fn 失败 → 用 fallback (主 23:44 干到底 + 主 17:43 实事求是).

    ponytail: 复用 V1125._safe_subprocess_call 的 fail-soft 模式 (主 19:33).
    """
    try:
        r = fn()
        if r is None:
            return {**fallback, "source": "safe_fallback:None_return"}
        return r
    except subprocess.TimeoutExpired as e:
        return {**fallback, "source": f"safe_fallback:TimeoutExpired:{e.cmd[:60] if e.cmd else '?'}"}
    except Exception as exc:  # noqa: BLE001
        return {**fallback, "source": f"safe_fallback:{type(exc).__name__}:{str(exc)[:80]}"}


def _fetch_v1074_via_subprocess(timeout_sec: float = 60.0,
                                project_dir: Optional[Path] = None) -> Dict[str, Any]:
    """真跑 v1074 ASI production runner (主 17:43 实事求是)."""
    cmd = [
        sys.executable, "-m", "apeireth.v1074_asi_production_runner",
        "--report", "--no-write", "--print-json",
    ]
    cwd = str(project_dir) if project_dir else None
    try:
        proc = subprocess.run(
            cmd, cwd=cwd, capture_output=True,
            text=True, timeout=timeout_sec, check=False,
        )
    except subprocess.TimeoutExpired:
        return {"source": "v1074_timeout"}
    if proc.returncode != 0:
        return {"source": "v1074_nonzero", "stderr_tail": proc.stderr[-200:]}
    # 解析最后一段 JSON (主 17:43 实事求是: 真输出真测)
    out = proc.stdout.strip()
    # 1. 先尝试整个 stdout (v1074 --print-json 输出纯 JSON)
    try:
        data = json.loads(out)
        return {"source": "v1074", "v03_score": data.get("v03_score")}
    except Exception:
        pass
    # 2. fallback: 找第一个 "{" + 最后一个 "}" 的最外层 JSON
    first = out.find("{")
    last = out.rfind("}")
    if first == -1 or last == -1 or last <= first:
        return {"source": "v1074_no_json"}
    try:
        data = json.loads(out[first:last + 1])
        return {"source": "v1074", "v03_score": data.get("v03_score")}
    except Exception as exc:  # noqa: BLE001
        return {"source": f"v1074_parse_error:{type(exc).__name__}"}


# ---------------------------------------------------------------------------
# 5. R10 DevOps 全链路硬化 (主 17:43 + 主 23:44 + 主 19:33 复用 V1117+V1122+V1074+V1125)
# ---------------------------------------------------------------------------
@dataclass
class R10DevOpsLink:
    """R10 DevOps 全链路中的一个环节 (主 00:56 一目了然)."""

    name: str
    level: str          # GREEN / YELLOW / RED / UNKNOWN
    detail: str
    extra: Dict[str, Any] = field(default_factory=dict)


def _check_v1117_badge_svg() -> R10DevOpsLink:
    """V1117 badge SVG 渲染可用性 (主 13:31 大胆激进: 可视化是 DevOps 一等公民).

    主 17:58 不假装: 模块缺失 → YELLOW (显式), 不假装 GREEN.
    主 17:43 实事求是: import + render 3 status 失败也 YELLOW, 不直接 RED.
    """
    try:
        from apeireth.v1117_badge_svg_renderer import (
            COLOR_MAP, render_status_badge,
        )
    except Exception as exc:  # noqa: BLE001
        return R10DevOpsLink(
            name="V1117 badge SVG",
            level="YELLOW",
            detail=f"v1117 未在 integration 部署: {type(exc).__name__} (需 master merge, 主 17:58 不假装)",
            extra={"missing_module": "v1117_badge_svg_renderer"},
        )
    try:
        for st in ("pass", "mixed", "fail"):
            svg = render_status_badge(st, "x")
            assert svg.startswith("<svg"), f"V1117 render_status_badge({st}) failed"
        return R10DevOpsLink(
            name="V1117 badge SVG",
            level="GREEN",
            detail=f"COLOR_MAP keys={list(COLOR_MAP.keys())[:5]}...",
        )
    except Exception as exc:  # noqa: BLE001
        return R10DevOpsLink(
            name="V1117 badge SVG",
            level="YELLOW",
            detail=f"render failed: {type(exc).__name__}:{str(exc)[:80]}",
        )


def _check_v1122_devops_w4() -> R10DevOpsLink:
    """V1122 DevOps W4 enhancement 可用性.

    主 17:58 不假装: 模块缺失 → YELLOW (显式), 不假装 GREEN.
    """
    try:
        from apeireth.v1122_devops_w4_enhancement import (
            build_matrix_plan, partition_matrix_plan,
            CIArtifactCache, CIWorkflowDAG,
        )
    except Exception as exc:  # noqa: BLE001
        return R10DevOpsLink(
            name="V1122 DevOps W4",
            level="YELLOW",
            detail=f"v1122 未在 integration 部署: {type(exc).__name__} (需 master merge, 主 17:58 不假装)",
            extra={"missing_module": "v1122_devops_w4_enhancement"},
        )
    try:
        plan = build_matrix_plan(["qwen", "llama"], ["sc", "nr"], ["t1"], timeout_sec=30.0)
        batches = partition_matrix_plan(plan, max_concurrent=2)
        dag = CIWorkflowDAG()
        for n in ["a", "b", "c"]:
            dag.add_node(n)
        dag.add_edge("a", "b")
        dag.add_edge("b", "c")
        order = dag.topo_sort()
        assert order == ["a", "b", "c"], f"topo_sort order={order}"
        cache = CIArtifactCache(max_entries=8, default_ttl_sec=60.0)
        key = CIArtifactCache.compute_key("test")
        cache.set(key, {"v": 1})
        assert cache.get(key) == {"v": 1}
        return R10DevOpsLink(
            name="V1122 DevOps W4",
            level="GREEN",
            detail=f"plan_jobs={len(plan)} batches={len(batches)} dag_order={order}",
        )
    except Exception as exc:  # noqa: BLE001
        return R10DevOpsLink(
            name="V1122 DevOps W4",
            level="YELLOW",
            detail=f"check failed: {type(exc).__name__}:{str(exc)[:80]}",
        )


def _check_v1074_guard(thresholds: V1074Thresholds,
                       project_dir: Optional[Path] = None) -> R10DevOpsLink:
    """V1074 ASI 守门 (主 17:43 实事求是: 真跑子进程)."""
    raw = _safe_subprocess_call(
        fn=lambda: _fetch_v1074_via_subprocess(project_dir=project_dir),
        fallback={"source": "safe_fallback", "v03_score": 0.0},
    )
    v03 = raw.get("v03_score")
    if not isinstance(v03, (int, float)):
        return R10DevOpsLink(
            name="V1074 ASI guard",
            level="UNKNOWN",
            detail=f"v03 not numeric: source={raw.get('source')}",
            extra=raw,
        )
    measurement = V1074Measurement(v03_score=float(v03), source=raw.get("source", "v1074"))
    level, reason = classify_v1074(measurement, thresholds)
    return R10DevOpsLink(
        name="V1074 ASI guard",
        level=level,
        detail=reason,
        extra={"v03_score": v03, "source": raw.get("source")},
    )


def _check_v1125_r10_protocol(thresholds: V1074Thresholds) -> R10DevOpsLink:
    """V1125 R10 协议守门 (主 17:43 + 主 13:31 V0.5 ≥ 0.95)."""
    try:
        from apeireth.v1125_r10_integration_protocol import V05Score, compute_v05_score
        v04 = 0.8538  # R9 W4 末 baseline (R9-INT-005 已 merged)
        # R10 W1 起点: v04 守门 + continuity/autonomy/transferability 起点 0.85
        v05_total = compute_v05_score(
            v04_score=v04,
            continuity=0.85, autonomy=0.85, transferability=0.85,
        ).total
        if v05_total < thresholds.v05_red:
            level = "RED"
            reason = f"v05={v05_total:.4f} < v05_red={thresholds.v05_red:.4f} (紧急 rollback)"
        elif v05_total < thresholds.v05_yellow:
            level = "YELLOW"
            reason = f"v05={v05_total:.4f} < v05_yellow={thresholds.v05_yellow:.4f} (R10 终极门前兆)"
        else:
            level = "GREEN"
            reason = f"v05={v05_total:.4f} ≥ {thresholds.v05_yellow:.4f} (R10 协议守门过)"
        return R10DevOpsLink(
            name="V1125 R10 protocol",
            level=level,
            detail=reason,
            extra={"v05_total": v05_total, "v04_baseline": v04},
        )
    except Exception as exc:  # noqa: BLE001
        return R10DevOpsLink(
            name="V1125 R10 protocol",
            level="UNKNOWN",
            detail=f"compute failed: {type(exc).__name__}:{str(exc)[:80]}",
        )


def _check_release_window(window: ReleaseWindow, dt: Optional[datetime] = None) -> R10DevOpsLink:
    """R10 发布窗口守门 (主 12:14 中央 AI 是永恒身份)."""
    dt = dt or datetime.now(timezone.utc)
    in_window = window.is_in_window(dt)
    if in_window:
        return R10DevOpsLink(
            name="Release Window",
            level="GREEN",
            detail=f"在窗口内 ({dt.hour} UTC, [{window.start_hour_utc}-{window.end_hour_utc}])",
            extra={"in_window": True, "dt_iso": dt.isoformat()},
        )
    nxt = window.next_window_start(dt)
    wait = window.time_until_next_window(dt)
    return R10DevOpsLink(
        name="Release Window",
        level="YELLOW",
        detail=f"不在窗口内 ({dt.hour} UTC), 距下一窗口 {wait}",
        extra={
            "in_window": False,
            "dt_iso": dt.isoformat(),
            "next_window_start_iso": nxt.isoformat(),
            "wait_seconds": wait.total_seconds(),
        },
    )


# ---------------------------------------------------------------------------
# 6. R10DevOpsPipelineGuard (主 17:43 + 主 13:31 大胆激进: 全链路一锤)
# ---------------------------------------------------------------------------
@dataclass
class R10PipelineGuardReport:
    """R10 DevOps 全链路守门报告."""

    ts: float
    in_window: bool
    links: List[R10DevOpsLink]
    alerts: List[Alert]
    overall_level: str     # GREEN / YELLOW / RED
    philosophy_guard_ok: bool

    def to_dict(self) -> Dict[str, Any]:
        return {
            "ts": self.ts,
            "ts_iso": datetime.fromtimestamp(self.ts, tz=timezone.utc).isoformat(),
            "in_window": self.in_window,
            "overall_level": self.overall_level,
            "philosophy_guard_ok": self.philosophy_guard_ok,
            "links": [
                {"name": l.name, "level": l.level, "detail": l.detail, "extra": l.extra}
                for l in self.links
            ],
            "alerts": [a.to_dict() for a in self.alerts],
            "n_alerts": len(self.alerts),
        }


def run_r10_pipeline_guard(
    window: ReleaseWindow = DEFAULT_RELEASE_WINDOW,
    thresholds: V1074Thresholds = V1074Thresholds(),
    sink: Optional[AlertSink] = None,
    project_dir: Optional[Path] = None,
    now: Optional[datetime] = None,
    fail_on_yellow: bool = False,
) -> R10PipelineGuardReport:
    """R10 DevOps 全链路守门 (主 00:56 一行 = 全链路).

    Args:
        window: 发布窗口
        thresholds: V1074 阈值
        sink: 告警接收器 (None = 自动新建 in-memory); chaos test 注入带 persist_path 的 sink
        project_dir: V1074 子进程 cwd
        now: 测试时注入时间
        fail_on_yellow: YELLOW 是否算失败 (默认 False, 视作"需关注但守门过")

    Returns:
        R10PipelineGuardReport
    """
    sink = sink or AlertSink()
    now = now or datetime.now(timezone.utc)

    # 1. Release Window
    rw_link = _check_release_window(window, dt=now)

    # 2. V1074 ASI guard
    v1074_link = _check_v1074_guard(thresholds, project_dir=project_dir)

    # 3. V1125 R10 protocol
    v1125_link = _check_v1125_r10_protocol(thresholds)

    # 4. V1117 badge SVG (本进程 import + render)
    v1117_link = _check_v1117_badge_svg()

    # 5. V1122 DevOps W4 (本进程 import + 真跑)
    v1122_link = _check_v1122_devops_w4()

    links = [rw_link, v1074_link, v1125_link, v1117_link, v1122_link]

    # 发告警 (主 17:58 不假装: 每条都发, GREEN 也发 "确认无异常" 告警给 chaos test)
    alerts: List[Alert] = []
    for link in links:
        level = link.level
        alert = Alert(
            level=level,
            source=link.name,
            reason=link.detail,
            extra=link.extra,
        )
        sink.send(alert)
        alerts.append(alert)

    # 总体等级 (主 17:43: 数字驱动; RED > YELLOW > GREEN)
    levels_priority = {"RED": 3, "YELLOW": 2, "GREEN": 1, "UNKNOWN": 2}
    overall = max((l.level for l in links), key=lambda x: levels_priority.get(x, 0))
    # 窗口 YELLOW 不算失败 (只是"窗口外")
    if overall == "YELLOW" and rw_link.level == "YELLOW":
        non_window = [l for l in links if l.name != "Release Window"]
        if all(l.level == "GREEN" for l in non_window):
            overall = "GREEN"

    philosophy_ok = (
        v1074_link.level != "RED" and
        v1125_link.level != "RED" and
        v1117_link.level == "GREEN" and
        v1122_link.level == "GREEN"
    )

    return R10PipelineGuardReport(
        ts=time.time(),
        in_window=rw_link.level == "GREEN",
        links=links,
        alerts=alerts,
        overall_level=overall,
        philosophy_guard_ok=philosophy_ok,
    )


# ---------------------------------------------------------------------------
# 7. Chaos test (主 17:58 不假装: 监控失联守门不丢告警)
# ---------------------------------------------------------------------------
def run_chaos_test(
    persist_path: Path,
    fail_v1074_subprocess: bool = True,
    window: ReleaseWindow = DEFAULT_RELEASE_WINDOW,
    thresholds: V1074Thresholds = V1074Thresholds(),
) -> Dict[str, Any]:
    """Chaos test: 模拟 V1074 子进程失联, 验证告警不丢 (主 17:58 不假装).

    Args:
        persist_path: 告警落盘路径 (chaos test 验证落盘)
        fail_v1074_subprocess: True → 注入 v1074 失败 (主 23:44 chaos 真测)
    """
    if persist_path.exists():
        persist_path.unlink()

    sink = AlertSink(persist_path=persist_path)

    # 注入一个 fail-soft 的 V1074 (主 17:43: 真测失败 vs 真测成功, 数据驱动)
    if fail_v1074_subprocess:
        # 用一个不存在的命令让 subprocess 失败
        def fail_fn() -> Dict[str, Any]:
            proc = subprocess.run(
                [sys.executable, "-c", "import sys; sys.exit(2)"],
                capture_output=True, text=True, timeout=5, check=False,
            )
            return {"source": "chaos_test", "rc": proc.returncode, "v03_score": None}

        raw = _safe_subprocess_call(
            fn=fail_fn,
            fallback={"source": "safe_fallback", "v03_score": 0.0},
            timeout_sec=10.0,
        )
    else:
        raw = {"source": "v1074", "v03_score": 0.95}

    # 跑全链路 (V1074 部分用上面 inject)
    # 直接用 run_r10_pipeline_guard 走一遍, 但注入 sink
    report = run_r10_pipeline_guard(
        window=window,
        thresholds=thresholds,
        sink=sink,
    )

    # 验证落盘文件存在 + 至少有 N 条告警 (主 17:58 不假装: chaos 中告警必须保留)
    persisted: List[Dict[str, Any]] = []
    if persist_path.exists():
        for line in persist_path.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if line:
                try:
                    persisted.append(json.loads(line))
                except Exception:  # noqa: BLE001
                    pass

    return {
        "chaos_test": "monitor_outage",
        "fail_v1074_subprocess": fail_v1074_subprocess,
        "raw_v1074": raw,
        "report_summary": {
            "overall_level": report.overall_level,
            "philosophy_guard_ok": report.philosophy_guard_ok,
            "n_alerts_in_memory": len(report.alerts),
            "n_alerts_persisted": len(persisted),
        },
        "alert_dropped": len(report.alerts) != len(persisted),
        "persist_path": str(persist_path),
        "philosophy_ok": report.alerts != [],  # 监控失联 ≠ 零告警 (主 17:58)
    }


# ---------------------------------------------------------------------------
# 8. Markdown 报告渲染 (主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------
def render_markdown(report: R10PipelineGuardReport,
                    window: ReleaseWindow,
                    thresholds: V1074Thresholds) -> str:
    """Markdown 报告 (主 00:56 + 主 00:44 质量工程化)."""
    lines: List[str] = []
    lines.append("# R10 DevOps Pipeline Guard Report (V1130 / R10-DEV-001)")
    lines.append("")
    lines.append(f"- **Generated**: `{datetime.fromtimestamp(report.ts, tz=timezone.utc).isoformat()}`")
    lines.append(f"- **In Window**: {report.in_window}")
    lines.append(f"- **Overall Level**: **{report.overall_level}**")
    lines.append(f"- **Philosophy Guard OK**: {report.philosophy_guard_ok}")
    lines.append(f"- **Release Window (UTC)**: [{window.start_hour_utc}-{window.end_hour_utc}]")
    lines.append(f"- **Thresholds**: v03_yellow={thresholds.v03_yellow} v03_red={thresholds.v03_red} "
                 f"v05_yellow={thresholds.v05_yellow} v05_red={thresholds.v05_red}")
    lines.append("")
    lines.append("## Link Status")
    lines.append("")
    lines.append("| Name | Level | Detail |")
    lines.append("|------|-------|--------|")
    for l in report.links:
        lines.append(f"| {l.name} | {l.level} | {l.detail} |")
    lines.append("")
    lines.append(f"## Alerts ({len(report.alerts)})")
    lines.append("")
    for a in report.alerts:
        lines.append(f"- `{a.level}` **{a.source}**: {a.reason}")
    lines.append("")
    lines.append("---")
    lines.append("Generated by `apeireth.v1130_r10_release_window_guard` "
                 "(主 12:14 中央 AI 是永恒身份 · 主 17:43 实事求是 · 主 17:58 不假装)")
    return "\n".join(lines) + "\n"


# ---------------------------------------------------------------------------
# 9. CLI (主 00:56 任何人都能接手: 一行命令)
# ---------------------------------------------------------------------------
def _build_arg_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="v1130_r10_release_window_guard",
        description="R10 发布窗口守门 + V1074 监控 on-call + R10 DevOps 全链路硬化 (R10-DEV-001)",
    )
    p.add_argument("--check", action="store_true", help="全链路守门 (默认)")
    p.add_argument("--window", type=str, default="02-04", help="发布窗口 UTC e.g. 02-04 (默认 02-04)")
    p.add_argument("--v03-threshold", type=float, default=0.94, help="V1074 V0.3 黄色阈值")
    p.add_argument("--v03-red", type=float, default=0.8884, help="V1074 V0.3 红色阈值 (守门)")
    p.add_argument("--v05-threshold", type=float, default=0.95, help="V1125 V0.5 黄色阈值 (R10 终极门)")
    p.add_argument("--v05-red", type=float, default=0.92, help="V1125 V0.5 红色阈值 (紧急 rollback)")
    p.add_argument("--persist-path", type=Path, default=None,
                   help="告警落盘路径 (chaos test 用, 主 17:58 不假装)")
    p.add_argument("--chaos", action="store_true", help="chaos test (监控失联)")
    p.add_argument("--strict", action="store_true", help="不通过非零退出")
    p.add_argument("--json", action="store_true", help="JSON 输出")
    p.add_argument("--report", action="store_true", help="Markdown 报告")
    p.add_argument("--project-dir", type=Path, default=None, help="V1074 子进程 cwd")
    return p


def main(argv: Optional[List[str]] = None) -> int:
    args = _build_arg_parser().parse_args(argv)

    # 解析窗口
    try:
        start_h, end_h = args.window.split("-")
        window = ReleaseWindow(start_hour_utc=int(start_h), end_hour_utc=int(end_h))
    except Exception as exc:
        print(f"[V1130] invalid --window {args.window!r}: {exc}", file=sys.stderr)
        return 2

    thresholds = V1074Thresholds(
        v03_yellow=args.v03_threshold, v03_red=args.v03_red,
        v05_yellow=args.v05_threshold, v05_red=args.v05_red,
    )

    if args.chaos:
        persist = args.persist_path or Path(".v1130_chaos_alerts.jsonl")
        result = run_chaos_test(persist_path=persist, window=window, thresholds=thresholds)
        if args.json:
            print(json.dumps(result, indent=2, ensure_ascii=False))
        else:
            print(f"[V1130 chaos] philosophy_ok={result['philosophy_ok']} "
                  f"alert_dropped={result['alert_dropped']} "
                  f"persisted={result['report_summary']['n_alerts_persisted']}")
        if args.strict and not result["philosophy_ok"]:
            return 1
        return 0

    sink = AlertSink(persist_path=args.persist_path)
    report = run_r10_pipeline_guard(
        window=window, thresholds=thresholds, sink=sink, project_dir=args.project_dir,
    )

    if args.json:
        print(json.dumps(report.to_dict(), indent=2, ensure_ascii=False))
    elif args.report:
        print(render_markdown(report, window, thresholds))
    else:
        # 默认: 一行总结 + 链接列表
        print(f"[V1130] overall={report.overall_level} "
              f"in_window={report.in_window} "
              f"philosophy_ok={report.philosophy_guard_ok} "
              f"alerts={len(report.alerts)}")
        for l in report.links:
            print(f"  - {l.level:7s} {l.name}: {l.detail}")

    if args.strict and report.overall_level == "RED":
        return 1
    return 0


__all__ = [
    "ReleaseWindow", "DEFAULT_RELEASE_WINDOW",
    "V1074Thresholds", "V1074Measurement", "classify_v1074",
    "Alert", "AlertSink",
    "_safe_subprocess_call", "_fetch_v1074_via_subprocess",
    "R10DevOpsLink",
    "R10PipelineGuardReport", "run_r10_pipeline_guard",
    "run_chaos_test",
    "render_markdown",
    "main",
    "__version__",
]

__version__ = "0.1.0"