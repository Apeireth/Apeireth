# docs/release/ — 各 release 版本索引

```
[Document-Meta]
Document:       docs/release/README.md
Version:        R119-4a
R-Cycle:        R119 (文档体系推倒重建)
Last-Modified:  2026-08-10
Status:         🟢 索引层
```

> **性质**: 9 个 release 版本的索引层 (`<version>/CHANGELOG.md`)。每条索引只放版本信息 + 主题 + 关键数字 + 跳到详细报告, 内容严守单源 = `reports/<R-cycle>.md`。

---

## 9 release 索引

| 版本 | 日期 | 主题 | 索引 |
|---|---|---|---|
| **v1.0.0** | 2026-08-05 | R20 阶段 1-6 1.0 release 收口, 12 项 checklist 100% PASS | [`1.0.0/CHANGELOG.md`](1.0.0/CHANGELOG.md) |
| **v1.1.0** | 2026-08-09 | R38 1.1 RC 9 B-stage 一气呵成 | [`1.1.0/CHANGELOG.md`](1.1.0/CHANGELOG.md) |
| **v1.1.1** | 2026-08-09 | R46-R53 1.1.1 follow-up | [`1.1.1/CHANGELOG.md`](1.1.1/CHANGELOG.md) |
| **v1.1.2** | 2026-08-09 | R54 B8 续升级 (1.1.2 patch) | [`1.1.2/CHANGELOG.md`](1.1.2/CHANGELOG.md) |
| **v1.1.2-followup-2** | 2026-08-09 | R57-R62 1.1.2 follow-up-2 | [`1.1.2-followup-2/CHANGELOG.md`](1.1.2-followup-2/CHANGELOG.md) |
| **v1.2-candidate** | 2026-08-09 | R63-R68 1.2 candidate (未 release) | [`1.2-candidate/CHANGELOG.md`](1.2-candidate/CHANGELOG.md) |
| **v1.2-patch-LIVE** | 2026-08-09 | R70-R72 1.2 patch LIVE (LIVE MiniMax + MCP push) | [`1.2-patch-live/CHANGELOG.md`](1.2-patch-live/CHANGELOG.md) |
| **v1.2-patch-LIVE-续** | 2026-08-10 | R78-R113 1.2 patch LIVE 续 (12 R + 1 LIVE) | [`1.2-patch-live-followup/CHANGELOG.md`](1.2-patch-live-followup/CHANGELOG.md) |
| **v1.2-R114-118** | 2026-08-10 | R114-R118 动态运营层 (codex 5c546a84) | [`1.2-r114-r118/CHANGELOG.md`](1.2-r114-r118/CHANGELOG.md) |

---

## 顶层入口

- 顶层 [`../../CHANGELOG.md`](../../CHANGELOG.md) (2.7KB) 链这里
- 1.0 release 收口索引: [`../1.0-release/README.md`](../1.0-release/README.md) (13.5KB, 12 项 checklist 100% 收口)
- 1.1 release 索引: [`../1.1-release/README.md`](../1.1-release/README.md) (5.4KB, R38 9 B-stage)

---

## 何时看

- **找某个版本的细节** → 上面 9 索引任一个
- **找 R 周期报告** → 每个 CHANGELOG.md 内嵌 `reports/<R-cycle>.md` 链接
- **找最后检查** → [`../final-check/`](../final-check/) (R14 末 / R54 / R70-R72)

---

## 单源原则

每个 `<version>/CHANGELOG.md` 都是**索引层** (~1-2KB), 不重复 release notes / commit 详单 / 测试统计等具体内容, 全部严守在 `reports/<R-cycle>.md` 单源。

`1.0.0/` 子目录内 3 文件:
- `CHANGELOG.md` (1.5KB) — 索引层
- `release-report.md` (32KB) — R20 阶段 1-6 详细 commit 报告
- `release-notes.md` (4.7KB) — GitHub v1.0.0 release body 候选

> R119 形式撤销前, 顶层 `APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md` 427KB = docs/stage1-6/ + 3 个 architecture-v* + R11 baseline 三值 的合并版。R119-3b 把它下沉到 `docs/omnibus/` 7 索引, OMNIBUS 顶层文件已删。
