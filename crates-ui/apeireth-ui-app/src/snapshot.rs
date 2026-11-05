//! ⭐⭐⭐ **视图快照**（application-layer 地基，E-178）。
//!
//! ## 要解决的问题
//!
//! [`crate::arbitration::Arbiter`] 解决的是「**回调**回来时判新旧」
//! ⇒ ⇒ 而本模块解决的是「**已经算好的视图**要不要用」。
//!
//! ⇒ ⇒ 场景：异步结果回来 ⇒ ⇒ **在回调里同步算出视图树**
//! ⇒ ⇒⇒ 而「算视图」与「用视图」之间**可能又发生了一次变更**
//! ⇒ ⇒⇒ ⇒ 于是「算的时候是新的、用的时候已经旧了」。
//!
//! ## ⭐ 口径：**快照 = 树 + 票据**
//!
//! - [`Snapshot`] 把「这棵树是在哪个代号下算出来的」**一起带走**；
//! - 宿主**只在快照仍然有效时**才把它渲染出去
//!   ⇒ ⇒ ⇒ **过期快照直接丢弃**（不渲染、不覆盖）。
//!
//! ## ⭐ 为什么需要「快照」这一层（而不是直接用仲裁器）
//!
//! ⇒ ⇒ 仲裁器判的是「**这个结果**还有效吗」
//! ⇒ ⇒ 而「算出树」与「渲染树」之间**还有一段路**
//! ⇒ ⇒⇒ ⇒ 若这段路里状态变了 ⇒ ⇒⇒ 那棵树**已经不代表当下**了
//! ⇒ ⇒⇒ ⇒ 所以要把「它算出来时的身份」**随树一起带过去**。
//!
//! ## ⭐ 与宿主渲染循环的关系
//!
//! 宿主每帧：①取**有效**快照 ⇒ ⇒ ②渲染 ⇒ ⇒ ③清掉已消费的快照。
//! ⇒ ⇒ 若本帧**没有**有效快照 ⇒ ⇒ ⇒ **不渲染**（保留上一帧的 DOM）
//! ⇒ ⇒⇒ ⇒ 这就是「旧回调不能覆盖新工作」在**渲染层**的落点。

use apeireth_ui_core::arbitration::{Arbiter, Ticket};
use apeireth_ui_dom::vnode::VNode;

/// ⭐⭐⭐ **视图快照**：一棵树 + 它算出来时的**票据**。
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// 视图树。
    pub tree: VNode,
    /// 这棵树是在哪个**票据**下算出来的（⇒ ⇒ 用来判是否过期）。
    pub ticket: Ticket,
}

impl Snapshot {
    /// ⭐ 建一张快照。
    pub fn new(tree: VNode, ticket: Ticket) -> Self {
        Self { tree, ticket }
    }

    /// ⭐ 票据的代号（供日志与断言）。
    pub fn generation(&self) -> u64 {
        self.ticket.generation()
    }
}

/// ⭐⭐⭐ **快照仓**：按**通道**持有「当前待渲染的快照」。
///
/// ## 口径
///
/// - [`Self::publish`]：**新快照取代旧快照** ⇒ ⇒ ⇒ 旧的**直接被顶掉**
///   （**不**保留「旧的可能还有用」）⇒ ⇒ ⇒ 这是「只认最新」的语义；
/// - [`Self::take_current`]：取出**仍然有效**的快照
///   ⇒ ⇒ ⇒ 若已过期 ⇒ ⇒ ⇒ 返回 `None`（**不渲染**）⇒ ⇒ ⇒ **不覆盖新工作**。
#[derive(Debug, Default)]
pub struct SnapshotStore {
    arbiter: Arbiter,
    pending: std::cell::RefCell<std::collections::HashMap<String, Snapshot>>,
}

impl SnapshotStore {
    /// ⭐ 建一个空的快照仓。
    pub fn new() -> Self {
        Self::default()
    }

    /// ⭐⭐⭐ **发起一次新工作** ⇒ ⇒ 领一张票据（并**作废**该通道的旧快照）。
    ///
    /// ## 为什么这里要顺带作废旧快照
    ///
    /// ⇒ ⇒ 新工作一开始 ⇒ ⇒⇒ 旧工作算出的树**已经不代表当下**
    /// ⇒ ⇒⇒ ⇒ 所以旧快照**立即**作废 ⇒ ⇒⇒ 不必等它被取出时才发现过期。
    pub fn begin(&self, channel: &str) -> Ticket {
        let ticket = self.arbiter.begin(channel);
        self.pending.borrow_mut().remove(channel);
        ticket
    }

    /// ⭐⭐⭐ **发布一张快照**（新工作算出了视图）。
    ///
    /// ## 口径
    ///
    /// - 票据**仍有效** ⇒ ⇒ 收下（并**顶掉**该通道的旧快照）；
    /// - 票据**已过期** ⇒ ⇒ **拒收**（返回 `false`）⇒ ⇒ ⇒
    ///   ⇒ ⇒⇒ **旧回调的结果不会覆盖新工作** ✓
    pub fn publish(&self, channel: &str, snapshot: Snapshot) -> bool {
        if !self.arbiter.accept(channel, snapshot.ticket.clone()) {
            return false;
        }
        self.pending
            .borrow_mut()
            .insert(channel.to_owned(), snapshot);
        true
    }

    /// ⭐⭐⭐ **取出该通道**当前**有效**的快照（并**从仓里移除**）。
    ///
    /// ## 返回
    ///
    /// - `Some(snapshot)` ⇒ ⇒ 有效 ⇒ ⇒ 可以渲染；
    /// - `None` ⇒ ⇒ **没有**快照，或快照**已过期** ⇒ ⇒ ⇒ **不要渲染**。
    pub fn take_current(&self, channel: &str) -> Option<Snapshot> {
        let snapshot = self.pending.borrow_mut().remove(channel)?;
        if self.arbiter.accept(channel, snapshot.ticket.clone()) {
            Some(snapshot)
        } else {
            // ⭐ 已过期 ⇒ ⇒ ⇒ **丢弃**（不返回）⇒ ⇒ ⇒ 不覆盖新工作。
            None
        }
    }

    /// ⭐ **该通道是否有待渲染的快照**（测试与诊断用）。
    pub fn has_pending(&self, channel: &str) -> bool {
        self.pending.borrow().contains_key(channel)
    }

    /// ⭐ **作废某通道**（如：离开屏幕 ⇒ ⇒ 该屏幕的快照全部作废）。
    pub fn invalidate(&self, channel: &str) {
        self.arbiter.invalidate(channel);
        self.pending.borrow_mut().remove(channel);
    }

    /// ⭐ **仲裁器**（只读借用）⇒ ⇒ 供宿主做**额外**判据。
    pub fn arbiter(&self) -> &Arbiter {
        &self.arbiter
    }
}

/// ⭐⭐⭐⭐ **渲染一张快照**（宿主侧的落地口）—— **过期快照一律不渲染**。
///
/// ## 为什么要有这个函数
///
/// ⇒ ⇒ [`SnapshotStore::take_current`] 已经能「不给出过期快照」
/// ⇒ ⇒ 但**宿主**还得自己判断「有没有可渲染的东西」
/// ⇒ ⇒⇒ ⇒ 这里把「取快照 + 判过期 + 渲染」**收成一步**
/// ⇒ ⇒⇒ ⇒ 免得某处宿主写成「先 take 再无条件渲染」。
///
/// ## 返回
///
/// - `Ok(Some(mounted))` ⇒ ⇒ **渲染了**（返回挂载点）；
/// - `Ok(None)` ⇒ ⇒ **没有可渲染的快照** ⇒ ⇒ ⇒ **保留上一帧 DOM**（不渲染）；
/// - `Err(error)` ⇒ ⇒ 渲染**失败**（错误边界按宿主配置处理）。
pub fn render_snapshot<D: apeireth_ui_dom::dom::Dom>(
    renderer: &mut apeireth_ui_dom::renderer::Renderer<D>,
    store: &SnapshotStore,
    channel: &str,
) -> Result<Option<apeireth_ui_dom::renderer::MountedNode<D>>, apeireth_ui_dom::dom::DomError> {
    // ⭐⭐⭐ **过期快照在这里被挡住** ⇒ ⇒ ⇒ 旧结果**进不了 DOM**。
    let Some(snapshot) = store.take_current(channel) else {
        return Ok(None);
    };
    let mounted = renderer.render_guarded(snapshot.tree)?;
    Ok(Some(mounted))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(text: &str) -> VNode {
        VNode::element("div").child(VNode::text(text)).build()
    }

    /// ⭐⭐⭐ **过期快照不渲染**（本模块存在的**唯一理由**）。
    #[test]
    fn a_stale_snapshot_is_not_taken() {
        let store = SnapshotStore::new();
        let first = store.begin("search");
        let second = store.begin("search");
        // ⭐ 第一份快照**过期** ⇒ ⇒ 拒收。
        assert!(
            !store.publish("search", Snapshot::new(tree("旧"), first)),
            "⭐ 过期快照必须被拒收（旧回调不能覆盖新工作）"
        );
        // ⭐ 第二份有效 ⇒ ⇒ 收下。
        assert!(store.publish("search", Snapshot::new(tree("新"), second.clone())));
        let taken = store.take_current("search").expect("必须有有效快照");
        assert_eq!(taken.generation(), second.generation());
    }

    /// ⭐⭐⭐ **新快照顶掉旧快照**（「只认最新」）。
    #[test]
    fn publishing_a_newer_snapshot_replaces_the_older_one() {
        let store = SnapshotStore::new();
        let first = store.begin("search");
        store.publish("search", Snapshot::new(tree("旧"), first.clone()));
        let second = store.begin("search");
        store.publish("search", Snapshot::new(tree("新"), second.clone()));
        let taken = store.take_current("search").expect("必须有快照");
        assert_eq!(
            taken.generation(),
            second.generation(),
            "⭐ 取出的必须是**新**快照（旧的被顶掉）"
        );
    }

    /// ⭐⭐⭐ **发起新工作 ⇒ ⇒ 旧快照立即作废**（不必等取出时才发现）。
    #[test]
    fn starting_new_work_invalidates_the_pending_snapshot() {
        let store = SnapshotStore::new();
        let ticket = store.begin("search");
        store.publish("search", Snapshot::new(tree("旧"), ticket));
        assert!(store.has_pending("search"), "⭐ 先确认快照在仓里");
        // ⭐ 新工作一开始 ⇒ ⇒ 旧快照**立即**作废。
        let _next = store.begin("search");
        assert!(
            !store.has_pending("search"),
            "⭐ 新工作一开始 ⇒ ⇒ 旧快照必须立即作废"
        );
    }

    /// ⭐⭐⭐ **通道隔离**：一个通道的快照**不该**影响另一个。
    #[test]
    fn channels_do_not_interfere() {
        let store = SnapshotStore::new();
        let city = store.begin("city");
        let search = store.begin("search");
        store.publish("city", Snapshot::new(tree("城"), city.clone()));
        store.publish("search", Snapshot::new(tree("搜"), search));
        // ⭐ 城市的新工作**不该**让搜索的快照失效。
        let _city_new = store.begin("city");
        assert!(
            store.take_current("search").is_some(),
            "⭐ 通道之间不得互相作废"
        );
        // ⭐ 但城市自己的旧快照**失效**了。
        assert!(store.take_current("city").is_none());
    }

    /// ⭐⭐⭐ **作废通道** ⇒ ⇒ 该通道的快照全部作废。
    #[test]
    fn invalidating_a_channel_drops_its_snapshot() {
        let store = SnapshotStore::new();
        let ticket = store.begin("city");
        store.publish("city", Snapshot::new(tree("城"), ticket));
        store.invalidate("city");
        assert!(
            store.take_current("city").is_none(),
            "⭐ 作废后不得再取到该通道的快照"
        );
    }

    /// ⭐⭐⭐ **取出即消费**（同一份快照不会被渲染两次）。
    #[test]
    fn taking_a_snapshot_consumes_it() {
        let store = SnapshotStore::new();
        let ticket = store.begin("x");
        store.publish("x", Snapshot::new(tree("t"), ticket));
        assert!(store.take_current("x").is_some());
        assert!(
            store.take_current("x").is_none(),
            "⭐ 快照取出即消费（不会渲染两次）"
        );
    }

    /// ⭐⭐⭐⭐ **端到端：真实异步竞态** ⇒ ⇒ **界面不会被旧结果改回去**。
    ///
    /// ## 它模拟什么
    ///
    /// ```text
    /// t0: 搜「猫」⇒ ⇒ 领票据 A ⇒ ⇒ 发请求
    /// t1: 搜「狗」⇒ ⇒ 领票据 B ⇒ ⇒ 发请求
    /// t2: B 的结果先回来 ⇒ ⇒ 算出「狗」的树 ⇒ ⇒ 发布
    /// t3: A 的结果后回来 ⇒ ⇒ 算出「猫」的树 ⇒ ⇒ ⇒ **必须被丢弃**
    /// ```
    ///
    /// ⇒ ⇒ 而 t3 那一刻，**屏幕上必须仍然是「狗」**。
    #[test]
    fn an_older_result_does_not_overwrite_the_newer_one_on_screen() {
        let store = SnapshotStore::new();
        // ① t0：搜「猫」
        let ticket_cat = store.begin("search");
        // ② t1：搜「狗」
        let ticket_dog = store.begin("search");

        // ③ t2：「狗」的结果先回来 ⇒ ⇒ 算出树 ⇒ ⇒ 发布 ⇒ ⇒ 渲染。
        assert!(store.publish("search", Snapshot::new(tree("狗"), ticket_dog.clone())));
        let rendered = store.take_current("search").expect("⭐ 「狗」必须能渲染");
        assert_eq!(rendered.generation(), ticket_dog.generation());

        // ④ t3：「猫」的结果后回来 ⇒ ⇒ **必须被拒收**。
        assert!(
            !store.publish("search", Snapshot::new(tree("猫"), ticket_cat)),
            "⭐ 旧结果必须被拒收（旧回调不能覆盖新工作）"
        );
        // ⑤ 屏幕上的内容**仍是「狗」**（没有被改回「猫」）。
        assert!(
            store.take_current("search").is_none(),
            "⭐ 旧结果不得进入渲染队列 ⇒ ⇒ 界面保持「狗」"
        );
    }

    /// ⭐⭐⭐⭐ **过期快照进不了 DOM**（端到端闭环的最后一段）。
    ///
    /// ## 它锁住什么
    ///
    /// ⇒ ⇒ 前面的测试只验到「仓里取不到过期快照」
    /// ⇒ ⇒⇒ 而这一条验的是**渲染层**：
    /// ⇒ ⇒⇒ 过期结果**不得**出现在 DOM 里 ⇒ ⇒⇒ 即「旧回调不能覆盖新工作」
    /// ⇒ ⇒⇒ **在真实渲染路径上**成立。
    #[test]
    fn a_stale_snapshot_never_reaches_the_dom() {
        use apeireth_ui_dom::dom::{Dom, InMemoryDom};
        use apeireth_ui_dom::renderer::Renderer;

        let dom = InMemoryDom::new();
        let root = dom.create_element("div").expect("建根必须成功");
        let mut renderer = Renderer::new(dom, root);
        let store = SnapshotStore::new();

        // ① t0：旧工作
        let stale = store.begin("search");
        // ② t1：新工作（⇒ ⇒ 旧票据立刻过期）
        let fresh = store.begin("search");
        // ③ 「新」的结果先到 ⇒ ⇒ 渲染。
        store.publish("search", Snapshot::new(tree("新"), fresh));
        let mounted = render_snapshot(&mut renderer, &store, "search")
            .expect("渲染必须成功")
            .expect("⭐ 必须渲染了新快照");
        let container = renderer.node_for_path(&[]).expect("容器必须可定位");
        let text = renderer.dom().text_content(container);
        assert!(text.contains("新"), "⭐ DOM 里必须是「新」：{text}");
        let _ = mounted;

        // ④ 「旧」的结果后到 ⇒ ⇒ **必须进不了 DOM**。
        assert!(
            !store.publish("search", Snapshot::new(tree("旧"), stale)),
            "⭐ 旧快照必须被拒收"
        );
        let rendered = render_snapshot(&mut renderer, &store, "search").expect("渲染必须成功");
        assert!(
            rendered.is_none(),
            "⭐ 过期快照**不得**触发渲染（DOM 必须保持不变）"
        );
        let text_after = renderer.dom().text_content(container);
        assert!(
            !text_after.contains("旧"),
            "⭐ DOM 里**不得**出现「旧」：{text_after}"
        );
    }

    /// ⭐⭐⭐⭐ **发布时有效、取时已过期 ⇒ ⇒ 取出时也要拒**（**第二道防线**）。
    ///
    /// ## ⭐ 这条路径为什么**可达**（实测得出，不是猜）
    ///
    /// ⇒ ⇒ `SnapshotStore::begin` / `SnapshotStore::invalidate` **都会清空** pending
    /// ⇒ ⇒⇒ 所以「发布 → 取」之间若只走这两个入口 ⇒ ⇒⇒ 快照**要么被清、要么仍有效**
    /// ⇒ ⇒⇒ ⇒ 「发布时有效、取时已过期」**看起来**不可达
    /// ⇒ ⇒⇒ **但**：仲裁器是**公开**的（[`SnapshotStore::arbiter`]）
    /// ⇒ ⇒⇒ 经 `arbiter().invalidate(channel)` 作废 ⇒ ⇒⇒ **不清** pending
    /// ⇒ ⇒⇒ ⇒ 于是快照**留在仓里**、而票据**已过期** ⇒ ⇒⇒ **正是这条路径**。
    ///
    /// ## 为什么保留这道防线
    ///
    /// ⇒ ⇒ 「发布」与「取」是**两个独立时刻** ⇒ ⇒⇒ 其间状态**可以**变
    /// ⇒ ⇒⇒ ⇒ 只在发布时判一次 ⇒ ⇒⇒ **不够** ⇒ ⇒⇒ 两处都判 ⇒ ⇒⇒ 才闭合。
    #[test]
    fn a_snapshot_that_expires_between_publish_and_take_is_rejected() {
        let store = SnapshotStore::new();
        let ticket = store.begin("search");
        store.publish("search", Snapshot::new(tree("t"), ticket));
        assert!(store.has_pending("search"), "⭐ 先确认快照在仓里");
        // ⭐ 经**公开的仲裁器**作废该通道 ⇒ ⇒⇒ pending **不清**、但票据**已过期**。
        store.arbiter().invalidate("search");
        assert!(
            store.has_pending("search"),
            "⭐ 经仲裁器作废**不清** pending（这正是那条可达路径）"
        );
        assert!(
            store.take_current("search").is_none(),
            "⭐ 取时必须**再判一次**过期（第二道防线）"
        );
    }

    /// ⭐⭐⭐ **没有快照时不渲染**（保留上一帧 DOM）。
    #[test]
    fn there_is_no_snapshot_when_nothing_was_published() {
        let store = SnapshotStore::new();
        assert!(store.take_current("x").is_none());
    }
}
