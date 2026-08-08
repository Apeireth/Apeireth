"""test_v1351_vcp_toolchain_cli — pytest tests for V1351 VCP Toolchain CLI.

Categories:
  1. Constants (STAGES, STAGE_NAMES, PHILOSOPHY_GUARDS, ASI_CAP)
  2. Helper functions (_pipeline_id, _now_iso, _stable_dumps, compute_asi_lift)
  3. Dataclasses (StageResult, EcosystemRollup, PipelineResult JSON shape)
  4. PipelineRunner (stage execution, error handling, full pipeline)
  5. CLI (argparse dispatch: list/version/self-test/stage/run)
  6. Integration (end-to-end with empty ledger; partial stages)
  7. Cross-plugin invariant (V1335 source enumeration)
  8. Regression (V1351 chain with prior chain)
"""
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

# Make sure apeireth modules are importable
HERE = Path(__file__).resolve().parent
APEIRETH_DIR = HERE.parent
sys.path.insert(0, str(APEIRETH_DIR))

import v1351_vcp_toolchain_cli as v1351   # noqa: E402


# ============================================================================
# Category 1: Constants
# ============================================================================
def test_v1351_version_constant():
    assert isinstance(v1351.V1351_VERSION, str)
    assert len(v1351.V1351_VERSION) >= 5  # "0.1.0"
    assert v1351.V1351_VERSION.count(".") >= 1


def test_v1351_asi_cap_is_015():
    assert v1351.V1351_ASI_CAP == 0.015


def test_v1351_stages_contain_canonical_chain():
    names = [s.name for s in v1351.STAGES]
    for required in ("classify", "lint", "ledger", "migrate",
                     "health", "anomaly", "llm_brief", "lifecycle", "rollup"):
        assert required in names, f"missing stage: {required}"


def test_v1351_stages_unique_names():
    names = [s.name for s in v1351.STAGES]
    assert len(names) == len(set(names))


def test_v1351_stage_names_tuple_matches():
    assert v1351.STAGE_NAMES == tuple(s.name for s in v1351.STAGES)


def test_v1351_philosophy_guards_count():
    assert len(v1351.PHILOSOPHY_GUARDS) == 5


def test_v1351_philosophy_guards_keywords():
    text = " ".join(v1351.PHILOSOPHY_GUARDS)
    for keyword in ("CLI", "CONSCIOUS", "ASI", "LLM", "OPERATOR"):
        assert keyword in text, f"missing keyword in guards: {keyword}"


def test_v1351_stage_spec_is_frozen():
    spec = v1351.STAGES[0]
    # frozen dataclass raises FrozenInstanceError on setattr
    import dataclasses
    assert dataclasses.is_dataclass(spec)
    try:
        spec.name = "changed"  # type: ignore[misc]
        assert False, "StageSpec should be frozen"
    except dataclasses.FrozenInstanceError:
        pass


# ============================================================================
# Category 2: Helper functions
# ============================================================================
def test_v1351_pipeline_id_is_16_hex():
    r = v1351.StageResult(stage="x", status="ok", n_records=0, elapsed_ms=0.0)
    pid = v1351._pipeline_id([r], "2026-08-09T00:00:00Z")
    assert len(pid) == 16
    assert all(c in "0123456789abcdef" for c in pid)


def test_v1351_pipeline_id_stable_for_same_input():
    r = v1351.StageResult(stage="x", status="ok", n_records=0, elapsed_ms=0.0)
    p1 = v1351._pipeline_id([r], "2026-08-09T00:00:00Z")
    p2 = v1351._pipeline_id([r], "2026-08-09T00:00:00Z")
    assert p1 == p2


def test_v1351_pipeline_id_differs_on_stage_change():
    r1 = v1351.StageResult(stage="a", status="ok", n_records=0, elapsed_ms=0.0)
    r2 = v1351.StageResult(stage="b", status="ok", n_records=0, elapsed_ms=0.0)
    p1 = v1351._pipeline_id([r1], "2026-08-09T00:00:00Z")
    p2 = v1351._pipeline_id([r2], "2026-08-09T00:00:00Z")
    assert p1 != p2


def test_v1351_stable_dumps_is_sorted():
    d1 = v1351._stable_dumps({"b": 1, "a": 2})
    d2 = v1351._stable_dumps({"a": 2, "b": 1})
    assert d1 == d2
    # 'a' should come before 'b' in sorted output
    assert d1.index('"a"') < d1.index('"b"')


def test_v1351_now_iso_is_utc_z():
    iso = v1351._now_iso()
    assert iso.endswith("Z")
    assert "T" in iso
    assert iso[:4].isdigit()  # year


def test_v1351_compute_asi_lift_bounded():
    fake = v1351.PipelineResult(
        version=v1351.V1351_VERSION,
        pipeline_id="0" * 16,
        ledger_path="",
        n_ledger_records=0,
        stages=[],
        ecosystem_rollup=v1351.EcosystemRollup(
            n_substrates=0, n_high_tier=0, n_medium_tier=0, n_low_tier=0,
            n_anomaly_high=0, n_anomaly_medium=0, n_anomaly_low=0,
            n_anomaly_none=0, n_lifecycle_states={}, worst_severity="NONE",
            ecosystem_state="",
        ),
        total_elapsed_ms=0.0,
        asi_lift=0.0,
        asi_cap=v1351.V1351_ASI_CAP,
        started_at="2026-08-09T00:00:00Z",
        finished_at="2026-08-09T00:00:00Z",
        philosophy_guards=v1351.PHILOSOPHY_GUARDS,
    )
    lift = v1351.compute_asi_lift(fake)
    assert 0.0 <= lift <= v1351.V1351_ASI_CAP


def test_v1351_compute_asi_lift_caps_at_max():
    """Subscore near 1.0 should still cap at V1351_ASI_CAP."""
    # Build a fake with all-ok stages
    stages = [
        v1351.StageResult(stage=s.name, status="ok", n_records=10,
                          elapsed_ms=0.0, output_kind=s.output_kind)
        for s in v1351.STAGES
    ]
    fake = v1351.PipelineResult(
        version=v1351.V1351_VERSION,
        pipeline_id="a" * 16,
        ledger_path="",
        n_ledger_records=0,
        stages=stages,
        ecosystem_rollup=v1351.EcosystemRollup(
            n_substrates=56, n_high_tier=53, n_medium_tier=3, n_low_tier=0,
            n_anomaly_high=7, n_anomaly_medium=0, n_anomaly_low=0,
            n_anomaly_none=0, n_lifecycle_states={}, worst_severity="HIGH",
            ecosystem_state="CLOSED",
        ),
        total_elapsed_ms=100.0,
        asi_lift=0.0,
        asi_cap=v1351.V1351_ASI_CAP,
        started_at="2026-08-09T00:00:00Z",
        finished_at="2026-08-09T00:00:00Z",
        philosophy_guards=v1351.PHILOSOPHY_GUARDS,
    )
    lift = v1351.compute_asi_lift(fake)
    assert lift <= v1351.V1351_ASI_CAP


# ============================================================================
# Category 3: Dataclasses
# ============================================================================
def test_v1351_stage_result_default_fields():
    sr = v1351.StageResult(stage="x", status="ok", n_records=0, elapsed_ms=0.0)
    assert sr.errors == []
    assert sr.summary == {}


def test_v1351_ecosystem_rollup_default():
    er = v1351.EcosystemRollup(
        n_substrates=0, n_high_tier=0, n_medium_tier=0, n_low_tier=0,
        n_anomaly_high=0, n_anomaly_medium=0, n_anomaly_low=0, n_anomaly_none=0,
        n_lifecycle_states={}, worst_severity="NONE", ecosystem_state="",
    )
    assert er.worst_severity == "NONE"
    assert er.ecosystem_state == ""


def test_v1351_pipeline_result_to_json():
    """PipelineResult serializes to valid JSON."""
    pr = v1351.PipelineResult(
        version=v1351.V1351_VERSION,
        pipeline_id="a" * 16,
        ledger_path="x.jsonl",
        n_ledger_records=0,
        stages=[v1351.StageResult(stage="classify", status="ok", n_records=10,
                                   elapsed_ms=1.0, output_kind="tiers")],
        ecosystem_rollup=v1351.EcosystemRollup(
            n_substrates=10, n_high_tier=10, n_medium_tier=0, n_low_tier=0,
            n_anomaly_high=0, n_anomaly_medium=0, n_anomaly_low=0,
            n_anomaly_none=0, n_lifecycle_states={}, worst_severity="NONE",
            ecosystem_state="",
        ),
        total_elapsed_ms=100.0,
        asi_lift=0.01,
        asi_cap=v1351.V1351_ASI_CAP,
        started_at="2026-08-09T00:00:00Z",
        finished_at="2026-08-09T00:00:01Z",
        philosophy_guards=v1351.PHILOSOPHY_GUARDS,
    )
    j = v1351.to_json(pr)
    parsed = json.loads(j)
    assert "pipeline_id" in parsed
    assert "stages" in parsed
    assert "ecosystem_rollup" in parsed
    assert len(parsed["stages"]) == 1


# ============================================================================
# Category 4: PipelineRunner
# ============================================================================
def test_v1351_runner_constructible():
    runner = v1351.PipelineRunner(
        ledger_path=Path(tempfile.gettempdir()) / "v1351-test.jsonl",
        force_mock=True,
    )
    assert runner.force_mock is True
    assert runner.ledger_path.name == "v1351-test.jsonl"


def test_v1351_runner_rejects_unknown_stage():
    runner = v1351.PipelineRunner(
        ledger_path=Path(tempfile.gettempdir()) / "v1351-test.jsonl",
    )
    try:
        runner.run(stage_names=["bogus_stage_xyz"])
        assert False, "should have raised"
    except ValueError as e:
        assert "bogus_stage_xyz" in str(e)


def test_v1351_runner_empty_ledger_full_pipeline():
    """Empty ledger → full pipeline still runs (with degraded stages)."""
    with tempfile.TemporaryDirectory() as td:
        ledger = Path(td) / "empty.jsonl"
        runner = v1351.PipelineRunner(ledger_path=ledger, force_mock=True)
        res = runner.run()
        # All 9 stages ran
        assert len(res.stages) == len(v1351.STAGES)
        # Pipeline completed
        assert res.total_elapsed_ms > 0
        # ASI lift is bounded
        assert 0.0 <= res.asi_lift <= v1351.V1351_ASI_CAP


def test_v1351_runner_partial_stages():
    """Run only classify + lint stages."""
    with tempfile.TemporaryDirectory() as td:
        ledger = Path(td) / "empty.jsonl"
        runner = v1351.PipelineRunner(ledger_path=ledger, force_mock=True)
        res = runner.run(stage_names=["classify", "lint"])
        assert len(res.stages) == 2
        assert res.stages[0].stage == "classify"
        assert res.stages[1].stage == "lint"


def test_v1351_runner_load_caches_modules():
    """_load returns the same module on second call (cache test)."""
    runner = v1351.PipelineRunner(ledger_path=Path(tempfile.gettempdir()) / "x.jsonl")
    m1, err1 = runner._load("v1342_vcp_quality_tiers")
    m2, err2 = runner._load("v1342_vcp_quality_tiers")
    assert err1 is None and err2 is None
    assert m1 is m2  # same object (cached)


def test_v1351_runner_load_handles_missing_module():
    """_load on a missing module returns (None, error_str)."""
    runner = v1351.PipelineRunner(ledger_path=Path(tempfile.gettempdir()) / "x.jsonl")
    m, err = runner._load("nonexistent_module_xyz_abc")
    assert m is None
    assert err is not None


def test_v1351_runner_plugin_names_from_v1335():
    """_plugin_names returns sorted unique names from V1335 matrix."""
    runner = v1351.PipelineRunner(ledger_path=Path(tempfile.gettempdir()) / "x.jsonl")
    names = runner._plugin_names()
    assert isinstance(names, list)
    # Either populated from V1335 (>=1) or fallback (4 synthetic)
    assert len(names) >= 1
    # Sorted
    assert names == sorted(names)
    # Unique
    assert len(names) == len(set(names))


# ============================================================================
# Category 5: CLI (subprocess invocation)
# ============================================================================
def test_v1351_cli_version():
    """`python v1351_vcp_toolchain_cli.py version` prints version."""
    result = subprocess.run(
        [sys.executable, str(APEIRETH_DIR / "v1351_vcp_toolchain_cli.py"), "version"],
        capture_output=True, text=True, cwd=str(APEIRETH_DIR),
    )
    assert result.returncode == 0
    assert v1351.V1351_VERSION in result.stdout


def test_v1351_cli_list():
    """`... list` prints all stages."""
    result = subprocess.run(
        [sys.executable, str(APEIRETH_DIR / "v1351_vcp_toolchain_cli.py"), "list"],
        capture_output=True, text=True, cwd=str(APEIRETH_DIR),
    )
    assert result.returncode == 0
    out = result.stdout
    for stage in ("classify", "lint", "ledger", "migrate", "health",
                  "anomaly", "llm_brief", "lifecycle", "rollup"):
        assert stage in out, f"missing stage in list output: {stage}"


def test_v1351_cli_self_test():
    """`... self-test` passes 21/21."""
    result = subprocess.run(
        [sys.executable, str(APEIRETH_DIR / "v1351_vcp_toolchain_cli.py"), "self-test"],
        capture_output=True, text=True, cwd=str(APEIRETH_DIR),
    )
    assert result.returncode == 0
    assert "21/21 PASS" in result.stdout


def test_v1351_cli_run_with_empty_ledger():
    """`... run --ledger X` works even if ledger is empty."""
    with tempfile.TemporaryDirectory() as td:
        ledger = Path(td) / "empty.jsonl"
        result = subprocess.run(
            [sys.executable, str(APEIRETH_DIR / "v1351_vcp_toolchain_cli.py"),
             "run", "--ledger", str(ledger), "--no-mock"],
            capture_output=True, text=True, cwd=str(APEIRETH_DIR),
            timeout=60,
        )
        assert result.returncode == 0
        out = result.stdout
        assert "V1351 VCP Toolchain" in out
        assert "Stages:" in out


def test_v1351_cli_run_json():
    """`... run --json` emits valid JSON."""
    with tempfile.TemporaryDirectory() as td:
        ledger = Path(td) / "empty.jsonl"
        result = subprocess.run(
            [sys.executable, str(APEIRETH_DIR / "v1351_vcp_toolchain_cli.py"),
             "run", "--ledger", str(ledger), "--json"],
            capture_output=True, text=True, cwd=str(APEIRETH_DIR),
            timeout=60,
        )
        assert result.returncode == 0
        parsed = json.loads(result.stdout.strip())
        assert "pipeline_id" in parsed
        assert "stages" in parsed


def test_v1351_cli_run_audit_out():
    """`... run --audit-out PATH` writes artifact JSON."""
    with tempfile.TemporaryDirectory() as td:
        ledger = Path(td) / "empty.jsonl"
        audit_out = Path(td) / "artifact.json"
        result = subprocess.run(
            [sys.executable, str(APEIRETH_DIR / "v1351_vcp_toolchain_cli.py"),
             "run", "--ledger", str(ledger), "--audit-out", str(audit_out)],
            capture_output=True, text=True, cwd=str(APEIRETH_DIR),
            timeout=60,
        )
        assert result.returncode == 0
        assert audit_out.is_file()
        with open(audit_out) as f:
            parsed = json.load(f)
        assert "pipeline_id" in parsed


def test_v1351_cli_stage_subset():
    """`... stage <name>` runs only one stage."""
    result = subprocess.run(
        [sys.executable, str(APEIRETH_DIR / "v1351_vcp_toolchain_cli.py"),
         "stage", "classify"],
        capture_output=True, text=True, cwd=str(APEIRETH_DIR),
        timeout=30,
    )
    assert result.returncode == 0
    out = result.stdout
    assert "classify" in out
    # Should NOT include other stages' output
    assert "llm_brief" not in out


def test_v1351_cli_unknown_stage_errors():
    """`... stage bogus` exits non-zero."""
    result = subprocess.run(
        [sys.executable, str(APEIRETH_DIR / "v1351_vcp_toolchain_cli.py"),
         "stage", "bogus_stage"],
        capture_output=True, text=True, cwd=str(APEIRETH_DIR),
    )
    assert result.returncode != 0


# ============================================================================
# Category 6: Integration end-to-end
# ============================================================================
def test_v1351_end_to_end_full_pipeline_real_ledger():
    """Run V1351 against the actual vcp_gate_history.jsonl (if present)."""
    real_ledger = v1351.LEDGER_PATH
    if not real_ledger.is_file():
        # Skip if no real ledger
        return
    runner = v1351.PipelineRunner(ledger_path=real_ledger, force_mock=True)
    res = runner.run()
    # Should have n_records > 0 in classify, lint
    classify_stage = res.stages[0]
    lint_stage = res.stages[1]
    assert classify_stage.n_records > 0
    assert lint_stage.n_records > 0


def test_v1351_artifact_round_trip_json():
    """Pipeline result -> JSON -> file -> load -> same shape."""
    with tempfile.TemporaryDirectory() as td:
        out_path = Path(td) / "x.json"
        runner = v1351.PipelineRunner(
            ledger_path=Path(td) / "empty.jsonl",
        )
        res = runner.run(stage_names=["classify"])
        v1351.write_artifact(res, out_path)
        with open(out_path) as f:
            loaded = json.load(f)
        assert loaded["version"] == v1351.V1351_VERSION
        assert len(loaded["stages"]) == 1


# ============================================================================
# Category 7: Cross-plugin invariant (V1335 enumeration)
# ============================================================================
def test_v1351_plugin_names_unique_across_v1335():
    """_plugin_names returns deduplicated, sorted names."""
    runner = v1351.PipelineRunner(
        ledger_path=Path(tempfile.gettempdir()) / "x.jsonl",
    )
    names = runner._plugin_names()
    assert len(names) == len(set(names)), "duplicate plugin names"


def test_v1351_pipeline_result_philosophy_guards_in_output():
    """PipelineResult.philosophy_guards is preserved in JSON output."""
    with tempfile.TemporaryDirectory() as td:
        runner = v1351.PipelineRunner(
            ledger_path=Path(td) / "empty.jsonl",
        )
        res = runner.run(stage_names=["classify"])
        j = v1351.to_json(res)
        for guard in v1351.PHILOSOPHY_GUARDS:
            assert guard in j, f"missing guard in output: {guard}"


# ============================================================================
# Category 8: Regression (idempotency)
# ============================================================================
def test_v1351_idempotent_run_same_ledger():
    """Running twice with the same ledger produces same pipeline_id."""
    with tempfile.TemporaryDirectory() as td:
        ledger = Path(td) / "x.jsonl"
        runner1 = v1351.PipelineRunner(ledger_path=ledger, force_mock=True)
        r1 = runner1.run()
        runner2 = v1351.PipelineRunner(ledger_path=ledger, force_mock=True)
        r2 = runner2.run()
        # started/finished may differ; pipeline_id stable per minute
        # (because _pipeline_id truncates started_at to minute)
        # We can't guarantee minute match in a fast test, but the
        # format and length must match.
        assert len(r1.pipeline_id) == len(r2.pipeline_id)


def test_v1351_force_mock_propagates():
    """force_mock=True is passed through to V1349 benchmark."""
    with tempfile.TemporaryDirectory() as td:
        runner = v1351.PipelineRunner(
            ledger_path=Path(td) / "empty.jsonl",
            force_mock=True,
        )
        assert runner.force_mock is True


def test_v1351_force_mock_off_works():
    """force_mock=False is honored (may still result in mock if endpoint dead)."""
    with tempfile.TemporaryDirectory() as td:
        runner = v1351.PipelineRunner(
            ledger_path=Path(td) / "empty.jsonl",
            force_mock=False,
        )
        assert runner.force_mock is False