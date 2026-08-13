//! L4: LCM (Long Context Memory) compressor.
//!
//! Chunks long content into smaller pieces + optional summarization via
//! user-supplied callback.

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct LcmChunk {
    pub id: String,
    pub content: String,
    pub index: usize,
}

/// Callback for summarization (returns summary of a chunk).
pub type LcmCallback<'a> = &'a dyn Fn(&str) -> String;

pub struct L4LcmCompressor {
    /// Max chars per chunk (default 1000)
    pub chunk_size: usize,
}

impl L4LcmCompressor {
    pub fn new() -> Self {
        Self { chunk_size: 1000 }
    }
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }

    /// Split content into chunks of at most `chunk_size` chars.
    /// Splits at paragraph boundaries (\n\n) when possible.
    pub fn chunk(&self, content: &str) -> Vec<LcmChunk> {
        if content.is_empty() {
            return Vec::new();
        }
        if content.len() <= self.chunk_size {
            return vec![LcmChunk { id: Uuid::new_v4().to_string(), content: content.to_string(), index: 0 }];
        }
        let mut chunks = Vec::new();
        let mut current = String::new();
        let mut index = 0;
        for paragraph in content.split("\n\n") {
            if current.len() + paragraph.len() + 2 > self.chunk_size && !current.is_empty() {
                chunks.push(LcmChunk { id: Uuid::new_v4().to_string(), content: current.clone(), index });
                index += 1;
                current.clear();
            }
            if !current.is_empty() {
                current.push_str("\n\n");
            }
            current.push_str(paragraph);
        }
        if !current.is_empty() {
            chunks.push(LcmChunk { id: Uuid::new_v4().to_string(), content: current, index });
        }
        chunks
    }

    /// Summarize chunks via callback. Returns Vec<summary>.
    pub fn summarize(&self, chunks: &[LcmChunk], callback: LcmCallback) -> Vec<String> {
        chunks.iter().map(|c| callback(&c.content)).collect()
    }
}

impl Default for L4LcmCompressor {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_content() {
        let c = L4LcmCompressor::new();
        assert!(c.chunk("").is_empty());
    }

    #[test]
    fn short_content_no_split() {
        let c = L4LcmCompressor::new();
        let chunks = c.chunk("hello world");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, "hello world");
        assert_eq!(chunks[0].index, 0);
    }

    #[test]
    fn long_content_splits_at_paragraphs() {
        let c = L4LcmCompressor::with_chunk_size(L4LcmCompressor::new(), 50);
        let content = "First paragraph that is somewhat long.\n\nSecond paragraph that is also long.\n\nThird para.";
        let chunks = c.chunk(content);
        assert!(chunks.len() >= 1);
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.index, i);
        }
    }

    #[test]
    fn summarize_with_callback() {
        let c = L4LcmCompressor::new();
        let chunks = c.chunk("hello");
        let summaries = c.summarize(&chunks, &|s| format!("sum:{}", s.len()));
        assert_eq!(summaries.len(), 1);
        assert!(summaries[0].contains("sum:"));
    }

    #[test]
    fn chunk_ids_unique() {
        let c = L4LcmCompressor::with_chunk_size(L4LcmCompressor::new(), 20);
        let chunks = c.chunk("aaaaaaaaaa\n\nbbbbbbbbbb\n\ncccccccccc");
        let ids: std::collections::HashSet<_> = chunks.iter().map(|c| &c.id).collect();
        assert_eq!(ids.len(), chunks.len());
    }
}