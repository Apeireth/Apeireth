//! 卡面组件：**工具卡**与**工具调用生命周期卡**。
//!
//! ## 两张卡的状态枚举**不同**（刻意不合并）
//!
//! | 卡 | 状态枚举 |
//! |---|---|
//! | 工具卡 | `pending` / `running` / `succeeded` / `failed` / `cancelled` |
//! | 生命周期卡 | `pending` / `running` / `success` / `error` |
//!
//! 两者的标签文案与外观 class 也**不同** ⇒ 合并枚举会引入「同一个状态两种
//! 外观」的歧义 ⇒ 各自独立建模。
//!
//! ## 判定的口径（与生产组件一致）
//!
//! | 判定 | 口径 |
//! |---|---|
//! | 参数区是否出现 | `args` **真值** 或 `rawArgs` **真值** |
//! | 参数区走哪一支 | `args` 是**对象**（非 `null`）⇒ 逐键预览；否则 ⇒ 原始文本 |
//! | 时长是否出现 | `durationMs` **真值**（`0` 不出现） |
//! | 生命周期延迟是否出现 | `latencyMs` **已定义**（`0` 出现，文本为 `0ms`） |
//! | 原始 JSON 区是否出现 | `resultFull` **真值** 或 `args` **真值** |
//! | 原始 JSON 的键 | `args` / `result` / `error`；**缺省（未定义）的键整个被省略** |
//!
//! ## 空白
//!
//! 组件模板里相邻元素之间的换行会折叠成**一个空格**（文本节点）⇒ 本组件
//! **逐个复现**这些空白文本节点（否则 DOM 逐字比对会出现差异）。

use crate::icon::Icon;
use crate::icon::{IconData, IconProps};
use crate::icon_data::{
    ICON_CHEVRON_DOWN, ICON_CHEVRON_RIGHT, ICON_CIRCLE_CHECK, ICON_CIRCLE_X, ICON_CLOCK,
    ICON_LOADER_CIRCLE, ICON_TRIANGLE_ALERT, ICON_WRENCH,
};
use crate::json::JsonValue;
use crate::num::{number_to_string, round, string_length, string_slice, to_fixed};
use apeireth_ui_core::{Signal, UiComponent};
use apeireth_ui_dom::VNode;
use apeireth_ui_macro::view;

/// 工具卡的状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToolCallStatus {
    /// 等待中。
    #[default]
    Pending,
    /// 执行中。
    Running,
    /// 已完成。
    Succeeded,
    /// 失败。
    Failed,
    /// 已取消。
    Cancelled,
}

/// 生命周期卡的状态（**与工具卡的状态枚举不同**，刻意不合并）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToolCallLifecycleStatus {
    /// 待执行。
    #[default]
    Pending,
    /// 执行中。
    Running,
    /// 成功。
    Success,
    /// 失败。
    Error,
}

/// 工具卡的数据形状。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ToolCallDetails {
    /// 状态。
    pub status: ToolCallStatus,
    /// 工具名。
    pub name: String,
    /// 耗时（毫秒）；**缺省 ⇒ 不显示**；`0` 同样不显示（真值判定）。
    pub duration_ms: Option<f64>,
    /// 调用参数（**缺省 ⇒ 未定义**；键序按插入序保留）。
    pub args: Option<JsonValue>,
    /// 原始参数文本（字符串）。
    pub raw_args: Option<String>,
    /// 结果摘要。
    pub result_summary: Option<String>,
    /// 完整结果。
    pub result_full: Option<JsonValue>,
    /// 错误信息。
    pub error: Option<String>,
}

/// 生命周期卡的数据形状。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ToolCallLifecycleCall {
    /// 调用标识。
    pub id: String,
    /// 工具名。
    pub name: String,
    /// 状态。
    pub status: ToolCallLifecycleStatus,
    /// 延迟（毫秒）；**缺省 ⇒ 不显示**；`0` 显示（文本 `0ms`）。
    pub latency_ms: Option<f64>,
    /// 错误信息。
    pub error: Option<String>,
    /// 参数（**缺省 ⇒ 未定义** ⇒ 不出现参数区）。
    pub arguments: Option<JsonValue>,
    /// 结果（**缺省 ⇒ 未定义** ⇒ 不出现结果区）。
    pub result: Option<JsonValue>,
}

/// 格式化耗时（毫秒 ⇒ 文本）。
///
/// 口径：缺省 / 非数 ⇒ 空串；小于 1000 ⇒ `Nms`；否则 ⇒ `N.Ns`。
pub fn format_duration(ms: Option<f64>) -> String {
    let Some(value) = ms else {
        return String::new();
    };
    if value.is_nan() {
        return String::new();
    }
    if value < 1000.0 {
        return format!("{}ms", number_to_string(value));
    }
    format!("{}s", to_fixed(value / 1000.0, 1))
}

/// 格式化延迟（毫秒 ⇒ 文本）。
///
/// 口径：缺省 / 非数 ⇒ 空串；小于 1000 ⇒ `Nms`（**取整**）；否则 ⇒ `N.Ns`。
pub fn format_latency(ms: Option<f64>) -> String {
    let Some(value) = ms else {
        return String::new();
    };
    if value.is_nan() {
        return String::new();
    }
    if value < 1000.0 {
        return format!("{}ms", number_to_string(round(value)));
    }
    format!("{}s", to_fixed(value / 1000.0, 1))
}

/// 生命周期卡的值 ⇒ 文本（**超长截断**）。
///
/// 口径：字符串 ⇒ 原样；其余 ⇒ 缩进序列化；超过 `limit` 个 **UTF-16 码元**
/// ⇒ 截断并追加「…（已截断，共 N 字符）」。
pub fn stringify_value(value: &JsonValue, limit: usize) -> String {
    let text = match value {
        JsonValue::String(text) => text.clone(),
        other => other.to_string_pretty(),
    };
    let length = string_length(&text);
    if length > limit {
        return format!(
            "{}…（已截断，共 {length} 字符）",
            string_slice(&text, 0, limit)
        );
    }
    text
}

/// 值的键值对列表（对象 ⇒ 键值对；数组 ⇒ 下标 ⇒ 值）。
///
/// 与「取对象的键值对」的口径一致 ⇒ 数组按下标列出。
fn entries(value: &JsonValue) -> Vec<(String, JsonValue)> {
    match value {
        JsonValue::Object(entries) => entries.clone(),
        JsonValue::Array(items) => items
            .iter()
            .enumerate()
            .map(|(index, item)| (index.to_string(), item.clone()))
            .collect(),
        _ => Vec::new(),
    }
}

/// 参数预览的键值对（键 + 文本）。
///
/// 文本口径：字符串 ⇒ 原样；其余 ⇒ **紧凑**序列化。
fn preview_rows(value: &JsonValue) -> Vec<(String, String)> {
    entries(value)
        .into_iter()
        .map(|(key, item)| {
            let text = match &item {
                JsonValue::String(text) => text.clone(),
                other => other.to_string_compact(),
            };
            (key, text)
        })
        .collect()
}

/// 工具卡的 props。
#[derive(Debug, Clone)]
pub struct ToolCallCardProps {
    /// 卡片数据。
    pub tool_call: ToolCallDetails,
    /// 展开态（**调用方持有**的信号 ⇒ 组件是纯函数）。
    pub expanded: Signal<bool>,
    /// 原始 JSON 态（调用方持有的信号）。
    pub show_raw_json: Signal<bool>,
}

impl ToolCallCardProps {
    /// 建一组 props。
    pub fn new(
        tool_call: ToolCallDetails,
        expanded: Signal<bool>,
        show_raw_json: Signal<bool>,
    ) -> Self {
        Self {
            tool_call,
            expanded,
            show_raw_json,
        }
    }
}

/// 工具卡组件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ToolCallCard;

impl UiComponent for ToolCallCard {
    type Props = ToolCallCardProps;
    type Output = VNode;

    fn render(props: &Self::Props) -> Self::Output {
        let tool_call = &props.tool_call;
        let expanded = props.expanded.get();
        let show_raw_json = props.show_raw_json.get();

        // ── 状态（标签 / 外观 class / 图标） ──────────────────────────────
        let (status_label, status_class, status_icon) = match tool_call.status {
            ToolCallStatus::Pending => ("等待中", "tool-status neutral", &ICON_CLOCK),
            ToolCallStatus::Running => ("执行中", "tool-status amber", &ICON_LOADER_CIRCLE),
            ToolCallStatus::Succeeded => ("已完成", "tool-status green", &ICON_CIRCLE_CHECK),
            ToolCallStatus::Failed => ("失败", "tool-status danger", &ICON_CIRCLE_X),
            ToolCallStatus::Cancelled => ("已取消", "tool-status neutral", &ICON_TRIANGLE_ALERT),
        };
        let is_failed = tool_call.status == ToolCallStatus::Failed;
        // 图标与标签之间的换行折叠成**一个空格**（文本节点带前导空格）。
        let status_label_text = format!(" {status_label}");
        let status_icon_node = Icon::render(
            &IconProps::new(status_icon, 12)
                .conditional_spin(tool_call.status == ToolCallStatus::Running),
        );

        // ── 标题组 ──────────────────────────────────────────────────────
        let wrench_node = Icon::render(&IconProps::new(&ICON_WRENCH, 13));
        let name_text = tool_call.name.clone();
        // 时长：**真值**才出现（`0` 不出现）。
        let duration_visible =
            matches!(tool_call.duration_ms, Some(value) if value != 0.0 && !value.is_nan());
        let duration_text = format_duration(tool_call.duration_ms);

        let chevron_node = Icon::render(&IconProps::new(
            if expanded {
                &ICON_CHEVRON_DOWN
            } else {
                &ICON_CHEVRON_RIGHT
            },
            14,
        ));
        let expand_label = if expanded {
            "收起详情"
        } else {
            "展开详情"
        };

        // ── 展开体 ──────────────────────────────────────────────────────
        let args_truthy = tool_call.args.as_ref().is_some_and(JsonValue::is_truthy);
        let raw_args_truthy = tool_call
            .raw_args
            .as_ref()
            .is_some_and(|text| !text.is_empty());
        let show_args_section = args_truthy || raw_args_truthy;
        let args_is_object = matches!(&tool_call.args, Some(value) if matches!(value, JsonValue::Object(_) | JsonValue::Array(_)));
        let arg_rows: Vec<(String, String)> = tool_call
            .args
            .as_ref()
            .map(preview_rows)
            .unwrap_or_default()
            .into_iter()
            .map(|(key, value)| (format!("{key}:"), value))
            .collect();
        // 非对象分支的文本：原始参数文本，否则缩进序列化的参数。
        let raw_args_text = match &tool_call.raw_args {
            Some(text) if !text.is_empty() => text.clone(),
            _ => tool_call
                .args
                .as_ref()
                .map(JsonValue::to_string_pretty)
                .unwrap_or_default(),
        };

        let show_result_section = tool_call
            .result_summary
            .as_ref()
            .is_some_and(|text| !text.is_empty());
        let result_summary_text = tool_call.result_summary.clone().unwrap_or_default();

        let show_error_section = tool_call
            .error
            .as_ref()
            .is_some_and(|text| !text.is_empty());
        let error_text = tool_call.error.clone().unwrap_or_default();

        let show_raw_section = args_truthy
            || tool_call
                .result_full
                .as_ref()
                .is_some_and(JsonValue::is_truthy);
        let raw_button_text = if show_raw_json {
            "隐藏完整数据"
        } else {
            "查看原始 JSON"
        };
        let raw_json_text = raw_json_block(tool_call);

        // ── 事件（信号由调用方持有 ⇒ 组件保持纯函数） ────────────────────
        let toggle_click = props.expanded.clone();
        let toggle_key = props.expanded.clone();
        let toggle_raw = props.show_raw_json.clone();

        view! {
            div(class = "tool-card", class:failed = is_failed) {
                div(
                    class = "tool-header",
                    role = "button",
                    tabindex = "0",
                    onclick = move |_| { toggle_click.set(!toggle_click.get()); },
                    onkeydown = move |event| {
                        if event.key() == Some("Enter") {
                            toggle_key.set(!toggle_key.get());
                        }
                    }
                ) {
                    div(class = "tool-title-group") {
                        span(class = "tool-icon") { { wrench_node } }
                        " "
                        strong(class = "tool-name") { name_text }
                        " "
                        span(class = status_class) {
                            { status_icon_node }
                            status_label_text
                        }
                        " "
                        if duration_visible {
                            span(class = "tool-duration") { duration_text }
                        }
                    }
                    " "
                    button(class = "expand-btn", aria-label = expand_label) {
                        { chevron_node }
                    }
                }
                " "
                if expanded {
                    div(class = "tool-body") {
                        if show_args_section {
                            div(class = "tool-section") {
                                span(class = "section-label") { "调用参数" }
                                " "
                                if args_is_object {
                                    div(class = "args-preview") {
                                        for (key, value) in arg_rows.clone() {
                                            div(class = "arg-row") {
                                                span(class = "arg-key") { key }
                                                " "
                                                span(class = "arg-val") { value }
                                            }
                                        }
                                    }
                                } else {
                                    pre(class = "code-block") { raw_args_text }
                                }
                            }
                        }
                        " "
                        if show_result_section {
                            div(class = "tool-section") {
                                span(class = "section-label") { "执行结果" }
                                " "
                                p(class = "result-text") { result_summary_text }
                            }
                        }
                        " "
                        if show_error_section {
                            div(class = "tool-section error-section") {
                                span(class = "section-label") { "错误信息" }
                                " "
                                p(class = "error-text") { error_text }
                            }
                        }
                        " "
                        if show_raw_section {
                            div(class = "tool-section raw-section") {
                                button(
                                    class = "raw-toggle-btn",
                                    onclick = move |_| { toggle_raw.set(!toggle_raw.get()); }
                                ) {
                                    raw_button_text
                                }
                                " "
                                if show_raw_json {
                                    pre(class = "code-block json-detail") { raw_json_text }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// 原始 JSON 块（`args` / `result` / `error` 三键；**未定义的键整个省略**）。
fn raw_json_block(tool_call: &ToolCallDetails) -> String {
    let mut entries: Vec<(String, JsonValue)> = Vec::new();
    if let Some(args) = &tool_call.args {
        entries.push((String::from("args"), args.clone()));
    }
    // `result` = 完整结果 **或** 摘要（**真值**判定 ⇒ 假值回落到摘要）。
    let result = match &tool_call.result_full {
        Some(value) if value.is_truthy() => Some(value.clone()),
        _ => tool_call.result_summary.clone().map(JsonValue::String),
    };
    if let Some(result) = result {
        entries.push((String::from("result"), result));
    }
    if let Some(error) = &tool_call.error {
        entries.push((String::from("error"), JsonValue::String(error.clone())));
    }
    JsonValue::Object(entries).to_string_pretty()
}

/// 生命周期卡的 props。
#[derive(Debug, Clone)]
pub struct ToolCallLifecycleCardProps {
    /// 调用数据。
    pub call: ToolCallLifecycleCall,
    /// 展开态（调用方持有的信号）。
    pub expanded: Signal<bool>,
}

impl ToolCallLifecycleCardProps {
    /// 建一组 props。
    pub fn new(call: ToolCallLifecycleCall, expanded: Signal<bool>) -> Self {
        Self { call, expanded }
    }
}

/// 生命周期卡组件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ToolCallLifecycleCard;

impl UiComponent for ToolCallLifecycleCard {
    type Props = ToolCallLifecycleCardProps;
    type Output = VNode;

    fn render(props: &Self::Props) -> Self::Output {
        let call = &props.call;
        let expanded = props.expanded.get();

        let (status_label, dot_class, label_class) = match call.status {
            ToolCallLifecycleStatus::Pending => {
                ("待执行", "status-dot pending", "status-label pending")
            }
            ToolCallLifecycleStatus::Running => {
                ("执行中", "status-dot running", "status-label running")
            }
            ToolCallLifecycleStatus::Success => {
                ("成功", "status-dot success", "status-label success")
            }
            ToolCallLifecycleStatus::Error => ("失败", "status-dot error", "status-label error"),
        };
        let is_failed = call.status == ToolCallLifecycleStatus::Error;
        let status_label_text = status_label.to_owned();

        let name_text = call.name.clone();
        // 延迟：**已定义**才出现（`0` 出现 ⇒ 文本 `0ms`）。
        let latency_visible = call.latency_ms.is_some();
        let latency_text = format_latency(call.latency_ms);

        let chevron_node = Icon::render(&IconProps::new(
            if expanded {
                &ICON_CHEVRON_DOWN
            } else {
                &ICON_CHEVRON_RIGHT
            },
            14,
        ));
        let expand_label = if expanded {
            "收起详情"
        } else {
            "展开详情"
        };
        let aria_expanded = if expanded { "true" } else { "false" };

        let show_error_section = call.error.as_ref().is_some_and(|text| !text.is_empty());
        let error_text = call.error.clone().unwrap_or_default();
        let show_arguments_section = call.arguments.is_some();
        let arguments_text = call
            .arguments
            .as_ref()
            .map(|value| stringify_value(value, 4000))
            .unwrap_or_default();
        let show_result_section = call.result.is_some();
        let result_text = call
            .result
            .as_ref()
            .map(|value| stringify_value(value, 4000))
            .unwrap_or_default();

        let toggle_click = props.expanded.clone();
        let toggle_key = props.expanded.clone();

        view! {
            div(class = "tool-lifecycle-card", class:failed = is_failed) {
                div(
                    class = "tool-header",
                    role = "button",
                    tabindex = "0",
                    aria-expanded = aria_expanded,
                    onclick = move |_| { toggle_click.set(!toggle_click.get()); },
                    onkeydown = move |event| {
                        if event.key() == Some("Enter") || event.key() == Some(" ") {
                            toggle_key.set(!toggle_key.get());
                        }
                    }
                ) {
                    div(class = "tool-title-group") {
                        span(class = dot_class, aria-hidden = "true") { }
                        " "
                        strong(class = "tool-name") { name_text }
                        " "
                        span(class = label_class) { status_label_text }
                        " "
                        if latency_visible {
                            span(class = "latency") { latency_text }
                        }
                    }
                    " "
                    button(class = "expand-btn", aria-label = expand_label) {
                        { chevron_node }
                    }
                }
                " "
                if expanded {
                    div(class = "tool-body") {
                        if show_error_section {
                            div(class = "error-section") {
                                span(class = "section-label") { "错误信息" }
                                " "
                                p(class = "error-text") { error_text }
                            }
                        }
                        " "
                        if show_arguments_section {
                            div(class = "section") {
                                span(class = "section-label") { "参数" }
                                " "
                                pre(class = "code-block") { arguments_text }
                            }
                        }
                        " "
                        if show_result_section {
                            div(class = "section") {
                                span(class = "section-label") { "结果" }
                                " "
                                pre(class = "code-block") { result_text }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// 图标 props 的条件 class（命中 ⇒ 追加该 token）。
trait ConditionalSpin {
    /// 条件 class：命中 ⇒ 追加 `spin`；未命中 ⇒ 不追加。
    fn conditional_spin(self, spinning: bool) -> Self;
}

impl ConditionalSpin for IconProps {
    fn conditional_spin(self, spinning: bool) -> Self {
        if spinning {
            self.extra_class("spin")
        } else {
            self
        }
    }
}

/// 图标数据类型的别名（供外部引用）。
pub type ToolCardIcon = IconData;

#[cfg(test)]
mod tests {
    //! 两张卡的**行为**断言（渲染结果 ⇒ 结构 / class / 文本），不读源码。

    use super::*;
    use apeireth_ui_core::Runtime;

    /// 收集 class token。
    fn classes(node: &VNode, out: &mut Vec<String>) {
        for token in node.class_tokens() {
            if !out.contains(&token) {
                out.push(token);
            }
        }
        for child in node.children() {
            classes(child, out);
        }
    }

    /// 收集文本节点（逐个，含空白）。
    fn text_nodes(node: &VNode, out: &mut Vec<String>) {
        match node {
            VNode::Element(element) => {
                for child in &element.children {
                    text_nodes(child, out);
                }
            }
            VNode::Text(text) => out.push(text.data.clone()),
            VNode::Fragment(children) => {
                for child in children {
                    text_nodes(child, out);
                }
            }
        }
    }

    /// 建一份工具卡 props。
    fn tool_props(tool_call: ToolCallDetails, expanded: bool) -> ToolCallCardProps {
        tool_props_with(tool_call, expanded, false)
    }

    /// 建一份工具卡 props（可开原始 JSON 态）。
    fn tool_props_with(
        tool_call: ToolCallDetails,
        expanded: bool,
        show_raw_json: bool,
    ) -> ToolCallCardProps {
        let runtime = Runtime::new();
        ToolCallCardProps::new(
            tool_call,
            runtime.signal(expanded),
            runtime.signal(show_raw_json),
        )
    }

    /// 建一份生命周期卡 props。
    fn lifecycle_props(call: ToolCallLifecycleCall, expanded: bool) -> ToolCallLifecycleCardProps {
        let runtime = Runtime::new();
        ToolCallLifecycleCardProps::new(call, runtime.signal(expanded))
    }

    /// 最小工具卡数据。
    fn basic_details(status: ToolCallStatus) -> ToolCallDetails {
        ToolCallDetails {
            status,
            name: String::from("fs.read"),
            ..ToolCallDetails::default()
        }
    }

    #[test]
    fn the_five_statuses_have_their_own_label_and_colour() {
        let cases = [
            (ToolCallStatus::Pending, "等待中", "neutral"),
            (ToolCallStatus::Running, "执行中", "amber"),
            (ToolCallStatus::Succeeded, "已完成", "green"),
            (ToolCallStatus::Failed, "失败", "danger"),
            (ToolCallStatus::Cancelled, "已取消", "neutral"),
        ];
        for (status, label, colour) in cases {
            let node = ToolCallCard::render(&tool_props(basic_details(status), false));
            let mut texts = Vec::new();
            text_nodes(&node, &mut texts);
            assert!(
                texts.iter().any(|text| text == &format!(" {label}")),
                "状态 {status:?} ⇒ 标签 `{label}`：{texts:?}"
            );
            let mut found = Vec::new();
            classes(&node, &mut found);
            assert!(
                found.iter().any(|token| token == "tool-status")
                    && found.iter().any(|token| token == colour),
                "状态 {status:?} ⇒ 外观 `{colour}`：{found:?}"
            );
        }
    }

    #[test]
    fn the_failed_status_adds_the_conditional_class() {
        let node = ToolCallCard::render(&tool_props(basic_details(ToolCallStatus::Failed), false));
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(found.iter().any(|token| token == "failed"), "{found:?}");
    }

    #[test]
    fn the_other_statuses_have_no_conditional_class() {
        let node =
            ToolCallCard::render(&tool_props(basic_details(ToolCallStatus::Succeeded), false));
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(!found.iter().any(|token| token == "failed"), "{found:?}");
    }

    #[test]
    fn the_body_is_absent_until_expanded() {
        let node = ToolCallCard::render(&tool_props(basic_details(ToolCallStatus::Pending), false));
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(!found.iter().any(|token| token == "tool-body"), "{found:?}");
    }

    #[test]
    fn the_body_appears_when_expanded() {
        let node = ToolCallCard::render(&tool_props(basic_details(ToolCallStatus::Pending), true));
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(found.iter().any(|token| token == "tool-body"), "{found:?}");
    }

    #[test]
    fn the_duration_is_hidden_when_it_is_zero() {
        let mut details = basic_details(ToolCallStatus::Pending);
        details.duration_ms = Some(0.0);
        let node = ToolCallCard::render(&tool_props(details, false));
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(
            !found.iter().any(|token| token == "tool-duration"),
            "耗时为 0 ⇒ 不出现时长：{found:?}"
        );
    }

    #[test]
    fn the_duration_is_shown_when_it_is_positive() {
        let mut details = basic_details(ToolCallStatus::Pending);
        details.duration_ms = Some(120.0);
        let node = ToolCallCard::render(&tool_props(details, false));
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(texts.iter().any(|text| text == "120ms"), "{texts:?}");
    }

    #[test]
    fn the_argument_rows_list_every_entry_in_order() {
        let mut details = basic_details(ToolCallStatus::Pending);
        details.args = Some(JsonValue::Object(vec![
            (String::from("path"), JsonValue::String(String::from("/x"))),
            (String::from("depth"), JsonValue::Number(2.0)),
            (String::from("recursive"), JsonValue::Bool(false)),
        ]));
        let node = ToolCallCard::render(&tool_props(details, true));
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(texts.iter().any(|text| text == "path:"), "{texts:?}");
        assert!(texts.iter().any(|text| text == "/x"), "{texts:?}");
        assert!(texts.iter().any(|text| text == "depth:"), "{texts:?}");
        assert!(texts.iter().any(|text| text == "2"), "{texts:?}");
        assert!(texts.iter().any(|text| text == "recursive:"), "{texts:?}");
        assert!(texts.iter().any(|text| text == "false"), "{texts:?}");
    }

    #[test]
    fn the_raw_argument_text_is_used_when_the_arguments_are_not_an_object() {
        let mut details = basic_details(ToolCallStatus::Pending);
        details.raw_args = Some(String::from("{\n  \"glob\": \"**/*.md\"\n}"));
        let node = ToolCallCard::render(&tool_props(details, true));
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(
            texts
                .iter()
                .any(|text| text == "{\n  \"glob\": \"**/*.md\"\n}"),
            "{texts:?}"
        );
    }

    #[test]
    fn the_raw_json_block_omits_the_undefined_keys() {
        let mut details = basic_details(ToolCallStatus::Pending);
        details.args = Some(JsonValue::Object(vec![(
            String::from("path"),
            JsonValue::String(String::from("/x")),
        )]));
        let node = ToolCallCard::render(&tool_props_with(details, true, true));
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        let raw = texts
            .iter()
            .find(|text| text.contains("\"args\""))
            .expect("原始 JSON 块");
        assert_eq!(raw, "{\n  \"args\": {\n    \"path\": \"/x\"\n  }\n}");
    }

    #[test]
    fn the_lifecycle_card_has_its_own_status_labels() {
        let cases = [
            (ToolCallLifecycleStatus::Pending, "待执行"),
            (ToolCallLifecycleStatus::Running, "执行中"),
            (ToolCallLifecycleStatus::Success, "成功"),
            (ToolCallLifecycleStatus::Error, "失败"),
        ];
        for (status, label) in cases {
            let call = ToolCallLifecycleCall {
                status,
                name: String::from("fs.read"),
                ..ToolCallLifecycleCall::default()
            };
            let node = ToolCallLifecycleCard::render(&lifecycle_props(call, false));
            let mut texts = Vec::new();
            text_nodes(&node, &mut texts);
            assert!(
                texts.iter().any(|text| text == label),
                "状态 {status:?} ⇒ 标签 `{label}`：{texts:?}"
            );
        }
    }

    #[test]
    fn the_lifecycle_latency_is_shown_even_when_it_is_zero() {
        let call = ToolCallLifecycleCall {
            status: ToolCallLifecycleStatus::Pending,
            name: String::from("fs.read"),
            latency_ms: Some(0.0),
            ..ToolCallLifecycleCall::default()
        };
        let node = ToolCallLifecycleCard::render(&lifecycle_props(call, false));
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(texts.iter().any(|text| text == "0ms"), "{texts:?}");
    }

    #[test]
    fn the_lifecycle_latency_is_absent_when_it_is_undefined() {
        let call = ToolCallLifecycleCall {
            status: ToolCallLifecycleStatus::Pending,
            name: String::from("fs.read"),
            ..ToolCallLifecycleCall::default()
        };
        let node = ToolCallLifecycleCard::render(&lifecycle_props(call, false));
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(
            !found.iter().any(|token| token == "latency"),
            "延迟未定义 ⇒ 不出现延迟：{found:?}"
        );
    }

    #[test]
    fn the_lifecycle_argument_section_needs_a_defined_value() {
        let call = ToolCallLifecycleCall {
            status: ToolCallLifecycleStatus::Running,
            name: String::from("shell.exec"),
            ..ToolCallLifecycleCall::default()
        };
        let node = ToolCallLifecycleCard::render(&lifecycle_props(call, true));
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(
            !found.iter().any(|token| token == "section"),
            "参数未定义 ⇒ 不出现参数区：{found:?}"
        );
    }

    #[test]
    fn the_lifecycle_long_value_is_truncated_with_the_total_length() {
        let long = "卷".repeat(4100);
        let call = ToolCallLifecycleCall {
            status: ToolCallLifecycleStatus::Success,
            name: String::from("fs.watch"),
            result: Some(JsonValue::String(long.clone())),
            ..ToolCallLifecycleCall::default()
        };
        let node = ToolCallLifecycleCard::render(&lifecycle_props(call, true));
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        let truncated = texts
            .iter()
            .find(|text| text.contains("已截断"))
            .expect("截断文本");
        assert!(
            truncated.ends_with("…（已截断，共 4100 字符）"),
            "截断文本：{truncated}"
        );
        assert_eq!(
            string_length(truncated),
            4000 + string_length("…（已截断，共 4100 字符）")
        );
    }

    #[test]
    fn the_duration_and_latency_formats_follow_their_own_rules() {
        assert_eq!(format_duration(None), "");
        assert_eq!(format_duration(Some(0.0)), "0ms");
        assert_eq!(format_duration(Some(120.0)), "120ms");
        assert_eq!(format_duration(Some(1500.0)), "1.5s");
        assert_eq!(format_duration(Some(f64::NAN)), "");
        assert_eq!(format_latency(None), "");
        assert_eq!(format_latency(Some(0.0)), "0ms");
        assert_eq!(format_latency(Some(120.4)), "120ms");
        assert_eq!(format_latency(Some(1500.0)), "1.5s");
        assert_eq!(format_latency(Some(f64::NAN)), "");
    }
}
