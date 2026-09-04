//! WS v1 session contract: the state machine that gates [`WsFrame`] traffic on
//! a single WebSocket connection.
//!
//! Recovered from the legacy `apeireth-api::ws_v1` server handler (R20 阶段 2),
//! which enforced the blueprint's connection rules inline in an axum task. The
//! rules are protocol semantics, not transport semantics, so they are
//! expressed here as a frame-level decision function that any transport
//! (axum, tokio-tungstenite, tests) can drive:
//!
//! 1. **Auth-first gate** — the first frame on a connection MUST be
//!    [`WsFrame::Auth`]; anything else is closed with `1008 ws_unauthorized`
//!    (legacy 1:1).
//! 2. **Version negotiation** — an [`AuthFrame`] whose `ws_version` differs
//!    from [`WS_PROTOCOL_VERSION`] is closed with `1008 ws_version_mismatch`.
//!    This is the negotiation hook the blueprint promised ("一旦 bump 必跟
//!    upgrade 校验同步").
//! 3. **Direction enforcement** — server-only frames (`ToolResult`,
//!    `StreamChunk`, `StreamEnd`, `Error`) sent by a client are answered with a
//!    non-fatal `invalid_direction` [`ErrorFrame`] (legacy 1:1).
//! 4. **Close-code taxonomy** — `1000` normal, `1008` auth/version failure,
//!    `1013` concurrency limit, `4xxx` business errors (blueprint §2.5).
//!
//! Re-authentication in the `Open` state is allowed (the legacy handler
//! re-validated a second `Auth` frame rather than rejecting it).
//!
//! Pure frame logic: no socket, no runtime, fully deterministic.

use crate::ws_v1::{AuthFrame, CloseFrame, ErrorFrame, WsFrame, WS_PROTOCOL_VERSION};

/// Close code: normal closure (client done) — blueprint §2.5.
pub const WS_CLOSE_NORMAL: u16 = 1000;

/// Close code: authentication or version-negotiation failure — blueprint §2.5.
pub const WS_CLOSE_UNAUTHORIZED: u16 = 1008;

/// Close code: too many concurrent sessions — blueprint §2.5.
pub const WS_CLOSE_TOO_MANY_CONCURRENT: u16 = 1013;

/// The two connection states of a WS v1 session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsSessionState {
    /// Pre-handshake: only [`WsFrame::Auth`] is acceptable.
    AwaitingAuth,
    /// Handshake complete: business frames are acceptable.
    Open,
}

/// What a transport should do with a frame the session received.
#[derive(Debug, Clone, PartialEq)]
pub enum WsFrameDecision {
    /// Frame is contract-allowed; hand it to the session's business logic.
    Deliver,
    /// Reply with a non-fatal error frame and keep the connection.
    Reply(ErrorFrame),
    /// Close the connection with the given close frame.
    Close(CloseFrame),
}

/// One connection's session guard: carries the handshake state and renders
/// every inbound frame into a [`WsFrameDecision`].
#[derive(Debug, Clone)]
pub struct WsSessionGuard {
    state: WsSessionState,
}

impl WsSessionGuard {
    /// A fresh connection: [`WsSessionState::AwaitingAuth`].
    pub fn new() -> Self {
        Self {
            state: WsSessionState::AwaitingAuth,
        }
    }

    /// Current handshake state.
    pub fn state(&self) -> WsSessionState {
        self.state
    }

    /// Whether the handshake completed successfully.
    pub fn is_open(&self) -> bool {
        self.state == WsSessionState::Open
    }

    /// Admit one inbound frame, advancing the handshake state when the frame
    /// is an accepted [`WsFrame::Auth`].
    ///
    /// This is the legacy `handle_ws_session` gate, frame-level and
    /// side-effect-free: it never panics, never rejects ambiguous frames
    /// silently, and distinguishes "process this" (`Deliver`) from "answer
    /// then continue" (`Reply`) and "tear down" (`Close`).
    pub fn admit(&mut self, frame: &WsFrame) -> WsFrameDecision {
        match self.state {
            WsSessionState::AwaitingAuth => {
                if let WsFrame::Auth(auth) = frame {
                    self.admit_auth(auth)
                } else {
                    // Anything else pre-auth — including an early Close — is
                    // refused with 1008 (legacy 1:1: close ws_unauthorized).
                    WsFrameDecision::Close(CloseFrame {
                        reason: "ws_unauthorized".into(),
                        code: WS_CLOSE_UNAUTHORIZED,
                    })
                }
            }
            WsSessionState::Open => match frame {
                WsFrame::ToolInvoke(_) | WsFrame::Ping(_) | WsFrame::Close(_) => {
                    WsFrameDecision::Deliver
                }
                // Re-auth is allowed (legacy handler re-validated a second
                // Auth frame instead of rejecting it).
                WsFrame::Auth(auth) => self.admit_auth(auth),
                // Server-only frames must never arrive from a client.
                WsFrame::ToolResult(_)
                | WsFrame::StreamChunk(_)
                | WsFrame::StreamEnd(_)
                | WsFrame::Error(_) => WsFrameDecision::Reply(ErrorFrame {
                    code: "invalid_direction".into(),
                    message: "server-only frame sent by client".into(),
                    fatal: false,
                }),
            },
        }
    }

    /// Version-negotiating auth admission (shared by both states).
    fn admit_auth(&mut self, auth: &AuthFrame) -> WsFrameDecision {
        if auth.ws_version != WS_PROTOCOL_VERSION {
            // Version negotiation failure: close 1008 (legacy 1:1 with the
            // blueprint's "mismatch → close 1008" rule).
            return WsFrameDecision::Close(CloseFrame {
                reason: "ws_version_mismatch".into(),
                code: WS_CLOSE_UNAUTHORIZED,
            });
        }
        self.state = WsSessionState::Open;
        WsFrameDecision::Deliver
    }
}

impl Default for WsSessionGuard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ws_v1::{PingFrame, StreamChunkFrame, ToolInvokeFrame, ToolResultFrame};

    fn auth(version: &str) -> WsFrame {
        WsFrame::Auth(AuthFrame {
            token: "ws-tok-abc".into(),
            ws_version: version.into(),
        })
    }

    fn invoke() -> WsFrame {
        WsFrame::ToolInvoke(ToolInvokeFrame {
            tool: "web_search".into(),
            action: "search".into(),
            args: serde_json::json!({"query": "apeireth"}),
            req_id: "r-001".into(),
        })
    }

    #[test]
    fn auth_first_gate_closes_1008() {
        // Legacy 1:1: non-auth frame before auth → close 1008 ws_unauthorized.
        let mut guard = WsSessionGuard::new();
        let d = guard.admit(&invoke());
        match d {
            WsFrameDecision::Close(cf) => {
                assert_eq!(cf.code, WS_CLOSE_UNAUTHORIZED);
                assert_eq!(cf.reason, "ws_unauthorized");
            }
            other => panic!("expected Close, got {other:?}"),
        }
        assert!(!guard.is_open());
    }

    #[test]
    fn valid_auth_opens_the_session() {
        let mut guard = WsSessionGuard::new();
        assert_eq!(
            guard.admit(&auth(WS_PROTOCOL_VERSION)),
            WsFrameDecision::Deliver
        );
        assert!(guard.is_open());
        // Business frame now delivers.
        assert_eq!(guard.admit(&invoke()), WsFrameDecision::Deliver);
    }

    #[test]
    fn version_mismatch_closes_1008() {
        // Version negotiation: ws_version != WS_PROTOCOL_VERSION → close 1008.
        let mut guard = WsSessionGuard::new();
        let d = guard.admit(&auth("2"));
        match d {
            WsFrameDecision::Close(cf) => {
                assert_eq!(cf.code, WS_CLOSE_UNAUTHORIZED);
                assert_eq!(cf.reason, "ws_version_mismatch");
            }
            other => panic!("expected Close, got {other:?}"),
        }
        assert!(
            !guard.is_open(),
            "failed negotiation must not open the session"
        );
    }

    #[test]
    fn server_only_frames_from_client_are_non_fatal_errors() {
        // Legacy 1:1: ToolResult/StreamChunk/StreamEnd/Error from a client →
        // invalid_direction error frame, connection continues.
        let mut guard = WsSessionGuard::new();
        guard.admit(&auth(WS_PROTOCOL_VERSION));

        let server_only = vec![
            WsFrame::ToolResult(ToolResultFrame {
                req_id: "r".into(),
                ok: true,
                data: None,
                error: None,
                meta: serde_json::Value::Null,
            }),
            WsFrame::StreamChunk(StreamChunkFrame {
                req_id: "r".into(),
                chunk: "x".into(),
                done: false,
            }),
            WsFrame::StreamEnd(crate::ws_v1::StreamEndFrame {
                req_id: "r".into(),
                total_chunks: 0,
                total_bytes: 0,
            }),
            WsFrame::Error(ErrorFrame {
                code: "boom".into(),
                message: "boom".into(),
                fatal: false,
            }),
        ];
        for f in server_only {
            match guard.admit(&f) {
                WsFrameDecision::Reply(err) => {
                    assert_eq!(err.code, "invalid_direction");
                    assert!(!err.fatal);
                }
                other => panic!("expected Reply for {}, got {other:?}", f.type_str()),
            }
        }
        assert!(guard.is_open(), "invalid_direction must be non-fatal");
    }

    #[test]
    fn ping_and_close_deliver_when_open() {
        let mut guard = WsSessionGuard::new();
        guard.admit(&auth(WS_PROTOCOL_VERSION));
        assert_eq!(
            guard.admit(&WsFrame::Ping(PingFrame { ts: 1 })),
            WsFrameDecision::Deliver
        );
        assert_eq!(
            guard.admit(&WsFrame::Close(crate::ws_v1::CloseFrame {
                reason: "client_done".into(),
                code: WS_CLOSE_NORMAL,
            })),
            WsFrameDecision::Deliver
        );
    }

    #[test]
    fn reauth_is_permitted_and_renegotiates_version() {
        let mut guard = WsSessionGuard::new();
        guard.admit(&auth(WS_PROTOCOL_VERSION));
        // Legacy behavior: a second Auth frame re-validates rather than rejects.
        assert_eq!(
            guard.admit(&auth(WS_PROTOCOL_VERSION)),
            WsFrameDecision::Deliver
        );
        assert!(guard.is_open());
        // A later bad version still closes.
        assert!(matches!(
            guard.admit(&auth("9")),
            WsFrameDecision::Close(CloseFrame {
                code: WS_CLOSE_UNAUTHORIZED,
                ..
            })
        ));
    }

    #[test]
    fn close_code_taxonomy_constants() {
        // Blueprint §2.5: 1000 normal / 1008 unauthorized / 1013 concurrency.
        assert_eq!(WS_CLOSE_NORMAL, 1000);
        assert_eq!(WS_CLOSE_UNAUTHORIZED, 1008);
        assert_eq!(WS_CLOSE_TOO_MANY_CONCURRENT, 1013);
    }

    #[test]
    fn early_client_close_is_honored() {
        let mut guard = WsSessionGuard::new();
        assert!(matches!(
            guard.admit(&WsFrame::Close(CloseFrame {
                reason: "bye".into(),
                code: WS_CLOSE_NORMAL,
            })),
            WsFrameDecision::Close(_)
        ));
    }
}
