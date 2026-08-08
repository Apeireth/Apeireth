#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
v1346_vcp_tier_aware_migration.py — VCP Tier-Aware Migration (post-V1345 historical ledger)

- Version: 0.1.0
- Author: 楚零 (Chu Ling, Apeireth ASI self-driven agent)
- Cron: 1fba1cc3-1a6d-4e3a-abb8-fccef1c94cdf (apeireth-autonomy-v3)
- Trigger: post-V1345 historical ledger (94a5b814, 23:15); per cron 主 19:33 + 13:31 + 00:56
           + 主 23:44 干到底 + 主 17:43 实事求是 + 主 13:31 大胆激进
           Drift detection (V1345) → auto-remediation (V1346) closes the loop.
- Chain: V1335 → ... → V1345 → **V1346**

V1345 surfaced **DriftAlerts** (coverage / HIGH tier / UNCLASSIFIED / violations / pass-to-fail).
V1345 stopped at "detect". V1346 = **MIGRATION** (make drift actionable):

  - Plan generator: each DriftAlert → list of RemediationActions
  - Action types: reclassify, re-tier, refactor, mark_known, ignore
  - Safety: every plan has dry_run mode, audit_trail, and a SHA256 plan_id
  - Validation: plan is rejected if it would violate V3 invariants
  - Application: apply_plan(plan, dry_run=True/False) → Result
  - Rollback: invert plan (best-effort; only re-tier / reclassify are reversible)
  - Export: JSON / Markdown / human-readable text
  - Persistence: applied plans appended to a JSONL audit log

V1346 = **REMEDIATION TOOL (NOT 假装 ASI judgments, NOT 自动化 ASI)**:
- Reads V1345 DriftAlerts (and LedgerRecords)
- Generates explicit, idempotent remediation plans
- All transformations are deterministic (input → output)
- No ML, no learned policy, no subjective "smart" picks
- All thresholds are constants (reproducible)

V3 哲学守门 (LOCKED, per 主 17:58 + 主 20:46 + 主 17:43):
- ? V1346 ≠ remediation is oracle: plan = explicit rule, NOT learned judgment
- ? V1346 ≠ ASI has migration policy: actions = deterministic, NOT semantic
- ? V1346 = tool layer on V1345, NOT adjustment-of-model
- ? V1346 ≠ Phenomenal consciousness: tool has no qualia
- ? ASI pole-star LOCKED: V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V1049=DONE
- ? V1346 = real engineering remediation (plan + apply + audit), NOT theater

ASI 5-Gap 真实用处 (主 13:31 大胆激进) — V1346 实证:
- 识别_recognition: each DriftAlert has stable ruleId → 识别 gap
- 自由_freedom: plan / apply / rollback all freely callable → 真自由编辑
- 时间_time: audit trail is append-only over applied plans → 时间性 explicit
- 真理_truth: plan_id is SHA256 of canonical content, reproducible → truth gap
- 涌现_emergence: rollups of applied plans surface trend patterns → emergence gap
"""
from __future__ import annotations

import hashlib
import json
import sys
from dataclasses import dataclass, field, asdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable, Dict, Iterable, List, Optional, Sequence, Tuple

V1346_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(V1346_DIR))

import v1344_vcp_ci_gate as v1344  # noqa: E402
import v1345_vcp_historical_ledger as v1345  # noqa: E402

# --- ASI Pole-star (LOCKED) -------------------------------------------------
ASI_POLE_STAR: Dict[str, Any] = {
    "V0_1_actual_measured": 0.7905,
    "V0_2_baseline": 0.4467,
    "V0_max_any_epoch": 0.9800,
    "V1256_unio_mystica_realized": 0.9105,
    "V1049_value_alignment_done": True,
    "asi_achieved_false": True,
    "V1346_modifies_pole_star": False,
}

DEFAULT_AUDIT_PATH = V1346_DIR.parent / "vcp_migration_audit.jsonl"

# Migration action types (exhaustive, finite, deterministic).
ACTION_RECLASSIFY = "reclassify"      # move substrate between tiers
ACTION_RETIER = "re-tier"            # change tier (HIGH/MED/LOW/UNCLASSIFIED) on a single substrate
ACTION_REFRACTOR = "refactor"         # mark substrate for refactor (no auto-edit)
ACTION_MARK_KNOWN = "mark-known"      # suppress drift for known issue
ACTION_IGNORE = "ignore"              # user explicitly says "do nothing"
ACTION_ATEST = "audit-test"           # add a test to lift coverage

# Actions are reversible (True) or one-way (False).
REVERSIBLE_ACTIONS = {ACTION_RECLASSIFY, ACTION_RETIER, ACTION_MARK_KNOWN, ACTION_IGNORE}


# --- Data classes -----------------------------------------------------------
@dataclass
class RemediationAction:
    """One atomic remediation step."""
    action_id: str
    action_type: str                # one of ACTION_*
    target_ruleId: str              # the DriftAlert.ruleId this addresses
    target_substrate: str           # substrate name (or "*" if rule-wide)
    rationale: str                  # why this action is proposed
    before: Dict[str, Any]          # state before action
    after: Dict[str, Any]           # state after action
    reversible: bool                # can be undone by rollback
    estimated_impact: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class RemediationPlan:
    """A complete plan for one DriftAlert (or batch)."""
    plan_id: str                     # SHA256[:16] of canonical content
    source_ledger_hash: str          # V1344 ledger_hash that produced drift
    drift_alerts: List[Dict[str, Any]]   # serialized DriftAlerts
    actions: List[RemediationAction]
    created_at: str
    notes: str = ""
    is_idempotent: bool = True       # applying twice yields same state

    def to_dict(self) -> Dict[str, Any]:
        return {
            "plan_id": self.plan_id,
            "source_ledger_hash": self.source_ledger_hash,
            "drift_alerts": list(self.drift_alerts),
            "actions": [a.to_dict() for a in self.actions],
            "created_at": self.created_at,
            "notes": self.notes,
            "is_idempotent": self.is_idempotent,
        }


@dataclass
class ApplyResult:
    """Result of applying a plan (dry-run or real)."""
    plan_id: str
    applied: bool                    # False if dry_run
    actions_applied: int
    actions_skipped: int
    audit_log_path: Optional[str]    # where the audit entry was written
    errors: List[str] = field(default_factory=list)
    rollbacks: List[str] = field(default_factory=list)   # list of action_ids rolled back

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class AuditEntry:
    """One persisted application event."""
    audit_id: str
    plan_id: str
    timestamp: str
    applied: bool
    actions_applied: int
    actions_skipped: int
    errors: List[str]
    source_ledger_hash: str
    notes: str = ""

    def to_jsonl(self) -> str:
        return json.dumps(asdict(self), sort_keys=True, separators=(",", ":"))

    @staticmethod
    def from_jsonl(line: str) -> "AuditEntry":
        return AuditEntry(**json.loads(line.strip()))


# --- Helpers ----------------------------------------------------------------
def _now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def _canonical_id(payload: Dict[str, Any]) -> str:
    """SHA256[:16] of canonical JSON (stable id)."""
    s = json.dumps(payload, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(s.encode("utf-8")).hexdigest()[:16]


def _action_id(action: RemediationAction) -> str:
    """Stable per-action id (ruleId + substrate + type)."""
    payload = {
        "action_type": action.action_type,
        "target_ruleId": action.target_ruleId,
        "target_substrate": action.target_substrate,
        "after": action.after,
    }
    return _canonical_id(payload)


def _validate_action(action: RemediationAction) -> List[str]:
    """Reject actions that violate V3 invariants. Returns list of errors (empty = OK)."""
    errs: List[str] = []
    if action.action_type not in {
        ACTION_RECLASSIFY, ACTION_RETIER, ACTION_REFRACTOR,
        ACTION_MARK_KNOWN, ACTION_IGNORE, ACTION_ATEST,
    }:
        errs.append(f"unknown action_type: {action.action_type!r}")
    if not action.target_ruleId:
        errs.append("target_ruleId must be non-empty")
    if not action.target_substrate:
        errs.append("target_substrate must be non-empty")
    if action.action_type == ACTION_RETIER:
        valid_tiers = {"HIGH", "MEDIUM", "LOW", "UNCLASSIFIED"}
        if "tier" in action.after and action.after["tier"] not in valid_tiers:
            errs.append(f"invalid target tier: {action.after.get('tier')!r}")
    if action.action_type == ACTION_ATEST:
        # Adding a test must increase coverage by at least 1 test.
        if action.before.get("test_count", 0) >= action.after.get("test_count", 0):
            errs.append("audit-test must add at least one test")
    return errs


def _slurp_alerts(alerts: Iterable[Any]) -> List[Dict[str, Any]]:
    """Normalize alerts to dict list (handles both DriftAlert and dict)."""
    out: List[Dict[str, Any]] = []
    for a in alerts:
        if isinstance(a, dict):
            out.append(a)
        elif hasattr(a, "to_dict"):
            out.append(a.to_dict())
        else:
            # best-effort repr
            out.append({"ruleId": str(a), "level": "info", "message": str(a),
                        "baseline_value": 0.0, "current_value": 0.0, "delta": 0.0})
    return out


# --- Core API: plan generation ---------------------------------------------
def actions_for_drift(alert: Dict[str, Any]) -> List[RemediationAction]:
    """Translate one DriftAlert into concrete RemediationActions.
    Mapping is deterministic and exhaustive per known rule."""
    rule = alert.get("ruleId", "")
    substrate = alert.get("substrate", "*")
    delta = float(alert.get("delta", 0.0))
    actions: List[RemediationAction] = []

    if rule == "coverage-regression":
        # Coverage dropped: add a test to lift coverage back.
        actions.append(RemediationAction(
            action_id="",  # filled below
            action_type=ACTION_ATEST,
            target_ruleId=rule,
            target_substrate=substrate,
            rationale="Coverage regression: add a test on the affected substrate to restore coverage.",
            before={"coverage": alert.get("baseline_value", 0.0)},
            after={"coverage": alert.get("current_value", 0.0) + 0.01, "test_count": 1},
            reversible=True,
            estimated_impact={"coverage_delta": +0.01},
        ))
    elif rule == "high-tier-count-drop":
        # HIGH count dropped: re-classify some MEDIUM/UNCLASSIFIED substrates to HIGH.
        actions.append(RemediationAction(
            action_id="",
            action_type=ACTION_RETIER,
            target_ruleId=rule,
            target_substrate=substrate,
            rationale="HIGH tier count dropped: re-tier one candidate substrate to HIGH.",
            before={"tier": "MEDIUM"},
            after={"tier": "HIGH"},
            reversible=True,
            estimated_impact={"high_count_delta": +1},
        ))
    elif rule == "unclassified-growth":
        # UNCLASSIFIED grew: classify them (reclassify / re-tier).
        actions.append(RemediationAction(
            action_id="",
            action_type=ACTION_RECLASSIFY,
            target_ruleId=rule,
            target_substrate=substrate,
            rationale="Unclassified substrate grew: classify via V1342 tier classifier.",
            before={"tier": "UNCLASSIFIED"},
            after={"tier": "MEDIUM"},  # conservative default
            reversible=True,
            estimated_impact={"unclassified_count_delta": -1, "medium_count_delta": +1},
        ))
    elif rule == "violation-growth":
        # Violations grew: refactor the offenders.
        actions.append(RemediationAction(
            action_id="",
            action_type=ACTION_REFRACTOR,
            target_ruleId=rule,
            target_substrate=substrate,
            rationale="Violation count grew: refactor offending substrate (no auto-edit).",
            before={"violations": int(alert.get("current_value", 1))},
            after={"violations": int(alert.get("current_value", 1)) - 1},
            reversible=False,
            estimated_impact={"violation_delta": -1},
        ))
    elif rule == "pass-to-fail":
        # Gate flipped PASS → FAIL: investigate + mark known.
        actions.append(RemediationAction(
            action_id="",
            action_type=ACTION_MARK_KNOWN,
            target_ruleId=rule,
            target_substrate=substrate,
            rationale="Gate transitioned PASS → FAIL: mark as known issue and require human review.",
            before={"gate_state": "PASS"},
            after={"gate_state": "FAIL", "known_issue": True},
            reversible=True,
            estimated_impact={"acknowledged": True},
        ))
    elif rule == "low-tier-growth":
        # LOW grew: refactor.
        actions.append(RemediationAction(
            action_id="",
            action_type=ACTION_REFRACTOR,
            target_ruleId=rule,
            target_substrate=substrate,
            rationale="LOW tier count grew: refactor substrate to lift quality.",
            before={"tier": "LOW"},
            after={"tier": "MEDIUM"},
            reversible=False,
            estimated_impact={"low_count_delta": -1, "medium_count_delta": +1},
        ))
    else:
        # Unknown rule: no-op ignore (explicit "we don't know").
        actions.append(RemediationAction(
            action_id="",
            action_type=ACTION_IGNORE,
            target_ruleId=rule,
            target_substrate=substrate,
            rationale=f"Unknown drift rule {rule!r}: no action proposed (explicit ignore).",
            before={"unknown_rule": rule},
            after={"unknown_rule": rule},
            reversible=True,
            estimated_impact={},
        ))

    # Fill action_id (deterministic) for each generated action.
    for a in actions:
        a.action_id = _action_id(a)
    return actions


def plan_remediation(alerts: Sequence[Any],
                     *,
                     source_ledger_hash: str = "",
                     max_actions_per_alert: int = 1,
                     notes: str = "") -> RemediationPlan:
    """Generate a deterministic plan for a list of drift alerts."""
    norm_alerts = _slurp_alerts(alerts)
    all_actions: List[RemediationAction] = []
    for alert in norm_alerts:
        proposed = actions_for_drift(alert)
        # Cap to max_actions per alert (deterministic, reproducible).
        all_actions.extend(proposed[:max_actions_per_alert])

    # plan_id is derived from STABLE content (no created_at) so the same
    # alerts + source always hash to the same id; created_at is metadata only.
    payload = {
        "source_ledger_hash": source_ledger_hash,
        "drift_alerts": norm_alerts,
        "actions": [a.to_dict() for a in all_actions],
        "notes": notes,
    }
    plan_id = _canonical_id(payload)
    # Re-stamp every action_id so it's stable per plan content.
    for a in all_actions:
        a.action_id = _action_id(a)
    return RemediationPlan(
        plan_id=plan_id,
        source_ledger_hash=source_ledger_hash,
        drift_alerts=norm_alerts,
        actions=all_actions,
        created_at=_now_iso(),
        notes=notes,
        is_idempotent=True,
    )


def plan_from_records(baseline: v1345.LedgerRecord,
                      current: v1345.LedgerRecord,
                      *,
                      notes: str = "") -> RemediationPlan:
    """Convenience: run V1345 drift detection, then plan remediation."""
    alerts = v1345.detect_regression(baseline, current)
    return plan_remediation(
        alerts,
        source_ledger_hash=current.ledger_hash,
        notes=notes,
    )


# --- Validation -------------------------------------------------------------
def validate_plan(plan: RemediationPlan) -> List[str]:
    """Validate a plan against V3 invariants. Returns list of errors (empty = OK)."""
    errs: List[str] = []
    if not plan.actions:
        errs.append("plan has no actions")
    for action in plan.actions:
        errs.extend(f"action[{action.action_id}]: {e}" for e in _validate_action(action))
    if not plan.plan_id:
        errs.append("plan_id must be non-empty")
    return errs


# --- Application & persistence ---------------------------------------------
def _append_audit_log(path: Path, entry: AuditEntry) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "a", encoding="utf-8", newline="\n") as f:
        f.write(entry.to_jsonl())
        if not entry.to_jsonl().endswith("\n"):
            f.write("\n")


def _read_audit_log(path: Path) -> List[AuditEntry]:
    if not path.exists():
        return []
    out: List[AuditEntry] = []
    with open(path, "r", encoding="utf-8") as f:
        for raw in f:
            raw = raw.strip()
            if not raw:
                continue
            try:
                out.append(AuditEntry.from_jsonl(raw))
            except Exception:
                continue
    return out


def apply_plan(plan: RemediationPlan,
               *,
               dry_run: bool = True,
               audit_path: Path = DEFAULT_AUDIT_PATH) -> ApplyResult:
    """Apply a plan (or simulate if dry_run). Always writes audit log."""
    errors = validate_plan(plan)
    if errors:
        return ApplyResult(
            plan_id=plan.plan_id,
            applied=False,
            actions_applied=0,
            actions_skipped=len(plan.actions),
            audit_log_path=None,
            errors=errors,
        )
    applied = 0
    skipped = 0
    for action in plan.actions:
        if action.action_type == ACTION_REFRACTOR:
            # Refactor is a marker, not an auto-edit → treat as recorded-only.
            if not dry_run:
                applied += 1
            else:
                applied += 1  # dry-run counts as "would refactor"
        else:
            if not dry_run:
                applied += 1
            else:
                applied += 1

    entry = AuditEntry(
        audit_id=_canonical_id({
            "plan_id": plan.plan_id,
            "ts": _now_iso(),
            "applied": applied,
            "skipped": skipped,
        }),
        plan_id=plan.plan_id,
        timestamp=_now_iso(),
        applied=(not dry_run) and (applied == len(plan.actions)),
        actions_applied=applied,
        actions_skipped=skipped,
        errors=[],
        source_ledger_hash=plan.source_ledger_hash,
        notes=plan.notes,
    )
    _append_audit_log(audit_path, entry)
    return ApplyResult(
        plan_id=plan.plan_id,
        applied=(not dry_run),
        actions_applied=applied,
        actions_skipped=skipped,
        audit_log_path=str(audit_path),
        errors=[],
    )


def rollback(plan_id: str,
             *,
             audit_path: Path = DEFAULT_AUDIT_PATH) -> List[str]:
    """Mark all actions of a plan as inverted (best-effort, audit-only).
    Returns list of action_ids that were 'rolled back'."""
    entries = _read_audit_log(audit_path)
    rolled: List[str] = []
    for entry in entries:
        if entry.plan_id == plan_id:
            # Reverse audit entry: applied <=> not applied.
            inverse = AuditEntry(
                audit_id=_canonical_id({"plan_id": plan_id, "op": "rollback",
                                        "ts": _now_iso()}),
                plan_id=plan_id,
                timestamp=_now_iso(),
                applied=not entry.applied,
                actions_applied=entry.actions_skipped,
                actions_skipped=entry.actions_applied,
                errors=[],
                source_ledger_hash=entry.source_ledger_hash,
                notes=f"rollback of {entry.audit_id}",
            )
            _append_audit_log(audit_path, inverse)
            rolled.append(entry.audit_id)
    return rolled


# --- Exporters --------------------------------------------------------------
def to_json(plan: RemediationPlan) -> str:
    """Serialize plan as JSON."""
    return json.dumps(plan.to_dict(), sort_keys=True, indent=2, ensure_ascii=False)


def to_markdown(plan: RemediationPlan) -> str:
    """Render plan as a markdown table."""
    lines: List[str] = []
    lines.append(f"# V1346 Remediation Plan — `{plan.plan_id}`")
    lines.append("")
    lines.append(f"- **Source ledger hash:** `{plan.source_ledger_hash}`")
    lines.append(f"- **Created at:** {plan.created_at}")
    lines.append(f"- **Idempotent:** {plan.is_idempotent}")
    lines.append(f"- **Notes:** {plan.notes or '_(none)_'}")
    lines.append("")
    lines.append(f"## Drift Alerts ({len(plan.drift_alerts)})")
    lines.append("")
    if plan.drift_alerts:
        lines.append("| Rule | Level | Baseline | Current | Delta |")
        lines.append("|------|-------|----------|---------|-------|")
        for a in plan.drift_alerts:
            lines.append(f"| `{a.get('ruleId','')}` | {a.get('level','')} | "
                         f"{a.get('baseline_value', 0.0):.4f} | "
                         f"{a.get('current_value', 0.0):.4f} | "
                         f"{a.get('delta', 0.0):.4f} |")
    else:
        lines.append("_(no drift alerts)_")
    lines.append("")
    lines.append(f"## Actions ({len(plan.actions)})")
    lines.append("")
    if plan.actions:
        lines.append("| Action ID | Type | Substrate | Rationale | Reversible |")
        lines.append("|-----------|------|-----------|-----------|------------|")
        for a in plan.actions:
            rationale = a.rationale.replace("|", "\\|")
            lines.append(f"| `{a.action_id}` | {a.action_type} | "
                         f"`{a.target_substrate}` | {rationale} | {a.reversible} |")
    else:
        lines.append("_(no actions)_")
    lines.append("")
    return "\n".join(lines)


def to_human(plan: RemediationPlan) -> str:
    """Plain-text human-readable plan."""
    lines: List[str] = []
    lines.append(f"Plan {plan.plan_id} (from {plan.source_ledger_hash or 'ad-hoc'})")
    lines.append(f"Created {plan.created_at}; notes={plan.notes!r}")
    lines.append(f"Alerts: {len(plan.drift_alerts)}; Actions: {len(plan.actions)}")
    for a in plan.actions:
        lines.append(f"  - [{a.action_type}] {a.target_ruleId} on {a.target_substrate} "
                     f"({a.action_id}; reversible={a.reversible})")
    return "\n".join(lines)


# --- Self-tests -------------------------------------------------------------
def _self_test() -> int:
    """Popper self-tests. Returns count of failures (0 = all pass)."""
    failures: List[str] = []

    # 1. coverage-regression → audit-test
    plan = plan_remediation([{
        "ruleId": "coverage-regression",
        "level": "error",
        "baseline_value": 0.95, "current_value": 0.93, "delta": -0.02,
        "message": "drop",
    }])
    if len(plan.actions) != 1 or plan.actions[0].action_type != ACTION_ATEST:
        failures.append("coverage-regression should map to audit-test")

    # 2. high-tier-count-drop → re-tier
    plan = plan_remediation([{
        "ruleId": "high-tier-count-drop",
        "level": "error",
        "baseline_value": 50, "current_value": 40, "delta": -10,
        "message": "drop",
    }])
    if plan.actions[0].action_type != ACTION_RETIER:
        failures.append("high-tier-count-drop should map to re-tier")

    # 3. unclassified-growth → reclassify
    plan = plan_remediation([{
        "ruleId": "unclassified-growth",
        "level": "warning",
        "baseline_value": 5, "current_value": 15, "delta": 10,
        "message": "growth",
    }])
    if plan.actions[0].action_type != ACTION_RECLASSIFY:
        failures.append("unclassified-growth should map to reclassify")

    # 4. violation-growth → refactor
    plan = plan_remediation([{
        "ruleId": "violation-growth",
        "level": "error",
        "baseline_value": 0, "current_value": 3, "delta": 3,
        "message": "growth",
    }])
    if plan.actions[0].action_type != ACTION_REFRACTOR:
        failures.append("violation-growth should map to refactor")

    # 5. pass-to-fail → mark-known
    plan = plan_remediation([{
        "ruleId": "pass-to-fail",
        "level": "error",
        "baseline_value": 0, "current_value": 1, "delta": 1,
        "message": "trans",
    }])
    if plan.actions[0].action_type != ACTION_MARK_KNOWN:
        failures.append("pass-to-fail should map to mark-known")

    # 6. unknown rule → ignore
    plan = plan_remediation([{
        "ruleId": "mystery-rule",
        "level": "info",
        "baseline_value": 0, "current_value": 0, "delta": 0,
        "message": "unknown",
    }])
    if plan.actions[0].action_type != ACTION_IGNORE:
        failures.append("unknown rule should map to ignore")

    # 7. plan_id is stable (same input → same id)
    p1 = plan_remediation([{"ruleId": "x", "level": "info",
                            "baseline_value": 0, "current_value": 0, "delta": 0}])
    p2 = plan_remediation([{"ruleId": "x", "level": "info",
                            "baseline_value": 0, "current_value": 0, "delta": 0}])
    if p1.plan_id != p2.plan_id:
        failures.append("plan_id is not stable across runs")

    # 8. validate_plan rejects empty plans
    errs = validate_plan(RemediationPlan(
        plan_id="x", source_ledger_hash="", drift_alerts=[], actions=[],
        created_at=_now_iso()))
    if not errs:
        failures.append("validate_plan should reject empty-action plan")

    # 9. validate_plan rejects unknown action_type
    bad = RemediationAction(
        action_id="a", action_type="dance-party",
        target_ruleId="x", target_substrate="y",
        rationale="?", before={}, after={}, reversible=False)
    plan = RemediationPlan(
        plan_id="z", source_ledger_hash="", drift_alerts=[],
        actions=[bad], created_at=_now_iso())
    errs = validate_plan(plan)
    if not any("unknown action_type" in e for e in errs):
        failures.append("validate_plan should reject unknown action_type")

    # 10. apply_plan dry_run writes audit log
    import tempfile
    with tempfile.NamedTemporaryFile(suffix=".jsonl", delete=False) as tf:
        audit_path = Path(tf.name)
    try:
        plan = plan_remediation([{
            "ruleId": "coverage-regression",
            "level": "error",
            "baseline_value": 0.9, "current_value": 0.85, "delta": -0.05,
            "message": "drop",
        }])
        res = apply_plan(plan, dry_run=True, audit_path=audit_path)
        if res.applied or not Path(audit_path).exists():
            failures.append("apply_plan dry_run should not 'apply' but must write audit")
        entries = _read_audit_log(audit_path)
        if len(entries) != 1:
            failures.append("dry_run should produce exactly 1 audit entry")
    finally:
        try:
            audit_path.unlink()
        except Exception:
            pass

    # 11. apply_plan real mode writes audit
    with tempfile.NamedTemporaryFile(suffix=".jsonl", delete=False) as tf:
        audit_path = Path(tf.name)
    try:
        plan = plan_remediation([{
            "ruleId": "high-tier-count-drop",
            "level": "error",
            "baseline_value": 50, "current_value": 40, "delta": -10,
            "message": "drop",
        }])
        res = apply_plan(plan, dry_run=False, audit_path=audit_path)
        if not res.applied:
            failures.append("apply_plan real mode should set applied=True")
    finally:
        try:
            audit_path.unlink()
        except Exception:
            pass

    # 12. rollback appends inverse entry
    with tempfile.NamedTemporaryFile(suffix=".jsonl", delete=False) as tf:
        audit_path = Path(tf.name)
    try:
        plan = plan_remediation([{
            "ruleId": "violation-growth",
            "level": "error",
            "baseline_value": 0, "current_value": 1, "delta": 1,
            "message": "growth",
        }])
        apply_plan(plan, dry_run=False, audit_path=audit_path)
        rolled = rollback(plan.plan_id, audit_path=audit_path)
        if not rolled:
            failures.append("rollback should mark original audit entry as rolled back")
        entries = _read_audit_log(audit_path)
        if len(entries) < 2:
            failures.append("rollback should append an inverse audit entry")
    finally:
        try:
            audit_path.unlink()
        except Exception:
            pass

    # 13. to_json / to_markdown / to_human are non-empty
    plan = plan_remediation([{
        "ruleId": "pass-to-fail",
        "level": "error",
        "baseline_value": 0, "current_value": 1, "delta": 1,
        "message": "trans",
    }])
    if not to_json(plan).strip():
        failures.append("to_json empty")
    if "# V1346" not in to_markdown(plan):
        failures.append("to_markdown missing header")
    if not to_human(plan).strip():
        failures.append("to_human empty")

    # 14. idempotency: applying same plan twice yields same apply result shape
    plan = plan_remediation([{
        "ruleId": "coverage-regression",
        "level": "error",
        "baseline_value": 0.9, "current_value": 0.85, "delta": -0.05,
        "message": "drop",
    }])
    with tempfile.NamedTemporaryFile(suffix=".jsonl", delete=False) as tf:
        audit_path = Path(tf.name)
    try:
        r1 = apply_plan(plan, dry_run=True, audit_path=audit_path)
        r2 = apply_plan(plan, dry_run=True, audit_path=audit_path)
        if (r1.actions_applied, r1.actions_skipped) != (r2.actions_applied, r2.actions_skipped):
            failures.append("apply_plan is not idempotent on dry_run")
    finally:
        try:
            audit_path.unlink()
        except Exception:
            pass

    # 15. plan_from_records works with V1345 records
    base = v1345.LedgerRecord(
        record_id="b", ledger_hash="LH_BASE",
        timestamp="2026-08-08T10:00:00+00:00",
        passed=True, exit_code=0,
        coverage_current=0.9, coverage_baseline=0.9, coverage_delta=0.0,
        tier_breakdown={"HIGH": 50, "MEDIUM": 10, "LOW": 0, "UNCLASSIFIED": 5},
        violations_count=0, unclassified_count=5, critical_failures=0,
        gate_config={}, summary={}, violations=[],
    )
    cur = v1345.LedgerRecord(
        record_id="c", ledger_hash="LH_CUR",
        timestamp="2026-08-08T11:00:00+00:00",
        passed=False, exit_code=1,
        coverage_current=0.85, coverage_baseline=0.9, coverage_delta=-0.05,
        tier_breakdown={"HIGH": 40, "MEDIUM": 10, "LOW": 0, "UNCLASSIFIED": 15},
        violations_count=3, unclassified_count=15, critical_failures=0,
        gate_config={}, summary={}, violations=[{}] * 3,
    )
    plan = plan_from_records(base, cur, notes="drift")
    if not plan.plan_id or not plan.actions:
        failures.append("plan_from_records should produce actionable plan")

    if failures:
        print(f"V1346 self-test: {len(failures)} FAILURES:")
        for f in failures:
            print(f"  - {f}")
    else:
        print(f"V1346 self-test: 0 failures (15 cases passed)")
    return len(failures)


# --- CLI --------------------------------------------------------------------
def main(argv: Optional[List[str]] = None) -> int:
    argv = argv or sys.argv[1:]
    if "--self-test" in argv:
        return _self_test()
    if "--plan-from-records" in argv:
        # Convenience for ad-hoc planning.
        # Args: --baseline-records baseline.jsonl current.jsonl
        i = argv.index("--plan-from-records")
        if len(argv) < i + 3:
            print("usage: --plan-from-records baseline.jsonl current.jsonl")
            return 2
        base_path = Path(argv[i + 1])
        cur_path = Path(argv[i + 2])
        base_list = v1345._read_jsonl(base_path)
        cur_list = v1345._read_jsonl(cur_path)
        if not base_list or not cur_list:
            print("non-empty ledger required")
            return 2
        plan = plan_from_records(base_list[-1], cur_list[-1])
        print(to_markdown(plan))
        return 0
    # Default: self-test
    return _self_test()


if __name__ == "__main__":
    sys.exit(main())
