//! 基础组件：图标（SVG）。
//!
//! 纪律（设计文档 §2.2）：组件库只允许出现 **class 名与结构**，不出现颜色
//! 字面量与时长字面量。SVG 图标的**几何数据**（路径、矩形参数）不是样式值，
//! 而是资产 —— 它由**构建期抽取**放进 [`crate::icon_data`]（生成物，带上游
//! 许可署名），组件本体只负责**结构**：标签、属性名、属性顺序。
//!
//! ## 为什么图标数据是「生成物」而不是手抄
//!
//! 像素对照要求两侧的 SVG **逐字一致** ⇒ 路径数据必须来自**同一份**源头。
//! 手抄会漂移（上游改一版就对不上）⇒ 由 `harness/scripts/extract-assets.mjs`
//! 在构建期从上游包**抽取**并生成本文件 ⇒ 单一真源 + 许可单一。
//!
//! ## 结构对齐
//!
//! 渲染出的 `<svg>` 与上游组件渲染出的 `<svg>` **属性集与属性顺序一致**
//! （`xmlns` / `width` / `height` / `viewBox` / `fill` / `stroke` /
//! `stroke-width` / `stroke-linecap` / `stroke-linejoin` / `class`）。

use apeireth_ui_core::UiComponent;
use apeireth_ui_dom::VNode;

/// 图标节点（SVG 的一个子元素：标签 + 属性表）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconNode {
    /// 元素标签（`path` / `rect` / …）。
    pub tag: &'static str,
    /// 属性表（保序）。
    pub attrs: &'static [(&'static str, &'static str)],
}

/// 图标数据（名字 + class + 节点表）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconData {
    /// 图标名（上游的名字，用作 class 后缀）。
    pub name: &'static str,
    /// 渲染出的 class 字符串。
    pub class: &'static str,
    /// 节点表。
    pub nodes: &'static [IconNode],
}

/// 图标组件的 props。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconProps {
    /// 图标数据（来自 [`crate::icon_data`]）。
    pub data: &'static IconData,
    /// 边长（像素）。
    pub size: u32,
    /// 追加的 class（拼在图标自带 class 之后）。
    pub extra_class: Option<String>,
}

impl IconProps {
    /// 建一组图标 props。
    pub fn new(data: &'static IconData, size: u32) -> Self {
        Self {
            data,
            size,
            extra_class: None,
        }
    }

    /// 追加 class（builder 风格）。
    pub fn extra_class(mut self, extra_class: impl Into<String>) -> Self {
        self.extra_class = Some(extra_class.into());
        self
    }
}

/// 图标组件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Icon;

impl UiComponent for Icon {
    type Props = IconProps;
    type Output = VNode;

    fn render(props: &Self::Props) -> Self::Output {
        let class = match &props.extra_class {
            Some(extra) => format!("{} {extra}", props.data.class),
            None => props.data.class.to_owned(),
        };
        let size = props.size.to_string();
        let mut svg = VNode::element("svg")
            .attr("xmlns", "http://www.w3.org/2000/svg")
            .attr("width", size.clone())
            .attr("height", size)
            .attr("viewBox", "0 0 24 24")
            .attr("fill", "none")
            .attr("stroke", "currentColor")
            .attr("stroke-width", "2")
            .attr("stroke-linecap", "round")
            .attr("stroke-linejoin", "round")
            .attr("class", class);
        for node in props.data.nodes {
            let mut child = VNode::element(node.tag);
            for (name, value) in node.attrs {
                child = child.attr(*name, *value);
            }
            svg = svg.child(child.build());
        }
        svg.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icon_data::ICON_X;

    #[test]
    fn icon_renders_the_svg_shell_with_the_upstream_attributes() {
        let node = Icon::render(&IconProps::new(&ICON_X, 16));
        assert_eq!(node.tag(), Some("svg"));
        assert_eq!(
            node.attr_value("width"),
            Some(&apeireth_ui_dom::AttrValue::Text(String::from("16")))
        );
        assert_eq!(
            node.attr_value("viewBox"),
            Some(&apeireth_ui_dom::AttrValue::Text(String::from("0 0 24 24")))
        );
        assert_eq!(
            node.attr_value("stroke"),
            Some(&apeireth_ui_dom::AttrValue::Text(String::from(
                "currentColor"
            )))
        );
    }

    #[test]
    fn icon_nodes_become_child_elements() {
        let node = Icon::render(&IconProps::new(&ICON_X, 16));
        let children = node.children();
        assert_eq!(children.len(), ICON_X.nodes.len());
        assert_eq!(children[0].tag(), Some("path"));
        assert_eq!(
            children[0].attr_value("d"),
            Some(&apeireth_ui_dom::AttrValue::Text(String::from(
                "M18 6 6 18"
            )))
        );
    }

    #[test]
    fn extra_class_is_appended_to_the_icon_class() {
        let node = Icon::render(&IconProps::new(&ICON_X, 12).extra_class("ap-icon"));
        let class = match node.attr_value("class") {
            Some(apeireth_ui_dom::AttrValue::Text(text)) => text.clone(),
            other => panic!("实测属性：{other:?}"),
        };
        assert!(class.ends_with(" ap-icon"), "实测 class：{class}");
    }

    #[test]
    fn icon_carries_no_colour_or_duration_literals() {
        // 令牌纪律：组件里不许出现颜色字面量（`currentColor` 是继承关键字，
        // 不是颜色值；`viewBox` 是坐标系，不是样式）。
        let node = Icon::render(&IconProps::new(&ICON_X, 16));
        for (name, value) in match &node {
            VNode::Element(element) => element
                .attrs
                .iter()
                .map(|(name, value)| (name.as_str(), value.to_attr_string()))
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        } {
            if let Some(text) = value {
                assert!(
                    !text.starts_with('#') && !text.starts_with("rgb"),
                    "属性 `{name}` 出现了颜色字面量：{text}"
                );
            }
        }
    }
}
