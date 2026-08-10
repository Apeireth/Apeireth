# R123-2 Readmap — 4 协议 handler 抽 trait 抽象 (R122-10 重复模式 #1)

**Date**: 2026-08-10
**Coder**: R123-2 (R122-11 重派, 任务工具 14:42 Connection error 挂, Mavis 自干重派)
**Slot**: v2.1 抽 trait 抽象, R122-10 5 重复模式 #1
**借鉴 ID**: `R123-2-NEW-ProtocolHandlerTrait-2026-08-10` (0 VCP 借鉴, 自创骨架, per 07 §1 O-2)

---

## 1. 任务一句话

新建 `crates/apeireth-api/src/protocol_handler_trait.rs` (~150-200 行) 1 个 `ProtocolHandler` trait +
1 个 `HandlerRegistry` 注册表 + 1 个 `route_dispatch` 通用模板, 加 8+ unit test, 集成 1 行 mod
声明, examples 演示 4 协议注册 + dispatch。**0 触碰现有 4 协议 handler impl, 0 装已替换,
0 接入真 handler** (R123+ 续真接)。

---

## 2. 现状摸清 (8 项关键事实)

| # | 事实 | 路径:行 | 影响 |
|---|------|---------|------|
| 1 | workspace version 1.1.0 | `Cargo.toml:246` | 0 改 ✓ |
| 2 | R11 baseline 3 值 0.8682/0.8532/0.9063 | `apeireth-asi/tests/integration_r_measure.rs:42-44` | 0 触碰 ✓ |
| 3 | ProtocolKind 7 变体 (R30 U10) | `apeireth-protocol/src/gateway.rs:33-48` | 4 LLM 协议 (OpenAIChat/OpenAiResponses/AnthropicMessages/Gemini) + 3 gateway (Acp/Mcp/OpenClawGateway); 测试用 3 gateway 之一当"未注册 kind" |
| 4 | NormalizedRequest 字段 model/messages/temperature/max_tokens/stream/stop/tools/tool_choice/metadata | `apeireth-protocol/src/normalized.rs:474-501` | cache_key 基于 model + messages 即可稳定 |
| 5 | NormalizedResponse 字段 id/model/content/finish_reason/usage/tool_calls/raw_metadata | `apeireth-protocol/src/normalized.rs:538-556` | 模板 OK, `NormalizedResponse::text()` 工厂现成 |
| 6 | ProtocolKind: Hash + Eq + Copy | `apeireth-protocol/src/gateway.rs:32` | 可做 HashMap key ✓ |
| 7 | 现有 `dispatch()` 返 `Result<NormalizedResponse, String>` | `protocol_handlers.rs:837-839` | 本 trait 沿用 String (跟 1.0 行为 0 漂移, 0 假装 ProtocolError) |
| 8 | HashMap 已在 11 个文件用 | (grep 结果) | 0 新 dep ✓ |

---

## 3. 集成点选择 (决策)

**0 触碰现有 4 协议 handler impl**:
- ❌ 0 改 `protocol_handlers.rs` 任何 fn
- ❌ 0 改 `v2_endpoints.rs` 任何 fn
- ❌ 0 改 `server.rs` 4 endpoint
- ❌ 0 改 `lib.rs` 现有 mod 声明 (仅在末尾加 1 行 `pub mod protocol_handler_trait;`)

**1 新 mod + 1 lib.rs 行 + 1 example + 1 Cargo.toml 块**:
- `crates/apeireth-api/src/protocol_handler_trait.rs` (~150-200 行) — trait + registry + route_dispatch + 8 tests
- `crates/apeireth-api/src/lib.rs:1` — `pub mod protocol_handler_trait;` (末尾追加)
- `crates/apeireth-api/examples/protocol_handler_demo.rs` (~60 行) — 4 协议 stub 注册 + dispatch demo
- `crates/apeireth-api/Cargo.toml` — 加 1 `[[example]]` 块 (0 新 dep)

**0 接入真 4 协议 handler** (硬约束 #8):
- 本任务仅"骨架 ready", `route_dispatch()` 当前 1:1 调 `handler.dispatch(req)`, 0 调 ResponseCache / ReplayCache
- R123+ 续替换 server.rs 4 endpoint 的 `protocol_handlers::dispatch_cached_with_status` → `route_dispatch(registry.get(kind), req)`

---

## 4. 4 协议 stub 设计 (per spec §3.1)

```rust
// 演示 example 用, 0 进 lib (1 字段 0 业务状态, 仅 demo)
struct OpenAiChatHandler;
impl ProtocolHandler for OpenAiChatHandler {
    fn endpoint_url(&self) -> &str { "/v1/chat/completions" }
    fn cache_key(&self, req: &NormalizedRequest) -> String {
        format!("openai_chat:{}:{}", req.model, req.messages.len())
    }
    fn dispatch(&self, req: NormalizedRequest) -> Result<NormalizedResponse, String> {
        Ok(NormalizedResponse::text("stub", req.model, "openai_chat stub"))
    }
}
// + OpenAiResponsesHandler / AnthropicMessagesHandler / GeminiHandler
```

---

## 5. 测试设计 (8+ test, 0 依赖具体 protocol)

| # | test 名 | 验证 |
|---|---------|------|
| 1 | `protocol_handler_trait_endpoint_url_returns_static_str` | impl stub → `endpoint_url()` 返 4 协议各自 path |
| 2 | `protocol_handler_trait_cache_key_stable_for_same_input` | 同一 NormalizedRequest → 同一 cache_key (跨 2 次调用) |
| 3 | `protocol_handler_trait_supports_stream_default_true` | 不重写 `supports_stream()` → 默 true |
| 4 | `protocol_handler_trait_dispatch_routes_to_handler` | `route_dispatch()` 调 handler.dispatch, 返正确 content |
| 5 | `handler_registry_register_4_protocols_4_handlers` | 4 协议 register → `len() == 4` |
| 6 | `handler_registry_dispatch_4_protocols_returns_correct` | 4 协议 dispatch → 各自 content 不混 |
| 7 | `handler_registry_dispatch_unknown_kind_returns_error` | 4 LLM 协议 register, dispatch `Acp` → Err 含 "no handler" |
| 8 | `protocol_handler_trait_send_sync_compiles` | 编译期: `Box<dyn ProtocolHandler + Send + Sync>` 是 Send + Sync |

---

## 6. 0 冲突核验

- ✅ R122-1 (cache) 已 commit, 改 `protocol_handlers.rs` cache fast path → 我 0 触碰
- ✅ R122-4-retry (jitter) 已 commit `95ac8e4f`, 改 `protocol_handlers.rs` 1 行 → 我 0 触碰
- ✅ R122-6 / R122-7 / R122-8 / R122-9 已 commit, 0 改 protocol_handlers.rs → 0 冲突
- ✅ 24 LOCKED crate mtime → 0 触碰 (我 1 新 mod + 1 lib.rs 行)
- ✅ 11 agent 公共 API 签名 → 0 改 (我 0 触碰 protocol_handlers.rs / v2_endpoints.rs / server.rs)

---

## 7. 时间预算

- readmap 5 min ✓ (现在)
- 实施 50 min (新建 3 文件)
- verify + report 15 min (`cargo build/test/check` 跑通 + 写 final + decision log)

**15:50 → 17:00 截止**

---

**R123-2 启动, 0 范围扩散, 0 装, 0 越界 commit, 等 final report.**
