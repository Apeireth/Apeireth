**Source**: https://raw.githubusercontent.com/Lumio-Research/hermes-agent-rs/main/README.md

---

# ⚡ Hermes Agent `v0.1`

**The self-evolving AI agent. One binary. Every platform.**

Rust rewrite of [Hermes Agent](https://github.com/NousResearch/hermes-agent) by [Nous Research](https://nousresearch.com).

`110,000+ lines of Rust` · `1,428 tests` · `17 crates` · `~16MB binary`

**[English](./README.md)** · **[中文](./README_ZH.md)** · **[日本語](./README_JA.md)** · **[한국어](./README_KO.md)**



---

> **v0.1 Status:** Core agent loop, 10 LLM providers, 30 tool backends, 17 platform adapters, memory system, and CLI/TUI are production-ready.

## Project Status

This repository is the Lumio Research Rust port of Hermes Agent. The reference
implementation baseline is
[`NousResearch/hermes-agent` `v2026.4.13`](https://github.com/NousResearch/hermes-agent/tree/v2026.4.13),
resolved to commit `1af2e18d408a9dcc2c61d6fc1eef5c6667f8e254`.

The `main` branch includes OpenAI-compatible function tool payload normalization
for strict providers such as DeepSeek, Moonshot/Kimi, SiliconFlow, and other
OpenAI-compatible endpoints. Users on the `v0.1.1` release binaries should build
from `main` or use a release newer than `v0.1.1` for that fix.

## Why Hermes?

🚀 **Zero dependencies** — Single static binary. No Python, no pip, no Docker. Copy it to a Raspberry Pi, a $3 VPS, or an air-gapped server and run it.

🧠 **Self-evolution engine** — Multi-armed bandit model selection, long-task planning, and prompt/memory shaping. The agent gets better the more you use it.

🔌 **17 platforms, 30+ tools, 8 memory backends** — Telegram, Discord, Slack, WhatsApp, Signal, Matrix, and 11 more. File ops, browser, code execution, vision, voice, web search, Home Assistant, and beyond.

⚡ **True concurrency** — Rust's tokio runtime dispatches tool calls across OS threads. A 30-second browser scrape doesn't block a 50ms file read. No GIL.

## Quick Start

```bash
# Install (auto-detects OS and CPU)
curl -fsSL https://raw.githubusercontent.com/Lumio-Research/hermes-agent-rs/main/scripts/install.sh | bash

# Set your API key
echo "ANTHROPIC_API_KEY=sk-..." >> ~/.hermes/.env

# Run interactive session
hermes
```

That's it. You're in an interactive session with tool access, memory, and streaming.

## What Can It Do?

**Chat with any LLM** — switch models mid-conversation:

```
hermes
> /model gpt-4o
> Summarize this repo and find security issues
```

**One-shot tasks** from the command line:

```bash
hermes chat --query "Refactor auth.rs to use the new error types"
```

**Start the full server** — API + platform gateway + cron, all in one:

```bash
hermes serve                          # API + gateway + cron
hermes serve --no-gateway             # API only (for dashboard UI)
hermes serve --no-gateway --no-cron   # API only (minimal)
```

**Multi-platform gateway** — connect Telegram, Discord, Slack, and more simultaneously:

```bash
hermes gateway start
```

**Run anywhere** — Docker, SSH, or remote sandboxes:

```yaml
# ~/.hermes/config.yaml
terminal:
  backend: docker
  image: ubuntu:24.04
```

**MCP + ACP** — connect external tool servers or expose Hermes as one:

```yaml
mcp:
  servers:
    - name: my-tools
      command: npx my-mcp-server
```

**Voice mode** — VAD + STT + TTS pipeline for hands-free interaction.

## Architecture

```
hermes-agent-rs/
├── crates/
│   ├── hermes-cli              # Binary entry point, TUI, slash commands
│   ├── hermes-agent            # Agent loop, LLM providers, memory plugins
│   ├── hermes-core             # Shared types, traits, error hierarchy
│   ├── hermes-intelligence     # Model routing, prompt building, self-evolution
│   ├── hermes-config           # Config loading, YAML/env merging
│   ├── hermes-tools            # 30+ tool backends, approval engine
│   ├── hermes-gateway          # 17 platform adapters, session management
│   ├── hermes-server           # HTTP/WebSocket API server
│   ├── hermes-runtime          # Unified runtime builder (server + gateway + cron)
│   ├── hermes-environments     # Terminal: Local/Docker/SSH/Daytona/Modal/Singularity
│   ├── hermes-mcp              # Model Context Protocol client/server
│   ├── hermes-acp              # Agent Communication Protocol
│   ├── hermes-skills           # Skill management and hub
│   ├── hermes-cron             # Cron scheduling
│   ├── hermes-bus              # In-process message bus (agent service abstraction)
│   ├── hermes-eval             # SWE-bench, Terminal-Bench, YC Bench
│   └── hermes-telemetry        # OpenTelemetry + Prometheus
├── apps/
│   ├── dashboard/              # Web management UI (React + Vite + Tailwind)
│   ├── web-app/                # Web app + Electron desktop shell (macOS/Windows/Linux)
│   └── mobile-app/             # Mobile client (Flutter — iOS/Android)
└── scripts/                    # Install, CI, and convenience scripts
```

**Key traits:** `LlmProvider` (10 providers) · `ToolHandler` (30+ backends) · `PlatformAdapter` (17 platforms) · `TerminalBackend` (6 backends) · `MemoryProvider` (8 plugins) · `AgentService` (local/remote transparent)

**Tool call parsers:** Hermes, Anthropic, OpenAI, Qwen, Llama, DeepSeek, Auto

## CLI Commands

```
hermes                    # Interactive REPL (default)
hermes chat               # Single-shot query
hermes serve              # Start API server + gateway + cron
hermes gateway            # Platform gateway only (no API)
hermes config             # Configuration management
hermes model              # Model / provider management
hermes auth               # Authentication (login/logout/status)
hermes tools              # Tool management
hermes mcp                # MCP server management
hermes skills             # Skill management
hermes plugins            # Plugin management
hermes sessions           # Session management
hermes memory             # Memory provider management
hermes cron               # Cron job management
hermes setup              # Interactive setup wizard
hermes doctor             # Health check
hermes status             # Running status
hermes version            # Version info
```

## Install

**One-liner** (auto-detects OS and CPU):

```bash
curl -fsSL https://raw.githubusercontent.com/Lumio-Research/hermes-agent-rs/main/scripts/install.sh | bash
```

**From source:**

```bash
cargo install --git https://github.com/Lumio-Research/hermes-agent-rs hermes-cli --locked
```

**Manual download:** grab the binary for your platform from [Releases](https://github.com/Lumio-Research/hermes-agent-rs/releases).

**Docker:**

```bash
docker run --rm -it -v ~/.hermes:/root/.hermes ghcr.io/lumio-research/hermes-agent-rs
```

## Development

**Prerequisites:** Rust 1.75+, Node.js 20+ (for dashboard/client apps)

```bash
# Build the CLI
cargo build -p hermes-cli

# Run directly (skips cargo lock contention with rust-analyzer)
scripts/hermes

# Run tests
cargo test --workspace

# Lint
cargo clippy --workspace --all-targets -- -D warnings

# Format
cargo fmt --all --check
```

**Dashboard UI** (web management panel):

```bash
cd apps/dashboard
npm install
npm run dev          # Vite dev server with HMR, proxies /api to hermes serve
```

**Client app** (Web + Desktop shared project):

```bash
cd apps/web-app
pnpm install
pnpm dev:web
```

**Electron desktop shell** (Linux / macOS / Windows):

```bash
cd apps/web-app
pnpm dev:desktop
```

Package artifacts:

```bash
pnpm dist
```

Parity checklist is tracked in `apps/web-app/electron/TAURI_PARITY.md`.

**Mobile app** (React Native + Expo — iOS / Android):

```bash
cd apps/mobile-app
pnpm install
pnpm ios            # iOS simulator
pnpm android        # Android emulator
pnpm start          # Expo dev server (Expo Go)
```

Shares types and API client with `apps/web-app`. Features:
- Session list + new chat
- Streaming chat via WebSocket
- Settings (API URL, token) persisted with AsyncStorage

**Protocol SDK** (IDE / external client integration):

```bash
cd sdk/typescript
pnpm install
pnpm build
pnpm test            # protocol contract tests
HERMES_E2E_BASE=http://127.0.0.1:8787 pnpm test:e2e
HERMES_SCHEMA_BASE=http://127.0.0.1:8787 pnpm sync:protocol
```

**One-command local protocol e2e** (auto start/stop `hermes-server`):

```bash
bash scripts/ci/protocol-e2e-local.sh
```

## Contributing

Contributions welcome. Before submitting a PR:

1. Run the full test suite:
   ```bash
   cargo test --workspace                                    # 1,428 tests
   cargo clippy --workspace --all-targets -- -D warnings     # Lint (warnings fail CI)
   cargo fmt --all --check                                   # Format
   ```

2. Run smoke tests:
   ```bash
   bash scripts/ci/smoke.sh          # Release binary smoke check
   bash scripts/ci/keypath-e2e.sh    # Core end-to-end paths
   bash scripts/ci/protocol-sdk-check.sh   # Protocol SDK contracts + optional e2e smoke
   bash scripts/ci/protocol-e2e-local.sh   # Local server + protocol SDK e2e smoke
   bash scripts/ci/electron-check.sh       # Electron shell build check
   bash scripts/ci/mobile-check.sh         # React Native type check + tests
   bash scripts/ci/protocol-matrix-check.sh # Rust↔TS↔Flutter protocol fixtures compatibility
   ```

3. If you changed `apps/dashboard`:
   ```bash
   cd apps/dashboard && npm run build
   ```

See [AGENTS.md](AGENTS.md) for architecture details and coding conventions.
For multi-client CI/CD operations, see [CI_CD_RUNBOOK.md](CI_CD_RUNBOOK.md).

## License

MIT — see [LICENSE](LICENSE).

Based on [Hermes Agent](https://github.com/NousResearch/hermes-agent) by [Nous Research](https://nousresearch.com).