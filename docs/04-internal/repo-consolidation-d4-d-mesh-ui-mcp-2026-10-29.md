# D4d: Execute Mesh UI and MCP follow-ups

2026-10-29. The owner's D4 mandate was executed against current protected main; the earlier D3b plans-only disposition is historical.

| Original scope | Actual execution | Disposition |
|---|---|---|
| #31 Mesh RC2 | Source-head protocol/mesh/QUIC/rendezvous all-target tests pass; actual main reconciliation attempted and aborted with six conflicts; source-head clippy fails on needless_continue at endpoint.rs:137 | Keep WIP; no real NAT/WAN/hardware claim |
| #78 UI | Actual independent source workspace tests/clippy/correct --all fmt pass; only dependency-free reactive core extracted into #105 with new-head independent CI and root six green | Core merged; broad DOM/GPU/demo remains WIP |
| #19 MCP guide | Current-main docs-only #106 reconciles client/transport explanation with current Host ownership/governance; client/bridge/assembly fixtures and docs checks pass | Guide merged; original closed only after separate exact-head recovery verification and evidence comment |

## Landed extracts

| PR | Exact head | Ordinary PR merge | Recovery tag |
|---|---|---|---|
| #105 | `2071a1ebbc4d036c3b367d4b00428ee1e139f58e` | `5d9ea8b7b094c85e9e9fbee0ad9e14d50950b9cd` | `recovery/pre-merge-pr105-2071a1eb` |
| #106 | `6170877573981cf8c64687992a8037a5feca2304` | `022f8c9e0af535a9a8a7296e04425fd23056af66` | `recovery/pre-merge-pr106-61708775` |

#105 was merged by the recorded normal `gh pr merge --merge --match-head-commit` command after fresh six required checks, current-head independent UI workflow success, exact local/remote tag SHA and object reachability, and workflow/script head-name dependency checks. The branch may auto-delete only after that recovery verification. No setting, admin, force or direct-main operation was used.

#106 has a separate evidence class: the merge command found it already MERGED. GitHub reports an ordinary two-parent PR merge by `Jimmyxiao2009` at `2026-10-29T07:28:39Z`; all six required check completions predate that timestamp, and the exact source is now in main. The matching recovery tag was already present when observed and has been independently reverified with complete object reachability. The remote lightweight tag's creation time before the merge is **NOT_INDEPENDENTLY_ESTABLISHED** by retained evidence. This audit does not retroactively claim a pre-merge anchor receipt or a command it did not execute. The original #19 closure used its own verified tag before closure and links the actual replacement merge; its historical source is preserved rather than described as integrated wholesale.

## Mesh readiness boundary

The source-head package test command is `cargo test -p apeireth-mesh-protocol -p apeireth-mesh -p apeireth-mesh-quic -p apeireth-mesh-rendezvous --all-targets --locked -j 2`. Local relay, endpoint-observation and provider full-turn fixtures execute, but their names do not prove real NAT/WAN traversal. Matching all-target clippy with `-D warnings` exits 101 on a redundant continue; no lint allow or assertion relaxation was applied.

The historical source lacks `scripts/arch_guard.py`; attempting that exact path exits 2, not an architecture PASS. The fresh-main trial conflicts in lockfile, canonical Runtime execute/mod/runtime, Kernel identities and documentation. The source predates current AgentId/ProfileId/namespace surfaces and 14-key assertions; those are not rolled back to make the old topic fit. The trial merge was aborted normally. A later bounded transport/identity slice must adapt to canonical AgentTask/dispatcher authority and the current architecture contract before fresh CI; real NAT/WAN readiness needs separate evidence. #31 stays open with actual failure/conflict comments and its unique work preserved.

## Independent reactive core boundary

The additive extract imports only `apeireth-ui-core` and its original code/assertions, correcting comments that referenced an omitted demo ownership scanner and old design document. Runtime::new remains public; ReactiveScope limits methods but does not prove all Host wiring. No scanner, browser, DOM, GPU or production frontend consumer is claimed. The UI Runtime schedules reactive effects and does not execute AgentTask or create another Kernel backend.

ADR-0021 and contract register the independent one-crate workspace and component boundary. The package version copies the existing root value; root workspace members, dependencies, version, size budgets and ratchets remain intact. Independent library size is outside the root scanner as explicitly documented. Dedicated CI runs check/clippy/fmt/native tests and compile-fail doctests; root Kernel checks cannot substitute for them. The current-head independent workflow passed before #105 merged. The broader #78 remains open WIP with merge evidence and readiness comments.

## MCP guide boundary

The guide preserves valid initialize/list/call and transport/result/error intent while correcting current ownership: the client is IMPLEMENTED, the opt-in Host bridge is PRODUCTION WIRED / DEFAULT OFF, and protocol primitives alone do not register or authorize tools. The simple client's handshake does not inherit all Host protocol/session/reconnect guarantees. Examples propagate errors and do not pretend to supply a working transport. No source, dependency, default or version changes are hidden in this docs extract, and no live external server/hardware validation is claimed.

The original #19 tip is preserved by `recovery/superseded-pr19-3aea2ccc`; comment and CLOSED-state receipts link #106's real merge. Broad historical source in the old branch is not described as having landed merely because its guide intent is superseded.

## Verification and evidence

Commands, versions, exact exits, source hashes, proof/merge/recovery receipts and comments are under `/tmp/apeireth-d4d/`, mirrored logs under `/workspace/apeireth-codex/`, and the milestone archive. The first UI fmt invocation without --all failed to find targets; the correct independent-workspace --all command passes and both attempts are retained. No test deletion, new ignore, weakened assertion, security exception, root baseline expansion or manual source-branch deletion occurred. Repository-wide test totals remain in [test-baseline](../03-reference/test-baseline.md).

Kernel status remains **Kernel Convergence Complete (under approved startup definition)**: retained approved normal_start P50=7.5642 ms, legacy A4 11.4143 ms unmet historical, true cold_start UNAVAILABLE. First-run/regression and other unapproved targets remain advisory. Final main verification is recorded separately in F3 after the remaining source integrations settle.
