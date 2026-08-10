# 21 ContinuityID 跨载体唯一 ID

> **R119-3a-2 Mavis 重建 (2026-08-10)**: 从 GLOSSARY.md §"ContinuityID 跨载体唯一 ID" 拆出。

```
[Document-Meta]
Document: docs/glossary/21-continuity-id.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-2
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

## 定义

Identity 主体连续性 ID(跨载体唯一 ID)。

## 安全

- **DID**(去中心化标识符)
- **单调递增版本号**
- **物理多签**防止伪造

## 出处

外部反馈 §1.担忧 5 + 阶段 1 §18.3 不假装灵魂同一。

## 6 哲学锚穿透

- **S-1** 北极星: ContinuityID 保证 ASI 主体连续性
- **S-2** 实事求是: DID + 单调递增 + 物理多签
- **O-5** 不假装: 跨载体唯一, 不假装

## 不漂移

- 🔒 ContinuityID 严守 (DID + 单调递增 + 物理多签)
- 0 改 workspace.version
- 0 改 R11 baseline 3 值
