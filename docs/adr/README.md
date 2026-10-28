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
| [0007](0007-r7-context-session-unification.md) | R7 上下文/提示/会话单一化 | **Accepted + 已交付**（2026-10-20 收口） | Wave 3 / ContextBlock·SessionStore·scheduler 收敛 |
| [0008](0008-r2-extension-abi.md) | R2 扩展 ABI 统一（单 ABI 家族 / 单注册表 / 单命名） | **Accepted + 已交付**（2026-10-20 收口） | Wave 3 / extension_debt·AgentModule 命名漂移 |
| [0009](0009-r1b-nervous-bus.md) | R1b 神经系统——同步信号总线 / 开关单入口 / 台账可校验 | **Accepted**（2026-10-20，owner 授权 Wave 4） | Wave 4 / 总线·开关表·台账校验 |
| [0010](0010-r8-governance-pipeline.md) | R8 治理单一管线——六段编排 / 单一入口 / 审计可落地 | **Accepted**（2026-10-20，owner 授权 Wave 4） | Wave 4 / 治理编排·入口收敛·审计落地 |
| [0011](0011-r6-verification-architecture.md) | R6 验证架构——三层门禁语义 / 单一执行器 / 四层验收载体 | **Accepted**（2026-10-20，owner 授权 Wave 4） | Wave 4 / 门禁语义·数字真源·四层验收 |
| [0012](0012-central-ai-and-household-companion.md) | 中央 AI 与每户伙伴并存——两类主体、两套数据归属 | **Accepted**（2026-10-22，owner 口述拍板） | 多人阶段前置裁定：数据/成长归属、跨主体流动边界 |
| [0013](0013-open-source-personal-utility-assistant.md) | 定位重裁定：开源免费的个人工具型助手，动态红线取代完全本地 | **Accepted**（2026-10-23，owner 口述拍板）；**取代 0012** | 主战场/红线/开源免费定位；承重墙=出境脱敏闸接电 |
| [0014](0014-astra-derived-contract.md) | 外部顾问判据落地：四条共识改写、平台第四职责、最小诚实自述集 | **Accepted**（2026-10-24，owner 裁定）；内容已全部落地合入 main | 存在理由、义务分级、授权策略对象、问责五步、开源语境差异判据 |
| [0016](0016-personal-line-carries-all-care-mechanisms.md) | 个人线带全部 14 条关怀机制，默认关，用户自行开启 | **Accepted**（2026-10-24，owner 拍板） | 机制归属裁定；数据面=配置文件+CLI（前端待接，不涉及本 ADR） |
| [0017](0017-kernel-convergence-observation-port.md) | KC-1 内核收束：通用完成观察口与注册式内部控制命令 | **Accepted + 已交付**（2026-10-27，KC-1 SPEC） | 内核边界 / ToolCompletionObserver / ControlCommand / `[kernel]` 守卫 |
| [0018](0018-registry-kernel-tool-events.md) | Registry Kernel 渐进迁移：工具事件与闭合消息投影 | **Accepted（协议基础设施范围）**（2026-10-27，owner 重构任务授权） | ToolEventV1 / 协议成对性 / 自动 Hook 与实例生命周期后续验收 |
| [0019](0019-registry-runtime-dispatch.md) | Registry Runtime 接线：沿用插件生命周期与唯一治理入口 | **Accepted（Phase 2 纵向切片）**（2026-10-27，owner 授权） | PluginManager / 固定 V1 Hook / Dispatcher / Legacy Adapter |
| [0020](0020-capability-module-ownership.md) | Capability 模块归属与执行上下文收敛 | **Accepted（Phase 3 执行路径迁移）**（2026-10-28，owner 授权） | 唯一工具目录 / Host 选装 / 执行上下文 / 原生取消与排水 |

## 命名

`NNNN-kebab-case-title.md`，四位递增编号（`0001-...`）。模板：[_template.md](_template.md)。
