"""Smoke tests for the user-facing Apeireth CLI."""
from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def _cli(*args: str) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env["PYTHONPATH"] = str(ROOT) + os.pathsep + env.get("PYTHONPATH", "")
    return subprocess.run(
        [sys.executable, "-m", "apeireth.cli", *args],
        cwd=ROOT,
        env=env,
        text=True,
        capture_output=True,
        timeout=30,
    )


def test_version() -> None:
    result = _cli("--version")
    assert result.returncode == 0
    assert result.stdout.startswith("apeireth ")


def test_run_returns_non_empty_string() -> None:
    result = _cli("run", "hello")
    assert result.returncode == 0
    assert result.stdout.strip()


def test_run_score_is_opt_in() -> None:
    result = _cli("run", "--score", "hello")
    assert result.returncode == 0
    assert "score" in result.stdout.lower()


def test_run_unknown_model_fails() -> None:
    result = _cli("run", "--model", "nonexistent", "hello")
    assert result.returncode != 0
    assert "unknown model" in result.stderr.lower()


def test_demo_does_not_crash() -> None:
    result = _cli("demo")
    assert result.returncode == 0
    assert "Phase 1" in result.stdout
    assert "Phase 5" in result.stdout
