# Quick Start

> Verified against master (2026-08-18). Rust toolchain: stable (rust-toolchain.toml pinned 1.97.1).

## Prerequisites

- Rust stable (rust-toolchain.toml)
- Windows 10+ (Job Object sandbox) / Linux/macOS (some Windows-only features degrade honestly)
- Optional: MiniMax API key for real LLM features

## Build

```bash
cargo build --workspace          # ~34 万行, 85 + 1 desktop crates
cargo check --workspace --all-targets   # 编译全 target 干净 (含 post-1.0.0 cron 改进)
```

## Test

```bash
cargo test --workspace           # 23,874 组全绿 (含 post-1.0.0 增量)
cargo test -p apeireth-cron --test integration_cron  # cron integration tests 25 cases
```

## Run the Companion (the full partner endpoint)

```powershell
# PowerShell:
$env:APEIRETH_API_KEY = (Get-Content C:\path\to\your-key.txt -Raw).Trim()
cargo run -p apeireth-companion --example companion_serve
```

Then talk to her at `http://127.0.0.1:8090/v1` (OpenAI-compatible, any non-empty key):

```bash
curl http://127.0.0.1:8090/v1/chat/completions \
  -H "Content-Type: application/json" -H "Authorization: Bearer any" \
  -d '{"model":"MiniMax-M3","messages":[{"role":"user","content":"你好"}]}'
```

What you get: L0/L1 injection, memory (persistent, `%APPDATA%\apeireth\memory.sqlite`), daemon resident (dream/reflect/utter), tool bridge, approval queue at `/v1/apeireth/approval-requests`.

Optional env:

| Var | Meaning |
|---|---|
| `APEIRETH_SEED_MEMORY` | seed memories (semicolon-separated) |
| `APEIRETH_GRANT` | explicit tool grant, e.g. `FileOperator:24` (hours) |
| `APEIRETH_DREAM_QUIET_SECONDS` | dream quiet period (default 6h) |
| `APEIRETH_LARK_*` / `APEIRETH_TELEGRAM_*` | delivery sinks (optional) |

## TUI

```bash
cargo run -p apeireth-tui
```

## Desktop 伙伴 (companion-desktop, post-v1.0.0 新增)

```bash
# 前置: Node 20+ + pnpm 9+ (Windows: WebView2 runtime)
cd frontend/companion-desktop
pnpm install
pnpm dev                                      # http://localhost:1420 (Vite + Svelte)

# 或: Tauri 桌面窗口 (需 WebView2)
pnpm tauri dev                                 # 桌面窗口 + dev tools
```

详见 [`frontend/companion-desktop/README.md`](../../frontend/companion-desktop/README.md)。 真实 LLM E2E 待 `APEIRETH_API_KEY` (目前是 mock SSE 端到端, **🟡 2026-08-19 TP34 后端 50%** — companion_serve 加 streaming 分支 + `extract_minimax_cot` helper, 6 种 RuntimeEvent 中 reasoning-delta 前端状态机待续, 详见 `docs/04-internal/next-team-handbook.md` TP34)。

## Tool orchestration e2e

```bash
cargo run -p apeireth-integration-e2e --example tool_orchestrator_e2e
cargo run -p apeireth-companion --example tp_acceptance_sim   # TP 验收模拟 4/4
```

## More

- [User manual](../02-guides/user-manual.md)
- [Architecture](../01-architecture/architecture.md)
- [Security model](../01-architecture/security.md)
- [Crate index](../03-reference/crates.md)
