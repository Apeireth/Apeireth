# apeireth-verify

> v2 port of v1.0-legacy/apeireth-verify (complete API surface preserved).

Cross-crate regression verification:
- VerdictTrace (8-field auditable trace)
- RegressionAssertion (InRange / Monotonic / Idempotent / Equivalent)
- regression_assert! macro
- trace_init! macro
- register_all_in_crate! macro
- verify_all / run_all

Const proofs in const_proofs.rs.
Organ Kani proofs in organ_kani_proofs.rs.
