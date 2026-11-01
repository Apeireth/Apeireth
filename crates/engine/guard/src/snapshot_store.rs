//! 特征快照的有界存储。
//!
//! # 为什么单独一个模块
//!
//! guard hook 是**长生命周期单例**（CLI 装配为 `Arc<BehaviorChainGuardHook>`），
//! 而每次评估都要写一份完整的 `FeatureSnapshot`。没有上限 ⇒ 按「动作数」
//! **无界增长**：一次长会话就能吃掉大量内存。
//!
//! 这个存储把「有界」这件事收在一处，顺带解决两个问题：
//!
//! 1. **写入有上限**：超过 `MAX_LAST_SNAPSHOTS` 时按**插入顺序**淘汰最旧的。
//!    快照本身没有时间戳字段，所以插入序另记一条队列 —— 与 `hook.rs` 里
//!    `chain_order` 同一套路。
//! 2. **会话清理**：按会话整体清除（`clear_session`）。队列与表同步清，
//!    否则队列里会留下永不命中的幽灵键。
//!
//! # 读口
//!
//! `last_feature_snapshot` 是**诊断读口**（给自述面/运维看最近一次评估的特征）。
//! 淘汰最旧条目**不影响护栏判定** —— 判定走 `chains` / `session_risk_history`，
//! 那两张表不受本存储影响。

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use crate::snapshot::FeatureSnapshot;
use apeireth_core::kernel::SessionId;

/// 同时驻留的特征快照上限（按 `(会话, 动作)` 计）。
pub const MAX_LAST_SNAPSHOTS: usize = 1024;

/// 有界的特征快照存储：`HashMap` + 插入序队列。
#[derive(Default)]
pub struct SnapshotStore {
    entries: Mutex<HashMap<(SessionId, String), FeatureSnapshot>>,
    /// `entries` 的插入序（先进先出）。用于「淘汰最旧的」。
    order: Mutex<VecDeque<(SessionId, String)>>,
}

impl SnapshotStore {
    /// 构造一个空存储。
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            order: Mutex::new(VecDeque::new()),
        }
    }

    /// 写入一份快照；超过上限时按插入顺序淘汰最旧的。
    pub fn insert(&self, session: &SessionId, action_id: &str, snapshot: FeatureSnapshot) {
        let key = (*session, action_id.to_string());
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        entries.insert(key.clone(), snapshot);
        let mut order = self.order.lock().unwrap_or_else(|e| e.into_inner());
        order.push_back(key);
        while order.len() > MAX_LAST_SNAPSHOTS {
            let Some(oldest) = order.pop_front() else {
                break;
            };
            entries.remove(&oldest);
        }
    }

    /// 取一份快照（诊断读口）。
    pub fn get(&self, session: &SessionId, action_id: &str) -> Option<FeatureSnapshot> {
        let entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        entries.get(&(*session, action_id.to_string())).cloned()
    }

    /// 清除某个会话的全部快照（会话结束时调用）。
    ///
    /// # 为什么不叫 `clear_session`
    ///
    /// 兄弟类型 `BehaviorChainGuardHook` 也有一个 `clear_session`。审计工具
    /// (`audit_unwired.py`) 用**裸 token 计数**判「生产零引用」，同名方法会
    /// **互相遮蔽** —— 一侧被调用，另一侧会被误判成「已接电」。
    /// 所以这里用 `remove_session`，避开同名。
    pub fn remove_session(&self, session: &SessionId) {
        self.entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|(s, _), _| s != session);
        self.order
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|(s, _)| s != session);
    }

    /// 当前驻留条数（测试与诊断用）。
    pub fn len(&self) -> usize {
        self.entries.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    /// 是否没有驻留任何快照。
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 是否驻留着某个 `(会话, 动作)` 的快照（测试与诊断用）。
    pub fn contains(&self, session: &SessionId, action_id: &str) -> bool {
        self.entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key(&(*session, action_id.to_string()))
    }
}
