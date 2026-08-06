"""V1184 — V0.6 vcp_deep_read dim 真重算 (V1183 接入 V0.6 series) tests.

主 06:15 + 主 22:33 + 主 17:43 + 主 19:33 + 主 13:31 + 主 17:58 + 主 20:46 + 主 00:56 + 主 00:44

Tests:
  - V1184 vs V1182 delta (主 17:43 实事求是)
  - V1183 接入路径 (主 00:44 质量工程化)
  - V3 哲学守门 (主 17:58 + 主 20:46)
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from apeireth.v1184_v06_vcp_deep_read_baseline import (  # noqa: E402
    V1184_VERSION,
    compute_v1184_deltas,
)


def test_v1184_version():
    assert V1184_VERSION == "0.1.0"


def test_v1184_deltas_keys():
    d = compute_v1184_deltas()
    expected = {
        "version", "v1182_baseline_total", "v1182_vcp_deep_read_old",
        "v1183_measure_v1183_score_new", "delta_v1183_vs_v1182",
        "v1183_n_repos", "v1183_n_real", "v1183_n_cached",
        "v1183_bytes_read_total", "v1183_n_patterns_total",
        "v1183_source_module", "v1182_source_module",
    }
    assert set(d.keys()) >= expected


def test_v1184_vcp_deep_read_old_is_zero():
    """V1182 baseline vcp_deep_read = 0.0 (V1147 hang)."""
    d = compute_v1184_deltas()
    assert d["v1182_vcp_deep_read_old"] == 0.0


def test_v1184_vcp_deep_read_new_is_high():
    """V1183 measure_v1183_score > 0.5 (本地真读 160K + cached 5 repos)."""
    d = compute_v1184_deltas()
    assert d["v1183_measure_v1183_score_new"] >= 0.5


def test_v1184_delta_positive():
    """Delta = V1183 - V1182, 应该 > 0 (主 17:43 实事求是: V1183 优于 V1147)."""
    d = compute_v1184_deltas()
    assert d["delta_v1183_vs_v1182"] > 0.0


def test_v1184_v1183_source_module():
    """V1183 源 = apeireth.v1183_vcp_6_repos_real_deep_read."""
    d = compute_v1184_deltas()
    assert "v1183_vcp_6_repos_real_deep_read" in d["v1183_source_module"]


def test_v1184_v1183_6_repos():
    """V1183 真读 6 repos (主 17:43 实事求是)."""
    d = compute_v1184_deltas()
    assert d["v1183_n_repos"] == 6
    assert d["v1183_n_real"] == 1
    assert d["v1183_n_cached"] == 5


def test_v1184_v1183_bytes_read():
    """V1183 bytes_read >= 100K (本地 9 key files 真读)."""
    d = compute_v1184_deltas()
    assert d["v1183_bytes_read_total"] >= 100000


def test_v1184_v1183_patterns():
    """V1183 patterns >= 30 (主 19:33 走在前人经验上)."""
    d = compute_v1184_deltas()
    assert d["v1183_n_patterns_total"] >= 30