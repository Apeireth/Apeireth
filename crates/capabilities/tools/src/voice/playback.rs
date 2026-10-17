//! 受控播放面 —— 同时只播一条, 新话打断旧话可配, 真停清理零僵尸。
//!
//! # 语义 (命名即承诺)
//!
//! - **独占**: 全进程同一时刻只有一条语句在播 ([`PlaybackController`] 的
//!   唯一在播槽);
//! - **新话打断旧话 = 可配** ([`PlaybackPolicy`]):
//!   [`PlaybackPolicy::InterruptPrevious`] (缺省) = 新话先真停旧话再开播;
//!   [`PlaybackPolicy::QueueSerial`] = 新话排队, 旧话播完才开播 (串行);
//! - **真停清理**: 播放票据 [`SpeechTicket`] 的 `Drop` = 真停 + 出槽 ——
//!   正常收尾 / 新话打断 / 调用被取消 (超时、回合中断) 三条路都留下
//!   **零活句柄** (配合 [`crate::voice::synth::SpeechPlayback`] 的 Drop 契约,
//!   杜绝僵尸播放)。
//!
//! 轮询收尾 ([`wait_playback`]) 是可取消的异步等待: 调用方 future 被丢,
//! 票据即 Drop, 播放即停 —— 「取消 = 不再出声」而不是「帧说停了声还在」。

use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::synth::{PlaybackEnd, SpeechPlayback, SynthEngine, SynthRequest};

/// 播放排队旋钮 (`APEIRETH_VOICE_INTERRUPT`): `1`(缺省) = 新话打断旧话,
/// `0` = 排队串行。坏值拒绝解析 (fail-closed: 不猜意图)。
pub const VOICE_INTERRUPT_ENV: &str = "APEIRETH_VOICE_INTERRUPT";

/// 收尾轮询间隔 (毫秒; 真停延迟的上界)。
pub const PLAYBACK_POLL_MS: u64 = 25;

/// 新话与旧话的关系 (可配面)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackPolicy {
    /// 新话打断旧话 (缺省: 说最新想说的)。
    #[default]
    InterruptPrevious,
    /// 排队串行: 旧话播完新话才开播 (同时仍只播一条)。
    QueueSerial,
}

impl PlaybackPolicy {
    /// 稳定标签 (`interrupt` / `queue`)。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InterruptPrevious => "interrupt",
            Self::QueueSerial => "queue",
        }
    }

    /// 解析旋钮值: `1` = 打断, `0` = 排队; 其余拒绝 (fail-closed)。
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw.trim() {
            "1" => Ok(Self::InterruptPrevious),
            "0" => Ok(Self::QueueSerial),
            other => Err(format!(
                "拒绝配置: {VOICE_INTERRUPT_ENV} 只接受 1 (打断) 或 0 (排队), got {other:?}"
            )),
        }
    }

    /// 从 env 解析 (缺省 = 打断档; 坏值报错, 不静默回退)。
    pub fn from_env() -> Result<Self, String> {
        let raw = std::env::var(VOICE_INTERRUPT_ENV).unwrap_or_default();
        if raw.trim().is_empty() {
            return Ok(Self::InterruptPrevious);
        }
        Self::parse(&raw).map_err(|error| format!("{VOICE_INTERRUPT_ENV}: {error}"))
    }
}

/// 在播槽 (全进程唯一)。
struct ActiveSlot {
    id: u64,
    playback: Arc<dyn SpeechPlayback>,
}

/// `try_begin` 的失败分类 (排队档的忙是可等待状态, 不是错误)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TryBeginError {
    /// 已有语句在播 (排队档: 等它收尾再试)。
    Busy,
    /// 引擎开播失败 (错误即帧的真因)。
    Start(String),
}

/// 受控播放器: 独占槽 + 可配的打断/排队语义 + Drop 真停。
pub struct PlaybackController {
    engine: Arc<dyn SynthEngine>,
    policy: PlaybackPolicy,
    active: Mutex<Option<ActiveSlot>>,
    next_id: std::sync::atomic::AtomicU64,
}

impl PlaybackController {
    /// 绑定合成引擎与排队策略。
    pub fn new(engine: Arc<dyn SynthEngine>, policy: PlaybackPolicy) -> Self {
        Self {
            engine,
            policy,
            active: Mutex::new(None),
            next_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// 生效的排队策略。
    pub const fn policy(&self) -> PlaybackPolicy {
        self.policy
    }

    /// 承载的合成引擎。
    pub fn engine(&self) -> &Arc<dyn SynthEngine> {
        &self.engine
    }

    /// 是否有语句在播。
    pub fn is_busy(&self) -> bool {
        self.active
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_some()
    }

    /// 开播一次合成即播 (不等待收尾)。
    ///
    /// - [`PlaybackPolicy::InterruptPrevious`]: 先真停在播旧话 (它的调用方
    ///   收到 `interrupted` 终态), 再开新话;
    /// - [`PlaybackPolicy::QueueSerial`]: 有话在播 = [`TryBeginError::Busy`]
    ///   (调用方等待重试, 保证串行)。
    ///
    /// 成功 = 独占槽登记 + 返回票据; 票据 Drop = 真停 + 出槽。
    pub fn try_begin(
        self: &Arc<Self>,
        request: &SynthRequest,
    ) -> Result<SpeechTicket, TryBeginError> {
        let mut active = self
            .active
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match (self.policy, active.as_ref()) {
            (PlaybackPolicy::QueueSerial, Some(_)) => return Err(TryBeginError::Busy),
            (PlaybackPolicy::InterruptPrevious, Some(slot)) => {
                slot.playback.stop();
            }
            (_, None) => {}
        }
        *active = None;
        let playback = self.engine.speak(request).map_err(TryBeginError::Start)?;
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        *active = Some(ActiveSlot {
            id,
            playback: Arc::clone(&playback),
        });
        Ok(SpeechTicket {
            controller: Arc::clone(self),
            id,
            playback,
        })
    }

    /// 真停当前在播语句 (外部停令面: 未来 `voice_stop` / 宿主停止键)。
    /// 返回是否真有在播语句被停。
    pub fn stop_active(&self) -> bool {
        let mut active = self
            .active
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match active.take() {
            Some(slot) => {
                slot.playback.stop();
                true
            }
            None => false,
        }
    }

    /// 票据收尾 (Drop 路径): 只清自己的槽, 幂等真停。
    fn release(&self, id: u64, playback: &Arc<dyn SpeechPlayback>) {
        let mut active = self
            .active
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if active.as_ref().is_some_and(|slot| slot.id == id) {
            *active = None;
        }
        playback.stop();
    }
}

impl Drop for PlaybackController {
    fn drop(&mut self) {
        self.stop_active();
    }
}

/// 一条在播语句的票据: Drop = 真停 + 出槽 (取消即闭嘴, 不留僵尸播放)。
pub struct SpeechTicket {
    controller: Arc<PlaybackController>,
    id: u64,
    playback: Arc<dyn SpeechPlayback>,
}

impl SpeechTicket {
    /// 底层播放句柄 (收尾读取 / 状态查询)。
    pub fn playback(&self) -> &Arc<dyn SpeechPlayback> {
        &self.playback
    }

    /// 承载引擎标识 (账本 / 输出帧字段)。
    pub fn engine_id(&self) -> &'static str {
        self.playback.engine_id()
    }
}

impl Drop for SpeechTicket {
    /// 真停清理: 调用被取消 (超时/回合中断) 时声音必须跟着停。
    fn drop(&mut self) {
        self.controller.release(self.id, &self.playback);
    }
}

/// 轮询等待一条在播语句收尾 (可取消: future 被丢 = 票据 Drop = 真停)。
pub async fn wait_playback(ticket: &SpeechTicket) -> PlaybackEnd {
    loop {
        if let Some(end) = ticket.playback().poll_end() {
            return end;
        }
        tokio::time::sleep(Duration::from_millis(PLAYBACK_POLL_MS)).await;
    }
}

/// 按策略开播一条语句并拿到票据 (排队档会等到槽空; 可取消)。
///
/// [`PlaybackPolicy::QueueSerial`] 的等待是可取消的: 调用方 future 被丢时
/// 本调用还没开播, 旧话不受影响 (取消的新话不打断旧话)。
pub async fn begin_speech(
    controller: &Arc<PlaybackController>,
    request: &SynthRequest,
) -> Result<SpeechTicket, TryBeginError> {
    loop {
        match controller.try_begin(request) {
            Ok(ticket) => return Ok(ticket),
            Err(TryBeginError::Busy) => {
                tokio::time::sleep(Duration::from_millis(PLAYBACK_POLL_MS)).await;
            }
            Err(error) => return Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::mock::{MockBehavior, MockSynthEngine};
    use super::*;

    fn request(text: &str) -> SynthRequest {
        SynthRequest {
            text: text.to_string(),
            voice: None,
            rate: 0,
            volume: 100,
        }
    }

    #[test]
    fn interrupt_policy_stops_the_previous_playback() {
        let engine = Arc::new(MockSynthEngine::new(MockBehavior::Hold));
        let state = engine.state();
        let controller = Arc::new(PlaybackController::new(
            engine,
            PlaybackPolicy::InterruptPrevious,
        ));
        let first = controller.try_begin(&request("旧话")).expect("first");
        assert!(controller.is_busy());
        let second = controller.try_begin(&request("新话")).expect("second");
        assert!(
            matches!(
                first.playback().poll_end(),
                Some(PlaybackEnd::Stopped { .. })
            ),
            "新话开播前旧话已被真停"
        );
        assert_eq!(state.max_running(), 1, "同时只播一条");
        drop(first);
        drop(second);
        assert_eq!(state.live(), 0, "句柄零泄漏");
    }

    #[test]
    fn queue_policy_is_serial_and_never_overlaps() {
        let engine = Arc::new(MockSynthEngine::new(MockBehavior::Hold));
        let state = engine.state();
        let controller = Arc::new(PlaybackController::new(engine, PlaybackPolicy::QueueSerial));
        let first = controller.try_begin(&request("旧话")).expect("first");
        assert!(
            matches!(
                controller.try_begin(&request("新话")),
                Err(TryBeginError::Busy)
            ),
            "排队档: 旧话在播时新话不许开播"
        );
        // 测试收尾拍 (假引擎的句柄控制面): 主动报播完。
        state.playback(0).expect("mock handle").complete();
        drop(first);
        let second = controller.try_begin(&request("新话")).expect("second");
        assert_eq!(state.max_running(), 1, "串行: 同时只播一条");
        drop(second);
        assert_eq!(state.live(), 0);
    }

    #[test]
    fn ticket_drop_stops_the_playback_and_clears_the_slot() {
        let engine = Arc::new(MockSynthEngine::new(MockBehavior::Hold));
        let state = engine.state();
        let controller = Arc::new(PlaybackController::new(
            engine,
            PlaybackPolicy::InterruptPrevious,
        ));
        let ticket = controller.try_begin(&request("一句")).expect("ticket");
        assert!(controller.is_busy());
        drop(ticket);
        assert!(!controller.is_busy(), "票据 Drop = 出槽");
        assert_eq!(state.stop_calls(), 1, "票据 Drop = 真停");
        assert_eq!(state.live(), 0, "句柄零泄漏");
    }

    #[tokio::test]
    async fn queue_wait_is_cancellable_without_touching_the_previous_speech() {
        let engine = Arc::new(MockSynthEngine::new(MockBehavior::Hold));
        let controller = Arc::new(PlaybackController::new(
            engine.clone(),
            PlaybackPolicy::QueueSerial,
        ));
        let first = controller.try_begin(&request("旧话")).expect("first");
        // 取消排队中的新话 (future 超时被丢): 旧话不受影响, 无新句柄。
        let waiting = tokio::time::timeout(
            std::time::Duration::from_millis(60),
            begin_speech(&controller, &request("新话")),
        )
        .await;
        assert!(
            waiting.is_err(),
            "排队中的新话在旧话播完前不许开播 (可取消)"
        );
        assert!(
            first.playback().is_running(),
            "取消的新话不打断旧话 (还没开播)"
        );
        assert_eq!(engine.state().history_len(), 1, "新话未开播");
        drop(first);
    }
}
