//! 「**主体 → 会话**」归属映射的**实现**（持有记忆库的那一端）。
//!
//! # 为什么在 CLI 装配根
//!
//! * 端口（trait）在 `apeireth-runtime`（被 gateway / runtime-assembly / CLI 都依赖）
//!   ⇒ **无循环依赖**；
//! * **实现**要持有记忆库（`apeireth-memory`）⇒ `apeireth-gateway` **不依赖** memory
//!   ⇒ ⇒ 实现放**装配层**（CLI）⇒ gateway 只认端口。
//!
//! # 写侧语义（与 memory 侧一致）
//!
//! * **幂等**（`INSERT OR IGNORE`）；
//! * `principal_id` / `session_id` 为空 ⇒ **拒**（fail-closed：空主体会把所有人的
//!   episode 归到同一主体 ⇒ 误删风险）；
//! * **只写映射**，不动其它账。

use std::sync::Arc;

use apeireth_memory::SqliteMemoryStore;
use apeireth_runtime::canonical::principal_sessions::PrincipalSessionRecorder;

/// 持有记忆库的归属写面实现。
pub struct MemoryPrincipalSessionRecorder {
    store: Arc<SqliteMemoryStore>,
}

impl MemoryPrincipalSessionRecorder {
    /// 绑定记忆库（装配根用**同一本**认知库 ⇒ 与治理面同源）。
    pub fn new(store: Arc<SqliteMemoryStore>) -> Self {
        Self { store }
    }
}

impl PrincipalSessionRecorder for MemoryPrincipalSessionRecorder {
    fn record(&self, principal_id: &str, session_id: &str) -> Result<(), String> {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|value| value.as_millis() as i64)
            .unwrap_or_default();
        apeireth_memory::principal_sessions::record_principal_session(
            &self.store,
            principal_id,
            session_id,
            now_ms,
        )
        .map(|_inserted| ())
        .map_err(|error| error.to_string())
    }
}
