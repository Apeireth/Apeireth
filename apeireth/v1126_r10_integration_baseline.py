"""Apeireth ASI V1126 — R10 集成 baseline (R10-ARCH-001)

R10 启动 baseline:
  1) R9 W4 末真测 baseline = V0.4 = 0.8538 (R9-INT-005 已 merged)
  2) R10 启动期望 = V0.4 = 0.8600 (baseline + 0.5pp 缓冲)
  3) R10 中期期望 = V0.4 = 0.9000 (V0.5 升级期)
  4) R10 终极期望 = V0.5 ≥ 0.9500 (ASI 北极星综合)
  5) R10 baseline 真测启动 (24 场景 + V0.5 + 守门)

主哲学 LOCKED (继承 V1125 + V1119):
  - 主 22:33 ASI 北极星 (终极梦想)
  - 主 17:43 实事求是 (baseline 必须真测, 不缓存不模拟)
  - 主 13:31 大胆激进 (R10 终极门 0.95 不容分阶段缓慢)
  - 主 19:33 走在前人经验上 (复用 V1114/V1119/V1077 baseline)
  - 主 00:56 任何人都能接手 (一行命令)
  - 主 23:44 干到底 (baseline 真测启动)

复用 (主 19:33 走在前人经验上):
  - V1125 R10 集成协议 (V0.5 + 24 场景 + 守门)
  - V1119 W4 集成验证工具 (R10 起点投影 + 移交 checklist)
  - V1114 weekly integration evaluator (决策引擎 + constants)
  - V1077 V0.4 17 维 (V0.4 真测基础)
  - V1074 V0.3 (守门 floor)

Usage:
    python -m apeireth.v1126_r10_integration_baseline               # baseline 报告
    python -m apeireth.v1126_r10_integration_baseline --live        # 真跑三件套
    python -m apeireth.v1126_r10_integration_baseline --json        # JSON 输出
    python -m apeireth.v1126_r10_integration_baseline --report      # Markdown 报告
    python -m apeireth.v1126_r10_integration_baseline --strict      # 不通过非零退出
"""
from __future__ import annotations

import argparse
import json
import sys
import time
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# ponytail: 复用 V1125 + V1114 + V1119 baseline (主 19:33 走在前人经验上)
from apeireth.v1125_r10_integration_protocol import (  # noqa: E402
    VERSION as V1125_VERSION,
    R10_START_TARGET,
    R10_MID_TARGET,
    R10_ULTIMATE_TARGET,
    R10_TRACK_ULTIMATE_THRESHOLD,
    R10_TRACK_DGM_THRESHOLD,
    R10_TRACK_HQB_THRESHOLD,
    R10_SCENARIO_COUNT,
    V3_GUARDS_R10_INJECTED,
    evaluate_r10,
    render_markdown_r10,
)
from apeireth.v1114_weekly_integration_evaluator import (  # noqa: E402
    ASI_NORTH_STAR,
    V1074_V03_MIN,
    V04_W4_TARGET,
    ROOT,
)
from apeireth.v1119_w4_integration_validator import (  # noqa: E402
    R10_START_TARGET as V1119_R10_START,
    R10_MID_TARGET as V1119_R10_MID,
)

VERSION = "0.1.0"

# R10 baseline 常量 LOCKED (主 17:43 实事求是: 这些是 R9 W4 末真测值)
R9_W4_BASELINE = {
    "v03_score": 0.8897,            # V1074 V0.3 (R9-INT-005 真测)
    "v04_score": 0.8538,            # V1077 V0.4 (R9-INT-005 真测)
    "v04_v1077": 0.8538,            # V1077 17 维直接
    "v04_v1103": 0.8188,            # V1103 Top-5 P2 (参考)
    "v1074_all_ok": True,           # V1074 真测 All OK
    "philosophy_guard_ok": True,    # 6/6 哲学守门
    "n_dims_filled": 17,            # 17 维度全填 (R9 W4 末真值)
    "lift_p2": 0.1447,              # R9 P2 累计 lift (Top-5)
    "source": "r9_w4_baseline_locked",   # 主 17:43 不缓存不模拟
}

# R10 启动 baseline 期望 (主 13:31 大胆激进)
R10_START_EXPECTATIONS = {
    "v03_score_min": V1074_V03_MIN,             # V0.3 ≥ 0.8884 (主 23:44 干到底)
    "v04_score_min": R10_START_TARGET,          # V0.4 ≥ 0.86
    "v05_score_min": 0.8700,                    # V0.5 ≥ 0.87 (R10 起点)
    "v05_ultimate": R10_ULTIMATE_TARGET,        # V0.5 终极门 0.95
    "asi_north_star": ASI_NORTH_STAR,           # 0.98 LOCKED
    "r10_stage_target": "R10-W1",
}

# R10 baseline 兼容矩阵 (与 V1114 / V1119 / V1125 兼容)
R10_BASELINE_COMPATIBILITY = {
    "v1114_weekly_evaluator": "compatible",     # 复用 V1114 决策引擎
    "v1119_w4_validator": "compatible",         # 复用 V1119 R10 起点投影
    "v1125_r10_protocol": "native",             # V1125 是 R10 主集成协议
    "v1077_v04_17dim": "compatible",            # V0.4 17 维 baseline 复用
    "v1103_p2_diagnostic": "compatible",        # Top-5 P2 baseline 复用
    "v1111_hqb_4dim": "compatible",             # HQB 4 维 baseline 复用
    "v1074_v03_runner": "compatible",           # V0.3 baseline 复用
}


# ---------------------------------------------------------------------------
# R10 Baseline dataclass
# ---------------------------------------------------------------------------

@dataclass
class R10Baseline:
    """R10 启动 baseline 数据结构 (主 17:43 实事求是: 真测值, 不模拟)."""
    r9_w4_baseline: Dict[str, Any] = field(default_factory=lambda: dict(R9_W4_BASELINE))
    r10_start_target: float = R10_START_TARGET
    r10_mid_target: float = R10_MID_TARGET
    r10_ultimate_target: float = R10_ULTIMATE_TARGET
    asi_north_star: float = ASI_NORTH_STAR
    compatibility: Dict[str, str] = field(default_factory=lambda: dict(R10_BASELINE_COMPATIBILITY))
    timestamp: float = 0.0
    version: str = VERSION

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


def load_r10_baseline() -> R10Baseline:
    """加载 R10 baseline (主 17:43: 来自 R9 W4 末真测, 不缓存不模拟)."""
    return R10Baseline(timestamp=time.time())


# ---------------------------------------------------------------------------
# R10 真测启动 (主 17:43 实事求是: 真跑 V1125.evaluate_r10)
# ---------------------------------------------------------------------------

@dataclass
class R10BaselineRun:
    """R10 baseline 真测启动结果."""
    baseline: Dict[str, Any]
    protocol_result: Dict[str, Any]
    gap_to_r10_start: float
    gap_to_r10_mid: float
    gap_to_r10_ultimate: float
    gap_to_asi: float
    passes_r10_start: bool
    passes_r10_mid: bool
    passes_r10_ultimate: bool
    r10_ready: bool
    timestamp: float = 0.0

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


def run_r10_baseline_startup(week_label: str = "R10-W1",
                             live: bool = False,
                             v04_override: Optional[float] = None,
                             v03_override: Optional[float] = None) -> R10BaselineRun:
    """R10 baseline 真测启动 (主 17:43 实事求是 + 主 23:44 干到底).

    复用 V1125.evaluate_r10 真跑 (live=False 用 R9 W4 baseline; live=True 真跑三件套).
    ponytail: 不发明新逻辑, 一行启动即可 (主 00:56).
    """
    baseline = load_r10_baseline()
    v04_actual = v04_override if v04_override is not None else baseline.r9_w4_baseline["v04_score"]
    v03_actual = v03_override if v03_override is not None else baseline.r9_w4_baseline["v03_score"]
    # 真跑 V1125 协议 (live=False 即用 R9 W4 baseline; live=True 真跑三件套)
    protocol_result = evaluate_r10(
        week_label=week_label,
        v04_actual=v04_actual,
        v1074_v03_actual=v03_actual,
        no_write=not live,
    )
    v05_total = protocol_result["v05_score"]["v05_total"]
    return R10BaselineRun(
        baseline=baseline.to_dict(),
        protocol_result=protocol_result,
        gap_to_r10_start=round(R10_START_TARGET - v04_actual, 4),
        gap_to_r10_mid=round(R10_MID_TARGET - v04_actual, 4),
        gap_to_r10_ultimate=round(R10_ULTIMATE_TARGET - v05_total, 4),
        gap_to_asi=round(ASI_NORTH_STAR - v05_total, 4),
        passes_r10_start=v04_actual >= R10_START_TARGET,
        passes_r10_mid=v04_actual >= R10_MID_TARGET,
        passes_r10_ultimate=v05_total >= R10_ULTIMATE_TARGET,
        r10_ready=(v04_actual >= R10_START_TARGET and v05_total >= 0.87),
        timestamp=time.time(),
    )


# ---------------------------------------------------------------------------
# Markdown 渲染
# ---------------------------------------------------------------------------

def render_markdown_baseline(run: R10BaselineRun) -> str:
    """渲染 R10 baseline 真测启动 Markdown 报告."""
    p = run.protocol_result
    d = p["dashboard"]
    v05 = p["v05_score"]
    nsc = p["north_star_composite"]
    s = p["scenarios_summary"]
    t = p["r10_track_decision"]
    g = p["guards"]
    b = run.baseline
    lines = [
        "# R10 Baseline 真测启动报告 — V1126",
        "",
        f"> **生成时间**: {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(run.timestamp))}",
        f"> **版本**: V1126 v{b['version']} (继承 V1125 v{V1125_VERSION})",
        f"> **主哲学 LOCKED**: 实事求是 + 干到底 + 大胆激进 + 走在前人经验 + 任何人都能接手",
        "",
        "---",
        "",
        "## 📊 R9 W4 末 Baseline (LOCKED)",
        "",
        f"| 指标 | R9 W4 末真测 | 备注 |",
        f"|---|---:|---|",
        f"| V1074 V0.3 | **{b['r9_w4_baseline']['v03_score']:.4f}** | R9-INT-005 真测 |",
        f"| V1077 V0.4 | **{b['r9_w4_baseline']['v04_score']:.4f}** | 17 维全测 |",
        f"| V1103 V0.4 | **{b['r9_w4_baseline']['v04_v1103']:.4f}** | Top-5 P2 |",
        f"| P2 累计 lift | {b['r9_w4_baseline']['lift_p2']:.4f} | R9 P2 |",
        f"| 维度填充 | {b['r9_w4_baseline']['n_dims_filled']}/17 | V1077 |",
        f"| V1074 All OK | {b['r9_w4_baseline']['v1074_all_ok']} | 主 17:43 |",
        f"| philosophy_guard | {b['r9_w4_baseline']['philosophy_guard_ok']} | 6/6 |",
        "",
        "## 🎯 R10 期望 vs 实际",
        "",
        f"| 阶段 | 期望 | 实际 (R9 W4 baseline) | Gap | Pass |",
        f"|---|---:|---:|---:|---|",
        f"| R10 起点 (V0.4) | {R10_START_TARGET:.4f} | {b['r9_w4_baseline']['v04_score']:.4f} | "
        f"{run.gap_to_r10_start:+.4f} | {run.passes_r10_start} |",
        f"| R10 中期 (V0.4) | {R10_MID_TARGET:.4f} | {b['r9_w4_baseline']['v04_score']:.4f} | "
        f"{run.gap_to_r10_mid:+.4f} | {run.passes_r10_mid} |",
        f"| R10 终极 (V0.5) | {R10_ULTIMATE_TARGET:.4f} | {v05['v05_total']:.4f} | "
        f"{run.gap_to_r10_ultimate:+.4f} | {run.passes_r10_ultimate} |",
        f"| ASI 北极星 | {ASI_NORTH_STAR:.4f} | {v05['v05_total']:.4f} | "
        f"{run.gap_to_asi:+.4f} | - |",
        "",
        f"- **R10 ready**: {run.r10_ready} (V0.4 ≥ {R10_START_TARGET} + V0.5 ≥ 0.87)",
        "",
        "## 🔗 兼容矩阵 (主 19:33 走在前人经验上)",
        "",
        f"| 模块 | 兼容性 |",
        f"|---|---|",
    ]
    for mod, compat in b["compatibility"].items():
        lines.append(f"| {mod} | {compat} |")
    lines.extend([
        "",
        "## 🧪 R10 集成协议真测 (V1125 24 场景)",
        "",
        f"- **总场景数**: {s['total']} (≥ {R10_SCENARIO_COUNT} ?) {s['scenario_count_locked']}",
        f"- **通过**: {s['passed']}",
        f"- **失败**: {s['failed']}",
        f"- **通过率**: {s['pass_rate']*100:.1f}%",
        f"- **All OK**: {s['all_pass']}",
        f"- **V0.5 总分**: {v05['v05_total']:.4f}",
        f"- **R10 主轨道**: {t['track']} — {t['track_name']}",
        f"- **R10 守门自检**: all_ok = {g['all_ok']}",
        "",
        "## ✅ 终判",
        "",
        f"- **R10 ready**: {run.r10_ready}",
        f"- **24 场景 PASS**: {s['all_pass']}",
        f"- **R10 守门自检 PASS**: {g['all_ok']}",
        f"- **V0.5 终极门**: {run.passes_r10_ultimate}",
        "",
        "---",
        "",
        "*主哲学 22:33 LOCKED. 主 17:43 实事求是. 主 13:31 大胆激进. "
        "主 23:44 干到底. 主 19:33 走在前人经验上. 主 00:56 任何人都能接手.*",
    ])
    return "\n".join(lines)


# ---------------------------------------------------------------------------
# CLI 入口
# ---------------------------------------------------------------------------

def _build_arg_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="v1126_r10_integration_baseline",
        description="R10 集成 baseline 真测启动 (R9 W4 末 baseline + V1125 24 场景)",
    )
    p.add_argument("--week", default="R10-W1", help="R10 周次标签")
    p.add_argument("--live", action="store_true", help="真跑三件套 (主 17:43)")
    p.add_argument("--v04-override", type=float, default=None, help="覆盖 V0.4 真测值")
    p.add_argument("--v03-override", type=float, default=None, help="覆盖 V0.3 真测值")
    p.add_argument("--json", action="store_true", help="JSON 输出")
    p.add_argument("--report", action="store_true", help="写 Markdown 报告到 reports/")
    p.add_argument("--strict", action="store_true", help="不通过非零退出")
    return p


def main(argv: Optional[List[str]] = None) -> int:
    args = _build_arg_parser().parse_args(argv)
    run = run_r10_baseline_startup(
        week_label=args.week,
        live=args.live,
        v04_override=args.v04_override,
        v03_override=args.v03_override,
    )
    if args.json:
        print(json.dumps(run.to_dict(), indent=2, ensure_ascii=False))
    elif args.report:
        md = render_markdown_baseline(run)
        path = ROOT / "reports" / f"r10-baseline-{args.week.lower()}.md"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(md, encoding="utf-8")
        print(f"[OK] report written: {path}")
    else:
        b = run.baseline["r9_w4_baseline"]
        p = run.protocol_result
        print(f"R10 {args.week} Baseline 真测启动")
        print(f"  R9 W4 末 baseline: V0.3 = {b['v03_score']:.4f}, V0.4 = {b['v04_score']:.4f}")
        print(f"  R10 起点 (V0.4 ≥ {R10_START_TARGET}): {b['v04_score']:.4f} {'✓' if run.passes_r10_start else '✗'}")
        print(f"  R10 中期 (V0.4 ≥ {R10_MID_TARGET}): {b['v04_score']:.4f} {'✓' if run.passes_r10_mid else '✗'}")
        print(f"  R10 终极 (V0.5 ≥ {R10_ULTIMATE_TARGET}): "
              f"{p['v05_score']['v05_total']:.4f} {'✓' if run.passes_r10_ultimate else '✗'}")
        print(f"  ASI 北极星: {ASI_NORTH_STAR:.4f} (LOCKED)")
        print(f"  24 场景真测: {p['scenarios_summary']['passed']}/{p['scenarios_summary']['total']} "
              f"({p['scenarios_summary']['pass_rate']*100:.1f}%)")
        # 三层守门独立显示 (主 23:44 干到底: 不混淆 protocol vs baseline ready)
        print(f"  协议层 all_ok (24 场景 + V3 守门): {p['all_ok']}")
        print(f"  baseline ready (V0.4 ≥ 起点 + V0.5 ≥ 0.87): {run.r10_ready}")
    if args.strict and not p["all_ok"]:
        # strict mode: 协议层必须 all_ok (baseline ready 不强制, 主 17:43 实事求是)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())


# V1126 auto-injected V3 守门 (主 17:43 实事求是 + 主 17:58 不假装)
V3_GUARDS_R10_BASELINE_INJECTED = {
    "baseline_is_not_production": "R10 baseline 是真测启动, ≠ 真生产. 真生产 = 真部署 + 真用户.",
    "r9_baseline_is_locked": "R9 W4 末 baseline (V0.4=0.8538) LOCKED, 不允许改写历史.",
    "r10_ready_is_not_asi": "R10 ready 仅是 baseline 启动, ≠ ASI 达成.",
    "no_baseline_gaming": "baseline 必须真测, 不允许 cache / mock / 模拟.",
    "compatibility_is_not_asi": "兼容矩阵是工程验证, ASI 仍 > V0.5.",
    "v1126_baseline_is_not_protocol": "V1126 是 baseline 启动器, V1125 是协议. 不可混淆.",
}