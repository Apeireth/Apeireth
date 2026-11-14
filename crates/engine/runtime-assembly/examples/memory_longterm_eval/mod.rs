//! The standalone long-range evaluation and the scale/sustained-run contract.
//!
//! ⭐ 口径（必须原样保留）：
//! * 本卷题集是**回归集**（本仓自编、与实现同一批人），**不是独立盲测**。
//! * 题面与期望答案分开存放：oracle 不进入任何模型可见上下文。
//! * 不同账号的记录落在不同会话作用域内 ⇒ 这是**同账号隔离**，不是强保密。
//! * 规模结果与持续运行结果**分别报告**；虚拟时间推进**不是**持续运行。
//! * 默认路径**未调用真实模型**：模型侧是确定性离线夹具，产物里逐处标明。
//! * 真实模型路径是**显式开关**（`--live`）：凭据只经环境变量按轮解析，
//!   不进参数、不进源码、不进日志、不进产物、不进报告。

pub mod artifacts;
pub mod config;
pub mod corpus;
pub mod evidence;
pub mod grading;
pub mod harness;
pub mod live;
pub mod pressure;
pub mod reader;
pub mod schema;
pub mod seal;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use apeireth_protocol::canonical::NormalizedUsage;
use apeireth_runtime::canonical::{ExecutionTrace, TraceEvent};

use self::artifacts::{hash, hash_bytes, new_run, read, write_new, Result, Stopwatch};
use self::config::{Config, Paths};
use self::corpus::{CORPUS_ID, ORACLE_CANARY};
use self::evidence::{Audit, Inspect};
use self::grading::GradeInput;
use self::harness::Harness;
use self::pressure::PressureOptions;
use self::schema::{
    validate_input, validate_oracle, Distribution, Grade, Span, MANIFEST_PROTOCOL, REPORT_PROTOCOL,
};

/// The usage of one provider call, as recorded by the offline fixture.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub prompt_cache_hit_tokens: Option<u32>,
}

/// The cost of one provider call.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Cost {
    /// "wired" when the amount could be computed, "not_wired" otherwise.
    pub status: String,
    pub currency: Option<String>,
    pub price_date: Option<String>,
    /// Where the price table came from.
    pub price_source: Option<String>,
    pub amount: Option<f64>,
    pub note: String,
}

/// The timing of one case.
///
/// ⭐ `wall_clock_us` is measured around the turn. The trace timings come from
/// the **injected virtual clock**, so they are virtual values and must never
/// be presented as real waiting time.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Timing {
    /// Measured wall-clock duration of the turn.
    pub wall_clock_us: u64,
    /// Measured wall-clock duration of the provider call itself.
    pub provider_call_us: u64,
    /// Virtual-clock wait before the provider was invoked.
    pub virtual_queue_wait_ms: Option<i64>,
    /// Virtual-clock duration of the provider call.
    pub virtual_provider_ms: Option<i64>,
    /// Virtual-clock duration of the whole turn.
    pub virtual_total_ms: Option<i64>,
    /// How the trace timings were produced.
    pub clock_source: String,
}

/// One evaluated case.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CaseArtifact {
    pub case: String,
    pub dimension: String,
    pub session: String,
    pub question: String,
    pub answer_sha256: String,
    pub answer_text: String,
    pub grade: Grade,
    pub evidence: Value,
    pub premise_summary: Value,
    pub usage: Usage,
    pub cost: Cost,
    pub timing: Timing,
    /// The correlation id of the request.
    pub request_id: String,
    /// The correlation id of the execution trace.
    pub trace_id: String,
    /// The provider that served the request.
    pub provider: String,
    /// The canonical model id that served the request.
    pub model: String,
    pub elapsed_us: u64,
    pub request_sha256: String,
}

/// The whole evaluation report.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EvaluationReport {
    pub protocol: String,
    pub corpus_id: String,
    pub corpus_kind: String,
    /// Whether a real model was called in this run.
    pub model_called: bool,
    pub model_note: String,
    /// The canonical model id of the run.
    pub model: String,
    /// The non-secret endpoint of the run (`None` on the offline path).
    pub endpoint: Option<String>,
    /// The **key names** of the vendor-private request fields in play.
    ///
    /// ⭐ Names only, never values: enough to reproduce the request shape
    /// without copying operator-supplied content into the artifact.
    pub vendor_fields: Vec<String>,
    /// Where the price table came from (`None` when no price table was set).
    pub price_source: Option<String>,
    pub cases: Vec<CaseArtifact>,
    pub passed_cases: usize,
    pub failed_cases: usize,
    pub premise_summary: Vec<Value>,
    pub negative_controls: Vec<Value>,
    pub probes: Vec<Value>,
    pub distributions: Vec<Distribution>,
    pub seal_chain: Value,
    /// The one-time reveal entry, when the run was bound to a custody dir.
    pub reveal: Option<Value>,
    /// What was written into the store before the questions were asked.
    pub corpus_write: Value,
    /// How many provider calls the run served (one per turn, one per query).
    pub provider_calls: usize,
    /// How many calls the deterministic corpus-acknowledgement fixture served.
    ///
    /// ⭐ The corpus path never reaches a real model; this counter is the
    /// evidence for that claim.
    pub corpus_calls: usize,
    /// How many committed user inputs the audit trail observed.
    ///
    /// ⭐ The direct-user premise is decided against this trail, so the count
    /// belongs in the artifact.
    pub observed_committed_inputs: usize,
    pub notes: Vec<String>,
}

/// The usage of one provider call, as the provider reported it.
pub fn usage_of(usage: &NormalizedUsage) -> Usage {
    Usage {
        prompt_tokens: usage.prompt_tokens,
        completion_tokens: usage.completion_tokens,
        total_tokens: usage.total_tokens,
        prompt_cache_hit_tokens: usage.prompt_cache_hit_tokens,
    }
}

/// The cost of one provider call.
///
/// ⭐ The amount is only computed when a price table was configured **and** the
/// provider reported a usage. Without a price table the cost is `not_wired`
/// — that is "cannot be computed", not "zero".
pub fn cost_of(config: &Config, usage: &Usage) -> Cost {
    let priced =
        config.input_price_per_million.is_some() && config.output_price_per_million.is_some();
    if !config.is_live() {
        return Cost {
            status: "not_wired".to_string(),
            currency: config.currency.clone(),
            price_date: config.price_date.clone(),
            price_source: config.price_source.clone(),
            amount: None,
            note: "本轮未调用真实模型 ⇒ 无真实用量，价格表未接入（不是 0 元）。".to_string(),
        };
    }
    if !priced {
        return Cost {
            status: "not_wired".to_string(),
            currency: config.currency.clone(),
            price_date: config.price_date.clone(),
            price_source: config.price_source.clone(),
            amount: None,
            note: "live 模式但价格表未接入 ⇒ 费用无法计算（不是 0 元）。".to_string(),
        };
    }
    if usage.total_tokens == 0 {
        return Cost {
            status: "not_wired".to_string(),
            currency: config.currency.clone(),
            price_date: config.price_date.clone(),
            price_source: config.price_source.clone(),
            amount: None,
            note: "provider 未报告用量 ⇒ 费用无法计算（不是 0 元）。".to_string(),
        };
    }
    let input_price = config.input_price_per_million.unwrap_or(0.0);
    let output_price = config.output_price_per_million.unwrap_or(0.0);
    let cache_price = config.cache_hit_price_per_million.unwrap_or(0.0);
    let cache_tokens = f64::from(usage.prompt_cache_hit_tokens.unwrap_or(0));
    let prompt_tokens = f64::from(usage.prompt_tokens);
    let uncached_prompt = (prompt_tokens - cache_tokens).max(0.0);
    let amount = (uncached_prompt * input_price
        + cache_tokens * cache_price
        + f64::from(usage.completion_tokens) * output_price)
        / 1_000_000.0;
    Cost {
        status: "wired".to_string(),
        currency: config.currency.clone(),
        price_date: config.price_date.clone(),
        price_source: config.price_source.clone(),
        amount: Some(amount),
        note: "按本轮配置的单价折算；单价来源见 price_source。".to_string(),
    }
}

/// The evaluation's own reasoning-effort switch.
///
/// ⭐ It belongs to this evaluation, not to the provider crate: the
/// production factory owns only the reasoning-mode switch, and the
/// architecture contract budgets one environment name per capability switch.
const EVAL_REASONING_EFFORT_ENV: &str = "APEIRETH_OPENAI_REASONING_EFFORT";

/// The live configuration, read from the environment.
///
/// ⭐ Only **names** and non-secret values are read here: the endpoint, the
/// model id, the name of the credential variable and the price table. The
/// credential itself is resolved per turn by the provider's own resolver.
fn live_config() -> Result<Config> {
    let endpoint = std::env::var("APEIRETH_OPENAI_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or("live 模式需要环境变量 APEIRETH_OPENAI_URL（端点，不含密钥）")?;
    let model = match std::env::var("APEIRETH_OPENAI_MODEL")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        Some(model) => model,
        None => std::env::var("APEIRETH_OPENAI_MODELS")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(|value| {
                value
                    .split(',')
                    .map(|model| model.trim().to_string())
                    .filter(|model| !model.is_empty())
                    .collect::<Vec<_>>()
                    .first()
                    .cloned()
                    .unwrap_or_default()
            })
            .filter(|model| !model.is_empty())
            .ok_or("live 模式需要环境变量 APEIRETH_OPENAI_MODEL 或 APEIRETH_OPENAI_MODELS")?,
    };
    let mut config = Config::live(
        &endpoint,
        &model,
        apeireth_provider::credentials::OPENAI_COMPATIBLE_API_KEY_ENV,
    );
    // 厂商的推理模式开关（**可选**，默认不发）。
    //
    // ⭐ 只有运维方明确要求时才发这个厂商私有字段：端点不认识它时可能直接
    // 拒单 ⇒ 默认不发＝端点保持自己的默认行为。
    //
    // ⭐ 取值口径在 provider 侧（`apeireth_provider::vendor_body`）：生产工厂
    // 与本评测共用同一份映射 ⇒ 不维护两份真源。
    if let Some(value) = std::env::var(apeireth_provider::vendor_body::THINKING_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        config.thinking = Some(
            apeireth_provider::vendor_body::parse_thinking(&value)
                .map_err(|error| error.to_string())?,
        );
    }
    config.reasoning_effort = std::env::var(EVAL_REASONING_EFFORT_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty());
    if let Some(price) = std::env::var("APEIRETH_PRICE_INPUT_PER_MILLION")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        config.input_price_per_million = price.parse().ok();
    }
    if let Some(price) = std::env::var("APEIRETH_PRICE_OUTPUT_PER_MILLION")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        config.output_price_per_million = price.parse().ok();
    }
    if let Some(price) = std::env::var("APEIRETH_PRICE_CACHE_HIT_PER_MILLION")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        config.cache_hit_price_per_million = price.parse().ok();
    }
    config.currency = std::env::var("APEIRETH_PRICE_CURRENCY")
        .ok()
        .filter(|value| !value.trim().is_empty());
    config.price_date = std::env::var("APEIRETH_PRICE_DATE")
        .ok()
        .filter(|value| !value.trim().is_empty());
    config.price_source = std::env::var("APEIRETH_PRICE_SOURCE")
        .ok()
        .filter(|value| !value.trim().is_empty());
    Ok(config)
}

/// The CLI arguments.
struct Cli {
    command: String,
    output: Option<PathBuf>,
    custody: Option<PathBuf>,
    pressure: PressureOptions,
    /// Whether the live (real model) path was requested.
    live: bool,
}

fn usage_text() -> &'static str {
    "memory-longterm-eval — 独立长程评测与规模/持续运行压力契约\n\
     \n\
     用法：\n\
     \x20 memory-longterm-eval plan\n\
     \x20     打印计划（题集、oracle、前提模型、封存链、协议摘要），不写盘、不发请求。\n\
     \x20 memory-longterm-eval run --output DIR [--live] [--custody DIR]\n\
     \x20     真实跑一遍评测：写题集 → 冷问 → 取证 → 评分 → 落产物。\n\
     \x20     默认走确定性离线路径（模型侧是离线夹具，产物标注「本轮未调用真实模型」）。\n\
     \x20     --live 才走真实模型：端点/模型/凭据名/单价表全部取自环境变量，\n\
     \x20     凭据不进参数、不进源码、不进日志、不进产物、不进报告。\n\
     \x20     --custody DIR 时先校验封存链再发第一个请求，并登记一次性揭封。\n\
     \x20 memory-longterm-eval pressure --output DIR [--records N] [--concurrency N]\n\
     \x20     [--turns N] [--seconds N] [--virtual-days N]\n\
     \x20     规模与持续运行压力（B3）；两者分别报告。\n\
     \x20 memory-longterm-eval seal --custody DIR\n\
     \x20     生成封存链：seal + build receipt + freeze。\n\
     \x20 memory-longterm-eval verify --custody DIR\n\
     \x20     由第三方重算整条封存链并校验（可核验性）。\n\
     \n\
     口径：回归集（非独立盲测）；同账号隔离（非强保密）；规模与持续运行分别报告；\n\
     虚拟时间推进不是持续运行；离线路径未调用真实模型。"
}

fn parse_cli(args: &[String]) -> Result<Cli> {
    let mut cli = Cli {
        command: args.first().cloned().unwrap_or_else(|| "help".to_string()),
        output: None,
        custody: None,
        pressure: PressureOptions::default(),
        live: false,
    };
    let mut index = 1;
    while index < args.len() {
        let arg = args[index].as_str();
        let mut take_value = |index: &mut usize| -> Result<String> {
            *index += 1;
            args.get(*index)
                .cloned()
                .ok_or_else(|| format!("参数 {arg} 缺值").into())
        };
        match arg {
            "--output" => cli.output = Some(PathBuf::from(take_value(&mut index)?)),
            "--custody" => cli.custody = Some(PathBuf::from(take_value(&mut index)?)),
            "--live" => cli.live = true,
            "--records" => {
                cli.pressure.records = take_value(&mut index)?
                    .parse()
                    .map_err(|_| "bad --records")?
            }
            "--concurrency" => {
                cli.pressure.concurrency = take_value(&mut index)?
                    .parse()
                    .map_err(|_| "bad --concurrency")?
            }
            "--turns" => {
                cli.pressure.chain_turns =
                    take_value(&mut index)?.parse().map_err(|_| "bad --turns")?
            }
            "--seconds" => {
                cli.pressure.sustained_seconds = take_value(&mut index)?
                    .parse()
                    .map_err(|_| "bad --seconds")?
            }
            "--virtual-days" => {
                cli.pressure.virtual_horizon_days = take_value(&mut index)?
                    .parse()
                    .map_err(|_| "bad --virtual-days")?
            }
            other => return Err(format!("未知参数 {other}").into()),
        }
        index += 1;
    }
    Ok(cli)
}

/// The plan of one evaluation: what would be run, against which bindings.
pub fn plan_document() -> Value {
    let input = corpus::input_bundle();
    let oracle = corpus::oracle_bundle(&input);
    let mut sources = BTreeMap::new();
    for (name, digest) in seal::evaluator_source_digests() {
        sources.insert(name, digest);
    }
    json!({
        "protocol": MANIFEST_PROTOCOL,
        "corpus_id": CORPUS_ID,
        "corpus_kind": "回归集（本仓自编，不是独立盲测）",
        "queries": input.queries.len(),
        "turns": input.turns.len(),
        "sessions": input.sessions.len(),
        "operations": input.operations.len(),
        "oracle_cases": oracle.cases.len(),
        "input_sha256": oracle.input_sha256,
        "oracle_sha256": hash(&serde_json::to_string(&oracle).unwrap_or_default()),
        "premises": schema::PremiseKind::ALL.iter().map(|kind| json!({
            "kind": kind.label(),
            "question": kind.question(),
        })).collect::<Vec<_>>(),
        "evaluator_sources": sources,
        "artifact_layout": Paths {
            input: "（题集由代码内建，不是外部文件）".to_string(),
            config: Some("（配置由代码内建，不是外部文件）".to_string()),
            seal: Some("custody/seal.json".to_string()),
            freeze: Some("custody/freeze.json".to_string()),
            build: Some("custody/build-receipt.json".to_string()),
            output: "<--output>/manifest.json, cases/, calls/, report.json".to_string(),
        },
        "model_called": false,
        "model_note": "本轮未调用真实模型：模型侧是确定性离线夹具。",
        "notes": input.notes,
    })
}

/// Write the manifest of a run.
fn write_manifest(dir: &Path, config: &Config, harness: &Harness) -> Result<Value> {
    let input = corpus::input_bundle();
    let oracle = corpus::oracle_bundle(&input);
    let mut sources = BTreeMap::new();
    for (name, digest) in seal::evaluator_source_digests() {
        sources.insert(name, digest);
    }
    let executable = std::env::current_exe()?;
    let manifest = json!({
        "protocol": MANIFEST_PROTOCOL,
        "corpus_id": CORPUS_ID,
        "corpus_kind": "回归集（本仓自编，不是独立盲测）",
        "input_sha256": oracle.input_sha256,
        "oracle_sha256": hash(&serde_json::to_string(&oracle).unwrap_or_default()),
        "evaluator_sources": sources,
        "executable_sha256": hash_bytes(&std::fs::read(&executable)?),
        "executable_path": executable.display().to_string(),
        "config": config,
        "policy_grants": harness.policy.iter().map(|permission| permission.label()).collect::<Vec<_>>(),
        "model_called": config.is_live(),
        "model": harness.provider_model,
        "provider_label": harness.provider_label,
        "endpoint": config.endpoint,
        "vendor_fields": harness.vendor_fields,
        "vendor_fields_note": "厂商私有请求字段的**键名**（不含取值）：记录这一跑的请求形状，不复制运维方输入。",
        "price_source": config.price_source,
        "model_note": if config.is_live() {
            "live 模式：真实模型由 --live 显式开启；凭据只经环境变量按轮解析，不进产物。"
        } else {
            "本轮未调用真实模型：模型侧是确定性离线夹具（provider.memory-longterm.offline-reader）。"
        },
        "corpus_provenance": {
            "generator": "corpus.rs 的内建题面（确定性生成，无随机种子）",
            "seed": null,
            "note": "同源确定性题集 ≠ 真人盲测；本卷是回归集，不是独立盲测。",
        },
        "oracle_isolation": {
            "canary": ORACLE_CANARY,
            "note": "canary 只出现在 oracle 侧；请求与存储里都不许出现。",
        },
        "custody": "同账号文件隔离（同一账号、同一台机器上的文件隔离），不是强保密。",
        "notes": input.notes,
    });
    write_new(&dir.join("manifest.json"), &manifest)?;
    Ok(manifest)
}

/// The timings of one turn.
///
/// ⭐ The trace timestamps come from the injected clock. On this run the clock
/// is virtual, so the queue/provider durations derived from the trace are
/// **virtual** values: they are recorded as such and never presented as real
/// waiting time. The wall-clock duration is measured around the turn.
fn timing_of(response: &apeireth_runtime::canonical::TurnResponse, wall_clock_us: u64) -> Timing {
    let trace: &ExecutionTrace = &response.trace;
    let first_at = trace.entries.first().map(|entry| entry.at.epoch_millis());
    let invoked_at = trace.entries.iter().find_map(|entry| match &entry.event {
        TraceEvent::ProviderInvoked { .. } => Some(entry.at.epoch_millis()),
        _ => None,
    });
    let succeeded_at = trace.entries.iter().find_map(|entry| match &entry.event {
        TraceEvent::ProviderSucceeded { .. } => Some(entry.at.epoch_millis()),
        _ => None,
    });
    let last_at = trace.entries.last().map(|entry| entry.at.epoch_millis());
    let queue = match (first_at, invoked_at) {
        (Some(first), Some(invoked)) => Some(invoked - first),
        _ => None,
    };
    let provider = match (invoked_at, succeeded_at) {
        (Some(invoked), Some(succeeded)) => Some(succeeded - invoked),
        _ => None,
    };
    let total = match (first_at, last_at) {
        (Some(first), Some(last)) => Some(last - first),
        _ => None,
    };
    Timing {
        wall_clock_us,
        provider_call_us: 0,
        virtual_queue_wait_ms: queue,
        virtual_provider_ms: provider,
        virtual_total_ms: total,
        clock_source:
            "轨迹时间戳取自注入时钟（本轮是虚拟时钟）⇒ 排队/提供方时长是虚拟值，不是真实墙钟等待时间"
                .to_string(),
    }
}

/// The provider and model that served the request, from the trace.
fn provider_of(response: &apeireth_runtime::canonical::TurnResponse) -> (String, String) {
    let mut provider = String::new();
    let mut model = String::new();
    for entry in &response.trace.entries {
        if let TraceEvent::ProviderInvoked {
            provider: id,
            model: id_model,
            ..
        } = &entry.event
        {
            provider = id.to_string();
            model = id_model.clone();
            break;
        }
    }
    if provider.is_empty() {
        provider = response.served_by.to_string();
    }
    (provider, model)
}

/// One premise probe: the same delivery, decided under a different condition.
fn probe(name: &str, requirement: &str, observed: String, passed: bool) -> Value {
    json!({
        "name": name,
        "requirement": requirement,
        "observed": observed,
        "passed": passed,
    })
}

/// Bind the shard to this run: seal, freeze, verify, reveal — all before the
/// first provider request.
///
/// ⭐ The closure has four parts, and every one of them is walked here:
/// * **独立保管** — the custody directory is a separate file area; holding it
///   under another holder or another host is **未实现** and is written down
///   as such (same account, same host ⇒ 同账号隔离, not 强保密).
/// * **首次揭封** — `claim_reveal` writes the first entry of the append-only
///   ledger.
/// * **单次使用** — a second claim for the same seal is refused, and the
///   refusal is asserted here (a probe in the artifact).
/// * **证明可核验** — `verify_reveal` recomputes the seal body digest and
///   reads the ledger, without any internal state of the sealing side.
fn bind_seal_chain(
    custody: &Path,
    input: &schema::InputBundle,
    oracle: &schema::OracleBundle,
    config: &Config,
    run_dir: &Path,
) -> Result<(Value, Value, Vec<Value>)> {
    let executable = std::fs::read(std::env::current_exe()?)?;
    let seal_path = custody.join("seal.json");
    let receipt_path = custody.join("build-receipt.json");
    let freeze_path = custody.join("freeze.json");
    let seal = if seal_path.is_file() {
        seal::read_seal(custody)?
    } else {
        seal::create_seal(input, oracle, custody)?
    };
    let receipt = if receipt_path.is_file() {
        seal::read_build_receipt(custody)?
    } else {
        let receipt = seal::create_build_receipt(
            "cargo build -p apeireth-runtime-assembly --example memory_longterm_eval",
            &format!("{:?}", config),
            &executable,
        )?;
        write_new(&receipt_path, &receipt)?;
        receipt
    };
    let config_sha256 = hash(&serde_json::to_string(config)?);
    let freeze = if freeze_path.is_file() {
        seal::read_freeze(custody)?
    } else {
        let freeze = seal::create_freeze(
            &seal,
            &receipt,
            &config_sha256,
            1,
            &hash(schema::READER_SYSTEM),
        );
        write_new(&freeze_path, &freeze)?;
        freeze
    };
    // ⭐ The binding is checked before anything is sent: a broken chain means
    // no model request may leave this process.
    if let Err(problem) =
        seal::verify_chain(input, &seal, &freeze, &receipt, &config_sha256, &executable)
    {
        return Err(format!(
            "seal/freeze/build/config binding failed; no model request sent: {problem}"
        )
        .into());
    }
    let run_dir_text = run_dir.display().to_string();
    let entry = seal::claim_reveal(
        custody,
        &seal.seal_sha256,
        &run_dir_text,
        &oracle_sha256(oracle),
    )?;
    // ⭐ One-time use: the second claim for the same seal must be refused.
    let second = seal::claim_reveal(
        custody,
        &seal.seal_sha256,
        &format!("{run_dir_text}-second"),
        &oracle_sha256(oracle),
    );
    let mut probes = Vec::new();
    probes.push(probe(
        "reveal_is_one_time",
        "a shard may be revealed only once; the second claim must be refused",
        match &second {
            Ok(_) => "the second reveal was accepted".to_string(),
            Err(error) => format!("the second reveal was refused: {error}"),
        },
        second.is_err(),
    ));
    // ⭐ Verifiable proof: an outside verifier only needs the files.
    let verified = seal::verify_reveal(custody, Some(&run_dir_text));
    probes.push(probe(
        "reveal_is_independently_verifiable",
        "the reveal must be verifiable from the custody files alone (seal body digest + ledger)",
        match &verified {
            Ok(entry) => format!(
                "the ledger entry {} was verified from the files",
                entry.seal_sha256
            ),
            Err(error) => format!("the reveal could not be verified: {error}"),
        },
        verified.is_ok(),
    ));
    let chain = json!({
        "status": "verified",
        "custody": custody.display().to_string(),
        "seal_sha256": seal.seal_sha256,
        "seal_body_digest_recomputed": seal::seal_body_digest(&seal),
        "build_receipt_sha256": freeze.build_receipt_sha256,
        "config_sha256": freeze.config_sha256,
        "expected_cases": freeze.expected_cases,
        "custody_mode": freeze.custody_mode,
        "independent_custody": "未实现：独立保管需要另一名持有人或另一台机器，本轮未接线。",
        "first_exposure_utc": freeze.first_exposure_utc,
        "tuning_on_this_shard_forbidden": freeze.tuning_on_this_shard_forbidden,
    });
    Ok((chain, serde_json::to_value(&entry)?, probes))
}

/// Run the evaluation end to end.
///
/// ⭐ The seal chain is verified **before the first request** and the one-time
/// reveal is claimed for this run. A failed binding aborts the run with
/// "no model request sent".
async fn evaluate(
    output: &Path,
    config: &Config,
    custody: Option<&Path>,
) -> Result<EvaluationReport> {
    let dir = new_run(output)?;
    let input = corpus::input_bundle();
    validate_input(&input)?;
    let oracle = corpus::oracle_bundle(&input);
    validate_oracle(&oracle, &input)?;
    config.validate(config.is_live())?;

    // ── The seal chain: verified before anything is sent ──────────────────
    // ⭐ Every run is bound to a custody directory: the default one lives
    // inside the run directory, so the closure is never skipped.
    let custody_dir = match custody {
        Some(path) => path.to_path_buf(),
        None => dir.join("custody"),
    };
    let (seal_chain, reveal, seal_probes) =
        bind_seal_chain(&custody_dir, &input, &oracle, config, &dir)?;

    let db = dir.join("memory.sqlite3");
    let harness = Harness::new(&db, config.clone(), &input)?;
    let corpus_report = harness.write_corpus(&input).await?;
    write_manifest(&dir, config, &harness)?;

    let mut cases: Vec<CaseArtifact> = Vec::new();
    let mut spans: Vec<Span> = Vec::new();
    let mut premise_summary: Vec<Value> = Vec::new();
    let mut probes: Vec<Value> = seal_probes.clone();

    for query in &input.queries {
        let oracle_case = oracle
            .cases
            .iter()
            .find(|case| case.id == query.id)
            .ok_or_else(|| format!("oracle case {} is missing", query.id))?;
        let started = Stopwatch::start();
        // ⭐ The question runs on the **production path**: the writeback and
        // the recovery drain are live, so the answer is produced by the same
        // chain a user turn takes.
        let response = harness.ask_writable(&query.session, &query.text).await?;
        let elapsed_us = started.elapsed_us();
        let request = harness
            .calls
            .last()
            .ok_or("the provider was never called for this case")?;
        let partition = harness.partition(&query.session)?;
        let receipt = evidence::inspect(&Inspect {
            request: &request,
            db: &db,
            partition: &partition,
            policy: &harness.policy,
            audit: &harness.audit,
            canary: ORACLE_CANARY,
            case: &query.id,
        })?;
        let grade = grading::grade(&GradeInput {
            case: &query.id,
            raw_answer: &response.text,
            oracle_case,
            evidence: &receipt.sources,
            runtime_errors: &[],
        });
        let request_sha256 = hash(&serde_json::to_string(&request).unwrap_or_default());
        let call = harness
            .calls
            .last_record()
            .ok_or("the provider call was never recorded for this case")?;
        let usage = usage_of(&call.usage);
        let (provider, model) = provider_of(&response);
        let mut timing = timing_of(&response, elapsed_us);
        timing.provider_call_us = call.elapsed_us;
        let artifact = CaseArtifact {
            case: query.id.clone(),
            dimension: oracle_case.dimension.clone(),
            session: query.session.clone(),
            question: query.text.clone(),
            answer_sha256: hash(&response.text),
            answer_text: response.text.clone(),
            grade,
            evidence: serde_json::to_value(&receipt)?,
            premise_summary: serde_json::to_value(&receipt.premise_summary)?,
            usage: usage.clone(),
            cost: cost_of(config, &usage),
            timing,
            request_id: response.request.to_string(),
            trace_id: response.trace.trace.to_string(),
            provider,
            model,
            elapsed_us,
            request_sha256,
        };
        let call_path = dir
            .join("calls")
            .join(format!("call-{}.json", artifact.case));
        write_new(&call_path, &request)?;
        spans.push(Span {
            case: query.id.clone(),
            stage: "cold_query".to_string(),
            elapsed_us,
            outcome: if artifact.grade.passed {
                "ok"
            } else {
                "failed"
            }
            .to_string(),
        });
        for summary in &receipt.premise_summary {
            premise_summary.push(serde_json::to_value(summary)?);
        }
        // Oracle leakage: the canary must not appear in the answer either.
        if response.text.contains(ORACLE_CANARY) {
            probes.push(probe(
                "oracle_canary_absent_from_answer",
                "the answer must not carry the oracle canary",
                format!("case {} carries the canary", query.id),
                false,
            ));
        }
        cases.push(artifact);
    }

    // ── The premise probes: each premise must be decidable and flip ───────
    probes.extend(premise_probes(&harness, &db, &input).await?);

    // ── The cold read-only query must reach the same answer ───────────────
    probes.extend(cold_query_probe(&harness, &db, &input, &oracle).await?);

    // ── Oracle leakage: the canary must not have reached the store ────────
    let canary_in_store = evidence::canary_in_store(&db, ORACLE_CANARY)?;
    probes.push(probe(
        "oracle_canary_absent_from_store",
        "the oracle canary must never be written into the memory store",
        if canary_in_store {
            "the canary was found in the store".to_string()
        } else {
            "the canary was not found in the store".to_string()
        },
        !canary_in_store,
    ));

    // ── Negative controls: the grader must reject every forgery ──────────
    let negative_controls = negative_control_report(&cases, &input)?;
    for control in &negative_controls {
        if control["rejected"] != json!(true) {
            probes.push(probe(
                "negative_control_rejected",
                "every forged answer must be rejected",
                format!("{}: {}", control["name"], control["errors"]),
                false,
            ));
        }
    }

    let passed_cases = cases.iter().filter(|case| case.grade.passed).count();
    let failed_cases = cases.len() - passed_cases;
    let distributions = artifacts::distributions(&spans);
    let corpus_write = json!({
        "turns": corpus_report.turns.iter().map(|turn| json!({
            "id": turn.id,
            "session": turn.session,
            "request": turn.request,
            "answer_sha256": turn.answer_sha256,
        })).collect::<Vec<_>>(),
        "forgets": corpus_report.forgets.iter().map(|forget| json!({
            "turn": forget.turn,
            "episode_id": forget.episode_id,
            "revision_before": forget.revision_before,
            "revision_after": forget.revision_after,
            "status_after": forget.status_after,
        })).collect::<Vec<_>>(),
        "stored_records": corpus_report.stored_records,
    });
    let report = EvaluationReport {
        protocol: REPORT_PROTOCOL.to_string(),
        corpus_id: CORPUS_ID.to_string(),
        corpus_kind: "回归集（本仓自编，不是独立盲测）".to_string(),
        model_called: config.is_live(),
        model_note: if config.is_live() {
            "live 模式：真实模型已被调用（--live 显式开启）；凭据只经环境变量按轮解析，不进产物。"
                .to_string()
        } else {
            "本轮未调用真实模型：模型侧是确定性离线夹具。".to_string()
        },
        model: harness.provider_model.clone(),
        endpoint: config.endpoint.clone(),
        vendor_fields: harness.vendor_fields.clone(),
        price_source: config.price_source.clone(),
        passed_cases,
        failed_cases,
        premise_summary,
        negative_controls,
        probes,
        distributions,
        seal_chain,
        reveal: Some(reveal),
        corpus_write,
        provider_calls: harness.calls.len(),
        corpus_calls: harness.corpus_calls.records().len(),
        observed_committed_inputs: harness.audit.observed_requests(),
        cases,
        notes: vec![
            "既有题集是回归集，不是独立盲测；同源确定性题集 ≠ 真人盲测。".to_string(),
            "不同账号的记录按会话作用域隔离：同账号隔离，不是强保密。".to_string(),
            if config.is_live() {
                "请求关联：每例记录 request_id、trace_id、provider 与 canonical model id。"
                    .to_string()
            } else {
                "请求关联：本轮未调用真实模型 ⇒ 无真实 provider 事件；关联 id 取自运行时轨迹。"
                    .to_string()
            },
            "排队时长：轨迹时间戳取自注入的虚拟时钟 ⇒ 是虚拟值，不是真实墙钟等待时间；真实墙钟耗时单独记在 timing.wall_clock_us。"
                .to_string(),
            if config.is_live() {
                "用量与费用：按 provider 报告的 usage 与本轮配置的单价折算；单价来源见 price_source；未接单价时写「未接线」，不是 0 元。"
                    .to_string()
            } else {
                "完整用量与费用：本轮未调用真实模型 ⇒ 无真实用量，价格表未接入（不是 0 元）。"
                    .to_string()
            },
            "语料写入走确定性 ack 夹具（不产生真实模型调用）；只有问句轮次走评测所用的模型路径。"
                .to_string(),
        ],
    };
    write_new(&dir.join("report.json"), &report)?;
    Ok(report)
}

/// The premise probes: every premise must be independently decidable, and
/// each must actually flip when its condition changes.
async fn premise_probes(
    harness: &Harness,
    db: &Path,
    input: &schema::InputBundle,
) -> Result<Vec<Value>> {
    let mut probes = Vec::new();
    let query = input
        .queries
        .iter()
        .find(|query| query.id == "c01_long_range_direct")
        .ok_or("the c01 query is missing")?;
    let response = harness.ask(&query.session, &query.text).await?;
    let request = harness
        .calls
        .last()
        .ok_or("the provider was never called for the probe")?;
    let partition = harness.partition(&query.session)?;

    // Probe 1: the Host authorization premise.
    let granted = evidence::inspect(&Inspect {
        request: &request,
        db,
        partition: &partition,
        policy: &harness.policy,
        audit: &harness.audit,
        canary: ORACLE_CANARY,
        case: "probe_authorization",
    })?;
    let mut denying = harness.policy.clone();
    denying.revoke(&apeireth_governance::Permission::ExecuteTool(
        evidence::RECALL_CAPABILITY.to_string(),
    ));
    let denied = evidence::inspect(&Inspect {
        request: &request,
        db,
        partition: &partition,
        policy: &denying,
        audit: &harness.audit,
        canary: ORACLE_CANARY,
        case: "probe_authorization",
    })?;
    let granted_holds = granted
        .premise_summary
        .iter()
        .any(|summary| summary.label == "host_authorization" && summary.satisfied_count > 0);
    let denied_fails = denied
        .premise_summary
        .iter()
        .any(|summary| summary.label == "host_authorization" && summary.satisfied_count == 0);
    probes.push(probe(
        "authorization_premise_follows_the_policy",
        "the host_authorization premise is true with the capability granted and false without it",
        format!(
            "granted_satisfied={} denied_satisfied={}",
            granted
                .premise_summary
                .iter()
                .find(|summary| summary.label == "host_authorization")
                .map(|summary| summary.satisfied_count)
                .unwrap_or(0),
            denied
                .premise_summary
                .iter()
                .find(|summary| summary.label == "host_authorization")
                .map(|summary| summary.satisfied_count)
                .unwrap_or(0)
        ),
        granted_holds && denied_fails,
    ));

    // Probe 2: the current-scope premise (a foreign partition must fail it).
    let foreign_partition = "00000000-0000-4000-8000-000000000000".to_string();
    let foreign = evidence::inspect(&Inspect {
        request: &request,
        db,
        partition: &foreign_partition,
        policy: &harness.policy,
        audit: &harness.audit,
        canary: ORACLE_CANARY,
        case: "probe_scope",
    })?;
    let own = evidence::inspect(&Inspect {
        request: &request,
        db,
        partition: &partition,
        policy: &harness.policy,
        audit: &harness.audit,
        canary: ORACLE_CANARY,
        case: "probe_scope",
    })?;
    let own_scope_holds = own
        .premise_summary
        .iter()
        .any(|summary| summary.label == "current_scope" && summary.satisfied_count > 0);
    let foreign_scope_fails = foreign
        .premise_summary
        .iter()
        .any(|summary| summary.label == "current_scope" && summary.satisfied_count == 0);
    probes.push(probe(
        "scope_premise_follows_the_partition",
        "the current_scope premise is true for the owning partition and false for a foreign one",
        format!(
            "own_satisfied={} foreign_satisfied={}",
            own.premise_summary
                .iter()
                .find(|summary| summary.label == "current_scope")
                .map(|summary| summary.satisfied_count)
                .unwrap_or(0),
            foreign
                .premise_summary
                .iter()
                .find(|summary| summary.label == "current_scope")
                .map(|summary| summary.satisfied_count)
                .unwrap_or(0)
        ),
        own_scope_holds && foreign_scope_fails,
    ));
    let _ = response;
    Ok(probes)
}

/// Run every negative control against the real grader.
fn negative_control_report(
    cases: &[CaseArtifact],
    input: &schema::InputBundle,
) -> Result<Vec<Value>> {
    let mut reports = Vec::new();
    for case in cases {
        let oracle_case = corpus::oracle_bundle(input)
            .cases
            .into_iter()
            .find(|oracle| oracle.id == case.case)
            .ok_or_else(|| format!("oracle case {} is missing", case.case))?;
        let evidence = serde_json::from_value::<Vec<schema::SourceEvidence>>(
            case.evidence["sources"].clone(),
        )?;
        for control in grading::negative_controls(&case.answer_text) {
            let grade = grading::grade(&GradeInput {
                case: &case.case,
                raw_answer: &control.answer,
                oracle_case: &oracle_case,
                evidence: &evidence,
                runtime_errors: &[],
            });
            reports.push(json!({
                "case": case.case,
                "name": control.name,
                "rejected": !grade.passed,
                "errors": grade.errors,
            }));
        }
    }
    Ok(reports)
}

/// The pressure run.
async fn run_pressure(output: &Path, options: &PressureOptions) -> Result<Value> {
    let dir = new_run(output)?;
    let input = corpus::input_bundle();
    let config = Config::offline();
    let db = dir.join("pressure.sqlite3");
    let mut harness = Harness::new(&db, config.clone(), &input)?;
    harness.set_persistent_sessions(true);
    let report = pressure::run(&harness, options).await?;
    let value = serde_json::to_value(&report)?;
    write_new(&dir.join("pressure.json"), &value)?;
    Ok(value)
}

/// Probe: the answer must also be reachable from storage alone.
///
/// ⭐ The evaluation questions run on the production path (writeback and
/// recovery drain live). This probe asks the same question again on a cold,
/// read-only runtime: with no live transcript, the answer can only come from
/// the memory plane ⇒ this is the long-range property itself.
async fn cold_query_probe(
    harness: &Harness,
    db: &Path,
    input: &schema::InputBundle,
    oracle: &schema::OracleBundle,
) -> Result<Vec<Value>> {
    let query = input
        .queries
        .iter()
        .find(|query| query.id == "c01_long_range_direct")
        .ok_or("the c01 query is missing")?;
    let oracle_case = oracle
        .cases
        .iter()
        .find(|case| case.id == query.id)
        .ok_or("the c01 oracle case is missing")?;
    let response = harness.ask(&query.session, &query.text).await?;
    let request = harness
        .calls
        .last()
        .ok_or("the provider was never called for the cold probe")?;
    let partition = harness.partition(&query.session)?;
    let receipt = evidence::inspect(&Inspect {
        request: &request,
        db,
        partition: &partition,
        policy: &harness.policy,
        audit: &harness.audit,
        canary: ORACLE_CANARY,
        case: "probe_cold",
    })?;
    let grade = grading::grade(&GradeInput {
        case: &query.id,
        raw_answer: &response.text,
        oracle_case,
        evidence: &receipt.sources,
        runtime_errors: &[],
    });
    Ok(vec![probe(
        "the_cold_query_answers_from_storage",
        "the same question on a cold read-only runtime must be answered from storage alone",
        format!(
            "grade_errors={:?} delivered_sources={}",
            grade.errors,
            receipt.sources.len()
        ),
        grade.passed,
    )])
}

/// The CLI entry point. `Ok(true)` means everything passed.
pub async fn run(args: &[String]) -> Result<bool> {
    let cli = parse_cli(args)?;
    match cli.command.as_str() {
        "help" | "--help" | "-h" => {
            println!("{}", usage_text());
            Ok(true)
        }
        "plan" => {
            println!("{}", serde_json::to_string_pretty(&plan_document())?);
            Ok(true)
        }
        "run" => {
            let output = cli
                .output
                .ok_or("run 需要 --output DIR（产物目录必须不存在）")?;
            let config = if cli.live {
                live_config()?
            } else {
                Config::offline()
            };
            let report = evaluate(&output, &config, cli.custody.as_deref()).await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "protocol": report.protocol,
                    "corpus_id": report.corpus_id,
                    "corpus_kind": report.corpus_kind,
                    "model_called": report.model_called,
                    "model": report.model,
                    "endpoint": report.endpoint,
                    "passed_cases": report.passed_cases,
                    "failed_cases": report.failed_cases,
                    "probes_failed": report.probes.iter().filter(|probe| probe["passed"] != json!(true)).count(),
                    "negative_controls": report.negative_controls.len(),
                    "negative_controls_rejected": report.negative_controls.iter().filter(|control| control["rejected"] == json!(true)).count(),
                    "seal_chain": report.seal_chain,
                    "output": output.display().to_string(),
                }))?
            );
            Ok(report.failed_cases == 0
                && report
                    .probes
                    .iter()
                    .all(|probe| probe["passed"] == json!(true))
                && report
                    .negative_controls
                    .iter()
                    .all(|control| control["rejected"] == json!(true)))
        }
        "pressure" => {
            let output = cli
                .output
                .ok_or("pressure 需要 --output DIR（产物目录必须不存在）")?;
            let value = run_pressure(&output, &cli.pressure).await?;
            let failed = value["checks"]
                .as_array()
                .map(|checks| {
                    checks
                        .iter()
                        .filter(|check| check["passed"] != json!(true))
                        .count()
                })
                .unwrap_or(0);
            println!("{}", serde_json::to_string_pretty(&value)?);
            Ok(failed == 0)
        }
        "seal" => {
            let custody = cli.custody.ok_or("seal 需要 --custody DIR")?;
            let input = corpus::input_bundle();
            let oracle = corpus::oracle_bundle(&input);
            let seal = seal::create_seal(&input, &oracle, &custody)?;
            let receipt = seal::create_build_receipt(
                "cargo build -p apeireth-runtime-assembly --example memory_longterm_eval",
                "desktop",
                &std::fs::read(std::env::current_exe()?)?,
            )?;
            // ⭐ The freeze binds the **exact** configuration of the run that
            // will consume the shard: the same --live switch and the same
            // environment must be in force when the shard is revealed.
            let config = if cli.live {
                live_config()?
            } else {
                Config::offline()
            };
            let config_sha256 = hash(&serde_json::to_string(&config)?);
            let freeze = seal::create_freeze(
                &seal,
                &receipt,
                &config_sha256,
                1,
                &hash(schema::READER_SYSTEM),
            );
            write_new(&custody.join("build-receipt.json"), &receipt)?;
            write_new(&custody.join("freeze.json"), &freeze)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "seal": seal,
                    "build_receipt": receipt,
                    "freeze": freeze,
                    "custody": custody.display().to_string(),
                    "custody_mode": "同账号文件隔离（不是强保密）",
                    "independent_custody": "未实现：独立保管需要另一名持有人或另一台机器，本轮未接线。",
                    "note": "封存链绑定本次配置（含端点与模型）；复核时需提供同样的 --live 开关与环境变量。",
                }))?
            );
            Ok(true)
        }
        "verify" => {
            let custody = cli.custody.ok_or("verify 需要 --custody DIR")?;
            let input = corpus::input_bundle();
            let oracle = corpus::oracle_bundle(&input);
            let seal = seal::read_seal(&custody)?;
            let receipt = seal::read_build_receipt(&custody)?;
            let freeze = seal::read_freeze(&custody)?;
            let config = if cli.live {
                live_config()?
            } else {
                Config::offline()
            };
            let config_sha256 = hash(&serde_json::to_string(&config)?);
            let outcome = seal::verify_chain(
                &input,
                &seal,
                &freeze,
                &receipt,
                &config_sha256,
                &std::fs::read(std::env::current_exe()?)?,
            );
            let revealed = seal::reveal_entries(&custody);
            // ⭐ The reveal proof is verified from the custody files alone.
            let reveal_proof = seal::verify_reveal(&custody, None);
            let verified = outcome.is_ok();
            let problems = outcome.err().map(|error| error.to_string());
            // ⭐ The reveal state is spelled out so that an outside reader is not
            // left guessing why the exit code is what it is:
            //   not_yet_revealed ⇒ the shard was never opened (nothing to verify),
            //   verified         ⇒ exactly one ledger entry, verified from the files,
            //   inconsistent     ⇒ the ledger exists but does not verify.
            let reveal_state = if reveal_proof.is_ok() {
                "verified"
            } else if revealed
                .iter()
                .filter(|entry| entry.seal_sha256 == seal.seal_sha256)
                .count()
                == 0
            {
                "not_yet_revealed"
            } else {
                "inconsistent"
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "verified": verified,
                    "problems": problems,
                    "seal_sha256": seal.seal_sha256,
                    "reveal_entries": revealed.len(),
                    "reveal_state": reveal_state,
                    "reveal_verified_from_files": reveal_proof.is_ok(),
                    "reveal_problem": reveal_proof.as_ref().err().map(|error| error.to_string()),
                    "oracle_sha256": oracle_sha256(&oracle),
                }))?
            );
            Ok(verified && reveal_proof.is_ok())
        }
        other => Err(format!("未知命令 {other}；用 --help 看用法").into()),
    }
}

fn oracle_sha256(oracle: &schema::OracleBundle) -> String {
    hash(&serde_json::to_string(oracle).unwrap_or_default())
}
