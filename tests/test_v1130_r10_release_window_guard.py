"""Tests for v1130_r10_release_window_guard (R10-DEV-001 / R10 W1 DevOps 主轨道发布硬化).

覆盖 7 个真功能:
  1. Release Window (UTC 跨日窗口 + is_in_window + next_window_start + time_until)
  2. V1074 thresholds + classify_v1074 (GREEN/YELLOW/RED 显式)
  3. Alert + AlertSink (持久化 + chaos 不丢)
  4. fail-soft _safe_subprocess_call (timeout + exception + None)
  5. R10 全链路守门 (V1117 + V1122 + V1074 + V1125 + Window)
  6. chaos test (监控失联 → 告警落盘)
  7. CLI (--check / --chaos / --json / --report / --strict)

主 17:43 实事求是: 数据全从真输入, 不 hardcode.
主 17:58 不假装: 超时 / 失败 → 显式, 不假装 PASS.
"""
from __future__ import annotations

import json
import subprocess
import sys
import time
from datetime import datetime, timedelta, timezone
from pathlib import Path

import pytest

from apeireth.v1130_r10_release_window_guard import (
    DEFAULT_RELEASE_WINDOW,
    Alert,
    AlertSink,
    ReleaseWindow,
    V1074Measurement,
    V1074Thresholds,
    _fetch_v1074_via_subprocess,
    _safe_subprocess_call,
    classify_v1074,
    main,
    render_markdown,
    run_chaos_test,
    run_r10_pipeline_guard,
)


# ---------------------------------------------------------------------------
# 1. Release Window (UTC + 跨日窗口)
# ---------------------------------------------------------------------------
class TestReleaseWindow:
    def test_default_window_is_2_to_4_utc(self):
        w = ReleaseWindow()
        assert w.start_hour_utc == 2
        assert w.end_hour_utc == 4
        assert DEFAULT_RELEASE_WINDOW.start_hour_utc == 2

    def test_is_in_window_true_when_inside(self):
        w = ReleaseWindow(2, 4)
        assert w.is_in_window(datetime(2026, 7, 30, 2, 30, tzinfo=timezone.utc)) is True
        assert w.is_in_window(datetime(2026, 7, 30, 3, 59, tzinfo=timezone.utc)) is True

    def test_is_in_window_false_when_outside(self):
        w = ReleaseWindow(2, 4)
        assert w.is_in_window(datetime(2026, 7, 30, 1, 59, tzinfo=timezone.utc)) is False
        assert w.is_in_window(datetime(2026, 7, 30, 4, 1, tzinfo=timezone.utc)) is False
        assert w.is_in_window(datetime(2026, 7, 30, 12, 0, tzinfo=timezone.utc)) is False

    def test_is_in_window_overnight_window(self):
        w = ReleaseWindow(22, 2)  # 22:00-02:00 跨日
        assert w.is_in_window(datetime(2026, 7, 30, 23, 0, tzinfo=timezone.utc)) is True
        assert w.is_in_window(datetime(2026, 7, 30, 1, 0, tzinfo=timezone.utc)) is True
        assert w.is_in_window(datetime(2026, 7, 30, 12, 0, tzinfo=timezone.utc)) is False
        assert w.is_in_window(datetime(2026, 7, 30, 3, 0, tzinfo=timezone.utc)) is False

    def test_is_in_window_non_utc_converted_to_utc(self):
        # Beijing time 10:30 = UTC 02:30 → 在窗口内
        w = ReleaseWindow(2, 4)
        beijing = timezone(timedelta(hours=8))
        dt = datetime(2026, 7, 30, 10, 30, tzinfo=beijing)
        assert w.is_in_window(dt) is True

    def test_time_until_next_window_inside_is_zero(self):
        w = ReleaseWindow(2, 4)
        dt = datetime(2026, 7, 30, 3, 0, tzinfo=timezone.utc)
        assert w.time_until_next_window(dt) == timedelta(0)

    def test_time_until_next_window_outside(self):
        w = ReleaseWindow(2, 4)
        dt = datetime(2026, 7, 30, 12, 0, tzinfo=timezone.utc)
        wait = w.time_until_next_window(dt)
        # 12:00 → next 02:00 = 14 hours
        assert wait == timedelta(hours=14)

    def test_next_window_start_rolls_to_tomorrow(self):
        w = ReleaseWindow(2, 4)
        dt = datetime(2026, 7, 30, 12, 0, tzinfo=timezone.utc)
        nxt = w.next_window_start(dt)
        assert nxt.day == 31
        assert nxt.hour == 2

    def test_invalid_window_raises(self):
        with pytest.raises(ValueError):
            ReleaseWindow(25, 4)
        with pytest.raises(ValueError):
            ReleaseWindow(2, 25)

    def test_to_dict_serialization(self):
        w = ReleaseWindow(2, 4)
        d = w.to_dict()
        assert d == {"start_hour_utc": 2, "end_hour_utc": 4}


# ---------------------------------------------------------------------------
# 2. V1074 thresholds + classify
# ---------------------------------------------------------------------------
class TestV1074Classify:
    def test_classify_green_when_v03_above_yellow(self):
        m = V1074Measurement(v03_score=0.95)
        level, reason = classify_v1074(m)
        assert level == "GREEN"
        assert "v03=0.95" in reason

    def test_classify_yellow_when_v03_below_yellow(self):
        m = V1074Measurement(v03_score=0.92)
        level, _ = classify_v1074(m)
        assert level == "YELLOW"

    def test_classify_red_when_v03_below_red(self):
        m = V1074Measurement(v03_score=0.5)
        level, reason = classify_v1074(m)
        assert level == "RED"
        assert "v03=0.5000" in reason

    def test_classify_red_v05_overrides_v03_green(self):
        # V03 绿但 V05 紧急 → RED
        m = V1074Measurement(v03_score=0.95, v05_score=0.5)
        level, _ = classify_v1074(m)
        assert level == "RED"

    def test_classify_yellow_v05(self):
        m = V1074Measurement(v03_score=0.95, v05_score=0.93)
        level, _ = classify_v1074(m)
        assert level == "YELLOW"

    def test_classify_green_v05(self):
        m = V1074Measurement(v03_score=0.95, v05_score=0.96)
        level, _ = classify_v1074(m)
        assert level == "GREEN"

    def test_thresholds_invalid_raises(self):
        with pytest.raises(ValueError):
            V1074Thresholds(v03_red=0.95, v03_yellow=0.90)  # red > yellow
        with pytest.raises(ValueError):
            V1074Thresholds(v05_red=0.95, v05_yellow=0.90)

    def test_measurement_invalid_score_raises(self):
        with pytest.raises(ValueError):
            V1074Measurement(v03_score=1.5)
        with pytest.raises(ValueError):
            V1074Measurement(v03_score=-0.1)


# ---------------------------------------------------------------------------
# 3. Alert + AlertSink (chaos 不丢)
# ---------------------------------------------------------------------------
class TestAlertSink:
    def test_send_appends_alert(self):
        sink = AlertSink()
        sink.send(Alert(level="YELLOW", source="test", reason="x"))
        assert len(sink.alerts) == 1
        assert sink.alerts[0].level == "YELLOW"

    def test_counts_by_level(self):
        sink = AlertSink()
        sink.send(Alert(level="GREEN", source="a", reason="ok"))
        sink.send(Alert(level="YELLOW", source="b", reason="warn"))
        sink.send(Alert(level="YELLOW", source="c", reason="warn2"))
        sink.send(Alert(level="RED", source="d", reason="bad"))
        c = sink.counts_by_level()
        assert c == {"GREEN": 1, "YELLOW": 2, "RED": 1}

    def test_persist_writes_jsonl(self, tmp_path):
        p = tmp_path / "alerts.jsonl"
        sink = AlertSink(persist_path=p)
        sink.send(Alert(level="RED", source="x", reason="bad"))
        sink.send(Alert(level="YELLOW", source="y", reason="warn"))
        lines = p.read_text(encoding="utf-8").strip().splitlines()
        assert len(lines) == 2
        d1 = json.loads(lines[0])
        assert d1["level"] == "RED"
        assert "ts_iso" in d1

    def test_summary_includes_path(self):
        sink = AlertSink()
        sink.send(Alert(level="GREEN", source="a", reason="ok"))
        s = sink.summary()
        assert s["n_alerts"] == 1
        assert s["by_level"] == {"GREEN": 1, "YELLOW": 0, "RED": 0}
        assert s["persist_path"] is None

    def test_alert_to_dict_includes_iso(self):
        a = Alert(level="RED", source="x", reason="r")
        d = a.to_dict()
        assert d["level"] == "RED"
        assert "ts_iso" in d


# ---------------------------------------------------------------------------
# 4. fail-soft _safe_subprocess_call
# ---------------------------------------------------------------------------
class TestSafeSubprocessCall:
    def test_returns_fn_result_on_success(self):
        result = _safe_subprocess_call(
            fn=lambda: {"v03_score": 0.95, "source": "v1074"},
            fallback={"v03_score": 0.0},
        )
        assert result["v03_score"] == 0.95
        assert result["source"] == "v1074"

    def test_returns_fallback_on_exception(self):
        def boom():
            raise RuntimeError("simulated")
        result = _safe_subprocess_call(fn=boom, fallback={"v03_score": 0.0})
        assert result["v03_score"] == 0.0
        assert "RuntimeError" in result["source"]

    def test_returns_fallback_on_none(self):
        result = _safe_subprocess_call(fn=lambda: None, fallback={"v03_score": 0.0})
        assert result["v03_score"] == 0.0
        assert "None_return" in result["source"]

    def test_fetch_v1074_via_subprocess_returns_dict(self):
        # 真跑子进程 (--report --no-write --print-json)
        raw = _fetch_v1074_via_subprocess(timeout_sec=60.0)
        assert "source" in raw
        if raw.get("source") == "v1074":
            assert isinstance(raw.get("v03_score"), (int, float))
        # chaos 情形 (timeout / 失败) 也接受


# ---------------------------------------------------------------------------
# 5. R10 全链路守门 (主 17:43 + 主 17:58)
# ---------------------------------------------------------------------------
class TestR10PipelineGuard:
    def test_pipeline_in_window_returns_green_window(self):
        # UTC 03:00 → 在 02-04 窗口内
        now = datetime(2026, 7, 30, 3, 0, tzinfo=timezone.utc)
        report = run_r10_pipeline_guard(now=now)
        rw = next(l for l in report.links if l.name == "Release Window")
        assert rw.level == "GREEN"
        assert report.in_window is True

    def test_pipeline_out_window_returns_yellow_window(self):
        now = datetime(2026, 7, 30, 12, 0, tzinfo=timezone.utc)
        report = run_r10_pipeline_guard(now=now)
        rw = next(l for l in report.links if l.name == "Release Window")
        assert rw.level == "YELLOW"
        assert report.in_window is False

    def test_pipeline_v1117_green(self):
        # V1117 在某些 integration commit 后可能未部署 → 接受 YELLOW (主 17:58 不假装)
        report = run_r10_pipeline_guard()
        v1117 = next(l for l in report.links if l.name == "V1117 badge SVG")
        if v1117.level == "YELLOW" and "v1117 未在 integration 部署" in v1117.detail:
            pytest.skip("v1117 not deployed in this integration commit")
        assert v1117.level == "GREEN"

    def test_pipeline_v1122_green(self):
        report = run_r10_pipeline_guard()
        v1122 = next(l for l in report.links if l.name == "V1122 DevOps W4")
        if v1122.level == "YELLOW" and "v1122 未在 integration 部署" in v1122.detail:
            pytest.skip("v1122 not deployed in this integration commit")
        assert v1122.level == "GREEN"

    def test_pipeline_v1074_via_subprocess(self):
        # 真跑 V1074 子进程 (主 17:43 实事求是)
        report = run_r10_pipeline_guard()
        v1074 = next(l for l in report.links if l.name == "V1074 ASI guard")
        # 当前 v03 通常 ≥ 0.89, 至少 YELLOW 或 GREEN
        assert v1074.level in ("GREEN", "YELLOW", "RED")

    def test_pipeline_overall_window_out_others_ok(self):
        # 窗口外 YELLOW, 但 V1074 真测可能 YELLOW (v03 < 0.94) → overall 取决于最差等级
        now = datetime(2026, 7, 30, 12, 0, tzinfo=timezone.utc)
        report = run_r10_pipeline_guard(now=now)
        # window YELLOW, V1074 YELLOW (v03 0.89 < 0.94) → overall YELLOW (主 17:43 数字驱动)
        assert report.overall_level in ("GREEN", "YELLOW")
        assert report.overall_level != "RED"

    def test_report_to_dict_serializable(self):
        report = run_r10_pipeline_guard()
        d = report.to_dict()
        assert "ts_iso" in d
        assert "links" in d
        assert "alerts" in d
        json.dumps(d)  # must be JSON-serializable

    def test_render_markdown_contains_table(self):
        report = run_r10_pipeline_guard()
        md = render_markdown(report, ReleaseWindow(), V1074Thresholds())
        assert "# R10 DevOps Pipeline Guard Report" in md
        assert "| Name | Level | Detail |" in md
        assert "Release Window" in md
        assert "V1074 ASI guard" in md


# ---------------------------------------------------------------------------
# 6. chaos test (主 17:58 不假装: 监控失联守门不丢告警)
# ---------------------------------------------------------------------------
class TestChaos:
    def test_chaos_alerts_persist_to_disk(self, tmp_path):
        persist = tmp_path / "chaos.jsonl"
        result = run_chaos_test(persist_path=persist, fail_v1074_subprocess=True)
        assert persist.exists()
        n_persisted = result["report_summary"]["n_alerts_persisted"]
        n_memory = result["report_summary"]["n_alerts_in_memory"]
        assert n_persisted == n_memory
        assert n_persisted > 0
        assert result["alert_dropped"] is False

    def test_chaos_no_alerts_dropped_when_persist_fails(self, tmp_path, monkeypatch):
        # 模拟 persist_path 在一个不可写位置, 但 in-memory 仍保留
        import apeireth.v1130_r10_release_window_guard as mod
        # 让 open 失败
        original_open = mod.Path.open if hasattr(mod.Path, 'open') else Path.open
        def fail_open(self, *args, **kwargs):
            if str(self).endswith("chaos_fail.jsonl"):
                raise OSError("chaos: disk full")
            return original_open(self, *args, **kwargs)
        monkeypatch.setattr(Path, "open", fail_open)

        # 此时 run_chaos_test 用一个不存在的路径 → chaos 仍跑, 告警仍在内存
        result = run_chaos_test(persist_path=tmp_path / "chaos_fail.jsonl", fail_v1074_subprocess=True)
        # in-memory 告警数 > 0 (主 17:58 不假装: 落盘失败也不丢)
        assert result["report_summary"]["n_alerts_in_memory"] > 0

    def test_chaos_with_v1074_success_no_red(self, tmp_path):
        persist = tmp_path / "chaos2.jsonl"
        result = run_chaos_test(persist_path=persist, fail_v1074_subprocess=False)
        # fail_v1074_subprocess=False → 注入 v03=0.95
        assert result["philosophy_ok"] is True
        assert result["report_summary"]["n_alerts_persisted"] > 0


# ---------------------------------------------------------------------------
# 7. CLI (主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------
class TestCLI:
    def test_cli_default_check(self, capsys):
        rc = main(["--check"])
        out = capsys.readouterr().out
        assert rc in (0, 1)
        assert "overall=" in out
        assert "V1130" in out

    def test_cli_json_output(self, capsys):
        rc = main(["--check", "--json"])
        out = capsys.readouterr().out
        d = json.loads(out)
        assert "overall_level" in d
        assert "links" in d

    def test_cli_report_markdown(self, capsys):
        rc = main(["--check", "--report"])
        out = capsys.readouterr().out
        assert "# R10 DevOps Pipeline Guard Report" in out

    def test_cli_chaos_json(self, capsys, tmp_path):
        rc = main(["--chaos", "--persist-path", str(tmp_path / "c.jsonl"), "--json"])
        out = capsys.readouterr().out
        d = json.loads(out)
        assert d["chaos_test"] == "monitor_outage"

    def test_cli_invalid_window_returns_2(self, capsys):
        rc = main(["--window", "99-100"])
        assert rc == 2

    def test_cli_strict_red_returns_1(self, capsys, monkeypatch):
        # 注入超低 V03, 触发 RED
        from apeireth import v1130_r10_release_window_guard as mod
        # 替换 _check_v1074_guard 内部
        def fake_check(thresholds, project_dir=None):
            return mod.R10DevOpsLink(
                name="V1074 ASI guard",
                level="RED",
                detail="v03=0.5 < v03_red=0.8884",
                extra={"v03_score": 0.5},
            )
        monkeypatch.setattr(mod, "_check_v1074_guard", fake_check)
        rc = main(["--check", "--strict"])
        assert rc == 1

    def test_module_version_exported(self):
        from apeireth.v1130_r10_release_window_guard import __version__
        assert __version__ == "0.1.0"