//! Apeireth Desktop Backend Supervisor
//!
//! Manages the lifecycle of the bundled canonical Apeireth gateway backend process.
//!
//! # Responsibilities
//! - Locate bundled apeireth executable
//! - Select free port for gateway
//! - Spawn owned backend process
//! - Track PID and ownership
//! - Health probe for readiness
//! - Capture stdout/stderr to persistent logs
//! - Detect unexpected exit
//! - Expose safe state to frontend
//! - Graceful shutdown on app exit

use crate::keychain;
use crate::logging::{DesktopLogger, LogLevel};
use crate::stored_config;
use crate::workspace;
use apeireth_core::stored_doc::{self, DocCompat};
// Workspace policy (clippy.toml): fs_err over std::fs, so filesystem errors
// carry the path that failed.
use fs_err as fs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{watch, RwLock};

/// Backend supervisor state machine
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum BackendState {
    Stopped,
    Starting,
    Ready,
    Failed,
    Stopping,
}

/// One supervised lifecycle transition, kept in order.
///
/// Evidence face for the spawn contract: a first boot must observe exactly
/// `Spawn` then `Ready` — one spawn straight to ready — never a
/// `RestartRequired`/`Stopped` bounce in between (E-07).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum LifecycleEvent {
    /// The owned sidecar process is being spawned.
    Spawn,
    /// The sidecar answered its readiness probe.
    Ready,
    /// The sidecar failed its readiness probe.
    ReadyFailed,
    /// The spawn attempt itself failed.
    SpawnFailed,
    /// A restart-relevant change forced a bounce of the running sidecar.
    RestartRequired,
    /// The owned sidecar process was stopped.
    Stopped,
}

/// Backend process information
#[derive(Debug, Clone, Serialize)]
pub struct BackendInfo {
    pub state: BackendState,
    pub ownership: BackendOwnership,
    pub pid: Option<u32>,
    pub endpoint: Option<String>,
    pub port: Option<u16>,
    #[serde(skip)]
    pub started_at: Option<Instant>,
    pub restart_count: u32,
    pub last_exit_code: Option<i32>,
    pub last_error: Option<String>,
    pub backend_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum BackendOwnership {
    OwnedByDesktop,
    External,
}

/// Provider environment the desktop injects into the spawned sidecar.
///
/// The Settings UI is the single configuration source for a packaged desktop:
/// the user's provider choice (key / endpoint / models) is pushed here over IPC
/// and becomes the sidecar's environment, exactly as the canonical CLI reads
/// it (`OPENAI_API_KEY` + `APEIRETH_OPENAI_URL` + `APEIRETH_OPENAI_MODELS`,
/// `APEIRETH_API_KEY` + `APEIRETH_API_URL` + `APEIRETH_API_MODELS` for
/// MiniMax, `APEIRETH_ANTHROPIC_KEY` + `APEIRETH_ANTHROPIC_URL` +
/// `APEIRETH_ANTHROPIC_MODELS` for Anthropic). Without this, a packaged app
/// would only ever see whatever environment the user happened to export
/// system-wide — two configuration sources with no guidance.
///
/// Secret hygiene: key fields are in-memory only. They cross IPC to reach the
/// child's environment, but are never persisted to the app-data config file
/// (see [`BackendSupervisor::persist_provider_env`]) and never appear in
/// [`BackendInfo`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BackendProviderEnv {
    pub openai_api_key: Option<String>,
    pub openai_url: Option<String>,
    pub openai_models: Option<String>,
    pub minimax_api_key: Option<String>,
    pub minimax_url: Option<String>,
    pub minimax_models: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub anthropic_url: Option<String>,
    pub anthropic_models: Option<String>,
}

impl BackendProviderEnv {
    /// Normalize for storage: trim whitespace; empty values mean "unset".
    pub fn sanitized(mut self) -> Self {
        let clean = |value: &mut Option<String>| {
            if let Some(v) = value {
                let trimmed = v.trim();
                if trimmed.is_empty() {
                    *value = None;
                } else {
                    *value = Some(trimmed.to_string());
                }
            }
        };
        clean(&mut self.openai_api_key);
        clean(&mut self.openai_url);
        clean(&mut self.openai_models);
        clean(&mut self.minimax_api_key);
        clean(&mut self.minimax_url);
        clean(&mut self.minimax_models);
        clean(&mut self.anthropic_api_key);
        clean(&mut self.anthropic_url);
        clean(&mut self.anthropic_models);
        self
    }

    /// True when nothing would be injected.
    pub fn is_empty(&self) -> bool {
        self.env_pairs().is_empty()
    }

    /// The exact (variable, value) pairs injected into the sidecar.
    pub fn env_pairs(&self) -> Vec<(&'static str, &str)> {
        let mut pairs = Vec::new();
        if let Some(value) = &self.openai_api_key {
            pairs.push(("OPENAI_API_KEY", value.as_str()));
        }
        if let Some(value) = &self.openai_url {
            pairs.push(("APEIRETH_OPENAI_URL", value.as_str()));
        }
        if let Some(value) = &self.openai_models {
            pairs.push(("APEIRETH_OPENAI_MODELS", value.as_str()));
        }
        if let Some(value) = &self.minimax_api_key {
            pairs.push(("APEIRETH_API_KEY", value.as_str()));
        }
        if let Some(value) = &self.minimax_url {
            pairs.push(("APEIRETH_API_URL", value.as_str()));
        }
        if let Some(value) = &self.minimax_models {
            pairs.push(("APEIRETH_API_MODELS", value.as_str()));
        }
        if let Some(value) = &self.anthropic_api_key {
            pairs.push(("APEIRETH_ANTHROPIC_KEY", value.as_str()));
        }
        if let Some(value) = &self.anthropic_url {
            pairs.push(("APEIRETH_ANTHROPIC_URL", value.as_str()));
        }
        if let Some(value) = &self.anthropic_models {
            pairs.push(("APEIRETH_ANTHROPIC_MODELS", value.as_str()));
        }
        pairs
    }

    /// Copy without any key material (what may reach disk).
    pub fn without_secrets(&self) -> Self {
        Self {
            openai_api_key: None,
            minimax_api_key: None,
            anthropic_api_key: None,
            ..self.clone()
        }
    }

    /// Fill missing key fields from the OS keychain (or any injected lookup).
    ///
    /// Priority is `env > keychain`: a key already present in memory (from a
    /// settings save this session) wins; only absent fields fall back to the
    /// keychain, so a fresh launch restores persisted keys without touching
    /// endpoints/models. The lookup is injected so the merge is unit-testable
    /// without touching the real OS credential store.
    pub fn with_keychain_fallback(
        mut self,
        mut lookup: impl FnMut(&str) -> Option<String>,
    ) -> Self {
        if self.openai_api_key.is_none() {
            self.openai_api_key = lookup(keychain::PROVIDER_OPENAI);
        }
        if self.minimax_api_key.is_none() {
            self.minimax_api_key = lookup(keychain::PROVIDER_MINIMAX);
        }
        if self.anthropic_api_key.is_none() {
            self.anthropic_api_key = lookup(keychain::PROVIDER_ANTHROPIC);
        }
        self
    }

    /// The (variable, value) pairs that CANNOT be hot-applied through the
    /// gateway's `/v1/admin/config` endpoint and therefore require a sidecar
    /// restart when they change.
    ///
    /// The endpoint can hot-apply exactly: the openai-compatible provider's
    /// `api_key` (runtime credential store, resolved per request) and
    /// `base_url` (capability hot-set) plus the default-model injection.
    /// Everything else — model lists (the provider's model registry is fixed at
    /// boot), the minimax/anthropic families, and all capability toggles — only
    /// reaches the sidecar through its environment. Hot-applying those would
    /// silently leave the running gateway on stale configuration (2026-09-12
    /// real-machine finding).
    ///
    /// 2026-09-26 曾把 OPENAI_API_KEY / APEIRETH_OPENAI_URL 一并列为重启相关
    /// （当时热更 key 实测不生效）。真热更（凭据按请求现解析）落地后该两对
    /// 恢复为可热更，行为由集成测试锁定（`apeireth-cli` 的
    /// `admin_hot_key_decoy`: 热更 key 后下一请求出站 Authorization 即新值）。
    pub fn restart_relevant_pairs(&self) -> Vec<(&'static str, &str)> {
        self.env_pairs()
            .into_iter()
            .filter(|(key, _)| *key != "OPENAI_API_KEY" && *key != "APEIRETH_OPENAI_URL")
            .collect()
    }
}

/// Advanced-capability toggles the desktop injects into the sidecar.
///
/// Mirrors the canonical CLI knobs (`APEIRETH_ENABLE_*` + `APEIRETH_COGNITIVE_*`).
/// Fail-closed by construction: only `true` values emit `"1"`; absent variables
/// mean OFF in the CLI, so a false toggle injects nothing. Three explicit
/// exceptions are spelled out: `shell_sandbox_off = true` emits
/// `APEIRETH_SHELL_SANDBOX=0` (the backend default is ON, so opting out needs
/// an explicit value); the memory-core trio (preference_learning /
/// proactive_recall / memory_injection) is ON by default in the CLI, so each
/// emits `"1"`/`"0"` explicitly — an off toggle must not be swallowed by the
/// CLI default; and the numeric knobs emit their value only when > 0
/// (0 = "do not inject", letting the backend keep its own default).
/// Shell/fetch stay behind the runtime's require-approval governance even when
/// enabled, so the UI toggle alone never grants unrestricted execution.
///
/// 2026-10-10 W2/W3 收官批：补齐 v1→v2 补漏落地的全部认知旋钮
/// (partner_bond / morphology / education / absorption / community /
/// onering_ledger / onion_layer / worktree_sandbox + 记忆流四件 + 议会数值)。
/// `serde(default)` keeps persisted JSON from before this extension loadable;
/// missing fields default to false/0 = inject nothing = backend defaults hold,
/// except the memory-core trio (see [`BackendCapabilityEnv::default`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "snake_case")]
pub struct BackendCapabilityEnv {
    pub enable_shell: bool,
    /// true → inject APEIRETH_SHELL_SANDBOX=0 (explicit unsandboxed run).
    pub shell_sandbox_off: bool,
    pub enable_fetch: bool,
    pub enable_local_read_tools: bool,
    /// 受控文件写入 (APEIRETH_ENABLE_FILE_WRITE, apply_patch 补丁式写入):
    /// **文件写入总闸：apply_patch 与 shell 写命令同受此闸** (唯一写总闸,
    /// 关 = shell 写意图拒绝即帧)。默认关, 仅 true 注入 "1" (fail-closed,
    /// false 不注入 "0")。
    pub enable_file_write: bool,
    /// 子开关「自动放行已读文件修改」(APEIRETH_ENABLE_FILE_WRITE_AUTO_PASS):
    /// 仅 true 注入 "1"; 依赖主开关的语义在映射层收口 (主开关关时恒 false)。
    pub enable_file_write_auto_pass: bool,
    pub disable_typed_recall: bool,
    pub enable_organs: bool,
    pub enable_preference_learning: bool,
    pub enable_proactive_recall: bool,
    pub enable_memory_injection: bool,
    pub enable_consolidation: bool,
    pub enable_reflexion: bool,
    pub enable_partner_bond: bool,
    pub enable_morphology_recall: bool,
    pub enable_education: bool,
    pub enable_absorption_insight: bool,
    pub enable_community_triage: bool,
    pub enable_onering_ledger: bool,
    pub cognitive_judge: bool,
    pub cognitive_council: bool,
    /// 1-7 advisor count; 0 = do not inject (backend default 3).
    pub council_advisors: u32,
    /// Per-advisor timeout in ms; 0 = do not inject (backend default 30000).
    pub council_timeout_ms: u32,
    /// Morphology recall temperature (f64, like the backend's `env_temperature`);
    /// 0.0 = do not inject (backend default 1.0).
    pub morphology_temperature: f64,
    pub enable_onion_layer: bool,
    pub enable_worktree_sandbox: bool,
    /// [Beta] 思考模式（reasoning_content 分流展示，对齐 provider
    /// `ReasoningAdapterConfig::from_env`）；字符串字段空 = 不注入。
    pub reasoning_enabled: bool,
    pub reasoning_model_filters: String,
    pub reasoning_tag: String,
    /// 「性格养成」第一铲: 遗忘衰减强度倍率 (APEIRETH_TUNE_MEMORY_FADE);
    /// `None` = 不注入 (后端默认 1.0 = 现行为, 零变化)。
    pub tune_memory_fade: Option<f64>,
    /// 好奇强度倍率 (APEIRETH_TUNE_CURIOSITY_STRENGTH); None = 不注入 (后端默认 1.0)。
    pub tune_curiosity_strength: Option<f64>,
    /// 语气情绪饱和倍率 (APEIRETH_TUNE_TONE_SATURATION); None = 不注入 (后端默认 1.0)。
    pub tune_tone_saturation: Option<f64>,
    /// 整合节奏·每 N 回合 (APEIRETH_TUNE_CONSOLIDATION_CADENCE); None = 不注入 (后端默认 1)。
    pub tune_consolidation_cadence: Option<f64>,
    /// 「从使用中学习」自校准开关 (APEIRETH_ENABLE_SELF_TUNING): 默认关,
    /// 仅 true 时注入 "1" (fail-closed, 缺席 = 关)。
    pub enable_self_tuning: bool,
    /// 单回合轮数上限 (APEIRETH_MAX_TURN_ROUNDS); `None` = 不注入 (后端默认 8)。
    /// 取值裁决在后端解析: 越界钳制 1..=64、非法回默认 —— 注入侧只做同源透传。
    pub turn_round_limit: Option<u32>,
    /// 单轮工具调用上限 (APEIRETH_MAX_TOOL_CALLS); `None` = 不注入 (后端默认 16)。
    /// 同 [`Self::turn_round_limit`] 的钳制/回默认语义。
    pub tool_call_limit: Option<u32>,
    /// 上下文注入总字符预算 (APEIRETH_CONTEXT_BUDGET_CHARS); `None` = 不注入
    /// (后端默认 24000)。后端语义: 正整数直通、越界/非法回默认 (无上界钳制)。
    pub context_budget_chars: Option<u64>,
}

/// Explicit on/off value for a default-on CLI knob: the desktop injects both
/// states because absence means ON on the backend side.
fn on_off_value(on: bool) -> String {
    if on { "1" } else { "0" }.to_string()
}

/// Render an experience-knob float for env injection (1 → "1", 0.5 → "0.5"),
/// mirroring the morphology-temperature rendering.
fn render_tuning_value(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
}

/// 产品默认，与 frontend `DEFAULT_CAPABILITY_TOGGLES` 对齐：记忆核心族三件
/// 默认开（全新安装 / 旧持久化 JSON 缺字段都按「未设 = 开」注入），其余默认关
/// fail-closed。显式 false 仍是显式关（注入 "0"）。
impl Default for BackendCapabilityEnv {
    fn default() -> Self {
        Self {
            enable_shell: false,
            shell_sandbox_off: false,
            enable_fetch: false,
            enable_local_read_tools: true,
            enable_file_write: false,
            enable_file_write_auto_pass: false,
            disable_typed_recall: false,
            enable_organs: false,
            enable_preference_learning: true,
            enable_proactive_recall: true,
            enable_memory_injection: true,
            enable_consolidation: false,
            enable_reflexion: false,
            enable_partner_bond: false,
            enable_morphology_recall: false,
            enable_education: false,
            enable_absorption_insight: false,
            enable_community_triage: false,
            enable_onering_ledger: false,
            cognitive_judge: false,
            cognitive_council: false,
            council_advisors: 0,
            council_timeout_ms: 0,
            morphology_temperature: 0.0,
            enable_onion_layer: false,
            enable_worktree_sandbox: false,
            reasoning_enabled: false,
            reasoning_model_filters: String::new(),
            reasoning_tag: String::new(),
            // 体验旋钮: 缺省不注入 (后端默认 = 基线 = 现行为, 零变化)。
            tune_memory_fade: None,
            tune_curiosity_strength: None,
            tune_tone_saturation: None,
            tune_consolidation_cadence: None,
            enable_self_tuning: false,
            // 预算旋钮: 缺省不注入 (后端默认 8 / 16 / 24000 = 现行为)。
            turn_round_limit: None,
            tool_call_limit: None,
            context_budget_chars: None,
        }
    }
}

impl BackendCapabilityEnv {
    /// The (variable, value) pairs injected into the sidecar. Only enabled
    /// capabilities appear; the CLI treats absence as OFF — except the
    /// memory-core trio below, which defaults ON in the CLI and therefore
    /// injects an explicit "1"/"0" in both states.
    pub fn env_pairs(&self) -> Vec<(&'static str, String)> {
        let mut pairs = Vec::new();
        if self.enable_shell {
            pairs.push(("APEIRETH_ENABLE_SHELL", "1".to_string()));
        }
        if self.shell_sandbox_off {
            // Backend default is sandbox ON; only the explicit opt-out ("0")
            // needs injecting.
            pairs.push(("APEIRETH_SHELL_SANDBOX", "0".to_string()));
        }
        if self.enable_fetch {
            pairs.push(("APEIRETH_ENABLE_FETCH", "1".to_string()));
        }
        if self.enable_local_read_tools {
            pairs.push(("APEIRETH_ENABLE_LOCAL_READ_TOOLS", "1".to_string()));
        } else {
            // 双向显式: 默认开旋钮的「关」必须显式反注入
            // (CLI 侧 `APEIRETH_DISABLE_LOCAL_READ_TOOLS=1` 逃生门胜出)。
            pairs.push(("APEIRETH_DISABLE_LOCAL_READ_TOOLS", "1".to_string()));
        }
        // 受控文件写入两件 (apply_patch 补丁式写入): 与 enable_shell/enable_fetch
        // 同契约 —— fail-closed, 仅 true 注入 "1", false 不注入 "0";
        // 子开关依赖主开关的语义在映射层收口 (主开关关时恒 false 不注入)。
        if self.enable_file_write {
            pairs.push(("APEIRETH_ENABLE_FILE_WRITE", "1".to_string()));
        }
        if self.enable_file_write_auto_pass {
            pairs.push(("APEIRETH_ENABLE_FILE_WRITE_AUTO_PASS", "1".to_string()));
        }
        if self.disable_typed_recall {
            pairs.push(("APEIRETH_DISABLE_TYPED_RECALL", "1".to_string()));
        }
        if self.enable_organs {
            pairs.push(("APEIRETH_ENABLE_ORGANS", "1".to_string()));
        }
        // 记忆核心族三件 CLI 默认开: 双向显式注入, 关 = "0" (CLI `=0` = off)。
        pairs.push((
            "APEIRETH_ENABLE_PREFERENCE_LEARNING",
            on_off_value(self.enable_preference_learning),
        ));
        pairs.push((
            "APEIRETH_ENABLE_PROACTIVE_RECALL",
            on_off_value(self.enable_proactive_recall),
        ));
        pairs.push((
            "APEIRETH_ENABLE_MEMORY_INJECTION",
            on_off_value(self.enable_memory_injection),
        ));
        if self.enable_consolidation {
            pairs.push(("APEIRETH_ENABLE_CONSOLIDATION", "1".to_string()));
        }
        if self.enable_reflexion {
            pairs.push(("APEIRETH_ENABLE_REFLEXION", "1".to_string()));
        }
        if self.enable_partner_bond {
            pairs.push(("APEIRETH_ENABLE_PARTNER_BOND", "1".to_string()));
        }
        if self.enable_morphology_recall {
            pairs.push(("APEIRETH_ENABLE_MORPHOLOGY_RECALL", "1".to_string()));
        }
        if self.enable_education {
            pairs.push(("APEIRETH_ENABLE_EDUCATION", "1".to_string()));
        }
        if self.enable_absorption_insight {
            pairs.push(("APEIRETH_ENABLE_ABSORPTION_INSIGHT", "1".to_string()));
        }
        if self.enable_community_triage {
            pairs.push(("APEIRETH_ENABLE_COMMUNITY_TRIAGE", "1".to_string()));
        }
        if self.enable_onering_ledger {
            pairs.push(("APEIRETH_ENABLE_ONERING_LEDGER", "1".to_string()));
        }
        if self.cognitive_judge {
            pairs.push(("APEIRETH_COGNITIVE_JUDGE", "1".to_string()));
        }
        if self.cognitive_council {
            pairs.push(("APEIRETH_COGNITIVE_COUNCIL", "1".to_string()));
        }
        if self.council_advisors > 0 {
            pairs.push((
                "APEIRETH_COUNCIL_ADVISORS",
                self.council_advisors.to_string(),
            ));
        }
        if self.council_timeout_ms > 0 {
            pairs.push((
                "APEIRETH_COUNCIL_TIMEOUT_MS",
                self.council_timeout_ms.to_string(),
            ));
        }
        if self.morphology_temperature > 0.0 {
            let t = self.morphology_temperature;
            let rendered = if t.fract() == 0.0 {
                format!("{t:.0}")
            } else {
                t.to_string()
            };
            pairs.push(("APEIRETH_MORPHOLOGY_TEMPERATURE", rendered));
        }
        if self.enable_onion_layer {
            pairs.push(("APEIRETH_ENABLE_ONION_LAYER", "1".to_string()));
        }
        if self.enable_worktree_sandbox {
            pairs.push(("APEIRETH_ENABLE_WORKTREE_SANDBOX", "1".to_string()));
        }
        if self.reasoning_enabled {
            pairs.push(("APEIRETH_REASONING_ENABLED", "1".to_string()));
            let filters = self.reasoning_model_filters.trim();
            if !filters.is_empty() {
                pairs.push(("APEIRETH_REASONING_MODEL_FILTERS", filters.to_string()));
            }
            let tag = self.reasoning_tag.trim();
            if !tag.is_empty() {
                pairs.push(("APEIRETH_REASONING_TAG", tag.to_string()));
            }
        }
        // 「性格养成」第一铲: 四个体验旋钮 (APEIRETH_TUNE_*) —— 与后端
        // orchestration::self_tuning 的 env 名严格一致; Some 才注入 (旧持久化
        // JSON 缺字段 = None = 不注入 = 后端基线现行为)。
        if let Some(value) = self.tune_memory_fade {
            pairs.push(("APEIRETH_TUNE_MEMORY_FADE", render_tuning_value(value)));
        }
        if let Some(value) = self.tune_curiosity_strength {
            pairs.push((
                "APEIRETH_TUNE_CURIOSITY_STRENGTH",
                render_tuning_value(value),
            ));
        }
        if let Some(value) = self.tune_tone_saturation {
            pairs.push(("APEIRETH_TUNE_TONE_SATURATION", render_tuning_value(value)));
        }
        if let Some(value) = self.tune_consolidation_cadence {
            pairs.push((
                "APEIRETH_TUNE_CONSOLIDATION_CADENCE",
                render_tuning_value(value),
            ));
        }
        if self.enable_self_tuning {
            pairs.push(("APEIRETH_ENABLE_SELF_TUNING", "1".to_string()));
        }
        // 预算旋钮 (APEIRETH_MAX_TURN_ROUNDS / APEIRETH_MAX_TOOL_CALLS /
        // APEIRETH_CONTEXT_BUDGET_CHARS): Some 才注入 (旧持久化 JSON 缺字段 =
        // None = 不注入 = 后端默认现行为)。取值裁决在后端解析 (越界钳制
        // 1..=64、非法回默认 / 正整数直通), 注入侧只做同源透传。
        if let Some(rounds) = self.turn_round_limit {
            pairs.push(("APEIRETH_MAX_TURN_ROUNDS", rounds.to_string()));
        }
        if let Some(calls) = self.tool_call_limit {
            pairs.push(("APEIRETH_MAX_TOOL_CALLS", calls.to_string()));
        }
        if let Some(chars) = self.context_budget_chars {
            pairs.push(("APEIRETH_CONTEXT_BUDGET_CHARS", chars.to_string()));
        }
        pairs
    }

    /// True when nothing would be injected into the sidecar at all.
    pub fn is_empty(&self) -> bool {
        self.env_pairs().is_empty()
    }
}

/// 一条学习日志记录 (只读回显), 镜像后端 `self_tuning::TuningRecord` 的
/// tuning-log.jsonl 行 (snake_case)。`param` ∈ memory_fade / curiosity_strength /
/// tone_saturation / consolidation_cadence。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TuningLogEntry {
    /// 单调递增序号 (撤销定位用)。
    pub seq: u64,
    /// 被调体验参数 (snake_case)。
    pub param: String,
    /// 调整前值。
    pub previous: f64,
    /// 调整后值 (被上限挡住时 == previous)。
    pub next: f64,
    /// 调整原因 (人可读)。
    pub reason: String,
    /// 事件时间戳 (epoch ms)。
    pub at_epoch_ms: i64,
}

/// 学习日志文件名 (与后端接线层 `TUNING_LOG_FILE` 一致)。
pub const TUNING_LOG_FILE: &str = "tuning-log.jsonl";

/// 学习日志读取上限 (16 MiB): 只读命令不为超大文件兜底买单, 超限报错不猜。
const TUNING_LOG_MAX_BYTES: u64 = 16 * 1024 * 1024;

/// 学习日志读取结果: 好记录 + 被跳过的坏行计数 (坏行读作不存在, 不静默丢)。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TuningLogRead {
    /// 解析成功且版本契约相符的记录 (保持文件顺序)。
    pub entries: Vec<TuningLogEntry>,
    /// 被跳过的行数 (坏 JSON / 信封畸形 / 身份不符 / 版本戳不符)。
    pub skipped: usize,
}

/// 只读解析 tuning-log.jsonl (逐记录类, 走存储文档 `scan_records` 语义):
/// 文件缺失 = 空列表 (尚无自动调整); 单条坏行**读作不存在** —— 跳过并计数,
/// 不砖化整个日志; 前信封旧裸体行只读兼容 (不静默迁移)。
pub fn read_tuning_log_file(path: &Path) -> Result<TuningLogRead, String> {
    match fs::metadata(path) {
        Ok(meta) if meta.len() > TUNING_LOG_MAX_BYTES => {
            return Err(format!(
                "tuning log too large ({} bytes > {TUNING_LOG_MAX_BYTES}): {}",
                meta.len(),
                path.display()
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(TuningLogRead::default());
        }
        Err(error) => {
            return Err(format!(
                "tuning log read failed ({}): {error}",
                path.display()
            ));
        }
    }
    let scan = stored_doc::scan_records_compat::<TuningLogEntry>(path, &tuning_log_doc_compat())
        .map_err(|error| format!("tuning log read failed ({}): {error}", path.display()))?;
    Ok(TuningLogRead {
        entries: scan
            .records
            .into_iter()
            .map(|record| record.into_body())
            .collect(),
        skipped: scan.skipped,
    })
}

/// Body for the gateway's hot-config endpoint (`POST /v1/admin/config`).
///
/// The canonical gateway may serve this endpoint to hot-apply configuration
/// without a process restart; older gateways return 404/405, which the caller
/// turns back into the legacy restart path.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
struct AdminConfigRequest {
    provider: Option<String>,
    base_url: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
    capabilities: BackendCapabilityEnv,
}

impl AdminConfigRequest {
    /// Build the request from the merged provider env + capability toggles.
    ///
    /// Only the active provider family is emitted. When several families are
    /// populated, the first in struct order wins (openai > minimax > anthropic).
    fn from_env(provider: &BackendProviderEnv, capabilities: &BackendCapabilityEnv) -> Self {
        let (name, base_url, api_key, model) = if provider.openai_api_key.is_some()
            || provider.openai_url.is_some()
            || provider.openai_models.is_some()
        {
            (
                Some(keychain::PROVIDER_OPENAI),
                provider.openai_url.clone(),
                provider.openai_api_key.clone(),
                provider.openai_models.clone(),
            )
        } else if provider.minimax_api_key.is_some()
            || provider.minimax_url.is_some()
            || provider.minimax_models.is_some()
        {
            (
                Some(keychain::PROVIDER_MINIMAX),
                provider.minimax_url.clone(),
                provider.minimax_api_key.clone(),
                provider.minimax_models.clone(),
            )
        } else if provider.anthropic_api_key.is_some()
            || provider.anthropic_url.is_some()
            || provider.anthropic_models.is_some()
        {
            (
                Some(keychain::PROVIDER_ANTHROPIC),
                provider.anthropic_url.clone(),
                provider.anthropic_api_key.clone(),
                provider.anthropic_models.clone(),
            )
        } else {
            (None, None, None, None)
        };

        Self {
            provider: name.map(str::to_string),
            base_url,
            api_key,
            model,
            capabilities: capabilities.clone(),
        }
    }
}

/// Outcome of a gateway hot-config apply attempt.
enum HotApply {
    /// The gateway accepted the config; no restart is needed.
    Applied,
    /// The gateway does not support `/v1/admin/config` (404/405) — old build.
    Unsupported,
    /// The request failed for another reason (network / server error).
    Failed(String),
}

impl Default for BackendInfo {
    fn default() -> Self {
        Self {
            state: BackendState::Stopped,
            ownership: BackendOwnership::External,
            pid: None,
            endpoint: None,
            port: None,
            started_at: None,
            restart_count: 0,
            last_exit_code: None,
            last_error: None,
            backend_version: None,
        }
    }
}

/// A live owned child. Port and start time live on [`BackendInfo`] rather than
/// being duplicated here, so there is one source of truth for diagnostics.
struct BackendProcess {
    child: Child,
    pid: u32,
}

pub struct BackendSupervisor {
    info: Arc<RwLock<BackendInfo>>,
    process: Arc<RwLock<Option<BackendProcess>>>,
    logger: Option<Arc<DesktopLogger>>,
    provider_env: RwLock<BackendProviderEnv>,
    capability_env: RwLock<BackendCapabilityEnv>,
    workspace_dir: RwLock<Option<PathBuf>>,
    /// The provider env the RUNNING sidecar was actually spawned with (raw,
    /// without keychain fallback). Compared against new config to decide
    /// hot-apply vs restart: only key/base_url changes are hot-applicable.
    spawned_provider_env: RwLock<BackendProviderEnv>,
    /// The capability toggles the RUNNING sidecar was spawned with.
    spawned_capability_env: RwLock<BackendCapabilityEnv>,
    /// The loopback access token the RUNNING sidecar was spawned with (auto-
    /// minted per spawn when the parent env has none). In-memory only — never
    /// persisted, never logged; see [`resolve_sidecar_gateway_token`].
    spawned_gateway_token: RwLock<Option<String>>,
    /// W2-① 常驻协同: 启动前是否探测常驻服务 (端口/健康检查), 已有常驻 =
    /// 直连不拉起侧车 (见 [`detect_resident_gateway`])。
    ///
    /// 生产构造 (`with_logger`) 开启; 测试构造 (`new_for_test` /
    /// `with_logger_in_dir`) 关闭 —— 生命周期测试自持 spawn 面, 必须与机器上
    /// 是否装有常驻服务无关 (hermetic)。探测路径的自有测试走
    /// [`Self::with_resident_probe_in_dir`]。
    resident_probe: bool,
    /// E-07 首启 spawn 判定: 是否已收到 Settings-UI 的首次配置推送
    /// ([`BackendSupervisor::apply_backend_config`] 落值后翻真)。首次 spawn
    /// 在翻真前持有等待 —— 模型/能力族变更的重启判定因此移到 spawn 前解析。
    config_resolution: watch::Sender<bool>,
    /// The supervised lifecycle transitions so far, in order
    /// (see [`Self::lifecycle_events`] / [`LifecycleEvent`]).
    lifecycle: std::sync::Mutex<Vec<LifecycleEvent>>,
}

/// 存储文档身份: 后端 provider 环境配置 (非密字段)。
pub const PROVIDER_ENV_DOC_NAME: &str = "companion-backend-provider-env";
/// provider 环境配置存储格式版本。
pub const PROVIDER_ENV_DOC_VERSION: u32 = 1;

/// 存储文档身份: 后端能力开关配置。
pub const CAPABILITY_ENV_DOC_NAME: &str = "companion-backend-capability-env";
/// 能力开关配置存储格式版本。
pub const CAPABILITY_ENV_DOC_VERSION: u32 = 1;

/// 学习日志逐记录信封身份 (与写端 `self_tuning_wire` 落盘信封一致)。
pub const TUNING_LOG_DOC_NAME: &str = "tuning-log-record";
/// 学习日志记录格式版本。
pub const TUNING_LOG_DOC_VERSION: u32 = 1;

fn provider_env_doc_compat() -> DocCompat {
    DocCompat::exact(PROVIDER_ENV_DOC_NAME, PROVIDER_ENV_DOC_VERSION)
}

fn capability_env_doc_compat() -> DocCompat {
    DocCompat::exact(CAPABILITY_ENV_DOC_NAME, CAPABILITY_ENV_DOC_VERSION)
}

fn tuning_log_doc_compat() -> DocCompat {
    DocCompat::exact(TUNING_LOG_DOC_NAME, TUNING_LOG_DOC_VERSION)
}

/// Env key of the sidecar's loopback access token: the same variable the
/// gateway's sensitive-endpoint token gate reads (`GatewayTokenGate::ENV`).
pub const GATEWAY_TOKEN_ENV: &str = "APEIRETH_GATEWAY_TOKEN";

/// Explicit "run without protection" declaration understood by the gateway
/// token gate (`GatewayTokenGate::UNPROTECTED_SENTINEL`). Never a real token.
pub const UNPROTECTED_TOKEN_SENTINEL: &str = "none";

/// Mint one cryptographically random loopback access token: 32 random bytes
/// hex-rendered (64 chars). One token per spawn, kept in process memory only.
pub fn mint_gateway_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The loopback token the sidecar is started with, plus where it came from
/// (`env` / `minted`) for the redacted startup log line.
///
/// An explicit non-empty parent-env value passes through untouched — that
/// includes the gateway's no-protection sentinel, which is the operator's
/// own declaration and must never be silently replaced. Everything else
/// (unset / blank) mints a fresh token for this spawn.
fn resolve_sidecar_gateway_token(parent_env: Option<&str>) -> (String, &'static str) {
    match parent_env.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => (value.to_string(), "env"),
        None => (mint_gateway_token(), "minted"),
    }
}

/// `Authorization` header value carrying the loopback access token to the
/// gateway's gated endpoints.
fn bearer_value(token: &str) -> String {
    format!("Bearer {token}")
}

// ---------------------------------------------------------------------------
// W2-① 常驻协同: 常驻服务探测面 + 服务令牌面
// ---------------------------------------------------------------------------

/// 常驻占位文件名 (与 `apeireth gateway serve --pid-file` / 服务脚本写入口径
/// 一致: `key=value` 行)。
pub const RESIDENT_DESCRIPTOR_FILE: &str = "gateway.pid";

/// How long a launch-time spawn ([`BackendSupervisor::start_after_initial_config`])
/// holds out for the Settings-UI's initial config push before proceeding on the
/// restored config. Only an upper bound: the wait ends as soon as the config
/// face has spoken.
pub const INITIAL_CONFIG_WINDOW: Duration = Duration::from_secs(5);

/// 服务令牌面的凭据库目标名 (keyring 默认映射: 目标 = 用户.服务; 服务安装
/// 面按此名写入, 本壳按此名读取, TUI 走同一身份) —— 明文不落盘。
pub const SERVICE_TOKEN_TARGET: &str = "apeireth.gateway-token";

/// 常驻占位记录 (解析自 PID 文件的 `key=value` 行)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResidentDescriptor {
    /// 常驻网关进程 id (诊断元数据)。
    pub pid: u32,
    /// 常驻网关监听端口 (探测/连接面)。
    pub port: u16,
    /// 常驻网关绑定地址 (诊断元数据)。
    pub bind: String,
    /// 常驻网关占位时刻 (epoch ms)。
    pub started_at_ms: i64,
}

impl ResidentDescriptor {
    /// 解析 `key=value` 行; 缺字段/坏值 = None (探测面读不懂 = 无常驻, 不猜)。
    pub fn parse(text: &str) -> Option<Self> {
        let mut pid = None;
        let mut port = None;
        let mut bind = None;
        let mut started_at_ms = None;
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line.split_once('=')?;
            match key.trim() {
                "pid" => pid = Some(value.trim().parse().ok()?),
                "port" => port = Some(value.trim().parse().ok()?),
                "bind" => bind = Some(value.trim().to_string()),
                "started_at_ms" => started_at_ms = Some(value.trim().parse().ok()?),
                _ => {}
            }
        }
        Some(Self {
            pid: pid?,
            port: port?,
            bind: bind?,
            started_at_ms: started_at_ms?,
        })
    }

    /// 渲染回 `key=value` 行 (稳定键序; 与写入方同契约)。
    pub fn render(&self) -> String {
        format!(
            "pid={}\nport={}\nbind={}\nstarted_at_ms={}\n",
            self.pid, self.port, self.bind, self.started_at_ms
        )
    }

    /// 连接端点 (常驻网关按回环探测, 与服务脚本/网关同口径)。
    pub fn endpoint(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

/// 常驻运行目录 (`APEIRETH_RUN_DIR` 显式绝对值最高优先, 否则平台缺省)。
///
/// Windows 缺省 `%LOCALAPPDATA%\Apeireth\run` (与服务安装面同锚);
/// 其余平台 `~/.apeireth/run`。绝不相对进程 CWD (工作目录无关)。
pub fn resident_run_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("APEIRETH_RUN_DIR") {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    if cfg!(windows) {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(local).join("Apeireth").join("run");
        }
    }
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".apeireth").join("run")
}

/// 常驻占位文件完整路径。
pub fn resident_descriptor_path() -> PathBuf {
    resident_run_dir().join(RESIDENT_DESCRIPTOR_FILE)
}

/// 读常驻占位文件; 缺失/坏内容 = None (探测面: 读不懂即无常驻)。
pub fn read_resident_descriptor(path: &Path) -> Option<ResidentDescriptor> {
    let text = fs::read_to_string(path).ok()?;
    ResidentDescriptor::parse(&text)
}

/// 探测常驻服务 (端口/健康检查): 占位记录在手且 `/health` 应答 = 常驻在跑。
///
/// 这是「已有常驻 = 直连, 无常驻 = 按现行为拉起侧车」的判定面; 占位陈旧
/// (端口不应答) 读作无常驻, 走既有 spawn 路径。
pub async fn detect_resident_gateway(path: &Path) -> Option<ResidentDescriptor> {
    let descriptor = read_resident_descriptor(path)?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .ok()?;
    let response = client
        .get(format!("{}/health", descriptor.endpoint()))
        .send()
        .await
        .ok()?;
    response.status().is_success().then_some(descriptor)
}

// ---------------------------------------------------------------------------
// D12 运维探针读面: 常驻服务探测报告 (只读)
// ---------------------------------------------------------------------------

/// 常驻服务健康判定 (运维探针读面)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResidentProbeHealth {
    /// 占位在手且 `/health` 应答 = 常驻在跑。
    Live,
    /// 占位在手但端口不应答 = 陈旧占位 (不猜在跑)。
    Stale,
    /// 无占位 (或占位读不懂) = 未探测到常驻。
    Absent,
}

/// 常驻服务探测报告 —— **只读**: 只汇报占位与健康判定, 不启停、不安装、
/// 不写任何状态 (零装边界; 界面消费见设置页 daemon 状态卡)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResidentProbeReport {
    /// 占位记录是否在手 (读不懂 = false, 不猜)。
    pub descriptor_present: bool,
    /// 占位记录的连接端点; 无记录 = None。
    pub endpoint: Option<String>,
    /// 占位记录的网关进程 id (诊断元数据)。
    pub pid: Option<u32>,
    /// 健康判定。
    pub health: ResidentProbeHealth,
}

/// 探测常驻服务的只读报告 (复用探测缝 [`detect_resident_gateway`] 的同一条
/// 占位 + `/health` 双口径): 无占位 = `absent`; 占位在手: 应答 = `live`,
/// 不应答 = `stale` (占位照报, 陈旧如实标注)。
pub async fn probe_resident_service(path: &Path) -> ResidentProbeReport {
    let Some(descriptor) = read_resident_descriptor(path) else {
        return ResidentProbeReport {
            descriptor_present: false,
            endpoint: None,
            pid: None,
            health: ResidentProbeHealth::Absent,
        };
    };
    let health = if detect_resident_gateway(path).await.is_some() {
        ResidentProbeHealth::Live
    } else {
        ResidentProbeHealth::Stale
    };
    ResidentProbeReport {
        descriptor_present: true,
        endpoint: Some(descriptor.endpoint()),
        pid: Some(descriptor.pid),
        health,
    }
}

/// 运维探针 IPC 读面 (D12): 常驻服务探测报告。**只读** —— 不启停、不安装、
/// 不写状态; 失败/读不懂如实报 `absent`, 不猜。
#[tauri::command]
pub async fn get_resident_probe() -> ResidentProbeReport {
    probe_resident_service(&resident_descriptor_path()).await
}

/// 常驻服务令牌面 (G10): 从系统凭据库读同源服务令牌 (探测到常驻 = 读服务
/// 令牌面)。明文只进内存, 不落盘、不进日志; 读不到 = None (三态门照旧)。
#[cfg(windows)]
pub fn read_service_token_face() -> Option<String> {
    service_token_store::read()
}

/// 常驻服务令牌面 (G10) 的非 Windows 面: 令牌面读取暂无接口 (桌面打包面为
/// Windows) —— 无令牌 = 三态门口径照旧, 不假装取到了。
#[cfg(not(windows))]
pub fn read_service_token_face() -> Option<String> {
    None
}

/// 凭据库密文解码 (写入契约 = UTF-16LE, 与服务安装面的写入编码一致)。
///
/// 只认契约编码: 非 UTF-16LE 形状 (空 / 奇数长度 / 非法码元) 读作未配置,
/// 不猜别的编码 (猜错 = 拿到乱码当令牌, 比拿不到更坏)。
fn decode_credential_blob(blob: &[u8]) -> Option<String> {
    if blob.is_empty() || blob.len() % 2 != 0 {
        return None;
    }
    let mut units = Vec::with_capacity(blob.len() / 2);
    for chunk in blob.chunks_exact(2) {
        units.push(u16::from_le_bytes([chunk[0], chunk[1]]));
    }
    let text = String::from_utf16(&units).ok()?;
    let trimmed = text.trim_end_matches('\u{0}').trim().to_string();
    (!trimmed.is_empty()).then_some(trimmed)
}

/// 访问令牌三源收口 (纯函数, 行为可钉): spawn 期令牌 (自铸/env 透传) 赢,
/// 其次 env 显式值, 最后常驻服务令牌面 (探测到常驻 = 读服务令牌面); 逃生阀
/// 哨兵 = 明示不设防 → 无令牌可呈; 三者皆无 = None (三态门警告帧语义照旧)。
fn resolve_gateway_access_token(
    spawned: Option<String>,
    env_value: Option<String>,
    service_face: Option<String>,
) -> Option<String> {
    fn normalize(value: String) -> Option<String> {
        let trimmed = value.trim().to_string();
        (!trimmed.is_empty()).then_some(trimmed)
    }
    let chosen = spawned
        .and_then(normalize)
        .or_else(|| env_value.and_then(normalize))
        .or_else(|| service_face.and_then(normalize))?;
    (chosen != UNPROTECTED_TOKEN_SENTINEL).then_some(chosen)
}

#[cfg(windows)]
mod service_token_store {
    use windows_sys::Win32::Security::Credentials::{
        CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC,
    };

    /// 系统凭据库读一枚通用凭据 (目标 [`super::SERVICE_TOKEN_TARGET`])。
    ///
    /// 只读不写: 写入方是服务安装面 (CredWriteW, 本地机器持久)。缺失/读不到
    /// 一律 None (含 ERROR_NOT_FOUND): 令牌面缺席 = 无令牌, 三态门照旧。
    pub fn read() -> Option<String> {
        let target: Vec<u16> = super::SERVICE_TOKEN_TARGET
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let mut credential: *mut CREDENTIALW = std::ptr::null_mut();
        // SAFETY: target 是 NUL 结尾 UTF-16 缓冲, 调用期间存活; 成功返回的
        // 指针经 CredFree 释放。
        let ok = unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) };
        if ok == 0 {
            return None;
        }
        // SAFETY: 成功时 credential 是有效 CREDENTIALW 指针。
        let decoded = unsafe {
            let blob = std::slice::from_raw_parts(
                (*credential).CredentialBlob,
                (*credential).CredentialBlobSize as usize,
            );
            super::decode_credential_blob(blob)
        };
        // SAFETY: 指针来自 CredReadW, CredFree 释放。
        unsafe { CredFree(credential as *mut _) };
        decoded
    }
}

impl BackendSupervisor {
    /// Base constructor. Production always attaches a logger via
    /// [`Self::with_logger`]; tests use `build(None)` when no log file is wanted.
    ///
    /// Config restore is **strict**: a present-but-broken config file is
    /// rejected (`Err`) instead of silently falling back to defaults; a missing
    /// file legitimately means "no config yet".
    /// Base constructor (resident probe off — see
    /// [`Self::build_with_resident_probe`]). Production always attaches a logger
    /// via [`Self::with_logger`]; tests use `build(None)` when no log file is
    /// wanted.
    ///
    /// Config restore is **strict**: a present-but-broken config file is
    /// rejected (`Err`) instead of silently falling back to defaults; a missing
    /// file legitimately means "no config yet".
    fn build(logger: Option<Arc<DesktopLogger>>) -> Result<Self, String> {
        Self::build_with_resident_probe(logger, false)
    }

    /// Constructor with the W2-① resident-service probe knob spelled out.
    fn build_with_resident_probe(
        logger: Option<Arc<DesktopLogger>>,
        resident_probe: bool,
    ) -> Result<Self, String> {
        // Restore the persisted non-secret provider config (endpoints/models),
        // capability toggles, and workspace dir so the very first spawn already
        // reflects the user's last settings. Keys are deliberately NOT persisted
        // to the app-data config; they live in memory for the session and are
        // otherwise restored from the OS keychain at spawn (env > keychain).
        let (persisted_provider, persisted_capabilities, persisted_workspace) =
            match logger.as_ref() {
                Some(attached) => (
                    Self::load_persisted_provider_env(attached)?.unwrap_or_default(),
                    Self::load_persisted_capability_env(attached)?.unwrap_or_default(),
                    workspace::load_workspace_dir(&Self::logger_app_data_dir(attached))?,
                ),
                None => (
                    BackendProviderEnv::default(),
                    BackendCapabilityEnv::default(),
                    None,
                ),
            };
        Ok(Self {
            info: Arc::new(RwLock::new(BackendInfo::default())),
            process: Arc::new(RwLock::new(None)),
            logger,
            provider_env: RwLock::new(persisted_provider.clone()),
            capability_env: RwLock::new(persisted_capabilities.clone()),
            workspace_dir: RwLock::new(persisted_workspace),
            spawned_provider_env: RwLock::new(persisted_provider),
            spawned_capability_env: RwLock::new(persisted_capabilities),
            spawned_gateway_token: RwLock::new(None),
            resident_probe,
            config_resolution: watch::channel(false).0,
            lifecycle: std::sync::Mutex::new(Vec::new()),
        })
    }

    /// App-data directory that owns the logs (e.g. `%LOCALAPPDATA%\Apeireth`).
    /// `None` for logger-less test supervisors, where tests control the
    /// sidecar's store paths through their own environment.
    fn app_data_dir(&self) -> Option<PathBuf> {
        self.logger.as_ref().map(|l| Self::logger_app_data_dir(l))
    }

    /// The directory that owns the logs and every app-data config file.
    ///
    /// 星槎便携契约（K-starraft.md）：可执行文件同级出现便携标记（`portable.flag`
    /// 文件或 `data/` 目录）时，数据目录随身走 `exe_dir/data`，凌驾常规安装位置；
    /// 无标记维持常规安装链路（其余逻辑不动）。密钥不随身（仍走系统凭据库）。
    fn logger_app_data_dir(logger: &DesktopLogger) -> PathBuf {
        if let Some(portable) = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf))
            .and_then(|dir| crate::portable_mode::resolve_portable_data_dir(&dir))
        {
            return portable;
        }
        logger
            .log_directory()
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| logger.log_directory().to_path_buf())
    }

    /// App-data file for the non-secret part of the provider environment.
    fn provider_env_path(logger: &DesktopLogger) -> PathBuf {
        logger
            .log_directory()
            .parent()
            .map(|dir| dir.join("backend-provider-env.json"))
            .unwrap_or_else(|| logger.log_directory().join("backend-provider-env.json"))
    }

    /// App-data file for the capability toggles (no secrets involved).
    fn capability_env_path(logger: &DesktopLogger) -> PathBuf {
        logger
            .log_directory()
            .parent()
            .map(|dir| dir.join("backend-capability-env.json"))
            .unwrap_or_else(|| logger.log_directory().join("backend-capability-env.json"))
    }

    fn load_persisted_provider_env(
        logger: &DesktopLogger,
    ) -> Result<Option<BackendProviderEnv>, String> {
        let path = Self::provider_env_path(logger);
        let loaded: Option<BackendProviderEnv> =
            stored_config::load_config(&path, &provider_env_doc_compat())?;
        Ok(loaded
            .map(BackendProviderEnv::sanitized)
            .map(|env| env.without_secrets()))
    }

    fn load_persisted_capability_env(
        logger: &DesktopLogger,
    ) -> Result<Option<BackendCapabilityEnv>, String> {
        let path = Self::capability_env_path(logger);
        stored_config::load_config(&path, &capability_env_doc_compat())
    }

    /// Persist only the non-secret part of the provider environment so the
    /// next launch restores endpoints/models without a restart cycle.
    fn persist_provider_env(&self, env: &BackendProviderEnv) {
        let Some(logger) = &self.logger else {
            return;
        };
        let path = Self::provider_env_path(logger);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Err(error) =
            stored_config::save_config(&path, &provider_env_doc_compat(), &env.without_secrets())
        {
            self.log_desktop(
                LogLevel::Error,
                &format!("provider env persist failed: {error}"),
            );
        }
    }

    /// Persist capability toggles (no secrets) for the next launch.
    fn persist_capability_env(&self, env: &BackendCapabilityEnv) {
        let Some(logger) = &self.logger else {
            return;
        };
        let path = Self::capability_env_path(logger);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Err(error) = stored_config::save_config(&path, &capability_env_doc_compat(), env) {
            self.log_desktop(
                LogLevel::Error,
                &format!("capability env persist failed: {error}"),
            );
        }
    }

    /// Explicit migration of the persisted provider env into the current
    /// stored-document format (rewrites + audit line). The default open path
    /// never migrates on its own.
    pub fn migrate_legacy_provider_env_config(logger: &DesktopLogger) -> Result<(), String> {
        let path = Self::provider_env_path(logger);
        if !path.exists() {
            return Ok(());
        }
        stored_config::migrate_legacy_config::<BackendProviderEnv>(
            &path,
            &provider_env_doc_compat(),
        )
        .map(|_| ())
    }

    /// Explicit migration of the persisted capability toggles into the current
    /// stored-document format (rewrites + audit line).
    pub fn migrate_legacy_capability_env_config(logger: &DesktopLogger) -> Result<(), String> {
        let path = Self::capability_env_path(logger);
        if !path.exists() {
            return Ok(());
        }
        stored_config::migrate_legacy_config::<BackendCapabilityEnv>(
            &path,
            &capability_env_doc_compat(),
        )
        .map(|_| ())
    }

    /// Attach a persistent logger so backend stdout/stderr reaches
    /// `apeireth-backend.log` and lifecycle events reach the desktop log.
    ///
    /// Fails (instead of defaulting) when a persisted config file exists but is
    /// malformed / foreign / version-unsupported.
    pub fn with_logger(logger: Arc<DesktopLogger>) -> Result<Self, String> {
        Self::build_with_resident_probe(Some(logger), true)
    }

    /// A logger-less supervisor for integration tests.
    ///
    /// Tests drive the real lifecycle but must not append to the user's actual
    /// log files, so no logger is attached (and no config file is read). The
    /// resident-service probe is also off: lifecycle tests own their spawn
    /// path and must not depend on whether the machine runs a resident
    /// service (hermeticity).
    #[doc(hidden)]
    pub fn new_for_test() -> Self {
        Self::build(None).expect("logger-less supervisor reads no config files")
    }

    /// Test-only production-shaped supervisor whose logs and app-data anchors
    /// live under `dir` (the parent of `dir\logs` becomes the app-data dir).
    /// Used by the regression test proving the sidecar's stores land in
    /// app-data regardless of the launch CWD. The resident-service probe is
    /// off here (lifecycle tests keep owning their spawn path);
    /// [`Self::with_resident_probe_in_dir`] turns it on.
    #[doc(hidden)]
    pub fn with_logger_in_dir(dir: PathBuf) -> Result<Self, String> {
        let logger = DesktopLogger::new_in_dir(dir.join("logs"))?;
        Self::build(Some(Arc::new(logger)))
    }

    /// Test-only production-shaped supervisor **with** the resident-service
    /// probe enabled — the exact behavior a production desktop launch has
    /// (W2-① 常驻协同: 已有常驻 = 直连, 无常驻 = 拉起侧车).
    #[doc(hidden)]
    pub fn with_resident_probe_in_dir(dir: PathBuf) -> Result<Self, String> {
        let logger = DesktopLogger::new_in_dir(dir.join("logs"))?;
        Self::build_with_resident_probe(Some(Arc::new(logger)), true)
    }

    /// Expose dev-build resolution so an integration test can report a missing
    /// backend instead of silently passing with nothing to spawn.
    #[doc(hidden)]
    pub fn resolve_dev_backend_for_test() -> Option<PathBuf> {
        Self::resolve_dev_backend()
    }

    fn log_desktop(&self, level: LogLevel, message: &str) {
        if let Some(logger) = &self.logger {
            logger.log_desktop(level, message);
        }
    }

    /// Drain one child pipe line-by-line into the backend log.
    ///
    /// The drain task is spawned even when no logger is attached. Dropping a
    /// piped stdout/stderr handle without a reader closes the pipe; the
    /// canonical CLI then panics on its first `eprintln!` (Windows exit 101)
    /// and never binds the gateway. Tests use a logger-less supervisor, and a
    /// production logger failure must not kill the child the same way.
    ///
    /// Each logged line is redacted by [`DesktopLogger::log_backend`], so
    /// provider credentials echoed by the runtime never reach disk.
    fn pump_stream<R>(&self, stream: R, channel: &'static str)
    where
        R: tokio::io::AsyncRead + Unpin + Send + 'static,
    {
        let logger = self.logger.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stream).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(logger) = &logger {
                    logger.log_backend(&format!("[{channel}] {line}"));
                }
            }
        });
    }

    /// Kill and reap an owned child after a failed start so a dead gateway
    /// cannot hold the session database or a TCP port.
    async fn reclaim_child(&self) {
        let mut process = self.process.write().await;
        if let Some(mut backend) = process.take() {
            let _ = backend.child.start_kill();
            let _ = tokio::time::timeout(Duration::from_secs(2), backend.child.wait()).await;
        }
    }

    /// Get current backend info (safe for frontend)
    pub async fn info(&self) -> BackendInfo {
        self.info.read().await.clone()
    }

    /// Append one lifecycle transition (drives the ordered evidence face).
    fn record_event(&self, event: LifecycleEvent) {
        if let Ok(mut events) = self.lifecycle.lock() {
            events.push(event);
        }
    }

    /// The supervised lifecycle transitions seen so far, in order.
    ///
    /// The ordered start/stop face tests assert the spawn contract against
    /// (E-07: first boot = `Spawn, Ready`, nothing in between).
    pub fn lifecycle_events(&self) -> Vec<LifecycleEvent> {
        self.lifecycle
            .lock()
            .map(|events| events.clone())
            .unwrap_or_else(|poisoned| poisoned.into_inner().clone())
    }

    /// Adopt an already-running resident gateway (W2-① 常驻协同): publish its
    /// endpoint as `Ready` with `External` ownership and spawn nothing.
    /// Ownership semantics are unchanged — `stop()` still refuses to stop a
    /// backend the desktop does not own; the resident lifecycle belongs to the
    /// service management surface (`packaging/service`).
    async fn adopt_resident_gateway(&self, resident: ResidentDescriptor) -> Result<String, String> {
        let endpoint = resident.endpoint();
        let mut info = self.info.write().await;
        info.state = BackendState::Ready;
        info.ownership = BackendOwnership::External;
        info.pid = Some(resident.pid);
        info.port = Some(resident.port);
        info.endpoint = Some(endpoint.clone());
        info.last_error = None;
        drop(info);
        self.log_desktop(
            LogLevel::Info,
            &format!(
                "backend.resident_adopted endpoint={endpoint} pid={} (external resident service; no sidecar spawned)",
                resident.pid
            ),
        );
        Ok(format!(
            "Connected to resident gateway at {endpoint} (PID: {}, external service)",
            resident.pid
        ))
    }

    /// Start the owned backend process
    pub async fn start(&self) -> Result<String, String> {
        let mut info = self.info.write().await;

        if info.state == BackendState::Starting || info.state == BackendState::Ready {
            return Err("Backend already starting or running".to_string());
        }

        info.state = BackendState::Starting;
        info.last_error = None;
        drop(info);

        // W2-① 常驻协同: 先探测常驻服务 (端口/健康检查)。已有常驻 = 直连
        // (External 归属, 不重复拉起侧车); 探测不到 = 走既有 spawn 路径
        // (无常驻 = 按现行为拉起侧车, 语义零变化)。
        if self.resident_probe {
            if let Some(resident) = detect_resident_gateway(&resident_descriptor_path()).await {
                return self.adopt_resident_gateway(resident).await;
            }
        }

        // Locate bundled backend executable
        let backend_path = self.resolve_backend_binary()?;

        // Select free port
        let port = self.select_free_port().await?;

        self.log_desktop(
            LogLevel::Info,
            &format!("backend.spawn path={backend_path} port={port}"),
        );
        self.record_event(LifecycleEvent::Spawn);

        // Spawn backend process
        match self.spawn_backend(&backend_path, port).await {
            Ok(mut child) => {
                let pid = child.id().ok_or("Failed to get backend PID")?;
                let started_at = Instant::now();

                // Drain stdout/stderr into apeireth-backend.log. Taking the
                // pipes here means the child never blocks on a full OS buffer.
                if let Some(stdout) = child.stdout.take() {
                    self.pump_stream(stdout, "stdout");
                }
                if let Some(stderr) = child.stderr.take() {
                    self.pump_stream(stderr, "stderr");
                }
                self.log_desktop(
                    LogLevel::Info,
                    &format!("backend.spawned pid={pid} port={port}"),
                );
                self.watch_for_exit(pid);

                // Store process handle
                let mut process = self.process.write().await;
                *process = Some(BackendProcess { child, pid });
                drop(process);

                // Update info
                let mut info = self.info.write().await;
                info.pid = Some(pid);
                info.port = Some(port);
                info.endpoint = Some(format!("http://127.0.0.1:{}", port));
                info.started_at = Some(started_at);
                info.ownership = BackendOwnership::OwnedByDesktop;
                drop(info);

                // Probe for readiness. A timeout or child death must land in
                // Failed rather than leaving the machine stuck in Starting,
                // and the child must be reaped so the next start is not
                // blocked by a leaked sqlite lock.
                if let Err(error) = self.wait_for_ready(port).await {
                    self.reclaim_child().await;
                    let mut info = self.info.write().await;
                    info.state = BackendState::Failed;
                    info.last_error = Some(error.clone());
                    info.pid = None;
                    info.endpoint = None;
                    info.port = None;
                    drop(info);
                    self.log_desktop(LogLevel::Error, &format!("backend.ready_failed {error}"));
                    self.record_event(LifecycleEvent::ReadyFailed);
                    return Err(error);
                }

                // Transition to Ready
                let mut info = self.info.write().await;
                info.state = BackendState::Ready;
                let endpoint = info.endpoint.clone().unwrap();
                let latency_ms = started_at.elapsed().as_millis();
                drop(info);
                self.log_desktop(
                    LogLevel::Info,
                    &format!("backend.ready pid={pid} port={port} latency_ms={latency_ms}"),
                );
                self.record_event(LifecycleEvent::Ready);

                Ok(format!("Backend started at {} (PID: {})", endpoint, pid))
            }
            Err(e) => {
                let mut info = self.info.write().await;
                info.state = BackendState::Failed;
                info.last_error = Some(e.clone());
                drop(info);
                self.log_desktop(LogLevel::Error, &format!("backend.spawn_failed {e}"));
                self.record_event(LifecycleEvent::SpawnFailed);
                Err(e)
            }
        }
    }

    /// Launch-time startup entry (E-07 first-boot spawn contract).
    ///
    /// The Settings-UI pushes its provider/capability config shortly after the
    /// window opens. Model lists and capability-family toggles are NOT
    /// hot-applicable (see [`BackendProviderEnv::restart_relevant_pairs`]), so
    /// spawning before that push lands means the change is only discovered
    /// AFTER the sidecar is ready and is immediately bounced:
    /// `spawn → ready → restart required → stopped → spawn`. Holding the very
    /// first spawn until the config face resolves moves that restart decision
    /// in front of the spawn — the launch spawn then carries the resolved
    /// config and goes straight to `Ready` (`Spawn → Ready`, one process).
    ///
    /// `window` only bounds the wait: once it expires the spawn proceeds on the
    /// restored config exactly like [`Self::start`]. A config push that arrives
    /// later keeps its existing semantics (hot-apply when hot-applicable,
    /// restart otherwise) — only the launch-time bounce is removed.
    pub async fn start_after_initial_config(&self, window: Duration) -> Result<String, String> {
        let mut resolution = self.config_resolution.subscribe();
        if !*resolution.borrow() {
            let _ = tokio::time::timeout(window, resolution.wait_for(|resolved| *resolved)).await;
        }
        {
            let info = self.info.read().await;
            if matches!(info.state, BackendState::Starting | BackendState::Ready) {
                return Ok(format!(
                    "Backend already started at {}",
                    info.endpoint.clone().unwrap_or_default()
                ));
            }
        }
        match self.start().await {
            Ok(message) => Ok(message),
            Err(error) => {
                // Benign startup race: the config-apply path won the spawn
                // (with this same just-resolved config) between the state check
                // and our start. Its spawn is the launch spawn.
                let state = self.info.read().await.state.clone();
                if matches!(state, BackendState::Starting | BackendState::Ready) {
                    Ok("Backend already starting (initial config applied first)".to_string())
                } else {
                    Err(error)
                }
            }
        }
    }

    /// Stop the owned backend process
    pub async fn stop(&self) -> Result<String, String> {
        let mut info = self.info.write().await;

        if info.ownership != BackendOwnership::OwnedByDesktop {
            return Err("Cannot stop external backend".to_string());
        }

        if info.state == BackendState::Stopped {
            return Ok("Backend already stopped".to_string());
        }

        info.state = BackendState::Stopping;
        let pid = info.pid;
        drop(info);

        let mut process = self.process.write().await;
        if let Some(mut backend_process) = process.take() {
            // Attempt graceful shutdown first
            let _ = backend_process.child.start_kill();

            // Wait bounded for exit
            let timeout = Duration::from_secs(5);
            match tokio::time::timeout(timeout, backend_process.child.wait()).await {
                Ok(Ok(status)) => {
                    let mut info = self.info.write().await;
                    info.state = BackendState::Stopped;
                    info.last_exit_code = status.code();
                    info.pid = None;
                    info.endpoint = None;
                    info.port = None;
                    drop(info);
                    self.log_desktop(
                        LogLevel::Info,
                        &format!("backend.stopped pid={pid:?} exit_code={:?}", status.code()),
                    );
                    self.record_event(LifecycleEvent::Stopped);

                    Ok(format!(
                        "Backend stopped (PID: {:?}, exit: {:?})",
                        pid,
                        status.code()
                    ))
                }
                Ok(Err(e)) => {
                    let mut info = self.info.write().await;
                    info.state = BackendState::Failed;
                    info.last_error = Some(format!("Wait failed: {}", e));
                    drop(info);
                    self.log_desktop(LogLevel::Error, &format!("backend.stop_failed {e}"));
                    Err(format!("Failed to wait for backend exit: {}", e))
                }
                Err(_) => {
                    // Timeout - force kill
                    let _ = backend_process.child.kill().await;
                    let mut info = self.info.write().await;
                    info.state = BackendState::Stopped;
                    info.last_error = Some("Forced kill after timeout".to_string());
                    info.pid = None;
                    info.endpoint = None;
                    info.port = None;
                    drop(info);
                    self.log_desktop(
                        LogLevel::Warn,
                        &format!("backend.force_killed pid={pid:?} reason=graceful_timeout"),
                    );
                    self.record_event(LifecycleEvent::Stopped);

                    Ok(format!("Backend force-killed (PID: {:?})", pid))
                }
            }
        } else {
            let mut info = self.info.write().await;
            info.state = BackendState::Stopped;
            Ok("No backend process to stop".to_string())
        }
    }

    /// Restart the backend (stop + start)
    pub async fn restart(&self) -> Result<String, String> {
        let _ = self.stop().await;

        let mut info = self.info.write().await;
        info.restart_count += 1;
        drop(info);

        tokio::time::sleep(Duration::from_millis(500)).await;
        self.start().await
    }

    /// Apply the Settings-UI provider configuration.
    ///
    /// Compatibility wrapper over [`Self::apply_backend_config`]; the frontend
    /// uses the combined command so a settings save with both provider and
    /// capability changes restarts the backend exactly once.
    pub async fn apply_provider_env(&self, env: BackendProviderEnv) -> Result<BackendInfo, String> {
        self.apply_backend_config(Some(env), None).await
    }

    /// Apply the Settings-UI configuration (provider env and/or capability
    /// toggles).
    ///
    /// Stores both (keys in memory only; endpoints/models and toggles also
    /// persisted for the next launch) and makes a running backend pick the
    /// changes up. The single IPC call stays a single application:
    ///
    /// - `Ready`: hot-apply via `POST /v1/admin/config` when the gateway
    ///   supports it; a pre-admin gateway (404/405) falls back to the legacy
    ///   whole-process restart;
    /// - `Starting`/`Stopping`: wait for the in-flight transition to settle,
    ///   then hot-apply/restart or start so the change is deterministically
    ///   applied;
    /// - `Failed`: start again with the new environment (a configuration fix
    ///   is exactly what should recover a failed boot);
    /// - `Stopped`: start with the new environment (applying a config implies
    ///   the sidecar should be running).
    ///
    /// An unchanged configuration is a no-op (no restart), so a settings save
    /// that only touched UI state never bounces the gateway.
    ///
    /// A call to this face also answers the launch-time config question the
    /// held first spawn waits on (see [`Self::start_after_initial_config`]):
    /// the signal fires after the values are stored, so the launch spawn reads
    /// the resolved config (E-07).
    pub async fn apply_backend_config(
        &self,
        provider: Option<BackendProviderEnv>,
        capabilities: Option<BackendCapabilityEnv>,
    ) -> Result<BackendInfo, String> {
        let provider_changed = if let Some(env) = provider {
            let sanitized = env.sanitized();
            let changed = {
                let mut current = self.provider_env.write().await;
                if *current == sanitized {
                    false
                } else {
                    *current = sanitized.clone();
                    true
                }
            };
            self.persist_provider_env(&sanitized);
            changed
        } else {
            false
        };
        let capabilities_changed = if let Some(env) = capabilities {
            let changed = {
                let mut current = self.capability_env.write().await;
                if *current == env {
                    false
                } else {
                    *current = env.clone();
                    true
                }
            };
            self.persist_capability_env(&env);
            changed
        } else {
            false
        };

        // E-07 first-boot resolution: a call to this face answers the launch-time
        // config question. The signal fires AFTER the values are stored, so the
        // held first spawn ([`Self::start_after_initial_config`]) reads the
        // resolved config and no post-ready bounce is needed.
        self.config_resolution.send_replace(true);

        if !provider_changed && !capabilities_changed {
            return Ok(self.info().await);
        }

        let state = self.info.read().await.state.clone();
        match state {
            BackendState::Stopped | BackendState::Failed => {
                self.start_resolving_config().await?;
            }
            BackendState::Ready => {
                self.apply_to_ready_gateway().await?;
            }
            BackendState::Starting | BackendState::Stopping => {
                self.wait_for_settle().await;
                let settled = self.info.read().await.state.clone();
                match settled {
                    BackendState::Ready => {
                        self.apply_to_ready_gateway().await?;
                    }
                    BackendState::Stopped | BackendState::Failed => {
                        self.start_resolving_config().await?;
                    }
                    _ => {}
                }
            }
        }
        Ok(self.info().await)
    }

    /// Start the sidecar for a config application, tolerating the benign
    /// launch-time race (E-07): when the held first spawn won with this same
    /// just-resolved config, its spawn is the one that counts.
    async fn start_resolving_config(&self) -> Result<(), String> {
        if let Err(error) = self.start().await {
            let state = self.info.read().await.state.clone();
            if !matches!(state, BackendState::Starting | BackendState::Ready) {
                return Err(error);
            }
        }
        Ok(())
    }

    /// Apply the current config to a gateway that is already `Ready`.
    ///
    /// key/base_url 变更走 `/v1/admin/config` 热应用（凭据按请求现解析，
    /// 下一请求生效）；模型列表 / 能力开关等其余 provider 环境变更仍走重启
    /// （见 `restart_relevant_pairs`）。
    /// 仅当 provider 环境与 spawn 期完全一致、且能力开关未变时，热应用才是
    /// 幂等回写。A gateway without the admin endpoint (old build) always takes
    /// the restart path.
    async fn apply_to_ready_gateway(&self) -> Result<(), String> {
        let hot_ok = self.hot_apply_sufficient().await;
        if hot_ok {
            match self.try_hot_apply().await {
                HotApply::Applied => {
                    self.log_desktop(LogLevel::Info, "backend.hot_apply applied");
                    Ok(())
                }
                HotApply::Unsupported => {
                    self.log_desktop(
                        LogLevel::Info,
                        "backend.hot_apply unsupported (pre-admin gateway); restarting",
                    );
                    self.record_event(LifecycleEvent::RestartRequired);
                    self.restart().await.map(|_| ())
                }
                HotApply::Failed(error) => {
                    self.log_desktop(
                        LogLevel::Warn,
                        &format!("backend.hot_apply failed ({error}); restarting"),
                    );
                    self.record_event(LifecycleEvent::RestartRequired);
                    self.restart().await.map(|_| ())
                }
            }
        } else {
            self.log_desktop(
                LogLevel::Info,
                "backend.restart required (models/capabilities/other-family change is not hot-applicable)",
            );
            self.record_event(LifecycleEvent::RestartRequired);
            self.restart().await.map(|_| ())
        }
    }

    /// Whether the pending config change can be applied without a restart:
    /// only the restart-relevant pairs (everything except the openai-compatible
    /// key/base_url) and capability toggles must be unchanged vs what the
    /// running sidecar was spawned with.
    async fn hot_apply_sufficient(&self) -> bool {
        let current = self.provider_env.read().await.clone();
        let spawned = self.spawned_provider_env.read().await.clone();
        let caps_current = self.capability_env.read().await.clone();
        let caps_spawned = self.spawned_capability_env.read().await.clone();
        current.restart_relevant_pairs() == spawned.restart_relevant_pairs()
            && caps_current == caps_spawned
    }

    /// POST the current config to `{endpoint}/v1/admin/config`.
    async fn try_hot_apply(&self) -> HotApply {
        let Some(endpoint) = self.info.read().await.endpoint.clone() else {
            return HotApply::Failed("gateway endpoint not available".to_string());
        };
        let provider_env = self.provider_env.read().await.clone();
        let provider_env = provider_env.with_keychain_fallback(keychain::get_provider_key);
        let capabilities = self.capability_env.read().await.clone();
        let payload = AdminConfigRequest::from_env(&provider_env, &capabilities);
        let url = format!("{endpoint}/v1/admin/config");

        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
        {
            Ok(client) => client,
            Err(error) => return HotApply::Failed(format!("client build failed: {error}")),
        };

        let request = client.post(&url).json(&payload);
        let request = match self.gateway_access_token().await {
            Some(token) => request.header(reqwest::header::AUTHORIZATION, bearer_value(&token)),
            None => request,
        };

        match request.send().await {
            Ok(response) if response.status().is_success() => HotApply::Applied,
            Ok(response)
                if response.status() == reqwest::StatusCode::NOT_FOUND
                    || response.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED =>
            {
                HotApply::Unsupported
            }
            Ok(response) => HotApply::Failed(format!("HTTP {}", response.status())),
            Err(error) => HotApply::Failed(error.to_string()),
        }
    }

    /// The loopback access token to present on the gateway's gated endpoints
    /// (admin config / approval resolve).
    ///
    /// Three sources, in order (see [`resolve_gateway_access_token`]): the
    /// value the running sidecar was spawned with (the explicit env value or
    /// the minted one), the parent env value (the fallback for an external
    /// backend the desktop did not spawn), and — when both are absent and a
    /// resident service is connected (W2-① G10 协同) — the **service token
    /// face** (探测到常驻 = 读服务令牌面, 同源令牌). The no-protection sentinel
    /// means the gateway gate is declared open — there is nothing to present,
    /// so `None`. The value never enters logs or persisted state.
    pub async fn gateway_access_token(&self) -> Option<String> {
        let spawned = self.spawned_gateway_token.read().await.clone();
        let env_value = std::env::var(GATEWAY_TOKEN_ENV).ok();
        let env_present = env_value
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty());
        // 服务令牌面只在前两源缺席时读 (凭据库访问有成本, 也不做无谓触碰)。
        let service_face = if spawned.is_some() || env_present {
            None
        } else {
            read_service_token_face()
        };
        resolve_gateway_access_token(spawned, env_value, service_face)
    }

    /// Fetch the gateway's effective config (`GET /v1/admin/config`) and pass
    /// the JSON body through for the settings UI to echo back.
    pub async fn get_gateway_effective_config(&self) -> Result<serde_json::Value, String> {
        let endpoint = self
            .info
            .read()
            .await
            .endpoint
            .clone()
            .ok_or_else(|| "gateway not running".to_string())?;
        let url = format!("{endpoint}/v1/admin/config");
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| format!("failed to build admin client: {e}"))?;
        let request = client.get(&url);
        let request = match self.gateway_access_token().await {
            Some(token) => request.header(reqwest::header::AUTHORIZATION, bearer_value(&token)),
            None => request,
        };
        let response = request
            .send()
            .await
            .map_err(|e| format!("gateway admin config request failed: {e}"))?;

        if response.status().is_success() {
            response
                .json::<serde_json::Value>()
                .await
                .map_err(|e| format!("gateway admin config returned invalid JSON: {e}"))
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(format!(
                "gateway admin config unavailable (HTTP {status}): {body}"
            ))
        }
    }

    /// The current workspace directory, or an empty string when unset.
    pub async fn get_workspace_dir(&self) -> String {
        self.workspace_dir
            .read()
            .await
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    }

    /// Validate, persist and apply a workspace directory, returning the new
    /// (normalized) value.
    ///
    /// The sidecar's tools (filesystem/search/repo/shell) root at its CWD, so
    /// when the value actually changes and the backend is running, the sidecar
    /// is restarted with the new root — a ~1s process restart, transparent to
    /// the user (per-conversation workspace switching, 2026-10-06).
    pub async fn set_workspace_dir(&self, dir: String) -> Result<String, String> {
        let raw = PathBuf::from(dir.trim());
        if raw.as_os_str().is_empty() {
            return Err("workspace directory must not be empty".to_string());
        }
        let normalized = workspace::normalize_dir(&raw);
        workspace::ensure_writable_dir(&normalized)?;

        let changed = {
            let mut current = self.workspace_dir.write().await;
            let changed = current.as_deref() != Some(normalized.as_path());
            *current = Some(normalized.clone());
            changed
        };

        // The path is non-secret, so plain JSON under app-data is fine.
        if let Some(app_data) = self.app_data_dir() {
            workspace::persist_workspace_dir(&app_data, &normalized)?;
        }

        let value = normalized.to_string_lossy().to_string();
        if changed {
            let state = self.info.read().await.state.clone();
            if state == BackendState::Ready {
                self.log_desktop(
                    LogLevel::Info,
                    "backend.restart required (workspace dir changed; tools reroot)",
                );
                self.restart().await.map(|_| ())?;
            }
        }
        Ok(value)
    }

    /// Common workspace candidates: home, documents, desktop, last-used.
    pub async fn list_workspace_suggestions(&self) -> Vec<String> {
        let last = self.workspace_dir.read().await.clone();
        workspace::workspace_suggestions(last.as_deref())
    }

    /// 学习日志 `tuning-log.jsonl` 落位: 与侧车 session db 同目录 ——
    /// 显式绝对 `APEIRETH_SESSION_DB` 最高优先 (镜像 spawn_backend 的锚定规则),
    /// 否则 store 目录 (`resolve_store_dir`: workspace/.apeireth 或 app-data/data),
    /// 与后端接线层 `runtime-assembly::tuning_log_path` 同落位。
    pub async fn tuning_log_path(&self) -> PathBuf {
        if let Ok(value) = std::env::var("APEIRETH_SESSION_DB") {
            let session_db = PathBuf::from(value);
            if session_db.is_absolute() {
                if let Some(dir) = session_db.parent() {
                    return dir.join(TUNING_LOG_FILE);
                }
            }
        }
        if let Some(data_dir) = self.app_data_dir() {
            let workspace_dir = self.workspace_dir.read().await.clone();
            let store_dir = workspace::resolve_store_dir(workspace_dir.as_deref(), Some(&data_dir))
                .unwrap_or_else(|| data_dir.join("data"));
            return store_dir.join(TUNING_LOG_FILE);
        }
        PathBuf::from(".apeireth").join(TUNING_LOG_FILE)
    }

    /// 只读读取学习日志 (调参面板「学习日志」数据源)。
    ///
    /// 好记录照读 (旧裸体行只读兼容); 坏行读作不存在并计数, 计数非零时如实
    /// 落桌面日志 (不静默丢行)。UI 契约仍是记录列表, 跳过计数走日志可观测。
    pub async fn read_tuning_log(&self) -> Result<Vec<TuningLogEntry>, String> {
        let path = self.tuning_log_path().await;
        let read = read_tuning_log_file(&path)?;
        if read.skipped > 0 {
            self.log_desktop(
                LogLevel::Warn,
                &format!(
                    "tuning log: {} bad line(s) skipped (read as nonexistent): {}",
                    read.skipped,
                    path.display()
                ),
            );
        }
        Ok(read.entries)
    }

    /// Wait until the state machine leaves its transitional states.
    async fn wait_for_settle(&self) {
        for _ in 0..60 {
            let state = self.info.read().await.state.clone();
            if state != BackendState::Starting && state != BackendState::Stopping {
                return;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }

    /// Platform executable name for the canonical CLI.
    pub fn backend_executable_name() -> &'static str {
        if cfg!(windows) {
            "apeireth.exe"
        } else {
            "apeireth"
        }
    }

    /// Find the packaged backend next to the running executable.
    ///
    /// This is the installed layout: Tauri places `externalBin` sidecars in the
    /// same directory as the app binary, so an install needs no source tree,
    /// no Cargo, and no `target/`.
    fn resolve_bundled_backend() -> Option<PathBuf> {
        let exe = std::env::current_exe().ok()?;
        let dir = exe.parent()?;
        let name = Self::backend_executable_name();

        // Alongside the app binary (Windows/Linux), then macOS .app layouts.
        let candidates = [
            dir.join(name),
            dir.join("resources").join(name),
            dir.join("../Resources").join(name),
        ];
        candidates.into_iter().find(|path| path.is_file())
    }

    /// Walk up from `start` to the Cargo workspace root that owns the CLI.
    ///
    /// The Tauri crate is its own isolated workspace whose cwd during
    /// `tauri dev` is `frontend/companion-desktop/src-tauri`, so joining
    /// `target/` onto the cwd finds the *desktop* build directory rather than
    /// the workspace that actually builds `apeireth`. Anchoring on
    /// `crates/adapters/cli` identifies the correct root regardless of cwd.
    fn find_cli_workspace_root(start: &Path) -> Option<PathBuf> {
        start
            .ancestors()
            .find(|dir| dir.join("crates/adapters/cli/Cargo.toml").is_file())
            .map(Path::to_path_buf)
    }

    /// Resolve a development build of the canonical CLI from the workspace.
    fn resolve_dev_backend() -> Option<PathBuf> {
        let name = Self::backend_executable_name();
        let anchors = [
            std::env::current_dir().ok(),
            std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(Path::to_path_buf)),
            Some(PathBuf::from(env!("CARGO_MANIFEST_DIR"))),
        ];

        for anchor in anchors.into_iter().flatten() {
            let Some(root) = Self::find_cli_workspace_root(&anchor) else {
                continue;
            };
            // Release before debug: an optimized backend is preferred when both exist.
            for profile in ["release", "debug"] {
                let candidate = root.join("target").join(profile).join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
        None
    }

    /// Resolve the canonical backend executable.
    ///
    /// Bundled sidecar wins in every configuration; a workspace build is only a
    /// development fallback. An installed app therefore never depends on a
    /// source checkout, and a dev run still works before packaging exists.
    fn resolve_backend_binary(&self) -> Result<String, String> {
        if let Some(path) = Self::resolve_bundled_backend() {
            return Ok(path.to_string_lossy().to_string());
        }

        if let Some(path) = Self::resolve_dev_backend() {
            self.log_desktop(
                LogLevel::Warn,
                "backend.resolve source=workspace_dev_build (no bundled sidecar found)",
            );
            return Ok(path.to_string_lossy().to_string());
        }

        Err(format!(
            "canonical backend executable '{}' not found beside the app or in a workspace target directory",
            Self::backend_executable_name()
        ))
    }

    /// Select a free localhost port
    async fn select_free_port(&self) -> Result<u16, String> {
        // Try to bind to port 0 to let OS assign a free port
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .map_err(|e| format!("Failed to bind ephemeral port: {}", e))?;

        let port = listener
            .local_addr()
            .map_err(|e| format!("Failed to get local addr: {}", e))?
            .port();

        drop(listener); // Release the port
        Ok(port)
    }

    /// Canonical spawn arguments, verified against the CLI's own help text:
    /// `apeireth gateway serve [--port PORT]`.
    pub fn spawn_args(port: u16) -> Vec<String> {
        vec![
            "gateway".to_string(),
            "serve".to_string(),
            "--port".to_string(),
            port.to_string(),
        ]
    }

    /// Spawn the backend process
    async fn spawn_backend(&self, binary_path: &str, port: u16) -> Result<Child, String> {
        let mut cmd = Command::new(binary_path);
        cmd.args(Self::spawn_args(port))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .kill_on_drop(true);

        // Inject the Settings-UI provider configuration as the child's
        // environment. This is the packaged-desktop equivalent of the user
        // exporting provider env vars before running `apeireth gateway serve`.
        // Priority: in-memory env > OS keychain (env wins when a key was pushed
        // this session; the keychain restores it on later launches).
        let provider_env = self.provider_env.read().await.clone();
        // Snapshot what THIS process is spawned with (raw, keychain-less) so
        // later config changes can be classified hot-able or restart-required.
        *self.spawned_provider_env.write().await = provider_env.clone();
        *self.spawned_capability_env.write().await = self.capability_env.read().await.clone();
        let provider_env = provider_env.with_keychain_fallback(keychain::get_provider_key);
        for (key, value) in provider_env.env_pairs() {
            cmd.env(key, value);
        }
        // Same for the advanced-capability toggles (fail-closed: only "1"
        // values are emitted).
        let capability_env = self.capability_env.read().await.clone();
        for (key, value) in capability_env.env_pairs() {
            cmd.env(key, value);
        }

        // G10 loopback hardening: the sidecar's sensitive endpoints (admin
        // config / approval resolve) are token-gated, and the gate must be
        // enforced in production. The parent env's explicit value passes
        // through untouched (including the gateway's no-protection sentinel);
        // otherwise a fresh 32-byte random token is minted for THIS spawn and
        // injected. Process-env surface only — never persisted to disk, so a
        // restart legitimately rotates it.
        let (gateway_token, token_source) =
            resolve_sidecar_gateway_token(std::env::var(GATEWAY_TOKEN_ENV).ok().as_deref());
        cmd.env(GATEWAY_TOKEN_ENV, &gateway_token);
        self.log_desktop(
            LogLevel::Info,
            &format!("backend.gateway_token source={token_source} value=[REDACTED]"),
        );
        *self.spawned_gateway_token.write().await = Some(gateway_token);

        // Anchor the sidecar's persistent stores to absolute paths and pin its
        // working directory to app-data. The canonical CLI defaults its
        // session/cognitive DBs to RELATIVE `.apeireth/...` paths, which break
        // when the desktop is launched from a non-writable CWD (Start-menu
        // shortcuts run with CWD = System32 → "failed to create parent
        // directory: 拒绝访问 (os error 5)", gateway exits 1 — real-world
        // boot failure, 2026-09-28). With a logger attached (production),
        // app data = the directory that owns `logs/`.
        if let Some(data_dir) = self.app_data_dir() {
            let workspace_dir = self.workspace_dir.read().await.clone();
            let store_dir = workspace::resolve_store_dir(workspace_dir.as_deref(), Some(&data_dir))
                .unwrap_or_else(|| data_dir.join("data"));

            // Explicit ABSOLUTE env vars are the highest priority; anchor the
            // defaults otherwise. Relative env values (e.g. a stale
            // `.apeireth/...` left in the inherited environment) are treated
            // as absent: honoring them would resurrect the 2026-09-28 CWD
            // boot failure (System32 + relative path = Access Denied).
            let session_env = std::env::var("APEIRETH_SESSION_DB").ok();
            let cognitive_env = std::env::var("APEIRETH_COGNITIVE_DB").ok();
            let anchor_session = session_env
                .as_deref()
                .map_or(true, |v| !Path::new(v).is_absolute());
            let anchor_cognitive = cognitive_env
                .as_deref()
                .map_or(true, |v| !Path::new(v).is_absolute());

            // Only create the store dir when we are actually going to anchor
            // paths into it: an explicit absolute env store dir is the user's
            // own location and must not cause app-data side effects.
            if anchor_session || anchor_cognitive {
                let _ = fs::create_dir_all(&store_dir);
            }
            if anchor_session {
                cmd.env("APEIRETH_SESSION_DB", store_dir.join("sessions.sqlite3"));
            }
            if anchor_cognitive {
                cmd.env("APEIRETH_COGNITIVE_DB", store_dir.join("cognitive.sqlite3"));
            }
            // Pin the sidecar's CWD. A configured workspace wins: the builtin
            // tools (filesystem/search/repo) and TrustedShell root themselves at
            // the sidecar's current_dir, so pinning to the user's chosen
            // workspace makes the model actually work on their real project
            // instead of the app-data dir (2026-10-06 真机: 模型报"此非 git
            // 库"且读不到用户项目). The anchor is always ABSOLUTE, so the
            // System32-CWD boot regression cannot resurface.
            let cwd = workspace_dir.unwrap_or(data_dir);
            cmd.current_dir(&cwd);
        }

        // Keep the child in the app's lifetime, not the user's screen: without
        // CREATE_NO_WINDOW a console window flashes on every launch.
        // `tokio::process::Command` exposes `creation_flags` inherently on
        // Windows, so no std extension trait import is needed.
        #[cfg(windows)]
        {
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        cmd.spawn()
            .map_err(|e| format!("Failed to spawn backend: {}", e))
    }

    /// Watch an owned child and record its exit.
    ///
    /// Without this, a backend that dies after reaching Ready leaves the UI
    /// showing a healthy state forever. Exit code and restart bookkeeping are
    /// recorded so the diagnostics surface can tell the truth.
    fn watch_for_exit(&self, pid: u32) {
        let info = Arc::clone(&self.info);
        let process = Arc::clone(&self.process);
        let logger = self.logger.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(750)).await;

                let mut guard = process.write().await;
                let Some(backend) = guard.as_mut() else {
                    return; // Deliberate stop() took the handle; nothing to report.
                };
                if backend.pid != pid {
                    return; // A restart replaced this child; its own watcher owns it.
                }

                match backend.child.try_wait() {
                    Ok(Some(status)) => {
                        guard.take();
                        drop(guard);

                        let mut info = info.write().await;
                        // Only an unexpected death is a failure; a requested
                        // stop has already moved the state to Stopping/Stopped.
                        if matches!(info.state, BackendState::Starting | BackendState::Ready) {
                            info.state = BackendState::Failed;
                            info.last_exit_code = status.code();
                            info.last_error = Some(format!(
                                "backend exited unexpectedly (code {:?})",
                                status.code()
                            ));
                            info.pid = None;
                            info.endpoint = None;
                            info.port = None;
                            if let Some(logger) = &logger {
                                logger.log_desktop(
                                    LogLevel::Error,
                                    &format!(
                                        "backend.exited_unexpectedly pid={pid} exit_code={:?}",
                                        status.code()
                                    ),
                                );
                            }
                        }
                        return;
                    }
                    Ok(None) => continue, // Still running.
                    Err(error) => {
                        drop(guard);
                        if let Some(logger) = &logger {
                            logger.log_desktop(
                                LogLevel::Warn,
                                &format!("backend.watch_failed pid={pid} error={error}"),
                            );
                        }
                        return;
                    }
                }
            }
        });
    }

    /// Wait for backend to become ready (health probe)
    async fn wait_for_ready(&self, port: u16) -> Result<(), String> {
        let endpoint = format!("http://127.0.0.1:{}/health", port);
        let timeout = Duration::from_secs(15);
        let start = Instant::now();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .map_err(|error| format!("failed to build health client: {error}"))?;

        loop {
            if start.elapsed() > timeout {
                return Err(format!("Backend startup timeout after {:?}", timeout));
            }

            // A child that dies during bootstrap (broken stdio, bind failure,
            // sqlite lock) must fail immediately. Waiting out the 15s budget
            // hides the real exit and poisons the next start.
            {
                let info = self.info.read().await;
                if info.state == BackendState::Failed {
                    return Err(info
                        .last_error
                        .clone()
                        .unwrap_or_else(|| "backend failed during startup".to_string()));
                }
            }
            {
                let mut guard = self.process.write().await;
                if let Some(backend) = guard.as_mut() {
                    if let Ok(Some(status)) = backend.child.try_wait() {
                        return Err(format!(
                            "backend exited during startup (code {:?})",
                            status.code()
                        ));
                    }
                }
            }

            match client.get(&endpoint).send().await {
                Ok(response) if response.status().is_success() => {
                    return Ok(());
                }
                _ => {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    }
}

// No `Drop` impl: stopping the child needs async, which `drop` cannot await.
// Shutdown of the owned backend is driven by the `RunEvent::ExitRequested`
// handler in `lib.rs`, which can block on `stop()`.

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::process::Command;

    /// Serializes env-mutating tests (`std::env` is process-global).
    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Scoped process-env mutation: restores previous values on drop.
    struct EnvGuard(Vec<(&'static str, Option<std::ffi::OsString>)>);

    impl EnvGuard {
        fn set(vars: &[(&'static str, &str)]) -> Self {
            let mut saved = Vec::new();
            for (key, value) in vars {
                saved.push((*key, std::env::var_os(key)));
                std::env::set_var(key, value);
            }
            Self(saved)
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, value) in self.0.drain(..) {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime")
    }

    /// Verified against the CLI's own help output:
    /// `apeireth gateway serve [--port PORT]`.
    #[test]
    fn spawn_args_match_canonical_cli_contract() {
        assert_eq!(
            BackendSupervisor::spawn_args(52719),
            vec!["gateway", "serve", "--port", "52719"]
        );
    }

    #[test]
    fn spawn_args_never_reference_legacy_backend() {
        let args = BackendSupervisor::spawn_args(8080).join(" ");
        for legacy in ["companion_serve", "8090", "target/debug", "examples"] {
            assert!(
                !args.contains(legacy),
                "legacy token {legacy:?} leaked into spawn args: {args}"
            );
        }
    }

    /// 自铸令牌 (G10): 32 随机字节的十六进制 (64 字符), 每次调用一枚新的。
    #[test]
    fn minted_gateway_token_is_32_random_bytes_hex() {
        let token = mint_gateway_token();
        assert_eq!(token.len(), 64, "32 字节 → 64 个十六进制字符: {token}");
        assert!(
            token.chars().all(|c| c.is_ascii_hexdigit()),
            "令牌只含十六进制: {token}"
        );

        let mut seen = std::collections::HashSet::new();
        for _ in 0..32 {
            assert!(seen.insert(mint_gateway_token()), "每枚令牌必须唯一");
        }
    }

    /// 注入口径: 显式 env 值原样透传 (含逃生阀哨兵), 未设/空白才自铸。
    #[test]
    fn sidecar_gateway_token_passes_through_explicit_env_and_mints_when_unset() {
        let (value, source) = resolve_sidecar_gateway_token(Some("  operator-secret  "));
        assert_eq!(value, "operator-secret", "显式值透传 (去空白)");
        assert_eq!(source, "env");

        // 逃生阀哨兵必须原样透传给侧车 —— 它是操作者的显式声明, 不得被替换。
        let (value, source) = resolve_sidecar_gateway_token(Some(UNPROTECTED_TOKEN_SENTINEL));
        assert_eq!(value, UNPROTECTED_TOKEN_SENTINEL);
        assert_eq!(source, "env");

        for unset in [None, Some(""), Some("   ")] {
            let (value, source) = resolve_sidecar_gateway_token(unset);
            assert_eq!(source, "minted", "未设即自铸 ({unset:?})");
            assert_eq!(value.len(), 64);
        }
    }

    /// 桌面自用的令牌面: 呈现的令牌恒等于运行中侧车被注入的那枚 (spawn 期
    /// 决定); 未自持 (外部后端) 时回落 env; 哨兵 = 门已明示不设防 → 无令牌
    /// 可呈, 回 None。
    #[test]
    fn gateway_access_token_tracks_the_spawned_sidecar_and_honors_the_sentinel() {
        let _lock = env_lock();
        runtime().block_on(async {
            let supervisor = BackendSupervisor::new_for_test();

            // 外部后端 (无 spawn): 回落 env 显式值。
            let _guard = EnvGuard::set(&[(GATEWAY_TOKEN_ENV, "operator-secret")]);
            assert_eq!(
                supervisor.gateway_access_token().await.as_deref(),
                Some("operator-secret"),
                "未自持时回落 env (外部后端口径)"
            );

            // 自持后端: spawn 期注入的那枚说了算 (env 后改也不动摇)。
            *supervisor.spawned_gateway_token.write().await = Some("minted-for-test".into());
            assert_eq!(
                supervisor.gateway_access_token().await.as_deref(),
                Some("minted-for-test"),
                "运行中侧车执行的是 spawn 期注入值"
            );

            // 逃生阀哨兵 (env 显式或被原样透传): 门明示不设防, 无令牌可呈。
            *supervisor.spawned_gateway_token.write().await =
                Some(UNPROTECTED_TOKEN_SENTINEL.into());
            assert_eq!(supervisor.gateway_access_token().await, None);
        });
    }

    /// The injected variable names must match the canonical CLI contract
    /// (see `crates/adapters/cli` + `crates/engine/provider/src/credentials.rs`).
    #[test]
    fn provider_env_pairs_match_canonical_cli_contract() {
        let env = BackendProviderEnv {
            openai_api_key: Some("sk-openai".into()),
            openai_url: Some("https://api.deepseek.com/v1".into()),
            openai_models: Some("deepseek-v4-flash".into()),
            minimax_api_key: Some("sk-mm".into()),
            minimax_url: Some("https://api.minimax.chat/v1".into()),
            minimax_models: Some("minimax-m3".into()),
            anthropic_api_key: Some("sk-ant".into()),
            anthropic_url: Some("https://api.anthropic.com".into()),
            anthropic_models: Some("claude-sonnet-4-5".into()),
        };
        let pairs: Vec<(&str, &str)> = env.env_pairs();
        let mut map = std::collections::HashMap::new();
        for (key, value) in pairs {
            map.insert(key, value);
        }
        assert_eq!(map["OPENAI_API_KEY"], "sk-openai");
        assert_eq!(map["APEIRETH_OPENAI_URL"], "https://api.deepseek.com/v1");
        assert_eq!(map["APEIRETH_OPENAI_MODELS"], "deepseek-v4-flash");
        assert_eq!(map["APEIRETH_API_KEY"], "sk-mm");
        assert_eq!(map["APEIRETH_API_URL"], "https://api.minimax.chat/v1");
        assert_eq!(map["APEIRETH_API_MODELS"], "minimax-m3");
        assert_eq!(map["APEIRETH_ANTHROPIC_KEY"], "sk-ant");
        assert_eq!(map["APEIRETH_ANTHROPIC_URL"], "https://api.anthropic.com");
        assert_eq!(map["APEIRETH_ANTHROPIC_MODELS"], "claude-sonnet-4-5");
        assert_eq!(map.len(), 9);
    }

    /// key/base_url 可经 `/v1/admin/config` 热更（凭据按请求现解析，下一请求
    /// 生效），不列入重启相关对；模型列表等其余注入仍需重启。
    #[test]
    fn key_and_base_url_are_hot_applicable_not_restart_relevant() {
        let env = BackendProviderEnv {
            openai_api_key: Some("sk-openai".into()),
            openai_url: Some("https://api.deepseek.com/v1".into()),
            openai_models: Some("deepseek-v4-flash".into()),
            ..Default::default()
        };
        let restart: std::collections::HashMap<&str, &str> =
            env.restart_relevant_pairs().into_iter().collect();
        assert!(!restart.contains_key("OPENAI_API_KEY"), "{restart:?}");
        assert!(!restart.contains_key("APEIRETH_OPENAI_URL"), "{restart:?}");
        assert_eq!(restart["APEIRETH_OPENAI_MODELS"], "deepseek-v4-flash");

        // 仅 key/URL 变更 → 重启相关对不变（hot_apply_sufficient 判定走热更）。
        let changed = BackendProviderEnv {
            openai_api_key: Some("sk-rotated".into()),
            openai_url: Some("https://api.deepseek.com/v2".into()),
            ..env.clone()
        };
        assert_eq!(
            changed.restart_relevant_pairs(),
            env.restart_relevant_pairs(),
            "key/base_url 变更必须落在热更通道, 不触发重启"
        );
    }

    #[test]
    fn provider_env_sanitized_trims_and_drops_empties() {
        let env = BackendProviderEnv {
            openai_api_key: Some("  sk-key  ".into()),
            openai_url: Some("   ".into()),
            openai_models: Some("deepseek-v4-flash".into()),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(env.openai_api_key.as_deref(), Some("sk-key"));
        assert_eq!(env.openai_url, None);
        assert_eq!(env.openai_models.as_deref(), Some("deepseek-v4-flash"));
        assert!(BackendProviderEnv::default().is_empty());
    }

    #[test]
    fn provider_env_without_secrets_keeps_no_keys() {
        let env = BackendProviderEnv {
            openai_api_key: Some("sk-openai".into()),
            openai_url: Some("https://api.deepseek.com/v1".into()),
            minimax_api_key: Some("sk-mm".into()),
            anthropic_api_key: Some("sk-ant".into()),
            anthropic_url: Some("https://api.anthropic.com".into()),
            ..Default::default()
        };
        let stripped = env.without_secrets();
        assert_eq!(stripped.openai_api_key, None);
        assert_eq!(stripped.minimax_api_key, None);
        assert_eq!(stripped.anthropic_api_key, None);
        assert_eq!(
            stripped.openai_url.as_deref(),
            Some("https://api.deepseek.com/v1")
        );
        assert_eq!(
            stripped.anthropic_url.as_deref(),
            Some("https://api.anthropic.com")
        );
    }

    /// The serialized shape is what the frontend sends over IPC.
    #[test]
    fn provider_env_serde_shape_is_snake_case() {
        let env = BackendProviderEnv {
            openai_models: Some("deepseek-v4-flash".into()),
            ..Default::default()
        };
        let json = serde_json::to_string(&env).unwrap();
        assert!(
            json.contains("\"openai_models\":\"deepseek-v4-flash\""),
            "{json}"
        );
        let round: BackendProviderEnv = serde_json::from_str(&json).unwrap();
        assert_eq!(round, env);
    }

    #[test]
    fn provider_env_keychain_fallback_env_wins_over_keychain() {
        let env = BackendProviderEnv {
            openai_api_key: Some("sk-env".into()),
            ..Default::default()
        };
        let merged = env.with_keychain_fallback(|provider| Some(format!("sk-keychain-{provider}")));
        assert_eq!(merged.openai_api_key.as_deref(), Some("sk-env"));
        assert_eq!(
            merged.minimax_api_key.as_deref(),
            Some("sk-keychain-minimax")
        );
        assert_eq!(
            merged.anthropic_api_key.as_deref(),
            Some("sk-keychain-anthropic")
        );
    }

    #[test]
    fn provider_env_keychain_fallback_fills_missing_keys_only() {
        let env = BackendProviderEnv {
            minimax_url: Some("https://api.minimax.chat/v1".into()),
            ..Default::default()
        };
        let merged = env.with_keychain_fallback(|provider| match provider {
            keychain::PROVIDER_OPENAI => Some("sk-oai".into()),
            keychain::PROVIDER_MINIMAX => Some("sk-mm".into()),
            keychain::PROVIDER_ANTHROPIC => Some("sk-ant".into()),
            _ => None,
        });
        assert_eq!(merged.openai_api_key.as_deref(), Some("sk-oai"));
        assert_eq!(merged.minimax_api_key.as_deref(), Some("sk-mm"));
        assert_eq!(merged.anthropic_api_key.as_deref(), Some("sk-ant"));
        assert_eq!(
            merged.minimax_url.as_deref(),
            Some("https://api.minimax.chat/v1"),
            "non-key fields must pass through unchanged"
        );
    }

    #[test]
    fn admin_config_request_emits_active_provider_fields() {
        let provider = BackendProviderEnv {
            openai_api_key: Some("sk-oai".into()),
            openai_url: Some("https://api.deepseek.com/v1".into()),
            openai_models: Some("deepseek-v4-flash".into()),
            ..Default::default()
        };
        let caps = BackendCapabilityEnv {
            enable_shell: true,
            ..Default::default()
        };
        let request = AdminConfigRequest::from_env(&provider, &caps);
        assert_eq!(request.provider.as_deref(), Some("openai"));
        assert_eq!(
            request.base_url.as_deref(),
            Some("https://api.deepseek.com/v1")
        );
        assert_eq!(request.api_key.as_deref(), Some("sk-oai"));
        assert_eq!(request.model.as_deref(), Some("deepseek-v4-flash"));
        assert!(request.capabilities.enable_shell);

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"provider\":\"openai\""), "{json}");
        assert!(json.contains("\"base_url\""), "{json}");
        assert!(json.contains("\"api_key\":\"sk-oai\""), "{json}");
        assert!(json.contains("\"capabilities\""), "{json}");
    }

    /// Capability env names must match the canonical CLI knobs, and the
    /// fail-closed contract holds: only true emits "1", false emits nothing —
    /// except the default-on knobs (memory-core trio + local read tools, CLI
    /// default ON), which always inject an explicit "1"/"0". The other explicit
    /// exceptions: `shell_sandbox = false` emits `APEIRETH_SHELL_SANDBOX=0`
    /// (backend default is ON), and numeric knobs emit their value only when > 0.
    #[test]
    fn capability_env_pairs_match_canonical_knobs_and_fail_closed() {
        let caps = BackendCapabilityEnv {
            enable_shell: true,
            enable_fetch: false,
            enable_organs: true,
            enable_local_read_tools: false,
            enable_preference_learning: false,
            enable_proactive_recall: false,
            enable_memory_injection: false,
            cognitive_judge: true,
            cognitive_council: false,
            ..Default::default()
        };
        let pairs = caps.env_pairs();
        let mut map = std::collections::HashMap::new();
        for (key, value) in pairs {
            map.insert(key, value);
        }
        assert_eq!(map["APEIRETH_ENABLE_SHELL"], "1");
        assert_eq!(map["APEIRETH_ENABLE_ORGANS"], "1");
        assert_eq!(map["APEIRETH_COGNITIVE_JUDGE"], "1");
        // 沙箱默认开：shell_sandbox_off=false 不注入；只有显式关闭才注入 "0"。
        assert!(!map.contains_key("APEIRETH_SHELL_SANDBOX"));
        // 记忆核心族默认开旋钮：显式关也注入 "0"（否则被 CLI 默认开覆盖）。
        assert_eq!(map["APEIRETH_ENABLE_PREFERENCE_LEARNING"], "0");
        assert_eq!(map["APEIRETH_ENABLE_PROACTIVE_RECALL"], "0");
        assert_eq!(map["APEIRETH_ENABLE_MEMORY_INJECTION"], "0");
        // 本地只读三件套默认开：显式关走 DISABLE 逃生门（CLI 侧 DISABLE 胜出）。
        assert_eq!(map["APEIRETH_DISABLE_LOCAL_READ_TOOLS"], "1");
        assert_eq!(
            map.len(),
            7,
            "false toggles emit nothing except default-on knobs: {map:?}"
        );
        // 全默认态: 默认开旋钮显式 "1", 其余一律不注入 (无任何能力被开启)。
        let default_map: std::collections::HashMap<_, _> = BackendCapabilityEnv::default()
            .env_pairs()
            .into_iter()
            .collect();
        assert_eq!(default_map.len(), 4, "{default_map:?}");
        assert_eq!(default_map["APEIRETH_ENABLE_PREFERENCE_LEARNING"], "1");
        assert_eq!(default_map["APEIRETH_ENABLE_PROACTIVE_RECALL"], "1");
        assert_eq!(default_map["APEIRETH_ENABLE_MEMORY_INJECTION"], "1");
        assert_eq!(default_map["APEIRETH_ENABLE_LOCAL_READ_TOOLS"], "1");
    }

    /// W2/W3 收官批新旋钮: 每个新 env 名与 canonical CLI 对齐, 数值旋钮
    /// 只在 > 0 时注入, 沙箱关闭显式注入 "0"。
    #[test]
    fn capability_env_pairs_cover_w2w3_knobs() {
        let caps = BackendCapabilityEnv {
            shell_sandbox_off: true,
            enable_local_read_tools: true,
            disable_typed_recall: true,
            enable_proactive_recall: true,
            enable_memory_injection: true,
            enable_consolidation: true,
            enable_reflexion: true,
            enable_partner_bond: true,
            enable_morphology_recall: true,
            enable_education: true,
            enable_absorption_insight: true,
            enable_community_triage: true,
            enable_onering_ledger: true,
            cognitive_council: true,
            council_advisors: 5,
            council_timeout_ms: 45000,
            morphology_temperature: 1.5,
            enable_onion_layer: true,
            enable_worktree_sandbox: true,
            ..Default::default()
        };
        let pairs = caps.env_pairs();
        let mut map = std::collections::HashMap::new();
        for (key, value) in pairs {
            map.insert(key, value);
        }
        assert_eq!(map["APEIRETH_SHELL_SANDBOX"], "0");
        assert_eq!(map["APEIRETH_ENABLE_LOCAL_READ_TOOLS"], "1");
        assert_eq!(map["APEIRETH_DISABLE_TYPED_RECALL"], "1");
        assert_eq!(map["APEIRETH_ENABLE_PROACTIVE_RECALL"], "1");
        assert_eq!(map["APEIRETH_ENABLE_MEMORY_INJECTION"], "1");
        assert_eq!(map["APEIRETH_ENABLE_CONSOLIDATION"], "1");
        assert_eq!(map["APEIRETH_ENABLE_REFLEXION"], "1");
        assert_eq!(map["APEIRETH_ENABLE_PARTNER_BOND"], "1");
        assert_eq!(map["APEIRETH_ENABLE_MORPHOLOGY_RECALL"], "1");
        assert_eq!(map["APEIRETH_ENABLE_EDUCATION"], "1");
        assert_eq!(map["APEIRETH_ENABLE_ABSORPTION_INSIGHT"], "1");
        assert_eq!(map["APEIRETH_ENABLE_COMMUNITY_TRIAGE"], "1");
        assert_eq!(map["APEIRETH_ENABLE_ONERING_LEDGER"], "1");
        assert_eq!(map["APEIRETH_COGNITIVE_COUNCIL"], "1");
        assert_eq!(map["APEIRETH_COUNCIL_ADVISORS"], "5");
        assert_eq!(map["APEIRETH_COUNCIL_TIMEOUT_MS"], "45000");
        assert_eq!(map["APEIRETH_MORPHOLOGY_TEMPERATURE"], "1.5");
        assert_eq!(map["APEIRETH_ENABLE_ONION_LAYER"], "1");
        assert_eq!(map["APEIRETH_ENABLE_WORKTREE_SANDBOX"], "1");
        // 19 个显式开启项 + 记忆核心族缺省开的 preference_learning 显式 "1"。
        assert_eq!(
            map["APEIRETH_ENABLE_PREFERENCE_LEARNING"], "1",
            "default-on trio member must still inject its explicit value"
        );
        assert_eq!(map.len(), 20, "exact knob coverage: {map:?}");

        // 数值为 0 = 不注入（后端用自带默认）；温度整数渲染不带小数点。
        // 默认开旋钮（记忆核心族 + 本地只读三件套）缺省开，仍显式注入 "1"。
        let minimal = BackendCapabilityEnv {
            council_advisors: 3,
            morphology_temperature: 2.0,
            ..Default::default()
        };
        let minimal: std::collections::HashMap<_, _> = minimal.env_pairs().into_iter().collect();
        assert_eq!(minimal["APEIRETH_COUNCIL_ADVISORS"], "3");
        assert_eq!(minimal["APEIRETH_MORPHOLOGY_TEMPERATURE"], "2");
        assert_eq!(minimal["APEIRETH_ENABLE_LOCAL_READ_TOOLS"], "1");
        assert_eq!(minimal.len(), 6, "{minimal:?}");

        // [Beta] 思考模式：enabled 才注入，filters/tag 空串不注入；
        // 关闭时不注入任何 reasoning 变量。
        let reasoning = BackendCapabilityEnv {
            reasoning_enabled: true,
            reasoning_model_filters: "deepseek, o3".into(),
            reasoning_tag: String::new(),
            ..Default::default()
        };
        let map: std::collections::HashMap<_, _> = reasoning.env_pairs().into_iter().collect();
        assert_eq!(map["APEIRETH_REASONING_ENABLED"], "1");
        assert_eq!(map["APEIRETH_REASONING_MODEL_FILTERS"], "deepseek, o3");
        assert!(!map.contains_key("APEIRETH_REASONING_TAG"), "空 tag 不注入");
        let reasoning_off = BackendCapabilityEnv {
            reasoning_enabled: false,
            reasoning_model_filters: "deepseek".into(),
            ..Default::default()
        };
        let pairs = reasoning_off.env_pairs();
        assert!(
            pairs
                .iter()
                .all(|(k, _)| !k.starts_with("APEIRETH_REASONING")),
            "关闭时 filters 也不注入: {pairs:?}"
        );
    }

    /// 「性格养成」第一铲: 四个体验旋钮 (APEIRETH_TUNE_*) + 自学习开关
    /// (APEIRETH_ENABLE_SELF_TUNING) 的注入名与后端旋钮严格一致;
    /// 缺省 (None/false) = 一个都不注入 = 后端基线现行为 (轻默认)。
    #[test]
    fn capability_env_pairs_inject_experience_tuning_knobs() {
        let caps = BackendCapabilityEnv {
            tune_memory_fade: Some(0.5),
            tune_curiosity_strength: Some(1.5),
            tune_tone_saturation: Some(0.8),
            tune_consolidation_cadence: Some(3.0),
            enable_self_tuning: true,
            ..Default::default()
        };
        let map: std::collections::HashMap<_, _> = caps.env_pairs().into_iter().collect();
        assert_eq!(map["APEIRETH_TUNE_MEMORY_FADE"], "0.5");
        assert_eq!(map["APEIRETH_TUNE_CURIOSITY_STRENGTH"], "1.5");
        assert_eq!(map["APEIRETH_TUNE_TONE_SATURATION"], "0.8");
        assert_eq!(
            map["APEIRETH_TUNE_CONSOLIDATION_CADENCE"], "3",
            "整数渲染不带小数点"
        );
        assert_eq!(map["APEIRETH_ENABLE_SELF_TUNING"], "1");

        let pairs = BackendCapabilityEnv::default().env_pairs();
        assert!(
            pairs
                .iter()
                .all(|(key, _)| !key.starts_with("APEIRETH_TUNE_")
                    && *key != "APEIRETH_ENABLE_SELF_TUNING"),
            "缺省不得注入体验旋钮: {pairs:?}"
        );
    }

    /// 预算旋钮 (APEIRETH_MAX_TURN_ROUNDS / APEIRETH_MAX_TOOL_CALLS /
    /// APEIRETH_CONTEXT_BUDGET_CHARS): Some 同源透传成 env 对, `None` 一个都不
    /// 注入 (后端默认 8 / 16 / 24000 现行为)。取值裁决 (越界钳制 1..=64、非法
    /// 回默认) 在后端解析, 已由 CLI 侧测试锁定, 这里只锁注入面。
    #[test]
    fn capability_env_pairs_inject_budget_knobs() {
        let caps = BackendCapabilityEnv {
            turn_round_limit: Some(4),
            tool_call_limit: Some(8),
            context_budget_chars: Some(12_000),
            ..Default::default()
        };
        let map: std::collections::HashMap<_, _> = caps.env_pairs().into_iter().collect();
        assert_eq!(map["APEIRETH_MAX_TURN_ROUNDS"], "4");
        assert_eq!(map["APEIRETH_MAX_TOOL_CALLS"], "8");
        assert_eq!(map["APEIRETH_CONTEXT_BUDGET_CHARS"], "12000");

        // 缺省 (None) = 一个预算变量都不注入 = 后端默认现行为。
        for (key, _) in BackendCapabilityEnv::default().env_pairs() {
            assert!(
                key != "APEIRETH_MAX_TURN_ROUNDS"
                    && key != "APEIRETH_MAX_TOOL_CALLS"
                    && key != "APEIRETH_CONTEXT_BUDGET_CHARS",
                "缺省不得注入预算变量: {key}"
            );
        }

        // 部分配置: 只注入配了的那枚, 其余照旧缺席。
        let partial = BackendCapabilityEnv {
            turn_round_limit: Some(2),
            ..Default::default()
        };
        let map: std::collections::HashMap<_, _> = partial.env_pairs().into_iter().collect();
        assert_eq!(map["APEIRETH_MAX_TURN_ROUNDS"], "2");
        assert!(!map.contains_key("APEIRETH_MAX_TOOL_CALLS"), "{map:?}");
        assert!(
            !map.contains_key("APEIRETH_CONTEXT_BUDGET_CHARS"),
            "{map:?}"
        );
    }

    /// 受控文件写入两件 (APEIRETH_ENABLE_FILE_WRITE / APEIRETH_ENABLE_FILE_WRITE_AUTO_PASS):
    /// 默认关 fail-closed —— false 一个变量都不注入 (不注入 "0"), true 才注入 "1"。
    /// 子开关「自动放行已读文件修改」依赖主开关的语义在前端映射层收口
    /// (capabilityEnvFromConfig: 主开关关时 enable_file_write_auto_pass 恒 false)。
    #[test]
    fn capability_env_pairs_gate_file_write_tool_fail_closed() {
        // 默认态: 两个文件写入变量都不注入。
        for (key, _) in BackendCapabilityEnv::default().env_pairs() {
            assert!(
                !key.starts_with("APEIRETH_ENABLE_FILE_WRITE"),
                "默认态不得注入文件写入变量: {key}"
            );
        }

        // 仅主开关: 只注入主变量, 子开关变量缺席。
        let main_only = BackendCapabilityEnv {
            enable_file_write: true,
            ..Default::default()
        };
        let map: std::collections::HashMap<_, _> = main_only.env_pairs().into_iter().collect();
        assert_eq!(map["APEIRETH_ENABLE_FILE_WRITE"], "1");
        assert!(
            !map.contains_key("APEIRETH_ENABLE_FILE_WRITE_AUTO_PASS"),
            "子开关未开不注入: {map:?}"
        );

        // 主开关 + 子开关: 两个都注入 "1"。
        let both = BackendCapabilityEnv {
            enable_file_write: true,
            enable_file_write_auto_pass: true,
            ..Default::default()
        };
        let map: std::collections::HashMap<_, _> = both.env_pairs().into_iter().collect();
        assert_eq!(map["APEIRETH_ENABLE_FILE_WRITE"], "1");
        assert_eq!(map["APEIRETH_ENABLE_FILE_WRITE_AUTO_PASS"], "1");
    }

    /// 学习日志只读解析 (逐记录类): 旧裸体行照读 + 坏行跳过并计数 + 缺文件 = 空。
    #[test]
    fn tuning_log_file_parses_jsonl_skips_bad_lines_and_missing_is_empty() {
        let dir =
            std::env::temp_dir().join(format!("apeireth-tuning-log-read-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(TUNING_LOG_FILE);
        fs::write(
            &path,
            concat!(
                "{\"seq\":1,\"param\":\"memory_fade\",\"previous\":1.0,\"next\":0.75,\"reason\":\"检索未命中偏多\",\"at_epoch_ms\":1000}\n",
                "not-json\n",
                "{\"seq\":2,\"param\":\"consolidation_cadence\",\"previous\":1.0,\"next\":2.0,\"reason\":\"拉长整合间隔\",\"at_epoch_ms\":2000}\n",
            ),
        )
        .unwrap();
        let read = read_tuning_log_file(&path).expect("解析成功");
        assert_eq!(read.entries.len(), 2, "好行照读");
        assert_eq!(read.skipped, 1, "坏行必须跳过并计数");
        assert_eq!(read.entries[0].seq, 1);
        assert_eq!(read.entries[0].param, "memory_fade");
        assert_eq!(read.entries[0].next, 0.75);
        assert_eq!(read.entries[1].param, "consolidation_cadence");
        assert_eq!(read.entries[1].at_epoch_ms, 2000);

        let missing = read_tuning_log_file(&dir.join("absent.jsonl")).expect("缺文件 = 空列表");
        assert!(missing.entries.is_empty());
        assert_eq!(missing.skipped, 0);
        let _ = fs::remove_dir_all(&dir);
    }

    /// 逐记录信封行照读, 与旧裸体行共存 (每行独立版本戳, 逐行裁决)。
    #[test]
    fn tuning_log_file_reads_envelope_rows_and_legacy_rows_side_by_side() {
        let dir =
            std::env::temp_dir().join(format!("apeireth-tuning-log-env-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(TUNING_LOG_FILE);
        let legacy_line =
            "{\"seq\":1,\"param\":\"memory_fade\",\"previous\":1.0,\"next\":0.75,\"reason\":\"旧裸体行\",\"at_epoch_ms\":1000}";
        let envelope_line = format!(
            r#"{{"name":"{TUNING_LOG_DOC_NAME}","version":{TUNING_LOG_DOC_VERSION},"compatible_versions":[{TUNING_LOG_DOC_VERSION}],"body":{{"seq":2,"param":"tone_saturation","previous":0.2,"next":0.21,"reason":"信封行","at_epoch_ms":2000}}}}"#
        );
        fs::write(&path, format!("{legacy_line}\n{envelope_line}\n")).unwrap();

        let read = read_tuning_log_file(&path).expect("解析成功");
        assert_eq!(read.entries.len(), 2, "信封行与旧裸体行共存照读");
        assert_eq!(read.skipped, 0, "好行不计跳过");
        // 好数据路径逐字节等价: 旧裸体行恢复出的条目序列化 == 原行字节。
        assert_eq!(
            serde_json::to_string(&read.entries[0]).unwrap(),
            legacy_line,
            "旧裸体好行必须逐字节等价恢复"
        );
        assert_eq!(read.entries[1].param, "tone_saturation");
        let _ = fs::remove_dir_all(&dir);
    }

    /// 旧版本持久化的 capability JSON（无 W2/W3 字段）必须仍能反序列化：
    /// serde(default) 把缺失字段补成 false/0 = 全部不注入（fail-closed），
    /// 记忆核心族三件例外：缺失 = 开 = 显式注入 "1"（「未设 = 开」与 CLI 对齐），
    /// 显式 false = 显式注入 "0"。
    #[test]
    fn capability_env_legacy_persisted_json_still_loads() {
        let legacy = r#"{
            "enable_shell": true,
            "enable_fetch": false,
            "enable_organs": true,
            "enable_preference_learning": false,
            "cognitive_judge": true,
            "cognitive_council": false
        }"#;
        let caps: BackendCapabilityEnv = serde_json::from_str(legacy).expect("legacy JSON loads");
        assert!(caps.enable_shell);
        assert!(caps.enable_organs);
        assert!(caps.cognitive_judge);
        assert!(
            !caps.shell_sandbox_off,
            "missing shell_sandbox_off defaults to false = sandbox stays ON"
        );
        assert_eq!(caps.council_advisors, 0, "missing numerics default to 0");
        let map: std::collections::HashMap<_, _> = caps.env_pairs().into_iter().collect();
        // 3 个 "1" (shell/organs/judge) + 显式 false 的 preference_learning "0"
        // + 缺失字段按默认开补 "1" 的 proactive/memory_injection/local_read_tools。
        assert_eq!(map["APEIRETH_ENABLE_PREFERENCE_LEARNING"], "0");
        assert_eq!(map["APEIRETH_ENABLE_PROACTIVE_RECALL"], "1");
        assert_eq!(map["APEIRETH_ENABLE_MEMORY_INJECTION"], "1");
        assert_eq!(map["APEIRETH_ENABLE_LOCAL_READ_TOOLS"], "1");
        assert_eq!(map.len(), 7, "{map:?}");
    }

    #[test]
    fn executable_name_is_platform_correct() {
        let name = BackendSupervisor::backend_executable_name();
        if cfg!(windows) {
            assert_eq!(name, "apeireth.exe");
        } else {
            assert_eq!(name, "apeireth");
        }
    }

    #[test]
    fn fresh_supervisor_is_stopped_and_unowned() {
        let supervisor = BackendSupervisor::build(None).unwrap();
        let info = tokio_block(supervisor.info());
        assert_eq!(info.state, BackendState::Stopped);
        assert_eq!(info.ownership, BackendOwnership::External);
        assert!(info.pid.is_none());
        assert!(info.endpoint.is_none());
        assert_eq!(info.restart_count, 0);
    }

    // -----------------------------------------------------------------------
    // E-07 首启 spawn 判定: 配置面先解析、起停事件序列有序。
    // -----------------------------------------------------------------------

    #[test]
    fn a_fresh_supervisor_has_an_empty_lifecycle_face() {
        let supervisor = BackendSupervisor::build(None).unwrap();
        assert!(
            supervisor.lifecycle_events().is_empty(),
            "no transitions have happened yet"
        );
    }

    #[test]
    fn apply_backend_config_resolves_the_launch_config_question() {
        let supervisor = BackendSupervisor::build(None).unwrap();
        let mut resolution = supervisor.config_resolution.subscribe();
        assert!(
            !*resolution.borrow(),
            "a fresh supervisor has not seen its config face yet"
        );

        // An unchanged config is a no-op apply (nothing to start), but it still
        // answers the launch-time question the held first spawn waits on.
        tokio_block(supervisor.apply_provider_env(BackendProviderEnv::default()))
            .expect("no-op apply");
        assert!(
            *resolution.borrow(),
            "the first apply must resolve the launch config"
        );
        assert!(
            tokio_block(resolution.wait_for(|resolved| *resolved)).is_ok(),
            "a waiter observes the resolution"
        );
        assert_eq!(
            tokio_block(supervisor.info()).state,
            BackendState::Stopped,
            "a no-op apply must not spawn anything"
        );
        assert!(
            supervisor.lifecycle_events().is_empty(),
            "a no-op apply drives no lifecycle transition"
        );
    }

    /// An external backend must never be killed by the desktop: a fresh
    /// supervisor owns nothing, so stop() has to refuse.
    #[test]
    fn stop_refuses_to_touch_external_backend() {
        let supervisor = BackendSupervisor::build(None).unwrap();
        let result = tokio_block(supervisor.stop());
        assert!(result.is_err(), "expected refusal, got {result:?}");
        assert!(
            result.unwrap_err().contains("external"),
            "refusal should name external ownership"
        );
    }

    #[test]
    fn ephemeral_port_selection_yields_a_bindable_port() {
        let supervisor = BackendSupervisor::build(None).unwrap();
        let port = tokio_block(supervisor.select_free_port()).expect("port");
        assert_ne!(port, 0);
        assert_ne!(port, 8090, "must never select the legacy companion port");
        // Released back to the OS, so it is bindable again.
        std::net::TcpListener::bind(("127.0.0.1", port)).expect("port should be free");
    }

    #[test]
    fn readiness_probe_times_out_on_a_dead_port() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let dead_port = listener.local_addr().expect("addr").port();
        drop(listener); // Nothing is listening now.

        let supervisor = BackendSupervisor::build(None).unwrap();
        let result = tokio_block(async {
            tokio::time::timeout(
                Duration::from_secs(20),
                supervisor.wait_for_ready(dead_port),
            )
            .await
            .expect("probe should return, not hang")
        });
        assert!(result.is_err(), "probe must fail when nothing is listening");
    }

    /// A readiness timeout has to land in Failed with the reason recorded —
    /// leaving the machine in Starting would show a permanent false "starting".
    #[test]
    fn failed_readiness_records_failed_state() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let dead_port = listener.local_addr().expect("addr").port();
        drop(listener);

        let supervisor = BackendSupervisor::build(None).unwrap();
        tokio_block(async {
            {
                let mut info = supervisor.info.write().await;
                info.state = BackendState::Starting;
            }
            if let Err(error) = supervisor.wait_for_ready(dead_port).await {
                let mut info = supervisor.info.write().await;
                info.state = BackendState::Failed;
                info.last_error = Some(error);
            }
        });

        let info = tokio_block(supervisor.info());
        assert_eq!(info.state, BackendState::Failed);
        assert!(info.last_error.is_some(), "failure reason must be recorded");
    }

    #[test]
    fn readiness_fails_fast_when_owned_child_already_exited() {
        let supervisor = BackendSupervisor::build(None).unwrap();
        let elapsed = tokio_block(async {
            let mut child = {
                #[cfg(windows)]
                {
                    Command::new("cmd")
                        .args(["/C", "exit", "42"])
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .stdin(std::process::Stdio::null())
                        .spawn()
                        .expect("spawn dummy child")
                }
                #[cfg(not(windows))]
                {
                    Command::new("sh")
                        .args(["-c", "exit 42"])
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .stdin(std::process::Stdio::null())
                        .spawn()
                        .expect("spawn dummy child")
                }
            };
            let pid = child.id().expect("pid");
            let _ = child.wait().await;
            {
                let mut info = supervisor.info.write().await;
                info.state = BackendState::Starting;
                info.ownership = BackendOwnership::OwnedByDesktop;
            }
            {
                let mut process = supervisor.process.write().await;
                *process = Some(BackendProcess { child, pid });
            }
            let started = Instant::now();
            let result = supervisor.wait_for_ready(1).await;
            assert!(result.is_err(), "expected failure, got {result:?}");
            let message = result.unwrap_err();
            assert!(
                message.contains("exited during startup"),
                "must report child death, not a 15s timeout: {message}"
            );
            started.elapsed()
        });
        assert!(
            elapsed < Duration::from_secs(3),
            "child death must fail fast, took {elapsed:?}"
        );
    }

    #[test]
    fn backend_info_serializes_without_secrets() {
        let supervisor = BackendSupervisor::build(None).unwrap();
        let info = tokio_block(supervisor.info());
        let json = serde_json::to_string(&info).expect("serialize");

        for forbidden in [
            "apiKey",
            "api_key",
            "Authorization",
            "Bearer",
            "sk-",
            "master_token",
        ] {
            assert!(
                !json.contains(forbidden),
                "diagnostic payload must not carry {forbidden:?}: {json}"
            );
        }
        // The fields the diagnostics surface needs are present.
        for field in ["state", "ownership", "pid", "endpoint", "restart_count"] {
            assert!(json.contains(field), "missing diagnostic field {field:?}");
        }
    }

    #[test]
    fn dev_resolution_never_returns_a_legacy_path() {
        // Whether or not a dev build exists here, it must never resolve to the
        // historical companion backend.
        if let Some(path) = BackendSupervisor::resolve_dev_backend() {
            let text = path.to_string_lossy().to_ascii_lowercase();
            assert!(!text.contains("companion_serve"));
            assert!(!text.contains("examples"));
            assert!(
                text.ends_with(BackendSupervisor::backend_executable_name()),
                "resolved path should be the canonical CLI: {text}"
            );
        }
    }

    fn tokio_block<F: std::future::Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(future)
    }

    // -----------------------------------------------------------------------
    // stored_doc 消费方接线: 配置拒开 + 好数据零回归 + 旧文件兼容 + 显式迁移
    // -----------------------------------------------------------------------

    fn config_scratch(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("apeireth-env-config-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// 拒开生效 (坏配置不再回退默认): 坏 provider 配置必须报错拒开,
    /// 启动构造同样失败, 不带着默认配置继续跑。
    #[test]
    fn provider_env_config_rejects_corrupt_file_without_default_fallback() {
        let dir = config_scratch("reject");
        let logger = DesktopLogger::new_in_dir(dir.join("logs")).expect("logger");
        fs::write(dir.join("backend-provider-env.json"), b"{ this is not json").unwrap();

        assert!(
            BackendSupervisor::load_persisted_provider_env(&logger).is_err(),
            "坏配置必须拒开, 不得回退默认值"
        );
        assert!(
            BackendSupervisor::with_logger_in_dir(dir.clone()).is_err(),
            "启动构造必须拒开坏配置"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// 好数据路径零回归: 恢复出的配置与旧裸体写端输出逐字节等价。
    #[test]
    fn provider_env_good_data_payload_stays_byte_equivalent() {
        let dir = config_scratch("equiv");
        let logger = Arc::new(DesktopLogger::new_in_dir(dir.join("logs")).expect("logger"));
        let supervisor = BackendSupervisor::with_logger(Arc::clone(&logger)).expect("构造成功");
        let env = BackendProviderEnv {
            openai_url: Some("https://api.example.test/v1".into()),
            openai_models: Some("model-a,model-b".into()),
            ..Default::default()
        };
        supervisor.persist_provider_env(&env);

        let raw = fs::read(dir.join("backend-provider-env.json")).unwrap();
        let doc: stored_doc::StoredDoc<BackendProviderEnv> =
            serde_json::from_slice(&raw).expect("落盘是存储文档信封");
        assert_eq!(doc.name, PROVIDER_ENV_DOC_NAME);
        assert_eq!(
            serde_json::to_string_pretty(&doc.body).unwrap(),
            serde_json::to_string_pretty(&env.without_secrets()).unwrap(),
            "好数据 body 必须与旧裸体写端输出逐字节等价"
        );
        // 恢复行为零回归。
        let restored = BackendSupervisor::load_persisted_provider_env(&logger)
            .expect("好配置可读")
            .expect("配置存在");
        assert_eq!(restored, env.without_secrets());
        let _ = fs::remove_dir_all(&dir);
    }

    /// 旧文件兼容: 前信封裸体配置迁移前后都可读; 默认打开不静默迁移; 迁移留审计。
    #[test]
    fn provider_env_legacy_file_is_readable_before_and_after_explicit_migration() {
        let dir = config_scratch("legacy");
        let logger = Arc::new(DesktopLogger::new_in_dir(dir.join("logs")).expect("logger"));
        let env = BackendProviderEnv {
            anthropic_url: Some("https://api.example.test/v2".into()),
            anthropic_models: Some("model-c".into()),
            ..Default::default()
        };
        let legacy_bytes = serde_json::to_vec_pretty(&env.without_secrets()).unwrap();
        let path = dir.join("backend-provider-env.json");
        fs::write(&path, &legacy_bytes).unwrap();

        // 迁移前可读 (旧裸体只读兼容), 文件字节不被改写。
        let before = BackendSupervisor::load_persisted_provider_env(&logger)
            .expect("旧裸体配置必须可读")
            .expect("配置存在");
        assert_eq!(before, env);
        assert_eq!(
            fs::read(&path).unwrap(),
            legacy_bytes,
            "默认打开路径不得静默迁移"
        );

        // 显式迁移留审计。
        BackendSupervisor::migrate_legacy_provider_env_config(&logger).expect("显式迁移必须成功");
        let audit = fs::read_to_string(dir.join("backend-provider-env.json.migrate-audit.jsonl"))
            .expect("显式迁移必须留审计");
        assert_eq!(audit.lines().count(), 1, "审计文件应恰一行: {audit}");

        // 迁移后仍可读, 内容不变。
        let after = BackendSupervisor::load_persisted_provider_env(&logger)
            .expect("迁移后必须可读")
            .expect("配置存在");
        assert_eq!(after, before);
        let _ = fs::remove_dir_all(&dir);
    }

    /// 能力开关配置走同一套语义: 旧裸体兼容 + 显式迁移留审计。
    #[test]
    fn capability_env_legacy_file_is_readable_before_and_after_explicit_migration() {
        let dir = config_scratch("caps-legacy");
        let logger = Arc::new(DesktopLogger::new_in_dir(dir.join("logs")).expect("logger"));
        let legacy_bytes = serde_json::to_vec_pretty(&BackendCapabilityEnv::default()).unwrap();
        let path = dir.join("backend-capability-env.json");
        fs::write(&path, &legacy_bytes).unwrap();

        let before = BackendSupervisor::load_persisted_capability_env(&logger)
            .expect("旧裸体配置必须可读")
            .expect("配置存在");
        assert_eq!(
            fs::read(&path).unwrap(),
            legacy_bytes,
            "默认打开路径不得静默迁移"
        );

        BackendSupervisor::migrate_legacy_capability_env_config(&logger).expect("显式迁移必须成功");
        let audit = fs::read_to_string(dir.join("backend-capability-env.json.migrate-audit.jsonl"))
            .expect("显式迁移必须留审计");
        assert_eq!(audit.lines().count(), 1, "审计文件应恰一行: {audit}");
        let after = BackendSupervisor::load_persisted_capability_env(&logger)
            .expect("迁移后必须可读")
            .expect("配置存在");
        assert_eq!(
            serde_json::to_value(&after).unwrap(),
            serde_json::to_value(&before).unwrap()
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // -----------------------------------------------------------------------
    // W2-① 常驻协同: 探测面 (直连/拉起) + 服务令牌面 (G10 同源)
    // -----------------------------------------------------------------------

    /// 一台应答 `/health` 的最小 HTTP 服务 (假常驻), 返回 JSON `{"status":"ok"}`。
    async fn spawn_fake_resident() -> (u16, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("fake resident bind");
        let port = listener.local_addr().expect("fake resident addr").port();
        let handle = tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut buffer = [0u8; 2048];
                let _ = stream.read(&mut buffer).await;
                let _ = stream
                    .write_all(
                        b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 16\r\n\r\n{\"status\":\"ok\"}",
                    )
                    .await;
            }
        });
        (port, handle)
    }

    /// 常驻占位解析: 契约往返 + 坏内容读作"无常驻" (不猜、不砖化)。
    #[test]
    fn resident_descriptor_round_trips_and_rejects_unparseable_content() {
        let descriptor = ResidentDescriptor {
            pid: 4242,
            port: 8080,
            bind: "127.0.0.1".to_string(),
            started_at_ms: 1_760_000_000_000,
        };
        assert_eq!(
            ResidentDescriptor::parse(&descriptor.render()).expect("round trip"),
            descriptor
        );
        assert_eq!(descriptor.endpoint(), "http://127.0.0.1:8080");

        // 坏内容 / 缺字段 / 空文件: 一律 None (探测面读不懂 = 无常驻)。
        assert_eq!(ResidentDescriptor::parse("nonsense"), None);
        assert_eq!(ResidentDescriptor::parse("pid=1\nport=x\n"), None);
        assert_eq!(ResidentDescriptor::parse("pid=1\n"), None);
        assert_eq!(ResidentDescriptor::parse(""), None);
    }

    /// 常驻运行目录: `APEIRETH_RUN_DIR` 显式绝对值最高优先 (工作目录无关)。
    #[test]
    fn resident_run_dir_prefers_the_explicit_env_anchor() {
        let _lock = env_lock();
        let _guard = EnvGuard::set(&[("APEIRETH_RUN_DIR", "X:\\somewhere\\run")]);
        assert_eq!(
            resident_run_dir(),
            std::path::PathBuf::from("X:\\somewhere\\run"),
            "显式 APEIRETH_RUN_DIR 必须胜出"
        );
        assert_eq!(
            resident_descriptor_path(),
            std::path::PathBuf::from("X:\\somewhere\\run").join(RESIDENT_DESCRIPTOR_FILE)
        );
    }

    /// 探测面: 占位陈旧 (端口不应答) = 无常驻; `/health` 应答 = 命中常驻
    /// (端口/健康检查双口径)。
    #[test]
    fn resident_probe_detects_a_live_service_and_ignores_a_stale_slot() {
        tokio_block(async {
            let dir = config_scratch("resident-probe");
            let slot = dir.join(RESIDENT_DESCRIPTOR_FILE);

            // 陈旧占位: 记录端口无人应答。
            let closed = {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                let port = listener.local_addr().unwrap().port();
                drop(listener);
                port
            };
            let stale = ResidentDescriptor {
                pid: 4242,
                port: closed,
                bind: "127.0.0.1".to_string(),
                started_at_ms: 1,
            };
            fs::write(&slot, stale.render()).unwrap();
            assert!(
                detect_resident_gateway(&slot).await.is_none(),
                "陈旧占位必须读作无常驻"
            );

            // 活常驻: /health 应答 = 探测命中。
            let (port, server) = spawn_fake_resident().await;
            let live = ResidentDescriptor {
                pid: 4242,
                port,
                bind: "127.0.0.1".to_string(),
                started_at_ms: 2,
            };
            fs::write(&slot, live.render()).unwrap();
            let detected = detect_resident_gateway(&slot)
                .await
                .expect("常驻必须被探测到");
            assert_eq!(detected, live);
            server.abort();

            let _ = fs::remove_dir_all(&dir);
        });
    }

    /// D12 运维探针读面: 只读报告三态如实 —— 无占位 `absent` / 占位陈旧
    /// `stale` (占位照报) / `/health` 应答 `live`; 全程不写状态。
    #[test]
    fn resident_probe_read_face_reports_absent_stale_and_live() {
        tokio_block(async {
            let dir = config_scratch("resident-probe-face");
            let slot = dir.join(RESIDENT_DESCRIPTOR_FILE);

            // 无占位: absent + 端点/进程号 None (不猜)。
            let absent = probe_resident_service(&slot).await;
            assert_eq!(
                absent,
                ResidentProbeReport {
                    descriptor_present: false,
                    endpoint: None,
                    pid: None,
                    health: ResidentProbeHealth::Absent,
                },
                "无占位必须如实报 absent"
            );

            // 陈旧占位: 占位在手但端口不应答 = stale (占位照报, 不猜在跑)。
            let closed = {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                let port = listener.local_addr().unwrap().port();
                drop(listener);
                port
            };
            let stale = ResidentDescriptor {
                pid: 4242,
                port: closed,
                bind: "127.0.0.1".to_string(),
                started_at_ms: 1,
            };
            fs::write(&slot, stale.render()).unwrap();
            let stale_report = probe_resident_service(&slot).await;
            assert!(stale_report.descriptor_present);
            assert_eq!(stale_report.health, ResidentProbeHealth::Stale);
            assert_eq!(
                stale_report.endpoint.as_deref(),
                Some(stale.endpoint().as_str())
            );
            assert_eq!(stale_report.pid, Some(4242));

            // 活常驻: /health 应答 = live (与直连判定同一探测缝)。
            let (port, server) = spawn_fake_resident().await;
            let live = ResidentDescriptor {
                pid: 4242,
                port,
                bind: "127.0.0.1".to_string(),
                started_at_ms: 2,
            };
            fs::write(&slot, live.render()).unwrap();
            let live_report = probe_resident_service(&slot).await;
            assert!(live_report.descriptor_present);
            assert_eq!(live_report.health, ResidentProbeHealth::Live);
            assert_eq!(live_report.pid, Some(4242));
            server.abort();

            let _ = fs::remove_dir_all(&dir);
        });
    }

    /// 桌面协同主语义: 已有常驻 = 直连 (External 归属, 不重复拉起侧车,
    /// 不归桌面停机); 无常驻 = 按现行为拉起侧车 (既有生命周期测试钉住)。
    #[test]
    fn start_adopts_an_existing_resident_service_without_spawning_a_sidecar() {
        let _lock = env_lock();
        tokio_block(async {
            let dir = config_scratch("resident-adopt");
            let run_dir = dir.join("run");
            fs::create_dir_all(&run_dir).unwrap();
            let _env = EnvGuard::set(&[("APEIRETH_RUN_DIR", run_dir.to_str().unwrap())]);

            let (port, server) = spawn_fake_resident().await;
            let descriptor = ResidentDescriptor {
                pid: 4242,
                port,
                bind: "127.0.0.1".to_string(),
                started_at_ms: 3,
            };
            fs::write(run_dir.join(RESIDENT_DESCRIPTOR_FILE), descriptor.render()).unwrap();

            let supervisor = BackendSupervisor::with_resident_probe_in_dir(dir.clone())
                .expect("production-shaped supervisor with resident probe");
            let message = supervisor
                .start()
                .await
                .expect("existing resident service must be adopted");
            assert!(message.contains("resident"), "{message}");
            assert!(message.contains("external"), "{message}");

            let info = supervisor.info().await;
            assert_eq!(info.state, BackendState::Ready);
            assert_eq!(info.ownership, BackendOwnership::External);
            assert_eq!(info.pid, Some(4242), "直连面透出常驻进程 id");
            assert_eq!(info.port, Some(port));
            assert_eq!(
                info.endpoint.as_deref(),
                Some(format!("http://127.0.0.1:{port}")).as_deref()
            );
            // 不拉起侧车: 进程面无子进程句柄 (真断言, 非状态猜测)。
            assert!(
                supervisor.process.read().await.is_none(),
                "已有常驻不得再拉起侧车"
            );
            // 既有语义照旧: 外部后端不归桌面停机。
            assert!(
                supervisor.stop().await.is_err(),
                "外部常驻的生命周期归服务管理面"
            );
            server.abort();

            let _ = fs::remove_dir_all(&dir);
        });
    }

    /// 令牌三源收口: spawn 期 > env > 服务令牌面 (探测到常驻 = 读同源令牌);
    /// 逃生阀哨兵 = 明示不设防 (无令牌可呈); 三者皆无 = None = 三态门照旧。
    #[test]
    fn access_token_resolution_prefers_spawned_then_env_then_service_face() {
        let resolve = resolve_gateway_access_token;
        assert_eq!(
            resolve(
                Some("spawned".into()),
                Some("env".into()),
                Some("face".into())
            )
            .as_deref(),
            Some("spawned")
        );
        assert_eq!(
            resolve(None, Some("env".into()), Some("face".into())).as_deref(),
            Some("env")
        );
        assert_eq!(
            resolve(None, None, Some("face".into())).as_deref(),
            Some("face"),
            "探测到常驻 = 读服务令牌面 (同源令牌)"
        );
        assert_eq!(
            resolve(None, None, None),
            None,
            "无令牌 = 三态门警告帧语义照旧"
        );

        // 空白值逐层跳过 (不发空 Bearer)。
        assert_eq!(
            resolve(Some("  ".into()), Some(" env ".into()), Some("face".into())).as_deref(),
            Some("env")
        );

        // 逃生阀哨兵: 明示不设防 → 无令牌可呈 (任一层出现即终止, 不再回落)。
        for (spawned, env_value, face) in [
            (
                Some(UNPROTECTED_TOKEN_SENTINEL.into()),
                Some("env".into()),
                Some("face".into()),
            ),
            (
                None,
                Some(UNPROTECTED_TOKEN_SENTINEL.into()),
                Some("face".into()),
            ),
            (None, None, Some(UNPROTECTED_TOKEN_SENTINEL.into())),
        ] {
            assert_eq!(resolve(spawned, env_value, face), None, "哨兵 = 不设防声明");
        }
    }

    /// 服务令牌面的凭据库编码契约: UTF-16LE (与服务安装面写入编码一致);
    /// 非契约编码读作未配置 (不猜、不拿乱码当令牌)。
    #[test]
    fn service_token_blob_encoding_matches_the_store_writer() {
        let mut utf16le = Vec::new();
        for unit in "loopback-token-0123456789abcdef".encode_utf16() {
            utf16le.extend_from_slice(&unit.to_le_bytes());
        }
        assert_eq!(
            decode_credential_blob(&utf16le).as_deref(),
            Some("loopback-token-0123456789abcdef"),
            "UTF-16LE 密文必须解出令牌"
        );

        assert_eq!(decode_credential_blob(&[]), None, "空密文 = 未配置");
        assert_eq!(
            decode_credential_blob(&[0x61, 0x62, 0x63]),
            None,
            "奇数长度 = 非 UTF-16LE 契约编码, 读作未配置"
        );
        let blank: Vec<u8> = "   ".encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        assert_eq!(decode_credential_blob(&blank), None, "空白密文 = 未配置");
    }

    /// 真机探针 (默认忽略): 本壳的凭据库读取面 (CredReadW) 与服务安装面写入
    /// 同源 —— 有服务令牌在库时必须读出同一枚 (设
    /// `SERVICE_TOKEN_PROBE_EXPECTED` 时强校验逐字节一致)。探针只读不写。
    ///
    ///   SERVICE_TOKEN_PROBE_EXPECTED=<token> cargo test -p companion-desktop \
    ///     --lib service_token_face_live_probe -- --ignored --nocapture
    #[test]
    #[ignore]
    fn service_token_face_live_probe() {
        match read_service_token_face() {
            Some(token) => {
                if let Ok(expected) = std::env::var("SERVICE_TOKEN_PROBE_EXPECTED") {
                    assert_eq!(token, expected, "同源令牌必须逐字节一致");
                }
                println!("SERVICE TOKEN FACE READ OK ({} bytes)", token.len());
            }
            None => println!("SERVICE TOKEN FACE EMPTY"),
        }
    }
}
