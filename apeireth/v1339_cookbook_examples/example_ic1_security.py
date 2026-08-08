#!/usr/bin/env python3
"""
VCP Substrate Example: IC1_security (SecurityInvariants)

Demonstrates path-traversal guard + URL-scheme validation + structured error.

Per V1335: Security invariants = fail() exit-0 / path-traversal guard /
url-scheme validation / input validation.

Run: python example_ic1_security.py
"""
from __future__ import annotations

import sys
from pathlib import Path
from urllib.parse import urlparse


class PathSanitizationSubstrate:
    """Reject path-traversal + symlink escapes."""

    def sanitize(self, path: str) -> str:
        if ".." in path:
            raise ValueError(f"path traversal detected: {path}")
        p = Path(path).resolve()
        # Allow only relative paths within current directory
        if p.is_absolute():
            # Only allow absolute paths inside /allowed/ (Unix-style example)
            if not str(p).replace("\\", "/").startswith("/allowed/") and not str(p).startswith(str(Path.cwd())):
                raise ValueError(f"path outside allowed root: {p}")
        return str(p)


def validate_url_scheme(url: str, allowed: tuple = ("http", "https")) -> str:
    """Reject URLs with disallowed schemes (e.g., file://, javascript:)."""
    parsed = urlparse(url)
    if parsed.scheme not in allowed:
        raise ValueError(f"disallowed URL scheme: {parsed.scheme}")
    return url


def _self_test() -> dict:
    """Probe-only self-test."""
    checks = {}
    ps = PathSanitizationSubstrate()
    # Should pass
    try:
        ps.sanitize("normal/file.txt")
        checks["normal_path_passes"] = True
    except ValueError:
        checks["normal_path_passes"] = False
    # Should fail
    try:
        ps.sanitize("../etc/passwd")
        checks["traversal_blocks"] = False  # should NOT pass
    except ValueError:
        checks["traversal_blocks"] = True  # should pass
    # URL scheme
    try:
        validate_url_scheme("javascript:alert(1)")
        checks["js_scheme_blocks"] = False
    except ValueError:
        checks["js_scheme_blocks"] = True
    return checks


if __name__ == "__main__":
    results = _self_test()
    for k, v in results.items():
        status = "OK" if v else "FAIL"
        print(f"  {k}: {status}")
    if not all(results.values()):
        sys.exit(1)
    print("ALL CHECKS PASS")
