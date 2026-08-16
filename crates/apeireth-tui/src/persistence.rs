//! 设置页持久化 (W3.5, R19-TUI)
//!
//! **职责**: 把 settings 页的 5 个开关 (theme / mode / language / splash / breath) 持久化到
//! 磁盘, 启动时读, 退出 / 改动时写。
//!
//! **路径** (跨平台, 编译期 hardcode 段拼接):
//! - Windows: `%APPDATA%\apeireth\settings.json`
//!   (e.g. `AppData\Roaming\apeireth\settings.json`)
//! - Unix: `${XDG_CONFIG_HOME:-~/.config}/apeireth/settings.json`
//!
//! **设计原则 (主人 2026-08-04 R19, 守 5 项不假装)**:
//! - ❌ 不假装: 找不到文件 → 默认 (App::new); 解析失败 → 默认 (兜底, 不让用户卡住)
//! - ❌ 不漂移: 字段是字符串, 不耦合 enum 顺序 (Theme::Archaic=0/Era=1, 以后加新 variant 不会
//!   silent load 失败)
//! - ✅ 编译期 hardcode: path 段拼接, 段名 "apeireth" / "settings.json" 常量化
//!
//! **不持久化字段** (session 状态, 跟设置无关):
//! - `input_buf` / `input_cursor` / `chat_history` / `processing` / `thinking_expanded` /
//!   `chat_rx` / `should_quit` / `started_at` / `render_tick` / `spinner_frame`

use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::app::{App, Language, Mode};
use crate::theme::Theme;

/// 持久化结构 — 字符串表示, 不耦合 enum 顺序
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Settings {
    /// `"archaic"` | `"era"` (未知值兜底 `archaic`)
    pub theme: String,
    /// `"focus"` | `"inspire"` (未知值兜底 `focus`)
    pub mode: String,
    /// `"zh"` | `"en"` (未知值兜底 `zh`)
    pub language: String,
    pub splash_enabled: bool,
    pub breath_enabled: bool,
}

impl Settings {
    /// 跟 `App::new` 默认值同步 (W2 报告 §全量检查, 5 字段默认)
    pub fn defaults() -> Self {
        Self {
            theme: "archaic".to_string(),
            mode: "focus".to_string(),
            language: "zh".to_string(),
            splash_enabled: true,
            breath_enabled: true,
        }
    }
}

/// 跨平台 config 目录:
/// - Windows: `%APPDATA%\apeireth`
/// - Unix: `${XDG_CONFIG_HOME:-~/.config}/apeireth`
pub fn settings_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            if !appdata.is_empty() {
                return Some(PathBuf::from(appdata).join(DIR_NAME));
            }
        }
    }
    #[cfg(not(windows))]
    {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            if !xdg.is_empty() {
                return Some(PathBuf::from(xdg).join(DIR_NAME));
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            if !home.is_empty() {
                return Some(PathBuf::from(home).join(".config").join(DIR_NAME));
            }
        }
    }
    None
}

/// 完整 settings.json 路径
pub fn settings_path() -> Option<PathBuf> {
    settings_dir().map(|d| d.join(FILE_NAME))
}

/// 读 settings.json (兜底: 找不到 / 解析失败 / 任何 IO 错误都返默认, 不 panic)
pub fn load() -> Settings {
    match settings_path() {
        Some(p) => load_from(&p),
        None => Settings::defaults(),
    }
}

/// 写 settings.json (确保父目录存在, 不存在则创建)
pub fn save(s: &Settings) -> io::Result<()> {
    let path = settings_path().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "no config dir available (APPDATA / XDG_CONFIG_HOME / HOME not set)",
        )
    })?;
    save_to(&path, s)
}

/// 从指定路径读 settings.json (内部用, 公共 API `load()` 走 `settings_path()`)
pub fn load_from(path: &std::path::Path) -> Settings {
    let Ok(content) = fs::read_to_string(path) else {
        return Settings::defaults();
    };
    // 兜底: 任何反序列化失败都返默认, 不让用户卡住
    let Ok(mut s) = serde_json::from_str::<Settings>(&content) else {
        return Settings::defaults();
    };
    // normalize: 未知 enum 字符串 → 默认 (例如老版本写过 "legacy" 这类以后删掉的字段)
    if s.theme != "archaic" && s.theme != "era" {
        s.theme = "archaic".to_string();
    }
    if s.mode != "focus" && s.mode != "inspire" {
        s.mode = "focus".to_string();
    }
    if s.language != "zh" && s.language != "en" {
        s.language = "zh".to_string();
    }
    s
}

/// 写 settings.json 到指定路径 (内部用, 公共 API `save()` 走 `settings_path()`)
pub fn save_to(path: &std::path::Path, s: &Settings) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(s)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(path, json)
}

// ---- 编译期 hardcode 段常量 ----

/// 目录名 (跨平台统一: `apeireth`)
const DIR_NAME: &str = "apeireth";
/// settings 文件名
const FILE_NAME: &str = "settings.json";

// ============================================================
// App ↔ Settings 转换
// (放在 persistence.rs 而非 app.rs: 因为 enum → 字符串 是持久化层职责,
// 跟 app.rs 业务状态机解耦, 同样理由: 改 Theme/Mode/Language 字段时只在
// persistence.rs 改, 不污染 app.rs)
// ============================================================

fn theme_to_str(t: Theme) -> &'static str {
    match t {
        Theme::Archaic => "archaic",
        Theme::Era => "era",
    }
}

fn theme_from_str(s: &str) -> Theme {
    match s {
        "era" => Theme::Era,
        _ => Theme::Archaic, // 默认 / 未知值兜底 (不 panic)
    }
}

fn mode_from_str(s: &str) -> Mode {
    match s {
        "inspire" => Mode::Inspire,
        _ => Mode::Focus,
    }
}

fn lang_from_str(s: &str) -> Language {
    match s {
        "en" => Language::En,
        _ => Language::Zh,
    }
}

impl App {
    /// 从持久化 settings 构造 App (不持久化字段保持 `Self::new()` 默认)
    pub fn with_loaded_settings(s: Settings) -> Self {
        Self {
            theme: theme_from_str(&s.theme),
            mode: mode_from_str(&s.mode),
            language: lang_from_str(&s.language),
            settings_cursor: 1, // R26-2: 不持久化, 启动归 1 (theme)
            splash_enabled: s.splash_enabled,
            breath_enabled: s.breath_enabled,
            // R26-2: splash_active 跟随设置 (启动时根据 splash_enabled 决定是否显示)
            splash_active: s.splash_enabled,
            // R26-2: breath_phase 运行时字段, 启动归 0 (主循环 tick 推进)
            breath_phase: 0.0,
            ..Self::new()
        }
    }

    /// 把 App 转换成可持久化的 Settings (只取 5 字段)
    pub fn to_settings(&self) -> Settings {
        Settings {
            theme: theme_to_str(self.theme).to_string(),
            mode: self.mode.label().to_string(),
            language: self.language.label().to_string(),
            splash_enabled: self.splash_enabled,
            breath_enabled: self.breath_enabled,
        }
    }
}

// ============================================================
// 单元测试 (7 个: 默认 / 缺文件 / 坏文件 / round-trip / 未知字段兜底 /
// App 覆盖 / App round-trip)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::NavPage;

    /// helper: 创一个 unique temp dir, 把 settings.json path 交给 f
    /// (不走 env var, 避免 cargo test 并发跑时 env 互相覆盖)
    fn with_temp_settings_path<F: FnOnce(&PathBuf)>(f: F) {
        let unique = format!(
            "apeireth-tui-test-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );
        let tmp = std::env::temp_dir().join(&unique);
        let _ = fs::remove_dir_all(&tmp); // 清理上次残留
        fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("apeireth").join("settings.json");
        f(&path);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn settings_defaults_match_app_new() {
        let d = Settings::defaults();
        assert_eq!(d.theme, "archaic");
        assert_eq!(d.mode, "focus");
        assert_eq!(d.language, "zh");
        assert_eq!(d.splash_enabled, true);
        assert_eq!(d.breath_enabled, true);
    }

    #[test]
    fn load_missing_file_returns_defaults() {
        with_temp_settings_path(|p| {
            let s = load_from(p);
            assert_eq!(s, Settings::defaults());
        });
    }

    #[test]
    fn load_corrupt_file_returns_defaults() {
        with_temp_settings_path(|p| {
            if let Some(parent) = p.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(p, "this is not json {{{").unwrap();
            let s = load_from(p);
            assert_eq!(s, Settings::defaults());
        });
    }

    #[test]
    fn save_then_load_round_trip() {
        with_temp_settings_path(|p| {
            let s = Settings {
                theme: "era".to_string(),
                mode: "inspire".to_string(),
                language: "en".to_string(),
                splash_enabled: false,
                breath_enabled: false,
            };
            save_to(p, &s).unwrap();
            let loaded = load_from(p);
            assert_eq!(loaded, s);
        });
    }

    #[test]
    fn unknown_field_falls_back_to_default() {
        // 老的 settings.json 可能含未识别的 enum 字符串 ("legacy"), 不 panic
        with_temp_settings_path(|p| {
            if let Some(parent) = p.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(
                p,
                r#"{"theme":"legacy","mode":"focus","language":"zh","splash_enabled":true,"breath_enabled":true}"#,
            )
            .unwrap();
            let s = load_from(p);
            // theme "legacy" 不识别 → 回退到默认 archaic
            assert_eq!(s.theme, "archaic");
            assert_eq!(s.mode, "focus");
            assert_eq!(s.language, "zh");
        });
    }

    #[test]
    fn app_with_loaded_settings_overrides() {
        let s = Settings {
            theme: "era".to_string(),
            mode: "inspire".to_string(),
            language: "en".to_string(),
            splash_enabled: false,
            breath_enabled: false,
        };
        let app = App::with_loaded_settings(s);
        assert_eq!(app.theme, Theme::Era);
        assert_eq!(app.mode, Mode::Inspire);
        assert_eq!(app.language, Language::En);
        assert_eq!(app.splash_enabled, false);
        assert_eq!(app.breath_enabled, false);
        // 不持久化字段保持默认
        assert_eq!(app.nav, NavPage::Bridge);
        assert!(app.input_buf.is_empty());
        assert!(app.chat_history.is_empty());
        assert!(!app.processing);
        assert!(!app.should_quit);
    }

    #[test]
    fn app_to_settings_then_with_loaded_is_identity() {
        let s_in = Settings {
            theme: "era".to_string(),
            mode: "inspire".to_string(),
            language: "en".to_string(),
            splash_enabled: false,
            breath_enabled: true,
        };
        let app1 = App::with_loaded_settings(s_in.clone());
        let s_out = app1.to_settings();
        let app2 = App::with_loaded_settings(s_out.clone());
        // 5 字段完全一致
        assert_eq!(app1.theme, app2.theme);
        assert_eq!(app1.mode, app2.mode);
        assert_eq!(app1.language, app2.language);
        assert_eq!(app1.splash_enabled, app2.splash_enabled);
        assert_eq!(app1.breath_enabled, app2.breath_enabled);
        // round-trip 设置也一致
        assert_eq!(s_in, s_out);
    }
}

// ============================================================
// 输入历史持久化 (R26-3-fixes)
// ============================================================
//
// **职责**: 持久化用户在 Dialogue 页提交过的输入到 `apeireth-tui-input-history.txt`,
// 跨 session 还能用 (PowerShell PSReadLine / Codex CLI 风格).
//
// **路径**:
// - Windows: `%APPDATA%\apeireth\apeireth-tui-input-history.txt`
// - Unix: `${XDG_CONFIG_HOME:-~/.config}/apeireth/apeireth-tui-input-history.txt`
//
// **格式**: 每行 1 条输入, 最旧在上, 最新在下. 单条最大 4096 字节 (防止恶意大输入撑爆).
// 上限 100 条 (FIFO).
//
// **不假装**:
// - 读失败 (文件不存在 / IO 错误) → 返空 Vec, 不 panic
// - 写失败 → 不 panic (eprintln 警告, 不让用户卡住)
// - 行解析失败 → 跳过该行 (兜底, 历史里可能有奇怪数据)

const INPUT_HISTORY_FILE: &str = "apeireth-tui-input-history.txt";
const INPUT_HISTORY_MAX: usize = 100;
const INPUT_HISTORY_LINE_MAX: usize = 4096;

pub fn input_history_path() -> Option<PathBuf> {
    settings_dir().map(|d| d.join(INPUT_HISTORY_FILE))
}

/// 读输入历史 (兜底: 任何 IO 错误返空)
pub fn load_input_history() -> Vec<String> {
    let Some(path) = input_history_path() else {
        return Vec::new();
    };
    let Ok(content) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    content
        .lines()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty() && s.len() <= INPUT_HISTORY_LINE_MAX)
        .collect()
}

/// 写输入历史 (覆盖整个文件). 错误不 panic, 仅 eprintln.
pub fn save_input_history(history: &[String]) {
    let Some(path) = input_history_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut content = String::new();
    for line in history.iter().take(INPUT_HISTORY_MAX) {
        // 单行最大 INPUT_HISTORY_LINE_MAX 字节 (防止大输入)
        let truncated: String = line.chars().take(INPUT_HISTORY_LINE_MAX).collect();
        content.push_str(&truncated);
        content.push('\n');
    }
    if let Err(e) = fs::write(&path, content) {
        eprintln!("[apeireth-tui] warn: save input history: {e}");
    }
}

/// Append 1 条到 history (FIFO 上限 INPUT_HISTORY_MAX). 返回新 history.
pub fn push_input_history(mut history: Vec<String>, input: String) -> Vec<String> {
    if input.trim().is_empty() {
        return history;
    }
    // 去重: 连续重复不存
    if history.last().map(|s| s.as_str()) == Some(input.as_str()) {
        return history;
    }
    history.push(input);
    // FIFO: 超出 100 条扔掉最旧的
    let len = history.len();
    if len > INPUT_HISTORY_MAX {
        history.drain(0..(len - INPUT_HISTORY_MAX));
    }
    history
}

#[cfg(test)]
mod history_tests {
    use super::*;

    #[test]
    fn push_input_history_appends_unique() {
        let h0 = Vec::new();
        let h1 = push_input_history(h0, "hello".to_string());
        let h2 = push_input_history(h1, "world".to_string());
        assert_eq!(h2, vec!["hello", "world"]);
    }

    #[test]
    fn push_input_history_skips_empty() {
        let h = vec!["a".to_string()];
        let h = push_input_history(h, "  ".to_string());
        assert_eq!(h, vec!["a"]);
    }

    #[test]
    fn push_input_history_skips_consecutive_dup() {
        let h = vec!["hello".to_string()];
        let h = push_input_history(h, "hello".to_string());
        assert_eq!(h, vec!["hello"]);
    }

    #[test]
    fn push_input_history_dedupes_non_consecutive() {
        let h = vec!["a".to_string(), "b".to_string()];
        let h = push_input_history(h, "a".to_string());
        assert_eq!(h, vec!["a", "b", "a"]);
    }

    #[test]
    fn push_input_history_fifo_cap_at_100() {
        let mut h = Vec::new();
        for i in 0..150 {
            h = push_input_history(h, format!("msg-{}", i));
        }
        assert_eq!(h.len(), INPUT_HISTORY_MAX);
        assert_eq!(h.first().unwrap(), "msg-50");
        assert_eq!(h.last().unwrap(), "msg-149");
    }
}
