#!/usr/bin/env python3
"""Fail when the current Cargo workspace reaches code under legacy/."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
LEGACY = (ROOT / "legacy").resolve()


def inside(path: Path, parent: Path) -> bool:
    try:
        path.relative_to(parent)
    except ValueError:
        return False
    return True


def metadata() -> dict:
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    if result.returncode:
        sys.stderr.write(result.stderr)
        raise SystemExit(result.returncode)
    return json.loads(result.stdout)


def main() -> int:
    data = metadata()
    packages = {package["id"]: package for package in data["packages"]}
    workspace = set(data["workspace_members"])
    manifest_violations: list[tuple[str, str, str, str]] = []

    # Cargo metadata includes normal, dev, and build dependencies in this
    # list, including target-specific dependencies.
    for package_id in workspace:
        package = packages[package_id]
        consumer = Path(package["manifest_path"]).resolve()
        for dependency in package["dependencies"]:
            dependency_path = dependency.get("path")
            if not dependency_path:
                continue
            resolved = Path(dependency_path).resolve()
            if inside(resolved, LEGACY):
                manifest_violations.append(
                    (
                        str(consumer),
                        dependency["name"],
                        dependency.get("kind") or "normal",
                        str(resolved),
                    )
                )

    # Traverse the resolved graph as a second, transitive guard. This catches
    # a current package that depends on an innocent-looking crate whose own
    # path dependency crosses into legacy/.
    nodes = {node["id"]: node for node in data.get("resolve", {}).get("nodes", [])}
    reachable = set(workspace)
    pending = list(workspace)
    while pending:
        package_id = pending.pop()
        for dependency_id in nodes.get(package_id, {}).get("dependencies", []):
            if dependency_id not in reachable:
                reachable.add(dependency_id)
                pending.append(dependency_id)

    transitive_violations = []
    for package_id in reachable:
        package = packages.get(package_id)
        if package is None:
            continue
        manifest = Path(package["manifest_path"]).resolve()
        if inside(manifest, LEGACY):
            transitive_violations.append((package["name"], str(manifest)))

    print(f"workspace members: {len(workspace)}")
    print(f"path dependency violations: {len(manifest_violations)}")
    print(f"transitive legacy packages: {len(transitive_violations)}")

    for consumer, name, kind, resolved in manifest_violations:
        print(f"PATH VIOLATION: {consumer} -> {name} ({kind}) -> {resolved}")
    for name, manifest in transitive_violations:
        print(f"GRAPH VIOLATION: {name} -> {manifest}")

    return 1 if manifest_violations or transitive_violations else 0


if __name__ == "__main__":
    raise SystemExit(main())
