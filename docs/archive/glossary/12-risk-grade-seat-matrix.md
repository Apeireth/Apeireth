# 12 风险分级 → 席位触发矩阵

> **R119-3a-2 Mavis 重建 (2026-08-10)**: 从 GLOSSARY.md §"风险分级 → 席位触发矩阵" 拆出。

```
[Document-Meta]
Document: docs/glossary/12-risk-grade-seat-matrix.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-2
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

## 定义

7 席审议庭按风险差异化触发,不是对所有决策都强制 7 席全量。

| risk 等级 | 触发席位 | 通过门槛 |
|---|---|---|
| **critical** | 全量 7 席 | ≥5 席同意 |
| **high** | 5 席 | ≥4 席同意 |
| **medium** | 3 席 | ≥2 席同意 |
| **low** | 1 席 | 该席同意 |
| **info** | 0 席 | 仅 record |

## 出处

阶段 1 §20.3 + D2 §12。

## 6 哲学锚穿透

- **S-1** 北极星: 风险差异化, 服务 ASI 完整性
- **S-2** 实事求是: 5 等级差异化, 不是全量
- **O-5** 不假装: info 仅 record, 不通过

## 不漂移

- 🔒 7 席审议庭 + 5 等级差异化严守
- 0 改 workspace.version
- 0 改 R11 baseline 3 值
