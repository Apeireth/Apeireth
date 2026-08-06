# R14-D8-Fix：按主人纠正"两把独立锁"修正 D8 错版

## 1. 任务元信息

| 字段 | 值 |
|---|---|
| 任务 ID | `dc52b9a0-7ca0-4bdd-9c7e-204b8715ecb3` |
| 角色 | 技术文档（technical_writer） |
| 触发 | 主人 2026-07-31 同日纠偏："我们实际上要的就是双洋葱正交，两把锁" |
| 修正方向 | 按阶段 2 §10 decision-system Phase 1 已落的"两把独立锁"设计：锁 A 原则洋葱 + 锁 B 权限洋葱 + AND 运算 |
| 修改性质 | 错版措辞保留为历史轨迹（不删除）+ D8-fix 纠错标注追加 + 新版结构化叙述追加 |
| 提交主题 | `R14-D8-Fix：按主人纠正"两把独立锁"修正 D8 错版` |

## 2. 修改摘要（5 文件 + 1 报告）

### 文件 1：`Apeireth-rust/docs/onion-wall-architecture-2026-07-31.md`（主体修改）

| # | 位置 | D8-fix 改动 |
|---|------|------|
| 1 | §1.1 主人原话引用 | 保留 R14-D8 旧措辞为历史轨迹；追加 D8-fix 主人纠偏原话 + D8-fix 纠错段 |
| 2 | §1.2 走法甲/乙表 | 走法乙状态改为 ~~采用~~ → "R14-D8-fix 改: 并入 `apeireth-core/src/onion/`" |
| 3 | §1.3 走法乙 3 细节 | 细节 1 "交叉咬合 / per-layer 双重过滤" 保留为历史轨迹 + D8-fix 纠错段 |
| 4 | §2 标题 | "内墙咬合形态" → 保留标题 + D8-fix 标注"应改为'双锁独立形态'" |
| 5 | §2.2 内容 | 旧措辞"per-layer 双重过滤 + onion_wall::gate + OnionGate::guard_decision" 保留为历史轨迹；追加 D8-fix 新版（锁 A 5 子 trait + 锁 B 6 子 trait + dispatcher AND + human_gate） |
| 6 | §2.3 衔接表 | 保留旧 13 行 `onion_wall/` 路径表为历史轨迹；追加 D8-fix 新版（`onion/principle/` + `onion/permission/` + `dispatcher.rs` + `human_gate.rs`） |
| 7 | §3 标题 | "onion_wall/" → "onion/"（标注形式，不删旧标题） |
| 8 | §3.1 子模块结构 | 旧 `onion_wall/` 12 文件树保留为引号块历史轨迹；追加 D8-fix 新版 `onion/` 树（principle/ 6 + permission/ 7 + dispatcher.rs + human_gate.rs） |
| 9 | §3.2 OnionGate trait | 旧 trait 签名 + 抽象层原则保留为引号块历史轨迹；追加 D8-fix 新版 PrincipleOnion + PermissionOnion + dispatcher 三 trait stub |
| 10 | §3.3 DecisionSignature | 旧 DecisionSignature struct + DecisionCategory enum 保留为引号块历史轨迹；追加 D8-fix 新版（5 + 6 = 11 个领域 Action struct 按层分布） |
| 11 | §3.4 守门映射表 | 旧 10 行 `OnionGate::guard_decision()` 集中入口映射保留为引号块历史轨迹；追加 D8-fix 新版（按"锁 A / 锁 B / HA / dispatcher"分布的 12 行映射） |
| 12 | §4 标题 + 迁移原则 | 旧措辞保留为历史轨迹 + D8-fix 标注（路径改 `onion/principle/keys.rs`，删除 `DecisionSignature` enum，主入口改 `PrincipleOnion::check_o_layer()`） |
| 13 | §6.1 衔接清单 | 旧 4 项保留为历史轨迹；追加 D8-fix 新版（trait 接口落地 / 子模块清单 / DB schema / crate 边界全部按 onion/principle/ + onion/permission/ 改写） |
| 14 | §7 anchor 表 | "§2 咬合形态"措辞保留为历史轨迹 + D8-fix 主人纠偏补充段 |

### 文件 2：`Apeireth-rust/crates/README.md`（5 处最小精化）

| # | 位置 | D8-fix 改动 |
|---|------|------|
| 1 | `apeireth-core` 行 | 旧"洋葱内墙模块（原则洋葱+权限洋葱交叉咬合）" → 新"双锁洋葱模块（锁 A 原则洋葱 5 重守门 + 锁 B 权限洋葱 Layer 0-6，独立检查 + AND 运算 + HA 硬门槛）" |
| 2 | `apeireth-philosophy` ~~行 | 追加 "R14-D8-fix 主人纠偏: 改为并入 core/onion/, 由两把独立锁构成" |
| 3 | §4 v2 收敛备选块 | 追加 R14-D8-fix 标注："principle/philosophy/permission 改为并入 `core/onion/principle/` 与 `core/onion/permission/`，哲学守门作为锁 A OLayerGuard 辅助语义网" |
| 4 | §3 30 crate v1 块 | "原则/权限洋葱内墙层... 合并到 core/onion_wall/" → "原则/权限洋葱双锁层... 合并到 core/onion/，分 principle/ 和 permission/ 两个子目录" |
| 5 | §5 R11 → 9 crates 映射表 | 追加 "(R14-D8-fix: → core/onion/principle/, 锁 A OLayerGuard 辅助语义网)" |

### 文件 3：`Apeireth-rust/docs/stage2-decisions-philosophy-guard.md`（2 处追加 D8-fix 标注）

| # | 位置 | D8-fix 改动 |
|---|------|------|
| 1 | 头部 R14-D8 主人精化勘误段 | 旧"原则洋葱+权限洋葱交叉咬合形成的'城堡内墙', 归入 onion_wall/" 保留为历史轨迹；追加 R14-D8-fix 主人纠偏段（两把独立锁 + AND 运算 + `onion/` + dispatcher + human_gate） |
| 2 | §2.1 段前 R14-D8 标注 | 旧 `onion_wall/keys` + `OnionGate::guard_decision(decision: DecisionSignature)` 保留为历史轨迹；追加 R14-D8-fix 主人纠偏标注（路径改 `onion/principle/keys.rs` + 拆为两把独立锁 + 5+6 Action struct） |

### 文件 4：`Apeireth-rust/docs/philosophy-traits-2026-07-30.md`（1 处追加 D8-fix 标注）

| # | 位置 | D8-fix 改动 |
|---|------|------|
| 1 | 头部 R14-D8 主人精化勘误段 | 旧"`OnionGate::guard_decision(decision: DecisionSignature)`"措辞保留为历史轨迹；追加 R14-D8-fix 主人纠偏段（拆为 `PrincipleOnion::check_o_layer()` + `PermissionOnion::check()` + `dispatcher::dispatch()`） |

### 文件 5：`Apeireth-rust/docs/stage2-decisions-addendum-sovereignty-continuity-governance.md`（3 处追加 D8-fix 标注）

| # | 位置 | D8-fix 改动 |
|---|------|------|
| 1 | §7.1 | 旧 R14-D7 精化段保留为历史轨迹；追加 D8-fix 主人纠偏标注（双洋葱正交 = 两把独立锁，**不是** R14-D8 错版的 per-layer 双重过滤 / 交叉咬合） |
| 2 | §7.3 | 旧 R14-D7 精化段保留为历史轨迹；追加 D8-fix 主人纠偏标注（HA 由 `onion/human_gate.rs` 单独实现，与 dispatcher 解耦） |
| 3 | §7.4 | §7.4 主体措辞**不精化**（与 D8-fix 完全一致），仅追加 D8-fix 一致性确认标注（锁 A 内部 5 子 trait 串行 AND 不与 §7.2 顶层 AND 冲突） |

### 文件 6：`Apeireth-rust/docs/stage2-decisions-addendum-sovereignty-continuity-governance.md` §7.5 错误示范
- ❌ **未修改**（主人 17:58 不假装错版示范是错的，与 D8-fix 无关）

### 文件 7：`Apeireth-rust/docs/stage2-decisions-addendum-sovereignty-continuity-governance.md` §7.2 AND 运算定义
- ❌ **未修改**（硬规则本身正确，D8-fix 与 §7.2 一致）

## 3. 完整目标文档 diff 摘要

> 报告文件自身是新增交付物，不纳入本节。

```text
Apeireth-rust/crates/README.md                                | 7 ++ 7 --
Apeireth-rust/docs/onion-wall-architecture-2026-07-31.md      | 336 ++ 118 --
Apeireth-rust/docs/philosophy-traits-2026-07-30.md           | 1 + 0 -
Apeireth-rust/docs/stage2-decisions-addendum-sovereignty...   | 6 + 0 -
Apeireth-rust/docs/stage2-decisions-philosophy-guard.md      | 8 + 0 -
```

## 4. 14 项自检清单

- [x] **文件 1** onion-wall-architecture-2026-07-31.md：14 处 D8-fix 标注（§1.1/§1.2/§1.3/§2 标题/§2.2/§2.3/§3 标题/§3.1/§3.2/§3.3/§3.4/§4 标题/§4 迁移原则/§6.1/§7），错版措辞全部保留为引号块或表格历史轨迹
- [x] **文件 2** crates/README.md：5 处最小精化措辞，cargo metadata `description` 字段未触碰
- [x] **文件 3** stage2-decisions-philosophy-guard.md：2 处追加 D8-fix 标注，原 21KB 主体 0 改动
- [x] **文件 4** philosophy-traits-2026-07-30.md：1 处追加 D8-fix 标注，原 trait 框架 0 改动
- [x] **文件 5** stage2-decisions-addendum-sovereignty-continuity-governance.md：3 处追加 D8-fix 标注（§7.1/§7.3/§7.4），§7.2 AND 硬规则不动 / §7.5 错误示范不动
- [x] **未写新 Rust 代码**（仅保留 trait 签名 stub + 新版 stub 追加，不写实现）
- [x] **未画 Mermaid 图**（仅 ASCII 简化示意 + 错版 ASCII 块保留为历史轨迹）
- [x] **未重写 V0.5 / V1136 / 哲学守门 9 键**（保留为历史轨迹 + D8-fix 标注）
- [x] **未修改其他 16 份 stage2 文档**（仅加 D8-fix 标注于 D2 addendum §7.1/§7.3/§7.4）
- [x] **未修改 crates/ 占位实现**（仅 crates/README 表格）
- [x] **未修改 cargo metadata `description` 字段**
- [x] **错版措辞保留为历史轨迹 + 加 D8-fix 纠错标注**（不删除错版）
- [x] **措辞最小精化**（仅追加 D8-fix 段 / 表格行 / 引号块；不重写章节标题，标题"内墙咬合形态"通过 D8-fix 标注形式保留 + 指出应改为"双锁独立形态"）
- [x] **主 17:58 不假装**：6 个主哲学 anchor 在 D8-fix 增量节中全贯穿

## 5. 边界声明锚点

- **主 17:58 不假装**: 错版措辞"per-layer 双重过滤 / 交叉咬合 / onion_wall/ / OnionGate 联合守门 / DecisionSignature 14+ 守卫"**完整保留**为历史轨迹；不删除、不假装"以前没说错"
- **主 17:43 实事求是**: 错版是 Leader 提议 + 我执行，未对齐阶段 2 §10 decision-system Phase 1；主人纠偏后改为两把独立锁，不假装"原来就对"
- **主 19:33 走在前人经验上**: 两把独立锁 + AND 运算 = 经典的"AND 门"安全架构思路，借鉴权限模型的"分层 + AND"模式
- **主 22:33 ASI 北极星**: 锁 A 原则洋葱 5 重守门（E/S/A/M/O）+ 锁 B 权限洋葱 Layer 0-6 + HA 硬门槛 = 三层防御，保留最后护栏
- **主 23:44 干到底**: 5 文件 + 1 报告完整落地；旧措辞 0 删除；D8-fix 标注 14 处全部就位
- **主 00:56 任何人都能接手**: 错版 + 新版 + 关键区别段并列存在，演化脉络清晰可读

## 6. 不做的事清单（主 17:58 不假装）

- ❌ 不删除 R14-D8 错版（交叉咬合 / onion_wall/ / OnionGate / DecisionSignature 等所有错版措辞完整保留）
- ❌ 不重写 §7.2 AND 运算硬规则（这是对的，D8-fix 与之完全对齐）
- ❌ 不修改 §7.5 错误示范（这是对的，与 D8-fix 无关）
- ❌ 不写新 Rust 代码（保留原 trait 签名 stub + 新版 stub 追加）
- ❌ 不画 Mermaid 图（ASCII 简化示意 + 引号块历史轨迹）
- ❌ 不重写 V0.5 / V1136 / 哲学守门 9 键
- ❌ 不修改其他 16 份 stage2 文档（仅加 D8-fix 标注于 D2 addendum §7.1/§7.3/§7.4）
- ❌ 不修改 crates/ 占位实现（仅 crates/README 表格）
- ❌ 不修改 cargo metadata `description` 字段
- ❌ 不重写章节标题（"内墙咬合形态"通过 D8-fix 标注形式保留 + 指出应改为"双锁独立形态"）

## 7. 结论

R14-D8-Fix 按主人 2026-07-31 同日纠偏的"两把独立锁"设计，对 5 份文档做最小精化措辞 + D8-fix 纠错标注追加。错版措辞（per-layer 双重过滤 / 交叉咬合 / onion_wall/ / OnionGate 联合守门 / DecisionSignature 14+ 守卫）完整保留为历史轨迹，不删除不假装"以前没说错"。新版（锁 A 5 子 trait + 锁 B 6 子 trait + dispatcher AND + human_gate）与阶段 2 §10 decision-system Phase 1 已落的设计精确对齐，为阶段 4 SCHEMA.md / ADR.md 写作提供清晰的"两把独立锁"语义层级结构。
