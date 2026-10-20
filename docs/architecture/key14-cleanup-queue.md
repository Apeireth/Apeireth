# 14 键清理台账（ADR-0006 · Wave 3 批 3）

> 规则：**现值文档**改「键集（现行 14 键）」；**带日期的历史卷宗保留原文**（必要时加复核注），不批量替换。
> 教训：批 3 初版曾用全仓 grep 批量替换，扫进 `library/`/`CHANGELOG`/历史规格等被中性扫描拦下并全量回滚——此后一律 **allowlist**。

## 已完成
- 批 1：7 份现值规格 / 23 处（AGENTS 现值行已先行修正）。
- 批 2：6 份 guides/reference / 21 处 + `TO-NEW-TEAM` 历史复核注。
- 批 3（本批 allowlist）：`SECURITY.md`（1 处）+ `docs/01-architecture/deferred-slot-activation-preference_learning-spec.md`（6 处）。

## allowlist 判定口径（本批）
- **改**：无日期、无 "首发/Last-Modified 2026-08" 等历史标记的现值规格/参考/安全文档。
- **不改**：`CHANGELOG.md`、带日期文件（`*20\d\d*.md`）、`docs/archive/`、`docs/design/`、`library/`、一次性审计/交接报告（如 `FINAL-HANDOFF*`、`HANDOFF-NOTES`、`engineering-report`、`*-research-*`）。
- **复核注**：仍在用但内容属历史快照的文档（如 `ENGINEER-MANIFESTO`、`TO-NEW-TEAM`）加注不改正文。

## 当前残留
- docs/ 现值口径（排除 archive/design）仍含「13 键」的文件：**91** 个（多属历史/需逐文件判定）。
- 其中 `AGENTS.md`/`ROADMAP.md` 的「13 键」为**有意保留的历史口径标注**（ADR-0006），不计入清理目标。

## 下一批
- 逐文件 allowlist 确认后替换；每次 commit 正文报残留数下降；
- 与 `crates/foundation/core/src/.r125-12-PHL-07-SPEC.md`（历史 spec）等一并归入"历史保留"清单，不追溯改写。
