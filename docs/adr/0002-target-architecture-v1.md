# 0002. 目标架构 v1 —— 七项重构方向收敛（Proposed）

- **Status**: Proposed（待 owner 裁决；裁决前**架构改动保持冻结**，本 ADR 只写目标、不动代码）
- **Date**: 2026-10-20
- **Owners**: 架构线（主笔）→ owner（裁决）
- **Supersedes**: —
- **契约影响**: 未来逐项把 `contract.toml` 的 `TODO-OWNER` target 条款转 accepted；**本 ADR 不改任何 accepted 条款**
- **例外关联**: —
- **预算净额**: 0（纯文档；代码改动按后续单项 ADR 计）

## 1. 背景（全部为 2026-10-20 实测）

架构体检发现 9 类结构性问题（详见 `docs/04-internal/architecture-review` 会话记录与探针数据）：118 条生产依赖边 vs 文档声明 32 条；两套扩展 ABI + 三份注册表；三个装配根；154 个 env 开关散在 148 文件；同名概念 `ContextAssembler`×2 双活；14 条生物机制跨 4 crate/6+ 接线点、仅 1 条有观测 probe；`core::Event` 死代码、`EventSpine` 0 命中；图像家族 126k LOC 占全仓 22.3%；SDK 孤岛 23.6k LOC 契约断头；全量门禁 >18 分钟。

规则体系（ADR-0001）已落地。本 ADR 给这片现状一个**目标形态**，让后续每个改动都能回答"离目标更近还是更远"。

## 2. 目标架构总则（五条，不改）

1. **对外伙伴、对内 OS**：产品面继续"数字伙伴"；工程按 OS 级不变量建设（隔离/可取消/可恢复/可审计/可回滚/可升级）。
2. **一个内核**：`Runtime::execute` 是唯一用户主循环；微内核不变量（零模块可启动、唯一会话变更点、治理路径唯一）不变。
3. **一个扩展模型**：所有可插拔物（认知模块/器官/工具/生物机制/MCP）走**同一 Extension ABI + 同一注册表 + 同一命名**。
4. **一条神经**：快事件走统一事件链；慢状态（调质/节律/免疫/痛觉/修复）走**唯一神经系统层**（信号总线 + 状态所有者 + 反射注册 + 节律时钟），一个接线根、一个观测口。
5. **一个真源**：结构规则 = `contract.toml`；能力状态 = `claims.yaml`；测试数字 = `test-baseline.md`；接线账 = 生成物。禁止双重手写。

## 3. 七项收敛（R0-R8，含依赖与归属）

| # | 收敛项 | 内容 | 依赖 | 主笔线 | 成本 |
|---|---|---|---|---|---|
| **R0** | 架构真源修正 | `ARCHITECTURE.md` 改生成物；14 键口径正式化（现 14 变体 vs 文档"13 键"，需 owner 授权）；crate 口径统一（含图像家族标注） | ADR-0001（已完成） | 架构线 | S |
| **R1a** | 神经系统·只读聚合 | 把 14 条 bio 机制的现状（开关/接线点/状态/观测）收进一个只读状态快照，`self_status` 单节输出；**不改行为、不改默认值** | R0 | 生物线（架构线出 spec） | M |
| **R1b** | 神经系统·订阅迁移 | 机制改订阅统一信号总线；开关收敛到单入口；补缺失信号源 | R1a + R2 + R3 | 生物线 | M-L |
| **R2** | 扩展 ABI 统一 | 一套 `Extension`（manifest + hooks + tools + signals）、一套注册表、一套命名；删 `LlmFactory` mirror；`Plugin`/`BehaviorModule`/`AgentModule` 收敛 | R0 | 架构线设计 + 生物线/工具线实施 | L |
| **R3** | 配置与接线架构 | typed config schema；env 登记表；**wiring ledger**（IMPLEMENTED→PRODUCTION WIRED→DEFAULT ENABLED→HARDWARE VALIDATED 四列 + 源码/测试引用）；装配根收敛到 runtime-assembly，适配器变薄 | R0 | 效率线（schema 由架构线定） | M |
| **R4** | crate 拓扑手术 | 图像家族迁独立 workspace（回主仓 ~20 members 口径）；SDK 孤岛与 perception 整包二选一（接线/归档，owner 裁决） | R0 | 架构线主导裁决 + 各线实施 | S-M |
| **R5** | 运行时健壮性 | 可取消 worker + 有界取消 + 上下文页表/检查点（**单进程内先做**）；多进程/总线/OTA 是否恢复由产品形态另裁 | R2/R3 | 专项线（生物线可起步） | L |
| **R6** | 验证架构 | 三层门禁（fast=受影响 crate / arch=依赖+接线+口径 / full=夜间收口）；形式化面如实（真证明或显式降级）；阶段 6 最小验证闭环（清单当关卡+人工签核+反思归档） | R0 | 效率线（架构线定契约） | M |
| **R7** | 上下文/提示/会话单一化 | `ContextAssembler`/`ContextBlock`/`assemble_tiered` 各收敛为一份；`SessionStore` 3 份收敛；调度器 4 份→1-2 条活路径；`PromptCacheStabilizer` 要么接线要么删 | R2 | 架构线设计 + 各线实施 | M |
| **R8** | 治理执行面统一 | 单一 Governance Pipeline（显式 stage 列表 + 统一 verdict + 审计落点）；真实人类批准做成可验证 trait；物理多签明确 v2.1 挂账；MEWG/席位触发矩阵接线或正式演替 | R3 | 安全线 | M |

## 4. 目标记分牌（方向值，非承诺；只许单向改善）

| 指标 | 现状 | 目标 | 方向 |
|---|---|---|---|
| 生产依赖边 | 118 | ≤ 90（层规则内收敛） | 只减 |
| 裸 env 名 | 154（src 口径） | ≤ 60（迁入 config schema） | 只减 |
| 同名概念 | ContextAssembler 2 / CapabilityRegistry 2 | 0 | 只减 |
| 生物机制观测 | 14 条 1 probe | 14/14 | 只增 |
| claims 覆盖 | 16 条 | 覆盖全部 PRODUCTION WIRED 能力 | 只增 |
| 最大 src 文件 | ~6,150 行 | ≤ 3,000 行 | 只减 |
| 全量门禁 | >18 分钟 | ≤ 12 分钟（分层后） | 只减 |
| 未接线字样 | 253 处 | 逐波下降 | 只减 |

## 5. 波次路线（裁决后启动；每波以架构复盘+基线缩表收口）

| 波 | 内容 | 出口判据 |
|---|---|---|
| **Wave 2**（2-3 周） | R0 剩余 + R1a 神经只读聚合 + R3a config schema/env 登记 + R4a 图像家族迁出 | `arch_guard --strict` 0 新增；图像家族迁出后全量门禁可分层；R1a 观测 14 条全有读数（只读） |
| **Wave 3**（3-4 周） | R2 扩展 ABI + R7 上下文单一化 + R3b wiring ledger + R4b SDK/perception 裁决执行 | 扩展点收敛为 1 套；重复实现清零；wiring ledger 覆盖 PRODUCTION WIRED |
| **Wave 4**（3-4 周） | R1b 订阅迁移 + R8 治理管线 + R6 验证架构 + 阶段 6 最小闭环 | 生物信号上总线；治理单一管线；四层验收有最小可跑闭环 |
| **Wave 5**（4-6 周） | R5 可取消 worker + 上下文页/检查点 + 发版冻结准备 | 插话能停/中断能续；冻结期例外清零 |

## 6. O-6 三阶审查

- **总体最优**：先把"神经/扩展/配置"三个横切面收敛，再做运行时健壮性——因为 R1a/R3/R7 直接降低后续每一项重构的返工率。拒的替代：①先做 R5（可取消 worker）——它依赖扩展/信号边界，边界未定会二次返工；②先重写 crate 拓扑——按现状 30 crate 的职责划分已够用，先动拓扑收益低于横切面。
- **系统最优**：R2/R7 动 foundation+runtime，R1 动 runtime-assembly+organ，R3 动 cli+runtime-assembly，R4 只动构建面——四个方向互不重叠，可按波并行；R5 留到最后，避免在移动的边界上做并发模型。拒的替代：①一次性全并行——同工作树共享 index/构建锁，冲突成本高于收益；②串行一条线做——周期过长，生物线的机制实施会停摆。
- **架构最优**：目标形态不引入新 crate、不新增外部依赖、不改 LOCKED 5 项；全部收敛在现有 31 members 内完成（R4 反而减少 members）。拒的替代：①新建"架构层 crate"承载神经系统——为架构引入编译依赖，违反最小内核；②推倒重写——8889+ 测试与治理资产是护城河。

## 7. 风险与明确不做

**风险**：R2 触及 runtime/plugin 公共契约，误伤面大 → 分两批（先新增 ABI 并存，再迁 impl，最后删旧）；R4 图像家族出主仓影响 `tools→image-core` 边 → 用 feature 包 + 注册缝过渡；R5 与产品形态未定 → 只做单进程内取消/检查点，不承诺多进程。

**明确不做**：不追阶段 2 的多进程 supervisor 树/5 层总线/6 DB（已被 founding-design-v2 合理演替，非缺口）；不为"更像 OS"加概念；不碰 LOCKED 5 项；不引入计费/账号/商店。

## 8. 验收与裁决位

- [ ] owner 裁决本 ADR（Accept / 改 / 退）
- [ ] 裁决后：`contract.toml` target 条款逐项转 accepted（每项配单项 ADR）
- [ ] Wave 2 启动前：R1a spec、R3 config schema、R4 迁出方案各出单项 ADR（Proposed）
- [ ] 每波收口：`arch_guard --strict` 0 新增 + 基线缩表 + 记分牌无指标变差

## 9. 性质与回滚

纯文档决策；回滚 = `git revert`，0 代码/0 LOCKED/0 外部依赖。本 ADR 被否决时，架构改动继续冻结，规则体系（ADR-0001）不受影响继续生效。
