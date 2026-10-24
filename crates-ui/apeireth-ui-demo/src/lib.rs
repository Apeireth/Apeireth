//! # apeireth-ui-demo — 试点应用（审批卡）
//!
//! P3 试点载体兼**装配级冒烟**（ADR-0015 §4）：组件 = Rust 函数 ⇒ 整条渲染链
//! 在 native `cargo test` 里直接跑（这是 D3 债「38 处源码镜像测试」的根解法：
//! 断言从「读源码文本」升级为「钉行为」）。
//!
//! ## 本批（P0 骨架）交付口径
//! - ✅ **已实现**：用 `#[component]` 宏声明的审批卡组件、响应式状态信号
//!   （审批 → 状态文案变化）、effect 驱动的审批计数、虚拟树的文本渲染器、
//!   可独立运行的二进制入口。
//! - 🔵 **未接线**：浏览器内对照（P3 逐像素 / 逐帧对照门；真实 DOM 后端已在
//!   P2 落地但**未经任何运行时验证**）、后端数据契约（HTTP / SSE）—— 引擎不碰
//!   契约，数据面归既有后端。
//!
//! ## 纪律
//! 试点**只做审批卡一面**（P4 是硬闸门：不过即停）；不迁其余组件。

#![forbid(unsafe_code)]

use std::cell::Cell;
use std::rc::Rc;

use apeireth_ui::component;
use apeireth_ui::core::{FlushReport, ReadSignal, Runtime, Signal};
use apeireth_ui::dom::VNode;
use apeireth_ui::kit::{Button, ButtonProps};
use apeireth_ui::UiComponent;

/// 审批卡组件（用 `#[component]` 宏声明：**函数即组件**）。
///
/// 宏展开后等价于：props 结构体 `ApprovalCardProps` + 组件类型 `ApprovalCard`
/// + `UiComponent` 实现（设计文档 §6.1）。
#[component]
pub fn ApprovalCard(request_id: String, title: String) -> VNode {
    VNode::element("section")
        .class("ap-approval-card")
        .child(
            VNode::element("h3")
                .class("ap-approval-card__title")
                .child(VNode::text(title))
                .build(),
        )
        .child(
            VNode::element("p")
                .class("ap-approval-card__id")
                .child(VNode::text(format!("编号 {request_id}")))
                .build(),
        )
        .child(
            VNode::element("div")
                .class("ap-approval-card__actions")
                .child(Button::render(&ButtonProps::new("批准")))
                .build(),
        )
        .build()
}

/// 审批卡视图树（便捷入口）。
pub fn approval_card(request_id: &str, title: &str) -> VNode {
    ApprovalCard::render(&ApprovalCardProps {
        request_id: request_id.to_owned(),
        title: title.to_owned(),
    })
}

/// 试点应用：审批卡 + 一个审批状态信号。
pub struct DemoApp {
    runtime: Runtime,
    approved: Signal<bool>,
    approval_count: Rc<Cell<u32>>,
}

impl Default for DemoApp {
    fn default() -> Self {
        Self::new()
    }
}

impl DemoApp {
    /// 建一个试点应用。
    ///
    /// effect 只读状态信号、只写计数器（计数器不是信号 ⇒ 不会自激）。
    pub fn new() -> Self {
        let runtime = Runtime::new();
        let approved = runtime.signal(false);
        let approval_count = Rc::new(Cell::new(0u32));

        let watched = approved.clone();
        let counter = approval_count.clone();
        runtime.effect(move || {
            if watched.get() {
                counter.set(counter.get() + 1);
            }
        });

        Self {
            runtime,
            approved,
            approval_count,
        }
    }

    /// 审批状态（只读视图）。
    pub fn approved(&self) -> ReadSignal<bool> {
        self.approved.read_only()
    }

    /// 审批次数（effect 累计）。
    pub fn approval_count(&self) -> u32 {
        self.approval_count.get()
    }

    /// 标记为已批准（写信号 ⇒ 触发一次调度）。
    pub fn approve(&self) {
        self.approved.set(true);
    }

    /// 跑一次调度（native 同步 flush；wasm 侧由宿主调度器驱动）。
    pub fn flush(&self) -> FlushReport {
        self.runtime.flush()
    }

    /// 渲染整棵视图树。
    pub fn render(&self) -> VNode {
        let state = if self.approved.get() {
            "已批准"
        } else {
            "待审批"
        };
        VNode::element("main")
            .class("ap-demo")
            .child(approval_card("REQ-0001", "审批卡试点"))
            .child(
                VNode::element("p")
                    .class("ap-demo__state")
                    .child(VNode::text(state))
                    .build(),
            )
            .build()
    }
}

/// 虚拟树 ⇒ 文本（供 native 冒烟打印与断言）。
pub fn tree_to_text(node: &VNode) -> String {
    let mut out = String::new();
    render_text(node, &mut out);
    out
}

/// 递归写文本。
fn render_text(node: &VNode, out: &mut String) {
    match node {
        VNode::Element(element) => {
            out.push('<');
            out.push_str(&element.tag);
            for (name, value) in &element.attrs {
                if let Some(value) = value.to_attr_string() {
                    out.push(' ');
                    out.push_str(name);
                    out.push_str("=\"");
                    out.push_str(&value);
                    out.push('"');
                }
            }
            out.push('>');
            for child in &element.children {
                render_text(child, out);
            }
            out.push_str("</");
            out.push_str(&element.tag);
            out.push('>');
        }
        VNode::Text(text) => out.push_str(&text.data),
        VNode::Fragment(children) => {
            for child in children {
                render_text(child, out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //! crate 级断言：跨层组合行为（宏组件 + 组件库 + 响应式信号）。

    use super::*;

    #[test]
    fn approval_card_renders_expected_structure() {
        let node = approval_card("REQ-0001", "审批卡试点");
        let text = tree_to_text(&node);

        assert!(text.contains("<section"), "实测文本：{text}");
        assert!(text.contains("ap-approval-card"), "实测文本：{text}");
        assert!(text.contains("编号 REQ-0001"), "实测文本：{text}");
        assert!(text.contains("批准"), "实测文本：{text}");
    }

    #[test]
    fn approval_signal_drives_state_and_effect() {
        let app = DemoApp::new();
        assert!(!app.approved().get());
        assert_eq!(app.approval_count(), 0);

        app.approve();
        app.flush();

        assert!(app.approved().get());
        assert_eq!(app.approval_count(), 1, "effect 必须跑过一次");
        assert!(tree_to_text(&app.render()).contains("已批准"));
    }
}
