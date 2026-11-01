//! 规则层差分装置 — **Rust 侧**比对测试（生产 TypeScript ↔ 引擎 Rust）。
//!
//! ## 这条测试回答什么
//!
//! 生产前端（TypeScript）与引擎侧（Rust）对**同一条业务规则**各有一份实现
//! ⇒ 两份实现会**漂移** ⇒ 既有对照装置只比**画面**、**不比规则** ⇒ 漂移无人知晓。
//! 本测试把「**同一组输入 ⇒ 两侧各跑一遍 ⇒ 输出逐字相同**」变成 **CI 门禁**。
//!
//! ## 真源
//!
//! - 用例 + TS 侧真值：[`cases.json`]（`crates-ui/apeireth-ui-demo/harness/rules/cases.json`）
//!   ⇒ `expected` 字段由 `gen_expected.mjs`（Node `--experimental-strip-types`，
//!   **直接 import 生产 .ts**）算出 ⇒ 人手改 `expected` 必被本测试判红。
//! - 装置说明 / 归一化口径 / 加规则流程：
//!   `crates-ui/apeireth-ui-demo/harness/rules/README.md`。
//!
//! ## 依赖边界
//!
//! - **0 网络、0 浏览器、0 外部依赖** ⇒ native `cargo test` 直接跑。
//! - 只读磁盘上那一份 JSON ⇒ 解析用手写的 **零依赖 JSON 解析器**（本文件内），
//!   不引 serde（crate 的 P0 纪律是 0 外部依赖，且 `Cargo.toml` 不在本批改写域内）。
//!
//! ## 归一化口径
//!
//! 两侧的类型不同（Rust 的 `ApprovalLedgerEntry` ↔ TS 的同名接口）⇒ 各自转成
//! **可比的 JSON** 再比：
//!
//! - 布尔 / 字符串 ⇒ 原样；
//! - 计数三元组 ⇒ 摊平成 `{all, open, expired}`；
//! - 条目数组 ⇒ `{length, items:[{approvalId, session, sessionTitle, createdAtMs, expiresAtMs}]}`
//!   （缺失与 `null` 一律归一为 `null`；顺序敏感；**不比**额外字段）。
//!
//! ⇒ 逐条规则的归一化一句话写在 `cases.json` 的 `normalization` 字段里，
//! 并由 [`NORMALIZATIONS`] 常量表**双向校验**（文档与适配器不许漂）。

#![forbid(unsafe_code)]

use apeireth_ui_kit::governance_tab::{
    approval_filter_counts, audit_event_group, audit_group_chips, capability_available,
    capability_unavailable_reason, filter_approval_ledger, filter_audit_items, find_capability,
    format_clock_ms, format_date_time_ms, format_duration_ms, format_relative_ms,
    guard_decision_badge, guard_event_row, guard_stage_label, is_approval_expired,
    merge_session_inbox, risk_score_percent, set_time_text_formatter, short_id, span_depth,
    span_duration_ms, span_status_badge, ActivityItem, ApprovalFilter, ApprovalLedgerEntry,
    BadgeVariant, Capability, CapabilityGroup, CapabilityManifest, GuardEvent, PendingApproval,
    TimeTextFormatter, TraceSpanItem,
};
use apeireth_ui_kit::settings_budget::{
    budget_knob_badge, budget_remaining_rows, effective_budget, format_duration,
    format_duration_f64, format_session_totals, resolve_budget_limit_input, BudgetBadge,
    BudgetKnobResolution, BudgetRemainingRow, KnobKey, KnobResolutionStatus, KnobRow, KnobValues,
    OutOfRange, SessionUsageTotals, ICON_INFO,
};
use apeireth_ui_kit::IconData;

// ── 零依赖 JSON 值与解析器 ────────────────────────────────────────────────

/// JSON 值（只覆盖本装置用到的子集）。
#[derive(Debug, Clone, PartialEq)]
enum Json {
    /// 空值。
    Null,
    /// 布尔。
    Bool(bool),
    /// 数值。
    Number(f64),
    /// 字符串。
    String(String),
    /// 数组（保序）。
    Array(Vec<Json>),
    /// 对象（保插入序）。
    Object(Vec<(String, Json)>),
}

impl Json {
    /// 取对象字段（缺省 ⇒ `None`）。
    fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(fields) => fields
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    /// 取字符串（缺省 / 非字符串 ⇒ `None`）。
    fn as_str(&self) -> Option<&str> {
        match self {
            Json::String(text) => Some(text.as_str()),
            _ => None,
        }
    }

    /// 取数值（缺省 / 非数值 ⇒ `None`）。
    fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Number(value) => Some(*value),
            _ => None,
        }
    }

    /// 取布尔（缺省 / 非布尔 ⇒ `None`）。
    fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(flag) => Some(*flag),
            _ => None,
        }
    }

    /// 取数组（缺省 / 非数组 ⇒ `None`）。
    fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Array(items) => Some(items.as_slice()),
            _ => None,
        }
    }
}

/// 递归下降 JSON 解析器（**手写** ⇒ 0 外部依赖）。
struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    /// 建解析器。
    fn new(text: &str) -> Self {
        Self {
            chars: text.chars().collect(),
            pos: 0,
        }
    }

    /// 解析一个完整 JSON 值（尾部只允许空白）。
    fn parse_document(mut self) -> Result<Json, String> {
        self.skip_whitespace();
        let value = self.parse_value()?;
        self.skip_whitespace();
        if self.pos != self.chars.len() {
            return Err(format!("JSON 尾部有多余内容（位置 {}）", self.pos));
        }
        Ok(value)
    }

    /// 读一个值。
    fn parse_value(&mut self) -> Result<Json, String> {
        self.skip_whitespace();
        match self.peek() {
            Some('{') => self.parse_object(),
            Some('[') => self.parse_array(),
            Some('"') => Ok(Json::String(self.parse_string()?)),
            Some('t') => self.parse_literal("true", Json::Bool(true)),
            Some('f') => self.parse_literal("false", Json::Bool(false)),
            Some('n') => self.parse_literal("null", Json::Null),
            Some(ch) if ch == '-' || ch.is_ascii_digit() => self.parse_number(),
            Some(ch) => Err(format!("意外的字符 {ch:?}（位置 {}）", self.pos)),
            None => Err("JSON 为空".to_owned()),
        }
    }

    /// 读一个对象。
    fn parse_object(&mut self) -> Result<Json, String> {
        self.bump();
        let mut fields: Vec<(String, Json)> = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some('}') {
            self.bump();
            return Ok(Json::Object(fields));
        }
        loop {
            self.skip_whitespace();
            if self.peek() != Some('"') {
                return Err(format!("对象的键必须是字符串（位置 {}）", self.pos));
            }
            let key = self.parse_string()?;
            self.skip_whitespace();
            if self.bump() != Some(':') {
                return Err(format!("对象键后缺 ':'（位置 {}）", self.pos));
            }
            let value = self.parse_value()?;
            fields.push((key, value));
            self.skip_whitespace();
            match self.bump() {
                Some(',') => continue,
                Some('}') => return Ok(Json::Object(fields)),
                _ => return Err(format!("对象未闭合（位置 {}）", self.pos)),
            }
        }
    }

    /// 读一个数组。
    fn parse_array(&mut self) -> Result<Json, String> {
        self.bump();
        let mut items: Vec<Json> = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(']') {
            self.bump();
            return Ok(Json::Array(items));
        }
        loop {
            let value = self.parse_value()?;
            items.push(value);
            self.skip_whitespace();
            match self.bump() {
                Some(',') => continue,
                Some(']') => return Ok(Json::Array(items)),
                _ => return Err(format!("数组未闭合（位置 {}）", self.pos)),
            }
        }
    }

    /// 读一个字符串（当前字符是起始引号）。
    fn parse_string(&mut self) -> Result<String, String> {
        self.bump();
        let mut out = String::new();
        loop {
            let Some(ch) = self.bump() else {
                return Err("字符串未闭合".to_owned());
            };
            match ch {
                '"' => return Ok(out),
                '\\' => {
                    let Some(escape) = self.bump() else {
                        return Err("转义未完成".to_owned());
                    };
                    match escape {
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
                            if (0xD800..0xDC00).contains(&code) {
                                if self.peek() != Some('\\') {
                                    return Err("高代理后缺低代理".to_owned());
                                }
                                self.bump();
                                if self.bump() != Some('u') {
                                    return Err("代理对格式非法".to_owned());
                                }
                                let low = self.parse_hex4()?;
                                if !(0xDC00..0xE000).contains(&low) {
                                    return Err("低代理非法".to_owned());
                                }
                                let combined = 0x1_0000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                                let Some(ch) = char::from_u32(combined) else {
                                    return Err("代理对无法转成字符".to_owned());
                                };
                                out.push(ch);
                            } else {
                                let Some(ch) = char::from_u32(code) else {
                                    return Err(format!("码点非法：U+{code:04X}"));
                                };
                                out.push(ch);
                            }
                        }
                        other => return Err(format!("未知转义 \\{other}")),
                    }
                }
                other => out.push(other),
            }
        }
    }

    /// 读四位十六进制（`\u` 的载荷）。
    fn parse_hex4(&mut self) -> Result<u32, String> {
        let mut value = 0u32;
        for _ in 0..4 {
            let Some(ch) = self.bump() else {
                return Err("\\u 转义不足四位".to_owned());
            };
            let Some(digit) = ch.to_digit(16) else {
                return Err(format!("\\u 转义含非十六进制字符 {ch:?}"));
            };
            value = value * 16 + digit;
        }
        Ok(value)
    }

    /// 读一个数值。
    fn parse_number(&mut self) -> Result<Json, String> {
        let start = self.pos;
        if self.peek() == Some('-') {
            self.bump();
        }
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() || ch == '.' || ch == 'e' || ch == 'E' || ch == '+' || ch == '-'
            {
                self.bump();
            } else {
                break;
            }
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        let value = text
            .parse::<f64>()
            .map_err(|_| format!("数值非法：{text}"))?;
        Ok(Json::Number(value))
    }

    /// 读一个字面量（`true` / `false` / `null`）。
    fn parse_literal(&mut self, text: &str, value: Json) -> Result<Json, String> {
        for expected in text.chars() {
            if self.bump() != Some(expected) {
                return Err(format!("字面量非法（期望 {text}）"));
            }
        }
        Ok(value)
    }

    /// 跳过空白。
    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.bump();
            } else {
                break;
            }
        }
    }

    /// 看下一个字符（不消费）。
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    /// 消费一个字符。
    fn bump(&mut self) -> Option<char> {
        let ch = self.peek();
        if ch.is_some() {
            self.pos += 1;
        }
        ch
    }
}

/// 解析 JSON 文本。
fn parse_json(text: &str) -> Result<Json, String> {
    Parser::new(text).parse_document()
}

/// JSON 值 ⇒ 可读文本（错误信息用）。
fn render(value: &Json) -> String {
    match value {
        Json::Null => String::from("null"),
        Json::Bool(flag) => flag.to_string(),
        Json::Number(number) => {
            if number.fract() == 0.0 && number.abs() < 9.007_199_254_740_992e15 {
                format!("{}", *number as i64)
            } else {
                format!("{number}")
            }
        }
        Json::String(text) => format!("\"{text}\""),
        Json::Array(items) => {
            let body: Vec<String> = items.iter().map(render).collect();
            format!("[{}]", body.join(", "))
        }
        Json::Object(fields) => {
            let body: Vec<String> = fields
                .iter()
                .map(|(key, value)| format!("\"{key}\": {}", render(value)))
                .collect();
            format!("{{{}}}", body.join(", "))
        }
    }
}

// ── 时刻文本夹具（**固定 +08:00** ⇒ 与 Node 侧同一时区） ──────────────────

/// 装置固定的时区偏移（Asia/Shanghai ⇒ **UTC+08:00**，无夏令时）。
const PLUS_EIGHT_OFFSET_MS: i64 = 8 * 3_600 * 1_000;

/// JS `Date` 的有效时间范围（±8.64e15 ms）—— 超出 ⇒ `new Date(ts)` 是 Invalid Date。
const JS_DATE_LIMIT_MS: i64 = 8_640_000_000_000_000;

/// 装置的时区名（**必须与 `cases.json` 的 `generator.tz` 一致** ⇒ 自检断言）。
const TIME_ZONE_LABEL: &str = "Asia/Shanghai";

/// 生产 `shortId(id, head = 8)` 的**默认 head**。
///
/// ⚠️ Rust 侧的 `short_id(id, head)` **没有默认参数** ⇒ 用例省略 `head` 时
/// ⇒ 装置**显式**补 8 ⇒ 与生产默认值同口径（若生产默认值改了 ⇒ 装置变红）。
const SHORT_ID_DEFAULT_HEAD: usize = 8;

/// 距 1970-01-01 的天数 ⇒ 民用日历日（Howard Hinnant 的 `civil_from_days`）。
///
/// ⚠️ 与 `governance_tab` 里的同名私有助手**同算法**（那里是私有的 ⇒ 夹具自带一份；
/// 本 crate 是 **0 外部依赖** ⇒ 不能引 chrono）。
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = if month <= 2 { year + 1 } else { year };
    (year, month, day)
}

/// 固定 +08:00 的时刻字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LocalParts {
    /// 年。
    year: i64,
    /// 月（1..=12）。
    month: i64,
    /// 日（1..=31）。
    day: i64,
    /// 时（0..=23）。
    hour: i64,
    /// 分（0..=59）。
    minute: i64,
    /// 秒（0..=59）。
    second: i64,
}

/// 毫秒 ⇒ **+08:00 本地**时刻字段（负数向下取整到秒 ⇒ 与 `new Date(ts)` 的取整一致）。
fn plus_eight_parts(ts_ms: i64) -> LocalParts {
    let local_ms = ts_ms.saturating_add(PLUS_EIGHT_OFFSET_MS);
    let seconds = local_ms.div_euclid(1000);
    let days = seconds.div_euclid(86_400);
    let rest = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    LocalParts {
        year,
        month,
        day,
        hour: rest / 3600,
        minute: (rest % 3600) / 60,
        second: rest % 60,
    }
}

/// 两个时刻是否落在同一个 **+08:00 日历日**（生产按**本地时区**判「同日」）。
fn same_plus_eight_day(left_ms: i64, right_ms: i64) -> bool {
    let left = plus_eight_parts(left_ms);
    let right = plus_eight_parts(right_ms);
    left.year == right.year && left.month == right.month && left.day == right.day
}

/// JS `Date` 的有效范围判定（超出 ⇒ `new Date(ts)` 是 Invalid Date ⇒ 生产返回「—」）。
fn js_date_valid(ts_ms: i64) -> bool {
    (-JS_DATE_LIMIT_MS..=JS_DATE_LIMIT_MS).contains(&ts_ms)
}

/// **固定 +08:00** 的时刻文本器（**测试夹具**；对应宿主的 `BrowserTimeText`）。
///
/// ## 为什么夹具自己实现
///
/// 引擎侧默认的 `UtcTimeText` 是**固定 UTC** ⇒ 与生产的**本地时区**口径不同
/// ⇒ 两侧直接比对必然不等（**假红**，不是规则漂移）
/// ⇒ ⇒ 装置必须在**同一个时区**比对 ⇒ 这里注入**固定 +08:00**（Asia/Shanghai）
/// ⇒ 与 Node 侧的 `TZ=Asia/Shanghai` 对齐（见 `cases.json` 的 `generator.tz`）。
///
/// ## 口径（逐字照抄生产 `governance/format.ts`）
///
/// - 非法日期（超出 JS 有效范围）⇒ 「—」（生产有 `Number.isNaN(d.getTime())` 守卫）；
/// - 时刻 ⇒ `HH:MM:SS`（2 位补零 ⇒ zh-CN 的 h23 口径）；
/// - 短日期时刻 ⇒ 同日只给 `HH:MM`，跨日给 `MM/DD HH:MM`
///   （**按 +08:00 日历日**判同日 —— 与生产的 `toDateString()` 本地日比较同口径）。
struct PlusEightTimeText;

impl TimeTextFormatter for PlusEightTimeText {
    fn clock(&self, ts_ms: i64) -> String {
        if !js_date_valid(ts_ms) {
            return String::from("—");
        }
        let parts = plus_eight_parts(ts_ms);
        format!("{:02}:{:02}:{:02}", parts.hour, parts.minute, parts.second)
    }

    fn date_time(&self, ts_ms: i64, now_ms: i64) -> String {
        if !js_date_valid(ts_ms) {
            return String::from("—");
        }
        let parts = plus_eight_parts(ts_ms);
        let clock = format!("{:02}:{:02}", parts.hour, parts.minute);
        if js_date_valid(now_ms) && same_plus_eight_day(ts_ms, now_ms) {
            return clock;
        }
        format!("{:02}/{:02} {}", parts.month, parts.day, clock)
    }
}

/// 装上**固定 +08:00** 的时刻文本器。
///
/// ⚠️ 进程级槽是 **thread-local** ⇒ 每个测试线程各自装一次即可。
fn install_plus_eight_time_text() {
    set_time_text_formatter(Some(std::rc::Rc::new(PlusEightTimeText)));
}

// ── 装置真源（cases.json） ────────────────────────────────────────────────

/// 装置真源（**编译期**嵌入 ⇒ 零运行时 IO、零新依赖）。
///
/// ## 为什么用 `include_str!` 而不是运行时读文件
///
/// - 仓库的 `clippy.toml` **禁用** `std::fs::read_to_string` / `std::fs::File::open`
///   （要求 `fs_err`）⇒ 运行时读文件要给本 crate **加依赖**，而本 crate 的 P0
///   纪律是 **0 外部依赖**；
/// - `include_str!` 是**编译期**读取 ⇒ **零依赖**，且 **fail-fast**（真源不在仓内
///   ⇒ 编译不过，而不是运行到一半才失败）；
/// - 先例：`crates-ui/apeireth-ui-gpu/tests/fire_shader_naga.rs`（同一口径）。
///
/// ## 路径层数
///
/// 本文件在 `crates-ui/apeireth-ui-kit/tests/` ⇒ `..` = `apeireth-ui-kit/`
/// ⇒ `../..` = `crates-ui/` ⇒ 装置真源在 `crates-ui/apeireth-ui-demo/harness/rules/`。
const CASES_JSON: &str = include_str!("../../apeireth-ui-demo/harness/rules/cases.json");

/// 装置真源的**人类可读路径**（仅用于错误信息）。
const CASES_PATH_LABEL: &str = "crates-ui/apeireth-ui-demo/harness/rules/cases.json";

/// 一个用例。
struct RuleCase {
    /// 用例 id（规则内唯一）。
    id: String,
    /// 输入（两侧共用）。
    args: Json,
    /// TS 侧真值（由 Node 生成）；`None` ⇒ **键缺失**（不是「值为 null」）。
    expected: Option<Json>,
    /// 一致性要求：`same` / `known_drift`。
    parity: String,
    /// `known_drift` 用例的 Rust 侧当前行为。
    rust_expected: Option<Json>,
    /// 漂移说明（`known_drift` 必填）。
    why: Option<String>,
}

/// 一条规则。
struct Rule {
    /// 规则名（Rust 侧函数名）。
    name: String,
    /// 归一化口径（与 [`NORMALIZATIONS`] 表双向校验）。
    normalization: String,
    /// 用例集。
    cases: Vec<RuleCase>,
    /// TS 侧被跳过（跑不通 ⇒ 不伪造 expected ⇒ 显式跳过并提示）。
    ts_status: Option<String>,
    /// 旋钮行夹具（**引擎侧宿主的输入**；= 生产 `BUDGET_KNOB_SPECS` 的规则相关字段）。
    ///
    /// 只有预算类规则有 ⇒ 其余规则为空数组。
    knob_rows: Vec<Json>,
}

/// 读装置真源（编译期嵌入的文本 ⇒ 无运行时 IO）。
fn load_rules() -> Vec<Rule> {
    let document = parse_json(CASES_JSON)
        .unwrap_or_else(|err| panic!("装置真源 {CASES_PATH_LABEL} 解析失败：{err}"));
    let rules = document
        .get("rules")
        .and_then(Json::as_array)
        .unwrap_or_else(|| panic!("装置真源 {CASES_PATH_LABEL} 缺 rules 数组"));
    rules
        .iter()
        .map(|rule| {
            let name = rule
                .get("rule")
                .and_then(Json::as_str)
                .unwrap_or_else(|| panic!("规则缺 rule 名"))
                .to_owned();
            let normalization = rule
                .get("normalization")
                .and_then(Json::as_str)
                .unwrap_or_else(|| panic!("规则「{name}」缺 normalization"))
                .to_owned();
            let ts_status = rule
                .get("ts_status")
                .and_then(Json::as_str)
                .map(str::to_owned);
            let knob_rows = rule
                .get("knob_rows")
                .and_then(Json::as_array)
                .map(<[Json]>::to_vec)
                .unwrap_or_default();
            let mut cases: Vec<RuleCase> = Vec::new();
            for case in rule
                .get("cases")
                .and_then(Json::as_array)
                .unwrap_or_else(|| panic!("规则「{name}」缺 cases 数组"))
            {
                let id = case
                    .get("id")
                    .and_then(Json::as_str)
                    .unwrap_or_else(|| panic!("规则「{name}」有用例缺 id"))
                    .to_owned();
                let args = case
                    .get("args")
                    .cloned()
                    .unwrap_or_else(|| panic!("用例「{name}/{id}」缺 args"));
                let expected = case.get("expected").cloned();
                let parity = case
                    .get("parity")
                    .and_then(Json::as_str)
                    .unwrap_or("same")
                    .to_owned();
                let rust_expected = case.get("rust_expected").cloned();
                let why = case.get("why").and_then(Json::as_str).map(str::to_owned);
                cases.push(RuleCase {
                    id,
                    args,
                    expected,
                    parity,
                    rust_expected,
                    why,
                });
            }
            Rule {
                name,
                normalization,
                cases,
                ts_status,
                knob_rows,
            }
        })
        .collect()
}

/// 装置登记的归一化口径（与 `cases.json` 的 `normalization` 字段**双向校验**）。
const NORMALIZATIONS: &[(&str, &str)] = &[
    (
        "is_approval_expired",
        "布尔原样：TS 的 boolean ⇒ JSON boolean；Rust 的 bool ⇒ JSON boolean（两侧零变换）。",
    ),
    (
        "approval_filter_counts",
        "计数三元组摊平成定长对象：TS 的 Record<ApprovalFilter,number> 取 {all,open,expired} 三个键；Rust 的 (usize,usize,usize) 按同序摊平 ⇒ 比三个整数。",
    ),
    (
        "filter_approval_ledger",
        "数组 ⇒ 长度 + 每项归一化条目：{\"length\": n, \"items\": [{\"approvalId\",\"session\",\"sessionTitle\",\"createdAtMs\",\"expiresAtMs\"}]}；缺失与 null 一律归一为 null；条目顺序敏感（两侧都是保序筛选 + 稳定排序）。",
    ),
    (
        "merge_session_inbox",
        "数组 ⇒ 长度 + 每项归一化条目：{\"length\": n, \"items\": [{\"approvalId\",\"session\",\"sessionTitle\",\"createdAtMs\",\"expiresAtMs\"}]}；缺失与 null 一律归一为 null；条目顺序敏感（两侧都是「同会话整组替换 + 其余不动 + 按 createdAtMs 倒序稳定排序」）。",
    ),
    (
        "format_relative_ms",
        "字符串原样：TS 的 string ⇒ JSON string；Rust 的 String ⇒ JSON string ⇒ 逐码点相等（含中文与空格）。",
    ),
    (
        "format_duration_ms",
        "字符串原样：TS 的 string ⇒ JSON string；Rust 的 String ⇒ JSON string ⇒ 逐码点相等（含 \"ms\" / \"s\" 后缀与小数点）。",
    ),
    (
        "format_clock_ms",
        "字符串原样：TS 的 string ⇒ JSON string；Rust 的 String ⇒ JSON string ⇒ 逐码点相等（两侧同在 Asia/Shanghai：Node 侧 TZ + Rust 侧固定 +08:00 夹具；超出 JS 有效日期范围时两侧同为「—」）。",
    ),
    (
        "format_date_time_ms",
        "字符串原样：TS 的 string ⇒ JSON string；Rust 的 String ⇒ JSON string ⇒ 逐码点相等（两侧同在 Asia/Shanghai：Node 侧 TZ + Rust 侧固定 +08:00 夹具；同日按**本地日历日**判定 ⇒ 同日只给「HH:MM」，跨日给「MM/DD HH:MM」）。",
    ),
    (
        "find_capability",
        "命中 ⇒ {\"found\": true, \"id\", \"supported\", \"available\", \"reason\", \"aliasOf\"}；未命中 ⇒ {\"found\": false, 其余全 null}；缺失与 null 一律归一为 null；只看命中的那一条能力声明（不比整个清单）。",
    ),
    (
        "capability_available",
        "布尔原样：TS 的 boolean ⇒ JSON boolean；Rust 的 bool ⇒ JSON boolean（两侧零变换）。",
    ),
    (
        "capability_unavailable_reason",
        "字符串或 null：TS 的 string | null ⇒ JSON string | null；Rust 的 Option<String> ⇒ 同 ⇒ 逐码点相等。",
    ),
    (
        "format_duration",
        "字符串原样：TS 的 string ⇒ JSON string；Rust 的 String ⇒ JSON string ⇒ 逐码点相等（含「—」占位与「m/s」后缀）。",
    ),
    (
        "format_session_totals",
        "五字段显示串对象：{\"tokens\", \"cache\", \"turns\", \"duration\", \"toolCalls\"} ⇒ 五个字符串逐码点相等（键序固定）。",
    ),
    (
        "resolve_budget_limit_input",
        "四字段对象：{\"value\": 整数|null, \"effective\": 整数, \"status\": \"ok\"|\"clamped\"|\"fallback\", \"feedback\": 字符串}（解析结果摊平；反馈文案逐码点相等）。",
    ),
    (
        "budget_knob_badge",
        "四字段对象：{\"effective\": 整数, \"source\": \"configured\"|\"constant\", \"note\": 字符串, \"label\": 字符串}（徽标摊平；口径注记与徽标文案逐码点相等）。",
    ),
    (
        "effective_budget",
        "三键徽标对象：{\"maxTurnRounds\": 徽标, \"maxToolCalls\": 徽标, \"contextBudgetChars\": 徽标}；徽标 = {\"effective\", \"source\", \"note\", \"label\"}（四字段逐码点相等）。",
    ),
    (
        "budget_remaining_rows",
        "数组 ⇒ 长度 + 每行归一化：{\"length\": n, \"items\": [{\"key\", \"label\", \"scope\", \"cap\", \"used\", \"remaining\", \"ratio\"}]}；ratio 为数字或 null（逐值相等）；行序敏感（固定七行）。",
    ),
    (
        "short_id",
        "字符串原样：TS 的 string ⇒ JSON string；Rust 的 String ⇒ JSON string ⇒ 逐码点相等（按 UTF-16 码元计数与截断；截断位补「…」；head 缺省 ⇒ 两侧同取 8）。",
    ),
    (
        "span_depth",
        "整数原样：TS 的 number ⇒ JSON number；Rust 的 usize ⇒ JSON number ⇒ 逐值相等（父链深度；上限 6；环由 visited 集合截断）。",
    ),
    (
        "span_duration_ms",
        "数值或 null：TS 的 number | null ⇒ JSON number | null；Rust 的 Option<f64> ⇒ 同 ⇒ 逐值相等（未收口 ⇒ null；负时长钳到 0）。",
    ),
    (
        "span_status_badge",
        "两字段对象：{\"variant\": \"green\"|\"danger\"|\"blue\"|\"dim\", \"label\": 字符串}（Rust 的 BadgeVariant 取 class_name ⇒ 与 TS 字面量同形；label 逐码点相等）。",
    ),
    (
        "audit_event_group",
        "字符串原样：TS 的 string ⇒ JSON string；Rust 的 String ⇒ JSON string ⇒ 逐码点相等（取首段并去空白；空 ⇒ 「other」）。",
    ),
    (
        "audit_group_chips",
        "数组 ⇒ 长度 + 每行两字段：{\"length\": n, \"items\": [{\"group\": 字符串, \"count\": 整数}]}；按 count 倒序（两侧皆稳定排序 ⇒ 同计数按首次出现序）；行序敏感。",
    ),
    (
        "filter_audit_items",
        "数组 ⇒ 长度 + 每项归一化条目：{\"length\": n, \"items\": [{\"id\", \"timestamp\", \"title\", \"summary\", \"source\", \"detail\"}]}；detail 缺失与 null 一律归一为 null；条目顺序敏感（保序筛选）。",
    ),
    (
        "guard_decision_badge",
        "两字段对象：{\"variant\": \"green\"|\"danger\"|\"amber\"|\"blue\"|\"dim\", \"label\": 字符串}（Rust 的 BadgeVariant 取 class_name ⇒ 与 TS 字面量同形；label 逐码点相等）。",
    ),
    (
        "guard_stage_label",
        "字符串原样：TS 的 string ⇒ JSON string；Rust 的 String ⇒ JSON string ⇒ 逐码点相等（三段管线中文名；未知名原样返回）。",
    ),
    (
        "risk_score_percent",
        "整数原样：TS 的 number ⇒ JSON number；Rust 的 i64 ⇒ JSON number ⇒ 逐值相等（0..1 钳制后 ×100 取整，半数向 +∞）。",
    ),
    (
        "guard_event_row",
        "四字段对象：{\"id\": 字符串, \"badge\": {\"variant\", \"label\"}, \"stageLabel\": 字符串, \"riskPct\": 整数}（徽标 = 守卫决策徽标语义；riskPct = 风险百分比；键序固定）。",
    ),
    (
        "format_duration_f64",
        "字符串原样：TS 的 string ⇒ JSON string；Rust 的 String ⇒ JSON string ⇒ 逐码点相等（含「—」占位与「m/s」后缀；小数毫秒可比）。",
    ),
];

// ── 适配器：JSON 输入 ⇒ 各自语言的类型 ⇒ 各自函数 ⇒ 归一化 JSON ──────────

/// 可选字符串（缺省 / `null` ⇒ `None`）。
fn optional_string(value: Option<&Json>) -> Result<Option<String>, String> {
    match value {
        None | Some(Json::Null) => Ok(None),
        Some(Json::String(text)) => Ok(Some(text.clone())),
        Some(other) => Err(format!("期望字符串，实际是 {}", render(other))),
    }
}

/// 必填字符串（缺省 ⇒ 空串；`null` ⇒ 空串）。
fn required_string(value: &Json, key: &str) -> Result<String, String> {
    match value.get(key) {
        Some(Json::String(text)) => Ok(text.clone()),
        None | Some(Json::Null) => Ok(String::new()),
        Some(other) => Err(format!("字段 {key} 期望字符串，实际是 {}", render(other))),
    }
}

/// 可选整数毫秒（缺省 / `null` ⇒ `None`）。
fn optional_i64(value: Option<&Json>) -> Result<Option<i64>, String> {
    match value {
        None | Some(Json::Null) => Ok(None),
        Some(Json::Number(number)) => {
            if number.fract() != 0.0 {
                return Err(format!("期望整数毫秒，实际是 {number}"));
            }
            Ok(Some(*number as i64))
        }
        Some(other) => Err(format!("期望整数毫秒，实际是 {}", render(other))),
    }
}

/// 必填整数毫秒。
fn required_i64(value: &Json, key: &str) -> Result<i64, String> {
    optional_i64(value.get(key))?.ok_or_else(|| format!("缺字段 {key}"))
}

/// JSON 待签文书 ⇒ Rust 待签文书。
fn build_pending(value: &Json) -> Result<PendingApproval, String> {
    Ok(PendingApproval {
        approval_id: required_string(value, "approval_id")?,
        session: required_string(value, "session")?,
        tool_name: required_string(value, "tool_name")?,
        governance_reason: required_string(value, "governance_reason")?,
        command_text: optional_string(value.get("command_text"))?,
        arguments_summary: optional_string(value.get("arguments_summary"))?,
        created_at: optional_string(value.get("created_at"))?,
        expires_at: optional_string(value.get("expires_at"))?,
    })
}

/// JSON 账本条目 ⇒ Rust 账本条目。
fn build_entry(value: &Json) -> Result<ApprovalLedgerEntry, String> {
    let approval = build_pending(
        value
            .get("approval")
            .ok_or_else(|| "账本条目缺 approval".to_owned())?,
    )?;
    Ok(ApprovalLedgerEntry {
        approval,
        session_title: optional_string(value.get("sessionTitle"))?,
        created_at_ms: optional_i64(value.get("createdAtMs"))?,
        expires_at_ms: optional_i64(value.get("expiresAtMs"))?,
    })
}

/// JSON 账本条目数组 ⇒ Rust 账本条目数组。
fn build_ledger(value: &Json) -> Result<Vec<ApprovalLedgerEntry>, String> {
    let items = value
        .as_array()
        .ok_or_else(|| "ledger 期望数组".to_owned())?;
    items.iter().map(build_entry).collect()
}

/// JSON 待签文书数组 ⇒ Rust 待签文书数组。
fn build_inbox(value: &Json) -> Result<Vec<PendingApproval>, String> {
    let items = value
        .as_array()
        .ok_or_else(|| "inbox 期望数组".to_owned())?;
    items.iter().map(build_pending).collect()
}

/// Rust 账本条目 ⇒ 归一化 JSON（键序固定；缺失 ⇒ `null`）。
fn normalize_entry(entry: &ApprovalLedgerEntry) -> Json {
    Json::Object(vec![
        (
            String::from("approvalId"),
            Json::String(entry.approval.approval_id.clone()),
        ),
        (
            String::from("session"),
            Json::String(entry.approval.session.clone()),
        ),
        (
            String::from("sessionTitle"),
            match &entry.session_title {
                Some(text) => Json::String(text.clone()),
                None => Json::Null,
            },
        ),
        (
            String::from("createdAtMs"),
            match entry.created_at_ms {
                Some(value) => Json::Number(value as f64),
                None => Json::Null,
            },
        ),
        (
            String::from("expiresAtMs"),
            match entry.expires_at_ms {
                Some(value) => Json::Number(value as f64),
                None => Json::Null,
            },
        ),
    ])
}

/// Rust 账本条目数组 ⇒ 归一化 JSON（长度 + 每项关键字段；顺序敏感）。
fn normalize_entries(entries: &[ApprovalLedgerEntry]) -> Json {
    Json::Object(vec![
        (String::from("length"), Json::Number(entries.len() as f64)),
        (
            String::from("items"),
            Json::Array(entries.iter().map(normalize_entry).collect()),
        ),
    ])
}

/// 可选布尔（缺省 / `null` ⇒ `None`）。
fn optional_bool(value: Option<&Json>) -> Result<Option<bool>, String> {
    match value {
        None | Some(Json::Null) => Ok(None),
        Some(Json::Bool(flag)) => Ok(Some(*flag)),
        Some(other) => Err(format!("期望布尔，实际是 {}", render(other))),
    }
}

/// 可选计数（缺省 / `null` ⇒ `None`）。
fn optional_u64(value: Option<&Json>) -> Result<Option<u64>, String> {
    match value {
        None | Some(Json::Null) => Ok(None),
        Some(Json::Number(number)) => {
            if number.fract() != 0.0 || *number < 0.0 {
                return Err(format!("期望非负整数，实际是 {number}"));
            }
            Ok(Some(*number as u64))
        }
        Some(other) => Err(format!("期望非负整数，实际是 {}", render(other))),
    }
}

/// JSON 能力声明 ⇒ Rust 能力声明。
fn build_capability(value: &Json) -> Result<Capability, String> {
    Ok(Capability {
        id: required_string(value, "id")?,
        supported: value
            .get("supported")
            .and_then(Json::as_bool)
            .unwrap_or(false),
        available: optional_bool(value.get("available"))?,
        reason: optional_string(value.get("reason"))?,
        alias_of: optional_string(value.get("alias_of"))?,
    })
}

/// JSON 能力清单 ⇒ Rust 能力清单（缺省 / `null` ⇒ `None`）。
fn build_manifest(value: Option<&Json>) -> Result<Option<CapabilityManifest>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    if matches!(value, Json::Null) {
        return Ok(None);
    }
    let groups = value
        .get("capabilities")
        .and_then(Json::as_array)
        .ok_or_else(|| "manifest.capabilities 期望数组".to_owned())?;
    let mut built: Vec<CapabilityGroup> = Vec::new();
    for group in groups {
        let items = group
            .get("capabilities")
            .and_then(Json::as_array)
            .ok_or_else(|| "manifest 组缺 capabilities 数组".to_owned())?;
        let mut capabilities: Vec<Capability> = Vec::new();
        for item in items {
            capabilities.push(build_capability(item)?);
        }
        built.push(CapabilityGroup { capabilities });
    }
    Ok(Some(CapabilityManifest {
        capabilities: built,
    }))
}

/// JSON 会话消耗累计 ⇒ Rust 会话消耗累计。
fn build_session_totals(value: &Json) -> Result<SessionUsageTotals, String> {
    Ok(SessionUsageTotals {
        turns: optional_u64(value.get("turns"))?.unwrap_or(0),
        prompt_tokens: optional_i64(value.get("promptTokens"))?,
        completion_tokens: optional_i64(value.get("completionTokens"))?,
        cache_hit_tokens: optional_i64(value.get("cacheHitTokens"))?,
        duration_ms: optional_i64(value.get("durationMs"))?,
        tool_calls: optional_u64(value.get("toolCalls"))?.unwrap_or(0),
        last_turn_tool_calls: optional_i64(value.get("lastTurnToolCalls"))?,
    })
}

/// Rust 能力声明 ⇒ 归一化 JSON（未命中 ⇒ found=false 且其余全 null）。
fn normalize_capability(capability: Option<&Capability>) -> Json {
    let (found, id, supported, available, reason, alias_of) = match capability {
        None => (
            false,
            Json::Null,
            Json::Null,
            Json::Null,
            Json::Null,
            Json::Null,
        ),
        Some(cap) => (
            true,
            Json::String(cap.id.clone()),
            Json::Bool(cap.supported),
            match cap.available {
                Some(flag) => Json::Bool(flag),
                None => Json::Null,
            },
            match &cap.reason {
                Some(text) => Json::String(text.clone()),
                None => Json::Null,
            },
            match &cap.alias_of {
                Some(text) => Json::String(text.clone()),
                None => Json::Null,
            },
        ),
    };
    Json::Object(vec![
        (String::from("found"), Json::Bool(found)),
        (String::from("id"), id),
        (String::from("supported"), supported),
        (String::from("available"), available),
        (String::from("reason"), reason),
        (String::from("aliasOf"), alias_of),
    ])
}

/// 过滤档名 ⇒ Rust 过滤档。
fn build_filter(value: &Json) -> Result<ApprovalFilter, String> {
    match value.as_str() {
        Some("all") => Ok(ApprovalFilter::All),
        Some("open") => Ok(ApprovalFilter::Open),
        Some("expired") => Ok(ApprovalFilter::Expired),
        other => Err(format!("filter 期望 all/open/expired，实际是 {other:?}")),
    }
}

/// 可选浮点数（缺省 / `null` ⇒ `None`）。
fn optional_f64(value: Option<&Json>) -> Result<Option<f64>, String> {
    match value {
        None | Some(Json::Null) => Ok(None),
        Some(Json::Number(number)) => Ok(Some(*number)),
        Some(other) => Err(format!("期望数值，实际是 {}", render(other))),
    }
}

/// 必填浮点数（缺省 ⇒ 显式失败 ⇒ 不静默当 0）。
fn required_f64(value: &Json, key: &str) -> Result<f64, String> {
    optional_f64(value.get(key))?.ok_or_else(|| format!("缺字段 {key}"))
}

/// 可选非负整数（缺省 / `null` ⇒ `None`）。
fn optional_usize(value: Option<&Json>) -> Result<Option<usize>, String> {
    match value {
        None | Some(Json::Null) => Ok(None),
        Some(Json::Number(number)) => {
            if number.fract() != 0.0 || *number < 0.0 {
                return Err(format!("期望非负整数，实际是 {number}"));
            }
            Ok(Some(*number as usize))
        }
        Some(other) => Err(format!("期望非负整数，实际是 {}", render(other))),
    }
}

/// JSON 字符串数组 ⇒ Rust 字符串数组（缺省 / `null` ⇒ 空数组）。
fn build_string_list(value: Option<&Json>) -> Result<Vec<String>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    if matches!(value, Json::Null) {
        return Ok(Vec::new());
    }
    let items = value
        .as_array()
        .ok_or_else(|| "期望字符串数组".to_owned())?;
    items
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| "期望字符串数组".to_owned())
        })
        .collect()
}

/// JSON span 条目 ⇒ Rust span 条目。
///
/// ⚠️ 规则只读 `span_id` / `parent_span_id` / `started_at` / `ended_at`；
/// 其余字段是**夹具填充**（Rust 结构体要求非缺省 ⇒ 置空串 / `None`）。
fn build_trace_span(value: &Json) -> Result<TraceSpanItem, String> {
    Ok(TraceSpanItem {
        span_id: required_string(value, "span_id")?,
        parent_span_id: optional_string(value.get("parent_span_id"))?,
        kind: required_string(value, "kind")?,
        actor: required_string(value, "actor")?,
        status: required_string(value, "status")?,
        summary: optional_string(value.get("summary"))?,
        started_at: required_f64(value, "started_at")?,
        ended_at: optional_f64(value.get("ended_at"))?,
    })
}

/// JSON span 条目数组 ⇒ Rust span 条目数组（缺省 / `null` ⇒ 空数组）。
fn build_trace_spans(value: Option<&Json>) -> Result<Vec<TraceSpanItem>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    if matches!(value, Json::Null) {
        return Ok(Vec::new());
    }
    let items = value
        .as_array()
        .ok_or_else(|| "spans 期望数组".to_owned())?;
    items.iter().map(build_trace_span).collect()
}

/// JSON 留痕条目 ⇒ Rust 留痕条目。
///
/// ⚠️ 规则只读 `title`；其余字段是**夹具填充**（Rust 结构体要求非缺省 ⇒ 置空串）。
fn build_activity_item(value: &Json) -> Result<ActivityItem, String> {
    Ok(ActivityItem {
        id: required_string(value, "id")?,
        timestamp: required_i64(value, "timestamp")?,
        title: required_string(value, "title")?,
        summary: required_string(value, "summary")?,
        source: required_string(value, "source")?,
        detail: optional_string(value.get("detail"))?,
    })
}

/// JSON 留痕条目数组 ⇒ Rust 留痕条目数组（缺省 / `null` ⇒ 空数组）。
fn build_activity_items(value: Option<&Json>) -> Result<Vec<ActivityItem>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    if matches!(value, Json::Null) {
        return Ok(Vec::new());
    }
    let items = value
        .as_array()
        .ok_or_else(|| "items 期望数组".to_owned())?;
    items.iter().map(build_activity_item).collect()
}

/// JSON 守卫事件 ⇒ Rust 守卫事件。
fn build_guard_event(value: &Json) -> Result<GuardEvent, String> {
    Ok(GuardEvent {
        timestamp_ms: required_i64(value, "timestamp_ms")?,
        session_id: required_string(value, "session_id")?,
        trace_id: required_string(value, "trace_id")?,
        round: required_i64(value, "round")?,
        capability_id: required_string(value, "capability_id")?,
        stage: required_string(value, "stage")?,
        decision: required_string(value, "decision")?,
        risk_score: required_f64(value, "risk_score")?,
        reasons: build_string_list(value.get("reasons"))?,
        evidence: build_string_list(value.get("evidence"))?,
    })
}

/// Rust 留痕条目 ⇒ 归一化 JSON（键序固定；`detail` 缺失 ⇒ `null`）。
fn normalize_activity_item(item: &ActivityItem) -> Json {
    Json::Object(vec![
        (String::from("id"), Json::String(item.id.clone())),
        (
            String::from("timestamp"),
            Json::Number(item.timestamp as f64),
        ),
        (String::from("title"), Json::String(item.title.clone())),
        (String::from("summary"), Json::String(item.summary.clone())),
        (String::from("source"), Json::String(item.source.clone())),
        (
            String::from("detail"),
            match &item.detail {
                Some(text) => Json::String(text.clone()),
                None => Json::Null,
            },
        ),
    ])
}

/// Rust 徽标语义 ⇒ 归一化 JSON（`variant` 取 `class_name` ⇒ 与 TS 字面量同形）。
fn normalize_badge_pair(variant: BadgeVariant, label: &str) -> Json {
    Json::Object(vec![
        (
            String::from("variant"),
            Json::String(String::from(variant.class_name())),
        ),
        (String::from("label"), Json::String(label.to_owned())),
    ])
}

// ── 预算类规则的适配件（**旋钮行从 JSON 构造**） ──────────────────────────

/// 夹具占位图标。
///
/// ⚠️ `KnobRow.icon` 是 `&'static IconData` ⇒ JSON 造不出图标数据
/// ⇒ 用 crate 内**既有**的公开图标常量占位 ⇒ **零改动 `src/**`**。
/// ⇒ 这四条规则（解析 / 徽标 / 生效值 / 余量行）**都不读** icon ⇒ 不影响输出。
static ICON_PLACEHOLDER: IconData = ICON_INFO;

/// 旋钮键名 ⇒ Rust 旋钮键（**生产的三枚真旋钮**）。
fn build_knob_key(value: Option<&Json>) -> Result<KnobKey, String> {
    match value.and_then(Json::as_str) {
        Some("maxTurnRounds") => Ok(KnobKey::MaxTurnRounds),
        Some("maxToolCalls") => Ok(KnobKey::MaxToolCalls),
        Some("contextBudgetChars") => Ok(KnobKey::ContextBudgetChars),
        other => Err(format!(
            "旋钮键期望 maxTurnRounds/maxToolCalls/contextBudgetChars，实际是 {other:?}"
        )),
    }
}

/// 越界处置名 ⇒ Rust 越界处置。
fn build_out_of_range(value: Option<&Json>) -> Result<OutOfRange, String> {
    match value.and_then(Json::as_str) {
        Some("clamp") => Ok(OutOfRange::Clamp),
        Some("reject") => Ok(OutOfRange::Reject),
        other => Err(format!("outOfRange 期望 clamp/reject，实际是 {other:?}")),
    }
}

/// 用例里的旋钮行 ⇒ Rust 旋钮行（**引擎侧宿主的输入**）。
///
/// ⚠️ `label` / `env` / `desc` / `hint` 是**宿主喂的文案** ⇒ 这四条规则不读它们
/// ⇒ 夹具置空（不是伪造文案，是「本规则不涉及」）。
fn build_knob_rows(value: &[Json]) -> Result<Vec<KnobRow>, String> {
    let mut rows = Vec::with_capacity(value.len());
    for item in value {
        rows.push(KnobRow {
            key: build_knob_key(item.get("key"))?,
            label: String::new(),
            env: String::new(),
            desc: String::new(),
            min: required_i64(item, "min")?,
            max: optional_i64(item.get("max"))?,
            fallback: required_i64(item, "fallback")?,
            out_of_range: build_out_of_range(item.get("outOfRange"))?,
            icon: &ICON_PLACEHOLDER,
            hint: String::new(),
        });
    }
    Ok(rows)
}

/// 旋钮行夹具 ⇒ 指定键的那一行（缺 ⇒ 显式失败，不静默用别的行）。
fn knob_row(rows: &[KnobRow], key: KnobKey) -> Result<&KnobRow, String> {
    rows.iter()
        .find(|row| row.key == key)
        .ok_or_else(|| format!("knob_rows 缺旋钮 {key:?}"))
}

/// 用例里的旋钮配置值 ⇒ Rust 旋钮配置值（缺省 / `null` ⇒ 未配置）。
fn build_knob_values(value: Option<&Json>) -> Result<KnobValues, String> {
    let mut knobs = KnobValues::default();
    let Some(value) = value else {
        return Ok(knobs);
    };
    if matches!(value, Json::Null) {
        return Ok(knobs);
    }
    knobs.max_turn_rounds = optional_i64(value.get("maxTurnRounds"))?;
    knobs.max_tool_calls = optional_i64(value.get("maxToolCalls"))?;
    knobs.context_budget_chars = optional_i64(value.get("contextBudgetChars"))?;
    Ok(knobs)
}

/// 旋钮输入字面量 ⇒ Rust 侧的文本形态（**镜像脚本侧的 `String(raw)`**）。
///
/// - 缺省 / `null` ⇒ 空串（脚本侧 `(raw ?? '').trim()` ⇒ 非法 ⇒ 回默认）；
/// - 字符串 ⇒ 原样（脚本侧会 `trim()` ⇒ 由 `parse_integer_field` 负责）；
/// - 数字 ⇒ 文本化（脚本侧 `String(raw)`；整数不带小数点 ⇒ 与 Rust 侧同形）。
fn build_raw_text(value: Option<&Json>) -> Result<String, String> {
    match value {
        None | Some(Json::Null) => Ok(String::new()),
        Some(Json::String(text)) => Ok(text.clone()),
        Some(Json::Number(number)) => Ok(format!("{number}")),
        Some(other) => Err(format!(
            "raw 期望字符串/数字/null，实际是 {}",
            render(other)
        )),
    }
}

/// Rust 生效值徽标 ⇒ 归一化 JSON。
fn normalize_badge(badge: &BudgetBadge) -> Json {
    Json::Object(vec![
        (
            String::from("effective"),
            Json::Number(badge.effective as f64),
        ),
        (
            String::from("source"),
            Json::String(String::from(badge.source.as_str())),
        ),
        (String::from("note"), Json::String(badge.note.clone())),
        (String::from("label"), Json::String(badge.label.clone())),
    ])
}

/// Rust 旋钮解析结果 ⇒ 归一化 JSON。
fn normalize_resolution(resolution: &BudgetKnobResolution) -> Json {
    let status = match resolution.status {
        KnobResolutionStatus::Ok => "ok",
        KnobResolutionStatus::Clamped => "clamped",
        KnobResolutionStatus::Fallback => "fallback",
    };
    Json::Object(vec![
        (
            String::from("value"),
            match resolution.value {
                Some(value) => Json::Number(value as f64),
                None => Json::Null,
            },
        ),
        (
            String::from("effective"),
            Json::Number(resolution.effective as f64),
        ),
        (String::from("status"), Json::String(String::from(status))),
        (
            String::from("feedback"),
            Json::String(resolution.feedback.clone()),
        ),
    ])
}

/// Rust 预算余量行 ⇒ 归一化 JSON。
fn normalize_remaining_row(row: &BudgetRemainingRow) -> Json {
    Json::Object(vec![
        (String::from("key"), Json::String(row.key.clone())),
        (String::from("label"), Json::String(row.label.clone())),
        (String::from("scope"), Json::String(row.scope.clone())),
        (String::from("cap"), Json::String(row.cap.clone())),
        (String::from("used"), Json::String(row.used.clone())),
        (
            String::from("remaining"),
            Json::String(row.remaining.clone()),
        ),
        (
            String::from("ratio"),
            match row.ratio {
                Some(value) => Json::Number(value),
                None => Json::Null,
            },
        ),
    ])
}

/// 按规则名跑 Rust 侧实现 ⇒ 归一化 JSON。
fn eval(rule: &Rule, args: &Json) -> Result<Json, String> {
    match rule.name.as_str() {
        "is_approval_expired" => {
            let entry = build_entry(
                args.get("entry")
                    .ok_or_else(|| "args 缺 entry".to_owned())?,
            )?;
            let now_ms = required_i64(args, "nowMs")?;
            Ok(Json::Bool(is_approval_expired(&entry, now_ms)))
        }
        "approval_filter_counts" => {
            let ledger = build_ledger(
                args.get("ledger")
                    .ok_or_else(|| "args 缺 ledger".to_owned())?,
            )?;
            let now_ms = required_i64(args, "nowMs")?;
            let (all, open, expired) = approval_filter_counts(&ledger, now_ms);
            Ok(Json::Object(vec![
                (String::from("all"), Json::Number(all as f64)),
                (String::from("open"), Json::Number(open as f64)),
                (String::from("expired"), Json::Number(expired as f64)),
            ]))
        }
        "filter_approval_ledger" => {
            let ledger = build_ledger(
                args.get("ledger")
                    .ok_or_else(|| "args 缺 ledger".to_owned())?,
            )?;
            let filter = build_filter(
                args.get("filter")
                    .ok_or_else(|| "args 缺 filter".to_owned())?,
            )?;
            let now_ms = required_i64(args, "nowMs")?;
            let selected = filter_approval_ledger(&ledger, filter, now_ms);
            let owned: Vec<ApprovalLedgerEntry> = selected.into_iter().cloned().collect();
            Ok(normalize_entries(&owned))
        }
        "merge_session_inbox" => {
            let ledger = build_ledger(
                args.get("ledger")
                    .ok_or_else(|| "args 缺 ledger".to_owned())?,
            )?;
            let session_id = args
                .get("sessionId")
                .and_then(Json::as_str)
                .ok_or_else(|| "args 缺 sessionId".to_owned())?
                .to_owned();
            let inbox = build_inbox(
                args.get("inbox")
                    .ok_or_else(|| "args 缺 inbox".to_owned())?,
            )?;
            let session_title = optional_string(args.get("sessionTitle"))?;
            let merged =
                merge_session_inbox(&ledger, &session_id, &inbox, session_title.as_deref());
            Ok(normalize_entries(&merged))
        }
        "format_relative_ms" => {
            let ts_ms = required_i64(args, "tsMs")?;
            let now_ms = required_i64(args, "nowMs")?;
            Ok(Json::String(format_relative_ms(ts_ms, now_ms)))
        }
        "format_duration_ms" => {
            let ms = args
                .get("ms")
                .and_then(Json::as_f64)
                .ok_or_else(|| "args 缺 ms".to_owned())?;
            Ok(Json::String(format_duration_ms(ms)))
        }
        "format_clock_ms" => {
            // ⚠️ 进程级时刻文本器默认是**固定 UTC** ⇒ 装置必须装**固定 +08:00**
            // ⇒ 与 Node 侧的 `TZ=Asia/Shanghai` 同一时区（否则必假红）。
            install_plus_eight_time_text();
            let ts_ms = required_i64(args, "tsMs")?;
            Ok(Json::String(format_clock_ms(ts_ms)))
        }
        "format_date_time_ms" => {
            install_plus_eight_time_text();
            let ts_ms = required_i64(args, "tsMs")?;
            let now_ms = required_i64(args, "nowMs")?;
            Ok(Json::String(format_date_time_ms(ts_ms, now_ms)))
        }
        "find_capability" => {
            let manifest = build_manifest(args.get("manifest"))?;
            let id = args
                .get("id")
                .and_then(Json::as_str)
                .ok_or_else(|| "args 缺 id".to_owned())?;
            Ok(normalize_capability(find_capability(manifest.as_ref(), id)))
        }
        "capability_available" => {
            let manifest = build_manifest(args.get("manifest"))?;
            let id = args
                .get("id")
                .and_then(Json::as_str)
                .ok_or_else(|| "args 缺 id".to_owned())?;
            Ok(Json::Bool(capability_available(manifest.as_ref(), id)))
        }
        "capability_unavailable_reason" => {
            let manifest = build_manifest(args.get("manifest"))?;
            let id = args
                .get("id")
                .and_then(Json::as_str)
                .ok_or_else(|| "args 缺 id".to_owned())?;
            Ok(match capability_unavailable_reason(manifest.as_ref(), id) {
                Some(text) => Json::String(text),
                None => Json::Null,
            })
        }
        "format_duration" => {
            let ms = optional_i64(args.get("ms"))?;
            Ok(Json::String(format_duration(ms)))
        }
        "format_session_totals" => {
            let totals = build_session_totals(
                args.get("totals")
                    .ok_or_else(|| "args 缺 totals".to_owned())?,
            )?;
            let view = format_session_totals(&totals);
            Ok(Json::Object(vec![
                (String::from("tokens"), Json::String(view.tokens)),
                (String::from("cache"), Json::String(view.cache)),
                (String::from("turns"), Json::String(view.turns)),
                (String::from("duration"), Json::String(view.duration)),
                (String::from("toolCalls"), Json::String(view.tool_calls)),
            ]))
        }
        "resolve_budget_limit_input" => {
            let rows = build_knob_rows(&rule.knob_rows)?;
            let key = build_knob_key(args.get("key"))?;
            let row = knob_row(&rows, key)?;
            let raw = build_raw_text(args.get("raw"))?;
            Ok(normalize_resolution(&resolve_budget_limit_input(&raw, row)))
        }
        "budget_knob_badge" => {
            let rows = build_knob_rows(&rule.knob_rows)?;
            let key = build_knob_key(args.get("key"))?;
            let row = knob_row(&rows, key)?;
            let configured = optional_i64(args.get("configured"))?;
            Ok(normalize_badge(&budget_knob_badge(configured, row)))
        }
        "effective_budget" => {
            let rows = build_knob_rows(&rule.knob_rows)?;
            let knobs = build_knob_values(args.get("knobs"))?;
            let budget = effective_budget(&knobs, &rows);
            Ok(Json::Object(vec![
                (
                    String::from("maxTurnRounds"),
                    normalize_badge(&budget.max_turn_rounds),
                ),
                (
                    String::from("maxToolCalls"),
                    normalize_badge(&budget.max_tool_calls),
                ),
                (
                    String::from("contextBudgetChars"),
                    normalize_badge(&budget.context_budget_chars),
                ),
            ]))
        }
        "budget_remaining_rows" => {
            let rows = build_knob_rows(&rule.knob_rows)?;
            let knobs = build_knob_values(args.get("knobs"))?;
            let totals = args
                .get("totals")
                .ok_or_else(|| "args 缺 totals".to_owned())?;
            // ⚠️ 生产侧是 `String(totals.turns)` ⇒ 缺 `turns` 会给「undefined」
            // ⇒ 而 Rust 侧是 `u64`（缺省 0）⇒ 两侧不可比 ⇒ 显式失败（不假绿）。
            if totals.get("turns").is_none() {
                return Err("totals 缺 turns（生产侧会显示「undefined」⇒ 两侧不可比）".to_owned());
            }
            let totals = build_session_totals(totals)?;
            let budget = effective_budget(&knobs, &rows);
            let remaining = budget_remaining_rows(&totals, &budget);
            Ok(Json::Object(vec![
                (String::from("length"), Json::Number(remaining.len() as f64)),
                (
                    String::from("items"),
                    Json::Array(remaining.iter().map(normalize_remaining_row).collect()),
                ),
            ]))
        }
        "short_id" => {
            let id = required_string(args, "id")?;
            // ⚠️ 生产 `shortId(id, head = 8)` 的**默认 head = 8**
            // ⇒ 用例省略 head 时 ⇒ 两侧同取 8（Rust 侧没有默认参数 ⇒ 装置显式补）。
            let head = optional_usize(args.get("head"))?.unwrap_or(SHORT_ID_DEFAULT_HEAD);
            Ok(Json::String(short_id(&id, head)))
        }
        "span_depth" => {
            let spans = build_trace_spans(args.get("spans"))?;
            let span =
                build_trace_span(args.get("span").ok_or_else(|| "args 缺 span".to_owned())?)?;
            Ok(Json::Number(span_depth(&spans, &span) as f64))
        }
        "span_duration_ms" => {
            let span =
                build_trace_span(args.get("span").ok_or_else(|| "args 缺 span".to_owned())?)?;
            Ok(match span_duration_ms(&span) {
                Some(value) => Json::Number(value),
                None => Json::Null,
            })
        }
        "span_status_badge" => {
            let status = required_string(args, "status")?;
            let (variant, label) = span_status_badge(&status);
            Ok(normalize_badge_pair(variant, &label))
        }
        "audit_event_group" => {
            let event_name = required_string(args, "eventName")?;
            Ok(Json::String(audit_event_group(&event_name)))
        }
        "audit_group_chips" => {
            let items = build_activity_items(args.get("items"))?;
            let chips = audit_group_chips(&items);
            let rows: Vec<Json> = chips
                .iter()
                .map(|(group, count)| {
                    Json::Object(vec![
                        (String::from("group"), Json::String(group.clone())),
                        (String::from("count"), Json::Number(*count as f64)),
                    ])
                })
                .collect();
            Ok(Json::Object(vec![
                (String::from("length"), Json::Number(chips.len() as f64)),
                (String::from("items"), Json::Array(rows)),
            ]))
        }
        "filter_audit_items" => {
            let items = build_activity_items(args.get("items"))?;
            let group = required_string(args, "group")?;
            let selected = filter_audit_items(&items, &group);
            Ok(Json::Object(vec![
                (String::from("length"), Json::Number(selected.len() as f64)),
                (
                    String::from("items"),
                    Json::Array(
                        selected
                            .iter()
                            .map(|item| normalize_activity_item(item))
                            .collect(),
                    ),
                ),
            ]))
        }
        "guard_decision_badge" => {
            let decision = required_string(args, "decision")?;
            let (variant, label) = guard_decision_badge(&decision);
            Ok(normalize_badge_pair(variant, &label))
        }
        "guard_stage_label" => {
            let stage = required_string(args, "stage")?;
            Ok(Json::String(guard_stage_label(&stage)))
        }
        "risk_score_percent" => {
            let score = required_f64(args, "score")?;
            Ok(Json::Number(risk_score_percent(score) as f64))
        }
        "guard_event_row" => {
            let event = build_guard_event(
                args.get("event")
                    .ok_or_else(|| "args 缺 event".to_owned())?,
            )?;
            let row = guard_event_row(&event);
            let (variant, label) = row.badge;
            Ok(Json::Object(vec![
                (String::from("id"), Json::String(row.id.clone())),
                (String::from("badge"), normalize_badge_pair(variant, &label)),
                (
                    String::from("stageLabel"),
                    Json::String(row.stage_label.clone()),
                ),
                (String::from("riskPct"), Json::Number(row.risk_pct as f64)),
            ]))
        }
        "format_duration_f64" => {
            // ⚠️ 生产 `formatDuration` 收 `number` ⇒ **可小数**
            // ⇒ Rust 侧走 `format_duration_f64`（`format_duration` 的薄封装同实现）。
            let ms = optional_f64(args.get("ms"))?;
            Ok(Json::String(format_duration_f64(ms)))
        }
        other => Err(format!("规则「{other}」没有 Rust 侧适配器")),
    }
}

// ── 比对 ──────────────────────────────────────────────────────────────────

/// 比对一个用例（`same` ⇒ 两侧必须逐字相同；`known_drift` ⇒ 必须仍是已知漂移）。
fn compare(rule: &Rule, case: &RuleCase, actual: &Json) -> Result<(), String> {
    let expected = case.expected.as_ref().ok_or_else(|| {
        format!(
            "规则「{}」用例「{}」缺 expected ⇒ 请跑 gen_expected.mjs",
            rule.name, case.id
        )
    })?;
    match case.parity.as_str() {
        "same" => {
            if actual == expected {
                Ok(())
            } else {
                Err(format!(
                    "规则「{}」用例「{}」两侧不一致：\n  TS 侧（expected）= {}\n  Rust 侧（实际）= {}\n  输入 args = {}",
                    rule.name,
                    case.id,
                    render(expected),
                    render(actual),
                    render(&case.args),
                ))
            }
        }
        "known_drift" => {
            let rust_expected = case.rust_expected.as_ref().ok_or_else(|| {
                format!(
                    "规则「{}」用例「{}」标了 known_drift 但缺 rust_expected",
                    rule.name, case.id
                )
            })?;
            if actual == expected {
                Err(format!(
                    "规则「{}」用例「{}」的**已知漂移已被修好**（两侧现在都是 {}）⇒ 请更新 cases.json（把 parity 改成 same、删掉 rust_expected）。\n  原漂移说明：{}",
                    rule.name,
                    case.id,
                    render(expected),
                    case.why.as_deref().unwrap_or("(无)"),
                ))
            } else if actual != rust_expected {
                Err(format!(
                    "规则「{}」用例「{}」的 Rust 侧行为变了：既不等于 TS 侧 expected（{}），也不等于登记的 rust_expected（{}）。\n  Rust 侧（实际）= {}\n  原漂移说明：{}",
                    rule.name,
                    case.id,
                    render(expected),
                    render(rust_expected),
                    render(actual),
                    case.why.as_deref().unwrap_or("(无)"),
                ))
            } else {
                Ok(())
            }
        }
        other => Err(format!(
            "规则「{}」用例「{}」的 parity 非法：{other}（只允许 same / known_drift）",
            rule.name, case.id
        )),
    }
}

/// 跑一条规则的全部用例（任一不一致 ⇒ panic 并打印两侧值）。
fn assert_rule(rule_name: &str) {
    let rules = load_rules();
    let rule = rules
        .iter()
        .find(|rule| rule.name == rule_name)
        .unwrap_or_else(|| panic!("装置真源里没有规则「{rule_name}」"));
    if rule.ts_status.is_some() {
        eprintln!(
            "[rules] ⚠ 规则「{rule_name}」在 TS 侧跑不通 ⇒ 已跳过（不伪造 expected ⇒ 不假绿）"
        );
        return;
    }
    let mut checked = 0usize;
    let mut drifts = 0usize;
    for case in &rule.cases {
        let actual = eval(rule, &case.args)
            .unwrap_or_else(|err| panic!("规则「{rule_name}」用例「{}」跑不起来：{err}", case.id));
        if case.parity == "known_drift" {
            drifts += 1;
        }
        if let Err(message) = compare(rule, case, &actual) {
            panic!("{message}");
        }
        checked += 1;
    }
    println!(
        "[rules] 规则「{rule_name}」：{checked} 个用例逐字一致（其中 {drifts} 个是已登记的已知漂移）"
    );
}

// ── 测试 ──────────────────────────────────────────────────────────────────

/// 装置真源自检：机器生成的标记、归一化口径、规则覆盖、用例唯一性。
#[test]
fn case_file_is_machine_generated_and_self_consistent() {
    let document = parse_json(CASES_JSON)
        .unwrap_or_else(|err| panic!("装置真源 {CASES_PATH_LABEL} 解析失败：{err}"));

    // `expected` 必须由 Node 生成器算出来（不是人手填的）。
    let generator = document
        .get("generator")
        .unwrap_or_else(|| panic!("装置真源缺 generator 标记 ⇒ 请跑 gen_expected.mjs"));
    assert!(
        generator
            .get("script")
            .and_then(Json::as_str)
            .unwrap_or_default()
            .contains("gen_expected.mjs"),
        "装置真源的 generator 标记异常：{}",
        render(generator)
    );
    assert!(
        !generator
            .get("node")
            .and_then(Json::as_str)
            .unwrap_or_default()
            .is_empty(),
        "装置真源的 generator 标记缺 node 版本"
    );
    // 时区标记必须存在（**两侧对齐的关键**）⇒ 且必须与 Rust 侧夹具的时区一致。
    let generator_tz = generator
        .get("tz")
        .and_then(Json::as_str)
        .unwrap_or_default();
    assert!(
        !generator_tz.is_empty(),
        "装置真源的 generator 标记缺时区（时区口径未登记 ⇒ 无法复现）"
    );
    assert_eq!(
        generator_tz, TIME_ZONE_LABEL,
        "装置真源的 generator.tz 与 Rust 侧夹具时区不一致 ⇒ 两侧不在同一时区 ⇒ 比对必假红"
    );

    let rules = load_rules();
    assert!(!rules.is_empty(), "装置真源里没有任何规则");
    let mut total = 0usize;
    for rule in &rules {
        // 归一化口径必须与 Rust 侧适配器登记的常量一致（文档不许漂）。
        let expected_text = NORMALIZATIONS
            .iter()
            .find(|(name, _)| *name == rule.name)
            .map(|(_, text)| *text)
            .unwrap_or_else(|| {
                panic!(
                    "规则「{}」不在 NORMALIZATIONS 表里 ⇒ 请在 rules_parity.rs 里补适配器与归一化口径",
                    rule.name
                )
            });
        assert_eq!(
            rule.normalization, expected_text,
            "规则「{}」的归一化口径与 Rust 侧登记的不一致",
            rule.name
        );
        // 规则必须至少 4 个用例（边界覆盖下限）。
        assert!(
            rule.cases.len() >= 4,
            "规则「{}」只有 {} 个用例 ⇒ 边界覆盖不足（至少 4 个）",
            rule.name,
            rule.cases.len()
        );
        // 用例 id 在规则内唯一。
        let mut ids: Vec<&str> = rule.cases.iter().map(|case| case.id.as_str()).collect();
        ids.sort_unstable();
        let count = ids.len();
        ids.dedup();
        assert_eq!(count, ids.len(), "规则「{}」有用例 id 重复", rule.name);
        for case in &rule.cases {
            // `expected` 键必须存在（TS 侧跑不通的用例走 skipped 通道，不在这里）。
            // ⚠️ 判据是**键存在**，不是「值非 null」⇒ 规则真值本身就是 null 的用例合法。
            assert!(
                case.expected.is_some(),
                "规则「{}」用例「{}」缺 expected ⇒ 请跑 gen_expected.mjs",
                rule.name,
                case.id
            );
            // 规则必须能被 Rust 侧适配器执行（防止加了规则却忘了写适配器）。
            eval(rule, &case.args).unwrap_or_else(|err| {
                panic!(
                    "规则「{}」用例「{}」Rust 侧适配器跑不起来：{err}",
                    rule.name, case.id
                )
            });
        }
        total += rule.cases.len();
    }
    println!(
        "[rules] 装置真源自检通过：{} 条规则 / {} 个用例",
        rules.len(),
        total
    );
}

/// `is_approval_expired`：TS ↔ Rust 逐字一致。
#[test]
fn is_approval_expired_matches_production_typescript() {
    assert_rule("is_approval_expired");
}

/// `approval_filter_counts`：TS ↔ Rust 逐字一致。
#[test]
fn approval_filter_counts_matches_production_typescript() {
    assert_rule("approval_filter_counts");
}

/// `filter_approval_ledger`：TS ↔ Rust 逐字一致。
#[test]
fn filter_approval_ledger_matches_production_typescript() {
    assert_rule("filter_approval_ledger");
}

/// `merge_session_inbox`：TS ↔ Rust 逐字一致。
#[test]
fn merge_session_inbox_matches_production_typescript() {
    assert_rule("merge_session_inbox");
}

/// `format_relative_ms`：TS ↔ Rust 逐字一致。
#[test]
fn format_relative_ms_matches_production_typescript() {
    assert_rule("format_relative_ms");
}

/// `format_duration_ms`：TS ↔ Rust 逐字一致。
#[test]
fn format_duration_ms_matches_production_typescript() {
    assert_rule("format_duration_ms");
}

/// `format_clock_ms`：TS ↔ Rust 逐字一致（**两侧同在 Asia/Shanghai**）。
#[test]
fn format_clock_ms_matches_production_typescript() {
    assert_rule("format_clock_ms");
}

/// `format_date_time_ms`：TS ↔ Rust 逐字一致（**同日按本地日历日**）。
#[test]
fn format_date_time_ms_matches_production_typescript() {
    assert_rule("format_date_time_ms");
}

/// `find_capability`：TS ↔ Rust 逐字一致。
#[test]
fn find_capability_matches_production_typescript() {
    assert_rule("find_capability");
}

/// `capability_available`：TS ↔ Rust 逐字一致。
#[test]
fn capability_available_matches_production_typescript() {
    assert_rule("capability_available");
}

/// `capability_unavailable_reason`：TS ↔ Rust 逐字一致。
#[test]
fn capability_unavailable_reason_matches_production_typescript() {
    assert_rule("capability_unavailable_reason");
}

/// `format_duration`：TS ↔ Rust 逐字一致。
#[test]
fn format_duration_matches_production_typescript() {
    assert_rule("format_duration");
}

/// `format_session_totals`：TS ↔ Rust 逐字一致。
#[test]
fn format_session_totals_matches_production_typescript() {
    assert_rule("format_session_totals");
}

/// `resolve_budget_limit_input`：TS ↔ Rust 逐字一致。
#[test]
fn resolve_budget_limit_input_matches_production_typescript() {
    assert_rule("resolve_budget_limit_input");
}

/// `budget_knob_badge`：TS ↔ Rust 逐字一致。
#[test]
fn budget_knob_badge_matches_production_typescript() {
    assert_rule("budget_knob_badge");
}

/// `effective_budget`：TS ↔ Rust 逐字一致。
#[test]
fn effective_budget_matches_production_typescript() {
    assert_rule("effective_budget");
}

/// `budget_remaining_rows`：TS ↔ Rust 逐字一致。
#[test]
fn budget_remaining_rows_matches_production_typescript() {
    assert_rule("budget_remaining_rows");
}

/// `short_id`：TS ↔ Rust 逐字一致（UTF-16 码元口径）。
#[test]
fn short_id_matches_production_typescript() {
    assert_rule("short_id");
}

/// `span_depth`：TS ↔ Rust 逐字一致。
#[test]
fn span_depth_matches_production_typescript() {
    assert_rule("span_depth");
}

/// `span_duration_ms`：TS ↔ Rust 逐字一致。
#[test]
fn span_duration_ms_matches_production_typescript() {
    assert_rule("span_duration_ms");
}

/// `span_status_badge`：TS ↔ Rust 逐字一致。
#[test]
fn span_status_badge_matches_production_typescript() {
    assert_rule("span_status_badge");
}

/// `audit_event_group`：TS ↔ Rust 逐字一致。
#[test]
fn audit_event_group_matches_production_typescript() {
    assert_rule("audit_event_group");
}

/// `audit_group_chips`：TS ↔ Rust 逐字一致。
#[test]
fn audit_group_chips_matches_production_typescript() {
    assert_rule("audit_group_chips");
}

/// `filter_audit_items`：TS ↔ Rust 逐字一致。
#[test]
fn filter_audit_items_matches_production_typescript() {
    assert_rule("filter_audit_items");
}

/// `guard_decision_badge`：TS ↔ Rust 逐字一致。
#[test]
fn guard_decision_badge_matches_production_typescript() {
    assert_rule("guard_decision_badge");
}

/// `guard_stage_label`：TS ↔ Rust 逐字一致。
#[test]
fn guard_stage_label_matches_production_typescript() {
    assert_rule("guard_stage_label");
}

/// `risk_score_percent`：TS ↔ Rust 逐字一致。
#[test]
fn risk_score_percent_matches_production_typescript() {
    assert_rule("risk_score_percent");
}

/// `guard_event_row`：TS ↔ Rust 逐字一致。
#[test]
fn guard_event_row_matches_production_typescript() {
    assert_rule("guard_event_row");
}

/// `format_duration_f64`：TS ↔ Rust 逐字一致（**小数毫秒面**）。
#[test]
fn format_duration_f64_matches_production_typescript() {
    assert_rule("format_duration_f64");
}
