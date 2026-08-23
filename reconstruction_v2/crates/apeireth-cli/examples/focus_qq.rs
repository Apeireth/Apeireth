use apeireth_runtime::host::UnifiedRuntimeHost;
use std::fs;
use std::thread;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let key_path = r"C:\Users\31683\apikey-ultra.txt";
    let api_key = fs::read_to_string(key_path)
        .expect("Failed to read API key")
        .trim()
        .to_string();

    println!("==================================================");
    println!("🚀 正在启动 Apeireth 2.0 现场物理窗口置顶演示...");
    println!("==================================================");

    let host = UnifiedRuntimeHost::new(api_key, ":memory:").await?;
    let session_id = "live_focus_qq_demo";

    println!("\n⏳ 3秒后将把【QQ窗口】直接拉到你的屏幕最前方，请观察你的桌面...");
    for i in (1..=3).rev() {
        println!("⏱️  倒计时: {}...", i);
        thread::sleep(Duration::from_secs(1));
    }

    let prompt = "请调用 desktop_action 的 focus_window 功能，把当前电脑上的【QQ】窗口立刻拉到物理屏幕最前方并置顶！";
    println!("\n📤 下发指令: \"{}\"", prompt);

    let output = host.handle_chat_turn(session_id, prompt).await?;

    println!("\n==================================================");
    println!("🤖 Apeireth 执行汇报：");
    println!("==================================================");
    println!("{}\n", output.assistant_text);

    let (chat_turns, tool_execs, latency, tokens) = host.telemetry.metrics_snapshot();
    println!("📊 实时遥测: 对话轮次={}, 工具调用={}, 耗时={}ms, 消耗Token={}", chat_turns, tool_execs, latency, tokens);

    Ok(())
}
