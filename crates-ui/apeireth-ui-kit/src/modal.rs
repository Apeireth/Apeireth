//! ⭐⭐⭐ **弹层组件**（L2 组件层第二批，E-177）。
//!
//! ## 这个组件解决什么
//!
//! 「模态对话框」是 UI 里**最容易漏接线**的一种组件 ⇒ ⇒ 因为它要同时做
//! **四件事**：①渲染结构 ②设焦点陷阱 ③聚焦初值 ④关闭时归还焦点。
//! ⇒ ⇒ 漏任何一件，后果都不同（详见引擎侧 [`apeireth_ui_dom::Renderer::open_modal`]）。
//!
//! ⇒ ⇒ ⇒ 所以本组件**只做第 ①件**（出结构）⇒ ⇒ **第 ②③④件归引擎**
//! ⇒ ⇒ ⇒ 应用层的接线因此是**一处**，不是**四处**。
//!
//! ## ⭐⭐⭐ 组件与引擎的分工（**关键设计**）
//!
//! | 在本组件 | 在引擎（`apeireth_ui-dom`） |
//! |---|---|
//! | 弹层的**结构**（遮罩 / 容器 / 标题 / 关闭按钮 / 内容） | `open_modal`（设陷阱 + 聚焦初值） |
//! | **哪些元素可聚焦**的**相对路径清单**（`ModalView::focusable`） | `close_modal`（关陷阱 + 归还焦点） |
//! | 无障碍属性（`role` / `aria-modal` / `aria-label`） | `close_top_modal_on_escape`（Esc 关最上层） |
//!
//! ## ⭐ 为什么组件**不自己管焦点**
//!
//! 组件是**纯函数**（无状态、无副作用）⇒ ⇒ 若它去调 `renderer.open_modal()`
//! ⇒ ⇒ 就需要 `&mut Renderer` ⇒ ⇒ 组件就不是纯函数了
//! ⇒ ⇒ ⇒ 且**焦点路径依赖挂载位置**（组件自己算不出来）
//! ⇒ ⇒⇒ ⇒ 所以：组件**产出**可聚焦路径清单，**调用方**把它交给引擎。
//!
//! ## ⭐ 挂载位置由**调用方**决定 ⇒ ⇒ 路径前缀也由调用方拼
//!
//! `ModalView::focusable` 里的路径是**相对弹层根**的 ⇒ ⇒
//! 调用方知道弹层挂在哪（如 `[2]`）⇒ ⇒ 拼成 `[2, 0]`、`[2, 1]` 交给
//! [`apeireth_ui_dom::Renderer::open_modal`]。
//!
//! ## 0 装口径
//!
//! - 本组件 = **IMPLEMENTED + native 单测覆盖**；
//! - **PRODUCTION WIRED = ❌**（无生产消费方）；**HARDWARE VALIDATED = ❌**（未在浏览器上屏）。

use apeireth_ui_dom::{Children, ElementHandler, VNode};

/// ⭐⭐⭐ **弹层视图**：结构 + **可聚焦元素的相对路径清单**。
///
/// ## 为什么组件要返回「路径清单」而不是只返回 `VNode`
///
/// 引擎的焦点陷阱需要**路径**（`open_modal(allowed, anchor, initial_focus)`）
/// ⇒ ⇒ 而**路径取决于挂载位置** ⇒ ⇒ 组件在**渲染时**并不知道自己会被挂到哪
/// ⇒ ⇒ ⇒ 所以组件只能给出**相对路径** ⇒ ⇒ 由调用方拼前缀。
///
/// ## 字段
///
/// - `tree`：可直接挂载的**弹层结构**（根 = 弹层容器）；
/// - `focusable`：**相对弹层根**的可聚焦元素路径（DOM 顺序）；
/// - `initial_focus`：建议的**初始焦点**（相对路径）⇒ ⇒ `None` ⇒ 由调用方定。
#[derive(Debug, Clone)]
pub struct ModalView {
    /// 弹层结构（根节点 = 弹层容器）。
    pub tree: VNode,
    /// **相对弹层根**的可聚焦元素路径（DOM 顺序）。
    pub focusable: Vec<Vec<usize>>,
    /// 建议的初始焦点（**相对**路径）。
    pub initial_focus: Option<Vec<usize>>,
}

impl ModalView {
    /// ⭐ 把**相对路径**拼成**绝对路径**（挂载前缀 + 相对路径）。
    ///
    /// ## 为什么要有这个helper
    ///
    /// 调用方每次都要写 `prefix.iter().chain(relative.iter()).copied().collect()`
    /// ⇒ ⇒ 重复且易错 ⇒ ⇒ ⇒ 而且**拼错**的后果是「焦点跳到别的元素」
    /// ⇒ ⇒⇒ ⇒ 收成一个函数 ⇒ ⇒ 少一处出错点。
    pub fn absolute_path(&self, prefix: &[usize], relative: &[usize]) -> Vec<usize> {
        let mut path = prefix.to_vec();
        path.extend_from_slice(relative);
        path
    }

    /// ⭐ 把全部**可聚焦路径**拼成**绝对路径**清单。
    pub fn absolute_focusable(&self, prefix: &[usize]) -> Vec<Vec<usize>> {
        self.focusable
            .iter()
            .map(|relative| self.absolute_path(prefix, relative))
            .collect()
    }

    /// ⭐ 初始焦点的**绝对路径**（`None` ⇒ 调用方自定）。
    pub fn absolute_initial_focus(&self, prefix: &[usize]) -> Option<Vec<usize>> {
        self.initial_focus
            .as_ref()
            .map(|relative| self.absolute_path(prefix, relative))
    }
}

/// ⭐⭐⭐ **弹层 props**。
#[derive(Debug, Clone)]
pub struct ModalProps {
    /// 弹层容器的 class 名（可含多个 token，空格分隔）。
    pub class_name: String,
    /// 遮罩的 class 名（`None` ⇒ 不出遮罩）。
    pub overlay_class_name: Option<String>,
    /// 标题文本（`None` ⇒ 不出标题）。
    pub title: Option<String>,
    /// 标题的 class 名。
    pub title_class_name: Option<String>,
    /// 关闭按钮的 class 名（`None` ⇒ 不出关闭按钮）。
    pub close_class_name: Option<String>,
    /// 关闭按钮的**无障碍标签**（⚠️ 必给 ⇒ ⇒ 按钮只有图标时**必须**有名字）。
    pub close_aria_label: Option<String>,
    /// 关闭按钮的点击处理器。
    pub on_close: Option<ElementHandler>,
    /// 弹层内容的 class 名。
    pub content_class_name: Option<String>,
    /// 弹层内容。
    pub children: Children,
    /// 无障碍标签（`aria-label`）。
    pub aria_label: Option<String>,
    /// 角色（默认 `dialog` ⇒ ⇒ 弹层的**语义**）。
    pub role: Option<String>,
    /// 标题元素（默认 `h2`）。
    pub title_tag: Option<String>,
}

impl ModalProps {
    /// 建一组弹层 props（**class 名必给**）。
    pub fn new(class_name: impl Into<String>) -> Self {
        Self {
            class_name: class_name.into(),
            overlay_class_name: None,
            title: None,
            title_class_name: None,
            close_class_name: None,
            close_aria_label: None,
            on_close: None,
            content_class_name: None,
            children: Children::default(),
            aria_label: None,
            role: Some(String::from("dialog")),
            title_tag: Some(String::from("h2")),
        }
    }

    /// 设遮罩 class（builder 风格）。
    pub fn overlay_class(mut self, class_name: impl Into<String>) -> Self {
        self.overlay_class_name = Some(class_name.into());
        self
    }

    /// 设标题（builder 风格）。
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// 设标题 class（builder 风格）。
    pub fn title_class(mut self, class_name: impl Into<String>) -> Self {
        self.title_class_name = Some(class_name.into());
        self
    }

    /// 设关闭按钮 class（builder 风格）。
    pub fn close_class(mut self, class_name: impl Into<String>) -> Self {
        self.close_class_name = Some(class_name.into());
        self
    }

    /// 设关闭按钮的**无障碍标签**（builder 风格）。
    ///
    /// ⚠️ 关闭按钮若只有图标 ⇒ ⇒ **必须**给这个 ⇒ ⇒ 否则读屏用户**听不到**它。
    pub fn close_aria_label(mut self, label: impl Into<String>) -> Self {
        self.close_aria_label = Some(label.into());
        self
    }

    /// 设关闭处理器（builder 风格）。
    pub fn on_close(mut self, handler: ElementHandler) -> Self {
        self.on_close = Some(handler);
        self
    }

    /// 设内容 class（builder 风格）。
    pub fn content_class(mut self, class_name: impl Into<String>) -> Self {
        self.content_class_name = Some(class_name.into());
        self
    }

    /// 设内容（builder 风格）。
    pub fn children(mut self, children: Children) -> Self {
        self.children = children;
        self
    }

    /// 设无障碍标签（builder 风格）。
    pub fn aria_label(mut self, label: impl Into<String>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// 设角色（builder 风格）。
    pub fn role(mut self, role: impl Into<String>) -> Self {
        self.role = Some(role.into());
        self
    }

    /// 设标题元素标签（builder 风格）。
    pub fn title_tag(mut self, tag: impl Into<String>) -> Self {
        self.title_tag = Some(tag.into());
        self
    }
}

/// ⭐⭐⭐⭐ **渲染一个弹层**（纯函数）。
///
/// ## 返回
///
/// [`ModalView`]：结构 + **相对路径**的可聚焦清单 + 建议初始焦点。
///
/// ## ⭐ 路径口径（**必须看，否则会用错**）
///
/// - 弹层**根** = 弹层容器（`class_name` 那个）⇒ ⇒ 它的**子节点**下标从 0 开始；
/// - `focusable` 里是**相对弹层根**的路径 ⇒ ⇒ 绝对路径 = 前缀 + 相对路径
///   ⇒ ⇒ 用 [`ModalView::absolute_focusable`] / [`ModalView::absolute_initial_focus`] 拼；
/// - 遮罩（若给了 `overlay_class_name`）是**弹层根的父级**⇒ ⇒
///   ⇒ ⇒ **不属于**弹层结构 ⇒ ⇒ 不进 `focusable`（也**不该**可聚焦）。
///
/// ## 焦点顺序
///
/// ⇒ ⇒ 关闭按钮（若有）**在前** ⇒ ⇒ 内容里的可聚焦元素在后
/// ⇒ ⇒ ⇒ 所以 `initial_focus` 默认落在**关闭按钮**上
/// ⇒ ⇒ ⇒ （⚠️ 常见无障碍惯例：焦点落在**对话框内第一个可操作元素**，
/// ⇒ ⇒ ⇒ 关闭按钮算一个 ⇒ ⇒ ⇒ 但**更稳妥**的是落在**确认/首要动作**上
/// ⇒ ⇒ ⇒ ⇒ 所以 `initial_focus` **可由调用方覆盖**）。
pub fn modal(props: &ModalProps) -> ModalView {
    let mut focusable: Vec<Vec<usize>> = Vec::new();
    let mut children: Vec<VNode> = Vec::new();

    // ① 标题（若有）⇒ ⇒ 标题**不**进可聚焦清单（标题通常不可聚焦）。
    if let Some(title) = props.title.as_ref() {
        let tag = props
            .title_tag
            .clone()
            .unwrap_or_else(|| String::from("h2"));
        let mut title_node = VNode::element(tag.as_str()).child(VNode::text(title.clone()));
        if let Some(class_name) = props.title_class_name.as_ref() {
            title_node = title_node.class(class_name.as_str());
        }
        children.push(title_node.build());
    }

    // ② 关闭按钮（若有）⇒ ⇒ **进**可聚焦清单（它是弹层里第一个可操作元素）。
    if let Some(close_class) = props.close_class_name.as_ref() {
        let mut close = VNode::element("button").class(close_class.as_str());
        if let Some(label) = props.close_aria_label.as_ref() {
            close = close.attr("aria-label", label.as_str());
        }
        if let Some(handler) = props.on_close.as_ref() {
            close = close.handler(apeireth_ui_dom::dom::EventKind::Click, handler.clone());
        }
        children.push(close.build());
        focusable.push(vec![children.len() - 1]);
    }

    // ③ 内容 ⇒ ⇒ 内容里的可聚焦元素由调用方自己报（组件**不猜**）。
    let mut content = VNode::element("div");
    if let Some(class_name) = props.content_class_name.as_ref() {
        content = content.class(class_name.as_str());
    }
    children.push(content.child(props.children.render()).build());

    // ④ 弹层根 ⇒ ⇒ 语义属性（`role=dialog` / `aria-modal`）。
    let mut root = VNode::element("div").class(props.class_name.as_str());
    if let Some(role) = props.role.as_ref() {
        root = root.attr("role", role.as_str());
    }
    // ⭐ `aria-modal=true` ⇒ ⇒ 读屏软件知道「这是模态」⇒ ⇒ **外面**的内容要屏蔽。
    root = root.attr("aria-modal", "true");
    if let Some(label) = props.aria_label.as_ref() {
        root = root.attr("aria-label", label.as_str());
    }
    for child in children {
        root = root.child(child);
    }
    let root = root.build();

    // ⑤ 遮罩（若有）⇒ ⇒ 遮罩是**弹层根的父级**⇒ ⇒ 不进可聚焦清单。
    let tree = match props.overlay_class_name.as_ref() {
        Some(overlay_class) => VNode::element("div")
            .class(overlay_class.as_str())
            .child(root)
            .build(),
        None => root,
    };

    // ⑥ 初始焦点 ⇒ ⇒ 默认落在关闭按钮（若有）⇒ ⇒ 否则 `None`（调用方自定）。
    let initial_focus = focusable.first().cloned();

    ModalView {
        tree,
        focusable,
        initial_focus,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_ui_dom::vnode::AttrValue;

    /// ⭐⭐⭐ **弹层结构带语义属性**（`role=dialog` / `aria-modal=true`）。
    ///
    /// ## 为什么这条重要
    ///
    /// ⇒ ⇒ 弹层的**语义**是「模态对话框」⇒ ⇒ 读屏软件据此**屏蔽外面**的内容
    /// ⇒ ⇒⇒ 缺 `aria-modal` ⇒ ⇒⇒ 读屏用户会**在弹层里读到背景内容**
    /// ⇒ ⇒⇒ ⇒ 而这在**视觉上完全看不出来**（页面看着正常）⇒ ⇒⇒ 极难查。
    #[test]
    fn modal_carries_the_dialog_semantics() {
        let props = ModalProps::new("dlg").aria_label("删除确认");
        let view = modal(&props);
        assert_eq!(view.tree.tag(), Some("div"));
        assert_eq!(
            view.tree.attr_value("role"),
            Some(&AttrValue::Text(String::from("dialog"))),
            "⭐ 弹层必须有 role=dialog"
        );
        assert_eq!(
            view.tree.attr_value("aria-modal"),
            Some(&AttrValue::Text(String::from("true"))),
            "⭐ 弹层必须有 aria-modal=true"
        );
        assert_eq!(
            view.tree.attr_value("aria-label"),
            Some(&AttrValue::Text(String::from("删除确认"))),
            "⭐ 弹层必须有 aria-label"
        );
    }

    /// ⭐⭐⭐ **关闭按钮进可聚焦清单**，且**初始焦点**落在它上面。
    ///
    /// ⇒ ⇒ 关闭按钮必须**可聚焦** ⇒ ⇒ 否则键盘用户**关不掉弹层**。
    #[test]
    fn close_button_is_focusable_and_is_the_default_initial_focus() {
        let props = ModalProps::new("dlg")
            .close_class("dlg-x")
            .close_aria_label("关闭");
        let view = modal(&props);
        assert_eq!(
            view.focusable.len(),
            1,
            "⭐ 关闭按钮必须进可聚焦清单（否则键盘用户关不掉）"
        );
        assert_eq!(
            view.initial_focus,
            Some(vec![0]),
            "⭐ 默认初始焦点应落在关闭按钮上（**无标题** ⇒ ⇒ 按钮在第 0 位）"
        );
    }

    /// ⭐⭐⭐ **没有关闭按钮时初始焦点为 `None`**（由调用方定）。
    #[test]
    fn without_a_close_button_the_initial_focus_is_none() {
        let view = modal(&ModalProps::new("dlg"));
        assert!(view.focusable.is_empty(), "⭐ 没有关闭按钮 ⇒ ⇒ 清单为空");
        assert!(
            view.initial_focus.is_none(),
            "⭐ 没有关闭按钮 ⇒ ⇒ 初始焦点交给调用方"
        );
    }

    /// ⭐⭐⭐ **遮罩是弹层根的父级 ⇒ ⇒ 不进可聚焦清单**。
    ///
    /// ⇒ ⇒ 遮罩若可聚焦 ⇒ ⇒ Tab 会停在遮罩上 ⇒ ⇒ 用户**出不来**。
    #[test]
    fn the_overlay_is_not_focusable() {
        let props = ModalProps::new("dlg")
            .overlay_class("ovl")
            .close_class("dlg-x");
        let view = modal(&props);
        // ⭐ 有遮罩 ⇒ ⇒ 根是遮罩 ⇒ ⇒ 弹层容器是它的**子节点**。
        assert_eq!(view.tree.tag(), Some("div"));
        assert_eq!(
            view.tree.attr_value("class"),
            Some(&AttrValue::Text(String::from("ovl"))),
            "⭐ 遮罩必须存在"
        );
        assert_eq!(
            view.tree.children().len(),
            1,
            "⭐ 遮罩下只有弹层容器一个子节点"
        );
        assert_eq!(
            view.tree.children()[0].attr_value("class"),
            Some(&AttrValue::Text(String::from("dlg"))),
            "⭐ 弹层容器在遮罩里面"
        );
        assert_eq!(
            view.focusable.len(),
            1,
            "⭐ 遮罩**不可聚焦**（否则 Tab 会停在遮罩上）"
        );
    }

    /// ⭐⭐⭐ **相对路径 ⇒ ⇒ 绝对路径**（前缀拼接）。
    ///
    /// ⇒ ⇒ 这是「组件给相对路径、调用方拼前缀」这条约定的**可执行契约**。
    #[test]
    fn relative_paths_become_absolute_with_the_prefix() {
        let props = ModalProps::new("dlg").close_class("dlg-x");
        let view = modal(&props);
        let absolute = view.absolute_focusable(&[2]);
        assert_eq!(
            absolute,
            vec![vec![2, 0]],
            "⭐ 相对 [0]（无标题 ⇒ ⇒ 关闭按钮在第 0 位）+ 前缀 [2] ⇒ ⇒ 绝对 [2,0]"
        );
        assert_eq!(
            view.absolute_initial_focus(&[2]),
            Some(vec![2, 0]),
            "⭐ 初始焦点同样要拼前缀"
        );
    }

    /// ⭐⭐⭐ **组件不写内联样式、不设 id**（L2 契约 C3/C4）。
    #[test]
    fn the_component_never_writes_inline_styles_or_ids() {
        let props = ModalProps::new("dlg")
            .overlay_class("ovl")
            .title("标题")
            .close_class("dlg-x");
        let view = modal(&props);
        assert!(
            view.tree.attr_value("style").is_none(),
            "⭐ 组件不得写内联样式"
        );
        assert!(view.tree.attr_value("id").is_none(), "⭐ 组件不得设 id");
        // ⭐ 递归检查所有子节点。
        fn walk(node: &VNode) -> bool {
            if node.attr_value("style").is_some() || node.attr_value("id").is_some() {
                return false;
            }
            node.children().iter().all(walk)
        }
        assert!(walk(&view.tree), "⭐ 子节点也不得写内联样式 / 设 id");
    }

    /// ⭐⭐⭐ **关闭按钮带无障碍标签**（图标按钮必须有名）。
    #[test]
    fn the_close_button_carries_its_accessible_name() {
        let props = ModalProps::new("dlg")
            .close_class("dlg-x")
            .close_aria_label("关闭");
        let view = modal(&props);
        // ⭐ 关闭按钮在弹层根的第 0 个子节点（**无标题** ⇒ ⇒ 按钮就是第 0 位）。
        let close = &view.tree.children()[0];
        assert_eq!(close.tag(), Some("button"));
        assert_eq!(
            close.attr_value("aria-label"),
            Some(&AttrValue::Text(String::from("关闭"))),
            "⭐ 关闭按钮必须有 aria-label（否则读屏用户听不到它）"
        );
    }

    /// ⭐⭐⭐ **标题在关闭按钮之前**（DOM 顺序 ⇒ ⇒ 读屏顺序正确）。
    #[test]
    fn the_title_comes_before_the_close_button() {
        let props = ModalProps::new("dlg")
            .title("删除确认")
            .close_class("dlg-x");
        let view = modal(&props);
        let children = view.tree.children();
        assert_eq!(children.len(), 3, "⭐ 标题 + 关闭按钮 + 内容");
        assert_eq!(children[0].tag(), Some("h2"), "⭐ 标题在前");
        assert_eq!(children[1].tag(), Some("button"), "⭐ 关闭按钮其次");
        assert_eq!(children[2].tag(), Some("div"), "⭐ 内容最后");
    }

    /// ⭐⭐⭐⭐ **端到端接线**：组件 ⇒ ⇒ 引擎 ⇒ ⇒ **焦点陷阱真成立**。
    ///
    /// ## 它锁住什么
    ///
    /// 组件与引擎是**两个 crate** ⇒ ⇒ 单测各测各的 ⇒ ⇒ **接线处最容易错**
    /// ⇒ ⇒⇒ 路径口径（相对 vs 绝对）错一位 ⇒ ⇒ 焦点就**落在别的元素**上
    /// ⇒ ⇒⇒ 而单测**测不出来** ⇒ ⇒⇒ 所以必须有这条**跨 crate 端到端**测试。
    ///
    /// ## 验什么
    ///
    /// ① 组件给的**相对路径** + 挂载前缀 ⇒ ⇒ 拼出的绝对路径**指向真实元素**；
    /// ② `open_modal` 之后 ⇒ ⇒ 初始焦点**落在关闭按钮**上；
    /// ③ 焦点**顺序**只认弹层里的元素（陷阱生效）。
    #[test]
    fn the_component_wires_into_the_engine_modal_trap() {
        use apeireth_ui_dom::dom::{Dom, InMemoryDom};
        use apeireth_ui_dom::renderer::Renderer;

        // ① 建树：外层有个按钮（弹层外的元素）+ 弹层本体。
        let dom = InMemoryDom::new();
        let root = dom.create_element("div").expect("建根必须成功");
        let mut renderer = Renderer::new(dom, root);

        let props = ModalProps::new("dlg")
            .close_class("dlg-x")
            .close_aria_label("关闭");
        let view = modal(&props);

        // ② 挂载：外层按钮 + 弹层（弹层挂在路径 [1] ⇒ ⇒ 它的子节点从 [1, …] 起）。
        renderer
            .render_guarded(
                VNode::element("div")
                    .child(VNode::element("button").attr("id", "outside").build())
                    .child(view.tree.clone())
                    .build(),
            )
            .expect("挂载必须成功");

        // ③ 组件给的相对路径 ⇒ ⇒ 拼前缀 ⇒ ⇒ 绝对路径。
        let prefix = vec![1];
        let allowed = view.absolute_focusable(&prefix);
        assert_eq!(
            allowed,
            vec![vec![1, 0]],
            "⭐ 组件的相对路径 [0] + 前缀 [1] ⇒ ⇒ 绝对 [1,0]"
        );
        let initial = view
            .absolute_initial_focus(&prefix)
            .expect("初始焦点必须存在");

        // ④ 开模态 ⇒ ⇒ 初始焦点落在弹层里的关闭按钮。
        let allowed_refs: Vec<&[usize]> = allowed.iter().map(|path| path.as_slice()).collect();
        renderer
            .open_modal(&allowed_refs, &[1], Some(&initial))
            .expect("开模态必须成功");
        assert!(renderer.has_modal(), "⭐ 必须有模态层");

        // ⑤⭐ 焦点**顺序**是**全序列**（引擎口径：`focus_order` 给全部可聚焦元素，
        //   ⇒ ⇒ 陷阱是在 `focus_next` 里过滤的）⇒ ⇒ 断言「有 2 个」。
        let order = renderer.focus_order();
        assert_eq!(
            order.tabbable.len(),
            2,
            "⭐ 全序列 = 外层按钮 + 弹层里的关闭按钮"
        );

        // ⑥⭐⭐ **陷阱真生效**：Tab 之后 ⇒ ⇒ 焦点**仍落在弹层内**
        //   ⇒ ⇒ （外层按钮在 DOM 顺序上更靠前 ⇒ ⇒ 若无陷阱，Tab 会跑到它那里）
        renderer
            .focus_at_path(&[1, 0])
            .expect("聚焦到关闭按钮必须成功");
        //   ⇒ ⇒ 注意：陷阱内**只有 1 个**可聚焦元素 ⇒ ⇒ `focus_next` **返回 false**
        //   ⇒ ⇒ （这正是 E-171 修过的那个点：过滤后只有 1 项 ⇒ ⇒ **不动** ⇒ ⇒ `None`
        //   ⇒ ⇒ ⇒ 否则会「绕一圈回到自己」⇒ ⇒ 并**谎报**「移动了」）。
        let moved = renderer.focus_next(false).expect("Tab 必须成功");
        assert!(
            !moved,
            "⭐ 陷阱内只有 1 个可聚焦元素 ⇒ ⇒ Tab **不该**移动（否则会绕回自己）"
        );
        let after = renderer.current_focus_path();
        assert_eq!(
            after,
            Some(vec![1, 0]),
            "⭐ 陷阱内只有关闭按钮 ⇒ ⇒ Tab 后焦点**必须仍在**弹层内（不得跳到外层按钮）"
        );
    }

    /// ⭐⭐⭐ **关闭按钮的点击处理器真的挂上了**。
    #[test]
    fn the_close_button_carries_the_handler() {
        use std::cell::Cell;
        use std::rc::Rc;
        let clicks = Rc::new(Cell::new(0));
        let counter = clicks.clone();
        let props = ModalProps::new("dlg")
            .close_class("dlg-x")
            .on_close(ElementHandler::new(move |_event| {
                counter.set(counter.get() + 1);
            }));
        let view = modal(&props);
        let close = &view.tree.children()[0];
        match close {
            VNode::Element(element) => assert_eq!(
                element.handlers.len(),
                1,
                "⭐ 关闭按钮必须挂上处理器（否则点了没反应）"
            ),
            _ => panic!("⭐ 关闭按钮必须是元素节点"),
        }
    }
}
