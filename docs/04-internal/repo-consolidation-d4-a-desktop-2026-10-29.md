# D4a — Desktop verification and protected merge

[Document-Meta]
Document: docs/04-internal/repo-consolidation-d4-a-desktop-2026-10-29.md
Version: 1.0
Last-Modified: 2026-10-29
Status: merged; native environment limitation retained
Author: Codex

D4 explicitly supersedes the earlier plans-only disposition. PR #34 at `50d9684d04e9b6e0cca97520414d76a9e2b97258` was freshly MERGEABLE/CLEAN with all six required contexts SUCCESS.
The exact head was tagged `recovery/pre-merge-pr34-50d9684d`, pushed and verified by local/remote SHA, commit-object checks, no missing reachable objects and connectivity fsck.
No workflow/script relies solely on its feature branch name for recovery. Normal `gh pr merge 34 --merge --match-head-commit 50d9684d04e9b6e0cca97520414d76a9e2b97258` returned 0.
No admin flag, settings change, force-push or direct main push occurred; fork ownership did not block this merge.

Main advanced from `a8c132fb129d2dfa658a73be894b33bb2e00fe84` to `32f1d8446b4d34b7682909ac3078743bddc5e53c`.
A detached verification worktree combined the PR head with the then-current main before merging; its tree `ff3c9216f89af1eafaeedf3d78ac6d7f48bf3145` exactly matches the resulting protected merge tree.
This tests the current Kernel/Desktop combination rather than only the old fork base.

| command on combined candidate | result |
| --- | --- |
| pnpm install --frozen-lockfile | PASS; initial missing-package run retained |
| pnpm test | all registered package suites PASS, including pet-default-off/window-lifecycle/scrollbar-theme |
| pnpm check | PASS; no errors or warnings |
| cargo test --manifest-path frontend/companion-desktop/src-tauri/Cargo.toml --lib --locked -j 2 | environment failure: missing glib-2.0/GTK/WebKit development libraries |
| official apt HTTP attempt / signed temporary HTTPS source attempt | HTTP 500 / HTTPS TLS transport failure; the latter returned 0 with failed-download warnings, not a successful dependency installation |
| docs_truth --verify --strict on combined source | PASS, no stale claim |

Node v24.22.1, pnpm 10.34.6, Rust/Cargo 1.97.1; Rust uses debug-info=0, incremental=0 and -j 2.
The first frontend run failed solely because the isolated worktree lacked installed frontend packages; the frozen install and complete successful rerun are recorded.
No assertion, threshold, test selection or default gate was weakened. No actual native GUI, DWM/titlebar, device or hardware validation is claimed.
The native Rust limitation is **BLOCKER_NATIVE_DESKTOP_LIBS**, not a production defect or a fake PASS.

The temporary merge was aborted normally after recording its tree and the detached worktree was removed.
The protected main change was then ordinarily merged back into active KC1 with the required Codex wrapper, preserving KC1 docs history.

The preservation audit also caught autonomous bot updates of B016/B017, whose original tips were not on current refs.
They are now independently recovered at verified tags `recovery/b-inventory-b016-a255c0bf` and `recovery/b-inventory-b017-dbc67125`.
These anchors were created **after observing the external bot rewrites**; they are not falsely described as preceding those external events. The agent did not force-update either bot ref.

Evidence/logs/exit files, exact tree proof, required checks, fork metadata, recovery ledgers and merge receipts are archived under `/workspace/apeireth-codex/d4a-evidence/`.
Kernel acceptance remains **Kernel Convergence Complete (under approved startup definition)**: retained normal_start 7.5642 ms, legacy cold 11.4143 ms unmet historical, true cold UNAVAILABLE.
Next D4b executes serial dependency PRs on the post-#98/#34 main; no unresolved unique or fork work is discarded.
