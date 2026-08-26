# Apeireth 桌面伙伴 (companion-desktop)

Svelte 5 + Tauri 2 桌面 App, **薄 Tauri shell + 现有 apeireth-companion 后端**.
对接 `apeireth-companion` 的 OpenAI 兼容 HTTP/SSE 端点 (`POST /v1/chat/completions`).

## 架构

```
┌─────────────────────────────────────────┐
│ Tauri shell (Rust, ~110 lines)          │   窗口 + 托盘 + 通知 + 全局快捷键
│  frontend/companion-desktop/src-tauri/  │   0 apeireth-* deps (隔离)
└────────────────┬────────────────────────┘
                 │ IPC
┌────────────────┴────────────────────────┐
│ Svelte 5 UI (~390 lines + deps)         │   App.svelte / runtime.ts / 主题 / Markdown
│  frontend/companion-desktop/src/       │   runtime.ts = OpenAI-compatible adapter
└────────────────┬────────────────────────┘
                 │ HTTP/SSE
┌────────────────┴────────────────────────┐
│ apeireth-companion :8090                 │   已存在, 不变 (per R20 §3.1)
│  OpenAI compatible chat completions     │
└─────────────────────────────────────────┘
```

- **Tauri shell** 不持任何业务逻辑 — 对话/记忆/工具/宪法都在 `apeireth-companion` 后端
- **Svelte 5 UI** 把 `runtime.ts` 当契约, 不裸碰后端
- **设计参考**: 移植 Pattern 项目 (`App.svelte` / `runtime.ts` / CSS) 到 apeireth, 改传输层

## 开发

### 前置

- Rust stable (>= 1.77.2)
- Node 20+
- pnpm 9+
- Windows: WebView2 runtime (Win10 1803+ 默认装)
- macOS: Xcode CLI tools
- Linux: webkit2gtk-4.1, libgtk-3-dev, libayatana-appindicator3-dev, librsvg2-dev

### 启动

```bash
# 1. 启 apeireth-companion 后端 (另一个 terminal)
cd crates/apeireth-companion
cargo run --bin companion_serve    # 监听 :8090

# 2. 启前端 dev server (Vite + Svelte)
cd frontend/companion-desktop
pnpm install
pnpm dev                            # http://localhost:1420

# 3. (optional) 启 Tauri 桌面 (需 WebView2 on Win)
pnpm tauri dev                      # 桌面窗口 + dev tools
```

### 验证

```bash
# TypeScript / Svelte 5 类型检查
pnpm check                          # = svelte-check --tsconfig ./tsconfig.json

# Rust (Tauri shell) 静态检查
cd src-tauri
cargo check --workspace --all-targets

# 集成 e2e (mock OpenAI SSE, 不需要真 LLM key)
node tests/mock-openai-sse.mjs &  # mock upstream on :9999
APEIRETH_LLM_BACKEND=scripted npx tsx tests/e2e-streamChat-test.mts
# 期望: accumulated = "APEIRETH_E2E_OK", PASS: true
```

### 打包

```bash
# 桌面 binary (.app / .exe / .AppImage per host)
pnpm tauri build

# multi-arch 需 QEMU + docker buildx (Linux host)
pnpm tauri build --target universal-apple-darwin    # macOS universal
pnpm tauri build --target x86_64-unknown-linux-gnu
pnpm tauri build --target aarch64-unknown-linux-gnu
```

## 设计文档 (per Phase 0-5)

- `docs/integration/phase0-audit.md` — Pattern → Apeireth 可行性
- `docs/integration/architecture.md` — UI → Agent Runtime Contract (方案 C)
- `docs/integration/legacy-audit.md` — Phase 3 legacy audit
- `docs/integration/runtime-bridge.md` — runtime.ts 契约 (§15)
- `docs/integration/phase5-report.md` — Phase 5 E2E 验证 + 已知 follow-up
- `docs/integration/native-readiness.md` — Tauri 多平台 native 准备

## CI

`.github/workflows/companion-desktop-ci.yml` — 独立 CI gate:

- `cargo check` (Tauri shell) on ubuntu-latest
- `pnpm install` + `pnpm check` (svelte-check)
- 8 硬墙守门: 0 触碰 24 LOCKED crate, workspace.version 1.2.0 不变,
  独立 workspace 守门

触发: push master (companion-desktop/**) + PR touch 它 + manual dispatch.

## 已知 follow-up

- **Real LLM E2E** (deferred, 无 APEIRETH_API_KEY in CI) — `phase5-report.md §已知`
- **macOS universal binary** (deferred, 仅 Windows + WebView2 验证)
- **Linux native packaging** (Tauri + .deb/.rpm/AppImage) — 跟根 release pipeline 独立
- **Tauri shell rustfmt / clippy 守门** — companion-desktop-ci.yml 可加, 当前只 check

## 仓库边界 (per 8 硬墙)

- `companion-desktop/src-tauri/Cargo.toml` 顶层有 `[workspace]` — **不污染 root cargo workspace**
- `cargo test --workspace` (root) **不会碰 Tauri shell** — companion-desktop-ci.yml 单独管
- 0 apeireth-* 依赖 (Tauri shell 只用 tauri + serde + serde_json)
- runtime.ts 用 HTTP/SSE 字符串调 `apeireth-api` 端点 (不算编译依赖)
