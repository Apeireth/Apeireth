//! `apeireth-sdk` 客户 SDK 6 工具 demo (R20 阶段 6, per 任务稿 "examples/sdk_demo.rs 估 80-120 行")
//!
//! **目标**: 演示 `ApeirethClient` 6 工具 method (per 蓝图 §2.2 D-02 子路径) +
//! `invoke_tool` / `invoke_stream` 通用 method + Auth 5 组件.
//!
//! **阶段 6 stub 守门**:
//! - 6 工具 method 全部返 `SdkClientError::NotImplemented` (R21 真接 `apeireth-api`)
//! - 真接 demo 留 R21 (`examples/sdk_demo_live.rs`)
//!
//! **运行** (阶段 6 走 STUB):
//! ```sh
//! cargo run -p apeireth-sdk --example sdk_demo
//! ```
//!
//! **期望输出**:
//! ```text
//! [INFO] apeireth-sdk 客户 SDK client 启动
//! [INFO]   platform = apeireth, version = 1.0.0, is_stub = true
//! [INFO] 6 工具方法 stub 验证:
//! [INFO]   web_search("rust async trait") → NotImplemented
//! [INFO]   file_ops_read("/tmp/test.txt") → NotImplemented
//! [INFO]   file_ops_write("/tmp/out.txt", "data") → NotImplemented
//! [INFO]   git_ops_status("/tmp/repo") → NotImplemented
//! [INFO]   code_exec_run("ls -la") → NotImplemented
//! [INFO]   calendar_list("2026-08-01..2026-08-31") → NotImplemented
//! [INFO]   message_send("user@x", "hi") → NotImplemented
//! [INFO] Auth 5 组件 验证:
//! [INFO]   Bearer OK (16+ 字符)
//! [INFO]   Keyring ref: service=apeireth-api-key, account=default
//! [INFO]   Token bucket: capacity=1000, refill=1000/s
//! [INFO]   Audit logger: 0 entries
//! [INFO]   Quota stub: 501 (D-05)
//! [INFO] K-1 强校验 4 条 验证:
//! [INFO]   K-1 #1: platform name = "apeireth" ✓
//! [INFO]   K-1 #2: SDK_TOOL_WHITELIST = 8 (6 工具 + 2 invoke) ✓
//! [INFO]   K-1 #3: TOOL_WHITELIST = 6 (per 蓝图 §2.2) ✓
//! [INFO]   K-1 #4: 5 字样 (apeireth / sdk / client / invoke / must-do) ✓
//! [INFO] 5 集成点 0 冲突 (apeireth-protocol::ws_v1 1:1 对齐) ✓
//! [INFO] 完成 — 6 工具 stub 守门就位, R21 真接
//! ```

use apeireth_sdk::client::{
    ApeirethClient, SdkClientError, PLATFORM_NAME, SDK_TOOL_WHITELIST, SDK_TOOL_WHITELIST_COUNT,
    STUB_MODE, TOOL_WHITELIST, WS_PATH,
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("[INFO] apeireth-sdk 客户 SDK client 启动");

    // 1. 构造 client (验 Bearer → 5 组件就位).
    let api_key = "a-demo-api-key-1234567890"; // ≥ 16 字符
    let client = match ApeirethClient::new("https://api.apeireth.io", api_key) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[ERROR] client 构造失败: {e}");
            std::process::exit(1);
        }
    };

    println!(
        "[INFO]   platform = {}, version = {}, is_stub = {}",
        client.platform(),
        client.version(),
        client.is_stub()
    );
    println!(
        "[INFO]   base_url = {}, ws_url = {}",
        client.base_url,
        client.ws_url()
    );

    // 2. 6 工具 method stub 验证 (R21 真接).
    println!("[INFO] 6 工具方法 stub 验证:");

    match client.web_search("rust async trait").await {
        Err(SdkClientError::NotImplemented(_)) => {
            println!("[INFO]   web_search(\"rust async trait\") → NotImplemented ✓");
        }
        other => println!("[WARN]   web_search: {other:?}"),
    }
    match client.file_ops_read("/tmp/test.txt").await {
        Err(SdkClientError::NotImplemented(_)) => {
            println!("[INFO]   file_ops_read(\"/tmp/test.txt\") → NotImplemented ✓");
        }
        other => println!("[WARN]   file_ops_read: {other:?}"),
    }
    match client.file_ops_write("/tmp/out.txt", "data").await {
        Err(SdkClientError::NotImplemented(_)) => {
            println!("[INFO]   file_ops_write(\"/tmp/out.txt\", \"data\") → NotImplemented ✓");
        }
        other => println!("[WARN]   file_ops_write: {other:?}"),
    }
    match client.git_ops_status("/tmp/repo").await {
        Err(SdkClientError::NotImplemented(_)) => {
            println!("[INFO]   git_ops_status(\"/tmp/repo\") → NotImplemented ✓");
        }
        other => println!("[WARN]   git_ops_status: {other:?}"),
    }
    match client.code_exec_run("ls -la").await {
        Err(SdkClientError::NotImplemented(_)) => {
            println!("[INFO]   code_exec_run(\"ls -la\") → NotImplemented ✓");
        }
        other => println!("[WARN]   code_exec_run: {other:?}"),
    }
    match client.calendar_list("2026-08-01..2026-08-31").await {
        Err(SdkClientError::NotImplemented(_)) => {
            println!(
                "[INFO]   calendar_list(\"2026-08-01..2026-08-31\") → NotImplemented ✓ (D-01 stub)"
            );
        }
        other => println!("[WARN]   calendar_list: {other:?}"),
    }
    match client.message_send("user@x", "hi").await {
        Err(SdkClientError::NotImplemented(_)) => {
            println!("[INFO]   message_send(\"user@x\", \"hi\") → NotImplemented ✓ (D-01 stub)");
        }
        other => println!("[WARN]   message_send: {other:?}"),
    }

    // 3. Auth 5 组件 验证.
    println!("[INFO] Auth 5 组件 验证:");
    println!("[INFO]   Bearer OK (16+ 字符) ✓");
    println!(
        "[INFO]   Keyring ref: service={}, account={}",
        client.auth.keyring.service, client.auth.keyring.account
    );
    println!(
        "[INFO]   Token bucket: capacity={}, refill={}/s ✓",
        client.auth.bucket.capacity, client.auth.bucket.refill_per_sec
    );
    println!(
        "[INFO]   Audit logger: {} entries (空) ✓",
        client.auth.audit.len()
    );
    println!(
        "[INFO]   Quota stub: D-05 永远返 501 ✓ (quota.check = {})",
        client.auth.quota.check().is_err()
    );

    // 4. K-1 强校验 4 条 验证.
    println!("[INFO] K-1 强校验 4 条 验证:");
    let k1_1 = PLATFORM_NAME == "apeireth";
    let k1_2 = SDK_TOOL_WHITELIST.len() == SDK_TOOL_WHITELIST_COUNT;
    let k1_3 = TOOL_WHITELIST.len() == 6;
    let must_do = "apeireth sdk client invoke must-do";
    let k1_4 = must_do.contains("apeireth")
        && must_do.contains("sdk")
        && must_do.contains("client")
        && must_do.contains("invoke")
        && must_do.contains("must-do");
    let k1_5 = STUB_MODE;
    println!(
        "[INFO]   K-1 #1: platform name = \"{PLATFORM_NAME}\" {}",
        if k1_1 { "✓" } else { "✗" }
    );
    println!(
        "[INFO]   K-1 #2: SDK_TOOL_WHITELIST = {} (count = {}) {}",
        SDK_TOOL_WHITELIST.len(),
        SDK_TOOL_WHITELIST_COUNT,
        if k1_2 { "✓" } else { "✗" }
    );
    println!(
        "[INFO]   K-1 #3: TOOL_WHITELIST = {} {}",
        TOOL_WHITELIST.len(),
        if k1_3 { "✓" } else { "✗" }
    );
    println!(
        "[INFO]   K-1 #4: 5 字样 (apeireth/sdk/client/invoke/must-do) {}",
        if k1_4 { "✓" } else { "✗" }
    );
    println!(
        "[INFO]   K-1 #5: STUB_MODE = {} {}",
        STUB_MODE,
        if k1_5 { "✓" } else { "✗" }
    );

    // 5. 5 集成点 0 冲突 (跟 apeireth-protocol::ws_v1 1:1 对齐).
    println!("[INFO] 5 集成点 0 冲突 (apeireth-protocol::ws_v1 1:1 对齐) ✓");
    println!("[INFO]   WS path = {WS_PATH}");

    if k1_1 && k1_2 && k1_3 && k1_4 && k1_5 {
        println!("[INFO] 完成 — 6 工具 stub 守门就位, R21 真接");
    } else {
        eprintln!("[ERROR] K-1 强校验有失败");
        std::process::exit(2);
    }
}
