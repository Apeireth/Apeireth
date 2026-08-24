//! Stream chunks for SSE.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamChunk {
    Text(String),
    Done,
    Error(String),
}

/// Stream text to a sender.
pub async fn stream_to_sender<S>(text: String, mut sender: S) -> Result<(), String>
where
    S: FnMut(StreamChunk) -> Result<(), String>,
{
    for ch in text.chars().collect::<Vec<_>>().chunks(64) {
        let s: String = ch.iter().collect();
        sender(StreamChunk::Text(s))?;
    }
    sender(StreamChunk::Done)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_to_sender_basic() {
        let collected = std::sync::Arc::new(parking_lot::Mutex::new(Vec::new()));
        let c2 = collected.clone();
        let result = futures::executor::block_on(async {
            stream_to_sender("hello world".to_string(), |chunk| {
                c2.lock().push(chunk);
                Ok(())
            }).await
        });
        assert!(result.is_ok());
        assert!(collected.lock().len() > 1);
        assert!(matches!(collected.lock().last().unwrap(), StreamChunk::Done));
    }

    #[test]
    fn stream_chunk_variants() {
        let c1 = StreamChunk::Text("x".into());
        let c2 = StreamChunk::Done;
        let c3 = StreamChunk::Error("e".into());
        assert!(matches!(c1, StreamChunk::Text(_)));
        assert!(matches!(c2, StreamChunk::Done));
        assert!(matches!(c3, StreamChunk::Error(_)));
    }
}
