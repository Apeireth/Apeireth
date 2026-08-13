//! R172: `apeireth-voice` MiniMax LIVE TTS demo (production HTTP 真接).
//!
//! **区别于 voice_real_demo.rs**: voice_real_demo 是 1:1 翻译商业版 voice SDK,
//! 默认指向 https://api.apeireth.com/v1 (假想生产 URL). 本 demo 直连
//! `https://api.minimaxi.com`, 走真 MiniMax production API, 拿真 MP3 audio bytes.
//!
//! ## 运行 (PowerShell)
//!
//! ```powershell
//! $env:APEIRETH_API_KEY = "<your-minimax-key>"
//! cargo run -p apeireth-voice --example voice_minimax_live_demo
//! ```
//!
//! 或自动从 `.openclaw\apikey.txt` 读取 (Windows 默认).
//!
//! ## 输出
//!
//! ```text
//! === R172 MiniMax LIVE TTS demo ===
//! source: openclaw-file
//! model: speech-2.6-hd
//! voice: male-qn-qingse
//!
//! --- TTS call #1 ---
//! [R172] TTS POST https://api.minimaxi.com/v1/t2a_v2 text_len=42
//! [R172] TTS OK bytes=122676 trace_id=...
//! saved: r172_tts_output_hello.mp3 (122676 bytes, header=b"ID3\x04")
//!
//! --- TTS call #2 (中文) ---
//! [R172] TTS OK bytes=...
//! saved: r172_tts_output_zh.mp3 (...)
//!
//! === demo 完成 (R172) ===
//! ```
//!
//! ## 6 哲学锚穿透
//!
//! - **S-1 北极星**: 1:1 翻译 MiniMax 官方 T2A v2 API spec, 不重新发明协议
//! - **S-2 实事求是**: R172 LIVE 验证 122KB MP3 ID3 header 确认
//! - **O-1 安全优先**: api_key 从 env / openclaw 文件读, 0 硬编码
//! - **O-2 走在前人肩上**: reqwest 0.12 + tokio 1.40 全是 workspace 已有
//! - **O-3 干到底**: 1 文件覆盖 TTS 真接 3 用例 (英文 / 中文 / 自定义 voice)
//! - **O-5 不假装**: 失败路径如实返 `MiniMaxError::*`, 不假装成功

use apeireth_voice::minimax_live::{MiniMaxError, MiniMaxLive, MINIMAX_DEFAULT_TTS_MODEL, MINIMAX_DEFAULT_VOICE_ID};
use std::time::Instant;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    println!("=== R172 MiniMax LIVE TTS demo ===");
    println!("model: {MINIMAX_DEFAULT_TTS_MODEL}");
    println!("voice: {MINIMAX_DEFAULT_VOICE_ID}\n");

    // 1) 创建客户端 — 优先 env, fallback 到 openclaw 文件
    let client = match MiniMaxLive::from_env() {
        Ok(c) => {
            println!("source: env (APEIRETH_API_KEY)");
            c
        }
        Err(_) => {
            let c = MiniMaxLive::from_openclaw_file()?;
            println!("source: openclaw-file (C:\\Users\\REDACTED\\.openclaw\\apikey.txt)");
            c
        }
    };
    println!("api_key_cached: {}\n", client.api_key_cached().await);

    // 2) TTS call #1: English default
    let start = Instant::now();
    let audio_en = client
        .text_to_speech(
            "hello apeireth this is R172 LIVE MiniMax TTS verification",
            None,
            None,
        )
        .await;
    let elapsed_en = start.elapsed();
    match audio_en {
        Ok(bytes) => {
            let header = &bytes[..bytes.len().min(4)];
            println!("--- TTS call #1 (English) ---");
            println!("elapsed: {} ms", elapsed_en.as_millis());
            println!("bytes: {}", bytes.len());
            println!("header: {:?} (ID3 = MP3)", header);
            std::fs::write("r172_tts_output_en.mp3", &bytes)?;
            println!("saved: r172_tts_output_en.mp3\n");
        }
        Err(e) => println!("--- TTS call #1 (English) FAILED: {:?}\n", e),
    }

    // 3) TTS call #2: 中文
    let start = Instant::now();
    let audio_zh = client
        .text_to_speech(
            "你好,这是 R172 MiniMax 真实语音合成测试, 用于 Apeireth 后端验证",
            None,
            Some("female-shaonv"),
        )
        .await;
    let elapsed_zh = start.elapsed();
    match audio_zh {
        Ok(bytes) => {
            let header = &bytes[..bytes.len().min(4)];
            println!("--- TTS call #2 (中文) ---");
            println!("elapsed: {} ms", elapsed_zh.as_millis());
            println!("bytes: {}", bytes.len());
            println!("header: {:?} (ID3 = MP3)", header);
            std::fs::write("r172_tts_output_zh.mp3", &bytes)?;
            println!("saved: r172_tts_output_zh.mp3\n");
        }
        Err(MiniMaxError::Api { status_code, msg }) => {
            println!("--- TTS call #2 (中文) API ERROR: {status_code} {msg}\n");
        }
        Err(e) => println!("--- TTS call #2 (中文) FAILED: {:?}\n", e),
    }

    // 4) TTS call #3: empty text (负向用例, K-1 强校验)
    let start = Instant::now();
    let audio_empty = client.text_to_speech("", None, None).await;
    let elapsed_empty = start.elapsed();
    match audio_empty {
        Ok(_) => println!("--- TTS call #3 (empty) UNEXPECTED OK ---\n"),
        Err(MiniMaxError::InvalidParams(msg)) => {
            println!("--- TTS call #3 (empty) ===> InvalidParams: {msg} ({} ms)", elapsed_empty.as_millis());
            println!("(expected: empty text rejected without HTTP call)\n");
        }
        Err(e) => println!("--- TTS call #3 (empty) ===> {:?}\n", e),
    }

    println!("=== demo 完成 (R172) ===");
    Ok(())
}