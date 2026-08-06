"""
V1154 tests — ASI time-philosophy real measurement.

(主 17:43 实事求是: 测试必真跑必检出失败, 不假装)
"""
from __future__ import annotations

import json
import time
from pathlib import Path

import pytest

from apeireth.v1154_asi_time_philosophy_real_measure import (
    V1154_DIM_NAMES,
    V1154_VERSION,
    TimeGroundingReport,
    _measure_causal_order_awareness,
    _measure_duration_self_perception,
    _measure_interval_reasoning,
    _measure_monotonic_elapsed,
    _measure_wall_clock_grounding,
    measure_time_grounding,
)


# ---------------------------------------------------------------------------
# 基础结构 (保证 5 sub-dim 真存在, 不为空, 主 17:43 不假装)
# ---------------------------------------------------------------------------

def test_v1154_dim_names_is_five():
    assert len(V1154_DIM_NAMES) == 5
    assert "wall_clock_grounding" in V1154_DIM_NAMES
    assert "monotonic_elapsed" in V1154_DIM_NAMES
    assert "interval_reasoning" in V1154_DIM_NAMES
    assert "causal_order_awareness" in V1154_DIM_NAMES
    assert "duration_self_perception" in V1154_DIM_NAMES


def test_v1154_dim_names_are_locked():
    locked = (
        "wall_clock_grounding",
        "monotonic_elapsed",
        "interval_reasoning",
        "causal_order_awareness",
        "duration_self_perception",
    )
    assert V1154_DIM_NAMES == locked


def test_v1154_version_is_semver():
    parts = V1154_VERSION.split(".")
    assert len(parts) == 3
    assert all(p.isdigit() for p in parts)


# ---------------------------------------------------------------------------
# sub-measurer 单独测
# ---------------------------------------------------------------------------

def test_wall_clock_grounding_scores_high():
    sc, ev = _measure_wall_clock_grounding()
    assert 0.0 <= sc <= 1.0
    # 真实系统 time.time / datetime.now / tz offset 必存在
    assert sc >= 0.99, f"T1 should be ~1.0 on real systems, got {sc}: {ev}"
    assert ev["checks"]["time_time_returns_recent_epoch"] is True
    assert ev["checks"]["datetime_iso_roundtrip"] is True
    assert ev["checks"]["local_tz_offset_present"] is True


def test_monotonic_elapsed_scores_high():
    sc, ev = _measure_monotonic_elapsed()
    assert 0.0 <= sc <= 1.0
    assert sc >= 0.95, f"T2 should be ~1.0 on real systems, got {sc}: {ev}"
    assert ev["checks"]["monotonic_non_decreasing"] is True
    assert ev["checks"]["sleep_real_elapsed"] is True


def test_interval_reasoning_scores_high():
    sc, ev = _measure_interval_reasoning()
    assert 0.0 <= sc <= 1.0
    assert sc >= 0.5, f"T3 should be ≥0.5 on real systems, got {sc}: {ev}"
    assert ev["checks"]["timedelta_forward_back"] is True


def test_causal_order_awareness_perfect():
    sc, ev = _measure_causal_order_awareness()
    assert sc == pytest.approx(1.0, abs=1e-9)
    assert ev["checks"]["before_event_lt_after_event"] is True
    assert ev["checks"]["three_event_total_order"] is True


def test_duration_self_perception_high():
    sc, ev = _measure_duration_self_perception()
    assert 0.0 <= sc <= 1.0
    assert sc >= 0.8, f"T5 should be ≥0.8 on real systems, got {sc}: {ev}"


# ---------------------------------------------------------------------------
# 主函数
# ---------------------------------------------------------------------------

def test_measure_time_grounding_returns_report():
    rep = measure_time_grounding()
    assert isinstance(rep, TimeGroundingReport)
    assert 0.0 <= rep.total <= 1.0
    assert len(rep.sub_dim_scores) == 5
    for name in V1154_DIM_NAMES:
        assert name in rep.sub_dim_scores
        assert 0.0 <= rep.sub_dim_scores[name] <= 1.0


def test_measure_time_grounding_total_is_mean():
    rep = measure_time_grounding()
    expected = sum(rep.sub_dim_scores.values()) / len(rep.sub_dim_scores)
    assert rep.total == pytest.approx(round(expected, 4), abs=1e-4)


def test_measure_time_grounding_records_elapsed():
    rep = measure_time_grounding()
    # 5 sub-dim × ~0.02-0.05s sleep = ~0.15s 起步
    assert rep.elapsed_seconds > 0.0
    assert rep.timestamp > 1.7e9  # recent epoch


def test_measure_time_grounding_writes_artifact(tmp_path: Path):
    rep = measure_time_grounding(artifact_dir=tmp_path)
    out = tmp_path / "v1154_time_grounding.json"
    assert out.exists()
    payload = json.loads(out.read_text())
    assert payload["version"] == V1154_VERSION
    assert 0.0 <= payload["total"] <= 1.0
    assert len(payload["sub_dim_scores"]) == 5
    assert payload["timestamp"] > 1.7e9
    assert "elapsed_seconds" in payload
    assert rep.artifact_path == str(out)


def test_measure_time_grounding_evidence_has_no_silent_none():
    """主 17:43 不假装: 任何 sub-dim evidence 必含真值, 不接受全 None."""
    rep = measure_time_grounding()
    assert len(rep.sub_dim_evidence) == 5
    for name, ev in rep.sub_dim_evidence.items():
        assert "checks" in ev
        assert len(ev["checks"]) > 0, f"{name} has 0 checks — pretend!"
        # 至少有一个 check 真值 (True/False), 不能全是空 dict
        assert any(isinstance(v, bool) for v in ev["checks"].values()), (
            f"{name} 没有真 bool check 值"
        )


def test_measure_time_grounding_no_default_score():
    """主 17:43 不假装: 不允许默认 0.5 掩盖失败 (每 sub-dim 必从实测产生)."""
    rep = measure_time_grounding()
    # 每个 sub-dim 必至少有一项真值 (assertion 而非猜)
    for name in V1154_DIM_NAMES:
        sc = rep.sub_dim_scores[name]
        ev = rep.sub_dim_evidence[name]
        # true measured: 必有真 check 触发
        any_pass = any(v is True for v in ev["checks"].values())
        if sc > 0.0:
            assert any_pass, f"{name} score {sc} 但无任何 check pass — 假装"


def test_measure_time_grounding_artifact_failure_does_not_break():
    """不可写目录不应崩溃主测 (主 00:44 质量工程化: 不让 IO 阻断主流程).

    Windows 上空路径也能 mkdir, 故使用 pathlib.PureWindowsPath 注入 NUL 字符
    (NTFS 上永远不可用) 来强制 IO 失败; POSIX 上 '/nonexistent/...' 即可。
    """
    import sys
    if sys.platform == "win32":
        bad_path = "C:\\nonexistent\\__v1154__\\artifacts\\v1154_time_grounding.json\x00bad"
    else:
        bad_path = "/nonexistent/readonly/__v1154__/v1154_time_grounding.json"
    rep = measure_time_grounding(artifact_dir=bad_path)
    # 不抛异常 = 通过; total 必仍合法
    assert 0.0 <= rep.total <= 1.0
    # 5 sub-dim 必仍有值 (artifact 失败不该影响测量)
    assert len(rep.sub_dim_scores) == 5


def test_measure_time_grounding_min_threshold_quality_gate():
    """主 00:44 质量工程化: 真实系统 5 sub-dim 都应能跑, total 应 ≥ 0.85."""
    rep = measure_time_grounding()
    assert rep.total >= 0.85, (
        f"真实系统 ASI 时间哲学 grounding 应 ≥ 0.85, got {rep.total}: "
        f"{rep.sub_dim_scores}"
    )


# ---------------------------------------------------------------------------
# 不变性 / 复现性
# ---------------------------------------------------------------------------

def test_time_grounding_is_reproducible_within_run():
    """同一进程多次跑, 结果应严格相同 (无随机性)."""
    r1 = measure_time_grounding()
    r2 = measure_time_grounding()
    assert r1.sub_dim_scores == r2.sub_dim_scores
    assert r1.total == r2.total


def test_time_grounding_version_is_stable():
    """API 锁定: dataclass 字段 + version 不应乱变."""
    rep = measure_time_grounding()
    expected_fields = {
        "total", "sub_dim_scores", "sub_dim_evidence", "notes",
        "artifact_path", "elapsed_seconds", "timestamp", "version",
    }
    assert set(rep.to_dict().keys()) == expected_fields


def test_time_grounding_has_no_floating_drift_between_calls():
    """连续两次测, wall_clock 项绝不应混淆 (T1 始终满分)."""
    a, _ = _measure_wall_clock_grounding()
    b, _ = _measure_wall_clock_grounding()
    # T1 在真系统严格一致
    assert a == pytest.approx(b, abs=1e-9)


# ---------------------------------------------------------------------------
# 哲学守门 (主 17:58 不假装)
# ---------------------------------------------------------------------------

def test_v1154_has_no_pretend_score():
    """报告字段 max 总分 = 1.0 严格不超, 含 ship 哲学守门."""
    rep = measure_time_grounding()
    assert rep.total <= 1.0
    for k, v in rep.sub_dim_scores.items():
        assert 0.0 <= v <= 1.0, f"{k} out of [0,1]: {v}"


def test_v1154_reports_are_serializable():
    """报告可 JSON 序列化 (主 22:33 any agent can接手)."""
    rep = measure_time_grounding()
    payload = json.dumps(rep.to_dict(), default=str)
    assert json.loads(payload)["version"] == V1154_VERSION


def test_v1154_does_not_use_random():
    """不假装确定性: 不引随机种子, 必真可复现."""
    import apeireth.v1154_asi_time_philosophy_real_measure as mod
    src = Path(mod.__file__).read_text(encoding="utf-8")
    forbidden = ("random.", "numpy.random", "np.random")
    for tok in forbidden:
        assert tok not in src, f"V1154 不应依赖 {tok}"


# ---------------------------------------------------------------------------
# 真测子模块跨调用单调性 (sanity)
# ---------------------------------------------------------------------------

def test_wall_clock_advances_between_calls():
    t0 = time.time()
    time.sleep(0.02)
    t1 = time.time()
    assert t1 > t0


def test_monotonic_strict_increasing_under_sleep():
    samples = []
    for _ in range(3):
        samples.append(time.monotonic())
        time.sleep(0.01)
    assert samples[0] < samples[1] < samples[2]
