//! gh_accel_demo — GitHub 加速插件端到端演示 (真网络, 实测选最快).
//!
//! 链路: 装插件 github-accel → gh_accel 工具: 拉 xiake.pro 节点池 (59 个)
//!      → 本机并发实测前 limit 个 → 选最快 (HTTP 2xx) → 生成加速 URL/命令
//!      → **真实验证**: 用最快节点实际抓取一个 GitHub 资源并打印状态.
//!
//! 0 假装: 节点是第三方免费服务, 结果只代表本次实测; 工具只给命令, 不执行.

use std::sync::Arc;
use std::time::Duration;

use apeireth_companion::gh_accel::GhAccelPlugin;
use apeireth_companion::plugin::PluginRegistry;
use apeireth_companion::tool_bridge::ToolBridge;
use apeireth_memory::SqliteMemoryStore;
use apeireth_tool_runtime::parser::ParsedToolCall;
use serde_json::json;

#[tokio::main]
async fn main() {
    let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    let bridge = Arc::new(ToolBridge::new(store));
    let plugins = PluginRegistry::new();

    println!("═══════════ gh_accel_demo — GitHub 加速插件 (xiake.pro 节点池) ═══════════\n");

    // 1) 装插件
    plugins.install(&bridge, Arc::new(GhAccelPlugin)).unwrap();
    println!("[1] 插件已装: github-accel (gh_accel 工具已注册)");

    // 2) 调用工具: 实测前 12 个节点
    let call = ParsedToolCall {
        tool_name: "gh_accel".into(),
        args: json!({
            "limit": 12,
            "github_url": "https://github.com/octocat/Hello-World/archive/refs/heads/master.zip"
        }),
        raw_marker: String::new(),
        archery: false,
        archery_no_reply: false,
    };
    let start = std::time::Instant::now();
    let r = bridge.execute_if_allowed(&call).await;
    assert!(r.success, "gh_accel 应可执行: {:?}", r.error);
    let out = r.output.clone();
    println!("\n[2] 节点池 {} 个, 实测 {} 个 (耗时 {:?})", out["pool_total"], out["probed"], start.elapsed());
    for row in out["results"].as_array().unwrap() {
        println!(
            "    {:<28} 本站{:>5}ms  实测{:>7}  http {}  {}",
            row["host"].as_str().unwrap_or("?"),
            row["site_latency_ms"].as_u64().unwrap_or(0),
            row["measured_ms"].as_u64().map(|m| format!("{m}ms")).unwrap_or_else(|| "超时".into()),
            row["http_status"].as_u64().map(|s| s.to_string()).unwrap_or_else(|| "-".into()),
            if row["ok"].as_bool().unwrap_or(false) { "✓ 可用" } else { "✗" },
        );
    }
    println!("\n    最快: {:?}", out["fastest"]);
    println!("    加速 URL: {:?}", out["accelerated_url"]);
    println!("    git clone: {:?}", out["commands"]["git_clone"]);

    // 3) 真实验证: 用最快节点实际抓加速 URL (archive zip — 真实用途)
    if let Some(fast) = out["fastest"].as_object() {
        let node = fast["url"].as_str().unwrap();
        let target = out["accelerated_url"].as_str().unwrap_or("").to_string();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("Mozilla/5.0 (Apeireth gh_accel demo)")
            .build()
            .unwrap();
        println!("\n[3] 真实验证: GET {target}");
        match client.get(&target).send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let body = resp.bytes().await.unwrap_or_default();
                let head = String::from_utf8_lossy(&body[..body.len().min(60)]);
                let is_zip = body.len() >= 2 && body[0] == b'P' && body[1] == b'K';
                println!("    HTTP {status}, {} bytes, zip魔数={}", body.len(), is_zip);
                println!("    开头: {:?}", head);
                let ok = status == 200 && is_zip;
                if ok {
                    println!("    ✅ 加速链路真实可用 (GitHub archive 经最快节点抓取成功)");
                } else {
                    println!("    ⚠️ 节点可达但内容异常 ({} 节点) — 如实标注, 不以假充真", node);
                }
            }
            Err(e) => println!("    ❌ 验证请求失败: {e} (节点刚测可用仍失败 = 免费节点不稳定, 如实记录)"),
        }
    } else {
        println!("\n[3] 无可用节点, 跳过真实验证 (免费节点池常有死节点, 稍后重试)");
    }

    println!("\n═══════════ 演示完成: 每次调用都重新实测, 不缓存节点状态 ═══════════");
}
