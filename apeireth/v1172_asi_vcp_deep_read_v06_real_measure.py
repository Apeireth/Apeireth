"""V1172 — ASI vcp_deep_read V0.6 follow-up (5 sub-dim 真测).

主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 19:33 走在前人经验上 + 主 13:31 大胆激进 +
主 17:58+20:46 不假装 + 主 23:44 干到底 + 主 00:56 任何人都能接手 + 主 00:44 质量工程化 +
主 06:15 V1053+ VCP 真源代码深读 + 主 14:50 V1170 alt runtime 真补 + 主 14:47 V1171 V0.6.1 patched.

主 17:43 实事求是真问题 (V1155 baseline):
  - V1155 vcp_deep_read = 0.6667 (V1147 source = V0.4 era — LOWEST of 21 dims)
  - V1147 VCP5DeepReadReport 真读 5 repos 默认 (n_real/n_partial/n_mock/n_missing)
  - V1147 没拆 5 sub-dim 真测, V1155 baseline 直接给了 0.6667 (over-fit)
  - V1172 = V0.6 series follow-up, 把 V1147 VCP5DeepReadReport 真 9 字段拆成 5 sub-dim 真测

V1172 真补路径 (主 17:43 实事求是):
  - 5 sub-dim 真测 (基于 V1147 VCP5DeepReadReport 真字段):
    D1 repo_inventory_real          — V1147 VCP_5_REPOS 5 仓库完整 + 5 字段 (name/url/owner/purpose/keywords)
    D2 github_api_reachability_real — V1147 n_http_requests_total ≥ 5 + repos[].status ∈ {REAL,PARTIAL,MOCK,MISSING}
    D3 pattern_extraction_real      — V1147 n_patterns_total ≥ 1 + repos[].patterns 中至少有 5 个非空
    D4 v06_mapping_real             — V1147 n_v06_mappings_total ≥ 1 + repos[].v06_mappings 中至少有 5 个非空
    D5 real_to_mock_ratio_real      — V1147 n_real / n_repos ≥ 0.6 (≥3 real reads); MISSING < 4
  - aggregate = mean(sub_dim_scores) ∈ [0, 1]
  - 任何 sub-dim 失败 → sub-dim score 衰减 (主 17:43 不刷 KPI)

主 00:56 任何人都能接手:
  - measure_vcp_deep_read_v06() → float (0..1) 主入口
  - measure_vcp_deep_read_v06_full() → VcpDeepReadV06Report dataclass + JSON dump
  - VcpDeepReadV06Report JSON 写 artifacts/v1172_vcp_deep_read_v06.json

主 00:44 质量工程化:
  - VcpDeepReadV06Report (主 22:33 北极星):
      total, sub_dim_scores (dict 5 keys), sub_dim_evidence (dict 5 keys)
      version, timestamp, snapshot_id (uuid), elapsed_seconds
      v1147_snapshot_id, v1147_n_real, v1147_n_patterns_total, v1147_n_v06_mappings_total

主 17:58 + 20:46 不假装:
  - 不假装 5 仓库在 = VCP 全部: V1147 = 5 VCP-adjacent repos, VCP 是更大概念
  - 不假装 web_fetch 200 = 文件真 clone: V1147 元数据 + README 真读, 不假装 git clone history
  - 不假装 1 个 pattern 借 = 1 行复用: 启发 + 映射, 不是单向复制
  - 不假装 V1172 = VCP ASI 全面: V1172 是 V1147 真实测的 V0.6 升级, 不假装 ASI VCP 突破
  - 不假装 V1155 hot-patched 0.6667 = 真测量: V1172 真 5 sub-dim 拆分, 写死 partial ≠ 满分

Usage:
    python -m apeireth.v1172_asi_vcp_deep_read_v06_real_measure              # 默认 measure + JSON dump
    python -m apeireth.v1172_asi_vcp_deep_read_v06_real_measure --json      # JSON stdout
    python -m apeireth.v1172_asi_vcp_deep_read_v06_real_measure --no-write  # 只 print
    python -m apeireth.v1172_asi_vcp_deep_read_v06_real_measure --report    # markdown 报告
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


V1172_VERSION = "0.1.0"
V1172_DIM_VERSION = "0.6"

# 5 sub-dim names (LOCKED 主 19:33 走在前人经验上 — V1147 5 axis 真测)
V1172_SUBDIM_NAMES: Tuple[str, ...] = (
    "repo_inventory_real",          # D1 — V1147 VCP_5_REPOS 5 仓库完整
    "github_api_reachability_real", # D2 — V1147 n_http_requests + status 真测
    "pattern_extraction_real",      # D3 — V1147 patterns 真测
    "v06_mapping_real",             # D4 — V1147 v06_mappings 真测
    "real_to_mock_ratio_real",      # D5 — V1147 n_real 比例
)

DEFAULT_ARTIFACT_DIR = "artifacts"

# V1155 baseline (主 17:43 实事求是 — 写死历史 hot-patched 值)
V1155_BASELINE_VCP_DEEP_READ = 0.6667

# V1147 真字段 (主 17:43 实事求是)
V1147_REPORT_FIELDS: Tuple[str, ...] = (
    "snapshot_id",
    "started_at",
    "finished_at",
    "version",
    "n_repos",
    "n_real",
    "n_partial",
    "n_mock",
    "n_missing",
    "n_patterns_total",
    "n_v06_mappings_total",
    "n_http_requests_total",
    "repos",
    "philos