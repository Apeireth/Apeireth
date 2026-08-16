# 自审报告 — N9 提示词装配引擎 (占位符变量宇宙)

- 任务 ID: a83d50af-175d-4ecf-a786-221e30d4cfea
- 角色: backend_engineer
- 日期: 2026-08-16

## 1. 改动文件

| 文件 | 类型 | 说明 |
|---|---|---|
| crates/apeireth-companion/src/prompt_assembler.rs | 新增 | 核心机制件 (~600 行含 13 单测) |
| crates/apeireth-companion/src/context.rs | 共享(最小) | 仅新增 `total_budget_chars()` getter (assemble 重预算用), 0 行为变更 |
| crates/apeireth-companion/src/lib.rs | 共享(最小) | `pub mod prompt_assembler` + 一行 re-export |
| crates/apeireth-companion/examples/virtual_time_simulation.rs | 共享(最小) | 新增第 7 段验收 (机制件 checklist 第 5 条) |
| docs/maintenance-guide.md | 文档 | 模块地图登记 prompt_assembler.rs |
| docs/backlog.md | 文档 | N9 划 ✅ + 完成方式/证据 |

## 2. 调研依据 (吸收写法, 不抄代码)

research/source/vcptoolbox/modules/messageProcessor.js (只读):
- resolveAllVariables: 特权角色判定 + processingStack 环检测 + expandedAgentName/expandedToolboxes 守卫
- AgentGuard: 全上下文单 agent (灵魂级安全); ToolboxGuard: 每种 toolbox 一次 (replaceFirstAliasPlaceholder 首现展开)
- replaceOtherVariables/replacePriorityVariables: 分型变量源 (时间/env/插件/文件递归)

## 3. 机制设计 (5 要素 → 实现)

1. 分型变量源 → `VariableSource` trait (kind + resolve) + StaticSource/TimeSource 内置; 注册序 = 无前缀解析优先级; `{{kind:name}}` 前缀定向寻址
2. 特权角色 → AssemblyRole + system_markers (默认 `[系统提示:]`/`[系统邀请指令:]`, 可配置); 非特权时 agent/toolbox 占位符静默移除 + 报告
3. AgentGuard → AssemblyGuard.expanded_agent: 首个 agent 展开 (含其嵌套), 后续任何 agent 移除
4. ToolboxGuard → AssemblyGuard.expanded_toolboxes: 每种首个占位符展开, 其余移除
5. 环检测 → 递归展开栈 + 深度上限 (默认 8); 环 → `[循环变量引用: a -> b -> a]` 诚实标记 + 报告

集成 (而非分立): `assemble(&ContextAssembler)` = 既有预算截断 → 逐块展开 → 复用 ContextAssembler 预算语义 (核心块保护+单块 cap+贪心砍大头) 重截断。

## 4. 测试结果

**prompt_assembler 模块: 17/17 全绿** (`cargo test -p apeireth-companion --lib prompt_assembler -j 4`)

| # | 测试 | 路径类型 |
|---|---|---|
| 1 | typed_sources_expand | 正常: 4 分型源展开 |
| 2 | prefixed_addressing_targets_kind | 正常: `{{kind:name}}` 定向寻址 |
| 3 | nested_value_recursion | 正常: 值内占位符递归展开 |
| 4 | agent_only_expands_in_privileged_role | 特权: system 展开 / user 静默移除不外泄 |
| 5 | user_with_system_marker_is_privileged | 特权: `[系统提示:]` 标记 user 视为特权 |
| 6 | agent_guard_single_agent_per_context | AgentGuard: 首个展开, 第二个移除, 跨文本复现移除 |
| 7 | toolbox_guard_once_per_name_first_occurrence | ToolboxGuard: 首现展开, 同文本重复+跨文本均移除留痕 |
| 8 | toolbox_non_privileged_removed | 特权: assistant 角色移除 |
| 9 | undefined_variable_preserved_and_reported | 失败: 未定义变量原样保留+报告 (含未知前缀) |
| 10 | circular_dependency_detected | 失败: a↔b 环 → `[循环变量引用: custom:va -> custom:vb -> custom:va]` |
| 11 | self_circular_detected | 失败: 自环检测 |
| 12 | depth_cap_guards_explosion | 失败: 12 级链深度上限 4 → 超限原样保留+报告 |
| 13 | invalid_registration_rejected | 非法输入: 空名/空格名/冒号名/重复名/零深度全拒 |
| 14 | malformed_placeholders_untouched | 非法输入: 5 种畸形占位符原样保留零报告 |
| 15 | assemble_rebudgets_after_expansion_with_core_protection | 集成: 展开膨胀后复用 ContextAssembler 预算重截断, 核心块保护 |
| 16 | expand_blocks_keeps_metadata | 集成: 块元数据 (core/cap/name) 保留 |
| 17 | time_source_with_virtual_clock_fastforward | 时间源: VirtualClock 快进 1 天 → date/today 跟随 |

**验证环境说明 (0 装 PASS)**: 主树工作区被其他任务包 WIP 反复阻塞 (tool-approval E0521 → 已修; job_object.rs E0277; tone.rs 未入库等), 验收测试在**隔离 worktree (HEAD a2a60564 + 补齐未跟踪模块文件 + 借用 WIP tone.rs 只读)** 执行。另: `cargo test -p apeireth-companion` 全目标运行时, companion_serve.exe 被他人运行中占用导致 LNK1104 链接失败 (环境问题, 非代码问题), 故以 `--lib` 全量 + 示例单独编译运行为验收证据。

**调试实录 (诚实审计)**: 首轮 15/17, 修复 2 处——① depth_cap 测试的 format! 字符串转义 bug (`{{` 被转义为单花括号, 测试自身问题, 引擎无错); ② toolbox 同文本重复移除未记入报告 → replace_forms 返回移除计数留痕。另自修 Datelike 导入缺失。

### 4.1 全量 lib 套件补充验证 (最终证据)

隔离 target 独立构建后执行 `cargo test -p apeireth-companion --lib -j 4 -- --skip continuity::tests`:

**337 passed / 2 failed** — 其中 prompt_assembler 17/17 全绿含于 337。两处失败**均属他人任务包 WIP**, 与本任务零关联:
- `thought_cluster::tests::invalid_inputs_rejected` / `read_cluster_sorted_empty_and_missing`: thought_cluster (未跟踪 WIP 模块) 内部不一致 — 其测试期望簇名 `簇X` 合法, 但其自身校验报 `InvalidName("簇X")`
- `continuity::tests::migrate_*` ×3: 挂起 >60s (疑似等真实 DB/死锁), 故 `--skip` 过滤 (已实测确认是挂起非慢)

0 装 PASS: 全量套件在**当前多成员 WIP 交织的工作区**无法绝对全绿 — 上述 2 失败 + 3 挂起全部归属 thought_cluster/continuity 两个未入库模块; 我的任务包 (prompt_assembler + context.rs getter + 示例第 7 段) 零失败。

## 5. 0 装 PASS 标注 (没做什么)

- ❌ 未接线 companion_serve / assemble.rs 实际链路 — 本模块为独立机制件; 接线属后续任务 (需与注入链顺序协商)
- ❌ 无动态折叠 (VCP DynamicFold 依赖 embedding 相似度 — 属 N11 context-fold 域)
- ❌ 无文件递归变量源 (.txt 引用) / 无异步结果占位符 — 留 VariableSource trait 扩展口
- ❌ toolbox 为静态文本块 (VCP 是 fold_blocks + 相似度阈值分档)
- ❌ 引擎同步 (变量源为内存数据); IO 型来源需异步预取后注入
- 与 VCP 的两处有意偏差: ① 嵌套 agent 展开后 expanded_agent 只在未设置时才设置 (VCP 会覆盖, 语义更贴合"单 agent"); ② 未知变量原样保留 + 报告 (VCP 部分替换为空串), 0 虚构内容

## 6. 给集成守门员的合并提示

- 共享文件改动仅 3 处 (context.rs getter / lib.rs 两行 / 示例一段), 均纯新增, 无冲突风险
- 新模块无新依赖 (chrono/thiserror/apeireth-core 均已在 companion Cargo.toml)
- 后续接线建议: companion_serve 注入链在 ContextAssembler::assemble_budgeted_blocks 之后调 PromptAssembler::assemble, 或直接用 assemble() 一步完成
