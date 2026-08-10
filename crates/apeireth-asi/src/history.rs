//! TraceRepository — in-memory append-only trace storage (round10-12 qa_engineer)
//!
//! 设计原则 (Ponytail):
//! - in-memory RingBuffer, append-only (无 update/delete API)
//! - 提供 tail(N) / trend(name, N) / diagnose(N) 三类查询
//! - 兼容未来 SQLite backend (接口预留, 当前仅内存实现)

use std::collections::VecDeque;

use crate::DimensionTrace;

/// TraceRepository — 追加式 trace 仓库 (内存版, 后续可替换为 SQLite)。
#[derive(Debug, Clone)]
pub struct TraceRepository {
    /// 内部环形缓冲 (保留最近 max_traces 条)。
    traces: VecDeque<DimensionTrace>,
    /// 最大保留条数 (避免内存无限增长)。
    max_traces: usize,
    /// 下一个 trace_id (单调递增)。
    next_trace_id: u64,
    /// 下一个 sample_id (单调递增)。
    next_sample_id: u64,
}

impl Default for TraceRepository {
    fn default() -> Self {
        Self {
            traces: VecDeque::new(),
            max_traces: 10_000,
            next_trace_id: 1,
            next_sample_id: 1,
        }
    }
}

impl TraceRepository {
    /// 创建新仓库。
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建指定容量的仓库。
    pub fn with_capacity(max_traces: usize) -> Self {
        Self {
            traces: VecDeque::with_capacity(max_traces.min(1024)),
            max_traces,
            next_trace_id: 1,
            next_sample_id: 1,
        }
    }

    /// 追加一条 trace (append-only, 不可修改既有 trace)。
    pub fn append(&mut self, mut trace: DimensionTrace) -> u64 {
        if trace.trace_id == 0 {
            trace.trace_id = self.next_trace_id;
            self.next_trace_id += 1;
        }
        if trace.sample_id == 0 {
            trace.sample_id = self.next_sample_id;
            self.next_sample_id += 1;
        }
        let id = trace.trace_id;
        if self.traces.len() >= self.max_traces {
            self.traces.pop_front();
        }
        self.traces.push_back(trace);
        id
    }

    /// 获取最近 N 条 trace (tail)。
    pub fn tail(&self, n: usize) -> Vec<DimensionTrace> {
        let start = self.traces.len().saturating_sub(n);
        self.traces.iter().skip(start).cloned().collect()
    }

    /// 获取某维度/子测度最近 N 个值 (用于 trend)。
    /// `name` 可以是 V05_DIMENSION_NAMES 或 V1136_SUBMEASURE_NAMES 中的任意名。
    pub fn trend(&self, name: &str, n: usize) -> Vec<f64> {
        let recent: Vec<&DimensionTrace> = self.traces.iter().rev().take(n).collect();
        let mut out = Vec::with_capacity(recent.len());
        for t in recent.iter().rev() {
            if let Some(v) = t.dim_by_name(name).or_else(|| t.sub_by_name(name)) {
                out.push(v);
            }
        }
        out
    }

    /// 当前仓库大小。
    pub fn len(&self) -> usize {
        self.traces.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.traces.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{V05_DIMENSION_NAMES, V05_DIM_COUNT, V1136_SUBMEASURE_COUNT};

    fn make_trace(value: f64) -> DimensionTrace {
        DimensionTrace {
            trace_id: 0, // auto-assigned
            sample_id: 0,
            timestamp: 1_700_000_000,
            v05_dims: [value; V05_DIM_COUNT],
            v1136_subs: [value; V1136_SUBMEASURE_COUNT],
            hook_overrides: vec![],
        }
    }

    #[test]
    fn append_assigns_id_monotonically() {
        let mut repo = TraceRepository::new();
        let id1 = repo.append(make_trace(0.5));
        let id2 = repo.append(make_trace(0.6));
        let id3 = repo.append(make_trace(0.7));
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
        assert_eq!(repo.len(), 3);
    }

    #[test]
    fn tail_returns_last_n_in_order() {
        let mut repo = TraceRepository::new();
        for i in 0..10 {
            repo.append(make_trace(f64::from(i) / 10.0));
        }
        let last3 = repo.tail(3);
        assert_eq!(last3.len(), 3);
        assert_eq!(last3[0].v05_dims[0], 0.7);
        assert_eq!(last3[1].v05_dims[0], 0.8);
        assert_eq!(last3[2].v05_dims[0], 0.9);
    }

    #[test]
    fn tail_n_zero_returns_empty() {
        let mut repo = TraceRepository::new();
        repo.append(make_trace(0.5));
        assert!(repo.tail(0).is_empty());
    }

    #[test]
    fn tail_n_exceeds_len_returns_all() {
        let mut repo = TraceRepository::new();
        repo.append(make_trace(0.5));
        repo.append(make_trace(0.6));
        assert_eq!(repo.tail(100).len(), 2);
    }

    #[test]
    fn trend_returns_recent_values_for_dim() {
        let mut repo = TraceRepository::new();
        repo.append(make_trace(0.1));
        repo.append(make_trace(0.2));
        repo.append(make_trace(0.3));
        let trend = repo.trend("thread_continuity", 3);
        assert_eq!(trend, vec![0.1, 0.2, 0.3]);
    }

    #[test]
    fn trend_unknown_dim_returns_empty() {
        let mut repo = TraceRepository::new();
        repo.append(make_trace(0.5));
        let trend = repo.trend("not.a.real.dim", 3);
        assert!(trend.is_empty());
    }

    #[test]
    fn with_capacity_evicts_oldest() {
        let mut repo = TraceRepository::with_capacity(3);
        for i in 0..5 {
            repo.append(make_trace(f64::from(i)));
        }
        assert_eq!(repo.len(), 3);
        let tail = repo.tail(5);
        assert_eq!(tail[0].v05_dims[0], 2.0);
        assert_eq!(tail[2].v05_dims[0], 4.0);
    }

    #[test]
    fn is_empty_initially() {
        let repo = TraceRepository::new();
        assert!(repo.is_empty());
        assert_eq!(repo.len(), 0);
    }

    #[test]
    fn append_preserves_explicit_ids() {
        let mut repo = TraceRepository::new();
        let mut t = make_trace(0.5);
        t.trace_id = 100;
        t.sample_id = 200;
        let id = repo.append(t);
        assert_eq!(id, 100);
        let tail = repo.tail(1);
        assert_eq!(tail[0].sample_id, 200);
    }

    #[test]
    fn trend_works_for_all_24_dims() {
        let mut repo = TraceRepository::new();
        repo.append(make_trace(0.5));
        for name in V05_DIMENSION_NAMES.iter() {
            let trend = repo.trend(name, 1);
            assert_eq!(trend.len(), 1);
            assert!((trend[0] - 0.5).abs() < 1e-9);
        }
    }
}
