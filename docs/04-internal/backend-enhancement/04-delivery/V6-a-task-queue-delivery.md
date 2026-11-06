# V6-a 任务编排持久队列 · 交付实录（持久任务队列 + 幂等唤醒）

> `Status: ✅ 已交付（2026-10-17，V6-a 批）` ｜ 战场：`crates/foundation/orchestration`（新件 `task_queue.rs`）+ runtime-assembly 装配 + CLI 面
> 出处：哲学研究治理族机制卡（`docs/04-internal/philosophy-study/03-governance-orchestration-family.md` §5-①/§2-③ + TOP3）假说改造；补「任务编排未建」空位。
> 来源纪律：中性表述、扫描零命中、不写 CHANGELOG、不 commit。

## 0. 一句话

跨重启可审计的任务交接一次解决：**持久任务队列**（状态机六态全显式）+ **幂等唤醒**（assign 即唤醒、同键重复投递单执行）+ **崩溃安全**（重启恢复 running→pending + attempt 计数防死循环）+ **failed 只能人判**（失败绝不自动重试）+ **环路熔断**（任务链深度 N 层拒绝即帧 + 审计）+ **结算接账**（一回合一段、只接账不重复记）+ 工具面 `tool.task_board`（增/列/改派/完成）与 `tool.self_status` 任务节。

## 1. 落点清单（设计 5 项 → 文件）

| 设计项 | 落点 | 语义 |
|---|---|---|
| 1 持久任务队列 + 崩溃安全 | `crates/foundation/orchestration/src/task_queue.rs`（队列核：状态机/幂等/唤醒/环路/结算）+ `crates/engine/runtime-assembly/src/sqlite_task_queue.rs`（sqlite 任务表，**与会话账本同库**） | `TaskStore` 端口 = 一次 update 一次原子读改写；`TaskQueue::open` 即崩溃恢复点 |
| 2 幂等唤醒 | `task_queue.rs`：`idempotency_key` 去重（任何状态命中 = 不重复执行 + `redelivered` 计数 + 审计）、assign 即唤醒（唤醒信号与状态事件**同汇** `TaskEvent`）、未投递唤醒跨重启重投 | failed 唯一入口 `human_judge_fail`（人判，`HumanOnly` 拒 agent）；重试唯一入口 `human_judge_retry`（显式动作，防雪崩） |
| 3 Turn 结算挂任务账 | `task_queue.rs::link_settlement`（[`TaskSettlement`] 只存轮次 + 账本引用，**无金额字段**） | 一回合一段、重复挂账不重复记；「算不算花过钱」沿三维账本既有口径（`work_consumption` 事件日志单趟折叠，取消/中断同答案）——只接账不重复记 |
| 4 环路检测 | `task_queue.rs`：`TaskSpec::parent_task_id` 任务链 + `TaskQueueConfig::max_chain_depth` 熔断 | 超深入队拒绝即帧 + `ChainRefused` 审计（涌现级失控在结构上不可能） |
| 5 工具面 + 自述节 | `crates/engine/runtime-assembly/src/canonical/task_board_tool.rs`（`tool.task_board` + `TaskBoardApprovalHook`）+ `crates/capabilities/tools/src/self_status.rs`（任务节） | 增/列/改派/完成四动作闭合词表（**无 failed/retry 面**）；自述任务节 = 分态计数真值（同队同账） |

**接缝诚实声明**：Turn 生命周期的自动挂账（回合结算处自动调 `link_settlement`）**未接线** —— 本批交付的是「接账」语义面（挂账幂等、无重复记账、金额只从三维账本读）与测试证据；自动挂账归口回合结算面（只接账不重复记的接缝已备好）。

## 2. 队列状态机图（六态，全部转移显式）

```text
                    ┌─────────────── 人判 fail (human_judge_fail)  ←──────┐
                    ↓                                                    │
  enqueue ──→ Pending ──assign(唤醒)──→ Assigned ──begin──→ Running      │
    (可带 assignee=Assigned)   ↑           │                  │  │        │
                    │          │           │ block            │  │complete│
                    │          │           ↓                  │  ↓        │
                    │          └──assign── Blocked ←──block───┘  Done    │
                    │                          │                          │
                    │   崩溃恢复 (open):       │ 人判 retry               │
                    └── running/assigned ──────┤ (human_judge_retry)      │
                        超 max_attempts        ↓                          │
                        ──→ Blocked      Pending ─────────────────────────┘
                                                    Failed (只能人判)
```

- `Pending` 入队待派 → `Assigned` 已指派并已唤醒 → `Running` 执行段在跑 → `Done` 完成（终态）；
- `Failed` **只能人判**进入（agent 无工具面、无 API 路径），进入后**不自动重试**：崩溃恢复不触碰 `Failed`，回 `Pending` 的唯一路径是显式人判 `human_judge_retry`；
- `Blocked` 外部等待/重投递熔断（崩溃恢复超 `max_attempts` 的不再回队，防崩溃死循环）；
- 非法转移一律拒绝即帧（`InvalidTransition`），无隐式回退。

**崩溃恢复账目**（`attempts` / `redelivered` 分工）：

| 计数 | +1 时机 | 防什么 |
|---|---|---|
| `attempts` | 进入 Running（执行段开始）+ 崩溃恢复重派 | 崩溃死循环（超 `max_attempts` 熔断为 Blocked + `AttemptFused` 审计） |
| `redelivered` | 同键重复投递 + 未投递唤醒跨重启重投 | 重复执行（同 key 单执行）/ 唤醒丢失（不丢） |

## 3. 工具面与治理档（任务工具治理档）

| 动作 | 语义 | 治理档 |
|---|---|---|
| `task_add` | 入队（可带幂等键 / 初始执行者 / 父任务） | **RequireApproval**（写面 = 写文件，write_intent 同族；无自动放行档） |
| `task_list` | 列任务（只读自家任务态，无外部内容注入面） | **Allow**（零审批，与 `tool.self_status` 同类只读面） |
| `task_reassign` | 指派/改派（assign 即唤醒） | **RequireApproval**（同上） |
| `task_complete` | 自报完成 | **RequireApproval**（同上）+ 认领授权：`caller == assignee`（委托者不能伪造工人状态），否则拒绝即帧 |
| （无） | failed / retry | **无工具面** —— failed 只能人判，重试是显式人判动作（防雪崩） |

- 动作缺失/未知 → fail-closed 停审批（不静默放行）；非法参数拒绝即帧（不进审批）。
- 档位唯一承载：`TaskBoardApprovalHook`（治理链，与本地审批面板 / IM 审批卡同一条审批链）；CLI 面 `APEIRETH_ENABLE_TASK_BOARD=1` = 注册 + grant + 钩子 + 队列装配（默认关 fail-closed，注册即闸）。
- 自述任务节（`tool.self_status`）：`task_board` 节 = pending/assigned/running/done/failed/blocked **计数真值**（与工具面同队同账），探测口缺位 = 全维显式 null + 原因（0 装诚实）。

## 4. 测试证据（本批实测值 · 2026-10-17 · rustc 1.97.1 / cargo test --locked -j 2）

**新增 27 条，全绿**（分布 4 个套件）：

| 套件 | 条数 | 钉住的行为 |
|---|---|---|
| `crates/foundation/orchestration/tests/task_queue.rs` | 13 | 状态机全转移显式/非法拒绝即帧；重启恢复 running→pending+attempt；同 key 双投递单执行；failed 人判（agent 碰不到人判面、失败不自动重试、重试显式）；环路熔断（深度 N 拒 + 审计）；结算挂账幂等无重复记账（金额只从三维账本读）；事件唤醒同汇；唤醒跨重启重投；并发入队原子（8 手 ×8 条零丢失）；自报必须是本人；分态计数真值；派发熔断防崩溃死循环；入参校验拒绝即帧 |
| `crates/engine/runtime-assembly/tests/task_queue_store.rs` | 5 | **真 sqlite 落盘**（会话账本同库）：重开恢复 running→pending+attempt；双句柄同库并发入队原子（20 条零丢失）；未投递唤醒跨重启重投；结算跨重启不重复；坏入参不落脏行 |
| `crates/engine/runtime-assembly/tests/task_board_tool.rs` | 6 | **任务工具治理档**（写面停审批 / 只读列零审批 / 未知 fail-closed / 不越权误伤）；工具面无 failed/retry（failed 只能人判）；增/列/改派/完成全回路（含自报认领授权拒绝）；同 key 双投递去重单任务；自述任务节计数真值；注册条件 + 名册行照实 |
| `crates/adapters/cli/tests/task_board_knob.rs` | 2 | 旋钮默认关 fail-closed（不在授权面）；开 = 授权 + 写面审批档 + 只读列放行 |
| `crates/capabilities/tools/src/self_status.rs`（单元） | 1 | 任务节计数真值 + 探测缺位显式 null + 原因 |

回归基线：既有套件零回归以收尾门禁实测为准（见 §5；单一真源 `docs/03-reference/test-baseline.md` 不代改）。

## 5. 校验证据（宪章门禁，`-j 2` 口径 · 2026-10-17 · 本批实测值）

| 门禁 | 命令 | 实测 |
|---|---|---|
| 测试 | `cargo test --workspace --all-targets --locked --no-fail-fast -j 2` | **207 套件 / 5605 passed / 0 failed**（既有套件零回归；单一真源 `docs/03-reference/test-baseline.md` 不代改，本行为本批实测值+日期） |
| clippy | `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` | **exit 0（0 警告）** |
| fmt | `cargo fmt --all -- --check` | **0 diff** |
| 中性扫描 | `scripts/check-neutral-terms.ps1` | **零命中**（「OK 76 changed files: added lines all neutral」） |

共享工作区同场有多条在途线（关怀规则/通用嗓/备份导出/人格层等），上表为**全工作区合流实测**：本批 4 个测试二进制（`task_queue` / `task_queue_store` / `task_board_tool` / `task_board_knob`）与他线套件同场全绿。语义清单：0 触碰 LOCKED 5 项、0 引新外部 dep、未写 CHANGELOG、未 commit。

## 6. 触碰面报备（战场外最小改动，纪律 §六.3）

| 文件 | 触碰 | 理由 |
|---|---|---|
| `crates/capabilities/tools/src/self_status.rs` + `src/lib.rs` | 任务节增量（`TaskBoardStatus` + 快照 2 字段 + 渲染 + schema 一节 + 1 测试 + 1 导出） | 「自述节」验收必经；最小增量，照 care/backup 节同款模式 |
| （未动）`crates/engine/guard/src/semantics.rs` 治理语义面 | 0 触碰 | 任务工具治理档由审批钩子唯一承载（与同批 care/voice 工具同口径）；未入规范描述符名册的工具走 fallback 启发式描述符（`known: false` 如实，无谎报） |

工具面 `tool.task_board` 落 runtime-assembly（战场内）而非工具 crate：`apeireth-plugin` 依赖 `apeireth-orchestration`（反向则成环），工具面随装配层承载（组织域工具同款思路）。队列核在 `crates/foundation/orchestration` 新件 `task_queue.rs`（本批 orchestration 仅此一件新文件 + `lib.rs` 注册两行）。
