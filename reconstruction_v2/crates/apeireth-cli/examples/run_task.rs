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
    println!("🚀 正在启动 Apeireth 2.0 物理桌面自动化 Agent...");
    println!("==================================================");

    let host = UnifiedRuntimeHost::new(api_key, ":memory:").await?;
    let session_id = "live_edge_bilibili_search";
    let prompt = "请帮我在 Edge 浏览器里打开 Bilibili 并搜索 live2d 模型（直接通过 open_url 打开 Bilibili 的 live2d 模型搜索结果页面）。请调用 desktop_action 在我的 Windows 电脑上真实执行这个操作并向我汇报。";



    println!("\n📤 下发用户任务指令: \"{}\"\n", prompt);
    println!("⏳ Apeireth 正在进行认知推演并调度底层 Win32/Shell 工具链...\n");

    let output = host.handle_chat_turn(session_id, prompt).await?;

    println!("==================================================");
    println!("🤖 Apeireth 执行汇报：");
    println!("==================================================");
    println!("{}\n", output.assistant_text);

    let (chat_turns, tool_execs, latency, tokens) = host.telemetry.metrics_snapshot();
    println!("📊 实时遥测数据: 对话轮次={}, 工具调用次数={}, 耗时={}ms, 消耗Token={}", chat_turns, tool_execs, latency, tokens);

    Ok(())
}
