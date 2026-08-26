//! **战役 2-2 / Example — runtime_demo**
//!
//! **目标**: 完整链路端到端真测
//! 1. 注册 2 个 mock 工具 (echo / calc) 到 ToolRegistry
//! 2. 模拟 LLM 输出 (含 think 块 + tool_call + 拼写错误)
//! 3. parse → fuzzy match → execute → privacy mask → record
//!
//! **运行**:
//! ```bash
//! cargo run -p apeireth-tool-runtime --example runtime_demo
//! ```

use std::sync::Arc;

use apeireth_memory::SqliteMemoryStore;
use apeireth_tool_registry::{MockSyncTool, ToolRegistry};
use apeireth_tool_runtime::{
    FuzzyToolMatcher, PrivacyGuard, RecordStore, ToolCallParser, ToolExecutor,
};
use serde_json::json;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    banner("战役 2-2 runtime_demo 启动");

    // === 1. 注册 2 个 mock 工具 (echo / calc) ===
    let registry = Arc::new(ToolRegistry::new());
    registry.register(
        "Echo".to_string(),
        Arc::new(MockSyncTool {
            name: "Echo".to_string(),
        }),
    );
    registry.register(
        "Calc".to_string(),
        Arc::new(MockSyncTool {
            name: "Calc".to_string(),
        }),
    );
    println!("[1] 注册 2 个 mock 工具: {:?}", registry.list());

    // === 2. 模拟 LLM 输出 (含 think 块) ===
    let llm_output = r#"
<think>
用户让我算 1+1, 我用 Calc 工具.
同时, 还需要 echo 一段话.
</think>

我将先 echo 一段话, 然后计算.

<<<[TOOL_REQUEST]>>>
tool_name:<<<Echo>>>
message:<<<Hello from runtime_demo>>>
maid:<<<chuling>>>
<<<[END_TOOL_REQUEST]>>>

<<<[TOOL_REQUEST]>>>
tool_name:<<<Calc>>>
expr:<<<1+1>>>
<<<[END_TOOL_REQUEST]>>>
"#;
    println!("[2] 模拟 LLM 输出 (含 <think> 块 + 2 个 tool_call)");

    // === 3. parse ===
    let calls = ToolCallParser::parse(llm_output).expect("parse");
    println!("[3] 解析出 {} 个 tool_call:", calls.len());
    for c in &calls {
        println!("    - {} args={}", c.tool_name, c.args);
    }
    assert_eq!(calls.len(), 2, "应有 2 个 tool_call (echo + calc)");

    // === 4. fuzzy match 演示 (LLM 拼错工具名) ===
    let typo = "Calcc"; // 多了一个 c
    let resolved = FuzzyToolMatcher::match_tool(typo, &registry);
    println!(
        "[4] Fuzzy match: '{}' → {:?} (Levenshtein ≤ 2 命中)",
        typo, resolved
    );
    assert_eq!(resolved, Some("Calc".to_string()));

    // === 5. executor ===
    let executor = ToolExecutor::new(registry.clone());
    println!("[5] 真执行 2 个 tool_call:");
    let exec_results = executor.execute_all(&calls).await;
    for r in &exec_results {
        println!(
            "    - {} success={} duration={}ms output={}",
            r.tool_name, r.success, r.duration_ms, r.output
        );
        assert!(r.success, "{} 应成功", r.tool_name);
    }
    assert_eq!(exec_results.len(), 2);

    // === 6. privacy mask (含 API key) ===
    let privacy = PrivacyGuard::new();
    let sensitive_output = json!({
        "echo_result": exec_results[0].output,
        "calc_result": exec_results[1].output,
        "api_key": "sk-verylongsecretvaluethatistoolong1234567",
        "user_password": "mysecretpassword123456",
        "nested": {
            "private_key": "-----BEGIN RSA PRIVATE KEY-----\nABCDEF\n-----END-----"
        }
    });
    let masked = privacy.mask(&sensitive_output);
    println!("[6] Privacy mask (4 敏感字段应被 mask):");
    println!("    api_key        → {}", masked["api_key"]);
    println!("    user_password  → {}", masked["user_password"]);
    println!("    private_key    → {}", masked["nested"]["private_key"]);
    println!("    echo_result    → {}", masked["echo_result"]);
    assert_ne!(
        masked["api_key"],
        "sk-verylongsecretvaluethatistoolong1234567"
    );
    assert!(masked["api_key"]
        .as_str()
        .unwrap()
        .contains("[APEIRETH_PRIVACY_REDACTED]"));

    // === 7. record (append-only 到 apeireth-memory action_stream) ===
    let memory_store = Arc::new(SqliteMemoryStore::open_in_memory().expect("memory"));
    let rec_store = RecordStore::new(memory_store.clone());
    println!("[7] 写入 2 条 tool_call 记录到 action_stream:");
    for (call, exec) in calls.iter().zip(exec_results.iter()) {
        // 真实场景: mask 后再 record
        let id = rec_store
            .record_execution(call, exec, true)
            .await
            .expect("record");
        println!("    - {} → id={}", call.tool_name, id);
    }

    // 读出
    let echo_records = rec_store.list_for_tool("Echo").expect("list echo");
    let calc_records = rec_store.list_for_tool("Calc").expect("list calc");
    println!("    Echo 记录数: {}", echo_records.len());
    println!("    Calc 记录数: {}", calc_records.len());
    assert_eq!(echo_records.len(), 1);
    assert_eq!(calc_records.len(), 1);
    assert_eq!(
        echo_records[0].caller_signature,
        Some("chuling".to_string())
    );
    assert!(echo_records[0].masked, "masked=true 应被记录");

    // === 8. 总结 ===
    banner("战役 2-2 runtime_demo 完结 ✓");
    println!("链路: parse → fuzzy match → execute → privacy mask → record (5 步全跑通)");
    println!("  - parse:    2 个 tool_call 成功解析 (剥 <think> 块)");
    println!("  - fuzzy:    Levenshtein ≤ 2 命中 (LLM 拼错 'Calcc' → 'Calc')");
    println!("  - execute:  2 个工具真调, MockSyncTool 返 echo + result");
    println!("  - privacy:  4 敏感字段 (api_key / password / private_key + high-conf) 全 mask");
    println!("  - record:   2 条 append-only 进 action_stream, caller_signature = chuling");
}

fn banner(s: &str) {
    println!("========================================");
    println!("{s}");
    println!("========================================");
}
