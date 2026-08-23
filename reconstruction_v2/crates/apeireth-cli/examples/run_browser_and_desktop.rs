use apeireth_runtime::host::UnifiedRuntimeHost;
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let key_path = r"C:\Users\31683\apikey-ultra.txt";
    let api_key = fs::read_to_string(key_path)
        .expect("Failed to read API key")
        .trim()
        .to_string();

    println!("==================================================");
    println!("🚀 正在启动 Apeireth 2.0 专用浏览器与桌面协同 Agent...");
    println!("==================================================");

    let host = UnifiedRuntimeHost::new(api_key, ":memory:").await?;
    let session_id = "live_browser_and_desktop_test";

    let prompt = "请执行以下两个任务：\n1. 使用你的专用 browser 工具抓取 Bilibili (https://www.bilibili.com) 的实时网页内容并提取核心分区。\n2. 使用 desktop_action 的 open_url 功能直接在物理桌面上为我打开 Bilibili 的 live2d 模型搜索页面 (https://search.bilibili.com/all?keyword=live2d模型)！";

    println!("\n📤 下发综合任务指令:\n{}\n", prompt);
    println!("⏳ Apeireth 正在调用专用 browser 工具与 Win32 物理桌面管道...\n");

    let output = host.handle_chat_turn(session_id, prompt).await?;

    println!("==================================================");
    println!("🤖 Apeireth 执行汇报：");
    println!("==================================================");
    println!("{}\n", output.assistant_text);

    let (chat_turns, tool_execs, latency, tokens) = host.telemetry.metrics_snapshot();
    println!("📊 实时遥测数据: 对话轮次={}, 工具调用次数={}, 耗时={}ms, 消耗Token={}", chat_turns, tool_execs, latency, tokens);

    Ok(())
}
