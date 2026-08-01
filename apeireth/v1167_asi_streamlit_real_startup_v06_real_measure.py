"""V1167 — ASI streamlit_real_startup V0.6 真补 (5 sub-dim 真测).

主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 19:33 走在前人经验上 + 主 13:31 大胆激进 +
主 17:58+20:46 不假装 + 主 23:44 干到底 + 主 00:56 任何人都能接手 + 主 00:44 质量工程化 +
主 06:15 V1050+ 真用 V1009 Streamlit.

主 06:15 06:32 真用方向: V1009 Streamlit web UI 真启动, 真访问.
主 17:43 实事求是真问题 (V1155 baseline):
  - V1134 真跑 streamlit run + 真 HTTP probe, 但 V1155 baseline 没分维度, 直接给了 0.95.
  - V1167 真补: 把 V1134 V1134StreamlitReport 真 12 字段拆成 5 sub-dim 真测.

V1167 真补路径 (主 17:43 实事求是):
  - 5 sub-dim 真测 (基于 V1134 V1134StreamlitReport 真字段):
    S1 streamlit_installed_real   — streamlit_installed + version 非空
    S2 app_path_real              — app_path 文件存在 + 非空
    S3 port_assigned_real         — port > 0 + 不是 0
    S4 started_ok_real            — started_ok + pid > 0 + startup_ms > 0
    S5 http_probe_real            — health_ok + (homepage_ok OR page_probe_ok)
  - aggregate = mean(sub_dim_scores) ∈ [0, 1]
  - 任何 sub-dim 失败 → sub-dim score 衰减 (主 17:43 不刷 KPI)

主 00:56 任何人都能接手:
  - measure_streamlit_real_startup_v06() → float (0..1) 主入口
  - measure_streamlit_real_startup_full() → StreamlitRealStartupReport dataclass + JSON dump
  - StreamlitRealStartupReport JSON 写 artifacts/v1167_streamlit_real_startup_v06.json

主 00:44 质量工程化:
  - StreamlitRealStartupReport (主 22:33 北极星):
      total, sub_dim_scores (dict 5 keys), sub_dim_evidence (dict 5 keys)
      version, timestamp, snapshot_id (uuid), elapsed_seconds
      v1134_report_id, v1134_started_ok, v1134_health_ok, v1134_port

主 17:58 + 20:46 不假装:
  - 不假装 started_ok = 真服务可用: subprocess up ≠ HTTP serve
  - 不假装 streamlit --version = 服务可起: CLI 可用 ≠ serve 可用
  - 不假装 homepage_ok = 真可访问: 1 次 200 ≠ 多次可达
  - 不假装 ASI has web UI: 5 sub-dim 测量 ≠ ASI 自部署 web

Usage:
    python -m apeireth.v1167_asi_streamlit_real_startup_v06_real_measure              # 默认 measure + JSON dump
    python -m apeireth.v1167_asi_streamlit_real_startup_v06_real_measure --json      # JSON stdout
    python -m apeireth.v1167_asi_streamlit_real_startup_v06_real_measure --no-write  # 只 print
    python -m apeireth.v1167_asi_streamlit_real_startup_v06_real_measure --report    # markdown 报告
"""

from __future__ import annotations

import argparse
import json
import os
import statistics
import sys
import time
import uuid
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple


V1167_VERSION = "0.1.0"
V1167_DIM_VERSION = "0.6"

# 5 sub-dim names (LOCKED 主 19:33 走在前人经验上 — 借鉴 Streamlit/Heroku/12-Factor 5 axis)
V1167_SUBDIM_NAMES: Tuple[str, ...] = (
    "streamlit_installed_real",     # S1 — Streamlit CLI 真装
    "app_path_real",                # S2 — app_path 文件真存
    "port_assigned_real",           # S3 — port 真分配
    "started_ok_real",              # S4 — subprocess 真起
    "http_probe_real",              # S5 — HTTP probe 真过
)

# 默认 artifact dir (主 00:56 任何人都能接手)
DEFAULT_ARTIFACT_DIR = "artifacts"

# V1134 baseline (主 17:43 实事求是 — V1155 hot-patched 0.95 没分维度)
V1134_BASELINE_STREAM_STARTUP = 0.9500

# Target (主 13:31 大胆激进)
TARGET_STREAM_STARTUP_V06 = 0.8500

# V1134 真字段 (主 17:43 实事求是)
V1134_REPORT_FIELDS: Tuple[str, ...] = (
    "report_id",
    "timestamp",
    "streamlit_installed",
    "streamlit_version",
    "app_path",
    "port",
    "started_ok",
    "startup_ms",
    "health_ok",
    "homepage_ok",
    "page_probe_ok",
    "pid",
    "pages_rendered",
)


# 阈值 (主 17:43 实事求是 — 写在常量里, 不在 measurement 里魔改)
_STARTUP_MS_MIN = 100.0
_STARTUP_MS_MAX = 30000.0


# ============================================================================
# SubDimEvidence + StreamlitRealStartupReport — 真测结果 dataclass (主 00:44 质量工程化)
# ============================================================================


@dataclass
class SubDimEvidence:
    name: str
    score: float
    checks: Dict[str, bool] = field(default_factory=dict)
    notes: List[str] = field(default_factory=list)
    raw: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class StreamlitRealStartupReport:
    """V1167 streamlit_real_startup V0.6 真测报告."""

    snapshot_id: str = field(default_factory=lambda: f"v1167-{uuid.uuid4().hex[:8]}")
    version: str = V1167_VERSION
    dim_version: str = V1167_DIM_VERSION
    timestamp: float = field(default_factory=time.time)
    elapsed_seconds: float = 0.0
    total: float = 0.0
    sub_dim_scores: Dict[str, float] = field(default_factory=dict)
    sub_dim_evidence: Dict[str, SubDimEvidence] = field(default_factory=dict)
    n_subdims_total: int = len(V1167_SUBDIM_NAMES)
    n_subdims_passed: int = 0
    n_subdims_partial: int = 0
    n_subdims_missing: int = 0
    notes: List[str] = field(default_factory=list)
    artifact_path: str = ""
    v1134_baseline: float = V1134_BASELINE_STREAM_STARTUP
    target: float = TARGET_STREAM_STARTUP_V06
    v1134_report_id: str = ""
    v1134_started_ok: bool = False
    v1134_health_ok: bool = False
    v1134_homepage_ok: bool = False
    v1134_page_probe_ok: bool = False
    v1134_port: int = 0
    v1134_pid: int = 0
    v1134_streamlit_version: str = ""
    v1134_pages_count: int = 0

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["sub_dim_evidence"] = {k: v.to_dict() for k, v in self.sub_dim_evidence.items()}
        return d

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "StreamlitRealStartupReport":
        new = cls(
            snapshot_id=data.get("snapshot_id", ""),
            version=data.get("version", V1167_VERSION),
            dim_version=data.get("dim_version", V1167_DIM_VERSION),
            timestamp=data.get("timestamp", 0.0),
            elapsed_seconds=data.get("elapsed_seconds", 0.0),
            total=data.get("total", 0.0),
            sub_dim_scores=data.get("sub_dim_scores", {}),
            n_subdims_total=data.get("n_subdims_total", len(V1167_SUBDIM_NAMES)),
            n_subdims_passed=data.get("n_subdims_passed", 0),
            n_subdims_partial=data.get("n_subdims_partial", 0),
            n_subdims_missing=data.get("n_subdims_missing", 0),
            notes=data.get("notes", []),
            artifact_path=data.get("artifact_path", ""),
            v1134_baseline=data.get("v1134_baseline", V1134_BASELINE_STREAM_STARTUP),
            target=data.get("target", TARGET_STREAM_STARTUP_V06),
            v1134_report_id=data.get("v1134_report_id", ""),
            v1134_started_ok=data.get("v1134_started_ok", False),
            v1134_health_ok=data.get("v1134_health_ok", False),
            v1134_homepage_ok=data.get("v1134_homepage_ok", False),
            v1134_page_probe_ok=data.get("v1134_page_probe_ok", False),
            v1134_port=data.get("v1134_port", 0),
            v1134_pid=data.get("v1134_pid", 0),
            v1134_streamlit_version=data.get("v1134_streamlit_version", ""),
            v1134_pages_count=data.get("v1134_pages_count", 0),
        )
        raw_evidence = data.get("sub_dim_evidence", {})
        for k, v in raw_evidence.items():
            new.sub_dim_evidence[k] = SubDimEvidence(
                name=v.get("name", k),
                score=v.get("score", 0.0),
                checks=v.get("checks", {}),
                notes=v.get("notes", []),
                raw=v.get("raw", {}),
            )
        return new

    def summary_line(self) -> str:
        return (
            f"V1167 streamlit_real_startup V0.6: total={self.total:.4f} "
            f"(Δ vs V1134 baseline {self.v1134_baseline:.4f} = "
            f"{self.total - self.v1134_baseline:+.4f}) | "
            f"target={self.target:.4f} (gap {self.target - self.total:+.4f}) | "
            f"5 sub-dim: {self.n_subdims_passed} pass / "
            f"{self.n_subdims_partial} partial / {self.n_subdims_missing} missing | "
            f"started={self.v1134_started_ok} health={self.v1134_health_ok} | "
            f"snapshot={self.snapshot_id}"
        )


# ============================================================================
# safe helpers
# ============================================================================


def _safe_import(name: str) -> Optional[Any]:
    try:
        import importlib
        return importlib.import_module(name)
    except Exception:
        return None


def _attr_first(mod: Any, names: List[str]) -> Optional[Any]:
    for n in names:
        a = getattr(mod, n, None)
        if a is not None:
            return a
    return None


def _call_safely(fn: Optional[Callable], *args: Any, default: Any = None, **kwargs: Any) -> Tuple[bool, Any]:
    if fn is None or not callable(fn):
        return False, default
    try:
        return True, fn(*args, **kwargs)
    except Exception:
        return False, default


def _safe_field(report: Any, name: str, default: Any = 0) -> Any:
    """Safely read a field from V1134StreamlitReport (主 17:43 实事求是)."""
    try:
        v = getattr(report, name, None)
        if v is None:
            return default
        return v
    except Exception:
        return default


# ============================================================================
# Main runner — get V1134StreamlitReport or build mock data
# ============================================================================


def _get_v1134_report(
    app_dir: Optional[str] = None,
    preferred_port: int = 8765,
    startup_timeout_s: float = 25.0,
) -> Tuple[bool, Any, str]:
    """Try to get a real V1134 report. Returns (ok, report_or_None, reason)."""
    v1134_mod = _safe_import("apeireth.v1134_streamlit_real_startup")
    if v1134_mod is None:
        return False, None, "v1134_module_not_found"

    fn = _attr_first(v1134_mod, [
        "run_real_streamlit", "start_streamlit", "run_streamlit", "main"
    ])
    if fn is None:
        return False, None, "v1134_run_function_not_found"

    try:
        rep = fn(app_dir=app_dir, preferred_port=preferred_port, startup_timeout_s=startup_timeout_s)
    except TypeError:
        try:
            rep = fn()
        except Exception as e:
            return False, None, f"v1134_run_failed: {e!r}"
    except Exception as e:
        return False, None, f"v1134_run_failed: {e!r}"

    if rep is None:
        return False, None, "v1134_run_returned_none"
    return True, rep, "ok"


# ============================================================================
# S1 — streamlit_installed_real
# ============================================================================


def _measure_streamlit_installed(report: Any) -> Tuple[float, SubDimEvidence]:
    """S1: streamlit CLI 真装."""
    ev = SubDimEvidence(
        name="streamlit_installed_real",
        score=0.0,
        notes=["S1: V1134 streamlit_installed + version 真测"]
    )

    installed = bool(_safe_field(report, "streamlit_installed", False))
    version = str(_safe_field(report, "streamlit_version", ""))

    test_results: List[Tuple[str, bool, str]] = []
    test_results.append(("installed", installed, f"installed={installed}"))
    test_results.append(("version_nonempty", bool(version), f"version={version[:30]}"))
    test_results.append(("version_has_digit", any(c.isdigit() for c in version),
                        f"version={version[:30]}"))
    test_results.append(("version_no_error_marker",
                        "error" not in version.lower() and "fail" not in version.lower(),
                        f"version={version[:30]}"))
    test_results.append(("cli_resolvable",
                        installed,  # proxy
                        f"installed={installed} (CLI 可解析)"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)

    if installed and version and any(c.isdigit() for c in version):
        base = 0.95
    elif installed:
        base = 0.7
    else:
        base = 0.0

    check_bonus = float(n_pass) / 5.0 * 0.1
    ev.score = min(1.0, max(0.0, base + check_bonus))
    ev.checks = {n: ok for n, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "installed": installed,
        "version": version[:80],
    }
    ev.notes.append(f"S1 score={ev.score:.4f} (n_pass={n_pass}/5, installed={installed})")
    return ev.score, ev


# ============================================================================
# S2 — app_path_real
# ============================================================================


def _measure_app_path(report: Any) -> Tuple[float, SubDimEvidence]:
    """S2: app_path 文件真存."""
    ev = SubDimEvidence(
        name="app_path_real",
        score=0.0,
        notes=["S2: V1134 app_path 真存文件"]
    )

    app_path = str(_safe_field(report, "app_path", ""))

    file_exists = bool(app_path) and os.path.isfile(app_path)
    file_size = os.path.getsize(app_path) if file_exists else 0
    file_nonempty = file_size > 0

    test_results: List[Tuple[str, bool, str]] = []
    test_results.append(("path_nonempty", bool(app_path), f"app_path={app_path[:60]}"))
    test_results.append(("file_exists", file_exists, f"app_path={app_path[:60]}, exists={file_exists}"))
    test_results.append(("file_nonempty", file_nonempty, f"size={file_size}"))
    test_results.append(("file_size_reasonable", 200 <= file_size <= 100000,
                        f"size={file_size} ∈ [200, 100000]"))
    test_results.append(("file_is_python", app_path.endswith(".py") if app_path else False,
                        f"app_path={app_path[:60]}"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)

    if file_exists and file_nonempty:
        base = 0.9
        # bonus if size in reasonable range
        if 200 <= file_size <= 100000:
            base = min(1.0, base + 0.05)
    elif file_exists:
        base = 0.5
    elif app_path:
        base = 0.2  # path declared but file missing
    else:
        base = 0.0

    check_bonus = float(n_pass) / 5.0 * 0.1
    ev.score = min(1.0, max(0.0, base + check_bonus))
    ev.checks = {n: ok for n, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "app_path": app_path[:120],
        "file_exists": file_exists,
        "file_size": file_size,
    }
    ev.notes.append(f"S2 score={ev.score:.4f} (n_pass={n_pass}/5, exists={file_exists}, size={file_size})")
    return ev.score, ev


# ============================================================================
# S3 — port_assigned_real
# ============================================================================


def _measure_port_assigned(report: Any) -> Tuple[float, SubDimEvidence]:
    """S3: port 真分配."""
    ev = SubDimEvidence(
        name="port_assigned_real",
        score=0.0,
        notes=["S3: V1134 port 真分配"]
    )

    port = int(_safe_field(report, "port", 0))

    test_results: List[Tuple[str, bool, str]] = []
    test_results.append(("port_positive", port > 0, f"port={port}"))
    test_results.append(("port_in_valid_range", 1 <= port <= 65535,
                        f"port={port} ∈ [1, 65535]"))
    test_results.append(("port_not_zero", port != 0, f"port={port} != 0"))
    test_results.append(("port_unprivileged", 1024 <= port <= 65535,
                        f"port={port} ∈ [1024, 65535] (unprivileged)"))
    test_results.append(("port_preferred_or_alt",
                        port in (8765, 8766, 8767, 0) or (1024 <= port <= 65535),
                        f"port={port}"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)

    if port > 0:
        base = 0.95
    elif port == 0:
        base = 0.0  # no free port
    else:
        base = 0.0

    check_bonus = float(n_pass) / 5.0 * 0.05
    ev.score = min(1.0, max(0.0, base + check_bonus))
    ev.checks = {n: ok for n, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "port": port,
    }
    ev.notes.append(f"S3 score={ev.score:.4f} (n_pass={n_pass}/5, port={port})")
    return ev.score, ev


# ============================================================================
# S4 — started_ok_real
# ============================================================================


def _measure_started_ok(report: Any) -> Tuple[float, SubDimEvidence]:
    """S4: subprocess 真起."""
    ev = SubDimEvidence(
        name="started_ok_real",
        score=0.0,
        notes=["S4: V1134 started_ok + pid + startup_ms 真测"]
    )

    started = bool(_safe_field(report, "started_ok", False))
    pid = int(_safe_field(report, "pid", 0))
    startup_ms = float(_safe_field(report, "startup_ms", 0.0))

    test_results: List[Tuple[str, bool, str]] = []
    test_results.append(("started_ok", started, f"started_ok={started}"))
    test_results.append(("pid_positive", pid > 0, f"pid={pid}"))
    test_results.append(("startup_ms_positive", startup_ms > 0.0,
                        f"startup_ms={startup_ms}"))
    test_results.append(("startup_ms_in_range",
                        _STARTUP_MS_MIN <= startup_ms <= _STARTUP_MS_MAX,
                        f"startup_ms={startup_ms} ∈ [{_STARTUP_MS_MIN}, {_STARTUP_MS_MAX}]"))
    test_results.append(("started_and_pid",
                        started and pid > 0,
                        f"started={started} && pid={pid}"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)

    if started and pid > 0:
        if _STARTUP_MS_MIN <= startup_ms <= _STARTUP_MS_MAX:
            base = 0.95
        else:
            base = 0.7  # 起了但 startup_ms 异常
    elif started:
        base = 0.6  # started=True 但 pid 缺失 (subprocess 已退?)
    else:
        base = 0.0

    check_bonus = float(n_pass) / 5.0 * 0.1
    ev.score = min(1.0, max(0.0, base + check_bonus))
    ev.checks = {n: ok for n, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "started_ok": started,
        "pid": pid,
        "startup_ms": startup_ms,
    }
    ev.notes.append(f"S4 score={ev.score:.4f} (n_pass={n_pass}/5, started={started}, pid={pid}, startup_ms={startup_ms})")
    return ev.score, ev


# ============================================================================
# S5 — http_probe_real
# ============================================================================


def _measure_http_probe(report: Any) -> Tuple[float, SubDimEvidence]:
    """S5: HTTP probe 真过."""
    ev = SubDimEvidence(
        name="http_probe_real",
        score=0.0,
        notes=["S5: V1134 health_ok + homepage_ok + page_probe_ok 真测"]
    )

    health = bool(_safe_field(report, "health_ok", False))
    homepage = bool(_safe_field(report, "homepage_ok", False))
    page_probe = bool(_safe_field(report, "page_probe_ok", False))

    n_probes_passed = sum([health, homepage, page_probe])

    test_results: List[Tuple[str, bool, str]] = []
    test_results.append(("health_ok", health, f"health_ok={health}"))
    test_results.append(("homepage_ok", homepage, f"homepage_ok={homepage}"))
    test_results.append(("page_probe_ok", page_probe, f"page_probe_ok={page_probe}"))
    test_results.append(("at_least_one_probe",
                        n_probes_passed >= 1,
                        f"probes_passed={n_probes_passed}"))
    test_results.append(("health_required", health,
                        f"health_ok={health} (required)"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)

    if health and homepage and page_probe:
        base = 1.0
    elif health and (homepage or page_probe):
        base = 0.85
    elif health:
        base = 0.6  # /_stcore/health 200 but no page probe
    elif homepage or page_probe:
        base = 0.3  # 页面可达但 health 未明
    else:
        base = 0.0

    check_bonus = float(n_pass) / 5.0 * 0.1
    ev.score = min(1.0, max(0.0, base + check_bonus))
    ev.checks = {n: ok for n, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "health_ok": health,
        "homepage_ok": homepage,
        "page_probe_ok": page_probe,
        "n_probes_passed": n_probes_passed,
    }
    ev.notes.append(f"S5 score={ev.score:.4f} (n_pass={n_pass}/5, health={health}, home={homepage}, page={page_probe})")
    return ev.score, ev


# ============================================================================
# 主入口
# ============================================================================


def measure_streamlit_real_startup_v06(
    preferred_port: int = 8765,
    startup_timeout_s: float = 25.0,
) -> float:
    """主入口 — 返回 streamlit_real_startup V0.6 score (0..1)."""
    rep = measure_streamlit_real_startup_full(
        write_artifact=False,
        preferred_port=preferred_port,
        startup_timeout_s=startup_timeout_s,
    )
    return rep.total


def measure_streamlit_real_startup_full(
    write_artifact: bool = True,
    artifact_dir: str = DEFAULT_ARTIFACT_DIR,
    preferred_port: int = 8765,
    startup_timeout_s: float = 25.0,
) -> StreamlitRealStartupReport:
    """Run all 5 sub-dims, return StreamlitRealStartupReport."""
    t0 = time.time()
    rep = StreamlitRealStartupReport()

    ok, v1134_rep, reason = _get_v1134_report(
        preferred_port=preferred_port,
        startup_timeout_s=startup_timeout_s,
    )
    if not ok or v1134_rep is None:
        rep.notes.append(f"V1134 unavailable: {reason} → all sub-dim = 0")
        rep.elapsed_seconds = time.time() - t0
        if write_artifact:
            try:
                ad = Path(artifact_dir)
                ad.mkdir(parents=True, exist_ok=True)
                artifact_path = ad / "v1167_streamlit_real_startup_v06.json"
                artifact_path.write_text(
                    json.dumps(rep.to_dict(), indent=2, ensure_ascii=False),
                    encoding="utf-8",
                )
                rep.artifact_path = str(artifact_path)
            except Exception as e:
                rep.notes.append(f"artifact write failed: {e!r}")
        return rep

    # Populate V1134 cross-refs
    rep.v1134_report_id = str(_safe_field(v1134_rep, "report_id", "unknown"))
    rep.v1134_started_ok = bool(_safe_field(v1134_rep, "started_ok", False))
    rep.v1134_health_ok = bool(_safe_field(v1134_rep, "health_ok", False))
    rep.v1134_homepage_ok = bool(_safe_field(v1134_rep, "homepage_ok", False))
    rep.v1134_page_probe_ok = bool(_safe_field(v1134_rep, "page_probe_ok", False))
    rep.v1134_port = int(_safe_field(v1134_rep, "port", 0))
    rep.v1134_pid = int(_safe_field(v1134_rep, "pid", 0))
    rep.v1134_streamlit_version = str(_safe_field(v1134_rep, "streamlit_version", ""))
    pages = _safe_field(v1134_rep, "pages_rendered", [])
    rep.v1134_pages_count = len(pages) if hasattr(pages, "__len__") else 0

    # S1
    s1, ev1 = _measure_streamlit_installed(v1134_rep)
    rep.sub_dim_scores["streamlit_installed_real"] = s1
    rep.sub_dim_evidence["streamlit_installed_real"] = ev1
    if s1 >= 0.8:
        rep.n_subdims_passed += 1
    elif s1 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # S2
    s2, ev2 = _measure_app_path(v1134_rep)
    rep.sub_dim_scores["app_path_real"] = s2
    rep.sub_dim_evidence["app_path_real"] = ev2
    if s2 >= 0.8:
        rep.n_subdims_passed += 1
    elif s2 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # S3
    s3, ev3 = _measure_port_assigned(v1134_rep)
    rep.sub_dim_scores["port_assigned_real"] = s3
    rep.sub_dim_evidence["port_assigned_real"] = ev3
    if s3 >= 0.8:
        rep.n_subdims_passed += 1
    elif s3 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # S4
    s4, ev4 = _measure_started_ok(v1134_rep)
    rep.sub_dim_scores["started_ok_real"] = s4
    rep.sub_dim_evidence["started_ok_real"] = ev4
    if s4 >= 0.8:
        rep.n_subdims_passed += 1
    elif s4 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # S5
    s5, ev5 = _measure_http_probe(v1134_rep)
    rep.sub_dim_scores["http_probe_real"] = s5
    rep.sub_dim_evidence["http_probe_real"] = ev5
    if s5 >= 0.8:
        rep.n_subdims_passed += 1
    elif s5 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    rep.total = sum(rep.sub_dim_scores.values()) / float(len(V1167_SUBDIM_NAMES))
    rep.total = min(1.0, max(0.0, rep.total))
    rep.elapsed_seconds = time.time() - t0

    if write_artifact:
        try:
            ad = Path(artifact_dir)
            ad.mkdir(parents=True, exist_ok=True)
            artifact_path = ad / "v1167_streamlit_real_startup_v06.json"
            artifact_path.write_text(
                json.dumps(rep.to_dict(), indent=2, ensure_ascii=False),
                encoding="utf-8",
            )
            rep.artifact_path = str(artifact_path)
            rep.notes.append(f"artifact written: {rep.artifact_path}")
        except Exception as e:
            rep.notes.append(f"artifact write failed: {e!r}")

    return rep


# ============================================================================
# 报告渲染 (主 00:44 质量工程化)
# ============================================================================


def render_report_md(rep: StreamlitRealStartupReport) -> str:
    lines: List[str] = []
    lines.append(f"# V1167 streamlit_real_startup V0.6 真补报告 — {rep.snapshot_id}\n")
    lines.append(f"- **version**: {rep.version}")
    lines.append(f"- **dim_version**: {rep.dim_version}")
    lines.append(f"- **timestamp**: {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(rep.timestamp))}")
    lines.append(f"- **elapsed**: {rep.elapsed_seconds:.3f}s")
    lines.append(f"- **artifact**: `{rep.artifact_path or 'N/A'}`")
    lines.append(f"- **v1134 report_id**: `{rep.v1134_report_id}`")
    lines.append(f"- **v1134 started_ok**: {rep.v1134_started_ok}")
    lines.append(f"- **v1134 health_ok**: {rep.v1134_health_ok}")
    lines.append(f"- **v1134 homepage_ok**: {rep.v1134_homepage_ok}")
    lines.append(f"- **v1134 page_probe_ok**: {rep.v1134_page_probe_ok}")
    lines.append(f"- **v1134 port**: {rep.v1134_port}")
    lines.append(f"- **v1134 pid**: {rep.v1134_pid}")
    lines.append(f"- **v1134 streamlit_version**: `{rep.v1134_streamlit_version}`")
    lines.append(f"- **v1134 pages_count**: {rep.v1134_pages_count}\n")

    lines.append("## Total")
    lines.append(f"- **streamlit_real_startup V0.6**: {rep.total:.4f}")
    lines.append(f"- **vs V1134 baseline**: {rep.v1134_baseline:.4f} (Δ = {rep.total - rep.v1134_baseline:+.4f})")
    lines.append(f"- **target**: {rep.target:.4f} (gap = {rep.target - rep.total:+.4f})\n")

    lines.append("## 5 sub-dim 真测\n")
    lines.append("| sub-dim | score | status |")
    lines.append("|---|---:|:---:|")
    for name in V1167_SUBDIM_NAMES:
        s = rep.sub_dim_scores.get(name, 0.0)
        status = "✅ pass" if s >= 0.8 else ("⚠️ partial" if s > 0.0 else "❌ missing")
        lines.append(f"| {name} | {s:.4f} | {status} |")

    lines.append("\n## Sub-dim Evidence\n")
    for name in V1167_SUBDIM_NAMES:
        ev = rep.sub_dim_evidence.get(name)
        if ev is None:
            continue
        lines.append(f"### {name} (score = {ev.score:.4f})")
        if ev.notes:
            for n in ev.notes:
                lines.append(f"- note: {n}")
        if ev.checks:
            for cn, cv in ev.checks.items():
                lines.append(f"- `{cn}`: {'✅' if cv else '❌'}")
        lines.append("")

    lines.append("## Notes\n")
    for n in rep.notes:
        lines.append(f"- {n}")
    lines.append("")
    lines.append("---")
    lines.append(f"_Generated by V1167 {rep.version}_")
    return "\n".join(lines)


# ============================================================================
# CLI
# ============================================================================


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="V1167 streamlit_real_startup V0.6 真补")
    parser.add_argument("--json", action="store_true", help="输出 JSON stdout")
    parser.add_argument("--no-write", action="store_true", help="不写 artifact")
    parser.add_argument("--report", action="store_true", help="输出 Markdown 报告")
    parser.add_argument("--artifact-dir", default=DEFAULT_ARTIFACT_DIR)
    parser.add_argument("--md-out", default=None)
    parser.add_argument("--port", type=int, default=8766, help="preferred_port")
    parser.add_argument("--startup-timeout", type=float, default=15.0, help="startup_timeout_s")
    args = parser.parse_args(argv)

    rep = measure_streamlit_real_startup_full(
        write_artifact=not args.no_write,
        artifact_dir=args.artifact_dir,
        preferred_port=args.port,
        startup_timeout_s=args.startup_timeout,
    )

    if args.json:
        print(json.dumps(rep.to_dict(), indent=2, ensure_ascii=False))
    elif args.report:
        md = render_report_md(rep)
        if args.md_out:
            Path(args.md_out).parent.mkdir(parents=True, exist_ok=True)
            Path(args.md_out).write_text(md, encoding="utf-8")
            print(f"report written: {args.md_out}")
        else:
            sys.stdout.write(md)
    else:
        print(rep.summary_line())

    return 0


if __name__ == "__main__":
    sys.exit(main())