"""Analyze real shadow Guard dataset events.

Deterministic deny and human approval are weak supervision signals, never
ground truth. Raw prompts, commands, paths, URLs, secrets, and CoT must not
appear in the input records.
"""

from __future__ import annotations

import json
import pathlib
import sys
from collections import Counter

ROOT = pathlib.Path(__file__).resolve().parents[2]

FORBIDDEN = ("prompt", "command", "path", "url", "secret", "password", "token", "cot")


def load_events(path: pathlib.Path) -> list[dict]:
    events = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            events.append(json.loads(line))
    return events


def nested(event: dict) -> tuple[str, dict]:
    for key in ("Classification", "Approval", "Execution"):
        if key in event:
            return key.lower(), event[key]
    kind = str(event.get("record_type") or event.get("kind") or "").lower()
    return kind, event


def analyze(events: list[dict]) -> dict:
    classifications = []
    approvals = {}
    executions = {}
    for event in events:
        kind, inner = nested(event)
        if "chain_features" in inner or "features" in inner or kind in {"classification"}:
            classifications.append(inner)
        elif kind == "approval" or "approval_id" in inner:
            approvals[(inner.get("trace_id", ""), inner.get("action_id", ""))] = inner.get("decision") or inner.get(
                "human_decision"
            )
        elif kind == "execution" or inner.get("outcome"):
            executions[(inner.get("trace_id", ""), inner.get("action_id", ""))] = inner.get("outcome")
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
        blob = json.dumps(item, ensure_ascii=True).lower()
        for word in FORBIDDEN:
            if f'"{word}"' in blob and word not in {"token"}:
                raise SystemExit(f"privacy invariant violated: {word}")
        pred = item.get("classifier_prediction") or item.get("shadow_prediction") or {}
        score = float(pred.get("score") or item.get("shadow_score") or 0.0)
        pred_risky = bool(pred.get("class") in {"high", "critical"} or score >= 0.5)
        decision = str(item.get("final_decision") or item.get("deterministic_decision") or "allow").lower()
        allow = decision in {"allow", "allowed"}
        deny = "deny" in decision
        approval = "approval" in decision or decision == "require_approval"
        if pred_risky != (not allow):
            disagreement += 1
        if pred_risky and allow:
            high_score_allow += 1
        if (not pred_risky) and deny:
            low_score_deny += 1
        if approval and abs(score - 0.5) < 0.15:
            approval_required_disagreement += 1
        categories[str(item.get("category") or item.get("operation_class") or "unknown")] += 1
        origins[str(item.get("tool_origin") or item.get("descriptor_source") or "unknown")] += 1
        descriptor_sources[str(item.get("descriptor_source") or "unknown")] += 1
        intent_classes[str(item.get("intent_class") or "unknown")] += 1
        bucket = f"{int(score * 10) / 10:.1f}"
        confidence_bins[bucket] += 1
        uncertain = abs(score - 0.5) < 0.1 or (pred_risky and allow) or ((not pred_risky) and deny)
        if uncertain:
            review.append(
                {
                    "feature_snapshot_id": item.get("feature_snapshot_id"),
                    "reason_codes": item.get("reason_codes") or item.get("risk_tags") or [],
                    "prediction": pred,
                    "decision": decision,
                    "outcome_taxonomy": executions.get((item.get("trace_id", ""), item.get("action_id", ""))),
                    "approval": approvals.get((item.get("trace_id", ""), item.get("action_id", ""))),
                    "weak_supervision_only": True,
                    "deterministic_deny_is_not_ground_truth": True,
                    "approval_is_not_ground_truth": True,
                }
            )
    return {
        "sample_count": len(classifications),
        "model_vs_deterministic_disagreement": disagreement,
        "high_score_allow": high_score_allow,
        "low_score_deny": low_score_deny,
        "approval_required_disagreements": approval_required_disagreement,
        "category_distribution": dict(categories),
        "tool_origin_distribution": dict(origins),
        "descriptor_source_distribution": dict(descriptor_sources),
        "intent_class_distribution": dict(intent_classes),
        "confidence_histogram": dict(sorted(confidence_bins.items())),
        "review_queue_size": len(review),
        "review_queue": review,
        "label_policy": "deterministic deny and approval outcomes are weak signals, not ground truth",
    }


def main() -> int:
    src = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "artifacts/guard-dataset-v3.jsonl"
    if len(sys.argv) > 1 and sys.argv[1] == "--self-test":
        src = ROOT / "scripts/guard_ml/shadow_fixture.jsonl"
    if not src.exists():
        print(f"missing {src}")
        return 1
    report = analyze(load_events(src))
    if src.name == "shadow_fixture.jsonl":
        assert report["sample_count"] == 3
        assert report["model_vs_deterministic_disagreement"] >= 2
        assert report["high_score_allow"] >= 1
        assert report["low_score_deny"] >= 1
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
