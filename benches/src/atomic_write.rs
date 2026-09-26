//! 原子写两档基准: `write_atomic` (完整性档) 与 `write_atomic_durable` (持久档)。
//!
//! 口径: 4 KiB 确定性载荷反复原子替换同一目标文件, 单次写 = 临时文件独占创建 +
//! 写入 + 替换 (+ 持久档的 sync_all 落盘)。Windows 平台差异 (如实标注):
//! 持久档在 Windows 只保证文件本体落盘, 无父目录 fsync 等价语义 (被测代码的
//! 既有平台边界)。无 README 目标行 (核心子系统基线), 仅记录。

use std::hint::black_box;
use std::time::Instant;

use apeireth_core::storage_atomic::{write_atomic, write_atomic_durable, DEFAULT_FILE_MODE};

use crate::support::{cleanup_dir, temp_dir, BenchConfig, Outcome, Rng};

/// 载荷大小 (字节)。
const PAYLOAD_BYTES: usize = 4 * 1024;

/// 确定性 JSON 形态载荷。
fn payload(rng: &mut Rng) -> Vec<u8> {
    let mut text = String::with_capacity(PAYLOAD_BYTES * 2);
    while text.len() < PAYLOAD_BYTES {
        text.push_str(&format!(
            "{{\"id\":{},\"note\":\"原子写基准载荷 deterministic payload\"}},",
            rng.next_u64()
        ));
    }
    text.into_bytes()
}

/// 运行原子写两档基准, 返回 [完整性档, 持久档] 两个结果项。
pub fn run(cfg: &BenchConfig) -> Vec<Outcome> {
    let mut rng = Rng::new(0xA701_1C00);
    let bytes = payload(&mut rng);
    let dir = temp_dir("atomic");
    let target = dir.join("target.bin");

    let samples = cfg.samples(200);
    let warmup = cfg.samples(10);

    let run_tier = |tier: &str,
                    write: &dyn Fn(&std::path::Path, &[u8]) -> std::io::Result<()>,
                    warmup: usize,
                    samples: usize|
     -> Vec<f64> {
        for _ in 0..warmup {
            write(&target, &bytes).expect("预热写入");
        }
        let mut out = Vec::with_capacity(samples);
        for _ in 0..samples {
            let start = Instant::now();
            write(&target, &bytes).expect("原子写入");
            out.push(start.elapsed().as_nanos() as f64);
            black_box(tier);
        }
        out
    };

    let integrity_samples = run_tier(
        "integrity",
        &|path, data| write_atomic(path, data, DEFAULT_FILE_MODE),
        warmup,
        samples,
    );
    let durable_samples = run_tier(
        "durable",
        &|path, data| write_atomic_durable(path, data, DEFAULT_FILE_MODE),
        warmup,
        samples,
    );

    cleanup_dir(&dir);

    vec![
        Outcome::from_ns(
            "atomic-write-integrity",
            "原子写-完整性档 (write_atomic 单次)",
            "write_atomic(4 KiB) 单次: 独占临时文件 + 写入 + 原子替换 (无 fsync)",
            "ms",
            1,
            warmup,
            integrity_samples,
            None,
            "同一目标文件反复替换; 断电语义 = 不承诺崩溃持久 (被测档位的既有契约); 无 README 目标行, 仅记录",
        ),
        Outcome::from_ns(
            "atomic-write-durable",
            "原子写-持久档 (write_atomic_durable 单次)",
            "write_atomic_durable(4 KiB) 单次: 同上 + sync_all 落盘后再替换",
            "ms",
            1,
            warmup,
            durable_samples,
            None,
            "Windows 平台差异: 无父目录 fsync 等价语义, 仅文件本体落盘 (被测代码既有边界); fsync 成本随磁盘/文件系统变化大, 如实呈现",
        ),
    ]
}
