# _SKILLS_INDEX — Library v1.0 全部 Skill 索引 (200 资源, 借鉴 superpowers)

> **Date**: 2026-08-10
> **Author**: R125-21 sub-agent (Mavis 派, per 决策 #51 §1.4 P3-4)
> **总**: 200 Library Skill (30 经典书 + 100 论文 + 50 视频 + 10 社区 + 10 hub)
> **借鉴 ID**: `R125-21-BORROW-obra/superpowers-2026-05-2026-08-10`

---

## 0. 一句话

**200 Library Skill 借鉴 superpowers 公开 `SKILL.md` 4 字段 frontmatter 1:1 映射, 9 organ 触发器 + 5 大类分类, 0 装 PASS 严守, 8 硬墙 0 越界, 0 主动 commit + 0 主动 push 严守.**

---

## 1. Library Skill 框架 (借鉴 superpowers 1:1)

**超 powers 公开 `SKILL.md` 4 字段 frontmatter** (per `borrowed-repos/superpowers/skills/*/SKILL.md`):
```markdown
---
name: {skill-id}                  # kebab-case 唯一
description: "{when to use}"      # 严格限定
---
# Title
## Overview (1 段)
## When to Use (3-5 行)
## Steps / Process (3-7 步)
## Iron Law / Hard Gate (1 段)
## References (5-7 项)
```

**Library v1.0 Skill 1:1 映射**:
- `name`: `{type}-{short-id}` (例: `book-heart-mans-search-for-meaning`, `paper-arxiv-2607-00151`)
- `description`: "Use when designing {organ} in Apeireth — {title} gives the {哲学/算法/政策/视频/社区/hub} 基础"
- Body 5 段 1:1 (Overview / When to Use / Key Takeaways / Apply to Apeireth / Iron Law / References)

---

## 2. 200 Library Skill 总览

| 类型 | 数量 | Skill 名称前缀 | 路径 | 状态 |
|---|---:|---|---|---|
| **books** | 30 | `book-{organ}-{title}` | `01-books-classic/{organ}/` | ✅ 30/30 done (R125-21 真写) |
| **papers** | 100 | `paper-arxiv-{id}` + `rfc-{num}` + `blog-{name}` | `02-papers-research/{arxiv,rfc,blogs}/` | ✅/⏳ mixed (R125-15b/c 39 真, R125-15a 30 准备, 31 stub) |
| **videos** | 50 | `video-{short-id}` | `03-videos-talks/{conferences,ai-engineer,podcasts,education}/` | ⏳ 0/50 done (R125-15d 准备) |
| **communities** | 10 | `community-{name}` | `04-communities/{tech-news,discord,academic,ml-twitter}/` | ✅ 10/10 done (R125-15e 整合 #4 commit) |
| **hubs** | 10 | `hub-{name}` | `05-hubs/{model-hubs,data-bench,academic,benchmark}/` | ⏳ 0/10 done (R125-15f P0-2 跑中) |
| **总** | **200** | (5 类 5 前缀) | (5 大类 5 路径) | **mixed** |

---

## 3. 9 organ 触发器 (30 books, 1:1 1 organ N books)

| Organ | 触发器 (skill 1:1) | 描述 (Apeireth 1:1) |
|---|---|---|
| **heart** | book-heart-{3 books} | 当 apeireth-central 设计 heart organ / 处理 9 organ 价值冲突 / 写 agent "为什么做这件事" 时 |
| **brain** | book-brain-{4 books} | 当 apeireth-central 设计 brain organ / 写推理 verdict cache / 处理 sub-agent "想当然" 错误 时 |
| **ear** | book-ear-{3 books} | 当 apeireth-central 设计 ear organ / 处理多模态输入 / 写 agent "我听懂主人话" 时 |
| **eye** | book-eye-{3 books} | 当 apeireth-central 设计 eye organ / 处理 TUI 屏幕渲染 / 多模态图像输入时 |
| **hand** | book-hand-{3 books} | 当 apeireth-central 设计 hand organ / 处理工具使用 / 写 agent "我用什么工具" 时 |
| **memory** | book-memory-{3 books} | 当 apeireth-central 设计 memory organ / 处理长程 AI 成长 / 写 agent "我这样记" 时 |
| **mind** | book-mind-{4 books} | 当 apeireth-central 设计 mind organ / 处理自我模型 / 主人思考 AI 意识时 |
| **body** | book-body-{3 books} | 当 apeireth-central 设计 body organ / 处理压力管理 / 写 agent "我这样感知身体" 时 |
| **voice** | book-voice-{4 books} | 当 apeireth-central 设计 voice organ / 处理 TUI 文字输出 / 写 agent "我这样写" 时 |

**总 9 organ × 3-4 books = 30 books = 30 Skill 1:1 触发器**.

---

## 4. Skill 注册表 (apeireth-central 1:1, per R125-15e 整合 #4 commit)

**R125-15e P0-1 整合 #4 commit done** (per 决策 #48 + R125-15e final 报告):
- `apeireth-central/src/skill_trait.rs` — Skill trait (id / name / when_to_use / steps) 4 字段 1:1 借鉴 superpowers
- `apeireth-central/src/skill_registry.rs` — SkillRegistry 中央注册 (`get(id) / all() / tdd_required(id)`)
- `apeireth-central/skills/{14 skills}.md` — 14 Skill .md (Brainstorming / TDD / SystematicDebugging / ...)
- 14 Skill .md 1:1 映射 superpowers 公开 `SKILL.md` 4 字段 frontmatter

**R125-21 升级方向 (1.0 release 后)**:
- 扩展 SkillRegistry 注册 200 Library Skill (30 books + 100 papers + 50 videos + 10 communities + 10 hubs)
- 9 organ 触发器 1:1 跟 30 books Skill 1:1 映射
- 5 大类触发器 1:1 跟 100 papers + 50 videos + 10 communities + 10 hubs 1:1 映射
- 主对话自动 invoke Library Skill 当 9 organ 触发器命中时

---

## 5. 200 Skill 1:1 映射 superpowers (借借鉴 ID)

**1:1 映射 superpowers 公开 `SKILL.md` 4 字段 frontmatter**:
- `name` (kebab-case) 1:1
- `description` (严格限定) 1:1
- `body` (5 段) 1:1

**0 装 PASS 严守**:
- ✅ cloned = 真实施 (R125-21 30 books 真写, 引用 R125-15 真实产物)
- ⏳ 限流 = 准备 (arxiv + 视频, 0 装"已抓")
- ❌ 跳过 = 0 集成 (OpenCog, 0 集成)

**0 装"已借鉴" 严守**:
- ❌ 0 抄 superpowers 14 skill 原文 (R125-21 写自己的 30 经典书)
- ❌ 0 抄 superpowers 私有 plugin 加载机制 (`.claude-plugin/`, `.codex-plugin/`, `hooks.json`, `marketplace.json`)
- ✅ 1:1 映射公开 SKILL.md 4 字段 (公开模式, 0 私有 fn)

---

## 6. 1.0 release 触发器 1:1 (主对话自动 invoke)

**主人 1.0 release 后, 主对话自动 invoke Library Skill 规则**:
- 主人说 "设计 heart" → auto invoke 3 book-heart-* Skill
- 主人说 "为什么做这个" → auto invoke book-heart-mans-search-for-meaning + book-heart-emotional-intelligence + book-heart-art-of-loving
- 主人说 "推理有 bug" → auto invoke book-brain-thinking-fast-and-slow + book-brain-on-intelligence
- 主人说 "代码工艺" → auto invoke book-hand-craft-software + book-hand-peak
- 主人说 "写代码" → auto invoke book-voice-elements-of-style + book-voice-on-writing-well + book-voice-on-writing-king
- 主人说 "AI 意识" → auto invoke book-mind-consciousness-explained + book-mind-society-of-mind + book-mind-i-am-a-strange-loop + book-mind-how-to-create-mind
- 主人说 "AI 长期记忆" → auto invoke book-memory-art-of-memory + book-memory-moonwalking-einstein + book-memory-make-it-stick
- 主人说 "AI 多模态" → auto invoke book-ear-language-instinct + book-eye-vision-david-marr + book-eye-perception-philosophy
- 主人说 "AI 具身" → auto invoke book-body-embodied-mind + book-body-how-the-body-knows + book-body-feeling-of-what-happens
- 主人说 "AI 跟人关系" → auto invoke book-heart-* + book-mind-society-of-mind + book-mind-strange-loop
- 主人说 "安全/合规" → auto invoke 100 papers 中 OWASP LLM Top 10 + NIST AI RMF + EU AI Act (per R125-15b 真实施)
- 主人说 "实时脉搏" → auto invoke 10 communities (per R125-15e 整合 #4 commit)
- 主人说 "实战材料" → auto invoke 10 hubs (等 R125-15f P0-2 done)

**0 必主人 0 主动查 Library**, 触发器自动 invoke.

---

## 7. 8 硬墙 0 越界 verify

跟 `library/v1.0/README.md` §6 1:1 严守. 0 越界 8 硬墙.

---

## 8. 关联决策 + 报告

- 决策 #36 §1.1 (superpowers ✅ cloned 234 files)
- 决策 #41 §1 P0-1 (R125-15e 14 Skill .md + SkillRegistry 整合 #4 commit)
- 决策 #48 (整合 #4 commit abf12243 done)
- 决策 #51 §1.4 P3-4 (R125-21 升级 = 本任务)
- 报告 `agent-r125-15e-final-2026-08-10.md` (R125-15e Skill framework 1:1)
- 报告 `library-upgrade-plan-2026-08-10.md` §2 阶段 6 (R125-21 spec)
