//! 基准 harness 公共支撑: 确定性数据生成、批量计时、统计汇总、结果输出。
//!
//! 计时口径 (统一约定, 所有基准共用):
//! - 计时器: `std::time::Instant` (Windows 下为 QPC, 分辨率 ~100 ns 量级), 0 第三方计时依赖;
//! - 每个样本 = `batch` 次被测操作的平均耗时 (batch > 1 用于摊薄计时器开销
//!   对亚微秒级操作的干扰); 样本间操作对象可轮换 (见各基准口径说明);
//! - 汇报 min / mean / stddev / P50 / P90 / P99 / max; 百分位取 nearest-rank
//!   (升序后 idx = ceil(p/100 * n) - 1), n 较小时 P99 退化为最大值附近, 如实呈现;
//! - warmup 轮不计数; 预热与样本量可由 `BENCH_SCALE` 缩放 (`BENCH_QUICK=1` = 0.1 倍);
//! - 原始逐样本数值在 `BENCH_RAW=1` 时随结果一并输出 (RAW 行)。

use std::time::Instant;

/// 确定性 xorshift64* 伪随机源 (基准数据程序生成, 可复现, 0 第三方依赖)。
pub struct Rng(u64);

impl Rng {
    /// 以种子构造; 种子 0 会被替换为固定非零常量。
    pub fn new(seed: u64) -> Self {
        Self(if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        })
    }

    /// 下一个 64 位伪随机值。
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// [0, 1) 均匀 f32。
    pub fn next_f32(&mut self) -> f32 {
        let mantissa = (self.next_u64() >> 40) as f32; // 24 个有效位
        mantissa / 16_777_216.0
    }

    /// [lo, hi) 均匀 usize; lo >= hi 时恒返 lo。
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() % (hi - lo) as u64) as usize
    }

    /// 从切片等概率取一项 (切片非空)。
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        let idx = self.range(0, items.len());
        &items[idx]
    }
}

/// 一组逐样本观测值的汇总统计 (单位由调用方决定, 本结构不换单位)。
#[derive(Debug, Clone)]
pub struct Stats {
    /// 样本数。
    pub n: usize,
    /// 最小值。
    pub min: f64,
    /// 均值。
    pub mean: f64,
    /// 样本标准差 (n-1 分母; n = 1 时为 0)。
    pub stddev: f64,
    /// 中位数 (nearest-rank)。
    pub p50: f64,
    /// 90 分位 (nearest-rank)。
    pub p90: f64,
    /// 99 分位 (nearest-rank)。
    pub p99: f64,
    /// 最大值。
    pub max: f64,
}

impl Stats {
    /// 从逐样本值汇总; 空输入返全零统计。
    pub fn from_values(values: &[f64]) -> Self {
        if values.is_empty() {
            return Self {
                n: 0,
                min: 0.0,
                mean: 0.0,
                stddev: 0.0,
                p50: 0.0,
                p90: 0.0,
                p99: 0.0,
                max: 0.0,
            };
        }
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.total_cmp(b));
        let n = sorted.len();
        let sum: f64 = sorted.iter().sum();
        let mean = sum / n as f64;
        let variance = if n > 1 {
            sorted.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / (n - 1) as f64
        } else {
            0.0
        };
        Self {
            n,
            min: sorted[0],
            mean,
            stddev: variance.sqrt(),
            p50: percentile(&sorted, 50.0),
            p90: percentile(&sorted, 90.0),
            p99: percentile(&sorted, 99.0),
            max: sorted[n - 1],
        }
    }
}

/// nearest-rank 百分位 (升序切片)。
fn percentile(sorted: &[f64], p: f64) -> f64 {
    let n = sorted.len();
    let rank = ((p / 100.0) * n as f64).ceil() as usize;
    let idx = rank.saturating_sub(1).min(n - 1);
    sorted[idx]
}

/// 目标线 (与 README 性能目标表同值); 比较口径: P50 与目标值比。
pub struct Target {
    /// 展示用标签, 如 "< 10.0 ms"。
    pub label: String,
    /// 目标值 (与观测同单位)。
    pub value: f64,
}

/// 一个基准项的完整结果。
pub struct Outcome {
    /// 稳定 key (机器可读)。
    pub key: String,
    /// 标题。
    pub title: String,
    /// 被测操作口径。
    pub op: String,
    /// 单位 ("ms" / "us" / "MiB")。
    pub unit: String,
    /// batch (每样本包含的操作次数)。
    pub batch: usize,
    /// warmup 轮数。
    pub warmup: usize,
    /// 汇总统计。
    pub stats: Stats,
    /// 可选目标线。
    pub target: Option<Target>,
    /// 备注 (口径细节/噪声边界)。
    pub note: String,
    /// 逐样本原始值 (与 unit 同单位)。
    pub raw: Vec<f64>,
}

impl Outcome {
    /// 从逐操作耗时 (纳秒) 构造; `unit` 仅支持 "ms" / "us"。
    pub fn from_ns(
        key: &str,
        title: &str,
        op: &str,
        unit: &str,
        batch: usize,
        warmup: usize,
        samples_ns: Vec<f64>,
        target: Option<Target>,
        note: &str,
    ) -> Self {
        let divisor = match unit {
            "ms" => 1_000_000.0,
            "us" => 1_000.0,
            _ => 1.0,
        };
        let values: Vec<f64> = samples_ns.iter().map(|v| v / divisor).collect();
        Self::from_values(key, title, op, unit, batch, warmup, values, target, note)
    }

    /// 从逐样本值 (已与 `unit` 同单位) 构造。
    pub fn from_values(
        key: &str,
        title: &str,
        op: &str,
        unit: &str,
        batch: usize,
        warmup: usize,
        values: Vec<f64>,
        target: Option<Target>,
        note: &str,
    ) -> Self {
        let stats = Stats::from_values(&values);
        Self {
            key: key.to_string(),
            title: title.to_string(),
            op: op.to_string(),
            unit: unit.to_string(),
            batch,
            warmup,
            stats,
            target,
            note: note.to_string(),
            raw: values,
        }
    }

    /// 判定文案: 有目标按 P50 比较 (达标/未达标 + 差距倍数); 无目标如实标注。
    pub fn verdict(&self) -> String {
        match &self.target {
            None => "— (无目标行, 仅记录)".to_string(),
            Some(target) => {
                if self.stats.p50 <= target.value {
                    let mut text = format!(
                        "✅ 达标 (P50 {:.4} {} ≤ {})",
                        self.stats.p50, self.unit, target.label
                    );
                    if self.stats.p99 > target.value {
                        text.push_str(&format!(
                            "; P99 {:.4} {} 超目标, 如实标注",
                            self.stats.p99, self.unit
                        ));
                    }
                    text
                } else {
                    format!(
                        "❌ 未达标 (P50 {:.4} {}, 差距 {:.2} 倍 vs {})",
                        self.stats.p50,
                        self.unit,
                        self.stats.p50 / target.value,
                        target.label
                    )
                }
            }
        }
    }

    /// markdown 表行 (供 run-benchmarks.ps1 汇总); 字段以 `|` 分隔。
    pub fn row(&self) -> String {
        let target_label = self
            .target
            .as_ref()
            .map(|t| t.label.clone())
            .unwrap_or_else(|| "—".to_string());
        format!(
            "ROW|{}|{}|{}|{}|{}|{:.4}|{:.4}|{:.4}|{:.4}|{:.4}|{:.4}|{}|{}|{}",
            self.key,
            sanitize(&self.title),
            sanitize(&self.op),
            self.stats.n,
            self.unit,
            self.stats.min,
            self.stats.mean,
            self.stats.stddev,
            self.stats.p50,
            self.stats.p99,
            self.stats.max,
            sanitize(&target_label),
            sanitize(&self.verdict()),
            sanitize(&self.note),
        )
    }

    /// 逐样本原始值行 (RAW)。
    pub fn raw_line(&self) -> String {
        let joined = self
            .raw
            .iter()
            .map(|v| format!("{v:.6}"))
            .collect::<Vec<_>>()
            .join(",");
        format!("RAW|{}|{}", self.key, joined)
    }
}

/// 分隔符安全化 (统计行以 `|` 分隔, 字段内不得含 `|`)。
fn sanitize(text: &str) -> String {
    text.replace('|', "/")
}

/// 打印一个基准项的完整结果块 (含 ROW 行; `raw` 为真时附 RAW 行)。
pub fn print_outcome(outcome: &Outcome, raw: bool) {
    println!("### {} [{}]", outcome.title, outcome.key);
    println!("- 操作口径: {}", outcome.op);
    println!(
        "- 样本: n={}, batch={}, warmup={}",
        outcome.stats.n, outcome.batch, outcome.warmup
    );
    println!(
        "- 统计 ({}) : min={:.4} mean={:.4} stddev={:.4} P50={:.4} P90={:.4} P99={:.4} max={:.4}",
        outcome.unit,
        outcome.stats.min,
        outcome.stats.mean,
        outcome.stats.stddev,
        outcome.stats.p50,
        outcome.stats.p90,
        outcome.stats.p99,
        outcome.stats.max
    );
    println!("- 判定: {}", outcome.verdict());
    if !outcome.note.is_empty() {
        println!("- 备注: {}", outcome.note);
    }
    println!("{}", outcome.row());
    if raw {
        println!("{}", outcome.raw_line());
    }
    println!();
}

/// 全局样本缩放 (BENCH_SCALE / BENCH_QUICK)。
#[derive(Debug, Clone, Copy)]
pub struct BenchConfig {
    /// 样本与 warmup 缩放系数。
    pub scale: f64,
    /// 是否输出逐样本 RAW 行。
    pub raw: bool,
}

impl BenchConfig {
    /// 从环境变量读取配置。
    ///
    /// - `BENCH_SCALE=<f64>`: 样本量缩放 (默认 1.0);
    /// - `BENCH_QUICK=1`: 冒烟模式, 缩放强制 0.1;
    /// - `BENCH_RAW=1`: 输出逐样本原始值。
    pub fn from_env() -> Self {
        let quick = std::env::var("BENCH_QUICK").is_ok_and(|v| v == "1");
        let mut scale = std::env::var("BENCH_SCALE")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(1.0);
        if quick {
            scale = (scale * 0.1).min(scale);
        }
        if !scale.is_finite() || scale <= 0.0 {
            scale = 1.0;
        }
        let raw = std::env::var("BENCH_RAW").is_ok_and(|v| v == "1");
        Self { scale, raw }
    }

    /// 按缩放取样本数 (至少 1)。
    pub fn samples(&self, base: usize) -> usize {
        ((base as f64 * self.scale).round() as usize).max(1)
    }
}

/// 批量计时: 每样本连续跑 `batch` 次 `op`, 返回逐操作平均耗时 (纳秒)。
pub fn measure_per_op<F: FnMut()>(
    warmup: usize,
    samples: usize,
    batch: usize,
    mut op: F,
) -> Vec<f64> {
    for _ in 0..warmup {
        for _ in 0..batch {
            op();
        }
    }
    let mut out = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        for _ in 0..batch {
            op();
        }
        let elapsed_ns = start.elapsed().as_nanos() as f64;
        out.push(elapsed_ns / batch as f64);
    }
    out
}

/// 手写计时循环的单次观测 (纳秒), 供需要穿插准备/清理工作的基准使用。
pub fn observe_ns<F: FnOnce()>(op: F) -> f64 {
    let start = Instant::now();
    op();
    start.elapsed().as_nanos() as f64
}

/// 进程内唯一的临时目录 (自动清理由调用方负责, 见 [`cleanup_dir`])。
pub fn temp_dir(tag: &str) -> std::path::PathBuf {
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir =
        std::env::temp_dir().join(format!("apeireth-bench-{tag}-{}-{seq}", std::process::id()));
    fs_err::create_dir_all(&dir).expect("创建基准临时目录");
    dir
}

/// 尽力清理临时目录 (失败不 panic: 清理失败不影响测量有效性)。
pub fn cleanup_dir(dir: &std::path::Path) {
    let _ = fs_err::remove_dir_all(dir);
}
