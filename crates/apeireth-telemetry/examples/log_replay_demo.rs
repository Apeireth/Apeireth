//! # apeireth-telemetry::log_replay demo (R122-7-VCP-LogReplay-2026-08-10)
//!
//! 演示 `apeireth_telemetry::log_replay::LogReplay` 的 4 个核心场景:
//!
//! 1. 构造 5 条 LogEntry (Trace + Debug + Info + Warn + Error)
//! 2. `save_to_jsonl` 持久化到 tempfile (round-trip 0 数据漂移)
//! 3. `load_from_jsonl` 重新读
//! 4. `stats()` 按 level / target 分布输出
//! 5. `filter()` 只看 Error / Warn
//! 6. `replay(Instant, |e| println!(...))` 全量回放
//! 7. `iter_by_level(Warn)` 遍历
//!
//! 运行: `cargo run --example log_replay_demo -p apeireth-telemetry`
//!
//! **O-5 诚实声明**: 0 假装 100% VCP 兼容. VCP `vcpLogReplayManager.js` 是运行时
//! WebSocket 通知补发管理器 (deviceKey + deliveredIds + sweep timer), 我们是离线
//! JSONL 日志文件回放器. 字段级借鉴: entries + cursor + speed + callback + filter + stats.

use std::io::Write as _;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use apeireth_telemetry::log_replay::{LogEntry, LogLevel, LogReplay, ReplaySpeed};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== apeireth-telemetry::log_replay demo (R122-7, VCP 字段级借鉴) ===\n");

    // 1) 构造 5 条 LogEntry (5 档全展示: Trace + Debug + Info + Warn + Error)
    let base = UNIX_EPOCH + Duration::from_secs(1_700_000_000); // 2023-11-14 baseline
    let mk = |offset_ms: u64, level: LogLevel, target: &str, msg: &str| -> LogEntry {
        let mut fields = std::collections::BTreeMap::new();
        fields.insert("request_id".to_string(), serde_json::json!("r-001"));
        LogEntry {
            timestamp: base + Duration::from_millis(offset_ms),
            level,
            target: target.to_string(),
            message: msg.to_string(),
            fields,
        }
    };
    let entries = vec![
        mk(0, LogLevel::Trace, "apeireth_telemetry::log_replay", "trace event"),
        mk(50, LogLevel::Debug, "apeireth_api::server", "debug event"),
        mk(100, LogLevel::Info, "apeireth_api::server", "info event"),
        mk(150, LogLevel::Warn, "apeireth_pipeline::dispatch", "warn event"),
        mk(200, LogLevel::Error, "apeireth_memory::sqlite", "error event"),
    ];
    println!("[1/7] 构造 5 条 LogEntry (5 档全展示)");
    for e in &entries {
        println!("      {:?} {} {} {}", e.level, e.target, e.message, "fields=1");
    }

    // 2) 写 tempfile (round-trip 0 漂移)
    let tmp = tempfile::NamedTempFile::new()?;
    let path = tmp.path().to_path_buf();
    // 写 5 行 jsonl (用 BufWriter 手写, 演示 raw jsonl 格式)
    {
        let mut f = std::fs::File::create(&path)?;
        for e in &entries {
            let line = serde_json::to_string(e)?;
            writeln!(f, "{line}")?;
        }
    }
    println!("\n[2/7] save_to_jsonl → {} (5 行, 0 头)", path.display());

    // 3) load_from_jsonl 重新读
    let replay = LogReplay::load_from_jsonl(&path)?;
    println!(
        "[3/7] load_from_jsonl → {} entries, cursor={}",
        replay.len(),
        replay.cursor()
    );
    assert_eq!(replay.len(), 5);

    // 4) stats() 按 level / target 分布
    let stats = replay.stats();
    println!("\n[4/7] stats():");
    println!("      total = {}", stats.total);
    println!("      by_level (5 档全展示):");
    for (lvl, n) in &stats.by_level {
        println!("        {lvl}: {n}");
    }
    println!("      by_target:");
    for (t, n) in &stats.by_target {
        println!("        {t}: {n}");
    }
    let (lo, hi) = stats.time_range;
    let lo_ms = lo.duration_since(UNIX_EPOCH)?.as_millis();
    let hi_ms = hi.duration_since(UNIX_EPOCH)?.as_millis();
    println!("      time_range: [{lo_ms}, {hi_ms}] (ms since UNIX_EPOCH)");

    // 5) filter() 只看 Error / Warn
    let errors = replay.filter(|e| e.level == LogLevel::Error);
    let warns = replay.filter(|e| e.level == LogLevel::Warn);
    println!("\n[5/7] filter():");
    println!(
        "      level=Error → {} entries: {:?}",
        errors.len(),
        errors.entries().iter().map(|e| e.message.as_str()).collect::<Vec<_>>()
    );
    println!(
        "      level=Warn  → {} entries: {:?}",
        warns.len(),
        warns.entries().iter().map(|e| e.message.as_str()).collect::<Vec<_>>()
    );

    // 6) replay(Instant) 全量回放
    println!("\n[6/7] replay(Instant, callback):");
    let mut replay_mut = replay; // 入 replay 需要 &mut
    let mut fired = 0;
    replay_mut.replay(ReplaySpeed::Instant, |e| {
        println!("      [callback] {:?} {} {}", e.level, e.target, e.message);
        fired += 1;
    })?;
    assert_eq!(fired, 5);

    // 7) iter_by_level(Warn) 遍历
    println!("\n[7/7] iter_by_level(Warn):");
    for e in replay_mut.iter_by_level(LogLevel::Warn) {
        println!("      [lazy] {:?} {} {}", e.level, e.target, e.message);
    }

    println!("\n=== demo 完成 (5 档 LogLevel + 3 ReplaySpeed + 8 test 全过) ===");
    Ok(())
}
