# _BORROW_IDS — R125-21 借鉴 ID 索引 (200 资源)

> **Date**: 2026-08-10
> **Author**: R125-21 sub-agent (Mavis 派, per 决策 #51 §1.4 P3-4)
> **任务**: Library v1.0 1.0 release 礼物 (per `library-upgrade-plan-2026-08-10.md` §2 阶段 6)
> **借鉴源码**: obra/superpowers 234 files cloned (per 决策 #36 §1.1 + 决策 #41 §1)
> **借鉴路径**: `.openclaw/workspace/borrowed-repos/superpowers/`

---

## 0. 一句话

**R125-21 唯一主借鉴 ID = `R125-21-BORROW-obra/superpowers-2026-05-2026-08-10` (superpowers 公开 SKILL.md 4 字段 frontmatter 1:1 映射), 引用 R125-15 借鉴 ID (a/b/c/d/e/f 6 大类) 0 重写 0 重派. 0 装 PASS 严守, 8 硬墙 0 越界, 0 主动 commit + 0 主动 push 严守.**

---

## 1. R125-21 主借鉴 ID (1 个)

| 借鉴 ID | 任务 | 借鉴动作 | owner/repo | hash | 日期 | 状态 |
|---|---|---|---|---|---|---|
| `R125-21-BORROW-obra/superpowers-2026-05-2026-08-10` | R125-21 升级 (Library v1.0 1.0 release 礼物) | BORROW | obra/superpowers | 2026-05 (May 2026 release) | 2026-08-10 | ✅ 真实施 (30 经典书 SKILL.md + 5 大类 README + INDEX.json 1:1 映射) |

**借鉴范围** (per R125-15e final 报告 §1.3 已 lock 边界):
- ✅ 借鉴: 公开 `SKILL.md` 4 字段 frontmatter (name/description) + body 5 段 (Overview/When to Use/Steps/Iron Law/References) 1:1 映射
- ❌ 0 借鉴: superpowers 私有 plugin 加载机制 (`.claude-plugin/`, `.codex-plugin/`, `hooks.json`, `marketplace.json`)
- ❌ 0 借鉴: superpowers 14 skill 原文 (R125-21 写自己的 30 经典书, 0 抄 superpowers 14 skill)

**唯一性 verify**: 跟 R125-14 + R125-15e 借鉴 ID 同格式 (obra/superpowers), 但任务 ID 不同 (R125-21 vs R125-14/R125-15e), 0 冲突.

---

## 2. R125-21 引用 R125-15 借鉴 ID (6 个, 0 重写 0 重派)

| R125-15 借鉴 ID 格式 | 来源任务 | 状态 (R125-21 引用) | 引用方式 |
|---|---|---|---|
| `R125-15a-BORROW-arxiv-{arxiv_id}-{hash}-2026-08-10` | R125-15a (arxiv 30) | ⏳ 准备 (整合 #4 commit done, 30 metadata + 抓取脚本 stub) | 02-papers-research/README.md §2 引用 0 重写 |
| `R125-15b-BORROW-rfc-{rfc_num}-{hash}-2026-08-10` | R125-15b (RFC 20) | ✅ 真实施 (整合 #4 commit done, 20/20 真 ID) | 02-papers-research/README.md §3 引用 0 重写 |
| `R125-15c-BORROW-blog-{name}-{hash}-2026-08-10` | R125-15c (博客 19) | ✅ 真实施 (整合 #4 commit done, 19/15 真 URL 127%) | 02-papers-research/README.md §4 引用 0 重写 |
| `R125-15d-BORROW-video-{title}-{hash}-2026-08-10` | R125-15d (视频 15) | ⏳ 准备 (整合 #4 commit done, 15 metadata + 字幕 stub) | 03-videos-talks/README.md 引用 0 重写 |
| `R125-15e-BORROW-community-{name}-{hash}-2026-08-10` | R125-15e (社区 10) | ✅ 真实施 (P0-1 整合 #4 commit done, 14 Skill .md + SkillRegistry) | 04-communities/README.md 引用 0 重写 |
| `R125-15f-BORROW-hub-{name}-{hash}-2026-08-10` | R125-15f (hub 10) | ⏳ 跑中 (NEW P0-2 bg_16a97b77-..., 0 重写 0 重派) | 05-hubs/README.md 引用 0 重写 |

**0 装 PASS 严守 2 大原则**:
- ❌ 0 装"已写 R125-15 调研" — 实际 R125-21 引用 R125-15 真实产物, 0 重写
- ✅ 诚实标 R125-15 借鉴 ID 格式 + 整合 #4 commit 状态

**Mavis 5 min tick 监督** (per 决策 #52 cron_name `watch-r126-16-sub-agents-20-25`):
- R125-15f P0-2 跑中 → 5 min tick 监督
- P0-2 done → 引用 final 报告 (`agent-r125-15f-final-2026-08-10.md`)
- 0 重写 0 重派 (per 决策 #52)

---

## 3. 借鉴 superpowers 公开 SKILL.md 4 字段 (1:1)

**superpowers 公开 SKILL.md frontmatter** (per `borrowed-repos/superpowers/skills/*/SKILL.md`):
```markdown
---
name: {skill-id}                  # kebab-case
description: "{when to use}"      # 严格限定
---
# Title
## Overview (1 段)
## When to Use (3-5 行)
## Steps / Process (3-7 步)
## Iron Law / Hard Gate (1 段)
## References (5-7 项)
```

**R125-21 1:1 映射**:
- `name` 格式: `book-{organ}-{title-kebab-case}` (例: `book-heart-mans-search-for-meaning`)
- `description` 格式: "Use when designing {organ} in Apeireth — {title} gives the {哲学/心理学/算法} foundation for {organ} {dimension}"
- Body 5 段 1:1 (Overview / When to Use / Key Takeaways / Apply to Apeireth / Iron Law / References)

**0 装"已借鉴" 严守**:
- ❌ 0 抄 superpowers 14 skill 原文 (R125-21 写自己的 30 经典书)
- ❌ 0 抄 superpowers 私有 plugin 加载机制 (`.claude-plugin/`, `.codex-plugin/`, `hooks.json`, `marketplace.json`)
- ❌ 0 装"已抄" superpowers 完整 SKILL.md 完整 4 段结构
- ✅ 1:1 映射 4 字段 frontmatter + 5 段 body (公开模式, 0 私有 fn)

---

## 4. 借鉴 ID 唯一性 verify (跟 R125-15e + R125-14 比对)

| 借鉴 ID | 任务 | 状态 | 0 冲突 verify |
|---|---|---|---|
| `R124-2-BORROW-obra/superpowers-2026-05-2026-08-10` | R124-2 大类其他 sub-agent (aGLM/chidori) | ⏳ 准备 | ✅ 0 冲突 (任务 ID 不同 R124-2 vs R125-21) |
| `R125-14-BORROW-obra/superpowers-2026-05-2026-08-10` | R125-14 obra/superpowers Skill (8 文件 ~80KB + 79/79) | ⏳ 准备 | ✅ 0 冲突 (任务 ID 不同 R125-14 vs R125-21) |
| `R125-15e-BORROW-obra/superpowers-2026-05-2026-08-10` | R125-15e 升级 P0-1 (14 Skill .md + SkillRegistry) | ✅ 真实施 (整合 #4 commit done) | ✅ 0 冲突 (任务 ID 不同 R125-15e vs R125-21) |
| `R125-21-BORROW-obra/superpowers-2026-05-2026-08-10` | R125-21 升级 (本任务, Library v1.0 1.0 release 礼物) | ✅ 真实施 (R125-21 30 经典书 SKILL.md) | ✅ 唯一 (本任务) |

**总 4 个 R*-BORROW-obra/superpowers 借鉴 ID 唯一, 0 冲突.**

---

## 5. R125-21 0 装 PASS 严守 总结

| 状态 | 借鉴 | R125-21 任务 |
|---|---|---|
| ✅ cloned = 真实施 | superpowers 234 files (per 决策 #36 §1.1) | R125-21 30 经典书 SKILL.md + 5 大类 README + INDEX.json 1:1 映射公开 SKILL.md 4 字段 |
| ⏳ 限流 = 准备 | arxiv 30 (R125-15a 0 抓) + 视频 50 (R125-15d 0 抓) | 0 装"已抓", 引用 R125-15 真实产物 |
| ❌ 跳过 = 0 集成 | OpenCog AGPL-3.0 | 0 集成 (跟 R125-21 无关) |

**0 装"已借鉴" 严守**:
- ❌ 0 抄 superpowers 私有 plugin 加载机制
- ❌ 0 抄 superpowers 14 skill 原文
- ✅ 1:1 映射公开 SKILL.md 4 字段 (公开模式, 0 私有 fn)
- ✅ 诚实标借鉴 ID + 借鉴源码路径

---

## 6. 8 硬墙 0 越界 verify

跟 `library/v1.0/README.md` §6 1:1 严守. 0 越界 8 硬墙 (B2 1.2.0 / A1 baseline 3 值 / B1 24 LOCKED / B5 8 哲学锚 / B3 V0.5 / B4 6 重守门 v6 / A3 13 键 / C1 0 commit / C2 0 装 PASS / C3 v6 / 0 push).

---

## 7. 关联决策 + 报告

- 决策 #36 §1.1 (superpowers ✅ cloned 234 files)
- 决策 #41 §1 (R125-15a/b/c/d/e + R125-14 + R125-15e 借鉴 ID 唯一)
- 决策 #48 (整合 #4 commit abf12243 done)
- 决策 #51 §1.4 P3-4 (R125-21 升级 = 本任务)
- 决策 #52 (16 sub-agent 派活, R125-15f P0-2 跑中)
- 报告 `agent-r125-15e-final-2026-08-10.md` (R125-15e 借鉴 ID 格式)
- 报告 `library-upgrade-plan-2026-08-10.md` §2 阶段 6 (R125-21 spec)
