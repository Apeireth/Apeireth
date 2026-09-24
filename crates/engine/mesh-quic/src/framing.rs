//! Length-prefixed framing over QUIC streams.
//!
//! `u32 big-endian length || payload bytes`. Message boundaries are
//! explicit — never `read_to_end`. The length is validated against
//! `max_frame_size` **before** the payload buffer is allocated, so a
//! hostile peer cannot inflate our memory with a forged length.

use apeireth_mesh::MeshTransportError;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Encode one frame (length prefix + payload).
pub(crate) fn frame(payload: &[u8], max_frame_size: u32) -> Result<Vec<u8>, MeshTransportError> {
    let len = u32::try_from(payload.len()).map_err(|_| {
        MeshTransportError::ProtocolViolation("frame exceeds u32 length space".to_string())
    })?;
    if len > max_frame_size {
        return Err(MeshTransportError::ProtocolViolation(format!(
            "outbound frame {len} bytes exceeds max {max_frame_size}"
        )));
    }
    let mut out = Vec::with_capacity(payload.len() + 4);
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(payload);
    Ok(out)
}

/// Read one frame from a QUIC stream.
///
/// Returns `Ok(None)` at clean end-of-stream. All failure shapes —
/// truncation, oversized length, I/O error — are typed errors; nothing
/// here panics on hostile input.
pub(crate) async fn read_frame<S: AsyncReadExt + Unpin>(
    stream: &mut S,
    max_frame_size: u32,
) -> Result<Option<Vec<u8>>, MeshTransportError> {
    let mut prefix = [0u8; 4];
    let mut filled = 0usize;
    while filled < 4 {
        match stream.read(&mut prefix[filled..]).await {
            Ok(0) => {
                return if filled == 0 {
                    Ok(None)
                } else {
                    Err(MeshTransportError::ProtocolViolation(
                        "truncated frame length prefix".to_string(),
                    ))
                };
            }
            Ok(n) => filled += n,
            Err(error) => {
                return Err(MeshTransportError::Io(error.to_string()));
            }
        }
    }
    let len = u32::from_be_bytes(prefix);
    if len > max_frame_size {
        return Err(MeshTransportError::ProtocolViolation(format!(
            "inbound frame {len} bytes exceeds max {max_frame_size}"
        )));
    }
    let mut payload = vec![0u8; len as usize];
    let mut filled = 0usize;
    while filled < payload.len() {
        match stream.read(&mut payload[filled..]).await {
            Ok(0) => {
                return Err(MeshTransportError::ProtocolViolation(
                    "truncated frame payload".to_string(),
                ));
            }
            Ok(n) => filled += n,
            Err(error) => {
                return Err(MeshTransportError::Io(error.to_string()));
            }
        }
    }
    Ok(Some(payload))
}

/// Write one already-framed buffer to a QUIC stream.
pub(crate) async fn write_frame<S: AsyncWriteExt + Unpin>(
    stream: &mut S,
    payload: &[u8],
    max_frame_size: u32,
) -> Result<(), MeshTransportError> {
    let framed = frame(payload, max_frame_size)?;
    stream
        .write_all(&framed)
        .await
        .map_err(|error| MeshTransportError::Io(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_round_trips_and_oversized_is_refused_before_allocation() {
        let framed = frame(b"hello", 1024).unwrap();
        assert_eq!(&framed[..4], &5u32.to_be_bytes());
        assert_eq!(&framed[4..], b"hello");

        let err = frame(&[0u8; 2048], 1024).unwrap_err();
        assert!(matches!(err, MeshTransportError::ProtocolViolation(_)));
    }

    #[tokio::test]
    async fn hostile_lengths_are_refused_and_truncation_is_typed() {
        // Oversized declared length: refused before any buffer forms.
        let hostile = 4096u32.to_be_bytes().to_vec();
        let err = read_frame(&mut &hostile[..], 1024).await.unwrap_err();
        assert!(matches!(err, MeshTransportError::ProtocolViolation(_)));

        // Truncated prefix.
        let err = read_frame(&mut &1u32.to_be_bytes()[..2], 1024)
            .await
            .unwrap_err();
        assert!(matches!(err, MeshTransportError::ProtocolViolation(_)));

        // Truncated payload.
        let mut short = 8u32.to_be_bytes().to_vec();
        short.extend_from_slice(&[0u8; 3]);
        let err = read_frame(&mut &short[..], 1024).await.unwrap_err();
        assert!(matches!(err, MeshTransportError::ProtocolViolation(_)));
    }
}
