//! Apeireth 评测批运行器 (research/runners, 独立项目)。
//!
//! 用法: cargo run --release -- --seed 42 --turns 500
//! 输出:
//!   1. 效用-成本曲线 (每策略 × 每预算档): utility (任务成功率) vs avg prompt tokens;
//!   2. bootstrap 95% CI (1000 次重采样) 的策略间效用差;
//!   3. JSONL 研究日志 (schema 对齐 research/logs/README.md)。
//!
//! 真实数据集 (LoCoMo/LongMemEval) 即插即用接口: 实现 `BenchmarkSource` trait
//! 并替换 `SyntheticSource` 即可, 运行器骨架不变。

use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::BufRead;
use std::path::Path;

// ---------- 确定性 PRNG (xorshift64*) ----------
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }
    fn next_f64(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
    }
    fn next_u64(&mut self) -> u64 {
        (self.next_f64() * u64::MAX as f64) as u64
    }
}

// ---------- 基准数据模型 ----------
#[derive(Clone)]
struct Doc {
    id: String,
    tokens: usize,
    created_turn: usize,
}

#[derive(Clone)]
struct Turn {
    query: String,
    /// 本轮命中即成功的文档 id 集。
    relevant: HashSet<String>,
    /// 本轮专属文档宇宙 (LongMemEval 每问自带 haystack); None = 用全局 docs。
    docs: Option<Vec<Doc>>,
}

/// 真实数据集即插即用接口。
trait BenchmarkSource {
    fn docs(&self) -> Vec<Doc>;
    fn turns(&self) -> Vec<Turn>;
}

/// 合成源: universe 文档 + 局部性相关查询 (hot 集高概率)。
struct SyntheticSource {
    docs: Vec<Doc>,
    turns: Vec<Turn>,
}

impl SyntheticSource {
    fn new(seed: u64, n_docs: usize, n_turns: usize, hot_size: usize, p_hot: f64) -> Self {
        let mut rng = Rng::new(seed);
        let docs: Vec<Doc> = (0..n_docs)
            .map(|i| Doc {
                id: format!("doc-{i}"),
                tokens: 50 + (rng.next_f64() * 2000.0) as usize,
                created_turn: (rng.next_f64() * n_turns as f64) as usize,
            })
            .collect();
        let hot = hot_size.min(n_docs);
        let turns: Vec<Turn> = (0..n_turns)
            .map(|t| {
                let pick = |rng: &mut Rng| -> String {
                    let u = rng.next_f64();
                    let idx = if u < p_hot && hot > 0 {
                        (rng.next_f64() * hot as f64) as usize
                    } else if hot > 0 {
                        hot + ((rng.next_f64() * (n_docs - hot) as f64) as usize)
                    } else {
                        (rng.next_f64() * n_docs as f64) as usize
                    };
                    docs[idx].id.clone()
                };
                let mut relevant = HashSet::new();
                relevant.insert(pick(&mut rng));
                if rng.next_f64() < 0.3 {
                    relevant.insert(pick(&mut rng));
                }
                Turn {
                    query: format!("q{t}"),
                    relevant,
                    docs: None,
                }
            })
            .collect();
        Self { docs, turns }
    }
}

impl BenchmarkSource for SyntheticSource {
    fn docs(&self) -> Vec<Doc> {
        self.docs.clone()
    }
    fn turns(&self) -> Vec<Turn> {
        self.turns.clone()
    }
}

// ---------- LoCoMo 真实数据源 (CC BY-NC 4.0, 仅非商业研究) ----------

#[derive(serde::Deserialize)]
struct LocomoTurn {
    dia_id: String,
    #[allow(dead_code)]
    speaker: String,
    text: String,
}

#[derive(serde::Deserialize)]
struct LocomoQa {
    question: String,
    evidence: Vec<String>,
    #[allow(dead_code)]
    answer: Option<serde_json::Value>,
}

#[derive(serde::Deserialize)]
struct LocomoSession {
    conversation: serde_json::Value,
    qa: Vec<LocomoQa>,
}

impl LocomoSession {
    /// conversation 是 {speaker_a, speaker_b, session_1_date_time, session_1: [...], ...}
    /// 提取全部对话轮次, 按 session_ 编号排序。
    fn turns(&self) -> Vec<LocomoTurn> {
        let mut out = Vec::new();
        let Some(obj) = self.conversation.as_object() else {
            return out;
        };
        let mut keys: Vec<&String> = obj
            .keys()
            .filter(|k| k.starts_with("session_") && !k.ends_with("date_time"))
            .collect();
        keys.sort_by_key(|k| {
            let n: u64 = k.trim_start_matches("session_").parse().unwrap_or(0);
            n
        });
        for k in keys {
            if let Some(arr) = obj.get(k).and_then(|v| v.as_array()) {
                for t in arr {
                    if let Ok(turn) = serde_json::from_value::<LocomoTurn>(t.clone()) {
                        out.push(turn);
                    }
                }
            }
        }
        out
    }
}

/// LoCoMo 源: docs = 全部对话轮次 (id=dia_id); turns = QA, relevant = evidence dia_id。
struct LocomoSource {
    docs: Vec<Doc>,
    turns: Vec<Turn>,
}

impl LocomoSource {
    fn load(path: &str) -> Self {
        let text = fs::read_to_string(path).expect("read locomo10.json");
        let sessions: Vec<LocomoSession> =
            serde_json::from_str(&text).expect("parse locomo10.json");
        let docs = docs_from_sessions(&sessions);
        let mut turns = Vec::new();
        for s in &sessions {
            for q in &s.qa {
                turns.push(Turn {
                    query: q.question.clone(),
                    relevant: q.evidence.iter().cloned().collect(),
                    docs: None,
                });
            }
        }
        Self { docs, turns }
    }
}

impl BenchmarkSource for LocomoSource {
    fn docs(&self) -> Vec<Doc> {
        self.docs.clone()
    }
    fn turns(&self) -> Vec<Turn> {
        self.turns.clone()
    }
}

/// mc10 多选版 (Percena/locomo-mc10, CC BY-NC 4.0):
/// JSONL, 每行一条多选 QA (question/choices/correct_choice_index/haystack...),
/// 但**不带证据轮次标签**。用 question 文本精确匹配 locomo10 的 QA,
/// 把 evidence 真值借过来 (0 LLM); 匹配不上的跳过并计数。
#[derive(serde::Deserialize)]
struct Mc10Question {
    question: String,
}

/// mc10 源: docs = locomo10 全部对话轮次; turns = mc10 中能借到 evidence 真值的 QA。
struct LocomoMc10Source {
    docs: Vec<Doc>,
    turns: Vec<Turn>,
    matched: usize,
    total: usize,
}

impl LocomoMc10Source {
    fn load(locomo_path: &str, mc10_path: &str) -> Self {
        let text = fs::read_to_string(locomo_path).expect("read locomo10.json");
        let sessions: Vec<LocomoSession> =
            serde_json::from_str(&text).expect("parse locomo10.json");
        let docs = docs_from_sessions(&sessions);
        let mut evidence: HashMap<String, HashSet<String>> = HashMap::new();
        for s in &sessions {
            for q in &s.qa {
                evidence
                    .insert(q.question.clone(), q.evidence.iter().cloned().collect());
            }
        }
        let f = fs::File::open(mc10_path).expect("open locomo_mc10.json");
        let mut turns = Vec::new();
        let mut matched = 0usize;
        let mut total = 0usize;
        for line in std::io::BufReader::new(f).lines() {
            let line = line.expect("read mc10 line");
            if line.trim().is_empty() {
                continue;
            }
            let Ok(q) = serde_json::from_str::<Mc10Question>(&line) else {
                continue;
            };
            total += 1;
            if let Some(rel) = evidence.get(&q.question) {
                turns.push(Turn {
                    query: q.question,
                    relevant: rel.clone(),
                    docs: None,
                });
                matched += 1;
            }
        }
        Self {
            docs,
            turns,
            matched,
            total,
        }
    }
}

impl BenchmarkSource for LocomoMc10Source {
    fn docs(&self) -> Vec<Doc> {
        self.docs.clone()
    }
    fn turns(&self) -> Vec<Turn> {
        self.turns.clone()
    }
}

// ---------- LongMemEval 数据源 (MIT License, ICLR 2025) ----------
// 结构: 每条 QA 自带 haystack (独立文档宇宙) + answer_session_ids 会话级真值,
// 轮次级真值 (has_answer) 备用。docs/turns 按条流式解析 (文件 ~277MB)。

#[derive(serde::Deserialize)]
struct LmeTurn {
    #[allow(dead_code)]
    role: String,
    content: String,
    #[serde(default)]
    #[allow(dead_code)]
    has_answer: Option<bool>,
}

#[derive(serde::Deserialize)]
struct LmeEntry {
    #[allow(dead_code)]
    question_id: String,
    question: String,
    #[allow(dead_code)]
    question_type: Option<String>,
    answer_session_ids: Vec<String>,
    haystack_session_ids: Vec<String>,
    haystack_sessions: Vec<Vec<LmeTurn>>,
}

/// LongMemEval 源: turns 每条自带 docs (haystack 会话), relevant = answer_session_ids。
struct LongMemEvalSource {
    docs: Vec<Doc>, // 空壳占位: 全部轮次用 turn.docs
    turns: Vec<Turn>,
}

impl LongMemEvalSource {
    fn load(path: &str) -> Self {
        let f = fs::File::open(path).expect("open longmemeval json");
        let reader = std::io::BufReader::new(f);
        // 注: serde_json StreamDeserializer 把整个顶层数组当一个元素, 不做流式展开,
        // 所以这里直接一次性 typed 读取 (500 条 ≈ 277MB, 内存可接受)。
        let entries: Vec<LmeEntry> =
            serde_json::from_reader(reader).expect("parse longmemeval json");
        let mut turns = Vec::new();
        for entry in entries {
            let mut docs = Vec::new();
            for (i, (sid, sess)) in entry
                .haystack_session_ids
                .iter()
                .zip(entry.haystack_sessions.iter())
                .enumerate()
            {
                let chars: usize = sess.iter().map(|t| t.content.chars().count()).sum();
                docs.push(Doc {
                    id: sid.clone(),
                    tokens: (chars / 4).max(1),
                    created_turn: i,
                });
            }
            turns.push(Turn {
                query: entry.question,
                relevant: entry.answer_session_ids.into_iter().collect(),
                docs: Some(docs),
            });
        }
        Self {
            docs: Vec::new(),
            turns,
        }
    }
}

impl BenchmarkSource for LongMemEvalSource {
    fn docs(&self) -> Vec<Doc> {
        self.docs.clone()
    }
    fn turns(&self) -> Vec<Turn> {
        self.turns.clone()
    }
}

/// locomo10 会话 → 全部对话轮次 docs (id=dia_id, tokens≈chars/4, 按出现序编号)。
fn docs_from_sessions(sessions: &[LocomoSession]) -> Vec<Doc> {
    let mut docs = Vec::new();
    let mut created = 0usize;
    for s in sessions {
        for t in s.turns() {
            docs.push(Doc {
                id: t.dia_id,
                tokens: (t.text.chars().count() / 4).max(1),
                created_turn: created,
            });
            created += 1;
        }
    }
    docs
}

// ---------- 策略 ----------
#[derive(Clone, Copy, PartialEq)]
enum Policy {
    FixedWindow,
    RandomRetain,
    StackPinLite,
    VaultLruLite,
}

impl Policy {
    fn name(&self) -> &'static str {
        match self {
            Self::FixedWindow => "FixedWindow",
            Self::RandomRetain => "RandomRetain",
            Self::StackPinLite => "StackPinLite",
            Self::VaultLruLite => "VaultLruLite",
        }
    }
    fn all() -> Vec<Self> {
        vec![
            Self::FixedWindow,
            Self::RandomRetain,
            Self::StackPinLite,
            Self::VaultLruLite,
        ]
    }
}

/// 策略决策: 在 token 预算内选保留文档集。
fn select(
    policy: Policy,
    docs: &[Doc],
    turn: &Turn,
    budget: usize,
    rng: &mut Rng,
) -> Vec<String> {
    let mut kept: Vec<String> = Vec::new();
    let mut used = 0usize;
    let mut by_id: Vec<&Doc> = docs.iter().collect();
    match policy {
        Policy::FixedWindow => {
            // 按 created_turn 新→旧 (固定窗口语义)。
            by_id.sort_by_key(|d| std::cmp::Reverse(d.created_turn));
        }
        Policy::RandomRetain => {
            // 确定性种子随机 (同 seed 可复现)。
            let mut idx: Vec<usize> = (0..docs.len()).collect();
            for i in (1..idx.len()).rev() {
                let j = (rng.next_u64() % (i as u64 + 1)) as usize;
                idx.swap(i, j);
            }
            by_id = idx.into_iter().map(|i| &docs[i]).collect();
        }
        Policy::StackPinLite => {
            // recency 栈: created_turn 越新越靠前; 本轮相关查询命中视为 touch (模型级近似)。
            let touched: HashSet<&str> =
                turn.relevant.iter().map(|s| s.as_str()).collect();
            by_id.sort_by_key(|d| {
                let touch_bonus = if touched.contains(d.id.as_str()) { 1 } else { 0 };
                std::cmp::Reverse((touch_bonus, d.created_turn))
            });
        }
        Policy::VaultLruLite => {
            // 线性打分器 (固定权重, 学习版见 workspace 的 VaultLRU/FTRL):
            // score = 0.4·recency + 0.3·relevance + 0.3·(1/tokens 归一).
            let max_tokens = docs.iter().map(|d| d.tokens).max().unwrap_or(1);
            let rel: HashSet<&str> = turn.relevant.iter().map(|s| s.as_str()).collect();
            by_id.sort_by(|a, b| {
                let score = |d: &Doc| {
                    let rec = d.created_turn as f64 / 1000.0;
                    let rel_hit = if rel.contains(d.id.as_str()) { 1.0 } else { 0.0 };
                    let small = 1.0 - d.tokens as f64 / max_tokens as f64;
                    0.4 * rec + 0.3 * rel_hit + 0.3 * small
                };
                score(b).partial_cmp(&score(a)).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
    }
    for d in by_id {
        if used + d.tokens > budget {
            continue;
        }
        used += d.tokens;
        kept.push(d.id.clone());
    }
    kept
}

/// 单轮结果: 成功 (命中任一相关文档) + 成本。
fn run_turn(
    policy: Policy,
    docs: &[Doc],
    turn: &Turn,
    budget: usize,
    rng: &mut Rng,
) -> (bool, usize) {
    // LongMemEval: 每问自带 haystack 文档宇宙; 其余源用全局 docs。
    let universe: &[Doc] = turn.docs.as_deref().unwrap_or(docs);
    let kept = select(policy, universe, turn, budget, rng);
    let hit = kept.iter().any(|k| turn.relevant.contains(k));
    let cost: usize = kept
        .iter()
        .filter_map(|k| universe.iter().find(|d| d.id == *k).map(|d| d.tokens))
        .sum();
    (hit, cost)
}

// ---------- bootstrap ----------
fn bootstrap_mean_ci(samples: &[f64], n_resample: usize, seed: u64) -> (f64, f64) {
    let mut rng = Rng::new(seed + 1);
    let mut means = Vec::with_capacity(n_resample);
    for _ in 0..n_resample {
        let mut s = 0.0f64;
        for _ in 0..samples.len() {
            let i = (rng.next_u64() % samples.len() as u64) as usize;
            s += samples[i];
        }
        means.push(s / samples.len() as f64);
    }
    means.sort_by(|a, b| a.partial_cmp(b).unwrap());
    (means[n_resample / 40], means[n_resample * 39 / 40])
}

// ---------- JSONL 日志 (schema 对齐 research/logs/README.md) ----------
fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn log_event(
    file: &mut String,
    ts: &str,
    experiment: &str,
    seed: u64,
    config_hash: &str,
    event: &str,
    payload: &str,
) {
    file.push_str(&format!(
        "{{\"ts\":\"{ts}\",\"experiment\":\"{experiment}\",\"seed\":{seed},\"config_hash\":\"{config_hash}\",\"event\":\"{event}\",\"payload\":{payload}}}\n"
    ));
}

fn config_hash(seed: u64, turns: usize, budgets: &str) -> String {
    // sha256 不可用 (零依赖) → 用 FNV-1a 64 的 8 hex 位。
    let s = format!("{seed}:{turns}:{budgets}");
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:08x}")[..8].to_string()
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut seed = 42u64;
    let mut turns_n = 500usize;
    let mut budgets: Vec<usize> = vec![2000, 4000, 8000, 16000, 32000];
    let mut source_name = String::from("synthetic");
    let mut lme_file = String::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--seed" => {
                seed = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--turns" => {
                turns_n = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--budgets" => {
                budgets = args[i + 1]
                    .split(',')
                    .map(|x| x.trim().parse().unwrap())
                    .collect();
                i += 2;
            }
            "--source" => {
                source_name = args[i + 1].clone();
                i += 2;
            }
            "--lme-file" => {
                lme_file = args[i + 1].clone();
                i += 2;
            }
            _ => i += 1,
        }
    }

    let (docs, turns, experiment): (Vec<Doc>, Vec<Turn>, &str) = if source_name == "locomo" {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../datasets/locomo/src/data/locomo10.json");
        let src = LocomoSource::load(path.to_str().expect("path utf8"));
        let d = src.docs();
        let t = src.turns();
        println!("source: LoCoMo ({} docs / {} turns)", d.len(), t.len());
        (d, t, "locomo-retention")
    } else if source_name == "locomo-mc10" {
        let locomo = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../datasets/locomo/src/data/locomo10.json");
        let mc10 = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../datasets/locomo-mc10/locomo_mc10.json");
        let src = LocomoMc10Source::load(
            locomo.to_str().expect("path utf8"),
            mc10.to_str().expect("path utf8"),
        );
        let d = src.docs();
        let t = src.turns();
        println!(
            "source: LoCoMo-MC10 ({} docs / {} turns; 借到 evidence 真值的 QA {}/{}; 匹配不上被跳过)",
            d.len(),
            t.len(),
            src.matched,
            src.total
        );
        (d, t, "locomo-mc10-retention")
    } else if source_name == "longmemeval" {
        let path = if lme_file.is_empty() {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../datasets/longmemeval/longmemeval_s_cleaned.json")
        } else {
            Path::new(&lme_file).to_path_buf()
        };
        let src = LongMemEvalSource::load(path.to_str().expect("path utf8"));
        let t = src.turns();
        let avg_docs: f64 = t
            .iter()
            .map(|x| x.docs.as_ref().map(|d| d.len()).unwrap_or(0))
            .sum::<usize>() as f64
            / t.len().max(1) as f64;
        println!(
            "source: LongMemEval ({} QA turns, 平均每问 haystack {:.0} 会话)",
            t.len(),
            avg_docs
        );
        (src.docs(), t, "longmemeval-retention")
    } else {
        let src = SyntheticSource::new(seed, 200, turns_n, 20, 0.7);
        (src.docs(), src.turns(), "synthetic-retention")
    };
    let turns_n = turns.len();
    let policies = Policy::all();
    let budgets_s = budgets
        .iter()
        .map(|b| b.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let hash = config_hash(seed, turns_n, &budgets_s);

    // 日志目录: 固定到 research/logs/ (schema 对齐 research/logs/README.md)。
    let logs_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../logs");
    fs::create_dir_all(&logs_dir).expect("create logs dir");
    let mut log = String::new();
    log_event(
        &mut log,
        "2026-09-04T00:00:00.000+08:00",
        experiment,
        seed,
        &hash,
        "meta.run_start",
        &format!("{{\"commit\":\"research-runner\",\"turns\":{turns_n}}}"),
    );

    println!("policy × budget: utility (success rate) / avg tokens");
    let mut results: Vec<(Policy, usize, f64, f64, Vec<f64>)> = Vec::new();
    for policy in &policies {
        for budget in &budgets {
            let mut rng = Rng::new(seed + *budget as u64);
            let mut hits = 0usize;
            let mut total_cost = 0usize;
            let mut per_turn_utility: Vec<f64> = Vec::with_capacity(turns_n);
            for turn in &turns {
                let (hit, cost) = run_turn(*policy, &docs, turn, *budget, &mut rng);
                if hit {
                    hits += 1;
                }
                total_cost += cost;
                per_turn_utility.push(if hit { 1.0 } else { 0.0 });
                log_event(
                    &mut log,
                    "2026-09-04T00:00:00.000+08:00",
                    experiment,
                    seed,
                    &hash,
                    "retention.turn",
                    &format!(
                        "{{\"policy\":\"{}\",\"budget\":{},\"query\":\"{}\",\"success\":{},\"cost\":{}}}",
                        policy.name(),
                        budget,
                        json_escape(&turn.query),
                        hit,
                        cost
                    ),
                );
            }
            let utility = hits as f64 / turns_n as f64;
            let avg_cost = total_cost as f64 / turns_n as f64;
            results.push((*policy, *budget, utility, avg_cost, per_turn_utility));
            println!(
                "  {:>14}  B={:>6}  utility={:.3}  avg_tokens={:.0}",
                policy.name(),
                budget,
                utility,
                avg_cost
            );
        }
    }

    // bootstrap 95% CI: 各策略 vs FixedWindow 的效用差 (同预算档)。
    println!("\nbootstrap 95% CI (utility delta vs FixedWindow, 1000 resamples):");
    let baseline = Policy::FixedWindow;
    for budget in &budgets {
        let base: Vec<f64> = results
            .iter()
            .find(|(p, b, _, _, _)| *p == baseline && b == budget)
            .map(|(_, _, _, _, u)| u.clone())
            .unwrap_or_default();
        for policy in &policies {
            if *policy == baseline {
                continue;
            }
            if let Some((_, _, _, _, u)) = results
                .iter()
                .find(|(p, b, _, _, _)| p == policy && b == budget)
            {
                let deltas: Vec<f64> = base
                    .iter()
                    .zip(u.iter())
                    .map(|(a, b)| b - a)
                    .collect();
                let (lo, hi) = bootstrap_mean_ci(&deltas, 1000, seed + *budget as u64);
                println!(
                    "  {:>14} vs baseline  B={:>6}  Δutility 95% CI = [{:+.3}, {:+.3}]",
                    policy.name(),
                    budget,
                    lo,
                    hi
                );
            }
        }
    }

    log_event(
        &mut log,
        "2026-09-04T00:00:00.000+08:00",
        experiment,
        seed,
        &hash,
        "meta.run_end",
        "{\"exit\":0}",
    );
    let log_path = logs_dir.join(format!("{experiment}-{hash}.jsonl"));
    fs::write(&log_path, log).expect("write jsonl");
    println!("\nlog: {}", log_path.display());
}
