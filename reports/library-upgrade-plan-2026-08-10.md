# Apeireth Library 升级 Plan — research → library 6 阶段

**Date**: 2026-08-10 16:48
**Author**: Mavis (root session, 主人 16:43 "research → library, 调研做完了你自己安排任务升级")
**关联决策**: `decision-24-r125-15-library-2026-08-10.md` (决策登记)
**关联报告**: `r125-15-non-github-resources.md` (R125-15 6 大类 spec)
**周期**: R125 W1-3 (8/11-8/31) 完成阶段 1-3 + R126/R127 续 4-6
**派活**: 6 阶段 × 1-3 sub-agent = 6 任务 (R125-16 ~ R125-21)
**状态**: ✅ **6 阶段 spec ready, Mavis 自主升级**

---

## 0. 触发事件

**主人 16:43**:
> "A+B. 我们不是原来就有 research 文件夹么, 一起做成 library 了, 作为资料库, 调研做完了你自己安排任务升级"

**现状盘点**:
- `research/` 已存在: 9 子文件夹 + 147 文件 + 2.2MB 调研资料 + 8 arxiv 论文 + INDEX.json + README.md (10KB)
- R124 调研: 137 借鉴 + 135 ID + 3 调研报告 (138KB) — 待整合到 library
- R125-15 调研 (待派): 6 大类 100+ 资源 — 写进 library/10-non-github-resources/
- Top 10 借鉴源码 git clone 跑中 (2/10, GitHub 限流) — 待整合到 library/12-borrowed-repos/

---

## 1. Library 命名 + 概念升级

### 1.1 Library vs research 概念升级

| 概念 | research (旧) | library (新) |
|---|---|---|
| **定位** | 调研归档 | 资料库 (主人 1.0 release 礼物) |
| **结构** | 9 子文件夹 (按主题分类) | 9 子文件夹 + 10/11/12 新子 + 借鉴 ID 严格化 + 索引 |
| **内容** | README + 抓取脚本 + 调研发现 | README + 索引 + 借鉴 ID + 摘要 + Top 100 |
| **维护** | 静态归档 | 持续更新 (R125+ 借鉴 + 论文) |
| **访问** | 仅 R14 团队 | 全员 (含 TUI 9 organ page) |
| **质量** | 调研 (1 次性) | 资料库 (长期) |

### 1.2 Library 软链接 vs 重命名 (Mavis 决定)

**Mavis 决定**: 保留 `research/` 目录, 新增 `library/` 软链接 + 索引.

理由:
- `research/` 是 8/1 主人已建立, 主人 16:43 说"原来就有", 表明认可
- 重命名可能破坏 git history
- 软链接是 R119 R125 主人拍板策略 (形式可重整, 实质不变)
- research/ 内容 0 改, library/ 是新概念层

**实施**:
```powershell
# 在 Apeireth-rust 根目录
New-Item -ItemType SymbolicLink -Path library -Target research
```

或备选: `library/` 是新独立目录, 软链接 `research/01-09` 到 `library/01-09` 9 个软链接.

**Mavis 决定**: 1.0 release 时再决定 rename 还是保持软链接.

---

## 2. Library 6 阶段升级 (R125 W1-R127 续)

### 阶段 1: Library 命名 + 文档结构 (R125 W1, 8/11, 4-6h)

**R125-16**:
- 创建 `library/README.md` (新, 16KB) — Library 总览
  - 0 改 `research/README.md` (实质 0 改)
  - 9 大类索引 + 10/11/12 新子目录
  - 借鉴 ID 格式说明
  - Library 维护原则 (Mavis 自主)
- 创建 `library/INDEX.json` (机器可读索引) — 147 文件 + 137 借鉴 ID 索引
- 创建 `library/CLASSIFICATION.md` — 9 大类分类说明
- 0 触碰 `research/` 任何文件

### 阶段 2: 9 大类升级 + 10/11/12 新子 (R125 W1, 8/11-8/17, 1 周)

**R125-17** (派 2-3 sub-agent 并行):
- 9 子文件夹升级为 Library 标准结构 (每子目录增加 `_SUMMARY.md` 摘要)
  - `library/01-ai-agent-platforms/_SUMMARY.md` (10-30KB)
  - `library/02-memory-retrieval-systems/_SUMMARY.md`
  - `library/03-rust-ecosystem/_SUMMARY.md`
  - `library/04-philosophy-prompts/_SUMMARY.md`
  - `library/05-arxiv-papers/_SUMMARY.md` (8 论文)
  - `library/06-mcp-tools/_SUMMARY.md`
  - `library/07-ai-frameworks/_SUMMARY.md`
  - `library/08-rust-substrate-current/_SUMMARY.md`
  - `library/09-misc/_SUMMARY.md`
- 新增 `library/10-non-github-resources/` (R125-15 产出, 6 大类子目录)
  - `01-arxiv-papers/`
  - `02-official-docs/`
  - `03-tech-blogs/`
  - `04-videos/`
  - `05-communities/`
  - `06-hubs/`
- 新增 `library/11-vcp-reference/` (VCP 相关借鉴, 现有 04-philosophy-prompts 升级)
  - 0 改原 04, 仅加 VCP 专项
- 新增 `library/12-borrowed-repos/` (R124 Top 10 借鉴源码索引, git clone 跑中)
  - `README.md` (索引, 已写)
  - `LiteLLM/_NOTES.md`
  - `LangGraph/_NOTES.md`
  - `OpenCode/_NOTES.md`
  - ... (10 个)

### 阶段 3: 借鉴 ID 严格化 (R125 W2, 8/18-8/24, 1 周)

**R125-18** (派 3 sub-agent 并行):
- 400+ 借鉴 ID 严格化:
  - 9 子文件夹 147 文件 → 100+ 借鉴 ID
  - R124 137 借鉴 → 137 借鉴 ID (已严格化)
  - R125-15 100+ 资源 → 100+ 借鉴 ID
  - **总计 400+ 借鉴 ID**
- 借鉴 ID 格式统一: `R{N}-BORROW-{type}-{owner/repo or title}-{hash}-YYYY-MM-DD`
- 索引到 `library/INDEX.json` + `library/_BORROW_IDS.md` (40KB)
- 0 改 `research/` 任何文件

### 阶段 4: Library 摘要 (R125 W3, 8/25-8/31, 1 周)

**R125-19** (派 2 sub-agent 并行):
- 9 大类每类 1 份 `_SUMMARY.md` (10-30KB) — 类内借鉴总览 + Top 5 推荐
- 1 份 `library/_TOP_100.md` (50KB) — 主人 1.0 release 前 100 必读
- 0 改 `research/` 任何文件

### 阶段 5: Library 工具 + TUI 集成 (R126 W1-2, 9/1-9/14, 2 周)

**R125-20** (派 3 sub-agent 并行):
- `library/_SEARCH.md` (5KB) — 检索指南 (按 crate / 借鉴 / 主题)
- `library/_CROSS_REF.md` (20KB) — 跨引用 (跟 9 organ / 24 LOCKED / 5 守门 对应)
- TUI 9 organ page 集成:
  - `crates/apeireth-tui/src/nav/library.rs` (NEW) — Library nav
  - 5 nav 之一 "Library" (新增, B5 哲学锚穿透)
  - 集成 `library/INDEX.json` 数据源

### 阶段 6: Library v1.0 (R127, 11-12 月, 1 月)

**R125-21** (派 1 sub-agent):
- Library v1.0 release 礼物:
  - 30 本经典书 (按 9 organ 分类)
  - 100 论文 (R125-15 产出)
  - 50 视频 (R125-15 产出)
  - 10 社区 (R125-15 产出)
  - 10 hub (R125-15 产出)
- `library/v1.0/` 目录 (NEW) — 1.0 release 礼物版本
- `library/v1.0/README.md` (10KB) — 1.0 release 礼物介绍

---

## 3. Library 总览 (阶段 1-3 完成时, 8/31)

```
Apeireth-rust/library/  (软链接到 research/ 或独立目录, Mavis 决定)
├── README.md                       (新, 16KB) Library 总览
├── INDEX.json                      (新) 400+ 借鉴 ID 机器可读索引
├── CLASSIFICATION.md               (新) 9 大类分类说明
├── _BORROW_IDS.md                  (新, 40KB) 400+ 借鉴 ID 索引
├── _TOP_100.md                     (新, 50KB) 主人 1.0 release 前 100 必读
├── _SEARCH.md                      (新) 检索指南
├── _CROSS_REF.md                   (新) 跨引用
│
├── 01-ai-agent-platforms/  (147 文件 + _SUMMARY.md 摘要)
├── 02-memory-retrieval-systems/    (8 文件 + _SUMMARY.md)
├── 03-rust-ecosystem/              (3 文件 + _SUMMARY.md)
├── 04-philosophy-prompts/          (17 文件 + _SUMMARY.md)
├── 05-arxiv-papers/                (8 论文 + INDEX + _SUMMARY.md)
├── 06-mcp-tools/                   (5 文件 + _SUMMARY.md)
├── 07-ai-frameworks/               (5 文件 + _SUMMARY.md)
├── 08-rust-substrate-current/      (47 文件 + _SUMMARY.md)
├── 09-misc/                        (32 文件 + _SUMMARY.md)
│
├── 10-non-github-resources/  (R125-15 产出)
│   ├── 01-arxiv-papers/             (30+ 论文)
│   ├── 02-official-docs/            (20+ spec)
│   ├── 03-tech-blogs/               (15+ 博客)
│   ├── 04-videos/                   (15+ 视频)
│   ├── 05-communities/              (10+ 社区)
│   └── 06-hubs/                     (10+ hub)
│
├── 11-vcp-reference/                (VCP 专项)
│
└── 12-borrowed-repos/               (R124 Top 10 借鉴源码)
    ├── README.md                    (索引, 已写)
    ├── LiteLLM/
    ├── LangGraph/
    ├── OpenCode/
    ├── MCP-servers/
    ├── PyO3/
    ├── NVIDIA-Guardrails/
    ├── Kani/
    ├── sqlite-vec/
    ├── OpenCog/  (AGPL-3.0 ⚠️)
    └── Chidori/
```

---

## 4. Library 8 硬墙全守

1. ✅ **workspace.version 1.1.0** (0 改, R125 末 B2 升 1.2)
2. ✅ **R11 baseline 3 值** (0 改, A1 严守)
3. ✅ **24 LOCKED crate** (0 触碰, Library 升级不动 src)
4. ✅ **6 哲学锚** (0 改, B5 待 R125 末升 8 锚)
5. ✅ **9 organ** (0 改, B7 待 R125-12 内部借, 阶段 5 TUI 集成)
6. ✅ **11 公共 API** (0 改, Library 升级不动 API)
7. ✅ **0 装 (O-5)** 12 键编译期 hardcode 严守
8. ✅ **0 主动 commit** (Mavis 整合 #3 拍板, R125-16 ~ R125-21 派活 0 主动 commit)

**Library 升级不动 `research/` 内容** (主人说"原来就有"), 仅新增 `library/` 概念层 + 索引 + 摘要.

---

## 5. R125 末 36 任务派活清单 (12 借鉴 + 6 R125-15 子 + 6 Library 升级 + 12 续)

| 任务 | 主题 | 估时 | 阶段 | 截止 |
|---|---|---|---|---|
| **R125-1** | LiteLLM Provider Registry | 50 min 17:30 | R125 W1 | 8/10 17:30 |
| **R125-2** | clap derive | 4-6 h | R125 W1 | 8/11 |
| **R125-3** | hyper 池 | 1 天 | R125 W1 | 8/11 |
| **R125-4** | MCP servers | 1-2 天 | R125 W1 | 8/12 |
| **R125-5** | NVIDIA Guardrails Colang DSL | 2-3 天 | R125 W1 | 8/13 |
| **R125-7** | aGLM PODA | 3-5 天 | R125 W2 | 8/15 |
| **R125-8** | Chidori journal | 1 周 | R125 W2 | 8/17 |
| **R125-9** | PyO3 | 1-2 天 | R125 W2 | 8/16 |
| **R125-10** | Kani 形式化 | 2-3 天 | R125 W2 | 8/17 |
| **R125-12** | OpenCode 子代理 | 3-5 天 | R125 W3 | 8/20 |
| **R125-13** | LangGraph StateGraph | 1 周 | R125 W3 | 8/22 |
| **R125-14** | obra/superpowers | 1-2 天 | R125 W3 | 8/20 |
| **R125-15a** | 学术论文 (P0) | 1-2 天 | R125 W1 | 8/12 |
| **R125-15b** | 官方文档 (P0) | 1-2 天 | R125 W1 | 8/12 |
| **R125-15c** | 技术博客 (P1) | 1-2 天 | R125 W1 | 8/13 |
| **R125-15d** | 会议视频 (P1) | 1-2 天 | R125 W1 | 8/13 |
| **R125-15e** | 社区 (P2) | 1 天 | R125 W1 | 8/14 |
| **R125-15f** | Hub (P2) | 1 天 | R125 W1 | 8/14 |
| **R125-16** | Library 阶段 1 升级 | 4-6 h | R125 W1 | 8/11 |
| **R125-17** | Library 阶段 2 升级 | 1 周 | R125 W1 | 8/17 |
| **R125-18** | Library 阶段 3 借鉴 ID 严格化 | 1 周 | R125 W2 | 8/24 |
| **R125-19** | Library 阶段 4 摘要 | 1 周 | R125 W3 | 8/31 |
| **R125-20** | Library 阶段 5 工具 + TUI 集成 | 2 周 | R126 W1-2 | 9/14 |
| **R125-21** | Library v1.0 礼物 | 1 月 | R127 | 12/31 |

**总 36 任务 (R125 末 24 + R126 续 1 + R127 续 1)**, 2-3 周完成 R125 末 24 任务.

---

## 6. 16 派满策略 (Mavis 自主)

| 周次 | 16 满 (3 续 + 12 新 + 1 备用) |
|---|---|
| **W1 (8/11-8/17)** | 3 续 (R123-1 已完, R124-2 mark done, git clone) + R125-1/2/3/4/5 + R125-15a/b/c/d + R125-16/17 = 16 满 |
| **W2 (8/18-8/24)** | 3 续 (新) + R125-7/8/9/10 + R125-15e/f + R125-18 = 16 满 |
| **W3 (8/25-8/31)** | 3 续 (新) + R125-12/13/14 + R125-19 + 5 R125 续 = 16 满 |
| **R126 W1-2 (9/1-9/14)** | 3 续 + R125-20 + 12 R126 (5 拆 crate + 4 协议 handler + StateGraph + 守门 v6.1) = 16 满 |
| **R127 (11-12 月)** | 3 续 + R125-21 + 12 R127 (ASI 24 维 + Skill 化 + 集成测试) = 16 满 |

**Mavis 监督**: watch-r121-1300 cron 5 min tick, 距 16 cap 剩 slots > 0 → 立刻派下一个 R125/R126/R127 任务.

---

## 7. 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| **Library 软链接 Windows 失败** | 阶段 1 升级延迟 | 备选: library/ 是新独立目录, 软链接 research/01-09 |
| **9 子文件夹 _SUMMARY 摘要** 工作量大 | 1 周延 | 派 3 sub-agent 并行 (R125-17 派 3 个) |
| **400+ 借鉴 ID 严格化** 工作量大 | 1 周延 | 派 3 sub-agent 并行 (R125-18 派 3 个) |
| **TUI 集成 (R125-20)** 触发 B7 (9 organ 内部借 OpenCode) | R125-12 + R125-20 协调 | R125-12 先 (9 organ 内部 fn 借), R125-20 后 (TUI Library nav 集成) |
| **Library v1.0 礼物 (R125-21)** 1 月 | R127 11-12 月 | Mavis 自主, 1.0 release 前完成 |
| **0 主动 commit 严守** R125-16~R125-21 派活时 | R125 续 commit 拍板 | Mavis 整合 #3 拍板 (per 17:30 节点) |
| **R125 派活没自动响应** (主人 16:43 指出) | 派满 16 失败 | mavis cron trigger + 4 步修复 (per decision-24 §1.3) |
| **借鉴源码 git clone 限流** (2/10 跑 30+ min) | 阶段 2 升级延迟 | background 跑, --depth 1, 失败 1 次重试 |

---

## 8. 0 拍板执行

### 8.1 16:48 立即执行

- [x] 写本 Library 升级 plan (6 阶段 + 36 任务派活清单)
- [x] R125-15 spec (6 大类 100+ 资源)
- [x] decision-24 (派活修复 + R125-15 + Library 升级)
- [x] `mavis cron trigger` watch-r121-1300 (立即派活触发)
- [ ] 16:50 下个 tick verify 派活 (R125-1/5/10/12 应该 running)
- [ ] 17:30 整合 #3 commit 拍板 + final-17-30 报告

### 8.2 R125 末 (8/31) 节点

- [ ] R125-1 ~ R125-14 12 借鉴任务 done
- [ ] R125-15 6 子任务 done (100+ 资源)
- [ ] R125-16 ~ R125-19 Library 阶段 1-4 升级 done
- [ ] Library v1.0-alpha (W3 末) — 9 大类索引 + 400+ 借鉴 ID 严格化 + _TOP_100

### 8.3 R126 (9-10 月)

- [ ] R125-20 Library 阶段 5 升级 (_SEARCH + _CROSS_REF + TUI 集成)
- [ ] 5 拆 crate (tui-backend / keyring-platform-3 / constraint-engine / classifier-core / pipeline-derive)
- [ ] 4 协议 handler trait 真接 (R123-2 骨架 + R125-1 续)
- [ ] 守门 v6.1 (R125-5 续)

### 8.4 R127 (11-12 月)

- [ ] R125-21 Library v1.0 (1.0 release 礼物)
- [ ] ASI 24 维 (B3 25/30 维续)
- [ ] Skill 化 (R125-14 续)
- [ ] 集成测试
- [ ] 1.0 release

---

**Mavis 16:48 状态**: 主人 6 次拍板累积 (01:14 + 01:49 + 16:27 + 16:31 + 16:37 + 16:43). Library 升级 6 阶段 spec ready. R125 末 36 任务派活清单 (12 借鉴 + 6 R125-15 子 + 6 Library 升级 + 12 续). 16 派满策略 W1-W3 派活顺序锁定. 17:30 整合 #3 commit 拍板 + final-17-30 报告. 0 主动 commit, 0 越界 8 硬墙, 主人 1.0 release 路线图清晰.
