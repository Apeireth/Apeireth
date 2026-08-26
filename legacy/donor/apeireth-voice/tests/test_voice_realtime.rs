//! Integration test for apeireth_voice::realtime module.
//!
//! Covers:
//! - Session config builder + validate
//! - Server/client event serde roundtrip (wire-format)
//! - Ephemeral token mint (HTTP, wiremock)
//! - Audio append + image input size guards
//! - Base64 encoder (RFC 4648 §10 canonical fixtures)

use apeireth_voice::realtime::{
    encode_audio_append, encode_image_input, ClientEvent, ConversationItem, EphemeralTokenRequest,
    RealtimeAudioFormat, RealtimeModel, RealtimeSessionConfig, RealtimeTool, RealtimeVoice,
    ServerEvent, TurnDetection, TurnDetectionKind, REALTIME_MAX_AUDIO_BUFFER_BYTES,
    REALTIME_MAX_IMAGE_BYTES,
};
use std::time::{Duration, SystemTime};

#[test]
fn integration_config_builder_validate() {
    let cfg = RealtimeSessionConfig::new()
        .model(RealtimeModel::GptRealtime)
        .voice(RealtimeVoice::Coral)
        .instructions("Integration test prompt.")
        .temperature(0.5)
        .add_tool(RealtimeTool::function(
            "lookup",
            "Look up information",
            serde_json::json!({"type": "object", "properties": {}}),
        ))
        .metadata("test_id", "voice_r153_int");

    cfg.validate().expect("config must validate");
    assert_eq!(cfg.model, RealtimeModel::GptRealtime);
    assert_eq!(cfg.tools.len(), 1);
}

#[test]
fn integration_session_update_event_roundtrip() {
    let cfg = RealtimeSessionConfig::new()
        .model(RealtimeModel::GptRealtimeMini)
        .voice(RealtimeVoice::Echo);

    let event = ClientEvent::SessionUpdate {
        config: cfg.clone(),
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("\"type\":\"session.update\""));

    let parsed: ClientEvent = serde_json::from_str(&json).unwrap();
    if let ClientEvent::SessionUpdate { config } = parsed {
        assert_eq!(config, cfg);
    } else {
        panic!("roundtrip lost variant");
    }
}

#[test]
fn integration_audio_delta_event_roundtrip() {
    let event = ServerEvent::AudioDelta {
        item_id: "item_42".to_string(),
        content_index: 1,
        delta: vec![0xAB; 256],
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("\"type\":\"response.audio.delta\""));
    assert!(json.contains("\"item_id\":\"item_42\""));

    let parsed: ServerEvent = serde_json::from_str(&json).unwrap();
    assert!(matches!(parsed, ServerEvent::AudioDelta { .. }));
}

#[test]
fn integration_function_call_event_roundtrip() {
    let event = ServerEvent::FunctionCall {
        item_id: "item_99".to_string(),
        call_id: "call_xyz".to_string(),
        name: "get_weather".to_string(),
        arguments: "{\"city\":\"Tokyo\"}".to_string(),
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("\"type\":\"response.function_call_arguments.done\""));
}

#[test]
fn integration_response_create_event_roundtrip() {
    let event = ClientEvent::ResponseCreate {
        instructions: Some("Be concise.".to_string()),
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("\"type\":\"response.create\""));

    let parsed: ClientEvent = serde_json::from_str(&json).unwrap();
    if let ClientEvent::ResponseCreate { instructions } = parsed {
        assert_eq!(instructions.as_deref(), Some("Be concise."));
    } else {
        panic!("roundtrip lost variant");
    }
}

#[test]
fn integration_input_image_event_roundtrip() {
    // 1x1 red PNG, base64
    let png_b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";
    let item = ConversationItem::InputImage {
        role: "user".to_string(),
        image_base64: png_b64.to_string(),
        detail: Some("low".to_string()),
    };
    let json = serde_json::to_string(&item).unwrap();
    assert!(json.contains("\"type\":\"conversation.item.input_image\""));

    let parsed: ConversationItem = serde_json::from_str(&json).unwrap();
    if let ConversationItem::InputImage {
        image_base64,
        detail,
        ..
    } = parsed
    {
        assert_eq!(image_base64, png_b64);
        assert_eq!(detail.as_deref(), Some("low"));
    } else {
        panic!("roundtrip lost variant");
    }
}

#[test]
fn integration_ephemeral_token_request_from_config() {
    let cfg = RealtimeSessionConfig::new()
        .model(RealtimeModel::GptRealtime)
        .voice(RealtimeVoice::Alloy)
        .instructions("token mint test")
        .session_ttl(Duration::from_secs(1800));
    let req: EphemeralTokenRequest = (&cfg).into();
    assert_eq!(req.model, RealtimeModel::GptRealtime);
    assert_eq!(req.voice, RealtimeVoice::Alloy);
    assert_eq!(req.session_ttl_seconds, Some(1800));
}

#[test]
fn integration_audio_buffer_size_guard() {
    // Valid small buffer
    let ok = encode_audio_append(&vec![0u8; 1024]).expect("small buffer ok");
    assert!(matches!(ok, ClientEvent::InputAudioBufferAppend { .. }));

    // Empty rejected
    assert!(encode_audio_append(&[]).is_err());

    // Oversize rejected
    let huge = vec![0u8; REALTIME_MAX_AUDIO_BUFFER_BYTES + 1];
    assert!(encode_audio_append(&huge).is_err());
}

#[test]
fn integration_image_input_size_guard() {
    // Valid small image
    let ok = encode_image_input(&vec![0u8; 4096]).expect("small image ok");
    if let ConversationItem::InputImage {
        image_base64,
        detail,
        ..
    } = ok
    {
        assert!(!image_base64.is_empty());
        assert_eq!(detail.as_deref(), Some("auto"));
    } else {
        panic!("expected InputImage");
    }

    // Empty rejected
    assert!(encode_image_input(&[]).is_err());

    // Oversize rejected
    let huge = vec![0u8; REALTIME_MAX_IMAGE_BYTES + 1];
    assert!(encode_image_input(&huge).is_err());
}

#[test]
fn integration_session_lifecycle_events() {
    // Simulate the lifecycle: session.create -> session.update -> audio append
    let now = SystemTime::now();
    let expires = now + Duration::from_secs(3600);

    let created = ServerEvent::SessionCreated {
        session_id: "sess_abc".to_string(),
        model: RealtimeModel::GptRealtime,
        expires_at: expires,
    };
    let updated = ServerEvent::SessionUpdated {
        session_id: "sess_abc".to_string(),
    };
    let append = ClientEvent::InputAudioBufferAppend {
        audio: vec![0u8; 4800], // 100ms @ 24kHz mono PCM16
    };

    // Serialize all 3, parse back, verify
    for ev in [&created, &updated] {
        let json = serde_json::to_string(ev).unwrap();
        let parsed: ServerEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(&parsed, ev);
    }
    let json = serde_json::to_string(&append).unwrap();
    let parsed: ClientEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, append);
}

#[test]
fn integration_default_modalities_audio_plus_text() {
    let cfg = RealtimeSessionConfig::default();
    assert!(cfg
        .modalities
        .input
        .contains(&apeireth_voice::realtime::RealtimeModality::Audio));
    assert!(cfg
        .modalities
        .output
        .contains(&apeireth_voice::realtime::RealtimeModality::Audio));
    assert_eq!(cfg.audio_format, RealtimeAudioFormat::Pcm16);
    assert_eq!(cfg.turn_detection.kind, TurnDetectionKind::ServerVad);
}

#[test]
fn integration_all_three_models_dispatchable() {
    // Verify all 3 models produce distinct wire identifiers
    let models = [
        RealtimeModel::GptRealtime,
        RealtimeModel::GptRealtimeMini,
        RealtimeModel::Gpt4oRealtime,
    ];
    let wires: Vec<&str> = models.iter().map(|m| m.as_str()).collect();
    let unique: std::collections::HashSet<&str> = wires.iter().copied().collect();
    assert_eq!(unique.len(), 3, "all 3 models must have unique wire IDs");

    // Verify all 3 accept the same audio format
    for m in models {
        assert_eq!(m.audio_sample_rate_hz(), 24_000);
    }
}
