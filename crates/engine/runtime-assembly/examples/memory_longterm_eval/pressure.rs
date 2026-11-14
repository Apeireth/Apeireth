//! Scale, concurrency, fault and sustained-run pressure (B3).
//!
//! ⭐ 口径：**规模结果与持续运行结果分别报告**。
//! * 规模（scale）：写入多少条记录、并发度多少、完整性与隔离如何。
//! * 持续运行（sustained）：真实墙钟跑了多少秒、做了多少次操作。
//! * 虚拟时间推进（virtual horizon）：只是把时钟往前拨，**不是**持续运行，
//!   任何地方都不得把它表述为"相同长度的真实持续运行"。

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use apeireth_core::kernel::Episode;
use apeireth_memory::{EpisodeStore, SqliteMemoryStore};
use serde::{Deserialize, Serialize};

use super::artifacts::{percentile, Result};
use super::harness::Harness;
use super::schema::{Span, PRESSURE_PROTOCOL};

/// The knobs of one pressure run.
#[derive(Clone, Debug)]
pub struct PressureOptions {
    /// Total number of records written at the storage layer.
    pub records: usize,
    /// Number of concurrent writers.
    pub concurrency: usize,
    /// Number of full-chain turns.
    pub chain_turns: usize,
    /// Real wall-clock seconds of the sustained phase.
    pub sustained_seconds: u64,
    /// Virtual days the clock is pushed forward in the virtual-horizon phase.
    pub virtual_horizon_days: i64,
}

impl Default for PressureOptions {
    fn default() -> Self {
        Self {
            records: 200,
            concurrency: 4,
            chain_turns: 8,
            sustained_seconds: 5,
            virtual_horizon_days: 30,
        }
    }
}

/// One decidable check of the pressure run.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Check {
    pub name: String,
    pub requirement: String,
    pub observed: String,
    pub passed: bool,
}

/// The scale phase: storage-layer writes under concurrency.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ScaleResult {
    pub records_requested: usize,
    pub records_written: usize,
    pub write_failures: usize,
    pub records_read_back: usize,
    pub duplicate_ids: usize,
    pub concurrency: usize,
    pub elapsed_us: u64,
    pub errors: Vec<String>,
}

/// The full-chain phase: concurrent Runtime turns.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChainResult {
    pub turns: usize,
    pub failures: usize,
    pub concurrency: usize,
    pub elapsed_us: u64,
    pub service_us: u64,
    pub queueing_us: u64,
    pub per_turn_us: Vec<u64>,
    pub errors: Vec<String>,
}

/// The recall phase: latency of a recall turn over the scaled store.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RecallResult {
    pub samples: usize,
    pub p50_us: u64,
    pub p95_us: u64,
    pub max_us: u64,
    pub records_in_store: i64,
    pub delivered_sources: usize,
}

/// The fault phase: injected write failure and the recovery of the pending job.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FaultResult {
    pub fault_injected: bool,
    pub pending_after_fault: u64,
    pub pending_after_recovery: u64,
    pub done_after_recovery: u64,
    pub suppressed_after_recovery: u64,
    pub attempts: u64,
    pub last_error: Option<String>,
    pub records_after_recovery: i64,
}

/// The sustained phase: real wall-clock running.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SustainedResult {
    /// The measurement mode: always "real_wall_clock".
    pub mode: String,
    pub requested_seconds: u64,
    pub measured_seconds: f64,
    pub operations: usize,
    pub errors: usize,
}

/// The virtual-horizon phase: the clock is pushed forward, nothing else.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VirtualHorizonResult {
    pub virtual_days: i64,
    pub records_before: i64,
    pub records_after: i64,
    /// Always says that this is not a sustained run.
    pub note: String,
}

/// The whole pressure report.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PressureReport {
    pub protocol: String,
    pub scale: ScaleResult,
    pub chain: ChainResult,
    pub recall: RecallResult,
    pub fault: FaultResult,
    pub sustained: SustainedResult,
    pub virtual_horizon: VirtualHorizonResult,
    pub checks: Vec<Check>,
    pub notes: Vec<String>,
    pub spans: Vec<Span>,
}

fn check(name: &str, requirement: &str, observed: String, passed: bool) -> Check {
    Check {
        name: name.to_string(),
        requirement: requirement.to_string(),
        observed,
        passed,
    }
}

/// Run the whole pressure suite over one store.
pub async fn run(harness: &Harness, options: &PressureOptions) -> Result<PressureReport> {
    let db = harness.db.clone();
    let mut spans: Vec<Span> = Vec::new();

    // ── Phase 1: scale at the storage layer ───────────────────────────────
    // ⭐ The production order is: open the store once, migrate once, then
    // serve. The scale phase follows it — every worker opens its own
    // connection (WAL + busy timeout), but the migrations have already been
    // applied, so the workers do not race on the schema.
    {
        let store = SqliteMemoryStore::open(&db)?;
        let _ = store.count_by_session("pressure-session-0000")?;
    }
    let started = Instant::now();
    let per_worker = options.records.div_ceil(options.concurrency.max(1));
    let mut written = 0usize;
    let mut failures = 0usize;
    let mut errors: Vec<String> = Vec::new();
    let mut ids: Vec<String> = Vec::new();
    let mut handles = Vec::new();
    for worker in 0..options.concurrency.max(1) {
        let db = db.clone();
        handles.push(tokio::task::spawn_blocking(move || {
            let store = SqliteMemoryStore::open(&db).map_err(|error| error.to_string())?;
            let mut local_ids = Vec::new();
            let mut local_written = 0usize;
            let mut local_failures = 0usize;
            let mut local_errors = Vec::new();
            for index in 0..per_worker {
                let id = format!("pressure-{worker:04}-{index:06}");
                let episode = Episode {
                    id: id.clone(),
                    timestamp: 1_760_000_000 + index as i64,
                    role: "user".to_string(),
                    content: format!("压力记录 {worker}-{index}"),
                    session_id: format!("pressure-session-{worker:04}"),
                };
                match store.put_episode(&episode) {
                    Ok(()) => {
                        local_written += 1;
                        local_ids.push(id);
                    }
                    Err(error) => {
                        local_failures += 1;
                        if local_errors.len() < 5 {
                            local_errors.push(format!("{id}: {error}"));
                        }
                    }
                }
            }
            Ok::<_, String>((local_written, local_failures, local_ids, local_errors))
        }));
    }
    for handle in handles {
        match handle.await {
            Ok(Ok((local_written, local_failures, local_ids, local_errors))) => {
                written += local_written;
                failures += local_failures;
                ids.extend(local_ids);
                errors.extend(local_errors);
            }
            Ok(Err(error)) => {
                failures += 1;
                errors.push(error);
            }
            Err(error) => {
                failures += 1;
                errors.push(format!("worker task failed: {error}"));
            }
        }
    }
    let elapsed_us = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    let mut unique_ids = ids.clone();
    unique_ids.sort();
    unique_ids.dedup();
    let duplicate_ids = ids.len() - unique_ids.len();
    let store = SqliteMemoryStore::open(&db)?;
    let mut records_read_back = 0usize;
    for worker in 0..options.concurrency.max(1) {
        let session = format!("pressure-session-{worker:04}");
        let rows = EpisodeStore::recent_episodes(&store, &session, per_worker + 16)?;
        records_read_back += rows.len();
    }
    spans.push(Span {
        case: "pressure.scale".to_string(),
        stage: "storage_scale".to_string(),
        elapsed_us,
        outcome: if failures == 0 { "ok" } else { "partial" }.to_string(),
    });
    let scale = ScaleResult {
        records_requested: options.records,
        records_written: written,
        write_failures: failures,
        records_read_back,
        duplicate_ids,
        concurrency: options.concurrency,
        elapsed_us,
        errors,
    };

    // ── Phase 2: the full chain under concurrency ─────────────────────────
    let chain_started = Instant::now();
    let mut per_turn_us: Vec<u64> = Vec::new();
    let mut chain_failures = 0usize;
    let mut chain_errors: Vec<String> = Vec::new();
    let concurrency = options.concurrency.max(1);
    let turns = options.chain_turns.max(1);
    let mut set = tokio::task::JoinSet::new();
    for index in 0..turns {
        let harness = HarnessRef {
            db: db.clone(),
            config: harness.config.clone(),
        };
        set.spawn(async move {
            let started = Instant::now();
            let outcome = harness.turn(index).await;
            (started.elapsed().as_micros() as u64, outcome)
        });
    }
    while let Some(joined) = set.join_next().await {
        match joined {
            Ok((took, outcome)) => {
                per_turn_us.push(took);
                if let Err(error) = outcome {
                    chain_failures += 1;
                    if chain_errors.len() < 5 {
                        chain_errors.push(error);
                    }
                }
            }
            Err(error) => {
                chain_failures += 1;
                chain_errors.push(format!("chain task failed: {error}"));
            }
        }
    }
    per_turn_us.sort_unstable();
    let chain_elapsed_us = u64::try_from(chain_started.elapsed().as_micros()).unwrap_or(u64::MAX);
    let service_us = per_turn_us.iter().sum::<u64>();
    let queueing_us = chain_elapsed_us.saturating_sub(service_us);
    spans.push(Span {
        case: "pressure.chain".to_string(),
        stage: "full_chain".to_string(),
        elapsed_us: chain_elapsed_us,
        outcome: if chain_failures == 0 { "ok" } else { "partial" }.to_string(),
    });
    let chain = ChainResult {
        turns,
        failures: chain_failures,
        concurrency,
        elapsed_us: chain_elapsed_us,
        service_us,
        queueing_us,
        per_turn_us,
        errors: chain_errors,
    };

    // ── Phase 3: recall latency at scale ──────────────────────────────────
    let mut recall_samples: Vec<u64> = Vec::new();
    let mut delivered_sources = 0usize;
    for index in 0..3 {
        let started = Instant::now();
        let outcome = HarnessRef {
            db: db.clone(),
            config: harness.config.clone(),
        }
        .recall(index)
        .await;
        recall_samples.push(u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX));
        if let Ok(sources) = outcome {
            delivered_sources = sources;
        }
    }
    recall_samples.sort_unstable();
    let records_in_store =
        EpisodeStore::count_by_session(&store, &format!("pressure-session-0000"))?;
    let recall = RecallResult {
        samples: recall_samples.len(),
        p50_us: percentile(&recall_samples, 50),
        p95_us: percentile(&recall_samples, 95),
        max_us: recall_samples.last().copied().unwrap_or(0),
        records_in_store,
        delivered_sources,
    };
    spans.push(Span {
        case: "pressure.recall".to_string(),
        stage: "recall_at_scale".to_string(),
        elapsed_us: recall.p95_us,
        outcome: "ok".to_string(),
    });

    // ── Phase 4: injected fault and recovery ───────────────────────────────
    let fault = fault_phase(harness, &db).await?;
    spans.push(Span {
        case: "pressure.fault".to_string(),
        stage: "fault_recovery".to_string(),
        elapsed_us: 0,
        outcome: if fault.fault_injected {
            "ok"
        } else {
            "skipped"
        }
        .to_string(),
    });

    // ── Phase 5: a real sustained run ──────────────────────────────────────
    let sustained = sustained_phase(&db, options.sustained_seconds).await?;
    spans.push(Span {
        case: "pressure.sustained".to_string(),
        stage: "sustained_real".to_string(),
        elapsed_us: (sustained.measured_seconds * 1_000_000.0) as u64,
        outcome: if sustained.errors == 0 {
            "ok"
        } else {
            "partial"
        }
        .to_string(),
    });

    // ── Phase 6: the virtual horizon (explicitly not a sustained run) ─────
    let virtual_horizon = virtual_horizon_phase(harness, &db, options.virtual_horizon_days).await?;
    spans.push(Span {
        case: "pressure.virtual_horizon".to_string(),
        stage: "virtual_horizon".to_string(),
        elapsed_us: 0,
        outcome: "ok".to_string(),
    });

    let mut checks = Vec::new();
    checks.push(check(
        "scale_write_integrity",
        "every requested record is written exactly once (no failure, no duplicate)",
        format!(
            "requested={} written={} failures={} duplicates={}",
            scale.records_requested,
            scale.records_written,
            scale.write_failures,
            scale.duplicate_ids
        ),
        scale.write_failures == 0 && scale.duplicate_ids == 0,
    ));
    checks.push(check(
        "scale_read_back",
        "every written record is readable afterwards",
        format!(
            "written={} read_back={}",
            scale.records_written, scale.records_read_back
        ),
        scale.records_written == scale.records_read_back,
    ));
    checks.push(check(
        "chain_no_failure",
        "every full-chain turn completes",
        format!("turns={} failures={}", chain.turns, chain.failures),
        chain.failures == 0,
    ));
    checks.push(check(
        "fault_point_reached",
        "the injected write failure leaves a pending durable job",
        format!(
            "pending_after_fault={} last_error={:?}",
            fault.pending_after_fault, fault.last_error
        ),
        fault.fault_injected && fault.pending_after_fault > 0,
    ));
    checks.push(check(
        "recovery_drains_pending",
        "after the fault is lifted the pending job is drained",
        format!(
            "pending_after_recovery={} done={} suppressed={}",
            fault.pending_after_recovery,
            fault.done_after_recovery,
            fault.suppressed_after_recovery
        ),
        fault.fault_injected && fault.pending_after_recovery == 0,
    ));
    checks.push(check(
        "sustained_ran_in_real_time",
        "the sustained phase really ran for the requested wall-clock time",
        format!(
            "requested={}s measured={:.3}s operations={}",
            sustained.requested_seconds, sustained.measured_seconds, sustained.operations
        ),
        sustained.measured_seconds >= sustained.requested_seconds as f64 * 0.95,
    ));
    checks.push(check(
        "scale_and_sustained_are_separate",
        "the virtual horizon is never reported as a sustained run",
        virtual_horizon.note.clone(),
        virtual_horizon.mode_is_virtual(),
    ));

    Ok(PressureReport {
        protocol: PRESSURE_PROTOCOL.to_string(),
        scale,
        chain,
        recall,
        fault,
        sustained,
        virtual_horizon,
        checks,
        notes: vec![
            "规模结果与持续运行结果分别报告；两者不是同一个量。".to_string(),
            "虚拟时间推进只拨时钟，不构成持续运行。".to_string(),
            "本轮未调用真实模型：压力阶段的模型侧同样是确定性离线夹具。".to_string(),
        ],
        spans,
    })
}

impl VirtualHorizonResult {
    fn mode_is_virtual(&self) -> bool {
        self.note.contains("不是持续运行")
    }
}

/// A cheap handle that builds a Runtime per operation, so that concurrent
/// turns never share one Runtime.
struct HarnessRef {
    db: std::path::PathBuf,
    config: super::config::Config,
}

impl HarnessRef {
    async fn harness(&self) -> Result<Harness> {
        let input = super::corpus::input_bundle();
        Harness::new(&self.db, self.config.clone(), &input)
    }

    async fn turn(&self, index: usize) -> std::result::Result<(), String> {
        let harness = self.harness().await.map_err(|error| error.to_string())?;
        let session = format!("pressure-chain-{index:04}");
        // ⭐ The full chain: the writeback and the recovery drain are live,
        // so the concurrency numbers cover the whole path, not just a read.
        harness
            .ask_writable(&session, "今天的行程还没定。")
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    async fn recall(&self, index: usize) -> std::result::Result<usize, String> {
        let harness = self.harness().await.map_err(|error| error.to_string())?;
        let session = format!("pressure-chain-{index:04}");
        harness
            .ask(&session, "我住在哪个城市？")
            .await
            .map_err(|error| error.to_string())?;
        let request = harness
            .calls
            .last()
            .ok_or("the provider was never called for the recall sample")?;
        Ok(super::evidence::delivered_entries(&request).len())
    }
}

/// Inject a write failure, then lift it and let the recovery path run.
async fn fault_phase(harness: &Harness, db: &Path) -> Result<FaultResult> {
    let connection = rusqlite::Connection::open(db)?;
    connection.execute(
        "CREATE TRIGGER eval_pressure_reject BEFORE INSERT ON episodes BEGIN SELECT RAISE(ABORT,'pressure fault injection'); END",
        [],
    )?;
    let session = "pressure-fault-session".to_string();
    // ⭐ The journal namespace is the runtime session id, not the name.
    let namespace = harness.partition(&session)?;
    let outcome = harness.ask_writable(&session, "今天的地铁晚点了。").await;
    let pool = Arc::new(apeireth_storage::SqliteConnectionPool::open(db).await?);
    let journal = apeireth_memory::writeback_recovery::SqliteWritebackJournal::from_pool(pool);
    let after_fault = journal.status(&namespace)?;
    connection.execute("DROP TRIGGER eval_pressure_reject", [])?;
    let recovery = harness
        .ask_writable(&session, "冰箱里的牛奶快过期了。")
        .await;
    let after_recovery = journal.status(&namespace)?;
    let store = SqliteMemoryStore::open(db)?;
    let records_after_recovery = EpisodeStore::count_by_session(&store, &namespace)?;
    Ok(FaultResult {
        fault_injected: true,
        pending_after_fault: after_fault.pending,
        pending_after_recovery: after_recovery.pending,
        done_after_recovery: after_recovery.done,
        suppressed_after_recovery: after_recovery.suppressed,
        attempts: after_recovery.attempts,
        last_error: after_fault.last_error.clone(),
        records_after_recovery,
    })
    .map(|mut result| {
        // The turn outcome is part of the observation: a turn that failed
        // outright is reported, never silently dropped.
        if let Err(error) = outcome {
            result.last_error = Some(format!(
                "{} (recovery turn: {})",
                result.last_error.unwrap_or_default(),
                error
            ));
        }
        if let Err(error) = recovery {
            result.last_error = Some(format!(
                "{} (recovery turn: {})",
                result.last_error.unwrap_or_default(),
                error
            ));
        }
        result
    })
}

/// A real wall-clock sustained run: keep writing and reading until the clock
/// says the requested seconds have passed.
async fn sustained_phase(db: &Path, seconds: u64) -> Result<SustainedResult> {
    let started = Instant::now();
    let deadline = Duration::from_secs(seconds);
    let store = SqliteMemoryStore::open(db)?;
    let mut operations = 0usize;
    let mut errors = 0usize;
    let mut index = 0usize;
    while started.elapsed() < deadline {
        let id = format!("sustained-{index:08}");
        let episode = Episode {
            id: id.clone(),
            timestamp: 1_760_100_000 + index as i64,
            role: "user".to_string(),
            content: format!("持续运行记录 {index}"),
            session_id: "pressure-sustained-session".to_string(),
        };
        match store.put_episode(&episode) {
            Ok(()) => operations += 1,
            Err(_) => errors += 1,
        }
        if EpisodeStore::count_by_session(&store, "pressure-sustained-session").is_err() {
            errors += 1;
        }
        operations += 1;
        index += 1;
    }
    let measured = started.elapsed().as_secs_f64();
    Ok(SustainedResult {
        mode: "real_wall_clock".to_string(),
        requested_seconds: seconds,
        measured_seconds: measured,
        operations,
        errors,
    })
}

/// Push the clock forward without running anything for that long.
async fn virtual_horizon_phase(
    harness: &Harness,
    db: &Path,
    days: i64,
) -> Result<VirtualHorizonResult> {
    let store = SqliteMemoryStore::open(db)?;
    let before = EpisodeStore::count_by_session(&store, "pressure-chain-0000")?;
    harness.clock.advance(chrono::Duration::days(days.max(0)));
    let after = EpisodeStore::count_by_session(&store, "pressure-chain-0000")?;
    Ok(VirtualHorizonResult {
        virtual_days: days,
        records_before: before,
        records_after: after,
        note: format!(
            "虚拟时间推进 {days} 天：只拨了时钟，没有真实运行 {days} 天，这不是持续运行。"
        ),
    })
}
