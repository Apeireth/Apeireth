# G3: Executed consolidation result

2026-10-29. Main: `9ff493eef19cb437bf6c34fc66ef37824f2bd5dc`. KC1 before this report: `4ccf8f690485ebbecf2adca6eae5856226688e00`. This report supersedes the integration snapshot in the original [G report](repo-consolidation-g-report-2026-10-29.md); its dated history is retained.

**Kernel Convergence Complete (under approved startup definition)**. The owner, repository user xiao ouyang, approved the daily startup definition on 2026-10-29 Asia/Shanghai with 「当然」:

> With existing-DB **normal_start P50 ≤ 10 ms** as the **daily core startup criterion**
> (existing DB, full process restart, normal OS caches; same READY/session-pair assertions as A1b).

Retained A1b P50=7.5642 ms passes that approved definition. A4 legacy OLD_COLD_P50=11.4143 ms remains **unmet historical**, and true cold_start stays **UNAVAILABLE** because real cache dropping was denied. First-run, regression and other ancillary budgets remain advisory/proposed. New F3 measurements and exact-tree software evidence are separate below; no cold-cache or hardware PASS is invented.

## Actual A–G outcome

A1 improved real Memory initialization without dropping migrations. A2 audited ignored tests and restored safe offline contracts without deleting tests or weakening assertions. A3 audited unified Runtime/AgentTask/managed desktop execution. A1b separated startup modes; owner approval explicitly changed the daily criterion. A4 and D3 supplied full software evidence. B/C froze inventory and classification; D2 integrated #64/#70 with anchors; E performed only the previously documented bounded cleanup. D3 fixed CI and integrated #98 as `a8c132fb129d2dfa658a73be894b33bb2e00fe84`. The original D3b plans-only disposition is superseded by the actual D4 work here.

D4 merged **33,34,96,97,99,100,101,102,103,104,105,106**, closed superseded originals **17,19,20,29**, and leaves open **22,31,78,94,95** at this fresh snapshot. The active KC1 line contains audit milestones and remains preserved; its newest audit docs are not falsely described as already in main. Main advanced through server-reported ordinary PR merges, with no agent direct-main push, force push, settings mutation or admin bypass.

| PR | Exact source head | Actual ordinary merge | Recovery tag | Evidence class |
|---|---|---|---|---|
| #33 | `c2592f4fe8b02d6b9783020ff91a3df05d64bd1d` | `a9026b9df2f3e2f5f0d920fe42d24c7e9933f409` | `recovery/pre-merge-pr33-c2592f4f` | protected ordinary merge |
| #34 | `50d9684d04e9b6e0cca97520414d76a9e2b97258` | `32f1d8446b4d34b7682909ac3078743bddc5e53c` | `recovery/pre-merge-pr34-50d9684d` | protected ordinary merge |
| #96 | `777614510808e603e47b7f4aa8f7fa33e7f8fc5d` | `dd243e6bcd8ded989c4ca32bde3313de3badde59` | `recovery/pre-merge-pr96-77761451` | protected ordinary merge |
| #97 | `bf9e80c3881e2dadbc89c3948c7b1f956e59bbc5` | `396fea41fa09d5ddd694501d831dd5e2b8d636e2` | `recovery/pre-merge-pr97-bf9e80c3` | protected ordinary merge |
| #99 | `1210e8e475de1c264a596700b36e55987eddfa10` | `62015d3207ed351367309dbfca7c573af0a78104` | `recovery/pre-merge-pr99-1210e8e4` | protected ordinary merge |
| #100 | `47b13fbd602d45f57dbe7f1a9191a11b7af21f6b` | `3b18a6e2bbaeed953eda74c0df62e9ad9c001f94` | `recovery/pre-merge-pr100-47b13fbd` | protected ordinary merge |
| #101 | `44cdffa79c46690e6a0b563a62a2230f093d4b0e` | `00f923c502f119d55b14cb70c69394c544a3f8d3` | `recovery/pre-merge-pr101-44cdffa7` | protected ordinary merge |
| #102 | `e7e8454baf3b66bcd1a6132e92687cebc81fe27c` | `074ec29c0ac6af028f5d66a9f5f349c69e433f4c` | `recovery/pre-merge-pr102-e7e8454b` | protected ordinary merge |
| #103 | `d23eecc0de103f3731fe13dac7cd419672a454c7` | `9ff493eef19cb437bf6c34fc66ef37824f2bd5dc` | `recovery/pre-merge-pr103-d23eecc0` | protected ordinary merge |
| #104 | `e50985668332bc67d823f0bb19e86d4bc87345e9` | `04dacbd51079791eae534a4567d6ea0ce377e3cc` | `recovery/pre-merge-pr104-e5098566` | protected ordinary merge |
| #105 | `2071a1ebbc4d036c3b367d4b00428ee1e139f58e` | `5d9ea8b7b094c85e9e9fbee0ad9e14d50950b9cd` | `recovery/pre-merge-pr105-2071a1eb` | protected ordinary merge |
| #106 | `6170877573981cf8c64687992a8037a5feca2304` | `022f8c9e0af535a9a8a7296e04425fd23056af66` | `recovery/pre-merge-pr106-61708775` | server-reported ordinary PR merge observed outside this merge command |

Each merge command executed by this agent used fresh required checks, exact-head recovery verification and `gh pr merge --merge --match-head-commit`. #33 had only the known non-required auto-merge automation failure because auto-merge was disabled; no security/test failure or repository setting was waived. #96 was refreshed with an ordinary main merge and FF push; #17/#20/#29 used additive replacement heads. Superseded originals were closed only after replacement merges and separate original-head recovery verification. No manual D4 branch deletion occurred.

#106 is an explicit observation boundary: it was already merged when this agent's command checked it. GitHub records the ordinary PR merge by Jimmyxiao2009; six required checks completed before merge, and exact-head recovery/object reachability is now independently verified. Remote tag creation **before** that external merge is **NOT_INDEPENDENTLY_ESTABLISHED** by retained evidence. No command or pre-merge receipt is invented for this event. This is a procedural evidence limitation, not a claim that the tag was absent or that software checks failed.

## Remaining WIP and exit conditions

| PR | Concrete work/result | Remaining exit condition |
|---|---|---|
| #22 | Fresh precise hyper-util 0.1.21 bump resolves; current deny rejects base64 0.22.1/0.23.1 duplication | Compatible upstream resolution or separately reviewed bounded migration, then fresh green additive PR; no policy exception |
| #31 | Actual mesh/QUIC/protocol/rendezvous fixtures pass; clippy fails needless_continue; reconciliation conflicts in current Runtime/Kernel; historical arch script absent | Bounded current-authority adaptation plus fresh mesh/root checks and separately evidenced real NAT/WAN |
| #78 | Independent workspace checked; pure reactive core landed as #105 | DOM/GPU/demo/Host ownership and production/browser readiness still need focused PRs and independent evidence |
| #94 | Response contract #103 and file cache #104 landed | Rate policy needs assembly adaptation; partner/milestone persistence needs storage review; parallel dispatch needs owner decision against canonical dispatcher |
| #95 | Env-lock #99 landed and scope labeled superseded-for-env-lock | Broad remaining unique work stays WIP and requires its own review; no wholesale discard |

The native Desktop Rust check remains limited by unavailable local system libraries; actual package tests/typecheck are retained. UI core is IMPLEMENTED with no production frontend consumer. File cache is an IMPLEMENTED library with no new production caller. Bounded Kani properties are not a proof of all filesystem persistence, races or hardware. No live-provider, real NAT/WAN, browser/GPU or Android hardware result is fabricated.

## Final main verification

[F3](repo-consolidation-d4-f3-main-verify-2026-10-29.md) records result **pass** at `9ff493eef19cb437bf6c34fc66ef37824f2bd5dc`. Its own detached-worktree scripts ran strict truth/generated checks, links/index, architecture/arch-doc, wiring and ledger. The five-stage full gate is bound by complete Git-tree equality to the newly gated candidate, not to the unequal old F2 tree. Independent UI native/doctest/clippy/fmt verification remains separate from the root Kernel gate.

New main normal_start P50=6.825757 ms, P95=8.565024 ms, n=30; idle P50=29.1523 MiB. Commands, environment, RAW receipts and exits are retained in F3. Repository-wide test totals remain in [test-baseline](../03-reference/test-baseline.md).

Only obsolete owned extraction heads and obsolete validation push runs for this D4 sequence's older main merge SHAs were cancelled to free runners. Current PR heads/current main and release/deployment workflows were untouched. Receipts preserve exact run IDs, SHAs, reasons and rerun commands. Cancelled old runs are never represented as passing current checks.

## Recovery and inventory preservation

| Metric | Fresh G3 snapshot |
|---|---:|
| branch refs including local validation/extraction refs | 45 |
| refs with commit IDs outside main and KC1 | 29 |
| unique live commit-ID union outside both | 3456 |
| original B unique IDs audited | 3463 |
| original IDs now contained in main/KC1 | 5 |
| remaining original IDs on live branches | 3454 |
| original IDs preserved only by verified remote tags | 4 |
| missing original IDs | 0 |
| new live IDs since B | 2 |
| tags / open PRs | 48 / 5 |

Commit-ID ancestry is not patch-value equivalence. Additive extracts do not make every original historical commit an ancestor of main. External bot rewrites changed B016/B017 tips; D4a records their retrospective original-tip recovery honestly. The corrected collector accounts for original IDs in main/KC1, live refs and verified remote tags, and separately reports new IDs. It does not infer preservation from an unchanged count or force the new set to be a subset of B. Every original B tip remains reachable; old tag objects are unchanged. Local unpublished validation commits are retained as validation history, not asserted to contain unique undelivered product features.

No ARCHIVE_ONLY, UNIQUE_PRESERVE or unresolved owner work was mass-deleted. Earlier E's bounded deletions remain historical; D4 closes only expressly superseded titles with preserved source. Recovery receipts include local/remote exact SHA, missing-object/connectivity checks and restore commands. Restore into a new branch with `git fetch origin tag <tag>` and `git push origin <tag>:refs/heads/<new-recovery-branch>`; never overwrite divergent work.

| B identity | Original ref | Original tip | Current preservation disposition |
|---|---|---|---|
| B001 | `refs/heads/refactor/kc1-kernel-convergence` | `cf59fd4904685530f9a773994c859d36bd2c103b` | main |
| B002 | `refs/remotes/origin/archive/v1.0-master` | `d63a233316567026d14d2f47f5a42cbf0640f047` | preserved live ref/tag |
| B003 | `refs/remotes/origin/backfill-and-ci-fixes-2026-10-24` | `14636741faf1712e05d759d020c64e83bc3e1684` | preserved live ref/tag |
| B004 | `refs/remotes/origin/backup-main-20260901-pre-upstream` | `0acfbb10a976816581524f38c1b1fe07b516c3de` | preserved live ref/tag |
| B005 | `refs/remotes/origin/backup/memory-v2.2-final-freeze-20260914` | `f1e8ee0e0b6b2802332dafd00023a7dd1f2a2fe6` | main |
| B006 | `refs/remotes/origin/backup/pre-2c2-upstream-rewrite` | `60331ded97e1fcf54b258c62b073848b8a004a95` | preserved live ref/tag |
| B007 | `refs/remotes/origin/backup/pre-rc2-replay-994fd67a` | `994fd67a033ee54e768542f12f864cd394921f02` | preserved live ref/tag |
| B008 | `refs/remotes/origin/batch-2026-10-22` | `b613f0ebb9da45b761f7e5d06a383ebf7a50cbce` | main |
| B009 | `refs/remotes/origin/bio-switches-and-w5-2026-10-24` | `7bb074f25e84ceff3f6d3b64727f816c03e04042` | preserved live ref/tag |
| B010 | `refs/remotes/origin/bio-switches-and-w5-v2` | `1190d490a36365cd59c2db49e073e9691e1effd9` | preserved live ref/tag |
| B011 | `refs/remotes/origin/bio-switches-w5-v3` | `200ca38674c333759f1748ff465c8c1eb6b1f1a0` | preserved live ref/tag |
| B012 | `refs/remotes/origin/coverage/baseline-2026-10-24` | `29e9d8645a85e5791d38a6a412acd4874b4c5e61` | preserved live ref/tag |
| B013 | `refs/remotes/origin/debt-round8` | `caf499b769cd1d3aa0a6281c77acc5e1ca5c6ae7` | preserved live ref/tag |
| B014 | `refs/remotes/origin/dependabot/cargo/dependencies-092765a05c` | `37f281caca8c33d0642b8cc963111970816fafa7` | preserved live ref/tag |
| B015 | `refs/remotes/origin/dependabot/cargo/http-aa340fe0a4` | `0c65ac7e7fc7029e8875da46c9f9dd6cf782c720` | preserved live ref/tag |
| B016 | `refs/remotes/origin/dependabot/cargo/serde-e3e0769812` | `a255c0bfef2ed6ab47b416e49dddd48459e0d577` | preserved live ref/tag |
| B017 | `refs/remotes/origin/dependabot/cargo/tokio-7b53e3ccb2` | `dbc671259ac6b0926390af6e94d6359b516b85cb` | preserved live ref/tag |
| B018 | `refs/remotes/origin/dependabot/github_actions/actions/cache-6` | `31b8ab1a2ff33e46d481a3178c89c2529b3cbca6` | preserved live ref/tag |
| B019 | `refs/remotes/origin/dependabot/github_actions/actions/setup-python-7` | `926b6a866bffe58d2d9e51b37586e007754fceea` | main |
| B020 | `refs/remotes/origin/dependabot/github_actions/actions/upload-artifact-7` | `d974a1096f1b039f480a4615e1f0a61908dd4351` | preserved live ref/tag |
| B021 | `refs/remotes/origin/docs/governance-followup-2026-10-24` | `e9b0861aaadfef09629b4ca1533e8f3bce0a6b72` | main |
| B022 | `refs/remotes/origin/egress-four-claims-2026-10-23` | `b6468b99ef92b88e0dd6cc2b28f8091d5816eee4` | preserved live ref/tag |
| B023 | `refs/remotes/origin/egress-restore-2026-10-23` | `53af1001072db0bc719b1b97088b4df4927022c4` | preserved live ref/tag |
| B024 | `refs/remotes/origin/feat/env-lock-crate-level` | `71631d349ac72b140cce71710c1fdb584b4ca2e4` | preserved live ref/tag |
| B025 | `refs/remotes/origin/feature/cognitive-infrastructure-vnext` | `f1e8ee0e0b6b2802332dafd00023a7dd1f2a2fe6` | main |
| B026 | `refs/remotes/origin/feature/cognitive-memory-convergence-v3` | `b50f9cd25638aa50cde0e65b64196501c89222b7` | main |
| B027 | `refs/remotes/origin/feature/daily-batch-20261003` | `a612f6f1d4aa90fa237283e0f299e24cbbf4f4f3` | main |
| B028 | `refs/remotes/origin/feature/memory-v2.2-production-completion` | `f1e8ee0e0b6b2802332dafd00023a7dd1f2a2fe6` | main |
| B029 | `refs/remotes/origin/feature/mesh-rc2-integration` | `b43ff3fd7316641748adb13af3071afc4ab6dc89` | preserved live ref/tag |
| B030 | `refs/remotes/origin/feature/module-plugin-unification` | `60331ded97e1fcf54b258c62b073848b8a004a95` | preserved live ref/tag |
| B031 | `refs/remotes/origin/fix/bench-idle-footprint-isolation` | `4669315493b1cd5920aa9debd26794e21ddefaa1` | preserved live ref/tag |
| B032 | `refs/remotes/origin/fix/bench-report-command-2026-10-24` | `fb4d9bbadb9ba61c81fe01e6116cd2f1852ee250` | preserved live ref/tag |
| B033 | `refs/remotes/origin/fix/ci-cargo-audit-yaml-2026-10-24` | `50576a88ec077ce3b86a95e91460b61dab85d91f` | main |
| B034 | `refs/remotes/origin/fix/elf-diff-locale-clean2` | `cc0f3961f88337918aa2719dcaafb7ae7550f851` | preserved live ref/tag |
| B035 | `refs/remotes/origin/fix/external-tool-etxtbsy-flake-2026-10-24` | `bb51f380b3ce29b27b72ec6f07749b843c72cca5` | main |
| B036 | `refs/remotes/origin/fix/gate-nested-worktree` | `c715f668fb2494d0307a310b369bcb8487b36e88` | preserved live ref/tag |
| B037 | `refs/remotes/origin/fix/historical-debt` | `196eebff2aaedbb76d534637f6aca4d575402944` | main |
| B038 | `refs/remotes/origin/fuzz/batch2-2026-10-24` | `a02ba4d824900fbbf3dd467387cc5fe5256c3e46` | preserved live ref/tag |
| B039 | `refs/remotes/origin/main` | `4ea9c26095e22b83dc30faaf8f7148ce7cbdb82c` | main |
| B040 | `refs/remotes/origin/open-source-difference-criteria-2026-10-23` | `32a78b1ac38e4ab830d100c5b77a0fe83577387c` | preserved live ref/tag |
| B041 | `refs/remotes/origin/rc/2.0.0-rc1-final` | `3dff564d083d57ee9c2fa40c33affbce0a4a3177` | main |
| B042 | `refs/remotes/origin/refactor/kc1-kernel-convergence` | `cf59fd4904685530f9a773994c859d36bd2c103b` | main |
| B043 | `refs/remotes/origin/replay/phase2c2-base` | `994fd67a033ee54e768542f12f864cd394921f02` | preserved live ref/tag |
| B044 | `refs/remotes/origin/ui-engine-p0-2026-10-24` | `5cfbccb78c7ff2310ae042db8984bc5f9460e027` | preserved live ref/tag |
| B045 | `refs/remotes/origin/unwired-guard-2026-10-24` | `120d96e773854134ca8a7a4ae3b0fb292fe77ace` | preserved live ref/tag |

## Evidence and handoff

Machine-readable merged/closed/open facts, per-ref unique-ID files, original-ID preservation rows, verified tag ledger, comments, cancelled-run receipts and SHA manifests are in `/tmp/apeireth-g3/` and the archived evidence under `/workspace/apeireth-codex/`. New source commits created by this agent and each docs milestone used the requested Codex author wrapper with strict architecture checks; non-executable hooks were additionally executed explicitly, without changing hook configuration. KC1 uses fetch/ancestor-check/ordinary push plus remote full-SHA equality.

Requested D4 scope is executed; **next=done** for this slice. The five real WIP PRs and the external #106 anchor chronology limitation remain explicit follow-ups. Kernel acceptance is qualified by the approved definition and retained environment boundaries, not unconditional completion of all repository or hardware work.
