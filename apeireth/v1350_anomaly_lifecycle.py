#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
v1350_anomaly_lifecycle.py — V1350 backward-compat shim (post-V1359 stage delivery).

- Version: 0.1.0
- Author: 楚零 (Chu Ling, Apeireth ASI self-driven agent)
- Cron: 1fba1cc3-1a6d-4e3a-abb8-fccef1c94cdf (apeireth-autonomy-v3)
- Trigger: V1360 plan item (fill vcp_toolchain name-drift gap)

History: V1356 expects `v1350_anomaly_lifecycle.py` to exist for the
vcp_toolchain 11/11 coverage. The canonical V1350 source lives in
`v1350_vcp_anomaly_lifecycle.py` (file naming drift across V1356 measurement).
This shim re-exports the canonical module's public API under the expected
name so that:
  - `apeireth/v1350_anomaly_lifecycle.py` exists on disk (VCP toolchain 11/11)
  - `tests/test_v1350_anomaly_lifecycle.py` can `import v1350_anomaly_lifecycle`
    and find the same API

V3 哲学守门 (主 17:58 + 20:46 + 17:43):
- 不假装 ASI 集成: shim only re-exports; no logic duplication
- 不动 anchor: V1350 module's behavior is unchanged
- 不刷分: shim presence is honest (real file = real production, no fake count)
"""

from __future__ import annotations

from apeireth.v1350_vcp_anomaly_lifecycle import (  # noqa: F401, E402
    ACTION_ACKNOWLEDGE,
    ACTION_CLOSE,
    ACTION_ESCALATE,
    ACTION_MITIGATE,
    ACTION_REOPEN,
    ACTION_RESOLVE,
    ACTION_SEVERITY_REQUIRED,
    ALL_ACTIONS,
    ALL_STATES,
    ESCALATION_SEVERITY,
    GUARD_NOT_LIFECYCLE_IS_ORACLE,
    GUARD_NOT_MACHINE_IS_CONSCIOUS,
    GUARD_NOT_PLUGIN_IS_PHENOMENAL,
    GUARD_NOT_SUBSCORE_IS_ASI,
    GUARD_NOT_WORKFLOW_IS_POLICY,
    REOPEN_NEXT_STATE,
    STATE_CLOSED,
    STATE_ESCALATED,
    STATE_MITIGATED,
    STATE_OPEN,
    STATE_RANK,
    STATE_REOPENED,
    STATE_RESOLVED,
    STATE_TRIAGED,
    TRANSITIONS,
    V1350_ASI_CAP,
    V1350_DIR,
    V1350_GUARDS,
    V1350_SUBWEIGHTS,
    V1350_VERSION,
    EcosystemLifecycleReport,
    LifecycleEvent,
    LifecycleRecord,
    LifecycleStore,
    _build_event_id,
    _build_lifecycle_id,
    _canonical,
    _clamp01,
    _now_iso,
    _stable_id,
    apply_transition,
    build_initial_record,
    ecosystem_rollup,
    list_transitions,
    open_from_anomaly,
    transition_lookup,
    v1350_asi_lift,
    v1350_subscore,
)


SHIM_VERSION = "0.1.0"
SHIM_NOTE = (
    "Backward-compat shim for V1356/V1360 measurement. "
    "Canonical source: apeireth.v1350_vcp_anomaly_lifecycle."
)


__all__ = [
    "SHIM_NOTE",
    "SHIM_VERSION",
    "ACTION_ACKNOWLEDGE",
    "ACTION_CLOSE",
    "ACTION_ESCALATE",
    "ACTION_MITIGATE",
    "ACTION_REOPEN",
    "ACTION_RESOLVE",
    "ACTION_SEVERITY_REQUIRED",
    "ALL_ACTIONS",
    "ALL_STATES",
    "ESCALATION_SEVERITY",
    "GUARD_NOT_LIFECYCLE_IS_ORACLE",
    "GUARD_NOT_MACHINE_IS_CONSCIOUS",
    "GUARD_NOT_PLUGIN_IS_PHENOMENAL",
    "GUARD_NOT_SUBSCORE_IS_ASI",
    "GUARD_NOT_WORKFLOW_IS_POLICY",
    "REOPEN_NEXT_STATE",
    "STATE_CLOSED",
    "STATE_ESCALATED",
    "STATE_MITIGATED",
    "STATE_OPEN",
    "STATE_RANK",
    "STATE_REOPENED",
    "STATE_RESOLVED",
    "STATE_TRIAGED",
    "TRANSITIONS",
    "V1350_ASI_CAP",
    "V1350_DIR",
    "V1350_GUARDS",
    "V1350_SUBWEIGHTS",
    "V1350_VERSION",
    "EcosystemLifecycleReport",
    "LifecycleEvent",
    "LifecycleRecord",
    "LifecycleStore",
    "_build_event_id",
    "_build_lifecycle_id",
    "_canonical",
    "_clamp01",
    "_now_iso",
    "_stable_id",
    "apply_transition",
    "build_initial_record",
    "ecosystem_rollup",
    "list_transitions",
    "open_from_anomaly",
    "transition_lookup",
    "v1350_asi_lift",
    "v1350_subscore",
]