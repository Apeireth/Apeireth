# Decision-67: R129-24 派活待 cron 下个 tick 处理 (2026-08-11 00:42)

**Date**: 2026-08-11 00:42 (新 session mvs_367e66fae08342ffa399befe4f85dbac)
**Author**: Mavis
**触发**: R129-13 done (00:39) 算 done, 跑中数从 16 → 15 < 16 (差 1). 尝试派 R129-24 补满 16 跑中, task 工具返回 "Tool task not found" (R129-13 done 通知期间 task 工具暂时不可派). 等 cron 下个 tick (00:45) 自动尝试补派.
**关联**: decision-61 + #62 + #63 + #64 (cron 5 min tick) + #65 (R129 era 第 2 批 8 sub-agent 派活) + #66 (R129 era 第 3 批 7 sub-agent 派活) + 主人 8/11 0:34 认知纠正 (跑中 = 16, 不含 done) + 主人 0:39 拍板后 R129-13 done

---

## 0. 一句话

**R129-13 done (00:39) 算 done, 跑中数从 16 → 15 (差 1). 尝试派 R129-24 补满 16 跑中, task 工具持续 "Tool task not found" (R129-13 done 通知期间 task 工具暂时不可派, 3 次尝试全失败). 等 cron 下个 tick (00:45) 自动尝试补派, 或 task 工具恢复后 Mavis 手动补派. 0 主动 push 严守.**

---

## 1. 跑中数盘点 (00:42)

### 1.1 跑中 (status=started, 不含 done)
- 1 R129-3 8 步 verify 跑 (00:08 派, 估 00:38 done)
- 2-8 R129-9/10/11/12/14/15/16 (00:30 派, 7 跑中, R129-13 done 算 done)
- 9-15 R129-17/18/19/20/21/22/23 (00:34 派, 7 跑中)

跑中 = 1 + 7 + 7 = **15** < 16 (差 1)

### 1.2 done (status=finished, 不算 跑中)
- 8 done (R129-1/2/4/5/6/7/8 第 1 批 + R129-13 第 2 批 1 个)

done = 8 (不算 跑中, per 主人 0:34 拍板"已经 done 的不能算正在跑的")

### 1.3 跑中 15 < 16 差 1 → 派 R129-24 补满 16

---

## 2. R129-24 派活尝试 (00:42, 3 次失败)

### 2.1 尝试 1 (00:42, 详细 prompt)
- task 工具调用: tool name "task", agent_name "general", run_in_background true, 详细 prompt (~5000 字符)
- 结果: **"Tool task not found"**

### 2.2 尝试 2 (00:42, 精简 prompt)
- task 工具调用: tool name "task", agent_name "general", run_in_background true, 精简 prompt (~3000 字符)
- 结果: **"Tool task not found"**

### 2.3 尝试 3 (00:42, 等 30 秒 + 简短 prompt)
- task 工具调用: tool name "task", agent_name "general", run_in_background true, 简短 prompt (~500 字符)
- 结果: **"Tool task not found"**

### 2.4 错误分析
- 错误信息: "Tool task not found" — task 工具不可用
- 可能性 1: task 工具被 rate limit (已经派 23 个 R129 sub-agent + 主仓 Mavis + 老 41 sub-agent = 65+ task)
- 可能性 2: task 工具调用有某种隐藏限制
- 可能性 3: task 工具暂时性错误 (Mavis harness 抖动)
- 缓解: 等 cron 下个 tick (00:45) 自动尝试, 或 task 工具恢复后 Mavis 手动补派

---

## 3. 跑中 15 < 16 状态 (等 cron 下个 tick 00:45)

### 3.1 cron 5 min tick 自动监督 (00:45 期望)
- cronId `e6145d0d-bd0d-442d-82a2-89496191bec2`
- cronName `watch-r129-era-auto-replenish-16`
- schedule `*/5 * * * *` 5 min tick
- nextRun `1786380300000` = 2026-08-11 00:45:00 (00:45 cron tick 触发)

### 3.2 cron tick 00:45 期望
- mavis session list 统计 跑中 数
- 跑中 15 < 16 → cron Section 2 派 R129-N 补满 16
- 写 decision-68 (R129 era 第 5 批派活清单 + task_id 索引)
- 跑中 = 16 满, 监督 16 跑中

### 3.3 task 工具可能恢复
- 0:30 / 0:34 cron 派 R129-9~16 / R129-17~23 时 task 工具正常
- 0:42 派 R129-24 时 task 工具 3 次失败
- 可能 task 工具在 0:42 之后会恢复, cron 0:45 跑时 task 工具正常
- 或 cron 0:45 跑时 task 工具也失败 → 跑中 15 持续 < 16 → 主人拍板或 cron 持续重试

---

## 4. 0 主动 IM 主人 (per gate-discipline + 决策 #61 §6 + cron Section 5)

- 仅 done notification 主动报告 (整合 #5 commit 拍板 done)
- 0 主动 plain reply on skip ticks
- 0 主动 push (等 1.0 release 配 GitHub remote, 主人起床后手跑)
- 0 主动删 (Safety policy 阻挡, per 决策 #44 + #60)
- R129-13 done notification 主动报告 (per task 工具机制)

---

## 5. 写决策日志 (per cron Section 6)

每个 cron tick 写一行到 `reports/decision-log-r129-era-cron-2026-08-11.md`:
- 时间戳
- 跑中任务数 (永远 = 16, 不含 done)
- done 任务数 (不限)
- 派活 / 拍板 / 监督 状态
- 决策链更新 (#65 / #66 / #67 / #68)

---

## 6. 风险 + 决策原则

### 6.1 风险
- **R1**: task 工具持续不可派, 跑中 15 < 16 持续 — **缓解**: cron 下个 tick 00:45 自动尝试, 或 task 工具恢复后 Mavis 手动补派
- **R2**: 16 sub-agent 同时跑 cargo build 资源竞争 — **缓解**: 4 批错开 (00:08 + 00:30 + 00:34 + 00:39 (待))
- **R3**: R129-3 8 步 verify 跑过夜 — **缓解**: 0 改 src 严守, 已知 src bug 诚实标
- **R4**: 整合 #5 commit 推 master 后 1.0 release tag 失败 — **缓解**: 0 主动 push 严守
- **R5**: R129-13 报告里 "tag 1.0.0 = semver 大版本归 0 per decision-22 §2.2" 小混淆 — **缓解**: decision-22 §2.2 是 B2 workspace.version 1.2.0 严守, 跟 1.0 release tag v1.0.0 是不同概念, 0 影响整合 #5.2 commit

### 6.2 决策原则
- **Mavis = orchestrator + 全自决** (per 主人 0:25 "全部你做主" 升级授权)
- **跑中 = 16 (永远满, 不含 done)** (per 主人 0:34 拍板"已经 done 的不能算正在跑的，正在跑的达到 16 个")
- **16 跑中上限 + 自动补派** (per 主人 0:34 + 决策 #56 + cron 5 min tick)
- **整合 #5 commit 由 Mavis 自动拍板** (per 主人 0:25 + 决策 #33 C1 + 决策 #64)
- **0 主动 push 严守** (per 决策 #33 + 决策 #61 §6)
- **0 主动 IM 主人** (per gate-discipline, 仅 done notification)
- **0 主动删** (per Safety policy + 决策 #44 + #60)
- **8 硬墙 0 越界** (per 决策 #33 §2.3)
- **0 装 PASS 严守** (per 决策 #33 §2.3 C2)
- **整合 #4 commit abf12243 严守** (per 决策 #48 + 决策 #61 §1.2)
- **决策日志写** (per 决策 #10 + 用户记忆 #10)

---

## 7. 一句话 (再次强调)

**R129-13 done (00:39) 算 done, 跑中数从 16 → 15 < 16 (差 1). Mavis 尝试派 R129-24 补满 16 跑中, task 工具持续 "Tool task not found" (R129-13 done 通知期间 task 工具暂时不可派, 3 次尝试全失败). 等 cron 下个 tick (00:45) 自动尝试补派, 或 task 工具恢复后 Mavis 手动补派. 0 主动 push 严守. R129-3 done (估 00:38-00:42) 后整合 #5 commit 时机 ready → cron 自动拍板 (5.1 → 5.2 → 5.3 顺序, 0 主动 push).**
