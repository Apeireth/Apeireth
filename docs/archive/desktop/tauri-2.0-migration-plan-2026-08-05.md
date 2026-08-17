# Tauri 2.0 迁移计划 (R20 阶段 5 估补)

**日期**: 2026-08-05
**作者**: R20 阶段 5 sub-agent
**状态**: Skeleton (1:1 翻译框架, 完整业务逻辑待 1 owner × 1 周接管)
**关联**: v0.9.21 商业版 Electron → Tauri 2.0 终极前端路线

---

## §0 背景与决策 (per 主人 2026-08-04 拍板)

> 主人: "我们最后要做的前端应该是 Tauri, 但由于现在手头的 ai 团队没有适合干尤其是审美设计的, 所以 web 和桌面都搁置, 先做好 tui 来为桌面做准备."

**终极路线**: TUI (现在) → Tauri 2.0 (终极, 等设计团队到位)
**TUI 角色**: 瘦客户端 (HTTP to apeireth-api), 是 Tauri 2.0 的"集成测试床"
**Tauri 2.0 角色**: 1.0 release 终极前端, 1:1 翻译 v0.9.21 商业版 Electron API

---

## §1 1:1 翻译表 (Electron API → Tauri 2.0 API)

| Electron API              | Tauri 2.0 API                          | scaffold 模块        | 状态     |
|---------------------------|----------------------------------------|----------------------|----------|
| `BrowserWindow`           | `tauri::WebviewWindow`                 | `src-tauri/src/window.rs` | ✅ Skeleton |
| `Menu.setApplicationMenu` | `tauri::menu::Menu` + `app.set_menu()` | `src-tauri/src/menu.rs`   | ✅ Skeleton |
| `Tray`                    | `tauri::tray::TrayIconBuilder`         | `src-tauri/src/tray.rs`   | ✅ Skeleton |
| `ipcMain.handle`          | `#[tauri::command]` + `invoke_handler` | `src-tauri/src/ipc.rs`    | ✅ Skeleton |
| `dialog.showOpenDialog`   | `tauri-plugin-dialog`                  | `src-tauri/src/ipc.rs`    | ✅ Skeleton |
| `shell.openPath`          | `tauri-plugin-opener`                  | `src-tauri/src/ipc.rs`    | ✅ Skeleton |
| `shell.openExternal`      | `tauri-plugin-shell`                   | `src-tauri/src/ipc.rs`    | ✅ Skeleton |
| `app.whenReady`           | `.setup(|app| { ... })`                | `src-tauri/src/lib.rs`    | ✅ Skeleton |
| `Notification`            | `tauri-plugin-notification`            | (W2 接入)               | 🚧 待接管 |
| `app.getPath`             | `tauri::path::PathResolver`            | (W2 接入)               | 🚧 待接管 |
| `app.quit`                | `app.exit(0)`                          | `src-tauri/src/tray.rs`   | ✅ Skeleton |
| `webContents.send`        | `window.emit()` / `app.emit_to()`      | (W2 接入)               | 🚧 待接管 |

**Sources**:
- RIVAL blueprint §2.5.3 (1:1 翻译点)
- Tauri 2 官方文档 <https://tauri.app/v2/>
- v0.9.21 商业版 `out/main/index.js` (6 BrowserWindow / 8 Menu / 1 Tray / 161 ipcMain / 34 dialog / 102 shell)

---

## §2 8 个 Bundle Target (跨 5 平台, 跟 D-06 8 包对齐)

| Target   | Platform                | 状态     |
|----------|-------------------------|----------|
| `deb`    | Debian / Ubuntu         | ✅ Cargo.toml `[features]` |
| `rpm`    | Fedora / RHEL           | ✅ Cargo.toml `[features]` |
| `appimage` | Universal Linux       | ✅ Cargo.toml `[features]` |
| `msi`    | Windows (MSI installer) | ✅ Cargo.toml `[features]` |
| `dmg`    | macOS                   | ✅ Cargo.toml `[features]` |

**编译期 hardcode** (K-1 强校验 #2):
```rust
pub const TAURI_BUNDLE_TARGETS: &[&str] = &["deb", "rpm", "appimage", "msi", "dmg"];
```

**Tauri 2 tauri.conf.json** (`src-tauri/tauri.conf.json` §`bundle.targets`):
```json
"targets": ["deb", "rpm", "appimage", "msi", "dmg"]
```

---

## §3 集成点 (跟现有 5 P0 + 9 skeleton + 1 i18n + 1 observability crate 对接)

### 3.1 跟 `apeireth-tui` (TUI 瘦客户端) 集成
- TUI 已经是瘦客户端 (HTTP to apeireth-api), Tauri 2.0 同样走 HTTP
- **共享**: `apeireth-http-client` (workspace dep `reqwest`)
- **共享**: `apeireth-tool-runtime` (tool 调用)
- **共享**: `apeireth-i18n` (zh / en)
- **共享**: `apeireth-observability` (tracing → Tauri log plugin)

### 3.2 跟 `apeireth-i18n` (R20 阶段 1) 集成
- Tauri 2 菜单 label / tooltip / tray menu 用 `i18n.t("menu.file.new")`
- 5 顶层菜单 (File / Edit / View / Window / Help) 跟 apeireth-i18n 字段对齐
- 🚧 W2 接管: `MenuItemBuilder::with_id("file_new", i18n.t("menu.file.new"))`

### 3.3 跟 `apeireth-observability` (R20 阶段 1) 集成
- `tauri-plugin-log` 接到 `apeireth-observability` 的 tracing subscriber
- 🚧 W2 接管: `log::info!` → tracing span (apeireth_tauri_lib::ipc::invoke_tool)

### 3.4 跟 `apeireth-tauri-stub` (DEPRECATED, 参考) 集成
- **0 触碰**: 不修改 `crates/apeireth-tauri-stub/` (24 LOCKED 之一)
- **可参考**: 9 器官 snapshot / 5 Self / 阶段判据 — 1 owner 接管时复用真后端调用
- **路径**: `crates/apeireth-tauri-stub/src/main.rs` 26KB Tauri 代码可参考 (但 24 LOCKED 禁动)

### 3.5 跟 `apeireth-api` (HTTP server) 集成
- Tauri 2 IPC → HTTP 转发到 `apeireth-api` (R17 serve.rs 风格)
- 流式 (SSE) → `apeireth-tui` 同样的 StreamExt 处理
- 🚧 W2 接管: `invoke_stream` skeleton 改 HTTP SSE

---

## §4 资源 + 时间表 (1 owner × 1 周)

| 阶段 | 任务 | 估时 | 状态 |
|------|------|------|------|
| **阶段 5 (本任务)** | Tauri 2.0 scaffold skeleton (1:1 翻译 6 大 API) | 5-10 min | ✅ 完成 |
| **W1.D1-2** | 替换 skeleton → 真 IPC (invoke_tool/stream/state 调 apeireth-api) | 2 天 | 🚧 待接管 |
| **W1.D3** | 集成 apeireth-i18n (菜单/tray/tooltip 多语言) | 1 天 | 🚧 待接管 |
| **W1.D4** | 集成 apeireth-observability (tracing → tauri-plugin-log) | 1 天 | 🚧 待接管 |
| **W1.D5** | 端到端测试 (5 平台 bundle: deb/rpm/appimage/msi/dmg) | 1 天 | 🚧 待接管 |
| **W2.D1-3** | 跟 apeireth-tui 共享瘦客户端 (UI 替换) | 3 天 | 🚧 待设计 |
| **W2.D4-5** | 9 器官 UI 集成 (9 卡片 + 状态心跳) | 2 天 | 🚧 待设计 |

**总估时**: 1 owner × 1 周 (skeleton 阶段 5-10 min, 完整业务 7-10 天)

---

## §5 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| Tauri 2.0 还在稳定期 (vs Electron 已稳定) | 中 | skeleton 阶段先用 2.x 稳定 API, 等 1 owner 接管时 lock minor |
| Tauri 2 plugin 生态 vs Electron 内置 API | 中 | 8 个 plugin 覆盖 dialog/shell/notification/fs/log/process/window-state/single-instance, 几乎 1:1 覆盖 |
| 跨平台 bundle (5 平台) CI 复杂度 | 中 | 先跑 3 平台 (msi/dmg/deb) 验证, 后 2 平台 (rpm/appimage) 跟上 |
| 跟 24 LOCKED crate 集成 (0 触碰约束) | 低 | src-tauri 独立子目录 + 不在 workspace members + 0 改 LOCKED 路径 mtime |
| Cargo.lock 升级 (tauri 2.x minor) | 低 | workspace 不引用, src-tauri/Cargo.toml 独立 lock, 不影响其他 crate |
| 缺审美设计 (主人 2026-08-04 拍板) | 高 | TUI 优先 + Tauri 2 skeleton 框架先出, UI 设计等团队到位 |

---

## §6 严守规范 (8 项不修改承诺)

1. ✅ 0 改 `crates/apeireth-*/src/` 24 LOCKED crate
2. ✅ 0 改 `Apeireth-rust/Cargo.toml` workspace root
3. ✅ 0 改 7 LOCKED 文档
4. ✅ 0 git add / 0 git commit
5. ✅ 0 引 NewAPI
6. ✅ 0 引 electron / nw.js
7. ✅ 0 重复造轮子 (引用 4 份 reports 完整 path)
8. ✅ src-tauri 独立子目录, 不在 workspace members

**6 哲学锚**: S-1 (不假装) / S-2 (编译期 hardcode) / O-2 (0 改 LOCKED) / O-3 (0 引 electron) / O-4 (0 引 NewAPI) / O-5 (0 重复造轮子)

---

## §7 验收清单 (R20 阶段 5)

### 7.1 必出文件 (8 个)
- [x] `src-tauri/Cargo.toml` (估 1.5-1.8 KB) — Tauri 2.0 dep
- [x] `src-tauri/tauri.conf.json` (估 50-80 行) — Tauri 2.0 config
- [x] `src-tauri/src/main.rs` (估 100-150 行) — main 进程 entry
- [x] `src-tauri/src/window.rs` (估 60-100 行) — Window 配置
- [x] `src-tauri/src/menu.rs` (估 60-100 行) — Menu 配置
- [x] `src-tauri/src/tray.rs` (估 40-60 行) — Tray 图标
- [x] `src-tauri/src/ipc.rs` (估 80-120 行) — IPC 通信
- [x] `src-tauri/examples/tauri_demo.rs` (估 60-100 行) — scaffold demo
- [x] `docs/desktop/tauri-2.0-migration-plan-2026-08-05.md` (本文件)

### 7.2 编译期 hardcode 8 项 (K-1 强校验 #1 #2)
- [x] `TAURI_SCHEMA_VERSION = "2"`
- [x] `PLATFORM_NAME = "apeireth"`
- [x] `TAURI_MAIN_WINDOW_WIDTH = 1200`
- [x] `TAURI_MAIN_WINDOW_HEIGHT = 800`
- [x] `TAURI_MIN_WINDOW_WIDTH = 800`
- [x] `TAURI_MIN_WINDOW_HEIGHT = 600`
- [x] `TAURI_DEV_URL = "http://localhost:1420"`
- [x] `TAURI_BUNDLE_TARGETS = ["deb", "rpm", "appimage", "msi", "dmg"]`

### 7.3 m3 防御 (TOOL_WHITELIST 8 工具, K-1 强校验 #3)
- [x] `apeireth_tauri_window_create`
- [x] `apeireth_tauri_window_close`
- [x] `apeireth_tauri_menu_setup`
- [x] `apeireth_tauri_tray_create`
- [x] `apeireth_tauri_dialog_open`
- [x] `apeireth_tauri_shell_open`
- [x] `apeireth_tauri_ipc_invoke_tool`
- [x] `apeireth_tauri_ipc_invoke_stream`

### 7.4 K-1 强校验 4 条 (4 fixture 验证)
- [x] K-1.1 平台名 "apeireth" — `PLATFORM_NAME = "apeireth"`
- [x] K-1.2 5 bundle targets — `TAURI_BUNDLE_TARGETS.len() == 5`
- [x] K-1.3 TOOL_WHITELIST 8 名字 — `TOOL_WHITELIST.len() == 8` + 8 fixture
- [x] K-1.4 5 K-1 字样 — "apeireth" / "tauri" / "window" / "ipc" / "must-do"

### 7.5 cargo check 验证
- [x] `cargo check --manifest-path src-tauri/Cargo.toml` (0 error 0 warning, 见报告)

### 7.6 0 触碰约束实查 (4 LOCKED 路径 mtime 对比)
- [x] `crates/apeireth-tauri-stub/` (DEPRECATED) — mtime 不变
- [x] `Apeireth-rust/Cargo.toml` workspace root — mtime 不变
- [x] 7 LOCKED 文档 — mtime 不变
- [x] 24 LOCKED crate mtime baseline 对比 — 无变化

---

## §8 关联文档 (0 重复造轮子)

引用 4 份 reports 完整 path, 不重复造轮子:
- `docs/v2-strategy/05-EXECUTION-NOW.md` §Step 1.3 (R17 stub 创建)
- `docs/tech-review-2026-08-05.md` §P0-1 (tauri-stub 暂离 build)
- `reports/competitive-analysis-2026-08-05.md` (R20 阶段 0 竞品分析)
- `docs/Apeireth-v2.1-Industry-Top-Backend-Roadmap.md` (v2.1 后端路线)
- `crates/apeireth-tauri-stub/README.md` (R19 worker 接管路径)
- `crates/apeireth-tauri-stub/src/main.rs` (26KB Tauri 代码, **24 LOCKED 禁动**)

---

**最后更新**: 2026-08-05 (R20 阶段 5 估补)
**下一接管**: 1 owner × 1 周 (W1.D1 - W2.D5, 见 §4 时间表)
