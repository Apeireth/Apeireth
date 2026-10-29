# D4c: Land bounded environment, provider and cache extracts

2026-10-29. The D3b plans were executed as focused additive PRs from protected main. The broad original #95 and #94 histories remain open WIP; extracting these scopes does not discard their remaining commits.

| PR | Exact merged head | Verified recovery tag | Protected merge |
|---|---|---|---|
| #99 | `1210e8e475de1c264a596700b36e55987eddfa10` | `recovery/pre-merge-pr99-1210e8e4` | `62015d3207ed351367309dbfca7c573af0a78104` |
| #103 | `d23eecc0de103f3731fe13dac7cd419672a454c7` | `recovery/pre-merge-pr103-d23eecc0` | `9ff493eef19cb437bf6c34fc66ef37824f2bd5dc` |
| #104 | `e50985668332bc67d823f0bb19e86d4bc87345e9` | `recovery/pre-merge-pr104-e5098566` | `04dacbd51079791eae534a4567d6ea0ce377e3cc` |

## #95 environment-lock contract → #99

Only seed 4d5b4c66's twelve test-support/test paths were extracted. Existing test functions and fail-closed input cases remain; shared helpers execute the same assertions. Participating unit modules share one cfg(test) mutex, with one lock per integration-test process. This does not synchronize arbitrary external environment reads.

Local tools all-target tests and clippy pass. Initial full-workspace O-6 CI caught a race in the imported exclusion regression itself: the observer could acquire before the holder, and the sleep established no order. A normal FF follow-up acquires the holder first, verifies WouldBlock through a channel, then retains both exclusion and post-release assertions. There is no ignore, assertion deletion, increased sleep or single-thread masking. Final-head CI passed before merge. #95 remains WIP, with the env-lock scope marked superseded and linked to the actual merge.

## #94 response contract → #103

Seeds d7d25743/ce6de635 now use the existing reasoning adapter before empty-response checks in canonical non-streaming complete. Default is OFF; model filters and finish reasons remain explicit. Existing assertions are retained; local HTTP mock complete-path tests explicitly pin OFF and ON. The parser returns a normalized response rather than rewriting a full forwarding JSON object. Streaming adaptation/live-provider validation remain outside this extract. Three existing configuration registry entries were classified from UNCLASSIFIED to PRODUCTION WIRED with original defaults/direct-reader evidence retained; generated wiring verifies without manual editing.

Provider all-target tests/clippy, targeted fixture checks, strict truth/arch, formatting and orphan ratchet pass locally. The HTTP fixture resides in a separate test module to respect the existing source-file size budget. No second Provider/dispatcher backend, external dependency or root version change was introduced.

## #94 persistent file cache → #104

Only f83cd595's file-fetch cache intent was extracted. This is an **IMPLEMENTED library API**, with no production consumer newly wired. URL metadata binds exact URL to content hash; every disk hit rehashes bytes. Corrupt blobs are repaired; missing/corrupt entries fall back to real local/remote acquisition. Optional cache-write failures preserve successful fetches and increment an observable counter.

Review corrected seed defects: hash syntax/URL identity validation, rejection of symlink cache records, actual allowed-root resolution before both memory/disk hits, and reuse of the existing exclusive-temp atomic-write primitive with Unix 0600. Cache root ownership remains a Host trust boundary; hash integrity is not independent content authenticity. No claim is made to eliminate filesystem TOCTOU or add power-loss durability, quotas or TTL.

Gateway all-target tests/clippy, root format, strict truth/arch and orphan/group ratchets pass. New regressions cover corrupted repair, cross-URL metadata, optional write failure, symlink records and changed root-link targets. The Kani mirror includes exact core atomic-write source, and its workflow trigger expands to cover that file. Ordinary mirror tests pass. Local cfg(kani) harness typecheck/default-input smoke passes; the full stub run retains two existing fold-render string failures caused by its pre-existing fixed cfg(kani) hint. Those assertions/source are unchanged, failure logs retained, and stub smoke is not called symbolic proof. Fresh formal Kani CI passed on the merged head for its existing bounded properties; this is not a formal proof of all filesystem/cache persistence or race behavior. Mirror formatting edits are mechanical.

## Remaining original work

#94 remains WIP: production rate policy requires assembly adaptation; partner/milestone persistence requires storage review; parallel dispatch requires an owner decision against the existing canonical dispatcher. #95's broad unique work remains WIP. Evidence comments link each actual extraction and merge; no wholesale conflicting mega-PR merge occurred.

Reproduce package verification with `cargo test -p apeireth-tools-canonical --all-targets --locked -j 2`, `cargo test -p apeireth-provider --all-targets --locked -j 2`, `cargo test -p apeireth-gateway --all-targets --locked -j 2` and matching package clippy `--all-targets --locked -j 2 -- -D warnings`; docs/arch/orphan and mirror commands with exact exits are in `/tmp/apeireth-d4c/` and the mirrored/archive evidence. Repository-wide totals remain in [test-baseline](../03-reference/test-baseline.md).

Kernel acceptance remains **Kernel Convergence Complete (under approved startup definition)**. Approved retained normal_start P50=7.5642 ms; legacy A4 OLD_COLD_P50=11.4143 ms remains unmet historical, and true cold_start is UNAVAILABLE because real cache dropping was denied. First-run/regression targets remain advisory. This milestone does not invent new performance samples or hardware validation.

All actual merges use fresh six required contexts plus no known failing software/security checks, exact-head local/remote/reachability-verified recovery tags, and `gh pr merge --merge --match-head-commit`. There is no force push, direct main push, admin bypass, repository-setting mutation or manual source-branch deletion. Restore a preserved head with `git fetch origin tag <tag>` then `git push origin <tag>:refs/heads/<new-recovery-branch>`; do not overwrite an existing divergent published ref. Individual receipts include the exact command, SHA and object-reachability proof.
