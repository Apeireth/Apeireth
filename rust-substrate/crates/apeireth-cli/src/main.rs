//! CLI — apeireth
//!
//! Subcommands:
//! - apeireth version
//! - apeireth identity hash <name> <purpose> <origin>
//! - apeireth episode new <actor> <content>
//! - apeireth note new <topic> <claim>
//! - apeireth benchmark insert-episodes --count 1000
//! - apeireth benchmark forget-sweep --count 10000
//! - apeireth benchmark reconsolidate --count 1000
//! - apeireth benchmark dispatcher --count N [--kind direct|http|file] [--async]
//!                                [--contexts M]
//!   (V30 async_dispatcher Rust port, 主 12:07 + 主 18:40)

use clap::{Parser, Subcommand};
use apeireth_core::{IdentityCard, Note, Episode, TaskKind};
use apeireth_core::episode::{Actor, EpisodeKind};
use apeireth_core::forget;
use std::time::Instant;

#[derive(Parser)]
#[command(name = "apeireth", about = "ASI 地基 — 主人 14:27 命名, 14:47 多语言混合, 核心 Rust")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Version,
    IdentityHash { name: String, purpose: String, origin: String },
    EpisodeNew { actor: String, content: String },
    NoteNew { topic: String, claim: String },
    Bench { #[command(subcommand)] bench: BenchCmd },
}

#[derive(Subcommand)]
enum BenchCmd {
    InsertEpisodes { #[arg(default_value = "1000")] count: usize },
    ForgetSweep { #[arg(default_value = "10000")] count: usize },
    Reconsolidate { #[arg(default_value = "1000")] count: usize },
    /// V30 async_dispatcher Rust port benchmark (主 12:07 + 主 18:40)
    Dispatcher {
        #[arg(default_value = "1000")]
        count: usize,
        /// task kind
        #[arg(long, value_parser = ["direct", "file", "custom"], default_value = "direct")]
        kind: String,
        /// 并发模式: spawn all 然后 await all (true) vs sequential (false)
        #[arg(long, default_value_t = false)]
        r#async: bool,
        /// 推 N 个 context 对象 (test push_context / purge_ttl_context)
        #[arg(long, default_value_t = 0)]
        contexts: usize,
        /// 注册 N 个 plugin (test register_plugin)
        #[arg(long, default_value_t = 0)]
        plugins: usize,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.cmd {
        Cmd::Version => {
            println!("apeireth 0.1.0");
            println!("主人 14:27 命名, 14:47 核心 Rust, 14:52 最高深度");
        }
        Cmd::IdentityHash { name, purpose, origin } => {
            let card = IdentityCard::new(name, purpose, origin);
            println!("integrity_hash: {}", card.integrity_hash());
        }
        Cmd::EpisodeNew { actor, content } => {
            let ep = Episode::new(Actor::from_str(&actor), content, "", EpisodeKind::Utterance, "", "stm");
            println!("eid: {}", ep.eid);
            println!("fingerprint: {}", ep.fingerprint);
        }
        Cmd::NoteNew { topic, claim } => {
            let n = Note::new(topic, claim, vec![], 0.5, 5, "stm");
            println!("nid: {}", n.nid);
        }
        Cmd::Bench { bench } => match bench {
            BenchCmd::InsertEpisodes { count } => {
                use apeireth_adapters::SqliteEpisodeRepository;
                use apeireth_ports::EpisodeRepository;
                let repo = SqliteEpisodeRepository::open("bench.db")?;
                let start = Instant::now();
                for i in 0..count {
                    let ep = Episode::new(
                        Actor::Master,
                        format!("bench episode {}", i),
                        "bench",
                        EpisodeKind::Utterance,
                        "bench",
                        "stm",
                    );
                    let _ = repo.append(&ep).await;
                }
                let elapsed = start.elapsed();
                println!("Inserted {} episodes in {:?}", count, elapsed);
                println!("  per-episode: {:?}", elapsed / count as u32);
            }
            BenchCmd::ForgetSweep { count } => {
                let mut notes: Vec<Note> = (0..count).map(|i| {
                    Note::new(
                        format!("topic {}", i),
                        format!("claim {}", i),
                        vec![],
                        0.5,
                        (i % 10) as u8,
                        "stm",
                    )
                }).collect();
                let start = Instant::now();
                let stats = forget::forget_sweep(&mut notes, 0.30);
                let elapsed = start.elapsed();
                println!("Forget sweep: scanned={} forgotten={} kept={} in {:?}", stats.scanned, stats.forgotten, stats.kept, elapsed);
            }
            BenchCmd::Reconsolidate { count } => {
                let mut card = IdentityCard::new("Apeireth", "p", "r");
                card.remember_forever.push("永恒".to_string());
                card.never_mention.push("私人".to_string());
                let mut notes: Vec<Note> = (0..count).map(|i| {
                    let topic = if i % 3 == 0 { "永恒主题".to_string() } else { format!("topic {}", i) };
                    Note::new(topic, format!("claim {}", i), vec![], 0.5, 5, "stm")
                }).collect();
                let start = Instant::now();
                let stats = apeireth_core::reconsolidate::reconsolidate(&mut notes, &card);
                let elapsed = start.elapsed();
                println!("Reconsolidate: boost={} flag={} align={} none={} in {:?}",
                    stats.boost.len(), stats.flag.len(), stats.align.len(), stats.none, elapsed);
            }
            BenchCmd::Dispatcher { count, kind, r#async, contexts, plugins } => {
                use apeireth_adapters::TokioDispatcher;
                use apeireth_ports::AsyncDispatcher;
                use apeireth_core::{PluginType, ContextType};

                let dispatcher = TokioDispatcher::new();

                // 注册 plugin
                for i in 0..plugins {
                    let pname = format!("plugin_{}", i);
                    dispatcher
                        .register_plugin(&pname, vec![PluginType::Async, PluginType::Sync])
                        .await
                        .ok();
                }

                // 推 context
                for i in 0..contexts {
                    dispatcher
                        .push_context(
                            if i % 2 == 0 {
                                ContextType::SyncUser
                            } else {
                                ContextType::AsyncUser
                            },
                            serde_json::json!({"i": i, "data": "ctx"}),
                            i % 3 == 0, // 部分 persistent
                            0,            // 全部 infinite — 测试 push + list + stats
                        )
                        .await
                        .ok();
                }

                // 准备 task kind 和 payload
                // ponytail: kind=file 用 tempdir 真文件 (测试 IO 路径);
                // kind=custom 显式失败 (测 n_failed 统计); 其他走 DirectCall.
                let (task_kind, payload) = match kind.as_str() {
                    "file" => {
                        let dir = tempfile::tempdir()?;
                        let path = dir.path().join("bench.txt");
                        std::fs::write(&path, "hello apeireth bench")?;
                        // 必须保留 dir 存活 — 用 leak 简化 bench 生命周期
                        let path_str = path.to_str().unwrap().to_string();
                        std::mem::forget(dir);
                        (
                            TaskKind::FileRead,
                            serde_json::json!({"path": path_str}),
                        )
                    }
                    "custom" => (TaskKind::Custom("bench".into()), serde_json::json!({})),
                    _ => (TaskKind::DirectCall, serde_json::json!({"i": 0})),
                };

                // 提交 N 个 pending task
                let submit_start = Instant::now();
                let mut ids = Vec::with_capacity(count);
                for i in 0..count {
                    let t = dispatcher
                        .submit_async_task(&format!("t_{}", i), task_kind.clone(), payload.clone())
                        .await?;
                    ids.push(t.task_id);
                }
                let submit_elapsed = submit_start.elapsed();

                // 执行
                let exec_start = Instant::now();
                for id in &ids {
                    dispatcher.execute_async_task(id).await?;
                }
                let exec_elapsed = exec_start.elapsed();

                // await
                let await_start = Instant::now();
                let mut n_success = 0usize;
                let mut n_failed = 0usize;
                let mut total_dur_ms = 0.0f64;
                if r#async {
                    // 并发 await — tokio::join! 风格
                    use futures::future::join_all;
                    let futs: Vec<_> = ids.iter().map(|id| dispatcher.await_task(id)).collect();
                    let results = join_all(futs).await;
                    for r in results {
                        match r {
                            Ok(t) => {
                                if t.status == apeireth_core::TaskStatus::Success {
                                    n_success += 1;
                                } else {
                                    n_failed += 1;
                                }
                                total_dur_ms += t.duration_ms;
                            }
                            Err(_) => n_failed += 1,
                        }
                    }
                } else {
                    for id in &ids {
                        match dispatcher.await_task(id).await {
                            Ok(t) => {
                                if t.status == apeireth_core::TaskStatus::Success {
                                    n_success += 1;
                                } else {
                                    n_failed += 1;
                                }
                                total_dur_ms += t.duration_ms;
                            }
                            Err(_) => n_failed += 1,
                        }
                    }
                }
                let await_elapsed = await_start.elapsed();

                let stats = dispatcher.stats().await?;

                println!("=== V30 async_dispatcher Rust port benchmark ===");
                println!("  count      : {}", count);
                println!("  kind       : {}", kind);
                println!("  async_mode : {}", r#async);
                println!("  plugins    : {}", plugins);
                println!("  contexts   : {}", contexts);
                println!("---");
                println!("  submit     : {:?} ({:.2} µs/task)",
                    submit_elapsed,
                    submit_elapsed.as_micros() as f64 / count.max(1) as f64);
                println!("  execute    : {:?} ({:.2} µs/task)",
                    exec_elapsed,
                    exec_elapsed.as_micros() as f64 / count.max(1) as f64);
                println!("  await      : {:?} ({:.2} µs/task)",
                    await_elapsed,
                    await_elapsed.as_micros() as f64 / count.max(1) as f64);
                println!("  throughput : {:.0} tasks/sec (await)", count as f64 / await_elapsed.as_secs_f64().max(1e-9));
                println!("---");
                println!("  n_success  : {}", n_success);
                println!("  n_failed   : {}", n_failed);
                println!("  avg task dur: {:.2} ms", if count > 0 { total_dur_ms / count as f64 } else { 0.0 });
                println!("  stats      : n_tasks={} n_context={} v3_guard={}",
                    stats.n_tasks, stats.n_context_objects, stats.v3_philosophy_guard);
                println!("==================================================");
            }
        }
    }
    Ok(())
}