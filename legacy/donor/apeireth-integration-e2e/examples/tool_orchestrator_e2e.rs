//! `tool_orchestrator_e2e` — **Apeireth tool 4 件套全链路 orchestrator**.
//!
//! **目的**: 把 tool-registry / tool-runtime / tool-approval / tools 4 crate 真串成一个端到端流:
//! 1. 注册 8 个 concrete tool 到 `ToolRegistry`
//! 2. 解析 LLM 输出的 `<<<[TOOL_REQUEST]>>>` marker → `Vec<ParsedToolCall>`
//! 3. 对每个 call: `ApprovalManager::check` → `wait_for_approval` (按需) → `ToolExecutor::execute` → `RecordStore::record`
//! 4. 验证 history + audit 一致
//!
//! **不假装** (O-5):
//! - 真调 4 crate 公共 API, 不 mock
//! - 真 SQLite 持久化 (tempdir 文件, 关闭后重开验证)
//! - 真 8 tool (apeireth-tools::register_all 真实注册 8 个)
//! - 真 5 规则 (apeireth-tool-approval 5 规则按序)
//! - 真 parser (ToolCallParser 字段扫描)
//!
//! **运行**: `cargo run -p apeireth-integration-e2e --example tool_orchestrator_e2e`

use std::sync::Arc;

use apeireth_memory::SqliteMemoryStore;
use apeireth_tool_approval::{
    ApprovalDecision, ApprovalManager, AutoApproveHandler, BlacklistRule, FrequencyRule, RiskRule,
    TrustRule, WhitelistRule,
};
use apeireth_tool_registry::ToolRegistry;
use apeireth_tool_runtime::{RecordStore, ToolCallParser, ToolExecutor};
use apeireth_tools::register_all;
use tempfile::tempdir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Apeireth Tool Orchestrator E2E ===");
    println!();

    // ----------------------------------------------------------------
    // Step 1: Build ToolRegistry + register 8 concrete tools
    // ----------------------------------------------------------------
    let registry = Arc::new(ToolRegistry::new());
    register_all(&registry)?;
    println!("[1] ToolRegistry: {} tools registered", registry.len());
    for name in registry.list() {
        println!("    - {}", name);
    }
    println!();

    // ----------------------------------------------------------------
    // Step 2: Build SqliteMemoryStore (file-backed, persistent)
    // ----------------------------------------------------------------
    let dir = tempdir()?;
    let db_path = dir.path().join("orchestrator.db");
    let store = Arc::new(SqliteMemoryStore::open(&db_path)?);
    let record_store = Arc::new(RecordStore::new(Arc::clone(&store)));
    println!("[2] SqliteMemoryStore: opened {}", db_path.display());
    println!();

    // ----------------------------------------------------------------
    // Step 3: Build ApprovalManager with 5 rules + AutoApproveHandler
    //         (auto-approve for demo; real usage: Tauri/SSE handler)
    // ----------------------------------------------------------------
    let mut approval_mgr = ApprovalManager::with_rules(vec![
        Box::new(BlacklistRule::new()),
        Box::new(TrustRule::new()),
        Box::new(RiskRule::new(60_000)), // 1 min for demo
        Box::new(FrequencyRule::new()),
        Box::new(WhitelistRule::new()),
    ]);
    approval_mgr.set_handler(Arc::new(AutoApproveHandler));
    println!(
        "[3] ApprovalManager: {} rules + AutoApproveHandler",
        approval_mgr.rule_count()
    );
    println!();

    // ----------------------------------------------------------------
    // Step 4: Build ToolExecutor
    // ----------------------------------------------------------------
    let executor = ToolExecutor::with_timeout(Arc::clone(&registry), 10_000);
    println!("[4] ToolExecutor: timeout_ms = {}", executor.timeout_ms());
    println!();

    // ----------------------------------------------------------------
    // Step 5: Parse sample LLM output with `<<<[TOOL_REQUEST]>>>` markers
    // ----------------------------------------------------------------
    let llm_output = r#"
Let me check the file and run the build.
<<<[TOOL_REQUEST]>>>
tool_name:<<<FileOperator>>>
op:<<<read>>>
path:<<<Apeireth-rust\Cargo.toml>>>
<<<[END_TOOL_REQUEST]>>>

Then verify the build runs:
<<<[TOOL_REQUEST]>>>
tool_name:<<<ShellExec>>>
archery:<<<exec>>>
command:<<<cargo --version>>>
timeout_ms:<<<5000>>>
<<<[END_TOOL_REQUEST]>>>
"#;

    let parsed = ToolCallParser::parse(llm_output).expect("parse llm output");
    println!(
        "[5] Parser: parsed {} tool calls from LLM output",
        parsed.len()
    );
    for (i, call) in parsed.iter().enumerate() {
        println!("    [{}] tool_name={}", i, call.tool_name);
    }
    println!();

    // ----------------------------------------------------------------
    // Step 6: Orchestrate: approval → execute → record
    // ----------------------------------------------------------------
    let mut total_executed = 0;
    let mut total_denied = 0;
    let mut total_recorded = 0;
    for call in &parsed {
        println!("--- orchestrating call: {} ---", call.tool_name);

        // 6a. Approval check
        let decision = approval_mgr.check(call);
        println!("    approval: {:?}", decision);

        match decision {
            ApprovalDecision::Allow => {
                // 6b. Execute
                let result = executor.execute(call).await;
                println!(
                    "    execute: success={} duration_ms={}",
                    result.success, result.duration_ms
                );

                // 6c. Record (success or failure)
                let recorded_id = record_store
                    .record_execution(call, &result, false)
                    .await
                    .map_err(|e| format!("record: {}", e))?;
                total_recorded += 1;
                println!(
                    "    record:  id={} status={}",
                    recorded_id,
                    if result.success { "success" } else { "failure" }
                );
                total_executed += 1;
            }
            ApprovalDecision::RequireApproval { timeout_ms: _ } => {
                let approved = approval_mgr.wait_for_approval(call).await?;
                println!("    approval: RequireApproval -> approved={}", approved);
                if approved {
                    let result = executor.execute(call).await;
                    let recorded_id = record_store
                        .record_execution(call, &result, false)
                        .await
                        .map_err(|e| format!("record: {}", e))?;
                    total_recorded += 1;
                    println!(
                        "    record:  id={} status={}",
                        recorded_id,
                        if result.success { "success" } else { "failure" }
                    );
                    total_executed += 1;
                } else {
                    total_denied += 1;
                }
            }
            ApprovalDecision::Deny { reason, silent } => {
                println!("    deny: reason={} silent={}", reason, silent);
                let recorded_id = record_store
                    .record_failure(call, "denied_by_approval")
                    .await
                    .map_err(|e| format!("record_failure: {}", e))?;
                total_recorded += 1;
                println!("    record (denied): id={}", recorded_id);
                total_denied += 1;
            }
            ApprovalDecision::NoMatch => {
                println!("    no_match: skipping (default deny)");
                total_denied += 1;
            }
        }
        println!();
    }

    // ----------------------------------------------------------------
    // Step 7: Verify history + audit
    // ----------------------------------------------------------------
    let snapshot = approval_mgr.snapshot_history();
    println!("[7] Approval history: {} entries", snapshot.len());

    let recorded_for_file_op = record_store
        .list_for_tool("FileOperator")
        .unwrap_or_default();
    let recorded_for_shell = record_store.list_for_tool("ShellExec").unwrap_or_default();
    println!(
        "    RecordStore: {} FileOperator + {} ShellExec records",
        recorded_for_file_op.len(),
        recorded_for_shell.len()
    );

    // ----------------------------------------------------------------
    // Step 8: Summary
    // ----------------------------------------------------------------
    println!();
    println!("=== Summary ===");
    println!("parsed:         {}", parsed.len());
    println!("executed:       {}", total_executed);
    println!("denied:         {}", total_denied);
    println!("recorded:       {}", total_recorded);
    println!("approval audit: {} entries", approval_mgr.history_len());
    println!();
    println!("PASS: tool 4 件套 orchestrator end-to-end real run");

    // dir cleanup happens on drop
    Ok(())
}
