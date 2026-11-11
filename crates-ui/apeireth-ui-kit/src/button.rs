//! 基础组件：按钮。
//!
//! 纪律（设计文档 §2.2）：组件库只允许出现 **class 名与结构**，不出现颜色
//! 字面量与时长字面量（那些归 CSS 资产与设计令牌）。
//!
//! ## class 名由调用方给
//!
//! 设计文档 §9.1：class 名是**视觉契约的一部分**（CSS 资产按这些名字写死，
//! 改名即改视觉）⇒ 组件库**不发明** class 名，只接受调用方给的 class 名。
//! 这也让同一个按钮组件能对齐不同的既有实现（试点面对齐的是既有卡片的
//! 按钮 class 名）。
//!
//! ## 刻意不加的
//!
//! - **不加 `type` 属性**：既有实现里的按钮**没有** `type` ⇒ 加了会让 DOM
//!   与既有实现不一致（设计文档 §9.2 的 D2 属性差异必须为零）。
//! - **不加内联样式**（设计文档 §9.2 C3）。

use apeireth_ui_core::UiComponent;
use apeireth_ui_dom::{Children, ElementHandler, VNode};

/// 按钮组件的 props。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ButtonProps {
    /// class 名（可含多个 token，空格分隔）。
    pub class_name: String,
    /// 按钮内容（文本或图标）。
    pub children: Children,
    /// 追加的 class（条件态外观；`None` ⇒ 不追加）。
    pub extra_class: Option<String>,
    /// 是否禁用。
    pub disabled: bool,
    /// 无障碍标签。
    pub aria_label: Option<String>,
    /// 悬浮提示。
    pub title: Option<String>,
    /// 点击处理器。
    pub on_click: Option<ElementHandler>,
}

impl ButtonProps {
    /// 建一组按钮 props（class 名必给）。
    pub fn new(class_name: impl Into<String>) -> Self {
        Self {
            class_name: class_name.into(),
            children: Children::default(),
            extra_class: None,
            disabled: false,
            aria_label: None,
            title: None,
            on_click: None,
        }
    }

    /// 设按钮内容为一段文本（builder 风格）。
    pub fn label(mut self, label: impl Into<String>) -> Self {
        let text = label.into();
        self.children = Children::new(move || VNode::text(text.clone()));
        self
    }

    /// 设按钮内容（builder 风格）。
    pub fn children(mut self, children: Children) -> Self {
        self.children = children;
        self
    }

    /// 追加 class（builder 风格）。
    pub fn extra_class(mut self, extra_class: impl Into<String>) -> Self {
        self.extra_class = Some(extra_class.into());
        self
    }

    /// 条件 class：命中 ⇒ 追加该 token；未命中 ⇒ 不追加（builder 风格）。
    pub fn conditional_class(mut self, token: impl Into<String>, enabled: bool) -> Self {
        self.extra_class = if enabled { Some(token.into()) } else { None };
        self
    }

    /// 设禁用态（builder 风格）。
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// 设无障碍标签（builder 风格）。
    pub fn aria_label(mut self, aria_label: impl Into<String>) -> Self {
        self.aria_label = Some(aria_label.into());
        self
    }

    /// 设悬浮提示（builder 风格）。
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// 设点击处理器（builder 风格）。
    pub fn on_click(mut self, on_click: ElementHandler) -> Self {
        self.on_click = Some(on_click);
        self
    }
}

/// 按钮组件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Button;

impl UiComponent for Button {
    type Props = ButtonProps;
    type Output = VNode;

    fn render(props: &Self::Props) -> Self::Output {
        let class_name = match &props.extra_class {
            Some(extra) => format!("{} {extra}", props.class_name),
            None => props.class_name.clone(),
        };
        let mut button = VNode::element("button").class(class_name);
        if props.disabled {
            // 布尔 false ⇒ 不写该属性（不写 `disabled="false"`）。
            button = button.attr("disabled", true);
        }
        if let Some(aria_label) = &props.aria_label {
            button = button.attr("aria-label", aria_label.clone());
        }
        if let Some(title) = &props.title {
            button = button.attr("title", title.clone());
        }
        if let Some(on_click) = &props.on_click {
            button = button.handler(apeireth_ui_dom::EventKind::Click, on_click.clone());
        }
        button.child(props.children.render()).build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_renders_the_given_class_name_and_content() {
        let node = Button::render(&ButtonProps::new("reject-btn").label("拒绝"));
        assert_eq!(node.tag(), Some("button"));
        assert_eq!(
            node.attr_value("class"),
            Some(&apeireth_ui_dom::AttrValue::Text(String::from(
                "reject-btn"
            )))
        );
        assert_eq!(node.children().len(), 1);
    }

    #[test]
    fn disabled_button_writes_the_attribute_and_enabled_one_does_not() {
        let enabled = Button::render(&ButtonProps::new("allow-btn"));
        assert!(enabled.attr_value("disabled").is_none(), "false ⇒ 不写属性");

        let disabled = Button::render(&ButtonProps::new("allow-btn").disabled(true));
        assert!(disabled.attr_value("disabled").is_some());
    }

    #[test]
    fn optional_attributes_are_omitted_when_absent() {
        let node = Button::render(&ButtonProps::new("close-btn"));
        assert!(node.attr_value("aria-label").is_none());
        assert!(node.attr_value("title").is_none());
        assert!(
            node.attr_value("type").is_none(),
            "既有实现的按钮没有 `type` ⇒ 不许补"
        );
    }

    #[test]
    fn optional_attributes_are_written_when_present() {
        let node = Button::render(
            &ButtonProps::new("close-btn")
                .aria_label("关闭")
                .title("关闭（不拒绝）"),
        );
        assert_eq!(
            node.attr_value("aria-label"),
            Some(&apeireth_ui_dom::AttrValue::Text(String::from("关闭")))
        );
        assert_eq!(
            node.attr_value("title"),
            Some(&apeireth_ui_dom::AttrValue::Text(String::from(
                "关闭（不拒绝）"
            )))
        );
    }

    #[test]
    fn conditional_class_is_added_only_when_enabled() {
        let on = Button::render(&ButtonProps::new("copy-btn").conditional_class("copied", true));
        assert_eq!(
            on.attr_value("class"),
            Some(&apeireth_ui_dom::AttrValue::Text(String::from(
                "copy-btn copied"
            )))
        );
        let off = Button::render(&ButtonProps::new("copy-btn").conditional_class("copied", false));
        assert_eq!(
            off.attr_value("class"),
            Some(&apeireth_ui_dom::AttrValue::Text(String::from("copy-btn")))
        );
    }

    #[test]
    fn click_handler_is_attached_to_the_button() {
        let node = Button::render(
            &ButtonProps::new("allow-btn").on_click(ElementHandler::new(|_event| {})),
        );
        match &node {
            VNode::Element(element) => {
                assert_eq!(element.handlers.len(), 1);
                assert_eq!(element.handlers[0].0, apeireth_ui_dom::EventKind::Click);
            }
            other => panic!("实测节点形态：{other:?}"),
        }
    }
}
