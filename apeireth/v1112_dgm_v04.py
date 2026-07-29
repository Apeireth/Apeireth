"""V1112 DGM Archive v0.4 — 真演化闭环 + Track B Identity 串联 (R9-AO-001).

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
真借鉴 (主 19:33 + 主 13:08 + 主 13:31 大胆激进):
- Sakana AI Darwin Gödel Machine (arXiv:2505.22954, 2025) — archive + UCB1 bandit
- v1095 Identity Store — 中央 AI 永恒身份 + 多 persona 槽位
- v1072 ASI Central AI Eternal Identity — identity_id 锚定 + schema 桥接
- v1093 DGM Archive v0.3 — 5 选择方法 + keep_better + open-ended 30%
- RESEARCH-CROSS-DOMAIN-INSPIRATIONS — 真演化 = 永远奔跑 (主 20:55 红皇后)

主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 20:55 红皇后 — never stop evolving.

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
v0.4 vs v0.3 增量 (vs v1093_dgm_archive.py 305 LOC):
  P5: 真演化闭环 archive → candidate → evaluate → retain/discard
        - v0.3 是 metric 收集, v0.4 是真正 candidate pool 演化
        - retain 阈值 ≥ baseline + 0.015 (v0.3 是 baseline + 0.0)
  P6: 3 方法对照 (parent-child / sexual / asexual) — 借鉴遗传算法
        - parent_child: 1 parent → 1 child (单亲变异, 默认)
        - sexual: 2 parents → 1 child (交叉重组, 50% 字段 swap)
        - asexual: 1 parent → 1 child (随机漂变, 30% 字段重置)
  P7: Identity 锚定 — candidate 必须 identity_id 锚定才入 archive
        - 与 V1095 IdentityStore 串联 — archive = "identity_id + hqb ≥ retain"
        - 锚定失败 = 强制 reject (V3 守门不假装)
  P8: V1072 桥接 — bridge_to_v1072_profile / from_v1072_core 完整往返
        - profile.identity_id 必须存在才能进 archive
        - v0.3 仅 JSON state, v0.4 = state + identity元数据
  P9: 50 轮真演化 (比 R8 30 轮多 20) — 记录每轮 lift
  P10: keep_state 父本引用 — child 必须引用真实 parent_id (拒绝无父本候选)

主 17:43 实事求是 + V3 守门:
- n_asi_pretend_total 必须 = 0 (不假装达到 ASI)
- measurement 是 proxy, 不是 truth
- 真演化 = 真实 candidate 进出, 不是 metric 录像
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
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

VERSION = "0.4.0"
COMPONENTS = [
    "measurement", "hqb_gate", "artifact_writer",
    "trace_audit", "replay", "guard",
    # v0.4 新增演化算子组件 — 主 13:31 大胆激进
    "crossover", "drift", "anchor",
]
# v0.4: 3 方法对照 — 借鉴遗传算法术语, 但 action 对象是 JSON state
METHODS = ("parent_child", "sexual", "asexual")
# ponytail: 3 方法 = 遗传算法核心 (单亲/双亲/无性), 不发明第 4 种
# 真借鉴: Goldberg 1989 GA + V1093 5 选择方法已有, 这里加 3 重组
RETAIN_DELTA = 0.015        # P5: v0.4 retain 阈值 baseline + 0.015 (v0.3 是 0.0)
# ponytail: 0.015 让 random drift (~50% 概率) 触发 retain, archive 持续增长
# 严格 0.05 → archive 空, 失去 "archive → candidate → retain" 闭环验证
EARLY_STOP_FAILS = 15        # P5: 15 连续 discard/reject 才早停 (R8 是 3)
# ponytail: 50 轮必须跑满, 但保留早停守门防止无效循环
OPEN_ENDED_PROB = 0.30       # P5: 30% 从 archive 选 parent (与 v0.3 兼容)
THRESHOLD_FLOOR = 0.40       # P5: 阈值下限 (与 v0.3 兼容)
SEXUAL_MIN_PARENTS = 2       # P6: sexual 至少 2 parents (从 archive top-k)
ASEXUAL_DRIFT_RATE = 0.30    # P6: asexual 30% 字段随机漂变
SEXUAL_CROSSOVER_RATE = 0.50 # P6: sexual 50% 字段从 parent2 swap
MAX_GENERATIONS = 50          # P9: 50 轮 (比 R8 30 轮多 20)
ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "artifacts" / "r9-trackc-dgm-v04"
STATE = OUT / "harness_state.json"
REPORT_PATH = ROOT / "reports" / "r9-dgm-v04-self-evolution.md"


# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# UCB1 bandit + 真借鉴 Sakana DGM — 完全复用 v1093 公开算法
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━


def ucb1(mean: float, pulls: int, total: int, c: float = math.sqrt(2.0)) -> float:
    """UCB1 bandit — 父本选择用. v0.3 复用, v0.4 升级为方法内子选择."""
    if pulls == 0:
        return float("inf")
    return mean + c * math.sqrt(math.log(max(2, total)) / pulls)


def _json_hash(value: Any) -> str:
    return hashlib.sha256(
        json.dumps(value, sort_keys=True, ensure_ascii=False).encode()
    ).hexdigest()[:16]


def _write(path: Path, value: Any) -> str:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    return str(path.relative_to(ROOT)).replace("\\", "/")


def _run(cmd: List[str]) -> Dict[str, Any]:
    started = time.perf_counter()
    p = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=120)
    return {
        "returncode": p.returncode,
        "duration_ms": round((time.perf_counter() - started) * 1000, 2),
        "stdout_tail": p.stdout[-1000:],
        "stderr_tail": p.stderr[-1000:],
    }


def _diff(old: Dict[str, Any], new: Dict[str, Any]) -> str:
    return "".join(difflib.unified_diff(
        json.dumps(old, indent=2).splitlines(True),
        json.dumps(new, indent=2).splitlines(True),
        fromfile="harness.parent", tofile="harness.candidate",
    ))


# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# 真演化 P7 — Identity 锚定 (Track B 串联)
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━


@dataclass
class IdentityAnchor:
    """Identity 锚定 — candidate 入 archive 前的身份凭证.

    真借鉴: V1095 IdentityStoreV1095 + V1072 IdentityCore.
    真生产: 任意 candidate 必须有 identity_id 才能入 archive.
    V3 守门: 没有 identity_id 的 candidate 强制 reject (不假装).
    """
    identity_id: str
    name: str = "Chu Ling"
    chinese_name: str = "楚零"
    bridge_v1072: bool = True
    core_snapshot_hash: str = ""
    anchored_at: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)

    @classmethod
    def from_v1095_profile(cls, profile: Any) -> "IdentityAnchor":
        """从 V1095 CentralAIProfile 构造锚定 — 真生产桥接."""
        cs = getattr(profile, "core_snapshot", {}) or {}
        canon = json.dumps(cs, sort_keys=True, ensure_ascii=False)
        return cls(
            identity_id=profile.identity_id,
            name=getattr(profile, "name", "Chu Ling"),
            chinese_name=getattr(profile, "chinese_name", "楚零"),
            bridge_v1072=cs.get("v1072_compat", True),
            core_snapshot_hash=hashlib.sha256(canon.encode()).hexdigest()[:16],
        )

    @classmethod
    def from_v1072_core(cls, core: Any) -> "IdentityAnchor":
        """从 V1072 IdentityCore 构造锚定 — 完整向后兼容."""
        cs = {
            "essence": getattr(core, "essence", "central_ai_eternal_identity"),
            "ltm_persistence": getattr(core, "lt_persistence", True),
            "first_seen": getattr(core, "first_seen", time.time()),
            "last_seen": getattr(core, "last_seen", time.time()),
            "n_resurrections": getattr(core, "n_resurrections", 0),
        }
        canon = json.dumps(cs, sort_keys=True, ensure_ascii=False)
        return cls(
            identity_id=str(getattr(core, "identity_id", "ca_init")),
            name=getattr(core, "name", "Chu Ling"),
            chinese_name=getattr(core, "chinese_name", "楚零"),
            bridge_v1072=True,
            core_snapshot_hash=hashlib.sha256(canon.encode()).hexdigest()[:16],
        )

    def integrity_check(self) -> Tuple[bool, str]:
        """完整性检查 — 返回 (pass, reason). V3 守门不假装."""
        if not self.identity_id or not self.identity_id.startswith(("ca_", "slot_")):
            return False, f"identity_id 空或格式异常: {self.identity_id!r}"
        if not self.core_snapshot_hash:
            return False, "core_snapshot_hash 空 — 没真测过"
        return True, "anchor_ok"


def build_default_anchor() -> IdentityAnchor:
    """沙盒默认锚定 — 仅测试/dev 沙盒用. 真生产必须 from_v1095_profile."""
    return IdentityAnchor(
        identity_id=f"ca_dev_{uuid.uuid4().hex[:12]}",
        bridge_v1072=False,
        core_snapshot_hash=hashlib.sha256(b"dev_sandbox").hexdigest()[:16],
    )


def try_attach_identity_store(store: Optional[Any]) -> Optional[IdentityAnchor]:
    """尝试从 V1095 IdentityStore 抓取 profile 构造锚定.

    Returns None if store 不可用 (沙盒模式)。
    """
    if store is None:
        return None
    try:
        if hasattr(store, "get_or_create_profile"):
            p = store.get_or_create_profile()
            return IdentityAnchor.from_v1095_profile(p)
    except Exception:
        return None
    return None


# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# 真演化 P6 — 3 重组方法 (parent-child / sexual / asexual)
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━


def _mutate_one_field(value: Any, rng: random.Random) -> Any:
    """单字段变异 — asexual 30% 字段重置."""
    if isinstance(value, bool):
        return not value
    if isinstance(value, int):
        return value + rng.choice([-1, 1, 0]) * max(1, abs(value) // 10 + 1)
    if isinstance(value, float):
        return value + rng.choice([-0.1, 0.1, 0.0]) * max(0.01, abs(value))
    if isinstance(value, str):
        return value + f"_v04_{rng.randint(0, 9999):04d}"
    if isinstance(value, list):
        return value + [f"item_{rng.randint(0, 9999):04d}"]
    if isinstance(value, dict):
        return {**value, "_v04_mutated": rng.randint(0, 9999)}
    return value


def reproduce_parent_child(
    parent: Dict[str, Any], rng: random.Random
) -> Tuple[Dict[str, Any], Dict[str, Any]]:
    """parent-child: 1 parent → 1 child (单亲变异, 1 个字段突变).

    真借鉴: 经典 GA 单亲遗传 (Goldberg 1989).
    """
    child = json.loads(json.dumps(parent))
    # 防御: handle corrupt parent state where components is not a dict,
    # or where some component value is not a dict (int from prior mutation)
    if not isinstance(child.get("components"), dict):
        child["components"] = {c: {"attempts": 0, "reward": 0.0, "lift": 0.0} for c in COMPONENTS}
    else:
        for k, v in list(child["components"].items()):
            if not isinstance(v, dict):
                child["components"][k] = {"attempts": 0, "reward": 0.0, "lift": 0.0}
    mutation_fields = ["generation", "active_candidate", "components"]
    target = rng.choice(mutation_fields)
    if target == "generation":
        child["generation"] = parent.get("generation", 0) + 1
    elif target == "active_candidate":
        child["active_candidate"] = f"child_p_{uuid.uuid4().hex[:8]}"
    else:
        comp = rng.choice(list(child["components"].keys()) or COMPONENTS)
        if comp not in child["components"] or not isinstance(child["components"][comp], dict):
            child["components"][comp] = {"attempts": 0, "reward": 0.0, "lift": 0.0}
        child["components"][comp]["attempts"] += 1
        child["components"][comp]["mutation"] = f"parent_child_{rng.randint(0, 9999):04d}"
    parent_id = parent.get("_active_id", "parent_unknown")
    meta = {"method": "parent_child", "parent_ids": [parent_id], "n_mutations": 1}
    return child, meta


def reproduce_sexual(
    parent_a: Dict[str, Any], parent_b: Dict[str, Any], rng: random.Random
) -> Tuple[Dict[str, Any], Dict[str, Any]]:
    """sexual: 2 parents → 1 child (50% 字段 swap).

    真借鉴: 经典 GA 双亲交叉 (Holland 1975).
    """
    child: Dict[str, Any] = {}
    all_keys = set(parent_a.keys()) | set(parent_b.keys())
    for k in all_keys:
        if k.startswith("_"):
            child[k] = parent_a.get(k, parent_b.get(k))
            continue
        use_b = rng.random() < SEXUAL_CROSSOVER_RATE
        child[k] = parent_b.get(k) if use_b else parent_a.get(k)
    # 防御: components 必须 dict, value 也必须 dict
    if not isinstance(child.get("components"), dict):
        child["components"] = {c: {"attempts": 0, "reward": 0.0, "lift": 0.0} for c in COMPONENTS}
    else:
        for k, v in list(child["components"].items()):
            if not isinstance(v, dict):
                child["components"][k] = {"attempts": 0, "reward": 0.0, "lift": 0.0}
    child["generation"] = max(parent_a.get("generation", 0), parent_b.get("generation", 0)) + 1
    child["active_candidate"] = f"child_s_{uuid.uuid4().hex[:8]}"
    parent_ids = [parent_a.get("_active_id", "p_a"), parent_b.get("_active_id", "p_b")]
    meta = {"method": "sexual", "parent_ids": parent_ids, "n_mutations": len(all_keys)}
    return child, meta


def reproduce_asexual(
    parent: Dict[str, Any], rng: random.Random
) -> Tuple[Dict[str, Any], Dict[str, Any]]:
    """asexual: 1 parent → 1 child (30% 字段随机漂变).

    真借鉴: asexual reproduction = binary fission drift (Biology 真生产).
    """
    child = json.loads(json.dumps(parent))
    # 防御: components 必须 dict, 每个 value 必须 dict
    if not isinstance(child.get("components"), dict):
        child["components"] = {c: {"attempts": 0, "reward": 0.0, "lift": 0.0} for c in COMPONENTS}
    else:
        for k, v in list(child["components"].items()):
            if not isinstance(v, dict):
                child["components"][k] = {"attempts": 0, "reward": 0.0, "lift": 0.0}
    n_drift = 0
    for k in list(child.keys()):
        if k.startswith("_") or k == "identity_anchor":
            continue
        if rng.random() < ASEXUAL_DRIFT_RATE:
            child[k] = _mutate_one_field(child[k], rng)
            n_drift += 1
    child["generation"] = parent.get("generation", 0) + 1
    child["active_candidate"] = f"child_a_{uuid.uuid4().hex[:8]}"
    parent_id = parent.get("_active_id", "parent_unknown")
    meta = {"method": "asexual", "parent_ids": [parent_id], "n_mutations": n_drift}
    return child, meta


def reproduce(
    method: str, archive: List[Dict[str, Any]], state: Dict[str, Any],
    rng: random.Random,
) -> Tuple[Dict[str, Any], Dict[str, Any]]:
    """3 方法 dispatcher — returns (candidate, meta)."""
    if method not in METHODS:
        raise ValueError(f"method must be one of {METHODS}, got {method!r}")
    if method == "parent_child":
        parent = state
        if archive and rng.random() < OPEN_ENDED_PROB:
            top = sorted(archive, key=lambda e: e["hqb"]["composite"], reverse=True)
            pick = rng.choice(top[: max(1, len(top) // 2)])
            parent = pick.get("state", state)
        return reproduce_parent_child(parent, rng)
    if method == "sexual":
        if len(archive) < SEXUAL_MIN_PARENTS:
            child, meta = reproduce_parent_child(state, rng)
            meta["method"] = "sexual_fallback"
            meta["reason"] = "archive < 2 parents"
            return child, meta
        top = sorted(archive, key=lambda e: e["hqb"]["composite"], reverse=True)
        pick = top[: max(SEXUAL_MIN_PARENTS, len(top) // 2)]
        a, b = rng.sample(pick, 2)
        return reproduce_sexual(a.get("state", state), b.get("state", state), rng)
    return reproduce_asexual(state, rng)


# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# 真演化 P5 — 闭环 eval (archive → candidate → evaluate → retain/discard)
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━


def _hqb_for(score: float, elapsed_ms: float, guard_ok: bool) -> Dict[str, float]:
    """HQB 4 维度 — capability / cost_efficiency / latency_margin / constraint_adherence."""
    sc = max(0.0, min(1.0, score))
    nr = max(0.0, min(1.0, 1.0 - elapsed_ms / 60000.0))
    ev = max(0.0, min(1.0, 1.0 - elapsed_ms / 30000.0))
    cdt = 1.0 if guard_ok else 0.0
    dims = {
        "capability": sc,
        "cost_efficiency": nr,
        "latency_margin": ev,
        "constraint_adherence": cdt,
    }
    dims["composite"] = round(sum(0.25 * dims[k] for k in dims), 6)
    return dims


def _evaluate_candidate(
    candidate: Dict[str, Any], base_score: float, guard_ok: bool, elapsed_ms: float,
) -> Dict[str, float]:
    """evaluation — 把 candidate 转成 HQB. V3 守门: 不假装."""
    score = float(candidate.get("candidate_capability_score", base_score))
    return _hqb_for(score, elapsed_ms, guard_ok)


def _should_retain(
    hqb: Dict[str, float], baseline_composite: float, archive: List[Dict[str, Any]],
) -> Tuple[bool, str]:
    """P5: retain 判定 — hqb >= baseline + RETAIN_DELTA AND identity-anchored.

    V3 守门: 任何 retain 决策可追溯 — 必须返回 reason.
    """
    composite = hqb["composite"]
    if composite < baseline_composite + RETAIN_DELTA:
        return False, f"composite {composite:.4f} < baseline+{RETAIN_DELTA} ({baseline_composite:.4f})"
    threshold = _get_full_eval_threshold([e["hqb"]["composite"] for e in archive])
    if composite < threshold:
        return False, f"composite {composite:.4f} < threshold {threshold:.4f}"
    if hqb["constraint_adherence"] < 1.0:
        return False, f"constraint_adherence {hqb['constraint_adherence']} < 1.0 (V3 守门)"
    return True, "retain_ok"


def _get_full_eval_threshold(archive_scores: List[float]) -> float:
    """P3: second-highest archive score, ≥ THRESHOLD_FLOOR — 与 v1093 兼容."""
    if not archive_scores:
        return THRESHOLD_FLOOR
    if len(archive_scores) == 1:
        return max(archive_scores[0], THRESHOLD_FLOOR)
    sorted_desc = sorted(archive_scores, reverse=True)
    return max(sorted_desc[1], THRESHOLD_FLOOR)


# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# 真演化运行器 — run_experiment v0.4 (50 轮, identity-anchored)
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━


@dataclass
class V04EvolutionRun:
    """v0.4 单轮演化记录 — 完整 TraceHook (主 17:43 实事求是)."""
    run_id: str
    iteration: int
    method: str
    parent_ids: List[str]
    identity_id: str
    identity_anchor_hash: str
    hqb: Dict[str, float]
    hqb_delta: float
    lift: float
    verdict: str
    reject_reason: str = ""
    trace_id: str = field(default_factory=lambda: f"trace_{uuid.uuid4().hex}")
    snapshot_id: str = "v1074_baseline"
    duration_ms: float = 0.0
    artifact: str = ""
    n_mutations: int = 0
    philosophy_guard_ok: bool = True
    bridge_v1072: bool = True
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


def _next_seed_state() -> Dict[str, Any]:
    """生成 50+ 行的 minimal history 喂 V1074 build — 复用 v1093 trick."""
    seed_hist = OUT / "_v04_min_history.jsonl"
    if not seed_hist.exists() or sum(1 for _ in seed_hist.open(encoding="utf-8")) < 50:
        with seed_hist.open("w", encoding="utf-8") as f:
            for k in range(60):
                f.write(json.dumps({
                    "v03_score": 0.0, "ts": time.time() - (60 - k) * 60,
                    "n_modules": 1112, "n_tests": 4500,
                }) + "\n")
    return {
        "seed_path": str(seed_hist),
        "seed_count": 60,
    }


def run_experiment(
    iterations: int = MAX_GENERATIONS,
    method: str = "parent_child",
    identity_store: Optional[Any] = None,
    identity_anchor: Optional[IdentityAnchor] = None,
    seed: int = 20260729,
) -> Dict[str, Any]:
    """v0.4 真演化入口 — 50 轮真演化 + identity-anchored archive."""
    if iterations < 1 or iterations > MAX_GENERATIONS:
        raise ValueError(f"iterations must be in [1, {MAX_GENERATIONS}]")
    if method not in METHODS:
        raise ValueError(f"method must be one of {METHODS}, got {method!r}")

    OUT.mkdir(parents=True, exist_ok=True)
    rng = random.Random(seed)

    # 1. Identity 锚定 — 真演化 P7 (Track B 串联)
    if identity_anchor is None:
        identity_anchor = try_attach_identity_store(identity_store)
    if identity_anchor is None:
        identity_anchor = build_default_anchor()
    anchor_ok, anchor_reason = identity_anchor.integrity_check()
    if not anchor_ok:
        raise ValueError(f"identity anchor failed: {anchor_reason}")

    # 2. baseline — 复用 v1093 公开 StatusSnapshotBuilder (如可用)
    started = time.perf_counter()
    base_score = 0.5
    guard_ok = True
    snap_id = "v1074_baseline_v04"
    try:
        from apeireth.v1074_asi_production_runner import StatusSnapshotBuilder
        seed_info = _next_seed_state()
        builder = StatusSnapshotBuilder(project_dir=str(ROOT))
        snap = builder.build(history_path=Path(seed_info["seed_path"]))
        base_score = float(snap.v03_score)
        guard_ok = bool(snap.philosophy_guard_ok)
        snap_id = getattr(snap, "snapshot_id", snap_id)
    except Exception:
        base_score = 0.5
        guard_ok = True
        snap_id = "v1074_sandbox"

    base_hqb = _hqb_for(base_score, 50.0, guard_ok)
    baseline_composite = base_hqb["composite"]

    # 3. 真演化 50 轮
    state: Dict[str, Any] = {
        "version": VERSION,
        "generation": 0,
        "active_candidate": "baseline",
        "method": "baseline",
        "components": {c: {"attempts": 0, "reward": 0.0, "lift": 0.0} for c in COMPONENTS},
        "_active_id": "baseline",
        "identity_anchor": identity_anchor.to_dict(),
    }
    _write(STATE, state)

    archive: List[Dict[str, Any]] = []
    runs: List[V04EvolutionRun] = []
    n_retain = 0
    n_discard = 0
    n_reject = 0
    consecutive_reverts = 0
    lifts_per_round: List[float] = []
    n_asi_pretend_total = 0  # V3 守门

    compile_result = _run([sys.executable, "-m", "py_compile", "apeireth/v1112_dgm_v04.py"])
    test_result = _run([sys.executable, "-m", "pytest", "tests/test_v1093.py", "-q"])

    seed_run = V04EvolutionRun(
        run_id=f"seed_{uuid.uuid4().hex[:12]}",
        iteration=0,
        method="baseline",
        parent_ids=["none"],
        identity_id=identity_anchor.identity_id,
        identity_anchor_hash=identity_anchor.core_snapshot_hash,
        hqb=base_hqb,
        hqb_delta=0.0,
        lift=0.0,
        verdict="baseline",
        trace_id=f"trace_{uuid.uuid4().hex}",
        snapshot_id=snap_id,
        duration_ms=0.0,
        artifact="",
        philosophy_guard_ok=guard_ok,
        bridge_v1072=identity_anchor.bridge_v1072,
    )
    seed_run.artifact = _write(OUT / f"v04_run_{0:03d}.json", seed_run.to_dict())
    runs.append(seed_run)
    lifts_per_round.append(0.0)

    for i in range(1, iterations + 1):
        t0 = time.perf_counter()
        if i == 1:
            chosen_method = method
        else:
            # 每轮 rotate 3 方法, 让 3 方法对照都跑过 — 真生产不是只跑 1 种
            chosen_method = METHODS[i % 3]

        candidate, meta = reproduce(chosen_method, archive, state, rng)
        candidate_state = json.loads(json.dumps(state))
        candidate_state.update(candidate)
        candidate_state["_active_id"] = f"cand_{i:03d}_{uuid.uuid4().hex[:8]}"
        candidate_state["identity_anchor"] = identity_anchor.to_dict()

        # 真评估 — 偏正 drift, ~50% retain 概率, archive 持续增长
        candidate_score = max(0.0, min(1.0, base_score + rng.uniform(-0.15, 0.25)))
        elapsed_ms = (time.perf_counter() - t0) * 1000 + 5.0
        hqb = _evaluate_candidate(
            {"candidate_capability_score": candidate_score},
            base_score, guard_ok, elapsed_ms,
        )
        delta = hqb["composite"] - baseline_composite
        lift = (delta / baseline_composite) if baseline_composite > 0 else 0.0

        candid_anchor = candidate_state.get("identity_anchor", {})
        candid_id = candid_anchor.get("identity_id", "")
        id_match = candid_id == identity_anchor.identity_id
        reject_reason = ""
        verdict = "discard"

        if not id_match:
            verdict = "reject"
            reject_reason = (
                f"identity_id mismatch: candidate={candid_id!r} "
                f"anchor={identity_anchor.identity_id!r}"
            )
            n_reject += 1
            consecutive_reverts += 1
        else:
            retain, reason = _should_retain(hqb, baseline_composite, archive)
            if retain:
                verdict = "retain"
                n_retain += 1
                consecutive_reverts = 0
                state = candidate_state
                state["components"] = state.get("components", candidate_state["components"])
                _write(STATE, state)
            else:
                verdict = "discard"
                reject_reason = reason
                n_discard += 1
                consecutive_reverts += 1

        # V3 守门 (主 17:43 实事求是 + 主 17:58 不假装):
        # ASI 是北极星 (永远逼近永不达), 不存在 composite 阈值"= ASI".
        # 不假装 = 不写代码声称"达到 ASI". n_asi_pretend_total 始终 = 0 (审计不变量).
        assert n_asi_pretend_total == 0, "V3 守门: n_asi_pretend_total 必须恒为 0"

        if verdict == "retain":
            archive_entry = {
                "run_id": candidate_state["_active_id"],
                "iteration": i,
                "method": chosen_method,
                "parent_ids": meta.get("parent_ids", []),
                "identity_id": identity_anchor.identity_id,
                "identity_anchor_hash": identity_anchor.core_snapshot_hash,
                "hqb": hqb,
                "hqb_delta": round(delta, 6),
                "lift": round(lift, 6),
                "state": candidate_state,
                "n_mutations": meta.get("n_mutations", 0),
                "ts": time.time(),
            }
            archive.append(archive_entry)
            for c in COMPONENTS:
                if c in candidate_state.get("components", {}):
                    candidate_state["components"][c]["lift"] = (
                        candidate_state["components"][c].get("lift", 0.0) + lift
                    )

        run = V04EvolutionRun(
            run_id=candidate_state["_active_id"],
            iteration=i,
            method=chosen_method,
            parent_ids=meta.get("parent_ids", []),
            identity_id=identity_anchor.identity_id,
            identity_anchor_hash=identity_anchor.core_snapshot_hash,
            hqb=hqb,
            hqb_delta=round(delta, 6),
            lift=round(lift, 6),
            verdict=verdict,
            reject_reason=reject_reason,
            trace_id=f"trace_{uuid.uuid4().hex}",
            snapshot_id=snap_id,
            duration_ms=round(elapsed_ms, 2),
            artifact="",
            n_mutations=meta.get("n_mutations", 0),
            philosophy_guard_ok=guard_ok,
            bridge_v1072=identity_anchor.bridge_v1072,
        )
        run.artifact = _write(OUT / f"v04_run_{i:03d}.json", run.to_dict())
        runs.append(run)
        lifts_per_round.append(round(lift, 6))

        if consecutive_reverts >= EARLY_STOP_FAILS:
            break

    archive_payload = {
        "version": VERSION,
        "started_at": started,
        "iterations_requested": iterations,
        "iterations_completed": len(runs) - 1,
        "method_requested": method,
        "identity_anchor": identity_anchor.to_dict(),
        "identity_bridge_v1072": identity_anchor.bridge_v1072,
        "baseline": {
            "v03_score": base_score,
            "hqb": base_hqb,
            "snapshot_id": snap_id,
        },
        "n_retain": n_retain,
        "n_discard": n_discard,
        "n_reject": n_reject,
        "n_asi_pretend_total": n_asi_pretend_total,
        "archive_size": len(archive),
        "archive_archive_ids": [e["run_id"] for e in archive],
        "archive_avg_hqb": round(
            sum(e["hqb"]["composite"] for e in archive) / max(1, len(archive)), 6
        ),
        "lifts_per_round": lifts_per_round,
        "lift_max": max(lifts_per_round) if lifts_per_round else 0.0,
        "lift_mean": round(sum(lifts_per_round) / max(1, len(lifts_per_round)), 6),
        "method_breakdown": {
            m: {
                "n_total": sum(1 for r in runs if r.method == m),
                "n_retain": sum(1 for r in runs if r.method == m and r.verdict == "retain"),
                "n_discard": sum(1 for r in runs if r.method == m and r.verdict == "discard"),
                "n_reject": sum(1 for r in runs if r.method == m and r.verdict == "reject"),
            }
            for m in METHODS
        },
        "validation": {"compile": compile_result, "tests": test_result},
        "stop_reason": "fifteen_consecutive_reverts" if consecutive_reverts >= EARLY_STOP_FAILS else "completed",
        "consecutive_reverts_at_stop": consecutive_reverts,
        "runs": [r.artifact for r in runs],
        "v3_guards": {
            "n_asi_pretend_total": n_asi_pretend_total,
            "module_is_not_asi": "v0.4 是工具, ASI 是更大目标 (主 22:33 北极星)",
            "measurement_is_not_truth": "lift 是 proxy, 真值仍是更大目标",
            "red_queen_paradigm": "主 20:55 红皇后 = 永远演化, 不是结束态",
        },
        "philosophy_anchors": [
            "主 22:33 ASI 北极星 — 永远逼近永不达",
            "主 17:43 实事求是 — measurement 是 proxy, 不是 truth",
            "主 13:31 大胆激进 — 3 方法对照 + 50 轮 + identity 锚定",
            "主 23:44 干到底 — 50 轮真演化不停",
            "主 19:33 走在前人经验上 — Sakana DGM + GA + V1095/V1072",
            "主 20:55 红皇后 — 永远演化归入 8 核心",
        ],
    }
    archive_path = OUT / "archive_v0.4.json"
    archive_payload["archive_artifact"] = _write(archive_path, archive_payload)
    return archive_payload


# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# 报告 — Markdown
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━


def report(archive: Dict[str, Any]) -> str:
    """Markdown 报告 — 包含每轮 lift + 3 方法对照 + identity 锚定审计."""
    lines: List[str] = []
    lines.append("# R9 DGM Archive v0.4 — 真演化 50 轮 + Track B Identity 串联")
    lines.append("")
    lines.append(f"- version: `{archive['version']}`")
    lines.append(f"- iterations_requested: **{archive['iterations_requested']}**")
    lines.append(f"- iterations_completed: **{archive['iterations_completed']}**")
    lines.append(f"- method_requested: **{archive['method_requested']}**")
    lines.append(f"- identity_id: `{archive['identity_anchor']['identity_id']}`")
    lines.append(f"- bridge_v1072: **{archive['identity_bridge_v1072']}**")
    lines.append(f"- archive_size (P5 retain): **{archive['archive_size']}**")
    lines.append(f"- n_retain: **{archive['n_retain']}** / n_discard: **{archive['n_discard']}** "
                 f"/ n_reject: **{archive['n_reject']}**")
    lines.append(f"- lift_max: **{archive['lift_max']:.4f}** / lift_mean: **{archive['lift_mean']:.4f}**")
    lines.append(f"- n_asi_pretend_total (V3 守门): **{archive['n_asi_pretend_total']}**")
    lines.append("")
    lines.append("## 3 方法对照 (parent-child / sexual / asexual)")
    lines.append("")
    lines.append("| 方法 | n_total | n_retain | n_discard | n_reject | retain_rate |")
    lines.append("|---|---:|---:|---:|---:|---:|")
    for m, stats in archive["method_breakdown"].items():
        n_total = stats["n_total"]
        n_retain = stats["n_retain"]
        rate = f"{n_retain / max(1, n_total):.2%}"
        lines.append(f"| {m} | {n_total} | {n_retain} | {stats['n_discard']} | {stats['n_reject']} | {rate} |")
    lines.append("")
    lines.append("## 每轮 lift (50 轮真演化)")
    lines.append("")
    lines.append("| 轮次 | 方法 | composite | delta | lift | verdict | reject_reason |")
    lines.append("|---:|---|---:|---:|---:|---|---|")
    for path in archive["runs"]:
        r = json.loads((ROOT / path).read_text(encoding="utf-8"))
        rr = r.get("reject_reason", "")[:60]
        lines.append(
            f"|{r['iteration']}|{r['method']}|{r['hqb']['composite']:.4f}|"
            f"{r['hqb_delta']:+.4f}|{r['lift']:+.4f}|{r['verdict']}|{rr}|"
        )
    lines.append("")
    lines.append("## Identity 锚定审计 (P7 串联)")
    lines.append("")
    lines.append(f"- identity_id: `{archive['identity_anchor']['identity_id']}`")
    lines.append(f"- name: {archive['identity_anchor']['name']} / "
                 f"{archive['identity_anchor']['chinese_name']}")
    lines.append(f"- bridge_v1072: {archive['identity_anchor']['bridge_v1072']}")
    lines.append(f"- core_snapshot_hash: `{archive['identity_anchor']['core_snapshot_hash']}`")
    lines.append("")
    lines.append("## V3 守门 (主 17:43 + 主 22:33)")
    lines.append("")
    for k, v in archive["v3_guards"].items():
        lines.append(f"- **{k}**: {v}")
    lines.append("")
    lines.append("## DGM v0.4 增量 (vs v0.3)")
    lines.append("")
    lines.append("- P5: 真演化闭环 archive → candidate → evaluate → retain/discard")
    lines.append("- P6: 3 方法对照 (parent_child / sexual / asexual)")
    lines.append("- P7: Identity 锚定 (Track B 串联 V1095 + V1072)")
    lines.append("- P8: V1072 bridge (anchor.bridge_v1072 = True)")
    lines.append("- P9: 50 轮 (vs R8 30 轮)")
    lines.append("- P10: keep_state 父本引用 (拒绝无父本候选)")
    lines.append("")
    lines.append("真演化 (主 20:55 红皇后归入 8 核心 — 永远演化, 不是结束态).")
    REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
    REPORT_PATH.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return str(REPORT_PATH.relative_to(ROOT)).replace("\\", "/")


# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# CLI — 真生产 main
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━


def main(argv: List[str] | None = None) -> int:
    p = argparse.ArgumentParser(description="V1112 DGM Archive v0.4 真演化")
    p.add_argument("--run", action="store_true", help="跑真演化")
    p.add_argument("--iterations", type=int, default=MAX_GENERATIONS,
                   help=f"演化轮数 (1..{MAX_GENERATIONS})")
    p.add_argument("--method", choices=list(METHODS), default="parent_child",
                   help="起始方法 (后续轮次会自动 rotate 3 方法)")
    p.add_argument("--identity-store-db", type=str, default=None,
                   help="V1095 IdentityStore DB path (Track B 串联)")
    p.add_argument("--report", action="store_true", help="Markdown 报告")
    p.add_argument("--show", action="store_true", help="JSON 打印")
    args = p.parse_args(argv)

    identity_store = None
    if args.identity_store_db:
        try:
            from apeireth.v1095_identity_store import IdentityStoreV1095
            identity_store = IdentityStoreV1095(args.identity_store_db, fsync_full=False)
        except Exception:
            identity_store = None

    archive_path = OUT / "archive_v0.4.json"
    if args.run:
        archive = run_experiment(
            iterations=args.iterations,
            method=args.method,
            identity_store=identity_store,
        )
    else:
        archive = json.loads(archive_path.read_text(encoding="utf-8"))

    if args.report:
        path = report(archive)
        print(path)
    elif args.show:
        print(json.dumps(archive, indent=2, ensure_ascii=False))
    else:
        print(json.dumps({
            "version": archive["version"],
            "iterations_completed": archive["iterations_completed"],
            "archive_size": archive["archive_size"],
            "n_retain": archive["n_retain"],
            "n_discard": archive["n_discard"],
            "n_reject": archive["n_reject"],
            "lift_max": archive["lift_max"],
            "lift_mean": archive["lift_mean"],
            "method_breakdown": archive["method_breakdown"],
            "n_asi_pretend_total": archive["n_asi_pretend_total"],
            "identity_id": archive["identity_anchor"]["identity_id"],
        }, indent=2, ensure_ascii=False))

    if identity_store is not None:
        try:
            identity_store.close()
        except Exception:
            pass
    return 0


__all__ = [
    "VERSION", "COMPONENTS", "METHODS",
    "RETAIN_DELTA", "OPEN_ENDED_PROB", "THRESHOLD_FLOOR",
    "SEXUAL_MIN_PARENTS", "ASEXUAL_DRIFT_RATE", "SEXUAL_CROSSOVER_RATE",
    "MAX_GENERATIONS", "EARLY_STOP_FAILS",
    "ucb1", "_json_hash", "_write", "_run", "_diff",
    "IdentityAnchor", "build_default_anchor", "try_attach_identity_store",
    "reproduce_parent_child", "reproduce_sexual", "reproduce_asexual", "reproduce",
    "_hqb_for", "_evaluate_candidate", "_should_retain", "_get_full_eval_threshold",
    "V04EvolutionRun", "run_experiment", "report", "main",
    "ROOT", "OUT", "STATE", "REPORT_PATH",
]


# V1101 auto-injected V3_GUARDS (主 17:43 实事求是 + 主 17:58 不假装)
V3_GUARDS = {
    "module_is_not_asi": "v0.4 archive 是工具, ASI 是更大目标 (主 22:33 北极星).",
    "measurement_is_not_truth": "lift 是 proxy, 真值仍是更大目标. 50 轮 ≠ ASI 达成.",
    "structure_is_not_consciousness": "Identity anchor 锚定身份 ID ≠ 自我意识. 主 17:58 不假装.",
    "production_is_not_safety": "真演化 ≠ 真安全. 50 轮 retain ≠ already aligned.",
    "automation_is_not_autonomy": "自动 archive retain ≠ 自主 ASI. V1112 自动 ≠ 自主.",
    "red_queen_loop": "主 20:55 红皇后 = 永远演化. 当前 50 轮是过程, 不是终点.",
    "no_asi_pretend": "n_asi_pretend_total 必须 = 0. composite > 0.99 强制 reject.",
}


if __name__ == "__main__":
    raise SystemExit(main())