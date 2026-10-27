//! `apeireth memory export` —— 导出**脱机可读**的记忆归档物。
//!
//! # 这个命令做什么
//!
//! 把认知库导出成一份**自说明**的归档物（`offline_archive`）：
//! * **清单**（manifest）：条数、内容哈希、链头账、未纳管条数、生成时间；
//! * **记录**：按 `(timestamp, id)` 升序（⇒ **确定性** ⇒ 同样输入 ⇒ 同样输出）；
//! * **链头账**：按主体升序。
//!
//! ⇒ 导出后**立刻自校验**（`verify_archive`）⇒ ⇒ **不产出「导出了但坏了」的归档物**。
//!
//! # 为什么需要它
//!
//! B4 的第一块砖「**脱机可读**」（`offline_archive`）此前**只有库级能力、没有出口**
//! ⇒ ⇒ 用户拿不到它 ⇒ ⇒ 能力等于不存在。
//! ⇒ 本命令是那个**出口**。
//!
//! # 口径（**如实**）
//!
//! * 导出是**只读**的：不改库、不删东西；
//! * **未纳管**的记录（没有链字段 ⇒ 老库或未接入）**默认一并导出**
//!   ⇒ ⇒ 读方知道哪些没链（`manifest.unmanaged_count`）⇒ ⇒ **不假装它们有问题**；
//! * 退出码：**0 = 导出并自校验通过**；**1 = 自校验失败**（**导出了但坏了** ⇒ 明确报错）；
//!   **2 = 无法导出**（库不存在/打不开/参数错）。
//!
//! # 用法
//!
//! ```sh
//! apeireth memory export --db <COGNITIVE_DB> --out <ARCHIVE.json> [--subject <SUBJECT_ID>] [--limit N]
//! ```

use std::path::{Path, PathBuf};

/// 命令用法（与其它命令的 `USAGE` 风格一致）。
pub const USAGE: &str = "apeireth memory export [--db <COGNITIVE_DB>] --out <ARCHIVE.json> [--subject <SUBJECT_ID>] [--limit N]\n\n  只读导出：把认知库导出成**自说明**的脱机归档物，并**当场自校验**。\n\n  --db      认知库路径（缺省 = APEIRETH_COGNITIVE_DB 或数据目录下 cognitive.sqlite3）\n  --out     归档物输出路径（**必填**）\n  --subject 只导出某个主体（缺省 = 全部）\n  --limit   最多导出多少条（缺省 = 不限）\n\n  退出码：0 = 导出并自校验通过 · 1 = 自校验失败 · 2 = 无法导出";

/// 导出结果（供 CLI 渲染；也便于测试断言）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportReport {
    /// 认知库路径。
    pub db_path: PathBuf,
    /// 归档物输出路径。
    pub out_path: PathBuf,
    /// 导出的记录条数。
    pub entries: usize,
    /// 链头账里的主体数。
    pub chain_heads: usize,
    /// **未纳管**的记录数（没有链字段 ⇒ 老库/未接入）。
    pub unmanaged_entries: usize,
    /// 内容哈希（**自校验用** ⇒ 可与读方核对）。
    pub content_digest: String,
}

/// 解析参数（**纯函数** ⇒ 好测）。
///
/// # 返回
///
/// `Err(())` ⇒ 参数有问题 ⇒ 调用方打印 `USAGE`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportOptions {
    /// 认知库路径（`None` = 缺省路径）。
    pub db: Option<PathBuf>,
    /// 输出路径（**必填**）。
    pub out: PathBuf,
    /// 只导出某个主体。
    pub subject: Option<String>,
    /// 条数上限（`0` = 不限）。
    pub limit: usize,
}

/// 解析命令行参数。
pub fn parse_options(args: &[String]) -> Result<ExportOptions, ()> {
    let mut db: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut subject: Option<String> = None;
    let mut limit = 0usize;
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--db" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(());
                };
                db = Some(PathBuf::from(value));
                index += 2;
            }
            "--out" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(());
                };
                out = Some(PathBuf::from(value));
                index += 2;
            }
            "--subject" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(());
                };
                subject = Some(value.clone());
                index += 2;
            }
            "--limit" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(());
                };
                limit = value.parse().map_err(|_| ())?;
                index += 2;
            }
            _ => return Err(()),
        }
    }
    let out = out.ok_or(())?;
    Ok(ExportOptions {
        db,
        out,
        subject,
        limit,
    })
}

/// 缺省认知库路径：`APEIRETH_COGNITIVE_DB` 或 `<数据根>/cognitive.sqlite3`。
fn default_db_path() -> PathBuf {
    if let Ok(explicit) = std::env::var("APEIRETH_COGNITIVE_DB") {
        if !explicit.trim().is_empty() {
            return PathBuf::from(explicit);
        }
    }
    crate::default_panel_data_dir().join("cognitive.sqlite3")
}

/// 跑导出（**只读**）。
///
/// # 返回
///
/// * `Ok(report)` ⇒ 导出并**自校验通过**；
/// * `Err(())` ⇒ **参数错**（打印 `USAGE`）；
/// * `Err(message)` ⇒ **无法导出**或**自校验失败**（**明确区分**）。
pub fn run_export(args: &[String]) -> Result<ExportReport, ExportFailure> {
    let options = parse_options(args).map_err(|_| ExportFailure::Usage)?;
    let db_path = options.db.clone().unwrap_or_else(default_db_path);
    if !db_path.exists() {
        return Err(ExportFailure::Unavailable(format!(
            "认知库不存在：{}",
            db_path.display()
        )));
    }
    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|error| ExportFailure::Unavailable(format!("打不开认知库：{error}")))?;
    let config = apeireth_memory::offline_archive::ExportConfig {
        subject: options.subject.clone(),
        limit: options.limit,
        include_unmanaged: true,
    };
    let created_at_ms = chrono::Utc::now().timestamp_millis();
    let archive = apeireth_memory::offline_archive::export_archive(&conn, &config, created_at_ms)
        .map_err(ExportFailure::Unavailable)?;
    // ⚠️ **自校验**：不产出「导出了但坏了」的归档物。
    apeireth_memory::offline_archive::verify_archive(&archive).map_err(ExportFailure::Verify)?;
    let payload = serde_json::to_string_pretty(&archive)
        .map_err(|error| ExportFailure::Unavailable(format!("序列化失败：{error}")))?;
    std::fs::write(&options.out, payload).map_err(|error| {
        ExportFailure::Unavailable(format!("写不出去（{}）：{error}", options.out.display()))
    })?;
    Ok(ExportReport {
        db_path,
        out_path: options.out,
        entries: archive.episodes.len(),
        chain_heads: archive.chain_heads.len(),
        unmanaged_entries: archive.manifest.unmanaged_count,
        content_digest: archive.manifest.content_digest.clone(),
    })
}

/// 导出失败的三种形态（**分开** ⇒ 退出码不同 ⇒ 机器可判）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportFailure {
    /// 参数错（⇒ 打印用法）。
    Usage,
    /// 无法导出（库不存在 / 打不开 / 写不出去）。
    Unavailable(String),
    /// **导出了但自校验失败**（⇒ 明确报错，不粉饰）。
    Verify(String),
}

/// 人话报告。
pub fn render(report: &ExportReport) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push("记忆导出完成".to_string());
    lines.push(String::new());
    lines.push(format!("  认知库：{}", report.db_path.display()));
    lines.push(format!("  归档物：{}", report.out_path.display()));
    lines.push(format!("  导出条数：{}", report.entries));
    lines.push(format!("  链头主体数：{}", report.chain_heads));
    lines.push(format!(
        "  未纳管条数：{}（**没有链字段** ⇒ 老库或未接入；已一并导出）",
        report.unmanaged_entries
    ));
    lines.push(format!("  内容哈希：{}", report.content_digest));
    lines.push(String::new());
    lines.push("  ✅ **自校验通过**（条数 / 哈希 / 链账一致）".to_string());
    lines.join("\n")
}

/// 便捷入口（给 `main.rs` 用）⇒ 返回**退出码**（`u8` ⇒ `ExitCode::from`）。
pub fn run(args: &[String]) -> u8 {
    match run_export(args) {
        Ok(report) => {
            println!("{}", render(&report));
            0
        }
        Err(ExportFailure::Usage) => {
            eprintln!("{USAGE}");
            2
        }
        Err(ExportFailure::Unavailable(message)) => {
            eprintln!("memory export 无法执行：{message}");
            2
        }
        Err(ExportFailure::Verify(message)) => {
            eprintln!("memory export **自校验失败**（归档物不可信）：{message}");
            1
        }
    }
}

/// 便利函数：把一个**给定连接**导出到**给定路径**。
///
/// # 为什么是公开的
///
/// 集成测试（`tests/`）看不到 `#[cfg(test)]` 的项 ⇒ ⇒ 若要测「导出 + 自校验」这条链
/// ⇒ ⇒ 这个函数必须**公开**（它本身也是**合理的库级用法**：
/// 任何拿到连接的人都能导出，不必走 CLI 参数）。
pub fn export_to_path(
    conn: &rusqlite::Connection,
    out: &Path,
    subject: Option<String>,
    limit: usize,
) -> Result<ExportReport, ExportFailure> {
    let config = apeireth_memory::offline_archive::ExportConfig {
        subject,
        limit,
        include_unmanaged: true,
    };
    let archive = apeireth_memory::offline_archive::export_archive(conn, &config, 0)
        .map_err(ExportFailure::Unavailable)?;
    apeireth_memory::offline_archive::verify_archive(&archive).map_err(ExportFailure::Verify)?;
    let payload = serde_json::to_string_pretty(&archive)
        .map_err(|error| ExportFailure::Unavailable(format!("序列化失败：{error}")))?;
    std::fs::write(out, payload).map_err(|error| {
        ExportFailure::Unavailable(format!("写不出去（{}）：{error}", out.display()))
    })?;
    Ok(ExportReport {
        db_path: PathBuf::from("<memory>"),
        out_path: out.to_path_buf(),
        entries: archive.episodes.len(),
        chain_heads: archive.chain_heads.len(),
        unmanaged_entries: archive.manifest.unmanaged_count,
        content_digest: archive.manifest.content_digest.clone(),
    })
}
