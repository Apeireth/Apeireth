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
