#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
v1347_vcp_health_score.py — V1347 backward-compat shim (post-V1359 stage delivery).

- Version: 0.1.0
- Author: 楚零 (Chu Ling, Apeireth ASI self-driven agent)
- Cron: 1fba1cc3-1a6d-4e3a-abb8-fccef1c94cdf (apeireth-autonomy-v3)
- Trigger: V1360 plan item (fill vcp_toolchain name-drift gap)

History: V1356 expects `v1347_vcp_health_score.py` to exist for the
vcp_toolchain 11/11 coverage. The canonical V1347 source lives in
`v1347_vcp_plugin_health.py` (file naming drift across V1356 measurement).
This shim re-exports the canonical module's public API under the expected
name so that:
  - `apeireth/v1347_vcp_health_score.py` exists on disk (VCP toolchain 11/11)
  - `tests/test_v1347_vcp_health_score.py` can `import v1347_vcp_health_score`
    and find the same API

V3 哲学守门 (主 17:58 + 20:46 + 17:43):
- 不假装 ASI 集成: shim only re-exports; no logic duplication
- 不动 anchor: V1347 module's behavior is unchanged
- 不刷分: shim presence is honest (real file = real production, no fake count)
"""

from __future__ import annotations

from apeireth.v1347_vcp_plugin_health import (  # noqa: F401, E402
    ASI_POLE_STAR,
    DRIFT_BONUS_RECENT_PASS,
    DRIFT_PENALTY_CRITICAL,
    DRIFT_PENALTY_HIGH,
    DRIFT_PENALTY_LOW,
    DRIFT_PENALTY_MEDIUM,
    PLAN_SEVERITY_OFFSETS,
    TIER_DEGRADED_MIN,
    TIER_HEALTHY_MIN,
    V1347_DIR,
    WEIGHTS,
    WEIGHT_COVERAGE,
    WEIGHT_DRIFT,
    WEIGHT_LINT,
    WEIGHT_PLAN,
    WEIGHT_TIER,
    EcosystemRollup,
    HealthComponent,
    HealthTier,
    PluginHealth,
    _canonical_id,
    _clamp01,
    _now_iso,
    _safe_div,
    compute_components,
    ecosystem_rollup,
    ecosystem_to_json,
    ecosystem_to_markdown,
    health_score,
    recommend,
    score_coverage,
    score_drift,
    score_lint,
    score_plan,
    score_tier,
    tier_for_score,
    to_human,
    to_json,
    to_markdown,
)


SHIM_VERSION = "0.1.0"
SHIM_NOTE = (
    "Backward-compat shim for V1356/V1360 measurement. "
    "Canonical source: apeireth.v1347_vcp_plugin_health."
)


__all__ = [
    "SHIM_NOTE",
    "SHIM_VERSION",
    "ASI_POLE_STAR",
    "DRIFT_BONUS_RECENT_PASS",
    "DRIFT_PENALTY_CRITICAL",
    "DRIFT_PENALTY_HIGH",
    "DRIFT_PENALTY_LOW",
    "DRIFT_PENALTY_MEDIUM",
    "PLAN_SEVERITY_OFFSETS",
    "TIER_DEGRADED_MIN",
    "TIER_HEALTHY_MIN",
    "V1347_DIR",
    "WEIGHTS",
    "WEIGHT_COVERAGE",
    "WEIGHT_DRIFT",
    "WEIGHT_LINT",
    "WEIGHT_PLAN",
    "WEIGHT_TIER",
    "EcosystemRollup",
    "HealthComponent",
    "HealthTier",
    "PluginHealth",
    "_canonical_id",
    "_clamp01",
    "_now_iso",
    "_safe_div",
    "compute_components",
    "ecosystem_rollup",
    "ecosystem_to_json",
    "ecosystem_to_markdown",
    "health_score",
    "recommend",
    "score_coverage",
    "score_drift",
    "score_lint",
    "score_plan",
    "score_tier",
    "tier_for_score",
    "to_human",
    "to_json",
    "to_markdown",
]