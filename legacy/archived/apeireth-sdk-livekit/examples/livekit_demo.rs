//! # apeireth-sdk-livekit stub demo (R20 阶段 4 效果)
//!
//! 演示 6 核心 API 返 `LiveKitError::NotImplemented` + 5 RoomState 状态流转 + 8 RoomEvent 发射 + 4 K-1 强校验.
//! **R21 续真接 livekit-server SDK 后, 本 demo 会被替换成真 WebRTC demo.**
//!
//! ## 运行
//!
//! ```bash
//! cargo run --manifest-path crates/apeireth-sdk-livekit/Cargo.toml --example livekit_demo
//! ```

use apeireth_sdk_livekit::{
    event_publish_stub, is_stub_mode, validate_tool_call, ConnectionQuality, LiveKitClient,
    LiveKitClientImpl, LiveKitError, Participant, Room, RoomEvent, RoomOptions, RoomState, Track,
    TrackKind, TrackSource, CORE_API_COUNT, DEFAULT_LIVEKIT_URL, EVENT_CHANNEL_CAPACITY,
    K1_STRONG_VALIDATION_COUNT, LIVEKIT_SCHEMA_VERSION, PLATFORM_NAME, PROVIDER_NAME,
    ROOM_EVENT_COUNT, ROOM_STATE_COUNT, SUPPORTED_ROOM_EVENTS, SUPPORTED_ROOM_STATES, TOOL_WHITELIST,
    TOOL_WHITELIST_COUNT,
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("=== apeireth-sdk-livekit stub demo (R20 阶段 4 效果) ===");
    println!();

    // 1) 编译期 hardcode 守门 (K-1 强校验)
    println!("[§1 编译期 hardcode]");
    println!("  LIVEKIT_SCHEMA_VERSION  = {}", LIVEKIT_SCHEMA_VERSION);
    println!("  PLATFORM_NAME           = {}", PLATFORM_NAME);
    println!("  PROVIDER_NAME           = {}", PROVIDER_NAME);
    println!("  STUB_MODE               = {}", is_stub_mode());
    println!("  DEFAULT_LIVEKIT_URL     = {}", DEFAULT_LIVEKIT_URL);
    println!("  CORE_API_COUNT          = {}", CORE_API_COUNT);
    println!("  ROOM_STATE_COUNT        = {}", ROOM_STATE_COUNT);
    println!("  ROOM_EVENT_COUNT        = {}", ROOM_EVENT_COUNT);
    println!("  K1_STRONG_VALIDATION_COUNT = {}", K1_STRONG_VALIDATION_COUNT);
    println!("  EVENT_CHANNEL_CAPACITY  = {}", EVENT_CHANNEL_CAPACITY);
    println!();

    // 2) 5 RoomState 状态机
    println!("[§2 5 RoomState 状态机]");
    for state in SUPPORTED_ROOM_STATES {
        println!(
            "  {:?} -> \"{}\" (terminal={}, can_publish={}, can_subscribe={})",
            state,
            state.as_str(),
            state.is_terminal(),
            state.can_publish(),
            state.can_subscribe()
        );
    }
    println!();

    // 3) 8 RoomEvent 类型
    println!("[§3 8 RoomEvent 事件类型 (m3 防御)]");
    for (i, evt) in SUPPORTED_ROOM_EVENTS.iter().enumerate() {
        println!("  [{:>2}] {}", i + 1, evt);
    }
    println!();

    // 4) 7 TOOL_WHITELIST (6 核心 API + 1 stub_status)
    println!("[§4 7 工具白名单 (m3 防御)]");
    println!("  TOOL_WHITELIST_COUNT = {}", TOOL_WHITELIST_COUNT);
    for (i, tool) in TOOL_WHITELIST.iter().enumerate() {
        println!("  [{:>2}] {}", i + 1, tool);
    }
    println!();

    // 5) m3 防御: validate_tool_call 测试
    println!("[§5 m3 防御: validate_tool_call]");
    let args = serde_json::json!({});
    let valid = validate_tool_call("apeireth_livekit_connect", &args);
    println!("  白名单内工具: {:?}", valid);
    let invalid = validate_tool_call("apeireth_livekit_bogus", &args);
    println!("  非白名单工具: {:?}", invalid);
    println!();

    // 6) 4 K-1 强校验演示
    println!("[§6 4 K-1 强校验演示]");
    println!("  K-1 #1 API Key:  \"\" -> {:?}", LiveKitError::validate_api_key(""));
    println!(
        "  K-1 #1 API Key:  \"short\" -> {:?}",
        LiveKitError::validate_api_key("short")
    );
    println!(
        "  K-1 #1 API Key:  \"API12345678\" -> {:?}",
        LiveKitError::validate_api_key("API12345678")
    );
    println!("  K-1 #2 Secret:  \"\" -> {:?}", LiveKitError::validate_api_secret(""));
    println!(
        "  K-1 #2 Secret:  \"abcdef1234567890abcdef1234567890\" -> {:?}",
        LiveKitError::validate_api_secret("abcdef1234567890abcdef1234567890")
    );
    println!("  K-1 #3 Room:    \"\" -> {:?}", LiveKitError::validate_room_name(""));
    println!(
        "  K-1 #3 Room:    \"my-room-1\" -> {:?}",
        LiveKitError::validate_room_name("my-room-1")
    );
    println!(
        "  K-1 #4 URL:     \"http://x.com\" -> {:?}",
        LiveKitError::validate_url("http://x.com")
    );
    println!(
        "  K-1 #4 URL:     \"wss://livekit.example.com\" -> {:?}",
        LiveKitError::validate_url("wss://livekit.example.com")
    );
    println!();

    // 7) LiveKitClientImpl 构造 + 6 核心 API 返 NotImplemented
    println!("[§7 6 核心 API stub 返 NotImplemented]");
    let mut client = LiveKitClientImpl::new();
    client
        .set_api_key("API12345678".to_string())
        .expect("valid API key");
    client
        .set_api_secret("abcdef1234567890abcdef1234567890".to_string())
        .expect("valid API secret");
    println!(
        "  connect                  : {:?}",
        client.connect(DEFAULT_LIVEKIT_URL, "stub.jwt.token").await
    );
    println!("  disconnect               : {:?}", client.disconnect().await);
    let track = Track::new(TrackKind::Video, TrackSource::Camera);
    println!(
        "  publish_track            : {:?}",
        client.publish_track(&track).await
    );
    println!(
        "  subscribe                : {:?}",
        client.subscribe("TR_xxxxxxxxxxxxx").await
    );
    println!(
        "  set_camera_enabled(true) : {:?}",
        client.set_camera_enabled(true).await
    );
    println!(
        "  set_microphone_enabled(false) : {:?}",
        client.set_microphone_enabled(false).await
    );
    println!();

    // 8) Room 5 状态流转
    println!("[§8 Room 5 状态流转演示]");
    let mut room = Room::new("my-room-1", RoomOptions::default()).expect("valid room");
    println!("  initial                  : {:?}", room.state());
    for new_state in [
        RoomState::Connecting,
        RoomState::Connected,
        RoomState::Reconnecting,
        RoomState::DisconnectedAlt,
        RoomState::Disconnected,
    ] {
        room.set_state(new_state);
        println!("  -> set_state({:>14}) : {}", new_state.as_str(), room.state());
    }
    println!();

    // 9) 8 RoomEvent 发射 + 订阅
    println!("[§9 8 RoomEvent 事件发射 + 订阅演示]");
    let mut rx = client.emitter().subscribe();
    println!("  subscriber count = {}", client.emitter().receiver_count());

    let p1 = Participant::new("user-1").unwrap();
    let _p2 = Participant::new("user-2").unwrap();
    let events = vec![
        RoomEvent::ParticipantConnected { participant: p1 },
        RoomEvent::ParticipantDisconnected {
            participant_sid: "PA_user-1".to_string(),
        },
        RoomEvent::TrackSubscribed {
            track_sid: "TR_xxxxxxxxxxxxx".to_string(),
            participant_sid: "PA_user-2".to_string(),
            source: TrackSource::Camera,
        },
        RoomEvent::TrackUnsubscribed {
            track_sid: "TR_xxxxxxxxxxxxx".to_string(),
            participant_sid: "PA_user-2".to_string(),
        },
        RoomEvent::ActiveSpeakersChanged {
            speakers: vec!["PA_user-2".to_string()],
        },
        RoomEvent::ConnectionStateChanged {
            previous: RoomState::Disconnected,
            current: RoomState::Connected,
        },
        RoomEvent::DataReceived {
            participant_sid: "PA_user-2".to_string(),
            payload: vec![1, 2, 3, 4, 5],
            reliable: true,
        },
        RoomEvent::Reconnected { disconnected_at: None },
    ];
    for evt in events {
        let _ = event_publish_stub(&client, evt.clone());
        let received = rx.try_recv().expect("subscribe must receive");
        println!("  emitted: type={:>26}", received.type_str());
    }
    println!();

    // 10) ConnectionQuality 5 等级
    println!("[§10 ConnectionQuality 5 等级]");
    let qualities = [
        ConnectionQuality::Excellent,
        ConnectionQuality::Good,
        ConnectionQuality::Poor,
        ConnectionQuality::Lost,
        ConnectionQuality::Unknown,
    ];
    for q in qualities {
        println!("  {:?} -> \"{}\"", q, q.as_str());
    }
    println!();

    // 11) stub_status 工具
    println!("[§11 stub_status 工具 (R21 续真接后删)]");
    let status = client.stub_status();
    println!("  stub_mode        : {}", status.stub_mode);
    println!("  platform         : {}", status.platform);
    println!("  url              : {}", status.url);
    println!("  schema_version   : {}", status.schema_version);
    println!("  api_key_set      : {}", status.api_key_set);
    println!("  api_secret_set   : {}", status.api_secret_set);
    println!("  connected        : {}", status.connected);
    println!("  room_state       : {:?}", status.room_state);
    println!();

    println!("=== demo 完 (R21 续真接: 整合 #2 sub-agent 1 commit 落地, 改 STUB_MODE=false + 接 livekit-server SDK) ===");
}
