# ADR-0006: 集成 rebase skip policy 实施细节 (V23 fail-forward)

**状态**: ✅ Accepted
**日期**: 2026-08-03
**作者**: architect2 (claude-sonnet-4.5, Ponytail: full)
**任务**: round8-08 ADR 0003-0006 补齐 (V28.1 增量)
**依据**: ADR-0009 (integration rebase skip policy) + V23 fail-forward 安全原则
**影响范围**: 仅文档新增, 不修改 LOCKED, 不修改源

---

## 上下文 (Context)

ADR-0009 已建立 integration rebase skip policy 高层原则:
- 单 worktree 单分支流程
- 显式 refspec push
- 不 silent no-op

但实施细节 (具体 git 命令、冲突处理、auto-resolve 阈值) 未文档化.

## 决策 (Decision)

### 1. 单一 worktree 单一本地分支

```bash
# 仅使用 rebase/d7d8-into-integration 作为本地分支
# 显式 push 到 integration-worktree remote
git push integration-worktree rebase/d7d8-into-integration:team/e8de47ae-0e59-459d-a763-88e52b7706c8/integration
```

### 2. V23 fail-forward 安全

- **fetch 先于 push**: `git fetch integration-worktree` 必须执行
- **冲突检测**: 若 push 被拒 ("Updates were rejected because a pushed branch tip is behind"),
  必须先 merge (而非 --force)
- **silent no-op 禁用**: 不允许"假成功"的 push
- **诚实诊断**: 冲突通知可能是过时信息, 必须 `git rev-parse` 验证

### 3. 冲突处理 3 步骤

```bash
# Step 1: 验证实际状态
git fetch integration-worktree
git rev-parse integration-worktree/team/e8de47ae-.../integration
git rev-parse HEAD

# Step 2: 若 divergence, merge (而非 rebase --force)
git merge integration-worktree/team/e8de47ae-.../integration

# Step 3: 重新 push
git push integration-worktree rebase/d7d8-into-integration:team/e8de47ae-.../integration
```

### 4. Auto-resolve 阈值

| 情况 | 行为 | 原因 |
|------|------|------|
| 工作已在 tip (0 diff) | team_complete_task | 任务实际完成 |
| 工作 ahead, 远端未跟进 | team_complete_task + 提供 evidence | 诚实登记 |
| 工作 behind 远端 | merge + push + team_complete_task | V23 fail-forward |
| 冲突文件 untracked | 删除本地副本 + merge + push | 远端文件权威 |

## 取舍 (Consequences)

**优点**:
1. 单分支流程避免交叉污染
2. 显式 refspec 杜绝误推
3. V23 fail-forward 强制正确性
4. Auto-resolve 减少无意义重派

**缺点**:
1. 每次 push 需 fetch (少量开销)
2. merge 而非 rebase 可能产生额外 merge commit (但保留完整历史)

## 守门 (Guardrails)

- 不允许 `git push --force` (除非 Leader 明确授权)
- 不允许 silent no-op (即使"工作已完成")
- 冲突通知必须用 `git rev-parse` 验证 (不信任系统提示)
- 7 项不修改承诺继续遵守 (V28.0 状态头追加是补充式)

## 验证 (Verification)

- round10-11 force-push stuck commits: 0 diff 已落地
- round12-08 V28.0 状态头追加: merge 0 conflict, push 成功
- round99 master audit 报告: 已 push 到 tip
- 1563 tests passed / 0 build error (V28.0 验证)

## 后续 (Follow-ups)

- 在 `CONTRIBUTING.md` 添加 V23 fail-forward 流程图
- CI 添加 "禁止 silent no-op" 检查
- round13-XX 派活若涉及 multi-worktree, 必须先更新本 ADR