# F2 — Main verification after the D3 PR decision

[Document-Meta]
Document: docs/04-internal/repo-consolidation-f2-main-verify-2026-10-29.md
Version: 1.0
Last-Modified: 2026-10-29
Status: pass (commands below; exact software tree proven)
Author: Codex

## Exact main and provenance

PR #98 was merged only through the protected ordinary path. Fresh main is `a8c132fb129d2dfa658a73be894b33bb2e00fe84`.
Its tree `7bdb74773a7f795c39e8bc19d8ff1d6fc71a2cad` equals frozen D3a source `6bbcba6999bc79cccd38db4f40651d4ad5c8bdc5` exactly; the latter is an ancestor of main.
No further PR was merged in D3b. KC1 post-merge documentation remains on its active branch and is not misreported as already on main.

Verification uses a detached worktree at the recorded main SHA and exactly that tree's own scripts.
The current KC1 checkout is kept on its authorized branch; there is no local main merge, commit or push.
The worktree is checked clean before/after and normally removed after verification.

## Commands actually run

| check | command | exit |
| --- | --- | --- |
| truth | `python scripts/docs_truth.py --verify --strict` | 0 |
| generated | `python scripts/docs_truth.py --check` | 0 |
| links | `python scripts/docs_links.py --check` | 0 |
| index | `python scripts/docs_index.py --check` | 0 |
| arch | `python scripts/arch_guard.py --strict` | 0 |
| arch-doc | `python scripts/arch_doc.py --check` | 0 |
| wiring | `python scripts/wiring_report.py --verify` | 0 |
| ledger | `python scripts/verify_ledger.py --quiet` | 0 |

These results cover the commands and SHA shown, not external hardware, live WAN/provider behavior or proposed topic extractions.
Logs retain cwd, exact command, timestamp and exit code; the JSON records the verified tree and worktree cleanup.

## Full software evidence and limits

The final D3a `pwsh scripts/gate_full.ps1` run passed all five stages with **GATE_FULL=0** on `6bbcba6999bc79cccd38db4f40651d4ad5c8bdc5`.
Its frozen full tree `7bdb74773a7f795c39e8bc19d8ff1d6fc71a2cad` is identical to this main tree, so the existing software run applies to these exact bytes.
A redundant full rerun in the detached main worktree was deferred for this reason; it was not blocked by ENOSPC/OOM or replaced with older A4 evidence.
F2 itself freshly executes the eight commands above from main. D3a also retains its initial full run and the intervening fixture-failure/fix evidence.
Required PR checks were six SUCCESS at merge; the then-running optional Windows nextest is not turned into a PASS by tree identity.

**Kernel Convergence Complete (under approved startup definition)** remains the qualified acceptance judgment.
Retained normal_start Dev P50 **7.5642 ms** passes the owner-approved daily criterion;
legacy **11.4143 ms** remains **unmet historical**, and true cold_start stays **UNAVAILABLE** (drop_caches denied).
No new startup matrix, cold-cache/hardware/device, live LLM, UI/GPU or Mesh WAN/NAT evidence was measured here.
See [Final Acceptance](kernel-convergence-final-acceptance-2026-10-28.md) and the [D3a report](repo-consolidation-d3-pr98-ci-merge-2026-10-29.md).

## Delivery

Evidence is archived under `/workspace/apeireth-codex/f2-evidence/` with SHA manifests.
The F2 note, historical F cross-note, INDEX and CHANGELOG are committed only on active KC1 using the required Codex wrapper and strict architecture checks,
then fetched/fast-forward-pushed and verified by remote SHA. They do not themselves update main.
G2 records the final integration/anchor/ref-preservation state and remaining focused work.
