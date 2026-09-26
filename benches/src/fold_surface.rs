//! 会话事件折叠基准: 1,000 条事件日志的 `fold_surface` 纯折叠。
//!
//! 口径: 日志恰为 1,000 条 LogEntry 记录 = 960 条 MessageAppended (seq 0..960,
//! 用户/助手交替, 混合 ASCII+CJK 文本) + 20 条 Masked + 20 条 SurfaceReplaced
//! (互不重叠的 2 消息跨度替换)。被测操作 = `fold_surface(&log)` 整次折叠
//! (派生 SurfaceView, 不含 `messages()` 物化)。折叠是纯函数, 同一日志重复折叠
//! 结果相同; 每样本 = batch 次折叠的平均。

use std::hint::black_box;
use std::time::Instant;

use apeireth_core::kernel::Timestamp;
use apeireth_protocol::canonical::NormalizedMessage;
use apeireth_runtime::{fold_surface, LogEntry, SessionEventKind, SurfaceOp};

use crate::support::{BenchConfig, Outcome, Rng};

/// 日志总记录数。
const RECORDS: usize = 1000;
/// MessageAppended 记录数。
const APPENDS: usize = 960;
/// 每样本折叠次数。
const BATCH: usize = 5;

/// 构造 1,000 条确定性事件日志。
fn build_log() -> Vec<LogEntry> {
    let mut rng = Rng::new(0xF01D_5EED);
    let ascii: Vec<String> = (0..256).map(|k| format!("w{k}")).collect();
    let cjk = [
        "记忆", "事件", "折叠", "会话", "视图", "替换", "掩码", "重放",
    ];

    let mut log = Vec::with_capacity(RECORDS);
    for seq in 0..APPENDS {
        let mut text = String::new();
        for _ in 0..rng.range(24, 48) {
            if rng.next_u64() % 3 == 0 {
                text.push_str(rng.pick(&cjk));
            } else {
                text.push_str(rng.pick(&ascii));
            }
            text.push(' ');
        }
        let message = if seq % 2 == 0 {
            NormalizedMessage::user(text)
        } else {
            NormalizedMessage::assistant(text)
        };
        log.push(LogEntry {
            at: Timestamp::now(),
            request: None,
            trace: None,
            event: SessionEventKind::MessageAppended { seq, message },
        });
    }
    // 20 条掩码 + 20 条跨度替换: 每 48 消息一格, 替换格首 2 条, 掩码格中 1 条。
    for cell in 0..20usize {
        log.push(LogEntry {
            at: Timestamp::now(),
            request: None,
            trace: None,
            event: SessionEventKind::SurfaceReplaced {
                op: SurfaceOp::replace(cell * 48, cell * 48 + 2),
                replacement: vec![NormalizedMessage::user(format!("折叠摘要 {cell}"))],
                note: "bench".to_string(),
            },
        });
        log.push(LogEntry {
            at: Timestamp::now(),
            request: None,
            trace: None,
            event: SessionEventKind::Masked {
                seq: cell * 48 + 24,
                reason: "bench".to_string(),
            },
        });
    }
    assert_eq!(log.len(), RECORDS, "日志恰为 {RECORDS} 条记录");
    log
}

/// 运行会话事件折叠基准。
pub fn run(cfg: &BenchConfig) -> Vec<Outcome> {
    let log = build_log();

    // 健全性预检: 折叠无拒绝记录且视图非空。
    let probe = fold_surface(&log);
    assert!(
        probe.rejected.is_empty(),
        "构造日志不应产生被拒记录: {:?}",
        probe.rejected
    );
    assert!(!probe.segments.is_empty(), "折叠视图非空");
    black_box(probe);

    let samples = cfg.samples(60);
    let warmup = cfg.samples(3);
    for _ in 0..warmup {
        for _ in 0..BATCH {
            black_box(fold_surface(&log));
        }
    }
    let mut samples_ns = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        for _ in 0..BATCH {
            black_box(fold_surface(&log));
        }
        samples_ns.push(start.elapsed().as_nanos() as f64 / BATCH as f64);
    }

    vec![Outcome::from_ns(
        "fold-surface",
        "会话事件折叠 (fold_surface / 1000 事件)",
        "fold_surface(&[LogEntry; 1000]) 单次纯折叠 (960 追加 + 20 替换 + 20 掩码)",
        "us",
        BATCH,
        warmup,
        samples_ns,
        None,
        "每样本 = 5 次折叠的平均; 日志构造不计时; 纯函数无 I/O, 无 README 目标行, 仅记录",
    )]
}
