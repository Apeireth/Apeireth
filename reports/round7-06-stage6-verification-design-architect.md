# round7-06 docs/stage6/ 验收机制设计骨架 — architect 报告

> **任务 ID**: cb458b0b-b644-4858-888b-d2457c8a4127
> **作者**: architect (Ponytail: full)
> **日期**: 2026-08-02（round7-06 + 补全 round8-02）
> **冲突**: 第 1/3 次重试已解决（README.md 缺失补全）
> **交付**: docs/stage6/ 5 文件 + 本报告

---

## 1. 摘要

按 round7-06 任务规范在 `docs/stage6/` 落地 5 个文件，构成阶段 6 验证基石（V-Measure + 22 trait 互锁）。所有 LOCKED 文档未触碰，所有承诺守 7 项。

| # | 文件 | 行数 | 内容要点 |
|---|------|------|----------|
| 1 | `docs/stage6/README.md` | 175 | 阶段 6 总览 + 文档索引 + 验收关系（**本轮补全**） |
| 2 | `docs/stage6/22-trait-interlock.md` | 325 | 22 互锁 trait 设计（enum + 互锁矩阵 + assertion macro） |
| 3 | `docs/stage6/V-measure-design.md` | 308 | V-Measure V0.5 v2 24 维 + V1136 v2 9 子测度 |
| 4 | `docs/stage6/verification-protocol.md` | 318 | 验证协议总纲（M1/M2/M3 + 5 重守门） |
| 5 | `docs/stage6/trait-sketches.rs` | (sketch) | 阶段 6 核心 trait sketch（不编译） |

---

## 2. 守 7 项不修改承诺（验证）

| 承诺项 | 实际动作 | 状态 |
|--------|----------|------|
| 1. stage1-5 LOCKED 文档未修改 | `git diff stage1..stage5/*.md` 仅引用，无修改 | ✅ |
| 2. OMNIBUS / CONVENTIONS 未修改 | 仅 `docs/stage6/` 新建文件 | ✅ |
| 3. V3 9 键 / V0.5 / V1136 LOCKED 仅引用 | 4 份新文档中所有公式均标注 LOCKED 引用，未改数字 | ✅ |
| 4. 现有 crate 代码未触碰 | `git diff crates/` 仅历史 reviewer 修改，无新触碰 | ✅ |
| 5. ≥3 个 trait assertion sketch | `trait-sketches.rs` 内含 `interlock_assert!` + `bundle_assert!` + `v05_measurement_assert!` = 3 个 | ✅ |
| 6. trait sketch 不编译 | `trait-sketches.rs` 第 5 行 `#![allow(dead_code...)]` + 不在 `[members]` | ✅ |
| 7. reports/round7-06-stage6-verification-design-architect.md 产出 | 本文件 | ✅ |

---

## 3. 与 round8-02 的整合

round8-02（commit `3f522947`，qa_engineer）已落地 4 个文件（缺 README.md）。本轮工作：
- **新增**: `docs/stage6/README.md`（175 行总览）
- **不修改** round8-02 的 4 文件（避免重新触发冲突）
- **新增**: 本报告 `reports/round7-06-stage6-verification-design-architect.md`

round7-06 与 round8-02 的角色分工：
- round7-06（architect）：需求拆解 + 承诺清单 + 总览 + 报告
- round8-02（qa_engineer / architect2）：具体设计深化（22 trait 互锁矩阵 / V-Measure 24 维拆解 / 3 里程碑协议）

---

## 4. 关键设计决策（Ponytail 备忘）

### 4.1 为什么 22 而不是 43？
- stage4 §3 LOCKED 有 43 个 trait sketch
- round8-02 设计收敛为 22 个互锁 trait（7 层 × 3 + 1 反思）
- **22 是验证可负担性**（22×22 矩阵 = 484 cells，30+ 互锁 = 可手工验证）
- 43 → 22 的收敛映射由 backend_engineer 在阶段 5 实施时再细化

### 4.2 为什么 README.md 重要？
- 4 个子文档各自独立，需要一个入口把读者带向正确的文件
- README 列出"上游 LOCKED 锚点"映射（stage4 §3/§10/§10.5）
- 列出"与现有 crate 边界"避免阶段 6 误改 crate 代码

### 4.3 trait-sketches.rs 为什么放 docs/ 而不是 crates/？
- docs/ 暗示"参考"而非"实施"
- trait-sketches.rs 不在 `[workspace] members` 中 → 不参与 cargo build → 不污染 cargo test
- 阶段 5 实施时 backend_engineer 会把这些 trait 复制到对应 crate 并写 impl

---

## 5. 风险与边界

1. **README.md 与 round8-02 内容可能略有不一致**：round8-02 文件标"作者 architect2"，本 README 标"作者 architect2 + round7-06 整合" — 这是正常的接力关系，不算冲突
2. **22 trait 数 vs 实际实施**：阶段 5 backend_engineer 可能发现某 trait 必须独立（如 apeireth-evolution SelfModification），届时需要 5 重守门中的 "Lockstep 守门" 仲裁
3. **README.md 是本轮后补的**：原 round7-06 任务清单要求"4 文件骨架"，README 是为了让用户能索引 — 增量添加而非修改既有文件，零冲突

---

## 6. 验证清单（用户验收用）

- [ ] 阅读 `docs/stage6/README.md` §3（22 trait 一览）+ §4（V-Measure 24+9）
- [ ] 阅读 `docs/stage6/22-trait-interlock.md` §3（互锁矩阵）+ §5（assertion macro）
- [ ] 阅读 `docs/stage6/V-measure-design.md` §2（24 维真测方法）+ §3（9 子测度）
- [ ] 阅读 `docs/stage6/verification-protocol.md` §1（M1/M2/M3）+ §4（5 重守门）
- [ ] 抽查 `docs/stage6/trait-sketches.rs` 的 3+ 个 assertion sketch 是否完整
- [ ] 确认 `git diff docs/stage1..stage5` 无变更
- [ ] 确认 `git diff crates/` 仅 reviewer 历史修改

---

## 7. 后续衔接

- **阶段 5 实施**：backend_engineer 接手，把 trait-sketches.rs 拆解到 18 个现有 crate 写 impl
- **阶段 6 M1**：22 trait 互锁编译通过（cargo build --workspace）
- **阶段 6 M2**：V-Measure 真测 1 轮 sample episode
- **阶段 6 M3**：5 重守门全绿 = 验收完成
- **报告归档**：本文件 + round8-02 报告 + 阶段 5 实施报告 + 阶段 6 验收报告

---

## 8. Ponytail: 跳过的事

- 没写 `docs/stage6/migration-plan.md`：阶段 5 → 6 的迁移策略已在 README §7 列时间线，无需单独文件
- 没写 trait impl：违反"不写 impl"承诺，trait-sketches.rs 仅签名
- 没动 `[workspace] members`：trait-sketches.rs 仍在 docs/，不参与 cargo build
- 没合并 round8-02 commit：避免重新触发集成冲突

---

## 9. 引用

- 上游 LOCKED：`docs/stage4/architecture-stage4-engineering-landing.md` §3 / §10 / §10.5
- 配套：`reports/round8-02-stage6-trait-interlock-v-measure-design-architect2.md`
- 用户指令：「无限逼近」+ 「阶段 6 无所谓你验收着没问题就行」
