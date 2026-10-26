//! 设置页三页 · `theme` 子面的 **wasm 宿主挂载**模块。
//!
//! ## 职责
//!
//! 1. **装载固定数据**（`harness/settings/fixtures/theme.json` ⇒ **编译期内嵌**
//!    ⇒ 引擎侧与参考侧**同一份文件** ⇒ 两侧喂的数据逐字相同）；
//! 2. **组装 props**（宿主注入 ⇒ 组件保持纯函数）；
//! 3. **挂载**（`WebDom` + `Renderer` + `Runtime::effect` + 微任务调度），
//!    并把「内联样式」过一遍浏览器的 CSSOM（**归一化**，见下）。
//!
//! ## 诚实边界
//!
//! | 事项 | 口径 |
//! |---|---|
//! | 主题 / 配色的取值**写 `<html>`** | **不迁**（宿主侧的事）⇒ 引擎侧只发「选中」的意图 |
//! | 文件选择 / 取文件 | **不迁**（宿主侧）⇒ 引擎侧只发「请求上传 / 文件已选」的意图 |
//! | 二次确认弹层 | **共享覆盖层组件**（不属任一面）⇒ 引擎侧只发「请求二次确认」的意图 |
//! | 危险动作的**执行**（删数据） | **不迁**（宿主侧执行）⇒ 引擎侧只发意图 |
//! | 数据面（HTTP / SSE 契约） | **不碰**（本面喂的是固定数据） |
//!
//! ## 内联样式的**归一化**（设计文档 §9.2 C3 的落地口径）
//!
//! 生产用样式指令（`style:background` / `style:--ap-accent-ui`）⇒ 浏览器把取值
//! **序列化**成自己的口径（默认角度省略 / 颜色转 `rgb()` / `url()` 加引号 /
//! 位置补全 …）⇒ 参考侧 DOM 里的 `style` 属性是**序列化后的串**。
//!
//! 引擎侧把**原文**写进属性 ⇒ 属性串会与参考侧**逐字不等**（值一样、串不同）。
//! ⇒ 这里在挂载后把同一段取值**再交给浏览器的 CSSOM 走一遍**
//! （`style.cssText = 原文`）⇒ **由浏览器自己序列化** ⇒ 两侧同口径、逐字相等。
//!
//! ⚠️ 归一化**必须**在每次 DOM 补丁之后重跑（补丁会把属性写回原文）⇒ 归一化
//! 与渲染在**同一条链**上（见 [`wasm::mount`]）。

use apeireth_ui_core::{Callback, ReactiveScope, Signal};
use apeireth_ui_kit::settings_theme::{AccentEntry, ThemeEntry, ThemeSettingsPanelProps};

/// 固定数据的原文（**编译期内嵌** ⇒ 与参考侧同一份文件）。
pub const FIXTURE_JSON: &str = include_str!("../harness/settings/fixtures/theme.json");

// ── 极小 JSON 解析器（**零外部依赖**） ─────────────────────────────────────

/// 解析出来的 JSON 值（本面只需要对象 / 数组 / 字符串 / 布尔 / 数值）。
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    /// 空值。
    Null,
    /// 布尔。
    Bool(bool),
    /// 数值。
    Number(f64),
    /// 字符串。
    String(String),
    /// 数组。
    Array(Vec<Json>),
    /// 对象（**保序**）。
    Object(Vec<(String, Json)>),
}

impl Json {
    /// 取对象的某个键。
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(entries) => entries
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    /// 取字符串。
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::String(text) => Some(text),
            _ => None,
        }
    }

    /// 取布尔。
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(flag) => Some(*flag),
            _ => None,
        }
    }

    /// 取数组。
    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Array(items) => Some(items),
            _ => None,
        }
    }
}

/// JSON 解析器（递归下降；**报错带位置** ⇒ 固定数据坏了要**显式失败**）。
struct JsonParser<'a> {
    chars: &'a [char],
    pos: usize,
}

/// 解析一段 JSON 文本。
pub fn parse_json(text: &str) -> Result<Json, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut parser = JsonParser {
        chars: &chars,
        pos: 0,
    };
    parser.skip_ws();
    let value = parser.parse_value()?;
    parser.skip_ws();
    if parser.pos != parser.chars.len() {
        return Err(format!(
            "固定数据解析失败：位置 {} 之后有多余内容",
            parser.pos
        ));
    }
    Ok(value)
}

impl JsonParser<'_> {
    /// 跳过空白。
    fn skip_ws(&mut self) {
        while matches!(self.chars.get(self.pos), Some(ch) if ch.is_whitespace()) {
            self.pos += 1;
        }
    }

    /// 取当前字符并前进。
    fn bump(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied();
        if ch.is_some() {
            self.pos += 1;
        }
        ch
    }

    /// 期望一个字面量（`true` / `false` / `null`）。
    fn expect_word(&mut self, word: &str) -> Result<(), String> {
        for expected in word.chars() {
            match self.bump() {
                Some(ch) if ch == expected => {}
                Some(ch) => {
                    return Err(format!(
                        "固定数据解析失败：位置 {} 期望 `{word}`，实际 `{ch}`",
                        self.pos - 1
                    ))
                }
                None => {
                    return Err(format!(
                        "固定数据解析失败：位置 {} 期望 `{word}`，实际到文件尾",
                        self.pos
                    ))
                }
            }
        }
        Ok(())
    }

    /// 解析一个值。
    fn parse_value(&mut self) -> Result<Json, String> {
        self.skip_ws();
        match self.chars.get(self.pos).copied() {
            Some('{') => self.parse_object(),
            Some('[') => self.parse_array(),
            Some('"') => Ok(Json::String(self.parse_string()?)),
            Some('t') => {
                self.expect_word("true")?;
                Ok(Json::Bool(true))
            }
            Some('f') => {
                self.expect_word("false")?;
                Ok(Json::Bool(false))
            }
            Some('n') => {
                self.expect_word("null")?;
                Ok(Json::Null)
            }
            Some(ch) if ch == '-' || ch.is_ascii_digit() => self.parse_number(),
            Some(ch) => Err(format!(
                "固定数据解析失败：位置 {} 出现无法识别的字符 `{ch}`",
                self.pos
            )),
            None => Err("固定数据解析失败：值缺失".to_owned()),
        }
    }

    /// 解析对象。
    fn parse_object(&mut self) -> Result<Json, String> {
        self.pos += 1; // `{`
        let mut entries: Vec<(String, Json)> = Vec::new();
        self.skip_ws();
        if self.chars.get(self.pos) == Some(&'}') {
            self.pos += 1;
            return Ok(Json::Object(entries));
        }
        loop {
            self.skip_ws();
            if self.chars.get(self.pos) != Some(&'"') {
                return Err(format!(
                    "固定数据解析失败：位置 {} 期望对象的键名",
                    self.pos
                ));
            }
            let key = self.parse_string()?;
            self.skip_ws();
            if self.chars.get(self.pos) != Some(&':') {
                return Err(format!("固定数据解析失败：位置 {} 期望 `:`", self.pos));
            }
            self.pos += 1;
            let value = self.parse_value()?;
            entries.push((key, value));
            self.skip_ws();
            match self.chars.get(self.pos).copied() {
                Some(',') => {
                    self.pos += 1;
                }
                Some('}') => {
                    self.pos += 1;
                    return Ok(Json::Object(entries));
                }
                Some(ch) => {
                    return Err(format!(
                        "固定数据解析失败：位置 {} 出现无法识别的字符 `{ch}`",
                        self.pos
                    ))
                }
                None => return Err("固定数据解析失败：对象未闭合".to_owned()),
            }
        }
    }

    /// 解析数组。
    fn parse_array(&mut self) -> Result<Json, String> {
        self.pos += 1; // `[`
        let mut items: Vec<Json> = Vec::new();
        self.skip_ws();
        if self.chars.get(self.pos) == Some(&']') {
            self.pos += 1;
            return Ok(Json::Array(items));
        }
        loop {
            let value = self.parse_value()?;
            items.push(value);
            self.skip_ws();
            match self.chars.get(self.pos).copied() {
                Some(',') => {
                    self.pos += 1;
                }
                Some(']') => {
                    self.pos += 1;
                    return Ok(Json::Array(items));
                }
                Some(ch) => {
                    return Err(format!(
                        "固定数据解析失败：位置 {} 出现无法识别的字符 `{ch}`",
                        self.pos
                    ))
                }
                None => return Err("固定数据解析失败：数组未闭合".to_owned()),
            }
        }
    }

    /// 解析字符串（含转义）。
    fn parse_string(&mut self) -> Result<String, String> {
        self.pos += 1; // 开引号
        let mut out = String::new();
        loop {
            match self.bump() {
                Some('"') => return Ok(out),
                Some('\\') => {
                    let escaped = self.bump().ok_or("固定数据解析失败：转义未闭合")?;
                    match escaped {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        '/' => out.push('/'),
                        'b' => out.push('\u{0008}'),
                        'f' => out.push('\u{000c}'),
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        'u' => {
                            let code = self.parse_hex4()?;
                            // 代理对：高位 + 低位 ⇒ 合成一个码点。
                            if (0xD800..0xDC00).contains(&code) {
                                if self.chars.get(self.pos) == Some(&'\\')
                                    && self.chars.get(self.pos + 1) == Some(&'u')
                                {
                                    self.pos += 2;
                                    let low = self.parse_hex4()?;
                                    if (0xDC00..0xE000).contains(&low) {
                                        let combined =
                                            0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                                        out.push(char::from_u32(combined).unwrap_or('\u{FFFD}'));
                                        continue;
                                    }
                                }
                                out.push('\u{FFFD}');
                                continue;
                            }
                            out.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                        }
                        other => {
                            return Err(format!("固定数据解析失败：无法识别的转义 `\\{other}`"))
                        }
                    }
                }
                Some(ch) => out.push(ch),
                None => return Err("固定数据解析失败：字符串未闭合".to_owned()),
            }
        }
    }

    /// 解析四位十六进制（`\uXXXX`）。
    fn parse_hex4(&mut self) -> Result<u32, String> {
        let mut code = 0u32;
        for _ in 0..4 {
            let ch = self.bump().ok_or("固定数据解析失败：\\u 转义不完整")?;
            let digit = ch
                .to_digit(16)
                .ok_or_else(|| format!("固定数据解析失败：`\\u` 后出现非法字符 `{ch}`"))?;
            code = code * 16 + digit;
        }
        Ok(code)
    }

    /// 解析数值。
    fn parse_number(&mut self) -> Result<Json, String> {
        let start = self.pos;
        if self.chars.get(self.pos) == Some(&'-') {
            self.pos += 1;
        }
        while matches!(self.chars.get(self.pos), Some(ch) if ch.is_ascii_digit()) {
            self.pos += 1;
        }
        if self.chars.get(self.pos) == Some(&'.') {
            self.pos += 1;
            while matches!(self.chars.get(self.pos), Some(ch) if ch.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        if matches!(self.chars.get(self.pos), Some('e') | Some('E')) {
            self.pos += 1;
            if matches!(self.chars.get(self.pos), Some('+') | Some('-')) {
                self.pos += 1;
            }
            while matches!(self.chars.get(self.pos), Some(ch) if ch.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        text.parse::<f64>()
            .map(Json::Number)
            .map_err(|error| format!("固定数据解析失败：数值 `{text}` 非法（{error}）"))
    }
}

// ── 固定数据（**两侧同一份**） ─────────────────────────────────────────────

/// 主题条目（固定数据的一行）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureTheme {
    /// 条目 id。
    pub id: String,
    /// 名称。
    pub label: String,
    /// 说明。
    pub desc: String,
    /// 样品（**内联渐变原文** ⇒ 逐字取自固定数据）。
    pub swatch: String,
}

/// 配色条目（固定数据的一行）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureAccent {
    /// 条目 id。
    pub id: String,
    /// 名称。
    pub label: String,
    /// 说明。
    pub desc: String,
    /// 强调色。
    pub accent: String,
    /// 强调色的前景色。
    pub ink: String,
}

/// 主题子面的固定数据。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ThemeFixture {
    /// 主题目录（**顺序即渲染顺序**）。
    pub themes: Vec<FixtureTheme>,
    /// 配色目录（**顺序即渲染顺序**）。
    pub accents: Vec<FixtureAccent>,
    /// 当前主题 id。
    pub active_theme: String,
    /// 当前配色 id。
    pub active_accent: String,
    /// 是否已启用自定义背景。
    pub custom_bg: bool,
    /// 自定义背景的预览地址（**固定值** ⇒ 两侧同一张图）。
    pub custom_preview_url: String,
    /// 背景操作进行中。
    pub bg_busy: bool,
    /// 背景操作的错误文案。
    pub bg_error: String,
}

/// 读固定数据（**编译期内嵌的那一份** ⇒ 与参考侧同一份文件）。
///
/// 固定数据坏了 ⇒ **显式报错**（不静默给空目录）。
pub fn theme_fixture() -> Result<ThemeFixture, String> {
    let root = parse_json(FIXTURE_JSON)?;
    let themes = root
        .get("themeCatalog")
        .and_then(Json::as_array)
        .ok_or("固定数据缺少 `themeCatalog` 数组")?
        .iter()
        .map(|item| {
            Ok(FixtureTheme {
                id: string_field(item, "id")?,
                label: string_field(item, "label")?,
                desc: string_field(item, "desc")?,
                swatch: string_field(item, "swatch")?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let accents = root
        .get("accentCatalog")
        .and_then(Json::as_array)
        .ok_or("固定数据缺少 `accentCatalog` 数组")?
        .iter()
        .map(|item| {
            Ok(FixtureAccent {
                id: string_field(item, "id")?,
                label: string_field(item, "label")?,
                desc: string_field(item, "desc")?,
                accent: string_field(item, "accent")?,
                ink: string_field(item, "ink")?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let state = root.get("state").ok_or("固定数据缺少 `state` 对象")?;
    Ok(ThemeFixture {
        themes,
        accents,
        active_theme: string_field(state, "activeTheme")?,
        active_accent: string_field(state, "activeAccent")?,
        custom_bg: bool_field(state, "customBg")?,
        custom_preview_url: string_field(state, "customPreviewUrl")?,
        bg_busy: bool_field(state, "bgBusy")?,
        bg_error: string_field(state, "bgError")?,
    })
}

/// 取一个字符串字段。
fn string_field(value: &Json, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Json::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("固定数据缺字段 `{key}`（或类型不对）"))
}

/// 取一个布尔字段。
fn bool_field(value: &Json, key: &str) -> Result<bool, String> {
    value
        .get(key)
        .and_then(Json::as_bool)
        .ok_or_else(|| format!("固定数据缺字段 `{key}`（或类型不对）"))
}

// ── 宿主状态（**信号由调用方持有** ⇒ 组件是纯函数） ────────────────────────

/// 主题子面的宿主状态（响应式部分）。
#[derive(Clone)]
pub struct ThemeHostState {
    /// 响应式作用域（**使用者形态** ⇒ 本状态**挂不了 effect**）。
    scope: ReactiveScope,
    /// 当前主题 id（**选中态的意图出口**：宿主写 ⇒ 组件重渲染）。
    active_theme: Signal<String>,
    /// 当前配色 id（同上）。
    active_accent: Signal<String>,
    /// 其它意图的最近一次（`"request-upload"` / `"use-theme-default"` /
    /// `"request-clear"` / `"file-changed"`；空串 ⇒ 还没有）。
    intent: Signal<String>,
}

impl ThemeHostState {
    /// 建一份状态（读固定数据 ⇒ 数据坏了**显式报错**）。
    pub fn new(fixture: ThemeFixture, scope: ReactiveScope) -> Self {
        Self {
            active_theme: scope.signal(fixture.active_theme.clone()),
            active_accent: scope.signal(fixture.active_accent.clone()),
            intent: scope.signal(String::new()),
            scope,
        }
    }

    /// 响应式作用域（**使用者形态**；供宿主复核「同一个运行时」）。
    pub fn scope(&self) -> &ReactiveScope {
        &self.scope
    }

    /// 当前主题 id。
    pub fn active_theme(&self) -> Signal<String> {
        self.active_theme.clone()
    }

    /// 当前配色 id。
    pub fn active_accent(&self) -> Signal<String> {
        self.active_accent.clone()
    }

    /// 其它意图的最近一次。
    pub fn intent(&self) -> Signal<String> {
        self.intent.clone()
    }
}

/// 组装组件 props（**宿主注入**：数据 → 组件）。
pub fn theme_panel_props(
    fixture: &ThemeFixture,
    state: &ThemeHostState,
) -> ThemeSettingsPanelProps {
    let active_theme = state.active_theme.get();
    let active_accent = state.active_accent.get();

    let themes: Vec<ThemeEntry> = fixture
        .themes
        .iter()
        .map(|item| {
            let id = item.id.clone();
            let sink = state.active_theme.clone();
            ThemeEntry::new(
                item.id.clone(),
                item.label.clone(),
                item.desc.clone(),
                item.swatch.clone(),
            )
            .on_pick(Callback::new(move || sink.set(id.clone())))
        })
        .collect();

    let accents: Vec<AccentEntry> = fixture
        .accents
        .iter()
        .map(|item| {
            let id = item.id.clone();
            let sink = state.active_accent.clone();
            AccentEntry::new(
                item.id.clone(),
                item.label.clone(),
                item.desc.clone(),
                item.accent.clone(),
                item.ink.clone(),
            )
            .on_pick(Callback::new(move || sink.set(id.clone())))
        })
        .collect();

    let upload_sink = state.intent.clone();
    let default_sink = state.intent.clone();
    let clear_sink = state.intent.clone();
    let file_sink = state.intent.clone();

    ThemeSettingsPanelProps {
        themes,
        accents,
        active_theme,
        active_accent,
        custom_bg: fixture.custom_bg,
        custom_preview_url: if fixture.custom_preview_url.is_empty() {
            None
        } else {
            Some(fixture.custom_preview_url.clone())
        },
        bg_busy: fixture.bg_busy,
        bg_error: fixture.bg_error.clone(),
        on_request_upload: Some(Callback::new(move || {
            upload_sink.set(String::from("request-upload"))
        })),
        on_use_theme_default: Some(Callback::new(move || {
            default_sink.set(String::from("use-theme-default"))
        })),
        on_request_clear: Some(Callback::new(move || {
            clear_sink.set(String::from("request-clear"))
        })),
        on_file_changed: Some(Callback::new(move || {
            file_sink.set(String::from("file-changed"))
        })),
    }
}

// ── wasm 装配（**native 编译不参与** ⇒ 行为由浏览器内对照钉住） ─────────────

/// wasm32 侧的装配。
#[cfg(target_arch = "wasm32")]
pub mod wasm {
    //! ## 诚实边界
    //!
    //! - 本模块整体被 `#[cfg(target_arch = "wasm32")]` 隔离 ⇒ **native 编译不参与**
    //!   ⇒ 它的行为**未经 native 侧验证**（浏览器内的验证由对照 harness 承担）。
    //! - **真后端未接线**：数据面是**固定数据**（编译期内嵌的那一份）。
    //! - **主题写 `<html>` 不迁**：点主题卡只改**宿主侧的状态**（`data-theme` 由
    //!   宿主写）⇒ 引擎侧**不碰** `<html>`。

    use std::cell::RefCell;
    use std::rc::Rc;

    use apeireth_ui_core::{HostScheduler, Runtime, UiComponent};
    use apeireth_ui_dom::{Renderer, VNode, WebDom};
    use apeireth_ui_kit::settings_theme::ThemeSettingsPanel;
    use apeireth_ui_macro::view;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::Element;

    use crate::settings_theme::{theme_fixture, theme_panel_props, ThemeFixture, ThemeHostState};

    /// 微任务调度器：把 flush 排进浏览器的微任务队列。
    ///
    /// ⚠️ **不用** `#[wasm_bindgen] extern "C"` 声明 `queueMicrotask`：本模块
    /// 在 **lib 与 bin 里各编译一份**（共享的 `main.rs` 与 `lib.rs` 都声明了
    /// 它）⇒ wasm-bindgen 的导出/描述符号会**重复** ⇒ 链接失败。
    /// ⇒ 这里用 `js_sys::Reflect` 直接调全局函数（**零导出** ⇒ 零冲突）。
    struct MicrotaskScheduler;

    impl HostScheduler for MicrotaskScheduler {
        fn schedule_flush(&self, flush: Box<dyn FnOnce()>) {
            let queue =
                js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("queueMicrotask"))
                    .ok()
                    .and_then(|value| value.dyn_into::<js_sys::Function>().ok());
            match queue {
                Some(queue) => {
                    let callback = Closure::once_into_js(move || {
                        flush();
                        JsValue::UNDEFINED
                    });
                    let _ = queue.call1(&JsValue::UNDEFINED, &callback);
                }
                None => {
                    // 没有微任务队列 ⇒ **同步**跑（不静默丢刷新）。
                    flush();
                }
            }
        }
    }

    /// wasm 侧的单例应用。
    struct ThemeApp {
        /// 宿主状态。
        state: ThemeHostState,
    }

    thread_local! {
        /// wasm 侧的单例应用（主题子面形态）。
        static THEME_APP: RefCell<Option<ThemeApp>> = const { RefCell::new(None) };
    }

    /// 渲染整棵视图树。
    ///
    /// ## 舞台与作用域锚点
    ///
    /// - `.settings-stage`：**对照专用样式表**里的 828px 舞台（参考侧靠生产
    ///   自己的布局得到同样的宽度 ⇒ 两侧裁剪框宽度相等）。
    /// - `.sp-theme`：**子面 scoped CSS 的作用域前缀**（抽取器按它前缀化）。
    fn render_tree(state: &ThemeHostState, fixture: &ThemeFixture) -> VNode {
        let props = theme_panel_props(fixture, state);
        let panel = ThemeSettingsPanel::render(&props);
        view! {
            div(class = "settings-stage") {
                div(class = "sp-theme") {
                    { panel }
                }
            }
        }
    }

    /// 挂载主题子面。
    ///
    /// 失败 ⇒ **既返回错误、又发布到页面**（`window.__SETTINGS_ERROR__`）⇒
    /// 调用方忽略返回值时**仍然看得见**（不静默）。
    pub fn mount(container: &Element) -> Result<(), String> {
        match mount_inner(container) {
            Ok(()) => Ok(()),
            Err(error) => {
                report_error(&error);
                Err(error)
            }
        }
    }

    /// 挂载主题子面（内部实现）。
    fn mount_inner(container: &Element) -> Result<(), String> {
        let dom = WebDom::new().map_err(|error| error.to_string())?;
        let root: web_sys::Node = container.clone().into();
        let mut renderer = Renderer::new(dom, root);

        let fixture = theme_fixture()?;
        // 应用根（**runtime 的所有者**）：建运行时 ⇒ ⇒ 交给状态持有者一个作用域。
        let runtime = Runtime::new();
        runtime.set_host_scheduler(Rc::new(MicrotaskScheduler));
        let state = ThemeHostState::new(fixture.clone(), runtime.reactive_scope());

        renderer
            .mount(render_tree(&state, &fixture))
            .map_err(|error| error.to_string())?;
        normalize_inline_styles(container)?;

        let renderer = Rc::new(RefCell::new(renderer));
        let effect_renderer = renderer.clone();
        let effect_state = state.clone();
        let effect_fixture = fixture.clone();
        let effect_container = container.clone();
        runtime.effect(move || {
            let tree = render_tree(&effect_state, &effect_fixture);
            let mut guard = effect_renderer.borrow_mut();
            let _ = guard.patch(tree);
            // 补丁会把属性写回**原文** ⇒ 归一化必须**跟着补丁**跑。
            let _ = normalize_inline_styles(&effect_container);
            publish_state();
        });

        THEME_APP.with(|slot| {
            *slot.borrow_mut() = Some(ThemeApp { state });
        });
        publish_state();
        set_ready();
        Ok(())
    }

    /// 把挂载出来的**内联样式**过一遍浏览器的 CSSOM（**归一化**）。
    ///
    /// ## 为什么需要
    ///
    /// 生产用样式指令 ⇒ 浏览器把取值**序列化**成自己的口径 ⇒ 参考侧 DOM 里的
    /// `style` 属性是**序列化后的串**。引擎侧写的是**原文** ⇒ 值一样、串不同
    /// ⇒ DOM 逐字对照不等。
    ///
    /// ⇒ 这里把同一段取值**再交给 CSSOM 走一遍**（`style.cssText = 原文`）
    /// ⇒ **由浏览器自己序列化** ⇒ 两侧同口径、逐字相等。
    ///
    /// ## 为什么用 `Reflect`
    ///
    /// 本 crate 的 web-sys feature 集**最小化**（没有 `CssStyleDeclaration`）
    /// ⇒ 用 `js_sys::Reflect` 取 `style` 对象并写 `cssText`（**不加依赖**）。
    fn normalize_inline_styles(container: &Element) -> Result<(), String> {
        let query =
            js_sys::Reflect::get(container.as_ref(), &JsValue::from_str("querySelectorAll"))
                .map_err(|error| format!("取 `querySelectorAll` 失败：{error:?}"))?;
        let query = query
            .dyn_into::<js_sys::Function>()
            .map_err(|_| "取 `querySelectorAll` 失败：不是函数".to_owned())?;
        let list = query
            .call1(container.as_ref(), &JsValue::from_str("[style]"))
            .map_err(|error| format!("查内联样式元素失败：{error:?}"))?;
        let length = js_sys::Reflect::get(&list, &JsValue::from_str("length"))
            .ok()
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0);
        for index in 0..(length as u32) {
            let item = js_sys::Reflect::get(&list, &JsValue::from_str(&index.to_string()))
                .map_err(|error| format!("取内联样式元素失败：{error:?}"))?;
            let Some(element) = item.dyn_ref::<Element>() else {
                continue;
            };
            let Some(raw) = element.get_attribute("style") else {
                continue;
            };
            let style = js_sys::Reflect::get(element.as_ref(), &JsValue::from_str("style"))
                .map_err(|error| format!("取样式对象失败：{error:?}"))?;
            js_sys::Reflect::set(
                &style,
                &JsValue::from_str("cssText"),
                &JsValue::from_str(&raw),
            )
            .map_err(|error| format!("写样式对象失败：{error:?}"))?;
        }
        Ok(())
    }

    /// 标记「已挂载」（对照脚本的 `ready()` 判据；两侧同名同义）。
    fn set_ready() {
        let Some(window) = web_sys::window() else {
            return;
        };
        let _ = js_sys::Reflect::set(
            window.as_ref(),
            &JsValue::from_str("__SETTINGS_READY__"),
            &JsValue::TRUE,
        );
    }

    /// 记录一条**显式错误**（对照脚本可读；不静默）。
    fn report_error(message: &str) {
        let Some(window) = web_sys::window() else {
            return;
        };
        let _ = js_sys::Reflect::set(
            window.as_ref(),
            &JsValue::from_str("__SETTINGS_ERROR__"),
            &JsValue::from_str(message),
        );
    }

    /// 把宿主状态**发布**到页面的全局（对照脚本可读；两侧同名同义）。
    ///
    /// ## 为什么用全局而不是 wasm 导出
    ///
    /// ⚠️ **二进制 crate 里所有子面的模块同体**（`main.rs` 声明了每个面的模块）
    /// ⇒ 若各面都导出同名 wasm 函数 ⇒ **符号重复 ⇒ 链接失败**。
    /// ⇒ 本面**不导出** wasm 函数（面与面之间零同名风险）⇒ 状态用
    /// `window.__SETTINGS_*` 全局发布（对照脚本只读）。
    fn publish_state() {
        let Some(window) = web_sys::window() else {
            return;
        };
        let (active_theme, active_accent, intent) = THEME_APP.with(|slot| {
            let borrowed = slot.borrow();
            let Some(app) = borrowed.as_ref() else {
                return (String::new(), String::new(), String::new());
            };
            (
                app.state.active_theme().get(),
                app.state.active_accent().get(),
                app.state.intent().get(),
            )
        });
        let entries = [
            ("__SETTINGS_ACTIVE_THEME__", active_theme),
            ("__SETTINGS_ACTIVE_ACCENT__", active_accent),
            ("__SETTINGS_INTENT__", intent),
        ];
        for (key, value) in entries {
            let _ = js_sys::Reflect::set(
                window.as_ref(),
                &JsValue::from_str(key),
                &JsValue::from_str(&value),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    //! 固定数据装载与 props 组装的**行为**断言。

    use super::*;
    use apeireth_ui_core::Runtime;

    #[test]
    fn the_json_parser_reads_the_fixture_shape() {
        let value =
            parse_json(r#"{"a": [1, 2.5, true, null], "b": {"c": "x"}}"#).expect("解析成功");
        assert_eq!(
            value
                .get("a")
                .and_then(Json::as_array)
                .map(|items| items.len()),
            Some(4)
        );
        assert_eq!(
            value
                .get("b")
                .and_then(|item| item.get("c"))
                .and_then(Json::as_str),
            Some("x")
        );
    }

    #[test]
    fn the_json_parser_keeps_the_escape_forms() {
        let value = parse_json(r#"{"k": "a\nb\t\"c\"é"}"#).expect("解析成功");
        assert_eq!(value.get("k").and_then(Json::as_str), Some("a\nb\t\"c\"é"));
    }

    #[test]
    fn the_json_parser_reports_broken_input() {
        assert!(parse_json("{\"a\": }").is_err(), "坏输入必须显式报错");
        assert!(parse_json("").is_err(), "空输入必须显式报错");
        assert!(parse_json("{} {}").is_err(), "尾部多余内容必须显式报错");
    }

    #[test]
    fn the_fixture_carries_the_ten_themes_and_four_accents() {
        let fixture = theme_fixture().expect("固定数据可读");
        assert_eq!(fixture.themes.len(), 10, "固定数据给了 10 个主题");
        assert_eq!(fixture.accents.len(), 4, "固定数据给了 4 套配色");
        assert_eq!(fixture.active_theme, "origin");
        assert_eq!(fixture.active_accent, "presence-gold");
    }

    #[test]
    fn the_fixture_keeps_the_gradient_verbatim() {
        let fixture = theme_fixture().expect("固定数据可读");
        let swatch = &fixture.themes[0].swatch;
        assert!(
            swatch.starts_with("radial-gradient(circle at 78% 22%,"),
            "渐变串必须逐字取自固定数据：{swatch}"
        );
        assert!(
            swatch.contains("#ffffff"),
            "渐变串里的颜色**不转写**（逐字）：{swatch}"
        );
    }

    #[test]
    fn the_props_carry_the_fixture_data() {
        let fixture = theme_fixture().expect("固定数据可读");
        let state = ThemeHostState::new(fixture.clone(), Runtime::new().reactive_scope());
        let props = theme_panel_props(&fixture, &state);
        assert_eq!(props.themes.len(), 10);
        assert_eq!(props.accents.len(), 4);
        assert_eq!(props.active_theme, "origin");
        assert_eq!(props.active_accent, "presence-gold");
        assert!(!props.custom_bg);
        assert!(props.custom_preview_url.is_none());
        assert!(props.bg_error.is_empty());
    }

    #[test]
    fn picking_a_theme_updates_the_host_state_only() {
        let fixture = theme_fixture().expect("固定数据可读");
        let state = ThemeHostState::new(fixture.clone(), Runtime::new().reactive_scope());
        let props = theme_panel_props(&fixture, &state);
        let entry = props.themes.first().expect("有主题");
        let callback = entry.on_pick.clone().expect("有意图出口");
        callback.call();
        assert_eq!(state.active_theme().get(), "origin");
    }

    #[test]
    fn the_other_intents_are_recorded_by_the_host() {
        let fixture = theme_fixture().expect("固定数据可读");
        let state = ThemeHostState::new(fixture.clone(), Runtime::new().reactive_scope());
        let props = theme_panel_props(&fixture, &state);
        props.on_request_upload.clone().expect("有上传意图").call();
        assert_eq!(state.intent().get(), "request-upload");
    }
}
