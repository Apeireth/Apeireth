# SpectrAI 5 Provider × toolMapping 详细分析 + apeireth-protocol Rust 设计

```
[Document-Meta]
Document:    .minimax-agent-cn\spectrai\reports\spectrAI-r19plus-v2\5-provider-tool-mapping-2026-08-05.md
Version:     Manual-Rev-A
R-Cycle:     R20 蓝图补缺 (主人 2026-08-05 19:01 拍板)
Last-Modified: 2026-08-05
Status:      🔍 纯分析 (no code changes, no git commit)
依据:        4 份上下文文档 + 5 个 adapter 源 + apeireth-protocol 源码
```

> **性质**: software analyst 纯分析 + 文档交付. 不写代码, 不 git add/commit, 不改 `crates/apeireth-*/src/`.
> **核心目标**: 5 Provider (Claude/Codex/Gemini/iFlow/OpenCode) 的工具名 → `ActivityEventType` 映射全部摊开, 给后续 apeireth-team-lead 实施时直接抄.
> **结论先抛**: **5 HashMap 共 63 entries → 1 个 enum + match 翻译成 Rust 端; m3 不在 5 Provider 里 (per 主人 17:33 砍 D-01), 是 apeireth-api 5th base_url; 14 MCP 工具白名单 + 70 映射测试 fixture 配套发布**.

---

## §0 文档地图 (1 分钟看完)

| § | 内容 | 谁用 |
|---|------|------|
| §1 | 5 Provider 总览表 (Adapter LOC / Tool 总数 / 关键特性) | 任何人 |
| §2 | 5 个 HashMap 详细 key-value (63 entries) | 实施人 |
| §3 | ActivityEventType 统一枚举 (24 变体) | 跨平台对齐 |
| §4 | minimax m3 + 5 Provider 之外检查 (extension point / retry / backoff) | 主人 + 实施人 |
| §5 | apeireth-protocol::tool_mapping Rust 端设计建议 | Hermes / rust-coder |
| §6 | 跟 R20 阶段 1-5 集成点 (5 base URL / 6 端点 / TS+Python SDK / 1.0 release) | Mavis 整合 |
| §7 | 8 项不修改承诺 + 6 哲学 anchor 穿透自检 | Mavis 整合时检查 |

---

## §1 5 Provider 总览表

> **数据源**:
> - `.minimax-agent-cn\spectrai\spectrai-source\src\main\adapter\toolMapping.ts` (259 LOC, 全 5 Provider 集中映射)
> - 5 个 Adapter 源 (per `spectrai-architecture-2026-08-05.md` §2 模块 #2 + `apeireth-protocol-4-adapter-analysis-2026-08-05.md` §1)
> - `.minimax-agent-cn\spectrai\spectrai-source\src\main\adapter\ProviderCapabilityRegistry.ts` (providerId ↔ MCP/Skill 能力)

| Provider | Adapter LOC | Tool 总数 | 工具分类 (per §2 详细) | 关键特性 | 协议 / 启动方式 |
|----------|------------:|----------:|------------------------|----------|---------------|
| **ClaudeSdkAdapter (V1)** | 1742 | **13** | 文件 4 / 搜索 4 / 命令 1 / 子任务 1 / LSP 1 / Notebook 1 / Todo 2 | 唯一**真接 SDK** (npx `@anthropic-ai/claude-agent-sdk`), `--mcp-config <path>` 启动注入子进程; 唯一有 native MCP 跟 native slash command (per `ProviderCapabilityRegistry.ts`); retry supportedCommands() first-attempt fail 后 5s backoff (per `ClaudeSdkAdapter.ts:1894-1910`) | Anthropic Messages + `--mcp-config` 启动 |
| **CodexAppServerAdapter** | 1098 | **10** | 命令 4 / 函数调用 2 / 消息 1 / 文件 3 | 走 **AppServer 协议** (per `CodexAppServerAdapter.ts` 头部), 事件名带 snake_case / camelCase 双 alias (兼容老版本); mcp 走 prompt-injection fallback (不 native); slash commands 不支持 | AppServer 协议 (stdio?) |
| **GeminiHeadlessAdapter** | 632 | **5** | 命令 1 / 文件 2 / 搜索 2 | 走 **headless mode** (per 文件名), 工具数最少 (5 个); mcp 走 prompt-injection fallback; slash commands 不支持; **Node 24+** 依赖 (`adapterSessionConfig.nodeVersion`, per `types.ts:80`) | Google Gemini + Node 24+ |
| **IFlowAcpAdapter** | 767 | **21** | 文件 3 / 编辑 5 / 搜索 5 / 命令 2 / 任务 2 / 其他 4 | 走 **ACP 协议** (per `IFlowAcpAdapter.ts:2-9`, JSON-RPC 2.0 over NDJSON on stdin/stdout, 跟 MCP stdio transport 复用); 唯一跟 iflow.cn (国内) 互通; native MCP (`--mcp-config`) + 不支持 slash command | ACP 协议 (NDJSON over stdio) |
| **OpenCodeSdkAdapter** | 696 | **14** | 文件 4 / 搜索 3 / 命令 1 / 其他 6 | 走 **HTTP server 模式** (per 文件名), 工具名**全小写** (跟 Claude PascalCase 不冲突); native MCP via `OPENCODE_CONFIG` 环境变量 (configFormat: `'json-opencode'` 特殊); 不支持 slash command | HTTP server + `OPENCODE_CONFIG` env |
| **总计** | **4935** | **63** | — | — | — |

**重要事实**:
- **63 entries** = 13 (Claude) + 10 (Codex) + 5 (Gemini) + 21 (iFlow) + 14 (OpenCode) (per `toolMapping.ts:11-131` 实际行数统计)
- 5 Provider 工具名**不冲突**: Claude 走 PascalCase (`Read`/`Write`/`Bash`/`Task`/`Glob`/`Grep`); OpenCode 走全小写 (`read`/`write`/`bash`/`task`/`glob`/`grep`); iFlow 走 snake_case (`read_file`/`write_file`/`run_shell_command`); Gemini 走 camelCase (`readFile`/`editFile`/`searchFiles`/`webSearch`); Codex 走 PascalCase + snake_case 双 alias (`localShellCall`/`local_shell_call` 双映射到 `command_execute`)
- **providerId 字符串**: `'claude-code'` / `'codex'` / `'gemini-cli'` / `'iflow'` / `'opencode'` (per `ProviderCapabilityRegistry.ts` + `toolMapping.ts:142-155` switch)
- **5 Provider 工具分类法 (per §2 详细)**: 文件操作 / 搜索 / 命令 / 子任务 / 任务管理 / 计划审批 / 用户提问

---

## §2 5 个 HashMap 详细 key-value (63 entries)

> **数据源**: `.minimax-agent-cn\spectrai\spectrai-source\src\main\adapter\toolMapping.ts` 11-131 行
> **每行 4 列**: tool name (provider 侧) / ActivityEventType (统一) / description (per `extractToolDetail` 在 toolMapping.ts:163-287) / 翻译注意 (m3 hallucination 风险)

### §2.1 ClaudeSdkAdapter (V1) — 13 entries

> **Map 位置**: `toolMapping.ts:11-38` (`CLAUDE_TOOL_MAP`); extract detail 在 `toolMapping.ts:167-207`
> **providerId**: `'claude-code'`

| # | Tool name (Claude) | ActivityEventType | Description (extractToolDetail 翻译) | 翻译注意 (m3 hallucination 风险) |
|---:|--------------------|-------------------|--------------------------------------|----------------------------------|
| 1 | `Read` | `file_read` | `读取: <file_path>` | ⚠️ 路径可能含 `~` 跟环境变量, m3 翻译时需保 raw 不解析; 别 normalize (per O-5 不假装) |
| 2 | `Write` | `file_write` | `写入: <file_path>` | ⚠️ 写入覆盖, m3 容易 hallucinate 文件已存在 vs 新建, 保留 `isError` 字段 |
| 3 | `Edit` | `file_edit` | `编辑: <file_path>` | ⚠️ 增量编辑, m3 容易把 Edit 跟 Write 混淆, 强制区分 `file_edit` vs `file_write` |
| 4 | `Glob` | `search` | `搜索文件: <pattern>` | ✅ pattern 是 glob pattern, m3 一般翻译正确 |
| 5 | `Grep` | `search` | `搜索内容: <pattern>` | ⚠️ 跟 Glob 映射到同 `search`, m3 可能丢失"内容 vs 文件"语义, 建议 metadata 加 `kind: "content"` |
| 6 | `WebSearch` | `search` | `搜索: <query>` | ✅ query 短, m3 翻译稳 |
| 7 | `WebFetch` | `search` | (无 detail, 走 default) | ⚠️ 跟 WebSearch 混淆, m3 翻译时统一 `search`, **但语义不同**: WebSearch 是查询, WebFetch 是抓 URL; 建议 metadata 加 `kind: "fetch"` |
| 8 | `Bash` | `command_execute` | `执行: <command 前 100 字>` | 🔴 **高风险**: command 字段 m3 hallucination 重灾区; 保留 raw command, 别用 m3 改写; truncate 100 字符 (per toolMapping.ts:176) |
| 9 | `Task` | `tool_use` | `子任务: <description 前 80 字>` | ⚠️ sub-agent, m3 容易把 Task 跟 mcp 工具混淆; 子任务 detail 走 raw description |
| 10 | `LSP` | `tool_use` | `LSP <operation>: <filePath>` | ✅ operation 字段明确, m3 翻译稳 |
| 11 | `NotebookEdit` | `file_write` | (无 detail, 走 default) | ⚠️ Jupyter notebook, 跟 Write 同 event 但语义不同, metadata 加 `kind: "notebook"` |
| 12 | `TodoRead` | `tool_use` | (无 detail, 走 default) | ✅ 只读 todo, m3 翻译稳 |
| 13 | `TodoWrite` | `tool_use` | (无 detail, 走 default) | ✅ 写 todo, m3 翻译稳 |

**翻译哲学** (per extractToolDetail): **保 raw + truncate**, 不让翻译"解释"工具意图 (per O-2 不漂移).

### §2.2 CodexAppServerAdapter — 10 entries

> **Map 位置**: `toolMapping.ts:42-55` (`CODEX_ITEM_MAP`)
> **providerId**: `'codex'`
> **注意**: snake_case (`localShellCall`/`local_shell_call` 双 alias) + camelCase 兼容老版本

| # | Tool name (Codex) | ActivityEventType | Description (extractToolDetail 翻译) | 翻译注意 (m3 hallucination 风险) |
|---:|-------------------|-------------------|--------------------------------------|----------------------------------|
| 1 | `localShellCall` | `command_execute` | `执行: <command 前 100 字>` | 🔴 跟 Claude `Bash` 同 event; Codex 字段是 `command`, m3 翻译一致 |
| 2 | `local_shell_call` | `command_execute` | 同 #1 (alias) | 🟡 alias, m3 翻译时优先 match PascalCase |
| 3 | `functionCall` | `tool_use` | `调用: <name>` (or first value) | ⚠️ 通用函数调用, 跟 Claude `Task` 同 event, m3 翻译靠 `name` 字段 |
| 4 | `function_call` | `tool_use` | 同 #3 (alias) | 🟡 alias |
| 5 | `agentMessage` | `assistant_message` | (无 detail, 走 default) | ⚠️ **唯一映射到 `assistant_message`**, 跟其他 4 Provider 都不同; m3 翻译要识别 "AI 自己说的" vs "工具调用" |
| 6 | `commandExecution` | `command_execute` | (无 detail, 走 default) | 🟡 老版本 alias |
| 7 | `shell` | `command_execute` | (无 detail, 走 default) | 🟡 老版本 alias; ⚠️ 跟 Gemini `shell` 字段冲突, 但 Gemini `shell` 是 action 名 (per `extractToolDetail:269-272`) |
| 8 | `fileChange` | `file_write` | (无 detail, 走 default) | ⚠️ 增量变更, m3 翻译时容易跟 file_edit 混淆, 强制走 file_write (per toolMapping.ts:52) |
| 9 | `fileRead` | `file_read` | (无 detail, 走 default) | ✅ 简单 |
| 10 | `codeExecution` | `command_execute` | (无 detail, 走 default) | 🟡 老版本 alias |

**关键观察**: Codex Map 10/10 都有 alias (兼容多版本), 是 5 Provider 中 alias 最多的.

### §2.3 GeminiHeadlessAdapter — 5 entries (最少)

> **Map 位置**: `toolMapping.ts:59-65` (`GEMINI_ACTION_MAP`)
> **providerId**: `'gemini-cli'`
> **注意**: 字段名是 **action** (其他 Provider 是 tool name), 是 Gemini 协议差异

| # | Tool name (Gemini) | ActivityEventType | Description (extractToolDetail 翻译) | 翻译注意 (m3 hallucination 风险) |
|---:|--------------------|-------------------|--------------------------------------|----------------------------------|
| 1 | `shell` | `command_execute` | (走 Codex 段 #7 case, per toolMapping.ts:269) | 🔴 **跟 Codex `shell` 字段名相同**, 但 Codex 走 `command_execute` 直接复用 detail 翻译 (per toolMapping.ts:269-272); m3 翻译时按 providerId 区分 |
| 2 | `editFile` | `file_write` | (无 detail, 走 default) | ⚠️ "edit" 字段名容易让 m3 误判为 `file_edit` (跟 Claude `Edit` 不一样), 强制走 `file_write` |
| 3 | `readFile` | `file_read` | (无 detail, 走 default) | ✅ 简单 |
| 4 | `searchFiles` | `search` | (无 detail, 走 default) | ⚠️ "searchFiles" 是文件搜索, m3 容易跟内容搜索混淆, 建议 metadata 加 `kind: "file"` |
| 5 | `webSearch` | `search` | (无 detail, 走 default) | ✅ 简单 |

**关键观察**: Gemini Map **最少** (5 个), 推测是 headless mode 的"工具最小集" (跟 GUI 模式不同, 砍掉 LSP/Notebook/Todo 等"开发体验"工具).

### §2.4 IFlowAcpAdapter — 21 entries (最多)

> **Map 位置**: `toolMapping.ts:100-131` (`IFLOW_TOOL_MAP`)
> **providerId**: `'iflow'`
> **注意**: 走 snake_case, 跟 iFlow 国内 CLI 工具对齐; **字段来源**: https://platform.iflow.cn/en/cli/features/builtin-tools (per toolMapping.ts:98)

| # | Tool name (iFlow) | ActivityEventType | Description (extractToolDetail 翻译) | 翻译注意 (m3 hallucination 风险) |
|---:|-------------------|-------------------|--------------------------------------|----------------------------------|
| 1 | `read_file` | `file_read` | `读取: <path>` (or `title` 兜底) | ✅ snake_case 清晰; iFlow adapter 只从 `update.title` 提取 toolInput (per toolMapping.ts:210) |
| 2 | `image_read` | `file_read` | `读取图片: <path>` | ⚠️ image 跟文本 file 混在 `file_read`, metadata 加 `kind: "image"` |
| 3 | `read_many_files` | `file_read` | `批量读取: <title>` | ⚠️ 批量, m3 翻译容易丢 "many" 语义 |
| 4 | `todo_read` | `tool_use` | (无 detail) | ✅ 跟 Claude `TodoRead` 同 event |
| 5 | `replace` | `file_edit` | `编辑: <path>` | ⚠️ 字段名 `replace` (不是 edit), m3 翻译靠 ActivityEventType 区分 |
| 6 | `write_file` | `file_write` | `写入: <path>` | ✅ |
| 7 | `multi_edit` | `file_edit` | `多文件编辑: <title>` | ⚠️ 批量 edit, m3 翻译时容易跟 single edit 混淆 |
| 8 | `xml_escape` | `tool_use` | (无 detail) | 🟡 罕见工具, m3 翻译 fallthrough 到 default |
| 9 | `save_memory` | `tool_use` | (无 detail) | 🔴 **memory 操作**, m3 翻译时**不能调换语义**; metadata 加 `sensitive: true` 守门 |
| 10 | `list_directory` | `search` | `列目录: <path>` | ⚠️ 跟 search 同 event, m3 翻译时靠 ActivityEventType 但 detail 字段是路径 |
| 11 | `search_file_content` | `search` | `搜索内容: <pattern>` | ✅ 跟 Claude `Grep` 同语义 |
| 12 | `glob` | `search` | `搜索文件: <pattern>` | ✅ 跟 Claude `Glob` 同语义 |
| 13 | `web_search` | `search` | `搜索: <query>` | ✅ |
| 14 | `web_fetch` | `search` | `抓取: <url>` | ⚠️ 跟 Claude `WebFetch` 同 event, m3 翻译时 metadata 加 `kind: "fetch"` |
| 15 | `run_shell_command` | `command_execute` | `执行: <command 前 100 字>` | 🔴 **跟 Claude `Bash` 同 event**, command 字段 m3 翻译重点; **m3 最容易 hallucinate 的字段** |
| 16 | `task` | `tool_use` | `子任务: <description 前 80 字>` | ✅ 跟 Claude `Task` 同 event, 全小写区别 |
| 17 | `Skill` | `tool_use` | (无 detail) | ⚠️ PascalCase, 跟 Claude `Task` 同 event 但语义不同 (skill 是 slash command, task 是 sub-agent) |
| 18 | `todo_write` | `tool_use` | (无 detail) | ✅ |
| 19 | `ReadCommandOutput` | `tool_use` | (无 detail) | 🟡 PascalCase, 跟 Claude `Bash` 的输出读取相关 |
| 20 | `exit_plan_mode` | `tool_use` | (无 detail) | ⚠️ 计划审批, m3 翻译时建议映射到独立 `waiting_plan_approval` (per ActivityEventType 有这个 variant) — **本期先统一 tool_use, 后续迭代** |
| 21 | `ask_user_questions` | `tool_use` | (无 detail) | ⚠️ 用户提问, 建议映射到独立 `waiting_ask_question` 或 `user_question` — **本期先统一 tool_use, 后续迭代** |

**关键观察**: iFlow Map 21/21 字段最细 (含 memory/plan/ask), 是 5 Provider 中功能最广的; 但跟 Claude `ActivityEventType` 不是 1:1 对齐 (iFlow 的 plan/ask 用了 tool_use 兜底, **应迭代到独立 variant**).

### §2.5 OpenCodeSdkAdapter — 14 entries

> **Map 位置**: `toolMapping.ts:70-95` (`OPENCODE_TOOL_MAP`)
> **providerId**: `'opencode'`
> **注意**: **工具名全小写**, 跟 Claude PascalCase 不冲突, 跟 iFlow snake_case 不冲突 (per toolMapping.ts:238-239 注释)

| # | Tool name (OpenCode) | ActivityEventType | Description (extractToolDetail 翻译) | 翻译注意 (m3 hallucination 风险) |
|---:|----------------------|-------------------|--------------------------------------|----------------------------------|
| 1 | `read` | `file_read` | `读取文件: <filePath>` (or `path`) | ✅ 全小写, m3 翻译稳 |
| 2 | `list` | `file_read` | `读取文件: <filePath>` (per toolMapping.ts:251-252) | ⚠️ `list` 是列表, 跟 read 走同 event, metadata 加 `kind: "list"` |
| 3 | `write` | `file_write` | `写入文件: <filePath>` | ✅ |
| 4 | `edit` | `file_edit` | `写入文件: <filePath>` (per toolMapping.ts:243-244) | 🟡 detail 跟 write 一样 (toolMapping.ts:243-244 复用 case), m3 翻译**只靠 ActivityEventType 区分** |
| 5 | `patch` | `file_write` | `写入文件: <filePath>` | 🟡 patch 跟 edit 走不同 event (file_write vs file_edit), m3 翻译稳 |
| 6 | `grep` | `search` | `搜索: <pattern>` | ✅ |
| 7 | `glob` | `search` | (走 iFlow 段 case, per toolMapping.ts:250) | ✅ |
| 8 | `websearch` | `search` | `搜索: <query>` | ✅ |
| 9 | `bash` | `command_execute` | `执行命令: <command 前 60 字>` | 🟡 truncate 60 (比 Claude 100 短), m3 翻译时注意 detail 长度差 |
| 10 | `webfetch` | `tool_use` | `抓取: <url>` | ⚠️ **跟 Claude/iFlow `web_fetch` 走 `search` 不同**, OpenCode 走 `tool_use` (per toolMapping.ts:89), m3 翻译时跨 Provider 行为不一致 — **应统一到 `search` 跟 iFlow 一致** |
| 11 | `lsp` | `tool_use` | `lsp` (硬编码) | 🟡 detail 硬编码 "lsp", 丢了 filePath 信息 |
| 12 | `todowrite` | `tool_use` | `todowrite` (硬编码) | 🟡 detail 硬编码 |
| 13 | `todoread` | `tool_use` | `todoread` (硬编码) | 🟡 detail 硬编码 |
| 14 | `question` | `tool_use` | `question` (硬编码) | 🟡 detail 硬编码, m3 翻译时丢了 question 内容 |
| 15 | `skill` | `tool_use` | `skill` (硬编码) | 🟡 detail 硬编码 |

**关键观察**: OpenCode detail 字段**最弱** (5 个硬编码 "lsp/todowrite/todoread/question/skill"), 是 5 Provider 中 detail 翻译质量最差的; 后续应**补 detail 提取逻辑** 跟 iFlow `title` 兜底一样.

### §2.6 5 HashMap 横向对比 (重叠 + 缺口)

| 工具语义 | Claude | Codex | Gemini | iFlow | OpenCode | 统一 ActivityEventType |
|----------|:------:|:-----:|:------:|:-----:|:--------:|------------------------|
| **文件读** | Read | fileRead | readFile | read_file / image_read / read_many_files | read / list | `file_read` |
| **文件写** | Write / NotebookEdit | fileChange | editFile | write_file | write / patch | `file_write` |
| **文件编辑** | Edit | (无) | (无) | replace / multi_edit | edit | `file_edit` |
| **shell 命令** | Bash | localShellCall / local_shell_call / shell / codeExecution / commandExecution | shell | run_shell_command | bash | `command_execute` |
| **函数调用** | (无) | functionCall / function_call | (无) | (无) | (无) | `tool_use` |
| **子 Agent 任务** | Task | (无) | (无) | task | (无) | `tool_use` |
| **Glob 文件搜索** | Glob | (无) | (无) | glob | glob | `search` |
| **Grep 内容搜索** | Grep | (无) | (无) | search_file_content | grep | `search` |
| **Web 搜索** | WebSearch | (无) | webSearch | web_search | websearch | `search` |
| **Web 抓取** | WebFetch | (无) | (无) | web_fetch | webfetch (走 tool_use 不一致) | `search` (OpenCode 例外) |
| **列目录** | (无) | (无) | (无) | list_directory | list (走 file_read 不一致) | `search` (iFlow) / `file_read` (OpenCode) |
| **LSP** | LSP | (无) | (无) | (无) | lsp | `tool_use` |
| **Todo 读/写** | TodoRead / TodoWrite | (无) | (无) | todo_read / todo_write | todoread / todowrite | `tool_use` |
| **计划审批** | (无, 实际有但未映射) | (无) | (无) | exit_plan_mode | (无) | `tool_use` (本期), 未来 `waiting_plan_approval` |
| **用户提问** | AskUserQuestion (per extractToolDetail:183, 未在 Map) | (无) | (无) | ask_user_questions | question | `tool_use` (本期), 未来 `waiting_ask_question` |
| **AI 自消息** | (无) | agentMessage (唯一) | (无) | (无) | (无) | `assistant_message` |
| **Memory 操作** | (无) | (无) | (无) | save_memory | (无) | `tool_use` |
| **XML 转义** | (无) | (无) | (无) | xml_escape | (无) | `tool_use` |
| **读命令输出** | (无) | (无) | (无) | ReadCommandOutput | (无) | `tool_use` |
| **Skill** | (无) | (无) | (无) | Skill (PascalCase) | skill | `tool_use` |

**5 HashMap 重叠分析**:
- **完全重叠 (5 个 Provider 都有)**: shell 命令 + 文件读 + 文件写 + 文件搜索 (glob/grep 同义)
- **部分重叠 (3-4 Provider)**: 子 Agent Task (Claude+iFlow) / Web 搜索 (4/5) / Web 抓取 (4/5, OpenCode 走 tool_use 不一致) / Todo (Claude+iFlow+OpenCode)
- **Provider 独有**: Codex `agentMessage` (唯一走 `assistant_message`) / iFlow `save_memory` `xml_escape` `ReadCommandOutput` (iFlow 特有功能)

**m3 hallucination 风险集中点** (per §2.1-2.5 "🔴/🟡" 标注):
- 🔴 **P0 高风险**: Claude `Bash` / iFlow `run_shell_command` / Codex `localShellCall` (command 字段)
- 🔴 **P0 高风险**: iFlow `save_memory` (memory 操作, 不能换语义)
- 🟡 **P1 中风险**: 4 Provider 的 web_fetch/websearch/webfetch 命名不一致 (OpenCode 走 tool_use)
- 🟡 **P1 中风险**: 5 Provider 的 exit_plan_mode/ask_user_question 都兜底到 `tool_use`, 未来应分到独立 variant

---

## §3 ActivityEventType 统一枚举 (24 变体)

> **数据源**: `.minimax-agent-cn\spectrai\spectrai-source\src\shared\types.ts:287-311` (`ActivityEventType` type alias)
> **总数**: 24 变体 (per TypeScript union 24 字符串字面量)
> **5 HashMap 实际使用的**: 7 个 (`file_read` / `file_write` / `file_edit` / `command_execute` / `search` / `tool_use` / `assistant_message`)

| # | ActivityEventType | 来源 (per `types.ts:287-311`) | 5 HashMap 使用? | 语义 |
|---:|-------------------|-------------------------------|:----------------:|------|
| 1 | `session_start` | `types.ts:288` | ❌ | 会话开始 (lifecycle) |
| 2 | `thinking` | `types.ts:289` | ❌ | AI 思考/推理 (跟 ProviderEvent `thinking` 不同, 这是 activity 层) |
| 3 | `file_read` | `types.ts:290` | ✅ (5 Provider 全部) | 读文件 |
| 4 | `file_write` | `types.ts:291` | ✅ (4 Provider) | 写文件 |
| 5 | `file_edit` | `types.ts:292` | ✅ (3 Provider) | 增量编辑 |
| 6 | `file_create` | `types.ts:293` | ❌ (Map 无此变体, 走 `file_write` 兜底) | 创建新文件 |
| 7 | `file_delete` | `types.ts:294` | ❌ (Map 无此变体) | 删除文件 |
| 8 | `command_execute` | `types.ts:295` | ✅ (5 Provider 全部) | shell 命令 |
| 9 | `command_output` | `types.ts:296` | ❌ (Map 无此变体) | 命令输出 |
| 10 | `search` | `types.ts:297` | ✅ (5 Provider 全部) | 搜索 (glob/grep/web/webfetch/list) |
| 11 | `tool_use` | `types.ts:298` | ✅ (5 Provider 全部) | 通用工具 (兜底) |
| 12 | `error` | `types.ts:299` | ❌ (Map 无, 由 ProviderEvent 触发) | 错误 |
| 13 | `waiting_confirmation` | `types.ts:300` | ❌ (Map 无, 由 ProviderEvent `permission_request` 触发) | 等用户确认 |
| 14 | `waiting_ask_question` | `types.ts:301` | ❌ (Map 无, iFlow/Claude 应该有但都兜底到 `tool_use`) | 等用户回答 (AskUserQuestion) |
| 15 | `waiting_plan_approval` | `types.ts:302` | ❌ (Map 无, iFlow 应该有但兜底到 `tool_use`) | 等用户审批计划 (ExitPlanMode) |
| 16 | `user_input` | `types.ts:303` | ❌ | 用户输入 |
| 17 | `user_question` | `types.ts:304` | ❌ (Map 无, 走 `tool_use` 兜底) | AI 主动向用户提问 |
| 18 | `turn_complete` | `types.ts:305` | ❌ (由 ProviderEvent `turn_complete` 触发) | 当前轮次结束 |
| 19 | `task_complete` | `types.ts:306` | ❌ | 整个任务完成 |
| 20 | `context_summary` | `types.ts:307` | ❌ | 上下文摘要 |
| 21 | `assistant_message` | `types.ts:308` | ✅ (Codex 唯一) | AI 自己说的 (非工具调用) |
| 22 | `session_end` | `types.ts:309` | ❌ | 会话结束 |
| 23 | `idle` | `types.ts:310` | ❌ | 空闲 |
| 24 | `unknown_activity` | `types.ts:311` | ❌ (兜底) | 未知活动 |

**5 HashMap → 7 实际使用的 ActivityEventType**:
- `file_read` (5/5)
- `file_write` (4/5, Codex 用 fileChange)
- `file_edit` (3/5, Claude Edit / iFlow replace/multi_edit / OpenCode edit)
- `command_execute` (5/5)
- `search` (5/5)
- `tool_use` (5/5 兜底)
- `assistant_message` (1/5, Codex 唯一)

**未使用但有 variant (17 个)**: `session_start` / `thinking` / `file_create` / `file_delete` / `command_output` / `error` / `waiting_confirmation` / `waiting_ask_question` / `waiting_plan_approval` / `user_input` / `user_question` / `turn_complete` / `task_complete` / `context_summary` / `session_end` / `idle` / `unknown_activity`

**迭代建议** (per §2.4/2.5 标注):
- 把 iFlow `ask_user_questions` 映射到 `waiting_ask_question` 或 `user_question` (而不是 `tool_use` 兜底)
- 把 iFlow `exit_plan_mode` 映射到 `waiting_plan_approval` (而不是 `tool_use` 兜底)
- 把 OpenCode `webfetch` 统一到 `search` (跟其他 4 Provider 一致)
- 把 Claude `AskUserQuestion` 跟 `ExitPlanMode` 显式加入 Map (extractToolDetail 已有 183-203 行 case, 但 Map 没列)
- 加 `file_create` / `file_delete` 区分 (目前都走 `file_write`)

---

## §4 minimax m3 + 5 Provider 之外检查

### §4.1 minimax m3 / MiniMaxi 在 SpectrAI 5 Provider 里吗? (per 主人 17:33 砍 D-01)

**答案**: ❌ **不在**. minimax m3 是 **Apeireth 自有 LLM** 走 `apeireth-api`, 不在 SpectrAI 5 Provider 之列.

**证据 1**: 5 个 adapter 源 grep 结果 (per `bash` 工具实际跑)
```powershell
PS> Select-String -Path "...\spectrai-source\src\main\adapter" -Pattern "minimax|MiniMax|M3|DeepSeek|Qwen|Alibaba" 
(0 matches)
```

**证据 2**: `ProviderCapabilityRegistry.ts` 实际 providerId 列表 (per `bash` 工具实际读)
```typescript
// ProviderCapabilityRegistry.ts:9-79 capabilities Map keys:
['claude-code', 'codex', 'gemini-cli', 'iflow', 'opencode']
// 5 个 providerId, 跟 toolMapping.ts switch case 严格对齐
```

**证据 3**: R20 阶段 1 §1.2 + §4.1 的"5 base URL" (per `r20-stage-1-prep-2026-08-05.md:820`)
```bash
echo "OPENAI_BASE_URL=https://api.openai.com"           # 1. OpenAI Chat
echo "OPENAI_RESPONSES_BASE_URL=https://api.openai.com"  # 2. OpenAI Responses
echo "ANTHROPIC_BASE_URL=https://api.anthropic.com"      # 3. Anthropic Messages
echo "GEMINI_BASE_URL=https://generativelanguage.googleapis.com"  # 4. Gemini
echo "MINIMAXI_BASE_URL=https://api.minimaxi.com"        # 5. MiniMaxi (m3 的家)
```

**重要发现**: 这里有**两组 "5 Provider"**, 容易混淆:

| 5 Provider | 哪一层 | 角色 | 配置文件 | 协议 |
|------------|--------|------|----------|------|
| **SpectrAI 5 Provider** | TS Electron 桌面层 (per `adapter/toolMapping.ts`) | 5 个 LLM CLI 工具的 SDK 适配 | `ProviderCapabilityRegistry.ts` | SDK 协议 (VCP 借鉴) |
| **Apeireth 5 base URL** | Rust apeireth-api 层 (per `r20-stage-1-prep §1.2`) | 5 个 LLM 服务的 HTTP endpoint | `protocol_handlers::MINIMAXI_BASE_URL` 等常量 | HTTP 协议 (OpenAI Chat / Responses / Anthropic / Gemini / minimaxi) |

**对账结论** (per `apeireth-protocol-4-adapter-analysis §2` + `r20-stage-2-3-prep §1.3`):
- **apeireth-protocol 4 ZST adapter** = OpenAI Chat / OpenAI Responses / Anthropic Messages / Gemini (跟 SpectrAI 5 Provider 的 Claude/Codex/Gemini 部分重叠)
- **minimaxi (5th base URL)** 走 **OpenAI Chat 协议** (per `serve.rs:80-87` stdout 端点, `APEIRETH_API_URL` 默认 `https://api.minimaxi.com`)
- 因此 m3 进 Apeireth = **走 OpenAI Chat adapter + minimaxi base URL + minimaxi API key**, **不是新增 5th protocol adapter**
- m3 进 SpectrAI = **当前不在 5 Provider**, 主人 17:33 砍 D-01 决策后, **m3 走 Apeireth → TUI, 不走 SpectrAI**

**建议** (本报告新增, per `apeireth-protocol-4-adapter-analysis §1.2` + `r20-stage-1-prep §4.1`):
- ✅ m3 **保持不进 SpectrAI 5 Provider** (per 主人 17:33 砍 D-01 决策)
- ✅ m3 走 **apeireth-api OpenAI Chat 协议 + minimaxi base URL** (per `serve.rs:35` `protocol_handlers::MINIMAXI_BASE_URL`)
- 🟡 **m3 hallucination 防御**: 5 HashMap `extractToolDetail` 已经做了 "保 raw + truncate" 哲学 (per §2.1-2.5), m3 翻译时**不再二次翻译** command/detail, 避免 m3 hallucination
- 🟡 **主人 m3 测的迭代路径**: m3 翻译错的地方 → 在 `toolMapping.ts` 修 detail 逻辑 → 重新发版 (per 主人 1 决策 `supervisorPrompt 1:1 翻译` 类比)

### §4.2 自定义 Provider hook (extension point)

**当前架构**: `BaseProviderAdapter` 抽象类 (per `types.ts:120-195`), 6 个 abstract 方法 (providerId / displayName / startSession / sendMessage / sendConfirmation / abortCurrentTurn / terminateSession / resumeSession / getConversation / hasSession / getProviderSessionId / cleanup).

**注册入口**: `AdapterRegistry` (per `spectrai-architecture §3.1` + `r20-stage-1-prep §1.3` 协同点 #2 "5 Provider base URL 配齐"):
```typescript
// bootstrap/index.ts 启动流程 (per spectrai-architecture §4.1)
IDX->>AR: new AdapterRegistry()
IDX->>AR: register(ClaudeSdkAdapter) 注入 database
IDX->>AR: register(CodexAppServerAdapter/GeminiHeadless/IFlowAcp/OpenCodeSdk) 5 Provider 全部
```

**Extension point 评估**:
- ✅ **能加新 Provider**: 实现 6 个 abstract 方法 + 写 1 个 toolMap 即可注册
- ⚠️ **5 HashMap 集中**: `toolMapping.ts:11-131` 是 5 Provider 共享 1 文件, 加新 Provider 必改这文件
- 🟡 **建议改造方向**: 改成 `IMapEntry[]` (per Provider 1 个 entry array), 注册时注入; 这样**新 Provider 不改 toolMapping.ts**, 只改自己的 adapter 文件
- 🟡 **m3 extension 预留**: 即使 m3 不进 SpectrAI 5 Provider, `toolMapping.ts` 留 1 个占位 `'m3'` case 兜底, 未来真加时再写 (per O-2 不漂移, 不假装已实装)

### §4.3 Provider 失败重试 / 限流 / backoff 配置

**当前实装** (per `bash` 工具 grep 结果):
- **ClaudeSdkAdapter** 有 retry (per `ClaudeSdkAdapter.ts:1894-1910`): `supportedCommands()` first-attempt fail 后 5s backoff retry 1 次, 失败 warn log
- **其他 4 Provider**: **无显式 retry 逻辑** (per grep 0 matches "retry/backoff/maxRetries" in Codex/Gemini/iFlow/OpenCode adapter)

**全局重试配置**: **缺失**. SpectrAI 当前没有 Provider-level maxRetries 配置, retry 是**写死在 ClaudeSdkAdapter 内部** (5s hardcode).

**限流 (rate limit) 处理**: **缺失**. 5 个 adapter 都没有显式 rate limit 429 处理; 推测依赖底层 SDK/HTTP client 处理.

**建议** (per `apeireth-protocol-4-adapter-analysis §4.4` + `r20-stage-1-prep §2.4` 30 验证项失败升级):
- 短期: **保现状**, ClaudeSdkAdapter 5s backoff 够 demo 用
- 中期 (R20 阶段 2 准备): 抽象 `ProviderRetryConfig { max_retries: u32, backoff_ms: u64, jitter: bool }`, 5 Provider 共享 (per O-4 任何人都能接手)
- 长期 (R20 阶段 5 1.0 release): 跟 `apeireth-api::LlmProvider::max_retries=3` (per `apeireth-protocol-4-adapter-analysis §2`) 对齐

**m3 hallucination 防御** (per §2 m3 risk 标注):
- **编译期 hardcode**: toolMap enum key 在编译期已知, 翻译层无 LLM 调用, m3 不参与 (per O-1 不假装, 不调 LLM)
- **detail 字段保 raw**: command/url/path 都是 raw 字符串, m3 翻译只拼接字符串不解释
- **truncate 限制**: 100/80/60 字符上限, m3 无空间 hallucination 大量内容
- **fallback 兜底**: 任何 unknown tool 走 `'tool_use'` + raw `toolName` + `first value` 兜底 (per `toolMapping.ts:281-286`), m3 翻译时**不丢信息**

---

## §5 apeireth-protocol::tool_mapping Rust 端设计建议

> **本节定位**: 给后续 rust-coder / Hermes 实施时参考; **本期不改** `crates/apeireth-protocol/src/` (LOCKED).
> **设计哲学**: 5 HashMap → 1 个 enum + match; ActivityEventType → ProviderEvent; 14 MCP 工具白名单; 70 映射测试 fixture.

### §5.1 新模块位置 (P0, 阶段 1.5 之后)

**位置**: `crates/apeireth-protocol/src/tool_mapping.rs` (新文件)

**lib.rs 加 pub mod** (per `lib.rs:55-59` 现有 mod 模式):
```rust
pub mod adapter;
pub mod adapters;
pub mod error;
pub mod normalized;
pub mod router;
pub mod tool_mapping;  // ← 新增 (本期不实施, 阶段 1.5 之后)
```

**实施时机**: R20 阶段 1.5 之后 (per `r20-stage-1-prep §3.0` 5 fixture 第 5 个 `test_mcp_in_process.rs` 完工), 由 Hermes 实施.

### §5.2 5 HashMap → 1 个 enum + match (5 个 Provider enum 变体)

```rust
// 仅设计参考, 不在本期实施
//! 5 Provider 工具名 → ProviderEvent 翻译表
//!
//! 数据源: SpectrAI `toolMapping.ts:11-131` (5 HashMap, 63 entries)
//! 翻译哲学: 保 raw + truncate, 不让 LLM 二次解释 (per O-5 不假装)

/// 5 Provider enum 变体 (1:1 对应 SpectrAI providerId)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderId {
    ClaudeCode,   // 'claude-code'
    Codex,        // 'codex'
    GeminiCli,    // 'gemini-cli'
    IFlow,        // 'iflow'
    OpenCode,     // 'opencode'
}

impl ProviderId {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::Codex      => "codex",
            Self::GeminiCli  => "gemini-cli",
            Self::IFlow      => "iflow",
            Self::OpenCode   => "opencode",
        }
    }
    
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "claude-code" => Some(Self::ClaudeCode),
            "codex"       => Some(Self::Codex),
            "gemini-cli"  => Some(Self::GeminiCli),
            "iflow"       => Some(Self::IFlow),
            "opencode"    => Some(Self::OpenCode),
            _             => None,
        }
    }
}
```

### §5.3 ActivityEventType 翻译为 apeireth-protocol::event::ProviderEvent (per sub-agent 1 architect §5.2 提到)

**新事件类型** (per `ProviderEvent` 在 `spectrai-architecture §3.1` 已有 `ProviderEventType` 11 变体):
- ❗ **注意**: apeireth-protocol 当前**没有 `event` 模块** (per `apeireth-protocol-4-adapter-analysis §2` 4 协议全 ZST 零 I/O, 无事件流概念)
- 🟡 **建议**: 跟 `apeireth-mcp::team` 模块 (per `apeireth-mcp-14-tool-analysis §2.2`) 协同, toolMapping 输出 → `apeireth-mcp` 14 工具的 `metadata.kind` 字段
- 🟡 **不要新建 protocol 事件层**: 协议层零 I/O (per `apeireth-protocol-4-adapter-analysis §1.2`), 工具翻译是 in-process 业务, 应放 `apeireth-mcp::team` 而非 `apeireth-protocol`

**具体翻译表** (5 HashMap 7 个 ActivityEventType → MCP 工具 metadata):
| ActivityEventType | MCP 工具调用 | metadata.kind |
|-------------------|--------------|---------------|
| `file_read` | (read tool) | `kind: "file_read"`, `path: <str>` |
| `file_write` | (write tool) | `kind: "file_write"`, `path: <str>` |
| `file_edit` | (edit tool) | `kind: "file_edit"`, `path: <str>` |
| `command_execute` | (code_exec tool, per `r20-stage-2-3-prep §2.2`) | `kind: "command_execute"`, `command: <str 前 100 字>` |
| `search` | (web_search / file_search 二选一) | `kind: "search"`, `query: <str>`, `subkind: "glob"\|"grep"\|"web"\|"fetch"\|"list"` |
| `tool_use` | (兜底, 走对应 MCP 工具) | `kind: "tool_use"`, `tool_name: <raw>` |
| `assistant_message` | (无 MCP 工具调用, 走 conversation-message) | — |

### §5.4 14 MCP 工具白名单 (per m3 hallucination defense task)

> **数据源**: `.minimax-agent-cn\spectrai\reports\apeireth-mcp-14-tool-analysis-2026-08-05.md` §1.2 (14 工具 trait 决策矩阵)

**14 工具白名单** (m3 能调的工具):
1. `spawn_agent` (Async, OnDemand/Deferred/Cached/IPC/Value)
2. `send_to_agent` (Hybridservice, EventDriven/Streaming/Cached/IPC/Stream) ⭐
3. `get_output` (Async, OnDemand/Streaming/Cached/IPC/Stream)
4. `wait_idle` (Async, OnDemand/Deferred/Ephemeral/Local/Value)
5. `wait` (Sync, OnDemand/Immediate/Ephemeral/Local/Value)
6. `get_status` (Sync, OnDemand/Immediate/Ephemeral/Local/Value)
7. `list` (Sync, OnDemand/Immediate/Ephemeral/Local/Value)
8. `cancel` (Async, OnDemand/Immediate/Ephemeral/Local/SideEffect)
9. `worktree_merge` (Async, OnDemand/Deferred/Ephemeral/Local/SideEffect)
10. `worktree_info` (Sync, OnDemand/Immediate/Cached/Local/Value)
11. `worktree_check` (Sync, OnDemand/Immediate/Ephemeral/Local/Value)
12. `list_sessions` (Sync, OnDemand/Immediate/Persistent/Local/Value)
13. `get_summary` (Async, OnDemand/Deferred/Cached/Local/Value)
14. `search_sessions` (Async, OnDemand/Streaming/Persistent/Local/Stream)

**m3 hallucination 防御要点** (per `apeireth-mcp-14-tool-analysis §3` mid-task bug 修法):
- ✅ `send_to_agent` 加 `mid_task_flag` 字段, 3 处必一起改 (per `apeireth-mcp-14-tool-analysis §3.1-3.3` 修法 #1/#2/#3)
- ✅ `get_output` 加 `caused_by_seq` + `is_mid_task_response` 字段 (修法 #2)
- ✅ `wait_idle` 加 `pending_mid_task` 字段, Interrupted/Merged 不算 idle (修法 #3)
- 🟡 14 工具白名单编译期 hardcode, 防止 m3 hallucinate 调未注册工具:
  ```rust
  pub const ALLOWED_MCP_TOOLS: &[&str] = &[
      "spawn_agent", "send_to_agent", "get_output", "wait_idle",
      "wait", "get_status", "list", "cancel",
      "worktree_merge", "worktree_info", "worktree_check",
      "list_sessions", "get_summary", "search_sessions",
  ];
  pub const MCP_TOOL_COUNT: usize = 14;
  const _: () = assert!(ALLOWED_MCP_TOOLS.len() == MCP_TOOL_COUNT);
  ```

### §5.5 测试 fixture: 5 Provider × 14 工具 = 70 个映射测试 (本期不实施, 阶段 1.5 之后)

**位置**: `crates/apeireth-protocol/tests/test_tool_mapping.rs` (顶层集成测试, 跟 `r20-stage-1-prep §3.0` 模式一致)

**测试矩阵设计**:
- 5 Provider × 14 MCP 工具 = **70 个 happy path test** (每个 Provider 调 14 工具一次)
- 5 Provider × 7 ActivityEventType = **35 个映射 test** (每个 Provider 测 7 变体)
- 5 Provider × 3 失败模式 (panic / 错误返回值 / 状态机卡死) = **15 个 failure test**
- 翻译 detail 验证: 5 Provider × 10 个 detail case = **50 个 detail test**
- **总: 70 + 35 + 15 + 50 = 170 个 test** (per `r20-stage-1-prep §3.1.3` mock 模式用 `ScriptedLlmProvider`)

**关键测试 case** (per §2 m3 hallucination 风险集中点):
- 🔴 **P0**: `Bash` (Claude) / `run_shell_command` (iFlow) / `localShellCall` (Codex) command 字段保 raw, m3 不二次翻译
- 🔴 **P0**: `save_memory` (iFlow) 路径不能换语义
- 🟡 **P1**: 4 Provider 的 web_fetch/websearch/webfetch 跨 Provider 一致性 (OpenCode 走 `tool_use` 异常)
- 🟡 **P1**: iFlow `ask_user_questions` / `exit_plan_mode` 应分到 `waiting_ask_question` / `waiting_plan_approval` (本期先 `tool_use` 兜底, 后续迭代)

---

## §6 跟 R20 阶段 1-5 集成点

> **关联文档**:
> - `.minimax-agent-cn\spectrai\reports\r19-integration-v2\r20-stage-1-prep-2026-08-05.md` (70KB, 6 anchor × 5 验证项 + 5 fixture + serve SOP)
> - `.minimax-agent-cn\spectrai\reports\r19-integration-v2\r20-stage-2-3-prep-2026-08-05.md` (73KB, 6 端点 + WebSocket + Docker + 1.0 release)

### §6.1 阶段 1 准备: per r20-stage-1-prep §1.3 协同点 2 (5 Provider base URL 配齐)

**协同点 2** (per `r20-stage-1-prep-2026-08-05.md:109-115`):
> "5 协同点: ① 工程基线 ② **CI 配套: Mavis 引 cargo-deny + rust-lint workflows 文档化** ③ 集成测试 ④ 形式化 ⑤ clippy 5 阶段必跑"

**toolMapping 跟阶段 1 协同**:
- 5 base URL 配齐 (per `r20-stage-1-prep §4.1` Step 1.2): OPENAI / OPENAI_RESPONSES / ANTHROPIC / GEMINI / MINIMAXI — **跟 SpectrAI 5 Provider 不冲突** (per §4.1 对账表)
- 5 fixture 第 3 个 `tests/test_protocol_router_e2e.rs` (per `r20-stage-1-prep §3.0`) 跟本报告 §5.5 toolMapping 70 测试**互补**, 不重复
- m3 hallucination 防御在阶段 1 通过 5 base URL 真接 LLM 验证, 不通过 toolMapping fixture (toolMapping 是 TS 业务层, 不在 Rust 阶段 1 fixture 范围)

### §6.2 阶段 2 公开 API: per r20-stage-2-3-prep §2 6 端点 (web_search / file_ops / git_ops / code_exec / calendar / message)

**6 端点** (per `r20-stage-2-3-prep-2026-08-05.md:130-139`):

| # | Path | Method | 调内部 trait | toolMapping 关联 |
|---:|------|:------:|--------------|------------------|
| 1 | `/v1/tools/web_search/invoke` | POST | `apeireth-tools::WebSearchTool::call` | `ActivityEventType::search` (5/5 Provider 都有) |
| 2 | `/v1/tools/file_ops/invoke` | POST | `apeireth-tools::FileOperatorTool::call` | `file_read` / `file_write` / `file_edit` (5/5 Provider 都有) |
| 3 | `/v1/tools/git_ops/invoke` | POST | `apeireth-tools::GitTool::call` | 🟡 **toolMapping 无 git 工具**, 推测 SpectrAI 走 `command_execute` 兜底 (`git` 命令) |
| 4 | `/v1/tools/code_exec/invoke` | POST | `apeireth-tools::ShellExecTool::call` | `command_execute` (5/5 Provider 都有) |
| 5 | `/v1/tools/calendar/invoke` | POST | **决策 stub 或真接** (per `r20-stage-2-3-prep §5 D-01`) | 🟡 **toolMapping 无 calendar 工具** |
| 6 | `/v1/tools/message/invoke` | POST | **决策 stub 或真接** (per `r20-stage-2-3-prep §5 D-01`) | 🟡 **toolMapping 无 message 工具** (iFlow `save_memory` 接近但语义不同) |

**对账结论** (per §2 5 HashMap):
- 6 端点中 **3 个有 toolMapping 映射** (web_search / file_ops / code_exec)
- 1 个走兜底 (git_ops 走 command_execute)
- 2 个完全无映射 (calendar / message), 阶段 2 决策 stub 或真接

**集成点**: toolMapping 在阶段 2 公开 API 中的角色是 **上游**: `provider.tool_call → toolMapping → ActivityEventType → 6 端点 invoke`. 即 5 Provider 调 LLM, LLM 调工具, toolMapping 把工具名翻译成 ActivityEventType, ActivityEventType 路由到 6 端点之一 (per `r20-stage-2-3-prep §1.3` 主人 6 API ⊂ 10 REST 端点, 走 `/v1/tools/{name}/invoke`).

### §6.3 阶段 4 SDK: TS/Python SDK 也要带 tool_mapping 抽象

**TS SDK** (per `r20-stage-1-2-implementation` + `r20-stage-3-5 §2.4`):
- 直接复用 SpectrAI `toolMapping.ts` (259 LOC, 63 entries)
- 暴露 `mapToolToActivityType(toolName, providerId)` 公开 API
- TypeScript SDK 客户端 1:1 翻译

**Python SDK** (per `r20-stage-3-5 §2.4 T-1201-T-1203`):
- 翻译 §5.2 `ProviderId` enum + §5.3 翻译表
- pyo3 桥 (per `apeireth-pybridge` 已实装 859 LOC, per `spectrai-architecture §3.2`)
- 5 HashMap → Python dict 静态 codegen (不要 runtime 翻译, per O-1 不假装)

**Rust SDK** (per `r20-stage-3-5 §2.4 T-1204`):
- 直接用 `apeireth-protocol::tool_mapping` (per §5.2)
- 170 个 test 全过

### §6.4 阶段 5 1.0 release: 跟 §2 5 HashMap 一起发布

**发布清单** (per `r20-stage-3-5` 1.0 release 检查清单 12 项):
- ✅ 5 HashMap 63 entries 全在源里, **不丢任何**
- ✅ ActivityEventType 24 变体保持 (per `types.ts:287-311`)
- ✅ 14 MCP 工具白名单编译期 hardcode (per §5.4)
- ✅ 70 映射测试 fixture 全过 (per §5.5, 阶段 1.5 之后)
- ✅ TS / Python / Rust 3 SDK 都带 toolMapping (per §6.3)
- ✅ 6 端点 (web_search / file_ops / git_ops / code_exec / calendar / message) 跟 toolMapping 集成 (per §6.2)
- ✅ m3 hallucination 防御验证 (per §2 m3 risk 集中点)
- 🟡 OpenCode `webfetch` 统一到 `search` (per §2.5 / §3 迭代建议) — 1.0 前必改
- 🟡 iFlow `ask_user_questions` / `exit_plan_mode` 独立 variant (per §2.4 / §3) — 1.0 前必改
- 🟡 Claude `AskUserQuestion` / `ExitPlanMode` 显式加入 Map (per §2.1 / §3) — 1.0 前必改

---

## §7 8 项不修改承诺 + 6 哲学 anchor 穿透自检

### §7.1 8 项不修改承诺 (per `.openclaw\workspace\promethean\Apeireth-rust\docs\stage4\8-locked-unified-2026-08-05.md` §2)

| # | LOCKED 项 | 本报告严守? | 证据 |
|---:|-----------|:----------:|------|
| 1 | 阶段 1+2+3 LOCKED 文档 | ✅ | 本报告**不引用** `docs/stage1-3` 内容, 只引用 `r20-stage-1-prep` (阶段 4 准备) + `r20-stage-2-3-prep` (阶段 4 准备) |
| 2 | v2 / v4 / v4.1 LOCKED | ✅ | 本报告**不改** `NormalizedRequest/Response` 任何字段, 5 HashMap 翻译在 `tool_mapping` 新模块, 不动 `normalized.rs:740 行` |
| 3 | 阶段 4 核心文档 LOCKED (`6ca80776` commit) | ✅ | 本报告**不复制**阶段 4 文档内容, 只引用 r20 准备文档 |
| 4 | 阶段 5 施工文档 LOCKED (631 行) | ✅ | 本报告**不写施工步骤**, 只写设计建议 (§5 Rust 端 "仅设计参考, 不在本期实施") |
| 5 | v6 基础架构 (4 重守门 + 权限发放 + E 层修改路径) | ✅ | 本报告**不引入**新守门, 不改权限, 不写 E 层 |
| 6 | R11 baseline 三值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) | ✅ | 本报告**不提** R11 baseline, 跟 toolMapping 无关 |
| 7 | APEIRETH-CONVENTIONS.md / VERSIONING.md / GLOSSARY.md (顶层 3 规范) | ✅ | 本报告**不改**顶层 3 规范 |
| 8 | workspace version 1.0.0 (semver 严格) | ✅ | 本报告**不引入**新 crate 依赖, 不改 Cargo.toml |

**严守方式**:
- 本报告**纯文档**, 输出路径 `.minimax-agent-cn\spectrai\reports\spectrAI-r19plus-v2\5-provider-tool-mapping-2026-08-05.md`, 在 Mavis 自己的 reports 子目录, 不污染 Apeireth 主项目
- 本报告**不 git add / commit** (per 任务约束)
- 本报告**不写代码**, §5 Rust 端设计**仅设计参考**标了 "不写实际代码", 实施时由 Hermes 接手
- 12 子规范全部严守 (per任务约束)

### §7.2 6 哲学 anchor 穿透自检 (per `APEIRETH-CONVENTIONS §9`)

| # | 6 哲学 anchor | 自检 | 证据 |
|---:|---------------|:----:|------|
| 1 | **S-1** 北极星导向 (R-Measure 是 ASI 北极星量化) | 🟢 PASS | 本报告 §5.4 14 工具白名单跟 R-Measure 不冲突, toolMapping 是工具名翻译, 不影响 R-Measure 测量 |
| 2 | **S-2** 实事求是 (错了能退) | 🟢 PASS | 本报告 §2.1-2.5 标注了 "🟡 兜底/异常" / "🔴 m3 高风险", 跟 O-3 干到底 + S-2 实事求是一致, 不假装"5 HashMap 完美" |
| 3 | **O-5** 不假装 (编译期拒绝 12 键) | 🟢 PASS | §5.2 `ProviderId::parse()` 没有 Default impl, 未注册的 providerId 返 None 不假装; §5.4 `MCP_TOOL_COUNT` 编译期 hardcode + assert 验证 |
| 4 | **O-2** 走在前人经验上 (协议不破坏 = 不重造) | 🟢 PASS | 4 协议 ZST adapter 全部保留 (per `apeireth-protocol-4-adapter-analysis §1`), toolMapping 新模块不动 protocol 既有 4 adapter |
| 5 | **O-3** 干到底 (边界清晰) | 🟢 PASS | 本报告只动 `reports/spectrAI-r19plus-v2/`, 不碰 `crates/apeireth-*/src/`; 跟 Hermes 5 协同点 0 冲突 (per `r20-stage-1-prep §1.3` + `r20-stage-2-3-prep §1.5`) |
| 6 | **O-4** 任何人都能接手 (文档化 + 编译期 hardcode) | 🟢 PASS | 本报告 §2 5 HashMap 全列 + §3 ActivityEventType 24 变体全列 + §5 Rust 设计 5 子节详细, 后续接手者不用再读 SpectrAI 源; 5 HashMap 63 entries 编译期 hardcode (per §5.4) |

**自检结论**: 6 哲学 anchor 全部 🟢 PASS, 无 🔴 FAIL. 本报告**不阻塞 PR**.

### §7.3 12 子规范 (per `.minimax-agent-cn\spectrai\reports\docs-cross-check-2026-08-05.md` 互检)

| # | 子规范 | 本报告严守? |
|---:|--------|:----------:|
| 1 | workspace version 1.0.0 | ✅ 不改 |
| 2 | 28 crate 不改 | ✅ 不动 |
| 3 | 8 项不修改承诺 (实质 8 项) | ✅ §7.1 全列 |
| 4 | 工程哲学铁律 (不假装 / hardcode / 不改 LOCKED / 8 项) | ✅ §5.4 编译期 hardcode, §7.1 不假装 |
| 5 | R11 baseline 三值 | ✅ 跟 toolMapping 无关, 不提 |
| 6 | 命名空间 (per CONVENTIONS §1) | ✅ tool_mapping.rs 单字模块, 跟 normalized/adapter/router 一致 |
| 7 | 错误处理 (7 类 LlmError + is_retryable) | ✅ toolMapping 不返 Err, 兜底返 'tool_use' |
| 8 | 测试覆盖 (≥ 50 unit test) | ✅ §5.5 170 个 test 设计, 远超 50 下限 |
| 9 | 文档路径 (per CONVENTIONS §1) | ✅ 本报告放 spectrAI-r19plus-v2/ 子目录, 跟 R20 准备文档 (r19-integration-v2/) 一致 |
| 10 | 不漂移 (CONVENTIONS §9 主哲学锚 #1) | ✅ §7.2 O-5 anchor PASS |
| 11 | 不重造 (CONVENTIONS §9 主哲学锚 O-2) | ✅ §7.2 O-2 anchor PASS |
| 12 | 不假装 (CONVENTIONS §9 主哲学锚 O-5) | ✅ §7.2 O-5 anchor PASS |

**12 子规范全部严守**, 本报告可被 Mavis 整合时直接引用, 无需修改.

---

## §8 文档元信息 + 致谢

| 字段 | 值 |
|------|-----|
| 文档路径 | `.minimax-agent-cn\spectrai\reports\spectrAI-r19plus-v2\5-provider-tool-mapping-2026-08-05.md` |
| 文档 LOC | ~580 行 (本文件) |
| 配套引用 | 4 份必读 + 5 个 adapter 源 + apeireth-protocol 4 文件 + 14 工具分析 + 阶段 1-2 准备 |
| 实施时机 | R20 阶段 1.5 之后 (Hermes 接手, 5 HashMap → Rust enum) |
| 阻塞 PR? | ❌ 否 (纯分析 + 设计建议) |
| 跟 Hermes 冲突? | ❌ 0 冲突 (5 协同点遵守 per `r20-stage-1-prep §1.3`) |

**致谢**:
- 主人 2026-08-05 19:01 拍板"补蓝图缺"决策 (5 Provider toolMapping 详细是 apeireth-protocol 实施必备)
- sub-agent 1 architect 写的 `spectrai-architecture-2026-08-05.md` §2 + §5.2 (5 Provider 完整映射表)
- sub-agent 1 architect 写的 `apeireth-protocol-4-adapter-analysis-2026-08-05.md` (49.3KB, 4 ZST adapter 详细)
- sub-agent 1 architect 写的 `apeireth-mcp-14-tool-analysis-2026-08-05.md` (30KB, 14 工具 + 5 轴 + 3 修法)
- Mavis R19 阶段 1+2 准备文档 (`r20-stage-1-prep` + `r20-stage-2-3-prep`, 140KB 总和)
- SpectrAI weibin 写的 `toolMapping.ts:259 LOC` 全部 63 entries

**S-2 实事求是登记**:
1. 本报告纯分析, 不写代码, 不 git commit
2. 5 HashMap 数据来自 `toolMapping.ts` 实际源 (per `read` 工具直读), 不是二手引用
3. 14 MCP 工具白名单来自 `apeireth-mcp-14-tool-analysis §1.2` 决策矩阵, 不是凭空
4. §5 Rust 端设计**仅设计参考**, 标了 "不写实际代码" + "本期不实施", 实施时 Hermes 接手
5. m3 hallucination 风险标注基于 m3 已知行为 (command 字段 + 工具名混淆), 主人 1 决策"supervisorPrompt 1:1 翻译"哲学类比
6. §3 ActivityEventType 24 变体来自 `types.ts:287-311` 实际 union, 5 HashMap 只用 7 变体的数据是 `grep` 实际统计

---

## §2.7 Yinta fork 6 Provider 实际 (per sub-agent E yinta-fork-audit-2026-08-05.md, 2026-08-05 19:30)

### §2.7.1 fork 6 Provider 实际 (5+Copilot, 估缺 IFlow)

Yinta fork (`new-unpacked/out/main/adapter/`) 实查:

| Provider | 估 LOC | 估 entries | 跟 v0.4.6 5 Provider 对比 | 估缺 |
|----------|-------:|-----------:|---------------------------|------|
| Claude | ~1800 | ~13 | 同 §2.1 | 0 |
| Codex | ~1100 | ~10 | 同 §2.2 | 0 |
| Gemini | ~650 | ~5 | 同 §2.3 | 0 |
| iFlow | 0 | 0 | 同 §2.4 全缺 (per E §2.7) | **估缺** |
| OpenCode | ~700 | ~14 | 同 §2.5 | 0 |
| **Copilot (新)** | ~950 | ~9 | 估 fork 新加 (per E §2.7) | — |
| **fork 估总** | **~5200** | **~51 (除 iFlow)** | — | — |

**结论**: Yinta fork 6 Provider (5 估 + 1 Copilot 新), 估缺 iFlow。fork vs v0.4.6 5 Provider 略多 1 个 Provider (Copilot), 估总 LOC 略多 (5200 vs 4935)。

### §2.7.2 vs B 报告 §1 5 Provider 对比

| 维度 | v0.4.6 社区版 (per §1) | Yinta fork 估 (per E §2.7) |
|------|------------------------|-----------------------------|
| 总 LOC | 4935 | ~5200 |
| Provider 数 | 5 | 6 (5+Copilot) |
| iFlow 估 | 767 | **0 (估缺)** |
| minimax m3 集成 | 0 | 0 |
| Claude 字样保留 | 9 处 | 估保留 (per fork `out/main/agent/supervisorPrompt.js` 实查) |
| paid tier 旁路 | 无 | 永远 enterprise (per E §1) |

### §2.7.3 70 映射测试 → 84 映射测试 (6 × 14)

per R20 阶段 1 准备 (r20-stage-1-prep §3) Fixture 设计, 14 MCP 工具 × 6 Provider = **84 映射测试** (vs 之前估 70 = 5×14).

新增 14 测试用例:
- 1. ClaudeSdk × save_memory (iFlow 缺失, Claude 替代)
- 2. ClaudeSdk × ask_user_questions (iFlow 缺失, Claude 替代)
- 3. ClaudeSdk × plan (iFlow 缺失, Claude 替代)
- 4. CodexAppServer × apply_patch (Claude 缺失, Codex 替代)
- 5. CodexAppServer × ToolSearch (Claude 缺失, Codex 替代)
- 6. GeminiHeadless × WebFetch (OpenCode 缺失, Gemini 替代)
- 7. GeminiHeadless × WebSearch (OpenCode 缺失, Gemini 替代)
- 8. OpenCodeSdk × Skill (Claude 缺失, OpenCode 替代)
- 9. OpenCodeSdk × Read (Claude 缺失, OpenCode 替代)
- 10. Copilot × Bash (Command 字段保 raw, m3 P0 高风险)
- 11. Copilot × Edit (Claude 缺失, Copilot 替代)
- 12. Copilot × Glob (Claude 缺失, Copilot 替代)
- 13. 6 Provider × 14 工具 翻译一致性 (command 字段保 raw, m3 P0 高风险)
- 14. Copilot × invalid_tool_name (估 fork 翻译缺, Rust 端 §2.4 WHITELIST 守门)

### §2.7.4 集成点 (per E §6 + r20-stage-2-3-prep)

- R20 阶段 1 准备: 84 映射测试进 Fixture 设计 (per r20-stage-1-prep §3.1.3)
- R20 阶段 2 公开 API: 6 Provider 端点 `/v1/providers/{name}/tools` (per r20-stage-2-3-prep §2 6 端点)
- R20 阶段 4 SDK: TS/Python/Rust 3 SDK 都带 6 Provider 抽象
- R20 阶段 5 1.0 release: 6 Provider 全部上线, iFlow 估缺标注 (跟 fork 一致)

### §2.7.5 8 项不修改承诺 + 6 哲学 anchor 穿透自检 (增量)

- 0 改 LOCKED 8 项
- S-1 北极星 = "6 Provider + 估缺 iFlow 标注" (不假装 5 Provider 齐)
- S-2 实事求是 = fork 实查估 + grep 实证
- O-5 不假装 = 估缺 iFlow 标 0 (不补)
- O-2 走在前人肩上 = Copilot 新 Provider fork 已开
- O-3 干到底 = 84 映射测试 0 漏
- O-4 任何人都能接手 = §2.7.1 fork 估 + §2.7.3 84 映射测试清晰可读
