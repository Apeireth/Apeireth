//! R3 生产情绪源接线 (`VoiceSpeakConfig::with_emotion_reader`) 的端到端回归:
//! 组合根注入**真读取面** (读 presence/PAD 合成器的现值) —— 情绪→语气微调
//! 生效, 无情绪数据仍零变化 (fail-safe 朝原参数, 不猜情绪)。
//!
//! 覆盖: 读取面注入事实 (名册 `voice_emotion_tune` 行口径) / 读取面读到
//! presence/PAD 合成器**现值** / 微调端到端生效 (合成器状态 → 读取面 →
//! 冻结映射表 → 引擎请求) / 无情绪数据 = 原参数零变化。
//! `std::env` 是进程全局, 全部用例串在一把锁后。

use std::sync::{Arc, Mutex};

use apeireth_cli::{
    voice_speak_config_with_emotion_from_env, VoiceEmotionSource, ENABLE_VOICE_SPEAK_ENV,
};
use apeireth_core::kernel::{ApprovalId, CapabilityId, RequestId, SessionId, Timestamp, TraceId};
use apeireth_gateway::{EventBus, PresenceService};
use apeireth_runtime::canonical::{RuntimeEvent, RuntimeEventSink, TraceEvent};
use apeireth_tools_canonical::voice::mock::{MockBehavior, MockSynthEngine};
use apeireth_tools_canonical::voice::{VOICE_RATE_DEFAULT, VOICE_VOLUME_DEFAULT};
use apeireth_tools_canonical::{EmotionBand, SpeakScope, VoiceSpeakParams, VoiceSpeakTool};

static ENV_LOCK: Mutex<()> = Mutex::new(());

const GUARDED_KEYS: &[&str] = &[
    "APEIRETH_DATA_DIR",
    ENABLE_VOICE_SPEAK_ENV,
    "APEIRETH_VOICE_QUIET_HOURS",
    "APEIRETH_VOICE_SPEAK_SCOPE",
    "APEIRETH_VOICE_INTERRUPT",
];

struct EnvGuard {
    keys: &'static [&'static str],
    previous: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn guard() -> Self {
        let previous = GUARDED_KEYS
            .iter()
            .map(|key| (*key, std::env::var(key).ok()))
            .collect();
        Self {
            keys: GUARDED_KEYS,
            previous,
        }
    }

    fn clear_all(&self) {
        for key in self.keys {
            std::env::remove_var(key);
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, previous) in &self.previous {
            match previous {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 发声面测试基线: 旋钮开 + 数据目录落临时位 + 显式关窗 (勿扰窗口不吃宿主
/// 本地时间, 测试确定性)。
fn voice_env_guard() -> (EnvGuard, tempfile::TempDir) {
    let data_dir = tempfile::tempdir().expect("tempdir");
    let guard = EnvGuard::guard();
    guard.clear_all();
    std::env::set_var("APEIRETH_DATA_DIR", data_dir.path());
    std::env::set_var(ENABLE_VOICE_SPEAK_ENV, "1");
    std::env::set_var("APEIRETH_VOICE_QUIET_HOURS", "off");
    (guard, data_dir)
}

/// 一轮真回合过 presence/PAD 合成器 (真实事件口, 非直接摆状态): 召回命中
/// 抬愉悦 (情感残响), 工具/轮次抬唤起, 审批等待压愉悦。
fn drive_turn(
    presence: &Arc<PresenceService>,
    recall_hits: u32,
    other_calls: u32,
    approvals: u32,
    rounds: u32,
) {
    let sink: &dyn RuntimeEventSink = presence.as_ref();
    let session = SessionId::new();
    let trace = TraceId::new();
    sink.emit(RuntimeEvent::TurnStarted {
        session,
        request: RequestId::new(),
        trace,
    });
    for index in 0..recall_hits {
        sink.emit(RuntimeEvent::Trace {
            session,
            trace,
            at: Timestamp::now(),
            event: TraceEvent::CapabilityDispatched {
                capability: CapabilityId::new("tool.mcp.recall_memory").expect("capability id"),
                tool_call_id: format!("call_recall_{index}"),
                round: 1,
            },
        });
    }
    for index in 0..other_calls {
        sink.emit(RuntimeEvent::Trace {
            session,
            trace,
            at: Timestamp::now(),
            event: TraceEvent::CapabilityDispatched {
                capability: CapabilityId::new("tool.shell").expect("capability id"),
                tool_call_id: format!("call_tool_{index}"),
                round: 1,
            },
        });
    }
    for index in 0..approvals {
        sink.emit(RuntimeEvent::Trace {
            session,
            trace,
            at: Timestamp::now(),
            event: TraceEvent::ApprovalRequested {
                approval_id: ApprovalId::new(),
                capability: CapabilityId::new("tool.shell").expect("capability id"),
                tool_call_id: format!("call_approval_{index}"),
                round: 1,
            },
        });
    }
    sink.emit(RuntimeEvent::TurnCompleted {
        session,
        request: RequestId::new(),
        trace,
        rounds,
        served_by: CapabilityId::new("provider.fake").expect("capability id"),
    });
}

fn care_params(text: &str) -> VoiceSpeakParams {
    VoiceSpeakParams {
        text: text.to_string(),
        rate: None,
        volume: None,
        voice: None,
        scope: Some(SpeakScope::Care),
    }
}

/// 生产组合根真注入读取面 (R3): 发声配置带情绪读取面, 未绑定合成器 =
/// 无情绪数据; 绑定后读取面读到的就是 presence/PAD 合成器**现值** (同一真值,
/// 不缓存不另造)。
#[tokio::test]
async fn production_voice_config_injects_the_presence_pad_reader_face() {
    let _lock = lock_env();
    let (_guard, _data) = voice_env_guard();

    let emotion = VoiceEmotionSource::default();
    let config = voice_speak_config_with_emotion_from_env(&emotion)
        .expect("config resolves")
        .expect("旋钮开 = 注入发声配置");
    assert!(
        config.emotion_reader.is_some(),
        "组合根必须注入情绪读取面 (with_emotion_reader)"
    );
    assert!(
        emotion.current().is_none(),
        "未绑定合成器 = 无情绪数据 (显式 None, 不猜情绪)"
    );

    // 绑到活的 presence/PAD 合成器, 跑一轮真回合 (2 次召回命中 → 愉悦 0.5)。
    let presence = PresenceService::new(EventBus::new(8));
    emotion.bind(&presence);
    drive_turn(&presence, 2, 0, 0, 1);

    let reading = emotion.current().expect("绑定后有现值");
    let pad = presence.current_pad().expect("合成器现值");
    assert_eq!(
        (reading.pleasure, reading.arousal, reading.dominance),
        (pad.p, pad.a, pad.d),
        "读取面读的就是合成器现值 (同一真值)"
    );
    assert_eq!(reading.pleasure, 0.5, "情感残响 2 命中 = 愉悦 0.5");
    let reader = config.emotion_reader.clone().expect("读取面在场");
    assert_eq!(
        EmotionBand::derive(&reader().expect("读取面现读")),
        EmotionBand::HighPleasure
    );
}

/// 情绪→语气微调端到端生效 (R3 第一律: 能力真能用): 合成器状态 → 读取面 →
/// 冻结映射表 → 引擎请求逐段真走。愉悦高档 = 语速 +1; 张力高档 = 语速 -1。
#[tokio::test]
async fn emotion_tune_reaches_the_speech_request_end_to_end() {
    let _lock = lock_env();
    let (_guard, _data) = voice_env_guard();

    let emotion = VoiceEmotionSource::default();
    let presence = PresenceService::new(EventBus::new(8));
    emotion.bind(&presence);
    let config = voice_speak_config_with_emotion_from_env(&emotion)
        .expect("config resolves")
        .expect("旋钮开 = 注入发声配置");
    let engine = Arc::new(
        MockSynthEngine::new(MockBehavior::AutoComplete).with_voices(vec!["默认嗓".to_string()]),
    );
    let state = engine.state();
    let tool = VoiceSpeakTool::new(config, engine);

    // 愉悦高档 (2 次召回命中 → pleasure 0.5 ≥ 0.5): 缺省语速吃 +1。
    drive_turn(&presence, 2, 0, 0, 1);
    let outcome = tool
        .speak_once(&care_params("轻快一句"))
        .await
        .expect("speak");
    assert_eq!(
        outcome.rate,
        VOICE_RATE_DEFAULT + 1,
        "愉悦高档微调必须端到端生效"
    );
    let requests = state.requests();
    assert_eq!(
        requests.last().expect("引擎请求").rate,
        VOICE_RATE_DEFAULT + 1,
        "微调后的生效参数必须原样进引擎请求"
    );

    // 张力高档 (4 次工具 + 1 次审批等待 + 2 轮: pleasure ≤ 0 且 arousal ≥ 0.5):
    // 缺省语速吃 -1。
    drive_turn(&presence, 0, 4, 1, 2);
    let outcome = tool
        .speak_once(&care_params("沉着一句"))
        .await
        .expect("speak");
    assert_eq!(
        outcome.rate,
        VOICE_RATE_DEFAULT - 1,
        "张力高档微调必须端到端生效"
    );
    let requests = state.requests();
    assert_eq!(
        requests.last().expect("引擎请求").rate,
        VOICE_RATE_DEFAULT - 1
    );
}

/// 无情绪数据 = 原参数零变化 (三条硬口径之一, fail-safe 朝原参数): 读取面
/// 缺席或返回 `None` 时生效参数 = 缺省常量, 不按默认情绪调。
#[tokio::test]
async fn no_emotion_data_keeps_the_original_parameters_unchanged() {
    let _lock = lock_env();
    let (_guard, _data) = voice_env_guard();

    // 读取面在场但未绑定合成器 (纯 CLI 形态 / 合成器缺席)。
    let emotion = VoiceEmotionSource::default();
    let config = voice_speak_config_with_emotion_from_env(&emotion)
        .expect("config resolves")
        .expect("旋钮开 = 注入发声配置");
    let engine = Arc::new(
        MockSynthEngine::new(MockBehavior::AutoComplete).with_voices(vec!["默认嗓".to_string()]),
    );
    let state = engine.state();
    let tool = VoiceSpeakTool::new(config, engine);

    let outcome = tool
        .speak_once(&care_params("平平一句"))
        .await
        .expect("speak");
    assert_eq!(
        (outcome.rate, outcome.volume),
        (VOICE_RATE_DEFAULT, VOICE_VOLUME_DEFAULT),
        "无情绪数据 = 原参数零变化"
    );
    let requests = state.requests();
    assert_eq!(
        (requests[0].rate, requests[0].volume),
        (VOICE_RATE_DEFAULT, VOICE_VOLUME_DEFAULT)
    );

    // 绑定了合成器但从未有情绪数据 (基线读数即中性档) = 同样零微调。
    let presence = PresenceService::new(EventBus::new(8));
    emotion.bind(&presence);
    let outcome = tool
        .speak_once(&care_params("再一句"))
        .await
        .expect("speak");
    assert_eq!(
        (outcome.rate, outcome.volume),
        (VOICE_RATE_DEFAULT, VOICE_VOLUME_DEFAULT),
        "中性读数 = 零微调 (冻结表中性档 0/0)"
    );
}
