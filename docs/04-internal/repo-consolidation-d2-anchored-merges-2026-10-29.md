# D2 Anchored protected merges

[Document-Meta]
Document: docs/04-internal/repo-consolidation-d2-anchored-merges-2026-10-29.md
Version: 1.0
Last-Modified: 2026-10-29
Status: D2 delivered；#64/#70 merged，#98 required checks 未通过
Author: Codex

## 结果与授权范围

用户本轮明确批准：在 merge 可能自动删 head 之前先推送、核验 exact-head recovery tag；KEEP_ACTIVE 若被删除立即恢复。
这替代 [D1](repo-consolidation-d-merge-2026-10-29.md) 当时“不删分支且未授权恢复锚”的配置阻塞，未改仓库设置/保护。
起点 clean `3432d34150153f05396406b608802935d36b0158`；始终在 KC1。main 只通过两次普通受保护 PR merge 前进，
没有本地 merge main、直接/force push main、管理员旁路或 auto-merge 排队。

| milestone | origin/main full SHA |
| --- | --- |
| before | `4ea9c26095e22b83dc30faaf8f7148ce7cbdb82c` |
| after PR #64 | `554c964b3f7e6e0e7851ccfc6abd10f176e982cf` |
| after PR #70 / D2 decisions | `ac73436c65bdbb34369421c2e8d896bf538e591d` |

| PR | exact head SHA | outcome | merge commit | recovery tag |
| --- | --- | --- | --- | --- |
| 64 | `50576a88ec077ce3b86a95e91460b61dab85d91f` | MERGED | `554c964b3f7e6e0e7851ccfc6abd10f176e982cf` | recovery/pre-merge-pr64-50576a88 |
| 70 | `bb51f380b3ce29b27b72ec6f07749b843c72cca5` | MERGED | `ac73436c65bdbb34369421c2e8d896bf538e591d` | recovery/pre-merge-pr70-bb51f380 |
| 98 | `3432d34150153f05396406b608802935d36b0158` | OPEN / BLOCKED | `—` | 未建：未进入 merge |

#64/#70 在 anchor 前、后都重新读取 exact head，均 CLEAN/MERGEABLE、main 可见六项 required 均 SUCCESS。
随后依次使用 `gh pr merge N --repo Apeireth/Apeireth --merge --match-head-commit <fullsha>`，exit 0；
fetch 后确认 main 变化、PR MERGED、head 是 main 祖先。#70 在 #64 改 base 后一度 UNKNOWN，未在 UNKNOWN 时 merge；重查 CLEAN 后才执行。
#64 head 自动删除已确认；#70 合并后的即时 head 查询仍存在，属于观测时刻事实，E 将再次核对。
无论分支后续是否存在，两个 exact tips 已有远端 recovery tag 覆盖。

## 合并前已验证恢复账

以下均是 lightweight tag，`git rev-parse refs/tags/<tag>` 直接返回原 head commit SHA；
`git ls-remote origin refs/tags/<tag>` 与之完全一致。`rev-list --objects --missing=print` 无缺失对象，
`git fsck --connectivity-only --no-dangling <sha>` exit 0；在 merge 命令之前完成并记录 UTC。

| tag | original commit | reachable objects | verification |
| --- | --- | ---: | --- |
| `recovery/pre-merge-pr64-50576a88` | `50576a88ec077ce3b86a95e91460b61dab85d91f` | 41481 | local/remote exact SHA；missing=0；fsck exit 0 |
| `recovery/pre-merge-pr70-bb51f380` | `bb51f380b3ce29b27b72ec6f07749b843c72cca5` | 41732 | local/remote exact SHA；missing=0；fsck exit 0 |

每个 anchor JSON 保存 tag、原 ref、完整 SHA、时间、对象清单哈希、命令与真实退出码。恢复命令（仅记录，未对已归并 topic 重建）：

```bash
git fetch origin refs/tags/recovery/pre-merge-pr64-50576a88:refs/tags/recovery/pre-merge-pr64-50576a88
git rev-parse refs/tags/recovery/pre-merge-pr64-50576a88
git push origin refs/tags/recovery/pre-merge-pr64-50576a88:refs/heads/fix/ci-cargo-audit-yaml-2026-10-24
```

```bash
git fetch origin refs/tags/recovery/pre-merge-pr70-bb51f380:refs/tags/recovery/pre-merge-pr70-bb51f380
git rev-parse refs/tags/recovery/pre-merge-pr70-bb51f380
git push origin refs/tags/recovery/pre-merge-pr70-bb51f380:refs/heads/fix/external-tool-etxtbsy-flake-2026-10-24
```

若将来 #98 合并：先以它当时 exact head 建 `recovery/pre-merge-pr98-<8sha>` 并执行同样核验；
若 KC1 remote head 被自动删除，在 fetch/prune 前即用 recovery tag 向 `refs/heads/refactor/kc1-kernel-convergence` 普通 push 重建。
本次 #98 没有 merge/删除，活动 KC1 始终存在，所以未执行重建，也不虚报 #98 anchor。

## #98 与其他 PR 的剩余状态

合并后快照 #98：**MERGEABLE / BLOCKED**，仍 OPEN，未 merge。
required 的 cargo deny 与 docs truth 为 FAILURE，nextest 未完成，Kani aggregate 初次缺失/随后状态见原始 snapshots；
不能把缺失/等待算通过。其他失败 rollup 与完整 URL 在 `post-merges/pr-98.json`；不读取可能输出凭据的失败日志。
保护详情端点 HTTP404、main branch metadata `protected=true` 与六项 required 并列保留；不把 404 当作无保护。

**Kernel Convergence Complete (under approved startup definition)** 保持，来源是已批准定义和 A1b/A4 保留证据；
normal_start Dev P50=7.5642 ms，legacy OLD_COLD_P50=11.4143 ms unmet 历史，真实 cold cache UNAVAILABLE。
这不等于 #98 PR checks 已通过，也不表示 main 已含 KC1；`git merge-base` 的 KC1/main 关系将在 F/G 单列。
#95/#94/#31/#19/#78、dependabot 和 fork 未被 wholesale merge、关闭或丢弃；批准 PR body 已正确，无需再写。

## 校验与交付

证据 `/tmp/apeireth-d2/` → `/workspace/apeireth-codex/d2-evidence/`；`initial/`、`post-merges/`、
`prN-before/after-anchor/after.json`、tag/对象/merge/fetch 日志与 `.exit`、`merge-results.json` 可复核完整顺序。
初次 helper 把不存在的本地 tag 查询 exit128 当异常，尚未创建 tag 或 merge；修正 quiet 存在性查询后重新取 fresh checks。
`attempt1/` 原诊断完整保留，不冒称权限拒绝或漏记失败。没有改产品源码。

收口在 KC1 跑 docs links/index、strict docs truth/generated、arch_guard --strict、whitespace 和 docs scope；
commit.ps1 Codex/ArchStrict，non-executable hook 体另经 Bash 实跑，fetch 后 ordinary FF-only push。
不把 KC1 文档检查冒称新 main 全量验证；F 将独立验证 main，旧 A4 全量不替代新 main。
下一步 E：仅可在已验证 anchor 与合并/明确去留条件都满足后清理，保留 unresolved unique/archive/owner/fork 和活动 KC1/main。
