# Phase B 仓库 refs 与未整合历史清单

[Document-Meta]
Document: docs/04-internal/repo-consolidation-b-inventory-2026-10-29.md
Version: 1.0 (inventory only)
Last-Modified: 2026-10-29
Status: B inventory 完成；未合并、删分支、建锚或变更产品源码
Author: Codex

## 快照与计数口径

本批只盘点，不执行 C/D/E/F/G。起点 clean `cf59fd4904685530f9a773994c859d36bd2c103b`，分支始终为
`refactor/kc1-kernel-convergence`；原 `git fetch --prune origin` 成功。
Git 图快照 UTC **2026-10-29T01:11:43.769568+00:00 → 2026-10-29T01:11:58.038426+00:00**；GitHub API 查询时间另在 JSON，
这些只读查询不构成跨 Git/API 的原子事务快照。

- **BRANCHES=45**：本地 **1** + origin remote-tracking **44** 个真实分支 refs；
  同名 local/origin 两行分别计数，去重后的分支名为 **44**。
  `origin/HEAD` 是符号别名，单列但不计分支。没有本地 main 分支。
- **UNIQUE_UNMERGED=33**：至少一个 commit ID 不可从冻结 main 或 KC1 任一 tip 到达的**分支 ref 行数**，不是提交总数。
  这些行涉及 **31** 个不同 tip；全体分支 refs 的此类提交去重并集为 **3463**。
  同 tip/共享历史不重复加总；该并集不含仅 tag 可达的额外历史或未抓取的 fork PR head。
- topic refs 中 **9** 个 tip 已是 main 祖先；仅被 KC1 包含而尚未进 main 的非活动 topic refs 为 **0**。
  另有活动 KC1 两行和受保护 main 一行；两条 KC1 行本身不能当作待删除候选。
- 所有 **9** 个本地 tags 均列出并与远端广告核对；stash **0**；worktree **1**；当前 open PR **16**。

冻结比较基线：

| base | full SHA |
| --- | --- |
| origin/main | `4ea9c26095e22b83dc30faaf8f7148ce7cbdb82c` |
| KC1 local/origin | `cf59fd4904685530f9a773994c859d36bd2c103b` |

KC1 在该快照相对 main 为 **ahead=46 / behind=0**，尚未合入 main。
本报告的文档提交会使 KC1（及既有 PR #98 的 head）再推进一次；表格有意保留**提交前快照**，
不伪造能包含自身提交的 SHA。交付后的实际 HEAD/推送回执见外部 STATE 和 B-RESULT。

`main_ahead/main_behind` 分别为 `main..tip` 与 `tip..main` 提交数；KC1 同理。
`uniq_main` 等于 main_ahead，`uniq_both` 是 `git rev-list tip --not <main_sha> <kc1_sha>` 的行数。
所有计数基于完整非 shallow 图；未配置 replace refs。**图上的不同 commit ID 不等于补丁/业务内容缺失**：
本批没有检查历史 squash/cherry-pick 等价性，也没有据此决定合并、丢弃或优先级。
原分支名、作者和 subject 是 Git 历史元数据；表格只做 Markdown 转义，精确值以 JSON 为准。

## 全部分支：身份与最后一次提交

此表与下一表按 ID 连接，均覆盖全部分支 refs；日期是 tip 的 committer ISO 时间。
JSON 另保留 author date、author/committer email、parents 与 upstream 信息。

| ID | scope | ref | tip SHA | last commit date | author | subject |
| --- | --- | --- | --- | --- | --- | --- |
| B001 | local | `refs/heads/refactor/kc1-kernel-convergence` | `cf59fd4904685530f9a773994c859d36bd2c103b` | 2026-10-29T08:59:19+08:00 | Codex | docs(docs/04-internal): record full A4 gates and retain formal Partial acceptance |
| B002 | remote | `refs/remotes/origin/archive/v1.0-master` | `d63a233316567026d14d2f47f5a42cbf0640f047` | 2026-08-25T23:44:52+08:00 | Yinta | fix(sovereignty): colang_dsl test expected wrong layer_name |
| B003 | remote | `refs/remotes/origin/backfill-and-ci-fixes-2026-10-24` | `14636741faf1712e05d759d020c64e83bc3e1684` | 2026-10-24T10:36:00+08:00 | Mavis | docs(status): 指纹 bump 到 3b02269c（远端=head） |
| B004 | remote | `refs/remotes/origin/backup-main-20260901-pre-upstream` | `0acfbb10a976816581524f38c1b1fe07b516c3de` | 2026-08-31T13:31:39+08:00 | Yinta | chore(license): add MIT OR Apache-2.0 dual licensing and compliance documentation |
| B005 | remote | `refs/remotes/origin/backup/memory-v2.2-final-freeze-20260914` | `f1e8ee0e0b6b2802332dafd00023a7dd1f2a2fe6` | 2026-09-13T18:18:23+08:00 | Jimmyxiao2009 | docs(memory): finalize lifecycle acceptance evidence |
| B006 | remote | `refs/remotes/origin/backup/pre-2c2-upstream-rewrite` | `60331ded97e1fcf54b258c62b073848b8a004a95` | 2026-09-26T02:35:34+08:00 | Jimmyxiao2009 | test(mesh): verify provider and full turns over relay |
| B007 | remote | `refs/remotes/origin/backup/pre-rc2-replay-994fd67a` | `994fd67a033ee54e768542f12f864cd394921f02` | 2026-09-29T17:26:07+08:00 | Jimmyxiao2009 | feat(mesh): route remote tools through canonical runtime [A' ADAPTED] |
| B008 | remote | `refs/remotes/origin/batch-2026-10-22` | `b613f0ebb9da45b761f7e5d06a383ebf7a50cbce` | 2026-10-22T14:41:40+08:00 | Mavis | fix(ci): claims_sha256 改为 LF 归一后哈希（跨平台 wiring 生成物稳定） |
| B009 | remote | `refs/remotes/origin/bio-switches-and-w5-2026-10-24` | `7bb074f25e84ceff3f6d3b64727f816c03e04042` | 2026-10-24T11:46:49+08:00 | Mavis | docs(w5): 哲学拦截层**不接**热路径 —— 阻塞不在接线，在动作描述 |
| B010 | remote | `refs/remotes/origin/bio-switches-and-w5-v2` | `1190d490a36365cd59c2db49e073e9691e1effd9` | 2026-10-24T12:05:01+08:00 | Mavis | docs(index): 补两篇新文档的索引条目（docs_index --check 绿） |
| B011 | remote | `refs/remotes/origin/bio-switches-w5-v3` | `200ca38674c333759f1748ff465c8c1eb6b1f1a0` | 2026-10-24T12:40:34+08:00 | Mavis | docs(status): 指纹 bump（远端 head 对齐） |
| B012 | remote | `refs/remotes/origin/coverage/baseline-2026-10-24` | `29e9d8645a85e5791d38a6a412acd4874b4c5e61` | 2026-10-24T21:13:29+08:00 | Mavis | test(coverage): 覆盖率棘轮基线落定（34.38%）+ 口径统一到「可动的 workspace 代码」 |
| B013 | remote | `refs/remotes/origin/debt-round8` | `caf499b769cd1d3aa0a6281c77acc5e1ca5c6ae7` | 2026-10-28T21:54:11+08:00 | Mavis | chore(budget): crate 体量基线 **+47 行**（memory 77,827 → 77,874）—— **接受并说明理由** |
| B014 | remote | `refs/remotes/origin/dependabot/cargo/dependencies-092765a05c` | `37f281caca8c33d0642b8cc963111970816fafa7` | 2026-10-16T11:15:13+08:00 | Yinta | Merge branch 'main' into dependabot/cargo/dependencies-092765a05c |
| B015 | remote | `refs/remotes/origin/dependabot/cargo/http-aa340fe0a4` | `0c65ac7e7fc7029e8875da46c9f9dd6cf782c720` | 2026-10-16T12:35:30+08:00 | Yinta | Merge branch 'main' into dependabot/cargo/http-aa340fe0a4 |
| B016 | remote | `refs/remotes/origin/dependabot/cargo/serde-e3e0769812` | `a255c0bfef2ed6ab47b416e49dddd48459e0d577` | 2026-10-26T06:25:13Z | dependabot[bot] | deps(deps): bump serde_json |
| B017 | remote | `refs/remotes/origin/dependabot/cargo/tokio-7b53e3ccb2` | `dbc671259ac6b0926390af6e94d6359b516b85cb` | 2026-10-26T06:25:20Z | dependabot[bot] | deps(deps): bump the tokio group with 2 updates |
| B018 | remote | `refs/remotes/origin/dependabot/github_actions/actions/cache-6` | `31b8ab1a2ff33e46d481a3178c89c2529b3cbca6` | 2026-10-16T11:15:22+08:00 | Yinta | Merge branch 'main' into dependabot/github_actions/actions/cache-6 |
| B019 | remote | `refs/remotes/origin/dependabot/github_actions/actions/setup-python-7` | `926b6a866bffe58d2d9e51b37586e007754fceea` | 2026-10-26T06:24:53Z | dependabot[bot] | ci(deps): bump actions/setup-python from 5 to 7 |
| B020 | remote | `refs/remotes/origin/dependabot/github_actions/actions/upload-artifact-7` | `d974a1096f1b039f480a4615e1f0a61908dd4351` | 2026-10-16T11:15:20+08:00 | Yinta | Merge branch 'main' into dependabot/github_actions/actions/upload-artifact-7 |
| B021 | remote | `refs/remotes/origin/docs/governance-followup-2026-10-24` | `e9b0861aaadfef09629b4ca1533e8f3bce0a6b72` | 2026-10-24T10:28:49+08:00 | Mavis | fix(docs): 桩的行数与名称表述改准 —— 修复验证代理指出的 2 处「数字不可复算」 |
| B022 | remote | `refs/remotes/origin/egress-four-claims-2026-10-23` | `b6468b99ef92b88e0dd6cc2b28f8091d5816eee4` | 2026-10-23T18:11:44+08:00 | Mavis | docs(status): 出域四件套补 claim —— 「接电 + 补 claim」只做了一半的那一半 |
| B023 | remote | `refs/remotes/origin/egress-restore-2026-10-23` | `53af1001072db0bc719b1b97088b4df4927022c4` | 2026-10-24T09:45:18+08:00 | Mavis | docs(status): 指纹 bump 到 9c95b3ff —— SHA 对齐后 claim 流程彻底简化 |
| B024 | remote | `refs/remotes/origin/feat/env-lock-crate-level` | `71631d349ac72b140cce71710c1fdb584b4ca2e4` | 2026-10-28T21:06:29+08:00 | Mavis | docs(backlog): **D4 / D19 复核批** —— 债栏 10 条「状态不明」全部核完（本轮共 6 条） |
| B025 | remote | `refs/remotes/origin/feature/cognitive-infrastructure-vnext` | `f1e8ee0e0b6b2802332dafd00023a7dd1f2a2fe6` | 2026-09-13T18:18:23+08:00 | Jimmyxiao2009 | docs(memory): finalize lifecycle acceptance evidence |
| B026 | remote | `refs/remotes/origin/feature/cognitive-memory-convergence-v3` | `b50f9cd25638aa50cde0e65b64196501c89222b7` | 2026-09-14T12:55:47+08:00 | Jimmyxiao2009 | fix(cognitive): reconcile non-Windows keychain error types after convergence |
| B027 | remote | `refs/remotes/origin/feature/daily-batch-20261003` | `a612f6f1d4aa90fa237283e0f299e24cbbf4f4f3` | 2026-10-14T20:27:07+08:00 | Yinta | docs: 手册终表刷新——K18 名单与 2026-10-13 终局实测全录 |
| B028 | remote | `refs/remotes/origin/feature/memory-v2.2-production-completion` | `f1e8ee0e0b6b2802332dafd00023a7dd1f2a2fe6` | 2026-09-13T18:18:23+08:00 | Jimmyxiao2009 | docs(memory): finalize lifecycle acceptance evidence |
| B029 | remote | `refs/remotes/origin/feature/mesh-rc2-integration` | `b43ff3fd7316641748adb13af3071afc4ab6dc89` | 2026-10-13T01:42:41+08:00 | Jimmyxiao2009 | build: refresh Cargo.lock for the merged workspace |
| B030 | remote | `refs/remotes/origin/feature/module-plugin-unification` | `60331ded97e1fcf54b258c62b073848b8a004a95` | 2026-09-26T02:35:34+08:00 | Jimmyxiao2009 | test(mesh): verify provider and full turns over relay |
| B031 | remote | `refs/remotes/origin/fix/bench-idle-footprint-isolation` | `4669315493b1cd5920aa9debd26794e21ddefaa1` | 2026-10-24T21:52:08+08:00 | Mavis | fix(bench): 待机内存基准改在独立子进程里测 —— 原口径测的是「基准跑完后的高水位」 |
| B032 | remote | `refs/remotes/origin/fix/bench-report-command-2026-10-24` | `fb4d9bbadb9ba61c81fe01e6116cd2f1852ee250` | 2026-10-24T21:18:56+08:00 | Mavis | fix(ci): 修 bench-report workflow 的跑法 —— 原来的命令产不出任何数据 |
| B033 | remote | `refs/remotes/origin/fix/ci-cargo-audit-yaml-2026-10-24` | `50576a88ec077ce3b86a95e91460b61dab85d91f` | 2026-10-24T12:02:29+08:00 | Xephylos | fix(ci): SARIF 转换步补 if: always()，修审计失败时的证据链断裂 |
| B034 | remote | `refs/remotes/origin/fix/elf-diff-locale-clean2` | `cc0f3961f88337918aa2719dcaafb7ae7550f851` | 2026-10-24T21:54:29+08:00 | Mavis | style(tools): 按 rustfmt 重排 doc_ops 测试里的链式调用 |
| B035 | remote | `refs/remotes/origin/fix/external-tool-etxtbsy-flake-2026-10-24` | `bb51f380b3ce29b27b72ec6f07749b843c72cca5` | 2026-10-24T13:17:35+08:00 | Xephylos | fix(test): 给外部工具 unix 用例加 ETXTBSY 退避重试，消掉 O-6 锚的随机红灯 |
| B036 | remote | `refs/remotes/origin/fix/gate-nested-worktree` | `c715f668fb2494d0307a310b369bcb8487b36e88` | 2026-10-24T21:24:45+08:00 | Mavis | fix(docs): 三个文档门禁修「嵌套 worktree 污染」—— 副本文档被当成本仓文档重复扫描 |
| B037 | remote | `refs/remotes/origin/fix/historical-debt` | `196eebff2aaedbb76d534637f6aca4d575402944` | 2026-10-15T21:59:01+08:00 | Yinta | Merge remote-tracking branch 'origin/main' into fix/historical-debt |
| B038 | remote | `refs/remotes/origin/fuzz/batch2-2026-10-24` | `a02ba4d824900fbbf3dd467387cc5fe5256c3e46` | 2026-10-24T21:58:23+08:00 | Mavis | fix(ci): fuzz workflow 必须 &#96;cargo +nightly fuzz&#96; —— 仓根 rust-toolchain.toml 压过了 nightly |
| B039 | remote | `refs/remotes/origin/main` | `4ea9c26095e22b83dc30faaf8f7148ce7cbdb82c` | 2026-10-25T08:24:31+08:00 | Yinta | test(fuzz+ci+docs): 门禁与测量加固整合批（28 commit）—— 修 7 个安全缺陷 + 22 target 入 CI |
| B040 | remote | `refs/remotes/origin/open-source-difference-criteria-2026-10-23` | `32a78b1ac38e4ab830d100c5b77a0fe83577387c` | 2026-10-23T18:12:44+08:00 | Mavis | docs(privacy): 新建「数据与隐私」对外页 —— 承诺分级的落点 |
| B041 | remote | `refs/remotes/origin/rc/2.0.0-rc1-final` | `3dff564d083d57ee9c2fa40c33affbce0a4a3177` | 2026-08-31T21:12:28+08:00 | Jimmyxiao2009 | feat(desktop): add model provider settings, call logging, branch/retry/edit interactions, and packaging automation |
| B042 | remote | `refs/remotes/origin/refactor/kc1-kernel-convergence` | `cf59fd4904685530f9a773994c859d36bd2c103b` | 2026-10-29T08:59:19+08:00 | Codex | docs(docs/04-internal): record full A4 gates and retain formal Partial acceptance |
| B043 | remote | `refs/remotes/origin/replay/phase2c2-base` | `994fd67a033ee54e768542f12f864cd394921f02` | 2026-09-29T17:26:07+08:00 | Jimmyxiao2009 | feat(mesh): route remote tools through canonical runtime [A' ADAPTED] |
| B044 | remote | `refs/remotes/origin/ui-engine-p0-2026-10-24` | `5cfbccb78c7ff2310ae042db8984bc5f9460e027` | 2026-10-28T22:17:15+08:00 | Mavis | docs(internal): ⚖️ L1 最终放行判定 —— 上屏可开工；「引擎可上线」不得写进排期（C1/C2/C3） |
| B045 | remote | `refs/remotes/origin/unwired-guard-2026-10-24` | `120d96e773854134ca8a7a4ae3b0fb292fe77ace` | 2026-10-24T14:50:36+08:00 | Mavis | docs(status): 问责闭环五步全部接线的如实记账 + 指纹 bump |

## 全部分支：图关系、PR 与保留判定

M/K 的 Y/N 表示 tip 是否为 main/KC1 的祖先。分类是盘点标签，**没有任何 B 删除授权**：

- `BASE_MAIN_KEEP` / `ACTIVE_KC1_KEEP`：保留集成基线与当前工作线。
- `ANCESTOR_OF_MAIN_CANDIDATE`：图证据已进 main；后续仍需 owner/PR/保护条件与恢复锚核验，不能现在删除。
- `CONTAINED_IN_KC1_NOT_MAIN`：候选由 KC1 图包含；不能等同 main 已整合，亦不能代替语义审查；本快照非活动 topic 为零。
- `UNIQUE_UNINTEGRATED_KEEP`：有 main/KC1 外 commit IDs；**禁止在缺少合并或明确去留决定及恢复锚时删除**。
- `backup/archive` 或 `dependabot` 是附加命名/自动化标记，不覆盖上述图关系。PR 栏仅查询当前 open PR，
  `NO_OPEN_PR_OBSERVED` 不表示从未有 PR、已合并或可删除；head=MATCH 才表示 API 的 PR head 与该行 tip 相同。

| ID | main_ahead | main_behind | kc1_ahead | kc1_behind | M | K | uniq_main | uniq_both | category | open PRs | name flag | exact-tip tag |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| B001 | 46 | 0 | 0 | 0 | N | Y | 46 | 0 | ACTIVE_KC1_KEEP | [98](https://github.com/Apeireth/Apeireth/pull/98) OPEN; head=MATCH | — | — |
| B002 | 2101 | 3414 | 2101 | 3460 | N | N | 2101 | 2101 | UNIQUE_UNINTEGRATED_KEEP | NO_OPEN_PR_OBSERVED | archive | — |
| B003 | 5 | 89 | 5 | 135 | N | N | 5 | 5 | UNIQUE_UNINTEGRATED_KEEP | NO_OPEN_PR_OBSERVED | — | — |
| B004 | 2405 | 3414 | 2405 | 3460 | N | N | 2405 | 2405 | UNIQUE_UNINTEGRATED_KEEP | NO_OPEN_PR_OBSERVED | backup | v2.0.0-preview |
| B005 | 0 | 867 | 0 | 913 | Y | Y | 0 | 0 | ANCESTOR_OF_MAIN_CANDIDATE | NO_OPEN_PR_OBSERVED | backup | — |
| B006 | 14 | 637 | 14 | 683 | N | N | 14 | 14 | UNIQUE_UNINTEGRATED_KEEP | NO_OPEN_PR_OBSERVED | backup | — |
| B007 | 2 | 585 | 2 | 631 | N | N | 2 | 2 | UNIQUE_UNINTEGRATED_KEEP | NO_OPEN_PR_OBSERVED | backup | — |
| B008 | 0 | 217 | 0 | 263 | Y | Y | 0 | 0 | ANCESTOR_OF_MAIN_CANDIDATE | NO_OPEN_PR_OBSERVED | — | — |
| B009 | 2 | 57 | 2 | 103 | N | N | 2 | 2 | UNIQUE_UNINTEGRATED_KEEP | NO_OPEN_PR_OBSERVED | — | — |
| B010 | 4 | 55 | 4 | 101 | N | N | 4 | 4 | UNIQUE_UNINTEGRATED_KEEP | NO_OPEN_PR_OBSERVED | — | — |
| B011 | 4 | 53 | 4 | 99 | N | N | 4 | 4 | UNIQUE_UNINTEGRATED_KEEP | NO_OPEN_PR_OBSERVED | — | — |
| B012 | 1 | 8 | 1 | 54 | N | N | 1 | 1 | UNIQUE_UNINTEGRATED_KEEP | NO_OPEN_PR_OBSERVED | — | — |
| B013 | 249 | 0 | 249 | 46 | N | N | 249 | 249 | UNIQUE_UNINTEGRATED_KEEP | [94](https://github.com/Apeireth/Apeireth/pull/94) OPEN; head=MATCH | — | — |
| B014 | 2 | 490 | 2 | 536 | N | N | 2 | 2 | UNIQUE_UNINTEGRATED_KEEP | [29](https://github.com/Apeireth/Apeireth/pull/29) OPEN; head=MATCH | dependabot | — |
| B015 | 3 | 489 | 3 | 535 | N | N | 3 | 3 | UNIQUE_UNINTEGRATED_KEEP | [22](https://github.com/Apeireth/Apeireth/pull/22) OPEN; head=MATCH | dependabot | — |
| B016 | 1 | 0 | 1 | 46 | N | N | 1 | 1 | UNIQUE_UNINTEGRATED_KEEP | [33](https://github.com/Apeireth/Apeireth/pull/33) OPEN; head=MATCH | dependabot | — |
| B017 | 1 | 0 | 1 | 46 | N | N | 1 | 1 | UNIQUE_UNINTEGRATED_KEEP | [97](https://github.com/Apeireth/Apeireth/pull/97) OPEN; head=MATCH | dependabot | — |
| B018 | 2 | 490 | 2 | 536 | N | N | 2 | 2 | UNIQUE_UNINTEGRATED_KEEP | [17](https://github.com/Apeireth/Apeireth/pull/17) OPEN; head=MATCH | dependabot | — |
| B019 | 1 | 0 | 1 | 46 | N | N | 1 | 1 | UNIQUE_UNINTEGRATED_KEEP | [96](https://github.com/Apeireth/Apeireth/pull/96) OPEN; head=MATCH | dependabot | — |
| B020 | 2 | 490 | 2 | 536 | N | N | 2 | 2 | UNIQUE_UNINTEGRATED_KEEP | [20](https://github.com/Apeireth/Apeireth/pull/20) OPEN; head=MATCH | dependabot | — |
| B021 | 0 | 70 | 0 | 116 | Y | Y | 0 | 0 | ANCESTOR_OF_MAIN_CANDIDATE | NO_OPEN_PR_OBSERVED | — | — |
| B022 | 1 | 123 | 1 | 169 | N | N | 1 | 1 | UNIQUE_UNINTEGRATED_KEEP | NO_OPEN_PR_OBSERVED | — | — |
| B023 | 20 | 114 | 20 | 160 | N | N | 20 | 20 | UNIQUE_UNINTEGRATED_KEEP | NO_OPEN_PR_OBSERVED | — | — |
| B024 | 62 | 0 | 62 | 46 | N | N | 62 | 62 | UNIQUE_UNINTEGRATED_KEEP | [95](https://github.com/Apeireth/Apeireth/pull/95) OPEN; head=MATCH | — | — |
| B025 | 0 | 867 | 0 | 913 | Y | Y | 0 | 0 | ANCESTOR_OF_MAIN_CANDIDATE | NO_OPEN_PR_OBSERVED | — | — |
| B026 | 0 | 777 | 0 | 823 | Y | Y | 0 | 0 | ANCESTOR_OF_MAIN_CANDIDATE | NO_OPEN_PR_OBSERVED | — | — |
| B027 | 0 | 515 | 0 | 561 | Y | Y | 0 | 0 | ANCESTOR_OF_MAIN_CANDIDATE | NO_OPEN_PR_OBSERVED | — | — |
| B028 | 0 | 867 | 0 | 913 | Y | Y | 0 | 0 | ANCESTOR_OF_MAIN_CANDIDATE | NO_OPEN_PR_OBSERVED | — | — |
| B029 | 43 | 524 | 43 | 570 | N | N | 43 | 43 | UNIQUE_UNINTEGRATED_KEEP | [31](https://github.com/Apeireth/Apeireth/pull/31) OPEN; head=MATCH | — | — |
| B030 | 14 | 637 | 14 | 683 | N | N | 14 | 14 | UNIQUE_UNINTEGRATED_KEEP | NO_OPEN_PR_OBSERVED | — | — |
| B031 | 1 | 4 | 1 | 50 | N | N | 1 | 1 | UNIQUE_UNINTEGRATED_KEEP | NO_OPEN_PR_OBSERVED | — | — |
| B032 | 2 | 8 | 2 | 54 | N | N | 2 | 2 | UNIQUE_UNINTEGRATED_KEEP | NO_OPEN_PR_OBSERVED | — | — |
| B033 | 3 | 63 | 3 | 109 | N | N | 3 | 3 | UNIQUE_UNINTEGRATED_KEEP | [64](https://github.com/Apeireth/Apeireth/pull/64) OPEN; head=MATCH | — | — |
| B034 | 3 | 8 | 3 | 54 | N | N | 3 | 3 | UNIQUE_UNINTEGRATED_KEEP | NO_OPEN_PR_OBSERVED | — | — |
| B035 | 1 | 51 | 1 | 97 | N | N | 1 | 1 | UNIQUE_UNINTEGRATED_KEEP | [70](https://github.com/Apeireth/Apeireth/pull/70) OPEN; head=MATCH | — | — |
| B036 | 1 | 4 | 1 | 50 | N | N | 1 | 1 | UNIQUE_UNINTEGRATED_KEEP | NO_OPEN_PR_OBSERVED | — | — |
| B037 | 0 | 495 | 0 | 541 | Y | Y | 0 | 0 | ANCESTOR_OF_MAIN_CANDIDATE | NO_OPEN_PR_OBSERVED | — | — |
| B038 | 3 | 26 | 3 | 72 | N | N | 3 | 3 | UNIQUE_UNINTEGRATED_KEEP | NO_OPEN_PR_OBSERVED | — | — |
| B039 | 0 | 0 | 0 | 46 | Y | Y | 0 | 0 | BASE_MAIN_KEEP | NO_OPEN_PR_OBSERVED | — | — |
| B040 | 2 | 123 | 2 | 169 | N | N | 2 | 2 | UNIQUE_UNINTEGRATED_KEEP | NO_OPEN_PR_OBSERVED | — | — |
| B041 | 0 | 976 | 0 | 1022 | Y | Y | 0 | 0 | ANCESTOR_OF_MAIN_CANDIDATE | NO_OPEN_PR_OBSERVED | — | v2.0.0-rc.1 |
| B042 | 46 | 0 | 0 | 0 | N | Y | 46 | 0 | ACTIVE_KC1_KEEP | [98](https://github.com/Apeireth/Apeireth/pull/98) OPEN; head=MATCH | — | — |
| B043 | 2 | 585 | 2 | 631 | N | N | 2 | 2 | UNIQUE_UNINTEGRATED_KEEP | NO_OPEN_PR_OBSERVED | — | — |
| B044 | 183 | 0 | 183 | 46 | N | N | 183 | 183 | UNIQUE_UNINTEGRATED_KEEP | [78](https://github.com/Apeireth/Apeireth/pull/78) OPEN; head=MATCH | — | — |
| B045 | 2 | 47 | 2 | 93 | N | N | 2 | 2 | UNIQUE_UNINTEGRATED_KEEP | NO_OPEN_PR_OBSERVED | — | — |

## 大块未整合主题与命名信号

下表只按名字及 tip 元数据归组，不开展 Phase C 代码/补丁评审；各行去重并集可能重叠，不能相加为全仓总数。

| theme | ref rows | uniq_both>0 rows | commit union outside both | largest member IDs | rough signal |
| --- | --- | --- | --- | --- | --- |
| archive | 1 | 1 | 2101 | B002 | 旧版本历史存档；不可因 archive 命名抹去独立历史 |
| backup | 4 | 3 | 2421 | B004, B006, B007 | 历史冻结/重写前快照；不是自动可删的备份噪声 |
| bio | 3 | 3 | 10 | B010, B011, B009 | 生物开关/W5 三条迭代线，图上各有独立提交 |
| coverage | 1 | 1 | 1 | B012 | 覆盖率基线 |
| dependabot | 7 | 7 | 12 | B015, B014, B018 | 依赖/工作流自动更新；7 行均有图上独立提交，不能只按 bot 名字删除 |
| docs | 1 | 0 | 0 | B021 | 文档治理后续线 |
| egress | 2 | 2 | 21 | B023, B022 | 出口恢复/声明对账 |
| feature | 7 | 3 | 119 | B024, B029, B030 | 认知/记忆/模块/组网与 crate 级环境锁；名称不证明已被 KC1 替代 |
| fix | 7 | 6 | 11 | B033, B034, B032 | 基准、CI、locale、ETXTBSY、nested worktree、历史债修复 |
| fuzz | 1 | 1 | 3 | B038 | fuzz 批次 |
| kc1 | 2 | 0 | 0 | B001, B042 | 活动内核收敛线及本地镜像 |
| main | 1 | 0 | 0 | B039 | 远端集成基线 |
| other | 5 | 4 | 258 | B013, B003, B040 | debt-round8、backfill、batch、差异标准、unwired guard |
| release-replay | 2 | 1 | 2 | B043, B041 | RC 冻结与 replay 基点 |
| ui | 1 | 1 | 183 | B044 | UI engine 独立主题 |

显著分叉包括 B004（历史 backup，uniq_both=2405）、B002（v1 archive，2101）、
B013（debt-round8，249）、B044（UI，183）、B024（crate-level env lock，62）、B029（Mesh，43）。
这些只表示图上的历史规模，不是本批决定应直接合并的提交数；大块旧历史可能包含已重写/替代内容，须留给 C 审查。
同 tip aliases 单列如下；有别名也不能据此免除 Phase E 的锚要求：

| tip SHA | ref IDs |
| --- | --- |
| `cf59fd4904685530f9a773994c859d36bd2c103b` | B001, B042 |
| `f1e8ee0e0b6b2802332dafd00023a7dd1f2a2fe6` | B005, B025, B028 |
| `60331ded97e1fcf54b258c62b073848b8a004a95` | B006, B030 |
| `994fd67a033ee54e768542f12f864cd394921f02` | B007, B043 |

## 当前 open PR 全集与 API 边界

使用已有 gh 登录执行只读 REST `--paginate`，未创建/关闭/改动任何 PR、评论或保护规则。
仓库 `main` 的 API 返回 **protected=true**，tip 与冻结 origin/main 一致；未进一步审计保护规则细项。
以后 main 更新仍只能走用户要求的受保护 GitHub PR 流程（D），B 不合并。
PR 的 `base_sha` 保留在 JSON，但图比较使用冻结 origin/main，不能把 PR 记录里的 base SHA 当成当前 main。

| PR | state/draft | head repository / ref | head SHA | base | origin head observed | title |
| --- | --- | --- | --- | --- | --- | --- |
| [98](https://github.com/Apeireth/Apeireth/pull/98) | open/false | Apeireth/Apeireth / refactor/kc1-kernel-convergence | `cf59fd4904685530f9a773994c859d36bd2c103b` | main | Y | refactor(kernel): KC-1 mechanism-only microkernel convergence |
| [97](https://github.com/Apeireth/Apeireth/pull/97) | open/false | Apeireth/Apeireth / dependabot/cargo/tokio-7b53e3ccb2 | `dbc671259ac6b0926390af6e94d6359b516b85cb` | main | Y | deps(deps): bump the tokio group with 2 updates |
| [96](https://github.com/Apeireth/Apeireth/pull/96) | open/false | Apeireth/Apeireth / dependabot/github_actions/actions/setup-python-7 | `926b6a866bffe58d2d9e51b37586e007754fceea` | main | Y | ci(deps): bump actions/setup-python from 5 to 7 |
| [95](https://github.com/Apeireth/Apeireth/pull/95) | open/false | Apeireth/Apeireth / feat/env-lock-crate-level | `71631d349ac72b140cce71710c1fdb584b4ca2e4` | main | Y | fix(tools): env 竞态根治 —— crate 级共享锁 + 断言去重 + 守卫测试 |
| [94](https://github.com/Apeireth/Apeireth/pull/94) | open/false | Apeireth/Apeireth / debt-round8 | `caf499b769cd1d3aa0a6281c77acc5e1ca5c6ae7` | main | Y | feat(debt): 28 commit —— A 类缺口接线 9/16 + 5 条自行发现的缺陷 + 3 处文档对账（含并行派发/守卫落盘/限流） |
| [78](https://github.com/Apeireth/Apeireth/pull/78) | open/false | Apeireth/Apeireth / ui-engine-p0-2026-10-24 | `5cfbccb78c7ff2310ae042db8984bc5f9460e027` | main | Y | feat(ui-engine): 全栈纯 Rust 前端引擎 —— 7 crate 独立 workspace + 试点像素 0.0% + 生产对照 0/576000 |
| [70](https://github.com/Apeireth/Apeireth/pull/70) | open/false | Apeireth/Apeireth / fix/external-tool-etxtbsy-flake-2026-10-24 | `bb51f380b3ce29b27b72ec6f07749b843c72cca5` | main | Y | fix(test): 给外部工具 unix 用例加 ETXTBSY 退避重试，消掉 O-6 锚的随机红灯 |
| [64](https://github.com/Apeireth/Apeireth/pull/64) | open/false | Apeireth/Apeireth / fix/ci-cargo-audit-yaml-2026-10-24 | `50576a88ec077ce3b86a95e91460b61dab85d91f` | main | Y | fix(ci): 修 cargo-audit workflow 的 0 job + 清两处 lock 的 11 条 RUSTSEC 漏洞面 |
| [34](https://github.com/Apeireth/Apeireth/pull/34) | open/false | Xephylos/Apeireth / feat/companion-desktop-pet-titlebar | `50d9684d04e9b6e0cca97520414d76a9e2b97258` | main | N | feat(companion-desktop): 默认关闭桌宠自动接管 + 固定主窗标题栏配色 |
| [33](https://github.com/Apeireth/Apeireth/pull/33) | open/false | Apeireth/Apeireth / dependabot/cargo/serde-e3e0769812 | `a255c0bfef2ed6ab47b416e49dddd48459e0d577` | main | Y | deps(deps): bump serde_json from 1.0.151 to 1.0.154 in the serde group across 1 directory |
| [31](https://github.com/Apeireth/Apeireth/pull/31) | open/false | Apeireth/Apeireth / feature/mesh-rc2-integration | `b43ff3fd7316641748adb13af3071afc4ab6dc89` | main | Y | Mesh RC2：NAT 穿透生产路径集成 + daily-batch 归并（基于 main 最新，fast-forward 无冲突） |
| [29](https://github.com/Apeireth/Apeireth/pull/29) | open/false | Apeireth/Apeireth / dependabot/cargo/dependencies-092765a05c | `37f281caca8c33d0642b8cc963111970816fafa7` | main | Y | deps(deps): bump the dependencies group across 1 directory with 4 updates |
| [22](https://github.com/Apeireth/Apeireth/pull/22) | open/false | Apeireth/Apeireth / dependabot/cargo/http-aa340fe0a4 | `0c65ac7e7fc7029e8875da46c9f9dd6cf782c720` | main | Y | deps(deps): bump hyper-util from 0.1.20 to 0.1.21 in the http group across 1 directory |
| [20](https://github.com/Apeireth/Apeireth/pull/20) | open/false | Apeireth/Apeireth / dependabot/github_actions/actions/upload-artifact-7 | `d974a1096f1b039f480a4615e1f0a61908dd4351` | main | Y | ci(deps): bump actions/upload-artifact from 4 to 7 |
| [19](https://github.com/Apeireth/Apeireth/pull/19) | open/false | Dazzanova/Apeireth / add-mcp-guide | `3aea2ccc264c63493d72749eb1e1b6976e2e7767` | main | N | docs: add MCP tool integration guide |
| [17](https://github.com/Apeireth/Apeireth/pull/17) | open/false | Apeireth/Apeireth / dependabot/github_actions/actions/cache-6 | `31b8ab1a2ff33e46d481a3178c89c2529b3cbca6` | main | Y | ci(deps): bump actions/cache from 4 to 6 |

#34 与 #19 的 head 在外部 fork，不在本仓 origin 的 44 个分支内；本批只记录其仓库、SHA 与 open 状态，
未抓取 fork、未计算其 unique 数，也不把它们遗漏地当成已整合。这两项不计入 BRANCHES/UNIQUE_UNMERGED。
API 在只读查询时可见 16 项，未调查 closed/merged PR 历史；没有把 open 列表当作 merge-status 数据库。
初次 gh 查询使用了不兼容的输出选项组合，保留 initial 日志；更正为分页逐页解析后成功，不是认证失败。
managed environment_status 工具和网络策略文件本环境不可用；实际 fetch / ls-remote / gh auth / gh API 均成功，
不把缺失策略快照冒称已验证 credential readiness。没有读取凭据内容或交互登录。

## 全部 tags（对象、提交与历史边界）

`tag` 表示 annotated tag 对象，`commit` 表示 lightweight tag；tip 比较使用 peeled commit。
所有 tag 对象与远端广告一致（remote_match=Y）。tagger/date/原 annotation subject、对象 SHA 和 peeled SHA 均在 JSON。
Tags 不能作为 GitHub PR head，故 open PR 状态为 N/A；没有验证签名、不可变保护或恢复演练。

| tag | object type | object SHA | target commit SHA | commit date / author | commit subject | main A/B | kc1 A/B | M/K | remote_match |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `archive/2026-10-15-rc4` | tag | `52742875215d0f7c47a246b92a17cc63bd725d31` | `91b2b6148c4d9e07770e9a053a21d10135c8d4aa` | 2026-10-15T22:04:15+08:00 / Yinta | docs: 存档点——rc.4 战役文档全量更新 | 0/504 | 0/550 | Y/Y | Y |
| `v1.0.0` | tag | `750e1b418c8c21874eca0d283c0930563c9aebab` | `84b2d7796bc410f7710cbb85ee6874ad3d219c89` | 2026-08-18T01:36:25+08:00 / Yinta | docs(1.0): RELEASE_NOTES 正式版重写 (主人拍板真正的 1.0, 五原型/发布前置/诚实标注) + README 产品轴 v1.0.0 | 1631/3414 | 1631/3460 | N/N | Y |
| `v1.5.0` | tag | `372806b05b33d5d39f85b2e88c5947c066722dc5` | `e700a1c4f6636217fceafbd042bea68ca63b0400` | 2026-08-23T13:35:20+08:00 / Yinta | feat(reconstruction_v2): deliver 10-crate complete architecture, docs and verification suite | 1992/3414 | 1992/3460 | N/N | Y |
| `v2.0.0-alpha.1` | tag | `f2679741c79b209b28b5215419bf85db20e2d5ca` | `c43c65d3999d45c8faa77059030b0839695ca8e3` | 2026-08-27T11:32:05+08:00 / Yinta | test(tools): give Windows unicode shell smoke a cold-start-safe timeout | 2121/3414 | 2121/3460 | N/N | Y |
| `v2.0.0-preview` | tag | `c4e9d19bb78f97f6795b9420fbfef8a45bd7d45b` | `0acfbb10a976816581524f38c1b1fe07b516c3de` | 2026-08-31T13:31:39+08:00 / Yinta | chore(license): add MIT OR Apache-2.0 dual licensing and compliance documentation | 2405/3414 | 2405/3460 | N/N | Y |
| `v2.0.0-rc.1` | commit | `3dff564d083d57ee9c2fa40c33affbce0a4a3177` | `3dff564d083d57ee9c2fa40c33affbce0a4a3177` | 2026-08-31T21:12:28+08:00 / Jimmyxiao2009 | feat(desktop): add model provider settings, call logging, branch/retry/edit interactions, and packaging automation | 0/976 | 0/1022 | Y/Y | Y |
| `v2.0.0-rc.2` | commit | `61fc7e022db895c8086db0cd9054f11f2beb7c03` | `61fc7e022db895c8086db0cd9054f11f2beb7c03` | 2026-09-29T13:54:58+08:00 / Yinta | docs: 口径注记——18 产品 crate + 1 bench 包不计（cargo metadata 计 19）随文自带解释 | 0/566 | 0/612 | Y/Y | Y |
| `v2.0.0-rc.3` | commit | `e11bf40ba4711c1ee25401053107b6304309e2db` | `e11bf40ba4711c1ee25401053107b6304309e2db` | 2026-09-29T15:56:48+08:00 / Yinta | docs(beta): rc.2 内测指南——安装/上手/反馈模板/已知问题诚实清单 | 0/563 | 0/609 | Y/Y | Y |
| `v2.0.0-rc.4` | commit | `2a8b46fe1175568197c5e57e69bec8e8096c4f65` | `2a8b46fe1175568197c5e57e69bec8e8096c4f65` | 2026-10-15T21:02:27+08:00 / Yinta | Merge remote-tracking branch 'origin/main' | 0/506 | 0/552 | Y/Y | Y |

特别注意：v1.0.0、v1.5.0、alpha.1、preview 的目标不是 main 或 KC1 祖先，保留其独立历史。
B004 恰与 `v2.0.0-preview` 同 tip，仅是现有 tag 线索，不是 B 已完成的恢复锚验证。
`archive/v1.0-master`、重写前模块备份和 replay 基点也不能因旧命名或另一个 alias 就删除。

## Stash、worktree 与符号别名

| stash count | observation |
| --- | --- |
| 0 | git stash list 为空；无 pop/drop/clear |

| path | HEAD | branch | status | locked / prunable |
| --- | --- | --- | --- | --- |
| `/workspace/Apeireth` | `cf59fd4904685530f9a773994c859d36bd2c103b` | `refs/heads/refactor/kc1-kernel-convergence` | CLEAN | N / N |

| symbolic ref | target | resolved SHA |
| --- | --- | --- |
| `refs/remotes/origin/HEAD` | `refs/remotes/origin/main` | `4ea9c26095e22b83dc30faaf8f7148ce7cbdb82c` |

## 后续删除前的恢复要求（只记录，不执行）

**本表任何行都没有删除授权。** Phase E 若将来要删除任何 branch，先有可复核 tag 或完整 bundle，
记录其覆盖 tip/独有提交、对象可达性、远端/离线保存位置及恢复核验结果，再按授权执行。
33 个 unique-unintegrated refs 必须先保留所有独有工作，经过合并或明确去留决定；
9 个 main-ancestor topic 候选也不能跳过恢复锚、owner/PR/worktree 复核。
仅由 KC1 包含的历史提交不能直接当作已进 main；KC1 自身尚有 46 个 main 外提交，要保持活动线与受保护 PR #98。

本批没有创建 tag/bundle；现有 tags、JSON、SHA 文本、日志或 alias **不等于已验证的恢复锚**。
实际快照 refs、PRs、tags、stash/worktree 和完整 unique SHA 清单均保存，后续可以重新验证，
不能仅凭当前名字或 old branch 数量做批量删除。

## 可复核证据与本批校验

原始及生成证据：`/tmp/apeireth-b/`，收口归档 `/workspace/apeireth-codex/b-evidence/`。
核心文件为 `inventory.py`、`render_inventory.py`、`inventory.json`、`git-calls.jsonl`、
`branch-refs-before.txt`、`unique-commit-union.txt`、`unintegrated-shas/*.txt`、`open-prs.json`、
`github-read-receipts.json`、`main-metadata.json` 与远端 refs 广告；b-*.log 均有同名真实 `.exit`。
Git 命令与日期见日志；Git/gh/Python 工具版本另记 `b-toolchain.log`。逐行复核使用冻结 SHA。
完整生成器要求 clean `cf59fd49` 输入，应在相应基线副本复跑；不能在已推进的 KC1 HEAD 上假装重建原快照：

```bash
git fetch --prune origin
git for-each-ref --format='%(refname) %(objectname) %(symref)' refs/heads refs/remotes
git rev-list --left-right --count <frozen_main_sha>...<tip_sha>
git rev-list --left-right --count <frozen_kc1_sha>...<tip_sha>
git merge-base --is-ancestor <tip_sha> <frozen_main_sha>
git merge-base --is-ancestor <tip_sha> <frozen_kc1_sha>
git rev-list <tip_sha> --not <frozen_main_sha> <frozen_kc1_sha>
git ls-remote --heads --tags origin
python /tmp/apeireth-b/inventory.py
python /tmp/apeireth-b/render_inventory.py
```

收口运行 docs_links / docs_index / docs_truth --verify --strict 与 --check，以及 arch_guard --strict；
实际日志/退出码随 B-RESULT 归档。纯盘点文档不改 Rust，不重跑 fmt 或 full gate；A4 软件验证只作为既有独立证据，
[Final Acceptance](kernel-convergence-final-acceptance-2026-10-28.md) 的正式 Partial、未批准的新启动定义和真实冷缓存缺口均保持。
提交仅含本笔记、INDEX 与 CHANGELOG，经 Codex commit.ps1 / ArchStrict，fetch 后普通快进推送 KC1。
STATE 收口为 **B done / next=C**；本批不启动后续阶段。
