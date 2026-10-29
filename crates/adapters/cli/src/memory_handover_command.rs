//! `apeireth memory handover` —— 生成**交接包**（并**当场跑演练**）。
//!
//! # 这个命令做什么
//!
//! 把认知库打包成一份「**交接给下一个人/下一个 agent**」的包：
//! * **数据**：脱机归档物（`offline_archive`）⇒ **随物走**；
//! * **当前状态**：库路径、条数、主体数、**未纳管条数**、**已知问题**（**不隐瞒坏消息**）；
//! * **下一步**：接手者**马上能做的事**（做什么 / 为什么 / 怎么验收 / 工作量）；
//! * **红线**：**不许碰**的东西。
//!
//! ⇒ 打包后**当场跑演练**（`drill_handover`）⇒ ⇒ **不产出「打出来但读不懂」的交接包**。
//!
//! # 为什么需要它
//!
//! B4 第三块砖「**交接演练**」（`handover`）此前**只有库级能力、没有出口**
//! ⇒ ⇒ 能力等于不存在。
//!
//! # 口径（**如实**）
//!
//! * 导出是**只读**的：不改库、不删东西；
//! * **已知问题**由调用方给出（CLI 从参数读）⇒ ⇒ **不隐瞒坏消息**；
//! * 退出码：**0 = 打包并演练通过**；**1 = 演练失败**（**打出来但读不懂** ⇒ 明确报错）；
//!   **2 = 无法打包**（库不存在/打不开/参数错）。
//!
//! # 用法
//!
//! ```sh
//! apeireth memory handover --db <COGNITIVE_DB> --out <PACKAGE.json> [--issue <TEXT>]... [--next <TEXT>]
//! ```

use std::path::{Path, PathBuf};

use apeireth_memory::handover::{
    build_package, default_red_lines, drill_handover, render_package_summary, CurrentState,
    NextStep,
};

/// 命令用法（与其它命令的 `USAGE` 风格一致）。
pub const USAGE: &str = "apeireth memory handover [--db <COGNITIVE_DB>] --out <PACKAGE.json> [--issue <TEXT>]... [--next <TEXT>]\n\n  只读打包：把认知库打成**交接包**（数据 + 状态 + 下一步 + 红线），并**当场跑演练**。\n\n  --db      认知库路径（缺省 = APEIRETH_COGNITIVE_DB 或数据目录下 cognitive.sqlite3）\n  --out     交接包输出路径（**必填**）\n  --issue   一条**已知问题**（可重复；**不隐瞒坏消息**）\n  --next    一条**下一步**（可重复；格式：做什么 | 为什么 | 怎么验收 | 工作量1-3）\n\n  退出码：0 = 打包并演练通过 · 1 = 演练失败 · 2 = 无法打包";

/// **大包**阈值（字节）⇒ ⇒ 超过就在报告里**如实提醒**。
///
/// # 为什么有这个
///
/// 交接包里的 `archive` 是**数据本体**（**逐字正文**）⇒ ⇒ 包体积**随库线性增长**
/// ⇒ ⇒ 真实库上可能到**几百 KB ~ 几 MB** ⇒ ⇒ **体积是产品口径**
/// （要不要改成「**路径 + 哈希**」）⇒ ⇒ **待 owner 裁决** ⇒ ⇒ 但**先如实报出来**。
pub const LARGE_PACKAGE_BYTES: u64 = 1024 * 1024;

/// 交接包参数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoverOptions {
    /// 认知库路径（`None` = 缺省路径）。
    pub db: Option<PathBuf>,
    /// 输出路径（**必填**）。
    pub out: PathBuf,
    /// **已知问题**（**不隐瞒坏消息**）。
    pub issues: Vec<String>,
    /// **下一步**。
    pub next_steps: Vec<NextStep>,
}

/// 解析命令行参数。
///
/// # 返回
///
/// `Err(())` ⇒ 参数有问题 ⇒ 调用方打印 `USAGE`。
pub fn parse_options(args: &[String]) -> Result<HandoverOptions, ()> {
    let mut db: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut issues: Vec<String> = Vec::new();
    let mut next_steps: Vec<NextStep> = Vec::new();
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
            "--issue" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(());
                };
                issues.push(value.clone());
                index += 2;
            }
            "--next" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(());
                };
                next_steps.push(parse_next_step(value)?);
                index += 2;
            }
            _ => return Err(()),
        }
    }
    let out = out.ok_or(())?;
    Ok(HandoverOptions {
        db,
        out,
        issues,
        next_steps,
    })
}

/// 解析一条 `--next`（格式：`做什么 | 为什么 | 怎么验收 | 工作量1-3`）。
fn parse_next_step(value: &str) -> Result<NextStep, ()> {
    let parts: Vec<&str> = value.split('|').map(str::trim).collect();
    if parts.len() != 4 {
        return Err(());
    }
    let cost: u8 = parts[3].parse().map_err(|_| ())?;
    if cost == 0 || cost > 3 {
        return Err(());
    }
    Ok(NextStep {
        what: parts[0].to_string(),
        why: parts[1].to_string(),
        acceptance: parts[2].to_string(),
        cost,
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

/// 打包失败的三种形态（**分开** ⇒ 退出码不同 ⇒ 机器可判）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandoverFailure {
    /// 参数错（⇒ 打印用法）。
    Usage,
    /// 无法打包（库不存在 / 打不开 / 写不出去）。
    Unavailable(String),
    /// **打包了但演练失败**（⇒ 明确报错，不粉饰）。
    Drill(String),
}

/// 打包结果（供 CLI 渲染；也便于测试断言）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoverReport {
    /// 认知库路径。
    pub db_path: PathBuf,
    /// 交接包输出路径。
    pub out_path: PathBuf,
    /// 导出的记录条数。
    pub entries: usize,
    /// **未纳管**条数（**如实** ⇒ 接手者知道账不完整）。
    pub unmanaged_entries: usize,
    /// **交接包的字节数**（**写盘后**量 ⇒ ⇒ 如实）。
    pub package_bytes: u64,
    /// 演练是否全过。
    pub drill_passed: bool,
    /// 演练的问题清单（全过时为空）。
    pub drill_problems: Vec<String>,
}

/// 跑打包（**只读**）。
pub fn run_handover(args: &[String]) -> Result<HandoverReport, HandoverFailure> {
    let options = parse_options(args).map_err(|_| HandoverFailure::Usage)?;
    let db_path = options.db.clone().unwrap_or_else(default_db_path);
    if !db_path.exists() {
        return Err(HandoverFailure::Unavailable(format!(
            "认知库不存在：{}",
            db_path.display()
        )));
    }
    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|error| HandoverFailure::Unavailable(format!("打不开认知库：{error}")))?;
    let config = apeireth_memory::offline_archive::ExportConfig {
        subject: None,
        limit: 0,
        include_unmanaged: true,
    };
    let created_at_ms = chrono::Utc::now().timestamp_millis();
    let archive = apeireth_memory::offline_archive::export_archive(&conn, &config, created_at_ms)
        .map_err(HandoverFailure::Unavailable)?;
    let state = CurrentState {
        db_path: db_path.display().to_string(),
        episode_count: archive.episodes.len(),
        subject_count: archive.chain_heads.len(),
        unmanaged_count: archive.manifest.unmanaged_count,
        current_algorithm: None,
        known_issues: options.issues.clone(),
    };
    let package = build_package(archive, state, options.next_steps.clone(), created_at_ms);
    // ⭐ **当场跑演练** ⇒ ⇒ 不产出「打出来但读不懂」的交接包。
    let (passed, problems) = drill_handover(&package);
    let payload = serde_json::to_string_pretty(&package)
        .map_err(|error| HandoverFailure::Unavailable(format!("序列化失败：{error}")))?;
    std::fs::write(&options.out, payload).map_err(|error| {
        HandoverFailure::Unavailable(format!("写不出去（{}）：{error}", options.out.display()))
    })?;
    // ⇒ **先量体积**（写盘后）⇒ ⇒ 再移动 `options.out`（避免移动后借用）。
    let package_bytes = std::fs::metadata(&options.out)
        .map(|meta| meta.len())
        .unwrap_or(0);
    Ok(HandoverReport {
        db_path,
        out_path: options.out,
        entries: package.archive.episodes.len(),
        unmanaged_entries: package.state.unmanaged_count,
        package_bytes,
        drill_passed: passed,
        drill_problems: problems,
    })
}

/// 便利函数（测试用 / 库级用法）：从**给定连接**打一个交接包。
///
/// # 为什么公开
///
/// 集成测试（`tests/`）看不到 `#[cfg(test)]` 的项 ⇒ ⇒ 要测「打包 + 演练」这条链
/// ⇒ ⇒ 这个函数必须**公开**（它本身也是**合理的库级用法**）。
pub fn handover_package_from_conn(
    conn: &rusqlite::Connection,
    out: &Path,
    db_path: &Path,
    issues: Vec<String>,
    next_steps: Vec<NextStep>,
) -> Result<HandoverReport, HandoverFailure> {
    let config = apeireth_memory::offline_archive::ExportConfig {
        subject: None,
        limit: 0,
        include_unmanaged: true,
    };
    let created_at_ms = 0;
    let archive = apeireth_memory::offline_archive::export_archive(conn, &config, created_at_ms)
        .map_err(HandoverFailure::Unavailable)?;
    let state = CurrentState {
        db_path: db_path.display().to_string(),
        episode_count: archive.episodes.len(),
        subject_count: archive.chain_heads.len(),
        unmanaged_count: archive.manifest.unmanaged_count,
        current_algorithm: None,
        known_issues: issues,
    };
    let package = build_package(archive, state, next_steps, created_at_ms);
    let (passed, problems) = drill_handover(&package);
    let payload = serde_json::to_string_pretty(&package)
        .map_err(|error| HandoverFailure::Unavailable(format!("序列化失败：{error}")))?;
    std::fs::write(out, payload).map_err(|error| {
        HandoverFailure::Unavailable(format!("写不出去（{}）：{error}", out.display()))
    })?;
    Ok(HandoverReport {
        db_path: db_path.to_path_buf(),
        out_path: out.to_path_buf(),
        entries: package.archive.episodes.len(),
        unmanaged_entries: package.state.unmanaged_count,
        package_bytes: std::fs::metadata(out).map(|meta| meta.len()).unwrap_or(0),
        drill_passed: passed,
        drill_problems: problems,
    })
}

/// 人话报告。
pub fn render(report: &HandoverReport) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push("交接包生成完成".to_string());
    lines.push(String::new());
    lines.push(format!("  认知库：{}", report.db_path.display()));
    lines.push(format!("  交接包：{}", report.out_path.display()));
    lines.push(format!("  记录条数：{}", report.entries));
    lines.push(format!(
        "  未纳管条数：{}（**没有链字段** ⇒ 账不完整 ⇒ **如实标出**）",
        report.unmanaged_entries
    ));
    lines.push(format!("  红线条数：{}", default_red_lines().len()));
    lines.push(format!(
        "  交接包体积：{} 字节（{:.1} KB）",
        report.package_bytes,
        report.package_bytes as f64 / 1024.0
    ));
    if report.package_bytes > LARGE_PACKAGE_BYTES {
        lines.push(String::new());
        lines.push(format!(
            "  ⚠️ **包偏大**（> {} KB）⇒ ⇒ 里面的 `archive` 是**数据本体**（逐字正文）⇒ ⇒ 可考虑改成「**路径 + 哈希**」⇒ ⇒ **这是产品口径 ⇒ 待 owner 裁决**",
            LARGE_PACKAGE_BYTES / 1024
        ));
    }
    lines.push(String::new());
    if report.drill_passed {
        lines.push("  ✅ **演练通过**（读方能独立读懂这个包）".to_string());
    } else {
        lines.push("  ❌ **演练失败**（读方读不懂这个包）".to_string());
        for problem in &report.drill_problems {
            lines.push(format!("    · {problem}"));
        }
    }
    lines.join("\n")
}

/// 便捷入口（给 `main.rs` 用）⇒ 返回**退出码**（`u8` ⇒ `ExitCode::from`）。
pub fn run(args: &[String]) -> u8 {
    match run_handover(args) {
        Ok(report) => {
            println!("{}", render(&report));
            if report.drill_passed {
                0
            } else {
                1
            }
        }
        Err(HandoverFailure::Usage) => {
            eprintln!("{USAGE}");
            2
        }
        Err(HandoverFailure::Unavailable(message)) => {
            eprintln!("memory handover 无法执行：{message}");
            2
        }
        Err(HandoverFailure::Drill(message)) => {
            eprintln!("memory handover **演练失败**（交接包不可信）：{message}");
            1
        }
    }
}
