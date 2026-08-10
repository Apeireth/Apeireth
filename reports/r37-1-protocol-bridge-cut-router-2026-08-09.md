# R37-1: ProtocolBridge — 砍 ProtocolRouter 那一层

**日期**: 2026-08-09
**作者**: Mavis
**状态**: ✅ 完成
**ROI**: ★★★★★ (砍 1 层中间件, 4 协议 dispatch 从 2 层 → 1 层, 0 业务漂移)

---

## 1. 目标

R34 架构调研 #4 (5 候选 ROI 排): "4 个 `ProtocolAdapter` 直接实现 `ProtocolBridge` trait, 砍 `ProtocolRouter`".

之前调用方要:
```rust
let router = ProtocolRouter::new();
let body = router.encode(ProtocolKind::OpenAiChat, &req)?;
let endpoint = router.adapter(ProtocolKind::OpenAiChat).endpoint_path();
let resp = router.decode(ProtocolKind::OpenAiChat, &raw)?;
```
3 层: 业务 → `ProtocolRouter` (持有 4 ZST adapter 字段) → `&dyn ProtocolAdapter` → trait method.

R37-1 后调用方:
```rust
use apeireth_protocol::OpenAiChatBridge;  // ZST
let body = OpenAiChatBridge::encode(&req)?;
let endpoint = OpenAiChatBridge::endpoint_path();
let resp = OpenAiChatBridge::decode(&raw)?;
```
1 层, 类型系统保证 dispatch (0 enum 介入, 0 match 样板), ZST 编译期 inline, 0 虚调用.

---

## 2. 设计

### 2.1 `ProtocolBridge` trait (高层 facade, 砍 router 中间层)

```rust
pub trait ProtocolBridge {
    fn name() -> &'static str;                                  // assoc fn, 0 self
    fn endpoint_path() -> &'static str;                         // assoc fn, 0 self
    fn encode(req: &NormalizedRequest) -> Result<Value, _>;     // assoc fn
    fn decode(raw: &Value) -> Result<NormalizedResponse, _>;    // assoc fn
}
```

**关键设计**:
- 用 **associated function** (非 `&self` method), 强制 ZST 调用, 编译期 inline, 0 虚调用, 0 堆分配
- 4 个 Bridge struct 都是 `struct XxxBridge;` (unit struct, zero-sized)
- 内部 delegate 给现有 `ProtocolAdapter` impl (adapter 代码 0 漂移, 0 重复造轮子)

### 2.2 4 Bridge struct (ZST)

| Bridge | name() | endpoint_path() | delegate to |
|--------|--------|-----------------|-------------|
| `OpenAiChatBridge` | "openai_chat" | "/v1/chat/completions" | `OpenAiChatAdapter` |
| `OpenAiResponsesBridge` | "openai_responses" | "/v1/responses" | `OpenAiResponsesAdapter` |
| `AnthropicMessagesBridge` | "anthropic_messages" | "/v1/messages" | `AnthropicMessagesAdapter` |
| `GeminiBridge` | "gemini" | "/v1beta/models/{model}:generateContent" | `GeminiAdapter` |

### 2.3 `encode_for_kind` / `decode_for_kind` / `endpoint_path_for_kind` dispatch helpers

`ProtocolKind` 是异构 enum, trait 没法做类型参数. 给调用方 `match kind` 样板, 但代码中央化:
```rust
pub fn encode_for_kind(kind: ProtocolKind, req: &NormalizedRequest) -> Result<Value, _> {
    match kind {
        ProtocolKind::OpenAiChat => OpenAiChatBridge::encode(req),
        ProtocolKind::OpenAiResponses => OpenAiResponsesBridge::encode(req),
        ProtocolKind::AnthropicMessages => AnthropicMessagesBridge::encode(req),
        ProtocolKind::Gemini => GeminiBridge::encode(req),
    }
}
```

### 2.4 `ProtocolRouter` 标 `#[deprecated]`, 不强删

- 0 强行删避免 caller breaking
- 加 `#[deprecated(since = "1.0.0", note = "用 ProtocolBridge trait + 4 Bridge struct")]`
- 0.5 release 周期后删 (1-2 R 后, 跟随 R36 91→40 瘦身后再做)
- router 实现 `#[allow(deprecated)]` 自调用, 0 internal deprecation warning

---

## 3. 改动

### 3.1 新增 `crates/apeireth-protocol/src/bridge.rs` (393 LOC)

- `ProtocolBridge` trait
- 4 ZST Bridge struct (OpenAiChatBridge / OpenAiResponsesBridge / AnthropicMessagesBridge / GeminiBridge)
- 3 dispatch helper 函数
- 10 unit test (bridge_tests mod, 涵盖 4 bridge + 3 helper + ZST 编译期断言)

### 3.2 `crates/apeireth-protocol/src/lib.rs`

- 加 `pub mod bridge;` + 6 个 re-export (4 Bridge + 3 helper)
- doc 重写 (强调 R37-1 砍 router 中间层)

### 3.3 `crates/apeireth-protocol/src/router.rs`

- `ProtocolRouter` 标 `#[deprecated]`
- 内部实现 `#[allow(deprecated)]` 避免 self-deprecation warning

### 3.4 `crates/apeireth-pipeline/src/lib.rs`

- 删 `router: ProtocolRouter` 字段
- `Pipeline::new/with_vcp_defaults/with_config` 不再构造 router
- `Pipeline::run/run_streaming` 用 `encode_for_kind` / `decode_for_kind` / `endpoint_path_for_kind`
- 删 `Pipeline::router()` getter (标 deprecated)
- 测试 1 处 `ProtocolRouter::supported_protocols()` 改用 `ProtocolKind` 数组

### 3.5 `crates/apeireth-api/src/protocol_handlers.rs`

- 3 处 `pipeline.router().encode/decode` 改用 `encode_for_kind` / `decode_for_kind`
- import 加 `decode_for_kind, encode_for_kind`

### 3.6 `crates/apeireth-protocol/tests/wire_format.rs`

- 3 处 `ProtocolRouter` 测试改 `ProtocolBridge` + dispatch helper

### 3.7 `crates/apeireth-protocol/examples/router_demo.rs`

- `ProtocolRouter` → `ProtocolBridge` + 4 bridge + dispatch helper

---

## 4. 测试

### 4.1 新增 10 个 unit test (apeireth-protocol)

```
test bridge::bridge_tests::openai_chat_bridge_name_and_endpoint ... ok
test bridge::bridge_tests::openai_responses_bridge_name_and_endpoint ... ok
test bridge::bridge_tests::anthropic_bridge_name_and_endpoint ... ok
test bridge::bridge_tests::gemini_bridge_name_and_endpoint ... ok
test bridge::bridge_tests::openai_chat_bridge_encode_decode_round_trip ... ok
test bridge::bridge_tests::gemini_bridge_encode_decode_round_trip ... ok
test bridge::bridge_tests::encode_for_kind_dispatches_all_4 ... ok
test bridge::bridge_tests::decode_for_kind_dispatches_all_4 ... ok
test bridge::bridge_tests::endpoint_path_for_kind_distinct ... ok
test bridge::bridge_tests::bridges_are_zero_sized ... ok

test result: ok. 10 passed; 0 failed
```

### 4.2 wire_format 改 3 test (ProtocolRouter → ProtocolBridge)

```
test bridge_dispatch_all_4_protocols ... ok
test bridge_endpoints_are_distinct ... ok
test bridge_supports_exactly_4_protocols ... ok
```

### 4.3 回归 (全 workspace)

- **apeireth-protocol**: 88 → 98 lib test (+10 bridge) + 17 wire_format test (改 3), 0 退化
- **apeireth-pipeline**: 80 lib test, 0 退化
- **apeireth-api**: 193 lib test, 0 退化
- **apeireth-tui**: 398/398 unit test, 0 退化
- **全 workspace** build: 0 error, warning 数跟 R30 末态一致 (tools 56 / api 3 等都是 R30 之前累积)

---

## 5. 不漂移 (主哲学锚 #1)

- 4 `ProtocolAdapter` impl 0 触碰 (跟 R17 LOCKED 一致, 0 改字段级)
- `ProtocolKind` enum 0 删 (config 字符串解析还要用)
- `ProtocolRouter` 0 删 (标 `#[deprecated]`, 0.5 release 周期后删)
- pipeline / api / tests / examples 内部 caller 全改, 公共 API 0 breaking (router 标 deprecated 是软迁移)
- compile 期内联 (ZST + assoc fn), runtime 0 虚调用, 0 性能漂移

---

## 6. 后续路线 (R36 准备)

R37-1 砍 router 中间层 = R36 "91→40 瘦身" 的前置条件之一. 后续 R36 收尾:
- ✅ R35 facade (R35 已做, 阶段 1 re-export 4+5 老 crate)
- ✅ R37-1 砍 router 中间层 (本 R)
- ⏭ R36 真删 5 老 provider crate (workspace members 91→86, 0 引用已验证)
- ⏭ R36-2 删 deprecated router (R37-1 标 deprecated, 1 R 周期后再删)

---

**Total LOC**: 1 new file (393) + 6 modify (protocol lib.rs / router.rs / pipeline lib.rs / api protocol_handlers.rs / wire_format.rs / router_demo.rs) + 10 new test + 3 test 改 bridge.
**build/test**: 全 workspace pass, 0 退化, 0 breaking (router 软迁移).
