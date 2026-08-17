# 13 分层验证网 L1-L5

> **R119-3a-2 Mavis 重建 (2026-08-10)**: 从 GLOSSARY.md §"分层验证网 L1-L5" 拆出。

```
[Document-Meta]
Document: docs/glossary/13-l1-l5-verification.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-2
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

## 定义

5 层验证清单(每一层明确验证对象 + 通过标准):

| 层级 | 名称 | 验证对象 |
|---|---|---|
| **L1** | 编译时 (compile-time) | `cargo check` / `clippy` / `fmt` / `cargo-deny` |
| **L2** | 运行时 (runtime) | 单元测试 / 集成测试 / 属性测试 / 模糊测试 / `cargo miri` / `loom` |
| **L3** | CI (continuous integration) | GitHub Actions / 契约测试 / 端到端 |
| **L4** | 集成 (integration) | 跨 crate 集成测试 |
| **L5** | 反思期 (reflection) | Cognitive-Dream 24h 周期自动验证 |

## 出处

阶段 1 §18.9 + §20.4。

## 6 哲学锚穿透

- **S-1** 北极星: 5 层验证保证 ASI 完整性
- **S-2** 实事求是: 每层明确对象 + 通过标准
- **O-5** 不假装: 编译期 hardcode + 运行时测试 + 反思期审计

## 不漂移

- 🔒 5 层验证网严守 (L1-L5)
- 0 改 workspace.version
- 0 改 R11 baseline 3 值
