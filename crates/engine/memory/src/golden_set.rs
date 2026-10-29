//! `apeireth-memory::golden_set` — 记忆质量考卷（golden set）与检索/注入指标。
//!
//! # 为什么要有「祖传考卷」
//!
//! owner 的验收口径是「每次改动在同一份卷子上**不许变差**」（设计稿
//! `docs/01-architecture/memory-knowledge-depth-system-spec.md` §6.4）。
//! 这条口径要能落地，必须先有三样东西：
//!
//! 1. **一份固定的卷子** —— `(query, 应命中记录列表)` 的集合，与具体算法无关；
//! 2. **一把客观的尺子** —— Recall@k / nDCG@k / Evidence Precision / Groundedness；
//! 3. **一道机器闸门** —— 「不许变差」写成可机检的函数与测试，
//!    而不是写进人自觉里（同上 §6.4 末句「规矩写进 CI」）。
//!
//! 本模块只提供这三样里**纯代码**的那部分：卷子格式、指标、回归闸门。
//! 它不联网、不读时钟、不读全局状态 ⇒ 同一输入两次跑出**逐字节相等**的结果。
//!
//! ## 外部公开基准怎么接（PerLTQA 四元组 → 本格式）
//!
//! PerLTQA 的每条 QA 是四元组 `(question, reference memory, memory anchor, answer)`：
//!
//! - `reference memory` ⇒ [`GoldenCase::expect_ids`]：该命中哪些记录（Recall@k 的标准答案）；
//! - `memory anchor` ⇒ [`GoldenCase::expect_fragments`]：该命中的答案片段（Groundedness 的标注）。
//!
//! 即「**我们只吃标签，不吃外部判分器**」（设计稿 §7.2）：省钱、可复现、判分权在我们手里。
//! 数据下载**不是**本批前置（本机到公开基准站点链路不通），所以卷子格式与指标先行，
//! 数据到位即可跑（设计稿 §8「适配器先行」）。
//!
//! ## 确定性纪律（为什么值得单独写一段）
//!
//! - 标注集合一律用 [`BTreeMap`] 表达（有序 ⇒ 无哈希序依赖 ⇒ 跨机器同分）；
//! - 指标是纯函数：输入 `(&EvalBatch, k)`，输出 [`Metrics`]，无副作用；
//! - 排序相关计算只做「首次出现去重 + 位置折损」，不依赖调用方的候选顺序之外的东西。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-25）
//!
//! 四级口径对照（本模块的真实位置）：
//!
//! - **IMPLEMENTED** ✔：类型、指标、闸门、测试齐备（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：未接入 canonical 运行时主路径，生产 0 调用点；
//! - **DEFAULT ENABLED** ✘：没有开关位，也未默认启用；
//! - **HARDWARE VALIDATED** ✘：尚未在真实卷子数据上跑过（公开基准数据未到位）。
//!
//! 对外一律按 **NOT IMPLEMENTED** 表述（新增未接线，不宣称可用）。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// 卷子版本号的域分隔前缀（换算法时改前缀 ⇒ 摘要域隔离）。
const VERSION_DOMAIN: &[u8] = b"apeireth/golden-set-version/v1\0";

/// 版本摘要保留的十六进制长度（12 位 = 48 bit，够区分不同卷子又便于人读）。
const VERSION_HEX_LEN: usize = 12;

/// JSONL 每行**必填**字段（缺任一 ⇒ [`GoldenSetError::MissingField`]）。
///
/// 为什么 `expect_ids` 也是必填而不是「缺了当空」：空标注（`[]`）与**漏写标注**
/// 是两件不同的事 —— 前者是「这题没有标准答案，跳过」，后者是「卷子坏了，报错」。
/// 显式写 `[]` 才允许前者，漏字段直接报错才允许后者。
const REQUIRED_FIELDS: [&str; 3] = ["id", "query", "expect_ids"];

/// 一条考卷题目。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldenCase {
    /// 题目 id（全局唯一，跨卷稳定）。
    pub id: String,
    /// 问题文本（检索用）。
    pub query: String,
    /// **该命中的记录 id**（标准答案；顺序不重要，重复去重）。
    pub expect_ids: Vec<String>,
    /// 可选：**该命中的答案片段**（原文子串）—— 用于 Groundedness / Citation 覆盖。
    /// 空 ⇒ 该题不参与 Groundedness 统计（而不是记 0 分）。
    #[serde(default)]
    pub expect_fragments: Vec<String>,
    /// 可选：题目所属能力类别（自由标签，如 "retrieval" / "forgetting" / "temporal"）。
    #[serde(default)]
    pub tags: Vec<String>,
}

/// 一份考卷（带版本号；改题必须换版本）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldenSet {
    /// 卷子版本号。改题必须换版本（否则历史分数不可比）。
    pub version: String,
    /// 题目集合。
    pub cases: Vec<GoldenCase>,
}

impl GoldenSet {
    /// 用显式版本号建卷（手写卷子 / 外部适配器用）。
    pub fn new(version: impl Into<String>, cases: Vec<GoldenCase>) -> Self {
        Self {
            version: version.into(),
            cases,
        }
    }

    /// 题目数。
    pub fn len(&self) -> usize {
        self.cases.len()
    }

    /// 是否是空卷。
    pub fn is_empty(&self) -> bool {
        self.cases.is_empty()
    }

    /// 按 tag 过滤子集（tag 为空 ⇒ 返回全部）。
    ///
    /// 用途：一份卷子里混着多类能力题（检索 / 遗忘 / 时间），
    /// 门禁可以只盯其中一类 —— 但**版本号保持不变**（子集仍是同一份卷子的一部分，
    /// 分数不可跨子集比较，所以调用方要自己记住比的是哪一档）。
    pub fn filtered(&self, tag: &str) -> GoldenSet {
        if tag.is_empty() {
            return self.clone();
        }
        GoldenSet {
            version: self.version.clone(),
            cases: self
                .cases
                .iter()
                .filter(|case| case.tags.iter().any(|t| t == tag))
                .cloned()
                .collect(),
        }
    }

    /// 加载 JSONL：每行一个 case（字段 snake_case）。空行忽略。
    ///
    /// **版本号自动取内容摘要**（`sha256:<12 位十六进制>`）——
    /// 「改题必须换版本」这条纪律由机器保证，不靠人自觉：题目内容一变，摘要就变。
    /// 需要人可读版本号（如 `perltqa-test-v1`）时用 [`GoldenSet::load_jsonl_with_version`]。
    ///
    /// 未知字段**忽略**（向前兼容：外部卷子带额外字段时不必先改本模块）。
    pub fn load_jsonl(text: &str) -> Result<Self, GoldenSetError> {
        let cases = parse_cases(text)?;
        Ok(GoldenSet {
            version: content_version(&cases),
            cases,
        })
    }

    /// 同 [`GoldenSet::load_jsonl`]，但版本号由调用方显式指定。
    pub fn load_jsonl_with_version(
        text: &str,
        version: impl Into<String>,
    ) -> Result<Self, GoldenSetError> {
        let cases = parse_cases(text)?;
        Ok(GoldenSet {
            version: version.into(),
            cases,
        })
    }
}

/// 一道题的检索结果（系统返回的有序候选）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Retrieved {
    /// 候选 id（按相关度降序）。
    pub ids: Vec<String>,
    /// 每个候选注入的文本片段（用于 Groundedness / Citation）。可为空。
    pub fragments: Vec<String>,
}

/// 一次评测的输入：一批题目 + 每题的实际返回。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EvalBatch {
    /// 题目（与 `results` 一一对应）。
    pub cases: Vec<GoldenCase>,
    /// 每题的实际返回（`results[i]` 对应 `cases[i]`）。
    pub results: Vec<Retrieved>,
}

/// 指标结果（全部为 f64；某指标的分母为 0 时该指标记 0.0 —— 「这批没得分」，
/// 既不是满分也不是零分；分母口径见 [`evaluate`] 的说明）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Metrics {
    /// Recall@k：expect_ids 里被 top-k 命中的比例（对所有题平均）。
    pub recall_at_k: f64,
    /// nDCG@k：按相关位置折损。
    pub ndcg_at_k: f64,
    /// 注入证据中「对（题目有 expect_fragments 时）确实有用」的比例。
    pub evidence_precision: f64,
    /// 注入片段中包含 expect_fragments 的比例（只算有标注的题）。
    pub groundedness: f64,
    /// 因缺标准答案（`expect_ids` 为空）而**整题无法评**的题数
    /// （这类题不参与 Recall@k 的分母）。
    ///
    /// 注意：**缺证据标注**（`expect_fragments` 为空）的题对证据族是「不适用」，
    /// 不进分母、也**不记 0 分**，但**不计入本计数**（口径见 [`evaluate`]）。
    pub skipped_cases: usize,
    /// 本批总题数。
    pub total_cases: usize,
}

/// 评测：cases 与 results 一一对应；长度不等 ⇒ 返回 Err。
///
/// # 口径（逐条钉死，别自创）
///
/// - **Recall@k** = `|expect ∩ top-k ids| / |expect|`，对**有标准答案**的题平均；
///   `expect` 为空的题**跳过**（不进分母，但计入 [`Metrics::skipped_cases`]）———
///   没有标准答案的题不该给系统送分，也不该罚系统。
/// - **nDCG@k**：DCG = `Σ_{i<k} rel_i / log2(i+2)`（`rel_i` = 该位置 id 是否在 expect 里，0/1）；
///   IDCG = 理想排序（expect 全部排前面）的 DCG；**单题 IDCG=0 时该题 nDCG 记 0
///   并计入分母**（保守方向：宁可低估，不许靠「跳过难题」把分刷高）。
///   ⚠️ 这与 Recall 的「expect 为空即跳过」**故意不对称**：
///   Recall 的分母是「有答案的题数」，nDCG 的分母是「全部题数」。
/// - **Evidence Precision**（片段级）：只对**有 `expect_fragments`** 的题统计
///   （无标注 ⇒ 该题对这族指标「不适用」：不进分母、**不记 0 分**、
///   也不计入 [`Metrics::skipped_cases`]——那一栏专记「整题无法评」的题）。
///   口径 = **「注入的片段里有多少是『有据的』」**：对每条注入片段，
///   若它**覆盖**了期望片段（见 [`covers_fragment`]）⇒ 算有据；
///   该题的比值 = 有据片段数 / 注入片段总数；再对有标注的题**取平均**。
///   有标注但**一条片段都没注入** ⇒ 该题记 0.0（什么都没注入不是「精准」，保守记 0）。
/// - **Groundedness**（题级）：同一批有标注的题里，**有多少题拿到了至少一条有据的证据**。
///   ⚠️ **与 Evidence Precision 刻意分母不同**（2026-10-26 口径拆分）：
///   前者问「**注入的东西里有多少有用**」（精度），后者问「**答案有没有被证据支撑**」（覆盖）。
///   ⇒ 之前两者**同一口径** ⇒ 一个指标测两件事 ⇒ 现已拆开。
/// - 分母为 0 的指标整体记 0.0（该指标这批没得分，不是「满分」也不是「零分」）。
pub fn evaluate(batch: &EvalBatch, k: usize) -> Result<Metrics, GoldenSetError> {
    if batch.cases.len() != batch.results.len() {
        return Err(GoldenSetError::LengthMismatch {
            cases: batch.cases.len(),
            results: batch.results.len(),
        });
    }

    let mut recall_sum = 0.0;
    let mut recall_n = 0usize;
    let mut ndcg_sum = 0.0;
    let mut ndcg_n = 0usize;
    let mut evidence_sum = 0.0;
    let mut evidence_n = 0usize;
    let mut grounded_sum = 0.0;
    let mut grounded_n = 0usize;
    let mut skipped_cases = 0usize;

    for (case, retrieved) in batch.cases.iter().zip(batch.results.iter()) {
        let expect = expect_set(&case.expect_ids);
        let fragments = expected_fragments(case);

        // --- Recall@k ---
        if expect.is_empty() {
            // 无标准答案 ⇒ 该题**整题无法评**（不进 Recall 分母），计入 skipped_cases。
            skipped_cases += 1;
        } else {
            let top = top_k(&retrieved.ids, k);
            let hit = count_distinct_hits(&top, &expect);
            recall_sum += hit as f64 / expect.len() as f64;
            recall_n += 1;
        }

        // --- nDCG@k（分母 = 全部题；IDCG=0 的题记 0 计入分母） ---
        let top = top_k(&retrieved.ids, k);
        let dcg = dcg_at_k(&top, &expect);
        let idcg = idcg_for(expect.len(), k);
        let ndcg = if idcg > 0.0 { dcg / idcg } else { 0.0 };
        ndcg_sum += ndcg;
        ndcg_n += 1;

        // --- Evidence Precision / Groundedness（只算有标注的题） ---
        // 缺 `expect_fragments` 的题对证据族是「**不适用**」而非「被跳过」：
        // 不进分母、**不记 0 分**，也不计入 skipped_cases
        // （否则一份只标了检索标注的正常卷子会显示一堆 skipped，容易被误读成「什么都没评上」）。
        if !fragments.is_empty() {
            // **片段级**：有据片段数 / 注入片段总数（精度）。
            let hit = retrieved
                .fragments
                .iter()
                .filter(|fragment| {
                    fragments
                        .iter()
                        .any(|expected| covers_fragment(fragment, expected))
                })
                .count();
            let precision = if retrieved.fragments.is_empty() {
                // 有标注却什么都没注入 ⇒ 记 0（保守），不算「无标注」。
                0.0
            } else {
                hit as f64 / retrieved.fragments.len() as f64
            };
            evidence_sum += precision;
            evidence_n += 1;
            // **题级**：这道题有没有拿到至少一条有据证据（覆盖）⇒ 与上面的精度**分母不同**。
            grounded_sum += if hit > 0 { 1.0 } else { 0.0 };
            grounded_n += 1;
        }
    }

    Ok(Metrics {
        recall_at_k: mean(recall_sum, recall_n),
        ndcg_at_k: mean(ndcg_sum, ndcg_n),
        evidence_precision: mean(evidence_sum, evidence_n),
        groundedness: mean(grounded_sum, grounded_n),
        skipped_cases,
        total_cases: batch.cases.len(),
    })
}

/// 基线对比：不允许准确度变差（owner 裁定 2026-10-25）。
///
/// 规则：`recall_at_k` / `ndcg_at_k` / `groundedness` **一分不许掉**；
/// 最差情况允许的劣化上限 = `baseline * 1% + 1e-9`（绝对下限）。
/// 返回 `Ok(())` 表示通过；`Err` 里列出**哪一项**掉了多少。
///
/// 口径说明：
///
/// - **怎么读「一分不许掉」与「允许 1%」**：owner 的裁定是「准确度一分不许掉」，
///   可机检的落地取**最差情况容忍度 = `baseline * 1%`**（相对），
///   即：**相对劣化超过 1% ⇒ 拦下**；恰好 1% ⇒ 放行（边界，测试钉住）。
/// - **比较哪三项**：契约钦定 `recall_at_k` / `ndcg_at_k` / `groundedness` 三项入闸；
///   `evidence_precision` 目前只作观察量（口径仍在收敛），**不参与**闸门。
/// - **1% 容忍度的单位**：`baseline * 1%` 是**相对**容忍度（基线 0.5 ⇒ 允许掉 0.005）；
///   `+ 1e-9` 是绝对下限，纯粹用来吸收浮点表示误差
///   （例：`0.5 - 0.495 = 0.005000000000000004` 比 `0.005` 大一点点）。
/// - **只查变差**：指标变好不报错（门禁只管「不许掉」）。
pub fn check_no_regression(baseline: &Metrics, current: &Metrics) -> Result<(), String> {
    /// 相对劣化容忍度（1%）。
    const TOLERANCE_RATIO: f64 = 0.01;
    /// 绝对下限（吸收浮点误差）。
    const TOLERANCE_FLOOR: f64 = 1e-9;

    let compared = [
        ("recall_at_k", baseline.recall_at_k, current.recall_at_k),
        ("ndcg_at_k", baseline.ndcg_at_k, current.ndcg_at_k),
        ("groundedness", baseline.groundedness, current.groundedness),
    ];

    let mut failures: Vec<String> = Vec::new();
    for (name, base, cur) in compared {
        let allowed = base * TOLERANCE_RATIO + TOLERANCE_FLOOR;
        let drop = base - cur;
        if drop > allowed {
            failures.push(format!(
                "{name} 掉了 {drop:.6}（基线 {base:.6} → 当前 {cur:.6}；允许上限 {allowed:.6}）"
            ));
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "记忆质量劣化（不允许变差）：{}",
            failures.join("；")
        ))
    }
}

/// 考卷错误类型。
///
/// ⚠️ **为什么不用 `#[derive(thiserror::Error)]`**：`thiserror` 会把**名为 `source` 的字段**
/// 自动当成 `std::error::Error::source()` 的返回值（`thiserror-impl` 的 `source_field()`
/// 按字段名自动探测），而本契约钦定该字段类型是 `String`（一段文本描述，不是错误类型）
/// ⇒ 派生宏生成的 `self.source.as_dyn_error()` 编译不过（`String: AsDynError` 不成立）。
/// 故此处**手写 `Display` + `std::error::Error`**：对外的错误类型名、变体名、字段名、
/// 字段类型、文案**完全不变**，只是 `source()` 一律为 `None`
/// （它本来就不是错误链节点，是一句人读描述）。
#[derive(Debug)]
pub enum GoldenSetError {
    /// JSONL 某一行解析失败。
    Json {
        /// 出错的行号（1 起）。
        line: usize,
        /// 底层错误描述。
        source: String,
    },
    /// 某一行缺少必填字段。
    MissingField {
        /// 缺失字段名。
        field: String,
        /// 出错的行号（1 起）。
        line: usize,
    },
    /// 题目与结果数量不等。
    LengthMismatch {
        /// 题目数。
        cases: usize,
        /// 结果数。
        results: usize,
    },
}

impl std::fmt::Display for GoldenSetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GoldenSetError::Json { line, source } => {
                write!(f, "考卷 JSONL 解析失败（第 {line} 行）：{source}")
            }
            GoldenSetError::MissingField { field, line } => {
                write!(f, "考卷题目缺少必填字段：{field}（第 {line} 行）")
            }
            GoldenSetError::LengthMismatch { cases, results } => {
                write!(f, "cases 与 results 长度不一致：{cases} vs {results}")
            }
        }
    }
}

impl std::error::Error for GoldenSetError {}

// ============================================
// 内部工具
// ============================================

/// 解析 JSONL 文本 ⇒ 题目列表（空行忽略；行号 1 起）。
fn parse_cases(text: &str) -> Result<Vec<GoldenCase>, GoldenSetError> {
    let mut cases = Vec::new();
    for (idx, raw) in text.lines().enumerate() {
        let line = idx + 1;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value: serde_json::Value =
            serde_json::from_str(trimmed).map_err(|e| GoldenSetError::Json {
                line,
                source: e.to_string(),
            })?;
        let object = value.as_object().ok_or_else(|| GoldenSetError::Json {
            line,
            source: "期望 JSON 对象".to_string(),
        })?;
        for field in REQUIRED_FIELDS {
            if !object.contains_key(field) {
                return Err(GoldenSetError::MissingField {
                    field: field.to_string(),
                    line,
                });
            }
        }
        let case: GoldenCase = serde_json::from_value(value).map_err(|e| GoldenSetError::Json {
            line,
            source: e.to_string(),
        })?;
        cases.push(case);
    }
    Ok(cases)
}

/// 内容摘要版本号（改题 ⇒ 改版本，机械保证）。
fn content_version(cases: &[GoldenCase]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(VERSION_DOMAIN);
    for case in cases {
        // 用紧凑 JSON 作规范化字节序列：字段顺序由结构体定义固定 ⇒ 摘要稳定可复现。
        let payload = serde_json::to_string(case).unwrap_or_default();
        hasher.update((payload.len() as u64).to_be_bytes());
        hasher.update(payload.as_bytes());
    }
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(VERSION_HEX_LEN);
    for b in digest.iter().take(VERSION_HEX_LEN / 2) {
        hex.push_str(&format!("{b:02x}"));
    }
    format!("sha256:{hex}")
}

/// 标准答案 id 集合（有序 ⇒ 无哈希序依赖；空串 id 视为无效标注，丢弃）。
fn expect_set(expect_ids: &[String]) -> BTreeMap<String, ()> {
    let mut set = BTreeMap::new();
    for id in expect_ids {
        let trimmed = id.trim();
        if trimmed.is_empty() {
            continue;
        }
        set.insert(trimmed.to_string(), ());
    }
    set
}

/// 判定一条**注入片段**是否「有据」：它是否**覆盖**了某个期望片段。
///
/// # 为什么不是「逐字包含」（2026-10-26 口径修正）
///
/// 期望片段是**句子里的一个切片**（挖题时按词位置裁 60 字）⇒ 注入窗口只要
/// **切过那句话**，逐字包含就必然失败 ⇒ 指标**对窗口对齐过敏**
/// （实测：注「取开头」0.0353 vs 注「最相关片段」0.0025 ⇒ 方向都是反的
/// ⇒ 该指标**测不出**注入好坏）。
///
/// # 现在的口径
///
/// **覆盖** = 注入片段与期望片段的**最长公共子串**长度 ≥ 期望片段长度的
/// [`FRAGMENT_COVERAGE_RATIO`]（且至少 4 字）。
///
/// * 用**最长公共子串**（不是集合交集）⇒ 窗口切掉一半时，剩下的那半**仍算覆盖**；
/// * 「至少 4 字」的下限 ⇒ 防止「1 个字碰巧相同」也算有据；
/// * **逐字包含仍算覆盖**（公共子串可以等于整个期望片段）⇒ 旧行为**不被推翻**，只是放宽。
pub const FRAGMENT_COVERAGE_RATIO: f64 = 0.6;

/// 期望片段长度（字符数）。
fn fragment_len(fragment: &str) -> usize {
    fragment.chars().count()
}

/// 注入片段是否覆盖了期望片段。
pub fn covers_fragment(injected: &str, expected: &str) -> bool {
    let expected_len = fragment_len(expected);
    if expected_len == 0 {
        return false;
    }
    if injected.contains(expected) {
        return true;
    }
    let common = longest_common_substring_len(injected, expected);
    let needed = ((expected_len as f64) * FRAGMENT_COVERAGE_RATIO).ceil() as usize;
    common >= needed.max(4)
}

/// 两个字符串的**最长公共子串**长度（按字符计）。
///
/// # 实现口径
///
/// 经典 DP（`O(n·m)`）⇒ **确定性**、无依赖。
/// ⚠️ 长度按**字符**计（不是字节）⇒ 中文不被算成 3 倍。
fn longest_common_substring_len(left: &str, right: &str) -> usize {
    let left: Vec<char> = left.chars().collect();
    let right: Vec<char> = right.chars().collect();
    if left.is_empty() || right.is_empty() {
        return 0;
    }
    let mut previous = vec![0usize; right.len() + 1];
    let mut best = 0usize;
    for a in &left {
        let mut current = vec![0usize; right.len() + 1];
        for (index, b) in right.iter().enumerate() {
            if a == b {
                current[index + 1] = previous[index] + 1;
                if current[index + 1] > best {
                    best = current[index + 1];
                }
            }
        }
        previous = current;
    }
    best
}

/// 期望片段列表（去掉空白片段：空串会被任何文本「包含」，会让指标虚高）。
fn expected_fragments(case: &GoldenCase) -> Vec<&str> {
    case.expect_fragments
        .iter()
        .map(|f| f.trim())
        .filter(|f| !f.is_empty())
        .collect()
}

/// 取 top-k 候选（`k` 超过候选数时取全部）。
fn top_k(ids: &[String], k: usize) -> Vec<&str> {
    ids.iter().take(k).map(|s| s.as_str()).collect()
}

/// 首次出现去重（同一 id 重复出现只算一次；防止重复候选把分数撑破 1.0）。
fn dedup_preserving_order<'a>(ids: &[&'a str]) -> Vec<&'a str> {
    let mut seen = BTreeMap::new();
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        if !seen.contains_key(*id) {
            seen.insert(*id, ());
            out.push(*id);
        }
    }
    out
}

/// top-k 里命中的**不同**标准答案个数。
fn count_distinct_hits(top: &[&str], expect: &BTreeMap<String, ()>) -> usize {
    dedup_preserving_order(top)
        .into_iter()
        .filter(|id| expect.contains_key(*id))
        .count()
}

/// DCG@k（`rel_i` = 该位置 id 是否在 expect 里，0/1）。
fn dcg_at_k(top: &[&str], expect: &BTreeMap<String, ()>) -> f64 {
    let ranked = dedup_preserving_order(top);
    let mut dcg = 0.0;
    for (i, id) in ranked.iter().enumerate() {
        if expect.contains_key(*id) {
            dcg += 1.0 / ((i + 2) as f64).log2();
        }
    }
    dcg
}

/// 理想排序的 DCG（expect 全部排前面；只看前 k 个位置）。
fn idcg_for(expect_len: usize, k: usize) -> f64 {
    let usable = expect_len.min(k);
    let mut idcg = 0.0;
    for i in 0..usable {
        idcg += 1.0 / ((i + 2) as f64).log2();
    }
    idcg
}

/// 平均值（分母为 0 ⇒ 记 0.0）。
fn mean(sum: f64, n: usize) -> f64 {
    if n == 0 {
        0.0
    } else {
        sum / n as f64
    }
}
