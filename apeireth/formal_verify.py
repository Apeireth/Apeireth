"""Apeireth formal verification contract shell — R6-PHL-03.

Architecture choice (contract only, no theorem prover is invoked):
- Primary: TLA+ models the Harness modification path as a temporal state machine;
  it fits safety/liveness, rollback, and four-gate ordering better than code proofs.
- Second: Lean 4 proves pure IR/functions after CompilerIR exists in R7+.
- Alternative: Dafny for executable imperative contracts. Rocq/Coq and Isabelle/HOL
  remain high-assurance options when proof staffing and extraction justify the cost.

Boundary: formal_verify proves that a *modification path* satisfies a spec. It is
not self_reproduction (same-form rebirth), self_mod_safety (runtime boundary), a
real prover integration, or evidence that Apeireth/ASI/Phenomenal is achieved.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Dict, List, Protocol, runtime_checkable

from .philosophy import check_philosophy

MODULE_NAME = "formal_verify"
PROTOCOL_VERSION = "0.1.0-contract"
CONTRACT_ONLY = True

# 主 17:58: formal vocabulary must not inflate evidence into truth.
PHILOSOPHY_NOTES: Dict[str, str] = {
    "spec_is_not_proof": "A written specification is a claim, not a proof.",
    "counterexample_is_not_bug": (
        "A counterexample refutes one claim; it does not classify every defect."
    ),
    "prover_is_not_truth": (
        "Every prover depends on logic, axioms, encoding, and soundness boundaries."
    ),
}


@dataclass
class VerificationSpec:
    """Named invariants and permitted future theorem-prover backends."""

    name: str
    invariants: List[str]
    theorem_provers: List[str]


@dataclass
class VerificationResult:
    """Proof attempt metadata; success alone never upgrades a claim to truth."""

    success: bool
    proof_id: str
    counterexample: Dict[str, Any] = field(default_factory=dict)


@runtime_checkable
class FormalVerifyProtocol(Protocol):
    """Contract for specifications, proof attempts, verification, and refutation."""

    def spec(self) -> str:
        """Return the specification description, not proof evidence."""
        ...

    def prove(self, claim: str) -> bool:
        """Attempt to prove one claim under an explicit future backend."""
        ...

    def verify(self, code: bytes) -> bool:
        """Check code against the declared specification."""
        ...

    def counterexample(self) -> Dict[str, Any]:
        """Return the latest counterexample for the attempted claim."""
        ...

    def invariants(self) -> List[str]:
        """Return modification-path invariants."""
        ...


def guard_formal_verify() -> Dict[str, Any]:
    """Reference V3 philosophy guard without claiming a real proof."""
    check = check_philosophy(
        module_name=MODULE_NAME,
        implementation_summary=(
            "Contract-only Protocol and dataclasses for future modification-path "
            "verification; no prover, proof, or ASI claim."
        ),
        claimed_pass=None,
        evidence=PHILOSOPHY_NOTES,
        categories=["contract_shell", "no_real_impl", "philosophy_referenced"],
        required_categories=["contract_shell", "no_real_impl", "philosophy_referenced"],
    )
    return {
        "module": MODULE_NAME,
        "protocol_version": PROTOCOL_VERSION,
        "guard_passed": check.passed,
        "guard_status": check.status,
        "guard_notes": dict(PHILOSOPHY_NOTES),
        "deviation_count": len(check.deviations),
    }


__all__ = [
    "CONTRACT_ONLY",
    "FormalVerifyProtocol",
    "MODULE_NAME",
    "PHILOSOPHY_NOTES",
    "PROTOCOL_VERSION",
    "VerificationResult",
    "VerificationSpec",
    "guard_formal_verify",
]
