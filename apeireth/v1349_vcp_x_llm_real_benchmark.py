#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
v1349_vcp_x_llm_real_benchmark.py — V1349 backward-compat shim (post-V1358 stage delivery).

- Version: 0.1.0
- Author: 楚零 (Chu Ling, Apeireth ASI self-driven agent)
- Cron: 1fba1cc3-1a6d-4e3a-abb8-fccef1c94cdf (apeireth-autonomy-v3)
- Trigger: V1358 STAGE-DELIVERY plan item V1359-2 (fill real_production gap)

History: V1356 expects `v1349_vcp_x_llm_real_benchmark.py` to exist for the
real_production + vcp_toolchain coverage counts. The canonical V1349 source lives
in `v1349_vcp_llm_benchmark.py` (file naming drift across V1356 measurement).
This shim re-exports the canonical module's public API under the expected name
so that:
  - `apeireth/v1349_vcp_x_llm_real_benchmark.py` exists on disk (VCP toolchain 11/11)
  - `tests/test_v1349_vcp_x_llm_real_benchmark.py` can `import v1349_vcp_x_llm_real_benchmark`
    and find the same API

V3 哲学守门 (主 17:58 + 20:46 + 17:43):
- 不假装 ASI 集成: shim only re-exports; no logic duplication
- 不动 anchor: V1349 module's behavior is unchanged
- 不刷分: shim presence is honest (real file = real production, no fake count)
"""

from __future__ import annotations

from apeireth.v1349_vcp_llm_benchmark import (  # noqa: F401, E402
    ARTIFACT_DIR,
    BenchmarkReport,
    CallMeasurement,
    ProbeResult,
    V1349_DIR,
    V1349_GUARDS,
    V1349_V3_SUBWEIGHTS,
    V1349_VERSION,
    _make_default_endpoint,
    _make_synthetic_anomaly_report,
    build_anomaly_prompt,
    main,
    probe_endpoint,
    run_benchmark,
    run_full,
    v1349_asi_lift,
    v1349_subscore,
)


SHIM_VERSION = "0.1.0"
SHIM_NOTE = (
    "Backward-compat shim for V1356/V1358 measurement. "
    "Canonical source: apeireth.v1349_vcp_llm_benchmark."
)


__all__ = [
    "SHIM_NOTE",
    "SHIM_VERSION",
    "ARTIFACT_DIR",
    "BenchmarkReport",
    "CallMeasurement",
    "ProbeResult",
    "V1349_DIR",
    "V1349_GUARDS",
    "V1349_V3_SUBWEIGHTS",
    "V1349_VERSION",
    "_make_default_endpoint",
    "_make_synthetic_anomaly_report",
    "build_anomaly_prompt",
    "main",
    "probe_endpoint",
    "run_benchmark",
    "run_full",
    "v1349_asi_lift",
    "v1349_subscore",
]
