"""R11 data-truth consistency checks.

The checker intentionally has no repair/update path.  It compares independent
artifacts with the locked values documented by the 2026-07-30 omnibus and
reports every missing or drifting value.  A caller must explicitly decide how
to handle a failure; source files are never rewritten.
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Callable, Dict, Iterable, Mapping, Optional, Sequence, Tuple, Union

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_DOCUMENT = ROOT / "APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md"
DEFAULT_SNAPSHOT = ROOT / "artifacts" / "asi_snapshot.json"
DEFAULT_V1136_REPORT = ROOT / "reports" / "r11-technical-writer.md"

# These are the locked R11 acceptance values, not values inferred from a live
# artifact.  Updating them is a deliberate baseline change and must be reviewed.
@dataclass(frozen=True)
class R11Expected:
    v05: float = 0.8595
    v04: float = 0.8031
    v03: float = 0.8964
    n_modules: int = 1153
    n_tests: int = 6394
    n_commits: int = 542
    snapshot_id: str = "snap_9c80c9165625"
    git_head: str = "f17b7ad1"
    version: str = "0.1.0"


EXPECTED = R11Expected()

_SOURCE_FIELDS: Dict[str, Tuple[str, ...]] = {
    "document": (
        "v05", "v04", "v03", "n_modules", "n_tests", "n_commits", "git_head"
    ),
    "v1136_report": ("v05", "v04", "version"),
    "snapshot": (
        "v03", "n_modules", "n_tests", "n_commits", "snapshot_id", "version"
    ),
    "dashboard": ("v05", "version"),
    "git": ("n_modules", "n_tests", "n_commits", "git_head"),
}

_MISSING = object()
_NUMERIC_FIELDS = {"v05", "v04", "v03"}
_COUNT_FIELDS = {"n_modules", "n_tests", "n_commits"}


@dataclass(frozen=True)
class ConsistencyIssue:
    """One explicit, reproducible mismatch or missing evidence field."""

    source: str
    field: str
    expected: Any
    actual: Any
    reason: str

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class ConsistencyReport:
    """Result of a no-write cross-source comparison."""

    expected: Dict[str, Any]
    sources: Dict[str, Dict[str, Any]] = field(default_factory=dict)
    issues: list[ConsistencyIssue] = field(default_factory=list)

    @property
    def passed(self) -> bool:
        return not self.issues

    def to_dict(self) -> Dict[str, Any]:
        return {
            "passed": self.passed,
            "expected": dict(self.expected),
            "sources": self.sources,
            "issues": [issue.to_dict() for issue in self.issues],
        }

    def render_markdown(self) -> str:
        lines = [
            "# R11 QA2 — 数据真态与证据一致性校验",
            "",
            f"- **结果**: {'PASS' if self.passed else 'FAIL'}",
            "- **自动覆盖**: **禁止**（本校验器只读）",
            "",
            "## Locked expected values",
            "",
            "| field | expected |",
            "|---|---:|",
        ]
        for key, value in self.expected.items():
            lines.append(f"| `{key}` | `{value}` |")
        lines.extend(["", "## Observed sources", "", "| source | values |", "|---|---|"])
        for source, values in self.sources.items():
            compact = ", ".join(f"`{k}`=`{v}`" for k, v in values.items()) or "(none)"
            lines.append(f"| `{source}` | {compact} |")
        lines.extend(["", "## Issues", ""])
        if self.issues:
            lines.extend([
                "| source | field | expected | actual | reason |",
                "|---|---|---:|---:|---|",
            ])
            for issue in self.issues:
                reason = issue.reason.replace("|", "\\|")
                lines.append(
                    f"| `{issue.source}` | `{issue.field}` | `{issue.expected}` | "
                    f"`{issue.actual}` | {reason} |"
                )
        else:
            lines.append("No drift detected across the supplied evidence sources.")
        lines.extend([
            "",
            "## Decision",
            "",
            ("PASS: all required source fields equal the locked baseline."
             if self.passed else
             "FAIL: evidence drift or missing provenance was found; no source was modified."),
            "",
        ])
        return "\n".join(lines)


Source = Union[Mapping[str, Any], str, Path]


def _find_mapping_value(value: Any, aliases: Iterable[str]) -> Any:
    """Find the first exact key in a nested JSON-like mapping."""
    aliases = tuple(aliases)
    if isinstance(value, Mapping):
        for alias in aliases:
            if alias in value:
                return value[alias]
        for nested in value.values():
            found = _find_mapping_value(nested, aliases)
            if found is not _MISSING:
                return found
    elif isinstance(value, (list, tuple)):
        for nested in value:
            found = _find_mapping_value(nested, aliases)
            if found is not _MISSING:
                return found
    return _MISSING


def _normalise_value(field_name: str, value: Any) -> Any:
    if field_name in _NUMERIC_FIELDS:
        if isinstance(value, bool):
            raise ValueError("boolean is not a score")
        return float(value)
    if field_name in _COUNT_FIELDS:
        if isinstance(value, bool):
            raise ValueError("boolean is not a count")
        number = int(value)
        if float(value) != number:
            raise ValueError("count must be an integer")
        return number
    return str(value)


def _markdown_value(text: str, patterns: Sequence[str]) -> Any:
    for pattern in patterns:
        match = re.search(pattern, text, flags=re.IGNORECASE | re.MULTILINE)
        if match:
            return match.group(1)
    return _MISSING


def _parse_markdown(text: str) -> Dict[str, Any]:
    """Parse canonical labels from V1136/runbook/omnibus Markdown."""
    patterns: Dict[str, Sequence[str]] = {
        "v05": (
            r"V0\.5\s+total\s*[\(\uff08]\s*V1136[^\)\uff09]*[\)\uff09]\s*\**\s*[:：][^0-9\-]*([0-9]+(?:\.[0-9]+)?)",
            r"V0\.5\s+total\s*[\(\uff08]\s*V1136[^\)\uff09]*[\)\uff09]\s*\**\s*[:：]\s*[*`]*([0-9]+(?:\.[0-9]+)?)",
            r"ASI\s+北极星\s+V0\.5\s+当前[^|\n]*\|\s*[*`]*([0-9]+(?:\.[0-9]+)?)",
        ),
        "v04": (
            r"ASI\s+北极星\s+V0\.4\s+当前[^|\n]*\|\s*[*`]*([0-9]+(?:\.[0-9]+)?)",
            r"V0\.4\s+真测当前\s*=\s*[*`]*([0-9]+(?:\.[0-9]+)?)",
        ),
        "v03": (
            r"ASI\s+北极星\s+V0\.3\s+当前[^|\n]*\|\s*[*`]*([0-9]+(?:\.[0-9]+)?)",
            r"V0\.3\s+真测当前\s*=\s*[*`]*([0-9]+(?:\.[0-9]+)?)",
        ),
        "n_modules": (
            r"真生产\s+modules?\b[^\n|]*?\|\s*[*`]*([0-9]+)",
            r"真生产\s+module\s*[:=]\s*[*`]*([0-9]+)",
        ),
        "n_tests": (
            r"真生产\s+tests?\b[^\n|]*?\|\s*[*`]*([0-9]+)",
            r"真生产\s+test(?:s)?\s*[:=]\s*[*`]*([0-9]+)",
        ),
        "n_commits": (
            r"真\s+commit\b[^\n|]*?\|\s*[*`]*([0-9]+)",
            r"真\s+commit\s*[:=]\s*[*`]*([0-9]+)",
        ),
        "snapshot_id": (
            r"snapshot_id\s*[:=]\s*[`*]*([A-Za-z0-9_-]+)",
            r"\b(snap_[A-Za-z0-9]+)\b",
        ),
        "git_head": (
            r"Master\s+HEAD\s*=\s*[`*]*([0-9a-f]{7,40})",
            r"master\s+HEAD[^\n]*?([0-9a-f]{7,40})",
        ),
        "version": (
            r"(?:\*\*)?Version(?:\*\*)?\s*[:=]\s*[`*]*([0-9]+\.[0-9]+\.[0-9]+)",
        ),
    }
    parsed: Dict[str, Any] = {}
    for field_name, field_patterns in patterns.items():
        value = _markdown_value(text, field_patterns)
        if value is not _MISSING:
            try:
                parsed[field_name] = _normalise_value(field_name, value)
            except (TypeError, ValueError):
                # Keep the malformed value so validation reports a useful drift.
                parsed[field_name] = value
    return parsed


def _parse_source(source: Source) -> Dict[str, Any]:
    if isinstance(source, Mapping):
        raw: Any = source
    elif isinstance(source, (bytes, bytearray)):
        text = source.decode("utf-8-sig")
        return _parse_markdown(text)
    elif isinstance(source, str):
        path = Path(source)
        if path.is_file():
            text = path.read_text(encoding="utf-8-sig")
            if path.suffix.lower() == ".json":
                raw = json.loads(text)
            else:
                return _parse_markdown(text)
        elif path.suffix.lower() == ".json":
            try:
                raw = json.loads(source)
            except json.JSONDecodeError:
                return _parse_markdown(source)
        else:
            return _parse_markdown(source)
    else:
        path = Path(source)
        text = path.read_text(encoding="utf-8-sig")
        if path.suffix.lower() == ".json":
            raw = json.loads(text)
        else:
            return _parse_markdown(text)

    aliases = {
        "v05": ("v05", "v05_score", "v05_total", "v05_total_v1136"),
        "v04": ("v04", "v04_score", "v04_base"),
        "v03": ("v03", "v03_score", "level_score"),
        "n_modules": ("n_modules", "modules"),
        "n_tests": ("n_tests", "tests"),
        "n_commits": ("n_commits", "commits"),
        "snapshot_id": ("snapshot_id",),
        "git_head": ("git_head", "head", "commit_sha", "sha"),
        "version": ("version", "runner_version"),
    }
    parsed: Dict[str, Any] = {}
    for field_name, names in aliases.items():
        value = _find_mapping_value(raw, names)
        if value is not _MISSING:
            try:
                parsed[field_name] = _normalise_value(field_name, value)
            except (TypeError, ValueError):
                parsed[field_name] = value
    return parsed


def _expected_dict(expected: R11Expected) -> Dict[str, Any]:
    return asdict(expected)


def _same_value(field_name: str, actual: Any, expected: Any) -> bool:
    if field_name in _NUMERIC_FIELDS:
        try:
            return float(actual) == float(expected)
        except (TypeError, ValueError):
            return False
    if field_name in _COUNT_FIELDS:
        try:
            return int(actual) == int(expected) and float(actual) == int(actual)
        except (TypeError, ValueError):
            return False
    if field_name == "git_head":
        # Omnibus records an abbreviated commit while git returns the full SHA.
        return str(actual).lower().startswith(str(expected).lower())
    return str(actual) == str(expected)


def _git_run(repo_root: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", *args], cwd=str(repo_root), capture_output=True, text=True, check=False
    )
    if result.returncode:
        raise RuntimeError(result.stderr.strip() or f"git {' '.join(args)} failed")
    return result.stdout.strip()


def build_git_provenance(repo_root: Union[str, Path] = ROOT) -> Dict[str, Any]:
    """Collect read-only git provenance and tracked test/module counts."""
    root = Path(repo_root)
    head = _git_run(root, "rev-parse", "HEAD")
    commit_count = int(_git_run(root, "rev-list", "--count", "HEAD"))
    module_paths = _git_run(root, "ls-files", "--", "apeireth/v*.py").splitlines()
    test_paths = _git_run(root, "ls-files", "--", "tests/test_*.py").splitlines()
    test_pattern = re.compile(r"^\s*(?:async\s+)?def\s+test_[A-Za-z0-9_]+\s*\(", re.MULTILINE)
    test_count = 0
    for relative_path in test_paths:
        test_count += len(test_pattern.findall((root / relative_path).read_text(encoding="utf-8")))
    return {
        "git_head": head,
        "n_commits": commit_count,
        "n_modules": len(module_paths),
        "n_tests": test_count,
    }


def _add_source_issues(
    report: ConsistencyReport,
    source_name: str,
    source: Optional[Source],
    expected: R11Expected,
) -> None:
    required = _SOURCE_FIELDS[source_name]
    if source is None:
        for field_name in required:
            report.issues.append(ConsistencyIssue(
                source_name, field_name, getattr(expected, field_name), None,
                "source not supplied; strict evidence is required",
            ))
        report.sources[source_name] = {}
        return
    try:
        values = _parse_source(source)
    except Exception as exc:
        report.sources[source_name] = {}
        report.issues.append(ConsistencyIssue(
            source_name, "source", "readable evidence", None,
            f"could not parse source: {type(exc).__name__}: {exc}",
        ))
        return
    report.sources[source_name] = values
    for field_name in required:
        expected_value = getattr(expected, field_name)
        if field_name not in values:
            report.issues.append(ConsistencyIssue(
                source_name, field_name, expected_value, None, "required field missing",
            ))
        elif not _same_value(field_name, values[field_name], expected_value):
            report.issues.append(ConsistencyIssue(
                source_name, field_name, expected_value, values[field_name],
                "drift from locked baseline; no automatic overwrite performed",
            ))


def check_consistency(
    *,
    v1136_report: Optional[Source],
    snapshot: Optional[Source],
    dashboard_payload: Optional[Source],
    document: Optional[Source],
    git_provenance: Optional[Mapping[str, Any]] = None,
    repo_root: Union[str, Path] = ROOT,
    expected: R11Expected = EXPECTED,
) -> ConsistencyReport:
    """Compare all supplied evidence sources against the locked R11 baseline.

    This function is intentionally pure with respect to supplied artifacts: it
    only reads them.  ``git_provenance`` can be injected by tests or CI; when
    omitted it is collected from ``repo_root`` via read-only git commands.
    """
    report = ConsistencyReport(expected=_expected_dict(expected))
    _add_source_issues(report, "document", document, expected)
    _add_source_issues(report, "v1136_report", v1136_report, expected)
    _add_source_issues(report, "snapshot", snapshot, expected)
    _add_source_issues(report, "dashboard", dashboard_payload, expected)
    if git_provenance is None:
        try:
            git_provenance = build_git_provenance(repo_root)
        except Exception as exc:
            report.sources["git"] = {}
            report.issues.append(ConsistencyIssue(
                "git", "source", "readable git provenance", None,
                f"could not collect provenance: {type(exc).__name__}: {exc}",
            ))
            git_provenance = None
    _add_source_issues(report, "git", git_provenance, expected)
    return report


def check_repository(repo_root: Union[str, Path] = ROOT) -> ConsistencyReport:
    """Run the strict check using the repository's current evidence files."""
    root = Path(repo_root)
    document = root / DEFAULT_DOCUMENT.name
    snapshot = root / "artifacts" / DEFAULT_SNAPSHOT.name
    v1136_report = root / "reports" / DEFAULT_V1136_REPORT.name
    return check_consistency(
        v1136_report=v1136_report if v1136_report.exists() else None,
        snapshot=snapshot if snapshot.exists() else None,
        dashboard_payload=None,
        document=document if document.exists() else None,
        repo_root=root,
    )


def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="R11 strict data-truth consistency check")
    parser.add_argument("--v1136-report", type=Path, default=None)
    parser.add_argument("--snapshot", type=Path, default=None)
    parser.add_argument("--dashboard", dest="dashboard_payload", type=Path, default=None)
    parser.add_argument("--document", type=Path, default=DEFAULT_DOCUMENT)
    parser.add_argument("--repo-root", type=Path, default=ROOT)
    args = parser.parse_args(argv)
    if args.v1136_report is None and args.snapshot is None and args.dashboard_payload is None:
        result = check_repository(args.repo_root)
    else:
        result = check_consistency(
            v1136_report=args.v1136_report,
            snapshot=args.snapshot,
            dashboard_payload=args.dashboard_payload,
            document=args.document,
            repo_root=args.repo_root,
        )
    print(result.render_markdown())
    return 0 if result.passed else 1


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
