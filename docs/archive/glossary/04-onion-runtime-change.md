# 04 门上内容动态变化

> **R119-3a-2 Mavis 重建 (2026-08-10)**: 从 GLOSSARY.md §"门上内容动态变化" 拆出。

```
[Document-Meta]
Document: docs/glossary/04-onion-runtime-change.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-2
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

## 定义

洋葱**结构**编译进核心(hardcode),**门上的具体内容**(12 键的判定逻辑、阈值、风险分级规则、决策策略、外部知识等)支持运行时动态变化(OTA / hot-reload / 反思期 / 演化)。

## 出处

主人 2026-07-31 最新指示。

## 例外

最核心层级(L0 HA 真实人类批准)**不可动态变**——这是 ASI 候选主体的最后护栏(阶段 1 §18.6 双根)。

## 比喻

🍖 **肉**——可生长。

## 6 哲学锚穿透

- **S-1** 北极星: 动态变化服务 ASI 演化
- **S-2** 实事求是: 运行时可调, 不是编译期
- **O-5** 不假装: L0 HA 例外永远不可变

## 不漂移

- 🔒 L0 HA 永远不可变 (例外)
- 🟢 其他门内容运行时可调
- 0 改 workspace.version
- 0 改 R11 baseline 3 值
