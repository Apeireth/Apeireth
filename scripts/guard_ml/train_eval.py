"""Train/validation/test evaluation for Guard joint features.

Logistic baseline is stdlib-only. LightGBM is optional and training-side only.
"""

from __future__ import annotations

import json
import math
import pathlib
import random
import sys
from collections import Counter, defaultdict

ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/guard_ml"))
from train_shadow import FEATURES, feature_vec, load_rows, sigmoid, train  # type: ignore


def split_by_family(rows: list[dict]) -> dict[str, list[dict]]:
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


def tpr_at_fpr(ys: list[int], ps: list[float], target_fpr: float):
    neg = sum(1 for y in ys if y == 0)
    pos = sum(ys)
    if neg < 100 or pos == 0:
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


def metrics(rows: list[dict], weights: list[float], bias: float) -> dict:
    if not rows:
        return {"status": "INSUFFICIENT_SAMPLE_SIZE"}
    ys = []
    ps = []
    tp = fp = tn = fn = 0
    for row in rows:
        y = 1 if str(row.get("label", "benign")) in {"1", "risky", "true"} else 0
        p = sigmoid(bias + sum(w * v for w, v in zip(weights, feature_vec(row))))
        ys.append(y)
        ps.append(p)
        pred = int(p >= 0.5)
        tp += int(pred == 1 and y == 1)
        fp += int(pred == 1 and y == 0)
        tn += int(pred == 0 and y == 0)
        fn += int(pred == 0 and y == 1)
    precision = tp / (tp + fp) if tp + fp else 0.0
    recall = tp / (tp + fn) if tp + fn else 0.0
    f1 = 2 * precision * recall / (precision + recall) if precision + recall else 0.0
    brier = sum((p - y) ** 2 for p, y in zip(ps, ys)) / len(rows)
    return {
        "n": len(rows),
        "precision": precision,
        "recall": recall,
        "f1": f1,
        "brier": brier,
        "auroc": auc_roc(ys, ps),
        "auprc": average_precision(ys, ps),
        "positives": sum(ys),
        "tpr_at_fpr_1pct": tpr_at_fpr(ys, ps, 0.01),
        "tpr_at_fpr_0_1pct": tpr_at_fpr(ys, ps, 0.001),
    }


def maybe_lightgbm(split: dict[str, list[dict]]) -> dict:
    try:
        import lightgbm  # type: ignore
        import numpy as np  # type: ignore
    except Exception:
        return {"status": "NOT_IMPLEMENTED", "reason": "lightgbm not installed"}
    def xy(rows):
        x = np.array([feature_vec(row) for row in rows], dtype=float)
        y = np.array([1 if str(row.get("label")) in {"1", "risky", "true"} else 0 for row in rows])
        return x, y
    x_train, y_train = xy(split["train"])
    model = lightgbm.LGBMClassifier(n_estimators=80, max_depth=3, verbosity=-1)
    model.fit(x_train, y_train)
    report = {}
    for name in ("validation", "test"):
        if not split[name]:
            report[name] = {"status": "INSUFFICIENT_SAMPLE_SIZE"}
            continue
        x, y = xy(split[name])
        pred = model.predict(x)
        tp = int(((pred == 1) & (y == 1)).sum())
        fp = int(((pred == 1) & (y == 0)).sum())
        fn = int(((pred == 0) & (y == 1)).sum())
        precision = tp / (tp + fp) if tp + fp else 0.0
        recall = tp / (tp + fn) if tp + fn else 0.0
        report[name] = {"precision": precision, "recall": recall, "f1": 2 * precision * recall / (precision + recall) if precision + recall else 0.0}
    return report


def main() -> int:
    random.seed(7)
    src = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "scripts/guard_ml/generated_features.jsonl"
    if not src.exists():
        src = ROOT / "scripts/guard_ml/scenarios.jsonl"
    rows = load_rows(src)
    split = split_by_family(rows)
    weights, bias = train(list(split["train"]))
    report = {
        "source": str(src),
        "train_size": len(split["train"]),
        "validation_size": len(split["validation"]),
        "test_size": len(split["test"]),
        "class_balance": {
            "risky": sum(1 for row in rows if str(row.get("label")) in {"1", "risky", "true"}),
            "benign": sum(1 for row in rows if str(row.get("label")) not in {"1", "risky", "true"}),
        },
        "category_distribution": dict(
            Counter(str(row.get("category") or row.get("family") or "unknown") for row in rows)
        ),
        "logistic": {
            "validation": metrics(split["validation"], weights, bias),
            "test": metrics(split["test"], weights, bias),
        },
        "lightgbm": maybe_lightgbm(split),
        "calibration": "uncalibrated",
    }
    out = ROOT / "artifacts/guard-joint-shadow-v0.metrics.json"
    out.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
