//! B1 确定性骨架 CLI (协议 Phase 0/1, 0 LLM)。
//!
//! 用法见 `--help`。`--mode preflight` 跑数据自检; `--mode betti` 跑每会话构图 +
//! vendored Betti 环启发式分析; `--dry-run` 打印 LLM 阶段调用清单 (骨架为 0 条)。

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use apeireth_research_b1::embed;
use apeireth_research_b1::log;
use apeireth_research_b1::session_split;
use apeireth_research_b1::topo::betti_hole_detector::{BettiHoleDetector, BettiTopologicalReport, ManifoldConceptNode};
use apeireth_research_b1::{activation_energy, build_nodes, truncate_nodes, NodeInput};
use apeireth_research_runner::{BenchmarkSource, Doc, LocomoSource};

/// 日志时间戳 (确定性骨架, 与 runners 一致用固定锚点, 不引入时钟非确定性)。
const TS: &str = "2026-09-06T00:00:00.000+08:00";
const EXPECT_UNIQUE_TURNS: usize = 1033;
const EXPECT_QA: usize = 1986;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Preflight,
    Betti,
}

impl Mode {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "preflight" => Some(Self::Preflight),
            "betti" => Some(Self::Betti),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
struct Config {
    mode: Mode,
    seed: u64,
    max_nodes: usize,
    threshold_ratio: f32,
    embed_cache: PathBuf,
    dry_run: bool,
}

fn default_cache() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(".cache/embeddings.json")
}

fn locomo_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../datasets/locomo/src/data/locomo10.json")
}

fn usage() {
    println!("usage: apeireth-research-b1 --mode preflight|betti [options]");
    println!("  --mode preflight|betti   数据自检 / Phase 1 Betti 分析 (默认 preflight)");
    println!("  --seed <u64>             确定性 seed (默认 42)");
    println!("  --max-nodes <usize>      每会话节点上限 (默认 150, 协议 §2.2)");
    println!("  --threshold-ratio <f32>  min_persistence = ratio * max_dist (默认 0.05, 协议 §2.2)");
    println!("  --embed-cache <path>     嵌入缓存 (默认 research/b1/.cache/embeddings.json)");
    println!("  --dry-run                打印 LLM 阶段调用清单 (骨架 = 0 条), 不花 API 钱");
    println!("  --help                   本帮助");
}

fn parse_args() -> Result<Config, String> {
    let args: Vec<String> = std::env::args().collect();
    let mut cfg = Config {
        mode: Mode::Preflight,
        seed: 42,
        max_nodes: 150,
        threshold_ratio: 0.05,
        embed_cache: default_cache(),
        dry_run: false,
    };
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--mode" => {
                let v = args.get(i + 1).ok_or("--mode 需要一个值")?;
                cfg.mode = Mode::parse(v).ok_or_else(|| format!("未知 mode: {v}"))?;
                i += 2;
            }
            "--seed" => {
                let v = args.get(i + 1).ok_or("--seed 需要一个值")?;
                cfg.seed = v.parse().map_err(|e| format!("--seed 解析失败: {e}"))?;
                i += 2;
            }
            "--max-nodes" => {
                let v = args.get(i + 1).ok_or("--max-nodes 需要一个值")?;
                cfg.max_nodes = v.parse().map_err(|e| format!("--max-nodes 解析失败: {e}"))?;
                i += 2;
            }
            "--threshold-ratio" => {
                let v = args.get(i + 1).ok_or("--threshold-ratio 需要一个值")?;
                cfg.threshold_ratio = v
                    .parse()
                    .map_err(|e| format!("--threshold-ratio 解析失败: {e}"))?;
                i += 2;
            }
            "--embed-cache" => {
                let v = args.get(i + 1).ok_or("--embed-cache 需要一个值")?;
                cfg.embed_cache = PathBuf::from(v);
                i += 2;
            }
            "--dry-run" => {
                cfg.dry_run = true;
                i += 1;
            }
            "--help" | "-h" => {
                usage();
                std::process::exit(0);
            }
            other => return Err(format!("未知参数: {other}")),
        }
    }
    if !(cfg.threshold_ratio > 0.0 && cfg.threshold_ratio <= 1.0) {
        return Err("--threshold-ratio 须在 (0, 1] 区间".to_string());
    }
    if cfg.max_nodes < 3 {
        return Err("--max-nodes 至少为 3 (少于 3 节点恒无洞)".to_string());
    }
    Ok(cfg)
}

fn main() -> ExitCode {
    let cfg = match parse_args() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            usage();
            return ExitCode::from(2);
        }
    };

    let result = match cfg.mode {
        Mode::Preflight => run_preflight(&cfg),
        Mode::Betti => run_betti(&cfg),
    };

    if cfg.dry_run {
        print_llm_plan(&cfg);
    }

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

// ---------------- Phase 0: preflight (0 LLM) ----------------

fn run_preflight(cfg: &Config) -> Result<(), String> {
    let path = locomo_path();
    if !path.exists() {
        return Err(format!("locomo10.json 不存在: {}", path.display()));
    }

    let slices = session_split::split_sessions(&path);
    let src = LocomoSource::load(path.to_str().ok_or("路径非 UTF-8")?);
    let docs = src.docs();
    let turns = src.turns();

    let sum_kept: usize = slices.iter().map(|s| s.n_kept()).sum();
    if docs.len() != EXPECT_UNIQUE_TURNS {
        return Err(format!(
            "去重后唯一轮次 = {} != {EXPECT_UNIQUE_TURNS} (数据变更?)",
            docs.len()
        ));
    }
    if turns.len() != EXPECT_QA {
        return Err(format!("QA = {} != {EXPECT_QA} (数据变更?)", turns.len()));
    }
    if sum_kept != EXPECT_UNIQUE_TURNS {
        return Err(format!(
            "每会话唯一轮次之和 = {sum_kept} != {EXPECT_UNIQUE_TURNS} (会话切分自检失败)"
        ));
    }

    println!(
        "preflight 自检通过: 去重唯一轮次 = {EXPECT_UNIQUE_TURNS}, QA = {EXPECT_QA}, 每会话唯一轮次之和 = {EXPECT_UNIQUE_TURNS}"
    );
    println!("会话 | n_raw | n_kept | 预计节点数 (min(n_kept, max_nodes))");
    for s in &slices {
        let kept = s.n_kept().min(cfg.max_nodes);
        println!("{:>4} | {:>5} | {:>6} | {:>6}", s.index, s.n_raw, s.n_kept(), kept);
    }

    let extra = format!("{}:{}", cfg.max_nodes, cfg.threshold_ratio);
    let hash = log::config_hash(cfg.seed, "b1-preflight", &extra);
    let logs_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../logs");
    fs::create_dir_all(&logs_dir).map_err(|e| format!("创建 logs 目录失败: {e}"))?;

    let mut buf = String::new();
    log::log_event(
        &mut buf,
        TS,
        "b1-preflight",
        cfg.seed,
        &hash,
        "meta.run_start",
        &serde_json::json!({
            "unique_turns": docs.len(),
            "qa": turns.len(),
            "session_count": slices.len(),
            "max_nodes": cfg.max_nodes,
        })
        .to_string(),
    );
    for s in &slices {
        log::log_event(
            &mut buf,
            TS,
            "b1-preflight",
            cfg.seed,
            &hash,
            "preflight.session",
            &serde_json::json!({
                "session_index": s.index,
                "n_raw": s.n_raw,
                "n_kept": s.n_kept(),
                "predicted_nodes": s.n_kept().min(cfg.max_nodes),
            })
            .to_string(),
        );
    }
    log::log_event(
        &mut buf,
        TS,
        "b1-preflight",
        cfg.seed,
        &hash,
        "meta.run_end",
        "{\"exit\":0}",
    );

    let log_path = logs_dir.join(format!("b1-preflight-{hash}.jsonl"));
    fs::write(&log_path, buf).map_err(|e| format!("写日志失败: {e}"))?;
    println!("log: {}", log_path.display());
    Ok(())
}

// ---------------- Phase 1: betti (0 LLM) ----------------

#[derive(serde::Serialize)]
struct SessionSummary {
    session_index: usize,
    /// 会话内全部轮次条目数 (含 session_N 快照重复历史)。
    n_raw: usize,
    /// 截断前唯一轮次数 n_s (全局去重后归属本会话)。
    n_unique: usize,
    /// 截断后节点数 (min(n_unique, max_nodes))。
    n_kept: usize,
    truncated: bool,
    betti_0: usize,
    hole_count: usize,
    lifetimes: Vec<f32>,
}

fn run_betti(cfg: &Config) -> Result<(), String> {
    let path = locomo_path();
    if !path.exists() {
        return Err(format!("locomo10.json 不存在: {}", path.display()));
    }

    let slices = session_split::split_sessions(&path);
    let src = LocomoSource::load(path.to_str().ok_or("路径非 UTF-8")?);
    let docs = src.docs();
    let doc_map: HashMap<String, Doc> = docs.into_iter().map(|d| (d.id.clone(), d)).collect();

    if !cfg.embed_cache.exists() {
        return Err(format!(
            "嵌入缓存不存在: {}\n  (骨架不含 fastembed; 请先用嵌入模型生成缓存, 或用 --embed-cache 指定路径。协议 §7.3)",
            cfg.embed_cache.display()
        ));
    }
    let cache = embed::load_cache(&cfg.embed_cache)?;
    if cache.dim != 384 {
        eprintln!(
            "warning: 缓存 dim = {} (协议默认 384-d all-MiniLM-L6-v2, 见 §2.2/§7.3)",
            cache.dim
        );
    }
    let embed_model = if cache.embed_model.is_empty() {
        "unknown".to_string()
    } else {
        cache.embed_model.clone()
    };

    let mut summaries: Vec<SessionSummary> = Vec::new();
    let mut reports: Vec<BettiTopologicalReport> = Vec::new();
    let mut all_lifetimes: Vec<f32> = Vec::new();

    for s in &slices {
        let mut inputs: Vec<NodeInput> = Vec::new();
        for t in &s.turns {
            let doc = doc_map
                .get(&t.dia_id)
                .ok_or_else(|| format!("会话 {} 的 {} 不在全局 docs (切分不一致)", s.index, t.dia_id))?;
            let citations = *s.evidence_citations.get(&t.dia_id).unwrap_or(&0);
            inputs.push(NodeInput {
                dia_id: t.dia_id.clone(),
                text: doc.text.clone(),
                created_turn: doc.created_turn,
                activation_energy: activation_energy(citations),
            });
        }
        let n_unique = inputs.len();
        let n_raw = s.n_raw;
        let (kept, _) = truncate_nodes(inputs, cfg.max_nodes);
        let truncated = n_unique > kept.len();

        let missing: Vec<String> = kept
            .iter()
            .filter(|k| !cache.vectors.contains_key(&k.dia_id))
            .map(|k| k.dia_id.clone())
            .collect();
        if !missing.is_empty() {
            return Err(format!(
                "会话 {} 缺 {} 个嵌入 (首个缺失: {}); 缓存不完整",
                s.index,
                missing.len(),
                missing[0]
            ));
        }

        let nodes: Vec<ManifoldConceptNode> = build_nodes(&kept, &cache.vectors);
        let max_dist = max_pairwise_dist(&nodes);
        let detector = BettiHoleDetector::new(cfg.threshold_ratio * max_dist, 10);
        let report = detector.analyze(&nodes);

        let hole_count = report.betti_1_voids.len();
        all_lifetimes.extend(report.betti_1_voids.iter().map(|v| v.persistence_lifetime));
        summaries.push(SessionSummary {
            session_index: s.index,
            n_raw,
            n_unique,
            n_kept: kept.len(),
            truncated,
            betti_0: report.betti_0_islands,
            hole_count,
            lifetimes: report
                .betti_1_voids
                .iter()
                .map(|v| v.persistence_lifetime)
                .collect(),
        });
        reports.push(report);
    }

    let extra = format!("{}:{:.4}:{}", cfg.max_nodes, cfg.threshold_ratio, embed_model);
    let hash = log::config_hash(cfg.seed, "b1-betti", &extra);
    let logs_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../logs");
    let metrics_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../metrics");
    fs::create_dir_all(&logs_dir).map_err(|e| format!("创建 logs 目录失败: {e}"))?;
    fs::create_dir_all(&metrics_dir).map_err(|e| format!("创建 metrics 目录失败: {e}"))?;

    let mut buf = String::new();
    log::log_event(
        &mut buf,
        TS,
        "b1-betti",
        cfg.seed,
        &hash,
        "meta.run_start",
        &serde_json::json!({
            "session_count": slices.len(),
            "max_nodes": cfg.max_nodes,
            "threshold_ratio": cfg.threshold_ratio,
            "embed_model": embed_model,
            "embed_dim": cache.dim,
        })
        .to_string(),
    );
    for (s, r) in summaries.iter().zip(reports.iter()) {
        log::log_event(
            &mut buf,
            TS,
            "b1-betti",
            cfg.seed,
            &hash,
            "betti.session",
            &serde_json::json!({
                "session_index": s.session_index,
                "n_raw": s.n_raw,
                "n_unique": s.n_unique,
                "n_kept": s.n_kept,
                "truncated": s.truncated,
                "report": r,
            })
            .to_string(),
        );
    }
    log::log_event(
        &mut buf,
        TS,
        "b1-betti",
        cfg.seed,
        &hash,
        "meta.run_end",
        "{\"exit\":0}",
    );

    let log_path = logs_dir.join(format!("b1-betti-{hash}.jsonl"));
    fs::write(&log_path, buf).map_err(|e| format!("写日志失败: {e}"))?;

    let beta0_distribution: Vec<usize> = summaries.iter().map(|s| s.betti_0).collect();
    let hole_count_distribution: Vec<usize> = summaries.iter().map(|s| s.hole_count).collect();
    let (bin_edges, bin_counts) = lifetime_histogram(&all_lifetimes);
    let metrics = serde_json::json!({
        "experiment": "b1-betti",
        "config_hash": hash,
        "seed": cfg.seed,
        "max_nodes": cfg.max_nodes,
        "threshold_ratio": cfg.threshold_ratio,
        "embed_model": embed_model,
        "embed_dim": cache.dim,
        "session_count": summaries.len(),
        "total_holes": all_lifetimes.len(),
        "beta0_distribution": beta0_distribution,
        "hole_count_distribution": hole_count_distribution,
        "lifetime_histogram": { "bin_edges": bin_edges, "counts": bin_counts },
        "sessions": summaries,
    });
    let metrics_path = metrics_dir.join(format!("b1-betti-{hash}.json"));
    fs::write(
        &metrics_path,
        serde_json::to_string_pretty(&metrics).map_err(|e| format!("序列化 metrics 失败: {e}"))?,
    )
    .map_err(|e| format!("写 metrics 失败: {e}"))?;

    println!("betti 分析完成 (config_hash {hash})");
    println!("β0 分布 (每会话): {beta0_distribution:?}");
    println!("洞数分布 (每会话): {hole_count_distribution:?}");
    println!("lifetime 直方图: edges={bin_edges:?} counts={bin_counts:?}");
    println!("log: {}", log_path.display());
    println!("metrics: {}", metrics_path.display());
    Ok(())
}

fn max_pairwise_dist(nodes: &[ManifoldConceptNode]) -> f32 {
    let mut m = 0.0f32;
    for i in 0..nodes.len() {
        for j in (i + 1)..nodes.len() {
            let d = BettiHoleDetector::euclidean_distance(&nodes[i].embedding, &nodes[j].embedding);
            if d > m {
                m = d;
            }
        }
    }
    m
}

fn lifetime_histogram(lifetimes: &[f32]) -> (Vec<f32>, Vec<usize>) {
    if lifetimes.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for &l in lifetimes {
        if l < min {
            min = l;
        }
        if l > max {
            max = l;
        }
    }
    let nbins = 10usize;
    if (max - min).abs() < 1e-9 {
        return (vec![min, max], vec![lifetimes.len()]);
    }
    let width = (max - min) / nbins as f32;
    let mut edges = Vec::with_capacity(nbins + 1);
    for b in 0..=nbins {
        edges.push(min + b as f32 * width);
    }
    edges[nbins] = max;
    let mut counts = vec![0usize; nbins];
    for &l in lifetimes {
        let mut idx = ((l - min) / width).floor() as usize;
        if idx >= nbins {
            idx = nbins - 1;
        }
        counts[idx] += 1;
    }
    (edges, counts)
}

// ---------------- dry-run (0 LLM 清单) ----------------

fn print_llm_plan(_cfg: &Config) {
    // TODO (协议 §7.3 / Phase 2-4): 骨架为 0 LLM, 不实现任何 LLM 调用。
    println!("\n[dry-run] LLM 阶段调用清单 (骨架阶段 = 0 条):");
    println!("  0 条 —— 本骨架不含任何 LLM 调用。");
    println!("  TODO: H1 双探针 Probe A/B (§2.4 方法 A) —— 需 DS_API_KEY");
    println!("  TODO: H2 提问生成 + judge (§3 Phase 3) —— 需 DS_API_KEY");
    println!("  TODO: H3 概念抽取 + 共振 + judge (§3 Phase 4) —— 需 DS_API_KEY");
    println!("  TODO: 成本闸门 $10 (协议 §7.4); 嵌入模型独立依赖 (协议 §7.3)");
}
