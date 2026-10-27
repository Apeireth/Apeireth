//! 护栏的**会话态登记表**（会话 LRU + 驱逐策略）。
//!
//! # 为什么独立成文件
//!
//! 护栏按会话键持有**多张表**（`session_scopes` / `session_risk_history` /
//! `session_behavior_summary` / `session_history` / `turn_intents` / `last_immune_signals`）
//! ⇒ ⇒ 长跑进程里**只增不减** ⇒ ⇒ **无界增长**（护栏接线审计 **P1-3**）。
//!
//! ⇒ 而 `clear_session` 虽有实现，**生产零调用点**
//! ⇒ ⇒（运行时**没有「会话结束」这个概念** ⇒ ⇒ 那是**协议决策**，护栏不能自己发明）
//! ⇒ ⇒ ⇒ 所以这里给这些表**自己**加上界：**会话 LRU** ⇒ 超限**整段驱逐**最久未用的会话。
//!
//! # 口径
//!
//! * **按会话**（不是按 trace）⇒ ⇒ 一个会话的所有表**一起**被清
//!   ⇒ ⇒ **不会出现「清了一半」的半吊子状态**；
//! * **LRU** ⇒ 命中即移到队尾 ⇒ ⇒ 驱逐的是**最久未用**的（**不是**任意一个）；
//! * 本模块**只管记账与选出受害者** ⇒ ⇒ **实际的清理**由调用方复用
//!   `BehaviorChainGuardHook::clear_session` 执行（**一份清理实现**）。

use std::collections::VecDeque;
use std::sync::Mutex;

use apeireth_core::kernel::SessionId;

/// **被跟踪会话数**上限（护栏审计 P1-3）。
///
/// ⚠️ 这是**可调旋钮** ⇒ ⇒ **要不要更大 ⇒ owner 可定**。
pub const MAX_TRACKED_SESSIONS: usize = 256;

/// 会话 LRU 登记表。
#[derive(Debug)]
pub struct SessionStateRegistry {
    /// 插入顺序（**队首 = 最久未用**）。
    order: Mutex<VecDeque<SessionId>>,
    /// 上限。
    limit: usize,
}

impl Default for SessionStateRegistry {
    fn default() -> Self {
        Self::new(MAX_TRACKED_SESSIONS)
    }
}

impl SessionStateRegistry {
    /// 新建登记表（`limit` = 上限）。
    pub fn new(limit: usize) -> Self {
        Self {
            order: Mutex::new(VecDeque::new()),
            limit,
        }
    }

    /// **记账一次会话访问** ⇒ ⇒ 返回**要驱逐的会话**（**最久未用的先**）。
    ///
    /// # 口径
    ///
    /// * 命中即**移到队尾** ⇒ ⇒ 「最近用过」⇒ ⇒ 驱逐时它**不在队首**；
    /// * 超限 ⇒ ⇒ 从**队首**弹出 ⇒ ⇒ 受害者是**最久未用**的；
    /// * ⚠️ **只记账与选受害者** ⇒ ⇒ **实际的清理**由调用方做
    ///   （⇒ ⇒ 调用方**必须**在**不持锁**时清理 ⇒ ⇒ 否则**死锁**）。
    pub fn track(&self, session_id: SessionId) -> Vec<SessionId> {
        let mut order = self
            .order
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if let Some(position) = (*order).iter().position(|entry| *entry == session_id) {
            order.remove(position);
        }
        order.push_back(session_id);
        let mut victims = Vec::new();
        while order.len() > self.limit {
            let Some(oldest) = order.pop_front() else {
                break;
            };
            victims.push(oldest);
        }
        victims
    }

    /// 当前**被跟踪**的会话数（**可观测**）。
    pub fn len(&self) -> usize {
        self.order.lock().map(|order| order.len()).unwrap_or(0)
    }

    /// 这个会话**当前是否还被跟踪**（**可观测** ⇒ ⇒ 用来验证 LRU 语义）。
    pub fn contains(&self, session_id: &SessionId) -> bool {
        self.order
            .lock()
            .map(|order| order.iter().any(|entry| *entry == *session_id))
            .unwrap_or(false)
    }

    /// 把一个会话**从登记表里移除**（**会话被显式清理**时调用）。
    ///
    /// ⚠️ **必须**在显式清理会话时调用 ⇒ ⇒ 否则留下**永不命中的幽灵 key**
    /// ⇒ ⇒ 挤占额度 ⇒ ⇒ **误杀**在用的会话。
    pub fn forget(&self, session_id: &SessionId) {
        self.order
            .lock()
            .map(|mut order| order.retain(|entry| *entry != *session_id))
            .ok();
    }
}
