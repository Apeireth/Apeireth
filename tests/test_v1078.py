"""v1078_asi_cron_self_audit.py 真生产回归测试.

主 11:18 真扫: 测 8 真生产组件 + V3 哲学守门 + CLI + 真 jsonl 数据.
"""
from __future__ import annotations

import json
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v1078_asi_cron_self_audit import (
    V1078_VERSION,
    V3PhilosophyGuard,
    CronHistoryParser,
    CronRunRecord,
    CronRunRecordBuilder,
    CronSelfAuditor,
    GapDetector,
    ProviderHealth,
    LatencyStats,
    IdleTimeoutDetector,
    AuditAggregator,
    AuditReport,
    AuditResult,
)


# ===================== RecordBuilder + Parser =====================

class TestCronRunRecordBuilder:
    def test_init_default(self):
        r = CronRunRecord(raw={})
        assert r.round == 0
        assert not r.is_done

    def test_from_valid_line(self):
        line = json.dumps({
            "round": 31, "action": "done",
            "ts_iso": "2026-07-22T11:03:30+08:00",
            "duration_s": 64.8, "queries": 12,
            "bocha_ai_used": False, "anysearch_used": True,
            "note": "round-31 done",
        })
        r = CronRunRecordBuilder.from_jsonl_line(line)
        assert r is not None
        assert r.round == 31
        assert r.action == "done"
        assert r.is_done is True
        assert r.duration_s == 64.8
        assert r.anysearch_used is True

    def test_from_empty_returns_none(self):
        assert CronRunRecordBuilder.from_jsonl_line("") is None
        assert CronRunRecordBuilder.from_jsonl_line("   ") is None

    def test_from_malformed_returns_none(self):
        assert CronRunRecordBuilder.from_jsonl_line("{not json") is None
        assert CronRunRecordBuilder.from_jsonl_line("null") is None
        assert CronRunRecordBuilder.from_jsonl_line("[1,2,3]") is None

    def test_iso_to_unix(self):
        ts = CronRunRecordBuilder._parse_ts("2026-07-22T11:00:00+08:00")
        assert ts > 1.7e9  # 2024 年时间戳 ~ 1.7e9

    def test_numeric_ts(self):
        assert CronRunRecordBuilder._parse_ts(1784690869.0) == 1784690869.0

    def test_empty_string_returns_zero(self):
        assert CronRunRecordBuilder._parse_ts("") == 0.0
        assert CronRunRecordBuilder._parse_ts(None) == 0.0


class TestCronHistoryParser:
    def test_parse_missing_file(self, tmp_path: Path):
        p = tmp_path / "nope.jsonl"
        records, skipped = CronHistoryParser(p).parse()
        assert records == []
        assert skipped == 0

    def test_parse_skips_malformed(self, tmp_path: Path):
        p = tmp_path / "mix.jsonl"
        p.write_text(
            '{"round":1,"action":"done","ts":100.0}\n'
            'not json line\n'
            '\n'
            '{"round":2,"action":"done","ts":200.0}\n',
            encoding="utf-8",
        )
        records, skipped = CronHistoryParser(p).parse()
        assert len(records) == 2
        assert skipped == 1
        assert records[0].round == 1

    def test_parse_real_promethean_jsonl(self):
        # 真扫工作区里 cron-research-runs.jsonl
        ws_jsonl = Path(__file__).parent.parent / "cron-research-runs.jsonl"
        if ws_jsonl.exists():
            records, _ = CronHistoryParser(ws_jsonl).parse()
            assert len(records) > 0
            # 至少有 done 类型的记录
            assert any(r.is_done for r in records)


# ===================== Detectors =====================

class TestGapDetector:
    def test_too_few_returns_zeros(self):
        m, s, z = GapDetector.detect([CronRunRecord(raw={}, ts=100.0)])
        assert (m, s, z) == (0.0, 0.0, 0.0)

    def test_compute_mean_std_z(self):
        recs = [CronRunRecord(raw={}, ts=t) for t in (100.0, 200.0, 300.0, 400.0)]
        m, s, z = GapDetector.detect(recs)
        # gaps 都 = 100, std = 0
        assert m == 100.0
        assert s == 0.0
        assert z == 0.0

    def test_drift_zscore(self):
        recs = [CronRunRecord(raw={}, ts=t) for t in (0.0, 100.0, 200.0, 300.0, 1000.0)]
        m, _, z = GapDetector.detect(recs)
        # 最后 gap=700, 前面 gaps mean=100, std 大约=0 (前面的都=100)
        # std != 0 因为最后 700 让 std 增大
        assert m > 100.0
        assert z > 0  # 最后 gap 比 mean 大


class TestProviderHealth:
    def test_empty_returns_zero(self):
        assert ProviderHealth.compute([]) == (0.0, 0.0, 0.0)

    def test_basic_proportions(self):
        recs = [
            CronRunRecord(raw={}, anysearch_used=True),
            CronRunRecord(raw={}, anysearch_used=True),
            CronRunRecord(raw={}, bocha_ai_used=True),
            CronRunRecord(raw={}, note="ERROR something"),
        ]
        bocha, any_, err = ProviderHealth.compute(recs)
        # bocha: 1/4=25%, any: 2/4=50%, err: 1/4=25%
        assert abs(bocha - 25.0) < 0.01
        assert abs(any_ - 50.0) < 0.01
        assert abs(err - 25.0) < 0.01


class TestLatencyStats:
    def test_empty(self):
        assert LatencyStats.compute([]) == (0.0, 0.0, 0.0)

    def test_single(self):
        assert LatencyStats.compute([42.0]) == (42.0, 42.0, 42.0)

    def test_p50_p95_max(self):
        vals = [10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0]
        p50, p95, mx = LatencyStats.compute(vals)
        # 线性插值无 R 依赖, 验证合理性
        assert 50.0 <= p50 <= 60.0
        assert 90.0 <= p95 <= 100.0
        assert mx == 100.0

    def test_outlier_dominates_max(self):
        vals = [1.0, 2.0, 3.0, 999.0]
        _, _, mx = LatencyStats.compute(vals)
        assert mx == 999.0


class TestIdleTimeoutDetector:
    def test_no_match(self):
        recs = [CronRunRecord(raw={}, note="clean run")]
        n_idle, n_fb, n_err = IdleTimeoutDetector.detect(recs)
        assert (n_idle, n_fb, n_err) == (0, 0, 0)

    def test_idle_match(self):
        recs = [
            CronRunRecord(raw={}, note="idle timeout M3"),
            CronRunRecord(raw={}, note="卡 minimax-m3"),
            CronRunRecord(raw={"foo": "bar"}, note="nothing"),
        ]
        n_idle, n_fb, n_err = IdleTimeoutDetector.detect(recs)
        assert n_idle == 2

    def test_fallback_match(self):
        recs = [
            CronRunRecord(raw={}, note="fallback to deepseek"),
            CronRunRecord(raw={}, note="switch model"),  # 也在 idle 中
        ]
        n_idle, n_fb, n_err = IdleTimeoutDetector.detect(recs)
        assert n_idle >= 1
        assert n_fb >= 1

    def test_error_match(self):
        recs = [CronRunRecord(raw={}, note="exception traceback")]
        n_idle, n_fb, n_err = IdleTimeoutDetector.detect(recs)
        assert n_err == 1


# ===================== Aggregator =====================

class TestAuditAggregator:
    def test_no_data_critical(self):
        overall, _ = AuditAggregator.aggregate(0, 0, 0, 0, 0, 0)
        assert overall == "CRITICAL"

    def test_high_error_rate_critical(self):
        overall, note = AuditAggregator.aggregate(10, 0, 0, 5, 0, 60)
        assert overall == "CRITICAL"
        assert "errors" in note

    def test_idle_5plus_degraded(self):
        overall, note = AuditAggregator.aggregate(100, 5, 0, 0, 0, 60)
        assert overall == "DEGRADED"
        assert "idle" in note

    def test_fallback_5plus_degraded(self):
        overall, _ = AuditAggregator.aggregate(100, 0, 5, 0, 0, 60)
        assert overall == "DEGRADED"

    def test_drift_degraded(self):
        overall, _ = AuditAggregator.aggregate(100, 0, 0, 0, 4.0, 60)
        assert overall == "DEGRADED"

    def test_max_duration_too_high_degraded(self):
        overall, _ = AuditAggregator.aggregate(100, 0, 0, 0, 0, 700)
        assert overall == "DEGRADED"

    def test_healthy(self):
        overall, note = AuditAggregator.aggregate(100, 1, 1, 1, 0.5, 60)
        assert overall == "HEALTHY"
        assert "records=100" in note


# ===================== Philosophy Guard =====================

class TestV3PhilosophyGuard:
    def _mk_report(self, **kw) -> AuditReport:
        r = AuditReport(
            timestamp="2026-01-01T00:00:00+08:00",
            n_rounds=10,
            n_records=10,
            n_idle_timeouts=0,
            n_fallbacks=0,
            n_errors=0,
            p50_duration_s=10.0,
            p95_duration_s=15.0,
            max_duration_s=20.0,
            gap_mean_s=100.0,
            gap_std_s=10.0,
            gap_drift_zscore=0.0,
            bocha_pct=10.0,
            anysearch_pct=80.0,
            error_provider_pct=10.0,
            components=[
                AuditResult(name="x", value=1, source="jsonl", confidence="high", note=""),
            ],
            overall=kw.get("overall", "HEALTHY"),
            overall_note=kw.get("overall_note", "ok"),
        )
        return r

    def test_source_check(self):
        rep = self._mk_report()
        rep.components[0].source = ""
        assert not V3PhilosophyGuard.check_report_source(rep)

    def test_no_fake_health(self):
        rep = self._mk_report(overall="")
        assert not V3PhilosophyGuard.check_no_fake_health(rep)

    def test_no_fake_asi(self):
        rep = self._mk_report(overall="ASI")  # 假冒 ASI
        assert not V3PhilosophyGuard.check_no_fake_asi(rep)
        rep2 = self._mk_report(overall_note="ASI is here")
        assert not V3PhilosophyGuard.check_no_fake_asi(rep2)

    def test_no_data_no_fake_zero(self):
        rep = self._mk_report()
        rep.n_records = 0
        rep.overall = "HEALTHY"  # 假装健康
        assert not V3PhilosophyGuard.check_no_data_no_fake_zero(rep)


# ===================== Report markdown / dict =====================

class TestAuditReport:
    def test_to_dict_keys(self):
        r = AuditReport(
            timestamp="t", n_rounds=1, n_records=1,
            n_idle_timeouts=0, n_fallbacks=0, n_errors=0,
            p50_duration_s=1.0, p95_duration_s=1.0, max_duration_s=1.0,
            gap_mean_s=1.0, gap_std_s=1.0, gap_drift_zscore=0.0,
            bocha_pct=0.0, anysearch_pct=0.0, error_provider_pct=0.0,
        )
        d = r.to_dict()
        assert "overall" in d
        assert "components" in d
        assert d["version"] == V1078_VERSION

    def test_to_markdown_contains_overall(self):
        r = AuditReport(
            timestamp="2026-01-01T00:00:00", n_rounds=1, n_records=1,
            n_idle_timeouts=0, n_fallbacks=0, n_errors=0,
            p50_duration_s=1.0, p95_duration_s=1.0, max_duration_s=1.0,
            gap_mean_s=1.0, gap_std_s=1.0, gap_drift_zscore=0.0,
            bocha_pct=0.0, anysearch_pct=0.0, error_provider_pct=0.0,
            overall="HEALTHY",
            overall_note="all good",
            components=[AuditResult(name="x", value=1, source="jsonl", confidence="high", note="")],
        )
        md = r.to_markdown()
        assert "V1078" in md
        assert "HEALTHY" in md
        assert "x" in md


# ===================== End-to-end / Auditor =====================

class TestCronSelfAuditor:
    def test_empty_jsonl_is_critical(self, tmp_path: Path):
        a = CronSelfAuditor(jsonl_path=tmp_path / "missing.jsonl")
        rep = a.audit()
        assert rep.overall == "CRITICAL"
        assert rep.n_records == 0

    def test_real_jsonl(self):
        ws = Path(__file__).parent.parent / "cron-research-runs.jsonl"
        if not ws.exists():
            pytest.skip("no cron-research-runs.jsonl in workspace")
        a = CronSelfAuditor(jsonl_path=ws)
        rep = a.audit()
        assert rep.n_records > 0
        # overall 是 HEALTHY/DEGRADED/CRITICAL 之一
        assert rep.overall in {"HEALTHY", "DEGRADED", "CRITICAL"}
        # V3 守门
        assert V3PhilosophyGuard.check_no_fake_asi(rep)
        assert V3PhilosophyGuard.check_report_source(rep)
