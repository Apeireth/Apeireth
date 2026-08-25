//! Runtime transport bridges layered above protocol normalization.

use std::collections::VecDeque;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeKind {
    Stream,
    Queue,
    Passthrough,
}

pub trait ExtendedBridge {
    fn bridge_kind(&self) -> BridgeKind;
    fn pending_items(&self) -> usize;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BridgeExtError {
    InvalidUtf8,
    QueueFull { capacity: usize },
    ZeroCapacity,
}

impl fmt::Display for BridgeExtError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUtf8 => formatter.write_str("stream contains invalid UTF-8"),
            Self::QueueFull { capacity } => {
                write!(formatter, "bridge queue is full (capacity={capacity})")
            }
            Self::ZeroCapacity => {
                formatter.write_str("bridge queue capacity must be greater than zero")
            }
        }
    }
}

impl Error for BridgeExtError {}

#[derive(Debug, Default, Clone)]
pub struct StreamBridge {
    buffer: Vec<u8>,
    completed_chunks: usize,
}

impl StreamBridge {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn push_chunk(&mut self, chunk: impl AsRef<[u8]>) {
        self.buffer.extend_from_slice(chunk.as_ref());
        self.completed_chunks += 1;
    }
    pub fn finish(&mut self) -> Result<String, BridgeExtError> {
        let bytes = std::mem::take(&mut self.buffer);
        self.completed_chunks = 0;
        String::from_utf8(bytes).map_err(|_| BridgeExtError::InvalidUtf8)
    }
    pub fn chunk_count(&self) -> usize {
        self.completed_chunks
    }
}

impl ExtendedBridge for StreamBridge {
    fn bridge_kind(&self) -> BridgeKind {
        BridgeKind::Stream
    }
    fn pending_items(&self) -> usize {
        self.completed_chunks
    }
}

#[derive(Debug, Clone)]
pub struct QueueBridge<T> {
    capacity: usize,
    queue: VecDeque<T>,
}

impl<T> QueueBridge<T> {
    pub fn new(capacity: usize) -> Result<Self, BridgeExtError> {
        if capacity == 0 {
            return Err(BridgeExtError::ZeroCapacity);
        }
        Ok(Self {
            capacity,
            queue: VecDeque::with_capacity(capacity),
        })
    }
    pub fn enqueue(&mut self, item: T) -> Result<(), BridgeExtError> {
        if self.queue.len() == self.capacity {
            return Err(BridgeExtError::QueueFull {
                capacity: self.capacity,
            });
        }
        self.queue.push_back(item);
        Ok(())
    }
    pub fn dequeue(&mut self) -> Option<T> {
        self.queue.pop_front()
    }
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl<T> ExtendedBridge for QueueBridge<T> {
    fn bridge_kind(&self) -> BridgeKind {
        BridgeKind::Queue
    }
    fn pending_items(&self) -> usize {
        self.queue.len()
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct PassthroughBridge;

impl PassthroughBridge {
    pub fn bridge<T>(&self, payload: T) -> T {
        payload
    }
}

impl ExtendedBridge for PassthroughBridge {
    fn bridge_kind(&self) -> BridgeKind {
        BridgeKind::Passthrough
    }
    fn pending_items(&self) -> usize {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_assembles_chunks() {
        let mut bridge = StreamBridge::new();
        bridge.push_chunk("hello ");
        bridge.push_chunk("world");
        assert_eq!(bridge.chunk_count(), 2);
        assert_eq!(bridge.finish().unwrap(), "hello world");
        assert_eq!(bridge.pending_items(), 0);
    }
    #[test]
    fn stream_supports_split_utf8_codepoint() {
        let mut bridge = StreamBridge::new();
        let bytes = "\u{54f2}".as_bytes();
        bridge.push_chunk(&bytes[..1]);
        bridge.push_chunk(&bytes[1..]);
        assert_eq!(bridge.finish().unwrap(), "\u{54f2}");
    }
    #[test]
    fn stream_rejects_invalid_utf8_and_resets() {
        let mut bridge = StreamBridge::new();
        bridge.push_chunk([0xff]);
        assert_eq!(bridge.finish(), Err(BridgeExtError::InvalidUtf8));
        assert_eq!(bridge.pending_items(), 0);
    }
    #[test]
    fn queue_is_bounded_fifo() {
        let mut bridge = QueueBridge::new(2).unwrap();
        bridge.enqueue(1).unwrap();
        bridge.enqueue(2).unwrap();
        assert_eq!(
            bridge.enqueue(3),
            Err(BridgeExtError::QueueFull { capacity: 2 })
        );
        assert_eq!(bridge.dequeue(), Some(1));
        assert_eq!(bridge.dequeue(), Some(2));
    }
    #[test]
    fn queue_rejects_zero_capacity() {
        assert_eq!(
            QueueBridge::<u8>::new(0).unwrap_err(),
            BridgeExtError::ZeroCapacity
        );
    }
    #[test]
    fn passthrough_preserves_payload() {
        assert_eq!(PassthroughBridge.bridge(vec![1, 2, 3]), vec![1, 2, 3]);
    }
    #[test]
    fn bridge_kinds_are_explicit() {
        assert_eq!(StreamBridge::new().bridge_kind(), BridgeKind::Stream);
        assert_eq!(
            QueueBridge::<u8>::new(1).unwrap().bridge_kind(),
            BridgeKind::Queue
        );
        assert_eq!(PassthroughBridge.bridge_kind(), BridgeKind::Passthrough);
    }
}
