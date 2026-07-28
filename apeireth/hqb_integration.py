"""R6 HQB integration adapter for V1074/V1082/V1083.

This is a score gate, not an AI/ASI implementation. Target runners stay read-only.
"""
from __future__ import annotations

import uuid
from pathlib import Path
from typing import Any, Dict, Optional

from .hqb.schema import HqbStore
from .philosophy import check_philosophy
from .v1085_hqb_core import HonestDecisionModule, Verdict
from .v1086_hqb_persistence import HQBPersistence
from .v36_hqb_benchmark import HQBScore

ACCEPT_THRESHOLD = 0.70
REJECT_THRESHOLD = 0.40
VETO_THRESHOLD = 0.95


def _score(value: float) -> float:
    value = float(value)
    if not 0.0 <= value <= 1.0:
        raise ValueError("HQB score must be in [0.0, 1.0]")
    return value


class HQBReadOnlyVerifier:
    """Read-only view; no HQB write method is exposed."""

    def __init__(self, store: HqbStore):
        self._store = store

    def get(self, decision_id: str) -> Optional[Dict[str, Any]]:
        return self._store.get_decision(decision_id)

    def list(self, limit: int = 100):
        return self._store.list_decisions(limit)


class HQBIntegration:
    """Record measurement/router quality behind the independent HQB gate."""

    def __init__(self, db_path: str = ":memory:", artifact_dir: Optional[Path] = None):
        self.store = HqbStore(db_path)
        self.verifier = HQBReadOnlyVerifier(self.store)
        self.gate = HonestDecisionModule(ACCEPT_THRESHOLD, REJECT_THRESHOLD, VETO_THRESHOLD)
        self.persistence = HQBPersistence(artifact_dir=artifact_dir or Path("artifacts/v1086"))

    def _record(self, source: str, score: float, audit: bool = False,
                snapshot_score: Optional[float] = None) -> Dict[str, Any]:
        score = _score(score)
        decision = self.gate.evaluate(
            HQBScore(score_id=f"{source}_{uuid.uuid4().hex[:8]}", sc=score, nr=score, ev=score, cdt=score),
            context=source,
        )
        if audit:
            decision.verdict = (Verdict.ACCEPT if score >= VETO_THRESHOLD else
                                Verdict.REVIEW if score >= REJECT_THRESHOLD else Verdict.REJECT)
            decision.reason = f"audit_quality={score:.4f}; audit gate (not ASI claim)"
        before = score if snapshot_score is None else _score(snapshot_score)
        row_id = self.store.record_decision(source, decision.verdict.value, score, "PASS", before)
        self.store.record_guard(row_id, "hqb", decision.verdict != Verdict.VETO, decision.reason)
        self.store.record_delta(row_id, before, score)
        self.persistence.record(decision, asi_v03=score)
        return {"source": source, "decision_id": row_id, "verdict": decision.verdict.value,
                "score": score, "row": self.verifier.get(row_id)}

    def record_v1074(self, asi_v03: float, snapshot_score: Optional[float] = None):
        return self._record("v1074", asi_v03, snapshot_score=snapshot_score)

    def record_v1082(self, audit_quality: float, snapshot_score: Optional[float] = None):
        return self._record("v1082", audit_quality, audit=True, snapshot_score=snapshot_score)

    def record_v1083(self, decision_quality: float, snapshot_score: Optional[float] = None):
        return self._record("v1083", decision_quality, snapshot_score=snapshot_score)

    def close(self) -> None:
        self.store.close()


def guard_hqb_integration() -> Dict[str, Any]:
    check = check_philosophy(
        "hqb_integration",
        "HQB records bounded quality decisions; it is not AI or ASI.",
        claimed_pass=None,
        evidence={"adapter": "hqb", "no_asi_claim": True},
        categories=["bounded_gate", "no_asi_claim", "philosophy_referenced"],
        required_categories=["bounded_gate", "no_asi_claim", "philosophy_referenced"],
    )
    return {"module": "hqb_integration", "guard_status": check.status,
            "guard_passed": check.passed, "deviation_count": len(check.deviations)}


__all__ = ["HQBIntegration", "HQBReadOnlyVerifier", "guard_hqb_integration"]
