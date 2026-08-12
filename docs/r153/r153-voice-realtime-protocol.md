# R153 — `apeireth-voice` GPT-Realtime-2 升级 (OpenAI Realtime API 协议)

> **R153 (2026-08-13)**: `apeireth-voice::realtime` 模块新增 — OpenAI Realtime API 协议 schema + lifecycle + dispatch 表面 (3-model 分发 + 128K context + ephemeral token + 10 server events + 8 client events + 4 conversation items + function calling + multimodal image input + server VAD + base64 encoder), 0 引外部 dep. 累计 +45 tests (32 unit + 12 integration + 1 example), 0 errors, 0 触碰 3 不可变脊柱. 同时清理 apeireth-voice Cargo.toml description + lib.rs 顶部 doc (去 R20 阶段 X / 1:1 翻译 / RIVAL / 商业版 字样), 保留 Porcupine + pvrecorder + OpenAI Realtime 上游 attribution per O-5. 详见 `crates/apeireth-voice/README.md` + `crates/apeireth-voice/examples/realtime_session_demo.rs` + `crates/apeireth-voice/tests/test_voice_realtime.rs`.

---

## 1. 动机

apeireth-voice 原本是 STUB skeleton (Porcupine + pvrecorder API surface) + HTTP 真接 (real.rs 4 块 TTS/STT/唤醒词/声纹) 双轨结构. 缺一个独立的"协议 schema + lifecycle + dispatch"层, 导致:

1. 想接 OpenAI Realtime API (gpt-realtime 系列) 时, 没现成的 type / event / token schema 可用.
2. ephemeral token、server VAD、function calling、image input 这类 Realtime-specific 概念散在外部文档, 没 Rust type 落地.
3. 任何 Realtime session 都要重新手搓 enum + serde rename + 大小守门, 重复造轮子.

R153 补上 `apeireth-voice::realtime` 模块, 提供 provider-agnostic 的协议 schema 层 — 实际 session I/O 走 `apeireth-api` 网关或任何 OpenAI-Realtime 兼容端点.

## 2. 设计决策

### 2.1 3-model 编译期 hardcode

```rust
pub enum RealtimeModel {
    GptRealtime,        // flagship, cost_tier=3
    GptRealtimeMini,    // cost-optimized, cost_tier=1
    Gpt4oRealtime,      // legacy, cost_tier=2
}
pub const REALTIME_MODEL_COUNT: usize = 3;
pub const SUPPORTED_REALTIME_MODELS: &[RealtimeModel] = &[...];
const _: () = assert!(SUPPORTED_REALTIME_MODELS.len() == REALTIME_MODEL_COUNT);
```

**不开放 runtime extension**: 编译期硬编码 3 个模型, 防止有人运行时塞入未知 model id 绕过 type check. 与 sovereignty 的"3 immutable spines"哲学一致 (Self-Disable / L0 HA / 13-key verdict cache 都是编译期硬约束).

### 2.2 wire-format 命名 (dotted notation)

OpenAI Realtime GA 2024-12 用点号分隔事件名 (`session.created`, `response.audio.delta`). serde `rename_all = "snake_case"` 会变成下划线, 所以逐 variant 加 `#[serde(rename = "...")]`:

```rust
#[serde(rename = "response.audio.delta")]
AudioDelta { ... }

#[serde(rename = "response.function_call_arguments.done")]
FunctionCall { ... }
```

测试断言显式带 dotted prefix (`assert!(json.contains("\"type\":\"response.audio.delta\""))`), 防止 wire-format 漂移.

### 2.3 0 引外部 dep

按主人"ponytail ceiling"指示: realtime 模块 0 外部 dep, 内嵌:
- `simple_base64_encode` (RFC 4648 标准 base64, 30 行手写, 避免 `base64` crate 340KB)
- serde tag-based enum (already in workspace)
- thiserror (already in workspace)

`apeireth-voice` 顶层仍引 `reqwest` (real.rs 用), 但 `realtime` 子模块纯协议 schema, 不引 HTTP client.

### 2.4 不动 STUB_MODE / VoiceRealImpl

R153 加新模块, 不动现有 STUB facade / VoiceRealImpl / 现有 23 fixture + 23 测试. `lib.rs` 顶部 doc 重写, 但保留 STUB_MODE hardcode + VoiceSdk 9 工具 + TOOL_WHITELIST, 维持 K-1 强校验 fixture 测试 (`fixture_4_k1_keywords_and_stub_mode`) 仍过.

## 3. 模块清单

| 类型 | 说明 | 编译期守门 |
|------|------|-----------|
| `RealtimeModel` | 3-model 枚举 | `REALTIME_MODEL_COUNT = 3` const assert |
| `RealtimeVoice` | 6 voices (alloy/ash/coral/echo/sage/verse) | `SUPPORTED_REALTIME_VOICES.len() == 6` |
| `RealtimeModality` + `RealtimeModalities` | input/output modality (text/audio) | Default = text+audio 双模 |
| `RealtimeAudioFormat` | PCM16 / G.711 μ-law / G.711 A-law | Default = PCM16 |
| `TurnDetection` + `TurnDetectionKind` | Server VAD 配置 | Default = server_vad, 0.5 threshold, 500ms silence |
| `RealtimeTool` | Function tool 定义 | `name` 1..=64 chars, `kind` 必 = "function" |
| `RealtimeSessionConfig` | Session config (builder pattern) | validate() 检查 tools ≤ 128, temperature 0..2, ttl ≤ 1h |
| `EphemeralToken` | Server-minted token | is_expired() / remaining() 检查 |
| `EphemeralTokenRequest` | POST /v1/realtime/sessions body | From<&RealtimeSessionConfig> 派生 |
| `ServerEvent` | 10 server-to-client events | serde tagged enum, dotted notation |
| `ClientEvent` | 8 client-to-server events | serde tagged enum, dotted notation |
| `ConversationItem` | 4 conversation item types | serde tagged enum |
| `encode_image_input` | base64 multimodal image, 5MB 上限 | `REALTIME_MAX_IMAGE_BYTES` const |
| `encode_audio_append` | audio buffer append, 15MB 上限 | `REALTIME_MAX_AUDIO_BUFFER_BYTES` const |
| `simple_base64_encode` | RFC 4648 base64, no external dep | RFC 4648 §10 canonical fixtures 测试 |

## 4. Wire-format 验证

R153 测试显式断言 wire-format dotted 命名:

```rust
// test_server_event_audio_delta_serde
assert!(json.contains("\"type\":\"response.audio.delta\""));

// test_server_event_function_call_serde
assert!(json.contains("\"type\":\"response.function_call_arguments.done\""));

// test_conversation_item_input_image_serde
assert!(json.contains("\"type\":\"conversation.item.input_image\""));

// test_client_event_session_update_serde
assert!(json.contains("\"type\":\"session.update\""));
```

任何对 serde rename 的无意识改动都会立即被测试拦截.

## 5. 测试覆盖

| 测试类型 | 数量 | 位置 |
|---------|------|------|
| Lib unit (model / voice / config / event serde / base64) | 32 | `src/realtime.rs::tests` |
| Integration (session lifecycle / event roundtrip / size guards) | 12 | `tests/test_voice_realtime.rs` |
| Pre-existing lib unit (STUB facade + K-1 fixtures) | 53 | `src/lib.rs::tests` + `src/real.rs::tests` |
| Pre-existing integration (wiremock + fixture) | 26 | `tests/test_voice_stub_in_process.rs` + `tests/test_voice_real_wiremock.rs` |
| **Total** | **91 + 0 failed** | |

## 6. 示例输出

`cargo run -p apeireth-voice --example realtime_session_demo` 输出片段:

```
Config: RealtimeSessionConfig {
  model: GptRealtimeMini, voice: Sage,
  modalities: RealtimeModalities { input: [Text, Audio], output: [Audio, Text] },
  audio_format: Pcm16, turn_detection: TurnDetection { kind: ServerVad, ... },
  tools: [RealtimeTool { kind: "function", name: "get_weather", ... }],
  tool_choice: Some("auto"), temperature: 0.7, max_output_tokens: 4096,
  session_ttl: 3600s, metadata: {"session_origin": "apeireth-r153-demo"}
}
session.update payload:
{
  "type": "session.update",
  "config": {
    "model": "gpt-realtime-mini",
    "voice": "sage",
    "modalities": { "input": ["text", "audio"], "output": ["audio", "text"] },
    "audio_format": "pcm16",
    ...
  }
}
Validation: OK
Tools registered: 1
```

## 7. 已知限制 / 留 R154+

1. **WebRTC signaling** 没做 — OpenAI Realtime 可选 WebRTC transport, 留给后续. 当前只暴露 WebSocket-style event schema, HTTP/WebSocket transport 在上层 (`apeireth-api` 网关).
2. **远程 MCP (model context protocol)** — OpenAI Realtime GA 2026 Q1 加的远程 MCP 接入. 留 R154+.
3. **SIP 集成** — 主人指示"占卜/酒馆"先冻结, SIP 同样属"电话集成"大方向, 等真接需求时做.
4. **G.711 codec 实现** — 当前只暴露 enum + size guard, 实际 PCM16 ↔ G.711 μ-law/A-law 转换留给上层 codec crate.

## 8. 文档同步

- `crates/apeireth-voice/README.md` 更新 (新增 `realtime` 模块说明 + wire-format examples)
- `crates/apeireth-voice/src/lib.rs` 顶部 doc 重写 (3-layer architecture table + borrowed upstream references per O-5)
- `crates/apeireth-voice/Cargo.toml` description 清理 (去 R20 阶段 X / 1:1 翻译 字样, 改为现代 3-layer 描述)
- 顶层 `README.md` R153 banner (本次提交)

## 9. 累计 regression

| 维度 | 数量 |
|------|------|
| 新 crate | 0 (扩展现有 `apeireth-voice`) |
| 新模块 | 1 (`apeireth_voice::realtime`) |
| 新 unit tests | 32 |
| 新 integration tests | 12 |
| 新 example | 1 (`realtime_session_demo`) |
| 新 Cargo.toml example 声明 | 1 |
| 0 触碰 3 不可变脊柱 | ✓ |
| 0 改 STUB_MODE | ✓ |
| 0 改 VoiceRealImpl | ✓ |
| 0 改现有 53 lib + 26 integration tests | ✓ |
| 0 改 workspace.version (1.2.0) | ✓ |

## 10. 0-touch 声明

按 8 项不修改承诺:

- ✓ 0 触碰 `docs/v4/v4.1/v2/V0.5/V1136/9键原始`
- ✓ 0 触碰 `workspace.version` (1.2.0)
- ✓ 0 触碰 R11 baseline 3 values
- ✓ 0 触碰 3 不可变脊柱 (Self-Disable / L0 HA / 13-key verdict cache)
- ✓ 保留 Porcupine / pvrecorder / OpenAI Realtime 上游 attribution per O-5
- ✓ 0 触碰 STUB_MODE (仍 = true)

## 11. 下一步 (R154 候选)

- `apeireth-relation` SurrealDB-style graph query + GQL-lite DSL 解析 (per master 优先级列表 #2)
- TUI × runtime 集成 (per master "后端完全做好了再接 tui" — 后端 R153 仍只 voice 模块层, 整体 runtime 早 OK)
- 调研 GitHub 优秀项目: 对每个模块逐一调研 (per master 8/12 指示)
