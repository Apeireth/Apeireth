# apeireth-voice

> Apeireth voice subsystem — wake word detection, audio capture, TTS, STT, voiceprint, and the OpenAI Realtime API protocol schema.


> **R172 LIVE (2026-08-13)**: New `MiniMaxLive` module + `voice_minimax_live_demo` — direct MiniMax production HTTP, real MP3 audio (111KB EN + 118KB ZH, ID3 header confirmed). See `docs/r172/r172-minimax-live-voice.md`. 0 引外部 dep, 0 触碰 3 不可变脊柱.
>
> **R153 LIVE (2024-12 GA)**: OpenAI Realtime API protocol schema (3 models, 128K context, ephemeral tokens, VAD, function calling, multimodal image). See `realtime` module.

## Three-layer architecture (R153 unified)

| Layer | Module | Purpose |
|-------|--------|---------|
| STUB facade | `src/lib.rs` | Porcupine + pvrecorder-style API surface (8 tools). Compile-time `STUB_MODE = true` guard returns `NotImplemented` for all 8 tools. Designed for downstream wiring once picovoice SDK is added. |
| Real HTTP client | `src/real.rs` | `VoiceRealImpl` — TTS / STT / wake-word / voiceprint over `reqwest` HTTP. Wiremock-tested. |
| Realtime protocol | `src/realtime.rs` (R153) | OpenAI Realtime API schema — 3-model dispatch (gpt-realtime / gpt-realtime-mini / gpt-4o-realtime), 128K context, ephemeral tokens, server VAD, function calling, multimodal image input. 0 引 external dep. |
| **MiniMax LIVE** (R172) | `src/minimax_live.rs` | **Production MiniMax API client** — TTS via `/v1/t2a_v2` returning hex-encoded MP3. 11 unit tests. Live demo: `cargo run -p apeireth-voice --example voice_minimax_live_demo` (writes real MP3 to disk). 0 引 external dep. |

## Quick start: MiniMax LIVE (R172)

```rust
use apeireth_voice::minimax_live::MiniMaxLive;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Reads APEIRETH_API_KEY env or openclaw file
    let client = MiniMaxLive::from_env()?;
    let mp3_bytes = client.text_to_speech(
        "hello apeireth, this is R172 LIVE MiniMax TTS",
        None,  // default model: speech-2.6-hd
        None,  // default voice: male-qn-qingse
    ).await?;
    std::fs::write("output.mp3", mp3_bytes)?;
    Ok(())
}
```

Production endpoint: `https://api.minimaxi.com/v1/t2a_v2` (hex-encoded MP3 audio).

## Borrowed upstream references (per O-5)

- **Porcupine** (`@picovoice/porcupine`): offline on-device wake word detection. API surface in `lib.rs` mirrors Porcupine's keyword model.
- **pvrecorder** (`@picovoice/pvrecorder`): cross-platform audio stream (16kHz, 16-bit PCM, 512 frames). API surface in `lib.rs` mirrors pvrecorder's frame model.
- **OpenAI Realtime API** (GA 2024-12, protocol v1): event schemas, session lifecycle, ephemeral tokens. Implementation in `realtime.rs`.

## Quick start

### STUB facade (8 Porcupine/pvrecorder tools)

```rust
use apeireth_voice::{VoiceSdk, VoiceConfig, AudioFrame, VOICE_FRAME_LENGTH};

let sdk = VoiceSdk::new(VoiceConfig::default())?;
let frame = AudioFrame::new(vec![0i16; VOICE_FRAME_LENGTH as usize]);
let r = sdk.wake_word_detect(&frame).await;
// Returns Err(VoiceError::NotImplemented("apeireth_voice_wake_word_detect"))
// while STUB_MODE = true.
```

### Real HTTP (TTS / STT / voiceprint)

```rust
use apeireth_voice::VoiceRealImpl;

let client = VoiceRealImpl::new(
    Default::default(),
    "https://api.apeireth.com/v1".to_string(),
    std::env::var("APEIRETH_VOICE_API_KEY")?,
)?;
// Use voice_kind / lang / audio methods...
```

### Realtime protocol (OpenAI Realtime API schema, R153)

```rust
use apeireth_voice::realtime::{
    RealtimeModel, RealtimeSessionConfig, RealtimeTool, RealtimeVoice,
    ClientEvent, EphemeralTokenRequest,
};
use serde_json::json;

let cfg = RealtimeSessionConfig::new()
    .model(RealtimeModel::GptRealtimeMini)
    .voice(RealtimeVoice::Sage)
    .instructions("You are a concise voice assistant.")
    .add_tool(RealtimeTool::function(
        "get_weather",
        "Look up current weather",
        json!({"type": "object", "properties": {"city": {"type": "string"}}}),
    ));

cfg.validate()?;
let req: EphemeralTokenRequest = (&cfg).into();
let event = ClientEvent::SessionUpdate { config: cfg };
```

## Status

Part of the Apeireth workspace (74 active crates after R128 94→75 merge).

**No-fake**: every public type or trait documented in this crate is real.
**Run-no-fear**: cargo check --workspace passes (0 errors).

## Tests (R153 cumulative)

| Type | Count |
|------|-------|
| Lib unit (lib.rs + real.rs + realtime.rs) | 53 + 32 = 85 |
| Integration (stub in-process + wiremock + realtime) | 7 + 19 + 12 = 38 |
| **Total** | **91 + 0 failed** |

Run with `cargo test -p apeireth-voice`.

## Where to start

- Cargo.toml: see [dependencies](Cargo.toml) for upstream crates.
- src/lib.rs: see top-level doc comment for module-level overview.
- src/realtime.rs: OpenAI Realtime API protocol schema (R153).
- src/real.rs: HTTP client for TTS/STT/voiceprint.

## Examples

- `cargo run -p apeireth-voice --example voice_stub_demo` — STUB facade demo
- `cargo run -p apeireth-voice --example voice_real_demo` — Real HTTP demo
- `cargo run -p apeireth-voice --example realtime_session_demo` — Realtime protocol demo

## See also

- [Apeireth conventions](../../docs/conventions/README.md)
- [R153 report](../../docs/r153/r153-voice-realtime-protocol.md)
- [Apeireth roadmap](../../docs/pages-source/roadmap.md)

---

_Last-modified: 2026-08-13 (R153). Tracked in git log._
