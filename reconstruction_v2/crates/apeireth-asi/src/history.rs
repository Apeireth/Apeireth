//! TraceRepository — in-memory append-only (v1 等价)
use std::collections::VecDeque;
use crate::DimensionTrace;

#[derive(Debug, Clone)]
pub struct TraceRepository {
    traces: VecDeque<DimensionTrace>,
    max_traces: usize,
    next_trace_id: u64,
    next_sample_id: u64,
}
impl Default for TraceRepository {
    fn default() -> Self { Self { traces: VecDeque::new(), max_traces: 10_000, next_trace_id: 1, next_sample_id: 1 } }
}
impl TraceRepository {
    pub fn new() -> Self { Self::default() }
    pub fn with_capacity(max_traces: usize) -> Self {
        Self { traces: VecDeque::with_capacity(max_traces.min(1024)), max_traces, next_trace_id: 1, next_sample_id: 1 }
    }
    pub fn append(&mut self, mut trace: DimensionTrace) -> u64 {
        if trace.trace_id == 0 { trace.trace_id = self.next_trace_id; self.next_trace_id += 1; }
        if trace.sample_id == 0 { trace.sample_id = self.next_sample_id; self.next_sample_id += 1; }
        let id = trace.trace_id;
        if self.traces.len() >= self.max_traces { self.traces.pop_front(); }
        self.traces.push_back(trace);
        id
    }
    pub fn tail(&self, n: usize) -> Vec<DimensionTrace> {
        let start = self.traces.len().saturating_sub(n);
        self.traces.iter().skip(start).cloned().collect()
    }
    pub fn trend(&self, name: &str, n: usize) -> Vec<f64> {
        let recent: Vec<&DimensionTrace> = self.traces.iter().rev().take(n).collect();
        let mut out = Vec::with_capacity(recent.len());
        for t in recent.iter().rev() {
            if let Some(v) = t.dim_by_name(name).or_else(|| t.sub_by_name(name)) { out.push(v); }
        }
        out
    }
    pub fn len(&self) -> usize { self.traces.len() }
    pub fn is_empty(&self) -> bool { self.traces.is_empty() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{V05_DIM_COUNT, V1136_SUBMEASURE_COUNT, V05_DIMENSION_NAMES};
    fn make_trace(value: f64) -> DimensionTrace {
        DimensionTrace { trace_id: 0, sample_id: 0, timestamp: 1_700_000_000,
            v05_dims: [value; V05_DIM_COUNT], v1136_subs: [value; V1136_SUBMEASURE_COUNT], hook_overrides: vec![] }
    }
    #[test]
    fn append_assigns_id_monotonic() {
        let mut r = TraceRepository::new();
        assert_eq!(r.append(make_trace(0.5)), 1);
        assert_eq!(r.append(make_trace(0.6)), 2);
        assert_eq!(r.append(make_trace(0.7)), 3);
        assert_eq!(r.len(), 3);
    }
    #[test]
    fn tail_ordered() {
        let mut r = TraceRepository::new();
        for i in 0..10 { r.append(make_trace(f64::from(i) / 10.0)); }
        let last3 = r.tail(3);
        assert_eq!(last3.len(), 3);
        assert_eq!(last3[0].v05_dims[0], 0.7);
        assert_eq!(last3[2].v05_dims[0], 0.9);
    }
    #[test]
    fn tail_zero() {
        let mut r = TraceRepository::new();
        r.append(make_trace(0.5));
        assert!(r.tail(0).is_empty());
    }
    #[test]
    fn tail_exceeds_len() {
        let mut r = TraceRepository::new();
        r.append(make_trace(0.5)); r.append(make_trace(0.6));
        assert_eq!(r.tail(100).len(), 2);
    }
    #[test]
    fn trend_recent() {
        let mut r = TraceRepository::new();
        r.append(make_trace(0.1)); r.append(make_trace(0.2)); r.append(make_trace(0.3));
        assert_eq!(r.trend("thread_continuity", 3), vec![0.1, 0.2, 0.3]);
    }
    #[test]
    fn trend_unknown_empty() {
        let mut r = TraceRepository::new();
        r.append(make_trace(0.5));
        assert!(r.trend("not.a.real.dim", 3).is_empty());
    }
    #[test]
    fn with_capacity_evicts() {
        let mut r = TraceRepository::with_capacity(3);
        for i in 0..5 { r.append(make_trace(f64::from(i))); }
        assert_eq!(r.len(), 3);
        let tail = r.tail(5);
        assert_eq!(tail[0].v05_dims[0], 2.0);
        assert_eq!(tail[2].v05_dims[0], 4.0);
    }
    #[test]
    fn is_empty_init() {
        let r = TraceRepository::new();
        assert!(r.is_empty());
    }
    #[test]
    fn explicit_ids_preserved() {
        let mut r = TraceRepository::new();
        let mut t = make_trace(0.5); t.trace_id = 100; t.sample_id = 200;
        let id = r.append(t); assert_eq!(id, 100);
        assert_eq!(r.tail(1)[0].sample_id, 200);
    }
    #[test]
    fn trend_all_24_dims() {
        let mut r = TraceRepository::new();
        r.append(make_trace(0.5));
        for name in V05_DIMENSION_NAMES.iter() {
            assert_eq!(r.trend(name, 1).len(), 1);
        }
    }
}
