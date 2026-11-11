#!/usr/bin/env python3
"""Provision the merge gate's isolated CI toolchain from rust-toolchain.toml.

CI only: requires GITHUB_ACTIONS, GITHUB_ENV and an existing RUNNER_TEMP.
Creates a fresh child directory; never removes or updates a shared rustup home.
The existing CARGO_HOME and PATH retain access to the installed rustup proxies.
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path


def read_spec(root: Path) -> tuple[str, str, list[str]]:
    with (root / "rust-toolchain.toml").open("rb") as handle:
        spec = tomllib.load(handle)["toolchain"]
    channel = spec.get("channel")
    profile = spec.get("profile")
    components = spec.get("components")
    if not isinstance(channel, str) or not re.fullmatch(r"\d+\.\d+\.\d+", channel):
        raise ValueError("rust-toolchain.toml must pin an exact release channel")
    if profile not in {"minimal", "default", "complete"}:
        raise ValueError("rust-toolchain.toml must declare a toolchain profile")
    if not isinstance(components, list) or not components or any(
        not isinstance(component, str) or not re.fullmatch(r"[a-z][a-z0-9-]*", component)
        for component in components
    ):
        raise ValueError("rust-toolchain.toml must declare nonempty components")
    return channel, profile, components


def provision(root: Path, environ: dict[str, str], run=subprocess.run) -> Path:
    """Install and verify before exporting the isolated home to later CI steps."""
    if environ.get("GITHUB_ACTIONS") != "true" or not all(
        environ.get(key) for key in ("GITHUB_ENV", "RUNNER_TEMP")
    ):
        raise ValueError("CI only: GITHUB_ACTIONS=true, GITHUB_ENV and RUNNER_TEMP are required")
    runner_temp = Path(environ["RUNNER_TEMP"]).resolve()
    if not runner_temp.is_dir():
        raise ValueError("RUNNER_TEMP must be an existing directory")
    channel, profile, components = read_spec(root)
    rustup_home = Path(
        tempfile.mkdtemp(prefix="apeireth-merge-gate-rustup-", dir=runner_temp)
    ).resolve()
    if rustup_home == runner_temp or not rustup_home.is_relative_to(runner_temp):
        raise ValueError("isolated RUSTUP_HOME must be strictly inside RUNNER_TEMP")
    if any("\n" in str(value) or "\r" in str(value) for value in (rustup_home, channel)):
        raise ValueError("CI environment values must be single-line")
    child_env = dict(environ, RUSTUP_HOME=str(rustup_home), RUSTUP_TOOLCHAIN=channel)
    command = [
        "rustup", "toolchain", "install", channel,
        "--profile", profile, "--no-self-update",
    ]
    for component in components:
        command.extend(["--component", component])
    run(command, cwd=root, env=child_env, check=True)
    installed = run(
        ["rustup", "component", "list", "--toolchain", channel, "--installed"],
        cwd=root, env=child_env, check=True, capture_output=True, text=True,
    ).stdout.splitlines()
    for component in components:
        if not any(line == component or line.startswith(component + "-") for line in installed):
            raise ValueError(f"requested component not installed: {component}")
    version = run(
        ["rustup", "run", channel, "rustc", "--version"],
        cwd=root, env=child_env, check=True, capture_output=True, text=True,
    ).stdout.strip()
    if not version.startswith(f"rustc {channel} "):
        raise ValueError(f"installed compiler does not match {channel}: {version}")
    print(version)
    run(["rustup", "run", channel, "cargo", "--version"], cwd=root, env=child_env, check=True)
    if "clippy" in components:
        run(
            ["rustup", "run", channel, "cargo", "clippy", "--version"],
            cwd=root, env=child_env, check=True,
        )
    with Path(environ["GITHUB_ENV"]).open("a", encoding="utf-8") as handle:
        handle.write(f"RUSTUP_HOME={rustup_home}\nRUSTUP_TOOLCHAIN={channel}\n")
    print(f"Verified CI toolchain {channel}; RUSTUP_HOME={rustup_home}")
    return rustup_home


def main() -> int:
    argparse.ArgumentParser(description=__doc__).parse_args()
    try:
        provision(Path(__file__).resolve().parent.parent, dict(os.environ))
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        print(f"FAIL: CI toolchain provisioning: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
