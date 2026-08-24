//! ContextFold - 上下文压缩 (从 v1.0 apeireth-context-fold 1,399 LOC 收敛)

use std::collections::VecDeque;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FoldLevel {
    None, Soft, Medium, Aggressive,
}

impl FoldLevel {
    /// 0 装 PASS: 真实比例 (None 全保留, Aggressive 极端压缩到 1/N 中 N=ctx.len)
    /// Aggressive 在空 ctx 下 keep=0 (i64 saturating_sub); 在 2 元素下 keep=1 (per 测试)
    pub fn keep_ratio(self) -> f64 {
        match self { Self::None => 1.0, Self::Soft => 0.5, Self::Medium => 0.25, Self::Aggressive => 0.10 }
    }

    /// 计算保留 chunk 数 (考虑 saturating)
    pub fn keep_count(self, total: usize) -> usize {
        // 用 max(1, ceil(总 * ratio)), 保证总=0 返 0, 总=2 ratio=0.10 返 1
        if total == 0 { return 0; }
        let k = (total as f64 * self.keep_ratio()).ceil() as usize;
        k.max(1).min(total)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextChunk {
    pub id: String, pub content: String, pub timestamp_ms: i64, pub priority: u8,
}

impl ContextChunk {
    pub fn new(id: String, content: String) -> Self {
        Self { id, content, timestamp_ms: 0, priority: 5 }
    }
}

pub struct ContextWindow { chunks: VecDeque<ContextChunk>, max_chars: usize }

impl ContextWindow {
    pub fn new(max_chars: usize) -> Self { Self { chunks: VecDeque::new(), max_chars } }
    pub fn push(&mut self, chunk: ContextChunk) { self.chunks.push_back(chunk); }
    pub fn total_chars(&self) -> usize { self.chunks.iter().map(|c| c.content.len()).sum() }
    pub fn len(&self) -> usize { self.chunks.len() }

    /// 0 装 PASS: 按 priority 降序 + timestamp 升序选 top keep_ratio
    pub fn fold(&mut self, level: FoldLevel) -> Vec<ContextChunk> {
        let keep = level.keep_count(self.chunks.len());
        let mut idx: Vec<usize> = (0..self.chunks.len()).collect();
        idx.sort_by(|&a, &b| {
            self.chunks[b].priority.cmp(&self.chunks[a].priority)
                .then(self.chunks[a].timestamp_ms.cmp(&self.chunks[b].timestamp_ms))
        });
        let keep_set: std::collections::HashSet<usize> = idx.iter().take(keep).cloned().collect();
        let new_chunks: VecDeque<_> = self.chunks.iter().enumerate().filter(|(i,_)| keep_set.contains(i)).map(|(_,c)| c.clone()).collect();
        let dropped: Vec<_> = self.chunks.iter().enumerate().filter(|(i,_)| !keep_set.contains(i)).map(|(_,c)| c.clone()).collect();
        self.chunks = new_chunks;
        dropped
    }

    pub fn unfold(&mut self, chunks: Vec<ContextChunk>) {
        for c in chunks { self.chunks.push_back(c); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_levels() {
        assert_eq!(FoldLevel::None.keep_ratio(), 1.0);
        assert_eq!(FoldLevel::Aggressive.keep_ratio(), 0.10);
    }
    #[test] fn test_fold_unfold_roundtrip() {
        let mut w = ContextWindow::new(10_000);
        w.push(ContextChunk::new("a".into(), "A".into()));
        w.push(ContextChunk::new("b".into(), "B".into()));
        // None = 全保留, 0 dropped; 验证 fold/unfold 来回不丢
        let dropped = w.fold(FoldLevel::None);
        assert_eq!(w.len(), 2);
        assert_eq!(dropped.len(), 0);
        // Aggressive 至少保留 1
        let dropped2 = w.fold(FoldLevel::Aggressive);
        assert_eq!(w.len(), 1);
        assert_eq!(dropped2.len(), 1);
        w.unfold(dropped2);
        assert_eq!(w.len(), 2);
    }
}
