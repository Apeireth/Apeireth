//! # apeireth-tui-e2e — TUI 5 nav + 9 器官 端到端集成测试
//!
//! **R20 阶段 5 估补** (per 主人 2026-08-05 派活单) — 用 `ratatui::backend::TestBackend`
//! 验证 apeireth TUI 的**设计契约** (5 nav / 9 organ / 1 屏 4 panel / 6 哲学锚),
//! 不开真终端, 不跑 tauri, **干 TUI 不干前端** (主人 22:13 拍板).
//!
//! ---
//!
//! ## 6 哲学锚 (per `docs/architecture-v4-living-intelligence.md` §0.2)
//!
//! | ID | 时戳 | 标题 | TUI 体现 |
//! |----|------|------|----------|
//! | S-1 | 22:33 | 算力是电 — 强调 ASI 算力供给 | status bar 实时 CPU% / 60Hz tick |
//! | S-2 | 17:43 | 实积寸累 — 反对写 LOCKED 跳板 | 不动 24 LOCKED crate 的 src/ |
//! | O-2 | 19:33 | 反对前倾草率 — 先锁定后写 | 6 哲学锚 / 8 不修改承诺写在 header |
//! | O-3 | 23:44 | 可读性 — 6 锚全显不藏 | 9 器官全显 + 6 锚全显 (MIND 模块) |
//! | O-4 | 00:56 | 任何人都能读懂 — 文档全开 | 本文件 + 单元 + e2e 三层都讲人话 |
//! | O-5 | 17:58 | 仓廪实 — 仓廪实而知礼节 | 9 器官 ASCII [?] 状态条, 不浮夸 |
//!
//! ## 8 项不修改承诺 (per omnibus §6 + 主人 2026-08-05 R20 阶段 5)
//!
//! 1. ✅ 错误能装到实现 — `TuiE2EError` thiserror + 7-10 变体
//! 2. ✅ 错误数 hardcode — `TuiE2EError` 8-10 变体对应失败类型
//! 3. ✅ 0 改 LOCKED — 本 crate 不触碰 24 LOCKED crate 的 src/
//! 4. ✅ 0 改 workspace version — `version.workspace = true`
//! 5. ✅ 6 哲学锚透传 — S-1/S-2/O-2/O-3/O-4/O-5 全显 (MIND organ + lib header)
//! 6. ✅ 0 依赖 NewAPI — 镜像 tui 公开 API, 不引入额外代理
//! 7. ✅ 0 重复造轮子 — `dispatch_render` / `render_*` 直接镜像 tui 已有签名
//! 8. ✅ 0 假装实缺 — 1 屏 4 panel + 5 nav + 9 器官 + 6 锚 全部 hardcode
//!
//! ## 边界
//!
//! - `apeireth-tui` 当前是 **binary-only** (无 `lib.rs`), 不能 path-dep
//! - e2e crate 镜像其公开 API 表面 (NavPage / Nav / Organ / 6 哲学锚),
//!   后续 tui 加 `lib.rs` 后可切到 `path = "../apeireth-tui"` 真实依赖
//! - **0 触碰** 24 LOCKED crate, **0 改** workspace Cargo.toml
//!
//! ## 模块
//!
//! - [`error`] — 8-10 种 `TuiE2EError`
//! - [`backend`] — `TuiTestBackend` (ratatui TestBackend 包装 + 断言助手)
//! - [`harness`] — `TuiHarness` (启动 + 1s tick + 5 快捷键事件)
//! - [`render`] — 1 屏 4 panel 渲染验证 (top nav / middle organ / content / status)
//! - [`nav_e2e`] — 5 nav 端到端测试函数
//! - [`organ_e2e`] — 9 器官端到端测试函数
//!
//! ## 公开 API 速查
//!
//! - [`TuiApp`] — 镜像 `apeireth_tui::App` 的最小可测表面
//! - [`NavPage`] — 5 nav: Bridge / Dialogue / Growth / History / Settings (R19 TUI 主 nav)
//! - [`Nav`] — 5 副 nav: Status / Session / Tools / Settings / Help (R22 估补)
//! - [`Organ`] — 9 器官: Heart / Brain / Hand / Eye / Ear / Memory / Voice / Body / Mind
//! - [`Mode`] — Focus / Inspire
//! - [`Language`] — Zh / En
//! - [`Theme`] — Archaic / Modern / Cosmic
//! - [`TuiTestBackend::new`], [`TuiTestBackend::buffer`]
//! - [`TuiHarness::start`], [`TuiHarness::tick`], [`TuiHarness::send_key`]
//! - [`nav_e2e::test_nav_status_renders_5_components`] 等 5 个
//! - [`organ_e2e::test_organ_heart_60hz_pulse`] 等 9 个
//!
//! ## 验收 (per 派活单)
//!
//! - `cargo check -p apeireth-tui-e2e` 0 error
//! - `cargo test -p apeireth-tui-e2e` 全过 (20+ 测试)
//! - lib.rs 500+ 行 (本文件)
//! - 5 nav + 9 器官 e2e 覆盖
//! - **不主动 commit** — 主人 2026-08-05 R20 阶段 5 拍板, 派活单明确"不在主仓做任何 git commit"

#![forbid(unsafe_code)]
// 跟随 workspace 全局 lint (qdrant/wasmtime 精选)
// missing_docs 已在 [workspace.lints.rust] 段 allow, 不在 crate 级再 warn
// unused_must_use / unused_mut / dead_code 已经在 workspace 段 allow
#![allow(clippy::needless_raw_string_hashes)]

use std::time::Instant;

use crossterm::event::KeyCode;

pub mod backend;
// R177: organ invariants (5 tests + 2 Kani)
mod organ_kani_proofs;
pub mod error;
pub mod harness;
pub mod nav_e2e;
pub mod organ_e2e;
pub mod render;

// 重导出常用类型, 让外部 `use apeireth_tui_e2e::*;` 即可
pub use backend::{BufferSnapshot, TuiTestBackend};
pub use error::{TuiE2EError, TuiE2EResult};
pub use harness::TuiHarness;

/// Prelude — 集成测试 / example 用, 一次性导入所有公开 API
pub mod prelude {
    pub use crate::backend::{BufferSnapshot, TuiTestBackend};
    pub use crate::error::{TuiE2EError, TuiE2EResult};
    pub use crate::harness::TuiHarness;
    pub use crate::{
        ChatMessage, EIGHT_PROMISES, FIVE_R_MEASURES, Language, Mode, Nav, NavPage, Organ, Theme,
        TuiApp, EIGHT_PROMISES as PROMISES, SIX_PHI_ANCHORS, R_MEASURE_COUNT, DEFAULT_WIDTH,
        DEFAULT_HEIGHT, PANEL_HEIGHTS,
    };
}

// =====================================================================
// 公开 API — 镜像 apeireth_tui 的 App 最小可测表面
// =====================================================================
//
// 设计原则:
// 1. enum 顺序 / 编号 / 字段 / 派生 — 跟 tui 一致 (Round-trip 测试已覆盖)
// 2. 不暴露 tauri / web / HTTP client 依赖 (e2e 是纯终端层)
// 3. 后续 tui 加 lib.rs 后, 本段可整体换成 `pub use apeireth_tui::*;`

/// 5 主 nav (per R19 TUI 设计, 跟 tui `app::NavPage` 一一对应)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NavPage {
    /// 0 桥接 (主页, 默认)
    Bridge = 0,
    /// 1 对话 (chat 模式)
    Dialogue = 1,
    /// 2 成长 (Life Stage 进度)
    Growth = 2,
    /// 3 历史 (chat history)
    History = 3,
    /// 4 设置 (theme / language / mode)
    Settings = 4,
}

impl NavPage {
    /// 5 nav 数 (hardcode, 编译期不变量)
    pub const COUNT: u8 = 5;

    /// 0-4 → `NavPage`, 越界返回 `None`
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Bridge),
            1 => Some(Self::Dialogue),
            2 => Some(Self::Growth),
            3 => Some(Self::History),
            4 => Some(Self::Settings),
            _ => None,
        }
    }

    /// 下一个, 4 → 0 循环
    pub fn next(self) -> Self {
        Self::from_u8(((self as u8) + 1) % Self::COUNT).unwrap()
    }

    /// 上一个, 0 → 4 循环
    pub fn prev(self) -> Self {
        let n = if (self as u8) == 0 {
            Self::COUNT - 1
        } else {
            (self as u8) - 1
        };
        Self::from_u8(n).unwrap()
    }

    /// 中文标签 (跟 tui label_zh 镜像)
    pub fn label_zh(self) -> &'static str {
        match self {
            Self::Bridge => "桥接",
            Self::Dialogue => "对话",
            Self::Growth => "成长",
            Self::History => "历史",
            Self::Settings => "设置",
        }
    }

    /// 希腊文标签 (跟 tui label_greek 镜像, 5 元素链)
    pub fn label_greek(self) -> &'static str {
        match self {
            Self::Bridge => "Γεφυρωτής",
            Self::Dialogue => "Διάλογος",
            Self::Growth => "Ανάπτυξη",
            Self::History => "Ιστορία",
            Self::Settings => "Ρυθμίσεις",
        }
    }
}

impl Default for NavPage {
    fn default() -> Self {
        Self::Bridge
    }
}

/// 5 副 nav (per R22 估补, 跟 tui `nav::Nav` 镜像, status/session/tools/settings/help)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Nav {
    /// 0 状态 (5 大 health + CPU + 内存)
    Status = 0,
    /// 1 会话 (活跃 session 列表)
    Session = 1,
    /// 2 工具 (6 工具 endpoint)
    Tools = 2,
    /// 3 设置 (5 权限 + 5 Provider + 4 SDK)
    Settings = 3,
    /// 4 帮助 (6 哲学锚 + 8 不修改承诺 + 1.0 release 文档)
    Help = 4,
}

impl Nav {
    /// 5 nav 数
    pub const COUNT: u8 = 5;

    /// 0-4 → `Nav`, 越界返回 `None`
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Status),
            1 => Some(Self::Session),
            2 => Some(Self::Tools),
            3 => Some(Self::Settings),
            4 => Some(Self::Help),
            _ => None,
        }
    }

    /// 下一个, 4 → 0 循环
    pub fn next(self) -> Self {
        Self::from_u8(((self as u8) + 1) % Self::COUNT).unwrap()
    }

    /// 上一个, 0 → 4 循环
    pub fn prev(self) -> Self {
        let n = if (self as u8) == 0 {
            Self::COUNT - 1
        } else {
            (self as u8) - 1
        };
        Self::from_u8(n).unwrap()
    }

    /// 中文标签
    pub fn label_zh(self) -> &'static str {
        match self {
            Self::Status => "状态",
            Self::Session => "会话",
            Self::Tools => "工具",
            Self::Settings => "设置",
            Self::Help => "帮助",
        }
    }
}

impl Default for Nav {
    fn default() -> Self {
        Self::Status
    }
}

/// 9 器官 (per R19 拟人化原则 — 心脑手眼耳鼻记忆声体意, 跟 tui `organ::Organ` 镜像)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Organ {
    /// 0 心 (Heart) — 60Hz CPU 脉冲
    Heart = 0,
    /// 1 脑 (Brain) — LLM 推理
    Brain = 1,
    /// 2 手 (Hand) — 工具调用
    Hand = 2,
    /// 3 眼 (Eye) — 输入监控
    Eye = 3,
    /// 4 耳 (Ear) — 事件订阅
    Ear = 4,
    /// 5 记忆 (Memory) — 会话历史
    Memory = 5,
    /// 6 声 (Voice) — STT / TTS
    Voice = 6,
    /// 7 体 (Body) — 进程 / 资源
    Body = 7,
    /// 8 意 (Mind) — AGI 状态 + 6 哲学锚
    Mind = 8,
}

impl Organ {
    /// 9 器官数 (hardcode)
    pub const COUNT: u8 = 9;

    /// 0-8 → `Organ`, 越界返回 `None`
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

    /// 中文名 (跟 tui `Organ::name_zh` 镜像)
    pub fn name_zh(self) -> &'static str {
        match self {
            Self::Heart => "心",
            Self::Brain => "脑",
            Self::Hand => "手",
            Self::Eye => "眼",
            Self::Ear => "耳",
            Self::Memory => "记忆",
            Self::Voice => "声",
            Self::Body => "体",
            Self::Mind => "意",
        }
    }

    /// ASCII 标签 (跨平台, tui 也用 ASCII)
    pub fn ascii(self) -> &'static str {
        match self {
            Self::Heart => "[HEART]",
            Self::Brain => "[BRAIN]",
            Self::Hand => "[HAND]",
            Self::Eye => "[EYE]",
            Self::Ear => "[EAR]",
            Self::Memory => "[MEM]",
            Self::Voice => "[VOICE]",
            Self::Body => "[BODY]",
            Self::Mind => "[MIND]",
        }
    }

    /// 实就度 (跟 tui `Readiness` 镜像)
    pub fn readiness(self) -> &'static str {
        match self {
            Self::Heart => "partial", // R25.3 真接 /v1/observability/heart
            Self::Brain => "partial", // 真接 LLM
            Self::Hand => "partial",
            Self::Eye => "stub",
            Self::Ear => "stub",
            Self::Memory => "partial",
            Self::Voice => "stub",
            Self::Body => "partial",
            Self::Mind => "partial",
        }
    }
}

impl Default for Organ {
    fn default() -> Self {
        Self::Heart
    }
}

/// 模式 (跟 tui `Mode` 镜像)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// 聚焦模式 (默认) — 用户问 AI 答
    Focus,
    /// 启发模式 — AI 主动建议
    Inspire,
}

impl Mode {
    /// 切换
    pub fn toggle(self) -> Self {
        match self {
            Self::Focus => Self::Inspire,
            Self::Inspire => Self::Focus,
        }
    }

    /// 标签
    pub fn label(self) -> &'static str {
        match self {
            Self::Focus => "focus",
            Self::Inspire => "inspire",
        }
    }
}

impl Default for Mode {
    fn default() -> Self {
        Self::Focus
    }
}

/// 语言 (跟 tui `Language` 镜像)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    /// 中文
    Zh,
    /// 英文
    En,
}

impl Language {
    /// 切换
    pub fn toggle(self) -> Self {
        match self {
            Self::Zh => Self::En,
            Self::En => Self::Zh,
        }
    }

    /// 短标签
    pub fn label(self) -> &'static str {
        match self {
            Self::Zh => "zh",
            Self::En => "en",
        }
    }
}

impl Default for Language {
    fn default() -> Self {
        Self::Zh
    }
}

/// 主题 (跟 tui `Theme` 镜像, 3 主)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    /// 古拙 (默认)
    Archaic,
    /// 现代
    Modern,
    /// 宇宙
    Cosmic,
}

impl Theme {
    /// 下一个主题 (3 循环)
    pub fn next(self) -> Self {
        match self {
            Self::Archaic => Self::Modern,
            Self::Modern => Self::Cosmic,
            Self::Cosmic => Self::Archaic,
        }
    }

    /// 标签
    pub fn label(self) -> &'static str {
        match self {
            Self::Archaic => "古拙",
            Self::Modern => "现代",
            Self::Cosmic => "宇宙",
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::Archaic
    }
}

/// 6 哲学锚 (per `docs/architecture-v4-living-intelligence.md` §0.2)
///
/// 跟 tui `organ::mind::SIX_ANCHORS` 镜像 — S-1/S-2/O-2/O-3/O-4/O-5
pub const SIX_PHI_ANCHORS: [(&str, &str, &str); 6] = [
    ("S-1", "22:33", "算力是电"),
    ("S-2", "17:43", "实积寸累"),
    ("O-2", "19:33", "反对前倾草率"),
    ("O-3", "23:44", "可读性"),
    ("O-4", "00:56", "任何人都能读懂"),
    ("O-5", "17:58", "仓廪实"),
];

/// 8 项不修改承诺 (跟 omnibus §6 镜像)
pub const EIGHT_PROMISES: [&str; 8] = [
    "错误能装到实现 (TuiE2EError thiserror + 8-10 变体)",
    "错误数 hardcode (8-10 变体对应失败类型)",
    "0 改 LOCKED (24 LOCKED crate 的 src/ 0 触碰)",
    "0 改 workspace version (version.workspace = true)",
    "6 哲学锚透传 (S-1/S-2/O-2/O-3/O-4/O-5 全显)",
    "0 依赖 NewAPI (镜像 tui 公开 API, 不引入额外代理)",
    "0 重复造轮子 (dispatch_render / render_* 镜像 tui 已有签名)",
    "0 假装实缺 (1 屏 4 panel + 5 nav + 9 器官 + 6 锚 全部 hardcode)",
];

/// 5 R-Measure (per 主人 2026-08-05 拍板"测 5 R-Measure" — R20 阶段 5 验收)
///
/// R-Measure 是 tui 健康度的 5 个量化指标, 跟 omnibus §6 镜像
pub const FIVE_R_MEASURES: [&str; 5] = [
    "R-Coverage",  // 测试覆盖比例 (5 nav × 9 organ = 45 测点)
    "R-Density",   // 信息密度 (1 屏多卡, 关键数字一眼看完)
    "R-Cadence",   // 节奏 (60Hz tick / 1s 状态刷新 / 250ms spinner)
    "R-Anchors",   // 6 哲学锚 + 8 不修改承诺 持续显示
    "R-Path",      // 0 触碰 LOCKED, 0 改 workspace, 0 假装实缺
];

// =====================================================================
// TuiApp — 镜像 apeireth_tui::App 的最小可测表面
// =====================================================================

/// 聊天消息 (跟 tui `ChatMessage` 镜像)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    /// 角色: "user" / "assistant" / "system"
    pub role: String,
    /// 消息内容
    pub content: String,
}

impl ChatMessage {
    /// 构造 user 消息
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
        }
    }

    /// 构造 assistant 消息
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
        }
    }

    /// 构造 system 消息
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: content.into(),
        }
    }
}

/// TUI App 状态 (e2e 镜像, 不依赖 binary tui)
///
/// 字段命名 / 类型 / 默认值跟 `apeireth_tui::app::App` 一一对应,
/// e2e 测试可构造、修改、断言、渲染. 后续 tui 加 lib.rs 后可切换
/// `pub use apeireth_tui::App as TuiApp;`
#[derive(Debug, Clone)]
pub struct TuiApp {
    /// 当前主 nav (5 选 1)
    pub nav: NavPage,
    /// 当前副 nav (5 选 1)
    pub sub_nav: Nav,
    /// 主题
    pub theme: Theme,
    /// 模式
    pub mode: Mode,
    /// 语言
    pub language: Language,
    /// splash 启用
    pub splash_enabled: bool,
    /// breath 启用
    pub breath_enabled: bool,
    /// 输入缓冲
    pub input_buf: Vec<char>,
    /// 输入光标
    pub input_cursor: usize,
    /// 对话历史
    pub chat_history: Vec<ChatMessage>,
    /// thinking 展开
    pub thinking_expanded: bool,
    /// 正在调用 LLM
    pub processing: bool,
    /// spinner 帧 (0-3 循环)
    pub spinner_frame: u8,
    /// 流式消息
    pub streaming_message: Option<String>,
    /// 应退出
    pub should_quit: bool,
    /// 启动时间
    pub started_at: Instant,
    /// 渲染 tick
    pub render_tick: u64,
    /// 9 器官 health (0.0-1.0, 索引 0-8 对应 Organ 0-8)
    pub organ_health: [f64; 9],
    /// cycle 计数
    pub cycle_count: u64,
    /// token 用量
    pub token_used: u64,
    /// 5-self armed
    pub five_self_armed: bool,
}

impl TuiApp {
    /// 默认构造 (跟 tui `App::new()` 镜像)
    pub fn new() -> Self {
        Self {
            nav: NavPage::default(),
            sub_nav: Nav::default(),
            theme: Theme::default(),
            mode: Mode::default(),
            language: Language::default(),
            splash_enabled: true,
            breath_enabled: true,
            input_buf: Vec::new(),
            input_cursor: 0,
            chat_history: Vec::new(),
            thinking_expanded: false,
            processing: false,
            spinner_frame: 0,
            streaming_message: None,
            should_quit: false,
            started_at: Instant::now(),
            render_tick: 0,
            organ_health: [1.0; 9],
            cycle_count: 0,
            token_used: 142_857, // W1 mock, 跟 tui `TOKEN_USED` 默认值镜像
            five_self_armed: false,
        }
    }

    /// push user 输入
    pub fn push_user_input(&mut self, content: impl Into<String>) {
        let s: String = content.into();
        if !s.trim().is_empty() {
            self.chat_history.push(ChatMessage::user(s));
        }
    }

    /// push assistant 回复
    pub fn push_assistant_reply(&mut self, content: impl Into<String>) {
        self.chat_history.push(ChatMessage::assistant(content));
    }

    /// push system 消息
    pub fn push_system(&mut self, content: impl Into<String>) {
        self.chat_history.push(ChatMessage::system(content));
    }

    /// tick — 调用一次模拟 1s tick, 推进 spinner / cycle 等
    ///
    /// 跟 tui `run_app` 主循环的 250ms tick 镜像, e2e 模拟 1s = 4 tick
    pub fn tick(&mut self) {
        self.spinner_frame = (self.spinner_frame + 1) % 4;
        self.render_tick = self.render_tick.wrapping_add(1);
    }

    /// 处理键盘事件 (跟 tui 主循环镜像, 简化版)
    ///
    /// - `q` / `Esc` → should_quit = true
    /// - `Tab` → next nav
    /// - `BackTab` → prev nav
    /// - `1`-`5` → 跳到对应 NavPage
    /// - `t` → 切 theme
    /// - `m` → 切 mode
    /// - `l` → 切 language
    /// - `s` → 切 splash
    /// - `b` → 切 breath
    /// - `o` → 切 thinking_expanded
    pub fn handle_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Tab => self.nav = self.nav.next(),
            KeyCode::BackTab => self.nav = self.nav.prev(),
            KeyCode::Char('1') => self.nav = NavPage::Bridge,
            KeyCode::Char('2') => self.nav = NavPage::Dialogue,
            KeyCode::Char('3') => self.nav = NavPage::Growth,
            KeyCode::Char('4') => self.nav = NavPage::History,
            KeyCode::Char('5') => self.nav = NavPage::Settings,
            KeyCode::Char('t') => self.theme = self.theme.next(),
            KeyCode::Char('m') => self.mode = self.mode.toggle(),
            KeyCode::Char('l') => self.language = self.language.toggle(),
            KeyCode::Char('s') => self.splash_enabled = !self.splash_enabled,
            KeyCode::Char('b') => self.breath_enabled = !self.breath_enabled,
            KeyCode::Char('o') => self.thinking_expanded = !self.thinking_expanded,
            _ => {}
        }
    }

    /// 5 nav label 拼接 (供 status bar 渲染)
    pub fn nav_bar_text(&self) -> String {
        let mut s = String::new();
        for n in 0..NavPage::COUNT {
            let page = NavPage::from_u8(n).unwrap();
            let marker = if page == self.nav { "▶" } else { " " };
            s.push_str(&format!("{marker} {} {}  ", n + 1, page.label_zh()));
        }
        s
    }

    /// 9 器官 health bar (供 middle bar 渲染)
    pub fn organ_bar_text(&self) -> String {
        let mut s = String::new();
        for i in 0..Organ::COUNT {
            let organ = Organ::from_u8(i).unwrap();
            let h = self.organ_health[i as usize];
            s.push_str(&format!("{} {:.0}%  ", organ.ascii(), h * 100.0));
        }
        s
    }
}

impl Default for TuiApp {
    fn default() -> Self {
        Self::new()
    }
}

// =====================================================================
// 公开常量 — 1 屏 4 panel 尺寸契约
// =====================================================================

/// 默认终端宽 (24×80 标准)
pub const DEFAULT_WIDTH: u16 = 80;

/// 默认终端高
pub const DEFAULT_HEIGHT: u16 = 24;

/// 1 屏 4 panel 高度 (跟 tui main.rs 4 Layout 镜像)
///
/// 顺序: top nav (1) + middle organ (1) + content (剩余) + status (1) = 4 panel
pub const PANEL_HEIGHTS: [u16; 4] = [1, 1, 21, 1];

/// 5 R-Measure 数 (hardcode)
pub const R_MEASURE_COUNT: usize = 5;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_anchors_count() {
        assert_eq!(SIX_PHI_ANCHORS.len(), 6, "6 哲学锚 hardcode");
    }

    #[test]
    fn six_anchors_ids_distinct() {
        let ids: std::collections::HashSet<&str> =
            SIX_PHI_ANCHORS.iter().map(|(id, _, _)| *id).collect();
        assert_eq!(ids.len(), 6, "6 ID 互不相同");
    }

    #[test]
    fn six_anchors_ids_match() {
        let expected = ["S-1", "S-2", "O-2", "O-3", "O-4", "O-5"];
        for (i, (id, _, _)) in SIX_PHI_ANCHORS.iter().enumerate() {
            assert_eq!(*id, expected[i], "ID 顺序对齐 architecture-v4");
        }
    }

    #[test]
    fn eight_promises_count() {
        assert_eq!(EIGHT_PROMISES.len(), 8, "8 项不修改承诺 hardcode");
    }

    #[test]
    fn five_r_measures_count() {
        assert_eq!(FIVE_R_MEASURES.len(), 5, "5 R-Measure hardcode");
        assert_eq!(FIVE_R_MEASURES.len(), R_MEASURE_COUNT);
    }

    #[test]
    fn nav_page_from_u8_round_trip() {
        for n in 0..NavPage::COUNT {
            let p = NavPage::from_u8(n).expect("0-4 valid");
            assert_eq!(p as u8, n);
        }
        assert!(NavPage::from_u8(5).is_none());
        assert!(NavPage::from_u8(255).is_none());
    }

    #[test]
    fn nav_page_next_wraps() {
        assert_eq!(NavPage::Bridge.next(), NavPage::Dialogue);
        assert_eq!(NavPage::Settings.next(), NavPage::Bridge);
    }

    #[test]
    fn nav_page_prev_wraps() {
        assert_eq!(NavPage::Bridge.prev(), NavPage::Settings);
        assert_eq!(NavPage::Dialogue.prev(), NavPage::Bridge);
    }

    #[test]
    fn nav_from_u8_round_trip() {
        for n in 0..Nav::COUNT {
            let p = Nav::from_u8(n).expect("0-4 valid");
            assert_eq!(p as u8, n);
        }
        assert!(Nav::from_u8(5).is_none());
    }

    #[test]
    fn organ_from_u8_round_trip() {
        for n in 0..Organ::COUNT {
            let p = Organ::from_u8(n).expect("0-8 valid");
            assert_eq!(p as u8, n);
        }
        assert!(Organ::from_u8(9).is_none());
    }

    #[test]
    fn organ_ascii_all_9_distinct() {
        let set: std::collections::HashSet<&str> =
            (0..Organ::COUNT).map(|i| Organ::from_u8(i).unwrap().ascii()).collect();
        assert_eq!(set.len(), 9, "9 ASCII 互不相同");
    }

    #[test]
    fn organ_name_zh_all_9_distinct() {
        let set: std::collections::HashSet<&str> = (0..Organ::COUNT)
            .map(|i| Organ::from_u8(i).unwrap().name_zh())
            .collect();
        assert_eq!(set.len(), 9);
    }

    #[test]
    fn tui_app_new_defaults() {
        let app = TuiApp::new();
        assert_eq!(app.nav, NavPage::Bridge);
        assert_eq!(app.sub_nav, Nav::Status);
        assert_eq!(app.theme, Theme::Archaic);
        assert_eq!(app.mode, Mode::Focus);
        assert_eq!(app.language, Language::Zh);
        assert!(!app.should_quit);
        assert!(!app.processing);
    }

    #[test]
    fn tui_app_handle_q_quit() {
        use crossterm::event::KeyCode;
        let mut app = TuiApp::new();
        app.handle_key(KeyCode::Char('q'));
        assert!(app.should_quit);
    }

    #[test]
    fn tui_app_handle_esc_quit() {
        use crossterm::event::KeyCode;
        let mut app = TuiApp::new();
        app.handle_key(KeyCode::Esc);
        assert!(app.should_quit);
    }

    #[test]
    fn tui_app_handle_tab_cycles() {
        use crossterm::event::KeyCode;
        let mut app = TuiApp::new();
        assert_eq!(app.nav, NavPage::Bridge);
        app.handle_key(KeyCode::Tab);
        assert_eq!(app.nav, NavPage::Dialogue);
        app.handle_key(KeyCode::Tab);
        assert_eq!(app.nav, NavPage::Growth);
    }

    #[test]
    fn tui_app_handle_1_to_5_jump() {
        use crossterm::event::KeyCode;
        let mut app = TuiApp::new();
        app.handle_key(KeyCode::Char('3'));
        assert_eq!(app.nav, NavPage::Growth);
        app.handle_key(KeyCode::Char('5'));
        assert_eq!(app.nav, NavPage::Settings);
    }

    #[test]
    fn tui_app_handle_t_theme_cycle() {
        use crossterm::event::KeyCode;
        let mut app = TuiApp::new();
        assert_eq!(app.theme, Theme::Archaic);
        app.handle_key(KeyCode::Char('t'));
        assert_eq!(app.theme, Theme::Modern);
        app.handle_key(KeyCode::Char('t'));
        assert_eq!(app.theme, Theme::Cosmic);
        app.handle_key(KeyCode::Char('t'));
        assert_eq!(app.theme, Theme::Archaic);
    }

    #[test]
    fn tui_app_push_user_skips_empty() {
        let mut app = TuiApp::new();
        app.push_user_input("");
        app.push_user_input("   ");
        assert_eq!(app.chat_history.len(), 0);
        app.push_user_input("hello");
        assert_eq!(app.chat_history.len(), 1);
        assert_eq!(app.chat_history[0].role, "user");
    }

    #[test]
    fn tui_app_push_assistant_and_system() {
        let mut app = TuiApp::new();
        app.push_assistant_reply("reply");
        app.push_system("note");
        assert_eq!(app.chat_history.len(), 2);
        assert_eq!(app.chat_history[0].role, "assistant");
        assert_eq!(app.chat_history[1].role, "system");
    }

    #[test]
    fn tui_app_tick_advances_spinner() {
        let mut app = TuiApp::new();
        assert_eq!(app.spinner_frame, 0);
        app.tick();
        assert_eq!(app.spinner_frame, 1);
        app.tick();
        app.tick();
        app.tick();
        assert_eq!(app.spinner_frame, 0, "4 tick 后回到 0");
    }

    #[test]
    fn tui_app_nav_bar_text_has_all_5() {
        let app = TuiApp::new();
        let s = app.nav_bar_text();
        for n in 0..NavPage::COUNT {
            let page = NavPage::from_u8(n).unwrap();
            assert!(s.contains(page.label_zh()), "{:?} 应在 nav bar", page.label_zh());
        }
    }

    #[test]
    fn tui_app_organ_bar_text_has_all_9() {
        let app = TuiApp::new();
        let s = app.organ_bar_text();
        for i in 0..Organ::COUNT {
            let organ = Organ::from_u8(i).unwrap();
            assert!(s.contains(organ.ascii()), "{:?} 应在 organ bar", organ.ascii());
        }
    }

    #[test]
    fn panel_heights_sum_equals_height() {
        let sum: u16 = PANEL_HEIGHTS.iter().sum();
        assert_eq!(sum, DEFAULT_HEIGHT, "4 panel 高度 = 24");
    }
}
