# Apeireth 桌面伙伴 (companion-desktop)

Svelte 5 + Tauri 2 桌面 App，**独立的薄 Tauri shell + 前端 canonical gateway 客户端**。
它不属于根 Rust workspace；根 workspace 的 canonical gateway 以 **bundled-backend 侧车**
方式随桌面分发（Tauri `externalBin`），由 `src-tauri/src/backend_supervisor.rs`
在启动时 spawn 并探活；前端 `runtime.ts` 全程讲 canonical Apeireth 2.0 gateway
协议（`/health`、`/v1/chat/completions`、`/v1/apeireth/events`、panel 内省），
旧 companion :8090 兼容层已不再承载主链路。

## 架构

```
┌─────────────────────────────────────────┐
│ Tauri shell (Rust)                      │   窗口 + 托盘 + 通知 + 单实例
│  frontend/companion-desktop/src-tauri/  │   0 apeireth-* 依赖 (进程隔离)
│  backend_supervisor.rs                  │   spawn + 探活 + 优雅回收侧车
└────────────────┬────────────────────────┘
                 │ spawn (externalBin sidecar)
┌────────────────┴────────────────────────┐
│ apeireth gateway serve                  │   canonical v2 runtime（根 workspace 产出）
│  自动选端口 → 前端 launch 时解析         │   /health · /v1/chat/completions · events · panel
└────────────────┬────────────────────────┘
                 │ HTTP/SSE (canonical gateway 协议)
┌────────────────┴────────────────────────┐
│ Svelte 5 UI                             │   App.svelte / runtime.ts / 面板内省
│  frontend/companion-desktop/src/        │   runtime.ts = canonical gateway 客户端
└─────────────────────────────────────────┘
```

- **Tauri shell** 不持任何业务逻辑 — 只负责侧车生命周期（`backend_supervisor`：
  bundled 侧车优先、dev 下回落 workspace 构建；`kill_on_drop` 优雅回收；端口冲突
  自动换口，前端 `desktop-bridge.ts` 每次启动重新解析端点）
- **Svelte 5 UI** 把 `runtime.ts` 当契约，不裸碰后端；面板内省对 gateway 未实现的
  API 显式报"不支持"（0 装），不伪造数据
- **装机实证 (2026-09-08)**：装机 E2E 确认桌面端启动后自行拉起
  `apeireth gateway serve` 并探活——bundled-backend 设计真机验证通过

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
# 1. 启根 workspace 的 canonical gateway（用于当前 canonical API smoke；另一个 terminal）
cargo run --locked --bin apeireth -- gateway serve --port 8080

# 桌面完整的历史面板/自拉起流程仍需要仓库外的 companion_serve :8090；
# 它不属于当前根 workspace，完整迁移暂列 deferred。

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

# 仅 Windows NSIS 包（装机 E2E 用此产物）
pnpm tauri build --bundles nsis

# multi-arch 需 QEMU + docker buildx (Linux host)
pnpm tauri build --target universal-apple-darwin    # macOS universal
pnpm tauri build --target x86_64-unknown-linux-gnu
pnpm tauri build --target aarch64-unknown-linux-gnu
```

### 装机与卸载 (Windows NSIS)

```powershell
# 静默安装（perMachine，装到 C:\Program Files\Apeireth Companion）
.\Apeireth Companion_2.0.0-rc.1_x64-setup.exe /S

# 静默卸载：uninstall.exe /S
# 语义（installer.nsh hook，2026-09-08 修复）：卸载前先杀主程序再杀侧车
# （apeireth.exe）——侧车运行中卸载不再出现"删不掉文件却清掉注册表"的半卸载；
# 杀不掉进程则 Abort 非零，绝不静默留残留。MSI 包走 WiX 模板无此 hook，
# Windows 推荐用 NSIS 包。
```

装机 E2E（全自动，需 DeepSeek 兼容 env）：

```powershell
$env:OPENAI_API_KEY='sk-...'; $env:APEIRETH_OPENAI_URL='https://api.deepseek.com/v1'; $env:APEIRETH_OPENAI_MODELS='deepseek-v4-flash'
pwsh scripts/install-e2e.ps1   # 装机→真聊天→gateway→桌面冒烟→孤儿复现→卸载残留检查
```

## 设计文档（历史集成记录）

- `docs/integration/README.md` — 说明这些文件均为历史记录
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
- workspace boundary 守门: 根 workspace 与 Tauri workspace 分离,
  独立 workspace 守门

触发: push master (companion-desktop/**) + PR touch 它 + manual dispatch.

## 已知 follow-up

- **真 LLM 对话 E2E 已在根 workspace 打通**（DeepSeek，`#[ignore]` live 测试：
  provider factory / CLI 双轮 / organ W1+W2 / Council 7-advisor）；桌面装机 E2E
  `scripts/install-e2e.ps1` 同样走真模型。CI 仍无 key，`tests/mock-openai-sse.mjs`
  保持 mock 路径（`phase5-report.md §已知`）。
- **真实流式**：SSE 事件端点存在，但缓冲默认关（B2 纪律），前端消费真实流式待授权
- **macOS universal binary** (deferred, 仅 Windows + WebView2 验证)
- **Linux native packaging** (Tauri + .deb/.rpm/AppImage) — 跟根 release pipeline 独立
- **MSI 卸载与 NSIS 对齐**：WiX 模板无侧车进程检查，卸载时侧车运行中可能留文件

## 仓库边界 (per 8 硬墙)

- `companion-desktop/src-tauri/Cargo.toml` 顶层有 `[workspace]` — **不污染 root cargo workspace**
- `cargo test --workspace` (root) **不会碰 Tauri shell** — companion-desktop-ci.yml 单独管
- 0 apeireth-* 依赖 (Tauri shell 只用 tauri + plugins + serde + serde_json + tokio + reqwest + fs-err；canonical 侧车以进程 spawn，不链接)
- runtime.ts 通过 canonical gateway HTTP/SSE 协议对接后端；旧 companion/API 端点字符串仅属于
  历史测试 fixture，不代表当前主链路。
