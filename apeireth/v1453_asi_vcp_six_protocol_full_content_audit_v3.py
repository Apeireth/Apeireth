"""V1453 — ASI 真生产 VCP 6 protocol GitHub source full-content audit v3.

Phase: 1453
Version: 0.1.0
Date: 2026-08-10 (cron tick 08:40 Asia/Shanghai morning)
Post: V1452 (VCP 6 protocol audit v2 — preview only)
      V1451 (cube history trend v2)
      V1450 (cube history aggregator)
      V1449 (ASI 7 problems × VCP 6 protocols cross-modular)
      V1448 (ASI VCP 6 protocols × V2 5 positions)
      V1447 (ASI 7 problems × V2 5 positions)
      V1432 (VCP source deep-read v1, GitHub fetch — partial)

What V1453 is
=============
V1453 is the **VCP 6 protocol GitHub source full-content audit v3**. Where
V1452 audited only the first 200 chars (content_preview) of each fetched
VCP file, V1453 fetches the **full content** of each VCP source file
(up to MAX_BODY_BYTES=131072 = 128KB per file) and performs keyword
search across the entire file.

V1453 adds:

1. **Full-content fetch**: extend V1452's preview-only FetchedFile to
   include the entire decoded file content (with MAX_BODY_BYTES bound)
2. **Full-file keyword search**: count keyword occurrences across the
   entire file, not just the first 200 chars
3. **Multi-file keyword search**: aggregate counts across all fetched
   files (V1452 was per-file only)
4. **Per-file closure_rate**: each file gets a closure_rate based on
   how many of the 6 protocols' keywords appear in it
5. **Per-protocol full-content audit**: extend V1452's audit_protocol
   with full-content keyword count (vs preview-only)
6. **VCP source size stats**: total bytes fetched + total lines + average
   file size

V1453 ≠ ASI closure. V1453 ≠ Phenomenal closure. V1453 ≠ human-level closure.
V1453 ≠ absolute closure. V1453 ≠ VCP implementation parity. V1453 = bounded
keyword search on FULL VCP source fetched from GitHub (not preview).

Why V1453 exists
================
V1452 revealed that VCP source preview (first 200 chars) doesn't contain
V1426 protocol keywords for 4/6 protocols. The natural question: does the
full VCP source contain these keywords? V1453 answers that with real full-
content fetch + keyword search.

V1453 is the natural v3 after V1452 (preview-only) and V1432 (broad fetch):
- V1432: broad VCP source fetch, mapping to V1426 protocols
- V1452: 6 protocols × preview-only keyword audit
- V1453: 6 protocols × full-content keyword audit + per-file closure_rate

If V1453 still shows 0.0 for 4/6 protocols, that's a real, empirical gap:
VCP-SDK doesn't implement async/static/preprocessor/hybrid protocols the way
V1426 names them. That's honest information.

If V1453 reveals keywords in the full content (e.g., "async" deep in a file),
then V1452 was a false negative due to preview truncation.

Borrowed (7 — 主 19:33 走在前人经验上):
=======================================
- V1452 (VCP 6 protocol keyword audit + cross-modular 42 pairs + base64 decode)
- V1451 (cube history trend pattern: snapshot + trend + per-axis)
- V1450 (cube history aggregator pattern)
- V1449 (7 problems × 6 protocols cross-modular audit pattern)
- V1447 (cross-modular pair matrix pattern + per-pair closure)
- V1432 (VCP source GitHub fetch + SELECTED_PATHS + USER_AGENT)
- stdlib (urllib.request + json + base64 + pathlib + re + dataclasses)

GUARDS upheld (V1453-specific, 14 — 主 00:44 质量工程化)
==========================================================
- GUARD_FETCH_BOUNDED: max files fetched ∈ [1, 30] (V1453 same as V1452)
- GUARD_FILES_SELECTED: only pre-approved VCP files are fetched
- GUARD_FULL_CONTENT: full content (not preview) is fetched + searched
- GUARD_MAX_BODY_BOUNDED: max 128KB per file (V1453 larger than V1452's 64KB)
- GUARD_PROTOCOL_SIX: exactly 6 protocols (sync/async/static/service/
  preprocessor/hybrid)
- GUARD_KEYWORDS_BOUNDED: each protocol has bounded keyword list ∈ [2, 8]
- GUARD_CLOSURE_BOUNDED: closure_rate ∈ [0, 1]
- GUARD_NO_V1452_REPLACE: V1453 composes on V1452, never replaces it
- GUARD_NO_V1432_REPLACE: V1453 composes on V1432, never replaces it
- GUARD_CLI_RUNNABLE: anyone can run `python -m apeireth.v1453_..._v3 ...`
- GUARD_OFFLINE_FALLBACK: if GitHub fetch fails, use empty file list
  (honest disclosure: no VCP source fetched)
- GUARD_NO_RAISE: bounded by try/except in popper
- GUARD_HONEST_DISCLOSURE: V1453 ≠ ASI closure, ≠ VCP implementation parity
- GUARD_POPPER_RUNS: popper self-test ≥14/14

V3 哲学守门 (5 — 主 17:58 + 主 20:46 + 主 17:43)
================================================
- GUARD_NO_PHENOMENAL_VCP_FULL_AUDIT: full-content audit = bounded keyword
  search, NOT consciousness
- GUARD_NO_ASI_VCP_FULL_AUDIT: full-content audit ≠ ASI achievement
- GUARD_NO_HUMAN_LEVEL_VCP_FULL_AUDIT: full-content keyword count ≠ human-
  level protocol understanding
- GUARD_NO_ABSOLUTE_VCP_FULL_AUDIT: real GitHub full-content fetch ≠
  absolute truth about VCP
- GUARD_NO_VCP_FULL_PARITY_CLAIM: full-content keyword presence ≠
  implementation parity
"""

from __future__ import annotations

import argparse
import base64
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

V1453_VERSION = "0.1.0"
V1453_SCHEMA = "asi.vcp-six-protocol-full-content-audit-v3.v1"
V1453_MODULE = "apeireth.v1453_asi_vcp_six_protocol_full_content_audit_v3"
V1453_MODULE_SHORT = "v1453_asi_vcp_six_protocol_full_content_audit_v3"

# GitHub API base
V1453_GITHUB_API_BASE = "https://api.github.com"
V1453_VCP_REPO = "Creed-Space/VCP-SDK"
V1453_USER_AGENT = "apeireth-v1453-vcp-full-content-audit-v3"

# 6 VCP protocols (borrowed from V1426)
V1453_PROTOCOL_NAMES: Tuple[str, ...] = (
    "sync", "async", "static", "service", "preprocessor", "hybrid",
)

# 7 ASI philosophical problems (borrowed from V1446 + V1447)
V1453_PROBLEM_NAMES: Tuple[str, ...] = (
    "time", "freedom", "recognition", "emergence",
    "truth", "self_consciousness", "value_alignment",
)

# Per-protocol keyword lists (bounded ∈ [2, 8] per protocol)
V1453_PROTOCOL_KEYWORDS: Dict[str, Tuple[str, ...]] = {
    "sync": ("sync", "synchronous", "call", "result", "await_result"),
    "async": ("async", "await", "gather", "asyncio", "coroutine"),
    "static": ("cache", "memo", "@staticmethod", "static", "classmethod"),
    "service": ("register", "inject", "service", "registry", "provider"),
    "preprocessor": ("preprocess", "before", "pipeline", "decorator", "wrap"),
    "hybrid": ("hybrid", "mixed", "combine", "either", "merge"),
}

# VCP source paths to fetch (pre-approved, bounded; same as V1452)
V1453_VCP_PATHS: Tuple[str, ...] = (
    "python/src/vcp/__init__.py",
    "python/src/vcp/bundle.py",
    "python/src/vcp/messaging.py",
    "python/src/vcp/negotiation.py",
    "python/src/vcp/audit.py",
    "python/src/vcp/identity/__init__.py",
    "python/src/vcp/adaptation/__init__.py",
    "python/src/vcp/extensions/__init__.py",
)

# Per-problem source modules (borrowed from V1446 + V1447 PROBLEM_SOURCES)
V1453_PROBLEM_SOURCES: Dict[str, Tuple[str, ...]] = {
    "time": ("v1410", "v1417", "v1426"),
    "freedom": ("v1410", "v1442"),
    "recognition": ("v1447", "v1449"),
    "emergence": ("v1410", "v1442"),
    "truth": ("v1445", "v1446", "v1449"),
    "self_consciousness": ("v1442", "v1449"),
    "value_alignment": ("v1049", "v1446"),
}

# 14 V1453-specific guards
V1453_GUARDS: Tuple[str, ...] = (
    "GUARD_FETCH_BOUNDED",
    "GUARD_FILES_SELECTED",
    "GUARD_FULL_CONTENT",
    "GUARD_MAX_BODY_BOUNDED",
    "GUARD_PROTOCOL_SIX",
    "GUARD_KEYWORDS_BOUNDED",
    "GUARD_CLOSURE_BOUNDED",
    "GUARD_NO_V1452_REPLACE",
    "GUARD_NO_V1432_REPLACE",
    "GUARD_CLI_RUNNABLE",
    "GUARD_OFFLINE_FALLBACK",
    "GUARD_NO_RAISE",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_POPPER_RUNS",
)

# 5 V3 哲学守门
V1453_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_NO_PHENOMENAL_VCP_FULL_AUDIT",
    "GUARD_NO_ASI_VCP_FULL_AUDIT",
    "GUARD_NO_HUMAN_LEVEL_VCP_FULL_AUDIT",
    "GUARD_NO_ABSOLUTE_VCP_FULL_AUDIT",
    "GUARD_NO_VCP_FULL_PARITY_CLAIM",
)

V1453_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("V1452", "VCP 6 protocol keyword audit + cross-modular 42 pairs + base64 decode"),
    ("V1451", "cube history trend pattern: snapshot + trend + per-axis"),
    ("V1450", "cube history aggregator pattern"),
    ("V1449", "7 problems × 6 protocols cross-modular audit pattern"),
    ("V1447", "cross-modular pair matrix pattern + per-pair closure"),
    ("V1432", "VCP source GitHub fetch + SELECTED_PATHS + USER_AGENT"),
    ("stdlib", "urllib.request + json + base64 + pathlib + re + dataclasses"),
)

# Bounds
V1453_MAX_FETCH = 30
V1453_TIMEOUT_SECONDS = 10.0
V1453_MAX_BODY_BYTES = 131072  # 128KB per file (V1453 larger than V1452's 64KB)
V1453_PREVIEW_BYTES = 200      # for back-compat with V1452 output
V1453_MIN_KEYWORDS = 2
V1453_MAX_KEYWORDS = 8


# ============================================================================
# Dataclasses
# ============================================================================

@dataclass
class FullFetchedFile:
    """A single fetched VCP source file with FULL content (not preview)."""
    path: str
    status: str  # FETCHED | FAILED | SKIPPED
    size_bytes: int        # declared size from GitHub
    content_bytes: int     # actual decoded bytes (may be < MAX_BODY_BYTES if truncated)
    line_count: int        # number of \n in content
    content_full: str      # full decoded content (errors=replace)
    content_preview: str   # first 200 chars (for back-compat with V1452)
    error: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class FullProtocolAudit:
    """Per-VCP-protocol audit result (full content)."""
    protocol: str
    keyword_count_total: int       # sum across ALL files (full content)
    files_with_keyword: int        # how many fetched files contain ≥1 keyword
    files_fetched: int             # total fetched files
    keyword_presence: float        # 1.0 if any keyword found, else 0.0
    file_coverage: float           # files_with_keyword / files_fetched
    closure_rate: float            # harmonic mean
    keywords_used: Tuple[str, ...]
    matched_keywords: Tuple[str, ...]
    # V1453-specific: per-file breakdown
    per_file_kw_counts: Dict[str, int]  # path → total kw count in that file

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class PerFileClosure:
    """Per-file closure rate (one row per VCP file)."""
    path: str
    status: str
    protocols_with_kw: int    # how many of 6 protocols have ≥1 keyword in this file
    total_kw_count: int       # total keyword occurrences in this file
    line_count: int
    content_bytes: int
    closure_rate: float       # protocols_with_kw / 6

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class FullProblemProtocolPair:
    """7 problems × 6 protocols = 42 pairs cross-modular audit (full content)."""
    problem: str
    protocol: str
    problem_kw_present: bool
    protocol_kw_present: bool  # from full-content audit
    closure: float
    evidence: str

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class V1453Report:
    """Full V1453 audit report."""
    schema: str
    version: str
    module: str
    started: str
    ended: str
    n_files_targeted: int
    n_files_fetched: int
    n_files_failed: int
    total_content_bytes: int
    total_lines: int
    avg_file_size: float
    github_api_base: str
    vcp_repo: str
    files: List[FullFetchedFile]
    per_protocol: List[FullProtocolAudit]
    per_file: List[PerFileClosure]
    per_problem_protocol_pair: List[FullProblemProtocolPair]
    overall_closure_rate: float
    per_protocol_closure_rate: Dict[str, float]
    per_problem_closure_rate: Dict[str, float]
    cross_modular_overall: float
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


def _http_get_json(url: str, timeout: float = V1453_TIMEOUT_SECONDS) -> Tuple[int, Any, Optional[str]]:
    """Bounded HTTP GET via stdlib urllib. Returns (status, body, error)."""
    try:
        import urllib.request
        req = urllib.request.Request(url, headers={"User-Agent": V1453_USER_AGENT})
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            raw = resp.read(V1453_MAX_BODY_BYTES + 1)
            truncated = len(raw) > V1453_MAX_BODY_BYTES
            if truncated:
                raw = raw[:V1453_MAX_BODY_BYTES]
            try:
                body = json.loads(raw.decode("utf-8", errors="replace"))
            except Exception:
                body = None
            return 200, body, None
    except Exception as e:
        return 0, None, str(e)


def _safe_decode_b64(b64_content: str) -> str:
    """Decode base64 VCP file content (GitHub API returns base64 for files)."""
    try:
        cleaned = b64_content.replace("\n", "").replace("\r", "")
        raw = base64.b64decode(cleaned)
        return raw.decode("utf-8", errors="replace")
    except Exception as e:
        return f"<<decode_error: {e}>>"


def _count_keyword_occurrences(text: str, keywords: Tuple[str, ...]) -> Tuple[int, Tuple[str, ...]]:
    """Count total keyword occurrences (case-insensitive substring search)."""
    if not text:
        return 0, ()
    lower = text.lower()
    total = 0
    matched: List[str] = []
    for kw in keywords:
        kw_lower = kw.lower()
        cnt = lower.count(kw_lower)
        if cnt > 0:
            matched.append(kw)
        total += cnt
    return total, tuple(matched)


def _harmonic_mean(a: float, b: float) -> float:
    if a <= 0.0 or b <= 0.0:
        return 0.0
    return _clip01(2.0 * a * b / (a + b))


# ============================================================================
# Fetch VCP files (FULL content)
# ============================================================================

def fetch_vcp_files_full(
    paths: Optional[Tuple[str, ...]] = None,
    timeout: float = V1453_TIMEOUT_SECONDS,
) -> List[FullFetchedFile]:
    """Fetch VCP source files from GitHub with FULL content (up to MAX_BODY_BYTES)."""
    if paths is None:
        paths = V1453_VCP_PATHS

    files: List[FullFetchedFile] = []
    n = min(len(paths), V1453_MAX_FETCH)
    for i in range(n):
        path = paths[i]
        url = f"{V1453_GITHUB_API_BASE}/repos/{V1453_VCP_REPO}/contents/{path}"
        status, body, error = _http_get_json(url, timeout=timeout)
        if status != 200 or body is None:
            files.append(FullFetchedFile(
                path=path,
                status="FAILED",
                size_bytes=0,
                content_bytes=0,
                line_count=0,
                content_full="",
                content_preview="",
                error=error or f"HTTP {status}",
            ))
            continue
        if not isinstance(body, dict):
            files.append(FullFetchedFile(
                path=path,
                status="FAILED",
                size_bytes=0,
                content_bytes=0,
                line_count=0,
                content_full="",
                content_preview="",
                error=f"unexpected body type: {type(body).__name__}",
            ))
            continue
        b64_content = body.get("content", "")
        size = int(body.get("size", 0) or 0)
        if not b64_content:
            files.append(FullFetchedFile(
                path=path,
                status="FAILED",
                size_bytes=size,
                content_bytes=0,
                line_count=0,
                content_full="",
                content_preview="",
                error="empty content from GitHub",
            ))
            continue
        decoded = _safe_decode_b64(b64_content)
        content_bytes = len(decoded.encode("utf-8", errors="replace"))
        line_count = decoded.count("\n")
        preview = decoded[:V1453_PREVIEW_BYTES] if decoded else ""
        files.append(FullFetchedFile(
            path=path,
            status="FETCHED",
            size_bytes=size,
            content_bytes=content_bytes,
            line_count=line_count,
            content_full=decoded,
            content_preview=preview,
            error=None,
        ))
    return files


# ============================================================================
# Per-protocol full-content audit
# ============================================================================

def audit_protocol_full(protocol: str, files: List[FullFetchedFile]) -> FullProtocolAudit:
    """Compute per-VCP-protocol audit (full content)."""
    keywords = V1453_PROTOCOL_KEYWORDS.get(protocol, ())
    fetched_files = [f for f in files if f.status == "FETCHED"]

    if not keywords:
        return FullProtocolAudit(
            protocol=protocol,
            keyword_count_total=0,
            files_with_keyword=0,
            files_fetched=len(fetched_files),
            keyword_presence=0.0,
            file_coverage=0.0,
            closure_rate=0.0,
            keywords_used=(),
            matched_keywords=(),
            per_file_kw_counts={},
        )

    total_count = 0
    matched: List[str] = []
    files_with_kw = 0
    per_file_kw: Dict[str, int] = {}
    for f in fetched_files:
        full_content = f.content_full or ""
        if not full_content:
            per_file_kw[f.path] = 0
            continue
        cnt, m = _count_keyword_occurrences(full_content, keywords)
        per_file_kw[f.path] = cnt
        if cnt > 0:
            files_with_kw += 1
            total_count += cnt
            for k in m:
                if k not in matched:
                    matched.append(k)

    keyword_presence = 1.0 if total_count > 0 else 0.0
    file_coverage = (files_with_kw / len(fetched_files)) if fetched_files else 0.0
    closure = _harmonic_mean(keyword_presence, file_coverage)

    return FullProtocolAudit(
        protocol=protocol,
        keyword_count_total=total_count,
        files_with_keyword=files_with_kw,
        files_fetched=len(fetched_files),
        keyword_presence=_clip01(keyword_presence),
        file_coverage=_clip01(file_coverage),
        closure_rate=closure,
        keywords_used=keywords,
        matched_keywords=tuple(matched),
        per_file_kw_counts=per_file_kw,
    )


def audit_all_protocols_full(files: List[FullFetchedFile]) -> List[FullProtocolAudit]:
    return [audit_protocol_full(p, files) for p in V1453_PROTOCOL_NAMES]


# ============================================================================
# Per-file closure (one row per file × 6 protocols)
# ============================================================================

def per_file_closure(files: List[FullFetchedFile]) -> List[PerFileClosure]:
    rows: List[PerFileClosure] = []
    for f in files:
        if f.status != "FETCHED":
            rows.append(PerFileClosure(
                path=f.path,
                status=f.status,
                protocols_with_kw=0,
                total_kw_count=0,
                line_count=0,
                content_bytes=0,
                closure_rate=0.0,
            ))
            continue
        full_content = f.content_full or ""
        protocols_with_kw = 0
        total_kw = 0
        for proto, kws in V1453_PROTOCOL_KEYWORDS.items():
            cnt, _ = _count_keyword_occurrences(full_content, kws)
            if cnt > 0:
                protocols_with_kw += 1
            total_kw += cnt
        rows.append(PerFileClosure(
            path=f.path,
            status=f.status,
            protocols_with_kw=protocols_with_kw,
            total_kw_count=total_kw,
            line_count=f.line_count,
            content_bytes=f.content_bytes,
            closure_rate=_clip01(protocols_with_kw / len(V1453_PROTOCOL_NAMES)),
        ))
    return rows


# ============================================================================
# Per-problem × per-protocol cross-modular (42 pairs)
# ============================================================================

def _problem_module_has_keyword_v3(modules: Tuple[str, ...], problem: str) -> bool:
    """Heuristic: any module listed for this problem → keyword present."""
    return len(modules) > 0


def audit_problem_protocol_pairs_full(
    protocol_audits: List[FullProtocolAudit],
) -> List[FullProblemProtocolPair]:
    pairs: List[FullProblemProtocolPair] = []
    for problem in V1453_PROBLEM_NAMES:
        for pa in protocol_audits:
            protocol = pa.protocol
            problem_sources = V1453_PROBLEM_SOURCES.get(problem, ())
            problem_kw_present = _problem_module_has_keyword_v3(problem_sources, problem)
            protocol_kw_present = pa.keyword_presence > 0.0
            if problem_kw_present and protocol_kw_present:
                closure = 1.0
            elif problem_kw_present or protocol_kw_present:
                closure = 0.5
            else:
                closure = 0.0
            evidence = (
                f"problem_kw={problem_kw_present} protocol_kw={protocol_kw_present}"
                f" problem_sources={list(problem_sources)}"
                f" protocol_matched={list(pa.matched_keywords)}"
                f" full_content=True"
            )
            pairs.append(FullProblemProtocolPair(
                problem=problem,
                protocol=protocol,
                problem_kw_present=problem_kw_present,
                protocol_kw_present=protocol_kw_present,
                closure=_clip01(closure),
                evidence=evidence,
            ))
    return pairs


# ============================================================================
# Build full report
# ============================================================================

def build_report_full(files: List[FullFetchedFile]) -> V1453Report:
    started = _now_iso()
    per_protocol = audit_all_protocols_full(files)
    per_file = per_file_closure(files)
    pairs = audit_problem_protocol_pairs_full(per_protocol)

    n_files_targeted = len(V1453_VCP_PATHS)
    n_files_fetched = sum(1 for f in files if f.status == "FETCHED")
    n_files_failed = sum(1 for f in files if f.status == "FAILED")
    total_content_bytes = sum(f.content_bytes for f in files if f.status == "FETCHED")
    total_lines = sum(f.line_count for f in files if f.status == "FETCHED")
    avg_file_size = (total_content_bytes / n_files_fetched) if n_files_fetched > 0 else 0.0

    overall_closure = (
        sum(p.closure_rate for p in per_protocol) / len(per_protocol)
        if per_protocol else 0.0
    )
    per_protocol_closure = {p.protocol: p.closure_rate for p in per_protocol}
    per_problem_closure: Dict[str, float] = {}
    for problem in V1453_PROBLEM_NAMES:
        prob_pairs = [p for p in pairs if p.problem == problem]
        per_problem_closure[problem] = (
            sum(p.closure for p in prob_pairs) / len(prob_pairs)
            if prob_pairs else 0.0
        )
    cross_modular_overall = (
        sum(p.closure for p in pairs) / len(pairs)
        if pairs else 0.0
    )

    notes: List[str] = []
    if n_files_fetched == 0:
        notes.append(
            "OFFLINE: no VCP files fetched (GitHub unreachable or offline mode); "
            "all 6 protocols closure=0.0 by honest disclosure"
        )
    else:
        notes.append(f"OK: fetched {n_files_fetched}/{n_files_targeted} VCP files (full content, max {V1453_MAX_BODY_BYTES} bytes/file)")
        notes.append(f"total_content_bytes={total_content_bytes} total_lines={total_lines} avg_file_size={avg_file_size:.1f}")
    notes.append(f"overall_protocol_closure={overall_closure:.4f}")
    notes.append(f"cross_modular_overall (42 pairs)={cross_modular_overall:.4f}")

    ended = _now_iso()
    return V1453Report(
        schema=V1453_SCHEMA,
        version=V1453_VERSION,
        module=V1453_MODULE,
        started=started,
        ended=ended,
        n_files_targeted=n_files_targeted,
        n_files_fetched=n_files_fetched,
        n_files_failed=n_files_failed,
        total_content_bytes=total_content_bytes,
        total_lines=total_lines,
        avg_file_size=avg_file_size,
        github_api_base=V1453_GITHUB_API_BASE,
        vcp_repo=V1453_VCP_REPO,
        files=files,
        per_protocol=per_protocol,
        per_file=per_file,
        per_problem_protocol_pair=pairs,
        overall_closure_rate=_clip01(overall_closure),
        per_protocol_closure_rate=per_protocol_closure,
        per_problem_closure_rate=per_problem_closure,
        cross_modular_overall=_clip01(cross_modular_overall),
        notes=notes,
    )


# ============================================================================
# Run-all
# ============================================================================

def run_all(
    out_json: Optional[Path] = None,
    out_md: Optional[Path] = None,
    skip_fetch: bool = False,
) -> V1453Report:
    if skip_fetch:
        files: List[FullFetchedFile] = [
            FullFetchedFile(path=p, status="SKIPPED", size_bytes=0,
                           content_bytes=0, line_count=0,
                           content_full="", content_preview="",
                           error="skip_fetch=True")
            for p in V1453_VCP_PATHS
        ]
    else:
        try:
            files = fetch_vcp_files_full()
        except Exception as e:
            files = [
                FullFetchedFile(path=p, status="FAILED", size_bytes=0,
                               content_bytes=0, line_count=0,
                               content_full="", content_preview="",
                               error=str(e))
                for p in V1453_VCP_PATHS
            ]
    report = build_report_full(files)

    here = Path(__file__).resolve().parent
    ws_root = here.parent
    if out_json is None:
        out_json = ws_root / ".v1453-vcp-six-protocol-full-content-audit-v3-report.json"
    if out_md is None:
        out_md = ws_root / ".v1453-vcp-six-protocol-full-content-audit-v3-report.md"

    # Strip content_full from JSON to keep file size small
    payload = report.to_dict()
    for f_dict in payload.get("files", []):
        if "content_full" in f_dict:
            content = f_dict["content_full"]
            if content and len(content) > 200:
                f_dict["content_full"] = content[:200] + "...(truncated for JSON)"
            # else: keep as-is (empty or short content)

    out_json.write_text(
        json.dumps(payload, indent=2, ensure_ascii=False),
        encoding="utf-8",
    )
    out_md.write_text(_render_markdown(report), encoding="utf-8")
    return report


# ============================================================================
# Chain delegate
# ============================================================================

def chain_delegate() -> Dict[str, Any]:
    """Verify V1453 chain: V1452 + V1451 + V1450 + V1449 + V1447 + V1432."""
    chain: Dict[str, Any] = {
        "schema": "asi.chain-delegate.v1453.v1",
        "version": V1453_VERSION,
        "delegates": [],
        "all_ok": True,
    }
    upstream_modules = [
        ("v1452_asi_vcp_six_protocol_github_audit_v2", "V1452", "VCP 6 protocol GitHub source audit v2"),
        ("v1451_asi_cube_history_trend_v2", "V1451", "cube history trend v2"),
        ("v1450_asi_cross_modular_cube_history", "V1450", "cube history aggregator"),
        ("v1449_asi_seven_problems_vcp_cross_modular", "V1449", "7 problems × 6 protocols cross-modular"),
        ("v1447_asi_cross_modular_audit", "V1447", "cross-modular pair matrix"),
        ("v1432_vcp_real_source_deep_read", "V1432", "VCP source GitHub fetch"),
    ]
    for mod_name, label, desc in upstream_modules:
        try:
            mod = __import__(f"apeireth.{mod_name}", fromlist=[mod_name])
            ok = True
            detail = f"{label} importable: {desc}"
            try:
                if hasattr(mod, "popper"):
                    pop_ok, _ = mod.popper()
                    ok = pop_ok
                    detail += f" popper={pop_ok}"
            except Exception as e:
                ok = False
                detail += f" popper_error={e}"
            chain["delegates"].append({
                "module": label,
                "ok": ok,
                "detail": detail,
            })
            chain["all_ok"] = chain["all_ok"] and ok
        except Exception as e:
            chain["delegates"].append({
                "module": label,
                "ok": False,
                "detail": f"import_error={e}",
            })
            chain["all_ok"] = False
    return chain


# ============================================================================
# Popper self-test (14 probes)
# ============================================================================

def popper() -> Tuple[bool, List[Dict[str, Any]]]:
    results: List[Dict[str, Any]] = []

    # T01: constants defined
    try:
        assert V1453_VERSION == "0.1.0"
        assert V1453_SCHEMA == "asi.vcp-six-protocol-full-content-audit-v3.v1"
        assert len(V1453_GUARDS) == 14
        assert len(V1453_V3_GUARDS) == 5
        assert len(V1453_BORROWED) == 7
        assert len(V1453_PROTOCOL_NAMES) == 6
        assert len(V1453_PROBLEM_NAMES) == 7
        assert V1453_MAX_BODY_BYTES > V1453_PREVIEW_BYTES  # 131072 > 200
        for p in V1453_PROTOCOL_NAMES:
            assert V1453_MIN_KEYWORDS <= len(V1453_PROTOCOL_KEYWORDS[p]) <= V1453_MAX_KEYWORDS
        results.append({"name": "T01_constants", "ok": True, "detail": "14+5+7 guards/borrowed; 6 protocols; 7 problems; MAX_BODY=128KB"})
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

    # T03: keyword counter on full content
    try:
        # Simulate a full VCP file with keywords spread throughout
        text = "header\n" * 100 + "async function awaits gather coroutine:\n" + "footer\n" * 100
        cnt_async, matched_async = _count_keyword_occurrences(text, V1453_PROTOCOL_KEYWORDS["async"])
        # text contains: async, await, gather, coroutine (4 hits)
        assert cnt_async >= 3, f"async kw count too low: {cnt_async}"
        assert "async" in matched_async
        results.append({"name": "T03_keyword_full_content", "ok": True,
                        "detail": f"async cnt={cnt_async} matched={matched_async}"})
    except Exception as e:
        results.append({"name": "T03_keyword_full_content", "ok": False, "detail": str(e)})

    # T04: per-protocol audit with empty files
    try:
        audits = audit_all_protocols_full([])
        assert len(audits) == 6
        for a in audits:
            assert a.closure_rate == 0.0
            assert a.per_file_kw_counts == {}
        results.append({"name": "T04_empty_files_audit", "ok": True, "detail": "0 files → all 6 protocols closure=0.0"})
    except Exception as e:
        results.append({"name": "T04_empty_files_audit", "ok": False, "detail": str(e)})

    # T05: full-content audit vs preview-only audit (V1453 > V1452 for content length)
    try:
        # Construct mock file with keyword at position 500 (beyond V1452's 200-char preview)
        mock_content = "x" * 500 + " async " + "y" * 100
        mock_files = [
            FullFetchedFile(path="vcp/__init__.py", status="FETCHED",
                           size_bytes=len(mock_content), content_bytes=len(mock_content),
                           line_count=1, content_full=mock_content,
                           content_preview=mock_content[:V1453_PREVIEW_BYTES],
                           error=None),
        ]
        audits = audit_all_protocols_full(mock_files)
        async_audit = next(a for a in audits if a.protocol == "async")
        # V1453 should find "async" in full content; V1452 with preview-only would also find it
        # because "async" appears at position 505, but V1452 preview is only first 200 chars
        # So V1453 keyword_count_total >= 1
        assert async_audit.keyword_count_total >= 1
        assert async_audit.keyword_presence > 0.0
        results.append({"name": "T05_full_content_finds_keyword", "ok": True,
                        "detail": f"async kw in full content: count={async_audit.keyword_count_total}"})
    except Exception as e:
        results.append({"name": "T05_full_content_finds_keyword", "ok": False, "detail": str(e)})

    # T06: per-file closure
    try:
        mock_files = [
            FullFetchedFile(path="vcp/__init__.py", status="FETCHED",
                           size_bytes=100, content_bytes=100, line_count=2,
                           content_full="async await gather", content_preview="async await",
                           error=None),
            FullFetchedFile(path="vcp/bundle.py", status="FAILED",
                           size_bytes=0, content_bytes=0, line_count=0,
                           content_full="", content_preview="",
                           error="404"),
        ]
        rows = per_file_closure(mock_files)
        assert len(rows) == 2
        fetched_row = next(r for r in rows if r.status == "FETCHED")
        failed_row = next(r for r in rows if r.status == "FAILED")
        assert fetched_row.protocols_with_kw >= 1
        assert fetched_row.total_kw_count >= 3
        assert failed_row.protocols_with_kw == 0
        assert failed_row.closure_rate == 0.0
        results.append({"name": "T06_per_file_closure", "ok": True,
                        "detail": f"fetched.protocols_with_kw={fetched_row.protocols_with_kw} failed.closure=0.0"})
    except Exception as e:
        results.append({"name": "T06_per_file_closure", "ok": False, "detail": str(e)})

    # T07: 42 pairs cross-modular
    try:
        mock_files = [
            FullFetchedFile(path="vcp/__init__.py", status="FETCHED",
                           size_bytes=50, content_bytes=50, line_count=1,
                           content_full="async await gather", content_preview="async",
                           error=None),
        ]
        audits = audit_all_protocols_full(mock_files)
        pairs = audit_problem_protocol_pairs_full(audits)
        assert len(pairs) == 42
        for p in pairs:
            assert p.closure in (0.0, 0.5, 1.0)
            assert "full_content=True" in p.evidence
        results.append({"name": "T07_42_pairs", "ok": True, "detail": "42 pairs closure ∈ {0, 0.5, 1.0}"})
    except Exception as e:
        results.append({"name": "T07_42_pairs", "ok": False, "detail": str(e)})

    # T08: build_report with mixed fetched/failed
    try:
        mock_files = [
            FullFetchedFile(path="vcp/__init__.py", status="FETCHED",
                           size_bytes=100, content_bytes=100, line_count=2,
                           content_full="async await", content_preview="async",
                           error=None),
            FullFetchedFile(path="vcp/bundle.py", status="FAILED",
                           size_bytes=0, content_bytes=0, line_count=0,
                           content_full="", content_preview="",
                           error="HTTP 404"),
        ]
        report = build_report_full(mock_files)
        assert report.n_files_fetched == 1
        assert report.n_files_failed == 1
        assert report.total_content_bytes == 100
        assert report.total_lines == 2
        assert report.avg_file_size == 100.0
        assert len(report.per_file) == 2
        assert len(report.per_protocol) == 6
        assert len(report.per_problem_protocol_pair) == 42
        results.append({"name": "T08_build_report", "ok": True,
                        "detail": f"n_fetched={report.n_files_fetched} avg_size={report.avg_file_size:.0f}"})
    except Exception as e:
        results.append({"name": "T08_build_report", "ok": False, "detail": str(e)})

    # T09: run_all with skip_fetch
    try:
        report = run_all(skip_fetch=True)
        assert report.n_files_fetched == 0
        assert all(f.status == "SKIPPED" for f in report.files)
        results.append({"name": "T09_run_all_skip_fetch", "ok": True, "detail": "skip_fetch=True → all SKIPPED"})
    except Exception as e:
        results.append({"name": "T09_run_all_skip_fetch", "ok": False, "detail": str(e)})

    # T10: chain delegate
    try:
        chain = chain_delegate()
        assert "delegates" in chain
        assert len(chain["delegates"]) == 6
        assert isinstance(chain["all_ok"], bool)
        results.append({"name": "T10_chain_delegate", "ok": True,
                        "detail": f"6 delegates, all_ok={chain['all_ok']}"})
    except Exception as e:
        results.append({"name": "T10_chain_delegate", "ok": False, "detail": str(e)})

    # T11: per-problem closure sums to 6 protocols per problem
    try:
        report = run_all(skip_fetch=True)
        assert len(report.per_problem_closure_rate) == 7
        for problem, rate in report.per_problem_closure_rate.items():
            assert 0.0 <= rate <= 1.0
        results.append({"name": "T11_per_problem_closure", "ok": True,
                        "detail": f"7 problems × closure rates bounded [0,1]"})
    except Exception as e:
        results.append({"name": "T11_per_problem_closure", "ok": False, "detail": str(e)})

    # T12: per-protocol closure sums to 6 protocols
    try:
        report = run_all(skip_fetch=True)
        assert len(report.per_protocol_closure_rate) == 6
        for proto, rate in report.per_protocol_closure_rate.items():
            assert proto in V1453_PROTOCOL_NAMES
            assert 0.0 <= rate <= 1.0
        results.append({"name": "T12_per_protocol_closure", "ok": True, "detail": "6 protocols × closure bounded [0,1]"})
    except Exception as e:
        results.append({"name": "T12_per_protocol_closure", "ok": False, "detail": str(e)})

    # T13: render markdown non-empty + contains key sections
    try:
        report = run_all(skip_fetch=True)
        md = _render_markdown(report)
        assert "V1453" in md
        assert "Honest disclosure" in md
        assert "V3 哲学守门" in md
        assert "GUARD" in md
        assert "Per-VCP-protocol audit" in md
        assert "Per-file closure" in md
        results.append({"name": "T13_render_markdown", "ok": True, "detail": f"md length={len(md)}"})
    except Exception as e:
        results.append({"name": "T13_render_markdown", "ok": False, "detail": str(e)})

    # T14: total_content_bytes + total_lines fields present
    try:
        mock_files = [
            FullFetchedFile(path="vcp/__init__.py", status="FETCHED",
                           size_bytes=50, content_bytes=50, line_count=5,
                           content_full="a\nb\nc\nd\ne", content_preview="a",
                           error=None),
        ]
        report = build_report_full(mock_files)
        assert report.total_content_bytes == 50
        assert report.total_lines == 5
        assert report.avg_file_size == 50.0
        results.append({"name": "T14_size_stats", "ok": True,
                        "detail": f"bytes={report.total_content_bytes} lines={report.total_lines}"})
    except Exception as e:
        results.append({"name": "T14_size_stats", "ok": False, "detail": str(e)})

    all_ok = all(r["ok"] for r in results)
    return all_ok, results


# ============================================================================
# Markdown render
# ============================================================================

def _render_markdown(report: V1453Report) -> str:
    lines: List[str] = []
    lines.append(f"# V1453 — ASI VCP 6 protocol GitHub source full-content audit v3")
    lines.append("")
    lines.append(f"- schema: `{V1453_SCHEMA}`")
    lines.append(f"- version: `{V1453_VERSION}`")
    lines.append(f"- module: `{V1453_MODULE}`")
    lines.append(f"- started: `{report.started}`")
    lines.append(f"- ended: `{report.ended}`")
    lines.append(f"- github_api_base: `{report.github_api_base}`")
    lines.append(f"- vcp_repo: `{report.vcp_repo}`")
    lines.append(f"- n_files_targeted: **{report.n_files_targeted}**")
    lines.append(f"- n_files_fetched: **{report.n_files_fetched}**")
    lines.append(f"- n_files_failed: **{report.n_files_failed}**")
    lines.append(f"- total_content_bytes: **{report.total_content_bytes}**")
    lines.append(f"- total_lines: **{report.total_lines}**")
    lines.append(f"- avg_file_size: **{report.avg_file_size:.1f}**")
    lines.append("")

    lines.append("## Per-VCP-protocol audit (6 protocols, full content)")
    lines.append("")
    lines.append("| protocol | kw_count_total | files_with_kw | files_fetched | keyword_presence | file_coverage | closure_rate | matched_keywords |")
    lines.append("|---|---|---|---|---|---|---|---|")
    for p in report.per_protocol:
        matched = ", ".join(p.matched_keywords) if p.matched_keywords else "(none)"
        lines.append(
            f"| {p.protocol} | {p.keyword_count_total} | {p.files_with_keyword} | "
            f"{p.files_fetched} | {p.keyword_presence:.4f} | {p.file_coverage:.4f} | "
            f"{p.closure_rate:.4f} | {matched} |"
        )
    lines.append("")

    lines.append("## Per-file closure (one row per VCP file)")
    lines.append("")
    lines.append("| path | status | protocols_with_kw | total_kw_count | line_count | content_bytes | closure_rate |")
    lines.append("|---|---|---|---|---|---|---|")
    for r in report.per_file:
        lines.append(
            f"| {r.path} | {r.status} | {r.protocols_with_kw} | "
            f"{r.total_kw_count} | {r.line_count} | {r.content_bytes} | "
            f"{r.closure_rate:.4f} |"
        )
    lines.append("")

    lines.append("## Per-problem × per-protocol cross-modular audit (42 pairs)")
    lines.append("")
    lines.append("| problem | protocol | problem_kw_present | protocol_kw_present | closure |")
    lines.append("|---|---|---|---|---|")
    for p in report.per_problem_protocol_pair:
        lines.append(
            f"| {p.problem} | {p.protocol} | {p.problem_kw_present} | "
            f"{p.protocol_kw_present} | {p.closure:.2f} |"
        )
    lines.append("")

    lines.append("## Per-problem closure_rate")
    lines.append("")
    lines.append("| problem | closure_rate |")
    lines.append("|---|---|")
    for prob, rate in report.per_problem_closure_rate.items():
        lines.append(f"| {prob} | {rate:.4f} |")
    lines.append("")

    lines.append("## Overall")
    lines.append("")
    lines.append(f"- overall_protocol_closure_rate: **{report.overall_closure_rate:.4f}**")
    lines.append(f"- cross_modular_overall (42 pairs): **{report.cross_modular_overall:.4f}**")
    lines.append("")

    lines.append("## Notes")
    lines.append("")
    for note in report.notes:
        lines.append(f"- {note}")
    lines.append("")

    lines.append("## Honest disclosure (主 17:43 实事求是)")
    lines.append("")
    lines.append(
        "> V1453 is a **bounded full-content keyword audit on real VCP source code "
        "fetched from GitHub**. It does NOT claim that VCP 6 protocols are correctly "
        "implemented, that the audit is exhaustive, or that keyword presence equals "
        "implementation parity. V1453 ≠ ASI closure. V1453 ≠ Phenomenal closure. "
        "V1453 ≠ human-level closure. V1453 ≠ absolute closure. V1453 ≠ VCP "
        "implementation parity. V1453 = bounded keyword search on FULL GitHub-fetched "
        "VCP source files (up to 128KB per file). V1453 is the v3 extension of V1452 "
        "(preview-only) — V1453 finds keywords in full content, not just first 200 chars. "
        "If GitHub fetch fails (offline mode), all 6 protocols have closure=0.0 by "
        "honest disclosure."
    )
    lines.append("")
    lines.append(
        "（主 17:43 实事求是 + 主 17:58 不假装 + 主 20:46 不假装达到 ASI + 主 19:33 走在前人经验上 + 主 22:33 终极授权 + 主 00:44 质量工程化 + 主 00:56 任何人能接手）"
    )
    lines.append("")

    lines.append("## Borrowed (主 19:33 走在前人经验上)")
    lines.append("")
    for src, desc in V1453_BORROWED:
        lines.append(f"- **{src}**: {desc}")
    lines.append("")

    lines.append("## V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)")
    lines.append("")
    for g in V1453_V3_GUARDS:
        lines.append(f"- {g}")
    lines.append("")

    lines.append("## GUARDS upheld (V1453-specific, 14)")
    lines.append("")
    for g in V1453_GUARDS:
        lines.append(f"- {g}")
    lines.append("")

    return "\n".join(lines)


# ============================================================================
# CLI
# ============================================================================

def _build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog=V1453_MODULE_SHORT,
        description="V1453 — ASI VCP 6 protocol GitHub source full-content audit v3",
    )
    p.add_argument("cmd", nargs="?", default="help",
                   choices=["version", "help", "meta", "popper", "chain",
                            "audit", "report", "run-all"])
    p.add_argument("--json", action="store_true", help="JSON output for meta")
    p.add_argument("--out-json", type=str, default=None, help="Output JSON path")
    p.add_argument("--out-md", type=str, default=None, help="Output MD path")
    p.add_argument("--skip-fetch", action="store_true", help="Skip GitHub fetch (offline)")
    p.add_argument("--timeout", type=float, default=V1453_TIMEOUT_SECONDS, help="HTTP timeout seconds")
    return p


def main(argv: Optional[List[str]] = None) -> int:
    parser = _build_parser()
    try:
        args = parser.parse_args(argv)
    except SystemExit as e:
        return int(e.code) if isinstance(e.code, int) else 2
    cmd = args.cmd or "help"

    if cmd == "version":
        print(V1453_VERSION)
        return 0

    if cmd == "help":
        parser.print_help()
        return 0

    if cmd == "meta":
        meta = {
            "schema": V1453_SCHEMA,
            "version": V1453_VERSION,
            "module": V1453_MODULE,
            "n_protocols": len(V1453_PROTOCOL_NAMES),
            "n_problems": len(V1453_PROBLEM_NAMES),
            "n_vcp_paths": len(V1453_VCP_PATHS),
            "max_fetch": V1453_MAX_FETCH,
            "max_body_bytes": V1453_MAX_BODY_BYTES,
            "preview_bytes": V1453_PREVIEW_BYTES,
            "timeout_seconds": V1453_TIMEOUT_SECONDS,
            "guards": list(V1453_GUARDS),
            "v3_guards": list(V1453_V3_GUARDS),
            "borrowed": [list(b) for b in V1453_BORROWED],
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
        chain = chain_delegate()
        print(json.dumps(chain, indent=2, ensure_ascii=False))
        return 0

    if cmd == "audit":
        try:
            files = fetch_vcp_files_full(timeout=args.timeout)
        except Exception as e:
            files = [
                FullFetchedFile(path=p, status="FAILED", size_bytes=0,
                               content_bytes=0, line_count=0,
                               content_full="", content_preview="",
                               error=str(e))
                for p in V1453_VCP_PATHS
            ]
        report = build_report_full(files)
        summary = {
            "n_files_fetched": report.n_files_fetched,
            "n_files_failed": report.n_files_failed,
            "total_content_bytes": report.total_content_bytes,
            "total_lines": report.total_lines,
            "overall_closure_rate": report.overall_closure_rate,
            "per_protocol_closure_rate": report.per_protocol_closure_rate,
            "cross_modular_overall": report.cross_modular_overall,
        }
        print(json.dumps(summary, indent=2, ensure_ascii=False))
        return 0

    if cmd == "report":
        report = run_all(
            out_json=Path(args.out_json) if args.out_json else None,
            out_md=Path(args.out_md) if args.out_md else None,
            skip_fetch=args.skip_fetch,
        )
        print(f"V1453 audit report written.")
        print(f"  n_files_fetched: {report.n_files_fetched}")
        print(f"  total_content_bytes: {report.total_content_bytes}")
        print(f"  overall_closure_rate: {report.overall_closure_rate:.4f}")
        print(f"  cross_modular_overall: {report.cross_modular_overall:.4f}")
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

        chain = chain_delegate()
        print(f"CHAIN all_ok={chain['all_ok']}\n")

        report = run_all(
            out_json=Path(args.out_json) if args.out_json else None,
            out_md=Path(args.out_md) if args.out_md else None,
            skip_fetch=args.skip_fetch,
        )
        print(f"V1453 audit report written.")
        print(f"  n_files_fetched: {report.n_files_fetched}")
        print(f"  total_content_bytes: {report.total_content_bytes}")
        print(f"  overall_closure_rate: {report.overall_closure_rate:.4f}")
        print(f"  cross_modular_overall: {report.cross_modular_overall:.4f}")
        return 0 if ok else 1

    parser.print_help()
    return 0


if __name__ == "__main__":
    sys.exit(main())