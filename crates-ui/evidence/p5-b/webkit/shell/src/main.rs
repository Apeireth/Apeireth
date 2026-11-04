//! ⭐⭐⭐ WebKitGTK 侧引擎探针（Tauri 2 + WebKitGTK）。
//!
//! ## 它做什么
//!
//! 1. 起一个**真实**的 Tauri 窗口（Linux 上 = **WebKitGTK** ⇒ ⇒ **不是** Chromium）；
//! 2. 页面加载引擎 wasm 并跑引擎侧探针（键盘语义 / 焦点模态 / DOM 基础）；
//! 3. 页面把结果**通过 IPC 回传**（`save_result` command）⇒ ⇒ Rust 写进 JSON 文件；
//! 4. 退出（取证脚本据文件判红绿）。
//!
//! ## 为什么走 IPC 而不是 `eval` 读回
//!
//! Tauri 2 的 `WebviewWindow::eval` **只执行、不回传**（返回 `Result<()>`）
//! ⇒ ⇒ 拿不到页面里的值 ⇒ ⇒ ⇒ 唯一可靠的回传通道是 **IPC command**。
//!
//! ## 为什么值得单独做
//!
//! 引擎此前的全部浏览器证据都来自 **Chromium** ⇒ ⇒ 「真实 WebKitGTK」**0 次验证**
//! ⇒ ⇒ 而「焦点陷阱」「IME」「Tab 走引擎」这些**依赖内核行为**的能力
//! ⇒ ⇒ ⇒ **恰恰是最可能跨内核不一致**的 ⇒ ⇒ ⇒ 必须换内核验一遍。

use std::io::Write;
use tauri::Manager;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn main() {
    let result_file = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/webkit-probe-result.json".to_owned());
    // ⭐ 结果槽：`Arc` ⇒ ⇒ command（静态 fn）也能拿到 ⇒ ⇒ 不用 Tauri 的 State 机制。
    let received: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    let file_for_command = result_file.clone();
    let sink_for_command = received.clone();
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![save_result])
        .setup(move |app| {
            let handle = app.handle().clone();
            let file = file_for_command.clone();
            let sink = sink_for_command.clone();
            std::thread::spawn(move || {
                let started = Instant::now();
                let limit = Duration::from_secs(180);
                loop {
                    std::thread::sleep(Duration::from_millis(400));
                    // ⭐⭐ 诊断：**通过 eval 反查页面状态**（`window.title()` 是**窗口**标题，
                    //   不是 `document.title` ⇒ ⇒ 上一版的诊断**无效** ⇒ ⇒ 这里改成
                    //   ⇒ ⇒ 由 Rust 主动 eval 一个表达式，把结果**写进 document.title**
                    //   ⇒ ⇒ ⇒ 再读窗口 title（此时它就等于页面状态了）。
                    if let Some(window) = handle.get_webview_window("main") {
                        let url = window.url().map(|u| u.to_string()).unwrap_or_default();
                        eprintln!("[probe] url={url}");
                    }
                    // ⭐⭐⭐ 回传通道：**location.hash**（已实测可用）⇒ ⇒ 解析 URL 取结果。
                    let payload = handle
                        .get_webview_window("main")
                        .and_then(|window| window.url().ok())
                        .and_then(|url| {
                            let text = url.to_string();
                            let index = text.find("#result=")?;
                            let encoded = &text[index + "#result=".len()..];
                            percent_decode(encoded)
                        })
                        .filter(|payload| !payload.is_empty());
                    if let Some(payload) = payload {
                        let mut out = std::fs::File::create(&file).expect("写结果文件失败");
                        out.write_all(payload.as_bytes()).expect("写结果失败");
                        let _ = out.flush();
                        handle.exit(0);
                        return;
                    }
                    if started.elapsed() >= limit {
                        let mut out = std::fs::File::create(&file).expect("写结果文件失败");
                        out.write_all(
                            r#"{"ok":false,"fatal":"探针未在时限内回传结果"}"#.as_bytes(),
                        )
                        .expect("写结果失败");
                        let _ = out.flush();
                        handle.exit(1);
                        return;
                    }
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("启动 Tauri 失败");
}

/// ⭐ **URL 百分号解码**（hash 回传通道用）。
///
/// ⚠️ **只解 `%XX` 与 `+`** ⇒ ⇒ 不处理 UTF-8 多字节的组合形式
/// ⇒ ⇒ ⇒ 但 `encodeURIComponent` 产出的**正是**逐字节 `%XX` ⇒ ⇒ ⇒ 够用 ✓
fn percent_decode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).ok()?;
                let value = u8::from_str_radix(hex, 16).ok()?;
                out.push(value);
                index += 3;
            }
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

/// ⭐ 页面调用的 command：把探针结果**收下**（写盘由后台线程做）。
///
/// ## 参数形状
///
/// 页面调 `invoke('save_result', { payload: { result: "<json>" } })`
/// ⇒ ⇒ ⇒ 参数名 `payload`、内部字段 `result` ⇒ ⇒ **必须与页面**逐字**对应**。
#[tauri::command]
fn save_result(
    payload: serde_json::Value,
    sink: tauri::State<'_, Arc<Mutex<Option<String>>>>,
) -> Result<(), String> {
    let text = payload
        .get("result")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "payload.result 缺失或不是字符串".to_owned())?
        .to_owned();
    let mut guard = sink.inner().lock().map_err(|error| error.to_string())?;
    *guard = Some(text);
    Ok(())
}
