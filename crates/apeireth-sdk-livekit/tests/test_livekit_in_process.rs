//! # apeireth-sdk-livekit in-process stub test (R20 阶段 4 效果, 14 测试)
//!
//! 验证 STUB 模式 5 守门 + 9 额外行为, 防止整合 #2 sub-agent 改 STUB_MODE=false 时漏防.
//!
//! ## Fixture 列表 (per 任务 K-1 强校验 4 条 + 10 额外)
//!
//! - **Fixture 1**: 5 RoomState 状态机 (Disconnected / Connecting / Connected / Reconnecting / DisconnectedAlt)
//! - **Fixture 2**: 4 K-1 强校验 (API Key / Secret / Room Name / wss:// URL)
//! - **Fixture 3**: 6 核心 API 全返 NotImplemented (connect / disconnect / publish_track / subscribe / set_camera_enabled / set_microphone_enabled)
//! - **Fixture 4**: 8 RoomEvent 类型 (ParticipantConnected / ParticipantDisconnected / TrackSubscribed / TrackUnsubscribed / ActiveSpeakersChanged / ConnectionStateChanged / DataReceived / Reconnected)
//! - **Fixture 5**: 7 TOOL_WHITELIST (6 核心 API + 1 stub_status)
//! - **额外 1-2**: 5 哲学 anchor + 8 项不修改承诺守门 (5 K-1 字样)
//! - **额外 3**: ActiveSpeakersChanged 事件 (5 说话者)
//! - **额外 4**: EventEmitter 多订阅者 fan-out
//! - **额外 5**: list_apis / list_room_states / list_events 工具
//! - **额外 6**: 默认 url = `wss://livekit.example.com`
//! - **额外 7**: stub_status 报告完整状态
//! - **额外 8**: set_api_key + set_api_secret 双 K-1 守门
//! - **额外 9**: K-1 字样 守门 (源码含 "apeireth" / "livekit" / "stub" / "wss" / "must-do")

use apeireth_sdk_livekit::{
    validate_tool_call, ConnectionQuality, EventEmitter, LiveKitClient, LiveKitClientImpl,
    LiveKitError, Participant, Permission, Room, RoomEvent, RoomOptions, RoomState, Track,
    TrackKind, TrackSource, CORE_API_COUNT, DEFAULT_LIVEKIT_URL, EVENT_CHANNEL_CAPACITY,
    K1_STRONG_VALIDATION_COUNT, LIVEKIT_SCHEMA_VERSION, PLATFORM_NAME, ROOM_EVENT_COUNT,
    ROOM_STATE_COUNT, STUB_MODE, SUPPORTED_CONNECTION_QUALITIES, SUPPORTED_PERMISSIONS,
    SUPPORTED_ROOM_EVENTS, SUPPORTED_ROOM_STATES, SUPPORTED_TRACK_KINDS,
    SUPPORTED_TRACK_SOURCES, TOOL_WHITELIST, TOOL_WHITELIST_COUNT,
};

/// Fixture 1: 5 RoomState 状态机
#[test]
fn fixture_1_room_state_5_states() {
    assert_eq!(SUPPORTED_ROOM_STATES.len(), 5, "5 状态机");
    assert_eq!(RoomState::COUNT, 5);
    assert_eq!(ROOM_STATE_COUNT, 5);
    assert_eq!(RoomState::Disconnected.as_str(), "disconnected");
    assert_eq!(RoomState::Connecting.as_str(), "connecting");
    assert_eq!(RoomState::Connected.as_str(), "connected");
    assert_eq!(RoomState::Reconnecting.as_str(), "reconnecting");
    assert_eq!(RoomState::DisconnectedAlt.as_str(), "disconnected_alt");
    // 5 状态机互不相同
    for i in 0..SUPPORTED_ROOM_STATES.len() {
        for j in (i + 1)..SUPPORTED_ROOM_STATES.len() {
            assert_ne!(SUPPORTED_ROOM_STATES[i], SUPPORTED_ROOM_STATES[j]);
        }
    }
}

/// Fixture 2: 4 K-1 强校验 (API Key + Secret + Room Name + wss:// URL)
#[test]
fn fixture_2_4_k1_strong_validations() {
    // K-1 #1: API Key
    assert!(matches!(
        LiveKitError::validate_api_key(""),
        Err(LiveKitError::ApiKeyMissing)
    ));
    assert!(matches!(
        LiveKitError::validate_api_key("short"),
        Err(LiveKitError::ApiKeyInvalid(_))
    ));
    assert!(LiveKitError::validate_api_key("API12345678").is_ok());

    // K-1 #2: API Secret
    assert!(matches!(
        LiveKitError::validate_api_secret(""),
        Err(LiveKitError::ApiSecretMissing)
    ));
    assert!(matches!(
        LiveKitError::validate_api_secret("too_short"),
        Err(LiveKitError::ApiSecretInvalid(_))
    ));
    assert!(LiveKitError::validate_api_secret("abcdef1234567890abcdef1234567890").is_ok());

    // K-1 #3: Room Name
    assert!(matches!(
        LiveKitError::validate_room_name(""),
        Err(LiveKitError::RoomNameEmpty)
    ));
    assert!(matches!(
        LiveKitError::validate_room_name("room name with spaces"),
        Err(LiveKitError::RoomNameInvalid(_))
    ));
    assert!(LiveKitError::validate_room_name("my-room-1").is_ok());

    // K-1 #4: wss:// URL
    assert!(matches!(
        LiveKitError::validate_url("http://example.com"),
        Err(LiveKitError::InvalidUrl(_))
    ));
    assert!(matches!(
        LiveKitError::validate_url("ws://example.com"),
        Err(LiveKitError::InvalidUrl(_))
    ));
    assert!(LiveKitError::validate_url("wss://livekit.example.com").is_ok());
    assert_eq!(K1_STRONG_VALIDATION_COUNT, 4);
}

/// Fixture 3: 6 核心 API 全返 NotImplemented
#[tokio::test]
async fn fixture_3_6_core_apis_return_not_implemented() {
    let mut client = LiveKitClientImpl::new();
    client
        .set_api_key("API12345678".to_string())
        .expect("valid API key");
    client
        .set_api_secret("abcdef1234567890abcdef1234567890".to_string())
        .expect("valid API secret");

    // API 1: connect
    let r1 = client
        .connect(DEFAULT_LIVEKIT_URL, "stub.jwt.token")
        .await;
    assert!(
        matches!(r1, Err(LiveKitError::NotImplemented("connect"))),
        "connect must return NotImplemented, got {:?}",
        r1
    );

    // API 2: disconnect
    let r2 = client.disconnect().await;
    assert!(
        matches!(r2, Err(LiveKitError::NotImplemented("disconnect"))),
        "disconnect must return NotImplemented, got {:?}",
        r2
    );

    // API 3: publish_track (RoomDisconnected 因为 connected=false, 守门优先)
    let track = Track::new(TrackKind::Video, TrackSource::Camera);
    let r3 = client.publish_track(&track).await;
    assert!(
        matches!(r3, Err(LiveKitError::RoomDisconnected(_))),
        "publish_track must check connected, got {:?}",
        r3
    );

    // API 4: subscribe (RoomDisconnected 守门)
    let r4 = client.subscribe("TR_xxxxxxxxxxxxx").await;
    assert!(
        matches!(r4, Err(LiveKitError::RoomDisconnected(_))),
        "subscribe must check connected, got {:?}",
        r4
    );

    // API 5: set_camera_enabled
    let r5 = client.set_camera_enabled(true).await;
    assert!(
        matches!(r5, Err(LiveKitError::NotImplemented("set_camera_enabled"))),
        "set_camera_enabled must return NotImplemented, got {:?}",
        r5
    );

    // API 6: set_microphone_enabled
    let r6 = client.set_microphone_enabled(false).await;
    assert!(
        matches!(r6, Err(LiveKitError::NotImplemented("set_microphone_enabled"))),
        "set_microphone_enabled must return NotImplemented, got {:?}",
        r6
    );

    assert_eq!(CORE_API_COUNT, 6);
}

/// Fixture 4: 8 RoomEvent 事件类型
#[test]
fn fixture_4_8_room_events() {
    assert_eq!(SUPPORTED_ROOM_EVENTS.len(), 8, "8 事件");
    assert_eq!(ROOM_EVENT_COUNT, 8);
    let expected_events = [
        "participant_connected",
        "participant_disconnected",
        "track_subscribed",
        "track_unsubscribed",
        "active_speakers_changed",
        "connection_state_changed",
        "data_received",
        "reconnected",
    ];
    for evt in expected_events {
        assert!(SUPPORTED_ROOM_EVENTS.contains(&evt), "must contain {evt}");
    }
}

/// Fixture 5: 7 TOOL_WHITELIST (6 核心 API + 1 stub_status)
#[test]
fn fixture_5_tool_whitelist_has_7_tools() {
    assert_eq!(TOOL_WHITELIST.len(), 7);
    assert_eq!(TOOL_WHITELIST_COUNT, 7);
    let expected = [
        "apeireth_livekit_connect",
        "apeireth_livekit_disconnect",
        "apeireth_livekit_publish_track",
        "apeireth_livekit_subscribe",
        "apeireth_livekit_set_camera_enabled",
        "apeireth_livekit_set_microphone_enabled",
        "apeireth_livekit_stub_status",
    ];
    for tool in expected {
        assert!(TOOL_WHITELIST.contains(&tool), "TOOL_WHITELIST must contain {tool}");
    }
}

/// 额外 1: STUB_MODE == true 守门 (K-1 强校验 #4)
#[test]
fn extra_1_stub_mode_is_true() {
    let _ = STUB_MODE;
    assert_eq!(apeireth_sdk_livekit::is_stub_mode(), STUB_MODE);
    assert!(apeireth_sdk_livekit::is_stub_mode());
}

/// 额外 2: 5 K-1 字样守门 (源码含 "apeireth" / "livekit" / "stub" / "wss" / "must-do")
#[test]
fn extra_2_5_k1_keywords_in_source() {
    let source = include_str!("../src/lib.rs");
    assert!(source.contains("apeireth"), "must-do: 源码必须出现 'apeireth' (K-1 字样 #1)");
    assert!(source.contains("livekit"), "must-do: 源码必须出现 'livekit' (K-1 字样 #2)");
    assert!(source.contains("stub"), "must-do: 源码必须出现 'stub' (K-1 字样 #3)");
    assert!(source.contains("wss"), "must-do: 源码必须出现 'wss' (K-1 字样 #4)");
    assert!(
        source.contains("must-do") || source.contains("MUST"),
        "must-do: 源码必须出现 'must-do' 守门字样 (K-1 字样 #5)"
    );
    // 编译期常量也守
    assert_eq!(PLATFORM_NAME, "apeireth");
    assert_eq!(LIVEKIT_SCHEMA_VERSION, "1");
}

/// 额外 3: ActiveSpeakersChanged 事件携带 5 说话者
#[test]
fn extra_3_active_speakers_changed_with_5_speakers() {
    let speakers = vec![
        "PA_speaker_1".to_string(),
        "PA_speaker_2".to_string(),
        "PA_speaker_3".to_string(),
        "PA_speaker_4".to_string(),
        "PA_speaker_5".to_string(),
    ];
    let event = RoomEvent::ActiveSpeakersChanged { speakers: speakers.clone() };
    assert_eq!(event.type_str(), "active_speakers_changed");
    assert!(!event.is_state_event());
    if let RoomEvent::ActiveSpeakersChanged { speakers: s } = &event {
        assert_eq!(s.len(), 5);
    } else {
        panic!("event must be ActiveSpeakersChanged");
    }
}

/// 额外 4: EventEmitter 多订阅者 fan-out (8 事件)
#[tokio::test]
async fn extra_4_event_emitter_fan_out_8_events() {
    let emitter = EventEmitter::new(EVENT_CHANNEL_CAPACITY);
    let mut rx1 = emitter.subscribe();
    let mut rx2 = emitter.subscribe();
    assert_eq!(emitter.receiver_count(), 2);

    // 发射 8 个不同事件, 每个事件都被两个订阅者收到
    let events = vec![
        RoomEvent::Reconnected { disconnected_at: None },
        RoomEvent::ParticipantDisconnected { participant_sid: "PA_1".to_string() },
        RoomEvent::ConnectionStateChanged {
            previous: RoomState::Disconnected,
            current: RoomState::Connected,
        },
        RoomEvent::DataReceived {
            participant_sid: "PA_2".to_string(),
            payload: vec![1, 2, 3],
            reliable: true,
        },
        RoomEvent::ActiveSpeakersChanged {
            speakers: vec!["PA_1".to_string()],
        },
        RoomEvent::TrackSubscribed {
            track_sid: "TR_1".to_string(),
            participant_sid: "PA_1".to_string(),
            source: TrackSource::Camera,
        },
        RoomEvent::TrackUnsubscribed {
            track_sid: "TR_1".to_string(),
            participant_sid: "PA_1".to_string(),
        },
        RoomEvent::ParticipantConnected {
            participant: Participant::new("user-1").unwrap(),
        },
    ];
    assert_eq!(events.len(), 8);
    for evt in events {
        let sent = emitter.emit(evt.clone()).expect("emit must succeed");
        assert_eq!(sent, 2, "must broadcast to 2 subscribers");
        let r1 = rx1.try_recv().expect("rx1 must receive");
        let r2 = rx2.try_recv().expect("rx2 must receive");
        assert_eq!(r1, evt);
        assert_eq!(r2, evt);
    }
}

/// 额外 5: list_apis / list_room_states / list_events 工具
#[test]
fn extra_5_list_helpers() {
    // 6 核心 API
    let apis = LiveKitClientImpl::list_apis();
    assert_eq!(apis.len(), CORE_API_COUNT);
    assert_eq!(apis.len(), 6);
    assert!(apis.contains(&"apeireth_livekit_connect"));
    assert!(apis.contains(&"apeireth_livekit_set_microphone_enabled"));
    // 不应包含 stub_status
    assert!(!apis.contains(&"apeireth_livekit_stub_status"));

    // 5 RoomState
    let states = LiveKitClientImpl::list_room_states();
    assert_eq!(states.len(), ROOM_STATE_COUNT);
    assert_eq!(states.len(), 5);

    // 8 RoomEvent
    let events = LiveKitClientImpl::list_events();
    assert_eq!(events.len(), ROOM_EVENT_COUNT);
    assert_eq!(events.len(), 8);
}

/// 额外 6: 默认 url = `wss://livekit.example.com` (K-1 强校验 #4 守门)
#[test]
fn extra_6_default_url_is_wss() {
    assert_eq!(DEFAULT_LIVEKIT_URL, "wss://livekit.example.com");
    assert!(DEFAULT_LIVEKIT_URL.starts_with("wss://"));
    let client = LiveKitClientImpl::new();
    assert!(client.url().starts_with("wss://"));
    assert_eq!(client.url(), DEFAULT_LIVEKIT_URL);
}

/// 额外 7: stub_status 报告完整状态
#[test]
fn extra_7_stub_status_reports_complete_state() {
    let mut client = LiveKitClientImpl::new();
    let status_before = client.stub_status();
    assert!(status_before.stub_mode);
    assert_eq!(status_before.platform, "apeireth");
    assert!(status_before.url.starts_with("wss://"));
    assert_eq!(status_before.schema_version, "1");
    assert!(!status_before.api_key_set);
    assert!(!status_before.api_secret_set);
    assert!(!status_before.connected);
    assert!(status_before.room_state.is_none());

    // 设置 api key + secret 后
    client.set_api_key("API12345678".to_string()).expect("valid");
    client
        .set_api_secret("abcdef1234567890abcdef1234567890".to_string())
        .expect("valid");
    let status_after = client.stub_status();
    assert!(status_after.api_key_set);
    assert!(status_after.api_secret_set);
    assert!(!status_after.connected);
}

/// 额外 8: set_api_key + set_api_secret 双 K-1 守门
#[test]
fn extra_8_set_api_key_secret_dual_k1() {
    let mut client = LiveKitClientImpl::new();
    // K-1 #1 守门: API key
    assert!(matches!(
        client.set_api_key(String::new()),
        Err(LiveKitError::ApiKeyMissing)
    ));
    assert!(!client.has_api_key());

    // K-1 #2 守门: API secret
    assert!(matches!(
        client.set_api_secret(String::new()),
        Err(LiveKitError::ApiSecretMissing)
    ));
    assert!(!client.has_api_secret());

    // 有效值
    client
        .set_api_key("API12345678".to_string())
        .expect("valid API key");
    assert!(client.has_api_key());
    client
        .set_api_secret("abcdef1234567890abcdef1234567890".to_string())
        .expect("valid API secret");
    assert!(client.has_api_secret());
}

/// 额外 9: validate_tool_call 接受白名单拒绝非白名单
#[test]
fn extra_9_validate_tool_call_whitelist() {
    let args = serde_json::json!({});
    // 白名单内 OK
    for tool in TOOL_WHITELIST {
        assert!(
            validate_tool_call(tool, &args).is_ok(),
            "{tool} should be whitelisted"
        );
    }
    // 非白名单 Err(ToolNotWhitelisted)
    let err = validate_tool_call("apeireth_livekit_bogus", &args).unwrap_err();
    assert!(matches!(err, LiveKitError::ToolNotWhitelisted(_)));
}

/// 额外 10: Room 创建 + 5 状态流转
#[test]
fn extra_10_room_state_transitions() {
    let mut room = Room::new("my-room-1", RoomOptions::default()).expect("valid room");
    assert_eq!(room.state(), RoomState::Disconnected);
    assert!(!room.is_connected());

    // 5 状态流转
    room.set_state(RoomState::Connecting);
    assert_eq!(room.state(), RoomState::Connecting);
    assert!(!room.is_connected());
    assert!(!room.state().can_publish());

    room.set_state(RoomState::Connected);
    assert_eq!(room.state(), RoomState::Connected);
    assert!(room.is_connected());
    assert!(room.state().can_publish());
    assert!(room.state().can_subscribe());

    room.set_state(RoomState::Reconnecting);
    assert_eq!(room.state(), RoomState::Reconnecting);
    assert!(!room.is_connected());
    assert!(!room.state().can_publish());
    assert!(room.state().can_subscribe());

    room.set_state(RoomState::DisconnectedAlt);
    assert_eq!(room.state(), RoomState::DisconnectedAlt);
    assert!(room.state().is_terminal());
    assert!(!room.is_connected());

    room.set_state(RoomState::Disconnected);
    assert!(room.state().is_terminal());
}

/// 额外 11: Room 5 ConnectionQuality 枚举
#[test]
fn extra_11_connection_quality_5() {
    assert_eq!(SUPPORTED_CONNECTION_QUALITIES.len(), 5);
    assert_eq!(ConnectionQuality::COUNT, 5);
    assert_eq!(ConnectionQuality::Excellent.as_str(), "excellent");
    assert_eq!(ConnectionQuality::Good.as_str(), "good");
    assert_eq!(ConnectionQuality::Poor.as_str(), "poor");
    assert_eq!(ConnectionQuality::Lost.as_str(), "lost");
    assert_eq!(ConnectionQuality::Unknown.as_str(), "unknown");
}

/// 额外 12: Room 5 Permission 位
#[test]
fn extra_12_permission_5() {
    assert_eq!(SUPPORTED_PERMISSIONS.len(), 5);
    assert_eq!(Permission::COUNT, 5);
    assert_eq!(Permission::CanPublish.as_str(), "can_publish");
    assert_eq!(Permission::CanSubscribe.as_str(), "can_subscribe");
    assert_eq!(Permission::CanPublishData.as_str(), "can_publish_data");
    assert_eq!(Permission::CanUpdateMetadata.as_str(), "can_update_metadata");
    assert_eq!(Permission::Hidden.as_str(), "hidden");
}

/// 额外 13: Track 2 Kind + 5 Source
#[test]
fn extra_13_track_2_kinds_5_sources() {
    assert_eq!(SUPPORTED_TRACK_KINDS.len(), 2);
    assert_eq!(TrackKind::COUNT, 2);
    assert_eq!(SUPPORTED_TRACK_SOURCES.len(), 5);
    assert_eq!(TrackSource::COUNT, 5);
    // 2 + 5 互不相同
    assert_ne!(TrackKind::Video, TrackKind::Audio);
}

/// 额外 14: LiveKitClientImpl EventEmitter 通过 client.emitter() 共享
#[tokio::test]
async fn extra_14_client_emitter_shared() {
    let client = LiveKitClientImpl::new();
    let mut rx = client.emitter().subscribe();

    // 发射 1 个事件
    let event = RoomEvent::Reconnected { disconnected_at: None };
    let sent = client
        .emitter()
        .emit(event.clone())
        .expect("emit must succeed");
    assert_eq!(sent, 1);

    let received = rx.try_recv().expect("subscribe must receive");
    assert_eq!(received, event);
}
