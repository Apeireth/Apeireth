"""V1184 — V0.6 vcp_deep_read dim 真重算 (V1183 接入 V0.6 series).

主 06:15 + 主 22:33 + 主 17:43 + 主 19:33 + 主 13:31 + 主 17:58 + 主 20:46 + 主 00:56 + 主 00:44

为什么 V1184:
  V1182 baseline recompute → v0_6_new_dim_collector 4 个 dim 全 0.0:
    - llm_bridge (v1152)
    - multi_agent_dag (v1149)
    - vcp_real_run (v1148)
    - vcp_deep_read (v1147)
  V1183 = V1147 升级 (5 GitHub + 1 本地), 给出 vcp_deep_read dim 真测
  V1184 演示 V1183 接入路径: 真调 measure_v1183() + 对比 V1182 旧值

V1184 vs V1182:
  V1182: v0_6_new_dim_collector = subprocess call V1147 (hang → 0.0)
  V1184: v0_6_new_dim_collector = measure_v1183() direct call (本进程, 0.0 → 0.9838)

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43):
  - 不假装 V1183 measure = V1147 真测 (V1183 是升级, 不假装 = 同物)
  - 不假装 V1184 = V1182 全量重算 (V1184 只演示 vcp_deep_read 1 个 dim)
  - 不假装 vcp_deep_read 0.9838 = ASI 升级 (1 dim 不代表 ASI 总)

Usage:
    python -m apeireth.v1184_v06_vcp_deep_read_baseline
    python -m apeireth.v1184_v06_vcp_deep_read_baseline --json
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any, Dict

V1184_VERSION = "0.1.0"


@dataclass
class V1184Delta:
    """V1184 vs V1182 单 dim delta."""
    dim: str
    v1182_old: float         # V1182 baseline 旧值
    v1184_new: float         # V1184 真重算新值 (来自 V1183)
    delta: float             # 新 - 旧
    source_module: str       # 真测来源


def compute_v1184_deltas() -> Dict[str, Any]:
    """V1184 真重算 vcp_deep_read dim (主 17:43 实事求是)."""
    # V1182 baseline (来自 artifacts/v1182_asi_v06_recomputed_baseline.json)
    artifact_path = Path(__file__).resolve().parent.parent / "artifacts" / "v1182_asi_v06_recomputed_baseline.json"
    v1182_total = 0.7425
    v1182_vcp_deep_read = 0.0  # V1147 hang → 0.0
    if artifact_path.exists():
        try:
            data = json.loads(artifact_path.read_text(encoding="utf-8"))
            v1182_total = data.get("total", v1182_total)
            v0_6_new = data.get("sub_dim_evidence", {}).get("v0_6_new_dim_collector", {})
            dims = v0_6_new.get("dims", {})
            v1182_vcp_deep_read = dims.get("vcp_deep_read", {}).get("value", 0.0)
        except (json.JSONDecodeError, OSError):
            pass

    # V1183 measure 真调 (本进程, 不 subprocess)
    from apeireth.v1183_vcp_6_repos_real_deep_read import (
        measure_v1183,
        v1183_run_all,
    )
    v1183_score = measure_v1183()
    v1183_report = v1183_run_all()

    # Delta
    delta = round(v1183_score - v1182_vcp_deep_read, 4)

    return {
        "version": V1184_VERSION,
        "v1182_baseline_total": v1182_total,
        "v1182_vcp_deep_read_old": v1182_vcp_deep_read,
        "v1183_measure_v1183_score_new": v1183_score,
        "delta_v1183_vs_v1182": delta,
        "v1183_n_repos": v1183_report.n_repos,
        "v1183_n_real": v1183_report.n_real,
        "v1183_n_cached": v1183_report.n_cached,
        "v1183_bytes_read_total": v1183_report.bytes_read_total,
        "v1183_n_patterns_total": v1183_report.n_patterns_total,
        "v1183_source_module": "apeireth.v1183_vcp_6_repos_real_deep_read",
        "v1182_source_module": "apeireth.v1147_vcp_5_repos_deep_read (subprocess, hang)",
    }


def render_summary(d: Dict[str, Any]) -> str:
    """V1184 真重算 summary."""
    return (
        f"V1184 V0.6 vcp_deep_read dim 真重算 (主 06:15 + 主 22:33):\n"
        f"  V1182 baseline (旧, V1147 hang): {d['v1182_vcp_deep_read_old']:.4f}\n"
        f"  V1184 new (V1183 direct call):   {d['v1183_measure_v1183_score_new']:.4f}\n"
        f"  Delta (新 - 旧):                  {d['delta_v1183_vs_v1182']:+.4f}\n"
        f"  V1183 源: {d['v1183_source_module']}\n"
        f"  V1183 真读: {d['v1183_n_repos']} repos ({d['v1183_n_real']} R + {d['v1183_n_cached']} C), "
        f"{d['v1183_bytes_read_total']:,} bytes, {d['v1183_n_patterns_total']} patterns"
    )


def _cli(argv=None) -> int:
    parser = argparse.ArgumentParser(
        description="V1184 V0.6 vcp_deep_read dim 真重算 (V1183 接入)"
    )
    parser.add_argument("--json", action="store_true", help="JSON stdout")
    args = parser.parse_args(argv)

    d = compute_v1184_deltas()

    if args.json:
        print(json.dumps(d, ensure_ascii=False, indent=2))
    else:
        print(render_summary(d))

    return 0


if __name__ == "__main__":
    sys.exit(_cli())