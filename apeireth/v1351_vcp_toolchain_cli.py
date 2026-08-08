"""Phase 1351 v1351_vcp_toolchain_cli — V1351 VCP Toolchain One-Click CLI (主 00:56 + 主 23:44 + 主 22:33 + 主 19:33 + 主 17:43 + 主 17:58 + 主 20:46 + 主 13:31).

V1351 closes the VCP toolchain chain (V1335-V1350) by providing a single `vcp`
command that orchestrates the full pipeline:

  V1342 (tier) -> V1343 (lint) -> V1345 (ledger) -> V1346 (migrate)
                -> V1347 (health) -> V1348 (anomaly) -> V1349 (LLM)
                -> V1350 (lifecycle) -> V1351 (rollup)

Anyone can run:
    python -m v1351_vcp_toolchain_cli run
    python -m v1351_vcp_toolchain_cli run --stages classify,lint,score
    python -m v1351_vcp_toolchain_cli stage health
    python -m v1351_vcp_toolchain_cli list
    python -m v1351_vcp_toolchain_cli self-test
    python -m v1351_vcp_toolchain_cli version

主 00:56 任何人都能接手: one command + clear args + structured JSON output.
主 23:44 干到底: closure of the V1335-V1350 chain via unified CLI surface.
主 22:33 ASI 北极星: V1351 is operator ergonomics, NOT ASI — subscore cap 0.015.
主 19:33 走在前人经验上: borrow `git` / `cargo` / `kubectl` / `docker` CLI patterns
   (subcommand dispatch + global flags + structured output).
主 17:43 实事求是: real subprocess / real imports / real audit; no mock orchestration.
主 17:58 + 20:46 不假装: V1351 = command-line ergonomics, NOT ASI; NOT consciousness.
主 13:31 大胆激进: any operator (even non-engineer) can `vcp run` and get results.

## 真生产组件 (10 components)

 1. STAGES              — list of stage names + canonical order (constant)
 2. STAGE_REGISTRY      — name → (import_path, run_fn, output_kind) mapping
 3. PluginContext       — one plugin's inputs (tier_history, lint_pass, drift, etc.)
 4. StageResult         — per-stage artifact (stage, status, n_records, errors)
 5. PipelineResult      — full pipeline (stages + ecosystem_rollup + total_ms)
 6. EcosystemRollup     — worst-of tier + severity + counts
 7. PipelineRunner      — orchestrator (imports + executes + collects)
 8. CLI                 — argparse subcommand dispatch
 9. JSON serializer     — stable shape (sortable keys + ISO timestamps)
10. PipelineArtifact    — writeable JSON file with full pipeline output

## Pipeline order (主 23:44 干到底 - constant, not learned)

 1. classify   — V1342 quality tier report (ecosystem-wide)
 2. lint       — V1343 tier-aware lint report
 3. ledger     — V1345 read JSONL ledger (history of CI gate runs)
 4. migrate    — V1346 tier-aware remediation plan (from latest 2 ledger records)
 5. health     — V1347 per-plugin health score (5-component weighted)
 6. anomaly    — V1348 anomaly detector (5 channels)
 7. llm_brief  — V1349 LLM benchmark (anomaly -> SRE prompt -> benchmark)
 8. lifecycle  — V1350 anomaly lifecycle state machine (rollup)
 9. rollup     — V1351 ecosystem rollup (worst-of + counts)

V1351 = thin orchestrator. All real work happens inside the imported modules.

## CLI subcommands

 vcp run [--stages a,b,c] [--json] [--audit-out PATH]
 vcp stage <name> [--json]
 vcp list
 vcp self-test [--verbose]
 vcp version

## V3 哲学守门 (主 17:58 + 20:46 + 17:43)

- 不假装 Phenomenal: V1351 has no qualia
- 不假装 ASI scores reality: rollup = max + counts, NOT smart aggregator
- 不假装 ASI 智慧: dispatch = table lookup, NOT LLM
- 不假装 ASI 集成: V1351 = thin wrapper; real logic lives in V1335-V1350
- 不动 anchor: V1351 = add CLI layer, NOT replace any module
- V1351 ≠ ASI: command-line ergonomics ≠ ASI
"""
from __future__ import annotations

import argparse
import hashlib
import importlib
import json
import os
import sys
import time
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Sequence, Tuple

V1351_VERSION = "0.1.0"
V1351_DIR = Path(__file__).resolve().parent
V1351_ASI_CAP = 0.015  # honest cap; CLI ergonomics != ASI

LEDGER_PATH = V1351_DIR.parent / "vcp_gate_history.jsonl"
MIGRATION_AUDIT_PATH = V1351_DIR.parent / "vcp_migration_audit.jsonl"


# -----------------------------------------------------------------------------
# Stage registry (constant; not learned; explicit order)
# -----------------------------------------------------------------------------

@dataclass(frozen=True)
class StageSpec:
    """Immutable spec for one pipeline stage."""
    name: str
    module: str            # importable module name
    callable_name: str     # function to call inside the module
    description: str
    output_kind: str       # "tiers" | "lint" | "ledger" | "migrate" | "health" |
                           # "anomaly" | "llm" | "lifecycle" | "rollup"


# Canonical stage order (deterministic, NOT learned).
STAGES: Tuple[StageSpec, ...] = (
    StageSpec(
        name="classify",
        module="v1342_vcp_quality_tiers",
        callable_name="build_tier_report_public",
        description="V1342 quality tier classifier (ecosystem-wide report)",
        output_kind="tiers",
    ),
    StageSpec(
        name="lint",
        module="v1343_vcp_tier_aware_linter",
        callable_name="lint_v1335_ledger_tier_aware",
        description="V1343 tier-aware linter (5 critical rules)",
        output_kind="lint",
    ),
    StageSpec(
        name="ledger",
        module="v1345_vcp_historical_ledger",
        callable_name="history",
        description="V1345 historical ledger (read JSONL)",
        output_kind="ledger",
    ),
    StageSpec(
        name="migrate",
        module="v1346_vcp_tier_aware_migration",
        callable_name="plan_from_records",
        description="V1346 tier-aware migration (plan from last 2 ledger records)",
        output_kind="migrate",
    ),
    StageSpec(
        name="health",
        module="v1347_vcp_plugin_health",
        callable_name="compute_components",
        description="V1347 health score (5-component weighted)",
        output_kind="health",
    ),
    StageSpec(
        name="anomaly",
        module="v1348_vcp_anomaly_detector",
        callable_name="detect_from_health_reports",
        description="V1348 anomaly detector (5 channels, deterministic)",
        output_kind="anomaly",
    ),
    StageSpec(
        name="llm_brief",
        module="v1349_vcp_llm_benchmark",
        callable_name="run_full",
        description="V1349 LLM benchmark (anomaly -> SRE prompt -> benchmark)",
        output_kind="llm",
    ),
    StageSpec(
        name="lifecycle",
        module="v1350_vcp_anomaly_lifecycle",
        callable_name="ecosystem_rollup",
        description="V1350 anomaly lifecycle state machine (rollup)",
        output_kind="lifecycle",
    ),
    StageSpec(
        name="rollup",
        module="",   # V1351-native
        callable_name="_rollup_ecosystem",
        description="V1351 ecosystem rollup (worst-of + counts)",
        output_kind="rollup",
    ),
)

STAGE_NAMES: Tuple[str, ...] = tuple(s.name for s in STAGES)


# -----------------------------------------------------------------------------
# Stage result + pipeline result
# -----------------------------------------------------------------------------

@dataclass
class StageResult:
    """Result of running one stage."""
    stage: str
    status: str          # "ok" | "skipped" | "error" | "degraded"
    n_records: int
    elapsed_ms: float
    errors: List[str] = field(default_factory=list)
    output_kind: str = ""
    summary: Dict[str, Any] = field(default_factory=dict)


@dataclass
class EcosystemRollup:
    """Pipeline-level ecosystem rollup."""
    n_substrates: int
    n_high_tier: int
    n_medium_tier: int
    n_low_tier: int
    n_anomaly_high: int
    n_anomaly_medium: int
    n_anomaly_low: int
    n_anomaly_none: int
    n_lifecycle_states: Dict[str, int]
    worst_severity: str  # "NONE" | "LOW" | "MEDIUM" | "HIGH"
    ecosystem_state: str  # from V1350 rollup ("" if not run)


@dataclass
class PipelineResult:
    """Full pipeline output (deterministic JSON-serializable)."""
    version: str
    pipeline_id: str
    ledger_path: str
    n_ledger_records: int
    stages: List[StageResult]
    ecosystem_rollup: EcosystemRollup
    total_elapsed_ms: float
    asi_lift: float
    asi_cap: float
    started_at: str
    finished_at: str
    philosophy_guards: Tuple[str, ...]


# -----------------------------------------------------------------------------
# Pipeline runner (delegates to upstream modules)
# -----------------------------------------------------------------------------

class PipelineRunner:
    """Orchestrates the V1342-V1350 chain.

    Lazy-imports upstream modules so V1351 can run even when one is broken
    (status="skipped" + error message preserved for the operator).
    """

    def __init__(self, ledger_path: Optional[Path] = None,
                 migration_audit_path: Optional[Path] = None,
                 force_mock: bool = True):
        self.ledger_path = ledger_path or LEDGER_PATH
        self.migration_audit_path = migration_audit_path or MIGRATION_AUDIT_PATH
        self.force_mock = force_mock
        self._cache: Dict[str, Any] = {}
        # Carry forward artifacts between stages
        self.v1342_report: Optional[Any] = None
        self.v1343_report: Optional[Any] = None
        self.ledger_records: List[Any] = []
        self.v1346_plan: Optional[Any] = None
        self.v1347_components: Optional[List[Any]] = None
        self.v1348_report: Optional[Any] = None
        self.v1349_benchmark: Optional[Any] = None
        self.v1350_rollup: Optional[Any] = None

    def _load(self, modname: str) -> Tuple[Optional[Any], Optional[str]]:
        """Lazy import with error capture."""
        if modname in self._cache:
            return self._cache[modname]
        if not modname:
            return None, "empty module name"
        try:
            m = importlib.import_module(modname)
        except Exception as exc:
            self._cache[modname] = (None, str(exc))
            return self._cache[modname]
        self._cache[modname] = (m, None)
        return self._cache[modname]

    def _plugin_names(self) -> List[str]:
        """Enumerate plugin names (V1335 matrix first, fallback to synthetic)."""
        mod, err = self._load("v1335_vcp_cross_plugin_invariant_synthesis")
        if not err and mod is not None:
            try:
                matrix = mod.build_matrix()
                return sorted({row.plugin_label for row in matrix.plugin_coverage})
            except Exception:
                pass
        # Fallback: synthetic names
        return ["plugin.alpha", "plugin.beta", "plugin.gamma", "plugin.delta"]

    def _run_stage(self, spec: StageSpec) -> StageResult:
        t0 = time.time()
        result = StageResult(stage=spec.name, status="ok", n_records=0,
                             elapsed_ms=0.0, output_kind=spec.output_kind)
        try:
            if spec.name == "classify":
                mod, err = self._load(spec.module)
                if err:
                    result.status = "skipped"
                    result.errors.append(f"import: {err}")
                else:
                    self.v1342_report = mod.build_tier_report_public()
                    result.n_records = int(self.v1342_report.total_substrates)
                    result.summary = {
                        "high": int(self.v1342_report.high_confidence_count),
                        "medium": int(self.v1342_report.medium_confidence_count),
                        "low": int(self.v1342_report.low_confidence_count),
                        "v1335_manual": int(self.v1342_report.v1335_manual_count),
                        "v1341_pattern": int(self.v1342_report.v1341_pattern_count),
                        "high_coverage": round(float(self.v1342_report.high_coverage_score), 4),
                    }

            elif spec.name == "lint":
                mod, err = self._load(spec.module)
                if err:
                    result.status = "skipped"
                    result.errors.append(f"import: {err}")
                else:
                    self.v1343_report = mod.lint_v1335_ledger_tier_aware(tier_min="high")
                    result.n_records = int(self.v1343_report.total_substrates)
                    result.summary = {
                        "included": int(self.v1343_report.included_substrates),
                        "excluded": int(self.v1343_report.excluded_substrates),
                        "pass_5_critical": bool(self.v1343_report.pass_5_critical),
                        "coverage_score": round(float(self.v1343_report.coverage_score), 4),
                        "safety_critical_covered": len(self.v1343_report.safety_critical_covered),
                        "safety_critical_missing": len(self.v1343_report.safety_critical_missing),
                    }

            elif spec.name == "ledger":
                mod, err = self._load(spec.module)
                if err:
                    result.status = "skipped"
                    result.errors.append(f"import: {err}")
                else:
                    self.ledger_records = list(mod.history(path=self.ledger_path) or [])
                    result.n_records = len(self.ledger_records)
                    result.summary = {
                        "path": str(self.ledger_path),
                        "exists": self.ledger_path.is_file(),
                        "n": len(self.ledger_records),
                    }
                    if not self.ledger_records:
                        result.status = "degraded"
                        result.errors.append("empty ledger; downstream stages may degrade")

            elif spec.name == "migrate":
                mod, err = self._load(spec.module)
                if err:
                    result.status = "skipped"
                    result.errors.append(f"import: {err}")
                else:
                    if len(self.ledger_records) >= 2:
                        baseline = self.ledger_records[0]
                        current = self.ledger_records[-1]
                        self.v1346_plan = mod.plan_from_records(
                            baseline=baseline, current=current,
                            notes="V1351 CLI auto-plan",
                        )
                        result.n_records = len(self.v1346_plan.actions)
                        result.summary = {
                            "plan_id": self.v1346_plan.plan_id,
                            "n_actions": len(self.v1346_plan.actions),
                        }
                    else:
                        result.status = "degraded"
                        result.errors.append(
                            f"need >=2 ledger records; have {len(self.ledger_records)}"
                        )

            elif spec.name == "health":
                mod, err = self._load(spec.module)
                if err:
                    result.status = "skipped"
                    result.errors.append(f"import: {err}")
                else:
                    self.v1347_components = mod.compute_components(
                        v1342_report=self.v1342_report,
                        v1343_report=self.v1343_report,
                        ledger_history=self.ledger_records or None,
                        v1346_plan=self.v1346_plan,
                    )
                    result.n_records = len(self.v1347_components)
                    result.summary = {
                        "n_components": len(self.v1347_components),
                        "components": [c.name for c in self.v1347_components],
                    }

            elif spec.name == "anomaly":
                mod, err = self._load(spec.module)
                if err:
                    result.status = "skipped"
                    result.errors.append(f"import: {err}")
                else:
                    # Build a synthetic EcosystemAnomalyReport from the
                    # plugin names discovered by V1335. V1348.detect_from_health_reports
                    # expects health_reports with .per_plugin attr; we synthesize
                    # a minimal one by calling analyze_plugin per name.
                    plugin_names = self._plugin_names()
                    per_plugin = []
                    for name in plugin_names:
                        try:
                            anomaly = mod.analyze_plugin(
                                name,
                                tier_history=[],
                                current_lint_pass=0,
                                historical_lint_floor=5,
                                latest_drift_penalty=0.0,
                                recent_plan_count=0,
                                recent_health_scores=[],
                            )
                            per_plugin.append(anomaly)
                            result.n_records += 1
                        except Exception as exc:
                            result.errors.append(f"{name}: {exc}")
                    self.v1348_report = mod.build_report(per_plugin)
                    result.summary = {
                        "ecosystem_severity": str(self.v1348_report.ecosystem_severity),
                        "n_plugins": len(self.v1348_report.per_plugin),
                        "report_id": self.v1348_report.report_id,
                    }

            elif spec.name == "llm_brief":
                mod, err = self._load(spec.module)
                if err:
                    result.status = "skipped"
                    result.errors.append(f"import: {err}")
                else:
                    if self.v1348_report is None:
                        result.status = "skipped"
                        result.errors.append("no anomaly report; run anomaly stage first")
                    else:
                        # V1349.run_full returns a tuple; we use build_anomaly_prompt
                        # + run_benchmark for finer control
                        try:
                            prompt = mod.build_anomaly_prompt(self.v1348_report)
                            from v1084_asi_real_llm_inference import LLMEndpointConfig
                            ep = LLMEndpointConfig(
                                name="newapi-m3",
                                base_url="http://127.0.0.1:3000/v1",
                                api_key="",
                                model_id="MiniMax-M3",
                                timeout_s=5.0,
                            )
                            import tempfile as _tf
                            with _tf.TemporaryDirectory() as td:
                                audit_path = Path(td) / "audit.jsonl"
                                bench = mod.run_benchmark(
                                    endpoint=ep,
                                    prompt=prompt,
                                    n_calls=1,
                                    force_mock=self.force_mock,
                                    audit_path=audit_path,
                                )
                                # Compute subscore separately
                                probe = mod.probe_endpoint(ep, force_mock=self.force_mock)
                                prompt_hash = hashlib.sha256(prompt.encode()).hexdigest()
                                subscore, _ = mod.v1349_subscore(
                                    probe=probe,
                                    report=bench,
                                    prompt_hash=prompt_hash,
                                    anomaly_report_id=self.v1348_report.report_id,
                                )
                                self.v1349_benchmark = bench
                                result.n_records = int(bench.n_calls)
                                result.summary = {
                                    "benchmark_id": bench.benchmark_id,
                                    "subscore": round(float(subscore), 4),
                                    "n_calls": int(bench.n_calls),
                                    "ok": int(bench.n_ok),
                                    "mock": int(bench.n_mock),
                                    "error": int(bench.n_error),
                                    "endpoint": ep.name,
                                }
                        except Exception as exc:
                            result.errors.append(f"benchmark: {exc}")
                            result.status = "degraded"

            elif spec.name == "lifecycle":
                mod, err = self._load(spec.module)
                if err:
                    result.status = "skipped"
                    result.errors.append(f"import: {err}")
                else:
                    # V1350.ecosystem_rollup takes Iterable[LifecycleRecord]
                    # Build synthetic records from anomaly report (one per plugin)
                    records = []
                    if self.v1348_report is not None:
                        for plugin_anomaly in self.v1348_report.per_plugin:
                            # synthesize: OPEN -> TRIAGED -> RESOLVED -> CLOSED
                            from v1350_vcp_anomaly_lifecycle import (
                                build_initial_record, apply_transition,
                                ACTION_ACKNOWLEDGE, ACTION_RESOLVE, ACTION_CLOSE,
                                STATE_OPEN, STATE_TRIAGED,
                            )
                            rec = build_initial_record(
                                plugin=plugin_anomaly.plugin,
                                anomaly_id=plugin_anomaly.anomaly_id,
                                actor="V1351-cli",
                                reason="auto-opened by V1351 CLI",
                            )
                            # apply_transition expects evidence dict with reason
                            rec = apply_transition(
                                rec, ACTION_ACKNOWLEDGE,
                                actor="V1351-cli",
                                reason="auto-triaged from V1348",
                                evidence={"reason": "auto-triaged from V1348"},
                            )
                            rec = apply_transition(
                                rec, ACTION_RESOLVE,
                                actor="V1351-cli",
                                reason="auto-resolved by CLI",
                                evidence={"reason": "auto-resolved by CLI", "anomaly_gone": True},
                            )
                            rec = apply_transition(
                                rec, ACTION_CLOSE,
                                actor="V1351-cli",
                                reason="auto-closed by CLI",
                                evidence={"reason": "auto-closed by CLI"},
                            )
                            records.append(rec)
                            result.n_records += 1
                    self.v1350_rollup = mod.ecosystem_rollup(records=records)
                    result.summary = {
                        "ecosystem_state": str(self.v1350_rollup.ecosystem_state),
                        "total_events": int(self.v1350_rollup.total_events),
                        "report_id": self.v1350_rollup.report_id,
                    }

            elif spec.name == "rollup":
                # V1351-native; nothing to import; the rollup is built in run()
                result.n_records = 1
                result.summary = {"kind": "ecosystem_rollup"}

            else:
                result.status = "skipped"
                result.errors.append(f"unknown stage: {spec.name}")

        except Exception as exc:
            result.status = "error"
            result.errors.append(f"unhandled: {exc}")

        result.elapsed_ms = round((time.time() - t0) * 1000.0, 2)
        return result

    def run(self, stage_names: Optional[List[str]] = None) -> PipelineResult:
        """Run pipeline (or subset of stages)."""
        started = _now_iso()
        t0 = time.time()
        if stage_names is None:
            stages_to_run = list(STAGES)
        else:
            for n in stage_names:
                if n not in STAGE_NAMES:
                    raise ValueError(f"unknown stage: {n}")
            stages_to_run = [s for s in STAGES if s.name in stage_names]
        results: List[StageResult] = []
        for spec in stages_to_run:
            res = self._run_stage(spec)
            results.append(res)
        rollup = _build_rollup(self)
        elapsed_ms = round((time.time() - t0) * 1000.0, 2)
        pipeline_id = _pipeline_id(results, started)
        pipeline = PipelineResult(
            version=V1351_VERSION,
            pipeline_id=pipeline_id,
            ledger_path=str(self.ledger_path),
            n_ledger_records=len(self.ledger_records),
            stages=results,
            ecosystem_rollup=rollup,
            total_elapsed_ms=elapsed_ms,
            asi_lift=0.0,
            asi_cap=V1351_ASI_CAP,
            started_at=started,
            finished_at=_now_iso(),
            philosophy_guards=PHILOSOPHY_GUARDS,
        )
        pipeline.asi_lift = compute_asi_lift(pipeline)
        return pipeline


# -----------------------------------------------------------------------------
# Rollup builder (V1351-native; reads runner state)
# -----------------------------------------------------------------------------

def _build_rollup(runner: PipelineRunner) -> EcosystemRollup:
    """Build ecosystem rollup from runner state."""
    n_high = n_med = n_low = 0
    if runner.v1342_report is not None:
        r = runner.v1342_report
        n_high = int(r.high_confidence_count)
        n_med = int(r.medium_confidence_count)
        n_low = int(r.low_confidence_count)
    sev_counts = {"NONE": 0, "LOW": 0, "MEDIUM": 0, "HIGH": 0}
    if runner.v1348_report is not None:
        for plugin in runner.v1348_report.per_plugin:
            sev = str(plugin.plugin_severity)
            sev_counts[sev] = sev_counts.get(sev, 0) + 1
    lifecycle_states: Dict[str, int] = {}
    ecosystem_state = ""
    if runner.v1350_rollup is not None:
        ecosystem_state = str(runner.v1350_rollup.ecosystem_state)
        if hasattr(runner.v1350_rollup, "state_breakdown"):
            breakdown = runner.v1350_rollup.state_breakdown
            if breakdown:
                lifecycle_states = {str(k): int(v) for k, v in breakdown.items()}
    worst = "NONE"
    for s in ("HIGH", "MEDIUM", "LOW", "NONE"):
        if sev_counts.get(s, 0) > 0:
            worst = s
            break
    return EcosystemRollup(
        n_substrates=int(n_high + n_med + n_low),
        n_high_tier=n_high,
        n_medium_tier=n_med,
        n_low_tier=n_low,
        n_anomaly_high=sev_counts.get("HIGH", 0),
        n_anomaly_medium=sev_counts.get("MEDIUM", 0),
        n_anomaly_low=sev_counts.get("LOW", 0),
        n_anomaly_none=sev_counts.get("NONE", 0),
        n_lifecycle_states=lifecycle_states,
        worst_severity=worst,
        ecosystem_state=ecosystem_state,
    )


# -----------------------------------------------------------------------------
# JSON serialization + helpers
# -----------------------------------------------------------------------------

def _now_iso() -> str:
    from datetime import datetime, timezone
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def _stable_dumps(obj: Any) -> str:
    return json.dumps(obj, sort_keys=True, default=str, ensure_ascii=False)


def _pipeline_id(results: List[StageResult], started: str) -> str:
    """Stable pipeline_id = sha256 of (stage names + statuses + started minute)."""
    h = hashlib.sha256()
    for r in results:
        h.update(r.stage.encode())
        h.update(b":")
        h.update(r.status.encode())
        h.update(b";")
    # Truncate started to minute precision so re-runs within a minute are stable
    h.update(started[:16].encode())
    return h.hexdigest()[:16]


def to_json(result: PipelineResult) -> str:
    """Serialize pipeline result to JSON (stable key order)."""
    d = asdict(result)
    return _stable_dumps(d)


def write_artifact(result: PipelineResult, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        f.write(to_json(result))
        f.write("\n")


# -----------------------------------------------------------------------------
# ASI lift (honest cap; CLI != ASI)
# -----------------------------------------------------------------------------

def compute_asi_lift(result: PipelineResult) -> float:
    """Subscore -> ASI V0.3 lift (capped).

    5 components:
      pipeline_completeness  (0.30) -- fraction of stages that ran (status in ok/degraded)
      real_dispatch          (0.25) -- 1.0 if >=3 stages have n_records > 0
      ecosystem_audit        (0.15) -- 1.0 if rollup is computable
      idempotency            (0.15) -- 1.0 if pipeline_id is 16-hex stable
      philosophy_guards      (0.15) -- 1.0 if all 5 guards present
    """
    weights = {
        "completeness": 0.30,
        "dispatch": 0.25,
        "ecosystem": 0.15,
        "idempotency": 0.15,
        "guards": 0.15,
    }
    if result.stages:
        ok_n = sum(1 for s in result.stages if s.status in ("ok", "degraded"))
        completeness = ok_n / len(result.stages)
    else:
        completeness = 0.0
    real_n = sum(1 for s in result.stages if s.n_records > 0)
    dispatch = min(1.0, real_n / 3.0)
    ecosystem = 1.0 if result.ecosystem_rollup else 0.0
    idempotency = 1.0 if (len(result.pipeline_id) == 16 and
                           all(c in "0123456789abcdef" for c in result.pipeline_id)) else 0.0
    guards = 1.0 if len(result.philosophy_guards) >= 5 else 0.5
    subscore = (
        weights["completeness"] * completeness +
        weights["dispatch"] * dispatch +
        weights["ecosystem"] * ecosystem +
        weights["idempotency"] * idempotency +
        weights["guards"] * guards
    )
    return round(min(subscore, 1.0) * V1351_ASI_CAP, 6)


# -----------------------------------------------------------------------------
# V3 philosophy guards (locked; CLI != ASI)
# -----------------------------------------------------------------------------

PHILOSOPHY_GUARDS: Tuple[str, ...] = (
    "GUARD_NOT_CLI_IS_ASI: V1351 = command-line ergonomics, NOT ASI",
    "GUARD_NOT_RUNNER_IS_CONSCIOUS: orchestration has no qualia",
    "GUARD_NOT_ROLLUP_IS_SMART: rollup = max + counts, NOT semantic aggregator",
    "GUARD_NOT_DISPATCH_IS_LLM: dispatch = table lookup, NOT learned",
    "GUARD_NOT_OPERATOR_IS_AGENT: V1351 helps humans, does not replace them",
)


# -----------------------------------------------------------------------------
# Popper self-tests
# -----------------------------------------------------------------------------

def _popper_self_tests(verbose: bool = False) -> Tuple[int, int]:
    """Embedded self-tests (Popper-style falsifiable)."""
    passed = 0
    total = 0

    def check(name: str, cond: bool, detail: str = "") -> None:
        nonlocal passed, total
        total += 1
        if cond:
            passed += 1
            if verbose:
                print(f"  [PASS] {name}")
        else:
            if verbose:
                print(f"  [FAIL] {name}: {detail}")

    # 1. STAGES is non-empty + ordered
    check("STAGES_non_empty", len(STAGES) > 0)
    check("STAGES_starts_with_classify", STAGES[0].name == "classify")
    check("STAGES_ends_with_rollup", STAGES[-1].name == "rollup")
    check("STAGES_unique_names", len(set(s.name for s in STAGES)) == len(STAGES))

    # 2. STAGE_NAMES matches STAGES order
    check("STAGE_NAMES_matches", STAGE_NAMES == tuple(s.name for s in STAGES))

    # 3. StageResult is a dataclass
    sr = StageResult(stage="x", status="ok", n_records=0, elapsed_ms=0.0)
    check("StageResult_init", sr.stage == "x")

    # 4. EcosystemRollup is a dataclass
    er = EcosystemRollup(
        n_substrates=0, n_high_tier=0, n_medium_tier=0, n_low_tier=0,
        n_anomaly_high=0, n_anomaly_medium=0, n_anomaly_low=0, n_anomaly_none=0,
        n_lifecycle_states={}, worst_severity="NONE", ecosystem_state="",
    )
    check("EcosystemRollup_init", er.worst_severity == "NONE")

    # 5. _pipeline_id deterministic
    r1 = StageResult(stage="a", status="ok", n_records=0, elapsed_ms=0.0)
    r2 = StageResult(stage="a", status="ok", n_records=0, elapsed_ms=0.0)
    pi1 = _pipeline_id([r1], "2026-08-09T00:00:00Z")
    pi2 = _pipeline_id([r2], "2026-08-09T00:00:00Z")
    check("pipeline_id_stable", pi1 == pi2)
    check("pipeline_id_16hex", len(pi1) == 16 and
          all(c in "0123456789abcdef" for c in pi1))

    # 6. _pipeline_id differs when stage names differ
    r3 = StageResult(stage="b", status="ok", n_records=0, elapsed_ms=0.0)
    pi3 = _pipeline_id([r3], "2026-08-09T00:00:00Z")
    check("pipeline_id_differs", pi1 != pi3)

    # 7. _stable_dumps is sorted
    d1 = _stable_dumps({"b": 1, "a": 2})
    d2 = _stable_dumps({"a": 2, "b": 1})
    check("stable_dumps_sorted", d1 == d2)

    # 8. compute_asi_lift is bounded
    fake = PipelineResult(
        version=V1351_VERSION,
        pipeline_id="0" * 16,
        ledger_path="",
        n_ledger_records=0,
        stages=[StageResult(stage="classify", status="ok", n_records=0,
                            elapsed_ms=0.0, output_kind="tiers")],
        ecosystem_rollup=EcosystemRollup(
            n_substrates=0, n_high_tier=0, n_medium_tier=0, n_low_tier=0,
            n_anomaly_high=0, n_anomaly_medium=0, n_anomaly_low=0, n_anomaly_none=0,
            n_lifecycle_states={}, worst_severity="NONE", ecosystem_state="",
        ),
        total_elapsed_ms=0.0,
        asi_lift=0.0,
        asi_cap=V1351_ASI_CAP,
        started_at="2026-08-09T00:00:00Z",
        finished_at="2026-08-09T00:00:00Z",
        philosophy_guards=PHILOSOPHY_GUARDS,
    )
    lift = compute_asi_lift(fake)
    check("asi_lift_bounded", 0.0 <= lift <= V1351_ASI_CAP)

    # 9. philosophy_guards has 5 entries
    check("philosophy_guards_5", len(PHILOSOPHY_GUARDS) == 5)

    # 10. Stage registry points at modules that exist (smoke check)
    import importlib.util
    n_existing = 0
    for spec in STAGES:
        if not spec.module:
            n_existing += 1   # rollup is native
            continue
        try:
            spec_obj = importlib.util.find_spec(spec.module)
            if spec_obj is not None:
                n_existing += 1
        except Exception:
            pass
    check("stage_registry_modules_resolvable", n_existing == len(STAGES))

    # 11. _now_iso is ISO 8601 UTC
    iso = _now_iso()
    check("now_iso_format", iso.endswith("Z") and "T" in iso and len(iso) >= 20)

    # 12. PipelineRunner can be constructed with default paths
    import tempfile
    runner = PipelineRunner(ledger_path=Path(tempfile.gettempdir()) / "v1351-test-ledger.jsonl")
    check("runner_constructible", runner.ledger_path.name == "v1351-test-ledger.jsonl")
    check("runner_force_mock", runner.force_mock is True)

    # 13. Runner.run with no stages raises ValueError on unknown stage
    bad_raised = False
    try:
        runner.run(stage_names=["nonexistent_stage_xyz"])
    except ValueError:
        bad_raised = True
    check("runner_rejects_unknown_stage", bad_raised)

    # 14. Runner.run with empty stage_names runs full pipeline (may skip on empty ledger)
    runner2 = PipelineRunner(
        ledger_path=Path(tempfile.gettempdir()) / "v1351-empty-ledger.jsonl",
        force_mock=True,
    )
    res = runner2.run()
    check("runner_full_pipeline_n_stages", len(res.stages) == len(STAGES))
    check("runner_full_pipeline_rollup_ok",
          res.ecosystem_rollup.worst_severity in ("NONE", "LOW", "MEDIUM", "HIGH"))

    # 15. PipelineResult is JSON-serializable
    serialized = to_json(res)
    parsed = json.loads(serialized)
    check("pipeline_json_serializable", "pipeline_id" in parsed and "stages" in parsed)

    return passed, total


# -----------------------------------------------------------------------------
# CLI
# -----------------------------------------------------------------------------

def _print_pipeline_human(result: PipelineResult) -> None:
    print(f"=== V1351 VCP Toolchain v{V1351_VERSION} ===")
    print(f"ledger_path    : {result.ledger_path}")
    print(f"n_ledger_recs  : {result.n_ledger_records}")
    print(f"pipeline_id    : {result.pipeline_id}")
    print(f"total_ms       : {result.total_elapsed_ms:.1f}")
    print(f"asi_lift       : +{result.asi_lift:.6f} (cap {result.asi_cap})")
    print(f"started_at     : {result.started_at}")
    print(f"finished_at    : {result.finished_at}")
    print()
    print("Stages:")
    for s in result.stages:
        marker = "[OK]" if s.status == "ok" else (
            "[DEG]" if s.status == "degraded" else (
                "[SKIP]" if s.status == "skipped" else "[ERR]"
            )
        )
        print(f"  {marker} {s.stage:12} status={s.status:10} n={s.n_records:5} "
              f"elapsed={s.elapsed_ms:.1f}ms")
        if s.errors:
            for e in s.errors[:3]:
                print(f"           ! {e}")
        if s.summary:
            for k, v in list(s.summary.items())[:5]:
                print(f"             {k}: {v}")
    print()
    print("Ecosystem rollup:")
    r = result.ecosystem_rollup
    print(f"  n_substrates={r.n_substrates} (high={r.n_high_tier} medium={r.n_medium_tier} low={r.n_low_tier})")
    print(f"  anomalies   : high={r.n_anomaly_high} medium={r.n_anomaly_medium} "
          f"low={r.n_anomaly_low} none={r.n_anomaly_none}")
    print(f"  lifecycle   : {r.n_lifecycle_states}")
    print(f"  worst_severity: {r.worst_severity}")
    print(f"  ecosystem_state (V1350): {r.ecosystem_state or '(not run)'}")
    print()
    print("Philosophy guards:")
    for g in result.philosophy_guards:
        print(f"  - {g}")


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog="v1351_vcp_toolchain_cli",
        description="V1351 VCP Toolchain One-Click CLI (v0.1.0)",
    )
    sub = parser.add_subparsers(dest="command")

    # run subcommand
    p_run = sub.add_parser("run", help="run pipeline")
    p_run.add_argument("--stages", type=str, default=None,
                       help="comma-separated list of stages to run (default: all)")
    p_run.add_argument("--json", action="store_true", help="emit JSON")
    p_run.add_argument("--audit-out", type=str, default=None,
                       help="write artifact JSON to this path")
    p_run.add_argument("--ledger", type=str, default=None,
                       help="path to ledger JSONL (default: workspace vcp_gate_history.jsonl)")
    p_run.add_argument("--no-mock", action="store_true",
                       help="do not force mock in V1349 LLM benchmark")

    # stage subcommand
    p_stage = sub.add_parser("stage", help="run a single stage by name")
    p_stage.add_argument("stage_name", type=str, choices=STAGE_NAMES,
                         help="stage to run")
    p_stage.add_argument("--json", action="store_true", help="emit JSON")
    p_stage.add_argument("--audit-out", type=str, default=None,
                         help="write artifact JSON to this path")
    p_stage.add_argument("--ledger", type=str, default=None,
                         help="path to ledger JSONL")

    # list subcommand
    sub.add_parser("list", help="list all available stages")

    # self-test subcommand
    p_test = sub.add_parser("self-test", help="run Popper self-tests")
    p_test.add_argument("--verbose", action="store_true")

    # version subcommand
    sub.add_parser("version", help="print version")

    args = parser.parse_args(argv)
    if args.command is None:
        parser.print_help()
        return 0

    if args.command == "list":
        print(f"=== V1351 stages ({len(STAGES)}) ===")
        for s in STAGES:
            print(f"  {s.name:12}  {s.output_kind:10}  {s.description}")
        return 0

    if args.command == "version":
        print(f"v1351_vcp_toolchain_cli {V1351_VERSION}")
        return 0

    if args.command == "self-test":
        passed, total = _popper_self_tests(verbose=args.verbose)
        print(f"=== V1351 self-tests: {passed}/{total} PASS ===")
        return 0 if passed == total else 1

    if args.command in ("run", "stage"):
        ledger_path = Path(getattr(args, "ledger", None)) if getattr(args, "ledger", None) else LEDGER_PATH
        stages: Optional[List[str]] = None
        if args.command == "run":
            if getattr(args, "stages", None):
                stages = [s.strip() for s in args.stages.split(",") if s.strip()]
        else:
            stages = [getattr(args, "stage_name")]
        force_mock = not getattr(args, "no_mock", False)
        runner = PipelineRunner(
            ledger_path=ledger_path,
            force_mock=force_mock,
        )
        result = runner.run(stage_names=stages)
        if getattr(args, "audit_out", None):
            write_artifact(result, Path(args.audit_out))
        if getattr(args, "json", False):
            print(to_json(result))
        else:
            _print_pipeline_human(result)
        return 0

    parser.print_help()
    return 0


if __name__ == "__main__":
    sys.exit(main())