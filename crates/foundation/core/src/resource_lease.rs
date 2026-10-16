//! 资源租约注册表: 引用计数租约 + pin 保活 + 快照即读 + 失败即帧。
//!
//! 面向"订阅共享/关停时机"资源面 (例如前端资源面的订阅管理), 五条语义:
//!
//! 1. **首持有者开流、末持有者关停** — 第一枚租约触发开流钩子, 最后一枚
//!    持有者 (租约或 pin 全体) 离开触发关停钩子; 中途共享无需反复开/关。
//! 2. **pin 保活** — [`ResourcePin`] 与租约同计数; 只要有 pin 在场, 流不关。
//! 3. **错过更新不回放** — 订阅端是 watch 通道: 后订阅者靠
//!    [`ResourceRegistry::snapshot`] 即读最新帧, 历史帧不补发。
//! 4. **失败即帧** — [`ResourceRegistry::publish_failure`] 把错误作为正式一帧
//!    挂在最后值旁; 下一成功帧自动清除它, 不留半态。
//! 5. **关停后无幽灵回调** — [`ResourceRegistry::shutdown`] 后不再触发任何钩子、
//!    不再投递任何更新; 事后释放的租约也不会补调关停钩子。
//!
//! 钩子在登记锁内同步调用: 开/关与计数互斥, 不会双开双关。钩子自身不得
//! 回调本注册表 (会死锁); 需要联动时在钩子外层另行编排。

use std::sync::{Arc, Mutex};

use thiserror::Error;
use tokio::sync::watch;

/// 开流钩子: 首持有者到来时调用一次; 返回错误则租约不发放 (fail-closed)。
pub type OpenHook = Arc<dyn Fn() -> Result<(), String> + Send + Sync>;

/// 关停钩子: 末持有者离开 (或关停) 时调用一次。
pub type CloseHook = Arc<dyn Fn() + Send + Sync>;

/// 一对开/关停钩子。
pub struct ResourceHooks {
    on_open: OpenHook,
    on_close: CloseHook,
}

impl ResourceHooks {
    /// 组装一对钩子。
    pub fn new(on_open: OpenHook, on_close: CloseHook) -> Self {
        Self { on_open, on_close }
    }

    /// 无动作钩子 (纯登记资源, 不驱动任何流)。
    pub fn inert() -> Self {
        Self {
            on_open: Arc::new(|| Ok(())),
            on_close: Arc::new(|| ()),
        }
    }
}

/// 租约操作的失败。
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ResourceError {
    /// 流未开 (无持有者): 该操作只在流开启期间有意义。
    #[error("resource {0} stream is not open")]
    StreamNotOpen(String),
    /// 注册表已关停: 不再发放租约、不再收帧、不再触发钩子。
    #[error("resource {0} is shut down")]
    Shutdown(String),
    /// 开流钩子拒绝: 租约未发放。
    #[error("resource {name} refused to open: {reason}")]
    OpenFailed {
        /// 资源名。
        name: String,
        /// 钩子给出的拒绝原因。
        reason: String,
    },
}

/// 资源面的一帧: 最后值 + 挂在其旁的失败帧 (下一成功帧自清)。
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceFrame<V> {
    /// 最后一次成功帧的值 (从未成功过则为空)。
    pub value: Option<V>,
    /// 最近一次失败帧; 成功帧到达后自动清除。
    pub failure: Option<String>,
}

impl<V> Default for ResourceFrame<V> {
    fn default() -> Self {
        Self {
            value: None,
            failure: None,
        }
    }
}

impl<V> ResourceFrame<V> {
    /// 当前是否挂着失败帧。
    pub fn has_failure(&self) -> bool {
        self.failure.is_some()
    }
}

/// 一枚持流租约: 持有期间流保持开启; 全部释放 (且无 pin) 即关停。
/// Drop 即释放, 无需显式归还。
pub struct ResourceLease {
    owner: Arc<dyn LeaseRelease>,
}

impl std::fmt::Debug for ResourceLease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceLease").finish_non_exhaustive()
    }
}

/// 一枚保活 pin: 与租约同计数; 只要有 pin 在场, 流不关。
/// Drop 即释放。
pub struct ResourcePin {
    owner: Arc<dyn LeaseRelease>,
}

impl std::fmt::Debug for ResourcePin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourcePin").finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy)]
enum LeaseKind {
    Holder,
    Keepalive,
}

trait LeaseRelease: Send + Sync {
    fn release_lease(&self, kind: LeaseKind);
}

impl Drop for ResourceLease {
    fn drop(&mut self) {
        self.owner.release_lease(LeaseKind::Holder);
    }
}

impl Drop for ResourcePin {
    fn drop(&mut self) {
        self.owner.release_lease(LeaseKind::Keepalive);
    }
}

struct SurfaceState<V> {
    name: String,
    holders: usize,
    pins: usize,
    open: bool,
    shutdown: bool,
    frame: ResourceFrame<V>,
    hooks: ResourceHooks,
}

/// 引用计数的资源租约注册表。持有 [`ResourceLease`] / [`ResourcePin`] 的
/// 一方共同决定流的开与关; 帧读写走 [`ResourceRegistry::snapshot`] 与
/// [`ResourceRegistry::subscribe`]。
pub struct ResourceRegistry<V> {
    state: Mutex<SurfaceState<V>>,
    updates: watch::Sender<ResourceFrame<V>>,
}

impl<V> std::fmt::Debug for ResourceRegistry<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        f.debug_struct("ResourceRegistry")
            .field("name", &state.name)
            .field("holders", &state.holders)
            .field("pins", &state.pins)
            .field("open", &state.open)
            .field("shutdown", &state.shutdown)
            .finish()
    }
}

impl<V: Clone + Send + Sync + 'static> ResourceRegistry<V> {
    /// 建一个命名资源面; 流的开/关由 `hooks` 驱动。
    pub fn new(name: impl Into<String>, hooks: ResourceHooks) -> Arc<Self> {
        let (updates, _initial) = watch::channel(ResourceFrame::default());
        Arc::new(Self {
            state: Mutex::new(SurfaceState {
                name: name.into(),
                holders: 0,
                pins: 0,
                open: false,
                shutdown: false,
                frame: ResourceFrame::default(),
                hooks,
            }),
            updates,
        })
    }

    /// 资源名。
    pub fn name(&self) -> String {
        self.lock_state().name.clone()
    }

    /// 领一枚持流租约; 首持有者触发开流钩子。
    pub fn acquire(self: &Arc<Self>) -> Result<ResourceLease, ResourceError> {
        self.enter(LeaseKind::Holder)?;
        let concrete = Arc::clone(self);
        let owner: Arc<dyn LeaseRelease> = concrete;
        Ok(ResourceLease { owner })
    }

    /// 领一枚保活 pin; 与租约同计数, 有 pin 在场流不关。
    pub fn pin(self: &Arc<Self>) -> Result<ResourcePin, ResourceError> {
        self.enter(LeaseKind::Keepalive)?;
        let concrete = Arc::clone(self);
        let owner: Arc<dyn LeaseRelease> = concrete;
        Ok(ResourcePin { owner })
    }

    /// 当前持流租约数 (不含 pin)。
    pub fn holders(&self) -> usize {
        self.lock_state().holders
    }

    /// 当前保活 pin 数。
    pub fn pins(&self) -> usize {
        self.lock_state().pins
    }

    /// 流是否开启。
    pub fn is_open(&self) -> bool {
        self.lock_state().open
    }

    /// 是否已关停。
    pub fn is_shutdown(&self) -> bool {
        self.lock_state().shutdown
    }

    /// 快照即读: 直接返回当前帧 (最后值 + 挂着的失败帧), 不回放历史。
    pub fn snapshot(&self) -> ResourceFrame<V> {
        self.lock_state().frame.clone()
    }

    /// 订阅更新: watch 语义 —— 只见订阅后的变化与当前值, 错过的帧不回放。
    pub fn subscribe(&self) -> watch::Receiver<ResourceFrame<V>> {
        self.updates.subscribe()
    }

    /// 发一帧成功值 (自清失败帧); 流未开或已关停时拒绝。
    pub fn publish(&self, value: V) -> Result<(), ResourceError> {
        let mut state = self.lock_state();
        self.ensure_accepting(&state)?;
        state.frame = ResourceFrame {
            value: Some(value),
            failure: None,
        };
        self.updates.send_replace(state.frame.clone());
        Ok(())
    }

    /// 失败即帧: 错误作为正式一帧挂在最后值旁, 下一成功帧自清;
    /// 流未开或已关停时拒绝。
    pub fn publish_failure(&self, failure: impl Into<String>) -> Result<(), ResourceError> {
        let mut state = self.lock_state();
        self.ensure_accepting(&state)?;
        state.frame.failure = Some(failure.into());
        self.updates.send_replace(state.frame.clone());
        Ok(())
    }

    /// 关停: 在流上时触发一次关停钩子; 此后无钩子、无更新、无新租约。
    pub fn shutdown(&self) {
        let mut state = self.lock_state();
        if state.shutdown {
            return;
        }
        state.shutdown = true;
        self.close_stream(&mut state);
    }

    fn enter(&self, kind: LeaseKind) -> Result<(), ResourceError> {
        let mut state = self.lock_state();
        if state.shutdown {
            return Err(ResourceError::Shutdown(state.name.clone()));
        }
        match kind {
            LeaseKind::Holder => state.holders += 1,
            LeaseKind::Keepalive => state.pins += 1,
        }
        if !state.open {
            if let Err(reason) = (state.hooks.on_open)() {
                match kind {
                    LeaseKind::Holder => state.holders -= 1,
                    LeaseKind::Keepalive => state.pins -= 1,
                }
                return Err(ResourceError::OpenFailed {
                    name: state.name.clone(),
                    reason,
                });
            }
            state.open = true;
        }
        Ok(())
    }

    fn ensure_accepting(&self, state: &SurfaceState<V>) -> Result<(), ResourceError> {
        if state.shutdown {
            return Err(ResourceError::Shutdown(state.name.clone()));
        }
        if !state.open {
            return Err(ResourceError::StreamNotOpen(state.name.clone()));
        }
        Ok(())
    }

    fn close_stream(&self, state: &mut SurfaceState<V>) {
        if !state.open {
            return;
        }
        // 关停是权威动作 (即便仍有持有者也即刻关流); 此后 open=false,
        // 事后释放不会再补调关停钩子 —— 关停后无幽灵回调。
        if state.shutdown || (state.holders == 0 && state.pins == 0) {
            (state.hooks.on_close)();
            state.open = false;
        }
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, SurfaceState<V>> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl<V: Clone + Send + Sync + 'static> LeaseRelease for ResourceRegistry<V> {
    fn release_lease(&self, kind: LeaseKind) {
        let mut state = self.lock_state();
        match kind {
            LeaseKind::Holder => state.holders = state.holders.saturating_sub(1),
            LeaseKind::Keepalive => state.pins = state.pins.saturating_sub(1),
        }
        self.close_stream(&mut state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// 计数钩子: 记录开/关次数, 可注入开流失败。
    fn counting_hooks(
        opens: Arc<AtomicUsize>,
        closes: Arc<AtomicUsize>,
    ) -> (Arc<AtomicUsize>, Arc<AtomicUsize>, ResourceHooks) {
        let open_hook = Arc::clone(&opens);
        let close_hook = Arc::clone(&closes);
        let hooks = ResourceHooks::new(
            Arc::new(move || {
                open_hook.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }),
            Arc::new(move || {
                close_hook.fetch_add(1, Ordering::SeqCst);
            }),
        );
        (opens, closes, hooks)
    }

    fn counters() -> (Arc<AtomicUsize>, Arc<AtomicUsize>) {
        (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)))
    }

    #[test]
    fn first_holder_opens_and_last_holder_closes() {
        let (opens, closes) = counters();
        let (_, _, hooks) = counting_hooks(opens.clone(), closes.clone());
        let registry = ResourceRegistry::<String>::new("surface_a", hooks);

        assert!(!registry.is_open(), "无持有者时流不开");
        let first = registry.acquire().expect("lease");
        assert!(registry.is_open(), "首持有者开流");
        assert_eq!(opens.load(Ordering::SeqCst), 1);

        let second = registry.acquire().expect("lease");
        assert_eq!(opens.load(Ordering::SeqCst), 1, "共享不重复开流");

        drop(first);
        assert!(registry.is_open(), "仍有持有者, 流不关");
        assert_eq!(closes.load(Ordering::SeqCst), 0);

        drop(second);
        assert!(!registry.is_open(), "末持有者关停");
        assert_eq!(closes.load(Ordering::SeqCst), 1);

        // 再次持有 = 重新开流 (开/关可循环)。
        let third = registry.acquire().expect("lease");
        assert_eq!(opens.load(Ordering::SeqCst), 2);
        drop(third);
        assert_eq!(closes.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn leases_and_pins_track_the_holder_count_and_pin_keeps_alive() {
        let (opens, closes) = counters();
        let (_, _, hooks) = counting_hooks(opens.clone(), closes.clone());
        let registry = ResourceRegistry::<u32>::new("surface_b", hooks);

        let lease_a = registry.acquire().expect("lease");
        let lease_b = registry.acquire().expect("lease");
        let pin = registry.pin().expect("pin");
        assert_eq!(registry.holders(), 2);
        assert_eq!(registry.pins(), 1);

        drop(lease_a);
        drop(lease_b);
        assert_eq!(registry.holders(), 0);
        assert!(registry.is_open(), "pin 保活: 即便租约清零流也不关");
        assert_eq!(closes.load(Ordering::SeqCst), 0);

        drop(pin);
        assert_eq!(registry.pins(), 0);
        assert!(!registry.is_open(), "最后一个 pin 释放即关停");
        assert_eq!(closes.load(Ordering::SeqCst), 1);

        // 单独一枚 pin 也开流。
        let pin_only = registry.pin().expect("pin");
        assert!(registry.is_open());
        assert_eq!(opens.load(Ordering::SeqCst), 2);
        drop(pin_only);
        assert_eq!(closes.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn late_subscriber_reads_the_snapshot_without_replay() {
        let registry = ResourceRegistry::<String>::new("surface_c", ResourceHooks::inert());
        let _lease = registry.acquire().expect("lease");

        registry.publish("v1".to_string()).expect("publish");
        registry.publish("v2".to_string()).expect("publish");
        registry.publish("v3".to_string()).expect("publish");

        // 后订阅者: 快照即读最新, 不回放 v1/v2。
        let snapshot = registry.snapshot();
        assert_eq!(snapshot.value.as_deref(), Some("v3"), "快照即读最新");

        let mut updates = registry.subscribe();
        assert!(
            !updates.has_changed().expect("watch alive"),
            "订阅即见当前值, 错过的帧不排队"
        );

        // 订阅后只送新变化, 且多帧连发也只看到合并后的最新帧。
        registry.publish("v4".to_string()).expect("publish");
        registry.publish("v5".to_string()).expect("publish");
        assert!(updates.has_changed().expect("watch alive"));
        let seen = updates.borrow_and_update().clone();
        assert_eq!(seen.value.as_deref(), Some("v5"), "错过更新不回放");
        assert!(!updates.has_changed().expect("watch alive"));
    }

    #[test]
    fn failure_frame_rides_beside_the_last_value_and_self_clears() {
        let registry = ResourceRegistry::<String>::new("surface_d", ResourceHooks::inert());
        let _lease = registry.acquire().expect("lease");

        registry.publish("v1".to_string()).expect("publish");
        registry
            .publish_failure("upstream stream error")
            .expect("failure frame");

        let frame = registry.snapshot();
        assert_eq!(frame.value.as_deref(), Some("v1"), "失败帧不清最后值");
        assert_eq!(
            frame.failure.as_deref(),
            Some("upstream stream error"),
            "错误作为正式一帧挂在最后值旁"
        );
        assert!(frame.has_failure());

        // 下一成功帧自清。
        registry.publish("v2".to_string()).expect("publish");
        let frame = registry.snapshot();
        assert_eq!(frame.value.as_deref(), Some("v2"));
        assert!(!frame.has_failure(), "下一成功帧自清失败帧");
    }

    #[test]
    fn shutdown_stops_hooks_updates_and_new_leases() {
        let (opens, closes) = counters();
        let (_, _, hooks) = counting_hooks(opens.clone(), closes.clone());
        let registry = ResourceRegistry::<String>::new("surface_e", hooks);

        let lease = registry.acquire().expect("lease");
        let mut updates = registry.subscribe();
        registry.publish("v1".to_string()).expect("publish");
        let _seen = updates.borrow_and_update().clone(); // 订阅者已见 v1

        // 提前释放再关停: 关停钩子恰好一次。
        drop(lease);
        assert_eq!(closes.load(Ordering::SeqCst), 1);
        registry.shutdown();
        assert!(registry.is_shutdown());
        assert_eq!(closes.load(Ordering::SeqCst), 1, "未在流上时关停不补调钩子");

        // 关停后: 无新租约、无更新、无钩子。
        assert!(matches!(
            registry.acquire(),
            Err(ResourceError::Shutdown(_))
        ));
        assert!(matches!(registry.pin(), Err(ResourceError::Shutdown(_))));
        assert!(matches!(
            registry.publish("v2".to_string()),
            Err(ResourceError::Shutdown(_))
        ));
        assert!(matches!(
            registry.publish_failure("late failure"),
            Err(ResourceError::Shutdown(_))
        ));
        assert_eq!(opens.load(Ordering::SeqCst), 1, "关停后不得再开流");
        assert!(
            !updates.has_changed().expect("watch alive"),
            "关停后无幽灵更新"
        );
        assert_eq!(registry.snapshot().value.as_deref(), Some("v1"));

        // 在流上关停: 钩子一次且事后释放不补调 (无幽灵回调)。
        let lease_again = registry.acquire();
        assert!(lease_again.is_err(), "关停后不再发放租约");
    }

    #[test]
    fn shutdown_while_open_closes_once_and_late_releases_stay_silent() {
        let (opens, closes) = counters();
        let (_, _, hooks) = counting_hooks(opens.clone(), closes.clone());
        let registry = ResourceRegistry::<String>::new("surface_f", hooks);

        let lease = registry.acquire().expect("lease");
        let pin = registry.pin().expect("pin");
        registry.shutdown();
        assert!(!registry.is_open());
        assert_eq!(closes.load(Ordering::SeqCst), 1, "关停即关流一次");

        // 幽灵回调检查: 事后释放的租约/pin 不得再触发关停钩子。
        drop(lease);
        drop(pin);
        assert_eq!(closes.load(Ordering::SeqCst), 1, "关停后无幽灵回调");
        assert_eq!(opens.load(Ordering::SeqCst), 1);

        // 流未开时发布也是显式拒绝 (不静默吞)。
        let closed_registry = ResourceRegistry::<String>::new("surface_g", ResourceHooks::inert());
        assert!(matches!(
            closed_registry.publish("v".to_string()),
            Err(ResourceError::StreamNotOpen(_))
        ));
    }

    #[test]
    fn open_hook_refusal_fails_closed_without_leaking_a_holder() {
        let hooks = ResourceHooks::new(
            Arc::new(|| Err("stream refused".to_string())),
            Arc::new(|| ()),
        );
        let registry = ResourceRegistry::<String>::new("surface_h", hooks);
        match registry.acquire() {
            Err(ResourceError::OpenFailed { name, reason }) => {
                assert_eq!(name, "surface_h");
                assert_eq!(reason, "stream refused");
            }
            other => panic!("expected OpenFailed, got {other:?}"),
        }
        assert_eq!(registry.holders(), 0, "拒绝发放不得漏计持有者");
        assert!(!registry.is_open());
    }
}
