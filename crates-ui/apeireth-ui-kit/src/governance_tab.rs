//! 治理卷宗面组件（四个卷宗 Tab + 共享子件）。
//!
//! ## 边界（**引擎不迁数据面**）
//!
//! 本模块**只回答「给定数据 ⇒ 渲染出同样的画面」**：
//!
//! | 面 | 处置 |
//! |---|---|
//! | 列表 / 详情 / 空态 / 错误态 / 加载态 / 能力不支持态 | **迁**（本模块） |
//! | 拉数据（HTTP）与变更调用（签批 / 撤销 / 试算 / 刷新） | **不迁**（留在宿主） |
//!
//! ⇒ 组件的 props 里只有**数据**与**宿主注入的回调**；组件自身不发请求、
//! 不持有连接。
//!
//! ## 真值口径（与生产组件逐字一致）
//!
//! - 条件表达式（`{#if}` / `{#each}` 的判定）、数字与文本格式化、缺省值处理
//!   **逐字照抄**生产组件与其引用的纯逻辑模块（`governance/format.ts`、
//!   `governance/ledger.ts`、`governance/trace.ts`、`governance/guard.ts`）。
//! - **class 名与结构**与生产一致（设计文档 §9.1：class 名是视觉契约）。
//!
//! ## 共享组件的复用口径
//!
//! | 生产组件 | 本模块的处置 | 理由 |
//! |---|---|---|
//! | `StatusBadge.svelte` | **复用** [`crate::badge::Badge`] | 结构相同（`<span>` + class + 文本）；class 名由调用方给（§9.1） |
//! | `EmptyState.svelte` | 本模块内实现 | 结构（图标 + 标题 + 描述 + 插槽）与 `Badge` / `Button` / `CardShell` 均不同 |
//! | `ErrorState.svelte` | 本模块内实现 | 同上（带图标 + 重试按钮） |
//! | `LoadingState.svelte` | 本模块内实现 | 同上（转圈图标 + 文本） |
//! | `GovManifestPending.svelte` | 本模块内实现 | 治理面专属（清单未到 vs 不支持是两种语义） |
//! | `GovUnsupported.svelte` | 本模块内实现 | 同上 |
//!
//! ## 时刻格式化的时区边界
//!
//! 生产的 `formatDateTimeMs` / `formatClockMs` 走 `toLocaleTimeString('zh-CN')`
//! ⇒ 依赖**运行环境的本地时区**。本模块按**对照 harness 固定的 UTC** 实现
//! （实测 Chrome 的 zh-CN 输出：`06:30` / `10/20`），并在
//! [`format_date_time_ms`] 的文档里登记该边界。
//!
//! ## 空白
//!
//! 生产模板里相邻元素之间的换行会折叠成**一个空格**（文本节点）⇒ 本模块
//! **逐个复现**这些空白文本节点（否则 DOM 逐字比对会出现差异）。

use std::rc::Rc;

use crate::badge::{Badge, BadgeProps};
use crate::icon::{Icon, IconProps};
use crate::icons_governance::{
    ICON_CHEVRON_DOWN, ICON_CHEVRON_RIGHT, ICON_CIRCLE_ALERT, ICON_CLOCK, ICON_CLOUD_OFF,
    ICON_FLASK_CONICAL, ICON_LOADER_CIRCLE, ICON_RADIO, ICON_ROTATE_CCW, ICON_SCROLL_TEXT,
    ICON_SHIELD_ALERT, ICON_SHIELD_OFF,
};
use crate::num::{number_to_string, string_length, string_slice, to_fixed};
use apeireth_ui_core::{Signal, UiComponent};
use apeireth_ui_dom::{Children, ElementHandler, EventKind, VNode};
use apeireth_ui_macro::view;

// ── §1 时刻与 id 格式化（`governance/format.ts` 的逐字移植） ─────────────────

/// 民用日历日 ⇒ 距 1970-01-01 的天数（Howard Hinnant 的 `days_from_civil`）。
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let shifted_month = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// 距 1970-01-01 的天数 ⇒ 民用日历日（`days_from_civil` 的逆）。
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

/// UTC 时刻的各字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UtcParts {
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

/// 毫秒 ⇒ UTC 时刻字段（负数向下取整到秒 ⇒ 与 `new Date(ts)` 的取整一致）。
fn utc_parts(ts_ms: i64) -> UtcParts {
    let seconds = ts_ms.div_euclid(1000);
    let days = seconds.div_euclid(86_400);
    let rest = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    UtcParts {
        year,
        month,
        day,
        hour: rest / 3600,
        minute: (rest % 3600) / 60,
        second: rest % 60,
    }
}

/// 两个时刻是否落在同一个 UTC 日。
fn same_utc_day(left_ms: i64, right_ms: i64) -> bool {
    let left = utc_parts(left_ms);
    let right = utc_parts(right_ms);
    left.year == right.year && left.month == right.month && left.day == right.day
}

/// `HH:MM:SS` 时刻（生产 `formatClockMs` 的口径）。
///
/// ⚠️ **时区边界**：生产走 `toLocaleTimeString('zh-CN')`（本地时区）；本实现
/// 固定 **UTC**（对照 harness 的时区口径，实测 Chrome 的 zh-CN 输出为
/// `06:30:15` 形态）。
pub fn format_clock_ms(ts_ms: i64) -> String {
    let parts = utc_parts(ts_ms);
    format!("{:02}:{:02}:{:02}", parts.hour, parts.minute, parts.second)
}

/// `MM/DD HH:MM` 短日期时刻（同日 ⇒ 只给 `HH:MM`）。
///
/// 生产 `formatDateTimeMs` 的口径：同日 ⇒ `toLocaleTimeString`；跨日 ⇒
/// `toLocaleDateString` + 空格 + 时刻。实测 Chrome 的 zh-CN 输出为
/// `10/20 06:30`（日期分隔符是 `/`）。
///
/// ⚠️ **时区边界**：同 [`format_clock_ms`]（固定 UTC）。
pub fn format_date_time_ms(ts_ms: i64, now_ms: i64) -> String {
    let parts = utc_parts(ts_ms);
    let clock = format!("{:02}:{:02}", parts.hour, parts.minute);
    if same_utc_day(ts_ms, now_ms) {
        return clock;
    }
    format!("{:02}/{:02} {}", parts.month, parts.day, clock)
}

/// 相对时间：刚刚 / N 秒前 / N 分钟前 / N 小时前 / N 天前。
///
/// 生产 `formatRelativeMs` 的逐字移植。
pub fn format_relative_ms(ts_ms: i64, now_ms: i64) -> String {
    let diff_sec = (now_ms - ts_ms).div_euclid(1000);
    if diff_sec < 0 {
        return String::from("刚刚");
    }
    if diff_sec < 5 {
        return String::from("刚刚");
    }
    if diff_sec < 60 {
        return format!("{diff_sec} 秒前");
    }
    let diff_min = diff_sec.div_euclid(60);
    if diff_min < 60 {
        return format!("{diff_min} 分钟前");
    }
    let diff_hour = diff_min.div_euclid(60);
    if diff_hour < 24 {
        return format!("{diff_hour} 小时前");
    }
    format!("{} 天前", diff_hour.div_euclid(24))
}

/// 长 id 截断：前 `head` 位 + `…`（按 **UTF-16 码元** 切，与脚本侧一致）。
///
/// 生产 `shortId` 的逐字移植。
pub fn short_id(id: &str, head: usize) -> String {
    if string_length(id) <= head {
        return id.to_owned();
    }
    format!("{}…", string_slice(id, 0, head))
}

/// 毫秒时长 ⇒ 人话（`86` ⇒ `86ms`；`1234` ⇒ `1.23s`）。
///
/// 生产 `formatDurationMs` 的逐字移植。
pub fn format_duration_ms(ms: f64) -> String {
    if ms < 1000.0 {
        return format!("{}ms", number_to_string(ms));
    }
    format!("{}s", to_fixed(ms / 1000.0, 2))
}

// ── §2 RFC 3339 时刻解析（`approvalTimeMs` 的口径） ─────────────────────────

/// RFC 3339 字符串 ⇒ epoch 毫秒；解析失败 ⇒ `None`。
///
/// 口径与生产 `approvalTimeMs` 一致：`Date.parse` 能吃的形态（`YYYY-MM-DDTHH:MM:SS`
/// + 可选小数 + `Z` 或 `±HH:MM` 偏移）。
pub fn parse_rfc3339_ms(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    if bytes.len() < 19 {
        return None;
    }
    let year: i64 = text.get(0..4)?.parse().ok()?;
    if bytes[4] != b'-' {
        return None;
    }
    let month: i64 = text.get(5..7)?.parse().ok()?;
    if bytes[7] != b'-' {
        return None;
    }
    let day: i64 = text.get(8..10)?.parse().ok()?;
    if bytes[10] != b'T' && bytes[10] != b't' && bytes[10] != b' ' {
        return None;
    }
    let hour: i64 = text.get(11..13)?.parse().ok()?;
    if bytes[13] != b':' {
        return None;
    }
    let minute: i64 = text.get(14..16)?.parse().ok()?;
    if bytes[16] != b':' {
        return None;
    }
    let second: i64 = text.get(17..19)?.parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    if hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    // 小数秒（超过 3 位则截断到毫秒）。
    let mut cursor = 19usize;
    let mut milli = 0i64;
    if bytes.get(cursor) == Some(&b'.') {
        cursor += 1;
        let start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        if cursor == start {
            return None;
        }
        let digits = &text[start..cursor];
        let mut millis_text = String::from(&digits[..digits.len().min(3)]);
        while millis_text.len() < 3 {
            millis_text.push('0');
        }
        milli = millis_text.parse().ok()?;
    }
    // 偏移（`Z` / `±HH:MM` / `±HHMM`）。
    let offset_minutes = match bytes.get(cursor) {
        None => 0,
        Some(b'Z') | Some(b'z') => {
            cursor += 1;
            0
        }
        Some(sign @ (b'+' | b'-')) => {
            let sign = if *sign == b'-' { -1 } else { 1 };
            let hours: i64 = text.get(cursor + 1..cursor + 3)?.parse().ok()?;
            let has_colon = bytes.get(cursor + 3) == Some(&b':');
            let minutes: i64 = if has_colon {
                text.get(cursor + 4..cursor + 6)?.parse().ok()?
            } else {
                text.get(cursor + 3..cursor + 5)?.parse().ok()?
            };
            cursor += if has_colon { 6 } else { 5 };
            sign * (hours * 60 + minutes)
        }
        Some(_) => return None,
    };
    if cursor != bytes.len() {
        return None;
    }
    let days = days_from_civil(year, month, day);
    let seconds = days * 86_400 + hour * 3600 + minute * 60 + second - offset_minutes * 60;
    Some(seconds * 1000 + milli)
}

/// epoch 毫秒 ⇒ RFC 3339 文本（`2026-10-24T09:30:00.000Z`）。
///
/// 与脚本侧 `new Date(ms).toISOString()` **同形** ⇒ 固定数据在两侧可互换。
pub fn format_rfc3339_ms(ts_ms: i64) -> String {
    let parts = utc_parts(ts_ms);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        parts.year,
        parts.month,
        parts.day,
        parts.hour,
        parts.minute,
        parts.second,
        ts_ms.rem_euclid(1000)
    )
}

// ── §3 能力清单（`runtime.ts` 的门控口径逐字移植） ─────────────────────────

/// 一条能力声明。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Capability {
    /// 能力 id。
    pub id: String,
    /// 静态语义（是否实现）。
    pub supported: bool,
    /// 动态语义（此刻是否可调用）；`None` ⇒ 旧清单 ⇒ 回落 `supported`。
    pub available: Option<bool>,
    /// 不可用原因（machine-readable）。
    pub reason: Option<String>,
    /// 兼容别名（指向另一个规范 id）。
    pub alias_of: Option<String>,
}

/// 一组能力声明。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CapabilityGroup {
    /// 组内能力。
    pub capabilities: Vec<Capability>,
}

/// 运行时能力清单。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CapabilityManifest {
    /// 能力组。
    pub capabilities: Vec<CapabilityGroup>,
}

/// 能力别名表（与生产同口径）。
const CAPABILITY_ALIASES: [(&str, &str); 2] = [
    ("approvals.read", "permissions.approval.read"),
    ("approvals.resolve", "permissions.approval.resolve"),
];

/// 取规范 id。
fn canonical_capability_id(id: &str) -> &str {
    CAPABILITY_ALIASES
        .iter()
        .find(|(alias, _)| *alias == id)
        .map(|(_, canonical)| *canonical)
        .unwrap_or(id)
}

/// 跨组查找能力声明（生产 `findCapability` 的逐字移植）。
pub fn find_capability<'a>(
    manifest: Option<&'a CapabilityManifest>,
    id: &str,
) -> Option<&'a Capability> {
    let manifest = manifest?;
    let want = canonical_capability_id(id);
    let mut alias_match: Option<&Capability> = None;
    for group in &manifest.capabilities {
        for cap in &group.capabilities {
            if cap.id == want || cap.id == id {
                return Some(cap);
            }
            let cap_alias = cap.alias_of.as_deref();
            if cap_alias == Some(want) || cap_alias == Some(id) {
                alias_match = Some(cap);
            }
        }
    }
    alias_match
}

/// 该能力**此时此刻**是否可调用（生产 `capabilityAvailable` 的逐字移植）。
pub fn capability_available(manifest: Option<&CapabilityManifest>, id: &str) -> bool {
    let Some(cap) = find_capability(manifest, id) else {
        return false;
    };
    cap.supported && cap.available.unwrap_or(true)
}

/// 该能力不可用的 machine-readable 原因（生产 `capabilityUnavailableReason`）。
pub fn capability_unavailable_reason(
    manifest: Option<&CapabilityManifest>,
    id: &str,
) -> Option<String> {
    let cap = find_capability(manifest, id)?;
    if cap.available == Some(false) {
        return cap.reason.clone();
    }
    None
}

// ── §4 共享子件 ────────────────────────────────────────────────────────────

/// 徽标外观（与生产 `StatusBadge` 的 `variant` 取值同域）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadgeVariant {
    /// 琥珀（待签 / 待完善）。
    Amber,
    /// 绿（正常收口）。
    Green,
    /// 蓝（信息）。
    Blue,
    /// 红（异常）。
    Danger,
    /// 中性。
    Neutral,
    /// 灰（禁用 / 未知）。
    Dim,
}

impl BadgeVariant {
    /// class 名（生产 `StatusBadge` 的 `variant` 字面量）。
    pub fn class_name(self) -> &'static str {
        match self {
            BadgeVariant::Amber => "amber",
            BadgeVariant::Green => "green",
            BadgeVariant::Blue => "blue",
            BadgeVariant::Danger => "danger",
            BadgeVariant::Neutral => "neutral",
            BadgeVariant::Dim => "dim",
        }
    }
}

/// 徽标尺寸（生产 `StatusBadge` 的 `size` 取值）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadgeSize {
    /// 常规。
    Normal,
    /// 紧凑。
    Small,
}

impl BadgeSize {
    /// class 名（生产 `StatusBadge` 的 `size` 字面量）。
    pub fn class_name(self) -> &'static str {
        match self {
            BadgeSize::Normal => "normal",
            BadgeSize::Small => "small",
        }
    }
}

/// 状态徽标（**复用** [`Badge`]：`<span>` + class + 文本）。
///
/// class 名 = `status-badge <variant> <size>`（与生产逐字一致）。
pub fn status_badge(variant: BadgeVariant, label: &str, size: BadgeSize) -> VNode {
    let label = label.to_owned();
    Badge::render(
        &BadgeProps::new("status-badge")
            .extra_class(format!("{} {}", variant.class_name(), size.class_name()))
            .children(Children::new(move || VNode::text(label.clone()))),
    )
}

/// 空态（生产 `EmptyState.svelte`）。
///
/// 结构：图标 + 标题 + （可选）描述 + （可选）插槽。
pub fn empty_state(
    icon: &str,
    title: &str,
    description: Option<&str>,
    children: Option<VNode>,
) -> VNode {
    let icon = icon.to_owned();
    let title = title.to_owned();
    let description = description.map(str::to_owned);
    let has_description = description.is_some();
    let description_text = description.unwrap_or_default();
    let has_children = children.is_some();
    let children_node = children.unwrap_or_else(|| VNode::fragment(Vec::new()));
    view! {
        div(class = "empty-state-wrap") {
            div(class = "empty-mark", aria-hidden = "true") { icon }
            " "
            h3(class = "empty-title") { title }
            " "
            if has_description {
                p(class = "empty-desc") { description_text }
            } else { }
            " "
            if has_children {
                div(class = "empty-actions") { { children_node } }
            } else { }
        }
    }
}

/// 错误态（生产 `ErrorState.svelte`）。
///
/// ⚠️ 生产侧四个卷宗面**都**传了 `onRetry` ⇒ 重试按钮恒在 ⇒ 本实现**总是**
/// 渲染它（`on_retry` 为 `None` 时按钮不接动作 ⇒ 画面一致、行为未接线）。
pub fn error_state(title: &str, message: &str, on_retry: Option<ElementHandler>) -> VNode {
    let title = title.to_owned();
    let message = message.to_owned();
    let icon = Icon::render(&IconProps::new(&ICON_CIRCLE_ALERT, 24));
    let retry_icon = Icon::render(&IconProps::new(&ICON_ROTATE_CCW, 14));
    let retry = on_retry;
    view! {
        div(class = "error-state-wrap", role = "alert") {
            div(class = "error-icon") { { icon } }
            " "
            h3(class = "error-title") { title }
            " "
            p(class = "error-message") { message }
            " "
            button(
                class = "retry-btn",
                onclick = move |event| {
                    if let Some(handler) = retry.clone() {
                        handler.call(event);
                    }
                }
            ) {
                { retry_icon }
                " "
                span { "重试" }
            }
        }
    }
}

/// 加载态（生产 `LoadingState.svelte`）。
pub fn loading_state(message: &str) -> VNode {
    let message = message.to_owned();
    let icon = Icon::render(&IconProps::new(&ICON_LOADER_CIRCLE, 24).extra_class("spin-icon"));
    view! {
        div(class = "loading-state-wrap", role = "status") {
            { icon }
            " "
            p(class = "loading-text") { message }
        }
    }
}

/// 能力清单未到（生产 `GovManifestPending.svelte`）。
///
/// `waited_too_long` = 等待预算（12s）已过 ⇒ 转「诚实说明」分支。
pub fn gov_manifest_pending(message: &str, waited_too_long: bool) -> VNode {
    if !waited_too_long {
        return loading_state(message);
    }
    let icon = Icon::render(&IconProps::new(&ICON_CLOUD_OFF, 22));
    view! {
        div(class = "manifest-pending", role = "status") {
            div(class = "mp-icon") { { icon } }
            " "
            h3(class = "mp-title") { "运行时能力清单迟迟未到" }
            " "
            p(class = "mp-desc") {
                "网关可能离线或仍在启动——这本账需要 gateway 的内省面才能翻开。\n      连接恢复后会自动载入，无需手动重试。"
            }
        }
    }
}

/// 能力不支持（生产 `GovUnsupported.svelte`）。
///
/// `reason` = machine-readable 原因（`None` ⇒ 不渲染原因括号）。
pub fn gov_unsupported(capability_id: &str, reason: Option<&str>, hint: &str) -> VNode {
    let capability_id = capability_id.to_owned();
    let reason = reason.map(str::to_owned);
    let hint = hint.to_owned();
    let icon = Icon::render(&IconProps::new(&ICON_SHIELD_OFF, 14));
    let has_reason = reason.is_some();
    let reason_text = reason.unwrap_or_default();
    let hint_text = hint.to_owned();
    let hint_present = !hint_text.is_empty();
    view! {
        div(class = "gov-unsupported", role = "note") {
            span(class = "u-icon") { { icon } }
            " "
            div(class = "u-body") {
                p(class = "u-title") { "当前运行时不支持此卷宗面" }
                " "
                p(class = "u-detail") {
                    "能力 "
                    code { capability_id }
                    " 未在 runtime capability manifest 中声明为可用"
                    if has_reason {
                        "（原因 "
                        code { reason_text }
                        "）"
                    } else { }
                    "。 "
                    if hint_present {
                        hint_text
                    } else { }
                }
            }
        }
    }
}

// ── §5 审批账本纯逻辑（`governance/ledger.ts` 的逐字移植） ──────────────────

/// 待签文书（后端 canonical DTO 的展示子集）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PendingApproval {
    /// 文书 id。
    pub approval_id: String,
    /// 所属会话 id。
    pub session: String,
    /// 工具名。
    pub tool_name: String,
    /// 治理理由（空串 ⇒ 缺省文案）。
    pub governance_reason: String,
    /// 命令文本（`None` ⇒ 不渲染命令区）。
    pub command_text: Option<String>,
    /// 参数摘要（`None` ⇒ 不渲染参数区）。
    pub arguments_summary: Option<String>,
    /// 创建时刻（RFC 3339 文本）。
    pub created_at: Option<String>,
    /// 过期时刻（RFC 3339 文本）。
    pub expires_at: Option<String>,
}

/// 账本条目（后端文书 + 派生展示字段）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ApprovalLedgerEntry {
    /// 文书本体。
    pub approval: PendingApproval,
    /// 会话标题（`None` ⇒ 显示短 id）。
    pub session_title: Option<String>,
    /// 创建时刻（epoch 毫秒；解析失败 ⇒ `None` ⇒ 不渲染时间行）。
    pub created_at_ms: Option<i64>,
    /// 过期时刻（epoch 毫秒；`None` ⇒ 不渲染过期行）。
    pub expires_at_ms: Option<i64>,
}

/// 审批过滤档。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ApprovalFilter {
    /// 全部。
    #[default]
    All,
    /// 待签（未过期）。
    Open,
    /// 已过期。
    Expired,
}

/// SSE 连接态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SseConnectionState {
    /// 首次建连中。
    #[default]
    Connecting,
    /// 已连接。
    Live,
    /// 断线退避中。
    Reconnecting,
    /// 能力 / 环境无 SSE。
    Unsupported,
}

impl SseConnectionState {
    /// 标签文案（生产 `SSE_STATE_LABEL`）。
    pub fn label(self) -> &'static str {
        match self {
            SseConnectionState::Connecting => "连接中",
            SseConnectionState::Live => "实时已连接",
            SseConnectionState::Reconnecting => "连接中断 · 重连中",
            SseConnectionState::Unsupported => "实时不可用",
        }
    }
}

/// 文书是否已过期（生产 `isApprovalExpired`）。
pub fn is_approval_expired(entry: &ApprovalLedgerEntry, now_ms: i64) -> bool {
    matches!(entry.expires_at_ms, Some(value) if value <= now_ms)
}

/// 三个过滤档的计数（生产 `approvalFilterCounts`）。
pub fn approval_filter_counts(
    ledger: &[ApprovalLedgerEntry],
    now_ms: i64,
) -> (usize, usize, usize) {
    let mut open = 0usize;
    let mut expired = 0usize;
    for entry in ledger {
        if is_approval_expired(entry, now_ms) {
            expired += 1;
        } else {
            open += 1;
        }
    }
    (ledger.len(), open, expired)
}

/// 按过滤档筛选账本（生产 `filterApprovalLedger`）。
pub fn filter_approval_ledger(
    ledger: &[ApprovalLedgerEntry],
    filter: ApprovalFilter,
    now_ms: i64,
) -> Vec<&ApprovalLedgerEntry> {
    match filter {
        ApprovalFilter::All => ledger.iter().collect(),
        ApprovalFilter::Expired => ledger
            .iter()
            .filter(|entry| is_approval_expired(entry, now_ms))
            .collect(),
        ApprovalFilter::Open => ledger
            .iter()
            .filter(|entry| !is_approval_expired(entry, now_ms))
            .collect(),
    }
}

/// 把某会话的 inbox 合入账本（生产 `mergeSessionInbox` 的逐字移植）。
///
/// 该会话的旧条目整体被替换；其余会话不动；结果按创建时刻**倒序**（稳定排序）。
pub fn merge_session_inbox(
    ledger: &[ApprovalLedgerEntry],
    session_id: &str,
    inbox: &[PendingApproval],
    session_title: Option<&str>,
) -> Vec<ApprovalLedgerEntry> {
    let mut merged: Vec<ApprovalLedgerEntry> = ledger
        .iter()
        .filter(|entry| entry.approval.session != session_id)
        .cloned()
        .collect();
    for approval in inbox {
        merged.push(ApprovalLedgerEntry {
            approval: approval.clone(),
            session_title: session_title.map(str::to_owned),
            created_at_ms: approval.created_at.as_deref().and_then(parse_rfc3339_ms),
            expires_at_ms: approval.expires_at.as_deref().and_then(parse_rfc3339_ms),
        });
    }
    merged.sort_by(|left, right| {
        let left_key = left.created_at_ms.unwrap_or(0);
        let right_key = right.created_at_ms.unwrap_or(0);
        right_key.cmp(&left_key)
    });
    merged
}

// ── §6 轨迹 / 留痕纯逻辑（`governance/trace.ts` 的逐字移植） ────────────────

/// 轨迹行（左栏行）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TraceRow {
    /// 轨迹 id。
    pub trace_id: String,
    /// span 数。
    pub span_count: i64,
    /// 开始时刻（epoch 毫秒）。
    pub started_at_ms: i64,
    /// 根 span 的 kind。
    pub root_kind: String,
    /// 根 span 的 actor。
    pub root_actor: String,
    /// 根 span 的 status。
    pub root_status: String,
    /// 根 span 的摘要。
    pub root_summary: String,
    /// 会话 id。
    pub session_id: Option<String>,
    /// 是否带异常。
    pub has_error: bool,
}

/// span 条目。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TraceSpanItem {
    /// span id。
    pub span_id: String,
    /// 父 span id。
    pub parent_span_id: Option<String>,
    /// kind。
    pub kind: String,
    /// actor。
    pub actor: String,
    /// status。
    pub status: String,
    /// 摘要（`None` ⇒ 不渲染摘要行）。
    pub summary: Option<String>,
    /// 开始时刻。
    pub started_at: f64,
    /// 结束时刻（`None` ⇒ 进行中）。
    pub ended_at: Option<f64>,
}

/// 事件留痕条目（左栏行）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActivityItem {
    /// 条目 id。
    pub id: String,
    /// 时刻（epoch 毫秒）。
    pub timestamp: i64,
    /// 标题（同时是分组名的事件前缀）。
    pub title: String,
    /// 摘要。
    pub summary: String,
    /// 来源。
    pub source: String,
    /// 原始记录（`None` ⇒ 不渲染原始记录区）。
    pub detail: Option<String>,
}

/// span 树的父链深度（生产 `spanDepth`；上限 6 层）。
pub fn span_depth(spans: &[TraceSpanItem], span: &TraceSpanItem) -> usize {
    let mut depth = 0usize;
    let mut current = span.parent_span_id.clone();
    let mut guard: Vec<String> = Vec::new();
    while let Some(id) = current {
        if guard.contains(&id) {
            break;
        }
        guard.push(id.clone());
        depth += 1;
        current = spans
            .iter()
            .find(|item| item.span_id == id)
            .and_then(|item| item.parent_span_id.clone());
    }
    depth.min(6)
}

/// span 时长（毫秒）；未收口 ⇒ `None`（生产 `spanDurationMs`）。
pub fn span_duration_ms(span: &TraceSpanItem) -> Option<f64> {
    let ended = span.ended_at?;
    Some((ended - span.started_at).max(0.0))
}

/// span status ⇒ 徽标语义（生产 `spanStatusBadge`）。
pub fn span_status_badge(status: &str) -> (BadgeVariant, String) {
    let lowered = status.to_lowercase();
    if lowered == "ok" {
        return (BadgeVariant::Green, String::from("ok"));
    }
    if lowered == "error" {
        return (BadgeVariant::Danger, String::from("error"));
    }
    if status.is_empty() {
        return (BadgeVariant::Dim, String::from("—"));
    }
    (BadgeVariant::Blue, status.to_owned())
}

/// 事件留痕的分组名（生产 `auditEventGroup`）。
pub fn audit_event_group(event_name: &str) -> String {
    let head = event_name.split('.').next().unwrap_or("").trim();
    if head.is_empty() {
        String::from("other")
    } else {
        head.to_owned()
    }
}

/// 分组计数（按 count 倒序；生产 `auditGroupChips`）。
pub fn audit_group_chips(items: &[ActivityItem]) -> Vec<(String, usize)> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for item in items {
        let group = audit_event_group(&item.title);
        match counts.iter_mut().find(|(name, _)| *name == group) {
            Some(slot) => slot.1 += 1,
            None => counts.push((group, 1)),
        }
    }
    counts.sort_by_key(|item| std::cmp::Reverse(item.1));
    counts
}

/// 按分组筛选留痕（生产 `filterAuditItems`）。
pub fn filter_audit_items<'a>(items: &'a [ActivityItem], group: &str) -> Vec<&'a ActivityItem> {
    if group == "all" {
        return items.iter().collect();
    }
    items
        .iter()
        .filter(|item| audit_event_group(&item.title) == group)
        .collect()
}

/// 轨迹过滤档。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TraceFilter {
    /// 全部。
    #[default]
    All,
    /// 有异常。
    Error,
}

// ── §7 守卫纯逻辑（`governance/guard.ts` 的逐字移植） ───────────────────────

/// 守卫状态。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GuardStatus {
    /// 守卫是否启用。
    pub enabled: bool,
    /// 快速守卫是否装配。
    pub fast_guard_active: bool,
    /// 链式守卫是否装配。
    pub chain_guard_active: bool,
    /// 意图守卫是否装配。
    pub intent_guard_active: bool,
    /// 跨轮监测是否装配。
    pub cross_turn_monitoring_active: bool,
    /// ML 分类器是否可用。
    pub ml_classifier_available: bool,
    /// ML 模型版本。
    pub ml_model_version: Option<String>,
    /// 评估总数。
    pub total_evaluations: i64,
    /// 放行数。
    pub total_allowed: i64,
    /// 否决数。
    pub total_denied: i64,
    /// 转人工数。
    pub total_approval_required: i64,
}

/// 守卫事件。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GuardEvent {
    /// 时刻（epoch 毫秒）。
    pub timestamp_ms: i64,
    /// 会话 id。
    pub session_id: String,
    /// 轨迹 id。
    pub trace_id: String,
    /// 轮次。
    pub round: i64,
    /// 能力 id。
    pub capability_id: String,
    /// 守卫阶段。
    pub stage: String,
    /// 决策。
    pub decision: String,
    /// 风险分（0..=1）。
    pub risk_score: f64,
    /// 理由。
    pub reasons: Vec<String>,
    /// 证据。
    pub evidence: Vec<String>,
}

/// 守卫决策 ⇒ 徽标语义（生产 `guardDecisionBadge`）。
pub fn guard_decision_badge(decision: &str) -> (BadgeVariant, String) {
    let lowered = decision.to_lowercase();
    match lowered.as_str() {
        "allow" | "allowed" => (BadgeVariant::Green, String::from("放行")),
        "deny" | "denied" | "block" | "blocked" => (BadgeVariant::Danger, String::from("否决")),
        "approval_required" | "require_approval" | "approval" => {
            (BadgeVariant::Amber, String::from("待签"))
        }
        "warn" | "warning" => (BadgeVariant::Amber, String::from("预警")),
        _ if decision.is_empty() => (BadgeVariant::Dim, String::from("未知")),
        _ => (BadgeVariant::Blue, decision.to_owned()),
    }
}

/// 守卫阶段 ⇒ 中文名（生产 `guardStageLabel`）。
pub fn guard_stage_label(stage: &str) -> String {
    match stage {
        "fast_guard" => String::from("快速守卫"),
        "chain_guard" => String::from("链式守卫"),
        "decision_fusion" => String::from("决策融合"),
        other => other.to_owned(),
    }
}

/// 风险分 0..1 ⇒ 0..100 整数百分比（生产 `riskScorePercent`）。
pub fn risk_score_percent(score: f64) -> i64 {
    if !score.is_finite() {
        return 0;
    }
    let clamped = score.clamp(0.0, 1.0);
    (clamped * 100.0).round() as i64
}

/// 守卫事件的展示行（生产 `guardEventRow`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardEventRow {
    /// 行 id（`trace-round-timestamp`）。
    pub id: String,
    /// 徽标语义。
    pub badge: (BadgeVariant, String),
    /// 阶段名。
    pub stage_label: String,
    /// 风险百分比。
    pub risk_pct: i64,
}

/// 守卫事件 ⇒ 列表行（生产 `guardEventRow`）。
pub fn guard_event_row(event: &GuardEvent) -> GuardEventRow {
    GuardEventRow {
        id: format!("{}-{}-{}", event.trace_id, event.round, event.timestamp_ms),
        badge: guard_decision_badge(&event.decision),
        stage_label: guard_stage_label(&event.stage),
        risk_pct: risk_score_percent(event.risk_score),
    }
}

/// 试算结果（宿主注入的**求值结果**；引擎不发起调用）。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GuardDryRunResult {
    /// 决策。
    pub decision: String,
    /// 阶段。
    pub stage: String,
    /// 风险分。
    pub risk_score: f64,
    /// 理由。
    pub reasons: Vec<String>,
    /// 证据。
    pub evidence: Vec<String>,
}

// ── §8 共用的构造小工具 ────────────────────────────────────────────────────

/// 按钮（HTML 布尔属性语义：`disabled` 存在即真 ⇒ 值写**空串**）。
///
/// 生产侧用 `button.disabled = <bool>` ⇒ DOM 上是 `disabled=""`（不是
/// `disabled="true"`）⇒ 这里必须逐字对齐，否则 DOM 全等判据会挂。
fn action_button(
    class: &str,
    label: &str,
    disabled: bool,
    title: Option<&str>,
    on_click: Option<ElementHandler>,
) -> VNode {
    let mut button = VNode::element("button").class(class);
    if disabled {
        button = button.attr("disabled", "");
    }
    if let Some(title) = title {
        button = button.attr("title", title);
    }
    if let Some(handler) = on_click {
        button = button.handler(EventKind::Click, handler);
    }
    button.child(VNode::text(label)).build()
}

/// 过滤 chip（文本 + 计数徽标 + 条件 class）。
fn filter_chip(label: &str, count: String, on: bool, on_click: Option<ElementHandler>) -> VNode {
    let mut classes = String::from("gov-chip");
    if on {
        classes.push_str(" on");
    }
    let mut chip = VNode::element("button").attr("class", classes);
    if let Some(handler) = on_click {
        chip = chip.handler(EventKind::Click, handler);
    }
    chip.child(VNode::text(label.to_owned()))
        .child(
            VNode::element("span")
                .attr("class", "chip-n")
                .child(VNode::text(count))
                .build(),
        )
        .build()
}

/// 刷新按钮（`gov-quiet`）：图标（加载时转圈）+ 「刷新」文本。
///
/// ⚠️ 生产侧四个卷宗面的刷新按钮都带 `disabled={loading}` ⇒ 加载态**必须**
/// 写 `disabled` 属性（空串 ⇒ 与生产一致）；图标在加载时带 `gov-spin` class。
fn refresh_button(disabled: bool, spinning: bool, title: Option<&str>) -> VNode {
    let mut icon_props = IconProps::new(&ICON_ROTATE_CCW, 13);
    if spinning {
        icon_props = icon_props.extra_class("gov-spin");
    }
    let icon = Icon::render(&icon_props);
    let mut button = VNode::element("button").class("gov-quiet");
    if disabled {
        button = button.attr("disabled", "");
    }
    if let Some(title) = title {
        button = button.attr("title", title);
    }
    button
        .child(icon)
        .child(VNode::text(String::from(" 刷新")))
        .build()
}

/// 二级过滤 chip（`gov-chip sm`）。
fn sub_filter_chip(
    label: &str,
    count: String,
    on: bool,
    on_click: Option<ElementHandler>,
) -> VNode {
    let mut classes = String::from("gov-chip sm");
    if on {
        classes.push_str(" on");
    }
    let mut chip = VNode::element("button").attr("class", classes);
    if let Some(handler) = on_click {
        chip = chip.handler(EventKind::Click, handler);
    }
    chip.child(VNode::text(label.to_owned()))
        .child(
            VNode::element("span")
                .attr("class", "chip-n")
                .child(VNode::text(count))
                .build(),
        )
        .build()
}

// ── §9 宿主回调类型（**变更调用留在宿主** ⇒ 组件只拿到回调） ──────────────

/// 审批过滤档切换回调。
pub type ApprovalsFilterHandler = Rc<dyn Fn(ApprovalFilter)>;
/// 审批签批回调（`(文书 id, 决策)`；**变更调用留在宿主**）。
pub type ApprovalsResolveHandler = Rc<dyn Fn(&str, &'static str)>;
/// 授权 id 回调（进入卡内确认 / 撤销 / 取消）。
pub type GrantsIdHandler = Rc<dyn Fn(&str)>;
/// 审计数据源切换回调。
pub type AuditSourceHandler = Rc<dyn Fn(AuditSource)>;
/// 轨迹过滤档切换回调。
pub type AuditTraceFilterHandler = Rc<dyn Fn(TraceFilter)>;
/// 留痕分组切换回调。
pub type AuditGroupHandler = Rc<dyn Fn(&str)>;
/// 轨迹 / 留痕选中回调。
pub type AuditSelectHandler = Rc<dyn Fn(&str)>;
/// 守卫事件展开回调。
pub type GuardEventHandler = Rc<dyn Fn(&str)>;

// ── §10 审批收件箱 Tab ──────────────────────────────────────────────────────

/// 审批收件箱 Tab 的 props。
#[derive(Clone)]
pub struct ApprovalsTabProps {
    /// 能力清单（`None` ⇒ 清单未到 ⇒ 等待态）。
    pub capabilities: Option<CapabilityManifest>,
    /// 账本（宿主注入）。
    pub ledger: Vec<ApprovalLedgerEntry>,
    /// 加载态。
    pub loading: bool,
    /// 错误文案。
    pub error: String,
    /// 过滤档（宿主持有 ⇒ 组件保持纯函数）。
    pub filter: Signal<ApprovalFilter>,
    /// SSE 连接态。
    pub sse_state: SseConnectionState,
    /// 当前时刻（epoch 毫秒）。
    pub now_ms: i64,
    /// 正在签批的文书 id。
    pub resolving_id: Option<String>,
    /// 签批提示语。
    pub resolve_note: String,
    /// 签批错误语。
    pub resolve_error: String,
    /// 引导卡主卡（回到对话）。
    pub on_open_chat: Option<ElementHandler>,
    /// 引导卡次卡（去看审计）。
    pub on_go_audit: Option<ElementHandler>,
    /// 签批动作（**变更调用留在宿主**）：`(文书 id, 决策)`。
    pub on_resolve: Option<ApprovalsResolveHandler>,
    /// 过滤档切换（宿主持有信号 ⇒ 组件保持纯函数）。
    pub on_filter: Option<ApprovalsFilterHandler>,
}

/// 审批收件箱 Tab（生产 `ApprovalsTab.svelte`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ApprovalsTab;

impl UiComponent for ApprovalsTab {
    type Props = ApprovalsTabProps;
    type Output = VNode;

    fn render(props: &Self::Props) -> Self::Output {
        let manifest = props.capabilities.as_ref();
        let can_read = capability_available(manifest, "permissions.approval.read");
        let can_resolve = capability_available(manifest, "permissions.approval.resolve");
        let can_list_sessions = capability_available(manifest, "sessions.read");
        let read_reason = capability_unavailable_reason(manifest, "permissions.approval.read");
        let resolve_reason =
            capability_unavailable_reason(manifest, "permissions.approval.resolve");

        let filter = props.filter.get();
        let now_ms = props.now_ms;
        let (count_all, count_open, count_expired) = approval_filter_counts(&props.ledger, now_ms);
        let visible = filter_approval_ledger(&props.ledger, filter, now_ms);

        let capabilities_missing = props.capabilities.is_none();
        let empty_ledger = props.ledger.is_empty();
        let visible_is_empty = visible.is_empty();
        let loading = props.loading;
        let error_present = !props.error.is_empty();
        let resolve_note_present = !props.resolve_note.is_empty();
        let resolve_error_present = !props.resolve_error.is_empty();
        let filter_is_all = matches!(filter, ApprovalFilter::All);
        let filter_is_expired = matches!(filter, ApprovalFilter::Expired);

        // ── 工具条 ──────────────────────────────────────────────────────
        let count_all_text = count_all.to_string();
        let count_open_text = count_open.to_string();
        let count_expired_text = count_expired.to_string();
        let chip_all_on = filter_is_all;
        let chip_open_on = matches!(filter, ApprovalFilter::Open);
        let chip_expired_on = filter_is_expired;
        let chip_all = filter_chip(
            "全部 ",
            count_all_text,
            chip_all_on,
            props.on_filter.as_ref().map(|handler| {
                let handler = handler.clone();
                ElementHandler::new(move |_event| {
                    handler(ApprovalFilter::All);
                })
            }),
        );
        let chip_open = filter_chip(
            "待签 ",
            count_open_text,
            chip_open_on,
            props.on_filter.as_ref().map(|handler| {
                let handler = handler.clone();
                ElementHandler::new(move |_event| {
                    handler(ApprovalFilter::Open);
                })
            }),
        );
        let chip_expired = filter_chip(
            "已过期 ",
            count_expired_text,
            chip_expired_on,
            props.on_filter.as_ref().map(|handler| {
                let handler = handler.clone();
                ElementHandler::new(move |_event| {
                    handler(ApprovalFilter::Expired);
                })
            }),
        );
        let sse_icon = Icon::render(&IconProps::new(&ICON_RADIO, 11));
        let sse_label = format!(" {}", props.sse_state.label());
        let sse_live = matches!(props.sse_state, SseConnectionState::Live);
        let sse_down = matches!(props.sse_state, SseConnectionState::Reconnecting);
        let toolbar = view! {
            div(class = "gov-toolbar") {
                div(class = "gov-chips", role = "tablist", aria-label = "审批过滤") {
                    { chip_all }
                    " "
                    { chip_open }
                    " "
                    { chip_expired }
                }
                " "
                div(class = "gov-toolbar-right") {
                    span(class = "sse-pill", class:live = sse_live, class:down = sse_down) {
                        { sse_icon }
                        sse_label
                    }
                    " "
                    { refresh_button(loading, loading, Some("重新对齐后端账本")) }
                }
            }
        };

        // ── 空态（双引导卡） ────────────────────────────────────────────
        let ember = view! { span(class = "guide-ember", aria-hidden = "true") { } };
        let scroll_icon = Icon::render(&IconProps::new(&ICON_SCROLL_TEXT, 15));
        let guides = view! {
            div(class = "gov-guides") {
                button(class = "guide-card primary") {
                    { ember }
                    " "
                    span(class = "guide-title") { "去对话" }
                    " "
                    span(class = "guide-sub") { "让他在对话里跑一次需要批准的工具调用，金色待签文书会在发生处浮起" }
                }
                " "
                button(class = "guide-card") {
                    { scroll_icon }
                    " "
                    span(class = "guide-title") { "看审计" }
                    " "
                    span(class = "guide-sub") { "已落笔的批准与拒绝，在审计卷里留痕" }
                }
            }
        };
        let empty_all = empty_state(
            "✒",
            "账上无待签",
            Some("当他在对话里停下、把一份文书递到你手边时，这里会同时记下一笔账——何时、哪个会话、哪件工具、为什么。"),
            Some(guides),
        );
        let empty_filtered_description = if filter_is_expired {
            "过期待签会记在这里——当前没有。"
        } else {
            "当前没有等待签字的文书。"
        };
        let empty_filtered = empty_state(
            "✒",
            "此过滤下无文书",
            Some(empty_filtered_description),
            None,
        );

        // ── 账本卡 ──────────────────────────────────────────────────────
        let mut cards: Vec<VNode> = Vec::new();
        for entry in &visible {
            let expired = is_approval_expired(entry, now_ms);
            let busy = props.resolving_id.as_deref() == Some(entry.approval.approval_id.as_str());
            let reason = if entry.approval.governance_reason.is_empty() {
                String::from("治理面要求主人批准")
            } else {
                entry.approval.governance_reason.clone()
            };
            let badge = if expired {
                status_badge(BadgeVariant::Dim, "已过期", BadgeSize::Small)
            } else {
                status_badge(BadgeVariant::Amber, "待签", BadgeSize::Small)
            };
            let command_text = entry.approval.command_text.clone().unwrap_or_default();
            let show_command = !command_text.is_empty();
            let args_text = entry.approval.arguments_summary.clone().unwrap_or_default();
            let show_args = !args_text.is_empty()
                && Some(args_text.as_str()) != entry.approval.command_text.as_deref();
            let args_line = format!("参数摘要：{args_text}");
            let session_label = match entry.session_title.as_deref() {
                Some(title) if !title.is_empty() => title.to_owned(),
                _ => format!("会话 {}", short_id(&entry.approval.session, 8)),
            };
            let show_created = entry.created_at_ms.is_some();
            let created_text = entry
                .created_at_ms
                .map(|value| format_date_time_ms(value, now_ms))
                .unwrap_or_default();
            let show_expiry = entry.expires_at_ms.is_some();
            let expiry_text = match entry.expires_at_ms {
                None => String::new(),
                Some(value) if value <= now_ms => String::from("已过期"),
                Some(value) => {
                    let minutes = ((value - now_ms) as f64 / 60000.0).round().max(1.0);
                    format!("{} 分钟后过期", number_to_string(minutes))
                }
            };
            let expiry_line = format!(" {expiry_text}");
            let disabled = !can_resolve || expired || props.resolving_id.is_some();
            let button_title = if !can_resolve {
                "当前运行时不支持 permissions.approval.resolve"
            } else if expired {
                "文书已过期，等后端清理即可"
            } else {
                ""
            };
            let clock_icon = Icon::render(&IconProps::new(&ICON_CLOCK, 10));
            let approve_label = if busy {
                String::from("签批中…")
            } else {
                String::from("批准")
            };
            let tool_name = entry.approval.tool_name.clone();
            let session_title_attr = entry.approval.session.clone();
            let reject_handler = props.on_resolve.as_ref().map(|resolve| {
                let resolve = resolve.clone();
                let approval_id = entry.approval.approval_id.clone();
                ElementHandler::new(move |_event| {
                    resolve(&approval_id, "reject");
                })
            });
            let approve_handler = props.on_resolve.as_ref().map(|resolve| {
                let resolve = resolve.clone();
                let approval_id = entry.approval.approval_id.clone();
                ElementHandler::new(move |_event| {
                    resolve(&approval_id, "approve");
                })
            });
            let reject_button = action_button(
                "btn-reject",
                "拒绝",
                disabled,
                Some(button_title),
                reject_handler,
            );
            let approve_button = action_button(
                "btn-approve",
                &approve_label,
                disabled,
                Some(button_title),
                approve_handler,
            );
            // ⭐ `key = 审批 id`（列表身份）：筛选「待处理 / 已过期 / 全部」会
            // **换掉整张卡片集合** ⇒ 无 key 时按位置配对 ⇒ **A 卡片的内容被配给 B**
            // ⇒ ⇒ 实测：`code` 与 `footer` 的标签互换 ⇒ 逐像素差 27.4%。
            // ⇒ ⇒ 有 key 时按**身份**配对 ⇒ 消失的移除、新出现的按身份挂上。
            let card_key = entry.approval.approval_id.clone();
            let card = view! {
                article(class = "gov-card", key = card_key, class:faded = expired) {
                    header(class = "gov-card-head") {
                        span(class = "gov-card-title") { tool_name }
                        " "
                        { badge }
                    }
                    " "
                    p(class = "gov-reason") { reason }
                    " "
                    if show_command {
                        code(class = "gov-command", key = "gov-command") { command_text }
                    } else { }
                    " "
                    if show_args {
                        p(class = "gov-args", key = "gov-args") { args_line }
                    } else { }
                    " "
                    footer(class = "gov-card-foot") {
                        span(class = "gov-meta", title = session_title_attr) { session_label }
                        " "
                        if show_created {
                            span(class = "gov-meta mono", key = "gov-created") { created_text }
                        } else { }
                        " "
                        if show_expiry {
                            span(class = "gov-meta mono", key = "gov-expiry", class:warn = !expired) {
                                { clock_icon }
                                expiry_line
                            }
                        } else { }
                        " "
                        span(class = "gov-actions") {
                            { reject_button }
                            " "
                            { approve_button }
                        }
                    }
                }
            };
            cards.push(card);
        }

        let list = view! {
            div(class = "gov-list") {
                for card in cards.clone() {
                    { card }
                }
            }
        };

        let unsupported_resolve = gov_unsupported(
            "permissions.approval.resolve",
            resolve_reason.as_deref(),
            "账本可读，但签批动作不可用。",
        );

        let body = if loading && empty_ledger {
            loading_state("正在对齐各会话的待批账本…")
        } else if error_present && empty_ledger {
            error_state("拉取审批账本失败", &props.error, None)
        } else if visible_is_empty {
            if empty_ledger && filter_is_all {
                empty_all
            } else {
                empty_filtered
            }
        } else {
            let mut parts: Vec<VNode> = vec![list];
            if !can_resolve {
                parts.push(unsupported_resolve);
            }
            VNode::fragment(parts)
        };

        let mut sections: Vec<VNode> = vec![toolbar];
        if !can_list_sessions {
            sections.push(view! {
                p(class = "gov-note") {
                    "会话清单能力 "
                    code { "sessions.read" }
                    " 不可用——账本只覆盖本次运行中观测到审批事件的会话。"
                }
            });
        }
        if resolve_note_present {
            let note = props.resolve_note.clone();
            sections.push(view! { p(class = "gov-note ok") { note } });
        }
        if resolve_error_present {
            let note = props.resolve_error.clone();
            sections.push(view! { p(class = "gov-note bad") { note } });
        }
        sections.push(body);

        let main_content = if capabilities_missing {
            gov_manifest_pending("正在读取运行时能力清单…", false)
        } else if !can_read {
            gov_unsupported(
                "permissions.approval.read",
                read_reason.as_deref(),
                "审批收件箱需要 canonical gateway 的审批内省面。",
            )
        } else {
            VNode::fragment(sections.clone())
        };

        view! {
            div(class = "gov-tab", style = "--gov-accent: var(--ap-register-deepops-blue)") {
                p(class = "gov-position") {
                    "审批在对话内完成——对话里的金色待签文书是「当下」；这里是账：全局待批的总账。已落笔的记录在审计。"
                }
                " "
                { main_content }
            }
        }
    }
}

// ── §10 授权管理 Tab ───────────────────────────────────────────────────────

/// 授权条目（生产 `GrantView` 的展示子集）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GrantView {
    /// 授权 id。
    pub id: String,
    /// 授权名。
    pub name: String,
    /// 入账时刻（epoch 毫秒）；`0` ⇒ 未记录（canonical policy 不记时间戳）。
    pub activated_at_ms: i64,
}

/// 授权管理 Tab 的 props。
#[derive(Clone)]
pub struct GrantsTabProps {
    /// 能力清单（`None` ⇒ 清单未到 ⇒ 等待态）。
    pub capabilities: Option<CapabilityManifest>,
    /// 授权列表（宿主注入）。
    pub grants: Vec<GrantView>,
    /// 加载态。
    pub loading: bool,
    /// 错误文案。
    pub error: String,
    /// 正在确认撤销的授权 id。
    pub confirming_id: Option<String>,
    /// 正在撤销的授权 id。
    pub revoking_id: Option<String>,
    /// 撤销错误语。
    pub revoke_error: String,
    /// 当前时刻（epoch 毫秒）。
    pub now_ms: i64,
    /// 进入卡内确认（**变更调用留在宿主**）。
    pub on_confirm: Option<GrantsIdHandler>,
    /// 撤销（**变更调用留在宿主**）。
    pub on_revoke: Option<GrantsIdHandler>,
    /// 取消确认。
    pub on_cancel: Option<GrantsIdHandler>,
}

impl std::fmt::Debug for GrantsTabProps {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GrantsTabProps")
            .field("grants", &self.grants.len())
            .field("loading", &self.loading)
            .field("error", &self.error)
            .field("confirming_id", &self.confirming_id)
            .field("revoking_id", &self.revoking_id)
            .field("revoke_error", &self.revoke_error)
            .field("now_ms", &self.now_ms)
            .finish_non_exhaustive()
    }
}

/// 授权管理 Tab（生产 `GrantsTab.svelte`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GrantsTab;

impl UiComponent for GrantsTab {
    type Props = GrantsTabProps;
    type Output = VNode;

    fn render(props: &Self::Props) -> Self::Output {
        let manifest = props.capabilities.as_ref();
        let can_read = capability_available(manifest, "permissions.grants.read");
        let can_revoke = capability_available(manifest, "permissions.revoke");
        let read_reason = capability_unavailable_reason(manifest, "permissions.grants.read");
        let revoke_reason = capability_unavailable_reason(manifest, "permissions.revoke");

        let capabilities_missing = props.capabilities.is_none();
        let empty = props.grants.is_empty();
        let loading = props.loading;
        let error_present = !props.error.is_empty();
        let revoke_error_present = !props.revoke_error.is_empty();

        let count_text = format!("{} 项授权在账", props.grants.len());
        let toolbar = view! {
            div(class = "gov-toolbar") {
                span(class = "gov-count mono") { count_text }
                " "
                { refresh_button(loading, loading, None) }
            }
        };

        let mut cards: Vec<VNode> = Vec::new();
        for grant in &props.grants {
            let confirming = props.confirming_id.as_deref() == Some(grant.id.as_str());
            let busy = props.revoking_id.as_deref() == Some(grant.id.as_str());
            let badge = status_badge(BadgeVariant::Green, "生效中", BadgeSize::Small);
            let activated_text = if grant.activated_at_ms > 0 {
                format!(
                    "入账 {}",
                    format_date_time_ms(grant.activated_at_ms, props.now_ms)
                )
            } else {
                String::from("授予时刻未记录（canonical policy 不记时间戳）")
            };
            let revoke_disabled = !can_revoke || props.revoking_id.is_some();
            let revoke_title = if !can_revoke {
                match revoke_reason.as_deref() {
                    Some(reason) => format!("当前运行时不支持 permissions.revoke（{reason}）"),
                    None => String::from("当前运行时不支持 permissions.revoke"),
                }
            } else {
                String::from("撤销此项授权")
            };
            let grant_name = grant.name.clone();
            let grant_id = grant.id.clone();
            let revoke_handler = props.on_revoke.as_ref().map(|revoke| {
                let revoke = revoke.clone();
                let grant_id = grant.id.clone();
                ElementHandler::new(move |_event| {
                    revoke(&grant_id);
                })
            });
            let confirm_handler = props.on_confirm.as_ref().map(|confirm| {
                let confirm = confirm.clone();
                let grant_id = grant.id.clone();
                ElementHandler::new(move |_event| {
                    confirm(&grant_id);
                })
            });
            let cancel_handler = props.on_cancel.as_ref().map(|cancel| {
                let cancel = cancel.clone();
                let grant_id = grant.id.clone();
                ElementHandler::new(move |_event| {
                    cancel(&grant_id);
                })
            });
            let revoke_button = action_button(
                "btn-revoke",
                "撤销",
                revoke_disabled,
                Some(&revoke_title),
                confirm_handler,
            );
            let cancel_button = action_button("btn-cancel", "取消", busy, None, cancel_handler);
            let confirm_label = if busy {
                String::from("撤销中…")
            } else {
                String::from("确认撤销")
            };
            let confirm_button = action_button(
                "btn-confirm-danger",
                &confirm_label,
                busy,
                None,
                revoke_handler,
            );
            let shield_icon = Icon::render(&IconProps::new(&ICON_SHIELD_ALERT, 13));
            let confirm_title = format!(" 确认撤销 {grant_name}？");
            let confirm_block = view! {
                div(class = "revoke-confirm", role = "alert") {
                    p(class = "rc-title") {
                        { shield_icon }
                        confirm_title
                    }
                    " "
                    p(class = "rc-body") {
                        "撤销即刻生效，作用于当前进程的同一 policy 实例；进程重启后按默认策略恢复。\n                  撤销后他再调用此能力会重新停下等审批。"
                    }
                    " "
                    div(class = "rc-actions") {
                        { cancel_button }
                        " "
                        { confirm_button }
                    }
                }
            };
            let card = view! {
                article(class = "gov-card", class:confirming = confirming) {
                    header(class = "gov-card-head") {
                        span(class = "gov-card-title") { grant_name }
                        " "
                        { badge }
                    }
                    " "
                    footer(class = "gov-card-foot") {
                        span(class = "gov-meta mono") { grant_id }
                        " "
                        span(class = "gov-meta") { activated_text }
                        " "
                        if !confirming {
                            span(class = "gov-actions") {
                                { revoke_button }
                            }
                        } else { }
                    }
                    " "
                    if confirming {
                        { confirm_block }
                    } else { }
                }
            };
            cards.push(card);
        }

        let list = view! {
            div(class = "gov-list") {
                for card in cards.clone() {
                    { card }
                }
            }
        };

        let body = if loading && empty {
            loading_state("正在读取授权账本…")
        } else if error_present && empty {
            error_state("拉取授权失败", &props.error, None)
        } else if empty {
            empty_state(
                "⚿",
                "账上无授权",
                Some("当你在审批文书上批准一项能力、或会话策略放宽时，被授权的能力会记在这里——哪一项、属于哪类、何时入账（若后端记录）。"),
                None,
            )
        } else {
            let mut parts: Vec<VNode> = vec![list];
            if !can_revoke {
                parts.push(gov_unsupported(
                    "permissions.revoke",
                    revoke_reason.as_deref(),
                    "授权账可读，撤销动作不可用。",
                ));
            }
            VNode::fragment(parts)
        };

        let mut sections: Vec<VNode> = vec![toolbar];
        if revoke_error_present {
            let note = props.revoke_error.clone();
            sections.push(view! { p(class = "gov-note bad") { note } });
        }
        sections.push(body);

        let main_content = if capabilities_missing {
            gov_manifest_pending("正在读取运行时能力清单…", false)
        } else if !can_read {
            gov_unsupported(
                "permissions.grants.read",
                read_reason.as_deref(),
                "授权账本需要 canonical gateway 的 grants 内省面。",
            )
        } else {
            VNode::fragment(sections.clone())
        };

        view! {
            div(class = "gov-tab", style = "--gov-accent: var(--ap-register-deepops-green)") {
                p(class = "gov-position") {
                    "授权是「他获得的能力」的账。撤销即刻生效、作用于当前进程；进程重启后按默认策略恢复。"
                }
                " "
                { main_content }
            }
        }
    }
}

// ── §11 审计 Tab ───────────────────────────────────────────────────────────

/// 审计数据源。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AuditSource {
    /// 执行轨迹。
    #[default]
    Traces,
    /// 事件留痕。
    Events,
}

/// 审计 Tab 的 props。
#[derive(Clone)]
pub struct AuditTabProps {
    /// 能力清单（`None` ⇒ 清单未到 ⇒ 等待态）。
    pub capabilities: Option<CapabilityManifest>,
    /// 当前数据源（宿主持有）。
    pub source: Signal<AuditSource>,
    /// 轨迹行（宿主注入）。
    pub trace_rows: Vec<TraceRow>,
    /// 轨迹拉取错误。
    pub trace_error: String,
    /// 轨迹过滤档（宿主持有）。
    pub trace_filter: Signal<TraceFilter>,
    /// 选中的轨迹 id。
    pub selected_trace_id: Option<String>,
    /// 选中的轨迹的 span 树（宿主注入）。
    pub trace_spans: Vec<TraceSpanItem>,
    /// 轨迹详情加载态。
    pub trace_detail_loading: bool,
    /// 轨迹详情错误。
    pub trace_detail_error: String,
    /// 事件留痕（宿主注入）。
    pub audit_items: Vec<ActivityItem>,
    /// 留痕拉取错误。
    pub audit_error: String,
    /// 留痕分组（宿主持有）。
    pub audit_group: Signal<String>,
    /// 选中的留痕 id。
    pub selected_audit_id: Option<String>,
    /// 加载态。
    pub loading: bool,
    /// 当前时刻（epoch 毫秒）。
    pub now_ms: i64,
    /// 切换数据源。
    pub on_source: Option<AuditSourceHandler>,
    /// 切换轨迹过滤档。
    pub on_trace_filter: Option<AuditTraceFilterHandler>,
    /// 切换留痕分组。
    pub on_audit_group: Option<AuditGroupHandler>,
    /// 选中一条轨迹。
    pub on_select_trace: Option<AuditSelectHandler>,
    /// 选中一条留痕。
    pub on_select_audit: Option<AuditSelectHandler>,
}

impl std::fmt::Debug for AuditTabProps {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AuditTabProps")
            .field("trace_rows", &self.trace_rows.len())
            .field("audit_items", &self.audit_items.len())
            .field("trace_spans", &self.trace_spans.len())
            .field("loading", &self.loading)
            .field("now_ms", &self.now_ms)
            .finish_non_exhaustive()
    }
}

/// 审计 Tab（生产 `AuditTab.svelte`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AuditTab;

impl UiComponent for AuditTab {
    type Props = AuditTabProps;
    type Output = VNode;

    fn render(props: &Self::Props) -> Self::Output {
        let manifest = props.capabilities.as_ref();
        let can_trace = capability_available(manifest, "trace.read");
        let can_audit = capability_available(manifest, "audit.read");
        let trace_reason = capability_unavailable_reason(manifest, "trace.read");
        let audit_reason = capability_unavailable_reason(manifest, "audit.read");

        let source = props.source.get();
        let trace_filter = props.trace_filter.get();
        let audit_group = props.audit_group.get();
        let is_traces = matches!(source, AuditSource::Traces);
        let capabilities_missing = props.capabilities.is_none();
        let now_ms = props.now_ms;

        // ── 数据源 chips ────────────────────────────────────────────────
        let trace_count_text = props.trace_rows.len().to_string();
        let audit_count_text = props.audit_items.len().to_string();
        let source_traces_on = is_traces;
        let source_events_on = !is_traces;
        let source_traces_chip = filter_chip(
            "执行轨迹 ",
            trace_count_text,
            source_traces_on,
            props.on_source.as_ref().map(|handler| {
                let handler = handler.clone();
                ElementHandler::new(move |_event| {
                    handler(AuditSource::Traces);
                })
            }),
        );
        let source_events_chip = filter_chip(
            "事件留痕 ",
            audit_count_text,
            source_events_on,
            props.on_source.as_ref().map(|handler| {
                let handler = handler.clone();
                ElementHandler::new(move |_event| {
                    handler(AuditSource::Events);
                })
            }),
        );
        let source_chips = view! {
            div(class = "gov-chips", role = "tablist", aria-label = "审计数据源") {
                { source_traces_chip }
                " "
                { source_events_chip }
            }
        };
        let toolbar = view! {
            div(class = "gov-toolbar") {
                { source_chips }
                " "
                { refresh_button(props.loading, props.loading, None) }
            }
        };

        // ── 二级过滤 chips ──────────────────────────────────────────────
        let trace_error_count = props.trace_rows.iter().filter(|row| row.has_error).count();
        let trace_error_count_text = trace_error_count.to_string();
        let trace_count_all_text = props.trace_rows.len().to_string();
        let trace_filter_all_on = matches!(trace_filter, TraceFilter::All);
        let trace_filter_error_on = matches!(trace_filter, TraceFilter::Error);
        let trace_all_chip = sub_filter_chip(
            "全部 ",
            trace_count_all_text,
            trace_filter_all_on,
            props.on_trace_filter.as_ref().map(|handler| {
                let handler = handler.clone();
                ElementHandler::new(move |_event| {
                    handler(TraceFilter::All);
                })
            }),
        );
        let trace_error_chip = sub_filter_chip(
            "有异常 ",
            trace_error_count_text,
            trace_filter_error_on,
            props.on_trace_filter.as_ref().map(|handler| {
                let handler = handler.clone();
                ElementHandler::new(move |_event| {
                    handler(TraceFilter::Error);
                })
            }),
        );
        let trace_chips = view! {
            div(class = "gov-chips sub", role = "tablist", aria-label = "轨迹过滤") {
                { trace_all_chip }
                " "
                { trace_error_chip }
            }
        };
        let group_chips = audit_group_chips(&props.audit_items);
        let mut group_nodes: Vec<VNode> = Vec::new();
        for (group, count) in &group_chips {
            let group_text = format!("{group} ");
            let count_text = count.to_string();
            let group_on = audit_group == *group;
            let group_name = group.clone();
            let node = sub_filter_chip(
                &group_text,
                count_text,
                group_on,
                props.on_audit_group.as_ref().map(|handler| {
                    let handler = handler.clone();
                    let group_name = group_name.clone();
                    ElementHandler::new(move |_event| {
                        handler(&group_name);
                    })
                }),
            );
            group_nodes.push(node);
        }
        let audit_count_all_text = props.audit_items.len().to_string();
        let audit_group_all_on = audit_group == "all";
        let audit_all_chip = sub_filter_chip(
            "全部 ",
            audit_count_all_text,
            audit_group_all_on,
            props.on_audit_group.as_ref().map(|handler| {
                let handler = handler.clone();
                ElementHandler::new(move |_event| {
                    handler("all");
                })
            }),
        );
        let audit_chips = view! {
            div(class = "gov-chips sub", role = "tablist", aria-label = "留痕分组") {
                { audit_all_chip }
                " "
                for node in group_nodes.clone() {
                    { node }
                }
            }
        };

        // ── 左栏 ────────────────────────────────────────────────────────
        let visible_traces: Vec<&TraceRow> = match trace_filter {
            TraceFilter::All => props.trace_rows.iter().collect(),
            TraceFilter::Error => props
                .trace_rows
                .iter()
                .filter(|row| row.has_error)
                .collect(),
        };
        let visible_audit = filter_audit_items(&props.audit_items, &audit_group);
        let select_trace = props.on_select_trace.clone();
        let select_audit = props.on_select_audit.clone();
        let mut trace_rows_node: Vec<VNode> = Vec::new();
        for row in &visible_traces {
            let selected = props.selected_trace_id.as_deref() == Some(row.trace_id.as_str());
            let title = if row.root_summary.is_empty() {
                let kind = if row.root_kind.is_empty() {
                    String::from("trace")
                } else {
                    row.root_kind.clone()
                };
                format!("{} · {}", kind, short_id(&row.trace_id, 8))
            } else {
                row.root_summary.clone()
            };
            let badge = if row.has_error {
                status_badge(BadgeVariant::Danger, "error", BadgeSize::Small)
            } else {
                let label = if row.root_status.is_empty() {
                    String::from("ok")
                } else {
                    row.root_status.clone()
                };
                status_badge(BadgeVariant::Green, &label, BadgeSize::Small)
            };
            let row_sub = format!(
                "{} spans · {}",
                row.span_count,
                format_date_time_ms(row.started_at_ms, now_ms)
            );
            let row_id = row.trace_id.clone();
            let select_trace = select_trace.clone();
            let node = view! {
                button(
                    class = "md-row",
                    class:on = selected,
                    onclick = move |_| {
                        if let Some(handler) = select_trace.clone() {
                            handler(&row_id);
                        }
                    }
                ) {
                    span(class = "md-row-top") {
                        span(class = "md-row-title") { title }
                        " "
                        { badge }
                    }
                    " "
                    span(class = "md-row-sub mono") { row_sub }
                }
            };
            trace_rows_node.push(node);
        }
        let mut audit_rows_node: Vec<VNode> = Vec::new();
        for item in &visible_audit {
            let selected = props.selected_audit_id.as_deref() == Some(item.id.as_str());
            let badge = status_badge(BadgeVariant::Blue, &item.source, BadgeSize::Small);
            let title = item.title.clone();
            let row_sub = format_date_time_ms(item.timestamp, now_ms);
            let row_id = item.id.clone();
            let select_audit = select_audit.clone();
            let node = view! {
                button(
                    class = "md-row",
                    class:on = selected,
                    onclick = move |_| {
                        if let Some(handler) = select_audit.clone() {
                            handler(&row_id);
                        }
                    }
                ) {
                    span(class = "md-row-top") {
                        span(class = "md-row-title mono") { title }
                        " "
                        { badge }
                    }
                    " "
                    span(class = "md-row-sub mono") { row_sub }
                }
            };
            audit_rows_node.push(node);
        }

        let master = if props.loading && props.trace_rows.is_empty() && props.audit_items.is_empty()
        {
            loading_state("正在读取审计账…")
        } else if is_traces && !props.trace_error.is_empty() && props.trace_rows.is_empty() {
            error_state("拉取轨迹失败", &props.trace_error, None)
        } else if !is_traces && !props.audit_error.is_empty() && props.audit_items.is_empty() {
            error_state("拉取留痕失败", &props.audit_error, None)
        } else if is_traces && visible_traces.is_empty() {
            let title = if matches!(trace_filter, TraceFilter::Error) {
                "没有带异常的轨迹"
            } else {
                "暂无执行轨迹"
            };
            empty_state(
                "⌘",
                title,
                Some("当一轮对话或一次工具调用完成时，它的完整执行轨迹（回合 → provider → 工具 → 治理）会记在这里。"),
                None,
            )
        } else if !is_traces && visible_audit.is_empty() {
            let title = if audit_group == "all" {
                String::from("暂无事件留痕")
            } else {
                format!("没有 {audit_group} 组留痕")
            };
            empty_state(
                "⌘",
                &title,
                Some("当回合完成、审批落笔、记忆变动时，运行时审计端口会把事件归档在这里。"),
                None,
            )
        } else if is_traces {
            VNode::fragment(trace_rows_node.clone())
        } else {
            VNode::fragment(audit_rows_node.clone())
        };

        // ── 右栏（轨迹详情） ────────────────────────────────────────────
        let selected_trace_id = props.selected_trace_id.clone();
        let mut span_nodes: Vec<VNode> = Vec::new();
        for span in &props.trace_spans {
            let (variant, label) = span_status_badge(&span.status);
            let badge = status_badge(variant, &label, BadgeSize::Small);
            let kind = span.kind.clone();
            let actor = span.actor.clone();
            let dur_text = match span_duration_ms(span) {
                None => String::from("进行中"),
                Some(value) => format_duration_ms(value),
            };
            let depth = span_depth(&props.trace_spans, span);
            let padding = format!("padding-left: {}px;", depth * 18);
            let summary = span.summary.clone();
            let has_summary = summary.is_some();
            let summary_text = summary.unwrap_or_default();
            let summary_padding = format!("padding-left: {}px;", depth * 18);
            let node = view! {
                div(class = "span-row", style = padding) {
                    span(class = "span-kind mono") { kind }
                    " "
                    span(class = "span-actor") { actor }
                    " "
                    { badge }
                    " "
                    span(class = "span-dur mono") { dur_text }
                }
                " "
                if has_summary {
                    div(class = "span-summary", style = summary_padding) { summary_text }
                } else { }
            };
            span_nodes.push(node);
        }
        let span_head_text = match &selected_trace_id {
            Some(id) => format!(
                "trace {} · {} spans",
                short_id(id, 12),
                props.trace_spans.len()
            ),
            None => String::new(),
        };
        let trace_detail = if selected_trace_id.is_none() {
            view! {
                div(class = "md-placeholder") {
                    p { "选择左侧一条轨迹，这里会出现它的完整 span 树——每一步的参与者、状态、耗时。" }
                }
            }
        } else if props.trace_detail_loading {
            loading_state("正在展开轨迹…")
        } else if !props.trace_detail_error.is_empty() {
            error_state("轨迹详情加载失败", &props.trace_detail_error, None)
        } else if props.trace_spans.is_empty() {
            view! {
                div(class = "md-placeholder") {
                    p { "该轨迹无 span 记录。" }
                }
            }
        } else {
            view! {
                div(class = "span-head mono") { span_head_text }
                " "
                div(class = "span-tree") {
                    for node in span_nodes.clone() {
                        { node }
                    }
                }
            }
        };

        // ── 右栏（留痕详情） ────────────────────────────────────────────
        let selected_audit = props
            .audit_items
            .iter()
            .find(|item| Some(item.id.as_str()) == props.selected_audit_id.as_deref());
        let audit_detail = match selected_audit {
            None => view! {
                div(class = "md-placeholder") {
                    p { "选择左侧一条留痕，这里会出现它的完整字段与原文。" }
                }
            },
            Some(item) => {
                let title = item.title.clone();
                let timestamp = format_date_time_ms(item.timestamp, now_ms);
                let show_summary = !item.summary.is_empty() && item.summary != item.title;
                let audit_summary = item.summary.clone();
                let show_detail = item.detail.is_some();
                let detail = item.detail.clone().unwrap_or_default();
                view! {
                    div(class = "audit-detail") {
                        div(class = "ad-row") {
                            span(class = "ad-lbl") { "事件" }
                            " "
                            span(class = "ad-val mono") { title }
                        }
                        " "
                        div(class = "ad-row") {
                            span(class = "ad-lbl") { "时刻" }
                            " "
                            span(class = "ad-val mono") { timestamp }
                        }
                        " "
                        if show_summary {
                            div(class = "ad-row") {
                                span(class = "ad-lbl") { "摘要" }
                                " "
                                span(class = "ad-val") { audit_summary }
                            }
                        } else { }
                        " "
                        if show_detail {
                            div(class = "ad-row col") {
                                span(class = "ad-lbl") { "原始记录" }
                                " "
                                pre(class = "ad-pre mono") { detail }
                            }
                        } else { }
                    }
                }
            }
        };

        let detail = if is_traces {
            trace_detail
        } else {
            audit_detail
        };

        let split = view! {
            div(class = "md-split") {
                div(class = "md-master") {
                    { master }
                }
                " "
                div(class = "md-detail") {
                    { detail }
                }
            }
        };

        let body = if is_traces && !can_trace {
            gov_unsupported("trace.read", trace_reason.as_deref(), "")
        } else if !is_traces && !can_audit {
            gov_unsupported("audit.read", audit_reason.as_deref(), "")
        } else {
            let mut parts: Vec<VNode> = vec![toolbar];
            if is_traces {
                parts.push(trace_chips);
            } else {
                parts.push(audit_chips);
            }
            parts.push(split);
            VNode::fragment(parts)
        };

        let main_content = if capabilities_missing {
            gov_manifest_pending("正在读取运行时能力清单…", false)
        } else if !can_trace && !can_audit {
            gov_unsupported(
                "trace.read / audit.read",
                trace_reason.as_deref().or(audit_reason.as_deref()),
                "审计卷宗需要 canonical gateway 的 trace/audit 内省面。",
            )
        } else {
            body
        };

        view! {
            div(class = "gov-tab", style = "--gov-accent: var(--ap-register-deepops-yellow)") {
                p(class = "gov-position") {
                    "审计是落笔之后的卷宗：每一轮执行的轨迹树、每一条系统留痕，都可在此检索复核。"
                }
                " "
                { main_content }
            }
        }
    }
}

// ── §12 守卫 Tab ───────────────────────────────────────────────────────────

/// 守卫 Tab 的 props。
#[derive(Clone)]
pub struct GuardTabProps {
    /// 能力清单（`None` ⇒ 清单未到 ⇒ 等待态）。
    pub capabilities: Option<CapabilityManifest>,
    /// 守卫状态（宿主注入）。
    pub status: Option<GuardStatus>,
    /// 守卫状态读取错误。
    pub status_error: String,
    /// 守卫事件（宿主注入）。
    pub events: Vec<GuardEvent>,
    /// 守卫事件拉取错误。
    pub events_error: String,
    /// 加载态。
    pub loading: bool,
    /// 已展开的事件 id（宿主持有）。
    pub expanded_ids: Signal<Vec<String>>,
    /// 试算结果（宿主注入的**求值结果**；引擎不发起调用）。
    pub dry_result: Option<GuardDryRunResult>,
    /// 试算错误。
    pub dry_error: String,
    /// 当前时刻（epoch 毫秒）。
    pub now_ms: i64,
    /// 展开 / 收起一条事件的理由与证据。
    pub on_toggle_event: Option<GuardEventHandler>,
}

impl std::fmt::Debug for GuardTabProps {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GuardTabProps")
            .field("status", &self.status)
            .field("events", &self.events.len())
            .field("loading", &self.loading)
            .field("now_ms", &self.now_ms)
            .finish_non_exhaustive()
    }
}

/// 守卫 Tab（生产 `GuardTab.svelte`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GuardTab;

impl UiComponent for GuardTab {
    type Props = GuardTabProps;
    type Output = VNode;

    fn render(props: &Self::Props) -> Self::Output {
        let manifest = props.capabilities.as_ref();
        let can_status = capability_available(manifest, "safety.guard.status.read");
        let can_events = capability_available(manifest, "safety.guard.events.read");
        let can_evaluate = capability_available(manifest, "safety.guard.evaluate");
        let status_reason = capability_unavailable_reason(manifest, "safety.guard.status.read");
        let events_reason = capability_unavailable_reason(manifest, "safety.guard.events.read");
        let evaluate_reason = capability_unavailable_reason(manifest, "safety.guard.evaluate");
        let any_guard = can_status || can_events || can_evaluate;

        let capabilities_missing = props.capabilities.is_none();
        let expanded_ids = props.expanded_ids.get();

        // ── 状态条 ──────────────────────────────────────────────────────
        let status_section = match &props.status {
            Some(status) => {
                let enabled_badge = if status.enabled {
                    status_badge(BadgeVariant::Green, "守卫启用", BadgeSize::Normal)
                } else {
                    status_badge(BadgeVariant::Dim, "守卫停用", BadgeSize::Normal)
                };
                let fast_badge = if status.fast_guard_active {
                    status_badge(BadgeVariant::Blue, "快速守卫", BadgeSize::Small)
                } else {
                    status_badge(BadgeVariant::Dim, "快速守卫", BadgeSize::Small)
                };
                let chain_badge = if status.chain_guard_active {
                    status_badge(BadgeVariant::Blue, "链式守卫", BadgeSize::Small)
                } else {
                    status_badge(BadgeVariant::Dim, "链式守卫", BadgeSize::Small)
                };
                let intent_badge = if status.intent_guard_active {
                    Some(status_badge(
                        BadgeVariant::Blue,
                        "意图守卫",
                        BadgeSize::Small,
                    ))
                } else {
                    None
                };
                let cross_badge = if status.cross_turn_monitoring_active {
                    Some(status_badge(
                        BadgeVariant::Blue,
                        "跨轮监测",
                        BadgeSize::Small,
                    ))
                } else {
                    None
                };
                let ml_label = if status.ml_classifier_available {
                    format!(
                        "ML 分类器 {}",
                        status.ml_model_version.clone().unwrap_or_default()
                    )
                    .trim()
                    .to_owned()
                } else {
                    String::from("ML 分类器不可用")
                };
                let ml_badge = if status.ml_classifier_available {
                    status_badge(BadgeVariant::Green, &ml_label, BadgeSize::Small)
                } else {
                    status_badge(BadgeVariant::Dim, &ml_label, BadgeSize::Small)
                };
                let has_intent = intent_badge.is_some();
                let has_cross = cross_badge.is_some();
                let intent_node = intent_badge.unwrap_or_else(|| VNode::fragment(Vec::new()));
                let cross_node = cross_badge.unwrap_or_else(|| VNode::fragment(Vec::new()));
                let evaluations = status.total_evaluations.to_string();
                let allowed = status.total_allowed.to_string();
                let denied = status.total_denied.to_string();
                let approval_required = status.total_approval_required.to_string();
                let denied_bad = status.total_denied > 0;
                let approval_warn = status.total_approval_required > 0;
                view! {
                    div(class = "guard-status") {
                        div(class = "gs-row") {
                            { enabled_badge }
                            " "
                            { fast_badge }
                            " "
                            { chain_badge }
                            " "
                            if has_intent {
                                { intent_node }
                            } else { }
                            " "
                            if has_cross {
                                { cross_node }
                            } else { }
                            " "
                            { ml_badge }
                        }
                        " "
                        div(class = "gs-tally") {
                            div(class = "tally-cell") {
                                span(class = "tally-num mono") { evaluations }
                                " "
                                span(class = "tally-lbl") { "评估" }
                            }
                            " "
                            div(class = "tally-cell") {
                                span(class = "tally-num mono ok") { allowed }
                                " "
                                span(class = "tally-lbl") { "放行" }
                            }
                            " "
                            div(class = "tally-cell") {
                                span(class = "tally-num mono", class:bad = denied_bad) { denied }
                                " "
                                span(class = "tally-lbl") { "否决" }
                            }
                            " "
                            div(class = "tally-cell") {
                                span(class = "tally-num mono", class:warn = approval_warn) { approval_required }
                                " "
                                span(class = "tally-lbl") { "转人工" }
                            }
                        }
                    }
                }
            }
            None => {
                if !props.status_error.is_empty() {
                    let message = format!("守卫状态读取失败：{}", props.status_error);
                    view! { p(class = "gov-note bad") { message } }
                } else {
                    loading_state("正在读取守卫状态…")
                }
            }
        };

        // ── dry-run 试算卡 ──────────────────────────────────────────────
        let flask_icon = Icon::render(&IconProps::new(&ICON_FLASK_CONICAL, 13));
        let dry_result_node = props.dry_result.as_ref().map(|result| {
            let (variant, label) = guard_decision_badge(&result.decision);
            let badge = status_badge(variant, &label, BadgeSize::Normal);
            let stage = guard_stage_label(&result.stage);
            let risk = risk_score_percent(result.risk_score);
            let risk_text = format!("风险 {risk}%");
            let has_reasons = !result.reasons.is_empty();
            let mut reason_nodes: Vec<VNode> = Vec::new();
            for reason in &result.reasons {
                let text = reason.clone();
                let node = view! { li { text } };
                reason_nodes.push(node);
            }
            let has_evidence = !result.evidence.is_empty();
            let mut evidence_nodes: Vec<VNode> = Vec::new();
            for item in &result.evidence {
                let text = item.clone();
                let node = view! { code(class = "dr-ev-item mono") { text } };
                evidence_nodes.push(node);
            }
            let evidence_count = result.evidence.len().to_string();
            let summary_text = format!("证据 {evidence_count} 条");
            view! {
                div(class = "dry-result") {
                    div(class = "dr-row") {
                        { badge }
                        " "
                        span(class = "dr-stage") { stage }
                        " "
                        span(class = "dr-risk mono") { risk_text }
                    }
                    " "
                    if has_reasons {
                        ul(class = "dr-list") {
                            for node in reason_nodes.clone() {
                                { node }
                            }
                        }
                    } else { }
                    " "
                    if has_evidence {
                        details(class = "dr-evidence") {
                            summary { summary_text }
                            " "
                            for node in evidence_nodes.clone() {
                                { node }
                            }
                        }
                    } else { }
                }
            }
        });
        let has_dry_error = !props.dry_error.is_empty();
        let dry_error_text = props.dry_error.clone();
        let dry_card = view! {
            section(class = "dry-card") {
                header(class = "dry-head") {
                    { flask_icon }
                    " "
                    span(class = "dry-title") { "试算一道调用" }
                    " "
                    span(class = "dry-tag mono") { "DRY-RUN · 只评估不执行" }
                }
                " "
                div(class = "dry-form") {
                    label(class = "dry-field") {
                        span(class = "dry-lbl") { "能力 id" }
                        " "
                        input(class = "dry-input mono", type = "text", placeholder = "tool.repo")
                    }
                    " "
                    label(class = "dry-field") {
                        span(class = "dry-lbl") { "参数 JSON" }
                        " "
                        textarea(class = "dry-input mono", rows = "3")
                    }
                    " "
                    label(class = "dry-field") {
                        span(class = "dry-lbl") { "声明范围（可选）" }
                        " "
                        input(class = "dry-input mono", type = "text", placeholder = "repo:write")
                    }
                    " "
                    div(class = "dry-actions") {
                        button(class = "btn-eval") { "试算" }
                    }
                }
                " "
                if has_dry_error {
                    p(class = "gov-note bad") { dry_error_text }
                } else { }
                " "
                if dry_result_node.is_some() {
                    { dry_result_node.clone().unwrap_or_else(|| VNode::fragment(Vec::new())) }
                } else { }
            }
        };

        // ── 事件账 ──────────────────────────────────────────────────────
        let count_text = format!("最近评估 {} 条", props.events.len());
        let toolbar = view! {
            div(class = "gov-toolbar") {
                span(class = "gov-count mono") { count_text }
                " "
                { refresh_button(props.loading, props.loading, None) }
            }
        };
        let toggle_event = props.on_toggle_event.clone();
        let mut event_nodes: Vec<VNode> = Vec::new();
        for event in &props.events {
            let row = guard_event_row(event);
            let (variant, label) = &row.badge;
            let badge = status_badge(*variant, label, BadgeSize::Small);
            let stage = row.stage_label.clone();
            let capability = event.capability_id.clone();
            let session = format!("会话 {}", short_id(&event.session_id, 8));
            let round = format!("R{}", event.round);
            let clock = format_clock_ms(event.timestamp_ms);
            let risk_text = format!("{}%", row.risk_pct);
            let risk_high = row.risk_pct >= 70;
            let risk_mid = row.risk_pct >= 40 && row.risk_pct < 70;
            let risk_width = format!("width: {}%;", row.risk_pct);
            let has_detail = !event.reasons.is_empty() || !event.evidence.is_empty();
            let expanded = expanded_ids.contains(&row.id);
            let chevron = if expanded {
                Icon::render(&IconProps::new(&ICON_CHEVRON_DOWN, 12))
            } else {
                Icon::render(&IconProps::new(&ICON_CHEVRON_RIGHT, 12))
            };
            let mut reason_nodes: Vec<VNode> = Vec::new();
            for reason in &event.reasons {
                let text = reason.clone();
                let node = view! { p(class = "ev-reason") { text } };
                reason_nodes.push(node);
            }
            let mut evidence_nodes: Vec<VNode> = Vec::new();
            for item in &event.evidence {
                let text = item.clone();
                let node = view! { code(class = "ev-ev-item mono") { text } };
                evidence_nodes.push(node);
            }
            let session_title = event.session_id.clone();
            let event_id = row.id.clone();
            let toggle_event = toggle_event.clone();
            let node = view! {
                article(class = "gov-card") {
                    header(class = "gov-card-head") {
                        span(class = "gov-card-title mono") { capability }
                        " "
                        span(class = "ev-badges") {
                            { badge }
                            " "
                            span(class = "ev-stage") { stage }
                        }
                    }
                    " "
                    div(class = "ev-risk-row") {
                        span(class = "ev-risk-bar") {
                            span(class = "ev-risk-fill", class:high = risk_high, class:mid = risk_mid, style = risk_width)
                        }
                        " "
                        span(class = "mono ev-risk-num") { risk_text }
                    }
                    " "
                    footer(class = "gov-card-foot") {
                        span(class = "gov-meta mono", title = session_title) { session }
                        " "
                        span(class = "gov-meta mono") { round }
                        " "
                        span(class = "gov-meta mono") { clock }
                        " "
                        if has_detail {
                            button(
                                class = "ev-expand",
                                onclick = move |_| {
                                    if let Some(handler) = toggle_event.clone() {
                                        handler(&event_id);
                                    }
                                }
                            ) {
                                { chevron }
                                " 理由与证据"
                            }
                        } else { }
                    }
                    " "
                    if expanded {
                        div(class = "ev-detail") {
                            for node in reason_nodes.clone() {
                                { node }
                            }
                            " "
                            for node in evidence_nodes.clone() {
                                { node }
                            }
                        }
                    } else { }
                }
            };
            event_nodes.push(node);
        }
        let list = view! {
            div(class = "gov-list") {
                for node in event_nodes.clone() {
                    { node }
                }
            }
        };
        let events_section = if !can_events {
            gov_unsupported("safety.guard.events.read", events_reason.as_deref(), "")
        } else if props.loading && props.events.is_empty() && props.status.is_none() {
            loading_state("正在读取守卫事件…")
        } else if !props.events_error.is_empty() && props.events.is_empty() {
            error_state("拉取守卫事件失败", &props.events_error, None)
        } else if props.events.is_empty() {
            empty_state(
                "⛨",
                "守卫尚未拦下任何评估记录",
                Some("当他在回合里调用能力时，每一次守卫评估（放行 / 否决 / 转人工）都会记在这里——能力、阶段、风险分与理由。"),
                None,
            )
        } else {
            list
        };

        let mut sections: Vec<VNode> = Vec::new();
        if can_status {
            sections.push(status_section);
        } else {
            sections.push(gov_unsupported(
                "safety.guard.status.read",
                status_reason.as_deref(),
                "",
            ));
        }
        if can_evaluate {
            sections.push(dry_card);
        } else {
            sections.push(gov_unsupported(
                "safety.guard.evaluate",
                evaluate_reason.as_deref(),
                "试算面不可用。",
            ));
        }
        sections.push(toolbar);
        sections.push(events_section);

        let main_content = if capabilities_missing {
            gov_manifest_pending("正在读取运行时能力清单…", false)
        } else if !any_guard {
            gov_unsupported(
                "safety.guard.*",
                status_reason
                    .as_deref()
                    .or(events_reason.as_deref())
                    .or(evaluate_reason.as_deref()),
                "守卫内省面需要 canonical gateway 装配 BehaviorChainGuardHook。",
            )
        } else {
            VNode::fragment(sections.clone())
        };

        view! {
            div(class = "gov-tab", style = "--gov-accent: var(--ap-register-deepops-red)") {
                p(class = "gov-position") {
                    "守卫的账：每一次评估、放行、否决、转人工都记在这里。守卫的实时通报不进本页——否决当下在对话内呈现。"
                }
                " "
                { main_content }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //! 治理卷宗面组件的**行为**断言（不是源码镜像断言）。
    //!
    //! 断言从「读源码文本」升级为「钉行为」：虚拟树的 class / 结构 / 文本
    //! 逐条对齐生产组件（生产侧的真值在浏览器对照里逐帧复核，见
    //! `crates-ui/apeireth-ui-demo/harness/governance/`）。

    use std::fmt::Write as _;

    use super::*;
    use apeireth_ui_dom::{ElementVNode, TextVNode};

    /// 虚拟树 ⇒ 文本（结构 + class + 文本；供断言比对）。
    fn tree_to_text(node: &VNode) -> String {
        let mut out = String::new();
        walk(node, &mut out);
        out
    }

    /// 递归展开虚拟树。
    fn walk(node: &VNode, out: &mut String) {
        match node {
            VNode::Element(element) => {
                let classes = element
                    .attr_value("class")
                    .and_then(|value| value.to_attr_string())
                    .unwrap_or_default();
                let _ = write!(out, "<{}{}>", element.tag, class_suffix(&classes));
                for child in &element.children {
                    walk(child, out);
                }
                out.push_str(&format!("</{}>", element.tag));
            }
            VNode::Text(TextVNode { data }) => {
                if !data.trim().is_empty() {
                    out.push_str(data);
                }
            }
            VNode::Fragment(children) => {
                for child in children {
                    walk(child, out);
                }
            }
        }
    }

    /// class ⇒ 后缀（空 ⇒ 无）。
    fn class_suffix(classes: &str) -> String {
        if classes.is_empty() {
            String::new()
        } else {
            format!(".{classes}")
        }
    }

    /// 找出第一个 class 精确匹配的祖先链（断言用）。
    fn find_by_class(node: &VNode, class: &str) -> Option<String> {
        match node {
            VNode::Element(element) => {
                let classes = element
                    .attr_value("class")
                    .and_then(|value| value.to_attr_string())
                    .unwrap_or_default();
                if classes.split_whitespace().any(|item| item == class) {
                    return Some(element.tag.clone());
                }
                for child in &element.children {
                    if let Some(found) = find_by_class(child, class) {
                        return Some(found);
                    }
                }
                None
            }
            VNode::Text(_) => None,
            VNode::Fragment(children) => {
                for child in children {
                    if let Some(found) = find_by_class(child, class) {
                        return Some(found);
                    }
                }
                None
            }
        }
    }

    /// 找出第一个 class **精确包含**该 token 的元素。
    fn find_element<'a>(node: &'a VNode, class: &str) -> Option<&'a ElementVNode> {
        match node {
            VNode::Element(element) => {
                let classes = element
                    .attr_value("class")
                    .and_then(|value| value.to_attr_string())
                    .unwrap_or_default();
                if classes.split_whitespace().any(|item| item == class) {
                    return Some(element);
                }
                for child in &element.children {
                    if let Some(found) = find_element(child, class) {
                        return Some(found);
                    }
                }
                None
            }
            VNode::Text(_) => None,
            VNode::Fragment(children) => {
                for child in children {
                    if let Some(found) = find_element(child, class) {
                        return Some(found);
                    }
                }
                None
            }
        }
    }

    /// 元素的属性文本。
    fn element_attr(element: &ElementVNode, name: &str) -> Option<String> {
        element
            .attr_value(name)
            .and_then(|value| value.to_attr_string())
    }

    /// 统计 class 精确匹配的元素个数。
    fn count_by_class(node: &VNode, class: &str) -> usize {
        match node {
            VNode::Element(element) => {
                let classes = element
                    .attr_value("class")
                    .and_then(|value| value.to_attr_string())
                    .unwrap_or_default();
                let own = usize::from(classes.split_whitespace().any(|item| item == class));
                own + element
                    .children
                    .iter()
                    .map(|child| count_by_class(child, class))
                    .sum::<usize>()
            }
            VNode::Text(_) => 0,
            VNode::Fragment(children) => children
                .iter()
                .map(|child| count_by_class(child, class))
                .sum(),
        }
    }

    /// 取元素的属性文本。
    fn attr_text(node: &VNode, name: &str) -> Option<String> {
        match node {
            VNode::Element(element) => element
                .attr_value(name)
                .and_then(|value| value.to_attr_string()),
            _ => None,
        }
    }

    /// 造一条能力声明。
    fn capability(id: &str, supported: bool, available: Option<bool>) -> Capability {
        Capability {
            id: String::from(id),
            supported,
            available,
            reason: None,
            alias_of: None,
        }
    }

    /// 造一份能力清单。
    fn manifest(caps: Vec<Capability>) -> CapabilityManifest {
        CapabilityManifest {
            capabilities: vec![CapabilityGroup { capabilities: caps }],
        }
    }

    /// 造一条待签文书。
    fn approval(id: &str, session: &str) -> PendingApproval {
        PendingApproval {
            approval_id: String::from(id),
            session: String::from(session),
            tool_name: String::from("fs.write"),
            governance_reason: String::from("写工作区文件需要主人点头"),
            command_text: None,
            arguments_summary: None,
            created_at: None,
            expires_at: None,
        }
    }

    // ── §1 时刻与 id 格式化 ────────────────────────────────────────────

    #[test]
    fn clock_is_rendered_as_hh_mm_ss() {
        assert_eq!(format_clock_ms(0), "00:00:00");
        assert_eq!(format_clock_ms(1_792_834_200_000), "09:30:00");
        assert_eq!(format_clock_ms(-1_000), "23:59:59");
    }

    #[test]
    fn same_day_shows_only_the_clock() {
        let now = 1_792_834_200_000;
        assert_eq!(format_date_time_ms(now, now), "09:30");
    }

    #[test]
    fn cross_day_shows_month_day_and_clock() {
        let now = 1_792_834_200_000;
        // 前一天 ⇒ `MM/DD HH:MM`（zh-CN 的日期分隔符是 `/`）。
        assert_eq!(format_date_time_ms(now - 86_400_000, now), "10/23 09:30");
        // 跨年。
        assert_eq!(
            format_date_time_ms(1_792_834_200_000 - 30 * 86_400_000, now),
            "09/24 09:30"
        );
    }

    #[test]
    fn relative_time_follows_the_script_side_rules() {
        let now = 1_000_000;
        assert_eq!(format_relative_ms(now + 5_000, now), "刚刚");
        assert_eq!(format_relative_ms(now - 1_000, now), "刚刚");
        assert_eq!(format_relative_ms(now - 4_000, now), "刚刚");
        assert_eq!(format_relative_ms(now - 30_000, now), "30 秒前");
        assert_eq!(format_relative_ms(now - 90_000, now), "1 分钟前");
        assert_eq!(format_relative_ms(now - 3_600_000, now), "1 小时前");
        assert_eq!(format_relative_ms(now - 86_400_000, now), "1 天前");
    }

    #[test]
    fn short_id_truncates_by_utf16_code_units() {
        assert_eq!(short_id("abcdef", 8), "abcdef");
        assert_eq!(short_id("abcdefghij", 8), "abcdefgh…");
        // 非 BMP 字符按 UTF-16 码元切（代理对算两个）。
        assert_eq!(short_id("😀😀😀😀😀", 2), "😀…");
    }

    #[test]
    fn duration_switches_to_seconds_above_one_thousand() {
        assert_eq!(format_duration_ms(86.0), "86ms");
        assert_eq!(format_duration_ms(999.0), "999ms");
        assert_eq!(format_duration_ms(1000.0), "1.00s");
        assert_eq!(format_duration_ms(1234.0), "1.23s");
    }

    // ── §2 RFC 3339 ────────────────────────────────────────────────────

    #[test]
    fn rfc3339_utc_is_parsed_to_epoch_milliseconds() {
        assert_eq!(parse_rfc3339_ms("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(
            parse_rfc3339_ms("2026-10-24T09:30:00Z"),
            Some(1_792_834_200_000)
        );
        assert_eq!(
            parse_rfc3339_ms("2026-10-24T09:30:00.250Z"),
            Some(1_792_834_200_250)
        );
    }

    #[test]
    fn rfc3339_offsets_are_applied() {
        let utc = parse_rfc3339_ms("2026-10-24T09:30:00Z").unwrap();
        assert_eq!(parse_rfc3339_ms("2026-10-24T11:30:00+02:00"), Some(utc));
        assert_eq!(parse_rfc3339_ms("2026-10-24T07:30:00-02:00"), Some(utc));
    }

    #[test]
    fn malformed_rfc3339_is_rejected() {
        assert_eq!(parse_rfc3339_ms(""), None);
        assert_eq!(parse_rfc3339_ms("2026-10-24"), None);
        assert_eq!(parse_rfc3339_ms("2026-13-24T09:30:00Z"), None);
        assert_eq!(parse_rfc3339_ms("2026-10-24T25:30:00Z"), None);
        assert_eq!(parse_rfc3339_ms("2026-10-24T09:30:00Z trailing"), None);
    }

    #[test]
    fn rfc3339_formatting_round_trips() {
        for ts in [0, 1_792_834_200_000, 1_767_225_615_250] {
            let text = format_rfc3339_ms(ts);
            assert_eq!(parse_rfc3339_ms(&text), Some(ts), "实测文本：{text}");
        }
        assert_eq!(format_rfc3339_ms(0), "1970-01-01T00:00:00.000Z");
    }

    // ── §3 能力门控 ────────────────────────────────────────────────────

    #[test]
    fn capability_gating_follows_the_script_side_rules() {
        let list = manifest(vec![
            capability("permissions.approval.read", true, Some(true)),
            capability("permissions.revoke", false, Some(false)),
        ]);
        assert!(capability_available(
            Some(&list),
            "permissions.approval.read"
        ));
        assert!(!capability_available(Some(&list), "permissions.revoke"));
        assert!(!capability_available(None, "permissions.approval.read"));
        assert!(!capability_available(Some(&list), "unknown.capability"));
    }

    #[test]
    fn old_manifests_fall_back_to_supported() {
        let list = manifest(vec![capability("trace.read", true, None)]);
        assert!(capability_available(Some(&list), "trace.read"));
        // 旧清单没有 `available` ⇒ 不可用原因为 `None`。
        assert_eq!(
            capability_unavailable_reason(Some(&list), "trace.read"),
            None
        );
    }

    #[test]
    fn capability_aliases_are_canonicalized() {
        let list = manifest(vec![capability(
            "permissions.approval.read",
            true,
            Some(true),
        )]);
        assert!(capability_available(Some(&list), "approvals.read"));
        assert!(!capability_available(Some(&list), "approvals.resolve"));
    }

    #[test]
    fn capability_reason_is_only_reported_when_unavailable() {
        let mut cap = capability("audit.read", true, Some(false));
        cap.reason = Some(String::from("not_implemented"));
        let list = manifest(vec![cap]);
        assert_eq!(
            capability_unavailable_reason(Some(&list), "audit.read"),
            Some(String::from("not_implemented"))
        );
    }

    // ── §5 账本 ────────────────────────────────────────────────────────

    #[test]
    fn expired_ledger_entries_are_counted_and_filtered() {
        let mut open = ApprovalLedgerEntry {
            approval: approval("ap-1", "sess"),
            expires_at_ms: Some(2_000),
            ..Default::default()
        };
        open.expires_at_ms = Some(2_000);
        let expired = ApprovalLedgerEntry {
            approval: approval("ap-2", "sess"),
            expires_at_ms: Some(1_000),
            ..Default::default()
        };
        let never = ApprovalLedgerEntry {
            approval: approval("ap-3", "sess"),
            ..Default::default()
        };
        let ledger = vec![open, expired, never];
        let (all, open_count, expired_count) = approval_filter_counts(&ledger, 1_500);
        assert_eq!((all, open_count, expired_count), (3, 2, 1));
        assert_eq!(
            filter_approval_ledger(&ledger, ApprovalFilter::All, 1_500).len(),
            3
        );
        assert_eq!(
            filter_approval_ledger(&ledger, ApprovalFilter::Open, 1_500).len(),
            2
        );
        assert_eq!(
            filter_approval_ledger(&ledger, ApprovalFilter::Expired, 1_500).len(),
            1
        );
    }

    #[test]
    fn merging_an_inbox_replaces_that_session_and_sorts_by_time_desc() {
        let old = ApprovalLedgerEntry {
            approval: approval("ap-old", "sess-a"),
            created_at_ms: Some(10),
            ..Default::default()
        };
        let fresh = vec![
            PendingApproval {
                created_at: Some(String::from("2026-10-24T09:00:00Z")),
                ..approval("ap-new", "sess-a")
            },
            PendingApproval {
                created_at: Some(String::from("2026-10-24T10:00:00Z")),
                ..approval("ap-newest", "sess-a")
            },
        ];
        let merged = merge_session_inbox(&[old], "sess-a", &fresh, Some("会话标题"));
        assert_eq!(merged.len(), 2, "同会话的旧条目整体被替换");
        assert_eq!(merged[0].approval.approval_id, "ap-newest");
        assert_eq!(merged[1].approval.approval_id, "ap-new");
        assert_eq!(merged[0].session_title.as_deref(), Some("会话标题"));
        assert!(merged[0].created_at_ms.is_some());
    }

    // ── §6 轨迹 ────────────────────────────────────────────────────────

    #[test]
    fn span_depth_walks_the_parent_chain_and_caps_at_six() {
        let mut spans: Vec<TraceSpanItem> = (0..8)
            .map(|index| TraceSpanItem {
                span_id: format!("span-{index}"),
                parent_span_id: if index == 0 {
                    None
                } else {
                    Some(format!("span-{}", index - 1))
                },
                ..Default::default()
            })
            .collect();
        spans[0].parent_span_id = None;
        assert_eq!(span_depth(&spans, &spans[1]), 1);
        assert_eq!(span_depth(&spans, &spans[7]), 6, "上限 6 层");
    }

    #[test]
    fn span_duration_is_none_while_running() {
        let mut span = TraceSpanItem {
            started_at: 100.0,
            ..Default::default()
        };
        assert_eq!(span_duration_ms(&span), None);
        span.ended_at = Some(350.0);
        assert_eq!(span_duration_ms(&span), Some(250.0));
    }

    #[test]
    fn span_status_badge_maps_to_the_five_colors() {
        assert_eq!(span_status_badge("ok").0, BadgeVariant::Green);
        assert_eq!(span_status_badge("ERROR").0, BadgeVariant::Danger);
        assert_eq!(span_status_badge("").0, BadgeVariant::Dim);
        assert_eq!(span_status_badge("pending").0, BadgeVariant::Blue);
        assert_eq!(span_status_badge("").1, "—");
    }

    #[test]
    fn audit_groups_are_counted_and_sorted_by_count() {
        let items = vec![
            ActivityItem {
                title: String::from("chat.turn.completed"),
                ..Default::default()
            },
            ActivityItem {
                title: String::from("chat.turn.completed"),
                ..Default::default()
            },
            ActivityItem {
                title: String::from("memory.write"),
                ..Default::default()
            },
        ];
        let chips = audit_group_chips(&items);
        assert_eq!(chips[0], (String::from("chat"), 2));
        assert_eq!(chips[1], (String::from("memory"), 1));
        assert_eq!(filter_audit_items(&items, "all").len(), 3);
        assert_eq!(filter_audit_items(&items, "chat").len(), 2);
    }

    // ── §7 守卫 ────────────────────────────────────────────────────────

    #[test]
    fn guard_decision_badge_maps_every_known_decision() {
        assert_eq!(guard_decision_badge("allow").0, BadgeVariant::Green);
        assert_eq!(guard_decision_badge("allowed").1, "放行");
        assert_eq!(guard_decision_badge("deny").1, "否决");
        assert_eq!(guard_decision_badge("blocked").0, BadgeVariant::Danger);
        assert_eq!(guard_decision_badge("approval_required").1, "待签");
        assert_eq!(guard_decision_badge("warning").0, BadgeVariant::Amber);
        assert_eq!(guard_decision_badge("").1, "未知");
        assert_eq!(guard_decision_badge("custom").0, BadgeVariant::Blue);
    }

    #[test]
    fn guard_stage_labels_are_translated() {
        assert_eq!(guard_stage_label("fast_guard"), "快速守卫");
        assert_eq!(guard_stage_label("chain_guard"), "链式守卫");
        assert_eq!(guard_stage_label("decision_fusion"), "决策融合");
        assert_eq!(guard_stage_label("other"), "other");
    }

    #[test]
    fn risk_score_is_clamped_and_rounded() {
        assert_eq!(risk_score_percent(0.0), 0);
        assert_eq!(risk_score_percent(0.725), 73);
        assert_eq!(risk_score_percent(1.0), 100);
        assert_eq!(risk_score_percent(1.5), 100, "越界钳制");
        assert_eq!(risk_score_percent(-0.5), 0);
        assert_eq!(risk_score_percent(f64::NAN), 0);
    }

    #[test]
    fn guard_event_row_id_is_trace_round_timestamp() {
        let event = GuardEvent {
            trace_id: String::from("trace-1"),
            round: 3,
            timestamp_ms: 42,
            ..Default::default()
        };
        assert_eq!(guard_event_row(&event).id, "trace-1-3-42");
    }

    // ── §4 共享子件 ────────────────────────────────────────────────────

    #[test]
    fn status_badge_reuses_the_badge_component() {
        let node = status_badge(BadgeVariant::Amber, "待签", BadgeSize::Small);
        assert_eq!(
            attr_text(&node, "class").as_deref(),
            Some("status-badge amber small")
        );
        assert!(tree_to_text(&node).contains("待签"));
    }

    #[test]
    fn empty_state_renders_icon_title_and_optional_parts() {
        let node = empty_state("✒", "账上无待签", None, None);
        let text = tree_to_text(&node);
        assert!(text.contains("<div.empty-state-wrap>"));
        assert!(text.contains("<div.empty-mark>✒</div>"));
        assert!(text.contains("<h3.empty-title>账上无待签</h3>"));
        assert!(!text.contains("empty-desc"));
        assert!(!text.contains("empty-actions"));
    }

    #[test]
    fn empty_state_renders_description_and_slot() {
        let node = empty_state(
            "✒",
            "标题",
            Some("描述"),
            Some(VNode::text(String::from("插槽"))),
        );
        let text = tree_to_text(&node);
        assert!(text.contains("<p.empty-desc>描述</p>"));
        assert!(text.contains("<div.empty-actions>插槽</div>"));
    }

    #[test]
    fn error_state_always_renders_the_retry_button() {
        let node = error_state("拉取失败", "网关不可达", None);
        let text = tree_to_text(&node);
        assert!(text.contains("<h3.error-title>拉取失败</h3>"));
        assert!(text.contains("<p.error-message>网关不可达</p>"));
        assert!(text.contains("<button.retry-btn>"));
        assert!(text.contains("<span>重试</span>"));
    }

    #[test]
    fn loading_state_carries_the_spinner_class() {
        let node = loading_state("正在读取…");
        let text = tree_to_text(&node);
        assert!(text.contains("<div.loading-state-wrap"));
        assert!(text.contains("spin-icon"));
        assert!(text.contains("<p.loading-text>正在读取…</p>"));
    }

    #[test]
    fn manifest_pending_switches_to_the_honest_note_after_the_budget() {
        let waiting = gov_manifest_pending("正在读取运行时能力清单…", false);
        assert!(tree_to_text(&waiting).contains("loading-state-wrap"));
        let late = gov_manifest_pending("正在读取运行时能力清单…", true);
        let text = tree_to_text(&late);
        assert!(text.contains("<div.manifest-pending"));
        assert!(text.contains("运行时能力清单迟迟未到"));
    }

    #[test]
    fn unsupported_names_the_capability_and_the_reason() {
        let node = gov_unsupported("trace.read", Some("not_implemented"), "提示");
        let text = tree_to_text(&node);
        assert!(text.contains("<div.gov-unsupported"));
        assert!(text.contains("<code>trace.read</code>"));
        assert!(text.contains("（原因 <code>not_implemented</code>）"));
        assert!(text.contains("提示"));
    }

    #[test]
    fn unsupported_omits_the_reason_when_absent() {
        let node = gov_unsupported("trace.read", None, "");
        let text = tree_to_text(&node);
        assert!(!text.contains("（原因"));
    }

    // ── §9 审批收件箱 Tab ──────────────────────────────────────────────

    /// 造一份审批面的 props。
    fn approvals_props(caps: Option<CapabilityManifest>) -> ApprovalsTabProps {
        let runtime = apeireth_ui_core::Runtime::new();
        ApprovalsTabProps {
            capabilities: caps,
            ledger: Vec::new(),
            loading: false,
            error: String::new(),
            filter: runtime.signal(ApprovalFilter::All),
            sse_state: SseConnectionState::Connecting,
            now_ms: 1_792_834_200_000,
            resolving_id: None,
            resolve_note: String::new(),
            resolve_error: String::new(),
            on_open_chat: None,
            on_go_audit: None,
            on_resolve: None,
            on_filter: None,
        }
    }

    #[test]
    fn approvals_tab_renders_the_position_line_and_the_account_color() {
        let node = ApprovalsTab::render(&approvals_props(None));
        assert_eq!(
            attr_text(&node, "style").as_deref(),
            Some("--gov-accent: var(--ap-register-deepops-blue)")
        );
        assert!(tree_to_text(&node).contains("审批在对话内完成"));
    }

    #[test]
    fn approvals_tab_waits_for_the_capability_manifest() {
        let node = ApprovalsTab::render(&approvals_props(None));
        assert_eq!(
            find_by_class(&node, "loading-state-wrap"),
            Some("div".into())
        );
        assert_eq!(count_by_class(&node, "manifest-pending"), 0);
    }

    #[test]
    fn approvals_tab_marks_an_unsupported_capability() {
        let caps = manifest(vec![capability(
            "permissions.approval.read",
            false,
            Some(false),
        )]);
        let node = ApprovalsTab::render(&approvals_props(Some(caps)));
        assert_eq!(find_by_class(&node, "gov-unsupported"), Some("div".into()));
        let text = tree_to_text(&node);
        assert!(text.contains("<code>permissions.approval.read</code>"));
    }

    #[test]
    fn approvals_tab_renders_the_three_filter_chips_with_counts() {
        let caps = manifest(vec![capability(
            "permissions.approval.read",
            true,
            Some(true),
        )]);
        let mut props = approvals_props(Some(caps));
        props.ledger = vec![ApprovalLedgerEntry {
            approval: approval("ap-1", "sess-a"),
            expires_at_ms: Some(1_792_834_200_000 + 60_000),
            ..Default::default()
        }];
        let node = ApprovalsTab::render(&props);
        let text = tree_to_text(&node);
        assert!(text.contains("<button.gov-chip on>全部 <span.chip-n>1</span></button>"));
        assert!(text.contains("<button.gov-chip>待签 <span.chip-n>1</span></button>"));
        assert!(text.contains("<button.gov-chip>已过期 <span.chip-n>0</span></button>"));
    }

    #[test]
    fn approvals_tab_renders_one_card_per_ledger_entry() {
        let caps = manifest(vec![capability(
            "permissions.approval.read",
            true,
            Some(true),
        )]);
        let mut props = approvals_props(Some(caps));
        props.ledger = vec![
            ApprovalLedgerEntry {
                approval: approval("ap-1", "sess-a"),
                session_title: Some(String::from("主线程")),
                ..Default::default()
            },
            ApprovalLedgerEntry {
                approval: approval("ap-2", "sess-b"),
                ..Default::default()
            },
        ];
        let node = ApprovalsTab::render(&props);
        assert_eq!(count_by_class(&node, "gov-card"), 2);
        let text = tree_to_text(&node);
        assert!(text.contains("主线程"));
        assert!(text.contains("会话 sess-b"), "无标题 ⇒ 显示短 id");
    }

    /// ⭐ **条件渲染的元素必须带 `key`**（2026-10-25 修「条件块出现 ⇒ 兄弟错位」）。
    ///
    /// ## 为什么
    ///
    /// 审批卡片里 `show_command` / `show_args` / `show_created` / `show_expiry`
    /// 都是**条件渲染** ⇒ 某个条件由假变真 ⇒ 该层**长度变化**
    /// ⇒ ⇒ 纯位置配对 ⇒ **后面的兄弟错位**
    /// ⇒ ⇒ 实测：「`code` 与 `footer` 的标签互换」⇒ 逐像素差 27.4%。
    ///
    /// ⇒ 带 `key` 后 ⇒ 引擎走**混合配对**（有 key 的按身份、无 key 的按位置）
    /// ⇒ ⇒ 条件块「出现 / 消失」不再让兄弟错位。
    #[test]
    fn approvals_tab_keys_the_conditional_elements_of_a_card() {
        let caps = manifest(vec![capability(
            "permissions.approval.read",
            true,
            Some(true),
        )]);
        let mut props = approvals_props(Some(caps));
        // ⚠️ **四个条件必须都为真** ⇒ 否则分支根本不渲染 ⇒ 测不出 key。
        let mut record = approval("ap-1", "sess-a");
        record.command_text = Some(String::from("fs.write ./a.txt"));
        record.arguments_summary = Some(String::from("path=./a.txt"));
        props.ledger = vec![ApprovalLedgerEntry {
            approval: record,
            session_title: Some(String::from("主线程")),
            // ⚠️ 时间戳在 **entry** 上（不是 `PendingApproval` 上）。
            created_at_ms: Some(1_767_225_600_000),
            expires_at_ms: Some(1_769_769_600_000),
        }];
        let node = ApprovalsTab::render(&props);
        let keys = collect_keys(&node);
        // ⚠️ 卡片本身也要带 key（筛选换掉整张卡片集合 ⇒ 按身份配对）。
        for expected in [
            "ap-1",
            "gov-command",
            "gov-args",
            "gov-created",
            "gov-expiry",
        ] {
            assert!(
                keys.iter().any(|key| key.contains(expected)),
                "条件元素 `{expected}` 必须带 key（实测：条件块出现 ⇒ 兄弟错位 ⇒ 27.4% 像素差）\n                 ⇒ 实测到的 key：{keys:?}"
            );
        }
    }

    /// 收集树里所有元素的 `key`（保序）。
    fn collect_keys(node: &VNode) -> Vec<String> {
        fn walk(node: &VNode, out: &mut Vec<String>) {
            match node {
                VNode::Element(element) => {
                    if let Some(key) = element.key.as_ref() {
                        out.push(format!("{key:?}"));
                    }
                    for child in &element.children {
                        walk(child, out);
                    }
                }
                VNode::Fragment(children) => {
                    for child in children {
                        walk(child, out);
                    }
                }
                VNode::Text(_) => {}
            }
        }
        let mut out = Vec::new();
        walk(node, &mut out);
        out
    }

    #[test]
    fn approvals_tab_renders_the_guide_cards_when_the_ledger_is_empty() {
        let caps = manifest(vec![capability(
            "permissions.approval.read",
            true,
            Some(true),
        )]);
        let node = ApprovalsTab::render(&approvals_props(Some(caps)));
        let text = tree_to_text(&node);
        assert!(text.contains("账上无待签"));
        assert_eq!(count_by_class(&node, "guide-card"), 2);
        assert!(text.contains("去对话"));
        assert!(text.contains("看审计"));
    }

    #[test]
    fn approvals_tab_marks_an_expired_entry_as_faded() {
        let caps = manifest(vec![capability(
            "permissions.approval.read",
            true,
            Some(true),
        )]);
        let mut props = approvals_props(Some(caps));
        props.ledger = vec![ApprovalLedgerEntry {
            approval: approval("ap-1", "sess-a"),
            expires_at_ms: Some(1_792_834_200_000 - 1),
            ..Default::default()
        }];
        let node = ApprovalsTab::render(&props);
        assert_eq!(attr_text(&node, "class").as_deref(), Some("gov-tab"));
        let text = tree_to_text(&node);
        assert!(text.contains("<article.gov-card faded>"));
        assert!(text.contains("已过期"));
    }

    #[test]
    fn approvals_tab_disables_the_resolve_buttons_when_unsupported() {
        let caps = manifest(vec![capability(
            "permissions.approval.read",
            true,
            Some(true),
        )]);
        let mut props = approvals_props(Some(caps));
        props.ledger = vec![ApprovalLedgerEntry {
            approval: approval("ap-1", "sess-a"),
            ..Default::default()
        }];
        let node = ApprovalsTab::render(&props);
        let text = tree_to_text(&node);
        assert!(text.contains("<button.btn-reject>拒绝</button>"));
        let reject = find_element(&node, "btn-reject").expect("拒绝按钮");
        assert_eq!(element_attr(reject, "disabled").as_deref(), Some(""));
        assert_eq!(
            element_attr(reject, "title").as_deref(),
            Some("当前运行时不支持 permissions.approval.resolve")
        );
    }

    #[test]
    fn approvals_tab_shows_the_loading_state_while_loading() {
        let caps = manifest(vec![capability(
            "permissions.approval.read",
            true,
            Some(true),
        )]);
        let mut props = approvals_props(Some(caps));
        props.loading = true;
        let node = ApprovalsTab::render(&props);
        assert!(tree_to_text(&node).contains("正在对齐各会话的待批账本…"));
    }

    #[test]
    fn approvals_tab_shows_the_error_state_when_the_fetch_failed() {
        let caps = manifest(vec![capability(
            "permissions.approval.read",
            true,
            Some(true),
        )]);
        let mut props = approvals_props(Some(caps));
        props.error = String::from("网关不可达（对照固定数据）");
        let node = ApprovalsTab::render(&props);
        let text = tree_to_text(&node);
        assert!(text.contains("拉取审批账本失败"));
        assert!(text.contains("网关不可达（对照固定数据）"));
    }

    // ── §10 授权管理 Tab ───────────────────────────────────────────────

    /// 造一份授权面的 props。
    fn grants_props(caps: Option<CapabilityManifest>) -> GrantsTabProps {
        GrantsTabProps {
            capabilities: caps,
            grants: Vec::new(),
            loading: false,
            error: String::new(),
            confirming_id: None,
            revoking_id: None,
            revoke_error: String::new(),
            now_ms: 1_792_834_200_000,
            on_confirm: None,
            on_revoke: None,
            on_cancel: None,
        }
    }

    #[test]
    fn grants_tab_renders_the_account_color_and_the_count() {
        let caps = manifest(vec![capability(
            "permissions.grants.read",
            true,
            Some(true),
        )]);
        let node = GrantsTab::render(&grants_props(Some(caps)));
        assert_eq!(
            attr_text(&node, "style").as_deref(),
            Some("--gov-accent: var(--ap-register-deepops-green)")
        );
        assert!(tree_to_text(&node).contains("0 项授权在账"));
    }

    #[test]
    fn grants_tab_renders_the_empty_state() {
        let caps = manifest(vec![capability(
            "permissions.grants.read",
            true,
            Some(true),
        )]);
        let node = GrantsTab::render(&grants_props(Some(caps)));
        let text = tree_to_text(&node);
        assert!(text.contains("账上无授权"));
        assert!(text.contains("当你在审批文书上批准一项能力"));
    }

    #[test]
    fn grants_tab_renders_the_confirm_block_only_for_the_confirmed_row() {
        let caps = manifest(vec![capability(
            "permissions.grants.read",
            true,
            Some(true),
        )]);
        let mut props = grants_props(Some(caps));
        props.grants = vec![
            GrantView {
                id: String::from("tool.repo.write"),
                name: String::from("tool.repo.write"),
                activated_at_ms: 1_792_834_200_000 - 3_600_000,
            },
            GrantView {
                id: String::from("memory.read"),
                name: String::from("memory.read"),
                activated_at_ms: 0,
            },
        ];
        let plain = GrantsTab::render(&props);
        assert_eq!(count_by_class(&plain, "revoke-confirm"), 0);
        assert!(tree_to_text(&plain).contains("授予时刻未记录（canonical policy 不记时间戳）"));
        assert!(tree_to_text(&plain).contains("入账 "));

        props.confirming_id = Some(String::from("tool.repo.write"));
        let confirming = GrantsTab::render(&props);
        assert_eq!(count_by_class(&confirming, "revoke-confirm"), 1);
        let text = tree_to_text(&confirming);
        assert!(text.contains("确认撤销 tool.repo.write？"));
        assert!(text.contains("撤销即刻生效，作用于当前进程的同一 policy 实例"));
    }

    #[test]
    fn grants_tab_marks_the_unsupported_revoke_capability() {
        let caps = manifest(vec![
            capability("permissions.grants.read", true, Some(true)),
            capability("permissions.revoke", false, Some(false)),
        ]);
        let mut props = grants_props(Some(caps));
        props.grants = vec![GrantView {
            id: String::from("tool.repo.write"),
            name: String::from("tool.repo.write"),
            activated_at_ms: 1,
        }];
        let node = GrantsTab::render(&props);
        let text = tree_to_text(&node);
        assert!(text.contains("<code>permissions.revoke</code>"));
    }

    // ── §11 审计 Tab ───────────────────────────────────────────────────

    /// 造一份审计面的 props。
    fn audit_props(caps: Option<CapabilityManifest>) -> AuditTabProps {
        let runtime = apeireth_ui_core::Runtime::new();
        AuditTabProps {
            capabilities: caps,
            source: runtime.signal(AuditSource::Traces),
            trace_rows: Vec::new(),
            trace_error: String::new(),
            trace_filter: runtime.signal(TraceFilter::All),
            selected_trace_id: None,
            trace_spans: Vec::new(),
            trace_detail_loading: false,
            trace_detail_error: String::new(),
            audit_items: Vec::new(),
            audit_error: String::new(),
            audit_group: runtime.signal(String::from("all")),
            selected_audit_id: None,
            loading: false,
            now_ms: 1_792_834_200_000,
            on_source: None,
            on_trace_filter: None,
            on_audit_group: None,
            on_select_trace: None,
            on_select_audit: None,
        }
    }

    /// 造一条轨迹行。
    fn trace_row(id: &str, status: &str) -> TraceRow {
        TraceRow {
            trace_id: String::from(id),
            span_count: 3,
            started_at_ms: 1_792_834_200_000,
            root_kind: String::from("turn"),
            root_actor: String::from("assistant"),
            root_status: String::from(status),
            root_summary: String::from("装配工作区"),
            session_id: Some(String::from("sess-a")),
            has_error: status == "error",
        }
    }

    #[test]
    fn audit_tab_renders_the_account_color_and_the_split() {
        let caps = manifest(vec![capability("trace.read", true, Some(true))]);
        let node = AuditTab::render(&audit_props(Some(caps)));
        assert_eq!(
            attr_text(&node, "style").as_deref(),
            Some("--gov-accent: var(--ap-register-deepops-yellow)")
        );
        assert_eq!(find_by_class(&node, "md-split"), Some("div".into()));
        assert_eq!(find_by_class(&node, "md-master"), Some("div".into()));
        assert_eq!(find_by_class(&node, "md-detail"), Some("div".into()));
    }

    #[test]
    fn audit_tab_renders_the_trace_rows_with_badges() {
        let caps = manifest(vec![capability("trace.read", true, Some(true))]);
        let mut props = audit_props(Some(caps));
        props.trace_rows = vec![trace_row("trace-1", "ok"), trace_row("trace-2", "error")];
        let node = AuditTab::render(&props);
        assert_eq!(count_by_class(&node, "md-row"), 2);
        let text = tree_to_text(&node);
        assert!(text.contains("装配工作区"));
        assert!(text.contains("3 spans · "));
        assert!(text.contains("<span.status-badge danger small>error</span>"));
    }

    #[test]
    fn audit_tab_shows_the_placeholder_when_no_trace_is_selected() {
        let caps = manifest(vec![capability("trace.read", true, Some(true))]);
        let mut props = audit_props(Some(caps));
        props.trace_rows = vec![trace_row("trace-1", "ok")];
        let node = AuditTab::render(&props);
        assert!(tree_to_text(&node).contains("选择左侧一条轨迹，这里会出现它的完整 span 树"));
    }

    #[test]
    fn audit_tab_renders_the_span_tree_for_the_selected_trace() {
        let caps = manifest(vec![capability("trace.read", true, Some(true))]);
        let mut props = audit_props(Some(caps));
        props.trace_rows = vec![trace_row("trace-1", "ok")];
        props.selected_trace_id = Some(String::from("trace-1"));
        props.trace_spans = vec![TraceSpanItem {
            span_id: String::from("span-1"),
            kind: String::from("tool"),
            actor: String::from("fs.write"),
            status: String::from("ok"),
            summary: Some(String::from("写入 notes.md")),
            started_at: 1_792_834_200_000.0,
            ended_at: Some(1_792_834_201_000.0),
            parent_span_id: None,
        }];
        let node = AuditTab::render(&props);
        let text = tree_to_text(&node);
        assert!(text.contains("<span.span-kind mono>tool</span>"));
        assert!(text.contains("fs.write"));
        assert!(text.contains("1.00s"));
        assert!(text.contains("写入 notes.md"));
        let row = find_element(&node, "span-row").expect("span 行");
        assert_eq!(
            element_attr(row, "style").as_deref(),
            Some("padding-left: 0px;")
        );
    }

    #[test]
    fn audit_tab_renders_the_audit_detail_for_the_selected_event() {
        let caps = manifest(vec![capability("audit.read", true, Some(true))]);
        let mut props = audit_props(Some(caps));
        props.source.set(AuditSource::Events);
        props.audit_items = vec![ActivityItem {
            id: String::from("chat.turn.completed-1"),
            timestamp: 1_792_834_200_000,
            title: String::from("chat.turn.completed"),
            summary: String::from("回合完成"),
            source: String::from("audit"),
            detail: Some(String::from("{\n  \"ts\": 1\n}")),
        }];
        props.selected_audit_id = Some(String::from("chat.turn.completed-1"));
        let node = AuditTab::render(&props);
        let text = tree_to_text(&node);
        assert!(text.contains("<span.ad-lbl>事件</span>"));
        assert!(text.contains("chat.turn.completed"));
        assert!(text.contains("<span.ad-lbl>摘要</span>"));
        assert!(text.contains("<span.ad-lbl>原始记录</span>"));
    }

    #[test]
    fn audit_tab_marks_both_sources_as_unsupported() {
        let caps = manifest(vec![capability("trace.read", false, Some(false))]);
        let node = AuditTab::render(&audit_props(Some(caps)));
        assert_eq!(count_by_class(&node, "gov-unsupported"), 1);
    }

    // ── §12 守卫 Tab ───────────────────────────────────────────────────

    /// 造一份守卫面的 props。
    fn guard_props(caps: Option<CapabilityManifest>) -> GuardTabProps {
        let runtime = apeireth_ui_core::Runtime::new();
        GuardTabProps {
            capabilities: caps,
            status: None,
            status_error: String::new(),
            events: Vec::new(),
            events_error: String::new(),
            loading: false,
            expanded_ids: runtime.signal(Vec::new()),
            dry_result: None,
            dry_error: String::new(),
            now_ms: 1_792_834_200_000,
            on_toggle_event: None,
        }
    }

    /// 造一份守卫状态。
    fn guard_status_fixture() -> GuardStatus {
        GuardStatus {
            enabled: true,
            fast_guard_active: true,
            chain_guard_active: true,
            intent_guard_active: true,
            cross_turn_monitoring_active: false,
            ml_classifier_available: true,
            ml_model_version: Some(String::from("m3")),
            total_evaluations: 128,
            total_allowed: 96,
            total_denied: 12,
            total_approval_required: 20,
        }
    }

    #[test]
    fn guard_tab_renders_the_account_color() {
        let caps = manifest(vec![capability(
            "safety.guard.status.read",
            true,
            Some(true),
        )]);
        let node = GuardTab::render(&guard_props(Some(caps)));
        assert_eq!(
            attr_text(&node, "style").as_deref(),
            Some("--gov-accent: var(--ap-register-deepops-red)")
        );
    }

    #[test]
    fn guard_tab_renders_the_status_badges_and_the_tally() {
        let caps = manifest(vec![capability(
            "safety.guard.status.read",
            true,
            Some(true),
        )]);
        let mut props = guard_props(Some(caps));
        props.status = Some(guard_status_fixture());
        let node = GuardTab::render(&props);
        let text = tree_to_text(&node);
        assert!(text.contains("<span.status-badge green normal>守卫启用</span>"));
        assert!(text.contains("<span.status-badge blue small>快速守卫</span>"));
        assert!(text.contains("<span.status-badge blue small>意图守卫</span>"));
        assert!(!text.contains("跨轮监测"), "未装配 ⇒ 不渲染");
        assert!(text.contains("ML 分类器 m3"));
        assert_eq!(count_by_class(&node, "tally-cell"), 4);
        assert!(text.contains("<span.tally-num mono ok>96</span>"));
        assert!(text.contains("<span.tally-num mono bad>12</span>"));
        assert!(text.contains("<span.tally-num mono warn>20</span>"));
    }

    #[test]
    fn guard_tab_renders_the_event_rows_with_the_risk_bar() {
        let caps = manifest(vec![capability(
            "safety.guard.events.read",
            true,
            Some(true),
        )]);
        let mut props = guard_props(Some(caps));
        props.events = vec![GuardEvent {
            timestamp_ms: 1_792_834_200_000,
            session_id: String::from("sess-alpha-0123456789"),
            trace_id: String::from("trace-1"),
            round: 3,
            capability_id: String::from("tool.repo.write"),
            stage: String::from("chain_guard"),
            decision: String::from("approval_required"),
            risk_score: 0.72,
            reasons: vec![String::from("写操作需要主人点头")],
            evidence: vec![String::from("policy:repo.write")],
        }];
        let node = GuardTab::render(&props);
        let text = tree_to_text(&node);
        assert!(text.contains("<span.status-badge amber small>待签</span>"));
        assert!(text.contains("链式守卫"));
        assert!(text.contains("72%"));
        let fill = find_element(&node, "ev-risk-fill").expect("风险条");
        assert_eq!(element_attr(fill, "style").as_deref(), Some("width: 72%;"));
        assert!(text.contains("<span.ev-risk-fill high>"));
        assert!(text.contains("R3"));
        assert!(text.contains("会话 sess-alp…"), "会话 id 取前 8 位");
    }

    #[test]
    fn guard_tab_expands_the_reasons_and_evidence_on_demand() {
        let caps = manifest(vec![capability(
            "safety.guard.events.read",
            true,
            Some(true),
        )]);
        let mut props = guard_props(Some(caps));
        props.events = vec![GuardEvent {
            timestamp_ms: 1_792_834_200_000,
            session_id: String::from("sess"),
            trace_id: String::from("trace-1"),
            round: 1,
            capability_id: String::from("tool.repo.write"),
            stage: String::from("chain_guard"),
            decision: String::from("allow"),
            risk_score: 0.1,
            reasons: vec![String::from("理由一")],
            evidence: vec![String::from("证据一")],
        }];
        let collapsed = GuardTab::render(&props);
        assert_eq!(count_by_class(&collapsed, "ev-detail"), 0);

        props
            .expanded_ids
            .set(vec![String::from("trace-1-1-1792834200000")]);
        let expanded = GuardTab::render(&props);
        assert_eq!(count_by_class(&expanded, "ev-detail"), 1);
        let text = tree_to_text(&expanded);
        assert!(text.contains("理由一"));
        assert!(text.contains("证据一"));
    }

    #[test]
    fn guard_tab_renders_the_dry_run_card_and_the_result() {
        let caps = manifest(vec![capability("safety.guard.evaluate", true, Some(true))]);
        let mut props = guard_props(Some(caps));
        props.dry_result = Some(GuardDryRunResult {
            decision: String::from("deny"),
            stage: String::from("decision_fusion"),
            risk_score: 0.93,
            reasons: vec![String::from("目标越界")],
            evidence: vec![String::from("scope:mismatch")],
        });
        let node = GuardTab::render(&props);
        let text = tree_to_text(&node);
        assert!(text.contains("试算一道调用"));
        assert!(text.contains("DRY-RUN · 只评估不执行"));
        let input = find_element(&node, "dry-input").expect("试算输入框");
        assert_eq!(
            element_attr(input, "placeholder").as_deref(),
            Some("tool.repo")
        );
        assert!(text.contains("<span.status-badge danger normal>否决</span>"));
        assert!(text.contains("风险 93%"));
        assert!(text.contains("目标越界"));
        assert!(text.contains("证据 1 条"));
    }

    #[test]
    fn guard_tab_shows_the_empty_state_when_no_event_is_recorded() {
        let caps = manifest(vec![capability(
            "safety.guard.events.read",
            true,
            Some(true),
        )]);
        let node = GuardTab::render(&guard_props(Some(caps)));
        assert!(tree_to_text(&node).contains("守卫尚未拦下任何评估记录"));
    }
}
