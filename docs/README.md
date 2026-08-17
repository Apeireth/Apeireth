# Apeireth Documentation

> 文档体系 1.0（2026-08-18 规范重构）：结构分层，历史归档，与实际代码对齐。

## Structure

```
docs/
├── 01-architecture/     # 架构（愿景/哲学/架构总览/安全模型）
├── 02-guides/           # 使用指南（快速开始/用户手册/部署）
├── 03-reference/        # 参考（crates 索引/术语/API）
├── 04-internal/         # 内部工作文档（台账/设计意图/团队）
└── archive/             # 历史归档（stage*/r*/adr/conventions... 保留不展示）
```

## Index

| 文档 | 说明 |
|---|---|
| [01-architecture/vision.md](01-architecture/vision.md) | 愿景：五原型 + 产品北极星 |
| [01-architecture/philosophy.md](01-architecture/philosophy.md) | 哲学：6 锚 / 双洋葱 / 0 装 PASS |
| [01-architecture/architecture.md](01-architecture/architecture.md) | 架构总览（对齐 85 crates）|
| [01-architecture/security.md](01-architecture/security.md) | 安全模型（对齐实际机制）|
| [02-guides/quick-start.md](02-guides/quick-start.md) | 快速开始（真实命令）|
| [02-guides/user-manual.md](02-guides/user-manual.md) | 用户手册 |
| [03-reference/crates.md](03-reference/crates.md) | 85 crates 索引（从代码生成）|
| [04-internal/design-intent.md](04-internal/design-intent.md) | 设计意图与主人拍板历史 |
| [04-internal/backlog.md](04-internal/backlog.md) | 唯一权威台账 |
| [04-internal/release-plan.md](04-internal/release-plan.md) | 发布计划 |

## Archive

历史设计/轮次/决策文档在 [`archive/`](archive/)（stage1-6、r149-r270、adr、conventions、glossary 等）——保留完整 git 历史，不再作为活跃文档索引。
