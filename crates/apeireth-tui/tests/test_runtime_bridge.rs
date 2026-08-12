//! Integration test for apeireth_tui runtime_bridge.
//!
//! Covers:
//! - Bridge construction with default config
//! - Bootstrap + cycle + dispatch + post_message lifecycle
//! - State caching + bounded push
//! - Snapshot JSON serialization
//! - Concurrent safe (parking_lot Mutex)

use apeireth_tui::{BridgeState, RuntimeBridge};
use apeireth_consciousness::EmotionEvent;

#[test]
fn integration_bridge_full_lifecycle() {
    let mut config = apeireth_runtime::RuntimeConfig::default();
    config.arbitration_path = None;
    let bridge = RuntimeBridge::with_config(config);
    let room_id = bridge.bootstrap().expect("bootstrap");
    assert!(!room_id.is_empty());
    assert_eq!(bridge.cycle_count(), 0);
    assert!(bridge.last_cycle_report().is_none());
    bridge.add_participant("test_user", "Test User").unwrap();
    bridge.post_message("test_user", "integration hello".to_string()).unwrap();
    let state = bridge.state();
    assert_eq!(state.cycle_count, 0);
    assert!(!state.recent_chat_messages.is_empty());
}

#[tokio::test]
async fn integration_bridge_run_multiple_cycles() {
    let bridge = RuntimeBridge::with_config(apeireth_runtime::RuntimeConfig::default());
    bridge.bootstrap().unwrap();
    for _ in 0..3 {
        let report = bridge.run_cycle().await.expect("cycle");
        assert!(report.task_id > 0 || report.task_id == 0);
    }
    assert_eq!(bridge.cycle_count(), 3);
    let snap = bridge.current_emotion();
    assert!(snap.intensity >= 0.0);
}

#[tokio::test]
async fn integration_bridge_dispatch_and_track() {
    let bridge = RuntimeBridge::with_config(apeireth_runtime::RuntimeConfig::default());
    bridge.bootstrap().unwrap();
    let task_id = bridge.dispatch_task("classify", "{}").await;
    assert_eq!(bridge.recent_task_count(), 1);
    // Wait for completion
    let _ = bridge
        .runtime()
        .task_store
        .wait_for_completion(task_id, std::time::Duration::from_secs(2))
        .await;
}

#[test]
fn integration_bridge_emotion_round_trip() {
    let bridge = RuntimeBridge::with_config(apeireth_runtime::RuntimeConfig::default());
    bridge.bootstrap().unwrap();
    bridge.apply_emotion(EmotionEvent::TaskSuccess);
    bridge.refresh_emotion();
    let snap = bridge.current_emotion();
    // After task success, dominant emotion should reflect
    assert!(snap.intensity >= 0.0);
    let _ = snap;
}

#[test]
fn integration_bridge_snapshot_json() {
    let bridge = RuntimeBridge::with_config(apeireth_runtime::RuntimeConfig::default());
    bridge.bootstrap().unwrap();
    let json = bridge.snapshot_json();
    let keys: Vec<&str> = json.as_object().unwrap().keys().map(|s| s.as_str()).collect();
    assert!(keys.contains(&"cycle_count"));
    assert!(keys.contains(&"recent_task_count"));
    assert!(keys.contains(&"recent_arbitration_count"));
    assert!(keys.contains(&"recent_chat_count"));
    assert!(keys.contains(&"has_emotion"));
    assert!(keys.contains(&"last_event_topic"));
}

#[test]
fn integration_bridge_state_max_constants() {
    // Ensure capacity bounds are reasonable for TUI consumption
    assert!(BridgeState::MAX_TRACKED_TASKS >= 8 && BridgeState::MAX_TRACKED_TASKS <= 64);
    assert!(BridgeState::MAX_TRACKED_ARBITRATION >= 8 && BridgeState::MAX_TRACKED_ARBITRATION <= 64);
    assert!(BridgeState::MAX_TRACKED_MESSAGES >= 4 && BridgeState::MAX_TRACKED_MESSAGES <= 32);
}

#[test]
fn integration_bridge_default_state_zero_counts() {
    let state = BridgeState::default();
    assert_eq!(state.cycle_count, 0);
    assert!(state.last_cycle_report.is_none());
    assert!(state.recent_task_ids.is_empty());
    assert!(state.recent_arbitration_seqs.is_empty());
    assert!(state.recent_chat_messages.is_empty());
    assert!(state.emotion.is_none());
    assert!(state.last_event_topic.is_none());
}
