//! `apeireth_memory::memory_chain` — 记忆层权威存储（D0）的哈希链集成与体检。
//!
//! # 为什么在记忆层做链
//!
//! 设计稿 `docs/01-architecture/memory-knowledge-depth-system-spec.md` 原则 P3：
//! 「权威层带哈希链 + 定期 scrub + **能定位到具体记录**」。
//! 缺口清单 `docs/status/d0-fidelity-audit.md` 的 G6（哈希链缺失）/ G7（没有 verify
//! 体检命令）就是本模块要对上的两条。
//!
//! 链原语（字节布局 + SHA-256 + 链接）**不在本模块**：它住在
//! `apeireth_core::hash_chain`（纯密码学、无策略），治理审计账本与本模块共用同一套
//! 布局 ⇒ 两边算出来的摘要口径一致，也不会出现「两个链算法」的漂移。
//! 本模块只负责**记忆层的策略**：链什么、按什么粒度分链、怎么写、怎么查。
//!
//! # 链的粒度：按 subject 分链（不是全局一条链）
//!
//! 记忆是多主体多会话的。一条全局链有两个硬伤：
//!
//! 1. **写阻塞**：A 主体写一条要等 B 主体写一条（链头是全局单点）。
//! 2. **不可定位**：一条链上任何一处断裂，都说不清「是哪条记录被改」。
//!
//! ⇒ **每个 subject（`episodes.continuity_id`）一条独立链**，链头落在
//! `memory_chain_heads` 表（`subject_id` 主键）。主体之间互不影响：A 主体被篡改
//! 不会让 B 主体的链变红。
//!
//! # 链什么：记录的全部内容
//!
//! 摘要必须覆盖**这条记录的全部内容** ⇒ 任何字段被改都会破链。覆盖字段（顺序固定，
//! 顺序本身是摘要口径的一部分）：
//!
//! | 字段 | 编码 | 为什么 |
//! |---|---|---|
//! | `id` | 长度前缀 | 主键；换 id = 换记录 |
//! | `content` | 长度前缀 | 原文（D0 ①，最不可再生） |
//! | `timestamp` | 定长 8 字节大端 | 发生时间（D0 ③） |
//! | `role` | 长度前缀 | 说话人（D0 ②） |
//! | `session_id` | 长度前缀 | 上下文（D0 ⑥） |
//! | `valid_from_ms` | 存在位 + 定长 8 字节 | 生效起点（D0 ③） |
//! | `valid_until_ms` | 存在位 + 定长 8 字节 | 失效时间（D0 ③） |
//! | `created_ms` | 存在位 + 定长 8 字节 | 记录时间（D0 ③） |
//! | `provenance` | 存在位 + 长度前缀 | 出处（D0 ④） |
//!
//! **为什么要长度前缀**：`["ab", "c"]` 与 `["a", "bc"]` 直接拼接的字节完全相同 ⇒
//! 摘要相同 ⇒ 链分不清两种切分（拼接歧义）。长度前缀让两种切分的前像不同。
//! 定长数值（时间戳）不需要前缀（长度由类型固定），但**可空值需要存在位**：
//! `NULL` 与「值恰好是 0」必须可区分。
//!
//! **「列不存在」按「值为 NULL」口径**：`episodes` 的 4 个 V4 元数据列在老库/别的
//! 后端 schema 里可能不存在。此时读写两侧都按 `NULL` 编码 ⇒ 摘要一致；日后该列被
//! `ALTER` 补上且存量行为 NULL ⇒ 摘要不变（不会误报篡改）。
//!
//! # 链的顺序 = 写入顺序（append-only 的必然结果）
//!
//! `episodes` 有 append-only 触发器（`BEFORE UPDATE` / `BEFORE DELETE` 直接 ABORT），
//! 所以**链上每条记录的摘要一旦写下就不能改**。这意味着链只能是**追加的线性链**：
//! 新记录的前驱 = 当前链头。若按「时间戳排序」重排并回填摘要，就得改老记录 ⇒ 与
//! append-only 冲突 ⇒ 做不了。
//!
//! ⇒ 链序 = 写入序；校验时**顺着链头指针走**（从创世前驱值出发逐条跟 `chain_prev_hash`），
//! 而不是按时间戳重排。绝大多数场景（时间戳单调递增）两者一致；时间戳乱序写入时，
//! 按时间戳重排会把「合法写入」误报成断链。
//!
//! # 口径（诚实标注）
//!
//! - 库级能力（链写入 + 校验 API + 汇总）：**IMPLEMENTED**（本文件 + 集成测试覆盖）。
//! - 生产接线：**只接了一条**写入路径 —— RC-1 后端 `SqliteBackend::put_episode`。
//!   v1 存储 `SqliteMemoryStore` 的既有入口（`EpisodeStore::put_episode` /
//!   `put_episode_full`）**保持原样不接链**（那两条路径的产出是「未纳管」记录），
//!   另提供显式链入口 [`SqliteMemoryStore::put_episode_chained`]。
//! - 设计稿 G7 要求的 `memory verify` **体检命令**：**NOT IMPLEMENTED**
//!   （本批只交付库级 API，CLI 入口未接线）。
//!
//! # 边界（不假装）
//!
//! - 本模块**只管链**：不判业务合法性、不做归档/遗忘、不改检索结果。
//! - 治理侧 `episode_governance.content_override`（软删/修订内容）**不在摘要内**：
//!   它是 sidecar，改它不改 `episodes` 行 ⇒ 链不会因此变红。治理改写另有 sidecar 审计，
//!   两套账本职责不同（链证「原始行没被改」，sidecar 证「谁批的改」）。
//! - 摘要**不含密钥**：链只提供篡改可检出性，不提供不可否认性（没有签名）。

use std::collections::BTreeMap;

use apeireth_core::hash_chain::{hash_linked, Chunk, GENESIS_PREVIOUS_HASH};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};

use crate::{MemoryError, MemoryResult};

/// 一条参与链摘要的 episode 字段快照（= 摘要覆盖的全部内容）。
///
/// 读写两侧共用这一个结构 ⇒ 「写时算什么」与「查时重算什么」逐字一致。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainEntry {
    /// 记录 id（主键）。
    pub id: String,
    /// 原文。
    pub content: String,
    /// 发生时间（epoch 秒）。
    pub timestamp: i64,
    /// 说话人 / 角色。
    pub role: String,
    /// 会话 id（上下文）。
    pub session_id: String,
    /// 生效起点（epoch 毫秒）。
    pub valid_from_ms: Option<i64>,
    /// 失效时间（epoch 毫秒）；`None` = 永久有效。
    pub valid_until_ms: Option<i64>,
    /// 记录时间（epoch 毫秒）。
    pub created_ms: Option<i64>,
    /// 来源（`provenance` 列原文）。
    pub provenance: Option<String>,
}

impl ChainEntry {
    /// 由 `Episode` + 可选元数据构造（`EpisodeMeta` 的 4 列）。
    ///
    /// `meta` 为 `None` 时 4 个元数据字段按 `NULL` 口径（用于后端那条只写 6 列的路径）。
    pub fn from_episode(
        episode: &apeireth_core::kernel::memory::Episode,
        meta: Option<&crate::EpisodeMeta>,
    ) -> Self {
        Self {
            id: episode.id.clone(),
            content: episode.content.clone(),
            timestamp: episode.timestamp,
            role: episode.role.clone(),
            session_id: episode.session_id.clone(),
            valid_from_ms: meta.and_then(|m| m.valid_from_ms),
            valid_until_ms: meta.and_then(|m| m.valid_until_ms),
            created_ms: meta.map(|m| m.created_ms),
            provenance: meta.map(|m| m.provenance.as_str().to_string()),
        }
    }
}

/// 一次带链写入的结果（**如实**体现这条记录有没有真的接上链）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainWriteOutcome {
    /// 所属主体（= 链的键）。
    pub subject_id: String,
    /// 记录 id。
    pub episode_id: String,
    /// 本条记录的链摘要（已存在时 = 库中原有的摘要）。
    pub chain_hash: String,
    /// 本条记录的前驱摘要（已存在时 = 库中原有的前驱）。
    pub chain_prev_hash: String,
    /// 该主体当前的链上条数（已存在时不推进）。
    pub entry_count: u64,
    /// 本次调用是否**新接了一条链**。
    ///
    /// `false` = 该 id 的记录早已存在（幂等重写）⇒ 未重复入链、链头未推进。
    pub chained: bool,
}

/// 一条链的校验结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainVerification {
    /// 主体 id。
    pub subject_id: String,
    /// 链上条目总数（= 带链字段的记录条数）。
    pub entry_count: usize,
    /// 是否**完全**通过（链首尾相连 + 每条摘要与内容重算一致 + 链头账实相符）。
    pub ok: bool,
    /// 第一个出问题的位置（0 基；无问题则 None）。
    pub first_bad_index: Option<usize>,
    /// 第一个出问题的记录 id（断在「缺记录」处时可能为 None）。
    pub first_bad_id: Option<String>,
    /// 链头摘要（顺着链走到的最后一条；空链 = 创世前驱值）。
    pub head_hash: String,
    /// **未纳管**记录数（没有链字段的记录）。
    ///
    /// 语义：这些记录**不算坏链**（它们从来就不在链上），单独计数以便体检报告
    /// 说清「多少数据还没纳入保护范围」。
    pub unmanaged_entries: usize,
    /// 被检出异常的链上条目数（摘要不符 / 断链失联 / 分叉）。
    pub broken_entries: usize,
}

/// 汇总：有多少主体、多少条记录、多少条链坏了。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainSummary {
    /// 有链的主体数。
    pub subjects: usize,
    /// 链上条目总数（所有主体的链上条数之和）。
    pub entries: usize,
    /// 校验不通过的主体数。
    pub broken_subjects: usize,
    /// 被检出异常的链上条目数（所有主体之和）。
    pub broken_entries: usize,
    /// **未纳管**记录数（全库口径：没有链字段的记录）。
    pub unmanaged_entries: usize,
}

/// 可空 i64 的编码：存在位（1 字节）+ 大端 8 字节。
///
/// **为什么要存在位**：`NULL` 与「值恰好是 0」在定长编码下字节相同 ⇒ 摘要相同 ⇒
/// 「这条记录的有效期被从 0 改成空」检不出来。存在位把两者分开。
fn encode_opt_i64(value: Option<i64>) -> Vec<u8> {
    match value {
        None => vec![0u8],
        Some(inner) => {
            let mut out = Vec::with_capacity(9);
            out.push(1u8);
            out.extend_from_slice(&inner.to_be_bytes());
            out
        }
    }
}

/// 算一条记录的链摘要：字段内容 + 前驱摘要。
///
/// 字节布局见模块文档；前驱摘要是最后一个长度前缀字段 ⇒ 链接关系进摘要
/// ⇒ 改链头即改摘要。摘要算法与治理审计账本同源（`apeireth_core::hash_chain`）。
pub fn entry_digest(previous_hash: &str, entry: &ChainEntry) -> String {
    let timestamp = entry.timestamp.to_be_bytes();
    let valid_from = encode_opt_i64(entry.valid_from_ms);
    let valid_until = encode_opt_i64(entry.valid_until_ms);
    let created = encode_opt_i64(entry.created_ms);
    let provenance_present = [u8::from(entry.provenance.is_some())];
    hash_linked(
        previous_hash,
        &[
            Chunk::LenPrefixed(entry.id.as_bytes()),
            Chunk::LenPrefixed(entry.content.as_bytes()),
            Chunk::Raw(&timestamp),
            Chunk::LenPrefixed(entry.role.as_bytes()),
            Chunk::LenPrefixed(entry.session_id.as_bytes()),
            Chunk::Raw(&valid_from),
            Chunk::Raw(&valid_until),
            Chunk::Raw(&created),
            Chunk::Raw(&provenance_present),
            Chunk::LenPrefixed(entry.provenance.as_deref().unwrap_or_default().as_bytes()),
        ],
    )
}

/// `episodes` 表的列存在性快照。
///
/// **为什么每次都要查**：不同库（老库 / 只建了 6 列的后端 schema / 已迁移的新库）
/// 的列集不一样。摘要覆盖的字段依赖列是否存在 ⇒ 读写两侧必须用**同一份**列快照，
/// 否则写时算的摘要和查时重算的摘要对不上（满库误报篡改）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct EpisodeColumns {
    valid_from_ms: bool,
    valid_until_ms: bool,
    created_ms: bool,
    provenance: bool,
    chain_hash: bool,
    chain_prev_hash: bool,
}

impl EpisodeColumns {
    /// 读 `PRAGMA table_info(episodes)` 得到列快照。
    ///
    /// 表不存在时 `PRAGMA` 返回空集（不报错）⇒ 全部 `false` ⇒ 后续加列会**如实报错**
    /// （而不是静默跳过）。
    fn detect(conn: &Connection) -> MemoryResult<Self> {
        let mut stmt = conn.prepare("PRAGMA table_info(episodes)")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
        let mut columns = Self::default();
        for name in rows {
            match name?.as_str() {
                "valid_from_ms" => columns.valid_from_ms = true,
                "valid_until_ms" => columns.valid_until_ms = true,
                "created_ms" => columns.created_ms = true,
                "provenance" => columns.provenance = true,
                "chain_hash" => columns.chain_hash = true,
                "chain_prev_hash" => columns.chain_prev_hash = true,
                _ => {}
            }
        }
        Ok(columns)
    }
}

/// 确保链所需的 schema 存在（**幂等**）。
///
/// 做的事：
/// 1. 建 `memory_chain_heads` 表（`CREATE TABLE IF NOT EXISTS`）。
/// 2. 给 `episodes` 补 `chain_hash` / `chain_prev_hash` 两列
///    —— 先查 `PRAGMA table_info`，**没有才 `ALTER TABLE ADD COLUMN`** ⇒ 重复调用安全、
///    既有库继续可用。
///
/// **加列失败一律如实报错**：静默跳过 = 老库悄悄失去链保护（比没有链更糟 ——
/// 它会让人以为数据受保护）。
pub fn ensure_chain_schema(conn: &Connection) -> MemoryResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS memory_chain_heads (
             subject_id   TEXT PRIMARY KEY,
             last_hash    TEXT NOT NULL,
             entry_count  INTEGER NOT NULL DEFAULT 0,
             updated_at_ms INTEGER NOT NULL
         )",
    )?;
    let columns = EpisodeColumns::detect(conn)?;
    if !columns.chain_hash {
        conn.execute_batch("ALTER TABLE episodes ADD COLUMN chain_hash TEXT")?;
    }
    if !columns.chain_prev_hash {
        conn.execute_batch("ALTER TABLE episodes ADD COLUMN chain_prev_hash TEXT")?;
    }
    Ok(())
}

/// 写入一条**带链**的 episode。
///
/// 硬失败语义：链算不出来 / 落库失败 ⇒ 返回 `Err`，**绝不**「无链写入后静默跳过」。
/// 宁可拒写，不可无链写入（无链写入 = 缺口 G6 原地不动）。
///
/// 幂等：同 id 重复写**不重复入链**（链头不推进），返回 `chained: false`。
pub fn put_chained_episode(
    conn: &Connection,
    subject_id: &str,
    entry: &ChainEntry,
) -> MemoryResult<ChainWriteOutcome> {
    let subject = subject_id.trim();
    if subject.is_empty() {
        return Err(MemoryError::Invalid(
            "memory chain subject_id is empty (a chain needs a subject key)".into(),
        ));
    }
    if entry.id.trim().is_empty() {
        return Err(MemoryError::Invalid(
            "memory chain episode id is empty".into(),
        ));
    }
    ensure_chain_schema(conn)?;
    let columns = EpisodeColumns::detect(conn)?;
    if !columns.chain_hash || !columns.chain_prev_hash {
        return Err(MemoryError::Other(
            "memory chain columns are missing after schema ensure".into(),
        ));
    }

    // IMMEDIATE 事务：先读链头再写 ⇒ 读-改-写必须原子，否则并发写会算出同一个前驱
    // （两条记录指同一个前驱 = 链分叉）。
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;

    let head: Option<(String, i64)> = tx
        .query_row(
            "SELECT last_hash, entry_count FROM memory_chain_heads WHERE subject_id = ?1",
            params![subject],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let (previous_hash, entry_count) = match head {
        Some((hash, count)) => (hash, count.max(0)),
        None => (GENESIS_PREVIOUS_HASH.to_string(), 0i64),
    };

    let digest = entry_digest(&previous_hash, entry);
    let changed = insert_episode(&tx, subject, entry, &digest, &previous_hash, &columns)?;

    if changed == 0 {
        // 同 id 早已存在 ⇒ 不入链（否则链头会指向一条库里不存在的记录 = 自造断链）。
        let existing: Option<(Option<String>, Option<String>)> = tx
            .query_row(
                "SELECT chain_hash, chain_prev_hash FROM episodes WHERE id = ?1",
                params![entry.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        tx.commit()?;
        let (chain_hash, chain_prev_hash) = existing.unwrap_or((None, None));
        return Ok(ChainWriteOutcome {
            subject_id: subject.to_string(),
            episode_id: entry.id.clone(),
            chain_hash: chain_hash.unwrap_or_default(),
            chain_prev_hash: chain_prev_hash.unwrap_or_default(),
            entry_count: entry_count as u64,
            chained: false,
        });
    }

    let new_count = entry_count + 1;
    tx.execute(
        "INSERT INTO memory_chain_heads (subject_id, last_hash, entry_count, updated_at_ms)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(subject_id) DO UPDATE SET
             last_hash    = excluded.last_hash,
             entry_count  = excluded.entry_count,
             updated_at_ms = excluded.updated_at_ms",
        params![subject, digest, new_count, now_ms()],
    )?;
    tx.commit()?;

    Ok(ChainWriteOutcome {
        subject_id: subject.to_string(),
        episode_id: entry.id.clone(),
        chain_hash: digest.clone(),
        chain_prev_hash: previous_hash,
        entry_count: new_count as u64,
        chained: true,
    })
}

/// 一次 INSERT 落库（append-only 触发器只拦 UPDATE/DELETE ⇒ INSERT 安全）。
///
/// 只写**实际存在**的列：老库 / 6 列后端 schema 里没有 V4 4 列 ⇒ 那些字段按 `NULL`
/// 口径参与摘要（读写两侧一致，见 [`EpisodeColumns`]）。
fn insert_episode(
    conn: &Connection,
    subject_id: &str,
    entry: &ChainEntry,
    digest: &str,
    previous_hash: &str,
    columns: &EpisodeColumns,
) -> MemoryResult<usize> {
    let mut names: Vec<&str> = vec![
        "id",
        "continuity_id",
        "session_id",
        "timestamp",
        "role",
        "content",
    ];
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = vec![
        Box::new(entry.id.clone()),
        Box::new(subject_id.to_string()),
        Box::new(entry.session_id.clone()),
        Box::new(entry.timestamp),
        Box::new(entry.role.clone()),
        Box::new(entry.content.clone()),
    ];
    if columns.valid_from_ms {
        names.push("valid_from_ms");
        values.push(Box::new(entry.valid_from_ms));
    }
    if columns.valid_until_ms {
        names.push("valid_until_ms");
        values.push(Box::new(entry.valid_until_ms));
    }
    if columns.created_ms {
        names.push("created_ms");
        values.push(Box::new(entry.created_ms));
    }
    if columns.provenance {
        names.push("provenance");
        values.push(Box::new(entry.provenance.clone()));
    }
    names.push("chain_hash");
    values.push(Box::new(digest.to_string()));
    names.push("chain_prev_hash");
    values.push(Box::new(previous_hash.to_string()));

    let placeholders = (1..=names.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "INSERT INTO episodes ({}) VALUES ({}) ON CONFLICT(id) DO NOTHING",
        names.join(", "),
        placeholders
    );
    let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|value| value.as_ref()).collect();
    let changed = conn.execute(&sql, params.as_slice())?;
    Ok(changed)
}

/// 一条链的校验结果。
///
/// 校验内容：
/// ① 链首尾相连：顺着 `chain_prev_hash` 从创世前驱值出发逐条走，链必须能走通；
/// ② 每条的 `chain_hash` == 用当前内容重算出的摘要（内容被改过会被检出）；
/// ③ 链头账实相符：`memory_chain_heads` 的 `last_hash` / `entry_count` 与链的实际
///    状态一致（尾部被删 / 被塞进未纳管记录会被检出）。
///
/// **必须能定位到具体第几条**（`first_bad_index` / `first_bad_id`）。
///
/// **无链记录不算坏链**：没有链字段的记录视为「未纳管」，单独计数
/// （`unmanaged_entries`），不参与 `ok` 的判定。
pub fn verify_subject_chain(
    conn: &Connection,
    subject_id: &str,
) -> MemoryResult<ChainVerification> {
    let subject = subject_id.trim();
    let columns = EpisodeColumns::detect(conn)?;
    if !columns.chain_hash || !columns.chain_prev_hash {
        // 老库（没有链列）⇒ 该主体的记录全部「未纳管」：不是坏链，是没纳管。
        let unmanaged = count_episodes(conn, Some(subject))?;
        return Ok(ChainVerification {
            subject_id: subject.to_string(),
            entry_count: 0,
            ok: true,
            first_bad_index: None,
            first_bad_id: None,
            head_hash: GENESIS_PREVIOUS_HASH.to_string(),
            unmanaged_entries: unmanaged,
            broken_entries: 0,
        });
    }

    let rows = load_subject_rows(conn, subject, &columns)?;
    let mut chained: Vec<usize> = Vec::new();
    let mut unmanaged = 0usize;
    for (index, row) in rows.iter().enumerate() {
        if row.chain_hash.is_some() {
            chained.push(index);
        } else {
            unmanaged += 1;
        }
    }

    // 前驱摘要 → 候选行下标。按 canonical 顺序（timestamp, id）追加 ⇒ 走链时的
    // 「同前驱多候选」判定是确定性的。用 BTreeMap 而不是 HashMap：输出不得依赖
    // 哈希序（确定性纪律）。
    let mut by_prev: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for &index in &chained {
        let previous = rows[index].chain_prev_hash.as_deref().unwrap_or("");
        by_prev.entry(previous).or_default().push(index);
    }

    let mut visited = vec![false; rows.len()];
    let mut expected_previous = GENESIS_PREVIOUS_HASH.to_string();
    let mut chain_index = 0usize;
    let mut first_bad: Option<(usize, Option<String>)> = None;
    let mut broken_entries = 0usize;
    let mut head_hash = GENESIS_PREVIOUS_HASH.to_string();

    loop {
        let candidates: Vec<usize> = by_prev
            .get(expected_previous.as_str())
            .map(|list| {
                list.iter()
                    .copied()
                    .filter(|index| !visited[*index])
                    .collect()
            })
            .unwrap_or_default();
        if candidates.is_empty() {
            break;
        }
        if candidates.len() > 1 {
            // 分叉：同一个前驱有多条后继 ⇒ 有人插入/改写过链。
            // 取 canonical 顺序的第一条继续走，其余算坏条（并标记已访问，避免重复计数）。
            broken_entries += candidates.len() - 1;
            if first_bad.is_none() {
                first_bad = Some((chain_index, Some(rows[candidates[1]].id.clone())));
            }
            for &extra in &candidates[1..] {
                visited[extra] = true;
            }
        }
        let index = candidates[0];
        visited[index] = true;
        let row = &rows[index];
        let previous = row.chain_prev_hash.clone().unwrap_or_default();
        let recomputed = entry_digest(&previous, &row.entry);
        if row.chain_hash.as_deref() != Some(recomputed.as_str()) {
            // 摘要与内容对不上 ⇒ 内容被改过，或链字段被改过。
            broken_entries += 1;
            if first_bad.is_none() {
                first_bad = Some((chain_index, Some(row.id.clone())));
            }
        }
        head_hash = row.chain_hash.clone().unwrap_or_default();
        expected_previous = head_hash.clone();
        chain_index += 1;
    }

    // 走不到的链上记录 ⇒ 断链（中间被删 / 被换 / 被塞进来）。
    let unlinked: Vec<usize> = chained
        .iter()
        .copied()
        .filter(|index| !visited[*index])
        .collect();
    if !unlinked.is_empty() {
        broken_entries += unlinked.len();
        if first_bad.is_none() {
            first_bad = Some((chain_index, Some(rows[unlinked[0]].id.clone())));
        }
    }

    // 链头账实相符：尾部被删（链少一条但链头还记着 N 条）会被这里抓到。
    let mut ok = first_bad.is_none();
    if let Some((last_hash, count)) = read_chain_head(conn, subject)? {
        if last_hash != head_hash || count.max(0) as usize != chained.len() {
            ok = false;
            if first_bad.is_none() {
                first_bad = Some((chain_index, None));
            }
        }
    }

    Ok(ChainVerification {
        subject_id: subject.to_string(),
        entry_count: chained.len(),
        ok,
        first_bad_index: first_bad.as_ref().map(|(index, _)| *index),
        first_bad_id: first_bad.as_ref().and_then(|(_, id)| id.clone()),
        head_hash,
        unmanaged_entries: unmanaged,
        broken_entries,
    })
}

/// 校验全部主体（返回每主体的结果，按 `subject_id` 排序 ⇒ 确定性）。
///
/// 主体集合 = 「链头表里有」∪「有链上记录的主体」，排序后逐个校验。
pub fn verify_all_chains(conn: &Connection) -> MemoryResult<Vec<ChainVerification>> {
    let mut subjects: Vec<String> = Vec::new();
    if table_exists(conn, "memory_chain_heads")? {
        let mut stmt = conn.prepare("SELECT subject_id FROM memory_chain_heads")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for subject in rows {
            subjects.push(subject?);
        }
    }
    let columns = EpisodeColumns::detect(conn)?;
    if columns.chain_hash {
        let mut stmt = conn
            .prepare("SELECT DISTINCT continuity_id FROM episodes WHERE chain_hash IS NOT NULL")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for subject in rows {
            subjects.push(subject?);
        }
    }
    subjects.sort();
    subjects.dedup();

    let mut results = Vec::with_capacity(subjects.len());
    for subject in subjects {
        results.push(verify_subject_chain(conn, &subject)?);
    }
    Ok(results)
}

/// 汇总：有多少主体、多少条记录、多少条链坏了。
///
/// `entries` = 链上条目总数；`unmanaged_entries` = 全库未纳管记录数
/// （两者之和 = `episodes` 总行数）；`broken_entries` = 被检出异常的链上条目数。
pub fn summarize(conn: &Connection) -> MemoryResult<ChainSummary> {
    let results = verify_all_chains(conn)?;
    let mut summary = ChainSummary {
        subjects: 0,
        entries: 0,
        broken_subjects: 0,
        broken_entries: 0,
        unmanaged_entries: 0,
    };
    for result in &results {
        summary.subjects += 1;
        summary.entries += result.entry_count;
        summary.broken_entries += result.broken_entries;
        if !result.ok {
            summary.broken_subjects += 1;
        }
    }
    // 未纳管是**全库**口径：某个主体整条链都没建时，它在上面的主体集合里根本不出现，
    // 但它的记录仍然是「未纳管」⇒ 必须在这里单独数一次，不能漏。
    let columns = EpisodeColumns::detect(conn)?;
    summary.unmanaged_entries = if columns.chain_hash {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM episodes WHERE chain_hash IS NULL",
            [],
            |row| row.get(0),
        )?;
        count.max(0) as usize
    } else {
        // 连链列都没有 ⇒ 全部记录都是「未纳管」。
        count_episodes(conn, None)?
    };
    Ok(summary)
}

/// 校验某个主体的链（v1 存储的显式链入口）。
///
/// 与 [`crate::SqliteMemoryStore::put_episode_full`] 的差别只有一处：**多写链摘要**
/// （一次 INSERT 同时写 `chain_hash` / `chain_prev_hash`）。
///
/// **为什么是独立入口而不是改既有函数**：`EpisodeStore::put_episode` / `put_episode_full`
/// 的签名与语义被既有调用方依赖（它们的产出是「未纳管」记录）。把链做成显式入口，
/// 调用方可以选择「这条要不要纳入链保护」，不必在一次改动里改变所有既有写入行为。
///
/// `meta` 显式给出 ⇒ 来源（`provenance`）由调用方负责（不猜）。
impl crate::SqliteMemoryStore {
    /// 写入一条**带链**的 episode（v1 存储的显式链入口）。
    pub fn put_episode_chained(
        &self,
        episode: &apeireth_core::kernel::memory::Episode,
        meta: &crate::EpisodeMeta,
        subject_id: &str,
    ) -> MemoryResult<ChainWriteOutcome> {
        if episode.id.trim().is_empty() {
            return Err(MemoryError::Invalid("episode id is empty".into()));
        }
        if episode.session_id.trim().is_empty() {
            return Err(MemoryError::Invalid("episode session_id is empty".into()));
        }
        if episode.role.trim().is_empty() {
            return Err(MemoryError::Invalid("episode role is empty".into()));
        }
        let conn = self.conn()?;
        let entry = ChainEntry::from_episode(episode, Some(meta));
        put_chained_episode(&conn, subject_id, &entry)
    }
}

/// 一个主体在 `episodes` 里的全部行（canonical 顺序：timestamp, id）。
struct SubjectRow {
    id: String,
    entry: ChainEntry,
    chain_hash: Option<String>,
    chain_prev_hash: Option<String>,
}

/// 读某主体的全部 episode 行（canonical 顺序 ⇒ 确定性）。
fn load_subject_rows(
    conn: &Connection,
    subject: &str,
    columns: &EpisodeColumns,
) -> MemoryResult<Vec<SubjectRow>> {
    let mut sql = String::from(
        "SELECT id, content, timestamp, role, session_id, chain_hash, chain_prev_hash",
    );
    if columns.valid_from_ms {
        sql.push_str(", valid_from_ms");
    }
    if columns.valid_until_ms {
        sql.push_str(", valid_until_ms");
    }
    if columns.created_ms {
        sql.push_str(", created_ms");
    }
    if columns.provenance {
        sql.push_str(", provenance");
    }
    sql.push_str(" FROM episodes WHERE continuity_id = ?1 ORDER BY timestamp ASC, id ASC");

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![subject], |row| {
        let id: String = row.get(0)?;
        let content: String = row.get(1)?;
        let timestamp: i64 = row.get(2)?;
        let role: String = row.get(3)?;
        let session_id: String = row.get(4)?;
        let chain_hash: Option<String> = row.get(5)?;
        let chain_prev_hash: Option<String> = row.get(6)?;
        let mut cursor = 7usize;
        let mut valid_from_ms = None;
        if columns.valid_from_ms {
            valid_from_ms = row.get(cursor)?;
            cursor += 1;
        }
        let mut valid_until_ms = None;
        if columns.valid_until_ms {
            valid_until_ms = row.get(cursor)?;
            cursor += 1;
        }
        let mut created_ms = None;
        if columns.created_ms {
            created_ms = row.get(cursor)?;
            cursor += 1;
        }
        let mut provenance = None;
        if columns.provenance {
            provenance = row.get(cursor)?;
        }
        Ok(SubjectRow {
            id: id.clone(),
            entry: ChainEntry {
                id,
                content,
                timestamp,
                role,
                session_id,
                valid_from_ms,
                valid_until_ms,
                created_ms,
                provenance,
            },
            chain_hash,
            chain_prev_hash,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// 数 `episodes` 行数（`subject` 为 `None` ⇒ 全库）。
fn count_episodes(conn: &Connection, subject: Option<&str>) -> MemoryResult<usize> {
    let count: i64 = match subject {
        Some(subject) => conn.query_row(
            "SELECT COUNT(*) FROM episodes WHERE continuity_id = ?1",
            params![subject],
            |row| row.get(0),
        )?,
        None => conn.query_row("SELECT COUNT(*) FROM episodes", [], |row| row.get(0))?,
    };
    Ok(count.max(0) as usize)
}

/// 表是否存在（`sqlite_master` 判定）。
fn table_exists(conn: &Connection, name: &str) -> MemoryResult<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        params![name],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// 读某主体的链头（`last_hash` / `entry_count`）。
///
/// **表不存在 ⇒ `None`**（不是错误）：老库里根本没有链头表，校验仍要能跑
/// （只是没有「链头账实相符」这条额外判据）。
fn read_chain_head(conn: &Connection, subject: &str) -> MemoryResult<Option<(String, i64)>> {
    if !table_exists(conn, "memory_chain_heads")? {
        return Ok(None);
    }
    let head = conn
        .query_row(
            "SELECT last_hash, entry_count FROM memory_chain_heads WHERE subject_id = ?1",
            params![subject],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    Ok(head)
}

/// 当前时间（epoch 毫秒）—— 只写进链头表的 `updated_at_ms`（元数据，不进摘要）。
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or(0)
}
