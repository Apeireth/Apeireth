# Apeireth 1.0 release 团队入职索引

```
[Document-Meta]
Document:       docs/1.0-release/team-onboarding.md
Version:        R20-Rev-A
R-Cycle:        R20 阶段 6 — 1.0 release 团队入职索引 (本目录引用)
Last-Modified:  2026-08-05
Status:         🟢 索引就绪 (1.0 release 收口)
Author:         Mavis (Mavis@local)
Originated:     主人 2026-08-05 22:13 拍板"只干 TUI,1.0 release 收口"
```

> **性质**: 1.0 release 团队入职的**索引文档**。本文档**不重写** `docs/team-onboarding.md` (5b27d041 LOCKED 估), 仅作为 1.0 release 收口目录 (`docs/1.0-release/`) 的入口索引, 任何接手者读此文档即可跳转至完整团队入职规范。
>
> **8 项不修改承诺**: 8 项详见 `docs/stage4/8-locked-unified-2026-08-05.md` §2
>
> **不假装**: 本文档是**索引**而非**重写**, 详细规范看源文档 `docs/team-onboarding.md`。

---

## §0. 跳转到完整团队入职文档

**主文档**: [`docs/team-onboarding.md`](../../team-onboarding.md) (5b27d041 commit, 187+ 行, 8 章节)

**何时用本文档 (本目录索引)**:
- 你正在 1.0 release 收口目录 (`docs/1.0-release/`) 工作
- 你需要快速跳转到团队入职规范
- 你需要知道 1.0 release 12 项 checklist 与团队入职的关联

**何时用源文档 (`docs/team-onboarding.md`)**:
- 你是新加入 Apeireth 项目的工程师 / 设计师 / AI 协作者
- 你需要完整的仓库结构 / 6 哲学 anchor / 8 项不修改承诺 / 团队节奏 / 接手检查清单
- 你需要详细的"任何人都能接手"规范 (per O-4)

---

## §1. 团队入职源文档 8 章节速查

源文档 `docs/team-onboarding.md` (5b27d041) 8 章节:

| § | 章节 | 关键 | 1.0 release 关联 |
|---:|------|------|------|
| 1 | Apeireth 是什么 | AGI 操作系统 Rust 重写, R14 阶段起, 立体架构 v2 + 生命架构 v4/v4.1, 17 crate 本源推导 + 双洋葱统一体, Self-Disable 防护 (apeireth-rollback 71GB 4 重防御) | 全局背景 |
| 2 | 仓库结构 | 58 crate 在 `crates/` (5 P0 MCP + 14 new R20 阶段 1 + 9 skeleton + 24 LOCKED + 6 杂项) | 全局背景 |
| 3 | 6 哲学 anchor | S-1 / S-2 / O-2 / O-3 / O-4 / O-5 (per `APEIRETH-CONVENTIONS.md` §9) | 12 项 checklist 穿透 |
| 4 | 8 项不修改承诺 | 7 LOCKED 文档 + workspace version 1.0.0 (per `8-locked-unified-2026-08-05.md` §2) | 12 项 checklist 严守 |
| 5 | 团队节奏 | R20 阶段 1-6 时间表, 11 主线 commit + 18 增量 commit | 1.0 release 时间表 |
| 6 | 接手检查清单 | 14 步接手流程 (读蓝图 → 读收官报告 → 跑 193 测试 → 跑 8 项承诺审计 → ...) | 1.0 release 验证 |
| 7 | 8 项承诺审计 | `scripts/audit/8-promise-audit.sh` (8 项实查) | #12 security |
| 8 | 关联文档 | `docs/release/` `docs/stage4/` `docs/ci/` `docs/security/` `docs/installation/` `docs/sdk/` `docs/api/` `docs/adr/` `docs/1.0-release/` | 1.0 release 全部依据 |

**详见**: [`docs/team-onboarding.md`](../../team-onboarding.md)

---

## §2. 1.0 release 12 项 checklist 与团队入职关联

| # | 12 项 | 团队入职关联 |
|---:|---|---|
| 1 | doc | §1 + §2 (Apeireth 是什么 + 仓库结构) |
| 2 | test | §6 接手检查清单 (跑 193 测试) |
| 3 | signature | §6 + §7 (cosign 验证 + 8 项承诺审计) |
| 4 | install | §6 (跑 8 包 dry-run) |
| 5 | upgrade | §6 (跑 D-07 迁移 dry-run) |
| 6 | uninstall | §6 (跑卸载 dry-run) |
| 7 | perf | §6 (跑 cargo bench baseline) |
| 8 | observability | §6 (跑 observability 3 端点) |
| 9 | ci | §6 + §7 (跑 3 workflow + 8 项承诺审计) |
| 10 | i18n | §6 (跑 i18n coverage) |
| 11 | license | §6 (验证 LICENSE + NOTICE + DEPENDENCY) |
| 12 | security | §7 (跑 cargo audit + cargo deny + 8 项承诺审计) |

---

## §3. 前端路线 (per 主人 2026-08-04 22:33 拍板)

- **TUI** — 现在 (过渡, 主人自己干)
- **Tauri 2.0** — 终极 (等设计团队到位)
- 1.0 release 收口**只干 TUI**, 不干前端 (per 主人 2026-08-05 22:13 拍板"只干 TUI")
- TUI 详细状态: `tui-status.md` (本目录)

**前端不在 1.0 release 范围**: 1.0 release 12 项 checklist **不**含 Tauri 2.0 desktop scaffold, Tauri 是 R20 阶段 5 估补项, 1.0 release 仅交付后端 + TUI (TUI 是 dev 自己干的"瘦客户端")。

---

## §4. 接手者速读路径 (per O-4 任何人都能接手)

新接手者按以下顺序读文档, 估 2 小时可上手 1.0 release 验证:

1. **`docs/1.0-release/README.md`** (本目录入口, 1.0 release 全貌, 5 min)
2. **`docs/1.0-release/checklist.md`** (12 项 checklist 100% 状态, 10 min)
3. **`docs/release/1.0.0-release-report-2026-08-05.md`** (R20-Rev-A 收官报告, 30 min)
4. **`docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md`** (604 行蓝图, 60 min)
5. **`docs/team-onboarding.md`** (5b27d041 团队入职, 187+ 行, 30 min)
6. **`docs/stage4/8-locked-unified-2026-08-05.md`** (8 项不修改承诺, 15 min)

合计: 估 2.5 小时完成 1.0 release 上下文建立。

---

## §5. 6 哲学 anchor 穿透

| 锚 | 本索引落地 |
|---|------|
| **S-1** ASI 完整性 | 跳转到 `docs/team-onboarding.md` 8 章节, 0 漏 |
| **S-2** 实事求是 | 本文档是**索引**而非**重写**, 不假装"另写一份" |
| **O-2** 走在前人肩上 | 复用 `docs/team-onboarding.md` (5b27d041 已有), 0 重写 |
| **O-3** 干到底 | 12 项 100% PASS 已在 `checklist.md` 落地, 索引 + 源文档全覆盖 |
| **O-4** 任何人都能接手 | §4 接手者速读路径 6 步 2.5 小时, 估可上手 |
| **O-5** 不假装 | §0 明确"本文档是索引而非重写", 不假装"另写一份团队规范" |

---

## §6. 8 项不修改承诺严守

| # | 项 | 本索引严守 |
|---|----|------|
| 1-7 | LOCKED 文档 | 0 改 (per `8-promise-audit.md` §2) |
| 8 | workspace version 1.0.0 | 0 改 `Cargo.toml` |
| 额外 | `docs/team-onboarding.md` (5b27d041) | **0 改**, 仅作源文档引用 |

**24 LOCKED crate src/**: 0 触碰 (per `8-promise-audit.md` §3)

---

## §7. 关联文档

- [`docs/team-onboarding.md`](../../team-onboarding.md) (5b27d041 源文档, LOCKED 估)
- [`docs/1.0-release/README.md`](./README.md) (本目录入口)
- [`docs/1.0-release/checklist.md`](./checklist.md) (12 项 checklist 100% 状态)
- [`docs/release/1.0.0-release-report-2026-08-05.md`](../../release/1.0.0-release-report-2026-08-05.md) (R20-Rev-A 收官报告)
- [`docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md`](../../stage4/v09021-rust-translation-blueprint-2026-08-05.md) (604 行蓝图)
- [`docs/stage4/8-locked-unified-2026-08-05.md`](../../stage4/8-locked-unified-2026-08-05.md) (8 项不修改承诺)
- [`docs/1.0-release/tui-status.md`](./tui-status.md) (TUI 5 nav + 9 器官 状态)
- `APEIRETH-CONVENTIONS.md` (顶层 3 规范文件, LOCKED)
- `APEIRETH-VERSIONING.md` (workspace version 1.0.0 严守, LOCKED)
- `APEIRETH-GLOSSARY.md` (术语表, LOCKED)

---

_本索引是 1.0 release 收口目录的**团队规范入口**, 任何接手者读本目录 → 跳转 `docs/team-onboarding.md` 即可知完整团队规范。等 Mavis 拍板 + 主人复核后, 由 Mavis 执行 git add + commit (不 push, 等 CI)。_
