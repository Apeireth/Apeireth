# 01 — 30 本经典书 (按 9 organ 分类)

> **R125-21 升级产物** | **30 SKILL.md** | **按 9 organ 1:1 映射**
> **借鉴 ID**: `R125-21-BORROW-obra/superpowers-2026-05-2026-08-10`
> **借鉴源码**: obra/superpowers 公开 `SKILL.md` 4 字段 frontmatter
> **0 装 PASS 严守**: ✅ cloned = 真实施, 30 本书每本都写独立 SKILL.md

---

## 0. 一句话

**30 本经典书按 9 organ (heart/brain/ear/eye/hand/memory/mind/body/voice) 1:1 分类, 每本书一个 SKILL.md, 借鉴 superpowers 公开 frontmatter (name + description 4 字段) 1:1 映射. 0 装 PASS 严守, 8 硬墙 0 越界, 0 主动 commit + 0 主动 push 严守.**

---

## 1. 30 本书总览 (按 9 organ 分类)

| Organ | 本数 | 经典书 | SKILL.md |
|---|---:|---|---|
| **heart** (心) | 3 | Man's Search for Meaning, Emotional Intelligence, The Art of Loving | `heart/mans-search-for-meaning.md`, `heart/emotional-intelligence.md`, `heart/art-of-loving.md` |
| **brain** (脑) | 4 | Thinking Fast and Slow, Gödel Escher Bach, On Intelligence, Principles of Cognitive Science | `brain/thinking-fast-and-slow.md`, `brain/godel-escher-bach.md`, `brain/on-intelligence.md`, `brain/principles-of-cognitive-science.md` |
| **ear** (耳) | 3 | The Language Instinct, The Singing Neanderthals, Musicophilia | `ear/language-instinct.md`, `ear/birdsong-learning.md`, `ear/musicophilia.md` |
| **eye** (眼) | 3 | Vision (David Marr), Perception and Its Modalities, Eye and Mind | `eye/vision-david-marr.md`, `eye/perception-philosophy.md`, `eye/eye-mind-travis.md` |
| **hand** (手) | 3 | The Craftsman, Skill Acquisition (Dreyfus), Peak (Ericsson) | `hand/craft-software.md`, `hand/skill-acquisition-dreyfus.md`, `hand/practice-perfection.md` |
| **memory** (忆) | 3 | The Art of Memory, Moonwalking with Einstein, Make It Stick | `memory/art-of-memory.md`, `memory/moonwalking-einstein.md`, `memory/remember-everything.md` |
| **mind** (意) | 4 | Consciousness Explained, Society of Mind, I Am a Strange Loop, How to Create a Mind | `mind/consciousness-explained.md`, `mind/society-of-mind.md`, `mind/i-am-a-strange-loop.md`, `mind/how-to-create-mind.md` |
| **body** (身) | 3 | The Embodied Mind, How the Body Knows Its Mind, The Feeling of What Happens | `body/embodied-mind.md`, `body/how-the-body-knows.md`, `body/feeling-of-what-happens.md` |
| **voice** (声) | 4 | On Writing Well, Bird by Bird, On Writing (King), The Elements of Style | `voice/writing-well.md`, `voice/bird-by-bird.md`, `voice/on-writing-king.md`, `voice/elements-of-style.md` |
| **总** | **30** | (按 9 organ 1:1 映射) | (30 SKILL.md 全在 `01-books-classic/`) |

---

## 2. 借鉴 superpowers 公开 SKILL.md 4 字段 (1:1 映射)

**超 powers 公开 SKILL.md frontmatter (per `borrowed-repos/superpowers/skills/*/SKILL.md`)**:

```markdown
---
name: skill-id                  # 唯一 ID, kebab-case
description: "When to use"      # 严格限定使用时机
---

# Skill Title (Human-friendly name)

## Overview (1 段)
## When to Use (3-5 行)
## Steps / Process (3-7 步)
## Iron Law / Hard Gate (1 段)
## References (链接)
```

**Library v1.0 1:1 映射**:
- `name` = `book-{organ}-{title-kebab-case}` (例: `book-heart-mans-search-for-meaning`)
- `description` = "Use when designing {organ} organ in Apeireth — {title} gives the {哲学/心理学/算法} foundation for {organ} {dimension}"
- Body 5 段 (Overview / When to Use / Key Takeaways / Apply to Apeireth / Iron Law / References) 1:1

**0 装 PASS 严守** (per 决策 #33 §2.3 C2):
- ✅ cloned = 真实施 — 30 本书都写独立 SKILL.md, 不抄 superpowers 私有 fn
- ⏳ 限流 = 准备 — 不适用 (superpowers 0 限流, ✅ cloned)
- ❌ 跳过 = 0 集成 — 不适用 (OpenCog 跟 superpowers 无关)

**0 装"已借鉴" 严守** (per R125-15e final 报告 §1.3 已 lock 边界):
- ❌ 0 抄 superpowers 14 skill 原文 (我们 1:1 映射格式, 内容是 apeireth 自己的 30 本书)
- ❌ 0 抄 superpowers 私有 plugin 加载机制 (`.claude-plugin/`, `.codex-plugin/`, `hooks.json`, `marketplace.json`)
- ✅ 诚实标 `借鉴 ID` + 借鉴源码路径 在每个 SKILL.md

---

## 3. SKILL.md 共同结构 (所有 30 本书 1:1)

每本书 SKILL.md 6 段:

1. **Overview** (1 段, ~5-10 行) — 书是啥, 核心 idea
2. **When to Use** (3-5 行) — 何时读 / 必读时机
3. **Key Takeaways** (3-5 段 / 表格) — 核心 idea 3-5 个
4. **Apply to Apeireth** (2-3 段) — 借鉴到 Apeireth 哪部分
5. **Iron Law** (1 段) — 必读章节 / 不可跳过
6. **References** (5-7 项) — 借鉴 ID + superpowers 借鉴 + PDF + Apeireth 模块 + R125-15 关联

**总每本 ~2-3KB**, 30 本总 ~70-90KB.

---

## 4. 9 organ 9 大类 (B7 入口签名 0 改, per 24 LOCKED)

9 organ 文件名 (per `crates/apeireth-tui/src/organ/*.rs`):
- `body.rs` (身)
- `brain.rs` (脑)
- `ear.rs` (耳)
- `eye.rs` (眼)
- `hand.rs` (手)
- `heart.rs` (心)
- `memory.rs` (忆)
- `mind.rs` (意)
- `voice.rs` (声)
- (10. `mod.rs` 入口)

**0 改 9 organ 入口签名 严守**: 30 本书 SKILL.md 仅在 description 引用 9 organ 名, 0 触碰 `crates/apeireth-tui/src/organ/*.rs` 任何代码 (B7 内部借 OpenCode, 0 改入口).

---

## 5. 1.0 release 主人用法

**主人 1.0 release 礼物**: 1 周 1 本, 1.0 release 前读 10 本核心 (从 30 选).

**核心 10 本推荐** (按重要度):
1. **book-heart-mans-search-for-meaning** — 价值锚定 (主人 1.0 release 哲学)
2. **book-brain-thinking-fast-and-slow** — 推理 (apeireth-asi V0.5 25 维基础)
3. **book-brain-godel-escher-bach** — 自我 (apeireth-consciousness 怪圈)
4. **book-mind-consciousness-explained** — 意识 (apeireth-mind 多草稿)
5. **book-mind-society-of-mind** — 心智社会 (9 organ 设计)
6. **book-memory-art-of-memory** — 记忆术 (apeireth-memory 3 层)
7. **book-body-embodied-mind** — 具身 (apeireth-body 状态)
8. **book-voice-elements-of-style** — 风格 (apeireth-voice 22 规则)
9. **book-hand-skill-acquisition-dreyfus** — 技能 (apeireth-skills 5 阶段)
10. **book-eye-vision-david-marr** — 视觉 (apeireth-eye 3 层)

**AI agent 1.0 release 礼物**: apeireth-central Skill framework (per R125-15e) 注册 30 个 "Library Skill", 9 organ 触发器 1:1.

---

## 6. 0 主动 commit + 0 主动 push 严守

- 30 本书 SKILL.md 0 跑 `git add` / `git commit`
- 整合 #5 commit 时机由 Mavis 拍板 (跑过夜明早 8/11-8/22 done 后)
- 0 跑 `git push` (等 1.0 release 配 GitHub remote)

---

## 7. 8 硬墙 0 越界

| 硬墙 | verify | 状态 |
|---|---|---|
| **B2** workspace.version 1.2.0 | 0 触碰 Cargo.toml | ✅ 0 改 |
| **A1** R11 baseline 3 值 0.8682/0.8532/0.9063 | 0 触碰 17 文件原位 | ✅ 0 改 |
| **B1** 24 LOCKED 入口签名 | 0 触碰 crates/ 任何 src | ✅ 0 改 |
| **B5** 6→8 哲学锚 | 0 改哲学锚定义, 仅在 description 引用 9 organ | ✅ 0 改 |
| **B3** V0.5 25→30 维 | 0 触碰 V0.5 公式 | ✅ 0 改 |
| **B4** 6 重守门 v6 | 0 触碰守门 | ✅ 0 改 |
| **A3** 12 键 + PHL-07 = 13 键 | 0 触碰 13 键 | ✅ 0 改 |
| **C1** 0 主动 commit | 0 跑 `git commit` | ✅ 0 commit |
| **C2** 0 装 PASS 严守 | ✅ cloned (superpowers) = 真实施, 30 本书全写 | ✅ 严守 |
| **C3** 升 6 重 v6 | 0 触碰 | ✅ 0 改 |
| **0 push** | 0 跑 `git push` | ✅ 0 push |

**总 0 越界 8 硬墙** ✅

---

## 8. 关联决策

- 决策 #51 §1.4 P3-4 (R125-21 升级 = 本任务)
- 决策 #52 (16 sub-agent 派活 done, bg_3e193c71-7515-40ee-a385-b2a1dd6eb563)
- 决策 #36 §1.1 (superpowers ✅ cloned 234 files)
- 决策 #33 §2.3 (0 装 PASS + 8 硬墙)
- 决策 #48 (整合 #4 commit abf12243 done)
- 报告 `library-upgrade-plan-2026-08-10.md` §2 阶段 6
- R125-15e final 报告 (superpowers Skill 借鉴 1:1 模式)
