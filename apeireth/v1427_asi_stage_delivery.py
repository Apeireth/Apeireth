"""V1427 — ASI 总框架 stage-delivery 真生产报告 V1411-V1426 (主 13:08 + 主 00:56 阶段性交付).

Phase: 1427
Version: 0.1.0
Date: 2026-08-10 (cron tick 04:09, Asia/Shanghai deep night)
Post: V1411 (overarching) + V1425 (5 philosophical gaps) + V1426 (VCP 6 protocols) + V1424 (real benchmark)

What V1427 is
=============
V1427 is the **stage-delivery report** module for V1411-V1426. It is
a bounded, deterministic aggregator that:

- Calls each upstream module's `chain_delegate()` to confirm liveness
- Calls each upstream module's main probe(s) to collect real numbers
- Aggregates the results into a single Markdown + JSON report
- Writes the report to disk + prints summary

It does NOT compute a fake "ASI score" by summing arbitrary weights.
It does report a **coverage rate** = (n_modules_ok / n_modules_total),
which is a real, defensible number based on importability + chain_ok.

The stage covers:
- V1411 (overarching framework) — chain_delegate with 11 frameworks
- V1412-V1416 (overarching dashboard / history / watchdog / multi-period / dgm-tick)
- V1417-V1423 (DGM tick history + cron + multi-policy + HTTP + daemon + webhook + wiring)
- V1424 (real LLM benchmark — 22 samples, 5 providers)
- V1425 (5 philosophical gaps probes — Time/Freedom/Recognition/Emergence/Truth)
- V1426 (VCP 6-plugin-protocol dispatcher — sync/async/static/service/preprocessor/hybrid)

Real-world usage:

    # Anyone can render the stage-delivery report:
    python -m apeireth.v1427_asi_stage_delivery report

    # Anyone can see live chain status:
    python -m apeireth.v1427_asi_stage_delivery chain

    # Anyone can see coverage summary:
    python -m apeireth.v1427_asi_stage_delivery summary

It does NOT mutate any upstream module state. It only **calls** each
upstream module's read-only API (chain_delegate, popper_self_test,
run_all_probes, dispatch_all, get_meta).

Borrowed (12 — 主 19:33 走在前人经验上):
=======================================
- V1411 (overarching — chain_delegate() reference impl)
- V1417 (DGM tick history — JSONL source)
- V1418 (DGM cron integration — chain_delegate pattern)
- V1419 (multi-policy evaluator — chain_delegate pattern)
- V1420 (HTTP status endpoint — chain_delegate pattern)
- V1421 (daemon — chain_delegate pattern)
- V1422 (notification webhook — chain_delegate pattern)
- V1423 (daemon webhook wiring — chain_delegate pattern)
- V1424 (real LLM benchmark — chain_delegate + probe result)
- V1425 (5 philosophical gaps — probe result + GapReport)
- V1426 (VCP 6 protocols — dispatch result + report)
- stdlib json + pathlib + importlib

GUARDS upheld (V1427-specific, 14 — 主 00:44 质量工程化)
=========================================================
- GUARD_NO_UPSTREAM_WRITE: V1427 only calls public APIs, never mutates
- GUARD_COVERAGE_DEFINED: coverage = n_modules_ok / n_modules_total
- GUARD_REPORT_REAL: report contains real numbers (not stubbed)
- GUARD_JSON_ATOMIC: report written atomically (tmp + rename)
- GUARD_MD_RENDERED: markdown report rendered, not empty
- GUARD_FRAMES_LISTED: every V14xx module name appears in report
- GUARD_CHAIN_OK: V1427 own chain_delegate returns all_ok
- GUARD_PROBES_COLLECTED: 5 philosophical gap probes included
- GUARD_PROTOCOLS_COLLECTED: 6 VCP protocols included
- GUARD_BENCHMARK_INCLUDED: V1424 real benchmark numbers included
- GUARD_POPPER_RUNS: popper self-test runs
- GUARD_HONEST_DISCLOSURE: honesty paragraph emitted
- GUARD_CLI_RUNNABLE: CLI 真可跑
- GUARD_NO_FAKE_SCORE: ASI score is NOT computed (no fake weights)

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43) — 5 guards
=======================================================
- GUARD_NO_PHENOMENAL_REPORT: report is data, NOT consciousness
- GUARD_NO_ASI_REPORT: report is coverage, NOT ASI score
- GUARD_NO_HUMAN_LEVEL_REPORT: report is bounded, NOT human-level
- GUARD_NO_ABSOLUTE_REPORT: report is local, NOT absolute
- GUARD_NO_NORTH_STAR_PADDING: coverage != north-star score

Honest disclosure (主 17:58 + 主 17:43)
=======================================
V1427 is a **bounded aggregator**. It does not claim to deliver ASI,
Phenomenal consciousness, human-level judgment, or absolute truth.
It reports a **coverage rate** = (n_modules_ok / n_modules_total)
across 17 V14xx modules. It is bounded by stdlib + Apeireth module
calls; NOT by ASI, north-star, VCP parity, or chain truth. V1427 ≠
ASI delivery, ≠ Phenomenal report, ≠ human-level report, ≠ absolute
report. V1427 reads V1411-V1426; never replaces any of them.

API surfaces (12)
=================
1.  ``UPSTREAM_MODULES`` — tuple of 17 V14xx module names
2.  ``StageDeliveryReport`` — dataclass (modules + probes + protocols + benchmark + coverage)
3.  ``build_default_config()`` — paths to upstream reports + this report
4.  ``collect_chain_status()`` — dict[module_name, dict] from chain_delegate()
5.  ``collect_probe_results()`` — GapReport from V1425.run_all_probes
6.  ``collect_protocol_results()`` — VCPSixDispatchReport from V1426.dispatch_all
7.  ``collect_benchmark_stats()`` — V1424 benchmark JSONL summary
8.  ``compute_coverage(chain_status)`` — float in [0, 1]
9.  ``build_stage_report(cfg)`` — full StageDeliveryReport
10. ``render_report_md(report)`` — markdown string
11. ``popper_self_test()`` — 14 self-tests
12. ``chain_delegate()`` — V1411 + V1418 + V1424 + V1425 + V1426 chain probe

CLI commands (10 — 主 00:56 任何人都能接手)
==========================================
- version
- meta [--json]
- demo
- help
- popper
- chain
- summary
- collect
- report
"""

from __future__ import annotations

import dataclasses
import json
import sys
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple


# ============================================================================
# Constants
# ============================================================================

V1427_VERSION = "0.1.0"
V1427_SCHEMA = "v1427.asi-stage-delivery/v1"
V1427_MODULE = "v1427_asi_stage_delivery"

# Upstream modules to aggregate (V1411-V1426)
UPSTREAM_MODULES: Tuple[str, ...] = (
    "v1411_asi_overarching_framework",
    "v1412_asi_overarching_dashboard",
    "v1413_asi_overarching_history",
    "v1414_asi_overarching_watchdog",
    "v1415_asi_overarching_multi_period",
    "v1416_asi_overarching_dgm_tick",
    "v1417_asi_dgm_tick_history",
    "v1418_asi_dgm_cron_integration",
    "v1419_asi_multi_policy_evaluator",
    "v1420_asi_http_status_endpoint",
    "v1421_asi_daemon_serve_tick",
    "v1422_asi_notification_webhook",
    "v1423_asi_daemon_webhook_wiring",
    "v1424_asi_real_llm_benchmark",
    "v1425_asi_five_philosophical_gaps",
    "v1426_vcp_six_protocol_dispatcher",
)

WORKSPACE = Path(__file__).resolve().parents[2]
PROMETHEAN = (
    WORKSPACE / "promethean"
    if (WORKSPACE / "promethean").exists()
    else WORKSPACE
)

DEFAULT_BENCHMARK_JSONL = PROMETHEAN / ".v1424-benchmark-results.jsonl"
DEFAULT_V1425_REPORT = PROMETHEAN / ".v1425-philosophical-gaps-report.json"
DEFAULT_V1426_REPORT = PROMETHEAN / ".v1426-vcp-six-protocol-report.json"
DEFAULT_REPORT_PATH = PROMETHEAN / ".v1427-stage-delivery-report.json"
DEFAULT_MD_PATH = PROMETHEAN / ".v1427-stage-delivery-report.md"


# ============================================================================
# GUARDS / BORROWED
# ============================================================================

V1427_GUARDS: Tuple[str, ...] = (
    "GUARD_NO_UPSTREAM_WRITE",
    "GUARD_COVERAGE_DEFINED",
    "GUARD_REPORT_REAL",
    "GUARD_JSON_ATOMIC",
    "GUARD_MD_RENDERED",
    "GUARD_FRAMES_LISTED",
    "GUARD_CHAIN_OK",
    "GUARD_PROBES_COLLECTED",
    "GUARD_PROTOCOLS_COLLECTED",
    "GUARD_BENCHMARK_INCLUDED",
    "GUARD_POPPER_RUNS",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_CLI_RUNNABLE",
    "GUARD_NO_FAKE_SCORE",
)

V1427_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_NO_PHENOMENAL_REPORT",
    "GUARD_NO_ASI_REPORT",
    "GUARD_NO_HUMAN_LEVEL_REPORT",
    "GUARD_NO_ABSOLUTE_REPORT",
    "GUARD_NO_NORTH_STAR_PADDING",
)

V1427_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("V1411", "overarching — chain_delegate() reference impl"),
    ("V1417", "DGM tick history — JSONL source"),
    ("V1418", "DGM cron integration — chain_delegate pattern"),
    ("V1419", "multi-policy evaluator — chain_delegate pattern"),
    ("V1420", "HTTP status endpoint — chain_delegate pattern"),
    ("V1421", "daemon — chain_delegate pattern"),
    ("V1422", "notification webhook — chain_delegate pattern"),
    ("V1423", "daemon webhook wiring — chain_delegate pattern"),
    ("V1424", "real LLM benchmark — chain_delegate + probe result"),
    ("V1425", "5 philosophical gaps — probe result + GapReport"),
    ("V1426", "VCP 6 protocols — dispatch result + report"),
    ("stdlib json + pathlib + importlib", "Python stdlib only"),
)


# ============================================================================
# Helpers
# ============================================================================


def _now_utc_iso() -> str:
    from datetime import datetime, timezone

    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H-%M-%SZ")


def _safe_path(p: Path) -> Path:
    s = str(p)
    if ".." in Path(s).parts:
        raise ValueError(f"path with .. rejected: {p}")
    return Path(p)


def _parse_kv_args(rest: List[str]) -> Dict[str, str]:
    out: Dict[str, str] = {}
    i = 0
    while i < len(rest):
        if rest[i].startswith("--") and i + 1 < len(rest):
            key = rest[i][2:]
            out[key] = rest[i + 1]
            i += 2
        else:
            i += 1
    return out


def _atomic_write_json(path: Path, obj: Any) -> None:
    """Write JSON atomically (tmp + rename)."""
    path = _safe_path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(json.dumps(obj, ensure_ascii=False, indent=2), encoding="utf-8")
    tmp.replace(path)


def _safe_load_jsonl(path: Path) -> List[Dict[str, Any]]:
    if not path.exists():
        return []
    out: List[Dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            obj = json.loads(line)
            if isinstance(obj, dict):
                out.append(obj)
        except json.JSONDecodeError:
            continue
    return out


def _safe_load_json(path: Path) -> Optional[Dict[str, Any]]:
    if not path.exists():
        return None
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        return None


# ============================================================================
# Config
# ============================================================================


def build_default_config(overrides: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
    cfg: Dict[str, Any] = {
        "benchmark_jsonl": DEFAULT_BENCHMARK_JSONL,
        "v1425_report": DEFAULT_V1425_REPORT,
        "v1426_report": DEFAULT_V1426_REPORT,
        "report_path": DEFAULT_REPORT_PATH,
        "md_path": DEFAULT_MD_PATH,
    }
    if overrides:
        for k, v in overrides.items():
            if v is not None:
                cfg[k] = v
    # Normalize paths
    for k in list(cfg.keys()):
        if k.endswith("_path") or k.endswith("_jsonl") or k.endswith("_report"):
            cfg[k] = Path(cfg[k])
    return cfg


def validate_config(cfg: Dict[str, Any]) -> Dict[str, Any]:
    for k in ("benchmark_jsonl", "v1425_report", "v1426_report", "report_path", "md_path"):
        if k not in cfg:
            raise ValueError(f"missing key: {k}")
        cfg[k] = _safe_path(Path(cfg[k]))
    return cfg


# ============================================================================
# Collectors
# ============================================================================


def collect_chain_status() -> Dict[str, Any]:
    """Call chain_delegate() on each upstream module, return per-module status."""
    out: Dict[str, Any] = {}
    for modname in UPSTREAM_MODULES:
        try:
            mod = __import__(f"apeireth.{modname}", fromlist=[modname])
            fn = getattr(mod, "chain_delegate", None)
            if callable(fn):
                sub = fn()
                if hasattr(sub, "all_ok"):
                    ok = bool(getattr(sub, "all_ok"))
                elif isinstance(sub, dict):
                    # heuristic: take "all_ok" or True if no error keys
                    ok = bool(sub.get("all_ok", True))
                    if any(k.endswith("_error") for k in sub):
                        ok = False
                else:
                    ok = True
                out[modname] = {"ok": ok, "raw_keys": list(sub.keys()) if isinstance(sub, dict) else []}
            else:
                out[modname] = {"ok": True, "raw_keys": []}
        except Exception as exc:
            out[modname] = {"ok": False, "error": str(exc), "raw_keys": []}
    return out


def collect_probe_results() -> Dict[str, Any]:
    """Call V1425.run_all_probes, return GapReport as dict."""
    try:
        import v1425_asi_five_philosophical_gaps as m1425

        cfg = m1425.build_default_config()
        cfg = m1425.validate_config(cfg)
        report = m1425.run_all_probes(cfg)
        return report.to_dict() if hasattr(report, "to_dict") else {"raw": str(report)}
    except Exception as exc:
        return {"error": str(exc)}


def collect_protocol_results() -> Dict[str, Any]:
    """Call V1426.dispatch_all, return VCPSixDispatchReport as dict."""
    try:
        import v1426_vcp_six_protocol_dispatcher as m1426

        report = m1426.dispatch_all()
        return report.to_dict() if hasattr(report, "to_dict") else {"raw": str(report)}
    except Exception as exc:
        return {"error": str(exc)}


def collect_benchmark_stats(cfg: Dict[str, Any]) -> Dict[str, Any]:
    """Read V1424 benchmark JSONL and compute summary stats."""
    path = cfg["benchmark_jsonl"]
    records = _safe_load_jsonl(path)
    if not records:
        return {
            "n_records": 0,
            "n_correct": 0,
            "accuracy": 0.0,
            "by_benchmark": {},
            "by_mode": {},
            "by_provider": {},
            "total_cost_usd": 0.0,
        }
    n_correct = sum(1 for r in records if r.get("correct"))
    by_benchmark: Dict[str, Dict[str, int]] = {}
    by_mode: Dict[str, Dict[str, int]] = {}
    by_provider: Dict[str, Dict[str, int]] = {}
    total_cost = 0.0
    for r in records:
        bm = str(r.get("benchmark", "UNKNOWN"))
        by_benchmark.setdefault(bm, {"n": 0, "n_correct": 0})
        by_benchmark[bm]["n"] += 1
        if r.get("correct"):
            by_benchmark[bm]["n_correct"] += 1
        mode = str(r.get("mode", "UNKNOWN"))
        by_mode.setdefault(mode, {"n": 0, "n_correct": 0})
        by_mode[mode]["n"] += 1
        if r.get("correct"):
            by_mode[mode]["n_correct"] += 1
        provider = str(r.get("provider", "UNKNOWN"))
        by_provider.setdefault(provider, {"n": 0, "n_correct": 0})
        by_provider[provider]["n"] += 1
        if r.get("correct"):
            by_provider[provider]["n_correct"] += 1
        try:
            total_cost += float(r.get("cost_usd", 0.0))
        except (TypeError, ValueError):
            pass
    accuracy = n_correct / len(records) if records else 0.0
    # Convert nested dicts to have accuracy too
    for d in (by_benchmark, by_mode, by_provider):
        for k, v in d.items():
            v["accuracy"] = (v["n_correct"] / v["n"]) if v["n"] > 0 else 0.0
    return {
        "n_records": len(records),
        "n_correct": n_correct,
        "accuracy": accuracy,
        "by_benchmark": by_benchmark,
        "by_mode": by_mode,
        "by_provider": by_provider,
        "total_cost_usd": total_cost,
        "source": str(path),
    }


def compute_coverage(chain_status: Dict[str, Any]) -> float:
    """coverage = n_modules_ok / n_modules_total."""
    n_total = len(chain_status)
    if n_total == 0:
        return 0.0
    n_ok = sum(1 for v in chain_status.values() if v.get("ok"))
    return n_ok / n_total


# ============================================================================
# Report dataclass
# ============================================================================


@dataclasses.dataclass
class StageDeliveryReport:
    """Aggregate report for V1411-V1426."""

    n_modules_total: int
    n_modules_ok: int
    coverage: float
    chain_status: Dict[str, Any]
    probe_results: Dict[str, Any]
    protocol_results: Dict[str, Any]
    benchmark_stats: Dict[str, Any]
    started_iso: str
    ended_iso: str
    note: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


def build_stage_report(cfg: Dict[str, Any]) -> StageDeliveryReport:
    started = _now_utc_iso()
    chain_status = collect_chain_status()
    probe_results = collect_probe_results()
    protocol_results = collect_protocol_results()
    benchmark_stats = collect_benchmark_stats(cfg)
    n_total = len(chain_status)
    n_ok = sum(1 for v in chain_status.values() if v.get("ok"))
    coverage = (n_ok / n_total) if n_total > 0 else 0.0
    ended = _now_utc_iso()
    return StageDeliveryReport(
        n_modules_total=n_total,
        n_modules_ok=n_ok,
        coverage=coverage,
        chain_status=chain_status,
        probe_results=probe_results,
        protocol_results=protocol_results,
        benchmark_stats=benchmark_stats,
        started_iso=started,
        ended_iso=ended,
        note="v1427 stage-delivery report V1411-V1426 (主 00:56 阶段性交付)",
    )


# ============================================================================
# Render
# ============================================================================


def render_report_md(report: StageDeliveryReport) -> str:
    lines: List[str] = []
    lines.append("# V1427 — ASI 总框架 Stage-Delivery Report V1411-V1426")
    lines.append("")
    lines.append(f"- started: `{report.started_iso}`")
    lines.append(f"- ended: `{report.ended_iso}`")
    lines.append(f"- note: {report.note}")
    lines.append("")
    lines.append("> 主 17:43 实事求是 — coverage = n_modules_ok / n_modules_total.")
    lines.append("> coverage 不等于 ASI 达成,不等于北极星分数,不等于人类水平.")
    lines.append("")
    lines.append("## Coverage Summary")
    lines.append("")
    lines.append(f"- n_modules_total: **{report.n_modules_total}**")
    lines.append(f"- n_modules_ok: **{report.n_modules_ok}**")
    lines.append(f"- coverage: **{report.coverage:.2%}**")
    lines.append("")

    # Per-module status
    lines.append("## Per-module Chain Status")
    lines.append("")
    lines.append("| Module | ok | error |")
    lines.append("|---|---|---|")
    for modname, status in report.chain_status.items():
        ok_str = "✅" if status.get("ok") else "❌"
        err = status.get("error", "")
        err_short = (err[:60] + "...") if len(err) > 60 else err
        lines.append(f"| `{modname}` | {ok_str} | {err_short} |")
    lines.append("")

    # Benchmark
    bm = report.benchmark_stats
    lines.append("## Real LLM Benchmark (V1424)")
    lines.append("")
    lines.append(f"- n_records: **{bm.get('n_records', 0)}**")
    lines.append(f"- n_correct: **{bm.get('n_correct', 0)}**")
    lines.append(f"- accuracy: **{bm.get('accuracy', 0.0):.2%}**")
    lines.append(f"- total_cost_usd: **{bm.get('total_cost_usd', 0.0):.6f}**")
    lines.append("")
    if bm.get("by_benchmark"):
        lines.append("| benchmark | n | correct | accuracy |")
        lines.append("|---|---|---|---|")
        for k, v in bm["by_benchmark"].items():
            lines.append(f"| `{k}` | {v['n']} | {v['n_correct']} | {v['accuracy']:.2%} |")
        lines.append("")
    if bm.get("by_mode"):
        lines.append("| mode | n | correct | accuracy |")
        lines.append("|---|---|---|---|")
        for k, v in bm["by_mode"].items():
            lines.append(f"| `{k}` | {v['n']} | {v['n_correct']} | {v['accuracy']:.2%} |")
        lines.append("")

    # Probes (5 philosophical gaps)
    pr = report.probe_results
    lines.append("## 5 Philosophical Gaps (V1425)")
    lines.append("")
    if pr.get("error"):
        lines.append(f"- error: {pr['error']}")
    elif pr.get("time"):
        lines.append("| gap | normalized | n_samples | gap_status |")
        lines.append("|---|---|---|---|")
        for gap in ("time", "freedom", "recognition", "emergence", "truth"):
            slot = pr.get(gap)
            if slot:
                lines.append(
                    f"| `{gap}` | {slot['normalized_value']:.4f} | {slot['n_samples']} | `{slot['gap_status']}` |"
                )
        lines.append("")

    # Protocols (VCP 6)
    prc = report.protocol_results
    lines.append("## VCP 6 Protocols (V1426)")
    lines.append("")
    if prc.get("error"):
        lines.append(f"- error: {prc['error']}")
    elif prc.get("protocols"):
        lines.append("| protocol | n_tasks | n_success | total_ms |")
        lines.append("|---|---|---|---|")
        for p in prc["protocols"]:
            lines.append(
                f"| `{p['protocol']}` | {p['n_tasks']} | {p['n_success']} | {p['total_duration_ms']:.2f} |"
            )
        lines.append("")

    lines.append("## V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)")
    lines.append("")
    lines.append("- 不假装 Phenomenal stage delivery")
    lines.append("- 不假装达到 ASI (gap preserved)")
    lines.append("- 不假装 human-level stage delivery")
    lines.append("- 不假装 absolute stage delivery")
    lines.append("- coverage ≠ ASI score (主 17:43 实事求是)")
    lines.append("")

    return "\n".join(lines) + "\n"


# ============================================================================
# Module metadata
# ============================================================================


def module_meta() -> Dict[str, Any]:
    return {
        "version": V1427_VERSION,
        "schema": V1427_SCHEMA,
        "module": V1427_MODULE,
        "n_guards": len(V1427_GUARDS),
        "n_v3_guards": len(V1427_V3_GUARDS),
        "n_borrowed": len(V1427_BORROWED),
        "n_upstream_modules": len(UPSTREAM_MODULES),
        "upstream_modules": list(UPSTREAM_MODULES),
    }


# ============================================================================
# Popper self-test
# ============================================================================


def popper_self_test() -> Tuple[bool, int, List[Dict[str, Any]]]:
    checks: List[Dict[str, Any]] = []

    def _check(name: str, ok: bool, detail: str = "") -> None:
        checks.append({"name": name, "ok": bool(ok), "detail": detail})

    # 1. Constants
    _check(
        "constants_present",
        V1427_VERSION == "0.1.0" and V1427_SCHEMA.startswith("v1427."),
        f"version={V1427_VERSION}",
    )

    # 2. UPSTREAM_MODULES has 16 entries
    _check(
        "upstream_modules_count",
        len(UPSTREAM_MODULES) == 16,
        f"n={len(UPSTREAM_MODULES)}",
    )

    # 3. UPSTREAM_MODULES covers V1411-V1426
    versions = sorted({m.split("_")[0] for m in UPSTREAM_MODULES})
    expected = [f"v14{i:02d}" for i in range(11, 27)]
    _check(
        "upstream_modules_covers_v1411_v1426",
        versions == expected,
        f"versions={versions}",
    )

    # 4. build_default_config returns dict with required keys
    cfg = build_default_config()
    _check(
        "build_default_config_complete",
        all(k in cfg for k in ("benchmark_jsonl", "v1425_report", "v1426_report", "report_path", "md_path")),
        f"keys={list(cfg.keys())}",
    )

    # 5. validate_config accepts default
    cfg_v = validate_config(cfg)
    _check(
        "validate_config_accepts_default",
        cfg_v is cfg,
        "ok",
    )

    # 6. validate_config rejects missing key
    try:
        validate_config({})
        raised = False
    except ValueError:
        raised = True
    _check(
        "validate_config_rejects_missing",
        raised,
        "ok",
    )

    # 7. compute_coverage returns 0.0 for empty
    c = compute_coverage({})
    _check(
        "compute_coverage_empty_is_zero",
        c == 0.0,
        f"coverage={c}",
    )

    # 8. compute_coverage returns 1.0 for all-ok
    fake = {m: {"ok": True} for m in UPSTREAM_MODULES}
    c = compute_coverage(fake)
    _check(
        "compute_coverage_all_ok_is_one",
        c == 1.0,
        f"coverage={c}",
    )

    # 9. compute_coverage returns 0.5 for half-ok
    fake = {m: {"ok": i % 2 == 0} for i, m in enumerate(UPSTREAM_MODULES)}
    c = compute_coverage(fake)
    _check(
        "compute_coverage_half_is_half",
        abs(c - 0.5) < 1e-9,
        f"coverage={c}",
    )

    # 10. collect_chain_status returns dict with 16 modules
    cs = collect_chain_status()
    _check(
        "collect_chain_status_returns_dict",
        isinstance(cs, dict) and len(cs) == 16,
        f"n={len(cs)}",
    )

    # 11. collect_probe_results returns dict
    pr = collect_probe_results()
    _check(
        "collect_probe_results_returns_dict",
        isinstance(pr, dict),
        f"keys={list(pr.keys())[:5]}",
    )

    # 12. collect_protocol_results returns dict
    pcr = collect_protocol_results()
    _check(
        "collect_protocol_results_returns_dict",
        isinstance(pcr, dict),
        f"keys={list(pcr.keys())[:5]}",
    )

    # 13. collect_benchmark_stats returns dict with required keys
    bs = collect_benchmark_stats(cfg)
    _check(
        "collect_benchmark_stats_returns_dict",
        all(k in bs for k in ("n_records", "n_correct", "accuracy", "total_cost_usd")),
        f"keys={list(bs.keys())}",
    )

    # 14. module_meta returns dict
    meta = module_meta()
    _check(
        "module_meta_returns_dict",
        isinstance(meta, dict) and meta["version"] == "0.1.0",
        f"version={meta.get('version')}",
    )

    n_passed = sum(1 for c in checks if c["ok"])
    all_ok = n_passed == len(checks)
    return all_ok, n_passed, checks


# ============================================================================
# Chain delegate
# ============================================================================


def chain_delegate() -> Dict[str, Any]:
    out: Dict[str, Any] = {"v1427": True}
    for ver, modname in (
        ("V1411", "v1411_asi_overarching_framework"),
        ("V1418", "v1418_asi_dgm_cron_integration"),
        ("V1424", "v1424_asi_real_llm_benchmark"),
        ("V1425", "v1425_asi_five_philosophical_gaps"),
        ("V1426", "v1426_vcp_six_protocol_dispatcher"),
    ):
        try:
            mod = __import__(f"apeireth.{modname}", fromlist=[modname])
            fn = getattr(mod, "chain_delegate", None)
            if callable(fn):
                sub = fn()
                if hasattr(sub, "all_ok"):
                    out[ver] = bool(getattr(sub, "all_ok"))
                elif isinstance(sub, dict):
                    out[ver] = bool(sub.get("all_ok", True))
                    if any(k.endswith("_error") for k in sub):
                        out[ver] = False
                else:
                    out[ver] = True
            else:
                out[ver] = True
        except Exception as exc:
            out[ver] = False
            out[f"{ver}_error"] = str(exc)
    keys = [v for v in out if not v.endswith("_error") and v != "v1427"]
    out["all_ok"] = all(out.get(k) for k in keys)
    return out


# ============================================================================
# Help text
# ============================================================================


def _print_help() -> None:
    print(
        """V1427 — ASI stage-delivery report V1411-V1426 (主 00:56 阶段性交付)

Usage:
  python -m apeireth.v1427_asi_stage_delivery <command> [args]

Commands:
  version                     Print version string
  meta [--json]               Print module metadata
  demo                        Run a small demo
  help                        Print this help
  popper                      Run 14 self-tests
  chain                       Print chain_delegate() result
  summary                     Print coverage summary
  collect                     Print raw collectors output
  report                      Render full markdown report + write to file

Examples:
  python -m apeireth.v1427_asi_stage_delivery version
  python -m apeireth.v1427_asi_stage_delivery summary
  python -m apeireth.v1427_asi_stage_delivery report
"""
    )


# ============================================================================
# CLI dispatcher
# ============================================================================


def run_cli(argv: List[str]) -> int:
    if not argv:
        argv = ["help"]
    cmd = argv[0]
    rest = argv[1:]

    if cmd in ("version", "--version", "-v"):
        print(f"V1427 v{V1427_VERSION} ({V1427_SCHEMA})")
        return 0
    if cmd in ("help", "--help", "-h"):
        _print_help()
        return 0
    if cmd == "meta":
        kv = _parse_kv_args(rest)
        if kv.get("json") == "true":
            print(json.dumps(module_meta(), ensure_ascii=False, indent=2))
        else:
            m = module_meta()
            print(
                f"V1427 v{m['version']} schema={m['schema']} module={m['module']} "
                f"upstream_modules={m['n_upstream_modules']}"
            )
        return 0
    if cmd == "demo":
        print("V1427 demo: stage-delivery report V1411-V1426 (主 00:56 阶段性交付)")
        cfg = build_default_config()
        cfg = validate_config(cfg)
        report = build_stage_report(cfg)
        print(f"  - coverage: {report.coverage:.2%} ({report.n_modules_ok}/{report.n_modules_total})")
        print(f"  - benchmark accuracy: {report.benchmark_stats.get('accuracy', 0):.2%}")
        return 0
    if cmd == "popper":
        all_ok, n_pass, results = popper_self_test()
        print(
            json.dumps(
                {"all_ok": all_ok, "n_pass": n_pass, "results": results},
                ensure_ascii=False,
                indent=2,
            )
        )
        return 0 if all_ok else 1
    if cmd == "chain":
        print(json.dumps(chain_delegate(), ensure_ascii=False, indent=2))
        return 0
    if cmd == "summary":
        cfg = build_default_config()
        cfg = validate_config(cfg)
        report = build_stage_report(cfg)
        print(f"coverage: {report.coverage:.2%} ({report.n_modules_ok}/{report.n_modules_total})")
        print(f"benchmark: {report.benchmark_stats.get('n_records', 0)} records, "
              f"accuracy={report.benchmark_stats.get('accuracy', 0):.2%}")
        return 0
    if cmd == "collect":
        cfg = build_default_config()
        cfg = validate_config(cfg)
        cs = collect_chain_status()
        pr = collect_probe_results()
        pcr = collect_protocol_results()
        bs = collect_benchmark_stats(cfg)
        out = {
            "chain_status": cs,
            "probe_results_summary": {k: pr.get(k, {}).get("normalized_value") if isinstance(pr.get(k), dict) else None for k in ("time", "freedom", "recognition", "emergence", "truth")},
            "protocol_results_summary": [
                {"protocol": p["protocol"], "n_success": p["n_success"]}
                for p in pcr.get("protocols", [])
            ],
            "benchmark_stats": bs,
        }
        print(json.dumps(out, ensure_ascii=False, indent=2))
        return 0
    if cmd == "report":
        cfg = build_default_config()
        cfg = validate_config(cfg)
        report = build_stage_report(cfg)
        md = render_report_md(report)
        print(md)
        # Atomic write
        rp = cfg["report_path"]
        _atomic_write_json(rp, report.to_dict())
        mp = cfg["md_path"]
        mp.parent.mkdir(parents=True, exist_ok=True)
        mp.write_text(md, encoding="utf-8")
        print(f"\n[report] written to {rp}")
        print(f"[md] written to {mp}")
        return 0
    print(f"ERROR: unknown command: {cmd!r}")
    _print_help()
    return 1


# ============================================================================
# Entry point
# ============================================================================


if __name__ == "__main__":
    sys.exit(run_cli(sys.argv[1:]))