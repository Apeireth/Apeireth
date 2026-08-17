# 07 Commit Hash 引用系统

> **R119-3a-1 Mavis 重建 (2026-08-10)**: 从 APEIRETH-CONVENTIONS.md §7 拆出,核验后写。

```
[Document-Meta]
Document: docs/conventions/07-hash.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-1
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

## 格式

`<short-hash>` 或 `snap-<short-hash>`

## 引用方式

| 引用 | 含义 | 例子 (核验) |
|---|---|---|
| `<short-hash>` | git commit 短 hash (7-8 字符) | `5c546a84` / `dd9b38f6` / `a64fe197` |
| `snap-<hash>` | R 周期 baseline 快照 | `snap-29d499bb` / `snap-a64fe197` / `snap-1f23b28f` / `snap-eafb42c7` / `snap-7f9928b3` / `snap-9c80c9165625` |

## 当前实际(核验)

- ✅ R38 1.1 RC master HEAD: `a64fe197` (snap-a64fe197)
- ✅ R38 B1 telemetry 1.1 真合并: `1f23b28f` (snap-1f23b28f)
- ✅ R46-R53 1.1.1 follow-up 末: `eafb42c7` (snap-eafb42c7)
- ✅ R63-R68 1.2 candidate 末: `7f9928b3` (snap-7f9928b3)
- ✅ R11 末真态 (历史): `9c80c9165625` (snap-9c80c9165625)
- ✅ R114-R118 (codex): `5c546a84`
- ✅ R78-R113 (R114-R118 之前): `dd9b38f6`
- ✅ R119-1 (Mavis hygiene): `79571cb4`
- ✅ R119-2 (Mavis top): `00df7125`
- ✅ R119-3a-1 (Mavis conventions): (本批)

## 不漂移

- 0 触碰任何 LOCKED 文档
- 0 改 workspace.version
- 0 改 R11 baseline 3 值
