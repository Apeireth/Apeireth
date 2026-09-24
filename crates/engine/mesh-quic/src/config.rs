//! Central transport limits. No magic numbers scattered through the code:
//! every bound a peer can pressure lives here, defaults conservative and
//! configurable.
//!
//! Ancestors of these numbers are load-bearing for safety — a malicious
//! peer must never be able to grow our memory without limit (frames are
//! length-checked before allocation; outbound queues are bounded).

/// Transport-level limits and timeouts for one QUIC mesh endpoint.
#[derive(Debug, Clone)]
pub struct QuicTransportConfig {
    /// Largest accepted/sent envelope frame, in bytes. The u32 length
    /// prefix is checked **before** any allocation, so an oversized frame
    /// is refused without a buffer ever forming. Provider payloads can be
    /// large (transcripts + tool declarations), so the default is
    /// generous-but-bounded rather than tiny.
    pub max_frame_size: u32,
    /// Bounded per-stream outbound queue depth (messages, not bytes).
    /// `send` reports [`MeshTransportError::Backpressure`](apeireth_mesh::MeshTransportError::Backpressure)
    /// when full — never an unbounded buffer.
    pub outbound_queue_capacity: usize,
    /// Dial timeout.
    pub connect_timeout_ms: u64,
    /// QUIC max idle timeout (peer silence closes the connection).
    pub idle_timeout_ms: u64,
    /// QUIC keepalive interval (transport assist only — the Mesh peer
    /// lifecycle remains the availability truth).
    pub keepalive_interval_ms: u64,
}

impl Default for QuicTransportConfig {
    fn default() -> Self {
        Self {
            max_frame_size: 8 * 1024 * 1024,
            outbound_queue_capacity: 256,
            connect_timeout_ms: 10_000,
            idle_timeout_ms: 30_000,
            keepalive_interval_ms: 5_000,
        }
    }
}

impl QuicTransportConfig {
    /// The shared ALPN protocol id for mesh QUIC channels.
    pub const ALPN: &'static [u8] = b"apeireth-mesh/2";
}
