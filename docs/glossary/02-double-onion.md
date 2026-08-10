# 02 双洋葱统一体 (Double Onion Unity)

> **R119-3a-2 Mavis 重建 (2026-08-10)**: 从 GLOSSARY.md §"双洋葱统一体" 拆出。

```
[Document-Meta]
Document: docs/glossary/02-double-onion.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-2
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

## 定义

原则洋葱 E/S/A/M/O **嵌入**权限洋葱 L0-L5,**不是**两个独立锁,是**一个统一体的两个切面**。

## 出处

阶段 1 §18.7 + v4 修正 #3+#4 + 阶段 3 §3.8 + R14-D7。

## 关键洞察

原则"长在"权限的每一层里,权限"承载"原则。

## 6 哲学锚穿透

- **S-1** 北极星: 双洋葱统一体是 ASI 完整性的工程化
- **S-2** 实事求是: 不是两个独立锁, 是一个统一体
- **O-2** 前人肩上: 借鉴 Unix 单一权限模型 + 哲学与实现统一
- **O-3** 干到底: 原则与权限嵌入, 不分立
- **O-4** 任何人都能接手: 双洋葱是 Apeireth 核心架构

## 不漂移

- 🔒 编译时 hardcode 严守 (per [03-onion-compile-hardcode](03-onion-compile-hardcode.md))
- 🔒 原则洋葱 E/S/A/M/O + 权限洋葱 L0-L5 结构严守
- 0 改 workspace.version
- 0 改 R11 baseline 3 值
