#!/usr/bin/env python3
"""Exercise CI retry evidence with an isolated, dependency-free Rust test.

The deliberate first-attempt failure belongs only to this reporting probe. Its
captured runner summary is never printed on success or counted as workspace tests.
"""

from __future__ import annotations

import argparse
import os
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import tomllib
import xml.etree.ElementTree as ET
from pathlib import Path


TEST_NAME = "controlled_first_attempt_failure"
STDOUT_MARKER = "REPORT_PROBE_FAILED_STDOUT"
STDERR_MARKER = "REPORT_PROBE_FAILED_STDERR"
PANIC_MARKER = "REPORT_PROBE_FIRST_ATTEMPT_PANIC"

MANIFEST = '''[package]
name = "nextest-reporting-probe"
version = "0.0.0"
edition = "2021"
publish = false

[workspace]
'''

LOCKFILE = '''version = 4

[[package]]
name = "nextest-reporting-probe"
version = "0.0.0"
'''

SOURCE = '''#[test]
fn controlled_first_attempt_failure() {
    use std::io::Write;

    let state = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("attempt-state");
    match std::fs::OpenOptions::new().write(true).create_new(true).open(&state) {
        Ok(mut file) => {
            file.write_all(b"1").expect("first attempt state");
            println!("REPORT_PROBE_FAILED_STDOUT");
            eprintln!("REPORT_PROBE_FAILED_STDERR");
            panic!("REPORT_PROBE_FIRST_ATTEMPT_PANIC");
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            assert_eq!(std::fs::read(&state).expect("attempt state"), b"1");
            std::fs::write(&state, b"2").expect("second attempt state");
        }
        Err(error) => panic!("probe state creation failed: {error}"),
    }
}
'''


def child(root: Path, relative: str) -> Path:
    """Resolve and validate every probe path before writing or reading it."""
    path = (root / relative).resolve()
    if path == root or not path.is_relative_to(root):
        raise ValueError("probe path must stay strictly inside its temporary directory")
    return path


def cleanup(root: Path, parent: Path) -> None:
    resolved = root.resolve(strict=True)
    if resolved != root or resolved.parent != parent:
        raise ValueError("refusing cleanup outside the newly created temporary directory")
    shutil.rmtree(resolved)


def prepare_workspace_report(root: Path) -> bool:
    """Remove only the expected JUnit file, rejecting redirected path components."""
    workspace = root.resolve(strict=True)
    target = workspace / "target"
    report = target / "nextest" / "ci" / "junit.xml"
    resolved_target = target.resolve()
    resolved_report = report.resolve()
    if (
        resolved_target != target
        or resolved_target == workspace
        or not resolved_target.is_relative_to(workspace)
        or resolved_report != report
        or not resolved_report.is_relative_to(resolved_target)
    ):
        raise ValueError("workspace JUnit path must stay inside its unredirected target directory")
    path = workspace
    for part in ("target", "nextest", "ci", "junit.xml"):
        path = path / part
        try:
            metadata = path.lstat()
        except FileNotFoundError:
            return False
        if stat.S_ISLNK(metadata.st_mode) or (
            getattr(metadata, "st_file_attributes", 0)
            & getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0)
        ):
            raise ValueError("workspace JUnit path must not contain symlinks or reparse points")
        if path != report and not stat.S_ISDIR(metadata.st_mode):
            raise ValueError("workspace JUnit parent must be a directory")
        if path == report and not stat.S_ISREG(metadata.st_mode):
            raise ValueError("workspace JUnit report must be a regular file")
    report.unlink()
    return True


def verify_report(report: Path, output: str) -> None:
    if not report.is_file():
        raise ValueError("CI JUnit report was not created by the controlled retry run")
    tree = ET.parse(report)
    cases = tree.findall(".//testcase")
    if len(cases) != 1 or cases[0].get("name") != TEST_NAME:
        raise ValueError("JUnit did not identify the sole controlled probe test")
    case = cases[0]
    if any(case.find(tag) is not None for tag in ("failure", "error", "skipped")):
        raise ValueError("JUnit did not record the final successful retry")
    attempts = [
        node for node in case
        if node.tag in {"flakyFailure", "flakyError", "rerunFailure", "rerunError"}
    ]
    if len(attempts) != 1:
        raise ValueError("JUnit must retain exactly one failed attempt for the probe")
    failure_text = "".join(attempts[0].itertext())
    if not all(marker in failure_text for marker in (STDOUT_MARKER, STDERR_MARKER, PANIC_MARKER)):
        raise ValueError("JUnit lost the failed attempt's stdout, stderr, or panic detail")
    lines = [line for line in output.splitlines() if TEST_NAME in line]
    if not any(re.search(r"\b(?:RETRY|TRY)\b", line) for line in lines):
        raise ValueError("runner log did not expose the controlled retry's test identity")
    if not any(re.search(r"\bFLAKY\b", line) for line in lines):
        raise ValueError("final runner log did not expose the flaky test identity")


def probe(root: Path) -> None:
    with (root / "rust-toolchain.toml").open("rb") as handle:
        channel = tomllib.load(handle)["toolchain"]["channel"]
    if not isinstance(channel, str) or not re.fullmatch(r"\d+\.\d+\.\d+", channel):
        raise ValueError("the probe requires the repository's exact pinned Rust release")
    config = (root / ".config" / "nextest.toml").resolve(strict=True)
    parent = Path(tempfile.gettempdir()).resolve(strict=True)
    temporary = Path(tempfile.mkdtemp(prefix="apeireth-nextest-report-", dir=parent)).resolve(strict=True)
    if temporary.parent != parent or temporary == parent:
        raise ValueError("probe directory must be a fresh direct child of the temporary root")
    output = ""
    try:
        child(temporary, "src").mkdir()
        child(temporary, "Cargo.toml").write_text(MANIFEST, encoding="utf-8")
        child(temporary, "Cargo.lock").write_text(LOCKFILE, encoding="utf-8")
        child(temporary, "src/lib.rs").write_text(SOURCE, encoding="utf-8")
        target = child(temporary, "target")
        state = child(temporary, "attempt-state")
        report = child(temporary, "target/nextest/ci/junit.xml")
        report.parent.mkdir(parents=True)
        report.write_text('<testsuites name="cached-report"/>', encoding="utf-8")
        if not prepare_workspace_report(temporary) or report.exists():
            raise ValueError("cached JUnit evidence was not removed before the new run")
        try:
            verify_report(report, "")
        except ValueError as error:
            if "was not created" not in str(error):
                raise
        else:
            raise ValueError("missing current-run JUnit evidence was incorrectly accepted")
        env = {key: value for key, value in os.environ.items() if not key.upper().startswith("NEXTEST_")}
        env.update({
            "RUSTC_WRAPPER": "",
            "RUSTC_WORKSPACE_WRAPPER": "",
            "RUSTUP_TOOLCHAIN": channel,
            "CARGO_TARGET_DIR": str(target),
            "CARGO_TERM_COLOR": "never",
        })
        command = [
            "cargo", "nextest", "run", "--manifest-path", str(child(temporary, "Cargo.toml")),
            "--config-file", str(config), "--profile", "ci", "--offline", "--locked", "-j", "2",
        ]
        result = subprocess.run(
            command, cwd=temporary, env=env, capture_output=True, text=True,
            encoding="utf-8", errors="replace", timeout=180, check=False,
        )
        output = result.stdout + result.stderr
        if result.returncode:
            raise ValueError(f"controlled retry run exited {result.returncode}")
        if not state.is_file() or state.read_bytes() != b"2":
            raise ValueError("probe did not execute exactly one failure followed by success")
        verify_report(report, output)
    except (OSError, ValueError, ET.ParseError, subprocess.TimeoutExpired) as error:
        detail = output[-20_000:]
        if detail:
            raise ValueError(
                f"{error}\nControlled probe diagnostics (not workspace results):\n{detail}"
            ) from error
        raise
    finally:
        cleanup(temporary, parent)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--prepare-workspace-report", action="store_true",
        help="remove the expected workspace JUnit file before this run can generate fresh evidence",
    )
    args = parser.parse_args()
    try:
        root = Path(__file__).resolve().parent.parent
        if args.prepare_workspace_report:
            removed = prepare_workspace_report(root)
            state = "removed" if removed else "absent"
            print(f"PASS: workspace JUnit path prepared; prior report {state}.")
            return 0
        probe(root)
    except (OSError, ValueError, KeyError, ET.ParseError, subprocess.TimeoutExpired) as error:
        print(f"FAIL: nextest report probe: {error}", file=sys.stderr)
        return 1
    print("PASS: isolated nextest report probe retained retry identity and failed output in JUnit and logs.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
