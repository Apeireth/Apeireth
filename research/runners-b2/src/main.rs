//! Apeireth B2「因果世界模型（个人时间线 + Brier 校准反事实推演）」实验 runner
//! (`research/runners-b2`) —— Tier-0 确定性骨架（0 LLM）。
//!
//! 覆盖协议 `_research_mem/ra/papers/b2/experiment-protocol-v1.md`：
//!   - §2.1 数据源与 session-level 切分（train=前 7 会话 / eval=后 3 会话）
//!   - §2.2 时间线窗口（会话内邻接窗口 `j - i <= W`）
//!   - §3 阶段 0 preflight / 阶段 1 数据构建（timeline.jsonl）
//!   - §3 阶段 2 W3 统计挖边 + 随机边控制图（0 LLM）
//!
//! 0 LLM：本骨架不含任何 LLM 调用；Tier-1（PCF/QA/judge）只留 TODO 占位。
//!
//! 复用面（只读参考；私有者 1:1 复制并注释出处）：
//!   - `apeireth_research_runner::LocomoSource`（path dep，去重/QA 口径交叉校验）
//!   - `research/runners/src/lib.rs`：`Rng` / `config_hash` / `log_event`（私有，复制）
//!   - `crates/engine/organ/src/causal_world_model_edges.rs`：W3 `from_timeline` 权重语义
//!   - `crates/engine/memory/src/intent_brier.rs`：brier 纯函数（见 `brier.rs`）

mod brier;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use apeireth_research_runner::{BenchmarkSource, LocomoSource};

// ============================================================
// 常量
// ============================================================

/// Session-level 切分：前 7 会话 train，后 3 会话 eval（协议 §2.1）。
const N_TRAIN_SESSIONS: usize = 7;
/// 边产出曲线档位（协议 §2.2 / §3 阶段 2）。
const MIN_EVIDENCE_CURVE: [u32; 5] = [2, 3, 4, 5, 7];
/// 去重后唯一轮次（协议 §2.1 / MANIFEST.md）。
const EXPECTED_UNIQUE_TURNS: usize = 1033;
/// QA 总数（协议 §2.1 / MANIFEST.md）。
const EXPECTED_QA: usize = 1986;

// ============================================================
// 确定性 PRNG（xorshift64*）
// 1:1 复制自 `research/runners/src/lib.rs::Rng`
// （其 `next_f64`/`next_u64` 为私有，无法跨 crate 复用）。
// ============================================================
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

// ============================================================
// JSONL 日志（schema 对齐 research/logs/README.md）
// ============================================================

/// FNV-1a 8 hex 配置哈希。算法 1:1 复制自
/// `research/runners/src/lib.rs::config_hash`（输入串按 B2 配置重构）。
fn config_hash(seed: u64, mode: &str, min_evidence: u32, window: usize, experiment: &str) -> String {
    let s = format!("{seed}:{mode}:{min_evidence}:{window}:{experiment}");
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:08x}")[..8].to_string()
}

/// `log_event` 风格（schema 1:1：ts/experiment/seed/config_hash/event/payload），
/// 用 serde_json 序列化替代手写字符串拼接（更安全）。
fn log_event(
    lines: &mut String,
    ts: &str,
    experiment: &str,
    seed: u64,
    hash: &str,
    event: &str,
    payload: serde_json::Value,
) {
    let line = serde_json::json!({
        "ts": ts,
        "experiment": experiment,
        "seed": seed,
        "config_hash": hash,
        "event": event,
        "payload": payload,
    });
    lines.push_str(&line.to_string());
    lines.push('\n');
}

/// 真实 UTC 事件时间（ISO-8601，秒精度，零依赖 civil-date 算法）。
fn utc_iso8601() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let sod = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    if m <= 2 {
        y += 1;
    }
    let (h, min, s) = (sod / 3600, (sod % 3600) / 60, sod % 60);
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{min:02}:{s:02}Z")
}

// ============================================================
// LoCoMo 数据结构（复制自 runners 私有结构，注释出处）
// ============================================================

/// 1:1 复制 `research/runners/src/lib.rs::LocomoTurn`（私有）。
#[derive(serde::Deserialize)]
struct LocomoTurn {
    dia_id: String,
    speaker: String,
    text: String,
}

/// 1:1 复制 `research/runners/src/lib.rs::LocomoQa`（其 `pub`，此处为解析顶层数组
/// 计 QA 数，字段最小化；answer 可能为数字/字符串/null，用 Value）。
#[derive(serde::Deserialize)]
struct LocomoQa {
    #[allow(dead_code)]
    question: String,
    #[allow(dead_code)]
    evidence: Vec<String>,
    #[allow(dead_code)]
    answer: Option<serde_json::Value>,
}

/// 1:1 复制 `research/runners/src/lib.rs::LocomoSession`（私有）。
#[derive(serde::Deserialize)]
struct LocomoSession {
    conversation: serde_json::Value,
    qa: Vec<LocomoQa>,
}

/// 带会话日期锚点的原始轮次（复制 runners 私有 `LocomoSession::dated_turns`）。
struct RawTurn {
    dia_id: String,
    speaker: String,
    text: String,
    date: String,
}

impl LocomoSession {
    /// 1:1 复制 `research/runners/src/lib.rs::LocomoSession::dated_turns`（私有）：
    /// 按 `session_N` 编号升序，取 `{k}_date_time` 为日期锚点。
    fn dated_turns(&self) -> Vec<RawTurn> {
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
            let date = obj
                .get(&format!("{k}_date_time"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if let Some(arr) = obj.get(k).and_then(|v| v.as_array()) {
                for t in arr {
                    if let Ok(turn) = serde_json::from_value::<LocomoTurn>(t.clone()) {
                        out.push(RawTurn {
                            dia_id: turn.dia_id,
                            speaker: turn.speaker,
                            text: turn.text,
                            date: date.clone(),
                        });
                    }
                }
            }
        }
        out
    }
}

// ============================================================
// 时间线与边
// ============================================================

/// timeline.jsonl 一行（协议 §3 阶段 1）。
#[derive(serde::Serialize, Clone)]
struct TimelineEntry {
    session: usize,
    idx: usize,
    dia_id: String,
    speaker: String,
    text: String,
    ts: String,
    split: &'static str,
}

/// 挖边用原始轮次（不序列化输出）：仅 session + dia_id。
/// 节点 = dia_id 标签（协议 §2.3），同一标签在多个会话各出现一次，故挖边证据流
/// 不去重（见 `build_raw_turns`）。
#[derive(Clone)]
struct MineTurn {
    session: usize,
    dia_id: String,
}

/// 因果边（协议 §3 阶段 2）。
#[derive(serde::Serialize, Clone)]
struct Edge {
    from: String,
    to: String,
    predicate: &'static str,
    weight: f64,
    evidence_count: u32,
    source: &'static str,
}

/// 构建去重个人时间线：`(session 编号, 会话内顺序)` 升序，`dia_id` 首次出现去重
/// （1:1 复制 runners 私有 `docs_from_sessions` 的去重口径）。
fn build_timeline(sessions: &[LocomoSession]) -> Vec<TimelineEntry> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    for (s, sess) in sessions.iter().enumerate() {
        let mut idx = 0usize;
        for t in sess.dated_turns() {
            if !seen.insert(t.dia_id.clone()) {
                continue;
            }
            let split = if s + 1 <= N_TRAIN_SESSIONS {
                "train"
            } else {
                "eval"
            };
            out.push(TimelineEntry {
                session: s + 1,
                idx,
                dia_id: t.dia_id,
                speaker: t.speaker,
                text: t.text,
                ts: t.date,
                split,
            });
            idx += 1;
        }
    }
    out
}

/// 挖边证据流（原始轮次，不去重）：每个会话的 `dated_turns` 按序展开，保留
/// dia_id 标签跨会话重复。节点 = dia_id（协议 §2.3），故同一「会话内邻接」的
/// (from,to) 对会在多个 train 会话重复出现，共现证据据此跨会话累计。
fn build_raw_turns(sessions: &[LocomoSession], train_only: bool) -> Vec<MineTurn> {
    let mut out = Vec::new();
    for (s, sess) in sessions.iter().enumerate() {
        if train_only && s + 1 > N_TRAIN_SESSIONS {
            continue;
        }
        for t in sess.dated_turns() {
            out.push(MineTurn {
                session: s + 1,
                dia_id: t.dia_id,
            });
        }
    }
    out
}

/// W3 统计挖边（1:1 语义对齐 `causal_world_model_edges.rs::from_timeline`，落在
/// 轮次粒度）：扫 train 会话内原始轮次流的 `(i,j)`，`j-i <= window`，跨会话累计
/// 共现计数。`count >= min_evidence` → 边；weight = count / 该源节点窗口内总出边
/// 候选数（条件概率近似，clamp [0,1]）。
///
/// 输入为 `build_raw_turns` 的原始 train 轮次（不去重）。
fn mine_edges(turns: &[MineTurn], window: usize, min_evidence: u32) -> Vec<Edge> {
    let mut by_session: BTreeMap<usize, Vec<&MineTurn>> = BTreeMap::new();
    for t in turns {
        by_session.entry(t.session).or_default().push(t);
    }

    let mut counts: HashMap<(String, String), u32> = HashMap::new();
    let mut source_total: HashMap<String, u32> = HashMap::new();

    for sess in by_session.values() {
        let n = sess.len();
        for i in 0..n {
            let from = sess[i];
            for j in (i + 1)..n {
                if j - i > window {
                    break;
                }
                let to = sess[j];
                *counts
                    .entry((from.dia_id.clone(), to.dia_id.clone()))
                    .or_insert(0) += 1;
                *source_total.entry(from.dia_id.clone()).or_insert(0) += 1;
            }
        }
    }

    let mut edges = Vec::new();
    for ((from, to), count) in counts {
        if count >= min_evidence {
            let total = source_total.get(&from).copied().unwrap_or(1).max(1);
            let weight = (count as f64 / total as f64).min(1.0);
            edges.push(Edge {
                from,
                to,
                predicate: "co-occurs",
                weight,
                evidence_count: count,
                source: "Statistical",
            });
        }
    }

    // 确定性排序：evidence_count 降序 → from 升序 → to 升序。
    edges.sort_by(|a, b| {
        b.evidence_count
            .cmp(&a.evidence_count)
            .then_with(|| a.from.cmp(&b.from))
            .then_with(|| a.to.cmp(&b.to))
    });
    edges
}

/// 随机边控制图（H1 baseline，协议 §3 阶段 2）：同数量、同 weight 分布，
/// 把 effect（`to`）节点随机打乱重配（Rng seed 固定），source=Random。
fn randomize_effects(edges: &[Edge], rng: &mut Rng) -> Vec<Edge> {
    let mut tos: Vec<String> = edges.iter().map(|e| e.to.clone()).collect();
    // Fisher-Yates，确定性（seed 固定）。
    for i in (1..tos.len()).rev() {
        let j = (rng.next_u64() % (i as u64 + 1)) as usize;
        tos.swap(i, j);
    }
    edges
        .iter()
        .zip(tos)
        .map(|(e, to)| Edge {
            from: e.from.clone(),
            to,
            predicate: "co-occurs",
            weight: e.weight,
            evidence_count: e.evidence_count,
            source: "Random",
        })
        .collect()
}

// ============================================================
// CLI 与数据加载
// ============================================================

struct Args {
    mode: String,
    min_evidence: u32,
    window: usize,
    seed: u64,
    out_dir: PathBuf,
    dry_run: bool,
}

fn parse_args() -> Args {
    let args: Vec<String> = std::env::args().collect();
    let mut out = Args {
        mode: "preflight".to_string(),
        min_evidence: 3,
        window: 3,
        seed: 42,
        out_dir: Path::new(env!("CARGO_MANIFEST_DIR")).join("../logs"),
        dry_run: false,
    };
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--mode" => {
                out.mode = args.get(i + 1).cloned().unwrap_or_default();
                i += 2;
            }
            "--min-evidence" => {
                out.min_evidence = args.get(i + 1).and_then(|v| v.parse().ok()).unwrap_or(3);
                i += 2;
            }
            "--window" => {
                out.window = args.get(i + 1).and_then(|v| v.parse().ok()).unwrap_or(3);
                i += 2;
            }
            "--seed" => {
                out.seed = args.get(i + 1).and_then(|v| v.parse().ok()).unwrap_or(42);
                i += 2;
            }
            "--out-dir" => {
                out.out_dir = PathBuf::from(args.get(i + 1).cloned().unwrap_or_default());
                i += 2;
            }
            "--dry-run" => {
                out.dry_run = true;
                i += 1;
            }
            _ => i += 1,
        }
    }
    out
}

fn data_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../datasets/locomo/src/data/locomo10.json")
}

fn load_sessions(path: &Path) -> Vec<LocomoSession> {
    if !path.exists() {
        panic!("locomo10.json 不存在: {}", path.display());
    }
    let text = fs::read_to_string(path).expect("read locomo10.json");
    serde_json::from_str(&text).expect("parse locomo10.json")
}

fn write_lines(path: &Path, lines: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create out dir");
    }
    fs::write(path, lines).expect("write file");
}

// ============================================================
// 阶段 0/1：preflight
// ============================================================

fn run_preflight(args: &Args) {
    let experiment = "b2-preflight";
    let hash = config_hash(args.seed, &args.mode, args.min_evidence, args.window, experiment);

    if args.dry_run {
        println!("== DRY RUN (0 API 消耗) ==");
        println!("mode: {}", args.mode);
        println!("data: {}", data_path().display());
        println!("步骤:");
        println!("  1. 断言 locomo10.json 存在");
        println!("  2. 断言去重后唯一轮次 == {EXPECTED_UNIQUE_TURNS}");
        println!("  3. 断言 QA == {EXPECTED_QA}");
        println!("  4. session-level 切分 train=前 {N_TRAIN_SESSIONS} / eval=后 3，打印各会话轮次数");
        println!("  5. 输出 {} + {}", "timeline.jsonl", format!("{experiment}-{hash}.jsonl"));
        println!();
        print_llm_calls();
        println!("(dry-run 不加载数据、不写任何文件)");
        return;
    }

    let path = data_path();
    let sessions = load_sessions(&path);
    let timeline = build_timeline(&sessions);
    let qa_local: usize = sessions.iter().map(|s| s.qa.len()).sum();

    // 交叉校验：复用 runners LocomoSource 的去重/QA 口径。
    let src = LocomoSource::load(path.to_str().expect("path utf8"));
    let unique = src.docs().len();
    let qa = src.qa.len();

    assert_eq!(
        unique, EXPECTED_UNIQUE_TURNS,
        "去重后唯一轮次 != 1033 (实测 {unique})"
    );
    assert_eq!(qa, EXPECTED_QA, "QA != 1986 (实测 {qa})");
    assert_eq!(
        qa_local, EXPECTED_QA,
        "本地解析 QA != 1986 (实测 {qa_local})"
    );
    assert_eq!(
        timeline.len(),
        unique,
        "本地 timeline 去重与 runners LocomoSource 口径不一致"
    );

    // 各会话（去重后）轮次数。
    let mut per_session: BTreeMap<usize, usize> = BTreeMap::new();
    for t in &timeline {
        *per_session.entry(t.session).or_insert(0) += 1;
    }
    let train_total: usize = per_session
        .iter()
        .filter(|(s, _)| **s <= N_TRAIN_SESSIONS)
        .map(|(_, n)| n)
        .sum();
    let eval_total = timeline.len() - train_total;

    println!("locomo10.json: OK ({})", path.display());
    println!("docs (去重后唯一轮次) = {unique}");
    println!("turns (QA) = {}", src.turns().len());
    println!("qa = {qa}");
    println!(
        "session split: train = 前 {N_TRAIN_SESSIONS} 会话, eval = 后 {} 会话",
        per_session.len().saturating_sub(N_TRAIN_SESSIONS)
    );
    for (s, n) in &per_session {
        let split = if *s <= N_TRAIN_SESSIONS { "train" } else { "eval" };
        println!("  session {s}: {n} turns ({split})");
    }
    println!("timeline total = {} (train {train_total}, eval {eval_total})", timeline.len());

    // timeline.jsonl。
    let mut lines = String::new();
    for t in &timeline {
        lines.push_str(&serde_json::to_string(t).expect("serialize timeline"));
        lines.push('\n');
    }
    write_lines(&args.out_dir.join("timeline.jsonl"), &lines);
    println!("wrote: {}", args.out_dir.join("timeline.jsonl").display());

    // meta 日志。
    let mut log = String::new();
    let ts = utc_iso8601();
    log_event(
        &mut log,
        &ts,
        experiment,
        args.seed,
        &hash,
        "meta.run_start",
        serde_json::json!({
            "mode": args.mode,
            "window": args.window,
            "min_evidence": args.min_evidence,
            "unique_turns": unique,
            "qa": qa,
            "train_sessions": N_TRAIN_SESSIONS,
            "train_turns": train_total,
            "eval_turns": eval_total,
        }),
    );
    log_event(
        &mut log,
        &ts,
        experiment,
        args.seed,
        &hash,
        "preflight.summary",
        serde_json::json!({
            "unique_turns": unique,
            "qa": qa,
            "sessions": per_session,
            "train_turns": train_total,
            "eval_turns": eval_total,
        }),
    );
    log_event(
        &mut log,
        &ts,
        experiment,
        args.seed,
        &hash,
        "meta.run_end",
        serde_json::json!({ "exit": 0 }),
    );
    let log_path = args.out_dir.join(format!("{experiment}-{hash}.jsonl"));
    write_lines(&log_path, &log);
    println!("log: {}", log_path.display());
}

// ============================================================
// 阶段 2：mine
// ============================================================

fn run_mine(args: &Args) {
    let experiment = "b2-mine";
    let hash = config_hash(args.seed, &args.mode, args.min_evidence, args.window, experiment);

    if args.dry_run {
        println!("== DRY RUN (0 API 消耗) ==");
        println!("mode: {}", args.mode);
        println!("data: {}", data_path().display());
        println!("window: {}  主档 min_evidence: {}   seed: {}", args.window, args.min_evidence, args.seed);
        println!("步骤:");
        println!("  1. 加载 + 去重 + train/eval 切分");
        println!("  2. W3 统计挖边 (train 内同 session, j-i <= window)");
        println!("  3. 随机控制图 (同数量同 weight 分布, effect 随机打乱, Rng seed 固定)");
        println!("  4. 输出 min_evidence ∈ {:?} 的边产出曲线", MIN_EVIDENCE_CURVE);
        for me in MIN_EVIDENCE_CURVE {
            println!(
                "     -> {} / {}",
                format!("edges-Statistical-{me}.jsonl"),
                format!("edges-Random-{me}.jsonl")
            );
        }
        println!();
        print_llm_calls();
        println!("(dry-run 不加载数据、不写任何文件)");
        return;
    }

    let path = data_path();
    let sessions = load_sessions(&path);
    let timeline = build_timeline(&sessions);
    let deduped_train = timeline.iter().filter(|t| t.split == "train").count();
    let raw_train = build_raw_turns(&sessions, true);
    if raw_train.is_empty() {
        panic!("train 会话为空，无法挖边");
    }

    println!("locomo10.json: OK ({})", path.display());
    println!(
        "deduped train turns = {deduped_train} (sessions 1..={N_TRAIN_SESSIONS})"
    );
    println!(
        "raw train turns (挖边证据流, 不去重) = {}, window = {}",
        raw_train.len(),
        args.window
    );

    let mut log = String::new();
    let ts = utc_iso8601();
    log_event(
        &mut log,
        &ts,
        experiment,
        args.seed,
        &hash,
        "meta.run_start",
        serde_json::json!({
            "mode": args.mode,
            "window": args.window,
            "primary_min_evidence": args.min_evidence,
            "deduped_train_turns": deduped_train,
            "raw_train_turns": raw_train.len(),
        }),
    );

    println!("min_evidence curve:");
    let mut curve = serde_json::Map::new();
    for me in MIN_EVIDENCE_CURVE {
        let stat = mine_edges(&raw_train, args.window, me);
        let rand = randomize_effects(&stat, &mut Rng::new(args.seed));

        let stat_path = args.out_dir.join(format!("edges-Statistical-{me}.jsonl"));
        let rand_path = args.out_dir.join(format!("edges-Random-{me}.jsonl"));
        write_edges(&stat_path, &stat);
        write_edges(&rand_path, &rand);

        println!("  min_evidence={me}: Statistical={} edges, Random={} edges", stat.len(), rand.len());
        curve.insert(
            me.to_string(),
            serde_json::json!({ "statistical": stat.len(), "random": rand.len() }),
        );

        log_event(
            &mut log,
            &ts,
            experiment,
            args.seed,
            &hash,
            "mine.curve",
            serde_json::json!({
                "min_evidence": me,
                "statistical_edges": stat.len(),
                "random_edges": rand.len(),
            }),
        );
    }

    log_event(
        &mut log,
        &ts,
        experiment,
        args.seed,
        &hash,
        "mine.summary",
        serde_json::json!({
            "window": args.window,
            "curve": curve,
        }),
    );
    log_event(
        &mut log,
        &ts,
        experiment,
        args.seed,
        &hash,
        "meta.run_end",
        serde_json::json!({ "exit": 0 }),
    );
    let log_path = args.out_dir.join(format!("{experiment}-{hash}.jsonl"));
    write_lines(&log_path, &log);
    println!("log: {}", log_path.display());
}

fn write_edges(path: &Path, edges: &[Edge]) {
    let mut lines = String::new();
    for e in edges {
        lines.push_str(&serde_json::to_string(e).expect("serialize edge"));
        lines.push('\n');
    }
    write_lines(path, &lines);
    println!("wrote: {}", path.display());
}

/// 打印将发起的 LLM 调用清单（骨架阶段 0 条）。
fn print_llm_calls() {
    println!("将发起的 LLM 调用清单: 0 条");
    println!("  [TODO] Tier-1 LLM 阶段（事实抽取 / EvoCause 提议 / PCF P_LLM / 反事实 QA 生成与 judge）");
    println!("         尚未实现，留占位；实现时必须在 main() 首行 fail-fast 断言 DS_API_KEY");
    println!("         （协议 §3 阶段 0 / §7.3），不允许静默降级为无 LLM。");
}

// ============================================================
// 入口
// ============================================================

fn main() {
    let args = parse_args();
    match args.mode.as_str() {
        "preflight" => run_preflight(&args),
        "mine" => run_mine(&args),
        other => {
            eprintln!(
                "未知 mode: {other}\n用法: cargo run -- --mode preflight|mine [--min-evidence N] [--window N] [--seed S] [--out-dir DIR] [--dry-run]"
            );
            std::process::exit(2);
        }
    }
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 内存 fixture：2 session × 10 轮，两会话 dia_id 标签完全相同（d0..d9）。
    /// window=3 时每会话 24 对，跨会话共现 count=2（对齐真实 locomo 标签跨会话重复）。
    fn fixture_turns() -> Vec<MineTurn> {
        let mut out = Vec::new();
        for s in 1..=2usize {
            for i in 0..10usize {
                out.push(MineTurn {
                    session: s,
                    dia_id: format!("d{i}"),
                });
            }
        }
        out
    }

    fn find_edge<'a>(edges: &'a [Edge], from: &str, to: &str) -> Option<&'a Edge> {
        edges.iter().find(|e| e.from == from && e.to == to)
    }

    #[test]
    fn mine_edges_count_and_evidence() {
        let tl = fixture_turns();
        let edges = mine_edges(&tl, 3, 2);
        // window=3 每会话 24 对；两会话共现 2 次 ≥ 2 → 全部成边。
        assert_eq!(edges.len(), 24, "window=3, min_evidence=2 应挖出 24 条边");
        assert!(
            edges.iter().all(|e| e.evidence_count == 2),
            "每条边在两会话各共现 1 次 → evidence_count=2"
        );
        assert!(
            edges.iter().all(|e| e.source == "Statistical"),
            "统计路径 source=Statistical"
        );
    }

    #[test]
    fn mine_edges_weight_normalization() {
        let tl = fixture_turns();
        let edges = mine_edges(&tl, 3, 2);
        // d8 只有一个出边候选 (d9)，两会话 count=2 → weight=2/2=1.0。
        let e = find_edge(&edges, "d8", "d9").expect("d8→d9");
        assert!((e.weight - 1.0).abs() < 1e-9, "d8→d9 weight=1.0");
        // d0 有三个出边候选 (d1,d2,d3)，count=2 → weight=2/6=1/3。
        let e = find_edge(&edges, "d0", "d1").expect("d0→d1");
        assert!((e.weight - 1.0 / 3.0).abs() < 1e-9, "d0→d1 weight=1/3");
    }

    #[test]
    fn mine_edges_below_threshold_empty() {
        let tl = fixture_turns();
        // 最大共现 = 2 < 3 → 0 边。
        assert!(mine_edges(&tl, 3, 3).is_empty());
    }

    #[test]
    fn mine_edges_respects_window() {
        let tl = fixture_turns();
        // window=1：每会话 9 对 (i,i+1)，两会话共现 2 次 → 9 边。
        let edges = mine_edges(&tl, 1, 2);
        assert_eq!(edges.len(), 9, "window=1 应只有相邻 9 对");
        assert!(find_edge(&edges, "d0", "d2").is_none(), "d0→d2 距离 2 超出 window=1");
        assert!(find_edge(&edges, "d0", "d1").is_some());
    }

    #[test]
    fn random_control_same_count_and_weights() {
        let tl = fixture_turns();
        let stat = mine_edges(&tl, 3, 2);
        let rand = randomize_effects(&stat, &mut Rng::new(42));
        assert_eq!(rand.len(), stat.len(), "随机图与统计图同数量");
        assert!(
            rand.iter().all(|e| e.source == "Random"),
            "随机图 source=Random"
        );
        // 同 weight 分布：weight 多重集相等。
        let mut sw: Vec<u64> = stat.iter().map(|e| e.weight.to_bits()).collect();
        let mut rw: Vec<u64> = rand.iter().map(|e| e.weight.to_bits()).collect();
        sw.sort();
        rw.sort();
        assert_eq!(sw, rw, "随机图 weight 分布与统计图一致");
        // effect 多重集相等（只是打乱）。
        let mut st: Vec<&String> = stat.iter().map(|e| &e.to).collect();
        let mut rt: Vec<&String> = rand.iter().map(|e| &e.to).collect();
        st.sort();
        rt.sort();
        assert_eq!(st, rt, "随机图 effect 节点多重集不变");
    }

    #[test]
    fn random_control_deterministic() {
        let tl = fixture_turns();
        let stat = mine_edges(&tl, 3, 2);
        let a = randomize_effects(&stat, &mut Rng::new(42));
        let b = randomize_effects(&stat, &mut Rng::new(42));
        let a_tos: Vec<&String> = a.iter().map(|e| &e.to).collect();
        let b_tos: Vec<&String> = b.iter().map(|e| &e.to).collect();
        assert_eq!(a_tos, b_tos, "固定 seed 的随机图必须可复现");
    }
}
