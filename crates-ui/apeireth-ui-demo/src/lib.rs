//! # apeireth-ui-demo — 试点应用（审批卡）
//!
//! P3 试点载体兼**装配级冒烟**（ADR-0015 §4）：组件 = Rust 函数 ⇒ 整条渲染链
//! 在 native `cargo test` 里直接跑（这是 D3 债「38 处源码镜像测试」的根解法：
//! 断言从「读源码文本」升级为「钉行为」）。
//!
//! ## 本批（P3）交付口径
//!
//! ### ✅ 已实现
//! - **审批卡试点组件**（[`approval_card`]）：21 个 class 名与生产实现**逐字
//!   一致**；结构、属性、文本节点逐条对齐。
//! - **`view!` 虚拟树宏**的实际使用（试点组件的内部结构由宏构造）。
//! - **组件库**（按钮 / 徽标 / 卡片壳 / 图标）在本试点上的实际消费。
//! - **四条交互**：允许 / 拒绝 / 关闭（**只关卡片、不做业务决策**）/ 复制
//!   （复制态 ⇒ 图标切换 + 条件 class）。
//! - **固定数据**（[`fixture`]）：两侧喂同一份 fixture ⇒ 差异只可能来自引擎。
//!
//! ### 🔵 未接线
//! - **真后端**（`/v1/apeireth/approval-requests` + SSE `approval_required`）：
//!   引擎不碰契约（设计文档 §1.3 第 6 条）⇒ 数据面归既有后端，试点只喂固定
//!   fixture。
//! - **浏览器内对照**（像素 / 逐帧）：由试点 harness 承担（`harness/`），
//!   **不是**本 crate 的 `cargo test` ⇒ native 侧只断言虚拟树与事件行为。
//!
//! ## 纪律
//!
//! 试点**只做审批卡一面**（P4 是硬闸门：不过即停）；不迁其余组件。

#![forbid(unsafe_code)]

pub mod approval_card;
pub mod fixture;
mod fixture_generated;

pub use approval_card::{format_clock_utc, ApprovalCard, ApprovalCardProps, ApprovalItem};
pub use fixture::pilot_item;

use std::rc::Rc;

use apeireth_ui_core::{Callback, Runtime, Signal};
use apeireth_ui_dom::VNode;

/// 试点应用状态（响应式部分）。
///
/// 三条交互的**语义**在这里落地（设计文档 §6.4：组件是纯函数，状态归调用方）：
///
/// | 交互 | 语义 |
/// |---|---|
/// | 允许 | 置「已允许」决策并停止处理中 |
/// | 拒绝 | 置「已拒绝」决策并停止处理中 |
/// | 关闭 | **只关卡片**（隐藏卡片），**不做业务决策**（不置允许 / 拒绝） |
/// | 复制 | 置「已复制」状态（复制态图标切换） |
pub struct PilotState {
    inner: Rc<PilotInner>,
}

/// 试点状态的内层（可共享 ⇒ 回调能捕获它）。
struct PilotInner {
    /// 运行时（信号宿主）。
    runtime: Runtime,
    /// 处理中。
    busy: Signal<bool>,
    /// 已复制。
    copied: Signal<bool>,
    /// 业务决策（`None` = 尚未决策；`Some(true)` = 允许；`Some(false)` = 拒绝）。
    decision: Signal<Option<bool>>,
    /// 卡片是否可见（关闭 ⇒ 隐藏）。
    visible: Signal<bool>,
}

impl Default for PilotState {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for PilotState {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl PilotState {
    /// 建一份初始状态。
    pub fn new() -> Self {
        let runtime = Runtime::new();
        Self {
            inner: Rc::new(PilotInner {
                busy: runtime.signal(false),
                copied: runtime.signal(false),
                decision: runtime.signal(None),
                visible: runtime.signal(true),
                runtime,
            }),
        }
    }

    /// 运行时（信号宿主）。
    pub fn runtime(&self) -> &Runtime {
        &self.inner.runtime
    }

    /// 处理中信号。
    pub fn busy(&self) -> Signal<bool> {
        self.inner.busy.clone()
    }

    /// 「已复制」信号。
    pub fn copied(&self) -> Signal<bool> {
        self.inner.copied.clone()
    }

    /// 业务决策信号。
    pub fn decision(&self) -> Signal<Option<bool>> {
        self.inner.decision.clone()
    }

    /// 卡片可见性信号。
    pub fn visible(&self) -> Signal<bool> {
        self.inner.visible.clone()
    }

    /// 允许。
    pub fn allow(&self) {
        self.inner.decision.set(Some(true));
        self.inner.busy.set(false);
    }

    /// 拒绝。
    pub fn reject(&self) {
        self.inner.decision.set(Some(false));
        self.inner.busy.set(false);
    }

    /// 关闭（**只关卡片，不做业务决策**）。
    pub fn dismiss(&self) {
        self.inner.visible.set(false);
    }

    /// 标记为已复制。
    pub fn mark_copied(&self) {
        self.inner.copied.set(true);
    }

    /// 跑一次调度（native 同步 flush）。
    pub fn flush(&self) {
        self.inner.runtime.flush();
    }
}

/// 建试点应用的三条回调（允许 / 拒绝 / 关闭）。
///
/// 返回顺序：`(允许, 拒绝, 关闭)`。
pub fn pilot_callbacks(state: &PilotState) -> (Callback, Callback, Callback) {
    let allow_state = state.clone();
    let reject_state = state.clone();
    let dismiss_state = state.clone();
    (
        Callback::new(move || allow_state.allow()),
        Callback::new(move || reject_state.reject()),
        Callback::new(move || dismiss_state.dismiss()),
    )
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
    use apeireth_ui_core::UiComponent;
    use apeireth_ui_dom::Children;

    /// 建一份带三条回调的试点 props。
    fn pilot_props(state: &PilotState) -> ApprovalCardProps {
        let (allow, reject, dismiss) = pilot_callbacks(state);
        ApprovalCardProps {
            item: pilot_item(),
            busy: state.busy().get(),
            copied: state.copied(),
            on_allow: allow,
            on_reject: reject,
            on_dismiss: Some(dismiss),
            children: Children::default(),
        }
    }

    /// 试点组件用到的 21 个 class 名（与生产实现逐字一致）。
    const PILOT_CLASS_NAMES: &[&str] = &[
        "approval-card",
        "card-header",
        "card-body",
        "card-footer",
        "title-wrap",
        "title",
        "time",
        "section",
        "section-label",
        "section-text",
        "command-row",
        "command",
        "sandbox-row",
        "sandbox-badge",
        "sandbox-meta",
        "allow-btn",
        "reject-btn",
        "close-btn",
        "copy-btn",
        "mono-path",
        "reason",
    ];

    #[test]
    fn approval_card_renders_the_twenty_one_class_names() {
        let state = PilotState::new();
        let node = ApprovalCard::render(&pilot_props(&state));
        let text = tree_to_text(&node);

        for class in PILOT_CLASS_NAMES {
            assert!(
                text.contains(class),
                "class `{class}` 未出现在渲染结果里：{text}"
            );
        }
    }

    #[test]
    fn approval_card_renders_the_expected_structure() {
        let state = PilotState::new();
        let node = ApprovalCard::render(&pilot_props(&state));
        let text = tree_to_text(&node);

        assert!(
            text.contains("<div class=\"approval-card\" role=\"group\""),
            "实测文本：{text}"
        );
        assert!(text.contains("批准"), "实测文本：{text}");
        assert!(text.contains("拒绝"), "实测文本：{text}");
    }

    #[test]
    fn dismiss_hides_the_card_without_making_a_business_decision() {
        let state = PilotState::new();
        state.dismiss();
        state.flush();

        assert!(!state.visible().get(), "关闭 ⇒ 卡片隐藏");
        assert!(
            state.decision().get().is_none(),
            "关闭**不做业务决策** ⇒ 决策仍为空"
        );
    }

    #[test]
    fn allow_and_reject_record_different_decisions() {
        let allow_state = PilotState::new();
        allow_state.allow();
        allow_state.flush();
        assert_eq!(allow_state.decision().get(), Some(true));

        let reject_state = PilotState::new();
        reject_state.reject();
        reject_state.flush();
        assert_eq!(reject_state.decision().get(), Some(false));
    }
}
