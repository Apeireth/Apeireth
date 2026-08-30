# Apeireth Documentation

> 文档体系 1.1（2026-08-26 封板前整理）：结构分层，历史归档，与当前根工作区对齐。带日期的集成/审计记录属于历史证据，不能替代当前 crate 清单或入口说明。

## Structure

```
docs/
├── 01-architecture/     # 架构（品牌/愿景/哲学/架构/安全/工程报告）
├── 02-guides/           # 使用（快速开始/用户手册/部署/开发）
├── 03-reference/        # 参考（crates/API/术语）
├── 04-internal/         # 内部工作文档（台账/设计意图/团队）
├── design/              # 前端设计（设计系统/交接/开场动画封存档案）
├── development/         # 开发（目录布局/新增模块/移植清单）
└── archive/             # 历史归档（stage*/r*/adr/conventions... 保留不展示）
```

## Index

| 文档 | 说明 |
|---|---|
| [01-architecture/brand.md](01-architecture/brand.md) | 品牌：命名（Apeiron）+ 宣言 + Logo Design Brief |
| [01-architecture/vision.md](01-architecture/vision.md) | 愿景：五原型 + 产品北极星 + 三远合一 |
| [01-architecture/philosophy.md](01-architecture/philosophy.md) | 哲学：8 锚 / 三洋葱 / 0 装 PASS |
| [01-architecture/architecture.md](01-architecture/architecture.md) | 架构总览（当前根工作区与独立桌面工作区）|
| [01-architecture/security.md](01-architecture/security.md) | 安全模型（对齐实际机制）|
| [01-architecture/engineering-report.md](01-architecture/engineering-report.md) | 工程报告（1.0 实测数据/里程碑/纪律）|
| [01-architecture/system-capabilities.md](01-architecture/system-capabilities.md) | 系统能力规范手册（全域能力体系与安全治理契约）|
| [01-architecture/v2-master-lineage-and-upgrade-blueprint.md](01-architecture/v2-master-lineage-and-upgrade-blueprint.md) | 2.0 终极升级蓝图白皮书（1.0 行级与 170+ 标杆解构）|
| [01-architecture/vcp-vs-apeireth-deep-comparison.md](01-architecture/vcp-vs-apeireth-deep-comparison.md) | Apeireth 2.0 vs VCPToolBox 深度架构对比与优劣势洞察报告 |
| [02-guides/quick-start.md](02-guides/quick-start.md) | 快速开始（真实命令）|
| [02-guides/user-manual.md](02-guides/user-manual.md) | 用户手册（功能详解/FAQ）|
| [02-guides/deployment.md](02-guides/deployment.md) | 部署（环境变量/持久化/前端接入/故障排查）|
| [02-guides/development.md](02-guides/development.md) | 开发指南（代码地图/模式/陷阱/提交规范）|
| [03-reference/crates.md](03-reference/crates.md) | 当前根工作区 crate 索引 |
| [03-reference/capabilities-matrix.md](03-reference/capabilities-matrix.md) | 工业级能力矩阵与对外契约参考 |
| [03-reference/team-handover-reference.md](03-reference/team-handover-reference.md) | **团队接手一站式参考全景手册（权威交接主入口）** |
| [03-reference/vcp-line-level-absorption-guide.md](03-reference/vcp-line-level-absorption-guide.md) | VCP 核心算法行级代码解构与 2.0 吸收升级指南 |
| [03-reference/api.md](03-reference/api.md) | API 参考（真实端点/工具协议/认证）|
| [03-reference/glossary.md](03-reference/glossary.md) | 术语表（品牌/架构/记忆/她本身/安全）|
| [04-internal/design-intent.md](04-internal/design-intent.md) | 设计意图与拍板历史 |
| [04-internal/backlog.md](04-internal/backlog.md) | 唯一权威台账 |
| [04-internal/release-plan.md](04-internal/release-plan.md) | 发布计划 |
| [design/01-DESIGN-SYSTEM.md](design/01-DESIGN-SYSTEM.md) | 前端设计系统（视觉令牌/层序规范）|
| [design/frontend-handoff.md](design/frontend-handoff.md) | 前端交接（现状/联调/坑与纪律/欠账，接手先读）|
| [design/intro-animation.md](design/intro-animation.md) | 开场动画「火之文明史」封存档案（2026-08-22 起默认关闭）|
| [development/repository-layout.md](development/repository-layout.md) | 当前目录、ownership 与依赖边界 |
| [../../ROADMAP.md](../../ROADMAP.md) | 顶层路线图（v2 下一步按优先级；v1 时代详单在 archive/roadmap） |
| [../../CHANGELOG.md](../../CHANGELOG.md) | 变更流水（v2.0.0-alpha.1 段为重构版记录，其余为 v1 历史） |

## 当前基线（2026-08-27）

默认分支 `main` @ `d6910cf7`，tag `v2.0.0-alpha.1`；13-crate 工作区（foundation 5 / engine 4 / capabilities 1 / adapters 3）+ 独立前端 workspace；旧 86-crate 代码在 `legacy/`（排除构建）。带"现状 (2026-08-27)"指引的历史文档正文不改，只标历史属性。

## Archive

历史设计/轮次/决策文档在 [`archive/`](archive/)（stage1-6、r149-r270、adr、conventions、glossary 等）——保留完整 git 历史，不再作为活跃文档索引。
