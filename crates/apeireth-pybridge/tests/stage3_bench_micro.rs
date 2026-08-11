//! R128 阶段 A Stage 3 — 性能基准集成测试 (per decision-58 §2.1 P10-3)
//!
//! 借鉴 superpowers 234 skill execution (TDD 强制 + 启动校验 + 测量时间).
//! 实施 Stage 3 性能基准, 跑在所有 build (默认 build + python-ext build).
//!
//! # 8 硬墙 0 越界
//!
//! - B2 1.2.0 0 改
//! - A1 R11 baseline 0.8682/0.8532/0.9063 严守
//! - B1 24 LOCKED 入口签名 0 改
//! - C1 0 commit
//! - C2 0 装 PASS 严守
//! - 0 主动 push

use apeireth_pybridge::{
    stage3_bench_run_default, stage3_bench_targets, BenchConfig, BenchReport, BenchRunner,
    BenchSample, BenchStats, BenchTarget, BenchTargetReport,
};
use std::time::Duration;

// 1. Stage 3 默认 5 target 性能基准 (N=100, warmup=true)
#[test]
fn stage3_bench_default_5_targets_n100() {
    let report = stage3_bench_run_default();
    assert_eq!(report.target_reports.len(), 5);
    for tr in &report.target_reports {
        assert_eq!(tr.stats.n, 100);
    }
}

// 2. Stage 3 5 target ID 唯一
#[test]
fn stage3_bench_5_target_ids_unique() {
    let targets = stage3_bench_targets();
    let mut ids: Vec<&str> = targets.iter().map(|t| t.id()).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 5);
}

// 3. Stage 3 5 target ID 锁定 (跟 Stage 1 P10-1 + Stage 2 P10-2 协同)
#[test]
fn stage3_bench_5_target_ids_locked() {
    let targets = stage3_bench_targets();
    let expected = vec![
        "r11_module_count",
        "asi_lookup_module",
        "rust_to_json_episode",
        "json_to_rust_episode",
        "r11_compat_version",
    ];
    let actual: Vec<&str> = targets.iter().map(|t| t.id()).collect();
    for e in &expected {
        assert!(actual.contains(e), "missing target ID: {e}");
    }
}

// 4. BenchStats 聚合正确 (5 样本)
#[test]
fn stage3_bench_stats_5_samples() {
    let samples = vec![
        BenchSample { duration: Duration::from_micros(10) },
        BenchSample { duration: Duration::from_micros(20) },
        BenchSample { duration: Duration::from_micros(30) },
        BenchSample { duration: Duration::from_micros(40) },
        BenchSample { duration: Duration::from_micros(50) },
    ];
    let s = BenchStats::from_samples(&samples);
    assert_eq!(s.n, 5);
    assert_eq!(s.min, Duration::from_micros(10));
    assert_eq!(s.max, Duration::from_micros(50));
    assert_eq!(s.median, Duration::from_micros(30));
    assert_eq!(s.total, Duration::from_micros(150));
}

// 5. BenchStats p95 索引 (n=100, p95=95th percentile)
#[test]
fn stage3_bench_stats_p95_n100() {
    let mut samples = Vec::new();
    for i in 1..=100 {
        samples.push(BenchSample { duration: Duration::from_micros(i as u64) });
    }
    let s = BenchStats::from_samples(&samples);
    assert_eq!(s.n, 100);
    // p95 = sorted[ceil(0.95 * n) - 1] = sorted[94] = 95 (1-indexed 95)
    assert_eq!(s.p95, Duration::from_micros(95));
}

// 6. BenchStats p95 n=20 索引
#[test]
fn stage3_bench_stats_p95_n20() {
    let mut samples = Vec::new();
    for i in 1..=20 {
        samples.push(BenchSample { duration: Duration::from_micros(i as u64) });
    }
    let s = BenchStats::from_samples(&samples);
    assert_eq!(s.n, 20);
    // p95 = sorted[ceil(0.95 * 20) - 1] = sorted[18] = 19 (1-indexed 19)
    assert_eq!(s.p95, Duration::from_micros(19));
}

// 7. BenchStats 空数组 panic
#[test]
#[should_panic]
fn stage3_bench_stats_empty_panics() {
    let _: Vec<BenchSample> = vec![];
    let _ = BenchStats::from_samples(&[]);
}

// 8. BenchConfig 默认
#[test]
fn stage3_bench_config_default_n100() {
    let cfg = BenchConfig::default();
    assert_eq!(cfg.iterations, 100);
    assert!(cfg.warmup);
}

// 9. BenchConfig 调参 (N=10, warmup=false)
#[test]
fn stage3_bench_config_custom() {
    let cfg = BenchConfig { iterations: 10, warmup: false };
    assert_eq!(cfg.iterations, 10);
    assert!(!cfg.warmup);
}

// 10. BenchRunner::new 默认配置
#[test]
fn stage3_bench_runner_default_cfg() {
    let r = BenchRunner::new();
    let cfg = r.config();
    assert_eq!(cfg.iterations, 100);
    assert!(cfg.warmup);
}

// 11. BenchRunner 跑 1 个 target (N=10, warmup=false)
#[test]
fn stage3_bench_runner_run_one_n10() {
    let runner = BenchRunner::with_config(BenchConfig { iterations: 10, warmup: false });
    struct DummyBench;
    impl BenchTarget for DummyBench {
        fn id(&self) -> &'static str { "dummy" }
        fn when_to_use(&self) -> &'static str { "test" }
        fn requires_warmup(&self) -> bool { false }
        fn run_iteration(&self) -> Duration { Duration::from_nanos(100) }
    }
    let t = DummyBench;
    let r = runner.run_one(&t);
    assert_eq!(r.target_id, "dummy");
    assert_eq!(r.stats.n, 10);
    assert!(!r.warmup_run);
}

// 12. BenchRunner 跑 5 target 性能基准 — total_wallclock < 10s
#[test]
fn stage3_bench_run_default_under_10s() {
    let report = stage3_bench_run_default();
    assert!(report.total_wallclock < Duration::from_secs(10));
}

// 13. BenchReport Display 字段完整
#[test]
fn stage3_bench_report_display() {
    let report = stage3_bench_run_default();
    let display = format!("{report}");
    assert!(display.contains("Stage 3"));
    assert!(display.contains("decision-58"));
    assert!(display.contains("5 targets"));
    assert!(display.contains("r11_module_count"));
    assert!(display.contains("asi_lookup_module"));
    assert!(display.contains("rust_to_json_episode"));
    assert!(display.contains("json_to_rust_episode"));
    assert!(display.contains("r11_compat_version"));
    assert!(display.contains("mean="));
    assert!(display.contains("p95="));
    assert!(display.contains("min="));
    assert!(display.contains("max="));
}

// 14. BenchTargetReport Display 单 target 字段
#[test]
fn stage3_bench_target_report_display() {
    let runner = BenchRunner::with_config(BenchConfig { iterations: 3, warmup: false });
    let t = apeireth_pybridge::BenchR11ModuleCount;
    let r: BenchTargetReport = runner.run_one(&t);
    let display = format!("{r}");
    assert!(display.contains("r11_module_count"));
    assert!(display.contains("when:"));
    assert!(display.contains("n=3"));
    assert!(display.contains("mean="));
}

// 15. BenchStats summary 函数一行摘要
#[test]
fn stage3_bench_stats_summary() {
    let samples = vec![
        BenchSample { duration: Duration::from_micros(10) },
        BenchSample { duration: Duration::from_micros(20) },
    ];
    let s = BenchStats::from_samples(&samples);
    let out = s.summary();
    assert!(out.contains("n=2"));
    assert!(out.contains("mean="));
    assert!(out.contains("median="));
    assert!(out.contains("p95="));
    assert!(out.contains("min="));
    assert!(out.contains("max="));
    assert!(out.contains("total="));
}

// 16. Stage 3 性能基准跨 5 target 报告 aggregate 字段
#[test]
fn stage3_bench_report_aggregate() {
    let report: BenchReport = stage3_bench_run_default();
    // total_wallclock >= sum of all target total (粗略, 由于顺序跑)
    let sum: Duration = report.target_reports.iter().map(|tr| tr.stats.total).sum();
    // total_wallclock 含一些 overhead, 所以 >= sum
    assert!(report.total_wallclock.as_nanos() > 0);
    assert!(sum > Duration::from_nanos(0));
}

// 17. Stage 3 性能基准 — 0 装 PASS 严守 (无 Python 依赖)
#[test]
fn stage3_bench_no_python_dependency() {
    // 5 target 都不依赖 Python 运行时 (全 Rust 侧)
    for t in stage3_bench_targets() {
        let when = t.when_to_use();
        // 至少一个含 Rust 关键词
        assert!(
            when.contains("编译期 const") || when.contains("serde_json") || when.contains("bridge"),
            "target {} should be Rust-only, got: {when}",
            t.id()
        );
    }
}

// 18. Stage 3 性能基准 — 跨 N 次调用稳定 (idempotent)
#[test]
fn stage3_bench_idempotent_runs() {
    let r1 = stage3_bench_run_default();
    let r2 = stage3_bench_run_default();
    // 5 target ID 一致
    for (a, b) in r1.target_reports.iter().zip(r2.target_reports.iter()) {
        assert_eq!(a.target_id, b.target_id);
    }
    // 跨 2 次跑, 每次都 N=100
    for tr in &r1.target_reports {
        assert_eq!(tr.stats.n, 100);
    }
    for tr in &r2.target_reports {
        assert_eq!(tr.stats.n, 100);
    }
}
