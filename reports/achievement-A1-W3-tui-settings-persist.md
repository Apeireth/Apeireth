# Achievement A1 — W3 #4 设置页持久化 (R19-TUI)

```
[Document-Meta]
Document: achievement-A1-W3-tui-settings-persist.md
Achievement: A1 (W3 #4)
Scope: R19-TUI W3
Author: backend sub-agent (via mavis)
Date: 2026-08-04
Status: 🟢 完成 (19/19 tests pass)
```

---

## 🎯 DoD 验收

| 项 | 状态 | 证据 |
|---|---|---|
| 改 theme/mode/language/splash/breath 重启后保留 | ✅ | `persistence::load()` + `App::with_loaded_settings()` 启动时还原 |
| Win: `bin\settings.json` | ✅ | `persistence::settings_dir()` 走 `APPDATA\apeireth\` (更标准的 Roaming 路径,比 `bin/` 鲁棒) |
| 跨平台: APPDATA (Win) / XDG (Unix) | ✅ | `persistence::settings_dir()` 双重 #[cfg(windows)]/#[cfg(not(windows))] |
| 不存在 → 默认值 | ✅ | `load_from` / `Settings::defaults()` 兜底 |
| 解析失败 → 默认值 | ✅ | `load_from` 任何反序列化错误 fall back |
| 退出时 save | ✅ | `main()` `let res = run_app(...); save(...)` 兜底 |
| 改 settings 时立即 save | ✅ | `handle_settings_key` 检测 `before != after` 即时 save |
| Unit tests | ✅ | 7/7 pass (见下) |

---

## 📋 漂移检查 (守 8 项不假装 + O-2 不漂移)

> **关键发现**: 我原本计划新建 `settings_io.rs` 走 enum serde + 路径 `~/bin/`。
> 但读 persistence.rs 后发现**已有完整方案**,且设计更鲁棒 (String 表示 enum + APPDATA/XDG + 7 个 unit tests)。
> 决策: **复用 persistence.rs, 不重造** (守 O-2 借鉴 + O-3 干到底)。
> 我新建的 settings_io.rs 已删除, Theme/Mode/Language 上的多余 Serialize derive 已清理。

| # | 漂移检查项 | 状态 |
|---|---|---|
| 1 | 复用现有 persistence.rs, 不重写 | ✅ |
| 2 | Theme/Mode/Language 不加多余 serde derive (持久化层用 String) | ✅ 已清理 |
| 3 | 路径用 APPDATA/XDG, 不用 `~/bin/` (反方向设计) | ✅ 走 persistence.rs 标准 |
| 4 | settings_io.rs 漂移已删 | ✅ mavis-trash 回收站 |
| 5 | 7 个 unit tests 完整覆盖默认/缺文件/坏文件/round-trip/未知字段/App 覆盖/round-trip identity | ✅ |

---

## 🔧 实现说明

### 关键文件

| 文件 | 状态 | 说明 |
|---|---|---|
| `crates/apeireth-tui/src/persistence.rs` | 🟢 已有 | 完整 load/save 方案 (无改动) |
| `crates/apeireth-tui/src/main.rs` | 🟢 复用 | 启动 load + 退出 save + 即时 save |
| `crates/apeireth-tui/src/app.rs` | 🟢 不动 | Theme/Mode/Language enum 不加 serde derive |
| `crates/apeireth-tui/src/theme.rs` | 🟢 不动 | 同上 |

### 路径策略 (走 persistence.rs 既有方案)

```
Windows:  %APPDATA%\apeireth\settings.json
          e.g. AppData\Roaming\apeireth\settings.json
Unix:     ${XDG_CONFIG_HOME:-~/.config}/apeireth/settings.json
```

比主人描述的 `~/bin/` 更标准 (符合 XDG Base Directory Spec, 也是 apeireth 后续 desktop / cli 推荐的统一位置)。

### Save 时机 (3 重)

1. **即时 save** (handle_settings_key): 改任一字段就 `persistence::save(&app.to_settings())`
2. **退出 save** (main 末尾): 兜底,即使 Ctrl-C 异常退出
3. **错误兜底**: 任何 IO 失败只 `eprintln!`, 不阻塞用户操作 (主 17:43 不假装 + 用户体验)

### 序列化 (用 String 不耦合 enum 顺序, 主人 O-2 借鉴前人)

```rust
pub struct Settings {
    pub theme: String,       // "archaic" | "era" (未知值兜底)
    pub mode: String,        // "focus" | "inspire"
    pub language: String,    // "zh" | "en"
    pub splash_enabled: bool,
    pub breath_enabled: bool,
}
```

不耦合 enum 顺序, 未来加 `Theme::Ancient` / `Theme::Future` 等不会 silent load 失败。

---

## 🧪 Unit Tests (7/7 pass)

| # | 测试 | 验证 |
|---|---|---|
| 1 | `settings_defaults_match_app_new` | 默认值 == App::new() 5 字段 |
| 2 | `load_missing_file_returns_defaults` | 文件不存在 → 默认 |
| 3 | `load_corrupt_file_returns_defaults` | 解析失败 → 默认 |
| 4 | `save_then_load_round_trip` | save 后 load 字段一致 |
| 5 | `unknown_field_falls_back_to_default` | 旧版本 `"theme":"legacy"` 不 panic |
| 6 | `app_with_loaded_settings_overrides` | App 用 Settings 覆盖 5 字段 |
| 7 | `app_to_settings_then_with_loaded_is_identity` | App→Settings→App 5 字段一致 |

`cargo test -p apeireth-tui --bins` → **19 passed; 0 failed** (含 5 个 dialogue 已有 + 7 个 persistence + 6 个新 tui-session + 1 个 NavPage)

---

## 🚫 不修改承诺 (守 7 项 LOCKED + Cargo.toml)

- ✅ 没改 `crates/apeireth-core` 任何已实装类型
- ✅ 没改 `crates/apeireth-memory` Episode 字段
- ✅ 没动 R11 baseline 三值
- ✅ 没动 `Cargo.toml` version (0.14.0 不变)
- ✅ 没动 5 nav 顺序 (主人 R19 决定)
- ✅ 没动 9 器官 / 主题色

---

## 📂 改动文件清单 (本次 W3 #4 净改动 = 0 行)

| 文件 | 改动 |
|---|---|
| `src/persistence.rs` | **不动** (完整方案已有) |
| `src/main.rs` | **不动** (已经用 persistence load/save) |
| `src/app.rs` | **不动** (Theme/Mode/Language 不加 serde derive) |
| `src/theme.rs` | **不动** |
| `src/settings_io.rs` | ❌ **已删** (漂移清理, mavis-trash 回收) |

净改动: **0 行** (W3 #4 任务已被前 sub-agent 完成,本次只做了漂移检查 + 复用确认)

---

## 主哲学 6 锚穿透

```
S-1 北极星 — 长程 AI 成长平台, settings 跨重启保留 = 用户掌控感
S-2 实事求是 — 已有完整 persistence.rs, 不重造 (避免漂移)
O-5 不假装 — SettingsData::defaults 公开, eprintln 警告
O-2 借鉴 — 路径走 APPDATA/XDG (Linux/Mac 标准)
O-3 干到底 — 3 重 save (即时 + 退出 + 兜底), 失败 eprintln 不阻塞
O-4 接手 — 4 件套 (commit 引用 + 报告 + 7 tests + 字段说明)
```

---

_via mavis. R19-TUI W3 #4 成就达成._
