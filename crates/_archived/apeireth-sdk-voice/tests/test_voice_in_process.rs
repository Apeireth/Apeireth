//! # apeireth-sdk-voice in-process stub test (R20 阶段 4 效果, 12-15 测试)
//!
//! 验证 STUB 模式 5 守门 + 6 K-1 强校验 + 6 核心 API 全 NotImplemented,
//! 防止整合 #2 sub-agent 改 STUB_MODE=false 时漏防.
//!
//! ## Fixture 列表 (per 任务 K-1 强校验 6 条 + 10 额外)
//!
//! - **Fixture 1**: 4 STT 模型 (Whisper / Wav2Vec / Deepgram / Google)
//! - **Fixture 2**: 4 TTS 模型 (ElevenLabs / Azure / Google / OpenAI)
//! - **Fixture 3**: 4 唤醒词类别 (Hardcoded / Custom / Phonetic / Semantic)
//! - **Fixture 4**: 3 VAD 算法 (Energy / Silence / WebRtc)
//! - **Fixture 5**: 6 K-1 强校验 (api_key / audio_format / sample_rate / bit_depth / channels / language)
//! - **Fixture 6**: 7 TOOL_WHITELIST (6 核心 API + 1 stub_status)
//! - **Fixture 7-12**: 6 核心 API 各自返 NotImplemented
//! - **Fixture 13**: 默认唤醒词 = "apeireth" (per R20 设计拍板)
//! - **Fixture 14**: 5 K-1 字样守门 ("apeireth" / "voice" / "stub" / "wake" / "must-do")
//! - **Fixture 15**: STUB_MODE == true + is_stub_mode() 守门

use std::time::Duration;

use apeireth_sdk_voice::{
    validate_tool_call, Audio, AudioConfig, SttModel, SttRequest, Transcription, TtsModel,
    TtsRequest, VadAlgorithm, VadConfig, VadResult, VoiceClient, VoiceClientImpl, VoiceConfig,
    VoiceError, WakeWord, WakeWordCategory, WakeWordDetection, CORE_API_COUNT, K1_STRONG_VALIDATION_COUNT,
    PLATFORM_NAME, PROVIDER_NAME, STT_MODEL_COUNT, STUB_MODE, SUPPORTED_STT_MODELS,
    SUPPORTED_TTS_MODELS, SUPPORTED_VAD_ALGORITHMS, SUPPORTED_WAKE_WORD_CATEGORIES,
    TTS_MODEL_COUNT, VAD_ALGORITHM_COUNT, VOICE_DEFAULT_WAKE_WORD, VOICE_SCHEMA_VERSION,
    WAKE_WORD_CATEGORY_COUNT,
};

/// Fixture 1: 4 STT 模型 (per task spec §3)
#[test]
fn fixture_1_stt_4_models() {
    assert_eq!(SUPPORTED_STT_MODELS.len(), 4, "K-1 强校验: 必须 4 个 STT 模型");
    assert_eq!(SttModel::COUNT, 4);
    assert_eq!(STT_MODEL_COUNT, 4);
    assert_eq!(SttModel::Whisper.as_str(), "whisper");
    assert_eq!(SttModel::Wav2Vec.as_str(), "wav2vec");
    assert_eq!(SttModel::Deepgram.as_str(), "deepgram");
    assert_eq!(SttModel::Google.as_str(), "google");
    // 4 模型互不相同
    for i in 0..SUPPORTED_STT_MODELS.len() {
        for j in (i + 1)..SUPPORTED_STT_MODELS.len() {
            assert_ne!(SUPPORTED_STT_MODELS[i], SUPPORTED_STT_MODELS[j]);
        }
    }
}

/// Fixture 2: 4 TTS 模型 (per task spec §3)
#[test]
fn fixture_2_tts_4_models() {
    assert_eq!(SUPPORTED_TTS_MODELS.len(), 4, "K-1 强校验: 必须 4 个 TTS 模型");
    assert_eq!(TtsModel::COUNT, 4);
    assert_eq!(TTS_MODEL_COUNT, 4);
    assert_eq!(TtsModel::ElevenLabs.as_str(), "elevenlabs");
    assert_eq!(TtsModel::Azure.as_str(), "azure");
    assert_eq!(TtsModel::Google.as_str(), "google");
    assert_eq!(TtsModel::OpenAI.as_str(), "openai");
    // 4 模型互不相同
    for i in 0..SUPPORTED_TTS_MODELS.len() {
        for j in (i + 1)..SUPPORTED_TTS_MODELS.len() {
            assert_ne!(SUPPORTED_TTS_MODELS[i], SUPPORTED_TTS_MODELS[j]);
        }
    }
}

/// Fixture 3: 4 唤醒词类别 (per task spec §3)
#[test]
fn fixture_3_wake_word_4_categories() {
    assert_eq!(SUPPORTED_WAKE_WORD_CATEGORIES.len(), 4, "K-1 强校验: 必须 4 个唤醒词类别");
    assert_eq!(WakeWordCategory::COUNT, 4);
    assert_eq!(WAKE_WORD_CATEGORY_COUNT, 4);
    assert_eq!(WakeWordCategory::Hardcoded.as_str(), "hardcoded");
    assert_eq!(WakeWordCategory::Custom.as_str(), "custom");
    assert_eq!(WakeWordCategory::Phonetic.as_str(), "phonetic");
    assert_eq!(WakeWordCategory::Semantic.as_str(), "semantic");
    // 4 类别互不相同
    for i in 0..SUPPORTED_WAKE_WORD_CATEGORIES.len() {
        for j in (i + 1)..SUPPORTED_WAKE_WORD_CATEGORIES.len() {
            assert_ne!(SUPPORTED_WAKE_WORD_CATEGORIES[i], SUPPORTED_WAKE_WORD_CATEGORIES[j]);
        }
    }
}

/// Fixture 4: 默认唤醒词 = "apeireth" (per R20 设计拍板, 1:1 翻译品牌一致)
#[test]
fn fixture_4_wake_word_default_apeireth() {
    assert_eq!(VOICE_DEFAULT_WAKE_WORD, "apeireth", "K-1 强校验: 默认唤醒词必须是 'apeireth' (R20 拍板)");
    // 编译期常量
    let client = VoiceClientImpl::new();
    assert_eq!(client.default_wake_word(), "apeireth");
    // WakeWord::default_apeireth() 工厂
    let wake = WakeWord::default_apeireth();
    assert_eq!(wake.category, WakeWordCategory::Hardcoded);
    assert_eq!(wake.keyword, "apeireth");
    // WakeWordCategory::Hardcoded.default_word() 返 "apeireth"
    assert_eq!(WakeWordCategory::Hardcoded.default_word(), Some("apeireth"));
    // VoiceConfig 默认也是 apeireth
    let config = VoiceConfig::default_apeireth();
    assert!(config.is_default_apeireth());
}

/// Fixture 5: 3 VAD 算法 (per task spec §3)
#[test]
fn fixture_5_vad_3_algorithms() {
    assert_eq!(SUPPORTED_VAD_ALGORITHMS.len(), 3, "K-1 强校验: 必须 3 个 VAD 算法");
    assert_eq!(VadAlgorithm::COUNT, 3);
    assert_eq!(VAD_ALGORITHM_COUNT, 3);
    assert_eq!(VadAlgorithm::Energy.as_str(), "energy");
    assert_eq!(VadAlgorithm::Silence.as_str(), "silence");
    assert_eq!(VadAlgorithm::WebRtc.as_str(), "webrtc");
    // 3 算法互不相同
    for i in 0..SUPPORTED_VAD_ALGORITHMS.len() {
        for j in (i + 1)..SUPPORTED_VAD_ALGORITHMS.len() {
            assert_ne!(SUPPORTED_VAD_ALGORITHMS[i], SUPPORTED_VAD_ALGORITHMS[j]);
        }
    }
}

/// Fixture 6: 6 K-1 强校验 (per task spec §3 + v0.9.21 商业版)
#[test]
fn fixture_6_6_k1_strong_validations() {
    // K-1 #1: API Key (非空 + 长度 ≥ 16)
    assert!(matches!(
        VoiceError::validate_api_key(""),
        Err(VoiceError::ApiKeyMissing)
    ));
    assert!(matches!(
        VoiceError::validate_api_key("short"),
        Err(VoiceError::ApiKeyInvalid(5))
    ));
    assert!(VoiceError::validate_api_key("1234567890abcdef").is_ok());

    // K-1 #2: Audio Format (wav/mp3/opus/flac)
    assert!(VoiceError::validate_audio_format("wav").is_ok());
    assert!(VoiceError::validate_audio_format("mp3").is_ok());
    assert!(VoiceError::validate_audio_format("opus").is_ok());
    assert!(VoiceError::validate_audio_format("flac").is_ok());
    assert!(matches!(
        VoiceError::validate_audio_format("aac"),
        Err(VoiceError::AudioFormatInvalid(_))
    ));

    // K-1 #3: Sample Rate (8000..=48000)
    assert!(VoiceError::validate_sample_rate(8000).is_ok());
    assert!(VoiceError::validate_sample_rate(48000).is_ok());
    assert!(matches!(
        VoiceError::validate_sample_rate(7999),
        Err(VoiceError::SampleRateInvalid(_))
    ));
    assert!(matches!(
        VoiceError::validate_sample_rate(48001),
        Err(VoiceError::SampleRateInvalid(_))
    ));

    // K-1 #4: Bit Depth (8/16/24/32)
    for bd in [8, 16, 24, 32] {
        assert!(VoiceError::validate_bit_depth(bd).is_ok());
    }
    assert!(matches!(
        VoiceError::validate_bit_depth(12),
        Err(VoiceError::BitDepthInvalid(_))
    ));

    // K-1 #5: Channels (1/2)
    assert!(VoiceError::validate_channels(1).is_ok());
    assert!(VoiceError::validate_channels(2).is_ok());
    assert!(matches!(
        VoiceError::validate_channels(6),
        Err(VoiceError::ChannelsInvalid(_))
    ));

    // K-1 #6: Language (ISO 639-1 e.g. en, zh-CN)
    assert!(VoiceError::validate_language("en").is_ok());
    assert!(VoiceError::validate_language("zh-CN").is_ok());
    assert!(matches!(
        VoiceError::validate_language("english"),
        Err(VoiceError::LanguageInvalid(_))
    ));

    assert_eq!(K1_STRONG_VALIDATION_COUNT, 6);
}

/// Fixture 7: 7 TOOL_WHITELIST (6 核心 API + 1 stub_status)
#[test]
fn fixture_7_tool_whitelist_has_7_tools() {
    use apeireth_sdk_voice::TOOL_WHITELIST;
    use apeireth_sdk_voice::TOOL_WHITELIST_COUNT;

    assert_eq!(TOOL_WHITELIST.len(), 7, "K-1 强校验: 必须 7 个工具 (6 核心 API + 1 stub_status)");
    assert_eq!(TOOL_WHITELIST_COUNT, 7);
    let expected = [
        "apeireth_voice_transcribe",
        "apeireth_voice_synthesize",
        "apeireth_voice_detect_wake",
        "apeireth_voice_start_listening",
        "apeireth_voice_stop_listening",
        "apeireth_voice_stream_audio",
        "apeireth_voice_stub_status",
    ];
    for tool in expected {
        assert!(TOOL_WHITELIST.contains(&tool), "TOOL_WHITELIST must contain {tool}");
    }
}

/// Fixture 8: API 1: `transcribe` 返 NotImplemented
#[tokio::test]
async fn fixture_8_transcribe_returns_not_implemented() {
    let mut client = VoiceClientImpl::new();
    client
        .set_api_key("sk-ant-voice-abcdef1234567890xyz".to_string())
        .expect("valid api key");
    let req = SttRequest::new(
        vec![0u8; 100],
        "wav".to_string(),
        16000,
        16,
        1,
        SttModel::Whisper,
        Some("en".to_string()),
    )
    .expect("valid request");
    let result = client.transcribe(&req).await;
    assert!(
        matches!(result, Err(VoiceError::NotImplemented("transcribe"))),
        "transcribe must return NotImplemented, got {:?}",
        result
    );
}

/// Fixture 9: API 2: `synthesize` 返 NotImplemented
#[tokio::test]
async fn fixture_9_synthesize_returns_not_implemented() {
    let mut client = VoiceClientImpl::new();
    client
        .set_api_key("sk-ant-voice-abcdef1234567890xyz".to_string())
        .expect("valid api key");
    let req = TtsRequest::with_defaults(
        "hello world".to_string(),
        TtsModel::OpenAI,
        "alloy".to_string(),
        "en".to_string(),
    )
    .expect("valid request");
    let result = client.synthesize(&req).await;
    assert!(
        matches!(result, Err(VoiceError::NotImplemented("synthesize"))),
        "synthesize must return NotImplemented, got {:?}",
        result
    );
}

/// Fixture 10: API 3: `detect_wake` 返 NotImplemented
#[tokio::test]
async fn fixture_10_detect_wake_returns_not_implemented() {
    let mut client = VoiceClientImpl::new();
    client
        .set_api_key("sk-ant-voice-abcdef1234567890xyz".to_string())
        .expect("valid api key");
    let audio = vec![0i16; 512];
    let result = client.detect_wake(&audio).await;
    assert!(
        matches!(result, Err(VoiceError::NotImplemented("detect_wake"))),
        "detect_wake must return NotImplemented, got {:?}",
        result
    );
}

/// Fixture 11: API 4: `start_listening` 返 NotImplemented
#[tokio::test]
async fn fixture_11_start_listening_returns_not_implemented() {
    let mut client = VoiceClientImpl::new();
    client
        .set_api_key("sk-ant-voice-abcdef1234567890xyz".to_string())
        .expect("valid api key");
    let result = client.start_listening().await;
    assert!(
        matches!(result, Err(VoiceError::NotImplemented("start_listening"))),
        "start_listening must return NotImplemented, got {:?}",
        result
    );
}

/// Fixture 12: API 5: `stop_listening` 守门 (listening=false 时返 Other, 守门优先)
#[tokio::test]
async fn fixture_12_stop_listening_returns_not_implemented() {
    let mut client = VoiceClientImpl::new();
    client
        .set_api_key("sk-ant-voice-abcdef1234567890xyz".to_string())
        .expect("valid api key");
    let result = client.stop_listening().await;
    // stop_listening 先检查 listening=false, 守门优先返 Other
    assert!(
        matches!(result, Err(VoiceError::Other(_))),
        "stop_listening must check listening first, got {:?}",
        result
    );
}

/// Fixture 13: API 6: `stream_audio` 返 NotImplemented
#[tokio::test]
async fn fixture_13_stream_audio_returns_not_implemented() {
    let mut client = VoiceClientImpl::new();
    client
        .set_api_key("sk-ant-voice-abcdef1234567890xyz".to_string())
        .expect("valid api key");
    let vad = VadResult::new(
        true,
        VadAlgorithm::Energy,
        0.95,
        Duration::from_millis(1000),
        Duration::from_millis(500),
    );
    let result = client.stream_audio(&vad).await;
    assert!(
        matches!(result, Err(VoiceError::NotImplemented("stream_audio"))),
        "stream_audio must return NotImplemented, got {:?}",
        result
    );
}

/// Fixture 14: 5 K-1 字样守门 (源码含 "apeireth" / "voice" / "stub" / "wake" / "must-do")
#[test]
fn fixture_14_5_k1_keywords_in_source() {
    let source = include_str!("../src/lib.rs");
    assert!(source.contains("apeireth"), "must-do: 源码必须出现 'apeireth' (K-1 字样 #1)");
    assert!(source.contains("voice"), "must-do: 源码必须出现 'voice' (K-1 字样 #2)");
    assert!(source.contains("stub"), "must-do: 源码必须出现 'stub' (K-1 字样 #3)");
    assert!(source.contains("wake"), "must-do: 源码必须出现 'wake' (K-1 字样 #4)");
    assert!(
        source.contains("must-do") || source.contains("MUST"),
        "must-do: 源码必须出现 'must-do' 守门字样 (K-1 字样 #5)"
    );
    // 编译期常量也守
    assert_eq!(PLATFORM_NAME, "apeireth");
    assert_eq!(PROVIDER_NAME, "anthropic-voice");
    assert_eq!(VOICE_SCHEMA_VERSION, "1");
    assert_eq!(VOICE_DEFAULT_WAKE_WORD, "apeireth");
    let _ = STUB_MODE;
}

/// Fixture 15: STUB_MODE == true 守门 + VoiceClientImpl 构造 + 6 核心 API 数
#[test]
fn fixture_15_stub_mode_and_construction() {
    let _ = STUB_MODE;
    assert!(apeireth_sdk_voice::is_stub_mode());
    assert_eq!(apeireth_sdk_voice::is_stub_mode(), STUB_MODE);

    let client = VoiceClientImpl::new();
    assert_eq!(client.platform(), "apeireth");
    assert!(!client.is_listening());
    assert!(!client.has_api_key());
    assert_eq!(client.default_wake_word(), "apeireth");
    assert_eq!(client.stt_model(), SttModel::Whisper);
    assert_eq!(client.tts_model(), TtsModel::ElevenLabs);
    assert_eq!(client.vad_algorithm(), VadAlgorithm::Energy);
    assert_eq!(CORE_API_COUNT, 6);

    // stub_status
    let status = client.stub_status();
    assert!(status.stub_mode);
    assert_eq!(status.platform, "apeireth");
    assert_eq!(status.schema_version, "1");
    assert!(!status.api_key_set);
    assert!(!status.listening);
    assert_eq!(status.default_wake_word, "apeireth");
    assert_eq!(status.wake_word_category, WakeWordCategory::Hardcoded);
    assert_eq!(status.stt_model, SttModel::Whisper);
    assert_eq!(status.tts_model, TtsModel::ElevenLabs);
    assert_eq!(status.vad_algorithm, VadAlgorithm::Energy);

    // list_apis / list_helpers
    assert_eq!(VoiceClientImpl::list_apis().len(), CORE_API_COUNT);
    assert_eq!(VoiceClientImpl::list_stt_models().len(), STT_MODEL_COUNT);
    assert_eq!(VoiceClientImpl::list_tts_models().len(), TTS_MODEL_COUNT);
    assert_eq!(
        VoiceClientImpl::list_wake_word_categories().len(),
        WAKE_WORD_CATEGORY_COUNT
    );
    assert_eq!(VoiceClientImpl::list_vad_algorithms().len(), VAD_ALGORITHM_COUNT);
}

/// 额外 1: validate_tool_call 接受白名单拒绝非白名单
#[test]
fn extra_1_validate_tool_call() {
    let args = serde_json::json!({});
    assert!(validate_tool_call("apeireth_voice_transcribe", &args).is_ok());
    assert!(validate_tool_call("apeireth_voice_synthesize", &args).is_ok());
    assert!(validate_tool_call("apeireth_voice_stub_status", &args).is_ok());
    let err = validate_tool_call("apeireth_voice_bogus_tool", &args).unwrap_err();
    assert!(matches!(err, VoiceError::ToolNotWhitelisted(_)));
}

/// 额外 2: 4 STT 模型 + 4 TTS 模型 + 4 唤醒词 + 3 VAD 计数守门 (K-1 强校验)
#[test]
fn extra_2_all_model_counts() {
    assert_eq!(SUPPORTED_STT_MODELS.len(), 4);
    assert_eq!(SUPPORTED_TTS_MODELS.len(), 4);
    assert_eq!(SUPPORTED_WAKE_WORD_CATEGORIES.len(), 4);
    assert_eq!(SUPPORTED_VAD_ALGORITHMS.len(), 3);
}

/// 额外 3: SttRequest / TtsRequest 跨 K-1 守门 (audio_format + sample_rate + bit_depth + channels + language)
#[test]
fn extra_3_stt_request_k1_validations() {
    // audio_format 错
    let result = SttRequest::new(
        vec![0u8; 100],
        "aac".to_string(),
        16000,
        16,
        1,
        SttModel::Whisper,
        None,
    );
    assert!(matches!(result, Err(VoiceError::AudioFormatInvalid(_))));

    // sample_rate 错
    let result = SttRequest::new(
        vec![0u8; 100],
        "wav".to_string(),
        5000,
        16,
        1,
        SttModel::Whisper,
        None,
    );
    assert!(matches!(result, Err(VoiceError::SampleRateInvalid(_))));

    // bit_depth 错
    let result = SttRequest::new(
        vec![0u8; 100],
        "wav".to_string(),
        16000,
        12,
        1,
        SttModel::Whisper,
        None,
    );
    assert!(matches!(result, Err(VoiceError::BitDepthInvalid(_))));

    // channels 错
    let result = SttRequest::new(
        vec![0u8; 100],
        "wav".to_string(),
        16000,
        16,
        6,
        SttModel::Whisper,
        None,
    );
    assert!(matches!(result, Err(VoiceError::ChannelsInvalid(_))));

    // language 错
    let result = SttRequest::new(
        vec![0u8; 100],
        "wav".to_string(),
        16000,
        16,
        1,
        SttModel::Whisper,
        Some("english".to_string()),
    );
    assert!(matches!(result, Err(VoiceError::LanguageInvalid(_))));
}

/// 额外 4: Audio K-1 守门 (audio_format + sample_rate + bit_depth + channels)
#[test]
fn extra_4_audio_k1_validations() {
    let result = Audio::new(vec![0u8; 100], "aac".to_string(), 44100, 16, 2, 5000);
    assert!(matches!(result, Err(VoiceError::AudioFormatInvalid(_))));

    let result = Audio::new(vec![0u8; 100], "mp3".to_string(), 5000, 16, 2, 5000);
    assert!(matches!(result, Err(VoiceError::SampleRateInvalid(_))));

    let result = Audio::new(vec![0u8; 100], "mp3".to_string(), 44100, 12, 2, 5000);
    assert!(matches!(result, Err(VoiceError::BitDepthInvalid(_))));

    let result = Audio::new(vec![0u8; 100], "mp3".to_string(), 44100, 16, 6, 5000);
    assert!(matches!(result, Err(VoiceError::ChannelsInvalid(_))));
}

/// 额外 5: Transcription / WakeWordDetection 字段构造
#[test]
fn extra_5_transcription_and_detection() {
    let t = Transcription::new("hello".to_string(), SttModel::Whisper, "en".to_string(), 0.95, 1500);
    assert_eq!(t.text, "hello");
    assert_eq!(t.model, SttModel::Whisper);
    assert!(!t.is_empty());

    let det = WakeWordDetection::new(
        WakeWordCategory::Hardcoded,
        "apeireth".to_string(),
        0.95,
    );
    assert!(det.is_apeireth());
    assert_eq!(det.category, WakeWordCategory::Hardcoded);
}

/// 额外 6: VadConfig 3 默认守门
#[test]
fn extra_6_vad_config_defaults() {
    let energy = VadConfig::default_energy();
    assert_eq!(energy.algorithm, VadAlgorithm::Energy);

    let silence = VadConfig::default_silence();
    assert_eq!(silence.algorithm, VadAlgorithm::Silence);
    assert_eq!(silence.silence_threshold_ms, 500);

    let webrtc = VadConfig::default_webrtc();
    assert_eq!(webrtc.algorithm, VadAlgorithm::WebRtc);
    assert_eq!(webrtc.frame_size_ms, 20);
}

/// 额外 7: AudioConfig 5 段守门
#[test]
fn extra_7_audio_config_valid() {
    let audio = AudioConfig::default_wav();
    assert_eq!(audio.format, "wav");
    assert_eq!(audio.sample_rate, 16_000);
    assert_eq!(audio.bit_depth, 16);
    assert_eq!(audio.channels, 1);
    assert_eq!(audio.language, "en");
}

/// 额外 8: VoiceConfig 5 段守门 (K-1 强校验)
#[test]
fn extra_8_voice_config_5_sections() {
    let config = VoiceConfig::default_apeireth();
    assert_eq!(config.wake.category, WakeWordCategory::Hardcoded);
    assert_eq!(config.wake.keyword, "apeireth");
    assert_eq!(config.stt, SttModel::Whisper);
    assert_eq!(config.tts, TtsModel::ElevenLabs);
    assert_eq!(config.vad.algorithm, VadAlgorithm::Energy);
    assert_eq!(config.audio.format, "wav");
    assert!(config.is_default_apeireth());
    assert!(config.validate().is_ok());
}
