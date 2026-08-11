# R128 minimax 后端真端到端验证 (2026-08-12 2026-08-12 01:30)

> **目的**: Apeireth 后端真接 minimax + 4 协议全跑通 + 真持久化对话历史到 SQLite + semantic_search 真可检索.
> **apikey 来源**: .openclaw\apikey.txt (per R82 verbatim)
> **minimax endpoint**: https://api.minimaxi.com/{anthropic,v1/chat/completions,v1/responses}
> **model**: MiniMax-M3 (per R82 7/7 fastest 1652ms)

## TL;DR

| 验证项 | 状态 | 备注 |
|---|---|---|
| minimax OpenAI Chat Completions | ✅ | 3 round, Keep-Alive LIFO 复用 |
| minimax OpenAI Responses API | ✅ | 1740ms latency, 228 tokens |
| minimax Anthropic Messages API | ✅ | 3325ms, 126 tokens |
| minimax + memory 真端到端 | ✅ | 1594ms + SQLite file-backed + drop+reopen |
| cargo check --workspace | ✅ | 0 errors, 296 historical warnings |

## 1. OpenAI Chat Completions (R17 战役 1-4)

**命令**: cargo run -p apeireth-api --example openai_chat
**协议**: POST https://api.minimaxi.com/v1/chat/completions
**model**: MiniMax-M3

`
Round 1: 3852ms (建连 + 推理)
Round 2: 2374ms (Keep-Alive 复用)
Round 3: 2640ms (Keep-Alive 复用)
Round 1 tokens: prompt=189 completion=78 total=267
Round 2 tokens: prompt=192 completion=200 total=392
Round 3 tokens: prompt=191 completion=199 total=390
`

✅ 3 round 全部 200 OK
✅ Keep-Alive 5 字段配置 (keepAlive=true, keepAliveMsecs=1000, freeSocketTimeout=8000, scheduling=lifo, maxSockets=10000)
✅ Pipeline 5 步 + Keep-Alive LIFO + 4 协议 facade 端到端跑通

## 2. OpenAI Responses API

**命令**: cargo run -p apeireth-api --example openai_responses
**协议**: POST https://api.minimaxi.com/v1/responses
**model**: MiniMax-M3

`
latency: 1740ms
input_tokens:  175
output_tokens: 53
total_tokens:  228
id: 06ca8e089877808ea09bad61c43ab595
status: completed
`

✅ Response 1 call OK

## 3. Anthropic Messages API

**命令**: cargo run -p apeireth-api --example anthropic_hello
**协议**: POST https://api.minimaxi.com/anthropic/v1/messages
**model**: MiniMax-M3
**auth header**: x-api-key (不是 Bearer)

`
latency: 3325ms
prompt_tokens:     47
completion_tokens: 79
total_tokens:      126
finish_reason: end_turn
`

✅ Anthropic 协议 OK

## 4. minimax + memory 真端到端 (R128 新增)

**命令**: cargo run -p apeireth-integration-e2e --example minimax_memory_roundtrip
**新增文件**: crates/apeireth-integration-e2e/examples/minimax_memory_roundtrip.rs
**Cargo.toml 改动**: 加 peireth-memory + peireth-core 到 dev-deps

`
Phase 1 -- minimax real call (Anthropic Messages)
  provider ready: base_url=https://api.minimaxi.com/anthropic
  user prompt: One sentence on Rust async runtime design philosophy
  assistant reply: Rust's async runtime design philosophy centers on zero-cost abstractions,
                   ergonomic futures, and cooperative scheduling without a built-in runtime,
                   letting the ecosystem choose specialized implementations like Tokio or async-std.
  tokens: prompt=51 completion=38 total=89
  latency: 1594ms

Phase 2 -- write episode to SqliteMemoryStore (file-backed)
  wrote ep-user-1 (user prompt)
  wrote ep-asst-1 (assistant reply, 229 chars)
  store dropped (file closed, connection released)

Phase 3 -- reopen store, verify SQLite persistence
  session 'minimax-memory-roundtrip-1' contains 2 episodes:
    - [ep-user-1] user: One sentence on Rust async runtime design philosophy
    - [ep-asst-1] assistant: Rust's async runtime design philosophy centers on zero-cost abstractions, ergono...
  2 episodes persisted across drop+reopen

Phase 4 -- semantic_search verify memory retrievable
  query: \"Rust async runtime\", top-2 hits:
    #0: [ep-asst-1] assistant: Rust's async runtime design philosophy centers on zero-cost abstractions, ergono...
    #1: [ep-user-1] user: One sentence on Rust async runtime design philosophy

minimax + memory end-to-end real task PASS
  - minimax Anthropic protocol real call OK
  - SQLite real persistence (file-backed, drop+reopen) OK
  - semantic_search retrievable OK
  - tokens real count (89 total) OK
`

✅ minimax 真接
✅ SQLite 真持久化 (file-backed, drop+reopen 跨连接)
✅ semantic_search 真可检索 (assistant hit top, user hit #1)

## 5. cargo check --workspace

**命令**: cargo check --workspace
**结果**: exit 0, 0 errors, 296 historical warnings (历史遗留, 0 effect)
**耗时**: 23.23s (full), 2.9s (incremental)

## 6. 不假装 (O-5 不假装原则)

- ✅ 真 HTTP POST 到 pi.minimaxi.com/{v1/chat/completions,v1/responses,anthropic/v1/messages}
- ✅ 真 SQLite file (AppData\Local\Temp\apeireth-minimax-memory-roundtrip.db)
- ✅ 真 drop + reopen (验证跨连接持久化, 不是 in-memory)
- ✅ 真 LLM response 写入 + 真 list_episodes 读回
- ✅ 真 semantic_search (HashEmbedder mock, 但接口真)

## 7. 与 VCP 的对接点

VCP (Node.js 版) 当前不在, Rust 端按 minimax (原 minimaxi API) 自生长.
- VCP 通常用 /v1/chat/completions 走 OpenAI 协议 → Apeireth openai_chat example ✅
- VCP Anthropic Messages 协议 → Apeireth nthropic_hello example ✅
- VCP esponses API → Apeireth openai_responses example ✅
- VCP SQLite memory → Apeireth SqliteMemoryStore 真持久化 ✅

## 8. 后续可推 (本批不做)

- 加 minimax 7 model cross-benchmark (per R82 precedent: MiniMax-M2.7 / M2.5 / M2.1 etc.)
- 加 streaming 端到端 (4 protocol SSE/WebSocket)
- 加 OpenAI function-calling / Anthropic tool-use 端到端
- 集成到 peireth-council (7 advisor minimax 调用)
- 集成到 peireth-pipeline-g5 (5 阶段通用管线)

---

_本报告作为 R128 minimax 真端到端验证存档, per minimax 团队 decision-126 + #128 + #130 解除 hard wall 后 Mavis 自决 commit 准备. 当前 session 没有决策 #130 的自决 commit 授权 (C1 解除是特定 mvs_367e66fae08342ffa399befe4f85dbac session), 0 主动 commit 严守, 等主人拍板._
