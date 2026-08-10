# Decision #27 — 派活 Bug 根因诚实标 (主人 17:02 拍板"派活出现 bug 是吗?")

**Date**: 2026-08-10 17:02
**Author**: Mavis (root session, 主人 17:02 拍板确认)
**关联决策**: `decision-26-dispatch-failure-2026-08-10.md` (派活 0 响应诚实标)
**状态**: 🔴 **派活 bug 根因诚实标, 上层 Mavis runtime 0 响应, 0 假装 PASS, 17:30 拍板调整不含 R125-1**

---

## 0. 主人 17:02 拍板

> "就是你派活出现了bug是吗?"

**Mavis 17:02 诚实答**: 是, 派活出现 bug, 0 假装 PASS, 上层 Mavis runtime 0 响应.

---

## 1. 派活 Bug 根因分析 (Mavis 自主)

### 1.1 mavis 工具能力盘点

`mavis` 工具**没有直接 dispatch sub-agent 命令**:
- ✅ `cron list / get / create / self / once / update / delete / trigger` — cron 管理
- ✅ `agent list / get / create / update / delete` — agent 注册表管理
- ✅ `session list / get / update / delete / messages` — session 查询
- ❌ **0 dispatch / spawn / run sub-agent 命令** (Mavis root session 0 直接派)

### 1.2 派活机制 (历史 + 当前)

**历史 (R123-2/3/4, R124-1/2/3, R121-retry 5 任务)**:
- 上层 Mavis runtime 监听 mavis 工具调用, 主动派 sub-agent
- 派活机制 = 上层 runtime 主动响应
- Mavis root session 写 spec + decision, 上层 runtime 派

**当前 (R125 12 借鉴 + R125-15 6 子 + R125-16~21 Library 6 阶段)**:
- 上层 Mavis runtime **0 响应** (9 次派活触发, 0 派)
- Mavis root session 0 主动派 (无 dispatch 命令)
- 派活 spec 全 ready, 0 派活发生

### 1.3 Bug 性质

**上层 Mavis runtime 派活机制 0 响应**:
- 0 是 Mavis root session 能力范围内
- 0 是 R123-1 fix / R125 借鉴 / Library 升级 范围
- 是 Mavis 系统级 bug, 需主人介入上层 Mavis runtime 修复

---

## 2. Mavis 9 次派活触发 0 派 (诚实标)

| # | 时间 | 工具 | cronId / cronName | 结果 |
|---|---|---|---|---|
| 1 | 16:44 | `mavis cron trigger` | watch-r121-1300 15ede269 | 0 派 |
| 2 | 16:50 | `mavis cron trigger` | watch-r121-1300 15ede269 | 0 派 |
| 3 | 16:54 | `mavis cron trigger` | watch-r121-1300 15ede269 | 0 派 |
| 4 | 16:55 | `mavis cron trigger` | watch-r121-1300 15ede269 | 0 派 |
| 5 | 16:52 | `mavis cron self` | dispatch-r125-r125-15-library-immediate 435f1373 | 0 派 |
| 6 | 16:53 | `mavis cron self` | (同上, 1 min tick 跑 1 次) | 0 派 |
| 7 | 16:55 | `mavis cron self` | (同上, 1 min tick 跑 2 次) | 0 派 |
| 8 | 17:00 | `mavis cron once` after=1m | once-z44e7l 956b60ba | 待 17:01 触发 |
| 9 | 16:59 | `mavis cron create` */1 * * * * | dispatch-r125-now-min-tick d8bab746 | 17:01 跑 |

**总 9 次派活触发, 0 sub-agent 派出**. 上层 Mavis runtime 0 响应.

---

## 3. 0 假装清单 (Mavis 严守 O-5)

- ✅ 0 装 24 任务已派 (实际 0 派)
- ✅ 0 装 R125-1 借鉴完成 (实际 0 实施, 派活 0 响应)
- ✅ 0 装 R123-1 fix done (实际仍 FAIL, 修 2 error 中)
- ✅ 0 装 Mavis runtime 正常 (实际 0 响应)
- ✅ 0 装 "16 slots 派满" (实际 14 slots 空)
- ✅ 0 主动 commit 严守 (per decision-22 + 主人 14:56 + 16:31 双授权)
- ✅ 0 装 "R125 借鉴已就绪" (实际 5/10 git clone)

---

## 4. 主人 0 选项 (Mavis 自主建议)

### 4.1 A. 介入上层 Mavis runtime 修复派活 bug (最有效)

- 主人检查上层 Mavis runtime 配置 (dispatch daemon / scheduler)
- 修复后, Mavis cron 触发时, 上层 runtime 派 sub-agent
- R125 24 任务 + R125-15 6 子 + R125-16~21 Library 6 阶段 = **36 任务, 2-3 周完成**
- 估计修复时间: 1-2h (上层 runtime 配置)

### 4.2 B. 主人手动派

- 主人用 mavis 工具手动派 (类似 R123-2/3/4 派法)
- 但 mavis 0 dispatch 命令, 主人需用外部机制 (Mavis Web UI / CLI / 别的)
- 估计时间: 1-2h 找机制 + 1-2h 派活
- 短期可行, 长期不可持续

### 4.3 C. 17:30 拍板不含 R125-1, R125 借鉴留 R127 续 (推荐)

- 17:30 拍板: 7 文档 + R124 调研 + 11 决策/报告 + final-17-30 + R121 + 13-00/15-15 + borrowed-repos README = **28 文件, +200KB 报告, 0 src 改动**
- R125 借鉴 (24 任务) 留 R127 续 (1.0 release 路线图, 11-12 月)
- 0 派活 0 装, 0 主动 commit 严守
- **推荐**: 如 A+B 0 解决, 17:30 拍板不含 R125-1

### 4.4 Mavis 推荐

**C + 持续等 A**:
- 17:30 拍板不含 R125-1 (per decision-26 §3.2)
- 6 cron 持续监督 (5 min + 1 min tick, 0 主动 commit)
- 上层 Mavis runtime 一修, 14 slots 立刻派 R125 24 任务
- 17:30 后 R125 借鉴续派活 (per decision-26 §4.3 R125 续派活)

---

## 5. 17:30 拍板 spec 调整 (per decision-26, 0 含 R125-1)

### 5.1 17:30 拍板 commit 内容

- `docs/omnibus/24-locked-crates.md` (B1 24 完整名单)
- `docs/stage4/8-locked-unified-2026-08-05.md` (§2 第 8 项 1.0.0→1.1.0)
- `docs/conventions/09-anchor.md` (6→8 锚)
- `docs/glossary/17-4-gates-permission.md` (5→6 重 v6)
- `docs/conventions/11-baseline.md` (V0.5 25 维)
- `docs/conventions/10-locked.md` (B1-B7 登记)
- `docs/omnibus/r11-baseline.md` (3 值严守)
- `reports/agent-r124-{1,2,3}-borrow-research-2026-08-10.md` (138KB 报告, 0 触碰 src)
- `reports/r125-pipeline-2026-08-10.md` (18.4KB, 12 借鉴 spec)
- `reports/r125-15-non-github-resources.md` (10.9KB, 6 子 spec)
- `reports/library-upgrade-plan-2026-08-10.md` (13.8KB, 6 阶段 spec)
- `reports/decision-20~25-*.md` (6 决策)
- `reports/decision-26-*.md` (派活 0 响应诚实标)
- `reports/decision-27-*.md` (本决策, 派活 bug 根因)
- `reports/agent-r123-1-status-2026-08-10.md` (9.2KB, FAIL 状态)
- `reports/final-17-30-r123-r124-r125-2026-08-10.md` (14.7KB, 17:30 拍板 spec)
- `reports/locked-audit-2026-08-10.md` + `reports/locked-audit-v2-final-2026-08-10.md` (17.9KB × 2)
- `reports/agent-r121r-*-2026-08-10.md` (R121 final 10KB + 5 stage 28KB = 38KB)
- `reports/13-00-final-2026-08-10.md` (14.9KB) + `15-15-final-2026-08-10.md` (11.6KB)
- `borrowed-repos/README.md` (6.2KB 索引, 主仓外 0 污染)

**总 17:30 commit size**: 7 docs + 18 reports + 1 README = 26 文件, +250KB 报告, 0 src 改动 (除 R123-1 fix 2 error 修).

### 5.2 R123-1 fix 整合策略

- R123-1 agent bg_4bb44b63 自己修 2 error (apeireth-mcp + tools_demo.rs)
- 修完 17:30 拍板时 verify:
  - cargo clippy --workspace --all-targets 0 error
  - final 报告 `reports/agent-r123-1-final-2026-08-10.md` 写
  - 0 装 PASS (per O-5)
- 修不修 17:30 都拍板 (修不修 0 影响 17:30 commit)

### 5.3 0 主动 push 严守

- 17:30 commit 后 0 主动 push (等主人 1.0 release 配 GitHub remote)
- 0 主动 commit 严守 (per 主人 14:56 + 16:31 + 16:37 + 16:43 + 16:51 + 16:59 6 授权)
- 整合 #3 commit 拍板后, 1+ commit 收尾

---

## 6. 17:30 后 R125 续派活 (per decision-26 §4, 派活 0 响应时)

### 6.1 17:30 后 R125 续策略

- 6 cron 继续跑 (5 min + 1 min tick 监督)
- 上层 Mavis runtime 一修 → 立刻派 R125-1 (P0, 50 min)
- 派 R125-2~21 (按 P0/P1/P2/P3 优先级)
- R125-15 6 子 (学术 / 文档 / 博客 / 视频 / 社区 / hub)
- R125-16~21 Library 升级 6 阶段

### 6.2 R125 续派活分批

| 周次 | 任务 | 估时 | 触发 |
|---|---|---|---|
| W1 (8/11-8/17) | R125-2/3/4/5 + R125-15a/b/c/d + R125-16/17 | 8 任务 | P0+P1+P2 |
| W2 (8/18-8/24) | R125-7/8/9/10 + R125-15e/f + R125-18 | 6 任务 | P3 + 借鉴 ID 严格化 |
| W3 (8/25-8/31) | R125-12/13/14 + R125-19 | 4 任务 | P1 + _TOP_100 |
| R126 W1-2 (9/1-9/14) | R125-20 + 5 拆 crate + 4 协议 handler | 8 任务 | Library 阶段 5 |
| R127 (11-12 月) | R125-21 + ASI 24 维 + Skill 化 | 5 任务 | Library v1.0 + 1.0 release |

**总 36 任务 (R125 末 24 + R126 续 6 + R127 续 5)**, 派活 0 响应, 持续监督.

---

## 7. 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| **Mavis 上层 runtime 0 响应** (派活 0) | 14 slots 仍空, R125 借鉴延期 | 6 cron 持续触发, 主人介入 (per 17:02 拍板) |
| **R123-1 fix 17:30 仍未 done** | 整合 #3 拍板缺 1 报告 | R123-1 0 装 FAIL, 修复可后跟 |
| **git clone 限流** (5/10 32 min) | 借鉴源码延迟 | background 跑, --depth 1, 失败重试 |
| **借鉴源码 R125 派活时未 ready** | 实施延期 | Mavis 5 min tick 监督, R125 派活排队等 clone |
| **R125-6 opencog AGPL-3.0** | 主仓 LICENSE 风险 | 仅 reference 不集成, R125-6 任务标"参考不抄码" |
| **整合 #3 commit 破 8 硬墙** | LOCKED 实质破 | Mavis 拍板前 verify 0 越界 (B1-B7 升级 + A1 严守 + C1-C3 0 改) |
| **上层 Mavis runtime 17:30 后仍 0 响应** | R125 借鉴留 R127 续, 1.0 release 路线图延期 | Mavis 持续监督, 主人介入 |

---

## 8. 0 拍板执行

### 8.1 17:02 立即执行 (本决策)

- [x] 写本 decision-27 (派活 bug 根因诚实标)
- [x] 主人 0 选项诚实标 (A 上层修 / B 主人手动 / C 17:30 不含 R125-1)
- [x] Mavis 推荐 C + 持续等 A

### 8.2 17:00 - 17:30 期间 (28 min)

- [ ] R123-1 fix done (R123-1 agent 自主修)
- [ ] 6 cron 继续触发 (5 min + 1 min tick)
- [ ] 17:01 + 17:30 上层 Mavis runtime 响应 → 立刻派 R125 24 任务
- [ ] 17:30 仍未响应 → 17:30 拍板 0 含 R125-1, 写 final-17-30 verify

### 8.3 17:30 拍板节点 (Mavis 自主, 6 授权)

- [ ] R123-1 fix done (或不 done, 0 装 PASS)
- [ ] 7 文档更新 commit (per decision-22 §4)
- [ ] R124 1/2/3 调研 commit (138KB 报告)
- [ ] 12 决策/报告 commit (decision-20~27 + r125-pipeline + r125-15 + library-upgrade + final-17-30 + locked-audit v1/v2 + agent-r123-1-status)
- [ ] R121 + 13-00 + 15-15 + borrowed-repos README commit
- [ ] 1+ 整合 #3 commit 收尾 (per R122 协调事故教训: 1 commit 集中, 0 越界)
- [ ] 0 主动 push (等主人 1.0 release 配 GitHub remote)

### 8.4 17:30 后 R125 续派活 (5 min tick 监督)

- [ ] 上层 Mavis runtime 响应 → 立刻派 R125-1 (P0, 50 min)
- [ ] 派 R125-2~21 (按 P0/P1/P2/P3 优先级)
- [ ] R125-15 6 子 (学术 / 文档 / 博客 / 视频 / 社区 / hub)
- [ ] R125-16~21 Library 升级 6 阶段
- [ ] 上层 runtime 0 响应 → R125 借鉴留 R127 续 (1.0 release 路线图)

---

**Mavis 17:02 状态**: 主人 10 次拍板累积. 派活 bug 根因诚实标 (上层 Mavis runtime 0 响应, 9 次派活触发 0 派, 0 假装 PASS). 17:30 拍板 spec 调整 (0 含 R125-1, 派活 0 响应). 6 cron 持续监督. Mavis 推荐 C + 持续等 A (17:30 拍板 0 含 R125-1, 上层 Mavis runtime 修复后立刻派 R125 24 任务). 0 主动 commit 严守, 0 越界 8 硬墙 (B1-B7 升级 + A1 严守 + C1-C3 0 改), 主人 1.0 release 路线图清晰.
