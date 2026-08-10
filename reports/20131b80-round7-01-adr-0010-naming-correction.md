# 20131b80 round7-01 ADR-0010 阶段 4 v15 命名修正任务报告

**Task ID**: 20131b80-3f16-43e2-858d-6e5e681b3111  
**Role**: architect  
**Status**: ✅ 完成（命名修正文档产出，未实施代码）

---

## 0. 任务范围

**目标**：基于用户授权 + leader-chronological-authority 原则 + reports/leader-engineering-audit-2026-08-02.md：
1. 新建 docs/stage4/stage4-correction-v15-four-gates-permission-grant.md 补充式修正
2. 说明 5 重守门→4 重守门嵌套 + 权限发放命名（最新最正确，符合第一性原理）
3. 工程实装核查：apeireth-constraint 当前 FiveGates trait 包含 5 项能力
4. 引用 § 工程实装 100% 对齐审计报告
5. 不修改任何 LOCKED 文档

**约束**：
- ❌ 不修改阶段 4 v5 LOCKED
- ❌ 不修改任何 LOCKED 文档
- ✅ 仅做命名修正层（补充式）
- ❌ 不实施代码

---

## 1. 交付物

### 1.1 主文档
**`docs/stage4/stage4-correction-v15-four-gates-permission-grant.md`** (11223 bytes)
- §0 元信息（v15 命名修正 + 不修改承诺）
- §1 核心修正：5→4 重守门 + 权限发放独立
- §2 工程实装核查（100% 对齐审计）
- §3 命名修正执行建议（不实施，仅登记）
- §4 不修改承诺
- §5 引用清单
- §6 提交

### 1.2 任务报告
**`reports/20131b80-round7-01-adr-0010-naming-correction.md`**（本文件）

---

## 2. 核心修正结论

### 2.1 命名演化第一性原理

| v5 LOCKED | v15 修正 | 第一性原理 |
|---|---|---|
| 5 个并列守门 | 4 重嵌套守门 | 守门嵌套 = 守住原则洋葱 + 权限洋葱 |
| 多 AI 一致 = gate3 | PermissionGrant（独立） | 多 AI 一致是"发钥匙"非"守门" |
| 物理隔离 = gate4 | gate3（外层嵌套） | 物理隔离是"运行时拦截外层" |
| 反思期 = gate5 | gate4（最外层） | 反思期是"事后审计" |

### 2.2 4 重守门嵌套结构（v15 最终）

```
Gate 1 (内层):   编译时 hardcode（原则洋葱整体）
Gate 2 (中间):   运行时拦截（verdict cache O(1)）
Gate 3 (外层):   物理隔离（重大修改多签）
Gate 4 (最外):   反思期审计（Cognitive-Dream 72h）

PermissionGrant (独立机制):
  Council (7 强制) + Human (L0 HA) + RiskLevel (critical/high/medium/low/info)
```

---

## 3. 工程实装 100% 对齐审计

### 3.1 当前实装状态

| 组件 | 当前 | v15 对齐 | 差异 |
|---|---|---|---|
| `FiveGates` trait | `crates/apeireth-constraint/src/lib.rs` | ⚠️ 待重命名 | FiveGates → FourGates |
| 编译时 hardcode | `HardCodeConstraint` trait | ✅ | 无需改 |
| 运行时拦截 | `gate2_runtime_intercept` | ✅ | 仅跟随重命名 |
| 物理隔离 | `gate4_physical_isolation` | ✅ | gate4 → gate3 |
| 反思期 | `gate5_reflection_period` | ✅ | gate5 → gate4 |
| 多 AI 一致 | `gate3_multi_ai_consensus` | ⚠️ 待剥离 | 移到 PermissionGrant |
| PermissionGrant trait | ❌ 不存在 | ❌ 待新建 | 新增 trait |
| Council 实施 | `apeireth-council` | ✅ | grant_via_council 调用 |
| 人类决策 | 7 advisor + L0 HA | ✅ | grant_via_human 调用 |

### 3.2 审计结论

- ✅ 4 重守门功能**已 100% 实装**（仅命名待修正）
- ✅ PermissionGrant 所需组件**已 100% 存在**（council + sovereignty + supervisor + human approval）
- ⚠️ 命名对齐缺失：纯命名重构，无功能变更
- ⚠️ 风险：纯重构不应破坏任何 tests（45 passed → 仍应 45+ passed）

---

## 4. 执行步骤建议（待 Leader 派活）

```
步骤 1（本任务）: 输出 v15 修正文档 ✅
步骤 2-5（待派活）:
  - 重命名 trait FiveGates → FourGates + 新建 PermissionGrant
  - 跟随 impl + tests 重命名
  - 更新 6 crate 引用
  - cargo test --workspace 验证
```

### 4.1 验收命令（步骤 5）

```bash
cargo build --workspace --offline                  # 必须 0 error
cargo test --workspace --offline                   # 879 passed → 仍 879+ passed
cargo test -p apeireth-constraint --offline        # 验证新 trait 实装
cargo clippy -p apeireth-constraint --offline -- -D warnings  # clippy 通过
grep -rn "FiveGates" crates/                       # 必须 0 命中（除历史 drift 报告）
grep -rn "FourGates\|PermissionGrant" crates/      # 必须 ≥ 2 命中
```

---

## 5. LOCKED 边界（不修改承诺）

❌ **不修改**：
- 阶段 1 LOCKED（1 文件）
- 阶段 2 LOCKED（18 文件）
- 阶段 3 LOCKED（14 文件）
- 阶段 4 LOCKED（4 主文档 + v1-v14 修正链 = 18 文件）

✅ **新建**：
- `docs/stage4/stage4-correction-v15-four-gates-permission-grant.md`（独立命名空间）
- `reports/20131b80-round7-01-adr-0010-naming-correction.md`（本文件）

---

## 6. 提交

- **状态**: ✅ 完成
- **不越界**: 仅命名修正文档，未实施代码
- **后续**: 由 Leader 派活给 backend_engineer2 实施 trait 重命名