# 01 命名空间系统

> **R119-3a-1 Mavis 重建 (2026-08-10)**: 从 APEIRETH-CONVENTIONS.md §1 拆出,核验后写。

```
[Document-Meta]
Document: docs/conventions/01-namespace.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-1
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

## 格式

`<namespace>-<id>`

## 11 个命名空间

| 命名空间 | 用途 | 例子 (核验后) |
|---|---|---|
| `V<n>` | 指标 | V0.5 / V1136 / V3-9keys / V3-12keys / V0172 / V0164 / V0180 |
| `Design-X.Y` | 设计层 | Design-1.0 / Design-2.0 / Design-2.1 / Design-3.0 / Design-4.0 / Design-5.0 / Design-omnibus-1.0 |
| `Fix-N` | 修正链 | Fix-3 / Fix-4 / ... / Fix-17 (R114-R118 加 Fix-15/16/17) |
| `R-N` | R 周期 | R11 / R12 / R13 / R14 / R17 / R23 / R33-R37 / R38 / R46-R53 / R54 / R57-R62 / R63-R72 / R78-R113 / R114-R118 |
| `Manual-Rev-X` | 手册修订 | Manual-Rev-A..L (R114-R118 Manual-Rev-L) |
| `A<n>` | 成就编号 | A1 / A5 / A10 / A15 / A20 |
| `ADR-NNNN` | 架构决策记录 | ADR-0001 ~ ADR-0018 (12 重排 + 3 配套 + 1 Rust SDK) |
| `snap-<hash>` | 基线快照 | snap-9c80c9165625 / snap-29d499bb / snap-a64fe197 / snap-1f23b28f / snap-eafb42c7 / snap-7f9928b3 |
| `D<n>` | 阶段编号 | D1 / D2 / D3 / D4 / D5 / D6 / D7 |
| `P<n>` | 架构图编号 | P1-P5 (5 主图) |
| `round<N>-<NN>` | R 周期 commit 形态 (R15+ 实践) | round15-04 / round16-12 / round17-01 |

## 核验

- ✅ R 周期:实际 R11-R118 (per git log + reports/),18 周期
- ✅ 修正链:实际 Fix-3..Fix-17 (R114-R118 加 Fix-15/16/17,per APEIRETH-VERSIONING.md)
- ✅ 手册修订:实际 Manual-Rev-A..L (R114-R118 Rev-L)
- ✅ 指标:实际 V0.5 / V1136 / V3-9keys / V1331 / V0172 / V0164 / V0180
- ✅ ADR:实际 12 (R20 重排 0001-0012) + 3 (0013/0014/0015) + 1 (0018 Rust SDK) = 16 主 + 21 archive

## 不漂移

- 0 触碰 24 LOCKED crate
- 0 改 workspace.version
- 0 改 R11 baseline 3 值
