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


def resolve_path(value: str | pathlib.Path) -> pathlib.Path:
    path = pathlib.Path(value)
    return path if path.is_absolute() else ROOT / path


def deterministic_split(group: str, salt: str | None = None) -> str:
    """Hash the raw family for family splits; salt auxiliary dimensions."""
    payload = group if salt in (None, "family") else f"{salt}:{group}"
    digest = hashlib.sha256(payload.encode("utf-8")).digest()
    n = digest[0] % 10
    if n <= 4:
        return "train"
    if n == 5:
        return "calibration"
    if n <= 7:
        return "validation"
    return "test"


def family_split(family: str) -> str:
    return deterministic_split(family)


def split_by_group(rows: list[dict], field: str, salt: str) -> dict[str, list[dict]]:
    """Assign whole groups to one partition; no row-level fallback splitting."""
    buckets = {name: [] for name in ("train", "calibration", "validation", "test")}
    groups: dict[str, set[str]] = {name: set() for name in buckets}
    for row in rows:
        group = str(row.get(field) or "unknown")
        target = deterministic_split(group, salt)
        buckets[target].append(row)
        groups[target].add(group)
    names = list(groups)
    for index, left in enumerate(names):
        for right in names[index + 1 :]:
            if groups[left] & groups[right]:
                raise SystemExit(f"{field} overlap between {left} and {right}")
    buckets.update({f"{field}_{name}": sorted(values) for name, values in groups.items()})
    return buckets


def file_sha256(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def repo_relative(path: pathlib.Path) -> str:
    try:
        return path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return str(path.resolve())


def git_worktree_dirty() -> bool:
    try:
        result = subprocess.run(
            ["git", "status", "--short"],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
        )
        return bool(result.stdout.strip())
    except Exception:
        return True


def provenance_hashes(dataset_path: pathlib.Path) -> dict[str, str]:
    hashes = {
        "dataset": file_sha256(dataset_path),
        "train_shadow_script": file_sha256(ROOT / "scripts/guard_ml/train_shadow.py"),
        "train_eval_script": file_sha256(ROOT / "scripts/guard_ml/train_eval.py"),
        "analyze_shadow_script": file_sha256(ROOT / "scripts/guard_ml/analyze_shadow.py"),
        "counterfactual_pairs": file_sha256(ROOT / "scripts/guard_ml/counterfactual_pairs.json"),
        "negation_pairs": file_sha256(ROOT / "scripts/guard_ml/negation_pairs.json"),
    }
    catalog_source = ROOT / "crates/engine/guard/src/scenario.rs"
    if catalog_source.exists():
        hashes["scenario_catalog_source"] = file_sha256(catalog_source)
    return hashes


def dimension_split(rows: list[dict], field: str, salt: str) -> dict[str, list[dict]]:
    """Split complete values of one dimension; each value gets one partition."""
    groups: dict[str, list[dict]] = defaultdict(list)
    for index, row in enumerate(rows):
        value = str(row.get(field) or f"missing:{index}")
        groups[value].append(row)
    buckets = {name: [] for name in ("train", "calibration", "validation", "test")}
    values = {name: set() for name in buckets}
    for value, items in groups.items():
        target = deterministic_split(value, salt)
        buckets[target].extend(items)
        values[target].add(value)
    buckets["groups"] = {name: sorted(items) for name, items in values.items()}  # type: ignore[assignment]
    return buckets


def evaluate_dimension_split(rows: list[dict], field: str, salt: str) -> dict:
    split = dimension_split(rows, field, salt)
    train_rows = split["train"]
    if len({is_risky(row) for row in train_rows}) < 2:
        return {"status": "INSUFFICIENT_SAMPLE_SIZE", "group_field": field}
    weights, bias = train(list(train_rows))
    return {
        "group_field": field,
        "train": summarize(train_rows),
        "calibration": summarize(split["calibration"]),
        "validation": metrics(split["validation"], weights, bias),
        "test": metrics(split["test"], weights, bias),
        "groups": split["groups"],
    }


def is_risky(row: dict) -> int:
    return 1 if str(row.get("label", "benign")) in {"1", "risky", "true"} else 0


def validate_family_splits(rows: list[dict]) -> None:
    for index, row in enumerate(rows, 1):
        family = str(row.get("family") or row.get("category") or "unknown")
        expected = family_split(family)
        declared = row.get("family_split")
        if declared not in (None, "") and str(declared) != expected:
            raise ValueError(f"row {index} family_split {declared!r} disagrees with {expected!r} for {family!r}")


def split_true_group(rows: list[dict]) -> dict[str, list[dict]]:
    """Assign complete families to partitions and verify pair integrity."""
    buckets = defaultdict(list)
    families = defaultdict(set)
    pair_partitions: dict[str, set[str]] = defaultdict(set)
    for row in rows:
        family = str(row.get("family") or row.get("category") or "unknown")
        name = family_split(family)
        buckets[name].append(row)
        families[name].add(family)
        pair_id = str(row.get("pair_id") or "")
        if pair_id:
            pair_partitions[pair_id].add(name)
    leaked_pairs = sorted(pair_id for pair_id, partitions in pair_partitions.items() if len(partitions) > 1)
    if leaked_pairs:
        raise SystemExit(f"pair overlap across partitions: {leaked_pairs}")
    for left, right in (("train", "validation"), ("train", "test"),
                        ("train", "calibration"), ("validation", "test"),
                        ("validation", "calibration"), ("test", "calibration")):
        overlap = families[left] & families[right]
        if overlap:
            raise SystemExit(f"family overlap between {left} and {right}: {sorted(overlap)}")
    return {
        "train": buckets["train"], "validation": buckets["validation"],
        "calibration": buckets["calibration"], "test": buckets["test"],
        "train_families": sorted(families["train"]),
        "calibration_families": sorted(families["calibration"]),
        "validation_families": sorted(families["validation"]),
        "test_families": sorted(families["test"]),
        "pair_partitions": {key: sorted(value) for key, value in sorted(pair_partitions.items())},
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


def _pair_split(row: dict) -> str:
    """Return a declared partition, without inventing one from row position."""
    for field in ("family_split", "split", "evaluation_split"):
        value = row.get(field)
        if value not in (None, ""):
            return str(value)
    return "unknown"


_FLIP_PAIR_FIELDS = (
    "flip_pair_id", "flipped_pair_id", "reverse_pair_id", "counterpart_id",
    "paired_flip_id", "flip_of",
)
_FLIP_GROUP_FIELDS = ("flip_group", "flipped_group", "reverse_group")


def _pair_flip_refs(pair: dict) -> tuple[set[str], set[str]]:
    """Extract optional flipped-pair references while preserving weak labels."""
    pair_ids: set[str] = set()
    groups: set[str] = set()
    for field in _FLIP_PAIR_FIELDS:
        value = pair.get(field)
        if value not in (None, ""):
            pair_ids.add(str(value))
    for field in _FLIP_GROUP_FIELDS:
        value = pair.get(field)
        if value not in (None, ""):
            groups.add(str(value))
    # Accept a nested flip/counterpart object used by some pair catalogs.
    for field in ("flip", "flipped", "reverse", "counterpart"):
        value = pair.get(field)
        values = value if isinstance(value, list) else [value]
        for item in values:
            if isinstance(item, dict):
                for key in ("id", "pair_id", "flip_pair_id", "flipped_pair_id"):
                    if item.get(key) not in (None, ""):
                        pair_ids.add(str(item[key]))
                for key in _FLIP_GROUP_FIELDS:
                    if item.get(key) not in (None, ""):
                        groups.add(str(item[key]))
            elif item not in (None, "", True, False):
                pair_ids.add(str(item))
    return pair_ids, groups


def _pair_flip_key(pair: dict):
    """Compatibility helper for callers that need one direct flip reference."""
    pair_ids, _ = _pair_flip_refs(pair)
    return next(iter(pair_ids), None)


def _flip_is_consistent(left: float, right: float) -> bool:
    return (left > 0 and right < 0) or (left < 0 and right > 0)


def pair_scores(rows: list[dict], pairs: list[dict], weights, bias, platt=None,
                evaluation_split: str = "test", pair_kind: str = "pair") -> dict:
    """Score pairs from one split and report explicit coverage/status semantics."""
    by_id = {str(row.get("id")): row for row in rows}
    expected = len(pairs)
    considered = correct = ties = failures = cross_split = missing = non_eval = 0
    unknown_split = 0
    transition_labelled = transition_correct = 0
    transition_counts: Counter[str] = Counter()
    transition_correct_counts: Counter[str] = Counter()
    margins: list[float] = []
    diagnostics: list[dict] = []
    scored_by_id: dict[str, float] = {}
    scored_pairs: dict[str, dict] = {}
    for pair in pairs:
        risky = by_id.get(str(pair.get("risky_id", "")))
        aligned = by_id.get(str(pair.get("aligned_id", "")))
        if not risky or not aligned:
            missing += 1
            continue
        risky_split, aligned_split = _pair_split(risky), _pair_split(aligned)
        pair_split = _pair_split(pair)
        declared = [value for value in (risky_split, aligned_split, pair_split) if value != "unknown"]
        if risky_split != "unknown" and aligned_split != "unknown" and risky_split != aligned_split:
            cross_split += 1
            continue
        if pair_split != "unknown" and any(value != "unknown" and value != pair_split
                                           for value in (risky_split, aligned_split)):
            cross_split += 1
            continue
        actual_split = declared[0] if declared else "unknown"
        if actual_split == "unknown":
            unknown_split += 1
            continue
        if actual_split != evaluation_split:
            non_eval += 1
            continue
        try:
            _, p_risky = predict_rows([risky], weights, bias, platt)
            _, p_aligned = predict_rows([aligned], weights, bias, platt)
            margin = p_risky[0] - p_aligned[0]
            if not math.isfinite(margin):
                raise ValueError("non-finite pair margin")
        except Exception:
            failures += 1
            continue
        considered += 1
        pair_id = str(pair.get("id") or f"pair:{considered}")
        scored_by_id[pair_id] = margin
        scored_pairs[pair_id] = pair
        margins.append(margin)
        transition = pair.get("transition")
        if pair_kind == "counterfactual":
            if transition not in {"RiskIncrease", "RiskDecrease", "NoMaterialChange"}:
                diagnostics.append({"pair_id": pair_id, "reason": "missing_or_invalid_transition_label"})
            else:
                transition_labelled += 1
                transition_counts[str(transition)] += 1
                predicted = "RiskIncrease" if margin > 0 else "RiskDecrease" if margin < 0 else "NoMaterialChange"
                if predicted == transition:
                    transition_correct += 1
                    transition_correct_counts[str(transition)] += 1
        if margin > 0:
            correct += 1
        elif margin == 0:
            ties += 1

    # Evaluate flip relationships after all pairs are scored, so references may
    # point either forward or backward in the catalog.
    flip_relationships: set[tuple[str, str]] = set()
    group_members: dict[str, list[str]] = defaultdict(list)
    for pair_id, pair in scored_pairs.items():
        _, groups = _pair_flip_refs(pair)
        for group in groups:
            group_members[group].append(pair_id)
    for pair_id, pair in scored_pairs.items():
        refs, _ = _pair_flip_refs(pair)
        for ref in refs:
            if ref in scored_by_id and ref != pair_id:
                flip_relationships.add(tuple(sorted((pair_id, ref))))
    for members in group_members.values():
        for index, left in enumerate(members):
            for right in members[index + 1:]:
                if left != right:
                    flip_relationships.add(tuple(sorted((left, right))))
    flip_considered = sum(1 for _ in flip_relationships)
    flip_consistent = sum(_flip_is_consistent(scored_by_id[left], scored_by_id[right])
                          for left, right in flip_relationships)
    flip_report = {
        "status": "OK" if flip_considered else "NOT_AVAILABLE",
        "considered": flip_considered,
        "consistent": flip_consistent,
        "accuracy": flip_consistent / flip_considered if flip_considered else None,
    }
    coverage = {
        "expected": expected, "considered": considered, "missing": missing,
        "cross_split": cross_split, "ties": ties, "failures": failures,
        "non_evaluation_split": non_eval, "unknown_split": unknown_split,
        "evaluation_split": evaluation_split,
        "transition_labelled": transition_labelled,
        "transition_unlabelled": max(0, considered - transition_labelled),
    }
    if not considered:
        status = "INSUFFICIENT_SAMPLE_SIZE"
    elif considered + non_eval < expected or any(coverage[key] for key in ("missing", "cross_split", "failures", "unknown_split")):
        status = "PARTIAL_COVERAGE"
    else:
        status = "OK"
    if not considered:
        result = {"status": status, "pair_kind": pair_kind, "n_pairs": 0,
                  "coverage": coverage, "flip_consistency": flip_report,
            "transition_accuracy": "INSUFFICIENT_SAMPLE_SIZE",
            "transition_by_label": {}, "diagnostics": diagnostics}
    else:
        ordered = sorted(margins)
        middle = len(ordered) // 2
        median = (ordered[middle] if len(ordered) % 2 else
                  (ordered[middle - 1] + ordered[middle]) / 2)
        result = {
            "pairwise_ordering_accuracy": correct / considered,
            "pair_kind": pair_kind,
            "tie_policy": "ties_count_as_incorrect",
            "mean_score_margin": sum(margins) / len(margins),
            "median_margin": median,
            "n_pairs": considered, "correct_pairs": correct,
            "incorrect_pairs": considered - correct - ties, "status": status,
            "coverage": coverage, "flip_consistency": flip_report,
            "transition_accuracy": (transition_correct / transition_labelled
                                     if transition_labelled else "NOT_AVAILABLE_NO_TRANSITION_LABEL"),
            "transition_by_label": {
                label: (transition_correct_counts[label] / count if count else None)
                for label, count in sorted(transition_counts.items())
            },
            "diagnostics": diagnostics,
        }
    # Keep coverage available both as a cohesive object and as first-class
    # fields for consumers that do not understand nested report sections.
    result.update({key: coverage[key] for key in
                   ("expected", "considered", "missing", "cross_split", "ties", "failures")})
    return result


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


def count_jsonl(path: pathlib.Path) -> int:
    if not path.exists():
        return 0
    return sum(1 for line in path.read_text(encoding="utf-8").splitlines() if line.strip())


def write_manifest(src: pathlib.Path, rows: list[dict]) -> None:
    families = {str(row.get("family")) for row in rows}
    languages = Counter(str(row.get("language") or "unknown") for row in rows)
    labels = Counter("risky" if is_risky(row) else "benign" for row in rows)
    dataset_hash = hashlib.sha256(src.read_bytes()).hexdigest()
    manifest_path = ROOT / "artifacts/guard-dataset-manifest.json"
    catalog_source = ROOT / "crates/engine/guard/src/scenario.rs"
    catalog_hash = file_sha256(catalog_source) if catalog_source.exists() else "unavailable"
    manifest = {
        "dataset_id": "guard-scenario-catalog-v3.3",
        "dataset_version": "v3.3",
        "scenario_count": count_jsonl(ROOT / "scripts/guard_ml/generated_features.jsonl"),
        "action_count": count_jsonl(ROOT / "scripts/guard_ml/action_samples.jsonl"),
        "trace_count": count_jsonl(ROOT / "scripts/guard_ml/trace_samples.jsonl"),
        "pair_count": count_jsonl(ROOT / "scripts/guard_ml/pair_samples.jsonl"),
        "family_count": len(families),
        "language_distribution": dict(languages),
        "class_balance": dict(labels),
        "scenario_catalog_hash": catalog_hash,
        "feature_schema_hash": feature_schema_hash(),
        "generator_commit": git_commit(),
        "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "training_dataset_hash": dataset_hash,
        "note": "Generated by the Rust scenario exporter; train_shadow.py and train_eval.py consume this snapshot without implicit fixture fallback.",
    }
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


def main() -> int:
    random.seed(7)
    src = resolve_path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "scripts/guard_ml/generated_features.jsonl"
    if not src.exists():
        print(f"missing dataset {src}; refusing implicit fixture fallback", file=sys.stderr)
        return 2
    try:
        rows = load_rows(src)
        validate_family_splits(rows)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"invalid dataset {src}: {error}", file=sys.stderr)
        return 2
    write_manifest(src, rows)
    true_split = split_true_group(rows)
    within = split_within_family(rows)
    template_holdout = [row for row in rows if row.get("holdout_group") == "intent_template"]
    tool_holdout = [row for row in rows if row.get("holdout_group") == "tool_shell"]
    # Holdout evaluation must be disjoint from family-baseline fitting. These
    # rows remain in the dataset for reporting, but never influence weights.
    family_train_rows = [row for row in true_split["train"] if not str(row.get("holdout_group") or "")]
    family_calibration_rows = [row for row in true_split["calibration"] if not str(row.get("holdout_group") or "")]
    if len({is_risky(row) for row in family_train_rows}) < 2:
        weights, bias = ([], 0.0)
        training_status = "INSUFFICIENT_SAMPLE_SIZE"
    else:
        weights, bias = train(list(family_train_rows))
        training_status = "OK"
    platt = fit_platt(family_calibration_rows, weights, bias) if weights else None
    b_rows = {f"b{i}": select_rows(rows, f"b{i}") for i in range(1, 11)}
    logistic_true = {
        "validation": metrics(true_split["validation"], weights, bias, platt),
        "test": metrics(true_split["test"], weights, bias, platt),
    }
    report = {
        "source": repo_relative(src),
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
        "family_baseline_fit": {
            "status": training_status,
            "rows": len(family_train_rows),
            "excluded_holdout_rows": len(true_split["train"]) - len(family_train_rows),
            "excluded_holdout_groups": sorted({str(row.get("holdout_group")) for row in true_split["train"]
                                                if str(row.get("holdout_group") or "")}),
            "calibration_rows": len(family_calibration_rows),
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
            "counterfactual": pair_scores(rows, infer_pairs(rows, "counterfactual"), weights, bias, platt, pair_kind="counterfactual"),
            "negation": pair_scores(rows, infer_pairs(rows, "negation"), weights, bias, platt, pair_kind="negation"),
            "fp_by_intent_class": breakdown(true_split["test"], weights, bias, "intent_class", platt),
            "fp_by_category": breakdown(true_split["test"], weights, bias, "category", platt),
            "fp_by_tool_origin": breakdown(true_split["test"], weights, bias, "tool_origin", platt),
            "fp_by_language": breakdown(true_split["test"], weights, bias, "language", platt),
            "fp_by_family": breakdown(true_split["test"], weights, bias, "family", platt),
        },
        "lightgbm": maybe_lightgbm(
            family_train_rows,
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
        "dimension_holdouts": {
            "intent_template": evaluate_dimension_split(rows, "intent_template_id", "intent"),
            "effect_template": evaluate_dimension_split(rows, "action_template_id", "effect"),
            "pair": evaluate_dimension_split(rows, "pair_id", "pair"),
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
        "dataset_path": repo_relative(src),
        "feature_schema_hash": feature_schema_hash(),
        "training_commit": git_commit(),
        "training_commit_is_worktree_head": not git_worktree_dirty(),
        "training_script_version": "train_shadow.py",
        "evaluation_script_version": "train_eval.py",
        "seed": 7,
        "input_hashes": provenance_hashes(src),
    }
    out = ROOT / "artifacts/guard-joint-shadow-v0.metrics.json"
    out.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps({k: report[k] for k in ("train_families", "validation_families", "test_families", "split_counts", "calibration")}, indent=2))
    print(json.dumps(report["logistic"]["unseen_family_holdout"], indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
