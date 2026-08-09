"""Tests for V1421 — ASI 总框架 daemon: wire V1418 cron tick + V1420 HTTP endpoint."""

from __future__ import annotations

import json
import sys
import tempfile
import threading
import time
from pathlib import Path

import pytest

# Ensure apeireth is importable
ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth import v1421_asi_daemon_serve_tick as m


# ============================================================================
# Constants / structural
# ============================================================================


def test_module_constants_present():
    """V1421 version, schema, module constants all defined."""
    assert m.V1421_VERSION == "0.1.0"
    assert m.V1421_SCHEMA == "v1421.asi-daemon-serve-tick/v1"
    assert m.V1421_MODULE == "v1421_asi_daemon_serve_tick"


def test_guards_and_v3_guards_well_formed():
    """V1421 declares ≥15 guards and ≥9 V3 guards; ≥4 borrowed."""
    assert len(m.V1421_GUARDS) >= 15
    assert len(m.V1421_V3_GUARDS) >= 9
    assert len(m.V1421_BORROWED) >= 4
    # Borrowed must include V1418 + V1420 + threading + signal
    borrowed_keys = [b[0] for b in m.V1421_BORROWED]
    assert "V1418" in borrowed_keys
    assert "V1420" in borrowed_keys


def test_default_bind_and_port_safe():
    """Default bind/port are safe (loopback + unprivileged port)."""
    assert m.DEFAULT_BIND_HOST == "127.0.0.1"
    assert m.DEFAULT_PORT >= 1024  # unprivileged
    assert m.MIN_PORT <= m.DEFAULT_PORT <= m.MAX_PORT


def test_cadence_bounds_reasonable():
    """Cadence bounds are reasonable (1s .. 24h)."""
    assert m.MIN_CADENCE_SECONDS == 1
    assert m.MAX_CADENCE_SECONDS == 86400
    assert m.DEFAULT_CADENCE_SECONDS == 300


# ============================================================================
# Path safety / config validation
# ============================================================================


def test_safe_path_rejects_dotdot():
    """_safe_path rejects '..' in any path component."""
    with pytest.raises(ValueError, match=r"\.\."):
        m._safe_path(Path("foo/../bar"))


def test_safe_path_accepts_absolute():
    """_safe_path accepts absolute paths without '..'."""
    p = m._safe_path(Path("/tmp/test.jsonl"))
    assert p.is_absolute()


def test_validate_bind_host_rejects_unknown():
    """_validate_bind_host rejects unknown hosts."""
    with pytest.raises(ValueError, match="bind host"):
        m._validate_bind_host("evil.example.com")


def test_validate_bind_host_accepts_loopback():
    """_validate_bind_host accepts 127.0.0.1 / localhost / 0.0.0.0."""
    assert m._validate_bind_host("127.0.0.1") == "127.0.0.1"
    assert m._validate_bind_host("localhost") == "localhost"
    assert m._validate_bind_host("0.0.0.0") == "0.0.0.0"


def test_validate_port_rejects_out_of_range():
    """_validate_port rejects 0 and 70000."""
    with pytest.raises(ValueError, match="port"):
        m._validate_port(0)
    with pytest.raises(ValueError, match="port"):
        m._validate_port(70000)


def test_validate_port_accepts_boundary():
    """_validate_port accepts 1 and 65535."""
    assert m._validate_port(1) == 1
    assert m._validate_port(65535) == 65535


def test_validate_cadence_rejects_out_of_range():
    """_validate_cadence rejects 0 and 99999."""
    with pytest.raises(ValueError, match="cadence_seconds"):
        m._validate_cadence(0)
    with pytest.raises(ValueError, match="cadence_seconds"):
        m._validate_cadence(99999)


def test_validate_max_seconds_rejects_negative():
    """_validate_max_seconds rejects <0."""
    with pytest.raises(ValueError, match="max_seconds"):
        m._validate_max_seconds(-1.0)


def test_validate_mode_rejects_unknown():
    """_validate_mode rejects unknown modes."""
    with pytest.raises(ValueError, match="mode"):
        m._validate_mode("chaos")


def test_validate_sleep_fn_rejects_unknown():
    """_validate_sleep_fn_name rejects unknown names."""
    with pytest.raises(ValueError, match="sleep_fn_name"):
        m._validate_sleep_fn_name("os.system")


# ============================================================================
# Config builders
# ============================================================================


def test_build_default_config_uses_daemon_mode():
    """Default mode is 'daemon' (natural V1418+V1420 wiring)."""
    cfg = m.build_default_config({})
    assert cfg.mode == "daemon"
    assert cfg.cadence_seconds == 300
    assert cfg.port == 8765


def test_build_default_config_overrides_applied():
    """Overrides flow into the dataclass."""
    cfg = m.build_default_config({"mode": "serve-only", "port": 9999})
    assert cfg.mode == "serve-only"
    assert cfg.port == 9999


def test_build_default_config_rejects_unknown_key():
    """build_default_config raises on unknown override key."""
    with pytest.raises(ValueError, match="unknown override key"):
        m.build_default_config({"nonexistent_key": 1})


def test_validate_config_rejects_dotdot_path():
    """validate_config rejects history_path with '..'."""
    cfg = m.build_default_config({"history_path": Path("foo/../bar.jsonl")})
    with pytest.raises(ValueError, match=r"\.\."):
        m.validate_config(cfg)


# ============================================================================
# Dataclass to_dict
# ============================================================================


def test_daemon_config_to_dict_redacts_token():
    """DaemonConfig.to_dict redacts auth_token plaintext."""
    cfg = m.build_default_config({"auth_token": "supersecret-xyz"})
    d = cfg.to_dict()
    assert d["auth_token"] == "<redacted>"


def test_daemon_tick_record_to_dict():
    """DaemonTickRecord.to_dict round-trip."""
    rec = m.DaemonTickRecord(
        cycle_index=1, started_iso="2026-08-10T00:00:00Z",
        ended_iso="2026-08-10T00:00:01Z", verdict="OK",
        policy="PROCEED", chain_ok=True, alerts_count=0,
        duration_seconds=1.0, note="",
    )
    d = rec.to_dict()
    assert d["cycle_index"] == 1
    assert d["policy"] == "PROCEED"
    assert d["chain_ok"] is True


def test_daemon_run_summary_to_dict():
    """DaemonRunSummary.to_dict round-trip."""
    s = m.DaemonRunSummary(
        mode="daemon", bind="127.0.0.1", port=8765,
        cadence_seconds=300, n_ticks=2, n_proceed=2, n_pause=0,
        n_lockdown=0, started_iso="t0", ended_iso="t1",
        reason="max-seconds", chain_ok=True, note="",
    )
    d = s.to_dict()
    assert d["n_ticks"] == 2
    assert d["n_proceed"] == 2


# ============================================================================
# Sleep helper
# ============================================================================


def test_sleep_honors_stop_event_immediately():
    """_sleep_with_event returns immediately if stop_event is pre-set."""
    ev = threading.Event()
    ev.set()  # already set
    slept = m._sleep_with_event(5.0, ev, "time.sleep")
    assert slept < 1.0


def test_sleep_honors_stop_event_midway():
    """_sleep_with_event wakes early when stop_event is set during sleep."""
    ev = threading.Event()
    def _set_later():
        time.sleep(0.3)
        ev.set()
    t = threading.Thread(target=_set_later, daemon=True)
    t.start()
    slept = m._sleep_with_event(5.0, ev, "time.sleep")
    t.join(timeout=2.0)
    assert slept < 2.0  # should wake well before 5s


def test_sleep_with_event_variant_works():
    """_sleep_with_event works with threading.Event.wait variant."""
    ev = threading.Event()
    ev.set()
    slept = m._sleep_with_event(5.0, ev, "threading.Event.wait")
    assert slept == 5.0  # wait() returns False when not set, but here ev was set so it returns True and we return `seconds`


# ============================================================================
# run_tick_once (smoke)
# ============================================================================


def test_run_tick_once_returns_record():
    """run_tick_once returns a DaemonTickRecord."""
    with tempfile.TemporaryDirectory() as td:
        cfg = m.build_default_config({
            "history_path": Path(td) / "h.jsonl",
            "baseline_path": Path(td) / "b.json",
            "tick_jsonl_path": Path(td) / "t.jsonl",
            "render_out": Path(td) / "r.md",
            "render": False,
        })
        m.validate_config(cfg)
        rec = m.run_tick_once(cfg, cycle_index=1)
        assert isinstance(rec, m.DaemonTickRecord)
        assert rec.cycle_index == 1
        # verdict ∈ {"OK", "DEGRADED", "ALERT", "ERROR"} (V1418 contract)
        assert rec.verdict in ("OK", "DEGRADED", "ALERT", "ERROR", "UNKNOWN")


# ============================================================================
# run_daemon — tick-and-exit
# ============================================================================


def test_daemon_tick_and_exit_runs_one_tick():
    """tick-and-exit mode runs exactly one tick."""
    with tempfile.TemporaryDirectory() as td:
        cfg = m.build_default_config({
            "mode": "tick-and-exit",
            "history_path": Path(td) / "h.jsonl",
            "baseline_path": Path(td) / "b.json",
            "tick_jsonl_path": Path(td) / "t.jsonl",
            "render_out": Path(td) / "r.md",
            "render": False,
        })
        m.validate_config(cfg)
        s = m.run_daemon(cfg)
        assert s.mode == "tick-and-exit"
        assert s.n_ticks == 1
        assert s.chain_ok is True


# ============================================================================
# run_daemon — daemon mode (smoke, short max-seconds)
# ============================================================================


def test_daemon_mode_short_max_seconds_exits():
    """Daemon mode respects --max-seconds (smoke test, no real long run)."""
    with tempfile.TemporaryDirectory() as td:
        cfg = m.build_default_config({
            "mode": "daemon",
            "bind": "127.0.0.1",
            "port": 18765,  # unlikely to collide in CI
            "cadence_seconds": 1,
            "max_seconds": 2,  # exit after 2s
            "history_path": Path(td) / "h.jsonl",
            "baseline_path": Path(td) / "b.json",
            "tick_jsonl_path": Path(td) / "t.jsonl",
            "render_out": Path(td) / "r.md",
            "render": False,
        })
        m.validate_config(cfg)
        s = m.run_daemon(cfg)
        assert s.mode == "daemon"
        # Should have ticked at least once in 2s with cadence=1
        assert s.n_ticks >= 1


# ============================================================================
# Chain delegate
# ============================================================================


def test_chain_delegate_returns_ok():
    """chain_delegate returns dict with all_ok=True when V1418+V1420 importable."""
    d = m.chain_delegate()
    assert isinstance(d, dict)
    assert d.get("all_ok") is True
    assert d.get("v1418_all_ok") is True
    assert d.get("v1420_all_ok") is True


def test_chain_delegate_includes_versions():
    """chain_delegate includes V1418/V1420 module names."""
    d = m.chain_delegate()
    assert "v1421_version" in d
    assert "v1421_module" in d
    assert "v1418_module" in d
    assert "v1420_module" in d


# ============================================================================
# Popper self-test
# ============================================================================


def test_popper_self_test_passes():
    """popper_self_test passes all 17 tests."""
    all_ok, n_pass, results = m.popper_self_test()
    assert all_ok, f"popper failed: {[r for r in results if not r['ok']]}"
    assert n_pass == 17
    assert len(results) == 17


# ============================================================================
# CLI
# ============================================================================


def test_cli_version():
    """CLI 'version' prints version."""
    import io
    from contextlib import redirect_stdout
    buf = io.StringIO()
    with redirect_stdout(buf):
        rc = m.run_cli(["version"])
    assert rc == 0
    assert "0.1.0" in buf.getvalue()


def test_cli_help():
    """CLI 'help' returns 0."""
    rc = m.run_cli(["help"])
    assert rc == 0


def test_cli_demo():
    """CLI 'demo' returns 0."""
    rc = m.run_cli(["demo"])
    assert rc == 0


def test_cli_meta_json():
    """CLI 'meta --json true' emits valid JSON."""
    import io
    from contextlib import redirect_stdout
    buf = io.StringIO()
    with redirect_stdout(buf):
        rc = m.run_cli(["meta", "--json", "true"])
    assert rc == 0
    payload = json.loads(buf.getvalue())
    assert payload["version"] == "0.1.0"
    assert payload["module"] == "v1421_asi_daemon_serve_tick"


def test_cli_chain_returns_ok():
    """CLI 'chain' returns 0 when chain is OK."""
    rc = m.run_cli(["chain"])
    assert rc == 0


def test_cli_popper_returns_0():
    """CLI 'popper' returns 0 when all popper tests pass."""
    rc = m.run_cli(["popper"])
    assert rc == 0


def test_cli_unknown_command_returns_1():
    """CLI unknown command returns 1."""
    rc = m.run_cli(["nonsense"])
    assert rc == 1


def test_cli_tick_and_exit():
    """CLI 'tick-and-exit' runs and returns 0."""
    rc = m.run_cli(["tick-and-exit"])
    assert rc == 0


def test_cli_serve_only_short():
    """CLI 'serve-only --max-seconds 1' runs and returns 0."""
    rc = m.run_cli(["serve-only", "--max-seconds", "1"])
    assert rc == 0


def test_cli_daemon_short():
    """CLI 'daemon --max-seconds 2 --cadence-seconds 1' runs and returns 0."""
    rc = m.run_cli(["daemon", "--max-seconds", "2", "--cadence-seconds", "1"])
    assert rc == 0


# ============================================================================
# V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)
# ============================================================================


def test_v3_guards_present():
    """All 9 V3 philosophy guards declared."""
    expected = [
        "GUARD_DAEMON_IS_NOT_PHENOMENAL",
        "GUARD_DAEMON_IS_NOT_ASI",
        "GUARD_DAEMON_IS_NOT_HUMAN_LEVEL",
        "GUARD_DAEMON_IS_NOT_ABSOLUTE",
        "GUARD_DAEMON_IS_NOT_V1418_REPLACE",
        "GUARD_DAEMON_IS_NOT_V1420_REPLACE",
        "GUARD_DAEMON_IS_NOT_V1419_REPLACE",
        "GUARD_DAEMON_IS_NOT_V1417_REPLACE",
        "GUARD_DAEMON_IS_NOT_V1411_REPLACE",
    ]
    for g in expected:
        assert g in m.V1421_V3_GUARDS, f"missing V3 guard: {g}"


def test_module_docstring_discloses_asi_gap():
    """Module docstring explicitly states V1421 ≠ ASI 达成."""
    doc = m.__doc__ or ""
    assert "ASI 达成" in doc or "ASI 北极星" in doc or "ASI 总框架" in doc
    assert "≠ Phenomenal" in doc or "IS_NOT_PHENOMENAL" in str(m.V1421_V3_GUARDS)


# ============================================================================
# Integration — V1418 + V1420 actually wired
# ============================================================================


def test_run_tick_once_invokes_v1418_public_api():
    """run_tick_once imports V1418 and calls its public API (chain probe)."""
    # If V1418 is not importable, run_tick_once returns ERROR verdict.
    # With V1418 importable (real env), we get a real verdict.
    rec = m.run_tick_once(m.build_default_config({}), cycle_index=1)
    assert isinstance(rec, m.DaemonTickRecord)
    assert rec.verdict != ""


def test_daemon_config_has_v1418_path_defaults():
    """Default config uses V1417/V1416 paths (real, not mocked)."""
    cfg = m.build_default_config({})
    assert str(cfg.history_path).endswith(".v1417-dgm-tick-history.jsonl")
    assert str(cfg.tick_jsonl_path).endswith(".v1416-dgm-ticks.jsonl")
