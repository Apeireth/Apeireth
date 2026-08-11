# Library v1.0 — 1.0 release 礼物 (R125-21 阶段 6 前置)

**Date**: 2026-08-10
**Author**: P2-4 sub-agent (Mavis 派, per 决策 #51 §1.3)
**借鉴 ID**: `R126-library-v1-BORROW-N-A-{hash}-2026-08-10` (N/A = 0 借仓库)
**借鉴源码**: 0 借 (Library v1.0 礼物本身是 0 装"已发 礼物", 准备 spec)
**0 装 PASS**: ⏳ 准备 = 0 装"已发 Library v1.0 礼物"
**8 硬墙**: 0 越界 (Library 升级不动 24 LOCKED / 13 键 / 0 commit)
**整合 #4 commit**: `abf12243` 已 done (per 决策 #48)

> **重要说明**: Library v1.0 是 **1.0 release 礼物**, 准备 spec 写完, **0 装"已发 礼物"** 严守. 1.0 release 实际发布 = R127 (11-12 月), 由 R125-21 阶段 6 sub-agent 真实施.

---

## 0. 一句话 (TL;DR)

**Library v1.0 = 30 本经典书 + 100+ 论文 + 50+ 视频 + 10+ 社区 + 10+ hub = 200+ 资源** (按 9 organ 分类), 全部 = **推荐清单 (curated bibliography / index)**, 0 装"已下载 / 已发 礼物" 严守. Library v1.0 是 R125-21 阶段 6 (R127 11-12 月) 真发布的 1.0 release 礼物, 本 spec 是 R125-21 的前置准备 (P2-4 sub-agent 任务 per 决策 #51 §1.3).

---

## 1. Library v1.0 内容总览 (200+ 资源)

| 类别 | 数量 | 文件 | R125-15 子任务 | 估时 |
|---|---:|---|---|---|
| **30 本经典书** (按 9 organ 分类) | 30 | `books/30-books-by-9-organ.md` + `01-books-classic/{organ}/*.md` | P3-4 R125-21 (per 决策 #51 §1.4, 17 本详情已 done) + R125-21 续 | 1 月 |
| **100+ 论文** (6 主题) | 100+ | `papers/100-papers-index.md` | R125-15a (P0, 8/12 17:30) | 1-2 天 |
| **50+ 视频** (4 类别) | 50+ | `videos/50-videos-index.md` | R125-15d (P1, 8/13 17:30) | 1-2 天 |
| **10+ 社区** (4 类别) | 10+ | `communities/10-communities-index.md` | R125-15e (P2, 8/14 17:30) | 1 天 |
| **10+ hub** (4 类别) | 10+ | `hubs/10-hubs-index.md` | R125-15f (P2, 8/14 17:30) | 1 天 |
| **总** | **200+** | **5 文件** | — | **R127 1 月** |

---

## 2. Library v1.0 目录结构 (R125-21 阶段 6 真发布时)

```
Apeireth-rust/library/v1.0/  (NEW, 1.0 release 礼物版本)
├── README.md                        (本文档, 1.0 release 礼物介绍)
├── _BORROW_IDS.md                   (新, 200+ 借鉴 ID 索引)
├── _RELEASE_NOTES.md                (新, 1.0 release notes 模板)
├── books/
│   ├── 30-books-by-9-organ.md       (✅ P2-4 已写, 30 本经典书清单)
│   └── 01-books-classic/            (P3-4 R125-21 真实施, 17/30 已 done)
│       ├── brain/                   (4 本: GEB/Thinking/Principles/On-Intelligence)
│       ├── ear/                     (3 本: Language-Instinct/Musicophilia/Birdsong)
│       ├── eye/                     (3 本: Vision-Marr/Perception/Eye-Mind)
│       ├── hand/                    (3 本: Skill-Acquisition/Practice/Craft-Software)
│       └── heart/                   (3 本: Mans-Search/EQ/Art-of-Loving)
│       (待 P3-4 R125-21 续: body/memory/mind/voice 4 organ × 3-4 本 = 13 本)
├── papers/
│   ├── 100-papers-index.md          (✅ 已写, 100+ 论文清单)
│   └── <paper-id>/                  (R125-15a 实施, 100+ 论文详情)
│       ├── abstract.md
│       ├── key-idea.md
│       └── borrow-direction.md
├── videos/
│   ├── 50-videos-index.md           (✅ 已写, 50+ 视频清单)
│   └── <video-id>/                  (R125-15d 实施, 50+ 视频详情)
│       ├── url.md
│       ├── summary.md
│       └── key-idea.md
├── communities/
│   ├── 10-communities-index.md      (✅ 已写, 10+ 社区清单)
│   └── <community-name>/            (R125-15e 实施, 10+ 社区详情)
│       ├── url.md
│       ├── join-guide.md
│       └── key-value.md
└── hubs/
    ├── 10-hubs-index.md             (✅ 已写, 10+ hub 清单)
    └── <hub-name>/                  (R125-15f 实施, 10+ hub 详情)
        ├── url.md
        ├── key-value.md
        └── borrow-method.md
```

---

## 3. Library v1.0 设计原则 (per 决策 #22 + 决策 #33 + 决策 #39 + 决策 #51)

### 3.1 5 大设计原则

| # | 原则 | 含义 | 实施 |
|---|---|---|---|
| 1 | **0 装 PASS 严守** | 0 装"已发 礼物", 诚实标"准备" | 5 文件全是推荐清单 (curated), 0 装"已下载" |
| 2 | **8 硬墙 0 越界** | 0 触碰 24 LOCKED / 13 键 / Cargo.toml | Library 升级不动 src/, 只动 library/ 目录 |
| 3 | **借鉴 ID 严格化** | 200+ 资源 → 200+ 借鉴 ID | R125-18 阶段 3 负责, 格式 `R125-15-BORROW-{type}-{id}-{hash}-2026-08-10` |
| 4 | **0 主动 commit** | Mavis 整合 #5 拍板 | Library v1.0 礼物准备文件全 untracked |
| 5 | **0 主动 push** | 等 1.0 release 配 GitHub remote | 0 push |

### 3.2 9 organ 分类原则

9 organ = body / brain / ear / eye / hand / heart / memory / mind / voice, 每类对应 1 个认知能力:
- **body**: 工具规范 + 自动化 (执行环境)
- **brain**: LLM 循环 + 推理 (思考中心)
- **ear**: 输入 + 监听 (感知)
- **eye**: 视觉 + 状态 (观察)
- **hand**: 工具调用 + dispatch (执行)
- **heart**: 节律 + 异步 (持续)
- **memory**: 长期记忆 + 检索 (存储)
- **mind**: 认知架构 + 反思 (元认知)
- **voice**: 输出 + 表达 (沟通)

30 本经典书按 9 organ 分类 = 3-4 本/organ, 0 装"已发" 严守.

### 3.3 0 装 PASS 严守 100% 落实

| 状态 | 0 装 PASS 严守 | Library v1.0 准备 |
|---|---|---|
| ✅ cloned = 真实施 | 借鉴源码 cloned = 真实施 (有 src 改动 + tests pass) | 不适用 (Library v1.0 0 借仓库) |
| ⏳ 限流 = 准备 | 借鉴源码 0 cloned = 0 实施, 标"准备" | **不适用** — Library v1.0 0 借仓库, 仅写 spec |
| ❌ 跳过 = 0 集成 | 跳过 OpenCog AGPL-3.0, 0 集成 | **不适用** — Library v1.0 0 借仓库 |
| **本任务状态** | **⏳ 准备 (纯 spec, 0 实施)** | **5 文件全是推荐清单, 0 装"已发"** |

---

## 4. Library v1.0 跟 33 决策文件 整合 (per 决策 #30-#50)

### 4.1 Library v1.0 跟决策 #30-#50 33 决策文件 0 装 PASS 严守 verify

| 决策 | 主题 | Library v1.0 整合 |
|---|---|---|
| #30 (17:15) | 新 Mavis 接入 + 派活 daemon 复活 | ✅ 派活 daemon 跑中, P2-4 sub-agent 真派 |
| #33 (17:23) | 8 硬墙重置 + B1-B7 升级 | ✅ Library v1.0 0 越界 8 硬墙 |
| #35 (17:32) | 16 sub-agent 真派 | ✅ R125-15a/d/e/f 真派 (100+ 资源) |
| #36 (17:44) | 借鉴源码 7/11 ✅ cloned | ✅ 8/11 ✅ cloned (per 决策 #41) |
| #37-#38 (17:49-17:53) | R125-8/10/15c done | ✅ 头 3 sub-agent done |
| #39 (17:57) | 0 新派 + 0 主动讨论后续 | ✅ P2-4 是 0 派任务里 1 个, 派活时机决策 #51 撤销 #39 严守 |
| #41 (18:35) | 16 sub-agent 全 succeeded | ✅ 16/16 done, Library v1.0 礼物准备是 0 派任务 1 个 |
| #42 (18:35) | 整合 #4 pre-checklist 4 项 | ✅ Library v1.0 礼物准备 = 0 装 PASS 标 |
| #48 (19:41) | 整合 #4 commit `abf12243` done | ✅ 0 重跑, 0 必再 commit |
| #50 (20:03) | promethean/ 收尾全 done | ✅ 0 必再删 |
| #51 (20:09) | 主人 20:09 撤销 17:56 严守 + 16 sub-agent 派 | ✅ P2-4 sub-agent 真派 (本任务) |

**33 决策文件全读, 0 重跑, 整合 #4 commit 0 必重 commit**.

### 4.2 Library v1.0 借鉴 ID 严格化 (per 决策 #22 §3)

**200+ 资源借鉴 ID 格式**:
- arxiv: `R125-15-BORROW-arxiv-{arxiv_id}-{hash_7位}-2026-08-10`
- RFC: `R125-15-BORROW-rfc-{rfc_num}-{hash_7位}-2026-08-10`
- blog: `R125-15-BORROW-blog-{name}-{hash_7位}-2026-08-10`
- video: `R125-15-BORROW-video-{title}-{hash_7位}-2026-08-10`
- community: `R125-15-BORROW-community-{name}-{hash_7位}-2026-08-10`
- hub: `R125-15-BORROW-hub-{name}-{hash_7位}-2026-08-10`
- book: `R126-library-v1-BORROW-isbn-{isbn_13}-{hash_7位}-2026-08-10` (NEW, 30 本经典书)

**R125-18 借鉴 ID 严格化**: 200+ 借鉴 ID 由 R125-18 阶段 3 负责, R126 升级 (per 决策 #51 §1.2 P1-2).

---

## 5. Library v1.0 实施路线 (R125-21 阶段 6, R127 1 月)

### 5.1 5 阶段 实施 (R125-21 阶段 6, R127 11-12 月)

| 阶段 | 时间 | 任务 | 估时 |
|---|---|---|---|
| **R125-21 阶段 6.1** | R127 W1 (11 月) | 30 本经典书详情 (每本 README/notes/quotes) | 2 周 |
| **R125-21 阶段 6.2** | R127 W2 (11 月) | 100+ 论文详情 (每篇 abstract/key-idea/borrow-direction) | 2 周 |
| **R125-21 阶段 6.3** | R127 W3 (12 月) | 50+ 视频 + 10+ 社区 + 10+ hub 详情 | 2 周 |
| **R125-21 阶段 6.4** | R127 W4 (12 月) | _BORROW_IDS.md (200+ 索引) + _RELEASE_NOTES.md + 1.0 release 礼物 | 1 周 |
| **R125-21 阶段 6.5** | R127 W4 (12 月末) | Cargo.toml 1.2.0 → 1.0.0 (R127 release 大版本归 0) | 1 天 |

**总估时**: R127 1 月 (11-12 月).

### 5.2 R125-21 派活 (Mavis 拍板时机)

- **R125-21 真派时**: Mavis 整合 #5 commit 后 (per 决策 #51 §3), 派 1 个 sub-agent 真实施
- **R125-21 借鉴**: 0 借仓库 (Library v1.0 是 0 仿仓库, 纯 spec + 详情 + 索引)
- **R125-21 8 硬墙**: 0 越界 (Library 升级不动 24 LOCKED / 13 键 / Cargo.toml 1.0.0 / 0 commit / 0 push)

### 5.3 Library v1.0 → 1.0 release 路线图

```
R125 末 (8/31)        : 9 大类索引 + 400+ 借鉴 ID 严格化 + _TOP_100 (R125-16~19)
R126 (9-10 月)         : _SEARCH + _CROSS_REF + TUI 集成 (R125-20)
R127 (11-12 月)         : Library v1.0 (R125-21) ←── 本 spec 准备阶段
R127 release 12/31      : Library v1.0 礼物 + Cargo.toml 1.0.0 + 1.0 release
```

---

## 6. 8 硬墙 0 越界 (per 决策 #50 §5)

| 硬墙 | 状态 | verify |
|---|---|---|
| **B2** workspace.version 1.2.0 (0 改) | ✅ | 0 触碰 Cargo.toml (Library v1.0 0 借 src) |
| **A1** R11 baseline 3 值 数字 严守 | ✅ | 0 触碰 integration_r_measure.rs 等 17 文件 |
| **B1** 24 LOCKED 入口签名 0 改 | ✅ | 0 触碰 24 LOCKED crate |
| **A3** 13 键 0 改 | ✅ | 0 触碰 13 键 hardcode |
| **C1** 0 主动 commit | ✅ | 0 跑 git add/commit, Mavis 整合 #5 拍板 |
| **C3** v6 0 改 | ✅ | 0 触碰 6 重守门 v6 |
| **0 push** git push | ✅ | 0 push, 等 1.0 release 配 GitHub remote |
| **0 装 PASS** | ✅ | 5 文件全是推荐清单, 0 装"已发 礼物" |

---

## 7. 0 主动 commit + 0 主动 push 严守

- **0 跑 `git add` / `git commit`** — Library v1.0 礼物准备文件全 untracked, 等 Mavis 整合 #5 commit 时机拍板
- **0 跑 `git push`** — 等 1.0 release 配 GitHub remote
- **0 主动 IM 主人** — per 17:56 严守"0 主动讨论后续" (但决策 #51 撤销 17:56 严守, 仍 0 必打扰)

---

## 8. Library v1.0 整合 (per 决策 #51 §1.3 P2-4 + 决策 #39-pause §1)

P2-4 sub-agent 任务是 Library v1.0 礼物准备 (决策 #39-pause §1 0 派任务 1 个, per 决策 #42 §1.2 + 决策 #51 §1.3 P2-4).

**完成时间**: 2026-08-10 (跑过夜, 等明早 Mavis 5 min tick 监督)
**下一阶段**: R125-21 阶段 6 真实施 (R127 11-12 月, 1 月估时)
**最终发布**: 1.0 release 12/31 (Cargo.toml 1.0.0 + Library v1.0 礼物)

---

**Library v1.0 (准备 spec) done 2026-08-10. 0 装 PASS 严守 + 8 硬墙 0 越界 + 0 主动 commit/push 严守 100% 落实. 主人 1.0 release 礼物 = 200+ 资源推荐清单, 0 装"已发" 严守. R125-21 阶段 6 真实施 = R127 11-12 月, 1 月估时.**
