# ADR — 架构决策记录（Architecture Decision Records）

> `Status: 📝 v0.1 草案（2026-10-20）`
> 依据：架构契约体系（[docs/architecture/README.md](../architecture/README.md)）。
> 本目录**只增不改**：新决策取代旧决策时，新 ADR 的 `Supersedes` 指旧 ADR，旧 ADR 保留原样。

## 什么时候必须写 ADR

| 触发 | 必须写 |
|---|---|
| 新增/删除 crate、层、trait、注册表、总线、事件类型族 | ✅ |
| 新增跨层依赖边（即使层规则允许） | ✅ |
| 改变扩展模型、配置模型、接线模型（如 R1 神经系统、R2 扩展 ABI、R3 config schema） | ✅ |
| 修改契约 `contract.toml` 的 accepted 条款或预算 | ✅ |
| 能力从 IMPLEMENTED → PRODUCTION WIRED 的**结构性**接线（非单点接线） | ✅ |
| 普通功能（走既有扩展点 + 准入清单） | ❌（commit 正文带准入八问即可） |
| LOCKED 5 项相关 | ❌（不是 ADR 范围；走 LOCKED 真例外流程 + owner 拍板） |

## 规则

1. **先 ADR 后代码**：ADR 状态 `Proposed` → owner 裁决 → `Accepted` 后才允许进入实现排期；
   紧急修复类可以 `Accepted` 与实现同批，但正文必须写清"为什么不能先 ADR"。
2. **O-6 三阶审查必带**：总体最优 / 系统最优 / 架构最优，各写"拒的 alternatives + 拒理由"。
3. **契约影响必写**：改了哪条 contract 条款/预算/允许边；基线净额（增/删各几）。
4. **迁移与回滚必写**：分几步、每步可独立验证、回滚命令。
5. **例外关联**：如果实现期需要违反契约（带到期日），必须引用 `docs/architecture/exceptions.toml` 的 `EXC-*`。
6. **状态词**：`Proposed` / `Accepted` / `Superseded` / `Rejected` / `Withdrawn`——只有 `Accepted` 的条款进入契约。

## 索引

| ADR | 标题 | 状态 | 关联 |
|---|---|---|---|
| [0001](0001-adopt-architecture-contract.md) | 采纳架构契约体系与例外预算 | Accepted | contract.toml / arch_guard.py / 例外预算旋钮 |
| [0002](0002-target-architecture-v1.md) | 目标架构 v1——七项重构方向收敛 | Accepted | R0-R8 / 波次路线 / 记分牌 |
| [0003](0003-r1a-nervous-readonly-snapshot.md) | R1a 神经系统·只读聚合（spec） | Accepted（实施归生物线） | Wave 2 / self_status 22 节 |
| [0004](0004-r3-config-and-wiring-ledger.md) | R3 配置 schema + env 登记 + wiring ledger | Accepted（实施归效率线） | Wave 2-4 / env-registry / 接线账 |
| [0005](0005-r4-crate-topology-surgery.md) | R4 crate 拓扑手术——图像家族门禁隔离 + SDK/perception 归档裁决 | Accepted（R4a/R4b；执行派实施线） | Wave 2 / 门禁时间 / 归档清单 |
| [0006](0006-formalize-14-keys.md) | 14 键正式化——LOCKED 键集口径修正 | Accepted（owner 2026-10-20） | AGENTS/ROADMAP/契约口径 |
| [0007](0007-r7-context-session-unification.md) | R7 上下文/提示/会话单一化 | **Proposed**（依赖 R2；待 owner 裁决） | Wave 3 / ContextBlock·SessionStore·scheduler 收敛 |
| [0008](0008-r2-extension-abi.md) | R2 扩展 ABI 统一（单 ABI 家族 / 单注册表 / 单命名） | **Proposed**（待 owner 裁决；R1b/R7 前置） | Wave 3 / extension_debt·AgentModule 命名漂移 |

## 命名

`NNNN-kebab-case-title.md`，四位递增编号（`0001-...`）。模板：[_template.md](_template.md)。
