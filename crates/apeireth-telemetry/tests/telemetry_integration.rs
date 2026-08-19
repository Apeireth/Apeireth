//! Integration tests for apeireth-telemetry (post-1.0.0)
//!
//! src/ 6 module 真实现 (cache/metric/trace/observability/log_replay/otlp).
//! 这里 (tests/) 加 OTLP 集成 + EvictionPolicy 边界 + JSON 序列化.
//! 0 触碰 src/, 0 编造"已实现".

use apeireth_telemetry::cache::policy::EvictionPolicy;
use apeireth_telemetry::otlp::{
    default_sink, json_lines_sink, JsonLinesOtlpSink, NoopOtlpSink, OtlpError, OtlpEvent, OtlpSink,
};
use std::io::Write;
use std::sync::{Arc, Mutex};

// =============================================================================
// EvictionPolicy
// =============================================================================

#[test]
fn eviction_policy_5_strategies() {
    assert_eq!(EvictionPolicy::ALL.len(), 5);
}

#[test]
fn eviction_policy_as_str() {
    assert_eq!(EvictionPolicy::Lru.as_str(), "LRU");
    assert_eq!(EvictionPolicy::Lfu.as_str(), "LFU");
    assert_eq!(EvictionPolicy::Fifo.as_str(), "FIFO");
    assert_eq!(EvictionPolicy::Arc.as_str(), "ARC");
    assert_eq!(EvictionPolicy::TinyLfu.as_str(), "TINY_LFU");
}

#[test]
fn eviction_policy_unique() {
    let names: Vec<&str> = EvictionPolicy::ALL.iter().map(|p| p.as_str()).collect();
    let unique: std::collections::HashSet<&str> = names.iter().copied().collect();
    assert_eq!(unique.len(), 5);
}

#[test]
fn eviction_policy_eq_copy_hash() {
    let p = EvictionPolicy::Lru;
    let p2 = p;
    assert_eq!(p, p2);
    let mut set = std::collections::HashSet::new();
    set.insert(p);
    set.insert(EvictionPolicy::Lfu);
    set.insert(EvictionPolicy::Lru);
    assert_eq!(set.len(), 2);
}

#[test]
fn eviction_policy_serde_roundtrip() {
    for p in EvictionPolicy::ALL {
        let s = serde_json::to_string(&p).unwrap();
        let back: EvictionPolicy = serde_json::from_str(&s).unwrap();
        assert_eq!(p, back);
    }
}

#[test]
fn eviction_policy_supports_distributed() {
    // 5 策略全返 true (per src)
    for p in EvictionPolicy::ALL {
        assert!(p.supports_distributed());
    }
}

// =============================================================================
// OtlpEvent
// =============================================================================

#[test]
fn otlp_event_new_sets_name_and_timestamp() {
    let e = OtlpEvent::new("test.event");
    assert_eq!(e.name, "test.event");
    assert!(e.timestamp_unix_ms > 0);
    assert!(e.attributes.is_empty());
    assert!(e.payload.is_none());
}

#[test]
fn otlp_event_with_attribute() {
    let e = OtlpEvent::new("e").with_attribute("k", "v");
    assert_eq!(e.attributes, vec![("k".to_string(), "v".to_string())]);
}

#[test]
fn otlp_event_with_multiple_attributes() {
    let e = OtlpEvent::new("e")
        .with_attribute("k1", "v1")
        .with_attribute("k2", "v2")
        .with_attribute("k3", "v3");
    assert_eq!(e.attributes.len(), 3);
}

#[test]
fn otlp_event_with_payload() {
    let e = OtlpEvent::new("e").with_payload(serde_json::json!({"tokens": 100}));
    assert!(e.payload.is_some());
    assert_eq!(e.payload.unwrap()["tokens"], 100);
}

#[test]
fn otlp_event_clone() {
    let e = OtlpEvent::new("e").with_attribute("k", "v");
    let e2 = e.clone();
    assert_eq!(e.name, e2.name);
    assert_eq!(e.attributes, e2.attributes);
}

#[test]
fn otlp_event_serde_roundtrip() {
    let e = OtlpEvent::new("test.event")
        .with_attribute("k", "v")
        .with_payload(serde_json::json!({"n": 42}));
    let s = serde_json::to_string(&e).unwrap();
    assert!(s.contains("test.event"));
    let back: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(back["name"], "test.event");
    assert_eq!(back["attributes"][0][0], "k");
}

// =============================================================================
// NoopOtlpSink
// =============================================================================

#[tokio::test]
async fn noop_sink_basic() {
    let s = NoopOtlpSink::new();
    assert_eq!(s.name(), "noop");
    assert!(!s.is_implemented());
}

#[tokio::test]
async fn noop_sink_emit_returns_ok() {
    let s = NoopOtlpSink::new();
    let e = OtlpEvent::new("test");
    assert!(s.emit(&e).await.is_ok());
}

#[tokio::test]
async fn noop_sink_flush_shutdown_ok() {
    let s = NoopOtlpSink::new();
    assert!(s.flush().await.is_ok());
    assert!(s.shutdown().await.is_ok());
}

#[tokio::test]
async fn default_sink_is_noop() {
    let s = default_sink();
    assert_eq!(s.name(), "noop");
    assert!(!s.is_implemented());
}

// =============================================================================
// JsonLinesOtlpSink
// =============================================================================

struct TestWriter {
    buf: Arc<Mutex<Vec<u8>>>,
}

impl Write for TestWriter {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.buf.lock().unwrap().extend_from_slice(data);
        Ok(data.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn json_lines_sink_basic() {
    let shared = Arc::new(Mutex::new(Vec::<u8>::new()));
    let s = JsonLinesOtlpSink::new(Box::new(TestWriter {
        buf: Arc::clone(&shared),
    }));
    assert!(s.is_implemented());
    assert_eq!(s.name(), "json_lines");
}

#[tokio::test]
async fn json_lines_sink_writes_one_line_per_event() {
    let shared = Arc::new(Mutex::new(Vec::<u8>::new()));
    let s = json_lines_sink(Box::new(TestWriter {
        buf: Arc::clone(&shared),
    }));
    s.emit(&OtlpEvent::new("event1")).await.unwrap();
    s.emit(&OtlpEvent::new("event2")).await.unwrap();
    s.flush().await.unwrap();
    let bytes = shared.lock().unwrap().clone();
    let text = String::from_utf8(bytes).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
}

#[tokio::test]
async fn json_lines_sink_json_valid() {
    let shared = Arc::new(Mutex::new(Vec::<u8>::new()));
    let s = json_lines_sink(Box::new(TestWriter {
        buf: Arc::clone(&shared),
    }));
    let e = OtlpEvent::new("test.event")
        .with_attribute("k", "v")
        .with_payload(serde_json::json!({"tokens": 128}));
    s.emit(&e).await.unwrap();
    s.flush().await.unwrap();
    let bytes = shared.lock().unwrap().clone();
    let text = String::from_utf8(bytes).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(text.trim()).unwrap();
    assert_eq!(parsed["name"], "test.event");
    assert_eq!(parsed["attributes"][0][0], "k");
    assert_eq!(parsed["attributes"][0][1], "v");
    assert_eq!(parsed["payload"]["tokens"], 128);
}

#[tokio::test]
async fn json_lines_sink_shutdown_ok() {
    let shared = Arc::new(Mutex::new(Vec::<u8>::new()));
    let s = json_lines_sink(Box::new(TestWriter {
        buf: Arc::clone(&shared),
    }));
    assert!(s.shutdown().await.is_ok());
}

// =============================================================================
// OtlpError
// =============================================================================

#[test]
fn otlp_error_io_display() {
    let e = OtlpError::Io("write fail".into());
    let s = e.to_string();
    assert!(s.contains("write fail"));
}

#[test]
fn otlp_error_serialize_display() {
    let e = OtlpError::Serialize("bad json".into());
    let s = e.to_string();
    assert!(s.contains("bad json"));
}

#[test]
fn otlp_error_not_implemented_display() {
    let e = OtlpError::NotImplemented("no http".into());
    let s = e.to_string();
    assert!(s.contains("no http"));
    assert!(s.contains("not implemented"));
}

// =============================================================================
// Cross-module integration
// =============================================================================

#[tokio::test]
async fn integration_noop_then_json_lines() {
    // 验证 OtlpSink trait dispatch + 两个 impl 行为差异
    let noop = NoopOtlpSink::new();
    let shared = Arc::new(Mutex::new(Vec::<u8>::new()));
    let json = JsonLinesOtlpSink::new(Box::new(TestWriter {
        buf: Arc::clone(&shared),
    }));

    let event = OtlpEvent::new("integration.test").with_attribute("source", "integration");

    noop.emit(&event).await.unwrap();
    json.emit(&event).await.unwrap();
    json.flush().await.unwrap();

    // Noop 不实现, json 实现
    assert!(!noop.is_implemented());
    assert!(json.is_implemented());

    // Json 写出 1 行
    let bytes = shared.lock().unwrap().clone();
    let text = String::from_utf8(bytes).unwrap();
    assert_eq!(text.lines().count(), 1);
}

#[tokio::test]
async fn integration_dyn_sink_via_trait() {
    // 通过 &dyn OtlpSink 调用
    let noop_sink = NoopOtlpSink::new();
    let sink: &dyn OtlpSink = &noop_sink;
    assert_eq!(sink.name(), "noop");
    let e = OtlpEvent::new("any");
    assert!(sink.emit(&e).await.is_ok());
}

#[tokio::test]
async fn integration_json_sink_multiple_events_ordered() {
    let shared = Arc::new(Mutex::new(Vec::<u8>::new()));
    let s = json_lines_sink(Box::new(TestWriter {
        buf: Arc::clone(&shared),
    }));
    for i in 0..5 {
        s.emit(&OtlpEvent::new(&format!("event.{i}")))
            .await
            .unwrap();
    }
    s.flush().await.unwrap();
    let bytes = shared.lock().unwrap().clone();
    let text = String::from_utf8(bytes).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 5);
    let names: Vec<String> = lines
        .iter()
        .map(|l| {
            let v: serde_json::Value = serde_json::from_str(l).unwrap();
            v["name"].as_str().unwrap().to_string()
        })
        .collect();
    assert_eq!(names[0], "event.0");
    assert_eq!(names[4], "event.4");
}

#[test]
fn integration_event_attributes_preserved() {
    let e = OtlpEvent::new("e")
        .with_attribute("user", "alice")
        .with_attribute("env", "prod");
    let s = serde_json::to_string(&e).unwrap();
    let back: serde_json::Value = serde_json::from_str(&s).unwrap();
    let attrs = back["attributes"].as_array().unwrap();
    assert_eq!(attrs.len(), 2);
}

#[test]
fn integration_payload_serialized_when_some() {
    let e = OtlpEvent::new("e").with_payload(serde_json::json!({"k": "v"}));
    let s = serde_json::to_string(&e).unwrap();
    assert!(s.contains("payload"));
    assert!(s.contains("\"k\":\"v\""));
}

#[test]
fn integration_payload_skipped_when_none() {
    let e = OtlpEvent::new("e");
    let s = serde_json::to_string(&e).unwrap();
    assert!(!s.contains("payload"), "payload=None 应跳过");
}
