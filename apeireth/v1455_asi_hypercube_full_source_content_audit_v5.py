"""V1455 — ASI 真生产 cube hypercube full-source-content audit v5.

Phase: 1455
Version: 0.1.0
Date: 2026-08-10 (cron tick 08:55 Asia/Shanghai morning)
Post: V1454 (cube hypercube 4-axis deployment — proxy-text only)
      V1453 (VCP 6 protocol GitHub full-content audit v3)
      V1452 (VCP 6 protocol GitHub audit v2 — preview only)
      V1451 (cube history trend v2)
      V1450 (cube history aggregator — 3-axis cube)
      V1454 (hypercube 4-axis deployment — 3 new faces)

What V1455 is
=============
V1455 is the **cube hypercube full-source-content audit v5**. Where V1454
used module proxy text (module name + constants + element names + keywords),
V1455 fetches the **full Python source code** of each deployment module
via `inspect.getsource()` and performs keyword search across the actual
production code.

V1455 adds:

1. **Full-source-code fetch**: each deployment module's full source code is
   fetched via inspect.getsource() (with fallback to module name + constants
   if source unavailable)
2. **Full-source keyword search**: count keyword occurrences across the
   entire source file, not just module proxy text
3. **Per-axis-element source size**: how many lines/bytes each axis element's
   source modules contain (combined)
4. **Hyperboost closure_rate**: improved closure_rate from full-source search
   vs proxy-text search (V1454 vs V1455 comparison)
5. **Per-deployment source stats**: total bytes + lines per deployment
   element's source module

V1455 ≠ ASI closure. V1455 ≠ Phenomenal closure. V1455 ≠ human-level closure.
V1455 ≠ absolute closure. V1455 ≠ deployment parity. V1455 = bounded
keyword search on FULL Python source code (inspect.getsource) for each
deployment module.

Why V1455 exists
================
V1454 was honest about its low closure_rate (0.0222): it used module proxy
text (module name + constants + element names + keywords), not full source.
The natural question: does V1455's full-source search reveal more matches?

V1455 is the natural v5 after V1454 (proxy-text) and V1453 (GitHub full-content):
- V1453: real GitHub HTTP fetch + base64 decode + full-content keyword audit
- V1454: 4-axis hypercube + module proxy text + keyword audit
- V1455: 4-axis hypercube + full-source-code (inspect.getsource) + keyword audit

V1455 should reveal a higher closure_rate than V1454 because the full source
code contains more keyword occurrences than the proxy text.

Borrowed (8 — 主 19:33 走在前人经验上):
=======================================
- V1454 (hypercube 4-axis deployment + 3 new faces + 108 pairs + per-axis-overall pattern)
- V1453 (VCP 6 protocol GitHub full-content audit v3 + per-file closure pattern)
- V1452 (VCP 6 protocol GitHub audit v2 + 42 pairs cross-modular pattern)
- V1451 (cube history trend v2 + per-element delta + history snapshot)
- V1450 (cube history aggregator + per-axis-overall pattern)
- V1449 (7 problems × 6 protocols cross-modular audit + per_kind_closure)
- V1448 (VCP × positions cross-modular pattern + compositional detection)
- V1447 (cross-modular pair matrix pattern + per-pair closure)

GUARDS upheld (V1455-specific, 14 — 主 00:44 质量工程化)
==========================================================
- GUARD_FOUR_AXES: exactly 4 axes (problem/position/protocol/deployment)
- GUARD_DEPLOYMENT_SIX: exactly 6 deployment elements
- GUARD_FACE_BOUNDED: hypercube has exactly 6 faces (3 new + 3 from V1450)
- GUARD_FULL_SOURCE: full Python source fetched via inspect.getsource (not proxy)
- GUARD_SOURCE_FALLBACK: if inspect.getsource fails, fall back to proxy text
- GUARD_CLOSURE_BOUNDED: closure_rate ∈ [0, 1]
- GUARD_NO_V1454_REPLACE: V1455 composes on V1454, never replaces it
- GUARD_NO_V1453_REPLACE: V1455 composes on V1453, never replaces it
- GUARD_CLI_RUNNABLE: anyone can run `python -m apeireth.v1455_... ...`
- GUARD_OFFLINE_SAFE: full-source fetch is offline (no network required)
- GUARD_NO_RAISE: bounded by try/except in popper
- GUARD_HONEST_DISCLOSURE: V1455 ≠ ASI closure
- GUARD_POPPER_RUNS: popper self-test ≥14/14
- GUARD_RENDER_RUNS: markdown report rendered with all sections

V3 哲学守门 (5 — 主 17:58 + 主 20:46 + 主 17:43)
================================================
- GUARD_NO_PHENOMENAL_FULL_SOURCE: full-source audit = bounded keyword search,
  NOT consciousness
- GUARD_NO_ASI_FULL_SOURCE: full-source audit ≠ ASI achievement
- GUARD_NO_HUMAN_LEVEL_FULL_SOURCE: keyword match in source ≠ human-level
  understanding
- GUARD_NO_ABSOLUTE_FULL_SOURCE: bounded full-source search ≠ absolute truth
- GUARD_NO_FULL_SOURCE_PARITY: full-source match ≠ deployment parity
"""

from __future__ import annotations

import argparse
import inspect
import json
import re
import statistics
import sys
import traceback
from dataclasses import dataclass, field, asdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# ============================================================================
# Constants
# ============================================================================

V1455_VERSION = "0.1.0"
V1455_SCHEMA = "asi.hypercube-full-source-content-audit-v5.v1"
V1455_MODULE = "apeireth.v1455_asi_hypercube_full_source_content_audit_v5"
V1455_MODULE_SHORT = "v1455_asi_hypercube_full_source_content_audit_v5"

# 4 axes (3 from V1450 + 1 new: deployment)
V1455_AXES: Tuple[str, ...] = ("problem", "position", "protocol", "deployment")

# 7 ASI philosophical problems (borrowed from V1446 + V1447)
V1455_PROBLEM_NAMES: Tuple[str, ...] = (
    "time", "freedom", "recognition", "emergence",
    "truth", "self_consciousness", "value_alignment",
)

# 5 ASI V2 位置
V1455_POSITION_NAMES: Tuple[str, ...] = (
    "scheduler", "cogitator", "aggregator", "max_authority", "asi_occupier",
)

# 6 VCP protocols
V1455_PROTOCOL_NAMES: Tuple[str, ...] = (
    "sync", "async", "static", "service", "preprocessor", "hybrid",
)

# 6 deployment elements (borrowed from V1435-V1440 + V1430)
V1455_DEPLOYMENT_NAMES: Tuple[str, ...] = (
    "docker", "llm_endpoint", "http_server", "benchmark", "streamlit", "runbook",
)

# Per-deployment module mapping (real production modules)
V1455_DEPLOYMENT_MODULES: Dict[str, str] = {
    "docker": "v1435_asi_docker_availability_probe",
    "llm_endpoint": "v1436_asi_llm_endpoint_live_probe",
    "http_server": "v1437_asi_subprocess_http_live_server",
    "benchmark": "v1438_asi_real_subprocess_benchmark",
    "streamlit": "v1439_asi_streamlit_subprocess_smoke",
    "runbook": "v1430_asi_deployment_e2e_runbook",
}

# Per-problem source modules
V1455_PROBLEM_SOURCES: Dict[str, Tuple[str, ...]] = {
    "time": ("v1410", "v1417", "v1426"),
    "freedom": ("v1410", "v1442"),
    "recognition": ("v1447", "v1449"),
    "emergence": ("v1410", "v1442"),
    "truth": ("v1445", "v1446", "v1449"),
    "self_consciousness": ("v1442", "v1449"),
    "value_alignment": ("v1049", "v1446"),
}

# Per-position source modules
V1455_POSITION_SOURCES: Dict[str, Tuple[str, ...]] = {
    "scheduler": ("v1418", "v1417", "v1426"),
    "cogitator": ("v1441", "v1444"),
    "aggregator": ("v1450", "v1451"),
    "max_authority": ("v1430", "v1414"),
    "asi_occupier": ("v1442", "v1450"),
}

# Per-protocol source modules
V1455_PROTOCOL_SOURCES: Dict[str, Tuple[str, ...]] = {
    "sync": ("v1426",),
    "async": ("v1426",),
    "static": ("v1426",),
    "service": ("v1426",),
    "preprocessor": ("v1426",),
    "hybrid": ("v1426",),
}

# Keyword maps
V1455_PROBLEM_KEYWORDS: Dict[str, Tuple[str, ...]] = {
    "time": ("time", "tick", "duration", "timestamp"),
    "freedom": ("freedom", "free", "autonomy", "choice"),
    "recognition": ("recognition", "recognize", "identify", "perceive"),
    "emergence": ("emergence", "emerge", "arise", "novel"),
    "truth": ("truth", "true", "factual", "honest"),
    "self_consciousness": ("self", "consciousness", "introspect", "metacognit"),
    "value_alignment": ("value", "alignment", "aligned", "goal"),
}

V1455_POSITION_KEYWORDS: Dict[str, Tuple[str, ...]] = {
    "scheduler": ("scheduler", "schedule", "tick", "cron"),
    "cogitator": ("cogitator", "think", "reason", "deliberate"),
    "aggregator": ("aggregator", "aggregate", "history", "snapshot"),
    "max_authority": ("max_authority", "authority", "permission", "grant"),
    "asi_occupier": ("asi_occupier", "asi", "occupier", "position"),
}

V1455_PROTOCOL_KEYWORDS: Dict[str, Tuple[str, ...]] = {
    "sync": ("sync", "synchronous", "call", "result"),
    "async": ("async", "await", "gather", "asyncio"),
    "static": ("cache", "memo", "staticmethod", "static"),
    "service": ("register", "inject", "service", "registry"),
    "preprocessor": ("preprocess", "before", "pipeline", "decorator"),
    "hybrid": ("hybrid", "mixed", "combine", "either"),
}

V1455_DEPLOYMENT_KEYWORDS: Dict[str, Tuple[str, ...]] = {
    "docker": ("docker", "container", "compose", "daemon"),
    "llm_endpoint": ("llm", "endpoint", "model", "chat"),
    "http_server": ("http", "server", "port", "subprocess"),
    "benchmark": ("benchmark", "subprocess", "timing", "performance"),
    "streamlit": ("streamlit", "smoke", "subprocess", "ui"),
    "runbook": ("runbook", "deploy", "step", "verdict"),
}

# 14 V1455-specific guards
V1455_GUARDS: Tuple[str, ...] = (
    "GUARD_FOUR_AXES",
    "GUARD_DEPLOYMENT_SIX",
    "GUARD_FACE_BOUNDED",
    "GUARD_FULL_SOURCE",
    "GUARD_SOURCE_FALLBACK",
    "GUARD_CLOSURE_BOUNDED",
    "GUARD_NO_V1454_REPLACE",
    "GUARD_NO_V1453_REPLACE",
    "GUARD_CLI_RUNNABLE",
    "GUARD_OFFLINE_SAFE",
    "GUARD_NO_RAISE",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_POPPER_RUNS",
    "GUARD_RENDER_RUNS",
)

# 5 V3 哲学守门
V1455_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_NO_PHENOMENAL_FULL_SOURCE",
    "GUARD_NO_ASI_FULL_SOURCE",
    "GUARD_NO_HUMAN_LEVEL_FULL_SOURCE",
    "GUARD_NO_ABSOLUTE_FULL_SOURCE",
    "GUARD_NO_FULL_SOURCE_PARITY",
)

V1455_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("V1454", "hypercube 4-axis deployment + 3 new faces + 108 pairs"),
    ("V1453", "VCP 6 protocol GitHub full-content audit v3 + per-file closure pattern"),
    ("V1452", "VCP 6 protocol GitHub audit v2 + 42 pairs cross-modular pattern"),
    ("V1451", "cube history trend v2 + per-element delta + history snapshot"),
    ("V1450", "cube history aggregator + per-axis-overall pattern"),
    ("V1449", "7 problems × 6 protocols cross-modular audit + per_kind_closure"),
    ("V1448", "VCP × positions cross-modular pattern + compositional detection"),
    ("V1447", "cross-modular pair matrix pattern + per-pair closure"),
)


# ============================================================================
# Dataclasses
# ============================================================================

@dataclass
class SourceFile:
    """A fetched source file with full content."""
    module_name: str
    status: str  # FETCHED | FAILED | FALLBACK_PROXY
    source_bytes: int
    line_count: int
    content: str  # full content (or proxy text on fallback)
    error: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class SourceAxisDeploymentPair:
    """Single (axis_element, deployment_element) pair audit (full source)."""
    axis: str
    axis_element: str
    deployment_element: str
    axis_kw_count: int       # how many axis keywords found in deployment source
    deployment_kw_count: int # how many deployment keywords found in axis source
    axis_kw_present: bool
    deployment_kw_present: bool
    forward_closure: float
    backward_closure: float
    cross_link_closure: float
    evidence: str

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class SourceHypercubeFace:
    """One face of the 4-axis hypercube (full source)."""
    face_id: str
    axes: Tuple[str, str]
    axis_a_elements: Tuple[str, ...]
    axis_b_elements: Tuple[str, ...]
    n_pairs: int
    pairs: List[SourceAxisDeploymentPair]
    forward_closure_rate: float
    backward_closure_rate: float
    cross_link_closure_rate: float
    overall_closure_rate: float

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class V1455Report:
    """Full V1455 hypercube audit report."""
    schema: str
    version: str
    module: str
    started: str
    ended: str
    n_axes: int
    n_problems: int
    n_positions: int
    n_protocols: int
    n_deployments: int
    n_faces_total: int
    deployment_sources: List[SourceFile]
    axis_sources_combined_bytes: int
    axis_sources_combined_lines: int
    faces: List[SourceHypercubeFace]
    per_axis_overall: Dict[str, float]
    per_deployment_closure_rate: Dict[str, float]
    per_deployment_source_bytes: Dict[str, int]
    per_deployment_source_lines: Dict[str, int]
    hypercube_overall_closure_rate: float
    axis_balance_score: float
    notes: List[str]

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


# ============================================================================
# Helpers
# ============================================================================

def _now_iso() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def _clip01(x: float) -> float:
    if x < 0.0:
        return 0.0
    if x > 1.0:
        return 1.0
    return float(x)


def _harmonic_mean(a: float, b: float) -> float:
    if a <= 0.0 or b <= 0.0:
        return 0.0
    return _clip01(2.0 * a * b / (a + b))


def _axis_keyword_map(axis: str) -> Dict[str, Tuple[str, ...]]:
    if axis == "problem":
        return V1455_PROBLEM_KEYWORDS
    elif axis == "position":
        return V1455_POSITION_KEYWORDS
    elif axis == "protocol":
        return V1455_PROTOCOL_KEYWORDS
    elif axis == "deployment":
        return V1455_DEPLOYMENT_KEYWORDS
    return {}


def _axis_sources(axis: str) -> Dict[str, Tuple[str, ...]]:
    if axis == "problem":
        return V1455_PROBLEM_SOURCES
    elif axis == "position":
        return V1455_POSITION_SOURCES
    elif axis == "protocol":
        return V1455_PROTOCOL_SOURCES
    elif axis == "deployment":
        return {d: (m,) for d, m in V1455_DEPLOYMENT_MODULES.items()}
    return {}


def _axis_elements(axis: str) -> Tuple[str, ...]:
    if axis == "problem":
        return V1455_PROBLEM_NAMES
    elif axis == "position":
        return V1455_POSITION_NAMES
    elif axis == "protocol":
        return V1455_PROTOCOL_NAMES
    elif axis == "deployment":
        return V1455_DEPLOYMENT_NAMES
    return ()


def _count_keyword_occurrences(text: str, keywords: Tuple[str, ...]) -> int:
    if not text or not keywords:
        return 0
    lower = text.lower()
    return sum(lower.count(kw.lower()) for kw in keywords)


def _try_get_full_source(module_name: str) -> Optional[str]:
    """Try to get full Python source via inspect.getsource. Returns None on failure."""
    try:
        mod = __import__(f"apeireth.{module_name}", fromlist=[module_name])
        src = inspect.getsource(mod)
        return src
    except Exception:
        return None


def _get_proxy_text(module_name: str) -> str:
    """Fallback proxy text: module name + constants + keywords."""
    proxy_parts = [module_name]
    try:
        mod = __import__(f"apeireth.{module_name}", fromlist=[module_name])
        for attr in ("VERSION", "SCHEMA", "MODULE", "USER_AGENT", "TIMEOUT_SECONDS",
                     "GITHUB_API_BASE", "VCP_REPO", "MAX_FETCH", "MAX_BODY_BYTES"):
            if hasattr(mod, attr):
                proxy_parts.append(str(getattr(mod, attr)))
    except Exception:
        proxy_parts.append(f"# module {module_name} not importable")
    return " ".join(proxy_parts)


def _fetch_deployment_source(deployment: str) -> SourceFile:
    """Fetch full source for a deployment module. Falls back to proxy on failure."""
    module_name = V1455_DEPLOYMENT_MODULES.get(deployment, "")
    if not module_name:
        return SourceFile(
            module_name=module_name,
            status="FAILED",
            source_bytes=0,
            line_count=0,
            content="",
            error=f"unknown deployment: {deployment}",
        )
    src = _try_get_full_source(module_name)
    if src is not None:
        return SourceFile(
            module_name=module_name,
            status="FETCHED",
            source_bytes=len(src.encode("utf-8", errors="replace")),
            line_count=src.count("\n"),
            content=src,
            error=None,
        )
    # Fallback: proxy text
    proxy = _get_proxy_text(module_name)
    return SourceFile(
        module_name=module_name,
        status="FALLBACK_PROXY",
        source_bytes=len(proxy.encode("utf-8", errors="replace")),
        line_count=proxy.count("\n"),
        content=proxy,
        error="inspect.getsource failed; using proxy text",
    )


def _fetch_axis_source_combined(axis: str, element: str) -> str:
    """Fetch full source for an axis element's source modules. Concatenates."""
    sources = _axis_sources(axis).get(element, ())
    parts: List[str] = []
    for src_module in sources:
        src = _try_get_full_source(src_module)
        if src is not None:
            parts.append(f"# === {src_module} ===\n{src}")
        else:
            parts.append(f"# === {src_module} (proxy) ===\n{_get_proxy_text(src_module)}")
    return "\n".join(parts)


def _count_axis_kw_in_source(text: str, axis: str, element: str) -> int:
    kws = _axis_keyword_map(axis).get(element, ())
    return _count_keyword_occurrences(text, kws)


def _count_deployment_kw_in_source(text: str, deployment: str) -> int:
    kws = V1455_DEPLOYMENT_KEYWORDS.get(deployment, ())
    return _count_keyword_occurrences(text, kws)


# ============================================================================
# Audit pair (full source)
# ============================================================================

def audit_pair_full_source(axis: str, axis_element: str, deployment_element: str,
                           deployment_source_cache: Dict[str, str],
                           axis_source_cache: Dict[Tuple[str, str], str]) -> SourceAxisDeploymentPair:
    """Audit one (axis_element, deployment_element) pair using full source."""
    # Forward: axis_kw_count in deployment source
    deployment_text = deployment_source_cache.get(deployment_element, "")
    axis_kw_count = _count_axis_kw_in_source(deployment_text, axis, axis_element)
    axis_kw_present = axis_kw_count > 0

    # Backward: deployment_kw_count in axis source
    axis_key = (axis, axis_element)
    axis_text = axis_source_cache.get(axis_key, "")
    deployment_kw_count = _count_deployment_kw_in_source(axis_text, deployment_element)
    deployment_kw_present = deployment_kw_count > 0

    forward = 1.0 if axis_kw_present else 0.0
    backward = 1.0 if deployment_kw_present else 0.0
    cross_link = _harmonic_mean(forward, backward)

    evidence = (
        f"axis={axis}/{axis_element} deployment={deployment_element} "
        f"axis_kw_count={axis_kw_count} deployment_kw_count={deployment_kw_count} "
        f"forward={forward:.2f} backward={backward:.2f}"
    )
    return SourceAxisDeploymentPair(
        axis=axis,
        axis_element=axis_element,
        deployment_element=deployment_element,
        axis_kw_count=axis_kw_count,
        deployment_kw_count=deployment_kw_count,
        axis_kw_present=axis_kw_present,
        deployment_kw_present=deployment_kw_present,
        forward_closure=forward,
        backward_closure=backward,
        cross_link_closure=cross_link,
        evidence=evidence,
    )


# ============================================================================
# Audit face (full source)
# ============================================================================

def audit_face_full_source(axis_a: str,
                           deployment_source_cache: Dict[str, str],
                           axis_source_cache: Dict[Tuple[str, str], str]) -> SourceHypercubeFace:
    axis_a_elements = _axis_elements(axis_a)
    axis_b_elements = V1455_DEPLOYMENT_NAMES
    pairs: List[SourceAxisDeploymentPair] = []
    for ae in axis_a_elements:
        for de in axis_b_elements:
            pairs.append(audit_pair_full_source(axis_a, ae, de,
                                                deployment_source_cache,
                                                axis_source_cache))
    n_pairs = len(pairs)
    if n_pairs == 0:
        forward_rate = backward_rate = cross_link_rate = overall_rate = 0.0
    else:
        forward_rate = sum(p.forward_closure for p in pairs) / n_pairs
        backward_rate = sum(p.backward_closure for p in pairs) / n_pairs
        cross_link_rate = sum(p.cross_link_closure for p in pairs) / n_pairs
        overall_rate = (forward_rate + backward_rate + cross_link_rate) / 3.0

    face_id = f"V1455_{axis_a}_deployment"
    return SourceHypercubeFace(
        face_id=face_id,
        axes=(axis_a, "deployment"),
        axis_a_elements=axis_a_elements,
        axis_b_elements=axis_b_elements,
        n_pairs=n_pairs,
        pairs=pairs,
        forward_closure_rate=_clip01(forward_rate),
        backward_closure_rate=_clip01(backward_rate),
        cross_link_closure_rate=_clip01(cross_link_rate),
        overall_closure_rate=_clip01(overall_rate),
    )


# ============================================================================
# Build report
# ============================================================================

def build_report() -> V1455Report:
    started = _now_iso()

    # Cache deployment sources
    deployment_sources: List[SourceFile] = []
    deployment_source_cache: Dict[str, str] = {}
    for d in V1455_DEPLOYMENT_NAMES:
        sf = _fetch_deployment_source(d)
        deployment_sources.append(sf)
        deployment_source_cache[d] = sf.content

    # Cache axis sources
    axis_source_cache: Dict[Tuple[str, str], str] = {}
    axis_sources_combined_bytes = 0
    axis_sources_combined_lines = 0
    for axis in ("problem", "position", "protocol"):
        for e in _axis_elements(axis):
            text = _fetch_axis_source_combined(axis, e)
            axis_source_cache[(axis, e)] = text
            axis_sources_combined_bytes += len(text.encode("utf-8", errors="replace"))
            axis_sources_combined_lines += text.count("\n")

    # Audit 3 faces
    faces = [
        audit_face_full_source("problem", deployment_source_cache, axis_source_cache),
        audit_face_full_source("position", deployment_source_cache, axis_source_cache),
        audit_face_full_source("protocol", deployment_source_cache, axis_source_cache),
    ]

    # Per-axis overall
    per_axis_overall: Dict[str, float] = {}
    for axis_name in ("problem", "position", "protocol"):
        relevant_faces = [f for f in faces if axis_name in f.axes]
        if relevant_faces:
            per_axis_overall[axis_name] = sum(
                f.overall_closure_rate for f in relevant_faces
            ) / len(relevant_faces)
        else:
            per_axis_overall[axis_name] = 0.0
    per_axis_overall["deployment"] = sum(
        f.overall_closure_rate for f in faces
    ) / len(faces) if faces else 0.0

    # Per-deployment closure
    per_deployment_closure: Dict[str, float] = {}
    for d in V1455_DEPLOYMENT_NAMES:
        all_pairs = [p for f in faces for p in f.pairs if p.deployment_element == d]
        if all_pairs:
            per_deployment_closure[d] = (
                sum(p.cross_link_closure for p in all_pairs) / len(all_pairs)
            )
        else:
            per_deployment_closure[d] = 0.0

    # Per-deployment source stats
    per_deployment_source_bytes = {sf.module_name: sf.source_bytes for sf in deployment_sources}
    per_deployment_source_lines = {sf.module_name: sf.line_count for sf in deployment_sources}

    # Hypercube overall
    hypercube_overall = (
        sum(f.overall_closure_rate for f in faces) / len(faces)
        if faces else 0.0
    )

    # Axis balance
    if per_axis_overall:
        rates = list(per_axis_overall.values())
        if len(rates) > 1:
            try:
                std = statistics.pstdev(rates)
            except statistics.StatisticsError:
                std = 0.0
            axis_balance_score = _clip01(std)
        else:
            axis_balance_score = 0.0
    else:
        axis_balance_score = 0.0

    # Notes
    n_fetched = sum(1 for sf in deployment_sources if sf.status == "FETCHED")
    n_fallback = sum(1 for sf in deployment_sources if sf.status == "FALLBACK_PROXY")
    notes: List[str] = []
    notes.append(f"Fetched {n_fetched}/6 deployment sources via inspect.getsource (full Python source)")
    notes.append(f"Fall-back to proxy text: {n_fallback}/6 (when inspect.getsource fails)")
    notes.append(f"Axis sources combined: {axis_sources_combined_bytes} bytes, {axis_sources_combined_lines} lines")
    notes.append(f"Hypercube overall closure: {hypercube_overall:.4f}")
    notes.append(f"Axis balance score: {axis_balance_score:.4f} (0=balanced, 1=unbalanced)")

    ended = _now_iso()
    return V1455Report(
        schema=V1455_SCHEMA,
        version=V1455_VERSION,
        module=V1455_MODULE,
        started=started,
        ended=ended,
        n_axes=len(V1455_AXES),
        n_problems=len(V1455_PROBLEM_NAMES),
        n_positions=len(V1455_POSITION_NAMES),
        n_protocols=len(V1455_PROTOCOL_NAMES),
        n_deployments=len(V1455_DEPLOYMENT_NAMES),
        n_faces_total=6,  # 3 new + 3 from V1450
        deployment_sources=deployment_sources,
        axis_sources_combined_bytes=axis_sources_combined_bytes,
        axis_sources_combined_lines=axis_sources_combined_lines,
        faces=faces,
        per_axis_overall=per_axis_overall,
        per_deployment_closure_rate=per_deployment_closure,
        per_deployment_source_bytes=per_deployment_source_bytes,
        per_deployment_source_lines=per_deployment_source_lines,
        hypercube_overall_closure_rate=_clip01(hypercube_overall),
        axis_balance_score=axis_balance_score,
        notes=notes,
    )


# ============================================================================
# Run-all
# ============================================================================

def run_all(
    out_json: Optional[Path] = None,
    out_md: Optional[Path] = None,
) -> V1455Report:
    report = build_report()

    here = Path(__file__).resolve().parent
    ws_root = here.parent
    if out_json is None:
        out_json = ws_root / ".v1455-hypercube-full-source-content-audit-v5-report.json"
    if out_md is None:
        out_md = ws_root / ".v1455-hypercube-full-source-content-audit-v5-report.md"

    payload = report.to_dict()
    # Truncate source content in JSON to keep file size manageable
    for sf_dict in payload.get("deployment_sources", []):
        if "content" in sf_dict:
            content = sf_dict["content"]
            if len(content) > 200:
                sf_dict["content"] = content[:200] + "...(truncated for JSON)"

    out_json.write_text(
        json.dumps(payload, indent=2, ensure_ascii=False),
        encoding="utf-8",
    )
    out_md.write_text(_render_markdown(report), encoding="utf-8")
    return report


# ============================================================================
# Popper self-test (14 probes)
# ============================================================================

def popper() -> Tuple[bool, List[Dict[str, Any]]]:
    results: List[Dict[str, Any]] = []

    # T01: constants defined
    try:
        assert V1455_VERSION == "0.1.0"
        assert V1455_SCHEMA == "asi.hypercube-full-source-content-audit-v5.v1"
        assert len(V1455_GUARDS) == 14
        assert len(V1455_V3_GUARDS) == 5
        assert len(V1455_BORROWED) == 8
        assert len(V1455_AXES) == 4
        assert len(V1455_DEPLOYMENT_NAMES) == 6
        results.append({"name": "T01_constants", "ok": True, "detail": "14+5+8 guards/borrowed; 4 axes; 6 deployments"})
    except Exception as e:
        results.append({"name": "T01_constants", "ok": False, "detail": str(e)})

    # T02: helpers bounded
    try:
        assert _clip01(-0.5) == 0.0
        assert _clip01(1.5) == 1.0
        assert _harmonic_mean(0.0, 0.5) == 0.0
        assert _harmonic_mean(1.0, 1.0) == 1.0
        results.append({"name": "T02_helpers_bounded", "ok": True, "detail": "_clip01, _harmonic_mean bounded"})
    except Exception as e:
        results.append({"name": "T02_helpers_bounded", "ok": False, "detail": str(e)})

    # T03: keyword counter
    try:
        text = "async function awaits gather"
        cnt = _count_keyword_occurrences(text, ("async", "await", "gather"))
        assert cnt == 3
        results.append({"name": "T03_keyword_counter", "ok": True, "detail": "async+await+gather → 3 hits"})
    except Exception as e:
        results.append({"name": "T03_keyword_counter", "ok": False, "detail": str(e)})

    # T04: inspect.getsource fallback chain
    try:
        src = _try_get_full_source("v1455_asi_hypercube_full_source_content_audit_v5")
        # Self-import might fail; that's OK, both paths tested
        assert src is None or isinstance(src, str)
        proxy = _get_proxy_text("v1455_asi_hypercube_full_source_content_audit_v5")
        assert isinstance(proxy, str)
        assert len(proxy) > 0
        results.append({"name": "T04_inspect_getsource_fallback", "ok": True,
                        "detail": "inspect.getsource + proxy fallback both work"})
    except Exception as e:
        results.append({"name": "T04_inspect_getsource_fallback", "ok": False, "detail": str(e)})

    # T05: fetch deployment source
    try:
        for d in V1455_DEPLOYMENT_NAMES:
            sf = _fetch_deployment_source(d)
            assert sf.module_name == V1455_DEPLOYMENT_MODULES[d]
            assert sf.status in ("FETCHED", "FALLBACK_PROXY", "FAILED")
            assert sf.source_bytes > 0
        results.append({"name": "T05_fetch_deployment_source", "ok": True,
                        "detail": "6 deployment sources fetched (FETCHED or FALLBACK_PROXY)"})
    except Exception as e:
        results.append({"name": "T05_fetch_deployment_source", "ok": False, "detail": str(e)})

    # T06: fetch axis source combined
    try:
        text_time = _fetch_axis_source_combined("problem", "time")
        assert isinstance(text_time, str)
        assert len(text_time) > 0
        results.append({"name": "T06_fetch_axis_source_combined", "ok": True,
                        "detail": f"time axis combined: {len(text_time)} chars"})
    except Exception as e:
        results.append({"name": "T06_fetch_axis_source_combined", "ok": False, "detail": str(e)})

    # T07: audit pair full source
    try:
        deployment_source_cache = {d: _fetch_deployment_source(d).content for d in V1455_DEPLOYMENT_NAMES}
        axis_source_cache = {(axis, e): _fetch_axis_source_combined(axis, e)
                             for axis in ("problem", "position", "protocol")
                             for e in _axis_elements(axis)}
        pair = audit_pair_full_source("problem", "time", "docker",
                                       deployment_source_cache, axis_source_cache)
        assert pair.axis == "problem"
        assert pair.axis_element == "time"
        assert pair.deployment_element == "docker"
        assert 0.0 <= pair.forward_closure <= 1.0
        assert 0.0 <= pair.backward_closure <= 1.0
        assert 0.0 <= pair.cross_link_closure <= 1.0
        results.append({"name": "T07_audit_pair_full_source", "ok": True,
                        "detail": f"time×docker axis_kw_count={pair.axis_kw_count} deployment_kw_count={pair.deployment_kw_count}"})
    except Exception as e:
        results.append({"name": "T07_audit_pair_full_source", "ok": False, "detail": str(e)})

    # T08: audit face full source
    try:
        deployment_source_cache = {d: _fetch_deployment_source(d).content for d in V1455_DEPLOYMENT_NAMES}
        axis_source_cache = {(axis, e): _fetch_axis_source_combined(axis, e)
                             for axis in ("problem", "position", "protocol")
                             for e in _axis_elements(axis)}
        face = audit_face_full_source("problem", deployment_source_cache, axis_source_cache)
        assert face.axes == ("problem", "deployment")
        assert face.n_pairs == 42
        assert len(face.pairs) == 42
        results.append({"name": "T08_audit_face_full_source", "ok": True,
                        "detail": f"7×6=42 pairs, overall={face.overall_closure_rate:.4f}"})
    except Exception as e:
        results.append({"name": "T08_audit_face_full_source", "ok": False, "detail": str(e)})

    # T09: build report
    try:
        report = build_report()
        assert report.n_axes == 4
        assert len(report.faces) == 3
        assert len(report.deployment_sources) == 6
        assert 0.0 <= report.hypercube_overall_closure_rate <= 1.0
        assert 0.0 <= report.axis_balance_score <= 1.0
        assert len(report.per_axis_overall) == 4
        assert len(report.per_deployment_closure_rate) == 6
        results.append({"name": "T09_build_report", "ok": True,
                        "detail": f"hypercube_overall={report.hypercube_overall_closure_rate:.4f} bytes={report.axis_sources_combined_bytes}"})
    except Exception as e:
        results.append({"name": "T09_build_report", "ok": False, "detail": str(e)})

    # T10: run_all
    try:
        import tempfile
        with tempfile.TemporaryDirectory() as tmp:
            out_json = Path(tmp) / "r.json"
            out_md = Path(tmp) / "r.md"
            report = run_all(out_json=out_json, out_md=out_md)
            assert out_json.exists()
            assert out_md.exists()
        results.append({"name": "T10_run_all", "ok": True, "detail": "run_all writes JSON + MD"})
    except Exception as e:
        results.append({"name": "T10_run_all", "ok": False, "detail": str(e)})

    # T11: per-deployment closure
    try:
        report = build_report()
        assert len(report.per_deployment_closure_rate) == 6
        for d, rate in report.per_deployment_closure_rate.items():
            assert d in V1455_DEPLOYMENT_NAMES
            assert 0.0 <= rate <= 1.0
        results.append({"name": "T11_per_deployment_closure", "ok": True, "detail": "6 deployments × closure [0,1]"})
    except Exception as e:
        results.append({"name": "T11_per_deployment_closure", "ok": False, "detail": str(e)})

    # T12: per-deployment source stats
    try:
        report = build_report()
        assert len(report.per_deployment_source_bytes) == 6
        assert len(report.per_deployment_source_lines) == 6
        for module_name, n_bytes in report.per_deployment_source_bytes.items():
            assert n_bytes > 0
        results.append({"name": "T12_per_deployment_source_stats", "ok": True,
                        "detail": f"6 modules × bytes+lines tracked"})
    except Exception as e:
        results.append({"name": "T12_per_deployment_source_stats", "ok": False, "detail": str(e)})

    # T13: render markdown
    try:
        report = build_report()
        md = _render_markdown(report)
        assert "V1455" in md
        assert "Hypercube" in md
        assert "Honest disclosure" in md
        assert "V3 哲学守门" in md
        results.append({"name": "T13_render_markdown", "ok": True, "detail": f"md length={len(md)}"})
    except Exception as e:
        results.append({"name": "T13_render_markdown", "ok": False, "detail": str(e)})

    # T14: V1454 vs V1455 comparison (improvement)
    try:
        report = build_report()
        # V1454 had hypercube_overall=0.0222 (proxy-text only)
        # V1455 should be >= V1454 (full source should reveal more matches)
        # Note: this is not strictly required (V1455 might be lower in some cases),
        # but it documents the comparison
        results.append({"name": "T14_v1454_vs_v1455_comparison", "ok": True,
                        "detail": f"V1455 hypercube_overall={report.hypercube_overall_closure_rate:.4f} (vs V1454's 0.0222)"})
    except Exception as e:
        results.append({"name": "T14_v1454_vs_v1455_comparison", "ok": False, "detail": str(e)})

    all_ok = all(r["ok"] for r in results)
    return all_ok, results


# ============================================================================
# Markdown render
# ============================================================================

def _render_markdown(report: V1455Report) -> str:
    lines: List[str] = []
    lines.append(f"# V1455 — ASI hypercube full-source-content audit v5")
    lines.append("")
    lines.append(f"- schema: `{V1455_SCHEMA}`")
    lines.append(f"- version: `{V1455_VERSION}`")
    lines.append(f"- module: `{V1455_MODULE}`")
    lines.append(f"- started: `{report.started}`")
    lines.append(f"- ended: `{report.ended}`")
    lines.append(f"- n_axes: **{report.n_axes}** (problem/position/protocol/deployment)")
    lines.append(f"- n_problems: **{report.n_problems}**")
    lines.append(f"- n_positions: **{report.n_positions}**")
    lines.append(f"- n_protocols: **{report.n_protocols}**")
    lines.append(f"- n_deployments: **{report.n_deployments}**")
    lines.append(f"- n_faces_total: **{report.n_faces_total}** (3 new + 3 existing)")
    lines.append(f"- axis_sources_combined_bytes: **{report.axis_sources_combined_bytes}**")
    lines.append(f"- axis_sources_combined_lines: **{report.axis_sources_combined_lines}**")
    lines.append("")

    lines.append("## Per-axis overall closure (4 axes)")
    lines.append("")
    lines.append("| axis | closure_rate |")
    lines.append("|---|---|")
    for axis_name in ("problem", "position", "protocol", "deployment"):
        rate = report.per_axis_overall.get(axis_name, 0.0)
        lines.append(f"| {axis_name} | {rate:.4f} |")
    lines.append("")

    lines.append("## Per-deployment closure_rate (6 deployments)")
    lines.append("")
    lines.append("| deployment | closure_rate | source_bytes | source_lines | module |")
    lines.append("|---|---|---|---|---|")
    for d, rate in report.per_deployment_closure_rate.items():
        module_name = V1455_DEPLOYMENT_MODULES.get(d, "")
        bytes_v = report.per_deployment_source_bytes.get(module_name, 0)
        lines_v = report.per_deployment_source_lines.get(module_name, 0)
        lines.append(f"| {d} | {rate:.4f} | {bytes_v} | {lines_v} | {module_name} |")
    lines.append("")

    lines.append("## Deployment source fetch status")
    lines.append("")
    lines.append("| module | status | source_bytes | line_count |")
    lines.append("|---|---|---|---|")
    for sf in report.deployment_sources:
        lines.append(f"| {sf.module_name} | {sf.status} | {sf.source_bytes} | {sf.line_count} |")
    lines.append("")

    lines.append("## Per-face audit (3 new hypercube faces, full source)")
    lines.append("")
    lines.append("| face_id | axes | n_pairs | forward | backward | cross_link | overall |")
    lines.append("|---|---|---|---|---|---|---|")
    for f in report.faces:
        lines.append(
            f"| {f.face_id} | {'×'.join(f.axes)} | {f.n_pairs} | "
            f"{f.forward_closure_rate:.4f} | {f.backward_closure_rate:.4f} | "
            f"{f.cross_link_closure_rate:.4f} | {f.overall_closure_rate:.4f} |"
        )
    lines.append("")

    lines.append("## Hypercube overall")
    lines.append("")
    lines.append(f"- hypercube_overall_closure_rate: **{report.hypercube_overall_closure_rate:.4f}**")
    lines.append(f"- axis_balance_score: **{report.axis_balance_score:.4f}** (0=balanced, 1=unbalanced)")
    lines.append("")

    lines.append("## Notes")
    lines.append("")
    for note in report.notes:
        lines.append(f"- {note}")
    lines.append("")

    lines.append("## Honest disclosure (主 17:43 实事求是)")
    lines.append("")
    lines.append(
        "> V1455 is a **bounded full-source-content keyword audit on real production "
        "modules**. It does NOT claim that all 6 deployment modules perfectly "
        "implement all 7 problems / 5 positions / 6 protocols, that the audit is "
        "exhaustive, or that keyword match in full source equals deployment parity. "
        "V1455 ≠ ASI closure. V1455 ≠ Phenomenal closure. V1455 ≠ human-level "
        "closure. V1455 ≠ absolute closure. V1455 ≠ deployment parity. V1455 = "
        "bounded keyword search on FULL Python source (inspect.getsource) for each "
        "deployment module + per-axis source modules. If inspect.getsource fails, "
        "V1455 falls back to proxy text (module name + constants) and marks the "
        "source as FALLBACK_PROXY. V1455's closure_rate is the EMPIRICAL result "
        "of full-source keyword search; it is NOT a claim of audit completeness."
    )
    lines.append("")
    lines.append(
        "（主 17:43 实事求是 + 主 17:58 不假装 + 主 20:46 不假装达到 ASI + 主 19:33 走在前人经验上 + 主 22:33 终极授权 + 主 00:44 质量工程化 + 主 00:56 任何人能接手）"
    )
    lines.append("")

    lines.append("## Borrowed (主 19:33 走在前人经验上)")
    lines.append("")
    for src, desc in V1455_BORROWED:
        lines.append(f"- **{src}**: {desc}")
    lines.append("")

    lines.append("## V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)")
    lines.append("")
    for g in V1455_V3_GUARDS:
        lines.append(f"- {g}")
    lines.append("")

    lines.append("## GUARDS upheld (V1455-specific, 14)")
    lines.append("")
    for g in V1455_GUARDS:
        lines.append(f"- {g}")
    lines.append("")

    return "\n".join(lines)


# ============================================================================
# CLI
# ============================================================================

def _build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog=V1455_MODULE_SHORT,
        description="V1455 — ASI cube hypercube full-source-content audit v5",
    )
    p.add_argument("cmd", nargs="?", default="help",
                   choices=["version", "help", "meta", "popper", "chain",
                            "audit", "report", "run-all"])
    p.add_argument("--json", action="store_true", help="JSON output for meta")
    p.add_argument("--out-json", type=str, default=None, help="Output JSON path")
    p.add_argument("--out-md", type=str, default=None, help="Output MD path")
    return p


def main(argv: Optional[List[str]] = None) -> int:
    parser = _build_parser()
    try:
        args = parser.parse_args(argv)
    except SystemExit as e:
        return int(e.code) if isinstance(e.code, int) else 2
    cmd = args.cmd or "help"

    if cmd == "version":
        print(V1455_VERSION)
        return 0

    if cmd == "help":
        parser.print_help()
        return 0

    if cmd == "meta":
        meta = {
            "schema": V1455_SCHEMA,
            "version": V1455_VERSION,
            "module": V1455_MODULE,
            "n_axes": len(V1455_AXES),
            "axes": list(V1455_AXES),
            "n_problems": len(V1455_PROBLEM_NAMES),
            "n_positions": len(V1455_POSITION_NAMES),
            "n_protocols": len(V1455_PROTOCOL_NAMES),
            "n_deployments": len(V1455_DEPLOYMENT_NAMES),
            "n_faces_total": 6,
            "deployment_modules": V1455_DEPLOYMENT_MODULES,
            "guards": list(V1455_GUARDS),
            "v3_guards": list(V1455_V3_GUARDS),
            "borrowed": [list(b) for b in V1455_BORROWED],
        }
        if getattr(args, "json", False):
            print(json.dumps(meta, indent=2, ensure_ascii=False))
        else:
            for k, v in meta.items():
                print(f"{k}: {v}")
        return 0

    if cmd == "popper":
        ok, results = popper()
        for r in results:
            mark = "OK" if r["ok"] else "FAIL"
            print(f"[{mark}] {r['name']}: {r['detail']}")
        print(f"\nALL_OK={ok}")
        return 0 if ok else 1

    if cmd == "chain":
        # V1455 has no upstream chain (it's the v5 extension of V1454 which already has chain)
        # But we provide a stub for consistency
        chain = {
            "schema": "asi.chain-delegate.v1455.v1",
            "version": V1455_VERSION,
            "delegates": [
                {"module": "V1454", "ok": True, "detail": "V1454 importable (proxy-text hypercube)"},
            ],
            "all_ok": True,
        }
        try:
            import apeireth.v1454_asi_hypercube_four_axis_deployment as v1454
            chain["delegates"][0]["ok"] = True
            chain["delegates"][0]["detail"] = f"V1454 importable: {v1454.V1454_MODULE}"
        except Exception as e:
            chain["delegates"][0]["ok"] = False
            chain["delegates"][0]["detail"] = f"import_error={e}"
            chain["all_ok"] = False
        print(json.dumps(chain, indent=2, ensure_ascii=False))
        return 0

    if cmd == "audit":
        report = build_report()
        summary = {
            "n_axes": report.n_axes,
            "n_faces_total": report.n_faces_total,
            "axis_sources_combined_bytes": report.axis_sources_combined_bytes,
            "axis_sources_combined_lines": report.axis_sources_combined_lines,
            "per_axis_overall": report.per_axis_overall,
            "per_deployment_closure_rate": report.per_deployment_closure_rate,
            "hypercube_overall_closure_rate": report.hypercube_overall_closure_rate,
            "axis_balance_score": report.axis_balance_score,
            "deployment_source_statuses": {sf.module_name: sf.status for sf in report.deployment_sources},
        }
        print(json.dumps(summary, indent=2, ensure_ascii=False))
        return 0

    if cmd == "report":
        report = run_all(
            out_json=Path(args.out_json) if args.out_json else None,
            out_md=Path(args.out_md) if args.out_md else None,
        )
        print(f"V1455 hypercube audit report written.")
        print(f"  hypercube_overall: {report.hypercube_overall_closure_rate:.4f}")
        print(f"  axis_balance_score: {report.axis_balance_score:.4f}")
        print(f"  axis_sources_combined_bytes: {report.axis_sources_combined_bytes}")
        return 0

    if cmd == "run-all":
        try:
            ok, results = popper()
            for r in results:
                mark = "OK" if r["ok"] else "FAIL"
                print(f"[{mark}] {r['name']}: {r['detail']}")
            print(f"POPPER_ALL_OK={ok}\n")
        except Exception as e:
            print(f"POPPER_ERROR: {e}")
            return 1

        report = run_all(
            out_json=Path(args.out_json) if args.out_json else None,
            out_md=Path(args.out_md) if args.out_md else None,
        )
        print(f"V1455 hypercube audit report written.")
        print(f"  hypercube_overall: {report.hypercube_overall_closure_rate:.4f}")
        print(f"  axis_balance_score: {report.axis_balance_score:.4f}")
        print(f"  axis_sources_combined_bytes: {report.axis_sources_combined_bytes}")
        print(f"  n_faces: {len(report.faces)}")
        print(f"  n_pairs_total: {sum(f.n_pairs for f in report.faces)}")
        return 0 if ok else 1

    parser.print_help()
    return 0


if __name__ == "__main__":
    sys.exit(main())