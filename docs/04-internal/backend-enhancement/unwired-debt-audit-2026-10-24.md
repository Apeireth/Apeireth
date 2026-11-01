# 孤儿 API 全量核查报告（2026-10-24 第四轮）

> 背景：[PR #77](https://github.com/Apeireth/Apeireth/pull/77) 把审计覆盖面从
> 3 个目录扩到**全仓 30 个 crate** 后，待判组（G5）一度有 **77 条**没人给理由。
> 本轮把 77 条**逐条查完**，方法是 4 个并行子代理按统一判据实测，
> 每条都要给出「主路径是什么 + 证据文件:行号」。
>
> **结论**：77 条里 **A 类（真缺口）7 条**、**D 类（真死代码）13 条**（已删）、
> 其余 57 条是「主路径存在」或「宿主 API 表面」⇒ 归组，不是债。
>
> **基线 164 → 149**（真减 15 条）。

---

## 1. 最重要的产出：一条判据 + 一个真安全缺口

### 1.1 判据：「先问这个能力有主路径吗」

上一轮（`unwired-memory-audit-2026-10-24.md`）提出，本轮 77 条的核查**再次验证**：

| 形态 | 特征 | 处置 |
|---|---|---|
| **A. 功能缺失** | 主路径**不存在**，功能对用户有意义 | **接线**（真债） |
| **B. 入口冗余** | 主路径**存在且在用** | **删冗余或归组**（不是债） |
| **C. 宿主 API** | 设计上供宿主/外部调用 | 归组 |
| **D. 死代码** | 零调用 + 有更直接的重复实现 | **删** |

**接错形态 B 的代价**：给库加第二套并行逻辑，比不接更糟（两份语义会漂移）。

### 1.2 真安全缺口：**OWASP ASI-01 工具描述投毒防线完全敞口**（已修）

这是本轮最有价值的发现，**且是文档与代码不一致**：

- `docs/01-architecture/system-capabilities.md` 宣称能「阻断静默投毒」；
- 代码里 `ToolDescAuditor`（2026-10 写的完整防线：零宽字符 / Bidi 覆写 / 双语越权
  指令 / 描述体积突增）**生产零调用**；
- 生产 MCP 桥 `catalog.rs:46` 直接 `description: tool.description.clone()`
  —— 外部服务器返回的工具描述**原样**进模型可见目录；
- 唯一的在线注入闸 `PromptInjectionHook` 只扫 capability **arguments**
  （`input_security.rs:672-687`），**不看 description**。

⇒ 外部/被劫持的 MCP 服务器可以在 description 里埋零宽字符、Bidi 覆写、
双语越权指令，**直接进模型上下文**。

**本批已修**（PR #80）：
1. `McpToolEntry::new` 接 `ToolDescAuditor::audit` ——
   `Clean` 原样、`Warning` 用**清洗后**、`Blocked` **拒绝注册**（fail-closed）；
2. `McpToolCatalog::replace_server` 接 `ToolDescAuditor::audit_diff` ——
   拦住「先给干净描述、重连时换掉」的**静默换描述**，且**保留旧描述**；
3. 6 个测试钉死行为（含「正常微调不被拒」——免得这道闸变成噪音源）。

> **一条被测试抓到的 bug（值得记）**：第一版把审计器的**任何** `Err` 都当
> 「空描述」放行，结果注入被放过。审计器用 `Err` 表达两种完全不同的结果
> （`EmptyDescription` = 常态；`InjectionKeywordDetected` = **审计判定拒绝**）。
> 是 `injection_keywords_reject_the_tool` 这个测试抓到的。
> **教训**：安全闸的「错误处理」本身要测试，不能想当然。

## 2. 逐组结果

| 组 | 条数 | 说明 |
|---|---|---|
| G1 集成测试辅助 | 0 | 现被 `tests/` 正确识别为 `test-only`，不进基线 |
| G2 W1 IM 通道 | 1 | owner 裁定暂缓 |
| G3 生物/认知/编排观测 | 31 | 生物族默认关 |
| G4 注入点/探测口 | 38 | 生产走 `install_*` |
| **G5 待判** | **7** | **全是 A 类真缺口**（见 §4） |
| G6 冗余表面/宿主入口 | 72 | 已逐条核实有主路径或属公开 API 表面 |

### 2.1 删掉的 13 条（D 类真死代码）

| 符号 | 位置 | 为什么删 |
|---|---|---|
| `append_fact` | memory/temporal_graph_store | 与 `insert` **逐字相同**，无 doc，零调用 |
| `next_occurrence_after` | orchestration/durable_schedule | 与 `cron::next_after` 重复实现，零消费者 |
| `set_potential` | orchestration/care_potential_field | 「从持久化恢复」的意图不存在（该类型无序列化） |
| `approval_event_fields` | runtime/events | 为**不存在的** `RuntimeEvent::ApprovalRequested` 变体写的 helper |
| `set_event_sink` | runtime/runtime | 替换语义会丢观察者，已被 `add_event_sink` 取代 |
| `record_turn_usage_at` | cli/onboarding | 自称「测试用」但测试绕过它直调内核 |
| `as_session_store` | runtime-assembly/sqlite_session | 一行 `Arc::new` 包装，生产已内联同一转换 |
| `state_for` | runtime-assembly/ritual_state_store | 只读查找零调用，两处已用别的方式实现 |
| `bottom` | memory/layered_memo/progression | 最低层常量直接用 `Layer::L1` 变体表达 |
| `clear_working` | memory/three_layer | 注释自述「debug 用」，连测试都不调 |
| `is_verbatim_zone` | memory/persona_layer | 逐字区由字段分离保证，无人需要此谓词 |
| `evict_one` | storage/cache | 与 `put_at` 内联淘汰重复，唯一声明用途（测试）已消失 |
| `stats_ref` | storage/cache | `stats()` 快照已覆盖全部字段 |
| `__register_all_asserts` | foundation/core | no-op 桩，唯一调用者在已归档的 legacy 示例 |

> 删除全部**逐条核对过**「零调用 + 有更直接的实现或意图已消失」，
> 删除后跑了受影响的 7 个 crate 的测试（2283 例全绿）。

### 2.2 排查掉的虚惊（4 条，最容易被误判成安全缺口）

| 符号 | 担心什么 | 实际 |
|---|---|---|
| `fetch_governed`（公开版） | 治理读被绕过？ | 内部走 `fetch_governed_on_conn`（多处使用），公开版只是包装 |
| `is_episode_forgotten` | 遗忘不生效 ⇒ 已删记忆还被检索？ | `coordinator.rs` 在 5 处调 `get_governed` 查 status，遗忘**会**过滤 |
| `evict_one` | 缓存没有上限？ | `put_at` 在达 `max_size` 时逐出，无 victim 时返回 `CapacityExceeded`（fail-closed） |
| `execute_streaming` | 流式执行没接？ | 已接网关 SSE（`canonical_entry.rs:1973/2046/316`） |

### 2.3 两个**整模块级**发现（不在符号粒度，需另立台账）

| 模块 | 状态 | 后果 |
|---|---|---|
| `JobBoard`（foundation/orchestration） | 全模块仅 `tests/job_output_ring.rs` 构造 | **并行工具派发**、长任务过程输出留存 ⇒ 生产无入口 |
| `CallScheduler`（engine/runtime） | `runtime.rs:719` 恒 `None`，`with_call_scheduler` 仅测试调用 | 同上 |

另：`apeireth-memory` 的 `hallways` / `cross_diary` 整模块未接线；
`EpisodeMeta` 全仓无构造者 ⇒ V4 的 4 列（时序有效性 / 来源）在生产上不生效。

## 3. 工具改进（这一批顺手做的三件事）

核查过程暴露了工具的三个问题，一并修了：

1. **`--update` 会冲掉人工资产** ——
   头部说明与分组标题是人写的判断依据，工具重写文件时把它们丢了
   ⇒ 每次还债都要重新分组，且很可能悄悄丢掉判断依据。
   **现在 `--update` 保留分组标记与注释。**
2. **分组信息只存在于行序里** ⇒ 工具改一下行序，分组就变了，且没法互相验证。
   **现在分组标记放行内**（`orphan <symbol>  # <GROUP>`），
   「谁属于哪组」变成**数据**而非排版。
3. **两个工具各有一份头部定义** ⇒ 必然漂移。
   **现在抽出 `scripts/unwired_baseline_common.py` 共享**，
   `audit_unwired.py --update` 与 `group_unwired_baseline.py` 输出**逐字一致**，
   可以互相验证：

   ```bash
   python3 scripts/audit_unwired.py --update       # 更新条目集合
   python3 scripts/group_unwired_baseline.py        # 按规则重排分组
   python3 scripts/group_unwired_baseline.py --check  # 校验两者一致
   ```

   `group` 脚本还会**自校验**：每条必须恰好落进一个组，不重不漏。

## 4. G5 剩下的 7 条：**全是 A 类真缺口**

| 符号 | 缺口 | 安全/用户后果 |
|---|---|---|
| `clear_session` | 会话结束没有落地口 | guard hook 是**长生命周期单例**；6 张 per-session 表无上限无 TTL，只有 `clear_session` 能清 ⇒ **资源耗尽型 DoS**；且同一 SessionId 复用会把上一会话的跨轮风险历史带进新会话（审计失真） |
| `from_decision` | 收容义务算得出但没人执行 | `revoke_session_grant`（deny && risk≥0.90 的整会话撤销）无自动执行点 ⇒ 高危拒绝后**没有整会话 fail-closed 闸**；`compensation_hint` 无人消费 ⇒ 事后补偿线索到不了任何面板 |
| `begin_wait` / `end_wait` / `idle_elapsed` / `is_tripped` | 空闲预算整组未接线 | `IdleWatchdog` 无生产消费者 ⇒ `timeout.idle_exceeded` **永不产生**；「用户空闲 N 秒就停止消耗」这条能力没有任何入口 |
| `verdict_keys_for_principle` | 哲学键零 hook 引用 | 文档要求 hook 的 deny reason 引用 13 个哲学键名（如「violates NotSafe under S」），仓内零 hook 引用哲学键 ⇒ **与 W5「哲学拦截层不接」的裁定一致**，但文档口径需对齐 |

**这 7 条本轮没有接线**，原因不是做不动，而是各自需要**产品/设计决定**：

- `clear_session`：接线需要知道「会话何时结束」——gateway 有连接生命周期、CLI 有会话结束，
  两者语义不同 ⇒ 要定统一口径。
- `from_decision`：整会话 fail-closed 是**明显的行为变更**（高危后整个会话都不能用），
  影响面大到该由 owner 拍板。
- 空闲看门狗：需要定「空闲」的定义与阈值，且和已有的「停止消耗」开关如何并存。
- 哲学键：owner 已裁定 W5 不接 ⇒ 这一条的正确处置是**把文档口径对齐**，
  而不是接线。

⇒ 归入**待 owner 决策**清单（见 §5），不做「看起来能跑」的假接线。

## 5. 需要 owner 拍板的两件事

| 事项 | 选项 | 我的建议 |
|---|---|---|
| **高危拒绝后是否整会话 fail-closed** | A：自动撤销整会话授权（`from_decision` 接线）<br>B：只标记 + 人工面板撤销（现状） | **A**，但要配套「用户如何知道会话被收容」的自述面，否则用户只看到后续动作莫名被拒 |
| **空闲预算的入口** | A：CLI/gateway 会话结束时调用 `end_wait`，空闲超时产生 `idle_exceeded`<br>B：不做（保持现状） | **A**，但阈值要走配置而不是硬编码 |

`clear_session` 的**内存无界增长**是独立的、无论上面怎么选都该修的问题：
建议在 gateway/CLI 的**会话结束点**接上（即使空闲预算不做，清理也该做）。

## 5.5 第四轮续：两处「有能力没接电」的接线

### 5.5.1 记忆来源：生产写入路径以前**不记来源**

子代理发现 `EpisodeMeta` **全仓无构造者** ⇒ V4 迁移新增的 4 列
（`valid_from_ms` / `valid_until_ms` / `created_ms` / `provenance`）在生产上**全是 NULL**，
读侧 `normalize_meta` 一律兜底成 `Provenance::Manual`（"手动注入"）。

⇒ **一条工具产物和一条用户手写的记忆，在库里长得一模一样**。

这直接断了「记录 ≠ 负责」最关键的一环：**可追溯性没有数据支撑** ——
Astra 那句「平台第四职责 = 影响与补救」要有前提，得先知道**是谁造成的**。

**已修**：`put_episode` 现在一并写全 V4 四列，`provenance` **按 role 保守推导**：

| role | provenance |
|---|---|
| `tool` / `tool_result` / `tool_use` | `Tool` |
| `user` / `assistant` | `Dialog` |
| `system` | `Observation` |
| 其它 / 认不出来 | `Manual`（**不假装**） |

这是**近似**而非精确追踪（真正的来源要由调用点告诉存储层，而 trait 签名锁定），
但它至少让「对话来的 / 工具来的 / 系统注入的」可区分。

### 5.5.2 特征快照表：无界增长 ⇒ 有界

`last_snapshots` 每次评估写一份完整 `FeatureSnapshot`，而 guard hook 是
**长生命周期单例** ⇒ 按「动作数」**无界增长**。而 `clear_session` **连它都不清**。

**已修**：
1. 新增 `snapshot_store.rs`：独立的有界存储（`HashMap` + 插入序队列），
   上限 `MAX_LAST_SNAPSHOTS = 1024`，超量按插入序淘汰最旧的；
2. `clear_session` 现在也清快照。

> per-session 的 6 张表（`session_risk_history` 等）**没有**加上限 ——
> 它们参与判定，淘汰活跃会话的风险历史会**放宽**护栏。那需要 owner 拍板。

### 5.5.3 顺带修掉的两个「工具自己造成的问题」

| 问题 | 怎么发现的 | 怎么修 |
|---|---|---|
| **尺寸棘轮违规**（`hook.rs` 1518 → 1576） | `arch_guard --strict` | **真拆分**：抽出 `snapshot_store.rs` + 测试移出到 `src/hook/tests.rs` ⇒ `hook.rs` 降到 **1030 行** |
| **同名方法跨类型遮蔽**（假阴性） | `clear_session` 莫名从基线消失 | 判据是裸 token 计数，`SnapshotStore::clear_session` 的调用让 `hook.clear_session` 被误判成「已接电」⇒ 改名 `remove_session`，并把盲区写进基线头部 |

### 5.5.4 收容义务：从「算了没人看」到**可观测**

`EnforcementDirective::from_decision` 算出了三项义务：
`revoke_session_grant`（整会话撤销授权）、`mark_trace_high_risk`（标记高风险 trace）、
`compensation_hint`（事后补偿线索）—— **全部无人消费**。

更根本的是：生产路径走 `GovernanceHook::evaluate`，它只返回 `Decision`
（**丢掉 risk_score 与 reasons**）⇒ 收容义务的输入在生产上根本不存在。

**已修（只做可观测，不改行为）**：

1. `GuardEventDto` 加 `containment: Option<EnforcementDirective>` ——
   义务**进事件流**（面板/审计可读）；
2. 生产判定路径上计算它（`containment_obligations()`），只在真触发义务时挂上，
   不给每个事件塞噪音。

**为什么不自动执行 `revoke_session_grant`**：整会话 fail-closed 是**行为变更**
（高危后整个会话都不能用），影响面大到该由 owner 拍板 —— 已在 PR #80 提出。
本批只让义务**能被看见**，不擅自改判定语义。

> 顺带发现：guard 的判定链（`fast_guard → chain_guard → fusion`）主路径是完整的，
> 护栏真正的盲区从来不是「单次判定」，而是「**会话级收容**」与「**会话结束/内存**」。

### 5.5.5 分组演进：G2（IM 通道）并入 G7（owner 裁定不接）

哲学键那条（`verdict_keys_for_principle`）与 IM 通道是**同一性质**：
owner 已裁定**不接/暂缓**，代码**有意保留** ⇒ 合并成一组 **G7「owner 裁定不接」**，
与 G5「待判」区分开。空组 G2 随之移除。

（G5 剩余 4 条全是空闲看门狗 —— 属**待决策**，见 §4。）

### 5.5.6 判据 v4 与两个真问题（第四轮续）

**判据 v4**：`use` 剔除从「逐行」改成「**整块**」。
旧判据逐行剔 `use`，多行 `pub use foo::{ A, B, C };` 只剔第一行
⇒ 花括号里的类型名被漏成「被引用」⇒ **整块未接线的模块看不出来**
（实测 `JobBoard` —— 并行工具派发/长任务输出留存，生产无入口）。

⇒ 整块剔除后**新增检出 15 条真孤儿**。**条数上升是判据变准，不是债变多。**
同时扫描面补 `examples/`（`stamp_artifact_file` 由此从假阳性变正常）。

15 条逐条判定后：删 3 条 D、接线 1 条 A、归 G6 九条、G7 一条（W1 IM）。

#### 真问题 1：读文件被判成有持久化副作用

canonical registry 里 `tool.filesystem` 是「读写合一」的
（`operations = [Read, Write]`、`persistent_effect = true`）—— 静态描述符只能取并集。
但一次具体调用**带参数**（`{"op": "read"}`），这一次是只读的。

不收窄的后果：护栏拿**过宽**的副作用集判风险（「只读一个文件」被当成有持久化副作用）；
且场景对账失败 ⇒ 训练集里的效果标注是错的。

**已修**：`semantics.rs` 加 `narrow_by_structured_readonly` ——
参数明确是只读操作时收窄 `persistent_effect` / `destructive`，
并去掉 `operation_classes` 里的 Write/Modify/Delete。写操作不受影响（两个测试各钉一边）。

#### 真问题 2：场景预期与实现**大面积**不一致

接上 `run_scenario_checked`（fail-closed）后立刻暴露：**上千个场景实例对账不上**
（绝大多数是 `expected_effects` 为空/漏项 —— `reconcile` 把空预期当「预期无副作用」）。

这不是能顺手修完的 —— 需要**单独一轮**补全场景预期数据。
所以本批**不**让导出直接失败（那只是把问题挪到 CI 失败里），而是**让它可见**：

- 导出器改用 `run_scenario_checked` 逐场景跑，**未对账的场景计入统计**，
  写进 manifest 的 `reconciliation` 段；
- 未对账的场景**仍会导出**（不丢数据），但被标记为不可信。

⇒ 门禁与训练流程现在**看得见**这批数据不可信，
而不是拿到一份看起来正常、实际效果标注错误的训练集。

### 5.5.7 一个流程事故：工具漏提交（本地绿、CI 红）

新建三个仓库常驻脚本时，`.gitignore` 的 `*.py`（为挡 sub-agent scratch）
把它们**静默跳过**了 —— `git add` 跳过、`git status` 也不显示。

⇒ **本地全绿**（文件在磁盘上，跑得通），**提交里没有**，
CI 的「孤儿 API 棘轮」直接红（`audit_unwired.py` import 了缺失的模块）。

**教训**：
1. **本地绿 ≠ 提交完整**。`.gitignore` 的静默忽略比报错更危险 ——
   它让「我以为提交了」和「实际提交了」之间出现**没人察觉的差距**；
2. **提交后必须验证提交树本身**，不信任工作区状态
   ⇒ 新增 `scripts/check_commit_tree.py`；
3. `.gitignore` 改成 `!scripts/*.py` 整目录放行 ——
   逐个 `!scripts/xxx.py` 的模式让「新建工具忘了加例外」**不可能发生**。

## 6. 下一批

1. G5 剩 7 条：按 §5 决策后处置（接线 / 文档对齐 / 删）；
2. 两个整模块级发现（`JobBoard` / `CallScheduler`）另立台账条目；
3. `EpisodeMeta` 无构造者 ⇒ V4 四列生产不生效，需定「谁来填来源与时序」。
