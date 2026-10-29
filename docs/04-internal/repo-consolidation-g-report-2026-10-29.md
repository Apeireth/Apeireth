# G Repository consolidation report

> Historical D2–G snapshot: the integration state and inventory below are frozen history. Current #98/D4 merges, preserved work and final main verification are recorded in [G3](repo-consolidation-d4-g3-report-2026-10-29.md).

[Document-Meta]
Document: docs/04-internal/repo-consolidation-g-report-2026-10-29.md
Version: 1.0
Last-Modified: 2026-10-29
Status: Kernel Convergence Complete (under approved startup definition)
Integration-Status: #98 OPEN/BLOCKED；main 尚未含 KC1，剩余工作已保留
Author: Codex

## 当前结论与范围

**Kernel Convergence Complete (under approved startup definition)**。
这是 KC1 的限定验收判断；**不是全部仓库工作已并入 main、#98 checks 全绿或所有冷缓存/硬件已通过**。
本轮按授权完成 D2→E→F→G 的执行/证据收口，仍把 #98 与未整合主题作为明确后续，不靠清理分支隐藏它们。

批准日期 **2026-10-29（Asia/Shanghai）**，批准者 **repository owner / user (xiao ouyang)**，答复词 **「当然」**。
定义原文，状态 **APPROVED**：

> With existing-DB **normal_start P50 ≤ 10 ms** as the **daily core startup criterion**
> (existing DB, full process restart, normal OS caches; same READY/session-pair assertions as A1b).

采用保留的 A1b Dev normal_start P50 **7.5642 ms（PASS）** 与 A4 **GATE_FULL=0 / 必需软件 PASS**，
见 [Final Acceptance](kernel-convergence-final-acceptance-2026-10-28.md)。无新性能或硬件样本。
原 A4 **OLD_COLD_P50=11.4143 ms，原 <10 ms unmet 历史** 仍报告，不再是批准后日常 blocker；
真实 cold_start **UNAVAILABLE**（真实 drop_caches 写被拒，Dev/Release n=0，环境限制，非 PASS）。
first_run targets、relative+absolute regression detection 和其他附加预算仍为提案，未静默丢弃或扩大批准。

## A–G 交付索引

| slice | delivered result / evidence | retained boundary |
| --- | --- | --- |
| A1 | [Memory first-init 优化](kernel-convergence-a1-cold-start-2026-10-29.md)，源 1497fe7f、文档 4de95fb9 | 原 Dev gate 未过；迁移/约束不跳过 |
| A2 | [ignored tests 审计与安全恢复](kernel-convergence-a2-ignored-tests-2026-10-29.md)，收口 f630f03a | 未删除测试、未弱化断言；手工/live/硬件项保持明确依赖，数量真源不重写 |
| A3 | [legacy production audit](kernel-convergence-a3-legacy-audit-2026-10-29.md)，7a5718fd | 未发现需改源码的独立生产后端；历史 CLI EPERM 由 A4 原夹具复验补齐，未冒充外部 live |
| A1b | [四种独立启动与断言](kernel-convergence-a1b-startup-definitions-2026-10-29.md)，源 100b7aa9、原文档 fffb712e | 原 legacy benchmark 保留；真实 cold cache unavailable |
| A4 | [五段/指定软件复验](kernel-convergence-a4-final-gates-2026-10-29.md)，cf59fd49 | 全量 PASS 属该冻结源码，非本轮新 main 验证；平台只在已记录范围内 |
| Approval | 3432d341：owner 明确批准日常定义、qualified Kernel judgment，保留旧数据 | 不授予绕过 PR/保护或无锚删除的许可 |
| B | [45 refs / 16 open PR 初始盘点](repo-consolidation-b-inventory-2026-10-29.md)，09f1f235 | 冻结快照不追溯改写，旧 unique-ID count 与 patch 价值不混同 |
| C | [互斥分类与 D/E/F 顺序](repo-consolidation-c-classify-2026-10-29.md)，cb41c09d | 分类不是未整合 unique 工作的删除授权 |
| D1 | [零合并与策略/CI 阻塞](repo-consolidation-d-merge-2026-10-29.md)，4b49f2df | 当时 no-delete 与 auto-delete 冲突，未旁路 |
| D2 | [先建 verified tags 后受保护 merge](repo-consolidation-d2-anchored-merges-2026-10-29.md)，efa7d7d6 | #64/#70 merged；#98 required failures 留 open |
| E | [恢复账与逐 ref 清理](repo-consolidation-e-cleanup-2026-10-29.md)，d953fa3d | 11 verified tags（9新+2复核）、7 E deletions；未决 unique/档案/owner/fork/活动线保留 |
| F | [exact main detached 验证](repo-consolidation-f-main-verify-2026-10-29.md)，22557719 | 六项 docs/architecture pass；full software gate 延后，main 尚非最终整合 candidate |
| G | 本报告与 `consolidation.json` / 完整 unique-ID 清单 | 执行收口 next=done，不代表剩余 PR/安全/软件验证自动完成 |

## Main 与受保护合并

| milestone | main full SHA |
| --- | --- |
| task start | `4ea9c26095e22b83dc30faaf8f7148ce7cbdb82c` |
| #64 ordinary merge | `554c964b3f7e6e0e7851ccfc6abd10f176e982cf` |
| #70 ordinary merge / final main | `ac73436c65bdbb34369421c2e8d896bf538e591d` |

两项均在 fresh CLEAN/MERGEABLE、六个 required SUCCESS 后，用 `gh pr merge --merge --match-head-commit <fullsha>`。
先创建/推送并验证 exact-head tag，再执行 merge。main first-parent 新增恰为上述两次 merge commit，
没有本地 merge main、direct/force push main、设置/保护修改、admin bypass 或 auto-merge 排队。
#64/#70 的 head 自动删除各自均发生在恢复锚验证之后；#70 即时仍在/随后 absent 的时序分别保留于 D2/E。
KC1/#98 未 merge，活动线从未删除，不虚报 active-line recreate；将来若可合并仍须先锚并立即恢复被自动删的 KC1。

## 恢复与清理总账

11 个不同 recovery tags 已 local/remote exact SHA、missing objects=0、connectivity fsck exit0 核验；
旧 9 个 tags 对象也与 B/远端一致，共 **20 tags**。没有签名或全库 clone 演练声明，也未创建 bundle。
恢复时先 fetch exact tag、rev-parse，再 ordinary push `refs/tags/<tag>:refs/heads/<original>`；
每条完整原 ref 和命令均在 E ledger，遇到新工作不能强推覆盖。

| recovery tag | exact original SHA | original ref |
| --- | --- | --- |
| `recovery/pre-cleanup-b008-b613f0eb` | `b613f0ebb9da45b761f7e5d06a383ebf7a50cbce` | `refs/heads/batch-2026-10-22` |
| `recovery/pre-cleanup-b021-e9b0861a` | `e9b0861aaadfef09629b4ca1533e8f3bce0a6b72` | `refs/heads/docs/governance-followup-2026-10-24` |
| `recovery/pre-cleanup-b025-f1e8ee0e` | `f1e8ee0e0b6b2802332dafd00023a7dd1f2a2fe6` | `refs/heads/feature/cognitive-infrastructure-vnext` |
| `recovery/pre-cleanup-b026-b50f9cd2` | `b50f9cd25638aa50cde0e65b64196501c89222b7` | `refs/heads/feature/cognitive-memory-convergence-v3` |
| `recovery/pre-cleanup-b027-a612f6f1` | `a612f6f1d4aa90fa237283e0f299e24cbbf4f4f3` | `refs/heads/feature/daily-batch-20261003` |
| `recovery/pre-cleanup-b028-f1e8ee0e` | `f1e8ee0e0b6b2802332dafd00023a7dd1f2a2fe6` | `refs/heads/feature/memory-v2.2-production-completion` |
| `recovery/pre-cleanup-b031-46693154` | `4669315493b1cd5920aa9debd26794e21ddefaa1` | `refs/heads/fix/bench-idle-footprint-isolation` |
| `recovery/pre-cleanup-b036-c715f668` | `c715f668fb2494d0307a310b369bcb8487b36e88` | `refs/heads/fix/gate-nested-worktree` |
| `recovery/pre-cleanup-b037-196eebff` | `196eebff2aaedbb76d534637f6aca4d575402944` | `refs/heads/fix/historical-debt` |
| `recovery/pre-merge-pr64-50576a88` | `50576a88ec077ce3b86a95e91460b61dab85d91f` | `refs/heads/fix/ci-cargo-audit-yaml-2026-10-24` |
| `recovery/pre-merge-pr70-bb51f380` | `bb51f380b3ce29b27b72ec6f07749b843c72cca5` | `refs/heads/fix/external-tool-etxtbsy-flake-2026-10-24` |

E 发起的七次删除仅为 **B008, B021, B025, B026, B027, B028, B037**，均是 main 图祖先，且在删除前逐一 fresh 核对 tag、tip、open PR 与 worktree。
另两项是 D2 合并后的 head 自动删除，不计入 E DELETED=7。C 的 unique-ID 内容包含行 B031/B036 只有恢复锚、原 ref 继续保留。
所有 ARCHIVE_ONLY、UNIQUE_PRESERVE、NEEDS_OWNER_DECISION、KEEP_ACTIVE 和 fork 均未作为“丢弃工作”关闭/删除。

## 收口时 refs / 未整合工作

快照 UTC `2026-10-29T02:32:20.996482+00:00`，比较冻结 main `ac73436c65bdbb34369421c2e8d896bf538e591d` 与 **G 文档提交前 KC1** `22557719eba868afb52b4eb2c7d78ca166d4137c`。
G docs 自身会再推进 KC1；最终 SHA/remote 回执在 STATE/RESULT，不伪造自包含 SHA。

| metric | initial B | current G snapshot | meaning |
| --- | ---: | ---: | --- |
| branch refs | 45 | 36 | local/origin 分行；当前 1 local + 35 origin，不计 origin/HEAD |
| unique-unmerged refs | 33 | 31 | 至少一个 commit ID 在 main/KC1 之外的 ref 行；不是主类 UNIQUE_PRESERVE 数 |
| distinct unique tips | 31 | 29 | shared tips 去重 |
| unique commit-ID union | 3463 | 3459 | 分支历史并集，不能当缺失功能计数；4 个原 ID 已随 #64/#70 进 main |
| open PRs | 16 | 14 | #64/#70 以 MERGED 闭合；其余保留，没有 discard closure |
| stash / worktrees | 0 / 1 | 0 / 1 | detached F worktree 已正常移除，原 KC1 clean |

原 B 的 **3463 个 commit IDs 全部保留**：3459 仍在未整合并集中，另 4 已进入 main/KC1。
7 个被删 ancestor refs 没有 unique work；原 unique heads #64/#70 先实际归并且有锚。
独有 SHA 集合按 ref 落 `unique-shas/*.txt`；图不同不证明语义未实现，patch/squash 判断仍需逐项 owner review。

| B ID | current ref | C primary class retained | unique outside main/KC1 | next disposition / exit |
| --- | --- | --- | ---: | --- |
| B002 | `refs/remotes/origin/archive/v1.0-master` | ARCHIVE_ONLY | 2101 | Owner must identify any wanted legacy deltas; independently verify full-history recovery before any later cleanup. |
| B003 | `refs/remotes/origin/backfill-and-ci-fixes-2026-10-24` | UNIQUE_PRESERVE | 5 | Review unmatched changes and present-day claim evidence; preserve original commits until explicit disposition. |
| B004 | `refs/remotes/origin/backup-main-20260901-pre-upstream` | ARCHIVE_ONLY | 2405 | Owner history decision plus separately verified tag/bundle; do not merge historical tree wholesale. |
| B006 | `refs/remotes/origin/backup/pre-2c2-upstream-rewrite` | ARCHIVE_ONLY | 14 | Verify recovery for exact tip and all unique objects; do not discard rewritten history based on naming. |
| B007 | `refs/remotes/origin/backup/pre-rc2-replay-994fd67a` | ARCHIVE_ONLY | 2 | Owner disposition and verified recovery before any later cleanup; compare with #31 without resetting current runtime. |
| B009 | `refs/remotes/origin/bio-switches-and-w5-2026-10-24` | NEEDS_OWNER_DECISION | 2 | Owner selects authoritative variant or records supersession after semantic comparison; preserve all three meanwhile. |
| B010 | `refs/remotes/origin/bio-switches-and-w5-v2` | NEEDS_OWNER_DECISION | 4 | Compare B009/B011 switches, defaults, docs and claims; no automatic newest-name winner. |
| B011 | `refs/remotes/origin/bio-switches-w5-v3` | NEEDS_OWNER_DECISION | 4 | Keep defaults and governance invariants; document owner-selected variant and remaining deltas. |
| B012 | `refs/remotes/origin/coverage/baseline-2026-10-24` | UNIQUE_PRESERVE | 1 | Owner reviews residual script logic versus current CI-calibrated baseline; do not restore obsolete measurement values. |
| B013 | `refs/remotes/origin/debt-round8` | NEEDS_OWNER_DECISION | 249 | Owner approves split/disposition with security, schema, retrieval and kernel-interface review; original 249 IDs preserved. |
| B014 | `refs/remotes/origin/dependabot/cargo/dependencies-092765a05c` | DEPENDABOT_OR_BOT | 2 | Refresh deliberately, audit resulting lock delta and run affected tests; no bot auto-delete. |
| B015 | `refs/remotes/origin/dependabot/cargo/http-aa340fe0a4` | DEPENDABOT_OR_BOT | 3 | Refresh on fresh branch or additive merge; validate dependency/security/MSRV changes without rewriting history. |
| B016 | `refs/remotes/origin/dependabot/cargo/serde-e3e0769812` | DEPENDABOT_OR_BOT | 1 | Keep pending core baseline; isolate intended version change and verify lockfile/tests. |
| B017 | `refs/remotes/origin/dependabot/cargo/tokio-7b53e3ccb2` | DEPENDABOT_OR_BOT | 1 | Review runtime/TLS and incidental resolver changes; rerun async cancellation and transport tests. |
| B018 | `refs/remotes/origin/dependabot/github_actions/actions/cache-6` | DEPENDABOT_OR_BOT | 2 | Refresh workflow changes; validate runner/action compatibility and required checks. |
| B019 | `refs/remotes/origin/dependabot/github_actions/actions/setup-python-7` | DEPENDABOT_OR_BOT | 1 | Wait for convergence baseline, then validate workflow compatibility and required-check execution. |
| B020 | `refs/remotes/origin/dependabot/github_actions/actions/upload-artifact-7` | DEPENDABOT_OR_BOT | 2 | Refresh and exercise artifact evidence path; do not infer obsolete noise from bot ownership. |
| B022 | `refs/remotes/origin/egress-four-claims-2026-10-23` | UNIQUE_PRESERVE | 1 | Owner may record patch-equivalent supersession after claim reconciliation; retain ID and recovery requirement. |
| B023 | `refs/remotes/origin/egress-restore-2026-10-23` | UNIQUE_PRESERVE | 20 | Review unmatched deltas and consequences/authorization wiring; never roll back newer claims or runtime assembly. |
| B024 | `refs/remotes/origin/feat/env-lock-crate-level` | MERGE_TO_MAIN_CANDIDATE | 62 | Owner scopes remaining work; verify cross-module serialization, no weakened assertions, security tests, clippy and all affected tests. |
| B029 | `refs/remotes/origin/feature/mesh-rc2-integration` | MERGE_TO_MAIN_CANDIDATE | 43 | Review only missing Mesh deltas against existing Mesh/Kernel ports; preserve pairing/trust, bounded failure and experimental real-NAT boundary; tests/CI before protected PR. |
| B030 | `refs/remotes/origin/feature/module-plugin-unification` | UNIQUE_PRESERVE | 14 | Compare #31, module ownership and current interfaces; owner records what remains valuable; no wholesale old-runtime merge. |
| B031 | `refs/remotes/origin/fix/bench-idle-footprint-isolation` | SUPERSEDED_BY_KC1_OR_MAIN | 1 | Owner explicitly records content supersession plus verified recovery before later deletion; graph ancestry is NOT claimed. |
| B032 | `refs/remotes/origin/fix/bench-report-command-2026-10-24` | UNIQUE_PRESERVE | 2 | Separate applied workflow from historical coverage calibration and obtain owner disposition. |
| B034 | `refs/remotes/origin/fix/elf-diff-locale-clean2` | NEEDS_OWNER_DECISION | 3 | Owner reviews residual fixture isolation and artifact disposition; preserve originals; no assertion weakening or bulk replay. |
| B036 | `refs/remotes/origin/fix/gate-nested-worktree` | SUPERSEDED_BY_KC1_OR_MAIN | 1 | Owner records exact-content supersession and verifies recovery anchor before Phase E deletion; not graph ancestry. |
| B038 | `refs/remotes/origin/fuzz/batch2-2026-10-24` | UNIQUE_PRESERVE | 3 | Compare current expanded fuzz matrix and assertions; retain branch until specific missing value or supersession is decided. |
| B040 | `refs/remotes/origin/open-source-difference-criteria-2026-10-23` | UNIQUE_PRESERVE | 2 | Reconcile generated claims and owner-recorded supersession; then verified recovery before any later cleanup. |
| B043 | `refs/remotes/origin/replay/phase2c2-base` | ARCHIVE_ONLY | 2 | Owner disposition and verified exact-tip/full-object recovery before any later cleanup. |
| B044 | `refs/remotes/origin/ui-engine-p0-2026-10-24` | KEEP_ACTIVE | 183 | Wait for KC1; review independent workspace/hooks/CI and readiness limits before any integration; no current merge/delete recommendation. |
| B045 | `refs/remotes/origin/unwired-guard-2026-10-24` | UNIQUE_PRESERVE | 2 | Compare current scanner/workflow and governance defaults; retain assertions and reconcile claims rather than replay old fingerprints. |

这些是 **KEEP/PRESERVE/review** 工作，不是删除名单。archive/backup、bio 多变体、旧 module/Mesh、coverage/egress/fuzz、
bot 和 UI 项目的历史价值须逐项决定；没有为压低计数批量并入主干或销毁历史。

## #98 与开放后续

快照 #98 **OPEN / MERGEABLE / BLOCKED**，未 merge；其批准 header 保留。
main **尚未包含 KC1**，F 的 `merge-base --is-ancestor` 返回 1；不能宣称批准/完成状态已经部署到 main。
后续须在当前 main 基线上做 fresh eligibility/缺失差异检查，修复真实 required failures；
若需刷新已发布工作，采用 owner-approved additive branch/PR 或普通追加 main merge，绝不 force-rebase。
当前 failed/pending details 在 `pr98.json`，新 G push 后 CI 是新的观测，旧红/绿结论不可自动继承。

开放工作：

- #98：required cargo deny/docs truth 失败与未完成/缺失 Kani/nextest，另有 secret/wiring/orphan-API 失败，具体以 fresh head 原始 rollup 为准；禁止将批准验收替代 required checks。
- #95/#94：范围膨胀，按 owner 选择聚焦 env-lock / 安全 / memory/接线增量；#31 Mesh 与 #78 UI 保留既有实验/ready 边界，不 wholesale merge。
- #19 fork guide-only 重建与 #34 桌面 default/titlebar 变更，先保留 fork head 再与作者协调 current-main PR；不虚构 fork unique 数。
- Dependabot：当前 main 上逐项串行解析共享 lock/工作流兼容与 advisory，不能用历史 #64 修复声称全安全。push server 的 advisory 通知原样归档，本批没有独立新扫描或清零声明。
- 最终整合后的 main 需要新 full gate；F 只实跑六项 docs/architecture，全部 exit0，明确未新跑 Rust/前端/设备/live/perf。A4 PASS 是 KC1 原软件范围证据，不代替新 main。
- 真实冷缓存与外部/硬件仍在既定范围内未实测；first_run 与相对+绝对回归建议继续保留。后续如删任何未决 ref，仍先 verified recovery + merge/明确 owner disposition。

## 交付验证与证据

执行日期 2026-10-29；Git/gh/Python/PowerShell 工具链沿本任务日志。主工作树始终 KC1，只有 F 的临时 detached main 只读工作树例外，
无本地 main 分支、产品源码/LOCKED/版本的新改写。main 的已有 topic 改动仅按 #64/#70 已授权 exact tips 通过 PR 纳入。

原始 evidence `/tmp/apeireth-d2/`、`/tmp/apeireth-e/`、`/tmp/apeireth-f/`、`/tmp/apeireth-g/`；
每阶段 `/workspace/apeireth-codex/<phase>-evidence/commit-<sha>/` 含 JSON、脚本、实际日志与 `.exit`、哈希 manifest。
`consolidation.json`、`unique-commit-union.txt`、`unique-shas/`、remote refs 广告、open PR/完整 #98 snapshot 可复算上述数字。
这些 metadata archive 本身不冒称 Git 恢复锚；真正锚是已推送且核验的 recovery tags。

G 收口运行 docs links/index、strict docs truth/generated、arch_guard --strict、范围/保留/graph/anchor 验证；
Codex commit.ps1 + ArchStrict，non-executable hook 体另经 Bash 实跑，每阶段 fetch 后 ordinary FF-only KC1 push / ls-remote。
STATE **next=done** 表示本指令的分阶段执行与报告交付已收口；上述开放 PR、full gate 与 owner review 仍是未完成后续。
