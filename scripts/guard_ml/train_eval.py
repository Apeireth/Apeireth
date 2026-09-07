"""True group-holdout evaluation for Guard joint features.

Logistic baseline is stdlib-only. LightGBM is optional and training-side only.
Guard 3.1 within-family splits are retained only as a labeled baseline.
"""

from __future__ import annotations

import hashlib
import json
import math
import pathlib
import random
import subprocess
import sys
from collections import Counter, defaultdict
from datetime import datetime, timezone

ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/guard_ml"))
from train_shadow import (  # type: ignore
    FEATURES,
    canonical_artifact_sha256,
    feature_schema_hash,
    feature_vec,
    git_commit,
    load_rows,
    sigmoid,
    stamp,
    train,
)


def family_split(family: str) -> str:
    digest = hashlib.sha256(family.encode("utf-8")).digest()
    n = digest[0] % 10
    if n <= 4:
        return "train"
    if n == 5:
        return "calibration"
    if n <= 7:
        return "validation"
    return "test"


def is_risky(row: dict) -> int:
    return 1 if str(row.get("label", "benign")) in {"1", "risky", "true"} else 0


def split_true_group(rows: list[dict]) -> dict[str, list[dict]]:
    buckets = defaultdict(list)
    families = defaultdict(set)
    for row in rows:
        if str(row.get("holdout_group") or ""):
            continue
        family = str(row.get("family") or row.get("category") or "unknown")
        name = family_split(family)
        buckets[name].append(row)
        families[name].add(family)
    for left, right in (
        ("train", "validation"),
        ("train", "test"),
        ("train", "calibration"),
        ("validation", "test"),
        ("validation", "calibration"),
        ("test", "calibration"),
    ):
        overlap = families[left] & families[right]
        if overlap:
            raise SystemExit(f"family overlap between {left} and {right}: {sorted(overlap)}")
    if families["train"] & families["validation"] or families["train"] & families["test"]:
        raise SystemExit("family isolation failed")
    return {
        "train": buckets["train"],
        "calibration": buckets["calibration"],
        "validation": buckets["validation"],
        "test": buckets["test"],
        "train_families": sorted(families["train"]),
        "calibration_families": sorted(families["calibration"]),
        "validation_families": sorted(families["validation"]),
        "test_families": sorted(families["test"]),
    }


def split_within_family(rows: list[dict]) -> dict[str, list[dict]]:
    grouped: dict[str, list[dict]] = defaultdict(list)
    for row in rows:
        family = row.get("family") or row.get("category") or "unknown"
        grouped[str(family)].append(row)
    train_rows, val_rows, test_rows = [], [], []
    for family, items in grouped.items():
        items = list(items)
        random.Random(family).shuffle(items)
        n = len(items)
        n_train = max(1, int(n * 0.6)) if n > 2 else n
        n_val = max(0, int(n * 0.2)) if n > 4 else 0
        train_rows.extend(items[:n_train])
        val_rows.extend(items[n_train : n_train + n_val])
        test_rows.extend(items[n_train + n_val :])
    return {"train": train_rows, "validation": val_rows, "test": test_rows}


def auc_roc(ys: list[int], ps: list[float]):
    pos = [p for y, p in zip(ys, ps) if y == 1]
    neg = [p for y, p in zip(ys, ps) if y == 0]
    if not pos or not neg:
        return "INSUFFICIENT_SAMPLE_SIZE"
    greater = sum(1 for p in pos for n in neg if p > n)
    equal = sum(1 for p in pos for n in neg if p == n)
    return (greater + 0.5 * equal) / (len(pos) * len(neg))


def average_precision(ys: list[int], ps: list[float]):
    if not any(ys) or not ys:
        return "INSUFFICIENT_SAMPLE_SIZE"
    order = sorted(range(len(ps)), key=lambda i: ps[i], reverse=True)
    tp = 0
    fp = 0
    positives = sum(ys)
    ap = 0.0
    prev_recall = 0.0
    for idx in order:
        if ys[idx] == 1:
            tp += 1
        else:
            fp += 1
        precision = tp / (tp + fp)
        recall = tp / positives
        ap += precision * (recall - prev_recall)
        prev_recall = recall
    return ap


def tpr_at_fpr(ys: list[int], ps: list[float], target_fpr: float, min_neg: int):
    neg = sum(1 for y in ys if y == 0)
    pos = sum(ys)
    if neg < min_neg or pos == 0:
        return "INSUFFICIENT_SAMPLE_SIZE"
    order = sorted(range(len(ps)), key=lambda i: ps[i], reverse=True)
    fp = 0
    tp = 0
    best = 0.0
    for idx in order:
        if ys[idx] == 1:
            tp += 1
        else:
            fp += 1
        fpr = fp / neg
        if fpr <= target_fpr:
            best = tp / pos
        else:
            break
    return best


def ece(ys: list[int], ps: list[float], bins: int = 10):
    if len(ys) < bins:
        return "INSUFFICIENT_SAMPLE_SIZE"
    total = 0.0
    for i in range(bins):
        lo = i / bins
        hi = (i + 1) / bins
        chosen = [(p, y) for p, y in zip(ps, ys) if (p >= lo and p < hi) or (i == bins - 1 and p == 1.0)]
        if not chosen:
            continue
        avg_p = sum(p for p, _ in chosen) / len(chosen)
        avg_y = sum(y for _, y in chosen) / len(chosen)
        total += abs(avg_p - avg_y) * len(chosen)
    return total / len(ys)


def predict_rows(rows: list[dict], weights: list[float], bias: float, platt=None) -> tuple[list[int], list[float]]:
    ys = []
    ps = []
    for row in rows:
        y = is_risky(row)
        logit = bias + sum(w * v for w, v in zip(weights, feature_vec(row)))
        if platt:
            a, b = platt
            logit = a * logit + b
        p = sigmoid(logit)
        ys.append(y)
        ps.append(p)
    return ys, ps


def metrics_from_scores(ys: list[int], ps: list[float]) -> dict:
    if not ys:
        return {"status": "INSUFFICIENT_SAMPLE_SIZE"}
    tp = fp = tn = fn = 0
    for y, p in zip(ys, ps):
        pred = int(p >= 0.5)
        tp += int(pred == 1 and y == 1)
        fp += int(pred == 1 and y == 0)
        tn += int(pred == 0 and y == 0)
        fn += int(pred == 0 and y == 1)
    precision = tp / (tp + fp) if tp + fp else 0.0
    recall = tp / (tp + fn) if tp + fn else 0.0
    f1 = 2 * precision * recall / (precision + recall) if precision + recall else 0.0
    brier = sum((p - y) ** 2 for p, y in zip(ps, ys)) / len(ys)
    neg = sum(1 for y in ys if y == 0)
    return {
        "n": len(ys),
        "positives": sum(ys),
        "negatives": neg,
        "precision": precision,
        "recall": recall,
        "f1": f1,
        "brier": brier,
        "auroc": auc_roc(ys, ps),
        "auprc": average_precision(ys, ps),
        "ece": ece(ys, ps),
        "tpr_at_fpr_1pct": tpr_at_fpr(ys, ps, 0.01, 200),
        "tpr_at_fpr_0_1pct": tpr_at_fpr(ys, ps, 0.001, 1000),
        "fp": fp,
        "fn": fn,
    }


def metrics(rows: list[dict], weights: list[float], bias: float, platt=None) -> dict:
    ys, ps = predict_rows(rows, weights, bias, platt)
    return metrics_from_scores(ys, ps)


def breakdown(rows: list[dict], weights: list[float], bias: float, key: str, platt=None) -> dict:
    groups = defaultdict(lambda: {"fp": 0, "fn": 0, "n": 0})
    ys, ps = predict_rows(rows, weights, bias, platt)
    for row, y, p in zip(rows, ys, ps):
        name = str(row.get(key) or "unknown")
        pred = int(p >= 0.5)
        groups[name]["n"] += 1
        groups[name]["fp"] += int(pred == 1 and y == 0)
        groups[name]["fn"] += int(pred == 0 and y == 1)
    return dict(groups)


def fit_platt(rows: list[dict], weights: list[float], bias: float) -> tuple[float, float] | None:
    if len(rows) < 20:
        return None
    a, b = 1.0, 0.0
    for _ in range(200):
        for row in rows:
            y = float(is_risky(row))
            z = bias + sum(w * v for w, v in zip(weights, feature_vec(row)))
            p = sigmoid(a * z + b)
            err = p - y
            a -= 0.05 * err * z
            b -= 0.05 * err
    if not math.isfinite(a) or not math.isfinite(b):
        return None
    return a, b


def maybe_lightgbm(train_rows: list[dict], eval_rows: dict[str, list[dict]]) -> dict:
    try:
        import lightgbm  # type: ignore
        import numpy as np  # type: ignore
    except Exception:
        return {"status": "NOT_IMPLEMENTED", "reason": "lightgbm or numpy not installed"}

    def xy(rows):
        x = np.array([feature_vec(row) for row in rows], dtype=float)
        y = np.array([is_risky(row) for row in rows])
        return x, y

    if len({is_risky(row) for row in train_rows}) < 2:
        return {"status": "INSUFFICIENT_SAMPLE_SIZE"}
    model = lightgbm.LGBMClassifier(n_estimators=80, max_depth=3, verbosity=-1)
    x_train, y_train = xy(train_rows)
    model.fit(x_train, y_train)
    report = {}
    for name, rows in eval_rows.items():
        if not rows:
            report[name] = {"status": "INSUFFICIENT_SAMPLE_SIZE"}
            continue
        x, y = xy(rows)
        proba = model.predict_proba(x)[:, 1].tolist()
        report[name] = metrics_from_scores(y.tolist(), proba)
    return report


def select_rows(rows: list[dict], benchmark: str) -> list[dict]:
    out = []
    for row in rows:
        marks = row.get("benchmarks") or []
        if benchmark in marks or (isinstance(marks, str) and benchmark == marks):
            out.append(row)
    return out


def pair_scores(rows: list[dict], pairs: list[dict], weights, bias, platt=None) -> dict:
    by_id = {str(row.get("id")): row for row in rows}
    correct = 0
    considered = 0
    margins = []
    for pair in pairs:
        risky = by_id.get(pair["risky_id"])
        aligned = by_id.get(pair["aligned_id"])
        if not risky or not aligned:
            continue
        _, p_risky = predict_rows([risky], weights, bias, platt)
        _, p_aligned = predict_rows([aligned], weights, bias, platt)
        considered += 1
        margin = p_risky[0] - p_aligned[0]
        margins.append(margin)
        if p_risky[0] > p_aligned[0]:
            correct += 1
    if not considered:
        return {"status": "INSUFFICIENT_SAMPLE_SIZE"}
    margins.sort()
    return {
        "pairwise_ordering_accuracy": correct / considered,
        "counterfactual_accuracy": correct / considered,
        "mean_score_margin": sum(margins) / len(margins),
        "median_margin": margins[len(margins) // 2],
        "n_pairs": considered,
    }


def load_pairs(kind: str) -> list[dict]:
    path = ROOT / f"scripts/guard_ml/{kind}_pairs.json"
    if not path.exists():
        return []
    return json.loads(path.read_text(encoding="utf-8"))


def infer_pairs(rows: list[dict], kind: str) -> list[dict]:
    loaded = load_pairs(kind)
    if loaded:
        return loaded
    groups = defaultdict(dict)
    suffix_risky = "_risky" if kind == "counterfactual" else "_deny"
    suffix_aligned = "_aligned" if kind == "counterfactual" else "_allow"
    for row in rows:
        row_id = str(row.get("id") or "")
        pair_id = row.get("pair_id")
        if not pair_id:
            continue
        if row_id.endswith(suffix_risky):
            groups[str(pair_id)]["risky_id"] = row_id
        elif row_id.endswith(suffix_aligned):
            groups[str(pair_id)]["aligned_id"] = row_id
    return [
        {"id": key, "risky_id": value["risky_id"], "aligned_id": value["aligned_id"]}
        for key, value in groups.items()
        if "risky_id" in value and "aligned_id" in value
    ]


def summarize(rows: list[dict]) -> dict:
    return {
        "row_count": len(rows),
        "family_count": len({str(row.get("family")) for row in rows}),
        "class_balance": {
            "risky": sum(is_risky(row) for row in rows),
            "benign": sum(1 - is_risky(row) for row in rows),
        },
        "category_balance": dict(Counter(str(row.get("category") or "unknown") for row in rows)),
        "language_balance": dict(Counter(str(row.get("language") or "unknown") for row in rows)),
    }


def write_manifest(src: pathlib.Path, rows: list[dict]) -> None:
    families = {str(row.get("family")) for row in rows}
    languages = Counter(str(row.get("language") or "unknown") for row in rows)
    labels = Counter("risky" if is_risky(row) else "benign" for row in rows)
    catalog_hash = hashlib.sha256(src.read_bytes()).hexdigest()
    manifest_path = ROOT / "artifacts/guard-dataset-manifest.json"
    previous = {}
    if manifest_path.exists():
        previous = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest = {
        **previous,
        "dataset_id": previous.get("dataset_id", "guard-scenario-catalog-v3.2"),
        "dataset_version": "v3.2",
        "scenario_count": len(rows),
        "family_count": len(families),
        "language_distribution": dict(languages),
        "class_balance": dict(labels),
        "scenario_catalog_hash": catalog_hash,
        "feature_schema_hash": feature_schema_hash(),
        "generator_commit": git_commit(),
        "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "training_dataset_hash": catalog_hash,
    }
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


def main() -> int:
    random.seed(7)
    src = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "scripts/guard_ml/generated_features.jsonl"
    if not src.exists():
        src = ROOT / "scripts/guard_ml/scenarios.jsonl"
    rows = load_rows(src)
    write_manifest(src, rows)
    true_split = split_true_group(rows)
    within = split_within_family(rows)
    weights, bias = train(list(true_split["train"]))
    platt = fit_platt(true_split["calibration"], weights, bias)
    template_holdout = [row for row in rows if row.get("holdout_group") == "intent_template"]
    tool_holdout = [row for row in rows if row.get("holdout_group") == "tool_shell"]
    b_rows = {f"b{i}": select_rows(rows, f"b{i}") for i in range(1, 11)}
    logistic_true = {
        "validation": metrics(true_split["validation"], weights, bias, platt),
        "test": metrics(true_split["test"], weights, bias, platt),
    }
    report = {
        "source": str(src),
        "seed": 7,
        "terminology": {
            "guard_3_1_split": "within-family held-out baseline, not true unseen-family holdout",
            "guard_3_2_split": "true family-held-out / group holdout",
            "model_role": "synthetic benchmark / shadow evaluation model",
        },
        "train_families": true_split["train_families"],
        "calibration_families": true_split["calibration_families"],
        "validation_families": true_split["validation_families"],
        "test_families": true_split["test_families"],
        "split_counts": {
            "train": summarize(true_split["train"]),
            "calibration": summarize(true_split["calibration"]),
            "validation": summarize(true_split["validation"]),
            "test": summarize(true_split["test"]),
            "all": summarize(rows),
        },
        "within_family_held_out_baseline": {
            "note": "Guard 3.1 metrics used within-family held-out instances, not true unseen-family holdout.",
            "logistic": {
                "validation": metrics(within["validation"], *train(list(within["train"]))),
                "test": metrics(within["test"], *train(list(within["train"]))),
            },
        },
        "logistic": {
            "unseen_family_holdout": logistic_true,
            "intent_template_holdout": metrics(template_holdout, weights, bias, platt),
            "tool_family_holdout": metrics(tool_holdout, weights, bias, platt),
            "benchmarks": {name: metrics(items, weights, bias, platt) for name, items in b_rows.items()},
            "counterfactual": pair_scores(rows, infer_pairs(rows, "counterfactual"), weights, bias, platt),
            "negation": pair_scores(rows, infer_pairs(rows, "negation"), weights, bias, platt),
            "fp_by_intent_class": breakdown(true_split["test"], weights, bias, "intent_class", platt),
            "fp_by_category": breakdown(true_split["test"], weights, bias, "category", platt),
            "fp_by_tool_origin": breakdown(true_split["test"], weights, bias, "tool_origin", platt),
            "fp_by_language": breakdown(true_split["test"], weights, bias, "language", platt),
            "fp_by_family": breakdown(true_split["test"], weights, bias, "family", platt),
        },
        "lightgbm": maybe_lightgbm(
            true_split["train"],
            {
                "test": true_split["test"],
                "intent_template_holdout": template_holdout,
                "tool_family_holdout": tool_holdout,
            },
        ),
        "calibration": {
            "kind": "platt" if platt else "uncalibrated",
            "a": None if not platt else platt[0],
            "b": None if not platt else platt[1],
            "split": "calibration families only; never reused as test",
        },
        "artifact_sha256_of_canonical_payload": None,
    }
    artifact_path = ROOT / "artifacts/guard-joint-shadow-v0.json"
    artifact_sha = None
    artifact_provenance = "unavailable"
    if artifact_path.exists():
        artifact_serialized = artifact_path.read_text(encoding="utf-8")
        artifact_value = json.loads(artifact_serialized)
        artifact_sha = canonical_artifact_sha256(artifact_value)
        artifact_provenance = "checked-in artifact canonical payload"
    report["artifact_sha256_of_canonical_payload"] = artifact_sha
    report["artifact_provenance"] = artifact_provenance
    report["training_provenance"] = {
        "dataset_hash": hashlib.sha256(src.read_bytes()).hexdigest(),
        "feature_schema_hash": feature_schema_hash(),
        "training_commit": git_commit(),
        "training_script_version": "train_shadow.py",
        "evaluation_script_version": "train_eval.py",
        "seed": 7,
    }
    out = ROOT / "artifacts/guard-joint-shadow-v0.metrics.json"
    out.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps({k: report[k] for k in ("train_families", "validation_families", "test_families", "split_counts", "calibration")}, indent=2))
    print(json.dumps(report["logistic"]["unseen_family_holdout"], indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
