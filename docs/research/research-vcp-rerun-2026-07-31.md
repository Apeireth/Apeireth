# R14-VCP 三项源码复调研：模型切换 / 插件协议 / 浪潮语义

> 日期：2026-07-31
> 任务：R14-VCP 真实源码复调研
> 研究对象：`Apeireth-rust/research/source/vcptoolbox/`（只读）
> 交付性质：研究证据与阶段 3+ 增量建议；**不写 Rust 代码、不冻结架构、不改 D1/D2/PREREQ 或既有 Stage 2/灵感文件**。

---

## 0. 执行摘要

本轮不是根据 VCP 门面文档复述，而是从本地 245MB 源码快照反向追到上游 Git commit，再对模型路由、65 份启用插件 manifest、TagMemo/Wave 主调用链逐段核验。

### 0.1 五个最重要的复核结论

1. **源码快照真实上游 commit 是 `9208a9135a7b121a0cc7f70bf2752ad8556d55ae`**（2026-07-11 14:28:04Z，message=`fix`）。本地目录删除了 `.git`，不能把 Apeireth 外层 commit 当 VCP commit；本报告以 GitHub tree API + 10/10 核心 Git blob SHA 一致性确认来源。
2. **模型“自然语言切换”是自然语言 route description 的 embedding/余弦匹配，不是 LLM 解析自由文本命令，也没有路由 DSL。**客户端只有显式选择 `VCPModelAuto`/预设名才进入自动路由；显式真实模型天然绕过自动路由。
3. **六类插件仍稳定，没有第七类。**快照实数为 65 启用 manifest、20 禁用 manifest、292 个插件脚本；类型分布为 35 synchronous / 2 asynchronous / 6 static / 3 service / 4 messagePreprocessor / 15 hybridservice。所谓“分布式”“WebSocket push”“人工审核”是正交能力，不是新 pluginType。
4. **“纯文本标记更省 token”不能成立为通则。**同一真实 SciCalculator 请求、`cl100k_base` 实测：VCP 标记 73 token，紧凑 Function Calling 49 token，完整 OpenAI tool-call 消息 73 token。输入侧真实 VCP 工具提示 553 token，对照紧凑 Function schema 89 token。VCP 的优势是模型兼容性、可读性和无需原生 FC，不是格式必然省 token。
5. **浪潮是真实的混合检索增强算法，但不是经典 LIF 神经元仿真，也不是 HNSW/向量库替代物。**它用 HNSW/Vexus KNN 找 tag/文档种子，再做有向共现图的离散阈值能量扩散、向量增强和 geodesic rerank；“河道能量”主要公式公开在源码，但分散在多阶段，不存在一条总公式。

### 0.2 对 Apeireth 的一句话建议

保留 Apeireth 已有的 **多策略 LLM 路由、PlainText+Structured+Binary 三轨插件调用、Qdrant/Tantivy/结构化记忆协调**；从 VCP 借鉴 **自然语言 route description、六类生命周期语义、查询级 energy field 与低可信回退**，但不要照搬“纯文本优于 schema”“Wave 是独立 DB”“LIF/300+ 插件已验证”等过度表述。

---

## 1. 证据基线、版本与统计口径

### 1.1 为什么不能直接 `git rev-parse`

`vcptoolbox/` 本地快照不含 `.git`；在其目录执行 `git rev-parse HEAD` 会向上找到 Apeireth 外层仓库，得到 `7d9370e...`，它不是 VCP 上游 commit。`git -C vcptoolbox ls-files` 为 0，也证明该目录没有自身 Git 索引。

上游地址由源码 `README.md:183-186,230` 明示为：

- `https://github.com/lioensky/VCPToolBox`

### 1.2 真实快照 commit 的确认方法

候选 commit：

- SHA：`9208a9135a7b121a0cc7f70bf2752ad8556d55ae`
- 时间：2026-07-11 14:28:04Z
- commit message：`fix`
- GitHub tree API：`/repos/lioensky/VCPToolBox/git/trees/9208a913...?recursive=1`
- tree 未截断：`truncated=false`

十个核心文件以原始字节计算 Git blob SHA，**10/10 与候选 tree 一致**：

| 文件 | Git blob SHA |
|---|---|
| `KnowledgeBaseManager.js` | `9b28fff0f32ad709c79a7d6f0d31cae0dda485c9` |
| `Plugin.js` | `5fd77cea37c30ef432f23d9627d05aba862b9744` |
| `README.md` | `bdca4dc7f7408e3f9ccaf5b9adfea362f2743296` |
| `ResidualPyramid.js` | `b1c2ecefd34369a2c50fbcc950d08a1c3ab9917a` |
| `SemanticModelRouter.json` | `ac9cd950ffdc8aa668e64424bbfa14af6d5658eb` |
| `TagMemoEngine.js` | `17852ad545f8e9d92bcba32e2c49b8c5b38302c0` |
| `modelRedirectHandler.js` | `0607b0e1ce655650b6450e084f0e0830660820b7` |
| `modules/semanticModelRouter.js` | `a9fad9eaec5db542bacc1225a6a43ab0053dd3a1` |
| `modules/vcpLoop/toolCallParser.js` | `e09fce0ba9edefd417d35ce628dac79e59e8b8b6` |
| `package.json` | `746ce6f19239d934e7e18f24a6dc44aaae7cd434` |

注意：部分本地文件是 CRLF。普通 `git hash-object` 可能受外层 `.gitattributes`/clean filter 影响；本轮使用原始 `blob <len>\0<bytes>` SHA-1 与 `git hash-object --no-filters` 复核，避免把行尾转换误判为源码漂移。

### 1.3 插件数与脚本数

#### 本地研究快照（本报告主要真相源）

| 口径 | 数量 | 定义 |
|---|---:|---|
| Plugin 一级目录 | 85 | `Plugin/*/`，含禁用/辅助目录，不能等同活跃插件 |
| 启用插件 | **65** | `Plugin/*/plugin-manifest.json` |
| 禁用插件 | **20** | `Plugin/*/plugin-manifest.json.block` |
| 插件脚本 | **292** | `Plugin/**` 下 `.js/.mjs/.py/.sh/.ps1` 文件 |
| 仓库 JS/Python | 339 | 62 `.py` + 277 非 dist `.js`；与“插件脚本”不是同一口径 |

#### 研究执行时上游主分支最新（用于提示漂移，不替代本地真相源）

- 最新 SHA：`f3593ad56fc88a58850ee6b6624d59805d09c7e9`
- 时间：2026-07-31 16:35:51Z
- 启用 manifest：66
- 禁用 manifest：20
- 插件脚本：295（同一五扩展名口径）

因此，源码/文档中的“79 活跃插件”或 README 的“300+ 官方插件”均不是该快照可复现统计；“300+”更可能混用了命令数、插件商店/历史/分布式生态或宣传口径，不能写成当前仓库活跃 manifest 数。

### 1.4 事实等级

- **S（Source）**：直接来自快照源码/manifest。
- **V（Verified provenance）**：由上游 API 与本地 blob 验证。
- **D（Document claim）**：VCP 文档/README 的自述，若与代码不一致，以 S 为准。
- **I（Inference）**：从源码结构推导，明确标注。
- **R（Recommendation）**：对 Apeireth 的非冻结建议。

---

# 2. 复调研一：模型路由与自然语言切换

## 2.1 “自然语言怎么描述切换意图”

VCP 不是让用户说“请切到 GPT”后由 LLM 解析，而是在 `SemanticModelRouter.json` 给每条 route 写自然语言能力描述：

```json
{
  "name": "research_and_coding",
  "model": "gpt-5.5",
  "description": "信息调研、资料检索、技术调研、文献整理、代码编写、脚本开发、调试程序、修改代码、命令行工具、自动化脚本、API集成、Web开发、数据处理脚本",
  "failoverPool": true
}
```

真实默认预设还包括：

- `daily_chat` → 日常聊天、闲聊、情感陪伴等；
- `deep_reasoning` → 多步骤推理、哲学/伦理/社科、跨学科分析；
- 文学预设分 `literary_discussion` / `literary_writing` / `literary_analysis`。

证据：`SemanticModelRouter.json:1-67`。

## 2.2 是否有路由 DSL 或结构化中间表示

有结构化配置，但没有独立 DSL/AST：

```text
Preset {
  defaultModel,
  fallbackModels[],
  matchThreshold,
  contextWeights[user, assistant],
  routes[] { name, model, description, failoverPool, enabled }
}
```

运行时中间结果 `resolveRoute()` 返回：

```text
{
  active,
  requestedModel,
  presetName,
  selectedModel,
  candidates[],
  match,
  rankedRoutes[],
  reason
}
```

这可视为轻量 `RoutingPlan`，但不是用户可编写的路由 DSL。`reason` 只有 `semantic_match`、`below_threshold_default`、`rag_plugin_unavailable`、`context_embedding_unavailable`、`routing_error:*` 等粗粒度原因。

证据：`modules/semanticModelRouter.js:157-227,370-405,409-503`。

## 2.3 谁解析自然语言

**不是 LLM。**调用链是：

1. `findLastRealUserMessage()` 找最后一条真实用户消息，跳过工具占位符/系统通知；
2. 提取最后一条 assistant 文本；
3. 复用 `RAGDiaryPlugin.getSingleEmbeddingCached()` 生成 user/assistant 向量；
4. 按 `[0.7, 0.3]` 默认权重求加权平均；
5. route `description` 也被 embedding（有内存+SQLite 持久化缓存）；
6. 做余弦相似度、阈值过滤和降序排序。

证据：

- `modules/semanticModelRouter.js:70-93,311-367,436-494`
- `docs/SEMANTIC_MODEL_ROUTER.md:7-16`

结论：VCP 的“自然语言路由”准确说法是 **embedding-based declarative semantic routing**。

## 2.4 切换条件判定

自动路由只在客户端请求的 `model` 满足以下条件时启用：

- `model === autoModelName`，默认 `VCPModelAuto`；或
- `model` 等于某个 preset key，例如 `VCPModelLiterature`。

显式真实模型（如 `gpt-4o`）不会被自动语义路由，只走 `ModelRedirect` 别名映射。

选择规则：

```text
score(route) = cosine(contextVector, embedding(route.description))
matched = score >= preset.matchThreshold
selected = highest(matched) or defaultModel
```

默认阈值 `0.18`。不是关键词规则，也没有成本、延迟、能力硬约束参与首选判定。

证据：`modules/semanticModelRouter.js:292-309,409-494`。

## 2.5 上下文迁移

VCP 的跨模型迁移很朴素、也很有效：

- 路由只改写 `body.model`；
- `messages`、`tools` 和其他 body 字段保持不变；
- 首次调用在所有消息预处理、Detector 和 Role Divider 完成后确定最终 body；
- retry 时再次解析同一个 JSON body，仅替换 `model`；
- 工具循环持续使用同一 `semanticModelFallbackCandidates`，不会因工具结果重新路由。

这不是“摘要后迁移”，也没有模型专属 prompt/schema 转译。上下文兼容性由“所有候选模型必须接受同一 messages/tools”这一隐含前提承担。

证据：

- `modules/chatCompletionHandler.js:376-404,1071-1181`
- `docs/SEMANTIC_MODEL_ROUTER.md:273-279`

## 2.6 失败回退与错误分类

### 候选链构造

若命中 route：

```text
[首选 route]
+ [其他超过阈值且 failoverPool=true 的 route，按相似度]
+ [defaultModel]
+ [fallbackModels]
```

若首选 route 的 `failoverPool=false`，其他 route 不进入链，但仍追加 `defaultModel + fallbackModels`。

证据：`modules/semanticModelRouter.js:370-405`。

### 重试/换模真实分类

| 情况 | 是否重试/换候选 | 真实行为 |
|---|---|---|
| HTTP 500 | 是 | 线性退避，下一 attempt 使用候选链下一模型 |
| HTTP 503 | 是 | 同上 |
| HTTP 429 | 是 | 同上；没有按 provider 单独 cooldown |
| 特定 HTTP 401 且 body 含 `token` | 是 | 视为瞬时上游 token 异常 |
| 连接超时 | 是 | `AbortController` 中止本 attempt，重试 |
| 一般网络异常 | 是（未耗尽时） | 重试 |
| 用户/外部 abort | 否 | 立即抛出，不重试 |
| HTTP 400/413/上下文过长 | 否 | 直接返回上游响应；无长度压缩后重试 |
| 能力不足（无 vision/FC 等） | 未分类 | 无 capability predicate |
| 费用过高/预算耗尽 | 未分类 | 无 cost/budget gate |
| 内容政策拒绝 | 未分类 | 无策略化改路由 |
| 空 embedding / RAG 不可用 | 是默认计划 | 不做语义匹配，走 default/fallback |

证据：`modules/chatCompletionHandler.js:407-530`。

**关键纠偏**：VCP 的失败换模是“对一组可重试 transport/status 错误轮换候选”，不是基于错误类型选择不同模型。

## 2.7 工具自然语言输出 → 系统动作

这里必须区分两个方向：

1. **模型自然语言/文本 → 系统工具动作**：模型输出 `<<<[TOOL_REQUEST]>>>...`；`ToolCallParser` 解析 marker、`tool_name` 与字段，PluginManager 执行。这是真实存在的文本到动作。
2. **工具自然语言输出 → 系统动作**：普通插件结果会被格式化后放回模型循环，由模型决定下一步；系统不会把任意工具自然语言直接当命令执行。静态占位符/消息预处理器属于显式注册的自动处理，不是自由文本动作解释器。

模型切换也不是由工具结果直接触发：同一次 VCP tool loop 使用固定候选链，不在每个工具结果后重新做语义路由。

证据：

- `modules/vcpLoop/toolCallParser.js:5-117,151-229`
- `modules/handlers/streamHandler.js:12-55`
- `docs/SEMANTIC_MODEL_ROUTER.md:275-278`

## 2.8 用户手动切换 vs 系统自动切换仲裁

真实优先级：

```text
用户选真实模型 ID  >  ModelRedirect 别名  >  不启用语义路由
用户选虚拟模型/预设 >  启用语义路由       >  系统自动选择与回退
用户取消请求         >  立即中止           >  禁止自动重试
```

所以 VCP 没有复杂仲裁器；“手动优先”由触发条件自然实现。用户选择 `VCPModelAuto` 等于显式授权系统自动切换。

## 2.9 与 Apeireth 现有设计差异

| 维度 | VCP 源码快照 | Apeireth 现有设计 | 判断 |
|---|---|---|---|
| 意图表达 | route description 自然语言 | `RoutingPolicy` 枚举 + capability | 可组合，不应二选一 |
| 解析 | embedding + cosine | Fixed/Cost/Latency/Capability/RR/Bandit | Apeireth 更完整 |
| 触发 | 仅虚拟模型名/预设 | 默认策略驱动 | 应保留显式手动 override |
| 成本 | 不参与首选 | `cost_estimate` + CostOptimized + budget | Apeireth 胜 |
| 延迟 | 不参与首选 | LatencyOptimized | Apeireth 胜 |
| 能力 | 描述中软表达 | `Capability` 硬筛选 | Apeireth 胜，软路由可做二级排序 |
| 上下文 | 原 body 原样换 model | 有 ContextCompactor 设计 | Apeireth 更安全，但需验证语义损失 |
| 失败分类 | 500/503/429/特殊401/网络 | fallback + circuit breaker + retry 草案 | Apeireth 应补 typed error matrix |
| 自动/手动 | 真实模型绕过；虚拟模型授权 | 未画出仲裁优先级 | VCP 这一点值得明确画图 |
| 工具循环再路由 | 不再评估 | 未明确 | 建议默认固定，显式 checkpoint 才重路由 |

Apeireth 证据：`stage2-decisions-llm-integration.md:21-29,34-98,125-220,377-415,497-547`。

## 2.10 借鉴 / 不借鉴决策

### 借鉴

- **自然语言 route description 作为软评分层**：降低策略配置门槛。
- **显式虚拟模型才授权自动路由**：保留用户主权，可解释。
- **RoutingPlan + reason + ranked candidates**：适合审计和复现。
- **一次工具循环固定候选链**：避免工具输出引发不可预测抖动。
- **route description embedding 持久化缓存**：减少重复成本。

### 不直接借鉴

- **只用余弦阈值决定模型**：缺成本/能力/长度/隐私硬约束。
- **所有可重试错误都顺序换下一个模型**：可能把 429、超时、能力不足错误混为一谈。
- **跨模型原样复用 tools/messages 即视为兼容**：不同 provider schema、thinking、vision 能力不一定一致。
- **文档中的动态均衡/成功率自动调权宣称**：本快照核心路由代码未实现该闭环。

## 2.11 阶段 3+ 增量项（非冻结）

- **P9 图纸增量候选**：画出 `ManualOverride > HardConstraints > SemanticScore > Cost/Latency > Fallback` 五级仲裁。
- 定义 `RoutingIntent` 结构：`task_semantics / required_capabilities / privacy / max_cost / deadline / context_tokens / user_override`。
- 定义 typed `RouteFailure`：timeout、rate_limited、context_too_long、unsupported_capability、budget_exceeded、policy_refusal、auth、provider_down。
- 定义 `ContextMigrationPolicy`：pass-through、provider-adapter、compact、reject；禁止静默丢字段。
- 在工具循环设置显式 reroute checkpoint；默认不因普通工具结果重新选模。
- 增加决策 trace：输入约束、候选过滤原因、评分、最终选择、fallback 原因、用户 override。

---

# 3. 复调研二：六类插件协议

## 3.1 六类是否仍稳定，有无新类

65 份启用 manifest 的 `pluginType` 精确分布：

| pluginType | 数量 | 加载/执行语义 | 示例 |
|---|---:|---|---|
| `synchronous` | **35** | stdio 子进程；等待标准 JSON 结果 | SciCalculator、FileOperator、VSearch |
| `asynchronous` | **2** | stdio 首响/任务 ID；后台完成/回调语义 | AgnesVideoGen、VideoGenerator |
| `static` | **6** | cron 刷新，结果进 placeholder cache | WeatherReporter、EmojiListGenerator |
| `service` | **3** | direct 模块，注册 HTTP/WS 服务 | VCPLog、ImageFileServer |
| `messagePreprocessor` | **4** | direct `processMessages()`，模型调用前改写消息 | CapturePreprocessor、ContextFoldingV2 |
| `hybridservice` | **15** | 同时作为 preprocessor/service，且可有 direct `processToolCall()` | LightMemo、RAGDiaryPlugin、VCPTavern |

**没有第七个 pluginType。**

以下是正交扩展，不应误报成新类型：

- `communication.protocol`: `stdio` / `direct` / 分布式来源；
- `webSocketPush`；
- `hasApiRoutes` / `registerApiRoutes`；
- `requiresContextBridge` / `requiresAdmin`；
- 分布式 plugin manifest (`isDistributed`, `serverId`)；
- 工具人工审核；
- `archery/no_reply` 调用修饰符。

证据：

- manifest 全量解析；
- `Plugin.js:543-750`（发现、direct 模块注册、预处理器顺序）；
- `Plugin.js:1055-1170`（distributed/direct hybrid/stdio 分发）；
- `Plugin.js:1571-1628`（service HTTP route 挂载）。

## 3.2 “六类协议”更准确的建模

它们不是六种互斥 wire protocol，而是把多个轴压扁成 `pluginType`：

```text
触发：periodic / pre-model / model-requested / external HTTP
等待：sync / async
驻留：ephemeral subprocess / in-process resident service
传输：stdio / direct / websocket-distributed
输出：placeholder / message rewrite / tool result / route
```

`hybridservice` 本身就证明分类是能力组合。Apeireth 若照抄 enum，未来会继续出现组合爆炸；更适合把“生命周期、触发、transport、residency、response mode”拆成正交 manifest 字段，同时保留六类作为兼容 profile。

## 3.3 真实调用案例

### 案例 A：同步 SciCalculator

模型输出：

```text
<<<[TOOL_REQUEST]>>>
tool_name:「始」SciCalculator「末」,
expression:「始」sqrt(variance([2,4,4,4,5,5,7,9])) + integral('exp(-x**2)', '-inf', 'inf')「末」
<<<[END_TOOL_REQUEST]>>>
```

解析器扫描 block 和 `「始」/「末」` 字段，生成：

```json
{
  "name": "SciCalculator",
  "args": { "expression": "..." },
  "archery": false,
  "markHistory": false
}
```

PluginManager 将 args JSON 作为 stdio 进程参数，要求插件输出 `{status,result/error}`。

证据：`Plugin/SciCalculator/plugin-manifest.json:17-26`、`modules/vcpLoop/toolCallParser.js`。

### 案例 B：异步 AgnesVideoGen

`submit` 秒级返回 `task_id`，用户/AI稍后用 `query`；manifest 类型虽是 asynchronous，但同一个插件也含同步 `concat` 命令。这进一步说明 type 是主生命周期 profile，不足以描述每个 command 的等待语义。

证据：`Plugin/AgnesVideoGen/plugin-manifest.json:1-82`。

### 案例 C：messagePreprocessor

`chatCompletionHandler` 在模型请求前遍历 `pluginManager.messagePreprocessors`，调用 `executeMessagePreprocessor(name, processedMessages, config)`；异常时保留原 messages，不中断主请求。

证据：`modules/chatCompletionHandler.js:1036-1061`、`Plugin.js:479-499`。

### 案例 D：hybridservice LightMemo

- manifest 为 direct hybridservice；
- `processToolCall()` 可执行 `SearchRAG` 或 `MapDistance`；
- 同进程注入 `vectorDBManager/getEmbedding/contextBridge`；
- 无需 stdio 子进程。

证据：`Plugin/LightMemo/plugin-manifest.json`、`Plugin/LightMemo/LightMemo.js:104-151`。

## 3.4 纯文本标记 vs Function Calling：真实 token 测量

### 测量条件

- tokenizer：`gpt-tokenizer@3.4.0`
- encoding：`cl100k_base`
- 样本：真实 `SciCalculator` manifest 中的表达式
- 测量对象：精确 UTF-8 字符串；不是 provider 账单（不同 provider 可能对 tool schema 另加隐藏 token）

### 输出调用载荷

| 序列化 | 字符 | UTF-8 bytes | token | 相对紧凑 FC |
|---|---:|---:|---:|---:|
| VCP marker 原文 | 167 | 191 | **73** | +24 / **+49.0%** |
| 紧凑 FC `{name,arguments}` | 133 | 133 | **49** | 基线 |
| 完整 OpenAI assistant tool-call JSON | 229 | 229 | **73** | 与 VCP 相同 |

结论：marker 与完整消息包装在该样本打平；与紧凑 function call 比，VCP 更贵。不能宣称“纯文本格式天然省 token”。

### 输入工具定义

按 `Plugin.js:759-788 buildVCPDescription()` 对 SciCalculator 真实 manifest 拼出的 VCP 工具提示，与一个紧凑等价 Function schema 对比：

| 输入定义 | 字符 | UTF-8 bytes | token |
|---|---:|---:|---:|
| VCP 真实 prompt（长功能清单+两份调用示例） | 1212 | 1580 | **553** |
| 紧凑 Function schema（name/description/expression schema） | 266 | 356 | **89** |

VCP 多 464 token（+521.3%）。这个差值既包含“标记重复”，也包含 VCP 更详尽的数学能力说明；所以它证明真实仓库当前注入不省 token，但不能推出所有同信息量 schema 都一定更省。

### 正确归因

VCP 的真实优势：

- 不要求 provider 原生 Function Calling；
- 任意可输出文本的模型可参与；
- 人类可直接阅读/调试；
- fuzzy marker/escape 兼容模型轻微格式漂移；
- 结果可自然语言化。

真正的 token 经济来自：

- 只注入相关工具（dynamic tool folding）；
- 缩短描述、移除重复 example；
- 分层发现（先 brief，命中后 full schema）；
- provider 可用时使用原生结构化通道。

不是来自 `<<<[TOOL_REQUEST]>>>` 这组字符本身。

## 3.5 与 Apeireth 现有设计差异

| 维度 | VCP | Apeireth 现有设计 | 判断 |
|---|---|---|---|
| 分类 | 六个组合型 `pluginType` | category + sandbox + PluginCall/Response | Apeireth 更正交，但图纸仍写“VCP 6 类协议” |
| transport | stdio/direct/distributed WS | inproc/WASM/subprocess + 5 层总线 | Apeireth 更宽 |
| 调用格式 | marker plain text 为主 | PlainText + Structured + Binary | Apeireth 三轨正确 |
| schema | prose + example | Structured args `Value`、manifest.toml | 不应删除 structured |
| 安全 | 运行时人工审核、requiresAdmin 等 | permission/council/multisig/sandbox | Apeireth 更强，需避免全量 council 阻塞 |
| 热加载 | direct 模块重载有边界 | registry + unload 草案 | 需以 supervisor 隔离落实 |
| async | plugin 级类型 | 尚未明确 command-level completion | 可借鉴 job handle，但不要 plugin 级锁死 |
| 分布式 | 来源 manifest + WS bridge | L3/L4 + MCP | 保持适配层，不当第七类 |

Apeireth 证据：`stage2-decisions-modularity.md:269-417`。

## 3.6 借鉴 / 不借鉴决策

### 借鉴

- 六类作为 **compatibility profiles**，便于迁移 VCP 生态。
- manifest 驱动、独立 config、可禁用 `.block`、预处理顺序。
- direct 与 stdio 双路径；异构插件默认 subprocess 隔离。
- command 结果标准 envelope + 自然语言正文双层。
- fuzzy marker 仅作为不支持 FC 模型的降级 adapter。
- async job handle、callback、placeholder/状态查询语义。

### 不借鉴

- 把六类写死成未来唯一 enum；应正交拆轴。
- 把任意自然语言工具结果直接提升为系统动作；必须经过显式 parser/schema/权限边界。
- 取消 Function Calling/JSON Schema；实测不支持“更省 token”的理由。
- 依赖 prompt 中重复长示例来保证正确性；应以契约测试和按需 schema 为主。
- 宣称“全部支持分布式”；具体 direct module、文件路径和依赖注入仍有本机耦合。

## 3.7 阶段 3+ 增量项（非冻结）

- **P10 图纸增量候选**：把六 profile 拆成 `Trigger × Residency × Transport × Completion × Output` 五轴，并标 VCP profile 映射。
- 定义 adapter 选择：native FC → structured JSON → VCP marker，按 provider 能力降级。
- async 改为 command-level `Immediate | Deferred(JobId) | Stream`，不要只看 pluginType。
- 工具目录实施 brief → full schema 两阶段发现，并记录真实 token budget。
- 为 plain-text parser 建 fuzz/property tests、嵌套 escape tests、prompt injection/假 marker tests。
- 所有自然语言 result 默认是数据，不是动作；动作必须重新进入权限化 `PluginCall`。

---

# 4. 复调研三：浪潮语义物理沙盘

## 4.1 实际组件与调用链

```text
用户/上下文文本
  → embedding
  → ResidualPyramid: 多层 HNSW tag KNN + Gram-Schmidt 残差
  → EPA: 逻辑深度/熵/跨域共振
  → Tag 种子加权（层衰减、语言补偿、core boost）
  → ordered bidirectional co-occurrence graph
  → 离散 spike/energy propagation（最多 4 hop）
  → 加权 tag 向量合成并归一化
  → Vexus/HNSW 文档 KNN
  → 可选 geodesic rerank（energy field + KNN 混合）
  → 可选 external reranker/RRF
```

核心文件：

- `TagMemoEngine.js`
- `ResidualPyramid.js`
- `EPAModule.js`
- `KnowledgeBaseManager.js`
- `Plugin/RAGDiaryPlugin/RAGDiaryPlugin.js`
- `Plugin/LightMemo/LightMemo.js`
- `rust-vexus-lite/`

## 4.2 联想网络真实数据结构

### 节点

SQLite `tags` 表中的 tag；每个节点有 embedding。派生节点属性包括：

- `tagIntrinsicResiduals`: 节点内生残差/概念锚质量；
- `adjustedWeight`: 本轮查询的初始/累计权重；
- core/ghost/emergent 标记。

### 边

由 `file_tags(file_id, tag_id, position)` 中同一文件的有序 tag 对构建：

- 每对同时写 forward/backward 边；
- 顺流通常更强；
- 边融合序位势能、tag 间余弦相似度、距离衰减；
- 每个文件超过 100 tag 时跳过矩阵 pair 计算；入库侧单文件 tag 截断到 50。

边在内存中为：

```text
Map<fromTagId, Map<toTagId, weight>>
```

相似度持久化在 SQLite `tag_pair_similarity(tag_a,tag_b,similarity,model_sig,computed_at)`。

## 4.3 河道能量公式是否公开

**公开，但分散为多阶段公式；“河道能量”不是单一闭式方程。**

### A. 残差金字塔

原始能量：

```text
E0 = ||q||²
```

每层用 HNSW 取 top-K tags，以 Gram–Schmidt 求当前残差对 tag 子空间的正交投影：

```text
E_explained(level) = max(0, ||R_old||² - ||R_new||²) / E0
```

当残差能量比低于默认 0.1（解释约 90%）或达到 maxLevels=3 时停止。

证据：`ResidualPyramid.js:25-111`。

### B. 查询级 TagBoost

```text
resonanceBoost = ln(1 + resonance)
dynamicBoost = logicDepth × (1 + resonanceBoost)
               / (1 + 0.5 × entropyPenalty)
               × activationMultiplier
effectiveTagBoost = baseTagBoost × clamp(dynamicBoost, boostMin, boostMax)
```

core boost 还结合 `logicDepth` 与 `1-coverage`。

证据：`TagMemoEngine.js:211-239`。

### C. 有向共现“河道”边权

默认序位势能范围 `PHI_MAX=0.9`、`PHI_MIN=0.5`：

```text
φ(pos) = φmax - (φmax-φmin) × (pos-1)/(n-1)
distanceFactor = exp(-distanceDecay × (Δpos-1))
baseWeight = φ_i × φ_j × distanceFactor
```

语义钟形增益：

```text
g(sim) = 0.4 + sim,                         sim < 0.15
g(sim) = 0.5 + 0.8 exp(-(sim-peak)²/(2σ²)), otherwise
```

默认 `peak=0.65`、`sigma=0.25`、未命中 sim fallback=0.1。

顺流：

```text
w(i→j) = baseWeight × forwardGain × g(sim)
```

逆流：

```text
reverseGain' = clamp(reverseGain × optionalAnchorBoost, min, max)
w(j→i) = baseWeight × reverseGain' × g(sim)
w(j→i) ≤ 0.95 × w(i→j)
```

默认 reverseGain=0.42，范围 [0.25,0.70]。

证据：`TagMemoEngine.js:901-1091`。

### D. “神经信号”传播

默认参数：

- max hops = 4
- base momentum = 2.0
- firing threshold = 0.10
- base decay = 0.25
- wormhole decay = 0.70
- tension threshold = 1.0
- max emergent nodes = 50
- max neighbors/node = 20

每条边：

```text
tension = coocWeight × targetIntrinsicResidual
isWormhole = tension >= threshold
decay = isWormhole ? 0.70 : 0.25
momentumCost = isWormhole ? 0 : 1
injectedCurrent = sourceEnergy × coocWeight × decay
```

多个入边的 `injectedCurrent` 求和；momentum 取最大；低于 0.01 丢弃。最终 energy field 是每个节点跨 hop 累计能量。

证据：`TagMemoEngine.js:325-449`。

### E. geodesic rerank

候选文档的 tag 命中达到 `minGeoSamples` 后：

```text
geoScore(doc) = sum(energy(tag in doc)) / hitCount
normalizedGeo = geoScore / maxGeo
finalScore = (1-α) × knnScore + α × normalizedGeo
```

若 energy field 太稀、熵太低、候选覆盖/强度/区分度不足，整批回退原 KNN 排序。

证据：`TagMemoEngine.js:681-899`。

## 4.4 它是真 LIF 神经网络吗

**不是经典 LIF。**源码注释叫 “LIF Spike Propagation / Lif-Router”，但实际状态和方程是离散图扩散：

- 有 energy threshold；
- 有 hop/TTL 式 momentum；
- 有边权、衰减、累计与涌现截断；
- **没有**连续/离散 membrane potential 的 leak-to-rest 方程；
- **没有**firing 后 reset；
- **没有**refractory period；
- **没有**按时间步积分的标准 LIF 参数 `τ_m / V_rest / V_reset / V_th`。

准确命名建议：**bounded thresholded energy diffusion with momentum and wormhole heuristic**（有界阈值能量扩散 + 动量/虫洞启发式）。

这不是贬低：其工程目标是关联召回，不需要为了仿生标签强行变成 SNN；但 Apeireth 文档不应把它当“真实神经网络信号传播已验证”。

## 4.5 “每用户独特语言坐标系”是否落地

门面文档宣称“为每个用户的记忆/语言/能量传播重画独特参考系”。源码确实按用户/日记数据构建 tag、共现和 embedding 派生结构，因此不同数据集会产生不同图；但本快照没有看到：

- 每用户单独训练 embedding 空间；
- 可学习的个体坐标变换矩阵；
- 对同词按用户生成不同基础 embedding 模型；
- 明确的 online metric learning。

更准确说法：**个体数据诱导的 tag topology/edge weights 不同**，不是基础语言向量坐标系被重新训练。

## 4.6 与 GraphRAG / Zep / HNSW 对比

> 下表对 GraphRAG/Zep/HNSW 采用其公开范式级定义作对照，本轮没有对这些外部项目做同等级源码审计；精确版本实现差异应另开任务。

| 维度 | VCP Wave | GraphRAG | Zep/Graphiti 类时序知识图 | HNSW |
|---|---|---|---|---|
| 基本对象 | tag、文档 chunk、共现位置 | entity/relation/community/report | episode/entity/fact/temporal edge | embedding point |
| 图如何来 | 日记 tag 共现 + 顺序 + embedding 相似 | LLM/entity extraction + relation graph | 对话/事件抽取，维护事实与时间有效性 | ANN 邻接图由几何构建 |
| 查询 | KNN 种子 → tag 扩散 → 向量增强/重排 | local/global graph traversal + community summaries | entity/fact/episode 检索 + temporal validity | 近似最近邻 |
| 方向/时间 | tag position 形成顺逆流；时间本身较弱 | 关系可有向；时间依实现 | 时间、事实失效/演化是核心 | 无语义时间 |
| 可解释性 | 可列 matched tags、energy、edge | entity/path/community 可解释 | fact/episode provenance 较强 | 仅邻居与距离 |
| LLM 写入成本 | tag 提取依上游流程；在线扩散无 LLM | 图抽取/摘要成本高 | 实体/事实抽取成本高 | 无 LLM |
| 主要优势 | 个体共现联想、轻量本地、可与 KNN 混合 | 跨文档实体关系与全局主题 | 对话事实的时间演化 | 大规模向量召回速度 |
| 主要风险 | 启发式参数多、共现≠因果、缺标准 benchmark | 抽取误差/成本/复杂部署 | 实体消歧和时间一致性复杂 | 只能“近”，不懂业务关系 |

**最关键结论**：Wave 与 HNSW 是互补层，不是替代；Wave 与 GraphRAG/Zep 才在“结构化关联/联想”层有部分竞争，但 Wave 的图来自 tag 共现而非显式事实关系。

## 4.7 API 与部署形态

### 运行形态

- Node.js 主进程内：`KnowledgeBaseManager`、`TagMemoEngine`、RAG/LightMemo direct module；
- SQLite：files/chunks/tags/file_tags/tag_pair_similarity/kv_store；
- 本地 Rust addon：`rust-vexus-lite`，承载向量索引/部分派生计算；
- embedding/rerank：可调用外部模型 API；
- 插件管理/前端：adminServer/Express；
- 可分布式接插件，但 Wave 核心本快照默认不是独立网络服务。

### 面向模型/用户的接口

1. `RAGDiaryPlugin`：`hybridservice + direct`，主要是 `processMessages()`，在上游模型调用前自动注入记忆。
2. `LightMemo SearchRAG`：VCP 文本工具，支持 `tag_boost=0.6+` 开 geodesic rerank。
3. `LightMemo MapDistance`：VCP 文本工具，输出 Markdown 距离表。
4. Admin REST：
   - `/admin_api/rag-params`
   - `/admin_api/rag-tags`
   - `/admin_api/vectordb-status`
   - `/admin_api/rag-active-full-training`

### “语义物理沙盘”的真实形态

README 有“浪潮语义物理沙盘”截图，但源码文本检索未发现同名前端组件/API。可复现的诊断接口是 `MapDistance`：

- 输出纯 KNN、Wave TagBoost、energy field、加权 geodesic 的 Markdown 表；
- 不是图形化粒子/流体仿真服务；
- 没有通用 REST `query-wave` API。

因此报告中应称“Wave/TagMemo 引擎 + MapDistance 诊断测绘”，而不是推断存在一个独立物理沙盘后端。

## 4.8 与 Apeireth 现有设计差异

| 维度 | VCP 真实源码 | Apeireth 现有设计 | 判断 |
|---|---|---|---|
| Wave 定位 | KNN 前后增强/重排引擎 | `BackendType::Wave` 独立 DB | **需要重审**：更像 ranking/association engine |
| 向量层 | SQLite + Vexus/HNSW 本地 | Qdrant | 可保留 DataBackend，Wave 叠在 query pipeline |
| 全文层 | BM25/jieba，另有冷知识库 | Tantivy | Apeireth 更系统 |
| 图语义 | tag 共现/位置，不是事实 KG | `Query::Graph` 泛型 | 需区分 AssociationGraph 与 FactGraph |
| 传播 | 有界离散能量扩散 | 文档写“神经网络信号传播” | 应降级术语，先 benchmark |
| 个体化 | 用户数据诱导不同图 | A 层经验联想 | 可借鉴，但不要宣称重训坐标系 |
| 回退 | 低可信 energy map 回 KNN | 未细化 | 值得强制 |
| 可观测 | matched tags/energy/MapDistance | Stage 3 P6 未完成 | 可直接转为验证图与指标 |
| 事务 | SQLite + Rust 写租约/WAL 屏障 | 6 DB + Saga | Apeireth 复杂度更高，首版勿过度分布式 |

Apeireth 证据：

- `inspiration-stage1-2026-07-30.md:512-567,578-597`
- `stage2-decisions-persistence.md:22-70,74-140,179-188`
- `stage3-blueprints/01-overall-architecture.md:56-63`

## 4.9 借鉴 / 不借鉴决策

### 借鉴

- `AssociationGraph` 与 ANN 互补：KNN 召回种子，图扩散补联想，再混合重排。
- 有向共现与位置衰减：比无向共现更能保留叙事顺序。
- query-scoped energy field：禁止全局 mutable cache 参与并发请求。
- 低可信地图回退 KNN：以 field size、entropy、coverage、spread 作门控。
- 残差金字塔的多意图/弱信号捕获思路；先以可解释实验验证。
- `MapDistance` 诊断形式：同一查询同时展示 baseline 与增强空间。
- 派生索引异步重建、模型签名失效、写租约/健康屏障等工程纪律。

### 不直接借鉴

- 把 Wave 当一个与 SQLite/Qdrant/Tantivy 同层的 DB backend。
- 把 tag 共现直接叫因果/事实关系。
- 把当前扩散算法叫经典 LIF/SNN。
- 照搬 0.42/0.70/0.10/4 hops 等参数；它们需要 Apeireth 数据校准。
- 先做“虫洞/朗飞结”品牌概念再做 benchmark。
- 接受“0.01ms/O(1)/十万 tag 0.7ms”等门面性能数字；本快照没有随附可复现 benchmark 结果证明完整查询链达到该值。

## 4.10 阶段 3+ 增量项（非冻结）

- **P6 记忆图纸增量候选**：把 `Wave[(DB)]` 改画为 `Association Engine`，位于 Vector/FullText candidates 与 final rerank 之间；是否改既有图由后续架构任务决定，本报告不改。
- 区分两张图：`AssociationGraph(tag co-occurrence)` 与 `FactGraph(entity/relation/temporal validity)`。
- 定义统一 `RetrievalTrace`：KNN baseline、seed tags、propagated nodes、edge contributions、fallback reason、final score。
- 先建 benchmark：Recall@K、nDCG、MRR、counter-association precision、latency P50/P95、memory、rebuild time。
- 建消融：KNN / +residual / +directed cooccurrence / +energy diffusion / +geodesic / +rerank。
- 以真实数据校准 hop、threshold、reverse gain、semantic peak；禁止照抄常数。
- 并发测试强制 query-scoped state；加入 cross-query contamination 检查。
- 保持 GraphRAG/Zep adapter 可能性，不在 Stage 3 先选唯一图路线。

---

# 5. 三项综合决策矩阵

| VCP 机制 | 证据强度 | 对 Apeireth 价值 | 决策 | 理由 |
|---|---|---|---|---|
| 自然语言 route description | 高 | 高 | 借鉴 | 配置友好，可作为 hard constraints 后的软评分 |
| 虚拟模型显式授权自动切换 | 高 | 高 | 借鉴 | 用户主权清晰 |
| embedding-only 模型决策 | 高 | 中 | 不单独采用 | 缺成本/能力/长度/隐私约束 |
| 固定工具循环候选链 | 高 | 中高 | 默认借鉴 | 降低行为抖动 |
| 六 pluginType | 高 | 高（兼容） | profile 借鉴 | 生态迁移有用，但内部应正交建模 |
| VCP marker | 高 | 中 | 降级 adapter | 兼容无 FC 模型，不作为主通道唯一选择 |
| “marker 更省 token” | 反证明确 | 负 | 不采纳 | 真实样本不支持 |
| 工具自然语言 result | 高 | 中高 | 作为数据层借鉴 | 人可读；不得直接变动作 |
| 有向 tag 共现图 | 高 | 高 | 借鉴并验证 | 保存叙事顺序，适合联想 |
| 离散能量扩散 | 高 | 中高 | 实验性借鉴 | 有工程价值，但启发式多 |
| “经典 LIF” | 反证明确 | 低 | 不采用术语 | 源码无标准 LIF 动力学 |
| Wave 作为 DB | 低/模型不符 | 中 | 重定位 | 更像 retrieval/ranking engine |
| geodesic 低可信回退 | 高 | 高 | 强烈借鉴 | 防止图噪声压过 KNN |
| 个人语言坐标系 | 部分 | 中 | 降级为个体图 | 数据诱导 topology 成立，重训坐标系未证实 |

---

# 6. 对 Stage 3 蓝图的最小增量建议（仅研究建议）

本报告不改图、不冻结架构。若 Leader 后续分派架构任务，建议只加三张/三处增量，不重画全部：

1. **P9 LLM 路由图**：补 `manual override → hard capability/privacy/context/budget filters → semantic route description → cost/latency ranking → typed fallback`。
2. **P10 Plugin 图**：六类从“六 wire protocols”改成“VCP compatibility profiles”，内部画五轴；PlainText/Structured/Binary 三轨保留。
3. **P6 Memory 图**：Wave 从 Data backend 候选移动为 retrieval pipeline 的 Association Engine；向量、全文、事实图、联想图并存，由 coordinator 合成。

这三处均是 **Stage 3+ candidate delta**，需架构师/用户后续讨论后才可落图。

---

# 7. 可验证里程碑建议

## M-VCP-Router

- 固定 30 条任务意图、5 个模型能力/成本/长度配置；
- 对比 semantic-only 与 hard-filter+semantic；
- 验证手动 override 100% 不被自动路由覆盖；
- typed failure 每类必须命中预期 fallback/reject。

## M-VCP-Plugin

- 同一 20 个工具以 native FC / structured JSON / VCP marker 三轨运行；
- 统计 parse success、误触发、token、延迟、schema violation；
- marker 只在 provider 不支持 FC 或显式兼容模式启用。

## M-VCP-Wave

- 至少 1k/10k/100k tag 三档；
- KNN baseline + 五阶段消融；
- 低可信 map 必须无损回退；
- 并发 100 请求不允许 energy field 串扰；
- 只有 Recall/nDCG 有统计显著提升且 P95/内存达标后，才升级为正式架构组件。

---

# 8. 源码证据索引

## 模型路由

- `SemanticModelRouter.json:1-67`
- `SemanticModelRouter.json.example:1-55`
- `modules/semanticModelRouter.js:157-227,292-405,409-503`
- `modelRedirectHandler.js:18-101`
- `modules/chatCompletionHandler.js:376-530,831-850,1036-1181`
- `docs/SEMANTIC_MODEL_ROUTER.md:7-16,30-111,273-279`

## 插件

- `Plugin.js:543-750,759-788,817-1170,1571-1628`
- `modules/vcpLoop/toolCallParser.js:1-268`
- `Plugin/SciCalculator/plugin-manifest.json:1-32`
- `Plugin/AgnesVideoGen/plugin-manifest.json:1-82`
- `Plugin/LightMemo/plugin-manifest.json:1-51`
- `knowledge/VCP百科全书/155_VCP文档_六大插件协议详解.txt:1-48`（文档旁证，数量以 manifest 为准）

## 浪潮

- `TagMemoEngine.js:197-239,301-449,681-899,901-1091,1189-1247`
- `ResidualPyramid.js:1-111,325-351`
- `KnowledgeBaseManager.js:15-24,901-976,978-1207`
- `Plugin/RAGDiaryPlugin/plugin-manifest.json:1-44`
- `Plugin/LightMemo/LightMemo.js:141-151,193-250,378-529,532-743`
- `routes/admin/rag.js:80-115,196-235`
- `adminServer.js:383-450`

## Apeireth 现有设计对照（只读）

- `docs/inspiration-stage1-2026-07-30.md:512-567,578-597`
- `docs/stage2-decisions-llm-integration.md:21-220,377-547`
- `docs/stage2-decisions-modularity.md:269-417`
- `docs/stage2-decisions-persistence.md:22-70,74-188`
- `docs/stage3-blueprints/01-overall-architecture.md:56-70,115-140`
- `docs/stage3-blueprints/README.md:23-40`

---

## 9. 最终边界声明

- 本轮只新增本研究报告。
- 未修改 VCP 源码、Rust 代码、Stage 2 决策、D1/D2/PREREQ、Stage 3 蓝图或主手册。
- 本报告的架构内容均为调研增量候选，不构成冻结决策。
- 对 GraphRAG/Zep/HNSW 的比较是范式级对照；若要做版本级选型，应另行进行同等级源码审计与 benchmark。
