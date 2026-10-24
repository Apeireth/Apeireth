//! 试点组件：**审批卡**（Rust 版）。
//!
//! ## 这是什么
//!
//! 生产前端 `frontend/companion-desktop/src/lib/components/ApprovalRequestCard.svelte`
//! 的 Rust 重写（P3 试点对象）。**业务名词只许出现在 demo crate**（设计文档
//! §1.3 第 12 条：引擎层不认识业务语义）⇒ 本文件是引擎里**唯一**出现业务
//! 名词的地方。
//!
//! ## 与既有实现的逐条对齐
//!
//! | 维度 | 对齐口径 |
//! |---|---|
//! | class 名 | **21 个 class 名逐字一致**（设计文档 §9.1） |
//! | DOM 结构 | 元素标签、嵌套关系、文本节点逐字一致 |
//! | 属性 | `role` / `aria-label` / `title` / `disabled` 逐条一致；**不补**既有实现没有的属性 |
//! | 内联样式 | **零**（设计文档 §9.2 C3） |
//! | `id` | **零**（设计文档 §9.2 C4） |
//!
//! ## 条件分支
//!
//! 既有实现用 `{#if}` 表达的条件块，本版用 `view!` 的 `if` 块 + `class:token`
//! 条件 class 表达（同样的判定，不引入新的语义）。
//!
//! ## 时间的格式化
//!
//! 既有实现用 `toLocaleTimeString('zh-CN', {hour: '2-digit', minute: '2-digit'})`
//! ⇒ 本版用 [`format_clock_utc`] 产出**同样的字符串**（24 小时制、两位补零）。
//! 对照环境把浏览器时区钉成 UTC ⇒ 两侧取的是同一时刻的同一面钟。
//!
//! ## 复制态
//!
//! 既有实现的「已复制」是组件内的局部状态（`$state`）。本版把该状态放在
//! props 里（`copied` 信号）—— 组件是**纯函数** `fn(&Props) -> VNode`
//! （设计文档 §6.1），局部状态由**调用方**持有。

use apeireth_ui_core::{Callback, Signal, UiComponent};
use apeireth_ui_dom::{Children, ElementHandler, VNode};
use apeireth_ui_kit::icon_data::{
    ICON_CHECK, ICON_COPY, ICON_SHIELD_ALERT, ICON_SHIELD_CHECK, ICON_X,
};
use apeireth_ui_kit::{
    Badge, BadgeProps, Button, ButtonProps, CardShell, CardShellProps, Icon, IconProps,
};
use apeireth_ui_macro::{component, view};

/// 审批项（卡片的内容）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApprovalItem {
    /// 标题（缺省用「需要批准的操作」）。
    pub title: Option<String>,
    /// 命令文本。
    pub command_text: Option<String>,
    /// 参数摘要。
    pub arguments_summary: Option<String>,
    /// 审批原因。
    pub reason: Option<String>,
    /// 创建时刻（**毫秒**时间戳；`None` ⇒ 不显示时间）。
    pub created_at: Option<i64>,
    /// 沙箱摘要（`None` ⇒ 不显示沙箱行）。
    pub sandbox: Option<String>,
    /// 冻结工作目录。
    pub cwd: Option<String>,
    /// 隔离态摘要。
    pub isolation: Option<String>,
}

impl ApprovalItem {
    /// 沙箱摘要里含「未沙箱」⇒ 灰色警示外观（既有实现的判定口径）。
    fn is_unsandboxed(&self) -> bool {
        self.sandbox
            .as_deref()
            .map(|text| text.contains("未沙箱"))
            .unwrap_or(false)
    }
}

/// 格式化时刻（UTC，24 小时制，两位补零）。
///
/// 与既有实现 `toLocaleTimeString('zh-CN', {hour: '2-digit', minute: '2-digit'})`
/// 在 **UTC 时区**下产出同样的字符串（例如 `01:10`）。
///
/// 纯算术（不读系统时区）⇒ 对照环境把浏览器时区钉成 UTC 时两侧必然一致。
pub fn format_clock_utc(epoch_millis: i64) -> String {
    let total_seconds = epoch_millis.div_euclid(1000);
    let seconds_of_day = total_seconds.rem_euclid(86_400);
    let minutes = seconds_of_day / 60;
    let hours = minutes / 60;
    format!("{:02}:{:02}", hours, minutes % 60)
}

/// 审批卡组件（用 `#[component]` 宏声明：**函数即组件**）。
///
/// 宏展开后等价于：props 结构体 `ApprovalCardProps` + 组件类型
/// `ApprovalCard` + `UiComponent` 实现（设计文档 §6.1）。
#[component]
pub fn ApprovalCard(
    /// 卡片内容。
    item: ApprovalItem,
    /// 处理中（禁用按钮 + 换文案）。
    #[prop(default = false)]
    busy: bool,
    /// 「已复制」状态（调用方持有的信号）。
    copied: Signal<bool>,
    /// 允许。
    on_allow: Callback,
    /// 拒绝。
    on_reject: Callback,
    /// 关闭（**只关卡片，不做业务决策**；缺省 ⇒ 不显示关闭按钮）。
    #[prop(default = None)]
    on_dismiss: Option<Callback>,
    /// 子内容插槽（试点组件不用；保留是为了证明插槽机制可用）。
    children: Children,
) -> VNode {
    // ── 头部 ────────────────────────────────────────────────────────────
    let title_text = item
        .title
        .clone()
        .unwrap_or_else(|| String::from("需要批准的操作"));
    let show_time = item.created_at.is_some();
    let clock = item
        .created_at
        .map(format_clock_utc)
        .unwrap_or_else(String::new);

    let close_button = on_dismiss.as_ref().map(|dismiss| {
        let dismiss = dismiss.clone();
        let handler = ElementHandler::new(move |_event| dismiss.call());
        Button::render(
            &ButtonProps::new("close-btn")
                .aria_label("关闭")
                .title("关闭（不拒绝）")
                .disabled(busy)
                .on_click(handler)
                .children(Children::new(|| Icon::render(&IconProps::new(&ICON_X, 16)))),
        )
    });

    // ── 主体 ────────────────────────────────────────────────────────────
    let mut sandbox_badge: Option<VNode> = None;
    if let Some(sandbox) = item.sandbox.clone() {
        let unsandboxed = item.is_unsandboxed();
        // 既有实现的文本节点带一个前导空格（模板里图标与文本之间的换行被
        // 折叠成一个空格）⇒ 这里逐字复现（否则 DOM 逐字比对会出现 D3 差异）。
        let text = format!(" {sandbox}");
        let icon = if unsandboxed {
            &ICON_SHIELD_ALERT
        } else {
            &ICON_SHIELD_CHECK
        };
        let mut props = BadgeProps::new("sandbox-badge")
            .title("该操作将在沙箱内执行：工作区限定 + 断网")
            .children(Children::new(move || {
                Icon::render(&IconProps::new(icon, 12))
            }));
        if unsandboxed {
            props = props.extra_class("unsandboxed");
        }
        sandbox_badge = Some(Badge::render(&props).with_text(text));
    }

    let isolation = item.isolation.clone();
    let cwd = item.cwd.clone();
    let command_text = item.command_text.clone();
    let copied_state = copied.clone();
    let arguments_summary = item.arguments_summary.clone();
    let reason = item.reason.clone();

    // ── 底部 ────────────────────────────────────────────────────────────
    let reject_handler = ElementHandler::new(move |_event| on_reject.call());
    let allow_handler = ElementHandler::new(move |_event| on_allow.call());
    let reject_button = Button::render(
        &ButtonProps::new("reject-btn")
            .label("拒绝")
            .disabled(busy)
            .on_click(reject_handler),
    );
    let allow_button = Button::render(
        &ButtonProps::new("allow-btn")
            .label(if busy { "处理中…" } else { "批准" })
            .disabled(busy)
            .on_click(allow_handler),
    );

    let body = view! {
        div(class = "card-header") {
            div(class = "title-wrap") {
                h3(class = "title") { title_text }
                if show_time {
                    span(class = "time") { clock }
                }
            }
            if close_button.is_some() {
                { { close_button.clone().unwrap_or_else(|| VNode::fragment(Vec::new())) } }
            }
        }
        div(class = "card-body") {
            if sandbox_badge.is_some() {
                div(class = "sandbox-row") {
                    { { sandbox_badge.clone().unwrap_or_else(|| VNode::fragment(Vec::new())) } }
                    if isolation.is_some() {
                        span(class = "sandbox-meta") { isolation.clone().unwrap_or_default() }
                    }
                }
                if cwd.is_some() {
                    div(class = "section") {
                        span(class = "section-label") { "工作目录（已冻结）" }
                        p(class = "section-text mono-path", title = cwd.clone().unwrap_or_default()) {
                            cwd.clone().unwrap_or_default()
                        }
                    }
                }
            }
            if command_text.is_some() {
                div(class = "command-row") {
                    code(class = "command") { command_text.clone().unwrap_or_default() }
                    { { copy_button(copied_state.clone(), busy) } }
                }
            }
            if arguments_summary.is_some() {
                div(class = "section") {
                    span(class = "section-label") { "参数摘要" }
                    p(class = "section-text") { arguments_summary.clone().unwrap_or_default() }
                }
            }
            if reason.is_some() {
                div(class = "section") {
                    span(class = "section-label") { "审批原因" }
                    p(class = "section-text reason") { reason.clone().unwrap_or_default() }
                }
            }
        }
        div(class = "card-footer") {
            { { reject_button } }
            { { allow_button } }
        }
    };

    // 卡片壳（class 名 / 角色 / 无障碍标签来自既有实现）。
    CardShell::render(
        &CardShellProps::new("approval-card")
            .role("group")
            .aria_label("待批准操作")
            .children(Children::new(move || body.clone())),
    )
}

/// 复制按钮（复制态 ⇒ 图标切换 + 条件 class）。
fn copy_button(copied: Signal<bool>, busy: bool) -> VNode {
    let copied_flag = copied.clone();
    let handler = ElementHandler::new(move |_event| copied_flag.set(true));
    let copied_state = copied.clone();
    Button::render(
        &ButtonProps::new("copy-btn")
            .conditional_class("copied", copied_state.get())
            .aria_label("复制命令")
            .title("复制命令")
            .disabled(busy)
            .on_click(handler)
            .children(Children::new(move || {
                if copied_state.get() {
                    Icon::render(&IconProps::new(&ICON_CHECK, 14))
                } else {
                    Icon::render(&IconProps::new(&ICON_COPY, 14))
                }
            })),
    )
}

/// 给元素追加一个文本子节点（既有实现的文本节点逐字复现用）。
trait WithText {
    /// 追加文本子节点。
    fn with_text(self, text: String) -> Self;
}

impl WithText for VNode {
    fn with_text(self, text: String) -> Self {
        match self {
            VNode::Element(mut element) => {
                element.children.push(VNode::text(text));
                VNode::Element(element)
            }
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    //! 试点组件的**行为**断言（渲染结果 ⇒ 结构 / class / 文本），不读源码。

    use super::*;
    use apeireth_ui_core::{Runtime, UiComponent};

    /// 建一份试点 props（`item` 可替换）。
    fn props_with(item: ApprovalItem) -> ApprovalCardProps {
        let runtime = Runtime::new();
        ApprovalCardProps {
            item,
            busy: false,
            copied: runtime.signal(false),
            on_allow: Callback::new(|| {}),
            on_reject: Callback::new(|| {}),
            on_dismiss: None,
            children: Children::default(),
        }
    }

    /// 渲染后的文本节点列表（逐个）。
    fn text_nodes(node: &VNode) -> Vec<String> {
        fn walk(node: &VNode, out: &mut Vec<String>) {
            match node {
                VNode::Element(element) => {
                    for child in &element.children {
                        walk(child, out);
                    }
                }
                VNode::Text(text) => out.push(text.data.clone()),
                VNode::Fragment(children) => {
                    for child in children {
                        walk(child, out);
                    }
                }
            }
        }
        let mut out = Vec::new();
        walk(node, &mut out);
        out
    }

    /// 渲染后的文本（扁平）。
    fn flat_text(node: &VNode) -> String {
        fn walk(node: &VNode, out: &mut String) {
            match node {
                VNode::Element(element) => {
                    for child in &element.children {
                        walk(child, out);
                    }
                }
                VNode::Text(text) => out.push_str(&text.data),
                VNode::Fragment(children) => {
                    for child in children {
                        walk(child, out);
                    }
                }
            }
        }
        let mut out = String::new();
        walk(node, &mut out);
        out
    }

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

    #[test]
    fn clock_is_two_digit_utc_hour_and_minute() {
        assert_eq!(format_clock_utc(1_761_354_600_000), "01:10");
        assert_eq!(format_clock_utc(0), "00:00");
        // 跨日回绕（负时间戳 ⇒ 取 UTC 日内时刻）。
        assert_eq!(format_clock_utc(-1_000), "23:59");
    }

    #[test]
    fn the_default_title_is_used_when_the_item_has_none() {
        let node = ApprovalCard::render(&props_with(ApprovalItem::default()));
        assert!(flat_text(&node).contains("需要批准的操作"));
    }

    #[test]
    fn the_time_row_is_absent_without_a_creation_timestamp() {
        let node = ApprovalCard::render(&props_with(ApprovalItem::default()));
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(
            !found.iter().any(|token| token == "time"),
            "没有创建时刻 ⇒ 不出现 `time` class：{found:?}"
        );
    }

    #[test]
    fn the_time_row_is_present_with_a_creation_timestamp() {
        let item = ApprovalItem {
            created_at: Some(1_761_354_600_000),
            ..ApprovalItem::default()
        };
        let node = ApprovalCard::render(&props_with(item));
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(found.iter().any(|token| token == "time"));
        assert!(flat_text(&node).contains("01:10"));
    }

    #[test]
    fn the_unsandboxed_variant_adds_the_conditional_class() {
        let item = ApprovalItem {
            sandbox: Some("未沙箱".to_owned()),
            ..ApprovalItem::default()
        };
        let node = ApprovalCard::render(&props_with(item));
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(
            found.iter().any(|token| token == "unsandboxed"),
            "「未沙箱」形态 ⇒ 追加 `unsandboxed`：{found:?}"
        );
    }

    #[test]
    fn the_sandboxed_variant_has_no_conditional_class() {
        let item = ApprovalItem {
            sandbox: Some("工作区沙箱".to_owned()),
            ..ApprovalItem::default()
        };
        let node = ApprovalCard::render(&props_with(item));
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(
            !found.iter().any(|token| token == "unsandboxed"),
            "「有墙」形态 ⇒ 不追加 `unsandboxed`：{found:?}"
        );
    }

    #[test]
    fn optional_sections_are_absent_when_their_data_is_missing() {
        let node = ApprovalCard::render(&props_with(ApprovalItem::default()));
        let mut found = Vec::new();
        classes(&node, &mut found);
        for absent in ["sandbox-row", "command-row", "section", "reason"] {
            assert!(
                !found.iter().any(|token| token == absent),
                "`{absent}` 不该出现：{found:?}"
            );
        }
    }

    #[test]
    fn the_busy_state_changes_the_confirm_label_and_disables_the_buttons() {
        let mut props = props_with(ApprovalItem::default());
        props.busy = true;
        let node = ApprovalCard::render(&props);
        let texts = text_nodes(&node);
        assert!(
            texts.iter().any(|text| text == "处理中…"),
            "处理中 ⇒ 文案切换：{texts:?}"
        );
        assert!(
            !texts.iter().any(|text| text == "批准"),
            "处理中 ⇒ 不显示「批准」：{texts:?}"
        );
    }
}
