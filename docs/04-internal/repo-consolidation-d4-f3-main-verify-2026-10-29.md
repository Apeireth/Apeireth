# F3: Verify the final protected main tree

2026-10-29. Verified main: `9ff493eef19cb437bf6c34fc66ef37824f2bd5dc`. Full Git tree: `089941685918d685e768f8680a9020ed6283b357`. Result: **pass** for the software and documentation checks listed below; this is not a hardware validation claim.

An isolated detached worktree ran the scripts from this exact main SHA. No script or generated output was copied from KC1 into main. The worktree remained clean, and a final fetch confirmed main had not changed during verification.

| Check | Recorded exit |
|---|---:|
| generated | 0 |
| index | 0 |
| truth | 0 |
| arch-doc | 0 |
| links | 0 |
| ledger | 0 |
| arch | 0 |
| wiring | 0 |
| gate_full | 0 |
| normal_start | 0 |
| idle-footprint | 0 |
| ui-tests | 0 |
| ui-doctests | 0 |
| ui-clippy | 0 |
| ui-fmt | 0 |
| perf_receipts | 0 |
| approved_normal_start_criterion | 0 |
| idle_regression_check | 0 |

## Full gate binding

Full five-stage `pwsh scripts/gate_full.ps1` evidence is bound to candidate `a68cf22d5185698797427de08ebbf68731cd3849` and full tree `089941685918d685e768f8680a9020ed6283b357`. Exact-tree reuse: **True**. The candidate is an unpublished ordinary integration validation branch; it is not a local merge or direct push into main. All five stages completed with exit 0 on that frozen tree. Final main's complete Git tree is equal before reuse; no PASS was borrowed from the unequal old F2 tree or from an incomplete stage. If equality had failed, the verifier would have rerun the full gate. The prior candidate and the comment-corrected candidate runs are retained separately, with inner logs, hashes and source-head receipts.

The main test/clippy/fmt and independent image test/clippy stages use the existing single executor. `CARGO_BUILD_JOBS=2`, dev debug=0 and incremental=0 were set; no threshold, workload, assertion, ignore or security rule was relaxed. Required PR CI is separate evidence and remained mandatory. Obsolete-head CI cancellations are explicitly recorded in G3 and do not stand in for current-head checks.

## New main performance samples

Commands: `CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 BENCH_RAW=1 cargo run -p apeireth-bench-harness --locked -j 2 -- normal_start` and the same environment with `idle-footprint`. These are new Dev samples from the verified main worktree, not invented replacements for retained A1b/A4 data.

| Metric | F3 measurement |
|---|---:|
| normal_start P50 ms | 6.825757 |
| normal_start P95 ms | 8.565024 |
| normal_start sample count | 30 |
| idle P50 MiB | 29.1523 |

The daily existing-DB/full-process/normal-cache READY/session-pair criterion is owner-approved; the verifier checks P50 ≤10 ms. Idle is checked against 35 MiB. The retained approved A1b normal_start P50=7.5642 ms remains historical comparison evidence. A4 legacy cold-start P50=11.4143 ms remains **unmet historical** under its old workload. No true-cold samples are invented: real cold_start stays **UNAVAILABLE**, and first-run/regression/P95 ancillary budgets remain proposed/advisory. Historical harness metadata about proposed budgets does not expand or revoke the owner's specifically approved daily criterion.

## Independent and environment boundaries

The root Kernel gate does not validate the independent UI workspace. This main worktree separately runs its native tests, compile-fail doctests, clippy and format. That library remains IMPLEMENTED with no production frontend/DOM/GPU/browser/hardware claim. Desktop package/typecheck evidence is retained from D4a; local native desktop Rust remains limited by unavailable system libraries. Mesh's local fixtures do not prove real NAT/WAN; #31 remains WIP. No Android hardware validation or new true-cold-cache claim is made.

Evidence is `/tmp/apeireth-f3/`, mirrored `f3-*.log/.exit` and the milestone archive, including `main-verify.json`, both candidate records and the complete inner gate log. Repository-wide test totals remain in [test-baseline](../03-reference/test-baseline.md).

Kernel status: **Kernel Convergence Complete (under approved startup definition)**, with the retained legacy and true-cold limitations above.
