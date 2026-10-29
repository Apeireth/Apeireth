# D4b: Dependabot execution checkpoint

2026-10-29. This is an in-progress checkpoint, not the final D4b result. The owner's D4 mandate replaces the earlier plans-only D3b disposition. Main changes below occurred through exact-head recovery tags and ordinary protected GitHub PR merges; there were no settings, admin, force-push or direct-main operations.

| Original | Executed action | Result at checkpoint |
|---|---|---|
| #97 Tokio | Fresh required checks + verified tag `recovery/pre-merge-pr97-bf9e80c3` + protected ordinary merge | Merged; main `396fea41fa09d5ddd694501d831dd5e2b8d636e2` |
| #33 serde_json | Same path at `c2592f4f`; required six pass | Merged; main `a9026b9df2f3e2f5f0d920fe42d24c7e9933f409` |
| #96 setup-python | Ordinary main merge appended to published bot head, FF-pushed; fresh CI + anchor | Merged; main `dd243e6bcd8ded989c4ca32bde3313de3badde59` |
| #29 dependency group | Actual fresh-main lockfile patch, deny and core tests; original broad-merge attempt aborted after neutral gate rejected imported historical text | Replacement #102 is in CI; original kept until replacement lands |
| #20 upload-artifact | Fresh additive head with only the original guard-workflow action update | Replacement #100 is in CI; original kept until replacement lands |
| #17 cache | Fresh additive head with only the original two workflow action updates | Replacement #101 is in CI; original kept until replacement lands |
| #22 hyper-util | Actual fresh-main precise bump succeeds; current cargo-deny exits 2 on base64 0.22.1 + 0.23.1 | Real WIP; no policy exception, merge or silent discard |

#33's non-required automation job failed only because automatic merging is disabled (`enablePullRequestAutoMerge`). Its required checks were green; the normal protected manual merge succeeded without changing that setting. No failed security or software check was waived.

The original observed bot heads were anchored before this serial work. Two older B-inventory tips had already been autonomously rewritten by the bot; D4a records the retrospective recovery anchors honestly. An agent did not force-push them. Every actual merge has separate exact-head local/remote/reachability receipts and restore commands.

The hyper-util blocker is reproducible with `cargo update -p hyper-util --precise 0.1.21` followed by `cargo deny --all-features --locked check`: existing jsonschema/reqwest retain base64 0.22.1 while the bump adds 0.23.1. The candidate patch and failure are archived; an evidence comment leaves #22 open with an upstream-compatible resolution as its exit condition. No broad schema/client migration is smuggled into the dependency bump.

Actual follow-on work is also in CI: #99 test-only environment lock, #103 provider response contract, #104 verified file cache, #105 independent reactive core, and the extracted MCP guide. Mesh package tests pass locally, but canonical Runtime reconciliation conflicts and unvalidated real NAT/WAN remain explicit WIP boundaries. Root Kernel checks do not stand in for UI or hardware checks.

Kernel status remains **Kernel Convergence Complete (under approved startup definition)**. The approved retained normal_start P50 is 7.5642 ms; legacy 11.4143 ms remains unmet historical, and true cold_start remains UNAVAILABLE. This checkpoint does not assert new performance samples or final-main verification.

Evidence: `/tmp/apeireth-d4b/`, mirrored `d4b-*.log` and `.exit` under `/workspace/apeireth-codex/`; PR creation, source hashes, recovery receipts and actual merge records are machine-readable JSON. The final D4b addendum will record replacement merges/closures or exact blockers after CI settles.

## Final D4b result

The checkpoint above is historical. All actionable serial updates below have now landed; #22 remains a measured policy blocker.

| PR | Exact merged head | Verified recovery tag | Protected merge |
|---|---|---|---|
| #97 | `bf9e80c3881e2dadbc89c3948c7b1f956e59bbc5` | `recovery/pre-merge-pr97-bf9e80c3` | `396fea41fa09d5ddd694501d831dd5e2b8d636e2` |
| #33 | `c2592f4fe8b02d6b9783020ff91a3df05d64bd1d` | `recovery/pre-merge-pr33-c2592f4f` | `a9026b9df2f3e2f5f0d920fe42d24c7e9933f409` |
| #96 | `777614510808e603e47b7f4aa8f7fa33e7f8fc5d` | `recovery/pre-merge-pr96-77761451` | `dd243e6bcd8ded989c4ca32bde3313de3badde59` |
| #101 | `44cdffa79c46690e6a0b563a62a2230f093d4b0e` | `recovery/pre-merge-pr101-44cdffa7` | `00f923c502f119d55b14cb70c69394c544a3f8d3` |
| #100 | `47b13fbd602d45f57dbe7f1a9191a11b7af21f6b` | `recovery/pre-merge-pr100-47b13fbd` | `3b18a6e2bbaeed953eda74c0df62e9ad9c001f94` |
| #102 | `e7e8454baf3b66bcd1a6132e92687cebc81fe27c` | `recovery/pre-merge-pr102-e7e8454b` | `074ec29c0ac6af028f5d66a9f5f349c69e433f4c` |

Original #17 → #101, #20 → #100 and #29 → #102 were closed with evidence comments only after replacement merges and separate exact-original-head recovery verification. Their historical commits remain recoverable; patch-intent supersession is not represented as full ancestry integration. #96 was refreshed by an ordinary main merge and FF push, never by force.

#22 remains open WIP: precise hyper-util 0.1.21 resolution succeeds, but cargo-deny rejects duplicate base64 0.22.1/0.23.1. No exception was added. The candidate patch, full failure and explicit exit condition remain preserved. The failed historical-neutral broad-merge attempt for #29 was aborted; its narrow additive replacement kept current Tokio/serde resolutions.

Kernel acceptance remains **Kernel Convergence Complete (under approved startup definition)**. Approved retained normal_start P50=7.5642 ms; legacy A4 OLD_COLD_P50=11.4143 ms remains unmet historical, and true cold_start is UNAVAILABLE because real cache dropping was denied. First-run/regression targets remain advisory. This milestone does not invent new performance samples or hardware validation.

All actual merges use fresh six required contexts plus no known failing software/security checks, exact-head local/remote/reachability-verified recovery tags, and `gh pr merge --merge --match-head-commit`. There is no force push, direct main push, admin bypass, repository-setting mutation or manual source-branch deletion. Restore a preserved head with `git fetch origin tag <tag>` then `git push origin <tag>:refs/heads/<new-recovery-branch>`; do not overwrite an existing divergent published ref. Individual receipts include the exact command, SHA and object-reachability proof.
