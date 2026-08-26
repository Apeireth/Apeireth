# apeireth-voice

> Apeireth voice subsystem

apeireth-voice 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。

## 模块 (8 src 文件 / 91 测试 + 1 Kani proof + 23 集成)

- `src/lib.rs` — voice subsystem facade (VoiceMode enum: Stub/Real/Realtime) + 7 测试
- `src/real.rs` — 真接实现 (reqwest 0.12 + rustls-tls, TTS/STT 多供应商) + 8 测试
- `src/realtime.rs` — R176 realtime session (WebSocket 风格状态机) + 37 测试
- `src/tone.rs` — EmotionTone / Prosody (per R176 bridge 输入) + 4 测试
- `src/minimax_live.rs` — MiniMax-Live 供应商适配 + 11 测试
- `src/consciousness_bridge.rs` — R176 bridge 4: consciousness → voice (Plutchik → Tone) + 10 测试
- `src/companion_bridge.rs` — R176 bridge 8: companion → voice (Bond → Tone) + 10 测试
- `src/bridge_kani_proofs.rs` — R176 bridge 4+8 Kani proofs (4 测试 + 1 `#[kani::proof]`)
- 集成测试: `tests/test_voice_stub_in_process.rs` (6) + `tests/test_voice_real_wiremock.rs` (5) + `tests/test_voice_realtime.rs` (12)
- 例: `examples/voice_stub_demo.rs`, `examples/voice_real_demo.rs`, `examples/realtime_session_demo.rs`
- bench: `benches/bench.rs` (criterion)
