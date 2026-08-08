#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
v1348_anomaly_detector.py — V1348 backward-compat shim (post-V1359 stage delivery).

- Version: 0.1.0
- Author: 楚零 (Chu Ling, Apeireth ASI self-driven agent)
- Cron: 1fba1cc3-1a6d-4e3a-abb8-fccef1c94cdf (apeireth-autonomy-v3)
- Trigger: V1360 plan item (fill vcp_toolchain name-drift gap)

History: V1356 expects `v1348_anomaly_detector.py` to exist for the
vcp_toolchain 11/11 coverage. The canonical V1348 source lives in
`v1348_vcp_anomaly_detector.py` (file naming drift across V1356 measurement).
This shim re-exports the canonical module's public API under the expected
name so that:
  - `apeireth/v1348_anomaly_detector.py` exists on disk (VCP toolchain 11/11)
  - `tests/test_v1348_anomaly_detector.py` can `import v1348_anomaly_detector`
    and find the same API

V3 哲学守门 (主 17:58 + 20:46 + 17:43):
- 不假装 ASI 集成: shim only re-exports; no logic duplication
- 不动 anchor: V1348 module's behavior is unchanged
- 不刷分: shim presence is honest (real file = real production, no fake count)
"""

from __future__ import annotations

from apeireth.v1348_vcp_anomaly_detector import (  # noqa: F401, E402
    ALL_CHANNELS,
    ASI_POLE_STAR,
    CHANNEL_DRIFT_SPIKE,
    CHANNEL_HEALTH_DROP,
    CHANNEL_LINT_REGRESSION,
    CHANNEL_PLAN_ACCELERATION,
    CHANNEL_TIER_JUMP,
    DEFAULT_THRESHOLDS,
    RECOMMENDATIONS,
    SEVERITY_HIGH,
    SEVERITY_LOW,
    SEVERITY_MEDIUM,
    SEVERITY_NONE,
    SEVERITY_ORDER,
    TIER_RANK,
    V1348_DIR,
    ChannelSignal,
    EcosystemAnomalyReport,
    PluginAnomaly,
    _build_anomaly_id,
    _build_report_id,
    _stable_id,
    analyze_plugin,
    build_report,
    detect_drift_spike,
    detect_from_health_reports,
    detect_health_drop,
    detect_lint_regression,
    detect_plan_acceleration,
    detect_tier_jump,
    max_severity,
    severity_for_score,
    tier_rank,
)


SHIM_VERSION = "0.1.0"
SHIM_NOTE = (
    "Backward-compat shim for V1356/V1360 measurement. "
    "Canonical source: apeireth.v1348_vcp_anomaly_detector."
)


__all__ = [
    "SHIM_NOTE",
    "SHIM_VERSION",
    "ALL_CHANNELS",
    "ASI_POLE_STAR",
    "CHANNEL_DRIFT_SPIKE",
    "CHANNEL_HEALTH_DROP",
    "CHANNEL_LINT_REGRESSION",
    "CHANNEL_PLAN_ACCELERATION",
    "CHANNEL_TIER_JUMP",
    "DEFAULT_THRESHOLDS",
    "RECOMMENDATIONS",
    "SEVERITY_HIGH",
    "SEVERITY_LOW",
    "SEVERITY_MEDIUM",
    "SEVERITY_NONE",
    "SEVERITY_ORDER",
    "TIER_RANK",
    "V1348_DIR",
    "ChannelSignal",
    "EcosystemAnomalyReport",
    "PluginAnomaly",
    "_build_anomaly_id",
    "_build_report_id",
    "_stable_id",
    "analyze_plugin",
    "build_report",
    "detect_drift_spike",
    "detect_from_health_reports",
    "detect_health_drop",
    "detect_lint_regression",
    "detect_plan_acceleration",
    "detect_tier_jump",
    "max_severity",
    "severity_for_score",
    "tier_rank",
]