"""Tests for V1440 — ASI Docker container run attempt (主 13:31 + 主 23:44 + 主 00:56 + 主 17:43).

Coverage:
- Constants / guards / borrowed / module_meta
- DockerRunMode enum (9 values)
- 3 dataclasses (DockerRunSubprocess + DockerAvailabilityLite + DockerRunResult)
- probe_docker_availability (real V1435 probe)
- build_docker_run_cmd (command structure)
- run_docker_container (real subprocess, offline-safe)
- run_v1440 (top-level orchestrator)
- render_report_md
- chain_delegate, popper_self_test
- CLI: meta --json, popper, chain, run (smoke), json (smoke), probe-only, help, version
"""

from __future__ import annotations

import json
import subprocess
import sys

import pytest


# ---------------------------------------------------------------------------
# Constants & guards
# ---------------------------------------------------------------------------


def test_v1440_importable():
    import apeireth.v1440_asi_docker_container_run as m
    assert m.V1440_VERSION == "0.1.0"


def test_v1440_guards_count():
    import apeireth.v1440_asi_docker_container_run as m
    assert len(m.V1440_GUARDS) == 14
    assert len(m.V1440_V3_GUARDS) == 5


def test_v1440_borrowed_count():
    import apeireth.v1440_asi_docker_container_run as m
    assert len(m.V1440_BORROWED) == 5


def test_v1440_default_constants():
    import apeireth.v1440_asi_docker_container_run as m
    assert m.DEFAULT_TIMEOUT_SECONDS >= 1
    assert m.MAX_TIMEOUT_SECONDS >= m.DEFAULT_TIMEOUT_SECONDS
    assert m.MAX_OUTPUT_BYTES > 0
    assert m.DEFAULT_IMAGE_PRIMARY
    assert m.DEFAULT_IMAGE_FALLBACK
    assert m.DEFAULT_IMAGE_PRIMARY != m.DEFAULT_IMAGE_FALLBACK
    assert m.DEFAULT_TEST_CMD
    assert m.DEFAULT_DOCKER_RUN_TIMEOUT > 0


def test_v1440_module_meta_keys():
    import apeireth.v1440_asi_docker_container_run as m
    meta = m.module_meta()
    assert meta["module"] == m.V1440_MODULE
    assert meta["version"] == "0.1.0"
    assert meta["default_image_primary"] == "alpine:latest"
    assert meta["default_image_fallback"] == "busybox:latest"


# ---------------------------------------------------------------------------
# Enums / Dataclasses
# ---------------------------------------------------------------------------


def test_v1440_docker_run_modes_count():
    import apeireth.v1440_asi_docker_container_run as m
    modes = list(m.DockerRunMode)
    assert len(modes) >= 9
    for required in (
        "DOCKER_NOT_INSTALLED",
        "DOCKER_DAEMON_DOWN",
        "RUN_OK",
        "RUN_TIMEOUT",
        "RUN_FAILED",
        "IMAGE_PULL_FAILED",
        "RUN_DENIED",
        "SKIPPED",
        "ERROR",
    ):
        assert required in [bm.value for bm in modes]


def test_v1440_docker_run_subprocess_default():
    import apeireth.v1440_asi_docker_container_run as m
    s = m.DockerRunSubprocess()
    assert s.mode == m.DockerRunMode.SKIPPED
    assert s.image == ""
    assert s.test_cmd == ""


def test_v1440_docker_availability_lite_default():
    import apeireth.v1440_asi_docker_container_run as m
    a = m.DockerAvailabilityLite()
    assert a.docker_path == "NOT_FOUND"
    assert a.daemon_ready is False
    assert a.n_running == -1


def test_v1440_docker_run_result_to_dict():
    import apeireth.v1440_asi_docker_container_run as m
    a = m.DockerAvailabilityLite()
    s = m.DockerRunSubprocess()
    r = m.DockerRunResult(availability=a, subprocess=s, mode=m.DockerRunMode.SKIPPED)
    d = r.to_dict()
    assert "availability" in d
    assert "subprocess" in d
    assert d["mode"] == "SKIPPED"
    assert d["subprocess"]["mode"] == "SKIPPED"


# ---------------------------------------------------------------------------
# Command builder
# ---------------------------------------------------------------------------


def test_v1440_build_docker_run_cmd_structure():
    import apeireth.v1440_asi_docker_container_run as m
    cmd = m.build_docker_run_cmd(image="alpine", test_cmd="echo hi", docker_bin="docker")
    assert cmd[0] == "docker"
    assert cmd[1] == "run"
    assert cmd[2] == "--rm"
    assert "alpine" in cmd


def test_v1440_build_docker_run_cmd_with_custom_bin():
    import apeireth.v1440_asi_docker_container_run as m
    cmd = m.build_docker_run_cmd(image="busybox", test_cmd="date", docker_bin="/usr/local/bin/docker")
    assert cmd[0] == "/usr/local/bin/docker"
    assert "busybox" in cmd
    assert "date" in cmd


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def test_v1440_coerce_timeout_bounds():
    import apeireth.v1440_asi_docker_container_run as m
    assert m._coerce_timeout(0) == 1
    assert m._coerce_timeout(-5) == 1
    assert m._coerce_timeout(99999) == m.MAX_TIMEOUT_SECONDS
    assert m._coerce_timeout("abc") == m.DEFAULT_DOCKER_RUN_TIMEOUT
    assert m._coerce_timeout(15) == 15


def test_v1440_truncate_bounded():
    import apeireth.v1440_asi_docker_container_run as m
    small = m._truncate("hello")
    big = m._truncate("x" * 100, max_bytes=10)
    assert "hello" in small
    assert len(big) <= 50


# ---------------------------------------------------------------------------
# probe_docker_availability
# ---------------------------------------------------------------------------


def test_v1440_probe_docker_availability_returns_dataclass():
    import apeireth.v1440_asi_docker_container_run as m
    lite = m.probe_docker_availability(timeout=5)
    assert isinstance(lite, m.DockerAvailabilityLite)
    # On this host, docker is missing, but the probe must not raise
    assert isinstance(lite.docker_path, str)


# ---------------------------------------------------------------------------
# run_docker_container
# ---------------------------------------------------------------------------


def test_v1440_run_docker_container_no_docker():
    """If docker is not installed, returns DOCKER_NOT_INSTALLED without raising."""
    import apeireth.v1440_asi_docker_container_run as m

    # Save original
    orig_which = m.shutil.which
    try:
        m.shutil.which = lambda x: None  # type: ignore[assignment]
        sub = m.run_docker_container(image="alpine", test_cmd="echo hi", timeout=2, docker_bin="docker")
        assert sub.mode == m.DockerRunMode.DOCKER_NOT_INSTALLED
        assert sub.image == "alpine"
    finally:
        m.shutil.which = orig_which


def test_v1440_run_docker_container_returns_dataclass():
    """Real subprocess (offline-safe) — does not raise."""
    import apeireth.v1440_asi_docker_container_run as m

    sub = m.run_docker_container(image="alpine", test_cmd="echo hi", timeout=2)
    assert isinstance(sub, m.DockerRunSubprocess)
    # On this host (no docker), expect DOCKER_NOT_INSTALLED
    assert sub.mode in (
        m.DockerRunMode.DOCKER_NOT_INSTALLED,
        m.DockerRunMode.RUN_FAILED,
        m.DockerRunMode.ERROR,
        m.DockerRunMode.RUN_OK,  # in case docker IS installed
    )


# ---------------------------------------------------------------------------
# run_v1440 (top-level)
# ---------------------------------------------------------------------------


def test_v1440_run_v1440_returns_result():
    """Top-level orchestrator returns DockerRunResult (offline-safe)."""
    import apeireth.v1440_asi_docker_container_run as m

    result = m.run_v1440(timeout=5)
    assert isinstance(result, m.DockerRunResult)
    assert isinstance(result.mode, m.DockerRunMode)
    assert result.started_iso
    assert result.ended_iso


def test_v1440_run_v1440_offline_safe():
    """If docker missing, returns DOCKER_NOT_INSTALLED without crash."""
    import apeireth.v1440_asi_docker_container_run as m

    # Save original
    orig_probe = m.probe_docker_availability
    try:
        m.probe_docker_availability = lambda timeout=5: m.DockerAvailabilityLite(  # type: ignore[assignment]
            docker_path="NOT_FOUND",
            docker_version="UNKNOWN",
            daemon_ready=False,
            server_version="UNREACHABLE",
            n_running=-1,
            mode="DOCKER_MISSING",
        )
        result = m.run_v1440(timeout=5)
        assert result.mode == m.DockerRunMode.DOCKER_NOT_INSTALLED
        assert "skipping docker run" in str(result.notes)
    finally:
        m.probe_docker_availability = orig_probe


def test_v1440_run_v1440_daemon_down():
    """If daemon is down, returns DOCKER_DAEMON_DOWN without crash."""
    import apeireth.v1440_asi_docker_container_run as m

    orig_probe = m.probe_docker_availability
    try:
        m.probe_docker_availability = lambda timeout=5: m.DockerAvailabilityLite(  # type: ignore[assignment]
            docker_path="C:\\docker\\docker.exe",
            docker_version="20.10.0",
            daemon_ready=False,
            server_version="UNREACHABLE",
            n_running=-1,
            mode="DAEMON_DOWN",
        )
        result = m.run_v1440(timeout=5)
        assert result.mode == m.DockerRunMode.DOCKER_DAEMON_DOWN
    finally:
        m.probe_docker_availability = orig_probe


def test_v1440_run_v1440_docker_ready_no_image():
    """If docker + daemon ready, attempts real docker run (may fail on no image)."""
    import apeireth.v1440_asi_docker_container_run as m

    orig_probe = m.probe_docker_availability
    orig_run = m.run_docker_container
    try:
        m.probe_docker_availability = lambda timeout=5: m.DockerAvailabilityLite(  # type: ignore[assignment]
            docker_path="C:\\docker\\docker.exe",
            docker_version="20.10.0",
            daemon_ready=True,
            server_version="20.10.0",
            n_running=0,
            mode="DOCKER_READY",
        )
        # Mock run_docker_container to avoid actually running
        m.run_docker_container = lambda image, test_cmd, timeout, docker_bin=None: m.DockerRunSubprocess(  # type: ignore[assignment]
            pid=1234,
            cmd=f"docker run --rm {image} sh -c {test_cmd}",
            rc=0,
            stdout="hello-from-v1440\n",
            stderr="",
            elapsed_ms=150.0,
            timed_out=False,
            mode=m.DockerRunMode.RUN_OK,
            image=image,
            test_cmd=test_cmd,
        )
        result = m.run_v1440(timeout=5)
        # Should be RUN_OK from our mock
        assert result.mode == m.DockerRunMode.RUN_OK
        assert "hello-from-v1440" in result.subprocess.stdout
    finally:
        m.probe_docker_availability = orig_probe
        m.run_docker_container = orig_run


# ---------------------------------------------------------------------------
# Render report
# ---------------------------------------------------------------------------


def test_v1440_render_report_md():
    import apeireth.v1440_asi_docker_container_run as m

    result = m.run_v1440(timeout=5)
    md = m.render_report_md(result)
    assert "# V1440" in md
    assert "## Availability (from V1435)" in md
    assert "## Subprocess" in md
    assert "## Honest disclosure" in md


# ---------------------------------------------------------------------------
# chain_delegate
# ---------------------------------------------------------------------------


def test_v1440_chain_delegate_all_ok():
    import apeireth.v1440_asi_docker_container_run as m

    ch = m.chain_delegate()
    assert ch["v1440"]["ok"] is True
    assert ch["v1435"]["ok"] is True
    assert ch["v1439"]["ok"] is True
    assert ch["v1438"]["ok"] is True
    assert ch["v1435"]["importable"] is True
    assert ch["v1439"]["importable"] is True


def test_v1440_chain_delegate_borrowed_listed():
    import apeireth.v1440_asi_docker_container_run as m

    ch = m.chain_delegate()
    assert len(ch["borrowed"]) == 5


# ---------------------------------------------------------------------------
# popper_self_test
# ---------------------------------------------------------------------------


def test_v1440_popper_self_test():
    import apeireth.v1440_asi_docker_container_run as m

    res = m.popper_self_test()
    assert res["n_tests"] == 14
    # All structural tests should pass even without docker
    assert res["n_fail"] == 0
    assert res["n_pass"] == 14


def test_v1440_popper_each_test_id_present():
    import apeireth.v1440_asi_docker_container_run as m

    res = m.popper_self_test()
    ids = [r["id"] for r in res["results"]]
    for i in range(1, 15):
        assert f"P{i:02d}" in ids


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def test_v1440_cli_version():
    import apeireth.v1440_asi_docker_container_run as m

    rc = m.main(["version"])
    assert rc == 0


def test_v1440_cli_help():
    import apeireth.v1440_asi_docker_container_run as m

    rc = m.main(["help"])
    assert rc == 0


def test_v1440_cli_meta_json():
    import apeireth.v1440_asi_docker_container_run as m

    rc = m.main(["meta", "--json"])
    assert rc == 0


def test_v1440_cli_popper():
    import apeireth.v1440_asi_docker_container_run as m

    rc = m.main(["popper"])
    assert rc == 0


def test_v1440_cli_chain():
    import apeireth.v1440_asi_docker_container_run as m

    rc = m.main(["chain"])
    assert rc == 0


def test_v1440_cli_probe_only():
    """Just V1435 probe (no docker run)."""
    import apeireth.v1440_asi_docker_container_run as m

    rc = m.main(["probe-only", "--timeout", "5"])
    assert rc == 0


def test_v1440_cli_run_smoke():
    """CLI run — full V1435 probe + docker run + report (offline-safe)."""
    import apeireth.v1440_asi_docker_container_run as m

    rc = m.main(["run", "--timeout", "5"])
    assert rc == 0


def test_v1440_cli_json_smoke():
    """CLI json — emits JSON (offline-safe)."""
    import apeireth.v1440_asi_docker_container_run as m

    rc = m.main(["json", "--timeout", "5"])
    assert rc == 0


def test_v1440_cli_unknown_command_returns_2():
    import apeireth.v1440_asi_docker_container_run as m

    rc = m.main(["not_a_real_command"])
    assert rc == 2
