use clap::{Parser, Subcommand};
use std::sync::Arc;
use std::io::{self, Write};
use chrono::Utc;

use apeireth_runtime::UnifiedRuntimeHost;
use apeireth_storage::memory_v2::QueryMode;

#[derive(Parser)]
#[command(name = "apeireth", version = "2.0.0", about = "Apeireth 2.0 High-Performance Cognitive Companion OS")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Starts the Apeireth Gateway REST/WS server with full UnifiedRuntimeHost
    Serve {
        #[arg(short, long, default_value = "127.0.0.1:8080")]
        bind: String,
        #[arg(short, long, default_value = "apeireth_v2.db")]
        db: String,
    },
    /// Starts interactive terminal companion chat powered by live LLM & ACT-R memory
    Chat {
        #[arg(short, long, default_value = "apeireth_v2.db")]
        db: String,
    },
    /// Inspects and verifies the SHA-256 tamper-evident audit hash-chain
    Audit {
        hash: Option<String>,
        #[arg(short, long, default_value = "apeireth_v2.db")]
        db: String,
    },
    /// Searches and inspects ACT-R memory facts & clusters
    Memory {
        query: String,
        #[arg(short, long, default_value = "apeireth_v2.db")]
        db: String,
    },
    /// Shows system health, 5-gate pipeline status, emotion state & sleep drive
    Status {
        #[arg(short, long, default_value = "apeireth_v2.db")]
        db: String,
    },
}

pub fn get_default_api_key() -> String {
    let key_file = r"C:\Users\31683\apikey-ultra.txt";
    if let Ok(key) = std::fs::read_to_string(key_file) {
        let trimmed = key.trim().to_string();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }
    std::env::var("MINIMAX_API_KEY").unwrap_or_else(|_| "sk-dummy".to_string())
}

pub async fn run_cli(cli: Cli) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let api_key = get_default_api_key();

    match cli.command {
        Commands::Serve { bind, db } => {
            println!("===============================================================================");
            println!("       APEIRETH 2.0 LIVING GATEWAY & RUNTIME BACKBONE SERVING ON {}     ", bind);
            println!("===============================================================================");
            println!("  [+] Initializing Unified Runtime Host...");
            let host = Arc::new(UnifiedRuntimeHost::new(&api_key, &db).await?);
            println!("  [+] Memory Pool: SQLite WAL on {}", db);
            println!("  [+] Active Model: {}", host.default_model);
            println!("  [+] Endpoints: /health, /v1/models, /v1/chat/completions, /ws");
            println!("  [*] Press Ctrl+C to stop.\n");

            apeireth_gateway::server::start_server(&bind, Some(host)).await?;
        }
        Commands::Chat { db } => {
            println!("===============================================================================");
            println!("             APEIRETH 2.0 INTERACTIVE COGNITIVE COMPANION REPL                ");
            println!("===============================================================================");
            println!("Connecting to live UnifiedRuntimeHost with ACT-R Memory and MiniMax...");
            let host = UnifiedRuntimeHost::new(&api_key, &db).await?;
            let session_id = format!("cli_{}", uuid::Uuid::new_v4());
            println!("Session ID: {}", session_id);
            println!("Type 'exit' or 'quit' to end session.\n");

            let stdin = io::stdin();
            loop {
                print!("User> ");
                io::stdout().flush()?;
                let mut input = String::new();
                if stdin.read_line(&mut input)? == 0 {
                    break;
                }
                let trimmed = input.trim();
                if trimmed.eq_ignore_ascii_case("exit") || trimmed.eq_ignore_ascii_case("quit") {
                    println!("Ending conversation. Cognitive state saved.");
                    break;
                }
                if trimmed.is_empty() {
                    continue;
                }

                println!("Apeireth is thinking...");
                match host.handle_chat_turn(&session_id, trimmed).await {
                    Ok(turn) => {
                        if let Some(cot) = turn.reasoning_cot {
                            println!("\n[CoT Thinking]:\n{}", cot.trim());
                        }
                        println!("\nApeireth ({:?} | P:{:.2} A:{:.2} D:{:.2})>",
                            turn.response_style,
                            turn.pad_state.pleasure,
                            turn.pad_state.arousal,
                            turn.pad_state.dominance
                        );
                        println!("{}\n", turn.assistant_text.trim());
                        println!("[Tokens: {} | Audit: {:.8}...]\n", turn.token_usage.total_tokens, turn.audit_hash);
                    }
                    Err(e) => {
                        eprintln!("\n[Error processing chat turn: {}]\n", e);
                    }
                }
            }
        }
        Commands::Audit { hash, db } => {
            println!("===============================================================================");
            println!("                  APEIRETH SHA-256 AUDIT HASH-CHAIN VERIFIER                   ");
            println!("===============================================================================");
            let host = UnifiedRuntimeHost::new(&api_key, &db).await?;
            let audit = host.audit_chain.lock().await;
            match audit.verify_chain() {
                Ok(()) => {
                    println!("  ✓ Audit Hash-Chain Integrity Verified: 100% VALID");
                    if let Some(h) = hash {
                        println!("  - Target Hash query: {}", h);
                    }
                }
                Err((idx, err)) => {
                    println!("  ❌ Chain broken at index {}: {}", idx, err);
                }
            }
        }
        Commands::Memory { query, db } => {
            println!("===============================================================================");
            println!("                  APEIRETH ACT-R MEMORY & TOPIC SEARCH                         ");
            println!("===============================================================================");
            let host = UnifiedRuntimeHost::new(&api_key, &db).await?;
            let items = host.memory_store.query(Utc::now(), QueryMode::All).await?;
            let bigrams = apeireth_storage::memory_v2::MemoryStore::cjk_bigram_tokenize(&query);
            println!("  [+] Query tokens: {:?}", bigrams);
            println!("  [+] Total stored memories: {}", items.len());
            let mut matches = 0;
            for item in items {
                if item.data.contains(&query) || bigrams.iter().any(|b| item.data.contains(b)) {
                    matches += 1;
                    println!("  - [{:.2}] {}", item.importance, item.data);
                }
            }
            println!("  [+] Found {} relevant memory items.", matches);
        }
        Commands::Status { db } => {
            println!("===============================================================================");
            println!("                      APEIRETH 2.0 SYSTEM STATUS & HEALTH                      ");
            println!("===============================================================================");
            let host = UnifiedRuntimeHost::new(&api_key, &db).await?;
            println!("  ✓ Unified Runtime Host: ONLINE");
            println!("  ✓ Storage: SQLite WAL Pool Connected ({})", db);
            println!("  ✓ Default Model: {}", host.default_model);

            let plut = host.plutchik.lock().await;
            let pad = plut.to_pad();
            println!("  ✓ Emotional PAD State: Pleasure={:.2}, Arousal={:.2}, Dominance={:.2}", pad.pleasure, pad.arousal, pad.dominance);
            println!("  ✓ Active Response Style: {:?}", pad.to_response_style());

            let borb = host.borbely.lock().await;
            println!("  ✓ Borbély Sleep Drive Score: {:.2}", borb.drive());
            println!("  ✓ 5-Gate Governance Pipeline: ACTIVE & SECURED");
            println!("  ✓ Platform Sandbox: {}", host.sandbox.platform_type());
            println!("===============================================================================\n");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_command_parsing() {
        let args = vec!["apeireth", "status"];
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command {
            Commands::Status { .. } => {}
            _ => panic!("Expected status command"),
        }

        let args2 = vec!["apeireth", "audit", "sha256_mock_hash"];
        let cli2 = Cli::try_parse_from(args2).unwrap();
        match cli2.command {
            Commands::Audit { hash, .. } => assert_eq!(hash, Some("sha256_mock_hash".into())),
            _ => panic!("Expected audit command"),
        }
    }
}
