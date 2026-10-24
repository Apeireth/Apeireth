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

## 6. 下一批

1. G5 剩 7 条：按 §5 决策后处置（接线 / 文档对齐 / 删）；
2. 两个整模块级发现（`JobBoard` / `CallScheduler`）另立台账条目；
3. `EpisodeMeta` 无构造者 ⇒ V4 四列生产不生效，需定「谁来填来源与时序」。
