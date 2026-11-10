//! 关系面的两个存储（羁绊 + 里程碑）的构造。
//!
//! # 为什么放一起
//!
//! 它们是同一条产品线的两半：**羁绊**是当下的关系状态，**里程碑**是关系史里
//! 那些值得记住的节点（初见、阶段跃迁）。两者都必须是**跨重启存活**的 ——
//! 内存版重启即散，那种关系记忆等于没有。
//!
//! 生产装配共用同一个 `cognitive.sqlite3` 连接池：关系状态与认知记忆同生共死。
//!
//! # 为什么单独一个模块
//!
//! 尺寸棘轮（`cli/src/lib.rs` 在基线边缘，只减不增）：新增存储时不该把装配根
//! 顶上去。构造与理由放在这里，装配根只留两行调用。

use std::sync::Arc;

use apeireth_memory::milestone::MilestoneStore;
use apeireth_memory::partner::PartnerStore;
use apeireth_memory::{SqliteMilestoneStore, SqlitePartnerStore};
use apeireth_storage::SqliteConnectionPool;

/// 开关开时构造羁绊存储（与认知库同一份库）；关时 `None` = 该面不注册。
pub fn partner_store(
    enabled: bool,
    pool: &Arc<SqliteConnectionPool>,
) -> Option<Arc<dyn PartnerStore>> {
    enabled
        .then(|| Arc::new(SqlitePartnerStore::from_arc(Arc::clone(pool))) as Arc<dyn PartnerStore>)
}

/// 开关开时构造里程碑存储（与认知库同一份库）；关时 `None` = 不记里程碑。
pub fn milestone_store(
    enabled: bool,
    pool: &Arc<SqliteConnectionPool>,
) -> Option<Arc<dyn MilestoneStore>> {
    enabled.then(|| {
        Arc::new(SqliteMilestoneStore::from_arc(Arc::clone(pool))) as Arc<dyn MilestoneStore>
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_switch_constructs_nothing() {
        let pool = Arc::new(
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("rt")
                .block_on(SqliteConnectionPool::in_memory())
                .expect("pool"),
        );
        assert!(partner_store(false, &pool).is_none());
        assert!(milestone_store(false, &pool).is_none());
        assert!(partner_store(true, &pool).is_some());
        assert!(milestone_store(true, &pool).is_some());
    }
}
