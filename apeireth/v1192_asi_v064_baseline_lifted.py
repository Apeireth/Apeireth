"""V1192 — ASI V0.6.4 baseline recompute (V1191 接入, real_llm_benchmark lift).

主 06:15 V1051 + 主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 19:33 走在前人经验上 +
主 13:31 大胆激进 + 主 17:58+20:46 不假装 + 主 23:44 干到底 + 主 00:56 任何人都能接手 +
主 00:44 质量工程化.

为什么 V1192:
  V1182 baseline recompute → ASI V0.6.2 = 0.7425, real_llm_benchmark dim = 0.416
  V1189 = V1182 v0_6_new_dim_collector lift → ASI V0.6.3 = 0.8903
  V1190 = V1133 endpoint fix → real LLM working benchmark (pass_rate 0.6364)
  V1191 = V1166 V0.6.4 lift → real_llm_benchmark = 0.9724 (vs V1166 baseline 0.416)
  V1192 = ASI V0.6.4 baseline recompute:
    - 拿 V1189 = 0.8903 作为 baseline (V1184-V1187 lift 已应用)
    - 拿 V1191 = 0.9724 作为 real_llm_benchmark 新值 (vs V1182 旧值 0.416)
    - lift = (V1191 - V1166_baseline) × V1166 weight 0.05
    - ASI V0.6.4 = V1189 + lift
  Expected lift: +(0.9724 - 0.416) × 0.05 = +0.0278
  Expected ASI V0.6.4: 0.8903 + 0.0278 = 0.9181

V1192 vs V1189:
  V1189: 4 v0_6_new_dim lift (vcp_deep_read/real_run/llm_bridge/multi_agent_dag)
  V1192: + 1 v0_6_real_collector lift (real_llm_benchmark via V1191)
  V1192 ASI = V1189 + (V1191 - V1166_baseline) × 0.05

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43):
  - 不假装 V1192 = ASI 北极星 (V1192 = 0.9181 离北极星 0.98 还差 0.062)
  - 不假装 V1192 = ASI V1.0 (V1192 = V0.6.4 中间版本)
  - 不假装 V1191 = V1182 全量 (V1191 只改 real_llm_benchmark, 其他 22 dim 保留 V1189 值)
  - 不假装 V1190 cached = 真跑 (V1190 是真跑 LLM, 不 cached)
  - 不假装 HTTP 200 = ASI 推理 (HTTP 200 + content match = sample pass; ASI 推理 ≠ benchmark)

主 00:56 任何人都能接手:
  - measure_v1192() → float (0..1) 主入口 (ASI V0.6.4 total)
  - run_v1192_full() → V1192Report dataclass + JSON dump
  - V1192Report JSON 写 artifacts/v1192_asi_v064_baseline_lifted.json
  - 默认从 artifacts/v1189_v1182_integration.json + artifacts/v1191_v06_4_real_llm_benchmark_lift.json 读

主 00:44 质量工程化:
  V1192Report:
    snapshot_id, version, timestamp, elapsed_seconds
    asi_v064 (V0.6.4 ASI total)
    asi_v089 (V1189 baseline = 0.8903)
    delta_asi (V1192 - V1189)
    v1166_old_value (0.416), v1191_new_value (0.9724)
    dim_lifts: dict with real_llm_benchmark lift
    vs_north_star: gap, position_pct
    vs_v1182: V1182 was 0.7425 → V1192 = 0.9181, delta +0.1756

Usage:
    python -m apeireth.v1192_asi_v064_baseline_lifted          # 默认从 artifact 读
    python -m apeireth.v1192_asi_v064_baseline_lifted --json  # JSON stdout
    python -m apeireth.v1192_asi_v064_baseline_lifted --report # Markdown report
    python -m apeireth.v1192_asi_v064_baseline_lifted --measure # 只 print measure_v1192()
"""
from __future__ import annotations

import argparse
import json
import sys
import time
import uuid
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

V1192_VERSION = "0.1.0"
V1192_DIM_VERSION = "0.6.4"

# 历史 baseline (主 17:43 — 写死历史值)
V1166_BASELINE = 0.4160
V1182_BASELINE = 0.7425
V1189_BASELINE = 0.8903
NORTH_STAR = 0.9800
V1166_WEIGHT = 0.05  # real_llm_benchmark dim weight in V1153 23-dim

# 默认 artifact paths
DEFAULT_V1189_ARTIFACT = "artifacts/v1189_v1182_integration.json"
DEFAULT_V1191_ARTIFACT = "artifacts/v1191_v06_4_real_llm_benchmark_lift.json"


# ============================================================================
# Dataclasses
# ============================================================================


@dataclass
class V1192Report:
    """V1192 ASI V0.6.4 baseline recompute (V1191 lift integrated)."""

    snapshot_id: str = field(default_factory=lambda: f"v1192-{uuid.uuid4().hex[:8]}")
    version: str = V1192_VERSION
    dim_version: str = V1192_DIM_VERSION
    timestamp: float = field(default_factory=time.time)
    elapsed_seconds: float = 0.0
    asi_v064: float = 0.0  # ASI V0.6.4 total
    asi_v089: float = 0.0  # V1189 baseline (V0.6.3)
    asi_v082: float = 0.0  # V1182 baseline (V0.6.2)
    delta_asi_v064_vs_v089: float = 0.0  # V1192 - V1189
    delta_asi_v064_vs_v082: float = 0.0  # V1192 - V1182
    v1166_old_value: float = V1166_BASELINE
    v1191_new_value: float = 0.0
    v1191_snapshot_id: str = ""
    v1189_snapshot_id: str = ""
    dim_weight_real_llm_benchmark: float = V1166_WEIGHT
    dim_lift_real_llm_benchmark: float = 0.0  # (v1191 - v1166) × weight
    vs_north_star_gap: float = 0.0
    vs_north_star_position_pct: float = 0.0
    artifact_path: str = ""
    notes: List[str] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "V1192Report":
        new = cls()
        for k, v in data.items():
            if hasattr(new, k):
                setattr(new, k, v)
        return new

    def summary_line(self) -> str:
        return (
            f"V1192 ASI V0.6.4 baseline: asi_v064={self.asi_v064:.4f} | "
            f"vs V1189 baseline {self.asi_v089:.4f}: delta {self.delta_asi_v064_vs_v089:+.4f} | "
            f"vs V1182 baseline {self.asi_v082:.4f}: delta {self.delta_asi_v064_vs_v082:+.4f} | "
            f"vs north_star 0.98: gap {self.vs_north_star_gap:.4f} ({self.vs_north_star_position_pct:.2f}% of north star) | "
            f"real_llm_benchmark lift: {self.v1166_old_value:.4f} → {self.v1191_new_value:.4f} "
            f"(× {self.dim_weight_real_llm_benchmark:.4f} = {self.dim_lift_real_llm_benchmark:+.4f}) | "
            f"snapshot={self.snapshot_id}"
        )


# ============================================================================
# Helpers
# ============================================================================


def _read_artifact(path: str) -> Tuple[bool, Optional[Dict[str, Any]], str]:
    p = Path(path)
    if not p.is_file():
        return False, None, f"artifact_not_found: {path}"
    try:
        data = json.loads(p.read_text(encoding="utf-8"))
        return True, data, "ok"
    except Exception as e:
        return False, None, f"artifact_parse_failed: {e!r}"


# ============================================================================
# Main runner
# ============================================================================


def _run_v1192_full(
    v1189_artifact_path: str = DEFAULT_V1189_ARTIFACT,
    v1191_artifact_path: str = DEFAULT_V1191_ARTIFACT,
    write_artifact: bool = True,
    artifact_dir: str = "artifacts",
    artifact_name: str = "v1192_asi_v064_baseline_lifted.json",
) -> V1192Report:
    t0 = time.time()
    rep = V1192Report()

    # Read V1189 (V0.6.3 baseline)
    ok89, data89, reason89 = _read_artifact(v1189_artifact_path)
    rep.notes.append(f"V1189 artifact ({v1189_artifact_path}): {reason89}")
    if ok89 and data89 is not None:
        rep.asi_v089 = float(data89.get("v1189_asi_lifted", V1189_BASELINE))
        rep.v1189_snapshot_id = str(data89.get("snapshot_id", ""))
    else:
        rep.asi_v089 = V1189_BASELINE
        rep.notes.append(f"V1189 artifact unavailable → use hardcoded baseline {V1189_BASELINE}")

    rep.asi_v082 = V1182_BASELINE  # 历史 baseline 写死

    # Read V1191 (V1166 lift)
    ok91, data91, reason91 = _read_artifact(v1191_artifact_path)
    rep.notes.append(f"V1191 artifact ({v1191_artifact_path}): {reason91}")
    if ok91 and data91 is not None:
        rep.v1191_new_value = float(data91.get("total", 0.0))
        rep.v1191_snapshot_id = str(data91.get("snapshot_id", ""))
    else:
        rep.v1191_new_value = V1166_BASELINE  # fallback: no lift
        rep.notes.append(f"V1191 artifact unavailable → use V1166 baseline {V1166_BASELINE} (no lift)")

    # Compute lift
    rep.dim_lift_real_llm_benchmark = (rep.v1191_new_value - rep.v1166_old_value) * V1166_WEIGHT
    rep.asi_v064 = rep.asi_v089 + rep.dim_lift_real_llm_benchmark
    rep.delta_asi_v064_vs_v089 = rep.dim_lift_real_llm_benchmark
    rep.delta_asi_v064_vs_v082 = rep.asi_v064 - rep.asi_v082

    # North star
    rep.vs_north_star_gap = NORTH_STAR - rep.asi_v064
    rep.vs_north_star_position_pct = (rep.asi_v064 / NORTH_STAR) * 100.0

    rep.elapsed_seconds = time.time() - t0
    rep.notes.append(
        f"ASI V0.6.4 = V1189 + dim_lift = {rep.asi_v089:.4f} + {rep.dim_lift_real_llm_benchmark:+.4f} = {rep.asi_v064:.4f}; "
        f"vs V1182 (0.7425): delta {rep.delta_asi_v064_vs_v082:+.4f}; "
        f"vs north_star 0.98: gap {rep.vs_north_star_gap:.4f}"
    )

    if write_artifact:
        _write_artifact(rep, artifact_dir, artifact_name)
    return rep


def _write_artifact(rep: V1192Report, artifact_dir: str, artifact_name: str) -> None:
    try:
        ad = Path(artifact_dir)
        ad.mkdir(parents=True, exist_ok=True)
        artifact_path = ad / artifact_name
        artifact_path.write_text(
            json.dumps(rep.to_dict(), indent=2, ensure_ascii=False),
            encoding="utf-8",
        )
        rep.artifact_path = str(artifact_path)
        rep.notes.append(f"artifact written: {rep.artifact_path}")
    except Exception as e:
        rep.notes.append(f"artifact write failed: {e!r}")


# ============================================================================
# Main entry
# ============================================================================


def measure_v1192(
    v1189_artifact_path: str = DEFAULT_V1189_ARTIFACT,
    v1191_artifact_path: str = DEFAULT_V1191_ARTIFACT,
) -> float:
    """V1192 measure → ASI V0.6.4 total (主入口, 主 00:56)."""
    rep = _run_v1192_full(
        v1189_artifact_path=v1189_artifact_path,
        v1191_artifact_path=v1191_artifact_path,
        write_artifact=False,
    )
    return rep.asi_v064


def run_v1192_full(
    v1189_artifact_path: str = DEFAULT_V1189_ARTIFACT,
    v1191_artifact_path: str = DEFAULT_V1191_ARTIFACT,
) -> V1192Report:
    """Run full V1192, write artifact, return V1192Report."""
    return _run_v1192_full(
        v1189_artifact_path=v1189_artifact_path,
        v1191_artifact_path=v1191_artifact_path,
        write_artifact=True,
    )


# ============================================================================
# Markdown report
# ============================================================================


def render_report_md(rep: V1192Report) -> str:
    lines: List[str] = []
    lines.append(f"# V1192 ASI V0.6.4 baseline 报告 — {rep.snapshot_id}\n")
    lines.append(f"- **version**: {rep.version}")
    lines.append(f"- **dim_version**: {rep.dim_version}")
    lines.append(f"- **timestamp**: {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(rep.timestamp))}")
    lines.append(f"- **elapsed**: {rep.elapsed_seconds:.3f}s")
    lines.append(f"- **artifact**: `{rep.artifact_path or 'N/A'}`\n")

    lines.append("## ASI 北极星进度")
    lines.append("| baseline | value |")
    lines.append("|---|---:|")
    lines.append(f"| ASI north star (target) | {NORTH_STAR:.4f} |")
    lines.append(f"| V1182 baseline (V0.6.2)  | {rep.asi_v082:.4f} |")
    lines.append(f"| V1189 baseline (V0.6.3)  | {rep.asi_v089:.4f} |")
    lines.append(f"| **V1192 (V0.6.4)**       | **{rep.asi_v064:.4f}** |")
    lines.append("")

    lines.append("## Deltas")
    lines.append(f"- **vs V1189 (V0.6.3)**: Δ = {rep.delta_asi_v064_vs_v089:+.4f}")
    lines.append(f"- **vs V1182 (V0.6.2)**: Δ = {rep.delta_asi_v064_vs_v082:+.4f}")
    lines.append(f"- **vs ASI north star 0.98**: gap = {rep.vs_north_star_gap:.4f}")
    lines.append(f"- **V1192 position**: {rep.vs_north_star_position_pct:.2f}% of north star\n")

    lines.append("## Real LLM Benchmark Lift (主 06:15 + 主 19:33)")
    lines.append(f"- V1182 baseline value: **{rep.v1166_old_value:.4f}** (V1166 = 0.416, V1133 fail)")
    lines.append(f"- V1191 lift value:      **{rep.v1191_new_value:.4f}** (V1190 真 LLM 接入)")
    lines.append(f"- V1191 snapshot_id:     `{rep.v1191_snapshot_id}`")
    lines.append(f"- V1189 snapshot_id:     `{rep.v1189_snapshot_id}`")
    lines.append(f"- dim weight:            **{rep.dim_weight_real_llm_benchmark:.4f}**")
    lines.append(f"- dim lift:              **{rep.dim_lift_real_llm_benchmark:+.4f}**\n")

    lines.append("## Notes\n")
    for n in rep.notes:
        lines.append(f"- {n}")
    lines.append("")
    lines.append("---")
    lines.append(f"_Generated by V1192 {rep.version}_")
    return "\n".join(lines)


# ============================================================================
# CLI
# ============================================================================


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="V1192 ASI V0.6.4 baseline")
    parser.add_argument("--v1189-artifact", default=DEFAULT_V1189_ARTIFACT)
    parser.add_argument("--v1191-artifact", default=DEFAULT_V1191_ARTIFACT)
    parser.add_argument("--no-write", action="store_true")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--report", action="store_true")
    parser.add_argument("--md-out", default=None)
    parser.add_argument("--artifact-dir", default="artifacts")
    parser.add_argument("--measure", action="store_true")
    args = parser.parse_args(argv)

    if args.measure:
        s = measure_v1192(
            v1189_artifact_path=args.v1189_artifact,
            v1191_artifact_path=args.v1191_artifact,
        )
        print(f"{s:.4f}")
        return 0

    rep = _run_v1192_full(
        v1189_artifact_path=args.v1189_artifact,
        v1191_artifact_path=args.v1191_artifact,
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
    sys.exit(main())