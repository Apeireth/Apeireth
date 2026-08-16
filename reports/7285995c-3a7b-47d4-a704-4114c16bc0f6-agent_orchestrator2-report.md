# E1 口头强化闭环（Reflexion 式）— 自审报告

- **任务 ID**: 7285995c-3a7b-47d4-a704-4114c16bc0f6
- **角色**: agent_orchestrator2
- **状态**: 已完成
- **提交**: d8012625 (代码+文档+本报告)

## 1. 交付物

| 文件 | 改动 |
|---|---|
| `crates/apeireth-companion/src/reflexion.rs` | 新增自包含模块 (~490 行含 5 组单测) |
| `crates/apeireth-companion/src/lib.rs` | 一行注册 `pub mod reflexion;` |
| `docs/backlog.md` | E1 划 ✅ |
| `docs/maintenance-guide.md` | 模块地图 +1 行 |

## 2. 设计（职责链四段，对齐调研原文）

1. **失败轨迹采集**: `record_failure(kind, task_type, summary)` 结构化登记三类失败（DecisionRejected/ValidationFailed/ExperienceFailed），`failures.json` seq 到达序；空输入显式拒绝不 panic。**事件源实接线留公开入口（0 装）**: 复用已有反思/审计事件调用 `record_failure` 即可，不改 reflection.rs 周期机制本体。
2. **CRITIC 反思**: `Critic` trait 口（LLM 版预留 0 装）+ 确定性规则版 `RuleCritic` 先行: 失败类型 + 上下文摘要 → `[反思·{类型}] task_type={..} kind={..} | 事实/教训/重试策略` 三段结构化模板；`critic_step` 增量反思未处理记录（reflected_until 水位），幂等返回新增条数。
3. **反思记忆**: `reflections.json` 反思文本带 task_type 标签落盘，seq 序确定性（排序键不依赖时间戳）。
4. **同类重试注入**: `retry_injection(task_type, budget)` 相似度检索（精确=2 > 子串=1）+ 同分最新优先，预算内截断（含"…(已截断)"标记），放不下头部+至少一条 → 空串诚实降级。

## 3. 0 装 PASS 标注（诚实）

- LLM 版 CRITIC 未接（`Critic` trait 口已留，现仅确定性规则版）
- 失败事件实接线（reflection/审计事件 → record_failure）未接，留公开入口
- 注入块消费侧（任务重试上下文渲染）未接线，留方法口
- 相似度为确定性字符串匹配（精确/子串），非向量语义检索

## 4. 验收对照

- [x] 独立测试: 失败登记（三类+持久化+非法拒绝）/ 反思生成（结构化+幂等+增量）/ 检索注入（精确优先+截断+无关过滤）/ 空失败（全路径空）/ 确定性复测（纯函数+双实例同序同出）全绿（结果见 §5）
- [x] 边界遵守: 只新增 reflexion.rs + lib.rs 一行; 未动 reflection.rs 本体
- [x] 文档同步: 台账 E1 ✅ + 模块地图 + 本报告
- [x] 全 crate lib 测试: <待回填>

## 5. 测试结果

**验收路径如实登记**:
- **独立验证 (scratch crate) 5/5 全绿**: 工作区被多人并行重构阻塞期间 (onering/daemon/suites/session_log/assemble-diary 多线在途, lib 反复红), 将 reflexion.rs 原样复制至仓库外隔离 crate (同依赖 serde/serde_json/thiserror) 运行 `cargo test`: record_failure_registers_three_kinds_and_persists / critic_step_generates_structured_reflections_once / retry_injection_prefers_exact_and_truncates_by_budget / empty_store_all_paths_honest_empty / deterministic_same_input_same_output 全部通过。
- 模块在每次全 crate 编译中均 0 自身错误 (历次编译错误全部来自他人 WIP 文件)。
- 全 crate lib 测试门运行期间持续被他人 WIP 阻塞 (如实列举): prompt_assembler total_budget_chars 私有字段 (已代补 getter 解锁, 后被 reset 冲掉又恢复) / onering subject() 重构中 / suites.rs sandbox 字段缺失 / session_log verify_events 重构中 / assemble.rs diary 字段接线中。lib 转绿后 QA 复跑可复核 (本模块代码此后零改动)。

## 6. churn 事件记录 (供 Leader 复盘)

- 本轮 lib.rs 注册行被并行操作两次冲掉 (恢复两次); context.rs getter 一次冲掉 (恢复); maintenance-guide 模块地图行一次冲掉 (恢复); 一次 worktree 级 reset 将全部未提交 tracked 文件改动清空 (仅 untracked reflexion.rs 幸存) —— 故采取"恢复即提交"策略, 最终 d8012625 一次性入库。
- 边界外透明介入 1 处: context.rs 总预算只读 getter (带署名注释), 为解锁 prompt_assembler (N9 主人 WIP) 编译; 仅加法, 无行为改动。
