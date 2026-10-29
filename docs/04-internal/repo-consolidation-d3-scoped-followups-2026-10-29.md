# D3b — Scoped follow-ups after the PR #98 decision

[Document-Meta]
Document: docs/04-internal/repo-consolidation-d3-scoped-followups-2026-10-29.md
Version: 1.0
Last-Modified: 2026-10-29
Status: scoped plans; unique work preserved
Author: Codex

## Result and scope

PR #98 is MERGED on main `a8c132fb129d2dfa658a73be894b33bb2e00fe84`; the exact head recovery and active KC1 recreation are in [D3a](repo-consolidation-d3-pr98-ci-merge-2026-10-29.md).
The post-merge re-scan found **no allowed independent narrow CLEAN+mergeable PR**. **MERGED=none** for D3b; no additional recovery/deletion action was needed.

This slice does not bulk-merge #94/#95/#31/#19/#78, discard fork work, force-rebase published refs, change settings,
or delete unresolved/unique/archive/active work. Existing recovery requirements continue to apply before any later merge/deletion.
Kernel acceptance remains **Kernel Convergence Complete (under approved startup definition)**;
legacy **11.4143 ms unmet historical** and true cold **UNAVAILABLE** remain visible in [Final Acceptance](kernel-convergence-final-acceptance-2026-10-28.md).
No new startup/hardware/live-provider samples are claimed.

## Fresh PR dispositions

| PR | exact observed tip | mergeability / state | failing checks | disposition |
| --- | --- | --- | --- | --- |
| #97 | `dbc67125` | MERGEABLE / UNSTABLE | 1 | defer serial dependency PR; readiness/check gaps |
| #96 | `926b6a86` | MERGEABLE / UNSTABLE | 1 | defer serial dependency PR; readiness/check gaps |
| #95 | `71631d34` | CONFLICTING / DIRTY | 0 | preserve broad topic; extract env-lock seed only |
| #94 | `caf499b7` | CONFLICTING / DIRTY | 8 | preserve broad topic; separate parser/policy/cache/storage plans |
| #78 | `5cfbccb7` | CONFLICTING / DIRTY | 2 | owner/readiness review; no bulk merge or discard |
| #34 | `50d9684d` | MERGEABLE / CLEAN | 0 | owner/readiness review; no bulk merge or discard |
| #33 | `a255c0bf` | MERGEABLE / UNSTABLE | 2 | defer serial dependency PR; readiness/check gaps |
| #31 | `b43ff3fd` | CONFLICTING / DIRTY | 0 | owner/readiness review; no bulk merge or discard |
| #29 | `37f281ca` | MERGEABLE / BLOCKED | 0 | defer serial dependency PR; readiness/check gaps |
| #22 | `0c65ac7e` | CONFLICTING / DIRTY | 10 | defer serial dependency PR; readiness/check gaps |
| #20 | `d974a109` | MERGEABLE / BLOCKED | 1 | defer serial dependency PR; readiness/check gaps |
| #19 | `3aea2ccc` | CONFLICTING / DIRTY | 0 | owner/readiness review; no bulk merge or discard |
| #17 | `31b8ab1a` | MERGEABLE / BLOCKED | 0 | defer serial dependency PR; readiness/check gaps |

A CLEAN state is necessary evidence for a protected merge, not proof that a broad or owner-sensitive topic is a narrow independent fix.
The unchanged #64/#70 MERGED state is recorded; neither is merged again.
The original B/C unique-work classification and source refs are preserved; a plan is not a deletion disposition.

## Extraction plan for #95

The title describes an environment-lock fix, but the full REST file/commit pagination shows a much broader accumulated topic
(observed before extraction: **62 commits / 114 files**). The selected seed is only
`4d5b4c66f39ddd3c6b600b806890eeb905bcdabd`, whose exact diff affects twelve paths.
Suggested new additive branch: `fix/consolidation-env-lock-contract`, based on **fresh origin/main after #98**.
Do not cherry-pick the entire published PR or overwrite the KC1/A2 restored tests.

Allowed extraction: the existing tools `test_support.rs`, test-only changes in `doc_ops`, `proposal`, `sheet_ops`, `image_ops`,
`device/sense`, `device/device_control`, and integration fixtures `device_sense`, `guard_watch`, `image_ops`, `proposal`, `sheet_ops`.
Reconcile every existing switch/permission assertion before helper consolidation; no deleted assertions or new ignores.
Prove all relevant environment-mutating tests share the same process lock. A writer-only lock does not establish safety against unrelated uncoordinated environment readers.
Keep temporary environment restoration exception-safe. Keep the current A2 fixture/denial coverage.

Planned verification (not executed as an extraction in D3b): tools all-target tests and clippy `--locked -j 2`, fmt and strict arch;
concurrency behavior must be demonstrated before a new focused PR is presented. No fresh branch/PR was created in this slice:
there is no safely reviewed additive result to publish yet, and the broad original remains preserved.

## Extraction plan for #94

Full REST pagination, rather than the first GraphQL page, records **249 commits / 393 files** at the observed source tip.
Each row is a separate proposed additive branch/PR from post-#98 main. Original ledgers, baselines and old CLI/runtime wiring must not be copied wholesale.

| plan | exact source seeds | bounded scope and review | required verification before a focused PR |
| --- | --- | --- | --- |
| Provider response contract | d7d25743 + ce6de635 | only canonical compatible/parser response plus mock-wire proof; inspect current semantics and keep hidden reasoning from silently becoming visible output | provider all-target tests; canonical_multi_provider; routing/budget/egress tests and clippy |
| Production rate policy | d4c265b5 + 23a53dd2 + d3d674df | adapt policy to current unified assembly; old seed touches CLI lib and arch baseline, so blind cherry-pick is inappropriate | production_governance, governance_chain_drill, managed_main; shared budget/cancellation and fail-closed limits |
| Gateway file cache | f83cd595 | file_fetcher implementation plus focused restart/atomicity/cache-key/root/symlink/expiry/quota tests | gateway file_fetcher and canonical_entry_e2e; retained authorization/egress checks |
| Partner then milestones | af412c9c, followed separately by 1fc6beef | storage/lifecycle/namespace retention review first; no copying old default switches, baseline changes or claimed migration status | extracted persistence/milestone tests; profile_memory_authorization; unchanged startup commands and real migration correctness |
| Parallel dispatch comparison | 761e758c | compare with existing canonical dispatcher before proposing code; do not introduce a second execution path | demonstrate an actual missing behavior, then approval/cancellation/accounting equivalence; requires owner/design decision |

`extraction-plans.json` records full SHAs, candidate branch names, exact selected paths and commands.
Commands mentioning tests that exist only on preserved topics are explicitly future commands, not tests claimed to have run on main.
The environment lock, provider parser and cache can be reviewed independently after #98; policy/storage work requires additional semantic decisions.
Parallel-dispatch wholesale transplantation is rejected because the Kernel already owns the execution boundary.

## Mesh, Desktop and UI readiness

- **Mesh #31:** fresh conflict/readiness is recorded in the table. Its retained source includes mesh/mesh-quic/mesh-protocol/rendezvous,
  Runtime routing and WAN/NAT fixtures. Reconcile remote dispatch with the canonical dispatcher and quota/authorization facts;
  prove pinned identity, replay/revocation, bounded cancellation, loopback and negative cases before any focused PR.
  Linux namespace NAT, real NAT/WAN and external endpoint tests remain unmeasured here. Its historical PR body is evidence of the author's claims, not new validation by D3.
- **Desktop #34 (fork):** retain the fork and its owner review. A CLEAN result does not replace review of default-off pet preferences,
  managed desktop entry compatibility and native titlebar/window behavior. Future checks belong to its actual frontend/desktop manifests;
  no GUI/device/Windows hardware validation was performed here. No bulk merge or discard.
- **UI #78:** preserve the separate crates-ui workspace and its evidence. Review standalone core/reactivity, WASM/GPU/font backends,
  pixel/DOM/frame comparison methodology, dependency licenses and intended production adoption separately.
  Root Kernel gates do not validate that independent workspace; historical pixel/GPU claims are not remeasured D3 results.

## Dependency update order

Use fresh post-#98 main for new serial focused dependency PRs. If updating a published bot ref would rewrite it, preserve that ref and prepare a fresh additive head instead. First inspect #97's grouped runtime dependency update against the current lock;
then inspect the setup action update #96; handle older action/toolchain/build bumps one at a time.
Each actual candidate must again be narrow, independent, non-conflicting, required-green and protected-mergeable, with a verified exact-head recovery tag first.
A stale BLOCKED/UNSTABLE/CONFLICTING bot PR is deferred, not silently approved or force-rebased. No bot comment, PR closure or broad ref cleanup was performed.

## Evidence and verification

Fresh open-PR state, complete metadata-only REST pages, exact seed commit stats and bounded extraction plans are archived under
`/workspace/apeireth-codex/d3b-evidence/`. Patch payloads/credential-shaped source literals are excluded from the metadata archive.
Docs links/index, strict truth/generated/wiring, strict arch/arch-doc and commit wrapper checks are recorded separately.
This D3b delivery is documentation/plans only; it does not claim the proposed extraction tests have been executed.
