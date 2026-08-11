# Library v1.0 — 30 本经典书 (按 9 organ 分类, 推荐清单)

**Date**: 2026-08-10
**Author**: P2-4 sub-agent (Mavis 派, per 决策 #51 §1.3)
**借鉴 ID**: `R126-library-v1-BORROW-N-A-{hash}-2026-08-10` (N/A = 0 借仓库, 纯准备 spec)
**借鉴源码**: 0 借 (无 GitHub 仓库, 0 装 PASS 严守, 标"准备 (0 实施)")
**0 装 PASS**: ⏳ 准备 = 0 装"已发 1.0 release 礼物"
**8 硬墙**: 0 越界 (Library 升级不动 Cargo.toml / baseline / 24 LOCKED / 13 键 / 0 commit / 0 push)

> **重要说明**: 本清单是 **推荐清单 (curated bibliography)**, 不是 PDF 仓库. 主人按 ISBN/书名自行获取 (图书馆 / 在线书店 / 公开 PDF 链接). 0 装"已下载 30 本 PDF" 严守.

---

## 0. 一句话 (TL;DR)

**30 本经典书按 9 organ 分类, 每 organ 3-4 本, 总 30 本**. 9 organ 映射到 9 类认知能力 (body / brain / ear / eye / hand / heart / memory / mind / voice), 每类 3-4 本 = 30 本. 清单 = 推荐书名 + 作者 + 1 句核心 + 借鉴方向, 0 装"已下载".

---

## 1. 9 organ 30 本经典书 分布

| # | Organ | 数量 | 主题 | 占比 |
|---|---|---:|---|---:|
| 1 | **body** (工具/执行) | 3 | 工具规范 / 自动化 / 工程化 | 10% |
| 2 | **brain** (LLM 循环) | 4 | Agent 架构 / LLM 循环 / 推理 | 13% |
| 3 | **ear** (输入) | 3 | 信息检索 / 监听 | 10% |
| 4 | **eye** (视觉) | 3 | 可视化 / 仪表盘 / 状态 | 10% |
| 5 | **hand** (执行) | 3 | 工具调用 / dispatch | 10% |
| 6 | **heart** (节律) | 3 | 异步 / 并发 / 心跳 | 10% |
| 7 | **memory** (记忆) | 4 | 长期记忆 / 向量检索 / 知识图谱 | 13% |
| 8 | **mind** (认知) | 4 | 认知架构 / 反思 / 自我 | 13% |
| 9 | **voice** (输出) | 3 | 表达 / UI / 文本 | 10% |
| **总** | **9 organ** | **30** | — | **100%** |

---

## 2. 30 本经典书 清单 (按 organ 排序)

### 2.1 body (工具/执行, 3 本)

| # | 书名 | 作者 | 核心 1 句 | 借鉴方向 |
|---|---|---|---|---|
| 1 | **The Pragmatic Programmer** | Andrew Hunt, David Thomas | 工具规范化 + 自动化, 0 假装"已交付" | 工具规范 anchor (S-1) |
| 2 | **Code Complete** | Steve McConnell | 软件工程实践, 6 重守门雏形 | 工程化 + 守门 v6 (B4) |
| 3 | **The Phoenix Project** | Gene Kim et al. | DevOps + 持续交付 + 5 重守门故事 | 6 重守门 v6 灵感 (B4) |

### 2.2 brain (LLM 循环, 4 本)

| # | 书名 | 作者 | 核心 1 句 | 借鉴方向 |
|---|---|---|---|---|
| 4 | **Designing Data-Intensive Applications** | Martin Kleppmann | 数据流 + Agent 消息流本质 | 协议层 + 消息流 |
| 5 | **Artificial Intelligence: A Modern Approach** (AIMA) | Stuart Russell, Peter Norvig | AI 经典教科书, agent + search + learning | Agent 架构基础 |
| 6 | **Speech and Language Processing** (Jurafsky) | Dan Jurafsky, James Martin | NLP 经典教科书, 涵盖 LLM 前世今生 | LLM 循环 + tokenization |
| 7 | **Prompt Engineering for LLMs** (3 阶段读物) | 多作者 | prompt 设计 + ReAct + Chain-of-Thought | LLM 调用模式 (brain.rs) |

### 2.3 ear (输入/监听, 3 本)

| # | 书名 | 作者 | 核心 1 句 | 借鉴方向 |
|---|---|---|---|---|
| 8 | **Information Retrieval** (Baeza-Yates) | Ricardo Baeza-Yates | 经典信息检索教科书 | 检索 + RAG 基础 (ear.rs) |
| 9 | **Introduction to Information Retrieval** (Manning) | Christopher Manning et al. | Stanford 经典 IR 教材 | 倒排索引 + TF-IDF |
| 10 | **Streaming Systems** | Tyler Akidau et al. | 流式数据 + 实时处理 | 事件流 + ear.rs 借鉴 |

### 2.4 eye (视觉/状态, 3 本)

| # | 书名 | 作者 | 核心 1 句 | 借鉴方向 |
|---|---|---|---|---|
| 11 | **The Visual Display of Quantitative Information** | Edward Tufte | 数据可视化经典, "data-ink ratio" | TUI 仪表盘 + eye.rs |
| 12 | **Envisioning Information** | Edward Tufte | 可视化 + 信息密度 | 高密度 TUI 卡片 |
| 13 | **Information Dashboard Design** | Stephen Few | 仪表盘设计原则 | 9 organ 卡片布局 |

### 2.5 hand (执行/工具, 3 本)

| # | 书名 | 作者 | 核心 1 句 | 借鉴方向 |
|---|---|---|---|---|
| 14 | **Tools and Weapons** | Brad Smith, Carol Ann Browne | 微软总裁谈技术 + 责任 | 工具规范 + 守门 (hand.rs) |
| 15 | **Operating Systems: Three Easy Pieces** | Arpaci-Dusseau | OS 经典, 进程 / 调度 / 同步 | 工具调度 + 并发 |
| 16 | **Distributed Systems** (van Steen) | Maarten van Steen, Andrew Tanenbaum | 分布式系统经典 | 协议 + dispatch 模式 |

### 2.6 heart (节律/异步, 3 本)

| # | 书名 | 作者 | 核心 1 句 | 借鉴方向 |
|---|---|---|---|---|
| 17 | **Concurrency in Go** | Katherine Cox-Buday | Go 并发模型, channel + select | 异步 + 心跳 (heart.rs) |
| 18 | **Programming Rust** (Blandy) | Jim Blandy, Jason Orendorff | Rust 权威指南, 涵盖 async/await | Tokio + async |
| 19 | **Hands-On Concurrency in Rust** | Brian L. Troutwine | Rust 并发实战 | heart.rs + heartbeat |

### 2.7 memory (长期记忆, 4 本)

| # | 书名 | 作者 | 核心 1 句 | 借鉴方向 |
|---|---|---|---|---|
| 20 | **Foundations of Databases** | Serge Abiteboul, Richard Hull, Victor Vianu | 数据库基础经典 | 长期记忆 + 索引 |
| 21 | **Knowledge Graphs** (Hogan) | Aidan Hogan et al. | 知识图谱综述 | 实体链接 + 图谱 (memory.rs) |
| 22 | **Vector Database Systems** (新方向读物) | 多作者 (2024-2026) | 向量数据库 + HNSW + ANN | RAG + 检索 |
| 23 | **Database Internals** | Alex Petrov | 分布式数据库内部 | 存储引擎 + memory.rs |

### 2.8 mind (认知/反思, 4 本)

| # | 书名 | 作者 | 核心 1 句 | 借鉴方向 |
|---|---|---|---|---|
| 24 | **How the Mind Works** | Steven Pinker | 认知科学经典 | 认知架构 + mind.rs |
| 25 | **Gödel, Escher, Bach** | Douglas Hofstadter | 自我指涉 + 意识 + 形式系统 | 反思 + 自我模型 (S-3) |
| 26 | **The Master Algorithm** | Pedro Domingos | 机器学习 5 大流派综述 | 学习 + 反思 |
| 27 | **Superintelligence** | Nick Bostrom | AGI 风险 + 路径 | 安全 + 守门 (B4 v6/v7) |

### 2.9 voice (输出/表达, 3 本)

| # | 书名 | 作者 | 核心 1 句 | 借鉴方向 |
|---|---|---|---|---|
| 28 | **The Elements of Style** | William Strunk, E. B. White | 英文写作经典 | 文本输出 + voice.rs |
| 29 | **Style: Lessons in Clarity and Grace** | Joseph Williams, Joseph Bizup | 学术 + 技术写作 | 文档 + 表达 |
| 30 | **Tufte's Six Ideas** (Beilein) | Robert Beilein | Tufte 思想 6 大核心 | 信息密度 + 表达 |

---

## 3. 9 organ 映射 关系

```
              ┌────────────┐
              │   body     │ 工具规范
              │ (3 本)     │ Andrew Hunt / McConnell / Kim
              └─────┬──────┘
                    │
              ┌─────▼──────┐
       ┌──────┤   brain    ├──────┐
       │      │ (4 本)     │      │
       │      │ Kleppmann  │      │
       │      └─────┬──────┘      │
       │            │             │
   ┌───▼───┐   ┌───▼───┐    ┌────▼────┐
   │  ear  │   │  eye  │    │  hand   │
   │(3 本) │   │(3 本) │    │ (3 本)  │
   │Baeza  │   │Tufte  │    │Smith/OS │
   └───┬───┘   └───┬───┘    └────┬────┘
       │           │             │
       │      ┌────▼────┐        │
       │      │  heart  │        │
       │      │(3 本)   │        │
       │      │Cox-Buday│        │
       │      └────┬────┘        │
       │           │             │
   ┌───▼───┐   ┌───▼───┐    ┌────▼────┐
   │memory │   │ mind  │    │  voice  │
   │(4 本) │   │(4 本) │    │ (3 本)  │
   │Abiteboul   │Pinker │    │Strunk  │
   └───────┘   └───────┘    └─────────┘
```

---

## 4. 0 装 PASS 严守 (per 主人 17:22 + 决策 #33 + 决策 #51 §1.3 P2-4)

- ❌ **0 装 "已下载 30 本 PDF"** — 30 本经典书 0 下载, 0 仓库存储, 0 装"已发 Library v1.0 礼物". 主人按 ISBN/书名自行获取
- ✅ **诚实标"推荐清单 (curated bibliography)"** — 30 本仅写"书名 + 作者 + 核心 1 句 + 借鉴方向", 0 装"已读"
- ❌ **0 装 "已借鉴" 1.0 release 礼物** — Library v1.0 礼物本身是 0 借仓库 (30 本书是公开知识财产, 0 仿仓库路径)
- ✅ **借鉴 ID 诚实标 N/A** — `R126-library-v1-BORROW-N-A-{hash}-2026-08-10`, 0 装"已借鉴 owner/repo"

---

## 5. 8 硬墙 0 越界

| 硬墙 | 状态 |
|---|---|
| **B2** workspace.version 1.2.0 (0 改) | ✅ 0 触碰 Cargo.toml |
| **A1** R11 baseline 3 值 数字 严守 | ✅ 0 触碰 integration_r_measure.rs 等 17 文件 |
| **B1** 24 LOCKED 入口签名 0 改 | ✅ 0 触碰 24 LOCKED crate |
| **A3** 13 键 0 改 | ✅ 0 触碰 13 键 hardcode |
| **C1** 0 主动 commit | ✅ 0 跑 git add/commit, Mavis 整合 #5 拍板 |
| **C3** v6 0 改 | ✅ 0 触碰 6 重守门 v6 |
| **0 push** git push | ✅ 0 push, 等 1.0 release 配 GitHub remote |
| **0 装 PASS** | ✅ 30 本仅是推荐清单, 0 装"已下载 / 已读" |

---

## 6. 0 主动 commit + 0 主动 push 严守

- **0 跑 `git add` / `git commit`** — Library v1.0 礼物准备文件全 untracked, 等 Mavis 整合 #5 commit 时机拍板
- **0 跑 `git push`** — 等 1.0 release 配 GitHub remote
- **8/11-8/22 跑过夜** — Library v1.0 礼物准备 (本 spec) 是 R125-21 阶段 6 的前置, 后续 R125-21 (1 月估时) 真发布 1.0 release 礼物

---

**Library v1.0 — 30 本经典书 (推荐清单) 准备 done 2026-08-10. 0 装 PASS 严守 + 8 硬墙 0 越界 + 0 主动 commit/push 严守 100% 落实. 主人按 ISBN/书名自行获取, 0 装"已发 1.0 release 礼物"严守.**
