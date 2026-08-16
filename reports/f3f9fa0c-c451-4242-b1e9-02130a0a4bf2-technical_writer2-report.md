# 自检-TW2: CHANGELOG 与版本一致性报告

**结论: ⚠️（存在出入，非阻断；发版前需修正）**
**范围**: CHANGELOG.md 顶部 / RELEASE_NOTES.md / 根 Cargo.toml workspace 版本（只读核对）

## 事实基线

| 来源 | 版本标识 | 位置 |
|---|---|---|
| 根 `Cargo.toml` `[workspace.package]` | **1.2.0** | Cargo.toml:224 |
| `CHANGELOG.md` 最新 semver 条目 | **[1.2.0] — R125-R127** | CHANGELOG.md:148 |
| `CHANGELOG.md` 顶部首条 | 日期条目 `[2026-08-16]`（无 semver）+ `[Unreleased] — R128` | CHANGELOG.md:3 / :45 |
| `RELEASE_NOTES.md` 标题 | **Apeireth v1.0.0** (Tag v1.0.0, planned) | RELEASE_NOTES.md:1-3 |

## 出入清单

1. ⚠️ **RELEASE_NOTES.md (v1.0.0) ≠ workspace.version (1.2.0)**：release notes 标题为 v1.0.0，但正文表格自认 workspace.version=1.2.0，标题与 Cargo.toml 不一致。若按 v1.0.0 打 tag，将与实际 1.2.0 冲突；若发 1.2.0，标题/Tag 行需改写。
2. ⚠️ **CHANGELOG.md 顶部无 semver 条目**：首条为日期型 `[2026-08-16]` banner 归位记录，且 `[Unreleased] R128` 之后已有 R131-R178 大量实际变更未归入任何版本条目（Keep-a-Changelog 规范下应挂 [Unreleased] 或新版本）。
3. ⚠️ **RELEASE_NOTES.md 引用行号漂移**：正文称 workspace.version 位于 `Cargo.toml:246`，实际在 **:224**。
4. ⚠️ **11 个活动 crate 硬编码版本，不继承 workspace**（0.1.0 ×7：i18n、lark、livekit、pipeline-g5、state、voice、memory/extensions；1.0.0 ×4：blueprint-impl、integration-e2e、rate-limiter、team-lead）。与根 Cargo.toml 注释"27 硬编码待 1.0 后清"一致，属已知 TODO，但会污染发布版本号。

## 一致项

- ✅ CHANGELOG 最新 semver 条目 [1.2.0] 与 workspace.version 1.2.0 一致。
- ✅ 绝大多数活动 crate 使用 `version.workspace = true` 继承（抽查如 apeireth-cli）。

## 建议（发版前）

- 拍板 v1.0.0 vs 1.2.0 单一口径，改 RELEASE_NOTES.md 标题/Tag 行或回退 workspace.version（涉及决策，需 Leader 定）。
- 将 CHANGELOG 顶部日期条目内容并入 [Unreleased] 或新建版本条目。
- 修正 RELEASE_NOTES.md 中 Cargo.toml 行号引用；清理 11 个硬编码 crate 改继承。

*核对人: 技术文档2 · 2026-08-16 · 任务 f3f9fa0c-c451-4242-b1e9-02130a0a4bf2*
