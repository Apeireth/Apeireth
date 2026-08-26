# apeireth-acp

> apeireth-acp — R23 acp 子模块: Agent Communication Protocol 抽象 + 信封 + 路由 + llm_facade (R177 6 provider facade)

apeireth-acp 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。

## 模块 (3 文件 / 38 测试 + 2 Kani proof)

- `src/lib.rs` — Envelope / AcpError / 7 顶层 pub fn (checksum/verify/sequence_number/payload_equivalent/matches_pair/to_json_string/from_json_string) + 13 测试
- `src/llm_facade.rs` — R177 LlmRequest/LlmResponse/LlmStatus (5 状态) + ALL_PROVIDER_NAMES (6: claude-code/codex/copilot/gemini-cli/opencode/minimax) + 15 测试
- `src/organ_kani_proofs.rs` — R177 acp organ Kani proofs (W3+W4) — 10 测试 + 2 `#[kani::proof]`
