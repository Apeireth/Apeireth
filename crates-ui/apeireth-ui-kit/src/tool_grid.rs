//! 卡面组件：**工具栏**与**工具网格**（工具卡面批二）。
//!
//! ## 面边界（这一面只回答「长什么样」）
//!
//! | 在本模块 | 不在本模块 |
//! |---|---|
//! | 工具栏的结构 / class 名 / 文本 | 数据拉取（HTTP / SSE 契约） |
//! | 工具网格的结构 / class 名 / 文本 | 能力门控（能力清单判定） |
//! | 搜索词对清单的**过滤口径** | 空态 / 加载态 / 错误态（状态条面） |
//! | 状态徽标的外观 | 详情弹窗 / 授权卷宗（治理卷宗面） |
//!
//! ⇒ 数据（工具清单、搜索词）由**宿主注入**（`Props`）；引擎不碰数据面与
//! 能力面（设计文档 §1.3 第 6 条）。
//!
//! ## class 名是视觉契约（设计文档 §9.1）
//!
//! class 名与生产实现**逐字一致**（含 `.status-badge {variant} {size}` 这一组）
//! ⇒ **不改名**（改名 ⇒ 像素不一致）。生产的状态徽标是「class 名 + 文本」的
//! 通用形态 ⇒ 本模块用组件库既有的 [`Badge`] 表达，**不另造同名组件**。
//!
//! ## 空白
//!
//! 生产组件模板里相邻元素之间的换行会折叠成**一个空格**（文本节点）⇒ 本模块
//! **逐个复现**这些空白文本节点（否则 DOM 逐字比对会出现差异）。
//!
//! 文本节点的**确切内容**以生产组件的真实渲染结果为准（浏览器实测 dump），
//! 不靠推理。

use crate::badge::{Badge, BadgeProps};
use crate::icon::{Icon, IconProps};
use crate::icons_tool_grid::{ICON_CPU, ICON_EXTERNAL_LINK, ICON_SEARCH, ICON_X};
use apeireth_ui_core::{Signal, UiComponent};
use apeireth_ui_dom::{Children, ElementHandler, VNode};
use apeireth_ui_macro::view;

/// 描述缺省时的兜底文案（与生产实现**逐字一致**）。
pub const TOOL_DESCRIPTION_FALLBACK: &str = "内置系统工具，供 Agent 自主决策调用。";

/// 工具摘要（**中性数据形状**；宿主注入）。
///
/// 刻意**不**照搬生产的工具条目类型（那带来源 / 权限 / 参数规范等业务字段）：
/// 本面只渲染「名称 / 描述 / 是否可用」三样。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ToolSummary {
    /// 名称。
    pub name: String,
    /// 描述（**缺省或空串 ⇒ 用兜底文案**）。
    pub description: Option<String>,
    /// 是否可用。
    pub available: bool,
}

impl ToolSummary {
    /// 建一条工具摘要。
    pub fn new(name: impl Into<String>, description: Option<String>, available: bool) -> Self {
        Self {
            name: name.into(),
            description,
            available,
        }
    }

    /// 渲染用的描述文本（缺省或空串 ⇒ 兜底文案）。
    pub fn description_text(&self) -> String {
        match &self.description {
            Some(text) if !text.is_empty() => text.clone(),
            _ => String::from(TOOL_DESCRIPTION_FALLBACK),
        }
    }
}

/// 搜索词为空时的**全部命中**口径：名称或描述包含该词（**忽略大小写**）。
///
/// 口径与生产实现一致：先 `trim` 再转小写；描述**为空**时不算命中。
pub fn filter_tools(tools: &[ToolSummary], query: &str) -> Vec<ToolSummary> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return tools.to_vec();
    }
    tools
        .iter()
        .filter(|tool| {
            tool.name.to_lowercase().contains(&needle)
                || tool
                    .description
                    .as_deref()
                    .is_some_and(|text| !text.is_empty() && text.to_lowercase().contains(&needle))
        })
        .cloned()
        .collect()
}

/// 工具栏的 props。
#[derive(Debug, Clone)]
pub struct ToolToolbarProps {
    /// 已装配的工具数（摘要里显示）。
    pub tool_count: usize,
    /// 搜索词（**调用方持有**的信号 ⇒ 组件是纯函数）。
    pub query: Signal<String>,
}

impl ToolToolbarProps {
    /// 建一组工具栏 props。
    pub fn new(tool_count: usize, query: Signal<String>) -> Self {
        Self { tool_count, query }
    }
}

/// 工具栏组件（搜索框 + 摘要元信息）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ToolToolbar;

impl UiComponent for ToolToolbar {
    type Props = ToolToolbarProps;
    type Output = VNode;

    fn render(props: &Self::Props) -> Self::Output {
        let query = props.query.get();
        let has_query = !query.is_empty();

        let search_icon =
            Icon::render(&IconProps::new(&ICON_SEARCH, 14).extra_class("search-icon"));
        let clear_icon = Icon::render(&IconProps::new(&ICON_X, 12));
        // 清空按钮 ⇒ 搜索词置空（信号由调用方持有 ⇒ 组件保持纯函数）。
        let clear_query = props.query.clone();
        let count_text = props.tool_count.to_string();

        view! {
            div(class = "tools-toolbar") {
                div(class = "search-input-wrap") {
                    { search_icon }
                    " "
                    input(type = "text", placeholder = "搜索工具名称或描述…")
                    " "
                    if has_query {
                        button(
                            class = "clear-search-btn",
                            onclick = move |_| { clear_query.set(String::new()); },
                            aria-label = "清除搜索"
                        ) {
                            { clear_icon }
                        }
                    }
                }
                " "
                div(class = "tools-summary-meta") {
                    span {
                        "已装配工具: "
                        b { count_text }
                        " 个"
                    }
                }
            }
        }
    }
}

/// 工具网格的 props。
#[derive(Debug, Clone)]
pub struct ToolGridProps {
    /// 工具清单（**宿主注入**；已按搜索词过滤）。
    pub tools: Vec<ToolSummary>,
    /// 点卡片 / 点「参数规范」⇒ 打开详情（**宿主可选接线**）。
    ///
    /// 详情弹窗属治理卷宗面 ⇒ 本批**不迁** ⇒ 未接线时点击**无副作用**
    /// （不假装已接线）。
    pub on_open: Option<ElementHandler>,
}

impl ToolGridProps {
    /// 建一组工具网格 props（详情未接线）。
    pub fn new(tools: Vec<ToolSummary>) -> Self {
        Self {
            tools,
            on_open: None,
        }
    }

    /// 接上「打开详情」处理器（builder 风格）。
    pub fn on_open(mut self, on_open: ElementHandler) -> Self {
        self.on_open = Some(on_open);
        self
    }
}

/// 工具网格组件（工具卡片列表）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ToolGrid;

impl UiComponent for ToolGrid {
    type Props = ToolGridProps;
    type Output = VNode;

    fn render(props: &Self::Props) -> Self::Output {
        let cards: Vec<VNode> = props
            .tools
            .iter()
            .map(|tool| render_card(tool, props.on_open.clone()))
            .collect();

        view! {
            div(class = "tools-container") {
                div(class = "tools-grid") {
                    for card in cards {
                        { card }
                    }
                }
            }
        }
    }
}

/// 渲染一张工具卡片。
fn render_card(tool: &ToolSummary, on_open: Option<ElementHandler>) -> VNode {
    let cpu_icon = Icon::render(&IconProps::new(&ICON_CPU, 15));
    let link_icon = Icon::render(&IconProps::new(&ICON_EXTERNAL_LINK, 11));

    // 状态徽标：外观 = class 名 + 文本（生产用通用徽标组件 ⇒ 这里用组件库既有的
    // `Badge` 表达，class 名逐字照抄生产）。
    let (status_label, status_class) = if tool.available {
        ("可用", "status-badge green small")
    } else {
        ("不可用", "status-badge neutral small")
    };
    let badge = Badge::render(
        &BadgeProps::new(status_class)
            .children(Children::new(move || VNode::text(status_label.to_owned()))),
    );

    let name_text = tool.name.clone();
    let description_text = tool.description_text();
    let source_text = String::from("内置能力");
    let details_text = String::from("参数规范");

    // 点卡片 ⇒ 打开详情（宿主可选接线）。
    let card_open = on_open.clone();
    let card_key = on_open.clone();
    // 点「参数规范」⇒ 打开详情（并**止住冒泡**，与生产一致）。
    let link_open = on_open;

    view! {
        div(
            class = "tool-card-box",
            role = "button",
            tabindex = "0",
            onclick = move |event| {
                if let Some(handler) = &card_open {
                    handler.call(event);
                }
            },
            onkeydown = move |event| {
                if event.key() == Some("Enter") {
                    if let Some(handler) = &card_key {
                        handler.call(event);
                    }
                }
            }
        ) {
            div(class = "card-head") {
                div(class = "tool-id-row") {
                    span(class = "tool-icon-box") {
                        { cpu_icon }
                    }
                    " "
                    strong(class = "tool-title") { name_text }
                }
                " "
                { badge }
            }
            " "
            p(class = "tool-desc") { description_text }
            " "
            div(class = "card-foot") {
                span(class = "source-pill") { source_text }
                " "
                button(
                    class = "details-link",
                    onclick = move |event| {
                        event.stop_propagation();
                        if let Some(handler) = &link_open {
                            handler.call(event);
                        }
                    }
                ) {
                    span { details_text }
                    " "
                    { link_icon }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //! 工具栏与工具网格的**行为**断言（渲染结果 ⇒ 结构 / class / 文本），不读源码。

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

    /// 建一组工具栏 props。
    fn toolbar_props(tool_count: usize, query: &str) -> ToolToolbarProps {
        let runtime = Runtime::new();
        ToolToolbarProps::new(tool_count, runtime.signal(String::from(query)))
    }

    /// 一条最小工具摘要。
    fn tool(name: &str, available: bool) -> ToolSummary {
        ToolSummary::new(name, None, available)
    }

    #[test]
    fn the_toolbar_renders_the_production_class_names() {
        let node = ToolToolbar::render(&toolbar_props(3, ""));
        let mut found = Vec::new();
        classes(&node, &mut found);
        for class in ["tools-toolbar", "search-input-wrap", "tools-summary-meta"] {
            assert!(
                found.iter().any(|token| token == class),
                "{class}：{found:?}"
            );
        }
    }

    #[test]
    fn the_toolbar_shows_the_tool_count_in_the_summary() {
        let node = ToolToolbar::render(&toolbar_props(6, ""));
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(texts.iter().any(|text| text == "已装配工具: "), "{texts:?}");
        assert!(texts.iter().any(|text| text == "6"), "{texts:?}");
        assert!(texts.iter().any(|text| text == " 个"), "{texts:?}");
    }

    #[test]
    fn the_clear_button_appears_only_when_the_query_is_not_empty() {
        let node = ToolToolbar::render(&toolbar_props(6, ""));
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(
            !found.iter().any(|token| token == "clear-search-btn"),
            "搜索词为空 ⇒ 不出现清除按钮：{found:?}"
        );
    }

    #[test]
    fn the_clear_button_appears_when_the_query_is_not_empty() {
        let node = ToolToolbar::render(&toolbar_props(6, "fs"));
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(
            found.iter().any(|token| token == "clear-search-btn"),
            "搜索词非空 ⇒ 出现清除按钮：{found:?}"
        );
    }

    #[test]
    fn the_clear_button_clears_the_query_signal() {
        let runtime = Runtime::new();
        let query = runtime.signal(String::from("fs"));
        let node = ToolToolbar::render(&ToolToolbarProps::new(6, query.clone()));
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(found.iter().any(|token| token == "clear-search-btn"));
        // 组件是纯函数：渲染本身不改状态。
        assert_eq!(query.get(), "fs");
    }

    #[test]
    fn the_search_input_keeps_the_production_attributes() {
        let node = ToolToolbar::render(&toolbar_props(6, ""));
        let mut found_input = false;
        fn walk(node: &VNode, found: &mut bool) {
            if node.tag() == Some("input") {
                *found = true;
                assert_eq!(
                    node.attr_value("type"),
                    Some(&apeireth_ui_dom::AttrValue::Text(String::from("text")))
                );
                assert_eq!(
                    node.attr_value("placeholder"),
                    Some(&apeireth_ui_dom::AttrValue::Text(String::from(
                        "搜索工具名称或描述…"
                    )))
                );
            }
            for child in node.children() {
                walk(child, found);
            }
        }
        walk(&node, &mut found_input);
        assert!(found_input, "工具栏里没有搜索框");
    }

    #[test]
    fn the_grid_renders_the_production_class_names() {
        let node = ToolGrid::render(&ToolGridProps::new(vec![tool("fs.read", true)]));
        let mut found = Vec::new();
        classes(&node, &mut found);
        for class in [
            "tools-container",
            "tools-grid",
            "tool-card-box",
            "card-head",
            "tool-id-row",
            "tool-icon-box",
            "tool-title",
            "tool-desc",
            "card-foot",
            "source-pill",
            "details-link",
        ] {
            assert!(
                found.iter().any(|token| token == class),
                "{class}：{found:?}"
            );
        }
    }

    #[test]
    fn the_card_is_a_button_with_the_production_role_and_tabindex() {
        let node = ToolGrid::render(&ToolGridProps::new(vec![tool("fs.read", true)]));
        fn walk(node: &VNode, found: &mut bool) {
            if node
                .class_tokens()
                .iter()
                .any(|token| token == "tool-card-box")
            {
                *found = true;
                assert_eq!(
                    node.attr_value("role"),
                    Some(&apeireth_ui_dom::AttrValue::Text(String::from("button")))
                );
                assert_eq!(
                    node.attr_value("tabindex"),
                    Some(&apeireth_ui_dom::AttrValue::Text(String::from("0")))
                );
            }
            for child in node.children() {
                walk(child, found);
            }
        }
        let mut found = false;
        walk(&node, &mut found);
        assert!(found, "没有工具卡片");
    }

    #[test]
    fn the_available_tool_shows_the_green_badge() {
        let node = ToolGrid::render(&ToolGridProps::new(vec![tool("fs.read", true)]));
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(
            found.iter().any(|token| token == "status-badge")
                && found.iter().any(|token| token == "green")
                && found.iter().any(|token| token == "small"),
            "{found:?}"
        );
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(texts.iter().any(|text| text == "可用"), "{texts:?}");
    }

    #[test]
    fn the_unavailable_tool_shows_the_neutral_badge() {
        let node = ToolGrid::render(&ToolGridProps::new(vec![tool("fs.read", false)]));
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(
            found.iter().any(|token| token == "neutral"),
            "不可用 ⇒ 中性外观：{found:?}"
        );
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(texts.iter().any(|text| text == "不可用"), "{texts:?}");
    }

    #[test]
    fn the_missing_description_falls_back_to_the_default_text() {
        let node = ToolGrid::render(&ToolGridProps::new(vec![tool("fs.read", true)]));
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(
            texts.iter().any(|text| text == TOOL_DESCRIPTION_FALLBACK),
            "{texts:?}"
        );
    }

    #[test]
    fn the_empty_description_falls_back_to_the_default_text() {
        let summary = ToolSummary::new("fs.read", Some(String::new()), true);
        let node = ToolGrid::render(&ToolGridProps::new(vec![summary]));
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(
            texts.iter().any(|text| text == TOOL_DESCRIPTION_FALLBACK),
            "空串描述 ⇒ 回落兜底文案：{texts:?}"
        );
    }

    #[test]
    fn the_given_description_is_used_as_is() {
        let summary = ToolSummary::new("fs.read", Some(String::from("读文件。")), true);
        let node = ToolGrid::render(&ToolGridProps::new(vec![summary]));
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(texts.iter().any(|text| text == "读文件。"), "{texts:?}");
    }

    #[test]
    fn the_grid_renders_one_card_per_tool() {
        let node = ToolGrid::render(&ToolGridProps::new(vec![
            tool("fs.read", true),
            tool("fs.write", true),
            tool("web.fetch", false),
        ]));
        let mut count = 0usize;
        fn walk(node: &VNode, count: &mut usize) {
            if node
                .class_tokens()
                .iter()
                .any(|token| token == "tool-card-box")
            {
                *count += 1;
            }
            for child in node.children() {
                walk(child, count);
            }
        }
        walk(&node, &mut count);
        assert_eq!(count, 3);
    }

    #[test]
    fn the_empty_query_keeps_every_tool() {
        let tools = vec![tool("fs.read", true), tool("web.fetch", false)];
        let filtered = filter_tools(&tools, "   ");
        assert_eq!(filtered.len(), 2, "空搜索词 ⇒ 全部命中");
    }

    #[test]
    fn the_query_matches_the_name_case_insensitively() {
        let tools = vec![tool("fs.read", true), tool("web.fetch", false)];
        let filtered = filter_tools(&tools, "FS");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "fs.read");
    }

    #[test]
    fn the_query_matches_the_description() {
        let tools = vec![
            ToolSummary::new("a.one", Some(String::from("读取文件")), true),
            ToolSummary::new("b.two", Some(String::from("写文件")), true),
        ];
        let filtered = filter_tools(&tools, "读取");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "a.one");
    }

    #[test]
    fn the_query_skips_tools_without_description() {
        let tools = vec![tool("a.one", true)];
        let filtered = filter_tools(&tools, "读取");
        assert!(filtered.is_empty(), "无描述 ⇒ 不命中：{filtered:?}");
    }

    #[test]
    fn the_query_trims_the_spaces() {
        let tools = vec![tool("fs.read", true)];
        let filtered = filter_tools(&tools, "  fs  ");
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn the_no_match_query_returns_nothing() {
        let tools = vec![tool("fs.read", true)];
        let filtered = filter_tools(&tools, "无此工具");
        assert!(filtered.is_empty(), "{filtered:?}");
    }
}
