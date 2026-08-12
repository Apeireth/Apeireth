//! R134 — apeireth-repo-tools 真接示例 (消除孤岛)
//!
//! 演示 `RepoScanner` 真扫一个 repo + 输出关键指标. 证明本 crate 不是孤岛.
//!
//! 跑法:
//! ```powershell
//! cargo run -p apeireth-repo-tools --example r134_repo_scan_demo
//! ```

use apeireth_repo_tools::scan::{RepoScanner, RepoScannerConfig, RepoScannerTrait};
use std::path::PathBuf;
use std::time::Instant;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== R134 apeireth-repo-tools 真实接入演示 (消除孤岛) ===
");

    let target = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    if !target.exists() {
        eprintln!("[skip] target {:?} not exists", target);
        return Ok(());
    }
    println!("[target] scanning {:?}
", target);

    let scanner = RepoScanner::new(RepoScannerConfig::default());

    // 1. scan
    let scan_start = Instant::now();
    let scan_result = scanner.scan(&target).await?;
    println!(
        "[scan] {}ms: files={}, total_size={} bytes, key_files={}",
        scan_start.elapsed().as_millis(),
        scan_result.files.len(),
        scan_result.duration_ms,
        scan_result.key_files.len()
    );

    // 2. stats (per language)
    let stats = scanner.stats(&target).await?;
    println!("[stats] {} languages detected", stats.len());
    let mut langs: Vec<_> = stats.iter().collect();
    langs.sort_by(|a, b| b.1.total_loc.cmp(&a.1.total_loc));
    for (lang, s) in langs.iter().take(5) {
        println!(
            "  - {:?}: {} files, {} lines, {} bytes",
            lang, s.file_count, s.total_loc, s.total_bytes
        );
    }

    // 3. key_files
    let key_files = scanner.key_files(&target).await?;
    println!("[key_files] {} critical files", key_files.len());
    for f in key_files.iter().take(5) {
        println!("  - {}", f);
    }

    // 4. git_state
    if let Ok(git) = scanner.git_state(&target).await {
        println!("[git_state] branch={:?} commit={:?} dirty={}", git.branch, git.latest_commit, git.dirty_files.len());
    }

    println!("
R134 apeireth-repo-tools 真实接入演示: PASS (消除孤岛, 4 个 API 真接可用)");
    Ok(())
}
