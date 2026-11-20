//! Negotiated progress for the single admitted native turn.

use super::*;
use apeireth_runtime::canonical::turn_cancellation::stop_summary;
use axum::body::{to_bytes, Body, Bytes};
use axum::http::{header, HeaderMap};
use std::convert::Infallible;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::ReceiverStream;

const MEDIA_TYPE: &str = "application/x-apeireth-turn+ndjson";
const MAX_DELTA_BYTES: usize = 16 * 1024;
const MAX_RECEIPT_BYTES: usize = 4 * 1024 * 1024;
const QUEUE: usize = 32;

pub(super) type DeltaSink = Arc<dyn Fn(String) + Send + Sync>;
pub(super) type TurnReply = (Result<Response, HttpError>, Option<RequestId>);

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Frame {
    Started {
        session: SessionId,
        request: RequestId,
    },
    Delta {
        sequence: u64,
        text: String,
    },
    Final {
        status: u16,
        body: serde_json::Value,
    },
    Stopped {
        session: SessionId,
        request: RequestId,
    },
}

pub(super) fn requested(headers: &HeaderMap) -> bool {
    headers
        .get_all(header::ACCEPT)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .any(|value| value.trim().eq_ignore_ascii_case(MEDIA_TYPE))
}

pub(super) struct Progress {
    pub(super) request: RequestId,
    tx: mpsc::Sender<Frame>,
}

impl Progress {
    pub(super) fn start(self, session: SessionId) -> (RequestId, DeltaSink) {
        let Self { request, tx } = self;
        let _ = tx.try_send(Frame::Started { session, request });
        let sequence = AtomicU64::new(1);
        let sink = Arc::new(move |text: String| {
            let sequence = sequence.fetch_add(1, Ordering::Relaxed);
            if text.len() <= MAX_DELTA_BYTES {
                let _ = tx.try_send(Frame::Delta { sequence, text });
            }
        });
        (request, sink)
    }
}

pub(super) struct Reader(mpsc::Receiver<Frame>);

pub(super) fn channel() -> (Progress, Reader) {
    let (tx, rx) = mpsc::channel(QUEUE);
    (
        Progress {
            request: RequestId::new(),
            tx,
        },
        Reader(rx),
    )
}

/// The first response and any operation replay share the original stop fact.
pub(super) fn stopped_request(
    session: SessionId,
    fresh_request: RequestId,
    outcome: Option<&CanonicalChatOutcome>,
) -> Option<RequestId> {
    let request = match outcome {
        Some(CanonicalChatOutcome::Completed(value)) => value.request.parse().ok()?,
        Some(CanonicalChatOutcome::PendingApproval(value)) => value.request.parse().ok()?,
        None => fresh_request,
    };
    stop_summary(session, request).map(|_| request)
}

impl Reader {
    pub(super) fn response(self, session: SessionId, worker: JoinHandle<TurnReply>) -> Response {
        // The reader may disappear at any time. The admitted worker owns execution, and its
        // terminal result is joined separately from the bounded, lossy preview queue.
        let progress = ReceiverStream::new(self.0).map(line);
        let terminal = tokio_stream::once(worker).then(move |worker| async move {
            let frame = match worker.await {
                Ok((_, Some(request))) => Frame::Stopped { session, request },
                Ok((reply, None)) => final_frame(session, reply).await,
                Err(_) => failed_frame(session),
            };
            line(frame)
        });
        let frames = progress.chain(terminal).map(Ok::<Bytes, Infallible>);
        Response::builder()
            .header(header::CONTENT_TYPE, MEDIA_TYPE)
            .header(header::CACHE_CONTROL, "no-store")
            .header(header::VARY, "Accept")
            .body(Body::from_stream(frames))
            .expect("static native stream response headers")
    }
}

async fn final_frame(session: SessionId, reply: Result<Response, HttpError>) -> Frame {
    let response = reply.unwrap_or_else(IntoResponse::into_response);
    let (parts, body) = response.into_parts();
    let Ok(bytes) = to_bytes(body, MAX_RECEIPT_BYTES).await else {
        return failed_frame(session);
    };
    let Ok(body) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return failed_frame(session);
    };
    Frame::Final {
        status: parts.status.as_u16(),
        body,
    }
}

fn failed_frame(session: SessionId) -> Frame {
    let (status, Json(body)) = native_http::owner_failed(session);
    Frame::Final {
        status: status.as_u16(),
        body: serde_json::to_value(body).expect("canonical error is JSON serializable"),
    }
}

fn line(frame: Frame) -> Bytes {
    let mut bytes = serde_json::to_vec(&frame).expect("native turn frame is JSON serializable");
    bytes.push(b'\n');
    Bytes::from(bytes)
}
