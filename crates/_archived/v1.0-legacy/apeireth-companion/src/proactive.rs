//! `apeireth-companion::proactive` — 主动涌现的「接真」层.
//!
//! 把纯机制 (emergence) 接上真实的外部世界:
//! - `LarkDelivery`         — 真送达 (飞书 IM, wrap apeireth-lark 的 LarkRealImpl)
//! - `MemoryContextSource`  — 真记忆检索 (wrap apeireth-memory 的 SqliteMemoryStore)
//! - `ProactiveDriver`      — 真心跳 (tokio interval), 把「一直运行」变成真的
//!
//! **诚实登记**:
//! - LarkDelivery 需飞书凭据 (app_id/app_secret + receive_id) 才能真发;
//! - MemoryContextSource 需真 SQLite 数据才有上下文 (空库返回 None).

use std::sync::Arc;
use std::time::Duration;

use apeireth_lark::{LarkClient, LarkConfig, LarkRealImpl, MessageType};
use apeireth_memory::{EpisodeStore, SqliteMemoryStore};
use async_trait::async_trait;

use crate::emergence::{Boundaries, Delivery, EmergenceLoop, Initiative};
use crate::Bond;

// ============================================================
// 上下文源 (记忆检索)
// ============================================================

/// 从记忆检索「关于用户的东西」, 供 Initiative.context_hint.
pub trait ContextSource: Send + Sync {
    /// 给定主体 (session_id / continuity_id), 返回一条最近的相关上下文 (非固定文案).
    fn context_for(&self, subject: &str) -> Option<String>;
}

/// 空上下文 (诚实: 没接记忆时的默认).
#[derive(Debug, Clone, Copy, Default)]
pub struct EmptyContext;

impl ContextSource for EmptyContext {
    fn context_for(&self, _subject: &str) -> Option<String> {
        None
    }
}

/// 真记忆检索: 从 SqliteMemoryStore 捞「这个 session」最近一条 episode 的 content.
#[derive(Clone)]
pub struct MemoryContextSource {
    store: Arc<SqliteMemoryStore>,
}

impl MemoryContextSource {
    pub fn new(store: Arc<SqliteMemoryStore>) -> Self {
        Self { store }
    }
}

impl ContextSource for MemoryContextSource {
    fn context_for(&self, subject: &str) -> Option<String> {
        let eps = self.store.recent_episodes(subject, 5).ok()?;
        let last = eps.last()?;
        let content: String = last.content.chars().take(200).collect();
        Some(content)
    }
}

// ============================================================
// 真送达 (飞书 IM)
// ============================================================

/// 真送达: 飞书 IM. 需 LarkConfig(app_id/app_secret) + receive_id(chat/user open_id).
pub struct LarkDelivery {
    client: Arc<LarkRealImpl>,
    receive_id: String,
}

impl LarkDelivery {
    pub fn new(config: LarkConfig, receive_id: impl Into<String>) -> Result<Self, String> {
        let client = LarkRealImpl::new(config).map_err(|e| e.to_string())?;
        Ok(Self {
            client: Arc::new(client),
            receive_id: receive_id.into(),
        })
    }
}

#[async_trait]
impl Delivery for LarkDelivery {
    async fn deliver(&self, i: &Initiative) -> Result<(), String> {
        let text = i.to_message();
        let content = serde_json::json!({ "text": text }).to_string();
        self.client
            .send_message(&self.receive_id, MessageType::Text, &content)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

// ============================================================
// 真心跳驱动
// ============================================================

/// 真心跳驱动: 把「一直运行」变成真的 (tokio interval).
/// 每个 tick: 检索上下文 → 决策 → 送达. 观察交互由外部事件源调 `observe`.
pub struct ProactiveDriver<D: Delivery, C: ContextSource> {
    loop_: EmergenceLoop<Bond>,
    delivery: D,
    context: C,
    subject: String,
    tick_interval: Duration,
}

impl<D: Delivery, C: ContextSource> ProactiveDriver<D, C> {
    pub fn new(
        bond: Bond,
        boundaries: Boundaries,
        delivery: D,
        context: C,
        subject: impl Into<String>,
        tick_interval: Duration,
    ) -> Self {
        Self {
            loop_: EmergenceLoop::new(bond, boundaries),
            delivery,
            context,
            subject: subject.into(),
            tick_interval,
        }
    }

    /// 观察一次用户交互 (外部事件源调用, 喂节律 + 刷新最后接触).
    pub fn observe(&mut self, at: chrono::DateTime<chrono::Utc>) {
        self.loop_.observe_interaction(at);
    }

    /// 主循环 (daemon): 永不返回, 由外部 shutdown / kill 终止.
    pub async fn run(mut self) {
        let mut interval = tokio::time::interval(self.tick_interval);
        loop {
            interval.tick().await;
            let now = chrono::Utc::now();
            let hint = self.context.context_for(&self.subject);
            if let Some(init) = self.loop_.tick(now, hint) {
                if let Err(e) = self.delivery.deliver(&init).await {
                    eprintln!("[proactive] deliver failed: {e}");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_lark::LarkConfig;
    use apeireth_memory::CoreEpisode;

    #[test]
    fn memory_context_source_returns_latest_episode() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        store
            .put_episode(&CoreEpisode {
                id: "e1".into(),
                timestamp: 1,
                role: "assistant".into(),
                content: "你在改 council 的 bug".into(),
                session_id: "s1".into(),
            })
            .unwrap();
        let src = MemoryContextSource::new(store);
        let ctx = src.context_for("s1").unwrap();
        assert!(ctx.contains("council"));
    }

    #[test]
    fn empty_context_returns_none() {
        let src = EmptyContext;
        assert!(src.context_for("anything").is_none());
    }

    #[test]
    fn lark_delivery_rejects_empty_config() {
        let cfg = LarkConfig {
            app_id: "".into(),
            ..Default::default()
        };
        assert!(LarkDelivery::new(cfg, "receive").is_err());
    }
}
