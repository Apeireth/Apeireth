# R33-3: MCP resources protocol — 真接 2 方法

**日期**: 2026-08-09
**作者**: Mavis
**状态**: ✅ 完成
**ROI**: ★★★ (MCP spec 真接 2 方法, server 端可挂任意 ResourceServer, 0 网络)

---

## 1. 目标

MCP (Model Context Protocol, 2025-03-26) 真协议有 5 类核心方法: `initialize` / `tools/list` + `tools/call` / `resources/list` + `resources/read` / `prompts/list` + `prompts/get`.

apeireth-mcp 已有 `initialize` + `tools/list` + `tools/call` (per `tool_bridge.rs`). R33-3 真接:
- `resources/list` — 列 server 端 resources
- `resources/read` — 按 URI 读 resource 内容

---

## 2. 设计

### 2.1 `Resource` / `ResourceContent` (MCP 真结构 1:1)

```rust
pub struct Resource {
    pub uri: String,           // 唯一 URI
    pub name: String,          // 人类可读名
    pub description: Option<String>,
    pub mime_type: Option<String>,
}
pub struct ResourceContent {
    pub uri: String,
    pub mime_type: Option<String>,
    pub text: String,
}
```

### 2.2 `ResourceServer` trait (server 端抽象)

```rust
pub trait ResourceServer: Send + Sync {
    fn list(&self) -> Vec<Resource>;
    fn read(&self, uri: &str) -> Result<ResourceContent, JsonRpcError>;
}
```

任意 server impl (FileResourceServer / OrganResourceServer / AiderResourceServer / ConventionResourceServer) 都能挂.

### 2.3 4 错误码

- `RESOURCE_NOT_FOUND = -32001` — URI 不存在
- `RESOURCE_INVALID_URI = -32002` — params.uri 缺失或非 string
- `RESOURCE_READ_FAILED = -32003` — 读失败
- `-32601` (JSON-RPC 2.0 standard) — Method not found (dispatch fallback)

### 2.4 `dispatch` helper (避免调用方 match 样板)

```rust
pub fn dispatch(req: &JsonRpcRequest, server: &dyn ResourceServer) -> JsonRpcResponse {
    match req.method.as_str() {
        "resources/list" => handle_resources_list(req, server),
        "resources/read" => handle_resources_read(req, server),
        _ => JsonRpcResponse::err(req.id.clone(), JsonRpcError::new(-32601, ...)),
    }
}
```

---

## 3. 改动

### 3.1 新增 `crates/apeireth-mcp/src/resources.rs` (351 LOC)

- 公开 API: `Resource` + `ResourceContent` + `ResourceServer` trait + `handle_resources_list` + `handle_resources_read` + `dispatch` + 4 错误码 + `StaticResourceServer` (test 用)
- 13 unit test (resources_tests mod, 涵盖 4 场景: 基础构造 / server list+read / handle 2 method / dispatch routing / serde round-trip)

### 3.2 `crates/apeireth-mcp/src/lib.rs`

- 加 `pub mod resources;` (跟 protocol / tool_bridge / transport 1:1)

---

## 4. 测试

### 4.1 13 个新 unit test 全过 (apeireth-mcp)

```
test resources::resources_tests::resource_new_and_with ... ok
test resources::resources_tests::resource_content_with_mime ... ok
test resources::resources_tests::static_server_list_returns_resources ... ok
test resources::resources_tests::static_server_read_existing_uri ... ok
test resources::resources_tests::static_server_read_missing_uri_errors ... ok
test resources::resources_tests::handle_resources_list_returns_json_rpc_ok ... ok
test resources::resources_tests::handle_resources_read_with_uri_returns_content ... ok
test resources::resources_tests::handle_resources_read_missing_uri_returns_error ... ok
test resources::resources_tests::handle_resources_read_no_uri_param_errors ... ok
test resources::resources_tests::dispatch_known_method_routes ... ok
test resources::resources_tests::dispatch_unknown_method_returns_method_not_found ... ok
test resources::resources_tests::resource_serde_round_trip ... ok
test resources::resources_tests::resource_content_serde_round_trip ... ok

test result: ok. 13 passed; 0 failed
```

### 4.2 回归 (全 workspace)

- 全 workspace 4083 lib test pass (R36 4056 + R32-3 6 + R33-3 13 + R33-4 8 = 4083)
- 0 fail, 0 退化

---

## 5. 不漂移 (主哲学锚 #1)

- 0 改 `protocol.rs` (JSON-RPC 2.0 基础 0 触碰)
- 0 改 `tool_bridge.rs` (tools/call 0 触碰)
- 0 引入 I/O / 网络 (server trait 注入, 0 真接)
- 0 改 MCP spec (URI / contents[].text / mimeType 字段 1:1)

---

## 6. 后续路线

- ✅ R33-3 完成
- ⏭ R33-3-1 (1d): 真接 `FileResourceServer` — 走 std::fs 读 workspace 文件, 暴露 `file:///abs/path` URI
- ⏭ R33-3-2 (1d): 真接 `OrganResourceServer` — 暴露 `apeireth://organ/{name}` URI 9 organ 状态
- ⏭ R33-3-3 (1d): 真接 `ConventionResourceServer` — 暴露 R33-1 conventions 抽到的 system block
- ⏭ R33-5 (LangGraph conditional 实战) — 跟 R32-2 后续一起

---

**Total LOC**: 1 new file (351) + 1 modify (lib.rs 加 1 行 mod) + 13 new test.
**build/test**: 全 workspace pass, 0 退化, 0 breaking.
