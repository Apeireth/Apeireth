//! # apeireth-voice in-process stub test (R20 阶段 1 续, Fixture 5 + 2 额外)
//!
//! 验证 STUB 模式 5 守门 + 2 额外行为, 防止整合 #2 sub-agent 改 STUB_MODE=false 时漏防.
//!
//! ## Fixture 列表 (per 任务 K-1 强校验 4 条 + 2 额外)
//!
//! - **Fixture 1**: 编译期 hardcode 7 项常量 (VOICE_SCHEMA_VERSION / PLATFORM_NAME / STUB_MODE /
//!   VOICE_SAMPLE_RATE_HZ / VOICE_FRAME_LENGTH / VOICE_DEFAULT_KEYWORD / VOICE_MAX_AUDIO_SECONDS)
//! - **Fixture 2**: 5 WakeWordType 枚举 (Apeireth / Computer / Jarvis / HeyApeireth / Custom)
//! - **Fixture 3**: TOOL_WHITELIST 9 工具名 (8 商业版 + 1 stub_status)
//! - **Fixture 4**: 5 K-1 字样 ("apeireth" / "voice" / "stub" / "wake_word" / "must-do") +
//!   STUB_MODE == true 守门
//! - **Fixture 5**: 8 stub 工具全部返 VoiceError::NotImplemented (额外 1, 体现 stub 模式)
//! - **额外**: 默认唤醒词 = "apeireth" (额外 2, 1:1 翻译品牌一致)

use apeireth_voice::{
    validate_tool_call, AudioFrame, AudioStreamSource, StubAudioStream, VoiceConfig, VoiceError,
    VoiceRecorder, VoiceSdk, VoiceWake, WakeWordType, PLATFORM_NAME, STUB_MODE,
    SUPPORTED_WAKE_WORDS, TOOL_WHITELIST, TOOL_WHITELIST_COUNT, VOICE_DEFAULT_KEYWORD,
    VOICE_FRAME_LENGTH, VOICE_MAX_AUDIO_SECONDS, VOICE_SAMPLE_RATE_HZ, VOICE_SCHEMA_VERSION,
};

/// Fixture 1: 编译期 hardcode 7 项常量守门
#[test]
fn fixture_1_compile_time_constants_match() {
    assert_eq!(VOICE_SCHEMA_VERSION, "1", "voice schema version must be '1'");
    assert_eq!(PLATFORM_NAME, "apeireth", "platform name must be 'apeireth' (K-1 品牌一致)");
    assert!(STUB_MODE, "STUB_MODE must be true until R20 stage 3 (K-1 强校验守门)");
    assert_eq!(VOICE_SAMPLE_RATE_HZ, 16000, "Porcupine official sample rate");
    assert_eq!(VOICE_FRAME_LENGTH, 512, "Porcupine official frame length");
    assert_eq!(VOICE_DEFAULT_KEYWORD, "apeireth", "default wake word must be 'apeireth' (K-1 品牌一致)");
    assert_eq!(VOICE_MAX_AUDIO_SECONDS, 30, "single recording max 30s");
}

/// Fixture 2: 5 WakeWordType 枚举守门
#[test]
fn fixture_2_wake_word_type_has_5_variants() {
    assert_eq!(SUPPORTED_WAKE_WORDS.len(), 5, "K-1 强校验: 必须 5 个 WakeWordType");
    assert_eq!(WakeWordType::Apeireth.as_str(), "apeireth");
    assert_eq!(WakeWordType::Computer.as_str(), "computer");
    assert_eq!(WakeWordType::Jarvis.as_str(), "jarvis");
    assert_eq!(WakeWordType::HeyApeireth.as_str(), "hey apeireth");
    assert_eq!(WakeWordType::Custom.as_str(), "custom");
}

/// Fixture 3: TOOL_WHITELIST 9 工具名守门
#[test]
fn fixture_3_tool_whitelist_has_9_tools() {
    assert_eq!(TOOL_WHITELIST.len(), 9, "K-1 强校验: 必须 9 个工具 (8 商业版 + 1 stub_status)");
    assert_eq!(TOOL_WHITELIST_COUNT, 9);
    let expected = [
        "apeireth_voice_wake_word_detect",
        "apeireth_voice_record_audio",
        "apeireth_voice_transcribe",
        "apeireth_voice_synthesize",
        "apeireth_voice_list_keywords",
        "apeireth_voice_load_model",
        "apeireth_voice_unload_model",
        "apeireth_voice_audio_stream",
        "apeireth_voice_stub_status",
    ];
    for tool in expected {
        assert!(TOOL_WHITELIST.contains(&tool), "TOOL_WHITELIST must contain {tool}");
    }
}

/// Fixture 4: 5 K-1 字样 + STUB_MODE == true 守门
#[test]
fn fixture_4_k1_keywords_and_stub_mode() {
    // 5 K-1 字样: "apeireth" / "voice" / "stub" / "wake_word" / "must-do"
    let source = include_str!("../src/lib.rs");
    assert!(source.contains("apeireth"), "must-do: 源码必须出现 'apeireth' (K-1 字样 #1)");
    assert!(source.contains("voice"), "must-do: 源码必须出现 'voice' (K-1 字样 #2)");
    assert!(source.contains("stub"), "must-do: 源码必须出现 'stub' (K-1 字样 #3)");
    assert!(source.contains("wake_word"), "must-do: 源码必须出现 'wake_word' (K-1 字样 #4)");
    assert!(source.contains("must-do") || source.contains("MUST"), "must-do: 源码必须出现 'must-do' 守门字样 (K-1 字样 #5)");
    // STUB_MODE == true 守门
    assert!(STUB_MODE, "STUB_MODE must be true (K-1 强校验 #4 守门)");
}

/// Fixture 5: 8 stub 工具全部返 VoiceError::NotImplemented (额外 1, 体现 stub 模式)
#[tokio::test]
async fn fixture_5_8_stub_tools_return_not_implemented() {
    let mut sdk = VoiceSdk::new(VoiceConfig::default()).expect("VoiceSdk::new must succeed in STUB mode");
    let frame = AudioFrame::new(vec![0i16; VOICE_FRAME_LENGTH as usize]);

    // 8 stub 工具必须全部返 VoiceError::NotImplemented
    let r1 = sdk.wake_word_detect(&frame).await;
    assert!(matches!(r1, Err(VoiceError::NotImplemented("apeireth_voice_wake_word_detect"))),
        "wake_word_detect must return NotImplemented, got {:?}", r1);

    let r2 = sdk.record_audio(5).await;
    assert!(matches!(r2, Err(VoiceError::NotImplemented("apeireth_voice_record_audio"))),
        "record_audio must return NotImplemented, got {:?}", r2);

    let r3 = sdk.transcribe(&[0i16; 512]).await;
    assert!(matches!(r3, Err(VoiceError::NotImplemented("apeireth_voice_transcribe"))),
        "transcribe must return NotImplemented, got {:?}", r3);

    let r4 = sdk.synthesize("hello").await;
    assert!(matches!(r4, Err(VoiceError::NotImplemented("apeireth_voice_synthesize"))),
        "synthesize must return NotImplemented, got {:?}", r4);

    // list_keywords 是编译期常量, 不返 NotImplemented
    let r5 = sdk.list_keywords();
    assert!(r5.is_ok(), "list_keywords must succeed (compile-time constant), got {:?}", r5);
    assert_eq!(r5.unwrap().len(), 5);

    let r6 = sdk.load_model(WakeWordType::Apeireth).await;
    assert!(matches!(r6, Err(VoiceError::NotImplemented("apeireth_voice_load_model"))),
        "load_model must return NotImplemented, got {:?}", r6);

    let r7 = sdk.unload_model().await;
    assert!(matches!(r7, Err(VoiceError::NotImplemented("apeireth_voice_unload_model"))),
        "unload_model must return NotImplemented, got {:?}", r7);

    let r8 = sdk.audio_stream().await;
    assert!(matches!(r8, Err(VoiceError::NotImplemented("apeireth_voice_audio_stream"))),
        "audio_stream must return NotImplemented, got {:?}", r8);
}

/// 额外 1: 默认唤醒词 = "apeireth" (1:1 翻译品牌一致)
#[test]
fn extra_1_default_keyword_is_apeireth() {
    assert_eq!(VOICE_DEFAULT_KEYWORD, "apeireth", "默认唤醒词必须 'apeireth' (1:1 翻译 v0.9.21 品牌一致)");
    assert_eq!(VoiceConfig::default().default_keyword, WakeWordType::Apeireth);
    let sdk = VoiceSdk::new(VoiceConfig::default()).unwrap();
    let status = sdk.stub_status().unwrap();
    assert_eq!(status.default_keyword, "apeireth");
    assert_eq!(status.platform, "apeireth");
    assert!(status.stub_mode, "stub_status 必须报 STUB_MODE=true");
}

/// 额外 2: validate_tool_call 接受白名单拒绝非白名单 + VoiceWake / VoiceRecorder 占位
#[test]
fn extra_2_validate_tool_call_and_components() {
    let args = serde_json::json!({});
    // 白名单内 OK
    assert!(validate_tool_call("apeireth_voice_wake_word_detect", &args).is_ok());
    assert!(validate_tool_call("apeireth_voice_stub_status", &args).is_ok());
    // 非白名单 Err(ToolNotWhitelisted)
    let err = validate_tool_call("apeireth_voice_bogus_tool", &args).unwrap_err();
    assert!(matches!(err, VoiceError::ToolNotWhitelisted(_)));

    // VoiceWake / VoiceRecorder 占位 (STUB 模式可 new, 但 start 返 NotImplemented)
    let mut wake = VoiceWake::new(VoiceConfig::default(), WakeWordType::Apeireth).unwrap();
    assert_eq!(wake.keyword(), WakeWordType::Apeireth);
    assert_eq!(wake.detection_count(), 0);

    let mut recorder = VoiceRecorder::new(VoiceConfig::default()).unwrap();
    assert_eq!(recorder.session_id(), None);
    assert_eq!(recorder.buffer_len(), 0);

    // StubAudioStream 占位
    let mut stream = StubAudioStream::new();
    let r = futures::executor::block_on(stream.next_frame());
    assert!(matches!(r, Err(VoiceError::NotImplemented("apeireth_voice_audio_stream"))));
}
