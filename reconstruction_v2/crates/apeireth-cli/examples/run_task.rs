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
    let prompt = "请调用 desktop_action 的 open_url 功能，在我的物理桌面上打开 Edge 浏览器并直接导航到 Bilibili 的 live2d 模型搜索页面 (https://search.bilibili.com/all?keyword=live2d模型)，确保浏览器窗口真实弹到屏幕最前方呈现在我眼前！";





    println!("\n📤 下发用户任务指令: \"{}\"\n", prompt);
    println!("⏳ Apeireth 正在进行认知推演并调度底层 Win32/Shell 工具链...\n");

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
