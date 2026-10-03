//! `apeireth onboard` —— 首次使用引导（OOBE）：分流普通用户 / 专业用户。
//!
//! 一次引导完成四件事（按顺序）：
//! 1. **简单介绍**：Apeireth 是什么、有哪些命令、什么默认关闭；
//! 2. **用户分流**：普通用户（预设值 + 使用中自动调参）/ 专业用户（只做介绍）；
//! 3. **API 设置引导**：服务商预设 → 端点 → 模型 → 密钥（密钥只进系统钥匙串，
//!    **任何机密都不落盘到引导档案**）；
//! 4. **自我描述词**：一两句话描述你自己与相处方式，作为身份段注入每次对话。
//!
//! ## 普通用户预设值（使用中自动调整参数）
//!
//! 普通用户写入的引导档案携带两组预设：
//! - **推荐配置**（与 frontend `RECOMMENDED_CAPABILITY_PRESET` / CLI 六旋钮同名镜像）：
//!   记忆核心族六旋钮一键开（危险项 shell/fetch **绝不**进预设）；
//! - **预算族**（上下文预算 / 回合轮数 / 单轮工具调用）：由 [`tuned_knobs`]
//!   依据上一回合用量（输入长度 / 轮数 / 输出 token）**确定性自动伸缩**，
//!   每次 `apeireth chat` 前按最新用量取值（见 [`activate_at`]）。
//!
//! 专业用户路径**只做介绍**：不写档案、不写预设、不碰任何环境变量。
//!
//! ## 优先级与逃生门（0 行为变化承诺）
//!
//! 引导档案永远是**第二优先**：显式环境变量（含非法值）> 引导档案预设 > 运行时默认。
//! 档案只在 `apeireth chat` 系列命令入口补位（gateway / 桌面侧车路径**不**激活，
//! 互不干扰）；`APEIRETH_DISABLE_ONBOARDING=1` 整体关闭档案消费（fail-closed 逃生门）。
//! 没有引导档案时，本模块所有消费点均为 no-op，既有行为逐字节不变。

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_credentials::keyring::{AuditSink, BackendKind, CountingAudit, KeyringBackend};
use apeireth_credentials::keyring_resolver::KeyringCredentialResolver;
use apeireth_credentials::{KeyringSelector, SecretBuf};
use apeireth_plugin::CredentialResolver;
use serde::{Deserialize, Serialize};

/// 引导档案整体逃生门：`=1` 时档案完全不消费（预设 / 调参 / 身份 / 密钥补位全关）。
pub const DISABLE_ONBOARDING_ENV: &str = "APEIRETH_DISABLE_ONBOARDING";

/// 引导档案文件名（数据目录下）。
pub const PROFILE_FILE: &str = "onboarding.json";
/// 自动调参状态文件名（数据目录下）。
pub const TUNING_FILE: &str = "onboarding-tuning.json";

/// 档案 schema 版本（首版）。
pub const PROFILE_SCHEMA_VERSION: u32 = 1;

// ============================================================================
// §1 用户分流
// ============================================================================

/// 用户分流：普通用户 = 预设值 + 自动调参；专业用户 = 只做介绍。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UserTier {
    /// 普通用户：写预设档案，使用中自动调整参数。
    Casual,
    /// 专业用户：只做介绍，不写任何预设。
    Pro,
}

impl UserTier {
    /// 档案里的稳定字面量。
    pub fn as_str(&self) -> &'static str {
        match self {
            UserTier::Casual => "casual",
            UserTier::Pro => "pro",
        }
    }

    /// 分流问答的容错解析：`1/2`、`casual/pro`、`普通(用户)?/专业(用户)?`，
    /// 大小写不敏感；空 / 未知 → `None`（由调用方决定缺省）。
    pub fn parse_answer(answer: &str) -> Option<UserTier> {
        let normalized = answer.trim().to_lowercase();
        match normalized.as_str() {
            "1" | "casual" | "普通" | "普通用户" => Some(UserTier::Casual),
            "2" | "pro" | "professional" | "专业" | "专业用户" => Some(UserTier::Pro),
            _ => None,
        }
    }
}

// ============================================================================
// §2 Provider 预设与通道（非机密配置面）
// ============================================================================

/// 凭据 / 配置通道：决定环境变量名与钥匙串逻辑名。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Channel {
    /// 通用通道（`APEIRETH_API_*`，Minimax 插件承载任意 OpenAI 兼容端点）。
    Generic,
    /// Anthropic 通道（`APEIRETH_ANTHROPIC_*`）。
    Anthropic,
}

impl Channel {
    /// 稳定字面量。
    pub fn as_str(&self) -> &'static str {
        match self {
            Channel::Generic => "generic",
            Channel::Anthropic => "anthropic",
        }
    }

    /// 密钥环境变量名。
    pub fn key_env(&self) -> &'static str {
        match self {
            Channel::Generic => "APEIRETH_API_KEY",
            Channel::Anthropic => "APEIRETH_ANTHROPIC_KEY",
        }
    }

    /// 凭据逻辑名（钥匙串 service / resolver 名，**不是**环境变量名）。
    pub fn credential_name(&self) -> &'static str {
        match self {
            Channel::Generic => "provider.minimax.api_key",
            Channel::Anthropic => "provider.anthropic.api_key",
        }
    }

    /// 端点环境变量名。
    pub fn url_env(&self) -> &'static str {
        match self {
            Channel::Generic => "APEIRETH_API_URL",
            Channel::Anthropic => "APEIRETH_ANTHROPIC_URL",
        }
    }

    /// 模型列表环境变量名（逗号分隔）。
    pub fn models_env(&self) -> &'static str {
        match self {
            Channel::Generic => "APEIRETH_API_MODELS",
            Channel::Anthropic => "APEIRETH_ANTHROPIC_MODELS",
        }
    }

    /// 默认模型环境变量名（两通道共用 `APEIRETH_MODEL`）。
    pub fn default_model_env(&self) -> &'static str {
        "APEIRETH_MODEL"
    }
}

/// 服务商预设（引导问答选项；全部非机密）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderPreset {
    /// 档案里的稳定 id。
    pub id: &'static str,
    /// 展示名。
    pub label: &'static str,
    /// 所属通道。
    pub channel: Channel,
    /// 默认端点。
    pub default_url: &'static str,
    /// 默认模型列表。
    pub default_models: &'static [&'static str],
}

/// 引导问答的服务商预设表（顺序 = 菜单顺序）。
pub const PROVIDER_PRESETS: &[ProviderPreset] = &[
    ProviderPreset {
        id: "deepseek",
        label: "DeepSeek（通用通道）",
        channel: Channel::Generic,
        default_url: "https://api.deepseek.com/v1",
        default_models: &["deepseek-chat"],
    },
    ProviderPreset {
        id: "minimax",
        label: "MiniMax（通用通道）",
        channel: Channel::Generic,
        default_url: "https://api.minimaxi.com/v1",
        default_models: &["MiniMax-M3"],
    },
    ProviderPreset {
        id: "openai-compatible",
        label: "其他 OpenAI 兼容端点（自定义）",
        channel: Channel::Generic,
        default_url: "https://api.openai.com/v1",
        default_models: &["gpt-4o-mini"],
    },
    ProviderPreset {
        id: "anthropic",
        label: "Anthropic 通道",
        channel: Channel::Anthropic,
        default_url: "https://api.minimaxi.com/anthropic",
        default_models: &["claude-sonnet-4-5"],
    },
];

/// 按菜单序号取预设（1 起）。
pub fn preset_by_index(index: usize) -> Option<&'static ProviderPreset> {
    index.checked_sub(1).and_then(|i| PROVIDER_PRESETS.get(i))
}

/// 按稳定 id 取预设。
pub fn preset_by_id(id: &str) -> Option<&'static ProviderPreset> {
    PROVIDER_PRESETS.iter().find(|preset| preset.id == id)
}

// ============================================================================
// §3 引导档案与普通用户预设
// ============================================================================

/// 普通用户预算族预设（自动调参的基准值，[`tuned_knobs`] 在其上伸缩）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CasualPreset {
    /// 注入上下文块总字符预算基准。
    pub context_budget_chars: usize,
    /// 单回合逻辑轮数上限基准。
    pub max_turn_rounds: u32,
    /// 单轮工具调用上限基准。
    pub max_tool_calls: usize,
}

impl Default for CasualPreset {
    /// 与运行时默认（24000 / 8 / 16）同族但略保守：轮数 6、工具 12，
    /// 留出自动上调空间（见 [`tuned_knobs`]）。
    fn default() -> Self {
        Self {
            context_budget_chars: 24_000,
            max_turn_rounds: 6,
            max_tool_calls: 12,
        }
    }
}

/// 引导档案（**绝不含机密**：密钥只进钥匙串，档案只有布尔 `key_stored`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnboardingProfile {
    /// 档案 schema 版本。
    pub schema_version: u32,
    /// 用户分流结果。
    pub tier: UserTier,
    /// 完成引导的时间（epoch 毫秒）。
    pub completed_at_ms: i64,
    /// 人设标识（默认 `apeireth`）。
    pub persona_id: String,
    /// 主体标识（默认 `local-user`）。
    pub subject_id: String,
    /// 自我描述词：一两句话描述你自己与相处方式。
    pub self_description: String,
    /// 服务商预设 id。
    pub provider_id: String,
    /// 配置通道。
    pub channel: Channel,
    /// 端点（非机密）。
    pub api_url: String,
    /// 模型列表（非机密）。
    pub models: Vec<String>,
    /// 默认模型（非机密）。
    pub default_model: String,
    /// 普通用户：使用中自动调整参数（专业用户无档案，恒不适用）。
    pub auto_tune: bool,
    /// 预算族预设基准。
    pub preset: CasualPreset,
    /// 密钥是否已存入系统钥匙串（只记布尔，值永不进档案）。
    pub key_stored: bool,
}

impl OnboardingProfile {
    /// 新建普通用户档案（预设值齐备，调参开）。
    pub fn casual(
        provider_id: impl Into<String>,
        channel: Channel,
        api_url: impl Into<String>,
        models: Vec<String>,
        default_model: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: PROFILE_SCHEMA_VERSION,
            tier: UserTier::Casual,
            completed_at_ms: now_ms(),
            persona_id: "apeireth".to_string(),
            subject_id: "local-user".to_string(),
            self_description: String::new(),
            provider_id: provider_id.into(),
            channel,
            api_url: api_url.into(),
            models,
            default_model: default_model.into(),
            auto_tune: true,
            preset: CasualPreset::default(),
            key_stored: false,
        }
    }
}

// ============================================================================
// §4 档案路径与持久化（路径可注入，测试即真库）
// ============================================================================

/// 引导档案 / 调参状态的落盘位置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnboardingPaths {
    /// `onboarding.json` 路径。
    pub profile: PathBuf,
    /// `onboarding-tuning.json` 路径。
    pub tuning: PathBuf,
}

impl OnboardingPaths {
    /// 数据目录下的标准位置。
    pub fn from_data_dir(data_dir: &Path) -> Self {
        Self {
            profile: data_dir.join(PROFILE_FILE),
            tuning: data_dir.join(TUNING_FILE),
        }
    }

    /// 默认数据目录（`APEIRETH_DATA_DIR` / `~/.apeireth`，与面板/钥匙串同根）。
    pub fn discover() -> Self {
        Self::from_data_dir(&crate::default_panel_data_dir())
    }
}

/// 读档案：文件不存在 → `Ok(None)`；损坏 → `Err`（如实报告，不静默当没有）。
pub fn try_load_profile_at(paths: &OnboardingPaths) -> Result<Option<OnboardingProfile>, String> {
    let bytes = match std::fs::read(&paths.profile) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("引导档案读取失败: {error}")),
    };
    serde_json::from_slice::<OnboardingProfile>(&bytes)
        .map(Some)
        .map_err(|error| format!("引导档案解析失败 ({}): {error}", paths.profile.display()))
}

/// 写档案（目录按需创建；写入为普通 JSON，无机密字段）。
pub fn save_profile_at(paths: &OnboardingPaths, profile: &OnboardingProfile) -> Result<(), String> {
    if let Some(parent) = paths.profile.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("引导档案目录创建失败: {error}"))?;
    }
    let json = serde_json::to_string_pretty(profile)
        .map_err(|error| format!("引导档案序列化失败: {error}"))?;
    std::fs::write(&paths.profile, json).map_err(|error| format!("引导档案写入失败: {error}"))
}

/// 读调参状态；不存在 / 损坏 → `None`（调参状态只影响预设伸缩，宁缺毋错）。
pub fn load_tuning_state_at(paths: &OnboardingPaths) -> Option<TuningState> {
    let bytes = std::fs::read(&paths.tuning).ok()?;
    serde_json::from_slice::<TuningState>(&bytes).ok()
}

/// 写调参状态（best-effort 由调用方决定是否吞错）。
pub fn save_tuning_state_at(paths: &OnboardingPaths, state: &TuningState) -> Result<(), String> {
    if let Some(parent) = paths.tuning.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("调参状态目录创建失败: {error}"))?;
    }
    let json = serde_json::to_string_pretty(state)
        .map_err(|error| format!("调参状态序列化失败: {error}"))?;
    std::fs::write(&paths.tuning, json).map_err(|error| format!("调参状态写入失败: {error}"))
}

/// 清除引导档案与调参状态；返回是否删掉了至少一个文件。
pub fn reset_at(paths: &OnboardingPaths) -> Result<bool, String> {
    let mut removed = false;
    for path in [&paths.profile, &paths.tuning] {
        match std::fs::remove_file(path) {
            Ok(()) => removed = true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("引导档案清除失败 ({}): {error}", path.display())),
        }
    }
    Ok(removed)
}

// ============================================================================
// §5 自动调参（普通用户：使用中自动调整参数）
// ============================================================================

/// 上一回合用量快照（自动调参的输入信号）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TuningState {
    /// 最近更新时间（epoch 毫秒）。
    pub updated_at_ms: i64,
    /// 累计回合数。
    pub turns: u64,
    /// 上一回合输入字符数。
    pub last_prompt_chars: usize,
    /// 上一回合逻辑轮数。
    pub last_rounds: u32,
    /// 上一回合输出字符数。
    pub last_output_chars: usize,
    /// 上一回合输出 token 数。
    pub last_output_tokens: u64,
}

/// 自动调参输出（写入预算族 env 补位值）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TunedKnobs {
    /// 上下文预算（字符）。
    pub context_budget_chars: usize,
    /// 回合轮数上限。
    pub max_turn_rounds: u32,
    /// 单轮工具调用上限。
    pub max_tool_calls: usize,
}

/// 自动调参伸缩区间（确定性钳制，永不越出）。
pub const MIN_CONTEXT_BUDGET_CHARS: usize = 8_000;
/// 见 [`MIN_CONTEXT_BUDGET_CHARS`]。
pub const MAX_CONTEXT_BUDGET_CHARS: usize = 96_000;
/// 见 [`MIN_CONTEXT_BUDGET_CHARS`]。
pub const MIN_TURN_ROUNDS: u32 = 2;
/// 见 [`MIN_CONTEXT_BUDGET_CHARS`]。
pub const MAX_TURN_ROUNDS: u32 = 16;
/// 见 [`MIN_CONTEXT_BUDGET_CHARS`]。
pub const MIN_TOOL_CALLS: usize = 4;
/// 见 [`MIN_CONTEXT_BUDGET_CHARS`]。
pub const MAX_TOOL_CALLS: usize = 32;

/// 确定性自动调参：以预设为基准，按上一回合用量伸缩预算族。
///
/// 三条规则（全部纯函数、可单测；无状态即回预设基准）：
/// 1. **重输入 / 重输出**（输入 ≥ 6000 字符或输出 ≥ 3000 token）→
///    上下文预算 ×1.5、轮数放宽到 ≥ 8；
/// 2. **上回合轮数吃紧**（≥ 5 轮，接近 6 轮基准）→ 轮数 8、工具调用 ≥ 16；
/// 3. **轻回合**（输入 ≤ 200 字符、≤ 2 轮、输出 ≤ 800 token）→
///    轮数收 ≤ 4、工具调用收 ≤ 8（快档，减少空转）。
///
/// 一切结果都钳制在 [`MIN_CONTEXT_BUDGET_CHARS`]..=[`MAX_TOOL_CALLS`] 的显式区间内。
pub fn tuned_knobs(preset: &CasualPreset, state: Option<&TuningState>) -> TunedKnobs {
    let mut knobs = TunedKnobs {
        context_budget_chars: preset.context_budget_chars,
        max_turn_rounds: preset.max_turn_rounds,
        max_tool_calls: preset.max_tool_calls,
    };
    if let Some(state) = state {
        let heavy_input = state.last_prompt_chars >= 6_000;
        let heavy_output = state.last_output_tokens >= 3_000;
        if heavy_input || heavy_output {
            knobs.context_budget_chars = knobs.context_budget_chars * 3 / 2;
            knobs.max_turn_rounds = knobs.max_turn_rounds.max(8);
        }
        if state.last_rounds >= 5 {
            knobs.max_turn_rounds = knobs.max_turn_rounds.max(8);
            knobs.max_tool_calls = knobs.max_tool_calls.max(16);
        }
        let light_turn = state.last_prompt_chars <= 200
            && state.last_rounds <= 2
            && state.last_output_tokens <= 800;
        if light_turn && state.turns > 0 {
            knobs.max_turn_rounds = knobs.max_turn_rounds.min(4);
            knobs.max_tool_calls = knobs.max_tool_calls.min(8);
        }
    }
    TunedKnobs {
        context_budget_chars: knobs
            .context_budget_chars
            .clamp(MIN_CONTEXT_BUDGET_CHARS, MAX_CONTEXT_BUDGET_CHARS),
        max_turn_rounds: knobs
            .max_turn_rounds
            .clamp(MIN_TURN_ROUNDS, MAX_TURN_ROUNDS),
        max_tool_calls: knobs.max_tool_calls.clamp(MIN_TOOL_CALLS, MAX_TOOL_CALLS),
    }
}

/// 普通用户回合一结束就回写用量（best-effort：失败静默，不影响回合结果）。
pub fn record_turn_usage(
    prompt_chars: usize,
    response: &apeireth_runtime::canonical::TurnResponse,
) {
    record_usage_at(
        &OnboardingPaths::discover(),
        prompt_chars,
        response.rounds,
        response.text.chars().count(),
        u64::from(response.usage.completion_tokens),
    );
}

/// [`record_turn_usage`] 的路径可注入版本（测试用）。
pub fn record_turn_usage_at(
    paths: &OnboardingPaths,
    prompt_chars: usize,
    response: &apeireth_runtime::canonical::TurnResponse,
) {
    record_usage_at(
        paths,
        prompt_chars,
        response.rounds,
        response.text.chars().count(),
        u64::from(response.usage.completion_tokens),
    );
}

/// 用量回写内核（纯标量，[`record_turn_usage`] / [`record_turn_usage_at`] 共用）。
///
/// 无档案 / 非普通用户 / 自动调参关 → no-op；写失败静默（调参状态是优化信号
/// 而非契约，绝不影响回合结果）。
pub fn record_usage_at(
    paths: &OnboardingPaths,
    prompt_chars: usize,
    rounds: u32,
    output_chars: usize,
    output_tokens: u64,
) {
    let Some(profile) = active_profile_at(paths) else {
        return;
    };
    if profile.tier != UserTier::Casual || !profile.auto_tune {
        return;
    }
    let mut state = load_tuning_state_at(paths).unwrap_or_default();
    state.turns = state.turns.saturating_add(1);
    state.last_prompt_chars = prompt_chars;
    state.last_rounds = rounds;
    state.last_output_chars = output_chars;
    state.last_output_tokens = output_tokens;
    state.updated_at_ms = now_ms();
    let _ = save_tuning_state_at(paths, &state);
}

// ============================================================================
// §6 档案激活（进程级 env 补位；显式 env 永远最高）
// ============================================================================

/// 激活结果摘要（只含变量**名**，永不回显任何值）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActivationSummary {
    /// 是否找到并消费了档案。
    pub applied: bool,
    /// 本次补位的环境变量名（未设才补；已设 = 显式用户意志，不覆盖）。
    pub filled_env: Vec<String>,
    /// 是否从钥匙串补位了密钥（只记布尔）。
    pub key_seeded: bool,
}

/// 生效中的档案：`APEIRETH_DISABLE_ONBOARDING=1` 或无档案 / 档案损坏 → `None`。
pub fn active_profile_at(paths: &OnboardingPaths) -> Option<OnboardingProfile> {
    if onboarding_disabled() {
        return None;
    }
    try_load_profile_at(paths).ok().flatten()
}

/// [`active_profile_at`] 的默认路径版本。
pub fn active_profile() -> Option<OnboardingProfile> {
    active_profile_at(&OnboardingPaths::discover())
}

fn onboarding_disabled() -> bool {
    std::env::var(DISABLE_ONBOARDING_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// 在进程环境里为**未设置**的变量补上引导档案预设（`apeireth chat` 系列入口调用）。
///
/// 补位内容（全部只在对应 env 未设 / 空白时写入）：
/// 1. 通道配置：端点 / 模型列表 / 默认模型；
/// 2. 普通用户推荐配置：记忆核心族六旋钮 = `1`（与 frontend
///    `RECOMMENDED_CAPABILITY_PRESET` 同名镜像；危险项 shell/fetch **绝不**触碰）；
/// 3. 自动调参预算族：上下文预算 / 轮数 / 工具调用（`auto_tune` 开时按最新用量取值）；
/// 4. 密钥补位：钥匙串里存过的密钥（`key_stored`）按通道补进对应 env。
///
/// **优先级契约**：显式环境变量（哪怕非法）> 档案预设 > 运行时默认 —— 本函数
/// 永不覆盖已设变量，因此用户/桌面端的显式配置始终最高优先。
pub fn activate_at(paths: &OnboardingPaths) -> ActivationSummary {
    let mut summary = ActivationSummary::default();
    let Some(profile) = active_profile_at(paths) else {
        return summary;
    };
    summary.applied = true;

    // 1. 通道配置（非机密）。
    fill_env(profile.channel.url_env(), &profile.api_url, &mut summary);
    if !profile.models.is_empty() {
        fill_env(
            profile.channel.models_env(),
            &profile.models.join(","),
            &mut summary,
        );
    }
    fill_env(
        profile.channel.default_model_env(),
        &profile.default_model,
        &mut summary,
    );

    if profile.tier == UserTier::Casual {
        // 2. 推荐配置六旋钮（危险项不进预设：shell/fetch/file-write 零触碰）。
        for knob in RECOMMENDED_PRESET_ENVS {
            fill_env(knob, "1", &mut summary);
        }
        // 3. 自动调参预算族。
        if profile.auto_tune {
            let tuned = tuned_knobs(&profile.preset, load_tuning_state_at(paths).as_ref());
            fill_env(
                "APEIRETH_CONTEXT_BUDGET_CHARS",
                &tuned.context_budget_chars.to_string(),
                &mut summary,
            );
            fill_env(
                "APEIRETH_MAX_TURN_ROUNDS",
                &tuned.max_turn_rounds.to_string(),
                &mut summary,
            );
            fill_env(
                "APEIRETH_MAX_TOOL_CALLS",
                &tuned.max_tool_calls.to_string(),
                &mut summary,
            );
        }
    }

    // 4. 密钥补位（只在 env 未设且档案记录了钥匙串写入时；值不进摘要）。
    if profile.key_stored && env_unset(profile.channel.key_env()) {
        if let Some(key) = read_stored_api_key(profile.channel.credential_name()) {
            if !key.is_empty() {
                std::env::set_var(profile.channel.key_env(), key);
                summary
                    .filled_env
                    .push(profile.channel.key_env().to_string());
                summary.key_seeded = true;
            }
        }
    }
    summary
}

/// [`activate_at`] 的默认路径版本（`apeireth chat` 等命令入口调用）。
pub fn activate_for_process() -> ActivationSummary {
    activate_at(&OnboardingPaths::discover())
}

/// 「推荐配置」六旋钮（与 frontend `RECOMMENDED_CAPABILITY_PRESET` /
/// `production_knobs::recommended_preset_maps_to_core_memory_knobs` 同名镜像）。
pub const RECOMMENDED_PRESET_ENVS: &[&str] = &[
    "APEIRETH_ENABLE_PROACTIVE_RECALL",
    "APEIRETH_ENABLE_PREFERENCE_LEARNING",
    "APEIRETH_ENABLE_MEMORY_INJECTION",
    "APEIRETH_ENABLE_CONSOLIDATION",
    "APEIRETH_ENABLE_REFLEXION",
    "APEIRETH_ENABLE_ORGANS",
];

fn env_unset(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .is_none_or(|value| value.trim().is_empty())
}

fn fill_env(name: &str, value: &str, summary: &mut ActivationSummary) {
    if value.trim().is_empty() || !env_unset(name) {
        return;
    }
    std::env::set_var(name, value);
    summary.filled_env.push(name.to_string());
}

// ============================================================================
// §7 自我描述词（身份段注入）
// ============================================================================

/// 身份系统段：把自我描述词 + 主体/人设标识带进每次 CLI 对话。
///
/// 无档案 / 无自我描述词 / 逃生门开启 → `None`（既有对话逐字节不变）。
pub fn identity_system_block() -> Option<String> {
    identity_system_block_at(&OnboardingPaths::discover())
}

/// [`identity_system_block`] 的路径可注入版本（测试用）。
pub fn identity_system_block_at(paths: &OnboardingPaths) -> Option<String> {
    let profile = active_profile_at(paths)?;
    if profile.self_description.trim().is_empty() {
        return None;
    }
    Some(format!(
        "【用户自我描述】{}\n（主体 {} / 人设 {} —— 来自 apeireth onboard 引导的自我描述词设置。）",
        profile.self_description.trim(),
        profile.subject_id,
        profile.persona_id,
    ))
}

// ============================================================================
// §8 钥匙串（密钥唯一落点；档案/日志/摘要零机密）
// ============================================================================

/// 密钥写入结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredKey {
    /// 实际使用的钥匙串后端。
    pub kind: BackendKind,
    /// 是否跨进程持久（`in-memory` 后端 = 否，如实上报）。
    pub durable: bool,
}

fn selected_keyring() -> Result<(Box<dyn KeyringBackend>, BackendKind), String> {
    // 与 `keyring_bootstrap::build_keyring_resolver` 同一选择语义:
    // `APEIRETH_KEYRING_BACKEND` 显式指定后端; 未设 = auto（平台钥匙串优先,
    // 自动降级加密文件后端）。写侧与读侧共用本函数, 永远对称。
    let env_value = std::env::var("APEIRETH_KEYRING_BACKEND").ok();
    let audit: Arc<dyn AuditSink> = Arc::new(CountingAudit::new());
    let selected = KeyringSelector::select(env_value.as_deref(), audit, None)
        .map_err(|error| format!("钥匙串后端选择失败: {error}"))?;
    Ok((selected.backend, selected.kind))
}

/// 把 API Key 写入系统钥匙串（平台钥匙串优先，自动降级加密文件后端）。
///
/// **只进钥匙串**：引导档案、调参状态、输出摘要、日志都拿不到密钥值。
/// `in-memory` 后端（极端降级）返回 `durable: false`，调用方必须如实告知
/// 用户改用环境变量，不能假装已持久。
pub fn store_api_key(credential_name: &str, key: &str) -> Result<StoredKey, String> {
    if key.trim().is_empty() {
        return Err("API Key 为空，未写入".to_string());
    }
    let (backend, kind) = selected_keyring()?;
    backend
        .set(credential_name, &SecretBuf::from_str(key))
        .map_err(|error| format!("钥匙串写入失败: {error}"))?;
    Ok(StoredKey {
        kind,
        durable: kind != BackendKind::InMemory,
    })
}

/// 从系统钥匙串读回存过的密钥（仅在档案记录 `key_stored` 时被激活路径消费）。
///
/// `in-memory` 后端不跨进程，直接视为无。
pub fn read_stored_api_key(credential_name: &str) -> Option<String> {
    let (backend, kind) = selected_keyring().ok()?;
    if kind == BackendKind::InMemory {
        return None;
    }
    let resolver = KeyringCredentialResolver::new(backend.into());
    resolver
        .resolve(credential_name)
        .map(|secret| secret.expose().to_string())
}

// ============================================================================
// §9 引导流程（交互脚本；IO 可注入 = 可单测）
// ============================================================================

/// 引导选项。
#[derive(Debug, Clone)]
pub struct WizardOptions {
    /// 档案落盘位置。
    pub paths: OnboardingPaths,
    /// 预选用户分流（`--tier`）；未选则交互询问。
    pub tier_override: Option<UserTier>,
    /// 是否真的写钥匙串（测试传 `false`，绝不碰真实钥匙串）。
    pub store_keys: bool,
}

/// 引导主流程：介绍 → 分流 →（普通用户：API 设置 + 自我描述词 + 预设；
/// 专业用户：只做介绍）。返回收尾摘要行。
pub fn run_wizard<R: BufRead, W: Write>(
    input: &mut R,
    output: &mut W,
    options: &WizardOptions,
) -> Result<String, String> {
    // ① 先行简单介绍。
    write_str(output, &intro_text())?;

    // ② 分流。
    let tier = match options.tier_override {
        Some(tier) => {
            write_line(
                output,
                &format!("\n用户类型（--tier 预选）：{}", tier_label(tier)),
            )?;
            tier
        }
        None => {
            write_str(output, "\n你是哪类用户？\n  [1] 普通用户（推荐：预设值 + 使用中自动调参）\n  [2] 专业用户（只做介绍，自己配置）\n")?;
            let answer = ask(output, input, "请选择", Some("1"))?;
            match UserTier::parse_answer(&answer) {
                Some(tier) => tier,
                // 空回车 = 缺省普通用户；明确乱输则如实报错，不猜。
                None if answer.trim().is_empty() => UserTier::Casual,
                None => return Err(format!("无法识别的用户类型: {answer}")),
            }
        }
    };

    if tier == UserTier::Pro {
        // ③' 专业用户：只做介绍，不写任何预设 / 档案 / 环境变量。
        write_str(output, &pro_reference_text())?;
        return Ok("专业用户引导完成（只做介绍，未写入任何预设；随时可 `apeireth onboard --tier casual` 换轨）".to_string());
    }

    // ③ 普通用户 · API 设置引导。
    write_str(output, "\n—— 第一步 · API 设置 ——\n")?;
    for (index, preset) in PROVIDER_PRESETS.iter().enumerate() {
        write_line(output, &format!("  [{}] {}", index + 1, preset.label))?;
    }
    let choice = ask(output, input, "选择服务商", Some("1"))?;
    let index: usize = match choice.trim().parse() {
        Ok(index) => index,
        Err(_) => return Err(format!("服务商序号无效: {choice}")),
    };
    let preset = preset_by_index(index).ok_or_else(|| format!("服务商序号超出范围: {index}"))?;

    let api_url = ask(
        output,
        input,
        &format!("API 端点（{}）", preset.channel.url_env()),
        Some(preset.default_url),
    )?;
    let models_raw = ask(
        output,
        input,
        "可用模型列表（逗号分隔）",
        Some(&preset.default_models.join(",")),
    )?;
    let models: Vec<String> = models_raw
        .split(',')
        .map(|model| model.trim().to_string())
        .filter(|model| !model.is_empty())
        .collect();
    if models.is_empty() {
        return Err("模型列表为空".to_string());
    }
    let default_model = ask(output, input, "默认模型", Some(&models[0]))?;

    write_line(
        output,
        &format!(
            "API Key（{}）：粘贴后只写入系统钥匙串，不写入任何文件、不回显；直接回车 = 稍后自己用环境变量设置",
            preset.channel.key_env()
        ),
    )?;
    let api_key = read_line(input)?;
    let mut key_stored = false;
    if !api_key.trim().is_empty() {
        if options.store_keys {
            match store_api_key(preset.channel.credential_name(), api_key.trim()) {
                Ok(stored) if stored.durable => {
                    key_stored = true;
                    write_line(
                        output,
                        "✓ API Key 已写入系统钥匙串（重启自动生效，无需重输）",
                    )?;
                }
                Ok(_) => {
                    write_line(
                        output,
                        "⚠ 当前环境只有内存钥匙串（不跨进程持久）。请改用环境变量：",
                    )?;
                    write_line(
                        output,
                        &format!("    $env:{} = \"sk-…\"", preset.channel.key_env()),
                    )?;
                }
                Err(error) => {
                    write_line(
                        output,
                        &format!("⚠ 钥匙串写入失败：{error}。请改用环境变量："),
                    )?;
                    write_line(
                        output,
                        &format!("    $env:{} = \"sk-…\"", preset.channel.key_env()),
                    )?;
                }
            }
        } else {
            write_line(output, "（钥匙串写入已在本次引导禁用，密钥未保存）")?;
        }
    } else {
        write_line(
            output,
            &format!(
                "稍后请设置环境变量：$env:{} = \"sk-…\"",
                preset.channel.key_env()
            ),
        )?;
    }

    // ④ 自我描述词。
    write_str(output, "\n—— 第二步 · 自我描述词 ——\n")?;
    let self_description = ask(
        output,
        input,
        "用一两句话描述你自己，以及希望他如何与你相处\n（例：『我是学生，常用中文，回答请简洁可靠』）",
        Some("我是 Apeireth 的日常使用者，希望以中文沟通，回答简洁、可靠。"),
    )?;
    let subject_id = ask(
        output,
        input,
        "怎么称呼你？（记忆主体标识）",
        Some("local-user"),
    )?;
    let persona_id = ask(output, input, "人设标识（多档案隔离用）", Some("apeireth"))?;

    // ⑤ 普通用户预设值 + 自动调参确认。
    write_str(output, "\n—— 第三步 · 普通用户预设值 ——\n")?;
    let preset_defaults = CasualPreset::default();
    write_line(
        output,
        &format!(
            "  推荐配置（记忆核心族六旋钮一键开；危险项 shell/fetch/文件写不进预设）\n  预算预设：上下文 {} 字符 / 回合轮数 {} / 单轮工具调用 {}",
            preset_defaults.context_budget_chars,
            preset_defaults.max_turn_rounds,
            preset_defaults.max_tool_calls
        ),
    )?;
    write_str(
        output,
        "  自动调参：按上一回合用量自动伸缩（上下文 8000..96000 字符 / 轮数 2..16 / 工具 4..32）\n  [1] 应用预设 + 自动调参（推荐）\n  [2] 应用预设但关闭自动调参\n  [3] 取消（不写入任何设置）\n",
    )?;
    let apply = ask(output, input, "请选择", Some("1"))?;
    let auto_tune = match apply.trim() {
        "1" => true,
        "2" => false,
        "3" => return Ok("已取消，未写入任何设置".to_string()),
        other => return Err(format!("无法识别的选择: {other}")),
    };

    // ⑥ 落盘（普通用户档案；无任何机密字段）。
    let mut profile =
        OnboardingProfile::casual(preset.id, preset.channel, api_url, models, default_model);
    profile.self_description = self_description;
    profile.subject_id = subject_id;
    profile.persona_id = persona_id;
    profile.auto_tune = auto_tune;
    profile.key_stored = key_stored;
    save_profile_at(&options.paths, &profile)?;

    write_str(
        output,
        &format!(
            "\n✓ 引导完成。档案：{}\n接下来试试：apeireth chat \"你好\" ｜ 查看档案：apeireth onboard --show ｜ 重新引导：apeireth onboard --reset\n",
            options.paths.profile.display()
        ),
    )?;
    Ok(format!(
        "普通用户引导完成（预设已写入，自动调参：{}）",
        if auto_tune { "开" } else { "关" }
    ))
}

/// 简单介绍（引导第 ① 步，先行输出）。
pub fn intro_text() -> String {
    "\n欢迎使用 Apeireth —— 一次把「怎么用」讲清楚（约 2 分钟）。\n\
     \n\
     Apeireth 是一个本地优先的 AI 助手运行时：\n\
     \x20 · 对话：apeireth chat \"你的问题\"（多会话、审批、审计都在本地留痕）\n\
     \x20 · 网关：apeireth gateway serve 供桌面端 / 前端接入\n\
     \x20 · 治理：危险能力（shell / fetch / 文件写）默认关闭，逐次人工审批\n\
     \x20 · 记忆：本地 SQLite 记忆库，记忆核心族可一键开启\n\
     \x20 · 认知：dream 闲时复盘 / council 议事 / subagent 长程任务 / nightwatch 守夜审计\n\
     \n\
     本次引导会带你完成：\n\
     \x20 ① 选择用户类型（普通用户 / 专业用户）\n\
     \x20 ② API 设置（服务商、端点、模型、密钥）\n\
     \x20 ③ 自我描述词（他如何理解你）\n\
     \x20 ④ 普通用户预设值（使用中自动调整参数）；专业用户只做介绍\n"
        .to_string()
}

/// 专业用户参考卡（只做介绍；全部配置旋钮一览）。
pub fn pro_reference_text() -> String {
    format!(
        "\n—— 专业用户通道（只做介绍，不写任何预设 / 档案 / 环境变量）——\n\
         一切都可以用环境变量精确控制，**显式环境变量永远最高优先**：\n\
         \x20 Provider（通用通道）：APEIRETH_API_KEY / APEIRETH_API_URL / APEIRETH_API_MODELS / APEIRETH_MODEL\n\
         \x20 Provider（Anthropic 通道）：APEIRETH_ANTHROPIC_KEY / APEIRETH_ANTHROPIC_URL / APEIRETH_ANTHROPIC_MODELS\n\
         \x20 Provider（OpenAI 兼容通道）：OPENAI_API_KEY / APEIRETH_OPENAI_URL / APEIRETH_OPENAI_MODELS\n\
         \x20 身份：APEIRETH_PERSONA_ID / APEIRETH_SUBJECT_ID\n\
         \x20 预算：APEIRETH_CONTEXT_BUDGET_CHARS / APEIRETH_MAX_TURN_ROUNDS / APEIRETH_MAX_TOOL_CALLS\n\
         \x20 能力：APEIRETH_ENABLE_SHELL / _FETCH / _FILE_WRITE / _MCP / _ORGANS（默认关闭 fail-closed）\n\
         \x20 记忆：APEIRETH_ENABLE_PROACTIVE_RECALL / _PREFERENCE_LEARNING / _MEMORY_INJECTION / _CONSOLIDATION / _REFLEXION\n\
         \x20 逃生门：{disable}=1（禁用引导档案整体消费）\n\
         完整旋钮手册：docs/02-guides/user-manual.md ｜ 引导说明：docs/02-guides/cli-onboarding.md ｜ .env.example\n",
        disable = DISABLE_ONBOARDING_ENV,
    )
}

fn tier_label(tier: UserTier) -> &'static str {
    match tier {
        UserTier::Casual => "普通用户（预设值 + 自动调参）",
        UserTier::Pro => "专业用户（只做介绍）",
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

fn write_line<W: Write>(output: &mut W, line: &str) -> Result<(), String> {
    writeln!(output, "{line}").map_err(|error| format!("输出失败: {error}"))
}

fn write_str<W: Write>(output: &mut W, text: &str) -> Result<(), String> {
    write!(output, "{text}").map_err(|error| format!("输出失败: {error}"))
}

fn read_line<R: BufRead>(input: &mut R) -> Result<String, String> {
    let mut line = String::new();
    input
        .read_line(&mut line)
        .map_err(|error| format!("读取输入失败: {error}"))?;
    if line.is_empty() {
        return Err("输入已结束（EOF）".to_string());
    }
    Ok(line)
}

fn ask<R: BufRead, W: Write>(
    output: &mut W,
    input: &mut R,
    question: &str,
    default: Option<&str>,
) -> Result<String, String> {
    match default {
        Some(default) => write_line(output, &format!("{question}（回车 = {default}）"))?,
        None => write_line(output, question)?,
    }
    let answer = read_line(input)?;
    let answer = answer.trim().to_string();
    if answer.is_empty() {
        match default {
            Some(default) => Ok(default.to_string()),
            None => Err("该项不能为空".to_string()),
        }
    } else {
        Ok(answer)
    }
}

// ============================================================================
// §10 单元测试（纯函数 + 路径可注入 IO；不碰真实钥匙串 / 不碰进程 env）
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn temp_paths(name: &str) -> (tempfile::TempDir, OnboardingPaths) {
        let dir = tempfile::tempdir().expect("temp dir");
        let paths = OnboardingPaths::from_data_dir(dir.path().join(name).as_path());
        (dir, paths)
    }

    #[test]
    fn tier_parses_tolerantly_and_rejects_unknown() {
        for answer in ["1", "casual", "Casual", "普通", "普通用户"] {
            assert_eq!(
                UserTier::parse_answer(answer),
                Some(UserTier::Casual),
                "{answer}"
            );
        }
        for answer in ["2", "pro", "PRO", "professional", "专业", "专业用户"] {
            assert_eq!(
                UserTier::parse_answer(answer),
                Some(UserTier::Pro),
                "{answer}"
            );
        }
        assert_eq!(UserTier::parse_answer(""), None);
        assert_eq!(UserTier::parse_answer("vip"), None);
    }

    #[test]
    fn provider_presets_cover_both_channels_and_index_lookup() {
        assert_eq!(PROVIDER_PRESETS.len(), 4);
        assert_eq!(preset_by_index(1).unwrap().id, "deepseek");
        assert_eq!(preset_by_index(4).unwrap().channel, Channel::Anthropic);
        assert!(preset_by_index(0).is_none());
        assert!(preset_by_index(5).is_none());
        assert_eq!(
            preset_by_id("minimax").unwrap().label,
            "MiniMax（通用通道）"
        );
        assert!(preset_by_id("nope").is_none());
        // 通道环境变量名与 .env.example / provider credentials 实际读取一致。
        assert_eq!(Channel::Generic.key_env(), "APEIRETH_API_KEY");
        assert_eq!(
            Channel::Generic.credential_name(),
            "provider.minimax.api_key"
        );
        assert_eq!(Channel::Anthropic.key_env(), "APEIRETH_ANTHROPIC_KEY");
    }

    #[test]
    fn profile_serde_roundtrip_contains_no_secret_fields() {
        let mut profile = OnboardingProfile::casual(
            "deepseek",
            Channel::Generic,
            "https://api.deepseek.com/v1",
            vec!["deepseek-chat".to_string()],
            "deepseek-chat",
        );
        profile.self_description = "我是学生".to_string();
        profile.key_stored = true;
        let json = serde_json::to_string(&profile).expect("serialize");
        assert!(!json.contains("sk-"), "档案绝不含密钥形态字段: {json}");
        assert!(json.contains("\"key-stored\"") || json.contains("\"key_stored\""));
        let back: OnboardingProfile = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, profile);
        assert_eq!(back.tier, UserTier::Casual);
        assert!(back.auto_tune, "普通用户缺省自动调参开");
    }

    #[test]
    fn profile_roundtrips_through_real_files() {
        let (_dir, paths) = temp_paths("profile-io");
        let profile = OnboardingProfile::casual(
            "deepseek",
            Channel::Generic,
            "https://api.deepseek.com/v1",
            vec!["deepseek-chat".to_string()],
            "deepseek-chat",
        );
        save_profile_at(&paths, &profile).expect("save");
        let loaded = try_load_profile_at(&paths).expect("load").expect("present");
        assert_eq!(loaded, profile);
        assert!(reset_at(&paths).expect("reset"));
        assert_eq!(try_load_profile_at(&paths).expect("load"), None);
        assert!(!reset_at(&paths).expect("reset again"), "幂等清除");
    }

    #[test]
    fn damaged_profile_is_reported_not_silently_ignored() {
        let (_dir, paths) = temp_paths("profile-broken");
        std::fs::create_dir_all(paths.profile.parent().unwrap()).unwrap();
        std::fs::write(&paths.profile, b"{not json").unwrap();
        assert!(try_load_profile_at(&paths).is_err(), "损坏档案要大声失败");
        // 激活侧宁缺毋错：损坏 = 不激活（与无档案同样零行为变化）。
        assert!(active_profile_at(&paths).is_none());
    }

    #[test]
    fn tuned_knobs_follow_usage_deterministically() {
        let preset = CasualPreset::default();
        // 无状态 = 预设基准。
        let base = tuned_knobs(&preset, None);
        assert_eq!(base.context_budget_chars, 24_000);
        assert_eq!(base.max_turn_rounds, 6);
        assert_eq!(base.max_tool_calls, 12);

        // 规则 1: 重输入 → 预算 ×1.5、轮数 ≥ 8。
        let heavy = tuned_knobs(
            &preset,
            Some(&TuningState {
                turns: 3,
                last_prompt_chars: 9_000,
                last_rounds: 1,
                last_output_tokens: 100,
                ..Default::default()
            }),
        );
        assert_eq!(heavy.context_budget_chars, 36_000);
        assert_eq!(heavy.max_turn_rounds, 8);

        // 规则 2: 上回合轮数吃紧 → 轮数 8 / 工具 ≥ 16。
        let tight = tuned_knobs(
            &preset,
            Some(&TuningState {
                turns: 5,
                last_prompt_chars: 500,
                last_rounds: 6,
                last_output_tokens: 400,
                ..Default::default()
            }),
        );
        assert_eq!(tight.max_turn_rounds, 8);
        assert_eq!(tight.max_tool_calls, 16);

        // 规则 3: 轻回合 → 快档 (轮 ≤ 4 / 工具 ≤ 8)。
        let light = tuned_knobs(
            &preset,
            Some(&TuningState {
                turns: 10,
                last_prompt_chars: 80,
                last_rounds: 1,
                last_output_tokens: 120,
                ..Default::default()
            }),
        );
        assert_eq!(light.max_turn_rounds, 4);
        assert_eq!(light.max_tool_calls, 8);

        // 钳制: 极端预设也永不越界。
        let extreme = CasualPreset {
            context_budget_chars: 10,
            max_turn_rounds: 999,
            max_tool_calls: 0,
        };
        let clamped = tuned_knobs(&extreme, None);
        assert_eq!(clamped.context_budget_chars, MIN_CONTEXT_BUDGET_CHARS);
        assert_eq!(clamped.max_turn_rounds, MAX_TURN_ROUNDS);
        assert_eq!(clamped.max_tool_calls, MIN_TOOL_CALLS);
    }

    #[test]
    fn identity_block_carries_self_description_and_defaults_to_none() {
        let (_dir, paths) = temp_paths("identity");
        assert_eq!(identity_system_block_at(&paths), None, "无档案 = 无注入");

        let mut profile = OnboardingProfile::casual(
            "deepseek",
            Channel::Generic,
            "https://api.deepseek.com/v1",
            vec!["deepseek-chat".to_string()],
            "deepseek-chat",
        );
        profile.self_description = "我是学生，回答简洁".to_string();
        profile.subject_id = "xiao-ming".to_string();
        save_profile_at(&paths, &profile).expect("save");
        let block = identity_system_block_at(&paths).expect("block");
        assert!(block.contains("我是学生，回答简洁"));
        assert!(block.contains("xiao-ming"));
        assert!(block.contains("自我描述"));

        // 空自我描述词 = 不注入。
        profile.self_description = "   ".to_string();
        save_profile_at(&paths, &profile).expect("save");
        assert_eq!(identity_system_block_at(&paths), None);
    }

    #[test]
    fn wizard_pro_flow_only_introduces_and_writes_nothing() {
        let (_dir, paths) = temp_paths("wizard-pro");
        let mut input = Cursor::new(b"2\n".to_vec());
        let mut output: Vec<u8> = Vec::new();
        let summary = run_wizard(
            &mut input,
            &mut output,
            &WizardOptions {
                paths: paths.clone(),
                tier_override: None,
                store_keys: false,
            },
        )
        .expect("wizard");
        let text = String::from_utf8(output).expect("utf8");
        assert!(text.contains("Apeireth 是一个本地优先"), "先介绍");
        assert!(
            text.contains("APEIRETH_API_KEY"),
            "专业用户拿到完整旋钮介绍"
        );
        assert!(text.contains("只做介绍"));
        assert!(summary.contains("只做介绍"));
        assert!(
            try_load_profile_at(&paths).expect("load").is_none(),
            "专业用户不写任何档案"
        );
    }

    #[test]
    fn wizard_casual_flow_writes_profile_with_presets_and_self_description() {
        let (_dir, paths) = temp_paths("wizard-casual");
        // 回答: 服务商 1（DeepSeek）/ 端点 回车 / 模型 回车 / 默认模型 回车 /
        // key 空（跳过）/ 自我描述词 / 称呼 / 人设 回车 / 应用 1。
        let script = "1\n\n\n\n\n我是工程师，回答直接给出结论\n小明\n\n1\n";
        let mut input = Cursor::new(script.as_bytes().to_vec());
        let mut output: Vec<u8> = Vec::new();
        let summary = run_wizard(
            &mut input,
            &mut output,
            &WizardOptions {
                paths: paths.clone(),
                tier_override: Some(UserTier::Casual),
                store_keys: false,
            },
        )
        .expect("wizard");
        assert!(summary.contains("自动调参：开"), "{summary}");

        let profile = try_load_profile_at(&paths)
            .expect("load")
            .expect("casual 档案必须落盘");
        assert_eq!(profile.tier, UserTier::Casual);
        assert_eq!(profile.provider_id, "deepseek");
        assert_eq!(profile.channel, Channel::Generic);
        assert_eq!(profile.api_url, "https://api.deepseek.com/v1");
        assert_eq!(profile.models, vec!["deepseek-chat".to_string()]);
        assert_eq!(profile.default_model, "deepseek-chat");
        assert_eq!(profile.self_description, "我是工程师，回答直接给出结论");
        assert_eq!(profile.subject_id, "小明");
        assert_eq!(profile.persona_id, "apeireth");
        assert!(profile.auto_tune);
        assert!(!profile.key_stored, "跳过 key = 不假装已存");
        assert_eq!(profile.preset, CasualPreset::default());

        let json = serde_json::to_string(&profile).expect("json");
        assert!(!json.contains("sk-"), "档案零机密: {json}");
    }

    #[test]
    fn wizard_can_disable_auto_tune_but_keeps_presets() {
        let (_dir, paths) = temp_paths("wizard-no-tune");
        let script = "\n\n\n\n\n\n\n\n2\n";
        let mut input = Cursor::new(script.as_bytes().to_vec());
        let mut output: Vec<u8> = Vec::new();
        run_wizard(
            &mut input,
            &mut output,
            &WizardOptions {
                paths: paths.clone(),
                tier_override: Some(UserTier::Casual),
                store_keys: false,
            },
        )
        .expect("wizard");
        let profile = try_load_profile_at(&paths).expect("load").expect("present");
        assert!(!profile.auto_tune);
        assert_eq!(profile.preset, CasualPreset::default(), "预设仍然写入");
    }

    #[test]
    fn wizard_cancel_writes_nothing() {
        let (_dir, paths) = temp_paths("wizard-cancel");
        let script = "\n\n\n\n\n\n\n\n3\n";
        let mut input = Cursor::new(script.as_bytes().to_vec());
        let mut output: Vec<u8> = Vec::new();
        let summary = run_wizard(
            &mut input,
            &mut output,
            &WizardOptions {
                paths: paths.clone(),
                tier_override: Some(UserTier::Casual),
                store_keys: false,
            },
        )
        .expect("wizard");
        assert!(summary.contains("取消"));
        assert!(try_load_profile_at(&paths).expect("load").is_none());
    }

    #[test]
    fn wizard_rejects_unknown_tier_answer() {
        let (_dir, paths) = temp_paths("wizard-bad-tier");
        let mut input = Cursor::new(b"vip\n".to_vec());
        let mut output: Vec<u8> = Vec::new();
        let err = run_wizard(
            &mut input,
            &mut output,
            &WizardOptions {
                paths: paths.clone(),
                tier_override: None,
                store_keys: false,
            },
        )
        .expect_err("乱输要大声失败");
        assert!(err.contains("无法识别"), "{err}");
    }
}
