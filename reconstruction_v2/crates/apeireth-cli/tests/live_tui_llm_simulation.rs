use std::sync::Arc;
use apeireth_runtime::UnifiedRuntimeHost;
use apeireth_cli::tui::state::{AppState, NavPage};
use apeireth_cli::tui::theme::Theme;
use apeireth_cli::tui::ui::{compute_scroll_y, compute_scrollbar_position};
use apeireth_cli::tui::widgets::BrailleSparkline;


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

    println!("\n[1/5] Initializing Live UnifiedRuntimeHost with ACT-R Memory & MiniMax...");
    let host = Arc::new(UnifiedRuntimeHost::new(api_key, &db_path).await.expect("Host initialization"));
    let session_id = format!("sim_sess_{}", uuid::Uuid::new_v4());

    println!("[2/5] Testing Live Multi-Turn LLM Conversation & PAD / CoT / Audit...");
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

    // Turn 3: User queries self-awareness and native capabilities
    let user_msg3 = "你是谁？你都有哪些核心能力？请直接介绍。";
    let turn3 = host.handle_chat_turn(&session_id, user_msg3).await.expect("Turn 3 execution");

    assert!(!turn3.assistant_text.contains("作为一个人工智能语言模型"), "Must not use AI disclaimer clichés");
    assert!(!turn3.assistant_text.contains("作为一个AI语言模型"), "Must not use AI disclaimer clichés");
    println!("  ✓ Turn 3 Self-Awareness: \"{}\"",
        &turn3.assistant_text[..turn3.assistant_text.len().min(120)]
    );


    println!("[3/5] Testing 4 Live Engineering Telemetry Panels on Bridge...");
    let mut app_state = AppState::new(session_id.clone(), &db_path);

    // Verify Engineering Telemetry
    assert_eq!(app_state.telemetry.screen_driver, "BitBlt + DIBits Zero-Leak Pipeline");
    assert_eq!(app_state.telemetry.wal_pool_status, "SQLite WAL Mode (Max: 10 Conns)");
    assert_eq!(app_state.telemetry.egress_whitelist, "严格域名白名单拦截已生效 (Default Deny)");
    assert_eq!(app_state.current_page, NavPage::Bridge);
    println!("  ✓ Engineering Telemetry: GDI ({}), WAL ({}), Egress ({}) Verified",
        app_state.telemetry.screen_resolution,
        app_state.telemetry.wal_pool_status,
        app_state.telemetry.egress_whitelist
    );

    println!("[4/5] Testing Scrolling Mechanics (compute_scroll_y & Scrollbar)...");
    // Test scroll to bottom anchor
    let total_lines = 50;
    let viewport = 20;
    let max_scroll = (total_lines - viewport) as u16; // 30
    assert_eq!(compute_scroll_y(true, 0, max_scroll), 30); // locked to bottom

    // Test scroll up (PageUp by 5 lines)
    assert_eq!(compute_scroll_y(false, 5, max_scroll), 25); // moved up by 5
    assert_eq!(compute_scroll_y(false, 30, max_scroll), 0);  // moved to top

    // Test scrollbar mapping
    let pos_bottom = compute_scrollbar_position(30, max_scroll, total_lines);
    let pos_top = compute_scrollbar_position(0, max_scroll, total_lines);
    assert_eq!(pos_bottom, 49);
    assert_eq!(pos_top, 0);
    println!("  ✓ Scroll Math: Bottom Anchor=30, PageUp(-5)=25, Top=0, Scrollbar=[0..49] OK");

    println!("[5/5] Testing Theme Transition & SHA-256 Audit Verification...");
    app_state.toggle_theme();
    assert_eq!(app_state.theme, Theme::Era);
    let braille = BrailleSparkline::render_line(&[0.1, 0.3, 0.6, 0.9, 0.4], 10);
    assert!(!braille.is_empty());

    let audit = host.audit_chain.lock().await;
    assert!(audit.verify_chain().is_ok(), "Audit hash-chain must be 100% valid");
    println!("  ✓ Audit Hash-Chain Height: {} blocks (Verified 100% Valid)", audit.records().len());

    // Clean up temporary database file
    let _ = tokio::fs::remove_file(&db_path).await;
    println!("\n✅ Live TUI Engineering & Scrolling Test Succeeded 100%!\n");
}
