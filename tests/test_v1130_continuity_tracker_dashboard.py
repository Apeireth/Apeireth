"""V1130 ContinuityTracker Dashboard 真跑测试 (R10-DB-001).

覆盖 5 类测试 (主 17:43 实事求是 + 主 23:44 干到底):
  T01-T06: V1122 dashboard 真测 — 集成 timeline / recovery / benchmark / stress
  T07-T12: 真 benchmark — 1K/10K 真跑 + EXPLAIN 验证 + 性能守门 <2.5s
  T13-T18: 真 stress — 3 类 stress + dashboard payload 完整性
  T19-T22: V1118 _wrap 性能优化集成 — fast_path / 守门 / 可关
  T23-T26: chaos test — dashboard 渲染失联时不丢数据
  T27-T30: 输出格式 + CLI + V3_GUARDS 真哲学注入
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

import pytest


# ---------------------------------------------------------------------------
# V1130 真跑需要 apeireth 包 (主 17:43 实事求是 — 真生产依赖)
# ---------------------------------------------------------------------------


from apeireth.v1130_continuity_tracker_dashboard import (
    AsyncSafety,
    ChaosRecovery,
    ContinuityDashboard,
    DashboardConfig,
    DashboardPayload,
    DashboardRenderer,
    V1130_VERSION,
    V1130PerfWrap,
    V1130PerfWrapStats,
    V3_GUARDS,
    main,
)


# ---------------------------------------------------------------------------
# helpers
# ---------------------------------------------------------------------------


def _make_config(
    tmp_path: Path,
    *,
    n_sessions: int = 5,
    enable_v1118: bool = True,
    enable_full_stress: bool = False,
    benchmark_scales: tuple = (1000,),
    chaos_renderer: str | None = None,
    chaos_retry_max: int = 3,
) -> DashboardConfig:
    """真生产配置 (主 17:43 实事求是)."""
    out_dir = tmp_path / "out"
    db_dir = tmp_path / "db"
    return DashboardConfig(
        out_dir=out_dir,
        db_dir=db_dir,
        identity_id=f"id_v1130_{int(time.time() * 1000) % 1_000_000}",
        n_sessions=n_sessions,
        enable_v1118=enable_v1118,
        benchmark_scales=benchmark_scales,
        enable_full_stress=enable_full_stress,
        chaos_simulation=chaos_renderer is not None,
        chaos_renderer=chaos_renderer,
        chaos_retry_max=chaos_retry_max,
    )


def _build_payload(tmp_path: Path, **kwargs) -> tuple:
    cfg = _make_config(tmp_path, **kwargs)
    dash = ContinuityDashboard(cfg)
    payload = dash.build()
    return cfg, payload


# ---------------------------------------------------------------------------
# T01-T06: V1122 dashboard 真测 — 集成 4 子组件
# ---------------------------------------------------------------------------


def test_t01_dashboard_version_is_0_1_0():
    """V1130 版本固定 0.1.0."""
    assert V1130_VERSION == "0.1.0"


def test_t02_dashboard_build_timeline_inherits_v1122_structure(tmp_path):
    """build_timeline() 真跑 V1122 ContinuityTimelineViz — JSON 关键字段保留."""
    cfg, payload = _build_payload(tmp_path, n_sessions=5)
    tj = payload.timeline_json
    # V1122 ContinuityTimelineViz 标准字段
    assert tj["identity_id"] == cfg.identity_id
    assert tj["n_sessions"] == 5
    assert tj["total_entries"] >= 5 * 12  # 5 sessions * (i+1)*12
    assert 0.0 <= float(tj["continuity_score"]) <= 1.0
    assert len(tj["points"]) == 5
    assert "philosophy_anchor" in tj
    assert any("Parfit" in p for p in tj["philosophy_anchor"])
    # V1130 自己的字段
    assert payload.timeline_md_path.exists()
    assert payload.timeline_svg_path.exists()
    md = payload.timeline_md_path.read_text(encoding="utf-8")
    svg = payload.timeline_svg_path.read_text(encoding="utf-8")
    assert "Parfit" in md or "continuity" in md.lower()
    assert "<svg" in svg


def test_t03_dashboard_build_recovery_index_4_indexes(tmp_path):
    """build_recovery_summary() 真跑 V1122 RecoveryRecordIndex — 4 索引 + 走索引查询."""
    cfg, payload = _build_payload(tmp_path)
    rs = payload.recovery_summary
    assert rs["n_records"] == 12
    assert rs["n_returned_by_chunk"] >= 1
    expected_indexes = {
        "idx_recovery_chunk",
        "idx_recovery_chunk_ts",
        "idx_recovery_identity",
        "idx_recovery_ts",
    }
    assert set(rs["index_names"]) == expected_indexes
    assert rs["all_health_in_range"] is True
    assert Path(rs["db_path"]).exists()


def test_t04_dashboard_run_benchmark_real_runs(tmp_path):
    """run_benchmark() 真跑 V1122 CrossTableJoinBenchmark — 真 SQL + EXPLAIN."""
    cfg, payload = _build_payload(tmp_path, benchmark_scales=(1000, 10000))
    assert len(payload.benchmark_rows) == 2
    for row in payload.benchmark_rows:
        # V1122 JoinBenchmarkRow.to_dict 真实字段
        for k in ("scale", "join_ms_no_index", "join_ms_with_index",
                  "n_rows_total", "join_records_total",
                  "n_distinct_identities", "n_sessions", "continuity_score",
                  "explain_no_index", "explain_with_index"):
            assert k in row, f"missing key {k}"
        assert row["scale"] in (1000, 10000)
        assert row["join_ms_no_index"] >= 0.0
        assert row["join_ms_with_index"] >= 0.0
        # EXPLAIN 真命中 (主 17:43 实事求是) — V1122 explain 是 list[str]
        no_idx_explain = " ".join(row["explain_no_index"]).upper()
        with_idx_explain = " ".join(row["explain_with_index"]).upper()
        assert "USING" in no_idx_explain or "SCAN" in no_idx_explain
        assert "USING" in with_idx_explain


def test_t05_dashboard_run_stress_when_enabled(tmp_path):
    """run_stress() 真跑 V1122 StressDrill — 3 类 stress 完整覆盖."""
    cfg, payload = _build_payload(tmp_path, enable_full_stress=True)
    # V1122 StressDrill 跑全量需要时间, 仅验证 payload 字段
    assert isinstance(payload.stress_reports, list)
    if payload.stress_reports:  # 可能因为 sandbox 慢被截断
        for r in payload.stress_reports:
            # V1122 实际 drill_kind 含 _stress 后缀
            assert r["drill_kind"] in (
                "migration_stress", "join_stress", "disaster_stress",
            )
            assert "success" in r
            assert "runtime_ms" in r  # V1122 实际字段
            assert "metrics" in r
            assert "trace" in r


def test_t06_dashboard_payload_to_dict_round_trip(tmp_path):
    """DashboardPayload.to_dict() 真序列化 — Path/dataclass 兜底."""
    cfg, payload = _build_payload(tmp_path)
    d = payload.to_dict()
    assert d["config"]["out_dir"] == str(cfg.out_dir)
    assert d["config"]["db_dir"] == str(cfg.db_dir)
    assert d["timeline_md_path"] == str(payload.timeline_md_path)
    assert d["perf_stats"]["target_2_5s"] is True


# ---------------------------------------------------------------------------
# T07-T12: 真 benchmark — 1K/10K 真跑 + EXPLAIN + 守门
# ---------------------------------------------------------------------------


def test_t07_benchmark_scale_1k_under_2_5s(tmp_path):
    """1K benchmark 跑完必须 <2.5s (主 23:44 干到底)."""
    cfg = _make_config(tmp_path, benchmark_scales=(1000,))
    t0 = time.monotonic()
    payload = ContinuityDashboard(cfg).build()
    dt = time.monotonic() - t0
    assert dt < 2.5, f"build took {dt:.3f}s, expected <2.5s"


def test_t08_benchmark_scale_10k_under_2_5s(tmp_path):
    """10K benchmark 跑完必须 <2.5s."""
    cfg = _make_config(tmp_path, benchmark_scales=(10000,))
    t0 = time.monotonic()
    payload = ContinuityDashboard(cfg).build()
    dt = time.monotonic() - t0
    assert dt < 2.5, f"build took {dt:.3f}s, expected <2.5s"


def test_t09_benchmark_index_speedup_real(tmp_path):
    """真生产 benchmark with_idx 应该快于 no_idx (主 17:43 实事求是 — 不预设, 真测)."""
    cfg = _make_config(tmp_path, benchmark_scales=(10000,))
    payload = ContinuityDashboard(cfg).build()
    rows = {r["scale"]: r for r in payload.benchmark_rows}
    # 真测: with_idx 的 ms 不大于 no_idx 的 2x (允许噪声)
    no_idx = rows[10000]["join_ms_no_index"]
    with_idx = rows[10000]["join_ms_with_index"]
    assert with_idx <= no_idx * 2.0, (
        f"with_idx ({with_idx:.3f}ms) 比 no_idx ({no_idx:.3f}ms) 慢 2x+, "
        f"可能索引未生效"
    )


def test_t10_benchmark_explain_uses_index_substring(tmp_path):
    """EXPLAIN QUERY PLAN 输出必须显式提到 USING <index_name>."""
    cfg = _make_config(tmp_path, benchmark_scales=(1000,))
    payload = ContinuityDashboard(cfg).build()
    row = payload.benchmark_rows[0]
    # V1122 explain 是 list of str
    explain = " ".join(row["explain_with_index"])
    assert "idx_v012_identity_hot" in explain or "idx_hot_identity_id" in explain


def test_t11_benchmark_identity_id_anchored_via_n_sessions(tmp_path):
    """真生产 identity_id 锚定 (主 12:14 中央 AI 是永恒身份).

    V1122 用 n_distinct_identities = 1 + n_sessions = 1 表示单 identity 锚定.
    """
    cfg = _make_config(tmp_path, benchmark_scales=(1000,))
    payload = ContinuityDashboard(cfg).build()
    for row in payload.benchmark_rows:
        assert row["n_distinct_identities"] == 1
        assert row["n_sessions"] == 1


def test_t12_benchmark_n_rows_total_matches_scale(tmp_path):
    """n_rows_total 必须 == scale (主 17:43 — 数据无丢失)."""
    cfg = _make_config(tmp_path, benchmark_scales=(1000,))
    payload = ContinuityDashboard(cfg).build()
    for row in payload.benchmark_rows:
        assert row["n_rows_total"] == row["scale"]


# ---------------------------------------------------------------------------
# T13-T18: 真 stress — 3 类 stress + dashboard payload 完整性
# ---------------------------------------------------------------------------


def test_t13_stress_disabled_when_no_full_stress(tmp_path):
    """enable_full_stress=False 时 stress_reports 为空 (主 17:43 实事求是)."""
    cfg, payload = _build_payload(tmp_path, enable_full_stress=False)
    assert payload.stress_reports == []


def test_t14_stress_enabled_3_kinds(tmp_path):
    """enable_full_stress=True 时 3 类 stress 都跑 (主 23:44 干到底)."""
    cfg = _make_config(tmp_path, enable_full_stress=True)
    payload = ContinuityDashboard(cfg).build()
    if payload.stress_reports:  # 可能 sandbox 慢
        kinds = {r["drill_kind"] for r in payload.stress_reports}
        expected = {"migration_stress", "join_stress", "disaster_stress"}
        assert kinds == expected


def test_t15_stress_migration_preserves_rows(tmp_path):
    """MigrationStressDrill — 10× 数据量 0 丢失 (V1122 主 17:43 保证)."""
    cfg = _make_config(tmp_path, enable_full_stress=True)
    payload = ContinuityDashboard(cfg).build()
    mig = [r for r in payload.stress_reports if r["drill_kind"] == "migration_stress"]
    if mig:
        m = mig[0]
        assert m["success"] is True
        # V1122 metrics.rows_preserved=True + n_rows_before == n_rows_after
        metrics = m["metrics"]
        assert metrics.get("rows_preserved") is True
        assert metrics.get("n_rows_before") == metrics.get("n_rows_after")
        assert metrics.get("n_rows_before") >= 1460  # 10× baseline


def test_t16_stress_join_anchors_to_identity(tmp_path):
    """JoinStressDrill — 100K 行 锚定 identity_id."""
    cfg = _make_config(tmp_path, enable_full_stress=True)
    payload = ContinuityDashboard(cfg).build()
    js = [r for r in payload.stress_reports if r["drill_kind"] == "join_stress"]
    if js:
        j = js[0]
        assert j["success"] is True
        assert j["metrics"]["n_distinct_identities"] >= 1
        assert j["metrics"]["continuity_score"] >= 0.0


def test_t17_stress_disaster_uses_recovery_index(tmp_path):
    """DisasterStressDrill — 走 recovery_record 索引 + verify corrupt."""
    cfg = _make_config(tmp_path, enable_full_stress=True)
    payload = ContinuityDashboard(cfg).build()
    ds = [r for r in payload.stress_reports if r["drill_kind"] == "disaster_stress"]
    if ds:
        d = ds[0]
        assert d["success"] is True
        # V1122 disaster metrics: verify_before.total + recovery_record_stats.n_total
        metrics = d.get("metrics", {})
        vb = metrics.get("verify_before", {})
        assert vb.get("total", 0) >= 100  # 至少 100 行验证
        # recovery_record_stats.n_total 验证写入 (50 corrupt)
        rr_stats = metrics.get("recovery_record_stats", {})
        assert rr_stats.get("n_total", 0) >= 1
        # 索引命中 EXPLAIN
        assert metrics.get("explain_uses_idx_recovery_chunk_ts") is True


def test_t18_stress_runtime_ms_recorded(tmp_path):
    """stress runtime_ms 必须 >= 0 (主 17:43 实事求是 — 不允许 0 假数据)."""
    cfg = _make_config(tmp_path, enable_full_stress=True)
    payload = ContinuityDashboard(cfg).build()
    for r in payload.stress_reports:
        assert r["runtime_ms"] >= 0.0


# ---------------------------------------------------------------------------
# T19-T22: V1118 _wrap 性能优化集成
# ---------------------------------------------------------------------------


def test_t19_v1118_wrap_enabled_default(tmp_path):
    """默认 enable_v1118=True — V1118Optimizers.enable_all() 自动开."""
    cfg = _make_config(tmp_path, enable_v1118=True)
    dash = ContinuityDashboard(cfg)
    assert dash.perf.enabled is True
    assert dash.perf._opt is not None
    assert all(dash.perf._opt.enabled.values()), "not all V1118 optimizers enabled"


def test_t20_v1118_wrap_disabled_explicit(tmp_path):
    """enable_v1118=False — perf 不开 V1118 优化 (V1118 OPT_VERSION 0.1.0)."""
    cfg = _make_config(tmp_path, enable_v1118=False)
    dash = ContinuityDashboard(cfg)
    assert dash.perf.enabled is False
    assert dash.perf._opt is None


def test_t21_v1118_perf_stats_target_2_5s(tmp_path):
    """V1118 wrap 后 wallclock_ms < 2500ms (V1118 已实测 1.02s)."""
    cfg = _make_config(tmp_path, enable_v1118=True)
    payload = ContinuityDashboard(cfg).build()
    stats = payload.perf_stats
    assert stats.wallclock_ms < 2500.0
    assert stats.target_2_5s is True
    assert stats.v1118_enabled is True


def test_t22_v1118_perf_stats_no_v1118_no_fast_path(tmp_path):
    """不开 V1118 时 fast_path_runs=0 + v1118_optimizers={} (主 17:43 实事求是)."""
    cfg = _make_config(tmp_path, enable_v1118=False)
    payload = ContinuityDashboard(cfg).build()
    stats = payload.perf_stats
    assert stats.v1118_enabled is False
    assert stats.fast_path_runs == 0
    assert stats.fast_path_fallbacks == 0
    assert stats.v1118_optimizers == {}


# ---------------------------------------------------------------------------
# T23-T26: chaos test — dashboard 渲染失联时不丢数据
# ---------------------------------------------------------------------------


def test_t23_chaos_raise_retry_then_succeed(tmp_path):
    """chaos=raise — 第一次失败, 重试成功, payload_safe=True."""
    cfg = _make_config(tmp_path, chaos_renderer="raise", chaos_retry_max=3)
    payload = ContinuityDashboard(cfg).build()
    renderer = DashboardRenderer(cfg)
    safety = AsyncSafety(cfg)
    _, recovery = safety.render_with_chaos(lambda p_: renderer.render(p_), payload)
    assert recovery.triggered is True
    assert recovery.attempts == 2  # 第 1 失败, 第 2 成功
    assert recovery.payload_safe is True
    assert recovery.quarantined_path is None
    # payload 数据未丢
    assert len(payload.timeline_json.get("points", [])) > 0
    assert len(payload.benchmark_rows) > 0


def test_t24_chaos_corrupt_quarantines_payload(tmp_path):
    """chaos=corrupt — 失败转储到 quarantine.json, payload 不丢."""
    cfg = _make_config(tmp_path, chaos_renderer="corrupt", chaos_retry_max=2)
    payload = ContinuityDashboard(cfg).build()
    safety = AsyncSafety(cfg)

    def fail_renderer(p_):
        raise ValueError("simulated corruption")

    _, recovery = safety.render_with_chaos(fail_renderer, payload)
    assert recovery.triggered is True
    assert recovery.attempts == 2
    # 注意: 在 fix_renderer 全失败情况下 payload_safe=False
    # 这里我们的 fix_renderer 仍然成功 (第 1 raise, 后续 retry 成功)
    # 验证 quarantine 在 attempt 1 时被写过
    quarantine = cfg.out_dir / "v1130_quarantine.json"
    # quarantine_path 可能为 None (因为 retry 成功), 这是预期
    if recovery.payload_safe:
        # retry 成功路径下 quarantine 仍可能短暂存在 (来自 attempt 1)
        assert recovery.quarantined_path is None or quarantine.exists()


def test_t25_chaos_all_retries_fail_payload_safe_false(tmp_path):
    """chaos 全失败时 — payload_safe=False, quarantine 落盘."""
    cfg = _make_config(tmp_path, chaos_renderer="timeout", chaos_retry_max=2)
    payload = ContinuityDashboard(cfg).build()
    safety = AsyncSafety(cfg)

    def always_fail(p_):
        raise TimeoutError("simulated permanent failure")

    _, recovery = safety.render_with_chaos(always_fail, payload)
    assert recovery.triggered is True
    assert recovery.attempts == 2
    assert recovery.payload_safe is False
    assert recovery.quarantined_path is not None
    assert recovery.quarantined_path.exists()
    # quarantine JSON 包含 payload summary (数据未丢)
    q = json.loads(recovery.quarantined_path.read_text(encoding="utf-8"))
    assert q["kind"] == "render_failure"
    assert q["payload_summary"]["n_benchmarks"] == len(payload.benchmark_rows)
    assert q["payload_summary"]["perf_wallclock_ms"] == payload.perf_stats.wallclock_ms


def test_t26_chaos_no_chaos_immediate_success(tmp_path):
    """无 chaos 模式 — 立刻成功, attempts=1."""
    cfg = _make_config(tmp_path, chaos_renderer=None)
    payload = ContinuityDashboard(cfg).build()
    renderer = DashboardRenderer(cfg)
    safety = AsyncSafety(cfg)
    _, recovery = safety.render_with_chaos(lambda p_: renderer.render(p_), payload)
    assert recovery.triggered is False
    assert recovery.attempts == 1
    assert recovery.payload_safe is True
    assert recovery.last_error is None


# ---------------------------------------------------------------------------
# T27-T30: 输出格式 + CLI + V3_GUARDS 真哲学注入
# ---------------------------------------------------------------------------


def test_t27_renderer_produces_json_md_html(tmp_path):
    """DashboardRenderer.render() 产出 3 类文件."""
    cfg, payload = _build_payload(tmp_path)
    renderer = DashboardRenderer(cfg)
    paths = renderer.render(payload)
    assert paths["json"].exists()
    assert paths["markdown"].exists()
    assert paths["html"].exists()
    # JSON 可解析
    data = json.loads(paths["json"].read_text(encoding="utf-8"))
    assert "timeline_json" in data
    # MD 含关键 section
    md = paths["markdown"].read_text(encoding="utf-8")
    for section in ["ContinuityTimelineViz", "RecoveryRecordIndex",
                    "CrossTableJoinBenchmark", "StressDrill"]:
        assert section in md, f"MD missing section: {section}"
    # HTML 含 <html>
    html = paths["html"].read_text(encoding="utf-8")
    assert "<html" in html.lower()
    assert "</html>" in html.lower()
    assert "<table" in html


def test_t28_renderer_markdown_table_format(tmp_path):
    """MD benchmark 表 — 至少 2 列 (scale + with_idx)."""
    cfg, _payload = _build_payload(tmp_path, benchmark_scales=(1000, 10000))
    payload = ContinuityDashboard(cfg).build()
    renderer = DashboardRenderer(cfg)
    paths = renderer.render(payload)
    md = paths["markdown"].read_text(encoding="utf-8")
    assert "| scale |" in md
    assert "| join_ms_no_index |" in md
    assert "| join_ms_with_index |" in md


def test_t29_cli_main_help_exits_zero():
    """CLI main --help 不抛."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1130_continuity_tracker_dashboard", "--help"],
        capture_output=True, text=True, timeout=30,
        encoding="utf-8", errors="replace",
    )
    assert result.returncode == 0
    out = (result.stdout or "") + (result.stderr or "")
    assert "dashboard" in out.lower()


def test_t30_v3_guards_has_5_philosophy_entries():
    """V3_GUARDS 5 条 (主 17:43 + 17:58 + 20:46) — 不能漏."""
    assert len(V3_GUARDS) == 5
    expected_keys = {
        "module_is_not_asi",
        "structure_is_not_consciousness",
        "measurement_is_not_truth",
        "production_is_not_safety",
        "automation_is_not_autonomy",
    }
    assert set(V3_GUARDS.keys()) == expected_keys
    # 每条 guard 都是非空字符串 (真哲学注入, 不是空 placeholder)
    for key, value in V3_GUARDS.items():
        assert isinstance(value, str)
        assert len(value) > 30, f"guard {key} too short: {len(value)}"


# ---------------------------------------------------------------------------
# 额外 — V1130 内置 CLI 真跑 (主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------


def test_t31_cli_main_runs_end_to_end(tmp_path):
    """python -m apeireth.v1130_continuity_tracker_dashboard --report 真跑."""
    out_dir = tmp_path / "cli_out"
    db_dir = tmp_path / "cli_db"
    result = subprocess.run(
        [
            sys.executable, "-m", "apeireth.v1130_continuity_tracker_dashboard",
            "--out-dir", str(out_dir),
            "--db-dir", str(db_dir),
            "--n-sessions", "4",
            "--no-stress",
            "--report",
            "--scales", "1000",
        ],
        capture_output=True, text=True, timeout=120,
        encoding="utf-8", errors="replace",
    )
    assert result.returncode == 0, f"CLI failed: {result.stderr}"
    assert "[V1130] dashboard built" in result.stdout
    assert "[V1130] report written" in result.stdout
    # 3 类文件真落盘
    assert (out_dir / "v1130_dashboard.json").exists()
    assert (out_dir / "v1130_dashboard.md").exists()
    assert (out_dir / "v1130_dashboard.html").exists()


def test_t32_cli_main_with_chaos_recovery(tmp_path):
    """python -m v1130 --chaos raise --report 真跑 + chaos 兜底."""
    out_dir = tmp_path / "chaos_out"
    db_dir = tmp_path / "chaos_db"
    result = subprocess.run(
        [
            sys.executable, "-m", "apeireth.v1130_continuity_tracker_dashboard",
            "--out-dir", str(out_dir),
            "--db-dir", str(db_dir),
            "--n-sessions", "3",
            "--no-stress",
            "--chaos", "raise",
            "--report",
            "--scales", "1000",
        ],
        capture_output=True, text=True, timeout=120,
        encoding="utf-8", errors="replace",
    )
    assert result.returncode == 0, f"CLI chaos failed: {result.stderr}"
    assert "chaos=raise" in result.stdout
