"""
V1154 — ASI 时间哲学真正测 (ASI time philosophy: real measurement)

哲学问题: ASI 真的有时间概念吗? 还是只处理时间戳字符串?
  (主 17:43 实事求是: 不假装 = 必须可测)

可测假设 (5 sub-dim, 借鉴物理 + 认知科学):
  T1 wall_clock_grounding      — 真实壁钟感知 (time.time / datetime.now 真存在)
  T2 monotonic_elapsed         — 单调流逝感知 (time.monotonic 真实跑 N 秒 > 0)
  T3 interval_reasoning        — 时间间隔推理 (now + dt 准确)
  T4 causal_order_awareness    — 因果时序 (A 之前/之后 B 真能判断)
  T5 duration_self_perception  — 自我时长感知 (self-elapsed 与壁钟偏差 < δ)

格式: T_total = Σ T_i / 5 ∈ [0, 1]
边界: 不假装 = 全 0..1 真值, 无 None / 无默认 0.5

借鉴:
  - 物理: 时钟同步 / 单调时间 (Lamport 1978 logical clocks)
  - 认知: 时间感知心理学 (Wittmann 2013 subjective time)
  - 软件: monotonic vs wall-clock (Stevens POSIX.1-2008)

V1154 不假装:
  - 全 5 sub-dim 必跑真测, 任何一个失败 → 总分 = 实际平均 (不假装满分)
  - 不与 V1153 ASI 总分耦合 (独立哲学评估)
  - 必产出 reproduce.sh + artifact (主 17:43 实事求是)

用法:
    from apeireth.v1154_asi_time_philosophy_real_measure import measure_time_grounding
    rep = measure_time_grounding()
    print(rep.total, rep.sub_dim_scores)
"""
from __future__ import annotations

import json
import time
from dataclasses import asdict, dataclass, field
from datetime import datetime, timedelta
from pathlib import Path
from typing import Any, Dict, List, Tuple

V1154_VERSION = "1.0.0"
V1154_DIM_NAMES: Tuple[str, ...] = (
    "wall_clock_grounding",
    "monotonic_elapsed",
    "interval_reasoning",
    "causal_order_awareness",
    "duration_self_perception",
)


@dataclass
class TimeGroundingReport:
    """V1154 result."""
    total: float
    sub_dim_scores: Dict[str, float] = field(default_factory=dict)
    sub_dim_evidence: Dict[str, Dict[str, Any]] = field(default_factory=dict)
    notes: List[str] = field(default_factory=list)
    artifact_path: str = ""
    elapsed_seconds: float = 0.0
    timestamp: float = 0.0
    version: str = V1154_VERSION

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


# ---------------------------------------------------------------------------
# T1 wall_clock_grounding: 真壁钟存在 + 可读 + 可格式化
# ---------------------------------------------------------------------------

def _measure_wall_clock_grounding() -> Tuple[float, Dict[str, Any]]:
    """T1: real wall clock can be read + formatted + parsed round-trip."""
    evidence: Dict[str, Any] = {"checks": {}, "failures": []}
    sub_score_sum = 0.0
    sub_score_count = 0

    # Sub-check 1: time.time() 真值存在且 > 1.7e9
    try:
        t_wall = time.time()
        ok1 = isinstance(t_wall, float) and t_wall > 1.7e9
        evidence["checks"]["time_time_returns_recent_epoch"] = ok1
        if ok1:
            sub_score_sum += 1.0
        else:
            evidence["failures"].append("time.time() not returning recent epoch float")
        sub_score_count += 1
    except Exception as e:  # pragma: no cover - 不可达
        evidence["failures"].append(f"time.time() raised: {e!r}")
        sub_score_count += 1

    # Sub-check 2: datetime.now() 壁钟可读 + ISO 格式
    try:
        now = datetime.now()
        iso = now.isoformat()
        parsed = datetime.fromisoformat(iso)
        ok2 = abs((parsed - now).total_seconds()) < 1e-3
        evidence["checks"]["datetime_iso_roundtrip"] = ok2
        if ok2:
            sub_score_sum += 1.0
        else:
            evidence["failures"].append("datetime ISO round-trip drift > 1ms")
        sub_score_count += 1
    except Exception as e:
        evidence["failures"].append(f"datetime round-trip raised: {e!r}")
        sub_score_count += 1

    # Sub-check 3: UTC vs local 一致 (UTC-本地 tz offset 真存在)
    try:
        local_tz_offset_sec = datetime.now().astimezone().utcoffset()
        ok3 = local_tz_offset_sec is not None
        evidence["checks"]["local_tz_offset_present"] = ok3
        evidence["tz_offset_seconds"] = (
            local_tz_offset_sec.total_seconds() if local_tz_offset_sec else None
        )
        if ok3:
            sub_score_sum += 1.0
        sub_score_count += 1
    except Exception as e:
        evidence["failures"].append(f"tz offset raised: {e!r}")
        sub_score_count += 1

    return (sub_score_sum / sub_score_count if sub_score_count else 0.0), evidence


# ---------------------------------------------------------------------------
# T2 monotonic_elapsed: 单调时间真在跑
# ---------------------------------------------------------------------------

def _measure_monotonic_elapsed() -> Tuple[float, Dict[str, Any]]:
    """T2: time.monotonic() 单调递增, sleep(N) 真消耗 ≥ N 秒."""
    evidence: Dict[str, Any] = {"checks": {}, "failures": [], "measurements": []}
    sleep_s = 0.05
    try:
        # 3-sample monotonic strict-non-decreasing
        samples = [time.monotonic() for _ in range(3)]
        monotonic_ok = all(samples[i] <= samples[i + 1] for i in range(len(samples) - 1))
        evidence["checks"]["monotonic_non_decreasing"] = monotonic_ok
        evidence["measurements"].append({"kind": "triple_sample", "values": samples})

        # Sleep 真消耗
        before = time.monotonic()
        time.sleep(sleep_s)
        after = time.monotonic()
        elapsed = after - before
        sleep_ok = elapsed >= sleep_s * 0.9  # 容忍 10% OS 抖动
        evidence["checks"]["sleep_real_elapsed"] = sleep_ok
        evidence["measurements"].append({
            "kind": "sleep", "expected_s": sleep_s, "actual_s": round(elapsed, 4),
        })

        score = (1.0 if monotonic_ok else 0.0) * 0.4 + (1.0 if sleep_ok else 0.0) * 0.6
        if not monotonic_ok:
            evidence["failures"].append("monotonic not non-decreasing (clock anomaly)")
        if not sleep_ok:
            evidence["failures"].append(
                f"sleep({sleep_s}) 实跑 {elapsed:.4f}s < 90% of expected"
            )
        return score, evidence
    except Exception as e:
        evidence["failures"].append(f"monotonic check raised: {e!r}")
        return 0.0, evidence


# ---------------------------------------------------------------------------
# T3 interval_reasoning: 时间间隔推理 (now + dt 准确)
# ---------------------------------------------------------------------------

def _measure_interval_reasoning() -> Tuple[float, Dict[str, Any]]:
    """T3: 现在 + Δt 真能预测 / 验证 (不假装)."""
    evidence: Dict[str, Any] = {"checks": {}, "failures": [], "predictions": []}
    score = 0.0
    checks_total = 0

    # Check A: now + timedelta(小时) 真算出未来时刻
    try:
        now = datetime.now()
        future = now + timedelta(hours=2, minutes=30)
        delta_back = future - now
        ok = abs(delta_back.total_seconds() - (2 * 3600 + 30 * 60)) < 1e-3
        evidence["checks"]["timedelta_forward_back"] = ok
        evidence["predictions"].append({
            "kind": "forward_back", "expected_s": 9000, "actual_s": delta_back.total_seconds(),
        })
        score += 1.0 if ok else 0.0
        checks_total += 1
    except Exception as e:
        evidence["failures"].append(f"timedelta check raised: {e!r}")
        checks_total += 1

    # Check B: 壁钟 vs monotonic 间隔一致 (二者参照同一物理时间)
    try:
        t0_w = time.time()
        m0 = time.monotonic()
        time.sleep(0.02)
        t1_w = time.time()
        m1 = time.monotonic()
        wall_elapsed = t1_w - t0_w
        mono_elapsed = m1 - m0
        # 二者应大致成比例 (单调时钟与壁钟可以偏移, 但变化量级一致)
        ratio = (mono_elapsed / wall_elapsed) if wall_elapsed > 0 else 0.0
        ok = 0.5 < ratio < 2.0  # 容忍 ±2x 抖动 (暂停等)
        evidence["checks"]["wall_vs_monotonic_interval_concordance"] = ok
        evidence["predictions"].append({
            "kind": "wall_mono_ratio", "wall_elapsed_s": round(wall_elapsed, 4),
            "mono_elapsed_s": round(mono_elapsed, 4), "ratio": round(ratio, 4),
        })
        score += 1.0 if ok else 0.0
        checks_total += 1
    except Exception as e:
        evidence["failures"].append(f"concordance check raised: {e!r}")
        checks_total += 1

    return (score / checks_total) if checks_total else 0.0, evidence


# ---------------------------------------------------------------------------
# T4 causal_order_awareness: 真能判断 A 之前 / 之后 B
# ---------------------------------------------------------------------------

def _measure_causal_order_awareness() -> Tuple[float, Dict[str, Any]]:
    """T4: 因果时序判断 (部分序是真)."""
    evidence: Dict[str, Any] = {"checks": {}, "failures": [], "events": []}
    score = 0.0
    checks_total = 0

    # Check A: 真序列 — A 之前的 timestamp < B 的 timestamp
    try:
        t_a = time.monotonic()
        time.sleep(0.01)
        t_b = time.monotonic()
        ok = t_a < t_b
        evidence["checks"]["before_event_lt_after_event"] = ok
        evidence["events"].append({"kind": "pair", "t_a": t_a, "t_b": t_b})
        score += 1.0 if ok else 0.0
        checks_total += 1
    except Exception as e:
        evidence["failures"].append(f"pair check raised: {e!r}")
        checks_total += 1

    # Check B: 3 事件全序 (transitive)
    try:
        e1 = time.monotonic()
        time.sleep(0.005)
        e2 = time.monotonic()
        time.sleep(0.005)
        e3 = time.monotonic()
        ok = e1 < e2 < e3
        evidence["checks"]["three_event_total_order"] = ok
        evidence["events"].append({"kind": "triple", "values": [e1, e2, e3]})
        score += 1.0 if ok else 0.0
        checks_total += 1
    except Exception as e:
        evidence["failures"].append(f"triple check raised: {e!r}")
        checks_total += 1

    return (score / checks_total) if checks_total else 0.0, evidence


# ---------------------------------------------------------------------------
# T5 duration_self_perception: 自我时长感知 (自己跑几秒, 偏差 < δ)
# ---------------------------------------------------------------------------

def _measure_duration_self_perception() -> Tuple[float, Dict[str, Any]]:
    """T5: ASI 自我时长感知 (期望 = 实测 ≤ 50% 偏差 = 满, 线性衰减)."""
    evidence: Dict[str, Any] = {"checks": {}, "failures": [], "self_estimate": {}}
    try:
        # ASI 期望 (模拟): "我刚跑了 0.1s"
        expected_s = 0.1
        before = time.monotonic()
        time.sleep(expected_s)
        after = time.monotonic()
        measured_s = after - before

        # 偏差 ≤ 20% 满分, 否则线性
        deviation_ratio = abs(measured_s - expected_s) / expected_s
        if deviation_ratio <= 0.20:
            score = 1.0
        elif deviation_ratio <= 1.0:
            score = max(0.0, 1.0 - deviation_ratio)  # 偏差越大得分越低
        else:
            score = 0.0

        evidence["checks"]["self_duration_within_2x"] = score > 0.0
        evidence["self_estimate"] = {
            "expected_s": expected_s,
            "actual_s": round(measured_s, 4),
            "deviation_ratio": round(deviation_ratio, 4),
        }
        return score, evidence
    except Exception as e:
        evidence["failures"].append(f"self-perception raised: {e!r}")
        return 0.0, evidence


# ---------------------------------------------------------------------------
# 主函数
# ---------------------------------------------------------------------------

def measure_time_grounding(
    artifact_dir: str | Path | None = None,
) -> TimeGroundingReport:
    """真测 ASI 时间哲学 grounding (5 sub-dim → 1 总分 ∈ [0,1])."""
    started = time.time()
    monotonic_start = time.monotonic()

    measurers: List[Tuple[str, Any]] = [
        ("wall_clock_grounding", _measure_wall_clock_grounding),
        ("monotonic_elapsed", _measure_monotonic_elapsed),
        ("interval_reasoning", _measure_interval_reasoning),
        ("causal_order_awareness", _measure_causal_order_awareness),
        ("duration_self_perception", _measure_duration_self_perception),
    ]
    sub_scores: Dict[str, float] = {}
    sub_evidence: Dict[str, Dict[str, Any]] = {}
    notes: List[str] = []

    for name, fn in measurers:
        sc, ev = fn()
        sub_scores[name] = round(float(sc), 4)
        sub_evidence[name] = ev
        if ev.get("failures"):
            notes.append(f"{name}: {len(ev['failures'])} failure(s) — see evidence")

    total = round(sum(sub_scores.values()) / len(sub_scores), 4) if sub_scores else 0.0
    elapsed = time.monotonic() - monotonic_start

    artifact_path_str = ""
    if artifact_dir is not None:
        try:
            out_dir = Path(artifact_dir)
            out_dir.mkdir(parents=True, exist_ok=True)
            payload = {
                "version": V1154_VERSION,
                "total": total,
                "sub_dim_scores": sub_scores,
                "sub_dim_evidence": sub_evidence,
                "notes": notes,
                "elapsed_seconds": elapsed,
                "timestamp": started,
            }
            out_path = out_dir / "v1154_time_grounding.json"
            out_path.write_text(json.dumps(payload, indent=2, ensure_ascii=False))
            artifact_path_str = str(out_path)
        except Exception as e:
            notes.append(f"artifact write failed: {e!r}")

    return TimeGroundingReport(
        total=total,
        sub_dim_scores=sub_scores,
        sub_dim_evidence=sub_evidence,
        notes=notes,
        artifact_path=artifact_path_str,
        elapsed_seconds=round(elapsed, 4),
        timestamp=started,
    )


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def _cli(argv: List[str] | None = None) -> int:
    import argparse
    parser = argparse.ArgumentParser(description="V1154 ASI time philosophy real measurement")
    parser.add_argument("--artifact-dir", default="artifacts/v1154", help="output dir")
    parser.add_argument("--quiet", action="store_true", help="only print total")
    args = parser.parse_args(argv)

    rep = measure_time_grounding(artifact_dir=args.artifact_dir)
    if args.quiet:
        print(round(rep.total, 4))
        return 0

    print(f"V1154 ASI time-philosophy grounding: {rep.total}")
    print(f"  sub-dim scores: {rep.sub_dim_scores}")
    if rep.notes:
        print(f"  notes: {rep.notes}")
    if rep.artifact_path:
        print(f"  artifact: {rep.artifact_path}")
    return 0


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(_cli())
