//! 子内容插槽（匿名 `children`）。
//!
//! 设计文档 §6.2：`children` = 一个「渲染出一棵子树」的闭包。
//!
//! ```text
//! children() 在组件内被调用时，当场求值
//! ⇒ 子树在父组件的响应式上下文里渲染
//! ⇒ 子树能读到父组件的信号。
//! ```
//!
//! ## 为什么住在 `ui-dom`
//!
//! `children` 的产物是**虚拟节点树** ⇒ 类型依赖 `VNode` ⇒ 它必须住在能看见
//! `VNode` 的那一层（`ui-dom`）。`ui-core` 不认识虚拟树（它的边界是
//! 「什么时候、哪些东西需要重新算」，设计文档 §3.3.1）⇒ 回调（`Callback`）
//! 住在 `ui-core`，插槽（`Children`）住在 `ui-dom`。
//!
//! ## 具名插槽
//!
//! **不实现**（设计文档 §6.2）：本仓 `{#snippet}` 仅 2 处，两处都能用普通
//! 泛型函数参数表达 ⇒ 不需要具名插槽机制。

use std::rc::Rc;

use crate::vnode::VNode;

/// 匿名子内容插槽。
#[derive(Clone)]
pub struct Children {
    inner: Rc<dyn Fn() -> VNode>,
}

impl Children {
    /// 包装一个「渲染出子树」的闭包。
    pub fn new(render: impl Fn() -> VNode + 'static) -> Self {
        Self {
            inner: Rc::new(render),
        }
    }

    /// 渲染子树（当场求值）。
    pub fn render(&self) -> VNode {
        (self.inner)()
    }
}

impl Default for Children {
    fn default() -> Self {
        Self {
            inner: Rc::new(|| VNode::fragment(Vec::new())),
        }
    }
}

impl std::fmt::Debug for Children {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Children(..)")
    }
}

impl PartialEq for Children {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for Children {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn children_render_the_wrapped_subtree() {
        let children = Children::new(|| VNode::text("内容"));
        match children.render() {
            VNode::Text(text) => assert_eq!(text.data, "内容"),
            other => panic!("实测节点形态：{other:?}"),
        }
    }

    #[test]
    fn default_children_render_an_empty_fragment() {
        let children = Children::default();
        match children.render() {
            VNode::Fragment(items) => assert!(items.is_empty()),
            other => panic!("实测节点形态：{other:?}"),
        }
    }

    #[test]
    fn equality_is_pointer_identity() {
        let first = Children::new(|| VNode::text("a"));
        let second = Children::new(|| VNode::text("a"));
        assert_eq!(first, first.clone());
        assert_ne!(first, second);
    }
}
