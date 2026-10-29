# Phase D 受保护 PR 整合执行账

[Document-Meta]
Document: docs/04-internal/repo-consolidation-d-merge-2026-10-29.md
Version: 1.0 (scoped execution; zero merges)
Last-Modified: 2026-10-29
Status: D PARTIAL（合并前置条件受阻）；文档交付完成，next=E；未执行 E–G
Author: Codex

## 结果与 main 边界

本批依次复核 D0、D1 #64、D2 #98、D3 #70，**MERGED=none**。
#64/#70 达到 CLEAN + MERGEABLE、六个 required checks 均 pass，但仓库
`delete_branch_on_merge=true`，两条同仓 head branch 均 `protected=false`，与用户的 **D 不删分支** 硬规则冲突。
本批在 merge 前停止，**未执行 `gh pr merge` 合并动作**（仅查看其帮助），不把预检阻塞伪装成权限拒绝或一次失败的 merge 命令。
#98 仍 BLOCKED，有失败/等待中的 required checks，因此未尝试 merge、排队 auto-merge 或绕过规则。

| ref / milestone | full SHA | change |
| --- | --- | --- |
| origin/main before D | `4ea9c26095e22b83dc30faaf8f7148ce7cbdb82c` | 基线 |
| origin/main after D decisions | `4ea9c26095e22b83dc30faaf8f7148ce7cbdb82c` | 未推进；没有 main 写操作 |
| KC1 D 起点 | `cb41c09ddff0af09b0baeb634d2bd19775536442` | clean；与 C 交付一致 |

本报告只向 `refactor/kc1-kernel-convergence` 提交文档并正常快进 push；最终完整 KC1 SHA 由外部 STATE、
`D-RESULT.md` 和远端确认回执记录。没有本地 main checkout/merge，也没有直接 push main。
D 的 **PARTIAL** 指本轮合并受阻；软件正式 Final Acceptance 也独立保持 **PARTIAL**，两者不混同。
[C 分类](repo-consolidation-c-classify-2026-10-29.md) 与 [B inventory](repo-consolidation-b-inventory-2026-10-29.md) 的历史快照不追溯改写。

## D0：保护规则、权限与 PR drift

起点 `git fetch --prune origin && git status && git log -3 --oneline` 成功，工作树 clean。
本轮重新查询全部原 16 个 open PR，其集合没有新增/移除；除 #98 因 C 文档从 C 的提交前快照推进到当前起点外，
其他 PR head SHA 与 C 一致。完整 head/base、rollup、required-check 对账、API 时间及 Git refs 在 JSON。

main branch metadata 明确返回 `protected=true`，六个 required context 的 enforcement 为 `everyone`。
仓库允许 merge commit / squash / rebase merge；当前身份 `push=true`，`admin=false`、`maintain=false`，
`allow_auto_merge=false`，**没有尝试管理员旁路或修改配置**。
`GET /repos/Apeireth/Apeireth/branches/main/protection` 返回 **HTTP 404 / gh exit 1**，
所以未取得完整 review/bypass/保护细项；`rules/branches/main` 返回空列表，不能据此说 main 没有保护。
已取得的 protected 标志与 required checks 原样记录，不猜测 404 的确切权限原因。

六个 required contexts：

- `cargo kani (approval SM + property families, mirror crate)`
- `cargo nextest (ubuntu-latest)`
- `cargo fmt --check`
- `cargo deny check (bans + licenses + sources + advisories)`
- `docs truth gate (P1c blocking)`
- `8 硬墙守门 (LOCKED / version / baseline / 14 键 / V1136)`

下表为 D1 预检停止、重新 fetch、完成 #98 状态正文更新后的只读快照（UTC 与各命令起止时间见 `after-d1/receipts.json`）。
P/F/W/M 分别是 pass / fail / pending / missing，按 main 的六个 context 全集对账。
这些是异步查询时的观测，不是跨 API 原子事务，也不是文档提交后新 head 的 CI 结果。
`gh pr checks` 查询退出码不代替 bucket/结论；缺失结果不能算通过。

| PR | head SHA | mergeable | mergeStateStatus | required P/F/W/M | D disposition |
| --- | --- | --- | --- | --- | --- |
| [98](https://github.com/Apeireth/Apeireth/pull/98) | `cb41c09ddff0af09b0baeb634d2bd19775536442` | MERGEABLE | BLOCKED | 2/2/2/0 | NO_MERGE_BLOCKED_CI |
| [97](https://github.com/Apeireth/Apeireth/pull/97) | `dbc671259ac6b0926390af6e94d6359b516b85cb` | MERGEABLE | UNSTABLE | 6/0/0/0 | DEFER_BOT_SERIAL_REVIEW |
| [96](https://github.com/Apeireth/Apeireth/pull/96) | `926b6a866bffe58d2d9e51b37586e007754fceea` | MERGEABLE | UNSTABLE | 6/0/0/0 | DEFER_BOT_SERIAL_REVIEW |
| [95](https://github.com/Apeireth/Apeireth/pull/95) | `71631d349ac72b140cce71710c1fdb584b4ca2e4` | MERGEABLE | CLEAN | 6/0/0/0 | DEFER_SCOPE |
| [94](https://github.com/Apeireth/Apeireth/pull/94) | `caf499b769cd1d3aa0a6281c77acc5e1ca5c6ae7` | MERGEABLE | BLOCKED | 5/1/0/0 | DEFER_SCOPE |
| [78](https://github.com/Apeireth/Apeireth/pull/78) | `5cfbccb78c7ff2310ae042db8984bc5f9460e027` | MERGEABLE | UNSTABLE | 6/0/0/0 | DEFER_SCOPE |
| [70](https://github.com/Apeireth/Apeireth/pull/70) | `bb51f380b3ce29b27b72ec6f07749b843c72cca5` | MERGEABLE | CLEAN | 6/0/0/0 | NO_MERGE_AUTO_DELETE_CONFLICT |
| [64](https://github.com/Apeireth/Apeireth/pull/64) | `50576a88ec077ce3b86a95e91460b61dab85d91f` | MERGEABLE | CLEAN | 6/0/0/0 | NO_MERGE_AUTO_DELETE_CONFLICT |
| [34](https://github.com/Apeireth/Apeireth/pull/34) | `50d9684d04e9b6e0cca97520414d76a9e2b97258` | MERGEABLE | CLEAN | 6/0/0/0 | DEFER_FORK_REVIEW |
| [33](https://github.com/Apeireth/Apeireth/pull/33) | `a255c0bfef2ed6ab47b416e49dddd48459e0d577` | MERGEABLE | UNSTABLE | 6/0/0/0 | DEFER_BOT_SERIAL_REVIEW |
| [31](https://github.com/Apeireth/Apeireth/pull/31) | `b43ff3fd7316641748adb13af3071afc4ab6dc89` | CONFLICTING | DIRTY | 0/0/0/6 | DEFER_SCOPE |
| [29](https://github.com/Apeireth/Apeireth/pull/29) | `37f281caca8c33d0642b8cc963111970816fafa7` | MERGEABLE | BLOCKED | 4/0/0/2 | DEFER_BOT_SERIAL_REVIEW |
| [22](https://github.com/Apeireth/Apeireth/pull/22) | `0c65ac7e7fc7029e8875da46c9f9dd6cf782c720` | CONFLICTING | DIRTY | 2/2/0/2 | DEFER_BOT_SERIAL_REVIEW |
| [20](https://github.com/Apeireth/Apeireth/pull/20) | `d974a1096f1b039f480a4615e1f0a61908dd4351` | MERGEABLE | BLOCKED | 4/0/0/2 | DEFER_BOT_SERIAL_REVIEW |
| [19](https://github.com/Apeireth/Apeireth/pull/19) | `3aea2ccc264c63493d72749eb1e1b6976e2e7767` | CONFLICTING | DIRTY | 0/0/0/6 | DEFER_SCOPE |
| [17](https://github.com/Apeireth/Apeireth/pull/17) | `31b8ab1a2ff33e46d481a3178c89c2529b3cbca6` | MERGEABLE | BLOCKED | 4/0/0/2 | DEFER_BOT_SERIAL_REVIEW |

#70 初次为 UNKNOWN/UNKNOWN，重查后为 MERGEABLE/CLEAN，未在 UNKNOWN 时执行任何 merge。
#31 没有报告 checks，`gh pr checks --required` 返回 exit 1 / `no checks reported on the 'feature/mesh-rc2-integration' branch`，
绝不把空列表当作全绿。#95 即使 CLEAN/六项绿，也按用户 scope 禁止 wholesale merge。

## D1 / D3：#64 与 #70 的非删除前置条件

#64 tip `50576a88ec077ce3b86a95e91460b61dab85d91f` 与 #70 tip
`bb51f380b3ce29b27b72ec6f07749b843c72cca5` 均在当前 main 上 MERGEABLE/CLEAN，六项 required pass。
#64 YAML/审计证据链与 lock 修复、#70 Unix ETXTBSY fixture 修复仍留在原 open PR；本批不改源码来绕过受阻的 merge。

[官方自动删除说明](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/configuring-pull-request-merges/managing-the-automatic-deletion-of-branches)
表明启用该仓库设置可在 merge 后自动删除 head，关闭设置需要管理员权限，保护规则也可能阻止自动删除。
当前两条 head 无保护，不能保证 merge 后保留，因此 D 的不删除规则优先；不试验性 merge 再恢复，也不擅自改设置/保护。
**这是预检发现的实际配置冲突，不是声称自动审批拒绝、gh merge 权限失败或用户撤销授权。**
命令帮助未提供已验证的单次保留 head 覆盖方式；没有使用 `--admin`、`--auto`、`--delete-branch` 或强推。

剩余出口：由后续授权的仓库管理动作解决自动删除与 no-delete 的冲突，再重新取 exact-head/required-check/保护快照。
若可合并，普通受保护 **merge commit** 是保留现有提交身份的首选（仓库允许），以 exact head 条件防止 head drift；
不在 C/D 之间擅自 squash/rebase 历史。若未来 main 推进导致冲突，另列新的 additive branch/PR 方案，原 ref 保留。
当前 main 未变，#70 本轮无需 force-rebase 或源码刷新；只是合并前置条件未满足。

## D2：#98 保持正式 Partial，并列出实际 CI 阻塞

依用户明确授权，执行 `gh pr edit 98 --repo Apeireth/Apeireth --body-file /tmp/apeireth-d/pr98-body.txt`，
在原正文前增加 dated formal-status 小节；原正文全文作为逐字相同 suffix 保留，正文前后 SHA256、fresh head 与回读结果均归档。
新增内容明确：**Formal Final Acceptance: PARTIAL**；旧 Dev legacy `<10 ms` 不变，A4 的读数只是历史证据；
normal_start 新定义 **PROPOSED_UNAPPROVED**，没有用户批准；真实 cold cache **UNAVAILABLE**，未用 normal cache 替代。
同时说明当前 BLOCKED 与自动删除冲突。没有改标题/基线/开关/测试/保护规则，没有宣称正式验收完成。

当前 #98 的失败 rollup（完整原始状态和 URL 见 `after-d1/pr-98.json`，不打印失败日志中的潜在敏感内容）：

| check | observed state | required by visible context list | evidence |
| --- | --- | --- | --- |
| cargo deny check (bans + licenses + sources + advisories) | FAILURE | Y | [job](https://github.com/Apeireth/Apeireth/actions/runs/40802675052/job/123214201486) |
| docs truth gate (P1c blocking) | FAILURE | Y | [job](https://github.com/Apeireth/Apeireth/actions/runs/40802675045/job/123214201308) |
| 孤儿 API 棘轮（基线只减不增） | FAILURE | N | [job](https://github.com/Apeireth/Apeireth/actions/runs/40802675067/job/123214201496) |
| wiring ledger (R3a/R3c) | FAILURE | N | [job](https://github.com/Apeireth/Apeireth/actions/runs/40802675146/job/123214201883) |
| Secret scan (R215 防御层, 高置信凭证 pattern) | FAILURE | N | [job](https://github.com/Apeireth/Apeireth/actions/runs/40802675060/job/123214201906) |

六项 required 的逐项观测：

| required context | state | bucket |
| --- | --- | --- |
| cargo kani (approval SM + property families, mirror crate) | QUEUED | pending |
| cargo nextest (ubuntu-latest) | IN_PROGRESS | pending |
| cargo fmt --check | SUCCESS | pass |
| cargo deny check (bans + licenses + sources + advisories) | FAILURE | fail |
| docs truth gate (P1c blocking) | FAILURE | fail |
| 8 硬墙守门 (LOCKED / version / baseline / 14 键 / V1136) | SUCCESS | pass |

D0 的 required 查询一度只返回五项，按六项全集对账把缺失 Kani aggregate 标为 missing；后续快照另记实际出现后的状态，
不把早期缺失/后续 pending 冒称 pass。其他非 required 失败也逐名保留，不能用 A4 本地 gate_full=0 覆盖本轮远端失败。
本批没有排队 merge、管理员绕过、伪造 check、关闭/忽略测试、弱化安全或把 CI 失败归为未经证实的 flake。
本轮 docs commit 会推进 #98 head 并触发新 CI；新 head 必须重新评估，不继承本表的绿/红结论。

## 延后范围与串行计划

- #95/#94：只保留，禁止 wholesale merge；后续在当前 main 上按 owner scope 审聚焦 env-lock、内存/接线/安全增量，保留原 expanded histories。
- #31/#19：当前 CONFLICTING/DIRTY；只记录 fresh additive branch/PR 方案。Mesh 适配当前 Kernel ports；#19 从新 main 整理 guide-only，保留 fork 原 head，不能把历史大 diff 当文档 merge。
- #78：当前 UNSTABLE、存在失败，独立 UI readiness/owner 审查未完成；不 wholesale merge。#34 虽重查 CLEAN，属于 fork/桌面审查范围，本批不擅自合并或关闭。
- Dependabot 依序在 #64 与 KC1 baseline 确定后再评估：先按新 advisory/兼容事实审 #29/#22，再 #33/#97；每次一个共享 Cargo.lock，逐次验证，不让旧 lock 覆盖修复。#17/#20/#96 基于修复后的 workflows 单独审 runner/action 与证据上传兼容。实际安全紧迫性应以新扫描调整，D 未新扫 advisory，全部 defer。
- D 的零合并不构成 E 删除授权。B 的 33 个 unique-unmerged 行及所有 archive/fork 工作仍保留；原 tags/JSON/alias 不变成恢复锚。后续 E 的创建/核验/去留动作均未执行。

## 可复核证据与验证范围

日期 **2026-10-29**；工具实测 Git 2.47.3、gh 2.103.0、Python 3.13.5、PowerShell 7.5.4，见 `d-toolchain.log`。
`/tmp/apeireth-d/` 保存 d0/after-d1 全部 PR/保护/required JSON、逐调用 stderr/真实 exit 与 UTC、Git refs、
D1/D3 policy decisions、#98 原/新正文和回读验证、C/B 输入、`d-summary.json`。
归档 `/workspace/apeireth-codex/d-evidence/`；根 `d-*.log` 与同名 `.exit` 同时镜像外部目录。
保护详情 404 与 #31 无 checks 是真实 read limitation/negative result，不因采集器成功退出就宣称查询全部成功。

```bash
git fetch --prune origin
gh api repos/Apeireth/Apeireth/branches/main
gh api repos/Apeireth/Apeireth/branches/main/protection
gh api repos/Apeireth/Apeireth/rules/branches/main
gh pr view 98 --repo Apeireth/Apeireth --json headRefOid,mergeable,mergeStateStatus,statusCheckRollup
gh pr checks 98 --repo Apeireth/Apeireth --required --json name,state,bucket,link
python /tmp/apeireth-d/snapshot.py d0
python /tmp/apeireth-d/verify.py
python scripts/docs_links.py --check
python scripts/docs_index.py --check
python scripts/docs_truth.py --verify --strict
python scripts/docs_truth.py --check
python scripts/arch_guard.py --strict
```

归档里的 snapshot 采集器只读；重新执行是新的异步快照，不能覆盖原记录或冒称重建旧状态。
`d2_prepare.py` 与 `gh pr edit` 属于已执行一次的授权正文更改，**不是只读复现实验**，不得随意重复覆盖。
报告生成器从归档 JSON 再现表格；验证器核对六项 required 全集、原 16 PR、正文 Partial/原 suffix、main 不变、
零 merge、所有非 KC1 refs/tags/stash/worktree 不变及精确 docs scope。

交付检查为 docs links/index、strict docs truth/generated 与 arch_guard --strict；没有新产品源码，所以不重跑全量 Rust gates 或性能矩阵，
不冒称主分支/PR 已由本地门禁认证。Codex commit.ps1 + ArchStrict 提交三份文档；两个 non-executable hook 体另用 Bash 实跑，
不改 hook 配置/模式、不绕过；fetch 后确认 KC1 普通快进 push 与远端 SHA。

[Final Acceptance](kernel-convergence-final-acceptance-2026-10-28.md) **PARTIAL** 保留，旧启动门槛与新定义未批准状态不变。
STATE 收口为 **D PARTIAL（文档/复核交付完成，合并阻塞保留），next=E**；E–G 尚未执行。
