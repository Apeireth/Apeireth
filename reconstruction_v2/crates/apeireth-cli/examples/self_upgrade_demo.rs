use std::fs;
use apeireth_runtime::host::UnifiedRuntimeHost;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let key_path = r"C:\Users\31683\apikey-ultra.txt";
    let api_key = fs::read_to_string(key_path)
        .expect("Failed to read API key")
        .trim()
        .to_string();

    println!("==================================================");
    println!("🚀 正在启动 Apeireth 2.0 自我升级与代码进化演示...");
    println!("==================================================");

    let host = UnifiedRuntimeHost::new(api_key, ":memory:").await?;
    let session_id = "live_self_upgrade_demo";

    let prompt = r#"Apeireth，你已经成功为自己升级并编写了 'system_monitor' 硬件监控工具！
请执行以下两步任务并向我汇报：
1. 调用 shell 工具执行 `cargo test -p apeireth-tools --test test_system_monitor -- --nocapture`，验证你自己编写的测试套件全部通过；
2. 调用你刚刚挂载并升级的新工具 `system_monitor`，采集并向我汇报当前电脑的物理内存、可用内存与系统硬件指标！"#;



    println!("\n📤 下发自我升级挑战指令:\n{}\n", prompt);
    println!("⏳ Apeireth 正在进行认知推演并开始自主编写 Rust 源码、调度编译器...\n");

    let output = host.handle_chat_turn(session_id, prompt).await?;

    println!("==================================================");
    println!("🤖 Apeireth 执行汇报：");
    println!("==================================================");
    println!("{}\n", output.assistant_text);

    if let Some(cot) = output.reasoning_cot {
        println!("🧠 思考链与认识论反思 (Reasoning CoT):\n{}\n", cot);
    }

    let (chat_turns, tool_execs, latency, tokens) = host.telemetry.metrics_snapshot();
    println!("📊 实时遥测数据: 对话轮次={}, 工具调用次数={}, 耗时={}ms, 消耗Token={}", chat_turns, tool_execs, latency, tokens);

    Ok(())
}
