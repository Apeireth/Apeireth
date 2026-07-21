# Round-21 真源码 cookbook 深读

> 调研者: 量化信息官 subagent @ [Tue 2026-07-21 11:39 GMT+8]
> 主题: anthropic-cookbook + openai-cookbook + awesome-ai-agents 真生产 patterns
> 注: 网络不稳, anthropic-cookbook 21.7MB 部分截断 (truncated tar archive), openai-cookbook 22.5MB 部分截断; 但关键源码 (chief_of_staff_agent / SRE / obs_agent + multi-agent-portfolio) 均已成功提取. awesome-ai-agents 仅 README.md (215 个项目清单).

---

## anthropic-cookbook — 真生产 patterns

**核心结构**: `claude_agent_sdk/` (5 个 agent: chief_of_staff / research / observability / sre / vulnerability_detection) + `capabilities/` (RAG / summarization / text_to_sql / classification / KG) + `hosting/` (Docker / k8s / Modal) + `finetuning/`.

### 1. Claude Agent SDK 多 agent 框架 (claude_agent_sdk/chief_of_staff_agent/agent.py)
**模式**: `ClaudeAgentOptions` 配置中心化 — `model / allowed_tools / continue_conversation / system_prompt / permission_mode / cwd / settings / setting_sources`. 一个 `send_query()` 抽象 + 一套 `activity_handler (sync 或 async)` 钩子. 通过 `setting_sources=["project", "local"]` 自动加载 CLAUDE.md / 命令 / 子 agent / hooks. 推荐借鉴到 Apeireth 的 tool_runner.py — 把工具清单 + 提示缓存 + 拒绝工具 (restrict_to_mcp) 集中到一个配置类.

### 2. `disallowed_tools` 强制路径 (observability_agent/agent.py)
**关键代码**:
```python
disallowed_tools = ["Bash", "Task", "WebSearch", "WebFetch"] if restrict_to_mcp else []
```
不删工具, 而是 *禁用* 绕过工具 (Bash → 用 `gh` CLI 绕过 MCP). 这是和 BetaToolRunner "3 防御" 同源的: prompt 层 + 配置层 (allowed_tools 白名单) + 配置层 (disallowed_tools 黑名单). **强烈推荐**.

### 3. RAG 三级召回 (capabilities/retrieval_augmented_generation/evaluation/prompts.py)
- `_retrieve_base`: 向量召回 top-3
- `_retrieve_level_two`: 每个 chunk 加 `chunk_heading + summary`, 上下文窗口更大
- `_retrieve_level_three`: `_rerank_results` 用 Claude Haiku 对 top-20 重排, 取 top-5; 重排 prompt 使用 XML 标签 `<relevant_indices>1,3,5</relevant_indices>` + `stop_sequences=["</relevant_indices>"]` — 这就是确定的 XML bounded-output pattern.
- 全部 prompt 使用 `<query>...</query>` 和 `<documents>...</documents>` 标签包围, 让 Claude 100% 忠实 context.

### 4. 上下文 XML tag 输出 (capabilities/text_to_sql/evaluation/prompts.py)
**关键**: `Please provide your answer within <sql>...</sql> tags` + CoT `<thought_process>...</thought_process>` 分离推理 / 输出. 这种 tag 边界输出比 JSON 解析更鲁棒, 推荐作为 Apeireth structured output 的默认格式 (尤其在没有 Pydantic 的场景).

### 5. Hosting SSE 流 + 双向 session map (claude_agent_sdk/hosting/server.py)
- 外部 `session_id → SDK session_id` 持久映射 (atomic temp-file + replace)
- `secrets.compare_digest` 防 timing-attack bearer token
- `_limit_body_size` 中间件先于 FastAPI JSON parser (256KB 上限)
- 注释明确标 "Note: … fine for this single-caller-per-session shape; a production server would lock around the read-create-write span, not just the write" — 真生产才有的 "knowing limits" 注释文化.

### 6. Activity subagent 嵌套深度追踪 (claude_agent_sdk/utils/agent_visualizer.py)
- 全局 `_subagent_context: dict = {active, name, depth}` 显式表示 "当前在第几层子 agent"
- API 注释明确写道: `WARNING: This global state is NOT thread-safe...` (设计知情)
- 推荐借鉴: Apeireth 日志/可观测性应有 depth-aware 缩进, 不只平铺.

### 7. SRE MCP server + hooks 防御 (claude_agent_sdk/site_reliability_agent/)
- 完整 infra: postgres + api-server + prometheus + grafana + traffic-generator
- "Known-good backup" 文件 (config/api-server.env.backup), agent 修复时可回退
- `.claude/hooks` 目录: 安全钩子在 write 前校验
- 这是 *agent + checkpoint/restore + observability* 三件套, 和 DGM 自我演化的"演化-评估-回滚"一脉相承.

---

## openai-cookbook — 真生产 patterns

**核心结构**: `examples/agents_sdk/` (5 个真生产例子: multi-agent-portfolio, sandboxed-code-migration, computer_use_with_daytona, deployment_manager, migrate-from-claude-agent-sdk) + `articles/` (10 篇, 含 techniques_to_improve_reliability, openai-harmony) + `agents_sdk/` Python 包.

### 1. `tools.py` + `@function_tool(failure_error_function=...)` (multi-agent-portfolio/tools.py)
```python
def code_interpreter_error_handler(ctx, error):
    return ("Error running code interpreter. ... Details: " + str(error))
@function_tool(failure_error_function=code_interpreter_error_handler)
def run_code_interpreter(request: str, input_files: list[str]) -> str:
    if not request or not isinstance(request, str):
        raise ValueError(...)
```
**关键**: 函数级 `failure_error_function` 把异常转换为 *对 LLM 友好的指导字符串*, LLM 下次调用就知道怎么修. 这就是 BetaToolRunner "3 防御" 的 *self-correcting error feedback*. **强烈推荐借鉴**.

### 2. `tool_retry_prompt.md` (multi-agent-portfolio/prompts/tool_retry_prompt.md)
完整防御 prompt:
```
If a tool call fails due to an authentication or server error (such as a 500 Internal Server Error,
or 4XX errors), timeout, or network issue, you MUST retry the same tool call up to 2 more times
before giving up. If the tool call still fails after 3 total attempts, report the error in your output.
```
**关键**: retry 上限 (3 次) + 显式 fallback 行为 + example 段落 (3 重试都失败 → "Tool call failed after 3 attempts: …"). 这就是 prompt-level retry policy. 推荐作为 Apeireth 默认 tool_runner prompt 注入.

### 3. Multi-agent portfolio (pm.py + editor.py + utils.py)
- 5 subagents: fundamental / macro / quant / pm / editor
- `run_all_specialists_parallel` 用 `asyncio.gather` 并发 3 个 specialist (减少 wall-clock 3x)
- Pydantic `MemoEditorInput(BaseModel)` 强类型工具输入校验 (替代 JSON schema dict)
- `ModelSettings(temperature=0, parallel_tool_calls=True, tool_choice="auto")` 显式生产配置
- `load_prompt(name)` 从 `prompts/*.md` 加载 + `<PLACEHOLDER>` 模板替换 (md-as-prompt 比 f-string 更可 diff)
- `DISCLAIMER = "I am an AI language model, ..."` 全局注入
- `FileSpanExporter(TracingExporter)` 把 trace JSONL 落盘 — observability.

### 4. Pydantic function tool + structured output (openai-python/examples/parsing_tools.py)
```python
class Query(BaseModel):
    table_name: Table  # enum
    columns: List[Column]
    conditions: List[Condition]
    order_by: OrderBy
completion = client.chat.completions.parse(model="gpt-4o-...", tools=[openai.pydantic_function_tool(Query)])
tool_call.function.parsed_arguments  # 已经 validate + parse
```
**关键**: `chat.completions.parse` + Pydantic enum 复合类型 + `pydantic_function_tool` 一次生成 → 不需要手工 retry parse. 这就是 OpenAI 的 native structured output pattern.

### 5. Reliability techniques (articles/techniques_to_improve_reliability.md)
6 条实践:
1. 更清晰的指令
2. 拆解复杂任务为子任务
3. 结构化指令 (XML tags / YAML / numbered steps)
4. 让模型先解释再回答 (CoT)
5. 让模型给多个候选 + 自评 (self-consistency)
6. 多输出 + 自选 best-of-N

### 6. Deployment manager (deployment_manager/app/runner.py)
- docker + local-process 双模式, 共用 Docker 部署原语
- `_port_is_open` pre-flight 防冲突
- `_seed_data_dir` 一次性 copy
- 完整 env propagation (`AGENTS_SDK_DEPLOYMENT_ID`, label `agents-sdk.manager=true`)
- refresh_status 解析 → status machine: pending → running → stopped
- subprocess.Popen / docker run 的 timeout + log capture 一致

### 7. AGENTS.md 自身 → 教程化 (openai-cookbook/AGENTS.md)
P0 评审规则:
- 描述性命名 (no placeholder names)
- secrets 必须 env var
- 长模拟不要让 row-level grade 偏向
- 默认 harness 是 crawl, walk 留给音频

---

## awesome-ai-agents — 真生产 agent 分类

**结构**: 单 README.md 5591 行, **215 个项目** (130 开源 + 85 闭源), 按 `### Category` 标签细分.

### 项目计数 (Top 10)
| Category | 数量 |
|---|---|
| Coding | 41 |
| General purpose | 25 |
| Productivity | 6 |
| Build-your-own | 6 |
| General purpose + Build your own + Multi-agent | 6 |
| General purpose + Productivity | 5 |
| Coding + general purpose | 5 |
| Coding + Data analysis | 4 |
| Coding + GitHub | 4 |
| Coding + Multi-agent | 3 |

### Top 项目 (按 Apeireth 关注度)
**Multi-agent**: AgentVerse, Agents (aiwaves), AI Legion, CAMEL, ChatArena, ChatDev, CrewAI, AgentForge, Multiagent Debate, AutoGen, MetaGPT, GPTSwarm, Agent4Rec, OpenAgents
**Memory-first**: MemGPT, BabyAGI 家族 (Bee/Cat/Deer/Elf/Command/Fox — 6 个变种!), Letta (前 MemGPT)
**Coding**: Aider, Continue, Cursor, Cody, Mentat, Devin, DevGPT, GPT Pilot, OpenDevin
**Tool Use**: SWE Agent (Princeton-NLP), UFO (Microsoft), Langroid, smol-developer
**Self-evolution**: Voyager (Minedojo, 自演化 prompt curriculum), Suspicion Agent
**Production-grade flowise / LangChain**: Langroid, LLM Stack, FastAgency, Flowise, BotDojo, Aomni
**Real-world**: Lindy, Bardeen, MultiOn, Cursor, Devin, Clay, Hex Magic, Zapier Central

### 真生产 patterns (按 agent 类型)
1. **Multi-agent supervisor + worker**: AutoGen, CrewAI (单 supervisor 分发 task 到 N 个 worker)
2. **SOP (Standard Operating Procedure)**: Agents (aiwaves-cn) — 用户定义 SOP 子任务序列, 类似 YAML workflow
3. **Controller LLM 动态决策**: Agents (aiwaves-cn) — controller 在每个 step 决定下一步哪个 agent 跑 (替代 pre-defined sequence)
4. **Long-short memory hybrid**: Agents (aiwaves-cn), MemGPT — STM (LLM-context) + LTM (向量 DB + semantic search)
5. **Reflection / autonomous learning**: Adala (HumanSignal) — "evolve through observations and reflections, not just automation"
6. **Sandbox integration / native support**: e2b (per repo's bottom section) — 60+ 项目有 sandbox integration

---

## Apeireth 可借鉴的具体真生产 patterns (表格)

| Cookbook Pattern | 来源 | 适用 Apeireth 模块 | 借鉴度 | 理由 |
|---|---|---|---|---|
| `ClaudeAgentOptions` 配置中心化 | anthropic SRE | tool_runner.py | 高 | 集中 allowed/disallowed/tools, 减少散落 |
| `disallowed_tools` 黑名单 | anthropic observability | tool_runner.py (BetaToolRunner 3 防御扩展) | **极高** | 第三方一致; 防 bypass |
| `failure_error_function` 友好错误 | openai multi-agent | tool_runner.py | **极高** | 自纠正反馈; 比 raise 直接返回 LLM |
| `tool_retry_prompt.md` 重试 policy | openai multi-agent | tool_runner.py 注入 | 高 | 明确 retry 上限 + fallback |
| Tag-bounded XML output (`<sql>`/`<thought_process>`) | anthropic text_to_sql | llm_kernel.py (call_llm parser) | 高 | 替代 JSON parse, 更鲁棒 |
| RAG 三级检索 + XML rerank | anthropic RAG | memory_3tier.py (LTM retrieval) | 中 | tier-3 rerank 太重, 仅当 LTM 召回质量差时启用 |
| Pydantic `chat.completions.parse` | openai parsing | llm_kernel.py (call_llm) | **极高** | 原生结构化输出, 无需 retry parse |
| `asyncio.gather` 并发 specialists | openai multi-agent | self_evolving.py (population evaluation) | 高 | 并行评估多 candidate |
| Multi-tier memory (LTM/STM) | awesome-ai-agents (MemGPT / Agents) | memory_3tier.py | 中-高 | 已有结构 → 借鉴命名一致性 + load-strategy |
| SOP workflow yaml | awesome-ai-agents (aiwaves Agents) | self_evolving.py (演化策略声明) | 中 | 把 DGM-style 演化 SOP 外化为 yaml |
| `_port_is_open` pre-flight | openai deployment_manager | tool_runner.py (server 启动) | 低 | 当前无 server, 暂无需求 |
| `setting_sources=["project", "local"]` 自动加载 CLAUDE.md | anthropic chief_of_staff | memory_3tier.py (STM context 加载) | 中 | 参考, 不直接模仿 |
| `_seed_data_dir` 一次性 copy | openai deployment_manager | memory_3tier.py (compile 时数据快照) | 低 | 当前无 |
| Failure mode annotation 文化 | anthropic server.py / agent_visualizer.py | 全模块 | **极高** | "knowing limits" 注释是好实践 |

---

## 推荐 / 不推荐 (具体到每个 pattern)

### 强烈推荐 (Round-21 之后立即落地)

1. **`disallowed_tools` 黑名单** (anthropic observability_agent/agent.py) → 纳入 tool_runner.py. BetaToolRunner 已有 prompt 防御, 加一个配置层 disallowed 比删工具更灵活.

2. **`failure_error_function` 友好错误** (openai multi-agent/tools.py) → tool_runner.py 装饰器. 把 `raise` 转换为 *对 LLM 友好的指导字符串* (含 "what you must provide"), 让 LLM 下次自己改对.

3. **`tool_retry_prompt.md` 重试 prompt 注入** (openai multi-agent/prompts/) → tool_runner.py 默认注入. retry ≤ 3 + 显式 fallback.

4. **Pydantic `chat.completions.parse`** (openai-python/examples/parsing_tools.py) → llm_kernel.py call_llm 主路径 (当需要结构化输出时). 替代手工 json.loads + retry.

5. **`<xml-tag>` 边界输出** (anthropic text_to_sql) → call_llm 默认结构化输出格式 (在没有 Pydantic 的次要场景, 或作为 Pydantic 的 fallback).

6. **`asyncio.gather` 并发** (openai multi-agent/pm.py) → self_evolving.py 的 population evaluation 并行.

7. **failure-mode 注释文化** (anthropic server.py) → 全模块. `# NOTE: ...` 标注限制/边界条件.

### 中度推荐 (条件性落地)

8. **Multi-tier memory** (MemGPT / Agents-aiwaves) → memory_3tier.py 已部分有, 借鉴命名一致性.

9. **RAG tier-3 rerank** (anthropic RAG) → 仅当 LTM 召回质量差时启用 (成本高).

10. **SOP yaml workflow** (aiwaves Agents) → self_evolving.py (DGM 演化 SOP 外化).

11. **Subagent depth tracking** (anthropic agent_visualizer.py) → 日志可观测性 (depth-aware 缩进).

12. **`asyncio.gather` + Pydantic input validation** (openai editor.py) → tool_runner.py 强类型化 Pydantic input.

### 不推荐 (当前阶段避免)

A. **完整 SSE + FastAPI + Docker + k8s + Modal (anthropic hosting/)** — 当前 Apeireth 是 in-process library, 不需要 server 形态. **避开** 整条路径.

B. **重为 standalone agent product (awesome-ai-agents 闭源区)** — 80+ 闭源 agent 全是 GUI / IDE / 商业产品方向, 不是 library 方向. 仅做 land-and-learn.

C. **Cursor / Cody 完整 IDE agent (awesome-ai-agents coding)** — 太具象化 (VSCode extension), 借不到通用模式.

D. **OpenAI `McpServerConfig` 完整形态** — anthropic 用 MCP 时附带 docker pull + GITHUB_TOKEN, 太重; 当前 tool_runner 自己的工具集足够.

E. **OpenAI Harmony (openai-harmony article)** — gpt-oss 专属, 不适用.

F. **Awesome-ai-agents "Build-your-own / no-code" 产品** — Langroid / Flowise 是另一条 (low-code 平台), 与 Apeireth (in-process Python) 方向不同.

---

## 5-10 行关键发现摘要

1. **anthropic-cookbook 真实生产 SDK patterns**: `ClaudeAgentOptions` 配置中心化 + `disallowed_tools` 黑名单防 bypass + tag-bounded XML 输出 (`<sql>...</sql>` / `<relevant_indices>`) + RAG 三级召回 (vector + summary + LLM rerank). 

2. **openai-cookbook 真实生产 agent SDK patterns**: `@function_tool(failure_error_function=...)` 自纠正错误反馈 + `tool_retry_prompt.md` retry-policy prompt 注入 + `asyncio.gather` 并发 specialists + `ModelSettings(temperature=0, parallel_tool_calls=True)` 显式生产配置 + `chat.completions.parse` + Pydantic 原生结构化输出.

3. **awesome-ai-agents 给的全局视野**: 215 项目里 Coding 41 / General 25 / Productivity 6 是主战场; MemGPT / BabyAGI 家族 / Adala 反映 memory + reflection 是 AGI 长期路径; e2b 是 sandbox 生态的事实标准.

4. **Apeireth tool_runner.py 高优先级借鉴 (按主 9:15 修好优先)**: disallowed_tools 黑名单 + failure_error_function 自纠正反馈 + tool_retry_prompt 注入 + Pydantic input 校验 (4 件套).

5. **Apeireth llm_kernel.py 高优先级借鉴**: Pydantic `chat.completions.parse` 原生结构化输出 (替代 retry parse JSON) + tag-bounded XML fallback 输出.

6. **Apeireth self_evolving.py 借鉴**: `asyncio.gather` 并发 evaluation + SOP yaml workflow.

7. **不推荐**: 整条 anthropic `hosting/` SSE + FastAPI + Docker + k8s (与 Apeireth library 形态不匹配), OpenAI Harmony (gpt-oss 专属), 闭源 IDE agent 整条产品线 (无通用模式).

8. **文化级借鉴**: anthropic server.py 和 agent_visualizer.py 的 *failure-mode annotation* (明确写出 "this works because ... but breaks when X") — 全模块推荐.

9. **网络受限的妥协**: openai-cookbook 22.5MB 部分截断 (tar archive truncated); 仅 multi-agent-portfolio 和 deployment_manager 关键源码已提取, articles/techniques_to_improve_reliability.md 完整; agents_sdk 部分还可能缺 agentkit_walkthrough 部分.

10. **下一轮建议 (Round-22)**: 把 openai-cookbook `examples/agents_sdk/multi-agent-portfolio` 完整克隆 (Pydantic input validation + asyncio.gather + agent_tool wrapping patterns), 再深读 `articles/openai-harmony.md` (但仅作为 Anthropic skills 对标参考).
