# Phase 5F — Native Readiness Audit（2026-08-19）

> 只调查，不大规模实现。检查下一阶段（Phase 6）native 能力现状。

## 现状盘点

| 能力 | 状态 | 依据 |
|---|---|---|
| **主窗口** | ✅ Already available | `tauri.conf.json` app.windows[0] label=main |
| **快捷窗 (QuickWindow)** | ⚠️ Needs implementation | `setup` 已建 `quick` 窗 (440×390 transparent, visible=false)，但无快捷键呼出、无独立 UI 逻辑 |
| **托盘 (Tray)** | ✅ Already available | `TrayIconBuilder` main tray + menu (退出/打开主窗) + 左键显示 |
| **关闭隐藏到托盘** | ✅ Already available | `on_window_event` CloseRequested → hide + prevent_close |
| **全局快捷键 (global shortcut)** | ❌ Needs implementation | 无 `tauri-plugin-global-shortcut`，当前无 Alt+Space 呼出 |
| **自启动 (autostart)** | ✅ Already available | `tauri-plugin-autostart` 已装 + capabilities 已授权 |
| **通知 (notification)** | ✅ Already available | `tauri-plugin-notification` 已装 + capabilities 已授权 |
| **单实例 (single-instance)** | ❌ Needs implementation | 无 `tauri-plugin-single-instance`，多开会各起一个进程 |
| **窗口 show/hide 生命周期** | ⚠️ Partial | 主窗 hide-on-close 已有；quick 窗 show/hide 无触发点 |
| **OOBE (首次引导)** | ❌ Needs implementation | 无首次运行检测/引导流程 |

## 潜在 Blocker

| Blocker | 说明 |
|---|---|
| **无全局快捷键插件** | Phase 6 需要 `tauri-plugin-global-shortcut`，属新增依赖（低风险） |
| **无单实例** | 桌面伴随体应单实例；需 `tauri-plugin-single-instance` |
| **quick 窗透明无内容** | 当前 `index.html?window=quick` 加载主 App，无 quick 专用 UI |

## 推荐 Phase 6 顺序

1. **OOBE + 配置引导** — 首次运行检测 → 引导填 backend URL/key → 用户能配好连接
2. **QuickWindow** — quick 窗专用 UI (快捷输入) + Alt+Space 全局快捷键呼出
3. **single-instance** — 防止多开
4. **autostart 开关 UI** — 设置页加「开机自启」toggle（backend 能力已有）
5. **tray 增强** — 托盘菜单加「新建对话」「显示快捷窗」
6. **通知接入** — 后端主动送达 → 系统通知（backend companion daemon 已有事件）

## 本轮修复（仅 blocker 级）

- 无。当前无「明显 blocker」阻碍 Phase 6；以上均为增量实现项。
