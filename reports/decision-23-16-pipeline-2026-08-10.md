# Decision #23 — 16 派满 + cron 监督 + 少人补上 派活策略

**Date**: 2026-08-10 16:42
**Author**: Mavis (root session, 主人 16:37 拍板"16 成员派满 + cron 监督 + 少人补上")
**关联决策**: `decision-22-master-auth-upgrade-2026-08-10.md` (主人 16:31 最高权限)
**关联报告**: `r125-pipeline-2026-08-10.md` (12 任务 spec) + `upgrade-roadmap-post-r124-2026-08-10.md`
**状态**: ✅ **决策登记 + cron update 完成 + R125 12 任务 spec ready**

---

## 0. 触发事件

**主人 16:37 拍板**:
> "对了, 要干你就多派几个人干, 我这电脑配置强的很, 16个成员是最多且不会卡的上限, 我建议你就是如果任务多就把16个人派满, 并设置cron不断监督, 少人就补上. 这样效率才高."

**主人 5 次拍板累积**:
- 8/10 01:14 R119 形式撤销 locked, 原意保留
- 8/10 01:49 R119-8 3 技术类 LOCKED 撤销
- 8/10 16:27 大胆更新 locked
- 8/10 16:31 最高权限 (Mavis 自主拍板, 24 LOCKED 自主确认)
- **8/10 16:37 16 派满 + cron 监督 + 少人补上** ⭐

---

## 1. 16 派满策略 (Mavis 实施)

### 1.1 当前活跃任务 (16:42)

| task_id | 描述 | 状态 | 截止 |
|---|---|---|---|
| bg_4bb44b63 | R123-1 clippy+doc 清 | running (11 批 clippy) | 17:25 |
| bg_ea620f18 | R124-2 战区 3 调研 | running (报告 47KB 写完) | 17:30 |
| bg_56e2ee14 | git clone Top 10 借鉴 | running (2/10) | 30+ min |
| (待派) R125-1 | LiteLLM Provider Registry | pending | 17:30 |
| (待派) R125-2 | clap derive | pending | 4-6h |
| (待派) R125-3 | hyper 池 | pending | 1 天 |
| (待派) R125-4 | MCP servers | pending | 1-2 天 |
| (待派) R125-5 | NVIDIA Guardrails | pending | 2-3 天 |
| (待派) R125-7 | aGLM PODA | pending | 3-5 天 |
| (待派) R125-8 | Chidori journal | pending | 1 周 |
| (待派) R125-9 | PyO3 | pending | 1-2 天 |
| (待派) R125-10 | Kani 形式化 | pending | 2-3 天 |
| (待派) R125-12 | OpenCode 子代理 | pending | 3-5 天 |
| (待派) R125-13 | LangGraph StateGraph | pending | 1 周 |
| (待派) R125-14 | obra/superpowers | pending | 1-2 天 |

**当前 3 活跃, 13 slots 剩 → 派 12 R125 + 1 备用 = 派满 16**.

### 1.2 派活顺序 (16:42-17:00, 3 批)

| 批 | 时间 | 任务 | 优先级 |
|---|---|---|---|
| **1** | 16:42-16:45 | R125-1 (P0 50min) + R125-5 + R125-10 + R125-12 (P1 locked 升级) | 4 任务 |
| **2** | 16:45-16:50 | R125-2 + R125-4 + R125-13 (P2 高 ROI) | 3 任务 |
| **3** | 16:50-17:00 | R125-3 + R125-7 + R125-8 + R125-9 + R125-14 (P3 中/高 ROI) | 5 任务 |
| **17:00 16 个满** | — | R123-1 (续) + R124-2 (续) + git clone (续) + R125-1 ~ R125-14 (12 任务) = 15 + 1 备用 | 16 满 |

### 1.3 跳过的 R125 任务

- **R125-6 OpenCog Atomspace + ECAN**: AGPL-3.0 ⚠️ 传染风险, 仅 reference 不集成 (R125-6 任务 0 派)
- **R125-11 sqlite-vec 单文件降级**: R120 A 已真接 sqlite-vec (1000 条 p99 1ms, 50x 加速), R125-11 任务是"评估单文件降级路径", 不是新实施 (R125-11 0 派, 留 R126 评估)

---

## 2. cron 监督策略 (per 主人 16:37 "cron 不断监督 + 少人就补上")

### 2.1 watch-r121-1300 cron 已 update (16:42)

**cron_id**: `15ede269-79a3-41af-a74c-de983a29e8b4`
**schedule**: `*/5 * * * *` (每 5 min)
**timezone**: Asia/Shanghai
**新 prompt**: 16 派满 + R125 派活清单 + 5 min auto-check 监督 + 少人补上 + 17:30 整合 #3 拍板

### 2.2 5 min auto-check 流程 (Mavis 实施)

| 步骤 | 动作 |
|---|---|
| 1 | 统计当前活跃任务数 (16 cap 16) |
| 2 | 计算距 16 cap 剩 slots: 16 - 活跃数 |
| 3 | 检查 R125 派活清单进度: 已派 N / 12 任务 |
| 4 | **决策** (派满策略): 剩 slots > 0 且未派完 → 立刻派下一个 R125 (按 P0/P1/P2/P3 优先级) |
| 5 | **决策** (少人补上): 任务 succeeded → 立刻派 replacement; failed → 诊断 + kill + 派替代; canceled → 同样补; 卡 30min → 诊断 + kill + 派替代 |
| 6 | 写入 mavis-progress (本对话 session 上下文) |

### 2.3 0 主动 commit + 0 主动 push (per 主人 14:56 + 16:31 双授权)

- **0 主动 commit**: 主人 14:56 "你拍" 拍板 R125 续自动派, 0 主动 commit
- **0 主动 push**: 等主人 1.0 release 配 GitHub remote
- **Mavis 整合 #3 拍板**: 17:30 节点 1+ 整合 commit 收尾 (per R122 协调事故教训: 1 commit 集中, 0 越界)

---

## 3. 17:30 整合 #3 commit 拍板节点

### 3.1 17:30 节点交付清单

- [ ] R123-1 done (clippy+doc 清)
- [ ] R124-1/2/3 调研 commit (138KB 报告, 0 触碰 src)
- [ ] R125-1 done (provider_registry.rs)
- [ ] 7 文档更新 commit (per decision-22 §4: 24-locked-crates.md 已写, 8-locked-unified §2 第 7/8 项 + 09-anchor 6→8 锚 + 11-baseline V0.5 25 维 + 17-4-gates 5→6 重 + 10-locked B1-B7 + r11-baseline 3 值严守)
- [ ] Top 10 借鉴 git clone done (background `bg_56e2ee14`)
- [ ] borrowed-repos/README.md 索引写完
- [ ] 1+ 整合 #3 commit 收尾
- [ ] final-17-30 报告写完 (`reports/final-17-30-r123-r124-r125-2026-08-10.md`)

### 3.2 整合 #3 commit 拍板 (Mavis 自主)

- 1+ 整合 commit 收尾
- 0 越界 8 硬墙 (per decision-22 §2 严守)
- 0 主动 push (等主人 1.0 release 配 GitHub remote)
- commit msg: 跟 R122 df6dfb69 风格, 简明 + 引用决策 + 报告路径

### 3.3 final-17-30 报告内容

`reports/final-17-30-r123-r124-r125-2026-08-10.md` 涵盖:
- R123-1 done + 1 commit
- R124-1/2/3 调研 commit
- R125-1 实施 + 1 commit
- 7 文档更新 commit
- 整合 #3 收尾 (1+ commits)
- 17:30 后 R125-2 ~ R125-14 派活清单 + 时间表
- 主人决定 R125 续 12 任务 vs 暂停

---

## 4. R125 12 任务 locked 影响 (per decision-22 §2)

| 任务 | 涉及 LOCKED crate | 触发 locked 改动 | 严守要求 |
|---|---|---|---|
| R125-1 | apeireth-pipeline (24 LOCKED) | 0 (R122-5 整合) | 只加 provider_registry.rs, 0 改 lib.rs |
| R125-2 | apeireth-cli (0 LOCKED) | 0 | 全文件重写, 0 越界 |
| R125-3 | apeireth-http-client (0 LOCKED) | 0 | 全文件重写, 0 越界 |
| R125-4 | apeireth-mcp (24 LOCKED) | 0 | 只加 protocol.rs, 0 改 lib.rs |
| R125-5 | apeireth-sovereignty (24 LOCKED) | **B4 + B6** (5→6 重 + 双→三洋葱) | 只加 colang_dsl.rs, 0 改 lib.rs |
| R125-7 | apeireth-evolution (24 LOCKED) | 0 | 只加 poda_cycle.rs, 0 改 lib.rs |
| R125-8 | apeireth-supervisor (24 LOCKED) | 0 | 只加 journal.rs, 0 改 lib.rs |
| R125-9 | apeireth-pybridge (0 LOCKED) | 0 | 全文件重写, 0 越界 |
| R125-10 | apeireth-formal (0 LOCKED) | **B3** (V0.5 24→25 维) | kani_harness.rs 扩, 0 越界 |
| R125-12 | apeireth-tui (0 LOCKED) + 9 organ (LOCKED) | **B7** (9 organ 内部借) + 12 键+1 (PHL-07) | 9 organ 文件名 + 入口签名 0 改, 内部 fn 借 |
| R125-13 | apeireth-graph (24 LOCKED) | **B3** (V0.5 24→30 维) | 只加 state_graph.rs, 0 改 lib.rs |
| R125-14 | apeireth-central (24 LOCKED) | 0 | 只加 skill.rs, 0 改 lib.rs |

**Mavis 严守**: R125 任务只加新 mod + 整合, 0 改 24 LOCKED crate 原 lib.rs (除 R125-12 9 organ 内部 fn 借 OpenCode).

---

## 5. 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| **借鉴源码 git clone 慢/失败** | Top 10 clone 30+ min 风险 | background 跑, --depth 1, 失败 1 次重试, 主仓外 0 污染 |
| **opencog AGPL-3.0 传染** | 主仓 LICENSE 风险 | R125-6 跳过 (AGPL-3.0 ⚠️), 仅 reference 不集成 |
| **R125 派活 task 工具挂** (R122-11 教训) | R125 续阻塞 | Mavis 自干 spec 备 0 阻塞 (本决策 + decision-21/22 已写) |
| **24 LOCKED 中 crate 实施破原 lib.rs** | LOCKED 实质破 | Mavis 严守"R125 任务只加新 mod + 整合, 0 改原 lib.rs" |
| **16 满员卡电脑** | 主人说"配置强不会卡" | 主人已确认 16 上限安全 |
| **协调事故** (R122-2/3/5 14:50-15:00 教训) | 主分支同时改冲突 | git worktree 隔离 (per 之前经验), 主分支 0 同时改 |
| **R125 派活 12 任务同时跑 resource 竞争** | 编译期 cargo 锁冲突 | Mavis 严守 0 同时改同一 crate, 借鉴源码在 borrowed-repos/ 0 冲突 |
| **主人 GitHub remote 未配** | R125 commit 难 | 0 主动 push, 留主人 1.0 release 前配 remote |

---

## 6. 0 拍板执行

### 6.1 16:42 立即执行

- [x] 写本决策 #23
- [x] 改 watch-r121-1300 cron (16 派满 + R125 派活 + 5 min auto-check 监督 + 少人补上)
- [x] 写 R125-pipeline 12 任务 spec 详细
- [ ] Mavis 调度下个 tick 派 R125-1 (P0 50min) + R125-5/10/12 (P1 locked 升级)
- [ ] 17:00 派满 16 (3 续 + 12 R125 + 1 备用)
- [ ] 17:25 R123-1 截止 + 17:30 整合 #3 commit
- [ ] 17:30 写 final-17-30 报告

### 6.2 17:30 节点

- [ ] Mavis 整合 #3 commit 拍板 (1+ commit, 0 越界)
- [ ] final-17-30 报告写完
- [ ] 17:30 后 cron 继续监督 (R125-1 已 done, 立刻补 R125-15 = R125-2 重派 / R125 续)

### 6.3 R125 末 (8/31) 节点

- [ ] 12 任务全部 done
- [ ] R126 派活 (5 拆 crate + 4 协议 handler trait 真接 + 守门 v6.1)
- [ ] R127 1.0 release 派活 (ASI 24 维 + Skill 化 + 集成测试)

---

**Mavis 16:42 状态**: 主人 5 次拍板 (01:14 + 01:49 + 16:27 + 16:31 + 16:37) 累积授权. 16 派满策略 + cron 监督 + 少人补上已就位. R125 12 任务 spec ready, 17:00 派满 16, 17:30 整合 #3 commit 拍板, R125 末 8/31 全部 done, R126/R127 续. 0 主动 commit, 0 越界 8 硬墙, 主人 1.0 release 路线图清晰.
