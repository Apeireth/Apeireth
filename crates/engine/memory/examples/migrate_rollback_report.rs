//! 迁移回滚**演练报告**（真迁移 ⇒ 核对 ⇒ 回滚 ⇒ 证明「回到原点」）。
//!
//! # 这个例子回答什么
//!
//! 设计稿 §8 **B4**（脱机归档 + **迁移回滚** + handover 演练）的验收 =
//! 「脱机可读；**回滚可演练**」。⇒ 这个例子把 B4 的**第二块砖**跑一遍，回答三件事：
//!
//! 1. **迁得动吗**：用**真实的** `continuity_link::migrate_subject` 迁移一把；
//! 2. **退得回吗**：核对账实 ⇒ 删掉迁移留下的副本 ⇒ 只删副本、不碰原记录；
//! 3. **真回到原点了吗**：**用哈希说话** —— 回滚后的**内容哈希**必须与迁移前**逐字相等**。
//!
//! # 口径（钉死）
//!
//! * **默认跑演练库**：不指定库路径时，例子在**临时文件库**上造 3 条记录再跑演练
//!   ⇒ 跑完即删 ⇒ **不碰任何真实库**；
//! * **指定库路径 ⇒ 需显式开闸**：环境变量 `APEIRETH_ROLLBACK_ALLOW=1` 才允许在
//!   那个库上跑（演练会**真的**迁移 + 回滚）⇒ 默认**拒绝**（不许手滑）；
//! * **迁移时刻注入**（本例子取当前 epoch 毫秒）⇒ 报告可复现；
//! * ⚠️ 只对**测试库**跑（**不许**对生产库跑演练）。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-27）
//!
//! 四级口径：
//!
//! - **IMPLEMENTED** ✔：能在库级跑出完整演练报告（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：生产主路径 0 调用点（只有人工在用）；
//! - **DEFAULT ENABLED** ✘：没有开关位；
//! - **HARDWARE VALIDATED** ✘：**未在生产记忆库上实测过**（本机没有生产库）
//!   ⇒ 不宣称已验证。

use std::path::PathBuf;
use std::process::ExitCode;

use apeireth_core::kernel::memory::Episode;
use apeireth_memory::migrate_rollback::drill_rollback;
use apeireth_memory::SqliteMemoryStore;
use rusqlite::Connection;
use tempfile::TempDir;

/// 库路径（命令行参数优先，其次环境变量）。
const DB_ENV: &str = "APEIRETH_MEMORY_DB";

/// 源锚点（会话锚点 ⇒ 与 `migrate_subject` 的取源口径同源）。
const FROM_ENV: &str = "APEIRETH_ROLLBACK_FROM";

/// 目标锚点。
const TO_ENV: &str = "APEIRETH_ROLLBACK_TO";

/// 「允许在**指定库**上跑演练」的闸门（默认关 ⇒ 不许手滑）。
const ALLOW_ENV: &str = "APEIRETH_ROLLBACK_ALLOW";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let from = env_or(FROM_ENV, "anchor-a");
    let to = env_or(TO_ENV, "anchor-b");
    let requested_db = args.first().map(PathBuf::from).or_else(|| env_path(DB_ENV));

    match requested_db {
        Some(path) => {
            if env_or(ALLOW_ENV, "") != "1" {
                eprintln!("· 指定了库路径 ⇒ 演练会**真的**迁移 + 回滚那个库 ⇒ 默认**拒绝**。");
                eprintln!("  确认那是测试库，再设 {ALLOW_ENV}=1 重跑。");
                eprintln!("  （不指定路径时，例子只跑临时演练库 ⇒ 不碰真实库）");
                return ExitCode::FAILURE;
            }
            run_on(path, &from, &to)
        }
        None => run_on_drill_library(&from, &to),
    }
}

/// 在**临时演练库**上跑一遍（默认路径 ⇒ 不碰真实库）。
fn run_on_drill_library(from: &str, to: &str) -> ExitCode {
    let dir = match TempDir::new() {
        Ok(dir) => dir,
        Err(error) => {
            eprintln!("· 建临时目录失败：{error}");
            return ExitCode::FAILURE;
        }
    };
    let path = dir.path().join("drill.sqlite3");
    let store = match SqliteMemoryStore::open(&path) {
        Ok(store) => store,
        Err(error) => {
            eprintln!("· 建演练库失败：{error}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(error) = seed(&store, from) {
        eprintln!("· 造演练数据失败：{error}");
        return ExitCode::FAILURE;
    }
    println!(
        "· 演练库：{}（临时文件 ⇒ 跑完即删 ⇒ **不碰真实库**）",
        path.display()
    );
    drop(store);
    run_on(path, from, to)
}

/// 在给定库上跑演练并打印报告。
fn run_on(path: PathBuf, from: &str, to: &str) -> ExitCode {
    let conn = match Connection::open(&path) {
        Ok(conn) => conn,
        Err(error) => {
            eprintln!("· 打不开库（{}）：{error}", path.display());
            return ExitCode::FAILURE;
        }
    };
    let mut conn = conn;
    match drill_rollback(&mut conn, from, to, now_ms()) {
        Ok(report) => {
            println!("{report}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("· **演练失败**：{error}");
            ExitCode::FAILURE
        }
    }
}

/// 造 3 条**带链**的原记录（生产默认写路径 ⇒ 自动入链）。
fn seed(store: &SqliteMemoryStore, from: &str) -> Result<(), String> {
    for (id, ts, content) in [
        ("demo-1", 100_i64, "演练事实一"),
        ("demo-2", 101_i64, "演练事实二\n带换行"),
        ("demo-3", 102_i64, "演练事实三"),
    ] {
        let episode = Episode {
            id: id.to_string(),
            timestamp: ts,
            role: "user".to_string(),
            content: content.to_string(),
            session_id: from.to_string(),
        };
        store
            .put_episode_for_subject(&episode, "drill-subject")
            .map_err(|error| format!("写入 {id} 失败：{error}"))?;
    }
    Ok(())
}

/// 环境变量 ⇒ 字符串（没给或空 ⇒ 用默认值）。
fn env_or(key: &str, fallback: &str) -> String {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

/// 环境变量 ⇒ 路径（没给或空 ⇒ `None`）。
fn env_path(key: &str) -> Option<PathBuf> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// 当前 epoch 毫秒（导出/迁移时刻 ⇒ 注入值）。
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}
