//! ASCII 渲染: 24 维详细表 + sparkline + diagnose (round10-12 qa_engineer)
//!
//! 设计原则:
//! - 纯 ASCII 输出 (兼容所有终端), 必要时用 unicode 图块
//! - 不绑死三值 (效果优先于数字): 用 0.0-1.0 真实值
//! - 输出函数全部 pure: 输入 trace, 输出 String

use crate::{DimensionTrace, V05_DIMENSION_NAMES, V1136_SUBMEASURE_COUNT, V1136_SUBMEASURE_NAMES};

/// 7 字符 sparkline 梯度 (从低到高): ▁▂▃▄▅▆▇█
const SPARK_CHARS: [char; 8] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇'];

/// ASCII 7 字符 sparkline (Ponytail: 7 bucket)。
pub fn ascii_sparkline(values: &[f64]) -> String {
    if values.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(values.len());
    for &v in values {
        let clamped = v.clamp(0.0, 1.0);
        let bucket = (clamped * 7.0).round() as usize;
        out.push(SPARK_CHARS[bucket.min(7)]);
    }
    out
}

/// 格式化 24 维详细表 (Ponytail: 单次循环, 紧凑列)。
pub fn format_trace_table(trace: &DimensionTrace) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "DimensionTrace #{} (sample {}, timestamp {})\n",
        trace.trace_id, trace.sample_id, trace.timestamp
    ));
    out.push_str(&format!(
        "{:<30} {:>8} {:>8}\n",
        "Dimension", "V0.5", "V1136_sub"
    ));
    out.push_str(&"-".repeat(50));
    out.push('\n');

    // 24 维 + 9 子测度按 name 对齐
    for (i, dim_name) in V05_DIMENSION_NAMES.iter().enumerate() {
        let v = trace.v05_dims[i];
        let sub = if i < V1136_SUBMEASURE_COUNT {
            trace.v1136_subs[i]
        } else {
            f64::NAN
        };
        let sub_str = if sub.is_nan() {
            "—".to_string()
        } else {
            format!("{:.4}", sub)
        };
        out.push_str(&format!("{:<30} {:>8.4} {:>8}\n", dim_name, v, sub_str));
    }

    // 剩余子测度 (i >= 24, 但 V1136 只有 9 个, 所以 i < 9, 全部对齐到 v05_dims 前 9 个)
    // 已合并在上面

    out.push_str(&"-".repeat(50));
    out.push('\n');
    out.push_str(&format!(
        "Mean V0.5: {:.4} | Mean V1136: {:.4} | Hook overrides: {}\n",
        trace.mean_v05(),
        trace.mean_v1136(),
        trace.hook_overrides.len()
    ));
    out
}

/// 健康诊断报告。
#[derive(Debug, Clone)]
pub struct DiagnosticReport {
    /// 最弱的 N 个维度 (按 v05_dims 升序)。
    pub weakest_dims: Vec<(String, f64)>,
    /// 最弱的 N 个子测度 (按 v1136_subs 升序)。
    pub weakest_subs: Vec<(String, f64)>,
    /// 改进建议 (基于最弱维度模板)。
    pub suggestions: Vec<String>,
}

/// 自动定位最弱维度并给出改进建议 (Ponytail: 模板化建议, 不假装智能)。
pub fn diagnose_weakest(trace: &DimensionTrace, top_n: usize) -> DiagnosticReport {
    let mut dim_pairs: Vec<(String, f64)> = V05_DIMENSION_NAMES
        .iter()
        .enumerate()
        .map(|(i, &n)| (n.to_string(), trace.v05_dims[i]))
        .collect();
    dim_pairs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut sub_pairs: Vec<(String, f64)> = V1136_SUBMEASURE_NAMES
        .iter()
        .enumerate()
        .map(|(i, &n)| (n.to_string(), trace.v1136_subs[i]))
        .collect();
    sub_pairs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let weakest_dims: Vec<(String, f64)> = dim_pairs.iter().take(top_n).cloned().collect();
    let weakest_subs: Vec<(String, f64)> = sub_pairs.iter().take(top_n).cloned().collect();

    let mut suggestions = Vec::new();
    for (name, value) in &weakest_dims {
        if *value < 0.5 {
            suggestions.push(format!(
                "[CRITICAL] dim `{name}` = {:.4} < 0.5: 触发深度审查 + 增加 philosophy_guard pass rate",
                value
            ));
        } else if *value < 0.7 {
            suggestions.push(format!(
                "[WARN] dim `{name}` = {:.4} < 0.7: 改进观察采样 + 增 quality_factor",
                value
            ));
        } else {
            suggestions.push(format!(
                "[INFO] dim `{name}` = {:.4} (top weakest, but > 0.7): 维持现状",
                value
            ));
        }
    }

    DiagnosticReport {
        weakest_dims,
        weakest_subs,
        suggestions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::V05_DIM_COUNT;

    fn make_test_trace() -> DimensionTrace {
        DimensionTrace {
            trace_id: 42,
            sample_id: 7,
            timestamp: 1_700_000_000,
            v05_dims: {
                let mut a = [0.5; V05_DIM_COUNT];
                a[0] = 0.3;
                a[1] = 0.2;
                a[2] = 0.4;
                a[5] = 0.6;
                a[6] = 0.8;
                a
            },
            v1136_subs: {
                let mut a = [0.5; V1136_SUBMEASURE_COUNT];
                a[0] = 0.25;
                a[3] = 0.15;
                a
            },
            hook_overrides: vec![],
        }
    }

    #[test]
    fn ascii_sparkline_empty() {
        assert_eq!(ascii_sparkline(&[]), "");
    }

    #[test]
    fn ascii_sparkline_monotonic_increasing() {
        let line = ascii_sparkline(&[0.0, 0.2, 0.5, 0.8, 1.0]);
        assert_eq!(line.chars().count(), 5);
        // bucket 0..7
        assert_eq!(line.chars().next().unwrap(), ' ');
        assert_eq!(line.chars().last().unwrap(), '▇');
    }

    #[test]
    fn ascii_sparkline_clamps_out_of_range() {
        let line = ascii_sparkline(&[-0.5, 1.5]);
        assert_eq!(line.chars().count(), 2);
        // clamped to 0 and 1
        assert_eq!(line.chars().next().unwrap(), ' ');
        assert_eq!(line.chars().last().unwrap(), '▇');
    }

    #[test]
    fn format_trace_table_contains_all_24_dims() {
        let trace = make_test_trace();
        let table = format_trace_table(&trace);
        for name in V05_DIMENSION_NAMES.iter() {
            assert!(table.contains(name), "missing dim {name}");
        }
    }

    #[test]
    fn format_trace_table_shows_trace_id() {
        let trace = make_test_trace();
        let table = format_trace_table(&trace);
        assert!(table.contains("#42"));
        assert!(table.contains("sample 7"));
    }

    #[test]
    fn diagnose_finds_weakest_3_dims() {
        let trace = make_test_trace();
        let report = diagnose_weakest(&trace, 3);
        assert_eq!(report.weakest_dims.len(), 3);
        // fact_recall = 0.2 should be first
        assert_eq!(report.weakest_dims[0].0, "fact_recall");
        assert!((report.weakest_dims[0].1 - 0.2).abs() < 1e-9);
        // thread_continuity = 0.3 second
        assert_eq!(report.weakest_dims[1].0, "thread_continuity");
        // context_window = 0.4 third
        assert_eq!(report.weakest_dims[2].0, "context_window");
    }

    #[test]
    fn diagnose_finds_weakest_3_subs() {
        let trace = make_test_trace();
        let report = diagnose_weakest(&trace, 3);
        assert_eq!(report.weakest_subs.len(), 3);
        assert_eq!(report.weakest_subs[0].0, "session_recovery_score");
        assert!((report.weakest_subs[0].1 - 0.15).abs() < 1e-9);
        assert_eq!(report.weakest_subs[1].0, "thread_continuity_score");
    }

    #[test]
    fn diagnose_suggestions_have_levels() {
        let trace = make_test_trace();
        let report = diagnose_weakest(&trace, 3);
        assert_eq!(report.suggestions.len(), 3);
        let critical = report
            .suggestions
            .iter()
            .filter(|s| s.contains("[CRITICAL]"))
            .count();
        assert!(
            critical >= 1,
            "expected at least 1 CRITICAL suggestion, got: {:?}",
            report.suggestions
        );
    }

    #[test]
    fn ascii_sparkline_length_matches_input() {
        let v = vec![0.1, 0.3, 0.5, 0.7, 0.9, 0.2];
        let s = ascii_sparkline(&v);
        assert_eq!(s.chars().count(), v.len());
    }
}
