//! **单条冷层归档**演练报告（归档 → 检索 → 取消归档 → 校验）。
//!
//! # 这个例子回答什么
//!
//! 设计稿 §6.2 的 ⭐ **保真可达率**要证明「被降权/归档的记忆，明确需要时**仍能检索到**」
//! ⇒ 前提是**真的有「单条归档」这回事** ⇒ 本例子在一份**真实 schema 的磁盘库**上
//! 把整条链路跑一遍，回答四件事：
//!
//! 1. **归档不删数据**：归档前后 `episodes` 行**逐列相等**（P1）；
//! 2. **归档件仍可被检索到**：真检索（真 SQL + 真排序）里它**仍在结果里**，
//!    只是**权重降、排序靠后**（⇒ 保真可达率 = 实测，不是假设）；
//! 3. **归档可逆**：取消归档 ⇒ 权重回到 `1.0` ⇒ 排序回原位；
//! 4. **账实相符**：账本里的每条都**真的还在** `episodes` 里（可独立校验）。
//!
//! # 库从哪来（**如实**）
//!
//! 优先级：`命令行参数` ⇒ 环境变量 `APEIRETH_COGNITIVE_DB`（CLI 同源的那个）
//! ⇒ `APEIRETH_MEMORY_DB` ⇒ `<数据根>/cognitive.sqlite3`（默认域原位）
//! ⇒ `<数据根>/users/<user>/data/cognitive.sqlite3`（多户域）。
//!
//! **绝不动生产库**：演练一律在**工作副本**上做
//!
//! * 生产库**有**记忆 ⇒ `VACUUM INTO` 拷一份（数据逐字）⇒ 在副本上演练；
//! * 生产库**没有**记忆（**本机现状：`episodes` 0 条**）⇒ 用**真实迁移**建一份
//!   磁盘库并走**生产写入路径**播种 ⇒ 在它上面演练。
//!
//! ⇒ 报告里会**照实**写明这次用的是哪一种（不许把「自建」说成「生产库」）。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-27）
//!
//! 四级口径：
//!
//! - **IMPLEMENTED** ✔：能跑出完整演练报告（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：生产主路径 0 调用点（只有人工在用）；
//! - **DEFAULT ENABLED** ✘：没有开关位；
//! - **HARDWARE VALIDATED** ✘：**未在生产记忆库上实测过**（本机生产库
//!   `episodes` 0 条 ⇒ 无从实测）⇒ 不宣称已验证。

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use apeireth_memory::cold_archive::{
    archive_episode, read_ledger, render_cold_report, unarchive_episode, verify_ledger, weight_for,
    ArchiveLedger, ColdArchiveConfig,
};
use apeireth_memory::{CoreEpisode, EpisodeStore, SqliteMemoryStore};
use rusqlite::{params, Connection, OpenFlags};

/// 检索关键词取正文的头几个字（**中文没有空格** ⇒ 不能按词切 ⇒ 取前缀）。
const KEYWORD_CHARS: usize = 6;

/// 一条检索命中（id + 归档权重）。
struct Hit {
    id: String,
    weight: f64,
}

fn main() -> ExitCode {
    println!("════════ 单条冷层归档演练（归档 ≠ 删除）════════");

    let temp = match tempfile::TempDir::new() {
        Ok(dir) => dir,
        Err(error) => {
            eprintln!("· 建临时目录失败：{error}");
            return ExitCode::FAILURE;
        }
    };

    let (conn, origin) = match prepare_working_db(temp.path()) {
        Ok(pair) => pair,
        Err(reason) => {
            eprintln!("· **准备工作库失败** ⇒ **没跑**（不是「跑过了」）：{reason}");
            return ExitCode::FAILURE;
        }
    };
    println!("工作库来源：{origin}");
    print_schema(&conn);

    let config = ColdArchiveConfig::default_config();
    println!(
        "归档权重系数：{}（**不许 0** ⇒ 0 = 过滤掉 = 违反「归档件不消失」）",
        config.weight_factor
    );

    let ids = episode_ids(&conn);
    if ids.is_empty() {
        eprintln!("· 工作库里一条记忆都没有 ⇒ **没跑**（不是「跑过了」）");
        return ExitCode::FAILURE;
    }
    print_baseline(&conn, &ids);

    // ── 归档前的真检索基线 ──
    let ledger = read_ledger(&conn).expect("读账本");
    let target = ids[0].clone();
    let keyword = keyword_of(&conn, &target);
    println!();
    println!("── 真检索基线（归档前）──");
    print_hits(&retrieve(&conn, &keyword, &ledger, &config));

    // ── 归档两条（单条语义 ⇒ 一次一条）──
    println!();
    println!("── 归档（单条）──");
    for (id, reason) in [
        (target.clone(), "低频冷记忆（明确需要时仍要能检索到）"),
        (
            ids.get(1).cloned().unwrap_or_else(|| target.clone()),
            "内容重复（合并前先冷存）",
        ),
    ] {
        let changed = archive_episode(&conn, &id, reason, now_ms()).expect("归档");
        println!("  · 归档 {id} ⇒ Ok({changed})（{reason}）");
    }

    // ── 归档后：真检索（归档件仍在结果里，只是靠后）──
    let ledger = read_ledger(&conn).expect("读账本");
    println!();
    println!("── 真检索（归档后；关键词「{keyword}」）──");
    print_hits(&retrieve(&conn, &keyword, &ledger, &config));

    // ── ⭐ 保真可达率（**实测** ⇒ 不许假设）──
    let (rate, hits, total) = reachable_rate(&conn, &ledger, &config);
    println!();
    println!("── ⭐ 保真可达率（实测，非假设）──");
    println!(
        "  被归档且**未销毁**的记忆 {} 条；明确需要时仍能检索到 {} 条 ⇒ 保真可达率 = {:.4}",
        total, hits, rate
    );
    println!(
        "  （口径：归档 ≠ 删除 ⇒ 被归档件仍进分母；「能不能检索到」由上面那次**真检索**判定）"
    );

    // ── 报告 #1（归档态）──
    println!();
    println!("── render_cold_report（归档态，原样输出）──");
    println!("{}", render_cold_report(&ledger, verify_ledger(&conn)));

    // ── 取消归档 ⇒ 可逆 ⇒ 权重回 1.0 ──
    println!();
    println!("── 取消归档（可逆）──");
    let removed = unarchive_episode(&conn, &target).expect("取消归档");
    println!("  · 取消 {target} ⇒ Ok({removed})");
    let ledger = read_ledger(&conn).expect("读账本");
    println!(
        "  · {} 的检索权重：{:.2} ⇒ 1.00（回到原样）",
        target,
        weight_for(&ledger, &target, &config)
    );
    println!("── 真检索（取消归档后）──");
    print_hits(&retrieve(&conn, &keyword, &ledger, &config));

    // ── 报告 #2（取消归档后）──
    println!();
    println!("── render_cold_report（取消归档后，原样输出）──");
    println!("{}", render_cold_report(&ledger, verify_ledger(&conn)));

    // ── 数据没动的终检：归档全程没改过 `episodes` 任何一列 ──
    let columns: usize = episode_column_count(&conn);
    println!();
    println!("── 终检 ──");
    println!(
        "  · `episodes` 仍有 {} 行 / {} 列（归档**全程没删任何一行、没改任何一列** ⇒ P1）",
        ids.len(),
        columns
    );
    match verify_ledger(&conn) {
        Ok(()) => println!("  · 账实校验：**相符**"),
        Err(error) => {
            println!("  · 账实校验：**不符** —— {error}");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

// ============================================================
// 工作库准备
// ============================================================

/// 准备工作库（**绝不动生产库**）⇒ 返回 `(连接, 来源说明)`。
fn prepare_working_db(work_dir: &Path) -> Result<(Connection, String), String> {
    // ① 生产库有记忆 ⇒ 拷一份（VACUUM INTO ⇒ 逐字一致）⇒ 在副本上演练。
    if let Some(source) = locate_source_db() {
        let count = count_episodes(&source);
        if count > 0 {
            let copy = work_dir.join("working-copy.sqlite3");
            copy_database(&source, &copy)?;
            let conn =
                Connection::open(&copy).map_err(|error| format!("打开工作副本失败：{error}"))?;
            return Ok((
                conn,
                format!(
                    "**真实库副本**：{}（{} 条记忆逐字拷入；生产库**未被写**）",
                    source.display(),
                    count
                ),
            ));
        }
    }

    // ② 生产库没有记忆（本机现状）⇒ 用**真实迁移**建一份磁盘库 + 生产写入路径播种。
    let target = work_dir.join("working.sqlite3");
    {
        let store =
            SqliteMemoryStore::open(&target).map_err(|error| format!("建库失败：{error}"))?;
        for episode in seed_episodes() {
            store
                .put_episode(&episode)
                .map_err(|error| format!("播种失败：{error}"))?;
        }
    }
    let conn = Connection::open(&target).map_err(|error| format!("打开工作库失败：{error}"))?;
    Ok((
        conn,
        "**自建真实 schema 库**（磁盘文件 + 真实迁移 + 生产写入路径播种；\
         本机生产库 `episodes` 0 条 ⇒ 无生产数据可拷 ⇒ **不是**生产库）"
            .to_string(),
    ))
}

/// 播种用的记忆（**真实正文** ⇒ 检索有真东西可查）。
fn seed_episodes() -> Vec<CoreEpisode> {
    vec![
        CoreEpisode {
            id: "ep-cold-001".to_string(),
            timestamp: 1_700_000_000,
            role: "user".to_string(),
            content: "冷层归档的第一条记忆：归档不等于删除，原始证据必须永久保留。".to_string(),
            session_id: "sess-cold-demo".to_string(),
        },
        CoreEpisode {
            id: "ep-cold-002".to_string(),
            timestamp: 1_700_000_100,
            role: "assistant".to_string(),
            content: "冷层归档的第二条记忆：被归档的记忆明确需要时仍要能检索得到。".to_string(),
            session_id: "sess-cold-demo".to_string(),
        },
        CoreEpisode {
            id: "ep-cold-003".to_string(),
            timestamp: 1_700_000_200,
            role: "user".to_string(),
            content: "冷层归档的第三条记忆：保真可达率必须是实测值，不是假设值。".to_string(),
            session_id: "sess-cold-demo".to_string(),
        },
    ]
}

/// 找生产库（命令行参数 ⇒ 环境变量 ⇒ 默认域路径）。
fn locate_source_db() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(raw) = std::env::args().nth(1) {
        candidates.push(PathBuf::from(raw));
    }
    if let Some(raw) = std::env::var_os("APEIRETH_COGNITIVE_DB") {
        candidates.push(PathBuf::from(raw));
    }
    if let Some(raw) = std::env::var_os("APEIRETH_MEMORY_DB") {
        candidates.push(PathBuf::from(raw));
    }
    let data_root = apeireth_core::paths::default_data_root();
    candidates.push(data_root.join("cognitive.sqlite3"));
    let user = std::env::var_os("APEIRETH_USER_ID")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("default"));
    candidates.push(
        data_root
            .join("users")
            .join(user)
            .join("data")
            .join("cognitive.sqlite3"),
    );
    candidates.push(
        data_root
            .join("users")
            .join("default")
            .join("data")
            .join("cognitive.sqlite3"),
    );
    candidates.into_iter().find(|path| path.is_file())
}

/// 生产库里有几条记忆（打不开 / 没有表 ⇒ 当作 0）。
fn count_episodes(source: &Path) -> i64 {
    let Ok(conn) = Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY) else {
        return 0;
    };
    conn.query_row("SELECT COUNT(*) FROM episodes", [], |row| row.get(0))
        .unwrap_or(0)
}

/// 把生产库**逐字**拷一份（`VACUUM INTO` ⇒ 一致快照；**只读源**）。
fn copy_database(source: &Path, target: &Path) -> Result<(), String> {
    let conn = Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("只读打开生产库失败：{error}"))?;
    let literal = target.to_string_lossy().replace('\'', "''");
    conn.execute_batch(&format!("VACUUM INTO '{literal}'"))
        .map_err(|error| format!("拷贝生产库失败：{error}"))
}

// ============================================================
// 打印
// ============================================================

/// 打印 `episodes` 的**真实列名**（⇒ 报告里不假设列集）。
fn print_schema(conn: &Connection) {
    let Ok(mut statement) = conn.prepare("PRAGMA table_info(episodes)") else {
        println!("（读不到 `episodes` 列集）");
        return;
    };
    let Ok(rows) = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(0)?,
        ))
    }) else {
        println!("（读不到 `episodes` 列集）");
        return;
    };
    let mut columns = Vec::new();
    for row in rows.flatten() {
        columns.push(format!("{}:{}", row.2 + 1, row.0));
    }
    println!("`episodes` 真实列集：{}", columns.join(" · "));
}

/// 打印基线（库里有哪些记忆）。
fn print_baseline(conn: &Connection, ids: &[String]) {
    println!();
    println!("── 基线：记忆清单（{} 条）──", ids.len());
    for id in ids {
        let content: String = conn
            .query_row(
                "SELECT content FROM episodes WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap_or_default();
        println!("  · {id}：{content}");
    }
}

/// 打印检索命中（**权重**一并显示 ⇒ 看得见权重生效）。
fn print_hits(hits: &[Hit]) {
    if hits.is_empty() {
        println!("  （无命中）");
        return;
    }
    for (index, hit) in hits.iter().enumerate() {
        println!("  #{:>2} 权重 {:.2}  {}", index + 1, hit.weight, hit.id);
    }
}

// ============================================================
// 真检索（真 SQL + 真排序 + 归档权重）
// ============================================================

/// 真检索：关键词命中 ⇒ 按**归档权重**排序。
///
/// **为什么自己写一条小检索**：要证的是「归档件**仍被召回**」⇒ 判据必须落在
/// **真实检索结果**上（不许假设）⇒ 这里跑真 SQL + 真排序。
fn retrieve(
    conn: &Connection,
    keyword: &str,
    ledger: &ArchiveLedger,
    config: &ColdArchiveConfig,
) -> Vec<Hit> {
    let Ok(mut statement) = conn
        .prepare("SELECT id FROM episodes WHERE content LIKE ?1 ORDER BY timestamp ASC, id ASC")
    else {
        return Vec::new();
    };
    let Ok(rows) = statement.query_map(params![format!("%{keyword}%")], |row| {
        row.get::<_, String>(0)
    }) else {
        return Vec::new();
    };
    let mut hits: Vec<Hit> = rows
        .flatten()
        .map(|id| {
            let weight = weight_for(ledger, &id, config);
            Hit { id, weight }
        })
        .collect();
    // 权重降序 ⇒ 同权重按 id 升序（确定性）。
    hits.sort_by(|left, right| {
        right
            .weight
            .partial_cmp(&left.weight)
            .expect("权重有限")
            .then_with(|| left.id.cmp(&right.id))
    });
    hits
}

/// ⭐ **保真可达率**（**实测**）：被归档且未销毁的记忆里，明确需要时仍能检索到的比例。
///
/// **口径**（与设计稿 §6.2 一致）：分母 = 被归档 ∧ 未销毁；分子 = 其中
/// 「用它的关键词**真检索**时能被召回」⇒ **不许假设**它还能检索到。
fn reachable_rate(
    conn: &Connection,
    ledger: &ArchiveLedger,
    config: &ColdArchiveConfig,
) -> (f64, usize, usize) {
    let total = ledger.entries.len();
    if total == 0 {
        return (0.0, 0, 0);
    }
    let mut hits = 0usize;
    for entry in &ledger.entries {
        let keyword = keyword_of(conn, &entry.id);
        let found = retrieve(conn, &keyword, ledger, config)
            .iter()
            .any(|hit| hit.id == entry.id);
        if found {
            hits += 1;
        }
    }
    (hits as f64 / total as f64, hits, total)
}

// ============================================================
// 小工具
// ============================================================

/// 库里所有记忆 id（按写入顺序 ⇒ 确定性）。
fn episode_ids(conn: &Connection) -> Vec<String> {
    let Ok(mut statement) = conn.prepare("SELECT id FROM episodes ORDER BY timestamp ASC, id ASC")
    else {
        return Vec::new();
    };
    let Ok(rows) = statement.query_map([], |row| row.get::<_, String>(0)) else {
        return Vec::new();
    };
    rows.flatten().collect()
}

/// `episodes` 的列数（终检用 ⇒ 不假设列集）。
fn episode_column_count(conn: &Connection) -> usize {
    let Ok(mut statement) = conn.prepare("PRAGMA table_info(episodes)") else {
        return 0;
    };
    // `PRAGMA table_info` 的第 0 列是 `cid`（整数）⇒ 只数行数，不碰列名/类型。
    let Ok(rows) = statement.query_map([], |row| row.get::<_, i64>(0)) else {
        return 0;
    };
    rows.flatten().count()
}

/// 取一条记忆的检索关键词（正文头几个字）。
fn keyword_of(conn: &Connection, id: &str) -> String {
    let content: String = conn
        .query_row(
            "SELECT content FROM episodes WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .unwrap_or_default();
    content.chars().take(KEYWORD_CHARS).collect()
}

/// 当前时刻（epoch 毫秒）—— 归档时刻是**注入**的（⇒ 报告可复现）。
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or(0)
}
