//! ⭐⭐⭐ **异步数据线**（请求仲裁 + 视图快照的**参考接线**，E-182）。
//!
//! ## 它是什么
//!
//! 一条**通道**上的「发请求 ⇒ ⇒ 等结果 ⇒ ⇒ 落地」的标准接线，**三步**：
//!
//! 1. [`AsyncLine::begin`]：领一张**票据**（同时作废该通道的旧快照）；
//! 2. （发请求 / 等结果 / 算树）—— **这一步是宿主的自由**（网络、worker、文件…）；
//! 3. [`AsyncLine::publish`]：把算出的树**发布**回去（**过期一律拒收**）。
//!
//! ⇒ ⇒⇒ 宿主拿到的是 [`RenderPlan::Snapshot`](crate::RenderPlan::Snapshot)
//! ⇒ ⇒⇒ ⇒ 由 [`HostState::render_plan`](crate::HostState::render_plan) 在帧边界取出。
//!
//! ## ⭐⭐⭐ 为什么要有这一层（而不是让每个宿主自己拼）
//!
//! ⇒ ⇒ 「旧回调不能覆盖新工作」（r10 §5.3）这条语义**极易漏**：
//! ⇒ ⇒⇒ 漏了**票据检查** ⇒ ⇒⇒ 旧结果会**直接改状态**或**直接渲染**
//! ⇒ ⇒⇒⇒ 现象是「界面**自己跳回去了**」⇒ ⇒⇒⇒ 而它**只在网络慢时出现**
//! ⇒ ⇒⇒⇒ ⇒ 复现要掐时机 ⇒ ⇒⇒⇒ ⇒ **极难查**。
//!
//! ⇒ ⇒⇒ 所以这里把「领票 / 发布 / 作废」**收成一处** ⇒ ⇒⇒
//! ⇒ ⇒⇒ 宿主只管**算树**，**不用**碰票据 ⇒ ⇒⇒ ⇒ 漏接的可能**最小**。
//!
//! ## ⭐ 为什么它**不是** wasm 专属
//!
//! ⇒ ⇒ 「算树」那一步用什么（网络 / worker / 定时器）**是宿主的事**
//! ⇒ ⇒⇒ 而「领票 / 发布」这一步**到处都一样** ⇒ ⇒⇒⇒ ⇒ 所以这一层放在 **native 可见**的地方
//! ⇒ ⇒⇒⇒ ⇒ 于是它**能被 native 单测钉住**，也能被**真实浏览器探针**复用。

use apeireth_ui_core::arbitration::Ticket;
use apeireth_ui_dom::vnode::VNode;

use crate::snapshot::{Snapshot, SnapshotStore};

/// ⭐⭐⭐ **一条异步数据线**（通道 + 票据 + 发布）。
///
/// ## ⭐ 它与 [`HostState`] 的关系
///
/// ⇒ ⇒ 它持有的是**同一个** [`SnapshotStore`]（克隆 ⇒ ⇒ **共享**）
/// ⇒ ⇒⇒ ⇒ 屏幕用它**发**快照、宿主用它**取**快照 ⇒ ⇒⇒ ⇒ 机制才成立
/// ⇒ ⇒⇒ ⇒ ⇒ **拿法**：`host_state.snapshots().clone()`。
#[derive(Debug, Clone)]
pub struct AsyncLine {
    /// ⭐⭐⭐ **共享**的快照仓（宿主与屏幕**同一份**）。
    store: SnapshotStore,
    /// ⭐ 通道名（**按通道隔离** ⇒ ⇒ 不同数据线**互不干扰**）。
    channel: String,
}

impl AsyncLine {
    /// ⭐ 建一条数据线（`store` 必须与宿主**共享** ⇒ ⇒ 用 `host_state.snapshots().clone()`）。
    pub fn new(store: SnapshotStore, channel: &str) -> Self {
        Self {
            store,
            channel: channel.to_owned(),
        }
    }

    /// ⭐ 通道名。
    pub fn channel(&self) -> &str {
        &self.channel
    }

    /// ⭐⭐⭐ **领一张票据**（= 发起了新工作）。
    ///
    /// ⇒ ⇒ **旧快照立即作废** ⇒ ⇒⇒ 新工作一开始 ⇒ ⇒ 旧工作算出的树**已不代表当下**。
    pub fn begin(&self) -> Ticket {
        self.store.begin(&self.channel)
    }

    /// ⭐⭐⭐ **发布算出的树**（= 结果回来了）。
    ///
    /// ## 返回
    ///
    /// - `true` ⇒ ⇒ **收下了**（这张快照有效 ⇒ ⇒ 下一帧会被渲染）；
    /// - `false` ⇒ ⇒ **拒收**（**过期** ⇒ ⇒⇒ ⇒ ⇒ **旧回调不会覆盖新工作** ✓）。
    pub fn publish(&self, tree: VNode, ticket: Ticket) -> bool {
        self.store
            .publish(&self.channel, Snapshot::new(tree, ticket))
    }

    /// ⭐⭐⭐ **作废这条线**（如：离开屏幕 ⇒ ⇒ 该线的快照全部作废）。
    pub fn invalidate(&self) {
        self.store.invalidate(&self.channel);
    }

    /// ⭐ 底层快照仓（诊断 / 高级用法）。
    pub fn store(&self) -> &SnapshotStore {
        &self.store
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_ui_core::arbitration::Arbiter;

    fn tree(text: &str) -> VNode {
        VNode::element("div").child(VNode::text(text)).build()
    }

    /// ⭐ 收集树里的文字（断言用）。
    fn text_of(node: &VNode) -> String {
        match node {
            VNode::Text(text) => text.data.clone(),
            _ => node.children().iter().map(text_of).collect(),
        }
    }

    /// ⭐⭐⭐ **过期结果一律拒收**（这条语义就是本层的**全部**价值）。
    #[test]
    fn a_stale_result_is_rejected() {
        let line = AsyncLine::new(SnapshotStore::new(), "search");
        let first = line.begin();
        let second = line.begin();
        // ⭐ 旧结果后到 ⇒ ⇒ 必须**拒收**。
        assert!(
            !line.publish(tree("旧"), first),
            "⭐ 过期结果必须被拒收（旧回调不能覆盖新工作）"
        );
        // ⭐ 新结果 ⇒ ⇒ 收下。
        assert!(line.publish(tree("新"), second), "⭐ 当前结果必须被收下");
    }

    /// ⭐⭐⭐ **发布后宿主能取到**（与 [`crate::HostState`] 接通）。
    #[test]
    fn a_published_snapshot_reaches_the_host() {
        let mut host = crate::HostState::new();
        let line = AsyncLine::new(host.snapshots().clone(), "search");
        let ticket = line.begin();
        assert!(line.publish(tree("结果"), ticket));
        // ⭐ 宿主在帧边界取出 ⇒ ⇒ 渲染**快照**。
        let plan = host.render_plan(Some("search"));
        match plan {
            crate::RenderPlan::Snapshot(tree) => {
                assert!(
                    text_of(&tree).contains("结果"),
                    "⭐ 宿主必须渲染**快照**（异步结果比本地视图更新鲜）"
                );
            }
            other => panic!("⭐ 应当选快照，实际是 {other:?}"),
        }
    }

    /// ⭐ **作废** ⇒ ⇒ 待渲染的快照被丢弃（离开屏幕时用）。
    #[test]
    fn invalidating_drops_the_pending_snapshot() {
        let mut host = crate::HostState::new();
        let line = AsyncLine::new(host.snapshots().clone(), "search");
        let ticket = line.begin();
        assert!(line.publish(tree("结果"), ticket));
        line.invalidate();
        assert_eq!(
            host.render_plan(Some("search")),
            crate::RenderPlan::Idle,
            "⭐ 作废后不得再渲染旧快照"
        );
    }

    /// ⭐⭐ **通道隔离**（两条线互不干扰）。
    ///
    /// ⭐ 口径提醒：两条线**必须**用**宿主那份**共享快照仓
    /// ⇒ ⇒⇒ 否则宿主的 `render_plan` 看的是**另一份**仓 ⇒ ⇒⇒⇒ 取不到任何东西
    /// ⇒ ⇒⇒⇒ （**这条口径本身**就是「共享语义」的考题）。
    #[test]
    fn two_lines_do_not_interfere() {
        let mut host = crate::HostState::new();
        let store = host.snapshots().clone();
        let search = AsyncLine::new(store.clone(), "search");
        let cities = AsyncLine::new(store, "cities");
        let search_ticket = search.begin();
        let cities_ticket = cities.begin();
        // ⭐ 城市的票据**不影响**搜索线（按通道隔离）。
        assert!(search.publish(tree("搜索结果"), search_ticket));
        assert!(cities.publish(tree("城市结果"), cities_ticket));
        // ⭐ 宿主按通道**各取各的** ⇒ ⇒ 两份都还在（互不顶掉）。
        assert!(matches!(
            host.render_plan(Some("search")),
            crate::RenderPlan::Snapshot(_)
        ));
        assert!(matches!(
            host.render_plan(Some("cities")),
            crate::RenderPlan::Snapshot(_)
        ));
    }

    /// ⭐ 票据**自带**通道与代号（诊断用）。
    #[test]
    fn a_ticket_carries_its_channel() {
        let line = AsyncLine::new(SnapshotStore::new(), "search");
        let ticket = line.begin();
        assert_eq!(ticket.channel(), "search");
        // ⭐ 仲裁器与数据线共享同一份代号 ⇒ ⇒ 号码单调递增。
        let arbiter = Arbiter::new();
        let _ = arbiter.begin("search");
        assert!(ticket.generation() >= 1);
    }
}
