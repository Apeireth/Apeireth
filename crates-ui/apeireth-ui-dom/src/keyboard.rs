//! # 键盘语义层（Esc / Tab 顺序 / 方向键游标）
//!
//! ## 它解决什么
//!
//! r10 接棒文档 §5.2 要求引擎提供：
//! **稳定焦点 ID**、**进入/关闭模态层的焦点归还**、**Esc**、
//! **Tab 与按钮语义**、**方向键选择**、**中文 IME 合成期不误发送**。
//!
//! ⇒ ⇒ 其中**焦点 ID**（E-169 已做）、**IME 不误发送**（E-169 已做）、
//! **Enter / Shift+Enter**（E-169 已做）**已闭合**；
//! ⇒ ⇒ 本模块补上**剩下的三样**：**Esc**、**Tab 顺序**、**方向键游标**。
//!
//! ## ⭐⭐⭐ 为什么 Tab 顺序必须由引擎做（而不是留给浏览器）
//!
//! 浏览器的原生 `Tab` **只认 DOM 顺序**，**不知道**引擎的**焦点陷阱**。
//! ⇒ ⇒ 弹层打开时，用户按 `Tab` 会**跳出弹层** ⇒ ⇒ 焦点落到弹层外的元素上
//! ⇒ ⇒ ⇒ **「焦点陷阱」在浏览器里其实并不成立**（它只挡住了**引擎自己**的
//! [`crate::renderer::Renderer::focus_at_path`] 调用 ⇒ ⇒ 挡不住**用户按 Tab**）。
//!
//! ⇒ ⇒ 所以引擎必须**自己**接管 `Tab`：
//! ① 收集**可聚焦元素**（按 **DOM 顺序**）⇒ ⇒ ② 在**陷阱允许集合内**移动
//! ⇒ ⇒ ③ **`prevent_default`** ⇒ ⇒ 浏览器的原生 Tab **不再插手**。
//!
//! ## 可聚焦元素的判定
//!
//! **原生可聚焦标签**：`input` / `textarea` / `select` / `button` / `a`（有 `href`）/ `summary`
//! ⇒ ⇒ **加上**显式 `tabindex`（`tabindex` **不为负** ⇒ ⇒ 负值 = 不可 Tab 到，
//! 但**可编程聚焦** ⇒ ⇒ 本模块按「可编程聚焦」与「可 Tab 到」**分开**处理）。
//!
//! ⚠️ 判定**只看结构**（标签 + `tabindex` / `disabled`）⇒ ⇒ **不读**计算样式
//! ⇒ ⇒ `display:none` / `visibility:hidden` 的元素**仍会被列入** ⇒ ⇒ ⚠️ 这是**已知简化**
//! ⇒ ⇒ ⇒ 真正要精确 ⇒ ⇒ 需要宿主侧提供「是否可见」的查询（**未做**）。

/// ⭐ **允许集合判据**（路径 ⇒ 是否允许）。
pub type AllowedPredicate = dyn Fn(&[usize]) -> bool;

/// ⭐⭐⭐ **可聚焦元素**在树里的位置 + 判定结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Focusable {
    /// 从**挂载树根**到该元素的子节点下标路径（与 `Renderer::focus_at_path` 同一口径）。
    pub path: Vec<usize>,
    /// 是否**可 Tab 到**（`tabindex` 为负 ⇒ ⇒ 不可 Tab 到，但**可编程聚焦**）。
    pub tabbable: bool,
}

/// ⭐⭐⭐ 判定一个元素**是否可聚焦**（按标签 + `tabindex` + `disabled`）。
///
/// ## 口径
///
/// - **原生可聚焦标签** ⇒ ⇒ 可聚焦；
/// - 有 `tabindex` ⇒ ⇒ 以 `tabindex` 为准（**负数 = 不可 Tab 到**）；
/// - `disabled` ⇒ ⇒ **不可聚焦**（与浏览器一致）。
pub fn is_focusable(tag: &str, tabindex: Option<i64>, disabled: bool) -> bool {
    if disabled {
        return false;
    }
    if let Some(index) = tabindex {
        // ⭐ 显式 `tabindex` **覆盖**标签默认值：
        //    `tabindex >= 0` ⇒ ⇒ 可聚焦**且**可 Tab 到；
        //    `tabindex < 0`  ⇒ ⇒ **不可**聚焦到（浏览器里 `tabindex="-1"` 只保留
        //    **编程聚焦**，Tab **到不了**）⇒ ⇒ ⇒ 这里**与浏览器一致**。
        return index >= 0;
    }
    matches!(
        tag,
        "input" | "textarea" | "select" | "button" | "summary" | "a"
    )
}

/// ⭐⭐⭐ **可聚焦顺序**（DOM 顺序的扁平列表）。
///
/// ## 为什么引擎要自己收集
///
/// 浏览器的 `Tab` 只按 DOM 顺序 ⇒ ⇒ **不知道**陷阱 ⇒ ⇒ ⇒ 陷阱在浏览器里不成立
/// ⇒ ⇒ ⇒ 引擎必须自己算一份「**可 Tab 序列**」⇒ ⇒ 在其中移动。
#[derive(Debug, Default, Clone)]
pub struct FocusOrder {
    /// 可 Tab 到的元素（DOM 顺序）。
    pub tabbable: Vec<Focusable>,
    /// 可编程聚焦但**不可 Tab 到**的元素（DOM 顺序）⇒ ⇒ 例如 `tabindex="-1"`。
    pub focusable_only: Vec<Focusable>,
}

impl FocusOrder {
    /// ⭐ **下一个**（`shift` = `false`）或**上一个**（`shift` = `true`）可 Tab 到的元素。
    ///
    /// ## 口径
    ///
    /// - `current` = 当前聚焦元素的路径 ⇒ ⇒ 从它的**位置**接着走；
    /// - 到末尾 ⇒ ⇒ **回到开头**（循环）⇒ ⇒ ⇒ 与浏览器一致；
    /// - `allowed` = 陷阱允许的路径集合（`None` = 无限制）⇒ ⇒ **只在里面走**。
    ///
    /// ## 返回 `None`
    ///
    /// ⇒ ⇒ **没有下一个**（序列为空，或陷阱内一个都没有）⇒ ⇒
    /// ⇒ ⇒ 调用方**不要** `prevent_default` ⇒ ⇒ ⇒ 让浏览器自己处理（退化为无 Tab 行为）。
    pub fn next(
        &self,
        current: Option<&[usize]>,
        shift: bool,
        allowed: Option<&AllowedPredicate>,
    ) -> Option<&Focusable> {
        if self.tabbable.is_empty() {
            return None;
        }
        // ⭐⭐⭐ **先按允许集合过滤，再在过滤后的序列里循环**。
        //
        // ## 为什么必须先过滤（而不是「全序列循环 + 逐个跳过」）
        //
        // 旧写法（先算下标、再逐个检查允许集合）⇒ ⇒ 在「陷阱内只有 1 项」时
        // ⇒ ⇒ 绕一圈会**回到那 1 项自己** ⇒ ⇒ 返回「移动了」⇒ ⇒
        // ⇒ ⇒ 但焦点**实际没动** ⇒ ⇒ ⇒ 调用方据此 `prevent_default`
        // ⇒ ⇒ ⇒ **Tab 在模态里彻底失灵**（用户按 Tab 什么也不发生）。
        //
        // ⇒ ⇒ ⇒ 正确口径：**陷阱内**的序列**只有一项** ⇒ ⇒ **不动** ⇒ ⇒ `None`。
        let candidates: Vec<&Focusable> = self
            .tabbable
            .iter()
            .filter(|item| allowed.is_none_or(|predicate| predicate(&item.path)))
            .collect();
        if candidates.is_empty() {
            return None;
        }
        let total = candidates.len();
        // ⭐ 陷阱内只有一项 ⇒ ⇒ **不动**（见上面的说明）
        if total == 1 {
            return None;
        }
        let index = current.and_then(|path| {
            candidates
                .iter()
                .position(|item| item.path.as_slice() == path)
        });
        // ⭐ 步进：正向 `+1`、反向 `-1`（**在允许集合内**循环）。
        let step = if shift { total - 1 } else { 1 };
        let base = match index {
            Some(index) => (index + step) % total,
            // ⭐ 没有当前项 ⇒ ⇒ 正向从**第一项**开始，反向从**最后一项**开始。
            None => {
                if shift {
                    total - 1
                } else {
                    0
                }
            }
        };
        Some(candidates[base])
    }
}

/// ⭐⭐⭐ **方向键游标**（列表 / 菜单的「当前项」）。
///
/// ## 为什么它与「焦点」是**两个**东西
///
/// - **焦点** = 浏览器焦点（哪个元素接收按键）⇒ ⇒ 引擎已有 [`crate::focus::FocusTracker`]；
/// - **游标** = 列表里的「当前项」（**不一定是焦点**）⇒ ⇒ 例如一个 `listbox`
///   里 ↑↓ 移动的是**选项**，焦点可以仍在列表容器上。
///
/// ⇒ ⇒ ⇒ 两者**分开**⇒ ⇒ ⇒ 应用可以只用其一，也可以只用其一。
#[derive(Debug, Default, Clone)]
pub struct Cursor {
    /// 当前项的路径（`None` = 还没有当前项）。
    pub current: Option<Vec<usize>>,
    /// 游标是否**循环**（到末尾回到开头）。
    pub wrap: bool,
}

impl Cursor {
    /// ⭐ 在**同层**的候选里移动游标。
    ///
    /// ## 口径
    ///
    /// - `candidates` = 同层的可选项（**调用方给**，按视觉顺序）⇒ ⇒
    ///   引擎**不猜**「同层」是什么（那需要布局 ⇒ ⇒ 布局归 CSS）；
    /// - `forward` = `true` ⇒ 下移；`false` ⇒ 上移；
    /// - `current` = 当前项的路径 ⇒ ⇒ 找不到 ⇒ ⇒ 从头/从尾开始。
    ///
    /// ## 返回 `None`
    ///
    /// ⇒ ⇒ 没有候选 ⇒ ⇒ 游标不动。
    pub fn move_within(&mut self, candidates: &[Vec<usize>], forward: bool) -> Option<&Vec<usize>> {
        if candidates.is_empty() {
            return None;
        }
        let index = self.current.as_ref().and_then(|path| {
            candidates
                .iter()
                .position(|item| item.as_slice() == path.as_slice())
        });
        let next = match index {
            Some(index) => {
                if forward {
                    if index + 1 < candidates.len() {
                        index + 1
                    } else if self.wrap {
                        0
                    } else {
                        index
                    }
                } else if index > 0 {
                    index - 1
                } else if self.wrap {
                    candidates.len() - 1
                } else {
                    index
                }
            }
            None => {
                if forward {
                    0
                } else {
                    candidates.len() - 1
                }
            }
        };
        self.current = Some(candidates[next].clone());
        self.current.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⭐ 原生可聚焦标签判定。
    #[test]
    fn native_focusable_tags_are_recognised() {
        for tag in ["input", "textarea", "select", "button", "summary", "a"] {
            assert!(is_focusable(tag, None, false), "{tag} 应可聚焦");
        }
        for tag in ["div", "span", "p", "section", "label"] {
            assert!(!is_focusable(tag, None, false), "{tag} 不该可聚焦");
        }
    }

    /// ⭐ `disabled` ⇒ ⇒ 不可聚焦（与浏览器一致）。
    #[test]
    fn disabled_elements_are_not_focusable() {
        assert!(!is_focusable("input", None, true));
        assert!(!is_focusable("button", Some(0), true));
    }

    /// ⭐ `tabindex` 显式存在时以它为准（负数 = 不可 Tab 到，但可编程聚焦）。
    #[test]
    fn explicit_tabindex_overrides_tag_defaults() {
        assert!(is_focusable("div", Some(0), false), "tabindex=0 ⇒ 可聚焦");
        assert!(
            !is_focusable("div", Some(-1), false),
            "⭐ tabindex=-1 ⇒ 不可聚焦到（与浏览器一致：只保留编程聚焦）"
        );
        assert!(
            !is_focusable("input", Some(-1), false),
            "⭐ tabindex=-1 ⇒ 覆盖标签默认值"
        );
        assert!(is_focusable("input", Some(0), false), "tabindex=0 ⇒ 可聚焦");
    }

    /// ⭐⭐ **Tab 顺序**：从当前位置往下走，**到末尾回到开头**（与浏览器一致）。
    #[test]
    fn tab_order_advances_and_wraps() {
        let order = FocusOrder {
            tabbable: vec![
                Focusable {
                    path: vec![0],
                    tabbable: true,
                },
                Focusable {
                    path: vec![1],
                    tabbable: true,
                },
                Focusable {
                    path: vec![2],
                    tabbable: true,
                },
            ],
            focusable_only: Vec::new(),
        };
        let first = order.next(Some(&[0]), false, None).expect("应有下一个");
        assert_eq!(first.path, vec![1]);
        let second = order.next(Some(&[1]), false, None).expect("应有下一个");
        assert_eq!(second.path, vec![2]);
        // ⭐ 末尾 ⇒ ⇒ 回到开头
        let wrapped = order.next(Some(&[2]), false, None).expect("应能绕回");
        assert_eq!(wrapped.path, vec![0]);
        // ⭐ Shift+Tab 反向
        let back = order.next(Some(&[1]), true, None).expect("应有上一个");
        assert_eq!(back.path, vec![0]);
    }

    /// ⭐⭐ **Tab 顺序尊重陷阱**：不在允许集合里的**跳过**。
    #[test]
    fn tab_order_skips_paths_outside_the_trap() {
        let order = FocusOrder {
            tabbable: vec![
                Focusable {
                    path: vec![0],
                    tabbable: true,
                },
                Focusable {
                    path: vec![1],
                    tabbable: true,
                },
                Focusable {
                    path: vec![2],
                    tabbable: true,
                },
            ],
            focusable_only: Vec::new(),
        };
        let allowed = |path: &[usize]| path[0] != 1;
        let next = order
            .next(Some(&[0]), false, Some(&allowed))
            .expect("应跳过 1");
        assert_eq!(next.path, vec![2], "⭐ 陷阱外的元素必须被跳过");
    }

    /// ⭐⭐ **陷阱内两项时，Tab 在它们之间循环**（E-171 的关键口径）。
    ///
    /// ## 为什么这条必须单列
    ///
    /// 修「先过滤再循环」之前 ⇒ ⇒ 全序列循环会**绕过**陷阱内的两项
    /// ⇒ ⇒ 或者在只有一项时**停在原地却报告「移动了」**
    /// ⇒ ⇒ ⇒ 前者让焦点跑到陷阱外，后者让 Tab 失灵。
    #[test]
    fn tab_cycles_within_the_allowed_subset() {
        let order = FocusOrder {
            tabbable: vec![
                Focusable {
                    path: vec![0],
                    tabbable: true,
                },
                Focusable {
                    path: vec![1],
                    tabbable: true,
                },
                Focusable {
                    path: vec![2],
                    tabbable: true,
                },
            ],
            focusable_only: Vec::new(),
        };
        // 只允许 0 与 2 ⇒ ⇒ 1 必须被跳过
        let allowed = |path: &[usize]| path[0] != 1;
        let next = order
            .next(Some(&[0]), false, Some(&allowed))
            .expect("应能跳到 2");
        assert_eq!(next.path, vec![2], "⭐ 必须跳过不允许的 1");
        // ⭐ 从 2 再往下 ⇒ ⇒ 绕回 0（**只在允许集合内**循环）
        let wrapped = order
            .next(Some(&[2]), false, Some(&allowed))
            .expect("应能绕回 0");
        assert_eq!(wrapped.path, vec![0], "⭐ 循环必须只在允许集合内");
    }

    /// ⭐ **陷阱内一个都没有** ⇒ ⇒ 返回 `None`（调用方不该 `prevent_default`）。
    #[test]
    fn no_candidate_inside_the_trap_yields_none() {
        let order = FocusOrder {
            tabbable: vec![Focusable {
                path: vec![0],
                tabbable: true,
            }],
            focusable_only: Vec::new(),
        };
        let allowed = |_: &[usize]| false;
        assert!(order.next(None, false, Some(&allowed)).is_none());
    }

    /// ⭐⭐ **方向键游标**：下移到末尾**循环**（默认开）。
    #[test]
    fn cursor_moves_and_wraps() {
        let candidates = vec![vec![0], vec![1], vec![2]];
        let mut cursor = Cursor {
            current: None,
            wrap: true,
        };
        assert_eq!(
            cursor.move_within(&candidates, true).cloned(),
            Some(vec![0]),
            "⭐ 首次下移应到第一项"
        );
        cursor.move_within(&candidates, true); // → [1]
        cursor.move_within(&candidates, true); // → [2]
        cursor.move_within(&candidates, true); // → 绕回 [0]
        assert_eq!(
            cursor.current.clone(),
            Some(vec![0]),
            "⭐ 到末尾后应循环回开头"
        );
    }

    /// ⭐ **不循环**时停在边界（`wrap = false`）。
    #[test]
    fn cursor_without_wrap_stops_at_the_edge() {
        let candidates = vec![vec![0], vec![1]];
        let mut cursor = Cursor {
            current: Some(vec![1]),
            wrap: false,
        };
        cursor.move_within(&candidates, true);
        assert_eq!(cursor.current.clone(), Some(vec![1]), "⭐ 不循环时不得越界");
    }
}
