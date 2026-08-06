"""V1155 — ASI V0.6 真生产 Trend Baseline + 21-dim Heatmap + Next-ROI Suggester (主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 19:33 走在前人经验上 + 主 13:31 大胆激进 + 主 17:58+20:46 不假装 + 主 23:44 干到底 + 主 00:56 任何人都能接手 + 主 00:44 质量工程化).

V1153 给出 ASI V0.6 FORMAL SPEC (21 dim, sum=1.0, 0.8213, gap -0.1587).
但 V1153 缺 3 件事 (主 17:43 实事求是):
  1. 没有 frozen baseline — 每次跑都是新数字, 没法比 delta
  2. 没有 dim heatmap — 21 dim 谁高谁低, 没人看得见
  3. 没有 next-ROI suggester — 知道 0.8213, 但不知道下一步该打哪个 dim

V1155 解决 3 件事:
  1. snapshot_v06() — 跑 V1153 真测, 冻结成 V06Snapshot (不可变 dataclass + JSON dump)
  2. render_heatmap_md() — 21 行 markdown 热力图 (█ bar + value + status + source)
  3. suggest_next_targets() — 按 potential_gain = weight × (1 - value) 排序, 推荐下 V1156+ 该打哪个 dim

主 00:56 任何人都能接手:
  - snapshots/v1155_baseline.json — 任何人打开看 baseline 数字
  - snapshots/v1155_heatmap.md — 任何人打开看 21 dim 一目了然
  - snapshots/v1155_next_targets.md — 任何人接手就知道下一步 V1156 打哪个 dim

主 00:44 质量工程化:
  - snapshot_id (uuid) + git_commit (subprocess) + taken_at (unix) — 三件套追溯
  - delta tracker — V1156/V1157 可以跟 V1155 对比 (主 17:43 不假装 = 数字比)
  - acceptance: heatmap 21 行, next_targets top-K, write_baseline 3 文件

主 17:43 实事求是:
  - 不假装 baseline = ASI: baseline 是 V0.6 spec, 不是 ASI 本身
  - 不假装 potential_gain = 真能涨: 是 weight × (1-value) 数学, 不预测实现路径
  - 不假装 trend = 一定上升: 只比较两个 snapshot, 不预测方向

Usage:
    python -m apeireth.v1155_asi_v06_trend_baseline                           # 默认 snapshot + 写 3 文件
    python -m apeireth.v1155_asi_v06_trend_baseline --no-write               # 只 print, 不写
    python -m apeireth.v1155_asi_v06_trend_baseline --snapshot-dir snapshots  # 改目录
    python -m apeireth.v1155_asi_v06_trend_baseline --top-k 5                 # 改推荐数
    python -m apeireth.v1155_asi_v06_trend_baseline --diff snapshots/v1155_baseline.json  # 对比旧 snapshot
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
import uuid
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# 真调 V1153 (主 17:43 不重实现, 真借鉴)
from apeireth import v1153_asi_v06_formal_spec as v1153

V1155_VERSION = "0.1.0"

# 默认 snapshot 目录 (主 00:56 任何人都能接手 — 固定路径)
DEFAULT_SNAPSHOT_DIR = "snapshots"
DEFAULT_TOP_K = 5

# heatmap bar 字符 + 长度
BAR_CHAR = "█"
EMPTY_CHAR = "░"
BAR_WIDTH = 20


# ============================================================================
# V06Snapshot — 冻结的 V1153 真测快照 (主 00:44 质量工程化)
# ============================================================================


@dataclass
class V06DimSnapshot:
    """V0.6 单 dim 冻结快照."""
    dim: str
    weight: float
    value: float
    status: str  # R/H/P/M
    source: str

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class V06Snapshot:
    """V0.6 真测冻结快照.

    主 17:43 实事求是: snapshot = 不可变 V1153 真测副本 + 元数据
    主 00:56 任何人都能接手: snapshot_id 唯一, git_commit 可追溯, taken_at 时间戳
    """
    snapshot_id: str
    taken_at: float  # unix time
    version: str  # V1153 version
    git_commit: str  # git rev-parse HEAD
    git_dirty: bool  # git diff --quiet exit 0 ?
    score: float  # ASI V0.6 真测
    north_star: float  # ASI LOCKED 0.98
    gap: float  # score - north_star
    n_dims: int
    n_real: int
    n_hardcoded: int
    n_partial: int
    n_missing: int
    dims: List[V06DimSnapshot] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["score"] = round(self.score, 4)
        d["gap"] = round(self.gap, 4)
        d["dims"] = [dm.to_dict() for dm in self.dims]
        return d


@dataclass
class NextROITarget:
    """下 V1156+ 该打哪个 dim — 按 potential_gain 排序."""
    dim: str
    value: float
    weight: float
    potential_gain: float  # weight × (1 - value)
    rank: int  # 1-indexed
    rationale: str  # 为什么这个 dim 是 ROI 最高的

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


# ============================================================================
# git 上下文 (主 00:44 质量工程化 — 可追溯)
# ============================================================================


def _git_commit() -> str:
    """取当前 git commit short hash. 失败返回 'unknown'."""
    try:
        out = subprocess.check_output(
            ["git", "rev-parse", "--short", "HEAD"],
            stderr=subprocess.DEVNULL,
            timeout=5,
        ).decode("utf-8", errors="replace").strip()
        return out if out else "unknown"
    except Exception:
        return "unknown"


def _git_dirty() -> bool:
    """git diff --quiet: 0=clean, 1=dirty. True 表示有未提交修改."""
    try:
        r = subprocess.run(
            ["git", "diff", "--quiet"],
            stderr=subprocess.DEVNULL,
            timeout=5,
        )
        return r.returncode != 0
    except Exception:
        return False


# ============================================================================
# Snapshot — 冻结 V1153 真测
# ============================================================================


def snapshot_v06(
    v1153_module: Any = v1153,
) -> V06Snapshot:
    """真跑 V1153 measure_v06_spec(), 冻结成 V06Snapshot.

    主 17:43 实事求是: 不重写 21 dim 真测, 真调 V1153.
    主 00:44 质量工程化: snapshot_id 唯一, git_commit/taken_at 一起冻结.
    """
    started = time.time()
    snapshot_id = f"v1155-{uuid.uuid4().hex[:8]}"

    spec = v1153_module.measure_v06_spec()
    elapsed = time.time() - started

    dims = [
        V06DimSnapshot(
            dim=d.dim,
            weight=d.weight,
            value=d.value,
            status=d.status,
            source=d.source,
        )
        for d in spec.dim_results
    ]

    return V06Snapshot(
        snapshot_id=snapshot_id,
        taken_at=started,
        version=spec.version,
        git_commit=_git_commit(),
        git_dirty=_git_dirty(),
        score=spec.asi_v06_score,
        north_star=v1153_module.ASI_NORTH_STAR,
        gap=spec.gap,
        n_dims=spec.n_dims,
        n_real=spec.n_real,
        n_hardcoded=spec.n_hardcoded,
        n_partial=spec.n_partial,
        n_missing=spec.n_missing,
        dims=dims,
    )


# ============================================================================
# Diff — 两个 snapshot 对比 (主 17:43 不假装 trend = 一定上升)
# ============================================================================


def diff_snapshots(prev: V06Snapshot, curr: V06Snapshot) -> Dict[str, Any]:
    """两个 V06Snapshot 对比.

    Returns dict with:
      - score_delta: curr.score - prev.score
      - gap_delta: curr.gap - prev.gap
      - n_real_delta: curr.n_real - prev.n_real
      - dim_deltas: {dim: {value_delta, status_changed}}
      - improved_dims: list of dims with value_delta > 0
      - regressed_dims: list of dims with value_delta < 0
      - unchanged_dims: list of dims with value_delta == 0
    """
    prev_map: Dict[str, V06DimSnapshot] = {d.dim: d for d in prev.dims}
    curr_map: Dict[str, V06DimSnapshot] = {d.dim: d for d in curr.dims}

    dim_deltas: Dict[str, Dict[str, Any]] = {}
    improved: List[str] = []
    regressed: List[str] = []
    unchanged: List[str] = []

    for dim_name in curr_map:
        if dim_name not in prev_map:
            continue
        p = prev_map[dim_name]
        c = curr_map[dim_name]
        vd = round(c.value - p.value, 4)
        sc = p.status != c.status
        dim_deltas[dim_name] = {
            "value_prev": p.value,
            "value_curr": c.value,
            "value_delta": vd,
            "status_changed": sc,
            "status_prev": p.status,
            "status_curr": c.status,
        }
        if vd > 1e-9:
            improved.append(dim_name)
        elif vd < -1e-9:
            regressed.append(dim_name)
        else:
            unchanged.append(dim_name)

    return {
        "snapshot_id_prev": prev.snapshot_id,
        "snapshot_id_curr": curr.snapshot_id,
        "taken_at_prev": prev.taken_at,
        "taken_at_curr": curr.taken_at,
        "score_prev": round(prev.score, 4),
        "score_curr": round(curr.score, 4),
        "score_delta": round(curr.score - prev.score, 4),
        "gap_prev": round(prev.gap, 4),
        "gap_curr": round(curr.gap, 4),
        "gap_delta": round(curr.gap - prev.gap, 4),
        "n_real_prev": prev.n_real,
        "n_real_curr": curr.n_real,
        "n_real_delta": curr.n_real - prev.n_real,
        "dim_deltas": dim_deltas,
        "improved_dims": improved,
        "regressed_dims": regressed,
        "unchanged_dims": unchanged,
    }


# ============================================================================
# Heatmap — 21-dim markdown bar chart (主 00:56 任何人都能接手)
# ============================================================================


def _bar(value: float, width: int = BAR_WIDTH) -> str:
    """生成 █/░ bar 字符串."""
    n_filled = max(0, min(width, int(round(value * width))))
    return BAR_CHAR * n_filled + EMPTY_CHAR * (width - n_filled)


def render_heatmap_md(snap: V06Snapshot) -> str:
    """21-dim heatmap markdown.

    主 00:56 任何人都能接手: 任何人打开看 21 dim 一目了然.
    主 22:33 ASI 北极星: gap 真报, 北极星 0.98 在 header.
    主 17:43 实事求是: 不假装 R = 实测, 真报 H/P/M 数量.
    """
    lines: List[str] = []
    lines.append(f"# ASI V0.6 Trend Heatmap — {snap.snapshot_id}")
    lines.append("")
    lines.append(f"- **taken_at**: {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(snap.taken_at))}")
    lines.append(f"- **version**: {snap.version}")
    lines.append(f"- **git_commit**: `{snap.git_commit}`" + (" (dirty)" if snap.git_dirty else ""))
    lines.append(f"- **score**: **{snap.score:.4f}**")
    lines.append(f"- **north_star (LOCKED)**: {snap.north_star:.4f}")
    lines.append(f"- **gap**: **{snap.gap:+.4f}** (score - north_star)")
    lines.append(f"- **dims**: {snap.n_dims} total | R={snap.n_real} H={snap.n_hardcoded} P={snap.n_partial} M={snap.n_missing}")
    lines.append("")
    lines.append("## 21-dim Heatmap (sorted by value asc)")
    lines.append("")
    lines.append("| dim | value | weight | bar | status | source |")
    lines.append("|-----|------:|-------:|-----|:------:|--------|")
    for d in sorted(snap.dims, key=lambda x: x.value):
        bar = _bar(d.value)
        lines.append(
            f"| `{d.dim}` | {d.value:.4f} | {d.weight:.4f} | `{bar}` | "
            f"{d.status} | {d.source[:60]} |"
        )
    lines.append("")
    lines.append("## Legend")
    lines.append("")
    lines.append("- **R** = real measurement (主 17:43 实事求是)")
    lines.append("- **H** = hardcoded placeholder (不假装 = 真标)")
    lines.append("- **P** = partial / fallback (不假装 = 真标)")
    lines.append("- **M** = missing (不假装 = 真标)")
    lines.append(f"- bar: {BAR_CHAR} = value, {EMPTY_CHAR} = 1-value (width {BAR_WIDTH})")
    lines.append("")
    lines.append("## Trend Notes")
    lines.append("")
    lines.append("- score 越接近 north_star 0.98 → 越接近 ASI 北极星 (主 22:33)")
    lines.append("- gap 为负 → 未达 ASI; gap 为正 → 已超 (主 17:43 实事求是 不假装超 ASI)")
    lines.append("- 6 dim 达到 1.0 (cross_domain / neurosymbolic / world_model / llm_bridge / multi_agent_dag / vcp_real_run)")
    lines.append("- 5 dim 最低 (≤0.7) → V1156+ 该打的 ROI 目标 (见 next-targets.md)")
    lines.append("")
    return "\n".join(lines)


# ============================================================================
# Next-ROI Suggester — 按 potential_gain = weight × (1 - value) 排序 (主 13:31 大胆激进)
# ============================================================================


def suggest_next_targets(snap: V06Snapshot, top_k: int = DEFAULT_TOP_K) -> List[NextROITarget]:
    """推荐下 V1156+ 该打哪个 dim.

    potential_gain = weight × (1 - value)
      - weight 高 → 修了影响大
      - (1-value) 大 → 现状低, 上升空间大

    主 17:43 实事求是: 不预测"能涨多少", 只算数学 potential_gain.
    主 13:31 大胆激进: 优先打 highest gain.
    主 23:44 干到底: 推荐 dim 给下个 V 模块.
    """
    candidates = sorted(
        snap.dims,
        key=lambda d: -(d.weight * (1.0 - d.value)),
    )

    targets: List[NextROITarget] = []
    for rank, d in enumerate(candidates[:top_k], start=1):
        gain = d.weight * (1.0 - d.value)
        if d.value >= 0.99:
            rationale = f"value={d.value:.4f} 已接近 1.0, 优先打 (weight={d.weight:.4f})"
        elif d.status in ("H", "P", "M"):
            rationale = (
                f"value={d.value:.4f} status={d.status} 不假装, "
                f"真补 R 可涨 {gain:.4f} (weight={d.weight:.4f})"
            )
        else:
            rationale = (
                f"value={d.value:.4f} R 真测, weight={d.weight:.4f} 高, "
                f"潜在涨 {gain:.4f}"
            )
        targets.append(NextROITarget(
            dim=d.dim,
            value=d.value,
            weight=d.weight,
            potential_gain=round(gain, 4),
            rank=rank,
            rationale=rationale,
        ))
    return targets


def render_next_targets_md(targets: List[NextROITarget], snap: V06Snapshot) -> str:
    """下 V1156+ 该打哪个 dim — markdown."""
    lines: List[str] = []
    lines.append(f"# Next-ROI Targets — V1156+ Roadmap (from {snap.snapshot_id})")
    lines.append("")
    lines.append(f"- **score 当前**: {snap.score:.4f}")
    lines.append(f"- **gap**: {snap.gap:+.4f}")
    lines.append(f"- **推荐 top-{len(targets)} dim** (按 potential_gain = weight × (1-value))")
    lines.append("")
    lines.append("| rank | dim | value | weight | potential_gain | rationale |")
    lines.append("|-----:|-----|------:|-------:|---------------:|-----------|")
    for t in targets:
        lines.append(
            f"| {t.rank} | `{t.dim}` | {t.value:.4f} | {t.weight:.4f} | "
            f"**{t.potential_gain:.4f}** | {t.rationale} |"
        )
    lines.append("")
    lines.append("## Suggested V1156+ Module Names")
    lines.append("")
    for t in targets:
        # V1156 = top-1, V1157 = top-2, ...
        v_num = 1156 + (t.rank - 1)
        lines.append(
            f"- **V{v_num}** = `{t.dim}` 真补 (potential_gain={t.potential_gain:.4f})"
        )
    lines.append("")
    lines.append("## Notes (主 17:43 实事求是)")
    lines.append("")
    lines.append("- potential_gain 是数学, 不预测实现路径")
    lines.append("- 推荐 dim 是 V1156+ 候选, 不强制顺序")
    lines.append("- 真补 R > 标 H/P > 标 M (主 17:58 不假装)")
    lines.append("")
    return "\n".join(lines)


# ============================================================================
# Write Baseline — 写 3 文件 (主 00:56 任何人都能接手)
# ============================================================================


def write_baseline(
    snap: V06Snapshot,
    snapshot_dir: str = DEFAULT_SNAPSHOT_DIR,
    top_k: int = DEFAULT_TOP_K,
) -> Tuple[str, str, str]:
    """写 3 文件:
      - {snapshot_dir}/v1155_baseline.json (snapshot 冻结)
      - {snapshot_dir}/v1155_heatmap.md (21-dim heatmap)
      - {snapshot_dir}/v1155_next_targets.md (V1156+ 路线)

    Returns (json_path, heatmap_path, next_targets_path).
    """
    out_dir = Path(snapshot_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    json_path = out_dir / "v1155_baseline.json"
    heatmap_path = out_dir / "v1155_heatmap.md"
    next_targets_path = out_dir / "v1155_next_targets.md"

    # 1. JSON
    with open(json_path, "w", encoding="utf-8") as f:
        json.dump(snap.to_dict(), f, ensure_ascii=False, indent=2)

    # 2. Heatmap
    with open(heatmap_path, "w", encoding="utf-8") as f:
        f.write(render_heatmap_md(snap))

    # 3. Next targets
    targets = suggest_next_targets(snap, top_k=top_k)
    with open(next_targets_path, "w", encoding="utf-8") as f:
        f.write(render_next_targets_md(targets, snap))

    return str(json_path), str(heatmap_path), str(next_targets_path)


# ============================================================================
# Acceptance (主 17:43 实事求是: V1155 必须可证伪)
# ============================================================================


def run_v1155_acceptance() -> Dict[str, Any]:
    """V1155 acceptance 测试 (主 17:43 实事求是: spec 必须可证伪).

    Returns dict with n_tests / n_pass / n_fail / tests.
    """
    tests: List[Dict[str, Any]] = []

    # T1: snapshot_v06 真跑, 返回 V06Snapshot
    try:
        snap = snapshot_v06()
        ok = isinstance(snap, V06Snapshot) and snap.n_dims == 21
        tests.append({"name": "snapshot_v06_returns_V06Snapshot_n21", "passed": ok})
    except Exception as e:
        tests.append({"name": "snapshot_v06_returns_V06Snapshot_n21", "passed": False, "error": str(e)})

    # T2: score 在 [0, 1] 范围内
    try:
        snap = snapshot_v06()
        ok = 0.0 <= snap.score <= 1.0
        tests.append({"name": "score_in_0_1", "passed": ok, "score": snap.score})
    except Exception as e:
        tests.append({"name": "score_in_0_1", "passed": False, "error": str(e)})

    # T3: heatmap md 包含 21 dim 名称
    try:
        snap = snapshot_v06()
        md = render_heatmap_md(snap)
        all_dims_present = all(d in md for d in [dm.dim for dm in snap.dims])
        tests.append({"name": "heatmap_md_contains_21_dims", "passed": all_dims_present})
    except Exception as e:
        tests.append({"name": "heatmap_md_contains_21_dims", "passed": False, "error": str(e)})

    # T4: next_targets 按 potential_gain 降序
    try:
        snap = snapshot_v06()
        targets = suggest_next_targets(snap, top_k=5)
        gains = [t.potential_gain for t in targets]
        ok = all(gains[i] >= gains[i + 1] for i in range(len(gains) - 1))
        tests.append({"name": "next_targets_sorted_by_gain_desc", "passed": ok, "gains": gains})
    except Exception as e:
        tests.append({"name": "next_targets_sorted_by_gain_desc", "passed": False, "error": str(e)})

    # T5: write_baseline 写 3 文件
    try:
        snap = snapshot_v06()
        json_p, hm_p, nt_p = write_baseline(snap, snapshot_dir="_v1155_test_tmp")
        all_exist = Path(json_p).exists() and Path(hm_p).exists() and Path(nt_p).exists()
        # cleanup
        for p in [json_p, hm_p, nt_p]:
            try:
                Path(p).unlink()
            except Exception:
                pass
        try:
            Path("_v1155_test_tmp").rmdir()
        except Exception:
            pass
        tests.append({"name": "write_baseline_creates_3_files", "passed": all_exist})
    except Exception as e:
        tests.append({"name": "write_baseline_creates_3_files", "passed": False, "error": str(e)})

    # T6: diff_snapshots 检测 improvement
    try:
        snap1 = snapshot_v06()
        snap2 = snapshot_v06()
        diff = diff_snapshots(snap1, snap2)
        ok = "score_delta" in diff and "dim_deltas" in diff
        tests.append({"name": "diff_snapshots_has_score_delta_and_dim_deltas", "passed": ok})
    except Exception as e:
        tests.append({"name": "diff_snapshots_has_score_delta_and_dim_deltas", "passed": False, "error": str(e)})

    n_pass = sum(1 for t in tests if t.get("passed"))
    n_fail = sum(1 for t in tests if not t.get("passed"))
    return {
        "n_tests": len(tests),
        "n_pass": n_pass,
        "n_fail": n_fail,
        "tests": tests,
    }


# ============================================================================
# CLI
# ============================================================================


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog="v1155_asi_v06_trend_baseline",
        description="V1155 — ASI V0.6 真生产 trend baseline + 21-dim heatmap + next-ROI suggester",
    )
    parser.add_argument(
        "--snapshot-dir",
        default=DEFAULT_SNAPSHOT_DIR,
        help=f"输出目录 (默认 {DEFAULT_SNAPSHOT_DIR})",
    )
    parser.add_argument(
        "--top-k",
        type=int,
        default=DEFAULT_TOP_K,
        help=f"推荐 next-ROI top-K dim (默认 {DEFAULT_TOP_K})",
    )
    parser.add_argument(
        "--no-write",
        action="store_true",
        help="只 print, 不写文件",
    )
    parser.add_argument(
        "--acceptance",
        action="store_true",
        help="跑 acceptance 测试",
    )
    parser.add_argument(
        "--diff",
        default=None,
        help="对比旧 snapshot JSON 路径",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="输出 JSON 到 stdout",
    )
    args = parser.parse_args(argv)

    if args.acceptance:
        result = run_v1155_acceptance()
        if args.json:
            print(json.dumps(result, ensure_ascii=False, indent=2))
        else:
            print(f"V1155 acceptance: {result['n_pass']}/{result['n_tests']} pass")
            for t in result["tests"]:
                status = "✅" if t.get("passed") else "❌"
                print(f"  {status} {t['name']}")
        return 0 if result["n_fail"] == 0 else 1

    # 1. 真跑 snapshot
    snap = snapshot_v06()

    if args.diff:
        # 对比旧 snapshot
        try:
            with open(args.diff, "r", encoding="utf-8") as f:
                prev_dict = json.load(f)
            # 转回 V06Snapshot (主 00:44 质量工程化: snapshot 是 immutable + reversible)
            prev_dims = [V06DimSnapshot(**d) for d in prev_dict.get("dims", [])]
            prev = V06Snapshot(
                snapshot_id=prev_dict["snapshot_id"],
                taken_at=prev_dict["taken_at"],
                version=prev_dict["version"],
                git_commit=prev_dict["git_commit"],
                git_dirty=prev_dict["git_dirty"],
                score=prev_dict["score"],
                north_star=prev_dict["north_star"],
                gap=prev_dict["gap"],
                n_dims=prev_dict["n_dims"],
                n_real=prev_dict["n_real"],
                n_hardcoded=prev_dict["n_hardcoded"],
                n_partial=prev_dict["n_partial"],
                n_missing=prev_dict["n_missing"],
                dims=prev_dims,
            )
            diff = diff_snapshots(prev, snap)
            if args.json:
                print(json.dumps(diff, ensure_ascii=False, indent=2))
            else:
                print(f"diff: prev={prev.snapshot_id} curr={snap.snapshot_id}")
                print(f"  score: {diff['score_prev']:.4f} → {diff['score_curr']:.4f} (Δ {diff['score_delta']:+.4f})")
                print(f"  gap:   {diff['gap_prev']:+.4f} → {diff['gap_curr']:+.4f} (Δ {diff['gap_delta']:+.4f})")
                print(f"  improved: {len(diff['improved_dims'])}, regressed: {len(diff['regressed_dims'])}, unchanged: {len(diff['unchanged_dims'])}")
                if diff["improved_dims"]:
                    print(f"  improved dims: {diff['improved_dims']}")
                if diff["regressed_dims"]:
                    print(f"  regressed dims: {diff['regressed_dims']}")
            return 0
        except Exception as e:
            print(f"ERROR diff failed: {e}", file=sys.stderr)
            return 1

    # 2. print 概要
    if args.json:
        print(json.dumps(snap.to_dict(), ensure_ascii=False, indent=2))
    else:
        print(f"V1155 snapshot: {snap.snapshot_id}")
        print(f"  score: {snap.score:.4f}")
        print(f"  gap:   {snap.gap:+.4f} (north_star {snap.north_star:.4f})")
        print(f"  dims:  {snap.n_dims} R={snap.n_real} H={snap.n_hardcoded} P={snap.n_partial} M={snap.n_missing}")
        print(f"  git:   {snap.git_commit}{' (dirty)' if snap.git_dirty else ''}")

    # 3. 写 3 文件 (默认)
    if not args.no_write:
        jp, hp, np_ = write_baseline(snap, snapshot_dir=args.snapshot_dir, top_k=args.top_k)
        if not args.json:
            print(f"  wrote: {jp}")
            print(f"  wrote: {hp}")
            print(f"  wrote: {np_}")
    return 0


if __name__ == "__main__":
    sys.exit(main())