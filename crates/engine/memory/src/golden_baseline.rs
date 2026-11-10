//! `apeireth-memory::golden_baseline` — 内部卷的**检索基线**（供「不许掉」闸用）。
//!
//! # 为什么要有基线
//!
//! owner 裁定（2026-10-25）「**准确度一分不许掉**」⇒ 已实现为
//! [`crate::golden_set::check_no_regression`]（相对劣化 > 1% ⇒ 拦下）。
//!
//! 但那个闸门只回答「**现在 vs 手上这份分数**」⇒ **手上这份分数从哪来？**
//! 没有固化 ⇒ 改检索代码时分数涨跌**无人知晓** ⇒ 纪律落空。
//!
//! ⇒ 本模块把「**上一次的真实分数**」固化成**一个文件**（`scripts/golden-baseline.json`）
//! ⇒ 改动检索代码后重跑一遍 ⇒ 涨了跌了**立刻可见** ⇒ 「不许掉」才落得到实处。
//!
//! # 基线文件（`scripts/golden-baseline.json`）的结构
//!
//! ```json
//! {
//!   "version": "internal-2026-10-26",
//!   "k": 10,
//!   "corpora": [
//!     { "name": "redchamber", "recall_at_k": 0.2983, "ndcg_at_k": 0.1615, "cases": 476 },
//!     { "name": "watershed", "recall_at_k": 0.6794, "ndcg_at_k": 0.4796, "cases": 248 }
//!   ]
//! }
//! ```
//!
//! # 三条纪律（都写进类型里，不靠人自觉）
//!
//! 1. **版本号**：改题 / 改口径 ⇒ 必须换 `version` ⇒ 换了版本就不与旧基线比
//!    （否则拿「换了题」当「掉了」或拿「没掉」当「没掉」，都是自欺）。
//! 2. **题数**：题数变了 ⇒ 分数**不可比** ⇒ [`compare_to_baseline`] 报「不可比」
//!    而不是默默比（题都换了还比分数 ⇒ 比的是两件事）。
//! 3. **语料缺失**：语料在**仓外**（`~/.apeireth/novels/`，下载器 `scripts/download_novels.sh`
//!    带 SHA-256）⇒ 基线脚本在**语料缺失时必须**如实说「跳过（缺语料）」，
//!    **不许**默默当通过。
//!
//! # 口径：基线**只固化两项检索指标**
//!
//! * 固化 `recall_at_k` 与 `ndcg_at_k`（**检索**的两项）；
//! * **不固化** `evidence_precision` / `groundedness`（**注入**族）——
//!   那两项测的是「注入给模型什么」，与「检索准不准」**不是一件事**
//!   ⇒ 混进检索基线 ⇒ 改注入策略会被误判成「检索掉了」。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-26）
//!
//! 四级口径（本模块的真实位置）：
//!
//! - **IMPLEMENTED** ✔：类型、读写、校验、比较齐备（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：生产主路径 0 调用点（只有门禁脚本与测试在用）；
//! - **DEFAULT ENABLED** ✘：没有开关位，也未默认启用；
//! - **HARDWARE VALIDATED** ✘：已在本机**真实语料**（两本小说）上跑过一遍
//!   （2026-10-26，见 `scripts/golden-baseline.json`），但**未在第二台机器复现**
//!   ⇒ 不宣称「跨机器可复现」。
//!
//! 对外一律按 **NOT IMPLEMENTED** 表述（新增未接线，不宣称可用）。

use crate::golden_set::Metrics;

/// 相对劣化容忍度（1%）—— **与 [`crate::golden_set::check_no_regression`] 同口径**。
///
/// 为什么复用同一个数：判「掉了」的标准只应有一处 ⇒ 两处各写各的 ⇒
/// 迟早一边放行一边拦下 ⇒ 纪律失效。
const TOLERANCE_RATIO: f64 = 0.01;

/// 绝对下限（吸收浮点表示误差）—— 同 [`crate::golden_set::check_no_regression`]。
const TOLERANCE_FLOOR: f64 = 1e-9;

/// 一份语料的基线分数。
#[derive(Debug, Clone, PartialEq)]
pub struct CorpusBaseline {
    /// 语料名（如 `redchamber`）。
    pub name: String,
    /// 基线指标。
    ///
    /// ⚠️ **只有 `recall_at_k` / `ndcg_at_k` 两项被固化**；
    /// 其余字段（证据族等）**不属于检索基线** ⇒ [`to_json`] 会**有意丢弃**它们，
    /// [`parse_baseline`] 解析出来的其它字段恒为 `0.0` / `0`。
    pub metrics: Metrics,
    /// 该基线对应的题数（**题数变了 ⇒ 基线不可比** ⇒ 必须报错，不许默默比）。
    pub cases: usize,
}

impl CorpusBaseline {
    /// 用**两项检索指标**建一条语料基线（其余指标字段归零 —— 它们不属于检索基线）。
    pub fn new(name: impl Into<String>, recall_at_k: f64, ndcg_at_k: f64, cases: usize) -> Self {
        Self {
            name: name.into(),
            metrics: Metrics {
                recall_at_k,
                ndcg_at_k,
                ..Metrics::default()
            },
            cases,
        }
    }
}

/// 一份基线（全部语料 + 评测口径）。
#[derive(Debug, Clone, PartialEq)]
pub struct GoldenBaseline {
    /// 基线版本号（**改题/改口径必须换版本** ⇒ 换了版本就不与旧基线比）。
    pub version: String,
    /// 评测的 top-k。
    pub k: usize,
    /// 各语料的基线。
    pub corpora: Vec<CorpusBaseline>,
}

impl GoldenBaseline {
    /// 用显式版本号建一份基线。
    pub fn new(version: impl Into<String>, k: usize, corpora: Vec<CorpusBaseline>) -> Self {
        Self {
            version: version.into(),
            k,
            corpora,
        }
    }
}

/// 基线里某份语料的基线（找不到 ⇒ `None`）。
pub fn corpus_baseline<'a>(baseline: &'a GoldenBaseline, name: &str) -> Option<&'a CorpusBaseline> {
    baseline.corpora.iter().find(|corpus| corpus.name == name)
}

/// 把当前分数与基线比 ⇒ 哪些指标**掉了**。
///
/// # 口径（照 [`crate::golden_set::check_no_regression`] 的纪律，但这里只**报告**，不判定）
///
/// * 比 `recall_at_k` 与 `ndcg_at_k` 两项（**注入族不参与** —— 见模块头「口径」）；
/// * 相对劣化 > **1%** ⇒ 算「掉了」（与 `check_no_regression` 的容忍度一致）；
/// * **题数不同 ⇒ 不比**（返回一条「不可比」说明）—— 题变了分数就不可比；
/// * **基线里没有这份语料** ⇒ 也不比（返回一条「不可比」说明）——
///   「没被基线覆盖」不等于「没掉」⇒ 不许默默当通过。
///
/// 返回：`(掉了的指标说明列表, 不可比的说明列表)`。
///
/// # 为什么不返回 `Result`
///
/// 本函数**只报告**：判定（拦下 / 放行）是**调用方**的权责
/// （门禁脚本要打印明细并决定退出码，测试要断言列表内容）
/// ⇒ 这里把「掉了什么」和「为什么没法比」**分开返回**，不替调用方做决定。
pub fn compare_to_baseline(
    baseline: &GoldenBaseline,
    name: &str,
    current: &Metrics,
    current_cases: usize,
) -> (Vec<String>, Vec<String>) {
    let Some(corpus) = corpus_baseline(baseline, name) else {
        return (
            Vec::new(),
            vec![format!(
                "基线（版本 {}）里没有语料「{name}」⇒ **不可比**（未被基线覆盖 ≠ 没掉）",
                baseline.version
            )],
        );
    };

    // 题数变了 ⇒ 分数不是同一把尺子量出来的 ⇒ 不比（但必须**说清楚**）。
    if corpus.cases != current_cases {
        return (
            Vec::new(),
            vec![format!(
                "语料「{name}」题数不同（基线 {} 题 ⇒ 当前 {} 题）⇒ **不可比** ⇒ 不判掉",
                corpus.cases, current_cases
            )],
        );
    }

    // 当前分数不是有限数（NaN / inf）⇒ 比不了 ⇒ 如实说「不可比」。
    // （否则 `drop > allowed` 对 NaN 恒为 false ⇒ **NaN 会静悄悄放行**。）
    let compared = [
        (
            "recall_at_k",
            corpus.metrics.recall_at_k,
            current.recall_at_k,
        ),
        ("ndcg_at_k", corpus.metrics.ndcg_at_k, current.ndcg_at_k),
    ];
    if compared.iter().any(|(_, _, cur)| !cur.is_finite()) {
        return (
            Vec::new(),
            vec![format!(
                "语料「{name}」当前分数不是有限数 ⇒ **不可比** ⇒ 不判掉"
            )],
        );
    }

    let mut dropped: Vec<String> = Vec::new();
    for (label, base, cur) in compared {
        let allowed = base * TOLERANCE_RATIO + TOLERANCE_FLOOR;
        let drop = base - cur;
        if drop > allowed {
            dropped.push(format!(
                "{label} 掉了 {drop:.6}（基线 {base:.6} → 当前 {cur:.6}；{}；允许上限 {allowed:.6}）",
                relative_text(base, drop)
            ));
        }
    }
    (dropped, Vec::new())
}

/// 相对劣化的文字说明（基线为 0 时没有百分比可言 ⇒ 如实说「基线为 0」）。
fn relative_text(base: f64, drop: f64) -> String {
    if base > 0.0 {
        format!("相对劣化 {:.2}%", drop / base * 100.0)
    } else {
        "基线为 0，无百分比可言".to_string()
    }
}

/// 解析基线 JSON（**格式错误 ⇒ 如实报错**，不 panic、不静默当空）。
///
/// # 必填字段
///
/// * 顶层：`version`（字符串）、`k`（整数）、`corpora`（数组）；
/// * 每条语料：`name`（非空字符串）、`recall_at_k`（数）、`ndcg_at_k`（数）、
///   `cases`（整数，**必填** —— 没有题数就分不清「分数变了」与「题变了」）。
///
/// 未知字段**忽略**（向前兼容：基线文件以后加字段不必先改本模块）。
pub fn parse_baseline(text: &str) -> Result<GoldenBaseline, String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|error| format!("基线 JSON 解析失败：{error}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| "基线 JSON 顶层必须是对象".to_string())?;

    let version = object
        .get("version")
        .ok_or_else(|| "基线缺少必填字段：version".to_string())?
        .as_str()
        .ok_or_else(|| "基线字段 version 必须是字符串".to_string())?
        .to_string();
    let k = object
        .get("k")
        .ok_or_else(|| "基线缺少必填字段：k".to_string())?
        .as_u64()
        .ok_or_else(|| "基线字段 k 必须是正整数".to_string())? as usize;
    let raw_corpora = object
        .get("corpora")
        .ok_or_else(|| "基线缺少必填字段：corpora".to_string())?
        .as_array()
        .ok_or_else(|| "基线字段 corpora 必须是数组".to_string())?;

    let mut corpora: Vec<CorpusBaseline> = Vec::with_capacity(raw_corpora.len());
    for (index, raw) in raw_corpora.iter().enumerate() {
        let corpus = raw
            .as_object()
            .ok_or_else(|| format!("基线 corpora[{index}] 必须是对象"))?;
        let name = corpus
            .get("name")
            .ok_or_else(|| format!("基线 corpora[{index}] 缺少必填字段：name"))?
            .as_str()
            .ok_or_else(|| format!("基线 corpora[{index}] 的 name 必须是字符串"))?
            .to_string();
        if name.trim().is_empty() {
            return Err(format!("基线 corpora[{index}] 的 name 不能为空"));
        }
        let recall_at_k = corpus
            .get("recall_at_k")
            .ok_or_else(|| format!("基线 corpora[{index}]（{name}）缺少必填字段：recall_at_k"))?
            .as_f64()
            .ok_or_else(|| format!("基线 corpora[{index}]（{name}）的 recall_at_k 必须是数字"))?;
        let ndcg_at_k = corpus
            .get("ndcg_at_k")
            .ok_or_else(|| format!("基线 corpora[{index}]（{name}）缺少必填字段：ndcg_at_k"))?
            .as_f64()
            .ok_or_else(|| format!("基线 corpora[{index}]（{name}）的 ndcg_at_k 必须是数字"))?;
        if !recall_at_k.is_finite() || !ndcg_at_k.is_finite() {
            return Err(format!(
                "基线 corpora[{index}]（{name}）的指标必须是有限数（不许 NaN / inf）"
            ));
        }
        let cases = corpus
            .get("cases")
            .ok_or_else(|| format!("基线 corpora[{index}]（{name}）缺少必填字段：cases"))?
            .as_u64()
            .ok_or_else(|| format!("基线 corpora[{index}]（{name}）的 cases 必须是正整数"))?
            as usize;
        if corpora.iter().any(|item| item.name == name) {
            return Err(format!("基线里有**重名**语料「{name}」⇒ 不知道该跟哪条比"));
        }
        corpora.push(CorpusBaseline::new(name, recall_at_k, ndcg_at_k, cases));
    }

    Ok(GoldenBaseline::new(version, k, corpora))
}

/// 序列化成基线 JSON（**确定性**：字段顺序固定）。
///
/// 字段顺序恒为 `version` → `k` → `corpora[]`（`name` → `recall_at_k` → `ndcg_at_k`
/// → `cases`）⇒ 两次调用**逐字相等**（同一份基线 ⇒ 同一个文件字节）。
///
/// ⚠️ **只写两项检索指标**（`recall_at_k` / `ndcg_at_k`）⇒ 证据族指标
/// **有意不落盘**（见模块头「口径」）⇒ [`parse_baseline`] 解析出来的其它指标字段恒为 0。
pub fn to_json(baseline: &GoldenBaseline) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!(
        "  \"version\": {},\n",
        json_string(&baseline.version)
    ));
    out.push_str(&format!("  \"k\": {},\n", baseline.k));
    if baseline.corpora.is_empty() {
        out.push_str("  \"corpora\": []\n}\n");
        return out;
    }
    out.push_str("  \"corpora\": [\n");
    let last = baseline.corpora.len() - 1;
    for (index, corpus) in baseline.corpora.iter().enumerate() {
        out.push_str("    {\n");
        out.push_str(&format!("      \"name\": {},\n", json_string(&corpus.name)));
        out.push_str(&format!(
            "      \"recall_at_k\": {},\n",
            json_f64(corpus.metrics.recall_at_k)
        ));
        out.push_str(&format!(
            "      \"ndcg_at_k\": {},\n",
            json_f64(corpus.metrics.ndcg_at_k)
        ));
        out.push_str(&format!("      \"cases\": {}\n", corpus.cases));
        out.push_str(if index == last { "    }\n" } else { "    },\n" });
    }
    out.push_str("  ]\n}\n");
    out
}

/// JSON 字符串字面量（含引号与转义）。
fn json_string(text: &str) -> String {
    // `serde_json` 对字符串的序列化不会失败 ⇒ `unwrap_or` 只是兜底。
    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string())
}

/// JSON 数字字面量（最短可往返表示；非有限数 ⇒ `null`，与 `serde_json` 一致）。
fn json_f64(value: f64) -> String {
    serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string())
}
