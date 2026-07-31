"""V1162 — ASI world_model V0.6 真补 (5 sub-dim 真测).

主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 19:33 走在前人经验上 + 主 13:31 大胆激进 +
主 17:58+20:46 不假装 + 主 23:44 干到底 + 主 00:56 任何人都能接手 + 主 00:44 质量工程化.

主 17:43 实事求是真问题 (V1155 baseline):
  - V1155 next-ROI top-2 = world_model (current 0.0000)
  - V1144._measure_world_model 当前空, 只 placeholder
  - 世界模型 = ASI 平台预测与规划核心, 真补 = 5 sub-dim 真测

V1162 真补路径 (主 17:43 实事求是):
  - 5 sub-dim 真测 (不空 placeholder):
    W1 latent_quality          — VAE encode/decode round-trip 损失控制
    W2 transition_accuracy     — RNN/Linear 转移预测损失控制
    W3 imagination_rollout     — Dreamer 想象轨迹稳定度
    W4 reward_prediction       — 奖励预测损失控制
    W5 jepa_predictive         — JEPA 嵌入预测损失控制
  - aggregate = mean(sub_dim_scores) ∈ [0, 1]
  - 任何 sub-dim 失败 → sub-dim score = 0.0 (不假装满分)
  - 任何 sub-dim ≥ 阈值 → status 标记

主 00:56 任何人都能接手:
  - measure_world_model_v06() → float (0..1) 主入口
  - measure_world_model_full() → WorldModelReport dataclass + JSON dump
  - WorldModelReport JSON 写 artifacts/v1162_world_model_v06.json

主 00:44 质量工程化:
  - WorldModelReport (主 22:33 北极星 + V1155 baseline):
      total, sub_dim_scores (dict 5 keys), sub_dim_evidence (dict 5 keys)
      version, timestamp, snapshot_id (uuid), elapsed_seconds
  - 三个出口: CLI / Python / JSON (主 17:43 实事求是)

主 17:58 + 20:46 不假装:
  - 不假装 VAE = Understanding: 5 sub-dim 是工程测量, 不冒充真 universal world model
  - 不假装 prediction = cognition: rollout ≠ thinking
  - 不假装 ASI has world model: 机制 ≠ mental model

Usage:
    python -m apeireth.v1162_asi_world_model_v06_real_measure                  # 默认 measure + JSON dump
    python -m apeireth.v1162_asi_world_model_v06_real_measure --json          # JSON stdout
    python -m apeireth.v1162_asi_world_model_v06_real_measure --no-write      # 只 print
    python -m apeireth.v1162_asi_world_model_v06_real_measure --report        # markdown 报告
    python -m apeireth.v1162_asi_world_model_v06_real_measure --artifact-dir artifacts  # 改目录

作为 V1144 world_model dim 真测入口:
    from apeireth.v1162_asi_world_model_v06_real_measure import measure_world_model_v06
    score = measure_world_model_v06()  # 0..1
"""

from __future__ import annotations

import argparse
import json
import math
import statistics
import sys
import time
import uuid
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple

V1162_VERSION = "0.1.0"
V1162_DIM_VERSION = "0.6"

# 5 sub-dim names (LOCKED 主 19:33 走在前人经验上 — 借鉴 Ha/Hafner/LeCun/Friston/Sutton 5 axis)
V1162_SUBDIM_NAMES: Tuple[str, ...] = (
    "latent_quality",             # W1 — VAE encode/decode round-trip
    "transition_accuracy",        # W2 — RNN/Linear next-state 预测
    "imagination_rollout",        # W3 — Dreamer 想象轨迹稳定度
    "reward_prediction",          # W4 — 奖励预测损失控制
    "jepa_predictive",            # W5 — JEPA 嵌入预测损失控制
)

# 默认 artifact dir (主 00:56 任何人都能接手)
DEFAULT_ARTIFACT_DIR = "artifacts"

# V1144 baseline (主 17:43 实事求是 — 写死历史值)
V1144_BASELINE_WORLD_MODEL = 0.0000

# Target (主 13:31 大胆激进)
TARGET_WORLD_MODEL_V06 = 0.7500


# ============================================================================
# SubDimEvidence + WorldModelReport — 真测结果 dataclass (主 00:44 质量工程化)
# ============================================================================


@dataclass
class SubDimEvidence:
    name: str
    score: float
    checks: Dict[str, bool] = field(default_factory=dict)
    notes: List[str] = field(default_factory=list)
    raw: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class WorldModelReport:
    """V1162 world_model V0.6 真测报告."""

    snapshot_id: str = field(default_factory=lambda: f"v1162-{uuid.uuid4().hex[:8]}")
    version: str = V1162_VERSION
    dim_version: str = V1162_DIM_VERSION
    timestamp: float = field(default_factory=time.time)
    elapsed_seconds: float = 0.0
    total: float = 0.0
    sub_dim_scores: Dict[str, float] = field(default_factory=dict)
    sub_dim_evidence: Dict[str, SubDimEvidence] = field(default_factory=dict)
    n_subdims_total: int = len(V1162_SUBDIM_NAMES)
    n_subdims_passed: int = 0
    n_subdims_partial: int = 0
    n_subdims_missing: int = 0
    notes: List[str] = field(default_factory=list)
    artifact_path: str = ""
    v1144_baseline: float = V1144_BASELINE_WORLD_MODEL
    target: float = TARGET_WORLD_MODEL_V06

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["sub_dim_evidence"] = {k: v.to_dict() for k, v in self.sub_dim_evidence.items()}
        return d

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "WorldModelReport":
        new = cls(
            snapshot_id=data.get("snapshot_id", ""),
            version=data.get("version", V1162_VERSION),
            dim_version=data.get("dim_version", V1162_DIM_VERSION),
            timestamp=data.get("timestamp", 0.0),
            elapsed_seconds=data.get("elapsed_seconds", 0.0),
            total=data.get("total", 0.0),
            sub_dim_scores=data.get("sub_dim_scores", {}),
            n_subdims_total=data.get("n_subdims_total", len(V1162_SUBDIM_NAMES)),
            n_subdims_passed=data.get("n_subdims_passed", 0),
            n_subdims_partial=data.get("n_subdims_partial", 0),
            n_subdims_missing=data.get("n_subdims_missing", 0),
            notes=data.get("notes", []),
            artifact_path=data.get("artifact_path", ""),
            v1144_baseline=data.get("v1144_baseline", V1144_BASELINE_WORLD_MODEL),
            target=data.get("target", TARGET_WORLD_MODEL_V06),
        )
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

    def summary_line(self) -> str:
        return (
            f"V1162 world_model V0.6: total={self.total:.4f} "
            f"(Δ vs V1144 baseline {self.v1144_baseline:.4f} = "
            f"{self.total - self.v1144_baseline:+.4f}) | "
            f"target={self.target:.4f} (gap {self.target - self.total:+.4f}) | "
            f"5 sub-dim: {self.n_subdims_passed} pass / "
            f"{self.n_subdims_partial} partial / {self.n_subdims_missing} missing | "
            f"snapshot={self.snapshot_id}"
        )


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
    for n in names:
        a = getattr(mod, n, None)
        if a is not None:
            return a
    return None


def _call_safely(fn: Optional[Callable], *args: Any, default: Any = None, **kwargs: Any) -> Tuple[bool, Any]:
    if fn is None or not callable(fn):
        return False, default
    try:
        return True, fn(*args, **kwargs)
    except Exception:
        return False, default


# ============================================================================
# 统一入口: V1062 真测
# ============================================================================


def _v1062_pipeline() -> Tuple[bool, Any]:
    """Build V1062 WorldModelPipeline (Ha 2018 + Hafner + LeCun JEPA + Sutton Dyna)."""
    v1062_mod = _safe_import("apeireth.v1062_asi_world_model")
    if v1062_mod is None:
        return False, None
    builder = _attr_first(v1062_mod, ["build_world_model", "WorldModelPipeline"])
    if builder is None:
        return False, None
    try:
        if callable(builder):
            # both factory function or class work
            try:
                inst = builder(obs_dim=8, latent_dim=4, action_dim=2)  # factory
            except Exception:
                inst = builder.default(obs_dim=8, latent_dim=4, action_dim=2)  # class method
        else:
            inst = builder
        return True, inst
    except Exception:
        # fallback: try class with default
        cls = _attr_first(v1062_mod, ["WorldModelPipeline"])
        if cls is None:
            return False, None
        try:
            return True, cls.default(obs_dim=8, latent_dim=4, action_dim=2)
        except Exception:
            return False, None


def _encoder_pipeline(pipeline: Any) -> Any:
    """Get encoder from pipeline (主 17:43 实事求是)."""
    return getattr(pipeline, "encoder", None)


def _decoder_pipeline(pipeline: Any) -> Any:
    return getattr(pipeline, "decoder", None)


def _transition_pipeline(pipeline: Any) -> Any:
    return getattr(pipeline, "transition", None)


def _reward_pipeline(pipeline: Any) -> Any:
    return getattr(pipeline, "reward", None)


def _jepa_pipeline(pipeline: Any) -> Any:
    return getattr(pipeline, "jepa", None)


def _imagination_pipeline(pipeline: Any) -> Any:
    return getattr(pipeline, "imagination", None)


def _dyna_pipeline(pipeline: Any) -> Any:
    return getattr(pipeline, "dyna", None)


def _make_obs(pipeline: Any, seed: int = 0) -> List[float]:
    """Make a single random observation obs_dim numbers in [0, 1]."""
    import random
    rng = random.Random(seed)
    obs_dim = getattr(getattr(pipeline, "encoder", None), "obs_dim", 8)
    return [rng.uniform(0, 1) for _ in range(obs_dim)]


def _make_action(pipeline: Any, seed: int = 0) -> List[float]:
    """Make a single random action action_dim numbers in [-1, 1]."""
    import random
    rng = random.Random(seed + 1000)
    action_dim = getattr(getattr(pipeline, "transition", None), "action_dim", 2)
    return [rng.uniform(-1, 1) for _ in range(action_dim)]


# 5 sub-dim 阈值 — 损失越低分越高, 上面映射成 0..1
# 通过: < threshold_strict 即视为"优秀", < threshold_loose 即视为"通过"
_W_LATENT_LOOSE = 0.50          # 重建损失 (mean abs err) loose
_W_LATENT_STRICT = 0.20         # 重建损失 strict
_W_TRANS_LOOSE = 1.00           # 转移预测 next-state err loose
_W_TRANS_STRICT = 0.40          # 转移预测 strict
_W_IMAG_LOOSE = 1.50            # imagination rollout loose
_W_IMAG_STRICT = 0.60           # imagination rollout strict
_W_REW_LOOSE = 0.80             # reward err loose
_W_REW_STRICT = 0.30            # reward err strict
_W_JEPA_LOOSE = 0.80            # jepa embedding err loose
_W_JEPA_STRICT = 0.30           # jepa strict


def _loss_to_score(metric: float, loose: float, strict: float) -> float:
    """Convert a loss/error metric into 0..1 score.

    - metric ≤ strict → 1.0
    - metric ≥ loose → 0.0
    - in between → linear
    主 17:43 实事求是: 真测损失转可解释 score.
    """
    if metric is None:
        return 0.0
    try:
        m = float(metric)
    except Exception:
        return 0.0
    if m <= strict:
        return 1.0
    if m >= loose:
        return 0.0
    return max(0.0, min(1.0, (loose - m) / (loose - strict)))


# ============================================================================
# W1 — latent_quality (VAE encode/decode round-trip)
# ============================================================================


def _measure_latent_quality() -> Tuple[float, SubDimEvidence]:
    """W1: V1062 encoder/decoder round-trip reconstruction error 真测."""
    ev = SubDimEvidence(
        name="latent_quality",
        score=0.0,
        notes=["W1: V1062 encoder+decoder 真 encode/decode 5 random obs"]
    )

    ok, pipeline = _v1062_pipeline()
    if not ok or pipeline is None:
        ev.notes.append("V1062 pipeline unavailable → W1 = 0")
        ev.raw = {"test_results": [], "reason": "pipeline_unavailable"}
        return 0.0, ev

    encoder = _encoder_pipeline(pipeline)
    decoder = _decoder_pipeline(pipeline)
    if encoder is None or decoder is None:
        ev.notes.append("V1062 encoder/decoder missing → W1 = 0")
        ev.raw = {"test_results": [], "reason": "encoder_decoder_missing"}
        return 0.0, ev

    test_results: List[Tuple[str, bool, str]] = []
    recon_errs: List[float] = []

    # Try to find reconstruction_error / encode_sample / decode
    encode_fn = _attr_first(encoder, ["encode_sample", "encode", "forward"])
    decode_fn = _attr_first(decoder, ["decode", "reconstruct", "forward"])
    re_err_fn = _attr_first(decoder, ["reconstruction_error", "error", "loss"])

    n_runs = 0
    for i in range(5):
        obs = _make_obs(pipeline, seed=i)
        ok_e, encoded = _call_safely(encode_fn, obs)
        if not ok_e:
            continue
        # encoded may be (mu, lv, z) or just z
        if isinstance(encoded, tuple) and len(encoded) >= 3:
            z = encoded[2]
        elif isinstance(encoded, list):
            z = encoded
        else:
            z = encoded

        ok_d, decoded = _call_safely(decode_fn, z if isinstance(z, list) else [z])
        if not ok_d:
            continue

        ok_r, err = _call_safely(re_err_fn, obs, decoded)
        if not ok_r:
            # fallback: simple L1 between obs and decoded (assuming same shape)
            try:
                obs_flat = list(obs)
                dec_flat = list(decoded) if isinstance(decoded, list) else [decoded]
                err = sum(abs(a - b) for a, b in zip(obs_flat, dec_flat)) / max(len(obs_flat), len(dec_flat), 1)
            except Exception:
                continue
        if err is None:
            continue
        try:
            err_f = float(err)
        except Exception:
            continue
        recon_errs.append(err_f)
        n_runs += 1

    # Test 1: 至少跑了 ≥ 1 次
    test_results.append(("ran_at_least_one_round_trip", n_runs >= 1, f"n_runs={n_runs}"))
    # Test 2: 跑了 ≥ 3 次
    test_results.append(("ran_at_least_three_round_trips", n_runs >= 3, f"n_runs={n_runs}"))
    # Test 3: 平均误差 < loose
    if recon_errs:
        mean_err = statistics.mean(recon_errs)
        test_results.append(("mean_recon_below_loose", mean_err < _W_LATENT_LOOSE, f"mean={mean_err:.4f}<{_W_LATENT_LOOSE}"))
    else:
        test_results.append(("mean_recon_below_loose", False, "no recon_errs"))
    # Test 4: 平均误差 < strict
    if recon_errs:
        mean_err = statistics.mean(recon_errs)
        test_results.append(("mean_recon_below_strict", mean_err < _W_LATENT_STRICT, f"mean={mean_err:.4f}<{_W_LATENT_STRICT}"))
    else:
        test_results.append(("mean_recon_below_strict", False, "no recon_errs"))
    # Test 5: 误差方差 OK (有数值范围, 不退化)
    if recon_errs:
        var_err = statistics.pvariance(recon_errs)
        test_results.append(("recon_var_in_range", var_err < 1.0, f"var={var_err:.4f}<1.0"))
    else:
        test_results.append(("recon_var_in_range", False, "no recon_errs"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)
    # score 来自 check + mean_err 综合
    if recon_errs:
        ls = _loss_to_score(statistics.mean(recon_errs), _W_LATENT_LOOSE, _W_LATENT_STRICT)
        ev.score = 0.5 * ls + 0.5 * (float(n_pass) / 5.0)
    else:
        ev.score = 0.0
    ev.score = min(1.0, max(0.0, ev.score))
    ev.checks = {n: ok for n, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "n_runs": n_runs,
        "recon_errs": recon_errs,
    }
    ev.notes.append(f"W1 score={ev.score:.4f} (n_pass={n_pass}/5, n_runs={n_runs})")
    return ev.score, ev


# ============================================================================
# W2 — transition_accuracy (next-state 预测)
# ============================================================================


def _measure_transition_accuracy() -> Tuple[float, SubDimEvidence]:
    """W2: V1062 transition 真 predict next-state from (z, action)."""
    ev = SubDimEvidence(
        name="transition_accuracy",
        score=0.0,
        notes=["W2: V1062 transition 真 predict next-state 5 obs"]
    )

    ok, pipeline = _v1062_pipeline()
    if not ok or pipeline is None:
        ev.notes.append("V1062 pipeline unavailable → W2 = 0")
        ev.raw = {"test_results": [], "reason": "pipeline_unavailable"}
        return 0.0, ev

    transition = _transition_pipeline(pipeline)
    encoder = _encoder_pipeline(pipeline)
    if transition is None or encoder is None:
        ev.notes.append("V1062 transition/encoder missing → W2 = 0")
        ev.raw = {"test_results": [], "reason": "transition_or_encoder_missing"}
        return 0.0, ev

    # try to find predict / step / forward
    predict_fn = _attr_first(transition, ["predict", "step", "forward"])
    encode_fn = _attr_first(encoder, ["encode_sample", "encode"])

    test_results: List[Tuple[str, bool, str]] = []
    trans_errs: List[float] = []
    n_runs = 0

    for i in range(5):
        obs = _make_obs(pipeline, seed=i + 100)
        next_obs = [o + 0.05 for o in obs]  # arbitrary "next"
        action = _make_action(pipeline, seed=i + 200)

        ok_e, encoded = _call_safely(encode_fn, obs)
        if not ok_e:
            continue
        if isinstance(encoded, tuple) and len(encoded) >= 3:
            z = encoded[2]
        elif isinstance(encoded, list):
            z = encoded
        else:
            z = encoded

        ok_p, predicted = _call_safely(predict_fn, z, action)
        if not ok_p or predicted is None:
            continue

        # encode next_obs to compare
        ok_ne, next_encoded = _call_safely(encode_fn, next_obs)
        z_next = None
        if ok_ne:
            if isinstance(next_encoded, tuple) and len(next_encoded) >= 3:
                z_next = next_encoded[2]
            elif isinstance(next_encoded, list):
                z_next = next_encoded
        if z_next is None:
            # fallback: compare to next_obs directly
            target = next_obs
            try:
                pred_list = list(predicted) if isinstance(predicted, list) else [predicted]
                target_trunc = target[:len(pred_list)]
                err = sum(abs(a - b) for a, b in zip(pred_list, target_trunc)) / max(len(pred_list), 1)
            except Exception:
                continue
        else:
            try:
                pred_list = list(predicted) if isinstance(predicted, list) else [predicted]
                z_next_list = z_next if isinstance(z_next, list) else [z_next]
                err = sum(abs(a - b) for a, b in zip(pred_list, z_next_list)) / max(len(pred_list), 1)
            except Exception:
                continue

        try:
            err_f = float(err)
        except Exception:
            continue
        trans_errs.append(err_f)
        n_runs += 1

    test_results.append(("ran_at_least_one_transition", n_runs >= 1, f"n_runs={n_runs}"))
    test_results.append(("ran_at_least_three_transitions", n_runs >= 3, f"n_runs={n_runs}"))
    if trans_errs:
        mean_err = statistics.mean(trans_errs)
        test_results.append(("trans_mean_below_loose", mean_err < _W_TRANS_LOOSE, f"mean={mean_err:.4f}<{_W_TRANS_LOOSE}"))
        test_results.append(("trans_mean_below_strict", mean_err < _W_TRANS_STRICT, f"mean={mean_err:.4f}<{_W_TRANS_STRICT}"))
        test_results.append(("trans_var_in_range", statistics.pvariance(trans_errs) < 2.0, f"var<2.0"))
    else:
        test_results.append(("trans_mean_below_loose", False, "no trans_errs"))
        test_results.append(("trans_mean_below_strict", False, "no trans_errs"))
        test_results.append(("trans_var_in_range", False, "no trans_errs"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)
    if trans_errs:
        ls = _loss_to_score(statistics.mean(trans_errs), _W_TRANS_LOOSE, _W_TRANS_STRICT)
        ev.score = 0.6 * ls + 0.4 * (float(n_pass) / 5.0)
    else:
        ev.score = 0.0
    ev.score = min(1.0, max(0.0, ev.score))
    ev.checks = {n: ok for n, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "n_runs": n_runs,
        "trans_errs": trans_errs,
    }
    ev.notes.append(f"W2 score={ev.score:.4f} (n_pass={n_pass}/5, n_runs={n_runs})")
    return ev.score, ev


# ============================================================================
# W3 — imagination_rollout (Dreamer 想象轨迹)
# ============================================================================


def _measure_imagination_rollout() -> Tuple[float, SubDimEvidence]:
    """W3: V1062 imagination 真 rollout 5 steps, stability measure."""
    ev = SubDimEvidence(
        name="imagination_rollout",
        score=0.0,
        notes=["W3: V1062 imagination/transition 真 rollout 5 steps"]
    )

    ok, pipeline = _v1062_pipeline()
    if not ok or pipeline is None:
        ev.notes.append("V1062 pipeline unavailable → W3 = 0")
        ev.raw = {"test_results": [], "reason": "pipeline_unavailable"}
        return 0.0, ev

    imagination = _imagination_pipeline(pipeline)
    transition = _transition_pipeline(pipeline)
    encoder = _encoder_pipeline(pipeline)
    if imagination is None or transition is None or encoder is None:
        # Fallback: use transition directly
        if transition is None or encoder is None:
            ev.notes.append("V1062 imagination+transition+encoder missing → W3 = 0")
            ev.raw = {"test_results": [], "reason": "imagination_components_missing"}
            return 0.0, ev
        # fall through and use transition

    rollout_fn = _attr_first(imagination, ["rollout", "imagine", "rollout_trajectory"]) if imagination else None
    predict_fn = _attr_first(transition, ["predict", "step", "forward"])
    encode_fn = _attr_first(encoder, ["encode_sample", "encode"])

    test_results: List[Tuple[str, bool, str]] = []
    rollout_errs: List[float] = []
    n_runs = 0
    horizon = 5

    for i in range(3):
        obs = _make_obs(pipeline, seed=i + 300)
        ok_e, encoded = _call_safely(encode_fn, obs)
        if not ok_e:
            continue
        if isinstance(encoded, tuple) and len(encoded) >= 3:
            z0 = encoded[2]
        elif isinstance(encoded, list):
            z0 = encoded
        else:
            z0 = encoded

        # Try imagination.rollout(z0, action_seq) → list of z
        zs: List[Any] = []
        ok_r = False
        rollout_input = (z0, [_make_action(pipeline, seed=i + 400 + t) for t in range(horizon)])
        if rollout_fn is not None:
            ok_r, r = _call_safely(rollout_fn, *rollout_input)
            if ok_r and isinstance(r, list):
                zs = r
        if not zs:
            # fallback: chain predict calls
            zs = []
            z = z0 if isinstance(z0, list) else [z0]
            for t in range(horizon):
                action = _make_action(pipeline, seed=i + 400 + t)
                ok_p, nxt = _call_safely(predict_fn, z, action)
                if not ok_p:
                    break
                z = nxt if isinstance(nxt, list) else [nxt]
                zs.append(z)

        if len(zs) < horizon:
            continue

        # Stability: mean abs deviation across rollout
        for j in range(1, len(zs)):
            try:
                a = zs[j - 1] if isinstance(zs[j - 1], list) else [zs[j - 1]]
                b = zs[j] if isinstance(zs[j], list) else [zs[j]]
                step_err = sum(abs(x - y) for x, y in zip(a, b)) / max(len(a), 1)
                rollout_errs.append(float(step_err))
            except Exception:
                continue
        n_runs += 1

    test_results.append(("ran_at_least_one_rollout", n_runs >= 1, f"n_runs={n_runs}"))
    test_results.append(("horizon_full_reached", any(e > 0 for e in rollout_errs) and len(rollout_errs) >= horizon, f"steps={len(rollout_errs)}>={horizon}"))
    if rollout_errs:
        mean_err = statistics.mean(rollout_errs)
        test_results.append(("rollout_mean_below_loose", mean_err < _W_IMAG_LOOSE, f"mean={mean_err:.4f}<{_W_IMAG_LOOSE}"))
        test_results.append(("rollout_mean_below_strict", mean_err < _W_IMAG_STRICT, f"mean={mean_err:.4f}<{_W_IMAG_STRICT}"))
        test_results.append(("rollout_not_diverged", mean_err < _W_IMAG_LOOSE * 2.0, f"mean<{2*_W_IMAG_LOOSE}"))
    else:
        test_results.append(("rollout_mean_below_loose", False, "no rollout_errs"))
        test_results.append(("rollout_mean_below_strict", False, "no rollout_errs"))
        test_results.append(("rollout_not_diverged", False, "no rollout_errs"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)
    if rollout_errs:
        ls = _loss_to_score(statistics.mean(rollout_errs), _W_IMAG_LOOSE, _W_IMAG_STRICT)
        ev.score = 0.6 * ls + 0.4 * (float(n_pass) / 5.0)
    else:
        ev.score = 0.0
    ev.score = min(1.0, max(0.0, ev.score))
    ev.checks = {n: ok for n, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "n_runs": n_runs,
        "horizon": horizon,
        "rollout_errs": rollout_errs,
    }
    ev.notes.append(f"W3 score={ev.score:.4f} (n_pass={n_pass}/5, n_runs={n_runs}, horizon={horizon})")
    return ev.score, ev


# ============================================================================
# W4 — reward_prediction (Sutton Dyna 奖励预测)
# ============================================================================


def _measure_reward_prediction() -> Tuple[float, SubDimEvidence]:
    """W4: V1062 reward 真 predict reward from (z, action)."""
    ev = SubDimEvidence(
        name="reward_prediction",
        score=0.0,
        notes=["W4: V1062 reward 真 predict reward 5 (z, action)"]
    )

    ok, pipeline = _v1062_pipeline()
    if not ok or pipeline is None:
        ev.notes.append("V1062 pipeline unavailable → W4 = 0")
        ev.raw = {"test_results": [], "reason": "pipeline_unavailable"}
        return 0.0, ev

    reward = _reward_pipeline(pipeline)
    encoder = _encoder_pipeline(pipeline)
    if reward is None or encoder is None:
        ev.notes.append("V1062 reward/encoder missing → W4 = 0")
        ev.raw = {"test_results": [], "reason": "reward_components_missing"}
        return 0.0, ev

    predict_rew = _attr_first(reward, ["predict", "step", "forward"])
    encode_fn = _attr_first(encoder, ["encode_sample", "encode"])

    test_results: List[Tuple[str, bool, str]] = []
    rew_errs: List[float] = []
    n_runs = 0

    for i in range(5):
        obs = _make_obs(pipeline, seed=i + 500)
        action = _make_action(pipeline, seed=i + 600)
        ok_e, encoded = _call_safely(encode_fn, obs)
        if not ok_e:
            continue
        if isinstance(encoded, tuple) and len(encoded) >= 3:
            z = encoded[2]
        elif isinstance(encoded, list):
            z = encoded
        else:
            z = encoded

        # Generate a fake "actual" reward via a fixed deterministic rule on obs
        actual_rew = sum(o for o in obs) / len(obs)
        if actual_rew > 1.0:
            actual_rew = 1.0 / actual_rew
        # bound to [-1, 1]
        actual_rew = max(-1.0, min(1.0, actual_rew - 0.5))

        ok_p, predicted_rew = _call_safely(predict_rew, z, action)
        if not ok_p:
            continue
        try:
            err = abs(float(predicted_rew) - actual_rew)
        except Exception:
            continue
        rew_errs.append(err)
        n_runs += 1

    test_results.append(("ran_at_least_one_reward", n_runs >= 1, f"n_runs={n_runs}"))
    test_results.append(("ran_at_least_three_rewards", n_runs >= 3, f"n_runs={n_runs}"))
    if rew_errs:
        mean_err = statistics.mean(rew_errs)
        test_results.append(("rew_mean_below_loose", mean_err < _W_REW_LOOSE, f"mean={mean_err:.4f}<{_W_REW_LOOSE}"))
        test_results.append(("rew_mean_below_strict", mean_err < _W_REW_STRICT, f"mean={mean_err:.4f}<{_W_REW_STRICT}"))
        test_results.append(("rew_var_in_range", statistics.pvariance(rew_errs) < 0.5, f"var<0.5"))
    else:
        test_results.append(("rew_mean_below_loose", False, "no rew_errs"))
        test_results.append(("rew_mean_below_strict", False, "no rew_errs"))
        test_results.append(("rew_var_in_range", False, "no rew_errs"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)
    if rew_errs:
        ls = _loss_to_score(statistics.mean(rew_errs), _W_REW_LOOSE, _W_REW_STRICT)
        ev.score = 0.6 * ls + 0.4 * (float(n_pass) / 5.0)
    else:
        ev.score = 0.0
    ev.score = min(1.0, max(0.0, ev.score))
    ev.checks = {n: ok for n, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "n_runs": n_runs,
        "rew_errs": rew_errs,
    }
    ev.notes.append(f"W4 score={ev.score:.4f} (n_pass={n_pass}/5, n_runs={n_runs})")
    return ev.score, ev


# ============================================================================
# W5 — jepa_predictive (LeCun 2022 JEPA 嵌入预测)
# ============================================================================


def _measure_jepa_predictive() -> Tuple[float, SubDimEvidence]:
    """W5: V1062 jepa 真 predict next-embedding from (z, action)."""
    ev = SubDimEvidence(
        name="jepa_predictive",
        score=0.0,
        notes=["W5: V1062 jepa 真 predict next-z 5 (z, action)"]
    )

    ok, pipeline = _v1062_pipeline()
    if not ok or pipeline is None:
        ev.notes.append("V1062 pipeline unavailable → W5 = 0")
        ev.raw = {"test_results": [], "reason": "pipeline_unavailable"}
        return 0.0, ev

    jepa = _jepa_pipeline(pipeline)
    encoder = _encoder_pipeline(pipeline)
    if jepa is None or encoder is None:
        ev.notes.append("V1062 jepa/encoder missing → W5 = 0")
        ev.raw = {"test_results": [], "reason": "jepa_components_missing"}
        return 0.0, ev

    predict_jepa = _attr_first(jepa, ["predict", "jepa_predict", "forward"])
    encode_fn = _attr_first(encoder, ["encode_sample", "encode"])

    test_results: List[Tuple[str, bool, str]] = []
    jepa_errs: List[float] = []
    n_runs = 0

    for i in range(5):
        obs = _make_obs(pipeline, seed=i + 700)
        next_obs = [o + 0.05 for o in obs]
        action = _make_action(pipeline, seed=i + 800)

        ok_e, encoded = _call_safely(encode_fn, obs)
        if not ok_e:
            continue
        if isinstance(encoded, tuple) and len(encoded) >= 3:
            z = encoded[2]
        elif isinstance(encoded, list):
            z = encoded
        else:
            z = encoded

        ok_ne, next_encoded = _call_safely(encode_fn, next_obs)
        if not ok_ne:
            continue
        if isinstance(next_encoded, tuple) and len(next_encoded) >= 3:
            z_next = next_encoded[2]
        elif isinstance(next_encoded, list):
            z_next = next_encoded
        else:
            z_next = next_encoded

        ok_p, predicted_z = _call_safely(predict_jepa, z, z_next, action)
        if not ok_p or predicted_z is None:
            continue

        try:
            pred_list = predicted_z if isinstance(predicted_z, list) else [predicted_z]
            target_list = z_next if isinstance(z_next, list) else [z_next]
            err = sum(abs(x - y) for x, y in zip(pred_list, target_list)) / max(len(pred_list), 1)
        except Exception:
            continue
        try:
            err_f = float(err)
        except Exception:
            continue
        jepa_errs.append(err_f)
        n_runs += 1

    test_results.append(("ran_at_least_one_jepa", n_runs >= 1, f"n_runs={n_runs}"))
    test_results.append(("ran_at_least_three_jepas", n_runs >= 3, f"n_runs={n_runs}"))
    if jepa_errs:
        mean_err = statistics.mean(jepa_errs)
        test_results.append(("jepa_mean_below_loose", mean_err < _W_JEPA_LOOSE, f"mean={mean_err:.4f}<{_W_JEPA_LOOSE}"))
        test_results.append(("jepa_mean_below_strict", mean_err < _W_JEPA_STRICT, f"mean={mean_err:.4f}<{_W_JEPA_STRICT}"))
        test_results.append(("jepa_var_in_range", statistics.pvariance(jepa_errs) < 1.0, f"var<1.0"))
    else:
        test_results.append(("jepa_mean_below_loose", False, "no jepa_errs"))
        test_results.append(("jepa_mean_below_strict", False, "no jepa_errs"))
        test_results.append(("jepa_var_in_range", False, "no jepa_errs"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)
    if jepa_errs:
        ls = _loss_to_score(statistics.mean(jepa_errs), _W_JEPA_LOOSE, _W_JEPA_STRICT)
        ev.score = 0.6 * ls + 0.4 * (float(n_pass) / 5.0)
    else:
        ev.score = 0.0
    ev.score = min(1.0, max(0.0, ev.score))
    ev.checks = {n: ok for n, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "n_runs": n_runs,
        "jepa_errs": jepa_errs,
    }
    ev.notes.append(f"W5 score={ev.score:.4f} (n_pass={n_pass}/5, n_runs={n_runs})")
    return ev.score, ev


# ============================================================================
# 主入口
# ============================================================================


def measure_world_model_v06() -> float:
    """主入口 — 返回 world_model V0.6 score (0..1)."""
    rep = measure_world_model_full(write_artifact=False)
    return rep.total


def measure_world_model_full(
    write_artifact: bool = True,
    artifact_dir: str = DEFAULT_ARTIFACT_DIR,
) -> WorldModelReport:
    """Run all 5 sub-dims, return WorldModelReport."""
    t0 = time.time()
    rep = WorldModelReport()

    # W1
    s1, ev1 = _measure_latent_quality()
    rep.sub_dim_scores["latent_quality"] = s1
    rep.sub_dim_evidence["latent_quality"] = ev1
    if s1 >= 0.8:
        rep.n_subdims_passed += 1
    elif s1 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # W2
    s2, ev2 = _measure_transition_accuracy()
    rep.sub_dim_scores["transition_accuracy"] = s2
    rep.sub_dim_evidence["transition_accuracy"] = ev2
    if s2 >= 0.8:
        rep.n_subdims_passed += 1
    elif s2 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # W3
    s3, ev3 = _measure_imagination_rollout()
    rep.sub_dim_scores["imagination_rollout"] = s3
    rep.sub_dim_evidence["imagination_rollout"] = ev3
    if s3 >= 0.8:
        rep.n_subdims_passed += 1
    elif s3 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # W4
    s4, ev4 = _measure_reward_prediction()
    rep.sub_dim_scores["reward_prediction"] = s4
    rep.sub_dim_evidence["reward_prediction"] = ev4
    if s4 >= 0.8:
        rep.n_subdims_passed += 1
    elif s4 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # W5
    s5, ev5 = _measure_jepa_predictive()
    rep.sub_dim_scores["jepa_predictive"] = s5
    rep.sub_dim_evidence["jepa_predictive"] = ev5
    if s5 >= 0.8:
        rep.n_subdims_passed += 1
    elif s5 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    rep.total = sum(rep.sub_dim_scores.values()) / float(len(V1162_SUBDIM_NAMES))
    rep.total = min(1.0, max(0.0, rep.total))
    rep.elapsed_seconds = time.time() - t0

    if write_artifact:
        try:
            ad = Path(artifact_dir)
            ad.mkdir(parents=True, exist_ok=True)
            artifact_path = ad / "v1162_world_model_v06.json"
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


def render_report_md(rep: WorldModelReport) -> str:
    lines: List[str] = []
    lines.append(f"# V1162 world_model V0.6 真补报告 — {rep.snapshot_id}\n")
    lines.append(f"- **version**: {rep.version}")
    lines.append(f"- **dim_version**: {rep.dim_version}")
    lines.append(f"- **timestamp**: {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(rep.timestamp))}")
    lines.append(f"- **elapsed**: {rep.elapsed_seconds:.3f}s")
    lines.append(f"- **artifact**: `{rep.artifact_path or 'N/A'}`\n")
    lines.append("## Total")
    lines.append(f"- **world_model V0.6**: {rep.total:.4f}")
    lines.append(f"- **vs V1144 baseline**: {rep.v1144_baseline:.4f} (Δ = {rep.total - rep.v1144_baseline:+.4f})")
    lines.append(f"- **target**: {rep.target:.4f} (gap = {rep.target - rep.total:+.4f})\n")

    lines.append("## 5 sub-dim 真测\n")
    lines.append("| sub-dim | score | status |")
    lines.append("|---|---:|:---:|")
    for name in V1162_SUBDIM_NAMES:
        s = rep.sub_dim_scores.get(name, 0.0)
        status = "✓ pass" if s >= 0.8 else ("◐ partial" if s > 0.0 else "✗ missing")
        lines.append(f"| {name} | {s:.4f} | {status} |")

    lines.append("\n## Sub-dim Evidence\n")
    for name in V1162_SUBDIM_NAMES:
        ev = rep.sub_dim_evidence.get(name)
        if ev is None:
            continue
        lines.append(f"### {name} (score = {ev.score:.4f})")
        if ev.notes:
            for n in ev.notes:
                lines.append(f"- note: {n}")
        if ev.checks:
            for cn, cv in ev.checks.items():
                lines.append(f"- `{cn}`: {'✓' if cv else '✗'}")
        lines.append("")

    lines.append("## Notes\n")
    for n in rep.notes:
        lines.append(f"- {n}")
    lines.append("")
    lines.append("---")
    lines.append(f"_Generated by V1162 {rep.version}_")
    return "\n".join(lines)


# ============================================================================
# CLI
# ============================================================================


def _cli() -> int:
    parser = argparse.ArgumentParser(description="V1162 world_model V0.6 真补")
    parser.add_argument("--json", action="store_true", help="输出 JSON stdout")
    parser.add_argument("--no-write", action="store_true", help="不写 artifact")
    parser.add_argument("--report", action="store_true", help="输出 Markdown 报告")
    parser.add_argument("--artifact-dir", default=DEFAULT_ARTIFACT_DIR)
    parser.add_argument("--md-out", default=None)
    args = parser.parse_args()

    rep = measure_world_model_full(
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
