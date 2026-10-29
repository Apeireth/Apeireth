//! 状态条面：**状态条**与**状态徽标**。
//!
//! ## 两件东西的边界（**刻意不合并**）
//!
//! | | 是什么 | 不是什么 |
//! |---|---|---|
//! | [`StatusBar`] | 一行若干格的**底栏**（每格 = 一段小字 + 可选的 `SIM` 标注） | 不是面板：点击直达动作，**不展开浮层** |
//! | [`StatusBadge`] | 一枚**圆角小徽标**（文案 + 变体 + 尺寸） | 不是状态条的一格：它是**独立组件**（生产侧亦然） |
//!
//! ## 「行为」的口径（逐条照抄生产组件，**这些是行为不是样式**）
//!
//! | 判定 | 口径 |
//! |---|---|
//! | 一格渲染成 `button` 还是 `span` | **只看** `indicator.clickable`（真 ⇒ `button`；否则 ⇒ `span`） |
//! | 一格的 class | `sb-item` + `tone-{色态}`（**两个 token**） |
//! | `SIM` 标注是否出现 | `indicator.sim` 为**真**才出现（缺省 ⇒ 不出现） |
//! | `SIM` 标注的文本 | 固定 `SIM`（**不是**数据） |
//! | 状态条的根 | `footer` + `status-bar` + `aria-label="状态条"` |
//! | 一格的 `title` | `indicator.title`（**原样**，不裁剪） |
//! | 徽标的 class | `status-badge` + 变体 token + 尺寸 token（**三个 token**） |
//! | 徽标的变体缺省 | `neutral` |
//! | 徽标的尺寸缺省 | `normal` |
//!
//! ## 空白
//!
//! 本组件**没有**空白文本节点：生产模板里 `{#each}` 的每一轮只产出一个元素，
//! 格内的文本与 `SIM` 徽标**紧邻**（中间无换行）⇒ 渲染结果里也没有折叠出的
//! 空格文本节点。（与卡面组件不同：那里相邻元素之间有换行 ⇒ 有空格节点。）
//!
//! ## 中性纪律
//!
//! 组件**不认识**任何业务来源（连接态 / 回合 / 计数 / 事件源 …）：它只渲染
//! 「一段小字 + 一个色态 + 一句悬停解释 + 可不可点」⇒ 业务语义归调用方
//! （`statusbar.ts` 是**数据面**，不属本组件）。

use apeireth_ui_core::{Callback, UiComponent};
use apeireth_ui_dom::VNode;
use apeireth_ui_macro::view;

/// 状态条一格的色态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusTone {
    /// 安静（常态；不加语义色）。
    #[default]
    Quiet,
    /// 待完善（黄）。
    Warn,
    /// 危险（红）。
    Danger,
}

impl StatusTone {
    /// 外观 class token（一格的 class = `sb-item` + 本 token）。
    pub fn class_token(self) -> &'static str {
        match self {
            StatusTone::Quiet => "tone-quiet",
            StatusTone::Warn => "tone-warn",
            StatusTone::Danger => "tone-danger",
        }
    }
}

/// 状态条上的一格的数据（**中性数据形状**）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StatusIndicator {
    /// 条上小字。
    pub text: String,
    /// 色态。
    pub tone: StatusTone,
    /// 悬停完整解释（**原样**渲染，不裁剪）。
    pub title: String,
    /// 可点 = 是入口（真 ⇒ 渲染成 `button`；否则 ⇒ 渲染成 `span`）。
    pub clickable: bool,
    /// 「模拟值」标注（真 ⇒ 追加 `SIM` 徽标）。
    pub sim: bool,
}

/// 状态条上的一格（数据 + 列表身份 + 点击回调）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StatusBarItem {
    /// 列表身份（**不出现在 DOM 里**；供 keyed 差分用）。
    pub key: String,
    /// 该格的数据。
    pub indicator: StatusIndicator,
    /// 点击回调。
    ///
    /// - `Some` ⇒ 点击时调用它；
    /// - `None` ⇒ 点击**无动作**（= 未接线；**不静默假装点击成功**）。
    ///
    /// 回调**只在 `clickable` 为真时挂上**（不可点的格渲染成 `span`，与生产
    /// 组件同构）⇒ `None` 的回调与「不可点」在 DOM 上**逐字相同**。
    pub on_click: Option<Callback>,
}

/// 状态条的 props。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StatusBarProps {
    /// 条上的各格（**顺序即渲染顺序**）。
    pub items: Vec<StatusBarItem>,
}

impl StatusBarProps {
    /// 建一组 props。
    pub fn new(items: Vec<StatusBarItem>) -> Self {
        Self { items }
    }
}

/// 状态条组件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StatusBar;

impl UiComponent for StatusBar {
    type Props = StatusBarProps;
    type Output = VNode;

    fn render(props: &Self::Props) -> Self::Output {
        let items = props.items.clone();
        view! {
            footer(class = "status-bar", aria-label = "状态条") {
                for item in items {
                    if item.indicator.clickable {
                        button(
                            key = item.key.clone(),
                            class = "sb-item",
                            class = item.indicator.tone.class_token(),
                            title = item.indicator.title.clone(),
                            onclick = {
                                let callback = item.on_click.clone();
                                move |_| {
                                    if let Some(callback) = callback.as_ref() {
                                        callback.call();
                                    }
                                }
                            }
                        ) {
                            item.indicator.text.clone()
                            if item.indicator.sim {
                                span(class = "sim-badge") { "SIM" }
                            }
                        }
                    } else {
                        span(
                            key = item.key.clone(),
                            class = "sb-item",
                            class = item.indicator.tone.class_token(),
                            title = item.indicator.title.clone()
                        ) {
                            item.indicator.text.clone()
                            if item.indicator.sim {
                                span(class = "sim-badge") { "SIM" }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// 状态徽标的变体。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusBadgeVariant {
    /// 琥珀（黄）。
    Amber,
    /// 绿。
    Green,
    /// 蓝。
    Blue,
    /// 红（危险）。
    Danger,
    /// 中性（**缺省**）。
    #[default]
    Neutral,
    /// 暗（弱化）。
    Dim,
}

impl StatusBadgeVariant {
    /// 外观 class token。
    pub fn class_token(self) -> &'static str {
        match self {
            StatusBadgeVariant::Amber => "amber",
            StatusBadgeVariant::Green => "green",
            StatusBadgeVariant::Blue => "blue",
            StatusBadgeVariant::Danger => "danger",
            StatusBadgeVariant::Neutral => "neutral",
            StatusBadgeVariant::Dim => "dim",
        }
    }
}

/// 状态徽标的尺寸。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusBadgeSize {
    /// 小号。
    Small,
    /// 正常（**缺省**）。
    #[default]
    Normal,
}

impl StatusBadgeSize {
    /// 外观 class token。
    pub fn class_token(self) -> &'static str {
        match self {
            StatusBadgeSize::Small => "small",
            StatusBadgeSize::Normal => "normal",
        }
    }
}

/// 状态徽标的 props。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StatusBadgeProps {
    /// 变体（缺省 `neutral`）。
    pub variant: StatusBadgeVariant,
    /// 文案。
    pub label: String,
    /// 尺寸（缺省 `normal`）。
    pub size: StatusBadgeSize,
}

impl StatusBadgeProps {
    /// 建一组 props（变体 / 尺寸取缺省）。
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            variant: StatusBadgeVariant::default(),
            label: label.into(),
            size: StatusBadgeSize::default(),
        }
    }
}

/// 状态徽标组件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StatusBadge;

impl UiComponent for StatusBadge {
    type Props = StatusBadgeProps;
    type Output = VNode;

    fn render(props: &Self::Props) -> Self::Output {
        view! {
            span(
                class = "status-badge",
                class = props.variant.class_token(),
                class = props.size.class_token()
            ) {
                props.label.clone()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //! 状态条面的**行为**断言（渲染结果 ⇒ 结构 / class / 文本），不读源码。

    use super::*;
    use apeireth_ui_core::Runtime;
    use apeireth_ui_dom::{DomEvent, ElementVNode, EventKind};
    use std::cell::Cell;
    use std::rc::Rc;

    /// 收集 class token（保序去重）。
    fn classes(node: &VNode, out: &mut Vec<String>) {
        for token in node.class_tokens() {
            if !out.contains(&token) {
                out.push(token.clone());
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

    /// 收集标签名（保序）。
    fn tags(node: &VNode, out: &mut Vec<String>) {
        if let Some(tag) = node.tag() {
            out.push(tag.to_owned());
        }
        for child in node.children() {
            tags(child, out);
        }
    }

    /// 找第一个给定标签的元素。
    fn find_element<'a>(node: &'a VNode, tag: &str) -> Option<&'a ElementVNode> {
        match node {
            VNode::Element(element) => {
                if element.tag == tag {
                    return Some(element);
                }
                element
                    .children
                    .iter()
                    .find_map(|child| find_element(child, tag))
            }
            VNode::Fragment(children) => children.iter().find_map(|child| find_element(child, tag)),
            VNode::Text(_) => None,
        }
    }

    /// 建一格。
    fn item(key: &str, tone: StatusTone, clickable: bool, sim: bool) -> StatusBarItem {
        StatusBarItem {
            key: String::from(key),
            indicator: StatusIndicator {
                text: format!("文本-{key}"),
                tone,
                title: format!("解释-{key}"),
                clickable,
                sim,
            },
            on_click: None,
        }
    }

    /// 渲染一条状态条。
    fn render_bar(items: Vec<StatusBarItem>) -> VNode {
        StatusBar::render(&StatusBarProps::new(items))
    }

    #[test]
    fn the_bar_root_is_a_footer_with_the_aria_label() {
        let node = render_bar(vec![item("a", StatusTone::Quiet, false, false)]);
        assert_eq!(node.tag(), Some("footer"));
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(found.iter().any(|token| token == "status-bar"), "{found:?}");
        assert_eq!(
            node.attr_value("aria-label")
                .and_then(|value| value.to_attr_string()),
            Some(String::from("状态条"))
        );
    }

    #[test]
    fn a_clickable_item_renders_a_button() {
        let node = render_bar(vec![item("a", StatusTone::Quiet, true, false)]);
        let mut found = Vec::new();
        tags(&node, &mut found);
        assert!(found.iter().any(|tag| tag == "button"), "{found:?}");
    }

    #[test]
    fn a_non_clickable_item_renders_a_span() {
        let node = render_bar(vec![item("a", StatusTone::Quiet, false, false)]);
        let mut found = Vec::new();
        tags(&node, &mut found);
        assert!(!found.iter().any(|tag| tag == "button"), "{found:?}");
        assert!(found.iter().any(|tag| tag == "span"), "{found:?}");
    }

    #[test]
    fn the_item_carries_the_item_class_and_the_tone_token() {
        let node = render_bar(vec![item("a", StatusTone::Warn, false, false)]);
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(found.iter().any(|token| token == "sb-item"), "{found:?}");
        assert!(found.iter().any(|token| token == "tone-warn"), "{found:?}");
    }

    #[test]
    fn the_three_tones_have_their_own_class_token() {
        assert_eq!(StatusTone::Quiet.class_token(), "tone-quiet");
        assert_eq!(StatusTone::Warn.class_token(), "tone-warn");
        assert_eq!(StatusTone::Danger.class_token(), "tone-danger");
    }

    #[test]
    fn the_item_title_is_carried_verbatim() {
        let node = render_bar(vec![item("a", StatusTone::Quiet, false, false)]);
        let cell = find_element(&node, "span").expect("格（span）");
        assert_eq!(
            cell.attr_value("title")
                .and_then(|value| value.to_attr_string()),
            Some(String::from("解释-a"))
        );
    }

    #[test]
    fn the_item_text_is_the_only_text_node() {
        let node = render_bar(vec![item("a", StatusTone::Quiet, false, false)]);
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert_eq!(texts, vec![String::from("文本-a")]);
    }

    #[test]
    fn the_sim_badge_is_absent_when_the_flag_is_off() {
        let node = render_bar(vec![item("a", StatusTone::Quiet, false, false)]);
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(!found.iter().any(|token| token == "sim-badge"), "{found:?}");
    }

    #[test]
    fn the_sim_badge_appears_when_the_flag_is_on() {
        let node = render_bar(vec![item("a", StatusTone::Danger, false, true)]);
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(found.iter().any(|token| token == "sim-badge"), "{found:?}");
    }

    #[test]
    fn the_sim_badge_text_is_fixed() {
        let node = render_bar(vec![item("a", StatusTone::Danger, false, true)]);
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(texts.iter().any(|text| text == "SIM"), "{texts:?}");
    }

    #[test]
    fn the_items_keep_the_given_order() {
        let node = render_bar(vec![
            item("a", StatusTone::Quiet, false, false),
            item("b", StatusTone::Warn, false, false),
            item("c", StatusTone::Danger, false, false),
        ]);
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert_eq!(
            texts,
            vec![
                String::from("文本-a"),
                String::from("文本-b"),
                String::from("文本-c")
            ]
        );
    }

    #[test]
    fn the_click_handler_calls_the_callback() {
        let hits = Rc::new(Cell::new(0u32));
        let counter = hits.clone();
        let mut entry = item("a", StatusTone::Quiet, true, false);
        entry.on_click = Some(Callback::new(move || counter.set(counter.get() + 1)));
        let node = render_bar(vec![entry]);

        let button = find_element(&node, "button").expect("按钮");
        assert_eq!(button.handlers.len(), 1, "按钮上恰好一个监听器");
        let mut event = DomEvent::new(EventKind::Click, Vec::new());
        button.handlers[0].1.call(&mut event);
        assert_eq!(hits.get(), 1, "点击回调被调用");
    }

    #[test]
    fn a_clickable_item_without_a_callback_carries_an_inert_listener() {
        // 未接线（`on_click: None`）⇒ 按钮仍挂监听器，但**点击无动作**
        // （不静默假装点击成功；DOM 形态与挂了回调时逐字相同）。
        let node = render_bar(vec![item("a", StatusTone::Quiet, true, false)]);
        let button = find_element(&node, "button").expect("按钮");
        assert_eq!(button.handlers.len(), 1, "可点的格 ⇒ 恰好一个监听器");
        let mut event = DomEvent::new(EventKind::Click, Vec::new());
        button.handlers[0].1.call(&mut event);
        // 没有任何回调 ⇒ 调用它**不产生任何效果**（不 panic、不改状态）。
    }

    #[test]
    fn a_non_clickable_item_never_carries_a_listener() {
        let hits = Rc::new(Cell::new(0u32));
        let counter = hits.clone();
        let mut entry = item("a", StatusTone::Quiet, false, false);
        entry.on_click = Some(Callback::new(move || counter.set(counter.get() + 1)));
        let node = render_bar(vec![entry]);
        let mut found = Vec::new();
        tags(&node, &mut found);
        assert!(!found.iter().any(|tag| tag == "button"), "{found:?}");
        assert_eq!(hits.get(), 0);
    }

    #[test]
    fn the_badge_carries_three_class_tokens() {
        let props = StatusBadgeProps {
            variant: StatusBadgeVariant::Amber,
            label: String::from("进行中"),
            size: StatusBadgeSize::Small,
        };
        let node = StatusBadge::render(&props);
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(
            found.iter().any(|token| token == "status-badge"),
            "{found:?}"
        );
        assert!(found.iter().any(|token| token == "amber"), "{found:?}");
        assert!(found.iter().any(|token| token == "small"), "{found:?}");
    }

    #[test]
    fn the_badge_default_variant_is_neutral() {
        let props = StatusBadgeProps::new("默认");
        let node = StatusBadge::render(&props);
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(found.iter().any(|token| token == "neutral"), "{found:?}");
    }

    #[test]
    fn the_badge_default_size_is_normal() {
        let props = StatusBadgeProps::new("默认");
        let node = StatusBadge::render(&props);
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(found.iter().any(|token| token == "normal"), "{found:?}");
    }

    #[test]
    fn the_badge_label_is_the_text_content() {
        let props = StatusBadgeProps::new("徽标文案");
        let node = StatusBadge::render(&props);
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert_eq!(texts, vec![String::from("徽标文案")]);
    }

    #[test]
    fn the_badge_root_is_a_span() {
        let node = StatusBadge::render(&StatusBadgeProps::new("徽标"));
        assert_eq!(node.tag(), Some("span"));
    }

    #[test]
    fn the_six_badge_variants_have_their_own_class_token() {
        assert_eq!(StatusBadgeVariant::Amber.class_token(), "amber");
        assert_eq!(StatusBadgeVariant::Green.class_token(), "green");
        assert_eq!(StatusBadgeVariant::Blue.class_token(), "blue");
        assert_eq!(StatusBadgeVariant::Danger.class_token(), "danger");
        assert_eq!(StatusBadgeVariant::Neutral.class_token(), "neutral");
        assert_eq!(StatusBadgeVariant::Dim.class_token(), "dim");
    }

    #[test]
    fn the_two_badge_sizes_have_their_own_class_token() {
        assert_eq!(StatusBadgeSize::Small.class_token(), "small");
        assert_eq!(StatusBadgeSize::Normal.class_token(), "normal");
    }

    #[test]
    fn the_bar_is_a_pure_function_of_its_props() {
        // 无处理器时两次渲染**逐字相等**（处理器按指针身份比较 ⇒ 带回调的两次
        // 渲染在 `PartialEq` 上**必然不等**，那是 `ui-dom` 的既定口径）。
        let items = vec![item("a", StatusTone::Quiet, false, false)];
        let first = render_bar(items.clone());
        let second = render_bar(items);
        assert_eq!(first, second, "同样的 props ⇒ 同样的产物");
    }

    #[test]
    fn two_renders_with_equal_data_have_the_same_shape() {
        // 带回调时：结构 / class / 文本**逐字相同**（只有处理器身份不同）。
        let build = || {
            let mut entry = item("a", StatusTone::Warn, true, false);
            entry.on_click = Some(Callback::new(|| {}));
            render_bar(vec![entry])
        };
        let first = build();
        let second = build();
        let mut first_tags = Vec::new();
        let mut second_tags = Vec::new();
        tags(&first, &mut first_tags);
        tags(&second, &mut second_tags);
        assert_eq!(first_tags, second_tags);
        let mut first_classes = Vec::new();
        let mut second_classes = Vec::new();
        classes(&first, &mut first_classes);
        classes(&second, &mut second_classes);
        assert_eq!(first_classes, second_classes);
        let mut first_texts = Vec::new();
        let mut second_texts = Vec::new();
        text_nodes(&first, &mut first_texts);
        text_nodes(&second, &mut second_texts);
        assert_eq!(first_texts, second_texts);
    }

    #[test]
    fn the_component_needs_no_runtime() {
        // 组件是纯函数 ⇒ 不依赖运行时（对照：卡面组件的展开态由调用方持有信号）。
        let runtime = Runtime::new();
        let _ = runtime.signal(true);
        let node = render_bar(vec![item("a", StatusTone::Quiet, false, false)]);
        assert_eq!(node.tag(), Some("footer"));
    }
}
