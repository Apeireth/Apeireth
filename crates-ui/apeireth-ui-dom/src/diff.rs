//! 差分：两棵虚拟树比对 ⇒ 补丁序列。
//!
//! 设计文档 §5.2：
//! - **class 按 token 差分**（只动该动的那一个 class，避免连带抹掉别的 class）；
//! - 其余属性**键级差分**（新增 / 改值 / 删除）；
//! - 子节点**按索引对齐**（无 key 形态；keyed 形态的 LIS 最小移动待 P2）。
//!
//! 补丁路径 `path` = 从根到目标节点的子节点下标序列（空 = 根）。

use crate::vnode::{ElementVNode, VNode};

/// 差分补丁。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Patch {
    /// 整棵子树替换（类型不匹配时）。
    Replace {
        /// 目标路径。
        path: Vec<usize>,
        /// 新子树。
        node: VNode,
    },
    /// 就地改文本（不重建文本节点，见设计 §5.2.4）。
    SetText {
        /// 目标路径。
        path: Vec<usize>,
        /// 新文本。
        text: String,
    },
    /// 写属性（`None` ⇒ 删属性）。
    SetAttribute {
        /// 目标路径。
        path: Vec<usize>,
        /// 属性名。
        name: String,
        /// 属性值（`None` ⇒ 删除该属性）。
        value: Option<String>,
    },
    /// 增删 class token。
    SetClassTokens {
        /// 目标路径。
        path: Vec<usize>,
        /// 要加的 token。
        added: Vec<String>,
        /// 要删的 token。
        removed: Vec<String>,
    },
    /// 插入子节点（位置 = 父节点子列表下标）。
    InsertChildren {
        /// 目标路径。
        path: Vec<usize>,
        /// 插入位置。
        at: usize,
        /// 待插入的子节点。
        children: Vec<VNode>,
    },
    /// 移除子节点（从 `from` 起 `count` 个）。
    RemoveChildren {
        /// 目标路径。
        path: Vec<usize>,
        /// 起始下标。
        from: usize,
        /// 移除个数。
        count: usize,
    },
}

/// 比对两棵虚拟树，产出补丁序列。
pub fn diff(old: &VNode, new: &VNode) -> Vec<Patch> {
    let mut patches = Vec::new();
    diff_node(&mut patches, &[], old, new);
    patches
}

/// 比对单个节点。
fn diff_node(patches: &mut Vec<Patch>, path: &[usize], old: &VNode, new: &VNode) {
    match (old, new) {
        (VNode::Element(old_element), VNode::Element(new_element)) => {
            diff_attrs(patches, path, old_element, new_element);
            diff_children(patches, path, old_element, new_element);
        }
        (VNode::Text(old_text), VNode::Text(new_text)) => {
            if old_text.data != new_text.data {
                patches.push(Patch::SetText {
                    path: path.to_vec(),
                    text: new_text.data.clone(),
                });
            }
        }
        _ => {
            if old != new {
                patches.push(Patch::Replace {
                    path: path.to_vec(),
                    node: new.clone(),
                });
            }
        }
    }
}

/// 属性差分：class 按 token，其余按键。
fn diff_attrs(patches: &mut Vec<Patch>, path: &[usize], old: &ElementVNode, new: &ElementVNode) {
    let old_tokens = class_tokens(old);
    let new_tokens = class_tokens(new);
    let added: Vec<String> = new_tokens
        .iter()
        .filter(|token| !old_tokens.contains(token))
        .cloned()
        .collect();
    let removed: Vec<String> = old_tokens
        .iter()
        .filter(|token| !new_tokens.contains(token))
        .cloned()
        .collect();
    if !added.is_empty() || !removed.is_empty() {
        patches.push(Patch::SetClassTokens {
            path: path.to_vec(),
            added,
            removed,
        });
    }

    let mut names: Vec<String> = old
        .attrs
        .iter()
        .map(|(name, _)| name.clone())
        .filter(|name| name != "class")
        .collect();
    for (name, _) in &new.attrs {
        if name != "class" && !names.contains(name) {
            names.push(name.clone());
        }
    }
    for name in names {
        let old_value = old
            .attr_value(&name)
            .and_then(|value| value.to_attr_string());
        let new_value = new
            .attr_value(&name)
            .and_then(|value| value.to_attr_string());
        if old_value != new_value {
            patches.push(Patch::SetAttribute {
                path: path.to_vec(),
                name,
                value: new_value,
            });
        }
    }
}

/// 子节点差分：按索引对齐（无 key 形态）。
fn diff_children(patches: &mut Vec<Patch>, path: &[usize], old: &ElementVNode, new: &ElementVNode) {
    let common = old.children.len().min(new.children.len());
    for index in 0..common {
        let mut child_path = path.to_vec();
        child_path.push(index);
        diff_node(
            patches,
            &child_path,
            &old.children[index],
            &new.children[index],
        );
    }
    if new.children.len() > common {
        patches.push(Patch::InsertChildren {
            path: path.to_vec(),
            at: common,
            children: new.children[common..].to_vec(),
        });
    }
    if old.children.len() > common {
        patches.push(Patch::RemoveChildren {
            path: path.to_vec(),
            from: common,
            count: old.children.len() - common,
        });
    }
}

/// 读 class token 集合（保序）。
fn class_tokens(element: &ElementVNode) -> Vec<String> {
    match element.attr_value("class") {
        Some(crate::vnode::AttrValue::Text(text)) => {
            text.split_whitespace().map(str::to_owned).collect()
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vnode::VNode;

    #[test]
    fn identical_trees_produce_no_patches() {
        let tree = || {
            VNode::element("div")
                .class("a")
                .attr("id", "x")
                .child(VNode::text("hello"))
                .build()
        };
        assert!(diff(&tree(), &tree()).is_empty());
    }

    #[test]
    fn class_is_diffed_by_token_not_by_whole_string() {
        let old = VNode::element("div").class("a").class("b").build();
        let new = VNode::element("div").class("b").class("c").build();

        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1, "class 变更只出一条补丁：{patches:?}");
        match &patches[0] {
            Patch::SetClassTokens { added, removed, .. } => {
                assert_eq!(added, &vec![String::from("c")]);
                assert_eq!(removed, &vec![String::from("a")]);
            }
            other => panic!("实测补丁形态：{other:?}"),
        }
    }

    #[test]
    fn text_change_is_patched_in_place() {
        let old = VNode::element("p").child(VNode::text("a")).build();
        let new = VNode::element("p").child(VNode::text("b")).build();

        assert_eq!(
            diff(&old, &new),
            vec![Patch::SetText {
                path: vec![0],
                text: String::from("b")
            }]
        );
    }

    #[test]
    fn child_count_change_yields_insert_and_remove_patches() {
        let old = VNode::element("ul")
            .child(VNode::element("li").build())
            .child(VNode::element("li").build())
            .build();
        let new = VNode::element("ul")
            .child(VNode::element("li").build())
            .build();

        let patches = diff(&old, &new);
        assert_eq!(
            patches,
            vec![Patch::RemoveChildren {
                path: vec![],
                from: 1,
                count: 1
            }]
        );
    }

    #[test]
    fn type_mismatch_replaces_the_subtree() {
        let old = VNode::element("div").child(VNode::text("a")).build();
        let new = VNode::element("div")
            .child(VNode::element("span").build())
            .build();

        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert!(matches!(patches[0], Patch::Replace { .. }));
    }
}
