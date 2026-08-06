"""R11 QA2 real data-truth and provenance consistency tests."""
from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

ROOT_HERE = Path(__file__).resolve().parent.parent
if str(ROOT_HERE) not in sys.path:
    sys.path.insert(0, str(ROOT_HERE))

from apeireth.r11_truth_consistency import (
    EXPECTED,
    R11Expected,
    build_git_provenance,
    check_consistency,
    check_repository,
    main,
)

ROOT = Path(__file__).resolve().parents[1]


def document_text(**overrides: object) -> str:
    values = {
        "v05": EXPECTED.v05,
        "v04": EXPECTED.v04,
        "v03": EXPECTED.v03,
        "n_modules": EXPECTED.n_modules,
        "n_tests": EXPECTED.n_tests,
        "n_commits": EXPECTED.n_commits,
        "git_head": EXPECTED.git_head,
    }
    values.update(overrides)
    return f"""# Apeireth COMPLETE OMNIBUS
| **ASI 北极星 V0.5 当前** | **{values['v05']}** (V1136 真测引擎) |
| **ASI 北极星 V0.4 当前** | **{values['v04']}** (V1101/V1102 lift 后) |
| **ASI 北极星 V0.3 当前** | **{values['v03']}** (V1074 runner) |
| **真生产 modules** | **{values['n_modules']}** |
| **真生产 tests** | **{values['n_tests']}** |
| **真 commit** | **{values['n_commits']}** (master HEAD = {values['git_head']}) |
"""


def valid_sources() -> dict[str, object]:
    return {
        "document": document_text(),
        "v1136_report": {
            "v05_total_v1136": EXPECTED.v05,
            "v04_score": EXPECTED.v04,
            "version": EXPECTED.version,
        },
        "snapshot": {
            "snapshot_id": EXPECTED.snapshot_id,
            "version": EXPECTED.version,
            "v03_score": EXPECTED.v03,
            "n_modules": EXPECTED.n_modules,
            "n_tests": EXPECTED.n_tests,
            "n_commits": EXPECTED.n_commits,
        },
        "dashboard_payload": {
            "version": EXPECTED.version,
            "real_run_summary": {"v05_total": EXPECTED.v05},
        },
        "git_provenance": {
            "git_head": EXPECTED.git_head + "0" * 32,
            "n_modules": EXPECTED.n_modules,
            "n_tests": EXPECTED.n_tests,
            "n_commits": EXPECTED.n_commits,
        },
    }


def run_with(**overrides: object):
    sources = valid_sources()
    sources.update(overrides)
    return check_consistency(**sources)


def issue_fields(report, source: str) -> set[str]:
    return {issue.field for issue in report.issues if issue.source == source}


def test_locked_expected_values_are_explicit() -> None:
    assert EXPECTED == R11Expected(
        v05=0.8595,
        v04=0.8031,
        v03=0.8964,
        n_modules=1153,
        n_tests=6394,
        n_commits=542,
        snapshot_id="snap_9c80c9165625",
        git_head="f17b7ad1",
        version="0.1.0",
    )


def test_all_consistent_sources_pass() -> None:
    report = run_with()
    assert report.passed is True
    assert report.issues == []


def test_nested_dashboard_payload_is_compared() -> None:
    report = run_with(dashboard_payload={
        "metadata": {"version": "0.1.0"},
        "real_run_summary": {"metrics": {"v05_total": 0.8595}},
    })
    assert report.passed is True
    assert report.sources["dashboard"]["v05"] == 0.8595


def test_v1136_markdown_report_is_supported() -> None:
    report = run_with(v1136_report="""# V1136
**Version**: 0.1.0
V0.4 真测当前 = **0.8031**
**V0.5 total (V1136 真测)**: 0.8595
""")
    assert report.passed is True


@pytest.mark.parametrize("actual", [0.8594, 0.8596, 0.8532])
def test_v05_drift_in_v1136_report_fails(actual: float) -> None:
    report = run_with(v1136_report={
        "v05_total_v1136": actual,
        "v04_score": EXPECTED.v04,
        "version": EXPECTED.version,
    })
    assert report.passed is False
    assert issue_fields(report, "v1136_report") == {"v05"}
    assert report.issues[0].actual == actual
    assert "no automatic overwrite" in report.issues[0].reason


def test_v04_provenance_drift_fails_even_when_v05_matches() -> None:
    report = run_with(v1136_report={
        "v05_total_v1136": EXPECTED.v05,
        "v04_score": 0.8538,
        "version": EXPECTED.version,
    })
    assert report.passed is False
    assert issue_fields(report, "v1136_report") == {"v04"}


def test_v03_snapshot_drift_fails() -> None:
    snapshot = dict(valid_sources()["snapshot"])
    snapshot["v03_score"] = 0.8963
    report = run_with(snapshot=snapshot)
    assert report.passed is False
    assert issue_fields(report, "snapshot") == {"v03"}


@pytest.mark.parametrize(
    ("field_name", "actual"),
    [("n_modules", 1154), ("n_tests", 6395), ("n_commits", 543)],
)
def test_snapshot_inventory_drift_fails(field_name: str, actual: int) -> None:
    snapshot = dict(valid_sources()["snapshot"])
    snapshot[field_name] = actual
    report = run_with(snapshot=snapshot)
    assert report.passed is False
    assert issue_fields(report, "snapshot") == {field_name}


@pytest.mark.parametrize(
    ("field_name", "actual"),
    [("n_modules", 1154), ("n_tests", 7099), ("n_commits", 557)],
)
def test_git_inventory_drift_fails(field_name: str, actual: int) -> None:
    provenance = dict(valid_sources()["git_provenance"])
    provenance[field_name] = actual
    report = run_with(git_provenance=provenance)
    assert report.passed is False
    assert issue_fields(report, "git") == {field_name}


def test_git_head_drift_fails_but_expected_prefix_is_accepted() -> None:
    mismatch = dict(valid_sources()["git_provenance"])
    mismatch["git_head"] = "c748f54e94d3707aac2a3735e508c9795c36d186"
    report = run_with(git_provenance=mismatch)
    assert report.passed is False
    assert issue_fields(report, "git") == {"git_head"}

    exact_baseline = dict(mismatch)
    exact_baseline["git_head"] = "f17b7ad1" + "a" * 32
    assert run_with(git_provenance=exact_baseline).passed is True


def test_document_drift_fails() -> None:
    report = run_with(document=document_text(v05=0.8532, n_commits=557))
    assert report.passed is False
    assert issue_fields(report, "document") == {"v05", "n_commits"}


def test_missing_required_evidence_is_explicit_failure() -> None:
    report = run_with(dashboard_payload={"version": "0.1.0"})
    assert report.passed is False
    assert issue_fields(report, "dashboard") == {"v05"}
    assert report.issues[0].reason == "required field missing"


def test_malformed_json_source_is_explicit_failure(tmp_path: Path) -> None:
    bad = tmp_path / "bad.json"
    bad.write_text("{not-json", encoding="utf-8")
    report = run_with(snapshot=bad)
    assert report.passed is False
    assert issue_fields(report, "snapshot") == {"source"}
    assert "could not parse source" in next(
        issue.reason for issue in report.issues if issue.source == "snapshot"
    )


def test_checker_never_overwrites_drifting_sources(tmp_path: Path) -> None:
    snapshot = tmp_path / "snapshot.json"
    dashboard = tmp_path / "dashboard.json"
    report_file = tmp_path / "v1136.json"
    document = tmp_path / "omnibus.md"
    snapshot.write_text(json.dumps({
        "snapshot_id": EXPECTED.snapshot_id,
        "version": EXPECTED.version,
        "v03_score": 0.1,
        "n_modules": 1,
        "n_tests": 2,
        "n_commits": 3,
    }), encoding="utf-8")
    dashboard.write_text(json.dumps({"version": "0.1.0", "v05_total": 0.1}), encoding="utf-8")
    report_file.write_text(json.dumps({
        "version": "0.1.0", "v04_score": 0.1, "v05_total_v1136": 0.1
    }), encoding="utf-8")
    document.write_text(document_text(v03=0.1), encoding="utf-8")
    before = {path: path.read_bytes() for path in (snapshot, dashboard, report_file, document)}

    result = check_consistency(
        v1136_report=report_file,
        snapshot=snapshot,
        dashboard_payload=dashboard,
        document=document,
        git_provenance=valid_sources()["git_provenance"],
    )

    assert result.passed is False
    assert before == {path: path.read_bytes() for path in before}


def test_markdown_report_names_fail_and_no_overwrite() -> None:
    provenance = dict(valid_sources()["git_provenance"])
    provenance["n_commits"] = 557
    report = run_with(git_provenance=provenance)
    markdown = report.render_markdown()
    assert "**结果**: FAIL" in markdown
    assert "自动覆盖" in markdown
    assert "`n_commits`" in markdown
    assert "`542`" in markdown
    assert "`557`" in markdown


def test_real_omnibus_parses_locked_values() -> None:
    # Real file integration: this is not a reduced fixture.
    report = run_with(document=ROOT / "APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md")
    assert issue_fields(report, "document") == set()
    assert report.sources["document"]["v05"] == 0.8595
    assert report.sources["document"]["n_commits"] == 542


def test_real_git_provenance_detects_current_repository_drift() -> None:
    # Real git integration: the current repository has advanced beyond the
    # snapshot baseline.  The assertion proves we fail rather than relabel it.
    #
    # 主 17:43 实事求是 + 主 23:44 干到底: 真生产是不停的, n_modules / n_tests /
    # n_commits 会随新 commit 持续上涨. 校验器要做的不是把"前进"判成漂移失败,
    # 而是要在 HEAD 演进 / 真分数被覆盖 / 倒退时仍然 fail-closed.
    provenance = build_git_provenance(ROOT)
    report = run_with(git_provenance=provenance)
    assert report.passed is False
    # Inventory must never regress below the locked snapshot baseline.
    assert provenance["n_modules"] >= EXPECTED.n_modules
    assert provenance["n_commits"] >= EXPECTED.n_commits
    # Head must have moved off the locked abbreviation; otherwise the snapshot
    # was rewritten under us, which is precisely the case this check guards.
    assert provenance["git_head"] != EXPECTED.git_head
    assert not str(provenance["git_head"]).lower().startswith(EXPECTED.git_head.lower())
    # All four strict sources must surface at least one drift issue.
    assert {"git_head", "n_commits", "n_tests"}.issubset(issue_fields(report, "git"))
    assert "n_modules" in issue_fields(report, "git")


def test_default_repository_check_is_fail_closed() -> None:
    report = check_repository(ROOT)
    assert report.passed is False
    assert issue_fields(report, "dashboard") == {"v05", "version"}
    assert "git_head" in issue_fields(report, "git")


def test_cli_returns_nonzero_on_real_drift(capsys: pytest.CaptureFixture[str]) -> None:
    rc = main(["--repo-root", str(ROOT)])
    output = capsys.readouterr().out
    assert rc == 1
    assert "**结果**: FAIL" in output
    assert "no source was modified" in output
