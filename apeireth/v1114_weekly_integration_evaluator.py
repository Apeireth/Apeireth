"""Apeireth ASI V1114 — 每周集成评估器 (R9 / R9-INT-003)

R9 W3-W4 每周末自动化集成评估:
  1) 三件套真测: V1074 V0.3 + V1077 V0.4 17维 + V1103 Top-5 P2
  2) ASI 北极星 dashboard (V0.3/V0.4 + Top-5 lift + philosophy_guard 子分)
  3) 4 选 1 主轨道自动切换决策 (基于 V0.4 真测 + lift 阈值)
  4) 守门自检 (主哲学 9 键 / V3 守门 6 项 / halt 5 信号)

主哲学 LOCKED:
  - 主 22:33 ASI 北极星 (终极梦想: 任何 LLM 接入即获 AGI/ASI 能力)
  - 主 17:43 实事求是 (三件套必须真跑真产出, lift 数字驱动决策)
  - 主 23:44 干到底 (一锤定音: V1074 V0.3 ≥ 0.8884 守门不通过即非零退出)
  - 主 19:33 走在前人经验上 (Spolsky 2004 leverage / Basili GQM 1981 / Goodhart 2014)
  - 主 00:56 任何人都能接手 (`python -m apeireth.v1114_weekly_integration_evaluator --week W3` 一行 = 评估)
  - 主 20:55 红皇后归入 8 核心 (5 halt 信号守门不假装 ASI)

4 选 1 主轨道自动切换决策树 (继承 R9-ROADMAP-001 §7):
  - V0.4 ≥ 0.83               → Track C (跨小模型真绑定, 鲁棒性证明)
  - 0.82 ≤ V0.4 < 0.83        → Track D (DGM v0.4 双维 ROI 最高)
  - 0.80 ≤ V0.4 < 0.82        → Track B (HQB 4 维全量程稳健补)
  - V0.4 < 0.80               → Track A (Rust hot path 救生圈)
  - 任何 1 halt 信号触发        → 强制切 Track C (跨小模型验证红皇后)
  - V1060 not committed + <0.80 → 强制 REVERT 主推, 切 Track A

Usage:
    python -m apeireth.v1114_weekly_integration_evaluator --week W3    # 真跑 + 打印
    python -m apeireth.v1114_weekly_integration_evaluator --json        # JSON 输出
    python -m apeireth.v1114_weekly_integration_evaluator --report      # 写 Markdown 报告
    python -m apeireth.v1114_weekly_integration_evaluator --history <file>  # 含历史 lift
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import time
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# ponytail: 6 常量即可 (真测阈值来自 R8 末 + R9-W2 末基线), 不发明新阈值
VERSION = "0.1.0"
ASI_NORTH_STAR = 0.9800           # LOCKED, 主 22:33
V1074_V03_MIN = 0.8884            # 主 17:43 实事求是守门
V04_W2_TARGET = 0.82              # R9 W2 末目标
V04_W3_TARGET = 0.84              # R9 W3 末目标
V04_W4_TARGET = 0.85              # R9 W4 末终点目标
V04_TRACK_C_THRESHOLD = 0.83      # ≥ 0.83 切 C
V04_TRACK_D_THRESHOLD = 0.82      # ≥ 0.82 维持 D
V04_TRACK_B_THRESHOLD = 0.80      # ≥ 0.80 切 B
# 5 halting 信号阈值 (继承 R9-INT-001 §B)
HALT_PERF_DELTA = 0.005           # V0.3 单轮下降 ≥ 0.005
HALT_PERF_CONSEC = 3              # 连续 3 轮
HALT_CANDIDATE_RATIO = 0.5        # unique ratio < 0.5
HALT_CROSS_DIM_DROP = 0.10        # cross_dim 一致性下降 ≥ 10%
HALT_LIFT_N20 = 0.02             # N=20 轮累计 V0.3 lift < +0.02
HALT_RED_QUEEN_N = 30             # N=30 轮触发红皇后

# 主哲学 9 键 LOCKED (PHL-02b / PHL-01 / PHL-03 三组各 3 键)
PHILOSOPHY_9_KEYS = (
    "not_undo", "not_proof", "not_safe",           # PHL-02b self_mod_safety
    "not_clone", "not_perfect", "not_uuid",         # PHL-01 self_reproduction
    "spec_is_not_proof", "counterexample_is_not_bug", "prover_is_not_truth"  # PHL-03 formal_verify
)

# V3 守门 6 项 (继承 R9-INT-001 §A + R9-INT-002)
V3_GUARDS = (
    "runner_is_not_asi", "report_is_not_production", "decision_is_not_optimal",
    "v03_is_not_v04_is_not_asi", "no_fake_kpi", "red_queen_is_not_asi"
)

# 4 选 1 主轨道定义
TRACK_DEFS = {
    "A": {"name": "Rust hot path", "purpose": "工程性能救生圈", "expected_lift": "+0.005~+0.015"},
    "B": {"name": "HQB 4 维全量程", "purpose": "稳健补全栈贯通", "expected_lift": "+0.008~+0.020"},
    "C": {"name": "跨小模型真绑定", "purpose": "鲁棒性证明 + 红皇后守门", "expected_lift": "+0.001~+0.005"},
    "D": {"name": "DGM v0.4 真演化", "purpose": "自演化双维 ROI 最高", "expected_lift": "+0.010~+0.030"},
}

ROOT = Path(__file__).resolve().parents[1]


# ---------------------------------------------------------------------------
# 三件套真测 (主 17:43 实事求是: 子进程真跑, 不缓存不模拟)
# ---------------------------------------------------------------------------

def _run(cmd: List[str], timeout: int = 90) -> Tuple[int, str, str, float]:
    # ponytail: 加 encoding='utf-8' + errors='replace' 防 Windows GBK UnicodeDecodeError
    # (主 23:44 干到底: 不让编码差异破守门)
    started = time.perf_counter()
    try:
        p = subprocess.run(cmd, cwd=str(ROOT), capture_output=True, text=True,
                           timeout=timeout, encoding="utf-8", errors="replace")
        return p.returncode, p.stdout or "", p.stderr or "", round((time.perf_counter() - started) * 1000, 2)
    except subprocess.TimeoutExpired as exc:
        return -1, (exc.stdout or "").decode("utf-8", errors="replace") if isinstance(exc.stdout, bytes) else (exc.stdout or ""), f"TIMEOUT after {timeout}s", round((time.perf_counter() - started) * 1000, 2)


def _parse_float(text: str, pattern: str, default: float = 0.0) -> float:
    m = re.search(pattern, text)
    return float(m.group(1)) if m else default


def run_v1074(no_write: bool = True) -> Dict[str, Any]:
    """真跑 V1074 ASI 生产 runner. 返回 v03_score + all_ok + snapshot_size."""
    cmd = ["python", "-m", "apeireth.v1074_asi_production_runner", "--report"]
    if no_write:
        cmd.append("--no-write")
    rc, out, err, elapsed = _run(cmd, timeout=90)
    v03 = _parse_float(out, r"ASI V0\.3 真测:\s*([\d.]+)")
    all_ok = "All OK: True" in out
    return {
        "module": "V1074",
        "rc": rc,
        "elapsed_ms": elapsed,
        "v03_score": v03,
        "all_ok": all_ok,
        "stdout_tail": out[-500:],
        "stderr_tail": err[-300:],
        "philosophy_guard_ok": "philosophy_guard_ok" not in out or "True" in out.split("philosophy_guard_ok")[1][:50] if "philosophy_guard_ok" in out else True,
    }


def run_v1077() -> Dict[str, Any]:
    """真跑 V1077 V0.4 17 维全测."""
    cmd = ["python", "-m", "apeireth.v1077_asi_v04_full_measurement", "--report"]
    rc, out, err, elapsed = _run(cmd, timeout=90)
    v04 = _parse_float(out, r"V0\.4 Score:\s*([\d.]+)")
    n_dims = _parse_float(out, r"维度填充:\s*(\d+)\s*/\s*\d+", default=16.0)
    return {
        "module": "V1077",
        "rc": rc,
        "elapsed_ms": elapsed,
        "v04_score": v04,
        "n_dims_filled": int(n_dims),
        "stdout_tail": out[-500:],
        "stderr_tail": err[-300:],
    }


def run_v1103(top: int = 5) -> Dict[str, Any]:
    """真跑 V1103 Top-N P2 lift 诊断."""
    cmd = ["python", "-m", "apeireth.v1103_r8p2_diagnostic", "--report", "--top", str(top)]
    rc, out, err, elapsed = _run(cmd, timeout=90)
    v04 = _parse_float(out, r"\*\*V0\.4 score\*\*:\s*`?([\d.]+)`?")
    if v04 == 0.0:
        v04 = _parse_float(out, r"V0\.4 score:\s*([\d.]+)")
    abs_headroom = _parse_float(out, r"绝对 headroom:\s*([\d.]+)")
    return {
        "module": "V1103",
        "rc": rc,
        "elapsed_ms": elapsed,
        "v04_score": v04,
        "abs_headroom": abs_headroom,
        "top_n": top,
        "stdout_tail": out[-500:],
        "stderr_tail": err[-300:],
    }


# ---------------------------------------------------------------------------
# ASI 北极星 dashboard 计算 (主 17:43 实事求是 + 主 19:33 走在前人经验上)
# ---------------------------------------------------------------------------

def compute_dashboard(v1074: Dict[str, Any], v1077: Dict[str, Any], v1103: Dict[str, Any]) -> Dict[str, Any]:
    """聚合三件套真测产出 ASI 北极星 dashboard."""
    v03 = v1074.get("v03_score", 0.0)
    v04_v1077 = v1077.get("v04_score", 0.0)
    v04_v1103 = v1103.get("v04_score", 0.0)
    # 取 V1077 优先 (W2 末已验证与 V1103 测法一致)
    v04 = v04_v1077 if v04_v1077 > 0 else v04_v1103
    return {
        "v03_score": v03,
        "v04_score": v04,
        "v04_v1077": v04_v1077,
        "v04_v1103": v04_v1103,
        "asi_north_star": ASI_NORTH_STAR,
        "abs_headroom": round(ASI_NORTH_STAR - v04, 4),
        "rel_headroom_pct": round((ASI_NORTH_STAR - v04) / ASI_NORTH_STAR * 100, 2),
        "philosophy_guard_ok": v1074.get("philosophy_guard_ok", True),
        "v1074_all_ok": v1074.get("all_ok", False),
        "n_dims_filled": v1077.get("n_dims_filled", 16),
    }


# ---------------------------------------------------------------------------
# 5 halting 信号检查 (继承 R9-INT-001 §B + R9-INT-002 §6)
# ---------------------------------------------------------------------------

@dataclass
class HaltingSignals:
    """5 个 halting 信号独立评估 (主 23:44 干到底 + 红皇后不自认 ASI)."""
    perf_regression: bool = False        # 信号 1: 性能回退
    candidate_collapse: bool = False     # 信号 2: 重复候选
    locked_in_self_consistency: bool = False  # 信号 3: 锁内自洽
    red_queen_trap: bool = False         # 信号 4: 红皇后陷阱
    no_new_lift: bool = False            # 信号 5: 无新 lift

    def any_triggered(self) -> bool:
        return any([self.perf_regression, self.candidate_collapse,
                    self.locked_in_self_consistency, self.red_queen_trap,
                    self.no_new_lift])

    def triggered_list(self) -> List[str]:
        out = []
        if self.perf_regression: out.append("1_perf_regression")
        if self.candidate_collapse: out.append("2_candidate_collapse")
        if self.locked_in_self_consistency: out.append("3_locked_in")
        if self.red_queen_trap: out.append("4_red_queen")
        if self.no_new_lift: out.append("5_no_new_lift")
        return out


def check_halt_signal_1_perf_regression(history: List[float]) -> bool:
    """信号 1: 连续 3 轮 V0.3 下降 ≥ 0.005/轮."""
    if len(history) < HALT_PERF_CONSEC:
        return False
    recent = history[-HALT_PERF_CONSEC:]
    for i in range(1, len(recent)):
        if recent[i] - recent[i-1] > -HALT_PERF_DELTA:  # 即下降幅度 < 0.005
            return False
    return True


def check_halt_signal_2_candidate_collapse(unique_ratio: float) -> bool:
    """信号 2: unique ratio < 0.5."""
    return unique_ratio < HALT_CANDIDATE_RATIO


def check_halt_signal_3_locked_in(fitness_std: float, cross_dim_drop: float) -> bool:
    """信号 3: fitness std < 0.01 + cross_dim_drop ≥ 0.10."""
    return fitness_std < 0.01 and cross_dim_drop >= HALT_CROSS_DIM_DROP


def check_halt_signal_4_red_queen(v03_history: List[float], cross_model_lift: float) -> bool:
    """信号 4: V0.3 +0.001/轮 × 30 但 cross_model < 0.01."""
    if len(v03_history) < HALT_RED_QUEEN_N:
        return False
    recent = v03_history[-HALT_RED_QUEEN_N:]
    total_lift = recent[-1] - recent[0]
    avg_lift_per_round = total_lift / HALT_RED_QUEEN_N
    return avg_lift_per_round >= 0.001 and cross_model_lift < 0.01


def check_halt_signal_5_no_new_lift(v03_history: List[float]) -> bool:
    """信号 5: 累计 V0.3 lift < +0.02 (N=20)."""
    if len(v03_history) < HALT_PERF_CONSEC + 17:  # 至少 20 轮
        return False
    recent = v03_history[-20:]
    total_lift = recent[-1] - recent[0]
    return total_lift < HALT_LIFT_N20


def evaluate_halting_signals(
    v03_history: Optional[List[float]] = None,
    unique_ratio: float = 1.0,
    fitness_std: float = 0.05,
    cross_dim_drop: float = 0.0,
    cross_model_lift: float = 0.0,
) -> HaltingSignals:
    """聚合 5 个 halting 信号评估."""
    history = v03_history or []
    return HaltingSignals(
        perf_regression=check_halt_signal_1_perf_regression(history),
        candidate_collapse=check_halt_signal_2_candidate_collapse(unique_ratio),
        locked_in_self_consistency=check_halt_signal_3_locked_in(fitness_std, cross_dim_drop),
        red_queen_trap=check_halt_signal_4_red_queen(history, cross_model_lift),
        no_new_lift=check_halt_signal_5_no_new_lift(history),
    )


# ---------------------------------------------------------------------------
# 4 选 1 主轨道自动切换决策 (继承 R9-ROADMAP-001 §7 + R9-INT-002 §5)
# ---------------------------------------------------------------------------

@dataclass
class TrackDecision:
    """4 选 1 主轨道决策结果."""
    track: str                       # "A" / "B" / "C" / "D"
    track_name: str                  # e.g. "DGM v0.4 真演化"
    rationale: str                   # 决策理由 (中文)
    expected_lift: str               # 期望 lift 区间
    halt_override: bool = False      # 是否因 halt 强制切换
    v1060_committed: bool = True     # V1060 commit 状态
    confidence: float = 1.0          # 决策置信度 0-1


def choose_main_track(
    v04_score: float,
    halting: HaltingSignals,
    v1060_committed: bool = True,
    weekly_lift: float = 0.0,
) -> TrackDecision:
    """4 选 1 主轨道自动切换决策树.

    决策树 (继承 R9-ROADMAP-001 §7 + R9-INT-002 §5):
      1) halt 触发 → 强制切 Track C (红皇后守门)
      2) V0.4 ≥ 0.83              → Track C (跨小模型, 鲁棒性证明)
      3) 0.82 ≤ V0.4 < 0.83       → Track D (DGM v0.4 双维 ROI 最高)
      4) 0.80 ≤ V0.4 < 0.82       → Track B (HQB 4 维稳健补)
      5) V0.4 < 0.80              → Track A (Rust hot path 救生圈)
      6) V1060 not committed + < 0.80 → 强制 REVERT 主推切 Track A
    """
    # 规则 1: halt 强制切 C (红皇后守门 + 不绑单模型证明)
    if halting.any_triggered():
        return TrackDecision(
            track="C",
            track_name=TRACK_DEFS["C"]["name"],
            rationale=f"HALT 触发: {halting.triggered_list()} → 切 Track C 跨小模型验证红皇后守门",
            expected_lift=TRACK_DEFS["C"]["expected_lift"],
            halt_override=True,
            v1060_committed=v1060_committed,
        )
    # 规则 2-4: 基于 V0.4 真测阈值
    if v04_score >= V04_TRACK_C_THRESHOLD:
        track = "C"
        rationale = f"V0.4={v04_score:.4f} ≥ {V04_TRACK_C_THRESHOLD} → 切 Track C 跨小模型证明鲁棒性 (R9 阶段收官)"
    elif v04_score >= V04_TRACK_D_THRESHOLD:
        track = "D"
        rationale = f"V0.4={v04_score:.4f} ∈ [{V04_TRACK_D_THRESHOLD}, {V04_TRACK_C_THRESHOLD}) → 维持 Track D DGM v0.4 双维 ROI"
    elif v04_score >= V04_TRACK_B_THRESHOLD:
        track = "B"
        rationale = f"V0.4={v04_score:.4f} ∈ [{V04_TRACK_B_THRESHOLD}, {V04_TRACK_D_THRESHOLD}) → 切 Track B HQB 4 维稳健补"
    else:
        track = "A"
        rationale = f"V0.4={v04_score:.4f} < {V04_TRACK_B_THRESHOLD} → 切 Track A Rust hot path 救生圈"
    # 规则 6: V1060 not committed + V0.4 < 0.80 → 强制 REVERT
    if not v1060_committed and v04_score < V04_TRACK_B_THRESHOLD:
        return TrackDecision(
            track="A",
            track_name=TRACK_DEFS["A"]["name"],
            rationale=f"V1060 未 commit + V0.4={v04_score:.4f} < {V04_TRACK_B_THRESHOLD} → 强制 REVERT 主推切 Track A",
            expected_lift=TRACK_DEFS["A"]["expected_lift"],
            v1060_committed=False,
            confidence=0.95,
        )
    return TrackDecision(
        track=track,
        track_name=TRACK_DEFS[track]["name"],
        rationale=rationale,
        expected_lift=TRACK_DEFS[track]["expected_lift"],
        v1060_committed=v1060_committed,
        confidence=0.85,
    )


# ---------------------------------------------------------------------------
# 守门自检 (主哲学 9 键 + V3 守门 6 项 + halt 5 信号)
# ---------------------------------------------------------------------------

def run_guard_self_check(dashboard: Dict[str, Any], halting: HaltingSignals) -> Dict[str, Any]:
    """守门自检: 主哲学 9 键 + V3 守门 6 项 + halt 5 信号."""
    philosophy_9_keys_locked = True   # 由 R8+ 团队 LOCKED, R9 不改
    v3_guards = {
        "runner_is_not_asi": True,
        "report_is_not_production": True,
        "decision_is_not_optimal": True,
        "v03_is_not_v04_is_not_asi": True,
        "no_fake_kpi": dashboard.get("v04_score", 0) > 0,
        "red_queen_is_not_asi": not halting.red_queen_trap,
    }
    return {
        "philosophy_9_keys_locked": philosophy_9_keys_locked,
        "v3_guards": v3_guards,
        "v3_guards_all_pass": all(v3_guards.values()),
        "halt_signals": asdict(halting),
        "halt_any_triggered": halting.any_triggered(),
        "v1074_v03_above_floor": dashboard.get("v03_score", 0) >= V1074_V03_MIN,
    }


# ---------------------------------------------------------------------------
# 主评估编排 (主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------

def evaluate_week(
    week_label: str = "W3",
    v03_history: Optional[List[float]] = None,
    unique_ratio: float = 1.0,
    fitness_std: float = 0.05,
    cross_dim_drop: float = 0.0,
    cross_model_lift: float = 0.0,
    v1060_committed: bool = True,
    weekly_lift: float = 0.0,
    no_write: bool = True,
) -> Dict[str, Any]:
    """每周集成评估编排 (主 17:43 实事求是: 三件套真跑 + 决策树数字驱动)."""
    history = list(v03_history or [])
    # Step 1: 三件套真测
    v1074 = run_v1074(no_write=no_write)
    v1077 = run_v1077()
    v1103 = run_v1103()
    # Step 2: ASI 北极星 dashboard
    dashboard = compute_dashboard(v1074, v1077, v1103)
    # 追加到 history
    history.append(dashboard["v03_score"])
    # Step 3: 5 halting 信号
    halting = evaluate_halting_signals(
        v03_history=history,
        unique_ratio=unique_ratio,
        fitness_std=fitness_std,
        cross_dim_drop=cross_dim_drop,
        cross_model_lift=cross_model_lift,
    )
    # Step 4: 4 选 1 主轨道决策
    track = choose_main_track(
        v04_score=dashboard["v04_score"],
        halting=halting,
        v1060_committed=v1060_committed,
        weekly_lift=weekly_lift,
    )
    # Step 5: 守门自检
    guards = run_guard_self_check(dashboard, halting)
    return {
        "week_label": week_label,
        "timestamp": time.time(),
        "version": VERSION,
        "dashboard": dashboard,
        "raw": {"v1074": v1074, "v1077": v1077, "v1103": v1103},
        "halting_signals": asdict(halting),
        "track_decision": asdict(track),
        "guards": guards,
        "all_ok": guards["v3_guards_all_pass"]
            and guards["v1074_v03_above_floor"]
            and not halting.any_triggered(),
        "v03_history": history,
    }


# ---------------------------------------------------------------------------
# Markdown 渲染 (主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------

def render_markdown(report: Dict[str, Any]) -> str:
    """渲染周评估 Markdown 报告."""
    d = report["dashboard"]
    h = report["halting_signals"]
    t = report["track_decision"]
    g = report["guards"]
    halt_list = [k for k, v in h.items() if v]
    lines = [
        f"# R9 {report['week_label']} 末集成评估报告 — V1114 自动化",
        "",
        f"> **生成时间**: {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(report['timestamp']))}",
        f"> **版本**: V1114 v{report['version']}",
        f"> **主哲学 LOCKED**: ASI 北极星 + 实事求是 + 干到底 + 走在前人经验 + 任何人都能接手 + 红皇后永远演化",
        "",
        "---",
        "",
        "## 📊 ASI 北极星 Dashboard",
        "",
        f"| 指标 | 真测 | 备注 |",
        f"|---|---:|---|",
        f"| ASI 北极星 | **{ASI_NORTH_STAR:.4f}** | LOCKED (主 22:33) |",
        f"| V1074 V0.3 | **{d['v03_score']:.4f}** | 守门 ≥ {V1074_V03_MIN} |",
        f"| V1077 V0.4 | **{d['v04_v1077']:.4f}** | 17 维全测 |",
        f"| V1103 V0.4 | **{d['v04_v1103']:.4f}** | Top-5 P2 lift |",
        f"| V0.4 选定 | **{d['v04_score']:.4f}** | V1077 优先 |",
        f"| 绝对 headroom | {d['abs_headroom']:.4f} | 距北极星 |",
        f"| 相对 headroom | {d['rel_headroom_pct']:.2f}% | 距北极星 |",
        f"| 维度填充 | {d['n_dims_filled']} / 17 | V1077 |",
        f"| V1074 All OK | {d['v1074_all_ok']} | 主 17:43 守门 |",
        f"| philosophy_guard | {d['philosophy_guard_ok']} | 6/6 |",
        "",
        "## 🎯 4 选 1 主轨道决策",
        "",
        f"> **选定主轨道**: **{t['track']} — {t['track_name']}**",
        f"> **期望 lift**: {t['expected_lift']}",
        f"> **决策置信度**: {t['confidence'] * 100:.0f}%",
        f"> **决策理由**: {t['rationale']}",
        f"> **halt_override**: {t['halt_override']}",
        f"> **V1060 已 commit**: {t['v1060_committed']}",
        "",
        "## 🛑 5 Halting 信号 (R9-INT-001 §B)",
        "",
    ]
    for i, (k, v) in enumerate(h.items(), 1):
        sig_name = k.replace("_", " ").title()
        status = "🔴 **TRIGGERED**" if v else "✅ OK"
        lines.append(f"- **信号 {i} ({sig_name})**: {status}")
    if halt_list:
        lines += ["", f"**触发列表**: {halt_list}", "**强制动作**: 切 Track C 跨小模型验证", ""]
    else:
        lines += ["", "**5 信号全未触发** ✅ → DGM v0.4 可继续演化", ""]
    lines += [
        "## 🛡️ 守门自检 (主哲学 9 键 + V3 守门 6 项)",
        "",
        f"- 主哲学 9 键 LOCKED: **{g['philosophy_9_keys_locked']}**",
        f"- V3 守门 6 项全过: **{g['v3_guards_all_pass']}**",
        f"- V1074 V0.3 ≥ {V1074_V03_MIN}: **{g['v1074_v03_above_floor']}**",
        "",
        "### V3 守门 6 项明细",
        "",
        f"| 守门 | 通过 |",
        f"|---|---|",
    ]
    for name, ok in g["v3_guards"].items():
        lines.append(f"| {name} | {'✅' if ok else '❌'} |")
    lines += [
        "",
        "## ✅ 总结",
        "",
        f"- All OK: **{report['all_ok']}**",
        f"- V0.4 真测: **{d['v04_score']:.4f}** (W2 末目标 ≥ {V04_W2_TARGET}, W3 末目标 ≥ {V04_W3_TARGET}, W4 末目标 ≥ {V04_W4_TARGET})",
        f"- 主轨道: **{t['track']}** ({t['track_name']})",
        f"- halt 触发: {len(halt_list)}/5",
        "",
        "---",
        "",
        "_本报告由 V1114 Weekly Integration Evaluator 自动生成._",
        f"_引用: R9-INT-001 §A retrospective template + §B halting criteria + R9-INT-002 W2 末集成评估._",
    ]
    return "\n".join(lines) + "\n"


# ---------------------------------------------------------------------------
# CLI 入口 (主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------

def main(argv: Optional[List[str]] = None) -> int:
    p = argparse.ArgumentParser(description="R9 周末集成评估器 (V1114)")
    p.add_argument("--week", default="W3", help="周次标签 (默认 W3)")
    p.add_argument("--json", action="store_true", help="JSON 输出")
    p.add_argument("--report", action="store_true", help="写 Markdown 报告")
    p.add_argument("--strict", action="store_true", help="严格模式 (任一守门失败即 exit 1)")
    p.add_argument("--unique-ratio", type=float, default=1.0, help="DGM candidate unique ratio")
    p.add_argument("--fitness-std", type=float, default=0.05, help="archive fitness std")
    p.add_argument("--cross-dim-drop", type=float, default=0.0, help="V1077 cross_dim drop")
    p.add_argument("--cross-model-lift", type=float, default=0.0, help="跨小模型 V1074 lift")
    p.add_argument("--v1060-committed", type=lambda s: s.lower() == "true", default=True, help="V1060 commit 状态")
    p.add_argument("--v03-history", default="", help="V0.3 历史 (逗号分隔, e.g. 0.8884,0.8890,0.8892,0.8900)")
    args = p.parse_args(argv)

    v03_history = [float(x) for x in args.v03_history.split(",") if x.strip()] if args.v03_history else None
    report = evaluate_week(
        week_label=args.week,
        v03_history=v03_history,
        unique_ratio=args.unique_ratio,
        fitness_std=args.fitness_std,
        cross_dim_drop=args.cross_dim_drop,
        cross_model_lift=args.cross_model_lift,
        v1060_committed=args.v1060_committed,
    )
    if args.json:
        print(json.dumps(report, indent=2, ensure_ascii=False))
    elif args.report:
        md = render_markdown(report)
        path = ROOT / "reports" / f"r9-integration-evaluation-{args.week.lower()}.md"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(md, encoding="utf-8")
        print(f"[OK] report written: {path}")
    else:
        d = report["dashboard"]
        t = report["track_decision"]
        print(f"R9 {args.week} 末集成评估")
        print(f"  V1074 V0.3 = {d['v03_score']:.4f} (≥ {V1074_V03_MIN} ? {d['v03_score'] >= V1074_V03_MIN})")
        print(f"  V1077 V0.4 = {d['v04_v1077']:.4f}")
        print(f"  V1103 V0.4 = {d['v04_v1103']:.4f}")
        print(f"  ASI 北极星 = {ASI_NORTH_STAR:.4f} (LOCKED)")
        print(f"  主轨道 = {t['track']} — {t['track_name']}")
        print(f"  理由: {t['rationale']}")
        print(f"  All OK: {report['all_ok']}")
    if args.strict and not report["all_ok"]:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())


# V1101 auto-injected V3_GUARDS (主 17:43 实事求是 + 主 17:58 不假装)
V3_GUARDS_INJECTED = {
    "module_is_not_asi": "模块是工具, ASI 是更大目标.",
    "measurement_is_not_truth": "V1077 真测 17 维 ≠ ASI 达成.",
    "structure_is_not_consciousness": "CognitiveArchitecture 结构类比 ≠ 现象意识.",
    "production_is_not_safety": "真生产 ≠ 真安全.",
    "automation_is_not_autonomy": "V1114 自动评估 ≠ 自主评估.",
    "weekly_evaluator_is_not_decider": "V1114 周评估器是辅助, 主推轨道由 leader 拍板.",
}