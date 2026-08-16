# A1 能力演化回路补全（部署→监控→回滚）— architect2 自审报告

- 任务 ID: 302d53b6-5b69-4bce-95b1-50c51fa2d9f3
- 角色: 架构师2 | 日期: 2026-08-16
- 提交: `f8245f2`（代码）/ `3a5730c`（文档）

## 1. 背景与哲学锚点
- 主人原话锚点：「我希望它能自己演化」。完整演化回路 = 提案→生成→验证→部署→监控→回滚。
- 开工前 `capability.rs` 只到「激活」；`evolution_gate.rs` 只到「验证闸门判定」。本任务补后三段（A 级设计欠账 A1）。

## 2. 改动文件（严格任务包边界）
| 文件 | 改动 |
|---|---|
| `crates/apeireth-companion/src/deploy.rs` | **新增**：DeployChannel trait + MockDeployChannel + DeployStatus/Deployment/MonitorMetrics + DeployManager（部署执行/监控登记/阈值触发回滚/预测线期限检查）+ 11 个单测（含全链集成测试） |
| `crates/apeireth-companion/src/capability.rs` | 状态机加 `RolledBack`（active→rolled_back 合法迁移，其余非法拒绝）+ `rollback(id, reason)` + `rollback_reason` 字段（`#[serde(default)]` 兼容旧持久化）+ 回滚收据含 rolled_back + 新测试 |
| `crates/apeireth-companion/src/evolution_gate.rs` | `LoopAction`（Promoted→Deploy / Rejected→Rollback / fail-open→HoldUnverified）+ `loop_action()` + `deploy_receipt()` + 新测试 |
| `crates/apeireth-companion/src/lib.rs` | 模块注册 `pub mod deploy;` + re-export（新模块 checklist #2 规定动作） |
| `docs/maintenance-guide.md` | 模块地图：deploy.rs 新行 + capability/evolution_gate 行更新 |
| `docs/release-plan.md` | §五蓝图改写（分层落地说明）+ 进度对账行（🟢 机制回路六段闭环）+ 设计偏差表更新 |

禁止触碰项核实：tool-runtime/tool-approval/context-fold/gateway/memory-graph 零改动（git diff 可查）。

## 3. 架构决策记录
1. **部署通道 = trait 抽象**：`DeployChannel::deploy(name, artifact)`；测试走 MockDeployChannel（可配失败/计数）；真执行体接线点 = 实现该 trait 挂 exec_worker/sandbox（机制件纪律：mock 先行可测）。
2. **监控双登记**：观测同时写 ① deploy 侧 Deployment.metrics（calls/failures/negative_signals，append-only rev 版本化，对齐 capability.rs 持久化写法）② capability 侧 record_use（EMA/Beta-Binomial，复用已有机制，不另立）。
3. **回滚触发三路**：差评信号越限（默认 ≥2，不看样本量）/ 失败率越限（默认 ≥50% 且 obs ≥4，防小样本误杀）/ 预测线期限（ExpectedOutcome.deadline 已过且零观测 → 按预期行 rollback 动作回退）。触发即调 capability 状态机 `active→rolled_back`，部署记录同步留痕，产出 `[deploy-revert]` 收据（revert 即学习信号，进 revert_receipts 供下一轮提案参考）。
4. **时间机制**：DeployManager 持 `Arc<dyn Clock>`（默认 SystemClock，测试注入 VirtualClock 快进，0 真等待）——对齐 core clock.rs 规范。
5. **状态机五段证据**：`full_evolution_loop_propose_verify_deploy_monitor_rollback` 单测一条链走完：提案→验证闸门（EvalGate 首轮通过→Promoted→LoopAction::Deploy）→批准激活→部署（Live）→监控（差评×2）→自动回滚（rolled_back + 收据进学习信号）；并对照验证失败路径（Rejected→LoopAction::Rollback 不部署）。

## 4. 测试结果（2026-08-16 实测回填）
- **本任务包模块全绿**：`cargo test -p apeireth-companion -j 4 --lib -- deploy capability evolution_gate` → **test result: ok. 25 passed; 0 failed**（deploy 11 + capability 新增 1 + 原有 + evolution_gate 新增 1）
- **crate 全量 lib 套件**（跳过他人活跃 WIP）：`-- --skip job_object --skip continuity` → **335 passed; 2 failed** — 2 个失败均为他人未提交新模块 `thought_cluster.rs`（N2 WIP，untracked 文件），与本任务包零关联（git status 可证：job_object.rs=M/continuity.rs=??/thought_cluster.rs=?? 均属他人）
- 状态机五段证据：`full_evolution_loop_propose_verify_deploy_monitor_rollback` 全链测试通过（提案→验证闸门 Promoted→LoopAction::Deploy→批准激活→部署 Live→差评监控×2→自动回滚 rolled_back+收据进 revert_receipts；对照路径 Rejected→LoopAction::Rollback 不部署）
- VirtualClock 时间机制：`deadline_triggers_rollback_with_virtual_clock` 快进 2h 触发期限回滚，0 真等待 ✅

新增测试清单（全部含正常/失败/非法状态迁移路径）：
- capability.rs：`rollback_flow_and_illegal_transitions`（pending/approved→rolled_back 非法、active→rolled_back 合法留痕、终态不可再迁移、收据留痕、激活清单为空）
- deploy.rs：`deploy_active_capability_live` / `deploy_requires_active_state`（pending/approved/404 全拒）/ `channel_failure_leaves_failed_record_and_retries`（失败留痕+能力保持 active+重试成功）/ `observe_accumulates_metrics`（含 capability 台账同步断言）/ `negative_signals_trigger_rollback`（单次不触发、二次触发、回滚后拒观测）/ `failure_rate_trigger_rollback`（min_observations 防误杀）/ `observe_requires_live_deployment`（未部署/已退役拒观测）/ `deadline_triggers_rollback_with_virtual_clock`（快进 2h 触发）/ `deadline_skipped_when_observations_exist` / `deployment_persistence_survives_reopen`（重放取最大 rev）/ `full_evolution_loop_*`（全链）
- evolution_gate.rs：`loop_action_maps_gate_decision_to_deploy_or_rollback`

## 5. 0 装 PASS 标注（诚实）
| 做了 | 没做（接线点/后续） |
|---|---|
| 部署执行机制（通道抽象+留痕+重试） | **真执行体**未接：实现 DeployChannel 挂 exec_worker/sandbox 是接线点，当前仅 MockDeployChannel |
| 监控登记（调用计数/失败率/差评信号+期限） | 观测数据来自调用方显式 feed（未从真实工具链路自动采集 — 接线点 = ToolBridge 钩子，属他人任务包未动） |
| 回滚状态机（active→rolled_back 留痕+收据） | 回滚的物理清理（如已注册工具卸载）随真执行体接入一起做 |
| 验证闸门→回路动作挂接（LoopAction） | 「生成」段（LLM 生成能力内容）未机制化，不在本任务范围 |
| 制品 = 文本描述 | 真制品形态（代码/工具注册项）待真执行体接入 |

## 6. 补记：编译阻塞与解决（并行作业实录）
本任务执行期间工作区处于多成员高频并行状态（峰值 27 个文件 M/?? 状态、10+ cargo 进程争 build 锁）。阻塞链与处置：
1. `apeireth-tool-approval` E0521/E0308（他人 WIP）→ 通报 leader 并转交修复提示（`entry: &'e ParsedApprovalEntry` 生命周期标注）→ 负责人自行修复 ✅
2. `job_object.rs` B3 WIP（E0277 + 引用未就绪 crate::sandbox）→ 通报 leader，不代改，等待其收敛 ✅
3. `prompt_assembler.rs` E0382（他人 WIP）→ 同上，等待自修复 ✅
4. build 目录锁/package cache 锁竞争 → 间歇重试（非轮询轰炸），全程未动他人文件一行
- 纪律核实：本任务包仅触碰 deploy.rs(新)/capability.rs/evolution_gate.rs/lib.rs(注册行)+2 份文档；deploy.rs 曾被外部补入 `EpisodeStore` trait 导入（cargo fix 或同事所为，属必需修复，已确认并以独立提交 a2a6056 收编）。

## 7. 给守门员的合并提示
- 本任务包与 architect 的账本接线不重叠（本包只动 deploy 新模块 + capability/evolution_gate/lib.rs 注册行）。
- capability.rs 序列化新增字段带 `#[serde(default)]`，旧库重放兼容（persistence_survives_reopen 覆盖）。
- DeployManager 阈值是 pub 字段，调用方可按场景调（默认：差评≥2 / 失败率≥50% 且 obs≥4）。
- 后续接线候选（不在本包）：① ToolBridge 工具执行钩子 → observe() 自动采集；② exec_worker 实现 DeployChannel；③ virtual_time_simulation 例加部署段（机制件 checklist #5，因边界纪律未动 examples，建议由持有该文件任务包的人补一小段）。
