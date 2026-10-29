//! `apeireth-memory::cold_archive` — **单条记忆的冷层归档**（归档 ≠ 删除）。
//!
//! # 为什么
//!
//! §6.2 的 ⭐ **保真可达率**要证明「被降权/归档的记忆，明确需要时**仍能检索到**」
//! ⇒ 需要一个**真的**「归档」语义（单条）⇒ 而仓里目前只有**整库脱机导出**
//! ⇒ ⇒ 本模块补上**单条**冷层归档。
//!
//! # 三条纪律
//!
//! 1. **不删数据**（P1：原始证据永久保留）⇒ 归档只是**打标记 + 降权**；
//! 2. **不消失**（保真可达）⇒ 归档件**仍能检索到**（权重降，**不是**过滤掉）；
//! 3. **账实相符可查**（归档账本 + 独立校验）。
//!
//! # 与「整库脱机归档」的区别（别混）
//!
//! [`crate::offline_archive`] 是**整库**的脱机可读副本（给人和灾备看的**文件**）；
//! 本模块是**单条**记忆的冷层标记（给检索侧看的**状态**）⇒
//! 一个是「导出物」，一个是「账本 + 权重」。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-27）
//!
//! 四级口径（本模块的真实位置）：
//!
//! - **IMPLEMENTED** ✔：归档 / 取消归档 / 读账 / 独立校验 / 权重 / 人话报告齐备
//!   （库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：**未接入**任何生产路径（生产 0 调用点；只有演练例子在用）；
//! - **DEFAULT ENABLED** ✘：没有开关位，也未默认启用；
//! - **HARDWARE VALIDATED** ✘：只在库级测试与**自建真实 schema 库**上跑过，
//!   **未在生产记忆库上实测**（本机生产库 `episodes` 0 条 ⇒ 无从实测）⇒ 不宣称已验证。
//!
//! 对外一律按 **NOT IMPLEMENTED** 表述（新增未接线，不宣称可用）。

use rusqlite::{params, Connection};

/// 归档件的**默认权重系数**（明显靠后，但**不是 0**）。
///
/// ⚠️ **0 是硬禁止**：0 = 检索时把它过滤掉 ⇒ 违反「归档件不消失」
/// ⇒ 「明确需要时仍能检索到」这条纪律就废了 ⇒ [`ColdArchiveConfig::new`] 直接拒 0。
pub const DEFAULT_WEIGHT_FACTOR: f64 = 0.2;

/// 归档账本表名（**独立表** ⇒ 自己建表 ⇒ **不依赖迁移**）。
///
/// **为什么独立表**：`episodes` 是 append-only（`BEFORE UPDATE` / `BEFORE DELETE`
/// 触发器直接 ABORT）⇒ 归档标记**不可能**写在 `episodes` 上 ⇒ 只能另起一账。
pub const COLD_ARCHIVE_TABLE: &str = "memory_cold_archive";

/// 一条**归档记录**（账本里的一行）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveEntry {
    /// 被归档的记录 id。
    pub id: String,
    /// 归档时刻（epoch 毫秒；**注入** ⇒ 确定性）。
    pub archived_at_ms: i64,
    /// **为什么**归档（**留痕** ⇒ 事后可查）。
    pub reason: String,
}

/// 归档的**账本**（可独立核对）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveLedger {
    /// 归档条目（**有序** ⇒ 确定性）。
    pub entries: Vec<ArchiveEntry>,
}

impl ArchiveLedger {
    /// 已归档的记录 id（**有序**）。
    pub fn archived_ids(&self) -> Vec<String> {
        self.entries.iter().map(|entry| entry.id.clone()).collect()
    }

    /// 某条是否已归档。
    pub fn is_archived(&self, id: &str) -> bool {
        self.entries.iter().any(|entry| entry.id == id)
    }
}

/// 归档配置（**权重降多少**）。
///
/// ⚠️ **只 derive `PartialEq`，不 derive `Eq`**：`weight_factor` 是 `f64`
/// ⇒ `f64` **不实现 `Eq`**（浮点没有全序等价）⇒ derive `Eq` 直接编译不过
/// ⇒ 契约草案里的 `Eq` 在这个字段上**不可实现**（如实标注差异，不硬凑）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColdArchiveConfig {
    /// 归档件的**权重系数**（**默认 0.2** ⇒ 明显靠后，但**不是 0**）。
    ///
    /// ⚠️ **不许用 0**（0 = 过滤掉 ⇒ 违反「不消失」）⇒
    /// ⇒ 本模块**拒绝 0**（返回 `Err`），这是硬约束。
    pub weight_factor: f64,
}

impl ColdArchiveConfig {
    /// 构造（`weight_factor = 0.0` ⇒ **直接报错**）。
    ///
    /// # 为什么连负数也拒
    ///
    /// 负系数会让归档件在排序里**反超**未归档件 ⇒ 语义上比「不降权」更荒唐
    /// ⇒ 与 0 同罪 ⇒ 一并拒（`Err`），不留给调用方踩。
    pub fn new(weight_factor: f64) -> Result<Self, String> {
        if !weight_factor.is_finite() {
            return Err(format!(
                "权重系数必须有限（实际 {weight_factor}）：NaN/Inf 会让排序口径不可复核"
            ));
        }
        if weight_factor <= 0.0 {
            return Err(format!(
                "权重系数必须 > 0（实际 {weight_factor}）：0 = 过滤掉 ⇒ 违反「归档件不消失」\
                 ⇒ 「明确需要时仍能检索到」就废了（本模块硬拒 0）"
            ));
        }
        Ok(Self { weight_factor })
    }

    /// 默认配置（0.2）。
    pub fn default_config() -> Self {
        Self {
            weight_factor: DEFAULT_WEIGHT_FACTOR,
        }
    }
}

/// 归档一条记忆（**不删数据** ⇒ 只写账本 + 打标记）。
///
/// # 口径
///
/// * **幂等**：同 id 重复归档 ⇒ `Ok(false)`（**不重复记账**）；
/// * **不碰** `episodes` 行的任何一列（**数据不动** ⇒ P1）；
/// * 账本写在**独立表** `memory_cold_archive`（自己建表 ⇒ **不依赖迁移**）。
///
/// # 错误
///
/// * id 为空 ⇒ `Err`；
/// * 目标记录**不存在** ⇒ `Err`（**不许**归档不存在的记忆）。
pub fn archive_episode(
    conn: &Connection,
    id: &str,
    reason: &str,
    archived_at_ms: i64,
) -> Result<bool, String> {
    let id = id.trim();
    if id.is_empty() {
        return Err("记忆 id 为空 ⇒ 不许归档（归档对象必须指名道姓）".to_string());
    }
    ensure_ledger_table(conn)?;
    // 目标必须**真实存在** ⇒ 不许给不存在的记忆记账（账实相符从写入侧就守）。
    if !episode_exists(conn, id)? {
        return Err(format!(
            "记忆不存在：{id} ⇒ 不许归档（不许给不存在的记忆记账 ⇒ 账实相符从写入侧就守）"
        ));
    }
    // `INSERT OR IGNORE` ⇒ 同 id 重复归档时**不重复记账**（幂等 ⇒ 账本不膨胀）。
    let changed = conn
        .execute(
            "INSERT OR IGNORE INTO memory_cold_archive (id, archived_at_ms, reason) \
             VALUES (?1, ?2, ?3)",
            params![id, archived_at_ms, reason],
        )
        .map_err(|error| format!("写归档账本失败：{error}"))?;
    Ok(changed > 0)
}

/// 读归档账本（**有序**）。
///
/// **口径**：按 `(archived_at_ms, id)` 升序 ⇒ 同库两次读**逐字相等**（确定性）。
/// 账本表不存在 ⇒ 返回**空账本**（不是错：这个库还没归档过任何东西），
/// 且**只读**连接也能读（不为了读而建表）。
pub fn read_ledger(conn: &Connection) -> Result<ArchiveLedger, String> {
    if !ledger_table_exists(conn)? {
        return Ok(ArchiveLedger {
            entries: Vec::new(),
        });
    }
    let mut statement = conn
        .prepare(
            "SELECT id, archived_at_ms, reason FROM memory_cold_archive \
             ORDER BY archived_at_ms ASC, id ASC",
        )
        .map_err(|error| format!("准备归档账本 SQL 失败：{error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(ArchiveEntry {
                id: row.get(0)?,
                archived_at_ms: row.get(1)?,
                reason: row.get(2)?,
            })
        })
        .map_err(|error| format!("读归档账本失败：{error}"))?;

    let mut entries = Vec::new();
    for row in rows {
        entries.push(row.map_err(|error| format!("读归档账本行失败：{error}"))?);
    }
    Ok(ArchiveLedger { entries })
}

/// **取消归档**（**可逆** ⇒ 归档不该是单向门）。
///
/// # 口径
///
/// * 取消一条已归档的 ⇒ `Ok(true)`（账本里没有它了）；
/// * 取消一条**未归档**的 ⇒ `Ok(false)`（**不记账**：没归档过就没有可取消的东西）。
pub fn unarchive_episode(conn: &Connection, id: &str) -> Result<bool, String> {
    let id = id.trim();
    if id.is_empty() {
        return Err("记忆 id 为空 ⇒ 不许取消归档".to_string());
    }
    ensure_ledger_table(conn)?;
    let changed = conn
        .execute("DELETE FROM memory_cold_archive WHERE id = ?1", params![id])
        .map_err(|error| format!("删归档账本记录失败：{error}"))?;
    Ok(changed > 0)
}

/// **独立校验**：账本里的每条**都真的存在于** `episodes`（⇒ 没删数据）。
///
/// # 返回
///
/// * `Ok(())` = 相符；
/// * `Err(reason)` = 不符（**如实**说明差在哪）。
///
/// # 为什么这条检查不是「空判据」
///
/// `episodes` 上的 append-only 触发器本来就拦着 DELETE ⇒ 正常路径下这条检查
/// 几乎总是通过 ⇒ **它的价值在异常路径**：万一有人绕过了触发器（改库脚本、
/// 迁移事故、文件损坏），账本就会指着一条**已经不在**的记忆 ⇒ 这时报错，
/// 就是「数据真的丢了」的**机器信号**（而不是等检索时才发现检索不到）。
pub fn verify_ledger(conn: &Connection) -> Result<(), String> {
    let ledger = read_ledger(conn)?;
    if ledger.entries.is_empty() {
        return Ok(());
    }
    if !table_exists(conn, "episodes")? {
        return Err(format!(
            "账实不符：`episodes` 表不存在 ⇒ 账本里的 {} 条归档记录全部对不上（原始证据不在）",
            ledger.entries.len()
        ));
    }
    for entry in &ledger.entries {
        if !episode_exists(conn, &entry.id)? {
            return Err(format!(
                "账实不符：归档记录 {} 在 `episodes` 里**不存在** ⇒ 原始证据丢了\
                 （归档 ≠ 删除 ⇒ 这条不该丢）",
                entry.id
            ));
        }
    }
    Ok(())
}

/// **归档件的检索权重**（供检索侧用；**不实现**检索本身）。
///
/// # 口径
///
/// * 未归档 ⇒ `1.0`（**原样**）；
/// * 已归档 ⇒ `weight_factor`（**不是 0** ⇒ 仍会被召回，只是靠后）。
pub fn weight_for(ledger: &ArchiveLedger, id: &str, config: &ColdArchiveConfig) -> f64 {
    if ledger.is_archived(id) {
        config.weight_factor
    } else {
        1.0
    }
}

/// 人话报告。
///
/// # 必须包含
///
/// * **条目数**（账上有多少条归档）；
/// * **校验结论**（相符 / 不符 + 不符的原因）；
/// * **口径提醒**：「归档 ≠ 删除」「归档件不消失（权重不许 0）」「状态 NOT IMPLEMENTED」。
pub fn render_cold_report(ledger: &ArchiveLedger, verify: Result<(), String>) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push("════════ 冷层归档报告（单条归档 · 归档 ≠ 删除）════════".to_string());
    lines.push(format!("归档条目数：{}", ledger.entries.len()));
    lines.push(String::new());
    lines.push(format!(
        "── 账本（{} 条；按 (归档时刻, id) 升序）──",
        ledger.entries.len()
    ));
    if ledger.entries.is_empty() {
        lines.push("  （空账本：这个库还没有归档过任何记忆）".to_string());
    }
    for entry in &ledger.entries {
        lines.push(format!(
            "  · {} · 归档于 {} ms · 原因：{}",
            entry.id, entry.archived_at_ms, entry.reason
        ));
    }

    lines.push(String::new());
    lines.push("── 账实校验 ──".to_string());
    match verify {
        Ok(()) => lines.push(format!(
            "  ✅ 相符：账本里的 {} 条归档记录**都还在** `episodes` 里（原始证据没删 ⇒ P1 成立）",
            ledger.entries.len()
        )),
        Err(reason) => lines.push(format!("  ❌ 不符：{reason}")),
    }

    lines.push(String::new());
    lines.push("── 口径提醒（不许删）──".to_string());
    lines.push(
        "⚠️ 归档 ≠ 删除：归档件**原文仍在** `episodes` 表里（逐字保留 ⇒ §2 原则 P1）".to_string(),
    );
    lines.push(format!(
        "⚠️ 归档件**不消失**：检索权重降为 weight_factor（默认 {}；**不许 0** ⇒ 0 = 过滤掉 = 违反「不消失」）",
        DEFAULT_WEIGHT_FACTOR
    ));
    lines.push("⚠️ 状态：NOT IMPLEMENTED（新增未接线；库级实现，生产 0 调用点）".to_string());
    lines.join("\n")
}

// ============================================================
// 内部实现
// ============================================================

/// 建账本表（**幂等**；自建 ⇒ **不依赖迁移**）。
///
/// **为什么自己建表**：迁移列表是 append-only 的历史账 ⇒ 往里加版本属于
/// 「改历史」（本仓纪律不许）⇒ 新表自己建（`IF NOT EXISTS` ⇒ 幂等），
/// 迁移列表**0 触碰**。
fn ensure_ledger_table(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS memory_cold_archive (
             id             TEXT PRIMARY KEY,
             archived_at_ms INTEGER NOT NULL,
             reason         TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_memory_cold_archive_time
             ON memory_cold_archive(archived_at_ms, id);",
    )
    .map_err(|error| format!("建归档账本表失败：{error}"))
}

/// 账本表在不在（`sqlite_master` 判定）。
fn ledger_table_exists(conn: &Connection) -> Result<bool, String> {
    table_exists(conn, COLD_ARCHIVE_TABLE)
}

/// 表在不在（`sqlite_master` 判定）。
fn table_exists(conn: &Connection, name: &str) -> Result<bool, String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            params![name],
            |row| row.get(0),
        )
        .map_err(|error| format!("查 sqlite_master 失败：{error}"))?;
    Ok(count > 0)
}

/// 这条记忆**真的存在**吗（只看 `id` ⇒ 不假设其它列存在）。
///
/// **为什么只查 `id`**：`episodes` 的列集随迁移版本变（V4 加 4 列、V19 加 2 列）
/// ⇒ SQL 若假设了不存在的列就会直接报错（本仓吃过这个亏）⇒
/// 归档侧只需要「这条在不在」⇒ **只查 `id`**（`id` 从 V1 起就有）。
fn episode_exists(conn: &Connection, id: &str) -> Result<bool, String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM episodes WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .map_err(|error| format!("查 `episodes` 失败：{error}"))?;
    Ok(count > 0)
}
