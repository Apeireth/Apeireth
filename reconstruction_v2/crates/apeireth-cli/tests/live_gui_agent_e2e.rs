use std::fs;
use apeireth_runtime::host::UnifiedRuntimeHost;
use apeireth_tools::vision::ScreenObserveTool;
use apeireth_tools::builtin::invest::InvestTool;
use apeireth_tools::builtin::learning::LearningTool;
use apeireth_tools::Tool;

#[tokio::test]
async fn test_live_gui_agent_and_capabilities_e2e() {
    let key_path = r"C:\Users\31683\apikey-ultra.txt";
    if !std::path::Path::new(key_path).exists() {
        eprintln!("[SKIP] API key file not found at {}", key_path);
        return;
    }

    let api_key = fs::read_to_string(key_path).expect("Read API key").trim().to_string();
    assert!(!api_key.is_empty(), "API key cannot be empty");

    println!("\n=== 1. Testing Live Win32 GDI Screen Perception & UI OmniParser ===");
    let screen_tool = ScreenObserveTool::new();
    let screen_res = screen_tool.execute(serde_json::json!({
        "detect_elements": true,
        "max_elements": 20
    })).await.expect("Screen observe execution");
    assert!(screen_res.success);
    println!("Screen Perception Output Preview:\n{}\n", screen_res.output.chars().take(300).collect::<String>());

    println!("=== 2. Testing Live Investment Financial Analysis Tool ===");
    let invest_tool = InvestTool::new();
    let risk_res = invest_tool.execute(serde_json::json!({
        "action": "risk_plan",
        "entry_price": 230.50,
        "stop_loss": 220.00,
        "take_profit": 260.00,
        "account_size": 50000.0,
        "risk_percent": 2.0
    })).await.expect("Invest risk execution");
    assert!(risk_res.success);
    assert!(risk_res.output.contains("Risk/Reward Ratio"));
    println!("Invest Risk Output:\n{}\n", risk_res.output);

    let hyp_res = invest_tool.execute(serde_json::json!({
        "action": "hypothesis",
        "symbol": "AAPL",
        "thesis": "Apple Intelligence rollouts drive upgraded supercycle in Q4",
        "target_price": 260.0,
        "timeframe_days": 90,
        "confidence": 0.82
    })).await.expect("Hypothesis execution");
    assert!(hyp_res.success);
    println!("Investment Hypothesis Logged:\n{}\n", hyp_res.output);

    println!("=== 3. Testing Live Knowledge Digestion & Learning Assistant Tool ===");
    let learning_tool = LearningTool::new();
    let learn_res = learning_tool.execute(serde_json::json!({
        "action": "digest",
        "topic": "Rust Async & Act-R Cognitive Architecture",
        "content": "Apeireth 2.0 combines Rust Tokio async runtime with ACT-R cognitive activation curves.\nMemories decay over time according to power laws.\nGovernance Layer 1-3 ensures strict safety boundaries before tool execution."
    })).await.expect("Learning digest execution");
    assert!(learn_res.success);
    assert!(learn_res.output.contains("Knowledge Digestion Report"));
    println!("Learning Digest Output:\n{}\n", learn_res.output);

    println!("=== 4. Testing Unified Living Runtime Host with All Tools Mounted ===");
    let host = UnifiedRuntimeHost::new(api_key, ":memory:").await.expect("Init UnifiedRuntimeHost");
    let session_id = "live_e2e_gui_test_sess";

    let turn_out = host.handle_chat_turn(session_id, "你能通过什么方式观察我的电脑屏幕并辅助我的学习与投资？").await.expect("Chat turn");
    println!("Apeireth Multi-Capability Response:\n{}\n", turn_out.assistant_text);

    assert!(!turn_out.assistant_text.trim().is_empty());
    assert!(!turn_out.audit_hash.is_empty());
    println!("=== All E2E GUI Perception & Investment/Learning Capabilities Verified Successfully! ===");
}
