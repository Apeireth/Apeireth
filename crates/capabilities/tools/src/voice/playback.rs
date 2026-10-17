//! 受控播放面 —— 同时只播一条, 排队/打断可配, 真停清理零僵尸。
//!
//! # 语义 (命名即承诺)
//!
//! - **独占**: 全进程同一时刻只有一条语句在播 ([`PlaybackController`] 的
//!   唯一在播槽);
//! - **排队串行 = 缺省** ([`PlaybackPolicy`], 设计裁定 R1⑯:
//!   [`PlaybackPolicy::QueueSerial`] = 新话排队, 旧话播完才开播 (串行),
//!   排队话帧最多 [`SPEECH_QUEUE_MAX`] 条, 满则**拒新 + 错误帧**
//!   (不丢最旧, 不静默);
//!   [`PlaybackPolicy::InterruptPrevious`] = 新话先真停旧话再开播 (显式可配);
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

/// 播放排队旋钮 (`APEIRETH_VOICE_INTERRUPT`): `0`(缺省) = 排队串行,
/// `1` = 新话打断旧话。坏值拒绝解析 (fail-closed: 不猜意图)。
pub const VOICE_INTERRUPT_ENV: &str = "APEIRETH_VOICE_INTERRUPT";

/// 话帧排队上限 (排队档): 等待开播的排队话帧最多 3 条 (设计裁定 R1⑯:
/// 排队+上限 3 条); 满则**拒新 + 错误帧** (钉「拒新」不「丢最旧」, 不静默)。
pub const SPEECH_QUEUE_MAX: usize = 3;

/// 收尾轮询间隔 (毫秒; 真停延迟的上界)。
pub const PLAYBACK_POLL_MS: u64 = 25;

/// 新话与旧话的关系 (可配面)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackPolicy {
    /// 排队串行 (缺省, 设计裁定 R1⑯): 旧话播完新话才开播 (同时仍只播一条),
    /// 排队话帧上限 [`SPEECH_QUEUE_MAX`] 条, 满则拒新。
    #[default]
    QueueSerial,
    /// 新话打断旧话 (`APEIRETH_VOICE_INTERRUPT=1` 显式可配)。
    InterruptPrevious,
}

impl PlaybackPolicy {
    /// 稳定标签 (`interrupt` / `queue`)。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InterruptPrevious => "interrupt",
            Self::QueueSerial => "queue",
        }
    }

    /// 解析旋钮值: `0` = 排队, `1` = 打断; 其余拒绝 (fail-closed)。
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw.trim() {
            "1" => Ok(Self::InterruptPrevious),
            "0" => Ok(Self::QueueSerial),
            other => Err(format!(
                "拒绝配置: {VOICE_INTERRUPT_ENV} 只接受 1 (打断) 或 0 (排队), got {other:?}"
            )),
        }
    }

    /// 从 env 解析 (缺省 = 排队串行档; 坏值报错, 不静默回退)。
    pub fn from_env() -> Result<Self, String> {
        let raw = std::env::var(VOICE_INTERRUPT_ENV).unwrap_or_default();
        if raw.trim().is_empty() {
            return Ok(Self::QueueSerial);
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
    /// 排队话帧已满 ([`SPEECH_QUEUE_MAX`] 条): 拒新即帧 (不丢最旧)。
    QueueFull,
    /// 引擎开播失败 (错误即帧的真因)。
    Start(String),
}

/// 排队票据: 排队档等待开播期间占一个队位; Drop = 归还队位 (取消不占位)。
pub struct QueueSlot {
    controller: Arc<PlaybackController>,
}

impl Drop for QueueSlot {
    fn drop(&mut self) {
        self.controller.release_queue_slot();
    }
}

/// 诊断显示 (只用于调试输出 / 断言消息; 不暴露控制器内部)。
///
/// 跨面最小修 (2026-10-17 性格锁定批报备): 本文件单测用 `expect_err` 需要
/// `Debug` 上界, 此前缺实现导致 `apeireth-tools-canonical` lib 测试目标编译
/// 不过 —— 补纯展示实现, 零语义变化。
impl std::fmt::Debug for QueueSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("QueueSlot { .. }")
    }
}

/// 受控播放器: 独占槽 + 可配的打断/排队语义 + Drop 真停。
pub struct PlaybackController {
    engine: Arc<dyn SynthEngine>,
    policy: PlaybackPolicy,
    active: Mutex<Option<ActiveSlot>>,
    queued: Mutex<usize>,
    next_id: std::sync::atomic::AtomicU64,
}

impl PlaybackController {
    /// 绑定合成引擎与排队策略。
    pub fn new(engine: Arc<dyn SynthEngine>, policy: PlaybackPolicy) -> Self {
        Self {
            engine,
            policy,
            active: Mutex::new(None),
            queued: Mutex::new(0),
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

    /// 当前排队话帧数 (等待开播的; 在播句不占队位)。
    pub fn queue_len(&self) -> usize {
        *self
            .queued
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// 订一个队位 (排队档等待开播用): 队满 ([`SPEECH_QUEUE_MAX`] 条) = 拒新
    /// [`TryBeginError::QueueFull`] (错误即帧, 不丢最旧)。
    ///
    /// 票据 ([`QueueSlot`]) Drop = 归还队位: 取消/失败的排队话帧不占位。
    pub fn try_reserve_queue(self: &Arc<Self>) -> Result<QueueSlot, TryBeginError> {
        let mut queued = self
            .queued
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if *queued >= SPEECH_QUEUE_MAX {
            return Err(TryBeginError::QueueFull);
        }
        *queued += 1;
        Ok(QueueSlot {
            controller: Arc::clone(self),
        })
    }

    /// 归还队位 (QueueSlot Drop 路径, 幂等不越界)。
    fn release_queue_slot(&self) {
        let mut queued = self
            .queued
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *queued = queued.saturating_sub(1);
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

/// 诊断显示 (同 [`QueueSlot`]: 跨面最小修补 `Debug` 上界, 零语义变化)。
impl std::fmt::Debug for SpeechTicket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpeechTicket")
            .field("id", &self.id)
            .finish_non_exhaustive()
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
/// 排队档 ([`PlaybackPolicy::QueueSerial`]) 先订队位: 队满 (等待中的话帧
/// 已达 [`SPEECH_QUEUE_MAX`] 条) = [`TryBeginError::QueueFull`] 拒新即帧
/// (不丢最旧); 打断档无队 (新话即打断, 不占队位)。
///
/// [`PlaybackPolicy::QueueSerial`] 的等待是可取消的: 调用方 future 被丢时
/// 本调用还没开播, 旧话不受影响 (取消的新话不打断旧话), 队位随票据归还。
pub async fn begin_speech(
    controller: &Arc<PlaybackController>,
    request: &SynthRequest,
) -> Result<SpeechTicket, TryBeginError> {
    let _queued = match controller.policy() {
        PlaybackPolicy::QueueSerial => Some(controller.try_reserve_queue()?),
        PlaybackPolicy::InterruptPrevious => None,
    };
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

    /// 设计裁定 R1⑯: 缺省排队串行 (APEIRETH_VOICE_INTERRUPT 缺省 0),
    /// 排队上限 3 条; 显式旋钮仍可回到打断档 (旧行为兼容)。
    #[test]
    fn default_policy_is_queue_serial_and_explicit_knob_still_picks_interrupt() {
        assert_eq!(
            PlaybackPolicy::default(),
            PlaybackPolicy::QueueSerial,
            "缺省 = 排队串行"
        );
        assert_eq!(
            PlaybackPolicy::parse("0").expect("0"),
            PlaybackPolicy::QueueSerial
        );
        assert_eq!(
            PlaybackPolicy::parse("1").expect("1"),
            PlaybackPolicy::InterruptPrevious,
            "旧行为兼容: 显式 1 = 新话打断旧话"
        );
        assert!(PlaybackPolicy::parse("2").is_err(), "坏值拒绝解析");
        assert_eq!(SPEECH_QUEUE_MAX, 3, "排队话帧上限 = 3 条");
    }

    /// 队满 = 拒新 + 错误帧 (钉「拒新」不「丢最旧」): 溢出话帧拿不到队位,
    /// 在播旧话照旧, 队位归还后恢复受理。
    #[tokio::test]
    async fn queue_cap_refuses_the_overflow_frame_and_keeps_the_oldest() {
        let engine = Arc::new(MockSynthEngine::new(MockBehavior::Hold));
        let state = engine.state();
        let controller = Arc::new(PlaybackController::new(engine, PlaybackPolicy::QueueSerial));
        let playing = controller.try_begin(&request("在播")).expect("playing");
        // 手工占满 3 个排队位 (确定性)。
        let slots: Vec<QueueSlot> = (0..SPEECH_QUEUE_MAX)
            .map(|_| controller.try_reserve_queue().expect("queue slot"))
            .collect();
        assert_eq!(controller.queue_len(), SPEECH_QUEUE_MAX, "3 条排队满");

        // 第 4 条排队 = 拒新即帧 (不丢最旧)。
        let refused = controller
            .try_reserve_queue()
            .expect_err("queue full refuses the new frame");
        assert_eq!(refused, TryBeginError::QueueFull);
        let Err(refused) = begin_speech(&controller, &request("溢出")).await else {
            panic!("begin_speech must refuse when the queue is full")
        };
        assert_eq!(refused, TryBeginError::QueueFull);
        assert_eq!(state.history_len(), 1, "拒新: 溢出话帧不开播");
        assert!(playing.playback().is_running(), "拒新不丢最旧 (在播照旧)");

        // 队位随票据归还 → 恢复受理。
        drop(slots);
        assert_eq!(controller.queue_len(), 0, "队位归还");
        drop(playing);
    }
}
