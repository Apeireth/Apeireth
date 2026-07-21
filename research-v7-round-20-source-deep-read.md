# Round-20 真源码深读 — langgraph / anthropic-sdk / openai-python

> 主题: 主 23:28 真研究哲学 + 主 22:33 自决 + 主 10:54 跨域. 三仓库 `--depth 1` clone 完成, 真读核心 src 文件不止 README. 实事求是: 推荐 / 不推荐都给理由. 避开 round-13/14/15/16/17/18/19 已覆盖 (claude-mem/letta/mem0 等), 这次聚焦 langgraph state machine + 两家 LLM SDK 真生产 API 借鉴.

---

## langgraph (stateful agents) — 代码层细节

**核心文件**: `libs/langgraph/langgraph/channels/{base,binop,last_value,topic,delta,ephemeral_value,named_barrier_value}.py` (state container 原语), `libs/langgraph/langgraph/graph/state.py` (StateGraph builder), `libs/langgraph/langgraph/pregel/_loop.py` + `_algo.py` + `_runner.py` (superstep engine), `libs/checkpoint/langgraph/checkpoint/base/__init__.py` (持久化接口).

### 数据结构 — Channel + Checkpoint 双层抽象

1. **`BaseChannel[Value, Update, Checkpoint]`** (channels/base.py) = state 容器原语. 单一接口: `update(values: Sequence[Update]) -> bool` / `get() -> Value` / `checkpoint() -> Checkpoint` / `from_checkpoint() -> Self` / `consume() -> bool` / `finish() -> bool`. 所有 state 操作都收敛到这 6 个方法, superstep engine 不直接操作 state — 这是 PG (Pregel) 模型的精髓.
2. **5 个内置 channel 策略** (libs/langgraph/langgraph/channels/):
   - `LastValue` — 一值一写, `update()` 拒绝多个 value per step (抛 `InvalidUpdateError`), 适合 final 状态
   - `BinaryOperatorAggregate(typ, operator)` — reducer channel `(Value, Value) -> Value`, 把 `Annotated[list, operator.add]` 编译成这个. 还内建了 `Overwrite` sentinel, 支持 JSON 跨边界序列化 (`{"type": "__overwrite__", "value": ...}`)
   - `Topic(typ, accumulate=False)` — Pub/Sub, 默认 step 清空, `accumulate=True` 跨 step 累积
   - `EphemeralValue` — step 末自动清空, 适合 scratch state
   - `NamedBarrierValue` / `NamedBarrierValueAfterFinish` — 多节点同步点 (cycle barrier)
3. **Checkpoint 数据结构** (libs/checkpoint/.../base/__init__.py):
   ```python
   class Checkpoint(TypedDict):
       v: int  # 格式版本
       id: str  # uuid6, 单调递增
       ts: str  # ISO 8601
       channel_values: dict[str, Any]
       channel_versions: dict[str, str|int|float]  # per-channel 版本号
       versions_seen: dict[str, dict[str, ...]]  # node_id -> channel -> version_seen
       updated_channels: list[str] | None
   ```
   **关键洞察: `versions_seen` 矩阵** — engine 决定下一步跑哪些 node 时用 (node 只有看到新版本 channel 才会被调度). 这就是 langgraph 循环 + checkpoint 能 deterministic replay 的核心.
4. **DeltaChannel** (beta, 2026) — 大 list 通道按 snapshot_frequency 阈值只存增量, 重建时把 parent chain 的 `pending_writes` 累加到最近的 `_DeltaSnapshot` blob. 涉及 `get_delta_channel_history()` API + `DeltaChannelHistory` TypedDict — 是对长 memory 的存储工程优化.

### 关键算法 — Channel 双 dispatch + reducer 编译

1. **State schema → channel 字典编译** (state.py `_add_schema`): 走 typing introspection, `Annotated[type, reducer]` → `BinaryOperatorAggregate(type, reducer)`, 纯 `type` → `LastValue`. 多 schema 注册到 `self.schemas[schema]` 防止重复创建. 这就是 `TypedDict` + reducer 表达式的实现.
2. **Overwrite sentinel 3 形态兼容** (binop.py `_get_overwrite`): typed `Overwrite` 实例 / dict `{"__overwrite__": v}` / JSON-decoded `{"type": "__overwrite__", "value": v}` — 解决了 orjson 等序列化器把 dataclass 退化成 dict 时的语义丢失问题. 这点对我们 serde 设计有借鉴.
3. **ChannelProtocol 抽象** (serde/types.py): `runtime_checkable` Protocol, 让 serialization 端和 execution 端解耦.
4. **`BaseCheckpointSaver` 接口**: `get / get_tuple / list / put / put_writes / delete_thread / copy_thread / prune(keep_latest|delete)`. 最近的 `copy_thread` docstring 警告 `DeltaChannel` 实现必须复制完整 parent chain (不只 head checkpoint), 否则下游 delta 历史重建会断 — 这个工程教训非常宝贵.

### LLM 集成方式 (langgraph 本身不调 LLM, 在 langchain-core runnables)

`StateGraph.add_node(node, ..., retry_policy, cache_policy, error_handler, timeout)` 节点级 policy + `_NodeDefaults` 全局默认. 编译后是 `CompiledStateGraph.invoke/stream/ainvoke/astream`. **借鉴点**: retry + cache + error handler + timeout 4 元组作为 cross-cutting concern 抽象, 我们的 `self_evolving.py` 可以参考 `RetryPolicy` 这种 weighted sequence (匹配首条).

---

## anthropic-sdk-python (Claude API) — 代码层细节

**核心文件**: `src/anthropic/_streaming.py` (Stream + SSEDecoder), `src/anthropic/_models.py` (BaseModel + pydantic 包装), `src/anthropic/lib/streaming/_messages.py` (MessageStream accumulator), `src/anthropic/lib/tools/_beta_runner.py` (ReAct loop), `src/anthropic/resources/messages/messages.py`.

### 数据结构 — Stream + Message 累积模型

1. **`Stream[T]`** (Generic, sync) + `AsyncStream[T]` (Generic, async): 拿 `httpx.Response`, 内嵌 `SSEDecoder` 解 SSE, 暴露 `__iter__/__next__/__enter__/__exit__/close`. `_process_response_data(data, cast_to, response)` 把原始 JSON dict 经 Pydantic validation 变成 typed 对象再 yield. **`finally: response.close()`** 模式保证连接释放.
2. **Event 类型白名单** (`_streaming.py` `__stream__`): 巨大的 if-chain 匹配 `sse.event`, 支持 ~40 种事件 (message_start, content_block_start, content_block_delta, content_block_stop, message_delta, message_stop, agent.message, agent.thinking, agent.tool_use, agent.tool_result, mcp_tool_use, session.*, span.*, system.message). **未列入的事件被静默丢弃** — 这是 forward-compatibility 友好做法.
3. **`MessageStream`** (`lib/streaming/_messages.py`) = `Stream[ParsedMessageStreamEvent[T]]` 高级包装. 内部维护 `__final_message_snapshot: ParsedMessage[T] | None`, 每来一个 SSE event 调 `accumulate_event()` 增量修改 snapshot, 再调 `build_events()` 产出 user-facing event. **关键洞察: text_stream = filtered iterator**, 只 yield text_delta 累加 — 让 consumer 写 `for text in stream.text_stream: print(text, end='')` 一样简单.
4. **`accumulate_event` 增量更新** (`lib/streaming/_messages.py` ~line 425):
   - text_delta → `content.text += delta.text`
   - **input_json_delta (tool_use) → 累积 bytes (`__json_buf`) 用 jiter 的 partial_mode 增量 parse 出来 (这才是 partial JSON 还能用的秘密)**
   - thinking_delta / signature_delta / citations_delta → append/update
   - message_delta → 更新 stop_reason, **usage 字段 cache_creation_input_tokens / cache_read_input_tokens** 也增量更新 — prompt caching 记账从这里落地.

### 关键算法 — 增量累积 + JSON 增量解析

1. **JSON 增量解析** (jiter `from_json(buf, partial_mode=True)`): 在 tool_use 流式场景下, 每个 SSE event 都是 JSON 片段 (`{"name":"g`, `{"name":"get_wea`, ...), 累积成完整 buffer 后 partial parse, 这样 consumer 拿到的是**持续可用的 dict** 而不是 raw string. 这个模式我们 `llm_kernel.py` 接到 thinking/reasoning 流可以借鉴.
2. **`BetaToolRunner` ReAct loop** (`lib/tools/_beta_runner.py`):
   ```python
   def __run__(self) -> Iterator[RunnerItemT]:
       while not self._should_stop():
           with self._handle_request() as item:
               yield item
               ...
           if message.stop_reason == "refusal": return
           if not self._check_and_compact():  # token 溢出时 summary
               response = self.generate_tool_call_response()
               if response is None: return
               self.append_messages(message, response)
   ```
   **真生产借鉴**: (a) refusal 检测 → 不调 tool_use 因为副作用未确认; (b) **`_check_and_compact()`** — token 阈值触发自动 summarization (用独立的 summarize prompt + 移除末尾 tool_use blocks 防 400); (c) `_cached_tool_call_response` 缓存避免重复 invoke; (d) tool 找不到 / ToolError / generic Exception 三层 error 路径都返回 `tool_result with is_error=True`, 用 dict 表达错误 (而不是 raise) — 这样 LLM 看到 tool_result 能自己 recover.

### LLM 集成 — Tool dispatch + Refusal + Caching

Prompt caching 在 response.usage 里直接返回 **3 个独立计数**: input_tokens / cache_creation_input_tokens / cache_read_input_tokens. Cache read 比 creation 便宜 ~10x. 我们 `llm_kernel.LLMResponse` 当前只有 tokens_in/tokens_out, **没区分 cache hit/miss**, 这是 round-21 真生产可以加的字段.

---

## openai-python (OpenAI API) — 代码层细节

**核心文件**: `src/openai/_streaming.py` (Stream 同源 fork), `src/openai/_models.py` (BaseModel + 判别联合缓存), `src/openai/resources/responses/responses.py` (.parse 装饰器方法), `src/openai/lib/_parsing/{_completions.py, _responses.py}` (Pydantic → strict JSON schema → 解析回路), `src/openai/lib/_tools.py` + `lib/_pydantic.py`.

### 数据结构 — Pydantic 严格 schema + 装饰器方法

1. **`BaseModel(pydantic.BaseModel)`** (`_models.py`): `extra="allow"` (宽容 API 新字段) + `defer_build=os.environ.get("DEFER_PYDANTIC_BUILD", "true")` (延迟 schema 构建加速 import). `to_dict()` 是 recursive serializer. `_request_id` 通过类变量注入 (header X-Request-ID).
2. **`DiscriminatorDetails` + `DISCRIMINATOR_CACHE`** (`_models.py` ~line 710): `weakref.WeakKeyDictionary[type, DiscriminatorDetails]`. 给 `Union[A, B, C]` 配 `@discriminated_union` decorator, 自动 build `{discriminator_field: variant_class}` 映射, 缓存复用. 这就是 OpenAI 处理 `tool_call.type=="function"|"custom"` / `output.type=="message"|"function_call"|"reasoning"|...` 这种 tagged union 的核心.
3. **`pydantic_function_tool(model)`** (`lib/_tools.py`): 把 Pydantic class 变成 `{"type": "function", "function": {name, strict: True, parameters: to_strict_json_schema(model)}}`. **严格模式必备** — 没有 `strict=True`, SDK 拒绝 auto-parse.
4. **`.parse(text_format=Class)` 装饰器** (`responses/responses.py` line 1277):
   ```python
   def parse(self, *, text_format: type[TextFormatT], ...):
       text = copy(text or {}); text["format"] = _type_to_text_format_param(text_format)
       tools = _make_tools(tools)
       def parser(raw_response: Response) -> ParsedResponse[TextFormatT]:
           return parse_response(input_tools=tools, text_format=text_format, response=raw_response)
       return self._post("/responses", body=..., parser=parser)
   ```
   **巧妙模式**: 同一 endpoint, 用 `parser` callback 做后处理 — response 直接 post, 但 callback 把 JSON 用 `parse_response()` 升级成 Pydantic tree. 我们 `llm_kernel.call_llm_minimax` 当前 return `LLMResponse(content: str)` 单字段, 真生产可以加 `parsed_output: BaseModel | None` 字段, 由 .parse() 风格装饰器自动注入.

### 关键算法 — strict JSON schema 适配

1. **`_ensure_strict_json_schema`** (`lib/_pydantic.py` ~line 60):
   - `type == "object"` → 强制 `additionalProperties: false`
   - 所有 properties 名 → `required` (OpenAI strict 要求所有字段都必填, 即便有 default)
   - `anyOf` / `allOf` 递归处理; `allOf` 只有一个 variant 时直接 inline
   - **`$ref` unravel** — 因为 `{"$ref": "...", "description": "x"}` 在 strict 模式不被允许, 必须解析成 `{"type": "string", "description": "x"}` 之类; 同时 strip `default=None` (因为 schema 已经 nullable, 重复没意义)
   - 整套 recursive 修 schema, 是 OpenAI strict mode 兼容性的强制要求

2. **`parse_text(text, text_format)`** (`lib/_parsing/_responses.py`): 直接 Pydantic v2 `model_validate_json(text)` / `TypeAdapter(typ).validate_json(text)`. Pydantic v1 直接 raise. 这是 structured outputs 的最终落地.

3. **Stream 终止**: `[DONE]` sentinel (vs Anthropic 用 event type 区分). 实现了 `synthesize_event_and_data` option 给 chat 模式当需要 event+data dict 时用.

---

## Apeireth 可借鉴的具体架构模式 (表格)

| 借鉴源 | 具体模式 | 映射到 Apeireth | 落地难度 | 价值 |
|---|---|---|---|---|
| langgraph `BaseChannel` | 6 接口 `update/get/checkpoint/from_checkpoint/consume/finish` | `llm_kernel.py` 加 `LLMChannel` (State + LLM response 双存储) | 中 | 中 |
| langgraph `BinaryOperatorAggregate` + `Annotated[list, op.add]` | reducer 编译 | `memory_3tier.compile()` 已经分 3 模式, 可考虑把 MTM "summary" 当 reducer 而非显式函数 | 低 | 中 |
| langgraph `Checkpoint.versions_seen` | node→channel matrix 决定可调度性 | `self_evolving.py` 给 `Patch` 加 `versions_seen` 矩阵避免重复 apply | 低 | 高 |
| langgraph `DeltaChannel` snapshot 阈值 | 大 list 增量存储 | `memory_3tier.LTM` 大量 anchor 时考虑只存 delta + 周期 snapshot | 中 | 中 |
| anthropic `accumulate_event` 增量 JSON 解析 (jiter partial) | text deltas → 持续可用 snapshot | `llm_kernel.call_llm_stream` 流式输出 `LLMResponse` 增量构建 | 中 | 高 |
| anthropic `BetaToolRunner` `refusal + compaction + cached tool response` | 工具调用循环 4 防御 | `apeireth` 当前没有 tool_runner, round-21 真生产可以加 | 高 | 极高 |
| anthropic `usage.{cache_creation,cache_read}_input_tokens` | prompt cache 独立记账 | `LLMResponse` 加 `cache_hit_tokens / cache_miss_tokens` | 低 | 高 |
| openai `_ensure_strict_json_schema` | pydantic → strict JSON schema | 已有 `llm_kernel`, 加 strict-mode 兼容 (`structured_outputs=True`) | 低 | 高 |
| openai `.parse(text_format=Class)` 装饰器 | 同一 endpoint + parser callback | `call_llm_minimax(prompt, response_format=Plan)` 模式 | 中 | 高 |
| openai `DiscriminatorDetails` 弱引用缓存 | tagged union 快查 | 暂无显式 union, 借鉴度低; 但 `Patch.action` 字符串枚举 → enum 转换可用 | 低 | 低 |

---

## 推荐 / 不推荐 (实事求是)

### 强烈推荐 (Round-21 即开始落地)

- **anthropic-sdk 的 accumulate_event + partial JSON parse**: 真生产的 stream 增量累积, **直接借鉴到 `llm_kernel.call_llm_stream()`**. 推荐理由: 现有 Apeireth 流式 LLM 没用增量 snapshot, 拿到 raw chunk 才拼起来, 中途断了就丢 — 借鉴 jiter partial parse 可以让 streaming 的 tool_use 输入也支持 streaming-aware consumer.
- **anthropic `BetaToolRunner` 的 refusal + compaction + cached response**: 我们目前没有 tool_runner (letta compile 借鉴正在做, 但 letta 的 tool_call 是阻塞式), 真生产 tool_runner 应当有 (1) refusal 终止 (2) token 阈值触发 summarization (3) tool_response 缓存. **Round-21 这条优先级最高**, 因为它解决 "agent tool loop 跑飞了" 的真生产崩溃问题.
- **anthropic usage cache 字段**: `LLMResponse` 加 3 字段 (cache_creation / cache_read / cache_miss) — 落地成本 < 50 行, 但能把 prompt caching 成本可视化, 帮我们调到 Anthropic prompt cache hit > 80%.

### 推荐 (按需落地)

- **openai `_ensure_strict_json_schema`**: MiniMax-M3 经 NewAPI 兼容 OpenAI 接口, 大概率也支持 strict mode. 加 `response_format_strict=True` flag 让 LLMKernel 切到 JSON 模式. 借鉴 openai 写法 (additionalProperties=false + 全字段 required + $ref unravel). 给 pydantic 化的 Plan/Decision/Action schema 注入. **落地 ~100 行**.

- **openai `.parse(text_format=Class)` 装饰器**: 把 `call_llm_minimax(prompt, response_format=PlanClass)` 这个便利 API 加到 `llm_kernel.py`. 复用 strict JSON schema + `_post` parser 模式.

- **langgraph `versions_seen` 矩阵**: `self_evolving.Patch` 加 `seen_in: dict[commit_id, set[patch_id]]` 字段, 防止同一个 patch 被多个 propose path 重复 commit. **借鉴度中**, 因为我们当前 Patches 是单线 commit, 但 wireup 到 self_evolving.VERIFY 阶段会很有用.

### 不推荐 (理由)

- **langgraph `Pregel` superstep engine**: 我们没有多 node DAG + checkpoint resume 这种需求, Apeireth 是 single agent + memory 层. 借鉴 engine 是不必要的复杂化. **不推荐**直接抄.
- **langgraph `channels.binop.py` Overwrite 3 形态 sentinel 兼容**: 真生产 + JSON 跨边界才需要, 我们内部 dataclass-to-dataclass round-trip 没有这个问题. 等真出现再借鉴.
- **openai `DiscriminatorDetails` 弱引用缓存**: 我们 union 类型少, plain match 够用. 等遇到 performance hot path 再考虑.
- **langgraph `DeltaChannel` snapshot 阈值**: LTM 锚点数量级 (千级), 用不到. 真到百万级再考虑.

### 不确定 (需进一步调研)

- **anthropic-sdk VS openai-sdk streaming 几乎 fork 同源** — 这两家都是 httpx + Stainless 风格的现代 SDK. 我们自己写 `llm_kernel` 时是直接 import `anthropic` / `openai` 包, 还是不依赖、只 copy `_streaming.SSEDecoder` (~150 行)? 倾向**不依赖** (`requests` 已经在用), 借鉴 SSEDecoder 思路自己写更可控.
- **langgraph `Checkpoint` JSON 序列化** — 我们目前 `to_dict()` 是 ad-hoc; 是否引入 langgraph 风格的 `v/id/ts` 三字段 + `versions_seen` 元数据? 如果 `self_evolving` 演化历史要 checkpoint 化, 这套更规范化. **Round-22 可考虑**.

---

## Round-20 落地清单

- [ ] **P0**: `llm_kernel.LLMResponse` 加 `cache_hit_tokens / cache_miss_tokens / cache_creation_tokens`
- [ ] **P0**: `self_evolving.Patch` 加 `versions_seen` 矩阵防重复 commit
- [ ] **P1**: `llm_kernel.call_llm_stream` 引入 partial JSON parse (jiter 或自实现)
- [ ] **P1**: 写 `apeireth/tool_runner.py` (借鉴 `BetaToolRunner`, refusal/compaction/cached-response 3 防御)
- [ ] **P2**: `llm_kernel.call_llm_minimax(prompt, response_format=PlanClass)` strict JSON schema 装饰器

> 主 23:28 真研究哲学 — 看真代码看机制再决定. 主 22:33 自决 — 这 5 步全部由主 session 直接推动, 不等外部授权.
