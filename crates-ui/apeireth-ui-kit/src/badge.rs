//! 基础组件：徽标。
//!
//! 纪律：只出 class 名与结构（设计文档 §2.2 / §8.3）。
//!
//! 徽标由「图标 + 文本」组成 ⇒ 内容用 [`Children`] 表达（调用方决定里面
//! 放什么）；条件态（例如「有墙 / 无墙」两种外观）由调用方用
//! [`BadgeProps::extra_class`] 追加 class。

use apeireth_ui_core::UiComponent;
use apeireth_ui_dom::{Children, VNode};

/// 徽标组件的 props。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BadgeProps {
    /// class 名（可含多个 token）。
    pub class_name: String,
    /// 追加的 class（条件态外观）。
    pub extra_class: Option<String>,
    /// 徽标内容。
    pub children: Children,
    /// 悬浮提示。
    pub title: Option<String>,
}

impl BadgeProps {
    /// 建一组徽标 props（class 名必给）。
    pub fn new(class_name: impl Into<String>) -> Self {
        Self {
            class_name: class_name.into(),
            extra_class: None,
            children: Children::default(),
            title: None,
        }
    }

    /// 追加 class（builder 风格）。
    pub fn extra_class(mut self, extra_class: impl Into<String>) -> Self {
        self.extra_class = Some(extra_class.into());
        self
    }

    /// 设徽标内容（builder 风格）。
    pub fn children(mut self, children: Children) -> Self {
        self.children = children;
        self
    }

    /// 设悬浮提示（builder 风格）。
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

/// 徽标组件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Badge;

impl UiComponent for Badge {
    type Props = BadgeProps;
    type Output = VNode;

    fn render(props: &Self::Props) -> Self::Output {
        let class_name = match &props.extra_class {
            Some(extra) => format!("{} {extra}", props.class_name),
            None => props.class_name.clone(),
        };
        let mut badge = VNode::element("span").class(class_name);
        if let Some(title) = &props.title {
            badge = badge.attr("title", title.clone());
        }
        badge.child(props.children.render()).build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badge_renders_a_span_with_the_given_class_name() {
        let node = Badge::render(&BadgeProps::new("sandbox-badge"));
        assert_eq!(node.tag(), Some("span"));
        assert_eq!(
            node.attr_value("class"),
            Some(&apeireth_ui_dom::AttrValue::Text(String::from(
                "sandbox-badge"
            )))
        );
    }

    #[test]
    fn extra_class_is_appended_after_the_base_class() {
        let node = Badge::render(&BadgeProps::new("sandbox-badge").extra_class("unsandboxed"));
        assert_eq!(
            node.attr_value("class"),
            Some(&apeireth_ui_dom::AttrValue::Text(String::from(
                "sandbox-badge unsandboxed"
            )))
        );
    }

    #[test]
    fn badge_carries_the_title_attribute_when_given() {
        let node = Badge::render(&BadgeProps::new("sandbox-badge").title("提示"));
        assert_eq!(
            node.attr_value("title"),
            Some(&apeireth_ui_dom::AttrValue::Text(String::from("提示")))
        );
    }
}
