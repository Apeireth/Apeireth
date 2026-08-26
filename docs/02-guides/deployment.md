# Deployment Guide

> 当前基线：`apeireth-cli` 启动 `apeireth-gateway`，由
> `apeireth-runtime::canonical::Runtime` 统一编排。

## Environment

The default credential resolver reads provider keys at execution time:

| Provider | Environment variable |
| --- | --- |
| MiniMax | `APEIRETH_API_KEY` |
| Anthropic | `APEIRETH_ANTHROPIC_KEY` |
| OpenAI-compatible | `OPENAI_API_KEY` (enable models with `APEIRETH_OPENAI_MODELS`) |

No key is stored in a long-lived runtime or provider object. Keep local values
in the environment or an ignored local configuration file.

## Run the gateway

```powershell
$env:APEIRETH_API_KEY = (Get-Content C:\path\to\your-key.txt -Raw).Trim()
cargo run -p apeireth-cli -- gateway serve --port 8080
```

The health endpoint is `GET http://127.0.0.1:8080/health`. Chat requests use
`POST /v1/chat` for the native canonical shape or
`POST /v1/chat/completions` for the OpenAI-compatible shape.

Example:

```bash
curl http://127.0.0.1:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"MiniMax-M3","messages":[{"role":"user","content":"你好"}]}'
```

## Docker

The root `Dockerfile` builds the current root workspace and starts the same
CLI gateway. Use the compose file only after supplying the required external
database password:

```bash
export POSTGRES_PASSWORD='use-a-local-secret'
docker compose up --build
```

If Docker is unavailable, validate the same source path locally with:

```bash
cargo build --release --workspace --locked
```

## Desktop

The desktop app is deployed independently from
`frontend/companion-desktop/`. See its README for Node, pnpm, WebView2, and
Tauri platform prerequisites. It is a thin UI/shell and does not replace the
canonical runtime or `ProcessExecutor`.

## Platform limits

Windows uses the tested Job Object path with suspended spawn and pre-resume
attachment. Linux and macOS retain their existing process-group containment
semantics, which are documented as partial. Filesystem isolation, process-level
network isolation, `ProcessSupervisor`, and runtime security are not provided by
this baseline.
