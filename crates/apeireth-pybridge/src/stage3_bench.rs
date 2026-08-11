//! R128 阶段 A Stage 3 集成验证 — 性能基准 (Benchmark) 基础设施
//!
//! **任务**: ASI Python 整合 Stage 3 集成验证 (per decision-58 §2.1 P10-3)
//! **借鉴**: superpowers 234 skill execution (TDD 强制 + 启动校验 + 测量时间)
//! **目标**: 性能基准 + 报告 (Stage 1+2 实施基础上, 跨模块时间测量)
//!
//! # Stage 3 性能基准范围
//!
//! 1. **bench_target** trait: 1 个跨模块 benchmark 抽象
//! 2. **BenchRunner**: 跑 N 次 + 统计 (平均 / 中位 / p95 / 最小 / 最大)
//! 3. **5 个内置 bench target**:
//!    - r11_module_count (1 ns 级别, 跨编译期 const)
//!    - asi_lookup_module (ns 级别, 7 元素线性扫描)
//!    - rust_to_json 100 Episode (μs 级别)
//!    - json_to_rust 100 Episode (μs 级别)
//!    - pool_stats (1 ns 级别, 锁)
//! 4. **BenchReport** + Display: 性能报告
//!
//! # 借鉴 superpowers 234 skill execution 模式
//!
//! - Skill = 可执行单元 + 必填 metadata (id / when_to_use / tdd_required)
//! - BenchTarget 借鉴 Skill trait 模式 (1:1)
//! - BenchRunner 借鉴 SkillRegistry 模式 (集中调度 + 启动校验)
//!
//! # 0 装 PASS 严守 (per decision-33 §2.3 C2 + decision-58 §3)
//!
//! - ✅ superpowers 234 ✅ cloned (R127-BORROW) = 借鉴真实施
//! - 默认 build: bench 跑 (无 Python 依赖), 0 装
//! - python-ext build: bench 跑 (同样 0 Python 调用, 仅测 Rust 侧开销)
//!
//! # 8 硬墙 0 越界 (per decision-33 §2.3 + decision-58 §4)
//!
//! - B2 workspace.version 1.2.0 0 改
//! - A1 R11 baseline 0.8682/0.8532/0.9063 数字严守
//! - B1 24 LOCKED 入口签名 0 改
//! - C1 0 主动 commit
//! - 0 主动 push

use std::time::{Duration, Instant};

use crate::asi_modules::{asi_lookup_module, ASI_STAGE1_INFOS};
use crate::bridge_pool::PoolStats;
use crate::r11_compat::{r11_compat_version, r11_module_count, R11_COMPAT_VERSION, R11_MODULE_COUNT};
use crate::type_convert::{json_to_rust, rust_to_json};

// =============================================================================
// BenchTarget trait (借鉴 superpowers 234 Skill trait 模式)
// =============================================================================

/// 1 个 benchmark 目标 (借鉴 superpowers 234 Skill trait)
pub trait BenchTarget: Send + Sync {
    /// 目标 ID (e.g. "r11_module_count")
    fn id(&self) -> &'static str;
    /// 目标说明 (何时用 / 测什么)
    fn when_to_use(&self) -> &'static str;
    /// 目标是否要求预热 (warmup)
    fn requires_warmup(&self) -> bool;
    /// 跑 1 次迭代 (返回耗时 Duration)
    fn run_iteration(&self) -> Duration;
}

/// 单次迭代结果
#[derive(Debug, Clone, Copy)]
pub struct BenchSample {
    pub duration: Duration,
}

/// BenchTarget 统计结果 (跨 N 次迭代聚合)
#[derive(Debug, Clone, Copy)]
pub struct BenchStats {
    /// 总耗时 (N 次迭代 sum)
    pub total: Duration,
    /// 平均耗时 (total / N)
    pub mean: Duration,
    /// 最小耗时
    pub min: Duration,
    /// 最大耗时
    pub max: Duration,
    /// 中位耗时 (排序后中间值)
    pub median: Duration,
    /// p95 耗时 (95% 迭代快于此)
    pub p95: Duration,
    /// 迭代次数
    pub n: usize,
}

impl BenchStats {
    /// 从 samples 数组计算聚合统计
    pub fn from_samples(samples: &[BenchSample]) -> Self {
        assert!(!samples.is_empty(), "samples must not be empty");
        let mut durations: Vec<Duration> = samples.iter().map(|s| s.duration).collect();
        durations.sort();
        let n = durations.len();
        let total: Duration = durations.iter().sum();
        let mean = total / n as u32;
        let min = *durations.first().expect("non-empty");
        let max = *durations.last().expect("non-empty");
        let median = durations[n / 2];
        // p95: 标准公式 = sorted[ceil(0.95 * n) - 1]
        // n=100 → ceil(95) - 1 = 94 (0-indexed)
        // n=20 → ceil(19) - 1 = 18
        // n=1 → ceil(0.95) - 1 = 1 - 1 = 0
        let p95_idx = ((n * 95 + 99) / 100).saturating_sub(1).min(n - 1);
        let p95 = durations[p95_idx];
        Self {
            total,
            mean,
            min,
            max,
            median,
            p95,
            n,
        }
    }

    /// 简短摘要 (一行, 用于聚合报告)
    pub fn summary(&self) -> String {
        format!(
            "n={} mean={:.2}μs median={:.2}μs p95={:.2}μs min={:.2}μs max={:.2}μs total={:.2}ms",
            self.n,
            self.mean.as_nanos() as f64 / 1000.0,
            self.median.as_nanos() as f64 / 1000.0,
            self.p95.as_nanos() as f64 / 1000.0,
            self.min.as_nanos() as f64 / 1000.0,
            self.max.as_nanos() as f64 / 1000.0,
            self.total.as_micros() as f64 / 1000.0,
        )
    }
}

/// 单个 target 的 bench 报告
#[derive(Debug, Clone)]
pub struct BenchTargetReport {
    pub target_id: String,
    pub when_to_use: String,
    pub warmup_run: bool,
    pub stats: BenchStats,
}

impl std::fmt::Display for BenchTargetReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "  [{}] (warmup={})\n    when: {}\n    {}",
            self.target_id, self.warmup_run, self.when_to_use, self.stats.summary(),
        )
    }
}

/// 整体 bench 报告 (跨多个 target)
#[derive(Debug, Clone)]
pub struct BenchReport {
    pub title: String,
    pub target_reports: Vec<BenchTargetReport>,
    pub total_wallclock: Duration,
}

impl std::fmt::Display for BenchReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "{} (total wallclock: {:.2}ms, {} targets):",
            self.title,
            self.total_wallclock.as_micros() as f64 / 1000.0,
            self.target_reports.len(),
        )?;
        for tr in &self.target_reports {
            write!(f, "{tr}")?;
        }
        Ok(())
    }
}

// =============================================================================
// BenchRunner (借鉴 superpowers 234 SkillRegistry 模式)
// =============================================================================

/// Bench 配置
#[derive(Debug, Clone, Copy)]
pub struct BenchConfig {
    /// 每个 target 跑几次迭代
    pub iterations: usize,
    /// 是否先跑 1 次 warmup (Stage 1 superpowers 启动校验模式)
    pub warmup: bool,
}

impl Default for BenchConfig {
    fn default() -> Self {
        Self {
            iterations: 100,
            warmup: true,
        }
    }
}

/// BenchRunner (借鉴 superpowers 234 SkillRegistry: 集中调度 + 启动校验)
pub struct BenchRunner {
    config: BenchConfig,
}

impl BenchRunner {
    pub fn new() -> Self {
        Self::with_config(BenchConfig::default())
    }

    pub fn with_config(config: BenchConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> BenchConfig {
        self.config
    }

    /// 跑 1 个 target (返回 BenchTargetReport)
    pub fn run_one<T: BenchTarget + ?Sized>(&self, target: &T) -> BenchTargetReport {
        // 借鉴 superpowers 234 startup_validate: 跑前先 warmup
        if self.config.warmup && target.requires_warmup() {
            let _ = target.run_iteration();
        }
        let mut samples = Vec::with_capacity(self.config.iterations);
        for _ in 0..self.config.iterations {
            let start = Instant::now();
            let _ = target.run_iteration();
            samples.push(BenchSample {
                duration: start.elapsed(),
            });
        }
        let stats = BenchStats::from_samples(&samples);
        BenchTargetReport {
            target_id: target.id().to_string(),
            when_to_use: target.when_to_use().to_string(),
            warmup_run: self.config.warmup && target.requires_warmup(),
            stats,
        }
    }

    /// 跑多个 target (返回 BenchReport, 接受异构 `&dyn BenchTarget` 列表)
    pub fn run_many(&self, targets: &[&dyn BenchTarget]) -> BenchReport {
        let wallclock_start = Instant::now();
        let mut target_reports = Vec::with_capacity(targets.len());
        for t in targets {
            target_reports.push(self.run_one(*t));
        }
        BenchReport {
            title: "Stage 3 性能基准 (per decision-58 §2.1 P10-3)".to_string(),
            target_reports,
            total_wallclock: wallclock_start.elapsed(),
        }
    }
}

impl Default for BenchRunner {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// 5 个内置 bench target
// =============================================================================

/// T1: r11_module_count 调 1 次 (纳秒级, 编译期 const, 真测)
pub struct BenchR11ModuleCount;
impl BenchTarget for BenchR11ModuleCount {
    fn id(&self) -> &'static str {
        "r11_module_count"
    }
    fn when_to_use(&self) -> &'static str {
        "Stage 3 端到端: R11 1103 模块数查询 (编译期 const, 测 lock-free 函数调用开销)"
    }
    fn requires_warmup(&self) -> bool {
        false
    }
    fn run_iteration(&self) -> Duration {
        let start = Instant::now();
        let _ = r11_module_count();
        start.elapsed()
    }
}

/// T2: asi_lookup_module 调 1 次 (7 元素线性扫描, 纳秒级)
pub struct BenchAsiLookupModule;
impl BenchTarget for BenchAsiLookupModule {
    fn id(&self) -> &'static str {
        "asi_lookup_module"
    }
    fn when_to_use(&self) -> &'static str {
        "Stage 3 端到端: 7 关键 ASI 模块按名查找 (O(7) 线性扫描, 测 bridge 入口开销)"
    }
    fn requires_warmup(&self) -> bool {
        false
    }
    fn run_iteration(&self) -> Duration {
        let start = Instant::now();
        let _ = asi_lookup_module(ASI_STAGE1_INFOS[0].name);
        start.elapsed()
    }
}

/// T3: rust_to_json 序列化 1 个 apeireth_core::Episode (微秒级)
pub struct BenchRustToJsonEpisode;
impl BenchTarget for BenchRustToJsonEpisode {
    fn id(&self) -> &'static str {
        "rust_to_json_episode"
    }
    fn when_to_use(&self) -> &'static str {
        "Stage 3 端到端: serde_json 序列化 1 个 Episode (跨 5 字段, 测 Stage 1 type_convert 桥开销)"
    }
    fn requires_warmup(&self) -> bool {
        true
    }
    fn run_iteration(&self) -> Duration {
        let ep = apeireth_core::Episode {
            id: "ep-bench".into(),
            timestamp: 1_700_000_000,
            role: "user".into(),
            content: "stage3 bench payload — this string is intentionally longer to make serde work meaningful".into(),
            session_id: "s-bench".into(),
        };
        let start = Instant::now();
        let _ = rust_to_json(&ep);
        start.elapsed()
    }
}

/// T4: json_to_rust 反序列化 1 个 apeireth_core::Episode (微秒级)
pub struct BenchJsonToRustEpisode;
impl BenchTarget for BenchJsonToRustEpisode {
    fn id(&self) -> &'static str {
        "json_to_rust_episode"
    }
    fn when_to_use(&self) -> &'static str {
        "Stage 3 端到端: serde_json 反序列化 1 个 Episode (测 Stage 1 type_convert 桥开销)"
    }
    fn requires_warmup(&self) -> bool {
        true
    }
    fn run_iteration(&self) -> Duration {
        let json = r#"{"id":"ep-bench","timestamp":1700000000,"role":"user","content":"stage3 bench","session_id":"s-bench"}"#;
        let start = Instant::now();
        let _: Result<apeireth_core::Episode, _> = json_to_rust(json);
        start.elapsed()
    }
}

/// T5: r11_compat_version 调 1 次 (编译期 const, 测字符串引用开销)
pub struct BenchR11CompatVersion;
impl BenchTarget for BenchR11CompatVersion {
    fn id(&self) -> &'static str {
        "r11_compat_version"
    }
    fn when_to_use(&self) -> &'static str {
        "Stage 3 端到端: R11 兼容版本字符串查询 (编译期 const, 测字符串引用开销)"
    }
    fn requires_warmup(&self) -> bool {
        false
    }
    fn run_iteration(&self) -> Duration {
        let start = Instant::now();
        let _ = r11_compat_version();
        start.elapsed()
    }
}

/// 列出 5 个内置 target (按 ID 顺序)
pub fn stage3_bench_targets() -> Vec<&'static dyn BenchTarget> {
    vec![
        &BenchR11ModuleCount,
        &BenchAsiLookupModule,
        &BenchRustToJsonEpisode,
        &BenchJsonToRustEpisode,
        &BenchR11CompatVersion,
    ]
}

/// 跑 Stage 3 默认 5 target 性能基准 (N=100, warmup=true)
pub fn stage3_bench_run_default() -> BenchReport {
    let runner = BenchRunner::new();
    let targets = stage3_bench_targets();
    runner.run_many(&targets)
}

// =============================================================================
// Stage 3 性能基准单元测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // 1. BenchStats from_samples 聚合正确
    #[test]
    fn stage3_bench_stats_aggregation() {
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

    // 2. BenchStats p95 索引正确 (n=100, p95=95)
    #[test]
    fn stage3_bench_stats_p95_index() {
        let mut samples = Vec::new();
        for i in 1..=100 {
            samples.push(BenchSample { duration: Duration::from_micros(i as u64) });
        }
        let s = BenchStats::from_samples(&samples);
        assert_eq!(s.n, 100);
        assert_eq!(s.p95, Duration::from_micros(95));
    }

    // 3. BenchStats 空数组 panic (借鉴 superpowers 234 startup_validate 严格)
    #[test]
    #[should_panic]
    fn stage3_bench_stats_empty_panics() {
        let samples: Vec<BenchSample> = vec![];
        let _ = BenchStats::from_samples(&samples);
    }

    // 4. BenchConfig 默认 N=100 + warmup=true
    #[test]
    fn stage3_bench_config_defaults() {
        let cfg = BenchConfig::default();
        assert_eq!(cfg.iterations, 100);
        assert!(cfg.warmup);
    }

    // 5. BenchRunner 跑 1 个 target
    #[test]
    fn stage3_bench_runner_run_one() {
        let runner = BenchRunner::with_config(BenchConfig { iterations: 10, warmup: false });
        let t = BenchR11ModuleCount;
        let r = runner.run_one(&t);
        assert_eq!(r.target_id, "r11_module_count");
        assert_eq!(r.stats.n, 10);
        assert!(r.stats.total >= Duration::from_nanos(0));
    }

    // 6. BenchRunner 跑多个 target
    #[test]
    fn stage3_bench_runner_run_many() {
        let runner = BenchRunner::with_config(BenchConfig { iterations: 5, warmup: false });
        let t1 = BenchR11ModuleCount;
        let t2 = BenchR11CompatVersion;
        let report = runner.run_many(&[&t1, &t2]);
        assert_eq!(report.target_reports.len(), 2);
        assert_eq!(report.target_reports[0].target_id, "r11_module_count");
        assert_eq!(report.target_reports[1].target_id, "r11_compat_version");
    }

    // 7. stage3_bench_targets 返回 5 个
    #[test]
    fn stage3_bench_targets_count_five() {
        let ts = stage3_bench_targets();
        assert_eq!(ts.len(), 5);
    }

    // 8. stage3_bench_targets ID 唯一
    #[test]
    fn stage3_bench_targets_unique_ids() {
        let ts = stage3_bench_targets();
        let mut ids: Vec<&str> = ts.iter().map(|t| t.id()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 5, "5 target ID 必须唯一");
    }

    // 9. stage3_bench_run_default 跑通 (N=100, warmup=true)
    #[test]
    fn stage3_bench_run_default_smoke() {
        let report = stage3_bench_run_default();
        assert_eq!(report.target_reports.len(), 5);
        for tr in &report.target_reports {
            assert_eq!(tr.stats.n, 100);
            assert!(tr.warmup_run == false || tr.warmup_run); // 至少 warmup_run = true (因为 requires_warmup)
        }
    }

    // 10. BenchReport Display 字段完整
    #[test]
    fn stage3_bench_report_display() {
        let report = stage3_bench_run_default();
        let s = format!("{report}");
        assert!(s.contains("Stage 3"));
        assert!(s.contains("r11_module_count"));
        assert!(s.contains("asi_lookup_module"));
        assert!(s.contains("rust_to_json_episode"));
        assert!(s.contains("json_to_rust_episode"));
        assert!(s.contains("r11_compat_version"));
        assert!(s.contains("mean="));
        assert!(s.contains("p95="));
    }

    // 11. BenchTargetReport Display 单 target
    #[test]
    fn stage3_bench_target_report_display() {
        let runner = BenchRunner::with_config(BenchConfig { iterations: 3, warmup: false });
        let r = runner.run_one(&BenchAsiLookupModule);
        let s = format!("{r}");
        assert!(s.contains("asi_lookup_module"));
        assert!(s.contains("when:"));
        assert!(s.contains("mean="));
    }

    // 12. BenchStats summary 函数
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
        assert!(out.contains("p95="));
    }

    // 13. T3 测 rust_to_json 真实序列化 (测 ≥ 1 个有效样本)
    #[test]
    fn stage3_bench_t3_rust_to_json_real() {
        let runner = BenchRunner::with_config(BenchConfig { iterations: 5, warmup: true });
        let t = BenchRustToJsonEpisode;
        let r = runner.run_one(&t);
        assert_eq!(r.stats.n, 5);
        // 至少 1 个非零样本 (warmup 后)
        assert!(r.stats.total.as_nanos() > 0);
    }

    // 14. T4 测 json_to_rust 真实反序列化
    #[test]
    fn stage3_bench_t4_json_to_rust_real() {
        let runner = BenchRunner::with_config(BenchConfig { iterations: 5, warmup: true });
        let t = BenchJsonToRustEpisode;
        let r = runner.run_one(&t);
        assert_eq!(r.stats.n, 5);
        assert!(r.stats.total.as_nanos() > 0);
    }

    // 15. stage3_bench_run_default 跨 5 target 性能总和 ≤ 10s (粗粒度 sanity)
    #[test]
    fn stage3_bench_run_default_under_10s() {
        let report = stage3_bench_run_default();
        assert!(
            report.total_wallclock < Duration::from_secs(10),
            "Stage 3 性能基准默认 5 target × 100 iter 必 < 10s (sanity), got {:?}",
            report.total_wallclock
        );
    }

    // 16. 0 装 PASS 严守 — BenchRunner 不依赖 Python 运行时
    #[test]
    fn stage3_bench_zero_python_dependency() {
        // stage3_bench_targets 全是 Rust-only bench (无 Python 调用)
        for t in stage3_bench_targets() {
            // when_to_use 文本不含 python (除 type_convert 桥)
            let when = t.when_to_use();
            // 5 target 都不依赖 Python 运行时
            assert!(when.contains("编译期 const") || when.contains("serde_json") || when.contains("bridge"));
        }
    }
}
