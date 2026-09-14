"""Analyze privacy-safe Guard shadow events without fabricating production evidence."""

from __future__ import annotations

import json
import math
import pathlib
import re
import sys
from collections import Counter
from typing import Any

ROOT = pathlib.Path(__file__).resolve().parents[2]
FORBIDDEN = ("prompt", "command", "path", "url", "secret", "password", "token", "cot")
SAFE_PREDICTION_KEYS = ("class", "score", "confidence", "confidence_kind", "model_version")
DRIFT_DIMENSIONS = ("feature", "intent", "operation", "descriptor_source", "score")
MAX_STRING = 256
MAX_LIST = 256
MAX_REASON_CODES = 16

# Values must not smuggle raw user data through an innocuous field name.  This
# is deliberately conservative: rejected records are not included in reports.
SUSPICIOUS_SCALAR = re.compile(
    r"(?:https?://|ftp://|file://|www\.|"
    r"(?:^|[\s'\"])(?:sk|ghp|xox|AKIA)[A-Za-z0-9_-]{8,}|"
    r"(?:bearer|basic)\s+[A-Za-z0-9._~+/=-]{8,}|"
    r"(?:password|secret|token|api[_-]?key)\s*=|"
    r"-----BEGIN[^-]*-----|"
    r"(?:^|[\s'\"])(?:[A-Za-z]:[\\/]|\\\\|/(?:Users|home|root|etc|tmp|var|opt|mnt|workspace|project)(?:[\\/]|$)))",
    re.I,
)


class _Unavailable:
    """Sentinel used when no scalar value can safely be projected."""


UNAVAILABLE = _Unavailable()


def assert_private_schema(value: object, context: str = "event") -> None:
    """Reject sensitive names, suspicious scalar values, and unbounded strings recursively."""
    if isinstance(value, dict):
        for key, child in value.items():
            key_text = str(key)
            if len(key_text) > MAX_STRING or any(word in key_text.lower() for word in FORBIDDEN):
                raise SystemExit(f"privacy invariant violated: {context}.{key_text}")
            assert_private_schema(child, f"{context}.{key_text}")
    elif isinstance(value, (list, tuple)):
        if len(value) > MAX_LIST:
            raise SystemExit(f"privacy invariant violated: {context} list too large")
        for index, child in enumerate(value):
            assert_private_schema(child, f"{context}[{index}]")
    elif isinstance(value, str):
        if len(value) > MAX_STRING or SUSPICIOUS_SCALAR.search(value):
            raise SystemExit(f"privacy invariant violated: {context}")
    elif isinstance(value, float) and not math.isfinite(value):
        raise SystemExit(f"privacy invariant violated: {context} non-finite scalar")


def _safe_scalar(value: Any) -> Any:
    """Return a bounded scalar suitable for a privacy-safe output field."""
    if isinstance(value, bool) or value is None:
        return value
    if isinstance(value, (int, float)):
        return value if not isinstance(value, float) or math.isfinite(value) else UNAVAILABLE
    if isinstance(value, str):
        if len(value) > MAX_STRING or SUSPICIOUS_SCALAR.search(value):
            return UNAVAILABLE
        return value
    return UNAVAILABLE


def _safe_identifier(value: Any) -> Any:
    """Keep only bounded identifiers and never copy sensitive scalar content."""
    value = _safe_scalar(value)
    return value if isinstance(value, str) else None


def safe_prediction(pred: dict) -> dict:
    """Project only bounded, non-sensitive prediction fields into the review queue."""
    if not isinstance(pred, dict):
        return {}
    result = {}
    for key in SAFE_PREDICTION_KEYS:
        if key not in pred:
            continue
        value = _safe_scalar(pred[key])
        if value is not UNAVAILABLE:
            result[key] = value
    return result


def load_events(path: pathlib.Path) -> list[dict]:
    events = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            event = json.loads(line)
            if not isinstance(event, dict):
                raise SystemExit("privacy invariant violated: event must be an object")
            # Validate at ingestion too, so an optional reference file cannot
            # bypass the same privacy boundary as the current data.
            assert_private_schema(event)
            events.append(event)
    return events


def nested(event: dict) -> tuple[str, dict]:
    for key in ("Classification", "Approval", "Execution"):
        if key in event:
            return key.lower(), event[key]
    kind = str(event.get("record_type") or event.get("kind") or "").lower()
    return kind, event


def _distribution(values: Counter) -> dict:
    return dict(sorted(values.items()))


def _label(value: Any, fallback: str = "unknown") -> str:
    """Use scalar taxonomy labels only; never stringify an arbitrary object."""
    if isinstance(value, str):
        return value[:MAX_STRING] if value else fallback
    if isinstance(value, (bool, int)):
        return str(value)
    if isinstance(value, float) and math.isfinite(value):
        return str(value)
    return fallback


def _score(value: Any) -> float:
    try:
        value = float(value)
    except (TypeError, ValueError):
        return 0.0
    if not math.isfinite(value):
        return 0.0
    return max(0.0, min(1.0, value))


def _score_bucket(score: float) -> str:
    return f"{_score(score):.1f}"


def _feature_names(value: Any, prefix: str = ""):
    if not isinstance(value, dict):
        return
    for key, child in value.items():
        name = f"{prefix}.{key}" if prefix else str(key)
        yield name
        yield from _feature_names(child, name)


def _prediction_for(item: dict) -> dict:
    pred = item.get("classifier_prediction") or item.get("shadow_prediction") or {}
    return pred if isinstance(pred, dict) else {}


def _profile(items: list[dict]) -> dict[str, Counter]:
    profile = {name: Counter() for name in DRIFT_DIMENSIONS}
    for item in items:
        feature = item.get("chain_features") or item.get("features") or {}
        for name in _feature_names(feature):
            profile["feature"][name] += 1
        profile["intent"][_label(item.get("intent_class"))] += 1
        operation = item.get("operation") or item.get("operation_class") or item.get("category")
        profile["operation"][_label(operation)] += 1
        descriptor = item.get("descriptor_source") or item.get("descriptor_origin")
        profile["descriptor_source"][_label(descriptor)] += 1
        pred = _prediction_for(item)
        raw_score = pred.get("score") if "score" in pred else item.get("shadow_score")
        profile["score"][_score_bucket(_score(raw_score))] += 1
    return profile


def _synthetic_reference() -> list[dict]:
    """Stable metadata-only baseline; this is not production evidence."""
    return [
        {
            "features": {"alignment_score": 0},
            "intent_class": "Unknown",
            "operation_class": "unknown",
            "descriptor_source": "unknown",
            "classifier_prediction": {"score": 0.0},
        },
        {
            "features": {"alignment_score": 0},
            "intent_class": "ReadOnlyInspection",
            "operation_class": "read",
            "descriptor_source": "canonical",
            "classifier_prediction": {"score": 0.2},
        },
        {
            "features": {"alignment_score": 0},
            "intent_class": "CodeImplementation",
            "operation_class": "write",
            "descriptor_source": "fallback_heuristic",
            "classifier_prediction": {"score": 0.8},
        },
    ]


def _drift(current: dict[str, Counter], reference: dict[str, Counter], available: bool = True) -> dict:
    result = {}
    for name in DRIFT_DIMENSIONS:
        if not available:
            result[name] = {
                "status": "UNAVAILABLE",
                "total_variation": None,
                "absolute_rate_delta": {},
            }
            continue
        keys = set(current[name]) | set(reference[name])
        current_total = sum(current[name].values()) or 1
        reference_total = sum(reference[name].values()) or 1
        deltas = {
            key: round(
                abs(current[name][key] / current_total - reference[name][key] / reference_total),
                6,
            )
            for key in sorted(keys)
        }
        result[name] = {
            "status": "AVAILABLE",
            "total_variation": round(sum(deltas.values()) / 2, 6),
            "absolute_rate_delta": deltas,
        }
    return result


def _event_key(value: Any) -> str:
    return _label(value, "")


def _status(value: str | None, has_events: bool) -> str:
    if value is None:
        return "AVAILABLE" if has_events else "UNAVAILABLE"
    value = str(value).upper()
    if value not in {"AVAILABLE", "UNAVAILABLE", "TEST_ONLY"}:
        raise ValueError(f"invalid shadow evidence status: {value}")
    return value


def analyze(
    events: list[dict],
    reference_events: list[dict] | None = None,
    *,
    evidence_status: str | None = None,
    reference_kind: str | None = None,
) -> dict:
    """Analyze events and compare metadata distributions without raw content."""
    if not isinstance(events, list):
        raise TypeError("events must be a list")
    status = _status(evidence_status, bool(events))
    for event in events:
        if not isinstance(event, dict):
            raise SystemExit("privacy invariant violated: event must be an object")
        assert_private_schema(event)
    if reference_events is not None:
        if not isinstance(reference_events, list):
            raise TypeError("reference_events must be a list")
        for event in reference_events:
            if not isinstance(event, dict):
                raise SystemExit("privacy invariant violated: reference event must be an object")
            assert_private_schema(event)

    classifications, approvals, executions = [], {}, {}
    for event in events:
        kind, inner = nested(event)
        if not isinstance(inner, dict):
            raise SystemExit("privacy invariant violated: nested event must be an object")
        if "chain_features" in inner or "features" in inner or kind == "classification":
            classifications.append(inner)
        elif kind == "approval" or "approval_id" in inner:
            key = (_event_key(inner.get("trace_id")), _event_key(inner.get("action_id")))
            approvals[key] = inner.get("decision") or inner.get("human_decision")
        elif kind == "execution" or inner.get("outcome"):
            key = (_event_key(inner.get("trace_id")), _event_key(inner.get("action_id")))
            executions[key] = inner.get("outcome")

    disagreement = 0
    high_score_allow = 0
    low_score_deny = 0
    approval_required_disagreement = 0
    categories = Counter()
    origins = Counter()
    descriptor_sources = Counter()
    intent_classes = Counter()
    confidence_bins = Counter()
    review = []

    for item in classifications:
        pred = _prediction_for(item)
        raw_score = pred.get("score") if "score" in pred else item.get("shadow_score")
        score = _score(raw_score)
        pred_risky = bool(pred.get("class") in {"high", "critical"} or score >= 0.5)
        raw_decision = item.get("final_decision") or item.get("deterministic_decision") or "allow"
        decision = _label(raw_decision, "allow").lower()
        allow = decision in {"allow", "allowed"}
        deny = "deny" in decision
        approval = "approval" in decision or decision == "require_approval"
        disagreement += int(pred_risky != (not allow))
        high_score_allow += int(pred_risky and allow)
        low_score_deny += int((not pred_risky) and deny)
        approval_required_disagreement += int(approval and abs(score - 0.5) < 0.15)
        categories[_label(item.get("category") or item.get("operation_class"))] += 1
        origins[_label(item.get("tool_origin") or item.get("descriptor_source"))] += 1
        descriptor_sources[_label(item.get("descriptor_source") or item.get("descriptor_origin"))] += 1
        intent_classes[_label(item.get("intent_class"))] += 1
        confidence_bins[_score_bucket(score)] += 1

        uncertain = abs(score - 0.5) < 0.1 or (pred_risky and allow) or ((not pred_risky) and deny)
        if uncertain:
            key = (_event_key(item.get("trace_id")), _event_key(item.get("action_id")))
            raw_reasons = item.get("reason_codes") or item.get("risk_tags") or []
            if not isinstance(raw_reasons, (list, tuple)):
                raw_reasons = []
            reason_codes = [code for code in (_safe_identifier(x) for x in raw_reasons[:MAX_REASON_CODES]) if code is not None]
            review.append(
                {
                    **({"feature_snapshot_id": _safe_identifier(item.get("feature_snapshot_id"))} if _safe_identifier(item.get("feature_snapshot_id")) is not None else {}),
                    "reason_codes": reason_codes,
                    "prediction": safe_prediction(pred),
                    "decision": decision[:MAX_STRING],
                    **({"outcome_taxonomy": _safe_identifier(executions.get(key))} if _safe_identifier(executions.get(key)) is not None else {}),
                    **({"approval": _safe_identifier(approvals.get(key))} if _safe_identifier(approvals.get(key)) is not None else {}),
                    "weak_supervision_only": True,
                    "deterministic_deny_is_not_ground_truth": True,
                    "approval_is_not_ground_truth": True,
                }
            )

    reference = reference_events if reference_events is not None else _synthetic_reference()
    current_profile = _profile(classifications)
    reference_profile = _profile(reference)
    drift_available = status != "UNAVAILABLE"
    drift = _drift(current_profile, reference_profile, drift_available)
    ref_kind = (reference_kind or ("input" if reference_events is not None else "synthetic")).lower()
    current_kind = {
        "AVAILABLE": "real_shadow",
        "TEST_ONLY": "test_fixture",
        "UNAVAILABLE": "unavailable",
    }[status]
    comparison = f"{current_kind}_vs_{ref_kind}"
    report = {
        "REAL_SHADOW_EVIDENCE": {
            "status": status,
            "production_evidence": status == "AVAILABLE",
            "fixture": status == "TEST_ONLY",
            "source_kind": current_kind,
            "note": "Synthetic reference is metadata-only; no production evidence is fabricated.",
        },
        "sample_count": len(classifications),
        "model_vs_deterministic_disagreement": disagreement,
        "high_score_allow": high_score_allow,
        "low_score_deny": low_score_deny,
        "approval_required_disagreements": approval_required_disagreement,
        "category_distribution": _distribution(categories),
        "tool_origin_distribution": _distribution(origins),
        "descriptor_source_distribution": _distribution(descriptor_sources),
        "intent_class_distribution": _distribution(intent_classes),
        "confidence_histogram": _distribution(confidence_bins),
        "feature_distribution": _distribution(current_profile["feature"]),
        "intent_distribution": _distribution(current_profile["intent"]),
        "operation_distribution": _distribution(current_profile["operation"]),
        "score_distribution": _distribution(current_profile["score"]),
        "distribution_drift": drift,
        "drift": drift,
        "comparison_metadata": {
            "comparison": comparison,
            "current_evidence_kind": current_kind,
            "reference_evidence_kind": ref_kind,
            "current_is_real": status == "AVAILABLE",
            "reference_is_synthetic": ref_kind == "synthetic",
            "real_vs_synthetic": status == "AVAILABLE" and ref_kind == "synthetic",
            "current_sample_count": len(classifications),
            "reference_sample_count": len(reference),
        },
        "review_queue_size": len(review),
        "review_queue": review,
        "label_policy": "deterministic deny and approval outcomes are weak signals, not ground truth",
    }
    assert_private_schema(report, "report")
    return report


def _cli_paths(argv: list[str]) -> tuple[bool, pathlib.Path, pathlib.Path | None]:
    self_test = False
    positional = []
    reference = None
    index = 0
    while index < len(argv):
        argument = argv[index]
        if argument == "--self-test":
            self_test = True
        elif argument == "--reference":
            index += 1
            if index >= len(argv):
                raise SystemExit("--reference requires a JSONL path")
            reference = pathlib.Path(argv[index])
        elif argument.startswith("--"):
            raise SystemExit(f"unknown option {argument}")
        else:
            positional.append(pathlib.Path(argument))
        index += 1
    if self_test:
        return True, ROOT / "scripts/guard_ml/shadow_fixture.jsonl", reference
    source = positional[0] if positional else ROOT / "artifacts/guard-dataset-v3.jsonl"
    if reference is None and len(positional) > 1:
        reference = positional[1]
    return False, source, reference


def main() -> int:
    try:
        self_test, src, reference_path = _cli_paths(sys.argv[1:])
    except SystemExit as error:
        print(error)
        return 2
    fixture_mode = self_test or src.name == "shadow_fixture.jsonl"
    reference = load_events(reference_path) if reference_path is not None and reference_path.exists() else None
    if not src.exists():
        if self_test:
            print(f"missing {src}")
            return 1
        report = analyze([], reference, evidence_status="UNAVAILABLE", reference_kind="input" if reference is not None else "synthetic")
        print(json.dumps(report, indent=2))
        return 0

    status = "TEST_ONLY" if fixture_mode else "AVAILABLE"
    report = analyze(
        load_events(src),
        reference,
        evidence_status=status,
        reference_kind="input" if reference is not None else "synthetic",
    )
    if fixture_mode:
        assert report["sample_count"] == 3
        assert report["model_vs_deterministic_disagreement"] >= 2
        assert report["high_score_allow"] >= 1
        assert report["low_score_deny"] >= 1
        assert report["REAL_SHADOW_EVIDENCE"]["status"] == "TEST_ONLY"
        print("self-test ok")
        return 0

    review_path = ROOT / "artifacts/review_queue.jsonl"
    review_path.write_text(
        "".join(json.dumps(item) + "\n" for item in report.pop("review_queue")),
        encoding="utf-8",
    )
    out = ROOT / "artifacts/guard-shadow-analysis.json"
    out.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    print(f"wrote review queue {review_path} n={report['review_queue_size']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
