//! `apeireth-memory::robustness_metrics` — 设计稿 §6.1 的 **L3 鲁棒/成长**四项。
//!
//! # 为什么
//!
//! §6.4 明写「比『**稳定分**』不比『单次分』（**确定性优先**）」⇒
//! **确定性是所有分数的前提**；而「**越权命中率恒 0**」是**安全红线**
//! （D3 模式层只产标注、不产记忆、永不回写 ⇒ 越权写回 = 事故）。
//!
//! # 四项
//!
//! * **确定性（逐字节）**：同一输入跑两次 ⇒ 产出**逐字节相同**；
//! * **噪声鲁棒**：往语料里掺噪声 ⇒ 分数**不塌**（给出门限与实测跌幅）；
//! * **规模延展斜率**：语料规模 ×N ⇒ 延迟的增长**斜率**（不是绝对值）；
//! * **越权命中率**：**恒 0**（任何非 0 都是事故 ⇒ **单独报警**）。
//!
//! # 口径纪律（三条，别自创）
//!
//! 1. **确定性比的是「摘要文本」**（[`crate::golden_run::summarize`] 的输出）⇒
//!    **逐字节**比较两次的摘要（不是「看起来差不多」）；
//! 2. **噪声鲁棒比的是 `recall@k` 的相对跌幅**（不是绝对分）⇒
//!    「干净 0.8 → 掺噪 0.6」= 跌 **25%**（相对），不是跌 0.2；
//! 3. **规模延展给的是斜率不是绝对值**（`耗时比 ÷ 规模比`）⇒
//!    ⚠️ **耗时是墙钟** ⇒ 斜率**只用于看趋势**，**不做断言依据**。
//!
//! # 越权怎么算（**不许假设**）
//!
//! 本模块**不**替调用方判断「有没有越权」——它只做**计数与判定**：
//! `attempts`（模式层试图写回原始层的次数）与 `violations`（其中**真的改了原始层**的次数）
//! **必须由调用方实测得出**（例如：跑一遍 D4 标注流程，比对原始层的前后摘要）。
//! ⇒ 本模块是**尺子**，不是**探测器**（探测器要按各系统的真实结构写，照抄会测错东西）。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-27）
//!
//! 四级口径对照（本模块的真实位置）：
//!
//! - **IMPLEMENTED** ✔：类型、四项判定、报告渲染与测试齐备（**库级**实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：未接入 canonical 运行时主路径，生产 **0** 调用点；
//! - **DEFAULT ENABLED** ✘：没有开关位，也未默认启用；
//! - **HARDWARE VALIDATED** ✘：已在**真实语料**（《紅樓夢》/《水滸傳》）上跑过
//!   （见 `examples/robustness_report.rs`），但**只在单机**跑过 ⇒ 不宣称跨机器可复现。
//!
//! 对外一律按 **NOT IMPLEMENTED** 表述（新增未接线，不宣称可用）。

use std::time::Instant;

use crate::golden_run::{summarize, RunReport};

/// **确定性**的一次判定。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeterminismVerdict {
    /// 两次跑出来的结果是否**逐字节相同**。
    pub identical: bool,
    /// 第一次的**摘要文本**（便于人工比对）。
    pub digest_a: String,
    /// 第二次的摘要文本。
    pub digest_b: String,
}

/// **噪声鲁棒**的一次判定。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoiseVerdict {
    /// 干净语料上的 `recall@k`。
    pub clean_recall: f64,
    /// 掺噪后的 `recall@k`。
    pub noisy_recall: f64,
    /// **相对跌幅**（0.0 = 没掉；1.0 = 全掉）。
    pub drop_ratio: f64,
    /// 是否**扛住了**（判据：`drop_ratio <= max_drop`）。
    pub survived: bool,
}

/// **规模延展**的一次判定。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScaleVerdict {
    /// 语料条数（小）。
    pub small_n: usize,
    /// 语料条数（大）。
    pub large_n: usize,
    /// 小规模的耗时（毫秒）。
    pub small_ms: u64,
    /// 大规模的耗时（毫秒）。
    pub large_ms: u64,
    /// **延展斜率** = `large_ms / small_ms` ÷ `large_n / small_n`
    /// （= **每条成本的增长率** ⇒ 1.0 = 线性；> 1 = 超线性）。
    pub slope: f64,
}

/// **越权**的一次判定。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverreachVerdict {
    /// 越权次数（**必须恒 0**）。
    pub hits: usize,
    /// 越权的明细（**必须为空**；非空时逐条列出 ⇒ 便于定位）。
    pub details: Vec<String>,
    /// 是否**通过**（`hits == 0`）。
    pub passed: bool,
}

/// 判定**确定性**：同一输入跑两次 ⇒ 摘要是否逐字节相同。
///
/// # 口径
///
/// * 用**同一个** runner 跑两次（同一语料、同一卷子、同一 k）；
/// * 比较两次 `RunReport` 的**摘要文本**（`golden_run::summarize`）⇒ **逐字节**。
///
/// # 为什么比「摘要文本」而不是比分数
///
/// 分数是**四舍五入后的数字**（`{:.4}`）⇒ 两个不同的报告可能给出同一个分数
/// ⇒ 比分数会漏掉差异；而摘要文本**包含语料条数、题数、全中题数、四个指标**
/// ⇒ 文本逐字节相同 ⇒ 分数必然也相同（反之不必然）。
/// ⇒ 判据取**更严**的一边（文本相同才算确定）。
pub fn measure_determinism(runner: &dyn Fn() -> RunReport) -> DeterminismVerdict {
    let first = runner();
    let second = runner();
    let digest_a = summarize(&first);
    let digest_b = summarize(&second);
    DeterminismVerdict {
        identical: digest_a == digest_b,
        digest_a,
        digest_b,
    }
}

/// 判定**噪声鲁棒**。
///
/// # 口径
///
/// * 在语料里**掺噪声**（见 [`inject_noise`]）⇒ 跑一次 ⇒ 与干净分比；
/// * `max_drop` = **允许的最大相对跌幅**（默认给 **10%**）⇒ 超过 ⇒ `survived = false`。
///
/// # 相对跌幅怎么算
///
/// `drop_ratio = (clean - noisy) / clean` ⇒ **相对**口径（不是绝对分差）。
/// * `clean == 0` ⇒ **没有可掉的分** ⇒ 记 `0.0`（**不许** NaN，也不许除零）；
/// * `noisy > clean`（掺噪后反而更高）⇒ 记 `0.0`（**没有跌幅**，不是负跌幅）。
pub fn measure_noise_robustness(
    clean_runner: &dyn Fn() -> RunReport,
    noisy_runner: &dyn Fn() -> RunReport,
    max_drop: f64,
) -> NoiseVerdict {
    let clean_recall = clean_runner().metrics.recall_at_k;
    let noisy_recall = noisy_runner().metrics.recall_at_k;
    let drop_ratio = if clean_recall > 0.0 {
        ((clean_recall - noisy_recall) / clean_recall).clamp(0.0, 1.0)
    } else {
        0.0
    };
    NoiseVerdict {
        clean_recall,
        noisy_recall,
        drop_ratio,
        survived: drop_ratio <= max_drop,
    }
}

/// 判定**规模延展斜率**。
///
/// # 口径
///
/// * 两个规模的语料（`small_n` / `large_n`）各跑一次 ⇒ 记耗时；
/// * 斜率 = `(large_ms/small_ms) / (large_n/small_n)`
///   ⇒ **1.0 = 线性**；**> 1.5 = 超线性**（要警惕）。
/// ⚠️ **耗时是墙钟** ⇒ **只用于看趋势**，不做断言（断言用斜率门限）。
///
/// # 为什么用**毫秒**记耗时（以及不足 1 ms 怎么办）
///
/// * 墙钟 ⇒ **机器相关** ⇒ 数字只能**同机比较**（跨机不可比）；
/// * 毫秒粒度 ⇒ 跑得快的规模会被记成 `0` ⇒ **除零** ⇒ 记 `1` ms
///   （**不记 0**：0 ms 会让斜率变成无穷大，比「假装精确」更糟）。
pub fn measure_scale_slope(
    small_runner: &dyn Fn() -> RunReport,
    large_runner: &dyn Fn() -> RunReport,
    small_n: usize,
    large_n: usize,
) -> ScaleVerdict {
    let small_ms = elapsed_ms(small_runner);
    let large_ms = elapsed_ms(large_runner);
    let slope = scale_slope_from_ms(small_ms, large_ms, small_n, large_n);
    ScaleVerdict {
        small_n,
        large_n,
        small_ms,
        large_ms,
        slope,
    }
}

/// **斜率公式**（纯函数；把「公式」从「计时」里拆出来 ⇒ 可用**已知耗时**直接验算）。
///
/// # 口径
///
/// `slope = (large_ms / small_ms) ÷ (large_n / small_n)`
///
/// * `small_ms == 0` ⇒ 按 `1` ms 算（**不许除零**）；
/// * `small_n == 0` 或 `large_n == 0` ⇒ **没有规模增长可测** ⇒ 记 `1.0`（中性，
///   **不谎报**成超线性）。
pub fn scale_slope_from_ms(small_ms: u64, large_ms: u64, small_n: usize, large_n: usize) -> f64 {
    let time_ratio = large_ms as f64 / small_ms.max(1) as f64;
    if small_n == 0 || large_n == 0 {
        return 1.0;
    }
    let size_ratio = large_n as f64 / small_n as f64;
    time_ratio / size_ratio
}

/// 判定**越权命中率**。
///
/// # 口径
///
/// * `attempts` = 「模式层试图写回原始层」的次数（由调用方实测得出）；
/// * `violations` = 其中**真的改了原始层**的次数；
/// * ⇒ `hits = violations.len()`（**必须恒 0**）。
///
/// # `attempts` 为什么没进结构体
///
/// 契约把 `OverreachVerdict` 钉成三个字段 ⇒ `attempts` 只在**本函数内**用：
/// 做**口径自检**——「报出的越权条数 > 尝试次数」⇒ 调用方的计数口径自相矛盾
/// （不是越权被藏起来了，是计数坏了）⇒ 这种情况下**照样按 `hits` 报警**
/// （不隐藏、不减半），并在 `details` 里留痕，否则定位不到根因。
pub fn measure_overreach(attempts: usize, violations: Vec<String>) -> OverreachVerdict {
    let hits = violations.len();
    let mut details = violations;
    if hits > attempts {
        details.push(format!(
            "⚠️ 计数口径异常：尝试 {attempts} 次，却报出 {hits} 条越权（越权数 > 尝试数 ⇒ 计数口径坏了，不是「越权被隐藏」）"
        ));
    }
    OverreachVerdict {
        hits,
        details,
        passed: hits == 0,
    }
}

/// 往语料里掺噪声（**确定性**：用给定种子 ⇒ 同样的种子 ⇒ 同样的噪声）。
///
/// # 口径
///
/// * 对每条记录，按 `rate` 概率在正文里**插入**噪声片段（固定文案）；
/// * **用种子驱动的 PRNG**（**不许**用随机数 ⇒ 否则不可复现）。
///
/// # 几个刻意决定
///
/// * **PRNG 自带**（xorshift64，与仓里 `betti_hole_detector` 同口径）⇒ 不引新依赖、
///   同种子同结果（**跨机器同结果** ⇒ 噪声实验可复现）；
/// * **插入**而不是替换 ⇒ 原正文一字不少（噪声是「多了东西」，不是「改了东西」）
///   ⇒ 跌幅只可能来自检索被干扰，不会来自证据被删；
/// * `rate` 夹到 `[0, 1]` ⇒ 越界输入不会 panic（`0` ⇒ 原样不动，`1` ⇒ 每条都掺）。
pub fn inject_noise(records: &[(String, String)], rate: f64, seed: u64) -> Vec<(String, String)> {
    let rate = rate.clamp(0.0, 1.0);
    let mut rng = XorShift64::new(seed);
    let mut out = Vec::with_capacity(records.len());
    for (id, text) in records {
        if rate <= 0.0 {
            out.push((id.clone(), text.clone()));
            continue;
        }
        if rng.next_f64() >= rate {
            out.push((id.clone(), text.clone()));
            continue;
        }
        let chars: Vec<char> = text.chars().collect();
        if chars.is_empty() {
            out.push((id.clone(), text.clone()));
            continue;
        }
        // 插入点：按字符边界取（**不切坏 UTF-8**）⇒ 位置均匀覆盖全文。
        let position = (rng.next_u64() % (chars.len() as u64 + 1)) as usize;
        let mut noisy = String::with_capacity(text.len() + NOISE_FRAGMENT.len());
        noisy.extend(chars[..position].iter());
        noisy.push_str(NOISE_FRAGMENT);
        noisy.extend(chars[position..].iter());
        out.push((id.clone(), noisy));
    }
    out
}

/// 噪声片段的**固定文案**（所有记录共用一段 ⇒ 噪声本身**不携带信息**，
/// 测的纯粹是「检索被无关文本干扰后掉不掉分」）。
///
/// # 为什么公开
///
/// 「噪声是**插入**、原正文一字不少」这条性质**必须可核对** ⇒
/// 调用方要能「把这段噪声去掉」再与原文比对 ⇒ 文案不能是私有常量。
pub const NOISE_FRAGMENT: &str =
    "〔无关噪声：此处插入一段与本条记忆毫无关系的文字，用来检验检索在噪声下的鲁棒性。〕";

/// 人话报告（四项）。
///
/// # 必须包含
///
/// * 四项的结果（或「**没测到**」）；
/// * ⚠️ **越权非 0 ⇒ 报警措辞**（不粉饰）。
///
/// # 措辞纪律
///
/// * 「没测到」**不许**写成「没问题」⇒ 没测就是没测；
/// * **斜率是趋势，不是断言依据** ⇒ 这句话必须出现在规模延展那一栏
///   （墙钟受机器负载影响 ⇒ 拿它当断言依据会误伤）。
pub fn render_robustness_report(
    determinism: Option<&DeterminismVerdict>,
    noise: Option<&NoiseVerdict>,
    scale: Option<&ScaleVerdict>,
    overreach: Option<&OverreachVerdict>,
) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push("── L3 鲁棒/成长（四项）──".to_string());

    // ① 确定性
    match determinism {
        Some(verdict) if verdict.identical => lines.push(
            "【确定性（逐字节）】✓ 两次跑批的摘要文本**逐字节相同**（比的是 `golden_run::summarize` 的输出）"
                .to_string(),
        ),
        Some(verdict) => lines.push(format!(
            "【确定性（逐字节）】🚨 **不一致**：两次跑批的摘要文本**逐字节不同** ⇒ 这个系统的分数**不可复现**（先查随机源/哈希序/时钟）\n  A：{}\n  B：{}",
            first_line(&verdict.digest_a),
            first_line(&verdict.digest_b)
        )),
        None => lines.push("【确定性（逐字节）】**没测到**（不是「没问题」，是没跑）".to_string()),
    }

    // ② 噪声鲁棒
    match noise {
        Some(verdict) => {
            let mut line = format!(
                "【噪声鲁棒】{} 干净 recall={:.4} · 掺噪 recall={:.4} · 相对跌幅 {:.1}%",
                if verdict.survived {
                    "✓ 扛住了"
                } else {
                    "🚨 没扛住（分数塌了）"
                },
                verdict.clean_recall,
                verdict.noisy_recall,
                verdict.drop_ratio * 100.0
            );
            if !verdict.survived {
                line.push_str(" ⇒ 检索对噪声**不鲁棒**（真实语料有噪声 ⇒ 这条指标必须守住）");
            }
            lines.push(line);
        }
        None => lines.push("【噪声鲁棒】**没测到**（不是「没问题」，是没跑）".to_string()),
    }

    // ③ 规模延展
    match scale {
        Some(verdict) => {
            let mut line = format!(
                "【规模延展斜率】{} 语料 {} 条 / {} ms → {} 条 / {} ms ⇒ 斜率 {:.3}（1.0 = 线性；> 1.5 = 超线性）",
                if verdict.slope <= 1.5 { "✓" } else { "⚠" },
                verdict.small_n,
                verdict.small_ms,
                verdict.large_n,
                verdict.large_ms,
                verdict.slope
            );
            line.push_str("　⚠️ **斜率是趋势，不是断言依据**（耗时是墙钟 ⇒ 受机器负载影响）");
            lines.push(line);
        }
        None => lines.push("【规模延展斜率】**没测到**（不是「没问题」，是没跑）".to_string()),
    }

    // ④ 越权命中率（**恒 0** ⇒ 任何非 0 都是事故）
    match overreach {
        Some(verdict) if verdict.hits == 0 => {
            lines.push(
                "【越权命中率】✓ 越权 **0** 次 ⇒ 恒 0（安全红线守住：模式层没回写原始层）"
                    .to_string(),
            );
        }
        Some(verdict) => {
            let mut line = format!(
                "【越权命中率】🚨🚨 **事故**：越权 **{}** 次（必须恒 0）⇒ 模式层**回写了原始层**（D3 铁律被破）",
                verdict.hits
            );
            for detail in &verdict.details {
                line.push_str("\n  · ");
                line.push_str(detail);
            }
            lines.push(line);
        }
        None => lines.push("【越权命中率】**没测到**（不是「没问题」，是没跑）".to_string()),
    }

    lines.join("\n")
}

/// 取摘要文本的**第一行**（报告里只放一行 ⇒ 够定位，又不至于把报告刷屏）。
fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or("")
}

/// 计时：跑一次 runner ⇒ 毫秒（**不足 1 ms 记 1**，见 [`measure_scale_slope`] 的口径）。
fn elapsed_ms(runner: &dyn Fn() -> RunReport) -> u64 {
    let start = Instant::now();
    let _report = runner();
    let elapsed = start.elapsed();
    let millis = elapsed.as_millis();
    if millis == 0 {
        // ⚠️ 真的跑了一点点 ⇒ 记 1 ms（记 0 会让斜率除零 ⇒ 变成无穷大）。
        1
    } else {
        u64::try_from(millis).unwrap_or(u64::MAX)
    }
}

/// 种子驱动的 xorshift64 PRNG（**仓内自带**，不引新依赖）。
///
/// # 口径（与 `betti_hole_detector` 的同名件一致 ⇒ 全仓同一套随机口径）
///
/// * `x ^= x << 13; x ^= x >> 7; x ^= x << 17`；
/// * **种子 0 会让状态永远停在 0**（退化）⇒ 种子 0 映射到固定常量。
struct XorShift64(u64);

impl XorShift64 {
    /// 造一个 PRNG（种子 0 ⇒ 换成固定常量，避免退化流）。
    fn new(seed: u64) -> Self {
        const FALLBACK_SEED: u64 = 0x9E37_79B9_7F4A_7C15;
        Self(if seed == 0 { FALLBACK_SEED } else { seed })
    }

    /// 下一个 `u64`。
    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    /// 下一个 `[0, 1)` 的浮点（取高 53 位 ⇒ 均匀、且不落在 1.0）。
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}
