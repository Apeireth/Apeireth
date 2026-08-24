//! `apeireth-bus::lifecycle` — 5 lifecycle hooks (审计 A1#5, 2026-08-16 backlog 全清).
//!
//! 宿主生命周期钩子: `UserPromptSubmit` / `SessionStart` / `SessionEnd` /
//! `PostToolUse` / `Stop`。
//!
//! 机制 (集成而非分立): `LifecycleBus` = hook 注册表 (trait 对象) + 可选 L0Bus
//! 广播 (topic `lifecycle.<event>`, 复用总线 pub-sub, 不另立通道)。
//!
//! 用法:
//! ```ignore
//! let hooks = LifecycleBus::new().register(Box::new(MyHook));
//! hooks.fire(LifecycleEvent::SessionStart, LifecycleContext::new("s1")).await;
//! ```
//!
//! 0 假装 (诚实): `fire()` 收集各 hook 错误返回 (不吞不假装); 无 hook 时
//! fire 是空操作; L0Bus 广播失败只记 eprintln, 不影响 hooks 执行。

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::l0::L0Bus;
use crate::BusMessage;

/// 五个宿主生命周期事件.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LifecycleEvent {
    /// 用户提交了一条 prompt (query 在 detail).
    UserPromptSubmit,
    /// 会话开始 (id 在 session_id).
    SessionStart,
    /// 会话结束.
    SessionEnd,
    /// 工具执行后 (工具名 + 结果摘要).
    PostToolUse,
    /// 宿主停止.
    Stop,
}

impl LifecycleEvent {
    /// 事件名 (topic 后缀 + 调试).
    pub fn as_str(&self) -> &'static str {
        match self {
            LifecycleEvent::UserPromptSubmit => "user_prompt_submit",
            LifecycleEvent::SessionStart => "session_start",
            LifecycleEvent::SessionEnd => "session_end",
            LifecycleEvent::PostToolUse => "post_tool_use",
            LifecycleEvent::Stop => "stop",
        }
    }
}

/// 事件上下文 (通用载荷; 事件相关字段放 detail).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LifecycleContext {
    /// 会话 id (可空).
    pub session_id: Option<String>,
    /// 事件相关详情 (如 query / tool 名 / 结果摘要).
    pub detail: Option<String>,
    /// 时间戳 (epoch millis).
    pub created_at_ms: i64,
}

impl LifecycleContext {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: Some(session_id.into()),
            detail: None,
            created_at_ms: crate::now_ms(),
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// 生命周期钩子: 宿主在对应事件 fire 时被异步调用.
#[async_trait::async_trait]
pub trait LifecycleHook: Send + Sync {
    /// 关注的 (事件, 会话) 对; None = 全部会话.
    fn watch(&self) -> (LifecycleEvent, Option<String>);
    /// 事件处理; Err 会被 fire() 收集返回 (不吞).
    async fn on_event(&self, ctx: &LifecycleContext) -> Result<(), String>;
}

/// 广播载荷 (挂 L0Bus 用).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleMessage {
    pub event: LifecycleEvent,
    pub ctx: LifecycleContext,
}

/// 生命周期总线: hook 注册表 + 可选 L0Bus 广播.
#[derive(Clone)]
pub struct LifecycleBus {
    inner: Arc<LifecycleBusInner>,
}

struct LifecycleBusInner {
    hooks: Vec<Box<dyn LifecycleHook>>,
    l0: Option<L0Bus<LifecycleMessage>>,
}

impl Default for LifecycleBus {
    fn default() -> Self {
        Self::new()
    }
}

impl LifecycleBus {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(LifecycleBusInner {
                hooks: Vec::new(),
                l0: None,
            }),
        }
    }

    /// 挂到现有 L0Bus (广播到 topic `lifecycle.<event>`; 复用总线, 不另立).
    pub fn with_l0(mut self, bus: L0Bus<LifecycleMessage>) -> Self {
        // Arc::get_mut 仅在唯一引用时可用; new() 后直接调用是唯一的
        match Arc::get_mut(&mut self.inner) {
            Some(inner) => inner.l0 = Some(bus),
            None => eprintln!("[lifecycle] with_l0 失败: 总线已共享 (clone 后不能再挂 L0)"),
        }
        self
    }

    /// 注册 hook (按注册顺序执行).
    pub fn register(mut self, h: Box<dyn LifecycleHook>) -> Self {
        match Arc::get_mut(&mut self.inner) {
            Some(inner) => inner.hooks.push(h),
            None => eprintln!("[lifecycle] register 失败: 总线已共享 (clone 后不能再注册)"),
        }
        self
    }

    /// 已注册 hook 数 (诊断).
    pub fn hook_count(&self) -> usize {
        self.inner.hooks.len()
    }

    /// 触发事件: 执行匹配 hooks (错误收集返回) + 广播到 L0Bus.
    /// 返回各 hook 的错误 (无错误 = 空 Vec; 不吞不假装).
    pub async fn fire(&self, event: LifecycleEvent, ctx: LifecycleContext) -> Vec<String> {
        let mut errors: Vec<String> = Vec::new();
        for h in &self.inner.hooks {
            let (we, wsession) = h.watch();
            if we != event {
                continue;
            }
            if let Some(s) = &wsession {
                if ctx.session_id.as_ref() != Some(s) {
                    continue;
                }
            }
            if let Err(e) = h.on_event(&ctx).await {
                errors.push(format!("[{}] {e}", event.as_str()));
            }
        }
        if let Some(l0) = &self.inner.l0 {
            let topic = format!("lifecycle.{}", event.as_str());
            if let Err(e) = l0
                .publish(&topic, BusMessage::new(LifecycleMessage { event, ctx }))
                .await
            {
                eprintln!("[lifecycle] L0 广播失败 ({topic}): {e}");
            }
        }
        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingHook {
        event: LifecycleEvent,
        session: Option<String>,
        count: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl LifecycleHook for CountingHook {
        fn watch(&self) -> (LifecycleEvent, Option<String>) {
            (self.event, self.session.clone())
        }
        async fn on_event(&self, _ctx: &LifecycleContext) -> Result<(), String> {
            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    struct FailingHook;

    #[async_trait::async_trait]
    impl LifecycleHook for FailingHook {
        fn watch(&self) -> (LifecycleEvent, Option<String>) {
            (LifecycleEvent::Stop, None)
        }
        async fn on_event(&self, _ctx: &LifecycleContext) -> Result<(), String> {
            Err("故意失败".to_string())
        }
    }

    #[tokio::test]
    async fn hooks_fire_on_matching_event_only() {
        let c1 = Arc::new(AtomicUsize::new(0));
        let c2 = Arc::new(AtomicUsize::new(0));
        let bus = LifecycleBus::new()
            .register(Box::new(CountingHook {
                event: LifecycleEvent::SessionStart,
                session: None,
                count: Arc::clone(&c1),
            }))
            .register(Box::new(CountingHook {
                event: LifecycleEvent::SessionEnd,
                session: None,
                count: Arc::clone(&c2),
            }));
        let errs = bus
            .fire(LifecycleEvent::SessionStart, LifecycleContext::new("s1"))
            .await;
        assert!(errs.is_empty());
        assert_eq!(c1.load(Ordering::SeqCst), 1, "SessionStart hook 应触发");
        assert_eq!(c2.load(Ordering::SeqCst), 0, "SessionEnd hook 不应触发");
    }

    #[tokio::test]
    async fn session_scoped_hook_filters() {
        let c = Arc::new(AtomicUsize::new(0));
        let bus = LifecycleBus::new().register(Box::new(CountingHook {
            event: LifecycleEvent::UserPromptSubmit,
            session: Some("s-target".to_string()),
            count: Arc::clone(&c),
        }));
        bus.fire(
            LifecycleEvent::UserPromptSubmit,
            LifecycleContext::new("s-other"),
        )
        .await;
        assert_eq!(c.load(Ordering::SeqCst), 0, "其他会话不应触发");
        bus.fire(
            LifecycleEvent::UserPromptSubmit,
            LifecycleContext::new("s-target"),
        )
        .await;
        assert_eq!(c.load(Ordering::SeqCst), 1, "目标会话应触发");
    }

    #[tokio::test]
    async fn errors_are_collected_not_swallowed() {
        let bus = LifecycleBus::new().register(Box::new(FailingHook));
        let errs = bus
            .fire(LifecycleEvent::Stop, LifecycleContext::new("s1"))
            .await;
        assert_eq!(errs.len(), 1, "hook 错误应收集返回");
        assert!(errs[0].contains("故意失败"));
    }

    #[tokio::test]
    async fn no_hooks_fire_is_noop() {
        let bus = LifecycleBus::new();
        let errs = bus
            .fire(LifecycleEvent::Stop, LifecycleContext::new("s1"))
            .await;
        assert!(errs.is_empty());
        assert_eq!(bus.hook_count(), 0);
    }

    #[tokio::test]
    async fn broadcasts_to_l0_topic() {
        let l0: L0Bus<LifecycleMessage> = L0Bus::new();
        let bus = LifecycleBus::new().with_l0(l0.clone());
        let mut sub = l0.subscribe("lifecycle.session_start").await.unwrap();
        bus.fire(LifecycleEvent::SessionStart, LifecycleContext::new("s1"))
            .await;
        let msg = tokio::time::timeout(std::time::Duration::from_secs(2), sub.next())
            .await
            .expect("应有广播")
            .unwrap()
            .unwrap();
        assert_eq!(msg.payload.event, LifecycleEvent::SessionStart);
        assert_eq!(msg.payload.ctx.session_id.as_deref(), Some("s1"));
    }
}
