//! 依赖图：双向边 + 节点标签。
//!
//! 设计文档 §4.4：图结构维护两个方向的边 ——
//! - `dependencies`：「我读了谁」（重新登记时用于清旧边）；
//! - `dependents`：「谁读了我」（写信号时用于失效传播）。
//!
//! 依赖边是**运行时**建立的（Rust 无编译期模板），每次节点重跑都会清空旧边
//! 再重新登记 ⇒ 条件依赖（依赖随分支变化）天然正确。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// 响应式节点标识（信号 / 派生 / effect 共用同一个编号空间）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub usize);

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// 依赖图：维护「我读了谁」与「谁读了我」两张表。
#[derive(Debug, Clone, Default)]
pub struct DependencyGraph {
    dependencies: BTreeMap<NodeId, BTreeSet<NodeId>>,
    dependents: BTreeMap<NodeId, BTreeSet<NodeId>>,
    labels: BTreeMap<NodeId, String>,
}

impl DependencyGraph {
    /// 建一张空图。
    pub fn new() -> Self {
        Self::default()
    }

    /// 登记依赖边：`dependent` 读了 `dependency`（重复登记幂等）。
    pub fn subscribe(&mut self, dependent: NodeId, dependency: NodeId) {
        if dependent == dependency {
            // 自依赖是自激循环的种子，直接不入图（由运行期上限兜底报错）。
            return;
        }
        self.dependencies
            .entry(dependent)
            .or_default()
            .insert(dependency);
        self.dependents
            .entry(dependency)
            .or_default()
            .insert(dependent);
    }

    /// 解除依赖边（幂等）。
    pub fn unsubscribe(&mut self, dependent: NodeId, dependency: NodeId) {
        if let Some(set) = self.dependencies.get_mut(&dependent) {
            set.remove(&dependency);
            if set.is_empty() {
                self.dependencies.remove(&dependent);
            }
        }
        if let Some(set) = self.dependents.get_mut(&dependency) {
            set.remove(&dependent);
            if set.is_empty() {
                self.dependents.remove(&dependency);
            }
        }
    }

    /// 清空某节点的全部出边（重跑前调用 ⇒ 旧依赖不再触发）。
    pub fn clear_dependencies(&mut self, node: NodeId) {
        let deps = self.dependencies.remove(&node).unwrap_or_default();
        for dep in deps {
            if let Some(set) = self.dependents.get_mut(&dep) {
                set.remove(&node);
                if set.is_empty() {
                    self.dependents.remove(&dep);
                }
            }
        }
    }

    /// 某节点读了哪些节点。
    pub fn dependencies_of(&self, node: NodeId) -> Vec<NodeId> {
        self.dependencies
            .get(&node)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }

    /// 哪些节点读了某节点。
    pub fn dependents_of(&self, node: NodeId) -> Vec<NodeId> {
        self.dependents
            .get(&node)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }

    /// 给节点挂一个人类可读标签（报错时用于定位）。
    pub fn set_label(&mut self, node: NodeId, label: impl Into<String>) {
        self.labels.insert(node, label.into());
    }

    /// 读节点标签。
    pub fn label(&self, node: NodeId) -> Option<&str> {
        self.labels.get(&node).map(String::as_str)
    }

    /// 摘掉节点的标签（节点销毁时调用 ⇒ 标签不得单调增长）。
    pub fn remove_label(&mut self, node: NodeId) {
        self.labels.remove(&node);
    }

    /// 当前标签条数（可观测：长跑进程里标签不得只增不减）。
    pub fn label_count(&self) -> usize {
        self.labels.len()
    }

    /// 把节点从图里**彻底摘掉**：双向边 + 标签一并清掉。
    ///
    /// 为什么需要：节点被销毁（effect `dispose` / 派生 `dispose`）后，若只清出边
    /// 不清标签，标签表就只增不减 ⇒ 长跑 SPA 的图结构随节点数线性增长。
    /// 边是**双向**的 ⇒ 一个节点可能既在别人的依赖集里、也在别人的被依赖集里，
    /// 两边都要摘（否则反向表会残留指向已销毁节点的空壳条目）。
    pub fn remove_node(&mut self, node: NodeId) {
        // 出边：本节点读了谁 ⇒ 谁的「被依赖集」里要去掉本节点。
        let deps = self.dependencies.remove(&node).unwrap_or_default();
        for dep in deps {
            if let Some(set) = self.dependents.get_mut(&dep) {
                set.remove(&node);
                if set.is_empty() {
                    self.dependents.remove(&dep);
                }
            }
        }
        // 入边：谁读了本节点 ⇒ 那些节点的「依赖集」里要去掉本节点。
        let dependents = self.dependents.remove(&node).unwrap_or_default();
        for dependent in dependents {
            if let Some(set) = self.dependencies.get_mut(&dependent) {
                set.remove(&node);
                if set.is_empty() {
                    self.dependencies.remove(&dependent);
                }
            }
        }
        self.labels.remove(&node);
    }

    /// 当前依赖边条数（两个方向的表各算一条存储 ⇒ 返回逻辑边数）。
    pub fn edge_count(&self) -> usize {
        self.dependencies.values().map(BTreeSet::len).sum()
    }

    /// 出现在图里的节点数（出现在任一张表里即计入）。
    pub fn node_count(&self) -> usize {
        let mut nodes: BTreeSet<NodeId> = BTreeSet::new();
        nodes.extend(self.dependencies.keys().copied());
        nodes.extend(self.dependents.keys().copied());
        nodes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscribe_is_idempotent_and_keeps_both_directions() {
        let mut graph = DependencyGraph::new();
        let reader = NodeId(1);
        let source = NodeId(2);

        graph.subscribe(reader, source);
        graph.subscribe(reader, source);

        assert_eq!(graph.dependencies_of(reader), vec![source]);
        assert_eq!(graph.dependents_of(source), vec![reader]);
        assert_eq!(graph.edge_count(), 1, "重复登记只算一条边");
        assert_eq!(graph.node_count(), 2);
    }

    #[test]
    fn clear_dependencies_removes_both_directions() {
        let mut graph = DependencyGraph::new();
        let reader = NodeId(1);
        let left = NodeId(2);
        let right = NodeId(3);
        graph.subscribe(reader, left);
        graph.subscribe(reader, right);

        graph.clear_dependencies(reader);
        assert!(graph.dependencies_of(reader).is_empty());
        assert!(graph.dependents_of(left).is_empty());
        assert!(graph.dependents_of(right).is_empty());
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn self_dependency_is_refused() {
        let mut graph = DependencyGraph::new();
        let node = NodeId(7);
        graph.subscribe(node, node);
        assert_eq!(graph.edge_count(), 0, "自依赖不入图（由运行期上限兜底）");
    }

    #[test]
    fn labels_are_readable() {
        let mut graph = DependencyGraph::new();
        let node = NodeId(9);
        graph.set_label(node, "视图::可见条目");
        assert_eq!(graph.label(node), Some("视图::可见条目"));
        assert_eq!(graph.label(NodeId(10)), None);
    }

    // ── 判别测试（钉行为不钉字面）──────────────────────────────────────────

    /// 解除订阅必须**两个方向**都清掉（否则失效传播会打到已解除的节点）。
    #[test]
    fn unsubscribe_clears_both_directions() {
        let mut graph = DependencyGraph::new();
        let reader = NodeId(1);
        let source = NodeId(2);
        graph.subscribe(reader, source);

        graph.unsubscribe(reader, source);
        assert!(graph.dependencies_of(reader).is_empty());
        assert!(graph.dependents_of(source).is_empty());
        assert_eq!(graph.edge_count(), 0);
    }

    /// 解除不存在的边是**幂等**空操作（不得误删别的边）。
    #[test]
    fn unsubscribing_an_absent_edge_is_a_noop() {
        let mut graph = DependencyGraph::new();
        let reader = NodeId(1);
        let kept = NodeId(2);
        let other = NodeId(3);
        graph.subscribe(reader, kept);

        graph.unsubscribe(reader, other);
        assert_eq!(
            graph.dependencies_of(reader),
            vec![kept],
            "解除无关边不得误删已有边"
        );
        assert_eq!(graph.edge_count(), 1);
    }

    /// 一个节点读多个来源时，边数按**来源**计（不是按读取次数）。
    #[test]
    fn edge_count_is_per_source_not_per_read() {
        let mut graph = DependencyGraph::new();
        let reader = NodeId(1);
        let left = NodeId(2);
        let right = NodeId(3);
        for _ in 0..5 {
            graph.subscribe(reader, left);
        }
        graph.subscribe(reader, right);

        assert_eq!(graph.edge_count(), 2, "重复登记只算一条边");
        assert_eq!(graph.dependencies_of(reader), vec![left, right]);
    }

    /// 清空出边后，反向表里不得残留**空集合**条目（避免节点数虚高）。
    #[test]
    fn clearing_dependencies_prunes_empty_reverse_entries() {
        let mut graph = DependencyGraph::new();
        let reader = NodeId(1);
        let source = NodeId(2);
        graph.subscribe(reader, source);
        assert_eq!(graph.node_count(), 2);

        graph.clear_dependencies(reader);
        assert_eq!(graph.node_count(), 0, "空集合条目必须被剪掉");
    }

    /// 摘掉节点 ⇒ **两个方向**的边与标签都必须消失（否则图与标签表只增不减）。
    #[test]
    fn removing_a_node_clears_both_directions_and_the_label() {
        let mut graph = DependencyGraph::new();
        let effect = NodeId(1);
        let signal = NodeId(2);
        let other = NodeId(3);

        graph.subscribe(effect, signal); // effect 读 signal
        graph.subscribe(other, effect); // other 读 effect
        graph.set_label(effect, "视图::待销毁");
        assert_eq!(graph.label_count(), 1);

        graph.remove_node(effect);

        assert!(graph.dependencies_of(effect).is_empty(), "出边必须清掉");
        assert!(graph.dependents_of(effect).is_empty(), "入边必须清掉");
        assert!(
            graph.dependents_of(signal).is_empty(),
            "反向表里不得残留已摘节点"
        );
        assert!(
            graph.dependencies_of(other).is_empty(),
            "别人依赖已摘节点的那条边也必须清掉"
        );
        assert_eq!(graph.label_count(), 0, "标签必须一并摘掉");
        assert_eq!(graph.edge_count(), 0);
    }
}
