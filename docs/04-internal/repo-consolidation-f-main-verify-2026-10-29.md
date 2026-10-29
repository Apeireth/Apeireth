# F Main verification

> Historical F snapshot retained. Post-D3 protected integration and fresh exact-main evidence are in [F2](repo-consolidation-f2-main-verify-2026-10-29.md); the earlier unintegrated-main/deferred-gate statements below describe their original snapshot.

[Document-Meta]
Document: docs/04-internal/repo-consolidation-f-main-verify-2026-10-29.md
Version: 1.0
Last-Modified: 2026-10-29
Status: VERIFY=pass（本节所列 docs/architecture 范围）；full gate deferred
Author: Codex

## Exact main 和验证范围

fetch 后 `origin/main=ac73436c65bdbb34369421c2e8d896bf538e591d`；[D2](repo-consolidation-d2-anchored-merges-2026-10-29.md) 的 #64/#70 是本次新增受保护合并。
主工作树保持 KC1 `refactor/kc1-kernel-convergence`，F 起点 `d953fa3da2853de2d8a271db2572ef7cecc2f2c5`。
另建 detached worktree `/tmp/apeireth-f/main-worktree`，HEAD 精确固定 main SHA，无本地 main 分支、无 commit/merge/push main。
在该 worktree 内运行**该 SHA 自带**的脚本与契约；没有从 KC1 复制较新检查器去替换 main。

| scope | exact command | exit | log |
| --- | --- | --- | --- |
| truth | `python scripts/docs_truth.py --verify --strict` | 0 | `f-main-truth.log` |
| generated | `python scripts/docs_truth.py --check` | 0 | `f-main-generated.log` |
| links | `python scripts/docs_links.py --check` | 0 | `f-main-links.log` |
| index | `python scripts/docs_index.py --check` | 0 | `f-main-index.log` |
| arch | `python scripts/arch_guard.py --strict` | 0 | `f-main-arch.log` |
| arch-doc | `python scripts/arch_doc.py --check` | 0 | `f-main-arch-doc.log` |

上述六项均 exit 0，**VERIFY=pass 仅指这些实际执行的 docs/architecture 项**，不表示全量软件或硬件验收通过。
worktree 执行后 `git status --porcelain` 为空，未修改 main tracked/untracked 文件；脚本版本另存 evidence，
随后正常移除 detached worktree（无 force），原 KC1 保持 clean，仓库再次仅有原 worktree。

## Full gate 与未整合边界

本轮 **未重跑 `gate_full.ps1`**。原因：F 按任务做当前 main 可行的 docs/architecture 检查；
#98 尚未通过/合并，main 还不是最终 Kernel 整合候选，应在后续受保护整合完成后对最终 candidate 新跑完整软件门禁。
不是声称磁盘/OOM/ENOSPC 阻塞，也不是用旧 A4 gate_full=0 替代新 main 验证。
本次没有运行 Rust tests/clippy、前端、Android、live/hardware 或新的性能矩阵；没有为通过修改断言、ignore、初始化或阈值。

`git merge-base --is-ancestor <F起点KC1> <main>` 返回 1，**KC1 尚未包含在 main**。
#98 当时 **OPEN / BLOCKED / MERGEABLE**；失败/未完成的远端 checks 在 `pr98.json`，
本批不 bypass/合并或自动归因 flake。当前 worktree 的通过不能代表该 PR checks 已通过。

**Kernel Convergence Complete (under approved startup definition)** 是批准后的 KC1 验收判断，
不是 main 已含全部 Kernel 工作的声明。A1b normal_start Dev P50=7.5642 ms 为保留证据，
legacy OLD_COLD_P50=11.4143 ms unmet 历史、真实冷缓存 UNAVAILABLE；无本轮新样本。
A4 原五段与软件 PASS 仅按原源码范围引用；本批也不增加 HARDWARE VALIDATED 声明。

## 证据与交付

原始 `/tmp/apeireth-f/main-verify.json`、`f-main-*.log/.exit`、main SHA/指令/脚本副本、
worktree add/remove、main/KC1 祖先与 fresh PR 状态均归档到 `/workspace/apeireth-codex/f-evidence/`。
`verify_main.py` 保留完整命令/cwd 与负祖先 exit 1，不能把非祖先写成通过。

KC1 的本批文档另跑 links/index、strict docs truth/generated、strict arch 与 Codex commit.ps1/ArchStrict；
non-executable hook 体经 Bash 实跑、普通快进 push。它们与上表的 main-worktree 日志分目录/前缀，不能混为一次验证。
没有产品/LOCKED/Cargo.lock/version、设置、分支删除或 recovery tag 变动。
下一步 G 汇总 A–G、审批、实际合并/恢复/清理、当前未整合历史与 #98 阻塞，明确后续软件/PR 工作。
