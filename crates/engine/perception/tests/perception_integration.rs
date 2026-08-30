//! Integration tests for `apeireth-perception` multimodal backends.
//!
//! 验证:
//! 1. `WhisperHttpBackend` + `StaticCredentials` 真实装配路径
//! 2. `XcapVisionBackend` 与 `NoopVisionBackend` 真实装配路径
//! 3. 异步并发安全性与 `Arc<dyn VoiceBackend>` / `Arc<dyn VisionBackend>` 注入能力

use std::sync::Arc;

use apeireth_perception::vision::{NoopVisionBackend, XcapVisionBackend, XcapVisionConfig};
use apeireth_perception::voice::{
    hex_decode_audio, split_pcm16_frames, Pcm16Buffer, WhisperHttpBackend, WhisperHttpConfig,
    PCM16_FRAME_SAMPLES,
};
use apeireth_plugin::credentials::StaticCredentials;
use apeireth_plugin::perception_backend::{
    AudioBuffer, LangHint, PerceptionBackendError, VisionBackend, VoiceBackend,
};

#[tokio::test]
async fn perception_voice_and_vision_backends_wire_cleanly() {
    // 1. 装配 Voice backend
    let creds = Arc::new(StaticCredentials::new().with(
        "provider.whisper.api_key",
        "sk-test-fake-key-12345678901234",
    ));
    let voice: Arc<dyn VoiceBackend> = Arc::new(WhisperHttpBackend::openai(creds));
    assert_eq!(voice.name(), "whisper_http");
    assert!(voice.ping().await.is_ok());

    // 2. 装配 Vision backend
    let vision: Arc<dyn VisionBackend> = Arc::new(XcapVisionBackend::default_monitor());
    assert_eq!(vision.name(), "xcap_vision");
    assert!(vision.ping().await.is_ok());

    // 3. 装配 Noop Vision backend
    let noop_vision: Arc<dyn VisionBackend> = Arc::new(NoopVisionBackend);
    assert_eq!(noop_vision.name(), "noop_vision");
    assert!(noop_vision.ping().await.is_err());
}

#[tokio::test]
async fn perception_voice_fails_on_empty_audio_safely() {
    let creds = Arc::new(
        StaticCredentials::new().with("provider.whisper.api_key", "sk-1234567890abcdef123456"),
    );
    let voice = WhisperHttpBackend::openai(creds);
    let res = voice
        .transcribe(AudioBuffer::empty(), LangHint::auto())
        .await;
    assert!(matches!(res, Err(PerceptionBackendError::Audio(_))));
}

#[tokio::test]
async fn perception_vision_captures_fail_closed_in_headless() {
    let vision = XcapVisionBackend::new(XcapVisionConfig {
        monitor_index: 999,
        format: "png".to_string(),
    });
    let res = vision.capture().await;
    assert!(matches!(
        res,
        Err(PerceptionBackendError::BackendUnavailable(_))
    ));
}

#[test]
fn pcm16_split_and_hex_decode_are_pure() {
    let buf = Pcm16Buffer::from_samples(vec![3i16; PCM16_FRAME_SAMPLES + 8]).unwrap();
    let frames = split_pcm16_frames(&buf.samples, buf.sample_rate, buf.channels);
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[1].samples.len(), 8);
    assert_eq!(hex_decode_audio("494433").unwrap(), b"ID3");
}
