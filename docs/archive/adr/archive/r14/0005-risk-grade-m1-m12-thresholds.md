# ADR-0005: 风险分级 M1-M12 阈值定义 (Risk Grade M1-M12 Thresholds)

**状态**: ✅ Accepted
**日期**: 2026-08-03
**作者**: architect2 (claude-sonnet-4.5, Ponytail: full)
**任务**: round8-08 ADR 0003-0006 补齐 (V28.1 增量)
**依据**: docs/stage4/stage4-correction-v15-four-gates-permission-grant.md + round8-05 constraint 实装
**影响范围**: 仅文档新增, 不修改 LOCKED, 不修改源

---

## 上下文 (Context)

权限洋葱 v15 (FourGates + PermissionGrant) 引入风险分级 M1-M12 机制:
- M1 = 读取操作 (低风险)
- M6 = 写入操作 (中风险)
- M12 = 自修改 + OTA 升级 (高风险, 需 HA M-of-N)

但阈值定义散落在 round8-05 (security_reviewer) 实装中, 未统一文档化.

## 决策 (Decision)

**12 个风险等级**, 每级对应 4 重守门通过条件 + HA 审批要求:

| 等级 | 操作类型 | FourGates | HA 审批 | 示例 |
|------|----------|-----------|---------|------|
| **M1** | 纯读取 | 1+2 pass | 0 | dim_by_name 查询 |
| **M2** | 缓存查询 | 1+2 pass | 0 | 12 keys O(1) cache |
| **M3** | 内部状态读 | 1+2+3 pass | 0 | Maturity 链接查询 |
| **M4** | 内部写入 | 1+2+3 pass | 1 人 | Supervisor 子树更新 |
| **M5** | 跨 crate 读 | 1+2+3 pass | 0 | Council 7 advisor 同步 |
| **M6** | 跨 crate 写 | 1+2+3+4 pass | 1 人 | 四守门 cross_crate |
| **M7** | PyBridge 调用 | 1+2+3+4 pass | 2 人 | LLM 推理触发 |
| **M8** | 决策系统改 | 1+2+3+4 pass | 2 人 | Cognition 6 状态机 |
| **M9** | 守门规则改 | 1+2+3+4 pass | 3 人 | PermissionGrant 变更 |
| **M10** | 器官新增 | 1+2+3+4 pass | 3 人 | 新 crate 注册 |
| **M11** | 自我修改 | 1+2+3+4 pass | 4 人 | SelfModification |
| **M12** | OTA 升级 | 1+2+3+4 pass | M-of-N | 7 阶段 OTA 跨 crate |

## 取舍 (Consequences)

**优点**:
1. 12 等级与人脑数字记忆友好
2. 等级 ↔ 守门 ↔ HA 审批 三维对齐
3. M12 OTA 强制 M-of-N 多签 (round6-01 HA 已实装)

**缺点**:
1. M7-M12 HA 审批人数递增, 可能降低效率 (但符合 L0 守门)
2. 12 等级需共识, 后续扩展 (M13+) 需新 ADR

## 守门 (Guardrails)

- 等级划分不可绕过 (FourGates 强制检查)
- HA 审批人数 = 等级 - 6 (M7=1, M8=2, M9=3, M10=4, M11=4, M12=M-of-N)
- M-of-N 阈值不可低于 3 (round6-01 HA 最低要求)
- 不修改 docs/stage4-v15 LOCKED

## 验证 (Verification)

- round12-02 (security_reviewer) FiveGates M1-M12 真实场景 24 测试已覆盖
- round8-05 (security_reviewer) constraint 5 重守门实装已 M1-M12 分级
- round10-10 (architect2) cross_crate M-of-N 已对接 sovereignty

## 后续 (Follow-ups)

- 在 `crates/apeireth-constraint/src/lib.rs` 添加 `RiskGrade` enum + 12 变体
- round13-XX 派活若新增等级, 必须先更新本 ADR
- CI 添加 "HA 审批人数 ≥ RiskGrade - 6" 校验