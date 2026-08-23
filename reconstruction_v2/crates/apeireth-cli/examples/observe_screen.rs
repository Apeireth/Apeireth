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

    let prompt = "请调用你的 screen_observe 工具，把当前我电脑上打开的【所有】应用程序窗口（包括代码编辑器、浏览器、后台工具、系统窗口等）一个不漏地全部列成清单汇报给我，标明每个窗口的软件类型和标题。";


    println!("\n📤 向 Apeireth 伴侣发出指令: \"{}\"\n", prompt);
    use apeireth_tools::vision::ScreenObserveTool;
    use apeireth_tools::Tool;
    let screen_tool = ScreenObserveTool::new();
    let raw_obs = screen_tool.execute(serde_json::json!({
        "detect_elements": true,
        "max_elements": 50
    })).await?;
    println!("🖥️ 【Win32 GDI & OmniParser 物理抓屏原始清单】:\n{}\n", raw_obs.output);

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
