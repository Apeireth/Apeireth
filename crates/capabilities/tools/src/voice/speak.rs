//! 发声工具面 (`tool.voice_speak`, 声明名 `voice_speak`) —— 文本 → 系统语音
//! 引擎合成即播 (通用嗓先行, 定制音色 = 资产活后置)。
//!
//! # 管线 (AI 先用: 它能让自己说话)
//!
//! 1. **参数校验** (拒绝即帧, 不钳制不猜): 文本非空且限长 / 语速
//!    `[-10, 10]` / 音量 `[0, 100]` / 嗓名须在**系统已装嗓清单**内;
//! 2. **安静纪律**: 勿扰窗口内**错误即帧** (抑制出声, 不静默吞请求、
//!    不假报已说; 风险钩子与工具面双保险, 同一窗口同一文案);
//! 3. **合成即播** ([`crate::voice::synth`]): 系统语音引擎受控子进程,
//!    文本走 stdin 数据通道 (脚本零插值);
//! 4. **受控播放** ([`crate::voice::playback`]): 同时只播一条, 新话打断
//!    旧话可配; 播放票据 Drop = 真停 (取消/超时不留僵尸播放);
//! 5. **语句账本** ([`crate::voice::ledger`]): `spoken.jsonl` append-only
//!    记「说了什么/何时/哪把嗓/时长」。
//!
//! # 治理 (发声面档: 与 shell 的执行档**分立**, 不借壳)
//!
//! - **注册即闸**: 组装根只在 `APEIRETH_ENABLE_VOICE_SPEAK=1` 下注册本工具
//!   并 grant ([`authorized_voice_speak_policy`]) —— 默认关, fail-closed;
//! - **发声面默认档 = 免逐次人批**: 声音是输出面 (无任意文件写: 唯一落盘是
//!   固定路径语句账本的追加行; 无网络), 破坏力档与 shell 的「每次人批」
//!   执行档**不同档** —— 拦截面换成: 文本限长 + 参数边界拒 + 勿扰窗口 +
//!   独占播放 + 账本全责记录。未 grant = 拒绝即帧 (不降级成审批, 不借壳);
//! - **单调边界 Guard** ([`VoiceSpeakBoundaryGuard`]): 坏参数 / 勿扰窗口
//!    一律拒绝即帧, 任何后续阶段不得翻回。
//!
//! # 脱敏口径 (如实定档)
//!
//! 合成文本**全文只进语句账本**; 错误帧 / 输出帧 / 审批载荷只带字符数、
//! 嗓名、时长、状态 —— 文本原文不上任何日志面。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::CapabilityId;
use apeireth_governance::{Decision, PermissionPolicy, ToolGuard, ToolGuardRequest};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::exec_pipeline::{
    AroundPolicy, OutputSchema, PreExecuteHook, PreExecuteRequest, PreVerdict, SchemaKind,
};
use crate::voice::ledger::{
    spoken_utterance_id, SpokenLedger, SpokenStatus, SpokenUtterance, DEFAULT_VOICE_LABEL,
    SPOKEN_LEDGER_FILE,
};
use crate::voice::playback::{
    begin_speech, wait_playback, PlaybackController, PlaybackPolicy, SpeechTicket, TryBeginError,
};
use crate::voice::quiet::{QuietClock, QuietHours};
use crate::voice::synth::{SynthEngine, SynthRequest};

/// 稳定能力标识 (注册表 / 治理授权 / 风险映射共用)。
pub const VOICE_SPEAK_CAPABILITY_ID: &str = "tool.voice_speak";

/// 模型侧声明名 (短名, 每次请求都进上下文)。
pub const VOICE_SPEAK_TOOL_NAME: &str = "voice_speak";

/// 生产执行时限 (毫秒): 覆盖限长文本在慢语速下的整段发声。
pub const VOICE_SPEAK_TIMEOUT_MS: u64 = 240_000;

/// 时限闸的接受上限 (毫秒)。
pub const VOICE_SPEAK_MAX_TIMEOUT_MS: u64 = 300_000;

/// 单次发声文本上限 (字符; 超出拒绝即帧 —— 长话请分句说)。
pub const VOICE_SPEAK_MAX_TEXT_CHARS: usize = 2_000;

/// 语速下限 (SAPI 面)。
pub const VOICE_RATE_MIN: i32 = -10;

/// 语速上限 (SAPI 面)。
pub const VOICE_RATE_MAX: i32 = 10;

/// 音量下限 (SAPI 面)。
pub const VOICE_VOLUME_MIN: u32 = 0;

/// 音量上限 (SAPI 面)。
pub const VOICE_VOLUME_MAX: u32 = 100;

/// 缺省语速 (引擎默认)。
pub const VOICE_RATE_DEFAULT: i32 = 0;

/// 缺省音量 (满格)。
pub const VOICE_VOLUME_DEFAULT: u32 = 100;

/// 工具描述全文 (注册声明与文档引用同一份文本)。
pub const VOICE_SPEAK_TOOL_DESCRIPTION: &str = "Speak one utterance aloud through the \
  system speech engine (synthesis and playback are one controlled call). Parameters: \
  `text` (required, non-empty, at most 2000 characters), `rate` (optional speech rate \
  in [-10, 10], refused outside the range), `volume` (optional loudness in [0, 100], \
  refused outside the range), `voice` (optional name from the installed system voice \
  list; omit it for the system default voice; an unknown name is refused and the \
  refusal lists the installed voices). Playback is exclusive: at most one utterance \
  plays at a time, and a new utterance either interrupts the previous one or queues \
  behind it per configuration. Failures are honest error frames: no engine, no \
  installed voice, synthesis errors, and do-not-disturb windows never pretend the \
  utterance was spoken. Every spoken (or half-spoken) utterance is appended to the \
  `spoken.jsonl` accountability ledger.";

/// 工具调用参数 (未知字段拒绝)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceSpeakParams {
    /// 要说的文本 (全文只进语句账本)。
    pub text: String,
    /// 语速 (`[-10, 10]`; 缺省 = 引擎默认)。
    #[serde(default)]
    pub rate: Option<i32>,
    /// 音量 (`[0, 100]`; 缺省 = 满格)。
    #[serde(default)]
    pub volume: Option<u32>,
    /// 嗓名 (系统已装嗓清单内的名字; 缺省 = 系统默认嗓)。
    #[serde(default)]
    pub voice: Option<String>,
}

impl VoiceSpeakParams {
    /// 解析后的生效语速。
    pub const fn effective_rate(&self) -> i32 {
        match self.rate {
            Some(rate) => rate,
            None => VOICE_RATE_DEFAULT,
        }
    }

    /// 解析后的生效音量。
    pub const fn effective_volume(&self) -> u32 {
        match self.volume {
            Some(volume) => volume,
            None => VOICE_VOLUME_DEFAULT,
        }
    }

    /// 文本字符数 (输出帧 / 账本口径; 不是字节数)。
    pub fn text_chars(&self) -> usize {
        self.text.chars().count()
    }
}

/// 参数校验 (纯判定, 零 IO): 返回 `Some(拒绝即帧文案)` 即拒绝。
///
/// 逐项: ① 文本去空白后非空且限长; ② 语速在 `[-10, 10]` (越界**拒绝**,
/// 不钳制 —— 钳制会把「说错参数」洗成「说得怪」); ③ 音量在 `[0, 100]`;
/// ④ 给了嗓就必须给非空嗓名。文案只带字符数/取值, **不回显文本原文**。
pub fn speak_request_refusal(params: &VoiceSpeakParams) -> Option<String> {
    let text = params.text.trim();
    if text.is_empty() {
        return Some("拒绝发声: 文本为空 (空文本不发声, 如实拒)".to_string());
    }
    let chars = params.text.chars().count();
    if chars > VOICE_SPEAK_MAX_TEXT_CHARS {
        return Some(format!(
            "拒绝发声: 文本超限 ({chars} 字符, 上限 {VOICE_SPEAK_MAX_TEXT_CHARS}; 长话请分句说)"
        ));
    }
    if let Some(rate) = params.rate {
        if !(VOICE_RATE_MIN..=VOICE_RATE_MAX).contains(&rate) {
            return Some(format!(
                "拒绝发声: 语速越界 ({rate}, 合法区间 [{VOICE_RATE_MIN}, {VOICE_RATE_MAX}])"
            ));
        }
    }
    if let Some(volume) = params.volume {
        if !(VOICE_VOLUME_MIN..=VOICE_VOLUME_MAX).contains(&volume) {
            return Some(format!(
                "拒绝发声: 音量越界 ({volume}, 合法区间 [{VOICE_VOLUME_MIN}, {VOICE_VOLUME_MAX}])"
            ));
        }
    }
    if let Some(voice) = params.voice.as_deref() {
        if voice.trim().is_empty() {
            return Some("拒绝发声: 嗓名为空 (要系统默认嗓请省略 voice 字段)".to_string());
        }
    }
    None
}

/// 选嗓校验 (已装嗓清单是唯一事实源; 需要引擎面, 不入静态校验)。
///
/// - 清单读取失败 = 错误即帧 (真因如实);
/// - **无嗓 = 错误即帧** (不静默、不假发声);
/// - 未知嗓 = 拒绝即帧并附已装嗓清单 (可自纠)。
pub fn resolve_voice(
    installed: &[String],
    params: &VoiceSpeakParams,
) -> Result<Option<String>, String> {
    if installed.is_empty() {
        return Err("拒绝发声: 系统无已装嗓 (无嗓=错误即帧, 不假发声)".to_string());
    }
    let Some(requested) = params.voice.as_deref() else {
        return Ok(None);
    };
    let requested = requested.trim();
    if installed.iter().any(|voice| voice == requested) {
        return Ok(Some(requested.to_string()));
    }
    Err(format!(
        "拒绝发声: 未知嗓 {requested:?} (系统已装嗓: {})",
        installed.join(", ")
    ))
}

/// 一次发声的结构化结果 (输出帧的字段来源)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeakOutcome {
    /// `spoken` (说完) / `interrupted` (说了半句)。
    pub status: &'static str,
    /// 实际嗓 (系统默认嗓记 [`DEFAULT_VOICE_LABEL`])。
    pub voice: String,
    /// 生效语速。
    pub rate: i32,
    /// 生效音量。
    pub volume: u32,
    /// 文本字符数 (脱敏口径: 只报数, 不报原文)。
    pub text_chars: usize,
    /// 实际发音时长 (毫秒)。
    pub duration_ms: u64,
    /// 承载引擎标识。
    pub engine: String,
}

/// 发声配置 (账本落点 + 勿扰窗口 + 排队策略; 时钟显式注入)。
#[derive(Clone)]
pub struct VoiceSpeakConfig {
    /// 语句账本路径 (`spoken.jsonl`)。
    pub ledger_path: PathBuf,
    /// 勿扰窗口 (缺省 = 默认窗口, fail-closed 朝安静)。
    pub quiet: QuietHours,
    /// 本地墙钟「当日第几分钟」读数 (组装根注入)。
    pub quiet_clock: QuietClock,
    /// 新话与旧话的关系 (缺省 = 新话打断旧话)。
    pub playback: PlaybackPolicy,
}

impl std::fmt::Debug for VoiceSpeakConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoiceSpeakConfig")
            .field("ledger_path", &self.ledger_path)
            .field("quiet", &self.quiet)
            .field("playback", &self.playback)
            .finish_non_exhaustive()
    }
}

impl VoiceSpeakConfig {
    /// 数据目录落点 (`<data_dir>/spoken.jsonl`) + 显式时钟 (缺省勿扰窗口与
    /// 打断档; 面上旋钮由组装根按 env 覆盖)。
    pub fn new(data_dir: impl AsRef<Path>, quiet_clock: QuietClock) -> Self {
        Self {
            ledger_path: data_dir.as_ref().join(SPOKEN_LEDGER_FILE),
            quiet: QuietHours::default(),
            quiet_clock,
            playback: PlaybackPolicy::default(),
        }
    }

    /// 覆盖勿扰窗口。
    #[must_use]
    pub fn with_quiet(mut self, quiet: QuietHours) -> Self {
        self.quiet = quiet;
        self
    }

    /// 覆盖排队策略。
    #[must_use]
    pub fn with_playback_policy(mut self, policy: PlaybackPolicy) -> Self {
        self.playback = policy;
        self
    }

    /// 覆盖账本落点。
    #[must_use]
    pub fn with_ledger_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.ledger_path = path.into();
        self
    }
}

/// 发声工具 (`tool.voice_speak`, 声明名 `voice_speak`)。
pub struct VoiceSpeakTool {
    id: CapabilityId,
    config: VoiceSpeakConfig,
    engine: Arc<dyn SynthEngine>,
    playback: Arc<PlaybackController>,
    ledger: SpokenLedger,
    ordinal: AtomicU64,
}

impl VoiceSpeakTool {
    /// 绑定发声配置与合成执行面 (生产 = 系统引擎桥, 测试 = 假引擎注入)。
    pub fn new(config: VoiceSpeakConfig, engine: Arc<dyn SynthEngine>) -> Self {
        let ledger = SpokenLedger::at(config.ledger_path.clone());
        let playback = Arc::new(PlaybackController::new(
            Arc::clone(&engine),
            config.playback,
        ));
        Self {
            id: CapabilityId::new(VOICE_SPEAK_CAPABILITY_ID).expect("valid capability id"),
            config,
            engine,
            playback,
            ledger,
            ordinal: AtomicU64::new(1),
        }
    }

    /// 输出归一合同: 发声报告的形状冻结为六字段。
    pub fn output_schema() -> OutputSchema {
        OutputSchema::new()
            .require("status", SchemaKind::String)
            .require("voice", SchemaKind::String)
            .require("duration_ms", SchemaKind::Number)
            .require("text_chars", SchemaKind::Number)
            .require("engine", SchemaKind::String)
            .require("ledger", SchemaKind::String)
    }

    /// 语句账本 (自述语音节的现探测走同一本)。
    pub fn ledger(&self) -> &SpokenLedger {
        &self.ledger
    }

    /// 受控播放器 (外部停令 / 测试断言面)。
    pub fn playback(&self) -> &Arc<PlaybackController> {
        &self.playback
    }

    /// 合成执行面 (现状探测面)。
    pub fn engine(&self) -> &Arc<dyn SynthEngine> {
        &self.engine
    }

    /// 发声一次 (校验 → 安静纪律 → 选嗓 → 合成即播 → 账本)。
    ///
    /// 失败一律 `Err(拒绝即帧文案)`, 文案不回显文本原文 (脱敏口径)。
    pub async fn speak_once(&self, params: &VoiceSpeakParams) -> Result<SpeakOutcome, String> {
        if let Some(refusal) = speak_request_refusal(params) {
            return Err(refusal);
        }
        // 安静纪律 (双保险之二: 工具面; 风险钩子是其一, 同一窗口同一文案)。
        if let Some(refusal) = self.config.quiet.suppression((self.config.quiet_clock)()) {
            return Err(refusal);
        }
        // 选嗓 (系统已装嗓清单是唯一事实源; 无嗓/未知嗓/清单不可用都错误即帧)。
        let engine = Arc::clone(&self.engine);
        let installed = tokio::task::spawn_blocking(move || engine.installed_voices())
            .await
            .map_err(|error| format!("已装嗓清单采集失败: {error}"))??;
        let voice = resolve_voice(&installed, params)?;
        let request = SynthRequest {
            text: params.text.clone(),
            voice: voice.clone(),
            rate: params.effective_rate(),
            volume: params.effective_volume(),
        };
        // 合成即播 (独占; 新话打断旧话可配)。
        let ticket = match begin_speech(&self.playback, &request).await {
            Ok(ticket) => ticket,
            Err(TryBeginError::Busy) => {
                return Err("拒绝发声: 播放槽忙 (串行档不应到达此处, 如实拒)".to_string())
            }
            Err(TryBeginError::Start(reason)) => {
                return Err(format!("合成失败 (错误即帧, 不假发声): {reason}"))
            }
        };
        // 轮询收尾 (可取消: 调用被丢 = 票据 Drop = 真停)。
        let end = wait_playback(&ticket).await;
        let engine_id = ticket.engine_id().to_string();
        drop(ticket);
        if let crate::voice::synth::PlaybackEnd::Failed { reason, .. } = &end {
            return Err(format!("合成失败 (错误即帧, 不假发声): {reason}"));
        }
        let outcome = SpeakOutcome {
            status: end.status(),
            voice: voice.unwrap_or_else(|| DEFAULT_VOICE_LABEL.to_string()),
            rate: request.rate,
            volume: request.volume,
            text_chars: params.text_chars(),
            duration_ms: end.duration_ms(),
            engine: engine_id,
        };
        // 语句账本 (说了什么/何时/哪把嗓/时长; 打断 = 说了半句如实入账)。
        let spoken_at_ms = epoch_ms();
        let utterance = SpokenUtterance {
            id: spoken_utterance_id(spoken_at_ms, self.ordinal.fetch_add(1, Ordering::SeqCst)),
            spoken_at_ms,
            text: params.text.clone(),
            voice: outcome.voice.clone(),
            duration_ms: outcome.duration_ms,
            status: if outcome.status == "interrupted" {
                SpokenStatus::Interrupted
            } else {
                SpokenStatus::Completed
            },
            rate: outcome.rate,
            volume: outcome.volume,
            engine: outcome.engine.clone(),
        };
        if let Err(error) = self.ledger.append(&utterance) {
            // 已发声但账本写失败 = 错误即帧 (同内容流台账口径: 不静默丢责任记录)。
            return Err(format!(
                "已发声 (voice={}, {}ms) 但语句账本写失败: {error}",
                outcome.voice, outcome.duration_ms
            ));
        }
        Ok(outcome)
    }
}

#[async_trait]
impl ToolCapability for VoiceSpeakTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let mut parameters = ToolParameters::new();
        parameters.extend(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "text": {
                        "type": "string",
                        "description": "The utterance to speak (non-empty, at most 2000 characters)."
                    },
                    "rate": {
                        "type": "integer",
                        "description": "Speech rate in [-10, 10]; refused outside the range. Omit for the engine default."
                    },
                    "volume": {
                        "type": "integer",
                        "description": "Loudness in [0, 100]; refused outside the range. Omit for full volume."
                    },
                    "voice": {
                        "type": "string",
                        "description": "Name from the installed system voice list. Omit for the system default voice; unknown names are refused with the installed list."
                    }
                },
                "required": ["text"],
                "additionalProperties": false
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        NormalizedTool::new(VOICE_SPEAK_TOOL_NAME)
            .with_description(VOICE_SPEAK_TOOL_DESCRIPTION.to_string())
            .with_parameters(parameters)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let name = VOICE_SPEAK_TOOL_NAME;
        let params: VoiceSpeakParams = match serde_json::from_value(call.arguments.clone()) {
            Ok(params) => params,
            Err(error) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("invalid voice_speak parameters: {error}"),
                )
                .with_name(name)
            }
        };
        match self.speak_once(&params).await {
            Ok(outcome) => ToolResult::ok(
                &call.id,
                serde_json::json!({
                    "status": outcome.status,
                    "voice": outcome.voice,
                    "rate": outcome.rate,
                    "volume": outcome.volume,
                    "text_chars": outcome.text_chars,
                    "duration_ms": outcome.duration_ms,
                    "engine": outcome.engine,
                    "ledger": SPOKEN_LEDGER_FILE,
                }),
            )
            .with_name(name),
            Err(refusal) => ToolResult::permanent_error(&call.id, refusal).with_name(name),
        }
    }
}

/// epoch 毫秒 (发声时刻)。
fn epoch_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

/// 发声面风险映射 (pre-execute 瀑布): 发声面档, **与 shell 的执行档分立**。
///
/// 裁定顺序 (命名即承诺):
/// 1. **授权面**: grant 即发声 (发声面默认档 = 免逐次人批, 注册即闸是那道
///    闸); 未 grant = **拒绝即帧** (fail-closed, 不降级成审批 —— 不借壳);
///    显式挂了审批档的宿主仍走同一条人工审批链 (`Ask`);
/// 2. **坏参数拒绝即帧** (不进审批、不钳制);
/// 3. **勿扰窗口拒绝即帧** (安静纪律, 与工具面同窗口同文案)。
pub struct VoiceSpeakRiskMappingHook {
    policy: Arc<Mutex<PermissionPolicy>>,
    quiet: QuietHours,
    quiet_clock: QuietClock,
}

impl VoiceSpeakRiskMappingHook {
    /// 挂在共享授权策略上的风险映射钩子 (勿扰窗口与时钟同源注入)。
    pub fn new(
        policy: Arc<Mutex<PermissionPolicy>>,
        quiet: QuietHours,
        quiet_clock: QuietClock,
    ) -> Self {
        Self {
            policy,
            quiet,
            quiet_clock,
        }
    }
}

impl PreExecuteHook for VoiceSpeakRiskMappingHook {
    fn name(&self) -> &str {
        "voice_speak_risk_mapping"
    }

    fn pre_verdict(&self, request: &PreExecuteRequest<'_>) -> PreVerdict {
        if request.capability.as_str() != VOICE_SPEAK_CAPABILITY_ID {
            return PreVerdict::Allow;
        }
        let decision = self
            .policy
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .decision_for_capability(request.capability.as_str());
        match decision {
            Decision::Allow => {}
            Decision::Deny { reason } => {
                return PreVerdict::Deny {
                    reason: format!("拒绝发声: 未授权 (注册即闸, fail-closed): {reason}"),
                }
            }
            Decision::RequireApproval { reason } => return PreVerdict::Ask { reason },
        }
        if let Ok(params) =
            serde_json::from_value::<VoiceSpeakParams>(request.call.arguments.clone())
        {
            if let Some(refusal) = speak_request_refusal(&params) {
                return PreVerdict::Deny { reason: refusal };
            }
        }
        if let Some(refusal) = self.quiet.suppression((self.quiet_clock)()) {
            return PreVerdict::Deny { reason: refusal };
        }
        PreVerdict::Allow
    }
}

/// 单调边界 Guard (只拒不放): 坏参数 / 勿扰窗口一律拒绝即帧。
pub struct VoiceSpeakBoundaryGuard {
    quiet: QuietHours,
    quiet_clock: QuietClock,
}

impl VoiceSpeakBoundaryGuard {
    /// 绑定勿扰窗口与时钟的边界 Guard。
    pub fn new(quiet: QuietHours, quiet_clock: QuietClock) -> Self {
        Self { quiet, quiet_clock }
    }
}

impl ToolGuard for VoiceSpeakBoundaryGuard {
    fn name(&self) -> &str {
        "voice_speak_boundary"
    }

    fn deny(&self, request: &ToolGuardRequest<'_>) -> Option<String> {
        if request.capability.as_str() != VOICE_SPEAK_CAPABILITY_ID {
            return None;
        }
        // 解析失败不归边界 Guard (由工具即帧报解析错误), 只判坏参数与勿扰。
        if let Ok(params) = serde_json::from_value::<VoiceSpeakParams>(request.arguments.clone()) {
            if let Some(refusal) = speak_request_refusal(&params) {
                return Some(refusal);
            }
        }
        self.quiet.suppression((self.quiet_clock)())
    }
}

/// 生产授权播种: 已注册的发声工具即获显式授权 (授权 = 「发声面有人拍板」)。
///
/// 发声面默认档**不设逐次人批** (与 shell 的 grant + require_approval 执行档
/// 分立): 注册即闸 + 参数边界 + 勿扰窗口 + 独占播放 + 账本全责, 就是发声面
/// 的拦截面。宿主要逐次人批 = 在授权策略上给本能力挂 `require_approval`,
/// 风险映射钩子会如实把它映射到同一条人工审批链。
pub fn authorized_voice_speak_policy() -> Arc<Mutex<PermissionPolicy>> {
    let policy = Arc::new(Mutex::new(PermissionPolicy::new()));
    policy
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .grant(apeireth_governance::Permission::ExecuteTool(
            VOICE_SPEAK_CAPABILITY_ID.to_string(),
        ));
    policy
}

/// 组装发声五段流水线: 风险映射 → 单调边界 Guard → 超时 → 输出归一合同。
pub fn voice_speak_pipeline(
    policy: Arc<Mutex<PermissionPolicy>>,
    quiet: QuietHours,
    quiet_clock: QuietClock,
) -> Arc<crate::exec_pipeline::ToolExecutionPipeline> {
    let guard = VoiceSpeakBoundaryGuard::new(quiet.clone(), Arc::clone(&quiet_clock));
    Arc::new(
        crate::exec_pipeline::ToolExecutionPipeline::new()
            .with_pre_hook(Arc::new(VoiceSpeakRiskMappingHook::new(
                policy,
                quiet,
                quiet_clock,
            )))
            .with_guard(Arc::new(guard))
            .with_around(
                AroundPolicy::new()
                    .with_timeout(VOICE_SPEAK_TIMEOUT_MS, VOICE_SPEAK_MAX_TIMEOUT_MS),
            )
            .with_output_schema(VoiceSpeakTool::output_schema()),
    )
}

/// 组装生产工具面: 发声工具 + 五段流水线 (模型侧一个能力)。
pub fn voice_speak_capability(
    config: VoiceSpeakConfig,
    engine: Arc<dyn SynthEngine>,
    policy: Arc<Mutex<PermissionPolicy>>,
) -> Arc<dyn ToolCapability> {
    let quiet = config.quiet.clone();
    let quiet_clock = Arc::clone(&config.quiet_clock);
    let tool: Arc<dyn ToolCapability> = Arc::new(VoiceSpeakTool::new(config, engine));
    Arc::new(crate::exec_pipeline::PipelinedCapability::new(
        tool,
        voice_speak_pipeline(policy, quiet, quiet_clock),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice::quiet::fixed_clock;

    fn params(text: &str) -> VoiceSpeakParams {
        VoiceSpeakParams {
            text: text.to_string(),
            rate: None,
            volume: None,
            voice: None,
        }
    }

    #[test]
    fn static_validation_refuses_without_touching_the_engine() {
        assert!(speak_request_refusal(&params("  ")).is_some(), "空文本拒");
        let long = VoiceSpeakParams {
            text: "字".repeat(VOICE_SPEAK_MAX_TEXT_CHARS + 1),
            ..params("")
        };
        let refusal = speak_request_refusal(&long).expect("超限拒");
        assert!(refusal.contains("文本超限"), "{refusal}");
        assert!(!refusal.contains("字字字"), "拒绝文案不回显文本原文 (脱敏)");

        let fast = VoiceSpeakParams {
            rate: Some(VOICE_RATE_MAX + 1),
            ..params("一句")
        };
        assert!(speak_request_refusal(&fast).is_some(), "语速上界拒");
        let slow = VoiceSpeakParams {
            rate: Some(VOICE_RATE_MIN - 1),
            ..params("一句")
        };
        assert!(speak_request_refusal(&slow).is_some(), "语速下界拒");
        let loud = VoiceSpeakParams {
            volume: Some(VOICE_VOLUME_MAX + 1),
            ..params("一句")
        };
        assert!(speak_request_refusal(&loud).is_some(), "音量上界拒");
        let empty_voice = VoiceSpeakParams {
            voice: Some("  ".to_string()),
            ..params("一句")
        };
        assert!(speak_request_refusal(&empty_voice).is_some(), "空嗓名拒");

        // 边界值本身合法 (拒绝的是越界, 不是边界)。
        let edge = VoiceSpeakParams {
            rate: Some(VOICE_RATE_MAX),
            volume: Some(VOICE_VOLUME_MAX),
            voice: Some("嗓".to_string()),
            ..params("一句")
        };
        assert!(speak_request_refusal(&edge).is_none());
        let floor = VoiceSpeakParams {
            rate: Some(VOICE_RATE_MIN),
            volume: Some(VOICE_VOLUME_MIN),
            ..params("一句")
        };
        assert!(speak_request_refusal(&floor).is_none());
    }

    #[test]
    fn voice_resolution_reads_the_installed_list_only() {
        let installed = vec!["嗓一".to_string(), "嗓二".to_string()];
        let named = VoiceSpeakParams {
            voice: Some("嗓二".to_string()),
            ..params("一句")
        };
        assert_eq!(
            resolve_voice(&installed, &named).expect("known voice"),
            Some("嗓二".to_string())
        );
        assert_eq!(
            resolve_voice(&installed, &params("一句")).expect("default"),
            None
        );
        let unknown = VoiceSpeakParams {
            voice: Some("没有这把嗓".to_string()),
            ..params("一句")
        };
        let refusal = resolve_voice(&installed, &unknown).expect_err("未知嗓拒");
        assert!(refusal.contains("未知嗓"), "{refusal}");
        assert!(refusal.contains("嗓一"), "拒绝附已装嗓清单 (可自纠)");
        let none = resolve_voice(&[], &params("一句")).expect_err("无嗓拒");
        assert!(none.contains("无嗓"), "{none}");
    }

    #[tokio::test]
    async fn quiet_window_shuts_the_tool_face_before_the_engine() {
        let quiet = QuietHours::parse("22:00-06:00").expect("window");
        let config = VoiceSpeakConfig::new(std::env::temp_dir(), fixed_clock(23 * 60))
            .with_quiet(quiet.clone());
        let engine = Arc::new(crate::voice::mock::MockSynthEngine::new(
            crate::voice::mock::MockBehavior::AutoComplete,
        ));
        let state = engine.state();
        let tool = VoiceSpeakTool::new(config, engine);
        let refusal = tool
            .speak_once(&params("夜里不该出声"))
            .await
            .expect_err("勿扰窗口内拒绝即帧");
        assert!(refusal.contains("勿扰窗口"), "{refusal}");
        assert_eq!(state.created(), 0, "勿扰抑制: 引擎零调用 (不造播放)");
    }
}
