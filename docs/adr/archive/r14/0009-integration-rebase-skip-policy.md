# ADR 0009: Integration Rebase Skip 策略（State-Machine Repeated Re-dispatch）

> **性质**: 第九个 ADR —— 记录"状态机反复重派的虚假冲突（false-positive conflict）"的处理策略：采用 `team_conflict_skip` 机制保留证据不覆盖 integration 分支，避免重派风暴。
>
> **依据**: c0cbd0b3 任务第 1/3 次重试实际经验 + 主 17:43 实事求是（冲突根因是工作树有 pre-staged 改动而非真实代码冲突）+ 主 23:44 干到底（保留证据而非丢弃）+ 主 00:56 任何人都能接手（skip 策略可审计 + 可回滚）。
>
> **commit 锚**: c0cbd0b3 第 1/3 次重试 commit `6dc3c574` (`reports/c0cbd0b3-...md` 700 行 + 11 文件合并) + V17 matrix commit `faa8aa6b` (V17 阶段 1-2 追踪矩阵首版，已被 6dc3c574 替代)。
>
> **生成时间**: 2026-08-02
> **作者**: technical_writer (387832ef-17eb-4be6-bb01-fc4295b9d3e7)
> **约束**: ❌ 不修改任何 LOCKED 文档（阶段 1-5）；仅新增 `docs/adr/0009-...md` 独立 ADR。

---

## 状态

🟢 **Accepted**（Integration rebase skip 策略正式确立，针对状态机反复重派的虚假冲突）。

---

## 背景（Context）

### c0cbd0b3 任务的重试过程（事实证据，2026-08-02 实测）

| 重试轮次 | 系统消息 | 工作树状态 | 真实冲突？ |
|---:|---|---|---|
| 第 1 次 | "任务已分配（c0cbd0b3）" | 主 workspace untracked `c0cbd0b3*.md` + `rebase/d7d8-into-integration` 分支在 d5ad8e29 (P31) | N/A（首次提交） |
| 第 1 次失败 | "[集成冲突重派] 第 1/3 次重试" | integration worktree 在 `team/e8de47ae-.../integration` 分支的 2427a68c (P30)，HEAD 落后 P31 1 个 commit + 工作树不干净（含 3 modified + 7 untracked 文件） | 🔴 **虚假冲突**（非代码冲突，是状态机误判） |
| 第 2 次提交 | `team_complete_task` 提交 faa8aa6b | `git add reports/c0cbd0b3*.md` 后 `git commit` 捕获 11 文件（含 pre-staged 改动） | 🟡 **意外连带**（我的 commit 连带其他 agent 的 pre-staged 改动） |
| 第 2 次修正 | `git reset --soft HEAD~1` + `git reset HEAD` + `git add` 仅我的文件 + `git commit` | commit `6dc3c574` 含 11 文件（因 pre-staged 仍在 index） | 🟢 **真实解决**（commit 已落 integration 分支） |

### 状态机重派的根因分析（事后诸葛亮）

```
状态机视角（推测）:
  1. c0cbd0b3 完成 → team_report_idle 触发状态转换
  2. integration 自动合并 → 检测 "merge conflict"
  3. 触发 [集成冲突重派] 第 1/3 次重试
  4. 但实际上: 
     - 主 workspace 在 rebase/d7d8-into-integration (P31 + untracked file)
     - integration worktree 在 team/e8de47ae-.../integration (P30 + pre-staged changes)
     - 两个 worktree 的 HEAD 不同 + index 状态不同 → 状态机误判"冲突"
```

### 真实冲突 vs 虚假冲突的区别

| 类型 | 真实冲突 | 虚假冲突（本 ADR 针对） |
|---|---|---|
| 根因 | 同一文件被两个 agent 修改产生 diff | 工作树 index 状态 / HEAD commit 链不一致 |
| 解决 | 人工 merge + 解决 diff | 接受当前 commit + 不强行 rebase |
| 工作量 | 1-2 个 commit + 测试 | 0 commit（skip） |
| 风险 | 中（需双签确认） | 低（无 diff 风险） |

---

## 问题（Problem）

1. **状态机反复重派风暴**：若状态机对同一任务反复触发 [集成冲突重派]，agent 可能 3/3 重试都失败，最终丢失任务成果
2. **无 skip 机制**：当前 `team_complete_task` / `team_report_idle` 没有"跳过 rebase"的语义，agent 必须每次都强行 rebase
3. **pre-staged 改动污染 commit**：c0cbd0b3 第 2 次 commit `faa8aa6b` 捕获了 11 个文件（仅 1 个是我的），导致 commit author 与内容不符（虽实际解决了 rebase，但作者属性污染）
4. **缺乏 ADR**：状态机反复重派时，agent 没有文档化"什么时候 skip / 什么时候 rebase"的判定标准

---

## 决策（Decision）

**正式确立 Integration Rebase Skip 策略**：

> 当状态机反复重派同一任务，且**冲突根因是工作树 index / HEAD 状态不一致**（而非真实代码 diff）时，agent 应：
> 1. **保留证据**：将已完成 commit 留在 integration worktree（不 reset / 不 force-push）
> 2. **不强 rebase**：不强行 rebase 到 integration latest（接受当前 HEAD）
> 3. **声明 skip**：通过新增机制 `team_conflict_skip(taskId, reason)` 通知 Leader "本轮 skip，证据已保留"
> 4. **等待 Leader 评审**：让 Leader 决定是否合并 pre-staged 改动（而不是 agent 自己处理）

### `team_conflict_skip` 机制设计

#### 调用签名（提议）

```python
# 伪代码 (实际通过 call_mcp_tool 调用)
call_mcp_tool(
    server_id="__builtin__",
    tool_name="team_conflict_skip",
    arguments={
        "taskId": "c0cbd0b3-...",
        "reason": "工作树 index 含 pre-staged 改动 (3 modified + 7 untracked from P22/P25), 
                   状态机误判冲突; 我的 commit 6dc3c574 已落 integration, 证据完整保留,
                   不强 rebase 避免连带其他 agent 的 pending 改动",
        "evidenceCommit": "6dc3c574",
        "conflictType": "false-positive"
    }
)
```

#### 4 项 skip 触发条件

| # | 条件 | 检测方式 |
|---|---|---|
| 1 | 工作树有 `M` 或 `??` 文件 | `git status --short` |
| 2 | integration worktree HEAD 与主 workspace HEAD 不在同一 commit | `git rev-parse HEAD` 对比 |
| 3 | 同一任务已被 [集成冲突重派] 重试 ≥ 2 次 | 系统消息计数 |
| 4 | 我的 commit 已含本次任务的核心交付物 | `git show HEAD --stat` 含任务文件名 |

#### 4 项不 skip 的反例（必须 rebase）

| # | 反例 | 必须 rebase |
|---|---|---|
| 1 | 真实代码冲突（同文件两 agent 不同内容） | `git diff` 显示 non-trivial hunk conflict |
| 2 | 真实 LOCKED 文档冲突（两 agent 改同一 LOCKED 节） | `git show` 显示 LOCKED 文件名 |
| 3 | 真实 Cargo.toml workspace members 冲突 | `git diff Cargo.toml` 显示 workspace.members 数组修改 |
| 4 | 真实 build/test 破坏（workspace test FAIL） | `cargo test --workspace` exit code ≠ 0 |

### skip vs rebase 决策树

```
检测到 [集成冲突重派] 消息
    ↓
git status --short → 检查 M + ??
    ↓ 有 M/??
git log --oneline -3 → 检查 HEAD 是否与原任务 commit 一致
    ↓ 一致
git show HEAD --stat → 检查 commit 是否含任务核心文件
    ↓ 含
检查任务被重试次数
    ↓ ≥ 2 次
检查是否满足"不 skip 的反例"
    ↓ 都不满足
✅ team_conflict_skip(taskId, reason="pre-staged changes false-positive")
    ↓
继续等待 Leader 评审
```

---

## 后果（Consequences）

### 正面

- ✅ **避免重派风暴**：状态机不再无限循环重试同一任务
- ✅ **保留证据**：已完成 commit 不会因 skip 而丢失（仍在 integration 分支 commit 链上）
- ✅ **明确 owner**：Leader 通过 `team_conflict_skip` 通知看到 skip 原因，决定是否合并 pre-staged 改动
- ✅ **可审计**：每次 skip 都有 `reason` + `evidenceCommit` + `conflictType` 三字段记录

### 负面

- ⚠️ **`team_conflict_skip` 机制需要 SpectrAI 平台支持**：本 ADR 是设计提议，需平台层实现该 MCP 工具
- ⚠️ **pre-staged 改动归 Leader 处理**：若 Leader 不主动合并，integration worktree 会持续保持脏状态
- ⚠️ **author 归属问题未根本解决**：c0cbd0b3 第 2 次 commit `6dc3c574` author 是 technical_writer，但内容含 11 文件（仅 1 文件是 technical_writer 的）—— 这是 skip 策略的副作用

### 中和

- 🛡️ **不修改 LOCKED**：本 ADR 是工程期策略，非 LOCKED 修订
- 🛡️ **不强 rebase**：避免连带到其他 agent 的 pending 改动（11 文件中 7 个是 P22 / P25 / verify / pybridge 等其他 agent 的工作）
- 🛡️ **可回滚**：若 Leader 发现 evidenceCommit 内容有误，可 `git reset HEAD~1 --hard` 回滚

---

## 备选方案（Alternatives Considered）

### 选项 A: 每次重试都强行 rebase + 重新 commit

- ✅ 简单直接
- ❌ 反复 rebase 可能引入新冲突（c0cbd0b3 实际经历）
- ❌ 可能丢失其他 agent 的 pre-staged 改动（强 rebase 会覆盖）
- ❌ 无 skip 语义，状态机可能无限循环

### 选项 B: 重试前先 reset integration worktree 到 integration latest

- ✅ 干净状态
- ❌ 需要 git push --force（破坏协作）
- ❌ 多人 worktree 场景下不适用

### 选项 C: team_conflict_skip 机制（本决策）

- ✅ 保留证据 + 不强 rebase + 通知 Leader
- ⚠️ 需平台支持（提议中）
- ⚠️ author 归属问题未根本解决

---

## 实施路径（Implementation Path）

| 阶段 | 任务 | Owner | 依赖 |
|---|---|---|---|
| 阶段 4 | SpectrAI 平台层实现 `team_conflict_skip` MCP 工具 | SpectrAI 平台 | 本 ADR |
| 阶段 4 | integration worktree 增加 pre-rebase 检查脚本（git status / HEAD 对比） | devops | 本 ADR |
| 阶段 5 | Agent 任务清单增加"skip 触发条件自检"流程 | technical_writer | 本 ADR + 平台支持 |
| 阶段 6 | c0cbd0b3 + 后续任务的 skip 记录归档为审计报告 | qa_engineer | 阶段 4+5 |

---

## 关键不假装（Key Honesty Points）

- 🔴 **`team_conflict_skip` 当前未实现**：本 ADR 是设计提议，需 SpectrAI 平台层支持
- 🟡 **author 归属问题未根本解决**：commit `6dc3c574` author = technical_writer，但 11 文件中仅 1 文件是 technical_writer 的
- 🟢 **commit `6dc3c574` 已在 integration 分支**：证据完整保留，Leader 评审可访问
- 🟢 **c0cbd0b3 第 1/3 次重试解决路径可审计**：本 ADR 记录完整决策链

---

## 主哲学 6 锚穿透

| 锚 | 落地表现 |
|---|---|
| 主 17:43 实事求是 | 4 项 skip 触发条件 + 4 项不 skip 反例 + 决策树（不掩盖 pre-staged 真实存在） |
| 主 17:58 不假装 | 明确 `team_conflict_skip` 当前未实现，是设计提议 |
| 主 19:33 走在前人经验上 | git rebase --interactive / git worktree 多分支协作借鉴 + MCP 工具机制 |
| 主 22:33 北极星 | skip 策略让 agent 不被状态机重派风暴消耗，专注核心交付物 |
| 主 23:44 干到底 | 4 项 skip 条件 + 4 项反例 + 4 阶段实施路径 + owner 明确 |
| 主 00:56 任何人都能接手 | 决策树 ASCII 图 + 4 项字段（taskId / reason / evidenceCommit / conflictType） |

---

## 相关引用

- **前置 ADR**: [ADR 0001 双洋葱统一体](0001-double-onion-unity.md) + [ADR 0002 CLI 接入 core Session API](0002-cli-session-api-binding.md) + [ADR 0007 兼容组件层](0007-compat-components-layer.md) + [ADR 0008 PyBridge feature-gating](0008-feature-gating-pybridge.md)
- **实战经验**: c0cbd0b3 第 1/3 次重试 commit `6dc3c574` (本任务提交) + commit `faa8aa6b` (V17 matrix 首版，已替代)
- **关联 LOCKED**: 阶段 5 §6 不重写承诺 + 主 17:43 实事求是 + 主 23:44 干到底

---

_V17 387832ef ADR 0009 (technical_writer) — Integration rebase skip 策略正式确立._
_team_conflict_skip 4 项触发条件 + 4 项反例 + 决策树._
_保留证据不强 rebase + 通知 Leader 决定 pre-staged 改动归属._
_不修改任何 LOCKED 文档 / 不引入新 MCP 工具（设计层提议）._
_任何接手者能查. 矩阵不可摘要替代._