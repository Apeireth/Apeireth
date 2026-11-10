//! `apeireth-memory::offline_archive` — 记忆数据的**脱机归档**（保原文 + 可读 + 自说明）。
//!
//! # 为什么
//!
//! §2 原则 **P1**：原始证据**永久保留** ⇒ 数据不能只活在 SQLite 里
//! ⇒ 万一库坏了/被换格式/要交给人看 ⇒ **必须**有一份**脱机可读**的副本。
//!
//! # 三条纪律
//!
//! 1. **保原文**：`content` **逐字**导出（**不许**摘要、不许截断、不许改写）；
//! 2. **自说明**：导出物**自带** schema 说明（字段含义、版本、时间语义）
//!    ⇒ 三年后没人记得这些字段啥意思也能读；
//! 3. **可验证**：导出后能**独立校验**（条数 + 内容哈希）⇒ 知道有没有丢。
//!
//! # 与「发行包」的区别（别混）
//!
//! `crates/adapters/cli/src/portable_bundle.rs` 是**发行包**（启动器 + 配置），
//! **不是**记忆数据归档 ⇒ 本模块是**数据面**的：导出 `episodes` 主表 + 链头账本。
//!
//! # 口径（钉死，别自创）
//!
//! * **排序确定性**：按 `(timestamp, id)` 升序 ⇒ 同库两次导出**逐字相等**；
//! * **导出时刻是注入的**（[`export_archive`] 的 `created_at_ms`）⇒ 导出物可复现；
//! * **只读**：本模块**只 SELECT** ⇒ 不改库里任何东西；
//! * **不纳管如实标出**：没有链摘要的记录（`chain_hash IS NULL`）在导出物里
//!   **照实导出**（`chain_hash = null`）⇒ 读方知道哪些数据没在哈希链保护范围内。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-26 首版 · 2026-10-27 重做接线）
//!
//! 四级口径（本模块的真实位置）：
//!
//! - **IMPLEMENTED** ✔：导出 + 自说明 + 独立校验 + JSONL 往返齐备（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：**未接入**任何生产路径（无 CLI、无定时任务、生产 0 调用点）；
//! - **DEFAULT ENABLED** ✘：没有开关位，也未默认启用；
//! - **HARDWARE VALIDATED** ✘：只在库级测试与**真实 schema** 造库上跑过，
//!   **未在生产记忆库上实测**（本机无生产库）⇒ 不宣称已验证。
//!
//! 对外一律按 **NOT IMPLEMENTED** 表述（新增未接线，不宣称可用）。
//!
//! **B4 的边界（别越界宣称）**：本模块只做 B4 的第一块砖（**脱机可读**）；
//! **迁移回滚** 与 **handover 演练**属 B4 的后两块 ⇒ **NOT IMPLEMENTED**（本批不做）。

use std::collections::BTreeMap;

use rusqlite::{params, Connection};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

/// 导出格式版本（**改了格式就要升** ⇒ 读方能判断能不能读）。
pub const ARCHIVE_FORMAT_VERSION: &str = "apeireth-offline-archive/v1";

/// 一条记忆（**脱机形态**）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ArchivedEpisode {
    /// 记录 id。
    pub id: String,
    /// 主体（`continuity_id`）。
    pub continuity_id: String,
    /// 会话 id。
    pub session_id: Option<String>,
    /// 发生时间（epoch 秒）。
    pub timestamp: i64,
    /// 说话人 / 角色。
    pub role: String,
    /// **原文（逐字，不改写）**。
    pub content: String,
    /// 生效起点（epoch 毫秒；`None` = 未记录）。
    pub valid_from_ms: Option<i64>,
    /// 失效时间（epoch 毫秒；`None` = 永久有效）。
    pub valid_until_ms: Option<i64>,
    /// 记录时间（epoch 毫秒；`None` = 未记录）。
    pub created_ms: Option<i64>,
    /// 出处（`None` = 未记录）。
    pub provenance: Option<String>,
    /// 链摘要（`None` = **未纳管** ⇒ 导出物里也要**如实标出来**）。
    pub chain_hash: Option<String>,
    /// 链前驱。
    pub chain_prev_hash: Option<String>,
}

/// 一条链头（链的账本 ⇒ 导出物要能**独立核对**每条链）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ArchivedChainHead {
    /// 主体 id（链的键）。
    pub subject_id: String,
    /// 链头摘要。
    pub last_hash: String,
    /// 链上条目数（**账** ⇒ 用来核对「导出物里的链上条数」）。
    pub entry_count: u64,
    /// 链头更新时间（epoch 毫秒）。
    pub updated_at_ms: i64,
}

/// **自说明**：这个导出物怎么读（**随物走** ⇒ 三年后仍可读）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ArchiveManifest {
    /// 格式版本（[`ARCHIVE_FORMAT_VERSION`]）。
    pub format_version: String,
    /// 导出时刻（epoch 毫秒；**注入** ⇒ 确定性）。
    pub created_at_ms: i64,
    /// 记录条数。
    pub episode_count: usize,
    /// 链头条数。
    pub chain_head_count: usize,
    /// **未纳管**条数（**如实标出** ⇒ 读方知道这不是全量可信的账）。
    ///
    /// **口径**：恒等于**导出物里**没有链摘要的记录条数
    /// （`include_unmanaged = false` 时那些记录被跳过 ⇒ 导出物里没有 ⇒ 计数为 0）。
    pub unmanaged_count: usize,
    /// 字段说明（字段名 → 含义）。
    pub field_docs: BTreeMap<String, String>,
    /// **内容哈希**（对**正文**算 ⇒ 可独立校验「没丢字」）。
    pub content_digest: String,
}

/// 一个完整的脱机归档物。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OfflineArchive {
    /// 自说明清单。
    pub manifest: ArchiveManifest,
    /// 记录（按 `(timestamp, id)` 升序）。
    pub episodes: Vec<ArchivedEpisode>,
    /// 链头账本（按 `subject_id` 升序）。
    pub chain_heads: Vec<ArchivedChainHead>,
}

/// 导出配置。
///
/// ⚠️ **不要 `Copy`**：`subject` 是 `Option<String>` ⇒ `Option<String>` **不是 `Copy`**
/// ⇒ 派生了 `Copy` 就**编译不过**（昨夜那一版就栽在这里 ⇒ 别再加回去）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportConfig {
    /// 只导出某个主体（`None` = 全部）。
    pub subject: Option<String>,
    /// 上限（条数；`0` = 不限）。
    pub limit: usize,
    /// 导出**未纳管**的记录吗（默认**要** ⇒ 如实 ⇒ 读方知道哪些没链）。
    pub include_unmanaged: bool,
}

impl ExportConfig {
    /// 构造（全量）。
    pub fn new() -> Self {
        Self {
            subject: None,
            limit: 0,
            include_unmanaged: true,
        }
    }
}

impl Default for ExportConfig {
    /// 与 [`ExportConfig::new`] 同口径（全量 + 含未纳管）。
    fn default() -> Self {
        Self::new()
    }
}

/// 从库导出（**只读** ⇒ 不改任何东西）。
///
/// # 口径
///
/// * **逐字**导出 `content`（**保原文**）；
/// * **排序确定性**：按 `(timestamp, id)` 升序；
/// * `include_unmanaged = false` ⇒ **跳过**无链记录（manifest 的 `unmanaged_count`
///   仍**如实**记导出物里剩下的无链条数 ⇒ 不说谎）；
/// * `subject` ⇒ 只导该主体（链头也只导该主体的）；
/// * `limit` ⇒ 截断（**manifest 里的条数是实际导出的条数** ⇒ 不许说谎）。
///
/// # 返回
///
/// * `Ok(archive)` = 导出物；
/// * `Err(reason)` = 如实报错（**不 panic**）。
///
/// # 边界（诚实说明）
///
/// * 库没有 `episodes` 表（老库 / 裸连接）⇒ 导出**空归档**（不是错：没有记忆就是空）；
/// * 库没有 V4/V19 的列（老库）⇒ 那些字段按 `NULL` 导出（读方看到 `null` = 未记录）。
pub fn export_archive(
    conn: &Connection,
    config: &ExportConfig,
    created_at_ms: i64,
) -> Result<OfflineArchive, String> {
    let episodes = read_episodes(conn, config)?;
    let chain_heads = read_chain_heads(conn, config)?;
    let unmanaged_count = episodes
        .iter()
        .filter(|episode| episode.chain_hash.is_none())
        .count();
    Ok(OfflineArchive {
        manifest: ArchiveManifest {
            format_version: ARCHIVE_FORMAT_VERSION.to_string(),
            created_at_ms,
            episode_count: episodes.len(),
            chain_head_count: chain_heads.len(),
            unmanaged_count,
            field_docs: default_field_docs(),
            content_digest: content_digest(&episodes),
        },
        episodes,
        chain_heads,
    })
}

/// **自说明**的字段说明表（**每次导出都带上** ⇒ 不许省略）。
pub fn default_field_docs() -> BTreeMap<String, String> {
    let mut docs = BTreeMap::new();
    docs.insert(
        "id".to_string(),
        "记录主键（来自 episodes.id；同一个库内唯一）".to_string(),
    );
    docs.insert(
        "continuity_id".to_string(),
        "主体 id（来自 episodes.continuity_id；跨载体唯一 ⇒ 同一主体的记忆聚在一起）".to_string(),
    );
    docs.insert(
        "session_id".to_string(),
        "会话 id（来自 episodes.session_id；null = 未记录）".to_string(),
    );
    docs.insert(
        "timestamp".to_string(),
        "发生时间（来自 episodes.timestamp；**epoch 秒**）".to_string(),
    );
    docs.insert(
        "role".to_string(),
        "说话人 / 角色（来自 episodes.role；user / assistant / system / tool）".to_string(),
    );
    docs.insert(
        "content".to_string(),
        "**原文**（来自 episodes.content；**逐字**导出：不摘要、不截断、不改写）".to_string(),
    );
    docs.insert(
        "valid_from_ms".to_string(),
        "生效起点（来自 episodes.valid_from_ms；**epoch 毫秒**；null = 未记录）".to_string(),
    );
    docs.insert(
        "valid_until_ms".to_string(),
        "失效时间（来自 episodes.valid_until_ms；**epoch 毫秒**；null = 永久有效）".to_string(),
    );
    docs.insert(
        "created_ms".to_string(),
        "记录时间（来自 episodes.created_ms；**epoch 毫秒**；null = 未记录）".to_string(),
    );
    docs.insert(
        "provenance".to_string(),
        "出处（来自 episodes.provenance 原文；dialog / tool / reflection / observation / manual；null = 未记录）"
            .to_string(),
    );
    docs.insert(
        "chain_hash".to_string(),
        "链摘要（来自 episodes.chain_hash；null = **未纳管** ⇒ 这条记录不在哈希链保护范围内）"
            .to_string(),
    );
    docs.insert(
        "chain_prev_hash".to_string(),
        "链前驱摘要（来自 episodes.chain_prev_hash；null = 未纳管）".to_string(),
    );
    docs.insert(
        "subject_id".to_string(),
        "链的主体键（来自 memory_chain_heads.subject_id）".to_string(),
    );
    docs.insert(
        "last_hash".to_string(),
        "链头摘要（来自 memory_chain_heads.last_hash）".to_string(),
    );
    docs.insert(
        "entry_count".to_string(),
        "链上条目数（来自 memory_chain_heads.entry_count ⇒ 用来核对导出物里的链上条数）"
            .to_string(),
    );
    docs.insert(
        "updated_at_ms".to_string(),
        "链头更新时间（来自 memory_chain_heads.updated_at_ms；**epoch 毫秒**）".to_string(),
    );
    docs.insert(
        "format_version".to_string(),
        format!("导出格式版本（本格式 = {ARCHIVE_FORMAT_VERSION}；读方先看它 ⇒ 知道能不能读）"),
    );
    docs.insert(
        "created_at_ms".to_string(),
        "导出时刻（**epoch 毫秒**；由调用方注入 ⇒ 导出物可复现）".to_string(),
    );
    docs.insert(
        "episode_count".to_string(),
        "本导出物里的记录条数（**实际导出**的条数，不是库里的总条数）".to_string(),
    );
    docs.insert(
        "chain_head_count".to_string(),
        "本导出物里的链头条数".to_string(),
    );
    docs.insert(
        "unmanaged_count".to_string(),
        "本导出物里**未纳管**（没有链摘要）的记录条数".to_string(),
    );
    docs.insert(
        "content_digest".to_string(),
        "内容哈希（对**正文**算：SHA-256 ⇒ 可独立校验「没丢字」「没重排」）".to_string(),
    );
    docs
}

/// 对**正文**算内容哈希（**确定性** ⇒ 同内容同哈希）。
///
/// # 口径
///
/// 逐条 `id` + `content` 依次喂入 ⇒ 顺序敏感（**顺序变了哈希就变** ⇒ 能查出重排）。
///
/// **具体算法**：`SHA-256`，前像 = 对每条记录依次拼接
/// `u64_be(len(id)) ‖ id_bytes ‖ u64_be(len(content)) ‖ content_bytes`
/// （长度是 **UTF-8 字节数**；长度前缀避免拼接歧义 —— `["ab","c"]` 与 `["a","bc"]`
/// 直接拼出来的字节相同 ⇒ 不加前缀就分不清 ⇒ 哈希会撞）。
/// 输出 = **小写 hex**（64 个字符）。
///
/// **只覆盖 `id` 与 `content`**：这是「**没丢字**」这一条纪律要的最小口径
/// （正文逐字 + 记录身份）⇒ 改一个字、改一条顺序、换一条记录，都会被检出。
///
/// **空归档的哈希 = SHA-256(空)**（不是空串 ⇒ 空与非空能区分）。
pub fn content_digest(episodes: &[ArchivedEpisode]) -> String {
    let mut hasher = Sha256::new();
    for episode in episodes {
        hasher.update((episode.id.len() as u64).to_be_bytes());
        hasher.update(episode.id.as_bytes());
        hasher.update((episode.content.len() as u64).to_be_bytes());
        hasher.update(episode.content.as_bytes());
    }
    hex_encode(&hasher.finalize())
}

/// **独立校验**：核对一份归档物是否**自洽**（不依赖库）。
///
/// # 检查
///
/// * manifest 的 `episode_count` == 实际条数；
/// * manifest 的 `content_digest` == 重算的哈希（⇒ **没丢字**）；
/// * 每条链：链上条目数 == 对应链头的 `entry_count`（⇒ **账实相符**）；
/// * 无链记录数 == manifest 的 `unmanaged_count`。
///
/// **外加一条**（同属「不许说谎」）：manifest 的 `chain_head_count` == 实际链头条数
/// （链头不许被偷偷少导 ⇒ 否则「链账核对」就成了空判据）。
///
/// # 返回
///
/// * `Ok(())` = 自洽；
/// * `Err(reason)` = 不自洽（**如实**，说明差在哪）。
///
/// # 已知边界（**如实**说明，别当 bug）
///
/// * **截断导出（`limit` < 全量）按设计**通不过「链账核对」：
///   导出物里链上条数必然少于链头记的 `entry_count` ⇒ 报 `Err`
///   ⇒ 这正是「这份归档不完整」的**信号**（不是误报）。
/// * 同理，`subject` 过滤 + `limit` 截断也会撞上这条。
pub fn verify_archive(archive: &OfflineArchive) -> Result<(), String> {
    let manifest = &archive.manifest;
    if manifest.episode_count != archive.episodes.len() {
        return Err(format!(
            "条数不符：manifest.episode_count = {}，实际导出 {} 条",
            manifest.episode_count,
            archive.episodes.len()
        ));
    }
    if manifest.chain_head_count != archive.chain_heads.len() {
        return Err(format!(
            "链头条数不符：manifest.chain_head_count = {}，实际导出 {} 条",
            manifest.chain_head_count,
            archive.chain_heads.len()
        ));
    }
    let recomputed = content_digest(&archive.episodes);
    if manifest.content_digest != recomputed {
        return Err(format!(
            "内容哈希不符：manifest = {}，重算 = {}（正文被改过 / 顺序被重排 / 记录被换过）",
            manifest.content_digest, recomputed
        ));
    }

    // 链账核对：每条链上记录数 == 对应链头的 entry_count。
    let mut chained_per_subject: BTreeMap<&str, usize> = BTreeMap::new();
    for episode in &archive.episodes {
        if episode.chain_hash.is_some() {
            *chained_per_subject
                .entry(episode.continuity_id.as_str())
                .or_insert(0) += 1;
        }
    }
    for head in &archive.chain_heads {
        let actual = chained_per_subject
            .get(head.subject_id.as_str())
            .copied()
            .unwrap_or(0);
        if actual as u64 != head.entry_count {
            return Err(format!(
                "链账不符：主体 {} 的链上条目数 = {}，链头 entry_count = {}",
                head.subject_id, actual, head.entry_count
            ));
        }
    }
    // 反向核对：有链上记录却没有链头 ⇒ 账本漏了这条链（**塞进链里的记录查不出来**）。
    for (subject, count) in &chained_per_subject {
        if !archive
            .chain_heads
            .iter()
            .any(|head| head.subject_id == *subject)
        {
            return Err(format!(
                "链账不符：主体 {subject} 有 {count} 条链上记录，却没有对应的链头"
            ));
        }
    }

    let unmanaged = archive
        .episodes
        .iter()
        .filter(|episode| episode.chain_hash.is_none())
        .count();
    if unmanaged != manifest.unmanaged_count {
        return Err(format!(
            "未纳管条数不符：manifest = {}，实际 = {}",
            manifest.unmanaged_count, unmanaged
        ));
    }
    Ok(())
}

/// 导出成 **JSONL 文本**（**一行一条** ⇒ 可 `grep`、可 `jq`、可被任何工具读）。
///
/// ⚠️ 第一行是 **manifest**（`"kind": "manifest"`），其后每行一条记录
/// （`"kind": "episode"`）与链头（`"kind": "chain_head"`）。
///
/// **行内字段顺序** = `serde_json` 的键序（字典序）⇒ **同一份归档物 ⇒ 同一份文本**
/// （确定性 ⇒ 可 diff、可复核）。
pub fn to_jsonl(archive: &OfflineArchive) -> String {
    let mut out = String::new();
    out.push_str(&manifest_to_json(&archive.manifest));
    out.push('\n');
    for episode in &archive.episodes {
        out.push_str(&episode_to_json(episode));
        out.push('\n');
    }
    for head in &archive.chain_heads {
        out.push_str(&chain_head_to_json(head));
        out.push('\n');
    }
    out
}

/// 从 JSONL **读回来**（往返 ⇒ 导出物可被程序消费）。
///
/// # 口径
///
/// * 第一条**非空**行必须是 manifest（否则 `Err`）；
/// * 其后每行按 `"kind"` 分派（`episode` / `chain_head`）⇒ 未知 `kind` ⇒ `Err`；
/// * **只做结构还原**，**不校验自洽**（校验是 [`verify_archive`] 的活儿）
///   ⇒ 读回来一份坏归档物**不会**在这里被悄悄「修好」；
/// * **未知格式版本显式拒**（设计稿 §2 原则 **P4**「不猜着读」）⇒ 读到别的
///   `format_version` 直接 `Err`，不做「尽力而为」的猜测解析。
pub fn from_jsonl(text: &str) -> Result<OfflineArchive, String> {
    let mut manifest: Option<ArchiveManifest> = None;
    let mut episodes: Vec<ArchivedEpisode> = Vec::new();
    let mut chain_heads: Vec<ArchivedChainHead> = Vec::new();

    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line)
            .map_err(|error| format!("第 {} 行不是合法 JSON：{error}", index + 1))?;
        let kind = value
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("第 {} 行缺 \"kind\"", index + 1))?;
        match kind {
            "manifest" => {
                if manifest.is_some() {
                    return Err(format!("第 {} 行：出现了第二个 manifest", index + 1));
                }
                manifest = Some(manifest_from_json(&value)?);
            }
            "episode" => episodes.push(episode_from_json(&value)?),
            "chain_head" => chain_heads.push(chain_head_from_json(&value)?),
            other => return Err(format!("第 {} 行：未知 kind \"{other}\"", index + 1)),
        }
    }

    let manifest = manifest.ok_or_else(|| "缺 manifest 行（第一行必须是 manifest）".to_string())?;
    if manifest.format_version != ARCHIVE_FORMAT_VERSION {
        return Err(format!(
            "归档格式版本不符：期望 `{ARCHIVE_FORMAT_VERSION}`，实际 `{}`（未知版本不猜着读）",
            manifest.format_version
        ));
    }
    Ok(OfflineArchive {
        manifest,
        episodes,
        chain_heads,
    })
}

// ============================================================
// 内部实现
// ============================================================

/// `episodes` 表的**列存在性快照**。
///
/// **为什么要每次都查**：`episodes` 的 V4 四列（`valid_from_ms` / `valid_until_ms` /
/// `created_ms` / `provenance`）与 V19 两列（`chain_hash` / `chain_prev_hash`）
/// 是**后加**的 ⇒ 老库 / 别的后端 schema 里可能没有 ⇒ SQL 若假设了不存在的列
/// 就会直接报错（本仓吃过这个亏）⇒ 读写两侧按**同一份**列快照来。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct EpisodeColumns {
    /// `episodes` 表本身在不在。
    exists: bool,
    /// 生效起点列。
    valid_from_ms: bool,
    /// 失效时间列。
    valid_until_ms: bool,
    /// 记录时间列。
    created_ms: bool,
    /// 出处列。
    provenance: bool,
    /// 链摘要列。
    chain_hash: bool,
    /// 链前驱列。
    chain_prev_hash: bool,
}

impl EpisodeColumns {
    /// 读 `PRAGMA table_info(episodes)` 得到列快照。
    ///
    /// 表不存在时 `PRAGMA` 返回空集（不报错）⇒ `exists = false` ⇒ 导出空归档。
    fn detect(conn: &Connection) -> Result<Self, String> {
        let mut columns = Self {
            exists: table_exists(conn, "episodes")?,
            ..Self::default()
        };
        if !columns.exists {
            return Ok(columns);
        }
        let mut stmt = conn
            .prepare("PRAGMA table_info(episodes)")
            .map_err(|error| format!("读 episodes 列集失败：{error}"))?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|error| format!("读 episodes 列集失败：{error}"))?;
        for name in rows {
            let name = name.map_err(|error| format!("读 episodes 列集失败：{error}"))?;
            match name.as_str() {
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

/// 读 `episodes`（**只读**）⇒ 导出物里的记录。
fn read_episodes(conn: &Connection, config: &ExportConfig) -> Result<Vec<ArchivedEpisode>, String> {
    let columns = EpisodeColumns::detect(conn)?;
    if !columns.exists {
        // 没有 `episodes` 表 ⇒ 这个库里没有记忆 ⇒ 导出**空归档**（不是错）。
        return Ok(Vec::new());
    }

    let mut sql = String::from("SELECT id, continuity_id, session_id, timestamp, role, content");
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
    if columns.chain_hash {
        sql.push_str(", chain_hash");
    }
    if columns.chain_prev_hash {
        sql.push_str(", chain_prev_hash");
    }
    sql.push_str(" FROM episodes");

    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    let mut has_where = false;
    if let Some(subject) = &config.subject {
        sql.push_str(" WHERE continuity_id = ?");
        args.push(Box::new(subject.clone()));
        has_where = true;
    }
    if !config.include_unmanaged {
        if columns.chain_hash {
            if has_where {
                sql.push_str(" AND chain_hash IS NOT NULL");
            } else {
                sql.push_str(" WHERE chain_hash IS NOT NULL");
            }
        } else {
            // 库里**没有链列** ⇒ 全部记录都算「未纳管」⇒ `include_unmanaged = false`
            // ⇒ 一条都不该导（如实：不是「导了但标成有链」）。
            return Ok(Vec::new());
        }
    }
    // 排序确定性：`(timestamp, id)` 升序 ⇒ 同库两次导出**逐字相等**。
    sql.push_str(" ORDER BY timestamp ASC, id ASC");
    if config.limit > 0 {
        sql.push_str(" LIMIT ?");
        args.push(Box::new(config.limit as i64));
    }

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|error| format!("准备导出 SQL 失败：{error}"))?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|value| value.as_ref()).collect();
    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            // `id` / `role` / `content` 是 NOT NULL 列 ⇒ 直接读 String；
            // `continuity_id` / `session_id` 读成 Option（老库可能为空）⇒ 空则归一成空串
            // （**如实**：读方看到空串 = 没有主体/会话，而不是崩掉）。
            let id: String = row.get(0)?;
            let continuity_id: Option<String> = row.get(1)?;
            let session_id: Option<String> = row.get(2)?;
            let timestamp: i64 = row.get(3)?;
            let role: String = row.get(4)?;
            let content: String = row.get(5)?;
            let mut cursor = 6usize;
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
                cursor += 1;
            }
            let mut chain_hash = None;
            if columns.chain_hash {
                chain_hash = row.get(cursor)?;
                cursor += 1;
            }
            let mut chain_prev_hash = None;
            if columns.chain_prev_hash {
                chain_prev_hash = row.get(cursor)?;
            }
            Ok(ArchivedEpisode {
                id,
                continuity_id: continuity_id.unwrap_or_default(),
                session_id,
                timestamp,
                role,
                content,
                valid_from_ms,
                valid_until_ms,
                created_ms,
                provenance,
                chain_hash,
                chain_prev_hash,
            })
        })
        .map_err(|error| format!("执行导出 SQL 失败：{error}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|error| format!("读 episodes 行失败：{error}"))?);
    }
    Ok(out)
}

/// 读 `memory_chain_heads`（**只读**）⇒ 导出物里的链头账本。
fn read_chain_heads(
    conn: &Connection,
    config: &ExportConfig,
) -> Result<Vec<ArchivedChainHead>, String> {
    if !table_exists(conn, "memory_chain_heads")? {
        // 没有链头表 ⇒ 这个库**没有链保护账本**（老库）⇒ 如实导出空账本。
        return Ok(Vec::new());
    }
    let mut sql = String::from(
        "SELECT subject_id, last_hash, entry_count, updated_at_ms FROM memory_chain_heads",
    );
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(subject) = &config.subject {
        sql.push_str(" WHERE subject_id = ?");
        args.push(Box::new(subject.clone()));
    }
    sql.push_str(" ORDER BY subject_id ASC");

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|error| format!("准备链头 SQL 失败：{error}"))?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|value| value.as_ref()).collect();
    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            Ok(ArchivedChainHead {
                subject_id: row.get(0)?,
                last_hash: row.get(1)?,
                entry_count: row.get::<_, i64>(2)?.max(0) as u64,
                updated_at_ms: row.get(3)?,
            })
        })
        .map_err(|error| format!("执行链头 SQL 失败：{error}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|error| format!("读 memory_chain_heads 行失败：{error}"))?);
    }
    Ok(out)
}

/// 字节数组 ⇒ **小写 hex**。
fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from_digit(u32::from(byte >> 4), 16).unwrap_or('0'));
        out.push(char::from_digit(u32::from(byte & 0x0f), 16).unwrap_or('0'));
    }
    out
}

/// manifest ⇒ JSON 行。
fn manifest_to_json(manifest: &ArchiveManifest) -> String {
    let mut docs = Map::new();
    for (key, value) in &manifest.field_docs {
        docs.insert(key.clone(), Value::String(value.clone()));
    }
    json!({
        "kind": "manifest",
        "format_version": manifest.format_version,
        "created_at_ms": manifest.created_at_ms,
        "episode_count": manifest.episode_count,
        "chain_head_count": manifest.chain_head_count,
        "unmanaged_count": manifest.unmanaged_count,
        "field_docs": Value::Object(docs),
        "content_digest": manifest.content_digest,
    })
    .to_string()
}

/// 一条记录 ⇒ JSON 行。
fn episode_to_json(episode: &ArchivedEpisode) -> String {
    json!({
        "kind": "episode",
        "id": episode.id,
        "continuity_id": episode.continuity_id,
        "session_id": optional_string(&episode.session_id),
        "timestamp": episode.timestamp,
        "role": episode.role,
        "content": episode.content,
        "valid_from_ms": optional_i64(&episode.valid_from_ms),
        "valid_until_ms": optional_i64(&episode.valid_until_ms),
        "created_ms": optional_i64(&episode.created_ms),
        "provenance": optional_string(&episode.provenance),
        "chain_hash": optional_string(&episode.chain_hash),
        "chain_prev_hash": optional_string(&episode.chain_prev_hash),
    })
    .to_string()
}

/// 一条链头 ⇒ JSON 行。
fn chain_head_to_json(head: &ArchivedChainHead) -> String {
    json!({
        "kind": "chain_head",
        "subject_id": head.subject_id,
        "last_hash": head.last_hash,
        "entry_count": head.entry_count,
        "updated_at_ms": head.updated_at_ms,
    })
    .to_string()
}

/// `Option<String>` ⇒ JSON（`None` ⇒ `null`）。
fn optional_string(value: &Option<String>) -> Value {
    match value {
        Some(inner) => Value::String(inner.clone()),
        None => Value::Null,
    }
}

/// `Option<i64>` ⇒ JSON（`None` ⇒ `null`）。
fn optional_i64(value: &Option<i64>) -> Value {
    match value {
        Some(inner) => Value::Number((*inner).into()),
        None => Value::Null,
    }
}

/// manifest 行 ⇒ 结构体。
fn manifest_from_json(value: &Value) -> Result<ArchiveManifest, String> {
    let field_docs_value = value
        .get("field_docs")
        .and_then(Value::as_object)
        .ok_or_else(|| "manifest 缺 field_docs".to_string())?;
    let mut field_docs = BTreeMap::new();
    for (key, item) in field_docs_value {
        let text = item
            .as_str()
            .ok_or_else(|| format!("field_docs[{key}] 不是字符串"))?;
        field_docs.insert(key.clone(), text.to_string());
    }
    Ok(ArchiveManifest {
        format_version: require_string(value, "format_version")?,
        created_at_ms: require_i64(value, "created_at_ms")?,
        episode_count: require_usize(value, "episode_count")?,
        chain_head_count: require_usize(value, "chain_head_count")?,
        unmanaged_count: require_usize(value, "unmanaged_count")?,
        field_docs,
        content_digest: require_string(value, "content_digest")?,
    })
}

/// 记录行 ⇒ 结构体。
fn episode_from_json(value: &Value) -> Result<ArchivedEpisode, String> {
    Ok(ArchivedEpisode {
        id: require_string(value, "id")?,
        continuity_id: require_string(value, "continuity_id")?,
        session_id: read_optional_string(value, "session_id"),
        timestamp: require_i64(value, "timestamp")?,
        role: require_string(value, "role")?,
        content: require_string(value, "content")?,
        valid_from_ms: read_optional_i64(value, "valid_from_ms"),
        valid_until_ms: read_optional_i64(value, "valid_until_ms"),
        created_ms: read_optional_i64(value, "created_ms"),
        provenance: read_optional_string(value, "provenance"),
        chain_hash: read_optional_string(value, "chain_hash"),
        chain_prev_hash: read_optional_string(value, "chain_prev_hash"),
    })
}

/// 链头行 ⇒ 结构体。
fn chain_head_from_json(value: &Value) -> Result<ArchivedChainHead, String> {
    let entry_count = value
        .get("entry_count")
        .and_then(Value::as_i64)
        .ok_or_else(|| "chain_head 缺 entry_count".to_string())?;
    Ok(ArchivedChainHead {
        subject_id: require_string(value, "subject_id")?,
        last_hash: require_string(value, "last_hash")?,
        entry_count: entry_count.max(0) as u64,
        updated_at_ms: require_i64(value, "updated_at_ms")?,
    })
}

/// 取一个**必有**的字符串字段。
fn require_string(value: &Value, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("缺字段 \"{key}\"（或类型不对）"))
}

/// 取一个**必有**的整数字段。
fn require_i64(value: &Value, key: &str) -> Result<i64, String> {
    value
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("缺字段 \"{key}\"（或类型不对）"))
}

/// 取一个**必有**的无符号计数字段。
fn require_usize(value: &Value, key: &str) -> Result<usize, String> {
    let raw = value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("缺字段 \"{key}\"（或类型不对）"))?;
    usize::try_from(raw).map_err(|_| format!("字段 \"{key}\" 超出 usize 范围"))
}

/// 取一个**可空**字符串字段（缺 key 或 `null` ⇒ `None`）。
fn read_optional_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

/// 取一个**可空**整数字段（缺 key 或 `null` ⇒ `None`）。
fn read_optional_i64(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(Value::as_i64)
}
