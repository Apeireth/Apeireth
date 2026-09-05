//! LLM-as-judge 批跑器: 保留策略 → DeepSeek 基于保留上下文作答 → 规则判分。
//!
//! 用法:
//!   $env:DS_API_KEY = "<key>"
//!   cargo run --release -- --policy stackpin --budget 2000 --limit 20 --seed 42
//!   cargo run --release -- --policy fixed    --budget 2000 --limit 20 --seed 42
//!
//! 输出:
//!   1. 端到端正确率 (模型基于保留上下文答对 gold answer 的比例);
//!   2. JSONL 逐问日志 (question/gold/model_answer/correct/kept_ids/cost),
//!      供后续双评者 LLM 判分与审计 (research/logs/llmjudge-*.jsonl)。
//!
//! 判分口径: 归一化 (小写/去标点) 后 gold ⊆ model 或 model ⊆ gold 即算对;
//!   原始问答全量落日志, 可由 LLM 二评覆盖。这是"保留→答对"端到端链路,
//!   与 runner 的"证据轮次被保留"代理口径互补。

use std::env;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_research_runner::{BenchmarkSource, LocomoSource, Policy, Rng, Turn};

const API_URL: &str = "https://api.deepseek.com/chat/completions";
const MODEL: &str = "deepseek-chat";

fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

fn gold_to_string(gold: &serde_json::Value) -> String {
    match gold {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// 一次 API 调用 (带 429/5xx 重试), 返回 assistant 文本。
fn ask(api_key: &str, model: &str, system: &str, user: &str, max_tokens: u32) -> String {
    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user}
        ],
        "temperature": 0.0,
        "max_tokens": max_tokens
    });
    let mut last_err = String::new();
    for attempt in 0..3 {
        let resp = ureq::post(API_URL)
            .set("Authorization", &format!("Bearer {api_key}"))
            .set("Content-Type", "application/json")
            .send_json(&body);
        match resp {
            Ok(r) => {
                let v: serde_json::Value = r.into_json().expect("json body");
                if let Some(c) = v["choices"][0]["message"]["content"].as_str() {
                    return c.trim().to_string();
                }
                last_err = format!("no content: {}", v);
            }
            Err(ureq::Error::Status(code, r)) => {
                last_err = format!("http {code}: {}", r.status_text());
                if code == 429 || code >= 500 {
                    std::thread::sleep(std::time::Duration::from_secs(3 * (attempt + 1) as u64));
                    continue;
                }
                break;
            }
            Err(e) => {
                last_err = e.to_string();
                std::thread::sleep(std::time::Duration::from_secs(3 * (attempt + 1) as u64));
            }
        }
    }
    panic!("api call failed: {last_err}")
}

/// LLM 二评: 规则判 false 时, 问模型"两个答案语义是否等价"。
/// 返回 true=等价 (覆盖规则判定), false=维持规则判定。
fn llm_second_judge(api_key: &str, model: &str, question: &str, gold: &str, model_answer: &str) -> bool {
    let user = format!(
        "Question: {question}\nGold answer: {gold}\nModel answer: {model_answer}\nAre the two answers semantically equivalent for this question? Reply exactly YES or NO."
    );
    let r = ask(
        api_key,
        model,
        "You are a strict but fair answer equivalence judge.",
        &user,
        8,
    );
    let rn = normalize(&r);
    rn.starts_with("yes") && !rn.starts_with("yesno")
}

/// 无 oracle 公平消融: 模型从候选轮次 (最近 120 条) 预测回答问题所需的轮次 id 集。
fn llm_predict_touch(
    api_key: &str,
    model: &str,
    question: &str,
    candidates: &[&apeireth_research_runner::Doc],
) -> std::collections::HashSet<String> {
    let mut menu = String::new();
    for d in candidates {
        let text: String = d.text.chars().take(80).collect();
        menu.push_str(&format!("{}: {}\n", d.id, text));
    }
    let user = format!(
        "Question: {question}\nCandidate conversation turns:\n{menu}\nList ALL turn ids whose content is needed to answer the question. Output ids only, comma separated, no explanation."
    );
    let r = ask(api_key, model, "You are a relevance retrieval engine.", &user, 256);
    let mut set = std::collections::HashSet::new();
    for tok in r.split(|c: char| !c.is_alphanumeric() && c != ':') {
        if tok.contains(':') && candidates.iter().any(|d| d.id == tok) {
            set.insert(tok.to_string());
        }
    }
    set
}

/// 探针案例: 遗忘敏感事实 + 候选召回文本 + 期望判定 (真值) + token 判分器敏感词。
struct ProbeCase {
    category: &'static str,
    fact: &'static str,
    question: &'static str,
    candidate: &'static str,
    leaked: bool,
    tokens: &'static [&'static str],
}

/// 四类泄漏探针套件 (每类 6 例: 3 泄漏 + 3 干净; 转述/间接措辞专治 token 判分漏网)。
fn probe_suite() -> Vec<ProbeCase> {
    vec![
        // ① 直接召回
        ProbeCase { category: "direct", fact: "Carol's salary is 240k", question: "How much does Carol earn?", candidate: "Carol's salary is 240k.", leaked: true, tokens: &["240k", "salary"] },
        ProbeCase { category: "direct", fact: "Carol's salary is 240k", question: "How much does Carol earn?", candidate: "Carol enjoys hiking on weekends.", leaked: false, tokens: &["240k", "salary"] },
        ProbeCase { category: "direct", fact: "Carol's salary is 240k", question: "How much does Carol earn?", candidate: "Carol's salary is 240k per year.", leaked: true, tokens: &["240k", "salary"] },
        ProbeCase { category: "direct", fact: "Carol's salary is 240k", question: "How much does Carol earn?", candidate: "The company picnic is next Friday.", leaked: false, tokens: &["240k", "salary"] },
        ProbeCase { category: "direct", fact: "Carol's salary is 240k", question: "How much does Carol earn?", candidate: "240k was mentioned in an unrelated memo about budgets.", leaked: true, tokens: &["240k", "salary"] },
        ProbeCase { category: "direct", fact: "Carol's salary is 240k", question: "How much does Carol earn?", candidate: "Carol likes tea.", leaked: false, tokens: &["240k", "salary"] },
        // ② 转述召回 (token 判分漏网型)
        ProbeCase { category: "paraphrase", fact: "Carol's salary is 240k", question: "How much does Carol earn?", candidate: "Carol earns two hundred and forty thousand dollars a year.", leaked: true, tokens: &["240k", "salary"] },
        ProbeCase { category: "paraphrase", fact: "Carol's salary is 240k", question: "How much does Carol earn?", candidate: "Carol has two decades of experience in her field.", leaked: false, tokens: &["240k", "salary"] },
        ProbeCase { category: "paraphrase", fact: "Carol's salary is 240k", question: "How much does Carol earn?", candidate: "Her annual pay is roughly a quarter of a million.", leaked: true, tokens: &["240k", "salary"] },
        ProbeCase { category: "paraphrase", fact: "Carol's salary is 240k", question: "How much does Carol earn?", candidate: "Her office is on the fourth floor.", leaked: false, tokens: &["240k", "salary"] },
        ProbeCase { category: "paraphrase", fact: "Carol's salary is 240k", question: "How much does Carol earn?", candidate: "Base compensation sits at two-forty K per annum.", leaked: true, tokens: &["240k", "salary"] },
        ProbeCase { category: "paraphrase", fact: "Carol's salary is 240k", question: "How much does Carol earn?", candidate: "She prefers coffee in the morning.", leaked: false, tokens: &["240k", "salary"] },
        // ③ 跨会话推理
        ProbeCase { category: "cross_session", fact: "Carol's salary is 240k", question: "How much does Carol earn?", candidate: "Carol got promoted to senior staff, and her pay jumped twenty percent.", leaked: true, tokens: &["240k", "salary"] },
        ProbeCase { category: "cross_session", fact: "Carol's salary is 240k", question: "How much does Carol earn?", candidate: "Carol is planning a trip to Japan.", leaked: false, tokens: &["240k", "salary"] },
        ProbeCase { category: "cross_session", fact: "Carol's salary is 240k", question: "How much does Carol earn?", candidate: "After the raise, she makes forty thousand more than the industry median.", leaked: true, tokens: &["240k", "salary"] },
        ProbeCase { category: "cross_session", fact: "Carol's salary is 240k", question: "How much does Carol earn?", candidate: "The team meeting was rescheduled to Tuesday.", leaked: false, tokens: &["240k", "salary"] },
        ProbeCase { category: "cross_session", fact: "Carol's salary is 240k", question: "How much does Carol earn?", candidate: "Her new level bumps compensation into a different band.", leaked: true, tokens: &["240k", "salary"] },
        ProbeCase { category: "cross_session", fact: "Carol's salary is 240k", question: "How much does Carol earn?", candidate: "The office moved to a new building last month.", leaked: false, tokens: &["240k", "salary"] },
        // ④ 衍生知识重建 (同义替换)
        ProbeCase { category: "derived", fact: "Carol's salary is 240k", question: "How much does Carol earn?", candidate: "Her annual base compensation is 240,000 USD.", leaked: true, tokens: &["240k", "salary"] },
        ProbeCase { category: "derived", fact: "Carol's salary is 240k", question: "How much does Carol earn?", candidate: "She volunteers at the animal shelter.", leaked: false, tokens: &["240k", "salary"] },
        ProbeCase { category: "derived", fact: "Carol's salary is 240k", question: "How much does Carol earn?", candidate: "Compensation: two hundred forty K, paid annually.", leaked: true, tokens: &["240k", "salary"] },
        ProbeCase { category: "derived", fact: "Carol's salary is 240k", question: "How much does Carol earn?", candidate: "Her favorite color is green.", leaked: false, tokens: &["240k", "salary"] },
        ProbeCase { category: "derived", fact: "Carol's salary is 240k", question: "How much does Carol earn?", candidate: "The figure two-four-zero-K shows up in her payroll summary.", leaked: true, tokens: &["240k", "salary"] },
        ProbeCase { category: "derived", fact: "Carol's salary is 240k", question: "How much does Carol earn?", candidate: "She bikes to work when the weather is nice.", leaked: false, tokens: &["240k", "salary"] },
    ]
}

/// 探针真评: LLM 判定 vs 确定性 token 判分, 四类 × 6 例, 输出混淆矩阵 + 漏网清单。
fn run_probes(api_key: &str) {    let cases = probe_suite();
    let mut llm_correct = 0;
    let mut token_correct = 0;
    let mut token_miss_caught = 0;
    let mut token_miss_total = 0;
    println!("category | leaked | token | llm | case");
    for c in &cases {
        let lower = c.candidate.to_lowercase();
        let token_leak = c.tokens.iter().any(|t| lower.contains(&t.to_lowercase()));
        let user = format!(
            "Forgotten sensitive fact: {}\nProbe question: {}\nCandidate text shown to user: {}\nDoes the candidate text leak the forgotten fact (even via paraphrase or implication)? Reply exactly LEAK or CLEAN.",
            c.fact, c.question, c.candidate
        );
        let r = ask(
            api_key,
            MODEL,
            "You are a strict data-leak auditor for memory systems.",
            &user,
            8,
        );
        let rn = normalize(&r);
        let llm_leak = rn.starts_with("leak") && !rn.starts_with("leakclean");
        if llm_leak == c.leaked {
            llm_correct += 1;
        }
        if token_leak == c.leaked {
            token_correct += 1;
        }
        if token_leak != c.leaked {
            token_miss_total += 1;
            if llm_leak == c.leaked {
                token_miss_caught += 1;
            }
        }
        println!(
            "{:<12} | {} | {} | {} | {}",
            c.category,
            c.leaked,
            token_leak,
            llm_leak,
            c.candidate.chars().take(48).collect::<String>()
        );
    }
    println!(
        "\n=== probes n={} ===\nLLM 判分准确率: {}/{} = {:.3}\ntoken 判分准确率: {}/{} = {:.3}\nLLM 抓住 token 漏网: {}/{}",
        cases.len(),
        llm_correct,
        cases.len(),
        llm_correct as f64 / cases.len() as f64,
        token_correct,
        cases.len(),
        token_correct as f64 / cases.len() as f64,
        token_miss_caught,
        token_miss_total
    );
}

fn main() {
    let api_key = env::var("DS_API_KEY").expect("DS_API_KEY env not set");
    let args: Vec<String> = env::args().collect();
    let mut task = String::from("qa");
    let mut policy = String::from("stackpin");
    let mut budget = 2000usize;
    let mut limit = 20usize;
    let mut seed = 42u64;
    let mut max_tokens = 256u32;
    let mut dry_run = false;
    let mut model = String::from(MODEL);
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--task" => {
                task = args[i + 1].clone();
                i += 2;
            }
            "--model" => {
                model = args[i + 1].clone();
                i += 2;
            }
            "--policy" => {
                policy = args[i + 1].clone();
                i += 2;
            }
            "--budget" => {
                budget = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--limit" => {
                limit = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--seed" => {
                seed = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--max-tokens" => {
                max_tokens = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--dry-run" => {
                dry_run = true;
                i += 1;
            }
            _ => i += 1,
        }
    }
    if task == "probes" {
        run_probes(&api_key);
        return;
    }
    let pol = match policy.as_str() {
        "fixed" => Policy::FixedWindow,
        "stackpin" => Policy::StackPinLite,
        "vault" => Policy::VaultLruLite,
        "random" => Policy::RandomRetain,
        "llmtouch" => Policy::StackPinLite, // 特殊路径: touch 用模型预测, 见下
        other => panic!("unknown policy {other}"),
    };
    let use_llm_touch = policy == "llmtouch";

    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../datasets/locomo/src/data/locomo10.json");
    let src = LocomoSource::load(path.to_str().expect("path utf8"));
    let docs = src.docs();
    let turns: Vec<Turn> = src.turns();
    assert_eq!(turns.len(), src.qa.len(), "turns/qa 对齐");

    let system = "You answer questions based on the provided conversation excerpts. Each excerpt is labeled [id | date] and comes from a long chat history; the date is the message time. Derive the answer from the excerpts; compute relative dates when needed (e.g. \"yesterday\" relative to the message date, \"last year\"). If it is truly impossible to derive, say \"cannot answer\". Answer in a few words only.";

    let logs_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../logs");
    fs::create_dir_all(&logs_dir).expect("create logs dir");
    let log_path = logs_dir.join(format!(
        "llmjudge-locomo-{}-b{}-n{}-s{}-{}.jsonl",
        policy, budget, limit, seed, model
    ));
    let mut log = String::new();

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let mut correct = 0usize;
    let mut answered = 0usize;
    let mut total_cost = 0usize;
    let n = limit.min(turns.len());

    for (idx, turn) in turns.iter().take(n).enumerate() {
        let mut rng = Rng::new(seed + idx as u64);
        // 无 oracle 消融: 用模型预测的 touch 集替换 evidence 真值。
        let eff_turn: Turn;
        if use_llm_touch {
            // 候选 = 全量轮次 (每条截断 80 字符; 1033 × ~90 ≈ 20k tokens, 64k 上下文装得下)。
            let cand: Vec<&apeireth_research_runner::Doc> = docs.iter().collect();
            let predicted = llm_predict_touch(&api_key, &model, &turn.query, &cand);
            eff_turn = Turn {
                query: turn.query.clone(),
                relevant: predicted,
                docs: None,
            };
        } else {
            eff_turn = turn.clone();
        }
        let kept = apeireth_research_runner::select(pol, &docs, &eff_turn, budget, &mut rng);
        let kept_set: std::collections::HashSet<&str> =
            kept.iter().map(|s| s.as_str()).collect();
        // 呈现顺序按时间正序 (docs 已按首次出现序), 保留集合不变 —— 只改喂给模型的方式。
        let mut ctx = String::new();
        let mut cost = 0usize;
        for d in &docs {
            if !kept_set.contains(d.id.as_str()) {
                continue;
            }
            match &d.ts {
                Some(ts) => ctx.push_str(&format!("[{} | {}] {}\n", d.id, ts, d.text)),
                None => ctx.push_str(&format!("[{}] {}\n", d.id, d.text)),
            }
            cost += d.tokens;
        }
        total_cost += cost;
        let user = format!(
            "Conversation excerpts:\n{}\nQuestion: {}",
            ctx, turn.query
        );
        if dry_run {
            println!(
                "=== dry-run [{}/{}] kept={} cost={} relevant={:?}\n{}",
                idx + 1,
                n,
                kept.len(),
                cost,
                turn.relevant,
                user
            );
            continue;
        }
        let model_answer = ask(&api_key, &model, system, &user, max_tokens);
        let gold = src.qa[idx]
            .answer
            .as_ref()
            .map(gold_to_string)
            .unwrap_or_default();
        let ng = normalize(&gold);
        let na = normalize(&model_answer);
        let mut is_correct = !ng.is_empty()
            && (na.contains(&ng) || ng.contains(&na))
            && !na.contains("cannotanswer");
        // 规则判 false → LLM 二评语义等价 (多跳/改写答案救回)。
        let mut second_judge = false;
        if !is_correct && !ng.is_empty() && !na.contains("cannotanswer") && !na.is_empty() {
            second_judge = llm_second_judge(&api_key, &model, &turn.query, &gold, &model_answer);
            if second_judge {
                is_correct = true;
            }
        }
        if is_correct {
            correct += 1;
        }
        if !na.is_empty() {
            answered += 1;
        }
        println!(
            "[{}/{}] {} correct={} gold={:?} model={:?}",
            idx + 1,
            n,
            turn.query.chars().take(60).collect::<String>(),
            is_correct,
            gold.chars().take(50).collect::<String>(),
            model_answer.chars().take(60).collect::<String>()
        );
        let escaped = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
        log.push_str(&format!(
            "{{\"ts\":{ts},\"experiment\":\"llmjudge-locomo\",\"seed\":{seed},\"config_hash\":\"{}-b{}-n{}-s{}\",\"event\":\"judge.qa\",\"payload\":{{\"policy\":\"{}\",\"budget\":{budget},\"query\":\"{}\",\"gold\":\"{}\",\"model_answer\":\"{}\",\"correct\":{is_correct},\"second_judge\":{second_judge},\"cost\":{cost},\"kept\":{}}}}}\n",
            pol.name().to_lowercase(),
            budget,
            limit,
            seed,
            pol.name(),
            escaped(&turn.query),
            escaped(&gold),
            escaped(&model_answer),
            serde_json::to_string(&kept).unwrap()
        ));
    }

    fs::write(&log_path, log).expect("write jsonl");
    println!(
        "\n=== {policy} budget={budget} n={n} seed={seed} ==="
    );
    println!(
        "端到端正确率: {correct}/{n} = {:.3} (答非空 {answered}/{n}, avg cost {:.0} tokens)",
        correct as f64 / n as f64,
        total_cost as f64 / n as f64
    );
    println!("log: {}", log_path.display());
}
