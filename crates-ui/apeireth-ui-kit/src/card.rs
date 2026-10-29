//! 基础组件：卡片壳。
//!
//! 纪律：只出 class 名与结构（设计文档 §2.2 / §8.3）。
//!
//! 卡片壳 = 一个容器元素 + class 名 + 可选的无障碍属性 + 子内容。
//! **刻意不设 `id`**（设计文档 §9.2 C4：DOM 定位靠 class 与结构）。

use apeireth_ui_core::UiComponent;
use apeireth_ui_dom::{Children, VNode};

/// 卡片壳组件的 props。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardShellProps {
    /// class 名（可含多个 token）。
    pub class_name: String,
    /// 角色（`group` 等）。
    pub role: Option<String>,
    /// 无障碍标签。
    pub aria_label: Option<String>,
    /// 卡片内容。
    pub children: Children,
}

impl CardShellProps {
    /// 建一组卡片壳 props（class 名必给）。
    pub fn new(class_name: impl Into<String>) -> Self {
        Self {
            class_name: class_name.into(),
            role: None,
            aria_label: None,
            children: Children::default(),
        }
    }

    /// 设角色（builder 风格）。
    pub fn role(mut self, role: impl Into<String>) -> Self {
        self.role = Some(role.into());
        self
    }

    /// 设无障碍标签（builder 风格）。
    pub fn aria_label(mut self, aria_label: impl Into<String>) -> Self {
        self.aria_label = Some(aria_label.into());
        self
    }

    /// 设卡片内容（builder 风格）。
    pub fn children(mut self, children: Children) -> Self {
        self.children = children;
        self
    }
}

/// 卡片壳组件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CardShell;

impl UiComponent for CardShell {
    type Props = CardShellProps;
    type Output = VNode;

    fn render(props: &Self::Props) -> Self::Output {
        let mut card = VNode::element("div").class(props.class_name.clone());
        if let Some(role) = &props.role {
            card = card.attr("role", role.clone());
        }
        if let Some(aria_label) = &props.aria_label {
            card = card.attr("aria-label", aria_label.clone());
        }
        card.child(props.children.render()).build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_shell_renders_a_div_with_the_given_class_name() {
        let node = CardShell::render(&CardShellProps::new("approval-card"));
        assert_eq!(node.tag(), Some("div"));
        assert_eq!(
            node.attr_value("class"),
            Some(&apeireth_ui_dom::AttrValue::Text(String::from(
                "approval-card"
            )))
        );
    }

    #[test]
    fn card_shell_writes_role_and_aria_label_when_given() {
        let node = CardShell::render(
            &CardShellProps::new("approval-card")
                .role("group")
                .aria_label("待批准操作"),
        );
        assert_eq!(
            node.attr_value("role"),
            Some(&apeireth_ui_dom::AttrValue::Text(String::from("group")))
        );
        assert_eq!(
            node.attr_value("aria-label"),
            Some(&apeireth_ui_dom::AttrValue::Text(String::from(
                "待批准操作"
            )))
        );
    }

    #[test]
    fn card_shell_never_sets_an_id() {
        let node = CardShell::render(&CardShellProps::new("approval-card"));
        assert!(
            node.attr_value("id").is_none(),
            "设计文档 §9.2 C4：组件不设 `id`"
        );
    }
}
