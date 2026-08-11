# _CLASSIFICATION — Library v1.0 9 organ + 5 大类 分类说明

> **Date**: 2026-08-10
> **Author**: R125-21 sub-agent (Mavis 派, per 决策 #51 §1.4 P3-4)

---

## 0. 一句话

**Library v1.0 双重分类: 9 organ 1:1 (30 经典书按 9 organ 分类) + 5 大类 1:1 (200 资源按类型分类). 9 organ 0 改 LOCKED (B7 内部借, 0 改入口签名), 5 大类 0 改 LOCKED (R125-15 6 大类 0 重分类).**

---

## 1. 9 organ 分类 (B7 0 改 LOCKED)

**9 organ 1:1 映射** (per `crates/apeireth-tui/src/organ/*.rs` 9 文件, 24 LOCKED 之外但 0 改入口签名):
- **body** (身) — 生理/具身/状态
- **brain** (脑) — 推理/认知/思考
- **ear** (耳) — 听觉/语言
- **eye** (眼) — 视觉/感知
- **hand** (手) — 行动/技能/操作
- **heart** (心) — 情感/价值/意义
- **memory** (忆) — 学习/记忆/经验
- **mind** (意) — 意识/自我/反思
- **voice** (声) — 表达/创造/输出
- (10. **mod.rs** 入口)

**B7 0 改 9 organ 入口签名 严守**: R125-21 0 触碰 `crates/apeireth-tui/src/organ/*.rs` 任何代码, 仅在 SKILL.md description 引用 9 organ 名.

---

## 2. 5 大类分类 (per 决策 #24 R125-15 6 大类 0 重分类)

| 大类 | 数量 | 路径 | 借鉴 | 状态 |
|---|---:|---|---|---|
| **01-books-classic** | 30 | `01-books-classic/` | R125-21 真写 30 经典书 SKILL.md | ✅ 30/30 done |
| **02-papers-research** | 100 | `02-papers-research/` | R125-15a 30 arxiv 准备 + R125-15b 20 RFC 真 + R125-15c 19 博客 真 + 31 stub | ✅/⏳ mixed |
| **03-videos-talks** | 50 | `03-videos-talks/` | R125-15d 15 metadata 准备 + 35 stub | ⏳ 0/50 done |
| **04-communities** | 10 | `04-communities/` | R125-15e ✅ done P0-1 整合 #4 commit | ✅ 10/10 done |
| **05-hubs** | 10 | `05-hubs/` | R125-15f NEW P0-2 跑中 | ⏳ 0/10 done |
| **总** | **200** | (5 大类 5 路径) | (5 类 5 来源) | **mixed** |

---

## 3. 双重分类映射 (9 organ × 5 大类)

| 9 organ 触发 | 30 books 1:1 | 100 papers 引用 | 50 videos 引用 | 10 communities 引用 | 10 hubs 引用 |
|---|---|---|---|---|---|
| **heart** (心) | 3 (mans-search/emotional-intelligence/art-of-loving) | 0-1 (Apeireth 价值论文) | 0-1 (主人价值类视频) | 1-2 (Hacker News / LessWrong) | 0-1 (Papers with Code) |
| **brain** (脑) | 4 (thinking-fast/godel/on-intelligence/principles) | 5-10 (AI Agent 架构 + 认知架构 + 形式化验证 + AGI 评估) | 5-10 (顶会 main track + 教育) | 2-3 (Reddit r/MachineLearning / EleutherAI) | 2-3 (OpenReview / arXiv-sanity) |
| **ear** (耳) | 3 (language-instinct/singing-neanderthals/musicophilia) | 1-2 (协议层 + LLM 服务) | 1-2 (Podcast + 教育) | 1 (Discord LangChain) | 1 (Hugging Face Models) |
| **eye** (眼) | 3 (vision-david-marr/perception-philosophy/eye-mind-travis) | 1-2 (CV arxiv 论文) | 1-2 (教育视频) | 0-1 (Reddit r/LocalLLaMA) | 0-1 (Replicate) |
| **hand** (手) | 3 (craft-software/skill-acquisition/practice-perfection) | 1-2 (工程化 RFC) | 0-1 (Podcast) | 0-1 (Discord maker) | 0-1 (Together AI) |
| **memory** (忆) | 3 (art-of-memory/moonwalking/remember-everything) | 1-2 (memory 论文) | 0-1 (教育) | 0-1 (学术社区) | 0-1 (Connected Papers) |
| **mind** (意) | 4 (consciousness-explained/society-of-mind/strange-loop/how-to-create-mind) | 1-2 (AGI 评估) | 0-1 (AI Engineer Summit) | 0-1 (AI Alignment Forum) | 0-1 (Kaggle AgentBench) |
| **body** (身) | 3 (embodied-mind/how-the-body-knows/feeling-of-what-happens) | 0-1 (守门 / Safety) | 0-1 (教育) | 0-1 (Hacker News) | 0-1 (Artificial Analysis) |
| **voice** (声) | 4 (writing-well/bird-by-bird/on-writing-king/elements-of-style) | 0-1 (工程化 RFC) | 0-1 (教育) | 0-1 (Hacker News) | 0-1 (OpenReview) |
| **总 (估)** | **30** | **~30** | **~15** | **~10** | **~10** |

**0 必 1:1 严格, 9 organ × 5 大类 仅示意, 实际触发器按 9 organ + 5 大类 双重命中**.

---

## 4. 1.0 release 主人用法

**主人按 9 organ 找书**:
1. 主人思考"AI 跟人价值" → 翻 `01-books-classic/heart/` 3 本
2. 主人思考"AI 推理" → 翻 `01-books-classic/brain/` 4 本
3. 主人思考"AI 自我" → 翻 `01-books-classic/mind/` 4 本
4. 主人思考"AI 长期记忆" → 翻 `01-books-classic/memory/` 3 本
5. 主人思考"AI 写代码" → 翻 `01-books-classic/voice/` 4 本

**主人按 5 大类找材料**:
1. 主人想读论文 → 翻 `02-papers-research/` 100 (含 5 大类子目录)
2. 主人想看视频 → 翻 `03-videos-talks/` 50
3. 主人想加社区 → 翻 `04-communities/` 10
4. 主人想用 hub → 翻 `05-hubs/` 10

**AI agent 1.0 release 触发器**:
- 9 organ 触发器 1:1 → 30 books Skill
- 5 大类触发器 1:1 → 200 资源 Skill

---

## 5. 0 改 LOCKED verify

| LOCKED | 严守 | R125-21 触碰 |
|---|---|---|
| **24 LOCKED crate mtime 16:34 之前** (per `docs/omnibus/24-locked-crates.md`) | ✅ 0 改 (整合 #4 commit done) | 0 触碰 (Library 资料库 0 涉及 src) |
| **9 organ 文件名 + 入口签名** (per `docs/omnibus/24-locked-crates.md` §9 organ) | ✅ 0 改 | 0 触碰 (仅在 description 引用名) |
| **8 LOCKED 文档** (per `docs/omnibus/24-locked-crates.md` §8 LOCKED 文档) | ✅ 0 改 | 0 触碰 |
| **R125-15 6 大类** (per `r125-15-non-github-resources.md`) | ✅ 0 重分类 | 0 重写 0 重派 (引用 R125-15 真实产物) |

**总 0 改 LOCKED 9 项 (24 + 9 + 8 + 6)** ✅

---

## 6. 关联决策 + 报告

- 决策 #24 (R125-15 6 大类 spec)
- 决策 #33 §2.3 (8 硬墙 + 0 装 PASS)
- 决策 #36 §1.1 (superpowers ✅ cloned 234 files)
- 决策 #41 §1 (R125-15 6 大类 借鉴 ID 唯一)
- 决策 #48 (整合 #4 commit abf12243 done)
- 决策 #51 §1.4 P3-4 (R125-21 升级 = 本任务)
- 报告 `library-upgrade-plan-2026-08-10.md` §2 阶段 6 (R125-21 spec)
- 报告 `r125-15-non-github-resources.md` (R125-15 6 大类 spec)
- 报告 `docs/omnibus/24-locked-crates.md` (24 + 9 + 8 = 41 LOCKED)
