//! # `apeireth-voice` R20 阶段 6 flesh out: 真接 TTS / STT / 唤醒词 / 声纹 4 块 demo
//!
//! 演示 `VoiceRealImpl` 真接 4 块 (TTS / STT / 唤醒词 / 声纹).
//! 跟 `voice_stub_demo` (STUB 模式) 严格分离, 是显式 opt-in 真接路径.
//!
//! **注意**: 本 demo 默认 `base_url` 指向 `https://api.apeireth.com/v1` (假想生产).
//! 真跑前需:
//! 1. 在 Apeireth 控制台创建应用, 拿 `api_key`
//! 2. 通过环境变量 `APEIRETH_VOICE_API_KEY` 注入
//! 3. 给应用授权 voice API 权限 (TTS / STT / 声纹)
//!
//! 不传环境变量时, demo 用 `VoiceRealImpl::new` 空 api_key 跑, 调用时返 AuthFailed.
//!
//! ## 运行
//!
//! ```bash
//! cargo run -p apeireth-voice --example voice_real_demo
//! # 或带环境变量:
//! APEIRETH_VOICE_API_KEY=sk-xxx cargo run -p apeireth-voice --example voice_real_demo
//! ```
//!
//! ## 输出 (无 env 时)
//!
//! ```text
//! [voice_real_demo] VoiceRealImpl 创建: base_url=https://api.apeireth.com/v1 api_key_cached=false
//! [voice_real_demo] detect_wake_word (STUB) -> Ok(WakeWord { keyword: "apeireth", ... })
//! [voice_real_demo] text_to_speech -> Err(RecordingFailed("network: ...")
//! [voice_real_demo] 演示完成 (R20 阶段 6 flesh out 真接实现已 ready, Mavis 整合 #3 拍板后切 STUB_MODE=false)
//! ```
//!
//! ## 6 哲学锚穿透 (per 蓝图 §1)
//!
//! - **S-1**: 1:1 翻译远端 voice API URL + 方法, 跟 `real.rs` 同款
//! - **S-2**: demo 不假装"调通", 无环境变量时如实返 AuthFailed
//! - **O-3**: 1 文件覆盖 4 块调用入口
//! - **O-5**: 缺 api_key 时如实 demo 失败, 不假装成功

use apeireth_voice::{
    AudioBuffer, Lang, VoiceConfig, VoiceError, VoiceKind, VoiceRealImpl, VOICE_API_BASE_URL,
    VOICE_SAMPLE_RATE_HZ,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    // 1) 配 VoiceRealImpl (从 env 读 api_key, 失败用空字符串)
    let api_key = std::env::var("APEIRETH_VOICE_API_KEY").unwrap_or_default();
    let real = VoiceRealImpl::new(VoiceConfig::default(), VOICE_API_BASE_URL, &api_key)?;

    println!(
        "[voice_real_demo] VoiceRealImpl 创建: base_url={} api_key_cached={}",
        real.base_url(),
        real.api_key_cached()
    );

    // 2) 演示 唤醒词 (STUB, 0 网络调用, 必 Ok)
    let audio_1s = AudioBuffer::from_samples(vec![0i16; VOICE_SAMPLE_RATE_HZ as usize]);
    let r1 = real.detect_wake_word(&audio_1s).await;
    println!("[voice_real_demo] detect_wake_word (STUB) -> {:?}", short_res(&r1));

    // 3) 演示 TTS (无 env 时会 401 / network error, 实事求是)
    let r2 = real
        .text_to_speech("hello apeireth, this is a TTS test", VoiceKind::ApeirethMale)
        .await;
    println!("[voice_real_demo] text_to_speech -> {:?}", short_res(&r2));

    // 4) 演示 STT
    let r3 = real.speech_to_text(&audio_1s, Lang::Zh).await;
    println!("[voice_real_demo] speech_to_text -> {:?}", short_res(&r3));

    // 5) 演示 声纹
    let r4 = real
        .voiceprint_match(&audio_1s, "u_apeireth_demo")
        .await;
    println!("[voice_real_demo] voiceprint_match -> {:?}", short_res(&r4));

    // 6) 演示 TTS too long (K-1 强校验, 不发 HTTP)
    let big_text = "a".repeat(4096 + 1);
    let r5 = real
        .text_to_speech(&big_text, VoiceKind::ApeirethFemale)
        .await;
    println!("[voice_real_demo] text_to_speech (too long) -> {:?}", short_res(&r5));

    // 7) 演示 STT empty audio
    let empty = AudioBuffer::from_samples(vec![]);
    let r6 = real.speech_to_text(&empty, Lang::En).await;
    println!("[voice_real_demo] speech_to_text (empty) -> {:?}", short_res(&r6));

    // 8) 演示 声纹 empty claimed_id
    let r7 = real.voiceprint_match(&audio_1s, "").await;
    println!("[voice_real_demo] voiceprint_match (empty id) -> {:?}", short_res(&r7));

    println!(
        "[voice_real_demo] 演示完成 (R20 阶段 6 flesh out 真接实现已 ready, Mavis 整合 #3 拍板后切 STUB_MODE=false)"
    );
    Ok(())
}

/// 短格式化结果 (不打印全 body / 全 samples, 防 noise).
fn short_res<T: std::fmt::Debug>(r: &Result<T, VoiceError>) -> String {
    match r {
        Ok(_) => "Ok".to_string(),
        Err(e) => {
            let s = format!("{e:?}");
            if s.len() > 100 {
                format!("{}...(truncated)", &s[..100])
            } else {
                s
            }
        }
    }
}
