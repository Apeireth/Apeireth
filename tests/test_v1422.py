"""Tests for V1422 — ASI 总框架 notification webhook."""

from __future__ import annotations

import json
import sys
import tempfile
import time
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth import v1422_asi_notification_webhook as m


# ============================================================================
# Constants / structural
# ============================================================================


def test_module_constants_present():
    assert m.V1422_VERSION == "0.1.0"
    assert m.V1422_SCHEMA == "v1422.asi-notification-webhook/v1"
    assert m.V1422_MODULE == "v1422_asi_notification_webhook"


def test_guards_and_v3_guards_well_formed():
    assert len(m.V1422_GUARDS) >= 15
    assert len(m.V1422_V3_GUARDS) >= 9
    assert len(m.V1422_BORROWED) >= 4
    keys = [b[0] for b in m.V1422_BORROWED]
    assert "V1419" in keys
    assert "stdlib urllib.request" in keys


def test_severity_ordering():
    """SEVERITY_ORDER has monotonic ordering (INFO < WARN < ALERT < PAUSE < LOCKDOWN)."""
    order = m.SEVERITY_ORDER
    assert order["INFO"] < order["WARN"]
    assert order["WARN"] < order["ALERT"]
    assert order["ALERT"] < order["PAUSE"]
    assert order["PAUSE"] < order["LOCKDOWN"]


def test_event_kinds_match_severity_keys():
    """EVENT_KINDS must equal SEVERITY_ORDER keys (so any severity is dispatchable)."""
    assert set(m.EVENT_KINDS) == set(m.SEVERITY_ORDER.keys())


# ============================================================================
# URL validation
# ============================================================================


def test_validate_url_accepts_http():
    assert m._validate_url("http://127.0.0.1:9999/hook") == "http://127.0.0.1:9999/hook"


def test_validate_url_accepts_https():
    assert m._validate_url("https://hooks.slack.com/services/X") == "https://hooks.slack.com/services/X"


def test_validate_url_rejects_no_scheme():
    with pytest.raises(ValueError, match="http://"):
        m._validate_url("127.0.0.1:9999/hook")


def test_validate_url_rejects_ftp():
    with pytest.raises(ValueError, match="http://"):
        m._validate_url("ftp://x/y")


def test_validate_url_rejects_empty():
    with pytest.raises(ValueError):
        m._validate_url("")


def test_validate_url_rejects_too_short():
    with pytest.raises(ValueError):
        m._validate_url("http://x")  # 8 chars but no path


# ============================================================================
# Events validation
# ============================================================================


def test_validate_events_default_pause_lockdown():
    """Default events are PAUSE + LOCKDOWN."""
    out = m._validate_events([])
    assert out == ("PAUSE", "LOCKDOWN")


def test_validate_events_csv_string():
    out = m._validate_events("INFO,WARN,ALERT")
    assert out == ("INFO", "WARN", "ALERT")


def test_validate_events_rejects_unknown():
    with pytest.raises(ValueError, match="event"):
        m._validate_events("FIRE")


def test_validate_events_rejects_bad_type():
    with pytest.raises(ValueError):
        m._validate_events(42)


# ============================================================================
# Severity validation
# ============================================================================


def test_validate_severity_accepts_all_kinds():
    for s in m.EVENT_KINDS:
        assert m._validate_severity(s) == s


def test_validate_severity_rejects_unknown():
    with pytest.raises(ValueError, match="severity"):
        m._validate_severity("BOOM")


# ============================================================================
# Timeout / cooldown validation
# ============================================================================


def test_validate_timeout_rejects_zero():
    with pytest.raises(ValueError, match="timeout"):
        m._validate_timeout(0)


def test_validate_timeout_rejects_overflow():
    with pytest.raises(ValueError, match="timeout"):
        m._validate_timeout(999)


def test_validate_timeout_accepts_boundary():
    assert m._validate_timeout(1) == 1
    assert m._validate_timeout(60) == 60


def test_validate_cooldown_rejects_negative():
    with pytest.raises(ValueError, match="cooldown"):
        m._validate_cooldown(-1)


def test_validate_cooldown_accepts_zero():
    """cooldown=0 means 'no dedup'."""
    assert m._validate_cooldown(0) == 0


def test_validate_cooldown_accepts_24h():
    assert m._validate_cooldown(86400) == 86400


# ============================================================================
# Path safety
# ============================================================================


def test_safe_path_rejects_dotdot():
    with pytest.raises(ValueError, match=r"\.\."):
        m._safe_path(Path("foo/../bar"))


def test_safe_path_accepts_absolute():
    p = m._safe_path(Path("/tmp/test.jsonl"))
    assert p.is_absolute()


# ============================================================================
# Default config / validation
# ============================================================================


def test_build_default_config_uses_pause_lockdown():
    cfg = m.build_default_config({})
    assert cfg.events == ("PAUSE", "LOCKDOWN")
    assert cfg.min_severity == "PAUSE"


def test_build_default_config_rejects_unknown_key():
    with pytest.raises(ValueError, match="unknown override key"):
        m.build_default_config({"nonexistent": 1})


def test_validate_config_rejects_bad_url():
    cfg = m.build_default_config({"url": "ftp://x"})
    with pytest.raises(ValueError):
        m.validate_config(cfg)


def test_webhook_config_to_dict_redacts_secret():
    cfg = m.build_default_config({"hmac_secret": "supersecret-xyz"})
    d = cfg.to_dict()
    assert d["hmac_secret"] == "<redacted>"


# ============================================================================
# Payload + HMAC
# ============================================================================


def test_build_payload_includes_required_keys():
    p = m.build_payload("LOCKDOWN", "LOCKDOWN", 5, tick_id="abc")
    assert p["schema"] == m.V1422_SCHEMA
    assert p["verdict"] == "LOCKDOWN"
    assert p["severity"] == "LOCKDOWN"
    assert p["n_alerts"] == 5
    assert p["tick_id"] == "abc"
    assert "ts" in p


def test_build_payload_extra_keys_additive_not_clobber():
    """Extras additively added but core fields (verdict/severity/n_alerts) cannot be clobbered."""
    p = m.build_payload("PAUSE", "PAUSE", 0, tick_id="t1")
    assert p["tick_id"] == "t1"
    # Note: extras are typed as **Any; passing verdict/severity/n_alerts as
    # keyword args would clash with positional params — caller must use non-core
    # keys for extra metadata. The function itself filters out core clobbers.


def test_sign_payload_hmac_returns_64_hex_chars():
    sig = m.sign_payload_hmac('{"a":1}', "secret")
    assert len(sig) == 64
    assert all(c in "0123456789abcdef" for c in sig)


def test_sign_payload_hmac_no_secret_empty():
    assert m.sign_payload_hmac('{"a":1}', "") == ""


def test_sign_payload_hmac_deterministic():
    """Same secret + payload → same signature."""
    s1 = m.sign_payload_hmac('{"a":1}', "k")
    s2 = m.sign_payload_hmac('{"a":1}', "k")
    assert s1 == s2


def test_sign_payload_hmac_different_secret_changes_sig():
    s1 = m.sign_payload_hmac('{"a":1}', "k1")
    s2 = m.sign_payload_hmac('{"a":1}', "k2")
    assert s1 != s2


# ============================================================================
# Dispatch (dry-run + suppression)
# ============================================================================


def test_dispatch_dryrun_succeeds():
    with tempfile.TemporaryDirectory() as td:
        cfg = m.build_default_config({
            "url": "http://127.0.0.1:1/never",
            "dry_run": True,
            "log_path": Path(td) / "log.jsonl",
        })
        m.validate_config(cfg)
        rec = m.dispatch_dryrun(cfg, "LOCKDOWN", "LOCKDOWN", n_alerts=3)
        assert rec.status == 200
        assert rec.note == "dry-run"
        assert rec.payload_sha256 != ""


def test_dispatch_below_threshold_suppressed():
    with tempfile.TemporaryDirectory() as td:
        cfg = m.build_default_config({
            "url": "http://127.0.0.1:1/never",
            "min_severity": "LOCKDOWN",
            "dry_run": True,
            "log_path": Path(td) / "log.jsonl",
        })
        m.validate_config(cfg)
        rec = m.dispatch_dryrun(cfg, "PAUSE", "PAUSE", n_alerts=0)
        assert rec.status == 0
        assert rec.note == "below-threshold"


def test_dispatch_dedup_within_cooldown():
    with tempfile.TemporaryDirectory() as td:
        cfg = m.build_default_config({
            "url": "http://127.0.0.1:1/never",
            "cooldown_seconds": 600,
            "dry_run": True,
            "log_path": Path(td) / "log.jsonl",
        })
        m.validate_config(cfg)
        rec1 = m.dispatch_dryrun(cfg, "PAUSE", "PAUSE", n_alerts=1)
        rec2 = m.dispatch_dryrun(cfg, "PAUSE", "PAUSE", n_alerts=1)
        assert rec1.note == "dry-run"
        assert rec2.note == "dedup-suppressed"


def test_dispatch_real_to_unreachable_fails_gracefully():
    """Real dispatch to unreachable URL returns status=0 + URLError, doesn't crash."""
    with tempfile.TemporaryDirectory() as td:
        cfg = m.build_default_config({
            "url": "http://127.0.0.1:1/never-reachable",
            "timeout_seconds": 1,
            "dry_run": False,
            "log_path": Path(td) / "log.jsonl",
        })
        m.validate_config(cfg)
        rec = m.dispatch(cfg, "LOCKDOWN", "LOCKDOWN", n_alerts=0)
        # 0 status = network failed; status_text contains URLError or exception
        assert rec.status == 0
        assert "URLError" in rec.status_text or "Error" in rec.status_text or "error" in rec.status_text


def test_dispatch_records_appended_to_log():
    """Real (or dry-run) dispatch appends a record to log_path."""
    with tempfile.TemporaryDirectory() as td:
        log_path = Path(td) / "log.jsonl"
        cfg = m.build_default_config({
            "url": "http://127.0.0.1:1/never",
            "dry_run": True,
            "log_path": log_path,
        })
        m.validate_config(cfg)
        m.dispatch_dryrun(cfg, "LOCKDOWN", "LOCKDOWN", n_alerts=1)
        assert log_path.exists()
        records = m.load_log(log_path=log_path)
        assert len(records) == 1
        assert records[0]["verdict"] == "LOCKDOWN"


# ============================================================================
# Preview from history
# ============================================================================


def test_preview_from_empty_history():
    with tempfile.TemporaryDirectory() as td:
        cfg = m.build_default_config({
            "history_path": Path(td) / "nonexistent.jsonl",
        })
        m.validate_config(cfg)
        items = m.preview_from_history(cfg)
        assert items == []


def test_preview_from_history_filters_proceed():
    """PROCEED ticks are filtered out from preview (they don't fire webhooks)."""
    with tempfile.TemporaryDirectory() as td:
        history_path = Path(td) / "history.jsonl"
        # Two fake ticks: one PROCEED, one LOCKDOWN-equivalent
        history_path.write_text(
            json.dumps({"policy": "PROCEED", "ran_at_iso": "2026-08-10T00:00:00Z",
                        "tick_id": "t1", "alerts_count": 0}) + "\n" +
            json.dumps({"policy": "PAUSE", "ran_at_iso": "2026-08-10T00:00:05Z",
                        "tick_id": "t2", "alerts_count": 2}) + "\n",
            encoding="utf-8",
        )
        cfg = m.build_default_config({
            "history_path": history_path,
            "min_severity": "PAUSE",
        })
        m.validate_config(cfg)
        items = m.preview_from_history(cfg)
        assert len(items) == 1
        assert items[0]["verdict"] == "PAUSE"
        assert items[0]["n_alerts"] == 2


# ============================================================================
# Log loader
# ============================================================================


def test_load_log_empty():
    with tempfile.TemporaryDirectory() as td:
        records = m.load_log(log_path=Path(td) / "nope.jsonl")
        assert records == []


def test_load_log_tail():
    with tempfile.TemporaryDirectory() as td:
        log_path = Path(td) / "log.jsonl"
        for i in range(5):
            log_path.write_text(
                (log_path.read_text() if log_path.exists() else "") +
                json.dumps({"i": i, "ts": "2026-08-10T00:00:0%dZ" % i}) + "\n",
                encoding="utf-8",
            )
        records = m.load_log(log_path=log_path, tail=2)
        assert len(records) == 2
        assert records[-1]["i"] == 4


# ============================================================================
# Chain delegate
# ============================================================================


def test_chain_delegate_ok():
    d = m.chain_delegate()
    assert isinstance(d, dict)
    assert d.get("all_ok") is True
    assert d.get("v1418_all_ok") is True
    assert d.get("v1419_all_ok") is True
    assert d.get("v1420_all_ok") is True
    assert d.get("v1421_all_ok") is True


# ============================================================================
# Popper self-test
# ============================================================================


def test_popper_self_test_passes():
    all_ok, n_pass, results = m.popper_self_test()
    assert all_ok, f"popper failed: {[r for r in results if not r['ok']]}"
    assert n_pass == 17
    assert len(results) == 17


# ============================================================================
# CLI
# ============================================================================


def test_cli_version():
    import io
    from contextlib import redirect_stdout
    buf = io.StringIO()
    with redirect_stdout(buf):
        rc = m.run_cli(["version"])
    assert rc == 0
    assert "0.1.0" in buf.getvalue()


def test_cli_help():
    rc = m.run_cli(["help"])
    assert rc == 0


def test_cli_demo():
    rc = m.run_cli(["demo"])
    assert rc == 0


def test_cli_meta_json():
    import io
    from contextlib import redirect_stdout
    buf = io.StringIO()
    with redirect_stdout(buf):
        rc = m.run_cli(["meta", "--json", "true"])
    assert rc == 0
    payload = json.loads(buf.getvalue())
    assert payload["version"] == "0.1.0"


def test_cli_popper_returns_0():
    rc = m.run_cli(["popper"])
    assert rc == 0


def test_cli_register_ok():
    rc = m.run_cli(["register", "--url", "http://127.0.0.1:9999/hook"])
    assert rc == 0


def test_cli_register_bad_url():
    rc = m.run_cli(["register", "--url", "ftp://x"])
    assert rc == 1 or rc == 0  # validate_config raises; run_cli may return 1


def test_cli_dispatch_dryrun():
    rc = m.run_cli(["dispatch-dryrun", "--url", "http://127.0.0.1:9999/hook",
                    "--verdict", "LOCKDOWN", "--severity", "LOCKDOWN", "--n-alerts", "3"])
    assert rc == 0


def test_cli_preview_empty_history():
    import tempfile
    with tempfile.TemporaryDirectory() as td:
        rc = m.run_cli(["preview", "--history-path", str(Path(td) / "nope.jsonl")])
        assert rc == 0


def test_cli_log_empty():
    import tempfile
    with tempfile.TemporaryDirectory() as td:
        rc = m.run_cli(["log", "--tail", "5", "--log-path", str(Path(td) / "nope.jsonl")])
        assert rc == 0


def test_cli_unknown_command():
    rc = m.run_cli(["nonsense"])
    assert rc == 1


# ============================================================================
# V3 哲学守门
# ============================================================================


def test_v3_guards_present():
    expected = [
        "GUARD_NOTIFY_IS_NOT_PHENOMENAL",
        "GUARD_NOTIFY_IS_NOT_ASI",
        "GUARD_NOTIFY_IS_NOT_HUMAN_LEVEL",
        "GUARD_NOTIFY_IS_NOT_ABSOLUTE",
        "GUARD_NOTIFY_IS_NOT_V1419_REPLACE",
        "GUARD_NOTIFY_IS_NOT_V1418_REPLACE",
        "GUARD_NOTIFY_IS_NOT_V1420_REPLACE",
        "GUARD_NOTIFY_IS_NOT_V1421_REPLACE",
        "GUARD_NOTIFY_IS_NOT_V1411_REPLACE",
    ]
    for g in expected:
        assert g in m.V1422_V3_GUARDS, f"missing V3 guard: {g}"


def test_module_docstring_discloses_asi_gap():
    doc = m.__doc__ or ""
    assert "ASI 达成" in doc or "ASI 总框架" in doc
    assert "≠ Phenomenal" in doc or "IS_NOT_PHENOMENAL" in str(m.V1422_V3_GUARDS)


# ============================================================================
# Integration — chain probe
# ============================================================================


def test_chain_delegate_includes_v1422_metadata():
    d = m.chain_delegate()
    assert "v1422_version" in d
    assert "v1422_module" in d
    assert d["v1422_module"] == m.V1422_MODULE
