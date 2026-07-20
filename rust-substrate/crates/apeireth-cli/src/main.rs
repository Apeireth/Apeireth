//! CLI — apeireth
//!
//! Subcommands:
//! - apeireth version
//! - apeireth identity hash <name> <purpose> <origin>
//! - apeireth episode new <actor> <content>
//! - apeireth note new <topic> <claim>
//! - apeireth benchmark insert-episodes --count 1000
//! - apeireth benchmark forget-sweep --count 10000

use clap::{Parser, Subcommand};
use apeireth_core::{IdentityCard, Note, Episode, Actor, EpisodeKind};
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
        }
    }
    Ok(())
}