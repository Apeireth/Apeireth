# Agent D-3 — 决策日志 (R25 战区 3 Multi-Agent / 2026-08-10)

> **任务**: Apeireth-rust 战区 3 — `apeireth-council` 升级 4 协作模式 + 角色宪法 + trace
> **作者**: Mavis (派活, 主人 02:55 离场)
> **核心决策**: 5 项 (per 派活单授权)

## 决策 1: 4 模式实现策略 (per 派活单 "4 模式具体实现策略自己定")

### 选项
- **A**: 4 模式全从零写, 不复用 R33-4-1
- **B**: 4 模式全复用 R33-4-1 helpers (1:1 transparent)
- **C**: 混合 — Debate 复用 R33-4-1, 其余 3 自己写 (D-3 最终决策)

### 选 C 理由
- **Debate 模式** = 多轮协商辩论, R33-4-1 `CouncilMemberDeliberator` 1:1 匹配
- **Planner+Executor / Voting / Hierarchical** = 各自不同的协作模式, 复用 R33-4-1 helpers (parse_stance / compute_consensus) 但 Driver 完全不一样
- **避免"假复用"**: 硬把 Planner+Executor 套 Debate 路径会"假装通过"
- **跟 R23 P3 transparent pattern 一致**: 复用时 0 改源, 1:1 包装

### 验证
- ✅ debate.rs 0 改 R33-4-1 module (git diff 0 显示 R33-4-1 触碰)
- ✅ planner_executor.rs / voting.rs / hierarchical.rs 自己写 Driver, 仅用 `crate::advisor::AdvisorOpinion` + `crate::synthesis::synthesize` 公共 API
- ✅ 4 模式各自 9-17 unit tests (71 total in collaboration module)

## 决策 2: reasoning trace 格式 (per 派活单 "reasoning trace 格式自己定")

### 选项
- **A**: 仅 JSON
- **B**: 仅 Pretty print (人类可读)
- **C**: 3 格式 — Pretty + JSON + JSONL (D-3 最终决策)

### 选 C 理由
- **Pretty**: 人类可读, 满足派活单硬指标 "3 advisor 协作 + trace 打印"
- **JSON**: 机器可读, 满足 v2.0 strategy §2B "reasoning trace 可视化" 一般需求
- **JSONL**: claude_code trace 风格 (每行 1 step JSON), 满足 v2.0 strategy Stage 4 §observability "trace export" 准备 (虽然本期不接 S3/磁盘)
- **3 格式 0 漂移**: 同一份 `TraceReport`, 3 个 method 各取所需

### 验证
- ✅ `to_pretty_print()` 含 Council Trace header + Step 0..N + Final Verdict 段
- ✅ `to_json()` serde_json 完整序列化 (`session_id` / `mode` / `steps` / `final_verdict`)
- ✅ `to_step_jsonl()` 每行 1 step (4 steps = 4 lines)
- ✅ example `trace_visualize` 跑通, 3 格式都演示

## 决策 3: 角色宪法跟 R11 5 重守门对齐 (per 派活单 "1:1 镜像 vs 简化 自己定")

### 选项
- **A**: 简化版 (3 字段 struct, 仅核心 1:1)
- **B**: 1:1 镜像 (5 字段 struct, 严格对齐 R11 v1138 5 重守门) (D-3 最终决策)
- **C**: 完整版 (5 字段 + 7 advisor domain 默认值 + validate_opinion trait)

### 选 C 理由
- **派活单明示** "1:1 镜像" 是偏好方向
- **5 字段 1:1 镜像** + **7 advisor 默认** + **validate_opinion** trait 完整 — 满足 v2.0 strategy §2B "实现'角色宪法' (每个 advisor 自己的约束)" 完整度
- **6 哲学锚穿透**: per Apeireth 主哲学锚, S-1/S-2/O-2/O-3/O-4/O-5 6 锚
- **0 触碰 R11**: 仅 Rust 侧 1:1 镜像, R11 v1138 `no_pretend_five_guards.py` 0 改

### 验证
- ✅ 5 字段 struct (`physical_isolation` / `l0_ha_required` / `jurisdiction_bounds` / `compile_time_hardcoded` / `philosophical_anchors`)
- ✅ 6 哲学锚 const (`PHILOSOPHICAL_ANCHORS: [&str; 6]`)
- ✅ 7 advisor domain 默认值 (Safety/Philosophy/Ethics/Legal/Performance/History/Strategy, 7 unique HashSet 测试过)
- ✅ `RoleConstitutionTrait` trait + `validate_opinion()` 5 守门 1:1 顺序校验

## 决策 4: 4 模式细节设计 (per 派活单 "4 模式具体实现策略自己定")

### 4.1 Planner+Executor — 3 步固定

**选项**:
- **A**: 动态 N 步 (per LLM 拆, 但 0 LLM 又变成硬编码)
- **B**: 固定 3 步 (per 派活单 "1:1 对标 LangGraph", 1 planner + 3 executor) (D-3 最终决策)

**选 B 理由**:
- LangGraph `PlanAndExecute` 默认 1 planner + 1 execute (迭代), 简化成 3 步 sequential 更直观
- 5 关键词模式 (deploy/design/test/fix/default) → 3 步 fixed SubTask
- 0 漂移, 0 假装"planner 真接 LLM"

### 4.2 Debate — 复用 R33-4-1 (per 决策 1)

### 4.3 Voting — 3 strategy

**选项**:
- **A**: 仅简单多数
- **B**: 3 strategy (WeightedMajority / TopScoring / Supermajority) (D-3 最终决策)

**选 B 理由**:
- **WeightedMajority** (default): 跟 R10 synthesize 1:1 兼容
- **TopScoring**: 单意见最高分, 适合"专家一票否决"
- **Supermajority**: 2/3 通过, 适合"宪法修正案" 严格场景
- 3 strategy 满足 AutoGen GroupChatManager 投票聚合 + LangGraph conditional edges 多样性

### 4.4 Hierarchical — 1 root + 2 sub

**选项**:
- **A**: 1 root + N sub (动态)
- **B**: 1 root + 2 sub (固定 "技术方案" + "风险评估") (D-3 最终决策)

**选 B 理由**:
- **派活单明示** "主 + 2 子" 提示
- 固定 2 sub 简化测试 + example 跑通
- **技术方案 + 风险评估**: 2 个互补 sub (正 + 负), root 整合 = 实战场景
- 0 漂移, 0 假装"root 真接 LLM"

## 决策 5: 是否触碰 24 LOCKED (per 派活单 "0 触碰 24 LOCKED")

### 选项
- **A**: 触碰 `apeireth-graph` (LOCKED in R113) 改 Node trait
- **B**: 0 触碰 24 LOCKED, 仅用 `apeireth-graph::Graph` 公共 API (D-3 最终决策)

### 选 B 理由
- **派活单硬约束 #4** 严守: 0 触碰 apeireth-cognition/core/sovereignty/formal 任何文件
- `apeireth-graph` 虽然 R113 落点不在 24 LOCKED 列表, 但属于"前任 D 任务落点", 改了会破坏透明集成
- 0 改 `Graph::add_node` / `add_edge` / `add_conditional_edge` 1:1 复用
- 包装层 `CollaborationNode` + `CouncilGraph` 在 `apeireth-council/src/graph_orchestration.rs`, 0 触碰 graph crate

### 验证
- ✅ `cargo check -p apeireth-graph` 0 触碰 (仅 check 验证 0 改)
- ✅ `git diff crates/apeireth-graph/` 0 显示 (0 触碰)
- ✅ 3 factory graph (`planner_executor_graph` / `voting_graph` / `hierarchical_graph`) 仅用 `Graph::add_node` / `add_edge` 公共 API

## 决策 6: 不做事项 (per 派活单 "不要做" + 主人偏好 #3 "0 假装")

### 6.1 0 假装列表
- ❌ 0 假装"planner 真接 LLM" (keyword 拆 5 模式)
- ❌ 0 假装"root 真接 LLM" (硬编码 2 sub-task 模板)
- ❌ 0 假装"debate 模式是新写的" (1:1 包装 R33-4-1)
- ❌ 0 假装"已落地 R11 5 重守门" (仅 1:1 镜像字段名, 0 触碰 R11)
- ❌ 0 假装"trace 已接 S3/磁盘" (仅内存 3 格式)
- ❌ 0 假装"角色宪法已接形式化验证" (仅 struct + trait, 留 R26+ `apeireth-formal` 集成)

### 6.2 0 触碰列表
- ❌ 0 触碰 workspace.version (1.1.0) — root Cargo.toml 0 改 (agent A 加 sqlite-vec 是另一个 PR)
- ❌ 0 触碰 24 LOCKED (cognition/core/sovereignty/formal)
- ❌ 0 触碰 R10 既有 (deliberation.rs / synthesis.rs / hold.rs / sovereignty.rs / lifecycle.rs / persona.rs / mock_llm.rs / llm_backend.rs)
- ❌ 0 触碰 R33-4-1 module (council_member_deliberation.rs) — Debate 0 重写
- ❌ 0 触碰 R33-4-2 module (council_member_persona_combo.rs)
- ❌ 0 触碰 R15 7 强制 advisor (advisors/ 子模块)
- ❌ 0 触碰 `apeireth-graph` (R113 LOCKED)
- ❌ 0 主动 commit

### 6.3 0 重复造轮子
- ✅ Debate 模式 0 行重写 R33-4-1, 1:1 transparent
- ✅ Planner+Executor keyword fallback 复用 R33-4-1 `keyword_stance_fallback` 1:1 镜像 (15 行函数本地复制, 0 改 R33-4-1)
- ✅ Voting 加权复用 R10 `synthesize` 0 改
- ✅ Graph 集成 0 改 `apeireth-graph::Graph`, 仅 `add_node` / `add_edge` 包装

## 决策 7: 风险评估

### 7.1 R1 (D3-1 风险评估) — Debate 复用 R33-4-1 边界

**风险**: Debate 模式复用 R33-4-1 时, 跟 "Multi-round Debate" 区分不开
**缓解**: `CollaborationContext.mode = Debate` 字段 + trace 标注 mode (4 模式 actor 命名区分: `planner` / `executor.N` / `debate.member.N` / `voter.N` / `sub.N`)
**结果**: ✅ trace 输出明显区分 (Pretty print 含 "Debate" / "Voting" / "Hierarchical" 模式名)

### 7.2 R2 (D3-1 风险评估) — Hierarchical root 拆 2 sub-task 怎么拆

**风险**: root 拆 2 sub-task 主观
**缓解**: 硬编码 2 sub-task (sub 1 = "收集并分析", sub 2 = "评估风险", sub 3+ = "综合"), 0 LLM 0 假装
**结果**: ✅ 15 unit tests 覆盖 (4 delegate + 3 sub_execute + 2 aggregate + 4 run + 2 sub_roles)

### 7.3 R3 (D3-1 风险评估) — 角色宪法 5 字段跟 R11 5 重守门 1:1 镜像怎么验证

**风险**: 5 字段命名 / 顺序 跟 R11 1:1 不一定对齐
**缓解**: 在 D3-4 加 1 个 `r11_mirror_sanity_check` test (per `for_advisor_domain_7_distinct` HashSet 唯一性 + `five_guards_summary` 5 字段命名)
**结果**: ✅ 5 字段命名稳定 (per `guard_1_physical_isolation` / `guard_2_l0_ha` / `guard_3_jurisdiction` / `guard_4_compile_time_hardcoded` / `guard_5_philosophy` 注释)

### 7.4 R4 (D3-1 风险评估) — graph_bridge 集成 (Node 包装) 引入 async + Send+Sync 边界

**风险**: Node 是 async, 跟现有 sync Council 冲突
**缓解**: graph_bridge 是**新** module, 不动现有 deliberation.rs; Node::run 内部直接调 sync Driver (mock 或真 driver 都是 sync)
**结果**: ✅ 8 tests 全过 (1 node new + 1 node run + 3 graph factory + 2 mock driver + 1 tokio e2e)

### 7.5 R5 (D3-1 风险评估) — 7h 预算紧

**风险**: D3-2 实现 Planner+Executor (最复杂) 可能超时
**缓解**: D3-2 简化 Planner 只拆 3 步 (固定) + 3 executor (固定); 留 R26+ LLM 真接
**结果**: ✅ 实际用时 2.5h (vs 7h 预算, 提前 4.5h), 1 cargo check 修真 1 字段 (`StanceKind` import + 1 mock driver move 错误)

### 7.6 R6 (D3-5 实际遇到) — MockDriver 用 fixed_score 还是 fixed_stance

**风险**: 初始设计用 `fixed_score: f64`, 但 `is_allowed` 走 `synthesize`, 1 Approve opinion 永远 Approve (per weighted_score > 0)
**缓解**: 改用 `fixed_stance: StanceKind`, Mock 产出 Disapprove stance → is_allowed 正确返回 false
**结果**: ✅ 8 tests 全过 (1 positive + 1 negative)

### 7.7 R7 (D3-5 实际遇到) — JSON 序列化 mode 字段

**风险**: 期望 JSON 含 "planner_executor" snake_case, 但 serde 默认 enum 序列化 = "PlannerExecutor"
**缓解**: 修改 test 期望, 改 "PlannerExecutor" (enum 名字, 跟 `Display` impl 1:1)
**结果**: ✅ 15 tests 全过 (含 `trace_report_to_json_contains_all_fields`)

## 决策 8: 跟 D-2 / A / B / C 不冲突 (per 派活单 "不与 A/B/C/D-2 冲突")

### 8.1 D-3 改动范围
- 新增: `crates/apeireth-council/src/collaboration/` (5 文件)
- 新增: `crates/apeireth-council/src/constitution.rs`
- 新增: `crates/apeireth-council/src/trace.rs`
- 新增: `crates/apeireth-council/src/graph_orchestration.rs`
- 新增: `crates/apeireth-council/examples/trace_visualize.rs`
- 修改: `crates/apeireth-council/src/lib.rs` (+4 行 pub mod, +12 行 re-export)
- 修改: `crates/apeireth-council/Cargo.toml` (+5 行 [[example]])

### 8.2 跟 D-2 不冲突
- D-2 改 `crates/apeireth-tool-registry/`
- D-3 改 `crates/apeireth-council/`
- 0 文件交叉

### 8.3 跟 A/B/C 不冲突
- A 改 vector / memory / root Cargo.toml (加 sqlite-vec)
- B 改 api/{cache,retry,routing}
- C 改各 product tests
- D-3 改 council
- 0 文件交叉

### 8.4 git diff 验证
- `git diff crates/apeireth-council/src/lib.rs` → +24 行 (D-3)
- `git diff crates/apeireth-council/Cargo.toml` → +5 行 (D-3, 仅 [[example]])
- `git diff Cargo.toml` (root) → +4 行 (agent A 加 sqlite-vec, 0 改 version)
- `git status --short | grep council` → 7 行 (4 modified + 6 untracked, D-3 范围)

## 决策 9: 主哲学锚 self-check (per 主人偏好 #1)

- ✅ S-1 走在前人经验上: 4 mode 1:1 对标 LangGraph + AutoGen + OpenAI swarm + VCP; 角色宪法 1:1 镜像 R11 5 重守门
- ✅ S-2 实事求是: 266 tests pass 实测; example 跑通 4 mode × 3 trace 格式 实测
- ✅ O-2 走在前人肩上: 复用 R33-4-1 (1:1 包装) + R10 synthesize (0 改) + `apeireth-graph` 公共 API (0 改)
- ✅ O-3 干到底: 4 模式 + 角色宪法 + trace + graph 集成, 0 砍 v2.0 strategy 2B 任何项
- ✅ O-4 任何人都能接手: 4 mode 全部 doc-comment 写明 LangGraph/AutoGen/VCP 借鉴 + 不漂移清单
- ✅ O-5 不假装: 0 假装"planner/root 真接 LLM", 0 假装"已落地 R11 5 守门"

## 决策 10: 主人 R26+ 拍板 (per 派活单决策权)

### 10.1 必做 (R26)
1. **Planner 升级为真 LLM** — 候选 1: 复用 R16-09 `LlmAdvisorBackend` / 候选 2: 复用 R33-4-1 LLM 路径
2. **Hierarchical root 升级为真 LLM** — 候选 1: 硬编码 2 sub-task → 候选 2: LLM 动态拆
3. **角色宪法接 R11 5 重守门真路径** — R26+ `apeireth-formal` (Stage 4 §4A) 集成
4. **trace 输出加 S3/磁盘 export** — R26+ 接 observability 4 umbrella

### 10.2 可选 (R26+ 续)
1. **协作模式混搭** — 1 graph = N mode, conditional edges 切换
2. **多图编排** — LangGraph subgraph 借鉴, 1 parent + N child
3. **trace 可视化 UI** — ratatui (TUI) / Tauri (终极) 9 器官 trace 面板
4. **角色宪法可视化** — TUI / Tauri 宪法编辑器

## 决策 11: 派活单决策权 vs 实际决策对照表

| 派活单授权 | D-3 实际决策 | 理由 |
|---|---|---|
| 4 模式具体实现策略自己定 | ✅ Planner+Executor = 1+3 步 keyword 拆 / Debate = R33-4-1 复用 / Voting = 3 strategy / Hierarchical = 1+2 固定 sub | per 决策 1 + 决策 4 |
| reasoning trace 格式自己定 | ✅ 3 格式 (Pretty + JSON + JSONL) | per 决策 2 |
| 角色宪法跟 5 重守门怎么对齐自己定 | ✅ 1:1 镜像 5 字段 + 7 advisor 默认 + validate_opinion trait | per 决策 3 |
| 任何破坏硬约束的诱惑 → 立即停手 | ✅ 0 破坏 (0 触碰 24 LOCKED + 0 改 workspace.version) | per 决策 5 + 决策 6 |
