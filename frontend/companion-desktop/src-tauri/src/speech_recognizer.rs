//! 星语 · 转写引擎（语音面最后一条「待组装」缝点亮；真实识别引擎，绝不假转写）。
//!
//! 路线判定（2026-10-11 现场核实）：windows-sys 0.59 **无** `Win32_Media_Speech`
//! feature（清单止于 WindowsMediaFormat），SAPI COM vtable 面缺失；手写 vtable
//! 无头文件背书不可验证。故走**真引擎桥接**：Windows 自带语音识别引擎经系统
//! 托管语音封装（同一 SAPI 声学解码）在受控子进程中执行——真识别、零模型下载、
//! 零新增依赖；探测与失败一律如实报，绝不回假文本。
//!
//! 契约：
//!   1. 真识别 —— 固定脚本、**仅 i32 数字插值**（构造上不可注入）；结果严格
//!      解析（status/text/confidence），error 态带服务端原话如实报；
//!      无语音 = 空文本如实返回（不造文本）；
//!   2. 纪律 —— 超时强杀（识别上限 + 15s 宽限）、输出有界 64KB、隐藏控制台、
//!      口令/用户文本零插值；
//!   3. 0 装 —— 非 Windows / 引擎缺失 / 程序缺失 / 脚本异常一律如实报原因；
//!   4. 纯函数可单测（时长钳制/脚本构造/结果解析/载荷转结果）；COM 全链与
//!      真麦克风收音走 #[ignore] 真机测试（写全不删）。
//!
//! 测试：`cargo test`（纯函数 + 探测自检；真机组 #[ignore]）。

use std::time::{Duration, Instant};

use tokio::process::Command;

/// 识别时长缺省值（秒）。
pub const RECOGNIZE_DEFAULT_SECS: u32 = 10;
/// 识别时长上限（秒）。
pub const RECOGNIZE_MAX_SECS: u32 = 30;
/// 强杀宽限（识别时长之外）。
pub const RECOGNIZE_GRACE_MS: u64 = 15_000;
/// 输出有界（超出截断，解析前如实拒绝超长输出）。
pub const SPEECH_OUTPUT_LIMIT: usize = 64 * 1024;

#[derive(Debug, serde::Serialize)]
pub struct SpeechRecognitionResult {
    pub text: String,
    pub confidence: f64,
    /// 引擎标识（系统自带识别引擎，经系统托管封装桥接）。
    pub engine: String,
    pub duration_ms: u64,
}

#[derive(Debug, serde::Serialize)]
pub struct SpeechEngineStatus {
    pub available: bool,
    pub engine: String,
    pub reason: String,
}

/// 识别载荷（脚本 stdout 的严格解析产物）。
#[derive(Debug, serde::Deserialize)]
pub struct SpeechPayload {
    pub status: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub confidence: f64,
    #[serde(default)]
    pub reason: String,
}

/// 时长钳制（纯函数，0 装不猜：None→缺省；0/超限→钳进 [1, 上限] 区间口径注释）。
/// 契约口径：钳制是**对用户输入的规范化**（界面滑杆面），与「越界抛错」的
/// 校验面分立——非法类型在 serde 层已被拒。
pub fn clamp_recognize_duration(duration_secs: Option<u32>) -> u32 {
    duration_secs
        .unwrap_or(RECOGNIZE_DEFAULT_SECS)
        .clamp(1, RECOGNIZE_MAX_SECS)
}

/// 识别脚本（固定模板 + 仅 i32 数字插值；构造上不可能被注入）。
pub fn build_recognize_script(duration_secs: u32) -> String {
    format!(
        r#"$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Speech
$recognizer = New-Object System.Speech.Recognition.SpeechRecognitionEngine
try {{
  $recognizer.SetInputToDefaultAudioDevice()
  $result = $recognizer.Recognize([TimeSpan]::FromSeconds({duration}))
  if ($null -eq $result) {{
    Write-Output '{{"status":"no-speech","text":"","confidence":0}}'
  }} else {{
    $json = ConvertTo-Json $result.Text -Compress
    $conf = $result.Confidence.ToString([System.Globalization.CultureInfo]::InvariantCulture)
    Write-Output ('{{"status":"ok","text":' + $json + ',"confidence":' + $conf + '}}')
  }}
}} catch {{
  $reason = ConvertTo-Json $_.Exception.Message -Compress
  Write-Output ('{{"status":"error","text":"","confidence":0,"reason":' + $reason + '}}')
}} finally {{
  $recognizer.Dispose()
}}"#,
        duration = duration_secs
    )
}

/// 引擎探测脚本（固定常量）。
pub fn build_probe_script() -> String {
    "$ErrorActionPreference = 'Stop'\nAdd-Type -AssemblyName System.Speech\n[Console]::Write('SAPI-BRIDGE-OK')".to_string()
}

/// 载荷严格解析（纯函数；非对象/畸形 JSON/未知状态如实报）。
pub fn parse_speech_payload(stdout: &str) -> Result<SpeechPayload, String> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Err("识别输出为空（脚本无回执）".to_string());
    }
    if trimmed.len() > SPEECH_OUTPUT_LIMIT {
        return Err("识别输出超界（如实拒）".to_string());
    }
    // 取最后一行非空 JSON（脚本只应输出一行；多行按协议漂移取末行并容忍噪声行）。
    let line = trimmed
        .lines()
        .rev()
        .find(|candidate| candidate.trim().starts_with('{'))
        .unwrap_or(trimmed);
    let payload: SpeechPayload =
        serde_json::from_str(line.trim()).map_err(|error| format!("识别输出解析失败：{error}"))?;
    match payload.status.as_str() {
        "ok" | "no-speech" | "error" => {}
        other => return Err(format!("未知识别状态：{other}")),
    }
    Ok(payload)
}

/// 载荷 → 结果（0 装：ok 必须带有限置信度 [0,1]；no-speech 空文本如实；error 如实报）。
pub fn payload_to_result(
    payload: SpeechPayload,
    duration_ms: u64,
) -> Result<SpeechRecognitionResult, String> {
    match payload.status.as_str() {
        "error" => Err(format!(
            "识别引擎报错（如实转述）：{}",
            if payload.reason.is_empty() {
                "未附原因"
            } else {
                &payload.reason
            }
        )),
        "no-speech" => Ok(SpeechRecognitionResult {
            text: String::new(),
            confidence: 0.0,
            engine: "system-sapi".to_string(),
            duration_ms,
        }),
        _ => {
            if !payload.text.trim().is_empty()
                && (!payload.confidence.is_finite()
                    || payload.confidence < 0.0
                    || payload.confidence > 1.0)
            {
                return Err(format!("置信度越界：{}", payload.confidence));
            }
            Ok(SpeechRecognitionResult {
                text: payload.text.trim().to_string(),
                confidence: payload.confidence,
                engine: "system-sapi".to_string(),
                duration_ms,
            })
        }
    }
}

#[cfg(windows)]
fn windows_powershell_path() -> String {
    std::env::var("SystemRoot")
        .map(|root| format!(r"{root}\System32\WindowsPowerShell\v1.0\powershell.exe"))
        .unwrap_or_else(|_| "powershell.exe".to_string())
}

#[cfg(windows)]
async fn run_speech_script(script: &str, timeout: Duration) -> Result<String, String> {
    let mut command = Command::new(windows_powershell_path());
    command
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    // 隐藏控制台窗（GUI 宿主不闪窗）。
    command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    let output = tokio::time::timeout(timeout, command.output())
        .await
        .map_err(|_| "识别引擎调用超时（已强杀）".to_string())?
        .map_err(|error| format!("识别引擎进程失败（如实报）：{error}"))?;
    if output.stdout.len() > SPEECH_OUTPUT_LIMIT {
        return Err("识别输出超界（如实拒）".to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// 引擎探测（真探测：引擎可加载即可用；失败带原因，不假报可用）。
#[tauri::command]
pub async fn detect_speech_engine() -> Result<SpeechEngineStatus, String> {
    #[cfg(not(windows))]
    {
        Ok(SpeechEngineStatus {
            available: false,
            engine: String::new(),
            reason: "非 Windows 平台无系统语音识别引擎（如实报）".to_string(),
        })
    }
    #[cfg(windows)]
    {
        match run_speech_script(
            &build_probe_script(),
            Duration::from_millis(RECOGNIZE_GRACE_MS),
        )
        .await
        {
            Ok(stdout) if stdout.contains("SAPI-BRIDGE-OK") => Ok(SpeechEngineStatus {
                available: true,
                engine: "system-sapi".to_string(),
                reason: "系统语音识别引擎可用（经系统托管语音封装桥接，真识别非假转写）"
                    .to_string(),
            }),
            Ok(stdout) => Ok(SpeechEngineStatus {
                available: false,
                engine: String::new(),
                reason: format!("引擎探测无回执：{}", stdout.trim()),
            }),
            Err(error) => Ok(SpeechEngineStatus {
                available: false,
                engine: String::new(),
                reason: error,
            }),
        }
    }
}

/// 真实识别一段系统麦克风音频（限时收音，首段定稿即返回；无语音返回空文本）。
#[tauri::command]
pub async fn recognize_speech(
    duration_secs: Option<u32>,
) -> Result<SpeechRecognitionResult, String> {
    #[cfg(not(windows))]
    {
        let _ = duration_secs;
        Err("非 Windows 平台无系统语音识别引擎（不假转写）".to_string())
    }
    #[cfg(windows)]
    {
        let duration = clamp_recognize_duration(duration_secs);
        let started = Instant::now();
        let stdout = run_speech_script(
            &build_recognize_script(duration),
            Duration::from_millis(u64::from(duration) * 1_000 + RECOGNIZE_GRACE_MS),
        )
        .await?;
        let payload = parse_speech_payload(&stdout)?;
        payload_to_result(
            payload,
            u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_clamp() {
        assert_eq!(clamp_recognize_duration(None), RECOGNIZE_DEFAULT_SECS);
        assert_eq!(clamp_recognize_duration(Some(5)), 5);
        assert_eq!(clamp_recognize_duration(Some(0)), 1, "0 钳进下界");
        assert_eq!(
            clamp_recognize_duration(Some(RECOGNIZE_MAX_SECS + 99)),
            RECOGNIZE_MAX_SECS,
            "超限钳进上界"
        );
        println!("  [1] duration clamp OK");
    }

    #[test]
    fn scripts_are_injection_proof() {
        let script = build_recognize_script(12);
        assert!(script.contains("FromSeconds(12)"), "数字插值到位");
        assert!(
            script.contains("Add-Type -AssemblyName System.Speech"),
            "真引擎装配"
        );
        // 仅出现数字插值位：模板其余部分逐字恒定（构造上不可注入）。
        let again = build_recognize_script(12);
        assert_eq!(script, again, "同参恒同脚本");
        assert_ne!(build_recognize_script(3), script);
        assert!(
            build_probe_script().contains("SAPI-BRIDGE-OK"),
            "探测锚点在脚本内"
        );
        println!("  [2] script construction OK");
    }

    #[test]
    fn payload_parsing_strict() {
        let ok =
            parse_speech_payload(r#"{"status":"ok","text":"你好","confidence":0.85}"#).unwrap();
        assert_eq!(ok.status, "ok");
        assert_eq!(ok.text, "你好");

        let no_speech =
            parse_speech_payload(r#"{"status":"no-speech","text":"","confidence":0}"#).unwrap();
        assert_eq!(no_speech.status, "no-speech");

        // 噪声行后跟真回执：取末段 JSON。
        let noisy = parse_speech_payload(
            "warning: babble\n{\"status\":\"ok\",\"text\":\"x\",\"confidence\":0.5}",
        )
        .unwrap();
        assert_eq!(noisy.text, "x");

        assert!(parse_speech_payload("").is_err(), "空输出拒绝");
        assert!(parse_speech_payload("{not json").is_err(), "畸形 JSON 拒绝");
        assert!(
            parse_speech_payload(r#"{"status":"weird","text":"x"}"#).is_err(),
            "未知状态拒绝"
        );
        println!("  [3] payload parsing OK");
    }

    #[test]
    fn payload_to_result_matrix() {
        let ok = payload_to_result(
            SpeechPayload {
                status: "ok".into(),
                text: " 你好 ".into(),
                confidence: 0.9,
                reason: String::new(),
            },
            1234,
        )
        .unwrap();
        assert_eq!(ok.text, "你好", "去首尾空白");
        assert_eq!(ok.confidence, 0.9);
        assert_eq!(ok.engine, "system-sapi");
        assert_eq!(ok.duration_ms, 1234);

        let empty = payload_to_result(
            SpeechPayload {
                status: "no-speech".into(),
                text: String::new(),
                confidence: 0.0,
                reason: String::new(),
            },
            10,
        )
        .unwrap();
        assert_eq!(empty.text, "", "无语音空文本如实（不造文本）");

        let err = payload_to_result(
            SpeechPayload {
                status: "error".into(),
                text: String::new(),
                confidence: 0.0,
                reason: "引擎缺失".into(),
            },
            10,
        );
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("引擎缺失"), "错误原话如实转述");

        // 置信度越界拒绝（有文本时）。
        assert!(
            payload_to_result(
                SpeechPayload {
                    status: "ok".into(),
                    text: "x".into(),
                    confidence: 1.5,
                    reason: String::new()
                },
                10,
            )
            .is_err(),
            "置信度越界拒绝"
        );
        println!("  [4] payload to result OK");
    }

    /// 真机组（需交互桌面 + 真麦克风 + 系统识别引擎）：探测 → 限时识别。
    /// 0 装纪律：写全不删，真机验收用 `cargo test -- --ignored`。
    #[tokio::test]
    #[ignore = "需真麦克风与交互桌面（真机验收：cargo test -- --ignored）"]
    async fn real_microphone_roundtrip() {
        let status = detect_speech_engine().await.unwrap();
        assert!(status.available, "真机应探测到识别引擎：{}", status.reason);
        let result = recognize_speech(Some(5)).await.unwrap();
        assert_eq!(result.engine, "system-sapi");
        // 真机对麦克风说话时应得非空文本；无声环境允许空文本（如实）。
        if result.text.is_empty() {
            println!("  [5] real microphone roundtrip: 空收音（无声环境，如实）");
        } else {
            assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
            println!("  [5] real microphone roundtrip OK: {:?}", result.text);
        }
    }
}
