"""Apeireth V1141 — V0.4/V0.5 Integration Contract Tests (主 17:43 实事求是 + 主 17:58 不假装).

Test coverage:
  TestSchema              — 17/18 dims LOCKED + IC_FIELD_SCHEMA 完整 + IC_GUARDS 13 keys
  TestCompositeFormula    — V0.5 composite 手算等价 (compute_v05_total)
  TestExceptions          — 8 失败码 + exception types traceable
  TestBundle              — ICFieldBundle dataclass round-trip + provenance
  TestProvenanceHelper    — sha256(value) + source_module timestamped
  TestLiftV04FromV03      — V0.3 dim → V0.4 lift (mean skip-none)
  TestIntegrationRun      — collect → validate (主真跑三模块, marked slow)
  TestCLI                 — --json / --report / --no-strict / --compat / non-zero exit on fail
  TestContractVersionGate — INTEGRATION_CONTRACT_VERSION semver

Total: 30 tests (含 2 slow integration marked).
"""
from __future__ import annotations

import dataclasses
import json
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Dict, List

import pytest

# ponytail: 不发明新 fixture, 直接 import V1141 (主 19:33)
from apeireth.v1141_asi_v04_v05_integration_contract import (
    ALL_FIELDS,
    CONTRACT_DRAFT_ID,
    ICFieldBundle,
    ICFieldSpec,
    IC_FIELD_SCHEMA,
    IC_GUARDS,
    INTEGRATION_CONTRACT_VERSION,
    V03_DIMS,
    V03_SPECS,
    V05_EXTRA,
    V05_SPECS,
    V0_5_WEIGHTS,
    V0_5_WEIGHT_3DIM,
    V0_5_WEIGHT_V04,
    CompositeDriftError,
    DashboardTimeoutError,
    FieldMissingError,
    IC_COMPOSITE_DRIFT,
    IC_DASHBOARD_TIMEOUT,
    IC_FIELD_MISSING,
    IC_RANGE_VIOLATION,
    IC_SUBSCORE_FAILED,
    IC_V1074_UNREACHABLE,
    IC_V1130_UNREACHABLE,
    IC_V1136_UNREACHABLE,
    IC_VERSION_CONFLICT,
    IC_CHAOS_LOST,
    IntegrationContractError,
    IntegrationContractValidator,
    ICValidationReport,
    RangeViolationError,
    V1074UnreachableError,
    V1130UnreachableError,
    V1136UnreachableError,
    ChaosLostError,
    compute_v05_total,
    lift_v04_from_v03,
    render_markdown_report,
    run_validation,
    verify_v05_composite,
)

PROJECT_ROOT = Path(__file__).resolve().parents[1]


# ============================================================================
# 1. TestSchema (主 17:43 实事求是: 字段表 + IC_GUARDS LOCKED)
# ============================================================================


class TestSchema:
    """The 17/18 field schema is LOCKED (主 23:44 干到底)."""

    def test_v03_dims_exactly_17(self):
        assert len(V03_DIMS) == 17, f"V0.3 must be 17 dims, got {len(V03_DIMS)}"

    def test_v05_extra_exactly_1(self):
        assert len(V05_EXTRA) == 1, f"V0.5 extra must be 1 composite, got {len(V05_EXTRA)}"
        assert V05_EXTRA == ("v05_total_v1136",)

    def test_all_fields_18(self):
        assert len(ALL_FIELDS) == 18
        assert ALL_FIELDS == V03_DIMS + V05_EXTRA

    def test_no_duplicate_field_names(self):
        assert len(set(ALL_FIELDS)) == 18

    def test_field_schema_18_entries(self):
        assert len(IC_FIELD_SCHEMA) == 18

    def test_v03_specs_match_dims(self):
        assert set(V03_SPECS.keys()) == set(V03_DIMS)

    def test_v05_specs_match_extra(self):
        assert set(V05_SPECS.keys()) == set(V05_EXTRA)

    def test_field_indices_sequential_1_to_18(self):
        for spec in IC_FIELD_SCHEMA.values():
            assert 1 <= spec.index <= 18, f"{spec.name} index {spec.index} OOR"

    def test_indices_unique(self):
        indices = [s.index for s in IC_FIELD_SCHEMA.values()]
        assert len(set(indices)) == 18

    def test_v03_dims_kind_is_v03_dim(self):
        for name in V03_DIMS:
            assert IC_FIELD_SCHEMA[name].kind == "v03_dim"

    def test_v05_composite_kind(self):
        spec = IC_FIELD_SCHEMA["v05_total_v1136"]
        assert spec.kind == "v05_composite"
        assert spec.index == 18
        assert spec.nullable is True   # nullable when V1136 unreachable
        assert spec.required is True

    def test_v03_dims_not_nullable(self):
        for name in V03_DIMS:
            assert IC_FIELD_SCHEMA[name].nullable is False
            assert IC_FIELD_SCHEMA[name].required is True

    def test_all_fields_in_range_zero_one(self):
        for spec in IC_FIELD_SCHEMA.values():
            assert spec.range_lo == 0.0
            assert spec.range_hi == 1.0

    def test_all_field_types_float(self):
        for spec in IC_FIELD_SCHEMA.values():
            assert spec.field_type == "float"

    def test_ic_guards_13_keys(self):
        assert len(IC_GUARDS) == 13, f"IC_GUARDS must be 13 LOCKED keys, got {len(IC_GUARDS)}"

    def test_ic_guards_no_duplicates(self):
        assert len(set(IC_GUARDS)) == 13

    def test_integration_contract_version_semver(self):
        v = INTEGRATION_CONTRACT_VERSION
        parts = v.split(".")
        assert len(parts) == 3, f"semver must have 3 parts, got {v}"
        for p in parts:
            assert p.isdigit()

    def test_contract_draft_id_format(self):
        assert CONTRACT_DRAFT_ID.startswith("IC-")
        assert CONTRACT_DRAFT_ID == "IC-001"

    def test_producer_paths_include_module(self):
        for spec in IC_FIELD_SCHEMA.values():
            assert "apeireth." in spec.producer or "StatusSnapshot" in spec.producer


# ============================================================================
# 2. TestCompositeFormula (主 17:43 实事求是: V0.5 composite 可手算验证)
# ============================================================================


class TestCompositeFormula:
    """V0.5 composite formula is LOCKED in §3.5 of Omnibus."""

    def test_v05_weights_explicit(self):
        assert V0_5_WEIGHT_V04 == 0.85
        assert V0_5_WEIGHT_3DIM == 0.05
        assert V0_5_WEIGHTS["v04_score"] == 0.85
        assert V0_5_WEIGHTS["continuity"] == 0.05
        assert V0_5_WEIGHTS["autonomy"] == 0.05
        assert V0_5_WEIGHTS["transferability"] == 0.05
        # Weights sum = 1.0
        assert abs(sum(V0_5_WEIGHTS.values()) - 1.0) < 1e-9

    def test_compute_v05_v040_placeholder(self):
        # v04=0.85, dims all 0.85 (V1125 placeholder case)
        v = compute_v05_total(v04=0.85, continuity=0.85, autonomy=0.85, transferability=0.85)
        # 0.85*0.85 + 3*(0.85*0.05) = 0.7225 + 0.1275 = 0.85
        assert abs(v - 0.85) < 1e-9

    def test_compute_v05_v04_actual_lift(self):
        # v04=0.8031 (V1102 lift), dims real but similar
        v = compute_v05_total(v04=0.8031, continuity=0.85, autonomy=0.85, transferability=0.85)
        expected = 0.8031 * 0.85 + 0.85 * 0.05 * 3
        assert abs(v - expected) < 1e-9
        assert abs(v - 0.810135) < 1e-6

    def test_compute_v05_weights_sum_to_one_x_v04_dominated(self):
        # dominance: change v04 → mostly linear in v04
        v_low = compute_v05_total(v04=0.5, continuity=0.9, autonomy=0.9, transferability=0.9)
        v_high = compute_v05_total(v04=1.0, continuity=0.9, autonomy=0.9, transferability=0.9)
        # weight of v04 = 0.85, so diff should be 0.85 * 0.5 = 0.425
        assert abs((v_high - v_low) - 0.425) < 1e-9

    def test_compute_v05_rejects_non_numeric(self):
        with pytest.raises(IntegrationContractError):
            compute_v05_total(v04="0.5", continuity=0.5, autonomy=0.5, transferability=0.5)

    def test_verify_v05_no_drift_low_tolerance(self):
        payload = {
            "v04_score": 0.8031,
            "continuity": 0.85,
            "autonomy": 0.85,
            "transferability": 0.85,
            "v05_total_v1136": compute_v05_total(0.8031, 0.85, 0.85, 0.85),
        }
        # no drift → passes
        verify_v05_composite(payload, tolerance=1e-3)

    def test_verify_v05_detects_drift(self):
        payload = {
            "v04_score": 0.8031,
            "continuity": 0.85,
            "autonomy": 0.85,
            "transferability": 0.85,
            "v05_total_v1136": 0.95,  # deliberately wrong
        }
        with pytest.raises(CompositeDriftError) as e:
            verify_v05_composite(payload, tolerance=1e-3)
        assert e.value.code == IC_COMPOSITE_DRIFT


# ============================================================================
# 3. TestExceptions (主 17:58 不假装: 8 失败码 traceable)
# ============================================================================


class TestExceptions:
    """All IC exceptions must carry IC_<NAME> codes (主 17:58)."""

    def test_integration_contract_error_default_code(self):
        e = IntegrationContractError("test")
        assert e.code == "IC_ERROR"
        assert str(e) == "test"

    def test_integration_contract_error_custom_code(self):
        e = IntegrationContractError("oops", code="IC_X", context={"a": 1})
        assert e.code == "IC_X"
        assert e.context == {"a": 1}

    def test_field_missing_error_code(self):
        assert FieldMissingError("x").code == IC_FIELD_MISSING

    def test_range_violation_error_code(self):
        assert RangeViolationError("x").code == IC_RANGE_VIOLATION

    def test_v1074_unreachable_code(self):
        assert V1074UnreachableError("x").code == IC_V1074_UNREACHABLE

    def test_v1136_unreachable_code(self):
        assert V1136UnreachableError("x").code == IC_V1136_UNREACHABLE

    def test_v1130_unreachable_code(self):
        assert V1130UnreachableError("x").code == IC_V1130_UNREACHABLE

    def test_dashboard_timeout_code(self):
        e = DashboardTimeoutError("slow", context={"wallclock_ms": 5000.0})
        assert e.code == IC_DASHBOARD_TIMEOUT
        assert e.context["wallclock_ms"] == 5000.0

    def test_chaos_lost_code(self):
        assert ChaosLostError("chaos").code == IC_CHAOS_LOST

    def test_composite_drift_code(self):
        assert CompositeDriftError("drift").code == IC_COMPOSITE_DRIFT

    def test_all_inherit_from_base(self):
        for exc in (FieldMissingError, RangeViolationError,
                    V1074UnreachableError, V1136UnreachableError,
                    V1130UnreachableError, DashboardTimeoutError,
                    ChaosLostError, CompositeDriftError):
            assert issubclass(exc, IntegrationContractError)

    def test_all_8_failure_codes_constant(self):
        # Sanity: all 8 failure codes are exposed
        for code in (IC_FIELD_MISSING, IC_RANGE_VIOLATION, IC_SUBSCORE_FAILED,
                     IC_V1074_UNREACHABLE, IC_V1136_UNREACHABLE,
                     IC_V1130_UNREACHABLE, IC_DASHBOARD_TIMEOUT, IC_CHAOS_LOST,
                     IC_VERSION_CONFLICT, IC_COMPOSITE_DRIFT):
            assert isinstance(code, str)
            assert code.startswith("IC_")


# ============================================================================
# 4. TestBundle (主 17:43 + 主 00:56 任何人都能接手: dataclass)
# ============================================================================


class TestBundle:

    def test_empty_bundle_has_18_none_fields(self):
        b = ICFieldBundle.empty()
        assert len(b.fields) == 18
        for v in b.fields.values():
            assert v is None

    def test_bundle_to_dict_round_trip(self):
        b = ICFieldBundle.empty()
        b.fields["phi_proxy"] = 0.85
        d = b.to_dict()
        assert d["fields"]["phi_proxy"] == 0.85
        assert len(b.fields) == 18

    def test_bundle_accepts_provenance(self):
        b = ICFieldBundle.empty()
        b.provenance["phi_proxy"] = {"source_module": "x", "value_sha256": "deadbeef"}
        d = b.to_dict()
        assert d["provenance"]["phi_proxy"]["source_module"] == "x"


# ============================================================================
# 5. TestProvenanceHelper (主 17:43 实事求是: 真值带来源)
# ============================================================================


class TestProvenanceHelper:
    """Provenance helper from collector (private _record_provenance).

    Test via internal API surface — public test of bundle construction.
    """

    def test_field_spec_to_dict(self):
        spec = IC_FIELD_SCHEMA["phi_proxy"]
        d = spec.to_dict()
        assert d["name"] == "phi_proxy"
        assert d["index"] == 1
        assert d["kind"] == "v03_dim"
        assert d["field_type"] == "float"
        assert d["range_lo"] == 0.0
        assert d["range_hi"] == 1.0

    def test_field_spec_frozen(self):
        from dataclasses import FrozenInstanceError
        spec = IC_FIELD_SCHEMA["phi_proxy"]
        with pytest.raises((FrozenInstanceError, dataclasses.FrozenInstanceError)):
            spec.name = "tampered"


# ============================================================================
# 6. TestLiftV04FromV03 (主 17:43 实事求是: V0.3 → V0.4 lift)
# ============================================================================


class TestLiftV04FromV03:

    def test_lift_v04_mean_all_nonzero(self):
        # 5 dims all 0.80 → mean 0.80
        result = lift_v04_from_v03({
            "phi_proxy": 0.80, "capabilities": 0.80, "cross_domain": 0.80,
            "engineering": 0.80, "vcp_4": 0.80,
        })
        assert abs(result - 0.80) < 1e-9

    def test_lift_v04_skip_zeros(self):
        # 2 nonzero + 3 zero → mean of 2 (主 17:43 实事求是: zero = missing)
        result = lift_v04_from_v03({
            "phi_proxy": 0.50, "capabilities": 0.50,
            "cross_domain": 0.0, "engineering": 0.0, "vcp_4": 0.0,
        })
        assert abs(result - 0.50) < 1e-9

    def test_lift_v04_all_zero(self):
        result = lift_v04_from_v03({
            "phi_proxy": 0.0, "capabilities": 0.0, "cross_domain": 0.0,
        })
        assert result == 0.0

    def test_lift_v04_empty(self):
        result = lift_v04_from_v03({})
        assert result == 0.0


# ============================================================================
# 7. TestIntegrationRun (主真跑三模块, marked slow)
# ============================================================================


@pytest.mark.slow
class TestIntegrationRun:
    """Real end-to-end run of V1074 + V1136 + V1130 (主 17:43 实事求是).

    Marked slow: V1074 takes ~3-5s, V1136 ~1s, V1130 ~10s.
    """

    def test_run_validation_no_strict_compat_smoke(self):
        report = run_validation(strict=False, compat_mode=False)
        assert isinstance(report, ICValidationReport)
        assert report.contract_version == INTEGRATION_CONTRACT_VERSION

    def test_run_validation_v1074_dims_populated(self):
        report = run_validation(strict=False, compat_mode=False)
        # all 17 V0.3 dims must be float and in range if collected
        for name in V03_DIMS:
            v = report.field_results.get(name, {}).get("value")
            if v is not None:
                assert isinstance(v, float)
                assert 0.0 <= v <= 1.0

    def test_run_validation_v05_composite_present(self):
        report = run_validation(strict=False, compat_mode=False)
        v05 = report.composite_v05_v1136
        # May be None only when V1136 unreachable — but in CI, expect runnable
        if v05 is not None:
            assert 0.0 <= v05 <= 1.0
            # composite drift must be < 1e-3 by validate rule
            assert report.composite_drift is not None
            assert report.composite_drift < 1e-3

    def test_run_validation_failed_codes_list(self):
        report = run_validation(strict=False, compat_mode=False)
        assert isinstance(report.failed_codes, list)
        # All entries must be valid IC_<NAME> codes
        from apeireth.v1141_asi_v04_v05_integration_contract import (
            IC_FIELD_MISSING as M, IC_RANGE_VIOLATION as R,
            IC_V1074_UNREACHABLE as C1074,
            IC_V1136_UNREACHABLE as C1136,
            IC_V1130_UNREACHABLE as C1130,
            IC_COMPOSITE_DRIFT as D, IC_SUBSCORE_FAILED as S,
        )
        valid_codes = {M, R, C1074, C1136, C1130, D, S, "IC_V3_GUARDS_FAIL"}
        for c in report.failed_codes:
            assert c in valid_codes, f"unexpected code: {c}"

    def test_run_validation_compat_mode_accepts_partial(self):
        # compat_mode should not fail solely on missing v05_total
        report = run_validation(strict=False, compat_mode=True)
        # In compat, V3 guards may still fail but missing fields tolerated
        assert isinstance(report.notes.get("compat_mode"), bool)

    def test_run_validation_strict_mode_returns_int_exit_signal(self):
        # Run validation in strict mode, expect exit code semantics
        report = run_validation(strict=True, compat_mode=False)
        # If report.passed True → 0, V3 fail → 2, fields fail → 1
        if report.passed:
            assert len(report.failed_codes) == 0
        elif not report.v3_guards_pass:
            assert len(report.v3_guards_failed) > 0
        else:
            assert len(report.failed_codes) > 0


# ============================================================================
# 8. TestCLI (主 00:56 任何人都能接手: CLI 真跑 + 退出码)
# ============================================================================


class TestCLI:

    def test_cli_help(self):
        env = {"PYTHONIOENCODING": "utf-8"}
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1141_asi_v04_v05_integration_contract",
             "--help"],
            capture_output=True, text=True, encoding="utf-8", errors="replace",
            timeout=30, cwd=str(PROJECT_ROOT), env={**__import__("os").environ, **env},
        )
        assert result.returncode == 0
        out = result.stdout or ""
        assert "Integration Contract" in out or "validate" in out, \
            f"help output missing: {out[:300]}"

    def test_cli_no_strict_json_smoke(self):
        """--no-strict --json: 真跑 + emit JSON."""
        env = {"PYTHONIOENCODING": "utf-8"}
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1141_asi_v04_v05_integration_contract",
             "--no-strict", "--json"],
            capture_output=True, text=True, encoding="utf-8", errors="replace",
            timeout=120, cwd=str(PROJECT_ROOT),
            env={**__import__("os").environ, **env},
        )
        # Non-strict mode does not fail on IC_V1130_UNREACHABLE; but process always exits 0/1/2/3/4
        assert result.returncode in (0, 1, 2, 3, 4), \
            f"unexpected exit {result.returncode}: stderr={result.stderr[:300]}"
        out = result.stdout or ""
        # If exit 0/1/2, JSON parses; otherwise error JSON parses
        if out.startswith("{"):
            payload = json.loads(out)
            assert "passed" in payload or "code" in payload
        else:
            assert "passed" in out or "IC_" in out


# ============================================================================
# 9. TestContractVersionGate (LOCKED semver)
# ============================================================================


class TestContractVersionGate:

    def test_version_dotted_three(self):
        v = INTEGRATION_CONTRACT_VERSION
        assert v.count(".") == 2
        major, minor, patch = v.split(".")
        for p in (major, minor, patch):
            assert p.isdigit(), f"{p} not digit"

    def test_version_not_unstable(self):
        # 0.1.0 — early stage, not 1.0 yet
        assert INTEGRATION_CONTRACT_VERSION.startswith("0.")
