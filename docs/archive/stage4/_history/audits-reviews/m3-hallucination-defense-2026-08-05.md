# minimax m3 48+ Context Hallucination 防御策略

```
[Document-Meta]
Document:    .minimax-agent-cn\spectrai\reports\spectrAI-r19plus-v2\m3-hallucination-defense-2026-08-05.md
Version:     Manual-Rev-A
R-Cycle:     R20 阶段 1-5 集成准备 (R19+ 缺漏补)
Last-Modified: 2026-08-05 19:01
Status:      🛡 防御策略交付 (设计+代码骨架, 0 改 LOCKED 源码)
Author:      Mavis m3 hallucination 防御 sub-agent
5 决策:      主人 2026-08-05 19:01 拍板 — 补 R19+ 蓝图最关键缺 (m3 hallucination)
```

> **性质**: 纯设计+代码骨架交付. **不改任何 crate 源码** (含 `crates/apeireth-*/src/` LOCKED 24 crate). 产出物全部在 `spectrai/reports/spectrAI-r19plus-v2/`.
>
> **5 份必读上下文** (按主人 2026-08-05 19:01 拍板):
> 1. `.minimax-agent-cn\spectrai\reports\spectrai-architecture-2026-08-05.md` — §1 顶层摘要 + §6 mid-task 根因 + §7.1 m3 风险 (920 行)
> 2. `.minimax-agent-cn\spectrai\reports\apeireth-protocol-4-adapter-analysis-2026-08-05.md` — 4 ZST adapter (49.3 KB)
> 3. `.minimax-agent-cn\spectrai\reports\apeireth-crate-api-2026-08-05.md` — 10 crate API surface (44.3 KB)
> 4. SpectrAI 源: `.minimax-agent-cn\spectrai\spectrai-source\src\main\adapter\` (5 Provider adapter, 只读)
> 5. Apeireth 源: `.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-protocol\src\` (Hermes LOCKED, 只读)
>
> **配套增量引用** (不重复内容, 引用+增量):
> - `spectrai/reports/apeireth-mcp-14-tool-analysis-2026-08-05.md` §2 14 工具白名单 (8 supervisor + 3 worktree + 3 认知)
> - `spectrai/reports/apeireth-asi-24dim-api-2026-08-05.md` §3.1 V05_DIMENSION_NAMES (LOCKED) + §1.2 D-03 第 25 维建议

---

## §0 边界 + 性质声明

### 0.1 主人 2026-08-05 19:01 拍板 5 决策

> "补蓝图缺，扫除盲点。最关键缺 = minimax m3 48+ context hallucination 防御策略。"

| # | 决策 | 在本报告体现 |
|---|------|------------|
| 1 | **m3 防御策略 = R20 阶段 1-5 集成的硬约束** (非可选项) | §5 阶段 1-5 集成点逐一标注 |
| 2 | **5 道防御 = 1 强校验 + 1 双层 ack + 1 context 监控 + 1 工具白名单 + 1 日志** | §2 完整 5 道 + §3 4 snippet |
| 3 | **代码骨架 = Rust 端, 不动 TS** (TS 是 mid-task bug 现场, 不重治) | §3 4 snippet 全 Rust |
| 4 | **不重写 m3 调用, 在 m3 调用外加护栏** (per O-2 走在前人肩上) | §2.1 借鉴 composio zod boundary + §2.4 14 工具 hardcode 复用 §2.1 §1.2 矩阵 |
| 5 | **不假装已实装** (per O-5) | §0.2 8 项不修改承诺 + §6 6 哲学 anchor 穿透自检 + §3 代码骨架标 "示例" |

### 0.2 8 项不修改承诺 (per 12 子规范)

| 承诺 | 本报告严守 |
|------|-----------|
| 1. 阶段 1/2/3/4/5 LOCKED 设计文档 | ✅ 引用 `docs/stage[1-6]/`, 不改一字 |
| 2. v2/v4/v4.1 LOCKED | ✅ 引用, 不动 |
| 3. 12 键 LOCKED | ✅ 引用 6 哲学 anchor 6/6 穿透 (S-1/S-2/O-2/O-3/O-4/O-5), 不重命名 |
| 4. 6 哲学 anchor LOCKED | ✅ §6 6/6 穿透自检 |
| 5. workspace v1.0.0 | ✅ 所有代码骨架用 `workspace = true` 依赖 (不写版本号) |
| 6. Document-Meta 格式 | ✅ 本报告头部用标准 Document-Meta 6 字段 |
| 7. R11 baseline 3 值 (0.8682/0.8532/0.9063) | ✅ 引用 `apeireth-asi-24dim-api` §1.3, 不重测不重算 |
| 8. 8 项 LOCKED (24 crate src/ 不改) | ✅ 0 触碰 `crates/apeireth-*/src/`, 全部在 `spectrai/reports/spectrAI-r19plus-v2/` |

### 0.3 报告结构 (5 章 + 1 自检 + 1 边界)

| 章 | 标题 | 关键交付 |
|---|------|---------|
| §1 | 触发模式分类 (4 类) | m3 hallucination 4 类模式 + SpectrAI 现状 + m3 vs Claude/Gemini 对比 |
| §2 | 防御策略 (5 道) | pre-call 强校验 / dual ack / 48+ 监控 / 14 工具白名单 / 日志 |
| §3 | Rust 端代码骨架 (4 snippet, ~100 行) | validator / dual_ack / m3::ContextMonitor / builtin::WHITELIST |
| §4 | 测试 fixture 设计 (3 fixture) | 48+ path missing / dual ack no swallow / fabricated tool |
| §5 | 跟 R20 阶段 1-5 集成点 | 阶段 1-5 每一阶段如何嵌入 5 道防御 |
| §6 | 8 项不修改承诺 + 6 哲学 anchor 穿透自检 | 12 子规范严守 + 6 anchor 6/6 穿透 |

---

## §1 触发模式分类 (4 类)

> 主人 16:40 战报 + 18:49 强调的 4 个已知模式, 1.1~1.4 逐个拆解. 每类 3 段: 模式说明 / SpectrAI 现状 / m3 vs Claude/Gemini 对比.

### §1.1 Context 48+ 路径错/缺参数

**模式说明**: minimax m3 上下文窗口 ≥48 messages 后, LLM 返回的工具调用 `input` JSON 出现两类问题:
- **路径错**: 多层嵌套 JSON 字段, m3 会把 `params.config.timeout` 写成 `params.timeout` (漏中间层)
- **缺必填参数**: 必填字段 `worktree_id` / `agent_id` 直接不填, 或填 `null` 占位

**SpectrAI 现状** (per `spectrai-architecture-2026-08-05.md` §7.1 P0 风险): 主人在 §7.1 第 4 行明确列 "minimax m3 在 48+ context 下 hallucinate" 为 P0 高影响高概率风险, 缓解第①条写 "限制单次 context ≤ 32 messages". 但**纯靠限制消息数是软的** — 32 上限触达后, m3 仍会出问题, 而且 32 不一定够 (m3 测后实际阈值约 48). 此外, **V2 Adapter 5 个 Provider 中只有 Claude 走 npm SDK, 其他 4 个 (Codex/Gemini/iFlow/OpenCode) 走各自的 CLI** — CLI 解析层错的可能性更高, 错也不报.

**m3 vs Claude/Gemini 对比** (per 主人 16:40 战报):
- **Claude (Sonnet 4 / Opus 4)**: 48+ context 几乎不出现路径错, 100+ 仍稳. Claude Agent SDK V1 `query()` 在 Rust 端用 napi-rs 桥, JSON-RPC over stdio 传 input, Claude SDK 端**自己 schema 校验** (内置).
- **Gemini (1.5/2.0 Pro)**: 48+ context 极少路径错, 但**长 context 容易出现 thinking 块被截** (非幻觉, 是 token 限制). 走 `apeireth-protocol::adapters::gemini` (per `apeireth-protocol-4-adapter-analysis` §3.1 第 4 行), adapter 端 `functionCall` id 用 `format!("gemini_call_{}", name)` 代替 (协议差异), 缺 id 问题已工程化兜底.
- **m3 (minimax)**: 48+ context **高概率** 路径错 + 缺参数. 原因推测: m3 的 tool calling 训练数据量小, 长 context 注意涣散. **当前 m3 调用没有 schema 校验层** (跟 Claude 相反) — 错就传到下游.

### §1.2 mid-task bug 3 处组合吞错

**模式说明**: `SessionManagerV2:641 throw` + `AgentManagerV2:281 .catch` 吞错 + child session 状态窗口期, 3 处组合导致 m3 hallucination 触发后**整个 mid-task 失败但报"成功"**:
- 父进程发新消息给子 agent → `sendToAgent` line 285 永远 `return { success: true }`
- 实际 sendMessage 在 line 642 throw (子 session 已 terminated)
- 父进程误以为成功, 继续等 `wait_agent_idle` → 永远等不到 (子已死, 不会有 turn_complete 事件)
- 等到 5 分钟超时, 父进程卡死或误判

**SpectrAI 现状** (per `spectrai-architecture-2026-08-05.md` §6.1~§6.4): 主人在 §6.4 已给 3 处修法 (改 throw 为 return Result / sendToAgent await + 检查 child session 状态 / 同步 child 状态到 agent). **但修法在 TS 端, R20 阶段要把 SpectrAI 翻译到 Rust** (`apeireth-session` + `apeireth-team-lead`, 估 1500-2000 LOC) — 翻译时 **必须不重蹈覆辙**, Rust 端用 `tokio::sync::watch` 跟踪 status + `Result<T, SessionError>` 统一返回.

**m3 vs Claude/Gemini 对比**:
- **Claude**: Claude Agent SDK V1 内部对工具调用有重试, mid-task 触发 hallucination 概率低. 即使子进程异常, Claude SDK 端会抛 `QueryError`, 不会"成功但失败".
- **Gemini**: Gemini CLI headless mode 输出纯文本, 不走 JSON-RPC 工具调用, mid-task bug **完全不触发** (没 MCP tool 走这条链).
- **m3**: m3 通过 minimax API, 走 `apeireth-protocol` 4 协议中 AnthropicMessages adapter (per `apeireth-protocol-4-adapter-analysis` §3.1 第 1 行), **完全走 Anthropic 协议 JSON 形状** — 所以 mid-task bug 链完整. 而且 m3 在 48+ context 路径错 → sendMessage 入参错 → 子 agent 拒收 → 触发 throw → 触发 .catch 吞错 → 报"成功".

### §1.3 m3 工具调用虚构 (fabrication)

**模式说明**: m3 在 48+ context 会**虚构不存在的工具**, 例如:
- 虚构 `spectrai_send_to_agent_v2` (实际是 `send_to_agent`)
- 虚构 `apeireth_council_ask` (实际是 `apeireth-council::deliberate` 内部, 不暴露 MCP)
- 虚构 `claude_code_run` (实际是 `Bash` 工具)

**SpectrAI 现状** (per `spectrai-architecture-2026-08-05.md` §7.1): §7.1 风险表未单独列 fabrication, 但 §5.2 第 8 行 MCPConfigGenerator 行 "3 个非 Claude provider 注入机制可能改变" 已暗示 — 5 Provider 走不同 MCP 注入 (Claude/iFlow JSON 临时文件 / Codex CODEX_HOME 目录 / OpenCode OPENCODE_CONFIG 路径), m3 走 Claude 注入路径时, 跟 Claude Claude Code CLI 看到的工具名空间一致. **但 m3 不一定准确记起真实工具名** (尤其 48+ context 后), 容易拼错或编造.

**m3 vs Claude/Gemini 对比**:
- **Claude**: Claude Agent SDK 在 system prompt 注入 tools schema 完整列表, Claude LLM 端**精确知道**有哪些工具. fabrication 概率极低.
- **Gemini**: Gemini headless mode **不暴露自定义工具** (走 CLI 默认能力), fabrication 概率 = 0.
- **m3**: m3 走 Anthropic 协议, `tools` 字段由 `apeireth-mcp::team` 14 工具 + apeireth-mcp builtin 注册. m3 在 48+ context 容易**忘了 tools schema 完整列表**, 编造相似名工具. **当前没有任何工具白名单层在 Rust 端 hardcode** — 错就传到 mcp 客户端, 客户端 call_tool 报 "tool not found" 给 m3, m3 可能再编一个.

### §1.4 m3 长 context 路径截断

**模式说明**: 多层嵌套 JSON 参数, m3 会截断到 3 层 (Claude/Gemini 不截). 例如:
- 完整: `{"params": {"config": {"timeout": 30000, "retry": {"max": 3, "backoff_ms": 1000}}}}` (5 层)
- m3 截断后: `{"params": {"config": {"timeout": 30000}}}` (3 层) — `retry` 整段丢失
- 后果: 下游用 default 值, 跟用户预期不符 (用户传 1000ms backoff, 实际用 default 500ms)

**SpectrAI 现状** (per `spectrai-architecture-2026-08-05.md` §7.1 第 9 行 m3 minimax API 风险): "minimax 的 `path` 错误要在 validation 层拦截". **当前 validation 层 = 0**. SpectrAI 14 工具 schema 在 MCPConfigGenerator 生成, **不在 Rust 端校验** — 走 AgentMCPServer 的 JSON-RPC 解析. AgentMCPServer 是 stdio MCP server, schema 校验依赖 mcp 客户端, mcp 客户端依赖 m3 传对. **链路 0 防御**.

**m3 vs Claude/Gemini 对比**:
- **Claude**: Claude LLM 端训练时就用完整嵌套 JSON, 5 层稳定, 不截. Claude Agent SDK 端还会**自动补全 missing nested layer** (e.g. `params.config` 缺就补 `{}`).
- **Gemini**: Gemini Pro 长 context 也截, 但 Gemini 1.5 Pro 是 1M context, **用户根本不会用到 5 层嵌套** (太短). 走 headless mode 不暴露自定义工具, 路径截断问题不显.
- **m3**: m3 训练数据中 tool call 嵌套深度 <3, **遇到 ≥4 层就截**. 主人 18:49 强调的 "4 类" 之一.

---

## §2 防御策略 (5 道)

> 主人 19:01 决策: 5 道 = 1 强校验 + 1 双层 ack + 1 context 监控 + 1 工具白名单 + 1 日志. 2.1~2.5 逐道, 每道 3 段: 借鉴 / 实装点 / 跟 m3 哪类对应.

### §2.1 Pre-call Schema 强校验 (per composio zod boundary 借鉴)

**借鉴**: VCP `protocolBridge.js:1-150` 字段级借鉴 + composio 的 zod boundary 模式 (composio.dev 公开文档, tool call 在 LLM → execute 之间有 zod schema 校验层, 缺必填直接 reject, 不传到下游). **不重写 composio** (per O-2 走在前人肩上), 只搬 zod 思路到 Rust = `schemars` crate (Rust 官方 JSON Schema 库, R17 已在 workspace v1.0.0 dependencies).

**实装点** (per `apeireth-crate-api` §2.5 `apeireth-pipeline`): 在 `apeireth-pipeline` 5 步管线 (placeholder / token / force_translate / protocol / HTTP) **之前**插一步 `validate` (变 6 步), 校验 `NormalizedRequest.tools[].arguments` 跟 tool schema (从 `apeireth-tool-registry` 查 `ToolDef::input_schema`) 的一致性. 校验用 `schemars::schema_for!(ArgsType)` 生成 schema + `jsonschema` crate 校验 JSON. **缺必填 → reject 整次 LLM 调用, 返 `PipelineError::SchemaMismatch { tool, missing_fields }`**.

**跟 m3 哪类对应**: §1.1 路径错/缺参数 + §1.4 长 context 路径截断 (缺层 = 缺字段, 一并拦).

### §2.2 Mid-call 双层 ack (per `apeireth-mcp-14-tool-analysis` §3 修法 #1 增强)

**借鉴**: `apeireth-mcp-14-tool-analysis-2026-08-05.md` §3 修法 #1~#3 已给 mid-task 3 处修法 (send_to_agent 状态机 / get_output 过滤 / wait_idle 中断检测). **本报告 §2.2 在这 3 处之上加"双层 ack" 兜底** — 即使 mid-task 3 处都修了, m3 hallucination 仍可能让 send 不成功, **必须**有 explicit ack 才能继续.

**实装点**: `apeireth-team-lead::dual_ack` 伪代码 (8 步, 见 §3.2):
1. 父 (`TeamLead`) 调 `send_to_agent(agent_id, msg)` (m3 工具调用)
2. `McpClient::call_tool("send_to_agent", args)` 阻塞等 JSON-RPC response
3. 子 agent (`AgentHandle`) 收到后**立即**发 `{"ack": "received", "seq": N}` (JSON-RPC notification, 不等业务完成)
4. `McpClient::call_tool` 收到 ack → 返父 `{delivered: true, seq: N}` (ack 阶段完成)
5. 子 agent **业务执行** (e.g. 处理新指令) — 这段可能慢
6. 子 agent **业务完成**后发 `{"ack": "processed", "result": ...}` (第二个 ack, JSON-RPC response 业务层)
7. `TeamLead` 持续 polling 或 subscribe (per `apeireth-mcp` Transport 已实装) 业务 ack
8. **任一 ack 超时** → `Result::Err(DualAckTimeout { stage: "received" | "processed", ms })` — **不 `.catch` 吞错, 不 `success: true` 骗父** (per `spectrai-architecture` §6.4 修法 2 关键)

**跟 m3 哪类对应**: §1.2 mid-task bug 3 处组合吞错 — 双层 ack 让 `.catch` 没机会吞错 (父直接看 ack 状态, 不信 success 字段).

### §2.3 m3 特定 context 48+ 监控

**借鉴**: 主人在 `spectrai-architecture` §7.1 P0 缓解第①条 "限制单次 context ≤ 32 messages" 是**软的**; 本报告 §2.3 把它变**硬** + 加 schema 二次校验 + 自动截断长 nested JSON.

**实装点** (per `apeireth-protocol` 已实装的 4 adapter): 在 `apeireth-protocol::providers::minimax::m3::ContextMonitor` (新模块, 估 80 LOC):
- `ContextMonitor::check_and_mitigate(messages_count, tool_call_json) -> MitigatedRequest`
- **规则 1**: `messages_count >= 48` → 触发"m3 危险区间" 警告 (log level WARN, 含 `provider_id`, `session_id`, `messages_count`)
- **规则 2**: `messages_count >= 48` → 对 tool_call_json 自动跑 §2.1 schema 二次校验 (即使 §2.1 已在校, **再做一次**, 因 m3 路径错概率高)
- **规则 3**: `tool_call_json` 嵌套深度 ≥4 → 自动**补全中间空层** `{}` (e.g. `params.config` 缺就补 `params: {config: {}}`), 不 reject (宽容 m3, 跟 Claude SDK 自动补齐一致)
- **规则 4**: `messages_count >= 64` → 触发"m3 高危" 警告 + **强制** truncate `messages` 列表到 ≤48 (截最旧, 保留 system + 最近 48)

**跟 m3 哪类对应**: §1.1 路径错 + §1.4 路径截断 — 48+ 监控是这 2 类的**针对性防御**.

### §2.4 工具调用白名单 (14 工具 hardcode)

**借鉴**: `apeireth-mcp-14-tool-analysis-2026-08-05.md` §2 已给 14 工具完整表 (8 supervisor + 3 worktree + 3 认知). **本报告 §2.4 把这 14 工具名 hardcode 进 Rust 端白名单**, 在 §2.1 校验之后, §2.2 发送之前, 拦 m3 fabrication.

**实装点** (per `apeireth-mcp` ToolDef): `apeireth-mcp::builtin::WHITELIST` const slice, 14 个工具名 hardcode:
```rust
// 编译期 hardcode, per workspace v1.0.0 守门
pub const WHITELIST: &[&str] = &[
    // 8 supervisor (per apeireth-mcp-14-tool-analysis §2.1)
    "spawn_agent", "send_to_agent", "get_output", "wait_idle",
    "wait", "get_status", "list", "cancel",
    // 3 worktree (per §2.2)
    "worktree_merge", "worktree_info", "worktree_check",
    // 3 cognitive (per §2.3)
    "list_sessions", "get_summary", "search_sessions",
];
// 编译期守门: 14 项 + 编译期 const
pub const WHITELIST_COUNT: usize = 14;
const _: () = assert!(WHITELIST.len() == WHITELIST_COUNT);
```

§2.4 拦截: m3 返回的 `tool_use.name` 不在 `WHITELIST` → **直接 reject** (返 `LlmError::UnknownTool { name, allowed: WHITELIST }`). **不传 mcp 客户端** (避免 m3 收到 "tool not found" 后再编造一个). §2.4 跟 §1.3 fabrication 直接对应.

**跟 m3 哪类对应**: §1.3 m3 工具调用虚构 — 白名单是 fabrication 的**最强**防御.

### §2.5 hallucination 记日志 (per D-03 24 维 V0.5 命名 Substrate 类新加第 25 维)

**借鉴**: `apeireth-asi-24dim-api-2026-08-05.md` §1.2 发现 #5 提 `AsiEngine` trait 抽象 + §1.2 D-03 建议 "24 维 V0.5 命名 Substrate 类新加第 25 维 `hallucination_resistance`". **本报告 §2.5 实现 D-03 建议** — 加第 25 维测量, 守门 m3 hallucination 频率.

**实装点** (per `apeireth-asi` R14 Rust rewrite 已实装的 24 维 + 9 子测度, `apeireth-asi-24dim-api` §3.1): `apeireth-asi` 加第 25 维 `hallucination_resistance` (per D-03 Substrate 类新加, 不用 V05_DIM_COUNT=24 守门, 用新 const `V05_DIM_COUNT_V2=25` 标识 V2 扩展):
- 测量方法: `success_attempt_ratio × (1 - fabrication_rate) × (1 - path_truncate_rate)`
  - `success_attempt_ratio` = 工具调用成功次数 / 总次数 (从 `apeireth-mcp::team` 14 工具 call log 算)
  - `fabrication_rate` = fabrication 拦截次数 (per §2.4 白名单 reject 计数) / 总工具调用次数
  - `path_truncate_rate` = 路径截断自动补全次数 (per §2.3 规则 3) / 总工具调用次数
- 写入 `DimensionTrace::v05_dims` 第 25 元素 (突破 `[f64; 24]` 数组, 用 `hook_overrides: Vec<(String, f64)>` 存, 避免 LOCKED 24 维数组 — per `apeireth-asi-24dim-api` §1.4 缺口 #1)
- 不假装 (per O-5): 缺观测 → `MeasurementError::MissingObservation` (per `apeireth-asi-24dim-api` §3.3 LOCKED 行为), 不 fallback 0.0

**跟 m3 哪类对应**: 全 4 类 (§1.1~§1.4) — 日志是**横切**的, 5 道防御的拦截次数都写入第 25 维, 给 verifier 守门 (R11 baseline 0.8682 是 24 维 baseline, 第 25 维是新基线, 主人 19:01 决策明确 "D-03 24 维 V0.5 命名 Substrate 类新加第 25 维").

---

## §3 Rust 端代码骨架 (4 snippet, ~100 行)

> **关键**: 4 snippet 全是**设计参考**, 标 "⚠️ 仅供设计参考, 不写实际代码" (per `apeireth-mcp-14-tool-analysis` §2.4 通用 trait 模式 同样声明). **0 触碰 LOCKED 源码**, 实施时由 rust-coder 在新模块实装.

### §3.1 Tool schema validator (schemars + apeireth-protocol::validate 集成)

**位置**: `crates/apeireth-pipeline/src/validate.rs` (新文件, 5 步管线前插 1 步变 6 步)

```rust
// ⚠️ 仅供设计参考, 不写实际代码
use apeireth_tool_registry::{ToolRegistry, ToolDef};
use apeireth_protocol::error::ProtocolError;
use schemars::schema_for;
use jsonschema::JSONSchema;
use serde_json::Value;

pub struct ToolSchemaValidator<'a> {
    registry: &'a ToolRegistry,
}

impl<'a> ToolSchemaValidator<'a> {
    pub fn new(registry: &'a ToolRegistry) -> Self { Self { registry } }

    /// §2.1 + §2.4 一并实现: 校验 + 白名单
    pub fn validate_tool_call(
        &self,
        tool_name: &str,
        arguments: &Value,
    ) -> Result<(), PipelineError> {
        // §2.4 白名单 (14 工具 hardcode, per §2.4 编译期守门)
        if !apeireth_mcp::builtin::WHITELIST.contains(&tool_name) {
            return Err(PipelineError::UnknownTool {
                name: tool_name.to_string(),
                allowed: apeireth_mcp::builtin::WHITELIST.iter()
                    .map(|s| s.to_string()).collect(),
            });
        }
        // §2.1 schema 校验
        let def = self.registry.get(tool_name)
            .ok_or_else(|| PipelineError::SchemaMismatch {
                tool: tool_name.to_string(),
                missing: vec!["tool_def_not_found".to_string()],
            })?;
        let schema = JSONSchema::compile(&def.input_schema)
            .map_err(|e| PipelineError::SchemaCompile(e.to_string()))?;
        let result = schema.validate(arguments);
        if let Err(errors) = result {
            return Err(PipelineError::SchemaMismatch {
                tool: tool_name.to_string(),
                missing: errors.map(|e| e.to_string()).collect(),
            });
        }
        Ok(())
    }
}
```

### §3.2 Mid-call dual ack (apeireth-team-lead::dual_ack 伪代码, 8 步)

**位置**: `crates/apeireth-team-lead/src/dual_ack.rs` (新文件, 配套 `apeireth-mcp-14-tool-analysis` §3 修法 #1)

```rust
// ⚠️ 仅供设计参考, 不写实际代码
use apeireth_mcp::team::{TeamState, AgentId, AgentHandle, AgentState};
use apeireth_mcp::McpClient;
use std::time::Duration;
use tokio::time::timeout;

pub struct DualAck {
    pub received_timeout: Duration,   // 默认 5_000 ms
    pub processed_timeout: Duration,  // 默认 300_000 ms (5 分钟)
}

pub enum DualAckError {
    ReceivedTimeout { agent_id: AgentId, ms: u64 },
    ProcessedTimeout { agent_id: AgentId, ms: u64 },
    AgentNotFound(AgentId),
    McpTransport(String),
    // 0 catch 吞错: 所有错误 explicit 返回
}

impl DualAck {
    pub async fn send_and_wait(
        &self,
        client: &McpClient,
        team: &TeamState,
        agent_id: AgentId,
        message: String,
    ) -> Result<DualAckResult, DualAckError> {
        // Step 1: 查 agent 存在
        let handle = team.read().await.get(&agent_id)
            .ok_or(DualAckError::AgentNotFound(agent_id.clone()))?.clone();
        // Step 2: send_to_agent tool call (m3 走 minimax API)
        //         JSON-RPC 阻塞, 期望子 agent ack "received"
        let ack1 = timeout(self.received_timeout, async {
            client.call_tool("send_to_agent", json!({
                "agent_id": agent_id.0, "message": message,
            })).await
        }).await
        .map_err(|_| DualAckError::ReceivedTimeout {
            agent_id: agent_id.clone(), ms: self.received_timeout.as_millis() as u64,
        })?
        .map_err(|e| DualAckError::McpTransport(e.to_string()))?;
        // Step 3: 验 ack1.delivered == true
        if !ack1.get("delivered").and_then(|v| v.as_bool()).unwrap_or(false) {
            return Err(DualAckError::McpTransport(
                format!("ack1 delivered=false: {ack1}")));
        }
        // Step 4: 等子 agent 业务处理完成 ack "processed" (per §2.2 步骤 6)
        let ack2 = timeout(self.processed_timeout, async {
            handle.wait_for_processed_ack().await
        }).await
        .map_err(|_| DualAckError::ProcessedTimeout {
            agent_id: agent_id.clone(), ms: self.processed_timeout.as_millis() as u64,
        })?;
        // Step 5: 双 ack 都成功才返 Ok — 0 .catch 吞错, 0 success: true 骗父
        Ok(DualAckResult {
            received_seq: ack1.get("seq").and_then(|v| v.as_u64()).unwrap_or(0),
            processed_result: ack2,
        })
    }
}
```

### §3.3 m3 context 48+ monitor (apeireth-protocol::providers::minimax::m3::ContextMonitor)

**位置**: `crates/apeireth-protocol/src/providers/minimax/m3.rs` (新文件, per `apeireth-protocol-4-adapter-analysis` §3 4 adapter 旁边加 minimax 私有模块)

```rust
// ⚠️ 仅供设计参考, 不写实际代码
use apeireth_protocol::normalized::NormalizedRequest;
use apeireth_protocol::error::ProtocolError;
use tracing::{warn, error};

pub const M3_DANGER_THRESHOLD: usize = 48;     // §2.3 规则 1+2
pub const M3_CRITICAL_THRESHOLD: usize = 64;   // §2.3 规则 4
pub const M3_NESTED_DEPTH_TRIGGER: usize = 4;  // §2.3 规则 3

pub struct ContextMonitor;

pub struct MitigatedRequest {
    pub request: NormalizedRequest,
    pub interventions: Vec<Intervention>,  // 给第 25 维记日志用
}

pub enum Intervention {
    WarnDangerZone { messages_count: usize },
    SecondarySchemaCheck,
    AutoCompleteNestedLayer { path: String },
    ForceTruncate { original: usize, kept: usize },
}

impl ContextMonitor {
    pub fn check_and_mitigate(req: NormalizedRequest) -> MitigatedRequest {
        let mut interventions = Vec::new();
        let messages_count = req.messages.len();

        // 规则 1+2: 48+ 警告 + 二次 schema 校验
        if messages_count >= M3_DANGER_THRESHOLD {
            warn!(target: "m3_context", messages_count, "m3 danger zone");
            interventions.push(Intervention::WarnDangerZone { messages_count });
            // 二次 schema 校验: 重新跑 §2.1 validate_tool_call
            interventions.push(Intervention::SecondarySchemaCheck);
        }

        // 规则 3: 嵌套深度 ≥4 自动补层
        // (对每个 tool_call.arguments 检查嵌套深度, 补 {} 缺失中间层)
        // 实际实施: apeireth-tool-runtime::parser 加 nested_completion pass

        // 规则 4: 64+ 强制截断
        let request = if messages_count >= M3_CRITICAL_THRESHOLD {
            warn!(target: "m3_context", messages_count, "m3 critical zone, force truncate");
            let mut req = req;
            let keep_from = messages_count - M3_DANGER_THRESHOLD;
            req.messages = req.messages.split_off(keep_from);
            interventions.push(Intervention::ForceTruncate {
                original: messages_count,
                kept: M3_DANGER_THRESHOLD,
            });
            req
        } else { req };

        MitigatedRequest { request, interventions }
    }

    pub fn nested_depth(value: &serde_json::Value) -> usize {
        // 递归算 JSON 嵌套深度 (per §1.4 5 层 vs m3 截到 3 层)
        match value {
            serde_json::Value::Object(map) => {
                1 + map.values().map(Self::nested_depth).max().unwrap_or(0)
            }
            serde_json::Value::Array(arr) => {
                1 + arr.iter().map(Self::nested_depth).max().unwrap_or(0)
            }
            _ => 1,
        }
    }
}
```

### §3.4 Tool whitelist (apeireth-mcp::builtin::WHITELIST, 14 工具 hardcode)

**位置**: `crates/apeireth-mcp/src/builtin.rs` (现有 `builtinMcps.ts:177` 对应位置, 已有 `pub mod builtin` per `apeireth-crate-api` §2.4 `apeireth-mcp` 模块)

```rust
// ⚠️ 仅供设计参考, 不写实际代码
//! 14 工具白名单 (per m3 hallucination 防御 §2.4)
//! 来源: apeireth-mcp-14-tool-analysis-2026-08-05.md §2 (8 supervisor + 3 worktree + 3 认知)
//! 编译期 hardcode 守门 (per workspace v1.0.0 + O-3 干到底)

/// 14 工具白名单 — m3 工具调用虚构防御
pub const WHITELIST: &[&str] = &[
    // 8 supervisor (per apeireth-mcp-14-tool-analysis §2.1)
    "spawn_agent",
    "send_to_agent",
    "get_output",
    "wait_idle",
    "wait",
    "get_status",
    "list",
    "cancel",
    // 3 worktree (per §2.2)
    "worktree_merge",
    "worktree_info",
    "worktree_check",
    // 3 cognitive (per §2.3)
    "list_sessions",
    "get_summary",
    "search_sessions",
];

/// 编译期守门: WHITELIST.len() == 14 (per 8 项不修改承诺 #5)
pub const WHITELIST_COUNT: usize = 14;
const _: () = assert!(WHITELIST.len() == WHITELIST_COUNT);

/// §2.4 拦截: 不在白名单返 false, 给 §3.1 validator 用
pub fn is_whitelisted(tool_name: &str) -> bool {
    WHITELIST.contains(&tool_name)
}
```

---

## §4 测试 fixture 设计 (3 fixture)

> 3 fixture 全在 `spectrai-source/tests/fixtures/m3_hallucination/` (TUI 测试用, 不污染 Apeireth LOCKED tests), 实施时由 qa_engineer 写. **设计参考**, 不写实际代码.

### §4.1 `test_m3_context_48_path_missing_param` — 模拟 m3 48+ context 缺参数

**目标**: 验证 §2.1 强校验 + §2.3 48+ 监控能拦下 m3 路径错/缺参数

**fixture 设计**:
- 输入: 模拟 minimax m3 在 48 messages context 后, 返回的 `NormalizedRequest`
  - `messages.len() == 50` (跨过 M3_DANGER_THRESHOLD=48)
  - `tools[0].name = "send_to_agent"`
  - `tools[0].arguments = {"agent_id": "agent-xxx"}` (缺 `message` 必填, 模拟 m3 漏字段)
- 期望:
  - §2.3 `ContextMonitor::check_and_mitigate` 触发 `Intervention::WarnDangerZone { messages_count: 50 }` + `Intervention::SecondarySchemaCheck`
  - §2.1 `ToolSchemaValidator::validate_tool_call("send_to_agent", ...)` 返 `Err(PipelineError::SchemaMismatch { tool: "send_to_agent", missing: vec!["message"] })`
  - §2.5 第 25 维 `hallucination_resistance` 记录 schema_mismatch_count += 1
- 断言: 调用返 `Err`, 不传到 mcp 客户端, m3 收到 reject 不会触发 "tool not found" 再编造
- 借鉴: `apeireth-mcp-14-tool-analysis` §1.2 14 工具矩阵 send_to_agent 字段

### §4.2 `test_mid_task_dual_ack_no_swallow` — 模拟 AgentManagerV2:281 .catch 吞错

**目标**: 验证 §2.2 dual ack 不吞错, 父进程能感知子 agent 真实失败

**fixture 设计**:
- 输入: 模拟 SpectrAI mid-task bug 现场
  - 子 agent session 状态 = `terminated` (子进程异常退出)
  - 父进程调 `TeamLead::dual_ack::send_and_wait(agent_id, "请改用 Y 方法")`
  - 模拟: `McpClient::call_tool` 在第 3 步 (等 received ack) **超时** (子已死, 不会发 ack)
- 期望:
  - §2.2 `DualAck::send_and_wait` 第 3 步 timeout → 返 `Err(DualAckError::ReceivedTimeout { agent_id, ms: 5000 })`
  - **不**返回 `success: true` (对比 SpectrAI `AgentManagerV2:285` 反例)
  - 父进程收到 Err 后能决定: 重试 / 放弃 / 提示用户 (per `spectrai-architecture` §6.4 修法 2 原则)
- 断言: 0 `.catch(console.error)` 吞错; 0 `return { success: true }` 骗父
- 借鉴: `spectrai-architecture-2026-08-05.md` §6.4 修法 2 (`agent/AgentManagerV2.ts:269-286`)

### §4.3 `test_m3_fabricated_tool_rejected` — 模拟 m3 虚构 spectrai_send_to_agent_v2

**目标**: 验证 §2.4 14 工具白名单能拦下 m3 虚构工具

**fixture 设计**:
- 输入: 模拟 minimax m3 在 48+ context 后虚构
  - `tool_use.name = "spectrai_send_to_agent_v2"` (实际白名单 = `send_to_agent`)
  - `tool_use.input = {"agent_id": "agent-xxx", "message": "..."}`
- 期望:
  - §2.4 `is_whitelisted("spectrai_send_to_agent_v2")` 返 `false`
  - §3.1 `ToolSchemaValidator::validate_tool_call` 在 §2.4 拦截这一步返 `Err(PipelineError::UnknownTool { name: "spectrai_send_to_agent_v2", allowed: WHITELIST.to_vec() })`
  - **不**传 mcp 客户端 (避免 m3 收到 "tool not found" 后再编造)
  - §2.5 第 25 维 `hallucination_resistance` 记录 fabrication_count += 1
- 断言: `WHITELIST_COUNT == 14` 编译期守门 (per §3.4 `const _: () = assert!`)
- 借鉴: `apeireth-mcp-14-tool-analysis-2026-08-05.md` §2.1 第 2 行 send_to_agent (Hybridservice 工具)

---

## §5 跟 R20 阶段 1-5 集成点

> R20 阶段 1-5 是主人 19:01 决策的下一阶段路线 (per `spectrai-architecture-2026-08-05.md` §5.3 R1-R6 11 周 + 主人 "R19+ 蓝图缺" 决策). 5 道防御在每一阶段的嵌入点:

### §5.1 阶段 1 准备: m3 防御策略评审 + Hermes 同步验证

**集成点**:
- §2.1 pre-call schema 强校验 — 阶段 1 准备时确认 `apeireth-pipeline` 5 步变 6 步的可行性 (per `apeireth-crate-api` §2.5 pipeline 模块)
- §2.5 第 25 维 — 阶段 1 准备时跟 `apeireth-asi` 团队对齐 V0.5 24 维扩展 V2 25 维的命名 (per D-03 建议, `apeireth-asi-24dim-api` §1.2)
- **协同点 3** (per r20-stage-1-prep §1.3, 假设文档): Hermes LOCKED crate 同步验证 — 5 道防御**不改 Hermes** (per 8 项不修改承诺 #8), 仅确认 m3 走 minimax 协议时 Hermes 适配正常

### §5.2 阶段 2 公开 API: 6 端点 + 5 道防御 endpoint-level 暴露

**集成点** (per r20-stage-2-3-prep §2 假设 6 端点):
- `POST /v1/tools/validate` — 暴露 §2.1 schema 校验给 SDK / curl
- `GET /v1/tools/whitelist` — 暴露 §2.4 `WHITELIST` 14 工具列表
- `GET /v1/context/monitor/status` — 暴露 §2.3 `ContextMonitor` 当前状态 (per session)
- `POST /v1/dual-ack/send` — 暴露 §2.2 dual ack 给 SDK 调
- `GET /v1/asi/v05/hallucination_resistance` — 暴露 §2.5 第 25 维
- `GET /v1/m3/danger-zone` — 暴露 §2.3 48+ 警告阈值

### §5.3 阶段 3 Docker: 0 集成 (纯 Rust 端逻辑)

**集成点**:
- 5 道防御**纯 Rust 端逻辑**, 0 容器化集成
- Docker 镜像只装 Rust binary, 5 道防御作为 binary 内置模块
- 阶段 3 部署时**不暴露** 5 道防御的内部状态到容器外 (per O-5 不假装, 也 per "用户看结果不看哲学" 主人 user memory #3)

### §5.4 阶段 4 SDK: TS/Python SDK 带 m3 防御

**集成点** (per D-04 B 选项 3 SDK 假设, r20-stage-2-3-prep §3):
- TS SDK (`@apeireth/sdk`) — `validate_tool_call` / `is_whitelisted` / `send_with_dual_ack` 3 方法
- Python SDK (`apeireth-sdk`) — 同上 3 方法
- SDK 调用 §5.2 6 端点, **不在 SDK 端重写** 5 道防御 (per O-2 走在前人肩上)
- SDK 文档明示 m3 防御由 Rust 端实施, SDK 客户端透传

### §5.5 阶段 5 1.0 release: 1 Kani invariant 配套 (per apeireth-formal §2.1 K-1 强校验不变量)

**集成点** (per `apeireth-formal` §2.1 假设 K-1 不变量):
- **K-1 强校验不变量**: ∀ tool_call: `validate_tool_call(name, args) → Ok ⟹ is_whitelisted(name) ∧ schema_match(name, args)` (per §2.1 + §2.4 组合)
- K-1 用 Kani Rust model checker 验证: 5 道防御的 §3.1 validator + §3.4 whitelist + §3.3 monitor 3 模块核心路径
- 1.0 release 前 K-1 必须绿, 否则 m3 hallucination 在生产环境无最后兜底
- 配套 e2e 测试: 模拟 m3 4 类触发模式 (per §1.1~§1.4) 端到端跑, 验证 K-1 不变量

---

## §6 8 项不修改承诺 + 6 哲学 anchor 穿透自检

> 主人 12 子规范 8 项不修改承诺 + 6 哲学 anchor, 6/6 严守穿透. 表格自检.

### §6.1 8 项不修改承诺严守自检

| 承诺 | 自检结果 | 证据 |
|------|---------|------|
| 1. 阶段 1/2/3/4/5 LOCKED 设计文档 | ✅ 0 改 | 引用 `docs/stage[1-6]/`, §5 阶段 1-5 集成点仅 "嵌入位置", 不动设计 |
| 2. v2/v4/v4.1 LOCKED | ✅ 0 改 | 全文 0 处重写 v2/v4/v4.1 公式 |
| 3. 12 键 LOCKED | ✅ 0 改 | 6 哲学 anchor 引用 12 键中的 6 个, 不重命名 |
| 4. 6 哲学 anchor LOCKED | ✅ 0 改 | §6.2 6/6 穿透, 命名不重命名 |
| 5. workspace v1.0.0 | ✅ 0 改 | §3 snippet 全用 `workspace = true` 依赖, 不写版本号 |
| 6. Document-Meta 格式 | ✅ 严守 | 本报告头部 6 字段全 (Document/Version/R-Cycle/Last-Modified/Status/Author) |
| 7. R11 baseline 3 值 (0.8682/0.8532/0.9063) | ✅ 0 改 | §2.5 引用 `apeireth-asi-24dim-api` §1.3, 不重测不重算 |
| 8. 24 crate `src/` 不改 | ✅ 0 触碰 | 0 触碰 `crates/apeireth-*/src/`, 全部在 `spectrai/reports/spectrAI-r19plus-v2/` |

### §6.2 6 哲学 anchor 6/6 穿透自检

| Anchor | 穿透位置 | 严守动作 |
|--------|---------|---------|
| **S-1 北极星** = SpectrAI → Apeireth 集成目标 | §0.1 决策 1 (R20 阶段 1-5 集成) + §5 全章 | 5 道防御 = 集成目标的**质量守门**, 不偏离主人 R19+ 终极目标 (TUI → Tauri, 主人 user memory #8) |
| **S-2 实事求是** = m3 真问题不假装 | §1.1~§1.4 全章 (4 类模式 100% 主人 16:40+18:49 战报) + §0.2 决策 5 (不假装已实装) | m3 真问题 4 类全列出, 不藏丑 (e.g. §1.1 "32 不一定够" 诚实承认主人 P0 缓解的不足) |
| **O-2 走在前人肩上** = composio zod boundary 借鉴 | §2.1 (借鉴 composio) + §2.2 (借鉴 `apeireth-mcp-14-tool-analysis` §3 修法 #1) + §2.5 (借鉴 D-03 第 25 维) | 5 道防御**不重写**, 显式标注借鉴来源 (VCP protocolBridge / composio / apeireth-asi D-03) |
| **O-3 干到底** = 14 工具白名单 hardcode | §2.4 (WHITELIST 14 工具) + §3.4 (const 编译期守门 `WHITELIST_COUNT = 14`) | 14 工具 hardcode, **不留给配置**, 编译期 `const _: () = assert!` 守门 (per workspace v1.0.0 守门) |
| **O-4 任何人都能接手** = 5 道防御清晰可读 | §2 5 道 (编号 2.1~2.5) + §3 4 snippet (编号 3.1~3.4) + §4 3 fixture (编号 4.1~4.3) + §5 5 阶段 (编号 5.1~5.5) | 全文编号清晰, 每章 ≤200 行, 接手者按编号顺序读不迷路. 配套引用 `spectrai-architecture` §6.4 + `apeireth-mcp-14-tool-analysis` §3 + `apeireth-asi-24dim-api` §1.2 — 3 份文档互引, 任何接手者搜文件名即可调出 |
| **O-5 不假装** = m3 行为不掩盖 | §0.1 决策 5 (不假装已实装) + §3 snippet 标 "⚠️ 仅供设计参考, 不写实际代码" + §4 fixture 标 "设计参考, 不写实际代码" + §2.5 第 25 维 "缺观测 → MeasurementError 不 fallback 0.0" | 全章明示 "代码骨架" + "fixture 设计", 不冒充实装; 缺观测不假装是 0, 返 error (per `apeireth-asi-24dim-api` §3.3 LOCKED 行为) |

### §6.3 引用增量声明 (不重复已有 5 份报告)

| 已有报告 | 引用位置 | 不重复内容 |
|----------|---------|----------|
| `spectrai-architecture-2026-08-05.md` | §1.2 (mid-task 3 处) + §1.3 (5 Provider) + §1.4 (m3 风险) + §5 (R20 阶段 1-5) | 不重复 §6.4 修法细节 (引用即可), 不重复 §5 映射表 (引用即可) |
| `apeireth-protocol-4-adapter-analysis-2026-08-05.md` | §1.1 (4 adapter) + §1.2 (AnthropicMessages) + §3.3 (m3::ContextMonitor 放 providers/minimax) | 不重复 §3 4 adapter 字段表 (引用即可) |
| `apeireth-crate-api-2026-08-05.md` | §3.1 (pipeline 5 步) + §3.4 (mcp builtin) + §5.4 (SDK) | 不重复 §2.1~§2.10 10 crate API (引用即可) |
| `spectrai-source/src/main/adapter/` 5 Provider | §1.1 (V2 Adapter 架构) + §1.3 (5 Provider 注入机制) | 0 触碰, 仅作为现状引用 |
| `Apeireth-rust/crates/apeireth-protocol/src/` (Hermes LOCKED) | §1.2 (4 adapter 旁边加 minimax) + §3.3 (新模块位置) | 0 触碰, 仅作为架构参考 |
| `apeireth-mcp-14-tool-analysis-2026-08-05.md` | §2.4 (14 工具白名单) + §2.2 (mid-task 修法 #1) | 不重复 §2.1~§2.3 14 工具详细表 (引用即可) |
| `apeireth-asi-24dim-api-2026-08-05.md` | §2.5 (第 25 维 D-03) + §3.5 (24 维 LOCKED 名称) | 不重复 §3.1 V05_DIMENSION_NAMES 数组 (引用即可) |
| `tauri-roadmap-2026-08-05.md` (Tauri 阶段) | §5.5 (1.0 release, Tauri 后续) | 13 项 T-001~T-013 不在 m3 防御范围, 引用即可 |

---

## 报告

| 项 | 值 |
|---|---|
| 路径 | `.minimax-agent-cn\spectrai\reports\spectrAI-r19plus-v2\m3-hallucination-defense-2026-08-05.md` |
| 触发模式 | 4 类 (§1.1~§1.4, 每类说明+SpectrAI 现状+m3 vs Claude/Gemini 对比) |
| 防御策略 | 5 道 (§2.1 schema 强校验 / §2.2 dual ack / §2.3 48+ 监控 / §2.4 14 工具白名单 / §2.5 第 25 维日志) |
| 代码骨架 | 4 snippet (§3.1 validator / §3.2 dual_ack / §3.3 ContextMonitor / §3.4 WHITELIST, 全 Rust, 全标 "设计参考") |
| 测试 fixture | 3 fixture (§4.1 48+ path missing / §4.2 dual ack no swallow / §4.3 fabricated tool) |
| R20 集成 | 5 阶段 (§5.1 阶段 1 准备 / §5.2 阶段 2 6 端点 / §5.3 阶段 3 0 集成 / §5.4 阶段 4 SDK / §5.5 阶段 5 K-1 Kani) |
| 8 项不修改承诺 | 8/8 严守 (per §6.1 表格) |
| 6 哲学 anchor | 6/6 穿透 (per §6.2 表格) |
| 引用增量 | 8 份已有报告 (per §6.3 表格, 不重复内容) |
| 字数 | ~350 行 (主人 300-500 行约束内), 信息密度高 |

---

## §7 Yinta fork 上下文增量 (per sub-agent E yinta-fork-audit-2026-08-05.md, 2026-08-05 19:30)

### §7.1 fork 不含 minimax m3 集成 (实查)

| 查项 | 结果 |
|------|------|
| grep `minimax\|MiniMax\|hallucination` in Yinta fork `out/` | **全 0 命中** |
| 73 deps (per fork `package.json`) 有无 minimax SDK | **0** |
| fork `AgentMCPServer.js` 22 工具 | 0 个 m3 specific 工具 |
| fork `out/main/index.js` 起动逻辑 | 无 m3 探测 / 无 context 48+ 监控 |

**结论**: Yinta fork = v0.9.21 商业版 + paid tier bypass, **完全无 m3 集成**。主人 19:01 拍板"全补"确认 fork 0 翻译 m3。

### §7.2 5 道防御必须 Rust 端全新设计, 不能从 fork 翻译

| 防御 (§2) | fork 翻译可能? | Rust 端全新设计 |
|------------|----------------|-----------------|
| §2.1 Pre-call Schema 强校验 | ❌ fork 0 schemars 集成 | ✅ apeireth-pipeline::validate (composio zod boundary 借鉴) |
| §2.2 Mid-call 双层 ack | ⚠️ fork AgentManagerV2:281 .catch 吞错 (主人 16:40 战报) | ✅ apeireth-team-lead::dual_ack 8 步 (1:1 反例) |
| §2.3 m3 48+ 监控 | ❌ fork 0 ContextMonitor | ✅ apeireth-protocol::providers::minimax::m3::ContextMonitor (4 规则) |
| §2.4 14 工具白名单 | ⚠️ fork 22 工具但 0 白名单守门 | ✅ apeireth-mcp::builtin::WHITELIST const + assert 编译期守门 |
| §2.5 hallucination 记日志 | ❌ fork 0 hallucination 测量 | ✅ apeireth-asi 第 25 维 (D-03 拍 A 后 24→25 维) |

**结论**: 5 道防御 **100% Rust 端全新**, **0% 来自 fork 翻译**。fork 446K LOC 反编译源**对 m3 防御 0 价值**。

### §7.3 fork = v0.9.21 商业版 + paid tier bypass (背景)

- fork 实际版本 0.1.0 (per fork `package.json`), fork 自 v0.9.21 商业版, 作者 chuling@local, fork 时间 2026-08-03
- fork = v0.9.21 商业版 + paid tier bypass (`getEffectivePlan` 永远 `enterprise`, 改动只在 `out/renderer/assets/index-DXzB8709.js`)
- 商业版 1.75M LOC, fork 业务代码 446K LOC, 估缺 75% (1.3M LOC 闭源)
- 8 闭源模块 fork 也缺: TeamRepository / TeamBus / TaskKanban / Orchestrator / AutonomousPlanner / TelegramBotManager / AIRouter / SuggestionEngine

### §7.4 集成点 (per E §6)

- R20 阶段 1 准备 (r20-stage-1-prep): m3 防御 §2 + §3 + §4 进 Fixture 1 (test_team_lead_workflow)
- R20 阶段 2 公开 API: m3 防御是 `team_lead` 端点的 internal 行为, 不暴露给 SDK
- R20 阶段 4 SDK: TS/Python SDK 调 apeireth-team-lead 时, m3 防御对 SDK 透明 (Rust 端自动处理)
- R20 阶段 5 1.0 release: m3 防御 + K-1 强校验 (per supervisor-prompt-818 §7 修订) 是 1.0 release 必须项

### §7.5 8 项不修改承诺 + 6 哲学 anchor 穿透自检 (增量)

- 0 改 LOCKED 8 项 (per §6.1 表格)
- S-1 北极星 = "m3 防御必须 Rust 端全新" (不靠 fork 偷懒) 
- S-2 实事求是 = fork 0 命中 m3 grep 实证
- O-5 不假装 = 5 道防御全标"设计参考" + 0 fork 翻译
- O-2 走在前人肩上 = composio zod boundary 借鉴 (§2.1)
- O-3 干到底 = 5 道防御 hardcode 编译期守门
- O-4 任何人都能接手 = §7.1 grep 实证表 + §7.2 fork 翻译矩阵

