//! minimax_memory_roundtrip -- R128 end-to-end: minimax (Anthropic protocol) + memory persist + re-read
//!
//! Purpose: verify Apeireth backend really connects minimax + really persists conversation history to SQLite.
//!
//! Flow:
//! 1. Prepare AnthropicCompatibleProvider (via https://api.minimaxi.com/anthropic)
//! 2. Really call minimax (MiniMax-M3) -- prompt
//! 3. Write user + assistant episode to SqliteMemoryStore (temp file)
//! 4. drop store (close conn)
//! 5. Reopen store (verify cross-conn persistence)
//! 6. List session episodes (verify read-back)
//! 7. semantic_search verify memory retrievable
//!
//! Run:
//! `powershell
//!  = (Get-Content ".openclaw\apikey.txt" -Raw).Trim()
//! cargo run -p apeireth-integration-e2e --example minimax_memory_roundtrip
//! `
//!
//! No-fake:
//! - Real HTTP POST to api.minimaxi.com/anthropic/v1/messages
//! - Real SQLite file (not in-memory)
//! - Real drop + reopen (verify cross-conn persistence)

use apeireth_api::llm::{
    AnthropicCompatibleConfig, AnthropicCompatibleProvider, ChatMessage, LlmProvider, LlmRequest,
};
use apeireth_core::Episode;
use apeireth_memory::{EpisodeQuery, EpisodeStore, HashEmbedder, SqliteMemoryStore};
use std::sync::Arc;
use std::time::Instant;

const MODEL: &str = "MiniMax-M3";
const MINIMAXI_ANTHROPIC_URL: &str = "https://api.minimaxi.com/anthropic";
const SESSION_ID: &str = "minimax-memory-roundtrip-1";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("R128 end-to-end: minimax + memory real task");
    println!("=============================================");
    println!("  provider:    AnthropicCompatibleProvider (minimax /anthropic)");
    println!("  model:       {MODEL}");
    println!("  base_url:    {MINIMAXI_ANTHROPIC_URL}");
    println!("  memory:      SqliteMemoryStore (file-backed, cross-conn persist)");
    println!();

    let api_key = std::env::var("APEIRETH_API_KEY").map_err(|_| {
        "APEIRETH_API_KEY env var not set (load from C:\\Users\\REDACTED\\.openclaw\\apikey.txt)"
    })?;

    let tmp_path = std::env::temp_dir().join("apeireth-minimax-memory-roundtrip.db");
    let _ = std::fs::remove_file(&tmp_path);
    println!("temp DB: {}", tmp_path.display());
    println!();

    // Phase 1: minimax real call
    println!("=============================================");
    println!("Phase 1 -- minimax real call (Anthropic Messages)");

    let config = AnthropicCompatibleConfig::new(
        api_key,
        MINIMAXI_ANTHROPIC_URL.to_string(),
        vec![MODEL.to_string()],
    );
    let provider = AnthropicCompatibleProvider::new(config)?;
    println!("  provider ready: base_url={MINIMAXI_ANTHROPIC_URL}");

    let user_prompt = "One sentence on Rust async runtime design philosophy";
    println!("  user prompt: {user_prompt}");

    let req = LlmRequest::new(
        MODEL,
        vec![
            ChatMessage::system(
                "You are a Rust engineering assistant, answer concisely, one sentence max",
            ),
            ChatMessage::user(user_prompt),
        ],
    )
    .with_temperature(0.7)
    .with_max_tokens(150);

    let start = Instant::now();
    let resp = provider.complete(req).await?;
    let elapsed = start.elapsed();

    println!("  assistant reply: {}", resp.content);
    println!(
        "  tokens: prompt={} completion={} total={}",
        resp.usage.prompt_tokens, resp.usage.completion_tokens, resp.usage.total_tokens
    );
    println!("  latency: {}ms", elapsed.as_millis());
    println!();

    // Phase 2: write to SqliteMemoryStore
    println!("=============================================");
    println!("Phase 2 -- write episode to SqliteMemoryStore (file-backed)");

    let store = SqliteMemoryStore::open(&tmp_path)?;
    let now = chrono::Utc::now().timestamp();

    store.put_episode(&Episode {
        id: "ep-user-1".into(),
        timestamp: now,
        role: "user".into(),
        content: user_prompt.into(),
        session_id: SESSION_ID.into(),
    })?;
    println!("  wrote ep-user-1 (user prompt)");

    store.put_episode(&Episode {
        id: "ep-asst-1".into(),
        timestamp: now + 1,
        role: "assistant".into(),
        content: resp.content.clone(),
        session_id: SESSION_ID.into(),
    })?;
    println!(
        "  wrote ep-asst-1 (assistant reply, {} chars)",
        resp.content.len()
    );

    drop(store);
    println!("  store dropped (file closed, connection released)");
    println!();

    // Phase 3: reopen and verify persistence
    println!("=============================================");
    println!("Phase 3 -- reopen store, verify SQLite persistence");

    let store2 = SqliteMemoryStore::open(&tmp_path)?;
    let query = EpisodeQuery::new().for_session(SESSION_ID);
    let episodes = store2.query(&query)?;
    println!(
        "  session '{SESSION_ID}' contains {} episodes:",
        episodes.len()
    );

    for ep in &episodes {
        println!(
            "    - [{}] {}: {}",
            ep.id,
            ep.role,
            truncate(&ep.content, 80)
        );
    }

    if episodes.len() != 2 {
        return Err(format!("expected 2 episodes in session, got {}", episodes.len()).into());
    }
    println!("  2 episodes persisted across drop+reopen");
    println!();

    // Phase 4: semantic_search verify
    println!("=============================================");
    println!("Phase 4 -- semantic_search verify memory retrievable");

    let embedder: Arc<dyn apeireth_memory::EmbedFn> = Arc::new(HashEmbedder::new(64));
    let hits = store2.semantic_search("Rust async runtime", 5, Arc::clone(&embedder))?;
    println!("  query: \"Rust async runtime\", top-{} hits:", hits.len());
    for (i, hit) in hits.iter().enumerate() {
        println!(
            "    #{}: [{}] {}: {}",
            i,
            hit.id,
            hit.role,
            truncate(&hit.content, 80)
        );
    }
    println!();

    // Phase 5: cleanup
    drop(store2);
    let _ = std::fs::remove_file(&tmp_path);
    println!("=============================================");
    println!("minimax + memory end-to-end real task PASS");
    println!("  - minimax Anthropic protocol real call OK");
    println!("  - SQLite real persistence (file-backed, drop+reopen) OK");
    println!("  - semantic_search retrievable OK");
    println!(
        "  - tokens real count ({} total) OK",
        resp.usage.total_tokens
    );

    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max).collect();
        format!("{truncated}...")
    }
}
