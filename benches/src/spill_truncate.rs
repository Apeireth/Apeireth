//! 溢出落盘取回基准: `truncate_with_spill` 单次溢出写 + 落盘文件取回。
//!
//! 口径:
//! - `spill-truncate`: `truncate_with_spill(content, keep_chars=2000, writer, scope)`
//!   单次调用 (内容 128 KiB 确定性长尾文本, 超预算是常态路径: 全文先落盘
//!   `<temp>/spill/` 再生成头/尾预览 + 取回指引); 计时只含该调用,
//!   落盘文件的删除清理不计时;
//! - `spill-retrieve`: 按取回指引把落盘全文读回 (`fs_err::read`), 校验字节数
//!   与原文一致; 单次读取计时。
//!
//! 无 README 目标行 (核心子系统基线), 仅记录。

use std::hint::black_box;
use std::time::Instant;

use apeireth_orchestration::{truncate_with_spill, SpillWriter};

use crate::support::{cleanup_dir, temp_dir, BenchConfig, Outcome, Rng};

/// 长尾内容大小 (字节)。
const CONTENT_BYTES: usize = 128 * 1024;
/// 保留预览字符数。
const KEEP_CHARS: usize = 2000;

/// 确定性长尾文本 (ASCII + CJK 混合, 约 CONTENT_BYTES 字节)。
fn long_tail(rng: &mut Rng) -> String {
    let chunk_ascii = "overflow tail payload for spill benchmark; ";
    let chunk_cjk = "溢出长尾内容落盘取回基准文本段落。";
    let mut text = String::with_capacity(CONTENT_BYTES + 1024);
    while text.len() < CONTENT_BYTES {
        if rng.next_u64() % 2 == 0 {
            text.push_str(chunk_ascii);
        } else {
            text.push_str(chunk_cjk);
        }
    }
    text
}

/// 运行溢出落盘取回基准, 返回 [落盘截断, 取回读取] 两个结果项。
pub fn run(cfg: &BenchConfig) -> Vec<Outcome> {
    let mut rng = Rng::new(0x5F11_1BEE);
    let content = long_tail(&mut rng);
    let dir = temp_dir("spill");
    let writer = SpillWriter::new(dir.as_path());

    // ---- 落盘 + 截断: truncate_with_spill 单次调用 ----
    let samples = cfg.samples(200);
    let warmup = cfg.samples(10);
    for _ in 0..warmup {
        let out = truncate_with_spill(&content, KEEP_CHARS, &writer, "bench-session");
        assert!(out.spilled_path.is_some(), "超预算内容必须落盘");
        let _ = fs_err::remove_file(out.spilled_path.expect("落盘路径"));
    }
    let mut truncate_samples = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        let out = truncate_with_spill(&content, KEEP_CHARS, &writer, "bench-session");
        truncate_samples.push(start.elapsed().as_nanos() as f64);
        assert!(out.spilled_path.is_some(), "超预算内容必须落盘");
        let path = out.spilled_path.expect("落盘路径");
        assert!(out.text.len() < content.len(), "预览必须短于原文");
        // 清理不计时。
        let _ = fs_err::remove_file(path);
    }

    // ---- 取回: 按指引读回落盘全文 ----
    let probe = truncate_with_spill(&content, KEEP_CHARS, &writer, "bench-session");
    let spill_path = probe.spilled_path.expect("落盘路径");
    for _ in 0..warmup {
        let bytes = fs_err::read(&spill_path).expect("取回落盘全文");
        black_box(bytes);
    }
    let mut retrieve_samples = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        let bytes = fs_err::read(&spill_path).expect("取回落盘全文");
        retrieve_samples.push(start.elapsed().as_nanos() as f64);
        assert_eq!(bytes.len(), content.len(), "取回字节数与原文一致");
        black_box(bytes);
    }

    cleanup_dir(&dir);

    vec![
        Outcome::from_ns(
            "spill-truncate",
            "溢出落盘取回-落盘截断 (truncate_with_spill 单次)",
            "truncate_with_spill(128 KiB 长尾, keep=2000) 单次: 全文落盘 + 头/尾预览 + 取回指引",
            "ms",
            1,
            warmup,
            truncate_samples,
            None,
            "内容 128 KiB, keep_chars=2000; 计时含 spill 文件创建+写入 (无 fsync), 不含事后删除清理; 临时目录在系统 %TEMP%; 无 README 目标行, 仅记录",
        ),
        Outcome::from_ns(
            "spill-retrieve",
            "溢出落盘取回-全文取回",
            "按取回指引 fs_err::read 读回落盘全文 (128 KiB) 单次",
            "ms",
            1,
            warmup,
            retrieve_samples,
            None,
            "同一落盘文件重复读取 (页缓存热), 呈现热读取下界; 冷读取 (重启/驱逐后) 会更慢, 如实标注口径边界",
        ),
    ]
}
