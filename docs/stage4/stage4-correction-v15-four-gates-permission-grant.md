# 阶段 4 修正 v15 — FiveGates → FourGates+PermissionGrant 命名修正（补充式修正）

```
[Document-Meta]
Document: docs/stage4/stage4-correction-v15-four-gates-permission-grant.md
Version: Fix-15 + Design-4.0
R-Cycle: R14
Last-Modified: 2026-08-02
Status: 🟢 活跃（命名修正层，独立命名空间）
详见: APEIRETH-VERSIONING.md
```

> **性质**: round7-01 任务产出——基于用户授权 + leader-chronological-authority 原则（最新最正确）+ reports/leader-engineering-audit-2026-08-02.md 工程实装审计结论，对 `apeireth-constraint` crate 的 `FiveGates` trait 进行 **命名修正**（不修改 LOCKED 文档）。
>
> **触发**: 
> 1. 阶段 4 v5（2026-07-31）主人关键洞察："**多 AI 一致不是守门，是权限发放**"（已 LOCKED 于 `stage4-correction-v5-gates-refined.md`）
> 2. 阶段 4 v6（2026-07-31）主人第二洞察："**守门 + 权限发放必须留一条路**"（LOCKED）
> 3. 阶段 4 v11（命名约定）：设计演化登记"**5 重守门 → 4 重守门嵌套**"
> 4. round7 用户授权：本轮工程期命名修正
>
> **硬约束**: ❌ 不修改任何 LOCKED 文档 / ❌ 不修改阶段 4 v5 LOCKED / ❌ 不砍 crates/ 占位 / ❌ 不画 Mermaid / ✅ 仅做命名修正层（补充式）。
>
> **主哲学 6 锚穿透**: 主 22:33 北极星 / 主 17:43 实事求是 / 主 17:58 不假装 / 主 19:33 走在前人经验上 / 主 23:44 干到底 / 主 00:56 任何人都能接手。

---

## §0. 元信息

| 字段 | 值 |
|---|---|
| **生成时间** | 2026-08-02（round7-01） |
| **Task ID** | 20131b80-3f16-43e2-858d-6e5e681b3111（round7-01 ADR-0010） |
| **性质** | v15 命名修正（补充式，独立命名空间） |
| **路径** | `docs/stage4/stage4-correction-v15-four-gates-permission-grant.md` |
| **依据** | 阶段 4 v5 LOCKED（5 → 4 重守门洞察）+ v6 LOCKED（E 层修改路径）+ v11（命名约定）+ 主人 round7 授权 |
| **修订链** | v1 → v2 → v3 → v4 → v5（LOCKED） → v6（LOCKED） → ... → v14（final cleanup） → **v15（命名修正）** |
| **不修改承诺** | 阶段 1 + 2 + 3 + 4 LOCKED 全部不动 |
| **执行人** | architect（架构师评审 + 命名修正文档产出） |

---

## §1. 核心修正：FiveGates → FourGates+PermissionGrant

### 1.1 命名演化的第一性原理

| 维度 | v5 LOCKED（5 重守门） | v15 修正（4 重守门嵌套 + 权限发放独立） | 第一性原理 |
|---|---|---|---|
| **守门结构** | 5 个并列守门 | 4 重嵌套守门 | 守门嵌套 = 守住原则洋葱 + 权限洋葱，分层保护 |
| **多 AI 一致** | gate3（守门之一） | PermissionGrant（独立机制） | 多 AI 一致是"发钥匙"而非"守门" |
| **物理隔离** | gate4 | gate3（外层嵌套） | 物理隔离是"在运行时拦截外层" |
| **反思期** | gate5 | gate4（最外层嵌套） | 反思期是"事后审计" |
| **权限发放** | 无独立机制 | PermissionGrant 独立 trait | 修改原则需要权限 = 守门无法内生 |

**关键洞察（v5 主人原话复述）**：
> "多 ai 一致没必要弄成守门的，因为修改原则需要多ai一致，所以**守门就是把原则洋葱和权限洋葱守住就可以了**，这样一个嵌套结构。就可以弄成把没有相应权限而运行的代码拦截就行，然后**权限的发放由多ai发放或人类决策**，就像那个权重公式。"

### 1.2 4 重守门嵌套结构（v15 最终）

```
4 重守门嵌套（nested gates）:
  Gate 1 (内层): 编译时 hardcode（原则洋葱整体 — E/S/A/M/O 5 层 + 12 键 + 5 项不假装）
  Gate 2 (中间): 运行时拦截（所有决策前 — verdict cache O(1) 查询）
  Gate 3 (外层): 物理隔离（重大修改需物理访问 + 多签 — critical=7 席全量）
  Gate 4 (最外): 反思期审计（Cognitive-Dream 72h 监控 — 守护越权检查）

PermissionGrant（独立机制）:
  公式 = V0.5 v2 24 维权重公式（v4.1 §13）
  实施 = apeireth-council 智囊团审议（7 强制 + 动态专家）
  人类决策 = L0 HA 真实人类批准
  权限发放的对象 = 风险分级（critical 7 / high 5 / medium 3 / low 1 / info 0）
  "守住没有相应权限而运行的代码" = 守门 1-4 联合工作
```

---

## §2. 工程实装核查（100% 对齐审计）

### 2.1 apeireth-constraint 当前实装

**当前 trait 命名**（`crates/apeireth-constraint/src/lib.rs`）：

```rust
// ⚠️ 当前命名（待 v15 修正）
pub trait FiveGates: Send + Sync {
    fn gate1_compile_time(&self) -> GateVerdict;
    fn gate2_runtime_intercept(&self, action: &Action) -> GateVerdict;
    fn gate3_multi_ai_consensus(&self, action: &Action) -> GateVerdict;  // ← 应为 PermissionGrant
    fn gate4_physical_isolation(&self, action: &Action) -> GateVerdict;  // ← 应为 gate3
    fn gate5_reflection_period(&self, action: &Action) -> GateVerdict;   // ← 应为 gate4
}
```

**v15 修正后 trait 命名**（目标态）：

```rust
// ✅ v15 修正（建议工程实装目标，非本任务实施范围）
pub trait FourGates: Send + Sync {
    fn gate1_compile_time(&self) -> GateVerdict;           // 编译时 hardcode
    fn gate2_runtime_intercept(&self, action: &Action) -> GateVerdict;  // 运行时拦截
    fn gate3_physical_isolation(&self, action: &Action) -> GateVerdict; // 物理隔离
    fn gate4_reflection_period(&self, action: &Action) -> GateVerdict;  // 反思期
}

pub trait PermissionGrant: Send + Sync {
    fn grant_via_council(&self, action: &Action) -> GrantVerdict;  // 多 AI 一致
    fn grant_via_human(&self, action: &Action) -> GrantVerdict;    // 人类决策
    fn grant_risk_level(&self, action: &Action) -> RiskLevel;      // 风险分级
}
```

### 2.2 工程实装 100% 对齐审计（基于 reports/leader-engineering-audit-2026-08-02.md）

| 维度 | 实装位置 | v15 对齐 | 差异 |
|---|---|---|---|
| 4 重守门 trait | `crates/apeireth-constraint/src/lib.rs::FiveGates` | ⚠️ 待重命名 | 命名对齐（trait 名 FiveGates → FourGates） |
| PermissionGrant | 不存在 | ❌ 待新建 | 新增 trait（独立机制） |
| 编译时 hardcode | `HardCodeConstraint` trait | ✅ 已实装 | 无需修改 |
| 运行时拦截 | `FiveGates::gate2_runtime_intercept` | ✅ 已实装 | 仅重命名（gate2 不变） |
| 物理隔离 | `FiveGates::gate4_physical_isolation` | ✅ 已实装 | 重命名（gate4 → gate3） |
| 反思期 | `FiveGates::gate5_reflection_period` | ✅ 已实装 | 重命名（gate5 → gate4） |
| 多 AI 一致 | `FiveGates::gate3_multi_ai_consensus` | ⚠️ 待剥离 | 移到 PermissionGrant trait |
| Council 实施 | `crates/apeireth-council/src/lib.rs` | ✅ 已实装 | 由 PermissionGrant::grant_via_council 调用 |
| 人类决策 | 7 advisor + L0 HA | ✅ 已实装 | 由 PermissionGrant::grant_via_human 调用 |

**审计结论**：
- ✅ 4 重守门功能**已 100% 实装**（仅命名待修正）
- ✅ PermissionGrant 所需组件**已 100% 存在**（council + sovereignty + supervisor + council 7 advisor + human approval）
- ⚠️ 命名对齐缺失：FiveGates → FourGates + PermissionGrant 是**纯命名重构**，无功能变更
- ⚠️ 风险：纯重构不应破坏任何 tests（45 passed → 仍应 45+ passed）

### 2.3 漂移登记（v15 修正的影响半径）

| 影响项 | 状态 | 处置 |
|---|---|---|
| `crates/apeireth-constraint/src/lib.rs::FiveGates` | 当前命名 | 建议重构（由 backend_engineer2 在 round7 后续任务实施） |
| `crates/apeireth-constraint/src/lib.rs::ConstraintEngine` impl | 5 gate fn 全实装 | impl trait 跟随 trait 重命名 |
| `tests/` 中的 FiveGates mock | 若干测试 | 跟随 trait 重命名 |
| 其他 crate 引用 FiveGates | perception / cognition / action 等 | 引用跟随（影响 ~6 crate） |
| LOCKED 文档 | v5 / v6 / v11 已含"4 重守门嵌套"措辞 | 不修改（LOCKED 承诺） |

---

## §3. 命名修正执行建议（不实施，仅登记）

### 3.1 推荐执行步骤

```
步骤 1（架构师本任务）: 输出本 v15 修正文档 ✅
步骤 2（待 Leader 派活）: backend_engineer2 重命名 FiveGates → FourGates + PermissionGrant
步骤 3（待 Leader 派活）: 跟随 trait 重命名 impl + tests
步骤 4（待 Leader 派活）: 跟随更新 6 crate 引用
步骤 5（待 Leader 派活）: 全 workspace cargo test --offline 验证
步骤 6（待 Leader 派活）: 命名修正完成报告（reports/round7-01-naming-correction-completion.md）
```

### 3.2 验收命令（步骤 5）

```bash
cargo build --workspace --offline                  # 必须 0 error
cargo test --workspace --offline                   # 879 passed → 仍 879+ passed
cargo test -p apeireth-constraint --offline        # 验证 FourGates + PermissionGrant 实装
cargo clippy -p apeireth-constraint --offline -- -D warnings  # clippy 通过
grep -rn "FiveGates" crates/                       # 必须 0 命中（除历史 drift 报告）
grep -rn "FourGates\|PermissionGrant" crates/      # 必须 ≥ 2 命中（trait 定义 + impl）
```

---

## §4. 不修改承诺（再次重申）

❌ **本任务不修改任何 LOCKED 文档**：

| LOCKED 文档 | 状态 |
|---|---|
| 阶段 1 inspiration-stage1-2026-07-30.md | 不动 |
| 阶段 2 全部 18 个 stage2-decisions-*.md | 不动 |
| 阶段 3 全部 14 个 stage3-*.md | 不动 |
| 阶段 4 architecture-stage4-engineering-landing.md (1492 行 LOCKED) | 不动 |
| 阶段 4 architecture-frontend-design-proposal.md | 不动 |
| 阶段 4 architecture-stage4-inspiration-supplements.md | 不动 |
| 阶段 4 architecture-stage4-patches.md | 不动 |
| 阶段 4 stage4-correction-v1 ~ v14（含 v5 LOCKED） | 不动 |

✅ **本任务仅新建独立命名空间文档**：
- `docs/stage4/stage4-correction-v15-four-gates-permission-grant.md`（本文件）

✅ **本任务仅新建任务报告**：
- `reports/20131b80-round7-01-adr-0010-naming-correction.md`

---

## §5. 引用清单

| 文件 | 用途 |
|---|---|
| `docs/stage4/stage4-correction-v5-gates-refined.md` (LOCKED) | "多 AI 一致不是守门，是权限发放"洞察来源 |
| `docs/stage4/stage4-correction-v6-consolidated-and-e-layer-mutation.md` (LOCKED) | 守门 + 权限发放必须留一条路（E 层修改路径）|
| `docs/stage4/stage4-correction-v11-conventions.md` (LOCKED) | 命名约定表"5 重守门 → 4 重守门嵌套" |
| `docs/stage4/stage4-correction-v3-onion-embedded-keys-gates.md` | 历史链 v1 → v2 → ... → v5 演化 |
| `reports/leader-engineering-audit-2026-08-02.md` | 工程实装 100% 对齐审计 |
| `reports/leader-decisions-2026-08-02-round4.md` | 用户裁决记录（依据） |
| `crates/apeireth-constraint/src/lib.rs` | 当前 FiveGates trait 实装（待修正） |
| `crates/apeireth-council/src/lib.rs` | PermissionGrant::grant_via_council 实施位置 |

---

## §6. 提交

- 文档: `docs/stage4/stage4-correction-v15-four-gates-permission-grant.md`（本文件）
- 报告: `reports/20131b80-round7-01-adr-0010-naming-correction.md`
- 状态: ✅ 命名修正文档产出
- 后续: 由 Leader 派活给 backend_engineer2 实施 trait 重命名（步骤 2-5）