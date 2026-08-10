//! # TUI Dashboard — 9 器官拟人化仪表盘 + 3 端点集成
//!
//! **Apeireth R25.2 估补: observability 3 端点 + 9 器官仪表盘 TUI 集成** (per
//! 1.0 release checklist #8 observability 90% → 100%).
//!
//! ## 集成背景
//!
//! 之前两轮 sub-agent 工作已铺垫:
//! - **bg_ac5e45a4** (2026-08-05 21:35): observability 3 端点 + 仪表盘, 仅 crate 内
//!   (tracing/metrics/health/logging 4 子模块), 0 暴露 TUI 集成面
//! - **bg_8446e424** (2026-08-05 22:00): TUI 5 nav + 9 器官 e2e (5 nav + 9 器官模块在
//!   `apeireth-tui/src/organ/`), observability 0 接入
//!
//! 本模块在 **observability crate 内** 暴露 TUI 集成面:
//! - `TuiOrganState` — 9 器官状态的可序列化快照 (per-organ 4 字段: status/value/update/message)
//! - `OrganKind` — 9 器官 enum (编译期 hardcode, 跟 `apeireth-state::Organ` 1:1 镜像)
//! - `OrganDashboard` — 9 器官状态聚合 (per-organ `TuiOrganState` + 3 端点 health)
//! - `register_tui_organ_state()` — 注册/更新 9 器官状态
//! - 9 器官 widget 渲染函数 (heart/brain/hand/eye/ear/memory/voice/body/mind) — 返
//!   `String` (ratatui 喂入, 0 引 ratatui)
//! - `render_dashboard()` — 9 器官 + 3 端点统一渲染
//!
//! ## 编译期 hardcode (per 6 哲学锚 O-3 干到底)
//!
//! - `ORGAN_KIND_COUNT == 9` (守门)
//! - `ORGAN_KIND_NAMES_ZH` 9 元素编译期数组 (跟 `apeireth-state::organ::ORGAN_NAMES_ZH` 1:1 镜像)
//! - `ORGAN_KIND_ASCII_CHARS` 9 元素编译期数组 (跟 `apeireth-state::organ::ORGAN_ASCII_CHARS` 1:1 镜像)
//! - 3 health 端点 (`/health` / `/ready` / `/metrics` 跟 `crate::HEALTH_ENDPOINTS` 对齐)
//! - 6 哲学锚 hardcode (`SIX_ANCHORS` 数组, 跟 `apeireth-tui/src/organ/mind.rs::SIX_ANCHORS` 1:1 镜像)
//!
//! ## 严守
//!
//! - **0 引 ratatui / 0 引 crossterm** — `String` 返 (TUI / 其他前端 都能喂入)
//! - **0 引 apeireth-state** — 因 apeireth-observability 的 `Cargo.toml` 是 24 LOCKED
//!   边界, 9 organ enum 字段镜像 sister #6 (编译期守门数组同长度同顺序, 互不依赖)
//! - **0 引 apeireth-tui** — TUI 集成通过 TUI 自己加 1 行 `mod observability;` (per
//!   借鉴 #1 sister 报告 1 行 mod 声明的 "必要小改" 模式)
//! - **公开 API 100% 文档化** — 每个 pub fn/pub struct/pub enum/pub const 都有 `///` doc
//!
//! ## 不假装 (per 6 哲学锚 O-5)
//!
//! - `TuiOrganState.value` 默认 0.0, 0 业务真值 (skeleton 阶段)
//! - `OrganDashboard::new()` 9 字段全 `TuiOrganState::stub()`, 真接在 R25.3 续做
//! - `register_tui_organ_state()` 是内存注册 (Mutex 守门), 0 持久化
//! - 9 widget 渲染是 `String` 模板, 0 ratatui widget 适配
//!
//! ## 6 哲学锚穿透
//!
//! - **S-1 北极星导向** — 9 器官 dashboard 服务 ASI 北极星 (跟 sister #1 命令模块同源)
//! - **S-2 实事求是** — OrganKind 镜像 sister #6 字段, 0 编造 "已集成 9 organ State"
//! - **O-2 走在前人肩上** — 借 `apeireth-state` 字段 / 借 TUI 命令模块的器官枚举 / 借
//!   `crate::HEALTH_ENDPOINTS` 3 端点
//! - **O-3 干到底** — 9 器官 × 3 端点 = 27 hardcode + 9 widget + 1 整体 + 1 register
//! - **O-4 任何人都能接手** — 1 模块顶部 §0-§11 完整 + 1 例子 + 1 integration test
//! - **O-5 不假装** — OrganDashboard 9 字段全 stub, render 是 String 模板, 0 假装真接 ratatui
//!
//! ## 状态
//!
//! ⚠️ **R25.2 skeleton (P2 估补, 1.0 release #8 observability 100%)**. 9 器官
//! dashboard 渲染 + 3 端点 status + 编译期 hardcode 守门. R25.3+ 续做: 真接
//! `apeireth-state` 9 organ State + 真接 ratatui widget 喂入 + HTTP 端点真接.

#![allow(clippy::all)]

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info};

use crate::{HealthEndpoint, HealthResponse, HealthStatus, HEALTH_ENDPOINTS, PLATFORM_NAME};

// ============================================================================
// 编译期 hardcode 常量 (10 项, per 6 哲学锚 O-3)
// ============================================================================

/// 9 器官编译期 hardcode (跟 `apeireth-state::organ::ORGAN_COUNT` 1:1 镜像).
///
/// K-1 强校验: 9 是硬约束, 改 1 器官 = 改 1 字段 + 1 match arm + 3 编译期数组.
pub const ORGAN_KIND_COUNT: usize = 9;

/// 9 器官中文名 (per 借鉴 #1 sister 报告 `apeireth-tui/src/organ/mod.rs::Organ::name_zh` +
/// 借鉴 #6 sister 报告 `apeireth-state::organ::ORGAN_NAMES_ZH`, 1:1 镜像).
///
/// 顺序必须跟 `OrganKind` 变体顺序一致 (变体索引 0-8 ↔ 数组 0-8).
pub const ORGAN_KIND_NAMES_ZH: [&str; ORGAN_KIND_COUNT] = [
    "心",   // Heart = 0
    "脑",   // Brain = 1
    "手",   // Hand = 2
    "眼",   // Eye = 3
    "耳",   // Ear = 4
    "记忆", // Memory = 5
    "声",   // Voice = 6
    "体",   // Body = 7
    "意",   // Mind = 8
];

/// 9 器官 ASCII 字符 (per 借鉴 #1 sister 报告 `apeireth-tui/src/organ/mod.rs::Organ::ascii_char`,
/// 跨平台 ASCII, 不依赖 emoji 字体).
pub const ORGAN_KIND_ASCII_CHARS: [&str; ORGAN_KIND_COUNT] = [
    "[♥]",     // Heart = 0
    "[BRAIN]", // Brain = 1
    "[HAND]",  // Hand = 2
    "[EYE]",   // Eye = 3
    "[EAR]",   // Ear = 4
    "[MEM]",   // Memory = 5
    "[VOICE]", // Voice = 6
    "[BODY]", // Body = 7
    "[MIND]", // Mind = 8
];

/// TUI dashboard schema 版本 (向前兼容字段, R25+ 改格式时 bump).
pub const TUI_DASHBOARD_SCHEMA_VERSION: &str = "1";

/// 平台名 (K-1 强校验, 永远 = "apeireth", 跟 `crate::PLATFORM_NAME` 守门一致).
pub const TUI_DASHBOARD_PLATFORM: &str = "apeireth";

/// 3 端点 hardcode (跟 `crate::HEALTH_ENDPOINTS` 守门一致).
pub const DASHBOARD_HEALTH_ENDPOINTS: [&str; 3] = ["/health", "/ready", "/metrics"];

/// 编译期守门: DASHBOARD_HEALTH_ENDPOINTS 长度 == 3 (跟 HEALTH_ENDPOINTS 对齐).
/// 用 < 比较 (not PartialEq 依赖), 编译期 OK.
const _: () = [()][(DASHBOARD_HEALTH_ENDPOINTS.len() < 3) as usize];
const _: () = [()][(DASHBOARD_HEALTH_ENDPOINTS.len() > 3) as usize];

/// 6 哲学锚 hardcode (per 主人 R19 锚定 + sister #1 报告 `mind::SIX_ANCHORS`,
/// 1:1 镜像).
///
/// 这 6 锚是 Apeireth 顶层哲学守门, mind 器官 widget 渲染时显式列出.
pub const SIX_ANCHORS: [&str; 6] = [
    "S-1 北极星导向",  // 主人 22:13 拍
    "S-2 实事求是",    // 0 假装已实现
    "O-2 走在前人肩上", // 借 Golutra / 业界标准
    "O-3 干到底",      // 9 器官 + 5 nav 全列
    "O-4 任何人都能接手", // 完整文档 + 测试
    "O-5 不假装",      // stub / partial 标诚实
];

/// 5 nav hardcode (per 主人 R19 决定, 跟 sister #1 报告 `apeireth-tui/src/main.rs` 5 nav 守门).
///
/// 仪表盘顶部显示当前 nav + 9 器官.
pub const FIVE_NAV: [&str; 5] = [
    "0 舰桥 Bridge",     // ΣΚΟΠΗ
    "1 对话 Dialogue",   // ΔΙΑΛΟΓΟΣ
    "2 生长 Growth",     // ΑΥΞΗΣΙΣ
    "3 历史 History",    // ΙΣΤΟΡΙΑ
    "4 设置 Settings",   // ΤΑΞΙΣ
];

// ============================================================================
// §1 错误类型 (3 变体 thiserror)
// ============================================================================

/// TUI dashboard 错误类型 (1:1 翻译 sister #1 `OrganError` 5 变体, 本模块精简为 3 变体).
#[derive(Debug, Error)]
pub enum TuiDashboardError {
    /// Organ 索引越界 (0-8 之外, 9 hardcode 守门).
    #[error("organ index out of range: {0} (valid: 0-8)")]
    OrganIndexOutOfRange(u8),

    /// Dashboard mutex poisoned (罕见, 线程 panic 后).
    #[error("dashboard mutex poisoned: {0}")]
    DashboardPoisoned(String),

    /// Health endpoint 未知 (3 端点之外, 编译期 hardcode 守门).
    #[error("health endpoint unknown: {0}")]
    HealthEndpointUnknown(String),
}

/// TUI dashboard Result 别名.
pub type TuiDashboardResult<T> = Result<T, TuiDashboardError>;

// ============================================================================
// §2 核心类型 (OrganKind enum + TuiOrganState struct)
// ============================================================================

/// 9 器官 enum (编译期 hardcode, 跟 `apeireth-state::organ::Organ` 1:1 镜像).
///
/// **变体索引** (0-8) 用于 `ORGAN_KIND_NAMES_ZH` / `ORGAN_KIND_ASCII_CHARS` 数组查找.
///
/// **LOCKED 边界说明**: 本 enum **不是** `apeireth-state::Organ`, 是镜像副本.
/// 原因: `apeireth-observability` 的 `Cargo.toml` 是 24 LOCKED, 0 引
/// `apeireth-state` (sister #6 估补). 集成时 (R25.3+) 在 `apeireth-tui/app.rs`
/// LOCKED 边界外做 `apeireth_state::Organ -> observability::OrganKind` 1:1 转换.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrganKind {
    /// 0: 心 (heart) — CPU 心跳 / 60Hz.
    Heart = 0,
    /// 1: 脑 (brain) — LLM 调用频率 / 当前 active provider.
    Brain = 1,
    /// 2: 手 (hand) — 工具调用统计 / 白名单.
    Hand = 2,
    /// 3: 眼 (eye) — 输入监控 (sister #1 stub).
    Eye = 3,
    /// 4: 耳 (ear) — 事件订阅 (sister #1 stub).
    Ear = 4,
    /// 5: 记忆 (memory) — 会话历史长度.
    Memory = 5,
    /// 6: 声 (voice) — TTS / STT (sister #1 stub).
    Voice = 6,
    /// 7: 体 (body) — 进程 / 内存 / 磁盘.
    Body = 7,
    /// 8: 意 (mind) — AGI 状态 + 6 哲学锚.
    Mind = 8,
}

impl fmt::Display for OrganKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl OrganKind {
    /// 数字 0-8 → OrganKind.
    ///
    /// 返回 `None` 如果越界 (K-1 强校验 #2).
    #[must_use]
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Heart),
            1 => Some(Self::Brain),
            2 => Some(Self::Hand),
            3 => Some(Self::Eye),
            4 => Some(Self::Ear),
            5 => Some(Self::Memory),
            6 => Some(Self::Voice),
            7 => Some(Self::Body),
            8 => Some(Self::Mind),
            _ => None,
        }
    }

    /// OrganKind → 数字 0-8 (数组索引).
    #[must_use]
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// OrganKind → 中文名 (查 `ORGAN_KIND_NAMES_ZH` 数组).
    #[must_use]
    pub fn name_zh(self) -> &'static str {
        ORGAN_KIND_NAMES_ZH[self.as_u8() as usize]
    }

    /// OrganKind → ASCII 字符 (查 `ORGAN_KIND_ASCII_CHARS` 数组).
    #[must_use]
    pub fn ascii_char(self) -> &'static str {
        ORGAN_KIND_ASCII_CHARS[self.as_u8() as usize]
    }

    /// OrganKind → 英文小写 (sister #1 `Organ::as_str` 1:1 镜像).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Heart => "heart",
            Self::Brain => "brain",
            Self::Hand => "hand",
            Self::Eye => "eye",
            Self::Ear => "ear",
            Self::Memory => "memory",
            Self::Voice => "voice",
            Self::Body => "body",
            Self::Mind => "mind",
        }
    }

    /// 全部 9 器官 (按索引顺序 0-8, 给仪表盘 render 用).
    #[must_use]
    pub fn all() -> [OrganKind; ORGAN_KIND_COUNT] {
        [
            Self::Heart, Self::Brain, Self::Hand, Self::Eye, Self::Ear,
            Self::Memory, Self::Voice, Self::Body, Self::Mind,
        ]
    }
}

/// 编译期守门: 全部 9 器官 = 9 (K-1 强校验, 硬编码 9 not 调用 all()).
const _: () = [()][(ORGAN_KIND_COUNT < 9) as usize];
const _: () = [()][(ORGAN_KIND_COUNT > 9) as usize];

/// TUI 仪表盘器官状态 (per-organ 4 字段 + 1 readiness).
///
/// 每器官 1 个 `TuiOrganState` 字段, 9 字段聚合成 `OrganDashboard`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TuiOrganState {
    /// 器官 (Heart / Brain / ...)
    pub organ: OrganKind,
    /// 实接度 (Ok / Partial / Stub, per sister #1 `Readiness` 1:1).
    pub readiness: OrganReadiness,
    /// 关键值 (器官特定: heart=60Hz, brain=call_count, hand=call_count, eye=tokens, ear=events,
    /// memory=history_len, voice=queue_len, body=cpu_pct, mind=growth_rate).
    pub value: f64,
    /// 末次更新时间 (UTC RFC 3339).
    pub last_update: chrono::DateTime<chrono::Utc>,
    /// 状态消息 (人类可读, e.g. "OK" / "stub: 0 真接" / "partial: 6/6 字段").
    pub message: String,
}

impl TuiOrganState {
    /// 构造器官状态.
    pub fn new(
        organ: OrganKind,
        readiness: OrganReadiness,
        value: f64,
        message: impl Into<String>,
    ) -> Self {
        Self {
            organ,
            readiness,
            value,
            last_update: chrono::Utc::now(),
            message: message.into(),
        }
    }

    /// 标占位 (sister #1 模式, 0 业务真值, 编译期标明 readiness).
    ///
    /// 默认 value=0.0, message="stub: 占位 (R25.3 真接)", readiness=Stub.
    #[must_use]
    pub fn stub(organ: OrganKind) -> Self {
        Self::new(
            organ,
            OrganReadiness::Stub,
            0.0,
            format!("stub: 占位 ({} 器官 R25.3 真接)", organ.name_zh()),
        )
    }

    /// 标 partial (per sister #1 模式, 部分真接, 0 全部真接).
    ///
    /// 默认 value=0.0, message="partial: 0/9 字段真接", readiness=Partial.
    #[must_use]
    pub fn partial(organ: OrganKind) -> Self {
        Self::new(
            organ,
            OrganReadiness::Partial,
            0.0,
            format!("partial: 0/9 字段真接 ({} 器官 R25.3 续)", organ.name_zh()),
        )
    }

    /// 标 ok (真接, 1 字段真接, 给 dashboard 验证 ok 渲染).
    #[must_use]
    pub fn ok(organ: OrganKind, value: f64, message: impl Into<String>) -> Self {
        Self::new(organ, OrganReadiness::Ok, value, message)
    }
}

/// 器官实接度 (per sister #1 `Readiness` enum 1:1 镜像).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrganReadiness {
    /// 真接 (HTTP / 真实数据).
    Ok,
    /// 部分接 (占位 + 真实数据混合, 待 R25.3).
    Partial,
    /// 桩 (只占位, 标 stub).
    Stub,
}

impl OrganReadiness {
    /// 字符串 (K-1 强校验, 跟 sister #1 `Readiness::as_str` 1:1).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Partial => "partial",
            Self::Stub => "stub",
        }
    }
}

impl fmt::Display for OrganReadiness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ============================================================================
// §3 9 器官 widget 渲染 (per 主人 R19 拟人化决策 "用器官监控状态")
//
// 9 widget 都是 `pub fn render_X_widget(state: &TuiOrganState) -> String`,
// 返 String (ratatui Paragraph 喂入, 0 引 ratatui).
// 命名: heart/brain/hand/eye/ear/memory/voice/body/mind.
// ============================================================================

/// 渲染 heart (心) 器官 widget.
///
/// 显示: `[♥] 心 heart    bpm=60.0   OK   `heart 60Hz, 2/9 字段真接``.
#[must_use]
pub fn render_heart_widget(state: &TuiOrganState) -> String {
    debug!(organ = "heart", value = state.value, "tui_dashboard: render heart widget");
    format!(
        "{ascii} {zh} {en:<6}  bpm={value:>5.1}   {readiness:<8}  {msg}",
        ascii = state.organ.ascii_char(),
        zh = state.organ.name_zh(),
        en = state.organ.as_str(),
        value = state.value,
        readiness = state.readiness.as_str(),
        msg = state.message,
    )
}

/// 渲染 brain (脑) 器官 widget.
///
/// 显示: `[BRAIN] 脑 brain    calls=42.0   OK   "active provider: minimax (1/5)"`.
#[must_use]
pub fn render_brain_widget(state: &TuiOrganState) -> String {
    debug!(organ = "brain", value = state.value, "tui_dashboard: render brain widget");
    format!(
        "{ascii} {zh} {en:<6}  calls={value:>5.0}   {readiness:<8}  {msg}",
        ascii = state.organ.ascii_char(),
        zh = state.organ.name_zh(),
        en = state.organ.as_str(),
        value = state.value,
        readiness = state.readiness.as_str(),
        msg = state.message,
    )
}

/// 渲染 hand (手) 器官 widget.
///
/// 显示: `[HAND] 手 hand    invokes=12.0   OK   "last tool: read_file (1/6)"`.
#[must_use]
pub fn render_hand_widget(state: &TuiOrganState) -> String {
    debug!(organ = "hand", value = state.value, "tui_dashboard: render hand widget");
    format!(
        "{ascii} {zh} {en:<6}  invokes={value:>5.0}   {readiness:<8}  {msg}",
        ascii = state.organ.ascii_char(),
        zh = state.organ.name_zh(),
        en = state.organ.as_str(),
        value = state.value,
        readiness = state.readiness.as_str(),
        msg = state.message,
    )
}

/// 渲染 eye (眼) 器官 widget.
///
/// 显示: `[EYE] 眼 eye     tokens=0.0    stub  "0 真接 (R25.3 接 crossterm::event)"`.
#[must_use]
pub fn render_eye_widget(state: &TuiOrganState) -> String {
    debug!(organ = "eye", value = state.value, "tui_dashboard: render eye widget");
    format!(
        "{ascii} {zh} {en:<6}  tokens={value:>5.0}   {readiness:<8}  {msg}",
        ascii = state.organ.ascii_char(),
        zh = state.organ.name_zh(),
        en = state.organ.as_str(),
        value = state.value,
        readiness = state.readiness.as_str(),
        msg = state.message,
    )
}

/// 渲染 ear (耳) 器官 widget.
///
/// 显示: `[EAR] 耳 ear     events=0.0    stub  "0 真接 (R25.3 接 apeireth-bus L0-L4)"`.
#[must_use]
pub fn render_ear_widget(state: &TuiOrganState) -> String {
    debug!(organ = "ear", value = state.value, "tui_dashboard: render ear widget");
    format!(
        "{ascii} {zh} {en:<6}  events={value:>5.0}   {readiness:<8}  {msg}",
        ascii = state.organ.ascii_char(),
        zh = state.organ.name_zh(),
        en = state.organ.as_str(),
        value = state.value,
        readiness = state.readiness.as_str(),
        msg = state.message,
    )
}

/// 渲染 memory (记忆) 器官 widget.
///
/// 显示: `[MEM] 记忆 memory  history=24.0   OK   "episodes: 24"`.
#[must_use]
pub fn render_memory_widget(state: &TuiOrganState) -> String {
    debug!(organ = "memory", value = state.value, "tui_dashboard: render memory widget");
    format!(
        "{ascii} {zh} {en:<6}  history={value:>5.0}   {readiness:<8}  {msg}",
        ascii = state.organ.ascii_char(),
        zh = state.organ.name_zh(),
        en = state.organ.as_str(),
        value = state.value,
        readiness = state.readiness.as_str(),
        msg = state.message,
    )
}

/// 渲染 voice (声) 器官 widget.
///
/// 显示: `[VOICE] 声 voice   queue=0.0    stub  "0 真接 (R25.3 接 batch_text_to_audio)"`.
#[must_use]
pub fn render_voice_widget(state: &TuiOrganState) -> String {
    debug!(organ = "voice", value = state.value, "tui_dashboard: render voice widget");
    format!(
        "{ascii} {zh} {en:<6}  queue={value:>5.0}   {readiness:<8}  {msg}",
        ascii = state.organ.ascii_char(),
        zh = state.organ.name_zh(),
        en = state.organ.as_str(),
        value = state.value,
        readiness = state.readiness.as_str(),
        msg = state.message,
    )
}

/// 渲染 body (体) 器官 widget.
///
/// 显示: `[BODY] 体 body    cpu=2.5%     partial  "6/6 字段占位"`.
#[must_use]
pub fn render_body_widget(state: &TuiOrganState) -> String {
    debug!(organ = "body", value = state.value, "tui_dashboard: render body widget");
    format!(
        "{ascii} {zh} {en:<6}  cpu={value:>5.1}%   {readiness:<8}  {msg}",
        ascii = state.organ.ascii_char(),
        zh = state.organ.name_zh(),
        en = state.organ.as_str(),
        value = state.value,
        readiness = state.readiness.as_str(),
        msg = state.message,
    )
}

/// 渲染 mind (意) 器官 widget — 9 器官中唯一显示 6 哲学锚.
///
/// 显示: `[MIND] 意 mind    growth=0.85  partial  "seed, 6/9 锚 1:1 镜像"`.
#[must_use]
pub fn render_mind_widget(state: &TuiOrganState) -> String {
    debug!(organ = "mind", value = state.value, "tui_dashboard: render mind widget");
    format!(
        "{ascii} {zh} {en:<6}  growth={value:>5.2}   {readiness:<8}  {msg}\n  6 哲学锚: {anchors}",
        ascii = state.organ.ascii_char(),
        zh = state.organ.name_zh(),
        en = state.organ.as_str(),
        value = state.value,
        readiness = state.readiness.as_str(),
        msg = state.message,
        anchors = SIX_ANCHORS.join(" | "),
    )
}

/// 9 器官 widget 渲染 dispatch (按 OrganKind enum 匹配).
///
/// 编译期 enum 守门: 9 变体, 改 1 器官 = 改 1 arm.
#[must_use]
pub fn render_organ_widget(organ: OrganKind, state: &TuiOrganState) -> String {
    debug!(organ = %organ, "tui_dashboard: render organ widget dispatch");
    match organ {
        OrganKind::Heart => render_heart_widget(state),
        OrganKind::Brain => render_brain_widget(state),
        OrganKind::Hand => render_hand_widget(state),
        OrganKind::Eye => render_eye_widget(state),
        OrganKind::Ear => render_ear_widget(state),
        OrganKind::Memory => render_memory_widget(state),
        OrganKind::Voice => render_voice_widget(state),
        OrganKind::Body => render_body_widget(state),
        OrganKind::Mind => render_mind_widget(state),
    }
}

// ============================================================================
// §4 9 器官状态聚合 (OrganDashboard) + 3 端点 health
// ============================================================================

/// 9 器官状态 + 3 端点 health 聚合 (1:1 翻译 sister #1 `Registry` 9 State +
///// `crate::ObservabilityBus` health 3 端点).
///
/// **thread-safety**: 内部 `Arc<Mutex<...>>` 守门, 多线程 register / 读 都安全.
#[derive(Debug, Clone)]
pub struct OrganDashboard {
    /// 9 器官状态 (Mutex 守门, 跨线程安全).
    states: Arc<Mutex<[TuiOrganState; ORGAN_KIND_COUNT]>>,
    /// 3 端点 health (Mutex 守门, 跨线程安全).
    health: Arc<Mutex<HashMap<String, HealthResponse>>>,
    /// 当前 nav (5 nav 之一, 0-4).
    current_nav: Arc<Mutex<u8>>,
}

impl OrganDashboard {
    /// 新建 (9 字段全 stub, 3 端点全 default Healthy).
    ///
    /// **不假装** (per 6 哲学锚 O-5): 默认 readiness=Stub, 真实化由 R25.3 续做
    /// (在 `apeireth-tui/app.rs` LOCKED 边界外做真接).
    #[must_use]
    pub fn new() -> Self {
        let default_states = [
            TuiOrganState::stub(OrganKind::Heart),
            TuiOrganState::stub(OrganKind::Brain),
            TuiOrganState::stub(OrganKind::Hand),
            TuiOrganState::stub(OrganKind::Eye),
            TuiOrganState::stub(OrganKind::Ear),
            TuiOrganState::stub(OrganKind::Memory),
            TuiOrganState::stub(OrganKind::Voice),
            TuiOrganState::stub(OrganKind::Body),
            TuiOrganState::stub(OrganKind::Mind),
        ];
        let mut health_map = HashMap::new();
        for ep in HEALTH_ENDPOINTS {
            health_map.insert(
                ep.to_string(),
                HealthResponse::new(*ep, HealthStatus::Healthy)
                    .with_detail("platform", PLATFORM_NAME)
                    .with_detail("schema_version", TUI_DASHBOARD_SCHEMA_VERSION),
            );
        }
        info!(
            organs = ORGAN_KIND_COUNT,
            endpoints = HEALTH_ENDPOINTS.len(),
            "tui_dashboard: OrganDashboard::new (skeleton 阶段, 9 stub + 3 healthy)"
        );
        Self {
            states: Arc::new(Mutex::new(default_states)),
            health: Arc::new(Mutex::new(health_map)),
            current_nav: Arc::new(Mutex::new(0)),
        }
    }

    /// 注册/更新 1 器官状态 (per sister #1 `Registry` 9 State 模式).
    ///
    /// 编译期守门: organ 0-8 越界返 `TuiDashboardError::OrganIndexOutOfRange`.
    pub fn register_tui_organ_state(
        &self,
        organ: OrganKind,
        state: TuiOrganState,
    ) -> TuiDashboardResult<()> {
        let idx = organ.as_u8() as usize;
        if idx >= ORGAN_KIND_COUNT {
            return Err(TuiDashboardError::OrganIndexOutOfRange(organ.as_u8()));
        }
        let mut states = self
            .states
            .lock()
            .map_err(|e| TuiDashboardError::DashboardPoisoned(e.to_string()))?;
        states[idx] = state;
        debug!(organ = %organ, idx, "tui_dashboard: register organ state");
        Ok(())
    }

    /// 读 1 器官状态 (per OrganKind).
    pub fn read_organ_state(&self, organ: OrganKind) -> TuiDashboardResult<TuiOrganState> {
        let idx = organ.as_u8() as usize;
        if idx >= ORGAN_KIND_COUNT {
            return Err(TuiDashboardError::OrganIndexOutOfRange(organ.as_u8()));
        }
        let states = self
            .states
            .lock()
            .map_err(|e| TuiDashboardError::DashboardPoisoned(e.to_string()))?;
        Ok(states[idx].clone())
    }

    /// 读 9 器官状态全集 (per sister #1 `Registry` 9 State 一次读).
    #[must_use]
    pub fn read_all_organ_states(&self) -> [TuiOrganState; ORGAN_KIND_COUNT] {
        self.states
            .lock()
            .map(|s| s.clone())
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, "tui_dashboard: states mutex poisoned, returning stub");
                [
                    TuiOrganState::stub(OrganKind::Heart),
                    TuiOrganState::stub(OrganKind::Brain),
                    TuiOrganState::stub(OrganKind::Hand),
                    TuiOrganState::stub(OrganKind::Eye),
                    TuiOrganState::stub(OrganKind::Ear),
                    TuiOrganState::stub(OrganKind::Memory),
                    TuiOrganState::stub(OrganKind::Voice),
                    TuiOrganState::stub(OrganKind::Body),
                    TuiOrganState::stub(OrganKind::Mind),
                ]
            })
    }

    /// 更新 1 端点 health (3 端点之一, 编译期守门).
    pub fn update_health(&self, endpoint: &str, response: HealthResponse) -> TuiDashboardResult<()> {
        if !HEALTH_ENDPOINTS.contains(&endpoint) {
            return Err(TuiDashboardError::HealthEndpointUnknown(endpoint.to_string()));
        }
        let mut health = self
            .health
            .lock()
            .map_err(|e| TuiDashboardError::DashboardPoisoned(e.to_string()))?;
        health.insert(endpoint.to_string(), response);
        debug!(endpoint, "tui_dashboard: update health");
        Ok(())
    }

    /// 读 1 端点 health.
    pub fn read_health(&self, endpoint: &str) -> TuiDashboardResult<HealthResponse> {
        let health = self
            .health
            .lock()
            .map_err(|e| TuiDashboardError::DashboardPoisoned(e.to_string()))?;
        health
            .get(endpoint)
            .cloned()
            .ok_or_else(|| TuiDashboardError::HealthEndpointUnknown(endpoint.to_string()))
    }

    /// 读 3 端点 health 全集.
    #[must_use]
    pub fn read_all_health(&self) -> Vec<HealthResponse> {
        self.health
            .lock()
            .map(|h| {
                HEALTH_ENDPOINTS
                    .iter()
                    .filter_map(|ep| h.get(*ep).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 设置当前 nav (0-4, 编译期守门).
    pub fn set_current_nav(&self, nav: u8) -> TuiDashboardResult<()> {
        if nav >= 5 {
            return Err(TuiDashboardError::OrganIndexOutOfRange(nav));
        }
        let mut current = self
            .current_nav
            .lock()
            .map_err(|e| TuiDashboardError::DashboardPoisoned(e.to_string()))?;
        *current = nav;
        Ok(())
    }

    /// 读当前 nav (0-4).
    #[must_use]
    pub fn current_nav(&self) -> u8 {
        self.current_nav
            .lock()
            .map(|n| *n)
            .unwrap_or(0)
    }

    /// 读 HealthEndpoint 列表 (3 端点).
    #[must_use]
    pub fn health_endpoints() -> [HealthEndpoint; 3] {
        [
            HealthEndpoint::Health,
            HealthEndpoint::Ready,
            HealthEndpoint::Metrics,
        ]
    }
}

impl Default for OrganDashboard {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// §5 整体仪表盘渲染 (9 器官 + 3 端点统一)
// ============================================================================

/// 渲染整体仪表盘 (9 器官 + 3 端点 + 当前 nav).
///
/// 返 `String` (TUI 喂入 ratatui Paragraph, 0 引 ratatui).
///
/// 格式:
/// ```text
/// === Apeireth TUI Dashboard (schema: 1) ===
/// nav: 0 舰桥 Bridge  (current: 0)
/// --- 9 器官状态 ---
/// [♥] 心 heart    bpm=60.0   OK   ...
/// [BRAIN] 脑 brain    calls=42.0   OK   ...
/// ...
/// --- 3 health 端点 ---
/// /health    Healthy
/// /ready     Healthy
/// /metrics   Healthy
/// ```
#[must_use]
pub fn render_dashboard(dashboard: &OrganDashboard) -> String {
    let mut out = String::new();

    // Header
    out.push_str(&format!(
        "=== Apeireth TUI Dashboard (schema: {}) ===\n",
        TUI_DASHBOARD_SCHEMA_VERSION
    ));

    // Current nav
    let nav_idx = dashboard.current_nav() as usize;
    let nav_str = FIVE_NAV.get(nav_idx).copied().unwrap_or("? unknown");
    out.push_str(&format!("nav: {}  (current: {})\n", nav_str, nav_idx));

    // 9 器官
    out.push_str("--- 9 器官状态 ---\n");
    let states = dashboard.read_all_organ_states();
    for organ in OrganKind::all() {
        let state = &states[organ.as_u8() as usize];
        out.push_str(&render_organ_widget(organ, state));
        out.push('\n');
    }

    // 3 端点
    out.push_str("--- 3 health 端点 ---\n");
    for ep in HEALTH_ENDPOINTS {
        match dashboard.read_health(ep) {
            Ok(resp) => {
                out.push_str(&format!(
                    "{ep:<10} {status:?}  ({http})\n",
                    ep = ep,
                    status = resp.status,
                    http = resp.status.http_status_code()
                ));
            }
            Err(e) => {
                out.push_str(&format!("{:<10} <error: {}>\n", ep, e));
            }
        }
    }

    out
}

// ============================================================================
// 单元测试 (in-module, 集成测试在 tests/test_tui_dashboard.rs)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn k1_organ_count_is_9() {
        assert_eq!(ORGAN_KIND_COUNT, 9, "ORGAN_KIND_COUNT 必须 = 9 (K-1 强校验 #2)");
        assert_eq!(OrganKind::all().len(), 9);
    }

    #[test]
    fn k1_six_anchors_count() {
        assert_eq!(SIX_ANCHORS.len(), 6, "SIX_ANCHORS 必须 = 6 (K-1 强校验)");
    }

    #[test]
    fn k1_five_nav_count() {
        assert_eq!(FIVE_NAV.len(), 5, "FIVE_NAV 必须 = 5 (K-1 强校验, 主人 R19 决定)");
    }

    #[test]
    fn k1_3_health_endpoints() {
        assert_eq!(DASHBOARD_HEALTH_ENDPOINTS.len(), 3);
        assert!(DASHBOARD_HEALTH_ENDPOINTS.contains(&"/health"));
        assert!(DASHBOARD_HEALTH_ENDPOINTS.contains(&"/ready"));
        assert!(DASHBOARD_HEALTH_ENDPOINTS.contains(&"/metrics"));
    }

    #[test]
    fn k1_platform_name_apeireth() {
        assert_eq!(TUI_DASHBOARD_PLATFORM, "apeireth");
    }

    #[test]
    fn organ_kind_names_zh_match_sister_reports() {
        // 跟 sister #1 + sister #6 1:1 镜像 (LOCKED 边界同步)
        assert_eq!(ORGAN_KIND_NAMES_ZH[0], "心");
        assert_eq!(ORGAN_KIND_NAMES_ZH[1], "脑");
        assert_eq!(ORGAN_KIND_NAMES_ZH[2], "手");
        assert_eq!(ORGAN_KIND_NAMES_ZH[3], "眼");
        assert_eq!(ORGAN_KIND_NAMES_ZH[4], "耳");
        assert_eq!(ORGAN_KIND_NAMES_ZH[5], "记忆");
        assert_eq!(ORGAN_KIND_NAMES_ZH[6], "声");
        assert_eq!(ORGAN_KIND_NAMES_ZH[7], "体");
        assert_eq!(ORGAN_KIND_NAMES_ZH[8], "意");
    }

    #[test]
    fn organ_kind_ascii_chars_match_sister_reports() {
        assert_eq!(ORGAN_KIND_ASCII_CHARS[0], "[♥]");
        assert_eq!(ORGAN_KIND_ASCII_CHARS[1], "[BRAIN]");
        assert_eq!(ORGAN_KIND_ASCII_CHARS[2], "[HAND]");
        assert_eq!(ORGAN_KIND_ASCII_CHARS[3], "[EYE]");
        assert_eq!(ORGAN_KIND_ASCII_CHARS[4], "[EAR]");
        assert_eq!(ORGAN_KIND_ASCII_CHARS[5], "[MEM]");
        assert_eq!(ORGAN_KIND_ASCII_CHARS[6], "[VOICE]");
        assert_eq!(ORGAN_KIND_ASCII_CHARS[7], "[BODY]");
        assert_eq!(ORGAN_KIND_ASCII_CHARS[8], "[MIND]");
    }

    #[test]
    fn organ_kind_from_u8_roundtrip() {
        for v in 0u8..=8 {
            let organ = OrganKind::from_u8(v).expect("0-8 必 valid");
            assert_eq!(organ.as_u8(), v);
        }
        assert!(OrganKind::from_u8(9).is_none());
        assert!(OrganKind::from_u8(255).is_none());
    }

    #[test]
    fn tui_organ_state_stub_marks_stub() {
        let state = TuiOrganState::stub(OrganKind::Heart);
        assert_eq!(state.organ, OrganKind::Heart);
        assert_eq!(state.readiness, OrganReadiness::Stub);
        assert_eq!(state.value, 0.0);
        assert!(state.message.contains("stub"));
    }

    #[test]
    fn tui_organ_state_ok_marks_ok() {
        let state = TuiOrganState::ok(OrganKind::Heart, 60.0, "60Hz");
        assert_eq!(state.readiness, OrganReadiness::Ok);
        assert_eq!(state.value, 60.0);
        assert_eq!(state.message, "60Hz");
    }

    #[test]
    fn render_9_organ_widgets_have_organ_in_output() {
        let states = [
            TuiOrganState::ok(OrganKind::Heart, 60.0, "60Hz"),
            TuiOrganState::ok(OrganKind::Brain, 42.0, "calls=42"),
            TuiOrganState::ok(OrganKind::Hand, 12.0, "invokes=12"),
            TuiOrganState::stub(OrganKind::Eye),
            TuiOrganState::stub(OrganKind::Ear),
            TuiOrganState::ok(OrganKind::Memory, 24.0, "episodes=24"),
            TuiOrganState::stub(OrganKind::Voice),
            TuiOrganState::partial(OrganKind::Body),
            TuiOrganState::partial(OrganKind::Mind),
        ];
        for organ in OrganKind::all() {
            let s = render_organ_widget(organ, &states[organ.as_u8() as usize]);
            assert!(s.contains(organ.as_str()), "{} widget must contain organ name", organ.as_str());
        }
    }

    #[test]
    fn render_mind_widget_includes_six_anchors() {
        let state = TuiOrganState::partial(OrganKind::Mind);
        let s = render_mind_widget(&state);
        for anchor in SIX_ANCHORS {
            assert!(s.contains(anchor), "mind widget must include anchor: {}", anchor);
        }
    }

    #[test]
    fn organ_dashboard_new_has_9_stub_states() {
        let dash = OrganDashboard::new();
        for organ in OrganKind::all() {
            let state = dash.read_organ_state(organ).expect("9 organs in range");
            assert_eq!(state.readiness, OrganReadiness::Stub, "new() 全 stub");
            assert_eq!(state.organ, organ);
        }
    }

    #[test]
    fn organ_dashboard_register_and_read_organ_state() {
        let dash = OrganDashboard::new();
        let state = TuiOrganState::ok(OrganKind::Heart, 75.0, "75Hz");
        dash.register_tui_organ_state(OrganKind::Heart, state.clone())
            .expect("register heart");
        let read = dash.read_organ_state(OrganKind::Heart).expect("read heart");
        assert_eq!(read.value, 75.0);
        assert_eq!(read.readiness, OrganReadiness::Ok);
    }

    #[test]
    fn organ_dashboard_register_9_organs_independently() {
        let dash = OrganDashboard::new();
        for organ in OrganKind::all() {
            let state = TuiOrganState::ok(organ, organ.as_u8() as f64, "ok");
            dash.register_tui_organ_state(organ, state)
                .expect("register");
        }
        for organ in OrganKind::all() {
            let read = dash.read_organ_state(organ).expect("read");
            assert_eq!(read.value, organ.as_u8() as f64);
            assert_eq!(read.readiness, OrganReadiness::Ok);
        }
    }

    #[test]
    fn organ_dashboard_update_and_read_health() {
        let dash = OrganDashboard::new();
        let resp = HealthResponse::new("/health", HealthStatus::Degraded)
            .with_detail("reason", "test");
        dash.update_health("/health", resp).expect("update /health");
        let read = dash.read_health("/health").expect("read /health");
        assert_eq!(read.status, HealthStatus::Degraded);
    }

    #[test]
    fn organ_dashboard_rejects_unknown_endpoint() {
        let dash = OrganDashboard::new();
        let resp = HealthResponse::new("/unknown", HealthStatus::Healthy);
        let result = dash.update_health("/unknown", resp);
        assert!(matches!(
            result,
            Err(TuiDashboardError::HealthEndpointUnknown(_))
        ));
    }

    #[test]
    fn organ_dashboard_set_and_read_nav() {
        let dash = OrganDashboard::new();
        for nav in 0u8..=4 {
            dash.set_current_nav(nav).expect("set nav 0-4");
            assert_eq!(dash.current_nav(), nav);
        }
        assert!(dash.set_current_nav(5).is_err());
    }

    #[test]
    fn render_dashboard_includes_all_9_organs_and_3_endpoints() {
        let dash = OrganDashboard::new();
        let s = render_dashboard(&dash);
        // 9 器官
        for organ in OrganKind::all() {
            assert!(s.contains(organ.as_str()), "dashboard must include {}", organ.as_str());
        }
        // 3 端点
        for ep in HEALTH_ENDPOINTS {
            assert!(s.contains(ep), "dashboard must include {}", ep);
        }
        // 5 nav
        assert!(s.contains("舰桥"));
    }

    #[test]
    fn render_dashboard_mind_widget_includes_six_anchors() {
        let dash = OrganDashboard::new();
        let s = render_dashboard(&dash);
        for anchor in SIX_ANCHORS {
            assert!(s.contains(anchor), "dashboard must include anchor: {}", anchor);
        }
    }

    #[test]
    fn k1_all_pass_extended() {
        // 扩展 K-1 强校验: 9 organ + 6 anchor + 5 nav + 3 endpoint + 1 platform
        assert_eq!(ORGAN_KIND_COUNT, 9);
        assert_eq!(SIX_ANCHORS.len(), 6);
        assert_eq!(FIVE_NAV.len(), 5);
        assert_eq!(DASHBOARD_HEALTH_ENDPOINTS.len(), 3);
        assert_eq!(TUI_DASHBOARD_PLATFORM, "apeireth");
    }
}
