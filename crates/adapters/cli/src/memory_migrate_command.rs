//! `apeireth memory migrate-subject` —— 主体锚点**迁移**命令（copy-forward）。
//!
//! # 这个命令做什么
//!
//! 把某个主体（`session_id` / 锚点）下的记忆**复制**到另一个主体锚点下：
//! * **原记录保留**（append-only ⇒ 不删、不改）；
//! * 副本 id = `mig-{原 id}` ⇒ **幂等**（同 id 已在库 ⇒ 跳过，**不重复入链、不推进链头**）；
//! * 上下文账本（`context_ledger_messages`）的键**重新指向**新锚点；
//! * ⚠️ **副本带链字段**（`chain_hash` / `chain_prev_hash`）⇒ 副本**入新主体的链**
//!   ⇒ 体检的「未纳管」不会因迁移而增加。
//!
//! # 为什么需要它
//!
//! 上一批（B1-e）把 `continuity_link::migrate_subject` 收编进哈希链体系，
//! 但它**库级已实现、生产零调用点** ⇒ 体检报告里的「未纳管」要真下降，
//! 需要一个**运维入口**。本命令就是那个入口。
//!
//! # 口径
//!
//! * **copy-forward**（不是 move）：原记录仍在库中；
//! * **幂等**：可重复执行 ⇒ 第二次全部 `skipped`；
//! * **失败即失败**：链写不进去 ⇒ **整体报错**、**不落无链副本**（不留半截状态）；
//! * **不可逆**：本命令**没有**回滚（副本是 append-only 的）
//!   ⇒ 执行前请确认 `--from` / `--to` 没写反。
//!
//! # 退出码
//!
//! * **0** = 迁移完成（`copied` / `skipped` 如实打印）；
//! * **2** = 参数错 / 库打不开 / 迁移失败。

use std::path::{Path, PathBuf};

/// 命令用法（与其它命令的 `USAGE` 风格一致）。
pub const USAGE: &str = "apeireth memory migrate-subject --from <ANCHOR> --to <ANCHOR> [--db <COGNITIVE_DB>]\n\n  主体锚点迁移（copy-forward）：把 --from 下的记忆复制到 --to，原记录保留。\n\n  --from  源锚点（源 session_id）\n  --to    目标锚点（新的主体）\n  --db    认知库路径（缺省 = APEIRETH_COGNITIVE_DB 或数据目录下 cognitive.sqlite3）\n\n  退出码：0 = 迁移完成 · 2 = 参数错 / 库打不开 / 迁移失败";

/// 迁移结果（供 CLI 渲染；也便于测试断言）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrateReport {
    /// 被迁移的库路径。
    pub db_path: PathBuf,
    /// 源锚点。
    pub from: String,
    /// 目标锚点。
    pub to: String,
    /// 复制出的副本数（**每条副本都带链字段** ⇒ 入新主体的链）。
    pub episodes_copied: usize,
    /// 已存在而跳过的数（**幂等**：第二次执行应当全是这个）。
    pub episodes_skipped: usize,
    /// 上下文账本重新指向的行数。
    pub ledger_rekeyed: usize,
}

/// 执行命令。
///
/// # 返回
///
/// * `Ok(0)` = 迁移完成；
/// * `Ok(2)` = **无法迁移**（参数错 / 库打不开 / 迁移失败）—— 已把原因打印到 stderr。
pub fn run_migrate(args: &[String]) -> u8 {
    let mut from: Option<String> = None;
    let mut to: Option<String> = None;
    let mut db: Option<PathBuf> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--from" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("--from 缺参数");
                    eprintln!("{USAGE}");
                    return 2;
                };
                from = Some(value.clone());
                index += 2;
            }
            "--to" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("--to 缺参数");
                    eprintln!("{USAGE}");
                    return 2;
                };
                to = Some(value.clone());
                index += 2;
            }
            "--db" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("--db 缺参数");
                    eprintln!("{USAGE}");
                    return 2;
                };
                db = Some(PathBuf::from(value));
                index += 2;
            }
            other => {
                eprintln!("未知参数：{other}");
                eprintln!("{USAGE}");
                return 2;
            }
        }
    }

    let (Some(from), Some(to)) = (from, to) else {
        eprintln!("--from 与 --to 都是必填");
        eprintln!("{USAGE}");
        return 2;
    };
    if from.trim().is_empty() || to.trim().is_empty() {
        eprintln!("锚点不能为空");
        return 2;
    }
    if from.trim() == to.trim() {
        eprintln!("源锚点与目标锚点相同，无需迁移");
        return 2;
    }

    let db_path = match db {
        Some(path) => path,
        None => match default_db_path() {
            Some(path) => path,
            None => {
                eprintln!("找不到认知库路径（用 --db 指定）");
                return 2;
            }
        },
    };
    if !db_path.exists() {
        eprintln!("认知库不存在：{}", db_path.display());
        return 2;
    }

    let store = match apeireth_memory::SqliteMemoryStore::open(&db_path) {
        Ok(store) => store,
        Err(error) => {
            eprintln!("打开认知库失败：{error}");
            return 2;
        }
    };

    // ⚠️ 执行时刻用**当前时间**（毫秒）⇒ 写进迁移报告 ⇒ 事后可查「什么时候迁的」。
    let executed_at = now_ms();

    // ⭐⭐ **主体登记**（P1 接线 · 2026-11-07）：**这是本命令为什么能兑现
    // 「`continuity_id` 的物理唯一性」的地方**。
    //
    // # 为什么这里是**真消费点**（不是「凑数」）
    //
    // 本命令是**生产面唯一**会把记录挂到**新主体**（`--to`）名下的入口
    // ⇒ ⇒ 迁移产出的副本带 `continuity_id = to`
    // ⇒ ⇒ 而 `continuity_id` 的**物理唯一性**靠 `identity_cards` 表的 UNIQUE 约束
    //    （`migrations.rs` 的 `identity_cards` DDL）守护
    // ⇒ ⇒ **主体没在 `identity_cards` 登记 ⇒ 约束永远不生效 ⇒ 表永远是空的**
    // ⇒ ⇒ 迁移完的主体是个「黑户」：载体迁移（`record_carrier_migration`）会
    //    直接报 `NotFound`（那张卡不存在）。
    //
    // # 为什么**不担心**重复登记
    //
    // `continuity_link::ensure_identity` **本来就是幂等的**：先 `exists` 再 `create`，
    // 且把 `IdentityConflict::AlreadyExists` **显式**映射成「同一人已登记 ⇒ 成功」
    // ⇒ ⇒ 每次都调安全 ⇒ ⇒ **不吞错误**（别的错误照原样报错）。
    //
    // # 登记**两个**锚点
    //
    // * `from`（源主体）—— 它的记录本来就在库里 ⇒ 它也该在册；
    // * `to`（目标主体）—— 迁移会让它名下第一次有记录 ⇒ 它最该在册。
    if let Err(error) = register_subjects(&store, &from, &to, &db_path) {
        eprintln!("主体登记失败：{error}");
        return 2;
    }
    // ⚠️ **可观测**：登记做了什么 ⇒ 打印出来（不许「静默写库」）。
    println!("主体已登记：{from} / {to}（identity_cards 在册 ⇒ continuity_id 物理唯一性生效）");

    match apeireth_memory::continuity_link::migrate_subject(&store, &from, &to, executed_at) {
        Ok(report) => {
            let rendered = MigrateReport {
                db_path,
                from: report.from.clone(),
                to: report.to.clone(),
                episodes_copied: report.episodes_copied,
                episodes_skipped: report.episodes_skipped,
                ledger_rekeyed: report.ledger_rekeyed,
            };
            println!("{}", render_report(&rendered));
            0
        }
        Err(error) => {
            // ⚠️ **失败即失败**：不落无链副本（同一事务回滚）⇒ 这里只如实报错。
            eprintln!("迁移失败：{error}");
            2
        }
    }
}

/// ⭐ **主体登记**（`identity_cards` 在册 ⇒ `continuity_id` 的物理唯一性才生效）。
///
/// # 口径
///
/// * **幂等**：`ensure_identity` 内部先查 `exists` ⇒ 重复调用安全；
/// * **载体（carrier）** = 认知库的**文件名主干**（如 `cognitive`）
///   ⇒ ⇒ 主体「在哪个载体上」有据可查（不是硬编码的假字符串）；
/// * **诞生时间** = 现在（秒）⇒ ⇒ 首次登记的时刻有据可查；
/// * ⚠️ **失败即失败**：登记失败 ⇒ **不迁移**（不产出「有记录但没在册」的主体）。
fn register_subjects(
    store: &apeireth_memory::SqliteMemoryStore,
    from: &str,
    to: &str,
    db_path: &Path,
) -> Result<(), String> {
    let carrier = db_path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .filter(|stem| !stem.trim().is_empty())
        .unwrap_or_else(|| "cognitive".to_string());
    let birth_time = now_ms() / 1000;
    for anchor in [from, to] {
        let anchor = anchor.trim();
        if anchor.is_empty() {
            return Err("锚点为空，无法登记主体".to_string());
        }
        apeireth_memory::continuity_link::ensure_identity(store, anchor, &carrier, birth_time)
            .map_err(|error| format!("登记主体 `{anchor}` 失败：{error}"))?;
    }
    Ok(())
}

/// 人话报告。
pub fn render_report(report: &MigrateReport) -> String {
    format!(
        "主体锚点迁移完成\n\
         \x20 库：{}\n\
         \x20 源锚点：{}\n\
         \x20 目标锚点：{}\n\
         \x20 复制副本：{} 条（**每条都入新主体的链**）\n\
         \x20 跳过已存在：{} 条\n\
         \x20 账本改键：{} 行\n\
         \n\
         ⚠️ **copy-forward**：原记录仍在库中；副本 id = `mig-{{原 id}}` ⇒ **幂等**（可重复执行）。",
        report.db_path.display(),
        report.from,
        report.to,
        report.episodes_copied,
        report.episodes_skipped,
        report.ledger_rekeyed,
    )
}

// ============================================================
// `apeireth memory rollback-drill` —— 迁移回滚**演练**（可演练 ⇒ 才谈得上「可回滚」）
// ============================================================

/// 回滚演练命令的用法。
///
/// ⚠️ **`--db` 是必填**（没有缺省）⇒ ⇒ 逼着操作者**指名**要演练哪个库
/// ⇒ ⇒ 不给「手一抖把生产库当演练库」的口子。
pub const ROLLBACK_DRILL_USAGE: &str = "apeireth memory rollback-drill --from <ANCHOR> --to <ANCHOR> --db <COGNITIVE_DB>\n\n  迁移回滚演练：建账 → 真迁移 → 核对 → 回滚 → 再核对「回到迁移前」。\n\n  --from  源锚点（源 session_id；该锚点下必须有记录，否则无可演练之物）\n  --to    目标锚点（演练用的新主体）\n  --db    认知库路径（**必填** ⇒ 不提供「默认打生产库」的口子）\n\n  退出码：0 = 演练通过（已证明回到迁移前） · 2 = 参数错 / 库打不开 / 演练未通过";

/// `apeireth memory` 全部子面的**总用法文本**（各面 `USAGE` 拼装）。
///
/// # 为什么放在这里（而不是 `main.rs`）
///
/// `main.rs` 是被 `scripts/arch-baseline.txt` 登记尺寸的文件（只许不变或变小）
/// ⇒ ⇒ 把「打印 5 段用法」这种纯文本装配摊在主分派里会**净增行数**
/// ⇒ ⇒ 而它属于 `memory` 命令族的知识 ⇒ ⇒ 归位到本模块（`main.rs` 只留一行调用）。
pub fn memory_usage_text() -> String {
    use crate::{
        memory_export_command as export, memory_handover_command as handover,
        memory_verify_command as verify,
    };
    [
        verify::USAGE,
        export::USAGE,
        handover::USAGE,
        USAGE,
        ROLLBACK_DRILL_USAGE,
    ]
    .join("\n\n")
}

/// 执行回滚演练命令。
///
/// # 口径（**必须如实说清**）
///
/// * 演练走**真实入口** `continuity_link::migrate_subject`（同库第二个连接）
///   ⇒ 造出真的 `mig-` 副本 ⇒ 核对 ⇒ **回滚删副本** ⇒ 再核对内容哈希；
/// * **原记录一条都不删**（P1 底线，库内强制）；
/// * ⚠️ **`context_ledger` 的改键副作用不回滚**（库级已知边界 ⇒ 本命令在报告里如实标注）；
/// * ⚠️ **演练会改库**（临时摘 append-only 触发器 ⇒ 事务内复原）⇒ ⇒ **只对测试库跑**。
///
/// # 返回
///
/// * `Ok(0)` = 演练通过；
/// * `Ok(2)` = 参数错 / 库打不开 / 演练未通过（原因已打到 stderr）。
pub fn run_rollback_drill(args: &[String]) -> u8 {
    let mut from: Option<String> = None;
    let mut to: Option<String> = None;
    let mut db: Option<PathBuf> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--from" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("--from 缺参数");
                    eprintln!("{ROLLBACK_DRILL_USAGE}");
                    return 2;
                };
                from = Some(value.clone());
                index += 2;
            }
            "--to" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("--to 缺参数");
                    eprintln!("{ROLLBACK_DRILL_USAGE}");
                    return 2;
                };
                to = Some(value.clone());
                index += 2;
            }
            "--db" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("--db 缺参数");
                    eprintln!("{ROLLBACK_DRILL_USAGE}");
                    return 2;
                };
                db = Some(PathBuf::from(value));
                index += 2;
            }
            other => {
                eprintln!("未知参数：{other}");
                eprintln!("{ROLLBACK_DRILL_USAGE}");
                return 2;
            }
        }
    }

    let (Some(from), Some(to)) = (from, to) else {
        eprintln!("--from 与 --to 都是必填");
        eprintln!("{ROLLBACK_DRILL_USAGE}");
        return 2;
    };
    if from.trim().is_empty() || to.trim().is_empty() {
        eprintln!("锚点不能为空");
        return 2;
    }
    if from.trim() == to.trim() {
        eprintln!("源锚点与目标锚点相同，无需演练");
        return 2;
    }
    // ⚠️ `--db` **必填**（与 `migrate-subject` 不同）⇒ 演练**必须**指名一个库。
    let Some(db_path) = db else {
        eprintln!("--db 是必填项（演练会改库 ⇒ 不给「默认打生产库」的口子）");
        eprintln!("{ROLLBACK_DRILL_USAGE}");
        return 2;
    };
    if !db_path.exists() {
        eprintln!("认知库不存在：{}", db_path.display());
        return 2;
    }

    let store = match apeireth_memory::SqliteMemoryStore::open(&db_path) {
        Ok(store) => store,
        Err(error) => {
            eprintln!("打开认知库失败：{error}");
            return 2;
        }
    };

    // 演练要一个**可写连接**（核对 / 回滚 / 导出都走它）。
    // ⚠️ `SqliteMemoryStore::conn()` 拿的是 Mutex 锁 ⇒ 这里**必须**在同一个
    // 作用域里用完就放（否则第二次开同库连接会撞锁）。
    let executed_at = now_ms();
    let outcome = {
        let mut guard = match store.conn() {
            Ok(guard) => guard,
            Err(error) => {
                eprintln!("拿连接失败：{error}");
                return 2;
            }
        };
        apeireth_memory::migrate_rollback::drill_rollback(&mut *guard, &from, &to, executed_at)
    };

    match outcome {
        Ok(report) => {
            println!("回滚演练完成（库：{}）", db_path.display());
            println!("{report}");
            0
        }
        Err(error) => {
            // ⚠️ **失败即失败**：演练没过就是没过 ⇒ 如实报错、退出码 2。
            eprintln!("回滚演练未通过：{error}");
            2
        }
    }
}

/// 缺省认知库路径（与 `memory verify` 同一口径）。
fn default_db_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("APEIRETH_COGNITIVE_DB") {
        if !path.trim().is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    let root = apeireth_core::paths::default_data_root();
    Some(root.join("cognitive.sqlite3"))
}

/// 当前时刻（毫秒）。
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_both_anchors_is_rejected() {
        assert_eq!(run_migrate(&[]), 2, "缺参数 ⇒ 退出码 2");
        assert_eq!(run_migrate(&["--from".to_string(), "a".to_string()]), 2);
    }

    #[test]
    fn identical_anchors_are_rejected() {
        let args = vec![
            "--from".to_string(),
            "x".to_string(),
            "--to".to_string(),
            "x".to_string(),
        ];
        assert_eq!(run_migrate(&args), 2, "同锚点 ⇒ 无需迁移 ⇒ 退出码 2");
    }

    #[test]
    fn unknown_argument_is_rejected() {
        let args = vec!["--nope".to_string()];
        assert_eq!(run_migrate(&args), 2, "未知参数 ⇒ 退出码 2");
    }

    #[test]
    fn missing_db_file_is_reported_not_panicked() {
        let args = vec![
            "--from".to_string(),
            "a".to_string(),
            "--to".to_string(),
            "b".to_string(),
            "--db".to_string(),
            "/nonexistent/definitely-not-here.sqlite3".to_string(),
        ];
        assert_eq!(
            run_migrate(&args),
            2,
            "库不存在 ⇒ 退出码 2（**不许 panic**）"
        );
    }

    #[test]
    fn report_renders_all_facts() {
        let report = MigrateReport {
            db_path: PathBuf::from("/tmp/x.sqlite3"),
            from: "old".to_string(),
            to: "new".to_string(),
            episodes_copied: 3,
            episodes_skipped: 1,
            ledger_rekeyed: 2,
        };
        let text = render_report(&report);
        for needle in ["old", "new", "3", "1", "2", "x.sqlite3", "mig-"] {
            assert!(text.contains(needle), "报告里应含 {needle}：\n{text}");
        }
    }

    /// **端到端**：造真库 → 走 CLI → 副本**入新主体的链** → 原记录仍在（copy-forward）。
    #[test]
    fn cli_migrates_episodes_into_the_new_subject_chain() {
        use apeireth_core::kernel::memory::Episode;
        use apeireth_memory::{EpisodeStore, SqliteMemoryStore};

        let dir = tempfile::tempdir().expect("temp dir");
        let db_path = dir.path().join("cognitive.sqlite3");
        let store = SqliteMemoryStore::open(&db_path).expect("open store");
        for index in 0..2 {
            store
                .put_episode(&Episode {
                    id: format!("ep-{index}"),
                    timestamp: 100 + index,
                    role: "user".to_string(),
                    content: format!("内容 {index}"),
                    session_id: "old-anchor".to_string(),
                })
                .expect("seed");
        }
        drop(store);

        // 走 **CLI 入口**（不是直接调库函数）⇒ 覆盖参数解析 + 打开 + 执行。
        let args = vec![
            "--from".to_string(),
            "old-anchor".to_string(),
            "--to".to_string(),
            "new-anchor".to_string(),
            "--db".to_string(),
            db_path.display().to_string(),
        ];
        assert_eq!(run_migrate(&args), 0, "迁移应成功");

        // 副本**带链**且落在**新主体**的链上；原记录**仍在**（copy-forward）。
        //
        // ⚠️ **连接用完立刻释放**：`store.conn()` 拿的是 Mutex 锁 + SQLite 文件锁
        // ⇒ 若**持有它**的时候再跑一次迁移（同库要写）⇒ **死锁**
        // （实测：卡到测试超时 360s）⇒ 每次查询开**独立作用域**，查完即 drop。
        {
            let store = SqliteMemoryStore::open(&db_path).expect("reopen");
            let conn = store.conn().expect("conn");
            let new_chain: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM episodes WHERE continuity_id = 'new-anchor' AND chain_hash IS NOT NULL",
                    [],
                    |row| row.get(0),
                )
                .expect("count new chain");
            assert_eq!(new_chain, 2, "两条副本都应带链入新主体");
            let originals: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM episodes WHERE id IN ('ep-0','ep-1')",
                    [],
                    |row| row.get(0),
                )
                .expect("count originals");
            assert_eq!(
                originals, 2,
                "**原记录必须保留**（copy-forward，不是 move）"
            );
        }

        // **幂等**：再跑一次 ⇒ 全部跳过、链头**不推进**。
        assert_eq!(run_migrate(&args), 0, "第二次应成功");
        {
            let store = SqliteMemoryStore::open(&db_path).expect("reopen again");
            let conn = store.conn().expect("conn");
            let chained: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM episodes WHERE continuity_id = 'new-anchor' AND chain_hash IS NOT NULL",
                    [],
                    |row| row.get(0),
                )
                .expect("count");
            assert_eq!(chained, 2, "幂等：重跑**不新增**副本");
        }
    }
}
