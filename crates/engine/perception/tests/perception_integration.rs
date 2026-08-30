//! Integration tests for `apeireth-perception` multimodal backends.
//!
//! 验证:
//! 1. `WhisperHttpBackend` + `StaticCredentials` 真实装配路径
//! 2. `XcapVisionBackend` 与 `NoopVisionBackend` 真实装配路径
//! 3. 异步并发安全性与 `Arc<dyn VoiceBackend>` / `Arc<dyn VisionBackend>` 注入能力

use std::sync::Arc;

use apeireth_core::kernel::SessionId;
use apeireth_perception::vision::{NoopVisionBackend, XcapVisionBackend, XcapVisionConfig};
use apeireth_perception::voice::{WhisperHttpBackend, WhisperHttpConfig};
use apeireth_perception::{
    default_attention_threshold, PerceptionModality, PerceptionOwner, SignalSource,
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
fn enabled_owner_runs_end_to_end_multimodal_pipeline() {
    let mut owner = PerceptionOwner::enabled(SessionId::new()).with_top_k(3);
    owner.ingest_text(SignalSource::Cli, "hello world", 0.6);
    owner.ingest_text(SignalSource::Internal, "noise", 0.1);
    owner.ingest_voice(SignalSource::Http, "say hi", 0.85);
    owner.ingest_vision(SignalSource::PyBridge, 1280, 720, Some("screen".into()));
    owner.ingest_tactile(SignalSource::Internal, -0.9);
    owner.ingest_command(SignalSource::Cli, "/status");

    let selected = owner.select();
    assert!(
        !selected.is_empty(),
        "pipeline should keep at least one event"
    );
    assert!(selected.len() <= 3);
    let threshold = default_attention_threshold();
    for event in &selected {
        assert!(event.attention_score >= threshold);
        assert!(!event.payload.is_null());
        assert!(matches!(
            event.source,
            PerceptionModality::Text
                | PerceptionModality::Voice
                | PerceptionModality::Vision
                | PerceptionModality::Tactile
                | PerceptionModality::Command
        ));
    }
}

#[test]
fn disabled_owner_is_the_default_production_path() {
    let mut owner = PerceptionOwner::default();
    owner.ingest_text(SignalSource::Cli, "must not leak", 1.0);
    assert!(owner.select().is_empty());
}
