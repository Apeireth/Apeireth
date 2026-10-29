//! `apeireth-memory::forgetting_metrics` — 设计稿 §6.2 的四个「遗忘」指标。
//!
//! # 口径（**必须**与文档一致，且**不得**与「删除式遗忘」混用）
//!
//! * **降权正确率**：该往后排的是否真往后排；
//! * **保真可达率**：被降权/归档的记忆，明确需要时**仍能检索到**（「不丢」的机器证明）；
//! * **销毁合规率**：所有销毁有授权 + 墓碑；**越权销毁恒 0**；
//! * **秩位位移**：降权后条目下移多少位（**纯排序层，数据不动**）。
//!
//! # 一条纪律
//!
//! **不许**把这些数字与公开基准的「删除式遗忘」分数直接比较
//! （§6.2 的诚实要求：我们的口径是「降权 + 仍可检索」）。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-27）
//!
//! 四级口径（本模块的真实位置）：
//!
//! - **IMPLEMENTED** ✔：四个指标的类型 + 计算 + 汇总 + 人话报告齐备（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：**未接入**任何生产路径（生产 0 调用点；只有演练例子在用）；
//! - **DEFAULT ENABLED** ✘：没有开关位，也未默认启用；
//! - **HARDWARE VALIDATED** ✘：只在库级测试与**自建真实库**的演练里跑过，
//!   **未在生产记忆库上实测过**（本机无生产库）⇒ 不宣称已验证。
//!
//! 对外一律按 **NOT IMPLEMENTED** 表述（新增未接线，不宣称可用）。
//!
//! # 三个「不许」（本模块的诚实纪律，逐条落到代码里）
//!
//! 1. **不许用 0 冒充「完美」**：没有可算的数据 ⇒ 返回 `None`（「没测到」），不返回 `0.0`；
//! 2. **不许假设「还能检索到」**：保真可达率必须由调用方用**真实检索**测出
//!    （见 [`judge_retrievable`]：判据来自考卷 + 真实检索结果，**不许**假设）；
//! 3. **不许把越权销毁当「扣分项」**：越权销毁数**必须恒 0** ⇒ 非 0 即事故。

use crate::golden_set::{GoldenCase, GoldenSet};

/// 一条**降权**记录（用于算降权正确率与秩位位移）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownweightedItem {
    /// 记录 id。
    pub id: String,
    /// 降权**前**的名次（0 基，越小越靠前）。
    pub rank_before: usize,
    /// 降权**后**的名次（0 基）。
    pub rank_after: usize,
}

/// **保真可达率**的判定输入：某条记忆被降权/归档了。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedItem {
    /// 记录 id。
    pub id: String,
    /// 是否**已归档**（归档 ≠ 删除 ⇒ 归档件仍应可检索）。
    pub archived: bool,
    /// 是否**已销毁**（销毁 ⇒ **不要求**可达）。
    pub destroyed: bool,
    /// 明确需要时，检索**能不能找到它**（由调用方用真实检索测 ⇒ **不许假设**）。
    pub retrievable_when_needed: bool,
}

/// 一次**销毁**的记录（用于算销毁合规率）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DestructionRecord {
    /// 记录 id。
    pub id: String,
    /// 是否有**授权**（授权来源，如「用户请求」「治理批准」）。
    pub authorized: bool,
    /// 是否留了**墓碑**（可追溯 ⇒ 「销毁了但知道是谁销毁的」）。
    pub has_tombstone: bool,
    /// 是否属于**受保护**对象（protected persona ⇒ **不许销毁** ⇒ 出现即为越权）。
    pub protected: bool,
}

/// 四个指标的**一次测量**。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForgettingMetrics {
    /// **降权正确率**：降权后名次**变差**（变大）的比例（0.0 ~ 1.0）。
    ///
    /// ⚠️ 「该往后排的是否真往后排」⇒ 判据是**名次变大**（变差）⇒ **1.0 = 全部真往后排**。
    /// ⇒ 名次**没动**或**反而变好**都算**没做对**（都进分母、不进分子）。
    /// ⇒ **口径裁决**：本指标是「**做对的比例**」（**越接近 1 越好**），
    /// **不是**「违规比例」—— 两者互补（`违规比例 = 1 − 本指标`）。
    pub downweight_correct_rate: f64,
    /// ⭐ **保真可达率**：被降权/归档（**未销毁**）的记忆里，明确需要时**仍能检索到**的比例。
    pub reachable_rate: f64,
    /// **销毁合规率**：销毁里「有授权 + 有墓碑 + 非受保护」的比例。
    pub destruction_compliance_rate: f64,
    /// **越权销毁数**（**必须恒 0**）⇒ 任何非 0 都是**事故**。
    pub unauthorized_destructions: usize,
    /// **秩位位移**：降权条目名次变化的**中位数**（0 基；正数 = 往后排了）。
    pub median_rank_shift: i64,
}

/// 算**降权正确率**与**秩位位移**。
///
/// # 口径
///
/// * **降权正确率** = 「名次**变大**（变差）」的比例（**越接近 1 越好**）
///   ⇒ 未被降权的条目**不计入**分母（**由调用方保证**：只把真被降权的条目放进来；
///   本函数**不替调用方剔除**任何条目 —— 保守方向：宁可低估，
///   不许靠「跳过难题」把分刷高，与 [`crate::golden_set::evaluate`] 的 nDCG 口径同纪律）；
/// * **秩位位移** = 名次变化（`after - before`）的**中位数**
///   （**中位数**不用均值：一次异常会拖走均值 ⇒ 违反「比稳定分」纪律）。
///
/// # 偶数条的中位数
///
/// 偶数条 ⇒ 取两个中位数的**平均**并**四舍五入**到最近整数
/// （`.5` 远离 0；与 `f64::round` 同口径 ⇒ 可复核）。
///
/// # 空输入
///
/// ⇒ 返回 `None`（**没有可算的** ⇒ 不许返回 0.0 冒充「完美」）。
pub fn measure_downweight(items: &[DownweightedItem]) -> Option<(f64, i64)> {
    if items.is_empty() {
        return None;
    }
    let mut shifts: Vec<i64> = Vec::with_capacity(items.len());
    let mut worse = 0usize;
    for item in items {
        let before = i64::try_from(item.rank_before).unwrap_or(i64::MAX);
        let after = i64::try_from(item.rank_after).unwrap_or(i64::MAX);
        let shift = after.saturating_sub(before);
        if shift > 0 {
            worse += 1;
        }
        shifts.push(shift);
    }
    let rate = worse as f64 / items.len() as f64;
    let median = median_i64(&mut shifts);
    Some((rate, median))
}

/// 算**保真可达率**。
///
/// # 口径
///
/// * 分母 = 被降权/归档且**未销毁**的条目（**销毁的不要求可达** ⇒ 不进分母）；
/// * 分子 = 其中「明确需要时检索得到」的；
/// * ⚠️ **被销毁的条目不进分母**（否则等于要求「删掉的还能找到」⇒ 口径自相矛盾）。
///
/// # `archived` 字段为什么**不**进分母
///
/// 本切片里的条目**全都是**「被降权/归档」的（由**调用方**保证）；
/// `archived` 是**如实标注位**（「这条是真的移进了归档层」），
/// 不参与分母计算 —— 因为「降权」与「归档」**同样**要求「不丢」。
///
/// # 空输入
///
/// ⇒ 返回 `None`（**没有可算的** ⇒ 不许返回 0.0 冒充「完美」）。
pub fn measure_reachable(items: &[ArchivedItem]) -> Option<f64> {
    let denominator = items.iter().filter(|item| !item.destroyed).count();
    if denominator == 0 {
        return None;
    }
    let hits = items
        .iter()
        .filter(|item| !item.destroyed && item.retrievable_when_needed)
        .count();
    Some(hits as f64 / denominator as f64)
}

/// 算**销毁合规率**与**越权销毁数**。
///
/// # 口径
///
/// * 合规 = 「有授权」且「有墓碑」且「**非受保护**」;
/// * **越权销毁** = 「**受保护却被销毁**」**或**「无授权却被销毁」
///   ⇒ **必须恒 0**（任何非 0 都是事故）；
/// * **没留墓碑** = **账不全** ⇒ 只进**合规率**的分母（合规率掉），**不算越权**
///   ⇒ 口径裁决与理由见下。
///
/// # 计数口径：数的是「**多少次事故**」，不是「踩了几条规矩」
///
/// 一条记录**最多计 1 次**（即使它同时踩了多个违规条件）⇒ 事故数不会被放大。
///
/// # 口径裁决：「无墓碑」**不算**越权（**为什么**；本模块自定的口径）
///
/// §6.2 的原文是**两句话**：「**所有销毁有授权+墓碑**」（⇒ 合规率）
/// ＋「**越权销毁恒 0**」（⇒ 红线）⇒ 「有墓碑」属于**合规**那一句，
/// 「越权」属于**红线**那一句。
///
/// ⇒ 本实现取：**越权 = 「受保护却被销毁」或「无授权却被销毁」**（**动了不该动的东西**）；
/// **没留墓碑 = 账不全** ⇒ 只进**合规率**的分母，**不进**越权计数。
///
/// ## 为什么把两者分开（**理由**，不是口味）
///
/// 1. **越权是安全红线**：它的含义是「**动了用户明确要求保留的东西**」
///    ⇒ 监控盯这个数时必须能**一眼分清**「动了不该动的」与「账记漏了」
///    ⇒ 两类问题混在一起 ⇒ 红线的数字被**稀释** ⇒ 报警时分不出轻重；
/// 2. **两个指标不许互为补数**：若把「无墓碑」也算越权 ⇒
///    「越权数 = 1 − 合规率」**恒成立**（两者精确互补）⇒ 四个指标里有两个是同一个数
///    ⇒ 白占一个指标位；分开后，**合规率**与**越权数**是**两件独立的事**
///    （一次事故可以只掉合规率、也可以只破红线、也可以两者都破）；
/// 3. **账不全照样刺眼**：没留墓碑 ⇒ 合规率立刻掉（红线之外还有合规线在看它）
///    ⇒ 不会被漏掉。
///
/// ⚠️ **与契约草案的差异（如实标注）**：契约草案曾把「无墓碑」也列进越权
/// ⇒ 本实现**有意不同**（理由如上）⇒ 若裁决要求改回，改动点只有本函数里
/// `unauthorized` 的那一行筛选条件（`protected || !authorized` ⇒ 再加 `|| !has_tombstone`）。
///
/// # 空输入
///
/// ⇒ 返回 `(0.0, 0)`：**0.0 的含义是「没测到」**（**不是**「0% 合规」，也**不是**「100% 合规」）
/// ⇒ 保守方向（宁可低估）⇒ [`summarize`] 会把整组指标标成「**没测到**」，报告层不会出现冒充。
pub fn measure_destruction(records: &[DestructionRecord]) -> (f64, usize) {
    if records.is_empty() {
        return (0.0, 0);
    }
    let compliant = records.iter().filter(|record| is_compliant(record)).count();
    // ⚠️ **越权 = 「动了不该动的东西」**（**受保护却被销毁** 或 **无授权却被销毁**）
    // ⇒ **「没留墓碑」不算越权**（那是**账不全**，不是**越权**）。
    //
    // # 为什么把两者分开（口径裁决，2026-10-27）
    //
    // 「越权」是**安全红线**：它意味着**动了用户明确要求保留的东西**。
    // 若把「没留墓碑」也塞进「越权」⇒ 红线的数字被**稀释**
    // ⇒ 监控盯「越权数」时**分不清**「动了不该动的」与「账记漏了」
    // ⇒ ⇒ 两类问题**必须分开报**：
    // * **越权**（红线，**恒 0**）：受保护却被销毁 / 无授权却被销毁；
    // * **不合规**（账不全，进合规率分母）：三种情况之一不满足。
    let unauthorized = records
        .iter()
        .filter(|record| record.protected || !record.authorized)
        .count();
    (compliant as f64 / records.len() as f64, unauthorized)
}

/// 一次销毁**合不合规**（授权 + 墓碑 + 非受保护，三件齐）。
fn is_compliant(record: &DestructionRecord) -> bool {
    record.authorized && record.has_tombstone && !record.protected
}

/// 中位数（升序后取中间；偶数条取两个中位数的平均并四舍五入）。
fn median_i64(values: &mut [i64]) -> i64 {
    if values.is_empty() {
        return 0;
    }
    values.sort_unstable();
    let len = values.len();
    if len % 2 == 1 {
        return values[len / 2];
    }
    let low = values[len / 2 - 1];
    let high = values[len / 2];
    ((low as f64 + high as f64) / 2.0).round() as i64
}

/// 汇总成四个指标（**任一子项缺数据 ⇒ 那项为 `None`** ⇒ **不许用 0 冒充**）。
#[derive(Debug, Clone, PartialEq)]
pub struct ForgettingReport {
    /// 四个指标（`None` = **没测到**，**不是** 0）。
    pub metrics: Option<ForgettingMetrics>,
    /// 逐项说明（含「**没测到**」的原因 ⇒ **如实**）。
    pub notes: Vec<String>,
}

/// 汇总。
///
/// # 口径
///
/// * 三个子项（降权 / 保真可达 / 销毁）**各自**独立判定「有没有数据」；
/// * **任一子项缺数据 ⇒ 整组 `metrics` 为 `None`**（不许拿部分数字冒充一次完整测量）；
/// * 但 **notes 里仍然如实写出**已算出的子项数字与缺项原因（信息不许丢）。
pub fn summarize(
    downweighted: &[DownweightedItem],
    archived: &[ArchivedItem],
    destructions: &[DestructionRecord],
) -> ForgettingReport {
    let downweight = measure_downweight(downweighted);
    let reachable = measure_reachable(archived);
    let (compliance, unauthorized) = measure_destruction(destructions);

    let mut notes: Vec<String> = Vec::new();

    match downweight {
        Some((rate, shift)) => notes.push(format!(
            "降权正确率 = {rate:.4}（分母 = {} 条被降权条目；秩位位移中位数 = {shift} 位，正数 = 往后排了）",
            downweighted.len()
        )),
        None => notes.push(
            "降权正确率 / 秩位位移：**没测到**（没有可算的降权记录 ⇒ 不许用 0.0 冒充「完美」）"
                .to_string(),
        ),
    }

    let destroyed_count = archived.iter().filter(|item| item.destroyed).count();
    match reachable {
        Some(rate) => notes.push(format!(
            "保真可达率 = {rate:.4}（分母 = {} 条被降权/归档且**未销毁**的记忆；被销毁的 {destroyed_count} 条不进分母）",
            archived.len() - destroyed_count
        )),
        None => notes.push(
            "保真可达率：**没测到**（没有「未销毁」的被降权/归档条目 ⇒ 不许用 0.0 冒充「完美」）"
                .to_string(),
        ),
    }

    if destructions.is_empty() {
        notes.push(
            "销毁合规率 / 越权销毁数：**没测到**（没有销毁记录 ⇒ 0.0 的含义是「没测到」，不是「0% 合规」）"
                .to_string(),
        );
    } else {
        let compliant = destructions
            .iter()
            .filter(|record| is_compliant(record))
            .count();
        notes.push(format!(
            "销毁合规率 = {compliance:.4}（{compliant}/{} 条销毁「有授权 + 有墓碑 + 非受保护」）",
            destructions.len()
        ));
        if unauthorized == 0 {
            notes.push("越权销毁数 = 0（**必须恒 0**；非 0 即事故）".to_string());
        } else {
            // ⚠️ 名单必须列**真正越权**的那些（受保护却被销毁 / 无授权），
            // **不是**「所有不合规的」⇒ 不然「账不全」的事故会被混进红线名单。
            let ids: Vec<&str> = destructions
                .iter()
                .filter(|record| record.protected || !record.authorized)
                .map(|record| record.id.as_str())
                .collect();
            notes.push(format!(
                "🚨 越权销毁数 = {unauthorized}（**必须恒 0** ⇒ 这是事故）：{}",
                ids.join(", ")
            ));
        }
    }

    // ⚠️ **任一子项缺数据 ⇒ 整组为 `None`**（销毁子项没数据 ⇒ `compliance` 只是
    // 「没测到」的 0.0 ⇒ 不许把它塞进一次「完整测量」里冒充合规率）。
    let metrics = match (downweight, reachable) {
        (Some((rate, shift)), Some(reach)) if !destructions.is_empty() => Some(ForgettingMetrics {
            downweight_correct_rate: rate,
            reachable_rate: reach,
            destruction_compliance_rate: compliance,
            unauthorized_destructions: unauthorized,
            median_rank_shift: shift,
        }),
        _ => None,
    };

    ForgettingReport { metrics, notes }
}

/// 人话报告。
///
/// # 必须包含
///
/// * 四个数字（或「**没测到**」）；
/// * ⚠️ **口径提醒**：「本口径是『**降权 + 仍可检索**』，**不是**删除式遗忘
///   ⇒ **不得**与拟人系统的 selective forgetting 分数直接比较」；
/// * **越权销毁数**（若非 0 ⇒ **报警**）。
pub fn render_report(report: &ForgettingReport) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push("════════ 遗忘指标报告（设计稿 §6.2）════════".to_string());

    match &report.metrics {
        Some(metrics) => {
            lines.push(format!(
                "降权正确率：{:.4}（越接近 1 越好：被降权的条目真往后排了）",
                metrics.downweight_correct_rate
            ));
            lines.push(format!(
                "保真可达率：{:.4}（⭐ 被降权/归档且未销毁的记忆，明确需要时仍能检索得到）",
                metrics.reachable_rate
            ));
            lines.push(format!(
                "销毁合规率：{:.4}（有授权 + 有墓碑 + 非受保护）",
                metrics.destruction_compliance_rate
            ));
            lines.push(format!(
                "越权销毁数：{}（**必须恒 0**；非 0 即事故）",
                metrics.unauthorized_destructions
            ));
            lines.push(format!(
                "秩位位移：中位数 {:+} 位（正数 = 往后排了；纯排序层，数据不动）",
                metrics.median_rank_shift
            ));
            if metrics.unauthorized_destructions > 0 {
                lines.push(format!(
                    "🚨🚨🚨 **越权销毁 {} 条 —— 事故**（红线：越权销毁恒 0）",
                    metrics.unauthorized_destructions
                ));
            }
        }
        None => {
            lines.push("降权正确率：**没测到**（不是 0）".to_string());
            lines.push("保真可达率：**没测到**（不是 0）".to_string());
            lines.push("销毁合规率：**没测到**（不是 0）".to_string());
            lines.push("越权销毁数：**没测到**（不是 0）".to_string());
            lines.push("秩位位移：**没测到**（不是 0）".to_string());
            lines.push(
                "⚠️ 至少一个子项缺数据 ⇒ 整组指标不报数（不许拿部分数字冒充一次完整测量）"
                    .to_string(),
            );
        }
    }

    lines.push(String::new());
    lines.push("── 口径提醒（不许删）──".to_string());
    lines.push(
        "⚠️ 本口径是「**降权 + 仍可检索**」，**不是**删除式遗忘 ⇒ **不得**与拟人系统的 selective forgetting 分数直接比较。"
            .to_string(),
    );

    if !report.notes.is_empty() {
        lines.push(String::new());
        lines.push("── 逐项说明 ──".to_string());
        for note in &report.notes {
            lines.push(format!("· {note}"));
        }
    }

    lines.join("\n")
}

/// 用**真实检索**判定「明确需要时**仍能检索到**」。
///
/// # 为什么要有这个入口（不许假设）
///
/// [`ArchivedItem::retrievable_when_needed`] **不许**由调用方「假设」成 `true`
/// ⇒ 判定必须落在**真实检索**的结果上。
/// 本函数提供「怎么算」的**机械口径**，「检索到什么」由调用方的**真实检索面**提供
/// （闭包）⇒ 于是「明确需要」的依据（考卷）与「找没找到」的依据（真实检索）**都在**。
///
/// # 口径
///
/// * 「明确需要」的客观依据 = 考卷里 [`GoldenCase::expect_ids`] 含该 id 的题目；
/// * 可达 = 这些题目里**每一道**的真实检索结果里都出现了该 id
///   （**保守方向**：宁可低估，不许靠「只有一道题好答」把分数刷高）；
/// * 一道都没有 ⇒ **没有判定依据** ⇒ 返回 `false`（保守：不许凭空说「可达」）。
pub fn judge_retrievable(
    item_id: &str,
    golden: &GoldenSet,
    mut retrieve: impl FnMut(&GoldenCase) -> Vec<String>,
) -> bool {
    let needed: Vec<&GoldenCase> = golden
        .cases
        .iter()
        .filter(|case| case.expect_ids.iter().any(|id| id.trim() == item_id))
        .collect();
    if needed.is_empty() {
        return false;
    }
    needed
        .iter()
        .all(|case| retrieve(*case).iter().any(|id| id == item_id))
}
