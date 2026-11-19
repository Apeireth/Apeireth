//! Bounded, read-only pages of locally executed turns from an immutable session log.

use std::io::{self, Write};

use super::*;

const HISTORY_ENDPOINT: &str = "/v1/sessions/:session_id/turns";
const MAX_PAGE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct HistoryQuery {
    after: u64,
    limit: Option<u8>,
}

#[derive(Debug, Serialize)]
struct HistoryPage<'a> {
    schema: u8,
    source: &'static str,
    session: SessionId,
    revision: u64,
    turns: Vec<HistoryTurn<'a>>,
    next_cursor: u64,
    has_more: bool,
}

#[derive(Debug, Serialize)]
struct HistoryTurn<'a> {
    cursor: u64,
    input_sha256: String,
    input: &'a str,
    request: RequestId,
    trace_id: TraceId,
    state: &'static str,
    text: Option<&'a str>,
    usage: Option<()>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recorded_at: Option<i64>,
}

#[derive(Debug, PartialEq)]
enum HistoryError {
    InvalidCursor,
    Unverifiable,
    TooLarge,
}

pub(crate) async fn get_session_turns(
    State(state): State<GatewayState>,
    domain: Option<Extension<DomainContext>>,
    user: Option<Extension<UserContext>>,
    Path(session): Path<String>,
    query: Result<Query<HistoryQuery>, QueryRejection>,
) -> Result<Response, Response> {
    let session = authorized_session_id(&state, user.as_ref().map(|u| &u.0), &session)?;
    let query = query
        .ok()
        .map(|Query(q)| q)
        .filter(|q| matches!(q.limit, None | Some(1..=16)))
        .ok_or_else(|| {
            failure(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidRequest,
                "invalid turn history selector",
            )
        })?;
    let record = load_scoped_session(
        &state,
        domain.as_ref().map(|d| &d.0),
        session,
        HISTORY_ENDPOINT,
    )
    .await?;
    match page(&record, query.after, query.limit.unwrap_or(8) as usize) {
        Ok(body) => Ok(Json(body).into_response()),
        Err(error) => {
            let (status, message) = match error {
                HistoryError::InvalidCursor => {
                    (StatusCode::BAD_REQUEST, "invalid turn history cursor")
                }
                HistoryError::Unverifiable => (
                    StatusCode::CONFLICT,
                    "turn history cannot be verified from the session log",
                ),
                HistoryError::TooLarge => (
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "one turn exceeds the history response limit",
                ),
            };
            Err(failure(status, ErrorCode::InvalidRequest, message))
        }
    }
}

fn page(record: &Session, after: u64, limit: usize) -> Result<HistoryPage<'_>, HistoryError> {
    let prefix = validate(record).map_err(|_| HistoryError::Unverifiable)?;
    if after > record.log.len() as u64 {
        return Err(HistoryError::InvalidCursor);
    }
    let starts: Vec<_> = record
        .log
        .iter()
        .enumerate()
        .filter(|(_, e)| matches!(e.event, SessionEventKind::TurnStarted))
        .collect();
    let mut result = HistoryPage {
        schema: 1,
        source: "session_log",
        session: record.id,
        revision: record.revision,
        turns: Vec::new(),
        next_cursor: after,
        has_more: false,
    };
    for (position, (index, start)) in starts.iter().enumerate() {
        if *index < prefix {
            continue;
        }
        // A local execution must have its own adjacent, original user input.
        if *index == 0 || *index == prefix {
            return Err(HistoryError::Unverifiable);
        }
        let input = committed::input_text(record, *index).ok_or(HistoryError::Unverifiable)?;
        let request = start.request.ok_or(HistoryError::Unverifiable)?;
        let trace = start.trace.ok_or(HistoryError::Unverifiable)?;
        let end = starts.get(position + 1).map_or(record.log.len(), |(i, _)| {
            committed::end_before_next_start(record, *i)
        });
        let completed = completion(record, *index, end, request, trace)
            .map_err(|_| HistoryError::Unverifiable)?;
        let cursor = *index as u64;
        if cursor <= after {
            continue;
        }
        if result.turns.len() == limit {
            result.has_more = true;
            break;
        }
        let previous_cursor = result.next_cursor;
        result.next_cursor = cursor;
        result.has_more = position + 1 < starts.len();
        result.turns.push(HistoryTurn {
            cursor,
            input_sha256: digest(input),
            input,
            request,
            trace_id: trace,
            state: if completed.is_some() {
                "completed"
            } else {
                "unconfirmed"
            },
            text: completed.map(|(text, _)| text),
            usage: None,
            recorded_at: completed.map(|(_, at)| at),
        });
        if !fits(&result) {
            result.turns.pop();
            if result.turns.is_empty() {
                return Err(HistoryError::TooLarge);
            }
            result.next_cursor = previous_cursor;
            result.has_more = true;
            break;
        }
    }
    Ok(result)
}

/// Count encoded bytes without allocating an unbounded encoded copy of a turn.
fn fits(page: &HistoryPage<'_>) -> bool {
    struct Budget(usize);
    impl Write for Budget {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0 = self
                .0
                .checked_add(bytes.len())
                .filter(|size| *size <= MAX_PAGE_BYTES)
                .ok_or_else(|| io::Error::other("history page byte budget exceeded"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Budget(0), page).is_ok()
}

#[cfg(test)]
mod committed_tests;
#[cfg(test)]
mod tests;
