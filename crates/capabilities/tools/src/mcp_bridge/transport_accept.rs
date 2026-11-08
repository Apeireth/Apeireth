//! The **http** face must name the response shapes it is able to read.
//!
//! # The gap this file pins
//!
//! The face built its POST without naming a response shape, so the HTTP
//! client sent its own default wildcard (`Accept: */*`) — the bridge itself
//! said nothing. A server that speaks the protocol strictly requires the
//! two shapes of this face (`application/json` and `text/event-stream`) to
//! be **listed**, and answers anything else with `406 Not Acceptable`.
//! Against such a server the whole face was unusable.
//!
//! # How the cases are driven
//!
//! * The peer is an **in-process double** (a loopback listener speaking the
//!   few HTTP lines the face needs) — never a real server, never a
//!   subprocess, never the network.
//! * The double **records the head and the body** of every request it
//!   serves, so the assertions are about the bytes on the wire, not about
//!   the code that produced them.
//! * The strict double **refuses** a request that does not name both
//!   shapes, so a passing case proves the header is really on the wire (a
//!   control case shows the double is not a rubber stamp).
//! * The one case that waits for a child uses a **bounded poll**: no case
//!   waits without a bound.

use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use apeireth_plugin::mcp::{Id, JsonRpcRequest};

use crate::mcp::McpError;

use super::{HttpChannel, McpChannel, StdioChannel, HTTP_ACCEPT};

/// A request that never comes back fails the case instead of stalling it.
const HANG_GUARD: Duration = Duration::from_secs(10);

/// The bound of the one poll that waits for a child to write what it read.
const CHILD_WRITE_BOUND: Duration = Duration::from_secs(5);

/// How the double answers.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Policy {
    /// A strict server: it answers a request that names **both** response
    /// shapes the protocol defines, and refuses every other request.
    Strict,
    /// A server that refuses every request, whatever it carries.
    Refusing,
}

/// What the double says when it refuses.
const REJECTION: &str =
    "Not Acceptable: the client must accept application/json and text/event-stream";

/// One request the double served.
#[derive(Clone)]
struct Recorded {
    head: String,
    body: Vec<u8>,
}

/// An in-process HTTP endpoint that records every request it serves.
struct RecordingEndpoint {
    addr: SocketAddr,
    recorded: Arc<Mutex<Vec<Recorded>>>,
}

impl RecordingEndpoint {
    /// Start the double on an ephemeral loopback port.
    async fn start(policy: Policy) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("the double must bind a loopback port");
        let addr = listener
            .local_addr()
            .expect("the double must know its port");
        let recorded: Arc<Mutex<Vec<Recorded>>> = Arc::new(Mutex::new(Vec::new()));
        let served = Arc::clone(&recorded);
        tokio::spawn(async move {
            while let Ok((socket, _)) = listener.accept().await {
                let served = Arc::clone(&served);
                tokio::spawn(async move {
                    serve(socket, policy, served).await;
                });
            }
        });
        Self { addr, recorded }
    }

    /// The URL the face is pointed at.
    fn url(&self) -> String {
        format!("http://{}/mcp", self.addr)
    }

    /// The **first** request the double served.
    async fn first(&self) -> Recorded {
        self.recorded
            .lock()
            .await
            .first()
            .cloned()
            .unwrap_or_else(|| panic!("the double must have served a request"))
    }
}

/// Serve one connection: record the request, then answer it.
async fn serve(mut socket: TcpStream, policy: Policy, served: Arc<Mutex<Vec<Recorded>>>) {
    let Some(head) = read_head(&mut socket).await else {
        return;
    };
    let mut body = vec![0u8; content_length(&head)];
    if !body.is_empty() && socket.read_exact(&mut body).await.is_err() {
        return;
    }
    served.lock().await.push(Recorded { head, body });

    let accepts_both = names_both_shapes(&served.lock().await[0].head);
    let refused = match policy {
        Policy::Strict => !accepts_both,
        Policy::Refusing => true,
    };
    if refused {
        let _ = socket
            .write_all(
                format!(
                    "HTTP/1.1 406 Not Acceptable\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{REJECTION}",
                    REJECTION.len()
                )
                .as_bytes(),
            )
            .await;
        let _ = socket.flush().await;
        return;
    }

    let id = serde_json::from_slice::<serde_json::Value>(&served.lock().await[0].body)
        .ok()
        .and_then(|value| value.get("id").and_then(serde_json::Value::as_i64))
        .unwrap_or_default();
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": { "ok": true },
    })
    .to_string();
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
}

/// Whether a request head names **both** response shapes of this face.
///
/// This is the check a strict server makes: the protocol requires the
/// client to list the two shapes, so a head that names neither (or only
/// one) is refused.
fn names_both_shapes(head: &str) -> bool {
    let Some(accept) = header(head, "accept") else {
        return false;
    };
    let types: Vec<&str> = accept.split(',').map(|part| part.trim()).collect();
    types.contains(&"application/json") && types.contains(&"text/event-stream")
}

/// One JSON-RPC request with the given id.
fn request(id: i64) -> JsonRpcRequest {
    JsonRpcRequest::new("initialize", Some(serde_json::json!({})), Id::Num(id))
}

/// The value of one header in a recorded head, if the head carries it.
fn header(head: &str, name: &str) -> Option<String> {
    head.lines()
        .skip(1)
        .filter_map(|line| line.split_once(':'))
        .find(|(key, _)| key.trim().eq_ignore_ascii_case(name))
        .map(|(_, value)| value.trim().to_string())
}

/// How many times a header appears in a recorded head.
fn header_count(head: &str, name: &str) -> usize {
    head.lines()
        .skip(1)
        .filter(|line| {
            line.split_once(':')
                .map(|(key, _)| key.trim().eq_ignore_ascii_case(name))
                .unwrap_or(false)
        })
        .count()
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

/// ① **The core case**: the request carries the protocol's `Accept` header
/// — the header a strict server checks before it answers at all. Without it
/// the strict double refuses the request.
#[tokio::test]
async fn the_http_face_sends_the_protocol_accept_header() {
    let endpoint = RecordingEndpoint::start(Policy::Strict).await;
    let channel = HttpChannel::open(&endpoint.url()).expect("the channel opens");

    let response = tokio::time::timeout(HANG_GUARD, channel.request(request(1)))
        .await
        .expect("the request must come back")
        .expect("the strict server must answer a request that names both shapes");
    assert_eq!(response.id, Some(Id::Num(1)));

    let recorded = endpoint.first().await;
    let accept = header(&recorded.head, "accept").expect(
        "the request must carry an Accept header: a strict server refuses one that names no shape",
    );
    assert_eq!(
        accept, HTTP_ACCEPT,
        "the Accept header must be the protocol value"
    );
    assert_eq!(
        header_count(&recorded.head, "accept"),
        1,
        "exactly one Accept header must go out: {}",
        recorded.head
    );
}

/// ② **Both response types are listed**: the protocol defines two shapes
/// for this face, and a client that can read both says so.
#[tokio::test]
async fn the_accept_header_lists_both_response_types() {
    let endpoint = RecordingEndpoint::start(Policy::Strict).await;
    let channel = HttpChannel::open(&endpoint.url()).expect("the channel opens");

    tokio::time::timeout(HANG_GUARD, channel.request(request(1)))
        .await
        .expect("the request must come back")
        .expect("the strict server must answer");

    let recorded = endpoint.first().await;
    let accept = header(&recorded.head, "accept").expect("the request must carry an Accept header");
    let types: Vec<&str> = accept.split(',').map(|part| part.trim()).collect();
    assert!(
        types.contains(&"application/json"),
        "the JSON response shape must be listed: {accept}"
    );
    assert!(
        types.contains(&"text/event-stream"),
        "the event-stream response shape must be listed: {accept}"
    );
    assert_eq!(
        types.len(),
        2,
        "the header must list exactly the two shapes: {accept}"
    );
}

/// ③ **The stdio face is untouched**: it still writes one bare
/// newline-delimited JSON-RPC line to the child — no HTTP framing, no
/// `Accept` header.
#[cfg(unix)]
#[tokio::test]
async fn the_stdio_face_still_writes_a_bare_json_rpc_line() {
    let directory = tempfile::tempdir().expect("a scratch directory");
    let path = directory.path().join("wire.txt");
    let channel = StdioChannel::open_with_response_deadline(
        "sh",
        &[
            "-c".to_string(),
            "cat > \"$1\"".to_string(),
            "sh".to_string(),
            path.to_string_lossy().into_owned(),
        ],
        Duration::from_secs(5),
    )
    .await
    .expect("the child starts");

    let notification = JsonRpcRequest::notification("notifications/initialized", None);
    channel
        .notify(notification.clone())
        .await
        .expect("the notification is written");

    let wire = wait_for_content(&path).await;
    let expected = format!(
        "{}\n",
        serde_json::to_string(&notification).expect("the request serializes")
    );
    assert_eq!(
        wire, expected,
        "the stdio face must still write the bare JSON-RPC line"
    );
    assert!(
        !wire.contains("Accept") && !wire.contains("HTTP/"),
        "the stdio face carries no HTTP framing: {wire}"
    );
}

/// ④ **The other headers are untouched**: the change adds one header, it
/// does not rewrite the request — the body is still the JSON-RPC message
/// and the content type is still JSON.
#[tokio::test]
async fn the_other_headers_of_the_request_are_untouched() {
    let endpoint = RecordingEndpoint::start(Policy::Strict).await;
    let channel = HttpChannel::open(&endpoint.url()).expect("the channel opens");

    tokio::time::timeout(HANG_GUARD, channel.request(request(1)))
        .await
        .expect("the request must come back")
        .expect("the strict server must answer");

    let recorded = endpoint.first().await;
    let content_type = header(&recorded.head, "content-type")
        .expect("the request must still declare the type of its body");
    assert_eq!(
        content_type, "application/json",
        "the body type must be untouched"
    );
    // The HTTP body is the JSON-RPC message itself — no newline framing
    // (that is the stdio face's rule, not this one's).
    let expected = serde_json::to_string(&request(1)).expect("the request serializes");
    assert_eq!(
        String::from_utf8_lossy(&recorded.body),
        expected,
        "the body must still be the JSON-RPC message"
    );
}

/// ⑤ **The failure face is readable**: a server that refuses the request
/// must have its own words in the error, not a generic "request failed".
/// The classification does not change — it is still a transport error.
#[tokio::test]
async fn a_refusal_carries_the_servers_own_words() {
    let endpoint = RecordingEndpoint::start(Policy::Refusing).await;
    let channel = HttpChannel::open(&endpoint.url()).expect("the channel opens");

    let error = tokio::time::timeout(HANG_GUARD, channel.request(request(1)))
        .await
        .expect("the request must come back")
        .expect_err("a server that refuses cannot answer");
    let McpError::Transport(reason) = error else {
        panic!("expected a transport error");
    };
    assert!(reason.contains("406"), "the status must be named: {reason}");
    assert!(
        reason.contains("Not Acceptable"),
        "the status phrase must be named: {reason}"
    );
    assert!(
        reason.contains("the client must accept application/json and text/event-stream"),
        "the server's own words must survive: {reason}"
    );
}

/// **The control**: the strict double really checks. A request that names
/// no response shape (the HTTP client's own default wildcard) is refused —
/// so a passing case above proves the header is on the wire, not that the
/// double answers everything.
#[tokio::test]
async fn the_double_refuses_a_request_that_names_no_shape() {
    let endpoint = RecordingEndpoint::start(Policy::Strict).await;

    let response = reqwest::Client::new()
        .post(endpoint.url())
        .json(&request(1))
        .send()
        .await
        .expect("the request reaches the double");
    assert_eq!(
        response.status(),
        406,
        "a request that names no response shape must be refused"
    );

    let recorded = endpoint.first().await;
    let accept = header(&recorded.head, "accept").unwrap_or_default();
    assert!(
        !names_both_shapes(&recorded.head),
        "the control request must not name both shapes: {accept}"
    );
}

/// Wait (bounded) until the child has written what it received.
async fn wait_for_content(path: &Path) -> String {
    let bound = tokio::time::Instant::now() + CHILD_WRITE_BOUND;
    while tokio::time::Instant::now() < bound {
        if let Ok(text) = std::fs::read_to_string(path) {
            if !text.is_empty() {
                return text;
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("the child must have written what it received");
}
