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
use crate::workspace;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::RwLock;

/// Backend supervisor state machine
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum BackendState {
    Stopped,
    Starting,
    Ready,
    Failed,
    Stopping,
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

/// 只读解析 tuning-log.jsonl: 文件缺失 = 空列表 (尚无自动调整);
/// 坏行跳过 (容忍部分损坏, 只读视图尽力而为)。
pub fn read_tuning_log_file(path: &Path) -> Result<Vec<TuningLogEntry>, String> {
    match std::fs::read_to_string(path) {
        Ok(raw) => {
            if raw.len() as u64 > TUNING_LOG_MAX_BYTES {
                return Err(format!(
                    "tuning log too large ({} bytes > {TUNING_LOG_MAX_BYTES}): {}",
                    raw.len(),
                    path.display()
                ));
            }
            Ok(raw
                .lines()
                .filter(|line| !line.trim().is_empty())
                .filter_map(|line| serde_json::from_str::<TuningLogEntry>(line).ok())
                .collect())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(format!(
            "tuning log read failed ({}): {error}",
            path.display()
        )),
    }
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
}

impl BackendSupervisor {
    /// Base constructor. Production always attaches a logger via
    /// [`Self::with_logger`]; tests use `build(None)` when no log file is wanted.
    fn build(logger: Option<Arc<DesktopLogger>>) -> Self {
        // Restore the persisted non-secret provider config (endpoints/models),
        // capability toggles, and workspace dir so the very first spawn already
        // reflects the user's last settings. Keys are deliberately NOT persisted
        // to the app-data config; they live in memory for the session and are
        // otherwise restored from the OS keychain at spawn (env > keychain).
        let persisted_provider = logger
            .as_ref()
            .and_then(|l| Self::load_persisted_provider_env(l))
            .unwrap_or_default()
            .without_secrets();
        let persisted_capabilities = logger
            .as_ref()
            .and_then(|l| Self::load_persisted_capability_env(l))
            .unwrap_or_default();
        let persisted_workspace = logger
            .as_ref()
            .map(|l| Self::logger_app_data_dir(l))
            .and_then(|dir| workspace::load_workspace_dir(&dir));
        Self {
            info: Arc::new(RwLock::new(BackendInfo::default())),
            process: Arc::new(RwLock::new(None)),
            logger,
            provider_env: RwLock::new(persisted_provider.clone()),
            capability_env: RwLock::new(persisted_capabilities.clone()),
            workspace_dir: RwLock::new(persisted_workspace),
            spawned_provider_env: RwLock::new(persisted_provider),
            spawned_capability_env: RwLock::new(persisted_capabilities),
        }
    }

    /// App-data directory that owns the logs (e.g. `%LOCALAPPDATA%\Apeireth`).
    /// `None` for logger-less test supervisors, where tests control the
    /// sidecar's store paths through their own environment.
    fn app_data_dir(&self) -> Option<PathBuf> {
        self.logger.as_ref().map(|l| Self::logger_app_data_dir(l))
    }

    /// The directory that owns the logs and every app-data config file.
    fn logger_app_data_dir(logger: &DesktopLogger) -> PathBuf {
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

    fn load_persisted_provider_env(logger: &DesktopLogger) -> Option<BackendProviderEnv> {
        let path = Self::provider_env_path(logger);
        let raw = std::fs::read_to_string(path).ok()?;
        serde_json::from_str::<BackendProviderEnv>(&raw)
            .ok()
            .map(BackendProviderEnv::sanitized)
            .map(|env| env.without_secrets())
    }

    fn load_persisted_capability_env(logger: &DesktopLogger) -> Option<BackendCapabilityEnv> {
        let path = Self::capability_env_path(logger);
        let raw = std::fs::read_to_string(path).ok()?;
        serde_json::from_str::<BackendCapabilityEnv>(&raw).ok()
    }

    /// Persist only the non-secret part of the provider environment so the
    /// next launch restores endpoints/models without a restart cycle.
    fn persist_provider_env(&self, env: &BackendProviderEnv) {
        let Some(logger) = &self.logger else {
            return;
        };
        let path = Self::provider_env_path(logger);
        let Ok(json) = serde_json::to_string_pretty(&env.without_secrets()) else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, json);
    }

    /// Persist capability toggles (no secrets) for the next launch.
    fn persist_capability_env(&self, env: &BackendCapabilityEnv) {
        let Some(logger) = &self.logger else {
            return;
        };
        let path = Self::capability_env_path(logger);
        let Ok(json) = serde_json::to_string_pretty(env) else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, json);
    }

    /// Attach a persistent logger so backend stdout/stderr reaches
    /// `apeireth-backend.log` and lifecycle events reach the desktop log.
    pub fn with_logger(logger: Arc<DesktopLogger>) -> Self {
        Self::build(Some(logger))
    }

    /// A logger-less supervisor for integration tests.
    ///
    /// Tests drive the real lifecycle but must not append to the user's actual
    /// log files, so no logger is attached.
    #[doc(hidden)]
    pub fn new_for_test() -> Self {
        Self::build(None)
    }

    /// Test-only production-shaped supervisor whose logs and app-data anchors
    /// live under `dir` (the parent of `dir\logs` becomes the app-data dir).
    /// Used by the regression test proving the sidecar's stores land in
    /// app-data regardless of the launch CWD.
    #[doc(hidden)]
    pub fn with_logger_in_dir(dir: PathBuf) -> Result<Self, String> {
        let logger = DesktopLogger::new_in_dir(dir.join("logs"))?;
        Ok(Self::with_logger(Arc::new(logger)))
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

    /// Start the owned backend process
    pub async fn start(&self) -> Result<String, String> {
        let mut info = self.info.write().await;

        if info.state == BackendState::Starting || info.state == BackendState::Ready {
            return Err("Backend already starting or running".to_string());
        }

        info.state = BackendState::Starting;
        info.last_error = None;
        drop(info);

        // Locate bundled backend executable
        let backend_path = self.resolve_backend_binary()?;

        // Select free port
        let port = self.select_free_port().await?;

        self.log_desktop(
            LogLevel::Info,
            &format!("backend.spawn path={backend_path} port={port}"),
        );

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
                self.log_desktop(LogLevel::Info, &format!("backend.spawned pid={pid} port={port}"));
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

                Ok(format!("Backend started at {} (PID: {})", endpoint, pid))
            }
            Err(e) => {
                let mut info = self.info.write().await;
                info.state = BackendState::Failed;
                info.last_error = Some(e.clone());
                drop(info);
                self.log_desktop(LogLevel::Error, &format!("backend.spawn_failed {e}"));
                Err(e)
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

                    Ok(format!("Backend stopped (PID: {:?}, exit: {:?})", pid, status.code()))
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
    pub async fn apply_provider_env(
        &self,
        env: BackendProviderEnv,
    ) -> Result<BackendInfo, String> {
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

        if !provider_changed && !capabilities_changed {
            return Ok(self.info().await);
        }

        let state = self.info.read().await.state.clone();
        match state {
            BackendState::Stopped => {
                self.start().await?;
            }
            BackendState::Ready => {
                self.apply_to_ready_gateway().await?;
            }
            BackendState::Failed => {
                self.start().await?;
            }
            BackendState::Starting | BackendState::Stopping => {
                self.wait_for_settle().await;
                let settled = self.info.read().await.state.clone();
                match settled {
                    BackendState::Ready => {
                        self.apply_to_ready_gateway().await?;
                    }
                    BackendState::Stopped | BackendState::Failed => {
                        self.start().await?;
                    }
                    _ => {}
                }
            }
        }
        Ok(self.info().await)
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
                    self.restart().await.map(|_| ())
                }
                HotApply::Failed(error) => {
                    self.log_desktop(
                        LogLevel::Warn,
                        &format!("backend.hot_apply failed ({error}); restarting"),
                    );
                    self.restart().await.map(|_| ())
                }
            }
        } else {
            self.log_desktop(
                LogLevel::Info,
                "backend.restart required (models/capabilities/other-family change is not hot-applicable)",
            );
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
        let provider_env =
            provider_env.with_keychain_fallback(|provider| keychain::get_provider_key(provider));
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

        match client.post(&url).json(&payload).send().await {
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
        let response = client
            .get(&url)
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
            Err(format!("gateway admin config unavailable (HTTP {status}): {body}"))
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
        if let Some(value) = std::env::var("APEIRETH_SESSION_DB").ok() {
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
    pub async fn read_tuning_log(&self) -> Result<Vec<TuningLogEntry>, String> {
        let path = self.tuning_log_path().await;
        read_tuning_log_file(&path)
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

        let port = listener.local_addr()
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
        let provider_env =
            provider_env.with_keychain_fallback(|provider| keychain::get_provider_key(provider));
        for (key, value) in provider_env.env_pairs() {
            cmd.env(key, value);
        }
        // Same for the advanced-capability toggles (fail-closed: only "1"
        // values are emitted).
        let capability_env = self.capability_env.read().await.clone();
        for (key, value) in capability_env.env_pairs() {
            cmd.env(key, value);
        }

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
            let anchor_session = session_env.as_deref().is_none_or(|v| !Path::new(v).is_absolute());
            let anchor_cognitive =
                cognitive_env.as_deref().is_none_or(|v| !Path::new(v).is_absolute());

            // Only create the store dir when we are actually going to anchor
            // paths into it: an explicit absolute env store dir is the user's
            // own location and must not cause app-data side effects.
            if anchor_session || anchor_cognitive {
                let _ = std::fs::create_dir_all(&store_dir);
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
                    return Err(info.last_error.clone().unwrap_or_else(|| {
                        "backend failed during startup".to_string()
                    }));
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
        assert_eq!(stripped.openai_url.as_deref(), Some("https://api.deepseek.com/v1"));
        assert_eq!(stripped.anthropic_url.as_deref(), Some("https://api.anthropic.com"));
    }

    /// The serialized shape is what the frontend sends over IPC.
    #[test]
    fn provider_env_serde_shape_is_snake_case() {
        let env = BackendProviderEnv {
            openai_models: Some("deepseek-v4-flash".into()),
            ..Default::default()
        };
        let json = serde_json::to_string(&env).unwrap();
        assert!(json.contains("\"openai_models\":\"deepseek-v4-flash\""), "{json}");
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
        assert_eq!(request.base_url.as_deref(), Some("https://api.deepseek.com/v1"));
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
        let default_map: std::collections::HashMap<_, _> =
            BackendCapabilityEnv::default().env_pairs().into_iter().collect();
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
            pairs.iter().all(|(k, _)| !k.starts_with("APEIRETH_REASONING")),
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

    /// 学习日志只读解析: JSONL 行回读 (snake_case 契约) + 坏行跳过 + 缺文件 = 空。
    #[test]
    fn tuning_log_file_parses_jsonl_skips_bad_lines_and_missing_is_empty() {
        let dir =
            std::env::temp_dir().join(format!("apeireth-tuning-log-read-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(TUNING_LOG_FILE);
        std::fs::write(
            &path,
            concat!(
                "{\"seq\":1,\"param\":\"memory_fade\",\"previous\":1.0,\"next\":0.75,\"reason\":\"检索未命中偏多\",\"at_epoch_ms\":1000}\n",
                "not-json\n",
                "{\"seq\":2,\"param\":\"consolidation_cadence\",\"previous\":1.0,\"next\":2.0,\"reason\":\"拉长整合间隔\",\"at_epoch_ms\":2000}\n",
            ),
        )
        .unwrap();
        let entries = read_tuning_log_file(&path).expect("解析成功");
        assert_eq!(entries.len(), 2, "坏行跳过");
        assert_eq!(entries[0].seq, 1);
        assert_eq!(entries[0].param, "memory_fade");
        assert_eq!(entries[0].next, 0.75);
        assert_eq!(entries[1].param, "consolidation_cadence");
        assert_eq!(entries[1].at_epoch_ms, 2000);

        let missing = read_tuning_log_file(&dir.join("absent.jsonl")).expect("缺文件 = 空列表");
        assert!(missing.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
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
        let supervisor = BackendSupervisor::build(None);
        let info = tokio_block(supervisor.info());
        assert_eq!(info.state, BackendState::Stopped);
        assert_eq!(info.ownership, BackendOwnership::External);
        assert!(info.pid.is_none());
        assert!(info.endpoint.is_none());
        assert_eq!(info.restart_count, 0);
    }

    /// An external backend must never be killed by the desktop: a fresh
    /// supervisor owns nothing, so stop() has to refuse.
    #[test]
    fn stop_refuses_to_touch_external_backend() {
        let supervisor = BackendSupervisor::build(None);
        let result = tokio_block(supervisor.stop());
        assert!(result.is_err(), "expected refusal, got {result:?}");
        assert!(
            result.unwrap_err().contains("external"),
            "refusal should name external ownership"
        );
    }

    #[test]
    fn ephemeral_port_selection_yields_a_bindable_port() {
        let supervisor = BackendSupervisor::build(None);
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

        let supervisor = BackendSupervisor::build(None);
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

        let supervisor = BackendSupervisor::build(None);
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
        let supervisor = BackendSupervisor::build(None);
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
        let supervisor = BackendSupervisor::build(None);
        let info = tokio_block(supervisor.info());
        let json = serde_json::to_string(&info).expect("serialize");

        for forbidden in ["apiKey", "api_key", "Authorization", "Bearer", "sk-", "master_token"] {
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
}
