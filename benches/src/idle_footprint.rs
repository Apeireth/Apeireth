//! 待机内存基准: 完整生产组装 Runtime 驻留后的进程工作集。
//!
//! 口径: 用 [`crate::assembly`] 同款组装把生产 Runtime 装进本进程并保持驻留,
//! 静置 2 秒让分配器/页缓存稳定后, 每 500 ms 读一次**本进程工作集**
//! (Windows: PowerShell `Get-Process ... .WorkingSet64`; Linux: `/proc/self/statm`
//! 常驻页), 取 5 个读数。附组装前基线读数 (仅进程自身), 差值即组装增量。
//!
//! 诚实边界: 这是 harness 进程 (链接全 workspace 依赖面) 的驻留工作集,
//! 与最终守护进程二进制的绝对值可能有出入 (后者依赖裁剪/LTO 配置);
//! 读数含页缓存共享与分配器高水位影响, 属真实工作集口径。

use std::thread::sleep;
use std::time::Duration;

use crate::assembly::{assemble_production, tokio_runtime};
use crate::support::{BenchConfig, Outcome, Target};

/// 驻留采样次数。
const READINGS: usize = 5;

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

/// 运行待机内存基准 (单结果项, 5 个读数取统计)。
pub fn run(cfg: &BenchConfig) -> Vec<Outcome> {
    let _ = cfg;
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

    let baseline_note = match baseline {
        Some(bytes) => format!(
            "组装前进程基线工作集 {:.2} MiB, 组装驻留增量 {:.2} MiB",
            bytes as f64 / 1_048_576.0,
            readings[0] - bytes as f64 / 1_048_576.0
        ),
        None => "组装前基线读取不可用".to_string(),
    };

    // 保持驻留到读数结束, 再显式销毁。
    drop(runtime);
    fs_err::remove_dir_all(&dir).ok();

    vec![Outcome::from_values(
        "idle-footprint",
        "待机内存 (进程常驻工作集)",
        "完整生产组装 Runtime 驻留进程的工作集, 静置 2s 后每 500ms 一读, 5 读数",
        "MiB",
        1,
        0,
        readings,
        Some(Target {
            label: "< 35.0 MB".to_string(),
            value: 35.0,
        }),
        &format!(
            "单位 MiB (2^20 B); 目标原文为 MB, 按同数值 MiB 从严判定; Windows 读数取 WorkingSet64; {baseline_note}; n=5 小样本, P99 即最大值附近"
        ),
    )]
}
