#!/usr/bin/env python3
"""
VCP Substrate Example: IC2_file_handling (FileHandlingInvariants)

Demonstrates atomic write (tmp+rename) + sha256 + line-ending normalize.

Per V1335: File handling invariants = atomic write (tmp+rename) / sha256 /
line-ending normalize / safe-timestamp / unique path.

Run: python example_ic2_file_handling.py
"""
from __future__ import annotations

import hashlib
import sys
import tempfile
from pathlib import Path


class AtomicJsonWriteSubstrate:
    """Atomic write: tmp file + rename."""

    def write(self, path: Path, data: str) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.NamedTemporaryFile(
            mode="w",
            dir=str(path.parent),
            delete=False,
            suffix=".tmp",
            encoding="utf-8",
        ) as f:
            f.write(data)
            tmp_path = Path(f.name)
        tmp_path.replace(path)  # atomic on POSIX + Windows


def sha256_first16(path: Path) -> str:
    """SHA-256 of file contents, return first 16 hex chars."""
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()[:16]


def normalize_line_endings(text: str) -> str:
    """Normalize CRLF + CR to LF."""
    return text.replace("\r\n", "\n").replace("\r", "\n")


def _self_test() -> dict:
    checks = {}
    aw = AtomicJsonWriteSubstrate()
    test_path = Path(tempfile.gettempdir()) / "v1339_ic2_test.txt"
    aw.write(test_path, "hello world\n")
    checks["atomic_write_succeeds"] = test_path.exists()
    if test_path.exists():
        sha = sha256_first16(test_path)
        checks["sha256_format"] = len(sha) == 16
        # cleanup
        test_path.unlink()
    else:
        checks["sha256_format"] = False
    norm = normalize_line_endings("a\r\nb\rc\n")
    checks["line_ending_normalized"] = (norm == "a\nb\nc\n")
    return checks


if __name__ == "__main__":
    results = _self_test()
    for k, v in results.items():
        status = "OK" if v else "FAIL"
        print(f"  {k}: {status}")
    if not all(results.values()):
        sys.exit(1)
    print("ALL CHECKS PASS")
