//! ⭐⭐⭐ **请求仲裁**（application-layer 地基，E-178）。
//!
//! ## 要解决的问题（r10 §5.3 的硬要求）
//!
//! 「**旧回调不能覆盖新工作**」—— 这是任何「发请求 ⇒ ⇒ 等结果回来」的界面
//! 都要面对的问题：
//!
//! ```text
//! t0: 用户搜「猫」 ⇒ ⇒ 发请求 A
//! t1: 用户搜「狗」 ⇒ ⇒ 发请求 B
//! t2: B 的结果先回来 ⇒ ⇒ 界面显示「狗」
//! t3: A 的结果后回来 ⇒ ⇒ 若不仲裁 ⇒ ⇒ **界面被改回「猫」** ← 错
//! ```
//!
//! ⇒ ⇒ 表现是「界面**自己跳回去了**」⇒ ⇒⇒ 而这**极难查**
//! ⇒ ⇒⇒ （网络慢的时候才出现 ⇒ ⇒⇒ 复现要掐时机）。
//!
//! ## ⭐ 口径：**代号**（generation）+ **票据**（ticket）
//!
//! - **通道**（channel）：一条独立的仲裁线（如「城市视图」「搜索」「详情」）。
//!   ⇒ ⇒ 不同通道**互不干扰**（城市的旧结果**不该**让搜索失效）。
//! - **代号**：每个通道一个**单调递增**的整数 ⇒ ⇒ 每发起一次新工作 ⇒ ⇒ 代号 +1。
//! - **票据**（ticket）：发起时领到的**当前代号** ⇒ ⇒ 结果回来时比对
//!   ⇒ ⇒ **代号一致 ⇒ ⇒ 结果仍然有效**；**不一致 ⇒ ⇒ 结果已过期**。
//!
//! ## ⭐ 为什么用「代号」而不是「取消请求」
//!
//! ⇒ ⇒ 真实世界的请求**未必可取消**（已发出的 HTTP、已启动的 worker、已触发的回调）
//! ⇒ ⇒⇒ 就算**能**取消，也有「取消与结果返回**竞态**」的经典问题
//! ⇒ ⇒⇒ ⇒ 所以不靠「取消」，而靠「**回来时判新旧**」⇒ ⇒⇒ **无需取消也能正确**。
//!
//! ## ⭐ 与 r10 原型的口径一致
//!
//! r10 的 `app.js` 用两个机制做同一件事：
//! - **`generation` 计数器** ⇒ ⇒ 回调用 `generation === cityGeneration` 比对；
//! - **请求身份**（`cityRequest !== request`）⇒ ⇒ 只有最新请求的对象算数。
//!
//! ⇒ ⇒ 本模块是这两条的**引擎侧等价物**，且**加了一条 r10 没有的**：**按通道隔离**
//! ⇒ ⇒ ⇒ r10 的 `cityGeneration` 是**全局**一个 ⇒ ⇒ ⇒ 多条数据线并发时会互相踩
//! ⇒ ⇒⇒ ⇒ 而 r10 目前**只有一条**数据线 ⇒ ⇒⇒ 所以那个缺陷**还没暴露**。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// ⭐⭐⭐ **仲裁器**：按**通道**维护单调递增的**代号**。
///
/// ## 典型用法
///
/// ```ignore
/// let arbiter = Arbiter::new();
/// // ① 发起工作 ⇒ ⇒ 领一张**票据**
/// let ticket = arbiter.begin("search");
/// // ② 结果回来 ⇒ ⇒ 用票据**判新旧**
/// if !arbiter.accept("search", ticket) {
///     return; // 过期 ⇒ ⇒ 丢弃（**不**覆盖新工作）
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct Arbiter {
    channels: Rc<RefCell<HashMap<String, u64>>>,
}

impl Arbiter {
    /// 建一个仲裁器。
    pub fn new() -> Self {
        Self::default()
    }

    /// ⭐ **发起一次新工作**：领一张**票据**（该通道的当前代号）。
    ///
    /// ## 口径
    ///
    /// - 首次使用该通道 ⇒ ⇒ 从 **1** 开始（`0` 留作「无票据」的哨兵）；
    /// - 之后每次 ⇒ ⇒ **+1** ⇒ ⇒ ⇒ 旧票据**必然**过期。
    pub fn begin(&self, channel: &str) -> Ticket {
        let mut channels = self.channels.borrow_mut();
        let counter = channels.entry(channel.to_owned()).or_insert(0);
        *counter += 1;
        Ticket {
            channel: channel.to_owned(),
            generation: *counter,
        }
    }

    /// ⭐⭐⭐ **结果回来时的判据**：这张票据**还有效**吗？
    ///
    /// ## 返回
    ///
    /// - `true` ⇒ ⇒ **仍然有效** ⇒ ⇒ 可以覆盖界面；
    /// - `false` ⇒ ⇒ **已过期** ⇒ ⇒ ⇒ **必须丢弃**（否则就是「旧回调覆盖新工作」）。
    ///
    /// ⚠️ **只读**（不消耗票据）⇒ ⇒ ⇒ 同一张票据**可以**被多处检查
    /// ⇒ ⇒⇒ （如「onFailure」与「onDiagnostics」各查一次）。
    pub fn accept(&self, channel: &str, ticket: Ticket) -> bool {
        let channels = self.channels.borrow();
        let Some(current) = channels.get(channel) else {
            // ⭐ 该通道**从未**发起过工作 ⇒ ⇒ 这张票据**来路不明** ⇒ ⇒ 判为过期。
            return false;
        };
        *current == ticket.generation && ticket.channel == channel
    }

    /// ⭐⭐⭐⭐ **在票据有效时才应用**（原子地「判新旧 + 应用」）。
    ///
    /// ## 为什么要有这个（`accept` 单独用**不够**）
    ///
    /// ⇒ ⇒ 「先 `accept` 再写」是**两步** ⇒ ⇒ 两步之间**状态可能变**
    /// ⇒ ⇒⇒ 若那之间又有新工作 ⇒ ⇒⇒ 判据**已过期** ⇒ ⇒⇒ 却还是写了
    /// ⇒ ⇒⇒ ⇒ 「判过了就写」这个模式**本身**就有竞态。
    ///
    /// ⇒ ⇒⇒ ⇒ 本方法把「判 + 做」收成**一次调用** ⇒ ⇒⇒ 中间**没有窗口期**。
    ///
    /// ## 返回
    ///
    /// - `Some(value)` ⇒ ⇒ **已应用**（票据有效）；
    /// - `None` ⇒ ⇒ **已过期** ⇒ ⇒ ⇒ **没有应用**（应用函数**根本没被调用**）。
    pub fn apply_if_current<T>(
        &self,
        channel: &str,
        ticket: &Ticket,
        apply: impl FnOnce() -> T,
    ) -> Option<T> {
        if self.accept(channel, ticket.clone()) {
            Some(apply())
        } else {
            None
        }
    }

    /// ⭐⭐⭐⭐ **在票据有效时才写信号**（最常见场景：异步结果**写回状态**）。
    ///
    /// ## ⭐⭐⭐ 为什么这一条是「根本」
    ///
    /// ⇒ ⇒ 异步结果最终要落到**状态**里（signal）⇒ ⇒ 若旧结果**写进去了**
    /// ⇒ ⇒⇒ 状态**被污染** ⇒ ⇒⇒ 界面**重算**后就是旧的 ⇒ ⇒⇒
    /// ⇒ ⇒⇒ ⇒ **「界面自己跳回去」的根源在「写回」这一层** ⇒ ⇒⇒
    /// ⇒ ⇒⇒ ⇒ 而快照（`ui-app`）只挡「整棵树的发布」⇒ ⇒⇒ ⇒ 那是**表层**
    /// ⇒ ⇒⇒ ⇒ ⇒ 所以这一条才是**更普适、更根本**的防线。
    ///
    /// ## 返回
    ///
    /// `true` ⇒ ⇒ **已写入**；`false` ⇒ ⇒ **已过期** ⇒ ⇒ ⇒ **信号**保持原值**。
    pub fn guarded_set<T>(
        &self,
        signal: &crate::signal::Signal<T>,
        channel: &str,
        ticket: &Ticket,
        value: T,
    ) -> bool {
        if self.accept(channel, ticket.clone()) {
            signal.set(value);
            true
        } else {
            false
        }
    }

    /// ⭐ **当前代号**（该通道）⇒ ⇒ 没有则 `None`。
    pub fn generation(&self, channel: &str) -> Option<u64> {
        self.channels.borrow().get(channel).copied()
    }

    /// ⭐ **作废某通道的全部票据**（如：用户离开该屏幕 ⇒ ⇒ 该屏幕的结果全部作废）。
    ///
    /// ## 口径
    ///
    /// ⇒ ⇒ 代号 **+1** ⇒ ⇒ ⇒ 所有**已发出**的票据**立刻**过期
    /// ⇒ ⇒⇒ （而不是等下一次 `begin`）。
    pub fn invalidate(&self, channel: &str) {
        let mut channels = self.channels.borrow_mut();
        let counter = channels.entry(channel.to_owned()).or_insert(0);
        *counter += 1;
    }

    /// ⭐ **通道数**（测试与诊断用）。
    pub fn channel_count(&self) -> usize {
        self.channels.borrow().len()
    }
}

/// ⭐⭐⭐ **票据**：一次工作的「身份 + 代号」。
///
/// ## 为什么是**值**而不是引用
///
/// ⇒ ⇒ 票据要能**跨异步边界**传（进闭包、进回调）
/// ⇒ ⇒ 若它是引用 ⇒ ⇒ 生命周期管理麻烦 ⇒ ⇒⇒ 且**可被意外改写**
/// ⇒ ⇒⇒ ⇒ 值语义 ⇒ ⇒ 天然是「不可变凭证」。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ticket {
    /// 通道名（用于**防串线**：别的通道的票据**不被接受**）。
    pub channel: String,
    /// 该通道的代号（发起时的当前值）。
    pub generation: u64,
}

impl Ticket {
    /// ⭐ 代号（供日志与断言用）。
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// ⭐ 通道名。
    pub fn channel(&self) -> &str {
        self.channel.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⭐⭐⭐ **旧回调不能覆盖新工作**（本模块存在的**唯一理由**）。
    #[test]
    fn an_older_ticket_is_rejected_after_a_newer_one_starts() {
        let arbiter = Arbiter::new();
        let first = arbiter.begin("search");
        let second = arbiter.begin("search");
        // ⭐ 最新的有效。
        assert!(arbiter.accept("search", second.clone()));
        // ⭐ 旧的**必须**无效（否则界面会跳回去）。
        assert!(
            !arbiter.accept("search", first),
            "⭐ 旧票据必须被判为过期（旧回调不能覆盖新工作）"
        );
    }

    /// ⭐⭐⭐ **通道隔离**：城市的旧结果**不该**让搜索失效。
    #[test]
    fn channels_are_isolated_from_each_other() {
        let arbiter = Arbiter::new();
        let city = arbiter.begin("city");
        let search = arbiter.begin("search");
        // ⭐ 各自有效。
        assert!(arbiter.accept("city", city.clone()));
        assert!(arbiter.accept("search", search.clone()));
        // ⭐ 但城市的**新**工作**不该**让搜索的票据过期。
        let city_new = arbiter.begin("city");
        assert!(arbiter.accept("search", search.clone()));
        // ⭐ 而城市自己的旧票据**过期**了。
        assert!(!arbiter.accept("city", city));
        assert!(arbiter.accept("city", city_new));
    }

    /// ⭐⭐⭐ **票据防串线**：别的通道的票据**不被接受**。
    #[test]
    fn a_ticket_from_another_channel_is_rejected() {
        let arbiter = Arbiter::new();
        let ticket = arbiter.begin("a");
        assert!(
            !arbiter.accept("b", ticket),
            "⭐ 通道 b 不得接受通道 a 的票据（防串线）"
        );
    }

    /// ⭐⭐⭐ **作废通道**：离开屏幕 ⇒ ⇒ 该屏幕的结果全部作废。
    #[test]
    fn invalidating_a_channel_rejects_its_outstanding_tickets() {
        let arbiter = Arbiter::new();
        let ticket = arbiter.begin("city");
        assert!(arbiter.accept("city", ticket.clone()));
        arbiter.invalidate("city");
        assert!(
            !arbiter.accept("city", ticket),
            "⭐ 作废后该通道的旧票据必须全部失效"
        );
    }

    /// ⭐⭐⭐ **作废**不通道**互不影响**。
    #[test]
    fn invalidating_one_channel_leaves_others_alone() {
        let arbiter = Arbiter::new();
        let city = arbiter.begin("city");
        let search = arbiter.begin("search");
        arbiter.invalidate("city");
        assert!(arbiter.accept("search", search));
        assert!(!arbiter.accept("city", city));
    }

    /// ⭐⭐⭐ **代号单调递增**（r10 `generation` 的等价口径）。
    #[test]
    fn generations_increase_monotonically() {
        let arbiter = Arbiter::new();
        let first = arbiter.begin("x");
        let second = arbiter.begin("x");
        assert!(
            second.generation() > first.generation(),
            "⭐ 代号必须单调递增"
        );
        assert_eq!(arbiter.generation("x"), Some(second.generation()));
    }

    /// ⭐⭐⭐ **票据是值语义**（可安全跨异步边界传）。
    #[test]
    fn a_ticket_can_be_moved_into_a_closure_and_still_works() {
        let arbiter = Arbiter::new();
        let ticket = arbiter.begin("x");
        let moved = ticket.clone();
        let check = move || arbiter.accept("x", moved.clone());
        assert!(check(), "⭐ 票据移进闭包后仍可用于判新旧");
        assert!(check(), "⭐ 票据可被**多次**检查（只读、不消耗）");
    }

    /// ⭐⭐⭐⭐ **过期结果写不进信号**（这才是「界面不跳回去」的**根本**）。
    ///
    /// ## 为什么这条比「快照」更根本
    ///
    /// ⇒ ⇒ 异步结果最终要落到**状态**里 ⇒ ⇒ 若旧结果**写进去了**
    /// ⇒ ⇒⇒ 状态**被污染** ⇒ ⇒⇒ 界面重算后就是旧的
    /// ⇒ ⇒⇒ ⇒ **根源在「写回」这一层** ⇒ ⇒⇒ ⇒ 快照只挡「整棵树的发布」（表层）。
    #[test]
    fn a_stale_result_cannot_write_into_the_signal() {
        let arbiter = Arbiter::new();
        let signal = crate::signal(String::from("初始"));
        let stale = arbiter.begin("search");
        // ⭐ 新工作开始 ⇒ ⇒ 旧票据过期。
        let fresh = arbiter.begin("search");
        // ⭐ 旧结果回来 ⇒ ⇒ **写不进**。
        let written = arbiter.guarded_set(&signal, "search", &stale, String::from("旧"));
        assert!(!written, "⭐ 过期结果不得写入信号");
        assert_eq!(
            signal.get(),
            String::from("初始"),
            "⭐ 信号必须保持原值（状态未被污染）"
        );
        // ⭐ 新结果回来 ⇒ ⇒ 写进。
        assert!(arbiter.guarded_set(&signal, "search", &fresh, String::from("新")));
        assert_eq!(signal.get(), String::from("新"), "⭐ 新结果必须写入");
    }

    /// ⭐⭐⭐⭐ **`apply_if_current` 在过期时**根本不调用**应用函数**。
    ///
    /// ⇒ ⇒ 这是「判 + 做」**没有窗口期**的关键 ⇒ ⇒
    /// ⇒ ⇒ 若实现是「先判再调用」⇒ ⇒⇒ 中间可能又有新工作 ⇒ ⇒⇒ 竞态。
    #[test]
    fn apply_if_current_does_not_run_the_apply_when_stale() {
        use std::cell::Cell;
        let arbiter = Arbiter::new();
        let stale = arbiter.begin("x");
        let _fresh = arbiter.begin("x");
        let ran = Cell::new(false);
        let result = arbiter.apply_if_current("x", &stale, || {
            ran.set(true);
            42
        });
        assert!(result.is_none(), "⭐ 过期 ⇒ ⇒ 不返回结果");
        assert!(!ran.get(), "⭐ 过期 ⇒ ⇒ **应用函数根本没被调用**");
    }

    /// ⭐⭐⭐⭐ **`apply_if_current` 有效时执行并返回结果**。
    #[test]
    fn apply_if_current_runs_the_apply_when_current() {
        let arbiter = Arbiter::new();
        let ticket = arbiter.begin("x");
        let result = arbiter.apply_if_current("x", &ticket, || 42);
        assert_eq!(result, Some(42), "⭐ 有效 ⇒ ⇒ 必须执行并返回");
    }

    /// ⭐⭐⭐ **来路不明的票据**被判为过期（不因为「没人发起过」而放行）。
    #[test]
    fn a_ticket_for_an_unknown_channel_is_rejected() {
        let arbiter = Arbiter::new();
        let ticket = arbiter.begin("known");
        let other = Arbiter::new();
        assert!(
            !other.accept("known", ticket),
            "⭐ 另一个仲裁器不得接受本仲裁器的票据"
        );
    }
}
