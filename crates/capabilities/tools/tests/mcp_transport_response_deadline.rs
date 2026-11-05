//! The transport response wait is **bounded**: a peer that takes the request
//! and never answers must not block the caller — and with it the startup
//! wiring — forever.
//!
//! The gap this file pins is the one the transport verification record
//! registered: `McpChannel::request` waited for the answer with no deadline,
//! so a server that answers the POST and then stays silent blocked the
//! handshake (and every later call) until the process was killed.
//!
//! # How the cases are driven
//!
//! * The peer is an **in-process double** (a loopback listener speaking the
//!   few HTTP lines the transport needs) — never a real server, never a
//!   subprocess, never the network.
//! * The deadline is injected, so the timeout cases need **no wall-clock
//!   wait**: a zero deadline is already expired when the wait starts, so the
//!   timer fires on the first poll. The one case that needs a live deadline
//!   (a link that must still answer *after* a timeout) uses 50 ms, which is
//!   the only wall-clock wait in this file.
//! * Every case that waits for an answer is wrapped in a **hang guard**: a
//!   request that never comes back fails the test instead of stalling it.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use apeireth_plugin::mcp::{Id, JsonRpcRequest};
use apeireth_tools_canonical::mcp::McpError;
use apeireth_tools_canonical::mcp_bridge::transport::{
    HttpChannel, McpChannel, SseChannel, StdioChannel, RESPONSE_TIMEOUT,
};

/// A request that never comes back fails the case instead of stalling it.
const HANG_GUARD: Duration = Duration::from_secs(10);

/// The deadline the "the link still answers after a timeout" case needs.
const SHORT_DEADLINE: Duration = Duration::from_millis(50);

/// What the double does with a request it receives.
#[derive(Clone, Copy)]
enum AnswerMode {
    /// Take the request and never answer it.
    Silent,
    /// Answer every request.
    Always,
    /// Answer every request except the one carrying this id.
    Except(i64),
}

/// An in-process server that speaks the few HTTP lines the SSE transport
/// needs: an event stream that announces a message endpoint, and a message
/// endpoint that takes the posted request.
struct SseDouble {
    addr: SocketAddr,
}

impl SseDouble {
    /// Start the double on an ephemeral loopback port.
    async fn start(mode: AnswerMode) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("the double must bind a loopback port");
        let addr = listener
            .local_addr()
            .expect("the double must know its port");
        let stream_slot: Arc<Mutex<Option<OwnedWriteHalf>>> = Arc::new(Mutex::new(None));
        tokio::spawn(async move {
            while let Ok((socket, _)) = listener.accept().await {
                let slot = Arc::clone(&stream_slot);
                tokio::spawn(async move {
                    handle_connection(socket, mode, slot).await;
                });
            }
        });
        Self { addr }
    }

    /// The event stream URL (the `GET` target).
    fn url(&self) -> String {
        format!("http://{}/sse", self.addr)
    }
}

/// One connection: an event stream, or a message post.
async fn handle_connection(
    mut socket: TcpStream,
    mode: AnswerMode,
    stream_slot: Arc<Mutex<Option<OwnedWriteHalf>>>,
) {
    let Some(head) = read_head(&mut socket).await else {
        return;
    };
    let method = head.split_whitespace().next().unwrap_or("").to_string();
    let mut body = vec![0u8; content_length(&head)];
    if !body.is_empty() && socket.read_exact(&mut body).await.is_err() {
        return;
    }

    if method == "GET" {
        event_stream(socket, stream_slot).await;
        return;
    }
    if method != "POST" {
        let _ = socket
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n")
            .await;
        return;
    }

    let id = serde_json::from_slice::<serde_json::Value>(&body)
        .ok()
        .and_then(|value| value.get("id").and_then(serde_json::Value::as_i64));
    let answered = match mode {
        AnswerMode::Silent => false,
        AnswerMode::Always => true,
        AnswerMode::Except(skip) => id != Some(skip),
    };
    if answered {
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id.unwrap_or_default(),
            "result": { "ok": true },
        });
        let frame = format!("event: message\ndata: {response}\n\n");
        let mut writer = stream_slot.lock().await.take();
        if let Some(writer) = writer.as_mut() {
            let _ = write_chunk(writer, frame.as_bytes()).await;
        }
        if let Some(writer) = writer {
            *stream_slot.lock().await = Some(writer);
        }
    }

    let _ = socket
        .write_all(b"HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\n\r\n")
        .await;
    let _ = socket.flush().await;
}

/// The event stream: the head, the announced message endpoint, then the
/// stream stays open (the answers arrive on it).
async fn event_stream(socket: TcpStream, stream_slot: Arc<Mutex<Option<OwnedWriteHalf>>>) {
    let mut socket = socket;
    let _ = socket
        .write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nTransfer-Encoding: chunked\r\n\r\n",
        )
        .await;
    let _ = socket.flush().await;
    let _ = write_chunk(&mut socket, b"event: endpoint\ndata: /messages\n\n").await;

    let (mut reader, writer) = socket.into_split();
    *stream_slot.lock().await = Some(writer);
    let mut sink = Vec::new();
    // The stream lives until the client goes away.
    while let Ok(read) = reader.read(&mut sink).await {
        if read == 0 {
            break;
        }
    }
}

/// Write one chunk of a chunked body.
async fn write_chunk<W: AsyncWriteExt + Unpin>(writer: &mut W, payload: &[u8]) {
    let _ = writer
        .write_all(format!("{:X}\r\n", payload.len()).as_bytes())
        .await;
    let _ = writer.write_all(payload).await;
    let _ = writer.write_all(b"\r\n").await;
    let _ = writer.flush().await;
}

/// Read the request head (up to the blank line).
async fn read_head(socket: &mut TcpStream) -> Option<String> {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let read = socket.read(&mut byte).await.ok()?;
        if read == 0 {
            return None;
        }
        head.push(byte[0]);
        if head.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8(head).ok()
}

/// The body length the request head announces.
fn content_length(head: &str) -> usize {
    head.lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.trim().eq_ignore_ascii_case("content-length") {
                value.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0)
}

/// An in-process HTTP endpoint: it answers the post either with a full
/// JSON-RPC body, or with the head alone and then goes quiet.
async fn start_http_double(answer: bool) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("the double must bind a loopback port");
    let addr = listener
        .local_addr()
        .expect("the double must know its port");
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let Some(head) = read_head(&mut socket).await else {
                continue;
            };
            let mut body = vec![0u8; content_length(&head)];
            if !body.is_empty() && socket.read_exact(&mut body).await.is_err() {
                continue;
            }
            if answer {
                let response =
                    serde_json::json!({ "jsonrpc": "2.0", "id": 1, "result": { "ok": true } });
                let payload = response.to_string();
                let _ = socket
                    .write_all(
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{payload}",
                            payload.len()
                        )
                        .as_bytes(),
                    )
                    .await;
                let _ = socket.flush().await;
            } else {
                // The head arrives, the body never does.
                let _ = socket
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 64\r\n\r\n",
                    )
                    .await;
                let _ = socket.flush().await;
                std::future::pending::<()>().await;
            }
        }
    });
    addr
}

/// One JSON-RPC request with the given id.
fn request(id: i64) -> JsonRpcRequest {
    JsonRpcRequest::new("initialize", Some(serde_json::json!({})), Id::Num(id))
}

/// ① **Controlled reproduction**: the server takes the request and never
/// answers. The client must come back with a timeout error — the whole
/// point of the fix. The hang guard turns a regression into a failed case
/// instead of a stalled suite.
#[tokio::test]
async fn a_server_that_never_answers_fails_with_a_bounded_timeout() {
    let double = SseDouble::start(AnswerMode::Silent).await;
    let channel = SseChannel::open_with_response_deadline(&double.url(), Duration::ZERO)
        .await
        .expect("the event stream opens");

    let outcome = tokio::time::timeout(HANG_GUARD, channel.request(request(1))).await;
    let error = outcome
        .expect("the request must come back: an unbounded response wait is the gap being closed")
        .expect_err("a server that never answers cannot answer");

    match error {
        McpError::Transport(reason) => {
            assert!(
                reason.contains("did not deliver the response"),
                "the error must name the wait that timed out: {reason}"
            );
        }
        other => panic!("expected a transport error, got {other:?}"),
    }
}

/// ② **The normal path is untouched**: an answer that arrives before the
/// deadline is handed to the caller, with no timeout in sight.
#[tokio::test]
async fn an_answer_that_arrives_in_time_is_returned() {
    let double = SseDouble::start(AnswerMode::Always).await;
    let channel = SseChannel::open(&double.url())
        .await
        .expect("the event stream opens");

    let outcome = tokio::time::timeout(HANG_GUARD, channel.request(request(1))).await;
    let response = outcome
        .expect("the request must come back")
        .expect("the answer must reach the caller");
    assert_eq!(response.id, Some(Id::Num(1)));
}

/// ③ **No waiter is left behind**: after a timeout the link still answers
/// the next request — the abandoned waiter neither wedges the routing table
/// nor eats the next answer.
#[tokio::test]
async fn the_link_still_answers_after_a_timed_out_request() {
    // The first request is never answered; the second one is.
    let double = SseDouble::start(AnswerMode::Except(1)).await;
    let channel = SseChannel::open_with_response_deadline(&double.url(), SHORT_DEADLINE)
        .await
        .expect("the event stream opens");

    let timed_out = tokio::time::timeout(HANG_GUARD, channel.request(request(1)))
        .await
        .expect("the first request must come back")
        .expect_err("the first request is never answered");
    assert!(
        matches!(&timed_out, McpError::Transport(reason) if reason.contains("did not deliver the response"))
    );

    let answered = tokio::time::timeout(HANG_GUARD, channel.request(request(2)))
        .await
        .expect("the second request must come back")
        .expect("the link must still answer after a timeout");
    assert_eq!(answered.id, Some(Id::Num(2)));
}

/// ④ **The wording is tellable apart**: a response wait that ran out of time
/// does not read like a connect failure, and does not read like a stream
/// that never announced its endpoint.
#[tokio::test]
async fn the_timeout_wording_is_tellable_apart_from_a_connect_failure() {
    let double = SseDouble::start(AnswerMode::Silent).await;
    let channel = SseChannel::open_with_response_deadline(&double.url(), Duration::ZERO)
        .await
        .expect("the event stream opens");
    let error = tokio::time::timeout(HANG_GUARD, channel.request(request(1)))
        .await
        .expect("the request must come back")
        .expect_err("a server that never answers cannot answer");
    let McpError::Transport(reason) = error else {
        panic!("expected a transport error");
    };
    assert!(reason.contains("did not deliver the response"));
    assert!(
        !reason.contains("did not announce a message endpoint"),
        "the endpoint failure keeps its own wording: {reason}"
    );
    assert!(
        !reason.contains("opening event stream"),
        "the connect failure keeps its own wording: {reason}"
    );
    assert!(
        !reason.contains("closed before the response arrived"),
        "the dead-link failure keeps its own wording: {reason}"
    );

    // The connect failure of the same face reads differently.
    let Err(refused) = SseChannel::open("http://127.0.0.1:9/sse").await else {
        panic!("nothing listens on that port");
    };
    let McpError::Transport(reason) = refused else {
        panic!("expected a transport error");
    };
    assert!(
        reason.contains("opening event stream"),
        "the connect failure names the connect phase: {reason}"
    );
    assert!(!reason.contains("did not deliver the response"));
}

/// ⑤ **The HTTP face is bounded too**: an endpoint that sends the head and
/// then goes quiet used to block the body read forever.
#[tokio::test]
async fn an_http_endpoint_that_never_sends_the_body_fails_with_a_bounded_timeout() {
    let addr = start_http_double(false).await;
    let channel =
        HttpChannel::open_with_response_deadline(&format!("http://{addr}/mcp"), Duration::ZERO)
            .expect("the channel opens");

    let outcome = tokio::time::timeout(HANG_GUARD, channel.request(request(1))).await;
    let error = outcome
        .expect("the request must come back: an unbounded response wait is the gap being closed")
        .expect_err("an endpoint that never sends the body cannot answer");
    match error {
        McpError::Transport(reason) => {
            assert!(
                reason.contains("did not deliver the response"),
                "the error must name the wait that timed out: {reason}"
            );
        }
        other => panic!("expected a transport error, got {other:?}"),
    }
}

/// The HTTP face keeps its normal path: an answer that arrives is parsed.
#[tokio::test]
async fn an_http_answer_that_arrives_in_time_is_returned() {
    let addr = start_http_double(true).await;
    let channel = HttpChannel::open(&format!("http://{addr}/mcp")).expect("the channel opens");

    let outcome = tokio::time::timeout(HANG_GUARD, channel.request(request(1))).await;
    let response = outcome
        .expect("the request must come back")
        .expect("the answer must reach the caller");
    assert_eq!(response.id, Some(Id::Num(1)));
}

/// ⑥ **The stdio face is bounded too**: a child that reads the request and
/// never answers used to block the caller forever.
#[cfg(unix)]
#[tokio::test]
async fn a_child_that_never_answers_fails_with_a_bounded_timeout() {
    let channel = StdioChannel::open_with_response_deadline(
        "sh",
        &["-c".to_string(), "sleep 300".to_string()],
        Duration::ZERO,
    )
    .await
    .expect("the child starts");

    let outcome = tokio::time::timeout(HANG_GUARD, channel.request(request(1))).await;
    let error = outcome
        .expect("the request must come back: an unbounded response wait is the gap being closed")
        .expect_err("a child that never answers cannot answer");
    match error {
        McpError::Transport(reason) => {
            assert!(
                reason.contains("did not deliver the response"),
                "the error must name the wait that timed out: {reason}"
            );
        }
        other => panic!("expected a transport error, got {other:?}"),
    }
}

/// The production deadline is the tier this batch aligned on: the same one
/// the bridge arms per call (`DEFAULT_CALL_TIMEOUT_MS`, 30 s), so the
/// transport is never the stricter of the two.
#[test]
fn the_production_response_deadline_is_the_bridge_call_tier() {
    assert_eq!(RESPONSE_TIMEOUT, Duration::from_secs(30));
}
