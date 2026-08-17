# 06 基线快照 (snap-<hash>)

> **R119-3a-1 Mavis 重建 (2026-08-10)**: 从 APEIRETH-VERSIONING.md §6 拆出,核验后写。

```
[Document-Meta]
Document: docs/versioning/06-snapshot.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-1
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

## 格式

`snap-<commit-hash>`

## 基线快照清单 (核验后)

| 基线 | commit | 含义 |
|---|---|---|
| `snap-9c80c9165625` | R11 末真态 (历史) | R11 baseline |
| `snap-29d499bb` | R14 末 (历史) | R14 Rust 重写 baseline |
| `snap-a64fe197` | R38 1.1 RC master HEAD | 1.1 release baseline |
| `snap-1f23b28f` | R38 B1 telemetry 1.1 真合并 | B1 增量 baseline |
| `snap-eafb42c7` | R46-R53 1.1.1 follow-up 末 (R54 前) | 1.1.1 baseline |
| `snap-7f9928b3` | R63-R68 1.2 candidate 末 | 1.2 candidate baseline |

## 当前 (R119 核验)

最新基线 = R114-R118 commit `5c546a84` (codex) — **未 snap 命名** (R119 形式撤销后, 留作历史)

## R119 严守

- 🔒 snap-a64fe197 / snap-1f23b28f / snap-eafb42c7 / snap-7f9928b3 / snap-9c80c9165625 / snap-29d499bb **不动** (历史记录)
- 🟢 R114-R118 (5c546a84) 形式可调 (per 主人 8/10 01:14 拍板"形式可调, 数据严守")

## 不漂移

- 0 触碰任何 LOCKED 文档
- 0 改 workspace.version
- 0 改 R11 baseline 3 值
