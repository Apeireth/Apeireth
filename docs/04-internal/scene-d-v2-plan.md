# 场景 D：v2 长程 AI 判断架构方案（2026-08-27）

> **现状 (2026-08-27) — 新增 P-arch 任务**。v2 工程重构把 13 键降级为哲学标准 + external hook 闸为唯一 runtime 治理（upstream `873d2857` 已装 3 个 hook：Permission / 凭据泄漏 / 注入检测）。但场景 D（需要语义上下文的 AI 判断）当前在 v2 工作区**完全缺位**——v1 的 companion_serve 里有完整实现（F1/F6/E7/W6 等），全在 `legacy/donor/apeireth-companion`。本文是 v2 移植+演化的架构方案。

```
[Document-Meta]
Document:        docs/04-internal/scene-d-v2-plan.md
Version:         Design-1.0 (v2 首发)
Last-Modified:   2026-08-27
Status:          🟢 活跃 (v2 场景 D 架构方案)
```

> 给谁看：架构决策者 + ROADMAP 维护者。
> 读法：先读 §1 框架（场景 D 是什么 + 跟 13 键/external hook 的关系）→ §2 三例详细分析 → §3 v2 架构设计 → §4 优先级与工作量 → §5 待拍板决策。
> **约束**: 不复制 v1 companion_serve 的形态。重构版要做**架构优化**，不是 v1 形态平移。

---

## 1. 框架：v2 三层判断模型

v2 工程重构后，**所有运行时判断归三类**：

| 层 | 判断依据 | 决策形态 | 谁实施 | 实现位置 |
|---|---|---|---|---|
| **L1 结构化规则** | 字符串 / capability_id / 政策列表 | 三态 Decision (Allow/Deny/RequireApproval) | Rust hook（确定性、零延迟） | `crates/foundation/governance/*` |
| **L2 哲学标准** | 13 键判别词汇表 | reason 字符串引用（非拦截） | hook 的 deny reason + CapabilityDescriptor risk | `crates/foundation/core/src/philosophy.rs` + `onion.rs`（已 ✅ 降级） |
| **L3 语义判断**（场景 D） | 对话历史 / 主人偏好 / 长程任务上下文 / 跨 agent 互审 | 需要 LLM（或多 LLM 协作） | runtime 调新基础设施 | **本文设计**：v2 缺位 |

**关键边界**：
- L1 = 拦截器（短延迟、确定）
- L2 = 词汇表（被引用、非强制）
- L3 = 调用器（**新建基础设施**，**不能复用 L1 hook**，否则就是把 LLM 评审塞进 hook —— 退化到 v1 self-introspection 的过度膨胀）

---

## 2. 场景 D 三个例子详细分析

### 2.1 例 1：主人偏好记住 + 主动应用（v1 = F1 情感记忆 + F6 价值内化）

**场景**：主人在第 1 个月说"我不喜欢过度工程"，第 2 个月没说。Apeireth 帮主人做新项目时，**自己**在数据库 schema 设计那里写："我准备加索引，但只加 1 个最需要的那个。"

**机制需求**：
- 持久化主人偏好（按时间线 + 主题标签）
- 长程上下文检索（"我在做这个任务时，主人之前表达过什么相关偏好？"）
- 主动应用（在生成响应/决策时调出偏好作为软约束）

**v1 实现位置**：`legacy/donor/apeireth-companion/memory_extractor.rs`（F1）+ `legacy/donor/apeireth-companion/value_cases.rs`（F6）+ `crates/engine/memory/src/memory_governance.rs`（部分 v2 残留）

**v2 移植路径**：
- **位置**：`crates/engine/memory`（M1B 已经预留 domain，但 ACT-R/价值管线未实现）
- **新增类型**：`UserPreference { topic, stance, evidence_refs, created_at, confidence }` + `PreferenceStore` trait（CRUD + 检索）
- **检索接口**：`Runtime::execute` 在每个 turn 注入 token budget 范围调 `PreferenceStore::recall_for_context(session_id, current_topic)` → `Vec<UserPreference>` 作为软约束注入 transcript
- **写入触发**：AI 在每个 turn 末调 `PreferenceStore::record(stance, evidence)`（**写入点要 explicit**，runtime 强制；不是 AI 自己决定写不写）

**架构选择**：
- ❌ **不**把 F1/F6 当作 hook 实现（hook 是结构化规则，不是长程偏好）
- ✅ 把它当**长程记忆子系统**（与现有 M1B memory crate 合并）
- ✅ runtime 强制每 turn 末有"偏好记录步骤"，**不允许 AI 跳过**

### 2.2 例 2：AI 在长程工作中自我评估（v1 = W6 Brier 自我诊断）

**场景**：Apeireth 帮主人做"3 周分布式系统"任务，第 2 周结束时，**AI 自己**反思："我这周的方向对吗？主人最初说想要'最小可用版本'，但我今天又开始堆配置中心了。这是不是背离了？"

**机制需求**：
- 工作进度与原始目标的对账（what was asked vs what was delivered）
- AI 评估自己输出质量（用 Brier 校准 = 预测置信 vs 实际对错）
- 在工作偏离时主动报警给主人

**v1 实现位置**：`legacy/donor/apeireth-companion/oracle.rs`（Brier 校准）+ `world_model.rs`（部分）

**v2 移植路径**：
- **位置**：新功能（v2 没有对应物）
- **新增类型**：`WorkCheckpoint { task_id, week_n, goals, delivered, self_assessment }` + `DeviationReport { from_goal_id, severity, evidence }`
- **触发**：runtime 在每个 turn 检查 `WorkCheckpoint` 距离上次 `N turns` 时强制 AI 写 checkpoint（**强制频率可配置**，默认每 100 turn 一次）
- **AI 评估自己**的方法：调用一次独立 LLM 评审（**不同模型实例 / 不同 temperature**）对自己上 N turn 的输出打 Brier 分

**关键架构点**：**评自己的 LLM 不能是同一个实例**——如果模型被 jailbreak 影响，它的自我评估也受影响。v2 设计要求 multi-instance：
- 主实例处理用户对话
- 评审实例用不同 temperature（建议 0.3 vs 0.7）独立判断
- 评审实例的 verdict 写到 `VerdictCache`-like 结构（**不是 13 键 verdict cache**，是新结构 `SelfAssessmentCache`），runtime 读这个 cache 决定是否触发 `DeviationReport`

**架构选择**：
- ❌ **不**让 AI 自己反思后写 verdict（v1 模式 = self-introspection 风险）
- ✅ **强制 multi-instance 评审**，runtime 调度（不是 AI 调度）
- ✅ 评审结果进独立 cache，与 13 键 verdict cache 物理隔离

### 2.3 例 3：多 agent 互审（v2 完全缺位）

**场景**：Apeireth 把"设计分布式系统"拆给 3 个子 agent：
- agent-A 做 plan
- agent-B 做 implementation
- agent-C 做 review
- agent-C 评审 agent-B 的代码

**机制需求**：
- 多 agent 编排（runtime 调度子 agent，而非 AI 调度）
- agent 间通信协议（task/result/feedback）
- 评审标准（评审 agent 看什么：API 一致性 / 测试覆盖 / 安全 / 文档）
- 评审结果处置（通过 / 需返工 / 升级到主人）

**v1 实现位置**：`legacy/donor/apeireth-companion/team-lead.rs`（任务调度）+ `legacy/donor/apeireth-companion/orchestrator.rs`（编排），但**v1 没有正式的互审协议**——只有任务队列。

**v2 移植路径**（**最不可替代，工作量最大**）：
- **位置**：**新 crate `crates/engine/orchestrator`**（与 runtime 平级，runtime 不自己调度多 agent）
- **核心类型**：`Subagent { id, role, capabilities, model_config }` + `OrchestrationSpec { plan_agent, implement_agent, review_agent, spec }` + `ReviewVerdict { score, blocking_issues, optional_suggestions }`
- **协议**：`Orchestrator::dispatch(spec) -> Vec<SubagentResult>`，子 agent 是独立 LLM 调用（隔离 prompt、隔离上下文、隔离 trace）
- **评审输出**：review agent 返 `ReviewVerdict`；orchestrator 根据 verdict 决定 pass/iterate/escalate
- **安全边界**：subagent 不能直接调 governance hook（必须通过 runtime 的标准路径）—— 防止"评审 agent 绕过安全闸"

**架构选择**：
- ❌ **不**让主 agent 自己起子 agent（v1 模式 = self-dispatch 安全风险）
- ✅ **runtime orchestrator 调度**，subagent 都是 LLM factory 的独立实例
- ✅ inter-agent 通信是**结构化数据**（JSON protocol），不是自由文本

---

## 3. v2 架构设计

### 3.1 新增 crate 与 trait

```
crates/
├── foundation/
│   ├── core/           (已有：13 键、L2 哲学标准)
│   ├── governance/     (已有：L1 hook，L3 不放进来)
│   └── orchestration/  ← NEW
│       ├── lib.rs
│       ├── subagent.rs          (Subagent struct + LlmFactory 隔离)
│       ├── protocol.rs         (JSON inter-agent protocol)
│       ├── review.rs           (ReviewVerdict + 评审标准)
│       └── preference.rs       (UserPreference + PreferenceStore trait)
├── engine/
│   ├── memory/         (扩展：M1B 加 preference subsystem)
│   ├── orchestrator/   ← NEW (多 agent 调度)
│   ├── provider/       (扩展：加 MultiInstanceProvider for 隔离评审)
│   └── runtime/        (扩展：每 N turn 强制 checkpoint 步骤)
└── capabilities/
    └── tools/          (不变)
```

### 3.2 runtime 的扩展点

新增 3 个 runtime 调用点（**不改现有结构，加 hook 即可**）：

```rust
// crates/engine/runtime/src/canonical/execute.rs
impl Runtime {
    pub fn execute_turn(&self, request: TurnRequest) -> TurnResponse {
        // 1. governance hook (L1, 已有)
        // 2. [NEW] PreferenceStore::recall_for_context() (L3 例 1)
        let prefs = self.preference_store.recall_for_context(&request.session, &request.input);
        // 3. provider complete with prefs as soft constraint in transcript
        let response = self.provider.complete(...with prefs...);
        // 4. [NEW] 每 N turn 强制 WorkCheckpoint
        if self.turn_counter % self.checkpoint_interval == 0 {
            self.force_work_checkpoint(&response);
        }
        // 5. [NEW] Post-turn preference write (AI 调 PreferenceStore::record)
        // 6. governance hook for tool calls (L1, 已有)
        // 7. tool dispatch + governance hook (L1, 已有)
    }
}
```

### 3.3 与 L1/L2 的边界

| 判断类型 | 路径 | LLM 调用 |
|---|---|---|
| L1: capability 能不能调 | governance hook（Permission/PII/injection） | 0 次 |
| L2: 这个 capability 违反 13 键哪一条 | hook deny reason 引用 13 键名 | 0 次 |
| L3: 长程任务方向对不对 | SelfAssessmentCache + 多 instance LLM | 1 次 / N turn |
| L3: 主人之前表达过什么偏好 | PreferenceStore 检索 + transcript 注入 | 1 次 / turn |
| L3: agent-B 的代码对不对 | orchestrator dispatch review agent | 1 次 / review |

**总 LLM 调用预算**（典型 turn）：
- 主对话：1 次
- 偏好检索：0 次（embedding 检索或 metadata filter，可缓存）
- 多 instance 评审（每 N turn）：1/N 次均摊
- 互审（复杂任务）：1 次 / review step（async、不阻塞主对话）

**对比 v1 self-introspection**：
- v1 每个 turn 都让 AI 自己反思：1 次/turn 额外
- v2 每 N turn 才评审：1/N 次均摊
- v2 主对话上下文有偏好注入（被动），不调额外 LLM
- 总成本 v2 < v1，安全性 v2 > v1（multi-instance 隔离）

### 3.4 多 instance 隔离（防止"评审被污染"）

```rust
// crates/engine/provider/src/isolation.rs (NEW)
pub trait IsolatedLlmInstance: Send + Sync {
    /// 独立实例标识
    fn instance_id(&self) -> &str;
    /// 隔离的 prompt（评审实例不接收主对话的 system prompt）
    fn isolated_prompt(&self, task: &ReviewTask) -> String;
    /// 评审输出的归一化（不返回自由文本，返回结构化 ReviewVerdict）
    fn parse_verdict(&self, output: &str) -> Result<ReviewVerdict, ParseError>;
}
```

**约束**：
- 评审实例不允许访问主人偏好（防止"评审被主人偏好污染"）
- 评审实例不允许修改主对话状态（防止"评审 agent 误改主对话"）
- 评审实例**只能用** read-only tools（grep / fetch 等），不能写

---

## 4. 优先级与工作量

| 例 | v2 移植工作量 | 风险 | 价值 | 推荐优先级 |
|---|---|---|---|---|
| 例 1（F1/F6 偏好） | 1-2 周（小） | 低（持久化偏好 = 已有 memory 模式） | 高（主人体验直接提升）| **P3**（与 M1B 记忆同步） |
| 例 2（W6 自我诊断） | 2-3 周（中）| 中（multi-instance 评审要小心）| 中（偏离检测有用但主人能容忍偶尔偏离） | **P5**（在 P3 之后）|
| 例 3（多 agent 互审） | 4-6 周（大）| 高（多 agent 编排 = 新架构，需要 runtime 重构）| **极高**（这是场景 D 最不可替代的功能）| **P6**（长程任务专用，需要新 crate） |

**总工作量**：7-11 周（约 2-3 个月一人）。

**优先级约束**：
- 例 1 → P3（与 M1B 同步进 crate engine/memory）
- 例 2 → P5（与 ProcessSupervisor 同步进 runtime 改造）
- 例 3 → P6（与器官移植同步进 orchestration crate 新建）

### 4.1 不能动的东西（架构约束）

1. **8 哲学锚不增不减**：场景 D 的所有实现必须穿透 S-1~S-3 + O-1~O-5，不新增锚
2. **0 装 PASS**：multi-instance LLM 调用、orchestrator dispatch 全部"必须做"明确标注
3. **机制而非补丁**：orchestrator 不能塞 if-else 特殊路径，要走 trait-based 多态
4. **集成而非分立**：新 crate `orchestration`/`preference` 挂在现有 foundation 或 engine，**不开顶级新层**
5. **13 键 v2 角色 = L2（哲学标准）**：L3 不复用 13 键做强制决策，**只能用 SelfAssessmentCache（新结构）+ PreferenceStore（新结构）**

---

## 5. 待拍板决策

| # | 决策项 | 选项 | 我的推荐 |
|---|---|---|---|
| 1 | L3 评审实例的模型选择 | (a) 同 provider 不同 temperature / (b) 同 provider 不同 model / (c) 不同 provider | (b) 同 provider 不同 model — 同 provider 减少 API key 管理，但 model 不同确保隔离 |
| 2 | PreferenceStore 持久化后端 | (a) 复用 crates/engine/storage SQLite / (b) 独立小 KV store | (a) — 已在 storage 路径上，避免再开 DB 连接管理 |
| 3 | 多 agent 通信协议 | (a) 自由文本（v1 模式）/ (b) JSON 结构化 | (b) — 自由文本让评审 agent 容易被 prompt injection |
| 4 | SelfAssessmentCache 触发频率 | (a) 每 50 turn / (b) 每 100 turn / (c) 每次 tool 失败时 | (c) — 事件驱动比时间驱动更准；50/100 turn 是 fallback |
| 5 | orchestrator 是否进 Cargo workspace | (a) 是（13-crate → 14-crate） / (b) 否（独立 `crates/_arch/orchestrator/`）| (a) — 进 workspace 让它与其他 crate 走同一 CI + 一致治理 |
| 6 | 例 3 是否需要"主人审批 orchestrator 编排" | (a) 否（orchestrator 是 runtime 内部）/ (b) 是（重大任务先给主人看 plan） | (b) — 长程任务应该让主人知道分了什么给谁 |
| 7 | L3 总延迟预算 | (a) 与 L1 hook 同延迟（<100ms）/ (b) 异步并行（不阻塞主对话） | (b) — L3 评审不能阻塞主对话流 |

---

## 6. 落实路径（推荐 3 阶段）

### Phase 1：例 1 — PreferenceStore（P3 同步进 M1B）
- 时间：1-2 周
- 文件：`crates/foundation/orchestration/preference.rs` (new) + `crates/engine/memory/src/preference.rs` (impl) + `crates/engine/runtime` 集成 recall_into_transcript
- 测试：偏好 recall 正确性 + 写入 race condition + transcript 注入 token budget
- 风险点：检索成本（如果用 embedding，要新加 model）

### Phase 2：例 2 — SelfAssessmentCache + Multi-Instance 评审（P5）
- 时间：2-3 周
- 文件：`crates/engine/provider/src/isolation.rs` (new IsolatedLlmInstance trait) + `crates/engine/runtime` WorkCheckpoint 步骤
- 测试：multi-instance 不共享 prompt / 评审输出 schema 强约束
- 风险点：multi-instance 延迟（要异步 + 不阻塞）

### Phase 3：例 3 — Orchestrator + 多 agent 互审（P6）
- 时间：4-6 周
- 文件：`crates/engine/orchestrator/` (new crate) + JSON protocol spec + 3 个 role-specific agent templates
- 测试：互审 verbatim 场景（plan 评审代码、code 评审测试、test 评审文档）
- 风险点：orchestrator 是新架构，与 runtime 边界要清晰（不能 cycle）

**总计 7-11 周（一人全时），与 ROADMAP §4 P3-P6 完美对齐。**

---

## 7. 与 ROADMAP / 已完成工作的关系

| ROADMAP 项 | 关系 |
|---|---|
| §4 P3 M1B 记忆全量移植 | 例 1 嵌进这里（PreferenceStore 是 M1B 的一部分）|
| §4 P5 ProcessSupervisor + 沙箱强化 | 例 2 的 multi-instance 隔离逻辑与沙箱强化共同进 runtime 改造 |
| §4 P6 companion 器官移植 | 例 3 与器官移植同期推进（orchestrator 是"AI 怎么组织自己工作"的机制） |
| §4 P-arch 场景 D 评估（**新加**） | 本文档 |
| 已完成 P0 治理接线（upstream 873d2857） | 不冲突：L1/L2 不变，L3 是新层 |

---

_本文 v2 首发 (2026-08-27)：场景 D 是 v2 架构必须补的层（v1 有，v2 砍了，砍得不彻底）。本方案按"不复制 v1 形态，做架构优化"原则：从 13 键降级 → 场景 D 的 v2-native 设计（multi-instance LLM + JSON 协议 + 5 原则洋葱挂载），不依赖 self-introspection，不复制 companion_serve 整套。三阶段 7-11 周，与 ROADMAP §4 P3-P6 对齐。**0 漂移承诺**: 8 哲学锚 / 13 键 LOCKED 数据 / 3 项不可变脊柱语义 / workspace.version / R11 baseline 全部不变；本批新增 `crates/foundation/orchestration/` crate + 多 instance LLM 隔离 + orchestrator crate，**不**改任何 LOCKED public API。_
