//! 待机内存基准: 完整生产组装 Runtime 驻留后的进程工作集。
//!
//! 口径: 用 [`crate::assembly`] 同款组装把生产 Runtime 装进**独立子进程**并保持驻留,
//! 静置 2 秒让分配器/页缓存稳定后, 每 500 ms 读一次**该子进程工作集**
//! (Windows: PowerShell `Get-Process ... .WorkingSet64`; Linux: `/proc/self/statm`
//! 常驻页), 取 5 个读数。附组装前基线读数 (仅进程自身), 差值即组装增量。
//!
//! ── 为什么必须在**独立子进程**里测 (2026-10-24 修正) ──────────────────────
//! 本基准原先在 harness 进程内直接测。问题: `all` 里它排在**第 8 位**
//! (前面已跑 `hybrid-search` 建 10K 节点索引、`cold-start` 拉子进程等),
//! 于是测到的根本不是「待机」, 而是「做完 7 个基准后的分配器高水位 + 页缓存」。
//!
//! 实测 (本机, 同一二进制、同一时间窗口):
//! ```
//!   单独跑 idle-footprint  → P50 11.62 / 11.48 / 11.55 MiB  (✅ 达标)
//!   放进 all 里跑         → P50 72.34 MiB                    (❌ 2.07 倍超标)
//! ```
//! **差 6 倍** —— 全部来自测量顺序, 不是产品内存回归。
//!
//! 修法: 父进程 spawn 一个**新起的** harness 子进程, 由它完成组装与读数,
//! 父进程只解析它回传的读数。这样无论 `all` 里排在第几位, 测的都是
//! 「一个刚启动、装完 Runtime 就待机的进程」—— 与守护进程的真实语义一致。
//!
//! 诚实边界: 这是 harness 进程 (链接全 workspace 依赖面) 的驻留工作集,
//! 与最终守护进程二进制的绝对值可能有出入 (后者依赖裁剪/LTO 配置);
//! 读数含页缓存共享与分配器高水位影响, 属真实工作集口径。
//! 另注: 本指标**对系统内存压力极敏感** —— 同机同码, 干净环境 11.5 MiB,
//! 在并发编译/测试负载下可读到 72 MiB。因此它在共享 runner 上**不可作门禁**,
//! 只宜作趋势记录。

use std::io::Write;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

use crate::assembly::{assemble_production, tokio_runtime};
use crate::support::{BenchConfig, Outcome, Target};

/// 驻留采样次数。
const READINGS: usize = 5;

/// 内部探针子命令名 (隐藏, 不进 `BENCH_NAMES` 的对外清单)。
pub const PROBE_ARG: &str = "__idle-footprint-probe";

/// 探针回传行前缀。
const PROBE_PREFIX: &str = "PROBE|";

/// 读取本进程工作集字节数 (平台分支, 失败返 None)。
fn working_set_bytes() -> Option<u64> {
    #[cfg(windows)]
    {
        let script = format!("(Get-Process -Id {}).WorkingSet64", std::process::id());
        for shell in ["powershell", "pwsh"] {
            if let Ok(output) = std::process::Command::new(shell)
                .args(["-NoProfile", "-NonInteractive", "-Command", &script])
                .output()
            {
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout);
                    if let Ok(value) = text.trim().parse::<u64>() {
                        return Some(value);
                    }
                }
            }
        }
        None
    }
    #[cfg(target_os = "linux")]
    {
        let statm = fs_err::read_to_string("/proc/self/statm").ok()?;
        let resident_pages: u64 = statm.split_whitespace().nth(1)?.parse().ok()?;
        Some(resident_pages * 4096)
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        None
    }
}

/// 在**本进程**内完成组装 + 读数, 返回 (组装前基线字节, 读数列表 MiB)。
///
/// 这是探针子进程里走的纯测量路径。
fn measure_here() -> (Option<u64>, Vec<f64>) {
    let baseline = working_set_bytes();

    let rt = tokio_runtime();
    let dir = std::env::temp_dir().join(format!("apeireth-bench-idle-{}", std::process::id()));
    fs_err::create_dir_all(&dir).expect("创建待机数据目录");
    let runtime = rt.block_on(assemble_production(&dir));

    sleep(Duration::from_secs(2));
    let mut readings: Vec<f64> = Vec::with_capacity(READINGS);
    for i in 0..READINGS {
        if i > 0 {
            sleep(Duration::from_millis(500));
        }
        let bytes = working_set_bytes().expect("读取进程工作集");
        readings.push(bytes as f64 / 1_048_576.0);
    }

    // 保持驻留到读数结束, 再显式销毁。
    drop(runtime);
    fs_err::remove_dir_all(&dir).ok();

    (baseline, readings)
}

/// 探针入口: 在**子进程**里跑测量, 把读数打到 stdout 一行 (`PROBE|...`)。
///
/// 由父进程 (见 [`run`]) 以 `__idle-footprint-probe` 参数拉起。
pub fn run_probe() {
    let (baseline, readings) = measure_here();
    let mut line = String::from(PROBE_PREFIX);
    match baseline {
        Some(bytes) => line.push_str(&format!("{bytes}")),
        None => line.push('-'),
    }
    for value in &readings {
        line.push('|');
        line.push_str(&format!("{value:.6}"));
    }
    let mut stdout = std::io::stdout();
    let _ = writeln!(stdout, "{line}");
    let _ = stdout.flush();
}

/// 拉起**独立子进程**完成测量, 返回 (基线字节, 读数列表 MiB)。
///
/// 失败 (找不到自身可执行文件 / 子进程异常 / 回传格式不对) 时返回 `None`,
/// 由调用方降级为本进程测量并在备注里如实标注 —— 宁可给出「口径打折」的数,
/// 也不静默不出数。
fn measure_in_child() -> Option<(Option<u64>, Vec<f64>)> {
    let exe = std::env::current_exe().ok()?;
    let output = Command::new(exe).arg(PROBE_ARG).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().find(|l| l.starts_with(PROBE_PREFIX))?;
    let mut parts = line[PROBE_PREFIX.len()..].split('|');
    let baseline = match parts.next()? {
        "-" => None,
        raw => Some(raw.parse::<u64>().ok()?),
    };
    let readings: Vec<f64> = parts.filter_map(|p| p.parse::<f64>().ok()).collect();
    if readings.is_empty() {
        return None;
    }
    Some((baseline, readings))
}

/// 运行待机内存基准 (单结果项, 5 个读数取统计)。
pub fn run(cfg: &BenchConfig) -> Vec<Outcome> {
    let _ = cfg;

    // 优先在**独立子进程**里测 (口径正确: 那个进程是「刚启动 → 装 Runtime → 待机」)。
    let (baseline, readings, isolation) = match measure_in_child() {
        Some((baseline, readings)) => (baseline, readings, "独立子进程 (口径正确)"),
        None => {
            let (baseline, readings) = measure_here();
            (
                baseline,
                readings,
                "⚠️ 子进程测量不可用, 已降级为本进程测量 (读数含本进程既有占用, 口径偏大)",
            )
        }
    };

    let baseline_note = match baseline {
        Some(bytes) => format!(
            "组装前进程基线工作集 {:.2} MiB, 组装驻留增量 {:.2} MiB",
            bytes as f64 / 1_048_576.0,
            readings[0] - bytes as f64 / 1_048_576.0
        ),
        None => "组装前基线读取不可用".to_string(),
    };

    vec![Outcome::from_values(
        "idle-footprint",
        "待机内存 (进程常驻工作集)",
        "独立子进程: 完整生产组装 Runtime 驻留后静置 2s, 每 500ms 一读, 5 读数",
        "MiB",
        1,
        0,
        readings,
        Some(Target {
            label: "< 35.0 MB".to_string(),
            value: 35.0,
        }),
        &format!(
            "单位 MiB (2^20 B); 目标原文为 MB, 按同数值 MiB 从严判定; Windows 读数取 WorkingSet64; \
             测量隔离: {isolation}; {baseline_note}; n=5 小样本, P99 即最大值附近; \
             本指标对系统内存压力极敏感 (同机同码 干净 11.5 MiB / 负载下 72 MiB), 宜作趋势不作门禁"
        ),
    )]
}
