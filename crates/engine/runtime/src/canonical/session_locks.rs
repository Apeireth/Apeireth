//! 每会话串行闸（`SessionLocks`）—— 从 `runtime.rs` 平移出来。
//!
//! 独立成文件的理由：这是一个**自洽的单一职责单元**（每会话一把锁 + 有界淘汰），
//! 与运行时其余部分无耦合；`runtime.rs` 在 arch_guard 尺寸基线内（只减不增），
//! 把它拆出来腾出空间给新能力，而不是靠压注释还账。
//!
//! 语义不变：不同会话可并发；同一会话的回合与审批解析互斥；
//! 锁表有界（M20），只驱逐「无等待者」的条目。

use std::collections::BTreeMap;
use std::sync::Arc;

use apeireth_core::kernel::SessionId;
use tokio::sync::Mutex as TokioMutex;

/// Per-session serialization for turns and approval resolution.
///
/// This is deliberately not a global runtime mutex. Different sessions may
/// proceed concurrently; the same session cannot start a new turn or resolve
/// an approval while another operation on that session is still in progress.
///
/// M20: the map is bounded (`MAX_SESSION_LOCKS`). An unbounded lock map grows
/// linearly with the number of distinct sessions a long-running process ever
/// touched — each entry is small, but the map is never pruned. Entries are
/// only evicted when no waiter holds them (`Arc::strong_count == 1`), so an
/// in-flight turn can never lose its serialization point.
#[derive(Debug, Default)]
pub struct SessionLocks {
    locks: TokioMutex<BTreeMap<SessionId, Arc<TokioMutex<()>>>>,
}

/// 同时驻留的 per-session 锁上限 (M20)。4096 个并发会话对一个本地运行时
/// 已是极端值; 超限时只驱逐"无等待者"的条目。
const MAX_SESSION_LOCKS: usize = 4096;

impl SessionLocks {
    pub(crate) async fn acquire(&self, session: SessionId) -> Arc<TokioMutex<()>> {
        let mut map = self.locks.lock().await;
        if let Some(existing) = map.get(&session) {
            return Arc::clone(existing);
        }
        // 无等待者 (仅 map 自己持有一份 Arc) 才可驱逐: 正在 hold 或等待该
        // 锁的任务不会因为淘汰而拿到另一个互斥体。
        if map.len() >= MAX_SESSION_LOCKS {
            let evictable = map
                .iter()
                .find(|(_, lock)| Arc::strong_count(lock) == 1)
                .map(|(id, _)| *id);
            if let Some(evictable) = evictable {
                map.remove(&evictable);
            }
        }
        let entry = map
            .entry(session)
            .or_insert_with(|| Arc::new(TokioMutex::new(())));
        Arc::clone(entry)
    }
}
