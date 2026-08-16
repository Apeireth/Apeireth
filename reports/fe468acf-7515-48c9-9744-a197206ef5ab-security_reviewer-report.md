# 自审报告 — 任务 fe468acf: toolApprovalManager 增强吸收 (命令级粒度 + 静默拒绝 + 结构化拒绝)

> 角色: security_reviewer | 日期: 2026-08-17 | crate: apeireth-tool-approval (独占改动, 其他 crate 零触碰)

## 1. 任务目标 → 交付对照

| 验收项 | 状态 | 证据 |
|---|---|---|
| 命令级粒度审批键 `Tool:command` | ✅ | `ApprovalListRule` (rule.rs): specificity 2 > 1, 同级静默优先 — 对照 VCP `considerMatch`; 命令从 args `command`/`command1..N` 提取 (`extract_commands`, 对照 VCP `extractCommands:93-115`) |
| 静默拒绝不打扰主人/AI | ✅ | `::SilentReject` 后缀条目命中被拒 → `silent=true`; 集成测试 `silent_reject_full_flow_audit_trail` |
| 静默拒绝留痕审计 | ✅ | `ApprovalAuditEntry` 审计台账 (上限 1000, 每个终态留痕) + `silent_rejection_audit()` 静默视图 |
| 结构化拒绝 `{rejected_by_user, error_type}` | ✅ | `wait_for_approval_outcome` → `ApprovalOutcome`/`Rejection`; 四错误码: rejected_by_user / approval_timeout / policy_deny / channel_unavailable |
| 高危仍走主人批准通道 (洋葱安全) | ✅ | 审批清单命中 → `RequireApproval` → handler 通道; 无 handler → `ChannelUnavailable` 拒绝 (fail-safe, 绝不放行); 集成测试 `onion_safety_high_risk_still_requires_master` |
| 批准/拒绝/静默拒绝/错误码/命令级匹配各路径测试 | ✅ | 见 §2 |
| 其他 crate 不改 | ✅ | 仅改 `crates/apeireth-tool-approval/**`; 消费方编译检查 (companion/e2e/tool-runtime examples) 退出码 0 |

## 2. 测试证据 (0 装)

- `cargo test -p apeireth-tool-approval -j 4` **全绿**: 106 单测 + 8 集成 (approval_list.rs) + 3 集成 (r133_2_bridge) + 28 集成 (rules.rs) + 1 doctest = **146 passed, 0 failed**
- 新增测试分布: 决策类型 6 / 命令提取 4 / 条目解析 3 / ApprovalListRule 匹配 10 / BlacklistRule 静默覆写 1 / manager 结构化流 10 / 端到端集成 8
- 下游零破坏: `cargo check -p apeireth-companion -p apeireth-integration-e2e -p apeireth-tool-runtime --examples -j 4` 退出码 0

## 3. 提交清单

| 提交 | 内容 |
|---|---|
| 7c29420f | 结构化拒绝类型 (RejectErrorType/Rejection/ApprovalOutcome/CheckDetail) + ApprovalRule trait 扩展 (silent_on_reject/matched_command 默认方法) + CallRecord 审计字段 |
| d0e18dfa | ApprovalListRule 命令级粒度规则 (extract_commands/parse_approval_entry/considerMatch 语义) + BlacklistRule 静默覆写 + 17 单测 |
| 3d3874d0 | check_detailed + wait_for_approval_outcome 结构化流 + handle_with_reason (VCP reason 协议) + 审计台账 + lib 导出/常量 (RULE_COUNT 5→6) |
| 9044935c | tests/approval_list.rs 端到端 8 测 + README P1 说明 |
| 1ecf7b04 | 模块地图登记 (maintenance-guide N19 行); 台账 N19/N20 与 team-work-doc §8.4 ✅ 标记被并行提交收编 (3a08a4eb / 10ed5edc 等), 内容已在 HEAD |

## 4. 0 假装 / 已知边界 (诚实标注)

1. **ApprovalBridge silent 透传仍是已知丢失** — `PolicyVerdict` (tool-runtime, N10 边界不改) 无 silent/matched_command 字段, bridge 侧注释早已载明; 已登记 **backlog N20** 并入 N10 后续. 决策器内的静默语义是完整的, 跨 bridge 传播待 tool-runtime ctx 扩展.
2. **`silent_on_reject`/`matched_command` 是 `check` 之后的二次查询** — 与 VCP 单次返回 matchedRule/matchedCommand 不同, 属接口兼容性折中 (trait 默认方法, 旧规则零改动). 规则状态并发修改的极端竞态已文档化 (check_detailed doc), 清单条目为低频配置项, 实战无碍.
3. **静默拒绝的理由仍承载在 `Rejection.reason`** — 静默约束是"上层不得回传给 AI", 审批层保留供审计; 此为协议约定, 无编译期强制.
4. **`wait_for_approval` 无 handler 时行为等价但错误码更精确** — 旧版走 DefaultDenyHandler 返 false, 新版直接 ChannelUnavailable 拒绝; 布尔结果不变, 已注册于测试 `outcome_no_handler_is_channel_unavailable` 回归.

## 5. 过程实录 (供 Leader 复盘)

- 在途改动曾被 code_reviewer2 暂存 (`git stash` 验收用, 备份 reports/_wip-backup/tool-approval/), 已从备份恢复并续作; 该 stash (现 stash@{1}) 归属 code_reviewer2, **未越权 drop** — 其内容已全部提交入库, 该 stash 已过时, 由 owner 自行处置.
- backlog 编号冲突: 我初编 N16/N17 与并行"孤儿 crate 接线批"撞号, 已重编 **N19/N20**.
- docs 并行混写: maintenance-guide/team-work-doc 中他人未提交行已隔离 (只提交自己的行, 他人行恢复回工作区).
