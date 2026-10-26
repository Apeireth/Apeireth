//! `apeireth-memory::wiki_corpus_real` — 真实语料 → 挖题器的接线。
//!
//! # 为什么有这个模块
//!
//! 设计稿 `docs/01-architecture/memory-knowledge-depth-system-spec.md` §7.1 的**双轨考卷**：
//! 外部卷（PerLTQA）证明「不落后于公开水平」，**内部卷**证明「我们自己的场景好用」
//! （中文 + 长期工程记忆 + 出处要求）。§8 的 B0-b 批次写得很直白：
//! **「挖 2000 条候选 → 先给 owner 看 20 条样本 → 人工精标 300 条」**。
//!
//! 上一批（W-2）把「挖」的**算法**做出来了（[`crate::golden_mining`]，纯函数、确定性、20 条测试全绿），
//! 但**没喂过真实数据** ⇒ owner 至今没见过「题像不像人话」。
//! 本模块补的就是**接线那一段**：把真实日记读成记录 ⇒ 喂给挖题器 ⇒ 产出
//! **可过目的样本**（[`render_samples`]）与**可跑的考卷**（[`to_jsonl`]）。
//!
//! # 定位（三条边界，钉死）
//!
//! 1. **只读**：不写日记、不改日记、不联网、不删任何文件。
//! 2. **纯确定**：同一批语料两次跑 ⇒ 记录、题目、样本文本、JSONL **逐字相等**
//!    （无随机数、无哈希序依赖、排序键全是显式比较）。
//! 3. **不假装**：语料不存在 ⇒ 返回空 + 计数，**不 panic、不报错**
//!    （本机可能压根没跑过 `apeireth dream`，那是正常状态，不是故障）。
//!
//! # 数据从哪来
//!
//! 真实语料位置 = `<数据根>/diary/*.json`（**一天一个文件**）。
//! 数据根 = `$APEIRETH_DATA_DIR`（非空时）否则 `$HOME/.apeireth`
//! —— 与 `apeireth_runtime_assembly::paths::default_data_root()` **同口径**
//! （见 [`default_data_root`] 的说明：单一真源在 runtime-assembly，
//! 本 crate 反向依赖它会成环 ⇒ 这里镜像一份）。
//!
//! # 格式容错（**如实、不崩**）
//!
//! 日记文件的**内容**有两种可能，本模块**都收**：
//!
//! - **裸数组**：`[{"source": "user", "body": "…", "timestamp_epoch_ms": 1730000000000}, …]`
//!   （契约钦定的形状：一条日记 = 一个元素）；
//! - **一日一页对象**：`{"date": "2026-10-25", "entries": [ … ]}`
//!   （这是本仓 `FileDiaryStore` **实际**落盘的形状 —— `apeireth dream` 写出来的就是它）。
//!
//! 容错口径：
//!
//! - 文件不是合法 JSON / 不是上面两种形状 ⇒ **跳过该文件**并计入 `skipped_files`；
//! - 元素缺 `body` 或 `body` 为空 ⇒ **跳过该元素**并计入 `skipped_entries`；
//! - `timestamp_epoch_ms` 缺失 ⇒ **不猜时间**（不从文件名反推 epoch），
//!   用 `0` 打底 + **文件内序号**做递增值（同一天内仍能按写入顺序排开）。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-26）
//!
//! 四级口径（本模块的真实位置）：
//!
//! - **IMPLEMENTED** ✔：类型、接线、导出齐备（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：**无 CLI、无定时任务、生产 0 调用点**；
//! - **DEFAULT ENABLED** ✘：没有开关位，也未默认启用；
//! - **HARDWARE VALIDATED** ✘：本机真实语料只有 **1 个日记文件 / 1 条条目**
//!   （`apeireth dream` 跑过一次），**未成规模**（设计稿要 2000 条候选 ⇒ 还差两个数量级）。
//!
//! 对外一律按 **NOT IMPLEMENTED** 表述（新增未接线，不宣称可用）。
//!
//! # 已知质量上限（**不掩盖**）
//!
//! 真实日记**没有标题字段** ⇒ [`crate::golden_mining`] 的「问题主体」只能退回
//! 分词关键词（`召回` / `延迟` …）而不是人写的摘要。设计稿 §7.1 要的是
//! 「中文 + 长期工程记忆 + **出处要求**」，本模块保证的是**出处**（每条题都能指回
//! `文件#序号`），**题目自然度**受限于语料本身有没有标题 —— 见交付说明。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::golden_mining::{mine, MineRecord, MinedCase, MiningConfig};
use crate::golden_set::GoldenCase;

/// 一条日记（真实语料的最小单位）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiaryDoc {
    /// 日期（`YYYY-MM-DD`；解析失败则原样保留）。
    pub date: String,
    /// 正文。
    pub body: String,
}

/// 一个「记录 + 其出处」的组合（挖出来的题要能指回原文）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcedRecord {
    /// 可挖的记录本体（id 即标准答案里的记录 id）。
    pub record: MineRecord,
    /// 该记录来自哪个文件（相对语料根）+ 大致位置（如 `2026-10-25.json#3`）。
    pub origin: String,
}

/// 日记文件的扩展名（只收 `.json`）。
const DIARY_FILE_EXT: &str = "json";

/// 记录 id 的前缀（便于在卷子里一眼认出「这是日记挖出来的题」）。
const RECORD_ID_PREFIX: &str = "diary";

/// 标签前缀（内部卷专用 ⇒ 与外部卷 PerLTQA 的 `external` 标签区分开）。
const TAG_INTERNAL: &str = "internal";

/// 标签：语料来源（本模块）。
const TAG_CORPUS: &str = "wiki_corpus_real";

/// 把日记目录（`<root>/*.json`，每个文件一天）读成记录。
///
/// **格式**：日记文件是 JSON 数组，元素形如
/// `{"source": "user", "body": "…", "timestamp_epoch_ms": 1730000000000}`
/// ⇒ 一条日记 = 一个元素。
/// **容错**（如实、但不崩）：
/// * 文件不是合法 JSON ⇒ **跳过该文件**并计入 `skipped_files`；
/// * 元素缺 `body` 或 body 为空 ⇒ 跳过该元素并计入 `skipped_entries`；
/// * `timestamp_epoch_ms` 缺失 ⇒ 用 0（**不猜**时间，但要能排序 ⇒ 同一天内按文件内顺序给递增值）。
/// **确定性**：按文件名（日期）字典序读，文件内按元素顺序。
///
/// # 返回值
///
/// `(记录列表, 跳过的文件数, 跳过的元素数)`。
///
/// # 目录不存在
///
/// 返回 `(空, 0, 0)` —— **不是错误**：本机可能没跑过 `apeireth dream`。
pub fn load_diary_records(root: &Path) -> (Vec<SourcedRecord>, usize, usize) {
    let mut records: Vec<SourcedRecord> = Vec::new();
    let mut skipped_files = 0usize;
    let mut skipped_entries = 0usize;

    // 目录不存在 / 读不到 ⇒ 当作「没有日记」处理（返回空，不 panic）。
    let Ok(dir) = std::fs::read_dir(root) else {
        return (records, skipped_files, skipped_entries);
    };

    // 先收集再排序：目录遍历顺序跟文件系统有关（不保证），**排序键必须显式**。
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    for entry in dir {
        let Ok(entry) = entry else {
            // 目录项本身读不到 ⇒ 当作跳过（不 panic、不静默）。
            skipped_files += 1;
            continue;
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some(DIARY_FILE_EXT) {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            // 文件名不是合法 UTF-8 ⇒ 没法当日期用 ⇒ 跳过。
            skipped_files += 1;
            continue;
        };
        files.push((name.to_string(), path));
    }
    // 确定性：按文件名（= 日期）字典序。
    files.sort_by(|left, right| left.0.cmp(&right.0));

    for (name, path) in files {
        let Ok(text) = std::fs::read_to_string(&path) else {
            skipped_files += 1;
            continue;
        };
        // 日期 = 文件名主干（`2026-10-25.json` ⇒ `2026-10-25`）；
        // 解析失败（不是 `YYYY-MM-DD`）⇒ **原样保留**，不报错、不猜日期。
        let date = date_from_file_name(&name);
        let Some((entries, entry_skips)) = parse_diary_entries(&text) else {
            // 文件级失败（JSON 非法 / 结构不是日记）⇒ 跳过整个文件。
            skipped_files += 1;
            continue;
        };
        skipped_entries += entry_skips;

        for (index, entry) in entries.into_iter().enumerate() {
            let doc = DiaryDoc {
                date: date.clone(),
                body: entry.body,
            };
            // 记录 id = 日期 + 文件内序号 ⇒ 稳定、唯一、可排序（同一批语料两次读必得同一批 id）。
            let id = format!("{RECORD_ID_PREFIX}-{}-{:03}", doc.date, index);
            // 时间戳：文件里有就用；没有 ⇒ **不猜**（0 打底）+ 文件内序号（递增值）
            // ⇒ 同一天内缺时间戳的记录仍能按写入顺序排开，且绝不编造绝对时间。
            let timestamp_ms = entry.timestamp_ms.unwrap_or(index as i64);
            records.push(SourcedRecord {
                record: MineRecord {
                    id,
                    content: doc.body,
                    // 日记没有标题字段 ⇒ **不编标题**（编一个假标题会让题目主体变成假话）。
                    // 代价见模块文档「已知质量上限」。
                    title: None,
                    timestamp_ms,
                    // 也不编 thread 分组键：同一天 ≠ 同一话题（编出来会让不相关的两条
                    // 被当成「同主题冲突」，题目就假了）。同主题判定交给共享词口径。
                    thread_id: None,
                },
                // 出处：`文件名#元素序号`（序号 0 起）⇒ owner 能直接翻回原文。
                origin: format!("{name}#{index}"),
            });
        }
    }

    (records, skipped_files, skipped_entries)
}

/// 把挖出来的题与**指回原文**的出处配对（每题给 `expect_ids` = 记录 id 列表，
/// `expect_fragments` = 原文里真实存在的片段）。
///
/// **复用** `crate::golden_mining::mine()`（**不要自己重写挖掘算法**）：
/// 1. 把记录喂给 `mine()` 得到 `MinedCase`；
/// 2. `MinedCase.expect_ids` 里装的是**记录 id** ⇒ 你要保证 id 与 `SourcedRecord.record.id` 对得上
///    （id 用 `record.id` 本身，别另造一套）。
/// 3. `expect_fragments` 必须**真的出现在**对应记录的 content 里（否则模型无法被判定为「有据」）
///    ⇒ 逐条**校验**；对不上的**剔除该片段**（不静默保留假的）。
///
/// # 剔除口径（为什么连 id 一起剔）
///
/// [`MinedCase`] 的契约是 `expect_fragments[i]` **属于** `expect_ids[i]`（一一对应）。
/// 只删片段会破坏这个一一对应（剩下的片段就对不上号了），留下一条**无主的 id** 更糟
/// （评测时那条 id 没有可核对的证据，却仍算进 Recall 的分母）。
/// ⇒ 对不上的片段**连同它配对的记录 id 一起剔除**；剔完一条不剩 ⇒ 整题丢弃
/// （没有标准答案的题不该进卷子）。
///
/// # 确定性
///
/// `mine()` 本身确定；本函数只做**过滤**（不改顺序、不改题目文本）⇒ 同一输入逐字相等。
pub fn build_cases(records: &[SourcedRecord], config: MiningConfig) -> Vec<MinedCase> {
    let mine_records: Vec<MineRecord> = records
        .iter()
        .map(|sourced| sourced.record.clone())
        .collect();
    mine(&mine_records, config)
        .into_iter()
        .map(|case| verify_case(&case, records))
        .filter(|case| !case.expect_ids.is_empty())
        .collect()
}

/// 逐条校验一题的「答案片段真的在对应记录的正文里」，对不上的剔除。
///
/// **公开**（而不是私有）：这是「不静默保留假的片段」这条硬契约的**唯一执行点**，
/// 必须能被单独测（正常路径下 `mine()` 不会产出对不上的片段 ⇒ 只能直接构造坏题来测）。
///
/// 详见 [`build_cases`] 的「剔除口径」。
pub fn verify_case(case: &MinedCase, records: &[SourcedRecord]) -> MinedCase {
    let index: BTreeMap<&str, &str> = records
        .iter()
        .map(|sourced| (sourced.record.id.as_str(), sourced.record.content.as_str()))
        .collect();

    let mut expect_ids: Vec<String> = Vec::new();
    let mut expect_fragments: Vec<String> = Vec::new();
    for (position, id) in case.expect_ids.iter().enumerate() {
        // 没有配对的片段 ⇒ 这条 id 无从核对 ⇒ 不留（保住一一对应）。
        let Some(fragment) = case.expect_fragments.get(position) else {
            continue;
        };
        let Some(content) = index.get(id.as_str()) else {
            // 标准答案指向一条**不存在**的记录 ⇒ 剔除（否则卷子自相矛盾）。
            continue;
        };
        if !content.contains(fragment.as_str()) {
            // 片段对不上原文 ⇒ 剔除（不静默保留假的）。
            continue;
        }
        expect_ids.push(id.clone());
        expect_fragments.push(fragment.clone());
    }

    MinedCase {
        id: case.id.clone(),
        query: case.query.clone(),
        expect_ids,
        expect_fragments,
        kind: case.kind.clone(),
    }
}

/// 导出成人可读的样本（给 owner 过目用）。
///
/// 每条**一行**，格式：
/// `[kind] query  ⇒  expect_ids=[...]  fragments=[…]`
/// **确定性**：按 (kind, query) 排序。
///
/// # 换行口径
///
/// 行与行之间用 `\n` 分隔，**末尾不带换行** ⇒ `split('\n').count()` 恰好等于行数。
///
/// # limit
///
/// `limit = 0` ⇒ 返回空串（不是「全部」）；超出条数 ⇒ 只给前 `limit` 条（排序后取前 N）。
pub fn render_samples(cases: &[MinedCase], limit: usize) -> String {
    let mut sorted: Vec<&MinedCase> = cases.iter().collect();
    sorted.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.query.cmp(&right.query))
            // 同 (kind, query) 时再按 id 兜底 ⇒ 排序全序（不会因输入顺序抖动）。
            .then_with(|| left.id.cmp(&right.id))
    });

    let mut out = String::new();
    for (position, case) in sorted.iter().take(limit).enumerate() {
        if position > 0 {
            out.push('\n');
        }
        out.push('[');
        out.push_str(&case.kind);
        out.push_str("] ");
        out.push_str(&case.query);
        out.push_str("  ⇒  expect_ids=[");
        out.push_str(&case.expect_ids.join(", "));
        out.push_str("]  fragments=[");
        out.push_str(&case.expect_fragments.join(", "));
        out.push(']');
    }
    out
}

/// 导出成考卷 JSONL（可直接被 `GoldenSet::load_jsonl` 读）。
///
/// # 字段映射
///
/// | 本仓卷子字段 | 来源 |
/// |---|---|
/// | `id` | [`MinedCase::id`]（挖掘阶段的确定性 id，`gm-{kind}-{序号}`） |
/// | `query` | [`MinedCase::query`] |
/// | `expect_ids` | [`MinedCase::expect_ids`]（= 记录 id ⇒ 指向真实日记） |
/// | `expect_fragments` | [`MinedCase::expect_fragments`]（= 原文子串） |
/// | `tags` | `internal` / `wiki_corpus_real` / `kind:{题型}`（与外部卷 `external` 区分） |
///
/// # 顺序
///
/// 保持**入参顺序**（`build_cases` 产出的顺序已按 `(kind, id)` 排好）
/// ⇒ 同一输入两次导出逐字相等。
pub fn to_jsonl(cases: &[MinedCase]) -> String {
    let mut out = String::new();
    for case in cases {
        let golden = GoldenCase {
            id: case.id.clone(),
            query: case.query.clone(),
            expect_ids: case.expect_ids.clone(),
            expect_fragments: case.expect_fragments.clone(),
            tags: vec![
                TAG_INTERNAL.to_string(),
                TAG_CORPUS.to_string(),
                format!("kind:{}", case.kind),
            ],
        };
        // `GoldenCase` 的字段全是字符串/字符串数组 ⇒ 序列化不会失败；
        // 万一失败（理论上不可能）⇒ 写空行（`load_jsonl` 会忽略空行），不 panic。
        match serde_json::to_string(&golden) {
            Ok(line) => {
                out.push_str(&line);
                out.push('\n');
            }
            Err(_) => out.push('\n'),
        }
    }
    out
}

/// 解析一个日记文件的内容 ⇒ `(日记条目, 被跳过的元素数)`。
///
/// * `None` ⇒ **文件级失败**（JSON 非法 / 结构不是日记数组或「一日一页」对象）
///   ⇒ 调用方计入 `skipped_files`。
/// * 元素缺 `body` / `body` 为空 ⇒ 跳过该元素（计入 `skipped_entries`）。
///
/// 两种文件形状都收：裸数组（契约形状）与 `{"date": …, "entries": [ … ]}`
/// （本仓 `FileDiaryStore` 实际落盘的形状）。
pub fn parse_diary_docs(text: &str, date: &str) -> Option<(Vec<DiaryDoc>, usize)> {
    let (entries, skipped) = parse_diary_entries(text)?;
    let docs = entries
        .into_iter()
        .map(|entry| DiaryDoc {
            date: date.to_string(),
            body: entry.body,
        })
        .collect();
    Some((docs, skipped))
}

/// 数据根目录：`$APEIRETH_DATA_DIR`（非空时）否则 `$USERPROFILE`/`$HOME` 下的 `.apeireth`。
///
/// # 为什么不是直接调 `apeireth_runtime_assembly::paths::default_data_root()`
///
/// 单一真源确实在 runtime-assembly，但**本 crate 不能反向依赖它**
/// （`apeireth-runtime-assembly` 依赖 `apeireth-memory` ⇒ 反向依赖成环，编不过），
/// 且本批不新增依赖 ⇒ 这里按**同一口径**镜像一份。
/// ⇒ 接线批次应补一条「两边口径一致」的一致性测试（或把单一真源下沉到 `apeireth-core`）。
pub fn default_data_root() -> PathBuf {
    // ⚠️ **委派给 core**（唯一真源）—— 本仓曾出过**两份逐字相同**的��现
    // （这里 + `runtime_assembly::paths`）⇒ 漂移就会让「生产写账」与「CLI 读账」
    // 指向不同文件（体检读不到刚写的记忆 / 备份漏文件 / 回滚找不到目标）。
    // 记忆层不能依赖 `runtime-assembly`（成环）⇒ 真源落 `apeireth-core`。
    apeireth_core::paths::default_data_root()
}

/// 真实日记目录（`<数据根>/diary`，一天一个 JSON 文件）。
pub fn default_diary_root() -> PathBuf {
    diary_root_for(&default_data_root())
}

/// 一个数据根 → 它的日记目录（`<数据根>/diary`）。
pub fn diary_root_for(data_dir: &Path) -> PathBuf {
    apeireth_core::paths::data_subdir(data_dir, "diary")
}

// ============================================================
// 内部
// ============================================================

/// 一条日记条目的内部解析结果（正文 + 可选时间戳）。
struct RawDiaryEntry {
    body: String,
    timestamp_ms: Option<i64>,
}

/// 解析日记文件的 JSON ⇒ `(条目, 跳过的元素数)`；`None` = 文件级失败。
fn parse_diary_entries(text: &str) -> Option<(Vec<RawDiaryEntry>, usize)> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let items: Vec<&serde_json::Value> = match &value {
        serde_json::Value::Array(items) => items.iter().collect(),
        serde_json::Value::Object(map) => {
            // 「一日一页」对象（`FileDiaryStore` 的落盘形状）。
            let entries = map.get("entries")?;
            let items = entries.as_array()?;
            items.iter().collect()
        }
        _ => return None,
    };

    let mut out: Vec<RawDiaryEntry> = Vec::new();
    let mut skipped = 0usize;
    for item in items {
        let Some(body) = item.get("body").and_then(|value| value.as_str()) else {
            skipped += 1;
            continue;
        };
        let body = body.trim();
        if body.is_empty() {
            skipped += 1;
            continue;
        }
        // 时间戳缺失 / 不是整数 ⇒ 当作缺失（**不猜**）。
        let timestamp_ms = item
            .get("timestamp_epoch_ms")
            .and_then(|value| value.as_i64());
        out.push(RawDiaryEntry {
            body: body.to_string(),
            timestamp_ms,
        });
    }
    Some((out, skipped))
}

/// 文件名主干 ⇒ 日期（`2026-10-25.json` ⇒ `2026-10-25`）。
///
/// 解析失败（不是 `YYYY-MM-DD`）⇒ **原样保留**主干（不报错、不猜日期）。
fn date_from_file_name(name: &str) -> String {
    match name.strip_suffix(".json") {
        Some(stem) => stem.to_string(),
        None => name.to_string(),
    }
}
