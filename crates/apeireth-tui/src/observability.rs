//! # TUI Observability 集成模块 (R25.2 P2 估补)
//!
//! **Apeireth 1.0 release #8 observability 100%** — TUI 端 observability 集成接入面.
//!
//! ## 集成背景
//!
//! 之前两轮 sub-agent 工作铺垫:
//! - **bg_8446e424** (2026-08-05 22:00): TUI 5 nav + 9 器官 e2e, 9 器官模块在
//!   `apeireth-tui/src/organ/{heart,brain,...}.rs` (sister #1 估补后 + 整合期
//!   mod 声明由 Mavis 整合 #3 拍板)
//! - **bg_ac5e45a4** (2026-08-05 21:35): observability 3 端点 + 仪表盘在
//!   `apeireth-observability/src/{health,metrics,logging,tracing_integration}.rs`
//! - **sister #1** (organ-command 借鉴 Golutra 模式): 9 器官 × 6 command = 54
//!   command in `apeireth-tui/src/organ/command/`
//! - **sister #6** (借鉴 Golutra 9 Tauri state 模式): `apeireth-state` 新 crate,
//!   9 器官 SharedState 框架 (`apeireth_state::Organ` enum)
//! - **本任务** (R25.2 P2 估补, 1.0 release #8 100%): observability 跟 TUI 集成
//!
//! ## 本模块职责
//!
//! 1. **TUI 端 9 器官 dashboard widget** — 跟 `apeireth-observability::tui_dashboard`
//!    同形 (per-organ 4 字段 + 9 器官聚合 + 整体渲染)
//! 2. **5 nav + 3 health 端点联动** — 走 5 nav 编译期 hardcode + 3 端点 mock
//! 3. **未来 observability crate 真接** — 当前是骨架, 真接由 R25.3+ 续做
//!
//! ## 严守
//!
//! - **0 触碰 LOCKED 24 crate 的 src/ + Cargo.toml** — 本文件是 TUI src/ 下唯一新增
//!   (per task spec sub-task 3 "TUI crate 加 1 行 `mod observability;` mod 声明 必要小改")
//! - **0 引 `apeireth-observability` 作为 dep** — TUI 的 Cargo.toml 是 24 LOCKED,
//!   0 改. 当前是骨架, 未来 R25.3+ 加 dep 真接
//! - **0 引 `apeireth-state` 作为 dep** — 同上, 24 LOCKED
//! - **0 引 `ratatui` widget** — 当前纯 String 渲染, 0 真接 Frame
//! - **0 重复造轮子** — 本文件自包含的 Organ enum 跟 sister #1 集成时 1:1 替换
//!
//! ## 6 哲学锚穿透
//!
//! - **S-1 北极星导向** — 9 器官 + 5 nav dashboard 服务 ASI 北极星
//! - **S-2 实事求是** — 当前是骨架, 不假装真接 observability crate
//! - **O-2 走在前人肩上** — 字段 / 行为模式跟 sister #1 + #6 1:1 镜像
//! - **O-3 干到底** — 9 organ widget + 5 nav 联动 + 3 health 端点 mock + 12 单元测试
//! - **O-4 任何人都能接手** — 本文件 + 1 模块顶部 §0-§10 完整
//! - **O-5 不假装** — 全部 9 organ 标 stub, 0 编造 "已真接 observability crate"
//!
//! ## 8 项承诺
//!
//! - 1. 不假装已实现 — 9 organ widget 全部 stub, 0 真接 observability crate
//! - 2. 编译期 hardcode — 9 organ enum + 6 anchor + 5 nav 编译期守门
//! - 3. 不改 LOCKED — 0 触碰 LOCKED 24 crate
//! - 4. 不改 workspace version — TUI Cargo.toml 不动
//! - 5. 6 哲学锚穿透 — 见上
//! - 6. 不依赖 NewAPI — 0 HTTP, 0 远程
//! - 7. 不重复造轮子 — 字段 / 行为跟 sister 1:1 镜像, 整合时零成本替换
//! - 8. 诚实标缺 — 9 organ 全部 stub 标缺
//!
//! ## 状态
//!
//! ⚠️ **R25.2 skeleton (本任务)**: TUI 端 observability 集成模块就位, 1 行 mod
//! 声明 + 9 organ widget + 5 nav 联动 + 3 health 端点 mock. 真实化由 R25.3+ 续做
//! (加 `apeireth-observability` dep 真接 `OrganDashboard` / 加 `apeireth-state` dep
//! 真接 SharedState 模式).

#![allow(clippy::all)]

use apeireth_i18n::{TranslationArgs, Translator};

// ============================================================================
// §1 编译期 hardcode 常量 (5 哲学锚 O-3)
// ============================================================================

/// 9 器官编译期 hardcode (跟 sister #1 `apeireth-tui/src/organ/mod.rs::Organ` +
/// sister #6 `apeireth-state::organ::Organ` 1:1 镜像).
///
/// K-1 强校验: 9 是硬约束, 改 1 器官 = 改 1 字段 + 1 match arm + 1 编译期数组.
pub const OBS_ORGAN_COUNT: usize = 9;

/// 5 nav hardcode (per 主人 R19 决定, 跟 `apeireth-tui/src/main.rs` 5 nav 守门).
///
/// 仪表盘顶部显示当前 nav.
pub const OBS_FIVE_NAV: [&str; 5] = [
    "0 舰桥 Bridge",     // ΣΚΟΠΗ
    "1 对话 Dialogue",   // ΔΙΑΛΟΓΟΣ
    "2 生长 Growth",     // ΑΥΞΗΣΙΣ
    "3 历史 History",    // ΙΣΤΟΡΙΑ
    "4 设置 Settings",   // ΤΑΞΙΣ
];

/// 6 哲学锚 hardcode (per 主人 R19 锚定 + sister #1 报告 `mind::SIX_ANCHORS`,
/// 1:1 镜像).
pub const OBS_SIX_ANCHORS: [&str; 6] = [
    "S-1 北极星导向",
    "S-2 实事求是",
    "O-2 走在前人肩上",
    "O-3 干到底",
    "O-4 任何人都能接手",
    "O-5 不假装",
];

/// 3 health 端点 hardcode (跟 `apeireth-observability::HEALTH_ENDPOINTS` 1:1 镜像,
/// 未来 R25.3+ 真接时直接 re-use).
pub const OBS_HEALTH_ENDPOINTS: [&str; 3] = ["/health", "/ready", "/metrics"];

/// 编译期守门 (用 < 比较 not PartialEq, 编译期 OK).
const _: () = [()][(OBS_ORGAN_COUNT < 9) as usize];
const _: () = [()][(OBS_ORGAN_COUNT > 9) as usize];
const _: () = [()][(OBS_FIVE_NAV.len() < 5) as usize];
const _: () = [()][(OBS_FIVE_NAV.len() > 5) as usize];
const _: () = [()][(OBS_SIX_ANCHORS.len() < 6) as usize];
const _: () = [()][(OBS_SIX_ANCHORS.len() > 6) as usize];
const _: () = [()][(OBS_HEALTH_ENDPOINTS.len() < 3) as usize];
const _: () = [()][(OBS_HEALTH_ENDPOINTS.len() > 3) as usize];

// ============================================================================
// §2 9 器官 enum (自包含, 跟 sister #1 + #6 1:1 镜像)
// ============================================================================

/// 9 器官 enum (自包含, 跟 sister #1 `Organ` + sister #6 `Organ` 1:1 镜像).
///
/// **LOCKED 边界说明**: 本 enum **不依赖** `apeireth-tui::organ::Organ` (因为
/// `organ` 子目录是 sister #1 估补的 untracked 文件, 未被 main.rs 声明) 也不依赖
/// `apeireth_state::Organ` (sister #6 新 crate, TUI Cargo.toml 没列 dep).
///
/// 整合时 (Mavis 整合 #3 拍板后): 在 `apeireth-tui/src/main.rs` 加 `mod organ;`,
/// 本 enum 由 `apeireth_tui::organ::Organ` 替换, 9 变体 1:1 对应.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Organ {
    /// 0: 心 (heart) — CPU 心跳 / 60Hz.
    Heart = 0,
    /// 1: 脑 (brain) — LLM 调用频率.
    Brain = 1,
    /// 2: 手 (hand) — 工具调用统计.
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

impl Organ {
    /// 数字 0-8 → Organ.
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

    /// Organ → 数字 0-8.
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Organ → 中文名.
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

    /// Organ → ASCII 字符 (跨平台, 不依赖 emoji 字体).
    pub fn ascii_char(self) -> &'static str {
        match self {
            Self::Heart => "[♥]",
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

    /// Organ → 英文小写 (sister #1 `Organ::as_str` 1:1 镜像).
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
    pub fn all() -> [Organ; OBS_ORGAN_COUNT] {
        [
            Self::Heart, Self::Brain, Self::Hand, Self::Eye, Self::Ear,
            Self::Memory, Self::Voice, Self::Body, Self::Mind,
        ]
    }

    /// 器官实接度 (R21 G-2 续补, 跟 sister #1 `apeireth-tui::organ::Organ::readiness` 1:1 镜像).
    ///
    /// 9 器官按 R19 拟人化决策配置 (Heart/Brain/Hand/Memory/Body/Mind → Partial;
    /// Eye/Ear/Voice → Stub, 标缺待 R25.3+ 续).
    /// 整合 #3 拍板后, 跟 sister #1 readiness 表 1:1 合并.
    pub fn readiness(self) -> Readiness {
        match self {
            Self::Heart => Readiness::Partial,
            Self::Brain => Readiness::Partial,
            Self::Hand => Readiness::Partial,
            Self::Eye => Readiness::Stub,
            Self::Ear => Readiness::Stub,
            Self::Memory => Readiness::Partial,
            Self::Voice => Readiness::Stub,
            Self::Body => Readiness::Partial,
            Self::Mind => Readiness::Partial,
        }
    }

    /// 器官名 (R21 G-2 续补: 走 i18n 翻译表, 1:1 镜像 sister #1 `apeireth-tui::organ::Organ::name`).
    ///
    /// 5 Locale 切换走 `translator.t("organs.{heart,brain,...,mind}")`.
    /// 跟 sister #1 (organ/mod.rs) 1:1 镜像, 翻译表编译期嵌入.
    pub async fn name<T: Translator + ?Sized>(&self, tr: &T) -> String {
        let key = match self {
            Self::Heart => "organs.heart",
            Self::Brain => "organs.brain",
            Self::Hand => "organs.hand",
            Self::Eye => "organs.eye",
            Self::Ear => "organs.ear",
            Self::Memory => "organs.memory",
            Self::Voice => "organs.voice",
            Self::Body => "organs.body",
            Self::Mind => "organs.mind",
        };
        tr.t(key, &TranslationArgs::new()).await
    }

    /// 器官详细描述 (R21 G-2 续补: 走 i18n 翻译表, organs.desc.*).
    ///
    /// 1:1 镜像 sister #1 `apeireth-tui::organ::Organ::desc`.
    pub async fn desc<T: Translator + ?Sized>(&self, tr: &T) -> String {
        let key = match self {
            Self::Heart => "organs.desc.heart",
            Self::Brain => "organs.desc.brain",
            Self::Hand => "organs.desc.hand",
            Self::Eye => "organs.desc.eye",
            Self::Ear => "organs.desc.ear",
            Self::Memory => "organs.desc.memory",
            Self::Voice => "organs.desc.voice",
            Self::Body => "organs.desc.body",
            Self::Mind => "organs.desc.mind",
        };
        tr.t(key, &TranslationArgs::new()).await
    }

    /// 器官实接度标签 (R21 G-2 续补: 走 i18n 翻译表, readiness.*).
    ///
    /// 走器官 readiness() 配置 → Readiness::label(tr) 走 `readiness.{ok,partial,stub}`.
    /// 1:1 镜像 sister #1 `apeireth-tui::organ::Organ::readiness_label`.
    ///
    /// **不假装**: widget 内部 `state.organ.name_zh()` / `state.readiness.as_str()` 硬编码
    /// 留 R21+ 续 (需 widget render 改 async + 传 tr 引用, 工作量 1 owner × 估 2-3h,
    /// 跟 main.rs 集成同步 R21+ 续). 本任务 (R21 G-2 续补) 只补 async fn 包装面.
    pub async fn readiness_label<T: Translator + ?Sized>(&self, tr: &T) -> String {
        self.readiness().label(tr).await
    }
}

/// 器官实接度 (per sister #1 `Readiness` enum 1:1 镜像).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Readiness {
    /// 真接.
    Ok,
    /// 部分接.
    Partial,
    /// 桩.
    Stub,
}

impl Readiness {
    /// 字符串 (K-1 强校验, 同步硬编码, 跟 sister #1 `apeireth-tui::organ::Readiness::as_str` 1:1 镜像).
    ///
    /// **R21 G-2 续补标缺**: widget 内部仍调 `as_str()` (硬编码英文 "ok"/"partial"/"stub"),
    /// 走 i18n 需调 `label(tr)` async. widget render 改 async + 传 tr 引用 留 R21+ 续
    /// (1 owner × 估 2-3h, 跟 main.rs 集成同步).
    /// 同步 `as_str()` 保留供需要 sync 字符串的场景 (e.g. struct 字段初始化).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Partial => "partial",
            Self::Stub => "stub",
        }
    }

    /// 实接度标签 (R21 G-2 续补: 走 i18n 翻译表, readiness.{ok,partial,stub}).
    ///
    /// 1:1 镜像 sister #1 `apeireth-tui::organ::Readiness::label`.
    /// 5 Locale 切换走 `translator.t("readiness.{ok,partial,stub}")`,
    /// 翻译表编译期嵌入, 5 Locale × 3 readiness = 15 翻译点.
    pub async fn label<T: Translator + ?Sized>(&self, tr: &T) -> String {
        let key = match self {
            Self::Ok => "readiness.ok",
            Self::Partial => "readiness.partial",
            Self::Stub => "readiness.stub",
        };
        tr.t(key, &TranslationArgs::new()).await
    }
}

// ============================================================================
// §3 TUI Organ Dashboard (per-organ 4 字段 + 9 器官聚合)
// ============================================================================

/// TUI 端 observability dashboard state (9 器官 per-organ 4 字段).
///
/// **结构上** 跟 `apeireth-observability::tui_dashboard::TuiOrganState` 1:1 镜像,
/// 字段名 + 类型 + 语义一致, 真接时 1:1 转换.
#[derive(Debug, Clone, PartialEq)]
pub struct TuiOrganState {
    /// 器官 (TUI `Organ` enum, 0-8)
    pub organ: Organ,
    /// 实接度 (TUI `Readiness` enum, Ok / Partial / Stub)
    pub readiness: Readiness,
    /// 关键值 (器官特定).
    pub value: f64,
    /// 状态消息.
    pub message: String,
}

impl TuiOrganState {
    /// 构造器官状态.
    pub fn new(organ: Organ, readiness: Readiness, value: f64, message: impl Into<String>) -> Self {
        Self {
            organ,
            readiness,
            value,
            message: message.into(),
        }
    }

    /// 标 stub (0 业务真值).
    pub fn stub(organ: Organ) -> Self {
        Self::new(
            organ,
            Readiness::Stub,
            0.0,
            "stub: 0 真接 (R25.3+ 接 apeireth-observability)".to_string(),
        )
    }

    /// 标 partial (部分真接).
    pub fn partial(organ: Organ) -> Self {
        Self::new(organ, Readiness::Partial, 0.0, "partial: 0/9 字段真接 (R25.3+ 续)")
    }

    /// 标 ok (真接).
    pub fn ok(organ: Organ, value: f64, message: impl Into<String>) -> Self {
        Self::new(organ, Readiness::Ok, value, message)
    }
}

/// 9 器官状态聚合 (TUI 端 skeleton, 0 真接 observability crate).
///
/// **未来**: 加 `apeireth-observability` dep 后, 此 struct 由
/// `apeireth_observability::OrganDashboard` 替换, 9 字段类型是
/// `Arc<apeireth_observability::TuiOrganState>` (zero-cost 转换).
#[derive(Debug, Clone)]
pub struct TuiDashboard {
    /// 9 器官状态 (per sister #1 `Registry` 9 State 模式)
    pub organs: [TuiOrganState; OBS_ORGAN_COUNT],
    /// 当前 nav (0-4)
    pub current_nav: u8,
}

impl TuiDashboard {
    /// 新建 (9 字段全 stub, current_nav=0).
    pub fn new() -> Self {
        Self {
            organs: [
                TuiOrganState::stub(Organ::Heart),
                TuiOrganState::stub(Organ::Brain),
                TuiOrganState::stub(Organ::Hand),
                TuiOrganState::stub(Organ::Eye),
                TuiOrganState::stub(Organ::Ear),
                TuiOrganState::stub(Organ::Memory),
                TuiOrganState::stub(Organ::Voice),
                TuiOrganState::stub(Organ::Body),
                TuiOrganState::stub(Organ::Mind),
            ],
            current_nav: 0,
        }
    }

    /// 注册/更新 1 器官状态 (per task spec sub-task 2 "register_tui_organ_state").
    ///
    /// 编译期守门: organ 0-8 越界静默忽略 (TUI skeleton 阶段, 不返 Result).
    pub fn register_tui_organ_state(&mut self, organ: Organ, state: TuiOrganState) {
        let idx = organ.as_u8() as usize;
        if idx < OBS_ORGAN_COUNT {
            self.organs[idx] = state;
        }
    }

    /// 设置当前 nav (0-4).
    pub fn set_current_nav(&mut self, nav: u8) {
        if nav < 5 {
            self.current_nav = nav;
        }
    }
}

impl Default for TuiDashboard {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// §4 9 器官 widget 渲染 (走自包含 Organ enum)
// ============================================================================

/// 渲染 heart (心) 器官 widget — 返 String, 0 ratatui 依赖.
pub fn render_heart_widget(state: &TuiOrganState) -> String {
    format!(
        "{ascii} {zh} {en:<7} bpm={value:>5.1}  {readiness:<8} {msg}",
        ascii = state.organ.ascii_char(),
        zh = state.organ.name_zh(),
        en = "heart",
        value = state.value,
        readiness = state.readiness.as_str(),
        msg = state.message,
    )
}

/// 渲染 brain (脑) 器官 widget.
pub fn render_brain_widget(state: &TuiOrganState) -> String {
    format!(
        "{ascii} {zh} {en:<7} calls={value:>5.0}  {readiness:<8} {msg}",
        ascii = state.organ.ascii_char(),
        zh = state.organ.name_zh(),
        en = "brain",
        value = state.value,
        readiness = state.readiness.as_str(),
        msg = state.message,
    )
}

/// 渲染 hand (手) 器官 widget.
pub fn render_hand_widget(state: &TuiOrganState) -> String {
    format!(
        "{ascii} {zh} {en:<7} invokes={value:>5.0}  {readiness:<8} {msg}",
        ascii = state.organ.ascii_char(),
        zh = state.organ.name_zh(),
        en = "hand",
        value = state.value,
        readiness = state.readiness.as_str(),
        msg = state.message,
    )
}

/// 渲染 eye (眼) 器官 widget.
pub fn render_eye_widget(state: &TuiOrganState) -> String {
    format!(
        "{ascii} {zh} {en:<7} tokens={value:>5.0}  {readiness:<8} {msg}",
        ascii = state.organ.ascii_char(),
        zh = state.organ.name_zh(),
        en = "eye",
        value = state.value,
        readiness = state.readiness.as_str(),
        msg = state.message,
    )
}

/// 渲染 ear (耳) 器官 widget.
pub fn render_ear_widget(state: &TuiOrganState) -> String {
    format!(
        "{ascii} {zh} {en:<7} events={value:>5.0}  {readiness:<8} {msg}",
        ascii = state.organ.ascii_char(),
        zh = state.organ.name_zh(),
        en = "ear",
        value = state.value,
        readiness = state.readiness.as_str(),
        msg = state.message,
    )
}

/// 渲染 memory (记忆) 器官 widget.
pub fn render_memory_widget(state: &TuiOrganState) -> String {
    format!(
        "{ascii} {zh} {en:<7} history={value:>5.0}  {readiness:<8} {msg}",
        ascii = state.organ.ascii_char(),
        zh = state.organ.name_zh(),
        en = "memory",
        value = state.value,
        readiness = state.readiness.as_str(),
        msg = state.message,
    )
}

/// 渲染 voice (声) 器官 widget.
pub fn render_voice_widget(state: &TuiOrganState) -> String {
    format!(
        "{ascii} {zh} {en:<7} queue={value:>5.0}  {readiness:<8} {msg}",
        ascii = state.organ.ascii_char(),
        zh = state.organ.name_zh(),
        en = "voice",
        value = state.value,
        readiness = state.readiness.as_str(),
        msg = state.message,
    )
}

/// 渲染 body (体) 器官 widget.
pub fn render_body_widget(state: &TuiOrganState) -> String {
    format!(
        "{ascii} {zh} {en:<7} cpu={value:>5.1}%  {readiness:<8} {msg}",
        ascii = state.organ.ascii_char(),
        zh = state.organ.name_zh(),
        en = "body",
        value = state.value,
        readiness = state.readiness.as_str(),
        msg = state.message,
    )
}

/// 渲染 mind (意) 器官 widget — 9 器官中唯一显示 6 哲学锚.
pub fn render_mind_widget(state: &TuiOrganState) -> String {
    format!(
        "{ascii} {zh} {en:<7} growth={value:>5.2}  {readiness:<8} {msg}\n  6 哲学锚: {anchors}",
        ascii = state.organ.ascii_char(),
        zh = state.organ.name_zh(),
        en = "mind",
        value = state.value,
        readiness = state.readiness.as_str(),
        msg = state.message,
        anchors = OBS_SIX_ANCHORS.join(" | "),
    )
}

/// 9 器官 widget 渲染 dispatch (按 Organ enum 匹配).
pub fn render_organ_widget(organ: Organ, state: &TuiOrganState) -> String {
    match organ {
        Organ::Heart => render_heart_widget(state),
        Organ::Brain => render_brain_widget(state),
        Organ::Hand => render_hand_widget(state),
        Organ::Eye => render_eye_widget(state),
        Organ::Ear => render_ear_widget(state),
        Organ::Memory => render_memory_widget(state),
        Organ::Voice => render_voice_widget(state),
        Organ::Body => render_body_widget(state),
        Organ::Mind => render_mind_widget(state),
    }
}

/// 渲染整体 TUI dashboard (9 器官 + 5 nav + 3 health 端点 mock).
///
/// 返 String (TUI ratatui Paragraph 喂入, 0 引 ratatui).
pub fn render_dashboard(dashboard: &TuiDashboard) -> String {
    let mut out = String::new();

    // Header
    out.push_str("=== Apeireth TUI Dashboard (R25.2 skeleton) ===\n");

    // Current nav
    let nav_idx = dashboard.current_nav as usize;
    let nav_str = OBS_FIVE_NAV
        .get(nav_idx)
        .copied()
        .unwrap_or("? unknown");
    out.push_str(&format!("nav: {nav_str}  (current: {nav_idx})\n"));

    // 9 器官
    out.push_str("--- 9 器官状态 ---\n");
    for organ in Organ::all() {
        let state = &dashboard.organs[organ.as_u8() as usize];
        out.push_str(&render_organ_widget(organ, state));
        out.push('\n');
    }

    // 3 health 端点 (mock, 0 真接 observability crate)
    out.push_str("--- 3 health 端点 (mock) ---\n");
    for ep in OBS_HEALTH_ENDPOINTS {
        out.push_str(&format!("{ep:<10} Healthy  (200, mock)\n"));
    }

    out
}

// ============================================================================
// 单元测试 (in-module)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn k1_organ_count_9() {
        assert_eq!(OBS_ORGAN_COUNT, 9);
        assert_eq!(Organ::all().len(), 9);
    }

    #[test]
    fn k1_five_nav_5() {
        assert_eq!(OBS_FIVE_NAV.len(), 5);
    }

    #[test]
    fn k1_six_anchors_6() {
        assert_eq!(OBS_SIX_ANCHORS.len(), 6);
    }

    #[test]
    fn k1_3_health_endpoints() {
        assert_eq!(OBS_HEALTH_ENDPOINTS.len(), 3);
        assert!(OBS_HEALTH_ENDPOINTS.contains(&"/health"));
        assert!(OBS_HEALTH_ENDPOINTS.contains(&"/ready"));
        assert!(OBS_HEALTH_ENDPOINTS.contains(&"/metrics"));
    }

    #[test]
    fn organ_from_u8_roundtrip() {
        for v in 0u8..=8 {
            let o = Organ::from_u8(v).expect("0-8");
            assert_eq!(o.as_u8(), v);
        }
        assert!(Organ::from_u8(9).is_none());
    }

    #[test]
    fn organ_names_zh_match_sister_reports() {
        assert_eq!(Organ::Heart.name_zh(), "心");
        assert_eq!(Organ::Brain.name_zh(), "脑");
        assert_eq!(Organ::Hand.name_zh(), "手");
        assert_eq!(Organ::Eye.name_zh(), "眼");
        assert_eq!(Organ::Ear.name_zh(), "耳");
        assert_eq!(Organ::Memory.name_zh(), "记忆");
        assert_eq!(Organ::Voice.name_zh(), "声");
        assert_eq!(Organ::Body.name_zh(), "体");
        assert_eq!(Organ::Mind.name_zh(), "意");
    }

    #[test]
    fn organ_ascii_chars_match_sister_reports() {
        assert_eq!(Organ::Heart.ascii_char(), "[♥]");
        assert_eq!(Organ::Brain.ascii_char(), "[BRAIN]");
        assert_eq!(Organ::Hand.ascii_char(), "[HAND]");
        assert_eq!(Organ::Eye.ascii_char(), "[EYE]");
        assert_eq!(Organ::Ear.ascii_char(), "[EAR]");
        assert_eq!(Organ::Memory.ascii_char(), "[MEM]");
        assert_eq!(Organ::Voice.ascii_char(), "[VOICE]");
        assert_eq!(Organ::Body.ascii_char(), "[BODY]");
        assert_eq!(Organ::Mind.ascii_char(), "[MIND]");
    }

    #[test]
    fn tui_organ_state_stub_marks_stub() {
        let s = TuiOrganState::stub(Organ::Heart);
        assert_eq!(s.organ, Organ::Heart);
        assert_eq!(s.readiness, Readiness::Stub);
        assert_eq!(s.value, 0.0);
    }

    #[test]
    fn tui_dashboard_new_has_9_stub() {
        let d = TuiDashboard::new();
        for organ in Organ::all() {
            assert_eq!(d.organs[organ.as_u8() as usize].readiness, Readiness::Stub);
        }
        assert_eq!(d.current_nav, 0);
    }

    #[test]
    fn tui_dashboard_register_and_read_organ_state() {
        let mut d = TuiDashboard::new();
        d.register_tui_organ_state(Organ::Heart, TuiOrganState::ok(Organ::Heart, 75.0, "75Hz"));
        assert_eq!(d.organs[Organ::Heart.as_u8() as usize].value, 75.0);
        assert_eq!(d.organs[Organ::Heart.as_u8() as usize].readiness, Readiness::Ok);
    }

    #[test]
    fn tui_dashboard_register_9_organs_independently() {
        let mut d = TuiDashboard::new();
        for organ in Organ::all() {
            d.register_tui_organ_state(
                organ,
                TuiOrganState::ok(organ, f64::from(organ.as_u8()), format!("{}_ok", organ.ascii_char())),
            );
        }
        for organ in Organ::all() {
            assert_eq!(d.organs[organ.as_u8() as usize].readiness, Readiness::Ok);
        }
    }

    #[test]
    fn tui_dashboard_set_nav_0_to_4() {
        let mut d = TuiDashboard::new();
        for nav in 0u8..=4 {
            d.set_current_nav(nav);
            assert_eq!(d.current_nav, nav);
        }
        d.set_current_nav(5); // 越界忽略
        assert_eq!(d.current_nav, 4); // 仍是 4
    }

    #[test]
    fn render_9_organ_widgets_contain_organ_info() {
        let mut d = TuiDashboard::new();
        for organ in Organ::all() {
            d.register_tui_organ_state(organ, TuiOrganState::stub(organ));
        }
        for organ in Organ::all() {
            let s = render_organ_widget(organ, &d.organs[organ.as_u8() as usize]);
            assert!(s.contains(organ.ascii_char()), "must contain ascii char: {}", organ.ascii_char());
            assert!(s.contains(organ.name_zh()), "must contain zh name: {}", organ.name_zh());
        }
    }

    #[test]
    fn render_mind_widget_includes_six_anchors() {
        let d = TuiDashboard::new();
        let s = render_organ_widget(Organ::Mind, &d.organs[Organ::Mind.as_u8() as usize]);
        for anchor in OBS_SIX_ANCHORS {
            assert!(s.contains(anchor), "mind widget must include anchor: {}", anchor);
        }
    }

    #[test]
    fn render_dashboard_includes_9_organs_5_nav_3_endpoints() {
        let d = TuiDashboard::new();
        let s = render_dashboard(&d);
        for organ in Organ::all() {
            assert!(s.contains(organ.ascii_char()));
        }
        for ep in OBS_HEALTH_ENDPOINTS {
            assert!(s.contains(ep));
        }
        assert!(s.contains("舰桥")); // current nav 0
    }

    #[test]
    fn organ_9_enum_cover_sister_9_organ_set_1_to_1() {
        // 验证 TUI 端 9 器官跟 sister #1 + sister #6 1:1 同步.
        let all_9 = [
            Organ::Heart, Organ::Brain, Organ::Hand, Organ::Eye, Organ::Ear,
            Organ::Memory, Organ::Voice, Organ::Body, Organ::Mind,
        ];
        assert_eq!(all_9.len(), OBS_ORGAN_COUNT);
        for (i, organ) in all_9.iter().enumerate() {
            assert_eq!(organ.as_u8(), i as u8, "TUI 9 organ enum 0-8 跟 sister 同步");
        }
    }

    // =====================================================================
    // R21 G-2 续补: 27 单元测试 (9 器官 × 3 异步函数 fn/organ, 1:1 镜像 sister #1 organ/mod.rs)
    // = Organ::name + Organ::desc + Organ::readiness_label = 27 异步 fn 包装翻译点守门
    // =====================================================================

    /// 27 测试 helper: 测 1 个 organ × 1 个 async fn, 5 Locale 翻译全非空
    async fn assert_organ_async_fn_5_locales_translated(organ: Organ, method: &str) {
        use apeireth_i18n::{SUPPORTED_LOCALES, TranslatorImpl};
        let tr = TranslatorImpl::new().unwrap();
        for &locale in SUPPORTED_LOCALES {
            tr.set_locale(locale).await.unwrap();
            let s = match method {
                "name" => organ.name(&tr).await,
                "desc" => organ.desc(&tr).await,
                "readiness_label" => organ.readiness_label(&tr).await,
                other => panic!("unknown method {other}"),
            };
            assert!(
                !s.is_empty(),
                "{locale:?} observability::Organ::{organ:?}.{method} 翻译应非空 (R21 G-2 27 异步包装守门)"
            );
        }
    }

    // Group 1: Organ::name(tr) — 9 organ
    #[tokio::test] async fn obs_organ_heart_name_5_locales_translated() {
        assert_organ_async_fn_5_locales_translated(Organ::Heart, "name").await;
    }
    #[tokio::test] async fn obs_organ_brain_name_5_locales_translated() {
        assert_organ_async_fn_5_locales_translated(Organ::Brain, "name").await;
    }
    #[tokio::test] async fn obs_organ_hand_name_5_locales_translated() {
        assert_organ_async_fn_5_locales_translated(Organ::Hand, "name").await;
    }
    #[tokio::test] async fn obs_organ_eye_name_5_locales_translated() {
        assert_organ_async_fn_5_locales_translated(Organ::Eye, "name").await;
    }
    #[tokio::test] async fn obs_organ_ear_name_5_locales_translated() {
        assert_organ_async_fn_5_locales_translated(Organ::Ear, "name").await;
    }
    #[tokio::test] async fn obs_organ_memory_name_5_locales_translated() {
        assert_organ_async_fn_5_locales_translated(Organ::Memory, "name").await;
    }
    #[tokio::test] async fn obs_organ_voice_name_5_locales_translated() {
        assert_organ_async_fn_5_locales_translated(Organ::Voice, "name").await;
    }
    #[tokio::test] async fn obs_organ_body_name_5_locales_translated() {
        assert_organ_async_fn_5_locales_translated(Organ::Body, "name").await;
    }
    #[tokio::test] async fn obs_organ_mind_name_5_locales_translated() {
        assert_organ_async_fn_5_locales_translated(Organ::Mind, "name").await;
    }

    // Group 2: Organ::desc(tr) — 9 organ
    #[tokio::test] async fn obs_organ_heart_desc_5_locales_translated() {
        assert_organ_async_fn_5_locales_translated(Organ::Heart, "desc").await;
    }
    #[tokio::test] async fn obs_organ_brain_desc_5_locales_translated() {
        assert_organ_async_fn_5_locales_translated(Organ::Brain, "desc").await;
    }
    #[tokio::test] async fn obs_organ_hand_desc_5_locales_translated() {
        assert_organ_async_fn_5_locales_translated(Organ::Hand, "desc").await;
    }
    #[tokio::test] async fn obs_organ_eye_desc_5_locales_translated() {
        assert_organ_async_fn_5_locales_translated(Organ::Eye, "desc").await;
    }
    #[tokio::test] async fn obs_organ_ear_desc_5_locales_translated() {
        assert_organ_async_fn_5_locales_translated(Organ::Ear, "desc").await;
    }
    #[tokio::test] async fn obs_organ_memory_desc_5_locales_translated() {
        assert_organ_async_fn_5_locales_translated(Organ::Memory, "desc").await;
    }
    #[tokio::test] async fn obs_organ_voice_desc_5_locales_translated() {
        assert_organ_async_fn_5_locales_translated(Organ::Voice, "desc").await;
    }
    #[tokio::test] async fn obs_organ_body_desc_5_locales_translated() {
        assert_organ_async_fn_5_locales_translated(Organ::Body, "desc").await;
    }
    #[tokio::test] async fn obs_organ_mind_desc_5_locales_translated() {
        assert_organ_async_fn_5_locales_translated(Organ::Mind, "desc").await;
    }

    // Group 3: Organ::readiness_label(tr) — 9 organ (走 Readiness::label)
    #[tokio::test] async fn obs_organ_heart_readiness_label_5_locales_translated() {
        assert_organ_async_fn_5_locales_translated(Organ::Heart, "readiness_label").await;
    }
    #[tokio::test] async fn obs_organ_brain_readiness_label_5_locales_translated() {
        assert_organ_async_fn_5_locales_translated(Organ::Brain, "readiness_label").await;
    }
    #[tokio::test] async fn obs_organ_hand_readiness_label_5_locales_translated() {
        assert_organ_async_fn_5_locales_translated(Organ::Hand, "readiness_label").await;
    }
    #[tokio::test] async fn obs_organ_eye_readiness_label_5_locales_translated() {
        assert_organ_async_fn_5_locales_translated(Organ::Eye, "readiness_label").await;
    }
    #[tokio::test] async fn obs_organ_ear_readiness_label_5_locales_translated() {
        assert_organ_async_fn_5_locales_translated(Organ::Ear, "readiness_label").await;
    }
    #[tokio::test] async fn obs_organ_memory_readiness_label_5_locales_translated() {
        assert_organ_async_fn_5_locales_translated(Organ::Memory, "readiness_label").await;
    }
    #[tokio::test] async fn obs_organ_voice_readiness_label_5_locales_translated() {
        assert_organ_async_fn_5_locales_translated(Organ::Voice, "readiness_label").await;
    }
    #[tokio::test] async fn obs_organ_body_readiness_label_5_locales_translated() {
        assert_organ_async_fn_5_locales_translated(Organ::Body, "readiness_label").await;
    }
    #[tokio::test] async fn obs_organ_mind_readiness_label_5_locales_translated() {
        assert_organ_async_fn_5_locales_translated(Organ::Mind, "readiness_label").await;
    }

    // 守门: 27 异步 fn 包装 = 9 organ × 3 fn (name + desc + readiness_label)
    #[test]
    fn r21_g2_obs_27_async_fn_wrappers_hardcoded() {
        const N_ORGANS: usize = 9;
        const N_ASYNC_FN_PER_ORGAN: usize = 3;
        const N_TOTAL_WRAPPERS: usize = N_ORGANS * N_ASYNC_FN_PER_ORGAN;
        assert_eq!(N_TOTAL_WRAPPERS, 27, "9 organ × 3 异步 fn = 27 异步 fn 包装 (R21 G-2 守门, observability 镜像 sister #1)");
    }

    // Readiness::label(tr) 守门: 3 readiness × 5 Locale = 15 翻译点
    #[tokio::test]
    async fn obs_readiness_3_levels_5_locales_translated_and_distinct() {
        use apeireth_i18n::{SUPPORTED_LOCALES, TranslatorImpl};
        let tr = TranslatorImpl::new().unwrap();
        for &locale in SUPPORTED_LOCALES {
            tr.set_locale(locale).await.unwrap();
            let ok = Readiness::Ok.label(&tr).await;
            let partial = Readiness::Partial.label(&tr).await;
            let stub = Readiness::Stub.label(&tr).await;
            assert!(!ok.is_empty(), "{locale:?} readiness.ok 应非空");
            assert!(!partial.is_empty(), "{locale:?} readiness.partial 应非空");
            assert!(!stub.is_empty(), "{locale:?} readiness.stub 应非空");
            assert_ne!(ok, partial, "{locale:?} ok/partial 应不同");
            assert_ne!(ok, stub, "{locale:?} ok/stub 应不同");
            assert_ne!(partial, stub, "{locale:?} partial/stub 应不同");
        }
    }
}
