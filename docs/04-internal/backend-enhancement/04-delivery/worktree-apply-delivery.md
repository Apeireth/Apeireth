# worktree_apply 受控工作树写交付记录（D-P2）

> `Status: ✅ 已交付（2026-10-13，干活能力轴 P2 后端批）`
> 关联：[D-doing-work.md](../02-axes/D-doing-work.md) · 施工纪律 [CONSTRUCTION-DISCIPLINE.md](../CONSTRUCTION-DISCIPLINE.md) · P1 先例 [git-write-tiering-delivery.md](git-write-tiering-delivery.md)

## 一、范围与语义

多文件大改走**「计划→改→验证→落」四拍受控写**，每拍可审、任一失败整批不落地：

| 拍 | 工具/阶段 | 写入面 | 语义 |
|----|-----------|--------|------|
| 计划拍 | `tool.worktree_plan`（只读） | **零落盘** | 任务目标 → 改单（文件列表 + 每文件动作 + 预期）+ plan_id（审计配对）；改单与补丁交叉一致（文件集 + 动作标签逐项相等，漂移拒绝即帧） |
| 改拍 | `tool.worktree_apply` 内部 | 暂存面（工作区零写入） | 暂存面物化（内容字节级）+ **事务补丁原语**在暂存面执行；补丁错误在暂存面即爆 |
| 验证拍 | `tool.worktree_apply` 内部 | 暂存面（工作区零写入） | 纯结构校验恒有（暂存面文件集与改单逐项相符）+ 可挂校验命令（受控通道：结构化 argv、无 shell 解析、工作目录限域暂存面、`ProcessLimits` 超时）；不过 = 整批未落地 |
| 落拍 | `tool.worktree_apply` 内部 | 真实工作区 | 同一补丁过**读前观测门禁**（未读不得覆盖/删除）+ 持久档原子写落盘，**全有或全无**（任一失败整批回滚——`apply_patch` 事务语义复用） |

- **写总闸语义**：与 `tool.apply_patch` 同受 `file_write` 唯一写总闸（注册即闸，组装根同一开关）；
- **git 分工**：两级工具**不含 git 写**——改单只动文件，提交走 `tool.git_write` L1（工具描述与文档同文 `WORKTREE_GIT_BOUNDARY_NOTE`，防模型幻觉 worktree 工具能做 git 操作）。

## 二、四拍时序图

```text
模型/审批链          worktree_plan(只读)     worktree_apply                暂存面            工作区(真面)
   │                      │                      │                          │                  │
   │──goal+files─────────▶│                      │                          │                  │
   │◀──改单(plan_id/文件数/│                      │                          │                  │
   │   动作分布/逐文件预期)│   计划拍(零落盘)      │                          │                  │
   │                      │                      │                          │                  │
   │──改单+patch[+verify]─┼─────────────────────▶│                          │                  │
   │                      │      ①计划拍          │                          │                  │
   │                      │   改单校验(工作区相对/ │                          │                  │
   │                      │   敏感面/唯一性)       │                          │                  │
   │                      │   补丁解析+交叉一致    │                          │                  │
   │                      │   (漂移→拒绝即帧)     │                          │                  │
   │                      │   边界预检(观测门禁:   │                          │                  │
   │                      │   未读不改→拒绝即帧)   │                          │                  │
   │                      │                      │                          │                  │
   │◀━━━━━━━━ 审批卡载荷 = 改单摘要: 文件 N (add a/update u/delete d); 验证命令 M 条 [...] ━━━━━━▶│
   │    (RequireApproval 级, 每次落盘都人批; read_only 预设拒 / standard 审批 / full 按预设)     │
   │                      │                      │                          │                  │
   │                      │      ②改拍            │──物化(内容字节级)────────▶│                  │
   │                      │                      │──事务补丁原语执行────────▶│                  │
   │                      │                      │   (暂存面, 补丁错误即爆)  │                  │
   │                      │                      │                          │                  │
   │                      │      ③验证拍          │──结构校验(文件集逐项相符) │                  │
   │                      │                      │──校验命令(受控通道,       │                  │
   │                      │                      │   cwd=暂存面)[可挂]──────▶│                  │
   │                      │                      │   任一不过=整批未落地      │                  │
   │                      │                      │                          │                  │
   │                      │      ④落拍            │──────────────────────────┼─────────────────▶│
   │                      │                      │  事务补丁原语: 预演+观测   │  持久档原子写      │
   │                      │                      │  门禁CAS+原子写           │  (全有或全无)      │
   │                      │                      │  任一失败→整批回滚────────┼──────────────────│◀─回滚恢复
   │◀──status applied / 错误帧(改拍失败|验证拍失败即帧|落拍失败(整批回滚))──────│                  │
   │                      │                      │──Drop: 暂存面清理(不弃尸)─▶│                  │
```

关键性质：

1. **落盘前工作区零写入**——改拍/验证拍全部发生在暂存面；只有落拍写工作区；
2. **全有或全无**——落拍复用 `TransactionalPatchApplier` 事务语义（预演 + 版本 CAS + 原子写 + 失败回滚），批级不变量：任一拍失败后工作区与调用前逐字节一致；
3. **观测门禁不变**——未读不得覆盖/删除（会话读事件 = 「已读」证据，版本 CAS 钥匙照旧）；
4. **改单是合同**——补丁声明与改单逐项核对（文件集 + 动作标签），补丁动改单之外的文件无处混入。

## 三、治理（两级工具均过五段流水线）

| 面 | `tool.worktree_plan` | `tool.worktree_apply` |
|----|----------------------|------------------------|
| 流水线 | pre(空=零审批) → Guard(改单边界, 单调) → around(5s) → 输出归一合同(六字段) | pre(授权映射) → Guard(边界+观测门禁, 单调) → around(120s) → 输出归一合同(八字段) |
| 审批档 | **只读档零审批**（无 ask 档钩子；计划不落盘） | **RequireApproval 级**——每次落盘都人批，无自动放行档 |
| 审批卡载荷 | 无 | 改单摘要：**文件数 / 动作分布 / 验证命令**（含命令显示串，脱敏后） |
| `read_only` 预设 | 放行（只读能力） | **拒**（`is_write_or_execute_capability` 名单收录） |
| `standard` 预设 | 放行 | 审批（RequireApproval） |
| `full` 预设 | 放行 | 按预设语义（RequireApproval→Allow 改写；本工具无审批地板） |
| 错误面 | 拒绝即帧（工作区外/敏感面/重复声明/空预期） | 拒绝即帧（同左 + 改单漂移 `改单与补丁不一致拒绝即帧` / 验证 `验证拍失败即帧` / 改拍失败 / 落拍失败（整批回滚）） |

## 四、测试证据

主套件 [`crates/capabilities/tools/tests/worktree_apply.rs`](../../../../crates/capabilities/tools/tests/worktree_apply.rs)（15 用例）+ 组装根 [`crates/engine/runtime-assembly/src/canonical/permission_preset.rs`](../../../../crates/engine/runtime-assembly/src/canonical/permission_preset.rs) 测试组：

| # | 用例 | 钉住的行为 |
|---|------|-----------|
| 1 | `plan_produces_the_change_order_with_a_stable_plan_id` | 计划拍产改单（文件/动作/预期/计数/分布）+ plan_id 稳定配对（同单同 id、改单变则变） |
| 2 | `plan_never_touches_the_workspace` | **计划零落盘**（声明建/改/删，工作区逐字节纹丝不动） |
| 3 | `plan_refuses_out_of_workspace_sensitive_and_ill_formed_targets` | **工作区外拒**（绝对路径/`..`）+ **敏感面拒**（.env / id_rsa）+ 重复声明/空预期/空集/空目标拒 |
| 4 | `apply_lands_the_change_order_through_all_four_beats` | **apply 按改单执行**全四拍（报告八字段冻结 + plan_id 与计划拍同文 + 校验命令走受控通道） |
| 5 | `apply_refuses_untouched_targets_until_they_were_read` | **观测门禁（未读不改）**：未读改/删拒绝即帧；会话读过后同单放行 |
| 6 | `apply_refuses_patch_drift_against_the_change_order` | 改单漂移四形态拒绝即帧（补丁多动/改单多列/动作不符/补丁重复目标），零写入 |
| 7 | `any_beat_failure_leaves_the_batch_fully_rolled_back` | **原子回滚（任一失败整批回滚）**：改拍失败与落拍失败都保持工作区逐字节原样 |
| 8 | `landing_mid_commit_failure_rolls_back_already_committed_files` | **落拍中途失败确定性回滚**（Windows：占句柄不给 FILE_SHARE_DELETE → 删除批撞共享冲突 → 已落地的修改被事务回滚恢复） |
| 9 | `verification_failure_aborts_before_landing` | **验证拍失败即回滚**（校验命令非零退出/无法启动都整批未落地） |
| 10 | `approval_payload_carries_the_change_order_summary` | **审批卡载荷**=改单摘要（文件数/动作分布/验证命令/档位声明）；漂移调用拒绝即帧不进审批 |
| 11 | `apply_sits_at_the_require_approval_level_while_plan_needs_none` | **审批级语义**（无授权 `pipeline.pre_ask` 即帧不落盘/授权放行）+ **plan 零审批** |
| 12 | `boundary_guards_refuse_out_of_workspace_sensitive_and_drifted_targets` | 单调边界 Guard 只拒不放（两级同判定，预检与 Guard 不各说各话） |
| 13 | `tool_descriptions_state_the_git_write_division` | **与 git_write 分工文案**（只动文件/不含 git 写/提交走 L1 分级词表，两级描述同文） |
| 14 | `malformed_parameters_are_refused_as_error_frames` | 参数面即帧（未知字段/缺字段/词表外动作/空补丁），错误即帧零谎帧 |
| 15 | `verify_command_display_redacts_secret_surfaces` | 校验命令显示串脱敏（凭据不入审批卡/日志） |
| P1 | `read_only_preset_covers_egress_credential_and_delete_exits` | **read_only 拒 apply**（名单收录）+ `tool.worktree_plan` 留在只读面 |
| P2 | `worktree_apply_approval_tier_matches_each_permission_preset` | **read_only 拒 / standard 审批 / full 按预设** 三档逐档钉死（含审批卡载荷断言） |

既有套件零回归：`cargo test --workspace --locked -j 2` 全绿（exit 0，2026-10-13 实跑）——tools crate 既有套件全数原样通过（`apply_patch_write` 13 / `git_write_grading` 16 / `delete_path_parity` 9 / `exec_pipeline` 12 / `architecture_invariants` 5 等），runtime-assembly 全绿（`permission_preset` 9/9，含恢复挂测的 `read_only_preset_covers_egress_credential_and_delete_exits`）。

## 五、还债与报备

1. **见债必还**（`permission_preset.rs`）：`read_only_preset_covers_egress_credential_and_delete_exits` 的 `#[test]` 属性曾被并入上一行行尾注释而**静默失效**（挂名不跑的陈旧断言，`dead_code = allow` 使其无警告漏网）——本批恢复挂测并钉 D-P2 新面，测试注释注明；
2. **跨范围最小改动报备**：`crates/engine/runtime-assembly/src/canonical/permission_preset.rs`（read_only 名单 +1 项、测试 3 处、上条还债）——不在本批战场声明内，但「read_only 预设拒 apply」验收必须由该名单承载（名单缺项 = 只读会话静默放行的 M2 病灶），改动限于最小集，特此报备。

## 六、接棒清单（未完）

1. **组装根注册**（`crates/engine/runtime-assembly/src/canonical/production.rs`，共享文件，四路并发不抢）：`ProductionModulesConfig` 加 `worktree_plan` / `worktree_apply` opt-in 开关，注册条件 = `config.file_write`（唯一写总闸）+ workspace root + 共享 `ObservedGate`，模块构造已是现成的 `WorktreePlanModule::new(root)` / `WorktreeApplyModule::new(root, gate)`（[tool_modules.rs](../../../../crates/engine/runtime-assembly/src/canonical/tool_modules.rs) 已挂载）；自省通道名册如需列开关生效值同批补。
   → ✅ **已收口（2026-10-13 总装批件一）**：两级开关按上述注册条件入 `production.rs`（注册即闸），自省名册两行 `worktree_plan` / `worktree_apply` 按实际注册条件取值；env 旋钮 `APEIRETH_ENABLE_WORKTREE_PLAN` / `APEIRETH_ENABLE_WORKTREE_APPLY`（子依赖主 + file_write，fail-closed）在 `crates/adapters/cli/src/lib.rs` 接线，授权同源（grant + `WorktreeApplyApprovalHook` 审批级，plan 零审批）；测试 `worktree_registration.rs` 3 + `worktree_knob.rs` 4。
2. 会话预设 UI/多客户端线如需展示审批卡，载荷字段即 `WorktreeApplyApprovalHook` 的 reason（改单摘要）。
