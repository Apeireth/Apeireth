# Phase C 仓库整合分类与后续顺序建议

[Document-Meta]
Document: docs/04-internal/repo-consolidation-c-classify-2026-10-29.md
Version: 1.0 (classification only; no disposition approval)
Last-Modified: 2026-10-29
Status: C 完成；仅分类建议，D/E/F/G 均未执行
Author: Codex

## 输入、范围与结论

本批以 [B inventory](repo-consolidation-b-inventory-2026-10-29.md) 的 **45 个 branch ref 行、16 个 open PR** 为封闭输入，
每个 B identity ID 恰好分配一个主类。起点 clean `09f1f235769c9fcdcb62fb54cb2fa58aa6cadcb6`，
`git fetch --prune origin` 成功；除活动 KC1 因 B 文档推进外，分支集合及其余 tip 与 B 一致。
B 的冻结 main 为 `4ea9c26095e22b83dc30faaf8f7148ce7cbdb82c`，冻结 KC1 为
`cf59fd4904685530f9a773994c859d36bd2c103b`。分类表保留 B 的原 tip/unique 口径，
JSON 同时保留 C 起点与本轮 PR head；不伪造包含自身文档提交的快照。

分类是建议，**不是 owner 的去留决定、合并批准或删除清单**。本批不改产品 Rust、LOCKED、Cargo.lock 或版本，
不删/改名/合并分支、不创建/关闭 PR、不创建 tag/bundle。只向原 KC1 正常推送本批文档，既有 PR #98 随 head 推进；
main 后续只经 D 的受保护 PR 流程。没有启动 D–G。

| primary class | ref 行数 | 其中 B UNIQUE_UNMERGED 行数 |
| --- | --- | --- |
| ARCHIVE_ONLY | 7 | 5 |
| DEPENDABOT_OR_BOT | 7 | 7 |
| KEEP_ACTIVE | 4 | 1 |
| MERGE_TO_MAIN_CANDIDATE | 4 | 4 |
| NEEDS_OWNER_DECISION | 5 | 5 |
| SUPERSEDED_BY_KC1_OR_MAIN | 9 | 2 |
| UNIQUE_PRESERVE | 9 | 9 |

**KEEP=4 / MERGE_CAND=4 / UNIQUE_PRESERVE=9** 是对应互斥主类的 ref 行计数，
KEEP 包含 KC1 local/origin 两行、main 与活动 UI 项目；不是按去重分支名或提交数计数。
**B UNIQUE_UNMERGED=33 仍完整保留**，跨上述所有类；其中甚至有两行内容已包含但 commit ID 未进 main 的情况。
B 的 31 个 distinct unique tips、3463 个去重历史 commit IDs 的原范围不变；两个外部 fork head 不并入这些数字。
主类 UNIQUE_PRESERVE=9 绝不表示只需保护九行。全体 45 行 `delete_authorized=false`；
所有后续删除都要先验证恢复锚，33 行还必须先合并或取得明确 owner 去留决定。

## 分类规则与证据强度

主类只取用户指定的七类。main/KC1 和仍在进行的 UI 项目归 KEEP_ACTIVE；
备份、archive、freeze/replay 基点优先归 ARCHIVE_ONLY，即使其 tip 已是 main 祖先；bot 单列但保留全部独有提交。
有明确增量价值和可审范围的工作归 MERGE_TO_MAIN_CANDIDATE，base/退出条件不等于整条历史可直接 merge。
冲突版本、膨胀范围或混入测量产物归 NEEDS_OWNER_DECISION；其余未证明可处置的历史归 UNIQUE_PRESERVE。

SUPERSEDED_BY_KC1_OR_MAIN 有两种明确分开的证据：
一是 **7 行 tip 已是 main/KC1 祖先**；另外 B005/B041 的祖先事实保留，但主类按历史冻结归 archive。
二是 **B031/B036 的完整 topic net diff 所涉及文件，在冻结 main 的 blob 与文件模式全部相同**，属于内容已包含，
没有声称这两个 tip 是 main 祖先。B036 的全 net patch 还通过 KC1 `git apply --reverse --check`。
B031 的 KC1 `main.rs` 已增加 A1b 路由，保留现版，不能倒退重放旧文件。
两条 unique-ID 内容包含结论仍须 owner 明确记录 supersession 与验证恢复锚，不能直接删。

`git cherry` 的 `-` 仅表示非 merge commit 的 patch identity 匹配，`+` 不证明业务内容未接线；
merge commit 不在这组加减数里。两个大型旧历史只审元数据与抽样，没有完成逐补丁等价性审查。
B022/B040 虽全部非 merge patch 匹配，但生成 claims/STATUS 已演进，仍保守归 UNIQUE_PRESERVE。
局部文件相同、版本后缀、同 tip alias、bot 作者或 PR 标题都不能单独推导删除或全量合并。

以下责任人为建议的 **repository owner + 主题/PR maintainer 角色**，不是冒称某个人已接受任务；
各行出口配合全局恢复与独有提交保留规则。精确 SHA、patch 匹配数、文件比较与退出条件见 `classification.json`。

## 全部分支分类（唯一主表）

`uniq_both` 是 B 冻结的 main/KC1 之外 commit ID 数。ref 保留 local/origin 的独立身份。

| ID | B ref | primary class | uniq_both | evidence / rationale | suggested base | exit condition |
| --- | --- | --- | --- | --- | --- | --- |
| B001 | `refs/heads/refactor/kc1-kernel-convergence` | KEEP_ACTIVE | 0 | Active local convergence line; B tip advances only by B/C docs. PR #98 remains required. | origin/main via protected PR #98 | Retain active branch; keep formal startup Partial visible in PR review. |
| B002 | `refs/remotes/origin/archive/v1.0-master` | ARCHIVE_ONLY | 2101 | Historical v1 archive; 2101 outside-both commit IDs. No full patch-equivalence review; not a bulk merge candidate. | archive retention; no integration base selected | Owner must identify any wanted legacy deltas; independently verify full-history recovery before any later cleanup. |
| B003 | `refs/remotes/origin/backfill-and-ci-fixes-2026-10-24` | UNIQUE_PRESERVE | 5 | Backfill/privacy/CI plus generated claims: 2 patch matches and 3 unmatched non-merge patches against main/KC1. Generated fingerprints are not replay instructions. | fresh branch from post-KC1 origin/main if residual value remains | Review unmatched changes and present-day claim evidence; preserve original commits until explicit disposition. |
| B004 | `refs/remotes/origin/backup-main-20260901-pre-upstream` | ARCHIVE_ONLY | 2405 | Pre-upstream history has 2405 outside-both IDs; preview tag points at tip but is not a verified recovery anchor. | archive retention | Owner history decision plus separately verified tag/bundle; do not merge historical tree wholesale. |
| B005 | `refs/remotes/origin/backup/memory-v2.2-final-freeze-20260914` | ARCHIVE_ONLY | 0 | Named memory freeze; tip is ancestor of main and shares B025/B028 tip. Archive purpose takes primary-class precedence. | already contained in main | Retain historical freeze or verify archival anchor and owner decision before any later ref cleanup. |
| B006 | `refs/remotes/origin/backup/pre-2c2-upstream-rewrite` | ARCHIVE_ONLY | 14 | Pre-rewrite backup shares B030 tip; 14 unique IDs include module/Mesh history. Alias is not recovery verification. | archive retention; B030/#31 review for wanted content | Verify recovery for exact tip and all unique objects; do not discard rewritten history based on naming. |
| B007 | `refs/remotes/origin/backup/pre-rc2-replay-994fd67a` | ARCHIVE_ONLY | 2 | Pre-replay backup shares B043 tip with 2 unique IDs; canonical Mesh adaptation history must remain recoverable. | archive retention | Owner disposition and verified recovery before any later cleanup; compare with #31 without resetting current runtime. |
| B008 | `refs/remotes/origin/batch-2026-10-22` | SUPERSEDED_BY_KC1_OR_MAIN | 0 | Tip is an ancestor of both frozen main and KC1. | already integrated in origin/main | Phase E: recheck ancestry/PR/worktree state and verify recovery anchor before any cleanup. |
| B009 | `refs/remotes/origin/bio-switches-and-w5-2026-10-24` | NEEDS_OWNER_DECISION | 2 | First bio/W5 variant: 2 unique IDs, only 1 patch match; competing v2/v3 variants differ in docs/claims and switch implementation. | post-KC1 origin/main only after choosing one variant | Owner selects authoritative variant or records supersession after semantic comparison; preserve all three meanwhile. |
| B010 | `refs/remotes/origin/bio-switches-and-w5-v2` | NEEDS_OWNER_DECISION | 4 | Second bio/W5 variant: 4 unique IDs, 1 matching and 3 unmatched patches. Version suffix alone does not establish supersession. | post-KC1 origin/main after owner selection | Compare B009/B011 switches, defaults, docs and claims; no automatic newest-name winner. |
| B011 | `refs/remotes/origin/bio-switches-w5-v3` | NEEDS_OWNER_DECISION | 4 | Third bio/W5 variant: 4 unique IDs; 1 matching and 3 unmatched patches. No ancestry or complete-content proof of replacing earlier variants. | post-KC1 origin/main after owner selection | Keep defaults and governance invariants; document owner-selected variant and remaining deltas. |
| B012 | `refs/remotes/origin/coverage/baseline-2026-10-24` | UNIQUE_PRESERVE | 1 | Historical local coverage baseline differs from current CI denominator/calibration. Replaying old baseline could misstate regression scope. | no merge until same-scope coverage review | Owner reviews residual script logic versus current CI-calibrated baseline; do not restore obsolete measurement values. |
| B013 | `refs/remotes/origin/debt-round8` | NEEDS_OWNER_DECISION | 249 | PR #94 grew from a title claiming 28 commits to API 249 commits / 393 files; spans memory, production wiring and policy. KC1 overlap needs explicit scope decisions. | fresh scoped PRs from post-KC1 origin/main | Owner approves split/disposition with security, schema, retrieval and kernel-interface review; original 249 IDs preserved. |
| B014 | `refs/remotes/origin/dependabot/cargo/dependencies-092765a05c` | DEPENDABOT_OR_BOT | 2 | PR #29 grouped dependency update is not present at target versions in current lockfile; stale bot branch also has merge history. | post-KC1 origin/main; normal reviewed dependency PR | Refresh deliberately, audit resulting lock delta and run affected tests; no bot auto-delete. |
| B015 | `refs/remotes/origin/dependabot/cargo/http-aa340fe0a4` | DEPENDABOT_OR_BOT | 3 | PR #22 hyper-util target is absent from current lockfile; dirty/stale base and transitive changes require review. | post-KC1 origin/main | Refresh on fresh branch or additive merge; validate dependency/security/MSRV changes without rewriting history. |
| B016 | `refs/remotes/origin/dependabot/cargo/serde-e3e0769812` | DEPENDABOT_OR_BOT | 1 | PR #33 serde_json update is not already in current lockfile; includes resolver drift beyond package version. | post-KC1 origin/main | Keep pending core baseline; isolate intended version change and verify lockfile/tests. |
| B017 | `refs/remotes/origin/dependabot/cargo/tokio-7b53e3ccb2` | DEPENDABOT_OR_BOT | 1 | PR #97 Tokio group target versions are absent from current lockfile; async lifecycle changes need post-KC1 regression coverage. | post-KC1 origin/main | Review runtime/TLS and incidental resolver changes; rerun async cancellation and transport tests. |
| B018 | `refs/remotes/origin/dependabot/github_actions/actions/cache-6` | DEPENDABOT_OR_BOT | 2 | PR #17 workflow cache action versions remain older in current workflows; no full supersession proof. | post-KC1 origin/main after #64 | Refresh workflow changes; validate runner/action compatibility and required checks. |
| B019 | `refs/remotes/origin/dependabot/github_actions/actions/setup-python-7` | DEPENDABOT_OR_BOT | 1 | PR #96 setup action update remains unapplied in current arch/docs workflows. | post-KC1 origin/main | Wait for convergence baseline, then validate workflow compatibility and required-check execution. |
| B020 | `refs/remotes/origin/dependabot/github_actions/actions/upload-artifact-7` | DEPENDABOT_OR_BOT | 2 | PR #20 artifact action update remains unapplied in target guard workflow. | post-KC1 origin/main | Refresh and exercise artifact evidence path; do not infer obsolete noise from bot ownership. |
| B021 | `refs/remotes/origin/docs/governance-followup-2026-10-24` | SUPERSEDED_BY_KC1_OR_MAIN | 0 | Governance follow-up tip is ancestor of main and KC1. | already integrated in origin/main | Recheck refs/PR/worktree and verify recovery before Phase E cleanup. |
| B022 | `refs/remotes/origin/egress-four-claims-2026-10-23` | UNIQUE_PRESERVE | 1 | One unique commit has a matching patch in main/KC1, but generated claim/STATUS files have evolved; no complete tip-content proof. | no new merge unless residual claim value is proven | Owner may record patch-equivalent supersession after claim reconciliation; retain ID and recovery requirement. |
| B023 | `refs/remotes/origin/egress-restore-2026-10-23` | UNIQUE_PRESERVE | 20 | Egress history has 20 unique IDs; 17 patch matches and 3 unmatched patches, including generated state/documentation. Majority matching is not full equivalence. | post-KC1 origin/main if specific residual is validated | Review unmatched deltas and consequences/authorization wiring; never roll back newer claims or runtime assembly. |
| B024 | `refs/remotes/origin/feat/env-lock-crate-level` | MERGE_TO_MAIN_CANDIDATE | 62 | PR #95 has valuable crate-wide test environment-lock work absent as CRATE_ENV_LOCK in current test_support. Current scope is 62 commits / 114 files, beyond its original narrow title. | post-KC1 origin/main; prefer focused test-support PR first | Owner scopes remaining work; verify cross-module serialization, no weakened assertions, security tests, clippy and all affected tests. |
| B025 | `refs/remotes/origin/feature/cognitive-infrastructure-vnext` | SUPERSEDED_BY_KC1_OR_MAIN | 0 | Tip already ancestor of main/KC1 and identical to memory freeze B005/B028. | already integrated in origin/main | Preserve independently verified recovery before any Phase E ref cleanup. |
| B026 | `refs/remotes/origin/feature/cognitive-memory-convergence-v3` | SUPERSEDED_BY_KC1_OR_MAIN | 0 | Cognitive-memory convergence tip is ancestor of main and KC1. | already integrated in origin/main | Recheck graph, PR/worktree use and verified recovery before cleanup. |
| B027 | `refs/remotes/origin/feature/daily-batch-20261003` | SUPERSEDED_BY_KC1_OR_MAIN | 0 | Daily batch tip is ancestor of main and KC1. | already integrated in origin/main | Do not replay completed work; verify anchor and owner/worktree checks before cleanup. |
| B028 | `refs/remotes/origin/feature/memory-v2.2-production-completion` | SUPERSEDED_BY_KC1_OR_MAIN | 0 | Memory completion tip is ancestor of main/KC1; shared tip does not itself verify recovery. | already integrated in origin/main | Phase E anchor and owner/worktree review before any cleanup. |
| B029 | `refs/remotes/origin/feature/mesh-rc2-integration` | MERGE_TO_MAIN_CANDIDATE | 43 | PR #31 carries distinct Mesh/NAT integration value, but old clean/fast-forward body is stale: current API says dirty; 43 unique IDs remain. | fresh integration branch from post-KC1 origin/main | Review only missing Mesh deltas against existing Mesh/Kernel ports; preserve pairing/trust, bounded failure and experimental real-NAT boundary; tests/CI before protected PR. |
| B030 | `refs/remotes/origin/feature/module-plugin-unification` | UNIQUE_PRESERVE | 14 | Module/Mesh tip shares backup B006. 14 unique IDs, 12 unmatched and 2 matching patches; rewritten APIs cannot safely be replayed into converged Kernel. | post-KC1 origin/main only for reviewed missing deltas | Compare #31, module ownership and current interfaces; owner records what remains valuable; no wholesale old-runtime merge. |
| B031 | `refs/remotes/origin/fix/bench-idle-footprint-isolation` | SUPERSEDED_BY_KC1_OR_MAIN | 1 | Content-contained: both files in branch net diff are byte-identical at frozen main; KC1 keeps identical idle_footprint.rs with additive main routing changes. Commit ID remains unique. | already content-contained in main; keep current KC1 routing | Owner explicitly records content supersession plus verified recovery before later deletion; graph ancestry is NOT claimed. |
| B032 | `refs/remotes/origin/fix/bench-report-command-2026-10-24` | UNIQUE_PRESERVE | 2 | Bench workflow file exactly matches current main/KC1, but inherited coverage commit/files do not. Full branch is not proven redundant. | no workflow replay; review coverage residual against current main | Separate applied workflow from historical coverage calibration and obtain owner disposition. |
| B033 | `refs/remotes/origin/fix/ci-cargo-audit-yaml-2026-10-24` | MERGE_TO_MAIN_CANDIDATE | 3 | PR #64 fixes duplicate YAML if keys and missing always condition on SARIF conversion, still visible in main/KC1; lockfiles also retain older targeted package versions. | current origin/main via protected PR #64; then refresh #98 | Review workflow parse/evidence failure path and narrow lock delta; fresh security scan and affected CI required, no stale vulnerability-count claims. |
| B034 | `refs/remotes/origin/fix/elf-diff-locale-clean2` | NEEDS_OWNER_DECISION | 3 | Locale fixes partly match current files, but topic includes two binary .profraw artifacts and different doc_ops fixture isolation. Net topic merge would mix code and measurement artifacts. | fresh focused test PR from post-KC1 origin/main if needed | Owner reviews residual fixture isolation and artifact disposition; preserve originals; no assertion weakening or bulk replay. |
| B035 | `refs/remotes/origin/fix/external-tool-etxtbsy-flake-2026-10-24` | MERGE_TO_MAIN_CANDIDATE | 1 | PR #70 adds bounded ETXTBSY-only retry to Unix test fixtures; helper is absent from current external.rs. One unique commit; narrow flaky-test value. | post-KC1 origin/main; refresh PR #70 without force | Check narrowed retry failure semantics and updated fixture call sites; repeat affected Unix tests and clippy; never mask real failures. |
| B036 | `refs/remotes/origin/fix/gate-nested-worktree` | SUPERSEDED_BY_KC1_OR_MAIN | 1 | Content-contained: all three changed documentation scripts are byte-identical in main/KC1; complete net patch passes reverse --check on KC1. Commit ID remains unique. | already content-contained in main/KC1 | Owner records exact-content supersession and verifies recovery anchor before Phase E deletion; not graph ancestry. |
| B037 | `refs/remotes/origin/fix/historical-debt` | SUPERSEDED_BY_KC1_OR_MAIN | 0 | Historical-debt tip is ancestor of main and KC1. | already integrated in origin/main | Recheck PR/worktree/graph and verify recovery anchor before cleanup. |
| B038 | `refs/remotes/origin/fuzz/batch2-2026-10-24` | UNIQUE_PRESERVE | 3 | Fuzz nightly command and content_sanitize already exist; other fuzz target/manifest/lock changes differ. No complete residual semantic proof. | post-KC1 origin/main only for missing target improvements | Compare current expanded fuzz matrix and assertions; retain branch until specific missing value or supersession is decided. |
| B039 | `refs/remotes/origin/main` | KEEP_ACTIVE | 0 | Protected main is the integration baseline; do not delete, directly push or merge locally. | main changes only through protected PRs in Phase D | Keep branch protections and current required checks; no cleanup recommendation. |
| B040 | `refs/remotes/origin/open-source-difference-criteria-2026-10-23` | UNIQUE_PRESERVE | 2 | Both unique commits have patch matches; privacy guide and README blobs match main/KC1, while generated claims have evolved. Preserve until explicit semantic disposition. | no replay of matching privacy content | Reconcile generated claims and owner-recorded supersession; then verified recovery before any later cleanup. |
| B041 | `refs/remotes/origin/rc/2.0.0-rc1-final` | ARCHIVE_ONLY | 0 | Historical RC1 freeze is ancestor of main/KC1 and has exact-tip rc.1 tag; existing tag is not a completed recovery verification. | archive/release-history retention | Preserve release history and verify recovery before any optional ref cleanup. |
| B042 | `refs/remotes/origin/refactor/kc1-kernel-convergence` | KEEP_ACTIVE | 0 | Origin mirror of active KC1 and PR #98; not an extra obsolete topic. | origin/main via protected PR #98 | Retain active convergence line and normal non-force pushes; formal acceptance remains Partial. |
| B043 | `refs/remotes/origin/replay/phase2c2-base` | ARCHIVE_ONLY | 2 | Replay base shares B007 tip and retains 2 unique commit IDs; historical adaptation base rather than ready current-runtime merge. | archive retention; compare #31 only if needed | Owner disposition and verified exact-tip/full-object recovery before any later cleanup. |
| B044 | `refs/remotes/origin/ui-engine-p0-2026-10-24` | KEEP_ACTIVE | 183 | PR #78 is active independent UI-engine work with 183 unique IDs / 870 changed files. Keep project available; current L1 docs do not imply production launch readiness. | post-KC1 origin/main only after project-owner readiness/scope review | Wait for KC1; review independent workspace/hooks/CI and readiness limits before any integration; no current merge/delete recommendation. |
| B045 | `refs/remotes/origin/unwired-guard-2026-10-24` | UNIQUE_PRESERVE | 2 | Unwired guard history has 2 unique IDs, 1 matching and 1 unmatched patch; current scripts and generated baselines have evolved. | post-KC1 origin/main only for a verified remaining guard defect | Compare current scanner/workflow and governance defaults; retain assertions and reconcile claims rather than replay old fingerprints. |

## 16 个 open PR：建议而非已执行动作

本轮只读 `gh api` 重新读取 B 的每个 PR 与 files 列表，并复查当前 open 集合仍为原 16 项；
main API 仍 `protected=true` 且 SHA 不变，未审计或更改保护细项。
#98 head 因 B 推进到 C 起点，其余 B head 一致；本批交付后 #98 再随 C 文档推进。
`keep=2 / wait-for-kc1=7 / rebase-onto-main=7 / close-as-superseded=0`。
没有足够证据建议把某个 open PR 直接关闭为全量冗余；bot 的目标版本在相应当前文件中仍未应用。

**rebase-onto-main 是后续内容刷新建议，不授权重写已发布 refs 或 force-push。**
D 可与作者协调在新分支构造聚焦替代 PR，或向原 topic 追加 main merge 后普通推送；原 head 保留到去留决定与恢复完成。
本批没有执行 rebase、创建替代分支或 PR。

| PR | suggested action | suggested base | rationale | required exit / review |
| --- | --- | --- | --- | --- |
| [98](https://github.com/Apeireth/Apeireth/pull/98) | keep | main via existing protected PR #98 | Convergence is active. B/C docs advance head; retain A4 software evidence and unchanged formal Partial, unapproved startup definition, unavailable true cold cache. | After #64 CI/security repair if needed, refresh latest checks/base without force; owner assesses Partial honestly before protected merge. |
| [97](https://github.com/Apeireth/Apeireth/pull/97) | wait-for-kc1 | post-KC1 main | Tokio/TLS versions remain unapplied; lock and async lifecycle interaction with converged runtime. | Review package-only delta and rerun async/transport tests after #98. |
| [96](https://github.com/Apeireth/Apeireth/pull/96) | wait-for-kc1 | post-KC1 main | Setup action version remains unapplied; keep CI baseline stable while core converges. | Validate runner compatibility and actual required-check job execution. |
| [95](https://github.com/Apeireth/Apeireth/pull/95) | wait-for-kc1 | post-KC1 main; focused new PRs if splitting | Crate-wide lock work valuable; observed 62 commits / 114 files exceed title. Cannot assume only test locking. | Owner scope decision, compare expanded security/fuzz work, test strict serialization/assertions and CI before integration. |
| [94](https://github.com/Apeireth/Apeireth/pull/94) | wait-for-kc1 | post-KC1 main; owner-approved scoped PRs | Observed 249 commits / 393 files versus title/body 28; broad memory/assembly changes overlap KC1. | Owner splits or approves explicit scope; preserve all unique commits and rerun relevant schema/security/Kernel gates. |
| [78](https://github.com/Apeireth/Apeireth/pull/78) | wait-for-kc1 | post-KC1 main after UI owner readiness | Active UI project is preserved; 183 commits / 870 files include workspace/hooks/CI. Title/body are not a production readiness verdict. | Owner readiness/milestone decision and independent UI/wasm/CI validation; no forced production cutover. |
| [70](https://github.com/Apeireth/Apeireth/pull/70) | rebase-onto-main | post-KC1 main; refreshed focused PR | Narrow ETXTBSY-only Unix fixture retry is absent in current external.rs, which KC1 also modifies. | Refresh additively or on new branch, preserve retry bounds/error assertions and repeat targeted fixture tests. |
| [64](https://github.com/Apeireth/Apeireth/pull/64) | keep | current main via existing protected PR #64 | Still-needed YAML duplicate-key, audit-evidence conversion and lock fixes; review first because broken workflow can undermine later required-check evidence. | Parse workflows, exercise failed-audit artifact/SARIF path, review both lockfiles and scan current advisories; do not import obsolete lock resolution blindly. |
| [34](https://github.com/Apeireth/Apeireth/pull/34) | wait-for-kc1 | post-KC1 main; fork owner refresh | Fork desktop preference/titlebar change is not in 45-ref inventory; 19 files, 1 commit, intersects desktop bootstrap reviewed by KC1. | Owner retains fork head, reviews defaults and targeted desktop/frontend tests on updated baseline; no invented graph uniqueness. |
| [33](https://github.com/Apeireth/Apeireth/pull/33) | wait-for-kc1 | post-KC1 main | Serde version remains unapplied; lock includes incidental resolver delta. | Isolate intended update and validate serialization/protocol plus lock reproducibility. |
| [31](https://github.com/Apeireth/Apeireth/pull/31) | rebase-onto-main | post-KC1 main; new integration branch | Old body claims clean FF against an earlier main; current read says dirty and 43 commits. Preserve Mesh/NAT value without reintroducing old runtime paths. | Resolve selected deltas through current ports; pairing/trust/loopback/namespace checks, keep real-NAT limits explicit. |
| [29](https://github.com/Apeireth/Apeireth/pull/29) | rebase-onto-main | post-KC1 main | Grouped versions are not current; stale merged main history and shared Cargo.lock need current resolution. | Fresh or additive refresh, package/security/MSRV review and affected tests, retain old head until disposition. |
| [22](https://github.com/Apeireth/Apeireth/pull/22) | rebase-onto-main | post-KC1 main | Hyper-util and transitive lock additions remain unapplied; current API dirty. | Regenerate only intended dependency delta and validate transport/TLS and security checks. |
| [20](https://github.com/Apeireth/Apeireth/pull/20) | rebase-onto-main | post-KC1 main | Target guard workflow still uses older artifact action; cannot close as redundant from bot label. | Validate artifact semantics and runner compatibility with full evidence upload before integration. |
| [19](https://github.com/Apeireth/Apeireth/pull/19) | rebase-onto-main | fresh guide-only branch from post-KC1 main, coordinated with fork owner | API scope is 2577 commits / 4596 files despite guide-only description; files API returned 3000, so full content is unreviewed. Never merge this broad historical diff as documentation. | Preserve fork head separately, extract/rewrite only intended guide against current APIs, then owner-reviewed protected replacement PR; do not force-rebase existing fork. |
| [17](https://github.com/Apeireth/Apeireth/pull/17) | rebase-onto-main | post-KC1 main after #64 | Cache action target versions remain unapplied in audit/deny workflows; intersects first CI repair. | Refresh from repaired workflows and validate compatible runners/cache/evidence behavior. |

PR 文件列表与正文只是分类证据，不是本轮安全/语义逐行审计。#94 实际 **249 commits / 393 files**，
#95 **62 / 114**，均超出原标题/正文的窄范围；#78 **183 / 870** 保留活动项目身份，不把 L1 阶段认作生产可上线。
#31 正文的旧「clean fast-forward」已过时，本轮 API `mergeable_state=dirty`，只能按更新后的 main 再审。
API mergeable_state 是短时观察，不等于 required checks 通过或批准合并。

#19 是重点保留与拆范围项：描述是单篇 guide，API 却报告 **2577 commits / 4596 files**；
files 端点实际仅返回 **3000** 行，**完整文件列表与完整内容未审**。建议只在新的 main 基线上重建 intended guide，
绝不把这个历史大 diff 当文档 merge。#34 是 **1 commit / 19 files** 的桌面默认开关/标题栏工作，需结合 KC1 desktop bootstrap 复验。
这两个 fork head 不在 B 的 origin branch 集合；未抓取 fork、未计算或虚构 unique 数，后续由作者/owner 先保证可恢复。
其余 PR 文件行数与 API changed_files 相符，但不表示所有 patch 已逐行认证。

## Phase D/E/F 推荐顺序（未执行）

| sequence | later phase | proposed work | dependency / preservation condition |
| --- | --- | --- | --- |
| D0 | D 准备 | 重新 fetch、查保护规则/required checks、核对 B/C 后 ref/PR drift；先与 owner 固定需要保留的 exact heads | 所有本批建议重验；保留 fork heads；不从名字推导授权，不直接更新 main |
| D1 | D 第一项 | 先审 #64 的 CI YAML、失败时 SARIF/artifact 证据链与两处 lock 修复，经受保护 PR | 当前 main/KC1 仍有重复 if 键，转换步缺 always；本批只记录，不修。重新扫描 advisory、验证 workflow，不能拿历史漏洞数作现状 |
| D2 | D 核心收敛 | 保持 #98 为活动主线，在更新后的 main 基线上复核 CI 与合并范围，再走受保护 PR | A4 软件 PASS 是独立历史证据；正式 startup 仍 Partial，owner 不得把 PR 合并叙述成 Complete。旧门槛不变，新定义未批准 |
| D3 | D 聚焦修复 | #70 Unix fixture 稳定性、#95 优先抽出 crate-wide env-lock，再审其余增量；#94 按 owner 批准范围拆主题 | 避免 249/62 commit 分支直接覆盖 KC1 Kernel/assembly/security；无强推，用新聚焦分支或追加 main merge，重新跑受影响测试 |
| D4 | D 独立能力与外部贡献 | #31 选择缺失 Mesh/NAT 增量适配现有 Kernel ports；#34 桌面变化、#19 guide-only 替代；#78 按项目 readiness 决定 | 保留 pairing/trust、实验 NAT 与硬件边界；fork 原 head 先保留；独立 UI workspace/hooks/CI 需 owner 审查，不能顺带生产切换 |
| D5 | D 依赖与 CI 更新 | #29/#22/#33/#97 逐项刷新 lock；#17/#20/#96 在修复后的 workflows 上审兼容与 required checks | 有实际安全紧迫性时 owner 可前移某项；本批未重新审全量 advisory。串行消解共享 Cargo.lock，勿覆盖 #64 修复，bot 不能自动丢弃 |
| E1 | E 可包含主题复核 | 先复核 9 个 SUPERSEDED 主类；B005/B041 的历史角色单独保留 | 图祖先的 7 行也要可恢复锚；B031/B036 是 unique-ID 内容包含，额外需要明确 owner supersession 决定。重验当前 PR/worktree 使用后才可能清理 |
| E2 | E archive 与未决项 | 7 个 ARCHIVE_ONLY 优先保留为档案；其余 unique 工作逐项 merge 或 owner disposition | 备份/alias/现有 tag 不是恢复核验；不能因 E1 完成就批量删余下 refs。KEEP_ACTIVE 的 main/KC1 不列为清理候选，UI 活动项目继续保留 |
| F proposal | F 后续收口建议，具体范围以后续用户任务为准 | D/E 完成后重新盘点 refs/PR、核对 disposition/recovery 台账及新 main 的验证证据 | 只在后续授权阶段执行；检查独有提交可达性和恢复演练、未决 backlog；未解决项继续保留，不为收口美化 Complete |

D 的主线优先于 E 清理；E 的任何删除之前都要完成恢复核验与对应去留条件。
推荐恢复记录至少包含：原 ref + exact tip、全部独有对象覆盖、tag/bundle 保存位置、对象可达性与恢复验证命令结果，
并在动作前重验 PR/worktree/保护状态。C **没有创建或验证恢复锚**；JSON、SHA 清单、同 tip alias 和旧 tag 线索均不是已验证的锚。
没有完整内容审查的旧 archive 不安排 wholesale merge；bio 三变体与测量产物混入项先由 owner 决定。

## 验证与证据

采集/分类日期 **2026-10-29**；工具链 Git 2.47.3、gh 2.103.0、Python 3.13.5、PowerShell 7.5.4，
实测版本/环境见 `c-toolchain.log`；每条日志记录命令、UTC 与真实退出码。
原始与 JSON：`/tmp/apeireth-c/`，收口归档 `/workspace/apeireth-codex/c-evidence/`；
根目录 `c-*.log` 与同名 `.exit` 同时镜像到 `/workspace/apeireth-codex/`。
`classification.json` 保存 B 完整 identity/tip、互斥主类、unique 保留要求、PR 建议、base、理由和退出条件。
`branch-review.json`、`patch-review.json`、`pr-*.json`、`supplement.json` 与只读调用回执保存原始观察；
两个巨大历史仅元数据/抽样，#19 files 截断如上，不能把采集成功当作所有内容验证成功。

以下工具在 evidence 中，读取冻结 B 输入并产生/验证分类；`inspect.py` 要求 clean C 起点与相同 refs，
应在对应基线副本复跑采集，不把推进后的 KC1 当作原快照。`classify.py` / `render.py` 可用归档 JSON 确定性重建主表：

```bash
python /tmp/apeireth-c/run.py start bash -c 'git fetch --prune origin && git status && git log -3 --oneline'
python /tmp/apeireth-c/inspect.py
python /tmp/apeireth-c/patch_review.py
python /tmp/apeireth-c/supplement.py
python /tmp/apeireth-c/classify.py
python /tmp/apeireth-c/render.py
python /tmp/apeireth-c/verify.py
python scripts/docs_links.py --check
python scripts/docs_index.py --check
python scripts/docs_truth.py --verify --strict
python scripts/docs_truth.py --check
python scripts/arch_guard.py --strict
```

验证器逐 ID 检查 B 的 45 行恰好在唯一主表出现一次、七类互斥、33 个 unique rows/完整 SHA 集合未丢失、
16 个 PR 恰好一次、内容包含行 blob/模式证据、未来恢复/owner 决定标记，以及精确三文件 docs scope。
交付日志和 `C-RESULT.md` 记录实际退出码、Codex commit.ps1 + ArchStrict、fetch 后普通快进 push 及远端完整 SHA。
Git 当前不执行缺少 executable bit 的 hook，本批另用 Bash 执行两个完整 hook 体；不改 hook 模式/配置，不绕过。
纯分类文档没有新增 Rust 验证范围，不重跑 full gate 或性能矩阵；不把 A4 通过冒称 C 新软件测试。

[Final Acceptance](kernel-convergence-final-acceptance-2026-10-28.md) 保持正式 **Partial**：旧 Dev cold-start 门槛未过，
normal_start 新定义仍 PROPOSED_UNAPPROVED，真实冷缓存不可用。C 不改变这些结论或测试真源。
交付 STATE 为 **C done / next=D**；以上顺序均为后续建议，D–G 未启动。
