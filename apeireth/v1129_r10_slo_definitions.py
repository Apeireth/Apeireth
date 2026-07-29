"""
v1129_r10_slo_definitions.py — R10 SLO 真定义 + badge status 走势 + V1074 监控可视化.

主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 17:58 不假装 + 主 23:44 干到底
+ 主 19:33 走在前人经验上 + 主 00:56 任何人都能接手.

借鉴 (主 19:33):
- Datadog SLO 2019: 多窗口 burn rate (1h 14.4x, 6h 6x, 24h 3x, 3d 1x)
- Google SRE Workbook 2017 (Chapter 5: Reliability Engineering): SLO + error budget
- Prometheus alertmanager 2016: multi-window burn rate alerts
- V1074 StatusSnapshot.score_history: 50 条 deque 真测
- V1117 render_badge_history_svg: badge 走势串接 SVG
- V1130 AlertSink: chaos test 监控失联不丢
- V1125 evaluate_r10: V0.5 score 真测

主 13:31 大胆激进: 4 大 SLO 维度 (Availability / Latency / Guard / V1074) 显式
主 17:58 不假装: UNKNOWN/RED/YELLOW/GREEN 显式分类, 不假装
主 23:44 干到底: 4 大真生产模块 (SLO 计算 / 错误预算 / badge 走势 / dashboard)
"""

from __future__ import annotations

import argparse
import dataclasses
import json
import math
import sys
import time
from dataclasses import dataclass, field, asdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable, Dict, Iterable, List, Optional, Sequence, Tuple


# ---------------------------------------------------------------------------
# 主 19:33 走在前人经验上 — references 显式
# ---------------------------------------------------------------------------

REFERENCES = [
    {
        "id": "DatadogSLO2019",
        "title": "Datadog SLO Multi-Window Burn Rate",
        "url": "https://docs.datadoghq.com/service_management/service_level_objectives/",
        "used_for": "Multi-window burn rate (1h / 6h / 24h / 3d)",
    },
    {
        "id": "GoogleSREWorkbook2017",
        "title": "Google SRE Workbook Chapter 5: Reliability Engineering",
        "url": "https://sre.google/workbook/alerting-on-slos/",
        "used_for": "SLO + Error Budget + Burn Rate Alerting",
    },
    {
        "id": "PrometheusAlertManager2016",
        "title": "Prometheus AlertManager multi-window burn rate",
        "url": "https://prometheus.io/docs/prometheus/latest/configuration/alerting_rules/",
        "used_for": "PageSeverity classification (page/ticket/none)",
    },
    {
        "id": "V1074ASIProductionRunner",
        "title": "V1074 ASI production runner (StatusSnapshot.score_history)",
        "url": "internal:apeireth.v1074_asi_production_runner",
        "used_for": "Real test data source (50 deque)",
    },
    {
        "id": "V1117BadgeSvgRenderer",
        "title": "V1117 badge SVG renderer (render_badge_history_svg)",
        "url": "internal:apeireth.v1117_badge_svg_renderer",
        "used_for": "Badge history SVG trend",
    },
    {
        "id": "V1125R10Protocol",
        "title": "V1125 R10 protocol evaluate_r10 (V0.5 score)",
        "url": "internal:apeireth.v1125_r10_protocol",
        "used_for": "R10 V0.5 score real measurement",
    },
    {
        "id": "V1130R10ReleaseWindowGuard",
        "title": "V1130 R10 release window guard (AlertSink)",
        "url": "internal:apeireth.v1130_r10_release_window_guard",
        "used_for": "Chaos test monitoring drop preserves alerts",
    },
]


# ---------------------------------------------------------------------------
# 主 17:58 不假装 — 显式 GREEN/YELLOW/RED/UNKNOWN 等级
# ---------------------------------------------------------------------------

LEVEL_GREEN = "GREEN"
LEVEL_YELLOW = "YELLOW"
LEVEL_RED = "RED"
LEVEL_UNKNOWN = "UNKNOWN"
VALID_LEVELS = {LEVEL_GREEN, LEVEL_YELLOW, LEVEL_RED, LEVEL_UNKNOWN}


def _utc_now_iso() -> str:
    return datetime.now(tz=timezone.utc).isoformat()


def _utc_now_ts() -> float:
    return time.time()


def _clamp(x: float, lo: float = 0.0, hi: float = 1.0) -> float:
    return max(lo, min(hi, x))


# ---------------------------------------------------------------------------
# 主 23:44 干到底 — SLO 真定义 (Datadog SLO 2019 + Google SRE 2017)
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class AvailabilitySLO:
    """可用性 SLO (Datadog SLO 2019): ASI 北极星 API 99.95% 月度.

    主 22:33 ASI 北极星: API 可用性是 ASI 终极门的运维保证.
    主 17:43 实事求是: error_rate + good_events / total_events 真测.
    """

    name: str = "ASI 北极星 API 可用性"
    target: float = 0.9995  # 99.95% 月度
    window_days: int = 30  # 月度窗口
    description: str = "ASI 北极星 API monthly availability ≥ 99.95%"

    def monthly_minutes_budget(self) -> float:
        """每月错误预算 (分钟). 0.05% * 30 * 24 * 60 = 21.6 分钟."""
        return (1.0 - self.target) * self.window_days * 24 * 60

    def evaluate(self, good_events: int, total_events: int) -> Dict[str, Any]:
        """评估当前 SLO 状态 (主 17:43 实事求是: 真测真算).

        Args:
            good_events: SLI 良事件数
            total_events: SLI 总事件数
        Returns:
            {availability, error_rate, budget_minutes_left, burn_rate, level}
        """
        if total_events <= 0:
            return {
                "name": self.name,
                "target": self.target,
                "availability": float("nan"),
                "error_rate": float("nan"),
                "budget_minutes_total": self.monthly_minutes_budget(),
                "budget_minutes_left": float("nan"),
                "burn_rate": float("nan"),
                "level": LEVEL_UNKNOWN,  # 主 17:58 不假装: 0 events = UNKNOWN
                "good_events": good_events,
                "total_events": total_events,
                "ts": _utc_now_iso(),
            }
        availability = good_events / total_events
        error_rate = 1.0 - availability
        # 主 17:43 实事求是: burn_rate = (1 - target) 当前 rate / target rate
        # target error rate = 1 - 0.9995 = 0.0005
        target_err_rate = 1.0 - self.target
        if target_err_rate <= 0:
            burn_rate = 0.0 if error_rate == 0 else float("inf")
        else:
            burn_rate = error_rate / target_err_rate
        # 错误预算剩余: total_minutes - (error_rate * window_days * 24 * 60)
        window_minutes = self.window_days * 24 * 60
        minutes_consumed = error_rate * window_minutes
        budget_left = max(0.0, self.monthly_minutes_budget() - minutes_consumed)
        # 等级 (主 17:58 不假装: 显式 GREEN/YELLOW/RED/UNKNOWN)
        if error_rate > 2 * target_err_rate:  # 2x burn → RED
            level = LEVEL_RED
        elif error_rate > target_err_rate:  # > target → YELLOW
            level = LEVEL_YELLOW
        else:
            level = LEVEL_GREEN
        return {
            "name": self.name,
            "target": self.target,
            "availability": availability,
            "error_rate": error_rate,
            "budget_minutes_total": self.monthly_minutes_budget(),
            "budget_minutes_left": budget_left,
            "burn_rate": burn_rate,
            "level": level,
            "good_events": good_events,
            "total_events": total_events,
            "ts": _utc_now_iso(),
        }


@dataclass(frozen=True)
class LatencySLO:
    """延迟 SLO (Google SRE 2017): ASI 北极星测量 P95 < 2s + P99 < 5s.

    主 17:43 实事求是: 真实 latency_samples 排序 + P95/P99 计算.
    """

    name: str = "ASI 北极星 API 延迟"
    p95_target_sec: float = 2.0
    p99_target_sec: float = 5.0
    description: str = "P95 < 2s, P99 < 5s"

    def percentile(self, sorted_samples: Sequence[float], p: float) -> float:
        """百分位计算 (Google SRE Workbook 2017 §5.2).

        主 17:43 实事求是: 真排序真插值.
        """
        if not sorted_samples:
            return float("nan")
        if not (0.0 < p <= 1.0):
            raise ValueError(f"percentile p must be in (0, 1], got {p}")
        n = len(sorted_samples)
        rank = p * (n - 1)
        lo_idx = int(math.floor(rank))
        hi_idx = int(math.ceil(rank))
        if lo_idx == hi_idx:
            return sorted_samples[lo_idx]
        frac = rank - lo_idx
        return sorted_samples[lo_idx] * (1 - frac) + sorted_samples[hi_idx] * frac

    def evaluate(self, latency_samples_sec: Sequence[float]) -> Dict[str, Any]:
        """评估延迟 SLO (主 17:43 实事求是).

        Returns:
            {p95, p99, p95_pass, p99_pass, level, n_samples, ts}
        """
        if not latency_samples_sec:
            return {
                "name": self.name,
                "p95_target_sec": self.p95_target_sec,
                "p99_target_sec": self.p99_target_sec,
                "p95_sec": float("nan"),
                "p99_sec": float("nan"),
                "p95_pass": False,
                "p99_pass": False,
                "level": LEVEL_UNKNOWN,
                "n_samples": 0,
                "ts": _utc_now_iso(),
            }
        s = sorted(latency_samples_sec)
        p95 = self.percentile(s, 0.95)
        p99 = self.percentile(s, 0.99)
        p95_pass = p95 <= self.p95_target_sec
        p99_pass = p99 <= self.p99_target_sec
        # 主 17:58 不假装: P95 或 P99 任一超 → YELLOW (alert); 双超 → RED
        if not p95_pass and not p99_pass:
            level = LEVEL_RED
        elif not p95_pass or not p99_pass:
            level = LEVEL_YELLOW
        else:
            level = LEVEL_GREEN
        return {
            "name": self.name,
            "p95_target_sec": self.p95_target_sec,
            "p99_target_sec": self.p99_target_sec,
            "p95_sec": p95,
            "p99_sec": p99,
            "p95_pass": p95_pass,
            "p99_pass": p99_pass,
            "level": level,
            "n_samples": len(s),
            "ts": _utc_now_iso(),
        }


@dataclass(frozen=True)
class GuardSLO:
    """守门 SLO (V1074 监控可视化): V1074 V0.3 ≥ 0.94 + V0.5 ≥ 0.92 rollback.

    主 22:33 ASI 北极星: V1074 是 ASI 终极门的真测守门.
    主 17:58 不假装: RED 触发 rollback, YELLOW 触发告警.
    """

    name: str = "V1074 守门 SLO"
    v03_yellow: float = 0.94   # V1074 V0.3 < 0.94 → YELLOW 告警
    v03_red: float = 0.8884    # V1074 V0.3 < 0.8884 → RED 立即回滚 (R10 baseline)
    v05_yellow: float = 0.95   # V1125 V0.5 < 0.95 → YELLOW R10 终极门前兆
    v05_red: float = 0.92      # V1125 V0.5 < 0.92 → RED 紧急 rollback
    description: str = "V1074 V0.3 ≥ 0.94 / V0.5 ≥ 0.92 rollback"

    def evaluate_v03(self, v03_score: float) -> Dict[str, Any]:
        """评估 V0.3 守门 (V1074 测量分)."""
        if v03_score < self.v03_red:
            level = LEVEL_RED
        elif v03_score < self.v03_yellow:
            level = LEVEL_YELLOW
        else:
            level = LEVEL_GREEN
        return {
            "name": f"{self.name} V0.3",
            "v03_score": v03_score,
            "yellow_threshold": self.v03_yellow,
            "red_threshold": self.v03_red,
            "level": level,
            "action": {
                LEVEL_GREEN: "继续 (no action)",
                LEVEL_YELLOW: "on-call Slack 告警",
                LEVEL_RED: "立即回滚到上一个 green commit",
            }[level],
            "ts": _utc_now_iso(),
        }

    def evaluate_v05(self, v05_score: float) -> Dict[str, Any]:
        """评估 V0.5 守门 (R10 终极门)."""
        if v05_score < self.v05_red:
            level = LEVEL_RED
        elif v05_score < self.v05_yellow:
            level = LEVEL_YELLOW
        else:
            level = LEVEL_GREEN
        return {
            "name": f"{self.name} V0.5",
            "v05_score": v05_score,
            "yellow_threshold": self.v05_yellow,
            "red_threshold": self.v05_red,
            "level": level,
            "action": {
                LEVEL_GREEN: "继续 (no action)",
                LEVEL_YELLOW: "R10 终极门前兆, 提升 continuity/autonomy",
                LEVEL_RED: "紧急 rollback + on-call 升级",
            }[level],
            "ts": _utc_now_iso(),
        }


# ---------------------------------------------------------------------------
# 主 23:44 干到底 — Multi-Window Burn Rate (Datadog SLO 2019)
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class BurnRateWindow:
    """Datadog SLO 2019 multi-window burn rate.

    short window (1h) × 14.4 = fast page
    medium window (6h) × 6 = medium page
    long window (24h) × 3 = slow page
    """

    short_sec: int = 3600          # 1h
    short_multiplier: float = 14.4  # 14.4x budget burn in 1h → page
    medium_sec: int = 21600         # 6h
    medium_multiplier: float = 6.0
    long_sec: int = 86400           # 24h
    long_multiplier: float = 3.0

    def alert_level(
        self,
        error_rate_short: float,
        error_rate_medium: float,
        error_rate_long: float,
        target_error_rate: float,
    ) -> str:
        """主 17:58 不假装: 显式 alert_level.

        Returns: 'page' (RED) / 'ticket' (YELLOW) / 'none' (GREEN) / 'unknown' (UNKNOWN)
        """
        if target_error_rate <= 0:
            return "none"
        # short window 14.4x → page (RED)
        if error_rate_short > self.short_multiplier * target_error_rate:
            return LEVEL_RED
        # medium window 6x → ticket (YELLOW)
        if error_rate_medium > self.medium_multiplier * target_error_rate:
            return LEVEL_YELLOW
        # long window 3x → ticket (YELLOW)
        if error_rate_long > self.long_multiplier * target_error_rate:
            return LEVEL_YELLOW
        return LEVEL_GREEN


# ---------------------------------------------------------------------------
# 主 17:43 实事求是 — Error Budget Tracker (Google SRE 2017 §5.4)
# ---------------------------------------------------------------------------


@dataclass
class ErrorBudgetTracker:
    """错误预算追踪器 (Google SRE Workbook 2017).

    主 22:33 ASI 北极星: 错误预算是 ASI 运维的资源分配核心.
    主 17:43 实事求是: 真 budget consumption 测量.
    """

    slo: AvailabilitySLO = field(default_factory=AvailabilitySLO)
    burn_log: List[Dict[str, Any]] = field(default_factory=list)

    def record_burn(self, error_events: int, total_events: int, ts: Optional[float] = None) -> None:
        """记录一次 SLO burn (主 17:43 实事求是)."""
        ts = ts if ts is not None else _utc_now_ts()
        if total_events <= 0:
            return
        err_rate = error_events / total_events
        minutes_consumed = err_rate * self.slo.window_days * 24 * 60
        self.burn_log.append({
            "ts": ts,
            "error_events": error_events,
            "total_events": total_events,
            "error_rate": err_rate,
            "minutes_consumed": minutes_consumed,
        })

    def budget_status(self) -> Dict[str, Any]:
        """当前错误预算状态."""
        total_minutes = self.slo.monthly_minutes_budget()
        consumed = sum(b["minutes_consumed"] for b in self.burn_log)
        left = max(0.0, total_minutes - consumed)
        pct_left = left / total_minutes if total_minutes > 0 else 0.0
        # 主 17:58 不假装: 显式 level
        if pct_left < 0.10:
            level = LEVEL_RED
        elif pct_left < 0.50:
            level = LEVEL_YELLOW
        else:
            level = LEVEL_GREEN
        return {
            "slo_name": self.slo.name,
            "target": self.slo.target,
            "window_days": self.slo.window_days,
            "total_minutes": total_minutes,
            "consumed_minutes": consumed,
            "remaining_minutes": left,
            "remaining_pct": pct_left,
            "n_burns": len(self.burn_log),
            "level": level,
            "ts": _utc_now_iso(),
        }


# ---------------------------------------------------------------------------
# 主 13:31 大胆激进 — V1074 监控可视化 (badge 走势 + dashboard)
# ---------------------------------------------------------------------------


def _safe_v1117_import():
    """主 17:43 实事求是: V1117 缺失 → None (fail-soft, 不假装 GREEN)."""
    try:
        from apeireth.v1117_badge_svg_renderer import (
            COLOR_MAP, STATUS_TO_COLOR,
            render_badge_history_svg, render_badge_svg, render_status_badge,
        )
        return {
            "COLOR_MAP": COLOR_MAP,
            "STATUS_TO_COLOR": STATUS_TO_COLOR,
            "render_badge_history_svg": render_badge_history_svg,
            "render_badge_svg": render_badge_svg,
            "render_status_badge": render_status_badge,
        }
    except Exception as exc:  # noqa: BLE001
        return None


def render_v1074_trend_badge(
    score_history: Sequence[Dict[str, Any]],
    label: str = "V1074 trend",
    yellow_threshold: float = 0.94,
    red_threshold: float = 0.8884,
) -> str:
    """V1074 历史 score → badge 串接 (主 13:31 大胆激进: 走势一目了然).

    主 17:43 实事求是: 真 score 真分类.
    主 17:58 不假装: 显式 pass/fail/mixed + UNKNOWN (无 score).
    """
    v1117 = _safe_v1117_import()
    if v1117 is None:
        return f"<!-- V1117 not available, skip {label} -->"
    history: List[Tuple[str, str]] = []
    for i, rec in enumerate(score_history):
        score = rec.get("v03_score") or rec.get("score") or rec.get("v05_score")
        if score is None:
            history.append((f"{i}", "unknown"))
        elif score < red_threshold:
            history.append((f"{i}", "fail"))
        elif score < yellow_threshold:
            history.append((f"{i}", "mixed"))
        else:
            history.append((f"{i}", "pass"))
    return v1117["render_badge_history_svg"](history, label=label)


def render_slo_status_badge(level: str, label: str = "SLO") -> str:
    """SLO 等级 → 单个 badge (主 13:31 大胆激进: 一目了然)."""
    v1117 = _safe_v1117_import()
    if v1117 is None:
        return f"<!-- V1117 not available, skip {label} -->"
    status = {
        LEVEL_GREEN: "pass",
        LEVEL_YELLOW: "mixed",
        LEVEL_RED: "fail",
        LEVEL_UNKNOWN: "unknown",
    }.get(level, "unknown")
    return v1117["render_status_badge"](status, label)


# ---------------------------------------------------------------------------
# 主 23:44 干到底 — SLO Dashboard (JSON + Markdown)
# ---------------------------------------------------------------------------


@dataclass
class SLODashboard:
    """SLO dashboard 数据聚合 (主 13:31 大胆激进: 一目了然).

    主 17:43 实事求是: 4 大 SLO 维度真测聚合.
    """

    availability: Optional[Dict[str, Any]] = None
    latency: Optional[Dict[str, Any]] = None
    guard_v03: Optional[Dict[str, Any]] = None
    guard_v05: Optional[Dict[str, Any]] = None
    error_budget: Optional[Dict[str, Any]] = None
    trend_svg: str = ""
    n_history: int = 0
    ts: str = field(default_factory=_utc_now_iso)

    def overall_level(self) -> str:
        """主 17:58 不假装: 最差等级 = overall."""
        levels = [
            d.get("level") for d in (
                self.availability, self.latency, self.guard_v03, self.guard_v05,
                self.error_budget,
            ) if d is not None
        ]
        if LEVEL_RED in levels:
            return LEVEL_RED
        if LEVEL_YELLOW in levels:
            return LEVEL_YELLOW
        if LEVEL_UNKNOWN in levels:
            return LEVEL_UNKNOWN
        return LEVEL_GREEN

    def to_dict(self) -> Dict[str, Any]:
        return {
            "availability": self.availability,
            "latency": self.latency,
            "guard_v03": self.guard_v03,
            "guard_v05": self.guard_v05,
            "error_budget": self.error_budget,
            "trend_svg_chars": len(self.trend_svg),
            "n_history": self.n_history,
            "overall_level": self.overall_level(),
            "ts": self.ts,
            "references": REFERENCES,
        }

    def to_json(self, indent: int = 2) -> str:
        return json.dumps(self.to_dict(), indent=indent, ensure_ascii=False, default=str)


def render_dashboard_markdown(dash: SLODashboard) -> str:
    """SLO dashboard → Markdown 报告 (主 00:44 质量工程化)."""
    parts: List[str] = []
    parts.append("# R10 SLO Dashboard (R10-DEV-002 V1129)")
    parts.append("")
    parts.append(f"_TS: {dash.ts}_")
    parts.append(f"_Overall: **{dash.overall_level()}**_")
    parts.append("")
    parts.append("## 1. 守门 SLO (V1074 + V0.5)")
    parts.append("")
    parts.append("| 项 | 值 | 等级 | 动作 |")
    parts.append("|---|---|---|---|")
    if dash.guard_v03:
        gv3 = dash.guard_v03
        parts.append(
            f"| V1074 V0.3 | {gv3.get('v03_score', '?'):.4f} | "
            f"{gv3.get('level', '?')} | {gv3.get('action', '?')} |"
        )
    if dash.guard_v05:
        gv5 = dash.guard_v05
        parts.append(
            f"| V0.5 终极门 | {gv5.get('v05_score', '?'):.4f} | "
            f"{gv5.get('level', '?')} | {gv5.get('action', '?')} |"
        )
    parts.append("")
    parts.append("## 2. 可用性 SLO (99.95% 月度)")
    parts.append("")
    if dash.availability:
        a = dash.availability
        parts.append(f"- 目标: {a.get('target', '?')}")
        parts.append(f"- 当前 availability: {a.get('availability', float('nan')):.6f}")
        parts.append(f"- 当前 error_rate: {a.get('error_rate', float('nan')):.6f}")
        parts.append(f"- 错误预算剩余: {a.get('budget_minutes_left', float('nan')):.2f} 分钟")
        parts.append(f"- burn rate: {a.get('burn_rate', float('nan')):.2f}x")
        parts.append(f"- 等级: **{a.get('level', '?')}**")
    parts.append("")
    parts.append("## 3. 延迟 SLO (P95 < 2s, P99 < 5s)")
    parts.append("")
    if dash.latency:
        l = dash.latency
        parts.append(f"- P95: {l.get('p95_sec', float('nan')):.4f}s (target {l.get('p95_target_sec')}s) → {'pass' if l.get('p95_pass') else 'FAIL'}")
        parts.append(f"- P99: {l.get('p99_sec', float('nan')):.4f}s (target {l.get('p99_target_sec')}s) → {'pass' if l.get('p99_pass') else 'FAIL'}")
        parts.append(f"- 样本数: {l.get('n_samples', '?')}")
        parts.append(f"- 等级: **{l.get('level', '?')}**")
    parts.append("")
    parts.append("## 4. 错误预算 (Error Budget)")
    parts.append("")
    if dash.error_budget:
        e = dash.error_budget
        parts.append(f"- 总预算: {e.get('total_minutes', 0):.2f} 分钟")
        parts.append(f"- 已消耗: {e.get('consumed_minutes', 0):.2f} 分钟")
        parts.append(f"- 剩余: {e.get('remaining_minutes', 0):.2f} 分钟 ({e.get('remaining_pct', 0):.2%})")
        parts.append(f"- 等级: **{e.get('level', '?')}**")
    parts.append("")
    parts.append("## 5. V1074 监控可视化 (badge 走势)")
    parts.append("")
    if dash.trend_svg:
        parts.append(f"```svg\n{dash.trend_svg}\n```")
    parts.append("")
    parts.append("## 6. 借鉴 (主 19:33 走在前人经验上)")
    parts.append("")
    for ref in REFERENCES:
        parts.append(f"- [{ref['id']}] {ref['title']} — {ref['used_for']}")
    parts.append("")
    return "\n".join(parts)


# ---------------------------------------------------------------------------
# 主 23:44 干到底 — SLO 真跑编排 (借鉴 V1130 run_r10_pipeline_guard)
# ---------------------------------------------------------------------------


@dataclass
class SLOEvalContext:
    """SLO 评估输入 (主 17:43 实事求是: 真数据)."""

    good_events: int = 0
    total_events: int = 0
    latency_samples_sec: Sequence[float] = field(default_factory=list)
    v03_score: float = 0.0
    v05_score: float = 0.0
    score_history: Sequence[Dict[str, Any]] = field(default_factory=list)


def evaluate_slos(ctx: SLOEvalContext) -> SLODashboard:
    """4 大 SLO 维度真测 (主 17:43 实事求是 + 主 23:44 干到底)."""
    availability_slo = AvailabilitySLO()
    latency_slo = LatencySLO()
    guard_slo = GuardSLO()

    availability = availability_slo.evaluate(ctx.good_events, ctx.total_events)
    latency = latency_slo.evaluate(ctx.latency_samples_sec)
    guard_v03 = guard_slo.evaluate_v03(ctx.v03_score)
    guard_v05 = guard_slo.evaluate_v05(ctx.v05_score)

    # 错误预算: 用 SLO 真测数据记录 burn
    tracker = ErrorBudgetTracker(slo=availability_slo)
    bad_events = max(0, ctx.total_events - ctx.good_events)
    if ctx.total_events > 0:
        tracker.record_burn(bad_events, ctx.total_events)
    budget = tracker.budget_status()

    trend_svg = render_v1074_trend_badge(ctx.score_history)

    return SLODashboard(
        availability=availability,
        latency=latency,
        guard_v03=guard_v03,
        guard_v05=guard_v05,
        error_budget=budget,
        trend_svg=trend_svg,
        n_history=len(ctx.score_history),
    )


# ---------------------------------------------------------------------------
# 主 17:43 实事求是 — Chaos test 兼容 (监控失联 → SLODashboard UNKNOWN)
# ---------------------------------------------------------------------------


def evaluate_slos_chaos_safe(ctx: Optional[SLOEvalContext]) -> SLODashboard:
    """Chaos test: ctx=None → 监控失联 → 显式 UNKNOWN (主 17:58 不假装).

    Returns:
        监控失联时返回 all-UNKNOWN dashboard, 不假装 GREEN.
    """
    if ctx is None:
        return SLODashboard(
            availability={
                "name": AvailabilitySLO().name,
                "level": LEVEL_UNKNOWN,
                "availability": float("nan"),
                "error_rate": float("nan"),
                "good_events": 0,
                "total_events": 0,
                "ts": _utc_now_iso(),
            },
            latency={
                "name": LatencySLO().name,
                "level": LEVEL_UNKNOWN,
                "n_samples": 0,
                "ts": _utc_now_iso(),
            },
            guard_v03={
                "name": "V1074 守门 V0.3",
                "level": LEVEL_UNKNOWN,
                "v03_score": float("nan"),
                "ts": _utc_now_iso(),
            },
            guard_v05={
                "name": "V1074 守门 V0.5",
                "level": LEVEL_UNKNOWN,
                "v05_score": float("nan"),
                "ts": _utc_now_iso(),
            },
            error_budget={
                "slo_name": AvailabilitySLO().name,
                "level": LEVEL_UNKNOWN,
                "total_minutes": AvailabilitySLO().monthly_minutes_budget(),
                "remaining_minutes": float("nan"),
                "remaining_pct": float("nan"),
                "n_burns": 0,
                "ts": _utc_now_iso(),
            },
            trend_svg="<!-- monitoring down, trend unknown -->",
            n_history=0,
        )
    return evaluate_slos(ctx)


# ---------------------------------------------------------------------------
# 主 00:56 任何人都能接手 — CLI
# ---------------------------------------------------------------------------


def _demo_score_history() -> List[Dict[str, Any]]:
    """真测分数历史 (主 17:43 实事求是: 真 mock 50 条)."""
    import random
    rng = random.Random(20260730)
    history: List[Dict[str, Any]] = []
    base = 0.86
    for i in range(50):
        # 模拟 lift: 0.86 → 0.92 慢慢升
        lift = i / 50 * 0.06
        noise = rng.uniform(-0.005, 0.005)
        score = base + lift + noise
        history.append({
            "ts": _utc_now_ts() - (50 - i) * 3600,
            "v03_score": score,
            "level": "ASI" if score >= 0.94 else "AGI" if score >= 0.85 else "ANI",
        })
    return history


def _demo_latency_samples() -> List[float]:
    """真测延迟样本 (主 17:43 实事求是: 1000 个真 mock)."""
    import random
    rng = random.Random(42)
    return [rng.uniform(0.1, 1.9) for _ in range(1000)]


def main(argv: Optional[Sequence[str]] = None) -> int:
    """CLI: --slo / --chaos / --json / --report (主 00:56 任何人都能接手)."""
    parser = argparse.ArgumentParser(description="R10 SLO Definitions (R10-DEV-002 V1129)")
    parser.add_argument("--slo", action="store_true", help="评估 4 大 SLO 维度")
    parser.add_argument("--chaos", action="store_true", help="监控失联 chaos test")
    parser.add_argument("--json", action="store_true", help="JSON output")
    parser.add_argument("--report", action="store_true", help="Markdown report")
    parser.add_argument("--v03", type=float, default=0.8946, help="V1074 V0.3 真测分数")
    parser.add_argument("--v05", type=float, default=0.8808, help="V1125 V0.5 真测分数")
    parser.add_argument("--good", type=int, default=9950, help="good events")
    parser.add_argument("--total", type=int, default=10000, help="total events")
    args = parser.parse_args(argv)

    if args.chaos:
        dash = evaluate_slos_chaos_safe(None)
    elif args.slo or not any([args.chaos, args.slo, args.report]):
        ctx = SLOEvalContext(
            good_events=args.good,
            total_events=args.total,
            latency_samples_sec=_demo_latency_samples(),
            v03_score=args.v03,
            v05_score=args.v05,
            score_history=_demo_score_history(),
        )
        dash = evaluate_slos(ctx)
    else:
        dash = evaluate_slos(SLOEvalContext())

    if args.json:
        print(dash.to_json())
    elif args.report:
        print(render_dashboard_markdown(dash))
    else:
        print(f"[V1129] overall={dash.overall_level()}")
        if dash.guard_v03:
            print(f"  V0.3: {dash.guard_v03.get('v03_score', '?')} → {dash.guard_v03.get('level')}")
        if dash.guard_v05:
            print(f"  V0.5: {dash.guard_v05.get('v05_score', '?')} → {dash.guard_v05.get('level')}")
        if dash.availability:
            a = dash.availability
            print(
                f"  availability: {a.get('availability', float('nan')):.4f} "
                f"budget_left={a.get('budget_minutes_left', float('nan')):.2f}m"
            )
        if dash.latency:
            l = dash.latency
            print(f"  latency: P95={l.get('p95_sec', float('nan')):.3f}s P99={l.get('p99_sec', float('nan')):.3f}s")
        print(f"  trend_svg_chars: {len(dash.trend_svg)}")
    return 0


__all__ = [
    # 等级
    "LEVEL_GREEN", "LEVEL_YELLOW", "LEVEL_RED", "LEVEL_UNKNOWN", "VALID_LEVELS",
    # SLO 真定义
    "AvailabilitySLO", "LatencySLO", "GuardSLO",
    # Burn rate + Error budget
    "BurnRateWindow", "ErrorBudgetTracker",
    # Badge 走势 + Dashboard
    "render_v1074_trend_badge", "render_slo_status_badge",
    "SLODashboard", "render_dashboard_markdown",
    # 编排 + chaos + CLI
    "SLOEvalContext", "evaluate_slos", "evaluate_slos_chaos_safe",
    "main",
    "REFERENCES",
]


if __name__ == "__main__":
    sys.exit(main())