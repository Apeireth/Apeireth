//! # apeireth-sdk-voice stub demo (R20 阶段 4 效果)
//!
//! 演示 6 核心 API 返 `VoiceError::NotImplemented` + 默认唤醒词 `"apeireth"` +
//! 4 STT / 4 TTS / 4 唤醒词 / 3 VAD 枚举 + 6 K-1 强校验.
//! **R21 续真接 @anthropic-ai/voice 后, 本 demo 会被替换成真 Voice API demo.**
//!
//! ## 运行
//!
//! ```bash
//! cargo run --manifest-path crates/apeireth-sdk-voice/Cargo.toml --example voice_demo
//! ```

use apeireth_sdk_voice::{
    validate_tool_call, is_stub_mode, Audio, AudioConfig, SttModel, SttRequest, TtsModel, TtsRequest,
    VadAlgorithm, VadConfig, VoiceClient, VoiceClientImpl, VoiceConfig, VoiceError, WakeWord,
    WakeWordCategory, WakeWordDetection, CORE_API_COUNT, K1_STRONG_VALIDATION_COUNT, PLATFORM_NAME,
    PROVIDER_NAME, STT_MODEL_COUNT, SUPPORTED_STT_MODELS, SUPPORTED_TTS_MODELS,
    SUPPORTED_VAD_ALGORITHMS, SUPPORTED_WAKE_WORD_CATEGORIES, TTS_MODEL_COUNT,
    VAD_ALGORITHM_COUNT, VOICE_DEFAULT_WAKE_WORD, VOICE_SCHEMA_VERSION, WAKE_WORD_CATEGORY_COUNT,
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("=== apeireth-sdk-voice stub demo (R20 阶段 4 效果) ===");
    println!();

    // 1) 编译期 hardcode 守门 (K-1 强校验)
    println!("[§1 编译期 hardcode]");
    println!("  VOICE_SCHEMA_VERSION         = {}", VOICE_SCHEMA_VERSION);
    println!("  PLATFORM_NAME                = {}", PLATFORM_NAME);
    println!("  PROVIDER_NAME                = {}", PROVIDER_NAME);
    println!("  STUB_MODE                    = {}", is_stub_mode());
    println!("  VOICE_DEFAULT_WAKE_WORD      = {}", VOICE_DEFAULT_WAKE_WORD);
    println!("  CORE_API_COUNT               = {}", CORE_API_COUNT);
    println!("  STT_MODEL_COUNT              = {}", STT_MODEL_COUNT);
    println!("  TTS_MODEL_COUNT              = {}", TTS_MODEL_COUNT);
    println!("  WAKE_WORD_CATEGORY_COUNT     = {}", WAKE_WORD_CATEGORY_COUNT);
    println!("  VAD_ALGORITHM_COUNT          = {}", VAD_ALGORITHM_COUNT);
    println!("  K1_STRONG_VALIDATION_COUNT   = {}", K1_STRONG_VALIDATION_COUNT);
    println!();

    // 2) 4 STT 模型
    println!("[§2 4 STT 模型 (per v0.9.21 商业版 + task spec §3)]");
    for (i, model) in SUPPORTED_STT_MODELS.iter().enumerate() {
        println!(
            "  [{:>2}] {:?} -> \"{}\" (offline={}, max_audio_secs={})",
            i + 1,
            model,
            model.as_str(),
            model.is_offline(),
            model.max_audio_seconds()
        );
    }
    println!();

    // 3) 4 TTS 模型
    println!("[§3 4 TTS 模型 (per v0.9.21 商业版 + task spec §3)]");
    for (i, model) in SUPPORTED_TTS_MODELS.iter().enumerate() {
        println!(
            "  [{:>2}] {:?} -> \"{}\" (default_format={}, default_sample_rate={}, max_text_length={})",
            i + 1,
            model,
            model.as_str(),
            model.default_format(),
            model.default_sample_rate(),
            model.max_text_length()
        );
    }
    println!();

    // 4) 4 唤醒词类别
    println!("[§4 4 唤醒词类别 (per v0.9.21 商业版 + task spec §3, 默认 Hardcoded + 'apeireth')]");
    for (i, cat) in SUPPORTED_WAKE_WORD_CATEGORIES.iter().enumerate() {
        println!(
            "  [{:>2}] {:?} -> \"{}\" (default_word={:?})",
            i + 1,
            cat,
            cat.as_str(),
            cat.default_word()
        );
    }
    println!();

    // 5) 3 VAD 算法
    println!("[§5 3 VAD 算法 (per v0.9.21 商业版 + task spec §3)]");
    for (i, algo) in SUPPORTED_VAD_ALGORITHMS.iter().enumerate() {
        println!(
            "  [{:>2}] {:?} -> \"{}\" (offline={})",
            i + 1,
            algo,
            algo.as_str(),
            algo.is_offline()
        );
    }
    println!();

    // 6) 7 TOOL_WHITELIST (6 核心 API + 1 stub_status)
    println!("[§6 7 工具白名单 (m3 防御)]");
    use apeireth_sdk_voice::{TOOL_WHITELIST, TOOL_WHITELIST_COUNT};
    println!("  TOOL_WHITELIST_COUNT = {}", TOOL_WHITELIST_COUNT);
    for (i, tool) in TOOL_WHITELIST.iter().enumerate() {
        println!("  [{:>2}] {}", i + 1, tool);
    }
    println!();

    // 7) m3 防御: validate_tool_call 测试
    println!("[§7 m3 防御: validate_tool_call]");
    let args = serde_json::json!({});
    let valid = validate_tool_call("apeireth_voice_transcribe", &args);
    println!("  白名单内工具: {:?}", valid);
    let invalid = validate_tool_call("apeireth_voice_bogus", &args);
    println!("  非白名单工具: {:?}", invalid);
    println!();

    // 8) 6 K-1 强校验演示
    println!("[§8 6 K-1 强校验演示]");
    println!(
        "  K-1 #1 API Key:  \"\" -> {:?}",
        VoiceError::validate_api_key("")
    );
    println!(
        "  K-1 #1 API Key:  \"short\" -> {:?}",
        VoiceError::validate_api_key("short")
    );
    println!(
        "  K-1 #1 API Key:  \"sk-ant-voice-abcdef1234567890xyz\" -> {:?}",
        VoiceError::validate_api_key("sk-ant-voice-abcdef1234567890xyz")
    );
    println!(
        "  K-1 #2 Format:   \"wav\" -> {:?}",
        VoiceError::validate_audio_format("wav")
    );
    println!(
        "  K-1 #2 Format:   \"aac\" -> {:?}",
        VoiceError::validate_audio_format("aac")
    );
    println!(
        "  K-1 #3 Rate:     16000 -> {:?}",
        VoiceError::validate_sample_rate(16000)
    );
    println!(
        "  K-1 #3 Rate:     5000 -> {:?}",
        VoiceError::validate_sample_rate(5000)
    );
    println!(
        "  K-1 #4 Bit:      16 -> {:?}",
        VoiceError::validate_bit_depth(16)
    );
    println!(
        "  K-1 #4 Bit:      12 -> {:?}",
        VoiceError::validate_bit_depth(12)
    );
    println!(
        "  K-1 #5 Chan:     1 -> {:?}",
        VoiceError::validate_channels(1)
    );
    println!(
        "  K-1 #5 Chan:     6 -> {:?}",
        VoiceError::validate_channels(6)
    );
    println!(
        "  K-1 #6 Lang:     \"en\" -> {:?}",
        VoiceError::validate_language("en")
    );
    println!(
        "  K-1 #6 Lang:     \"english\" -> {:?}",
        VoiceError::validate_language("english")
    );
    println!();

    // 9) VoiceClientImpl 构造 + 6 核心 API 返 NotImplemented
    println!("[§9 6 核心 API stub 返 NotImplemented]");
    let mut client = VoiceClientImpl::new();
    client
        .set_api_key("sk-ant-voice-abcdef1234567890xyz".to_string())
        .expect("valid API key");
    println!("  构造完成, platform={}", client.platform());
    println!("  default_wake_word = {}", client.default_wake_word());
    println!("  stt_model         = {:?}", client.stt_model());
    println!("  tts_model         = {:?}", client.tts_model());
    println!("  vad_algorithm     = {:?}", client.vad_algorithm());

    // API 1: transcribe
    let req = SttRequest::new(
        vec![0u8; 100],
        "wav".to_string(),
        16000,
        16,
        1,
        SttModel::Whisper,
        Some("en".to_string()),
    )
    .expect("valid SttRequest");
    println!("  transcribe                    : {:?}", client.transcribe(&req).await);

    // API 2: synthesize
    let req2 = TtsRequest::with_defaults(
        "hello world".to_string(),
        TtsModel::ElevenLabs,
        "voice-1".to_string(),
        "en".to_string(),
    )
    .expect("valid TtsRequest");
    println!("  synthesize                    : {:?}", client.synthesize(&req2).await);

    // API 3: detect_wake
    let audio = vec![0i16; 512];
    println!(
        "  detect_wake                   : {:?}",
        client.detect_wake(&audio).await
    );

    // API 4: start_listening
    println!(
        "  start_listening               : {:?}",
        client.start_listening().await
    );

    // API 5: stop_listening (但 listening=false, 守门优先)
    println!(
        "  stop_listening                : {:?}",
        client.stop_listening().await
    );

    // API 6: stream_audio
    let vad_result = apeireth_sdk_voice::VadResult::new(
        true,
        VadAlgorithm::Energy,
        0.95,
        std::time::Duration::from_millis(1000),
        std::time::Duration::from_millis(500),
    );
    println!(
        "  stream_audio                  : {:?}",
        client.stream_audio(&vad_result).await
    );
    println!();

    // 10) stub_status 工具
    println!("[§10 stub_status 工具 (R21 续真接后删)]");
    let status = client.stub_status();
    println!("  stub_mode           : {}", status.stub_mode);
    println!("  platform            : {}", status.platform);
    println!("  schema_version      : {}", status.schema_version);
    println!("  api_key_set         : {}", status.api_key_set);
    println!("  listening           : {}", status.listening);
    println!("  default_wake_word   : {}", status.default_wake_word);
    println!("  wake_word_category  : {:?}", status.wake_word_category);
    println!("  stt_model           : {:?}", status.stt_model);
    println!("  tts_model           : {:?}", status.tts_model);
    println!("  vad_algorithm       : {:?}", status.vad_algorithm);
    println!();

    // 11) VoiceConfig 5 段演示
    println!("[§11 VoiceConfig 5 段演示 (K-1 强校验)]");
    let config = VoiceConfig::default_apeireth();
    println!("  wake.category   = {:?}", config.wake.category);
    println!("  wake.keyword    = {}", config.wake.keyword);
    println!("  stt             = {:?}", config.stt);
    println!("  tts             = {:?}", config.tts);
    println!("  vad.algorithm   = {:?}", config.vad.algorithm);
    println!("  audio.format    = {}", config.audio.format);
    println!("  audio.sample_rate = {}", config.audio.sample_rate);
    println!("  is_default_apeireth = {}", config.is_default_apeireth());
    println!();

    println!("=== demo 完 (R21 续真接: 整合 #2 sub-agent 1 commit 落地, 改 STUB_MODE=false + 接 @anthropic-ai/voice) ===");
}
