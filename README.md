# Apeireth — An AGI Operating System / LLM Base

> **[English](README.md) | [中文](README.zh-CN.md)**

> *「5 years from now, he will smile and tell me what he improved today — happy because of me, sad because he didn't do well somewhere」* — the owner, 2026-08-15

A Rust-written **AGI operating system** that gives an LLM a **home**: memory, security boundaries, tools, and active companionship. Not a chatbot — a **companion** that lives across sessions.

**Philosophy**: emergence over predefinition — *「I don't want its abilities to be entirely predefined by us; I want it to evolve on its own」*.

## Status (v1.0.0 — 2026-08-18)

| | |
|---|---|
| Version | **v1.0.0** (product axis; workspace crates 1.2.0) |
| Active crates | 84 (~340K lines of Rust) |
| Tests | `cargo test --workspace` **368 suites, 0 failures** (incl. real-API stress with backoff) |
| Build | `cargo check --workspace --all-targets` clean |
| License | Apache-2.0 |

## What She Can Do

- **Remember you** — memory v2 (importance/reconcile/ranking), memory graph (temporal causal facts), rolling summaries, dream consolidation, emotion timeline (F1)
- **Think about you** — world model (W1 text simulation + W2/W3 causal graph, Brier-calibrated), curiosity engine (E4, memory-echo biased), hypothesis testing (F4), value cases (F6)
- **Act safely** — 9 tool sub-crates with schema validation + guardrails, 5-rule approval + ApprovalBridge, double-onion permissions, Job Object sandbox, outbound default-deny with audit chain (S4)
- **Be with you** — emergence loop (E7: when to speak, learned from your responses), rhythm learning, mood-aware gating, proactive delivery (SSE/Lark/Telegram)
- **Run anywhere** — companion_serve (OpenAI-compatible partner endpoint), TUI, CLI

## Quick Start

```bash
cargo build --workspace

# PowerShell:
$env:APEIRETH_API_KEY = (Get-Content C:\path\to\your-key.txt -Raw).Trim()
cargo run -p apeireth-companion --example companion_serve   # :8090, OpenAI-compatible

curl http://127.0.0.1:8090/v1/chat/completions \
  -H "Content-Type: application/json" -H "Authorization: Bearer any" \
  -d '{"model":"MiniMax-M3","messages":[{"role":"user","content":"你好"}]}'
```

Full guide: [docs/02-guides/quick-start.md](docs/02-guides/quick-start.md)

## Documentation

- [docs/ (index)](docs/README.md)
- Architecture: [vision](docs/01-architecture/vision.md) · [philosophy](docs/01-architecture/philosophy.md) · [architecture](docs/01-architecture/architecture.md) · [security](docs/01-architecture/security.md)
- Reference: [85 crates](docs/03-reference/crates.md)
- Release notes: [RELEASE_NOTES.md](RELEASE_NOTES.md)

## License

Apache-2.0 — see [LICENSE](LICENSE).
