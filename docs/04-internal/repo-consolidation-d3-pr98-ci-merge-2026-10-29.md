# D3a — PR #98 CI repair and protected merge evidence

[Document-Meta]
Document: docs/04-internal/repo-consolidation-d3-pr98-ci-merge-2026-10-29.md
Version: 1.0
Last-Modified: 2026-10-29
Status: Kernel Convergence Complete (under approved startup definition)
Author: Codex

## Scope and acceptance

D3 starts from clean KC1 `9e485fe274a4207ce7c6a4f511a4b074901edb28`.
Main initially `ac73436c65bdbb34369421c2e8d896bf538e591d`, already containing protected merges #64/#70; neither was merged again.
An ordinary two-parent merge brought main into KC1, preserving published history and all KC1 acceptance work.
No settings/admin mutation, force-push, direct main push, test deletion/ignore or assertion weakening occurred.

**Kernel Convergence Complete (under approved startup definition)** remains the acceptance judgment.
Owner approval and retained Dev normal_start P50 **7.5642 ms** are in [Final Acceptance](kernel-convergence-final-acceptance-2026-10-28.md).
Legacy Dev cold-start **11.4143 ms** remains **unmet historical**, and true cold_start remains **UNAVAILABLE** (drop_caches denied; no fabricated samples).
No performance, live external-provider or hardware samples were added here.

## CI causes and bounded fixes

| failure / evidence | repair | preserved boundary |
| --- | --- | --- |
| Duplicate audit YAML `if` | ordinary main→KC1 merge carries #64 workflow and its two lockfile fixes | original audit severity; no advisory suppression added |
| ETXTBSY fixture | same merge carries already-protected #70 fixture retry | only the existing executable-write race; original error assertions retained |
| Deny license rejection of locked borrow-or-share 0.2.4 | exact crate/version MIT-0 allowance after reading the registry license and metadata | no global license allowance, dependency change or vulnerability ignore |
| Six new orphan symbols | retain five public Host compatibility surfaces with behavior tests; remove only unused internal providers_arc | no ratchet growth, no artificial production call, no public compatibility deletion |
| Secret scan bootstrap | detect existing pwsh before package installation; keep both scans mandatory | fresh runner log had apt `Unable to locate package powershell` even though the shell was already installed |
| Historical scan loses lines and prints credentials | preserve native output line boundaries; report location/rule with redacted content; isolated-repository regression tests | same detection patterns, all refs and original path/regex allowlists |
| Three historical synthetic fixtures | content-addressed exception ledger: blob + path + line + match SHA256 | current files and every modified identity remain detected; count reported explicitly |
| macOS context_overflow fixed-window failure | reproduce on Linux with long TMPDIR, use relative randomized spill fixture | original 2400 window, overlay, budgets, call count and all assertions unchanged; no product logic change |
| Wiring/claim drift | review five affected claims, preserve levels/defaults, regenerate STATUS and wiring with their scripts | includes A1 fresh schema/test evidence; no hand-edited generated output |

The dependency license is the permissive no-attribution MIT variant; its standardized identity is documented by [SPDX MIT-0](https://spdx.org/licenses/MIT-0.html).
The exception applies to `borrow-or-share =0.2.4` only. The registry license retains its original copyright text.

Host compatibility tests cover candidate-specific tool projection across transient fallback, streaming fallback/full response,
session request identity/accounting, egress denial without a provider call, and schema/session-scope/shutdown rejection.
The memory Host budget test uses a fitting Unicode item plus an oversized item, verifies inclusion/exclusion under two budgets,
and verifies unchanged persisted content. Its initial diagnostic incorrectly expected truncation; the existing retrieval contract excludes oversized candidates.
The final test asserts that actual contract without changing product logic or weakening existing assertions.
These are tested compatibility APIs, not newly claimed production/default wiring.

Historical findings were individually inspected: image metadata is a synthetic stripping marker; the PEM source is a string template with literal interpolation;
the scanner comment explicitly labels its old value as a fake redaction fixture. Exact source identities and rationale are in
`scripts/secret-scan-history-fixtures.json`; no credential values are copied to this report or evidence logs.
Adversarial tests verify each mismatched identity field still fails, the same token in a changed blob/current file still fails,
and deleted credentials are still found. No broad fuzz/scanner path exception was introduced.

## Local verification

Commands ran on 2026-10-29, Rust/Cargo 1.97.1, PowerShell 7.5.4, Python 3.13.5, Linux x86_64.
Cargo used `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0` and `-j 2` / `CARGO_BUILD_JOBS=2`.
No resource failure or cleanup was needed. A second full run follows the final fixture-only CI correction; it is not a resource-failure retry.

| command | result |
| --- | --- |
| cargo test -p apeireth-tools-canonical --lib external::tests --locked -j 2 | PASS |
| cargo test -p apeireth-runtime --lib canonical::provider:: --locked -j 2 | PASS |
| cargo test -p apeireth-runtime --test legacy_tool_adapter_contract --locked -j 2 | PASS |
| cargo test -p apeireth-runtime-assembly --test convergence_memory --locked -j 2 | PASS |
| cargo clippy -p apeireth-runtime -p apeireth-runtime-assembly -p apeireth-tools-canonical --all-targets --locked -j 2 -- -D warnings | PASS |
| cargo fmt --check | PASS |
| cargo deny --all-features check (cargo-deny 0.20.2) | four classes PASS; existing nonfatal stale-policy warnings remain in log |
| python scripts/audit_unwired.py --check; python scripts/group_unwired_baseline.py --check | PASS; no baseline growth |
| python -m unittest discover -s scripts -p test_secret_scan.py | PASS |
| pwsh -NoProfile scripts/secret-scan.ps1 -Mode scan-all / scan-history | PASS; history explicitly reports three reviewed immutable fixtures |
| pwsh scripts/gate_full.ps1 -Log /tmp/apeireth-d3a/gate-full-final-inner.log | GATE_FULL=0, all five segments PASS |
| docs truth strict/generated, wiring verify/env registry, verification ledger | PASS |

The first full cargo gate passed with stable cargo inputs while later script/docs commits landed; its input identity is in `local-verification-summary.json`.
The final fixture-only correction is separately recorded with deterministic failing/passing long-TMPDIR evidence and an unchanged-assertion suffix hash.
The final five-segment run passed on frozen head `6bbcba6999bc79cccd38db4f40651d4ad5c8bdc5` with tree `7bdb74773a7f795c39e8bc19d8ff1d6fc71a2cad` and a clean worktree. `final-verification-summary.json` records GATE_FULL=0; subsequent main verification can compare this exact tree.
Scanner contracts/history were verified separately on their fixed inputs.
Test-number truth remains [test-baseline](../03-reference/test-baseline.md); this slice does not replace its aggregate counts.
The first added test compile lacked a ModelFeature import; that diagnostic and the memory-contract diagnostic are retained alongside final passing runs.

## Remote checks and protected action

At the final protected action the PR was **MERGEABLE / UNSTABLE**, with all six required contexts SUCCESS and no failed checks. Windows nextest was still running as a nonrequired check; it is not reported as PASS here. The normal protected merge succeeded without admin or a review bypass.

| required context | fresh result |
| --- | --- |
| cargo kani (approval SM + property families, mirror crate) | SUCCESS |
| cargo fmt --check | SUCCESS |
| cargo nextest (ubuntu-latest) | SUCCESS |
| docs truth gate (P1c blocking) | SUCCESS |
| 8 硬墙守门 (LOCKED / version / baseline / 14 键 / V1136) | SUCCESS |
| cargo deny check (bans + licenses + sources + advisories) | SUCCESS |

Source head: `6bbcba6999bc79cccd38db4f40651d4ad5c8bdc5`. Recovery tag: `recovery/pre-merge-pr98-6bbcba69` at that exact head.
Local rev-parse, remote ls-remote and commit object checks matched; rev-list found zero missing reachable objects and connectivity fsck returned 0
(43360 reachable objects recorded with a listing hash). Workflow/script searches found no literal dependency on KC1 as the only recovery reference.

`gh pr merge 98 --merge --match-head-commit 6bbcba6999bc79cccd38db4f40651d4ad5c8bdc5` returned 0.
Main advanced from `ac73436c65bdbb34369421c2e8d896bf538e591d` to ordinary merge `a8c132fb129d2dfa658a73be894b33bb2e00fe84`; its first parent is the old main and second parent the verified source head.
Its tree equals the frozen source tree exactly. The head was automatically deleted, observed and immediately re-created by a normal tag-to-head push before fetch/prune;
remote KC1 then matched `6bbcba6999bc79cccd38db4f40651d4ad5c8bdc5`. No repository setting changed.

Recovery commands, for a missing active head only (never force over an advanced branch):

```bash
git fetch origin refs/tags/recovery/pre-merge-pr98-6bbcba69:refs/tags/recovery/pre-merge-pr98-6bbcba69
git rev-parse refs/tags/recovery/pre-merge-pr98-6bbcba69
git push origin refs/tags/recovery/pre-merge-pr98-6bbcba69:refs/heads/refactor/kc1-kernel-convergence
```

Protection was reported enabled with the six contexts enforced for everyone. The detailed protection endpoint returned HTTP 404 under this token;
that visibility limitation was retained rather than interpreted as absent protection. The repository reports push access and no admin access.


## Commits, evidence and next

The source merge and three repair/generated commits are listed by exact SHA in `source-commit.json`, `scanner-commit.json`, `generated-commit.json` and the Git log; the final fixture correction is in `overflow-commit.json`.
The documentation outcome commit is separate. Every commit used `pwsh scripts/commit.ps1` with explicit Codex author and `-ArchStrict`.
The Git hook files lack executable bits in this checkout; their full Bash bodies were also executed explicitly with the author environment and passed.
No global hooksPath, file mode, repository setting or protection change was used.

Logs/exit files, sanitized failing job logs, immutable-fixture review, initial/current required-check snapshots,
final verification tree, anchor reachability and protected-action/recreation receipts are archived under `/workspace/apeireth-codex/d3a-evidence/`.
The report retains initial missing-identity merge diagnostics, CLI projection limitations and the first failing test diagnostics;
only final passing commands are labeled PASS. Superseded PR runs were cancelled to release capacity; the exact selected old heads and cancellation responses are retained.
Next is the D3b bounded-candidate review, then F2 exact-main verification and G2 addendum; no unique/fork/archive cleanup is authorized by this note.
