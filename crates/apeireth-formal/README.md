# apeireth-formal

A small, deterministic proof-engine facade for Apeireth invariants.

- `invariants/` contains Kani-friendly POD harnesses.
- `FormalEngine` dispatches the five built-in invariant descriptions.
- `TlaSpec` renders a minimal auditable TLA+ module.

The runtime tests are smoke tests; Kani remains the authoritative symbolic verifier.
