# 5-Minute Quickstart & Contributor Guide

Welcome to **Apeireth**! This guide gets you up and running with the entire system — CLI, HTTP Gateway, and Desktop Companion — in under 5 minutes.

---

## ⚡ 1. Prerequisites (Check in 30 Seconds)

Ensure you have the following installed:
- **Rust toolchain** (1.97.1 or newer): `rustc --version`
- **Node.js** (v20+) & **pnpm** (v9+): `node -v && pnpm -v`
- *(Optional for desktop bundle)*: **Tauri prerequisites** for your OS ([Tauri Setup Guide](https://v2.tauri.app/start/prerequisites/))

---

## 🚀 2. Build and Test in 2 Commands

Clone the repository and verify the core workspace:

```bash
# 1. Clone
git clone https://github.com/Apeireth/apeireth-rust.git
cd apeireth-rust

# 2. Run all workspace tests (16 crates)
cargo test --workspace
```

---

## 💬 3. Run the CLI / Start the HTTP Gateway

### Option A: Interactive CLI Chat
```bash
cargo run -p apeireth-cli -- chat
```

### Option B: Launch the Local HTTP Gateway (Port 8080)
```bash
cargo run -p apeireth-cli -- gateway serve --port 8080
```

Test the health endpoint:
```bash
curl http://127.0.0.1:8080/health
# {"status":"ok","version":"1.2.0","engine":"apeireth"}
```

---

## 🖥️ 4. Launch the Desktop Companion (UI)

```bash
cd frontend/companion-desktop
pnpm install
pnpm tauri dev
```

---

## 🌟 5. Good First Issues for New Contributors

Looking for a great first contribution? Check out these areas:

1. **Add new Hybrid Search ranking benchmarks** (`crates/engine/memory/src/hybrid_search.rs`)
2. **Implement an additional Voice TTS Emotion profile** (`crates/engine/perception/src/voice/emotion_voice.rs`)
3. **Add new activity scene presets for Ambient Context** (`crates/foundation/orchestration/src/ambient_context.rs`)
4. **Enhance UI animations in Svelte 5 desktop companion** (`frontend/companion-desktop/src/`)
5. **Add documentation examples for MCP tool integration** (`docs/02-guides/`)

Feel free to open an issue or pull request — we are friendly and provide rapid reviews!
