"""Train the Guard joint shadow logistic model from generated FeatureV2 rows.

This script is dependency-free (Python 3 stdlib only). It is a baseline, not a
calibrated production model.
"""

from __future__ import annotations

import json
import math
import hashlib
import pathlib
import random
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
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


def load_rows(path: pathlib.Path) -> list[dict]:
    rows = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            rows.append(json.loads(line))
    return rows


def feature_vec(row: dict) -> list[float]:
    src = row.get("features") or row.get("chain_features") or {}
    values = []
    for name in FEATURES:
        if name.startswith("cross_turn_"):
            nested = src.get("cross_turn") or {}
            key = name[len("cross_turn_") :]
            values.append(float(nested.get(key, src.get(name, 0.0)) or 0.0))
        elif name in ("sensitive_to_external_flow", "retry_after_denial", "alternate_tool_after_denial", "denied_count", "external_effect_count"):
            v1 = src.get("v1") or {}
            raw = v1.get(name, src.get(name, 0.0))
            values.append(float(raw if not isinstance(raw, bool) else int(raw)))
        else:
            raw = src.get(name, 0.0)
            values.append(float(raw if not isinstance(raw, bool) else int(raw)))
    return values


def sigmoid(z: float) -> float:
    z = max(-30.0, min(30.0, z))
    return 1.0 / (1.0 + math.exp(-z))


def train(rows: list[dict], steps: int = 400, lr: float = 0.2) -> tuple[list[float], float]:
    weights = [0.0] * len(FEATURES)
    bias = 0.0
    for _ in range(steps):
        random.shuffle(rows)
        for row in rows:
            x = feature_vec(row)
            y = 1.0 if str(row.get("label", "benign")) in {"1", "risky", "true"} else 0.0
            z = bias + sum(w * v for w, v in zip(weights, x))
            p = sigmoid(z)
            err = p - y
            for i, v in enumerate(x):
                weights[i] -= lr * err * v
            bias -= lr * err
    return weights, bias


def main() -> int:
    src = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "scripts/guard_ml/generated_features.jsonl"
    if not src.exists():
        src = ROOT / "scripts/guard_ml/scenarios.jsonl"
    rows = load_rows(src)
    weights, bias = train(list(rows))
    artifact = {
        "artifact_schema": "guard-joint-artifact-v1",
        "schema_version": "AgentChainFeatureV2",
        "feature_schema": "AgentChainFeatureV2",
        "model_id": "guard-joint-shadow-v0",
        "model_version": "guard-joint-shadow-v0.1",
        "feature_names": FEATURES,
        "weights": weights,
        "bias": bias,
        "critical_threshold": 0.9,
        "high_threshold": 0.7,
        "medium_threshold": 0.4,
        "mode": "shadow",
        "training_dataset_hash": hashlib.sha256(src.read_bytes()).hexdigest(),
        "training_script_version": "train_shadow.py",
        "calibration": {"kind": "uncalibrated"},
    }
    out = ROOT / "artifacts/guard-joint-shadow-v0.json"
    out.write_text(json.dumps(artifact, indent=2), encoding="utf-8")
    print(f"wrote {out} from {src} n={len(rows)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
