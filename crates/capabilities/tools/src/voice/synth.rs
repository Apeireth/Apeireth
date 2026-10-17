//! 合成引擎面 —— 文本 → 系统语音引擎合成即播 (Windows SAPI 桥)。
//!
//! # 桥接姿势 (与语音输入侧同源)
//!
//! Windows 系统语音合成引擎经**系统托管语音封装**在受控子进程中执行 ——
//! 真合成、零模型下载、零新增依赖。纪律与输入侧的识别桥同款:
//!
//! 1. **零插值脚本**: 合成脚本是**恒定常量**, 请求 (文本 / 嗓名 / 语速 / 音量)
//!    走 **stdin JSON 数据通道** —— 脚本层构造上不可能被注入 (对比输入侧的
//!    「仅 i32 数字插值」, 本面连数字插值都没有);
//! 2. **受控子进程**: 隐藏控制台窗 / 超时强杀 / 输出有界 64KB / `kill_on_drop`;
//! 3. **真停清理**: 播放句柄 [`SapiPlayback`] 的 `Drop` = 杀子进程并回收
//!    ([`SpeechPlayback`] 的 Drop 契约), 杜绝僵尸播放;
//! 4. **0 装**: 非 Windows / 引擎缺失 / 无嗓 / 脚本异常一律**错误即帧**如实报,
//!    绝不假报已说 —— 与输入侧「空结果不造文本」同口径 (合成失败不造发声)。
//!
//! # 可注入的引擎面
//!
//! 合成执行面是 trait ([`SynthEngine`]): 生产面 = 系统引擎桥
//! ([`system_synth_engine`]); 测试面 = 假引擎注入 ([`crate::voice::mock`]),
//! 行为可钉死 (立即播完 / 挂住等停 / 合成失败)。

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

#[cfg(windows)]
use std::time::{Duration, Instant};

/// 系统语音合成引擎的稳定标识 (账本 / 输出帧的 `engine` 字段)。
pub const ENGINE_SYSTEM_SAPI: &str = "system-sapi";

/// 引擎缺席面 (非 Windows 平台) 的稳定标识。
pub const ENGINE_ABSENT: &str = "absent";

/// 引擎探测回执锚点 (探测脚本与解析共用同一常量)。
pub const SYNTH_PROBE_ANCHOR: &str = "SAPI-SYNTH-OK";

/// 受控子进程输出上限 (超出拒绝即帧, 不静默截断)。
pub const SYNTH_OUTPUT_LIMIT: usize = 64 * 1024;

/// 探测 / 已装嗓清单的执行时限 (毫秒)。
pub const SYNTH_PROBE_TIMEOUT_MS: u64 = 15_000;

/// 一次合成即播的请求 (参数已过 [`crate::voice::speak`] 的边界校验)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SynthRequest {
    /// 合成文本 (全文只进账本; 引擎面收到的是它的数据通道副本)。
    pub text: String,
    /// 嗓名; `None` = 系统默认嗓。
    pub voice: Option<String>,
    /// 语速 (SAPI 面 `[-10, 10]`, 已校验)。
    pub rate: i32,
    /// 音量 (SAPI 面 `[0, 100]`, 已校验)。
    pub volume: u32,
}

/// 一次播放的终态 (错误即帧的分类来源)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackEnd {
    /// 播完 (实际发音时长)。
    Completed {
        /// 实际发音时长 (毫秒)。
        duration_ms: u64,
    },
    /// 被真停 (新话打断 / 外部停令 / 调用被取消): 已发的部分如实记时长。
    Stopped {
        /// 被停时已发音的时长 (毫秒)。
        duration_ms: u64,
    },
    /// 合成失败 (引擎报错 / 无嗓 / 回执异常): 错误即帧, 不假报已说。
    Failed {
        /// 失败时已过时长 (毫秒)。
        duration_ms: u64,
        /// 引擎侧真因 (如实转述)。
        reason: String,
    },
}

impl PlaybackEnd {
    /// 实际发音/经过时长 (毫秒)。
    pub const fn duration_ms(&self) -> u64 {
        match self {
            Self::Completed { duration_ms }
            | Self::Stopped { duration_ms }
            | Self::Failed { duration_ms, .. } => *duration_ms,
        }
    }

    /// 稳定状态标签 (`spoken` / `interrupted` / `failed`)。
    pub const fn status(&self) -> &'static str {
        match self {
            Self::Completed { .. } => "spoken",
            Self::Stopped { .. } => "interrupted",
            Self::Failed { .. } => "failed",
        }
    }
}

/// 一条在播语句的受控播放句柄。
///
/// # Drop 契约 (真停清理)
///
/// 实现**必须**在 `Drop` 时执行 [`SpeechPlayback::stop`] 语义 —— 句柄没了,
/// 声音也必须停 (杜绝僵尸播放)。上层的 [`crate::voice::playback::SpeechTicket`]
/// 据此保证: 调用被取消 / 超时 / 正常收尾, 三条路都留下零活句柄。
pub trait SpeechPlayback: Send + Sync {
    /// 承载这条播放的引擎标识。
    fn engine_id(&self) -> &'static str;

    /// 是否仍在发音。
    fn is_running(&self) -> bool;

    /// 非阻塞收尾读取: 已结束给终态, 未结束给 `None`。
    fn poll_end(&self) -> Option<PlaybackEnd>;

    /// 真停 (幂等): 打断发音并回收底层句柄。
    fn stop(&self);

    /// 已发音时长 (毫秒)。
    fn elapsed_ms(&self) -> u64;
}

/// 引擎现状 (真探测语义: 缺席/失败如实报, 不假报可用)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineStatus {
    /// 引擎是否可用。
    pub available: bool,
    /// 现状说明 (可用/缺席/失败的真因)。
    pub reason: String,
}

/// 合成执行面 (可注入: 生产 = 系统引擎桥, 测试 = 假引擎)。
pub trait SynthEngine: Send + Sync {
    /// 引擎稳定标识。
    fn engine_id(&self) -> &'static str;

    /// 引擎现状 (真探测或如实缺席)。
    fn status(&self) -> EngineStatus;

    /// 系统已装嗓清单 (选嗓校验的唯一事实源; 失败给真因)。
    fn installed_voices(&self) -> Result<Vec<String>, String>;

    /// 合成即播: 返回受控播放句柄 (Drop = 真停清理)。
    fn speak(&self, request: &SynthRequest) -> Result<Arc<dyn SpeechPlayback>, String>;
}

// ---------------------------------------------------------------------------
// 脚本面 (恒定常量 + 回执解析): 纯函数, 构造上不可能被注入
// ---------------------------------------------------------------------------

/// 合成脚本 (恒定常量, 零插值)。
///
/// 请求从 stdin 读一行 JSON (`{text, voice, rate, volume}`), `ConvertFrom-Json`
/// 解析成对象后逐字段取值 —— 文本/嗓名是**数据**不是代码, 注入面为零。
/// stdin/stdout 显式锁 UTF-8: 控制台缺省按本地代码页解 stdin, 不锁会把
/// 多字节文本解成乱码再喂 JSON 解析 (真机验收抓到过这一帧, 帧如实报了
/// 「合成输出为空」)。
pub fn build_speak_script() -> String {
    r#"$ErrorActionPreference = 'Stop'
[Console]::InputEncoding = [System.Text.Encoding]::UTF8
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Add-Type -AssemblyName System.Speech
$raw = [Console]::In.ReadLine()
if ($null -eq $raw) {
  Write-Output '{"status":"error","reason":"no request line on stdin"}'
  exit 0
}
$req = $raw | ConvertFrom-Json
$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer
try {
  if ($null -ne $req.voice -and [string]$req.voice -ne '') {
    $synth.SelectVoice([string]$req.voice)
  }
  $synth.Rate = [int]$req.rate
  $synth.Volume = [int]$req.volume
  $synth.Speak([string]$req.text)
  Write-Output '{"status":"ok"}'
} catch {
  $reason = ConvertTo-Json $_.Exception.Message -Compress
  Write-Output ('{"status":"error","reason":' + $reason + '}')
} finally {
  $synth.Dispose()
}"#
    .to_string()
}

/// 已装嗓清单脚本 (恒定常量, 零插值): 每行一个嗓名; 失败行 `ERROR:` 前缀。
/// stdout 显式锁 UTF-8 (嗓名可含多字节字符)。
pub fn build_voices_script() -> String {
    r#"$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Add-Type -AssemblyName System.Speech
$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer
try {
  foreach ($voice in $synth.GetInstalledVoices()) {
    Write-Output $voice.VoiceInfo.Name
  }
} catch {
  Write-Output ('ERROR:' + $_.Exception.Message)
} finally {
  $synth.Dispose()
}"#
    .to_string()
}

/// 引擎探测脚本 (恒定常量): 引擎可加载即给锚点回执。
pub fn build_probe_script() -> String {
    format!(
        "$ErrorActionPreference = 'Stop'\nAdd-Type -AssemblyName System.Speech\n[Console]::Write('{SYNTH_PROBE_ANCHOR}')"
    )
}

/// 合成回执 (脚本 stdout 的严格解析产物)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SynthPayload {
    /// `ok` / `error`。
    pub status: String,
    /// `error` 态的引擎真因 (如实转述)。
    #[serde(default)]
    pub reason: String,
}

/// 合成回执严格解析 (纯函数; 空输出/畸形/未知状态如实拒)。
pub fn parse_synth_payload(stdout: &str) -> Result<SynthPayload, String> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Err("合成输出为空 (脚本无回执, 错误即帧)".to_string());
    }
    if trimmed.len() > SYNTH_OUTPUT_LIMIT {
        return Err("合成输出超界 (如实拒)".to_string());
    }
    // 取末段非空 JSON (脚本只应输出一行; 多行按协议漂移取末行并容忍噪声行)。
    let line = trimmed
        .lines()
        .rev()
        .find(|candidate| candidate.trim().starts_with('{'))
        .unwrap_or(trimmed);
    let payload: SynthPayload =
        serde_json::from_str(line.trim()).map_err(|error| format!("合成输出解析失败: {error}"))?;
    match payload.status.as_str() {
        "ok" | "error" => Ok(payload),
        other => Err(format!("未知合成状态: {other}")),
    }
}

/// 已装嗓清单解析 (纯函数): 逐行嗓名; `ERROR:` 行 = 引擎真因 (如实拒)。
pub fn parse_voices(stdout: &str) -> Result<Vec<String>, String> {
    let mut voices = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(reason) = trimmed.strip_prefix("ERROR:") {
            return Err(format!("已装嗓清单读取失败 (如实转述): {}", reason.trim()));
        }
        voices.push(trimmed.to_string());
    }
    Ok(voices)
}

/// 一次合成请求的 stdin 数据行 (JSON; 文本/嗓名是数据通道载荷, 不进脚本)。
pub fn synth_request_line(request: &SynthRequest) -> Result<String, String> {
    serde_json::to_string(request).map_err(|error| format!("合成请求序列化失败: {error}"))
}

// ---------------------------------------------------------------------------
// Windows SAPI 桥执行面 (受控子进程; Drop = 真停)
// ---------------------------------------------------------------------------

#[cfg(windows)]
fn windows_powershell_path() -> String {
    std::env::var("SystemRoot")
        .map(|root| format!(r"{root}\System32\WindowsPowerShell\v1.0\powershell.exe"))
        .unwrap_or_else(|_| "powershell.exe".to_string())
}

/// 一个受控合成子进程的播放句柄 (Drop = 真停清理: 杀子进程 + 回收)。
#[cfg(windows)]
pub struct SapiPlayback {
    engine_id: &'static str,
    started: Instant,
    state: Mutex<SapiPlaybackState>,
}

#[cfg(windows)]
enum SapiPlaybackState {
    Running {
        child: std::process::Child,
        stdout: std::process::ChildStdout,
        stopped: bool,
    },
    Ended(PlaybackEnd),
}

#[cfg(windows)]
impl SapiPlayback {
    fn new(child: std::process::Child, stdout: std::process::ChildStdout) -> Self {
        Self {
            engine_id: ENGINE_SYSTEM_SAPI,
            started: Instant::now(),
            state: Mutex::new(SapiPlaybackState::Running {
                child,
                stdout,
                stopped: false,
            }),
        }
    }

    /// 进程收尾: 读有界回执 → 终态 (真因如实转述)。
    fn finish(child: &mut std::process::Child, stdout: &mut std::process::ChildStdout) -> String {
        use std::io::Read;
        let mut buffer = Vec::new();
        let _ = stdout.read_to_end(&mut buffer);
        let _ = child.wait();
        if buffer.len() > SYNTH_OUTPUT_LIMIT {
            return String::new();
        }
        String::from_utf8_lossy(&buffer).into_owned()
    }
}

#[cfg(windows)]
impl SpeechPlayback for SapiPlayback {
    fn engine_id(&self) -> &'static str {
        self.engine_id
    }

    fn is_running(&self) -> bool {
        matches!(
            &*self.state.lock().unwrap_or_else(|p| p.into_inner()),
            SapiPlaybackState::Running { .. }
        )
    }

    fn poll_end(&self) -> Option<PlaybackEnd> {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let duration_ms = self.started.elapsed().as_millis() as u64;
        match &mut *state {
            SapiPlaybackState::Ended(end) => Some(end.clone()),
            SapiPlaybackState::Running {
                child,
                stdout,
                stopped,
            } => {
                let exited = matches!(child.try_wait(), Ok(Some(_)) | Err(_));
                if !exited {
                    return None;
                }
                let stdout_text = Self::finish(child, stdout);
                let end = if *stopped {
                    PlaybackEnd::Stopped { duration_ms }
                } else {
                    match parse_synth_payload(&stdout_text) {
                        Ok(payload) if payload.status == "ok" => {
                            PlaybackEnd::Completed { duration_ms }
                        }
                        Ok(payload) => PlaybackEnd::Failed {
                            duration_ms,
                            reason: if payload.reason.is_empty() {
                                "引擎报错未附原因".to_string()
                            } else {
                                payload.reason
                            },
                        },
                        Err(error) => PlaybackEnd::Failed {
                            duration_ms,
                            reason: error,
                        },
                    }
                };
                *state = SapiPlaybackState::Ended(end.clone());
                Some(end)
            }
        }
    }

    fn stop(&self) {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let duration_ms = self.started.elapsed().as_millis() as u64;
        match &mut *state {
            SapiPlaybackState::Ended(_) => {}
            SapiPlaybackState::Running {
                child,
                stdout,
                stopped,
            } => {
                *stopped = true;
                let _ = child.kill();
                let _ = Self::finish(child, stdout);
                *state = SapiPlaybackState::Ended(PlaybackEnd::Stopped { duration_ms });
            }
        }
    }

    fn elapsed_ms(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }
}

#[cfg(windows)]
impl Drop for SapiPlayback {
    /// 真停清理 (Drop 契约): 句柄没了, 声音也必须停 —— 杜绝僵尸播放。
    fn drop(&mut self) {
        self.stop();
    }
}

/// Windows 系统语音合成引擎桥 (受控子进程 + 零插值脚本 + stdin 数据通道)。
#[cfg(windows)]
pub struct SapiSpeechEngine {
    powershell: String,
}

#[cfg(windows)]
impl SapiSpeechEngine {
    /// 系统 PowerShell 路径构造 (SystemRoot 锚定, 缺省回退 PATH)。
    pub fn new() -> Self {
        Self {
            powershell: windows_powershell_path(),
        }
    }

    /// 探测 / 已装嗓清单的一次受控执行 (有界输出 + 时限强杀)。
    fn run_capture(&self, script: &str, timeout: Duration) -> Result<String, String> {
        use std::io::Read;
        use std::os::windows::process::CommandExt;
        use std::process::{Command, Stdio};
        let mut child = Command::new(&self.powershell)
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                script,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW: 宿主不闪窗
            .spawn()
            .map_err(|error| format!("合成引擎子进程启动失败 (如实报): {error}"))?;
        // 数据通道关闭 = 脚本的 ReadLine 立即拿到空 (无请求行, 如实报错)。
        drop(child.stdin.take());
        let deadline = Instant::now() + timeout;
        let mut buffer = Vec::new();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) => {
                    if Instant::now() >= deadline {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err("合成引擎探测超时 (已强杀, 如实报)".to_string());
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("合成引擎探测失败 (如实报): {error}"));
                }
            }
        }
        if let Some(mut stdout) = child.stdout.take() {
            let _ = stdout.read_to_end(&mut buffer);
        }
        let _ = child.wait();
        if buffer.len() > SYNTH_OUTPUT_LIMIT {
            return Err("合成引擎输出超界 (如实拒)".to_string());
        }
        Ok(String::from_utf8_lossy(&buffer).into_owned())
    }
}

#[cfg(windows)]
impl Default for SapiSpeechEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(windows)]
impl SynthEngine for SapiSpeechEngine {
    fn engine_id(&self) -> &'static str {
        ENGINE_SYSTEM_SAPI
    }

    fn status(&self) -> EngineStatus {
        match self.run_capture(
            &build_probe_script(),
            Duration::from_millis(SYNTH_PROBE_TIMEOUT_MS),
        ) {
            Ok(stdout) if stdout.contains(SYNTH_PROBE_ANCHOR) => EngineStatus {
                available: true,
                reason: "系统语音合成引擎可用 (经系统托管语音封装桥接, 真合成非假发声)".to_string(),
            },
            Ok(stdout) => EngineStatus {
                available: false,
                reason: format!("引擎探测无回执: {}", stdout.trim()),
            },
            Err(error) => EngineStatus {
                available: false,
                reason: error,
            },
        }
    }

    fn installed_voices(&self) -> Result<Vec<String>, String> {
        let stdout = self.run_capture(
            &build_voices_script(),
            Duration::from_millis(SYNTH_PROBE_TIMEOUT_MS),
        )?;
        parse_voices(&stdout)
    }

    fn speak(&self, request: &SynthRequest) -> Result<Arc<dyn SpeechPlayback>, String> {
        use std::io::Write;
        use std::os::windows::process::CommandExt;
        use std::process::{Command, Stdio};
        let line = synth_request_line(request)?;
        let mut child = Command::new(&self.powershell)
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &build_speak_script(),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW: 宿主不闪窗
            .spawn()
            .map_err(|error| format!("合成引擎子进程启动失败 (如实报): {error}"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "合成引擎子进程无输出通道 (如实报)".to_string())?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| "合成引擎子进程无输入通道 (如实报)".to_string())?;
        if let Err(error) = stdin
            .write_all(line.as_bytes())
            .and_then(|()| stdin.write_all(b"\n"))
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("合成请求写入失败 (如实报): {error}"));
        }
        drop(stdin);
        Ok(Arc::new(SapiPlayback::new(child, stdout)))
    }
}

// ---------------------------------------------------------------------------
// 缺席面 (非 Windows): 每次调用错误即帧, 不假嗓不静默
// ---------------------------------------------------------------------------

/// 引擎缺席面: 平台无系统语音合成引擎时的如实实现 (错误即帧, 绝不假发声)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbsentSynthEngine {
    reason: String,
}

impl AbsentSynthEngine {
    /// 缺席原因 (如实文案)。
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl SynthEngine for AbsentSynthEngine {
    fn engine_id(&self) -> &'static str {
        ENGINE_ABSENT
    }

    fn status(&self) -> EngineStatus {
        EngineStatus {
            available: false,
            reason: self.reason.clone(),
        }
    }

    fn installed_voices(&self) -> Result<Vec<String>, String> {
        Err(self.reason.clone())
    }

    fn speak(&self, _request: &SynthRequest) -> Result<Arc<dyn SpeechPlayback>, String> {
        Err(self.reason.clone())
    }
}

/// 平台系统合成引擎: Windows = SAPI 桥; 其余平台 = 缺席面 (错误即帧)。
pub fn system_synth_engine() -> Arc<dyn SynthEngine> {
    #[cfg(windows)]
    {
        Arc::new(SapiSpeechEngine::new())
    }
    #[cfg(not(windows))]
    {
        Arc::new(AbsentSynthEngine::new(
            "非 Windows 平台无系统语音合成引擎 (如实报, 不假嗓)",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> SynthRequest {
        SynthRequest {
            text: "你好".to_string(),
            voice: Some("test-voice".to_string()),
            rate: 2,
            volume: 80,
        }
    }

    #[test]
    fn scripts_are_constants_with_zero_interpolation() {
        // 合成脚本是恒定常量: 同步构造逐字节相同, 请求不进脚本 (stdin 数据通道)。
        assert_eq!(build_speak_script(), build_speak_script());
        assert_eq!(build_voices_script(), build_voices_script());
        assert!(
            build_probe_script().contains(SYNTH_PROBE_ANCHOR),
            "探测锚点在脚本内"
        );
        let script = build_speak_script();
        assert!(
            script.contains("[Console]::In.ReadLine()"),
            "请求走 stdin 数据通道"
        );
        assert!(
            script.contains("[Console]::InputEncoding = [System.Text.Encoding]::UTF8")
                && script.contains("[Console]::OutputEncoding = [System.Text.Encoding]::UTF8"),
            "stdin/stdout 显式锁 UTF-8 (多字节文本不许按本地代码页解)"
        );
        assert!(
            build_voices_script()
                .contains("[Console]::OutputEncoding = [System.Text.Encoding]::UTF8"),
            "已装嗓清单同样锁 UTF-8"
        );
        assert!(
            !script.contains("你好") && !script.contains("test-voice"),
            "文本/嗓名零插值 (构造上不可能被注入)"
        );
    }

    #[test]
    fn request_line_carries_text_as_json_data() {
        let line = synth_request_line(&request()).expect("serialize");
        let parsed: SynthRequest = serde_json::from_str(&line).expect("roundtrip");
        assert_eq!(parsed, request());
    }

    #[test]
    fn synth_payload_parsing_is_strict() {
        let ok = parse_synth_payload(r#"{"status":"ok"}"#).expect("ok");
        assert_eq!(ok.status, "ok");

        let noisy = parse_synth_payload("noise\n{\"status\":\"error\",\"reason\":\"无嗓\"}")
            .expect("take last json line");
        assert_eq!(noisy.reason, "无嗓");

        assert!(parse_synth_payload("").is_err(), "空输出拒绝即帧");
        assert!(parse_synth_payload("{not json").is_err(), "畸形拒");
        assert!(
            parse_synth_payload(r#"{"status":"weird"}"#).is_err(),
            "未知状态拒"
        );
    }

    #[test]
    fn voices_parsing_reports_engine_reasons_honestly() {
        assert_eq!(
            parse_voices("嗓一\n嗓二\n").expect("voices"),
            vec!["嗓一".to_string(), "嗓二".to_string()]
        );
        let error = parse_voices("ERROR:引擎缺失").expect_err("ERROR 行如实拒");
        assert!(error.contains("引擎缺失"), "{error}");
    }

    #[test]
    fn absent_engine_refuses_rather_than_faking_a_voice() {
        let engine = AbsentSynthEngine::new("无嗓 (如实报)");
        assert!(!engine.status().available);
        assert!(engine.installed_voices().is_err());
        assert!(engine.speak(&request()).is_err(), "缺席面错误即帧");
        assert_eq!(engine.engine_id(), ENGINE_ABSENT);
    }

    #[test]
    fn playback_end_labels_are_stable() {
        assert_eq!(PlaybackEnd::Completed { duration_ms: 1 }.status(), "spoken");
        assert_eq!(
            PlaybackEnd::Stopped { duration_ms: 2 }.status(),
            "interrupted"
        );
        assert_eq!(
            PlaybackEnd::Failed {
                duration_ms: 3,
                reason: "x".into()
            }
            .status(),
            "failed"
        );
    }
}
