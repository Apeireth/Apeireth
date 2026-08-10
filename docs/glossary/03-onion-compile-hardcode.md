# 03 洋葱结构编译时 hardcode

> **R119-3a-2 Mavis 重建 (2026-08-10)**: 从 GLOSSARY.md §"洋葱结构编译时 hardcode" 拆出。

```
[Document-Meta]
Document: docs/glossary/03-onion-compile-hardcode.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-2
Last-Modified: 2026-08-10
Status: 🔒 LOCKED (编译时 hardcode, 不变)
```

## 定义

双洋葱统一体的**结构**("有原则洋葱 + 权限洋葱 + 双洋葱嵌套 + L0 HA 核心 + N 层洋葱")由 Rust 类型系统编译时 hardcode。**这是骨架**,确定"有哪些层 + 层与层关系"。

## 出处

主人 2026-07-31 最新指示(修正之前的"12 键编译时 hardcode"理解)。

## 比喻

🦴 **骨骼**——结构确定。

## 6 哲学锚穿透

- **S-1** 北极星: 编译时保证 ASI 完整性
- **S-2** 实事求是: 编译期 hardcode, 不是运行时
- **O-5** 不假装: 12 键 / 5 守门编译期拒绝 (per GLOSSARY §7)

## 不漂移

- 🔒 编译期 hardcode 严守 (Rust 6 大编译时约束: 所有权/借用/生命周期/Trait/无反射/零成本)
- 🔒 双洋葱结构 (5 原则 + 6 权限 = 11 层) 严守
- 0 改 workspace.version
- 0 改 R11 baseline 3 值
