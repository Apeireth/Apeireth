# Quick Start

> 当前基线：根 workspace 13 个 Rust crate；桌面端是独立的
> `frontend/companion-desktop` workspace。工具链由 `rust-toolchain.toml` 锁定。

## Prerequisites

- Rust toolchain from `rust-toolchain.toml`
- Node 20+ and pnpm 9+ for the desktop frontend
- An API key only when running a real provider-backed chat

## Build and test

```bash
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo fmt --all -- --check
```

The canonical process-isolation integration test is:

```bash
cargo test -p apeireth-tools-canonical --test process_executor --locked -- --nocapture
```

## Run the canonical gateway

```bash
cargo run -p apeireth-cli -- gateway serve --port 8080
```

The gateway is backed by one `apeireth-runtime::canonical::Runtime`:

```text
GET  http://127.0.0.1:8080/health
POST http://127.0.0.1:8080/v1/chat
POST http://127.0.0.1:8080/v1/chat/completions
```

For a provider-backed turn, configure the resolver before starting the
gateway. The default mappings are `APEIRETH_API_KEY` for MiniMax,
`APEIRETH_ANTHROPIC_KEY` for Anthropic, and `OPENAI_API_KEY` for the optional
OpenAI-compatible provider.

```powershell
$env:APEIRETH_API_KEY = (Get-Content C:\path\to\your-key.txt -Raw).Trim()
cargo run -p apeireth-cli -- gateway serve --port 8080
```

## CLI

```bash
cargo run -p apeireth-cli -- --help
cargo run -p apeireth-cli -- session
cargo run -p apeireth-cli -- chat "hello" --model MiniMax-M3
```

## Desktop frontend

```bash
cd frontend/companion-desktop
pnpm install --frozen-lockfile
pnpm check
pnpm build
pnpm dev
```

The frontend talks to the gateway over HTTP. Its mock upstream and streaming
integration test are kept with the frontend at
`frontend/companion-desktop/tests/`.

## More

- [Repository layout and ownership](../development/repository-layout.md)
- [Current architecture](../01-architecture/architecture.md)
- [API reference](../03-reference/api.md)
- [Desktop frontend guide](../../frontend/companion-desktop/README.md)
