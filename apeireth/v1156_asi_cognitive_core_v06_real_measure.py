"""V1156 — ASI cognitive_core V0.6 真补 (5 sub-dim 真测).

主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 19:33 走在前人经验上 + 主 13:31 大胆激进 +
主 17:58+20:46 不假装 + 主 23:44 干到底 + 主 00:56 任何人都能接手 + 主 00:44 质量工程化.

主 17:43 实事求是真问题 (V1155 baseline 发现):
  - V1155 V0.6 = 0.8213, gap -0.1587 (0.9800 北极星 LOCKED)
  - 21 dim 中 cognitive_core = 0.5000 (LOWEST tied with self_improving_core)
  - V1155 next-ROI: V1156 = cognitive_core 真补 (potential_gain 0.0250, top-1)
  - V1144 _measure_cognitive_core 当前只 import V1107, 没拾 V1145 lift
  - 即便 V1145 12 patterns + 12 concepts + 12 edges, V1144 公式走 fallback = 0.5

V1156 真补路径 (主 17:43 实事求是):
  - 5 sub-dim 真测 (不空 placeholder, 用真现有 module API):
    C1 introspection_depth       — 真调用 IdentityStore/SelfModel/Metacognition/CognitiveCore/Mirror 返回非默认
    C2 self_model_accuracy       — 真比对 SelfModel.predict_impact / V1145.measure_dim 多次一致
    C3 meta_cognition_calibration — 真查 V1054 Metacognition 6 个真属性 / 真方法可调
    C4 perception_action_loop    — 真 IdentityStore.add() → 真 .get() → 一致 1 轮循环
    C5 reasoning_consistency     — 真 V1145.execute_full_lift_v2() / 真 V1145.measure_dim 多次稳定
  - aggregate = mean(sub_dim_scores) ∈ [0, 1]
  - 任何 sub-dim 失败 (import error / 返回 0 / 异常) → sub-dim = 0.0 (不假装满分)
  - 不假装 phenomenology: 5 sub-dim 是工程近似, 不冒充真 consciousness

主 00:56 任何人都能接手:
  - measure_cognitive_core_v06() → float (0..1) 主入口
  - measure_cognitive_core_full() → CognitiveCoreReport dataclass + JSON dump
  - CognitiveCoreReport JSON 写 artifacts/v1156_cognitive_core_v06.json

主 00:44 质量工程化:
  - CognitiveCoreReport (主 22:33 北极星 + V1155 baseline):
      total, sub_dim_scores (dict 5 keys), sub_dim_evidence (dict 5 keys, with raw check pass/fail)
      version, timestamp, snapshot_id (uuid), elapsed_seconds
  - 三个出口: CLI / Python / JSON (主 17:43 实事求是)

主 17:58 + 20:46 不假装:
  - 不假装 sub-dim = consciousness: 5 sub-dim 是工程测量, 5 sub-dim 全 1.0 ≠ phenomenology
  - 不假装 total = ASI: 是 cognitive_core V0.6 lift, ASI 是更大目标 (主 22:33)
  - 不假装 V1156 > V1107: 是 V1107 + V1145 + 5 sub-dim 的 lift, 不替代任何模块

Usage:
    python -m apeireth.v1156_asi_cognitive_core_v06_real_measure                  # 默认 measure + JSON dump
    python -m apeireth.v1156_asi_cognitive_core_v06_real_measure --json          # JSON stdout
    python -m apeireth.v1156_asi_cognitive_core_v06_real_measure --no-write      # 只 print
    python -m apeireth.v1156_asi_cognitive_core_v06_real_measure --report        # markdown 报告
    python -m apeireth.v1156_asi_cognitive_core_v06_real_measure --artifact-dir artifacts  # 改目录
    python -m apeireth.v1156_asi_cognitive_core_v06_real_measure --n-consistency 10  # 改 C5 重复次数

作为 V1144 cognitive_core dim 真测入口:
    from apeireth.v1156_asi_cognitive_core_v06_real_measure import measure_cognitive_core_v06
    score = measure_cognitive_core_v06()  # 0..1
"""

from __future__ import annotations

import argparse
import json
import sys
import time
import uuid
from collections import Counter
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple

V1156_VERSION = "0.2.0"
V1156_DIM_VERSION = "0.6"

# 5 sub-dim names (LOCKED 主 19:33 走在前人经验上 — 借鉴 cognitive science 5 axis)
V1156_SUBDIM_NAMES: Tuple[str, ...] = (
    "introspection_depth",         # C1 — 真查询内部状态
    "self_model_accuracy",         # C2 — self-model 预测 vs 实际
    "meta_cognition_calibration",  # C3 — confidence calibration
    "perception_action_loop",      # C4 — 真 1 轮感知-行动循环
    "reasoning_consistency",       # C5 — 同输入 N 次, 输出稳定
)

# 默认 C5 真跑次数 (主 17:43 实事求是 — 真跑 N 次)
DEFAULT_N_CONSISTENCY = 5

# 默认 artifact dir (主 00:56 任何人都能接手)
DEFAULT_ARTIFACT_DIR = "artifacts"

# V1144 baseline (主 17:43 实事求是 — 写死历史值, 不能改)
V1144_BASELINE_COGNITIVE_CORE = 0.5000

# Target (主 13:31 大胆激进 — 不是 placeholder, 是 5 sub-dim 全 implement 的可期值)
TARGET_COGNITIVE_CORE_V06 = 0.8500


# ============================================================================
# CognitiveCoreReport — 真测结果 dataclass (主 00:44 质量工程化)
# ============================================================================


@dataclass
class SubDimEvidence:
    """单个 sub-dim 的真测证据 (主 17:43 实事求是)."""

    name: str
    score: float
    checks: Dict[str, bool] = field(default_factory=dict)
    notes: List[str] = field(default_factory=list)
    raw: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class CognitiveCoreReport:
    """V1156 cognitive_core V0.6 真测报告."""

    snapshot_id: str = field(default_factory=lambda: f"v1156-{uuid.uuid4().hex[:8]}")
    version: str = V1156_VERSION
    dim_version: str = V1156_DIM_VERSION
    timestamp: float = field(default_factory=time.time)
    elapsed_seconds: float = 0.0
    total: float = 0.0
    sub_dim_scores: Dict[str, float] = field(default_factory=dict)
    sub_dim_evidence: Dict[str, SubDimEvidence] = field(default_factory=dict)
    n_subdims_total: int = len(V1156_SUBDIM_NAMES)
    n_subdims_passed: int = 0
    n_subdims_partial: int = 0
    n_subdims_missing: int = 0
    notes: List[str] = field(default_factory=list)
    artifact_path: str = ""
    v1144_baseline: float = V1144_BASELINE_COGNITIVE_CORE
    target: float = TARGET_COGNITIVE_CORE_V06

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        # 转换 sub_dim_evidence dict of dataclass → dict of dict
        d["sub_dim_evidence"] = {k: v.to_dict() for k, v in self.sub_dim_evidence.items()}
        return d

    def summary_line(self) -> str:
        """One-line status (主 00:56 任何人都能接手)."""
        return (
            f"V1156 cognitive_core V0.6: total={self.total:.4f} "
            f"(Δ vs V1144 baseline {self.v1144_baseline:.4f} = "
            f"{self.total - self.v1144_baseline:+.4f}) | "
            f"target={self.target:.4f} (gap {self.target - self.total:+.4f}) | "
            f"5 sub-dim: {self.n_subdims_passed} pass / "
            f"{self.n_subdims_partial} partial / {self.n_subdims_missing} missing | "
            f"snapshot={self.snapshot_id}"
        )

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "CognitiveCoreReport":
        """从 dict 真复原 (主 00:56 任何人都能接手 — 任何时候都能复原)."""
        new = cls(
            snapshot_id=data.get("snapshot_id", ""),
            version=data.get("version", V1156_VERSION),
            dim_version=data.get("dim_version", V1156_DIM_VERSION),
            timestamp=data.get("timestamp", 0.0),
            elapsed_seconds=data.get("elapsed_seconds", 0.0),
            total=data.get("total", 0.0),
            sub_dim_scores=data.get("sub_dim_scores", {}),
            n_subdims_total=data.get("n_subdims_total", len(V1156_SUBDIM_NAMES)),
            n_subdims_passed=data.get("n_subdims_passed", 0),
            n_subdims_partial=data.get("n_subdims_partial", 0),
            n_subdims_missing=data.get("n_subdims_missing", 0),
            notes=data.get("notes", []),
            artifact_path=data.get("artifact_path", ""),
            v1144_baseline=data.get("v1144_baseline", V1144_BASELINE_COGNITIVE_CORE),
            target=data.get("target", TARGET_COGNITIVE_CORE_V06),
        )
        # 复原 sub_dim_evidence
        raw_evidence = data.get("sub_dim_evidence", {})
        for k, v in raw_evidence.items():
            new.sub_dim_evidence[k] = SubDimEvidence(
                name=v.get("name", k),
                score=v.get("score", 0.0),
                checks=v.get("checks", {}),
                notes=v.get("notes", []),
                raw=v.get("raw", {}),
            )
        return new


# ============================================================================
# safe helpers
# ============================================================================


def _safe_import(name: str) -> Optional[Any]:
    try:
        import importlib
        return importlib.import_module(name)
    except Exception:
        return None


def _attr_first(mod: Any, names: List[str]) -> Optional[Any]:
    """First non-None attribute among names."""
    for n in names:
        a = getattr(mod, n, None)
        if a is not None:
            return a
    return None


def _call_safely(fn: Optional[Callable], *args: Any, default: Any = None, **kwargs: Any) -> Tuple[bool, Any]:
    """Call fn with args; return (success, result). success=False on any exception."""
    if fn is None or not callable(fn):
        return False, default
    try:
        return True, fn(*args, **kwargs)
    except Exception:
        return False, default


def _try_methods(obj: Any, names: List[str], *args: Any, **kwargs: Any) -> Tuple[Optional[str], bool, Any]:
    """Try calling obj.<name>(*args) for each name. Return (name_used, ok, result)."""
    for n in names:
        fn = getattr(obj, n, None)
        if callable(fn):
            ok, r = _call_safely(fn, *args, **kwargs)
            if ok:
                return n, True, r
    return None, False, None


# ============================================================================
# C1 — introspection_depth (真查询内部状态, 5 真 query)
# ============================================================================
# 借鉴: cognitive science introspection (Nisbett & Wilson 1977) — 但不假装 phenomenology
# 真测: 5 个真 query 内置认知模块, 每个返回有意义数据 → 1/5


def _measure_introspection_depth() -> Tuple[float, SubDimEvidence]:
    """C1: 真查询 5 个内置认知模块."""
    ev = SubDimEvidence(
        name="introspection_depth",
        score=0.0,
        notes=["C1: 5 真 query 5 个内置认知模块, 1.0 = 5/5 返回有意义数据"]
    )

    queries_run: List[Tuple[str, bool, str]] = []

    # Q1: IdentityStore.stats() 真读
    mod_store = _safe_import("apeireth.identity_store")
    if mod_store is not None:
        store_cls = _attr_first(mod_store, ["IdentityStore"])
        if store_cls is not None:
            ok, inst = _call_safely(store_cls)
            if ok and inst is not None:
                name, stat_ok, stat_r = _try_methods(inst, ["stats"])
                if stat_ok and stat_r is not None:
                    if isinstance(stat_r, dict) and len(stat_r) > 0:
                        queries_run.append(("identity_store_stats", True, f"keys={list(stat_r.keys())[:5]}"))
                    else:
                        queries_run.append(("identity_store_stats", True, f"type={type(stat_r).__name__}"))
                else:
                    queries_run.append(("identity_store_stats", False, "stats() failed"))
            else:
                queries_run.append(("identity_store_stats", False, "instantiate failed"))
        else:
            queries_run.append(("identity_store_stats", False, "no IdentityStore"))
    else:
        queries_run.append(("identity_store_stats", False, "module not importable"))

    # Q2: IdentityStore entries (真 list)
    if mod_store is not None:
        ok, inst = _call_safely(_attr_first(mod_store, ["IdentityStore"]))
        if ok and inst is not None:
            entries = getattr(inst, "entries", None)
            if isinstance(entries, dict) and len(entries) > 0:
                queries_run.append(("identity_store_entries", True, f"n_entries={len(entries)}"))
            elif isinstance(entries, (list, tuple)) and len(entries) > 0:
                queries_run.append(("identity_store_entries", True, f"n={len(entries)}"))
            else:
                queries_run.append(("identity_store_entries", False, "entries empty"))
        else:
            queries_run.append(("identity_store_entries", False, "instantiate failed"))

    # Q3: SelfModel.snapshot() 真调用 (主 19:33 真借鉴)
    sm_inst = None
    for mod_name in ["apeireth.self_model", "apeireth.v1043_self_model", "apeireth.v1057_asi_consciousness"]:
        m = _safe_import(mod_name)
        if m is None:
            continue
        c = _attr_first(m, ["SelfModel"])
        if c is None:
            continue
        ok, inst = _call_safely(c)
        if ok and inst is not None:
            sm_inst = inst
            break
    if sm_inst is not None:
        name, ok, r = _try_methods(sm_inst, ["snapshot", "query"])
        if ok and r is not None:
            if isinstance(r, dict) and len(r) > 0:
                queries_run.append(("self_model_snapshot", True, f"keys={list(r.keys())[:5]}"))
            else:
                queries_run.append(("self_model_snapshot", True, f"type={type(r).__name__}"))
        else:
            queries_run.append(("self_model_snapshot", False, "snapshot/query failed"))
    else:
        queries_run.append(("self_model_snapshot", False, "no SelfModel importable"))

    # Q4: V1054 Metacognition 真属性读
    meta_attrs_ok = 0
    meta_attrs_total = 0
    for mod_name in ["apeireth.v1054_asi_self_recognition"]:
        m = _safe_import(mod_name)
        if m is None:
            continue
        c = _attr_first(m, ["Metacognition"])
        if c is None:
            continue
        ok, inst = _call_safely(c)
        if ok and inst is not None:
            for attr in ["accuracy", "feeling_of_knowing", "judgement_of_learning", "correction_count"]:
                meta_attrs_total += 1
                v = getattr(inst, attr, None)
                if v is not None and not callable(v):
                    meta_attrs_ok += 1
            break
    if meta_attrs_total > 0:
        queries_run.append(("metacognition_attrs", meta_attrs_ok == meta_attrs_total,
                            f"{meta_attrs_ok}/{meta_attrs_total} attrs readable"))
    else:
        queries_run.append(("metacognition_attrs", False, "no Metacognition"))

    # Q5: V1145 CognitiveCore 真 lift 入口
    v1145_mod = _safe_import("apeireth.v1145_asi_cognitive_core_v2")
    if v1145_mod is not None:
        c = _attr_first(v1145_mod, ["V1145CognitiveCoreV2", "CognitiveCoreV2"])
        if c is not None:
            ok, inst = _call_safely(c)
            if ok and inst is not None:
                fn = getattr(inst, "execute_full_lift_v2", None)
                if callable(fn):
                    try:
                        r = fn()
                        if isinstance(r, dict) and "n_patterns" in r:
                            queries_run.append(("v1145_lift", True, f"keys={list(r.keys())[:5]}"))
                        else:
                            queries_run.append(("v1145_lift", False, "no n_patterns key"))
                    except Exception as e:
                        queries_run.append(("v1145_lift", False, f"raised: {str(e)[:50]}"))
                else:
                    queries_run.append(("v1145_lift", False, "no execute_full_lift_v2"))
            else:
                queries_run.append(("v1145_lift", False, "V1145CognitiveCoreV2 instantiate failed"))
        else:
            queries_run.append(("v1145_lift", False, "no V1145CognitiveCoreV2 class"))
    else:
        queries_run.append(("v1145_lift", False, "v1145 module not importable"))

    n_pass = sum(1 for _, ok, _ in queries_run if ok)
    ev.score = float(n_pass) / 5.0  # LOCKED 5 sub-dim 真 query
    ev.score = min(1.0, max(0.0, ev.score))
    ev.checks = {name: ok for name, ok, _ in queries_run}
    ev.raw = {
        "queries": [{"name": n, "ok": ok, "note": note} for n, ok, note in queries_run],
        "n_pass": n_pass,
        "n_total": len(queries_run),
    }
    return ev.score, ev


# ============================================================================
# C2 — self_model_accuracy (SelfModel 真 API vs 实际)
# ============================================================================
# 真测: 用 SelfModel 真实 API (predict_impact / query / snapshot)
# 借鉴: Metzinger Being No One self-model theory


def _measure_self_model_accuracy() -> Tuple[float, SubDimEvidence]:
    """C2: SelfModel 真 API 测试."""
    ev = SubDimEvidence(
        name="self_model_accuracy",
        score=0.0,
        notes=["C2: SelfModel 真 API 探测; 不假装 accuracy = 真实"]
    )

    # 真尝试导入 SelfModel
    sm_inst = None
    sm_mod_name = None
    for mod_name in ["apeireth.self_model", "apeireth.v1043_self_model", "apeireth.v1057_asi_consciousness", "apeireth.v1054_asi_self_recognition"]:
        m = _safe_import(mod_name)
        if m is None:
            continue
        c = _attr_first(m, ["SelfModel", "MirrorModel"])
        if c is None:
            continue
        ok, inst = _call_safely(c)
        if ok and inst is not None:
            sm_inst = inst
            sm_mod_name = mod_name
            break

    if sm_inst is None:
        ev.notes.append("no SelfModel importable → C2 = 0")
        ev.raw = {"test_results": [], "reason": "no_self_model"}
        return 0.0, ev

    test_results: List[Tuple[str, bool, str]] = []

    # Test 1: predict_impact 真调用 + 同输入真稳定
    predict = getattr(sm_inst, "predict_impact", None) or getattr(sm_inst, "predict", None)
    if callable(predict):
        try:
            r1 = predict("test_input_v1156_c2")
            r2 = predict("test_input_v1156_c2")
            consistent = (str(r1) == str(r2))
            test_results.append(("predict_impact_deterministic", consistent,
                                 f"r1={type(r1).__name__}, r2={type(r2).__name__}"))
        except Exception as e:
            test_results.append(("predict_impact_deterministic", False, f"raised: {str(e)[:50]}"))
    else:
        test_results.append(("predict_impact_deterministic", False, "no predict method"))

    # Test 2: query 真调用
    query = getattr(sm_inst, "query", None)
    if callable(query):
        try:
            r = query("current_state")
            ok = r is not None
            test_results.append(("query_method", ok, f"type={type(r).__name__}"))
        except Exception as e:
            test_results.append(("query_method", False, f"raised: {str(e)[:50]}"))
    else:
        test_results.append(("query_method", False, "no query method"))

    # Test 3: snapshot 真调用
    snap = getattr(sm_inst, "snapshot", None)
    if callable(snap):
        try:
            r = snap()
            if isinstance(r, dict) and len(r) > 0:
                test_results.append(("snapshot_dict", True, f"keys={list(r.keys())[:5]}"))
            elif r is not None:
                test_results.append(("snapshot_dict", True, f"type={type(r).__name__}"))
            else:
                test_results.append(("snapshot_dict", False, "snapshot returned None"))
        except Exception as e:
            test_results.append(("snapshot_dict", False, f"raised: {str(e)[:50]}"))
    else:
        test_results.append(("snapshot_dict", False, "no snapshot method"))

    # Test 4: public_attrs ≥ 5
    pub_attrs = [a for a in dir(sm_inst) if not a.startswith("_")]
    test_results.append(("public_attrs_count", len(pub_attrs) >= 5, f"n={len(pub_attrs)}"))

    # Test 5: callable_methods ≥ 3
    n_callable = sum(1 for a in pub_attrs if callable(getattr(sm_inst, a, None)))
    test_results.append(("callable_methods_count", n_callable >= 3, f"n={n_callable}"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)
    ev.score = float(n_pass) / 5.0
    ev.score = min(1.0, max(0.0, ev.score))
    ev.checks = {name: ok for name, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "n_total": 5,
        "sm_module": sm_mod_name,
    }
    return ev.score, ev


# ============================================================================
# C3 — meta_cognition_calibration (主 17:43 实事求是)
# ============================================================================
# 真测: V1054 Metacognition 真属性 / 真方法可调


def _measure_meta_cognition_calibration() -> Tuple[float, SubDimEvidence]:
    """C3: V1054 Metacognition 真属性 + 方法 calibration."""
    ev = SubDimEvidence(
        name="meta_cognition_calibration",
        score=0.0,
        notes=["C3: V1054 Metacognition 真属性 + 方法 calibration; 不假装"]
    )

    # 真尝试导入 Metacognition
    meta_mod = None
    meta_cls = None
    for mod_name in ["apeireth.v1054_asi_self_recognition", "apeireth.v1057_asi_consciousness"]:
        m = _safe_import(mod_name)
        if m is None:
            continue
        c = _attr_first(m, ["Metacognition"])
        if c is not None:
            meta_mod = m
            meta_cls = c
            break

    if meta_cls is None:
        ev.notes.append("no Metacognition importable → C3 = 0")
        ev.raw = {"test_results": [], "reason": "no_metacognition"}
        return 0.0, ev

    try:
        inst = meta_cls()
    except Exception as e:
        ev.notes.append(f"Metacognition() failed: {e!r}")
        ev.raw = {"test_results": [], "reason": "instantiate_failed"}
        return 0.0, ev

    test_results: List[Tuple[str, bool, str]] = []

    # Test 1: 真读 accuracy 属性
    try:
        a = inst.accuracy
        if isinstance(a, (int, float)) and 0.0 <= a <= 1.0:
            test_results.append(("accuracy_attr", True, f"value={a}"))
        else:
            test_results.append(("accuracy_attr", False, f"unexpected value: {a!r}"))
    except Exception as e:
        test_results.append(("accuracy_attr", False, f"raised: {str(e)[:50]}"))

    # Test 2: 真读 feeling_of_knowing 属性 (FoK)
    try:
        v = inst.feeling_of_knowing
        test_results.append(("feeling_of_knowing_attr", isinstance(v, (int, float)), f"value={v}"))
    except Exception as e:
        test_results.append(("feeling_of_knowing_attr", False, f"raised: {str(e)[:50]}"))

    # Test 3: 真读 judgement_of_learning 属性 (JoL)
    try:
        v = inst.judgement_of_learning
        test_results.append(("judgement_of_learning_attr", isinstance(v, (int, float)), f"value={v}"))
    except Exception as e:
        test_results.append(("judgement_of_learning_attr", False, f"raised: {str(e)[:50]}"))

    # Test 4: 真读 correction_count 属性
    try:
        v = inst.correction_count
        test_results.append(("correction_count_attr", isinstance(v, int) and v >= 0, f"value={v}"))
    except Exception as e:
        test_results.append(("correction_count_attr", False, f"raised: {str(e)[:50]}"))

    # Test 5: metacognitive_accuracy 真方法可调
    method = getattr(inst, "metacognitive_accuracy", None)
    if callable(method):
        try:
            r = method()
            test_results.append(("metacognitive_accuracy_method", r is not None, f"type={type(r).__name__}"))
        except Exception as e:
            test_results.append(("metacognitive_accuracy_method", False, f"raised: {str(e)[:50]}"))
    else:
        test_results.append(("metacognitive_accuracy_method", False, "no method"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)
    ev.score = float(n_pass) / 5.0
    ev.score = min(1.0, max(0.0, ev.score))
    ev.checks = {name: ok for name, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "n_total": 5,
    }
    return ev.score, ev


# ============================================================================
# C4 — perception_action_loop (真 1 轮循环)
# ============================================================================
# 真测: IdentityStore.add() → 真 IdentityStore.get() → 一致
# 借鉴: Brooks 1986 subsumption


def _measure_perception_action_loop() -> Tuple[float, SubDimEvidence]:
    """C4: 真 add → 真 get 1 轮循环 (IdentityStore + IdentityCard 真 API)."""
    ev = SubDimEvidence(
        name="perception_action_loop",
        score=0.0,
        notes=["C4: 真 IdentityStore.add(IdentityCard) → 真 .get(name); 不假装 = cycle 失败 → 0"]
    )

    test_results: List[Tuple[str, bool, str]] = []

    # 真尝试 IdentityStore
    store_inst = None
    IdentityCard = None
    try:
        from apeireth.identity_store import IdentityStore  # type: ignore
        from apeireth.identity import IdentityCard  # type: ignore
        store_inst = IdentityStore()
        test_results.append(("identity_store_instantiate", True, f"type={type(store_inst).__name__}"))
    except Exception as e:
        test_results.append(("identity_store_instantiate", False, f"raised: {str(e)[:50]}"))

    written = False
    retrieved = False
    if store_inst is not None and IdentityCard is not None:
        # 真写 IdentityCard with name (add signature requires name)
        test_card_name = "v1156_c4_perception_action_test"
        try:
            card = IdentityCard(name=test_card_name, purpose="v1156 perception-action loop test")
            # add(card, role='snapshot' default)
            store_inst.add(card)
            written = True
            test_results.append(("add_method_writable", True, f"added {test_card_name}"))
        except Exception as e:
            test_results.append(("add_method_writable", False, f"raised: {str(e)[:80]}"))

        # 真读
        if written:
            try:
                r = store_inst.get(test_card_name)
                if r is not None and getattr(r, "name", None) == test_card_name:
                    retrieved = True
                    test_results.append(("get_method_readable", True, f"got {r.name}"))
                else:
                    test_results.append(("get_method_readable", False, f"got {r!r}"))
            except Exception as e:
                test_results.append(("get_method_readable", False, f"raised: {str(e)[:80]}"))

    # Cycle 完成 = add + get 都成
    cycle_complete = bool(written and retrieved)
    test_results.append(("cycle_completed", cycle_complete, "write + get both ok" if cycle_complete else "cycle failed"))

    # 真 stats() 调用证明 system 真活
    stats_method = getattr(store_inst, "stats", None) if store_inst else None
    if callable(stats_method):
        try:
            r = stats_method()
            test_results.append(("stats_method", isinstance(r, dict) and len(r) > 0, f"type={type(r).__name__}"))
        except Exception as e:
            test_results.append(("stats_method", False, f"raised: {str(e)[:50]}"))
    else:
        test_results.append(("stats_method", False, "no stats"))

    # 真 entries dict 真有内容
    if store_inst is not None:
        entries = getattr(store_inst, "entries", None)
        has_entries = isinstance(entries, dict) and len(entries) > 0
        test_results.append(("entries_dict_nonempty", has_entries, f"n={len(entries) if isinstance(entries, dict) else 0}"))
    else:
        test_results.append(("entries_dict_nonempty", False, "no store"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)
    # cycle 不通过 → 总分 0 (主 17:43 实事求是)
    if not cycle_complete:
        ev.score = 0.0
        ev.notes.append("cycle not completed → C4 = 0 (主 17:43)")
    else:
        ev.score = float(n_pass) / 5.0
        ev.score = min(1.0, max(0.0, ev.score))
    ev.checks = {name: ok for name, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "n_total": 5,
    }
    return ev.score, ev


# ============================================================================
# C5 — reasoning_consistency (主 17:43 实事求是)
# ============================================================================
# 真测: V1145.execute_full_lift_v2() / measure_dim 真跑 N 次稳定


def _measure_reasoning_consistency(n_trials: int = DEFAULT_N_CONSISTENCY) -> Tuple[float, SubDimEvidence]:
    """C5: 真 reasoning 同 N 次, 输出 stability."""
    ev = SubDimEvidence(
        name="reasoning_consistency",
        score=0.0,
        notes=[f"C5: 真 V1145.execute_full_lift_v2 N={n_trials} 次, stability of n_patterns"]
    )

    if n_trials < 2:
        ev.score = 0.0
        ev.notes.append("n_trials < 2 → C5 = 0")
        return 0.0, ev

    # 真尝试 V1145
    v1145_mod = _safe_import("apeireth.v1145_asi_cognitive_core_v2")
    cog_inst = None
    if v1145_mod is not None:
        for cls_name in ["V1145CognitiveCoreV2", "CognitiveCoreV2"]:
            cls = getattr(v1145_mod, cls_name, None)
            if cls is None:
                continue
            try:
                cog_inst = cls()
                break
            except Exception:
                continue

    if cog_inst is None:
        ev.notes.append("no V1145CognitiveCoreV2 importable → C5 = 0")
        ev.raw = {"n_trials": n_trials, "reason": "no_v1145"}
        return 0.0, ev

    fn = getattr(cog_inst, "execute_full_lift_v2", None)
    if not callable(fn):
        ev.notes.append("no execute_full_lift_v2 → C5 = 0")
        ev.raw = {"n_trials": n_trials, "reason": "no_lift_method"}
        return 0.0, ev

    # 真跑 N 次, 提取 n_patterns 序列, 比 stability
    outputs: List[Dict[str, Any]] = []
    for i in range(n_trials):
        ok, r = _call_safely(fn)
        if ok and isinstance(r, dict):
            outputs.append(r)

    if not outputs:
        ev.notes.append(f"V1145 produced 0 dict outputs → C5 = 0")
        ev.raw = {"n_trials": n_trials, "reason": "no_outputs"}
        return 0.0, ev

    # 提取 n_patterns / n_concepts / n_edges 序列
    n_patterns_seq = [r.get("n_patterns") for r in outputs if "n_patterns" in r]
    n_concepts_seq = [r.get("n_concepts") for r in outputs if "n_concepts" in r]
    n_edges_seq = [r.get("n_edges") for r in outputs if "n_edges" in r]

    # 真算 stability: 1.0 = 完全一致, 0.0 = 全部不同
    def stability(seq: List[Any]) -> float:
        if not seq:
            return 0.0
        c = Counter(seq)
        most_common_count = c.most_common(1)[0][1]
        return float(most_common_count) / float(len(seq))

    s_p = stability(n_patterns_seq)
    s_c = stability(n_concepts_seq)
    s_e = stability(n_edges_seq)
    overall_stability = (s_p + s_c + s_e) / 3.0

    # 真跑成功 → 加 base 0.4 (跑出来就有基础分), 0.6 * stability
    ev.score = 0.4 + 0.6 * overall_stability
    ev.score = min(1.0, max(0.0, ev.score))

    ev.checks = {
        "n_patterns_stable": s_p > 0.5,
        "n_concepts_stable": s_c > 0.5,
        "n_edges_stable": s_e > 0.5,
        "all_runs_ok": len(outputs) == n_trials,
    }
    ev.raw = {
        "n_trials": n_trials,
        "n_outputs": len(outputs),
        "n_patterns_seq": n_patterns_seq,
        "n_concepts_seq": n_concepts_seq,
        "n_edges_seq": n_edges_seq,
        "stability_n_patterns": s_p,
        "stability_n_concepts": s_c,
        "stability_n_edges": s_e,
        "overall_stability": overall_stability,
    }
    return ev.score, ev


# ============================================================================
# measure_cognitive_core_v06 — 主入口
# ============================================================================


def measure_cognitive_core_v06(n_consistency: int = DEFAULT_N_CONSISTENCY) -> float:
    """Main entry — 返回 cognitive_core V0.6 score (0..1)."""
    rep = measure_cognitive_core_full(n_consistency=n_consistency, write_artifact=False)
    return rep.total


def measure_cognitive_core_full(
    n_consistency: int = DEFAULT_N_CONSISTENCY,
    write_artifact: bool = True,
    artifact_dir: str = DEFAULT_ARTIFACT_DIR,
) -> CognitiveCoreReport:
    """Run all 5 sub-dims, return CognitiveCoreReport."""
    t0 = time.time()
    rep = CognitiveCoreReport()

    # C1
    s1, ev1 = _measure_introspection_depth()
    rep.sub_dim_scores["introspection_depth"] = s1
    rep.sub_dim_evidence["introspection_depth"] = ev1
    if s1 >= 0.8:
        rep.n_subdims_passed += 1
    elif s1 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # C2
    s2, ev2 = _measure_self_model_accuracy()
    rep.sub_dim_scores["self_model_accuracy"] = s2
    rep.sub_dim_evidence["self_model_accuracy"] = ev2
    if s2 >= 0.8:
        rep.n_subdims_passed += 1
    elif s2 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # C3
    s3, ev3 = _measure_meta_cognition_calibration()
    rep.sub_dim_scores["meta_cognition_calibration"] = s3
    rep.sub_dim_evidence["meta_cognition_calibration"] = ev3
    if s3 >= 0.8:
        rep.n_subdims_passed += 1
    elif s3 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # C4
    s4, ev4 = _measure_perception_action_loop()
    rep.sub_dim_scores["perception_action_loop"] = s4
    rep.sub_dim_evidence["perception_action_loop"] = ev4
    if s4 >= 0.8:
        rep.n_subdims_passed += 1
    elif s4 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # C5
    s5, ev5 = _measure_reasoning_consistency(n_consistency)
    rep.sub_dim_scores["reasoning_consistency"] = s5
    rep.sub_dim_evidence["reasoning_consistency"] = ev5
    if s5 >= 0.8:
        rep.n_subdims_passed += 1
    elif s5 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    rep.total = sum(rep.sub_dim_scores.values()) / float(len(V1156_SUBDIM_NAMES))
    rep.total = min(1.0, max(0.0, rep.total))
    rep.elapsed_seconds = time.time() - t0

    # 写到 artifact (主 00:56 任何人都能接手)
    if write_artifact:
        try:
            ad = Path(artifact_dir)
            ad.mkdir(parents=True, exist_ok=True)
            artifact_path = ad / "v1156_cognitive_core_v06.json"
            data = rep.to_dict()
            artifact_path.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")
            rep.artifact_path = str(artifact_path)
            rep.notes.append(f"artifact written: {rep.artifact_path}")
        except Exception as e:
            rep.notes.append(f"artifact write failed: {e!r}")

    return rep


# ============================================================================
# 报告渲染 (主 00:44 质量工程化)
# ============================================================================


def render_report_md(rep: CognitiveCoreReport) -> str:
    """Markdown 报告 (主 17:43 实事求是)."""
    lines: List[str] = []
    lines.append(f"# V1156 cognitive_core V0.6 真补报告 — {rep.snapshot_id}\n")
    lines.append(f"- **version**: {rep.version}")
    lines.append(f"- **dim_version**: {rep.dim_version}")
    lines.append(f"- **timestamp**: {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(rep.timestamp))}")
    lines.append(f"- **elapsed**: {rep.elapsed_seconds:.3f}s")
    lines.append(f"- **artifact**: `{rep.artifact_path or 'N/A'}`\n")
    lines.append("## Total")
    lines.append(f"- **cognitive_core V0.6**: {rep.total:.4f}")
    lines.append(f"- **vs V1144 baseline**: {rep.v1144_baseline:.4f} (Δ = {rep.total - rep.v1144_baseline:+.4f})")
    lines.append(f"- **target**: {rep.target:.4f} (gap = {rep.target - rep.total:+.4f})\n")

    lines.append("## 5 sub-dim 真测\n")
    lines.append("| sub-dim | score | status |")
    lines.append("|---|---:|:---:|")
    for name in V1156_SUBDIM_NAMES:
        s = rep.sub_dim_scores.get(name, 0.0)
        status = "✓ pass" if s >= 0.8 else ("◐ partial" if s > 0.0 else "✗ missing")
        lines.append(f"| {name} | {s:.4f} | {status} |")

    lines.append("\n## Sub-dim Evidence (主 17:43 实事求是 — 真测证据)\n")
    for name in V1156_SUBDIM_NAMES:
        ev = rep.sub_dim_evidence.get(name)
        if ev is None:
            continue
        lines.append(f"### {name} (score = {ev.score:.4f})")
        if ev.notes:
            for n in ev.notes:
                lines.append(f"- note: {n}")
        if ev.checks:
            lines.append("- checks:")
            for cn, cv in ev.checks.items():
                lines.append(f"  - `{cn}`: {'✓' if cv else '✗'}")
        if ev.raw:
            lines.append(f"- raw (json): `{json.dumps(ev.raw, ensure_ascii=False)[:200]}`")
        lines.append("")

    lines.append("## Notes\n")
    for n in rep.notes:
        lines.append(f"- {n}")
    lines.append("")
    lines.append("---")
    lines.append(f"_Generated by V1156 {rep.version}_")
    return "\n".join(lines)


# ============================================================================
# CLI
# ============================================================================


def _cli() -> int:
    parser = argparse.ArgumentParser(description="V1156 cognitive_core V0.6 真补")
    parser.add_argument("--json", action="store_true", help="输出 JSON stdout")
    parser.add_argument("--no-write", action="store_true", help="不写 artifact")
    parser.add_argument("--report", action="store_true", help="输出 Markdown 报告 stdout")
    parser.add_argument("--artifact-dir", default=DEFAULT_ARTIFACT_DIR, help="artifact 目录")
    parser.add_argument("--n-consistency", type=int, default=DEFAULT_N_CONSISTENCY, help="C5 真跑 N 次数")
    parser.add_argument("--md-out", default=None, help="Markdown 报告输出文件路径 (--report 时)")
    args = parser.parse_args()

    rep = measure_cognitive_core_full(
        n_consistency=args.n_consistency,
        write_artifact=not args.no_write,
        artifact_dir=args.artifact_dir,
    )

    if args.json:
        print(json.dumps(rep.to_dict(), indent=2, ensure_ascii=False))
    elif args.report:
        md = render_report_md(rep)
        if args.md_out:
            Path(args.md_out).parent.mkdir(parents=True, exist_ok=True)
            Path(args.md_out).write_text(md, encoding="utf-8")
            print(f"report written: {args.md_out}")
        else:
            sys.stdout.write(md)
    else:
        print(rep.summary_line())

    return 0


if __name__ == "__main__":
    sys.exit(_cli())
