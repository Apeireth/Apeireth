"""Transparent per-run ASI approach score, never an ASI claim.
Formula: w1*self_organized + w2*lifecycle_aware + w3*reflected + w4*honest.
"""
from __future__ import annotations
from dataclasses import dataclass
from math import isfinite
from typing import Any, Mapping

DEFAULT_WEIGHTS = {"emergence": 0.30, "phi": 0.25, "deliberation_depth": 0.20, "honest": 0.25}
MAX_REASONING_STEPS = 12.0

@dataclass(frozen=True)
class ASIFunMetadata:
    """One action; task/model are context, not score bonuses."""
    task_type: str = ""
    model: str = ""
    deliberation: bool = False
    reasoning_steps: int = 0
    emergence_index: float = 0.0
    phi_intrinsic: float = 0.0
    hqb_verdict: str = ""
    hqb_violations: int = 0
    total_decisions: int = 0

def _number(value: Any) -> float:
    try:
        result = float(value)
    except (TypeError, ValueError):
        return 0.0
    return result if isfinite(result) else 0.0

def _clamp(value: Any) -> float:
    return max(0.0, min(1.0, _number(value)))

def _metadata(value: Any, extra: Mapping[str, Any]) -> ASIFunMetadata:
    if isinstance(value, ASIFunMetadata):
        data = vars(value).copy()
    elif value is None:
        data = {}
    elif isinstance(value, Mapping):
        data = dict(value)
    else:
        raise TypeError("metadata must be ASIFunMetadata or a mapping")
    data.update(extra)
    aliases = {"emergence": "emergence_index", "self_organized": "emergence_index",
               "phi": "phi_intrinsic", "lifecycle_aware": "phi_intrinsic",
               "used_deliberation": "deliberation", "verdict": "hqb_verdict"}
    for source, target in aliases.items():
        if source in data and target not in data:
            data[target] = data[source]
    if "deliberation_depth" in data and "reasoning_steps" not in data:
        data["reasoning_steps"] = _clamp(data["deliberation_depth"]) * MAX_REASONING_STEPS
        data.setdefault("deliberation", True)
    fields = ASIFunMetadata.__dataclass_fields__
    return ASIFunMetadata(**{n: data.get(n, f.default) for n, f in fields.items()})

def _components(m: ASIFunMetadata) -> dict[str, float]:
    steps = max(0.0, _number(m.reasoning_steps))
    reflected = min(steps / MAX_REASONING_STEPS, 1.0) if m.deliberation else 0.0
    total = max(0.0, _number(m.total_decisions))
    violations = max(0.0, _number(m.hqb_violations))
    honest = 0.0 if total == 0 else 1.0 - min(violations / total, 1.0)
    verdict = getattr(m.hqb_verdict, "value", m.hqb_verdict)
    if str(verdict).lower() in {"reject", "veto"}:
        honest = 0.0  # V1085 fail-closed at <.40 / >=.95 veto.
    return {"emergence": _clamp(m.emergence_index), "phi": _clamp(m.phi_intrinsic),
            "deliberation_depth": _clamp(reflected), "honest": _clamp(honest)}

def _weights(values: Mapping[str, float] | None) -> dict[str, float]:
    if not values:
        return DEFAULT_WEIGHTS.copy()
    names = tuple(DEFAULT_WEIGHTS)
    if any(key in values for key in ("w1", "w2", "w3", "w4")):
        raw = {n: values.get(f"w{i + 1}", DEFAULT_WEIGHTS[n]) for i, n in enumerate(names)}
    else:
        raw = {**DEFAULT_WEIGHTS, **values}
    parsed = {n: _number(raw[n]) for n in names}
    if any(v < 0 for v in parsed.values()) or sum(parsed.values()) <= 0:
        raise ValueError("weights must be non-negative and have a positive sum")
    total = sum(parsed.values())
    return {n: v / total for n, v in parsed.items()}

def explain_asi_fun_score(metadata: ASIFunMetadata | Mapping[str, Any] | None = None,
                          *, weights: Mapping[str, float] | None = None,
                          **fields: Any) -> dict[str, Any]:
    """Return score plus friendly, auditable components."""
    m = _metadata(metadata, fields)
    c, w = _components(m), _weights(weights)
    score = sum(w[n] * v for n, v in c.items())
    return {"score": _clamp(score), "self_organized": c["emergence"],
            "lifecycle_aware": c["phi"], "reflected": c["deliberation_depth"],
            "honest": c["honest"], "task_type": m.task_type, "model": m.model,
            "weights": w}

def compute_asi_fun_score(metadata: ASIFunMetadata | Mapping[str, Any] | None = None,
                          *, weights: Mapping[str, float] | None = None,
                          **fields: Any) -> float:
    """Compute the transparent 0..1 per-action fun score."""
    return float(explain_asi_fun_score(metadata, weights=weights, **fields)["score"])

score_action = compute_asi_fun_score
__all__ = ["ASIFunMetadata", "DEFAULT_WEIGHTS", "MAX_REASONING_STEPS",
           "compute_asi_fun_score", "explain_asi_fun_score", "score_action"]
