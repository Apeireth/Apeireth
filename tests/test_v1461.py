"""V1461 — Tests for ASI Real Windows Docker-Equivalent Subprocess Sandbox.

Author: 楚零 (Chu Ling) | cron tick 2026-08-10 12:15 Asia/Shanghai
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

# Ensure the promethean package is importable
_PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
if str(_PROMETHEAN_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROMETHEAN_ROOT))

import apeireth.v1461_asi_docker_equivalent_subprocess_sandbox as v1461  # noqa: E402


# ============================================================================
# Module-level smoke tests
# ============================================================================


def test_module_importable():
    """V1461 module imports without error."""
    assert v1461.V1461_VERSION == "0.1.0"
    assert v1461.V1461_MODULE == "v1461_asi_docker_equivalent_subprocess_sandbox"
    assert v1461.V1461_SCHEMA == "v1461.asi-docker-equivalent-subprocess-sandbox/v1"
    assert len(v1461.V1461_GUARDS) >= 12
    assert len(v1461.V1461_V3_GUARDS) >= 5
    assert len(v1461.V1461_BORROWED) >= 8


def test_guards_declared():
    """V1461 declares exactly 12 guards."""
    assert len(v1461.V1461_GUARDS) == 12
    expected = {
        "GUARD_SPEC_DECLARED", "GUARD_RUNNER_BUILT", "GUARD_TIMEOUT_BOUNDED",
        "GUARD_OUTPUT_BOUNDED", "GUARD_ENV_ALLOWLISTED", "GUARD_TEMPDIR_ISOLATED",
        "GUARD_SUBPROCESS_REAL", "GUARD_EXIT_CODE_CAPTURED", "GUARD_HONEST_DISCLOSURE",
        "GUARD_CLI_RUNNABLE", "GUARD_BORROWED_LINEAGE", "GUARD_MODES_EXHAUSTIVE",
    }
    assert set(v1461.V1461_GUARDS) == expected


def test_v3_guards_declared():
    """V1461 declares 5 V3 philosophical guards."""
    assert len(v1461.V1461_V3_GUARDS) == 5
    for g in v1461.V1461_V3_GUARDS:
        assert g.startswith("GUARD_SANDBOX_NOT_"), g


def test_modes_exhaustive():
    """V1461 SandboxMode has exactly 9 values."""
    assert len(v1461.SandboxMode) == 9
    expected = {
        "SANDBOX_OK", "TIMEOUT", "FAILED", "DENIED", "BIN_NOT_FOUND",
        "BIN_INVALID", "BOUNDED_ERROR", "SKIPPED", "ERROR",
    }
    assert {m.value for m in v1461.SandboxMode} == expected


def test_timeout_bounds():
    """V1461 timeout bounds are sane (1 ≤ default ≤ max)."""
    assert 1 <= v1461.DEFAULT_TIMEOUT_S <= v1461.MAX_TIMEOUT_S
    assert v1461.MAX_TIMEOUT_S == 120


def test_output_bounds():
    """V1461 output bounds are sane (256 ≤ default ≤ max)."""
    assert v1461.MIN_MAX_OUTPUT_BYTES <= v1461.DEFAULT_MAX_OUTPUT_BYTES <= v1461.MAX_MAX_OUTPUT_BYTES
    assert v1461.MAX_MAX_OUTPUT_BYTES == 65536


# ============================================================================
# Env filter tests
# ============================================================================


def test_is_allowlisted_env_key_keeps_safe():
    """is_allowlisted_env_key keeps PATH / HOME / etc."""
    for k in ("PATH", "HOME", "USER", "TEMP", "LANG", "PYTHONIOENCODING"):
        assert v1461._is_allowlisted_env_key(k), k


def test_is_allowlisted_env_key_blocks_secrets():
    """is_allowlisted_env_key blocks PROXY/SECRET/TOKEN/KEY/PASSWORD substrings."""
    for k in ("FOO_PROXY", "GITHUB_TOKEN", "AWS_SECRET_ACCESS_KEY", "DB_PASSWORD", "MY_AUTH"):
        assert not v1461._is_allowlisted_env_key(k), k


def test_build_filtered_env_drops_secrets():
    """build_filtered_env drops *_PROXY and *_SECRET_* keys."""
    saved = dict(os.environ)
    try:
        os.environ["FOO_PROXY_URL"] = "http://x"
        os.environ["GH_TOKEN"] = "secret"
        os.environ["PATH"] = "/usr/bin"
        env = v1461.build_filtered_env()
        assert "PATH" in env
        assert "FOO_PROXY_URL" not in env
        assert "GH_TOKEN" not in env
    finally:
        os.environ.clear()
        os.environ.update(saved)


def test_build_filtered_env_extra_blocked():
    """build_filtered_env rejects non-allowlisted extra keys."""
    env = v1461.build_filtered_env({"FOO_PROXY": "http://x"})
    assert "FOO_PROXY" not in env
    env2 = v1461.build_filtered_env({"PATH": "/custom"})
    assert env2.get("PATH") == "/custom"


# ============================================================================
# Spec validation tests
# ============================================================================


def test_spec_valid_default():
    """Default SandboxSpec is valid."""
    spec = v1461.SandboxSpec(command=[sys.executable, "--version"])
    valid, issues = spec.is_valid()
    assert valid
    assert issues == []


def test_spec_rejects_empty_command():
    """Empty command → invalid."""
    spec = v1461.SandboxSpec(command=[])
    valid, issues = spec.is_valid()
    assert not valid
    assert any("command" in i for i in issues)


def test_spec_rejects_bad_timeout():
    """Timeout 0 or 999 → invalid."""
    for t in (0, -1, 121, 999):
        spec = v1461.SandboxSpec(command=["x"], timeout_s=t)
        valid, issues = spec.is_valid()
        assert not valid
        assert any("timeout" in i for i in issues)


def test_spec_rejects_bad_output():
    """max_output_bytes out of bounds → invalid."""
    for n in (0, 100, 100_000):
        spec = v1461.SandboxSpec(command=["x"], max_output_bytes=n)
        valid, issues = spec.is_valid()
        assert not valid
        assert any("max_output_bytes" in i for i in issues)


def test_spec_rejects_disallowed_env_key():
    """env_extra key with deny substring → invalid."""
    spec = v1461.SandboxSpec(command=["x"], env_extra={"FOO_PROXY": "http://x"})
    valid, issues = spec.is_valid()
    assert not valid
    assert any("env key" in i for i in issues)


# ============================================================================
# Runner behavior tests — real subprocess
# ============================================================================


def test_runner_python_version_succeeds():
    """Running python --version in sandbox returns rc=0 SANDBOX_OK."""
    spec = v1461.SandboxSpec(command=[sys.executable, "--version"])
    r = v1461.SandboxRunner().run(spec)
    assert r.mode == v1461.SandboxMode.SANDBOX_OK
    assert r.rc == 0
    assert not r.timed_out
    assert "Python" in (r.stdout + r.stderr)


def test_runner_intentional_fail_returns_failed():
    """Raising SystemExit(7) in subprocess → FAILED mode with rc=7."""
    spec = v1461.SandboxSpec(
        command=[sys.executable, "-c", "raise SystemExit(7)"]
    )
    r = v1461.SandboxRunner().run(spec)
    assert r.mode == v1461.SandboxMode.FAILED
    assert r.rc == 7


def test_runner_bin_not_found():
    """Non-existent binary → BIN_NOT_FOUND."""
    spec = v1461.SandboxSpec(command=["__definitely_not_a_binary_xyz__"])
    r = v1461.SandboxRunner().run(spec)
    assert r.mode == v1461.SandboxMode.BIN_NOT_FOUND
    assert r.rc is None


def test_runner_timeout_enforced():
    """Subprocess exceeding timeout → TIMEOUT mode."""
    spec = v1461.SandboxSpec(
        command=[sys.executable, "-c", "import time; time.sleep(3)"],
        timeout_s=1,
    )
    r = v1461.SandboxRunner().run(spec)
    assert r.mode == v1461.SandboxMode.TIMEOUT
    assert r.timed_out is True
    assert r.elapsed_ms < 2500  # bounded


def test_runner_tempdir_isolated():
    """Each run uses a fresh tempdir (workdir differs across runs)."""
    runner = v1461.SandboxRunner()
    r1 = runner.run(v1461.SandboxSpec(command=[sys.executable, "-c", "print('a')"]))
    r2 = runner.run(v1461.SandboxSpec(command=[sys.executable, "-c", "print('b')"]))
    assert r1.workdir and r2.workdir
    assert r1.workdir != r2.workdir


def test_runner_output_truncated():
    """Output exceeding max_output_bytes is truncated."""
    spec = v1461.SandboxSpec(
        command=[sys.executable, "-c", "print('x' * 5000)"],
        max_output_bytes=512,
    )
    r = v1461.SandboxRunner().run(spec)
    assert len(r.stdout.encode("utf-8")) <= 512
    assert r.stdout_truncated is True


def test_runner_bounded_error_on_invalid_spec():
    """Invalid spec → BOUNDED_ERROR, no subprocess spawned."""
    spec = v1461.SandboxSpec(command=[], timeout_s=999)
    r = v1461.SandboxRunner().run(spec)
    assert r.mode == v1461.SandboxMode.BOUNDED_ERROR
    assert r.rc is None


def test_run_in_sandbox_helper():
    """run_in_sandbox convenience wrapper works."""
    r = v1461.run_in_sandbox(
        command=[sys.executable, "-c", "print('hi')"],
        image_alias="test:1",
    )
    assert r.mode == v1461.SandboxMode.SANDBOX_OK
    assert r.spec.image_alias == "test:1"


# ============================================================================
# Batch + summary tests
# ============================================================================


def test_run_batch_returns_all_results():
    """run_batch returns one result per spec."""
    runner = v1461.SandboxRunner()
    specs = [
        v1461.SandboxSpec(command=[sys.executable, "-c", "print('a')"]),
        v1461.SandboxSpec(command=[sys.executable, "-c", "print('b')"]),
        v1461.SandboxSpec(command=[sys.executable, "-c", "raise SystemExit(1)"]),
    ]
    results = runner.run_batch(specs)
    assert len(results) == 3
    assert results[0].mode == v1461.SandboxMode.SANDBOX_OK
    assert results[1].mode == v1461.SandboxMode.SANDBOX_OK
    assert results[2].mode == v1461.SandboxMode.FAILED


def test_summarize_empty():
    """summarize with 0 runs returns n_runs=0."""
    s = v1461.SandboxRunner().summarize()
    assert s == {"n_runs": 0}


def test_summarize_with_runs():
    """summarize aggregates correctly."""
    runner = v1461.SandboxRunner()
    runner.run_batch([
        v1461.SandboxSpec(command=[sys.executable, "-c", "print(1)]" if False else "print(1)"]),
        v1461.SandboxSpec(command=[sys.executable, "-c", "raise SystemExit(2)"]),
        v1461.SandboxSpec(command=[sys.executable, "-c", "print(3)"]),
    ])
    s = runner.summarize()
    assert s["n_runs"] == 3
    assert s["ok"] == 2
    # ok_rate is rounded to 4 decimals (round(x, 4))
    assert abs(s["ok_rate"] - round(2/3, 4)) < 1e-4
    assert "SANDBOX_OK" in s["mode_counts"]
    assert "FAILED" in s["mode_counts"]


# ============================================================================
# Top-level helper + CLI
# ============================================================================


def test_run_v1461_top_level():
    """run_v1461 returns a structured summary with results."""
    s = v1461.run_v1461()
    assert s["v1461_version"] == "0.1.0"
    assert s["n_runs"] >= 4
    assert "honest_disclosure" in s
    assert "container" in s["honest_disclosure"]


def test_render_report_md():
    """render_report_md produces markdown with mode table + disclosure."""
    s = v1461.run_v1461()
    md = v1461.render_report_md(s)
    assert "# V1461" in md
    assert "Mode counts" in md
    assert "Honest disclosure" in md
    assert "container" in md


def test_cli_help():
    """CLI help prints docstring."""
    rc = v1461.main(["help"])
    assert rc == 0


def test_cli_meta():
    """CLI meta prints JSON metadata."""
    rc = v1461.main(["meta"])
    assert rc == 0


def test_cli_popper():
    """CLI popper runs self-test."""
    rc = v1461.main(["popper"])
    assert rc == 0


def test_cli_chain():
    """CLI chain probes upstream."""
    rc = v1461.main(["chain"])
    # all_ok may be False if upstream missing; we only assert exit code in {0,1}
    assert rc in (0, 1)


def test_cli_status():
    """CLI status prints version."""
    rc = v1461.main(["status"])
    assert rc == 0


def test_cli_run_subprocess_real():
    """CLI run actually invokes a subprocess via the sandbox."""
    rc = v1461.main(["run", sys.executable, "-c", "print('cli-real')"])
    assert rc == 0


def test_cli_run_no_command_errors():
    """CLI run with no command → exit 2."""
    rc = v1461.main(["run"])
    assert rc == 2


def test_cli_unknown_cmd():
    """CLI unknown subcommand → SystemExit(2) from argparse."""
    import pytest
    with pytest.raises(SystemExit) as exc_info:
        v1461.main(["nonsense_xyz"])
    assert exc_info.value.code == 2


# ============================================================================
# popper + meta + chain_delegate
# ============================================================================


def test_popper_self_test_all_pass():
    """popper_self_test returns all_ok=True."""
    r = v1461.popper_self_test()
    assert r["all_ok"] is True
    assert r["passed"] == r["total"]


def test_module_meta_shape():
    """module_meta returns the expected keys."""
    m = v1461.module_meta()
    for k in ("v1461_version", "v1461_schema", "v1461_module", "phase",
              "guards", "v3_guards", "borrowed", "modes", "platform"):
        assert k in m, k
    assert m["phase"] == 1461


def test_chain_delegate_returns_dict():
    """chain_delegate returns a dict with 'probes' and 'all_ok'."""
    r = v1461.chain_delegate()
    assert "v1461" in r
    assert "probes" in r
    assert "all_ok" in r


def test_safe_basename_strips_path_chars():
    """_safe_basename strips path separators and special chars."""
    assert v1461._safe_basename("foo/bar") == "foo_bar"
    assert v1461._safe_basename("a b c") == "a_b_c"
    assert v1461._safe_basename("...hidden...") == "hidden"


def test_truncate_short_unchanged():
    """_truncate leaves short text unchanged with truncated=False."""
    t, trunc = v1461._truncate("hello", 100)
    assert t == "hello"
    assert trunc is False


def test_truncate_long_truncated():
    """_truncate trims long text and sets truncated=True."""
    t, trunc = v1461._truncate("a" * 1000, 100)
    assert len(t) == 100
    assert trunc is True