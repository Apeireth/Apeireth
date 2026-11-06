#!/usr/bin/env python3
"""Summarize independent startup receipts; proposed budgets never replace the old gate.

Usage: python scripts/startup_report.py --logs <logs...> --environment env.json
       --legacy-dev legacy.log --out report.json [--baseline previous-report.json]
Regression comparison requires matching environment, fixture, profile and sample n.
Missing cold-cache evidence remains unavailable, never a zero-duration pass.
"""
import argparse
import json
import math
from pathlib import Path

MODES = ("first_run", "normal_start", "cold_start", "warm_resume")
# Advisory starting budgets (P50/P95 ms), explicitly unapproved. Cold-cache
# budgets need calibration on a runner that can actually drop kernel caches.
BUDGETS = {
    "Dev": {"first_run": (30, 60), "normal_start": (10, 20),
            "cold_start": (25, 50), "warm_resume": (1, 2)},
    "Release": {"first_run": (20, 40), "normal_start": (10, 20),
                "cold_start": (20, 40), "warm_resume": (1, 2)},
}


def parse_logs(paths):
    records, unavailable = {}, {}
    for path in paths:
        profile = None
        for line in Path(path).read_text().splitlines():
            fields = line.split("|")
            if fields[0] == "STARTUP_META":
                profile = fields[2]
                if "fixture=session-pair-v1" not in fields:
                    raise ValueError("unsupported fixture")
            elif fields[0] == "STARTUP_UNAVAILABLE":
                unavailable[f"{profile}/{fields[1]}"] = "|".join(fields[2:])
            elif fields[0] == "STARTUP":
                if len(fields) != 10 or fields[2] != profile:
                    raise ValueError(f"invalid startup receipt: {line}")
                key = "/".join((fields[2], fields[1], fields[3]))
                n = int(fields[4])
                p50, p95, minimum, mean, maximum = map(float, fields[5:])
                if n <= 0 or not all(math.isfinite(v) and v >= 0 for v in (p50, p95, minimum, mean, maximum)):
                    raise ValueError(f"invalid stats: {line}")
                if not minimum <= p50 <= p95 <= maximum or not minimum <= mean <= maximum:
                    raise ValueError(f"unordered stats: {line}")
                if key in records:
                    raise ValueError(f"duplicate measurement: {key}")
                records[key] = dict(n=n, p50_ms=p50, p95_ms=p95,
                                    min_ms=minimum, mean_ms=mean, max_ms=maximum)
    for key in unavailable:
        if any(name.startswith(key + "/") for name in records):
            raise ValueError(f"partial cache-drop failure cannot supply valid samples: {key}")
    return records, unavailable


def legacy(path):
    for line in Path(path).read_text().splitlines():
        if line.startswith("ROW|cold-start|"):
            f = line.split("|")
            return dict(n=int(f[4]), p50_ms=float(f[9]), verdict=f[13], log=str(path))
    raise ValueError("legacy cold-start receipt missing")


def compare(before, after):
    # Commit SHA is deliberately excluded: this is a cross-change comparison.
    env_keys = ("platform", "cpu", "memory", "toolchain", "filesystem")
    if any(before["environment"][k] != after["environment"][k] for k in env_keys):
        raise ValueError("environment changed; do not attribute a regression")
    if before["fixture"] != after["fixture"]:
        raise ValueError("fixture changed")
    regressions, missing = [], []
    for profile in BUDGETS:
        for mode in MODES:
            key = f"{profile}/{mode}/total"
            a, b = before["measurements"].get(key), after["measurements"].get(key)
            if a is None or b is None:
                missing.append(key)
                continue
            if a["n"] != b["n"]:
                raise ValueError(f"sample count changed: {key}")
            for metric, ratio, floor in (("p50_ms", .20, 1.), ("p95_ms", .30, 2.)):
                delta = b[metric] - a[metric]
                if delta > max(a[metric] * ratio, floor):
                    regressions.append(dict(key=key, metric=metric, before=a[metric], after=b[metric]))
    return dict(state="REGRESSION" if regressions else "INCOMPLETE" if missing else "NO_REGRESSION",
                regressions=regressions, unavailable_comparisons=missing)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--logs", nargs="+", required=True)
    parser.add_argument("--environment", required=True)
    parser.add_argument("--legacy-dev", required=True)
    parser.add_argument("--baseline")
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    records, unavailable = parse_logs(args.logs)
    old = legacy(args.legacy_dev)
    report = dict(fixture="session-pair-v1", environment=json.loads(Path(args.environment).read_text()),
                  measurements=records, unavailable=unavailable, legacy_dev=old,
                  formal_acceptance="PARTIAL" if old["p50_ms"] >= 10 else "OLD_STARTUP_GATE_MET_ONLY",
                  target_approval="PROPOSED_UNAPPROVED", proposed_budgets_ms=BUDGETS)
    report["proposed_results"] = {
        f"{profile}/{mode}": (
            "UNAVAILABLE" if f"{profile}/{mode}/total" not in records else
            "WITHIN_PROPOSED_BUDGET" if records[f"{profile}/{mode}/total"]["p50_ms"] <= budget[0]
            and records[f"{profile}/{mode}/total"]["p95_ms"] <= budget[1] else "EXCEEDS_PROPOSED_BUDGET")
        for profile, modes in BUDGETS.items() for mode, budget in modes.items()}
    if args.baseline:
        report["comparison"] = compare(json.loads(Path(args.baseline).read_text()), report)
    Path(args.out).write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({key: report[key] for key in ("formal_acceptance", "target_approval", "unavailable", "proposed_results")}, indent=2))
    comparison = report.get("comparison", {})
    if comparison:
        print(json.dumps(comparison, indent=2))
    return {"REGRESSION": 1, "INCOMPLETE": 2}.get(comparison.get("state"), 0)


if __name__ == "__main__":
    raise SystemExit(main())
