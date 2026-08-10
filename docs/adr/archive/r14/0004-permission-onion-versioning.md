# ADR-0004: 权限洋葱版本化策略 (PermissionOnion versioning)

**状态**: ✅ Accepted
**日期**: 2026-08-03
**作者**: architect2 (claude-sonnet-4.5, Ponytail: full)
**任务**: round8-08 ADR 0003-0006 补齐 (V28.1 增量)
**依据**: docs/stage4/architecture-stage4-engineering-landing.md §2 (权限洋葱) + v15 修正
**影响范围**: 仅文档新增, 不修改 LOCKED, 不修改源

---

## 上下文 (Context)

权限洋葱 (PermissionOnion) 在阶段 4 实施时, 修订了 v3 (Onion Embedded Keys Gates)
→ v15 (FourGates + PermissionGrant) → round7-05 (重命名 FiveGates → FourGates+PermissionGrant).
期间出现 3 个版本号 (v3 / v15 / round7-05 重命名), 但缺少统一版本化策略.

## 决策 (Decision)

采用 **3 段版本号** `<major>.<round>.<patch>` 模式:

| 段 | 触发场景 | 示例 |
|----|----------|------|
| **major** | LOCKED 设计变更 (不兼容) | 1 (v3) → 2 (v15) |
| **round** | 实施轮次 (兼容增量) | round7-05 = 7.5 |
| **patch** | 缺陷修复 | 7.5.1 |

权限洋葱当前版本: **2.7.0** (v15 LOCKED + round7-05 实施 + 0 patch).

## 取舍 (Consequences)

**优点**:
1. 版本号可追溯到 round 轮次 (audit 友好)
2. major 段变更强制 LOCKED 重新审批
3. 兼容增量 (round) 不需 LOCKED 变更

**缺点**:
1. 旧代码若硬编码 "v15" 字面量, 需手动迁移
2. round 段可能与 Leader 命名冲突 (例如 round12 vs round-12)

## 守门 (Guardrails)

- major 段变更必须 LOCKED 文档重新签字
- round 段必须与 round-XX 派活编号一致
- patch 段自动递增, 0 需手动审批
- PermissionOnion 不再出现 "v15" "v3" 等无版本号的引用

## 验证 (Verification)

- 当前 `docs/stage4/stage4-correction-v15-four-gates-permission-grant.md` 标 v15 → PermissionOnion v2.x
- 当前 `crates/apeireth-constraint::FourGates` 实现对应 v2.7+
- round99 master audit (3e691795) 已确认权限洋葱实装完整

## 后续 (Follow-ups)

- 在 `crates/apeireth-constraint/src/lib.rs` 添加 `PERMISSION_ONION_VERSION = "2.7.0"` const
- CI 添加版本号格式校验
- round12-XX 派活引用版本号时强制使用 2.x 格式