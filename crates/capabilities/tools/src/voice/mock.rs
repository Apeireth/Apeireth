//! 假引擎面注入 (测试替身) —— [`SynthEngine`] 执行面的可钉死实现。
//!
//! 「合成调用真 SAPI 或假引擎面注入」两条验收路里的后者: 行为可钉死
//! (立即播完 / 挂住等停 / 合成失败), 记录每一次收到的合成请求, 并用
//! 计数器钉住两个硬语义: **同时只播一条** (`max_running == 1`) 与
//! **句柄零泄漏** (`created == dropped`)。
//!
//! 这是设计里的注入面 (合成执行面是 trait), 不是产品工具: 生产面永远是
//! [`crate::voice::synth::system_synth_engine`]。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::synth::{EngineStatus, PlaybackEnd, SpeechPlayback, SynthEngine, SynthRequest};

/// 假引擎的行为档 (测试按拍选档)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MockBehavior {
    /// 每次合成在首次收尾读取时即报播完 (正常收尾拍)。
    AutoComplete,
    /// 挂住: 不自己收尾, 等 [`MockPlayback::complete`] 或 [`SpeechPlayback::stop`]
    /// (打断 / 排队 / 取消拍)。
    Hold,
    /// 合成失败 (错误即帧拍): 终态带真因。
    Fail {
        /// 失败真因 (如实进错误帧)。
        reason: String,
    },
}

/// 假引擎的共享计数面 (断言「同时只播一条」与「句柄零泄漏」)。
#[derive(Debug, Default)]
pub struct MockState {
    created: AtomicUsize,
    dropped: AtomicUsize,
    running: AtomicUsize,
    max_running: AtomicUsize,
    stop_calls: AtomicUsize,
    requests: Mutex<Vec<SynthRequest>>,
    history: Mutex<Vec<std::sync::Weak<MockPlayback>>>,
}

impl MockState {
    /// 已造播放句柄数。
    pub fn created(&self) -> usize {
        self.created.load(Ordering::SeqCst)
    }

    /// 已 Drop 播放句柄数 (Drop 契约: Drop 即真停)。
    pub fn dropped(&self) -> usize {
        self.dropped.load(Ordering::SeqCst)
    }

    /// 现存活句柄数 (0 = 无泄漏)。
    pub fn live(&self) -> usize {
        self.created().saturating_sub(self.dropped())
    }

    /// 观测到的最大同时在播数 (「同时只播一条」断言面)。
    pub fn max_running(&self) -> usize {
        self.max_running.load(Ordering::SeqCst)
    }

    /// `stop` 被真实调用的次数 (真停断言面)。
    pub fn stop_calls(&self) -> usize {
        self.stop_calls.load(Ordering::SeqCst)
    }

    /// 收到的合成请求 (按到达顺序; 参数注入断言面)。
    pub fn requests(&self) -> Vec<SynthRequest> {
        self.requests
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// 第 `index` 条播放句柄 (仍存活时)。
    pub fn playback(&self, index: usize) -> Option<Arc<MockPlayback>> {
        self.history
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(index)
            .and_then(std::sync::Weak::upgrade)
    }

    /// 播放句柄历史条数 (含已 Drop 的弱引用位)。
    pub fn history_len(&self) -> usize {
        self.history
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
    }

    fn record_request(&self, request: &SynthRequest) {
        self.requests
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(request.clone());
        self.created.fetch_add(1, Ordering::SeqCst);
        let running = self.running.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_running.fetch_max(running, Ordering::SeqCst);
    }

    /// 「真停」计数: 仍在播 → 被停的转移次数 (Drop 清理同计)。
    fn record_stop(&self) {
        self.stop_calls.fetch_add(1, Ordering::SeqCst);
    }

    fn record_end(&self) {
        self.running.fetch_sub(1, Ordering::SeqCst);
    }

    fn record_drop(&self) {
        self.dropped.fetch_add(1, Ordering::SeqCst);
    }
}

/// 假合成引擎 (记录请求 + 计数句柄 + 行为可钉死)。
pub struct MockSynthEngine {
    behavior: MockBehavior,
    voices: Result<Vec<String>, String>,
    state: Arc<MockState>,
}

impl MockSynthEngine {
    /// 默认已装嗓 = `["mock-voice"]`。
    pub fn new(behavior: MockBehavior) -> Self {
        Self {
            behavior,
            voices: Ok(vec!["mock-voice".to_string()]),
            state: Arc::new(MockState::default()),
        }
    }

    /// 已装嗓清单显式给定 (`[]` = 无嗓拍)。
    #[must_use]
    pub fn with_voices(mut self, voices: Vec<String>) -> Self {
        self.voices = Ok(voices);
        self
    }

    /// 已装嗓清单读取失败 (引擎缺失拍, 真因如实)。
    #[must_use]
    pub fn with_voices_error(mut self, reason: impl Into<String>) -> Self {
        self.voices = Err(reason.into());
        self
    }

    /// 共享计数面。
    pub fn state(&self) -> Arc<MockState> {
        Arc::clone(&self.state)
    }

    /// 记录一个播放句柄进历史 (测试可取句柄做 `complete` / `stop`)。
    fn new_playback(&self, request: &SynthRequest) -> Arc<MockPlayback> {
        self.state.record_request(request);
        let playback = Arc::new(MockPlayback {
            engine_id: "mock",
            request: request.clone(),
            behavior: self.behavior.clone(),
            started: Instant::now(),
            ended: Mutex::new(None),
            end_recorded: AtomicUsize::new(0),
            state: Arc::clone(&self.state),
        });
        self.state
            .history
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(Arc::downgrade(&playback));
        playback
    }
}

impl SynthEngine for MockSynthEngine {
    fn engine_id(&self) -> &'static str {
        "mock"
    }

    fn status(&self) -> EngineStatus {
        match &self.voices {
            Ok(_) => EngineStatus {
                available: true,
                reason: "假引擎面 (测试替身, 非产品发声)".to_string(),
            },
            Err(reason) => EngineStatus {
                available: false,
                reason: reason.clone(),
            },
        }
    }

    fn installed_voices(&self) -> Result<Vec<String>, String> {
        self.voices.clone()
    }

    fn speak(&self, request: &SynthRequest) -> Result<Arc<dyn SpeechPlayback>, String> {
        Ok(self.new_playback(request))
    }
}

/// 假播放句柄 (Drop = 真停, 与 [`SpeechPlayback`] 的 Drop 契约同款)。
pub struct MockPlayback {
    engine_id: &'static str,
    request: SynthRequest,
    behavior: MockBehavior,
    started: Instant,
    ended: Mutex<Option<PlaybackEnd>>,
    /// 0 = 未计入收尾; 1 = 已计入 (收尾计数只准一次)。
    end_recorded: AtomicUsize,
    state: Arc<MockState>,
}

impl MockPlayback {
    /// 这条播放对应的合成请求 (参数注入断言面)。
    pub fn request(&self) -> &SynthRequest {
        &self.request
    }

    /// 测试收尾拍: 主动报播完 (Hold 档专用)。
    pub fn complete(&self) {
        self.finish(PlaybackEnd::Completed {
            duration_ms: self.elapsed_ms(),
        });
    }

    /// 统一收尾 (计数只准一次); 返回是否发生了「在播 → 终态」的转移。
    fn finish(&self, end: PlaybackEnd) -> bool {
        let mut ended = self
            .ended
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if ended.is_some() {
            return false;
        }
        *ended = Some(end);
        if self.end_recorded.fetch_add(1, Ordering::SeqCst) == 0 {
            self.state.record_end();
        }
        true
    }
}

impl SpeechPlayback for MockPlayback {
    fn engine_id(&self) -> &'static str {
        self.engine_id
    }

    fn is_running(&self) -> bool {
        self.ended
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_none()
    }

    fn poll_end(&self) -> Option<PlaybackEnd> {
        if let Some(end) = &*self
            .ended
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
        {
            return Some(end.clone());
        }
        match &self.behavior {
            MockBehavior::Hold => None,
            MockBehavior::AutoComplete => {
                self.complete();
                self.ended
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .clone()
            }
            MockBehavior::Fail { reason } => {
                let end = PlaybackEnd::Failed {
                    duration_ms: self.elapsed_ms(),
                    reason: reason.clone(),
                };
                self.finish(end.clone());
                Some(end)
            }
        }
    }

    fn stop(&self) {
        let end = PlaybackEnd::Stopped {
            duration_ms: self.elapsed_ms(),
        };
        if self.finish(end) {
            self.state.record_stop();
        }
    }

    fn elapsed_ms(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }
}

impl Drop for MockPlayback {
    /// Drop 契约 (真停清理): 句柄没了, 声音也必须停。
    fn drop(&mut self) {
        self.stop();
        self.state.record_drop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> SynthRequest {
        SynthRequest {
            text: "测试".to_string(),
            voice: None,
            rate: 0,
            volume: 100,
        }
    }

    #[test]
    fn mock_engine_records_requests_and_counts_handles() {
        let engine = MockSynthEngine::new(MockBehavior::AutoComplete);
        let playback = engine.speak(&request()).expect("speak");
        assert_eq!(engine.state().requests(), vec![request()]);
        assert!(playback.is_running() || playback.poll_end().is_some());
        let _ = playback.poll_end();
        drop(playback);
        assert_eq!(engine.state().live(), 0, "句柄零泄漏");
        assert_eq!(engine.state().max_running(), 1, "同时只播一条");
    }

    #[test]
    fn hold_behavior_stops_only_when_told() {
        let engine = MockSynthEngine::new(MockBehavior::Hold);
        let playback = engine.speak(&request()).expect("speak");
        assert!(playback.poll_end().is_none(), "Hold 档不自收尾");
        playback.stop();
        assert!(
            matches!(playback.poll_end(), Some(PlaybackEnd::Stopped { .. })),
            "真停后终态如实"
        );
        assert_eq!(engine.state().stop_calls(), 1, "在播被真停恰一次");
    }
}
