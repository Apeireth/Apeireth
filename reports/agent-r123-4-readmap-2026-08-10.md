# R123-4 Readmap — v2.1 P2-13 多模态生成 via MCP dispatcher template

> **作者**: R123-4 (Mavis coder 团队)
> **时间**: 2026-08-10 15:46 → 15:55 (8 min)
> **任务**: 1 个 v2.1 P2 缺口: 多模态生成 via MCP (per 07 §2 P2-13)
> **借鉴 ID**: `R123-4-VCP-MultimodalMCP-2026-08-10` (per 07 §3 借 VCP 9 Gen 插件架构)
> **状态**: ✅ readmap 完成, 0 改 src, 开干

---

## §0. TL;DR

- 0 改任何现有 19 文件, 仅加 1 新 mod (`multimodal.rs`) + 1 example + 1 行 mod 声明
- dispatcher 是 template, 0 真接 9 个 Gen 插件 MCP server (per O-5 诚实标缺, R124+ 真接)
- 8+ unit test 覆盖 (GenPlugin 9 variants / OutputFormat 6 variants / dispatch not_connected / serde round-trip / size default / optional seed / multimodal in registry)
- 11 agent 公共 API 签名 0 改
- 24 LOCKED 0 触碰
- workspace.version (1.1.0) 0 改
- Cargo.toml 加 1 个 `[[example]]` 段 (允许, 不改 version)

---

## §1. 现状核验 (15:46 完成)

### 1.1 `crates/apeireth-mcp/` 结构 (16 文件 + 2 examples + 2 tests + Cargo.toml = 21)

| 路径 | 角色 | 是否触碰 |
|------|------|---------|
| `src/lib.rs` (27KB) | 顶层 + McpClient/Server/ServerInfo/McpError | ❌ 0 改, 加 1 行 `pub mod multimodal;` |
| `src/protocol.rs` (9.5KB) | JSON-RPC 2.0 基础类型 | ❌ 0 改 |
| `src/initialize.rs` (16KB) | initialize handshake | ❌ 0 改 |
| `src/tools.rs` (14KB) | MCP tools protocol (Tool/ToolContent/ToolServer) | ❌ 0 改 |
| `src/tool_bridge.rs` (10KB) | registry 桥接 (ToolDef/ToolHandler) | ❌ 0 改 |
| `src/resources.rs` (12KB) | MCP resources | ❌ 0 改 |
| `src/resource_servers.rs` (32KB) | 3 ResourceServer impl | ❌ 0 改 |
| `src/subscriptions.rs` (15KB) | resources/subscribe | ❌ 0 改 |
| `src/tool_subscriptions.rs` (19KB) | tools/subscribe | ❌ 0 改 |
| `src/prompts.rs` (17KB) | prompts | ❌ 0 改 |
| `src/telemetry_bridge.rs` (19KB) | handler call metrics | ❌ 0 改 |
| `src/transport/mod.rs` (8KB) | Transport trait + MemoryTransport | ❌ 0 改 |
| `src/transport/stdio.rs` (5KB) | StdioTransport | ❌ 0 改 |
| `src/transport/sse.rs` (17KB) | SseTransport | ❌ 0 改 |
| `src/transport/http_streamable.rs` (11KB) | HTTP-streamable | ❌ 0 改 |
| `examples/hello.rs` (4KB) | 端到端互调示例 | ❌ 0 改 |
| `examples/resource_servers_demo.rs` (2KB) | resource demo | ❌ 0 改 |
| `tests/conformance.rs` (3.8KB) | MCP conformance | ❌ 0 改 |
| `tests/multi_transport.rs` (18KB) | 3 transport test | ❌ 0 改 |
| `Cargo.toml` (1.6KB) | 1.6KB | ⚠️ 加 1 个 `[[example]]` 段 (允许) |
| **🆕 `src/multimodal.rs`** (250 行) | dispatcher template + 9 plugin stub | ✅ 新建 |
| **🆕 `examples/multimodal_mcp_demo.rs`** (80 行) | 演示 9 plugin + 6 format | ✅ 新建 |

### 1.2 8 墙硬约束 (per 主人 13:30 任务)

| 墙 | 严守方式 |
|----|---------|
| 1. workspace.version (1.1.0) | ❌ 不改 Cargo.toml `[workspace.package] version` |
| 2. R11 baseline 3 值 | ❌ 不改 V1141/V1131/V1136 |
| 3. 24 LOCKED crate mtime | ❌ 不碰 (apeireth-mcp src 0 在 24 LOCKED 名单 src, 仅 tests/multi_transport.rs 在 #8 "评估好 测试修复") |
| 4. 9 器官 logic | ❌ 不碰 |
| 5. 6 哲学锚 / 12 键 / 5 重守门 / V0.5 24 维 / 双洋葱 | ❌ 不碰 |
| 6. 11 agent 公共 API 签名 | ❌ 不改 (multimodal 0 改 Tool trait, 走 handler_from_fn 显式注册) |
| 7. 0 主动 commit | ❌ 不 commit, 仅改工作树 |
| 8. 0 装 (O-5) | ✅ dispatcher template 返 "not connected" 错误, R124+ 真接 |

### 1.3 R123-3 0 冲突核验

- R123-3 也在 apeireth-mcp 加 1 新 mod (browser.rs, 估计 P2-12 Playwright via MCP)
- 你加 `multimodal.rs`, 2 个不同文件名, 0 文件冲突
- 各自加 1 行 `pub mod xxx;` 在 lib.rs, 0 同一行冲突
- R123-3 也加 1 example, 0 冲突 (不同文件名)
- 0 触碰 11 agent 公共 API
- 0 触碰 Cargo.toml 现有 1 个 `[[example]]` hello 段, 各加各的 1 个 `[[example]]` 段

---

## §2. multimodal.rs 设计 (1 dispatcher + 9 插件 stub + 6 output format)

### 2.1 公共 API (multimodal.rs 暴露)

```rust
// 9 个 Gen 插件 enum (per 07 §2 P2-13, 9 plugin 列表)
pub enum GenPlugin { ComfyUI, Flux, Doubao, GPTImage, Gemini, Agnes, AgnesVideo, DMX, NanoBanana }

// 输出格式 (6 variants, 覆盖常见模态)
pub enum OutputFormat { Png, Jpg, Gif, Mp4, Webp, Glb }

// 请求结构
pub struct GenRequest {
    pub plugin: GenPlugin,
    pub prompt: String,
    pub seed: Option<u64>,
    pub size: Option<(u32, u32)>,
    pub output_format: OutputFormat,
}

// 响应结构 (含 image_url / video_url / model_url 3 field, 0 假设哪种)
pub struct GenResponse {
    pub plugin: GenPlugin,
    pub image_url: Option<String>,
    pub video_url: Option<String>,
    pub model_url: Option<String>,
    pub seed_used: Option<u64>,
}

// 路由表: per plugin URL 模板 (0 真连, 仅 URL 模板)
pub fn plugin_endpoint(plugin: GenPlugin) -> &'static str

// dispatcher template — 永远返 Err("not connected")
pub fn gen_dispatch(req: GenRequest) -> Result<GenResponse, String>

// 给 mcp tool handler 用的 sync wrapper
pub fn dispatch_multimodal_handler(args: Value) -> Result<Value, String>

// Size 默认 1024x1024
pub const DEFAULT_SIZE: (u32, u32) = (1024, 1024);
```

### 2.2 借鉴 VCP 9 Gen 插件 (per 07 §2 P2-13, VCP `Plugin/*Gen*/`)

| 插件 | 借鉴 URL 模板 (占位) | 借鉴描述 |
|------|---------------------|---------|
| ComfyUI | `"http://localhost:8188/prompt"` | ComfyUI workflow 提交, 通用后端 |
| Flux | `"https://api.flux.ai/v1/generate"` | FLUX.1 [pro/dev/schnell] |
| Doubao | `"https://ark.cn-beijing.volces.com/api/v3/images/generations"` | 豆包文生图 |
| GPTImage | `"https://api.openai.com/v1/images/generations"` | gpt-image-1 / dall-e-3 |
| Gemini | `"https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent"` | Imagen 3 / Imagen 4 |
| Agnes | `"http://localhost:7860/sdg/prompt"` | Agnes SD (Stability Diffusion-like) |
| AgnesVideo | `"http://localhost:7860/sdg/video"` | Agnes 视频版 |
| DMX | `"https://api.deepmindx.com/v1/generate"` | DeepMind-X 占位 |
| NanoBanana | `"https://api.nanobanana.ai/v1/generate"` | NanoBanana 占位 |

> **0 真连, 0 真接 schema**: URL 模板仅占位, dispatch 返 "plugin X not connected (R123-4 template placeholder, R124+ 真接)"

### 2.3 9 GenPlugin 路由表

```rust
pub fn plugin_endpoint(plugin: GenPlugin) -> &'static str {
    match plugin {
        GenPlugin::ComfyUI => "http://localhost:8188/prompt",
        GenPlugin::Flux => "https://api.flux.ai/v1/generate",
        GenPlugin::Doubao => "https://ark.cn-beijing.volces.com/api/v3/images/generations",
        GenPlugin::GPTImage => "https://api.openai.com/v1/images/generations",
        GenPlugin::Gemini => "https://generativelanguage.googleapis.com/v1beta/models",
        GenPlugin::Agnes => "http://localhost:7860/sdg/prompt",
        GenPlugin::AgnesVideo => "http://localhost:7860/sdg/video",
        GenPlugin::DMX => "https://api.deepmindx.com/v1/generate",
        GenPlugin::NanoBanana => "https://api.nanobanana.ai/v1/generate",
    }
}
```

### 2.4 gen_dispatch (template, 0 真接)

```rust
pub fn gen_dispatch(req: GenRequest) -> Result<GenResponse, String> {
    // 0 真接 — O-5 诚实标缺
    Err(format!(
        "plugin {:?} not connected (R123-4 template placeholder, R124+ 真接 via apeireth-mcp + apeireth-mcp-tools multimodal)",
        req.plugin
    ))
}
```

### 2.5 mcp handler wrapper (serde bridge)

```rust
pub fn dispatch_multimodal_handler(args: Value) -> Result<Value, String> {
    let req: GenRequest = serde_json::from_value(args)
        .map_err(|e| format!("invalid GenRequest JSON: {e}"))?;
    // 0 真接 — O-5 标缺
    Err(format!(
        "plugin {:?} not connected (template placeholder, R124+ 真接)",
        req.plugin
    ))
    // (如果 R124 真接, 替换为: gen_dispatch(req).map(|r| serde_json::to_value(r).unwrap()))
}
```

> **设计取舍**: `dispatch_multimodal_handler` 返 Err 而非 Ok(value) 是有意的, 这样 example 调 `call_tool("multimodal", ...)` 会看到 `isError: true`, 跟 dispatcher 当前状态一致。

### 2.6 unit test 清单 (8+, 0 依赖网络)

1. `gen_plugin_9_variants_distinct_endpoints` — 9 plugin 各有独立 endpoint
2. `gen_request_serde_round_trip_with_all_fields` — GenRequest 序列化 1:1
3. `gen_dispatch_returns_not_connected_error_for_all_9_plugins` — O-5 诚实标缺
4. `output_format_6_variants_serialize_correctly` — 6 format 序列化 (kebab-case / snake_case)
5. `gen_request_seed_optional_works` — seed None / Some(42) 都对
6. `gen_request_size_default_1024x1024` — DEFAULT_SIZE 常量
7. `gen_response_serializes_with_image_url_field` — GenResponse 序列化
8. `multimodal_tool_listed_in_mcp_registry` — ToolDef `name = "multimodal"` + 1 行 ToolContent

(可选 9+10) `gen_dispatch_error_message_contains_plugin_name` / `plugin_endpoint_returns_https_or_http_url`

---

## §3. 1 行 mod 声明 (在 lib.rs:47 之后)

```rust
pub mod transport;
// 🆕 R123-4: 多模态生成 dispatcher template (per 07 §2 P2-13, 0 真接, R124+ 真接)
pub mod multimodal;
```

**0 改 lib.rs 任何现有 700 行代码, 仅末尾加 1 行**。

---

## §4. Cargo.toml 加 1 个 `[[example]]` 段

```toml
[[example]]
name = "multimodal_mcp_demo"
path = "examples/multimodal_mcp_demo.rs"
```

> **0 触碰**: workspace.version / 其他 14 个 `[[example]]` 段 / `[dependencies]` / `[dev-dependencies]`
> **仅加**: 1 个新段 (允许, 跟 R123-3 各加各的)

---

## §5. 验收硬指标 (per 主人 13:30 任务)

| 指标 | 目标 | 自验方式 |
|------|------|---------|
| `cargo build -p apeireth-mcp` | 0 error | local |
| `cargo test -p apeireth-mcp --lib multimodal_tests` | 8+ passed, 0 failed | local |
| `cargo test -p apeireth-mcp --lib` | 全 0 failed (含 R123-3 browser tests) | local |
| `cargo check --workspace` | 0 failed | local |
| 11 agent 公共 API 签名 | 0 改 | grep 现有 API |
| 24 LOCKED | 0 触碰 | mtime 扫 |
| workspace.version (1.1.0) | 0 改 | git diff Cargo.toml:246 |
| multimodal dispatcher 0 真接 | ✅ template 返 not_connected | 8+ test 守 |

---

## §6. 风险与防御

| 风险 | 防御 |
|------|------|
| 1. R123-3 也加 `pub mod xxx;` 在 lib.rs 同一行 | 我加在 `pub mod transport;` **之后** 1 行, R123-3 加在 transport **之前**, 0 同一行冲突 (但要 R123-3 别加 transport 之后) |
| 2. Cargo.toml 段格式错 (要 `path = ...` 不要 `path = "examples/..."` 相对) | 跟现有 `[[example]]` hello 段 1:1 |
| 3. dispatcher 真接 (装 0) | 0 HTTP client 调用, 0 reqwest, gen_dispatch 直接 Err |
| 4. multimodal 触碰 lib.rs 现有 re-exports | 仅 1 行 `pub mod multimodal;`, 0 改 67-72 行 re-exports |
| 5. multimodal 加 Cargo.toml 段改 [package] | 仅加新 `[[example]]` 段, 0 改 [package] / [dependencies] / [dev-dependencies] |
| 6. tool 0 真接 vs mcp tool list 显示 | example 演示 list 后调 call, call 返 isError=true 提示 "0 真接" |

---

## §7. 时间预算 (15:46 启动, 17:30 截止, 1h44m)

| 阶段 | 计划 | 状态 |
|------|------|------|
| 8 min | 读 readmap (本文件) | ✅ 15:46-15:55 |
| 60 min | 实施: multimodal.rs (250 行) + 1 行 mod + example (80 行) + Cargo.toml 段 | 🟢 15:55-16:55 |
| 30 min | verify: cargo build / test / check workspace + 写 final + decision log | ⏳ 16:55-17:25 |
| 5 min | 收尾 + 报回 parent session | ⏳ 17:25-17:30 |

---

## §8. 总结

- 0 改任何现有 19 文件 (含 lib.rs / tools.rs / tool_bridge.rs / Cargo.toml [package] / [dependencies])
- 仅 1 行 mod 声明 + 1 新 multimodal.rs (~250 行) + 1 新 example (~80 行) + 1 个新 `[[example]]` 段
- 8+ unit test 全过
- dispatcher template 0 真接, O-5 诚实标缺
- 11 agent 公共 API 签名 0 改 (multimodal 走 handler_from_fn 显式注册)
- 24 LOCKED 0 触碰
- workspace.version 0 改

**R123-4 readmap 完成, 实施 60 min 开始.**

---

_本 readmap 是 R123-4 任务的实施蓝图, 0 范围扩散, 0 装, 0 越界 commit. 验证后写 final + decision log._
