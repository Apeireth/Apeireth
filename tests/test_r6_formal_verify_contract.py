"""R6 formal_verify contract smoke tests; no real proof is attempted."""
from __future__ import annotations

import inspect
from dataclasses import is_dataclass

from apeireth import formal_verify as fv
from apeireth.formal_verify import (
    CONTRACT_ONLY,
    FormalVerifyProtocol,
    VerificationResult,
    VerificationSpec,
    guard_formal_verify,
)


def test_protocol_exists() -> None:
    assert isinstance(FormalVerifyProtocol, type)
    assert hasattr(fv, "FormalVerifyProtocol")


def test_protocol_methods() -> None:
    expected = {"spec", "prove", "verify", "counterexample", "invariants"}
    assert expected.issubset(dir(FormalVerifyProtocol))
    assert list(inspect.signature(FormalVerifyProtocol.prove).parameters) == ["self", "claim"]
    assert list(inspect.signature(FormalVerifyProtocol.verify).parameters) == ["self", "code"]


def test_runtime_checkable_protocol() -> None:
    assert FormalVerifyProtocol._is_runtime_protocol is True


def test_dataclasses() -> None:
    assert is_dataclass(VerificationSpec) and is_dataclass(VerificationResult)
    spec = VerificationSpec("four_gate_order", ["snapshot_before_apply"], ["TLA+"])
    result = VerificationResult(False, "", {"state": "apply_without_snapshot"})
    assert spec.invariants == ["snapshot_before_apply"]
    assert result.counterexample["state"] == "apply_without_snapshot"


def test_no_real_implementation_yet() -> None:
    assert CONTRACT_ONLY is True
    assert fv.PROTOCOL_VERSION.endswith("contract")
    for name in ("spec", "prove", "verify", "counterexample", "invariants"):
        assert not hasattr(fv, name), f"unexpected module-level implementation: {name}"


def test_philosophy_guard_imports() -> None:
    from apeireth.v1074_asi_production_runner import v1074_philosophy_guard

    assert all(v1074_philosophy_guard().values())
    guard = guard_formal_verify()
    assert guard["guard_status"] in {"PASS", "WARN"}
    assert guard["deviation_count"] == 0
    assert set(guard["guard_notes"]) == {
        "spec_is_not_proof", "counterexample_is_not_bug", "prover_is_not_truth"
    }


def test_module_in_apeireth() -> None:
    import apeireth

    assert apeireth.formal_verify is fv
    assert fv.MODULE_NAME == "formal_verify"


def test_architecture_choice_documented() -> None:
    doc = fv.__doc__ or ""
    assert all(term in doc for term in ("TLA+", "Lean 4", "Dafny", "Rocq", "Isabelle/HOL"))
    assert "self_reproduction" in doc and "self_mod_safety" in doc
