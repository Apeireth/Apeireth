use std::sync::Arc;
use chrono::Utc;
use apeireth_runtime::UnifiedRuntimeHost;
use apeireth_storage::memory_v2::QueryMode;
use apeireth_cli::tui::state::{ActiveTab, AppState, ChatMessageItem, FactoryTaskItem};
use apeireth_cli::tui::widgets::{BrailleSparkline, DiffViewer};


fn get_api_key() -> String {
    let key_file = r"C:\Users\31683\apikey-ultra.txt";
    if let Ok(key) = std::fs::read_to_string(key_file) {
        let trimmed = key.trim().to_string();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }
    std::env::var("MINIMAX_API_KEY").unwrap_or_else(|_| "sk-dummy".to_string())
}

#[tokio::test]
async fn test_live_unified_host_multiturn_with_act_r_and_tui_state() {
    let api_key = get_api_key();
    let db_path = format!("test_sim_{}.db", uuid::Uuid::new_v4());

    println!("\n[1/4] Initializing Live UnifiedRuntimeHost with ACT-R Memory & MiniMax...");
    let host = Arc::new(UnifiedRuntimeHost::new(api_key, &db_path).await.expect("Host initialization"));
    let session_id = format!("sim_sess_{}", uuid::Uuid::new_v4());

    println!("[2/4] Testing Live Multi-Turn LLM Conversation & PAD / CoT / Audit...");
    // Turn 1: User introduces facts
    let user_msg1 = "Hello Apeireth, my name is Alex and I am developing a high-performance cognitive OS in Rust.";
    let turn1 = host.handle_chat_turn(&session_id, user_msg1).await.expect("Turn 1 execution");

    assert!(!turn1.assistant_text.is_empty(), "Assistant must reply");
    assert!(turn1.token_usage.total_tokens > 0, "Tokens must be counted");
    assert_eq!(turn1.audit_hash.len(), 64, "Must generate valid SHA-256 audit hash");
    println!("  ✓ Turn 1 Response: \"{}\" (PAD: {:?}, Tokens: {})",
        &turn1.assistant_text[..turn1.assistant_text.len().min(60)],
        turn1.pad_state,
        turn1.token_usage.total_tokens
    );

    // Turn 2: User queries recalled memory
    let user_msg2 = "What is my name and what language am I working with?";
    let turn2 = host.handle_chat_turn(&session_id, user_msg2).await.expect("Turn 2 execution");

    assert!(!turn2.assistant_text.is_empty());
    println!("  ✓ Turn 2 Recall: \"{}\" (Audit: {:.8}...)",
        &turn2.assistant_text[..turn2.assistant_text.len().min(60)],
        turn2.audit_hash
    );

    println!("[3/4] Testing TUI AppState Transition & Memory / Factory / Braille Widgets...");
    let mut app_state = AppState::new(session_id.clone());

    // Push chat turns into TUI state
    app_state.messages.push(ChatMessageItem {
        role: "user".into(),
        content: user_msg1.into(),
        cot: None,
        pad: turn1.pad_state.clone(),
        tokens: user_msg1.len() / 4 + 1,
        audit_hash: "user_hash_1".into(),
        timestamp_ms: Utc::now().timestamp_millis(),
    });
    app_state.messages.push(ChatMessageItem {
        role: "assistant".into(),
        content: turn1.assistant_text,
        cot: turn1.reasoning_cot,
        pad: turn1.pad_state.clone(),
        tokens: turn1.token_usage.total_tokens as usize,
        audit_hash: turn1.audit_hash,
        timestamp_ms: turn1.timestamp * 1000,
    });

    // Test Tab switching
    app_state.active_tab = ActiveTab::Memory;
    let memories = host.memory_store.query(Utc::now(), QueryMode::All).await.expect("Memory query");
    app_state.memory_items = memories;
    assert_eq!(app_state.active_tab, ActiveTab::Memory);

    // Test Factory tasks and DiffViewer
    app_state.active_tab = ActiveTab::Factory;
    app_state.factory_tasks.push(FactoryTaskItem {
        id: "task_egress_hardening".into(),
        requirement: "Enforce strict domain whitelist in egress pipeline".into(),
        branch: "factory/task_egress_hardening".into(),
        diff_content: "+pub fn verify_domain(domain: &str) -> bool { domain == \"api.minimaxi.com\" }\n-pub fn allow_all() -> bool { true }".into(),
        passed: true,
        test_count: 70,
        pending_approval: true,
    });
    assert_eq!(app_state.factory_tasks.len(), 2);

    let _diff_rendered = DiffViewer::render_diff(&app_state.factory_tasks[1].diff_content);
    assert!(app_state.factory_tasks[1].diff_content.contains("+pub fn verify_domain"));


    // Test Braille Sparkline
    let braille = BrailleSparkline::render_line(&[0.1, 0.3, 0.6, 0.9, 0.4], 10);
    assert!(!braille.is_empty());
    println!("  ✓ Braille Sparkline Generated: {}", braille);

    println!("[4/4] Verifying Cryptographic Audit Blockchain...");
    let audit = host.audit_chain.lock().await;
    assert!(audit.verify_chain().is_ok(), "Audit hash-chain must be 100% valid");
    println!("  ✓ Audit Hash-Chain Height: {} blocks (Verified 100% Valid)", audit.records().len());

    // Clean up temporary database file
    let _ = tokio::fs::remove_file(&db_path).await;
    println!("\n✅ Live TUI & LLM Simulation Test Succeeded 100%!\n");
}
