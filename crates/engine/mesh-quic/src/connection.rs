//! [`QuicMeshConnection`]: envelope transport over one authenticated QUIC
//! connection.
//!
//! Stream model (Phase 2A, deliberately simple): each side opens **two
//! long-lived unidirectional streams** toward its peer —
//!
//! - `control`: `Heartbeat` / `Cancel` traffic;
//! - `data`: everything else (handshake, adverts, invokes, results).
//!
//! Splitting here is not speculative: a large provider payload in a single
//! stream would head-of-line-block a `Cancel` behind it, and the mesh contract
//! says cancellation must move promptly. QUIC multiplexes the two streams
//! independently.
//!
//! Bounds everywhere: the inbound merge queue is bounded, the per-stream
//! outbound queues are bounded (`send` reports backpressure rather than
//! buffering without limit), and every frame is length-checked before
//! allocation.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use apeireth_mesh::identity::KeyFingerprint;
use apeireth_mesh::{MeshConnection, MeshEnvelope, MeshPayload, MeshTransportError};
use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::config::QuicTransportConfig;
use crate::framing::{frame, read_frame, write_frame};

/// Stream kinds, carried as the first frame of each unidirectional stream.
const STREAM_CONTROL: u8 = 0;
const STREAM_DATA: u8 = 1;

/// One persistent authenticated connection between exactly two nodes.
pub struct QuicMeshConnection {
    outbound_control: mpsc::Sender<Vec<u8>>,
    outbound_data: mpsc::Sender<Vec<u8>>,
    inbound: tokio::sync::Mutex<mpsc::Receiver<MeshEnvelope>>,
    closed: Arc<AtomicBool>,
    connection: quinn::Connection,
    peer_fingerprint: Option<KeyFingerprint>,
    max_frame_size: u32,
}

impl QuicMeshConnection {
    /// Wrap an established QUIC connection: open the two outbound streams,
    /// start the inbound accept loop, and wire the bounded queues.
    pub(crate) async fn establish(
        connection: quinn::Connection,
        config: &QuicTransportConfig,
        peer_fingerprint: Option<KeyFingerprint>,
    ) -> Result<Arc<Self>, MeshTransportError> {
        let mut lanes = Vec::with_capacity(2);
        let mut writer_handles = Vec::with_capacity(2);
        let closed = Arc::new(AtomicBool::new(false));
        for kind in [STREAM_CONTROL, STREAM_DATA] {
            let send_stream = connection.open_uni().await.map_err(|error| {
                MeshTransportError::Io(format!("open stream {kind} failed: {error}"))
            })?;
            let (tx, mut rx) = mpsc::channel::<Vec<u8>>(config.outbound_queue_capacity);
            let closed_flag = Arc::clone(&closed);
            let max_frame_size = config.max_frame_size;
            writer_handles.push(tokio::spawn(async move {
                let mut send_stream = send_stream;
                // One-shot kind marker, then framed envelopes.
                if write_frame(&mut send_stream, &[kind], max_frame_size)
                    .await
                    .is_err()
                {
                    return;
                }
                while let Some(framed) = rx.recv().await {
                    if closed_flag.load(Ordering::SeqCst) {
                        return;
                    }
                    if write_frame(&mut send_stream, &framed, max_frame_size)
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
                let _ = send_stream.finish();
            }));
            lanes.push(tx);
        }

        // Inbound: accept unidirectional streams until the connection dies;
        // both kinds merge into one bounded queue (the kind only selects the
        // outbound lane — it never changes envelope semantics).
        let (inbound_tx, inbound_rx) =
            mpsc::channel::<MeshEnvelope>(config.outbound_queue_capacity);
        {
            let connection = connection.clone();
            let max_frame_size = config.max_frame_size;
            let closed_flag = Arc::clone(&closed);
            tokio::spawn(async move {
                while let Ok(mut stream) = connection.accept_uni().await {
                    let inbound_tx = inbound_tx.clone();
                    let closed_flag = Arc::clone(&closed_flag);
                    let connection = connection.clone();
                    tokio::spawn(async move {
                        // First frame is the stream kind marker.
                        match read_frame(&mut stream, max_frame_size).await {
                            Ok(Some(bytes)) if bytes.len() == 1 => {}
                            _ => return,
                        }
                        while !closed_flag.load(Ordering::SeqCst) {
                            match read_frame(&mut stream, max_frame_size).await {
                                Ok(Some(bytes)) => {
                                    // All network input is untrusted: a
                                    // malformed payload fails the whole
                                    // connection closed — never a panic, never
                                    // a silent half-alive link.
                                    let Ok(envelope) =
                                        serde_json::from_slice::<MeshEnvelope>(&bytes)
                                    else {
                                        closed_flag.store(true, Ordering::SeqCst);
                                        connection.close(0u32.into(), b"mesh: malformed frame");
                                        return;
                                    };
                                    if inbound_tx.send(envelope).await.is_err() {
                                        return;
                                    }
                                }
                                Ok(None) => return,
                                Err(error) => {
                                    // Oversized/truncated frames are protocol
                                    // violations: close the connection.
                                    if matches!(error, MeshTransportError::ProtocolViolation(_)) {
                                        closed_flag.store(true, Ordering::SeqCst);
                                        connection.close(0u32.into(), b"mesh: frame violation");
                                    }
                                    return;
                                }
                            }
                        }
                    });
                }
            });
        }
        // Keep writer tasks alive for the connection lifetime.
        tokio::spawn(async move {
            for handle in writer_handles {
                let _ = handle.await;
            }
        });

        let mut lanes = lanes.into_iter();
        let (Some(outbound_control), Some(outbound_data)) = (lanes.next(), lanes.next()) else {
            return Err(MeshTransportError::Io(
                "stream lanes mis-initialized".to_string(),
            ));
        };
        Ok(Arc::new(Self {
            outbound_control,
            outbound_data,
            inbound: tokio::sync::Mutex::new(inbound_rx),
            closed,
            connection,
            peer_fingerprint,
            max_frame_size: config.max_frame_size,
        }))
    }
}

#[async_trait]
impl MeshConnection for QuicMeshConnection {
    async fn send(&self, envelope: MeshEnvelope) -> Result<(), MeshTransportError> {
        if self.is_closed() {
            return Err(MeshTransportError::Closed("connection closed".to_string()));
        }
        // Control traffic rides its own lane so a large payload can never
        // head-of-line-block a cancellation or heartbeat.
        let lane = match &envelope.payload {
            MeshPayload::Heartbeat(_) | MeshPayload::Cancel(_) => &self.outbound_control,
            _ => &self.outbound_data,
        };
        let bytes = serde_json::to_vec(&envelope).map_err(|error| {
            MeshTransportError::ProtocolViolation(format!("envelope not encodable: {error}"))
        })?;
        // Frame size is enforced here so an oversized envelope is refused at
        // `send` (typed) rather than deep in the writer task.
        let _checked = frame(&bytes, self.max_frame_size)?;
        lane.try_send(bytes).map_err(|error| match error {
            mpsc::error::TrySendError::Full(_) => {
                MeshTransportError::Backpressure("outbound queue full".to_string())
            }
            mpsc::error::TrySendError::Closed(_) => {
                MeshTransportError::Closed("stream writer gone".to_string())
            }
        })
    }

    async fn recv(&self) -> Result<MeshEnvelope, MeshTransportError> {
        let mut inbound = self.inbound.lock().await;
        match inbound.recv().await {
            Some(envelope) => Ok(envelope),
            None => {
                self.closed.store(true, Ordering::SeqCst);
                Err(MeshTransportError::Closed("connection closed".to_string()))
            }
        }
    }

    fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        self.connection.close(0u32.into(), b"mesh: closed");
    }

    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst) || self.connection.close_reason().is_some()
    }

    fn peer_fingerprint(&self) -> Option<KeyFingerprint> {
        self.peer_fingerprint.clone()
    }
}
