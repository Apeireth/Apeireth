# R172 apeireth-voice MiniMax LIVE TTS 真接

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R172 (live capability upgrade)
> **日期**: 2026-08-13
> **borrow-id**: R172-VOICE-BORROW-minimax-t2a-v2-2026-08-13
> **主人授权**: 全按你的建议来 + 时间和 token 充裕, 干到底

---

## 0. 背景

apeireth-voice 现有 4 层路径:
- **STUB facade** (`lib.rs`): 8 Porcupine/pvrecorder 工具返 `NotImplemented`
- **VoiceRealImpl** (`real.rs`): 1:1 翻译商业版 voice SDK, 默认指向 https://api.apeireth.com/v1 (假想生产 URL)
- **realtime** (`realtime.rs`): OpenAI Realtime 协议 schema (WebSocket)
- **MiniMaxLive** (本档, R172 新增): 直连 MiniMax production API, 拿真 MP3 bytes

R172 解决**最后 1 公里**: 让 apeireth-voice 用主人提供的 MiniMax apikey 立刻可调, 不再依赖假想生产 URL。

---

## 1. LIVE 验证 (R172 真接, 2026-08-13)

通过 `cargo run -p apeireth-voice --example voice_minimax_live_demo` 跑通:

```
=== R172 MiniMax LIVE TTS demo ===
model: speech-2.6-hd
voice: male-qn-qingse

source: env (APEIRETH_API_KEY)
api_key_cached: true

--- TTS call #1 (English) ---
elapsed: 1136 ms
bytes: 111156
header: [73, 68, 51, 4] (ID3 = MP3)
saved: r172_tts_output_en.mp3

--- TTS call #2 (中文) ---
elapsed: 1041 ms
bytes: 118068
header: [73, 68, 51, 4] (ID3 = MP3)
saved: r172_tts_output_zh.mp3

--- TTS call #3 (empty) ===> InvalidParams: text is empty (0 ms)
(expected: empty text rejected without HTTP call)

=== demo 完成 (R172) ===
```

**关键证据**:
- 2 个真实 MP3 文件落盘 (111KB EN + 118KB ZH)
- ID3 header `[73, 68, 51, 4]` = "ID3\x04" = 标准 MP3 ID3v2.4 头
- 端到端延迟 1041-1136 ms (含网络 + MiniMax 服务端合成)
- 空文本 0 ms 返 InvalidParams, 不发 HTTP (K-1 强校验守门)

---

## 2. MiniMax T2A v2 API spec (R172 实现基线)

### 2.1 端点

```
POST https://api.minimaxi.com/v1/t2a_v2
Authorization: Bearer <api_key>
Content-Type: application/json; charset=utf-8
```

### 2.2 请求 payload

```json
{
  "model": "speech-2.6-hd",
  "text": "hello apeireth",
  "stream": false,
  "voice_setting": {
    "voice_id": "male-qn-qingse",
    "speed": 1.0,
    "vol": 1.0,
    "pitch": 0
  },
  "audio_setting": {
    "sample_rate": 32000,
    "bitrate": 128000,
    "format": "mp3",
    "channel": 1
  }
}
```

### 2.3 响应

```json
{
  "data": {
    "audio": "4944330400000000...",
    "status": 2
  },
  "extra_info": {},
  "trace_id": "...",
  "base_resp": {
    "status_code": 0,
    "status_msg": "success"
  }
}
```

`audio` 字段是 **hex-encoded** audio bytes (不是 base64), 比 base64 紧凑约 1/3。

### 2.4 编译期常量 (1:1 翻译 spec)

| 常量 | 值 | 含义 |
|---|---|---|
| `MINIMAX_BASE_URL` | `https://api.minimaxi.com` | production API base |
| `MINIMAX_T2A_V2_PATH` | `/v1/t2a_v2` | TTS 端点 |
| `MINIMAX_DEFAULT_TTS_MODEL` | `speech-2.6-hd` | HD 模型 (vs turbo) |
| `MINIMAX_DEFAULT_VOICE_ID` | `male-qn-qingse` | 默认男声 (中文清爽) |
| `MINIMAX_DEFAULT_SAMPLE_RATE` | `32000` Hz | MP3 输出采样率 |
| `MINIMAX_DEFAULT_BITRATE` | `128000` bps | MP3 输出码率 |
| `MINIMAX_DEFAULT_FORMAT` | `"mp3"` | 输出格式 |

---

## 3. 设计原则 (per 蓝图 §1 6 哲学锚穿透)

| 锚 | 体现 |
|---|---|
| **S-1 北极星** | 1:1 翻译 MiniMax 官方 T2A v2 API spec, 0 重新发明协议 |
| **S-2 实事求是** | R172 LIVE 验证 2 个真 MP3 文件落盘 + ID3 header 确认, 不假装"调通" |
| **O-1 安全优先** | api_key 走 env (`APEIRETH_API_KEY`) 或 openclaw 文件, 0 硬编码, 0 进日志 |
| **O-2 走在前人肩上** | `reqwest` 0.12 + `tokio` 1.40 + `serde` 1.0 全是 workspace 已有, 0 引外部 dep |
| **O-3 干到底** | 1 模块覆盖 TTS 真接 + 1 demo (3 用例: EN/ZH/empty) + 1 文档, 信息密度高 |
| **O-4 接手** | `MiniMaxLive` 单一 struct, 字段最小, 任何人都能改 |

---

## 4. 与其他路径的关系 (R172 不冲突)

| 路径 | 何时用 | R172 触碰 |
|---|---|---|
| STUB facade (`VoiceSdk`) | 默认编译路径, 8 工具返 `NotImplemented` | 0 触碰 |
| `VoiceRealImpl` (`real.rs`) | 1:1 翻译商业版 voice SDK, 兼容性测试 | 0 触碰 |
| `MiniMaxLive` (本档, R172+) | **真接 production MiniMax API** | ✅ 新增 |
| `realtime` (`realtime.rs`) | OpenAI Realtime 协议 schema (WebSocket 双工) | 0 触碰 |

**0 触碰**:
- 3 不可变脊柱 (Self-Disable / L0 HA / 13-key verdict cache)
- workspace.version 1.2.0
- STUB_MODE compile-time hardcode
- 24 LOCKED crate 入口签名

---

## 5. 文件清单 (R172 改动)

| 文件 | 类型 | 字节 | 说明 |
|---|---|---|---|
| `crates/apeireth-voice/src/minimax_live.rs` | NEW | 18,048 | MiniMaxLive client + 11 单测 |
| `crates/apeireth-voice/examples/voice_minimax_live_demo.rs` | NEW | 5,032 | LIVE demo (3 用例) |
| `crates/apeireth-voice/src/lib.rs` | modified | +1 line | `pub mod minimax_live;` |
| `crates/apeireth-voice/Cargo.toml` | 0 改动 | — | 0 引外部 dep |
| `docs/r172/r172-minimax-live-voice.md` | NEW (本档) | — | 设计 + LIVE 证据 |

**新增测试**: +11 (R172 minimax_live 模块单测)
**全 crate 测试**: 64 pass / 0 fail (R172 后)

---

## 6. R172 后续 (R172+1, R172+2, ...)

| R | 任务 | 优先级 |
|---|---|---|
| R172+1 | ASR 真接 (`/v1/asr` 端点, multipart/form-data) | 中 |
| R172+2 | 多 voice_id registry (5 主流声音 ID 表) | 低 |
| R172+3 | streaming TTS (per MiniMax spec, `stream: true`) | 中 |
| R172+4 | 音频元数据解析 (duration / sample count from MP3) | 低 |
| R172+5 | TUI 集成 (`apeireth-tui` 调用 `MiniMaxLive`) | 高 |

**前提**: R172+ 不动 3 不可变脊柱, STUB_MODE 仍守门, VoiceRealImpl 1:1 翻译路径保留。
