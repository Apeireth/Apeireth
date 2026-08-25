//! ASCII 渲染: 24 维详细表 + sparkline + diagnose (v1 等价)
use crate::{DimensionTrace, V05_DIMENSION_NAMES, V1136_SUBMEASURE_COUNT, V1136_SUBMEASURE_NAMES};

const SPARK_CHARS: [char; 8] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇'];

pub fn ascii_sparkline(values: &[f64]) -> String {
    if values.is_empty() { return String::new(); }
    let mut out = String::with_capacity(values.len());
    for &v in values {
        let clamped = v.clamp(0.0, 1.0);
        let bucket = (clamped * 7.0).round() as usize;
        out.push(SPARK_CHARS[bucket.min(7)]);
    }
    out
}

pub fn format_trace_table(trace: &DimensionTrace) -> String {
    let mut out = String::new();
    out.push_str(&format!("DimensionTrace #{} (sample {}, timestamp {})\n", trace.trace_id, trace.sample_id, trace.timestamp));
    out.push_str(&format!("{:<30} {:>8} {:>8}\n", "Dimension", "V0.5", "V1136_sub"));
    out.push_str(&"-".repeat(50));
    out.push('\n');
    for (i, dim_name) in V05_DIMENSION_NAMES.iter().enumerate() {
        let v = trace.v05_dims[i];
        let sub = if i < V1136_SUBMEASURE_COUNT { trace.v1136_subs[i] } else { f64::NAN };
        let sub_str = if sub.is_nan() { "—".to_string() } else { format!("{:.4}", sub) };
        out.push_str(&format!("{:<30} {:>8.4} {:>8}\n", dim_name, v, sub_str));
    }
    out.push_str(&"-".repeat(50));
    out.push('\n');
    out.push_str(&format!("Mean V0.5: {:.4} | Mean V1136: {:.4} | Hook overrides: {}\n",
        trace.mean_v05(), trace.mean_v1136(), trace.hook_overrides.len()));
    out
}

#[derive(Debug, Clone)]
pub struct DiagnosticReport {
    pub weakest_dims: Vec<(String, f64)>,
    pub weakest_subs: Vec<(String, f64)>,
    pub suggestions: Vec<String>,
}

pub fn diagnose_weakest(trace: &DimensionTrace, top_n: usize) -> DiagnosticReport {
    let mut dim_pairs: Vec<(String, f64)> = V05_DIMENSION_NAMES.iter().enumerate()
        .map(|(i, &n)| (n.to_string(), trace.v05_dims[i])).collect();
    dim_pairs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut sub_pairs: Vec<(String, f64)> = V1136_SUBMEASURE_NAMES.iter().enumerate()
        .map(|(i, &n)| (n.to_string(), trace.v1136_subs[i])).collect();
    sub_pairs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let weakest_dims: Vec<(String, f64)> = dim_pairs.iter().take(top_n).cloned().collect();
    let weakest_subs: Vec<(String, f64)> = sub_pairs.iter().take(top_n).cloned().collect();
    let mut suggestions = Vec::new();
    for (name, value) in &weakest_dims {
        if *value < 0.5 {
            suggestions.push(format!("[CRITICAL] dim `{name}` = {:.4} < 0.5: 触发深度审查 + 增加 philosophy_guard pass rate", value));
        } else if *value < 0.7 {
            suggestions.push(format!("[WARN] dim `{name}` = {:.4} < 0.7: 改进观察采样 + 增 quality_factor", value));
        } else {
            suggestions.push(format!("[INFO] dim `{name}` = {:.4} (top weakest, but > 0.7): 维持现状", value));
        }
    }
    DiagnosticReport { weakest_dims, weakest_subs, suggestions }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{V05_DIM_COUNT, V1136_SUBMEASURE_COUNT};
    fn make_test_trace() -> DimensionTrace {
        let mut dims = [0.5; V05_DIM_COUNT]; dims[0] = 0.3; dims[1] = 0.2; dims[2] = 0.4; dims[5] = 0.6; dims[6] = 0.8;
        let mut subs = [0.5; V1136_SUBMEASURE_COUNT]; subs[0] = 0.25; subs[3] = 0.15;
        DimensionTrace { trace_id: 42, sample_id: 7, timestamp: 1_700_000_000, v05_dims: dims, v1136_subs: subs, hook_overrides: vec![] }
    }
    #[test]
    fn sparkline_empty() { assert_eq!(ascii_sparkline(&[]), ""); }
    #[test]
    fn sparkline_monotonic() {
        let line = ascii_sparkline(&[0.0, 0.2, 0.5, 0.8, 1.0]);
        assert_eq!(line.chars().count(), 5);
        assert_eq!(line.chars().next().unwrap(), ' ');
        assert_eq!(line.chars().last().unwrap(), '▇');
    }
    #[test]
    fn sparkline_clamps() {
        let line = ascii_sparkline(&[-0.5, 1.5]);
        assert_eq!(line.chars().count(), 2);
        assert_eq!(line.chars().next().unwrap(), ' ');
        assert_eq!(line.chars().last().unwrap(), '▇');
    }
    #[test]
    fn sparkline_length_matches() {
        let v = vec![0.1, 0.3, 0.5, 0.7, 0.9, 0.2];
        assert_eq!(ascii_sparkline(&v).chars().count(), v.len());
    }
    #[test]
    fn table_contains_all_24() {
        let trace = make_test_trace();
        let table = format_trace_table(&trace);
        for name in V05_DIMENSION_NAMES.iter() { assert!(table.contains(name)); }
    }
    #[test]
    fn table_shows_trace_id() {
        let table = format_trace_table(&make_test_trace());
        assert!(table.contains("#42"));
        assert!(table.contains("sample 7"));
    }
    #[test]
    fn diagnose_3_dims() {
        let report = diagnose_weakest(&make_test_trace(), 3);
        assert_eq!(report.weakest_dims.len(), 3);
        assert_eq!(report.weakest_dims[0].0, "fact_recall");
        assert!((report.weakest_dims[0].1 - 0.2).abs() < 1e-9);
    }
    #[test]
    fn diagnose_3_subs() {
        let report = diagnose_weakest(&make_test_trace(), 3);
        assert_eq!(report.weakest_subs.len(), 3);
        assert_eq!(report.weakest_subs[0].0, "session_recovery_score");
    }
    #[test]
    fn diagnose_suggestions_levels() {
        let report = diagnose_weakest(&make_test_trace(), 3);
        let crit = report.suggestions.iter().filter(|s| s.contains("[CRITICAL]")).count();
        assert!(crit >= 1);
    }
}
