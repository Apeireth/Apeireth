"""Tests for V1435 — ASI 真生产 docker availability probe (主 13:31 + 主 23:44 + 主 00:56 + 主 17:43)."""

from __future__ import annotations

import json
import subprocess
import sys

import pytest


# ---------------------------------------------------------------------------
# Constants & guards
# ---------------------------------------------------------------------------


def test_v1435_importable():
    import apeireth.v1435_asi_docker_availability_probe as m
    assert m.V1435_VERSION == "0.1.0"


def test_v1435_guards_count():
    import apeireth.v1435_asi_docker_availability_probe as m
    assert len(m.V1435_GUARDS) == 14
    assert len(m.V1435_V3_GUARDS) == 5


def test_v1435_borrowed_count():
    import apeireth.v1435_asi_docker_availability_probe as m
    assert len(m.V1435_BORROWED) == 5


def test_v1435_default_and_max_timeout():
    import apeireth.v1435_asi_docker_availability_probe as m
    assert m.DEFAULT_TIMEOUT_SECONDS >= 1
    assert m.MAX_TIMEOUT_SECONDS >= m.DEFAULT_TIMEOUT_SECONDS
    assert m.MAX_TIMEOUT_SECONDS <= 300


# ---------------------------------------------------------------------------
# Enums / Dataclasses
# ---------------------------------------------------------------------------


def test_v1435_probe_modes_count():
    import apeireth.v1435_asi_docker_availability_probe as m
    assert len(list(m.ProbeMode)) == 5
    assert m.ProbeMode.DOCKER_READY.value == "DOCKER_READY"
    assert m.ProbeMode.DOCKER_MISSING.value == "DOCKER_MISSING"


def test_v1435_subprocess_call_dataclass():
    import apeireth.v1435_asi_docker_availability_probe as m
    sc = m.SubprocessCall(cmd="docker --version", rc=0, stdout="hi", elapsed_ms=12.5)
    d = sc.to_dict()
    assert d["cmd"] == "docker --version"
    assert d["rc"] == 0
    assert d["stdout_bytes"] >= 2
    assert d["elapsed_ms"] == 12.5
    assert d["timed_out"] is False


def test_v1435_docker_probe_result_dataclass():
    import apeireth.v1435_asi_docker_availability_probe as m
    r = m.DockerProbeResult(probe_mode="DOCKER_READY", docker_version="24.0.7", n_running_containers=5)
    d = r.to_dict()
    assert d["probe_mode"] == "DOCKER_READY"
    assert d["docker_version"] == "24.0.7"
    assert d["n_running_containers"] == 5
    assert d["calls"] == []


# ---------------------------------------------------------------------------
# Subprocess runner (offline-safe)
# ---------------------------------------------------------------------------


def test_v1435_run_subprocess_echo():
    """Echo is universal; should succeed."""
    import apeireth.v1435_asi_docker_availability_probe as m
    c = m.run_subprocess("echo hello-world", timeout=5)
    assert c.mode == "OK"
    assert "hello-world" in c.stdout
    assert c.timed_out is False
    assert c.elapsed_ms >= 0


def test_v1435_run_subprocess_nonexistent():
    """Non-existent binary should NOT raise; returns NOT_FOUND or FAILED."""
    import apeireth.v1435_asi_docker_availability_probe as m
    c = m.run_subprocess("definitely-not-a-real-binary-xyz-1435", timeout=2)
    assert c.mode in ("NOT_FOUND", "FAILED")
    assert c.timed_out is False


def test_v1435_run_subprocess_clamps_huge_timeout():
    """timeout > MAX should be clamped (no crash)."""
    import apeireth.v1435_asi_docker_availability_probe as m
    c = m.run_subprocess("echo clamped", timeout=9999)
    assert c.mode == "OK"
    assert "clamped" in c.stdout


def test_v1435_run_subprocess_clamps_zero_timeout():
    """timeout < 1 should be clamped to DEFAULT (no crash)."""
    import apeireth.v1435_asi_docker_availability_probe as m
    c = m.run_subprocess("echo zero-clamp", timeout=0)
    assert c.mode == "OK"
    assert "zero-clamp" in c.stdout


def test_v1435_run_subprocess_handles_failure_rc():
    """A command that exits non-zero should return mode=FAILED, not raise."""
    import apeireth.v1435_asi_docker_availability_probe as m
    # Use Python via shell to force a non-zero exit
    if sys.platform == "win32":
        c = m.run_subprocess("python -c \"import sys; sys.exit(7)\"", timeout=5)
    else:
        c = m.run_subprocess("python3 -c 'import sys; sys.exit(7)'", timeout=5)
    assert c.mode == "FAILED"
    assert c.rc == 7


# ---------------------------------------------------------------------------
# Parsers
# ---------------------------------------------------------------------------


def test_v1435_first_version_parses_semver():
    import apeireth.v1435_asi_docker_availability_probe as m
    assert m._first_version("Docker version 20.10.5, build abc123") == "20.10.5"
    assert m._first_version("docker-compose version 1.29.2") == "1.29.2"
    assert m._first_version("24.0.7") == "24.0.7"


def test_v1435_first_version_handles_missing():
    import apeireth.v1435_asi_docker_availability_probe as m
    assert m._first_version("") is None
    assert m._first_version("no version here") is None


def test_v1435_kv_parse_extracts_fields():
    import apeireth.v1435_asi_docker_availability_probe as m
    sample = (
        "Server Version: 24.0.7\n"
        "Kernel Version: 5.15.0\n"
        "Operating System: Ubuntu 22.04\n"
    )
    assert m._kv_parse(sample, "Server Version") == "24.0.7"
    assert m._kv_parse(sample, "Kernel Version") == "5.15.0"
    assert m._kv_parse(sample, "Operating System") == "Ubuntu 22.04"
    assert m._kv_parse(sample, "Missing Key") is None
    assert m._kv_parse("", "Server Version") is None


# ---------------------------------------------------------------------------
# Aggregated probe (offline-safe end-to-end)
# ---------------------------------------------------------------------------


def test_v1435_run_docker_probe_offline_safe():
    """Full probe must complete without raising on any host."""
    import apeireth.v1435_asi_docker_availability_probe as m
    result = m.run_docker_probe(timeout=3)
    # result is one of the 5 modes
    assert result.probe_mode in {p.value for p in m.ProbeMode}
    assert result.started_iso
    assert result.ended_iso
    assert result.timeout_seconds == 3
    assert len(result.calls) >= 1
    # No call should have unhandled exception
    for c in result.calls:
        assert c.mode in ("OK", "FAILED", "NOT_FOUND", "TIMEOUT", "PENDING")


def test_v1435_run_docker_probe_has_5_calls():
    """Probe runs 5 subprocess calls (binary + v2 + v1 + daemon + ps)."""
    import apeireth.v1435_asi_docker_availability_probe as m
    result = m.run_docker_probe(timeout=3)
    # binary probe + v2 + v1 + daemon + ps = 5 (or fewer if binary missing → 1)
    assert 1 <= len(result.calls) <= 5


def test_v1435_run_docker_probe_mode_consistent():
    """If mode is DOCKER_READY, server_version must be set and n_running >= 0."""
    import apeireth.v1435_asi_docker_availability_probe as m
    result = m.run_docker_probe(timeout=3)
    if result.probe_mode == m.ProbeMode.DOCKER_READY.value:
        assert result.server_version is not None
        assert result.n_running_containers >= 0
    elif result.probe_mode == m.ProbeMode.DOCKER_MISSING.value:
        assert result.docker_path is None
        assert result.docker_version is None


# ---------------------------------------------------------------------------
# Render / Serialize
# ---------------------------------------------------------------------------


def test_v1435_render_probe_summary_md():
    import apeireth.v1435_asi_docker_availability_probe as m
    r = m.DockerProbeResult(probe_mode="DOCKER_READY", docker_version="24.0.7", n_running_containers=2)
    md = m.render_probe_summary_md(r)
    assert "V1435" in md
    assert "DOCKER_READY" in md
    assert "24.0.7" in md
    assert "Honest disclosure" in md
    assert "phenomenal" in md.lower() or "Phenomenal" in md  # honesty paragraph mentions it


def test_v1435_result_to_dict_serializable():
    import apeireth.v1435_asi_docker_availability_probe as m
    r = m.DockerProbeResult(probe_mode="DOCKER_READY", docker_version="24.0.7", n_running_containers=3)
    j = json.dumps(m.result_to_dict(r))
    parsed = json.loads(j)
    assert parsed["probe_mode"] == "DOCKER_READY"
    assert parsed["docker_version"] == "24.0.7"
    assert parsed["n_running_containers"] == 3


def test_v1435_result_to_dict_with_calls():
    import apeireth.v1435_asi_docker_availability_probe as m
    r = m.DockerProbeResult()
    r.calls.append(m.SubprocessCall(cmd="docker --version", rc=0, stdout="hi", mode="OK", elapsed_ms=10))
    d = m.result_to_dict(r)
    assert len(d["calls"]) == 1
    assert d["calls"][0]["cmd"] == "docker --version"
    assert d["calls"][0]["mode"] == "OK"


# ---------------------------------------------------------------------------
# Popper self-test
# ---------------------------------------------------------------------------


def test_v1435_popper_self_test_passes():
    import apeireth.v1435_asi_docker_availability_probe as m
    out = m.popper_self_test()
    assert out["passed"] == out["total"], f"popper self-test failed: {out}"
    assert out["total"] == 14


def test_v1435_popper_self_test_includes_honesty():
    import apeireth.v1435_asi_docker_availability_probe as m
    out = m.popper_self_test()
    # P14 tests markdown render which includes honesty paragraph
    p14 = next(r for r in out["results"] if r["id"] == "P14")
    assert p14["ok"]


def test_v1435_popper_self_test_guards_count():
    import apeireth.v1435_asi_docker_availability_probe as m
    out = m.popper_self_test()
    p02 = next(r for r in out["results"] if r["id"] == "P02")
    assert p02["ok"]


# ---------------------------------------------------------------------------
# Chain delegate
# ---------------------------------------------------------------------------


def test_v1435_chain_delegate():
    import apeireth.v1435_asi_docker_availability_probe as m
    out = m.chain_delegate()
    assert out["v1435"]["ok"] is True
    assert "v1434" in out
    assert "v1430" in out
    assert "v1076" in out
    assert "borrowed" in out
    # Should not raise even if upstream modules can't be imported
    assert isinstance(out["all_ok"], bool)


def test_v1435_chain_delegate_borrows_v1434():
    """V1435 borrows from V1434 (artifact generator)."""
    import apeireth.v1435_asi_docker_availability_probe as m
    borrowed = out_borrowed = m.V1435_BORROWED
    names = {m for m, _ in borrowed}
    assert "v1434_asi_vcp_consistency_http" in names


# ---------------------------------------------------------------------------
# Module meta
# ---------------------------------------------------------------------------


def test_v1435_module_meta():
    import apeireth.v1435_asi_docker_availability_probe as m
    meta = m.module_meta()
    assert meta["module"] == "v1435_asi_docker_availability_probe"
    assert meta["version"] == "0.1.0"
    assert meta["schema"] == "v1435.asi-docker-availability-probe/v1"
    assert len(meta["guards"]) == 14
    assert len(meta["v3_guards"]) == 5
    assert len(meta["borrowed"]) == 5
    assert len(meta["probe_modes"]) == 5
    assert meta["default_timeout"] >= 1
    assert meta["max_timeout"] > meta["default_timeout"]


# ---------------------------------------------------------------------------
# CLI smoke
# ---------------------------------------------------------------------------


def test_v1435_cli_version():
    """CLI version command emits a version string."""
    r = subprocess.run(
        [sys.executable, "-m", "apeireth.v1435_asi_docker_availability_probe", "version"],
        capture_output=True, text=True, timeout=10,
    )
    assert r.returncode == 0
    assert r.stdout.strip() == "0.1.0"


def test_v1435_cli_meta_json():
    r = subprocess.run(
        [sys.executable, "-m", "apeireth.v1435_asi_docker_availability_probe", "meta", "--json"],
        capture_output=True, text=True, timeout=10,
    )
    assert r.returncode == 0
    meta = json.loads(r.stdout)
    assert meta["version"] == "0.1.0"


def test_v1435_cli_popper():
    r = subprocess.run(
        [sys.executable, "-m", "apeireth.v1435_asi_docker_availability_probe", "popper"],
        capture_output=True, text=True, timeout=15,
    )
    assert r.returncode == 0
    out = json.loads(r.stdout)
    assert out["passed"] == out["total"]


def test_v1435_cli_call_echo():
    """CLI `call --cmd "..." --timeout N` runs a real subprocess."""
    r = subprocess.run(
        [sys.executable, "-m", "apeireth.v1435_asi_docker_availability_probe", "call",
         "--cmd", "echo cli-call-test", "--timeout", "5"],
        capture_output=True, text=True, timeout=15,
    )
    assert r.returncode == 0
    out = json.loads(r.stdout)
    assert out["mode"] == "OK"
    assert out["cmd"] == "echo cli-call-test"


def test_v1435_cli_probe_offline_safe():
    """CLI probe runs end-to-end and never crashes."""
    r = subprocess.run(
        [sys.executable, "-m", "apeireth.v1435_asi_docker_availability_probe", "probe", "--timeout", "3"],
        capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=60,
    )
    assert r.returncode == 0
    assert r.stdout is not None
    assert "V1435" in r.stdout
    assert "probe_mode" in r.stdout


def test_v1435_cli_json_offline_safe():
    r = subprocess.run(
        [sys.executable, "-m", "apeireth.v1435_asi_docker_availability_probe", "json", "--timeout", "3"],
        capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=60,
    )
    assert r.returncode == 0
    assert r.stdout is not None
    out = json.loads(r.stdout)
    assert "probe_mode" in out
    assert "calls" in out


def test_v1435_cli_chain():
    r = subprocess.run(
        [sys.executable, "-m", "apeireth.v1435_asi_docker_availability_probe", "chain"],
        capture_output=True, text=True, timeout=15,
    )
    assert r.returncode == 0
    out = json.loads(r.stdout)
    assert "all_ok" in out
    assert "v1434" in out
