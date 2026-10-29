# E 恢复锚与谨慎清理账

[Document-Meta]
Document: docs/04-internal/repo-consolidation-e-cleanup-2026-10-29.md
Version: 1.0
Last-Modified: 2026-10-29
Status: E delivered；verified recovery tags=11（新增9，复核D2两项），individual deletions=7
Author: Codex

## 范围和结果

基线为 [C 分类](repo-consolidation-c-classify-2026-10-29.md) 的九个 SUPERSEDED 行，
以及 [D2 已合并 heads](repo-consolidation-d2-anchored-merges-2026-10-29.md)。先 fetch/prune、查当前 main/open PR，
在任何 E 删除前已推送并核验对应 exact-tip recovery tag。**ANCHORS=11** 是本轮验证的不同 tags 总数，
含 D2 两个已有 tags；新建 **9** 个，不把复核算成新建。**DELETED=7** 仅计 E 发起的删除命令，
D2 的服务器自动删除单列，不混入 E 数字。

main 保持 `ac73436c65bdbb34369421c2e8d896bf538e591d`，本阶段没有 merge、设置/保护变动或 direct/force push main。
七个删除均为 tip 已在 main 图上的祖先；C 的内容包含行 B031/B036 仍有 unique commit IDs，未获本任务逐 ID 的明确去留决定，
因此**只建锚、不删**。ARCHIVE_ONLY、UNIQUE_PRESERVE、NEEDS_OWNER_DECISION、KEEP_ACTIVE 与未归并 fork 工作未删除。

| ID | original ref | original full SHA | verified recovery tag | E outcome | reason |
| --- | --- | --- | --- | --- | --- |
| B008 | `refs/heads/batch-2026-10-22` | `b613f0ebb9da45b761f7e5d06a383ebf7a50cbce` | `recovery/pre-cleanup-b008-b613f0eb` | DELETED | Individually deleted after merged-ancestry, exact-tip, no-open-PR/worktree and verified recovery checks; no force. |
| B021 | `refs/heads/docs/governance-followup-2026-10-24` | `e9b0861aaadfef09629b4ca1533e8f3bce0a6b72` | `recovery/pre-cleanup-b021-e9b0861a` | DELETED | Individually deleted after merged-ancestry, exact-tip, no-open-PR/worktree and verified recovery checks; no force. |
| B025 | `refs/heads/feature/cognitive-infrastructure-vnext` | `f1e8ee0e0b6b2802332dafd00023a7dd1f2a2fe6` | `recovery/pre-cleanup-b025-f1e8ee0e` | DELETED | Individually deleted after merged-ancestry, exact-tip, no-open-PR/worktree and verified recovery checks; no force. |
| B026 | `refs/heads/feature/cognitive-memory-convergence-v3` | `b50f9cd25638aa50cde0e65b64196501c89222b7` | `recovery/pre-cleanup-b026-b50f9cd2` | DELETED | Individually deleted after merged-ancestry, exact-tip, no-open-PR/worktree and verified recovery checks; no force. |
| B027 | `refs/heads/feature/daily-batch-20261003` | `a612f6f1d4aa90fa237283e0f299e24cbbf4f4f3` | `recovery/pre-cleanup-b027-a612f6f1` | DELETED | Individually deleted after merged-ancestry, exact-tip, no-open-PR/worktree and verified recovery checks; no force. |
| B028 | `refs/heads/feature/memory-v2.2-production-completion` | `f1e8ee0e0b6b2802332dafd00023a7dd1f2a2fe6` | `recovery/pre-cleanup-b028-f1e8ee0e` | DELETED | Individually deleted after merged-ancestry, exact-tip, no-open-PR/worktree and verified recovery checks; no force. |
| B031 | `refs/heads/fix/bench-idle-footprint-isolation` | `4669315493b1cd5920aa9debd26794e21ddefaa1` | `recovery/pre-cleanup-b031-46693154` | KEPT | Unique commit IDs remain; content containment is not owner disposition. Tag verified, ref kept. |
| B036 | `refs/heads/fix/gate-nested-worktree` | `c715f668fb2494d0307a310b369bcb8487b36e88` | `recovery/pre-cleanup-b036-c715f668` | KEPT | Unique commit IDs remain; content containment is not owner disposition. Tag verified, ref kept. |
| B037 | `refs/heads/fix/historical-debt` | `196eebff2aaedbb76d534637f6aca4d575402944` | `recovery/pre-cleanup-b037-196eebff` | DELETED | Individually deleted after merged-ancestry, exact-tip, no-open-PR/worktree and verified recovery checks; no force. |

每个删除前再次确认：exact remote tip 未漂移、对应 tag 的 remote SHA 一致、tip 是当前 origin/main 祖先、
当前完整 open PR 中无该 head/base、本地 worktree 未使用该分支；然后单独 `git push origin --delete <name>`，
不使用 force 或批量通配符。删除后核对远端 absent、原 tip 仍在 tag，fetch/prune 清理已消失的 tracking refs。
B025/B028 与 ARCHIVE_ONLY 的 B005 同 tip，B005 本身明确保留；alias 不等于自动去留授权。

#64 在 D2 merge 后自动删 head；#70 的 D2 即时观测尚在，E 重查已 absent，故没有 E 发起的 #70 删除。
两者在各自 merge 之前已有已验证恢复锚，E 再次核验。KC1/#98 未 merge，活动 KC1/main 均保持，未触发 active-line recreate。

## 完整恢复账

下列每项均核验 `git rev-parse refs/tags/<tag>`、远端 `ls-remote` 的 exact SHA，
`rev-list --objects --missing=print` missing=0，以及 `git fsck --connectivity-only --no-dangling <sha>` exit 0。
ledger 另存核验 UTC、reachable object 数与清单哈希、原 ref、完整恢复命令、每条真实退出码。

| tag | exact SHA | covered original ref | reachable objects |
| --- | --- | --- | ---: |
| `recovery/pre-cleanup-b008-b613f0eb` | `b613f0ebb9da45b761f7e5d06a383ebf7a50cbce` | `refs/heads/batch-2026-10-22` | 39663 |
| `recovery/pre-cleanup-b021-e9b0861a` | `e9b0861aaadfef09629b4ca1533e8f3bce0a6b72` | `refs/heads/docs/governance-followup-2026-10-24` | 41414 |
| `recovery/pre-cleanup-b025-f1e8ee0e` | `f1e8ee0e0b6b2802332dafd00023a7dd1f2a2fe6` | `refs/heads/feature/cognitive-infrastructure-vnext` | 28869 |
| `recovery/pre-cleanup-b026-b50f9cd2` | `b50f9cd25638aa50cde0e65b64196501c89222b7` | `refs/heads/feature/cognitive-memory-convergence-v3` | 29872 |
| `recovery/pre-cleanup-b027-a612f6f1` | `a612f6f1d4aa90fa237283e0f299e24cbbf4f4f3` | `refs/heads/feature/daily-batch-20261003` | 35151 |
| `recovery/pre-cleanup-b028-f1e8ee0e` | `f1e8ee0e0b6b2802332dafd00023a7dd1f2a2fe6` | `refs/heads/feature/memory-v2.2-production-completion` | 28869 |
| `recovery/pre-cleanup-b031-46693154` | `4669315493b1cd5920aa9debd26794e21ddefaa1` | `refs/heads/fix/bench-idle-footprint-isolation` | 42148 |
| `recovery/pre-cleanup-b036-c715f668` | `c715f668fb2494d0307a310b369bcb8487b36e88` | `refs/heads/fix/gate-nested-worktree` | 42148 |
| `recovery/pre-cleanup-b037-196eebff` | `196eebff2aaedbb76d534637f6aca4d575402944` | `refs/heads/fix/historical-debt` | 35522 |
| `recovery/pre-merge-pr64-50576a88` | `50576a88ec077ce3b86a95e91460b61dab85d91f` | `refs/heads/fix/ci-cargo-audit-yaml-2026-10-24` | 41481 |
| `recovery/pre-merge-pr70-bb51f380` | `bb51f380b3ce29b27b72ec6f07749b843c72cca5` | `refs/heads/fix/external-tool-etxtbsy-flake-2026-10-24` | 41732 |

恢复模板（执行时使用 ledger 的完整原 ref/tag，禁止强推覆盖已有新工作）：

```bash
git fetch origin refs/tags/<tag>:refs/tags/<tag>
git rev-parse refs/tags/<tag>
git push origin refs/tags/<tag>:refs/heads/<original-branch>
```

只有在原分支 absent 或普通快进允许时才会恢复成功；如有新工作，先比较，不 force。
本阶段没有 restore 演练写原 ref；已验证的是用户规定的 tag exact SHA/remote/object reachability，
不能宣称做了独立 clone 全库恢复。未创建 bundle，原 B/C 的 JSON/SHA 清单仍是 metadata，不冒充恢复锚。

## 证据、验证与后续

`/tmp/apeireth-e/cleanup-ledger.json` 与每个 `Bxxx-anchor.json` / reused D2 anchor 记录完整动作，
`e-*.log/.exit` 记录 tag push、对象校验、逐 ref 删除/远端 absence、PR/worktree/祖先检查。
负祖先检查 exit 1（B031/B036）是实际“非祖先”结果，不当作通过；这两行原 ref 均再次核对保留。
归档 `/workspace/apeireth-codex/e-evidence/`。其余 C refs 的 exact tip、保护类保留在脚本末尾逐一验证。

KC1 收口跑 docs links/index、strict docs truth/generated、strict arch、whitespace，Codex commit.ps1 + ArchStrict；
non-executable hook 体经 Bash 实跑，普通 FF push。没有产品/测试/Cargo/LOCKED/版本变更或新性能采样。
{STATUS} 保持在已批准/保留证据范围；#98 仍需普通远端门禁，不把清理进度称为合并或软件验证成功。
下一步 F，在独立 detached main worktree 运行可行的 docs/architecture 检查，记录 full gate 的实际执行/延后范围。
