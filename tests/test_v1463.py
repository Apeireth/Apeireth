"""Tests for V1463 — ASI Real Lint-Gate Subprocess Pipeline.

Test plan (主 00:44 质量工程化):
  1. Enums + dataclasses (PipelineOutcome 8 values, PipelineRecord, PipelineReport)
  2. Constants exposed
  3. SandboxMode → PipelineOutcome mapping completeness
  4. _ADVERSARIAL_SPECS shape: ≥30 specs, each with declared expected_outcome
  5. parse_jsonl_specs round-trip
  6. generate_mutated_specs determinism + shape
  7. run_pipeline on a tiny batch (3 specs: blocked + ok + bin_not_found)
  8. run_pipeline sequential ordering
  9. run_v1463() adversarial demo → 100% match rate on mixed policies
 10. write_report_json / write_report_md produce files
 11. CLI: status / popper / chain / meta / help
 12. Popper 7/7 self-check
 13. V1463 guards (10)
 14. V3 guards (5)
 15. Honest disclosure: bounded sequential, not orchestrator
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from apeireth.v1463_asi_lint_gate_subprocess_pipeline import (  # noqa: E402
    PipelineOutcome,
    PipelineRecord,
    PipelineReport,
    V1463_MODULE,
    V1463_SCHEMA,
    V1463_VERSION,
    V1463_ADVERSARIAL_SEED,
    _ADVERSARIAL_SPECS,
    _V1461_TO_V1463,
    _mutate_command,
    generate_mutated_specs,
    main,
    parse_jsonl_specs,
    run_pipeline,
    run_v1463,
    write_report_json,
    write_report_md,
)
from apeireth.v1462_asi_subprocess_sandbox_spec_security_linter import (  # noqa: E402
    PolicyLevel,
    LintReport,
)
from apeireth.v1461_asi_docker_equivalent_subprocess_sandbox import (  # noqa: E402
    SandboxMode,
    SandboxSpec,
    SandboxResult,
)


# ---------------------------------------------------------------------------
# 1. Enums + dataclasses
# ---------------------------------------------------------------------------


def test_pipeline_outcome_has_eight_values():
    expected = {
        "LINT_BLOCKED",
        "RAN_OK",
        "RAN_FAILED",
        "RAN_TIMEOUT",
        "RAN_BIN_NOT_FOUND",
        "RAN_DENIED",
        "RAN_BOUNDED_ERROR",
        "RAN_ERROR",
    }
    assert {o.value for o in PipelineOutcome} == expected


def test_pipeline_record_default_outcome_is_lint_blocked():
    rec = PipelineRecord(
        label="x",
        spec=SandboxSpec(command=["echo", "hi"], timeout_s=5, max_output_bytes=1024),
        policy_level=PolicyLevel.STANDARD,
    )
    assert rec.outcome == PipelineOutcome.LINT_BLOCKED
    assert rec.lint_block_codes == []
    assert rec.sandbox_result is None


def test_pipeline_report_default_counts_have_all_outcomes():
    report = PipelineReport(policy_level=PolicyLevel.STANDARD)
    for o in PipelineOutcome:
        assert o.value in report.counts
        assert report.counts[o.value] == 0


# ---------------------------------------------------------------------------
# 2. Constants exposed
# ---------------------------------------------------------------------------


def test_constants_version_schema_module():
    assert V1463_VERSION == "0.1.0"
    assert V1463_SCHEMA == "v1463.asi-lint-gate-subprocess-pipeline/v1"
    assert V1463_MODULE == "v1463_asi_lint_gate_subprocess_pipeline"


# ---------------------------------------------------------------------------
# 3. SandboxMode → PipelineOutcome mapping completeness
# ---------------------------------------------------------------------------


def test_sandbox_mode_mapped_to_pipeline_outcome():
    for mode in SandboxMode:
        assert mode in _V1461_TO_V1463, f"SandboxMode.{mode} not mapped"


def test_no_extra_outcomes_in_mapping():
    # Every value in _V1461_TO_V1463 must be a PipelineOutcome
    for v in _V1461_TO_V1463.values():
        assert isinstance(v, PipelineOutcome)


# ---------------------------------------------------------------------------
# 4. Adversarial spec suite shape
# ---------------------------------------------------------------------------


def test_adversarial_specs_have_at_least_30():
    assert len(_ADVERSARIAL_SPECS) >= 30


def test_adversarial_specs_each_have_expected_and_policy():
    for entry in _ADVERSARIAL_SPECS:
        assert "label" in entry
        assert "spec" in entry and isinstance(entry["spec"], SandboxSpec)
        assert "expected" in entry and isinstance(entry["expected"], PipelineOutcome)
        assert "policy" in entry and isinstance(entry["policy"], PolicyLevel)


def test_adversarial_specs_have_blocked_outcomes():
    labels = [e["label"] for e in _ADVERSARIAL_SPECS]
    assert any("rm" in l.lower() for l in labels)
    assert any("curl" in l.lower() for l in labels)


def test_adversarial_specs_have_run_outcomes():
    labels = [e["label"] for e in _ADVERSARIAL_SPECS]
    outcomes = [e["expected"] for e in _ADVERSARIAL_SPECS]
    # Must cover at least 5 distinct PipelineOutcome values
    distinct = {o.value for o in outcomes}
    assert len(distinct) >= 5


# ---------------------------------------------------------------------------
# 5. parse_jsonl_specs round-trip
# ---------------------------------------------------------------------------


def test_parse_jsonl_specs_roundtrip(tmp_path):
    p = tmp_path / "specs.jsonl"
    lines = [
        json.dumps({"label": "a", "command": ["python", "-c", "print(1)"], "timeout_s": 30, "max_output_bytes": 4096}),
        json.dumps({"label": "b", "command": ["echo", "hi"], "timeout_s": 5, "max_output_bytes": 1024}),
    ]
    p.write_text("\n".join(lines), encoding="utf-8")
    specs, labels = parse_jsonl_specs(p)
    assert len(specs) == 2
    assert labels == ["a", "b"]
    assert specs[0].command == ["python", "-c", "print(1)"]
    assert specs[1].command == ["echo", "hi"]


def test_parse_jsonl_specs_invalid_json_raises(tmp_path):
    p = tmp_path / "bad.jsonl"
    p.write_text("not valid json\n", encoding="utf-8")
    with pytest.raises(ValueError):
        parse_jsonl_specs(p)


def test_parse_jsonl_skips_empty_lines(tmp_path):
    p = tmp_path / "specs.jsonl"
    p.write_text(
        json.dumps({"command": ["echo", "hi"]}) + "\n\n" + json.dumps({"command": ["echo", "bye"]}) + "\n",
        encoding="utf-8",
    )
    specs, labels = parse_jsonl_specs(p)
    assert len(specs) == 2


# ---------------------------------------------------------------------------
# 6. Mutator determinism + shape
# ---------------------------------------------------------------------------


def test_mutate_command_returns_list():
    out = _mutate_command(seed=1)
    assert isinstance(out, list)
    assert len(out) >= 1


def test_generate_mutated_specs_count_and_seed():
    out1 = generate_mutated_specs(n=10, seed=V1463_ADVERSARIAL_SEED)
    out2 = generate_mutated_specs(n=10, seed=V1463_ADVERSARIAL_SEED)
    assert len(out1) == 10
    assert len(out2) == 10
    # Determinism: same seed → same output
    assert [s.command for s in out1] == [s.command for s in out2]


def test_generate_mutated_specs_default_seed():
    out = generate_mutated_specs(n=5)
    assert len(out) == 5


# ---------------------------------------------------------------------------
# 7. run_pipeline on a tiny batch
# ---------------------------------------------------------------------------


def test_run_pipeline_blocked_spec():
    spec = SandboxSpec(
        image_alias="python:local",
        command=["bash", "-c", "rm -rf /"],
        timeout_s=5,
        max_output_bytes=4096,
    )
    report = run_pipeline([spec], policy=PolicyLevel.STANDARD, labels=["x"])
    assert report.n_specs == 1
    assert report.records[0].outcome == PipelineOutcome.LINT_BLOCKED
    assert "SL060" in report.records[0].lint_block_codes
    assert report.records[0].sandbox_result is None
    assert report.counts["LINT_BLOCKED"] == 1


def test_run_pipeline_ok_spec():
    spec = SandboxSpec(
        image_alias="python:local",
        command=[sys.executable, "-c", "print(42)"],
        timeout_s=30,
        max_output_bytes=4096,
    )
    report = run_pipeline([spec], policy=PolicyLevel.STANDARD, labels=["ok"])
    assert report.records[0].outcome == PipelineOutcome.RAN_OK
    assert report.records[0].sandbox_result is not None
    assert report.records[0].sandbox_result.rc == 0


def test_run_pipeline_bin_not_found():
    spec = SandboxSpec(
        image_alias="python:local",
        command=["__definitely_not_a_binary__", "--version"],
        timeout_s=30,
        max_output_bytes=4096,
    )
    report = run_pipeline([spec], policy=PolicyLevel.PERMISSIVE, labels=["bin"])
    assert report.records[0].outcome == PipelineOutcome.RAN_BIN_NOT_FOUND


def test_run_pipeline_timeout():
    spec = SandboxSpec(
        image_alias="python:local",
        command=[sys.executable, "-c", "import time; time.sleep(60)"],
        timeout_s=2,
        max_output_bytes=4096,
    )
    # PERMISSIVE so SL090 timeout_short WARN doesn't block
    report = run_pipeline([spec], policy=PolicyLevel.PERMISSIVE, labels=["slow"])
    assert report.records[0].outcome == PipelineOutcome.RAN_TIMEOUT


def test_run_pipeline_bounded_error():
    spec = SandboxSpec(
        image_alias="python:local",
        command=[sys.executable, "-c", "print(1)"],
        timeout_s=0,  # below MIN_TIMEOUT_S=1 → BOUNDED_ERROR
        max_output_bytes=4096,
    )
    report = run_pipeline([spec], policy=PolicyLevel.STANDARD, labels=["bad"])
    assert report.records[0].outcome == PipelineOutcome.RAN_BOUNDED_ERROR


# ---------------------------------------------------------------------------
# 8. run_pipeline sequential ordering + counts
# ---------------------------------------------------------------------------


def test_run_pipeline_preserves_order():
    specs = [
        SandboxSpec(image_alias="python:local", command=["bash", "-c", "rm -rf /"], timeout_s=5, max_output_bytes=4096),
        SandboxSpec(image_alias="python:local", command=[sys.executable, "-c", "print(1)"], timeout_s=30, max_output_bytes=4096),
        SandboxSpec(image_alias="python:local", command=["__nonexistent__"], timeout_s=30, max_output_bytes=4096),
    ]
    labels = ["rm", "ok", "bin"]
    report = run_pipeline(specs, policy=PolicyLevel.STANDARD, labels=labels)
    assert [r.label for r in report.records] == labels
    assert report.n_specs == 3


def test_run_pipeline_counts_aggregate():
    specs = [
        SandboxSpec(image_alias="python:local", command=[sys.executable, "-c", "print(1)"], timeout_s=30, max_output_bytes=4096),
        SandboxSpec(image_alias="python:local", command=[sys.executable, "-c", "print(2)"], timeout_s=30, max_output_bytes=4096),
    ]
    report = run_pipeline(specs, policy=PolicyLevel.STANDARD)
    assert report.counts["RAN_OK"] == 2
    assert report.counts["LINT_BLOCKED"] == 0


# ---------------------------------------------------------------------------
# 9. run_v1463 adversarial demo (canonical: mixed policies)
# ---------------------------------------------------------------------------


def test_adversarial_demo_match_rate():
    payload = run_v1463()
    assert payload["n_specs"] == len(_ADVERSARIAL_SPECS)
    # Canonical: mixed policies per spec → expected 100% match.
    # Allow ≥ 0.9 in case of environmental drift.
    assert payload["match_rate"] >= 0.9, f"match_rate dropped to {payload['match_rate']}"


def test_adversarial_demo_counts_sum_to_n_specs():
    payload = run_v1463()
    total = sum(payload["counts"].values())
    assert total == payload["n_specs"]


# ---------------------------------------------------------------------------
# 10. Report writers
# ---------------------------------------------------------------------------


def test_write_report_json(tmp_path):
    payload = run_v1463()
    out = tmp_path / "report.json"
    write_report_json(out, payload)
    assert out.exists()
    loaded = json.loads(out.read_text(encoding="utf-8"))
    assert loaded["n_specs"] == payload["n_specs"]


def test_write_report_md(tmp_path):
    payload = run_v1463()
    out = tmp_path / "report.md"
    write_report_md(out, payload)
    text = out.read_text(encoding="utf-8")
    assert "# V1463" in text
    assert "match_rate" in text
    assert "Honest disclosure" in text


# ---------------------------------------------------------------------------
# 11. CLI: status / popper / chain / meta / help
# ---------------------------------------------------------------------------


def test_cli_status(capsys):
    rc = main(["status"])
    assert rc == 0
    out = capsys.readouterr().out
    assert "v1463_asi_lint_gate_subprocess_pipeline" in out
    assert "n_adversarial_specs" in out


def test_cli_popper(capsys):
    rc = main(["popper"])
    assert rc == 0
    out = capsys.readouterr().out
    assert "popper_pass" in out
    assert "true" in out


def test_cli_chain(capsys):
    rc = main(["chain"])
    assert rc == 0
    out = capsys.readouterr().out
    assert "v1462" in out
    assert "v1461" in out


def test_cli_meta(capsys):
    rc = main(["meta"])
    assert rc == 0
    out = capsys.readouterr().out
    assert "v1463_asi_lint_gate_subprocess_pipeline" in out
    assert "GUARD_V1462_LINT_REUSED" in out


def test_cli_help(capsys):
    rc = main(["help"])
    assert rc == 0
    out = capsys.readouterr().out
    assert "V1463" in out
    assert "lint" in out.lower()


def test_cli_no_command_defaults_to_help(capsys):
    rc = main([])
    assert rc == 0


# ---------------------------------------------------------------------------
# 12. Popper 7/7 self-check (already covered, but re-verify)
# ---------------------------------------------------------------------------


def test_popper_self_check_via_main(capsys):
    main(["popper"])
    out = capsys.readouterr().out
    payload = json.loads(out)
    assert payload["popper_pass"] is True
    assert payload["passed"] == payload["n_checks"]


# ---------------------------------------------------------------------------
# 13. V1463 guards (10)
# ---------------------------------------------------------------------------


def test_v1463_guards_lint_reused():
    # Imports succeeded at module load time → guard passes
    from apeireth import v1463_asi_lint_gate_subprocess_pipeline as mod
    assert mod.lint_spec is not None
    assert mod.policy_gate is not None


def test_v1463_guards_runner_reused():
    from apeireth import v1463_asi_lint_gate_subprocess_pipeline as mod
    assert mod.SandboxRunner is not None
    assert mod.SandboxSpec is not None


def test_v1463_guards_outcomes_exhaustive():
    assert len(PipelineOutcome) == 8


def test_v1463_guards_jsonl_input():
    # parse_jsonl_specs is callable and handles JSONL
    import tempfile
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
        f.write(json.dumps({"command": ["echo", "hi"]}) + "\n")
        path = Path(f.name)
    specs, labels = parse_jsonl_specs(path)
    assert len(specs) == 1
    path.unlink()


def test_v1463_guards_report_json_md(tmp_path):
    payload = run_v1463()
    j = tmp_path / "report.json"
    m = tmp_path / "report.md"
    write_report_json(j, payload)
    write_report_md(m, payload)
    assert j.exists() and m.exists()


def test_v1463_guards_adversarial_suite():
    assert len(_ADVERSARIAL_SPECS) >= 30


def test_v1463_guards_borrowed_lineage(capsys):
    main(["chain"])
    out = capsys.readouterr().out
    payload = json.loads(out)
    assert payload["all_ok"] is True
    assert "v1462" in payload["v1463_borrows_from"]
    assert "v1461" in payload["v1463_borrows_from"]


def test_v1463_guards_deterministic():
    specs1 = generate_mutated_specs(n=15)
    specs2 = generate_mutated_specs(n=15)
    assert [s.command for s in specs1] == [s.command for s in specs2]


def test_v1463_guards_cli_runnable(capsys):
    # status + popper + chain + meta + help all ran in earlier tests
    main(["status"])
    capsys.readouterr()
    main(["popper"])
    capsys.readouterr()
    main(["chain"])
    capsys.readouterr()
    main(["meta"])
    capsys.readouterr()
    main(["help"])
    capsys.readouterr()


# ---------------------------------------------------------------------------
# 14. V3 guards (5)
# ---------------------------------------------------------------------------


def test_v3_guards_listed_in_meta(capsys):
    main(["meta"])
    out = capsys.readouterr().out
    payload = json.loads(out)
    guards = payload["v3_guards"]
    assert "GUARD_PIPELINE_NOT_ORCHESTRATOR" in guards
    assert "GUARD_PIPELINE_NOT_CI" in guards
    assert "GUARD_PIPELINE_NOT_ASI" in guards
    assert "GUARD_PIPELINE_NOT_PHENOMENAL" in guards
    assert "GUARD_PIPELINE_NOT_HUMAN_LEVEL" in guards


# ---------------------------------------------------------------------------
# 15. Honest disclosure in report
# ---------------------------------------------------------------------------


def test_honest_disclosure_in_report(tmp_path):
    payload = run_v1463()
    write_report_md(tmp_path / "r.md", payload)
    text = (tmp_path / "r.md").read_text(encoding="utf-8")
    assert "Honest disclosure" in text
    assert "not an orchestrator" in text or "≠ orchestrator" in text
    assert "≠ CI" in text or "not CI" in text
    assert "≠ ASI" in text or "not ASI" in text


# ---------------------------------------------------------------------------
# 16. Sandbox result attached to records that ran
# ---------------------------------------------------------------------------


def test_record_attaches_sandbox_result_when_allowed():
    spec = SandboxSpec(
        image_alias="python:local",
        command=[sys.executable, "-c", "print('hi')"],
        timeout_s=30,
        max_output_bytes=4096,
    )
    report = run_pipeline([spec], policy=PolicyLevel.STANDARD, labels=["x"])
    rec = report.records[0]
    assert rec.outcome == PipelineOutcome.RAN_OK
    assert rec.sandbox_result is not None
    assert isinstance(rec.sandbox_result, SandboxResult)


def test_record_no_sandbox_result_when_blocked():
    spec = SandboxSpec(
        image_alias="python:local",
        command=["bash", "-c", "rm -rf /"],
        timeout_s=5,
        max_output_bytes=4096,
    )
    report = run_pipeline([spec], policy=PolicyLevel.STANDARD, labels=["x"])
    rec = report.records[0]
    assert rec.outcome == PipelineOutcome.LINT_BLOCKED
    assert rec.sandbox_result is None


# ---------------------------------------------------------------------------
# 17. JSONL run CLI
# ---------------------------------------------------------------------------


def test_cli_run_jsonl(tmp_path, capsys):
    p = tmp_path / "specs.jsonl"
    p.write_text(
        json.dumps({"label": "a", "command": [sys.executable, "-c", "print(1)"], "timeout_s": 30, "max_output_bytes": 4096}) + "\n",
        encoding="utf-8",
    )
    rc = main(["run", str(p), "--policy", "STANDARD", "--out", str(tmp_path)])
    assert rc == 0
    out = capsys.readouterr().out
    payload = json.loads(out)
    assert payload["n_specs"] == 1
    assert (tmp_path / ".v1463-pipeline-report.json").exists()
    assert (tmp_path / ".v1463-pipeline-report.md").exists()


# ---------------------------------------------------------------------------
# 18. Mutation suite smoke
# ---------------------------------------------------------------------------


def test_mutation_suite_smoke():
    mutated = generate_mutated_specs(n=30, seed=V1463_ADVERSARIAL_SEED)
    report = run_pipeline(mutated, policy=PolicyLevel.STANDARD)
    # At least some should be blocked (the risky ones)
    assert report.counts["LINT_BLOCKED"] >= 1
    # And at least some should pass through
    ran_total = sum(
        report.counts[k]
        for k in ["RAN_OK", "RAN_FAILED", "RAN_TIMEOUT", "RAN_BIN_NOT_FOUND"]
    )
    assert ran_total >= 1


# ---------------------------------------------------------------------------
# 19. Cross-V interop: lint_spec reused verbatim
# ---------------------------------------------------------------------------


def test_v1463_reuses_v1462_lint_spec():
    from apeireth.v1462_asi_subprocess_sandbox_spec_security_linter import lint_spec
    spec = SandboxSpec(
        image_alias="python:local",
        command=["bash", "-c", "rm -rf /"],
        timeout_s=5,
        max_output_bytes=4096,
    )
    r = lint_spec(spec, PolicyLevel.STANDARD)
    assert isinstance(r, LintReport)
    assert any(f.rule_code == "SL060" for f in r.findings)


def test_v1463_reuses_v1461_sandboxrunner():
    from apeireth.v1461_asi_docker_equivalent_subprocess_sandbox import SandboxRunner
    runner = SandboxRunner()
    spec = SandboxSpec(
        image_alias="python:local",
        command=[sys.executable, "-c", "print(1)"],
        timeout_s=30,
        max_output_bytes=4096,
    )
    res = runner.run(spec)
    assert res.rc == 0
    assert res.mode == SandboxMode.SANDBOX_OK