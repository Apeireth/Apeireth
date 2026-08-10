//! # apeireth-voice stub demo (R20 阶段 1 续)
//!
//! 演示 8 stub 工具返 `VoiceError::NotImplemented` + 1 stub_status 工具返 STUB 状态.
//! **R20 阶段 3 整合 #2 sub-agent 改 STUB_MODE = false + 真接 picovoice SDK 后, 本 demo
//! 会被替换成真录音 / 真唤醒词检测 demo.**
//!
//! ## 运行
//!
//! ```bash
//! cargo run --manifest-path crates/apeireth-voice/Cargo.toml --example voice_stub_demo
//! ```

use apeireth_voice::{
    is_stub_mode, validate_tool_call, AudioFrame, StubAudioStream, AudioStreamSource, VoiceConfig,
    VoiceError, VoiceRecorder, VoiceSdk, VoiceWake, WakeWordType, SUPPORTED_WAKE_WORDS,
    TOOL_WHITELIST, TOOL_WHITELIST_COUNT, VOICE_DEFAULT_KEYWORD, VOICE_FRAME_LENGTH,
    VOICE_SAMPLE_RATE_HZ, VOICE_SCHEMA_VERSION, PLATFORM_NAME,
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("=== apeireth-voice stub demo (R20 阶段 1 续) ===");
    println!();

    // 1) 编译期 hardcode 守门 (K-1 强校验)
    println!("[§1 编译期 hardcode]");
    println!("  VOICE_SCHEMA_VERSION  = {}", VOICE_SCHEMA_VERSION);
    println!("  PLATFORM_NAME         = {}", PLATFORM_NAME);
    println!("  STUB_MODE             = {}", is_stub_mode());
    println!("  VOICE_SAMPLE_RATE_HZ  = {}", VOICE_SAMPLE_RATE_HZ);
    println!("  VOICE_FRAME_LENGTH    = {}", VOICE_FRAME_LENGTH);
    println!("  VOICE_DEFAULT_KEYWORD = {}", VOICE_DEFAULT_KEYWORD);
    println!();

    // 2) 5 WakeWordType 枚举
    println!("[§2 5 WakeWordType 枚举]");
    for w in SUPPORTED_WAKE_WORDS {
        println!("  {:?} -> \"{}\"", w, w.as_str());
    }
    println!();

    // 3) 9 工具白名单
    println!("[§3 9 工具白名单 (m3 防御)]");
    println!("  TOOL_WHITELIST_COUNT = {}", TOOL_WHITELIST_COUNT);
    for (i, tool) in TOOL_WHITELIST.iter().enumerate() {
        println!("  [{:>2}] {}", i + 1, tool);
    }
    println!();

    // 4) m3 防御: validate_tool_call 测试
    println!("[§4 m3 防御: validate_tool_call]");
    let args = serde_json::json!({});
    let valid = validate_tool_call("apeireth_voice_stub_status", &args);
    println!("  白名单内工具: {:?}", valid);
    let invalid = validate_tool_call("apeireth_voice_bogus_tool", &args);
    println!("  非白名单工具: {:?}", invalid);
    println!();

    // 5) 8 stub 工具返 NotImplemented
    println!("[§5 8 stub 工具返 NotImplemented]");
    let mut sdk = VoiceSdk::new(VoiceConfig::default()).expect("VoiceSdk::new must succeed in STUB mode");
    let frame = AudioFrame::new(vec![0i16; VOICE_FRAME_LENGTH as usize]);

    println!("  wake_word_detect : {:?}", sdk.wake_word_detect(&frame).await);
    println!("  record_audio     : {:?}", sdk.record_audio(5).await);
    println!("  transcribe       : {:?}", sdk.transcribe(&[0i16; 512]).await);
    println!("  synthesize       : {:?}", sdk.synthesize("hello apeireth").await);
    // list_keywords 是编译期常量, 不返 NotImplemented
    match sdk.list_keywords() {
        Ok(kws) => println!("  list_keywords    : Ok({} keywords)", kws.len()),
        Err(e) => println!("  list_keywords    : Err({:?})", e),
    }
    println!("  load_model       : {:?}", sdk.load_model(WakeWordType::Apeireth).await);
    println!("  unload_model     : {:?}", sdk.unload_model().await);
    println!("  audio_stream     : {:?}", sdk.audio_stream().await);
    println!();

    // 6) stub_status (额外 1 工具)
    println!("[§6 stub_status 工具 (R20 阶段 3 后删)]");
    let status = sdk.stub_status().expect("stub_status must succeed");
    println!("  stub_mode        : {}", status.stub_mode);
    println!("  platform         : {}", status.platform);
    println!("  default_keyword  : {}", status.default_keyword);
    println!("  schema_version   : {}", status.schema_version);
    println!();

    // 7) VoiceWake / VoiceRecorder / StubAudioStream 占位
    println!("[§7 VoiceWake / VoiceRecorder / StubAudioStream 占位]");
    let mut wake = VoiceWake::new(VoiceConfig::default(), WakeWordType::Apeireth)
        .expect("VoiceWake::new must succeed in STUB mode");
    println!("  VoiceWake::start  : {:?}", wake.start().await);

    let mut recorder = VoiceRecorder::new(VoiceConfig::default())
        .expect("VoiceRecorder::new must succeed in STUB mode");
    println!("  VoiceRecorder::start : {:?}", recorder.start("apeireth").await);

    let mut stream = StubAudioStream::new();
    println!("  StubAudioStream::next_frame : {:?}", stream.next_frame().await);
    stream.close().await.expect("close must succeed");
    println!();

    println!("=== demo 完 (R20 阶段 3 续: 整合 #2 sub-agent 1 commit 落地, 改 STUB_MODE=false + 接 picovoice) ===");
}
