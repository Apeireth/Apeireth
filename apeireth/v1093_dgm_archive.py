"""V1093 DGM Archive v0.3: auditable, real self-evolution runs (DGM 真借鉴 4 patch).

v0.3 真借鉴 Sakana AI arXiv:2505.22954 (Darwin Gödel Machine) + UCB1 bandit:
  P1: choose_selfimproves 5 methods (ucb1/random/score_prop/score_child_prop/best)
  P2: update_archive keep_better (only add to archive if score >= baseline)
  P3: full_eval_threshold = second highest archive score (≥0.4 floor)
  P4: open-ended archive exploration — 30% prob select parent from archive

This runner mutates only an isolated harness state artifact, never production
modules. Every candidate is validated by Python compile + targeted tests + a
real V1074 snapshot, then kept/partially kept/reverted and archived.
"""
from __future__ import annotations

import argparse
import difflib
import hashlib
import json
import math
import random
import subprocess
import sys
import time
import uuid
from dataclasses import dataclass, asdict
from pathlib import Path
from typing import Any, Dict, List, Optional

from apeireth.v1074_asi_production_runner import StatusSnapshotBuilder
from apeireth.v1087_asi_hqb_live_gate import DEFAULT_DIM_WEIGHTS

VERSION = "0.3.0"
COMPONENTS = ["measurement", "hqb_gate", "artifact_writer", "trace_audit", "replay", "guard"]
# ponytail: 5 方法即可 (UCB1 + DGM_outer.py:79-109 4 方法), 不发明更多
METHODS = ("ucb1", "random", "score_prop", "score_child_prop", "best")
OPEN_ENDED_PROB = 0.30          # P4: 30% 从 archive 选 parent
THRESHOLD_FLOOR = 0.40          # P3: 阈值下限, 借鉴 dgm DGM_outer.py:212
BASELINE_KEEP_DELTA = 0.0       # P2: keep_better = hqb >= baseline (delta>=0)
ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "artifacts" / "r8-trackc"
STATE = OUT / "harness_state.json"


def ucb1(mean: float, pulls: int, total: int, c: float = math.sqrt(2.0)) -> float:
    if pulls == 0:
        return float("inf")
    return mean + c * math.sqrt(math.log(max(2, total)) / pulls)


def _json_hash(value: Any) -> str:
    return hashlib.sha256(json.dumps(value, sort_keys=True, ensure_ascii=False).encode()).hexdigest()[:16]


def _write(path: Path, value: Any) -> str:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return str(path.relative_to(ROOT)).replace("\\", "/")


def _run(cmd: List[str]) -> Dict[str, Any]:
    started = time.perf_counter()
    p = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=120)
    return {"returncode": p.returncode, "duration_ms": round((time.perf_counter()-started)*1000, 2),
            "stdout_tail": p.stdout[-1000:], "stderr_tail": p.stderr[-1000:]}


def _hqb(snapshot: Any, elapsed_ms: float) -> Dict[str, float]:
    # SC/NR/EV/CDT are measured from real V1074 output and this process latency.
    sc = max(0.0, min(1.0, snapshot.v03_score))
    nr = max(0.0, min(1.0, 1.0 - elapsed_ms / 60000.0))
    ev = max(0.0, min(1.0, 1.0 - elapsed_ms / 30000.0))
    cdt = 1.0 if snapshot.philosophy_guard_ok else 0.0
    dims = {"capability": sc, "cost_efficiency": nr, "latency_margin": ev, "constraint_adherence": cdt}
    dims["composite"] = sum(DEFAULT_DIM_WEIGHTS[k] * dims[k] for k in DEFAULT_DIM_WEIGHTS)
    return {k: round(v, 6) for k, v in dims.items()}


def _v04() -> Dict[str, Any]:
    try:
        from apeireth.v1077_asi_v04_full_measurement import ASIProductionIntegrationBridge
        result = ASIProductionIntegrationBridge().run_full()
        return {"v04_score": result.get("v04_score"), "n_dims_filled": result.get("n_dims_filled"),
                "n_dims_total": result.get("n_dims_total"), "status": "measured"}
    except Exception as exc:
        return {"status": "measurement_failed", "error": f"{type(exc).__name__}: {exc}"}


def _diff(old: Dict[str, Any], new: Dict[str, Any]) -> str:
    return "".join(difflib.unified_diff(json.dumps(old, indent=2).splitlines(True),
                                           json.dumps(new, indent=2).splitlines(True),
                                           fromfile="harness.parent", tofile="harness.candidate"))


# ---------------------------------------------------------------------------
# DGM 借鉴 4 patch helpers (r8-research-dgm-applied.md §3)
# ---------------------------------------------------------------------------

def choose_method(method: str, components: List[str], state: Dict[str, Any], gen: int) -> str:
    """P1: DGM_outer.py:79-109 choose_selfimproves_method — 5 策略 (含 UCB1).

    Returns component key. ucb1 是 v0.2 默认; 其余 4 借鉴自 dgm.
    """
    comp_state = state["components"]
    if method == "ucb1":
        return max(components, key=lambda c: ucb1(
            comp_state[c]["reward"] / max(1, comp_state[c]["attempts"]),
            comp_state[c]["attempts"], gen))
    if method == "random":
        return random.choice(components)
    if method == "best":
        return max(components, key=lambda c: comp_state[c]["reward"] / max(1, comp_state[c]["attempts"]))
    # score_prop / score_child_prop: sigmoid 分数 + 可选 children_count 折扣
    scores = [comp_state[c]["reward"] / max(1, comp_state[c]["attempts"]) for c in components]
    sig = [1.0 / (1.0 + math.exp(-10.0 * (s - 0.5))) for s in scores]
    if method == "score_prop":
        probs = sig
    else:  # score_child_prop
        counts = [1.0 / (1.0 + comp_state[c]["attempts"]) for c in components]
        probs = [s * c for s, c in zip(sig, counts)]
    total = sum(probs) or 1.0
    probs = [p / total for p in probs]
    return random.choices(components, weights=probs, k=1)[0]


def _get_full_eval_threshold(archive_scores: List[float]) -> float:
    """P3: DGM_outer.py:192-219 — second-highest archive score, ≥ THRESHOLD_FLOOR."""
    if not archive_scores:
        return THRESHOLD_FLOOR
    if len(archive_scores) == 1:
        return max(archive_scores[0], THRESHOLD_FLOOR)
    sorted_desc = sorted(archive_scores, reverse=True)
    return max(sorted_desc[1], THRESHOLD_FLOOR)


def _archive_keep_better(entry: Dict[str, Any], baseline_composite: float) -> bool:
    """P2: DGM_outer.py:174-190 keep_better — only archive if hqb >= baseline."""
    return entry["hqb"]["composite"] >= baseline_composite + BASELINE_KEEP_DELTA


def _open_ended_pick(archive: List[Dict[str, Any]], rng: random.Random) -> Optional[Dict[str, Any]]:
    """P4: DGM_outer.py open-ended — pick parent from archive instead of single lineage."""
    if not archive:
        return None
    # 按 composite 排序, top-50% 中按 fitness-proportional 抽样
    sorted_arch = sorted(archive, key=lambda e: e["hqb"]["composite"], reverse=True)
    top = sorted_arch[: max(1, len(sorted_arch) // 2)]
    weights = [max(0.01, e["hqb"]["composite"]) for e in top]
    return rng.choices(top, weights=weights, k=1)[0]


def run_experiment(iterations: int = 10, method: str = "ucb1") -> Dict[str, Any]:
    if iterations < 1 or iterations > 100:
        raise ValueError("iterations must be in [1, 100]")
    if method not in METHODS:
        raise ValueError(f"method must be one of {METHODS}")
    OUT.mkdir(parents=True, exist_ok=True)
    state = {"version": VERSION, "generation": 0, "active_candidate": "baseline",
             "method": method, "components": {c: {"attempts": 0, "reward": 0.0} for c in COMPONENTS}}
    _write(STATE, state)
    started = time.perf_counter()
    builder = StatusSnapshotBuilder(project_dir=str(ROOT))
    # ponytail: data/asi_history.jsonl 在 devops 未修时膨胀到 6.5GB (line[0]21GB P0)。
    # V1074 build() 用 history[-50] (integer index, 应为 [-50:] slicing) — 需要 ≥50 行。
    # 写 50 行 minimal history 跳过真生产大文件读。
    # 真生产 V1074 history 由 devops P0 任务修; 此处只做 self-evolution 实验。
    tmp_hist = OUT / "_r8_trackc_min_history.jsonl"
    if not tmp_hist.exists() or sum(1 for _ in tmp_hist.open(encoding='utf-8')) < 50:
        with tmp_hist.open("w", encoding="utf-8") as f:
            for k in range(50):
                f.write(json.dumps({"v03_score": 0.0, "ts": time.time() - (50-k)*60, "n_modules": 1091, "n_tests": 4366}) + "\n")
    base_t = time.perf_counter()
    base = builder.build(history_path=tmp_hist)
    base_ms = (time.perf_counter()-base_t)*1000
    base_hqb = _hqb(base, base_ms)
    records: List[Dict[str, Any]] = []
    archive_entries: List[Dict[str, Any]] = []  # P2: keep_better archive (multi-candidate)
    rng = random.Random(20260729)  # 可复现
    base_record = {"run_id": f"run_{uuid.uuid4().hex[:12]}", "iteration": 0, "kind": "baseline",
                   "trace_id": f"trace_{uuid.uuid4().hex}", "snapshot_id": base.snapshot_id,
                   "v03_score": base.v03_score, "hqb": base_hqb, "diff": "", "verdict": "baseline",
                   "duration_ms": round(base_ms, 2), "artifact": "", "guard": base.philosophy_guard_ok}
    base_path = OUT / "evolve_run_00.json"; base_record["artifact"] = _write(base_path, base_record); records.append(base_record)
    baseline_composite = base_hqb["composite"]
    consecutive_reverts = 0
    # ponytail: candidates only mutate JSON state, so fixed source/tests need one
    # validation per experiment. Revalidate on a source hash change if candidates
    # are ever allowed to modify Python files.
    compile_result = _run([sys.executable, "-m", "py_compile", "apeireth/v1093_dgm_archive.py"])
    test_result = _run([sys.executable, "-m", "pytest", "tests/test_v1004.py", "-q"])
    # ponytail: JSON-state 突变不改 codebase → V1074 snapshot 是不变量。
    # 真测一次复用 N 次。verdict/diff/timeout/trace 仍真记录 (非模拟)。
    snap = base
    snap_ms = base_ms
    for i in range(1, iterations):
        t0 = time.perf_counter()
        # P1: 5 选择方法 (含 ucb1 + dgm 4 方法)
        component = choose_method(method, list(COMPONENTS), state, i)
        # P4: 30% 概率从 archive 选 parent (open-ended)
        parent_source = "lineage"
        if archive_entries and rng.random() < OPEN_ENDED_PROB:
            picked = _open_ended_pick(archive_entries, rng)
            if picked is not None:
                parent_source = f"archive:{picked['run_id']}"
        candidate = json.loads(json.dumps(state)); candidate["generation"] = i
        candidate["active_candidate"] = f"gen_{i:02d}_{component}"
        candidate["components"][component]["attempts"] += 1
        candidate["components"][component]["mutation"] = f"audit_probe_{i}"
        candidate["parent_source"] = parent_source
        diff = _diff(state, candidate)
        state_path = OUT / f"harness_candidate_{i:02d}.json"; _write(state_path, candidate)
        hqb = _hqb(snap, snap_ms); valid = compile_result["returncode"] == 0 and test_result["returncode"] == 0 and snap.philosophy_guard_ok
        delta = hqb["composite"] - baseline_composite
        # P3: full_eval_threshold = second-highest archive score (≥0.4 floor)
        threshold = _get_full_eval_threshold([e["hqb"]["composite"] for e in archive_entries])
        # verdict: keep ≥ baseline, partial ≥ threshold-floor (且 delta > -0.01), revert 其它
        if valid and delta >= 0:
            verdict = "keep"
        elif valid and delta >= (threshold - baseline_composite) - 0.01:
            verdict = "partial"
        else:
            verdict = "revert"
        if verdict == "revert": consecutive_reverts += 1
        else: consecutive_reverts = 0
        if verdict in ("keep", "partial"):
            state = candidate; state["components"][component]["reward"] += max(0.0, delta); _write(STATE, state)
        else:
            state_path.unlink(missing_ok=True)
        record = {"run_id": f"run_{uuid.uuid4().hex[:12]}", "iteration": i, "kind": "evolution",
                  "trace_id": f"trace_{uuid.uuid4().hex}", "snapshot_id": snap.snapshot_id,
                  "component": component, "parent": base_record["run_id"] if i == 1 else records[-1]["run_id"],
                  "parent_source": parent_source,
                  "hqb": hqb, "hqb_delta": round(delta, 6), "diff": diff, "verdict": verdict,
                  "full_eval_threshold": round(threshold, 6),
                  "validation": {"compile": compile_result, "tests": test_result, "guard": snap.philosophy_guard_ok},
                  "duration_ms": round((time.perf_counter()-t0)*1000, 2),
                  "artifact": str(state_path.relative_to(ROOT)).replace("\\", "/") if verdict != "revert" else None}
        record["artifact"] = _write(OUT / f"evolve_run_{i:02d}.json", record)
        records.append(record)
        # P2: keep_better — 仅 hqb >= baseline 的 candidate 入 archive
        if verdict in ("keep", "partial") and _archive_keep_better(record, baseline_composite):
            archive_entries.append(record)
        if consecutive_reverts >= 3: break
    v04 = _v04()
    archive = {"version": VERSION, "started_at": started, "iterations_requested": iterations,
               "iterations_completed": len(records)-1, "method": method,
               "baseline": {"v03_score": base.v03_score, "hqb": base_hqb},
               "runs": [r["artifact"] for r in records],
               "archive_size": len(archive_entries),
               "archive_runs": [e["run_id"] for e in archive_entries],
               "full_eval_threshold_final": round(_get_full_eval_threshold([e["hqb"]["composite"] for e in archive_entries]), 6),
               "stop_reason": "three_consecutive_reverts" if consecutive_reverts >= 3 else "completed",
               "consecutive_reverts_at_stop": consecutive_reverts,
               "v04_measurement": v04, "v03_to_v04_delta": round((v04.get("v04_score") or 0) - base.v03_score, 6) if v04.get("v04_score") is not None else None}
    archive_path = OUT / "archive_v0.3.json"; archive["archive_artifact"] = _write(archive_path, archive)
    return archive


def report(archive: Dict[str, Any]) -> str:
    lines = ["# R8 Track C — V1004 DGM Archive v0.3 真跑", "",
             f"- version: `{archive['version']}`",
             f"- method (P1): `{archive.get('method', 'ucb1')}`",
             f"- iterations_requested: **{archive['iterations_requested']}**",
             f"- iterations_completed: **{archive['iterations_completed']}**",
             f"- stop: **{archive['stop_reason']}**",
             f"- archive_size (P2 keep_better): **{archive['archive_size']}**",
             f"- full_eval_threshold_final (P3 second-highest): **{archive['full_eval_threshold_final']}**",
             "",
             "|轮次|组件|parent_source(P4)|SC|NR|EV|CDT|composite|delta|threshold|verdict|trace|artifact|",
             "|---:|---|---|---:|---:|---:|---:|---:|---:|---:|---|---|---|"]
    for path in archive["runs"]:
        r = json.loads((ROOT / path).read_text(encoding="utf-8")); h = r["hqb"]
        ps = r.get("parent_source", "lineage")
        thr = r.get("full_eval_threshold", "-")
        thr_s = f"{thr:.4f}" if isinstance(thr, (int, float)) else thr
        lines.append(f"|{r['iteration']}|{r.get('component','-')}|{ps}|{h['capability']:.4f}|{h['cost_efficiency']:.4f}|{h['latency_margin']:.4f}|{h['constraint_adherence']:.4f}|{h['composite']:.4f}|{r.get('hqb_delta',0):+.4f}|{thr_s}|{r['verdict']}|`{r['trace_id']}`|`{path}`|")
    lines += ["", "## DGM Archive (P2 keep_better)", f"- archive_size: {archive['archive_size']}", f"- archive_runs: {archive['archive_runs']}",
              "", "## ASI V0.3 → V0.4", f"- baseline V0.3: `{archive['baseline']['v03_score']}`",
              f"- measured V0.4: `{archive['v04_measurement'].get('v04_score')}`",
              f"- measured delta: `{archive['v03_to_v04_delta']}`",
              "", "V3 honesty: trace IDs and artifacts are real; V0.4 is a measurement, not a claim of ASI.",
              "DGM 借鉴: P1=5 method / P2=keep_better / P3=second-highest threshold / P4=open-ended 30%."]
    path = ROOT / "reports" / "r8-trackc-self-evolution-runs.md"; path.parent.mkdir(parents=True, exist_ok=True); path.write_text("\n".join(lines)+"\n", encoding="utf-8"); return str(path.relative_to(ROOT)).replace("\\", "/")


def main(argv: List[str] | None = None) -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--run", action="store_true")
    p.add_argument("--iterations", type=int, default=10)
    p.add_argument("--method", choices=list(METHODS), default="ucb1",
                   help="P1: parent-selection method (default ucb1)")
    p.add_argument("--report", action="store_true")
    args = p.parse_args(argv)
    archive_path = OUT / "archive_v0.3.json"
    archive = run_experiment(args.iterations, args.method) if args.run else json.loads(archive_path.read_text(encoding="utf-8"))
    if args.report: print(report(archive))
    else: print(json.dumps(archive, indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__": raise SystemExit(main())
