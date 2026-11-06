//! `apeireth-memory::migrate_rollback` — 迁移的**回滚**（可演练、机器可证）。
//!
//! # 为什么
//!
//! `continuity_link::migrate_subject` 是 **copy-forward**（原记录保留、复制副本）
//! ⇒ 数据不丢 ⇒ 但**副本留在库里**（`mig-{原 id}`）⇒ 主体里多了一份
//! ⇒ 若迁移**做错了**（迁错锚点、迁错内容），需要能**退回去**。
//!
//! # 三条纪律
//!
//! 1. **只删副本，不碰原记录**（原记录是 P1 的底线 ⇒ **永不可删**）；
//! 2. **有账可查**：回滚前先**核对**（数量、id、链字段）⇒ 与**迁移前快照**比对；
//! 3. **失败即失败**：任一步不符 ⇒ **不删任何东西**（宁可回滚失败，也不要删错）。
//!
//! # 四级口径（钉死，别自创）
//!
//! * **IMPLEMENTED** ✔：账 / 核对 / 回滚 / 演练四件齐备（库级实现，仅此一级）；
//! * **PRODUCTION WIRED** ✘：**未接入**任何生产路径（无 CLI、无定时任务、生产 0 调用点）；
//! * **DEFAULT ENABLED** ✘：没有开关位，也未默认启用；
//! * **HARDWARE VALIDATED** ✘：只在库级测试与**真实 schema** 造库上跑过，
//!   **未在生产记忆库上实测**（本机无生产库）⇒ 不宣称已验证。
//!
//! 对外一律按 **NOT IMPLEMENTED** 表述（新增未接线，不宣称可用）。
//!
//! # B4 的位置（别越界宣称）
//!
//! 设计稿 §8 **B4** = 脱机归档（保原文）+ **迁移回滚** + handover 演练。
//! 本模块做的是 B4 的**第二块砖**（迁移回滚）；另两块砖
//! （[`crate::offline_archive`] 脱机可读 / [`crate::handover`] handover 演练）
//! 由各自的模块承担 ⇒ **三块砖齐了才谈得上 B4 完成**。
//!
//! # 「源锚点」到底指什么（**口径钉死**）
//!
//! `continuity_link::migrate_subject` 是按 **会话锚点（`session_id`）**取源的
//! （`EpisodeQuery::new().for_session(&from)`）⇒ 本模块的「源锚点」**同口径**
//! ⇒ 账里的 `source_ids` = 迁移**当时**会被复制走的那批记录的 id（**逐字对齐**
//! `migrate_subject` 的取源条件 ⇒ 建账的「承诺」才可能兑现）。
//!
//! # 回滚怎么删（**为什么要动 append-only 触发器**）
//!
//! `episodes` 是 append-only 表（V1 建了 `episodes_no_delete` 触发器 ⇒ `DELETE` 直接
//! `RAISE(ABORT)`）⇒ **回滚必须先把该触发器摘掉，事务内删完再复原**：
//!
//! * 摘触发器 / 删副本 / 复原触发器 / 复原链头 **全在同一个事务里**
//!   ⇒ 任一步出错 ⇒ 事务整体回滚 ⇒ **触发器也跟着回来**（SQLite 的 DDL 是事务性的）
//!   ⇒ 库不会停在「没有 append-only 保护」的中间态；
//! * 删的行**只有账里的 `mig-` 副本**（三重保险：账内 + `mig-` 前缀 + 不是原记录）；
//! * 原记录（`source_ids`）**一条都不删**（P1 底线 ⇒ 代码里显式拒删）。
//!
//! # 已知边界（**如实说，别当没这回事**）
//!
//! * **`context_ledger` 的重键副作用不回滚**：`migrate_subject` 会把
//!   `context_ledger_messages` 里 `from` 的行改键到 `to`（那是**改键**，不是复制）。
//!   本模块**不**擅自把它改回去 —— 改键回去会把「本来就是 `to` 的行」也搬走
//!   ⇒ 宁可少做，也不搬错 ⇒ 该副作用在演练报告里**显式标注**。
//!   它不影响 `episodes` 的内容哈希 ⇒ 不影响「回到原点」的判定。
//! * **演练需要一个能重开的库**：`drill_rollback` 要在**真实**的
//!   `continuity_link::migrate_subject` 上跑迁移 ⇒ 那需要**第二个连接**指向同一个库
//!   ⇒ 纯内存库（`open_in_memory`）没有文件路径 ⇒ 退化到「同写路径等价实现」
//!   （同查询、同副本 id 规则、同入链函数，但**不是** `migrate_subject` 本体）
//!   ⇒ 报告里**如实标注**走的哪条路。文件库（含具名共享内存库）走**真实入口**。
//! * **演练只对测试库跑**：本模块**不**提供「对生产库跑演练」的开关
//!   （没有开关位 ⇒ 调用方自己负责别指错库）。

use std::collections::BTreeMap;

use apeireth_core::kernel::memory::Episode;
use rusqlite::{params, Transaction};

use crate::continuity_link::MIGRATED_ID_PREFIX;
use crate::memory_chain::{put_chained_episode, ChainEntry};
use crate::offline_archive::{export_archive, verify_archive, ExportConfig, OfflineArchive};
use crate::SqliteMemoryStore;

/// `episodes` 的 append-only 删除触发器名（V1 建；回滚时临时摘掉、事务内复原）。
const EPISODES_NO_DELETE_TRIGGER: &str = "episodes_no_delete";

/// 复原用的触发器 SQL（与 `migrations.rs` V1 **逐字一致** ⇒ 复原后与新建库无差别）。
const EPISODES_NO_DELETE_SQL: &str = "CREATE TRIGGER IF NOT EXISTS episodes_no_delete
BEFORE DELETE ON episodes
BEGIN
    SELECT RAISE(ABORT, 'episodes is append-only: tombstone via reflection_stream instead');
END;";

/// 一次迁移的**账**。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationLedger {
    /// 源锚点。
    pub from: String,
    /// 目标锚点。
    pub to: String,
    /// 迁移时刻（epoch 毫秒；**注入** ⇒ 确定性）。
    pub executed_at_ms: i64,
    /// 复制出的副本 id（**有序** ⇒ 确定性）。
    pub copied_ids: Vec<String>,
}

/// 一份**迁移前快照**（回滚时用来核对）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreMigrationSnapshot {
    /// 迁移前的归档物（**只读**导出 ⇒ 用来证明「原件还在、内容没变」）。
    pub before: OfflineArchive,
    /// 迁移前源锚点下的记录 id（**有序**）。
    pub source_ids: Vec<String>,
}

/// 回滚的**判定**（**四档** ⇒ 不许含糊）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollbackVerdict {
    /// **可以回滚**：账实相符 ⇒ 能安全退。
    Safe,
    /// **账实不符**：副本数量/内容与账不符 ⇒ **不许删**（**如实报差在哪**）。
    Mismatch,
    /// **没有可回滚的**（没迁移过 / 账为空）。
    NothingToRollback,
    /// **原记录不在了** ⇒ **绝不许回滚**（P1 底线 ⇒ 宁可不动）。
    SourceMissing,
}

impl RollbackVerdict {
    /// 判定的**中文名**（报告里用 ⇒ 不给「Safe/Mismatch」这种半吊子话）。
    pub fn label(self) -> &'static str {
        match self {
            RollbackVerdict::Safe => "可以回滚",
            RollbackVerdict::Mismatch => "账实不符",
            RollbackVerdict::NothingToRollback => "没有可回滚的",
            RollbackVerdict::SourceMissing => "原记录不在了",
        }
    }
}

/// 回滚的**结果**。
#[derive(Debug, Clone, PartialEq)]
pub struct RollbackOutcome {
    /// 判定。
    pub verdict: RollbackVerdict,
    /// 实际删除的副本 id（**只有 `Safe` 时非空**）。
    pub removed_ids: Vec<String>,
    /// 人话说明（**Mismatch 时必须说明差在哪**）。
    pub detail: String,
    /// 回滚后的归档物（用来证明「回到迁移前」）。
    pub after: OfflineArchive,
}

/// 为一次迁移**建账**（迁移**之前**调用 ⇒ 账是「迁移前的承诺」）。
///
/// # 口径
///
/// * 记录源锚点下的**全部**记录 id（**有序**）；
/// * 记录迁移前的**归档快照**（用 `offline_archive::export_archive`）；
/// * ⚠️ **不写库**（纯读）⇒ 建账不会改变任何东西。
///
/// # 「承诺」是什么
///
/// 账里的 `copied_ids` = 源记录 id 逐个加 `mig-` 前缀 ⇒ 这正是
/// `continuity_link::migrate_subject` 会造出来的副本 id（**同一套规则**）
/// ⇒ 迁移之后库里**就该**出现这些 id ⇒ 回滚时按账核对、按账删。
pub fn prepare_ledger(
    conn: &rusqlite::Connection,
    from: &str,
    to: &str,
    executed_at_ms: i64,
) -> Result<(PreMigrationSnapshot, MigrationLedger), String> {
    let from = from.trim();
    let to = to.trim();
    if from.is_empty() || to.is_empty() {
        return Err("迁移锚点不能为空（from / to 均须非空）".to_string());
    }
    if from == to {
        return Err(format!(
            "迁移锚点相同（from == to == {from}）⇒ 没有可迁移的东西 ⇒ 不建账"
        ));
    }

    let source_ids = read_source_ids(conn, from)?;
    let before = export_archive(conn, &ExportConfig::new(), executed_at_ms)?;
    let copied_ids = source_ids
        .iter()
        .map(|id| format!("{MIGRATED_ID_PREFIX}{id}"))
        .collect();

    Ok((
        PreMigrationSnapshot { before, source_ids },
        MigrationLedger {
            from: from.to_string(),
            to: to.to_string(),
            executed_at_ms,
            copied_ids,
        },
    ))
}

/// **核对**：迁移后的库与账**是否相符**。
///
/// # 口径
///
/// * 副本（`mig-{原 id}`）**都存在**且**都带链字段** ⇒ 否则 `Mismatch`；
/// * **原记录都还在**且**内容逐字未变**（比对迁移前快照）⇒ 否则 `SourceMissing`；
/// * 副本数量 == 账里的 `copied_ids` 数。
///
/// **判定顺序**（不许含糊）：账为空 ⇒ `NothingToRollback`；一个副本都没有 ⇒
/// `NothingToRollback`（没迁移过）；副本缺失/被改/没链/账外多出 ⇒ `Mismatch`；
/// 原记录不在了或被改写 ⇒ `SourceMissing`；全对 ⇒ `Safe`。
pub fn verify_migration(
    conn: &rusqlite::Connection,
    snapshot: &PreMigrationSnapshot,
    ledger: &MigrationLedger,
) -> RollbackVerdict {
    diagnose(conn, snapshot, ledger).0
}

/// **回滚**（删除迁移产生的副本）。
///
/// # 口径
///
/// * **先核对**（`verify_migration`）⇒ 不是 `Safe` ⇒ **什么都不做**（返回对应判定）；
/// * `Safe` ⇒ **只删副本**（`mig-` 前缀的那些）⇒ **绝不碰原记录**；
/// * 删完**再导出一次归档** ⇒ 证明「回到迁移前」（内容哈希应与迁移前**一致**）。
///
/// ⚠️ **失败即失败**：任一步出错 ⇒ 返回 `Err`，**不删半截**（用事务）。
pub fn rollback_migration(
    conn: &mut rusqlite::Connection,
    snapshot: &PreMigrationSnapshot,
    ledger: &MigrationLedger,
) -> Result<RollbackOutcome, String> {
    let (verdict, detail) = diagnose(conn, snapshot, ledger);
    if verdict != RollbackVerdict::Safe {
        // 不是 Safe ⇒ 一个副本都不删（**如实**把「差在哪」带回去）。
        let after = export_archive(conn, &ExportConfig::new(), ledger.executed_at_ms)?;
        return Ok(RollbackOutcome {
            verdict,
            removed_ids: Vec::new(),
            detail,
            after,
        });
    }

    let before_digest = snapshot.before.manifest.content_digest.clone();
    let tx = conn
        .transaction()
        .map_err(|error| format!("开回滚事务失败：{error}"))?;

    // ① 临时摘掉 append-only 删除触发器（事务内复原 ⇒ 失败也不会留下裸库）。
    let had_trigger = trigger_exists(&tx, EPISODES_NO_DELETE_TRIGGER)?;
    if had_trigger {
        tx.execute_batch(&format!("DROP TRIGGER {EPISODES_NO_DELETE_TRIGGER}"))
            .map_err(|error| format!("临时摘掉 append-only 删除触发器失败：{error}"))?;
    }

    // ② 只删账里的副本（三重保险：账内 + `mig-` 前缀 + 不是原记录）。
    let mut removed_ids = Vec::new();
    for id in &ledger.copied_ids {
        // **先**拒原记录（P1 底线最重 ⇒ 报错也要先说它），**再**拒非 `mig-` id。
        if snapshot.source_ids.iter().any(|source| source == id) {
            return Err(format!(
                "拒绝删除原记录 id `{id}`（P1 底线：原记录永不可删）"
            ));
        }
        if !id.starts_with(MIGRATED_ID_PREFIX) {
            return Err(format!(
                "拒绝删除非迁移副本 id `{id}`（没有 `{MIGRATED_ID_PREFIX}` 前缀）"
            ));
        }
        let changed = tx
            .execute("DELETE FROM episodes WHERE id = ?1", params![id])
            .map_err(|error| format!("删除副本 `{id}` 失败：{error}"))?;
        if changed != 1 {
            return Err(format!(
                "删除副本 `{id}` 失败：影响行数 = {changed}（账实已不符 ⇒ 停手）"
            ));
        }
        removed_ids.push(id.clone());
    }

    // ③ 复原链头：迁移把 `to` 这条链推进了 N 条 ⇒ 回滚必须把链头退回去，
    //    否则「链上条数 ≠ 链头 entry_count」⇒ 归档物自检直接不通过。
    restore_chain_head(&tx, &ledger.to, &snapshot.before)?;

    // ④ 复原 append-only 触发器（还在事务里 ⇒ 后面任何失败都一起回滚）。
    if had_trigger {
        tx.execute_batch(EPISODES_NO_DELETE_SQL)
            .map_err(|error| format!("复原 append-only 删除触发器失败：{error}"))?;
    }

    // ⑤ 事务内导出 ⇒ 校验「真的回到迁移前」；不符 ⇒ 报错并让事务整体回滚。
    let staged = export_archive(&tx, &ExportConfig::new(), ledger.executed_at_ms)?;
    let staged_digest = staged.manifest.content_digest.clone();
    if staged_digest != before_digest {
        return Err(format!(
            "回滚后内容哈希与迁移前不一致（迁移前 = {before_digest}，回滚后 = {staged_digest}）\
             ⇒ 判失败 ⇒ 事务回滚（一个副本都没删）"
        ));
    }

    tx.commit()
        .map_err(|error| format!("提交回滚事务失败：{error}"))?;

    let after = export_archive(conn, &ExportConfig::new(), ledger.executed_at_ms)?;
    let detail = format!(
        "已删除 {} 个迁移副本（{}）；原记录 {} 条**一条未动**；\
         回滚后内容哈希 = {staged_digest}（与迁移前一致 ⇒ 已回到迁移前）",
        removed_ids.len(),
        removed_ids.join(", "),
        snapshot.source_ids.len()
    );
    Ok(RollbackOutcome {
        verdict: RollbackVerdict::Safe,
        removed_ids,
        detail,
        after,
    })
}

/// **演练**：迁移 → 核对 → 回滚 → 核对「回到迁移前」。
///
/// # 口径（**这是本批的核心产物**）
///
/// * 建账 → 迁移（用**真实的** `continuity_link::migrate_subject`）→ 核对 → 回滚 → **再核对**；
/// * **断言**：回滚后的归档**内容哈希 == 迁移前的哈希** ⇒ **数据真的回到了原点**；
/// * 返回人话报告。
///
/// ⚠️ 只对**测试库**调用（**不许**对生产库跑演练）。
pub fn drill_rollback(
    conn: &mut rusqlite::Connection,
    from: &str,
    to: &str,
    executed_at_ms: i64,
) -> Result<String, String> {
    // ① 建账（纯读 ⇒ 不改库）。
    let (snapshot, ledger) = prepare_ledger(conn, from, to, executed_at_ms)?;

    // ② 真迁移（文件库走**真实入口** `continuity_link::migrate_subject`）。
    let invocation = run_migration(conn, &ledger)?;

    // ③ 核对（不通过 ⇒ 演练中止，**不做**回滚 ⇒ 宁可不动也不删错）。
    let (verdict, detail) = diagnose(conn, &snapshot, &ledger);
    if verdict != RollbackVerdict::Safe {
        return Err(format!(
            "迁移后核对未通过（{}）：{detail} ⇒ 演练中止（不做回滚，避免删错）",
            verdict.label()
        ));
    }
    let migrated = export_archive(conn, &ExportConfig::new(), executed_at_ms)?;

    // ④ 回滚。
    let outcome = rollback_migration(conn, &snapshot, &ledger)?;
    if outcome.verdict != RollbackVerdict::Safe {
        return Err(format!(
            "回滚未按预期完成（{}）：{}",
            outcome.verdict.label(),
            outcome.detail
        ));
    }
    if outcome.removed_ids != ledger.copied_ids {
        return Err(format!(
            "回滚删除的副本与账不符（账 = {:?}，实删 = {:?}）",
            ledger.copied_ids, outcome.removed_ids
        ));
    }

    // ⑤ 再核对「回到迁移前」。
    let final_archive = export_archive(conn, &ExportConfig::new(), executed_at_ms)?;
    let before_digest = snapshot.before.manifest.content_digest.clone();
    let after_digest = final_archive.manifest.content_digest.clone();
    if before_digest != after_digest {
        return Err(format!(
            "回滚后内容哈希与迁移前不一致（迁移前 = {before_digest}，回滚后 = {after_digest}）\
             ⇒ **没有**回到原点"
        ));
    }
    let before_self_check = verify_archive(&snapshot.before);
    let after_self_check = verify_archive(&final_archive);

    Ok(render_report(
        &ledger,
        &snapshot,
        &invocation,
        &migrated,
        &outcome,
        &final_archive,
        verdict,
        &detail,
        before_self_check,
        after_self_check,
    ))
}

// ============================================================
// 内部实现
// ============================================================

/// 迁移的**调用方式**（**如实记账** ⇒ 报告里说清楚走的哪条路）。
#[derive(Debug, Clone, PartialEq, Eq)]
enum MigrationInvocation {
    /// 真实入口 [`crate::continuity_link::migrate_subject`]（同库的第二个连接）。
    Real {
        /// 实际入链的副本数。
        copied: usize,
        /// 因 id 已存在而跳过的副本数（幂等重跑口径）。
        skipped: usize,
    },
    /// **退化路径**：同写路径等价实现（纯内存库没法重开第二个连接 ⇒ 只能在本连接上
    /// 走**同一条**链写路径）⇒ **不是** `migrate_subject` 本体 ⇒ 报告里显式标注。
    SameWritePath {
        /// 实际入链的副本数。
        copied: usize,
    },
}

impl MigrationInvocation {
    /// 报告里的一行说明（**不藏着**）。
    fn describe(&self) -> String {
        match self {
            MigrationInvocation::Real { copied, skipped } => format!(
                "continuity_link::migrate_subject（**真实入口**，同库第二个连接）；入链 {copied} 条，跳过 {skipped} 条"
            ),
            MigrationInvocation::SameWritePath { copied } => format!(
                "⚠️ 同写路径等价实现（**不是** migrate_subject 本体：纯内存库无法重开第二个连接）；入链 {copied} 条"
            ),
        }
    }
}

/// 读源锚点下的**全部**记录 id（**有序** ⇒ 确定性）。
///
/// 口径与 `continuity_link::migrate_subject` **同源**（`session_id = from`）。
fn read_source_ids(conn: &rusqlite::Connection, from: &str) -> Result<Vec<String>, String> {
    if !table_exists(conn, "episodes")? {
        // 没有 `episodes` 表 ⇒ 这个库里没有记忆 ⇒ 源集合为空（如实）。
        return Ok(Vec::new());
    }
    let mut stmt = conn
        .prepare("SELECT id FROM episodes WHERE session_id = ?1 ORDER BY id ASC")
        .map_err(|error| format!("准备源记录查询失败：{error}"))?;
    let rows = stmt
        .query_map(params![from], |row| row.get::<_, String>(0))
        .map_err(|error| format!("执行源记录查询失败：{error}"))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|error| format!("读源记录失败：{error}"))?);
    }
    Ok(out)
}

/// **核对**（判定 + 人话差异说明）。
///
/// `verify_migration` 只取判定，`rollback_migration` 还要拿「差在哪」⇒ 真相源在这。
fn diagnose(
    conn: &rusqlite::Connection,
    snapshot: &PreMigrationSnapshot,
    ledger: &MigrationLedger,
) -> (RollbackVerdict, String) {
    if ledger.copied_ids.is_empty() {
        return (
            RollbackVerdict::NothingToRollback,
            "账为空：迁移前源锚点下没有任何记录 ⇒ 没有任何副本可回滚".to_string(),
        );
    }

    let current = match export_archive(conn, &ExportConfig::new(), ledger.executed_at_ms) {
        Ok(archive) => archive,
        Err(reason) => {
            return (
                RollbackVerdict::Mismatch,
                format!("读不到当前库状态（{reason}）⇒ 无法确认账实相符 ⇒ 按「不许删」处理"),
            )
        }
    };
    let before_by_id: BTreeMap<&str, &crate::offline_archive::ArchivedEpisode> = snapshot
        .before
        .episodes
        .iter()
        .map(|episode| (episode.id.as_str(), episode))
        .collect();
    let current_by_id: BTreeMap<&str, &crate::offline_archive::ArchivedEpisode> = current
        .episodes
        .iter()
        .map(|episode| (episode.id.as_str(), episode))
        .collect();

    // ① 副本在不在。
    let mut present = 0usize;
    let mut missing: Vec<&str> = Vec::new();
    for id in &ledger.copied_ids {
        if current_by_id.contains_key(id.as_str()) {
            present += 1;
        } else {
            missing.push(id.as_str());
        }
    }
    if present == 0 {
        return (
            RollbackVerdict::NothingToRollback,
            format!(
                "库里一个迁移副本都没有（账里承诺 {} 个）⇒ 没迁移过（或早已被回滚）⇒ 无可回滚",
                ledger.copied_ids.len()
            ),
        );
    }
    if !missing.is_empty() {
        return (
            RollbackVerdict::Mismatch,
            format!(
                "副本缺失：账里承诺 {} 个副本，库里少了 {} 个（{}）⇒ 账实不符 ⇒ 不许删",
                ledger.copied_ids.len(),
                missing.len(),
                missing.join(", ")
            ),
        );
    }

    // ② 副本逐条核对：主体对不对、内容对不对、有没有链字段。
    for id in &ledger.copied_ids {
        let copy = current_by_id[id.as_str()];
        if copy.continuity_id != ledger.to {
            return (
                RollbackVerdict::Mismatch,
                format!(
                    "副本 `{id}` 的主体是 `{}`，不是目标锚点 `{}` ⇒ 账实不符 ⇒ 不许删",
                    copy.continuity_id, ledger.to
                ),
            );
        }
        let original_id = id.strip_prefix(MIGRATED_ID_PREFIX).unwrap_or(id);
        match before_by_id.get(original_id) {
            None => {
                return (
                    RollbackVerdict::Mismatch,
                    format!(
                        "副本 `{id}` 在迁移前快照里找不到对应原记录 `{original_id}` ⇒ 账不完整 ⇒ 不许删"
                    ),
                )
            }
            Some(original) => {
                if original.content != copy.content {
                    return (
                        RollbackVerdict::Mismatch,
                        format!(
                            "副本 `{id}` 的内容与原记录 `{original_id}` 不一致（副本被改过？）⇒ 账实不符 ⇒ 不许删"
                        ),
                    );
                }
            }
        }
        if copy.chain_hash.is_none() {
            return (
                RollbackVerdict::Mismatch,
                format!(
                    "副本 `{id}` 没有链字段（chain_hash 为空）⇒ 它不在哈希链保护范围内 ⇒ 账实不符 ⇒ 不许删"
                ),
            );
        }
    }

    // ③ 目标锚点下不许有账外副本（副本数量必须等于账里的数）。
    let extra: Vec<&str> = current
        .episodes
        .iter()
        .filter(|episode| {
            episode.continuity_id == ledger.to
                && episode.id.starts_with(MIGRATED_ID_PREFIX)
                && !ledger.copied_ids.iter().any(|id| id == &episode.id)
        })
        .map(|episode| episode.id.as_str())
        .collect();
    if !extra.is_empty() {
        return (
            RollbackVerdict::Mismatch,
            format!(
                "目标锚点 `{}` 下有账外副本 {} 个（{}）⇒ 副本数量与账不符 ⇒ 不许删",
                ledger.to,
                extra.len(),
                extra.join(", ")
            ),
        );
    }

    // ④ 原记录都还在，且**逐字未变**（P1 底线 ⇒ 不在 ⇒ 绝不许回滚）。
    for id in &snapshot.source_ids {
        let now = match current_by_id.get(id.as_str()) {
            None => {
                return (
                    RollbackVerdict::SourceMissing,
                    format!("原记录 `{id}` 不在了 ⇒ 原记录已丢 ⇒ **绝不许回滚**（P1 底线）"),
                )
            }
            Some(episode) => *episode,
        };
        match before_by_id.get(id.as_str()) {
            None => {
                return (
                    RollbackVerdict::SourceMissing,
                    format!("原记录 `{id}` 在迁移前快照里不存在 ⇒ 账不完整 ⇒ **绝不许回滚**"),
                )
            }
            Some(before) => {
                if before.content != now.content {
                    return (
                        RollbackVerdict::SourceMissing,
                        format!(
                            "原记录 `{id}` 的内容被改写（迁移前 ≠ 现在）⇒ 原记录已不在「逐字未变」的状态 ⇒ **绝不许回滚**"
                        ),
                    );
                }
            }
        }
    }

    (
        RollbackVerdict::Safe,
        format!(
            "账实相符：{} 个副本都在、都带链字段、内容与原记录逐字一致；{} 条原记录都还在且逐字未变 ⇒ 可安全回滚",
            ledger.copied_ids.len(),
            snapshot.source_ids.len()
        ),
    )
}

/// 复原 `to` 这条链的链头（迁移推进过的链头要退回去，否则归档物自检不通过）。
fn restore_chain_head(
    tx: &Transaction<'_>,
    to: &str,
    before: &OfflineArchive,
) -> Result<(), String> {
    if !table_exists(tx, "memory_chain_heads")? {
        // 没有链头表 ⇒ 这个库没有链账本 ⇒ 无链头可复原（如实）。
        return Ok(());
    }
    match before.chain_heads.iter().find(|head| head.subject_id == to) {
        Some(head) => {
            tx.execute(
                "INSERT INTO memory_chain_heads (subject_id, last_hash, entry_count, updated_at_ms)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(subject_id) DO UPDATE SET
                     last_hash    = excluded.last_hash,
                     entry_count  = excluded.entry_count,
                     updated_at_ms = excluded.updated_at_ms",
                params![
                    to,
                    head.last_hash,
                    head.entry_count as i64,
                    head.updated_at_ms
                ],
            )
            .map_err(|error| format!("复原链头失败：{error}"))?;
        }
        None => {
            // 迁移前没有这条链 ⇒ 迁移新建的链头要删掉（否则链账里留着空链）。
            tx.execute(
                "DELETE FROM memory_chain_heads WHERE subject_id = ?1",
                params![to],
            )
            .map_err(|error| format!("删除迁移新建的链头失败：{error}"))?;
        }
    }
    Ok(())
}

/// 跑一次**真迁移**。
///
/// 优先走**真实入口** [`crate::continuity_link::migrate_subject`]（需要能重开同一个库
/// ⇒ 文件库 / 具名共享内存库）；纯内存库没法重开 ⇒ 退到**同写路径等价实现**
/// （同查询、同副本 id 规则、同入链函数）⇒ 报告里**如实标注**。
fn run_migration(
    conn: &rusqlite::Connection,
    ledger: &MigrationLedger,
) -> Result<MigrationInvocation, String> {
    if let Some(file) = database_file(conn)? {
        if !file.trim().is_empty() {
            let store = SqliteMemoryStore::open(&file)
                .map_err(|error| format!("打开同库 store 失败（{file}）：{error}"))?;
            let report = crate::continuity_link::migrate_subject(
                &store,
                &ledger.from,
                &ledger.to,
                ledger.executed_at_ms,
            )
            .map_err(|error| format!("continuity_link::migrate_subject 失败：{error}"))?;
            return Ok(MigrationInvocation::Real {
                copied: report.episodes_copied,
                skipped: report.episodes_skipped,
            });
        }
    }
    let copied = copy_forward_same_path(conn, ledger)?;
    Ok(MigrationInvocation::SameWritePath { copied })
}

/// **同写路径等价实现**（退化路径；**不是** `migrate_subject` 本体）。
///
/// 与 [`crate::continuity_link::migrate_subject`] 的**取源条件、副本 id 规则、入链函数**
/// 完全一致 ⇒ 造出来的副本与真迁移**逐字同形**；差别只有「谁发起的这次写入」。
fn copy_forward_same_path(
    conn: &rusqlite::Connection,
    ledger: &MigrationLedger,
) -> Result<usize, String> {
    if !table_exists(conn, "episodes")? {
        return Ok(0);
    }
    let mut stmt = conn
        .prepare(
            "SELECT id, timestamp, role, content, session_id FROM episodes \
             WHERE session_id = ?1 ORDER BY timestamp ASC, id ASC",
        )
        .map_err(|error| format!("准备取源 SQL 失败：{error}"))?;
    let rows = stmt
        .query_map(params![ledger.from], |row| {
            Ok(Episode {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                session_id: row.get(4)?,
            })
        })
        .map_err(|error| format!("执行取源 SQL 失败：{error}"))?;

    let mut copied = 0usize;
    for row in rows {
        let original = row.map_err(|error| format!("读源记录失败：{error}"))?;
        let copy = Episode {
            id: format!("{MIGRATED_ID_PREFIX}{}", original.id),
            timestamp: original.timestamp,
            role: original.role.clone(),
            content: original.content.clone(),
            session_id: ledger.to.clone(),
        };
        let entry = ChainEntry::from_episode(&copy, None);
        let outcome = put_chained_episode(conn, &ledger.to, &entry)
            .map_err(|error| format!("副本 `{}` 入链失败：{error}", copy.id))?;
        if outcome.chained {
            copied += 1;
        }
    }
    // 与 `migrate_subject` 同口径：`context_ledger` 存在时也重键（**如实标注**：回滚不改键回去）。
    if crate::context_ledger::context_ledger_table_exists(conn) {
        conn.execute(
            "UPDATE context_ledger_messages SET continuity_id = ?1 WHERE continuity_id = ?2",
            params![ledger.to, ledger.from],
        )
        .map_err(|error| format!("重键 context_ledger 失败：{error}"))?;
    }
    Ok(copied)
}

/// 库里 `main` 数据库的文件名（`PRAGMA database_list`）。
///
/// 纯内存库返回空串 ⇒ 用来判定「能不能重开第二个连接」。
fn database_file(conn: &rusqlite::Connection) -> Result<Option<String>, String> {
    let mut stmt = conn
        .prepare("PRAGMA database_list")
        .map_err(|error| format!("读 database_list 失败：{error}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        })
        .map_err(|error| format!("读 database_list 失败：{error}"))?;
    for row in rows {
        let (name, file) = row.map_err(|error| format!("读 database_list 失败：{error}"))?;
        if name == "main" {
            return Ok(Some(file));
        }
    }
    Ok(None)
}

/// 表在不在（`sqlite_master` 判定）。
fn table_exists(conn: &rusqlite::Connection, name: &str) -> Result<bool, String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            params![name],
            |row| row.get(0),
        )
        .map_err(|error| format!("查 sqlite_master 失败：{error}"))?;
    Ok(count > 0)
}

/// 触发器在不在（`sqlite_master` 判定）。
fn trigger_exists(conn: &rusqlite::Connection, name: &str) -> Result<bool, String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'trigger' AND name = ?1",
            params![name],
            |row| row.get(0),
        )
        .map_err(|error| format!("查 sqlite_master 失败：{error}"))?;
    Ok(count > 0)
}

/// 渲染演练报告（人话 + 机器可复核的哈希）。
#[allow(clippy::too_many_arguments)]
fn render_report(
    ledger: &MigrationLedger,
    snapshot: &PreMigrationSnapshot,
    invocation: &MigrationInvocation,
    migrated: &OfflineArchive,
    outcome: &RollbackOutcome,
    final_archive: &OfflineArchive,
    verify_verdict: RollbackVerdict,
    verify_detail: &str,
    before_self_check: Result<(), String>,
    after_self_check: Result<(), String>,
) -> String {
    let before_digest = &snapshot.before.manifest.content_digest;
    let migrated_digest = &migrated.manifest.content_digest;
    let after_digest = &final_archive.manifest.content_digest;
    let back = before_digest == after_digest;

    let mut out = String::new();
    out.push_str("════════ 迁移回滚演练报告 ════════\n");
    out.push_str(&format!("源锚点（from）  ：{}\n", ledger.from));
    out.push_str(&format!("目标锚点（to）  ：{}\n", ledger.to));
    out.push_str(&format!(
        "迁移时刻         ：{} ms（epoch 毫秒，注入值）\n",
        ledger.executed_at_ms
    ));
    out.push_str(&format!("迁移调用方式     ：{}\n", invocation.describe()));
    out.push('\n');
    out.push_str("── 建账（迁移前，纯读）──\n");
    out.push_str(&format!(
        "源锚点记录数     ：{} 条（session_id = {}）\n",
        snapshot.source_ids.len(),
        ledger.from
    ));
    out.push_str(&format!(
        "账里的副本承诺   ：{} 个（{}）\n",
        ledger.copied_ids.len(),
        ledger.copied_ids.join(", ")
    ));
    out.push_str(&format!(
        "迁移前条数       ：{} 条\n",
        snapshot.before.manifest.episode_count
    ));
    out.push_str(&format!("迁移前内容哈希   ：{before_digest}\n"));
    out.push('\n');
    out.push_str("── 迁移后（回滚前）──\n");
    out.push_str(&format!(
        "迁移后条数       ：{} 条\n",
        migrated.manifest.episode_count
    ));
    out.push_str(&format!("迁移后内容哈希   ：{migrated_digest}\n"));
    out.push_str(&format!(
        "核对判定         ：{}（{verify_detail}）\n",
        verify_verdict.label()
    ));
    out.push_str(&format!(
        "回滚结论         ：{}（{}）\n",
        outcome.verdict.label(),
        outcome.detail
    ));
    out.push('\n');
    out.push_str("── 回滚后 ──\n");
    out.push_str(&format!(
        "删除的副本       ：{} 个（{}）\n",
        outcome.removed_ids.len(),
        outcome.removed_ids.join(", ")
    ));
    out.push_str(&format!(
        "原记录           ：{} 条**一条未动**（P1 底线）\n",
        snapshot.source_ids.len()
    ));
    out.push_str(&format!(
        "回滚后条数       ：{} 条\n",
        final_archive.manifest.episode_count
    ));
    out.push_str(&format!("回滚后内容哈希   ：{after_digest}\n"));
    out.push_str(&format!(
        "回滚后自检       ：{}\n",
        match &after_self_check {
            Ok(()) => "通过（条数 / 哈希 / 链账一致）".to_string(),
            Err(reason) => format!("未通过（{reason}）"),
        }
    ));
    out.push_str(&format!(
        "迁移前自检       ：{}\n",
        match &before_self_check {
            Ok(()) => "通过".to_string(),
            Err(reason) => format!("未通过（{reason}）"),
        }
    ));
    out.push('\n');
    if back {
        out.push_str("── 结论 ──\n");
        out.push_str(&format!(
            "**回到原点**：回滚后内容哈希 = {after_digest}，与迁移前内容哈希 {before_digest} **逐字相等**\n"
        ));
        out.push_str(
            "⇒ 迁移留下的副本已被完整撤销，数据真的回到了迁移前（不是「看起来差不多」）。\n",
        );
    } else {
        out.push_str("── 结论 ──\n");
        out.push_str(&format!(
            "**没有回到原点**：回滚后内容哈希 {after_digest} ≠ 迁移前 {before_digest}\n"
        ));
    }
    out.push_str(
        "已知边界：`context_ledger` 的重键副作用**不回滚**（改键回去会搬错行 ⇒ 宁可少做）。\n",
    );
    out
}
