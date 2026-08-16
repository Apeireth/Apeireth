//! R32-3-2: 跨 4 model benchmark example — 跑 MiniMax-M2.7-highspeed / M2.7 / M2.5 / M3
//!
//! 跑法: `cargo run -p apeireth-eval --example r32-3-2_cross_model_benchmark`
//! 需 env `APEIRETH_MINIMAX_API_KEY` (或 openclaw/apikey.txt) 开启 LIVE 调用.
//!
//! 输出: stdout JSON 报告 + markdown 表格
//! 写文件: 报告写到 `reports/r32-3-2-cross-model-benchmark-YYYY-MM-DD.md`

use apeireth_eval::cross_model_benchmark::{report_to_markdown, run_cross_model_benchmark};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let api_key = std::env::var("APEIRETH_MINIMAX_API_KEY").ok();
    let report = run_cross_model_benchmark(&workspace_root, api_key.as_deref(), None).await;

    // stdout JSON
    let json = serde_json::to_string_pretty(&report)?;
    println!("{json}");

    // markdown 摘要
    let md = report_to_markdown(&report);
    println!("\n--- markdown ---\n{md}");

    // 写到 reports/ (best-effort)
    let reports_dir = workspace_root.join("reports");
    if let Err(e) = std::fs::create_dir_all(&reports_dir) {
        eprintln!("create reports dir 失败: {e}");
    } else {
        let stamp = chrono_like_now_short();
        let path = reports_dir.join(format!("r32-3-2-cross-model-benchmark-{stamp}.md"));
        if let Err(e) = std::fs::write(&path, &md) {
            eprintln!("写 {path:?} 失败: {e}");
        } else {
            eprintln!("\n报告写入: {path:?}");
        }
    }

    Ok(())
}

fn chrono_like_now_short() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86400;
    let mut year = 1970i64;
    let mut d = days as i64;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let yd = if leap { 366 } else { 365 };
        if d < yd {
            break;
        }
        d -= yd;
        year += 1;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let months = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1u32;
    for &md in &months {
        if d < md {
            return format!("{year:04}-{month:02}-{:02}", (d + 1) as u32);
        }
        d -= md;
        month += 1;
    }
    format!("{year:04}-12-31")
}
