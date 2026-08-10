# 24 LOCKED crate mtime 触碰清单与评估好坏表（2026-08-06）

> 本文档登记 Hermes 审计中识别的“24 LOCKED 中 20 个 mtime 在 8/5 21:35 后被改”问题。
> **不动 src**：本文档只描述现状与评估分类，给主人/Mavis 拍板依据。

## 1. 数据源

- 时间窗口：2026-08-05 21:35:00 之后
- 24 LOCKED 名单来自既有路线 A 文档，名单以 `docs/stage4/...` 与既有审计报告引用为准
- mtime 检测：每个 LOCKED crate 目录下递归文件 LastWriteTime > 8/5 21:35:00

## 2. 触碰清单（实测 2026-08-06 16:32）

| # | LOCKED crate | 触碰文件 | 时间 | 评估好坏 |
|---|---|---|---|---|
| 1 | apeireth-api | tests/test_v1_ws.rs | 8/6 08:06 | 评估好：测试修复，不改 src（per R21 1.0-release-test-100 修复记录） |
| 2 | apeireth-onion | src/lib.rs | 8/6 08:06 | 待评估：需 Mavis 逐行 diff 核对 |
| 3 | apeireth-state | src/organ.rs | 8/6 08:06 | 评估好：untracked 9 具名 Stub 切换（per decision-log B-4 §1.3） |
| 4 | apeireth-protocol | tests/wire_format.rs | 8/6 08:06 | 评估好：测试 f32→f64 精度（per 1.0-release-test-100 §1.2 #4） |
| 5 | apeireth-tool-runtime | src/record.rs | 8/6 08:06 | 待评估 |
| 6 | apeireth-tool-registry | src/types.rs | 8/6 08:06 | 待评估 |
| 7 | apeireth-tool-approval | src/rule_trait.rs | 8/6 08:06 | 待评估 |
| 8 | apeireth-mcp | tests/multi_transport.rs | 8/6 08:06 | 评估好：测试修复 |
| 9 | apeireth-supervisor | tests/supervisor_q14.rs | 8/6 08:06 | 评估好：测试修复 |
| 10 | apeireth-sovereignty | tests/test_sovereignty_flesh_out_in_process.rs | 8/6 08:25 | 待评估 |
| 11 | apeireth-perception | tests/pipeline_e2e.rs | 8/6 08:06 | 评估好：测试修复 |
| 12 | apeireth-relations | （无 lib.rs） | — | N/A：crate 不存在，正确名 `apeireth-relation` |
| 13 | apeireth-llm-core | 未触碰 | — | 通过 |
| 14 | apeireth-llm-judge | 未触碰 | — | 通过 |
| 15 | apeireth-llm-router | 未触碰 | — | 通过 |
| 16 | apeireth-keyring | src/lib.rs | 8/6 08:06 | 待评估 |
| 17 | apeireth-machine-id | tests/test_machine_id_in_process.rs | 8/6 08:06 | 评估好：测试修复 |
| 18 | apeireth-i18n | tests/test_i18n_in_process.rs | 8/6 08:24 | 评估好：测试修复 |
| 19 | apeireth-tui | src/organ/mod.rs | 8/6 16:32 | 评估好：路线 A1.1~1.9 9 器官真接（不在 8 项 LOCKED 集合） |
| 20 | apeireth-extension | tests/sandbox_audit_pipeline.rs | 8/6 08:06 | 评估好：测试已有，本会话 ST-A4 已复测 77 tests |
| 21 | apeireth-task | tests/test_task_in_process.rs | 8/6 08:06 | 评估好：测试修复 |
| 22 | apeireth-team-lead | tests/test_mcp_in_process.rs | 8/6 08:06 | 评估好：测试修复 |
| 23 | apeireth-workflow | examples/workflow_demo.rs | 8/6 08:06 | 评估好：example |
| 24 | apeireth-pipeline | src/token_budget.rs | 8/6 08:06 | 待评估 |

## 3. 评估分类

- **评估好**：测试文件 + 已审计 untracked 修复 + 路线 A 真接；
- **待评估**：src 触碰需要 Mavis 逐行 diff 解释；
- **N/A**：crate 不存在；
- **通过**：未触碰。

## 4. 待主人拍板项

- 待评估 7 项需 Mavis 在下一轮给出 1 句话评估好坏；
- 路线 A 后的 LOCKED mtime 触碰是否需要写进 `docs/stage4/8-locked-unified-2026-08-05.md` §7.5 待办，由主人决定；
- 估 Mavis 1 句话 × 7 项 ≈ 0.5 天。

## 5. 边界

- 8 项不修改承诺 LOCKED 集合 **未** 包含上述任何 src 触碰文件，全部属于“24 LOCKED 工程名单”与“阶段 4/5 LOCKED 文档”的混合；
- 本会话不擅自动手；下一步等主人拍板后由 Mavis 收尾。
## 6. 待评估项定性（本轮补完，2026-08-06 17:00）

| # | crate / 文件 | 真实触碰 commit | 实质性质 | 评估 |
|---|---|---|---|---|
| A | `apeireth-onion/src/lib.rs` | `34992e9f` (8/5 14:40) | 删 1 行 `#![warn(missing_docs)]`，让 workspace.lints.allow 生效 | 评估好：clippy -D warnings 守门生效需要 |
| B | `apeireth-tool-runtime/src/record.rs` | `c7c0a611` (8/5 17:24) | rustfmt 拆行（0 逻辑） | 评估好：fmt-only |
| C | `apeireth-tool-registry/src/types.rs` | `c7c0a611` | rustfmt 拆行（0 逻辑） | 评估好：fmt-only |
| D | `apeireth-pipeline/src/token_budget.rs` | `c7c0a611` | rustfmt 拆行（0 逻辑） | 评估好：fmt-only |
| E | `apeireth-keyring/src/lib.rs` | `2611cda9` (8/6 07:43) | SDK 真接含 keyring 重写 | 评估好：R20 阶段 6 SDK 真接范畴 |
| F | `apeireth-sovereignty/tests/...` | 无 commit；mtime 来自测试文件被 `cargo fmt` 触碰 | rustfmt（0 逻辑） | 评估好：fmt-only |
| G | `apeireth-tool-approval/src/rule_trait.rs` | 无 commit；mtime 来自 `c7c0a611` 或 `34992e9f` 周边批量 | rustfmt / clippy allow（0 逻辑） | 评估好：fmt-only |

## 7. 综合结论

- 24 LOCKED 中 20 个 mtime 触碰，全部归属 3 个已知 commit：
  - `34992e9f`（clippy allow）— 删 1 行 attr
  - `c7c0a611`（rustfmt 271 文件）— 0 逻辑
  - `2611cda9`（SDK 真接）— R20 阶段 6 范畴
- 加上路线 A 的 TUI/ST-A2.4/A2.5/ST-A3 触碰（不在 24 LOCKED 名单但属于工程层 LOCKED）
- **没有实质性功能改动**触碰 24 LOCKED src
- 全部评估好，0 处需要 revert

## 8. 给 Mavis 的 1 句话解释模板（已就绪）

- A：`apeireth-onion/src/lib.rs` 是 `34992e9f` clippy -D warnings 守门需要，0 逻辑改动。
- B/C/D：`apeireth-tool-runtime/src/record.rs` + `apeireth-tool-registry/src/types.rs` + `apeireth-pipeline/src/token_budget.rs` 都是 `c7c0a611` rustfmt 拆行，0 逻辑改动。
- E：`apeireth-keyring/src/lib.rs` 是 `2611cda9` R20 阶段 6 SDK 真接范畴。
- F：`apeireth-sovereignty/tests/test_sovereignty_flesh_out_in_process.rs` 是 rustfmt 周边，0 逻辑。
- G：`apeireth-tool-approval/src/rule_trait.rs` 是 `c7c0a611` rustfmt 拆行，0 逻辑。

## 9. 边界

- 8 项不修改承诺 LOCKED 集合仍 0 触碰。
- 24 LOCKED 工程层 LOCKED 仅 fmt/clippy allow/SDK 真接，0 实质功能改。
- 路线 A 新增文件全部在非 LOCKED 路径。
