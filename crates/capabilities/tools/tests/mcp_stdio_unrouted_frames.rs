//! The stdio face counts the frames it could not **route**.
//!
//! # The gap this file closes
//!
//! The stdio read loop parses every line as a JSON-RPC response. A server
//! **notification** (a message with no `id:`) parses fine — and then the
//! routing table has no caller waiting for it, so the frame was dropped
//! **silently**: no count, no log. The event-stream face counts the very
//! same kind of loss ([`McpChannel::unrouted_frames`]); the stdio face left
//! the operation side with **no trace at all**.
//!
//! # What is pinned here
//!
//! * a server notification ⇒ the **unrouted** counter grows (and the frame
//!   is still only *dropped*, never handled);
//! * a frame that answers a pending request ⇒ **not** counted (the counter is
//!   not a frame counter);
//! * a broken line ⇒ counted as **malformed**, not as unrouted (the two
//!   counters stay separate);
//! * the link stays open: the child is still there after the loss.

use std::sync::Arc;
use std::time::Duration;

use apeireth_plugin::mcp::{Id, JsonRpcRequest};
use apeireth_tools_canonical::mcp_bridge::{McpChannel, StdioChannel};

/// How long the child stays alive after it wrote its line.
const CHILD_LIFETIME: &str = "sleep 30";

/// A request that never gets an answer — the child only writes its own line.
fn request(id: i64) -> JsonRpcRequest {
    JsonRpcRequest::new("initialize", Some(serde_json::json!({})), Id::Num(id))
}

/// Wait for the counter to reach `at_least` (the read task is asynchronous ⇒
/// a bounded wait turns "read it" into a fact).
async fn wait_for_unrouted(channel: &Arc<dyn McpChannel>, at_least: u64) -> u64 {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        let count = channel.unrouted_frames();
        if count >= at_least || tokio::time::Instant::now() >= deadline {
            return count;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

/// The child script that writes one line and then stays alive.
fn child_script(line: &str) -> String {
    format!("printf '%s\\n' '{line}'; {CHILD_LIFETIME}")
}

/// ① **The gap this file closes**: a server notification (no `id:`) is
/// dropped — and the drop is **counted**.
#[cfg(unix)]
#[tokio::test]
async fn a_server_notification_is_counted_as_an_unrouted_frame() {
    let channel = StdioChannel::open(
        "sh",
        &[
            "-c".to_string(),
            child_script(
                "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/message\",\"params\":{}}",
            ),
        ],
    )
    .await
    .expect("the child starts");

    let count = wait_for_unrouted(&channel, 1).await;
    assert_eq!(
        count, 1,
        "the server notification must be counted (it used to vanish silently)"
    );
}

/// ② **The control**: a frame that answers a pending request is **routed**,
/// so it is not counted.
#[cfg(unix)]
#[tokio::test]
async fn an_answer_to_a_pending_request_is_not_counted_as_unrouted() {
    let channel = StdioChannel::open(
        "sh",
        &[
            "-c".to_string(),
            child_script("{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}"),
        ],
    )
    .await
    .expect("the child starts");

    let response = tokio::time::timeout(Duration::from_secs(10), channel.request(request(1)))
        .await
        .expect("the child answers")
        .expect("the answer reaches the caller");
    assert_eq!(response.id, Some(Id::Num(1)));

    // The frame was routed: the unrouted counter stays at zero.
    assert_eq!(
        channel.unrouted_frames(),
        0,
        "a routed answer must not be counted as unrouted"
    );
}

/// ③ **The control**: a broken line is counted as **malformed** — the two
/// counters stay separate.
#[cfg(unix)]
#[tokio::test]
async fn a_broken_line_is_counted_as_malformed_and_not_as_unrouted() {
    let channel = StdioChannel::open("sh", &["-c".to_string(), child_script("{not json")])
        .await
        .expect("the child starts");

    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        if channel.malformed_frames() >= 1 || tokio::time::Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert_eq!(
        channel.malformed_frames(),
        1,
        "the broken line must be counted as malformed"
    );
    assert_eq!(
        channel.unrouted_frames(),
        0,
        "a broken line is malformed, not unrouted: the counters stay separate"
    );
}

/// ④ **The link stays open**: the loss is a drop, never a disconnect.
#[cfg(unix)]
#[tokio::test]
async fn an_unrouted_frame_does_not_break_the_link() {
    let channel = StdioChannel::open(
        "sh",
        &[
            "-c".to_string(),
            child_script(
                "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/message\",\"params\":{}}",
            ),
        ],
    )
    .await
    .expect("the child starts");

    let count = wait_for_unrouted(&channel, 1).await;
    assert_eq!(count, 1);

    // The link is still writable: a notification goes through.
    channel
        .notify(JsonRpcRequest::notification(
            "notifications/initialized",
            None,
        ))
        .await
        .expect("the link must not be broken by the unrouted frame");
}
