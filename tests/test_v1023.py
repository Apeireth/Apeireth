"""V1023 真生产 tests (主 23:44 干到底)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import time
import pytest
from apeireth.v1023_scheduler import (
    V1023_VERSION, ScheduledJob, parse_cron, matches_cron, V1023Scheduler,
)


class TestV1023:
    def test_parse_cron(self):
        parsed = parse_cron("0 * * * *")
        assert parsed["minute"] == "0"
        assert parsed["hour"] == "*"

    def test_parse_cron_invalid(self):
        with pytest.raises(ValueError):
            parse_cron("invalid")

    def test_parse_cron_too_few_parts(self):
        with pytest.raises(ValueError):
            parse_cron("* * *")

    def test_matches_cron_wildcard(self):
        """V1023 真测 cron wildcard (主 17:43 实事求是)."""
        # * * * * * should match any time
        assert matches_cron("* * * * *") is True

    def test_matches_cron_specific(self):
        # Get current minute and match
        now = time.time()
        t = time.localtime(now)
        expr = f"{t.tm_min} {t.tm_hour} {t.tm_mday} {t.tm_mon} {t.tm_wday}"
        assert matches_cron(expr, now) is True

    def test_matches_cron_step(self):
        # */15 should match minutes 0, 15, 30, 45
        ts = time.mktime(time.struct_time((2026, 7, 22, 0, 15, 0, 0, 0, 0)))
        assert matches_cron("*/15 * * * *", ts) is True
        ts = time.mktime(time.struct_time((2026, 7, 22, 0, 16, 0, 0, 0, 0)))
        assert matches_cron("*/15 * * * *", ts) is False

    def test_matches_cron_list(self):
        ts = time.mktime(time.struct_time((2026, 7, 22, 0, 5, 0, 0, 0, 0)))
        assert matches_cron("0,5,10 * * * *", ts) is True

    def test_matches_cron_mismatch(self):
        ts = time.mktime(time.struct_time((2026, 7, 22, 0, 5, 0, 0, 0, 0)))
        assert matches_cron("30 * * * *", ts) is False

    def test_init(self):
        s = V1023Scheduler()
        assert s.n_jobs() == 0

    def test_add_job(self):
        """V1023 真测 APScheduler 真借鉴 (主 19:33)."""
        s = V1023Scheduler()
        jid = s.add_job("test", "fn", "* * * * *")
        assert s.n_jobs() == 1
        assert jid in s.jobs

    def test_add_job_invalid_cron(self):
        s = V1023Scheduler()
        with pytest.raises(ValueError):
            s.add_job("test", "fn", "invalid")

    def test_tick(self):
        s = V1023Scheduler()
        s.add_job("test", "fn", "* * * * *")
        fired = s.tick()
        assert "test" in [s.jobs[jid].name for jid in fired]
        # Actually test only the count
        assert len(fired) >= 1

    def test_tick_disabled(self):
        s = V1023Scheduler()
        jid = s.add_job("test", "fn", "* * * * *")
        s.disable_job(jid)
        fired = s.tick()
        assert jid not in fired

    def test_disable_enable(self):
        s = V1023Scheduler()
        jid = s.add_job("test", "fn", "* * * * *")
        assert s.disable_job(jid) is True
        assert s.jobs[jid].enabled is False
        assert s.enable_job(jid) is True
        assert s.jobs[jid].enabled is True

    def test_disable_unknown(self):
        s = V1023Scheduler()
        assert s.disable_job("unknown") is False

    def test_run_count(self):
        s = V1023Scheduler()
        jid = s.add_job("test", "fn", "* * * * *")
        s.tick()
        s.tick()
        assert s.jobs[jid].run_count == 2

    def test_stats(self):
        s = V1023Scheduler()
        s.add_job("j1", "fn1", "* * * * *")
        s.add_job("j2", "fn2", "0 * * * *")
        st = s.stats()
        assert st["n_jobs"] == 2

    def test_v22_33_asi_integration(self):
        """V1023 真测主 22:33 ASI 北极星."""
        s = V1023Scheduler()
        st = s.stats()
        assert "ASI" in st["philosophy"]

    def test_v19_33_apscheduler(self):
        """V1023 真测主 19:33 APScheduler 真借鉴."""
        s = V1023Scheduler()
        jid = s.add_job("v1001_tick", "v1001.tick", "* * * * *")
        assert jid in s.jobs
        fired = s.tick()
        assert isinstance(fired, list)

    def test_v17_43_truth(self):
        """V1023 真测主 17:43 实事求是 — 真 cron 匹配."""
        # 2026-07-22 00:15:00 → minute=15 should match */15
        ts = time.mktime(time.struct_time((2026, 7, 22, 0, 15, 0, 0, 0, 0)))
        assert matches_cron("*/15 * * * *", ts) is True
        # minute=16 shouldn't match */15
        ts = time.mktime(time.struct_time((2026, 7, 22, 0, 16, 0, 0, 0, 0)))
        assert matches_cron("*/15 * * * *", ts) is False

    def test_complete_integration(self):
        """V1023 真测完整 scheduler (主 23:44 + 主 22:33 + 主 19:33 + 主 17:33)."""
        s = V1023Scheduler()
        s.add_job("v1001_tick", "v1001.tick", "* * * * *")
        s.add_job("v1002_measure", "v1002.measure", "0 * * * *")
        # 真 tick
        fired = s.tick()
        assert len(fired) == 1  # 只有 * * * * * 触发
        # run count
        for jid in s.jobs:
            assert s.jobs[jid].run_count >= 1 if jid == list(s.jobs.keys())[0] else s.jobs[jid].run_count == 0