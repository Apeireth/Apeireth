# Decision #32 — R125 派活大主管启动 + 0 装 PASS 监督策略 (17:18)

**Date**: 2026-08-10 17:18
**Author**: Mavis (新 root session, mvs_47dd64fb4fc24e23b30edd5f649bfebb)
**触发**: 17:15 派 R125 派活大主管 bg_62424f99 (1 task = 派 16 个 R125 sub-agent), 17:17 cron self `watch-r125-supervisor-17-22` 5 min tick 监督
**关联**: decision-30 (新 Mavis 接入 + daemon 复活) + decision-31 (17:30 拍板 dry-run) + handoff §5 (R125 24 任务派活清单)

---

## 0. 一句话

**R125 派活大主管 bg_62424f99 17:15 派出, 1 task = 派 16 R125 sub-agent (P0 5 / P1 5 / P2 4 / P3 2), 5 min tick cron self 监督, 0 越界 8 硬墙, 0 装 PASS (借鉴源码 0 clone = 0 实施), 0 主动 commit, 0 主动 push, 17:30 拍板按 decision-31 §2 spec 干**.

---

## 1. 派活清单 (16 任务, 16 slots 派满)

| # | 任务 | 借鉴 ID | P | 估时 | 截止 |
|---|------|---------|---|------|------|
| 1 | R125-1 LiteLLM Provider Registry | R124-1-BORROW-BerriAI/litellm-3a8e2c1-2026-08-10 | P0 | 50 min | 8/10 17:30 (0 含, 跑过夜) |
| 2 | R125-2 clap derive | R124-1-BORROW-clap-rs/clap-4f7a2c1-2026-08-10 | P0 | 4-6h | 8/11 |
| 3 | R125-3 hyper 池复用 | R124-1-BORROW-hyperium/hyper-util-2e9d4b6-2026-08-10 | P0 | 1 天 | 8/11 |
| 4 | R125-4 MCP servers 协议对齐 | R124-3-BORROW-modelcontextprotocol/servers-primitive-namespace-2026-08-10 | P0 | 1-2 天 | 8/12 |
| 5 | R125-5 NVIDIA Guardrails Colang DSL | R124-3-BORROW-NVIDIA-NeMo/Guardrails-Colang-DSL-2026-08-10 | P0 | 2-3 天 | 8/13 |
| 6 | R125-7 aGLM PODA cycle | R124-2-BORROW-GATERAGE/aglm-2024Q4-2026-08-10 | P1 | 3-5 天 | 8/15 |
| 7 | R125-8 Chidori host-call journal | R124-2-BORROW-ThousandBirdsInc/chidori-2025-12-2026-08-10 | P1 | 1 周 | 8/17 |
| 8 | R125-9 PyO3 重构 pybridge | R124-3-BORROW-PyO3/PyO3-2026-08-10 | P1 | 1-2 天 | 8/16 |
| 9 | R125-10 Kani 形式化 24 LOCKED | R124-3-BORROW-model-checking/kani-harness-pattern-2026-08-10 | P1 | 2-3 天 | 8/17 |
| 10 | R125-12 OpenCode 子代理 | R124-1-BORROW-anomalyco/opencode-7a4b9c2-2026-08-10 | P1 | 3-5 天 | 8/20 |
| 11 | R125-13 LangGraph StateGraph | R124-1-BORROW-langchain-ai/langgraph-5f8a3c7-2026-08-10 | P2 | 1 周 | 8/22 |
| 12 | R125-14 obra/superpowers Skill | R124-2-BORROW-obra/superpowers-2026-05-2026-08-10 | P2 | 1-2 天 | 8/20 |
| 13 | R125-15a 学术论文 30+ | R125-15-BORROW-arxiv-{name|id}-{hash}-2026-08-10 | P2 | 1-2 天 | 8/12 |
| 14 | R125-15b 官方文档/RFC 20+ | R125-15-BORROW-rfc-{name|id}-{hash}-2026-08-10 | P2 | 1-2 天 | 8/12 |
| 15 | R125-15c 技术博客 15+ | R125-15-BORROW-blog-{name|id}-{hash}-2026-08-10 | P3 | 1-2 天 | 8/13 |
| 16 | R125-15d 会议视频 15+ | R125-15-BORROW-video-{name|id}-{hash}-2026-08-10 | P3 | 1-2 天 | 8/13 |

**总输出**: 16 任务 + 16 借鉴 ID 全部唯一 + 16 final 报告 (R125 续 8/15-9/10).

---

## 2. 0 装 PASS 监督策略 (per decision-25 §1)

### 2.1 借鉴源码 git clone 状态 (5/10 → 4/10 实际)

| # | 仓库 | 借鉴 ID | 17:18 clone 状态 |
|---|------|---------|------------------|
| 1 | langgraph | R124-1-BORROW-langchain-ai/langgraph-5f8a3c7-2026-08-10 | ✅ cloned (.git) |
| 2 | opencode | R124-1-BORROW-anomalyco/opencode-7a4b9c2-2026-08-10 | ⏳ handoff §5 写 cloned, 17:18 目录 0 可见, 限流可能 |
| 3 | PyO3 | R124-3-BORROW-PyO3/PyO3-2026-08-10 | ✅ cloned (.git) |
| 4 | MCP servers | R124-3-BORROW-modelcontextprotocol/servers-2026-08-10 | ✅ cloned (servers 目录) |
| 5 | NVIDIA Guardrails | R124-3-BORROW-NVIDIA/NeMo-Guardrails-2026-08-10 | ✅ cloned (Guardrails 目录) |
| 6 | LiteLLM | R124-1-BORROW-BerriAI/litellm-3a8e2c1-2026-08-10 | ⏳ 限流中, R125-1 紧急, 17:30 0 含实施 |
| 7 | Kani | R124-3-BORROW-model-checking/kani-2026-08-10 | ⏳ 限流中 |
| 8 | sqlite-vec | R120 A 已真接 | ✅ R120 A 真接 |
| 9 | OpenCog | R125-9 跳过 (AGPL-3.0 ⚠️) | ❌ 不 clone |
| 10 | Chidori | R124-2-BORROW-ThousandBirdsInc/chidori-2025-12-2026-08-10 | ⏳ 限流中 |

**路径**: `.openclaw\workspace\borrowed-repos\`
**索引**: `borrowed-repos/README.md` 6.2KB (已写)
**0 污染**: 主仓 git status 0 包含 borrowed-repos

### 2.2 0 装 PASS 监督 (per task)

**R125 主管**对每个 sub-agent prompt 写:
> "借鉴源码 0 clone 完成 = 0 实施, 报告诚实标'待 clone', 0 假装'已借鉴'."

**R125 sub-agent**在 final 报告 (`reports/agent-r125-{N}-final-2026-08-10.md`) 写:
> "借鉴源码状态: ✅ cloned / ⏳ 限流中, 0 实施" 或者 "✅ cloned, 已实施" 或者 "0 实施 (借鉴 ID 索引完成, src 0 改)"

**Mavis 监督** (cron self 5 min tick) 检查:
1. R125 主管 bg_62424f99 派了几个 sub-agent
2. 每个 sub-agent 报告里借鉴源码状态 0 装 PASS
3. 0 越界 8 硬墙
4. 0 主动 commit

### 2.3 5 min tick cron self (per task `watch-r125-supervisor-17-22`)

每 5 min 跑:
1. `task_query bg_62424f99` — 状态 (running/succeeded/failed)
2. 距 17:30 剩余时间
3. <mavis-progress>...</mavis-progress> 输出 (gate-discipline 严守)
4. 17:25 评估 R123-1 (cron `r123-1-deadline-1725` 也跑)
5. 17:28 dry-run
6. 17:30 commit

---

## 3. 5 min tick 监督 (5 个 cron + 1 个 cron self)

| cron | 频率 | 作用 | 状态 |
|------|------|------|------|
| `dispatch-r125-r125-15-library-immediate` (435f1373) | */1 min | 1 min tick 派活 (老 session mvs_ee7ca3badb) | 🟢 active |
| `dispatch-r125-now-min-tick` (d8bab746) | */1 min | 1 min tick 派活 (老 session) | 🟢 active |
| `watch-r121-1300` (15ede269) | */5 min | 5 min tick 监督 R121-R125 (老 session) | 🟢 active |
| `r123-1-deadline-1725` (2e6c171c) | */5 min | 5 min 提醒 R123-1 17:30 截止 (老 session) | 🟢 active |
| `R120-finalize-1000` (4b2dd57d) | 0 */8 h | 8h 周期 R120 final 整理 (老 session) | 🟢 active |
| **`watch-r125-supervisor-17-22` (8118c76b)** | **\*/5 min** | **5 min tick 监督 R125 主管 + 17:30 拍板 (新 session me)** | **🟢 active (17:15 建)** |

**5 + 1 = 6 cron 跑中, 5 老 (mvs_ee7ca3badb session) + 1 新 (我 mvs_47dd64fb session)**.

---

## 4. 0 越界 8 硬墙 (per decision-22 §2) verify

| 硬墙 | 0 越界 | R125 派活动作 |
|------|--------|--------------|
| 1. workspace.version 1.1.0 | ✅ | R125 续 0 触碰 (R125 末 B2 升 1.2.0) |
| 2. R11 baseline 3 值 (0.8682/0.8532/0.9063) | ✅ | R125-10 Kani 形式化 = B3 25 维新增, 0 改 R11 baseline 值 |
| 3. 24 LOCKED crate mtime 16:34 baseline | ✅ | R125 续 0 触碰 24 LOCKED (R125-1/2/3/4/5/7/8/9/12/13/14 都新增 mod/feature, 0 改 LOCKED) |
| 4. 6 哲学锚 (R125 末升 8) | ✅ | R125 续 0 改 (B5 落实 R125 末升 8 锚) |
| 5. 9 organ 文件名 + 入口签名 | ✅ | R125-12 OpenCode 子代理 = 内部借, 0 改 organ 文件名 + 入口签名 |
| 6. 11 公共 API | ✅ | R125 续 0 改 (R125 内部新增, 0 触碰 11 公共 API) |
| 7. 0 装 (O-5) 12 键 + PHL-07 编译期 hardcode | ✅ | R125 续 0 装 (12 键 + PHL-07 编译期严守, R125 0 假装 PASS) |
| 8. 0 主动 commit + 0 主动 push | ✅ | R125 主管 0 主动 commit (Mavis 整合 #3 拍板 17:30, 0 提前) |

---

## 5. 借鉴 ID 严格化 (per decision-22 §3)

### 5.1 格式
```
R124-{1,2,3}-BORROW-{owner/repo}-{hash}-2026-08-10   # GitHub 仓库
R125-15-BORROW-{arxiv|blog|video|community|hub|rfc}-{name|id}-{hash}-2026-08-10  # 非 GitHub
```

### 5.2 唯一性 verify

R125 16 任务 16 借鉴 ID 全部唯一:
- 11 个 GitHub 仓库 (R124-1/2/3 不同 owner/repo)
- 4 个 R125-15 非 GitHub 大类 (arxiv/rfc/blog/video, 各自 {name|id} 不同)

**0 重复** (R125 主管派活 prompt 必带借鉴 ID 唯一 verify).

---

## 6. R125 主管 prompt 8 硬墙严守 (per decision-22 §2 + decision-25 §1 + decision-31 §3)

R125 主管 sub-agent prompt 8 硬墙严守, 关键约束:
1. workspace.version 1.1.0 0 改
2. R11 baseline 3 值 0 改
3. 24 LOCKED crate mtime 16:34 baseline 0 触碰
4. 6 哲学锚 0 改 (R125 末升 8 锚, 0 装 0 提前)
5. 9 organ 文件名 + 入口签名 0 改
6. 11 公共 API 0 改
7. 0 装 (O-5) 严守: 借鉴源码 0 clone = 0 实施, 0 假装"已借鉴"
8. 0 主动 commit + 0 主动 push (Mavis 整合 #3 拍板 17:30 节点)

---

## 7. 17:25 R123-1 截止评估

**R123-1** (`mvs_fa42b99...`, started, updatedAt 17:14:11, 0 cargo process 9+ min):
- 16:48 status 报告: clippy-final 2 ERROR 修中 (apeireth-mcp 1 + tools_demo.rs 2 delimiter)
- 17:18 现在: 0 cargo process = 0 修进展 = 卡 thinking 阶段
- 17:25 截止: 距 7 min
- Mavis 0 干预 (decision-24 严守)
- 0 装 PASS: 0 假装"已修"
- 17:25 没 done → 0 装 PASS, 17:30 拍板按 handoff §3 spec 干 (0 add src)
- 17:25 done → R123-1 sub-agent 自己 commit (R123-1 commit 链)

**Mavis 0 干预, 17:25 cron tick `r123-1-deadline-1725` 自动评估**.

---

## 8. 决策链 (接 #31)

- **#30 (17:15)**: 新 Mavis 接入 + 派活 daemon 复活 (sanity check pong OK) + 16 派满立刻执行
- **#31 (17:17)**: 17:30 拍板 dry-run + 138 src 改动诚实标
- **#32 (17:18)**: R125 派活大主管启动 + 0 装 PASS 监督策略 + 5+1 cron 监督 + 16 借鉴 ID 严格化

---

## 9. 一句话 (TL;DR)

**R125 主管 bg_62424f99 跑中 (派 16 R125 sub-agent), 5 min tick cron self 监督, 0 装 PASS 借鉴源码 4/10 实际 + 5/10 限流 + 1/10 跳过, 8 硬墙 0 越界, 17:30 拍板按 decision-31 §2 spec 干**.
