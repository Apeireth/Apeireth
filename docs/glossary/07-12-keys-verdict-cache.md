# 07 12 键 verdict cache

> **R119-3a-2 Mavis 重建 (2026-08-10)**: 从 GLOSSARY.md §"12 键 verdict cache" 拆出。

```
[Document-Meta]
Document: docs/glossary/07-12-keys-verdict-cache.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-2
Last-Modified: 2026-08-10
Status: 🔒 LOCKED (12 键编译期 hardcode)
```

## 定义

12 键运行时判定结果缓存(O(1) 查询)。

## 12 键组成

### V3 9 键 (PHL-01..09, R11 LOCKED)

1. **NotClone** — 不假装克隆
2. **NotPerfect** — 不假装完美
3. **NotUuid** — 不假装唯一
4. **NotUndo** — 不假装可撤销
5. **NotProof** — 不假装可证明
6. **NotSafe** — 不假装绝对安全
7. **SpecIsNotProof** — 规格不是证明
8. **CounterexampleIsNotBug** — 反例不是 bug
9. **ProverIsNotTruth** — 证明者不是真理

### v4.1 新增 3 键 (PHL-04..06)

10. **PHL-04 NotUnobservable** — 不假装不可观测
11. **PHL-05 NotUnscientific** — 不假装不科学
12. **PHL-06 NotSelfRelationless** — 不假装不与自身关系

## 出处

阶段 1 §10 + 阶段 2 §12 + v4.1 §15 + leader 自创"verdict cache"(运行时 O(1) 查询缓存术语)。

## 6 哲学锚穿透

- **S-1** 北极星: 12 键编译期保证 ASI 完整性
- **S-2** 实事求是: 运行时 O(1) 查询
- **O-5** 不假装: 12 键编译时 hardcode

## 不漂移

- 🔒 12 键编译期 hardcode 严守
- 🔒 9 键 V3 R11 LOCKED
- 0 改 workspace.version
- 0 改 R11 baseline 3 值
