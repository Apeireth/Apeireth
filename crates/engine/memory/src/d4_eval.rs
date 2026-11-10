//! `apeireth-memory::d4_eval` — D4 模式层的**离线评估器**。
//!
//! # 为什么要有这个模块
//!
//! 设计稿 `docs/01-architecture/memory-knowledge-depth-system-spec.md` §3.5 定的纵深体系里，
//! **D4（模式层）**住着那些「写好了、纯数学、默认关、零消费」的实验算法：
//! 五维时空 / 程序性记忆 / 反思闭环 / 做梦 / 社区分层 / `hallways` / `layered_memo` /
//! `ensemble` 预测市场 / `intent_brier` / `online_calibration` / `calibration_critic` 等。
//!
//! owner 的核心意图是：「那些没接线的算法和想法都算今天今人的智慧啊，
//! 我们完全可以建立记忆知识纵深体系」⇒ 处置**不是「删掉」**，而是
//! **给它们一个位置 + 让它们能被公平评估**。
//!
//! 本模块就是那把「尺子」：**不接线也能量出「某个 D4 算法开启后会怎样」**，
//! 于是 owner 可以**按数字**决定「开哪个」，而不是按嗓门大小决定。
//! 这也是设计稿 §8 **B0-c** 批次的验收口径（「36 个模块进候选池并各有一行数字」）。
//!
//! # 本模块只做「评估与判定」，**不实现任何 D4 算法**
//!
//! [`AlgoProfile`] 是**画像**（描述算法要什么、产什么），不是实现。
//! 于是：
//!
//! - **写域零冲突**：不碰任何既有 D4 模块（`five_dimensional` / `procedural` / `ensemble` …）；
//! - **未来接得进来**：每个 D4 算法接线时，只需在**调用方**（生产装配或离线脚本）
//!   构造 [`AlgoProfile`] + 跑一次 [`evaluate`] 就能拿到结论；
//! - **判定口径唯一**：全仓只有这一处定义「值得开」的判据，不散落在各人脑子里。
//!
//! # 判据（照抄设计稿 §6.4「不许变差」+ 增量价值，别自创）
//!
//! 1. [`AlgoVerdict::passes_regression_gate`] =
//!    `check_no_regression(baseline, with_algo)` 是否为 `Ok`
//!    （准确度不许掉 ⇒ 掉分就是不准开）；
//! 2. [`AlgoVerdict::delta_primary`] = `with_algo.recall_at_k - baseline.recall_at_k`；
//! 3. [`AlgoVerdict::worth_enabling`] = 通过闸 **且** `delta_primary > 0`
//!    ⇒ **delta 恰好为 0 ⇒ 不值得开**（没有收益 ⇒ 白付成本）。
//!
//! ⚠️ 「不许变差」闸允许 **1% 的相对劣化容忍度**（口径在 `golden_set::check_no_regression`），
//! 所以可能出现「过了闸但 delta < 0」——此时**仍然不值得开**
//! （在容忍度里 ≠ 有增量价值），[`AlgoVerdict::rationale`] 会如实写清。
//!
//! # 关键指标 = `recall_at_k`
//!
//! D4 的价值主要体现在「该找到的找得更全」⇒ 关键指标取 [`PRIMARY_METRIC`]。
//! 注意它是**字段访问**（`metrics.recall_at_k`）而不是按名字动态取 ——
//! 名字只是给人看的标签，取值路径写死 ⇒ 编译期就能发现口径漂移。
//!
//! # 契约缺口：`EvalInput` 收 `EvalBatch`，结论却要给 `Metrics`
//!
//! 契约钦定 [`EvalInput`] 的 `baseline` / `with_algo` 是**原始批次**（`cases` + `results`），
//! 而 [`AlgoVerdict`] 的同名字段是**算好的指标**（`Metrics`）⇒ 本模块必须**自己**把批次算成指标。
//! 算指标要用 `golden_set::evaluate(&batch, k)`，而契约**没有给 `k` 字段** ⇒
//! 本模块用 [`DEFAULT_K`]（=10）作为默认截断深度，并额外提供 [`evaluate_with_k`]
//! 让调用方按需覆盖。**契约未规定的那一格，就在这里显式钉死**（不猜、不藏）。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-26）
//!
//! 四级口径对照（本模块的真实位置）：
//!
//! - **IMPLEMENTED** ✔：类型、评估与判定逻辑、测试齐备（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：未接入 canonical 运行时主路径，生产 0 调用点；
//! - **DEFAULT ENABLED** ✘：D4 全部默认关；本模块只是**离线尺子**，不引入任何开关位；
//! - **HARDWARE VALIDATED** ✘：尚未在真实卷子数据上跑过（卷子数据未到位）。
//!
//! 对外一律按 **NOT IMPLEMENTED** 表述（新增未接线，不宣称可用）。

// `GoldenCase` 用不到（批次自带题目）⇒ 不 import（避免 unused import 噪音）。
// `evaluate` 改名 `score_batch`：与本模块的 `evaluate` 同名会歧义。
use crate::golden_set::{
    check_no_regression, evaluate as score_batch, EvalBatch, GoldenSet, Metrics,
};

/// 一个 D4 算法的**离线画像**（描述它能做什么，不含具体实现）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlgoProfile {
    /// 算法 id（如 `betti_hole_detector` / `kuramoto_resonance` / …）。
    pub id: String,
    /// 人话标题（中文）。
    pub title: String,
    /// 算法**消费**什么（决定它能不能在这批记录上跑）。
    pub consumes: AlgoInput,
    /// 算法**产出**什么（标注种类）。
    pub produces: AnnotationKind,
    /// 额外成本提示（用于算「值不值」；如 `Some("每轮额外 1 次全量重算")`）。
    pub cost_hint: Option<String>,
}

/// 算法要消费的东西（决定可运行性）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlgoInput {
    /// 只需要记录文本（最便宜）。
    Text,
    /// 需要记录 + 检索结果（要看排序）。
    TextAndRanking,
    /// 需要记录 + 检索结果 + 注入文本（要看最终产物）。
    FullPipeline,
}

/// 标注种类（D4 只产标注，不产记忆）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnotationKind {
    /// 标注「这段文本的认知空洞在哪」。
    Gap,
    /// 标注「这些内容相关联」。
    Association,
    /// 标注「这是值得沉淀的模式」。
    Pattern,
    /// 标注「这条的置信度/新颖度」。
    Score,
}

/// 一次离线评估的输入。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalInput {
    /// 被评估的算法。
    pub profile: AlgoProfile,
    /// 考卷。
    pub golden: GoldenSet,
    /// **基线**结果（不开启任何 D4 时的检索表现）。
    pub baseline: EvalBatch,
    /// **开启该 D4 后**的结果。
    pub with_algo: EvalBatch,
}

/// 一次离线评估的结论。
///
/// ⚠️ **不派生 `Eq`**：本结构含 `f64` 字段（`delta_primary`），
/// 而 `f64` **没有** `Eq` 实现（浮点没有全序等价）⇒ 派 `Eq` 编译不过。
/// 契约原文写的是 `PartialEq, Eq`，此处按**可编译的最小改动**只保留 `PartialEq`
/// （`Metrics` 本身也只有 `PartialEq`）。
#[derive(Debug, Clone, PartialEq)]
pub struct AlgoVerdict {
    /// 被评估算法的 id（与 [`AlgoProfile::id`] 一致）。
    pub algo_id: String,
    /// 基线指标。
    pub baseline: Metrics,
    /// 开启后指标。
    pub with_algo: Metrics,
    /// **增量价值** = 关键指标提升（见 [`PRIMARY_METRIC`]）。
    /// 正数 = 变好；负数 = 变坏；零 = 没变化。
    pub delta_primary: f64,
    /// **关键指标**的名字（可机检的常量，如 `recall_at_k`）。
    pub primary_metric: String,
    /// **结论**：能不能通过「不许变差」闸（调用 `check_no_regression`）。
    pub passes_regression_gate: bool,
    /// **是否值得开**（按增量价值与成本提示综合；规则见 [`evaluate`]）。
    pub worth_enabling: bool,
    /// 人话结论（如「开启后 recall 提升 0.03，成本：每轮额外 1 次全量重算」）。
    pub rationale: String,
}

/// **关键指标**：D4 的价值主要体现在「该找到的找得更全」。
pub const PRIMARY_METRIC: &str = "recall_at_k";

/// 默认的 k（`recall_at_k` / `ndcg_at_k` 的截断深度）。
///
/// **为什么需要它**：契约钦定 [`EvalInput`] 收原始批次（`EvalBatch`），
/// 而结论要给算好的指标（`Metrics`）⇒ 本模块必须自己算；
/// 算指标必须给 `k`，而契约没给 ⇒ 这里**显式钉死默认值 10**（不猜、不藏）。
/// 需要别的深度请用 [`evaluate_with_k`]。
pub const DEFAULT_K: usize = 10;

/// 离线评估：比较基线与开启后的表现，给出结论（`k` 取 [`DEFAULT_K`]）。
///
/// # 判据（照此实现，别自创）
///
/// 1. `passes_regression_gate` = `check_no_regression(baseline, with_algo)` 是否为 Ok
///    （准确度不许掉 ⇒ 掉分就是不准开）；
/// 2. `delta_primary` = `with_algo.primary - baseline.primary`；
/// 3. `worth_enabling` = `passes_regression_gate` **且** `delta_primary > 0`
///    ⇒ 「不倒退且有提升」才值得开。
///    ⚠️ **delta 恰好为 0 ⇒ 不值得开**（没有收益 ⇒ 白付成本）。
/// 4. `rationale` 必须**如实**写出：提升/退步的数值 + 成本提示（如有）。
///
/// # 为什么「画像」不参与判定
///
/// [`AlgoProfile::consumes`] / [`AlgoProfile::produces`] 只描述算法**能不能跑**、
/// **产出什么**，它们**不改变**判定结果 —— 判定只看「同一份卷子上的检索表现变化」。
/// 理由：画像不同 ⇒ 该跑不同的实验，但**量出来的尺子必须是同一把**；
/// 否则「甲算法涨 0.02」和「乙算法涨 0.02」就没法放在一张表里比。
///
/// # 确定性
///
/// 纯函数：不读时钟、不读环境变量、不用随机数 ⇒ 同一输入两次调用**逐字相等**。
pub fn evaluate(input: EvalInput) -> AlgoVerdict {
    evaluate_with_k(input, DEFAULT_K)
}

/// 离线评估（显式指定 `k`）。
///
/// 与 [`evaluate`] 同口径，只是把 `recall_at_k` / `ndcg_at_k` 的截断深度交回给调用方。
/// **存在的理由**：契约没有 `k` 字段（见 [`DEFAULT_K`] 的说明），
/// 需要别的深度时至少有明面上的出口，而不是让调用方去改常量。
pub fn evaluate_with_k(input: EvalInput, k: usize) -> AlgoVerdict {
    let EvalInput {
        profile,
        golden,
        baseline,
        with_algo,
    } = input;

    // 批次 → 指标（契约缺口在这一格显式补上；评分失败 ⇒ 指标按 0 计并如实写进 rationale）。
    let (base_metrics, base_warn) = score(&baseline, k);
    let (algo_metrics, algo_warn) = score(&with_algo, k);

    // ① 不许变差闸：准确度掉分 ⇒ 不准开（口径在 golden_set::check_no_regression）。
    let passes_regression_gate = check_no_regression(&base_metrics, &algo_metrics).is_ok();

    // ② 增量价值：关键指标之差（字段访问，不按名字动态取）。
    let delta_primary = algo_metrics.recall_at_k - base_metrics.recall_at_k;

    // ③ 值得开 = 不倒退 **且** 有提升。
    //    ⚠️ delta 恰好为 0 ⇒ 不值得开（没有收益 ⇒ 白付成本）。
    let worth_enabling = passes_regression_gate && delta_primary > 0.0;

    let rationale = build_rationale(
        &profile,
        &golden,
        &base_metrics,
        &algo_metrics,
        delta_primary,
        passes_regression_gate,
        worth_enabling,
        base_warn.as_deref(),
        algo_warn.as_deref(),
    );

    AlgoVerdict {
        algo_id: profile.id.clone(),
        baseline: base_metrics,
        with_algo: algo_metrics,
        delta_primary,
        primary_metric: PRIMARY_METRIC.to_string(),
        passes_regression_gate,
        worth_enabling,
        rationale,
    }
}

/// 批量评估：按 `id` 排序输出（确定性）。
///
/// **为什么排序**：同一批候选换个输入顺序跑，报告必须**逐字一样**，
/// 否则「这版比上版好在哪」会被顺序噪声淹没。排序键取算法 `id`（全局唯一且稳定）。
pub fn evaluate_all(inputs: Vec<EvalInput>) -> Vec<AlgoVerdict> {
    let mut verdicts: Vec<AlgoVerdict> = inputs.into_iter().map(evaluate).collect();
    // `sort_by` 是稳定排序 ⇒ 同 id 的候选之间也保持输入次序（不会互相吃掉）。
    verdicts.sort_by(|a, b| a.algo_id.cmp(&b.algo_id));
    verdicts
}

/// 一句话汇总（给 CLI/报告用）：如「3 个候选：2 个值得开，1 个不通过不变差闸」。
///
/// # 为什么分三类而不是两类
///
/// 「没通过不变差闸」（掉分，不准开）与「过了闸但没提升」（不倒退也没收益，白付成本）
/// 是**两件不同的事** ⇒ 分开报，owner 才能一眼看出「是算法没用」还是「算法用错了地方」。
/// 零项的类别不打印（避免「0 个不通过不变差闸」这种噪音）。
pub fn summarize(verdicts: &[AlgoVerdict]) -> String {
    let total = verdicts.len();
    if total == 0 {
        return "0 个候选：没有可评估的 D4 算法".to_string();
    }

    let worth = verdicts.iter().filter(|v| v.worth_enabling).count();
    let failed_gate = verdicts
        .iter()
        .filter(|v| !v.passes_regression_gate)
        .count();
    // 过闸但不值得开 ⇒ 只可能是「零增量」（含 1% 容忍度内的微小退步）。
    let no_gain = verdicts
        .iter()
        .filter(|v| v.passes_regression_gate && !v.worth_enabling)
        .count();

    let mut out = format!("{total} 个候选：{worth} 个值得开");
    if failed_gate > 0 {
        out.push_str(&format!("，{failed_gate} 个不通过不变差闸"));
    }
    if no_gain > 0 {
        out.push_str(&format!("，{no_gain} 个无增量（不变差但没提升）"));
    }
    out
}

// ============================================
// 内部工具
// ============================================

/// 批次 → 指标（`golden_set::evaluate` 的薄封装 + 失败兜底）。
///
/// **为什么失败不 panic 也不静默**：批次 `cases` 与 `results` 长度不等 ⇒ 指标算不出来。
/// 此时**按 0 计**并把原因带出去（`rationale` 会如实写）——
/// 「算不出分数」绝不能被当成「分数是 0 的正常结果」混进决策。
fn score(batch: &EvalBatch, k: usize) -> (Metrics, Option<String>) {
    match score_batch(batch, k) {
        Ok(metrics) => (metrics, None),
        Err(err) => (
            Metrics {
                total_cases: batch.cases.len(),
                ..Metrics::default()
            },
            Some(format!("批次无法评分（{err}）⇒ 指标按 0 计")),
        ),
    }
}

/// 拼一句**如实**的人话结论。
///
/// 如实 = 数值必须来自实际计算（不夸大、不省略）：
/// - 方向词（提升/退步/无变化）由 `delta` 的符号决定；
/// - 数值一律带 6 位小数（机器可断言、人类可读）；
/// - 成本提示**有就写**（这是「值不值」的另一半），没有就明说「未标注」；
/// - 闸没过 ⇒ 明说「不准开」，不许用「基本持平」这种含糊词掩盖掉分。
#[allow(clippy::too_many_arguments)]
fn build_rationale(
    profile: &AlgoProfile,
    golden: &GoldenSet,
    baseline: &Metrics,
    with_algo: &Metrics,
    delta_primary: f64,
    passes_regression_gate: bool,
    worth_enabling: bool,
    baseline_warning: Option<&str>,
    with_algo_warning: Option<&str>,
) -> String {
    let direction = if delta_primary > 0.0 {
        "提升"
    } else if delta_primary < 0.0 {
        "退步"
    } else {
        "无变化"
    };

    let gate_text = if passes_regression_gate {
        "通过『不许变差』闸"
    } else {
        "不通过『不许变差』闸（准确度不许掉 ⇒ 不准开）"
    };

    let worth_text = if worth_enabling {
        "值得开".to_string()
    } else if !passes_regression_gate {
        "不准开（掉分就是不准开）".to_string()
    } else {
        // 过了闸却没收益：要么 delta = 0，要么 delta < 0 但在 1% 容忍度内。
        format!(
            "不值得开（关键指标无增量：{}，没有收益 ⇒ 白付成本）",
            format_delta(delta_primary)
        )
    };

    let cost_text = match &profile.cost_hint {
        Some(cost) => format!("成本：{cost}"),
        None => "成本：未标注（画像没写成本提示）".to_string(),
    };

    let mut warnings = String::new();
    for warning in [baseline_warning, with_algo_warning].into_iter().flatten() {
        warnings.push_str(&format!("；注意：{warning}"));
    }

    format!(
        "「{}·{}」{}开启后 {} {} {}（基线 {:.6} → 当前 {:.6}）；{}；{}；{}{}",
        profile.id,
        profile.title,
        golden_note(golden),
        PRIMARY_METRIC,
        direction,
        format_delta(delta_primary),
        baseline.recall_at_k,
        with_algo.recall_at_k,
        gate_text,
        worth_text,
        cost_text,
        warnings,
    )
}

/// 卷子说明（版本 + 题数）。
///
/// 版本号可能为空（调用方自造卷子时没给）⇒ 那就如实说「无版本号」，
/// 不编一个假版本出来。
fn golden_note(golden: &GoldenSet) -> String {
    let version = golden.version.trim();
    let count = golden.cases.len();
    if version.is_empty() {
        format!("在无版本号卷子（{count} 题）上：")
    } else {
        format!("在卷子 {version}（{count} 题）上：")
    }
}

/// 增量数值的格式化：**带符号、6 位小数**。
///
/// 为什么要显式带符号：`+0.030000` 与 `-0.030000` 一眼可分，
/// 而 `0.030000` 看不出是涨是跌；机器断言也更好写。
/// 零单独走一支（避免写成 `+0.000000`，那看着像「涨了 0」）。
fn format_delta(delta: f64) -> String {
    if delta > 0.0 {
        format!("+{delta:.6}")
    } else if delta < 0.0 {
        format!("{delta:.6}")
    } else {
        "0.000000".to_string()
    }
}
