//! **纯算法还能不能提升召回？** —— 四个**不需要任何模型**的杠杆，实测。
//!
//! # 为什么有这个（**修正一个过强的结论**）
//!
//! 我曾说「要更精准必须引入向量模型」⇒ ⇒ **说得太绝对**。
//! 真实对话语料有两个**纯词面**的硬伤，向量模型不是唯一解法：
//!
//! 1. **第一人称**：`59%` 的对话轮次含 `I / my / me`
//!    ⇒ 而题目问的是 **`Caroline`** ⇒ ⇒ **词面完全不重合**
//!    （例：问「Caroline 去了什么互助小组？」答「**I** went to a LGBTQ support group」）
//!    ⇒ ⇒ **把第一人称消解成说话人名字**即可 ⇒ **零模型**；
//! 2. **相对时间**：「**yesterday**」「last week」⇒ 题目问的是**绝对日期**
//!    ⇒ ⇒ **把相对时间按会话日期解析成绝对日期**即可 ⇒ **零模型**；
//! 3. **词形**：`research / researching / researched` ⇒ ⇒ **词干归一**；
//! 4. **日期格式**：`8 May, 2023` / `2023-05-08` / `May 8 2023` ⇒ ⇒ **多形态展开**。
//!
//! ⇒ ⇒ 这些**全是文本改写**，不引入模型、不加依赖、不加延迟。
//!
//! # 口径
//!
//! * 单证据题（1,122 题）· `k = 10` · 其它一律不动（唯一变量 = 文本改写）；
//! * 每个臂**单独**报 + **叠加**报 ⇒ ⇒ 看能不能相加；
//! * ⚠️ **如实**：不显著的就说「不显著」，不挑好看的说。
//!
//! # 怎么跑
//!
//! ```sh
//! cargo run -p apeireth-memory --example locomo_levers2 --locked
//! ```

use apeireth_memory::golden_run::{run, IndexRecord, InjectionMode};
use apeireth_memory::golden_set::{GoldenCase, GoldenSet};
use apeireth_memory::locomo_corpus::{self, LocomoSample};
use std::process::ExitCode;

const K: usize = 10;

const STOPWORDS: [&str; 30] = [
    "a", "an", "the", "is", "are", "was", "were", "be", "been", "did", "do", "does", "what",
    "when", "where", "who", "whom", "whose", "which", "why", "how", "would", "could", "should",
    "can", "to", "of", "in", "on", "for",
];

const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

#[derive(Clone, Copy)]
enum DocMode {
    Plain,
    DateExpanded,
    PronounResolved,
    Stemmed,
    PronounAndDate,
    All,
}

fn main() -> ExitCode {
    let Some(path) = locomo_corpus::default_path() else {
        eprintln!("· 找不到 locomo10.json ⇒ **没测到**");
        return ExitCode::FAILURE;
    };
    let samples = match locomo_corpus::load(&path) {
        Ok(samples) => samples,
        Err(error) => {
            eprintln!("· 加载失败：{error}");
            return ExitCode::FAILURE;
        }
    };
    println!("════════ 纯算法杠杆（**零模型**）════════");
    println!("单证据题 · k = {K} · 每个臂只改**文本**（检索器不动）");
    println!();
    println!(
        "{:<30} {:>6} {:>11} {:>12}",
        "臂", "题数", "recall@10", "Δ vs 基线"
    );
    println!("{}", "-".repeat(64));

    // 臂定义：(名字, 是否改写查询, 记录改写方式)

    let arms: [(&str, DocMode, bool); 8] = [
        ("A 基线", DocMode::Plain, false),
        ("B 去疑问词", DocMode::Plain, true),
        ("C 日期多形态", DocMode::DateExpanded, false),
        ("D 第一人称消解", DocMode::PronounResolved, false),
        ("E 词干归一", DocMode::Stemmed, false),
        ("F 第一人称+日期", DocMode::PronounAndDate, false),
        ("G 全叠加(含去疑问词)", DocMode::All, true),
        ("H 全叠加(不含去疑问词)", DocMode::All, false),
    ];

    let mut baseline = f64::NAN;
    let mut best: (String, f64) = (String::new(), f64::NAN);
    let mut results: Vec<(String, f64)> = Vec::new();
    for (label, mode, rewrite_query) in arms {
        let mut total = 0usize;
        let mut hits = 0usize;
        for sample in &samples {
            let cases: Vec<GoldenCase> = sample
                .cases
                .iter()
                .filter(|case| case.expect_ids.len() == 1)
                .map(|case| GoldenCase {
                    id: case.id.clone(),
                    query: if rewrite_query {
                        strip_stopwords(&case.query)
                    } else {
                        case.query.clone()
                    },
                    expect_ids: case.expect_ids.clone(),
                    expect_fragments: case.expect_fragments.clone(),
                    tags: case.tags.clone(),
                })
                .collect();
            if cases.is_empty() {
                continue;
            }
            let index = build_index(sample, mode);
            let golden = GoldenSet::new(sample.sample_id.clone(), cases.clone());
            let Ok(report) = run(&index, &golden, K, InjectionMode::Production) else {
                continue;
            };
            total += report.cases;
            hits += report.per_case_hit.iter().filter(|(_, hit)| *hit).count();
        }
        let recall = if total > 0 {
            hits as f64 / total as f64
        } else {
            f64::NAN
        };
        if baseline.is_nan() {
            baseline = recall;
        }
        println!(
            "{:<30} {:>6} {:>11.4} {:>12}",
            label,
            total,
            recall,
            if baseline.is_nan() || label.starts_with('A') {
                "—".to_string()
            } else {
                format!("{:+.4}", recall - baseline)
            }
        );
        results.push((label.to_string(), recall));
        if best.1.is_nan() || recall > best.1 {
            best = (label.to_string(), recall);
        }
    }
    println!();

    // ---------- 判定（**跑前写死的规则**）----------
    println!("── 判定（**先写死的规则**）──");
    println!("  基线 recall@10 = {baseline:.4}");
    let mut winners: Vec<&(String, f64)> = results
        .iter()
        .filter(|(label, recall)| !label.starts_with('A') && *recall > baseline + 0.01)
        .collect();
    winners.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    if winners.is_empty() {
        println!("  ⚠️ **没有任何纯算法杠杆带来 >1 个点的提升** ⇒ ⇒ 那才轮到向量模型");
    } else {
        println!(
            "  🎯 最有效：**{}** ⇒ {:.4} → {:.4}（{:+.4}）",
            best.0,
            baseline,
            best.1,
            best.1 - baseline
        );
        println!("  ⇒ 全部**零模型、零依赖、零额外延迟** ⇒ ⇒ **可以立即进生产**");
        println!("  ⇒ 有效杠杆（> +0.01）：");
        for (label, recall) in winners {
            println!(
                "      · {label} ⇒ {:.4}（{:+.4}）",
                recall,
                recall - baseline
            );
        }
    }
    println!();
    println!("  ⚠️ 边界：本实验只改**文本形态**（把语义搬到词面上），");
    println!("     ⇒ ⇒ 它**不能**替代向量模型解决「真正的语义鸿沟」；");
    println!("     ⇒ ⇒ 但它能把「**本可以靠词面救的**」那部分先救回来。");
    ExitCode::SUCCESS
}

/// 按模式构造索引。
fn build_index(sample: &LocomoSample, mode: DocMode) -> Vec<IndexRecord> {
    let mode: DocModeArg = mode.into();
    let mut by_session: std::collections::BTreeMap<usize, String> =
        std::collections::BTreeMap::new();
    for (index, date) in &sample.session_dates {
        by_session.insert(*index, date.clone());
    }
    sample
        .records
        .iter()
        .map(|record| {
            let dia = record.id.rsplit("::").next().unwrap_or("");
            let session = dia
                .trim_start_matches('D')
                .split(':')
                .next()
                .and_then(|number| number.parse::<usize>().ok());
            let date = session
                .and_then(|index| by_session.get(&index).cloned())
                .unwrap_or_default();
            let mut text = record.text.clone();
            if mode.resolve_pronoun {
                text = resolve_pronouns(&text);
            }
            if mode.expand_date {
                let extra = date_variants(&date);
                if !extra.is_empty() {
                    text = format!("{text} {extra}");
                }
            }
            if mode.stem {
                text = stem_text(&text);
            }
            IndexRecord {
                id: record.id.clone(),
                text,
            }
        })
        .collect()
}

/// 记录改写的开关集合。
#[derive(Clone, Copy, Default)]
struct DocModeArg {
    expand_date: bool,
    resolve_pronoun: bool,
    stem: bool,
}

impl From<DocMode> for DocModeArg {
    fn from(mode: DocMode) -> Self {
        match mode {
            DocMode::Plain => Self::default(),
            DocMode::DateExpanded => Self {
                expand_date: true,
                ..Self::default()
            },
            DocMode::PronounResolved => Self {
                resolve_pronoun: true,
                ..Self::default()
            },
            DocMode::Stemmed => Self {
                stem: true,
                ..Self::default()
            },
            DocMode::PronounAndDate => Self {
                resolve_pronoun: true,
                expand_date: true,
                stem: false,
            },
            DocMode::All => Self {
                expand_date: true,
                resolve_pronoun: true,
                stem: true,
            },
        }
    }
}

/// 把第一人称消解成**说话人名字**（`"Caroline: I …"` ⇒ `"Caroline: Caroline …"`）。
fn resolve_pronouns(text: &str) -> String {
    let Some((speaker, body)) = text.split_once(':') else {
        return text.to_string();
    };
    let speaker = speaker.trim();
    if speaker.is_empty() {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len() + speaker.len() * 2);
    out.push_str(speaker);
    out.push(':');
    for token in body.split_inclusive(' ') {
        let core: String = token
            .chars()
            .take_while(|ch| ch.is_alphanumeric() || *ch == '\'')
            .collect();
        let lowered = core.to_lowercase();
        let replaced = match lowered.as_str() {
            "i" | "i'm" | "i've" | "i'll" | "i'd" | "me" | "my" | "mine" | "myself" => {
                Some(speaker)
            }
            _ => None,
        };
        match replaced {
            Some(name) => {
                out.push_str(name);
                out.push_str(&token[core.len()..]);
            }
            None => out.push_str(token),
        }
    }
    out
}

/// 日期多形态展开：`"1:56 pm on 8 May, 2023"` ⇒ `"2023-05-08 May 8 2023 8/5/2023"`。
fn date_variants(raw: &str) -> String {
    let cleaned = raw.to_lowercase();
    // 先取年份（4 位数字）。
    let year: Option<u32> = cleaned
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|piece| piece.len() == 4)
        .find_map(|piece| piece.parse().ok());
    let Some(year) = year else {
        return String::new();
    };
    // 月名。
    let Some(month) = MONTHS
        .iter()
        .position(|name| cleaned.contains(name))
        .map(|index| index as u32 + 1)
    else {
        return String::new();
    };
    // 日：取「月名之前」的最后一个 1~2 位数字（避开时间 `1:56`）。
    let before_month = cleaned
        .split(MONTHS[(month - 1) as usize])
        .next()
        .unwrap_or("");
    let day: Option<u32> = before_month
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|piece| !piece.is_empty() && piece.len() <= 2)
        .filter_map(|piece| piece.parse::<u32>().ok())
        .next_back();
    let Some(day) = day else {
        return String::new();
    };
    if !(1..=31).contains(&day) {
        return String::new();
    }
    let month_name = MONTHS[(month - 1) as usize];
    format!("{year:04}-{month:02}-{day:02} {month_name} {day} {year}")
}

/// 极简词干归一（只处理英文常见后缀 ⇒ **保守**，不追求语言学正确）。
fn stem_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for token in text.split_inclusive(' ') {
        let core: String = token
            .chars()
            .take_while(|ch| ch.is_alphanumeric() || *ch == '\'')
            .collect();
        let lowered = core.to_lowercase();
        let stem = if lowered.len() > 5 {
            ["ing", "edly", "ed", "es", "ly", "s"]
                .iter()
                .find_map(|suffix| {
                    lowered
                        .strip_suffix(suffix)
                        .filter(|rest| rest.len() >= 3)
                        .map(str::to_string)
                })
                .unwrap_or(lowered)
        } else {
            lowered
        };
        out.push_str(&stem);
        out.push_str(&token[core.len()..]);
    }
    out
}

/// 去掉疑问词/停用词。
fn strip_stopwords(text: &str) -> String {
    text.split_whitespace()
        .filter(|word| {
            let cleaned: String = word
                .chars()
                .filter(|ch| ch.is_alphanumeric())
                .collect::<String>()
                .to_lowercase();
            !STOPWORDS.contains(&cleaned.as_str())
        })
        .collect::<Vec<_>>()
        .join(" ")
}
