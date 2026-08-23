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
    println!("🔍 正在初始化 Apeireth 2.0 Living Runtime Host...");
    println!("==================================================");

    let host = UnifiedRuntimeHost::new(api_key, ":memory:").await?;
    let session_id = "live_screen_check_session";

    let prompt = "请调用你的 screen_observe 工具捕获并分析我现在的物理屏幕，如实、详细地告诉我：你捕获到的物理分辨率是多少、当前前台活动窗口以及屏幕上检测到了哪些可见窗口与控件，并用你作为认知伴侣的口吻直接向我汇报。";

    println!("\n📤 向 Apeireth 伴侣发出指令: \"{}\"\n", prompt);
    println!("⏳ 正在执行物理 GDI 屏幕抓取、OmniParser 逆解与大模型认知推理...\n");

    let output = host.handle_chat_turn(session_id, prompt).await?;

    println!("==================================================");
    println!("🤖 Apeireth 大模型实时观察汇报：");
    println!("==================================================");
    println!("{}\n", output.assistant_text);

    if let Some(cot) = output.reasoning_cot {
        println!("🧠 思考链 (Reasoning CoT):\n{}\n", cot);
    }

    let (chat_turns, tool_execs, latency, tokens) = host.telemetry.metrics_snapshot();
    println!("📊 实时遥测数据: 对话轮次={}, 工具调用次数={}, 总耗时={}ms, 消耗Token={}", chat_turns, tool_execs, latency, tokens);

    Ok(())
}
