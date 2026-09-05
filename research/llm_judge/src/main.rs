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
fn ask(api_key: &str, system: &str, user: &str, max_tokens: u32) -> String {
    let body = serde_json::json!({
        "model": MODEL,
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
fn llm_second_judge(api_key: &str, question: &str, gold: &str, model_answer: &str) -> bool {
    let user = format!(
        "Question: {question}\nGold answer: {gold}\nModel answer: {model_answer}\nAre the two answers semantically equivalent for this question? Reply exactly YES or NO."
    );
    let r = ask(
        api_key,
        "You are a strict but fair answer equivalence judge.",
        &user,
        8,
    );
    let rn = normalize(&r);
    rn.starts_with("yes") && !rn.starts_with("yesno")
}

fn main() {
    let api_key = env::var("DS_API_KEY").expect("DS_API_KEY env not set");
    let args: Vec<String> = env::args().collect();
    let mut policy = String::from("stackpin");
    let mut budget = 2000usize;
    let mut limit = 20usize;
    let mut seed = 42u64;
    let mut max_tokens = 256u32;
    let mut dry_run = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
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
    let pol = match policy.as_str() {
        "fixed" => Policy::FixedWindow,
        "stackpin" => Policy::StackPinLite,
        "vault" => Policy::VaultLruLite,
        "random" => Policy::RandomRetain,
        other => panic!("unknown policy {other}"),
    };

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
        "llmjudge-locomo-{}-b{}-n{}-s{}.jsonl",
        pol.name().to_lowercase(),
        budget,
        limit,
        seed
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
        let kept = apeireth_research_runner::select(pol, &docs, turn, budget, &mut rng);
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
        let model_answer = ask(&api_key, system, &user, max_tokens);
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
            second_judge = llm_second_judge(&api_key, &turn.query, &gold, &model_answer);
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
