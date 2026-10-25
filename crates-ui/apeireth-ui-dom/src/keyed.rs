//! keyed 列表差分：按 key 配对 + **最长递增子序列（LIS）** 最小移动。
//!
//! 设计文档 §5.2.3：
//!
//! | 形态 | 语义 | 差分策略 |
//! |---|---|---|
//! | **keyed**（`{#each xs as x (x.id)}`） | 节点身份由 key 决定 | 按 key 配对；用 LIS 求出「可原地保留」的子集，其余**移动**；**移动次数 = 配对数 − LIS 长度**（下界） |
//! | **unkeyed**（`{#each xs as x}`） | 节点身份由位置决定 | 按索引对齐（设计文档 §5.2.2） |
//!
//! ## 身份规则不许混用（显式，不静默降级）
//!
//! 一侧带 key、另一侧不带 ⇒ 整体走 **unkeyed**（位置身份）。理由：身份规则
//! 混用会产生**无法判定**的状态漂移（同一个节点既按 key 又按位置寻址）。
//! 这条规则写在 [`list_diff_mode`] 里，可被调用方直接查询。
//!
//! ## 本模块的产出边界
//!
//! [`KeyedDiff`] 是**差分计划**（该复用谁、该移动谁、该新建谁、该移除谁），
//! **不是** DOM 操作序列。DOM 操作序列由 `ui-dom` 的渲染器（P2）消费 ——
//! 渲染器尚未存在，故本模块**不产出** `Patch`（P0 的 `Patch` 是按下标寻址的
//! 索引模型，与 keyed 身份模型不兼容；两者的统一属 P2 渲染器的工作）。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::vnode::{Key, VNode};

/// 列表差分模式（设计文档 §5.2.3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListDiffMode {
    /// keyed：子节点身份由 key 决定（按 key 配对 + LIS 最小移动）。
    Keyed,
    /// unkeyed：子节点身份由位置决定（按索引对齐，设计文档 §5.2.2）。
    Unkeyed,
}

/// keyed 差分的失败原因。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyedDiffError {
    /// 同一侧出现**重复 key**：身份不唯一 ⇒ 差分不可判定。
    DuplicateKey {
        /// 重复的 key。
        key: Key,
        /// 首次出现的位置（重复所在列表的下标）。
        first: usize,
        /// 再次出现的位置（重复所在列表的下标）。
        second: usize,
    },
}

impl fmt::Display for KeyedDiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyedDiffError::DuplicateKey { key, first, second } => {
                write!(
                    f,
                    "DuplicateKey: key “{key}” 在同一列表里出现多次（位置 {first} 与 {second}）"
                )
            }
        }
    }
}

impl std::error::Error for KeyedDiffError {}

/// 新顺序下的一个槽位：复用旧子节点，或新建一个。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyedSlot {
    /// 复用旧子节点（旧列表下标）。
    Reuse(usize),
    /// 新建子节点。
    New(VNode),
}

/// keyed 列表的差分计划。
///
/// **不变量**（可被机械核对）：
/// - `moved.len() == 配对数 − retained.len()` ⇒ 移动次数达到 LIS 下界；
/// - `retained` 是配对序列（按新顺序）的**最长严格递增子序列**；
/// - `matched.len() == 新列表长度`，`inserted.len() + 配对数 == 新列表长度`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyedDiff {
    mode: ListDiffMode,
    matched: Vec<Option<usize>>,
    removed: Vec<usize>,
    moved: Vec<(usize, usize)>,
    retained: Vec<usize>,
    /// 「原地保留」旧下标的**集合**（供 O(1) 判定）。
    ///
    /// ## 为什么要有它（2026-10-25 修）
    ///
    /// 渲染器逐个判定「这个配对要不要移动」⇒ 若用 `retained().contains(&index)`
    /// （`Vec` 线性查）⇒ 每个子节点 O(N) ⇒ **整体 O(N²)**。
    /// ⇒ ⇒ 集合查表 **O(1)** ⇒ 整体 O(N)。
    retained_set: BTreeSet<usize>,
    inserted: Vec<(usize, VNode)>,
    updated: Vec<(usize, usize)>,
}

impl KeyedDiff {
    /// 差分模式。
    pub fn mode(&self) -> ListDiffMode {
        self.mode
    }

    /// 新子节点（按新顺序）→ 旧子节点下标；`None` = 新建。
    pub fn matched(&self) -> &[Option<usize>] {
        &self.matched
    }

    /// 需要移除的旧子节点下标（升序）。
    pub fn removed(&self) -> &[usize] {
        &self.removed
    }

    /// 需要移动的子节点：(旧下标 → 新下标)，按新下标升序。
    pub fn moved(&self) -> &[(usize, usize)] {
        &self.moved
    }

    /// 原地保留的子节点旧下标（升序）—— 构成 LIS。
    pub fn retained(&self) -> &[usize] {
        &self.retained
    }

    /// 某个旧下标是否**原地保留**（在 LIS 里 ⇒ 不需要移动）。
    ///
    /// **O(1)**（集合查表）⇒ 渲染器逐个判定时**不是** O(N²)。
    pub fn is_retained(&self, old_index: usize) -> bool {
        self.retained_set.contains(&old_index)
    }

    /// 某个旧下标是否**需要移动**（不在 LIS 里）。
    pub fn is_moved(&self, old_index: usize) -> bool {
        !self.is_retained(old_index)
    }

    /// 需要新建的子节点：(新下标, 节点)，按新下标升序。
    pub fn inserted(&self) -> &[(usize, VNode)] {
        &self.inserted
    }

    /// 内容需要更新的配对子节点：(旧下标 → 新下标)。
    pub fn updated(&self) -> &[(usize, usize)] {
        &self.updated
    }

    /// 配对成功的子节点数（复用了旧节点的项数）。
    pub fn matched_count(&self) -> usize {
        self.matched.iter().filter(|slot| slot.is_some()).count()
    }

    /// 移动次数（LIS 下界：配对数 − LIS 长度）。
    pub fn move_count(&self) -> usize {
        self.moved.len()
    }

    /// 原地保留的子节点数（LIS 长度）。
    pub fn retained_count(&self) -> usize {
        self.retained.len()
    }

    /// 新顺序下的槽位序列（渲染器据此复用 / 新建）。
    pub fn slots(&self) -> Vec<KeyedSlot> {
        self.matched
            .iter()
            .enumerate()
            .map(|(new_index, slot)| match slot {
                Some(old_index) => KeyedSlot::Reuse(*old_index),
                None => KeyedSlot::New(self.inserted_node_at(new_index)),
            })
            .collect()
    }

    /// 读某个新下标对应的新建节点（内部用）。
    fn inserted_node_at(&self, new_index: usize) -> VNode {
        self.inserted
            .iter()
            .find(|(index, _)| *index == new_index)
            .map(|(_, node)| node.clone())
            .expect("新建槽位必然在 inserted 里")
    }
}

/// 求**最长严格递增子序列**的下标（按递增顺序返回）。
///
/// - 输入可以是任意序列；输出是「长度最大」的递增子序列的下标集合；
/// - 复杂度 O(n log n)（二分 + 尾部最小值 + 回溯）；
/// - 严格递增 ⇒ 相等的值不会被同时选入。
pub fn longest_increasing_subsequence(values: &[usize]) -> Vec<usize> {
    if values.is_empty() {
        return Vec::new();
    }
    // tails[k] = 长度为 k+1 的递增子序列的最小末尾值；
    // tail_index[k] = 该末尾值对应的下标；
    // prev[i] = 下标 i 在最优子序列里的前驱。
    let mut tails: Vec<usize> = Vec::with_capacity(values.len());
    let mut tail_index: Vec<usize> = Vec::with_capacity(values.len());
    let mut prev: Vec<Option<usize>> = vec![None; values.len()];

    for (index, &value) in values.iter().enumerate() {
        // 第一个 >= value 的位置 ⇒ 保持**严格**递增。
        let position = tails.partition_point(|tail| *tail < value);
        if position > 0 {
            prev[index] = Some(tail_index[position - 1]);
        }
        if position == tails.len() {
            tails.push(value);
            tail_index.push(index);
        } else {
            tails[position] = value;
            tail_index[position] = index;
        }
    }

    let mut result = Vec::with_capacity(tails.len());
    let mut cursor = tail_index.last().copied();
    while let Some(index) = cursor {
        result.push(index);
        cursor = prev[index];
    }
    result.reverse();
    result
}

/// 判定一组子节点该走哪种差分模式。
///
/// 规则（显式）：**两侧全部子节点都带 key** ⇒ [`ListDiffMode::Keyed`]；
/// 否则 ⇒ [`ListDiffMode::Unkeyed`]（身份由位置决定）。
pub fn list_diff_mode(old: &[VNode], new: &[VNode]) -> ListDiffMode {
    if old.is_empty() && new.is_empty() {
        return ListDiffMode::Unkeyed;
    }
    let all_keyed = old.iter().all(|child| child.key().is_some())
        && new.iter().all(|child| child.key().is_some());
    if all_keyed {
        ListDiffMode::Keyed
    } else {
        ListDiffMode::Unkeyed
    }
}

/// keyed 列表差分：按 key 配对 + LIS 最小移动。
///
/// 返回 [`KeyedDiffError::DuplicateKey`] 的情形是**任一侧出现重复 key**
/// （身份不唯一 ⇒ 差分不可判定）⇒ 显式报错，不静默取其一。
///
/// > **两侧都要查**（P2 补齐）：新列表里的重复 key 会让两个新槽位指向**同一个**
/// > 旧节点 ⇒ 差分结果不可判定（渲染器会把同一个节点挂两次）。旧列表的重复
/// > key 同理。
pub fn diff_keyed(old: &[VNode], new: &[VNode]) -> Result<KeyedDiff, KeyedDiffError> {
    match list_diff_mode(old, new) {
        ListDiffMode::Unkeyed => Ok(diff_unkeyed(old, new)),
        ListDiffMode::Keyed => diff_by_key(old, new),
    }
}

/// 按索引对齐的差分（unkeyed，设计文档 §5.2.2）。
fn diff_unkeyed(old: &[VNode], new: &[VNode]) -> KeyedDiff {
    let common = old.len().min(new.len());
    let matched: Vec<Option<usize>> = (0..new.len())
        .map(|index| if index < common { Some(index) } else { None })
        .collect();
    let removed: Vec<usize> = (common..old.len()).collect();
    let inserted: Vec<(usize, VNode)> = (common..new.len())
        .map(|index| (index, new[index].clone()))
        .collect();
    let retained: Vec<usize> = (0..common).collect();
    let retained_set: BTreeSet<usize> = retained.iter().copied().collect();
    let moved: Vec<(usize, usize)> = Vec::new();
    let updated: Vec<(usize, usize)> = (0..common)
        .filter(|index| old[*index] != new[*index])
        .map(|index| (index, index))
        .collect();
    KeyedDiff {
        mode: ListDiffMode::Unkeyed,
        matched,
        removed,
        moved,
        retained,
        retained_set,
        inserted,
        updated,
    }
}

/// 按 key 配对的差分（keyed，设计文档 §5.2.3）。
fn diff_by_key(old: &[VNode], new: &[VNode]) -> Result<KeyedDiff, KeyedDiffError> {
    let mut old_index_by_key: BTreeMap<&Key, usize> = BTreeMap::new();
    for (index, child) in old.iter().enumerate() {
        let Some(key) = child.key() else {
            continue;
        };
        if let Some(first) = old_index_by_key.insert(key, index) {
            return Err(KeyedDiffError::DuplicateKey {
                key: key.clone(),
                first,
                second: index,
            });
        }
    }

    // 新列表同样要查重复 key：两个新槽位指向同一个旧节点 ⇒ 差分不可判定。
    let mut new_index_by_key: BTreeMap<&Key, usize> = BTreeMap::new();
    for (index, child) in new.iter().enumerate() {
        let Some(key) = child.key() else {
            continue;
        };
        if let Some(first) = new_index_by_key.insert(key, index) {
            return Err(KeyedDiffError::DuplicateKey {
                key: key.clone(),
                first,
                second: index,
            });
        }
    }

    let mut matched: Vec<Option<usize>> = Vec::with_capacity(new.len());
    let mut inserted: Vec<(usize, VNode)> = Vec::new();
    for (index, child) in new.iter().enumerate() {
        match child
            .key()
            .and_then(|key| old_index_by_key.get(key).copied())
        {
            Some(old_index) => matched.push(Some(old_index)),
            None => {
                matched.push(None);
                inserted.push((index, child.clone()));
            }
        }
    }

    // 被移除的旧子节点：它的 key 不在新列表里。
    let new_keys: BTreeSet<&Key> = new.iter().filter_map(|child| child.key()).collect();
    let removed: Vec<usize> = old
        .iter()
        .enumerate()
        .filter(|(_, child)| match child.key() {
            Some(key) => !new_keys.contains(key),
            None => true,
        })
        .map(|(index, _)| index)
        .collect();

    // 配对序列（按新顺序的旧下标）⇒ 其 LIS 即「可原地保留」的子集。
    let sequence: Vec<usize> = matched.iter().filter_map(|slot| *slot).collect();
    let lis_positions = longest_increasing_subsequence(&sequence);
    let retained: Vec<usize> = lis_positions
        .iter()
        .map(|position| sequence[*position])
        .collect();
    let retained_set: BTreeSet<usize> = retained.iter().copied().collect();
    let mut moved: Vec<(usize, usize)> = Vec::new();
    for (new_index, slot) in matched.iter().enumerate() {
        let Some(old_index) = slot else {
            continue;
        };
        if !retained_set.contains(old_index) {
            moved.push((*old_index, new_index));
        }
    }

    let mut updated: Vec<(usize, usize)> = Vec::new();
    for (new_index, slot) in matched.iter().enumerate() {
        let Some(old_index) = slot else {
            continue;
        };
        if old[*old_index] != new[new_index] {
            updated.push((*old_index, new_index));
        }
    }

    Ok(KeyedDiff {
        mode: ListDiffMode::Keyed,
        matched,
        removed,
        moved,
        retained,
        retained_set,
        inserted,
        updated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vnode::VNode;

    /// 造一个带 key 的元素。
    fn keyed(key: &str, text: &str) -> VNode {
        VNode::element("li")
            .key(key)
            .child(VNode::text(text))
            .build()
    }

    /// 造一个不带 key 的元素。
    fn plain(text: &str) -> VNode {
        VNode::element("li").child(VNode::text(text)).build()
    }

    // ── LIS 算法 ──────────────────────────────────────────────────────────

    #[test]
    fn lis_of_an_ordered_sequence_is_the_whole_sequence() {
        let indices = longest_increasing_subsequence(&[0, 1, 2, 3]);
        assert_eq!(indices, vec![0, 1, 2, 3]);
    }

    #[test]
    fn lis_of_a_reversed_sequence_has_length_one() {
        let indices = longest_increasing_subsequence(&[3, 2, 1, 0]);
        assert_eq!(indices.len(), 1, "逆序 ⇒ 只能保留一个");
    }

    #[test]
    fn lis_picks_the_longest_increasing_run() {
        let sequence = [0usize, 8, 1, 9, 2, 10];
        let indices = longest_increasing_subsequence(&sequence);
        let values: Vec<usize> = indices.iter().map(|i| sequence[*i]).collect();
        assert_eq!(values, vec![0, 1, 2, 10], "实测下标：{indices:?}");
        assert_eq!(
            indices.len(),
            4,
            "0 < 1 < 2 < 10 是最长递增子序列（长度 4）"
        );
        assert!(
            indices.windows(2).all(|pair| pair[0] < pair[1]),
            "返回的下标必须递增"
        );
    }

    #[test]
    fn lis_is_strict_so_equal_values_are_not_both_picked() {
        let indices = longest_increasing_subsequence(&[5, 5, 5]);
        assert_eq!(indices.len(), 1, "严格递增 ⇒ 相等值只取一个");
    }

    #[test]
    fn lis_of_an_empty_sequence_is_empty() {
        assert!(longest_increasing_subsequence(&[]).is_empty());
    }

    // ── 模式判定 ──────────────────────────────────────────────────────────

    #[test]
    fn all_keyed_children_mean_keyed_mode() {
        let old = vec![keyed("a", "1")];
        let new = vec![keyed("b", "2")];
        assert_eq!(list_diff_mode(&old, &new), ListDiffMode::Keyed);
    }

    #[test]
    fn missing_key_means_unkeyed_mode() {
        let old = vec![keyed("a", "1")];
        let new = vec![plain("2")];
        assert_eq!(
            list_diff_mode(&old, &new),
            ListDiffMode::Unkeyed,
            "身份规则不许混用"
        );
    }

    // ── keyed 差分 ────────────────────────────────────────────────────────

    #[test]
    fn keyed_diff_pairs_children_by_key_not_by_position() {
        let old = vec![keyed("a", "1"), keyed("b", "2")];
        let new = vec![keyed("b", "2"), keyed("a", "1")];
        let diff = diff_keyed(&old, &new).expect("差分必须成功");

        assert_eq!(diff.mode(), ListDiffMode::Keyed);
        assert_eq!(
            diff.matched(),
            &[Some(1), Some(0)],
            "新位置 0 复用旧位置 1（按 key 配对）"
        );
        assert!(diff.removed().is_empty(), "没有子节点被移除");
    }

    #[test]
    fn keyed_diff_keeps_the_minimum_number_of_nodes_in_place() {
        // 旧顺序 a b c d，新顺序 d a b c ⇒ 保留 a b c（长度 3 的 LIS）⇒ 只移动 1 个。
        let old = vec![
            keyed("a", "1"),
            keyed("b", "2"),
            keyed("c", "3"),
            keyed("d", "4"),
        ];
        let new = vec![
            keyed("d", "4"),
            keyed("a", "1"),
            keyed("b", "2"),
            keyed("c", "3"),
        ];
        let diff = diff_keyed(&old, &new).expect("差分必须成功");

        assert_eq!(diff.retained_count(), 3, "LIS 长度 = 3");
        assert_eq!(
            diff.move_count(),
            1,
            "移动次数 = 配对数 − LIS 长度（= 1，下界）"
        );
        assert_eq!(diff.moved(), &[(3, 0)], "只有 d 需要移动");
        assert_eq!(diff.retained(), &[0, 1, 2], "a b c 原地保留");
    }

    #[test]
    fn keyed_diff_move_count_is_the_lis_lower_bound() {
        // 完全逆序 ⇒ LIS 长度 1 ⇒ 移动次数 = n − 1。
        let old: Vec<VNode> = ["a", "b", "c", "d", "e"]
            .iter()
            .map(|k| keyed(k, k))
            .collect();
        let new: Vec<VNode> = ["e", "d", "c", "b", "a"]
            .iter()
            .map(|k| keyed(k, k))
            .collect();
        let diff = diff_keyed(&old, &new).expect("差分必须成功");

        assert_eq!(diff.retained_count(), 1);
        assert_eq!(diff.move_count(), 4, "移动次数 = 5 − 1（下界）");
        assert_eq!(
            diff.matched_count() - diff.retained_count(),
            diff.move_count(),
            "不变量：配对数 − LIS 长度 = 移动次数"
        );
    }

    #[test]
    fn keyed_diff_reports_new_and_removed_children() {
        let old = vec![keyed("a", "1"), keyed("b", "2")];
        let new = vec![keyed("b", "2"), keyed("c", "3")];
        let diff = diff_keyed(&old, &new).expect("差分必须成功");

        assert_eq!(diff.removed(), &[0], "a 被移除");
        assert_eq!(diff.inserted().len(), 1, "c 是新建的");
        assert_eq!(diff.inserted()[0].0, 1, "c 落在新位置 1");
        assert_eq!(diff.matched(), &[Some(1), None]);
    }

    #[test]
    fn keyed_diff_flags_children_whose_content_changed() {
        let old = vec![keyed("a", "旧"), keyed("b", "2")];
        let new = vec![keyed("a", "新"), keyed("b", "2")];
        let diff = diff_keyed(&old, &new).expect("差分必须成功");

        assert_eq!(diff.updated(), &[(0, 0)], "只有 a 的内容变了");
    }

    #[test]
    fn keyed_diff_rejects_duplicate_keys_in_one_list() {
        let old = vec![keyed("a", "1"), keyed("a", "2")];
        let new = vec![keyed("a", "1")];
        let err = diff_keyed(&old, &new).expect_err("重复 key 必须显式报错");
        match err {
            KeyedDiffError::DuplicateKey { key, first, second } => {
                assert_eq!(key, Key::Str(String::from("a")));
                assert_eq!(first, 0);
                assert_eq!(second, 1);
            }
        }
    }

    /// **P2 补齐**：新列表里的重复 key 同样必须报错（身份不唯一 ⇒ 不可判定）。
    #[test]
    fn keyed_diff_rejects_duplicate_keys_in_the_new_list() {
        let old = vec![keyed("a", "1")];
        let new = vec![keyed("x", "1"), keyed("x", "2")];
        let err = diff_keyed(&old, &new).expect_err("新列表重复 key 必须显式报错");
        match err {
            KeyedDiffError::DuplicateKey { key, first, second } => {
                assert_eq!(key, Key::Str(String::from("x")));
                assert_eq!(first, 0);
                assert_eq!(second, 1);
            }
        }
    }

    /// **P2 补齐的反例**：两个不同 key 的新列表不受影响（不误报）。
    #[test]
    fn keyed_diff_accepts_distinct_keys_in_the_new_list() {
        let old = vec![keyed("a", "1")];
        let new = vec![keyed("x", "1"), keyed("y", "2")];
        let diff = diff_keyed(&old, &new).expect("不同 key 必须成功");
        assert_eq!(diff.matched(), &[None, None]);
        assert_eq!(diff.inserted().len(), 2);
    }

    #[test]
    fn keyed_diff_slots_describe_the_new_order() {
        let old = vec![keyed("a", "1"), keyed("b", "2")];
        let new = vec![keyed("b", "2"), keyed("c", "3")];
        let diff = diff_keyed(&old, &new).expect("差分必须成功");

        let slots = diff.slots();
        assert_eq!(slots[0], KeyedSlot::Reuse(1), "b 复用旧位置 1");
        assert!(
            matches!(slots[1], KeyedSlot::New(_)),
            "c 是新建的：{slots:?}"
        );
    }

    // ── unkeyed 差分 ──────────────────────────────────────────────────────

    #[test]
    fn unkeyed_diff_aligns_by_index() {
        let old = vec![plain("1"), plain("2")];
        let new = vec![plain("x"), plain("y")];
        let diff = diff_keyed(&old, &new).expect("差分必须成功");

        assert_eq!(diff.mode(), ListDiffMode::Unkeyed);
        assert_eq!(diff.matched(), &[Some(0), Some(1)], "按位置复用");
        assert_eq!(diff.move_count(), 0, "unkeyed 不移动");
        assert_eq!(diff.updated(), &[(0, 0), (1, 1)], "位置相同但内容都变了");
    }

    #[test]
    fn unkeyed_diff_inserts_and_removes_by_position() {
        let old = vec![plain("1"), plain("2")];
        let new = vec![plain("1"), plain("2"), plain("3")];
        let diff = diff_keyed(&old, &new).expect("差分必须成功");

        assert_eq!(diff.inserted().len(), 1);
        assert_eq!(diff.inserted()[0].0, 2);
        assert!(diff.removed().is_empty());
    }

    #[test]
    fn keyed_and_unkeyed_give_different_identities_for_the_same_reorder() {
        let old = vec![keyed("a", "1"), keyed("b", "2")];
        let new = vec![keyed("b", "2"), keyed("a", "1")];

        let keyed_diff = diff_keyed(&old, &new).expect("差分必须成功");
        let unkeyed_old = vec![plain("1"), plain("2")];
        let unkeyed_new = vec![plain("2"), plain("1")];
        let unkeyed_diff = diff_keyed(&unkeyed_old, &unkeyed_new).expect("差分必须成功");

        assert_eq!(
            keyed_diff.matched(),
            &[Some(1), Some(0)],
            "keyed 按身份复用"
        );
        assert_eq!(
            unkeyed_diff.matched(),
            &[Some(0), Some(1)],
            "unkeyed 按位置复用"
        );
    }

    // ── 「是否移动」的判定：**O(1)**（2026-10-25 修） ───────────────────────

    /// ⭐ **`is_retained` 与 `retained()` 逐项一致**（O(1) 判定的正确性）。
    ///
    /// ## 为什么要有这条
    ///
    /// 渲染器逐个判定「这个配对要不要移动」⇒ 若判定是 O(N)（`Vec` 线性查）
    /// ⇒ 整个列表差分是 **O(N²)** ⇒ ⇒ 长列表每次重渲染都慢。
    /// ⇒ ⇒ 改成集合查表（O(1)）⇒ ⇒ 语义必须**逐项相同**（不许漂移）。
    #[test]
    fn the_retained_membership_matches_the_retained_list() {
        let old: Vec<VNode> = ["a", "b", "c", "d", "e"]
            .iter()
            .map(|key| keyed(key, key))
            .collect();
        let new: Vec<VNode> = ["e", "c", "a", "d", "b"]
            .iter()
            .map(|key| keyed(key, key))
            .collect();
        let diff = diff_keyed(&old, &new).expect("差分必须成功");

        for index in 0..old.len() {
            assert_eq!(
                diff.is_retained(index),
                diff.retained().contains(&index),
                "旧下标 {index} 的「原地保留」判定必须一致"
            );
            assert_eq!(diff.is_moved(index), !diff.is_retained(index));
        }
        assert_eq!(diff.retained_count(), diff.retained().len());
        assert_eq!(
            diff.move_count(),
            diff.matched_count() - diff.retained_count()
        );
    }
}
