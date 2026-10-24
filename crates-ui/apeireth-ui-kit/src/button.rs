//! 基础组件：按钮（共享组件的 Rust 重写，P0 骨架批）。
//!
//! 纪律（设计文档 §2.2）：组件库只允许出现 **class 名与结构**，不出现颜色
//! 字面量与时长字面量（那些归 CSS 资产与设计令牌）。
//!
//! **P0 class 名为占位**：正式 class 名须与被替换实现**逐字一致**（设计 §9.1），
//! 该对齐在 P5 迁移批逐面完成；P0 批不冒充已对齐。

use apeireth_ui_core::UiComponent;
use apeireth_ui_dom::VNode;

/// 按钮组件的 props。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ButtonProps {
    /// 按钮文案。
    pub label: String,
    /// 是否禁用。
    pub disabled: bool,
}

impl ButtonProps {
    /// 建一组按钮 props。
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            disabled: false,
        }
    }

    /// 设禁用态（builder 风格）。
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
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
        let mut button = VNode::element("button")
            .class("ap-button")
            .attr("type", "button")
            .child(VNode::text(props.label.clone()));
        if props.disabled {
            // 布尔 false ⇒ 不写该属性（不写 `disabled="false"`）。
            button = button.attr("disabled", true);
        }
        button.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_renders_label_and_omits_false_attributes() {
        let node = Button::render(&ButtonProps::new("提交"));
        match node {
            VNode::Element(element) => {
                assert_eq!(element.tag, "button");
                assert_eq!(element.children.len(), 1);
                assert!(element.attr_value("disabled").is_none(), "false ⇒ 不写属性");
                assert_eq!(
                    element.attr_value("class"),
                    Some(&apeireth_ui_dom::AttrValue::Text(String::from("ap-button")))
                );
            }
            other => panic!("实测节点形态：{other:?}"),
        }
    }

    #[test]
    fn disabled_button_writes_the_attribute() {
        let props = ButtonProps::new("提交").disabled(true);
        let node = Button::render(&props);
        match node {
            VNode::Element(element) => {
                assert!(element.attr_value("disabled").is_some());
            }
            other => panic!("实测节点形态：{other:?}"),
        }
    }
}
