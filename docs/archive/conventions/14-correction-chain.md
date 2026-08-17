# 14 修正链 v3-v17 (思想历史保留)

> **R119-3a-1 Mavis 重建 (2026-08-10)**: 主人 8/10 01:14 拍板"思想历史 + 最新技术文档",修正链 v3-v17 是规范层思想沉淀,**保留**。

```
[Document-Meta]
Document: docs/conventions/14-correction-chain.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-1
Last-Modified: 2026-08-10
Status: 🟢 活跃 (思想历史保留)
```

## 修正链核验 (Fix-3..Fix-17, 15 条)

| 修正链 | 主题 | 出处 |
|---|---|---|
| v3 → Fix-3 | 12 键 + 5 重守门位置 | R14 阶段 4 |
| v4 → Fix-4 | 5 重守门融入每层 | R14 阶段 4 |
| v5 → Fix-5 | 4 重守门嵌套 + 权限发放 | R14 阶段 4 v5 修正 |
| v6 → Fix-6 | 5 重治理 + 4 重守门 + 权限发放 + E 层修改路径 | R14 阶段 4 |
| v7 → Fix-7 | HA 部署模式自适应 | R14 阶段 4 |
| v8 → Fix-8 | 偏差修正 (4 关系 + 三域分离 + 主体连续性 + SGI) | R14 阶段 4 |
| v9 → Fix-9 | 漂移检查 | R14 阶段 4 |
| v10 → Fix-10 | 版本号系统 | R14 阶段 4 |
| v11 → Fix-11 | R35 facade 真合并 (apeireth-telemetry umbrella) | R35 |
| v12 → Fix-12 | R38 1.1 真合并 (4 老 facade + 5 老 provider 真删, 6 哲学锚穿透) | R38 |
| v13 → Fix-13-R54 | R54 B8 续: backend wire-up + cognition_graph 真接 TUI memory | R54 |
| v14 → Fix-14-R72 | R70-R72 1.2 patch LIVE 主题 | R70-R72 |
| v15 → Fix-15-R78-113 | R78-R113 1.2 patch LIVE 续 (skills semver + mcp bridge + cognition bus) | R78-R113 |
| v16 → Fix-16-R114 | R114 MCP 真接 (initialize + prompts + subscriptions + tool_subscriptions) | R114 |
| v17 → Fix-17-R114-118 | R114-R118 动态运营层 (Eval/Council MCP + CLI + TUI cognition live + Protocol bridges) | R114-R118 |

## 核验: 修正链来源文档

- `docs/stage4/stage4-correction-v3-onion-embedded-keys-gates.md`
- `docs/stage4/stage4-correction-v4-onion-dedupe.md`
- `docs/stage4/stage4-correction-v5-gates-refined.md`
- `docs/stage4/stage4-correction-v6-consolidated-and-e-layer-mutation.md`
- `docs/stage4/stage4-correction-v7-deployment-mode-adaptive.md`
- `docs/stage4/stage4-correction-v8-deviation-check.md`
- `docs/stage4/stage4-correction-v9-drift-check.md`
- `docs/stage4/stage4-correction-v10-versioning-system.md`
- `docs/stage4/stage4-correction-v11-conventions.md`
- `docs/stage4/stage4-correction-v12-final-check.md`
- `docs/stage4/stage4-correction-v13-placeholder-dirs.md`
- `docs/stage4/stage4-correction-v14-final-cleanup.md`
- `docs/stage4/stage4-correction-v15-four-gates-permission-grants.md`

(13 修正链文件, Fix-3..Fix-15 在 stage4 LOCKED 范围)

## 思想 vs 技术区分

主人 8/10 01:14 拍板: **"我们就要思想历史 + 最新技术文档"**

- ✅ 思想历史保留 (本文件 Fix-3..Fix-17)
- ✅ 最新技术文档保留 (R114-R118 动态运营层)
- ❌ 技术发展史报告链不要 (R-Round 报告按"思想历史"原则筛选, 但 Fix 链保留因为是规范层沉淀)

## 不漂移

- 0 触碰任何 LOCKED 文档 (Fix-3..Fix-15 在 stage4 LOCKED)
- 0 改 workspace.version
- 0 改 R11 baseline 3 值
