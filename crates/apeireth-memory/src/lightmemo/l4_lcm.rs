//! L4: LCM (Long Context Memory) compressor.
//!
//! Chunks long content into smaller pieces + optional summarization via
//! user-supplied callback.

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LcmChunk {
    pub id: String,
    pub content: String,
    pub index: usize,
    /// P0-6: chunk 策略版本 (跟 L4LcmCompressor::current_version() 对齐).
    ///
    /// 当 chunk 策略改变 (chunk_size / 切分逻辑) 时, bump L4LcmCompressor 的
    /// 版本号. 旧 chunk 的 version 跟新 current_version 不一致时, 读侧
    /// 通过 `is_stale(current_version)` 判定为 stale, 应该被 re-chunked.
    pub chunk_strategy_version: u32,
}

impl LcmChunk {
    /// 判定此 chunk 是否跟当前 strategy 版本一致 (false = stale).
    ///
    /// P0-6: 不一致的 chunk 应当被 caller 重 chunked (重 embed + 重索引),
    /// 而不是用旧分片提供错误上下文.
    pub fn is_stale(&self, current_version: u32) -> bool {
        self.chunk_strategy_version != current_version
    }
}

/// Callback for summarization (returns summary of a chunk).
pub type LcmCallback<'a> = &'a dyn Fn(&str) -> String;

/// P0-6: 当前 chunk 策略版本号.
///
/// ## 演进历史
/// - **v1** (默认, 初始): chunk_size = 1000, 切分 `\\n\\n` 段落.
/// - **v2**: bump 时由 owner 拍板, 这里是未来 hook.
///
/// ## 何时 bump
/// - 改 chunk_size 默认值
/// - 改切分算法 (paragraph → sentence / token / semantic)
/// - 改内容 normalize 逻辑 (trim / lowercase / 等)
///
/// ## 不应 bump 时
/// - 仅修 bug (bugfix 是"行为修正", 不算"策略变更")
/// - 加新方法 (向后兼容)
pub const CURRENT_CHUNK_STRATEGY_VERSION: u32 = 1;

pub struct L4LcmCompressor {
    /// Max chars per chunk (default 1000)
    pub chunk_size: usize,
    /// P0-6: chunk 策略版本 (跟 CURRENT_CHUNK_STRATEGY_VERSION 对齐).
    ///
    /// bump 后, 用旧 compressor 切的 chunk 会 `is_stale(new_compressor.current_version()) == true`.
    pub chunk_strategy_version: u32,
}

impl L4LcmCompressor {
    pub fn new() -> Self {
        Self { chunk_size: 1000, chunk_strategy_version: CURRENT_CHUNK_STRATEGY_VERSION }
    }
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }
    /// P0-6: 显式设 chunk_strategy_version (用于 v1 -> v2 migration 测试).
    pub fn with_version(mut self, version: u32) -> Self {
        self.chunk_strategy_version = version;
        self
    }
    /// P0-6: 取当前 compressor 的 chunk_strategy_version.
    pub fn current_version(&self) -> u32 {
        self.chunk_strategy_version
    }

    /// Split content into chunks of at most `chunk_size` chars.
    /// Splits at paragraph boundaries (\n\n) when possible.
    pub fn chunk(&self, content: &str) -> Vec<LcmChunk> {
        if content.is_empty() {
            return Vec::new();
        }
        if content.len() <= self.chunk_size {
            return vec![LcmChunk {
                id: Uuid::new_v4().to_string(),
                content: content.to_string(),
                index: 0,
                chunk_strategy_version: self.chunk_strategy_version,
            }];
        }
        let mut chunks = Vec::new();
        let mut current = String::new();
        let mut index = 0;
        for paragraph in content.split("\n\n") {
            if current.len() + paragraph.len() + 2 > self.chunk_size && !current.is_empty() {
                chunks.push(LcmChunk {
                    id: Uuid::new_v4().to_string(),
                    content: current.clone(),
                    index,
                    chunk_strategy_version: self.chunk_strategy_version,
                });
                index += 1;
                current.clear();
            }
            if !current.is_empty() {
                current.push_str("\n\n");
            }
            current.push_str(paragraph);
        }
        if !current.is_empty() {
            chunks.push(LcmChunk {
                id: Uuid::new_v4().to_string(),
                content: current,
                index,
                chunk_strategy_version: self.chunk_strategy_version,
            });
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

    // =====================================================================
    // P0-6: chunk_strategy_version + is_stale 判定
    // =====================================================================

    /// 1. 默认 compressor 切出的 chunk 都带当前版本号
    #[test]
    fn default_chunks_have_current_version() {
        let c = L4LcmCompressor::new();
        assert_eq!(c.current_version(), CURRENT_CHUNK_STRATEGY_VERSION);
        let chunks = c.chunk("anything");
        for ch in &chunks {
            assert_eq!(ch.chunk_strategy_version, CURRENT_CHUNK_STRATEGY_VERSION);
        }
    }

    /// 2. 多 chunk 场景也带版本号
    #[test]
    fn multi_chunks_have_current_version() {
        let c = L4LcmCompressor::with_chunk_size(L4LcmCompressor::new(), 20);
        let chunks = c.chunk("aaaaaaaaaa\n\nbbbbbbbbbb\n\ncccccccccc");
        assert!(chunks.len() > 1);
        for ch in &chunks {
            assert_eq!(ch.chunk_strategy_version, CURRENT_CHUNK_STRATEGY_VERSION);
        }
    }

    /// 3. with_version() 设老版本
    #[test]
    fn with_version_assigns_old_strategy() {
        let c = L4LcmCompressor::new().with_version(1);
        let chunks = c.chunk("hello world");
        for ch in &chunks {
            assert_eq!(ch.chunk_strategy_version, 1);
        }
    }

    /// 4. is_stale: chunk version == current -> false
    #[test]
    fn chunk_is_not_stale_when_versions_match() {
        let ch = LcmChunk {
            id: "x".into(),
            content: "x".into(),
            index: 0,
            chunk_strategy_version: CURRENT_CHUNK_STRATEGY_VERSION,
        };
        assert!(!ch.is_stale(CURRENT_CHUNK_STRATEGY_VERSION));
    }

    /// 5. is_stale: chunk version < current -> true
    #[test]
    fn chunk_is_stale_when_version_older() {
        let old = LcmChunk {
            id: "x".into(),
            content: "x".into(),
            index: 0,
            chunk_strategy_version: 1,
        };
        let new_version = CURRENT_CHUNK_STRATEGY_VERSION.max(2);
        assert!(old.is_stale(new_version));
    }

    /// 6. 迁移场景: v1 chunks 在 v2 视角下全 stale
    #[test]
    fn migration_v1_to_v2_marks_all_stale() {
        let v1 = L4LcmCompressor::new().with_version(1);
        let v1_chunks = v1.chunk("hello world");
        assert!(!v1_chunks.is_empty());
        let v2_version = 2;
        let stale_count = v1_chunks.iter().filter(|ch| ch.is_stale(v2_version)).count();
        assert_eq!(stale_count, v1_chunks.len());
    }

    /// 7. CURRENT_CHUNK_STRATEGY_VERSION 常量是 1
    #[test]
    fn current_version_constant_is_one() {
        assert_eq!(CURRENT_CHUNK_STRATEGY_VERSION, 1);
    }
}