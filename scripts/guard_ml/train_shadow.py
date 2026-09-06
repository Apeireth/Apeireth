#!/usr/bin/env python3
"""Train the non-production Guard joint shadow artifact.

The script is deliberately dependency-free. It consumes only taxonomy and
numeric feature values from scenario JSONL; prompts, commands, paths, URLs,
secrets, and reasoning text are not part of the format.
"""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path

FEATURES = [
    "alignment_score",
    "credential_to_external",
    "unrequested_network_egress",
    "unrequested_credential_access",
    "unrequested_shell_execution",
    "unrequested_delete",
    "unrequested_publish",
    "sensitive_to_external_flow",
    "retry_after_denial",
    "alternate_tool_after_denial",
    "denied_count",
    "external_effect_count",
    "scope_expansion_count",
    "cross_turn_denied_action_count",
    "cross_turn_credential_probe_count",
    "failed_action_ratio",
]


def sigmoid(value: float) -> float:
    return 1.0 / (1.0 + math.exp(-max(-30.0, min(30.0, value))))


def load_rows(path: Path) -> list[dict]:
    rows = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            row = json.loads(line)
            rows.append(
                (
                    [float(row.get("features", {}).get(name, 0.0)) for name in FEATURES],
                    float(row["label"]),
                )
            )
    if not rows or len({label for _, label in rows}) < 2:
        raise ValueError("scenario dataset must contain both classes")
    return rows


def train(rows: list[tuple[list[float], float]]) -> tuple[list[float], float]:
    weights = [0.0] * len(FEATURES)
    bias = 0.0
    learning_rate = 0.08
    regularization = 0.001
    for _ in range(1200):
        gradient = [0.0] * len(FEATURES)
        bias_gradient = 0.0
        for values, label in rows:
            prediction = sigmoid(bias + sum(weight * value for weight, value in zip(weights, values)))
            error = prediction - label
            bias_gradient += error
            for index, value in enumerate(values):
                gradient[index] += error * value
        scale = 1.0 / len(rows)
        bias -= learning_rate * bias_gradient * scale
        for index in range(len(weights)):
            gradient[index] = gradient[index] * scale + regularization * weights[index]
            weights[index] -= learning_rate * gradient[index]
    return weights, bias


def metrics(rows: list[tuple[list[float], float]], weights: list[float], bias: float) -> dict:
    scored = [(sigmoid(bias + sum(w * x for w, x in zip(weights, values))), label) for values, label in rows]
    tp = fp = tn = fn = 0
    for score, label in scored:
        predicted = score >= 0.5
        if predicted and label == 1:
            tp += 1
        elif predicted and label == 0:
            fp += 1
        elif not predicted and label == 0:
            tn += 1
        else:
            fn += 1
    precision = tp / max(1, tp + fp)
    recall = tp / max(1, tp + fn)
    return {
        "sample_count": len(rows),
        "positive_count": sum(label for _, label in rows),
        "confusion_matrix": {"tp": tp, "fp": fp, "tn": tn, "fn": fn},
        "precision": precision,
        "recall": recall,
        "f1": 2 * precision * recall / max(1e-12, precision + recall),
        "brier": sum((score - label) ** 2 for score, label in scored) / len(scored),
        "note": "synthetic deterministic shadow evaluation; not production calibrated",
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", type=Path, default=Path(__file__).with_name("scenarios.jsonl"))
    parser.add_argument("--output", type=Path, default=Path("artifacts/guard-joint-shadow-v0.json"))
    parser.add_argument("--metrics", type=Path, default=Path("artifacts/guard-joint-shadow-v0.metrics.json"))
    args = parser.parse_args()
    rows = load_rows(args.input)
    weights, bias = train(rows)
    artifact = {
        "schema_version": "AgentChainFeatureV2",
        "model_id": "guard-joint-shadow-v0",
        "model_version": "guard-joint-shadow-v0.1",
        "feature_names": FEATURES,
        "weights": weights,
        "bias": bias,
        "critical_threshold": 0.9,
        "high_threshold": 0.7,
        "medium_threshold": 0.4,
        "mode": "shadow",
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.metrics.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(artifact, indent=2) + "\n", encoding="utf-8")
    args.metrics.write_text(json.dumps(metrics(rows, weights, bias), indent=2) + "\n", encoding="utf-8")
    print(json.dumps(metrics(rows, weights, bias), sort_keys=True))


if __name__ == "__main__":
    main()
